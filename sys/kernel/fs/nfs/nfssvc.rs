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
//! Built beside `bsd`'s server (N6.7b2) and the server since N6.7b4 (D-9): `sysent[155]`'s
//! row (`bsd`'s `vfs_syscalls`, the system call prologue's) calls [`sys_nfssvc`], `abi`'s
//! entry is `Native`, `nfs_init` makes the tables, and `bsd`'s `sys/nfs/` is gone.
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
//! - `nfsrv3_procs[]` is `nfs_serv`'s (N6.7b3), handed to `sys_nfssvc` and the nfsds' loop
//!   (`procs`) by `sysent[155]`'s row (N6.7b4): the C's global table is a value the call is
//!   given.
//! - `sys_nfssvc` answers `ENOMEM` when no `struct nfsd` is to be had, and `nfssvc_addsock`
//!   `ENOMEM` or `ENOBUFS` when no record or option mbuf is (LZ's; the C's `M_WAIT` cannot
//!   fail).
//! - A procedure's reply that comes with an error is freed; the C leaks it.
//! - `nfsrv_slpderef` never frees the datagram socket's record (LZ's): the C frees it and
//!   leaves `nfs_udpsock` pointing at the freed record for the next `NFSSVC_ADDSOCK`.
//! - `nfsrv_init` leaves `nfsd_head` as it is (the C's `TAILQ_INIT`): no nfsd is on it when
//!   the last one out calls it, and a list of pinned nfsds is not dropped from under them.
//! - `nfsrv_zapsock` also frees the fragments of a record being assembled (`ns_frag`), which
//!   the C leaks.
//! - `nfsrv_init` makes the new datagram socket's record before it clears `SLP_INIT` and wakes
//!   the system calls waiting on "nfsd init": the C makes it after (`nfs_syscalls.c:571`,
//!   then `:577`), while its `malloc(M_WAITOK)` may sleep, so a woken `NFSSVC_ADDSOCK` of a
//!   datagram socket reads `nfs_udpsock`, the record `nfsrv_init(1)` has just freed.
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
//! - The socket's `so_upcallarg` (LZ: the record's address exposed into a word, cast back by
//!   the upcall) is one more handle of the record, erased (`MArc::erase`; N6.7b4, D-10):
//!   `nfssvc_addsock` gives it to the socket, `nfsrv_zapsock` takes it back with the C's
//!   `so_upcall = NULL`, so a socket never names a freed record and the record never keeps
//!   its socket alive through a cycle once zapped.

use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::klock;
use hal::malloc::{M_WAITOK, MArc, MBox};
use hal::owned::OwnedSlist;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{BoxPool, PR_WAITOK, PoolBox};
use hal::rwlock::Guarded;
use hal::sleep::{INFSLP, PCATCH, tsleep_nsec, wakeup};
use hal::slot::LoanSlot;
use hal::sys::errno::Errno;
use hal::sys::types::Pid;
use hal::user::UserPtr;

use super::nfs::{NFSSVC_ADDSOCK, NFSSVC_NFSD, NfsdArgs, NfsdSrvargs};
use super::nfs_socket::{nfs_send, nfs_sndlock, nfs_sndunlock};
use super::nfs_srvcache::{
    RC_DOIT, RC_DROPIT, RC_REPLY, nfsrv_cleancache, nfsrv_getcache, nfsrv_updatecache,
};
use super::nfs_srvsocket::{nfsrv_dorec, nfsrv_rcv, nfsrv_wakenfsd};
use super::nfs_subs::NFSSTATS;
use super::nfsm_subs::XdrReader;
use super::nfsmount::NfsNam;
use super::nfsproto::{
    NFS_MAXDATA, NFS_MAXDGRAMDATA, NFS_MAXPACKET, NFS_NPROCS, NFS_V2MAXDATA, NFSX_UNSIGNED,
};
use crate::assert::kpanic;
use crate::cred::UcredCell;
use crate::descrip::closef;
use crate::file::{File, FileGlue, FileRef, FileSlot};
use crate::fs::subr::vfs;
use crate::net::mbuf::{M_WAIT, MNext, Mbuf, MbufBox, MbufList, m_get, m_prepend};
use crate::net::proto::PR_CONNREQUIRED;
use crate::net::sockbuf::{PSOCK, SB_NOINTR};
use crate::net::socket::{Socket, SocketRef, solock_shared, soreserve, sosetopt, soshutdown};
use crate::net::syscalls::{getsock, sockargs};
use crate::netinet::in_::in_nam2sin;
use crate::proc::{ProcGlue, Thread, suser};
use crate::sys::mbuf::{MT_SONAME, MT_SOOPTS};
use crate::sys::netinet_in::{IPPORT_RESERVED, IPPROTO_TCP, IPPROTO_UDP};
use crate::sys::netinet_tcp::TCP_NODELAY;
use crate::sys::socket::{AF_INET, SHUT_RDWR, SO_KEEPALIVE, SOCK_DGRAM, SOCK_STREAM, SOL_SOCKET};

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
    /// The duplicate request cache (`nfs_srvcache.c`'s globals).
    pub cache: super::nfs_srvcache::NfsrvCacheState,
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
            cache: super::nfs_srvcache::NfsrvCacheState::new(),
        }
    }
}

impl<G: ProcGlue> Default for NfsrvTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `sys_nfssvc(p, v, retval)`: the NFS server's pseudo system call for the nfsds. Based on
/// the flag value it either adds a socket to the selection list (`NFSSVC_ADDSOCK`, a
/// `struct nfsd_args` at `argp`) or remains in the kernel as an nfsd (`NFSSVC_NFSD`, a
/// `struct nfsd_srvargs` at `argp`, read and dropped) until a signal; anything else is
/// `EINVAL`. Must be super user (`suser`, first); waits while the server's tables are
/// being made again ("nfsd init"). `EINTR` and `ERESTART` (the nfsd's end) are success.
/// `procs` is `nfsrv3_procs[]` (`nfs_serv`'s, which `sysent[155]`'s row hands it).
pub fn sys_nfssvc<G: ProcGlue>(
    p: &Thread<G>,
    flag: i32,
    argp: usize,
    procs: &[NfsrvProc<G>; NFS_NPROCS],
) -> Result<(), Errno> {
    // Must be super user
    suser(p)?;

    let t = &vfs::<G>().nfsrv;
    while t.nfssvc_sockhead_flag.load(Ordering::Relaxed) & SLP_INIT != 0 {
        t.nfssvc_sockhead_flag
            .fetch_or(SLP_WANTINIT, Ordering::Relaxed);
        let _ = tsleep_nsec(
            ptr::from_ref(&t.nfssvc_sockhead),
            PSOCK,
            "nfsd init",
            INFSLP,
        );
    }

    let error = match flag {
        NFSSVC_ADDSOCK => {
            let nfsdarg = UserPtr::<NfsdArgs>::new(argp).copyin()?;
            let fp = getsock(p, nfsdarg.sock)?;
            // Get the client address for connected sockets.
            let nam = if nfsdarg.name == 0 || nfsdarg.namelen == 0 {
                None
            } else {
                // A negative `int` is a huge `size_t` to the C's sockargs: EINVAL.
                let len = usize::try_from(nfsdarg.namelen).unwrap_or(usize::MAX);
                match sockargs(UserPtr::new(nfsdarg.name), len, MT_SONAME) {
                    Ok(nam) => Some(nam),
                    Err(e) => {
                        let _ = fp.frele(Some(p));
                        return Err(e);
                    }
                }
            };
            let error = nfssvc_addsock(&fp, nam, p);
            let _ = fp.frele(Some(p));
            error
        }
        NFSSVC_NFSD => {
            let _nsd: NfsdSrvargs = UserPtr::<NfsdSrvargs>::new(argp).copyin()?;
            let Some(nfsd) = MBox::new(M_NFSD, M_WAITOK, Nfsd::new(p.p_tid.get())) else {
                return Err(Errno::ENOMEM);
            };
            nfssvc_nfsd(&nfsd, p, procs)
        }
        _ => Err(Errno::EINVAL),
    };

    match error {
        Err(Errno::EINTR | Errno::ERESTART) => Ok(()),
        e => e,
    }
}

/// `(struct socket *)fp->f_data` of a file `getsock` found: `panic(9)` for another.
fn so_of_file<G: ProcGlue>(fp: &File<G::FileGlue>) -> &SocketRef<G> {
    match fp.data().and_then(<G::FileGlue as FileGlue>::socket_of) {
        Some(so) => so,
        None => kpanic(format_args!(
            "nfssvc: file {:#x} is not a socket",
            fp.addr()
        )),
    }
}

/// The socket a server socket wraps (`slp->ns_so`), counted for the caller's use.
fn so_of<G: ProcGlue>(slp: &NfssvcSock<G>) -> Option<SocketRef<G>> {
    slp.ns_k.get(&klock::reader()).ns_so.clone()
}

/// `*mtod(m, int32_t *) = 1; m->m_len = sizeof(int32_t)`: an option's value for `sosetopt`.
fn set_mtod_one(m: &mut Mbuf) {
    m.m_len().set(size_of::<i32>() as u32);
    if let Some(d) = m.data_mut() {
        d.copy_from_slice(&1i32.to_ne_bytes());
    }
}

/// `nfssvc_addsock(fp, mynam)`: adds a socket to the list for servicing by nfsds: the one
/// datagram socket (`EPERM` while it serves one already) or a connected stream with its
/// client's address `mynam`; IPv4 UDP and TCP only (`EINVAL`). Its buffers are reserved for
/// `NFS_MAXPACKET`, a stream's keep-alive and `TCP_NODELAY` set, its receives and sends made
/// uninterruptible and without a time limit, and an nfsd woken for it.
pub fn nfssvc_addsock<G: ProcGlue>(
    fp: &FileRef<G::FileGlue>,
    mynam: Option<MbufBox>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let t = &vfs::<G>().nfsrv;
    let so = so_of_file::<G>(fp);
    let mut tslp = None;

    // Add it to the list, as required.
    if i32::from(so.proto().protocol()) == IPPROTO_UDP {
        let Some(udp) = t.nfs_udpsock.get(&klock::reader()).clone() else {
            kpanic(format_args!("nfssvc_addsock: nfsrv_init has not run"));
        };
        if udp.flag() & SLP_VALID != 0 {
            drop(mynam);
            return Err(Errno::EPERM);
        }
        tslp = Some(udp);
    }
    // Allow only IPv4 UDP and TCP sockets.
    let sotype = so.so_type();
    if (sotype != SOCK_STREAM && sotype != SOCK_DGRAM) || so.dom_family() != i32::from(AF_INET) {
        drop(mynam);
        return Err(Errno::EINVAL);
    }

    let siz = if sotype == SOCK_STREAM {
        NFS_MAXPACKET + size_of::<u64>() // sizeof (u_long)
    } else {
        NFS_MAXPACKET
    };
    let reserved = {
        let _l = solock_shared(so);
        soreserve(so, siz as u64, siz as u64)
    };
    if let Err(e) = reserved {
        drop(mynam);
        return Err(e);
    }

    // Set protocol specific options { for now TCP only } and reserve some space. For datagram
    // sockets, this can get called repeatedly for the same socket, but that isn't harmful.
    if sotype == SOCK_STREAM {
        let Some(mut m) = m_get(M_WAIT, MT_SOOPTS) else {
            return Err(Errno::ENOBUFS);
        };
        set_mtod_one(&mut m);
        let _ = sosetopt(so, p, SOL_SOCKET, SO_KEEPALIVE, Some(&mut m));
    }
    if so.dom_family() == i32::from(AF_INET) && i32::from(so.proto().protocol()) == IPPROTO_TCP {
        let Some(mut m) = m_get(M_WAIT, MT_SOOPTS) else {
            return Err(Errno::ENOBUFS);
        };
        set_mtod_one(&mut m);
        let _ = sosetopt(so, p, IPPROTO_TCP, TCP_NODELAY, Some(&mut m));
    }
    {
        let _l = solock_shared(so);
        let mut rcv = so.so_rcv.lock();
        rcv.clear_flags(SB_NOINTR);
        rcv.set_timeo_nsecs(INFSLP);
        drop(rcv);
        let mut snd = so.so_snd.lock();
        snd.clear_flags(SB_NOINTR);
        snd.set_timeo_nsecs(INFSLP);
    }

    // A connected socket's record keeps its client's address; the datagram socket's never
    // reads one (the module's Deviations).
    let slp = if let Some(slp) = tslp {
        drop(mynam);
        slp
    } else {
        let nam = mynam.as_deref().and_then(|m| NfsNam::new(m.data()));
        let Some(slp) = MArc::new(M_NFSSVC, M_WAITOK, NfssvcSock::new(nam)) else {
            return Err(Errno::ENOMEM);
        };
        let mut w = klock::witness();
        t.nfssvc_sockhead.insert_tail(&mut w, slp.as_pin());
        slp.ns_k.get_mut(&mut w).ns_self = Some(slp.clone());
        slp
    };
    slp.ns_sotype.store(sotype, Ordering::Relaxed);
    let old_fp = {
        let mut w = klock::witness();
        let k = slp.ns_k.get_mut(&mut w);
        k.ns_so = Some(so.clone());
        k.ns_fp.replace(fp.fref())
    };
    drop(old_fp);
    // so->so_upcallarg = slp; so->so_upcall = nfsrv_rcv: the record's handle, counted (the
    // function is the glue's registered one, nfs_srvsocket's nfsrv_rcv_upcall). What the slot
    // held (this record again, for a datagram socket added twice) is not the last handle.
    drop(so.set_upcall(Some(MArc::erase(slp.clone()))));
    slp.ns_flag.store(SLP_VALID | SLP_NEEDQ, Ordering::Relaxed);
    nfsrv_wakenfsd(&slp);
    Ok(())
}

/// `nfssvc_checknam(nam)`: whether the client's address is acceptable: an IPv4 address with
/// a reserved source port.
pub fn nfssvc_checknam(nam: Option<&Mbuf>) -> bool {
    let Some(nam) = nam else {
        return false;
    };
    match in_nam2sin(nam) {
        Ok(sin) => i32::from(u16::from_be(sin.sin_port)) < IPPORT_RESERVED,
        Err(_) => false,
    }
}

/// `nfssvc_nfsd(nfsd)`: called by `nfssvc()` for nfsds. Just loops around servicing rpc
/// requests until it is killed by a signal: a socket found (`nfsrv_getslp`), its data
/// received if it has some (under its send lock), a record taken (`nfsrv_dorec`), the cache
/// asked (`nfsrv_getcache`), a request from an unreserved port dropped (but for the NULL
/// procedure), the procedure run, the cache told, the reply sent (a record mark first on a
/// stream, under the socket's send lock), the socket zapped on `EPIPE`. The nfsd's own frame
/// holds its socket and its request; the last nfsd out makes the tables again
/// (`nfsrv_init(1)`).
pub fn nfssvc_nfsd<G: ProcGlue>(
    nfsd: &MBox<Nfsd<G>>,
    p: &Thread<G>,
    procs: &[NfsrvProc<G>; NFS_NPROCS],
) -> Result<(), Errno> {
    nfsd_enter(nfsd);
    let mut frame = NfsdFrame::default();
    let error = loop {
        if let Err(e) = nfsd_step(nfsd, p, procs, &mut frame) {
            break e;
        }
    };
    // done:
    nfsd_leave(nfsd);
    Err(error)
}

/// The socket an nfsd serves (`nfsd->nfsd_slp` while it holds a request) and the request it
/// holds (`nd`): its loop's frame, from one pass to the next.
pub struct NfsdFrame<G: ProcGlue> {
    /// The socket of the request in progress (`NFSD_REQINPROG`).
    pub cur: Option<NfssvcSockRef<G>>,
    /// The request taken (`nfsrv_dorec`'s).
    pub nd: Option<PoolBox<NfsrvDescript>>,
}

impl<G: ProcGlue> Default for NfsdFrame<G> {
    fn default() -> Self {
        Self {
            cur: None,
            nd: None,
        }
    }
}

/// `nfssvc_nfsd`'s start: the nfsd on `nfsd_head`, counted in `nfs_numnfsd`.
pub fn nfsd_enter<G: ProcGlue>(nfsd: &MBox<Nfsd<G>>) {
    let t = &vfs::<G>().nfsrv;
    {
        let mut w = klock::witness();
        t.nfsd_head.insert_tail(&mut w, nfsd.as_pin());
    }
    t.nfs_numnfsd.fetch_add(1, Ordering::Relaxed);
}

/// `nfssvc_nfsd`'s `done:`: the nfsd off `nfsd_head` (the caller frees it), and the tables
/// made again when it was the last (`nfsrv_init(1)`).
pub fn nfsd_leave<G: ProcGlue>(nfsd: &MBox<Nfsd<G>>) {
    let t = &vfs::<G>().nfsrv;
    {
        let mut w = klock::witness();
        t.nfsd_head.remove(&mut w, nfsd);
    }
    if t.nfs_numnfsd.fetch_sub(1, Ordering::Relaxed) == 1 {
        nfsrv_init::<G>(true); // Reinitialize everything
    }
}

/// One pass of `nfssvc_nfsd`'s loop, from its `loop:` to its next `goto loop`: `Err` is its
/// `goto done` with the error the nfsd ends with.
pub fn nfsd_step<G: ProcGlue>(
    nfsd: &Nfsd<G>,
    p: &Thread<G>,
    procs: &[NfsrvProc<G>; NFS_NPROCS],
    frame: &mut NfsdFrame<G>,
) -> Result<(), Errno> {
    let mut error = Ok(());
    let slp = if nfsd.flag() & NFSD_REQINPROG == 0 {
        // attach an nfssvc_sock to nfsd
        let slp = nfsrv_getslp(nfsd)?;
        if slp.flag() & SLP_VALID != 0 {
            if slp.flag() & (SLP_DISCONN | SLP_NEEDQ) == SLP_NEEDQ {
                slp.clear_flags(SLP_NEEDQ);
                let _ = nfs_sndlock::<G>(&slp.ns_solock, None);
                if let Some(so) = so_of(&slp) {
                    nfsrv_rcv(&so, &slp, M_WAIT);
                }
                nfs_sndunlock(&slp.ns_solock);
            }
            if slp.flag() & SLP_DISCONN != 0 {
                nfsrv_zapsock(&slp);
            }
            match nfsrv_dorec(&slp) {
                Ok(n) => frame.nd = Some(n),
                Err(e) => error = Err(e),
            }
            nfsd.nfsd_flag.fetch_or(NFSD_REQINPROG, Ordering::Relaxed);
        }
        slp
    } else {
        match frame.cur.take() {
            Some(s) => s,
            None => kpanic(format_args!(
                "nfssvc_nfsd: a request in progress, no socket"
            )),
        }
    };

    if error.is_err() || slp.flag() & SLP_VALID == 0 {
        frame.nd = None;
        nfsd.nfsd_flag.fetch_and(!NFSD_REQINPROG, Ordering::Relaxed);
        nfsrv_slpderef(slp);
        return Ok(());
    }

    let Some(so) = so_of(&slp) else {
        kpanic(format_args!("nfssvc_nfsd: a valid socket wraps no socket"));
    };
    let sotype = so.so_type();
    let connected = so.pr_flags(PR_CONNREQUIRED);
    let Some(mut req) = frame.nd.take() else {
        kpanic(format_args!("nfssvc_nfsd: no request descriptor"));
    };
    if req.nd_nam2.is_none() {
        // nd->nd_nam = slp->ns_nam: the socket's name mbuf lent (a copy while another nfsd
        // holds it).
        req.nd_nam = slp
            .ns_namm
            .take()
            .or_else(|| slp.ns_nam.as_ref().and_then(|n| n.mbuf(M_WAIT)));
    }

    let (cacherep, mut mreq) = nfsrv_getcache(&mut req, &slp);
    let mut send = cacherep == RC_REPLY;
    if cacherep == RC_DOIT {
        // Unless this is a null request (server ping), make sure that the client is using a
        // reserved source port.
        if req.nd_procnum != 0 && !nfssvc_checknam(req.nam()) {
            // drop it
            req.nd_mrep = None;
            req.nd_nam2 = None;
        } else {
            let Some(proc_fn) = procs.get(req.nd_procnum) else {
                kpanic(format_args!(
                    "nfssvc_nfsd: procedure {} past the table",
                    req.nd_procnum
                ));
            };
            let (m, err) = proc_fn(&mut req, &slp, p);
            mreq = m;
            if mreq.is_none() {
                req.nd_nam2 = None;
                req.nd_mrep = None;
            } else if err != 0 {
                NFSSTATS.srv_errs.fetch_add(1, Ordering::Relaxed);
                nfsrv_updatecache::<G>(&req, false, mreq.as_deref_mut());
                req.nd_nam2 = None;
                mreq = None;
            } else {
                NFSSTATS.srvrpccnt[req.nd_procnum].fetch_add(1, Ordering::Relaxed);
                nfsrv_updatecache::<G>(&req, true, mreq.as_deref_mut());
                req.nd_mrep = None;
                send = true;
            }
        }
    } else if cacherep == RC_DROPIT {
        req.nd_mrep = None;
        req.nd_nam2 = None;
    }

    if send {
        let Some(m) = mreq.take() else {
            kpanic(format_args!("nfssvc_nfsd: a reply to send is missing"));
        };
        let err = nfsrv_send_reply(&slp, &so, sotype, connected, &mut req, m);
        if let Err(e @ (Errno::EINTR | Errno::ERESTART)) = err {
            drop(slp.ns_namm.replace(req.nd_nam.take()));
            drop(req);
            nfsrv_slpderef(slp);
            return Err(e);
        }
    }

    // The connection's name back for the next request; the descriptor to its pool.
    if req.nd_nam.is_some() {
        drop(slp.ns_namm.replace(req.nd_nam.take()));
    }
    drop(req);

    if let Ok(n) = nfsrv_dorec(&slp) {
        frame.nd = Some(n);
        frame.cur = Some(slp);
    } else {
        nfsd.nfsd_flag.fetch_and(!NFSD_REQINPROG, Ordering::Relaxed);
        nfsrv_slpderef(slp);
    }
    Ok(())
}

/// `nfssvc_nfsd`'s `RC_REPLY` case: the reply `m` sized (`panic(9)` for an empty one or one
/// past `NFS_MAXPACKET`, as the C), a Sun RPC record mark prepended on a stream, sent to the
/// request's sender (under the socket's send lock when it is connected) unless the socket is
/// no longer valid (`EPIPE`), the request's mbufs freed, the socket zapped on `EPIPE`.
fn nfsrv_send_reply<G: ProcGlue>(
    slp: &NfssvcSock<G>,
    so: &Socket<G>,
    sotype: i32,
    connected: bool,
    req: &mut NfsrvDescript,
    m: MbufBox,
) -> Result<(), Errno> {
    let mut siz = 0usize;
    let mut c: Option<&Mbuf> = Some(&m);
    while let Some(mm) = c {
        siz += mm.m_len().get() as usize;
        c = mm.m_next.get();
    }
    if siz == 0 || siz > NFS_MAXPACKET {
        kpanic(format_args!("bad nfs svc reply, siz = {siz}"));
    }
    // `siz <= NFS_MAXPACKET`: an `int`.
    if let Some(h) = m.pkthdr() {
        h.len.set(siz as i32);
        h.ph_ifidx.set(0);
    }

    // For stream protocols, prepend a Sun RPC Record Mark.
    let m = if sotype == SOCK_STREAM {
        m_prepend(m, NFSX_UNSIGNED as i32, M_WAIT).map(|mut m| {
            if let Some(d) = m.data_mut() {
                d[..NFSX_UNSIGNED].copy_from_slice(&(0x8000_0000u32 | siz as u32).to_be_bytes());
            }
            m
        })
    } else {
        Some(m)
    };

    if connected {
        let _ = nfs_sndlock::<G>(&slp.ns_solock, None);
    }
    let error = match m {
        Some(m) if slp.flag() & SLP_VALID != 0 => {
            nfs_send(Some(so), req.nd_nam2.as_deref(), m, None)
        }
        Some(m) => {
            drop(m);
            Err(Errno::EPIPE)
        }
        // m_prepend found no mbuf (it frees the reply): the C's M_WAIT cannot fail.
        None => Err(Errno::ENOBUFS),
    };
    req.nd_nam2 = None;
    req.nd_mrep = None;
    if error == Err(Errno::EPIPE) {
        nfsrv_zapsock(slp);
    }
    if connected {
        nfs_sndunlock(&slp.ns_solock);
    }
    error
}

/// `nfsrv_zapsock(slp)`: shuts down a socket associated with an `nfssvc_sock` structure.
/// Should be called with the send lock set, if required. The trick here is to increment the
/// sref at the start, so that the nfsds will stop using it and clear `ns_flag` at the end so
/// that it will not be reassigned during cleanup: its flags cleared, the file it holds closed
/// (the socket shut down first), the client's address and every mbuf it queued freed.
pub fn nfsrv_zapsock<G: ProcGlue>(slp: &NfssvcSock<G>) {
    slp.clear_flags(SLP_ALLFLAGS);
    let (fp, so) = {
        let mut w = klock::witness();
        let k = slp.ns_k.get_mut(&mut w);
        (k.ns_fp.take(), k.ns_so.clone())
    };
    let Some(fp) = fp else {
        return;
    };
    if let Some(so) = so {
        // so->so_upcall = NULL: the socket's handle of this record goes (never its last: the
        // caller holds the record, and the list's membership holds one).
        drop(so.set_upcall(None));
        let _ = soshutdown(&so, SHUT_RDWR);
    }
    let _ = closef(fp, None);
    drop(slp.ns_namm.take());
    drop(slp.ns_rcv.take());
}

/// `nfsrv_slpderef(slp)`: dereferences a server socket structure (the caller's handle, which
/// goes). If it has no more references and is no longer valid, it leaves the list and is
/// freed with its last handle; the datagram socket's record stays for the next
/// `NFSSVC_ADDSOCK` (LZ's).
pub fn nfsrv_slpderef<G: ProcGlue>(slp: NfssvcSockRef<G>) {
    let t = &vfs::<G>().nfsrv;
    let sref = slp.ns_sref.fetch_sub(1, Ordering::Relaxed).wrapping_sub(1);
    if sref == 0 && slp.flag() & SLP_VALID == 0 {
        let member = {
            let mut w = klock::witness();
            let udp = t
                .nfs_udpsock
                .get(&w)
                .as_ref()
                .is_some_and(|u| MArc::ptr_eq(u, &slp));
            if udp {
                None
            } else {
                t.nfssvc_sockhead.remove(&mut w, &slp);
                slp.ns_k.get_mut(&mut w).ns_self.take()
            }
        };
        drop(member);
    }
    drop(slp);
}

/// `nfsrv_init(terminating)`: initializes the data structures for the server, handshaking
/// with any new nfsds starting up to avoid any chance of corruption (`SLP_INIT`, "nfsd
/// init"): when `terminating`, every socket zapped and freed and the cache cleaned; then the
/// datagram socket's record made, and at boot the descriptors' pool.
pub fn nfsrv_init<G: ProcGlue>(terminating: bool) {
    let t = &vfs::<G>().nfsrv;
    if t.nfssvc_sockhead_flag.load(Ordering::Relaxed) & SLP_INIT != 0 {
        kpanic(format_args!("nfsd init"));
    }
    t.nfssvc_sockhead_flag.fetch_or(SLP_INIT, Ordering::Relaxed);
    if terminating {
        loop {
            let slp = {
                let r = klock::reader();
                let Some(first) = t.nfssvc_sockhead.first(&r) else {
                    break;
                };
                match first.ns_k.get(&r).ns_self.clone() {
                    Some(s) => s,
                    None => kpanic(format_args!(
                        "nfsrv_init: a socket on the list without its handle"
                    )),
                }
            };
            if slp.flag() & SLP_VALID != 0 {
                nfsrv_zapsock(&slp);
            }
            let member = {
                let mut w = klock::witness();
                t.nfssvc_sockhead.remove(&mut w, &slp);
                slp.ns_k.get_mut(&mut w).ns_self.take()
            };
            drop(member);
            drop(slp);
        }
        nfsrv_cleancache::<G>(); // And clear out server cache
    }

    // The datagram socket's record, made while SLP_INIT still holds `nfssvc(2)` back (the
    // module's Deviations: the C makes it after the wakeup, a window in which an
    // NFSSVC_ADDSOCK reads the record it freed).
    let Some(udp) = MArc::new(M_NFSSVC, M_WAITOK, NfssvcSock::new(None)) else {
        kpanic(format_args!(
            "nfsrv_init: no memory for the datagram socket"
        ));
    };
    let old = {
        let mut w = klock::witness();
        t.nfssvc_sockhead.insert_head(&mut w, udp.as_pin());
        udp.ns_k.get_mut(&mut w).ns_self = Some(udp.clone());
        t.nfs_udpsock.get_mut(&mut w).replace(udp)
    };
    drop(old);

    t.nfssvc_sockhead_flag
        .fetch_and(!SLP_INIT, Ordering::Relaxed);
    if t.nfssvc_sockhead_flag.load(Ordering::Relaxed) & SLP_WANTINIT != 0 {
        t.nfssvc_sockhead_flag
            .fetch_and(!SLP_WANTINIT, Ordering::Relaxed);
        wakeup(ptr::from_ref(&t.nfssvc_sockhead));
    }

    t.nfsd_head_flag
        .fetch_and(!NFSD_CHECKSLP, Ordering::Relaxed);

    if !terminating {
        let _ = t
            .nfsrv_descript_pl
            .init(hal::intr::IPL_NONE, PR_WAITOK, "ndscpl");
    }
}

/// `nfsrv_getslp(nfsd)`: finds an `nfssvc_sock` for the nfsd, sleeping if needed ("nfsd",
/// interruptible: a signal's error ends the nfsd): the socket a waker handed it, or one
/// marked `SLP_DOREC` when `NFSD_CHECKSLP` says some wait. The handle comes into the nfsd's
/// frame, counted in the socket's `ns_sref`.
pub fn nfsrv_getslp<G: ProcGlue>(nfsd: &Nfsd<G>) -> Result<NfssvcSockRef<G>, Errno> {
    let t = &vfs::<G>().nfsrv;
    loop {
        // again:
        while nfsd.nfsd_slp.update(|s| s.is_none())
            && t.nfsd_head_flag.load(Ordering::Relaxed) & NFSD_CHECKSLP == 0
        {
            nfsd.nfsd_flag.fetch_or(NFSD_WAITING, Ordering::Relaxed);
            t.nfsd_waiting.fetch_add(1, Ordering::Relaxed);
            let error = tsleep_nsec(ptr::from_ref(nfsd), PSOCK | PCATCH, "nfsd", INFSLP);
            t.nfsd_waiting.fetch_sub(1, Ordering::Relaxed);
            error?;
        }

        if nfsd.nfsd_slp.update(|s| s.is_none())
            && t.nfsd_head_flag.load(Ordering::Relaxed) & NFSD_CHECKSLP != 0
        {
            let r = klock::reader();
            let found = t
                .nfssvc_sockhead
                .iter(&r)
                .find(|slp| slp.flag() & (SLP_VALID | SLP_DOREC) == SLP_VALID | SLP_DOREC);
            match found.and_then(|slp| slp.ns_k.get(&r).ns_self.clone()) {
                Some(slp) => {
                    slp.clear_flags(SLP_DOREC);
                    slp.ns_sref.fetch_add(1, Ordering::Relaxed);
                    drop(nfsd.nfsd_slp.replace(Some(slp)));
                }
                None => {
                    t.nfsd_head_flag
                        .fetch_and(!NFSD_CHECKSLP, Ordering::Relaxed);
                }
            }
        }

        if let Some(slp) = nfsd.nfsd_slp.take() {
            return Ok(slp);
        }
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

/// The host tests' reset of a test glue's server tables (testing.md's `*_test_reset`): the
/// sockets and nfsds an earlier test left linked unlinked and forgotten with their handles
/// (their memory is gone with that test's), the flags and counts back to the C's start, the
/// cache's reset. Nothing is freed. The caller holds the kernel lock (the set-ups'):
/// `vfs_test_reset` calls it.
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn nfsrv_test_reset<G: ProcGlue>() {
    let t = &vfs::<G>().nfsrv;
    {
        let mut w = klock::witness();
        while t.nfssvc_sockhead.remove_first(&mut w) {}
        while t.nfsd_head.remove_first(&mut w) {}
        core::mem::forget(t.nfs_udpsock.get_mut(&mut w).take());
    }
    t.nfssvc_sockhead_flag.store(0, Ordering::Relaxed);
    t.nfsd_head_flag.store(0, Ordering::Relaxed);
    t.nfs_numnfsd.store(0, Ordering::Relaxed);
    t.nfsd_waiting.store(0, Ordering::Relaxed);
    super::nfs_srvcache::nfsrv_cache_test_reset::<G>();
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

    // The server over real kernel sockets on lo0 (the nfs client's fixture, `testnfs`): the
    // nfsd's loop driven pass by pass in the test's thread (`nfsd_step`; a host sleep has no
    // signal to end it), the socket's upcall made by hand where a test drives it (the
    // datagram test also lets the registered one run), the client a
    // kernel socket of the same glue; then nfssvc(2)'s errnos.

    mod sock {
        use core::cell::Cell;
        use std::vec::Vec;

        use hal::sys::netinet_in::SockaddrIn;
        use hal::sys::socket::{AF_INET, MSG_DONTWAIT, SO_LINGER, SOCK_DGRAM, SOCK_STREAM};

        use super::super::*;
        use crate::descrip::{fdinit, sys_close};
        use crate::fs::nfs::nfs_srvsocket::nfs_rephead;
        use crate::fs::nfs::nfsm_subs::test_bytes;
        use crate::fs::nfs::testnfs::{self, Fixture, G, lo_sin, name, pump};
        use crate::net::mbuf::{M_WAIT, MHLEN, MT_DATA, m_gethdr, mclget};
        use crate::net::socket::{
            sobind, soclose, soconnect, socreate, solisten, solock, soreceive, sosend,
        };
        use crate::net::syscalls::{sys_bind, sys_socket};
        use crate::resource::{Plimit, PlimitRef, RLIM_INFINITY, RLIMIT_NOFILE};
        use crate::testutil::USER_BASE;

        /// The server's port.
        const PORT: u16 = 2049;
        /// Where the tests put `struct nfsd_args` and the client's address in user memory.
        const UARGS: usize = USER_BASE + 0x2_0000;
        /// The client's name in user memory.
        const UNAME: usize = USER_BASE + 0x2_0100;

        std::thread_local! {
            /// The client's port each procedure of the test table saw (`nd.nam()`).
            static SEEN: Cell<Option<u16>> = const { Cell::new(None) };
        }

        /// The test table's procedure: a reply of the RPC header and the procedure's number,
        /// the sender's port noted.
        fn tproc(
            nd: &mut NfsrvDescript,
            slp: &NfssvcSock<G>,
            _p: &Thread<G>,
        ) -> (Option<MbufBox>, i32) {
            let port = nd
                .nam()
                .map(|m| u16::from_be_bytes([m.data()[2], m.data()[3]]));
            SEEN.with(|s| s.set(port));
            let mut w = nfs_rephead(4, nd, slp, 0);
            w.u32(nd.nd_procnum as u32);
            (Some(w.into_chain()), 0)
        }

        const PROCS: [NfsrvProc<G>; NFS_NPROCS] = [tproc; NFS_NPROCS];

        /// A version 3 call of procedure `proc_`, xid `xid`, AUTH_UNIX as root.
        fn call(xid: u32, proc_: u32) -> Vec<u8> {
            let w: [u32; 15] = [xid, 0, 2, 100_003, 3, proc_, 1, 20, 0, 0, 0, 0, 0, 0, 0];
            w.iter().flat_map(|x| x.to_be_bytes()).collect()
        }

        /// The fixture as root, with a descriptor table, the server's tables made afresh.
        fn server() -> Fixture {
            let fx = testnfs::setup(0);
            let pr = fx.user.process();
            pr.set_fd(fdinit());
            let mut limits = Plimit::new();
            for l in &mut limits.pl_rlimit {
                l.rlim_cur = RLIM_INFINITY;
                l.rlim_max = RLIM_INFINITY;
            }
            limits.pl_rlimit[RLIMIT_NOFILE].rlim_cur = 64;
            drop(pr.set_limit(PlimitRef::new(limits)));
            crate::testutil::set_test_maxfiles(4096);
            // `testnfs::setup`'s `vfs_test_reset` reset the server's tables, and its
            // `nfs_init` made them again (`nfsrv_init(0)`, `nfsrv_initcache`).
            fx
        }

        /// A socket descriptor of type `ty` bound to 127.0.0.1:`port`.
        fn bound(fx: &Fixture, ty: i32, port: u16) -> i32 {
            let fd = sys_socket(fx.user, i32::from(AF_INET), ty, 0).expect("a socket");
            let sin = lo_sin(port);
            UserPtr::<SockaddrIn>::new(UNAME)
                .copyout(&sin)
                .expect("user memory");
            sys_bind(fx.user, fd, UserPtr::new(UNAME), 16).expect("bind");
            fd
        }

        /// `NFSSVC_ADDSOCK` of descriptor `fd` (with the client's address `nam`).
        fn addsock(fx: &Fixture, fd: i32, nam: Option<&SockaddrIn>) -> Result<(), Errno> {
            let mut a = NfsdArgs {
                sock: fd,
                ..NfsdArgs::default()
            };
            if let Some(sin) = nam {
                UserPtr::<SockaddrIn>::new(UNAME)
                    .copyout(sin)
                    .expect("user memory");
                a.name = UNAME;
                a.namelen = 16;
            }
            UserPtr::<NfsdArgs>::new(UARGS)
                .copyout(&a)
                .expect("user memory");
            sys_nfssvc(fx.user, NFSSVC_ADDSOCK, UARGS, &PROCS)
        }

        /// The record of the socket of descriptor `fd`.
        fn record(fd_so: &SocketRef<G>) -> NfssvcSockRef<G> {
            let t = &vfs::<G>().nfsrv;
            let r = klock::reader();
            let found = t.nfssvc_sockhead.iter(&r).find(|s| {
                s.ns_k
                    .get(&r)
                    .ns_so
                    .as_ref()
                    .is_some_and(|so| core::ptr::eq::<Socket<G>>(&**so, &**fd_so))
            });
            match found.and_then(|s| s.ns_k.get(&r).ns_self.clone()) {
                Some(s) => s,
                None => panic!("no record for the socket"),
            }
        }

        /// The socket at descriptor `fd`.
        fn sock_of(fx: &Fixture, fd: i32) -> SocketRef<G> {
            let fp = crate::net::syscalls::getsock(fx.user, fd).expect("a socket");
            let so = so_of_file::<G>(&fp).clone();
            let _ = fp.frele(Some(fx.user));
            so
        }

        /// Sends `b` on `so` (to `to`), without waiting.
        fn send(so: &SocketRef<G>, to: Option<&MbufBox>, b: &[u8]) {
            let mut m = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
            if b.len() > MHLEN {
                mclget(&mut m, M_WAIT);
            }
            m.m_len().set(b.len() as u32);
            m.m_pkthdr().len.set(b.len() as i32);
            m.data_mut().expect("room").copy_from_slice(b);
            assert_eq!(
                sosend(so, to.map(|t| &**t), None, Some(m), None, MSG_DONTWAIT),
                Ok(())
            );
        }

        /// The bytes waiting on `so` (one datagram, or the stream's), without waiting.
        fn recv(so: &SocketRef<G>) -> Option<Vec<u8>> {
            let mut uio = crate::fs::nfs::nfs_socket::count_uio::<G>(1 << 20, None);
            let (mut m, mut flags) = (None, MSG_DONTWAIT);
            soreceive(so, None, &mut uio, Some(&mut m), None, Some(&mut flags), 0).ok()?;
            m.as_deref().map(test_bytes)
        }

        /// A client socket of type `ty` bound to 127.0.0.1:`port`.
        fn client(fx: &Fixture, ty: i32, port: u16) -> SocketRef<G> {
            let so = socreate(fx.user, i32::from(AF_INET), ty, 0).expect("a socket");
            let nam = name(&lo_sin(port));
            let mut w = solock(&so);
            sobind(&so, &mut w, &nam, fx.user).expect("bind");
            drop(w);
            so
        }

        /// Closes `so` with a zero linger (a reset, no TIME_WAIT left behind).
        fn abort(so: &SocketRef<G>) {
            {
                let w = solock(so);
                so.set_options(&w, SO_LINGER, true);
            }
            let _ = soclose(so, 0);
        }

        /// A fresh nfsd of the test's thread.
        fn nfsd(fx: &Fixture) -> MBox<Nfsd<G>> {
            MBox::new(M_NFSD, M_WAITOK, Nfsd::new(fx.user.p_tid.get())).expect("an nfsd")
        }

        #[test]
        fn a_datagram_request_is_answered_and_the_last_nfsd_out_resets_the_server() {
            let fx = server();
            let t = &vfs::<G>().nfsrv;
            let fd = bound(&fx, SOCK_DGRAM, PORT);
            assert_eq!(addsock(&fx, fd, None), Ok(()));
            let so = sock_of(&fx, fd);
            let udp = t
                .nfs_udpsock
                .get(&klock::reader())
                .clone()
                .expect("the record");
            // No nfsd waits yet: the socket is also marked for the first that looks.
            assert_eq!(udp.flag(), SLP_VALID | SLP_NEEDQ | SLP_DOREC);
            // A second datagram socket is refused while the first serves.
            let fd2 = bound(&fx, SOCK_DGRAM, PORT + 1);
            assert_eq!(addsock(&fx, fd2, None), Err(Errno::EPERM));

            // A client on a reserved port asks for procedure 1, another on 1024 for
            // procedure 1 too (dropped) and for the NULL procedure (answered).
            let c = client(&fx, SOCK_DGRAM, 700);
            let c2 = client(&fx, SOCK_DGRAM, 1024);
            let to = name(&lo_sin(PORT));
            send(&c, Some(&to), &call(11, 1));
            send(&c2, Some(&to), &call(12, 1));
            send(&c2, Some(&to), &call(13, 0));
            pump(fx.lo);
            // The upcall: no nfsd waits, so the socket is marked for the next that looks.
            nfsrv_rcv(&so, &udp, crate::net::mbuf::M_DONTWAIT);
            assert_ne!(udp.flag() & SLP_DOREC, 0);
            assert_ne!(t.nfsd_head_flag.load(Ordering::Relaxed) & NFSD_CHECKSLP, 0);

            let n = nfsd(&fx);
            nfsd_enter(&n);
            let mut frame = NfsdFrame::default();
            // Pass 1: the socket found by the walk, its datagrams received, the first served.
            assert_eq!(nfsd_step(&n, fx.user, &PROCS, &mut frame), Ok(()));
            assert_eq!(SEEN.with(Cell::take), Some(700));
            assert!(frame.cur.is_some(), "more records queued");
            // Pass 2: the unreserved request dropped; pass 3: the NULL one answered.
            assert_eq!(nfsd_step(&n, fx.user, &PROCS, &mut frame), Ok(()));
            assert_eq!(SEEN.with(Cell::take), None);
            assert_eq!(nfsd_step(&n, fx.user, &PROCS, &mut frame), Ok(()));
            assert_eq!(SEEN.with(Cell::take), Some(1024));
            assert!(frame.cur.is_none() && frame.nd.is_none());
            assert_eq!(n.flag() & NFSD_REQINPROG, 0);
            assert_eq!(udp.ns_sref.load(Ordering::Relaxed), 0);
            pump(fx.lo);

            let r = recv(&c).expect("a reply");
            assert_eq!(&r[..4], &11u32.to_be_bytes());
            assert_eq!(&r[r.len() - 4..], &1u32.to_be_bytes());
            let r2 = recv(&c2).expect("the NULL reply");
            assert_eq!(&r2[..4], &13u32.to_be_bytes());
            assert!(recv(&c2).is_none(), "no reply from an unreserved port");
            assert!(recv(&c).is_none());

            // A retransmission of the served (idempotent) request is done again. No call of
            // nfsrv_rcv by hand: the socket's upcall (`nfs_init` registered it,
            // `nfssvc_addsock` set it with the record's handle) marks the socket.
            assert_eq!(udp.flag() & SLP_NEEDQ, 0);
            assert!(
                so.upcall_arg().is_some_and(|a| a
                    .downcast_ref::<NfssvcSock<G>>()
                    .is_some_and(|s| core::ptr::eq(s, &*udp))),
                "so_upcallarg is the record"
            );
            send(&c, Some(&to), &call(11, 1));
            pump(fx.lo);
            assert_ne!(udp.flag() & SLP_NEEDQ, 0, "the upcall marked the socket");
            assert_eq!(nfsd_step(&n, fx.user, &PROCS, &mut frame), Ok(()));
            assert_eq!(SEEN.with(Cell::take), Some(700));
            pump(fx.lo);
            assert!(recv(&c).is_some());

            // The last nfsd out: every socket zapped, a fresh datagram record.
            nfsd_leave(&n);
            drop(n);
            assert_eq!(udp.flag(), 0);
            assert!(so.upcall_arg().is_none(), "zapped: so_upcall is NULL");
            let fresh = t
                .nfs_udpsock
                .get(&klock::reader())
                .clone()
                .expect("a new record");
            assert!(!MArc::ptr_eq(&fresh, &udp));
            assert_eq!(t.nfs_numnfsd.load(Ordering::Relaxed), 0);
            assert_eq!(t.nfssvc_sockhead_flag.load(Ordering::Relaxed) & SLP_INIT, 0);

            // A restart: the second datagram socket, refused before, is the fresh record's
            // (the old one, zapped and off the list, stays untouched), and an nfsd serves it.
            assert_eq!(addsock(&fx, fd2, None), Ok(()));
            assert_eq!(udp.flag(), 0);
            assert_eq!(fresh.flag() & SLP_VALID, SLP_VALID);
            let so2 = sock_of(&fx, fd2);
            send(&c, Some(&name(&lo_sin(PORT + 1))), &call(31, 1));
            pump(fx.lo);
            nfsrv_rcv(&so2, &fresh, crate::net::mbuf::M_DONTWAIT);
            let n = nfsd(&fx);
            nfsd_enter(&n);
            let mut frame = NfsdFrame::default();
            assert_eq!(nfsd_step(&n, fx.user, &PROCS, &mut frame), Ok(()));
            assert_eq!(SEEN.with(Cell::take), Some(700));
            pump(fx.lo);
            let r = recv(&c).expect("the restarted server's reply");
            assert_eq!(&r[..4], &31u32.to_be_bytes());
            nfsd_leave(&n);
            drop(n);
            assert_eq!(fresh.flag(), 0);
            drop((fresh, udp, so, so2));
            abort(&c);
            abort(&c2);
            assert_eq!(sys_close(fx.user, fd), Ok(()));
            assert_eq!(sys_close(fx.user, fd2), Ok(()));
        }

        #[test]
        fn stream_records_are_answered_with_their_marks_to_the_connections_client() {
            let fx = server();
            let lfd = bound(&fx, SOCK_STREAM, PORT);
            let l = sock_of(&fx, lfd);
            {
                let mut w = solock(&l);
                solisten(&l, &mut w, 5).expect("listen");
            }
            let c = client(&fx, SOCK_STREAM, 701);
            {
                let mut w = solock(&c);
                soconnect(&c, &mut w, &name(&lo_sin(PORT))).expect("connect");
            }
            for _ in 0..4 {
                pump(fx.lo);
            }
            // The server's end of the connection, at a descriptor of its own (nfsd(8)'s
            // accept), then added with the client's address.
            UserPtr::<u32>::new(UARGS + 0x200)
                .copyout(&16)
                .expect("user memory");
            let afd = crate::net::syscalls::sys_accept(
                fx.user,
                lfd,
                UserPtr::new(UARGS + 0x100),
                UserPtr::new(UARGS + 0x200),
            )
            .expect("a connection");
            let acc = sock_of(&fx, afd);
            let peer = lo_sin(701);
            assert_eq!(addsock(&fx, afd, Some(&peer)), Ok(()));
            let slp = record(&acc);
            assert_eq!(slp.sotype(), SOCK_STREAM);
            assert!(slp.ns_nam.is_some());

            // Two records, the second in two fragments, cut across segments.
            let mut s = Vec::new();
            let r1 = call(21, 1);
            s.extend((0x8000_0000u32 | r1.len() as u32).to_be_bytes());
            s.extend(&r1);
            let r2 = call(22, 4);
            s.extend((10u32).to_be_bytes());
            s.extend(&r2[..10]);
            s.extend((0x8000_0000u32 | (r2.len() - 10) as u32).to_be_bytes());
            s.extend(&r2[10..]);
            // No Nagle on the client: both segments leave at once.
            let mut opt =
                crate::net::mbuf::m_get(M_WAIT, crate::sys::mbuf::MT_SOOPTS).expect("an mbuf");
            set_mtod_one(&mut opt);
            assert_eq!(
                sosetopt(
                    &c,
                    fx.user,
                    crate::sys::netinet_in::IPPROTO_TCP,
                    crate::sys::netinet_tcp::TCP_NODELAY,
                    Some(&mut opt)
                ),
                Ok(())
            );
            send(&c, None, &s[..7]);
            send(&c, None, &s[7..]);
            for _ in 0..4 {
                pump(fx.lo);
            }
            nfsrv_rcv(&acc, &slp, crate::net::mbuf::M_DONTWAIT);

            let n = nfsd(&fx);
            nfsd_enter(&n);
            let mut frame = NfsdFrame::default();
            assert_eq!(nfsd_step(&n, fx.user, &PROCS, &mut frame), Ok(()));
            // The connection's client, as the address the procedure sees.
            assert_eq!(SEEN.with(Cell::take), Some(701));
            assert!(frame.cur.is_some());
            assert_eq!(nfsd_step(&n, fx.user, &PROCS, &mut frame), Ok(()));
            assert_eq!(SEEN.with(Cell::take), Some(701));
            assert!(frame.cur.is_none());
            for _ in 0..3 {
                pump(fx.lo);
            }
            let got = recv(&c).expect("the replies");
            // Two records, each with its mark (last fragment) and the procedure's word.
            let len1 = u32::from_be_bytes(got[..4].try_into().unwrap());
            assert_ne!(len1 & 0x8000_0000, 0);
            let len1 = (len1 & 0x7fff_ffff) as usize;
            assert_eq!(&got[4..8], &21u32.to_be_bytes());
            assert_eq!(&got[4 + len1 - 4..4 + len1], &1u32.to_be_bytes());
            let rest = &got[4 + len1..];
            assert_eq!(&rest[4..8], &22u32.to_be_bytes());
            assert_eq!(&rest[rest.len() - 4..], &4u32.to_be_bytes());

            // The client goes (a reset); the last nfsd out zaps the connection.
            abort(&c);
            for _ in 0..3 {
                pump(fx.lo);
            }
            nfsd_leave(&n);
            drop(n);
            assert_eq!(slp.flag(), 0);
            drop((slp, acc));
            assert_eq!(sys_close(fx.user, afd), Ok(()));
            drop(l);
            assert_eq!(sys_close(fx.user, lfd), Ok(()));
        }

        #[test]
        fn nfssvc_refuses_as_the_c_refuses() {
            let fx = server();
            // Not the super user: EPERM, before anything else.
            let user = fx.user;
            let mut c = crate::cred::Ucred::new();
            c.cr_uid.set(1000);
            let root = user.set_cred(crate::cred::CredRef::new(c));
            assert_eq!(
                sys_nfssvc(user, NFSSVC_ADDSOCK, UARGS, &PROCS),
                Err(Errno::EPERM)
            );
            assert_eq!(sys_nfssvc(user, 0x10, UARGS, &PROCS), Err(Errno::EPERM));
            drop(user.set_cred(root.expect("root's credential")));
            // An unknown flag; unreadable arguments; a descriptor that is no socket.
            assert_eq!(sys_nfssvc(user, 0x10, UARGS, &PROCS), Err(Errno::EINVAL));
            assert_eq!(
                sys_nfssvc(user, NFSSVC_ADDSOCK, 8, &PROCS),
                Err(Errno::EFAULT)
            );
            assert_eq!(sys_nfssvc(user, NFSSVC_NFSD, 8, &PROCS), Err(Errno::EFAULT));
            UserPtr::<NfsdArgs>::new(UARGS)
                .copyout(&NfsdArgs {
                    sock: 40,
                    ..NfsdArgs::default()
                })
                .expect("user memory");
            assert_eq!(
                sys_nfssvc(user, NFSSVC_ADDSOCK, UARGS, &PROCS),
                Err(Errno::EBADF)
            );
            // A name longer than a sockaddr may be (sockargs' UCHAR_MAX): EINVAL.
            let fd = bound(&fx, SOCK_STREAM, PORT + 2);
            let a = NfsdArgs {
                sock: fd,
                name: UNAME,
                namelen: 256,
                ..NfsdArgs::default()
            };
            UserPtr::<NfsdArgs>::new(UARGS)
                .copyout(&a)
                .expect("user memory");
            assert_eq!(
                sys_nfssvc(user, NFSSVC_ADDSOCK, UARGS, &PROCS),
                Err(Errno::EINVAL)
            );
            assert_eq!(sys_close(fx.user, fd), Ok(()));
            // A socket that is neither a stream nor a datagram one (raw IP): EINVAL.
            let raw = sys_socket(fx.user, i32::from(AF_INET), hal::sys::socket::SOCK_RAW, 255)
                .expect("a raw socket");
            assert_eq!(addsock(&fx, raw, None), Err(Errno::EINVAL));
            assert_eq!(sys_close(fx.user, raw), Ok(()));
        }

        #[test]
        fn checknam_wants_an_inet_address_on_a_reserved_port() {
            let _fx = server();
            assert!(!nfssvc_checknam(None));
            assert!(nfssvc_checknam(Some(&name(&lo_sin(1023)))));
            assert!(!nfssvc_checknam(Some(&name(&lo_sin(1024)))));
            let mut sin = lo_sin(700);
            sin.sin_family = 0;
            assert!(!nfssvc_checknam(Some(&name(&sin))));
        }
    }
}
/* </TESTS> */
