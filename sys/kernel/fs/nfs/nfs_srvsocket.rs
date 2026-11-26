/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

/*
 * Copyright (c) 1989, 1991, 1993, 1995
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)nfs_socket.c	8.5 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS server's half of `nfs_socket.c` (N6.7b2): the reply's RPC header
//! (`nfs_rephead`), the request's (`nfs_getreq`), a server socket's receive (`nfsrv_rcv`,
//! the socket upcall and the nfsds' blocking call), the stream's record marks
//! (`nfsrv_getstream`), a record taken for an nfsd (`nfsrv_dorec`) and the wakeup of an idle
//! nfsd (`nfsrv_wakenfsd`). The server's since N6.7b4, which registers the upcall
//! (`nfs_init`).
//!
//! Upstream: sys/nfs/nfs_socket.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_socket.rs@e8249449d905
//!
//! `option NFSSERVER` only: the module is behind the kernel's `nfsserver` feature.
//!
//! ## Deviations
//! - `nfs_rephead` answers the reply being built as an `XdrWriter` (its chain is the C's
//!   `*mrq`, the writer the C's `*mbp`); `nfs_getreq` frees the request on every error, as the
//!   C's `nfsd_dissect` does, and answers `Result`.
//! - A last fragment of length 0 cut with no fragment before it (its mark followed by more of
//!   the stream: the cut takes no mbuf) queues no record; the C stores the empty fragment list
//!   as the queue's tail (`ns_recend = NULL`), so the next record overwrites `ns_rec` and the
//!   records queued before are lost. A mark of length 0 that ends the stream cuts the mark's
//!   emptied mbuf as its record, as the C does (its parse is `EBADRPC`).
//! - `nfs_realign` is gone: the server reads every request through `XdrReader`, which copies
//!   the words out, so an mbuf whose data is not word aligned needs no copy.
//!
//! ## Redesign
//! - LZ's receive parked the mbufs on the socket's record (`ns_raw`, `ns_rawend`, `ns_rec`,
//!   `ns_recend`, `ns_frag`: `Cell<Option<&'static Mbuf>>`), walked and cut them through raw
//!   links and read the record mark through an `unsafe` view (two `unsafe`); the descriptor
//!   was a pool item cast to `&mut` (one `unsafe`) and the upcall's argument a word cast back
//!   to the record (one). Here the queued mbufs are owned lists in one `LoanSlot` of the
//!   record ([`NsRcv`]): the stream's bytes an `OwnedSlist` of mbufs (O(1) at both ends, the
//!   C's head and tail pointers), the records an `MbufList`. Every change the C makes without
//!   a sleep is one claim of the slot (two atomics, no spl); `nfsrv_getstream`, which may
//!   sleep in `m_copym(M_WAIT)`, works on the stream's state taken out of the slot by
//!   `nfsrv_rcv` and put back with the records it cut, unless the socket was zapped
//!   meanwhile (then what it held is freed). A record is cut by moving whole mbufs from the
//!   stream's head (O(1) each) and copying the part of the last one the record ends in, as
//!   the C does; the mark is read byte by byte from the head, the C's loop. The descriptor is
//!   a `PoolBox` of "ndscpl" its nfsd owns.
//! - `nfsrv_rcv`'s upcall argument (`so_upcallarg`, LZ's record address exposed into a word
//!   and cast back with one `unsafe`) is the record's counted handle with its type erased
//!   (N6.7b4, D-10): [`nfsrv_rcv_upcall`] takes it back by naming `NfssvcSock<G>`, and the
//!   handle the socket's slot holds keeps the record alive across the call, whatever
//!   `nfsrv_zapsock` does meanwhile (it clears the slot, the C's `so_upcall = NULL`).
//! - `nfsrv_wakenfsd` walks `nfsd_head` under the kernel lock's reader and hands the socket
//!   to the nfsd it wakes through the nfsd's mailbox (`nfsd_slp`, a slot), the handle the
//!   socket's list membership holds cloned: no exclusive witness, so no spl pair.

use core::mem;
use core::sync::atomic::Ordering::Relaxed;

use hal::ds::owned::OwnedSlist;
use hal::mem::pool::{AnyArc, PR_WAITOK, PoolBox};
use hal::sync::klock;
use hal::sync::sleep::wakeup_one;
use hal::sys::errno::Errno;
use hal::sys::socket::MSG_DONTWAIT;

use super::nfs_socket::count_uio;
use super::nfs_srvsubs::nfsrv_errmap;
use super::nfs_subs::{
    NFSSTATS, NFSV3_PROCID, nfs_prog, rpc_auth_unix, rpc_autherr, rpc_call, rpc_mismatch,
    rpc_msgaccepted, rpc_msgdenied, rpc_reply, rpc_vers,
};
use super::nfsm_subs::{XdrWriter, nfsm_rndup};
use super::nfsproto::{
    NFS_MAXNAMLEN, NFS_MAXPACKET, NFS_NPROCS, NFS_VER2, NFS_VER3, NFSERR_AUTHERR, NFSERR_RETVOID,
    NFSPROC_COMMIT, NFSPROC_NOOP, NFSPROC_NULL, NFSV2PROC_STATFS, NFSX_UNSIGNED,
};
use super::nfssvc::{
    ND_NFSV3, NFSD_CHECKSLP, NFSD_WAITING, NfsrvDescript, NfssvcSock, NsRcv, SLP_DISCONN,
    SLP_DOREC, SLP_GETSTREAM, SLP_LASTFRAG, SLP_NEEDQ, SLP_VALID, nd_reader,
};
use super::rpcv2::{
    AUTH_REJECTCRED, RPC_GARBAGE, RPC_PROCUNAVAIL, RPC_PROGMISMATCH, RPC_PROGUNAVAIL, RPC_VER2,
    RPCAUTH_MAXSIZ, RPCAUTH_UNIXGIDS,
};
use super::xdr_subs::txdr_unsigned;
use crate::assert::{kassert, kpanic};
use crate::fs::subr::vfs;
use crate::net::mbuf::{
    M_DONTWAIT, M_WAIT, MAX_HDR, MHLEN, MNext, MT_DATA, MT_SONAME, MbufBox, MbufList, m_copym,
    m_gethdr, mclget,
};
use crate::net::proto::PR_CONNREQUIRED;
use crate::net::socket::{Socket, soreceive};
use crate::proc::ProcGlue;
use crate::proc::cred::UcredCell;
use crate::sys::socket::SOCK_STREAM;
use crate::sys::syslimits::NGROUPS_MAX;

/// The receive's count of bytes wanted: "as many as there are" (`uio_resid = 1000000000`).
const RCV_ALL: usize = 1_000_000_000;

/// The high bit of a record mark: the last fragment of its record.
const LASTFRAG_BIT: u32 = 0x8000_0000;

/// `nfs_rephead(siz, nd, slp, err, mrq, mbp)`: generates the RPC reply header for `nd`,
/// status `err` (0, an errno, `NFSERR_RETVOID` for a reply without a status word, or
/// `NFSERR_AUTHERR | why`), in a fresh packet header mbuf: a cluster when the reply (`siz`
/// bytes more) is big, else leading space left for the lower level headers. The writer
/// goes on after the header (the C's `*mbp`; its chain is the C's `*mrq`).
pub fn nfs_rephead<G: ProcGlue>(
    siz: usize,
    nd: &NfsrvDescript,
    _slp: &NfssvcSock<G>,
    err: i32,
) -> XdrWriter {
    let Some(mut mreq) = m_gethdr(M_WAIT, MT_DATA) else {
        kpanic(format_args!("nfs_rephead: out of mbufs"));
    };
    // If this is a big reply, use a cluster else try and leave leading space for the lower
    // level headers.
    let siz = siz.saturating_add(super::rpcv2::RPC_REPLYSIZ);
    let max_hdr = usize::try_from(MAX_HDR.load(Relaxed)).unwrap_or(0);
    if siz >= MHLEN.saturating_sub(max_hdr) {
        mclget(&mut mreq, M_WAIT);
    } else {
        mreq.data_add(max_hdr as isize);
    }
    mreq.m_len().set(0);
    let mut mb = XdrWriter::with_first(mreq);
    mb.word(txdr_unsigned(nd.nd_retxid));
    mb.word(rpc_reply);
    if err == Errno::ERPCMISMATCH.as_i32() || err & NFSERR_AUTHERR != 0 {
        mb.word(rpc_msgdenied);
        if err & NFSERR_AUTHERR != 0 {
            mb.word(rpc_autherr);
            mb.word(txdr_unsigned((err & !NFSERR_AUTHERR) as u32));
        } else {
            mb.word(rpc_mismatch);
            mb.word(txdr_unsigned(RPC_VER2));
            mb.word(txdr_unsigned(RPC_VER2));
        }
    } else {
        mb.word(rpc_msgaccepted);
        // AUTH_UNIX requires RPCAUTH_NULL.
        mb.word(0);
        mb.word(0);
        if err == Errno::EPROGUNAVAIL.as_i32() {
            mb.word(txdr_unsigned(RPC_PROGUNAVAIL));
        } else if err == Errno::EPROGMISMATCH.as_i32() {
            mb.word(txdr_unsigned(RPC_PROGMISMATCH));
            let tl = mb.build::<{ 2 * NFSX_UNSIGNED }>();
            tl[..4].copy_from_slice(&txdr_unsigned(NFS_VER2).to_ne_bytes());
            tl[4..].copy_from_slice(&txdr_unsigned(NFS_VER3).to_ne_bytes());
        } else if err == Errno::EPROCUNAVAIL.as_i32() {
            mb.word(txdr_unsigned(RPC_PROCUNAVAIL));
        } else if err == Errno::EBADRPC.as_i32() {
            mb.word(txdr_unsigned(RPC_GARBAGE));
        } else {
            mb.word(0);
            if err != NFSERR_RETVOID {
                mb.word(if err != 0 {
                    txdr_unsigned(nfsrv_errmap(nd, err) as u32)
                } else {
                    0
                });
            }
        }
    }
    if err != 0 && err != NFSERR_RETVOID {
        NFSSTATS.srvrpc_errs.fetch_add(1, Relaxed);
    }
    mb
}

/// `nfs_getreq(nd, nfsd, has_header)`: parses the RPC request `nd` holds, from its cursor
/// (the transaction id and the call word first when `has_header`): verifies it and fills in
/// the credentials. A request of another RPC version, program or NFS version, an unknown
/// procedure and a credential other than `AUTH_UNIX` are answers (`nd_repstat`, the
/// procedure `NFSPROC_NOOP`), not errors. Every length from the wire is compared as the C
/// compares it before it is a size; a bad one, or the request's end met first, is
/// `EBADRPC`, the request freed.
pub fn nfs_getreq(nd: &mut NfsrvDescript, has_header: bool) -> Result<(), Errno> {
    let r = getreq(nd, has_header);
    if r.is_err() {
        nd.nd_mrep = None;
    }
    r
}

/// `nfs_getreq`'s body; its caller frees the request on an error.
fn getreq(nd: &mut NfsrvDescript, has_header: bool) -> Result<(), Errno> {
    let mut r = nd_reader(&nd.nd_mrep, nd.nd_off)?;
    let mut tl = [0u32; 10];
    let words = if has_header { 10 } else { 8 };
    for w in &mut tl[..words] {
        *w = r.word()?;
    }
    let mut tl = &tl[..words];
    if has_header {
        nd.nd_retxid = u32::from_be(tl[0]);
        if tl[1] != rpc_call {
            return Err(Errno::EBADRPC);
        }
        tl = &tl[2..];
    }
    // tl: rpcvers, prog, vers, proc, the credential's flavor and length, the stamp, the
    // machine name's length.
    nd.nd_off = r.offset();
    nd.nd_repstat = 0;
    nd.nd_flag = 0;
    if tl[0] != rpc_vers {
        nd.nd_repstat = Errno::ERPCMISMATCH.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    if tl[1] != nfs_prog {
        nd.nd_repstat = Errno::EPROGUNAVAIL.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    let nfsvers = u32::from_be(tl[2]);
    if nfsvers != NFS_VER2 && nfsvers != NFS_VER3 {
        nd.nd_repstat = Errno::EPROGMISMATCH.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    if nfsvers == NFS_VER3 {
        nd.nd_flag = ND_NFSV3;
    }
    let procnum = u32::from_be(tl[3]) as usize;
    nd.nd_procnum = procnum;
    if procnum == NFSPROC_NULL {
        return Ok(());
    }
    if procnum >= NFS_NPROCS
        || procnum > NFSPROC_COMMIT
        || (nd.nd_flag == 0 && procnum > NFSV2PROC_STATFS)
    {
        nd.nd_repstat = Errno::EPROCUNAVAIL.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    if nd.nd_flag & ND_NFSV3 == 0 {
        nd.nd_procnum = NFSV3_PROCID[procnum];
    }
    let auth_type = tl[4];
    let len = i32::from_be_bytes(tl[5].to_ne_bytes());
    if len < 0 || len as usize > RPCAUTH_MAXSIZ {
        return Err(Errno::EBADRPC);
    }

    // Handle auth_unix
    if auth_type != rpc_auth_unix {
        nd.nd_repstat = NFSERR_AUTHERR | AUTH_REJECTCRED as i32;
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    let len = i32::from_be_bytes(tl[7].to_ne_bytes());
    if len < 0 || len as usize > NFS_MAXNAMLEN {
        return Err(Errno::EBADRPC);
    }
    r.adv(nfsm_rndup(len as usize))?;
    let uid = r.u32()?;
    let gid = r.u32()?;
    let len = r.i32()?;
    if len < 0 || len as usize > RPCAUTH_UNIXGIDS {
        return Err(Errno::EBADRPC);
    }
    // The C dissects the groups and the verifier's two words as one block first.
    let n = len as usize;
    let mut block = [0u32; RPCAUTH_UNIXGIDS + 2];
    for w in &mut block[..n + 2] {
        *w = r.word()?;
    }
    nd.nd_cr = UcredCell::new();
    nd.nd_cr.set_uid(uid);
    nd.nd_cr.set_gid(gid);
    for (i, g) in block[..n].iter().enumerate() {
        if i < NGROUPS_MAX {
            nd.nd_cr.set_group(i, u32::from_be(*g));
        }
    }
    // At most NGROUPS_MAX (16): an `i16`.
    nd.nd_cr.set_ngroups(n.min(NGROUPS_MAX) as i16);
    // block[n] is the verifier's flavor, block[n + 1] its length.
    let len = i32::from_be_bytes(block[n + 1].to_ne_bytes());
    if len < 0 || len as usize > RPCAUTH_MAXSIZ {
        return Err(Errno::EBADRPC);
    }
    if len > 0 {
        r.adv(nfsm_rndup(len as usize))?;
    }
    nd.nd_off = r.offset();
    Ok(())
}

/// `nfsrv_rcv(so, arg, waitflag)`: the socket upcall routine of the nfsd sockets, and the
/// nfsds' own call with `M_WAIT`. Essentially do as much as possible non-blocking, else
/// punt and it will be called with `M_WAIT` from an nfsd: the upcall (`M_DONTWAIT`) only
/// marks the socket `SLP_NEEDQ` and wakes an nfsd; an nfsd's call receives what the socket
/// holds, cuts a stream into records or queues each datagram with its sender, and marks the
/// socket `SLP_DISCONN` when the connection is gone.
pub fn nfsrv_rcv<G: ProcGlue>(so: &Socket<G>, slp: &NfssvcSock<G>, waitflag: i32) {
    klock::kernel_lock();
    rcv(so, slp, waitflag);
    klock::kernel_unlock();
}

/// [`nfsrv_rcv`] as the socket layer's registered upcall (`so->so_upcall`, which
/// `nfssvc_addsock` sets): `arg` is the socket's record, the counted handle it stored in
/// `so_upcallarg`, taken back by naming its type (O(1)); any other argument is not nfs's,
/// and nothing is done.
pub fn nfsrv_rcv_upcall<G: ProcGlue>(so: &Socket<G>, arg: &AnyArc, waitflag: i32) {
    if let Some(slp) = arg.downcast_ref::<NfssvcSock<G>>() {
        nfsrv_rcv(so, slp, waitflag);
    }
}

/// `nfsrv_rcv`'s body, under the kernel lock.
fn rcv<G: ProcGlue>(so: &Socket<G>, slp: &NfssvcSock<G>, waitflag: i32) {
    if slp.flag() & SLP_VALID == 0 {
        return;
    }

    if waitflag == M_DONTWAIT {
        // Defer soreceive() to an nfsd.
        slp.set_flags(SLP_NEEDQ);
    } else if so.so_type() == SOCK_STREAM {
        rcv_stream(so, slp, waitflag);
    } else {
        rcv_dgram(so, slp);
    }

    // Now try and process the request records, non-blocking.
    if waitflag == M_DONTWAIT
        && (slp.ns_rcv.update(|r| !r.ns_rec.is_empty())
            || slp.flag() & (SLP_NEEDQ | SLP_DISCONN) != 0)
    {
        nfsrv_wakenfsd(slp);
    }
}

/// `nfsrv_rcv` on a stream socket: the bytes the socket holds appended to the stream, and the
/// records they complete cut off it (`nfsrv_getstream`).
fn rcv_stream<G: ProcGlue>(so: &Socket<G>, slp: &NfssvcSock<G>, waitflag: i32) {
    // Do soreceive().
    let mut auio = count_uio::<G>(RCV_ALL, None);
    let mut flags = MSG_DONTWAIT;
    let mut mp = None;
    let error = soreceive(
        so,
        None,
        &mut auio,
        Some(&mut mp),
        None,
        Some(&mut flags),
        0,
    );
    let Some(m) = mp.filter(|_| error.is_ok()) else {
        if error == Err(Errno::EWOULDBLOCK) {
            slp.set_flags(SLP_NEEDQ);
        } else {
            slp.set_flags(SLP_DISCONN);
        }
        return;
    };
    let got = i32::try_from(RCV_ALL - auio.uio_resid).unwrap_or(i32::MAX);

    // The stream's state taken out for the parse, which may sleep (`m_copym(M_WAIT)`).
    let mut st = slp.ns_rcv.update(|r| NsRcv {
        ns_raw: mem::take(&mut r.ns_raw),
        ns_frag: mem::take(&mut r.ns_frag),
        ns_rec: MbufList::new(),
        ns_cc: r.ns_cc,
        ns_reclen: r.ns_reclen,
    });
    append_chain(&mut st.ns_raw, m);
    st.ns_cc = st.ns_cc.saturating_add(got);

    // Now try and parse record(s) out of the raw stream data.
    let error = nfsrv_getstream(slp, &mut st, waitflag);

    // Back in the slot with the records cut, unless the socket was zapped meanwhile (it
    // emptied the slot and cleared SLP_VALID): then what the parse held is freed here.
    let stale = slp.ns_rcv.update(|r| {
        if slp.flag() & SLP_VALID == 0 {
            return Some(st);
        }
        // `ns_solock` keeps a second receive out while the state is taken out: nothing was
        // put in its place meanwhile.
        kassert!(r.ns_raw.is_empty() && r.ns_frag.is_empty());
        r.ns_raw = mem::take(&mut st.ns_raw);
        r.ns_frag = mem::take(&mut st.ns_frag);
        r.ns_cc = st.ns_cc;
        r.ns_reclen = st.ns_reclen;
        r.ns_rec.concat(&mut st.ns_rec);
        None
    });
    drop(stale);
    if let Err(e) = error {
        if e == Errno::EPERM {
            slp.set_flags(SLP_DISCONN);
        } else {
            slp.set_flags(SLP_NEEDQ);
        }
    }
}

/// `nfsrv_rcv` on a datagram socket: each datagram queued as a record, its sender's name
/// first, until the socket holds none.
fn rcv_dgram<G: ProcGlue>(so: &Socket<G>, slp: &NfssvcSock<G>) {
    loop {
        let mut auio = count_uio::<G>(RCV_ALL, None);
        let mut flags = MSG_DONTWAIT;
        let mut nam = None;
        let mut mp = None;
        let error = soreceive(
            so,
            Some(&mut nam),
            &mut auio,
            Some(&mut mp),
            None,
            Some(&mut flags),
            0,
        );
        let got = mp.is_some();
        if let Some(mp) = mp {
            let rec = match nam {
                Some(mut n) => {
                    drop(n.m_next.set(mp));
                    n
                }
                None => mp,
            };
            slp.ns_rcv.update(|r| r.ns_rec.push_tail(rec));
        }
        if let Err(e) = error
            && so.proto().flags() & PR_CONNREQUIRED != 0
            && e != Errno::EWOULDBLOCK
        {
            slp.set_flags(SLP_DISCONN);
            return;
        }
        if !got {
            return;
        }
    }
}

/// The chain `m` appended to the stream `raw`, mbuf by mbuf (the C links the chain at
/// `ns_rawend` and walks it to its end for the next `ns_rawend`: the same walk).
fn append_chain(raw: &mut OwnedSlist<MNext>, m: MbufBox) {
    let mut next = Some(m);
    while let Some(mut b) = next {
        next = b.m_next.take();
        raw.push_tail(b);
    }
}

/// `nfsrv_getstream(slp, waitflag)`: tries and extracts RPC requests from the mbuf data
/// list received on a stream socket (`st`, the socket's stream taken out by `nfsrv_rcv`):
/// each record mark read, a record too long for NFS refused (`EPERM`: the connection is
/// dropped), each fragment cut off the stream and joined to its record, each record whose
/// last fragment came queued on `st.ns_rec`. `waitflag` says whether a cut may sleep
/// (`EWOULDBLOCK` when it may not and no mbuf is to be had).
pub fn nfsrv_getstream<G: ProcGlue>(
    slp: &NfssvcSock<G>,
    st: &mut NsRcv,
    waitflag: i32,
) -> Result<(), Errno> {
    if slp.flag() & SLP_GETSTREAM != 0 {
        return Ok(());
    }
    slp.set_flags(SLP_GETSTREAM);
    let r = getstream(slp, st, waitflag);
    slp.clear_flags(SLP_GETSTREAM);
    r
}

/// `nfsrv_getstream`'s loop, `SLP_GETSTREAM` held.
fn getstream<G: ProcGlue>(slp: &NfssvcSock<G>, st: &mut NsRcv, waitflag: i32) -> Result<(), Errno> {
    loop {
        if st.ns_reclen == 0 {
            if st.ns_cc < NFSX_UNSIGNED as i32 {
                return Ok(());
            }
            let recmark = u32::from_be_bytes(take_mark(&st.ns_raw));
            st.ns_cc -= NFSX_UNSIGNED as i32;
            st.ns_reclen = (recmark & !LASTFRAG_BIT) as i32;
            if recmark & LASTFRAG_BIT != 0 {
                slp.set_flags(SLP_LASTFRAG);
            } else {
                slp.clear_flags(SLP_LASTFRAG);
            }
            if st.ns_reclen as usize > NFS_MAXPACKET {
                return Err(Errno::EPERM);
            }
        }

        // Now get the record part.
        let mut recm = OwnedSlist::<MNext>::new();
        match st.ns_cc.cmp(&st.ns_reclen) {
            core::cmp::Ordering::Equal => {
                recm = mem::take(&mut st.ns_raw);
                st.ns_cc = 0;
                st.ns_reclen = 0;
            }
            core::cmp::Ordering::Greater => {
                let reclen = st.ns_reclen as u32;
                let mut len: u32 = 0;
                while len < reclen {
                    let Some(mut m) = st.ns_raw.pop_head() else {
                        break;
                    };
                    let ml = m.m_len().get();
                    if len + ml > reclen {
                        let want = reclen - len;
                        let Some(m2) = m_copym(&mut m, 0, want as i32, waitflag) else {
                            // Nothing cut: the stream as it was.
                            st.ns_raw.push_head(m);
                            recm.concat(&mut st.ns_raw);
                            st.ns_raw = recm;
                            return Err(Errno::EWOULDBLOCK);
                        };
                        m.data_add(want as isize);
                        m.m_len().set(ml - want);
                        st.ns_raw.push_head(m);
                        append_chain(&mut recm, m2);
                        len = reclen;
                    } else {
                        len += ml;
                        recm.push_tail(m);
                    }
                }
                st.ns_cc -= len as i32;
                st.ns_reclen = 0;
            }
            core::cmp::Ordering::Less => return Ok(()),
        }

        // Accumulate the fragments into a record.
        st.ns_frag.concat(&mut recm);
        if slp.flag() & SLP_LASTFRAG != 0
            && let Some(rec) = mem::take(&mut st.ns_frag).into_head()
        {
            st.ns_rec.push_tail(rec);
        }
    }
}

/// The record mark at the head of the stream `raw`, its four bytes consumed from the mbufs
/// they lie in (`m_data += n; m_len -= n`, empty mbufs passed over), as the C copies it; the
/// caller made sure the stream holds four bytes (`ns_cc`).
fn take_mark(raw: &OwnedSlist<MNext>) -> [u8; NFSX_UNSIGNED] {
    let mut mark = [0u8; NFSX_UNSIGNED];
    let mut got = 0;
    let mut m = raw.first();
    while got < NFSX_UNSIGNED {
        let Some(cur) = m else {
            kpanic(format_args!(
                "nfsrv_getstream: ns_cc counts bytes ns_raw lacks"
            ));
        };
        let d = cur.data();
        let n = d.len().min(NFSX_UNSIGNED - got);
        mark[got..got + n].copy_from_slice(&d[..n]);
        if n > 0 {
            cur.data_add(n as isize);
            cur.m_len().set(cur.m_len().get() - n as u32);
            got += n;
        }
        if got < NFSX_UNSIGNED {
            m = OwnedSlist::<MNext>::next(cur);
        }
    }
    mark
}

/// `nfsrv_dorec(slp, nfsd, ndp)`: the socket's first queued record taken for an nfsd and its
/// RPC header parsed (`nfs_getreq`): the descriptor, which the nfsd owns. `ENOBUFS` when the
/// socket is not valid or holds no record; `nfs_getreq`'s error, the record freed.
pub fn nfsrv_dorec<G: ProcGlue>(slp: &NfssvcSock<G>) -> Result<PoolBox<NfsrvDescript>, Errno> {
    let rec = slp.ns_rcv.update(|r| {
        if slp.flag() & SLP_VALID == 0 {
            return None;
        }
        r.ns_rec.pop_head()
    });
    let Some(mut m) = rec else {
        return Err(Errno::ENOBUFS);
    };
    let (mreq, nam) = if i32::from(m.m_type().get()) == MT_SONAME {
        let data = m.m_next.take();
        (data, Some(m))
    } else {
        (Some(m), None)
    };
    let Some(mut nd) = vfs::<G>()
        .nfsrv
        .nfsrv_descript_pl
        .get(PR_WAITOK, NfsrvDescript::default)
    else {
        return Err(Errno::ENOBUFS);
    };
    nd.nd_mrep = mreq;
    nd.nd_off = 0;
    nd.nd_nam2 = nam;
    nfs_getreq(&mut nd, true)?;
    Ok(nd)
}

/// `nfsrv_wakenfsd(slp)`: searches for a sleeping nfsd and wakes it up, handing it `slp`
/// (one more reference: `ns_sref`, and the handle the socket's list membership holds,
/// cloned). If none is found, sets `NFSD_CHECKSLP`, so that one of the running nfsds will go
/// look for the work in the socket list, and marks the socket `SLP_DOREC`.
pub fn nfsrv_wakenfsd<G: ProcGlue>(slp: &NfssvcSock<G>) {
    if slp.flag() & SLP_VALID == 0 {
        return;
    }
    let t = &vfs::<G>().nfsrv;
    let r = klock::reader();
    for nfsd in t.nfsd_head.iter(&r) {
        if nfsd.flag() & NFSD_WAITING == 0 {
            continue;
        }
        nfsd.nfsd_flag.fetch_and(!NFSD_WAITING, Relaxed);
        let Some(me) = slp.ns_k.get(&r).ns_self.clone() else {
            kpanic(format_args!("nfsrv_wakenfsd: a valid socket on no list"));
        };
        slp.ns_sref.fetch_add(1, Relaxed);
        if nfsd.nfsd_slp.replace(Some(me)).is_some() {
            kpanic(format_args!("nfsd wakeup"));
        }
        wakeup_one(core::ptr::from_ref(nfsd));
        return;
    }
    drop(r);
    slp.set_flags(SLP_DOREC);
    t.nfsd_head_flag.fetch_or(NFSD_CHECKSLP, Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the server's RPC layer: the reply header of every status (LZ's
    // `rephead_builds_the_reply_header`), a call's header and credential, version 2's
    // procedure map and the answers that are not errors (LZ's
    // `getreq_parses_an_auth_unix_call`), every bound of the header (each its own case, then
    // a seeded property test over arbitrary words and chain splits: never a panic, never a
    // read past the chain), the stream's records and fragments (LZ's
    // `getstream_splits_records_and_joins_fragments`, then over every split of the bytes into
    // mbufs and the mark's bound), a record taken for an nfsd, and the wakeup of an idle
    // nfsd.

    use std::boxed::Box;
    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::fs::nfs::nfsm_subs::{TestRng, test_bytes, test_chain};
    use crate::fs::nfs::nfsproto::NFSPROC_GETATTR;
    use crate::fs::nfs::nfssvc::{Nfsd, NfssvcSockRef};
    use crate::fs::testfs::TestGlue;
    use crate::net::mbuf::m_devget;
    use crate::sys::nfs_rpcv2::AUTH_BADCRED;

    type G = TestGlue;

    fn wire(w: &[u32]) -> Vec<u8> {
        w.iter().flat_map(|x| x.to_be_bytes()).collect()
    }

    fn mksock() -> NfssvcSock<G> {
        NfssvcSock::new(None)
    }

    #[test]
    fn rephead_builds_the_reply_header() {
        let _g = crate::net::mbuf::tests::setup();
        let s = mksock();
        let mut nd = NfsrvDescript::default();
        nd.nd_retxid = 0x1234;
        let xid = 0x1234u32;
        let reply =
            |nd: &NfsrvDescript, err: i32| test_bytes(&nfs_rephead(0, nd, &s, err).into_chain());

        // Success: accepted, null verifier, status 0, then the NFS status word 0.
        assert_eq!(reply(&nd, 0), wire(&[xid, 1, 0, 0, 0, 0, 0]));
        // A void reply has no NFS status word.
        assert_eq!(reply(&nd, NFSERR_RETVOID), wire(&[xid, 1, 0, 0, 0, 0]));
        let e = |x: Errno| x.as_i32();
        assert_eq!(
            reply(&nd, e(Errno::EPROGUNAVAIL)),
            wire(&[xid, 1, 0, 0, 0, 1])
        );
        assert_eq!(
            reply(&nd, e(Errno::EPROGMISMATCH)),
            wire(&[xid, 1, 0, 0, 0, 2, 2, 3])
        );
        assert_eq!(
            reply(&nd, e(Errno::EPROCUNAVAIL)),
            wire(&[xid, 1, 0, 0, 0, 3])
        );
        assert_eq!(reply(&nd, e(Errno::EBADRPC)), wire(&[xid, 1, 0, 0, 0, 4]));
        // An errno through the version 2 map (ENOENT is NFSERR_NOENT).
        assert_eq!(reply(&nd, e(Errno::ENOENT)), wire(&[xid, 1, 0, 0, 0, 0, 2]));
        // Denied: RPC version mismatch (low and high version), or an authentication error.
        assert_eq!(
            reply(&nd, e(Errno::ERPCMISMATCH)),
            wire(&[xid, 1, 1, 0, 2, 2])
        );
        assert_eq!(
            reply(&nd, NFSERR_AUTHERR | AUTH_BADCRED as i32),
            wire(&[xid, 1, 1, 1, 1])
        );
        // A big reply starts in a cluster; a small one leaves the headers' room.
        let big = nfs_rephead(MHLEN, &nd, &s, 0).into_chain();
        assert!(big.ext().is_some());
        let small = nfs_rephead(0, &nd, &s, 0).into_chain();
        assert!(small.ext().is_none());
        assert!(small.data_off() >= usize::try_from(MAX_HDR.load(Relaxed)).unwrap_or(0));
    }

    /// A version `vers` call of procedure `proc_` with an AUTH_UNIX credential of uid 1000,
    /// gid 10 and `groups`, and a verifier of `verf` bytes, as its words.
    fn call_bytes(vers: u32, proc_: u32, groups: &[u32], verf: u32) -> Vec<u8> {
        let mut w = vec![77, 0, 2, 100_003, vers, proc_, 1];
        let cred_len = 5 * 4 + 8 + 4 * groups.len() as u32;
        w.extend([cred_len, 0, 5]);
        let mut b = wire(&w);
        b.extend(b"host\0\0\0\0");
        let mut w = vec![1000, 10, groups.len() as u32];
        w.extend(groups);
        w.extend([0, verf]);
        b.extend(wire(&w));
        b.extend(vec![0u8; nfsm_rndup(verf as usize)]);
        b
    }

    /// A descriptor of the request `b`, cut into `cut`-byte mbufs.
    fn descript(b: &[u8], cut: usize) -> NfsrvDescript {
        let mut parts: Vec<&[u8]> = b.chunks(cut.max(1)).collect();
        if parts.is_empty() {
            parts.push(&[]);
        }
        NfsrvDescript {
            nd_mrep: Some(test_chain(&parts)),
            ..NfsrvDescript::default()
        }
    }

    #[test]
    fn getreq_parses_an_auth_unix_call() {
        let _g = crate::net::mbuf::tests::setup();
        let mut nd = descript(&call_bytes(3, 1, &[20, 30], 8), 12);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_retxid, 77);
        assert_eq!(nd.nd_repstat, 0);
        assert_eq!(nd.nd_flag, ND_NFSV3);
        assert_eq!(nd.nd_procnum, NFSPROC_GETATTR);
        assert_eq!(nd.nd_cr.cr_uid.get(), 1000);
        assert_eq!(nd.nd_cr.cr_gid.get(), 10);
        assert_eq!(nd.nd_cr.cr_ngroups.get(), 2);
        assert_eq!(nd.nd_cr.cr_groups[1].get(), 30);
        // The cursor stands after the verifier.
        assert_eq!(nd.nd_off, call_bytes(3, 1, &[20, 30], 8).len());

        // Version 2 procedure numbers are mapped to version 3's (v2 STATFS 17 is FSSTAT 18).
        let mut nd = descript(&call_bytes(2, 17, &[], 0), 7);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_flag, 0);
        assert_eq!(nd.nd_procnum, NFSV3_PROCID[17]);

        // RPC errors are reported in nd_repstat with the NOOP procedure.
        let mut nd = descript(&call_bytes(4, 1, &[], 0), 40);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_repstat, Errno::EPROGMISMATCH.as_i32());
        assert_eq!(nd.nd_procnum, NFSPROC_NOOP);
        let mut nd = descript(&call_bytes(2, 18, &[], 0), 40);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_repstat, Errno::EPROCUNAVAIL.as_i32());
        let mut b = call_bytes(3, 1, &[], 0);
        b[11] = 3; // RPC version 3
        let mut nd = descript(&b, 40);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_repstat, Errno::ERPCMISMATCH.as_i32());
        let mut b = call_bytes(3, 1, &[], 0);
        b[15] = 4; // program 100004
        let mut nd = descript(&b, 40);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_repstat, Errno::EPROGUNAVAIL.as_i32());
        // An AUTH_NULL credential is refused by an answer.
        let mut b = call_bytes(3, 1, &[], 0);
        b[27] = 0;
        let mut nd = descript(&b, 40);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_repstat, NFSERR_AUTHERR | AUTH_REJECTCRED as i32);
        // The NULL procedure is not looked further at.
        let mut nd = descript(&call_bytes(3, 0, &[], 0)[..40], 40);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_procnum, NFSPROC_NULL);

        // A reply instead of a call is garbage; the request is freed.
        let mut b = call_bytes(3, 1, &[], 0);
        b[7] = 1;
        let mut nd = descript(&b, 40);
        assert_eq!(nfs_getreq(&mut nd, true), Err(Errno::EBADRPC));
        assert!(nd.nd_mrep.is_none());
    }

    #[test]
    fn each_length_of_the_header_is_bounded_as_the_c_bounds_it() {
        let _g = crate::net::mbuf::tests::setup();
        // (word index, value, accepted): the credential's length (word 7), the machine name's
        // (word 9), the group count (after the name), the verifier's length (last word).
        let base = call_bytes(3, 1, &[], 0);
        let set =
            |b: &mut Vec<u8>, at: usize, v: i32| b[at..at + 4].copy_from_slice(&v.to_be_bytes());
        let name_at = 36;
        let ngroups_at = 40 + 8 + 8;
        let verf_at = ngroups_at + 8;
        let cases: &[(usize, i32, bool)] = &[
            (28, RPCAUTH_MAXSIZ as i32, true),
            (28, RPCAUTH_MAXSIZ as i32 + 1, false),
            (28, -1, false),
            (name_at, NFS_MAXNAMLEN as i32 + 1, false),
            (name_at, -1, false),
            (ngroups_at, RPCAUTH_UNIXGIDS as i32 + 1, false),
            (ngroups_at, -1, false),
            (verf_at, RPCAUTH_MAXSIZ as i32 + 1, false),
            (verf_at, -1, false),
        ];
        for &(at, v, ok) in cases {
            let mut b = base.clone();
            set(&mut b, at, v);
            b.extend(vec![0u8; 512]);
            let mut nd = descript(&b, 9);
            let r = nfs_getreq(&mut nd, true);
            assert_eq!(r.is_ok(), ok, "word at {at} = {v}: {r:?}");
            if !ok {
                assert_eq!(r, Err(Errno::EBADRPC));
                assert!(nd.nd_mrep.is_none());
            }
        }
        // A name of NFS_MAXNAMLEN bytes is taken (then the request ends: EBADRPC after it).
        let mut b = base.clone();
        set(&mut b, name_at, NFS_MAXNAMLEN as i32);
        let mut nd = descript(&b, 40);
        assert_eq!(nfs_getreq(&mut nd, true), Err(Errno::EBADRPC));
        // Sixteen groups are the most a credential carries, all kept.
        let g: Vec<u32> = (100..116).collect();
        let mut nd = descript(&call_bytes(3, 1, &g, 4), 5);
        assert_eq!(nfs_getreq(&mut nd, true), Ok(()));
        assert_eq!(nd.nd_cr.cr_ngroups.get(), 16);
        assert_eq!(nd.nd_cr.cr_groups[15].get(), 115);
        // Every cut of a good call is EBADRPC (the 40 header bytes are one dissection).
        let full = call_bytes(3, 1, &[7, 8], 8);
        for cut in 0..full.len() {
            let mut nd = descript(&full[..cut], 3);
            assert_eq!(nfs_getreq(&mut nd, true), Err(Errno::EBADRPC), "cut {cut}");
        }
    }

    #[test]
    fn arbitrary_headers_never_panic_or_read_past_the_chain() {
        let _g = crate::net::mbuf::tests::setup();
        let mut rng = TestRng(0x6e66_7372_7631_7632);
        for case in 0..4000 {
            let mut b = call_bytes(
                2 + (rng.next_u64() % 2) as u32,
                (rng.next_u64() % 24) as u32,
                &[1, 2],
                4,
            );
            // Overwrite a few words with arbitrary (often bounding) values.
            for _ in 0..(rng.next_u64() % 4) {
                let at = (rng.next_u64() as usize % (b.len() / 4)) * 4;
                let v = match rng.next_u64() % 4 {
                    0 => rng.next_u64() as u32,
                    1 => (rng.next_u64() % 600) as u32,
                    2 => u32::MAX,
                    _ => 0,
                };
                b[at..at + 4].copy_from_slice(&v.to_be_bytes());
            }
            let cut = b.len() - (rng.next_u64() as usize % (b.len() + 1));
            let split = 1 + rng.next_u64() as usize % 17;
            let total = cut;
            let mut nd = descript(&b[..cut], split);
            match nfs_getreq(&mut nd, true) {
                Ok(()) => assert!(nd.nd_off <= total, "case {case}: cursor past the chain"),
                Err(e) => {
                    assert_eq!(e, Errno::EBADRPC, "case {case}");
                    assert!(nd.nd_mrep.is_none());
                }
            }
        }
    }

    /// The stream `s` in a receive state, cut into `cut`-byte mbufs.
    fn stream(s: &[u8], cut: usize) -> NsRcv {
        let mut st = NsRcv::default();
        let parts: Vec<&[u8]> = s.chunks(cut.max(1)).collect();
        append_chain(&mut st.ns_raw, test_chain(&parts));
        st.ns_cc = s.len() as i32;
        st
    }

    fn records(st: &NsRcv) -> Vec<Vec<u8>> {
        let mut v = Vec::new();
        let mut r = st.ns_rec.first();
        while let Some(m) = r {
            v.push(test_bytes(m));
            r = MbufList::next(m);
        }
        v
    }

    fn raw_bytes(st: &NsRcv) -> Vec<u8> {
        st.ns_raw.iter().flat_map(|m| m.data().to_vec()).collect()
    }

    #[test]
    fn getstream_splits_records_and_joins_fragments() {
        let _g = crate::net::mbuf::tests::setup();
        let rec1: Vec<u8> = (1..=8).collect();
        let frag_a: Vec<u8> = (11..=14).collect();
        let frag_b: Vec<u8> = (15..=18).collect();
        let mut s = Vec::new();
        s.extend(0x8000_0008u32.to_be_bytes());
        s.extend(&rec1);
        s.extend(4u32.to_be_bytes()); // not the last fragment
        s.extend(&frag_a);
        s.extend(0x8000_0004u32.to_be_bytes());
        s.extend(&frag_b);
        s.extend(0x8000_000cu32.to_be_bytes());
        s.extend([21, 22, 23, 24, 25]); // 5 of 12 bytes so far
        let mut rec2 = frag_a.clone();
        rec2.extend(&frag_b);

        // Every split of the stream into mbufs gives the same records.
        for cut in 1..=s.len() {
            let slp = mksock();
            let mut st = stream(&s, cut);
            assert_eq!(nfsrv_getstream(&slp, &mut st, M_WAIT), Ok(()), "cut {cut}");
            assert_eq!(slp.flag() & SLP_GETSTREAM, 0);
            assert_eq!(records(&st), [rec1.clone(), rec2.clone()], "cut {cut}");
            assert!(st.ns_frag.is_empty());
            assert_eq!(st.ns_reclen, 12);
            assert_eq!(st.ns_cc, 5);
            assert_eq!(raw_bytes(&st), [21, 22, 23, 24, 25]);
            assert_ne!(slp.flag() & SLP_LASTFRAG, 0);
        }

        // The record mark of a record too big for NFS disconnects; NFS_MAXPACKET is taken.
        let slp = mksock();
        let big = (0x8000_0000u32 | (NFS_MAXPACKET as u32 + 1)).to_be_bytes();
        let mut st = stream(&big, 2);
        assert_eq!(nfsrv_getstream(&slp, &mut st, M_WAIT), Err(Errno::EPERM));
        assert_eq!(slp.flag() & SLP_GETSTREAM, 0);
        let max = (0x8000_0000u32 | NFS_MAXPACKET as u32).to_be_bytes();
        let mut st = stream(&max, 3);
        assert_eq!(nfsrv_getstream(&slp, &mut st, M_WAIT), Ok(()));
        assert_eq!(st.ns_reclen, NFS_MAXPACKET as i32);
        // A last fragment of length 0 with more of the stream after it: no record for it,
        // and the records before and after it kept (the C loses the ones before).
        let slp = mksock();
        let mut st = Vec::new();
        st.extend(0x8000_0003u32.to_be_bytes());
        st.extend([1, 2, 3]);
        st.extend(0x8000_0000u32.to_be_bytes());
        st.extend(0x8000_0002u32.to_be_bytes());
        st.extend([4, 5]);
        let mut st = stream(&st, 5);
        assert_eq!(nfsrv_getstream(&slp, &mut st, M_WAIT), Ok(()));
        assert_eq!(records(&st), [vec![1, 2, 3], vec![4, 5]]);
        assert!(st.ns_frag.is_empty());
        // A parse already running (SLP_GETSTREAM) is not entered again.
        slp.set_flags(SLP_GETSTREAM);
        let mut st = stream(&s, 4);
        assert_eq!(nfsrv_getstream(&slp, &mut st, M_WAIT), Ok(()));
        assert!(st.ns_rec.is_empty());
        // A last fragment of length 0 with nothing before it: a record of the mark's emptied
        // mbuf, as the C queues it (its parse is then EBADRPC).
        let slp = mksock();
        let mut st = stream(&0x8000_0000u32.to_be_bytes(), 4);
        assert_eq!(nfsrv_getstream(&slp, &mut st, M_WAIT), Ok(()));
        assert_eq!(records(&st), [Vec::<u8>::new()]);
    }

    #[test]
    fn arbitrary_streams_are_cut_into_the_records_their_marks_say() {
        let _g = crate::net::mbuf::tests::setup();
        let mut rng = TestRng(0x7374_7265_616d_7321);
        for case in 0..600 {
            // A model: records of random fragments, each fragment a mark and its bytes.
            let mut s = Vec::new();
            let mut want = Vec::new();
            for _ in 0..(1 + rng.next_u64() % 4) {
                let mut rec = Vec::new();
                let nfrag = 1 + rng.next_u64() % 3;
                for f in 0..nfrag {
                    let n = (rng.next_u64() % 40) as usize;
                    let bytes: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
                    let last = if f + 1 == nfrag { LASTFRAG_BIT } else { 0 };
                    s.extend((last | n as u32).to_be_bytes());
                    s.extend(&bytes);
                    rec.extend(bytes);
                }
                want.push(rec);
            }
            let split = 1 + rng.next_u64() as usize % 13;
            let slp = mksock();
            let mut st = stream(&s, split);
            assert_eq!(
                nfsrv_getstream(&slp, &mut st, M_WAIT),
                Ok(()),
                "case {case}"
            );
            // A record whose fragments are all empty queues an empty-mbuf record or none.
            let got: Vec<Vec<u8>> = records(&st);
            let want: Vec<Vec<u8>> = want.into_iter().filter(|r| !r.is_empty()).collect();
            let got: Vec<Vec<u8>> = got.into_iter().filter(|r| !r.is_empty()).collect();
            assert_eq!(got, want, "case {case}");
            assert_eq!(st.ns_cc, 0);
            assert!(raw_bytes(&st).is_empty());
        }
    }

    /// The tables' pool made for `G` (once; a later test's call keeps it).
    fn pools() {
        let _ =
            vfs::<G>()
                .nfsrv
                .nfsrv_descript_pl
                .init(hal::cpu::intr::IPL_NONE, PR_WAITOK, "ndscpl");
    }

    #[test]
    fn dorec_takes_a_datagram_with_its_sender_and_parses_it() {
        let _g = crate::net::mbuf::tests::setup();
        pools();
        let s = mksock();
        s.set_flags(SLP_VALID);
        assert_eq!(nfsrv_dorec(&s).err(), Some(Errno::ENOBUFS));
        let sin = [16u8, 2, 3, 4, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut nam = m_devget(&sin, 0).expect("a name");
        nam.m_type().set(MT_SONAME as i16);
        drop(nam.m_next.set(test_chain(&[&call_bytes(3, 1, &[], 0)])));
        let bad = test_chain(&[&[0u8; 12]]);
        s.ns_rcv.update(|r| {
            r.ns_rec.push_tail(nam);
            r.ns_rec.push_tail(bad);
        });
        let nd = nfsrv_dorec(&s).expect("a request");
        assert_eq!(nd.nd_procnum, NFSPROC_GETATTR);
        assert_eq!(nd.nam().map(|m| m.data()[2]), Some(3));
        assert!(nd.nd_nam2.is_some());
        // A short record without a name: the parse's error, the record gone.
        assert_eq!(nfsrv_dorec(&s).err(), Some(Errno::EBADRPC));
        assert!(s.ns_rcv.update(|r| r.ns_rec.is_empty()));
        // A socket no longer valid gives nothing.
        s.ns_rcv
            .update(|r| r.ns_rec.push_tail(test_chain(&[&[0u8; 4]])));
        s.clear_flags(SLP_VALID);
        assert_eq!(nfsrv_dorec(&s).err(), Some(Errno::ENOBUFS));
    }

    #[test]
    fn wakenfsd_hands_the_socket_to_a_waiting_nfsd_or_marks_it() {
        let _g = crate::net::mbuf::tests::setup();
        hal::sync::klock::kernel_lock();
        let t = &vfs::<G>().nfsrv;
        let s: NfssvcSockRef<G> = hal::mem::malloc::MArc::new(
            super::super::nfssvc::M_NFSSVC,
            hal::mem::malloc::M_WAITOK,
            mksock(),
        )
        .expect("a socket");
        s.set_flags(SLP_VALID);
        {
            let mut w = klock::witness();
            s.ns_k.get_mut(&mut w).ns_self = Some(s.clone());
        }
        // No nfsd: the socket is marked for the next one that looks.
        nfsrv_wakenfsd(&s);
        assert_ne!(s.flag() & SLP_DOREC, 0);
        assert_ne!(t.nfsd_head_flag.load(Relaxed) & NFSD_CHECKSLP, 0);
        s.clear_flags(SLP_DOREC);
        t.nfsd_head_flag.store(0, Relaxed);

        // A waiting nfsd gets it, counted.
        let nfsd = Box::pin(Nfsd::<G>::new(7));
        nfsd.nfsd_flag.store(NFSD_WAITING, Relaxed);
        {
            let mut w = klock::witness();
            t.nfsd_head.insert_tail(&mut w, nfsd.as_ref());
        }
        nfsrv_wakenfsd(&s);
        assert_eq!(nfsd.flag() & NFSD_WAITING, 0);
        assert_eq!(s.ns_sref.load(Relaxed), 1);
        let got = nfsd.nfsd_slp.take().expect("handed the socket");
        assert!(hal::mem::malloc::MArc::ptr_eq(&got, &s));
        assert_eq!(s.flag() & SLP_DOREC, 0);
        // An invalid socket wakes no one.
        s.clear_flags(SLP_VALID);
        nfsd.nfsd_flag.store(NFSD_WAITING, Relaxed);
        nfsrv_wakenfsd(&s);
        assert!(nfsd.nfsd_slp.take().is_none());
        {
            let mut w = klock::witness();
            t.nfsd_head.remove(&mut w, &nfsd);
            drop(s.ns_k.get_mut(&mut w).ns_self.take());
        }
        drop(got);
        hal::sync::klock::kernel_unlock();
    }
}
/* </TESTS> */
