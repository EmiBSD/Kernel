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
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)uipc_usrreq.c	8.3 (Berkeley) 1/4/94
 */

/*-
 * Copyright (c) 1982, 1986, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)uipc_proto.c	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)unpcb.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! The UNIX communications domain: local sockets (`kern/uipc_usrreq.c`), the domain and its
//! protocol switch (`kern/uipc_proto.c`) and the protocol control block (`<sys/unpcb.h>`).
//!
//! Upstream: sys/kern/uipc_usrreq.c @ 3ce1f3f79392
//! Upstream: sys/kern/uipc_proto.c @ 3ce1f3f79392
//! Upstream: sys/sys/unpcb.h @ 3ce1f3f79392
//! LZ: sys/kern/uipc_usrreq.rs@3c62ede68802
//! LZ: sys/kern/uipc_proto.rs@3c62ede68802
//! LZ: sys/sys/unpcb.rs@3c62ede68802
//!
//! Local sockets: stream, sequenced packet and datagram sockets that live in the kernel
//! only. Each socket has a protocol control block ([`Unpcb`]); a connected stream socket's
//! data goes straight into its peer's receive buffer, and the sender's send buffer mirrors
//! that buffer's counts for back pressure. A socket bound to a path names a `VSOCK` vnode
//! (`uipc_bind`), which `unp_connect` finds through `namei`. `SCM_RIGHTS` control messages
//! carry descriptors: `unp_internalize` turns the sender's descriptor numbers into
//! references to its files held in flight, `unp_externalize` installs them in the receiver's
//! table, and `unp_dispose`/`unp_discard` hand the ones never received to the garbage
//! collector (`unp_gc`, a task on `systqmp`), which also finds and frees cycles of sockets
//! that are referenced only from each other's buffers.
//!
//! A socket may be associated with a vnode in the file system; the control block then holds
//! a reference to the vnode, given back when the socket goes away. A socket may be connected
//! to another socket (`unp_conn`); a datagram socket may be referenced by a number of sockets
//! connected to it (`unp_refs`, linked through `unp_nextref`), and may itself reference a
//! socket (not necessarily one which is referencing it), hence two separate members. Stream
//! sockets keep copies of the receive buffer's `sb_cc` and `sb_mbcnt` in the sender's send
//! buffer, so that changes in the receive buffer modify the back pressure on the sender.
//!
//! Locks used to protect global data and struct members: \[I\] immutable after creation,
//! \[G\] `unp_gc_lock`, \[F\] the in-flight table's mutex, \[a\] atomic, \[s\] socket lock.
//!
//! ## Deviations
//! - `NKCOV` is 0: no kcov descriptors to refuse.
//! - `pool_get(PR_WAITOK)` can fail here (see `hal::pool`): `uipc_attach` answers `ENOBUFS`
//!   as for `PR_NOWAIT`. `unp_externalize`'s `mallocarray(M_WAITOK)` answers `ENOMEM`, and an
//!   `fdexpand` failing there (it cannot in C) ends the call with its error; in both cases the
//!   files go to the collector as the C's other failures send them.
//! - The bound address (`unp_addr`) is the `struct sockaddr_un` itself, kept in the control
//!   block, not an mbuf (an mbuf is not shareable between CPUs by reference): `uipc_bind`'s
//!   `m_getclr(M_WAITOK)` and `unp_connect`'s `m_copym(M_NOWAIT)` of the listener's address
//!   cannot fail, where the C's copy can leave an accepted socket unnamed under mbuf
//!   exhaustion.
//! - `unp_nam2sun` leaves the caller's address mbuf as it is and hands the NUL-terminated
//!   path back: the C's in-place `m_len++` and `sun_len++` reach nothing after the call (the
//!   bound address is a copy, and the callers free the mbuf). Its checks are the C's, the
//!   `m_trailingspace` refusal included.
//! - The in-flight table's array is `malloc(M_TEMP, M_WAITOK)`: a refusal (none in the
//!   kernel; `hal::malloc` before its bridge) is `panic(9)`, as LZ's `unp_discard` panicked
//!   for its deferral: the files in flight cannot be kept without it.
//! - `unpcb`'s item is the control block's Rust layout (376 bytes, where the C's `struct
//!   unpcb` is 144 on LP64), as every kernel pool's since N3: `vmstat -m` shows that size.
//!
//! ## Redesign
//! - LZ's `struct unpcb` was a pool item of `Cell`s and raw `&'static` links (`unp_conn`,
//!   `unp_vnode`, `unp_file`, `unp_addr`), counted by a `refcnt` that `unp_detach` waited on
//!   before `pool_put`. Here [`Unpcb`] is a counted item of the glue's `unpcb` pool
//!   ([`UnpRef`], a `hal::pool::PoolArc`): `unp_ref`/`unp_rele` are a handle's clone and drop,
//!   and `unp_detach`'s `refcnt_finalize` is the count itself (the last handle frees the
//!   control block; the lock dances that held one, `unp_solock_peer` and `unp_detach`'s, keep
//!   it alive while they wait). The socket holds it erased in its read-mostly `so_pcb` slot
//!   (`sotounpcb` reads it as the C reads `so_pcb` without the socket lock in `fptounp`); the
//!   control block holds its socket counted (`unp_socket`), the cycle cut by `unp_detach`,
//!   which clears the slot.
//! - The `[s]` members are one cell keyed to the socket's `so_lock` ([`UnpState`]: the vnode,
//!   the peer, the fake inode, the address, the flags, the peer's credentials), opened with
//!   the witness of [`SoLocked`]; `unp_refs` is a pinned list keyed to the same lock. The
//!   `[G]` counters (`unp_msgcount`, `unp_gcrefs`, `unp_gcflags`) are atomics written only
//!   under `unp_gc_lock` (read by the collector's walks, which borrow the lock's witness);
//!   `unp_file` is a counted reference in a cell keyed to it, held while the socket's file is
//!   in flight (`unp_msgcount > 0`) and given back when the count falls to 0, so the
//!   collector's `f_count == unp_msgcount` test counts it: `f_count == unp_msgcount + 1`.
//! - `struct fdpass` (D-N6.2-2) keeps its 16 bytes in the control mbuf, so every size and
//!   refusal is the C's (`CMSG_SPACE(nfds * sizeof(struct fdpass))`, the `MCLBYTES` refusal,
//!   `sosend`'s reservation), but its `fp` word is the number (plus one; 0 is the C's NULL)
//!   of a slot of the glue's in-flight table, which owns the references (`FileSlot`s) where the
//!   C's message holds them: the kernel never follows a word of mbuf data. The table grows by
//!   doubling (never past what `unp_rights`, capped at `maxfiles / 10`, has reserved) and
//!   links its free slots and its deferred ones (the C's `unp_deferred` sets) through the
//!   slots, so a slot is found, freed or deferred in O(1). `unp_rights` is an atomic
//!   reserved by a compare-and-swap loop with the C's bound, so the C's `unp_rights_mtx`
//!   sections become one atomic operation each.
//! - `unp_scan`'s operation is a closure over the entries' bytes; `unp_discard` defers the
//!   slots and zeroes the entries as the C copies and zeroes them.
//! - Cost against the C: `sotounpcb` is the slot's reader count (two atomic operations) and,
//!   where the request lets the socket go (bind, connect, detach, the datagram send), a
//!   handle's count (two more), where the C reads a pointer. Passing descriptors takes one
//!   section of the in-flight table's mutex per call for up to [`CHUNK`] descriptors (the
//!   C's one `unp_rights_mtx` section, which became an atomic), one more per further
//!   [`CHUNK`]; the collector takes the table's mutex once per buffer it scans, where the C
//!   reads `fp` from the message. The lock dances (`unp_solock_peer`, `unp_detach`'s
//!   referencing sockets, `unp_connect`'s pair) are the C's, step by step; `unp_disconnect`
//!   clears the send buffers' counts with the C's plain stores (no `sb_mtx`).

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::malloc::{M_WAITOK, MArray};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::pin::pin_static;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{PR_NOWAIT, PR_WAITOK, PoolArc, PoolHome, TypedPool};
use hal::rwlock::{Guarded, RawRwlock, WriteWitness};
use hal::sys::errno::Errno;
use hal::sys::pledge::PLEDGE_UNIX;
use hal::sys::socket::{
    AF_UNIX, Cmsghdr, MSG_CMSG_CLOEXEC, MSG_CMSG_CLOFORK, NET_UNIX_DEFERRED, NET_UNIX_INFLIGHT,
    PF_UNIX, SCM_RIGHTS, SO_ACCEPTCONN, SOCK_DGRAM, SOCK_SEQPACKET, SOCK_STREAM, SOL_SOCKET,
    Sockpeercred, UNPCTL_RECVSPACE, UNPCTL_SENDSPACE, cmsg_align, cmsg_len, cmsg_space,
};
use hal::sys::stat::{ACCESSPERMS, Stat};
use hal::sys::sysctl::SysctlBoundedArgs;
use hal::sys::un::SockaddrUn;
use hal::user::{bytes_of, pod_from_bytes};

use crate::assert::{kassert, kpanic};
use crate::clock::Timespec;
use crate::descrip::{
    UF_EXCLOSE, UF_FORKCLOSE, UF_PLEDGED, UF_PLEDGEOPEN, closef, fd_getfile_flags, fdalloc,
    fdexpand, fdplock, fdremove,
};
use crate::file::{
    DTYPE_KQUEUE, DTYPE_SOCKET, DTYPE_VMM, DTYPE_VNODE, FDUP_MAX_COUNT, File, FileGlue, FileRef,
    FileSlot,
};
use crate::fs::lookup::{namei, ndinit};
use crate::fs::namei::{CREATE, FOLLOW, LOCKLEAF, LOCKPARENT, LOOKUP, NOFOLLOW, NiDirp};
use crate::fs::namei::{UNVEIL_CREATE, UNVEIL_WRITE};
use crate::fs::subr::{vattr_null, vput, vrele};
use crate::fs::vnode::{LK_EXCLUSIVE, VBLK, VDIR, VSOCK, VWRITE, Vattr, Vnode};
use crate::fs::vnops::vn_isunder;
use crate::fs::vops::{Cred, vop_abortop, vop_access, vop_create, vop_lock, vop_unlock};
use crate::net::domain::{Domain, net};
use crate::net::mbuf::{
    M_EXT, M_WAIT, MCLBYTES, MLEN, MT_CONTROL, Mbuf, MbufBox, m_purge, m_trailingspace, mclget,
};
use crate::net::proto::{
    PR_ADDR, PR_ATOMIC, PR_CONNREQUIRED, PR_RIGHTS, PR_WANTRCVD, Protocol, Usrreqs,
};
use crate::net::sockbuf::{SB_MAX, SS_CANTSENDMORE, SS_ISCONNECTED};
use crate::net::socket::{
    FDPASS_SIZE, SoLocked, Socket, SocketRef, socantrcvmore, socantsendmore, sofree_kept,
    soisconnected, soisdisconnected, solock, solock_pair, sonewconn, soref, soreserve, sorwakeup,
    sounlock_detached, sowwakeup,
};
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::{SysctlReq, bounded_arr, rdint};
use crate::task::{Task, Taskq, task_add};

/// `M_TEMP` (`<sys/malloc.h>`): `unp_externalize`'s array and the in-flight table.
const M_TEMP: i32 = 127;

/// Both send and receive buffers are allocated `PIPSIZ` bytes of buffering for stream
/// sockets, although the total for sender and receiver is actually only `PIPSIZ`.
const PIPSIZ: i32 = 32768;

/// `CMSG_ALIGN(sizeof(struct cmsghdr))`: where a control message's data starts.
const HDR: usize = cmsg_align(size_of::<Cmsghdr>());

/// The descriptors `unp_internalize`, `unp_externalize` and the collector move through the
/// in-flight table in one section of its mutex.
pub const CHUNK: usize = 8;

/// The in-flight table's "no slot" link.
const NONE: u32 = u32::MAX;

// flag bits in unp_flags

/// `UNP_FEIDS`: `unp_connid` contains information.
pub const UNP_FEIDS: i32 = 0x01;
/// `UNP_FEIDSBIND`: `unp_connid` was set by a bind.
pub const UNP_FEIDSBIND: i32 = 0x02;
/// `UNP_BINDING`: unp is binding now.
pub const UNP_BINDING: i32 = 0x04;
/// `UNP_CONNECTING`: unp is connecting now.
pub const UNP_CONNECTING: i32 = 0x08;

// flag bits in unp_gcflags

/// `UNP_GCDEAD`: unp could be dead.
pub const UNP_GCDEAD: u32 = 0x01;

/// `struct fdpass`: a file in flight in an `SCM_RIGHTS` message, as the message's bytes hold
/// it: the in-flight table's slot of the reference plus one (0: none, the C's NULL `fp`), and
/// the sender's `UF_PLEDGED` flag.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fdpass {
    /// `fp`: the slot number plus one.
    pub slot: u64,
    /// `flags`: the descriptor's `UF_PLEDGED`.
    pub flags: i32,
    /// The C's padding after `flags`.
    pub pad: u32,
}

hal::abi_pod!(Fdpass {
    slot: u64,
    flags: i32,
    pad: u32,
});

/// The file of an open socket's glue.
type Fp<G> = FileRef<<G as ProcGlue>::FileGlue>;

/// A counted handle to a control block (`struct unpcb *` with `unp_ref`).
pub type UnpRef<G> = PoolArc<Unpcb<G>>;

/// The `[s]` members of a control block, under its socket's `so_lock`.
pub struct UnpState<G: ProcGlue> {
    /// \[s\] `unp_vnode`: if associated with file (a reference, `vrele`'d by `unp_detach`).
    vnode: Option<&'static Vnode<G>>,
    /// \[s\] `unp_conn`: control block of connected socket.
    conn: Option<UnpRef<G>>,
    /// \[s\] `unp_ino`: fake inode number.
    ino: u64,
    /// \[s\] `unp_addr`: bound address of socket.
    addr: Option<SockaddrUn>,
    /// \[s\] `unp_flags`: this unpcb contains peer eids.
    flags: i32,
    /// \[s\] `unp_connid`: id of peer process.
    connid: Sockpeercred,
}

/// `struct unpcb`: protocol control block for an active instance of a UNIX internal protocol.
pub struct Unpcb<G: ProcGlue> {
    /// The item's place in `unpcb_pool` (`unp_ref` of a `&Unpcb`).
    home: PoolHome<Self>,
    /// \[I\] `unp_socket`: back to socket (counted; `unp_detach` clears the socket's slot).
    socket: SocketRef<G>,
    /// \[I\] `unp_ctime`: holds creation time.
    ctime: Timespec,
    /// \[s\] The members under the socket's lock.
    s: Guarded<UnpState<G>>,
    /// \[s\] `unp_refs`: referencing socket linked list (the datagram sockets connected to
    /// this one).
    refs: PinList<UnpRefs<G>>,
    /// `unp_nextref`: link in `unp_refs` list (of the socket this one is connected to, under
    /// that socket's lock).
    nextref: PinEntry<Self>,
    /// \[G\] `unp_link`: link in per-AF list of sockets.
    link: PinEntry<Self>,
    /// \[G\] `unp_file`: backpointer for `unp_gc()`, counted while the file is in flight.
    file: Guarded<Option<Fp<G>>>,
    /// \[G\] `unp_msgcount`: references from socket rcv buf.
    msgcount: AtomicI64,
    /// \[G\] `unp_gcrefs`: references from gc.
    gcrefs: AtomicI64,
    /// \[G\] `unp_gcflags`: garbage collector flags.
    gcflags: AtomicU32,
}

hal::pool_home!(impl[G: ProcGlue] Unpcb<G>, home);

hal::pin_adapter!(
    /// `unp_refs`: the control blocks connected to this one, through `unp_nextref`.
    impl[G: ProcGlue] pub UnpRefs<G>: Unpcb<G>, nextref
);

hal::pin_adapter!(
    /// `unp_head`: all UNIX domain sockets, through `unp_link`, for `unp_gc()`.
    impl[G: ProcGlue] pub UnpLink<G>: Unpcb<G>, link
);

impl<G: ProcGlue> Unpcb<G> {
    /// A zeroed control block of `so`, as `pool_get(PR_ZERO)`, `unp_socket = so` and
    /// `getnanotime(&unp->unp_ctime)` leave it.
    fn new(so: &Socket<G>, t: &UnixTables<G>) -> Self {
        Self {
            home: PoolHome::new(),
            socket: soref(so),
            ctime: crate::tc::getnanotime(),
            s: Guarded::new_keyed(
                so.so_lock.lock(),
                UnpState {
                    vnode: None,
                    conn: None,
                    ino: 0,
                    addr: None,
                    flags: 0,
                    connid: Sockpeercred::default(),
                },
            ),
            refs: PinList::new(),
            nextref: PinEntry::new(),
            link: PinEntry::new(),
            file: Guarded::new_keyed(&t.unp_gc_lock, None),
            msgcount: AtomicI64::new(0),
            gcrefs: AtomicI64::new(0),
            gcflags: AtomicU32::new(0),
        }
    }

    /// `unp_socket`.
    pub fn socket(&self) -> &Socket<G> {
        &self.socket
    }

    /// `unp_msgcount`, read as `kern.file` reads it.
    pub fn msgcount(&self) -> i64 {
        self.msgcount.load(Ordering::Relaxed)
    }

    /// The `[s]` members, for the holder of the socket's lock.
    pub fn state<'a>(&'a self, w: &'a SoLocked<'_>) -> &'a UnpState<G> {
        self.s.get(wit_ref(w))
    }
}

impl<G: ProcGlue> UnpState<G> {
    /// `unp_conn`.
    pub fn conn(&self) -> Option<&Unpcb<G>> {
        self.conn.as_deref()
    }

    /// `unp_vnode`.
    pub fn vnode(&self) -> Option<&'static Vnode<G>> {
        self.vnode
    }

    /// `unp_addr`.
    pub fn addr(&self) -> Option<&SockaddrUn> {
        self.addr.as_ref()
    }

    /// `unp_flags`.
    pub fn flags(&self) -> i32 {
        self.flags
    }

    /// `unp_connid`.
    pub fn connid(&self) -> Sockpeercred {
        self.connid
    }
}

/// A slot of the in-flight table: a reference, and the next free or deferred slot's number.
struct InflightSlot<F: FileGlue> {
    /// The reference a message holds (empty while free, or taken out by `unp_externalize`).
    fp: FileSlot<F>,
    /// The next slot of the free or deferred list it is on.
    next: u32,
}

/// \[F\] The references of the files in flight (D-N6.2-2): the slots the messages' `struct
/// fdpass` entries name, the free ones linked from `free`, and the deferred ones (the C's
/// `unp_deferred` sets, sent over sockets that are now closed) linked from `deferred`.
pub struct Inflight<F: FileGlue> {
    /// The slots, made at the first descriptor passed and doubled as needed.
    slots: Option<MArray<InflightSlot<F>>>,
    /// The first free slot.
    free: u32,
    /// The free slots.
    nfree: u32,
    /// The first deferred slot (`SLIST_FIRST(&unp_deferred)`).
    deferred: u32,
}

impl<F: FileGlue> Inflight<F> {
    /// An empty table.
    const fn new() -> Self {
        Self {
            slots: None,
            free: NONE,
            nfree: 0,
            deferred: NONE,
        }
    }

    /// The table's slots.
    fn len(&self) -> usize {
        self.slots.as_ref().map_or(0, |s| s.len())
    }

    /// The slot `i`, which must exist.
    fn slot(&mut self, i: u32) -> &mut InflightSlot<F> {
        match self.slots.as_mut().and_then(|s| s.get_mut(i as usize)) {
            Some(s) => s,
            None => kpanic(format_args!("unp: in-flight slot {i} past the table")),
        }
    }

    /// `fp` into a free slot (one exists: the caller made room); its number plus one.
    fn put(&mut self, fp: FileRef<F>) -> u64 {
        let i = self.free;
        if i == NONE {
            kpanic(format_args!("unp: no free in-flight slot"));
        }
        let s = self.slot(i);
        let next = s.next;
        s.fp.fill(fp);
        s.next = NONE;
        self.free = next;
        self.nfree -= 1;
        u64::from(i) + 1
    }

    /// The reference of `slot` (a message's entry) taken out, the slot freed.
    fn take(&mut self, slot: u64) -> FileRef<F> {
        let i = slot_index(slot);
        let free = self.free;
        let s = self.slot(i);
        let Some(fp) = s.fp.take() else {
            kpanic(format_args!("unp: in-flight slot {i} empty"));
        };
        s.next = free;
        self.free = i;
        self.nfree += 1;
        fp
    }

    /// The file of `slot`, `None` for a cleared entry or an empty slot.
    fn get(&self, slot: u64) -> Option<&File<F>> {
        let i = usize::try_from(slot.checked_sub(1)?).ok()?;
        self.slots.as_ref()?.get(i)?.fp.get().map(|fp| &**fp)
    }

    /// `fp` into a free slot on the deferred list (one exists).
    fn put_deferred(&mut self, fp: FileRef<F>) {
        let slot = self.put(fp);
        self.defer(slot);
    }

    /// The reference of `slot` (a message's entry) on the deferred list.
    fn defer(&mut self, slot: u64) {
        let i = slot_index(slot);
        let head = self.deferred;
        let s = self.slot(i);
        if s.fp.is_none() {
            kpanic(format_args!("unp: deferring the empty in-flight slot {i}"));
        }
        s.next = head;
        self.deferred = i;
    }

    /// The first deferred reference taken out, its slot freed.
    fn pop_deferred(&mut self) -> Option<FileRef<F>> {
        let i = self.deferred;
        if i == NONE {
            return None;
        }
        self.deferred = self.slot(i).next;
        Some(self.take(u64::from(i) + 1))
    }
}

/// The slot number of a message's `fp` word (`fp - 1`); `panic(9)` for the C's NULL, which
/// no table operation is given.
fn slot_index(slot: u64) -> u32 {
    match slot.checked_sub(1).and_then(|i| u32::try_from(i).ok()) {
        Some(i) => i,
        None => kpanic(format_args!("unp: in-flight slot word {slot:#x}")),
    }
}

/// The UNIX domain's globals of one glue (`uipc_usrreq.c`'s), a member of its
/// [`NetTables`](crate::net::domain::NetTables).
pub struct UnixTables<G: ProcGlue> {
    /// `unpcb_pool`.
    unpcbpl: TypedPool<Unpcb<G>>,
    /// `unp_gc_lock`.
    unp_gc_lock: RawRwlock,
    /// \[G\] `unp_head`: list of all UNIX domain sockets, for `unp_gc()`.
    unp_head: PinList<UnpLink<G>>,
    /// \[F\] the references in flight and the deferred ones (`unp_deferred`, `unp_df_lock`).
    inflight: Mutexed<Inflight<G::FileGlue>>,
    /// \[a\] `unp_rights`: file descriptors in flight.
    unp_rights: AtomicI32,
    /// \[a\] `unp_ino`: prototype for fake inode numbers (the C's `unp_ino_mtx`).
    unp_ino: AtomicU64,
    /// \[G\] `unp_defer`: number of deferred fp to close by the GC task.
    unp_defer: AtomicI32,
    /// \[G\] `unp_gcing`: GC task currently running.
    unp_gcing: AtomicBool,
    /// `unp_gc_task`.
    unp_gc_task: Task,
}

impl<G: ProcGlue> UnixTables<G> {
    /// The empty tables, before `unp_init`.
    pub const fn new() -> Self {
        Self {
            unpcbpl: TypedPool::new(),
            unp_gc_lock: RawRwlock::new("unpgclk"),
            unp_head: PinList::new(),
            inflight: Mutexed::new(IPL_SOFTNET, Inflight::new()),
            unp_rights: AtomicI32::new(0),
            unp_ino: AtomicU64::new(0),
            unp_defer: AtomicI32::new(0),
            unp_gcing: AtomicBool::new(false),
            unp_gc_task: Task::new(unp_gc_task::<G>, ptr::null_mut()),
        }
    }

    /// `unp_rights`: the file descriptors in flight (`net.unix.inflight`).
    pub fn rights(&self) -> i32 {
        self.unp_rights.load(Ordering::Relaxed)
    }

    /// `unpcb_pool`.
    pub fn pool(&self) -> &TypedPool<Unpcb<G>> {
        &self.unpcbpl
    }
}

impl<G: ProcGlue> Default for UnixTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `unixdomain`: the UNIX domain (a type without data, one per glue).
pub struct Unix<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Unix<G> {
    /// `unixdomain`.
    pub const NEW: Self = Self(PhantomData);
    /// The domain, as its protocols name it.
    const DOMAIN: &'static dyn Domain<G> = &Self(PhantomData);
    /// `unixsw[]`: definitions of protocols supported in the UNIX domain.
    const PROTOSW: &'static [&'static dyn Protocol<G>] = &[
        &UnixProto::<G> {
            ty: SOCK_STREAM as i16,
            flags: PR_CONNREQUIRED | PR_WANTRCVD | PR_RIGHTS,
            dgram: false,
            _g: PhantomData,
        },
        &UnixProto::<G> {
            ty: SOCK_SEQPACKET as i16,
            flags: PR_ATOMIC | PR_CONNREQUIRED | PR_WANTRCVD | PR_RIGHTS,
            dgram: false,
            _g: PhantomData,
        },
        &UnixProto::<G> {
            ty: SOCK_DGRAM as i16,
            flags: PR_ATOMIC | PR_ADDR | PR_RIGHTS,
            dgram: true,
            _g: PhantomData,
        },
    ];
}

/// An entry of `unixsw[]`.
pub struct UnixProto<G> {
    /// `pr_type`.
    ty: i16,
    /// `pr_flags`.
    flags: i16,
    /// `pr_usrreqs` is `uipc_dgram_usrreqs` (else `uipc_usrreqs`).
    dgram: bool,
    _g: PhantomData<fn() -> G>,
}

/// `uipc_usrreqs`: the stream and sequenced packet sockets' requests.
pub struct UipcUsrreqs<G>(PhantomData<fn() -> G>);

/// `uipc_dgram_usrreqs`: the datagram sockets' requests.
pub struct UipcDgramUsrreqs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> UipcUsrreqs<G> {
    /// `&uipc_usrreqs`.
    const USR: &'static dyn Usrreqs<G> = &Self(PhantomData);
}

impl<G: ProcGlue> UipcDgramUsrreqs<G> {
    /// `&uipc_dgram_usrreqs`.
    const USR: &'static dyn Usrreqs<G> = &Self(PhantomData);
}

/// `unp[st,sq,dg]_[send,recv]space`: the buffer sizes `uipc_attach` reserves (`u_int` in C;
/// the bounds keep them positive). Datagram sockets really use the sendspace as the maximum
/// datagram size, and don't really want to reserve the sendspace. Their recvspace should be
/// large enough for at least one max-size datagram plus address.
pub static UNPST_SENDSPACE: AtomicI32 = AtomicI32::new(PIPSIZ);
/// \[a\] `unpst_recvspace`.
pub static UNPST_RECVSPACE: AtomicI32 = AtomicI32::new(PIPSIZ);
/// \[a\] `unpsq_sendspace`.
pub static UNPSQ_SENDSPACE: AtomicI32 = AtomicI32::new(PIPSIZ);
/// \[a\] `unpsq_recvspace`.
pub static UNPSQ_RECVSPACE: AtomicI32 = AtomicI32::new(PIPSIZ);
/// \[a\] `unpdg_sendspace`: really max datagram size.
pub static UNPDG_SENDSPACE: AtomicI32 = AtomicI32::new(8192);
/// \[a\] `unpdg_recvspace`.
pub static UNPDG_RECVSPACE: AtomicI32 = AtomicI32::new(PIPSIZ);

/// `unpstctl_vars`.
static UNPSTCTL_VARS: [SysctlBoundedArgs; 2] = [
    SysctlBoundedArgs::new(UNPCTL_RECVSPACE, &UNPST_RECVSPACE, 0, SB_MAX as i32),
    SysctlBoundedArgs::new(UNPCTL_SENDSPACE, &UNPST_SENDSPACE, 0, SB_MAX as i32),
];
/// `unpsqctl_vars`.
static UNPSQCTL_VARS: [SysctlBoundedArgs; 2] = [
    SysctlBoundedArgs::new(UNPCTL_RECVSPACE, &UNPSQ_RECVSPACE, 0, SB_MAX as i32),
    SysctlBoundedArgs::new(UNPCTL_SENDSPACE, &UNPSQ_SENDSPACE, 0, SB_MAX as i32),
];
/// `unpdgctl_vars`.
static UNPDGCTL_VARS: [SysctlBoundedArgs; 2] = [
    SysctlBoundedArgs::new(UNPCTL_RECVSPACE, &UNPDG_RECVSPACE, 0, SB_MAX as i32),
    SysctlBoundedArgs::new(UNPCTL_SENDSPACE, &UNPDG_SENDSPACE, 0, SB_MAX as i32),
];

/// `sun_noname`: the address of an unbound socket, `{ sizeof(sun_noname), AF_UNIX }`.
const SUN_NONAME: [u8; 16] = {
    let mut a = [0u8; 16];
    a[0] = 16;
    a[1] = AF_UNIX;
    a
};

impl<G: ProcGlue> Domain<G> for Unix<G> {
    fn family(&self) -> i32 {
        i32::from(AF_UNIX)
    }
    fn name(&self) -> &'static str {
        "unix"
    }
    fn init(&self) {
        unp_init::<G>();
    }
    fn externalize(&self, m: &mut Mbuf, controllen: u32, flags: i32) -> Result<(), Errno> {
        G::with_curproc(|p| match p {
            Some(p) => unp_externalize(m, controllen, flags, p),
            None => kpanic(format_args!("unp_externalize: no curproc")),
        })
    }
    fn dispose(&self, m: &mut Mbuf) {
        unp_dispose::<G>(m);
    }
    fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
        Self::PROTOSW
    }
    fn peercred(&self, so: &Socket<G>, w: &SoLocked<'_>) -> Option<Sockpeercred> {
        let unp = unp_of(so);
        let s = unp.state(w);
        (s.flags & UNP_FEIDS != 0).then_some(s.connid)
    }
}

impl<G: ProcGlue> Protocol<G> for UnixProto<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        Unix::<G>::DOMAIN
    }
    fn ty(&self) -> i16 {
        self.ty
    }
    fn protocol(&self) -> i16 {
        i16::from(PF_UNIX)
    }
    fn flags(&self) -> i16 {
        self.flags
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        Some(if self.dgram {
            UipcDgramUsrreqs::<G>::USR
        } else {
            UipcUsrreqs::<G>::USR
        })
    }
}

/// The glue's UNIX domain tables.
#[inline]
fn tables<G: ProcGlue>() -> &'static UnixTables<G> {
    &net::<G>().unix
}

/// The witness of `so_lock` a UNIX socket's lock holds (`panic(9)` without one: a UNIX
/// socket is never locked by the net lock alone).
fn wit<'w, 'a>(w: &'w mut SoLocked<'a>) -> &'w mut WriteWitness<'a> {
    match w.so_lock() {
        Some(x) => x,
        None => kpanic(format_args!("unp: a socket locked without its so_lock")),
    }
}

/// [`wit`], shared.
fn wit_ref<'w, 'a>(w: &'w SoLocked<'a>) -> &'w WriteWitness<'a> {
    match w.so_lock_ref() {
        Some(x) => x,
        None => kpanic(format_args!("unp: a socket locked without its so_lock")),
    }
}

/// `unp`'s `[s]` members, to change them.
fn st<'a, G: ProcGlue>(unp: &'a Unpcb<G>, w: &'a mut SoLocked<'_>) -> &'a mut UnpState<G> {
    unp.s.get_mut(wit(w))
}

/// `unp_ref(unp)`: a counted handle to a control block a holder lends.
fn unp_ref<G: ProcGlue>(unp: &Unpcb<G>) -> UnpRef<G> {
    match PoolArc::from_ref(unp) {
        Some(r) => r,
        None => kpanic(format_args!(
            "unp_ref: control block {unp:p} not in unpcb_pool"
        )),
    }
}

/// `sotounpcb(so)`: the control block of a UNIX socket, counted; `None` before `uipc_attach`
/// and after `unp_detach` (the C's NULL `so_pcb`).
pub fn sotounpcb<G: ProcGlue>(so: &Socket<G>) -> Option<UnpRef<G>> {
    let pcb = so.pcb_read();
    pcb.as_ref()?.downcast_ref::<Unpcb<G>>().map(unp_ref)
}

/// `sotounpcb(so)` of a request on an attached socket, whose control block the C uses
/// unchecked.
fn unp_of<G: ProcGlue>(so: &Socket<G>) -> UnpRef<G> {
    match sotounpcb(so) {
        Some(unp) => unp,
        None => kpanic(format_args!("unp: socket {so:p} without its control block")),
    }
}

/// `unp_init`: the control blocks' pool (`unpcb`, `IPL_SOFTNET`). A second call (a test
/// glue's) leaves it as it is.
pub fn unp_init<G: ProcGlue>() {
    let _ = tables::<G>().unpcbpl.init_recorded(IPL_SOFTNET, 0, "unpcb");
}

/// A peer [`unp_solock_peer`] found and locked.
struct Peer<'p, 'l, G: ProcGlue> {
    /// `unp->unp_conn`.
    unp: &'p Unpcb<G>,
    /// Its socket.
    so: &'p Socket<G>,
    /// Its lock's witness; `None` when the peer is the socket itself (a datagram socket
    /// connected to itself), whose witness is the caller's.
    w: Option<&'p mut SoLocked<'l>>,
}

/// `unp_solock_peer(so)`: `f` with the peer of `so` (its control block `unp`) locked as
/// well, in address order: when the peer is lower, `so` is unlocked, the peer locked, `so`
/// locked again and the connection looked at anew (a datagram socket could be reconnected
/// meanwhile); `f` gets `None` without a peer.
fn unp_solock_peer<'a, G: ProcGlue, R>(
    so: &Socket<G>,
    unp: &Unpcb<G>,
    w: &mut SoLocked<'a>,
    f: impl FnOnce(Option<Peer<'_, '_, G>>, &mut SoLocked<'a>) -> R,
) -> R {
    loop {
        let Some(unp2) = unp.state(w).conn.clone() else {
            return f(None, w);
        };
        let so2: &Socket<G> = &unp2.socket;
        if ptr::eq(so, so2) {
            return f(
                Some(Peer {
                    unp: &unp2,
                    so: so2,
                    w: None,
                }),
                w,
            );
        }
        let mut w2 = if ptr::from_ref(so) < ptr::from_ref(so2) {
            solock(so2)
        } else {
            let w2 = w.unlocked(|| solock(so2));
            // Datagram socket could be reconnected due to re-lock.
            if !unp
                .state(w)
                .conn
                .as_ref()
                .is_some_and(|c| PoolArc::ptr_eq(c, &unp2))
            {
                drop(w2);
                continue;
            }
            w2
        };
        let r = f(
            Some(Peer {
                unp: &unp2,
                so: so2,
                w: Some(&mut w2),
            }),
            w,
        );
        drop(w2);
        return r;
    }
}

/// `uipc_setaddr(unp, nam)`: `addr` (a control block's `unp_addr`), or `sun_noname`, into the
/// address mbuf `nam`.
fn uipc_setaddr(addr: Option<&SockaddrUn>, nam: &mut Mbuf) {
    let bytes: &[u8] = match addr {
        Some(a) => bytes_of(core::slice::from_ref(a)),
        None => &SUN_NONAME,
    };
    nam.m_len().set(bytes.len() as u32);
    match nam.data_mut() {
        Some(d) => d.copy_from_slice(bytes),
        None => kpanic(format_args!("uipc_setaddr: a read-only address mbuf")),
    }
}

/// `uipc_attach(so, proto, wait)`.
fn uipc_attach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>, wait: i32) -> Result<(), Errno> {
    if so.has_pcb() {
        return Err(Errno::EISCONN);
    }
    if so.so_snd.hiwat() == 0 || so.so_rcv.hiwat() == 0 {
        let (snd, rcv) = match so.so_type() {
            SOCK_STREAM => (&UNPST_SENDSPACE, &UNPST_RECVSPACE),
            SOCK_SEQPACKET => (&UNPSQ_SENDSPACE, &UNPSQ_RECVSPACE),
            SOCK_DGRAM => (&UNPDG_SENDSPACE, &UNPDG_RECVSPACE),
            _ => kpanic(format_args!("unp_attach")),
        };
        soreserve(
            so,
            u64::from(snd.load(Ordering::Relaxed) as u32),
            u64::from(rcv.load(Ordering::Relaxed) as u32),
        )?;
    }
    let t = tables::<G>();
    let flags = if wait == M_WAIT { PR_WAITOK } else { PR_NOWAIT };
    let Some(unp) = t.unpcbpl.get_homed(flags, || Unpcb::new(so, t)) else {
        return Err(Errno::ENOBUFS);
    };
    drop(so.set_pcb(w, Some(PoolArc::erase(unp.clone()))));

    let mut g = t.unp_gc_lock.write();
    t.unp_head.insert_head(&mut g, unp.as_pin());
    drop(g);

    Ok(())
}

/// `uipc_detach(so)`.
fn uipc_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let Some(unp) = sotounpcb(so) else {
        return Err(Errno::EINVAL);
    };
    unp_detach(so, w, unp);
    Ok(())
}

/// `uipc_bind(so, nam, p)`: binds `so` to the path of `nam`, a new `VSOCK` vnode.
fn uipc_bind<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let unp = unp_of(so);
    {
        let s = unp.state(w);
        if s.flags & (UNP_BINDING | UNP_CONNECTING) != 0 {
            return Err(Errno::EINVAL);
        }
        if s.vnode.is_some() {
            return Err(Errno::EINVAL);
        }
    }
    let (path, pathlen) = unp_nam2sun(nam)?;

    st(&unp, w).flags |= UNP_BINDING;

    // The bound address: the family and the path, the rest zeroed, `sun_len` the whole
    // structure's (the C's `nam2`, whose `m_len` is `sizeof(struct sockaddr_un)`).
    let mut addr = SockaddrUn {
        sun_len: size_of::<SockaddrUn>() as u8,
        sun_family: AF_UNIX,
        sun_path: [0; hal::sys::un::SUN_PATH_LEN],
    };
    addr.sun_path[..pathlen].copy_from_slice(&path[..pathlen]);

    // Enforce `i_lock' -> `solock' because fifo subsystem requires it. The socket can't be
    // closed concurrently because the file descriptor reference is still held. The socket
    // is locked again before the kernel lock is let go, as the C's.
    let r = w.unlocked(|| {
        let mut nd = ndinit(
            CREATE,
            NOFOLLOW | LOCKPARENT,
            NiDirp::Sys(&path[..pathlen]),
            p,
        );
        nd.ni_pledge = PLEDGE_UNIX;
        nd.ni_unveil = UNVEIL_CREATE;

        hal::klock::kernel_lock();
        // SHOULD BE ABLE TO ADOPT EXISTING AND wakeup() ALA FIFO's
        uipc_bind_create(&mut nd, p)
    });
    let r = r.map(|vp| {
        let s = st(&unp, w);
        s.addr = Some(addr);
        s.vnode = Some(vp);
        s.connid = peercred_of(p);
        s.flags |= UNP_FEIDSBIND;
        drop(vp.set_socket(Some(soref(so))));
        let _ = vop_unlock(vp);
    });
    hal::klock::kernel_unlock();
    st(&unp, w).flags &= !UNP_BINDING;

    r
}

/// `uipc_bind`'s `namei` and `VOP_CREATE`, the kernel lock held: the new socket vnode,
/// locked; `EADDRINUSE` for a name that exists.
fn uipc_bind_create<G: ProcGlue>(
    nd: &mut crate::fs::vops::Nd<'_, G>,
    p: &Thread<G>,
) -> Result<&'static Vnode<G>, Errno> {
    namei(nd)?;
    let Some(dvp) = nd.ni_dvp else {
        kpanic(format_args!("uipc_bind: no parent from namei"));
    };
    if let Some(vp) = nd.ni_vp {
        let _ = vop_abortop(dvp, &mut nd.ni_cnd);
        if ptr::eq(dvp, vp) {
            vrele(dvp);
        } else {
            vput(dvp);
        }
        vrele(vp);
        return Err(Errno::EADDRINUSE);
    }
    let mut vattr = Vattr::new();
    vattr_null(&mut vattr);
    vattr.va_type = VSOCK;
    vattr.va_mode = ACCESSPERMS & !p.fd().cmask();
    let error = vop_create(dvp, &mut nd.ni_vp, &mut nd.ni_cnd, &mut vattr);
    vput(dvp);
    error?;
    match nd.ni_vp {
        Some(vp) => Ok(vp),
        None => kpanic(format_args!("uipc_bind: VOP_CREATE gave no vnode")),
    }
}

/// `unp_connid`'s value for the thread `p`: its effective ids and its process's id.
fn peercred_of<G: ProcGlue>(p: &Thread<G>) -> Sockpeercred {
    let cred = p.ucred();
    Sockpeercred {
        uid: cred.cr_uid.get(),
        gid: cred.cr_gid.get(),
        pid: p.process().ps_pid,
    }
}

/// `uipc_listen(so)`: only a bound socket, not binding or connecting, listens.
fn uipc_listen<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let unp = unp_of(so);
    let s = unp.state(w);
    if s.flags & (UNP_BINDING | UNP_CONNECTING) != 0 {
        return Err(Errno::EINVAL);
    }
    if s.vnode.is_none() {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// `uipc_connect(so, nam)`.
fn uipc_connect<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
) -> Result<(), Errno> {
    G::with_curproc(|p| match p {
        Some(p) => unp_connect(so, w, nam, p),
        None => kpanic(format_args!("uipc_connect: no curproc")),
    })
}

/// `uipc_accept(so, nam)` and `uipc_peeraddr(so, nam)`: pass back name of connected socket,
/// if it was bound and we are still connected (our peer may have closed already!).
fn uipc_peeraddr<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>, nam: &mut Mbuf) {
    let unp = unp_of(so);
    unp_solock_peer(so, &unp, w, |peer, w| match peer {
        Some(Peer {
            unp: unp2,
            w: Some(w2),
            ..
        }) => uipc_setaddr(unp2.state(w2).addr.as_ref(), nam),
        Some(Peer {
            unp: unp2, w: None, ..
        }) => uipc_setaddr(unp2.state(w).addr.as_ref(), nam),
        None => uipc_setaddr(None, nam),
    });
}

/// `uipc_shutdown(so)`.
fn uipc_shutdown<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) {
    socantsendmore(so);

    let unp = unp_of(so);
    if let Some(unp2) = unp.state(w).conn.as_deref() {
        socantrcvmore(&unp2.socket);
    }
}

/// `uipc_rcvd(so)`: adjust backpressure on sender and wakeup any waiting to write.
fn uipc_rcvd<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) {
    let pcb = so.pcb_read();
    let Some(unp) = pcb.as_ref().and_then(|a| a.downcast_ref::<Unpcb<G>>()) else {
        kpanic(format_args!(
            "uipc_rcvd: socket {so:p} without its control block"
        ));
    };
    let Some(unp2) = unp.state(w).conn.as_deref() else {
        return;
    };
    let so2 = &*unp2.socket;

    let rg = so.so_rcv.lock();
    let mut sg = so2.so_snd.lock();
    sg.set_cc_mbcnt(so.so_rcv.cc(), so.so_rcv.mbcnt());
    drop(sg);
    drop(rg);
    sowwakeup(so2);
}

/// `uipc_send(so, m, nam, control)`: send to paired receive port, and then raise send buffer
/// counts to maintain backpressure; wake up readers.
fn uipc_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    let mut m = m;
    let mut control = control;

    if let Some(c) = control.as_deref_mut() {
        let r = w.unlocked(|| unp_internalize_cur::<G>(c));
        if let Err(e) = r {
            // out:
            drop(control);
            drop(m);
            return Err(e);
        }
    }

    let error = 'dispose: {
        // We hold both solock() and `sb_mtx' mutex while modifying SS_CANTSENDMORE flag.
        // solock() is enough to check it.
        if so.so_snd.has_state(SS_CANTSENDMORE) {
            break 'dispose Err(Errno::EPIPE);
        }
        let pcb = so.pcb_read();
        let Some(unp) = pcb.as_ref().and_then(|a| a.downcast_ref::<Unpcb<G>>()) else {
            kpanic(format_args!(
                "uipc_send: socket {so:p} without its control block"
            ));
        };
        let Some(unp2) = unp.state(w).conn.as_deref() else {
            break 'dispose Err(Errno::ENOTCONN);
        };
        let so2 = &*unp2.socket;

        // sbappend*() should be serialized together with so_snd modification.
        let mut rg = so2.so_rcv.lock();
        let mut sg = so.so_snd.lock();
        if let Some(c) = control.take() {
            if let Err((m0, c)) = rg.sbappendcontrol(m.take(), c) {
                m = m0;
                control = Some(c);
                drop(sg);
                drop(rg);
                break 'dispose Err(Errno::ENOBUFS);
            }
        } else if so.so_type() == SOCK_SEQPACKET {
            rg.sbappendrecord(m.take());
        } else {
            rg.sbappend(m.take());
        }
        sg.set_cc_mbcnt(so2.so_rcv.cc(), so2.so_rcv.mbcnt());
        let dowakeup = so2.so_rcv.cc() > 0;
        drop(sg);
        drop(rg);

        if dowakeup {
            sorwakeup(so2);
        }
        Ok(())
    };

    // dispose: we need to undo unp_internalize in case of errors
    if error.is_err()
        && let Some(c) = control.as_deref_mut()
    {
        unp_dispose::<G>(c);
    }
    // out:
    drop(control);
    drop(m);
    error
}

/// `uipc_dgram_send(so, m, nam, control)`.
fn uipc_dgram_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    nam: Option<&Mbuf>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    let mut m = m;
    let mut control = control;

    if let Some(c) = control.as_deref_mut() {
        let r = w.unlocked(|| unp_internalize_cur::<G>(c));
        if let Err(e) = r {
            drop(control);
            drop(m);
            return Err(e);
        }
    }

    let unp = unp_of(so);
    let error = 'dispose: {
        if let Some(nam) = nam {
            if unp.state(w).conn.is_some() {
                break 'dispose Err(Errno::EISCONN);
            }
            let r = G::with_curproc(|p| match p {
                Some(p) => unp_connect(so, w, nam, p),
                None => kpanic(format_args!("uipc_dgram_send: no curproc")),
            });
            if let Err(e) = r {
                break 'dispose Err(e);
            }
        }

        let dowakeup;
        {
            let s = unp.state(w);
            let Some(unp2) = s.conn.as_deref() else {
                break 'dispose Err(if nam.is_some() {
                    Errno::ECONNREFUSED
                } else {
                    Errno::ENOTCONN
                });
            };
            let so2 = &*unp2.socket;

            let from: &[u8] = match &s.addr {
                Some(a) => bytes_of(core::slice::from_ref(a)),
                None => &SUN_NONAME,
            };

            let mut rg = so2.so_rcv.lock();
            match rg.sbappendaddr(from, m.take(), control.take()) {
                Ok(()) => dowakeup = Ok(()),
                Err((m0, c)) => {
                    m = m0;
                    control = c;
                    dowakeup = Err(Errno::ENOBUFS);
                }
            }
            drop(rg);
            if dowakeup.is_ok() {
                sorwakeup(so2);
            }
        }
        if nam.is_some() {
            unp_disconnect(so, w, &unp);
        }
        dowakeup
    };

    // dispose: we need to undo unp_internalize in case of errors
    if error.is_err()
        && let Some(c) = control.as_deref_mut()
    {
        unp_dispose::<G>(c);
    }
    drop(control);
    drop(m);
    error
}

/// `uipc_abort(so)`.
fn uipc_abort<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) {
    let unp = unp_of(so);
    unp_detach(so, w, unp);
    sofree_kept(so, w);
}

/// `uipc_sense(so, sb)`: the socket's `fstat(2)`: its send buffer's size, no device, a fake
/// inode number made at the first call, the creation time.
fn uipc_sense<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>, sb: &mut Stat) {
    let unp = unp_of(so);

    sb.st_blksize = so.so_snd.hiwat() as _;
    sb.st_dev = -1; // NODEV
    let s = st(&unp, w);
    if s.ino == 0 {
        s.ino = unp_ino_next(tables::<G>());
    }
    let t: hal::sys::stat::Timespec = unp.ctime.into();
    sb.st_atim = t;
    sb.st_mtim = t;
    sb.st_ctim = t;
    sb.st_ino = s.ino;
}

/// `if (++unp_ino == 0) unp_ino = 1; unp->unp_ino = unp_ino` under `unp_ino_mtx`: the next
/// fake inode number, never 0.
fn unp_ino_next<G: ProcGlue>(t: &UnixTables<G>) -> u64 {
    let mut cur = t.unp_ino.load(Ordering::Relaxed);
    loop {
        let next = match cur.wrapping_add(1) {
            0 => 1,
            n => n,
        };
        match t
            .unp_ino
            .compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return next,
            Err(now) => cur = now,
        }
    }
}

/// `uipc_connect2(so, so2)`: [`unp_connect2`], then both sockets' peer credentials are the
/// calling thread's (`socketpair(2)`).
fn uipc_connect2<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    so2: &Socket<G>,
    w2: Option<&mut SoLocked<'_>>,
) -> Result<(), Errno> {
    let mut w2 = w2;
    unp_connect2(so, w, so2, w2.as_deref_mut())?;

    let connid = G::with_curproc(|p| match p {
        Some(p) => peercred_of(p),
        None => kpanic(format_args!("uipc_connect2: no curproc")),
    });
    let unp = unp_of(so);
    let s = st(&unp, w);
    s.connid = connid;
    s.flags |= UNP_FEIDS;
    let unp2 = unp_of(so2);
    let s2 = match w2 {
        Some(w2) => st(&unp2, w2),
        None => st(&unp2, w),
    };
    s2.connid = connid;
    s2.flags |= UNP_FEIDS;

    Ok(())
}

/// What `kern.file` reports of a UNIX socket's control block (`fill_file`'s `AF_UNIX` case).
#[derive(Clone, Copy, Debug, Default)]
pub struct UnpKinfo {
    /// The control block's address (`so_pcb`).
    pub pcb: usize,
    /// `unp_msgcount`.
    pub msgcount: i64,
    /// `unp_conn`'s address (0 for none).
    pub conn: usize,
    /// `SLIST_FIRST(&unp_refs)`'s address.
    pub refs: usize,
    /// `SLIST_NEXT(unp, unp_nextref)`'s address (on the list of the socket it is connected
    /// to).
    pub nextref: usize,
    /// `unp_vnode`'s address.
    pub vnode: usize,
    /// `unp_addr`'s address (0 for an unbound socket).
    pub addr: usize,
    /// The bound address.
    pub sun: Option<SockaddrUn>,
}

/// `fill_file`'s `AF_UNIX` case: the control block of `so` (locked: `w`) as `kern.file`
/// reports it, `None` without one. The peer is locked too for `unp_nextref`, the link of
/// `so` on its peer's list, which the C reads without the peer's lock.
pub fn unp_kinfo<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Option<UnpKinfo> {
    let unp = sotounpcb(so)?;
    let addr_of = |x: &Unpcb<G>| core::ptr::from_ref(x).addr();
    let mut k = {
        let s = unp.state(w);
        UnpKinfo {
            pcb: addr_of(&unp),
            msgcount: unp.msgcount(),
            conn: s.conn.as_deref().map_or(0, addr_of),
            refs: 0,
            nextref: 0,
            vnode: s.vnode.map_or(0, |v| core::ptr::from_ref(v).addr()),
            addr: s.addr.as_ref().map_or(0, |a| core::ptr::from_ref(a).addr()),
            sun: s.addr,
        }
    };
    k.refs = unp.refs.first(wit_ref(w)).map_or(0, addr_of);
    if so.so_type() == SOCK_DGRAM {
        k.nextref = unp_solock_peer(so, &unp, w, |peer, w| match peer {
            Some(Peer {
                unp: unp2,
                w: Some(w2),
                ..
            }) => unp2.refs.next(wit_ref(w2), &unp).map_or(0, addr_of),
            Some(Peer {
                unp: unp2, w: None, ..
            }) => unp2.refs.next(wit_ref(w), &unp).map_or(0, addr_of),
            None => 0,
        });
    }
    Some(k)
}

/// `uipc_sysctl(name, ..)`: `net.unix`: the buffer sizes per type, the descriptors in flight
/// and the deferred ones. All sysctl names at this level are terminal.
pub fn uipc_sysctl<G: ProcGlue>(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let t = tables::<G>();
    let Some(&name0) = name.first() else {
        return Err(Errno::EISDIR);
    };
    match name0 {
        SOCK_STREAM | SOCK_SEQPACKET | SOCK_DGRAM => {
            if name.len() != 2 {
                return Err(Errno::ENOTDIR);
            }
            let vars: &[SysctlBoundedArgs] = match name0 {
                SOCK_STREAM => &UNPSTCTL_VARS,
                SOCK_SEQPACKET => &UNPSQCTL_VARS,
                _ => &UNPDGCTL_VARS,
            };
            bounded_arr(vars, &name[1..], req)
        }
        NET_UNIX_INFLIGHT | NET_UNIX_DEFERRED => {
            if name.len() != 1 {
                return Err(Errno::ENOTDIR);
            }
            let v = if name0 == NET_UNIX_INFLIGHT {
                t.unp_rights.load(Ordering::Relaxed)
            } else {
                t.unp_defer.load(Ordering::Relaxed)
            };
            rdint(req, v)
        }
        _ => Err(Errno::ENOPROTOOPT),
    }
}

/// `unp_detach(unp)`: takes the control block apart: the bound vnode, the connection, the
/// sockets connected to it; the socket's slot cleared, so the last handle frees it.
fn unp_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>, unp: UnpRef<G>) {
    let t = tables::<G>();
    let vp = st(&unp, w).vnode.take();

    let mut g = t.unp_gc_lock.write();
    t.unp_head.remove(&mut g, &unp);
    drop(g);

    if let Some(vp) = vp {
        // Enforce `i_lock' -> solock() lock order.
        let vso = w.unlocked(|| {
            let _ = vop_lock(vp, LK_EXCLUSIVE);
            let vso = vp.set_socket(None);

            hal::klock::kernel_lock();
            vput(vp);
            hal::klock::kernel_unlock();
            vso
        });
        // The vnode's reference to the socket: never its last (the caller holds one).
        drop(vso);
    }

    if unp.state(w).conn.is_some() {
        // Datagram socket could be connected to itself. Such socket will be disconnected
        // here.
        unp_disconnect(so, w, &unp);
    }

    loop {
        let Some(unp2) = unp.refs.first(wit_ref(w)).map(unp_ref) else {
            break;
        };
        let so2: &Socket<G> = &unp2.socket;

        let mut w2 = if ptr::from_ref(so) < ptr::from_ref(so2) {
            solock(so2)
        } else {
            let w2 = w.unlocked(|| solock(so2));
            if !unp2
                .state(&w2)
                .conn
                .as_ref()
                .is_some_and(|c| PoolArc::ptr_eq(c, &unp))
            {
                // `unp2' was disconnected due to re-lock.
                drop(w2);
                continue;
            }
            w2
        };

        let conn = st(&unp2, &mut w2).conn.take();
        unp.refs.remove(wit(w), &unp2);
        so2.set_error(Some(Errno::ECONNRESET));
        so2.st_clear(SS_ISCONNECTED);

        drop(w2);
        drop(conn);
    }

    // refcnt_finalize(&unp->unp_refcnt, "unpfinal"): the last handle frees the block.
    soisdisconnected(so);
    let pcb = so.set_pcb(w, None);
    st(&unp, w).addr = None;
    drop(pcb);
    drop(unp);
    if t.unp_rights.load(Ordering::Relaxed) != 0 {
        task_add(unp_gc_tq(), pin_static(&t.unp_gc_task));
    }
}

/// `unp_connect(so, nam, p)`: connects `so` to the socket bound to the path of `nam`; a
/// connection-oriented one through a new socket `sonewconn` queues on the listener.
fn unp_connect<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let unp = unp_of(so);
    if unp.state(w).flags & (UNP_BINDING | UNP_CONNECTING) != 0 {
        return Err(Errno::EISCONN);
    }
    let (path, len) = unp_nam2sun(nam)?;

    st(&unp, w).flags |= UNP_CONNECTING;

    // Enforce `i_lock' -> `solock' because fifo subsystem requires it. The socket can't be
    // closed concurrently because the file descriptor reference is still held.
    let error = w.unlocked(|| {
        let mut nd = ndinit(LOOKUP, FOLLOW | LOCKLEAF, NiDirp::Sys(&path[..len]), p);
        nd.ni_pledge = PLEDGE_UNIX;
        nd.ni_unveil = UNVEIL_WRITE;

        hal::klock::kernel_lock();
        let error = match namei(&mut nd) {
            Ok(()) => match nd.ni_vp {
                Some(vp) => {
                    let error = unp_connect_vp(so, vp, p);
                    vput(vp);
                    error
                }
                None => kpanic(format_args!("unp_connect: no vnode from namei")),
            },
            Err(e) => Err(e),
        };
        hal::klock::kernel_unlock();
        error
    });

    st(&unp, w).flags &= !UNP_CONNECTING;

    // The peer socket could be closed by concurrent thread when `so' and `vp' are unlocked.
    if error.is_ok() && unp.state(w).conn.is_none() {
        return Err(Errno::ECONNREFUSED);
    }
    error
}

/// `unp_connect`'s work on the vnode `namei` found (locked), `so` unlocked and the kernel
/// lock held.
fn unp_connect_vp<G: ProcGlue>(
    so: &Socket<G>,
    vp: &'static Vnode<G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if vp.vtype() != VSOCK {
        return Err(Errno::ENOTSOCK);
    }
    vop_access(vp, VWRITE, Cred::real(p.ucred()), p)?;
    let Some(so2) = vp.socket() else {
        return Err(Errno::ECONNREFUSED);
    };
    if so.so_type() != so2.so_type() {
        return Err(Errno::EPROTOTYPE);
    }

    if so.pr_flags(PR_CONNREQUIRED) {
        let w2 = solock(&so2);

        if !so2.has_options(SO_ACCEPTCONN) {
            return Err(Errno::ECONNREFUSED);
        }
        let Some(so3) = sonewconn(&so2, 0, M_WAIT) else {
            return Err(Errno::ECONNREFUSED);
        };

        // `unp_addr', `unp_connid' and 'UNP_FEIDSBIND' flag are immutable since we set them in
        // uipc_bind(): read here, under the listener's lock.
        let unp2 = unp_of(&so2);
        let (addr2, connid2, feidsbind) = {
            let s2 = unp2.state(&w2);
            (s2.addr, s2.connid, s2.flags & UNP_FEIDSBIND != 0)
        };

        // Since `so2' is protected by vnode(9) lock, `so3' can't be PRU_ABORT'ed here.
        drop(w2);
        sounlock_detached(&so3);
        let (mut wa, wb) = solock_pair(so, &so3);
        let Some(mut wb) = wb else {
            kpanic(format_args!(
                "unp_connect: sonewconn gave the socket itself"
            ));
        };

        let unp = unp_of(so);
        let unp3 = unp_of(&so3);
        {
            let s3 = st(&unp3, &mut wb);
            if addr2.is_some() {
                s3.addr = addr2;
            }
            s3.connid = peercred_of(p);
            s3.flags |= UNP_FEIDS;
        }
        if feidsbind {
            let s = st(&unp, &mut wa);
            s.connid = connid2;
            s.flags |= UNP_FEIDS;
        }

        let error = unp_connect2(so, &mut wa, &so3, Some(&mut wb));
        // `so2' can't be PRU_ABORT'ed concurrently
        drop(wb);
        drop(wa);
        error
    } else {
        let (mut wa, mut wb) = solock_pair(so, &so2);
        let error = unp_connect2(so, &mut wa, &so2, wb.as_mut());
        drop(wb);
        drop(wa);
        error
    }
}

/// `unp_connect2(so, so2)`: connects two locked sockets of one type (`w2` is `None` when
/// they are one socket, a datagram socket connected to itself).
pub fn unp_connect2<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    so2: &Socket<G>,
    w2: Option<&mut SoLocked<'_>>,
) -> Result<(), Errno> {
    if so2.so_type() != so.so_type() {
        return Err(Errno::EPROTOTYPE);
    }
    let unp = unp_of(so);
    let unp2 = unp_of(so2);
    drop(st(&unp, w).conn.replace(unp2.clone()));
    match so.so_type() {
        SOCK_DGRAM => {
            match w2 {
                Some(w2) => unp2.refs.insert_head(wit(w2), unp.as_pin()),
                None => unp2.refs.insert_head(wit(w), unp.as_pin()),
            }
            soisconnected(so);
        }
        SOCK_STREAM | SOCK_SEQPACKET => {
            let s2 = match w2 {
                Some(w2) => st(&unp2, w2),
                None => st(&unp2, w),
            };
            drop(s2.conn.replace(unp.clone()));
            soisconnected(so);
            soisconnected(so2);
        }
        _ => kpanic(format_args!("unp_connect2")),
    }
    Ok(())
}

/// `unp_disconnect(unp)`: takes `so` (of control block `unp`) off its peer.
fn unp_disconnect<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>, unp: &Unpcb<G>) {
    unp_solock_peer(so, unp, w, |peer, w| {
        let Some(peer) = peer else {
            return;
        };
        let old = st(unp, w).conn.take();
        match so.so_type() {
            SOCK_DGRAM => {
                match peer.w {
                    Some(w2) => peer.unp.refs.remove(wit(w2), unp),
                    None => peer.unp.refs.remove(wit(w), unp),
                }
                so.st_clear(SS_ISCONNECTED);
            }
            SOCK_STREAM | SOCK_SEQPACKET => {
                so.so_snd.set_cc_mbcnt_unlocked(0, 0);
                soisdisconnected(so);
                let old2 = match peer.w {
                    Some(w2) => st(peer.unp, w2).conn.take(),
                    None => st(peer.unp, w).conn.take(),
                };
                peer.so.so_snd.set_cc_mbcnt_unlocked(0, 0);
                soisdisconnected(peer.so);
                drop(old2);
            }
            _ => {}
        }
        drop(old);
    });
}

/// `fptounp(fp)`: `f` on the control block of a UNIX socket file, `None` for any other file
/// (the socket's slot read as the C reads `so_pcb`, without the socket lock). `f` runs
/// inside the slot's read section, so it must not sleep nor take a lock that may
/// (`hal::slot`'s rule): the collector's scan, atomics alone. A caller that takes
/// `unp_gc_lock` uses [`fptounp`].
fn with_fptounp<G: ProcGlue, R>(
    fp: &File<G::FileGlue>,
    f: impl FnOnce(&Unpcb<G>) -> R,
) -> Option<R> {
    if fp.f_type() != DTYPE_SOCKET {
        return None;
    }
    let so = <G::FileGlue as FileGlue>::socket_of(fp.data()?)?;
    if so.dom_family() != i32::from(AF_UNIX) {
        return None;
    }
    let pcb = so.pcb_read();
    let unp = pcb.as_ref()?.downcast_ref::<Unpcb<G>>()?;
    Some(f(unp))
}

/// `fptounp(fp)` counted: the control block of a UNIX socket file, its slot's read section
/// over before the caller sleeps or locks (`unp_msgcount_inc`, `unp_msgcount_dec`).
fn fptounp<G: ProcGlue>(fp: &File<G::FileGlue>) -> Option<UnpRef<G>> {
    with_fptounp::<G, _>(fp, |unp| unp_ref(unp))
}

/// How many of `fp`'s references are its UNIX socket's control block's own: 1 while the
/// block holds `fp` as its `unp_file` (the socket's file in flight), else 0. The C's
/// `unp_file` is not counted, so `kern.file` reports `f_count` less this, the C's count.
pub fn unp_file_refs<G: ProcGlue>(fp: &File<G::FileGlue>) -> u32 {
    let Some(unp) = fptounp::<G>(fp) else {
        return 0;
    };
    let t = tables::<G>();
    let g = t.unp_gc_lock.read();
    let n = u32::from(
        unp.file
            .get(&g)
            .as_ref()
            .is_some_and(|f| ptr::eq::<File<G::FileGlue>>(&**f, fp)),
    );
    drop(g);
    n
}

/// `unp_msgcount++` (and `unp_file = fp`) of a UNIX socket file going in flight, under
/// `unp_gc_lock`.
fn unp_msgcount_inc<G: ProcGlue>(fp: &Fp<G>) {
    let t = tables::<G>();
    let Some(unp) = fptounp::<G>(fp) else {
        return;
    };
    let mut g = t.unp_gc_lock.write();
    unp.msgcount.fetch_add(1, Ordering::Relaxed);
    let file = unp.file.get_mut(&mut g);
    if file.is_none() {
        *file = Some(fp.clone());
    }
    drop(g);
    drop(unp);
}

/// `unp_msgcount--` of a UNIX socket file leaving flight, under `unp_gc_lock`: the control
/// block's own reference to the file when the count falls to 0, for the caller to drop
/// with no lock held.
fn unp_msgcount_dec<G: ProcGlue>(fp: &File<G::FileGlue>) -> Option<Fp<G>> {
    let t = tables::<G>();
    let unp = fptounp::<G>(fp)?;
    let mut g = t.unp_gc_lock.write();
    let last = if unp.msgcount.fetch_sub(1, Ordering::Relaxed) == 1 {
        unp.file.get_mut(&mut g).take()
    } else {
        None
    };
    drop(g);
    drop(unp);
    last
}

/// The control message's header (`mtod(m, struct cmsghdr *)`), `None` for an mbuf too short
/// to hold one.
fn cmsghdr_of(m: &Mbuf) -> Option<Cmsghdr> {
    pod_from_bytes::<Cmsghdr>(m.data())
}

/// `cm->cmsg_len = len` in the control message `m`.
fn set_cmsg_len(m: &mut Mbuf, len: usize) {
    match m.data_mut().and_then(|d| d.get_mut(..4)) {
        Some(d) => d.copy_from_slice(&(len as u32).to_ne_bytes()),
        None => kpanic(format_args!("unp: a read-only control message")),
    }
}

/// `((struct fdpass *)CMSG_DATA(cm))[i]`.
fn fdpass_at(m: &Mbuf, i: usize) -> Fdpass {
    let at = HDR + i * FDPASS_SIZE;
    match m
        .data()
        .get(at..at + FDPASS_SIZE)
        .and_then(pod_from_bytes::<Fdpass>)
    {
        Some(e) => e,
        None => kpanic(format_args!("unp: fdpass {i} past the control message")),
    }
}

/// `((struct fdpass *)CMSG_DATA(cm))[i] = e`.
fn set_fdpass_at(m: &mut Mbuf, i: usize, e: Fdpass) {
    let at = HDR + i * FDPASS_SIZE;
    match m.data_mut().and_then(|d| d.get_mut(at..at + FDPASS_SIZE)) {
        Some(d) => d.copy_from_slice(bytes_of(core::slice::from_ref(&e))),
        None => kpanic(format_args!("unp: fdpass {i} past the control message")),
    }
}

/// `((int *)CMSG_DATA(cm))[i]`.
fn int_at(m: &Mbuf, i: usize) -> i32 {
    let at = HDR + i * size_of::<i32>();
    match m.data().get(at..at + 4) {
        Some(b) => i32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
        None => kpanic(format_args!("unp: descriptor {i} past the control message")),
    }
}

/// `((int *)CMSG_DATA(cm))[i] = v`.
fn set_int_at(m: &mut Mbuf, i: usize, v: i32) {
    let at = HDR + i * size_of::<i32>();
    match m.data_mut().and_then(|d| d.get_mut(at..at + 4)) {
        Some(d) => d.copy_from_slice(&v.to_ne_bytes()),
        None => kpanic(format_args!("unp: descriptor {i} past the control message")),
    }
}

/// The in-flight table locked with `n` free slots at least (`n` at most what `unp_rights`
/// reserved: the table grows by doubling, its new array made with the mutex released and
/// swapped in under it, the old one freed after), and `f` run on it.
fn with_room<G: ProcGlue, R>(
    t: &UnixTables<G>,
    n: usize,
    f: impl FnOnce(&mut MutexedGuard<'_, Inflight<G::FileGlue>>) -> R,
) -> R {
    loop {
        let mut g = t.inflight.lock();
        if g.nfree as usize >= n {
            return f(&mut g);
        }
        let len = g.len();
        drop(g);

        let want = (len * 2).max(len + n).max(64);
        let Some(new) = MArray::new(M_TEMP, M_WAITOK, want, |i| InflightSlot {
            fp: FileSlot::new(),
            next: if i + 1 < want { (i + 1) as u32 } else { NONE },
        }) else {
            kpanic(format_args!("unp: no memory for the in-flight table"));
        };
        let old = {
            let mut g = t.inflight.lock();
            if g.len() == len {
                let mut new = new;
                let inf = &mut *g;
                if let Some(old) = inf.slots.as_mut() {
                    for (to, from) in new.iter_mut().zip(old.iter_mut()) {
                        core::mem::swap(&mut to.fp, &mut from.fp);
                        to.next = from.next;
                    }
                }
                // The new slots (from `len` on, linked in order) before the old free ones.
                if let Some(last) = new.get_mut(want - 1) {
                    last.next = inf.free;
                }
                inf.free = len as u32;
                inf.nfree += (want - len) as u32;
                inf.slots.replace(new)
            } else {
                // Grown meanwhile: look again.
                None
            }
        };
        drop(old);
    }
}

/// `unp_externalize(rights, controllen, flags)` for the receiving thread `p`: the files in
/// flight in the control message `rights` installed in `p`'s descriptor table, their numbers
/// written over the message; on any error they go to the collector.
pub fn unp_externalize<G: ProcGlue>(
    rights: &mut Mbuf,
    controllen: u32,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let t = tables::<G>();
    let Some(cm) = cmsghdr_of(rights) else {
        return Err(Errno::EINVAL);
    };

    // This code only works because SCM_RIGHTS is the only supported control message type on
    // unix sockets. Enforce this here.
    if cm.cmsg_type != SCM_RIGHTS || cm.cmsg_level != SOL_SOCKET {
        return Err(Errno::EINVAL);
    }

    let Some(data) = (cm.cmsg_len as usize).checked_sub(HDR) else {
        kpanic(format_args!(
            "unp_externalize: a rights message shorter than its header"
        ));
    };
    let nfds = data / FDPASS_SIZE;
    let controllen = (controllen as usize).saturating_sub(HDR);
    if nfds > controllen / size_of::<i32>() {
        // out:
        unp_discard::<G>(rights, nfds);
        return Err(Errno::EMSGSIZE);
    }

    // The files out of the in-flight table, with their flags (the C's `fds` array beside).
    let Some(mut pass) = MArray::new(M_TEMP, M_WAITOK, nfds, |_| Pass::<G::FileGlue> {
        fp: FileSlot::new(),
        flags: 0,
        fd: -1,
    }) else {
        unp_discard::<G>(rights, nfds);
        return Err(Errno::ENOMEM);
    };
    for lo in (0..nfds).step_by(CHUNK) {
        let hi = (lo + CHUNK).min(nfds);
        let mut g = t.inflight.lock();
        for (i, x) in pass.iter_mut().enumerate().take(hi).skip(lo) {
            let e = fdpass_at(rights, i);
            x.fp.fill(g.take(e.slot));
            x.flags = e.flags as u8;
        }
        drop(g);
    }

    let error = externalize_files(&mut pass, flags, p);
    if let Err(e) = error {
        // out: the files back in flight, deferred (no lock required: we are the only `cm`
        // holder).
        unp_defer_files::<G>(&mut pass);
        for i in 0..nfds {
            set_fdpass_at(rights, i, Fdpass::default());
        }
        return Err(e);
    }

    t.unp_rights.fetch_sub(nfds as i32, Ordering::Relaxed);

    // Copy temporary array to message and adjust length, in case of transition from large
    // struct file pointers to ints.
    for (i, x) in pass.iter().enumerate() {
        set_int_at(rights, i, x.fd);
    }
    set_cmsg_len(rights, cmsg_len(nfds * size_of::<i32>()));
    rights.m_len().set(cmsg_len(nfds * size_of::<i32>()) as u32);
    drop(pass);
    Ok(())
}

/// A file `unp_externalize` moves: the reference, the sender's flags, the new descriptor.
struct Pass<F: FileGlue> {
    /// The reference (out of the table; a clone of the descriptor's once it is installed).
    fp: FileSlot<F>,
    /// The sender's `UF_PLEDGED`.
    flags: u8,
    /// The descriptor (`fds[i]`).
    fd: i32,
}

/// `unp_externalize`'s checks and installation of the files of `pass` for `p`.
fn externalize_files<G: ProcGlue>(
    pass: &mut MArray<Pass<G::FileGlue>>,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let fdp = p.fd();

    // Make sure the recipient should be able to see the descriptors.. fdp->fd_rdir requires
    // KERNEL_LOCK().
    hal::klock::kernel_lock();
    let mut error = Ok(());
    for x in pass.iter() {
        let Some(fp) = x.fp.get() else {
            continue;
        };
        if let Err(e) = G::pledge_recvfd(p, fp) {
            error = Err(e);
            break;
        }

        // No to block devices. If passing a directory, make sure that it is underneath the
        // root.
        if let Some(rdir) = fdp.rdir()
            && fp.f_type() == DTYPE_VNODE
            && let Some(vp) = fp.data().and_then(<G::FileGlue as FileGlue>::vnode_of)
            && (vp.vtype() == VBLK || (vp.vtype() == VDIR && !vn_isunder(vp, rdir, p)))
        {
            error = Err(Errno::EPERM);
            break;
        }
    }
    hal::klock::kernel_unlock();
    error?;

    let mut g = fdplock(fdp);
    'restart: loop {
        // First loop -- allocate file descriptor table slots for the new descriptors.
        for i in 0..pass.len() {
            match fdalloc(p, &mut g, 0) {
                Ok(fd) => {
                    // Make the slot reference the descriptor so that fdalloc() works
                    // properly.. We finalize it all in the loop below.
                    let x = &mut pass[i];
                    let Some(fp) = x.fp.take() else {
                        kpanic(format_args!("unp_externalize: a file passed twice"));
                    };
                    let mut fl = x.flags & UF_PLEDGED;
                    if flags & MSG_CMSG_CLOEXEC != 0 {
                        fl |= UF_EXCLOSE;
                    }
                    if flags & MSG_CMSG_CLOFORK != 0 {
                        fl |= UF_FORKCLOSE;
                    }
                    // A reference kept for the loop below (never the last: the slot holds
                    // one while the table is locked).
                    x.fp.fill(fp.clone());
                    g.place_ofile(fd, fp, fl);
                    x.fd = fd;
                }
                Err(error) => {
                    // Back out what we've done so far.
                    for x in pass.iter_mut().take(i).rev() {
                        let back = fdremove(&mut g, x.fd);
                        drop(x.fp.take());
                        match back {
                            Some(fp) => x.fp.fill(fp),
                            None => kpanic(format_args!("unp_externalize: descriptor emptied")),
                        }
                    }

                    if error == Errno::ENOSPC {
                        fdexpand(p, &mut g)?;
                        continue 'restart;
                    }

                    drop(g);

                    // This is the error that has historically been returned, and some callers
                    // may expect it.
                    return Err(Errno::EMSGSIZE);
                }
            }
        }
        break;
    }

    // Keep `fdp' locked to prevent concurrent close() of just inserted descriptors. Such
    // descriptors could have the only `f_count' reference which is now shared between
    // control message and `fdp'.

    // Now that adding them has succeeded, update all of the descriptor passing state.
    for x in pass.iter_mut() {
        if let Some(fp) = x.fp.take() {
            drop(unp_msgcount_dec::<G>(&fp));
            drop(fp);
        }
    }
    drop(g);
    Ok(())
}

/// The files of `pass` (out of the table) back in flight on the deferred list (`unp_discard`
/// of an `unp_externalize` that failed after it took them out).
fn unp_defer_files<G: ProcGlue>(pass: &mut MArray<Pass<G::FileGlue>>) {
    let t = tables::<G>();
    let n = pass.len();
    // In reverse, so the collector closes them in their order (the C's deferral set).
    for hi in (0..n).rev().step_by(CHUNK).map(|i| i + 1) {
        let lo = hi.saturating_sub(CHUNK);
        with_room(t, hi - lo, |g| {
            for x in pass[lo..hi].iter_mut().rev() {
                if let Some(fp) = x.fp.take() {
                    g.put_deferred(fp);
                }
            }
        });
    }
    task_add(unp_gc_tq(), pin_static(&t.unp_gc_task));
}

/// `unp_internalize(control, p)` of the calling thread (`curproc`).
fn unp_internalize_cur<G: ProcGlue>(control: &mut Mbuf) -> Result<(), Errno> {
    G::with_curproc(|p| match p {
        Some(p) => unp_internalize(control, p),
        None => kpanic(format_args!("unp_internalize: no curproc")),
    })
}

/// `unp_internalize(control, p)`: the descriptor numbers of the `SCM_RIGHTS` message
/// `control` turned into references to `p`'s files held in flight (each a `struct fdpass` of
/// the message, which grows into a cluster if it must); `EMFILE` past `maxfiles / 10` in
/// flight.
pub fn unp_internalize<G: ProcGlue>(control: &mut Mbuf, p: &Thread<G>) -> Result<(), Errno> {
    let t = tables::<G>();
    let mlen = control.m_len().get() as usize;

    // Check for two potential msg_controllen values because IETF stuck their nose in a place
    // it does not belong.
    if mlen < cmsg_len(0) {
        return Err(Errno::EINVAL);
    }
    let Some(cm) = cmsghdr_of(control) else {
        return Err(Errno::EINVAL);
    };
    let clen = cm.cmsg_len as usize;
    if clen < cmsg_len(0) {
        return Err(Errno::EINVAL);
    }
    if cm.cmsg_type != SCM_RIGHTS
        || cm.cmsg_level != SOL_SOCKET
        || !(clen == mlen || mlen == cmsg_align(clen))
    {
        return Err(Errno::EINVAL);
    }
    let nfds = (clen - HDR) / size_of::<i32>();

    // unp_rights + nfds > maxfiles / 10: EMFILE, else reserved.
    let max = i64::from(<G::FileGlue as FileGlue>::maxfiles() / 10);
    let mut cur = t.unp_rights.load(Ordering::Relaxed);
    loop {
        if i64::from(cur) + nfds as i64 > max {
            return Err(Errno::EMFILE);
        }
        match t.unp_rights.compare_exchange_weak(
            cur,
            cur + nfds as i32,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(now) => cur = now,
        }
    }

    let error = internalize_files(control, nfds, p);
    if error.is_err() {
        // nospace:
        t.unp_rights.fetch_sub(nfds as i32, Ordering::Relaxed);
    }
    error
}

/// `unp_internalize`'s room for the `struct fdpass` entries and its walk of the descriptors.
fn internalize_files<G: ProcGlue>(
    control: &mut Mbuf,
    nfds: usize,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let t = tables::<G>();

    // Make sure we have room for the struct file pointers
    loop {
        let neededspace = cmsg_space(nfds * FDPASS_SIZE) as i64 - i64::from(control.m_len().get());
        if neededspace <= i64::from(m_trailingspace(control)) {
            break;
        }
        // if we already have a cluster, the message is just too big
        if control.flags() & M_EXT != 0 {
            return Err(Errno::E2BIG);
        }

        // copy cmsg data temporarily out of the mbuf
        let len = control.m_len().get() as usize;
        let mut tmp = [0u8; MLEN];
        tmp[..len].copy_from_slice(control.data());

        // allocate a cluster and try again
        mclget(control, M_WAIT);
        if control.flags() & M_EXT == 0 {
            return Err(Errno::ENOBUFS); // allocation failed
        }

        // copy the data back into the cluster
        match control.data_mut() {
            Some(d) => d.copy_from_slice(&tmp[..len]),
            None => kpanic(format_args!("unp_internalize: a read-only cluster")),
        }
    }

    // adjust message & mbuf to note amount of space actually used.
    set_cmsg_len(control, cmsg_len(nfds * FDPASS_SIZE));
    control.m_len().set(cmsg_space(nfds * FDPASS_SIZE) as u32);

    // The entries are filled from the last one down, over the ints they replace, a chunk
    // at a time: each chunk's files are checked and taken, then moved into the table in one
    // section of its mutex.
    let fdp = p.fd();
    let g = fdplock(fdp);
    let mut hi = nfds;
    while hi > 0 {
        let lo = hi.saturating_sub(CHUNK);
        let mut got: [FileSlot<G::FileGlue>; CHUNK] = core::array::from_fn(|_| FileSlot::new());
        let mut pledged = [0u8; CHUNK];
        'fds: for i in (lo..hi).rev() {
            let fd = int_at(control, i);
            let error = 'fail: {
                let Some((fp, ofl)) = fd_getfile_flags(fdp, fd) else {
                    break 'fail (Errno::EBADF, None);
                };
                if fp.count() >= FDUP_MAX_COUNT {
                    break 'fail (Errno::EDEADLK, Some(fp));
                } else if ofl & UF_PLEDGEOPEN != 0 {
                    break 'fail (Errno::EPERM, Some(fp));
                }
                if let Err(e) = G::pledge_sendfd(p, &fp) {
                    break 'fail (e, Some(fp));
                }

                // kqueue and vmm descriptors cannot be copied
                if fp.f_type() == DTYPE_KQUEUE || fp.f_type() == DTYPE_VMM {
                    break 'fail (Errno::EINVAL, Some(fp));
                }
                // NKCOV is 0: no kcov descriptor to refuse.
                pledged[i - lo] = ofl & UF_PLEDGED;
                unp_msgcount_inc::<G>(&fp);
                got[i - lo].fill(fp);
                continue 'fds;
            };

            // fail:
            drop(g);
            let (e, fp) = error;
            if let Some(fp) = fp {
                let _ = fp.frele(Some(p));
            }
            // Back out what we just did: this chunk's files above `i`, then the entries
            // already in the table.
            for slot in &mut got[i + 1 - lo..hi - lo] {
                if let Some(fp) = slot.take() {
                    drop(unp_msgcount_dec::<G>(&fp));
                    let _ = fp.frele(Some(p));
                }
            }
            for c in (hi..nfds).step_by(CHUNK) {
                let end = (c + CHUNK).min(nfds);
                let mut back: [FileSlot<G::FileGlue>; CHUNK] =
                    core::array::from_fn(|_| FileSlot::new());
                let mut inf = t.inflight.lock();
                for (k, j) in (c..end).enumerate() {
                    back[k].fill(inf.take(fdpass_at(control, j).slot));
                }
                drop(inf);
                for slot in &mut back {
                    if let Some(fp) = slot.take() {
                        drop(unp_msgcount_dec::<G>(&fp));
                        let _ = fp.frele(Some(p));
                    }
                }
            }
            return Err(e);
        }

        let mut slots = [0u64; CHUNK];
        with_room(t, hi - lo, |inf| {
            for (k, slot) in got[..hi - lo].iter_mut().enumerate() {
                if let Some(fp) = slot.take() {
                    slots[k] = inf.put(fp);
                }
            }
        });
        for i in lo..hi {
            set_fdpass_at(
                control,
                i,
                Fdpass {
                    slot: slots[i - lo],
                    flags: i32::from(pledged[i - lo]),
                    pad: 0,
                },
            );
        }
        hi = lo;
    }
    drop(g);
    Ok(())
}

/// The queue `unp_gc` runs on: `systqmp`, as the C's. In this crate's host tests a queue no
/// thread serves, which they drain by calling [`unp_gc`] themselves: the test glues' file
/// tables are their own threads', so the collector's closes must run on the test's thread,
/// not on the `systqmp` worker another test module starts.
fn unp_gc_tq() -> &'static Taskq {
    #[cfg(test)]
    {
        static TQ: Taskq = crate::task::taskq_unthreaded(b"unpgctq");
        &TQ
    }
    #[cfg(not(test))]
    {
        &crate::task::SYSTQMP
    }
}

/// `unp_gc`'s task function.
fn unp_gc_task<G: ProcGlue>(_arg: *mut core::ffi::c_void) {
    unp_gc::<G>();
}

/// `unp_gc`: closes the files deferred by `unp_discard`, then finds the sockets that only
/// messages in flight reference, in buffers of sockets that only messages in flight
/// reference, and discards what their buffers hold (their files are closed by the next run).
pub fn unp_gc<G: ProcGlue>() {
    let t = tables::<G>();

    let g = t.unp_gc_lock.write();
    if t.unp_gcing.load(Ordering::Relaxed) {
        return;
    }
    t.unp_gcing.store(true, Ordering::Relaxed);
    drop(g);

    // close any fds on the deferred list
    loop {
        let mut fps: [FileSlot<G::FileGlue>; CHUNK] = core::array::from_fn(|_| FileSlot::new());
        let mut inf = t.inflight.lock();
        let mut n = 0;
        while n < CHUNK
            && let Some(fp) = inf.pop_deferred()
        {
            fps[n].fill(fp);
            n += 1;
        }
        drop(inf);
        if n == 0 {
            break;
        }
        for slot in &mut fps[..n] {
            let Some(fp) = slot.take() else {
                continue;
            };
            let own = unp_msgcount_dec::<G>(&fp);
            t.unp_rights.fetch_sub(1, Ordering::Relaxed);
            // closef() expects a refcount of 2: the deferral's reference goes.
            let _ = closef(fp, None);
            drop(own);
        }
    }

    let mut nunref = 0;

    let g = t.unp_gc_lock.write();

    // Determine sockets which may be prospectively dead. Such sockets have their
    // `unp_msgcount' equal to the `f_count' (less the control block's own reference here).
    // If `unp_msgcount' is 0, the socket has not been passed and can't be unreferenced.
    for unp in t.unp_head.iter(&g) {
        unp.gcflags.store(0, Ordering::Relaxed);

        let msgcount = unp.msgcount.load(Ordering::Relaxed);
        if msgcount == 0 {
            continue;
        }
        let Some(fp) = unp.file.get(&g) else {
            continue;
        };
        if i64::from(fp.count()) == msgcount + 1 {
            unp.gcflags.fetch_or(UNP_GCDEAD, Ordering::Relaxed);
            unp.gcrefs.store(msgcount, Ordering::Relaxed);
            nunref += 1;
        }
    }

    // Scan all sockets previously marked as dead. Remove the `unp_gcrefs' reference each
    // socket holds on any dead socket in its buffer.
    for unp in t.unp_head.iter(&g) {
        if unp.gcflags.load(Ordering::Relaxed) & UNP_GCDEAD == 0 {
            continue;
        }
        unp_scan_gcrefs::<G>(t, &unp.socket, false);
    }

    // If the dead socket has `unp_gcrefs' reference counter greater than 0, it can't be
    // unreferenced. Mark it as alive and increment the `unp_gcrefs' reference for each dead
    // socket within its buffer. Repeat this until we have no new alive sockets found.
    loop {
        t.unp_defer.store(0, Ordering::Relaxed);

        for unp in t.unp_head.iter(&g) {
            if unp.gcflags.load(Ordering::Relaxed) & UNP_GCDEAD == 0 {
                continue;
            }
            if unp.gcrefs.load(Ordering::Relaxed) == 0 {
                continue;
            }

            unp.gcflags.fetch_and(!UNP_GCDEAD, Ordering::Relaxed);

            unp_scan_gcrefs::<G>(t, &unp.socket, true);

            kassert!(nunref > 0);
            nunref -= 1;
        }
        if t.unp_defer.load(Ordering::Relaxed) <= 0 {
            break;
        }
    }

    // If there are any unreferenced sockets, then for each dispose of files in its receive
    // buffer and then close it.
    if nunref != 0 {
        for unp in t.unp_head.iter(&g) {
            if unp.gcflags.load(Ordering::Relaxed) & UNP_GCDEAD == 0 {
                continue;
            }
            let sb = &unp.socket.so_rcv;

            // This socket could still be connected and if so it's `so_rcv' is still
            // accessible by concurrent PRU_SEND thread.
            let mut sg = sb.lock();
            let m = sg.take_all_zeroed();
            sg.set_timeo_nsecs(hal::sleep::INFSLP);
            drop(sg);

            let mut m = m;
            if let Some(m) = m.as_deref_mut() {
                unp_dispose::<G>(m);
            }
            m_purge(m);
        }
    }

    t.unp_gcing.store(false, Ordering::Relaxed);
    drop(g);
}

/// The collector's scan of `so`'s receive buffer, under `unp_gc_lock`: for each file in
/// flight there whose socket is marked dead, `unp_remove_gcrefs` (`restore` false) or
/// `unp_restore_gcrefs` (true).
fn unp_scan_gcrefs<G: ProcGlue>(t: &UnixTables<G>, so: &Socket<G>, restore: bool) {
    let rg = so.so_rcv.lock();
    let inf = t.inflight.lock();
    for rec in rg.data().records() {
        unp_scan_record(rec, |m, qfds| {
            for i in 0..qfds {
                let e = fdpass_at(m, i);
                let Some(fp) = inf.get(e.slot) else {
                    continue;
                };
                let _ = with_fptounp::<G, _>(fp, |unp| {
                    if unp.gcflags.load(Ordering::Relaxed) & UNP_GCDEAD == 0 {
                        return;
                    }
                    if restore {
                        unp.gcrefs.fetch_add(1, Ordering::Relaxed);
                        t.unp_defer.fetch_add(1, Ordering::Relaxed);
                    } else {
                        kassert!(unp.gcrefs.load(Ordering::Relaxed) > 0);
                        unp.gcrefs.fetch_sub(1, Ordering::Relaxed);
                    }
                });
            }
        });
    }
    drop(inf);
    drop(rg);
}

/// `unp_dispose(m)`: the files in flight in the records `m` handed to the collector.
pub fn unp_dispose<G: ProcGlue>(m: &mut Mbuf) {
    unp_scan_mut(m, |cm, qfds| unp_discard::<G>(cm, qfds));
}

/// The `SCM_RIGHTS` message of one record (`unp_scan`'s inner walk: the first such control
/// mbuf of the record, then the next record): `op` gets it and its entries' count.
fn unp_scan_record<'a>(rec: impl Iterator<Item = &'a Mbuf>, mut op: impl FnMut(&Mbuf, usize)) {
    for m in rec {
        if let Some(qfds) = rights_of(m) {
            if qfds > 0 {
                op(m, qfds);
            }
            break; // XXX, but saves time
        }
    }
}

/// The entries of an `SCM_RIGHTS` control mbuf, `None` for any other mbuf (`unp_scan`'s
/// test).
fn rights_of(m: &Mbuf) -> Option<usize> {
    if i32::from(m.m_type().get()) != MT_CONTROL
        || (m.m_len().get() as usize) < size_of::<Cmsghdr>()
    {
        return None;
    }
    let cm = cmsghdr_of(m)?;
    if cm.cmsg_level != SOL_SOCKET || cm.cmsg_type != SCM_RIGHTS {
        return None;
    }
    Some((cm.cmsg_len as usize).saturating_sub(HDR) / FDPASS_SIZE)
}

/// `unp_scan(m0, op)` over owned records (by `m_nextpkt`, each a chain by `m_next`) whose
/// entries `op` changes.
fn unp_scan_mut(m0: &mut Mbuf, mut op: impl FnMut(&mut Mbuf, usize)) {
    let mut rec = Some(m0);
    while let Some(r) = rec {
        let mut m = Some(&mut *r);
        while let Some(x) = m {
            if let Some(qfds) = rights_of(x) {
                if qfds > 0 {
                    op(x, qfds);
                }
                break; // XXX, but saves time
            }
            m = x.next_mut();
        }
        rec = r.m_nextpkt.get_mut();
    }
}

/// `unp_discard(rp, nfds)`: the `nfds` files of the control message `cm` deferred to the
/// collector (their slots on the deferred list), the entries cleared; the collector queued.
fn unp_discard<G: ProcGlue>(cm: &mut Mbuf, nfds: usize) {
    let t = tables::<G>();
    // In reverse, so the collector closes them in their order (the C's deferral set).
    let mut hi = nfds;
    while hi > 0 {
        let lo = hi.saturating_sub(CHUNK);
        let mut inf = t.inflight.lock();
        for i in (lo..hi).rev() {
            let e = fdpass_at(cm, i);
            if e.slot != 0 {
                inf.defer(e.slot);
            }
        }
        drop(inf);
        for i in lo..hi {
            set_fdpass_at(cm, i, Fdpass::default());
        }
        hi = lo;
    }

    task_add(unp_gc_tq(), pin_static(&t.unp_gc_task));
}

/// `unp_nam2sun(nam, &sun, &pathlen)`: the NUL-terminated path of the UNIX address in `nam`
/// (in the returned buffer) and its length: `EINVAL` for a short address, a length that is not
/// the mbuf's or past `struct sockaddr_un`, or a path that fills `sun_path` or the address
/// with no room left for its NUL; `EAFNOSUPPORT` for another family.
pub fn unp_nam2sun(nam: &Mbuf) -> Result<([u8; hal::sys::un::SUN_PATH_LEN + 1], usize), Errno> {
    let d = nam.data();
    let m_len = d.len();

    if m_len < 2 {
        return Err(Errno::EINVAL);
    }
    if d[1] != AF_UNIX {
        return Err(Errno::EAFNOSUPPORT);
    }
    let sa_len = usize::from(d[0]);
    if sa_len != m_len {
        return Err(Errno::EINVAL);
    }
    if sa_len > size_of::<SockaddrUn>() {
        return Err(Errno::EINVAL);
    }

    // ensure that sun_path is NUL terminated and fits
    let path = &d[SockaddrUn::PATH_OFFSET..sa_len];
    let size = path.len();
    let len = path.iter().position(|&c| c == 0).unwrap_or(size);
    if len == hal::sys::un::SUN_PATH_LEN {
        return Err(Errno::EINVAL);
    }
    if len == size && m_trailingspace(nam) == 0 {
        return Err(Errno::EINVAL);
    }
    let mut buf = [0u8; hal::sys::un::SUN_PATH_LEN + 1];
    buf[..len].copy_from_slice(&path[..len]);
    Ok((buf, len))
}

impl<G: ProcGlue> Usrreqs<G> for UipcUsrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        _proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        uipc_attach(so, w, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        uipc_detach(so, w)
    }
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        uipc_bind(so, w, nam, p)
    }
    fn listen(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        uipc_listen(so, w)
    }
    fn connect(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &Mbuf) -> Result<(), Errno> {
        uipc_connect(so, w, nam)
    }
    fn accept(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        uipc_peeraddr(so, w, nam);
        Ok(())
    }
    fn disconnect(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        let unp = unp_of(so);
        unp_disconnect(so, w, &unp);
        Ok(())
    }
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        uipc_shutdown(so, w);
        Ok(())
    }
    fn rcvd(&self, so: &Socket<G>, w: &mut SoLocked<'_>) {
        uipc_rcvd(so, w);
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        _addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        uipc_send(so, w, top, control)
    }
    fn abort(&self, so: &Socket<G>, w: &mut SoLocked<'_>) {
        uipc_abort(so, w);
    }
    fn sense(&self, so: &Socket<G>, w: &mut SoLocked<'_>, ub: &mut Stat) -> Result<(), Errno> {
        uipc_sense(so, w, ub);
        Ok(())
    }
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        let unp = unp_of(so);
        uipc_setaddr(unp.state(w).addr.as_ref(), nam);
        Ok(())
    }
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        uipc_peeraddr(so, w, nam);
        Ok(())
    }
    fn connect2(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        so2: &Socket<G>,
        w2: Option<&mut SoLocked<'_>>,
    ) -> Result<(), Errno> {
        uipc_connect2(so, w, so2, w2)
    }
}

impl<G: ProcGlue> Usrreqs<G> for UipcDgramUsrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        _proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        uipc_attach(so, w, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        uipc_detach(so, w)
    }
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        uipc_bind(so, w, nam, p)
    }
    fn listen(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        uipc_listen(so, w)
    }
    fn connect(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &Mbuf) -> Result<(), Errno> {
        uipc_connect(so, w, nam)
    }
    fn disconnect(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        let unp = unp_of(so);
        unp_disconnect(so, w, &unp);
        Ok(())
    }
    fn shutdown(&self, so: &Socket<G>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        // uipc_dgram_shutdown(so)
        socantsendmore(so);
        Ok(())
    }
    fn rcvd(&self, so: &Socket<G>, _w: &mut SoLocked<'_>) {
        // The C's table has no pru_rcvd; soreceive calls it only with PR_WANTRCVD.
        kpanic(format_args!("pru_rcvd of the datagram socket {so:p}"));
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        uipc_dgram_send(so, w, top, addr, control)
    }
    fn abort(&self, so: &Socket<G>, _w: &mut SoLocked<'_>) {
        // The C's table has no pru_abort; only a listener's queued sockets are aborted, and a
        // datagram socket does not listen.
        kpanic(format_args!("pru_abort of the datagram socket {so:p}"));
    }
    fn sense(&self, so: &Socket<G>, w: &mut SoLocked<'_>, ub: &mut Stat) -> Result<(), Errno> {
        uipc_sense(so, w, ub);
        Ok(())
    }
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        let unp = unp_of(so);
        uipc_setaddr(unp.state(w).addr.as_ref(), nam);
        Ok(())
    }
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        uipc_peeraddr(so, w, nam);
        Ok(())
    }
    fn connect2(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        so2: &Socket<G>,
        w2: Option<&mut SoLocked<'_>>,
    ) -> Result<(), Errno> {
        uipc_connect2(so, w, so2, w2)
    }
}

const _: () = {
    assert!(size_of::<Fdpass>() == FDPASS_SIZE);
    assert!(HDR == 16);
    assert!(MCLBYTES >= cmsg_space(0));
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    //! The UNIX domain over the test file system's glue (`TestGlue`: `namei` on its tree, a
    //! test thread with a descriptor table): stream, datagram and sequenced packet pairs, the
    //! accept queue through `bind`/`listen`/`connect`, `SCM_RIGHTS` with its checks and
    //! refusals (the C's errors table-driven), the collector of unreceived rights and of a
    //! cycle, `unp_nam2sun`'s vectors, `net.unix`, the pool and the `struct fdpass` size.

    use core::ffi::c_void;
    use std::sync::MutexGuard;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use hal::sys::socket::{MSG_DONTWAIT, MSG_NOSIGNAL, MSG_TRUNC, SHUT_WR, SO_PEERCRED, SO_TYPE};
    use hal::user::UserPtr;

    use super::*;
    use crate::descrip::{falloc, fd_getfile, fdinsert, sys_close};
    use crate::fs::testfs::{TestFiles, TestGlue, test_thread_cached};
    use crate::net::domain::{domaininit, soinit};
    use crate::net::mbuf::{MT_SONAME, MT_SOOPTS, m_get, mbinit, nmbclust_update};
    use crate::net::socket::{
        soaccept, soaccept_take, sobind, soconnect, soconnect2, socreate, sogetopt, solisten,
        solock_shared, soreceive, sosend, soshutdown,
    };
    use crate::testutil::{self, TestFileData, USER_BASE, user_bytes};
    use crate::uio::{Iovec, Uio, UioRw, UioSeg};

    pub(crate) type G = TestGlue;
    pub(crate) type So = SocketRef<G>;

    /// The tests' domains: the UNIX domain alone.
    static DOMS: [&dyn Domain<G>; 1] = [&Unix::<G>::NEW];

    /// Where the tests' sends copy from and receives copy to, in the user-copy double.
    const USEND: usize = USER_BASE + 0x2_0000;
    /// The receive buffer.
    const URECV: usize = USER_BASE + 0x3_0000;

    /// The test thread with its descriptor table and root, the mbufs, the socket pools and
    /// the UNIX domain.
    pub(crate) fn setup() -> (MutexGuard<'static, ()>, &'static Thread<G>) {
        let (g, p) = test_thread_cached(64 << 20, 64);
        testutil::kernel_maps_install();
        mbinit(&crate::net::mbuf::tests::TEST_NMBCLUST);
        nmbclust_update(256 * 1024).expect("the limit");
        soinit::<G>();
        domaininit::<G>(&DOMS);
        unp_init::<G>();
        testutil::set_pledge_fd(None);
        (g, p)
    }

    /// Every UNIX socket gone, nothing in flight: the tests leave the glue's tables as they
    /// found them.
    pub(crate) fn teardown() {
        let t = tables::<G>();
        let g = t.unp_gc_lock.write();
        assert!(t.unp_head.is_empty(&g));
        drop(g);
        assert_eq!(t.rights(), 0);
        assert_eq!(t.inflight.lock().deferred, NONE);
    }

    /// The collector run until the deferred files are closed and `want` rights are left in
    /// flight: by hand, or by `systqmp`'s worker where another test of the binary started it
    /// (the task `unp_discard` queues runs there then, at any time).
    pub(crate) fn collect(want: i32) {
        let t = tables::<G>();
        for _ in 0..100_000 {
            unp_gc::<G>();
            if t.rights() == want
                && t.inflight.lock().deferred == NONE
                && !t.unp_gcing.load(Ordering::Relaxed)
            {
                return;
            }
            std::thread::yield_now();
        }
        std::panic!("unp_gc: {} rights in flight, {want} wanted", t.rights());
    }

    /// The socket file's operations, for a stand-in file of another type.
    static SOCKET_OPS: crate::net::file::SocketFileOps<G> = crate::net::file::SocketFileOps::NEW;

    /// A file at a new descriptor of `p`: a socket file of `so` (the glue's `socket_file`),
    /// or, without one, a file of type `ty` holding nothing.
    pub(crate) fn file(p: &Thread<G>, ty: i32, so: Option<So>) -> i32 {
        let mut g = crate::descrip::fdplock(p.fd());
        let (fp, fd) = falloc(p, &mut g).expect("falloc");
        match so {
            Some(so) => {
                let (ops, data) = TestFiles::socket_file(so);
                fp.install(ty, ops, data);
            }
            None => fp.install(ty, &SOCKET_OPS, TestFileData::default()),
        }
        fp.f_flag.store(
            (hal::sys::fcntl::FREAD | hal::sys::fcntl::FWRITE) as u32,
            Ordering::Relaxed,
        );
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        fd
    }

    /// A UNIX socket of `type_` and its descriptor.
    pub(crate) fn sock(p: &Thread<G>, type_: i32) -> (So, i32) {
        let so = socreate(p, i32::from(AF_UNIX), type_, 0).expect("socreate");
        let fd = file(p, DTYPE_SOCKET, Some(so.clone()));
        (so, fd)
    }

    /// `socketpair(2)`: two connected sockets and their descriptors.
    pub(crate) fn pair(p: &Thread<G>, type_: i32) -> ((So, i32), (So, i32)) {
        let a = sock(p, type_);
        let b = sock(p, type_);
        soconnect2(&a.0, &b.0).expect("connect2");
        if type_ == SOCK_DGRAM {
            soconnect2(&b.0, &a.0).expect("connect2");
        }
        (a, b)
    }

    /// A user uio of `len` bytes at `at`, of thread `p`.
    pub(crate) fn uio<'a>(p: &'a Thread<G>, iov: &'a mut [Iovec; 1], rw: UioRw) -> Uio<'a, G> {
        Uio {
            uio_resid: iov[0].iov_len,
            uio_iov: iov,
            uio_offset: 0,
            uio_segflg: UioSeg::UIO_USERSPACE,
            uio_rw: rw,
            uio_procp: Some(p),
        }
    }

    /// `send(2)` of `data` with `control`: the bytes sent.
    pub(crate) fn sendc(
        p: &Thread<G>,
        so: &So,
        data: &[u8],
        control: Option<MbufBox>,
        flags: i32,
    ) -> Result<usize, Errno> {
        UserPtr::<u8>::new(USEND)
            .copyout_slice(data)
            .expect("user memory");
        let mut iov = [Iovec {
            iov_base: USEND as *mut c_void,
            iov_len: data.len(),
        }];
        let mut u = uio(p, &mut iov, UioRw::UIO_WRITE);
        sosend(so, None, Some(&mut u), None, control, flags)?;
        Ok(data.len() - u.uio_resid)
    }

    /// `send(2)` of `data`.
    pub(crate) fn send(p: &Thread<G>, so: &So, data: &[u8], flags: i32) -> Result<usize, Errno> {
        sendc(p, so, data, None, flags)
    }

    /// `recvmsg(2)` into `len` bytes: the bytes, the flags, the control and the address.
    #[allow(clippy::type_complexity, reason = "a test's tuple")]
    pub(crate) fn recvc(
        p: &Thread<G>,
        so: &So,
        len: usize,
        flags: i32,
        controllen: u32,
    ) -> Result<(Vec<u8>, i32, Option<MbufBox>, Option<MbufBox>), Errno> {
        let mut iov = [Iovec {
            iov_base: URECV as *mut c_void,
            iov_len: len,
        }];
        let mut u = uio(p, &mut iov, UioRw::UIO_READ);
        let mut fl = flags;
        let mut from = None;
        let mut ctl = None;
        let want = controllen != 0;
        soreceive(
            so,
            Some(&mut from),
            &mut u,
            None,
            want.then_some(&mut ctl),
            Some(&mut fl),
            controllen,
        )?;
        let n = len - u.uio_resid;
        let bytes = user_bytes(URECV, n).expect("user memory");
        Ok((bytes, fl, ctl, from))
    }

    /// `recv(2)` into `len` bytes: the bytes and the flags.
    pub(crate) fn recv(
        p: &Thread<G>,
        so: &So,
        len: usize,
        flags: i32,
    ) -> Result<(Vec<u8>, i32), Errno> {
        recvc(p, so, len, flags, 0).map(|(b, f, _, _)| (b, f))
    }

    /// An `SCM_RIGHTS` message of the descriptors `fds` (in a cluster past `MLEN`).
    pub(crate) fn rights(fds: &[i32]) -> MbufBox {
        let mut m = m_get(M_WAIT, MT_CONTROL).expect("an mbuf");
        let len = cmsg_len(fds.len() * 4);
        if len > MLEN {
            mclget(&mut m, M_WAIT);
        }
        m.m_len().set(len as u32);
        let d = m.data_mut().expect("writable");
        d[..4].copy_from_slice(&(len as u32).to_ne_bytes());
        d[4..8].copy_from_slice(&SOL_SOCKET.to_ne_bytes());
        d[8..12].copy_from_slice(&SCM_RIGHTS.to_ne_bytes());
        for (i, fd) in fds.iter().enumerate() {
            d[HDR + 4 * i..HDR + 4 * i + 4].copy_from_slice(&fd.to_ne_bytes());
        }
        m
    }

    /// The descriptors of a received `SCM_RIGHTS` message.
    pub(crate) fn fds_of(m: &Mbuf) -> Vec<i32> {
        let cm = cmsghdr_of(m).expect("a header");
        let n = (cm.cmsg_len as usize - HDR) / 4;
        (0..n).map(|i| int_at(m, i)).collect()
    }

    /// An address mbuf of the path `path` (`sun_len` the whole address's).
    pub(crate) fn sun(path: &[u8]) -> MbufBox {
        let mut m = m_get(M_WAIT, MT_SONAME).expect("an mbuf");
        let len = 2 + path.len() + 1;
        m.m_len().set(len as u32);
        let d = m.data_mut().expect("writable");
        d.fill(0);
        d[0] = len as u8;
        d[1] = AF_UNIX;
        d[2..2 + path.len()].copy_from_slice(path);
        m
    }

    /// The items `unpcb_pool` lends now.
    fn unpcbs() -> u64 {
        tables::<G>().unpcbpl.nout()
    }

    /// The items `sockpl` lends now.
    fn sockets() -> u64 {
        net::<G>().sockpl.nout()
    }

    #[test]
    fn stream_pair_moves_bytes_both_ways_and_shuts_down() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_STREAM);
        assert!(a.has_state(SS_ISCONNECTED) && b.has_state(SS_ISCONNECTED));

        assert_eq!(send(p, &a, b"hello", 0), Ok(5));
        // The sender's buffer mirrors the receiver's, for back pressure.
        assert_eq!(b.so_rcv.cc(), 5);
        assert_eq!(a.so_snd.cc(), 5);
        assert_eq!(recv(p, &b, 2, 0), Ok((b"he".to_vec(), 0)));
        assert_eq!(a.so_snd.cc(), 3);
        assert_eq!(recv(p, &b, 16, 0), Ok((b"llo".to_vec(), 0)));
        assert_eq!(a.so_snd.cc(), 0);
        assert_eq!(recv(p, &b, 16, MSG_DONTWAIT), Err(Errno::EWOULDBLOCK));

        assert_eq!(send(p, &b, b"world", 0), Ok(5));
        assert_eq!(recv(p, &a, 16, 0), Ok((b"world".to_vec(), 0)));

        // socketpair(2) records the creator as both peers.
        let mut m = m_get(M_WAIT, MT_SOOPTS).expect("an mbuf");
        assert_eq!(sogetopt(&a, SOL_SOCKET, SO_PEERCRED, &mut m), Ok(()));
        let cred = pod_from_bytes::<Sockpeercred>(m.data()).expect("credentials");
        assert_eq!(cred, peercred_of(p));
        assert_eq!(sogetopt(&b, SOL_SOCKET, SO_TYPE, &mut m), Ok(()));
        assert_eq!(pod_from_bytes::<i32>(m.data()), Some(SOCK_STREAM));
        drop(m);

        // Shut down a's sending side: b reads EOF, a cannot send, b still can.
        assert_eq!(soshutdown(&a, SHUT_WR), Ok(()));
        assert!(b.so_rcv.has_state(crate::net::sockbuf::SS_CANTRCVMORE));
        assert_eq!(recv(p, &b, 16, 0), Ok((vec![], 0)));
        assert_eq!(send(p, &a, b"x", MSG_NOSIGNAL), Err(Errno::EPIPE));
        assert_eq!(send(p, &b, b"!", 0), Ok(1));
        assert_eq!(recv(p, &a, 16, 0), Ok((b"!".to_vec(), 0)));

        // Closing a disconnects b.
        drop(a);
        assert_eq!(sys_close(p, fa), Ok(()));
        assert!(!b.has_state(SS_ISCONNECTED));
        assert_eq!(send(p, &b, b"?", MSG_NOSIGNAL), Err(Errno::EPIPE));
        assert_eq!(recv(p, &b, 16, 0), Ok((vec![], 0)));
        drop(b);
        assert_eq!(sys_close(p, fb), Ok(()));
        teardown();
    }

    #[test]
    fn datagrams_keep_their_boundaries_and_sender() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_DGRAM);

        assert_eq!(send(p, &a, b"first", 0), Ok(5));
        assert_eq!(send(p, &a, b"second", 0), Ok(6));

        // One record per read, with the sender's address (unbound: sun_noname).
        let (bytes, _, _, from) = recvc(p, &b, 16, 0, 0).expect("a datagram");
        assert_eq!(bytes, b"first");
        assert_eq!(from.as_deref().map(Mbuf::data), Some(&SUN_NONAME[..]));

        // A short read truncates the datagram and drops the rest of it.
        assert_eq!(recv(p, &b, 3, 0), Ok((b"sec".to_vec(), MSG_TRUNC)));
        assert_eq!(recv(p, &b, 16, MSG_DONTWAIT), Err(Errno::EWOULDBLOCK));

        // The pair is connected both ways.
        assert_eq!(send(p, &b, b"back", 0), Ok(4));
        assert_eq!(recv(p, &a, 16, 0), Ok((b"back".to_vec(), 0)));
        drop((a, b));
        assert_eq!(sys_close(p, fa), Ok(()));
        assert_eq!(sys_close(p, fb), Ok(()));
        teardown();
    }

    #[test]
    fn seqpacket_keeps_records() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_SEQPACKET);

        assert_eq!(send(p, &a, b"ab", 0), Ok(2));
        assert_eq!(send(p, &a, b"cd", 0), Ok(2));
        assert_eq!(recv(p, &b, 16, 0), Ok((b"ab".to_vec(), 0)));
        assert_eq!(recv(p, &b, 16, 0), Ok((b"cd".to_vec(), 0)));
        drop((a, b));
        assert_eq!(sys_close(p, fa), Ok(()));
        assert_eq!(sys_close(p, fb), Ok(()));
        teardown();
    }

    #[test]
    fn bind_listen_connect_and_accept_go_through_namei() {
        let (_g, p) = setup();
        let before = (unpcbs(), sockets());
        let (l, fl) = sock(p, SOCK_STREAM);

        // Only a bound socket listens.
        let mut w = solock_shared(&l);
        assert_eq!(solisten(&l, &mut w, 5), Err(Errno::EINVAL));
        assert_eq!(sobind(&l, &mut w, &sun(b"/sock"), p), Ok(()));
        // A second bind of the socket, and a bind to a name in use, are refused.
        assert_eq!(sobind(&l, &mut w, &sun(b"/sock2"), p), Err(Errno::EINVAL));
        assert_eq!(solisten(&l, &mut w, 5), Ok(()));
        drop(w);
        let (other, fo) = sock(p, SOCK_STREAM);
        let mut w = solock_shared(&other);
        assert_eq!(
            sobind(&other, &mut w, &sun(b"/sock"), p),
            Err(Errno::EADDRINUSE)
        );
        // The listener's name, and a connection to a name that is not there or not a
        // socket of the type.
        let mut nam = m_get(M_WAIT, MT_SONAME).expect("an mbuf");
        assert_eq!(
            l.usrreqs().sockaddr(&l, &mut solock_shared(&l), &mut nam),
            Ok(())
        );
        assert_eq!(&nam.data()[2..7], b"/sock");
        assert_eq!(nam.data()[0] as usize, size_of::<SockaddrUn>());
        assert_eq!(
            soconnect(&other, &mut w, &sun(b"/nonexistent")),
            Err(Errno::ENOENT)
        );
        drop(w);
        let (d, fd) = sock(p, SOCK_DGRAM);
        let mut wd = solock_shared(&d);
        assert_eq!(
            soconnect(&d, &mut wd, &sun(b"/sock")),
            Err(Errno::EPROTOTYPE)
        );
        drop(wd);

        // connect(2): a new socket on the listener's queue, connected to the client.
        let (c, fc) = sock(p, SOCK_STREAM);
        let mut wc = solock_shared(&c);
        assert_eq!(soconnect(&c, &mut wc, &sun(b"/sock")), Ok(()));
        drop(wc);
        assert_eq!(l.qlens(), (0, 1));
        assert!(c.has_state(SS_ISCONNECTED));

        // accept(2): off the queue, the peer's name (the client is unbound: sun_noname).
        let (so3, r) = soaccept_take(&l, solock_shared(&l), true, |so, w| {
            soaccept(so, w, &mut nam)
        })
        .expect("a connection");
        assert_eq!(r, Ok(()));
        assert_eq!(nam.data(), &SUN_NONAME[..]);
        let f3 = file(p, DTYPE_SOCKET, Some(so3.clone()));
        // The client's peer is named after the listener.
        assert_eq!(
            c.usrreqs().peeraddr(&c, &mut solock_shared(&c), &mut nam),
            Ok(())
        );
        assert_eq!(&nam.data()[2..7], b"/sock");
        drop(nam);

        assert_eq!(send(p, &c, b"ping", 0), Ok(4));
        assert_eq!(recv(p, &so3, 8, 0), Ok((b"ping".to_vec(), 0)));
        assert_eq!(send(p, &so3, b"pong", 0), Ok(4));
        assert_eq!(recv(p, &c, 8, 0), Ok((b"pong".to_vec(), 0)));

        // The peer's credentials: the connector's, the listener's on the client.
        let mut m = m_get(M_WAIT, MT_SOOPTS).expect("an mbuf");
        assert_eq!(sogetopt(&c, SOL_SOCKET, SO_PEERCRED, &mut m), Ok(()));
        assert_eq!(
            pod_from_bytes::<Sockpeercred>(m.data()),
            Some(peercred_of(p))
        );
        drop(m);

        // A connection still on the queue is aborted with its listener.
        let (c2, fc2) = sock(p, SOCK_STREAM);
        let mut w2 = solock_shared(&c2);
        assert_eq!(soconnect(&c2, &mut w2, &sun(b"/sock")), Ok(()));
        drop(w2);
        assert_eq!(l.qlens(), (0, 1));
        drop(l);
        assert_eq!(sys_close(p, fl), Ok(()));
        assert!(!c2.has_state(SS_ISCONNECTED));
        assert_eq!(recv(p, &c2, 8, 0), Ok((vec![], 0)));

        // The listener's vnode lost its socket: a connection is refused.
        let (c3, fc3) = sock(p, SOCK_STREAM);
        let mut w3 = solock_shared(&c3);
        assert_eq!(
            soconnect(&c3, &mut w3, &sun(b"/sock")),
            Err(Errno::ECONNREFUSED)
        );
        drop(w3);

        drop((other, d, c, so3, c2, c3));
        for f in [fo, fd, fc, f3, fc2, fc3] {
            assert_eq!(sys_close(p, f), Ok(()));
        }
        assert_eq!((unpcbs(), sockets()), before);
        teardown();
    }

    #[test]
    fn a_datagram_socket_connects_to_a_bound_one_and_sends_to_it() {
        let (_g, p) = setup();
        let (srv, fs) = sock(p, SOCK_DGRAM);
        let mut w = solock_shared(&srv);
        assert_eq!(sobind(&srv, &mut w, &sun(b"/dg"), p), Ok(()));
        drop(w);
        let (cl, fcl) = sock(p, SOCK_DGRAM);

        // sendto(2): connected for the send only.
        let nam = sun(b"/dg");
        UserPtr::<u8>::new(USEND)
            .copyout_slice(b"hi")
            .expect("user memory");
        let mut iov = [Iovec {
            iov_base: USEND as *mut c_void,
            iov_len: 2,
        }];
        let mut u = uio(p, &mut iov, UioRw::UIO_WRITE);
        assert_eq!(sosend(&cl, Some(&nam), Some(&mut u), None, None, 0), Ok(()));
        assert!(!cl.has_state(SS_ISCONNECTED));
        let (bytes, _, _, from) = recvc(p, &srv, 8, 0, 0).expect("a datagram");
        assert_eq!(bytes, b"hi");
        assert_eq!(from.as_deref().map(Mbuf::data), Some(&SUN_NONAME[..]));

        // connect(2), then a send to an address is EISCONN.
        let mut w = solock_shared(&cl);
        assert_eq!(soconnect(&cl, &mut w, &nam), Ok(()));
        drop(w);
        assert!(cl.has_state(SS_ISCONNECTED));
        let mut iov = [Iovec {
            iov_base: USEND as *mut c_void,
            iov_len: 2,
        }];
        let mut u = uio(p, &mut iov, UioRw::UIO_WRITE);
        assert_eq!(
            sosend(&cl, Some(&nam), Some(&mut u), None, None, 0),
            Err(Errno::EISCONN)
        );
        drop(nam);

        // The bound socket closed: its referencing socket loses its connection.
        drop(srv);
        assert_eq!(sys_close(p, fs), Ok(()));
        assert!(!cl.has_state(SS_ISCONNECTED));
        assert_eq!(cl.error(), Some(Errno::ECONNRESET));
        drop(cl);
        assert_eq!(sys_close(p, fcl), Ok(()));
        teardown();
    }

    #[test]
    fn a_descriptor_passes_through_scm_rights() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_STREAM);
        let (x, fx) = sock(p, SOCK_DGRAM);
        let xfile = fd_getfile(p.fd(), fx).expect("the file");
        let count = xfile.count();

        // In flight: one reference more, one right, the socket's message count and its
        // control block's reference to its file.
        assert_eq!(sendc(p, &a, b"m", Some(rights(&[fx])), 0), Ok(1));
        assert_eq!(tables::<G>().rights(), 1);
        assert_eq!(xfile.count(), count + 2);
        assert_eq!(unp_of(&x).msgcount(), 1);
        // kern.file's f_count is the C's: the message's reference, not the block's.
        assert_eq!(unp_file_refs::<G>(&xfile), 1);
        assert_eq!(xfile.count() - unp_file_refs::<G>(&xfile), count + 1);
        assert_eq!(unp_file_refs::<G>(&fd_getfile(p.fd(), fa).expect("a")), 0);

        // Received: a new descriptor of the same file; nothing in flight.
        let (bytes, _, ctl, _) = recvc(p, &b, 8, 0, cmsg_space(4) as u32).expect("the message");
        assert_eq!(bytes, b"m");
        let ctl = ctl.expect("the control message");
        let fds = fds_of(&ctl);
        assert_eq!(fds.len(), 1);
        assert_ne!(fds[0], fx);
        let got = fd_getfile(p.fd(), fds[0]).expect("the new descriptor");
        assert!(FileRef::ptr_eq(&got, &xfile));
        drop(got);
        assert_eq!(tables::<G>().rights(), 0);
        assert_eq!(unp_of(&x).msgcount(), 0);
        assert_eq!(xfile.count(), count + 1);
        assert_eq!(unp_file_refs::<G>(&xfile), 0);
        drop((ctl, xfile));

        drop((a, b, x));
        for f in [fa, fb, fx, fds[0]] {
            assert_eq!(sys_close(p, f), Ok(()));
        }
        teardown();
    }

    #[test]
    fn many_descriptors_grow_the_message_and_the_table() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_STREAM);
        let (x, fx) = sock(p, SOCK_STREAM);

        // 13 descriptors outgrow MLEN (a cluster); 127 fill a cluster; 128 are E2BIG.
        for (n, want) in [
            (12, Ok(1)),
            (13, Ok(1)),
            (127, Ok(1)),
            (128, Err(Errno::E2BIG)),
        ] {
            let fds = vec![fx; n];
            assert_eq!(
                sendc(p, &a, b"m", Some(rights(&fds)), 0),
                want,
                "{n} descriptors"
            );
            if want.is_ok() {
                assert_eq!(tables::<G>().rights(), n as i32);
                let (_, _, ctl, _) =
                    recvc(p, &b, 8, 0, cmsg_space(4 * n) as u32).expect("the message");
                let got = fds_of(&ctl.expect("the control message"));
                assert_eq!(got.len(), n);
                for f in got {
                    assert_eq!(sys_close(p, f), Ok(()));
                }
            }
            assert_eq!(tables::<G>().rights(), 0);
        }
        assert!(tables::<G>().inflight.lock().len() >= 127);

        drop((a, b, x));
        for f in [fa, fb, fx] {
            assert_eq!(sys_close(p, f), Ok(()));
        }
        teardown();
    }

    #[test]
    fn internalize_refuses_as_the_c_does_and_backs_out() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_STREAM);
        let (x, fx) = sock(p, SOCK_STREAM);
        let xfile = fd_getfile(p.fd(), fx).expect("the file");
        let count = xfile.count();
        let kq = file(p, DTYPE_KQUEUE, None);

        // A bad descriptor after a good one: EBADF, the good one's reference and count
        // given back (the back-out), nothing in flight.
        let cases: [(&[i32], Errno); 3] = [
            (&[fx, 1000], Errno::EBADF),
            (&[1000, fx], Errno::EBADF),
            (&[fx, kq], Errno::EINVAL),
        ];
        for (fds, e) in cases {
            assert_eq!(sendc(p, &a, b"m", Some(rights(fds)), 0), Err(e), "{fds:?}");
            assert_eq!(tables::<G>().rights(), 0);
            assert_eq!(xfile.count(), count);
            assert_eq!(unp_of(&x).msgcount(), 0);
        }

        // A message that is not SCM_RIGHTS, or whose length disagrees with the mbuf's.
        let mut m = rights(&[fx]);
        m.data_mut().expect("writable")[8] = 2;
        assert_eq!(sendc(p, &a, b"m", Some(m), 0), Err(Errno::EINVAL));
        let mut m = rights(&[fx]);
        m.data_mut().expect("writable")[0] = 8;
        assert_eq!(sendc(p, &a, b"m", Some(m), 0), Err(Errno::EINVAL));

        // pledge_sendfd refuses: its error, nothing in flight.
        testutil::set_pledge_fd(Some(Errno::EPERM));
        assert_eq!(
            sendc(p, &a, b"m", Some(rights(&[fx])), 0),
            Err(Errno::EPERM)
        );
        testutil::set_pledge_fd(None);
        assert_eq!(tables::<G>().rights(), 0);
        assert_eq!(xfile.count(), count);

        // Past maxfiles / 10 in flight: EMFILE (the test glue's maxfiles is 4096).
        let fds = vec![fx; 50];
        for _ in 0..8 {
            assert_eq!(sendc(p, &a, b"m", Some(rights(&fds)), 0), Ok(1));
        }
        assert_eq!(tables::<G>().rights(), 400);
        assert_eq!(
            sendc(p, &a, b"m", Some(rights(&fds[..10])), 0),
            Err(Errno::EMFILE)
        );
        assert_eq!(sendc(p, &a, b"m", Some(rights(&fds[..9])), 0), Ok(1));
        assert_eq!(tables::<G>().rights(), 409);

        // The receiver closed: every right goes to the collector, which closes them.
        assert_eq!(b.so_rcv.datacc(), 9);
        drop(b);
        assert_eq!(sys_close(p, fb), Ok(()));
        collect(0);
        assert_eq!(xfile.count(), count);
        assert_eq!(unp_of(&x).msgcount(), 0);
        drop(xfile);

        drop((a, x));
        for f in [fa, fx, kq] {
            assert_eq!(sys_close(p, f), Ok(()));
        }
        teardown();
    }

    #[test]
    fn externalize_refuses_and_hands_the_files_to_the_collector() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_STREAM);
        let (x, fx) = sock(p, SOCK_STREAM);
        let xfile = fd_getfile(p.fd(), fx).expect("the file");
        let count = xfile.count();

        // A control buffer too short for the descriptors (room for one int after the
        // header): EMSGSIZE, the files deferred.
        assert_eq!(sendc(p, &a, b"m", Some(rights(&[fx, fx])), 0), Ok(1));
        assert_eq!(
            recvc(p, &b, 8, 0, cmsg_len(4) as u32).map(|_| ()),
            Err(Errno::EMSGSIZE)
        );
        collect(0);
        assert_eq!(xfile.count(), count);
        // The control message went, its data stays for the next read (the C's soreceive).
        assert_eq!(recv(p, &b, 8, 0), Ok((b"m".to_vec(), 0)));

        // pledge_recvfd refuses: its error, the files deferred.
        assert_eq!(sendc(p, &a, b"m", Some(rights(&[fx])), 0), Ok(1));
        testutil::set_pledge_fd(Some(Errno::EPERM));
        assert_eq!(
            recvc(p, &b, 8, 0, cmsg_space(4) as u32).map(|_| ()),
            Err(Errno::EPERM)
        );
        testutil::set_pledge_fd(None);
        collect(0);
        assert_eq!(xfile.count(), count);
        assert_eq!(recv(p, &b, 8, 0), Ok((b"m".to_vec(), 0)));

        // A message read without asking for its control: the rights are disposed of.
        assert_eq!(sendc(p, &a, b"m", Some(rights(&[fx])), 0), Ok(1));
        assert_eq!(recv(p, &b, 8, 0), Ok((b"m".to_vec(), 0)));
        collect(0);
        assert_eq!(xfile.count(), count);
        drop(xfile);

        drop((a, b, x));
        for f in [fa, fb, fx] {
            assert_eq!(sys_close(p, f), Ok(()));
        }
        teardown();
    }

    #[test]
    fn a_cycle_of_sockets_is_collected() {
        let (_g, p) = setup();
        let before = (unpcbs(), sockets());
        let ((a, fa), (b, fb)) = pair(p, SOCK_STREAM);

        // a in b's buffer, b in a's buffer, then both descriptors closed: only the messages
        // hold them.
        assert_eq!(sendc(p, &a, b"m", Some(rights(&[fa])), 0), Ok(1));
        assert_eq!(sendc(p, &b, b"m", Some(rights(&[fb])), 0), Ok(1));
        drop((a, b));
        assert_eq!(sys_close(p, fa), Ok(()));
        assert_eq!(sys_close(p, fb), Ok(()));
        assert_eq!(unpcbs(), before.0 + 2);

        // The first run finds them dead and discards their buffers; the second closes the
        // discarded files, which frees both sockets.
        collect(0);
        assert_eq!((unpcbs(), sockets()), before);
        teardown();
    }

    #[test]
    fn a_live_socket_in_a_dead_buffer_stays() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, SOCK_STREAM);
        let (x, fx) = sock(p, SOCK_STREAM);

        // x's descriptor stays open: x is not dead, and nothing is discarded.
        assert_eq!(sendc(p, &a, b"m", Some(rights(&[fx])), 0), Ok(1));
        collect(1);
        assert_eq!(unp_of(&x).msgcount(), 1);
        let (_, _, ctl, _) = recvc(p, &b, 8, 0, cmsg_space(4) as u32).expect("the message");
        let got = fds_of(&ctl.expect("the control message"));

        drop((a, b, x));
        for f in [fa, fb, fx, got[0]] {
            assert_eq!(sys_close(p, f), Ok(()));
        }
        teardown();
    }

    #[test]
    fn nam2sun_checks_the_address_as_the_c_does() {
        let (_g, _p) = setup();
        let full = [b'x'; 104];
        // (sun_len, family, path, m_len, answer)
        let cases: [(u8, u8, &[u8], usize, Result<usize, Errno>); 8] = [
            (7, AF_UNIX, b"/a\0", 1, Err(Errno::EINVAL)),
            (7, 2, b"/abc\0", 7, Err(Errno::EAFNOSUPPORT)),
            (8, AF_UNIX, b"/abc\0", 7, Err(Errno::EINVAL)),
            (107, AF_UNIX, b"/abc\0", 107, Err(Errno::EINVAL)),
            (106, AF_UNIX, &full, 106, Err(Errno::EINVAL)),
            (7, AF_UNIX, b"/abc\0", 7, Ok(4)),
            // No NUL within sun_len: one is appended where the mbuf has room.
            (6, AF_UNIX, b"/abc", 6, Ok(4)),
            (2, AF_UNIX, b"", 2, Ok(0)),
        ];
        for (sun_len, family, path, m_len, want) in cases {
            let mut m = m_get(M_WAIT, MT_SONAME).expect("an mbuf");
            m.m_len().set(MLEN as u32);
            let d = m.data_mut().expect("writable");
            d.fill(0);
            d[0] = sun_len;
            d[1] = family;
            d[2..2 + path.len()].copy_from_slice(path);
            m.m_len().set(m_len as u32);
            let got = unp_nam2sun(&m).map(|(buf, len)| {
                assert_eq!(buf[len], 0);
                len
            });
            assert_eq!(got, want, "sun_len {sun_len}, m_len {m_len}");
        }
        // A path that fills the mbuf to its end has no room for its NUL.
        let mut m = m_get(M_WAIT, MT_SONAME).expect("an mbuf");
        let off = MLEN - 6;
        m.set_data_off(off);
        m.m_len().set(6);
        let d = m.data_mut().expect("writable");
        d.copy_from_slice(&[6, AF_UNIX, b'/', b'a', b'b', b'c']);
        assert_eq!(unp_nam2sun(&m).map(|(_, l)| l), Err(Errno::EINVAL));
    }

    #[test]
    fn net_unix_reads_and_bounds_its_variables() {
        let (_g, _p) = setup();
        let at = USER_BASE + 0x2_0000;
        let mut len = 4;
        let mut req = SysctlReq {
            oldp: at,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            uipc_sysctl::<G>(&[SOCK_STREAM, UNPCTL_SENDSPACE], &mut req),
            Ok(())
        );
        assert_eq!(user_bytes(at, 4), Some(PIPSIZ.to_ne_bytes().to_vec()));
        assert_eq!(
            uipc_sysctl::<G>(&[SOCK_DGRAM, UNPCTL_SENDSPACE], &mut req),
            Ok(())
        );
        assert_eq!(user_bytes(at, 4), Some(8192i32.to_ne_bytes().to_vec()));
        assert_eq!(uipc_sysctl::<G>(&[NET_UNIX_INFLIGHT], &mut req), Ok(()));
        assert_eq!(user_bytes(at, 4), Some(0i32.to_ne_bytes().to_vec()));
        assert_eq!(
            uipc_sysctl::<G>(&[SOCK_STREAM], &mut req),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            uipc_sysctl::<G>(&[NET_UNIX_DEFERRED, 1], &mut req),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(uipc_sysctl::<G>(&[99], &mut req), Err(Errno::ENOPROTOOPT));

        // A write past SB_MAX is refused; the in-flight count is read-only.
        UserPtr::<i32>::new(at + 8)
            .copyout(&((SB_MAX as i32) + 1))
            .expect("user memory");
        let mut len = 4;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: at + 8,
            newlen: 4,
        };
        assert_eq!(
            uipc_sysctl::<G>(&[SOCK_STREAM, UNPCTL_RECVSPACE], &mut req),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            uipc_sysctl::<G>(&[NET_UNIX_INFLIGHT], &mut req),
            Err(Errno::EPERM)
        );
        assert_eq!(UNPST_RECVSPACE.load(Ordering::Relaxed), PIPSIZ);
    }

    #[test]
    fn the_pool_the_sense_and_the_fdpass_entry_are_the_c_s() {
        let (_g, p) = setup();
        assert_eq!(size_of::<Fdpass>(), 16);
        assert!(testutil::pool_knobs("unpcb").is_some());
        // The control block's Rust layout and the pool's header (a deviation: the C's is 144).
        assert_eq!(tables::<G>().pool().item_size(), Some(376));

        let (a, fa) = sock(p, SOCK_STREAM);
        let mut st = Stat::default();
        let mut w = solock_shared(&a);
        assert_eq!(a.usrreqs().sense(&a, &mut w, &mut st), Ok(()));
        let ino = st.st_ino;
        assert_ne!(ino, 0);
        assert_eq!(st.st_dev, -1);
        assert_eq!(st.st_blksize as i32, PIPSIZ);
        assert_eq!(a.usrreqs().sense(&a, &mut w, &mut st), Ok(()));
        assert_eq!(st.st_ino, ino);
        drop(w);
        drop(a);
        assert_eq!(sys_close(p, fa), Ok(()));
        teardown();
    }
}
/* </TESTS> */
