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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)uipc_socket.c	8.3 (Berkeley) 4/15/94
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)uipc_socket2.c	8.1 (Berkeley) 6/10/93
 */

/*-
 * Copyright (c) 1982, 1986, 1990, 1993
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
 *	@(#)socketvar.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! The socket: `struct socket` (`<sys/socketvar.h>`), its lock and connection queue, and
//! the socket operations (`kern/uipc_socket.c`, the state routines of
//! `kern/uipc_socket2.c`).
//!
//! Upstream: sys/kern/uipc_socket.c @ 3ce1f3f79392
//! Upstream: sys/kern/uipc_socket2.c @ 3ce1f3f79392
//! Upstream: sys/sys/socketvar.h @ 3ce1f3f79392
//! LZ: sys/kern/uipc_socket.rs@3c62ede68802
//! LZ: sys/kern/uipc_socket2.rs@3c62ede68802
//! LZ: sys/sys/socketvar.rs@3c62ede68802
//!
//! These routines are called by the socket file's operations and the system calls, or from
//! a system process, and implement the semantics of socket operations by switching out to
//! the protocol specific routines ([`Usrreqs`]). `socreate` allocates a socket from
//! `sockpl` and attaches it to its protocol; `soclose` and `sofree` take it apart again, the
//! last reference giving it back to the pool. `sosend` copies user data into mbufs and hands
//! them to the protocol; `soreceive` takes records off the receive buffer (address, control,
//! data) and copies them out. Socket splicing (`SOCKET_SPLICE`, configured in GENERIC)
//! moves data from one socket's receive buffer to another's send buffer in the kernel:
//! `sosplice`, `somove`, `sounsplice`.
//!
//! The state routines (`soisconnecting`, `soisconnected`, `soisdisconnecting`,
//! `soisdisconnected`) move a socket through its connection states and wake whoever waits.
//! Normal sequence from the active (originating) side is that `soisconnecting()` is called
//! during processing of `connect()` call, resulting in an eventual call to
//! `soisconnected()` if/when the connection is established. When the connection is torn down
//! `soisdisconnecting()` is called during processing of `disconnect()` call, and
//! `soisdisconnected()` is called when the connection to the peer is totally severed.
//! Connectionless protocols can call `soisconnected()` and `soisdisconnected()` only,
//! bypassing the in-progress calls when setting up a "connection" takes no time. From the
//! passive side, a socket is created with two queues of sockets: `so_q0` for connections in
//! progress and `so_q` for connections already made and awaiting user acceptance. As a
//! protocol is preparing incoming connections, it creates a socket structure queued on
//! `so_q0` by calling `sonewconn()`. When the connection is established, `soisconnected()`
//! is called, and transfers the socket structure to `so_q`, making it available to
//! `accept()`. If a socket is closed with sockets on either `so_q0` or `so_q`, these sockets
//! are dropped.
//!
//! Locks used to protect struct members: \[I\] immutable after creation, \[a\] atomic,
//! \[mr\] `sb_mtx` of `so_rcv`, \[ms\] `sb_mtx` of `so_snd`, \[br\] `sblock()` of `so_rcv`,
//! \[bs\] `sblock()` of `so_snd`, \[s\] `solock()`, \[q\] the connection queue's mutex.
//!
//! ## Deviations
//! - `struct socket **aso` and the other out-parameters are return values.
//! - `WITNESS` is not configured (`soalloc`'s `inet46` lock name); `DIAGNOSTIC`'s panics are
//!   under feature `diagnostic`.
//! - `accept(2)` finding its socket gone from the queue it peeked retries where the C's
//!   `soqremque` failure is `panic("accept")` (the queue's mutex is not the socket lock the
//!   C's peek held).
//! - `sockpl`'s item is the socket's Rust layout (984 bytes with the pool's header, larger
//!   than the C's `struct socket`): the counted item's header, the two
//!   buffers' owned records and atomics, the queue's mutex and pinned lists. `vmstat -m`
//!   shows that size under the C's pool name; the row and the name stay (the coordinator's
//!   decision of 2026-11-08, item 16: pool item sizes are a diagnostic-only detail; the
//!   N12 performance work keeps the layout as small as the shape allows).
//!
//! ## Redesign
//! - LZ's `struct socket` was `Cell`s and raw pointers in a `socket_pool` item handed
//!   around as `&'static Socket`, freed by `sorele`'s count. Here [`Socket`] is a counted
//!   item of the glue's `sockpl` ([`SocketRef`], a `hal::mem::pool::PoolArc`): the C's
//!   `so_refcnt` is the handle's count, `soref` is `PoolArc::from_ref`, `sorele` a drop, and
//!   `sorele`'s body the socket's `Drop` (U13). The C's existence reference (`soalloc`'s
//!   one, dropped by `sofree`) is a reference parked on the item (`PoolArc::forget`) that
//!   `sofree` adopts once, when it decommissions the socket.
//! - `solock()` is [`SoLocked`], the witness of the socket's lock (the net lock for the
//!   internet domains, exclusive, or shared with `so_lock`; `so_lock` for the others). The
//!   `[s]` members are atomics written under it (`so_state`, `so_options`, `so_linger`,
//!   `so_timeo`, `so_qlimit`; read where the C reads them unlocked or with `READ_ONCE`),
//!   the protocol control block a read-mostly slot (`so_pcb`, read without the lock by the
//!   UNIX domain's collector as the C reads it) beside the word `bsd`'s protocols keep
//!   (`legacy_pcb`), and the splice state a set-once box. No `Guarded` cell is keyed to two
//!   locks (D-N6.2-U1, shape B).
//! - N6.6a0 (D-1): the socket lock's form is per protocol ([`Protocol::legacy`], recorded
//!   at `soalloc` for the socket's life). A `bsd` protocol's socket is held by detached
//!   holdings of `so_lock` (an internet one by the net lock alone for `solock`), as before. A
//!   kernel protocol's socket is held by a witness of `so_lock`, an internet one's under
//!   either holding of the net lock: [`solock`] takes `so_lock` after the exclusive net lock
//!   where the C takes the net lock alone (`solock`, uipc_socket2.c:324-336), so the `[s]`
//!   cells of the socket, the pcb, the tcpcb and the SYN cache entries key to one lock and
//!   open in parts at once ([`SoLocked::inet`], [`InetLocked`], [`so_part`]). The extra
//!   holding never waits: every other holder of that `so_lock` holds the net lock shared
//!   (`in_pcbsolock`, [`solock_shared`], [`solock_nonet`]'s callers), which the exclusive
//!   holding excludes. Order: the net lock, then `so_lock`, a listener's before its child's;
//!   nothing takes them the other way. `sosleep_nsec`'s exclusive case lets `so_lock` go
//!   around the sleep as the shared case does (a sleeper keeping it would block its own
//!   waker). `bsd`'s C-style callers lend their proof of the net lock ([`solocked_legacy`]):
//!   a kernel protocol's socket gets their detached holding attached for the request
//!   (`hal::sync::rwlock::DetachableRwlock::attach`) or `so_lock` taken for it, and the lent net
//!   lock is never let go (a wait or an unlock under it is `panic(9)`; none of `bsd`'s six
//!   callers waits). Cost against the C: one uncontended `so_lock` pair per exclusive-lock
//!   request of a kernel internet socket (`setsockopt`, `getsockopt`, `soclose`'s steps),
//!   nothing per packet. No internet protocol is the kernel's before N6.6a5 (TCP), so no
//!   socket changes its lock at N6.6a0.
//! - The connection queues (`so_q0`, `so_q`, `so_head`, `so_onq`, the lengths) are under one
//!   leaf mutex per socket, `cq` (D-N6.2-1): the listener's `q0`/`q` (pinned lists of the
//!   children, keyed to the listener's `cq`), the child's `head` (counted: the C's raw
//!   `so_head`) and `onq`. Lock order: a socket lock, then the listener's `cq`, then the
//!   child's; nothing sleeps under a `cq`; `cq` is also the interlock of a listener's
//!   `so_timeo` wait channel (`accept`'s; every wakeup of a listener's channel is made under
//!   it, a connection's under its socket lock alone, as the C's). The C's re-lock dances
//!   (`soref(head); sounlock(so); solock(head); solock(so)` in `soisconnected` and
//!   `sofree`) become one or two mutex sections, and `bsd`'s callbacks need no witness of
//!   the listener. A queued child holds one parked reference for the queue
//!   (`PoolArc::forget` at insert, `PoolArc::adopt` at removal, exactly once each).
//! - `so_upcall` is not a trait (decision 46, D-N6.2-3): one registered function per glue
//!   ([`NetTables`]), the C's `so_upcall` word an atomic kind, and `so_upcallarg` a counted
//!   handle with its type erased (N6.7b4, D-10: nfs's server socket record, an `MArc`), in a
//!   read-mostly slot: `sorwakeup` tests the kind (one load, as the C's test of
//!   `so_upcall`), then clones the handle out of the slot (a reader pair and a count pair)
//!   and calls the function with it outside the slot, so the record outlives the call
//!   whatever `nfsrv_zapsock` does meanwhile, and a writer never waits on a reader that waits
//!   for the kernel lock its writer holds.
//! - Cost against the C: the internet domains' socket events take one leaf mutex more per
//!   `sonewconn`, `soisconnected` and accept (`cq`); the UNIX domain's promotion and `sofree`
//!   of a queued socket take 1-2 mutex sections where the C takes 3 rwlock operations and
//!   2 reference counts. The lock and `spl` transitions of `sosend`, `soreceive` and the
//!   buffer routines are the C's, step by step (`sb_mtx` sections, `solock_shared` around
//!   `pru_send`/`pru_rcvd`). `socreate` and `sonewconn` store a fresh socket's buffer
//!   timeouts and watermarks without its buffers' mutexes (the C's plain stores: nobody else
//!   reaches it yet), `sofree` takes `cq` only for a queued socket (the `queued` mirror), the
//!   last reference purges the receive buffer through its `&mut` (the C's unlocked
//!   `m_purge`), and `so_timeo_wakeup` takes `cq` for a listener only: a UNIX socket's
//!   `socket(2)` and `close(2)` take the C's mutex and rwlock sections, no more (N6.2a's
//!   decision-43 rows, `benches/net.rs`). A `MSG_PEEK` of 4 KiB costs 17 ns more than LZ's
//!   in release (72.3 -> 89.3 ns, the rows of 2026-11-08 15:44), not attributed: the work
//!   around the copy is LZ's (`soreceive.peek.1` 36.8 -> 36.9 ns; `sockbuf.record.out.in`,
//!   the record taken out and put back with its mutex section, 10.8 ns), both trees copy
//!   one mbuf, its data at 0 mod 64, into a destination at 0 mod 64 (one LZ run at 48)
//!   through the same `machine::kcopy_out`, and still the length-dependent part differs;
//!   a decision-43 item for the user, N12's row.
//! - N6.6a5 (the review's S3, the user's decision): `so_pcb` of a kernel internet protocol
//!   has two views. The counted slot (`pcb`, a `ReadMostly`) stays for the readers that hold
//!   no socket lock (`fptounp`, `kern.file`'s by-pid walk), and the requests read
//!   [`Socket::pcb_locked`], a `Guarded` copy keyed to `so_lock` in part [`so_part::PCB`],
//!   borrowed for as long as the request's witness part is: the C's load of `so_pcb`, with no
//!   count (`sotoinpcb_locked`), where `sotoinpcb`'s counted handle cost the reader pair of
//!   the slot and the pcb's count pair on every `write(2)` and `read(2)`. The copy is keyed
//!   at its first write (a `OnceStatic`, since the socket is built before it is placed in its
//!   pool item, where `so_lock`'s identity is known), filled by TCP's attach and emptied by
//!   `tcp_free` (the pcb and the socket count each other until then), both under the lock.

use core::mem;
use core::sync::atomic::{
    AtomicBool, AtomicI16, AtomicI32, AtomicI64, AtomicU8, AtomicU32, AtomicU64, AtomicUsize,
    Ordering,
};

use hal::cpu::intr::IPL_SOFTNET;
use hal::ds::pinlist::{PinEntry, PinList};
use hal::mem::pool::{AnyArc, PoolArc, PoolBox, PoolHome};
use hal::sync::mutex::{Mutexed, MutexedGuard};
use hal::sync::netlock::{NETLOCK, NetHeld};
use hal::sync::once::OnceStatic;
use hal::sync::rwlock::{Divisible, Guarded, ReadWitness, WitnessPart, WriteWitness};
use hal::sync::sleep::{INFSLP, PCATCH, wakeup, wakeup_one};
use hal::sync::slot::ReadMostly;
use sysdefs::errno::Errno;
use sysdefs::socket::{PF_INET, PF_INET6, SO_ACCEPTCONN, SOMAXCONN};
use sysdefs::types::Off;

use crate::assert::{kassert, kpanic};
use crate::base::task::Task;
use crate::net::domain::net;
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::proto::{PR_SPLICE, Protocol, Usrreqs};
use crate::net::sockbuf::{
    SB_SPLICE, SS_CANTRCVMORE, SS_CANTSENDMORE, SS_ISCONNECTED, SS_ISCONNECTING, SS_ISDISCONNECTED,
    SS_ISDISCONNECTING, SS_NOFDREF, Sockbuf,
};
use crate::proc::{ProcGlue, Thread};
use crate::sig::sigio::{SigioRef, pgsigio, sigio_copy, sigio_free, sigio_init};
use crate::time::clock::Timeval;
use crate::time::timeout::Timeout;

/// `SOMINCONN`: the smallest backlog `listen(2)` grants.
pub const SOMINCONN: i32 = 80;

/// \[a\] `somaxconn`: the largest backlog `listen(2)` grants (`kern.somaxconn`).
pub static SOMAXCONN_VAR: AtomicI32 = AtomicI32::new(SOMAXCONN);
/// \[a\] `sominconn`: the smallest (`kern.sominconn`).
pub static SOMINCONN_VAR: AtomicI32 = AtomicI32::new(SOMINCONN);

/// A counted handle to a socket (`struct socket *` with a reference: `soref`/`sorele`).
pub type SocketRef<G> = PoolArc<Socket<G>>;

/// `so_upcall`: the function a socket's upcall runs (`sorwakeup`), with the argument it was
/// set with (`so_upcallarg`, a counted handle the function downcasts) and `M_DONTWAIT`.
pub type UpcallFn<G> = fn(&Socket<G>, &AnyArc, i32);

/// `struct splice`: structure used for manipulating splice option (`SO_SPLICE`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Splice {
    /// Drain socket file descriptor.
    pub sp_fd: i32,
    /// The C's padding before `sp_max`.
    pub sp_pad: i32,
    /// If set, maximum bytes to splice.
    pub sp_max: Off,
    /// Idle timeout.
    pub sp_idle: Timeval,
}

sysdefs::abi_pod!(Splice {
    sp_fd: i32,
    sp_pad: i32,
    sp_max: Off,
    sp_idle: Timeval,
});

const _: () = assert!(size_of::<Splice>() == 32);

/// `so_onq`: which of its listener's queues a socket is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Onq {
    /// On none.
    None,
    /// `so_q0`: a partial connection.
    Q0,
    /// `so_q`: a connection ready to be accepted.
    Q,
}

/// The connection queue's state of a socket, under its `cq` mutex: as a listener the
/// lengths of `so_q0` and `so_q` (the lists are the socket's `q0` and `q`, keyed to this
/// mutex), as a queued child its listener and which queue it is on.
pub struct ConnQueue<G: ProcGlue> {
    /// \[q\] `so_q0len`: partials on `so_q0`.
    q0len: i16,
    /// \[q\] `so_qlen`: number of connections on `so_q`.
    qlen: i16,
    /// \[q\] `so_head`: back pointer to accept socket (counted).
    head: Option<SocketRef<G>>,
    /// \[q\] `so_onq`: queue (q or q0) that we're on.
    onq: Onq,
}

hal::pin_adapter!(
    /// `so_qe`: a queued socket on its listener's `so_q0` or `so_q`.
    impl[G: ProcGlue] pub SoQe<G>: Socket<G>, qe
);

/// `struct sosplice`: variables for socket splicing, allocated only when needed (`sosppl`).
pub struct Sosplice<G: ProcGlue> {
    /// \[mr ms\] `ssp_socket`: send data to drain socket; keyed to this socket's `so_rcv`
    /// mutex (the drain's `so_snd` mutex is held too where it changes).
    socket: Guarded<Option<SocketRef<G>>>,
    /// \[mr ms\] `ssp_soback`: back ref to source socket; keyed to this socket's `so_snd`
    /// mutex.
    soback: Guarded<Option<SocketRef<G>>>,
    /// \[mr\] `ssp_len`: number of bytes spliced (read by `sogetopt` under the socket lock).
    len: AtomicI64,
    /// \[I\] `ssp_max`: maximum number of bytes.
    max: AtomicI64,
    /// \[I\] `ssp_idletv`: idle timeout, seconds.
    idle_sec: AtomicI64,
    /// \[I\] `ssp_idletv`: idle timeout, microseconds.
    idle_usec: AtomicI64,
    /// `ssp_idleto`.
    idleto: Timeout,
    /// `ssp_task`: task for somove.
    task: Task,
    /// \[I\] `ssp_queue`: softnet queue where we add (set by each `sosplice`).
    queue: hal::ds::link::SyncLink<crate::base::task::Taskq>,
}

/// `struct socket`: kernel structure per socket. Contains send and receive buffer queues,
/// handle on protocol and pointer to protocol private data and error information.
pub struct Socket<G: ProcGlue> {
    /// The item's place in `sockpl` (`soref` of a `&Socket`).
    home: PoolHome<Self>,
    /// \[I\] `so_proto`: protocol handle.
    proto: &'static dyn Protocol<G>,
    /// \[I\] `so_proto->pr_domain->dom_family`, read by every lock.
    family: i32,
    /// \[I\] The protocol is `bsd`'s: `so_lock` is held by detached holdings
    /// ([`Protocol::legacy`]); a kernel protocol's socket is held by a witness of it, an
    /// internet one's under either holding of the net lock (D-1).
    legacy_lock: bool,
    /// \[I\] `so_type`: generic type, see `socket.h`.
    so_type: i32,
    /// \[I\] `so_euid`: who opened the socket.
    euid: u32,
    /// \[I\] `so_ruid`.
    ruid: u32,
    /// \[I\] `so_egid`.
    egid: u32,
    /// \[I\] `so_rgid`.
    rgid: u32,
    /// \[I\] `so_cpid`: pid of process that opened socket.
    cpid: i32,
    /// `so_lock`: this socket lock (the UNIX domain's and the other per-socket domains'
    /// [`solock`]; taken with the shared net lock by `solock_shared` of an internet socket,
    /// and detached by `bsd`'s internet protocols).
    pub(crate) so_lock: hal::sync::rwlock::DetachableRwlock,
    /// \[s\] `so_state`: internal state flags `SS_*`.
    state: AtomicU32,
    /// \[s\] `so_options`: from socket call, see `socket.h`.
    options: AtomicI32,
    /// \[s\] `so_linger`: time to linger while closing.
    linger: AtomicI16,
    /// \[s\] `so_timeo`: connection timeout; its address is the channel of the connection,
    /// accept and close waits (under `cq`).
    timeo: AtomicI16,
    /// \[s\] `so_qlimit`: max number queued connections.
    qlimit: AtomicI16,
    /// \[q\] `so_qlen`, mirrored for its `READ_ONCE` readers (`filt_soread`, `tcp_input`'s
    /// check).
    qlen_mirror: AtomicI16,
    /// \[a\] `so_error`: error affecting connection (an errno number, 0 for none).
    error: AtomicU32,
    /// \[mr\] `so_oobmark`: chars to oob mark.
    oobmark: AtomicU64,
    /// The connection queue's state, and the interlock of `so_timeo`.
    cq: Mutexed<ConnQueue<G>>,
    /// \[q\] `so_q0`: queue of partial connections (under this socket's `cq`).
    q0: PinList<SoQe<G>>,
    /// \[q\] `so_q`: queue of incoming connections (under this socket's `cq`).
    q: PinList<SoQe<G>>,
    /// \[q\] `so_qe`: our queue entry (q or q0), under the listener's `cq`.
    qe: PinEntry<Self>,
    /// `so_sigio`: async I/O registration.
    sigio: SigioRef,
    /// \[s\] `so_pcb` of the kernel's protocols: their control block, counted and erased
    /// (the C's `void *`; the UNIX domain's `Unpcb`).
    pcb: ReadMostly<Option<AnyArc>>,
    /// \[s\] `so_pcb` of `bsd`'s protocols: the address of their control block (`inp`,
    /// `rop`, `kp`), which they set and read themselves.
    legacy_pcb: AtomicUsize,
    /// Whether `sofree` has decommissioned the socket (adopted the existence reference).
    freed: AtomicBool,
    /// Whether the socket is on a listener's queue (`so_onq != NULL`), written with
    /// `ConnQueue::onq` under `cq` and read without it by `sofree`, which takes `cq` only for
    /// a queued socket (the C reads `so_head` under the socket's lock alone): a socket is
    /// queued once, by `sonewconn` before anyone else reaches it, and moved from `so_q0` to
    /// `so_q` under its own lock, which `sofree`'s caller holds.
    queued: AtomicBool,
    /// \[s\] `so_pcb` of a kernel internet protocol as its requests read it
    /// ([`Socket::pcb_locked`]): the pcb, counted, in a cell keyed to `so_lock` in part
    /// [`so_part::PCB`], made at its first write ([`Socket::set_pcb_locked`]) once the
    /// socket is in its pool item (the lock's identity is its address there).
    pcb_locked: OnceStatic<Guarded<Option<AnyArc>>>,
    /// \[s br\] `so_sp`: splice state, allocated only when needed.
    sp: OnceStatic<PoolBox<Sosplice<G>>>,
    /// \[a\] `so_upcall`'s kind: 0 none, 1 the glue's registered function.
    upcall_kind: AtomicU8,
    /// \[a\] `so_upcallarg`: arg for above, counted (the slot's own exclusion).
    upcall_arg: ReadMostly<Option<AnyArc>>,
    /// `so_rcv`: the receive buffer.
    pub so_rcv: Sockbuf<G>,
    /// `so_snd`: the send buffer.
    pub so_snd: Sockbuf<G>,
}

hal::pool_home!(impl[G: ProcGlue] Socket<G>, home);

/// What `socreate` and `sonewconn` give a new socket before anything else sees it: the C's
/// assignments right after `soalloc`.
#[derive(Clone, Copy)]
pub struct SoInit {
    /// `so_type`.
    pub so_type: i32,
    /// `so_state`.
    pub state: u32,
    /// `so_options`.
    pub options: i32,
    /// `so_linger`.
    pub linger: i16,
    /// `so_timeo`.
    pub timeo: i16,
    /// `so_euid`.
    pub euid: u32,
    /// `so_ruid`.
    pub ruid: u32,
    /// `so_egid`.
    pub egid: u32,
    /// `so_rgid`.
    pub rgid: u32,
    /// `so_cpid`.
    pub cpid: i32,
}

impl<G: ProcGlue> Socket<G> {
    /// A new socket of protocol `prp` as `soalloc` and its caller's assignments leave it.
    fn new(prp: &'static dyn Protocol<G>, init: SoInit) -> Self {
        Self {
            home: PoolHome::new(),
            proto: prp,
            family: prp.domain().family(),
            legacy_lock: prp.legacy(),
            so_type: init.so_type,
            euid: init.euid,
            ruid: init.ruid,
            egid: init.egid,
            rgid: init.rgid,
            cpid: init.cpid,
            so_lock: hal::sync::rwlock::DetachableRwlock::new("socket"),
            state: AtomicU32::new(init.state),
            options: AtomicI32::new(init.options),
            linger: AtomicI16::new(init.linger),
            timeo: AtomicI16::new(init.timeo),
            qlimit: AtomicI16::new(0),
            qlen_mirror: AtomicI16::new(0),
            error: AtomicU32::new(0),
            oobmark: AtomicU64::new(0),
            cq: Mutexed::new(
                IPL_SOFTNET,
                ConnQueue {
                    q0len: 0,
                    qlen: 0,
                    head: None,
                    onq: Onq::None,
                },
            ),
            q0: PinList::new(),
            q: PinList::new(),
            qe: PinEntry::new(),
            sigio: SigioRef::new(),
            pcb: ReadMostly::new(None),
            legacy_pcb: AtomicUsize::new(0),
            freed: AtomicBool::new(false),
            queued: AtomicBool::new(false),
            pcb_locked: OnceStatic::new(),
            sp: OnceStatic::new(),
            upcall_kind: AtomicU8::new(0),
            upcall_arg: ReadMostly::new(None),
            so_rcv: Sockbuf::new(),
            so_snd: Sockbuf::new(),
        }
    }

    /// `so_proto`.
    #[inline]
    pub fn proto(&self) -> &'static dyn Protocol<G> {
        self.proto
    }

    /// `so->so_proto->pr_usrreqs`: set for every socket (`socreate` refuses a protocol
    /// without).
    #[inline]
    pub fn usrreqs(&self) -> &'static dyn Usrreqs<G> {
        match self.proto.usrreqs() {
            Some(u) => u,
            None => kpanic(format_args!("socket {self:p}: no pr_usrreqs")),
        }
    }

    /// `so->so_proto->pr_flags & flags`.
    #[inline]
    pub fn pr_flags(&self, flags: i16) -> bool {
        self.proto.flags() & flags != 0
    }

    /// `so->so_proto->pr_domain->dom_family`.
    #[inline]
    pub fn dom_family(&self) -> i32 {
        self.family
    }

    /// Whether the socket belongs to an internet domain, whose sockets the net lock
    /// serialises.
    #[inline]
    pub fn is_inet(&self) -> bool {
        self.family == i32::from(PF_INET) || self.family == i32::from(PF_INET6)
    }

    /// `so_type`.
    #[inline]
    pub fn so_type(&self) -> i32 {
        self.so_type
    }

    /// `so_euid`, `so_ruid`, `so_egid`, `so_rgid`, `so_cpid`.
    pub fn creds(&self) -> (u32, u32, u32, u32, i32) {
        (self.euid, self.ruid, self.egid, self.rgid, self.cpid)
    }

    /// `so_state`.
    #[inline]
    pub fn state(&self) -> u32 {
        self.state.load(Ordering::Relaxed)
    }

    /// `so->so_state & bits`.
    #[inline]
    pub fn has_state(&self, bits: u32) -> bool {
        self.state() & bits != 0
    }

    /// `so->so_state |= bits`, the socket locked (by the caller, or under the C's convention
    /// of the state routines `bsd`'s protocols call under their own locks).
    #[inline]
    pub(crate) fn st_set(&self, bits: u32) {
        self.state.fetch_or(bits, Ordering::Relaxed);
    }

    /// `so->so_state &= ~bits`, as [`Socket::st_set`].
    #[inline]
    pub(crate) fn st_clear(&self, bits: u32) {
        self.state.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `so->so_state |= bits` by `bsd`'s internet protocols, which hold the socket's lock by
    /// their own conventions (`udp_connect`, `tcp_usrreq`'s connect and disconnect).
    pub fn set_state_legacy(&self, bits: u32) {
        self.state.fetch_or(bits, Ordering::Relaxed);
    }

    /// `so->so_state &= ~bits` by `bsd`'s internet protocols.
    pub fn clear_state_legacy(&self, bits: u32) {
        self.state.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `so_lock`, for `bsd`'s C-style holdings of it (`solock`/`sounlock` of the domains
    /// still in `bsd`, `in_pcbsolock`, `sonewconn`'s returned socket): detached holdings
    /// only, which open no cell.
    #[inline]
    pub fn so_lock_legacy(&self) -> &hal::sync::rwlock::DetachableRwlock {
        &self.so_lock
    }

    /// `so->so_options |= bits` (or `&= ~bits`) by `bsd`'s protocols, which hold the
    /// socket's lock by their own conventions (`route_attach`'s `SO_USELOOPBACK`, nfs).
    pub fn set_options_legacy(&self, bits: i32, on: bool) {
        if on {
            self.options.fetch_or(bits, Ordering::Relaxed);
        } else {
            self.options.fetch_and(!bits, Ordering::Relaxed);
        }
    }

    /// `so->so_linger = v` by a protocol, the socket locked (`tcp_attach`'s `TCP_LINGERTIME`).
    pub(crate) fn set_linger(&self, v: i16) {
        self.linger.store(v, Ordering::Relaxed);
    }

    /// `so_options`.
    #[inline]
    pub fn options(&self) -> i32 {
        self.options.load(Ordering::Relaxed)
    }

    /// `so->so_options & bits`.
    #[inline]
    pub fn has_options(&self, bits: i32) -> bool {
        self.options() & bits != 0
    }

    /// `so->so_options |= bits` (or `&= ~bits` when not `on`), the socket locked.
    pub fn set_options(&self, _w: &SoLocked<'_>, bits: i32, on: bool) {
        if on {
            self.options.fetch_or(bits, Ordering::Relaxed);
        } else {
            self.options.fetch_and(!bits, Ordering::Relaxed);
        }
    }

    /// `so_linger`.
    #[inline]
    pub fn linger(&self) -> i16 {
        self.linger.load(Ordering::Relaxed)
    }

    /// `so_timeo`.
    #[inline]
    pub fn timeo(&self) -> i16 {
        self.timeo.load(Ordering::Relaxed)
    }

    /// `so_qlimit`.
    #[inline]
    pub fn qlimit(&self) -> i16 {
        self.qlimit.load(Ordering::Relaxed)
    }

    /// `READ_ONCE(so->so_qlen)`.
    #[inline]
    pub fn qlen(&self) -> i16 {
        self.qlen_mirror.load(Ordering::Relaxed)
    }

    /// `so_q0len` and `so_qlen`, read under the queue's mutex.
    pub fn qlens(&self) -> (i16, i16) {
        let g = self.cq.lock();
        (g.q0len, g.qlen)
    }

    /// `TAILQ_FIRST(&so->so_q)`: the first complete connection still on the listener's
    /// queue, counted (`bsd`'s protocol tests look at it before an `accept`).
    pub fn queued_first(&self) -> Option<SocketRef<G>> {
        let g = self.cq.lock();
        self.q.first(&g).map(soref)
    }

    /// `READ_ONCE(so->so_error)`.
    #[inline]
    pub fn error(&self) -> Option<Errno> {
        Errno::from_raw(self.error.load(Ordering::Relaxed) as i32)
    }

    /// The raw `so_error` (0 for none), for `SO_ERROR` and `kern.file`.
    #[inline]
    pub fn error_raw(&self) -> u32 {
        self.error.load(Ordering::Relaxed)
    }

    /// `WRITE_ONCE(so->so_error, error)`; `None` clears it.
    #[inline]
    pub fn set_error(&self, error: Option<Errno>) {
        self.error
            .store(error.map_or(0, |e| e.as_i32() as u32), Ordering::Relaxed);
    }

    /// `so_oobmark`.
    #[inline]
    pub fn oobmark(&self) -> u64 {
        self.oobmark.load(Ordering::Relaxed)
    }

    /// `so->so_oobmark = v` (`so_rcv`'s mutex held, or the protocol's lock: `tcp_input`).
    #[inline]
    pub fn set_oobmark(&self, v: u64) {
        self.oobmark.store(v, Ordering::Relaxed);
    }

    /// `so_sigio`.
    #[inline]
    pub fn sigio(&self) -> &SigioRef {
        &self.sigio
    }

    /// The wait channel of `&so->so_timeo` (connection, accept and close waits).
    #[inline]
    pub fn timeo_chan(&self) -> *const AtomicI16 {
        core::ptr::from_ref(&self.timeo)
    }

    /// The kernel's protocol control block (`so_pcb` of the UNIX domain), counted.
    pub fn pcb(&self) -> Option<AnyArc> {
        self.pcb.read().clone()
    }

    /// The kernel's protocol control block, borrowed while the returned reference lives
    /// (`sotounpcb(so)`, and `fptounp`'s read of `so_pcb` without the socket lock): one
    /// reader of the slot counted, no reference to the block taken. The holder must not sleep
    /// meanwhile (the slot's writer, `set_pcb`, would wait for it).
    pub fn pcb_read(&self) -> hal::sync::slot::ReadMostlyRef<'_, Option<AnyArc>> {
        self.pcb.read()
    }

    /// `so->so_pcb = pcb` of a kernel protocol, the socket locked; the old one given back.
    pub fn set_pcb(&self, _w: &SoLocked<'_>, pcb: Option<AnyArc>) -> Option<AnyArc> {
        self.pcb.replace(pcb)
    }

    /// `so->so_pcb` of a kernel internet protocol, for a holder of the socket's lock: the
    /// pcb borrowed from the locked slot for as long as `part`, the lock's part
    /// [`so_part::PCB`], is borrowed (the C's load; no count is taken, and the slot cannot
    /// change meanwhile: its writer needs the part mutably). `None` before the protocol's
    /// attach fills it and after its close empties it. A part of another lock is `panic(9)`.
    #[inline]
    pub fn pcb_locked<'a>(&'a self, part: &'a WitnessPart<'_>) -> Option<&'a AnyArc> {
        self.pcb_locked.get()?.get(part).as_ref()
    }

    /// `so->so_pcb = pcb` in the locked slot ([`Socket::pcb_locked`]), by the holder of the
    /// socket lock's part [`so_part::PCB`] (the protocol's attach, `tcp_free`); the old one
    /// given back, for the caller to drop. The cell is made, keyed to this socket's
    /// `so_lock`, at the first `Some`; a part of another lock is `panic(9)`.
    pub fn set_pcb_locked(
        &self,
        part: &mut WitnessPart<'_>,
        pcb: Option<AnyArc>,
    ) -> Option<AnyArc> {
        let cell = if let Some(cell) = self.pcb_locked.get() {
            cell
        } else {
            pcb.as_ref()?;
            // Only the lock's holder writes the slot: no other `set` can race this one.
            let _ =
                self.pcb_locked
                    .set(Guarded::keyed(self.so_lock.lock().id(), so_part::PCB, None));
            match self.pcb_locked.get() {
                Some(cell) => cell,
                None => kpanic(format_args!("set_pcb_locked: socket {self:p}: no cell")),
            }
        };
        mem::replace(cell.get_mut(part), pcb)
    }

    /// `so->so_pcb = inp` of a kernel internet protocol's socket (`in_pcballoc`; `None`,
    /// `in_pcbdetach`'s NULL), its caller holding the socket's lock (the protocol's attach and
    /// detach): the pcb counted in the slot the kernel's protocols read ([`Socket::pcb`]),
    /// beside the address `bsd`'s word keeps for both. A `bsd` protocol's socket keeps the
    /// address alone: `pcb` comes back untouched, for its caller to drop.
    pub(crate) fn set_inet_pcb(&self, pcb: Option<AnyArc>) -> Option<AnyArc> {
        if self.legacy_lock {
            return pcb;
        }
        self.pcb.replace(pcb)
    }

    /// `so->so_pcb` of a `bsd` protocol (its control block's address, 0 for none).
    #[inline]
    pub fn legacy_pcb(&self) -> usize {
        self.legacy_pcb.load(Ordering::Acquire)
    }

    /// `so->so_pcb = inp` of a `bsd` protocol.
    #[inline]
    pub fn set_legacy_pcb(&self, word: usize) {
        self.legacy_pcb.store(word, Ordering::Release);
    }

    /// `so->so_sp->ssp_socket`'s address as `kern.file` reports it (0 for none), read under
    /// the receive buffer's mutex, which keys it.
    pub fn splice_drain_addr(&self) -> usize {
        let Some(sp) = self.splice() else {
            return 0;
        };
        let g = self.so_rcv.lock();
        sp.drain(&g)
            .map_or(0, |s| core::ptr::from_ref::<Socket<G>>(&s).addr())
    }

    /// `so->so_pcb != NULL`.
    pub fn has_pcb(&self) -> bool {
        self.legacy_pcb() != 0 || self.pcb.read().is_some()
    }

    /// `so->so_sp`.
    pub fn splice(&self) -> Option<&Sosplice<G>> {
        self.sp.get().map(|b| &**b)
    }

    /// `so->so_upcallarg = arg; so->so_upcall = f` with the glue's registered function `f`
    /// (nfs's `nfssvc_addsock`), or `so->so_upcall = NULL` for `None` (`nfsrv_zapsock`): the
    /// argument first and the kind after it, cleared in the other order, so `sorwakeup`
    /// never sees the kind without its argument. As the C's two stores: the C marks them
    /// \[s\], but nfs writes them under the kernel lock alone, the socket unlocked, so no
    /// witness is asked for and no lock is taken here. The argument it held is
    /// answered, for the caller to drop where it likes (never the last handle while the
    /// caller holds its record, as nfs does).
    pub fn set_upcall(&self, arg: Option<AnyArc>) -> Option<AnyArc> {
        if arg.is_some() {
            let old = self.upcall_arg.replace(arg);
            self.upcall_kind.store(1, Ordering::Release);
            old
        } else {
            self.upcall_kind.store(0, Ordering::Release);
            self.upcall_arg.replace(None)
        }
    }

    /// `so_upcallarg` for a reader (ddb's `so_print`): a counted copy, `None` without an
    /// upcall.
    pub fn upcall_arg(&self) -> Option<AnyArc> {
        if self.upcall_kind.load(Ordering::Acquire) == 0 {
            return None;
        }
        self.upcall_arg.read().clone()
    }
}

/// The glue's registered `so_upcall` function (nfs's), once.
pub fn register_upcall<G: ProcGlue>(f: UpcallFn<G>) {
    let _ = net::<G>().upcall.set(f);
}

/// `soref(so)` of a socket a holder lends.
#[inline]
pub fn soref<G: ProcGlue>(so: &Socket<G>) -> SocketRef<G> {
    match PoolArc::from_ref(so) {
        Some(r) => r,
        None => kpanic(format_args!("soref: socket {so:p} not in sockpl")),
    }
}

impl<G: ProcGlue> Drop for Socket<G> {
    /// `sorele`'s body, the last reference gone: the async I/O registration, the knotes,
    /// the send buffer released, the receive buffer's rights disposed of and its mbufs
    /// freed; the splice box goes with the item.
    fn drop(&mut self) {
        sigio_free(&self.sigio);
        self.so_rcv.klist.free();
        self.so_snd.klist.free();

        self.so_snd.lock().sbrelease();

        // Unlocked dispose and cleanup, as the C's: the socket is unlinked from everywhere.
        let m = self.so_rcv.take_all_unshared();
        crate::net::domain::dispose_and_purge(self.proto, m);
    }
}

/// The net lock's part of a socket lock.
enum NetHold<'a> {
    /// None (a per-socket domain, or `solock_nonet`).
    None,
    /// `NET_LOCK()`.
    Excl(WriteWitness<'static>),
    /// `NET_LOCK_SHARED()`.
    Shared(ReadWitness<'static>),
    /// The net lock the caller holds, proved by the proof it passes: `bsd`'s C-style holding
    /// lent ([`solocked_legacy`]), or a kernel caller's own holding ([`solock_net`],
    /// [`solock_nonet`], [`solock_pair`]). Never let go here: the holding is the caller's.
    Lent(&'a NetHeld<'a>),
}

/// `so_lock`'s part of a socket lock.
enum SoHold<'a> {
    /// Not held (released for a while, or taken out by the drop; an internet socket under
    /// the net lock alone, `bsd`'s until N6.6b1, is gone).
    None,
    /// Held with a witness (the kernel's domains: the cells keyed to `so_lock` open with it).
    Witness(WriteWitness<'a>),
    /// Held by a detached holding (the domains still in `bsd`, whose protocols release and
    /// take the lock again C-style inside a request: `route_attach`, `route_output`,
    /// `pfkeyv2_send`); no cell opens with it.
    Detached,
    /// The detached holding of `bsd`'s caller, lent for a request ([`solocked_legacy`]):
    /// released and taken again around a wait, never let go by the drop.
    Lent,
    /// The detached holding of `bsd`'s caller on a kernel protocol's socket, attached for a
    /// request ([`solocked_legacy`]): a witness while the request runs, detached back to the
    /// caller by the drop.
    Attached(WriteWitness<'a>),
}

/// Which part of a kernel internet socket's `so_lock` witness opens which `[s]` cells
/// ([`SoLocked::inet`]): the socket's, the pcb's, the tcpcb's, the SYN cache entries'
/// (`hal::sync::rwlock::Guarded::keyed(so_lock's id, part, ..)`). One lock serialises them all in
/// the C; the parts let a request hold a reference into each at once.
pub mod so_part {
    /// The socket's own cells.
    pub const SO: u8 = 0;
    /// The pcb's slots (`inp_ppcb`).
    pub const PCB: u8 = 1;
    /// The tcpcb's state.
    pub const TCP: u8 = 2;
    /// The `[s]` members of the listener's SYN cache entries.
    pub const SYN: u8 = 3;
}

/// The socket lock of a kernel internet protocol's socket opened in parts
/// ([`SoLocked::inet`]): one exclusive part of the `so_lock` witness for each object the lock
/// serialises ([`so_part`]), and the proof of the net lock the socket lock holds (either
/// mode, or the one `bsd`'s caller lent). It borrows the [`SoLocked`] mutably, so the socket
/// cannot be unlocked or slept on while it lives.
pub struct InetLocked<'w> {
    /// Part [`so_part::SO`].
    pub so: WitnessPart<'w>,
    /// Part [`so_part::PCB`].
    pub pcb: WitnessPart<'w>,
    /// Part [`so_part::TCP`].
    pub tcp: WitnessPart<'w>,
    /// Part [`so_part::SYN`].
    pub syn: WitnessPart<'w>,
    /// The net lock's proof.
    pub net: NetHeld<'w>,
}

/// `solock(so)` held: the witness of the socket's lock. Its drop is `sounlock`.
#[must_use = "dropping it unlocks the socket"]
pub struct SoLocked<'a> {
    /// The socket's `so_lock`, to take it again after a release.
    lock: &'a hal::sync::rwlock::DetachableRwlock,
    /// `so_lock` held (or not, for a `bsd` protocol's internet socket under the net lock
    /// alone).
    so: SoHold<'a>,
    /// The net lock (internet sockets). Declared after `so`: dropped after it, so `so_lock`
    /// is let go first.
    net: NetHold<'a>,
}

impl<'a> SoLocked<'a> {
    /// The socket's `so_lock` taken as `so`'s protocol takes it: with a witness for the
    /// kernel's protocols, by a detached holding for `bsd`'s.
    fn take<G: ProcGlue>(so: &'a Socket<G>, net: NetHold<'a>) -> Self {
        let hold = if so.legacy_lock {
            so.so_lock.enter_write_detached();
            SoHold::Detached
        } else {
            SoHold::Witness(so.so_lock.lock().write())
        };
        SoLocked {
            lock: &so.so_lock,
            so: hold,
            net,
        }
    }

    /// The witness of `so_lock`, for the cells keyed to it (the UNIX domain's control
    /// block); `None` for a `bsd` protocol's socket (a detached holding, or an internet one
    /// locked by the net lock alone).
    #[inline]
    pub fn so_lock(&mut self) -> Option<&mut WriteWitness<'a>> {
        match &mut self.so {
            SoHold::Witness(w) | SoHold::Attached(w) => Some(w),
            SoHold::None | SoHold::Detached | SoHold::Lent => None,
        }
    }

    /// [`SoLocked::so_lock`], shared: to read the cells keyed to `so_lock`.
    #[inline]
    pub fn so_lock_ref(&self) -> Option<&WriteWitness<'a>> {
        match &self.so {
            SoHold::Witness(w) | SoHold::Attached(w) => Some(w),
            SoHold::None | SoHold::Detached | SoHold::Lent => None,
        }
    }

    /// The socket lock of a kernel internet protocol's socket opened in its parts
    /// ([`InetLocked`]): the `so_lock` witness split in four, beside the net lock's proof.
    /// `None` without a witness of `so_lock` (a `bsd` protocol's socket) or without a proof
    /// of the net lock (`solock_nonet`, whose caller holds it; the other sockets).
    pub fn inet(&mut self) -> Option<InetLocked<'_>> {
        let SoLocked { so, net, .. } = self;
        let net = match net {
            NetHold::Excl(w) => NetHeld::of(w)?,
            NetHold::Shared(w) => NetHeld::of(w)?,
            NetHold::Lent(n) => n.reborrow(),
            NetHold::None => return None,
        };
        let w = match so {
            SoHold::Witness(w) | SoHold::Attached(w) => w,
            SoHold::None | SoHold::Detached | SoHold::Lent => return None,
        };
        let [so, pcb, tcp, syn] = w.split::<4>();
        Some(InetLocked {
            so,
            pcb,
            tcp,
            syn,
            net,
        })
    }

    /// The proof of the net lock this socket lock holds or carries (either mode): `None`
    /// for a socket of a per-socket domain. For the caller that locks a second socket under
    /// this one's net lock (`soclose`'s queued children).
    pub fn net_proof(&self) -> Option<NetHeld<'_>> {
        match &self.net {
            NetHold::Excl(w) => NetHeld::of(w),
            NetHold::Shared(w) => NetHeld::of(w),
            NetHold::Lent(n) => Some(n.reborrow()),
            NetHold::None => None,
        }
    }

    /// `so_lock` let go (whatever the holding), what it was given back to take it again.
    fn release_so(&mut self) -> u8 {
        match mem::replace(&mut self.so, SoHold::None) {
            SoHold::None => 0,
            SoHold::Witness(w) => {
                drop(w);
                1
            }
            SoHold::Detached => {
                self.lock.exit_write_detached();
                2
            }
            SoHold::Lent => {
                self.lock.exit_write_detached();
                3
            }
            SoHold::Attached(w) => {
                // Only `solocked_legacy` attaches, and only under a lent net lock, whose
                // waits and unlocks are refused before they release `so_lock`.
                self.lock.detach(w);
                kpanic(format_args!(
                    "sounlock: an attached holding of bsd's caller is not let go"
                ))
            }
        }
    }

    /// `so_lock` taken again as [`SoLocked::release_so`] said it was held.
    fn retake_so(&mut self, how: u8) {
        self.so = match how {
            1 => SoHold::Witness(self.lock.lock().write()),
            2 => {
                self.lock.enter_write_detached();
                SoHold::Detached
            }
            3 => {
                self.lock.enter_write_detached();
                SoHold::Lent
            }
            _ => SoHold::None,
        };
    }

    /// Runs `f` with the socket unlocked (`sounlock(so)` .. `solock(so)` around namei and
    /// the vnode operations of `uipc_bind` and `unp_connect`, `sosleep_nsec`'s release
    /// around accept's wait), the same locks taken again in their order before returning:
    /// `so_lock` released first, then the net lock; the net lock taken again first.
    pub fn unlocked<R>(&mut self, f: impl FnOnce() -> R) -> R {
        if let NetHold::Lent(_) = self.net {
            kpanic(format_args!(
                "sounlock: the net lock lent by the caller is not let go"
            ));
        }
        let had_so = self.release_so();
        let net = match mem::replace(&mut self.net, NetHold::None) {
            NetHold::None | NetHold::Lent(_) => 0,
            NetHold::Excl(w) => {
                drop(w);
                1
            }
            NetHold::Shared(w) => {
                drop(w);
                2
            }
        };
        let r = f();
        self.net = match net {
            1 => NetHold::Excl(NETLOCK.write()),
            2 => NetHold::Shared(NETLOCK.read()),
            _ => NetHold::None,
        };
        self.retake_so(had_so);
        r
    }

    /// The holding of `so_lock` given over to the caller, detached (`sonewconn`'s socket,
    /// returned locked): the witness detached, a detached holding kept; the lock stays held
    /// until [`sounlock_detached`].
    fn detach_so(&mut self) {
        if let SoHold::Witness(w) = mem::replace(&mut self.so, SoHold::None) {
            self.lock.detach(w);
        }
    }

    /// The socket lock handed back to its taker still held (the C's `*solocked = so` of
    /// `tcp_input_solocked`, `syn_cache_get`'s new socket returned locked): the witness of
    /// `so_lock` detached, the lock held by the running process until [`solock_attach`] or
    /// [`sounlock_detached`]; the net lock's proof was the caller's and stays so. `panic(9)`
    /// for a holding that is not a witness of `so_lock` (a `bsd` protocol's socket, or one
    /// `bsd`'s caller lent).
    pub fn detach(mut self) {
        if !matches!(self.net, NetHold::Lent(_) | NetHold::None) {
            kpanic(format_args!(
                "SoLocked::detach: the net lock is this holding's own"
            ));
        }
        match &self.so {
            SoHold::Witness(_) => self.detach_so(),
            SoHold::None | SoHold::Detached | SoHold::Lent | SoHold::Attached(_) => kpanic(
                format_args!("SoLocked::detach: no witness of so_lock to detach"),
            ),
        }
    }
}

impl Drop for SoLocked<'_> {
    /// `sounlock(so)`: `so_lock` let go (a detached holding by its exit; a witness by its
    /// drop; an attached one detached back to `bsd`'s caller), then the net lock.
    fn drop(&mut self) {
        match mem::replace(&mut self.so, SoHold::None) {
            SoHold::Detached => self.lock.exit_write_detached(),
            SoHold::Attached(w) => self.lock.detach(w),
            SoHold::None | SoHold::Witness(_) | SoHold::Lent => {}
        }
    }
}

/// `solock(so)`: the net lock for the internet domains, the socket's lock for the others. A
/// kernel internet protocol's socket holds `so_lock` too, by a witness taken after the
/// exclusive net lock (D-1): the C serialises it by the net lock alone, which no cell can be
/// keyed to beside `so_lock`; the extra holding is uncontended, since every other holder of
/// that `so_lock` holds the net lock shared (`in_pcbsolock`, `solock_shared`,
/// `solock_nonet`'s callers), which this holding excludes.
pub fn solock<G: ProcGlue>(so: &Socket<G>) -> SoLocked<'_> {
    if so.is_inet() {
        // Every internet protocol is the kernel's since N6.6b1 (`bsd`'s raw rows answer with
        // the kernel's raw IP): no internet socket is held C-style.
        if so.legacy_lock {
            kpanic(format_args!(
                "solock: socket {so:p}: an internet socket of a legacy protocol"
            ));
        }
        let net = NetHold::Excl(NETLOCK.write());
        SoLocked {
            lock: &so.so_lock,
            so: SoHold::Witness(so.so_lock.lock().write()),
            net,
        }
    } else {
        SoLocked::take(so, NetHold::None)
    }
}

/// The lock `bsd`'s caller holds C-style on a socket (`solock(so)` of `bsd`'s adapters: the
/// net lock of an internet socket, with or without a detached holding of `so_lock`; a
/// detached holding of `so_lock` for the per-socket domains still in `bsd`), lent as a
/// witness for a request that wants one (`sobind`, `soconnect`, `solisten` of nfs and `bsd`'s
/// tests, nfs's send). An internet socket's caller lends its proof of the
/// net lock, `net` (`SaToken::lend_net`), which the witness carries for [`SoLocked::inet`]
/// and never lets go (a wait or an unlock under it is `panic(9)`). Of a kernel protocol's
/// internet socket (D-1), the caller's detached holding of `so_lock` is attached for the
/// request and detached back by the drop; without one, the caller holds the net lock
/// exclusively (`bsd`'s `solock`), and `so_lock` is taken for the request, uncontended, as
/// [`solock`] takes it (every internet socket is a kernel protocol's since N6.6b1). A `bsd`
/// protocol's socket (pfkey's, a per-socket domain) gets the holdings as they are: the drop
/// lets nothing go. `panic(9)` for an internet socket without `net`, for the other kernel
/// domains' sockets (their requests are the kernel's), and for a lock the caller does not
/// hold.
pub fn solocked_legacy<'a, G: ProcGlue>(
    so: &'a Socket<G>,
    net: Option<&'a NetHeld<'a>>,
) -> SoLocked<'a> {
    if so.is_inet() {
        let Some(net) = net else {
            kpanic(format_args!(
                "solocked_legacy: socket {so:p}: the net lock is not lent"
            ));
        };
        // No internet socket is a `bsd` protocol's since N6.6b1 (see `solock`).
        if so.legacy_lock {
            kpanic(format_args!(
                "solocked_legacy: socket {so:p}: an internet socket of a legacy protocol"
            ));
        }
        let hold = if so.so_lock.held_detached() {
            SoHold::Attached(so.so_lock.attach())
        } else {
            kassert!(hal::sync::netlock::write_held());
            SoHold::Witness(so.so_lock.lock().write())
        };
        SoLocked {
            lock: &so.so_lock,
            so: hold,
            net: NetHold::Lent(net),
        }
    } else {
        if !so.legacy_lock {
            kpanic(format_args!(
                "solocked_legacy: socket {so:p} of a kernel domain"
            ));
        }
        kassert!(so.so_lock.held_detached());
        SoLocked {
            lock: &so.so_lock,
            so: SoHold::Lent,
            net: NetHold::None,
        }
    }
}

/// `so`'s lock taken under the caller's proof of the net lock, `net` (`in_pcbsolock`'s: the
/// C's `rw_enter_write(&so->so_lock)` of an internet socket whose caller holds the net lock,
/// either mode): `so_lock` as `so`'s protocol takes it (a witness for the kernel's, whose
/// cells [`SoLocked::inet`] opens; a detached holding for `bsd`'s), the proof carried and
/// never let go here. The drop lets `so_lock` go (`in_pcbsounlock`).
pub fn solock_net<'a, G: ProcGlue>(so: &'a Socket<G>, net: &'a NetHeld<'a>) -> SoLocked<'a> {
    SoLocked::take(so, NetHold::Lent(net))
}

/// `so`'s lock as [`solock_net`] gives it, from the detached holding of `so_lock` the running
/// process keeps (`tcp_input_mlist`'s socket kept locked between two segments, the socket
/// `sonewconn` returned locked): the holding attached back to a witness
/// (`DetachableRwlock::attach`, one compare-and-swap, no wait), the caller's proof of the net
/// lock `net` carried. Its drop lets `so_lock` go; [`SoLocked::detach`] keeps it held,
/// detached again. `panic(9)` for a `bsd` protocol's socket (held C-style, never by a
/// witness) and when the running process holds no detached holding of `so_lock`.
pub fn solock_attach<'a, G: ProcGlue>(so: &'a Socket<G>, net: &'a NetHeld<'a>) -> SoLocked<'a> {
    if so.legacy_lock {
        kpanic(format_args!(
            "solock_attach: socket {so:p} of a bsd protocol"
        ));
    }
    SoLocked {
        lock: &so.so_lock,
        so: SoHold::Witness(so.so_lock.attach()),
        net: NetHold::Lent(net),
    }
}

/// `solock_shared(so)`: the shared net lock (internet domains), then the socket's lock.
pub fn solock_shared<G: ProcGlue>(so: &Socket<G>) -> SoLocked<'_> {
    let net = if so.is_inet() {
        NetHold::Shared(NETLOCK.read())
    } else {
        NetHold::None
    };
    SoLocked::take(so, net)
}

/// `solock_nonet(so)`: the socket's lock; an internet socket's caller holds the net lock and
/// passes its proof, `net` (a listener's: `sonewconn`'s new socket, `soclose`'s queued
/// children), which the lock carries for [`SoLocked::inet`] and never lets go: a protocol's
/// request on the socket (`pru_attach`, `pru_abort`) can prove it. `panic(9)` for an
/// internet socket without the proof; the other sockets need none (`None`).
pub fn solock_nonet<'a, G: ProcGlue>(
    so: &'a Socket<G>,
    net: Option<&'a NetHeld<'a>>,
) -> SoLocked<'a> {
    if !so.is_inet() {
        return SoLocked::take(so, NetHold::None);
    }
    let Some(net) = net else {
        kpanic(format_args!(
            "solock_nonet: socket {so:p}: no proof of the net lock"
        ));
    };
    SoLocked::take(so, NetHold::Lent(net))
}

/// `solock_persocket(so)`: whether `so` has a lock of its own (it is not an internet one).
#[inline]
pub fn solock_persocket<G: ProcGlue>(so: &Socket<G>) -> bool {
    !so.is_inet()
}

/// `solock_pair(so1, so2)`: locks two sockets of one type, in address order; the second
/// witness is `None` when they are one socket. An internet pair's caller holds the net lock
/// (the C's `NET_LOCK_SHARED()` at the top of `solock_pair`, taken by the caller here so both
/// witnesses can carry its proof, `net`, for [`SoLocked::inet`]); `panic(9)` for an internet
/// pair without it. The other pairs need none (`None`).
pub fn solock_pair<'a, G: ProcGlue>(
    so1: &'a Socket<G>,
    so2: &'a Socket<G>,
    net: Option<&'a NetHeld<'a>>,
) -> (SoLocked<'a>, Option<SoLocked<'a>>) {
    kassert!(so1.so_type == so2.so_type);

    let hold = || {
        if !so1.is_inet() {
            return NetHold::None;
        }
        match net {
            Some(n) => NetHold::Lent(n),
            None => kpanic(format_args!(
                "solock_pair: socket {so1:p}: no proof of the net lock"
            )),
        }
    };
    if core::ptr::eq(so1, so2) {
        return (SoLocked::take(so1, hold()), None);
    }
    if core::ptr::from_ref(so1) < core::ptr::from_ref(so2) {
        let a = SoLocked::take(so1, hold());
        (a, Some(SoLocked::take(so2, hold())))
    } else {
        let b = SoLocked::take(so2, hold());
        (SoLocked::take(so1, hold()), Some(b))
    }
}

/// The shared net lock an internet pair of `so1`'s domain is locked under (`solock_pair`'s
/// `NET_LOCK_SHARED()`), taken by its caller before the pair: `None` for the other domains.
fn pair_netlock<G: ProcGlue>(so1: &Socket<G>) -> Option<ReadWitness<'static>> {
    so1.is_inet().then(|| NETLOCK.read())
}

/// `soassertlocked(so)`'s and `soassertlocked_readonly(so)`'s question, for the paths
/// `bsd` reaches without a witness: is the lock that serialises `so` held now (by anyone:
/// a running process's own holding cannot be told from another's here)?
pub fn solocked<G: ProcGlue>(so: &Socket<G>) -> bool {
    if so.is_inet() {
        hal::sync::netlock::status() != 0
    } else {
        so.so_lock.lock().status() != 0
    }
}

/// `sosleep_nsec(so, ident, prio, wmesg, nsecs)`: sleeps on `ident`, releasing the lock that
/// serialises `so` while asleep, the same locks held again on return.
pub fn sosleep_nsec<I: ?Sized>(
    w: &mut SoLocked<'_>,
    ident: *const I,
    prio: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    match &mut w.net {
        NetHold::Shared(_) => {
            // `rw_exit_write(&so->so_lock)` .. `rw_enter_write` around the net lock's sleep.
            let had_so = w.release_so();
            let r = match &mut w.net {
                NetHold::Shared(n) => n.sleep_nsec(ident, prio, wmesg, nsecs),
                _ => Ok(()),
            };
            w.retake_so(had_so);
            r
        }
        NetHold::Excl(_) => {
            // The exclusive net lock's sleep. A kernel protocol's socket holds `so_lock` too
            // (D-1): let go around the sleep as the shared case lets it go, since a sleeper
            // keeping it would block its own waker; the net lock is taken again first.
            let had_so = w.release_so();
            let r = match &mut w.net {
                NetHold::Excl(n) => n.sleep_nsec(ident, prio, wmesg, nsecs),
                _ => Ok(()),
            };
            w.retake_so(had_so);
            r
        }
        NetHold::Lent(_) => kpanic(format_args!(
            "sosleep_nsec: the net lock lent by the caller is not let go"
        )),
        NetHold::None => match &mut w.so {
            SoHold::Witness(s) | SoHold::Attached(s) => s.sleep_nsec(ident, prio, wmesg, nsecs),
            SoHold::Detached | SoHold::Lent => w.lock.sleep_detached(ident, prio, wmesg, nsecs),
            SoHold::None => kpanic(format_args!("sosleep_nsec: socket not locked")),
        },
    }
}

/// `soalloc(prp, wait)`: a socket of protocol `prp` with its locks and lists made, one
/// handle and the parked existence reference, or `None` when the pool is empty.
pub fn soalloc<G: ProcGlue>(
    prp: &'static dyn Protocol<G>,
    wait: i32,
    init: SoInit,
) -> Option<SocketRef<G>> {
    let flags = if wait == crate::net::mbuf::M_WAIT {
        hal::mem::pool::PR_WAITOK
    } else {
        hal::mem::pool::PR_NOWAIT
    };
    let so = net::<G>()
        .sockpl
        .get_homed(flags, || Socket::new(prp, init))?;
    // WITNESS: not configured.
    sigio_init(&so.sigio);
    // The existence reference, which `sofree` adopts.
    PoolArc::forget(so.clone());
    Some(so)
}

/// The connection queue's state of `so`, locked.
fn cq<G: ProcGlue>(so: &Socket<G>) -> MutexedGuard<'_, ConnQueue<G>> {
    so.cq.lock()
}

/// `soqinsque(head, so, q)`: queues `so` on `head`'s partial (`Onq::Q0`) or complete queue;
/// the queue holds a parked reference to `so` from now on. The caller holds both sockets'
/// locks, as the C's.
pub fn soqinsque<G: ProcGlue>(head: &Socket<G>, so: &Socket<G>, q: Onq) {
    // The queue's reference.
    let r = soref(so);
    let mut hg = cq(head);
    {
        let mut sg = cq(so);
        kassert!(sg.onq == Onq::None);
        sg.head = Some(soref(head));
        sg.onq = q;
        so.queued.store(true, Ordering::Release);
    }
    match q {
        Onq::Q0 => {
            hg.q0len += 1;
            head.q0.insert_tail(&mut hg, r.as_pin());
        }
        Onq::Q => {
            hg.qlen += 1;
            head.qlen_mirror.store(hg.qlen, Ordering::Relaxed);
            head.q.insert_tail(&mut hg, r.as_pin());
        }
        Onq::None => kpanic(format_args!("soqinsque: no queue")),
    }
    drop(hg);
    PoolArc::forget(r);
}

/// `soqremque(so, q)`: takes `so` off its listener's `q`; the queue's reference given back
/// (`None` when `so` is not on that queue). The listener's `cq`, then `so`'s, held across
/// the check and the removal, so a socket moved or removed meanwhile is seen.
pub fn soqremque<G: ProcGlue>(so: &Socket<G>, q: Onq) -> Option<SocketRef<G>> {
    let head = {
        let sg = cq(so);
        if sg.onq != q {
            return None;
        }
        sg.head.clone()?
    };
    let old_head;
    {
        let mut hg = cq(&head);
        let mut sg = cq(so);
        if sg.onq != q || !sg.head.as_ref().is_some_and(|h| PoolArc::ptr_eq(h, &head)) {
            return None;
        }
        match q {
            Onq::Q0 => {
                head.q0.remove(&mut hg, so);
                hg.q0len -= 1;
            }
            Onq::Q => {
                head.q.remove(&mut hg, so);
                hg.qlen -= 1;
                head.qlen_mirror.store(hg.qlen, Ordering::Relaxed);
            }
            Onq::None => return None,
        }
        sg.onq = Onq::None;
        so.queued.store(false, Ordering::Release);
        old_head = sg.head.take();
    }
    // The listener's reference goes after the queues' mutexes.
    drop(old_head);
    drop(head);
    PoolArc::adopt(so)
}

/// The first socket of `head`'s queue `q`, counted (`TAILQ_FIRST(&head->so_q)`), under
/// `head`'s `cq`.
fn soq_first<G: ProcGlue>(head: &Socket<G>, q: Onq) -> Option<SocketRef<G>> {
    let hg = cq(head);
    let first = match q {
        Onq::Q0 => head.q0.first(&hg),
        Onq::Q => head.q.first(&hg),
        Onq::None => None,
    };
    first.map(soref)
}

/// `soisconnecting(so)`: `so` is in process of connecting to its peer.
pub fn soisconnecting<G: ProcGlue>(so: &Socket<G>) {
    so.st_clear(SS_ISCONNECTED | SS_ISDISCONNECTING);
    so.st_set(SS_ISCONNECTING);
}

/// `soisconnected(so)`: `so` is connected; a connection on a listener's `so_q0` moves to its
/// `so_q`, where `accept(2)` finds it, in one section of the listener's queue.
pub fn soisconnected<G: ProcGlue>(so: &Socket<G>) {
    so.st_clear(SS_ISCONNECTING | SS_ISDISCONNECTING);
    so.st_set(SS_ISCONNECTED);

    // A socket on no queue (the mirror, stable under the socket's lock the caller holds:
    // queued once by sonewconn, taken off under it): the C's `head == NULL` branch, its own
    // channel woken as `so_timeo_wakeup` wakes it, no `cq`.
    if !so.queued.load(Ordering::Acquire) {
        so_timeo_wakeup(so);
        sorwakeup(so);
        sowwakeup(so);
        return;
    }
    let sg = cq(so);
    let head = if sg.onq == Onq::Q0 {
        sg.head.clone()
    } else {
        None
    };
    let Some(head) = head else {
        wakeup(so.timeo_chan());
        drop(sg);
        sorwakeup(so);
        sowwakeup(so);
        return;
    };
    // The listener's queue first: the order head -> child.
    drop(sg);
    {
        let mut hg = cq(&head);
        let mut sg = cq(so);
        if sg.onq != Onq::Q0 || !sg.head.as_ref().is_some_and(|h| PoolArc::ptr_eq(h, &head)) {
            return;
        }
        head.q0.remove(&mut hg, so);
        hg.q0len -= 1;
        let r = soref(so);
        head.q.insert_tail(&mut hg, r.as_pin());
        drop(r);
        hg.qlen += 1;
        head.qlen_mirror.store(hg.qlen, Ordering::Relaxed);
        sg.onq = Onq::Q;
        drop(sg);
        wakeup_one(head.timeo_chan());
    }
    sorwakeup(&head);
}

/// `soisdisconnecting(so)`: `so` is in process of disconnecting: it can neither send nor
/// receive any more.
pub fn soisdisconnecting<G: ProcGlue>(so: &Socket<G>) {
    so.st_clear(SS_ISCONNECTING);
    so.st_set(SS_ISDISCONNECTING);

    so.so_rcv.lock().set_state(SS_CANTRCVMORE);
    so.so_snd.lock().set_state(SS_CANTSENDMORE);

    so_timeo_wakeup(so);
    sowwakeup(so);
    sorwakeup(so);
}

/// `soisdisconnected(so)`: the connection to the peer is totally severed.
pub fn soisdisconnected<G: ProcGlue>(so: &Socket<G>) {
    so.so_rcv.lock().set_state(SS_CANTRCVMORE);
    so.so_snd.lock().set_state(SS_CANTSENDMORE);

    so.st_clear(SS_ISCONNECTING | SS_ISCONNECTED | SS_ISDISCONNECTING);
    so.st_set(SS_ISDISCONNECTED);

    so_timeo_wakeup(so);
    sowwakeup(so);
    sorwakeup(so);
}

/// `wakeup(&so->so_timeo)`, for `bsd`'s `tcp_close` too; the caller holds `so`'s lock. The
/// channel's sleepers are `connect`'s and `soclose`'s lingering, under the socket's lock,
/// which the waker holds, and a listener's `accept` (`soaccept_take`), whose interlock is
/// the queue's mutex: only a listener's wakeup takes `cq` (the C's interlock is the
/// socket's lock for all three, so the C takes no other).
pub fn so_timeo_wakeup<G: ProcGlue>(so: &Socket<G>) {
    if so.has_options(SO_ACCEPTCONN) {
        let _g = cq(so);
        wakeup(so.timeo_chan());
    } else {
        wakeup(so.timeo_chan());
    }
}

/// `sonewconn(head, connstatus, wait)`: when an attempt at a new connection is noted on a
/// socket which accepts connections, `sonewconn` is called. If the connection is possible
/// (subject to space constraints, etc.) then we allocate a new structure, properly linked
/// into the data structure of the original socket, and return this, locked: its `so_lock`
/// held detached (the caller's `sounlock_nonet`, [`sounlock_detached`]). `connstatus` may be
/// 0 or `SS_ISCONNECTED`. The caller holds `head`'s lock.
pub fn sonewconn<G: ProcGlue>(
    head: &Socket<G>,
    connstatus: u32,
    wait: i32,
    net: Option<&NetHeld<'_>>,
) -> Option<SocketRef<G>> {
    let soqueue = if connstatus != 0 { Onq::Q } else { Onq::Q0 };

    if crate::net::mbuf::m_pool_used() > 95 {
        return None;
    }
    {
        let hg = cq(head);
        if i32::from(hg.qlen) + i32::from(hg.q0len) > i32::from(head.qlimit()) * 3 {
            return None;
        }
    }
    let so = soalloc(
        head.proto,
        wait,
        SoInit {
            so_type: head.so_type,
            state: head.state() | SS_NOFDREF,
            options: head.options() & !SO_ACCEPTCONN,
            linger: head.linger(),
            timeo: head.timeo(),
            euid: head.euid,
            ruid: head.ruid,
            egid: head.egid,
            rgid: head.rgid,
            cpid: head.cpid,
        },
    )?;

    // Lock order will be `head' -> `so' while these sockets are linked. An internet
    // listener's caller holds the net lock (`net`, its proof): the new socket's lock carries
    // it, so `pru_attach` can prove it.
    let mut w = solock_nonet(&so, net);

    let ok = 'fail: {
        // Inherit watermarks but those may get clamped in low mem situations.
        if soreserve(&so, head.so_snd.hiwat(), head.so_rcv.hiwat()).is_err() {
            break 'fail false;
        }
        // The listener's values under its buffers' mutexes; the new socket is nobody
        // else's yet (not on the queue), so its fields are stored as the C stores them.
        for (h, s) in [(&head.so_snd, &so.so_snd), (&head.so_rcv, &so.so_rcv)] {
            let _hg = h.lock();
            s.init_fresh(h.wat(), h.lowat(), h.timeo_nsecs());
        }

        sigio_copy(&so.sigio, &head.sigio);

        soqinsque(head, &so, soqueue);
        if so.usrreqs().attach(&so, &mut w, 0, wait).is_err() {
            drop(soqremque(&so, soqueue));
            break 'fail false;
        }
        if connstatus != 0 {
            so.st_set(connstatus);
            sorwakeup(head);
            so_timeo_wakeup(head);
        }

        true
    };
    if ok {
        // Returned locked: the holding of `so_lock` detached for the caller to end.
        w.detach_so();
        drop(w);
        return Some(so);
    }

    // fail:
    drop(w);
    so.freed.store(true, Ordering::Relaxed);
    drop(PoolArc::adopt(&*so));
    None
}

/// `sounlock_nonet(so)` of a socket `sonewconn` handed over locked (its detached holding of
/// `so_lock`).
pub fn sounlock_detached<G: ProcGlue>(so: &Socket<G>) {
    so.so_lock.exit_write_detached();
}

/// `socantsendmore(so)` indicates that no more data will be sent on the socket; it would
/// normally be applied to a socket when the user informs the system that no more data is to
/// be sent, by the protocol code (in case `PRU_SHUTDOWN`).
pub fn socantsendmore<G: ProcGlue>(so: &Socket<G>) {
    so.so_snd.lock().set_state(SS_CANTSENDMORE);
    sowwakeup(so);
}

/// `socantrcvmore(so)` indicates that no more data will be received, and will normally be
/// applied to the socket by a protocol when it detects that the peer will send no more
/// data. Data queued for reading in the socket may yet be read.
pub fn socantrcvmore<G: ProcGlue>(so: &Socket<G>) {
    so.so_rcv.lock().set_state(SS_CANTRCVMORE);
    sorwakeup(so);
}

/// `soreserve(so, sndcc, rcvcc)`: reserves `sndcc` bytes of send buffer and `rcvcc` of
/// receive buffer and sets their low-water marks; `ENOBUFS` past `sb_max`.
pub fn soreserve<G: ProcGlue>(so: &Socket<G>, sndcc: u64, rcvcc: u64) -> Result<(), Errno> {
    let mut rg = so.so_rcv.lock();
    let mut sg = so.so_snd.lock();
    sg.sbreserve(sndcc)?;
    sg.set_wat(sndcc);
    if so.so_snd.lowat() == 0 {
        sg.set_lowat(crate::net::mbuf::MCLBYTES as i64);
    }
    if so.so_snd.lowat() > so.so_snd.hiwat() as i64 {
        sg.set_lowat(so.so_snd.hiwat() as i64);
    }
    if let Err(e) = rg.sbreserve(rcvcc) {
        sg.sbrelease();
        return Err(e);
    }
    rg.set_wat(rcvcc);
    if so.so_rcv.lowat() == 0 {
        rg.set_lowat(1);
    }
    Ok(())
}

/// `socreate(dom, aso, type, proto)`: a new socket of domain `dom`, type `type_` and protocol
/// `proto` (0: the domain's first of that type), attached to its protocol, for the thread
/// `p`.
pub fn socreate<G: ProcGlue>(
    p: &Thread<G>,
    dom: i32,
    type_: i32,
    proto: i32,
) -> Result<SocketRef<G>, Errno> {
    let prp = if proto != 0 {
        crate::net::domain::pffindproto::<G>(dom, proto, type_)
    } else {
        crate::net::domain::pffindtype::<G>(dom, type_)
    };
    let Some(prp) = prp.filter(|prp| prp.usrreqs().is_some()) else {
        return Err(Errno::EPROTONOSUPPORT);
    };
    if i32::from(prp.ty()) != type_ {
        return Err(Errno::EPROTOTYPE);
    }
    let cred = p.ucred();
    let state = if crate::proc::suser(p).is_ok() {
        crate::net::sockbuf::SS_PRIV
    } else {
        0
    };
    let Some(so) = soalloc(
        prp,
        crate::net::mbuf::M_WAIT,
        SoInit {
            so_type: type_,
            state,
            options: 0,
            linger: 0,
            timeo: 0,
            euid: cred.cr_uid.get(),
            ruid: cred.cr_ruid.get(),
            egid: cred.cr_gid.get(),
            rgid: cred.cr_rgid.get(),
            cpid: p.process().ps_pid,
        },
    ) else {
        return Err(Errno::ENOBUFS);
    };
    // A fresh socket, nobody else's yet: the C's stores without the buffers' mutexes.
    so.so_snd.init_fresh(0, 0, INFSLP);
    so.so_rcv.init_fresh(0, 0, INFSLP);

    let mut w = solock_shared(&so);
    if let Err(error) = so
        .usrreqs()
        .attach(&so, &mut w, proto, crate::net::mbuf::M_WAIT)
    {
        so.st_set(SS_NOFDREF);
        // sofree() calls sounlock().
        sofree(&so, w, false);
        return Err(error);
    }
    drop(w);
    Ok(so)
}

/// `sobind(so, nam, p)`: binds `so` to the address in `nam`.
pub fn sobind<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
    p: &Thread<G>,
) -> Result<(), Errno> {
    so.usrreqs().bind(so, w, nam, p)
}

/// `solisten(so, backlog)`: makes `so` accept connections, at most `backlog` of them queued
/// (clamped to `[sominconn, somaxconn]`).
pub fn solisten<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    backlog: i32,
) -> Result<(), Errno> {
    let somaxconn_local = SOMAXCONN_VAR.load(Ordering::Relaxed);
    let sominconn_local = SOMINCONN_VAR.load(Ordering::Relaxed);
    let mut backlog = backlog;

    match so.so_type {
        sysdefs::socket::SOCK_STREAM | sysdefs::socket::SOCK_SEQPACKET => {}
        _ => return Err(Errno::EOPNOTSUPP),
    }

    if so.has_state(SS_ISCONNECTED | SS_ISCONNECTING | SS_ISDISCONNECTING) {
        return Err(Errno::EINVAL);
    }
    if isspliced(so) || issplicedback(so) {
        return Err(Errno::EOPNOTSUPP);
    }
    so.usrreqs().listen(so, w)?;
    let empty = {
        let g = cq(so);
        so.q.is_empty(&g)
    };
    if empty {
        so.options.fetch_or(SO_ACCEPTCONN, Ordering::Relaxed);
    }
    if backlog < 0 || backlog > somaxconn_local {
        backlog = somaxconn_local;
    }
    if backlog < sominconn_local {
        backlog = sominconn_local;
    }
    so.qlimit.store(backlog as i16, Ordering::Relaxed);
    Ok(())
}

/// `sofree(so, keep_lock)`: decommissions a socket that has neither a protocol control
/// block nor a file reference, taking it off its listener's partial queue first: the
/// existence reference goes (the last reference frees the socket). Unlocks `so` unless
/// `keep_lock`, which gives the witness back.
pub fn sofree<'a, G: ProcGlue>(
    so: &'a Socket<G>,
    w: SoLocked<'a>,
    keep_lock: bool,
) -> Option<SoLocked<'a>> {
    let mut w = w;
    sofree_kept(so, &mut w);
    if keep_lock { Some(w) } else { None }
}

/// `sofree(so, 1)`: [`sofree`] with the caller keeping its witness (a protocol's
/// `pru_abort`, `uipc_abort`'s `unp_detach(unp); sofree(so, 1)`). The caller holds a
/// reference to `so` besides (its `&Socket` borrows it), so the existence reference this
/// drops is never the last while the witness lives.
pub fn sofree_kept<G: ProcGlue>(so: &Socket<G>, _w: &mut SoLocked<'_>) {
    if so.has_pcb() || !so.has_state(SS_NOFDREF) {
        return;
    }
    // A socket on no queue (the mirror read without `cq`, which only a queued one takes):
    // nothing to look at, as the C's `so_head == NULL`.
    let queued = if so.queued.load(Ordering::Acquire) {
        {
            let sg = cq(so);
            // We must not decommission a socket that's on the accept(2) queue. If we do,
            // then accept(2) may hang after select(2) indicated that the listening socket
            // was ready.
            if sg.onq == Onq::Q {
                return;
            }
        }
        // A partial connection leaves its listener's queue (with the queue's reference).
        soqremque(so, Onq::Q0)
    } else {
        None
    };
    if so.freed.swap(true, Ordering::AcqRel) {
        drop(queued);
        return;
    }
    let existence = PoolArc::adopt(so);
    drop(queued);
    drop(existence);
}

/// `sofree(so, 1)` by `bsd`'s protocols (`in_pcbdetach` after `so->so_pcb = NULL`), which
/// hold the socket's lock by their own conventions: no witness to keep.
pub fn sofree_legacy<G: ProcGlue>(so: &Socket<G>) {
    if so.has_pcb() || !so.has_state(SS_NOFDREF) {
        return;
    }
    // `cq` only for a queued socket, as `sofree_kept`.
    let queued = if so.queued.load(Ordering::Acquire) {
        if cq(so).onq == Onq::Q {
            return;
        }
        soqremque(so, Onq::Q0)
    } else {
        None
    };
    if so.freed.swap(true, Ordering::AcqRel) {
        drop(queued);
        return;
    }
    let existence = PoolArc::adopt(so);
    drop(queued);
    drop(existence);
}

/// `solinger_nsec(so)`: how long `soclose` lingers.
fn solinger_nsec<G: ProcGlue>(so: &Socket<G>) -> u64 {
    if so.linger() == 0 {
        return INFSLP;
    }
    crate::time::clock::sec_to_nsec(so.linger() as u64)
}

/// `soclose(so, flags)`: close a socket on last file table reference removal. Initiate
/// disconnect if connected. Free socket when disconnect complete.
pub fn soclose<G: ProcGlue>(so: &Socket<G>, flags: i32) -> Result<(), Errno> {
    let mut error = Ok(());

    let mut w = solock_shared(so);
    // Revoke async IO early. There is a final revocation in sofree().
    sigio_free(&so.sigio);
    'discard: {
        'drop: {
            if so.has_state(SS_ISCONNECTED) {
                if !so.has_pcb() {
                    break 'discard;
                }
                if !so.has_state(SS_ISDISCONNECTING)
                    && let Err(e) = sodisconnect(so, &mut w)
                {
                    error = Err(e);
                    break 'drop;
                }
                if so.has_options(sysdefs::socket::SO_LINGER) {
                    if so.has_state(SS_ISDISCONNECTING)
                        && flags & sysdefs::socket::MSG_DONTWAIT != 0
                    {
                        break 'drop;
                    }
                    while so.has_state(SS_ISCONNECTED) {
                        if let Err(e) = sosleep_nsec(
                            &mut w,
                            so.timeo_chan(),
                            crate::net::sockbuf::PSOCK | PCATCH,
                            "netcls",
                            solinger_nsec(so),
                        ) {
                            error = Err(e);
                            break;
                        }
                    }
                }
            }
        }
        // drop:
        if so.has_pcb() {
            let error2 = so.usrreqs().detach(so, &mut w);
            if error.is_ok() {
                error = error2;
            }
        }
        if so.has_options(SO_ACCEPTCONN) {
            let persocket = solock_persocket(so);

            // The reference to each queued socket is taken under the listener's queue mutex,
            // before that mutex is let go (`soq_first`), so it cannot go meanwhile.
            while let Some(so2) = soq_first(so, Onq::Q0) {
                let mut w2 = solock(&so2);
                let Some(queued) = soqremque(&so2, Onq::Q0) else {
                    continue;
                };
                drop(w);
                soabort(&so2, &mut w2);
                drop(w2);
                drop(queued);
                drop(so2);
                w = solock_shared(so);
            }
            while let Some(so2) = soq_first(so, Onq::Q) {
                if persocket {
                    let mut w2 = solock_nonet(&so2, None);
                    let Some(queued) = soqremque(&so2, Onq::Q) else {
                        continue;
                    };
                    drop(w);
                    soabort(&so2, &mut w2);
                    drop(w2);
                    drop(queued);
                    drop(so2);
                    w = solock_shared(so);
                } else {
                    // The listener's net lock, proved for the child's `pru_abort`.
                    let net = w.net_proof();
                    let mut w2 = solock_nonet(&so2, net.as_ref());
                    let Some(queued) = soqremque(&so2, Onq::Q) else {
                        continue;
                    };
                    soabort(&so2, &mut w2);
                    drop(w2);
                    drop(queued);
                }
            }
        }
    }
    // discard:
    if let Some(sp) = so.splice() {
        drop(w);
        soclose_unsplice(so, sp);
        w = solock_shared(so);
    }

    if so.has_state(SS_NOFDREF) {
        kpanic(format_args!(
            "soclose NOFDREF: so {so:p}, so_type {}",
            so.so_type
        ));
    }
    so.st_set(SS_NOFDREF);

    // sofree() calls sounlock().
    sofree_kept(so, &mut w);
    drop(w);
    error
}

/// `soabort(so)`: `pru_abort`.
pub fn soabort<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) {
    so.usrreqs().abort(so, w);
}

/// `soaccept(so, nam)`: a connection taken off its listener's queue gets a file reference
/// and the peer's address in `nam`.
pub fn soaccept<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &mut Mbuf,
) -> Result<(), Errno> {
    if !so.has_state(SS_NOFDREF) {
        kpanic(format_args!(
            "soaccept !NOFDREF: so {so:p}, so_type {}",
            so.so_type
        ));
    }
    so.st_clear(SS_NOFDREF);
    if !so.has_state(SS_ISDISCONNECTED) || !so.pr_flags(crate::net::proto::PR_ABRTACPTDIS) {
        so.usrreqs().accept(so, w, nam)
    } else {
        Err(Errno::ECONNABORTED)
    }
}

/// `soconnect(so, nam)`: connects `so` to the address in `nam`.
pub fn soconnect<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
) -> Result<(), Errno> {
    if so.has_options(SO_ACCEPTCONN) {
        return Err(Errno::EOPNOTSUPP);
    }
    // If protocol is connection-based, can only connect once. Otherwise, if connected, try
    // to disconnect first. This allows user to disconnect by connecting to, e.g., a null
    // address.
    if so.has_state(SS_ISCONNECTED | SS_ISCONNECTING)
        && (so.pr_flags(crate::net::proto::PR_CONNREQUIRED) || sodisconnect(so, w).is_err())
    {
        Err(Errno::EISCONN)
    } else {
        so.usrreqs().connect(so, w, nam)
    }
}

/// `soconnect2(so1, so2)`: connects two sockets of one protocol to each other.
pub fn soconnect2<G: ProcGlue>(so1: &Socket<G>, so2: &Socket<G>) -> Result<(), Errno> {
    let r = pair_netlock(so1);
    let net = r.as_ref().and_then(NetHeld::of);
    let (mut w1, mut w2) = solock_pair(so1, so2, net.as_ref());
    so1.usrreqs().connect2(so1, &mut w1, so2, w2.as_mut())
}

/// `sodisconnect(so)`: starts disconnecting a connected socket.
pub fn sodisconnect<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    if !so.has_state(SS_ISCONNECTED) {
        return Err(Errno::ENOTCONN);
    }
    if so.has_state(SS_ISDISCONNECTING) {
        return Err(Errno::EALREADY);
    }
    so.usrreqs().disconnect(so, w)
}

/// `doaccept`'s take of a connection from the listener `head`, locked by `wh`
/// (`solock_shared(head)`), in the C's order: the wait while the queue is empty and
/// `so_error` (`ECONNABORTED` once the listener can receive no more and its queue is empty,
/// `EWOULDBLOCK` for `nonblock`, the sleep's error); `so = TAILQ_FIRST(&head->so_q)`, counted
/// under the listener's `cq`; `solock_nonet(so)` while `head`'s lock is held (the order
/// listener, then connection); `soqremque(so, 1)` (a connection another took meanwhile is
/// looked for again, where the C's `panic("accept")`); `knote(&head->so_rcv.sb_klist, 0)`;
/// `sounlock_nonet(head)`, the net lock going over to the connection; then `f` with the
/// connection's witness (the C's `soaccept(so, nam)`), and `sounlock_shared(so)` after it.
/// Returns the connection, counted by the queue's reference (which becomes the file's), and
/// what `f` returned; `head`'s lock is let go on every return. Every waker writes what the
/// wait reads before it takes the listener's `cq`, under which the check and the sleep are
/// one section, so no wakeup is lost.
pub fn soaccept_take<'h, G: ProcGlue, R>(
    head: &'h Socket<G>,
    mut wh: SoLocked<'h>,
    nonblock: bool,
    f: impl FnOnce(&Socket<G>, &mut SoLocked<'_>) -> R,
) -> Result<(SocketRef<G>, R), Errno> {
    loop {
        let so = {
            let mut hg = cq(head);
            if hg.qlen == 0 && head.error().is_none() {
                if head.so_rcv.has_state(SS_CANTRCVMORE) {
                    head.set_error(Some(Errno::ECONNABORTED));
                } else if nonblock {
                    return Err(Errno::EWOULDBLOCK);
                } else {
                    // The socket's locks released for the sleep, as `sosleep_nsec` releases
                    // them; the queue's mutex is the channel's interlock, let go by the
                    // sleep and before the socket's locks are taken again.
                    wh.unlocked(move || {
                        let r = hg.sleep_nsec(
                            head.timeo_chan(),
                            crate::net::sockbuf::PSOCK | PCATCH,
                            "netacc",
                            INFSLP,
                        );
                        drop(hg);
                        r
                    })?;
                    continue;
                }
            }
            if let Some(e) = head.error() {
                head.set_error(None);
                return Err(e);
            }
            match head.q.first(&hg) {
                Some(so) => soref(so),
                None => continue,
            }
        };
        // Do not sleep after we have taken the socket out of the queue.
        if so.is_inet() {
            kassert!(hal::sync::netlock::status() != 0);
        }
        let mut ws = SoLocked::take(&so, NetHold::None);
        let Some(queued) = soqremque(&so, Onq::Q) else {
            // Taken by another accepter or a close meanwhile: look again.
            drop(ws);
            continue;
        };
        // connection has been removed from the listen queue
        head.so_rcv.knote(0);
        // sounlock_nonet(head): the listener's so_lock let go, its net lock now the
        // connection's.
        ws.net = mem::replace(&mut wh.net, NetHold::None);
        drop(wh);
        let r = f(&so, &mut ws);
        drop(ws);
        drop(so);
        return Ok((queued, r));
    }
}

/// `SBLOCKWAIT(f)`.
const fn sblockwait(f: i32) -> i32 {
    if f & sysdefs::socket::MSG_DONTWAIT != 0 {
        0
    } else {
        crate::net::sockbuf::SBL_WAIT
    }
}

/// `sosendallatonce(so)`: do we have to send all at once on a socket?
#[inline]
pub fn sosendallatonce<G: ProcGlue>(so: &Socket<G>) -> bool {
    so.pr_flags(crate::net::proto::PR_ATOMIC)
}

/// `soissending(so)`: are we sending on this socket?
#[inline]
pub fn soissending<G: ProcGlue>(so: &Socket<G>) -> bool {
    so.so_snd.has_state(crate::net::sockbuf::SS_ISSENDING)
}

/// `isspliced(so)`: `so` sends its received data to a drain socket.
#[inline]
pub fn isspliced<G: ProcGlue>(so: &Socket<G>) -> bool {
    so.so_rcv.is_spliced()
}

/// `issplicedback(so)`: `so` is the drain of a spliced source socket.
#[inline]
pub fn issplicedback<G: ProcGlue>(so: &Socket<G>) -> bool {
    so.so_snd.is_spliced()
}

/// `soreadable(so)`: can we read something from `so`? (The socket and its receive buffer's
/// mutex are held by the caller, or the reads are the filters' own snapshot.)
pub fn soreadable<G: ProcGlue>(so: &Socket<G>) -> bool {
    if isspliced(so) {
        return false;
    }
    so.so_rcv.has_state(SS_CANTRCVMORE)
        || so.error().is_some()
        || so.so_rcv.cc() as i64 >= so.so_rcv.lowat()
}

/// `sowriteable(so)`: can we write something to `so`?
pub fn sowriteable<G: ProcGlue>(so: &Socket<G>) -> bool {
    (so.so_snd.sbspace() >= so.so_snd.lowat()
        && (so.has_state(SS_ISCONNECTED) || !so.pr_flags(crate::net::proto::PR_CONNREQUIRED)))
        || so.so_snd.has_state(SS_CANTSENDMORE)
        || so.error().is_some()
}

/// The `cmsg_type` of the control message at the start of `m`, 0 if `m` is too short.
fn cmsg_type_of(m: &Mbuf) -> i32 {
    hal::user::pod_from_bytes::<sysdefs::socket::Cmsghdr>(m.data()).map_or(0, |c| c.cmsg_type)
}

/// `sosend(so, addr, uio, top, control, flags)`: send on a socket. If send must go all at
/// once and message is larger than send buffering, then hard error. Lock against other
/// senders. If must go all at once and not enough room now, then inform user that this
/// would block and do nothing. Otherwise, if nonblocking, send as much as possible. The data
/// to be sent is described by `uio` if `Some`, otherwise by the mbuf chain `top` (which must
/// be `None` if `uio` is not). Data provided in mbuf chain must be small enough to send all
/// at once.
///
/// Returns an error on error, timeout or signal; callers must check for short counts if
/// `EINTR`/`ERESTART` are returned. Data and control buffers are freed on return.
pub fn sosend<G: ProcGlue>(
    so: &Socket<G>,
    addr: Option<&Mbuf>,
    mut uio: Option<&mut crate::file::uio::Uio<'_, G>>,
    mut top: Option<MbufBox>,
    mut control: Option<MbufBox>,
    flags: i32,
) -> Result<(), Errno> {
    use sysdefs::socket::{AF_UNIX, MSG_DONTWAIT, MSG_EOR, MSG_OOB, SCM_RIGHTS, SO_ZEROIZE};

    let mut clen: i64 = 0;
    let atomic = sosendallatonce(so) || top.is_some();

    let mut resid: usize = match &uio {
        Some(uio) => uio.uio_resid,
        None => top
            .as_deref()
            .and_then(Mbuf::pkthdr)
            .map_or(0, |h| h.len.get() as usize),
    };
    // MSG_EOR on a SOCK_STREAM socket is invalid.
    if so.so_type == sysdefs::socket::SOCK_STREAM && flags & MSG_EOR != 0 {
        return Err(Errno::EINVAL);
    }
    if let Some(uio) = &uio
        && let Some(p) = uio.uio_procp
    {
        p.p_ru.ru_msgsnd.set(p.p_ru.ru_msgsnd.get() + 1);
    }
    if let Some(c) = &control {
        // In theory clen should be unsigned (since control->m_len is). However, space must
        // be signed, as it might be less than 0 if we over-committed, and we must use a
        // signed comparison of space and clen.
        clen = i64::from(c.m_len().get());
        // reserve extra space for AF_UNIX's internalize
        let hdr = sysdefs::socket::cmsg_align(size_of::<sysdefs::socket::Cmsghdr>()) as i64;
        if so.family == i32::from(AF_UNIX) && clen >= hdr && cmsg_type_of(c) == SCM_RIGHTS {
            clen = sysdefs::socket::cmsg_space(
                (clen - hdr) as usize * (FDPASS_SIZE / size_of::<i32>()),
            ) as i64;
        }
    }

    'restart: loop {
        let sbl = so.so_snd.sblock(sblockwait(flags))?;
        let mut g = so.so_snd.lock();
        g.set_state(crate::net::sockbuf::SS_ISSENDING);
        let error: Result<(), Errno> = 'release: {
            loop {
                if so.so_snd.has_state(SS_CANTSENDMORE) {
                    break 'release Err(Errno::EPIPE);
                }
                if let Some(e) = so.error() {
                    so.set_error(None);
                    break 'release Err(e);
                }
                if !so.has_state(SS_ISCONNECTED) {
                    if so.pr_flags(crate::net::proto::PR_CONNREQUIRED) {
                        if !(resid == 0 && clen != 0) {
                            break 'release Err(Errno::ENOTCONN);
                        }
                    } else if addr.is_none() {
                        break 'release Err(Errno::EDESTADDRREQ);
                    }
                }
                let mut space = g.space();
                if flags & MSG_OOB != 0 {
                    space += 1024;
                }
                let hiwat = so.so_snd.hiwat() as i64;
                if so.family == i32::from(AF_UNIX) {
                    if atomic && resid as i64 > hiwat {
                        break 'release Err(Errno::EMSGSIZE);
                    }
                } else if clen > hiwat || (atomic && resid as i64 > hiwat - clen) {
                    break 'release Err(Errno::EMSGSIZE);
                }
                if space < clen
                    || (space - clen < resid as i64 && (atomic || space < so.so_snd.lowat()))
                {
                    if flags & MSG_DONTWAIT != 0 {
                        break 'release Err(Errno::EWOULDBLOCK);
                    }
                    drop(sbl);
                    let error = g.sbwait();
                    g.clear_state(crate::net::sockbuf::SS_ISSENDING);
                    drop(g);
                    error?;
                    continue 'restart;
                }
                space -= clen;
                loop {
                    match uio.as_deref_mut() {
                        None => {
                            // Data is prepackaged in "top".
                            resid = 0;
                            if flags & MSG_EOR != 0
                                && let Some(t) = &top
                            {
                                t.m_flags().set(t.flags() | crate::net::mbuf::M_EOR);
                            }
                        }
                        Some(uio) => {
                            drop(g);
                            let chain = m_getuio(atomic, space, uio);
                            g = so.so_snd.lock();
                            let t = match chain {
                                Ok(t) => t,
                                Err(e) => break 'release Err(e),
                            };
                            space -= i64::from(t.pkthdr().map_or(0, |h| h.len.get()));
                            resid = uio.uio_resid;
                            if flags & MSG_EOR != 0 {
                                t.m_flags().set(t.flags() | crate::net::mbuf::M_EOR);
                            }
                            top = Some(t);
                        }
                    }
                    if resid == 0 {
                        g.clear_state(crate::net::sockbuf::SS_ISSENDING);
                    }
                    if let Some(t) = &top
                        && so.has_options(SO_ZEROIZE)
                    {
                        t.m_flags().set(t.flags() | crate::net::mbuf::M_ZEROIZE);
                    }
                    drop(g);
                    let error = {
                        let mut w = solock_shared(so);
                        let (t, c) = (top.take(), control.take());
                        if flags & MSG_OOB != 0 {
                            so.usrreqs().sendoob(so, &mut w, t, addr, c)
                        } else {
                            so.usrreqs().send(so, &mut w, t, addr, c)
                        }
                    };
                    g = so.so_snd.lock();
                    clen = 0;
                    if let Err(e) = error {
                        break 'release Err(e);
                    }
                    if !(resid != 0 && space > 0) {
                        break;
                    }
                }
                if resid == 0 {
                    break;
                }
            }
            Ok(())
        };
        // release:
        g.clear_state(crate::net::sockbuf::SS_ISSENDING);
        drop(g);
        drop(sbl);
        // out: the data and the control not handed over are freed with `top` and `control`.
        return error;
    }
}

/// `m_getuio(mp, atomic, space, uio)`: copies at most `space` bytes of `uio` into a new
/// packet (a chain of mbufs and clusters); a datagram's first mbuf leaves room for the
/// protocol headers.
pub fn m_getuio<G: ProcGlue>(
    atomic: bool,
    space: i64,
    uio: &mut crate::file::uio::Uio<'_, G>,
) -> Result<MbufBox, Errno> {
    use crate::net::mbuf::{
        M_DONTWAIT, M_EXT, M_WAIT, MAX_HDR, MAXMCLBYTES, MCLBYTES, MHLEN, MINCLSIZE, MLEN, MT_DATA,
        m_align, m_get, m_gethdr, mclgetl,
    };

    let mut chain: hal::ds::owned::OwnedSlist<crate::net::mbuf::MNext> =
        hal::ds::owned::OwnedSlist::new();
    let mut resid = uio.uio_resid as u64;
    let mut space = space;
    let max_hdr = MAX_HDR.load(Ordering::Relaxed) as u64;

    loop {
        let first = chain.is_empty();
        let (m, mut mlen) = if first {
            (m_gethdr(M_WAIT, MT_DATA), MHLEN as u64)
        } else {
            (m_get(M_WAIT, MT_DATA), MLEN as u64)
        };
        let Some(mut m) = m else {
            return Err(Errno::ENOBUFS);
        };

        resid = resid.min(space as u64);
        let mut nopages = true;
        let mut len = 0;
        if resid >= MINCLSIZE as u64 {
            let _ = mclgetl(&mut m, M_DONTWAIT, resid.min(MAXMCLBYTES as u64) as u32);
            if m.flags() & M_EXT == 0 {
                let _ = mclgetl(&mut m, M_DONTWAIT, MCLBYTES as u32);
            }
            if m.flags() & M_EXT != 0 {
                nopages = false;
                mlen = u64::from(m.ext().map_or(0, crate::net::mbuf::Ext::ext_size));
                len = mlen.min(resid);
                // For datagram protocols, leave room for protocol headers in first mbuf.
                if atomic && first && len < mlen.wrapping_sub(max_hdr) {
                    m.data_add(max_hdr as isize);
                }
            }
        }
        if nopages {
            len = mlen.min(resid);
            // For datagram protocols, leave room for protocol headers in first mbuf.
            if atomic && first && len < mlen.wrapping_sub(max_hdr) {
                m_align(&m, len as i32);
            }
        }

        m.m_len().set(len as u32);
        let Some(buf) = m.data_mut() else {
            kpanic(format_args!("m_getuio: a fresh mbuf read-only"));
        };
        <G::FileGlue as crate::file::FileGlue>::uiomove(buf, uio)?;

        // adjust counters
        resid = uio.uio_resid as u64;
        space -= len as i64;
        chain.push_tail(m);
        if let Some(h) = chain.first().and_then(Mbuf::pkthdr) {
            h.len.set(h.len.get() + len as i32);
        }

        // Is there more space and more data?
        if !(space > 0 && resid > 0) {
            break;
        }
    }

    match chain.into_head() {
        Some(top) => Ok(top),
        None => kpanic(format_args!("m_getuio: no mbuf")),
    }
}

/// The record of the receive buffer `soreceive` reads, out of the buffer while it copies
/// (the C's `m` walking `sb_mb`'s record), and where its walk stands.
struct RcvRecord {
    /// What the read has not passed: the C's `m` is its first mbuf.
    rec: crate::net::sockbuf::SbRecord,
    /// `MSG_PEEK`'s walk: the mbufs the read has passed, in order, nothing freed or counted
    /// out (they go back in front of `rec` with [`RcvRecord::into_record`]).
    passed: hal::ds::owned::OwnedSlist<crate::net::mbuf::MNext>,
    /// `MSG_PEEK` has passed the record's last mbuf too (`m == NULL`); that mbuf stays in
    /// `rec`, the record's last, where the senders' continuation joins it.
    past_last: bool,
}

impl RcvRecord {
    /// A record a reader took out, nothing passed yet.
    fn new(rec: crate::net::sockbuf::SbRecord) -> Self {
        Self {
            rec,
            passed: hal::ds::owned::OwnedSlist::new(),
            past_last: false,
        }
    }

    /// The C's `m`: the mbuf the read stands on, O(1).
    fn m(&self) -> Option<&Mbuf> {
        if self.past_last {
            None
        } else {
            self.rec.first()
        }
    }

    /// `MSG_PEEK`'s `m = m->m_next`: the mbuf the read stands on passed, O(1).
    fn pass(&mut self) {
        if self.past_last {
            return;
        }
        match self.rec.pop_first_not_last() {
            Some(b) => self.passed.push_tail(b),
            None => self.past_last = true,
        }
    }

    /// `m_copym(m, 0, m->m_len, M_DONTWAIT)` of the mbuf the read stands on (`MSG_PEEK`'s
    /// copy of an address or a control message).
    fn copy_current(&mut self) -> Option<MbufBox> {
        if self.past_last {
            return None;
        }
        let len = self.rec.first()?.m_len().get() as i32;
        rcv_copy_front(&mut self.rec, len, crate::net::mbuf::M_DONTWAIT)
    }

    /// What the senders appended to the record while it was out moved to its end
    /// ([`crate::net::sockbuf::SbGuard::refill_record`]); a read past the record's last
    /// mbuf stands on the first one appended.
    fn refill(&mut self, g: &mut crate::net::sockbuf::SbGuard<'_, impl ProcGlue>) {
        g.refill_record(&mut self.rec);
        if self.past_last
            && let Some(b) = self.rec.pop_first_not_last()
        {
            self.passed.push_tail(b);
            self.past_last = false;
        }
    }

    /// The record whole again, the passed mbufs back in front, O(1).
    fn into_record(mut self) -> crate::net::sockbuf::SbRecord {
        self.rec.prepend(core::mem::take(&mut self.passed));
        self.rec
    }
}

/// `soreceive(so, paddr, uio, mp0, controlp, flagsp, controllen)`: implement receive
/// operations on a socket. We depend on the way that records are added to the sockbuf by
/// `sbappend*`. In particular, each record (mbufs linked through `m_next`) must begin with
/// an address if the protocol so specifies, followed by an optional mbuf or mbufs containing
/// ancillary data, and then zero or more mbufs of data. In order to avoid blocking network
/// for the entire time here, we release the `solock()` while doing the actual copy to user
/// space. Although the sockbuf is locked, new data may still be appended (see the record
/// taken out of the buffer, `kernel::net::sockbuf`).
///
/// The caller may receive the data as a single mbuf chain by supplying an `mp0` for use in
/// returning the chain. The uio is then used only for the count in `uio_resid`.
#[allow(clippy::too_many_arguments, reason = "the C's argument list")]
pub fn soreceive<G: ProcGlue>(
    so: &Socket<G>,
    mut paddr: Option<&mut Option<MbufBox>>,
    uio: &mut crate::file::uio::Uio<'_, G>,
    mp0: Option<&mut Option<MbufBox>>,
    controlp: Option<&mut Option<MbufBox>>,
    mut flagsp: Option<&mut i32>,
    controllen: u32,
) -> Result<(), Errno> {
    use crate::net::mbuf::{M_BCAST, M_EOR, M_MCAST, M_WAIT, MT_CONTROL, MT_OOBDATA};
    use crate::net::proto::{PR_ADDR, PR_ATOMIC, PR_CONNREQUIRED, PR_WANTRCVD};
    use sysdefs::socket::{
        MSG_BCAST, MSG_DONTWAIT, MSG_EOR, MSG_MCAST, MSG_OOB, MSG_PEEK, MSG_TRUNC, MSG_WAITALL,
        SCM_RIGHTS,
    };

    let pr = so.proto;
    let mut orig_resid = uio.uio_resid;
    let mut uio_error: Result<(), Errno> = Ok(());

    if let Some(p) = paddr.as_deref_mut() {
        *p = None;
    }
    let want_control = controlp.is_some();
    let mut controls: hal::ds::owned::OwnedSlist<crate::net::mbuf::MNext> =
        hal::ds::owned::OwnedSlist::new();
    let want_mp = mp0.is_some();
    let mut mpchain: hal::ds::owned::OwnedSlist<crate::net::mbuf::MNext> =
        hal::ds::owned::OwnedSlist::new();
    let mut flags = match flagsp.as_deref() {
        Some(f) => *f & !MSG_EOR,
        None => 0,
    };
    if flags & MSG_OOB != 0 {
        let Some(mut m) = crate::net::mbuf::m_get(M_WAIT, crate::net::mbuf::MT_DATA) else {
            return Err(Errno::ENOBUFS);
        };
        let mut error = {
            let mut w = solock_shared(so);
            so.usrreqs().rcvoob(so, &mut w, &mut m, flags & MSG_PEEK)
        };
        let mut m = Some(m);
        if error.is_ok() {
            while let Some(mut mm) = m {
                let len = uio.uio_resid.min(mm.m_len().get() as usize);
                error = <G::FileGlue as crate::file::FileGlue>::uiomove_out(&mm.data()[..len], uio);
                m = mm.m_next.take();
                drop(mm);
                if !(uio.uio_resid != 0 && error.is_ok()) {
                    break;
                }
            }
        }
        // bad:
        drop(m);
        return error;
    }

    let result: Result<(), Errno> = 'restart: loop {
        let sbl = so.so_rcv.sblock(sblockwait(flags))?;
        let mut g = so.so_rcv.lock();

        // If we have less data than requested, block awaiting more (subject to any timeout)
        // if:
        //   1. the current count is less than the low water mark,
        //   2. MSG_WAITALL is set, and it is possible to do the entire receive operation at
        //      once if we block (resid <= hiwat), or
        //   3. MSG_DONTWAIT is not set.
        // If MSG_WAITALL is set but resid is larger than the receive buffer, we have to do
        // the receive in sections, and thus risk returning a short count if a timeout or
        // signal occurs after we start.
        let has_m = !isspliced(so) && !g.data().is_empty();
        let sb_cc = so.so_rcv.cc();
        let must_wait = !has_m
            || ((flags & MSG_DONTWAIT == 0 && (sb_cc as usize) < uio.uio_resid)
                && ((sb_cc as i64) < so.so_rcv.lowat()
                    || (flags & MSG_WAITALL != 0 && uio.uio_resid as u64 <= so.so_rcv.hiwat()))
                && !g.data().has_next_record()
                && pr.flags() & PR_ATOMIC == 0);
        if must_wait {
            'dontblock: {
                if cfg!(feature = "diagnostic") && !has_m && sb_cc != 0 && !isspliced(so) {
                    kpanic(format_args!(
                        "receive 1: so {so:p}, so_type {}, sb_cc {sb_cc}",
                        so.so_type
                    ));
                }
                if let Some(error2) = so.error() {
                    if has_m {
                        break 'dontblock;
                    }
                    if flags & MSG_PEEK == 0 {
                        so.set_error(None);
                    }
                    break 'restart Err(error2);
                }
                if so.so_rcv.has_state(SS_CANTRCVMORE) {
                    if has_m {
                        break 'dontblock;
                    } else if so.so_rcv.cc() == 0 {
                        break 'restart Ok(());
                    }
                }
                if has_m
                    && g.data().first_record().any(|m| {
                        i32::from(m.m_type().get()) == MT_OOBDATA || m.flags() & M_EOR != 0
                    })
                {
                    break 'dontblock;
                }
                if !so.has_state(SS_ISCONNECTED | SS_ISCONNECTING)
                    && pr.flags() & PR_CONNREQUIRED != 0
                {
                    break 'restart Err(Errno::ENOTCONN);
                }
                if uio.uio_resid == 0 && !want_control {
                    break 'restart Ok(());
                }
                if flags & MSG_DONTWAIT != 0 {
                    break 'restart Err(Errno::EWOULDBLOCK);
                }

                drop(sbl);
                let error = g.sbwait();
                drop(g);
                error?;
                continue 'restart;
            }
        }
        // dontblock: the first record comes out of the buffer for the copies; what is left
        // of it goes back at the end (`put_back_record`).
        if let Some(p) = uio.uio_procp {
            p.p_ru.ru_msgrcv.set(p.p_ru.ru_msgrcv.get() + 1);
        }
        let Some(rec) = g.take_first_record() else {
            kpanic(format_args!("soreceive: no record"));
        };
        let mut r = RcvRecord::new(rec);
        let mut error: Result<(), Errno> = Ok(());

        if pr.flags() & PR_ADDR != 0 {
            if cfg!(feature = "diagnostic")
                && r.m()
                    .is_none_or(|m| i32::from(m.m_type().get()) != crate::net::mbuf::MT_SONAME)
            {
                kpanic(format_args!(
                    "receive 1a: so {so:p}, so_type {}",
                    so.so_type
                ));
            }
            orig_resid = 0;
            if flags & MSG_PEEK != 0 {
                if let Some(p) = paddr.as_deref_mut() {
                    *p = r.copy_current();
                }
                r.pass();
            } else if let Some(a) = r.rec.pop_first() {
                g.sbfree(&a);
                match paddr.as_deref_mut() {
                    Some(p) => *p = Some(a),
                    None => drop(a),
                }
            }
        }
        while let Some(cmtype) = r
            .m()
            .filter(|m| i32::from(m.m_type().get()) == MT_CONTROL)
            .map(cmsg_type_of)
            && error.is_ok()
        {
            let mut skip = false;
            if flags & MSG_PEEK != 0 {
                if cmtype == SCM_RIGHTS {
                    // don't leak internalized SCM_RIGHTS msgs
                    skip = true;
                } else if want_control && let Some(c) = r.copy_current() {
                    controls.push_tail(c);
                }
                r.pass();
            } else {
                let Some(mut cm) = r.rec.pop_first() else {
                    break;
                };
                g.sbfree(&cm);
                let rights = pr.domain().rights();
                if want_control {
                    if let Some(rights) = rights {
                        drop(g);
                        error = rights.externalize(&mut cm, controllen, flags);
                        g = so.so_rcv.lock();
                    }
                    controls.push_tail(cm);
                } else {
                    // Dispose of any SCM_RIGHTS message that went through the read path
                    // rather than recv.
                    if let Some(rights) = rights {
                        drop(g);
                        rights.dispose(&mut cm);
                        g = so.so_rcv.lock();
                    }
                    drop(cm);
                }
            }
            let _ = skip;
            orig_resid = 0;
        }

        // If m is non-NULL, we have some data to read.
        let mut type_ = 0;
        if let Some(m) = r.m() {
            type_ = i32::from(m.m_type().get());
            if type_ == MT_OOBDATA {
                flags |= MSG_OOB;
            }
            if m.flags() & M_BCAST != 0 {
                flags |= MSG_BCAST;
            }
            if m.flags() & M_MCAST != 0 {
                flags |= MSG_MCAST;
            }
        }

        let mut moff: u64 = 0;
        let mut offset: u64 = 0;
        loop {
            if r.m().is_none() {
                // The record ran out: what a sender appended to it meanwhile continues it.
                r.refill(&mut g);
            }
            let Some(m) = r.m() else {
                break;
            };
            if !(uio.uio_resid > 0 && error.is_ok()) {
                break;
            }
            let mtype = i32::from(m.m_type().get());
            if mtype == MT_OOBDATA {
                if type_ != MT_OOBDATA {
                    break;
                }
            } else if type_ == MT_OOBDATA || mtype == MT_CONTROL {
                // If there is more than one control message in the stream, we do a short
                // read. Next can be received or disposed by another system call.
                break;
            }
            g.clear_state(crate::net::sockbuf::SS_RCVATMARK);
            let mut len = uio.uio_resid as u64;
            let oobmark = so.oobmark();
            if oobmark != 0 && len > oobmark - offset {
                len = oobmark - offset;
            }
            let mlen = u64::from(m.m_len().get());
            if len > mlen - moff {
                len = mlen - moff;
            }
            // If mp is set, just pass back the mbufs. Otherwise copy them out via the uio,
            // then free. The record is out of the buffer, so senders may append to the
            // buffer meanwhile.
            if !want_mp && uio_error.is_ok() {
                let resid = uio.uio_resid;
                drop(g);
                let m = r
                    .m()
                    .unwrap_or_else(|| kpanic(format_args!("soreceive: no mbuf")));
                uio_error = <G::FileGlue as crate::file::FileGlue>::uiomove_out(
                    &m.data()[moff as usize..(moff + len) as usize],
                    uio,
                );
                g = so.so_rcv.lock();
                if uio_error.is_err() {
                    uio.uio_resid = resid - len as usize;
                }
            } else {
                uio.uio_resid -= len as usize;
            }
            let m = r
                .m()
                .unwrap_or_else(|| kpanic(format_args!("soreceive: no mbuf")));
            if len == u64::from(m.m_len().get()) - moff {
                if m.flags() & M_EOR != 0 {
                    flags |= MSG_EOR;
                }
                if flags & MSG_PEEK != 0 {
                    r.pass();
                    moff = 0;
                    orig_resid = 0;
                } else if let Some(b) = r.rec.pop_first() {
                    g.sbfree(&b);
                    if want_mp {
                        mpchain.push_tail(b);
                    } else {
                        drop(b);
                    }
                }
            } else if flags & MSG_PEEK != 0 {
                moff += len;
                orig_resid = 0;
            } else {
                if want_mp {
                    // The copy may sleep for memory: the mutex is let go, the record stays
                    // out.
                    drop(g);
                    if let Some(c) = rcv_copy_front(&mut r.rec, len as i32, M_WAIT) {
                        mpchain.push_tail(c);
                    }
                    g = so.so_rcv.lock();
                }
                let m = r
                    .m()
                    .unwrap_or_else(|| kpanic(format_args!("soreceive: no mbuf")));
                m.data_add(len as isize);
                m.m_len().set(m.m_len().get() - len as u32);
                g.sub_data(m, len);
            }
            if so.oobmark() != 0 {
                if flags & MSG_PEEK == 0 {
                    so.set_oobmark(so.oobmark() - len);
                    if so.oobmark() == 0 {
                        g.set_state(crate::net::sockbuf::SS_RCVATMARK);
                        break;
                    }
                } else {
                    offset += len;
                    if offset == so.oobmark() {
                        break;
                    }
                }
            }
            if flags & MSG_EOR != 0 {
                break;
            }
            // If the MSG_WAITALL flag is set (for non-atomic socket), we must not quit
            // until "uio->uio_resid == 0" or an error termination. If a signal/timeout
            // occurs, return with a short count but without error. Keep sockbuf locked
            // against other readers.
            while flags & MSG_WAITALL != 0
                && r.m().is_none()
                && uio.uio_resid > 0
                && !sosendallatonce(so)
                && !g.next_record_exists()
            {
                r.refill(&mut g);
                if r.m().is_some() {
                    break;
                }
                if so.so_rcv.has_state(SS_CANTRCVMORE) || so.error().is_some() {
                    break;
                }
                if g.sbwait().is_err() {
                    g.put_back_record(r.into_record());
                    drop(g);
                    drop(sbl);
                    soreceive_out(mp0, mpchain, controlp, controls);
                    return Ok(());
                }
                if flags & MSG_PEEK != 0 {
                    // `m = so->so_rcv.sb_mb`: after the wait the C reads again from the
                    // buffer's start, so a peek copies what it peeked once more.
                    g.put_back_record(r.into_record());
                    let Some(rec) = g.take_first_record() else {
                        kpanic(format_args!("soreceive: the peeked record is gone"));
                    };
                    r = RcvRecord::new(rec);
                    moff = 0;
                }
            }
        }

        let data_left = r.m().is_some();
        g.put_back_record(r.into_record());
        if data_left && pr.flags() & PR_ATOMIC != 0 {
            flags |= MSG_TRUNC;
            if flags & MSG_PEEK == 0 {
                g.sbdroprecord();
            }
        }
        if flags & MSG_PEEK == 0 && pr.flags() & PR_WANTRCVD != 0 {
            drop(g);
            {
                let mut w = solock_shared(so);
                so.usrreqs().rcvd(so, &mut w);
            }
            g = so.so_rcv.lock();
        }
        if orig_resid == uio.uio_resid
            && orig_resid != 0
            && flags & MSG_EOR == 0
            && !so.so_rcv.has_state(SS_CANTRCVMORE)
        {
            drop(g);
            drop(sbl);
            continue 'restart;
        }

        if uio_error.is_err() {
            error = uio_error;
        }

        if let Some(f) = flagsp.as_deref_mut() {
            *f |= flags;
        }
        break 'restart error;
    };
    // release: the guard and sblock went with the loop's locals.
    soreceive_out(mp0, mpchain, controlp, controls);
    result
}

/// `soreceive`'s out-parameters written: the data chain (`*mp0`) and the control messages
/// (`*controlp`), each a chain by `m_next`.
fn soreceive_out(
    mp0: Option<&mut Option<MbufBox>>,
    mpchain: hal::ds::owned::OwnedSlist<crate::net::mbuf::MNext>,
    controlp: Option<&mut Option<MbufBox>>,
    controls: hal::ds::owned::OwnedSlist<crate::net::mbuf::MNext>,
) {
    if let Some(mp) = mp0 {
        *mp = mpchain.into_head();
    }
    if let Some(cp) = controlp {
        *cp = controls.into_head();
    }
}

/// `*mp = m_copym(m, 0, len, wait)` of the record's first mbuf (`soreceive`'s partial
/// mbuf passed back as a chain; its whole length, `MSG_PEEK`'s copy of an address or a
/// control message), the mbuf left at the record's front.
fn rcv_copy_front(rec: &mut crate::net::sockbuf::SbRecord, len: i32, wait: i32) -> Option<MbufBox> {
    let mut m = rec.pop_first()?;
    let c = crate::net::mbuf::m_copym(&mut m, 0, len, wait);
    rec.push_first(m);
    c
}

/// `soshutdown(so, how)`: `SHUT_RD` flushes the receive side, `SHUT_WR` tells the protocol
/// no more data will be sent, `SHUT_RDWR` both.
pub fn soshutdown<G: ProcGlue>(so: &Socket<G>, how: i32) -> Result<(), Errno> {
    use sysdefs::socket::{SHUT_RD, SHUT_RDWR, SHUT_WR};

    let shut_wr = |so: &Socket<G>| {
        let mut w = solock_shared(so);
        so.usrreqs().shutdown(so, &mut w)
    };

    match how {
        SHUT_RD => {
            sorflush(so);
            Ok(())
        }
        SHUT_RDWR => {
            sorflush(so);
            shut_wr(so)
        }
        SHUT_WR => shut_wr(so),
        _ => Err(Errno::EINVAL),
    }
}

/// `sorflush(so)`: no more data will be received: empty the receive buffer, disposing of the
/// rights in it.
pub fn sorflush<G: ProcGlue>(so: &Socket<G>) {
    let sbl = so
        .so_rcv
        .sblock(crate::net::sockbuf::SBL_WAIT | crate::net::sockbuf::SBL_NOINTR);
    // with SBL_WAIT and SLB_NOINTR sblock() must not fail
    kassert!(sbl.is_ok());

    {
        let _w = solock_shared(so);
        socantrcvmore(so);
    }
    let m = {
        let mut g = so.so_rcv.lock();
        let m = g.take_all_zeroed();
        g.set_timeo_nsecs(INFSLP);
        m
    };
    drop(sbl);

    crate::net::domain::dispose_and_purge(so.proto, m);
}

/// `SOSP_FREEING_READ`: `sounsplice` must not wake the source, it is being freed.
const SOSP_FREEING_READ: i32 = 1;
/// `SOSP_FREEING_WRITE`: `sounsplice` must not wake the drain, it is being freed.
const SOSP_FREEING_WRITE: i32 = 2;

impl<G: ProcGlue> Sosplice<G> {
    /// `ssp_socket`, the drain, under the source's `so_rcv` mutex.
    fn drain(&self, g: &crate::net::sockbuf::SbGuard<'_, G>) -> Option<SocketRef<G>> {
        self.socket.get(g.witness_ref()).clone()
    }

    /// `ssp_soback`, the source, under the drain's `so_snd` mutex.
    fn source(&self, g: &crate::net::sockbuf::SbGuard<'_, G>) -> Option<SocketRef<G>> {
        self.soback.get(g.witness_ref()).clone()
    }

    /// `ssp_len`: the bytes spliced so far.
    pub fn spliced_len(&self) -> Off {
        self.len.load(Ordering::Relaxed)
    }

    /// `timerisset(&ssp_idletv)`'s value, in nanoseconds (0: no idle timeout).
    fn idle_nsec(&self) -> u64 {
        let tv = Timeval::new(
            self.idle_sec.load(Ordering::Relaxed),
            self.idle_usec.load(Ordering::Relaxed),
        );
        if tv.tv_sec == 0 && tv.tv_usec == 0 {
            0
        } else {
            tv.to_nsec()
        }
    }
}

/// `sosplice_alloc`: `so`'s splice state, from `sosppl` (`PR_WAITOK | PR_ZERO`), its idle
/// timeout and its task set with the socket (the C's `arg = so`, counted: [`soclose`]
/// clears both after their barriers).
fn sosplice_alloc<G: ProcGlue>(so: &Socket<G>) -> Option<&Sosplice<G>> {
    let sp = net::<G>()
        .sosppl
        .get(hal::mem::pool::PR_WAITOK, || Sosplice {
            socket: Guarded::new_mutexed(so.so_rcv.mutex(), None),
            soback: Guarded::new_mutexed(so.so_snd.mutex(), None),
            len: AtomicI64::new(0),
            max: AtomicI64::new(0),
            idle_sec: AtomicI64::new(0),
            idle_usec: AtomicI64::new(0),
            idleto: Timeout::new(noop_timeout, core::ptr::null_mut()),
            task: Task::new(noop_task, core::ptr::null_mut()),
            queue: hal::ds::link::SyncLink::new(),
        })?;
    sp.idleto.set_shared(
        soidle::<G>,
        PoolArc::erase(soref(so)),
        crate::time::timeout::KCLOCK_NONE,
        crate::time::timeout::TIMEOUT_PROC | crate::time::timeout::TIMEOUT_MPSAFE,
    );
    sp.task.set_shared(sotask::<G>, PoolArc::erase(soref(so)));
    // Two allocations racing for one socket cannot happen: the caller holds the socket's
    // lock; a second set is refused and its box freed.
    let _ = so.sp.set(sp);
    so.splice()
}

/// A timeout's placeholder function before `set_shared`.
fn noop_timeout(_arg: *mut core::ffi::c_void) {}

/// A task's placeholder function before `set_shared`.
fn noop_task(_arg: *mut core::ffi::c_void) {}

/// The socket a splice task or timeout was set with.
fn spliced_socket<G: ProcGlue>(arg: &AnyArc) -> &Socket<G> {
    match arg.downcast_ref::<Socket<G>>() {
        Some(so) => so,
        None => kpanic(format_args!("splice: an argument that is no socket")),
    }
}

/// `sosplice(so, fd, max, tv)`: splices `so`'s received data into the socket the
/// descriptor `fd` of `p` names (at most `max` bytes, unspliced after `tv` idle); `fd` < 0
/// unsplices.
pub fn sosplice<G: ProcGlue>(
    so: &Socket<G>,
    p: &Thread<G>,
    fd: i32,
    max: Off,
    tv: Option<&Timeval>,
) -> Result<(), Errno> {
    if !so.pr_flags(PR_SPLICE) {
        return Err(Errno::EPROTONOSUPPORT);
    }
    if max != 0 && max < 0 {
        return Err(Errno::EINVAL);
    }
    if let Some(tv) = tv
        && (tv.tv_sec < 0 || !tv.is_valid())
    {
        return Err(Errno::EINVAL);
    }

    // If no fd is given, unsplice by removing existing link.
    if fd < 0 {
        let rl = so.so_rcv.sblock(crate::net::sockbuf::SBL_WAIT)?;
        let sl = so.so_snd.sblock(crate::net::sockbuf::SBL_WAIT)?;
        let drain = so.splice().and_then(|sp| sp.drain(&so.so_rcv.lock()));
        let error = match drain {
            Some(drain) => {
                sounsplice(so, &drain, 0);
                Ok(())
            }
            None => Err(Errno::EPROTO),
        };
        drop(sl);
        drop(rl);
        return error;
    }

    // Find sosp, the drain socket where data will be spliced into.
    let Some(fp) = crate::file::descrip::fd_getfile(p.fd(), fd) else {
        return Err(Errno::EBADF);
    };
    let Some(sosp) = fp
        .data()
        .and_then(<G::FileGlue as crate::file::FileGlue>::socket_of)
        .cloned()
    else {
        return Err(Errno::ENOTSOCK);
    };

    if sosp.usrreqs().send_id() != so.usrreqs().send_id() {
        return Err(Errno::EPROTONOSUPPORT);
    }

    let rl = so.so_rcv.sblock(crate::net::sockbuf::SBL_WAIT)?;
    let sl = sosp.so_snd.sblock(crate::net::sockbuf::SBL_WAIT)?;
    let r = pair_netlock(so);
    let net = r.as_ref().and_then(NetHeld::of);
    let (mut w1, w2) = solock_pair(so, &sosp, net.as_ref());

    let error: Result<(), Errno> = 'release: {
        if so.has_options(SO_ACCEPTCONN) || sosp.has_options(SO_ACCEPTCONN) {
            break 'release Err(Errno::EOPNOTSUPP);
        }
        if !so.has_state(SS_ISCONNECTED | SS_ISCONNECTING)
            && so.pr_flags(crate::net::proto::PR_CONNREQUIRED)
        {
            break 'release Err(Errno::ENOTCONN);
        }
        if !sosp.has_state(SS_ISCONNECTED | SS_ISCONNECTING) {
            break 'release Err(Errno::ENOTCONN);
        }
        let Some(sp) = so.splice().or_else(|| sosplice_alloc(so)) else {
            break 'release Err(Errno::ENOBUFS);
        };
        let Some(sosp_sp) = sosp.splice().or_else(|| sosplice_alloc(&sosp)) else {
            break 'release Err(Errno::ENOBUFS);
        };
        if isspliced(so) || issplicedback(&sosp) {
            break 'release Err(Errno::EBUSY);
        }

        sp.len.store(0, Ordering::Relaxed);
        sp.max.store(max, Ordering::Relaxed);
        let (s, u) = tv.map_or((0, 0), |tv| (tv.tv_sec, tv.tv_usec));
        sp.idle_sec.store(s, Ordering::Relaxed);
        sp.idle_usec.store(u, Ordering::Relaxed);

        // To prevent sorwakeup() calling somove() before this somove() has finished, the
        // socket buffers are not marked as spliced yet.

        // Splice so and sosp together.
        {
            let mut gr = so.so_rcv.lock();
            let mut gs = sosp.so_snd.lock();
            *sp.socket.get_mut(gr.witness()) = Some(sosp.clone());
            gr.set_spliced(true);
            *sosp_sp.soback.get_mut(gs.witness()) = Some(soref(so));
            gs.set_spliced(true);
            sp.queue
                .set(Some(net_tq(so.usrreqs().flowid(so, &mut w1) as u32)));
        }

        drop(w2);
        drop(w1);
        drop(sl);

        if somove(so, crate::net::mbuf::M_WAIT) {
            let mut gr = so.so_rcv.lock();
            let mut gs = sosp.so_snd.lock();
            gr.set_flags(SB_SPLICE);
            gs.set_flags(SB_SPLICE);
        }

        drop(rl);
        return Ok(());
    };
    // release:
    drop(w2);
    drop(w1);
    drop(sl);
    drop(rl);
    error
}

/// `net_tq(flowid)`: the softnet task queue of a flow (`kernel::net::if_`'s); the system's
/// own queue before `softnet_init` made them (the host tests' sockets).
fn net_tq(flowid: u32) -> &'static crate::base::task::Taskq {
    match crate::net::if_::net_tq(flowid) {
        Some(tq) => tq,
        None => &crate::base::task::SYSTQMP,
    }
}

/// `sounsplice(so, sosp, freeing)`: dissolves the splice of `so` into `sosp`, waking the
/// sockets that are not being freed (`SOSP_FREEING_*`). The caller holds `sblock(so_rcv)`.
pub fn sounsplice<G: ProcGlue>(so: &Socket<G>, sosp: &Socket<G>, freeing: i32) {
    let (Some(sp), Some(sosp_sp)) = (so.splice(), sosp.splice()) else {
        kpanic(format_args!("sounsplice: not spliced"));
    };

    let (drain, source) = {
        let mut gr = so.so_rcv.lock();
        let mut gs = sosp.so_snd.lock();
        gr.clear_flags(SB_SPLICE);
        gs.clear_flags(SB_SPLICE);
        sp.idleto.del();
        if let Some(q) = sp.queue.get() {
            crate::base::task::task_del(q, &sp.task);
        }
        let drain = sp.socket.get_mut(gr.witness()).take();
        let source = sosp_sp.soback.get_mut(gs.witness()).take();
        kassert!(
            drain
                .as_ref()
                .is_some_and(|d| core::ptr::eq(core::ptr::from_ref(&**d), sosp))
        );
        kassert!(
            source
                .as_ref()
                .is_some_and(|s| core::ptr::eq(core::ptr::from_ref(&**s), so))
        );
        gr.set_spliced(false);
        gs.set_spliced(false);
        (drain, source)
    };

    // Do not wakeup a socket that is about to be freed.
    if freeing & SOSP_FREEING_READ == 0 {
        let _w = solock_shared(so);
        let readable = {
            let _g = so.so_rcv.lock();
            so.qlen() != 0 || soreadable(so)
        };
        if readable {
            sorwakeup(so);
        }
    }
    if freeing & SOSP_FREEING_WRITE == 0 {
        let _w = solock_shared(sosp);
        if sowriteable(sosp) {
            sowwakeup(sosp);
        }
    }

    // sorele(sosp); sorele(so): the splice's references, after the locks.
    drop(drain);
    drop(source);
}

/// `soidle(arg)`: the splice idle timeout of the socket: unsplice with `ETIMEDOUT`.
fn soidle<G: ProcGlue>(arg: &AnyArc) {
    let so = spliced_socket::<G>(arg);
    let Ok(rl) = so
        .so_rcv
        .sblock(crate::net::sockbuf::SBL_WAIT | crate::net::sockbuf::SBL_NOINTR)
    else {
        return;
    };
    let drain = so.splice().and_then(|sp| sp.drain(&so.so_rcv.lock()));
    if so.so_rcv.has_flags(SB_SPLICE)
        && let Some(drain) = drain
    {
        so.set_error(Some(Errno::ETIMEDOUT));
        sounsplice(so, &drain, 0);
    }
    drop(rl);
}

/// `sotask(arg)`: the splice task of the socket: move what arrived.
fn sotask<G: ProcGlue>(arg: &AnyArc) {
    let so = spliced_socket::<G>(arg);
    let Ok(rl) = so
        .so_rcv
        .sblock(crate::net::sockbuf::SBL_WAIT | crate::net::sockbuf::SBL_NOINTR)
    else {
        return;
    };
    if so.so_rcv.has_flags(SB_SPLICE) {
        let _ = somove(so, crate::net::mbuf::M_DONTWAIT);
    }
    drop(rl);
}

/// `soclose`'s splice part: unsplice from the source and into the drain, then wait for the
/// idle timeout and the tasks, and break the cycle the task and the timeout hold
/// (`set_shared` of the socket's own reference).
fn soclose_unsplice<G: ProcGlue>(so: &Socket<G>, sp: &Sosplice<G>) {
    // Concurrent sounsplice() locks `sb_mtx' mutexes on both `so_snd' and `so_rcv' before
    // unsplice sockets.
    let soback = sp.source(&so.so_snd.lock());

    if let Some(soback) = &soback {
        // `so' can be only unspliced, and never spliced again. Thus if issplicedback(so)
        // check is positive, socket is still spliced and `ssp_soback' points to the same
        // socket that `soback'.
        let rl = soback
            .so_rcv
            .sblock(crate::net::sockbuf::SBL_WAIT | crate::net::sockbuf::SBL_NOINTR);
        if issplicedback(so) {
            let mut freeing = SOSP_FREEING_WRITE;
            if core::ptr::eq(core::ptr::from_ref(&**soback), so) {
                freeing |= SOSP_FREEING_READ;
            }
            sounsplice(soback, so, freeing);
        }
        drop(rl);
    }

    // notsplicedback:
    let rl = so
        .so_rcv
        .sblock(crate::net::sockbuf::SBL_WAIT | crate::net::sockbuf::SBL_NOINTR);
    let drain = sp.drain(&so.so_rcv.lock());
    if isspliced(so)
        && let Some(drain) = drain
    {
        let mut freeing = SOSP_FREEING_READ;
        if core::ptr::eq(so, core::ptr::from_ref(&*drain)) {
            freeing |= SOSP_FREEING_WRITE;
        }
        sounsplice(so, &drain, freeing);
    }
    drop(rl);

    sp.idleto.barrier();
    let spq = sp.queue.get();
    if let Some(spq) = spq {
        crate::base::task::taskq_barrier(spq);
    }
    if let Some(soback) = soback {
        let spqback = soback.splice().and_then(|bsp| bsp.queue.get());
        if let Some(spqback) = spqback
            && !spq.is_some_and(|spq| core::ptr::eq(spq, spqback))
        {
            crate::base::task::taskq_barrier(spqback);
        }
        drop(soback);
    }
    // The task and the timeout hold the socket: neither runs any more.
    sp.idleto.clear();
    sp.task.clear();
}

/// `somove(so, wait)`: move data from receive buffer of spliced source socket to send buffer
/// of drain socket. Try to move as much as possible in one big chunk. It is a TCP only
/// implementation. Returns `false` when splicing has been finished, `true` to continue. The
/// caller holds `sblock(so_rcv)`.
pub fn somove<G: ProcGlue>(so: &Socket<G>, wait: i32) -> bool {
    use crate::net::mbuf::{
        M_BCAST, M_LOOP, M_MAXLOOP, M_MCAST, M_PKTHDR, MT_CONTROL, MT_DATA, m_adj, m_get,
        m_resethdr, m_split,
    };
    use crate::net::proto::{PR_ADDR, PR_ATOMIC, PR_WANTRCVD};
    use sysdefs::socket::SO_OOBINLINE;

    let Some(sp) = so.splice() else {
        kpanic(format_args!("somove: so {so:p} not spliced"));
    };
    let Some(sosp) = sp.drain(&so.so_rcv.lock()) else {
        kpanic(format_args!("somove: so {so:p} not spliced"));
    };
    let mut error: Option<Errno>;
    let mut maxreached = false;

    let sl = if so.pr_flags(PR_WANTRCVD) {
        so.so_snd
            .sblock(crate::net::sockbuf::SBL_WAIT | crate::net::sockbuf::SBL_NOINTR)
            .ok()
    } else {
        None
    };

    let mut gr = so.so_rcv.lock();
    let mut gs = sosp.so_snd.lock();

    // `pru_rcvd(so)` with both mutexes let go (`mtx_leave` .. `mtx_enter` around it).
    macro_rules! rcvd_unlocked {
        () => {{
            drop(gs);
            drop(gr);
            {
                let mut w = solock_shared(so);
                so.usrreqs().rcvd(so, &mut w);
            }
            gr = so.so_rcv.lock();
            gs = sosp.so_snd.lock();
        }};
    }

    // nextpkt:
    'release: loop {
        if let Some(e) = so.error() {
            error = Some(e);
            break 'release;
        }
        if sosp.so_snd.has_state(SS_CANTSENDMORE) {
            error = Some(Errno::EPIPE);
            break 'release;
        }

        error = sosp.error();
        if let Some(e) = error {
            if e != Errno::ETIMEDOUT && e != Errno::EFBIG && e != Errno::ELOOP {
                break 'release;
            }
            error = None;
        }
        if !sosp.has_state(SS_ISCONNECTED) {
            break 'release;
        }

        // Calculate how many bytes can be copied now.
        let mut len = so.so_rcv.datacc();
        let (splen, spmax) = (sp.spliced_len(), sp.max.load(Ordering::Relaxed));
        if spmax != 0 {
            kassert!(splen < spmax);
            if spmax <= splen + len as Off {
                len = (spmax - splen) as u64;
                maxreached = true;
            }
        }
        let mut space = gs.space();
        let oobmark = so.oobmark();
        if oobmark != 0 && oobmark < len && oobmark < (space + 1024) as u64 {
            space += 1024;
        }
        if space <= 0 {
            maxreached = false;
            break 'release;
        }
        if (space as u64) < len {
            maxreached = false;
            if space < sosp.so_snd.lowat() {
                break 'release;
            }
            len = space as u64;
        }
        gs.set_state(crate::net::sockbuf::SS_ISSENDING);

        let Some(mut rec) = gr.take_first_record() else {
            break 'release;
        };
        let nextrecord = gr.next_record_exists();

        // Drop address and control information not used with splicing: the first data
        // mbuf, looked at before anything is taken off.
        let skip = usize::from(so.pr_flags(PR_ADDR))
            + rec
                .iter()
                .skip(usize::from(so.pr_flags(PR_ADDR)))
                .take_while(|m| i32::from(m.m_type().get()) == MT_CONTROL)
                .count();
        let Some((dflags, loopcnt, pktlen)) = rec.iter().nth(skip).map(|d| {
            (
                d.flags(),
                d.pkthdr().map_or(0, |h| h.ph_loopcnt.get()),
                d.pkthdr().map(|h| h.len.get() as u64),
            )
        }) else {
            // No data in the record: it goes whole.
            while let Some(b) = rec.pop_first() {
                gr.sbfree(&b);
            }
            gr.put_back_record(rec);
            if so.pr_flags(PR_WANTRCVD) {
                rcvd_unlocked!();
            }
            continue 'release; // goto nextpkt
        };

        // By splicing sockets connected to localhost, userland might create a loop.
        // Dissolve splicing with error if loop is detected by counter.
        if dflags & M_PKTHDR != 0
            && (loopcnt >= M_MAXLOOP || (dflags & M_LOOP != 0 && dflags & (M_BCAST | M_MCAST) != 0))
        {
            gr.put_back_record(rec);
            error = Some(Errno::ELOOP);
            break 'release;
        }

        if so.pr_flags(PR_ATOMIC) {
            let Some(pktlen) = pktlen else {
                kpanic(format_args!(
                    "somove !PKTHDR: so {so:p}, so_type {}",
                    so.so_type
                ));
            };
            if sosp.so_snd.hiwat() < pktlen {
                gr.put_back_record(rec);
                error = Some(Errno::EMSGSIZE);
                break 'release;
            }
            if len < pktlen {
                gr.put_back_record(rec);
                break 'release;
            }
            if pktlen < len {
                maxreached = false;
                len = pktlen;
            }
        }
        // Throw away the name and the control mbufs after it has been assured that the
        // whole first record can be processed.
        for _ in 0..skip {
            if let Some(b) = rec.pop_first() {
                gr.sbfree(&b);
            }
        }

        // Take at most len mbufs out of receive buffer.
        let mut taken: hal::ds::owned::OwnedSlist<crate::net::mbuf::MNext> =
            hal::ds::owned::OwnedSlist::new();
        let mut off: u64 = 0;
        while off <= len {
            let Some(cur) = rec.first() else {
                break;
            };
            let size = len - off;
            if cfg!(feature = "diagnostic") {
                let t = i32::from(cur.m_type().get());
                if t != MT_DATA && t != crate::net::mbuf::MT_HEADER {
                    kpanic(format_args!("somove type: so {so:p}, m_type {t}"));
                }
            }
            if u64::from(cur.m_len().get()) > size {
                // Move only a partial mbuf at maximum splice length or if the drain buffer is
                // too small for this large mbuf.
                if !maxreached && sosp.so_snd.datacc() > 0 {
                    len -= size;
                    break;
                }
                let copy = if wait == crate::net::mbuf::M_WAIT {
                    drop(gs);
                    drop(gr);
                    let c = rcv_copy_front(&mut rec, size as i32, wait);
                    gr = so.so_rcv.lock();
                    gs = sosp.so_snd.lock();
                    c
                } else {
                    rcv_copy_front(&mut rec, size as i32, wait)
                };
                let Some(copy) = copy else {
                    len -= size;
                    break;
                };
                if let Some(front) = rec.first() {
                    front.data_add(size as isize);
                    front.m_len().set(front.m_len().get() - size as u32);
                    gr.sub_data(front, size);
                }
                off += u64::from(copy.m_len().get());
                taken.push_tail(copy);
            } else {
                let Some(b) = rec.pop_first() else {
                    break;
                };
                gr.sbfree(&b);
                off += u64::from(b.m_len().get());
                taken.push_tail(b);
            }
        }
        gr.put_back_record(rec);

        let Some(mut m) = taken.into_head() else {
            break 'release;
        };
        if m.flags() & M_PKTHDR != 0 {
            m_resethdr(&mut m);
            if let Some(h) = m.pkthdr() {
                h.len.set(len as i32);
            }
        }

        // Receive buffer did shrink by len bytes, adjust oob.
        let mut rcvstate = so.so_rcv.state();
        gr.clear_state(crate::net::sockbuf::SS_RCVATMARK);
        let mut oobmark = so.oobmark();
        so.set_oobmark(oobmark.saturating_sub(len));
        if oobmark != 0 {
            if oobmark == len {
                gr.set_state(crate::net::sockbuf::SS_RCVATMARK);
            }
            if oobmark >= len {
                oobmark = 0;
            }
        }

        // Send window update to source peer as receive buffer has changed.
        if so.pr_flags(PR_WANTRCVD) {
            rcvd_unlocked!();
        }

        // Handle oob data. If any malloc fails, ignore error. TCP urgent data is not very
        // reliable anyway.
        while (rcvstate & crate::net::sockbuf::SS_RCVATMARK != 0 || oobmark != 0)
            && so.has_options(SO_OOBINLINE)
        {
            let mut o: Option<MbufBox> = None;

            drop(gs);
            drop(gr);

            if rcvstate & crate::net::sockbuf::SS_RCVATMARK != 0 {
                o = m_get(wait, MT_DATA);
                rcvstate &= !crate::net::sockbuf::SS_RCVATMARK;
            } else if oobmark != 0 {
                if let Some(oo) = m_split(&mut m, oobmark as i32, wait) {
                    if let Some(h) = m.pkthdr() {
                        h.ph_loopcnt.set(h.ph_loopcnt.get().wrapping_add(1));
                    }
                    let e = {
                        let mut w = solock_shared(&sosp);
                        sosp.usrreqs().send(&sosp, &mut w, Some(m), None, None)
                    };
                    if let Err(e) = e {
                        gr = so.so_rcv.lock();
                        gs = sosp.so_snd.lock();
                        error = Some(if sosp.so_snd.has_state(SS_CANTSENDMORE) {
                            Errno::EPIPE
                        } else {
                            e
                        });
                        drop(oo);
                        break 'release;
                    }
                    len -= oobmark;
                    sp.len.fetch_add(oobmark as Off, Ordering::Relaxed);
                    m = oo;
                    o = m_get(wait, MT_DATA);
                }
                oobmark = 0;
            }
            if let Some(mut o) = o {
                o.m_len().set(1);
                let byte = m.data().first().copied().unwrap_or(0);
                if let Some(d) = o.data_mut() {
                    d[0] = byte;
                }

                let e = {
                    let mut w = solock_shared(&sosp);
                    sosp.usrreqs().sendoob(&sosp, &mut w, Some(o), None, None)
                };
                if let Err(e) = e {
                    gr = so.so_rcv.lock();
                    gs = sosp.so_snd.lock();
                    error = Some(if sosp.so_snd.has_state(SS_CANTSENDMORE) {
                        Errno::EPIPE
                    } else {
                        e
                    });
                    drop(m);
                    break 'release;
                }
                len -= 1;
                sp.len.fetch_add(1, Ordering::Relaxed);
                if oobmark != 0 {
                    oobmark -= 1;
                    if oobmark == 0 {
                        rcvstate |= crate::net::sockbuf::SS_RCVATMARK;
                    }
                }
                m_adj(&m, 1);
            }

            gr = so.so_rcv.lock();
            gs = sosp.so_snd.lock();
        }

        // Append all remaining data to drain socket.
        if so.so_rcv.cc() == 0 || maxreached {
            gs.clear_state(crate::net::sockbuf::SS_ISSENDING);
        }

        drop(gs);
        drop(gr);
        if let Some(h) = m.pkthdr() {
            h.ph_loopcnt.set(h.ph_loopcnt.get().wrapping_add(1));
        }
        let e = {
            let mut w = solock_shared(&sosp);
            sosp.usrreqs().send(&sosp, &mut w, Some(m), None, None)
        };
        gr = so.so_rcv.lock();
        gs = sosp.so_snd.lock();

        if let Err(e) = e {
            error = Some(
                if sosp.so_snd.has_state(SS_CANTSENDMORE) || !sosp.has_pcb() {
                    Errno::EPIPE
                } else {
                    e
                },
            );
            break 'release;
        }
        sp.len.fetch_add(len as Off, Ordering::Relaxed);

        // Move several packets if possible.
        if !maxreached && nextrecord {
            continue 'release; // goto nextpkt
        }
        break 'release;
    }

    // release:
    gs.clear_state(crate::net::sockbuf::SS_ISSENDING);

    if error.is_none() && maxreached && sp.max.load(Ordering::Relaxed) == sp.spliced_len() {
        error = Some(Errno::EFBIG);
    }
    if error.is_some() {
        so.set_error(error);
    }

    let unsplice = (so.so_rcv.has_state(SS_CANTRCVMORE) && so.so_rcv.cc() == 0)
        || sosp.so_snd.has_state(SS_CANTSENDMORE)
        || maxreached
        || error.is_some();

    drop(gs);
    drop(gr);
    drop(sl);

    if unsplice {
        sounsplice(so, &sosp, 0);
        return false;
    }
    let idle = sp.idle_nsec();
    if idle != 0 {
        hal::ds::pin::project(
            so.sp
                .get()
                .map_or_else(|| kpanic(format_args!("somove: no splice")), |b| b.as_pin()),
            |s| &s.idleto,
        )
        .add_nsec(idle);
    }
    true
}

/// `sowakeup(so, sb)`: wakeup processes waiting on a socket buffer. Do asynchronous
/// notification via `SIGIO` if the socket buffer has the `SB_ASYNC` flag set.
pub fn sowakeup<G: ProcGlue>(so: &Socket<G>, sb: &Sockbuf<G>) {
    let (dowakeup, dopgsigio) = sb.lock().wakeup_locked();

    if dowakeup {
        wakeup(sb.cc_chan());
    }
    if dopgsigio {
        pgsigio::<G>(&so.sigio, sysdefs::signal::SIGIO, false);
    }
}

/// `sorwakeup(so)`: data arrived on `so`: run a splice's task, or wake the readers and the
/// upcall.
pub fn sorwakeup<G: ProcGlue>(so: &Socket<G>) {
    if so.pr_flags(PR_SPLICE) {
        let g = so.so_rcv.lock();
        if so.so_rcv.has_flags(SB_SPLICE)
            && let Some(b) = so.sp.get()
            && let Some(q) = b.queue.get()
        {
            crate::base::task::task_add(q, hal::ds::pin::project(b.as_pin(), |s| &s.task));
        }
        let spliced = isspliced(so);
        drop(g);
        if spliced {
            return;
        }
    }
    sowakeup(so, &so.so_rcv);
    if so.upcall_kind.load(Ordering::Acquire) != 0
        && let Some(f) = net::<G>().upcall.get()
        && let Some(arg) = so.upcall_arg()
    {
        f(so, &arg, crate::net::mbuf::M_DONTWAIT);
    }
}

/// `sowwakeup(so)`: space appeared on `so`: run the source's splice task, or wake the
/// writers.
pub fn sowwakeup<G: ProcGlue>(so: &Socket<G>) {
    if so.pr_flags(PR_SPLICE) {
        let g = so.so_snd.lock();
        if so.so_snd.has_flags(SB_SPLICE)
            && let Some(sp) = so.splice()
            && let Some(back) = sp.source(&g)
            && let Some(bsp) = back.sp.get()
            && let Some(q) = bsp.queue.get()
        {
            crate::base::task::task_add(q, hal::ds::pin::project(bsp.as_pin(), |s| &s.task));
        }
        let spliced = issplicedback(so);
        drop(g);
        if spliced {
            return;
        }
    }
    sowakeup(so, &so.so_snd);
}

/// `sohasoutofband(so)`: out-of-band data arrived: `SIGURG` and the except knotes.
pub fn sohasoutofband<G: ProcGlue>(so: &Socket<G>) {
    pgsigio::<G>(&so.sigio, sysdefs::signal::SIGURG, false);
    let mut g = so.so_rcv.lock();
    so.so_rcv.klist.knote_locked(g.witness(), 0);
}

/// The `int` an option's mbuf holds (`*mtod(m, int *)`), `None` when it is shorter.
fn opt_int(m: &Mbuf) -> Option<i32> {
    m.data().first_chunk::<4>().map(|b| i32::from_ne_bytes(*b))
}

/// `*mtod(m, int *) = v`, `m_len = sizeof(int)`.
fn set_opt_int(m: &mut Mbuf, v: i32) {
    set_opt_bytes(m, &v.to_ne_bytes());
}

/// The option's value `b` into `m` (`m_len` its length).
fn set_opt_bytes(m: &mut Mbuf, b: &[u8]) {
    m.m_len().set(b.len() as u32);
    if let Some(d) = m.data_mut() {
        d.copy_from_slice(b);
    }
}

/// `sosetopt(so, level, optname, m)`: sets a socket option (`SOL_SOCKET`) or hands it to
/// the protocol; `m` holds the value, `p` is the thread (`SO_BINDANY`'s `suser`,
/// `SO_SPLICE`'s descriptor).
pub fn sosetopt<G: ProcGlue>(
    so: &Socket<G>,
    p: &Thread<G>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    use crate::net::proto::PRCO_SETOPT;
    use sysdefs::socket::{
        SO_BINDANY, SO_BROADCAST, SO_DEBUG, SO_DONTROUTE, SO_KEEPALIVE, SO_LINGER, SO_OOBINLINE,
        SO_RCVBUF, SO_RCVLOWAT, SO_RCVTIMEO, SO_REUSEADDR, SO_REUSEPORT, SO_RTABLE, SO_SNDBUF,
        SO_SNDLOWAT, SO_SNDTIMEO, SO_SPLICE, SO_TIMESTAMP, SO_USELOOPBACK, SO_ZEROIZE, SOL_SOCKET,
    };

    let mlen = m.as_deref().map(|m| m.m_len().get() as usize);

    if level != SOL_SOCKET {
        if so.proto.has_ctloutput() {
            let mut w = solock(so);
            return so
                .proto
                .ctloutput(PRCO_SETOPT, so, &mut w, level, optname, m);
        }
        return Err(Errno::ENOPROTOOPT);
    }

    match optname {
        SO_LINGER => {
            let Some(m) = m.filter(|_| mlen == Some(size_of::<sysdefs::socket::Linger>())) else {
                return Err(Errno::EINVAL);
            };
            let Some(l) = hal::user::pod_from_bytes::<sysdefs::socket::Linger>(m.data()) else {
                return Err(Errno::EINVAL);
            };
            if l.l_linger < 0 || l.l_linger > i32::from(i16::MAX) {
                return Err(Errno::EINVAL);
            }

            let w = solock(so);
            so.linger.store(l.l_linger as i16, Ordering::Relaxed);
            so.set_options(&w, optname, l.l_onoff != 0);
            Ok(())
        }
        SO_BINDANY | SO_DEBUG | SO_KEEPALIVE | SO_USELOOPBACK | SO_BROADCAST | SO_REUSEADDR
        | SO_REUSEPORT | SO_OOBINLINE | SO_TIMESTAMP | SO_ZEROIZE => {
            if optname == SO_BINDANY {
                crate::proc::suser(p)?; // XXX
            }
            let Some(v) = m
                .filter(|_| mlen.is_some_and(|l| l >= 4))
                .and_then(|m| opt_int(m))
            else {
                return Err(Errno::EINVAL);
            };

            let w = solock(so);
            so.set_options(&w, optname, v != 0);
            Ok(())
        }
        SO_DONTROUTE => {
            let Some(v) = m
                .filter(|_| mlen.is_some_and(|l| l >= 4))
                .and_then(|m| opt_int(m))
            else {
                return Err(Errno::EINVAL);
            };
            if v != 0 {
                return Err(Errno::EOPNOTSUPP);
            }
            Ok(())
        }
        SO_SNDBUF | SO_RCVBUF | SO_SNDLOWAT | SO_RCVLOWAT => {
            let sb = if optname == SO_SNDBUF || optname == SO_SNDLOWAT {
                &so.so_snd
            } else {
                &so.so_rcv
            };

            let Some(v) = m
                .filter(|_| mlen.is_some_and(|l| l >= 4))
                .and_then(|m| opt_int(m))
            else {
                return Err(Errno::EINVAL);
            };
            let mut cnt = i64::from(v) as u64;
            if (cnt as i64) <= 0 {
                cnt = 1;
            }

            let mut g = sb.lock();
            match optname {
                SO_SNDBUF | SO_RCVBUF => {
                    if sb.has_state(SS_CANTSENDMORE | SS_CANTRCVMORE) {
                        Err(Errno::EINVAL)
                    } else if crate::net::sockbuf::sbcheckreserve(cnt, sb.wat()).is_err()
                        || g.sbreserve(cnt).is_err()
                    {
                        Err(Errno::ENOBUFS)
                    } else {
                        g.set_wat(cnt);
                        Ok(())
                    }
                }
                _ => {
                    g.set_lowat(if cnt > sb.hiwat() {
                        sb.hiwat() as i64
                    } else {
                        cnt as i64
                    });
                    Ok(())
                }
            }
        }
        SO_SNDTIMEO | SO_RCVTIMEO => {
            let sb = if optname == SO_SNDTIMEO {
                &so.so_snd
            } else {
                &so.so_rcv
            };

            let Some(tv) = m
                .filter(|_| mlen.is_some_and(|l| l >= size_of::<Timeval>()))
                .and_then(|m| {
                    hal::user::pod_from_bytes::<Timeval>(&m.data()[..size_of::<Timeval>()])
                })
            else {
                return Err(Errno::EINVAL);
            };
            if !tv.is_valid() {
                return Err(Errno::EINVAL);
            }
            let mut nsecs = tv.to_nsec();
            if nsecs == u64::MAX {
                return Err(Errno::EDOM);
            }
            if nsecs == 0 {
                nsecs = INFSLP;
            }

            sb.lock().set_timeo_nsecs(nsecs);
            Ok(())
        }
        SO_RTABLE => match so.proto.domain().protocols().first() {
            Some(first) if so.proto.has_ctloutput() => {
                let level = i32::from(first.protocol());
                let mut w = solock(so);
                so.proto
                    .ctloutput(PRCO_SETOPT, so, &mut w, level, optname, m)
            }
            _ => Err(Errno::ENOPROTOOPT),
        },
        SO_SPLICE => match m {
            None => sosplice(so, p, -1, 0, None),
            Some(m) if (m.m_len().get() as usize) < size_of::<i32>() => Err(Errno::EINVAL),
            Some(m) if (m.m_len().get() as usize) < size_of::<Splice>() => {
                sosplice(so, p, opt_int(m).unwrap_or(-1), 0, None)
            }
            Some(m) => {
                let Some(sp) =
                    hal::user::pod_from_bytes::<Splice>(&m.data()[..size_of::<Splice>()])
                else {
                    return Err(Errno::EINVAL);
                };
                sosplice(so, p, sp.sp_fd, sp.sp_max, Some(&sp.sp_idle))
            }
        },
        _ => Err(Errno::ENOPROTOOPT),
    }
}

/// `sogetopt(so, level, optname, m)`: reads a socket option (`SOL_SOCKET`) into `m`, or asks
/// the protocol.
pub fn sogetopt<G: ProcGlue>(
    so: &Socket<G>,
    level: i32,
    optname: i32,
    m: &mut Mbuf,
) -> Result<(), Errno> {
    use crate::net::proto::PRCO_GETOPT;
    use sysdefs::socket::{
        AF_UNIX, SO_BINDANY, SO_BROADCAST, SO_DEBUG, SO_DOMAIN, SO_DONTROUTE, SO_ERROR,
        SO_KEEPALIVE, SO_LINGER, SO_OOBINLINE, SO_PEERCRED, SO_PROTOCOL, SO_RCVBUF, SO_RCVLOWAT,
        SO_RCVTIMEO, SO_REUSEADDR, SO_REUSEPORT, SO_RTABLE, SO_SNDBUF, SO_SNDLOWAT, SO_SNDTIMEO,
        SO_SPLICE, SO_TIMESTAMP, SO_TYPE, SO_USELOOPBACK, SO_ZEROIZE, SOL_SOCKET,
    };

    if level != SOL_SOCKET {
        if !so.proto.has_ctloutput() {
            return Err(Errno::ENOPROTOOPT);
        }
        m.m_len().set(0);

        let mut w = solock(so);
        return so
            .proto
            .ctloutput(PRCO_GETOPT, so, &mut w, level, optname, Some(m));
    }

    m.m_len().set(size_of::<i32>() as u32);

    match optname {
        SO_LINGER => {
            let l = {
                let _w = solock_shared(so);
                sysdefs::socket::Linger {
                    l_onoff: so.options() & SO_LINGER,
                    l_linger: i32::from(so.linger()),
                }
            };
            set_opt_bytes(m, hal::user::bytes_of(core::slice::from_ref(&l)));
        }
        SO_BINDANY | SO_USELOOPBACK | SO_DEBUG | SO_KEEPALIVE | SO_REUSEADDR | SO_REUSEPORT
        | SO_BROADCAST | SO_OOBINLINE | SO_ACCEPTCONN | SO_TIMESTAMP | SO_ZEROIZE => {
            set_opt_int(m, so.options() & optname);
        }
        SO_DONTROUTE => set_opt_int(m, 0),
        SO_TYPE => set_opt_int(m, so.so_type),
        SO_ERROR => {
            let _w = solock(so);
            set_opt_int(m, so.error_raw() as i32);
            so.set_error(None);
        }
        SO_DOMAIN => set_opt_int(m, so.family),
        SO_PROTOCOL => set_opt_int(m, i32::from(so.proto.protocol())),
        SO_SNDBUF => set_opt_int(m, so.so_snd.hiwat() as i32),
        SO_RCVBUF => set_opt_int(m, so.so_rcv.hiwat() as i32),
        SO_SNDLOWAT => set_opt_int(m, so.so_snd.lowat() as i32),
        SO_RCVLOWAT => set_opt_int(m, so.so_rcv.lowat() as i32),
        SO_SNDTIMEO | SO_RCVTIMEO => {
            let sb = if optname == SO_SNDTIMEO {
                &so.so_snd
            } else {
                &so.so_rcv
            };

            let nsecs = {
                let _g = sb.lock();
                sb.timeo_nsecs()
            };

            let mut tv = Timeval::new(0, 0);
            if nsecs != INFSLP {
                tv = Timeval::from_nsec(nsecs);
            }
            set_opt_bytes(m, hal::user::bytes_of(core::slice::from_ref(&tv)));
        }
        SO_RTABLE => match so.proto.domain().protocols().first() {
            Some(first) if so.proto.has_ctloutput() => {
                let level = i32::from(first.protocol());
                let mut w = solock(so);
                so.proto
                    .ctloutput(PRCO_GETOPT, so, &mut w, level, optname, Some(m))?;
            }
            _ => return Err(Errno::ENOPROTOOPT),
        },
        SO_SPLICE => {
            let len: Off = {
                let _w = solock_shared(so);
                so.splice().map_or(0, Sosplice::spliced_len)
            };
            set_opt_bytes(m, &len.to_ne_bytes());
        }
        SO_PEERCRED => {
            if i32::from(so.proto.protocol()) != i32::from(AF_UNIX) {
                return Err(Errno::EOPNOTSUPP);
            }
            let w = solock(so);
            let cred = so.proto.domain().peercred(so, &w);
            drop(w);
            match cred {
                Some(cred) => set_opt_bytes(m, hal::user::bytes_of(core::slice::from_ref(&cred))),
                None => return Err(Errno::ENOTCONN),
            }
        }
        _ => return Err(Errno::ENOPROTOOPT),
    }
    Ok(())
}

/// `so_print(v, pr)` (`DDB`): the socket's state, through `pr` (`db_printf`).
pub fn so_print<G: ProcGlue>(so: &Socket<G>, pr: &mut dyn core::fmt::Write) -> core::fmt::Result {
    let (q0len, qlen, head, onq) = {
        let g = cq(so);
        (
            g.q0len,
            g.qlen,
            g.head.as_ref().map_or(0, |h| PoolArc::as_ptr(h).addr()),
            g.onq,
        )
    };
    writeln!(pr, "socket {so:p}")?;
    writeln!(pr, "so_type: {}", so.so_type)?;
    writeln!(pr, "so_options: 0x{:04x}", so.options())?;
    writeln!(pr, "so_linger: {}", so.linger())?;
    writeln!(pr, "so_state: 0x{:04x}", so.state())?;
    writeln!(pr, "so_pcb: {:#x}", so.legacy_pcb())?;
    writeln!(
        pr,
        "so_proto: {:p}",
        core::ptr::from_ref(so.proto).cast::<()>()
    )?;
    writeln!(pr, "so_sigio: {:#x}", so.sigio.sigio_addr())?;
    writeln!(pr, "so_head: {head:#x}")?;
    writeln!(pr, "so_onq: {onq:?}")?;
    writeln!(pr, "so_q0len: {q0len}")?;
    writeln!(pr, "so_qlen: {qlen}")?;
    writeln!(pr, "so_qlimit: {}", so.qlimit())?;
    writeln!(pr, "so_timeo: {}", so.timeo())?;
    writeln!(pr, "so_obmark: {}", so.oobmark())?;
    if let Some(sp) = so.splice() {
        writeln!(pr, "so_sp: {:p}", core::ptr::from_ref(sp))?;
        writeln!(pr, "\tssp_len: {}", sp.spliced_len())?;
        writeln!(pr, "\tssp_max: {}", sp.max.load(Ordering::Relaxed))?;
        writeln!(
            pr,
            "\tssp_idletv: {} {}",
            sp.idle_sec.load(Ordering::Relaxed),
            sp.idle_usec.load(Ordering::Relaxed)
        )?;
    } else {
        writeln!(pr, "so_sp: 0x0")?;
    }
    writeln!(pr, "so_rcv:")?;
    sobuf_print(&so.so_rcv, pr)?;
    writeln!(pr, "so_snd:")?;
    sobuf_print(&so.so_snd, pr)?;
    writeln!(
        pr,
        "so_upcall: {} so_upcallarg: {:#x}",
        so.upcall_kind.load(Ordering::Relaxed),
        so.upcall_arg.read().as_ref().map_or(0, AnyArc::addr)
    )?;
    writeln!(pr, "so_euid: {} so_ruid: {}", so.euid, so.ruid)?;
    writeln!(pr, "so_egid: {} so_rgid: {}", so.egid, so.rgid)?;
    writeln!(pr, "so_cpid: {}", so.cpid)
}

/// `sobuf_print(sb, pr)` (`DDB`).
pub fn sobuf_print<G: ProcGlue>(
    sb: &Sockbuf<G>,
    pr: &mut dyn core::fmt::Write,
) -> core::fmt::Result {
    writeln!(pr, "\tsb_cc: {}", sb.cc())?;
    writeln!(pr, "\tsb_datacc: {}", sb.datacc())?;
    writeln!(pr, "\tsb_hiwat: {}", sb.hiwat())?;
    writeln!(pr, "\tsb_wat: {}", sb.wat())?;
    writeln!(pr, "\tsb_mbcnt: {}", sb.mbcnt())?;
    writeln!(pr, "\tsb_mbmax: {}", sb.mbmax())?;
    writeln!(pr, "\tsb_lowat: {}", sb.lowat())?;
    writeln!(pr, "\tsb_flags: {:04x}", sb.flags())?;
    writeln!(pr, "\tsb_state: {:04x}", sb.state())?;
    writeln!(pr, "\tsb_timeo_nsecs: {}", sb.timeo_nsecs())
}

/// The size of `struct fdpass` (a file in flight in a UNIX domain `SCM_RIGHTS` message), for
/// `sosend`'s reservation of the space `unp_internalize` grows a message to.
pub const FDPASS_SIZE: usize = 16;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    //! The socket core over a stream protocol of the tests (`TestUsr`: attach and detach a
    //! counter, abort decommissions, send appends to the peer of a pair), its domain the
    //! test glue's only one.

    use std::sync::atomic::AtomicBool;
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;
    use crate::net::domain::{Domain, domaininit, soinit};
    use crate::net::mbuf::tests::setup;

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// The test domain's family (`AF_UNIX`'s: a per-socket lock).
    const FAMILY: i32 = 1;

    /// `attach` refuses when set.
    static ATTACH_FAILS: AtomicBool = AtomicBool::new(false);
    /// Whether the last `attach` could open its socket lock's parts (a kernel internet
    /// socket locked with a proof of the net lock).
    static ATTACH_PARTS: AtomicBool = AtomicBool::new(false);
    /// The same for the last `abort`.
    static ABORT_PARTS: AtomicBool = AtomicBool::new(false);

    struct TestDom;
    struct TestProto;
    struct TestUsr;

    static PROTOS: [&dyn Protocol<Procs>; 1] = [&TestProto];
    static DOMS: [&dyn Domain<Procs>; 1] = [&TestDom];

    impl Domain<Procs> for TestDom {
        fn family(&self) -> i32 {
            FAMILY
        }
        fn name(&self) -> &'static str {
            "test"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<Procs>] {
            &PROTOS
        }
    }

    impl Protocol<Procs> for TestProto {
        fn domain(&self) -> &'static dyn Domain<Procs> {
            &TestDom
        }
        fn ty(&self) -> i16 {
            sysdefs::socket::SOCK_STREAM as i16
        }
        fn protocol(&self) -> i16 {
            0
        }
        fn flags(&self) -> i16 {
            crate::net::proto::PR_CONNREQUIRED
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<Procs>> {
            Some(&TestUsr)
        }
    }

    impl Usrreqs<Procs> for TestUsr {
        fn attach(
            &self,
            _so: &Socket<Procs>,
            w: &mut SoLocked<'_>,
            _proto: i32,
            _wait: i32,
        ) -> Result<(), Errno> {
            ATTACH_PARTS.store(w.inet().is_some(), Ordering::Relaxed);
            if ATTACH_FAILS.load(Ordering::Relaxed) {
                Err(Errno::ENOBUFS)
            } else {
                Ok(())
            }
        }
        fn detach(&self, _so: &Socket<Procs>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
            Ok(())
        }
        fn shutdown(&self, so: &Socket<Procs>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
            socantsendmore(so);
            Ok(())
        }
        fn rcvd(&self, _so: &Socket<Procs>, _w: &mut SoLocked<'_>) {}
        fn send(
            &self,
            so: &Socket<Procs>,
            _w: &mut SoLocked<'_>,
            top: Option<MbufBox>,
            _addr: Option<&Mbuf>,
            control: Option<MbufBox>,
        ) -> Result<(), Errno> {
            drop(control);
            so.so_rcv.lock().sbappend(top);
            sorwakeup(so);
            Ok(())
        }
        fn listen(&self, _so: &Socket<Procs>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
            Ok(())
        }
        fn abort(&self, so: &Socket<Procs>, w: &mut SoLocked<'_>) {
            ABORT_PARTS.store(w.inet().is_some(), Ordering::Relaxed);
            soisdisconnected(so);
            sofree_kept(so, w);
        }
        fn sockaddr(
            &self,
            _so: &Socket<Procs>,
            _w: &mut SoLocked<'_>,
            _addr: &mut Mbuf,
        ) -> Result<(), Errno> {
            Ok(())
        }
        fn peeraddr(
            &self,
            _so: &Socket<Procs>,
            _w: &mut SoLocked<'_>,
            _addr: &mut Mbuf,
        ) -> Result<(), Errno> {
            Ok(())
        }
    }

    /// The doubles, the pools, the test domain.
    fn net_setup() -> std::sync::MutexGuard<'static, ()> {
        let g = setup();
        soinit::<Procs>();
        domaininit::<Procs>(&DOMS);
        g
    }

    /// A socket of the test protocol.
    fn socket(state: u32) -> SocketRef<Procs> {
        let so = soalloc(
            &TestProto,
            crate::net::mbuf::M_WAIT,
            SoInit {
                so_type: sysdefs::socket::SOCK_STREAM,
                state,
                options: 0,
                linger: 0,
                timeo: 0,
                euid: 0,
                ruid: 0,
                egid: 0,
                rgid: 0,
                cpid: 1,
            },
        )
        .expect("a socket");
        soreserve(&so, 4096, 4096).expect("reserve");
        so
    }

    /// The items `sockpl` lends now.
    fn nout() -> u64 {
        net::<Procs>().sockpl.nout()
    }

    #[test]
    fn the_last_reference_after_sofree_frees_the_socket() {
        let _g = net_setup();
        let before = nout();
        let so = socket(0);
        assert_eq!(nout(), before + 1);
        // Held by its file (this handle) and its existence reference: sofree without
        // SS_NOFDREF keeps it.
        let w = solock(&so);
        let w = sofree(&so, w, true).expect("kept");
        drop(w);
        assert_eq!(PoolArc::count(&so), 2);
        so.st_set(SS_NOFDREF);
        drop(sofree(&so, solock(&so), false));
        assert_eq!(PoolArc::count(&so), 1);
        // A second decommission adopts nothing.
        drop(sofree(&so, solock(&so), false));
        assert_eq!(PoolArc::count(&so), 1);
        drop(so);
        assert_eq!(nout(), before);
    }

    #[test]
    fn a_connection_is_queued_promoted_and_accepted_once() {
        let _g = net_setup();
        let head = socket(0);
        head.qlimit.store(5, Ordering::Relaxed);
        let child = sonewconn(&head, 0, crate::net::mbuf::M_WAIT, None).expect("a connection");
        sounlock_detached(&child);
        assert_eq!(head.qlens(), (1, 0));
        assert_eq!(PoolArc::count(&child), 3);

        // Nothing to accept yet: a non-blocking accept would block.
        assert_eq!(
            soaccept_take(&head, solock_shared(&head), true, |_, _| ()).err(),
            Some(Errno::EWOULDBLOCK)
        );

        soisconnected(&child);
        assert_eq!(head.qlens(), (0, 1));
        assert_eq!(head.qlen(), 1);
        assert!(child.has_state(SS_ISCONNECTED));

        // The connection handed to the accepter locked (its witness), already off the queue.
        let (got, locked) = soaccept_take(&head, solock_shared(&head), true, |so, w| {
            (
                w.so_lock().is_some(),
                so.has_state(SS_ISCONNECTED),
                head.qlens(),
            )
        })
        .expect("the connection");
        assert_eq!(locked, (true, true, (0, 0)));
        assert!(PoolArc::ptr_eq(&got, &child));
        assert_eq!(head.qlens(), (0, 0));
        // The queue's reference went to the accepter; the queue holds none any more.
        assert!(soqremque(&child, Onq::Q).is_none());
        drop(got);
        assert_eq!(PoolArc::count(&child), 2);

        child.st_set(SS_NOFDREF);
        drop(sofree(&child, solock(&child), false));
        drop(child);
    }

    #[test]
    fn an_attach_that_fails_takes_the_child_off_the_queue() {
        let _g = net_setup();
        let before = nout();
        let head = socket(0);
        head.qlimit.store(5, Ordering::Relaxed);
        ATTACH_FAILS.store(true, Ordering::Relaxed);
        let child = sonewconn(&head, 0, crate::net::mbuf::M_WAIT, None);
        ATTACH_FAILS.store(false, Ordering::Relaxed);
        assert!(child.is_none());
        assert_eq!(head.qlens(), (0, 0));
        assert_eq!(nout(), before + 1);
        head.st_set(SS_NOFDREF);
        drop(sofree(&head, solock(&head), false));
        drop(head);
        assert_eq!(nout(), before);
    }

    #[test]
    fn a_listener_closed_drops_its_queued_connections() {
        let _g = net_setup();
        let before = nout();
        let head = socket(0);
        head.qlimit.store(5, Ordering::Relaxed);
        {
            let mut w = solock(&head);
            solisten(&head, &mut w, 5).expect("listen");
        }
        assert!(head.has_options(SO_ACCEPTCONN));
        let a = sonewconn(&head, 0, crate::net::mbuf::M_WAIT, None).expect("a");
        sounlock_detached(&a);
        let b = sonewconn(&head, SS_ISCONNECTED, crate::net::mbuf::M_WAIT, None).expect("b");
        sounlock_detached(&b);
        assert_eq!(head.qlens(), (1, 1));
        drop(a);
        drop(b);
        soclose(&head, 0).expect("close");
        assert_eq!(head.qlens(), (0, 0));
        drop(head);
        assert_eq!(nout(), before);
    }

    /// A listener of the test protocol, `listen`ed with room for `n` connections.
    fn listener(n: i32) -> SocketRef<Procs> {
        let head = socket(0);
        let mut w = solock(&head);
        solisten(&head, &mut w, n).expect("listen");
        drop(w);
        head
    }

    /// `soaccept_take` with the C's `soaccept` in its closure (the connection gets its file
    /// reference: `SS_NOFDREF` cleared; the test protocol has no `pru_accept`, so its answer
    /// is the C's EOPNOTSUPP), the connection or the error.
    fn accept(head: &Socket<Procs>, nonblock: bool) -> Result<SocketRef<Procs>, Errno> {
        let mut nam = crate::net::mbuf::m_get(crate::net::mbuf::M_WAIT, 3).expect("an mbuf");
        soaccept_take(head, solock_shared(head), nonblock, |so, w| {
            soaccept(so, w, &mut nam)
        })
        .map(|(so, r)| {
            assert_eq!(r, Err(Errno::EOPNOTSUPP));
            assert!(!so.has_state(SS_NOFDREF));
            so
        })
    }

    /// A complete connection on `head`'s queue (sonewconn, then promoted).
    fn connection(head: &Socket<Procs>) -> SocketRef<Procs> {
        let c = sonewconn(head, 0, crate::net::mbuf::M_WAIT, None).expect("a connection");
        sounlock_detached(&c);
        soisconnected(&c);
        c
    }

    #[test]
    fn two_accepters_take_one_connection_once() {
        // 200 rounds of two threads accepting the one connection queued, each round from
        // the same start: exactly one gets it, the other answers EWOULDBLOCK; nothing leaks.
        // The accepters are serialised by the listener's lock, as the C's are, so the
        // window between soaccept_take's peek under `cq` and its soqremque is not contended
        // here, nor anywhere: everything that takes a socket off `so_q` holds the listener's
        // lock (accept, and soclose of the listener); sofree takes partial connections off
        // `so_q0` alone and leaves a complete one queued. The retry on `None` is the C's
        // panic("accept") made safe, unreachable by the same argument.
        let _g = net_setup();
        let before = nout();
        let head = listener(5);
        for round in 0..200 {
            let c = connection(&head);
            let got: Vec<Result<SocketRef<Procs>, Errno>> = std::thread::scope(|sc| {
                let t: Vec<_> = (0..2).map(|_| sc.spawn(|| accept(&head, true))).collect();
                t.into_iter()
                    .map(|j| j.join().expect("an accepter"))
                    .collect()
            });
            let (ok, err): (Vec<_>, Vec<_>) = got.into_iter().partition(Result::is_ok);
            assert_eq!((ok.len(), err.len()), (1, 1), "round {round}");
            assert_eq!(err[0].as_ref().err(), Some(&Errno::EWOULDBLOCK));
            let so = ok
                .into_iter()
                .next()
                .and_then(Result::ok)
                .expect("the connection");
            assert!(PoolArc::ptr_eq(&so, &c));
            assert_eq!(head.qlens(), (0, 0));
            drop(c);
            soclose(&so, 0).expect("close");
        }
        soclose(&head, 0).expect("close");
        drop(head);
        assert_eq!(nout(), before);
    }

    #[test]
    fn a_sleeping_accept_is_woken_by_a_connection() {
        // 50 rounds: an accepter blocks on the empty queue (the main thread waits 0, 1 or 2
        // ms first, so the connection comes before, around and after its sleep), then a
        // connection is queued and promoted: the accepter always takes it, no wakeup lost.
        let _g = net_setup();
        let before = nout();
        let head = listener(5);
        // A value the main thread changes before the connection comes, read by the
        // accepter's closure: what accept(2) reads there (the listener's FNONBLOCK, the C's
        // read after soqremque) is read after the wait.
        let changed = AtomicBool::new(false);
        for round in 0..50u64 {
            changed.store(false, Ordering::Relaxed);
            let so = std::thread::scope(|sc| {
                let t = sc.spawn(|| {
                    let mut nam =
                        crate::net::mbuf::m_get(crate::net::mbuf::M_WAIT, 3).expect("an mbuf");
                    soaccept_take(&head, solock_shared(&head), false, |so, w| {
                        let seen = changed.load(Ordering::Relaxed);
                        let _ = soaccept(so, w, &mut nam);
                        seen
                    })
                });
                std::thread::sleep(std::time::Duration::from_millis(round % 3));
                changed.store(true, Ordering::Relaxed);
                let c = connection(&head);
                // Bounded: a lost wakeup fails the test instead of hanging the binary.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while !t.is_finished() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "round {round}: no wakeup"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                let (so, seen) = t.join().expect("the accepter").expect("the connection");
                assert!(seen, "round {round}: the closure ran before the wait ended");
                assert!(PoolArc::ptr_eq(&so, &c), "round {round}");
                so
            });
            assert_eq!(head.qlens(), (0, 0));
            soclose(&so, 0).expect("close");
        }
        soclose(&head, 0).expect("close");
        drop(head);
        assert_eq!(nout(), before);
    }

    #[test]
    fn a_listener_closed_during_a_promotion_frees_everything() {
        // 200 rounds: one thread promotes a partial connection (soisconnected: q0 to q)
        // while the main thread closes the listener (soclose aborts both queues): whichever
        // queue the connection is on when the close takes it, it is aborted once and every
        // socket goes back to the pool.
        let _g = net_setup();
        let before = nout();
        for round in 0..200 {
            let head = listener(5);
            let c = sonewconn(&head, 0, crate::net::mbuf::M_WAIT, None).expect("a connection");
            sounlock_detached(&c);
            // Both sides start together.
            let start = std::sync::Barrier::new(2);
            std::thread::scope(|sc| {
                let t = sc.spawn(|| {
                    start.wait();
                    soisconnected(&c);
                });
                start.wait();
                soclose(&head, 0).expect("close");
                t.join().expect("the promoter");
            });
            assert_eq!(head.qlens(), (0, 0), "round {round}");
            assert!(c.has_state(SS_ISDISCONNECTED) || c.has_state(SS_ISCONNECTED));
            drop(c);
            drop(head);
            assert_eq!(nout(), before, "round {round}");
        }
    }

    #[test]
    fn a_waitall_peek_reads_again_from_the_start_after_its_wait() {
        // MSG_PEEK | MSG_WAITALL of more than sb_hiwat bytes (so soreceive does not wait
        // first) on a buffer holding "A": the peek copies "A" and waits; "B" comes; the C
        // reads again from sb_mb, the buffer's start, so the read goes on with "AB" and waits
        // again; the receive side shut wakes it, it reads "AB" from the start once more and
        // stops: "AABAB", a short count (uipc_socket.c's sbwait under MSG_WAITALL).
        use crate::file::uio::{Iovec, Uio, UioRw, UioSeg};
        use sysdefs::socket::{MSG_PEEK, MSG_WAITALL};
        let _g = net_setup();
        let so = socket(0);
        so.st_set(SS_ISCONNECTED);
        let len = so.so_rcv.hiwat() as usize + 1;
        // socreate's untimed waits (the test socket is soalloc's, its timeouts 0).
        so.so_rcv.lock().set_timeo_nsecs(INFSLP);
        let byte = |c: u8| {
            let mut m = crate::net::mbuf::m_get(crate::net::mbuf::M_WAIT, 1).expect("an mbuf");
            m.m_len().set(1);
            m.data_mut().expect("writable")[0] = c;
            m
        };
        so.so_rcv.lock().sbappendstream(byte(b'A'));
        let got = std::thread::scope(|sc| {
            let t = sc.spawn(|| {
                crate::testutil::set_copy_kernel(true);
                let mut buf = std::vec![0u8; len];
                let mut iov = [Iovec {
                    iov_base: buf.as_mut_ptr().cast(),
                    iov_len: len,
                }];
                let mut u = Uio::<Procs> {
                    uio_resid: len,
                    uio_iov: &mut iov,
                    uio_offset: 0,
                    uio_segflg: UioSeg::UIO_SYSSPACE,
                    uio_rw: UioRw::UIO_READ,
                    uio_procp: None,
                };
                let mut fl = MSG_PEEK | MSG_WAITALL;
                let r = soreceive(&so, None, &mut u, None, None, Some(&mut fl), 0);
                (r, buf, u.uio_resid)
            });
            // The reader asleep in sbwait (SB_WAIT set under the buffer's mutex before it
            // sleeps), bounded.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !so.so_rcv.has_flags(crate::net::sockbuf::SB_WAIT) {
                assert!(
                    std::time::Instant::now() < deadline,
                    "the reader never waited"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            so.so_rcv.lock().sbappendstream(byte(b'B'));
            sorwakeup(&so);
            // Its second wait (after "AB"), then the receive side shut: a short count.
            while !(so.so_rcv.has_flags(crate::net::sockbuf::SB_WAIT) && so.so_rcv.cc() == 2)
                && !t.is_finished()
            {
                assert!(
                    std::time::Instant::now() < deadline,
                    "the reader never waited again"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            socantrcvmore(&so);
            while !t.is_finished() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "the reader never woke"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            t.join().expect("the reader")
        });
        assert_eq!(got.0, Ok(()));
        assert_eq!(&got.1[..5], b"AABAB");
        assert_eq!(got.2, len - 5);
        // A peek: the buffer still holds both bytes.
        assert_eq!(so.so_rcv.cc(), 2);
    }

    #[test]
    fn a_closed_listener_answers_econnaborted_once_its_queue_is_empty() {
        let _g = net_setup();
        let head = socket(0);
        head.so_rcv.lock().set_state(SS_CANTRCVMORE);
        assert_eq!(
            soaccept_take(&head, solock_shared(&head), false, |_, _| ()).err(),
            Some(Errno::ECONNABORTED)
        );
        assert!(head.error().is_none());
        head.st_set(SS_NOFDREF);
        drop(sofree(&head, solock(&head), false));
    }

    #[test]
    fn solock_pair_takes_one_lock_for_one_socket_and_two_in_order() {
        let _g = net_setup();
        let a = socket(0);
        let b = socket(0);
        {
            let (_w1, w2) = solock_pair(&a, &a, None);
            assert!(w2.is_none());
            assert_ne!(a.so_lock.lock().status(), 0);
        }
        {
            let (_w1, w2) = solock_pair(&a, &b, None);
            assert!(w2.is_some());
            assert_ne!(a.so_lock.lock().status(), 0);
            assert_ne!(b.so_lock.lock().status(), 0);
        }
        assert_eq!(a.so_lock.lock().status(), 0);
        assert_eq!(b.so_lock.lock().status(), 0);
        for s in [a, b] {
            s.st_set(SS_NOFDREF);
            drop(sofree(&s, solock(&s), false));
        }
    }

    /// An internet domain of the tests (N6.6a0, D-1): a protocol of the kernel's, whose
    /// sockets hold `so_lock` by a witness, and one of `bsd`'s, held by detached holdings.
    struct TestInetDom;
    struct KernInet;
    struct BsdInet;

    static INET_PROTOS: [&dyn Protocol<Procs>; 2] = [&KernInet, &BsdInet];

    impl Domain<Procs> for TestInetDom {
        fn family(&self) -> i32 {
            i32::from(PF_INET)
        }
        fn name(&self) -> &'static str {
            "inet"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<Procs>] {
            &INET_PROTOS
        }
    }

    impl Protocol<Procs> for KernInet {
        fn domain(&self) -> &'static dyn Domain<Procs> {
            &TestInetDom
        }
        fn ty(&self) -> i16 {
            sysdefs::socket::SOCK_STREAM as i16
        }
        fn protocol(&self) -> i16 {
            6
        }
        fn flags(&self) -> i16 {
            crate::net::proto::PR_CONNREQUIRED
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<Procs>> {
            Some(&TestUsr)
        }
    }

    impl Protocol<Procs> for BsdInet {
        fn domain(&self) -> &'static dyn Domain<Procs> {
            &TestInetDom
        }
        fn ty(&self) -> i16 {
            sysdefs::socket::SOCK_STREAM as i16
        }
        fn protocol(&self) -> i16 {
            7
        }
        fn flags(&self) -> i16 {
            crate::net::proto::PR_CONNREQUIRED
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<Procs>> {
            Some(&TestUsr)
        }
        fn legacy(&self) -> bool {
            true
        }
    }

    /// A socket of `prp`, an internet protocol of the tests.
    fn inet_socket(prp: &'static dyn Protocol<Procs>) -> SocketRef<Procs> {
        soalloc(
            prp,
            crate::net::mbuf::M_WAIT,
            SoInit {
                so_type: sysdefs::socket::SOCK_STREAM,
                state: 0,
                options: 0,
                linger: 0,
                timeo: 0,
                euid: 0,
                ruid: 0,
                egid: 0,
                rgid: 0,
                cpid: 1,
            },
        )
        .expect("a socket")
    }

    /// The test's socket decommissioned and freed.
    fn inet_free(so: SocketRef<Procs>) {
        so.st_set(SS_NOFDREF);
        drop(sofree(&so, solock(&so), false));
    }

    #[test]
    fn a_kernel_inet_socket_holds_so_lock_under_the_exclusive_net_lock() {
        let _g = net_setup();
        let so = inet_socket(&KernInet);
        let cell = Guarded::keyed(so.so_lock.lock().id(), so_part::PCB, 5u32);
        let tcp = Guarded::keyed(so.so_lock.lock().id(), so_part::TCP, 0u64);
        {
            let mut w = solock(&so);
            assert_eq!(hal::sync::netlock::status(), hal::sync::rwlock::RW_WRITE);
            assert_eq!(so.so_lock.lock().status(), hal::sync::rwlock::RW_WRITE);
            assert!(w.so_lock().is_some());
            let mut l = w.inet().expect("the parts");
            // Two objects' cells open at once through their parts.
            let p = cell.get_mut(&mut l.pcb);
            let t = tcp.get_mut(&mut l.tcp);
            *p += 1;
            *t = u64::from(*p);
            let r = l.net.reader();
            drop(r);
        }
        assert_eq!(so.so_lock.lock().status(), 0);
        assert_eq!(hal::sync::netlock::status(), 0);
        {
            // solock_shared: the shared net lock, the same witness.
            let mut w = solock_shared(&so);
            assert_eq!(hal::sync::netlock::status(), hal::sync::rwlock::RW_READ);
            let l = w.inet().expect("the parts");
            assert_eq!((*cell.get(&l.pcb), *tcp.get(&l.tcp)), (6, 6));
        }
        assert_eq!(so.so_lock.lock().status(), 0);
        inet_free(so);
    }

    /// Since N6.6b1 no internet protocol is `bsd`'s, so `solock` refuses a legacy internet
    /// socket (until then it held it by the net lock alone, no `so_lock`).
    #[test]
    #[should_panic(expected = "solock: socket")]
    fn solock_refuses_an_internet_socket_of_a_legacy_protocol() {
        let _g = net_setup();
        let so = inet_socket(&BsdInet);
        let _w = solock(&so);
    }

    #[test]
    fn bsd_lends_its_holdings_of_a_kernel_inet_socket_for_a_request() {
        let _g = net_setup();
        let so = inet_socket(&KernInet);
        // Nothing asserted while the detached holdings are held: a failure would leave them
        // held for the next test.
        hal::sync::netlock::lock();
        // The caller's detached holding of so_lock: attached for the request, detached back.
        so.so_lock.enter_write_detached();
        let attached = hal::sync::netlock::lend_detached(|net| {
            let mut w = solocked_legacy(&so, Some(net));
            let attached = !so.so_lock.held_detached();
            let parts = w.inet().is_some();
            drop(w);
            attached && parts
        });
        let kept = so.so_lock.held_detached();
        so.so_lock.exit_write_detached();
        // The exclusive net lock alone: so_lock taken for the request, let go by the drop.
        let taken = hal::sync::netlock::lend_detached(|net| {
            let mut w = solocked_legacy(&so, Some(net));
            let held = so.so_lock.lock().status() == hal::sync::rwlock::RW_WRITE;
            let parts = w.inet().is_some();
            drop(w);
            held && parts && so.so_lock.lock().status() == 0
        });
        hal::sync::netlock::unlock();
        assert!(attached, "the caller's holding attached for the request");
        assert!(kept, "the caller's holding detached back");
        assert!(taken, "so_lock taken for the request and let go");
        inet_free(so);
    }

    /// The same refusal in `solocked_legacy` (until N6.6b1 it lent a legacy internet
    /// socket's holdings as they were). No lock is held when it panics.
    #[test]
    #[should_panic(expected = "solocked_legacy: socket")]
    fn solocked_legacy_refuses_an_internet_socket_of_a_legacy_protocol() {
        let _g = net_setup();
        let so = inet_socket(&BsdInet);
        let w = hal::sync::netlock::NETLOCK.read();
        let net = hal::sync::netlock::NetHeld::of(&w).expect("the net lock");
        let _l = solocked_legacy(&so, Some(&net));
    }

    #[test]
    #[should_panic(expected = "the net lock is not lent")]
    fn an_inet_socket_lent_without_the_net_lock_s_proof_panics() {
        let _g = net_setup();
        let so = inet_socket(&KernInet);
        let _w = solocked_legacy(&so, None);
    }

    #[test]
    fn a_kernel_inet_listener_s_children_are_locked_with_its_proof() {
        // N6.6a0's review, should-fix 1: sonewconn's pru_attach and soclose's pru_abort of a
        // queued child reach a kernel internet protocol under the listener's net lock, and
        // the child's lock carries its proof (solock_nonet's), so the protocol can prove it.
        let _g = net_setup();
        let head = inet_socket(&KernInet);
        soreserve(&head, 4096, 4096).expect("reserve");
        head.qlimit.store(5, Ordering::Relaxed);
        {
            let mut w = solock(&head);
            solisten(&head, &mut w, 5).expect("listen");
        }
        ATTACH_PARTS.store(false, Ordering::Relaxed);
        ABORT_PARTS.store(false, Ordering::Relaxed);
        let child = {
            let w = solock(&head);
            let net = w.net_proof();
            sonewconn(
                &head,
                SS_ISCONNECTED,
                crate::net::mbuf::M_WAIT,
                net.as_ref(),
            )
        }
        .expect("a connection");
        sounlock_detached(&child);
        let attached = ATTACH_PARTS.load(Ordering::Relaxed);
        drop(child);
        // The queued child aborted under the listener's lock and proof.
        soclose(&head, 0).expect("close");
        let aborted = ABORT_PARTS.load(Ordering::Relaxed);
        assert!(attached, "pru_attach: the parts open");
        assert!(aborted, "pru_abort: the parts open");
        assert_eq!(head.qlens(), (0, 0));
    }

    #[test]
    #[should_panic(expected = "no proof of the net lock")]
    fn an_inet_socket_locked_nonet_without_a_proof_panics() {
        let _g = net_setup();
        let so = inet_socket(&KernInet);
        let _w = solock_nonet(&so, None);
    }

    #[test]
    fn a_kernel_inet_socket_asleep_under_the_exclusive_net_lock_lets_so_lock_go() {
        let _g = net_setup();
        let so = inet_socket(&KernInet);
        let started = AtomicBool::new(false);
        let (slept, free) = std::thread::scope(|sc| {
            let t = sc.spawn(|| {
                let mut w = solock(&so);
                // Set holding the net lock, which the sleep lets go only once queued.
                started.store(true, Ordering::Release);
                let r = sosleep_nsec(
                    &mut w,
                    so.timeo_chan(),
                    crate::net::sockbuf::PSOCK,
                    "test",
                    INFSLP,
                );
                let held = so.so_lock.lock().status() == hal::sync::rwlock::RW_WRITE
                    && hal::sync::netlock::status() == hal::sync::rwlock::RW_WRITE;
                drop(w);
                r.is_ok() && held
            });
            while !started.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            // The net lock is free only once the sleeper sleeps on it.
            let net = NETLOCK.write();
            let free = so.so_lock.lock().try_write().is_some();
            wakeup(so.timeo_chan());
            drop(net);
            (t.join().unwrap_or(false), free)
        });
        assert!(free, "so_lock let go while asleep");
        assert!(slept, "woken, both locks taken again");
        inet_free(so);
    }

    #[test]
    fn sblock_held_elsewhere_is_ewouldblock_without_wait() {
        let _g = net_setup();
        let so = socket(0);
        let held = so
            .so_rcv
            .sblock(crate::net::sockbuf::SBL_WAIT)
            .expect("sblock");
        std::thread::scope(|s| {
            s.spawn(|| {
                assert_eq!(so.so_rcv.sblock(0).err(), Some(Errno::EWOULDBLOCK));
            });
        });
        drop(held);
        assert!(so.so_rcv.sblock(0).is_ok());
        so.st_set(SS_NOFDREF);
        drop(sofree(&so, solock(&so), false));
    }

    #[test]
    fn the_options_and_their_values_round_trip() {
        let _g = net_setup();
        let so = socket(0);
        let mut m = crate::net::mbuf::m_get(crate::net::mbuf::M_WAIT, 4).expect("an mbuf");
        sogetopt(
            &so,
            sysdefs::socket::SOL_SOCKET,
            sysdefs::socket::SO_TYPE,
            &mut m,
        )
        .expect("SO_TYPE");
        assert_eq!(opt_int(&m), Some(sysdefs::socket::SOCK_STREAM));
        sogetopt(
            &so,
            sysdefs::socket::SOL_SOCKET,
            sysdefs::socket::SO_RCVBUF,
            &mut m,
        )
        .expect("SO_RCVBUF");
        assert_eq!(opt_int(&m), Some(4096));
        assert_eq!(
            sogetopt(&so, 6, 1, &mut m).err(),
            Some(Errno::ENOPROTOOPT),
            "a level the protocol does not take"
        );
        so.st_set(SS_NOFDREF);
        drop(sofree(&so, solock(&so), false));
    }
}
/* </TESTS> */
