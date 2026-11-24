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
 * Copyright (c) 1989, 1993
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
 *	@(#)nfs_syscalls.c	8.5 (Berkeley) 3/30/95
 */

/*
 * Copyright (c) 1989, 1993, 1995
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
 *	@(#)nfs.h	8.4 (Berkeley) 5/1/95
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS server's daemons and sockets (N6.7b2): `nfs.h`'s server structures (the socket a
//! server receives on, `struct nfssvc_sock`; an nfsd, `struct nfsd`; a request,
//! `struct nfsrv_descript`) and their flags, and `nfs_syscalls.c`'s server half: the
//! `nfssvc(2)` system call, the sockets it adds, the nfsds' loop and the tables they share.
//! Built beside `bsd`'s server and unregistered until N6.7b4 switches the server on (D-9:
//! `nfssvc` stays `bsd`'s until then).
//!
//! Upstream: sys/nfs/nfs_syscalls.c @ 3ce1f3f79392
//! Upstream: sys/nfs/nfs.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_syscalls.rs@e8249449d905
//! LZ: sys/nfs/nfs.rs@e8249449d905
//!
//! `option NFSSERVER` only: the module is behind the kernel's `nfsserver` feature (D-16).
//!
//! ## Deviations
//! - `nfsd_procp` is the nfsd's thread id; the nfsd's own frame holds its `&Thread` and hands
//!   it to each procedure. `nfsd_nd`, a record of the descriptor the nfsd serves that nothing
//!   reads, is gone: the frame owns the descriptor.
//! - `ns_nam`, the client's address of a connected socket, is kept as its bytes and lent to
//!   a request as a name mbuf (the socket's own, made once; a copy when another nfsd holds
//!   it): the C points every request of the socket at one mbuf, which `nfsrv_zapsock` frees
//!   under a request still using it. The address of a datagram socket is never read (every
//!   datagram's record carries its sender), so `NFSSVC_ADDSOCK` keeps none for one.
//!
//! ## Redesign
//! - LZ's `nfssvc_sock` and `nfsd` were `malloc(9)` blocks cast to `&'static` (two `unsafe`
//!   each to make and free), linked on `TailqHead`s in `Sync` wrappers (three `unsafe impl`,
//!   four raw links), with `Cell` members, parked mbufs and a raw `*const Proc`. Here a
//!   socket is an [`MArc`] of `M_NFSSVC` pinned on `nfssvc_sockhead`, a `hal::pinlist` list
//!   bound to the kernel lock (the C's lock for all of the server, D-11): the list's
//!   membership holds one handle (`ns_self`, set at the insert, taken at the unlink), an nfsd
//!   holds one while it serves the socket, and `ns_sref` stays the C's count that decides the
//!   unlink. An nfsd is an [`hal::malloc::MBox`] of `M_NFSD` its system call's frame owns, pinned on
//!   `nfsd_head`, its address the channel it sleeps on. The words the C changes under the
//!   kernel lock are relaxed atomics (`ns_flag`, `ns_sref`, `nfsd_flag`; `ns_solock` is
//!   `nfs_sndlock`'s flag word, its address the channel); the handles a list or a waker
//!   reaches are kernel-lock cells. The mbufs a socket queues (`ns_raw`, `ns_frag`, `ns_rec`,
//!   `ns_cc`, `ns_reclen`) are one [`NsRcv`] in a [`LoanSlot`]: an mbuf is `Send` and not
//!   `Sync`, and the C changes them only under the kernel lock, in stretches without a sleep,
//!   so a stretch is one claim of the slot (two atomics, no spl), and the one stretch that
//!   sleeps (a record split by `m_copym(M_WAIT)`) works on the state taken out of the slot.
//! - LZ's `nfsrv_descript` was a pool item cast to `&mut`, its request a parked chain and its
//!   cursor a raw pointer into it (`nd_md`, `nd_dpos`). Here [`NfsrvDescript`] is a
//!   [`hal::pool::PoolBox`] of "ndscpl" the nfsd owns for one request: `nd_mrep` owns the chain, the
//!   cursor is the bytes consumed of it (`nd_off`, read again with `XdrReader::at`, as the
//!   client's `Reply`), and the address mbufs are owned (`nd_nam2`, the datagram's) or lent
//!   (`nd_nam`, the connection's).
//! - `nfsrv3_procs[]` is an array of one function type, [`NfsrvProc`], indexed by
//!   `nd_procnum`: no operations table.

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::malloc::MArc;
use hal::owned::OwnedSlist;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::BoxPool;
use hal::rwlock::Guarded;
use hal::slot::LoanSlot;
use hal::sys::errno::Errno;
use hal::sys::types::Pid;

use super::nfsm_subs::XdrReader;
use super::nfsmount::NfsNam;
use super::nfsproto::{NFS_MAXDATA, NFS_MAXDGRAMDATA, NFS_NPROCS, NFS_V2MAXDATA};
use crate::cred::UcredCell;
use crate::file::FileSlot;
use crate::net::mbuf::{MNext, Mbuf, MbufBox, MbufList};
use crate::net::socket::SocketRef;
use crate::proc::{ProcGlue, Thread};

/// `M_NFSSVC`: NFS server structures (`<sys/malloc.h>`'s 50, "NFS srvsock").
pub const M_NFSSVC: i32 = 50;
/// `M_NFSD`: NFS server daemon structures (`<sys/malloc.h>`'s 52, "NFS daemon").
pub const M_NFSD: i32 = 52;

/// `SLP_VALID`: connection is usable; bits for `ns_flag`.
pub const SLP_VALID: i32 = 0x01;
/// `SLP_DOREC`: receive operation required.
pub const SLP_DOREC: i32 = 0x02;
/// `SLP_NEEDQ`: connection has data to queue from socket.
pub const SLP_NEEDQ: i32 = 0x04;
/// `SLP_DISCONN`: connection is closed.
pub const SLP_DISCONN: i32 = 0x08;
/// `SLP_GETSTREAM`: extracting RPC from TCP connection.
pub const SLP_GETSTREAM: i32 = 0x10;
/// `SLP_LASTFRAG`: last fragment received on TCP connection.
pub const SLP_LASTFRAG: i32 = 0x20;
/// `SLP_ALLFLAGS`: convenience.
pub const SLP_ALLFLAGS: i32 = 0xff;

/// `SLP_INIT`: NFS data undergoing initialization; a bit of `nfssvc_sockhead_flag`.
pub const SLP_INIT: i32 = 0x01;
/// `SLP_WANTINIT`: thread waiting on NFS initialization.
pub const SLP_WANTINIT: i32 = 0x02;

/// `NFSD_WAITING`: bits for `nfsd_flag`.
pub const NFSD_WAITING: i32 = 0x01;
/// `NFSD_REQINPROG`.
pub const NFSD_REQINPROG: i32 = 0x02;

/// `ND_NFSV3`: bits for `nd_flag`.
pub const ND_NFSV3: i32 = 0x08;

/// `NFSD_CHECKSLP`: a bit of `nfsd_head_flag`.
pub const NFSD_CHECKSLP: i32 = 0x01;

/// A counted handle to a server socket.
pub type NfssvcSockRef<G> = MArc<NfssvcSock<G>>;

/// The mbufs a server socket queues (`ns_raw` .. `ns_reclen`), changed in stretches without
/// a sleep under the kernel lock, each one claim of the socket's slot.
#[derive(Default)]
pub struct NsRcv {
    /// `ns_raw`, `ns_rawend`: the stream's bytes not yet parsed into records.
    pub ns_raw: OwnedSlist<MNext>,
    /// `ns_frag`: the fragments of the record being assembled.
    pub ns_frag: OwnedSlist<MNext>,
    /// `ns_rec`, `ns_recend`: queued RPC records (a datagram's begins with its sender's
    /// `MT_SONAME` mbuf).
    pub ns_rec: MbufList,
    /// `ns_cc`: actual chars queued in `ns_raw`.
    pub ns_cc: i32,
    /// `ns_reclen`: length of first queued record fragment.
    pub ns_reclen: i32,
}

/// The handles of a server socket, under the kernel lock.
pub struct NsLinks<G: ProcGlue> {
    /// The list's membership: the handle `nfssvc_sockhead` holds (set at the insert, taken
    /// at the unlink), which a list walker or a waker clones to hand the socket on.
    pub ns_self: Option<NfssvcSockRef<G>>,
    /// `ns_fp`: fp from the...
    pub ns_fp: FileSlot<G::FileGlue>,
    /// `ns_so`: ...socket this struct wraps.
    pub ns_so: Option<SocketRef<G>>,
}

/// `struct nfssvc_sock`: a socket the server receives requests on (the module's Redesign).
pub struct NfssvcSock<G: ProcGlue> {
    /// `ns_chain`: list of all `nfssvc_sock`s (the kernel lock).
    pub ns_chain: PinEntry<NfssvcSock<G>>,
    /// `ns_self`, `ns_fp`, `ns_so`.
    pub ns_k: Guarded<NsLinks<G>>,
    /// `ns_so->so_type`, set with `ns_so` (`nfsrv_fhtovp`'s port 20 test reads it with no
    /// witness).
    pub ns_sotype: AtomicI32,
    /// `ns_nam`: `MT_SONAME` of client (a connected socket's; the module's Deviations).
    pub ns_nam: Option<NfsNam>,
    /// `ns_nam` as a name mbuf, made at its first use and lent to each request.
    pub ns_namm: LoanSlot<Option<MbufBox>>,
    /// `ns_raw` .. `ns_reclen`.
    pub ns_rcv: LoanSlot<NsRcv>,
    /// `ns_flag`: socket status flags (`SLP_*`).
    pub ns_flag: AtomicI32,
    /// `ns_solock`: lock for connected socket (`nfs_sndlock`'s word; its address the
    /// channel).
    pub ns_solock: AtomicI32,
    /// `ns_sref`: # of refs to this struct.
    pub ns_sref: AtomicU32,
}

hal::pin_adapter!(
    /// `TAILQ_HEAD(, nfssvc_sock) nfssvc_sockhead`: the server's sockets, through `ns_chain`.
    impl[G: ProcGlue] pub NsChain<G>: NfssvcSock<G>, ns_chain
);

impl<G: ProcGlue> NfssvcSock<G> {
    /// A socket with every member cleared (`malloc(M_ZERO)`), the client's address `nam`
    /// (a connected socket's) kept.
    pub fn new(nam: Option<NfsNam>) -> Self {
        Self {
            ns_chain: PinEntry::new(),
            ns_k: Guarded::new_kernel(NsLinks {
                ns_self: None,
                ns_fp: FileSlot::new(),
                ns_so: None,
            }),
            ns_sotype: AtomicI32::new(0),
            ns_nam: nam,
            ns_namm: LoanSlot::new(None),
            ns_rcv: LoanSlot::new(NsRcv::default()),
            ns_flag: AtomicI32::new(0),
            ns_solock: AtomicI32::new(0),
            ns_sref: AtomicU32::new(0),
        }
    }

    /// `slp->ns_flag`.
    #[inline]
    pub fn flag(&self) -> i32 {
        self.ns_flag.load(Ordering::Relaxed)
    }

    /// `slp->ns_flag |= bits`.
    #[inline]
    pub fn set_flags(&self, bits: i32) {
        self.ns_flag.fetch_or(bits, Ordering::Relaxed);
    }

    /// `slp->ns_flag &= ~bits`.
    #[inline]
    pub fn clear_flags(&self, bits: i32) {
        self.ns_flag.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `slp->ns_so->so_type`.
    #[inline]
    pub fn sotype(&self) -> i32 {
        self.ns_sotype.load(Ordering::Relaxed)
    }
}

/// `struct nfsd`: one of these structures is allocated for each nfsd (the module's
/// Redesign).
pub struct Nfsd<G: ProcGlue> {
    /// `nfsd_chain`: list of all nfsd's (the kernel lock).
    pub nfsd_chain: PinEntry<Nfsd<G>>,
    /// `nfsd_flag`: `NFSD_` flags.
    pub nfsd_flag: AtomicI32,
    /// `nfsd_slp`: current socket, as `nfsrv_wakenfsd` hands it to a waiting nfsd (counted
    /// in its `ns_sref`); the nfsd takes it into its frame (`nfsrv_getslp`). A slot: the
    /// waker fills it while it walks `nfsd_head` under the kernel lock's reader.
    pub nfsd_slp: LoanSlot<Option<NfssvcSockRef<G>>>,
    /// `nfsd_procp`: the nfsd's thread, by its id.
    pub nfsd_procp: Pid,
}

hal::pin_adapter!(
    /// `TAILQ_HEAD(nfsdhead, nfsd) nfsd_head`: every nfsd, through `nfsd_chain`.
    impl[G: ProcGlue] pub NfsdChain<G>: Nfsd<G>, nfsd_chain
);

impl<G: ProcGlue> Nfsd<G> {
    /// The nfsd of thread `tid`, every other member cleared (`malloc(M_ZERO)`).
    pub fn new(tid: Pid) -> Self {
        Self {
            nfsd_chain: PinEntry::new(),
            nfsd_flag: AtomicI32::new(0),
            nfsd_slp: LoanSlot::new(None),
            nfsd_procp: tid,
        }
    }

    /// `nfsd->nfsd_flag`.
    #[inline]
    pub fn flag(&self) -> i32 {
        self.nfsd_flag.load(Ordering::Relaxed)
    }
}

/// `struct nfsrv_descript`: used by the server for describing each request; the nfsd that
/// took the request owns it (the module's Redesign).
#[derive(Default)]
pub struct NfsrvDescript {
    /// `nd_mrep`: request mbuf list.
    pub nd_mrep: Option<MbufBox>,
    /// `nd_md` and `nd_dpos`: the bytes of `nd_mrep` dissected so far.
    pub nd_off: usize,
    /// `nd_nam` when it is the connection's address (`slp->ns_nam`), lent for the request.
    pub nd_nam: Option<MbufBox>,
    /// `nd_nam2`: return socket addr (a datagram's sender).
    pub nd_nam2: Option<MbufBox>,
    /// `nd_procnum`: RPC #.
    pub nd_procnum: usize,
    /// `nd_flag`: `ND_*`.
    pub nd_flag: i32,
    /// `nd_repstat`: reply status (an errno or an NFS status).
    pub nd_repstat: i32,
    /// `nd_retxid`: reply xid, in host order.
    pub nd_retxid: u32,
    /// `nd_cr`: credentials.
    pub nd_cr: UcredCell,
}

impl NfsrvDescript {
    /// `nd->nd_nam`: the client's address, the datagram's sender (`nd_nam2`) or the
    /// connection's.
    pub fn nam(&self) -> Option<&Mbuf> {
        self.nd_nam2.as_deref().or(self.nd_nam.as_deref())
    }

    /// A dissection cursor at `nd_md`/`nd_dpos`: `nd_off` bytes into the request (`EBADRPC`
    /// past its end, or with no request); the caller stores `offset()` back into `nd_off`.
    pub fn reader(&self) -> Result<XdrReader<'_>, Errno> {
        nd_reader(&self.nd_mrep, self.nd_off)
    }
}

/// [`NfsrvDescript::reader`] over the two members alone (`nd_reader(&nd.nd_mrep,
/// nd.nd_off)`), so that the other members stay writable while the cursor lives.
pub fn nd_reader(nd_mrep: &Option<MbufBox>, nd_off: usize) -> Result<XdrReader<'_>, Errno> {
    match nd_mrep.as_deref() {
        Some(m) => XdrReader::at(m, nd_off),
        None => Err(Errno::EBADRPC),
    }
}

/// One of `nfsrv3_procs[]`: a server procedure (`nfs_serv.c`), given the request, the socket
/// it came on and the nfsd's thread; it answers the reply it built (the C's `*mrq`, `None`
/// for none) and the C's `int` (an errno, or 0).
pub type NfsrvProc<G> =
    fn(&mut NfsrvDescript, &NfssvcSock<G>, &Thread<G>) -> (Option<MbufBox>, i32);

/// The server's tables (`nfs_syscalls.c`'s and `nfs.h`'s globals) of a glue, under the
/// kernel lock.
pub struct NfsrvTables<G: ProcGlue> {
    /// `nfssvc_sockhead`.
    pub nfssvc_sockhead: PinList<NsChain<G>>,
    /// `nfssvc_sockhead_flag` (`SLP_INIT`, `SLP_WANTINIT`); its list's address is the
    /// channel of `"nfsd init"`.
    pub nfssvc_sockhead_flag: AtomicI32,
    /// `nfsd_head`.
    pub nfsd_head: PinList<NfsdChain<G>>,
    /// `nfsd_head_flag` (`NFSD_CHECKSLP`).
    pub nfsd_head_flag: AtomicI32,
    /// `nfs_udpsock`: the datagram socket's record, made by `nfsrv_init`.
    pub nfs_udpsock: Guarded<Option<NfssvcSockRef<G>>>,
    /// `nfs_numnfsd`: the nfsds running.
    pub nfs_numnfsd: AtomicI32,
    /// `nfsd_waiting`: the nfsds asleep in `nfsrv_getslp`.
    pub nfsd_waiting: AtomicI32,
    /// `nfsrv_descript_pl` ("ndscpl"): the request descriptors.
    pub nfsrv_descript_pl: BoxPool<NfsrvDescript>,
}

impl<G: ProcGlue> NfsrvTables<G> {
    /// No socket, no nfsd, the pool not made.
    pub const fn new() -> Self {
        Self {
            nfssvc_sockhead: PinList::new(),
            nfssvc_sockhead_flag: AtomicI32::new(0),
            nfsd_head: PinList::new(),
            nfsd_head_flag: AtomicI32::new(0),
            nfs_udpsock: Guarded::new_kernel(None),
            nfs_numnfsd: AtomicI32::new(0),
            nfsd_waiting: AtomicI32::new(0),
            nfsrv_descript_pl: BoxPool::new(),
        }
    }
}

impl<G: ProcGlue> Default for NfsrvTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `NFS_SRVMAXDATA(n)`: the largest data a server reply may carry.
pub fn nfs_srvmaxdata(n: &NfsrvDescript) -> usize {
    if n.nd_flag & ND_NFSV3 != 0 {
        if n.nd_nam2.is_some() {
            NFS_MAXDGRAMDATA
        } else {
            NFS_MAXDATA
        }
    } else {
        NFS_V2MAXDATA
    }
}

const _: () = {
    assert!(NFS_NPROCS == 23);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the server's structures: the flags against the C header, the largest
    // reply data, the descriptor's address and cursor.

    use super::*;
    use crate::fs::nfs::nfsm_subs::test_chain;

    #[test]
    fn the_largest_reply_data_follows_the_version_and_the_transport() {
        let _g = crate::net::mbuf::tests::setup();
        let mut nd = NfsrvDescript::default();
        assert_eq!(nfs_srvmaxdata(&nd), NFS_V2MAXDATA);
        nd.nd_flag = ND_NFSV3;
        assert_eq!(nfs_srvmaxdata(&nd), NFS_MAXDATA);
        nd.nd_nam2 = Some(test_chain(&[&[16, 2, 0, 1, 127, 0, 0, 1]]));
        assert_eq!(nfs_srvmaxdata(&nd), NFS_MAXDGRAMDATA);
    }

    #[test]
    fn the_address_is_the_datagram_sender_first_then_the_connection() {
        let _g = crate::net::mbuf::tests::setup();
        let mut nd = NfsrvDescript::default();
        assert!(nd.nam().is_none());
        nd.nd_nam = Some(test_chain(&[&[16, 2, 3, 4]]));
        assert_eq!(nd.nam().map(|m| m.data()[2]), Some(3));
        nd.nd_nam2 = Some(test_chain(&[&[16, 2, 8, 9]]));
        assert_eq!(nd.nam().map(|m| m.data()[2]), Some(8));
    }

    #[test]
    fn the_cursor_starts_where_the_last_dissection_stopped() {
        let _g = crate::net::mbuf::tests::setup();
        let mut nd = NfsrvDescript::default();
        assert_eq!(nd.reader().err(), Some(Errno::EBADRPC));
        nd.nd_mrep = Some(test_chain(&[&[0, 0, 0, 1], &[0, 0, 0, 2, 0, 0, 0, 3]]));
        nd.nd_off = 4;
        let mut r = nd.reader().expect("in the chain");
        assert_eq!(r.u32(), Ok(2));
        nd.nd_off = 13;
        assert_eq!(nd.reader().err(), Some(Errno::EBADRPC));
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_headers() {
        let nfs = crate::reftest::defines("sys/nfs/nfs.h");
        let ours: &[(&str, i32)] = &[
            ("SLP_VALID", SLP_VALID),
            ("SLP_DOREC", SLP_DOREC),
            ("SLP_NEEDQ", SLP_NEEDQ),
            ("SLP_DISCONN", SLP_DISCONN),
            ("SLP_GETSTREAM", SLP_GETSTREAM),
            ("SLP_LASTFRAG", SLP_LASTFRAG),
            ("SLP_ALLFLAGS", SLP_ALLFLAGS),
            ("NFSD_WAITING", NFSD_WAITING),
            ("NFSD_REQINPROG", NFSD_REQINPROG),
            ("ND_NFSV3", ND_NFSV3),
            ("NFSD_CHECKSLP", NFSD_CHECKSLP),
        ];
        for (name, value) in ours {
            assert_eq!(
                crate::reftest::int(&nfs, name),
                Some(i64::from(*value)),
                "{name}"
            );
        }
        let malloc = crate::reftest::defines("sys/sys/malloc.h");
        assert_eq!(
            crate::reftest::int(&malloc, "M_NFSSVC"),
            Some(i64::from(M_NFSSVC))
        );
        assert_eq!(
            crate::reftest::int(&malloc, "M_NFSD"),
            Some(i64::from(M_NFSD))
        );
    }
}
/* </TESTS> */
