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
//! `nfs/nfs_socket.c`, the server's half (D-5; the client's RPC engine, connect and
//! reconnect, receive, the reply match, the retransmit timer, the congestion window and the
//! RTT estimator, is the kernel's, `kernel::fs::nfs::nfs_socket`, since N6.7a5): socket
//! operations for use by the NFS server: the send and its lock, the socket upcall, the record
//! marking of stream sockets, parsing an RPC request, the reply header, waking an nfsd.
//!
//! Upstream: sys/nfs/nfs_socket.c @ 3ce1f3f79392
//!
//! The file sits under `option NFSCLIENT` or `option NFSSERVER` (`sys/conf/files`); inside it
//! `nfs_receive`, `nfs_reply` and `nfs_request` are `#ifdef NFSCLIENT` (feature `nfsclient`)
//! and `nfsrv_rcv`, `nfsrv_getstream`, `nfsrv_dorec` and `nfsrv_wakenfsd` `#ifdef NFSSERVER`
//! (feature `nfsserver`), as in the C. Requests, mounts and server sockets are the
//! `&'static` `Cell` objects of `nfs.rs` and `nfsmount.rs`; mbufs follow `docs/C_TO_RUST.md`
//! and the reply cursor conventions of `nfsm_subs.rs`.
//!
//! ## Deviations
//! - Errors are `Result<_, Errno>`.
//! - `nfs_sndlock`/`nfs_sndunlock` take the flag word as `&Cell<i32>` (the C's `int *`:
//!   `&slp->ns_solock`); its address is the sleep channel. `nfs_send` and `nfs_sndlock` are
//!   the C's with `rep == NULL`, the server's call (the client's are the kernel's).
//! - `MGET`/`m_copym`/`M_PREPEND`/`pool_get` with `M_WAIT`/`PR_WAITOK` can fail here (the
//!   pools cannot sleep, `subr_pool.rs`): `nfs_rephead` and `nfsrv_dorec` return `ENOBUFS`
//!   (freeing what the C would have consumed); `nfs_realign` leaves the chain as it is when it cannot get
//!   an mbuf (the dissection views of `nfsm_subs.rs` copy bytes, so a misaligned chain is
//!   still read correctly), and ignores `m_copyback`'s error, as the C does.
//! - `nfs_rephead` returns the reply and its build cursor (`*mrq`, `*mbp`) instead of
//!   storing them, and builds the six header words with `nfsm_build` (same bytes, same
//!   `m_len`). `nfsrv_dorec` returns the descriptor (`*ndp`). `nfs_getreq` takes
//!   `has_header` as a `bool`.
//! - `nfsrv_rcv` frees a partial chain or address that `soreceive` hands back with an error
//!   (the C leaks them), and queues a datagram that comes without an address as a record of
//!   its own (the C dereferences the NULL address); `nfsrv_dorec` takes such a record as it
//!   takes one without `MT_SONAME`.
//! - `nfsrv_dorec` takes its descriptors from `nfs_syscalls.rs`'s `NFSRV_DESCRIPT_PL`.

use core::cell::Cell;
use core::cmp::min;
use core::ffi::c_void;
use core::ptr;
#[cfg(feature = "nfsserver")]
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

#[cfg(feature = "nfsserver")]
use crate::kern::kern_synch::wakeup_one;
use crate::kern::kern_synch::{tsleep_nsec, wakeup};
use crate::kern::subr_pool::{pool_get, pool_put};
use crate::kern::subr_prf::panic;
use crate::kern::uipc_mbuf::{
    MAX_HDR, m_adj, m_copyback, m_copym, m_freem, m_get, m_gethdr, m_trailingspace,
};
use crate::kern::uipc_socket::{soreceive, sosend};
use crate::nfs::nfs::{ND_NFSV3, Nfsd, NfsrvDescript, NfssvcSock};
#[cfg(feature = "nfsserver")]
use crate::nfs::nfs::{
    NFSD_CHECKSLP, NFSD_HEAD, NFSD_HEAD_FLAG, NFSD_WAITING, SLP_DISCONN, SLP_DOREC, SLP_GETSTREAM,
    SLP_LASTFRAG, SLP_NEEDQ, SLP_VALID,
};
use crate::nfs::nfs_subs::{
    NFSSTATS, NFSV3_PROCID, nfs_prog, nfsm_build, rpc_auth_unix, rpc_autherr, rpc_call,
    rpc_mismatch, rpc_msgaccepted, rpc_msgdenied, rpc_reply, rpc_vers,
};
#[cfg(feature = "nfsserver")]
use crate::nfs::nfs_syscalls::NFSRV_DESCRIPT_PL;
use crate::nfs::nfsm_subs::{XdrIn, nfsd_adv, nfsd_dissect, nfsm_rndup};
use crate::nfs::nfsproto::{
    NFS_MAXNAMLEN, NFS_MAXPACKET, NFS_NPROCS, NFS_VER2, NFS_VER3, NFSERR_AUTHERR, NFSERR_RETVOID,
    NFSPROC_COMMIT, NFSPROC_NOOP, NFSPROC_NULL, NFSV2PROC_STATFS, NFSX_UNSIGNED,
};
use crate::nfs::rpcv2::{
    AUTH_REJECTCRED, RPC_GARBAGE, RPC_PROCUNAVAIL, RPC_PROGMISMATCH, RPC_PROGUNAVAIL, RPC_REPLYSIZ,
    RPC_VER2, RPCAUTH_MAXSIZ, RPCAUTH_UNIXGIDS,
};
use crate::nfs::xdr_subs::{fxdr_unsigned, txdr_unsigned};
use crate::sys::errno::Errno;
use crate::sys::mbuf::{
    M_DONTWAIT, M_WAIT, MHLEN, MINCLSIZE, MT_DATA, MT_SONAME, Mbuf, mclget, mtod,
};
use crate::sys::mbuf::{m_data_add, m_set_next, m_set_nextpkt, m_take_next};
use crate::sys::mount::{NFSMNT_SNDLOCK, NFSMNT_WANTSND};
use crate::sys::param::{PZERO, align, aligned_pointer};
use crate::sys::pool::PR_WAITOK;
use crate::sys::protosw::PR_CONNREQUIRED;
#[cfg(feature = "nfsserver")]
use crate::sys::socket::MSG_DONTWAIT;
use crate::sys::socket::SOCK_STREAM;
use crate::sys::socketvar::{SS_ISCONNECTED, Socket};
use crate::sys::syslimits::NGROUPS_MAX;
use crate::sys::systm::INFSLP;
#[cfg(feature = "nfsserver")]
use crate::sys::systm::{kernel_lock, kernel_unlock};
use crate::sys::ucred::UcredCell;
use crate::sys::uio::{Uio, UioRw, UioSeg};

/// `nfs_realign_test`: calls of `nfs_realign`.
pub static NFS_REALIGN_TEST: AtomicU32 = AtomicU32::new(0);

/// `nfs_realign_count`: chains `nfs_realign` had to copy.
pub static NFS_REALIGN_COUNT: AtomicU32 = AtomicU32::new(0);

/// `nfs_send(so, nam, top, rep)` of the server (`rep == NULL`; the client's half is the
/// kernel's, `kernel::fs::nfs::nfs_socket`): this is the nfs send routine. For connection
/// based socket types, it must be called with an `nfs_sndlock()` on the socket.
///
/// For the server side: return `EINTR` or `ERESTART` if interrupted by a signal; return
/// `EPIPE` if a connection is lost for connection based sockets (TCP...); do any cleanup
/// required by recoverable socket errors (???).
pub fn nfs_send(
    so: &'static Socket,
    nam: Option<&'static Mbuf>,
    top: &'static Mbuf,
) -> Result<(), Errno> {
    let soflags = i32::from(so.proto().flags());
    let sendnam = if soflags & i32::from(PR_CONNREQUIRED) != 0 || so.has_state(SS_ISCONNECTED) {
        None
    } else {
        nam
    };
    let flags = 0;

    let Err(error) = sosend(so, sendnam, None, Some(top), None, flags) else {
        return Ok(());
    };

    // Handle any recoverable (soft) socket errors here. (???)
    if error != Errno::EINTR
        && error != Errno::ERESTART
        && error != Errno::EWOULDBLOCK
        && error != Errno::EPIPE
    {
        return Ok(());
    }
    Err(error)
}

/// `nfs_rephead(siz, nd, slp, err, mrq, mbp)`: generate the rpc reply header; `siz` is used
/// to decide if adding a cluster is worthwhile. `err` is an errno number or an NFS status,
/// with the `NFSERR_AUTHERR`/`NFSERR_RETVOID` marks. Returns the reply (`*mrq`) and the build
/// cursor at its end (`*mbp`).
pub fn nfs_rephead(
    siz: usize,
    nd: &NfsrvDescript,
    _slp: Option<&NfssvcSock>,
    err: i32,
) -> Result<(&'static Mbuf, &'static Mbuf), Errno> {
    let Some(mreq) = m_gethdr(M_WAIT, MT_DATA) else {
        return Err(Errno::ENOBUFS);
    };
    let mut mb = mreq;
    // If this is a big reply, use a cluster else try and leave leading space for the lower
    // level headers.
    let siz = siz + RPC_REPLYSIZ;
    let max_hdr = MAX_HDR.load(Relaxed);
    if siz as i64 >= MHLEN as i64 - i64::from(max_hdr) {
        mclget(mreq, M_WAIT);
    } else {
        m_data_add(mreq, max_hdr as isize);
    }
    mreq.m_len().set(0);

    // The six header words (the sixth dropped for an authentication error) and the words
    // that follow them.
    let mut hdr = [0u32; 6];
    let mut extra = [0u32; 2];
    let mut nextra = 0;
    hdr[0] = txdr_unsigned(nd.nd_retxid);
    hdr[1] = rpc_reply;
    let autherr = err & NFSERR_AUTHERR != 0;
    if err == Errno::ERPCMISMATCH.as_i32() || autherr {
        hdr[2] = rpc_msgdenied;
        if autherr {
            hdr[3] = rpc_autherr;
            hdr[4] = txdr_unsigned((err & !NFSERR_AUTHERR) as u32);
        } else {
            hdr[3] = rpc_mismatch;
            hdr[4] = txdr_unsigned(RPC_VER2);
            hdr[5] = txdr_unsigned(RPC_VER2);
        }
    } else {
        hdr[2] = rpc_msgaccepted;

        // AUTH_UNIX requires RPCAUTH_NULL.
        hdr[3] = 0;
        hdr[4] = 0;

        if err == Errno::EPROGUNAVAIL.as_i32() {
            hdr[5] = txdr_unsigned(RPC_PROGUNAVAIL);
        } else if err == Errno::EPROGMISMATCH.as_i32() {
            hdr[5] = txdr_unsigned(RPC_PROGMISMATCH);
            extra = [txdr_unsigned(NFS_VER2), txdr_unsigned(NFS_VER3)];
            nextra = 2;
        } else if err == Errno::EPROCUNAVAIL.as_i32() {
            hdr[5] = txdr_unsigned(RPC_PROCUNAVAIL);
        } else if err == Errno::EBADRPC.as_i32() {
            hdr[5] = txdr_unsigned(RPC_GARBAGE);
        } else {
            hdr[5] = 0;
            if err != NFSERR_RETVOID {
                if err != 0 {
                    extra[0] = txdr_unsigned(crate::nfs::nfs_subs::nfsrv_errmap(nd, err) as u32);
                }
                nextra = 1;
            }
        }
    }

    let mut tl = nfsm_build(&mut mb, 6 * NFSX_UNSIGNED);
    for w in hdr {
        tl.put(w);
    }
    if autherr {
        mreq.m_len().set(mreq.m_len().get() - NFSX_UNSIGNED as u32);
    }
    if nextra > 0 {
        let mut tl = nfsm_build(&mut mb, nextra * NFSX_UNSIGNED);
        for &w in &extra[..nextra] {
            tl.put(w);
        }
    }

    if err != 0 && err != NFSERR_RETVOID {
        NFSSTATS.srvrpc_errs.fetch_add(1, Relaxed);
    }
    Ok((mreq, mb))
}

/// `nfs_sndlock(flagp, rep)` of the server (`rep == NULL`): lock a socket against others.
/// Necessary for STREAM sockets to ensure you get an entire rpc request/reply. `flagp` is
/// `&slp->ns_solock`.
pub fn nfs_sndlock(flagp: &Cell<i32>) -> Result<(), Errno> {
    while flagp.get() & NFSMNT_SNDLOCK != 0 {
        flagp.set(flagp.get() | NFSMNT_WANTSND);
        let _ = tsleep_nsec(ptr::from_ref(flagp), PZERO - 1, "nfsndlck", INFSLP);
    }
    flagp.set(flagp.get() | NFSMNT_SNDLOCK);
    Ok(())
}

/// `nfs_sndunlock(flagp)`: unlock the stream socket for others.
pub fn nfs_sndunlock(flagp: &Cell<i32>) {
    if flagp.get() & NFSMNT_SNDLOCK == 0 {
        panic(format_args!("nfs sndunlock"));
    }
    flagp.set(flagp.get() & !NFSMNT_SNDLOCK);
    if flagp.get() & NFSMNT_WANTSND != 0 {
        flagp.set(flagp.get() & !NFSMNT_WANTSND);
        wakeup(ptr::from_ref(flagp));
    }
}

/// `ALIGNED_POINTER(x, void *)`, the alignment `nfs_realign` wants for `m_data` and `m_len`.
fn aligned_void_pointer(x: usize) -> bool {
    aligned_pointer::<*const c_void>(x)
}

/// `nfs_realign_fixup(m, n, off)`: auxiliary routine to align the length of mbuf copies made
/// with `m_copyback()`.
pub fn nfs_realign_fixup(m: &'static Mbuf, n: &'static Mbuf, off: &mut u32) {
    realign_fixup(m, n, off, aligned_void_pointer);
}

/// `nfs_realign_fixup` with the alignment test as a parameter (host tests use a strict one).
fn realign_fixup(m: &'static Mbuf, n: &'static Mbuf, off: &mut u32, aligned: fn(usize) -> bool) {
    // The maximum number of bytes that m_copyback() places in a mbuf is always an aligned
    // quantity, so realign happens at the chain's tail.
    let mut n = n;
    while let Some(next) = n.m_next().get() {
        n = next;
    }

    // Pad from the next elements in the source chain. Loop until the destination chain is
    // aligned, or the end of the source is reached.
    let mut m = m;
    loop {
        let Some(next) = m.m_next().get() else {
            return;
        };
        m = next;

        let nlen = n.m_len().get() as usize;
        let padding = min(align(nlen) - nlen, m.m_len().get() as usize);
        if padding > m_trailingspace(n) as usize {
            panic(format_args!("nfs_realign_fixup: no memory to pad to"));
        }

        // SAFETY: `padding` bytes are `m`'s data (at most its `m_len`) and fit in `n`'s
        // trailing space (checked above); the two mbufs are different storage.
        unsafe { ptr::copy_nonoverlapping(mtod::<u8>(m), mtod::<u8>(n).add(nlen), padding) };

        n.m_len().set((nlen + padding) as u32);
        m_adj(m, padding as i32);
        *off += padding as u32;

        if aligned(n.m_len().get() as usize) {
            break;
        }
    }
}

/// The C's `ALIGN_POINTER(n)` of `nfs_realign`, kept as written:
/// `(u_int)(((n) + sizeof(void *)) & ~sizeof(void *))`.
const fn align_pointer(n: usize) -> usize {
    ((n + size_of::<*const c_void>()) & !size_of::<*const c_void>()) as u32 as usize
}

/// `nfs_realign(pm, hsiz)`: the NFS RPC parsing code uses the data address and the length of
/// mbuf structures to calculate on-memory addresses. This function makes sure these
/// parameters are correctly aligned. (`hsiz` is unused, as in C.)
pub fn nfs_realign(pm: &mut Option<&'static Mbuf>, _hsiz: usize) {
    realign(pm, aligned_void_pointer);
}

/// `nfs_realign` with the alignment test as a parameter (host tests use a strict one).
fn realign(pm: &mut Option<&'static Mbuf>, aligned: fn(usize) -> bool) {
    let mut prev: Option<&'static Mbuf> = None; // the mbuf whose m_next is *pm; None: pm
    let mut cur = *pm;
    let mut n = None;
    let mut off: u32 = 0;

    NFS_REALIGN_TEST.fetch_add(1, Relaxed);
    while let Some(m) = cur {
        if !aligned(mtod::<u8>(m).addr()) || !aligned(m.m_len().get() as usize) {
            let Some(nm) = m_get(M_WAIT, MT_DATA) else {
                return;
            };
            if align_pointer(m.m_len().get() as usize) >= MINCLSIZE {
                mclget(nm, M_WAIT);
            }
            nm.m_len().set(0);
            n = Some(nm);
            break;
        }
        prev = Some(m);
        cur = m.m_next().get();
    }
    // If n is non-NULL, loop on m copying data, then replace the portion of the chain that
    // had to be realigned.
    let Some(n) = n else {
        return;
    };
    NFS_REALIGN_COUNT.fetch_add(1, Relaxed);
    while let Some(m) = cur {
        let len = m.m_len().get() as usize;
        // SAFETY: the `m_len` bytes at `m_data` are `m`'s data, which nothing writes while
        // they are copied out.
        let data = unsafe { core::slice::from_raw_parts(mtod::<u8>(m), len) };
        let _ = m_copyback(n, off as i32, data, M_WAIT);

        // If an unaligned amount of memory was copied, fix up the last mbuf created by
        // m_copyback().
        if !aligned(len) {
            realign_fixup(m, n, &mut off, aligned);
        }

        off += len as u32;
        cur = m.m_next().get();
    }
    match prev {
        None => {
            m_freem(pm.take());
            *pm = Some(n);
        }
        Some(prev) => {
            m_freem(m_take_next(prev));
            let _ = m_set_next(prev, Some(n));
        }
    }
}

/// The raw words of a dissected view, copied out (`tl[0..N]`).
fn words<const N: usize>(tl: &XdrIn<'_>) -> [u32; N] {
    core::array::from_fn(|i| tl.get(i))
}

/// `nfs_getreq(nd, nfsd, has_header)`: parse an RPC request: verify it, fill in the cred
/// struct. A request the server must answer with an RPC error comes back `Ok` with
/// `nd_repstat` set and `nd_procnum` `NFSPROC_NOOP`; a malformed one frees the request and
/// fails with `EBADRPC`.
pub fn nfs_getreq(
    nd: &mut NfsrvDescript,
    _nfsd: Option<&Nfsd>,
    has_header: bool,
) -> Result<(), Errno> {
    let mut w = [0u32; 10];
    let mut i;
    if has_header {
        w = words::<10>(&nfsd_dissect(nd, 10 * NFSX_UNSIGNED)?);
        nd.nd_retxid = fxdr_unsigned(w[0]);
        if w[1] != rpc_call {
            m_freem(nd.nd_mrep.take());
            return Err(Errno::EBADRPC);
        }
        i = 2;
    } else {
        let tl = words::<8>(&nfsd_dissect(nd, 8 * NFSX_UNSIGNED)?);
        w[..8].copy_from_slice(&tl);
        i = 0;
    }
    nd.nd_repstat = 0;
    nd.nd_flag = 0;
    if w[i] != rpc_vers {
        nd.nd_repstat = Errno::ERPCMISMATCH.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    i += 1;
    if w[i] != nfs_prog {
        nd.nd_repstat = Errno::EPROGUNAVAIL.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    i += 1;
    let nfsvers = fxdr_unsigned(w[i]);
    i += 1;
    if nfsvers != NFS_VER2 && nfsvers != NFS_VER3 {
        nd.nd_repstat = Errno::EPROGMISMATCH.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    if nfsvers == NFS_VER3 {
        nd.nd_flag = ND_NFSV3;
    }
    nd.nd_procnum = fxdr_unsigned(w[i]) as usize;
    i += 1;
    if nd.nd_procnum == NFSPROC_NULL {
        return Ok(());
    }
    if nd.nd_procnum >= NFS_NPROCS
        || nd.nd_procnum > NFSPROC_COMMIT
        || (nd.nd_flag == 0 && nd.nd_procnum > NFSV2PROC_STATFS)
    {
        nd.nd_repstat = Errno::EPROCUNAVAIL.as_i32();
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }
    if nd.nd_flag & ND_NFSV3 == 0 {
        nd.nd_procnum = NFSV3_PROCID[nd.nd_procnum];
    }
    let auth_type = w[i];
    i += 1;
    let len = fxdr_unsigned(w[i]) as i32;
    if len < 0 || len as usize > RPCAUTH_MAXSIZ {
        m_freem(nd.nd_mrep.take());
        return Err(Errno::EBADRPC);
    }

    // Handle auth_unix
    if auth_type == rpc_auth_unix {
        // w[i + 1]: the stamp; w[i + 2]: the length of the machine name.
        let len = fxdr_unsigned(w[i + 2]) as i32;
        if len < 0 || len as usize > NFS_MAXNAMLEN {
            m_freem(nd.nd_mrep.take());
            return Err(Errno::EBADRPC);
        }
        nfsd_adv(nd, nfsm_rndup(len as usize))?;
        let tl = words::<3>(&nfsd_dissect(nd, 3 * NFSX_UNSIGNED)?);
        // The request's credential, built as the C fills `nd_cr` and stored once complete (a
        // request that fails here is dropped).
        let cr = UcredCell::new();
        cr.set_uid(fxdr_unsigned(tl[0]));
        cr.set_gid(fxdr_unsigned(tl[1]));
        let len = fxdr_unsigned(tl[2]) as i32;
        if len < 0 || len as usize > RPCAUTH_UNIXGIDS {
            m_freem(nd.nd_mrep.take());
            return Err(Errno::EBADRPC);
        }
        let len = len as usize;
        let tl: [u32; RPCAUTH_UNIXGIDS + 2] = {
            let tl = nfsd_dissect(nd, (len + 2) * NFSX_UNSIGNED)?;
            core::array::from_fn(|j| if j < len + 2 { tl.get(j) } else { 0 })
        };
        for (i, w) in tl[..len].iter().enumerate().take(NGROUPS_MAX) {
            cr.set_group(i, fxdr_unsigned(*w)); // the groups past NGROUPS_MAX are skipped
        }
        cr.set_ngroups(min(len, NGROUPS_MAX) as i16);
        nd.nd_cr = Some(cr);
        // tl[len]: the verifier's flavor; tl[len + 1]: its length.
        let len = fxdr_unsigned(tl[len + 1]) as i32;
        if len < 0 || len as usize > RPCAUTH_MAXSIZ {
            m_freem(nd.nd_mrep.take());
            return Err(Errno::EBADRPC);
        }
        if len > 0 {
            nfsd_adv(nd, nfsm_rndup(len as usize))?;
        }
    } else {
        nd.nd_repstat = NFSERR_AUTHERR | AUTH_REJECTCRED as i32;
        nd.nd_procnum = NFSPROC_NOOP;
        return Ok(());
    }

    Ok(())
}

/// [`nfsrv_rcv`] as the socket layer's registered upcall (`so->so_upcall`, set by
/// `nfssvc_addsock`): `arg` is the `nfssvc_sock`'s address it stored in `so_upcallarg`,
/// exposed there.
#[cfg(feature = "nfsserver")]
pub fn nfsrv_rcv_upcall(so: &Socket, arg: usize, waitflag: i32) {
    nfsrv_rcv(
        crate::sys::socketvar::as_static(so),
        core::ptr::with_exposed_provenance_mut(arg),
        waitflag,
    );
}

/// `nfsrv_rcv(so, arg, waitflag)`: socket upcall routine for the nfsd sockets. The `arg` is
/// a pointer to the `struct nfssvc_sock`. Essentially do as much as possible non-blocking,
/// else punt and it will be called with `M_WAIT` from an nfsd.
#[cfg(feature = "nfsserver")]
pub fn nfsrv_rcv(so: &'static Socket, arg: *mut c_void, waitflag: i32) {
    // SAFETY: `arg` is the `nfssvc_sock` that `nfssvc_addsock` registered with this socket
    // (or the nfsd passes); it is freed only by `nfsrv_slpderef` after `nfsrv_zapsock`
    // cleared the upcall and while no nfsd holds a reference.
    let slp: &'static NfssvcSock = unsafe { &*arg.cast::<NfssvcSock>() };

    kernel_lock();

    if slp.ns_flag.get() & SLP_VALID == 0 {
        kernel_unlock(); // out
        return;
    }

    'dorecs: {
        // Defer soreceive() to an nfsd.
        if waitflag == M_DONTWAIT {
            slp.ns_flag.set(slp.ns_flag.get() | SLP_NEEDQ);
            break 'dorecs;
        }

        if so.so_type() == SOCK_STREAM {
            // Do soreceive().
            let mut auio = Uio {
                uio_iov: &mut [],
                uio_offset: 0,
                uio_resid: 1_000_000_000,
                uio_segflg: UioSeg::UIO_SYSSPACE,
                uio_rw: UioRw::UIO_READ,
                uio_procp: None,
            };
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
            let m = match (error, mp) {
                (Ok(()), Some(m)) => m,
                (error, mp) => {
                    m_freem(mp);
                    if error == Err(Errno::EWOULDBLOCK) {
                        slp.ns_flag.set(slp.ns_flag.get() | SLP_NEEDQ);
                    } else {
                        slp.ns_flag.set(slp.ns_flag.get() | SLP_DISCONN);
                    }
                    break 'dorecs;
                }
            };
            let received = (1_000_000_000 - auio.uio_resid) as i32;
            if let Some(rawend) = slp.ns_rawend.get() {
                let _ = m_set_next(rawend, Some(m));
                slp.ns_cc.set(slp.ns_cc.get() + received);
            } else {
                slp.ns_raw.set(Some(m));
                slp.ns_cc.set(received);
            }
            let mut last = m;
            while let Some(next) = last.m_next().get() {
                last = next;
            }
            slp.ns_rawend.set(Some(last));

            // Now try and parse record(s) out of the raw stream data.
            if let Err(error) = nfsrv_getstream(slp, waitflag) {
                if error == Errno::EPERM {
                    slp.ns_flag.set(slp.ns_flag.get() | SLP_DISCONN);
                } else {
                    slp.ns_flag.set(slp.ns_flag.get() | SLP_NEEDQ);
                }
            }
        } else {
            loop {
                let mut auio = Uio {
                    uio_iov: &mut [],
                    uio_offset: 0,
                    uio_resid: 1_000_000_000,
                    uio_segflg: UioSeg::UIO_SYSSPACE,
                    uio_rw: UioRw::UIO_READ,
                    uio_procp: None,
                };
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
                if let Some(data) = mp {
                    let m = match nam {
                        Some(nam) => {
                            let _ = m_set_next(nam, Some(data));
                            nam
                        }
                        None => data,
                    };
                    if let Some(recend) = slp.ns_recend.get() {
                        let _ = m_set_nextpkt(recend, Some(m));
                    } else {
                        slp.ns_rec.set(Some(m));
                    }
                    slp.ns_recend.set(Some(m));
                    let _ = m_set_nextpkt(m, None);
                } else {
                    m_freem(nam);
                }
                if let Err(error) = error
                    && so.proto().flags() & PR_CONNREQUIRED != 0
                    && error != Errno::EWOULDBLOCK
                {
                    slp.ns_flag.set(slp.ns_flag.get() | SLP_DISCONN);
                    break 'dorecs;
                }
                if mp.is_none() {
                    break;
                }
            }
        }
    }

    // Now try and process the request records, non-blocking.
    // dorecs:
    if waitflag == M_DONTWAIT
        && (slp.ns_rec.get().is_some() || slp.ns_flag.get() & (SLP_NEEDQ | SLP_DISCONN) != 0)
    {
        nfsrv_wakenfsd(slp);
    }

    // out:
    kernel_unlock();
}

/// `nfsrv_getstream(slp, waitflag)`: try and extract an RPC request from the mbuf data list
/// received on a stream socket. The `waitflag` argument indicates whether or not it can
/// sleep. `EPERM` for a record too long to be NFS, `EWOULDBLOCK` when an mbuf is short.
#[cfg(feature = "nfsserver")]
pub fn nfsrv_getstream(slp: &NfssvcSock, waitflag: i32) -> Result<(), Errno> {
    if slp.ns_flag.get() & SLP_GETSTREAM != 0 {
        return Ok(());
    }
    slp.ns_flag.set(slp.ns_flag.get() | SLP_GETSTREAM);
    let done = |r: Result<(), Errno>| {
        slp.ns_flag.set(slp.ns_flag.get() & !SLP_GETSTREAM);
        r
    };
    loop {
        if slp.ns_reclen.get() == 0 {
            if (slp.ns_cc.get() as usize) < NFSX_UNSIGNED {
                return done(Ok(()));
            }
            let Some(mut m) = slp.ns_raw.get() else {
                panic(format_args!(
                    "nfsrv_getstream: {} bytes, no data",
                    slp.ns_cc.get()
                ));
            };
            let mut recmark = [0u8; NFSX_UNSIGNED];
            if m.m_len().get() as usize >= NFSX_UNSIGNED {
                // SAFETY: the first `NFSX_UNSIGNED` bytes of `m`'s data (checked above).
                unsafe {
                    ptr::copy_nonoverlapping(mtod::<u8>(m), recmark.as_mut_ptr(), NFSX_UNSIGNED)
                };
                m_data_add(m, NFSX_UNSIGNED as isize);
                m.m_len().set(m.m_len().get() - NFSX_UNSIGNED as u32);
            } else {
                for b in &mut recmark {
                    while m.m_len().get() == 0 {
                        let Some(next) = m.m_next().get() else {
                            panic(format_args!("nfsrv_getstream: short record mark"));
                        };
                        m = next;
                    }
                    // SAFETY: `m` has at least one byte of data (the loop above).
                    *b = unsafe { mtod::<u8>(m).read() };
                    m_data_add(m, 1);
                    m.m_len().set(m.m_len().get() - 1);
                }
            }
            slp.ns_cc.set(slp.ns_cc.get() - NFSX_UNSIGNED as i32);
            let recmark = u32::from_be_bytes(recmark);
            slp.ns_reclen.set((recmark & !0x8000_0000) as i32);
            if recmark & 0x8000_0000 != 0 {
                slp.ns_flag.set(slp.ns_flag.get() | SLP_LASTFRAG);
            } else {
                slp.ns_flag.set(slp.ns_flag.get() & !SLP_LASTFRAG);
            }
            if slp.ns_reclen.get() as usize > NFS_MAXPACKET {
                return done(Err(Errno::EPERM));
            }
        }

        // Now get the record part.
        let mut recm = None;
        let cc = slp.ns_cc.get();
        let reclen = slp.ns_reclen.get();
        if cc == reclen {
            recm = slp.ns_raw.get();
            slp.ns_raw.set(None);
            slp.ns_rawend.set(None);
            slp.ns_cc.set(0);
            slp.ns_reclen.set(0);
        } else if cc > reclen {
            let mut len = 0;
            let mut m = slp.ns_raw.get();
            let mut om: Option<&'static Mbuf> = None;
            while len < reclen {
                let Some(mm) = m else {
                    panic(format_args!("nfsrv_getstream: {} of {} bytes", len, reclen));
                };
                let mlen = mm.m_len().get() as i32;
                if len + mlen > reclen {
                    let Some(m2) = m_copym(mm, 0, reclen - len, waitflag) else {
                        return done(Err(Errno::EWOULDBLOCK));
                    };
                    if let Some(om) = om {
                        let _ = m_set_next(om, Some(m2));
                        recm = slp.ns_raw.get();
                    } else {
                        recm = Some(m2);
                    }
                    m_data_add(mm, ((reclen - len) as usize) as isize);
                    mm.m_len().set((mlen - (reclen - len)) as u32);
                    len = reclen;
                } else if len + mlen == reclen {
                    om = Some(mm);
                    len += mlen;
                    m = mm.m_next().get();
                    recm = slp.ns_raw.get();
                    let _ = m_set_next(mm, None);
                } else {
                    om = Some(mm);
                    len += mlen;
                    m = mm.m_next().get();
                }
            }
            slp.ns_raw.set(m);
            slp.ns_cc.set(cc - len);
            slp.ns_reclen.set(0);
        } else {
            return done(Ok(()));
        }

        // Accumulate the fragments into a record.
        match slp.ns_frag.get() {
            None => slp.ns_frag.set(recm),
            Some(first) => {
                let mut last = first;
                while let Some(next) = last.m_next().get() {
                    last = next;
                }
                let _ = m_set_next(last, recm);
            }
        }
        if slp.ns_flag.get() & SLP_LASTFRAG != 0 {
            if let Some(recend) = slp.ns_recend.get() {
                let _ = m_set_nextpkt(recend, slp.ns_frag.get());
            } else {
                slp.ns_rec.set(slp.ns_frag.get());
            }
            slp.ns_recend.set(slp.ns_frag.get());
            slp.ns_frag.set(None);
        }
    }
}

/// `nfsrv_dorec(slp, nfsd, ndp)`: parse an RPC header. Dequeues the socket's next record into
/// a new `nfsrv_descript_pl` descriptor, which becomes `nfsd->nfsd_nd` and is returned
/// (`*ndp`); `ENOBUFS` when there is no record.
#[cfg(feature = "nfsserver")]
pub fn nfsrv_dorec(slp: &NfssvcSock, nfsd: &Nfsd) -> Result<NonNull<NfsrvDescript>, Errno> {
    if slp.ns_flag.get() & SLP_VALID == 0 {
        return Err(Errno::ENOBUFS);
    }
    let Some(m) = slp.ns_rec.get() else {
        return Err(Errno::ENOBUFS);
    };
    slp.ns_rec.set(m.m_nextpkt().get());
    if slp.ns_rec.get().is_some() {
        let _ = m_set_nextpkt(m, None);
    } else {
        slp.ns_recend.set(None);
    }
    let (nam, m) = if i32::from(m.m_type().get()) == MT_SONAME {
        (Some(m), m_take_next(m))
    } else {
        (None, Some(m))
    };
    let Some(item) = pool_get(&NFSRV_DESCRIPT_PL, PR_WAITOK) else {
        m_freem(nam);
        m_freem(m);
        return Err(Errno::ENOBUFS);
    };
    let ndp = item.cast::<NfsrvDescript>();
    // SAFETY: a fresh `nfsrv_descript_pl` item, sized and aligned for a `struct
    // nfsrv_descript`; the nfsd owns it until it puts it back.
    let nd: &mut NfsrvDescript = unsafe {
        ndp.as_ptr().write(NfsrvDescript::new());
        &mut *ndp.as_ptr()
    };
    let mut m = m;
    nfs_realign(&mut m, 10 * NFSX_UNSIGNED);
    nd.nd_md = m;
    nd.nd_mrep = m;
    nd.nd_nam2 = nam;
    nd.nd_dpos = m.map_or(ptr::null_mut(), mtod::<u8>);
    if let Err(error) = nfs_getreq(nd, Some(nfsd), true) {
        m_freem(nam);
        pool_put(&NFSRV_DESCRIPT_PL, item);
        return Err(error);
    }
    nfsd.nfsd_nd.set(Some(ndp));
    Ok(ndp)
}

/// `nfsrv_wakenfsd(slp)`: search for a sleeping nfsd and wake it up. SIDE EFFECT: If none
/// found, set `NFSD_CHECKSLP` flag, so that one of the running nfsds will go look for the work
/// in the nfssvc_sock list.
#[cfg(feature = "nfsserver")]
pub fn nfsrv_wakenfsd(slp: &'static NfssvcSock) {
    if slp.ns_flag.get() & SLP_VALID == 0 {
        return;
    }

    for nfsd in NFSD_HEAD.0.iter() {
        if nfsd.nfsd_flag.get() & NFSD_WAITING != 0 {
            nfsd.nfsd_flag.set(nfsd.nfsd_flag.get() & !NFSD_WAITING);
            if nfsd.nfsd_slp.get().is_some() {
                panic(format_args!("nfsd wakeup"));
            }
            slp.ns_sref.set(slp.ns_sref.get() + 1);
            nfsd.nfsd_slp.set(Some(slp));
            wakeup_one(ptr::from_ref(nfsd));
            return;
        }
    }

    slp.ns_flag.set(slp.ns_flag.get() | SLP_DOREC);
    NFSD_HEAD_FLAG.fetch_or(NFSD_CHECKSLP, Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `nfs_socket.c`'s server half: the send lock of a server socket,
    // `nfs_realign` on a misaligned chain, the reply header of `nfs_rephead`, the request parse
    // of `nfs_getreq` and the record marking of `nfsrv_getstream`.

    use std::boxed::Box;
    use std::vec::Vec;

    use super::*;
    use crate::kern::uipc_mbuf::tests::setup;
    use crate::nfs::nfs_subs::tests::{bytes, chain, lens};
    use crate::nfs::nfsproto::NFSPROC_GETATTR;
    use crate::nfs::rpcv2::AUTH_BADCRED;
    use crate::sys::param::ALIGNBYTES;

    /// The XDR words `w` as wire bytes.
    fn wire(w: &[u32]) -> Vec<u8> {
        w.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// The server's lock word (`ns_solock`): taken free, given back, the waiters' bit cleared.
    #[test]
    fn the_server_send_lock() {
        let solock = Cell::new(0);
        nfs_sndlock(&solock).expect("free lock");
        assert_eq!(solock.get(), NFSMNT_SNDLOCK);
        nfs_sndunlock(&solock);
        assert_eq!(solock.get(), 0);
        solock.set(NFSMNT_SNDLOCK | NFSMNT_WANTSND);
        nfs_sndunlock(&solock);
        assert_eq!(solock.get(), 0);
    }

    /// Eight-byte alignment, the host's `ALIGNED_POINTER(x, void *)` on the real machines.
    fn strict(x: usize) -> bool {
        x.is_multiple_of(8)
    }

    #[test]
    fn realign_copies_a_misaligned_chain() {
        let _g = setup();
        assert_eq!(ALIGNBYTES, 7);

        let data: Vec<u8> = (0..29).collect();
        // An aligned first mbuf is kept; the misaligned rest is copied.
        let head = chain(&[&data[..8], &data[8..18], &data[18..21], &data[21..]]);
        let second = head.m_next().get();
        let mut pm = Some(head);
        let before = NFS_REALIGN_COUNT.load(Relaxed);
        realign(&mut pm, strict);
        assert_eq!(NFS_REALIGN_COUNT.load(Relaxed), before + 1);
        let head2 = pm.expect("a chain");
        assert!(ptr::eq(head2, head));
        assert!(!ptr::eq(
            head.m_next().get().expect("rest"),
            second.expect("old rest")
        ));
        assert_eq!(bytes(head2), data);
        let l = lens(head2);
        assert!(l[..l.len() - 1].iter().all(|&n| strict(n)), "{l:?}");
        m_freem(head2);

        // An aligned chain is left alone.
        let head = chain(&[&data[..8], &data[8..24]]);
        let mut pm = Some(head);
        let before = NFS_REALIGN_COUNT.load(Relaxed);
        realign(&mut pm, strict);
        assert_eq!(NFS_REALIGN_COUNT.load(Relaxed), before);
        assert_eq!(lens(pm.expect("chain")), [8, 16]);
        m_freem(pm);

        // A misaligned first mbuf replaces the head.
        let head = chain(&[&data[..5], &data[5..]]);
        let mut pm = Some(head);
        realign(&mut pm, strict);
        let head2 = pm.expect("a chain");
        assert!(!ptr::eq(head2, head));
        assert_eq!(bytes(head2), data);
        m_freem(head2);
    }

    #[test]
    fn rephead_builds_the_reply_header() {
        let _g = setup();
        let mut nd = NfsrvDescript::new();
        nd.nd_retxid = 0x1234;
        let xid = 0x1234u32;

        let reply = |nd: &NfsrvDescript, err: i32| -> Vec<u8> {
            let (mreq, _mb) = nfs_rephead(0, nd, None, err).expect("reply");
            let b = bytes(mreq);
            m_freem(mreq);
            b
        };

        // Success: accepted, null verifier, status 0, then the NFS status word 0.
        assert_eq!(reply(&nd, 0), wire(&[xid, 1, 0, 0, 0, 0, 0]));
        // A void reply has no NFS status word.
        assert_eq!(reply(&nd, NFSERR_RETVOID), wire(&[xid, 1, 0, 0, 0, 0]));
        assert_eq!(
            reply(&nd, Errno::EPROGUNAVAIL.as_i32()),
            wire(&[xid, 1, 0, 0, 0, 1])
        );
        assert_eq!(
            reply(&nd, Errno::EPROGMISMATCH.as_i32()),
            wire(&[xid, 1, 0, 0, 0, 2, 2, 3])
        );
        assert_eq!(
            reply(&nd, Errno::EPROCUNAVAIL.as_i32()),
            wire(&[xid, 1, 0, 0, 0, 3])
        );
        assert_eq!(
            reply(&nd, Errno::EBADRPC.as_i32()),
            wire(&[xid, 1, 0, 0, 0, 4])
        );
        // Denied: RPC version mismatch (low and high version), or an authentication error.
        assert_eq!(
            reply(&nd, Errno::ERPCMISMATCH.as_i32()),
            wire(&[xid, 1, 1, 0, 2, 2])
        );
        assert_eq!(
            reply(&nd, NFSERR_AUTHERR | AUTH_BADCRED as i32),
            wire(&[xid, 1, 1, 1, 1])
        );
    }

    /// A version `vers` call of procedure `proc_` with an AUTH_UNIX credential of uid 1000,
    /// gid 10 and `groups`, as an mbuf chain cut into `cut`-byte mbufs.
    fn call(vers: u32, proc_: u32, groups: &[u32], cut: usize) -> &'static Mbuf {
        let mut w = std::vec![77, 0, 2, 100_003, vers, proc_, 1];
        let cred_len = 5 * 4 + 8 + 4 * groups.len() as u32;
        w.extend([cred_len, 0, 5]);
        let mut b = wire(&w);
        b.extend(b"host\0\0\0\0");
        let mut w = std::vec![1000, 10, groups.len() as u32];
        w.extend(groups);
        w.extend([0, 0]); // AUTH_NULL verifier
        b.extend(wire(&w));
        let parts: Vec<&[u8]> = b.chunks(cut).collect();
        // `chain` copies the parts, so the vector may go.
        chain(&parts)
    }

    /// A descriptor dissecting `m` from its start.
    fn descript(m: &'static Mbuf) -> NfsrvDescript {
        let mut nd = NfsrvDescript::new();
        nd.nd_mrep = Some(m);
        nd.nd_md = Some(m);
        nd.nd_dpos = mtod::<u8>(m);
        nd
    }

    #[test]
    fn getreq_parses_an_auth_unix_call() {
        let _g = setup();

        let mut nd = descript(call(3, 1, &[20, 30], 12));
        nfs_getreq(&mut nd, None, true).expect("a call");
        assert_eq!(nd.nd_retxid, 77);
        assert_eq!(nd.nd_repstat, 0);
        assert_eq!(nd.nd_flag, ND_NFSV3);
        assert_eq!(nd.nd_procnum, NFSPROC_GETATTR);
        let cr = nd.nd_cr.as_ref().expect("the request's credential");
        assert_eq!(cr.cr_uid.get(), 1000);
        assert_eq!(cr.cr_gid.get(), 10);
        assert_eq!(cr.cr_ngroups.get(), 2);
        assert_eq!(cr.cr_groups[1].get(), 30);
        m_freem(nd.nd_mrep);

        // Version 2 procedure numbers are mapped to version 3's (v2 READ is 6, v3 READ 6;
        // v2 STATFS 17 is v3 FSSTAT 18).
        let mut nd = descript(call(2, 17, &[], 7));
        nfs_getreq(&mut nd, None, true).expect("a call");
        assert_eq!(nd.nd_flag, 0);
        assert_eq!(nd.nd_procnum, NFSV3_PROCID[17]);
        m_freem(nd.nd_mrep);

        // RPC errors are reported in nd_repstat with the NOOP procedure.
        let mut nd = descript(call(4, 1, &[], 40));
        nfs_getreq(&mut nd, None, true).expect("a call");
        assert_eq!(nd.nd_repstat, Errno::EPROGMISMATCH.as_i32());
        assert_eq!(nd.nd_procnum, NFSPROC_NOOP);
        m_freem(nd.nd_mrep);

        let mut nd = descript(call(2, 18, &[], 40));
        nfs_getreq(&mut nd, None, true).expect("a call");
        assert_eq!(nd.nd_repstat, Errno::EPROCUNAVAIL.as_i32());
        m_freem(nd.nd_mrep);

        // A reply instead of a call is garbage; the request is freed.
        let m = call(3, 1, &[], 40);
        // SAFETY: the second word of the first mbuf's data (40 bytes long).
        unsafe { mtod::<u8>(m).add(7).write(1) };
        let mut nd = descript(m);
        assert_eq!(nfs_getreq(&mut nd, None, true), Err(Errno::EBADRPC));
        assert!(nd.nd_mrep.is_none());
    }

    #[cfg(feature = "nfsserver")]
    mod stream {
        use super::*;

        /// A server socket that lives for the rest of the test run.
        fn leak_slp() -> &'static NfssvcSock {
            Box::leak(Box::new(NfssvcSock::new()))
        }

        /// The records queued on `slp`, each as bytes.
        fn records(slp: &NfssvcSock) -> Vec<Vec<u8>> {
            let mut v = Vec::new();
            let mut r = slp.ns_rec.get();
            while let Some(m) = r {
                v.push(bytes(m));
                r = m.m_nextpkt().get();
            }
            v
        }

        #[test]
        fn getstream_splits_records_and_joins_fragments() {
            let _g = setup();
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

            let parts: Vec<&[u8]> = s.chunks(3).collect();
            let raw = chain(&parts);
            let slp = leak_slp();
            slp.ns_raw.set(Some(raw));
            let mut last = raw;
            while let Some(n) = last.m_next().get() {
                last = n;
            }
            slp.ns_rawend.set(Some(last));
            slp.ns_cc.set(s.len() as i32);

            assert_eq!(nfsrv_getstream(slp, M_WAIT), Ok(()));
            assert_eq!(slp.ns_flag.get() & SLP_GETSTREAM, 0);
            let mut rec2 = frag_a.clone();
            rec2.extend(&frag_b);
            assert_eq!(records(slp), [rec1, rec2]);
            assert!(slp.ns_frag.get().is_none());
            assert_eq!(slp.ns_reclen.get(), 12);
            assert_eq!(slp.ns_cc.get(), 5);
            assert_eq!(
                bytes(slp.ns_raw.get().expect("partial record")),
                [21, 22, 23, 24, 25]
            );
            assert_ne!(slp.ns_flag.get() & SLP_LASTFRAG, 0);

            // The record mark of a record too big for NFS disconnects.
            let big = (0x8000_0000u32 | (NFS_MAXPACKET as u32 + 1)).to_be_bytes();
            let slp = leak_slp();
            slp.ns_raw.set(Some(chain(&[&big[..2], &big[2..]])));
            slp.ns_cc.set(4);
            assert_eq!(nfsrv_getstream(slp, M_WAIT), Err(Errno::EPERM));
            assert_eq!(slp.ns_flag.get() & SLP_GETSTREAM, 0);
        }
    }
}
/* </TESTS> */
