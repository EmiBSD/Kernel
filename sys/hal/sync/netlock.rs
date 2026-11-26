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
 *	@(#)if.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (c) 1980, 1986, 1993
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
 *	@(#)if.c	8.3 (Berkeley) 1/4/94
 */

/*-
 * Copyright (c) 1982, 1988, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)systm.h	8.4 (Berkeley) 2/23/94
 */
/* </LICENSES> */

/* <CODE> */
//! The network lock: `if.c`'s `struct rwlock netlock` and `<sys/systm.h>`'s `NET_LOCK()`,
//! `NET_UNLOCK()`, `NET_LOCK_SHARED()`, `NET_UNLOCK_SHARED()` and `rw_status(&netlock)`.
//! "Network stack data structures are, unless stated otherwise, protected by the
//! `NET_LOCK()`. It's a single non-recursive lock for the whole subsystem." The shared form
//! is for the "softnet" thread and for a read-only `ioctl(2)`/`sysctl(2)` or a socket call
//! that has locks of its own for what it changes: never where a member the net lock guards
//! (`[N]`) is written.
//!
//! Upstream: sys/net/if.h @ 3ce1f3f79392
//! Upstream: sys/net/if.c @ 3ce1f3f79392
//! Upstream: sys/sys/systm.h @ 3ce1f3f79392
//! LZ: sys/net/if_.rs@3c62ede68802
//! LZ: sys/sys/systm.rs@3c62ede68802
//!
//! [`NETLOCK`] is the one lock object, `hal`'s since N6.0 (it was `sys/net/if_.rs`'s, reached
//! by the kernel crate through a bridge). New code takes it through its witnesses
//! (`NETLOCK.write()`, `NETLOCK.read()`), whose `Guarded` cells key to it
//! (`Guarded::new_static(&NETLOCK, ..)`, N6.2's socket members). The C's call style, a lock
//! taken in one function and released in another (`NET_LOCK()` in `soo_ioctl`,
//! `NET_UNLOCK()` deep in `ifioctl`; the vnode pager letting go of the lock a faulting
//! process holds), is [`lock`]/[`unlock`] and [`lock_shared`]/[`unlock_shared`]: holdings
//! with no witness, *detached*, which this module marks as its own ([`DETACHED_WRITER`],
//! [`DETACHED_READERS`]) and alone releases. A release finds the mark or is `panic(9)` (the
//! C's `rw_exit` diagnostic): [`unlock`] releases only the detached write holding of the
//! running process, [`unlock_shared`] only one of the detached read holdings, so neither can
//! take a holding a witness owns, and no witness is ever made of a detached holding.
//!
//! **Proofs** (N6.3a-N6.5e). A reader of what the lock guards that is not a cell of its
//! witnesses takes a [`NetHeld`], and every source of one keeps the holding it proves for as
//! long as the proof lives:
//! - [`NetHeld::of`] borrows a witness of [`NETLOCK`], which holds the lock until its drop;
//! - [`lend_detached`] lends one, for a closure, of the running process's detached write
//!   holding;
//! - `hal::ds::link::SaToken::lend_net` (`bsd`'s, for the holdings it has no witness of) lends
//!   one for the life of a guard ([`NetLend`], [`lend_held`]) of the running process's write
//!   holding (detached, or a witness's further up the stack), or of a read share the guard
//!   joins and owns.
//!
//! While a proof of the write holding is lent ([`LENT`]), every release of that holding is
//! refused (`panic(9)`): [`unlock`] and [`sleep_nsec`] for the detached one, the release paths
//! of a write witness of `NETLOCK` for a witness's (`hal::sync::rwlock::WriteWitness`'s drop, its
//! sleeps, `unlocked`, `downgrade`). A read share joined for a lend is a read holding of its
//! own (one more reader on the lock word, `RawRwlock::join_read`), released by the guard's
//! drop: no other release can take it, since [`unlock_shared`] releases only what
//! [`lock_shared`] counted. So no proof outlives its holding, whatever its holder does with
//! the lock meanwhile.
//!
//! A lend never spans a release of the lender's own holding. For a write holding the release
//! is refused (`panic(9)`). A read lend cannot tell the lender's own read holding from
//! another's (`rw_status` says only `RW_READ`), so a release of the lender's own read share
//! while its guard lives ([`unlock_shared`], [`sleep_nsec`]'s read arm,
//! `kernel::net::socket::SoLocked::unlocked`'s shared arm) succeeds and the joined share
//! stays: still sound (the proof keeps its share), but the lender's next [`lock_shared`]
//! waits behind any writer that queued meanwhile, which waits for the lender's own share: a
//! self-deadlock, where a write lend would have been a `panic(9)`. No path does it today
//! (`bsd`'s guards live for the statement that calls into the kernel, and none of those
//! callees lets the net lock go); a check needs to know whose a read share is, the
//! per-thread count named below.
//!
//! **Net cells** (N6.5e). A member the C keeps under the net lock (`[N]`: an interface's
//! address list, a route's `rt_ifa`) that kernel code reaches under every kind of holding,
//! detached ones included, cannot be a cell of [`NETLOCK`]'s witnesses, which a detached
//! holding does not give. It is a *net cell* (`Guarded::new_net`), keyed to an identity of
//! its own ([`NET_CELLS_ID`], a word no lock owns, as the kernel lock's cells), and opened by
//! the witnesses a proof gives: [`NetHeld::reader`] a shared [`NetReader`] (walks and
//! lookups: the C's `NET_ASSERT_LOCKED()`), [`NetHeld::writer`] the one exclusive
//! [`NetWriter`] (changes: `NET_ASSERT_LOCKED_EXCLUSIVE()`). A witness of `NETLOCK` itself
//! opens no net cell (another identity), so every access to one passes this module's checks:
//! - a reader made on a read holding (`rw_status` `RW_READ`) counts nothing; one made on the
//!   running process's write holding is counted in [`NET_BORROW`], and is refused
//!   (`panic(9)`) while the writer is out; with the lock neither written by the running
//!   process nor read, it is refused;
//! - the writer is made only on the running process's write holding with no reader counted
//!   (`NET_BORROW` 0), which it sets to -1 until its drop; on any other holding, or with a
//!   reader or the writer out, it is refused;
//! - the write holding is not let go, nor slept on, while `NET_BORROW` is not 0 ([`unlock`],
//!   [`sleep_nsec`], a write witness's release paths).
//!
//! The invariant, from those checks: (i) a live [`NetHeld`] means the running process holds
//! the net lock, and keeps holding it in the same mode until the proof is gone (the sources
//! above: a borrowed witness, a lend whose holding refuses every release while it lasts, a
//! read share the lend joined and owns); (ii) `NET_BORROW` is changed only by a witness made
//! after [`status`] answered `RW_WRITE`, at its making and at its drop (inside that holding:
//! (iv)), so only by the one process that writes the lock, and what it counts is that
//! holding's witnesses; (iii) a `NetWriter` exists only on that holding, with no reader
//! counted, and no reader can be counted while it lives; no uncounted reader lives then
//! either, since an uncounted reader was made on a read holding of the process that made it
//! (`RW_READ` there, and (i): its proof proves a holding of that process, which a write
//! holding is not), and that read holding, which (iv) keeps, excludes every write holding;
//! (iv) a witness borrows the proof it came from, which lives no longer than the holding it
//! proves (i), so a holding never ends or changes mode under a live witness. So the
//! references into a net cell are one `&mut` (through the writer, borrowed mutably) or `&`s
//! (through readers, or the writer borrowed shared), never both. The cost is the C's under
//! `DIAGNOSTIC` (GENERIC's): a walk loads the lock word once, as `NET_ASSERT_LOCKED()` does;
//! on the write holding it adds an uncontended relaxed read-modify-write at the start and at
//! the end. A lend costs a read-modify-write pair the C lacks: on [`LENT`] (written by the
//! write holder only) for a write holding, on the lock word (which every reader writes) for a
//! joined read share; `bsd`'s calls into the kernel pay it once per call, until they move.
//! The calls made once per packet (on a shared holding, a contended pair each), with the
//! milestone that takes their caller out of `bsd`:
//! - `bsd`'s TCP output through `ip_output`/`ip6_output` (N6.6a); the raw sockets'
//!   unconnected sends through `in_pcbselsrc` (`raw_ip.rs:423`) and `in6_pcbselsrc`
//!   (`raw_ip6.rs:664-666`; UDP's sends, and their lend to `ip_output`/`ip6_output`, are the
//!   kernel's since N6.6b0), and divert's `ipv4_input`/`ipv6_input` and (N6.5e's 5e1) its
//!   `divert_packet`'s walk of an inbound packet's interface addresses
//!   (`sys/netinet/ip_divert.rs`, `sys/netinet6/ip6_divert.rs`) (N6.6b);
//! - pf(4)'s route-to, `rtalloc_mpath` (`sys/net/pf.rs:8184`, `:8376`) and
//!   `ip_forward`/`ip6_forward` (`sys/net/pf.rs:9883`, `:9908`, `sys/net/pf_norm.rs:1329`),
//!   and pf's `rtalloc` (`sys/net/pf.rs:4507`, `:4515`, `:8015`, `:8066`) (N8); a packet
//!   from `bsd`'s `ip_output` into pf's route-to pays two lends; (N6.5e's 5e1) route-to's
//!   read of `rt_ifa` for a loopback source leaving a non-loopback interface
//!   (`sys/net/pf.rs:8219`, `:8410`), a third on that branch only;
//! - IP-in-IP decapsulation, `rtalloc` and `ip_input_if`/`ip6_input_if`
//!   (`sys/netinet/ip_ipip.rs:453`, `:479`, `:483`) (N8); wg(4)'s `ipv4_input`/
//!   `ipv6_input` (N8);
//! - the `Inet::with_net_held` leg: once per broadcast copy and first solicitation, never
//!   per unicast packet (N7, the proof carried by the drivers' output).
//!
//! The exact alternative without contended atomics, for N6.6's planner or N12 (not written):
//! a per-thread count of the read shares the running thread holds, in its
//! `hal::cpu::thread::MachineSlot`, so that a read lend proves the caller's own share by one
//! compare instead of joining one, and [`unlock_shared`] releases only the running thread's
//! own share (one compare per read release), which would also turn a read lend spanning a
//! release of the lender's own share into a `panic(9)`.
//!
//! ## Deviations
//! - The lock carries no `dt(4)` trace index (`rw_init_flags_trace(.., DT_RWLOCK_IDX_NETLOCK)`):
//!   `dt(4)` is not ported (`sys/dev/dt` is absent; `bsd`'s `rwl_traceidx` is never read).
//! - `NET_ASSERT_LOCKED()`, `NET_ASSERT_LOCKED_EXCLUSIVE()` and `NET_ASSERT_UNLOCKED()` are not
//!   here: the C reports a failure through `splassert_fail()` under `splassert_ctl`, both
//!   `bsd`'s (`kern/subr_prf.rs`), out of `hal`'s reach; `bsd`'s three wrappers
//!   (`sys/sys/systm.rs`) make the C's tests over [`status`] and report through them. The
//!   kernel's first caller (N6.2) decides their shape.
//! - The vnode pager lets go of a *detached* write holding only ([`held_detached`]): a holding
//!   a witness owns cannot be released behind it. Today every holder is `bsd`'s (detached),
//!   as the C's; kernel code that holds a witness across a user copy (N6.2) needs its own
//!   answer (the witness's `unlocked`).
//!
//! ## Redesign
//! - LZ's `pub static NETLOCK: Rwlock` in `sys/net/if_.rs` (`bsd`'s `rwlock(9)`), reached by
//!   the kernel crate through the `NetLock` bridge (N3.4c-N5) -> `hal`'s [`RawRwlock`], so
//!   the kernel's network code (N6.1-N6.7) and `bsd`'s 50 callers share one lock and one
//!   protocol, and the kernel's cells can key to it; the bridge and its registration are
//!   gone. `bsd`'s seven `NET_*` wrappers keep their names over this module.
//! - `rwsleep_nsec(ident, &netlock, ..)` (`sosleep_nsec`) is [`sleep_nsec`]: the detached
//!   holding released for the sleep and taken again in its mode.
//! - Kernel code that holds the lock detached (across a `bsd` driver request that may let it
//!   go) and then reads what it guards borrows a [`NetHeld`] from [`lend_detached`] (N6.3b2):
//!   the proof is lent for a closure, and the holding cannot be let go until it returns.
//! - N6.5e: the `[N]` members LZ keeps as `&'static` links checked by `NET_ASSERT_LOCKED()`
//!   (an interface's address list, a route's `rt_ifa`) become net cells opened by
//!   [`NetHeld::reader`] and [`NetHeld::writer`], the kernel lock's idiom (`hal::sync::klock`'s
//!   `reader`/`witness`, per process here, since a net lock holder may sleep and migrate):
//!   the C's two assertions, made in every build, are the checks that keep a cell's readers
//!   and its writer apart; [`unlock`], [`sleep_nsec`] and a write witness's release paths
//!   refuse a witness left out.
//! - N6.5e (review 1's B1): `bsd`'s proof by check of `rw_status` (`SaToken::net_held`, a
//!   `NetHeld<'static>`), which a read holding of another process satisfied and nothing kept
//!   alive, is a lend ([`lend_held`] behind `SaToken::lend_net`: a [`NetLend`] guard, which
//!   `bsd`'s `net_held` returns, so its call sites keep their shape): the write holding of
//!   the running process kept while lent (the release paths refuse it), or a read share the
//!   guard joins. Every proof is exact; the trust premise it rested on is gone.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};

use crate::assert::hal_panic;
use crate::sync::rwlock::{LockId, RW_READ, RW_WRITE, RawRwlock, ReadWitness, Witness, rw_self};
use sysdefs::base::errno::Errno;

/// The proof that the running process holds the net lock, for a reader of what it guards
/// that is not a `Guarded` cell (`hal::ds::link::SaLink::read`, review 2's S-a): made from a
/// witness of [`NETLOCK`] ([`NetHeld::of`], borrowing the witness), or lent to a closure by
/// [`lend_detached`] (the running process's detached write holding) or by
/// `hal::ds::link::SaToken::lend_net` (`bsd`'s holdings, which no witness exists of: the running
/// process's write holding, or a read share joined for the closure). Every source keeps the
/// holding until the proof is gone (the module's notes). One per walk, where the C asserts
/// once (`NET_ASSERT_LOCKED()`). It opens the net cells through the witnesses it gives
/// ([`NetHeld::reader`], [`NetHeld::writer`]). Not `Send`.
///
/// A witness borrows the proof it came from, so it cannot outlive it:
///
/// ```compile_fail
/// use hal::sync::netlock::{NETLOCK, NetHeld};
/// fn main() {
///     let w = NETLOCK.write();
///     let r = {
///         let net = NetHeld::of(&w).unwrap();
///         net.reader()
///     };
///     drop(r);
/// }
/// ```
///
/// nor can a reference into a net cell outlive the witness that opened it:
///
/// ```compile_fail
/// use hal::sync::netlock::{NETLOCK, NetHeld};
/// use hal::sync::rwlock::Guarded;
/// static CELL: Guarded<u32> = Guarded::new_net(7);
/// fn main() {
///     let w = NETLOCK.write();
///     let net = NetHeld::of(&w).unwrap();
///     let v: &u32 = {
///         let r = net.reader();
///         CELL.get(&r)
///     };
///     assert_eq!(*v, 7);
/// }
/// ```
///
/// while the same uses inside their borrows compile and run:
///
/// ```
/// use hal::sync::netlock::{NETLOCK, NetHeld};
/// use hal::sync::rwlock::Guarded;
/// static CELL: Guarded<u32> = Guarded::new_net(7);
/// fn main() {
///     let w = NETLOCK.write();
///     let net = NetHeld::of(&w).unwrap();
///     {
///         let mut wr = net.writer();
///         *CELL.get_mut(&mut wr) += 1;
///     }
///     let r = net.reader();
///     assert_eq!(*CELL.get(&r), 8);
/// }
/// ```
pub struct NetHeld<'a> {
    _held: PhantomData<&'a ()>,
    _not_send: PhantomData<*const ()>,
}

impl<'a> NetHeld<'a> {
    /// The proof from `w`, a witness of the net lock (`NETLOCK.read()`, `NETLOCK.write()`, or
    /// a part of one): `None` for another lock's.
    pub fn of<W: Witness>(w: &'a W) -> Option<Self> {
        (w.lock_id() == NETLOCK.id()).then(Self::new)
    }

    /// The proof again, for a borrow of this one: it cannot outlive this proof, so it cannot
    /// outlive the holding (a socket lock that carries the proof `bsd`'s caller lent and hands
    /// it to the parts it opens, `kernel::net::socket::SoLocked::inet`). No atomic.
    #[inline]
    pub fn reborrow(&self) -> NetHeld<'_> {
        NetHeld::new()
    }

    /// The proof, for `hal` code that has established the holding itself.
    pub(crate) fn new() -> Self {
        Self {
            _held: PhantomData,
            _not_send: PhantomData,
        }
    }

    /// `NET_ASSERT_LOCKED()`, then a shared witness of the net cells that borrows this proof
    /// (walks, lookups). On the running process's write holding it is counted
    /// ([`NET_BORROW`]), and `panic(9)` while the net cells' writer is out (a walk inside a
    /// change, or a change's callee walking: the writer reads through itself instead); on a
    /// read holding nothing is counted. `panic(9)` when the lock is neither written by the
    /// running process nor read (`rw_status` cannot say whose a read holding is, but no proof
    /// outlives its holding, so a live one leaves the status `RW_WRITE` or `RW_READ`).
    pub fn reader(&self) -> NetReader<'_> {
        let registered = match status() {
            RW_WRITE => {
                if let Err(n) = NET_BORROW.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    if n >= 0 { n.checked_add(1) } else { None }
                }) {
                    borrowed("reader", n);
                }
                true
            }
            RW_READ => false,
            s => unproved("reader", "held", s),
        };
        NetReader {
            registered,
            _held: PhantomData,
            _not_send: PhantomData,
        }
    }

    /// `NET_ASSERT_LOCKED_EXCLUSIVE()`, then the exclusive witness of the net cells that
    /// borrows this proof (changes): `panic(9)` unless the running process writes the net
    /// lock, and while a reader or the writer of the net cells is out on that holding (the
    /// caller passes the one it has down instead).
    pub fn writer(&self) -> NetWriter<'_> {
        let s = status();
        if s != RW_WRITE {
            unproved("writer", "held exclusively", s);
        }
        if let Err(n) = NET_BORROW.compare_exchange(0, -1, Ordering::Relaxed, Ordering::Relaxed) {
            borrowed("writer", n);
        }
        NetWriter {
            _held: PhantomData,
            _not_send: PhantomData,
        }
    }
}

/// A shared witness of the net cells ([`NetHeld::reader`]): reads them
/// (`Guarded::get`) while the proof it borrows lives. Counted in [`NET_BORROW`] when it was
/// made on the running process's write holding. Not `Send`.
#[must_use = "a witness opens the net cells only while it lives"]
pub struct NetReader<'a> {
    /// Counted in [`NET_BORROW`] (made on the write holding): its drop takes the count back.
    registered: bool,
    _held: PhantomData<&'a ()>,
    _not_send: PhantomData<*const ()>,
}

/// The exclusive witness of the net cells ([`NetHeld::writer`]): opens them mutably
/// (`Guarded::get_mut`) while the proof it borrows lives. Made only on the running process's
/// write holding with no reader counted; [`NET_BORROW`] is -1 until its drop. Not `Send`.
#[must_use = "a witness opens the net cells only while it lives"]
pub struct NetWriter<'a> {
    _held: PhantomData<&'a ()>,
    _not_send: PhantomData<*const ()>,
}

/// A proof of the net lock lent to `bsd` for the guard's life ([`lend_held`], behind
/// `hal::ds::link::SaToken::lend_net`): the proof ([`NetLend::net`]) borrows the guard, and the
/// guard keeps the holding it proves until its drop: the running process's write holding,
/// counted in [`LENT`] so that its release paths refuse to run, or a read share it joined
/// and owns. A guard never dropped (`mem::forget`) never gives its proof up: a write lend
/// leaves [`LENT`] counted, so the next release of the write holding is `panic(9)`; a read
/// lend leaks its share, so the lock stays read for ever (a hang). Never a proof without its
/// holding. Not `Send`.
#[must_use = "the proof is lent only while the guard lives"]
pub struct NetLend {
    /// The read share joined for the lend; `None` for a lend of the write holding, counted in
    /// [`LENT`] until the drop.
    share: Option<ReadWitness<'static>>,
    /// The proof, lent by [`NetLend::net`] for a borrow of the guard.
    held: NetHeld<'static>,
}

impl NetLend {
    /// The proof, for as long as the guard is borrowed.
    pub fn net(&self) -> &NetHeld<'_> {
        &self.held
    }
}

/// `netlock`: serialize socket operations to ensure no new sleeping points are introduced
/// in IP output paths.
pub static NETLOCK: RawRwlock = RawRwlock::new("netlock");

/// The writer word of the detached write holding ([`lock`]), 0 when there is none. Set by
/// the holder right after it took the lock, cleared by the holder before it lets it go, so
/// it is never another process's word while the lock is held.
static DETACHED_WRITER: AtomicUsize = AtomicUsize::new(0);

/// How many of the lock's read holdings are detached ([`lock_shared`]): incremented after a
/// reader entered, decremented before one leaves, so it never counts a holding a witness
/// owns and the readers' count on the lock word is never below the witnesses'.
static DETACHED_READERS: AtomicU32 = AtomicU32::new(0);

/// How many proofs of the running process's write holding are lent ([`lend_detached`]'s
/// closures, [`lend_held`]'s guards). While one is, the holding cannot be let go: [`unlock`]
/// and [`sleep_nsec`] refuse it for the detached holding, a write witness's release paths
/// for a witness's ([`before_write_release`]), so no proof outlives the holding it proves.
/// Only the write holder lends (the holding is exclusive), and a lend ends before its
/// closure returns or with its guard's drop, both while the holding lives.
static LENT: AtomicU32 = AtomicU32::new(0);

/// The identity of the net cells (`Guarded::new_net`): a word no lock owns, compared and
/// never dereferenced (`LockId`'s rule: no two live locks share one), so no witness but
/// [`NetReader`] and [`NetWriter`] opens a net cell, a witness of [`NETLOCK`] included.
pub(crate) static NET_CELLS_ID: AtomicUsize = AtomicUsize::new(0);

/// The net cells' witnesses out on the write holding: 0 none, `n > 0` that many counted
/// [`NetReader`]s, -1 the [`NetWriter`]. Changed only by the process that writes the net lock
/// ([`NetHeld::reader`] and [`NetHeld::writer`] change it after [`status`] answered
/// `RW_WRITE`, and a witness's drop runs while the holding its proof borrows lives), so
/// relaxed: the running process sees its own changes in order, and the lock's release and
/// acquire order them for the next writer, which finds 0 (no holding is let go with a
/// witness out: the witnesses' borrows; [`unlock`] and [`sleep_nsec`] check it).
static NET_BORROW: AtomicI32 = AtomicI32::new(0);

/// `rw_status(&netlock)`: `RW_WRITE` (this process writes), `RW_WRITE_OTHER`, `RW_READ` or 0
/// (free); the values of `<sys/rwlock.h>`.
#[inline]
pub fn status() -> i32 {
    NETLOCK.status()
}

/// `rw_status(&netlock) == RW_WRITE`: the running process holds the net lock for writing
/// (detached or through a witness).
#[inline]
pub fn write_held() -> bool {
    status() == RW_WRITE
}

/// The running process holds the net lock for writing by [`lock`] (a detached holding, which
/// [`unlock`] may release).
#[inline]
pub fn held_detached() -> bool {
    DETACHED_WRITER.load(Ordering::Acquire) == rw_self() && write_held()
}

/// `NET_LOCK()`: waits for the net lock and takes it for writing, detached (no witness), until
/// [`unlock`].
pub fn lock() {
    let me = NETLOCK.write_wait();
    DETACHED_WRITER.store(me, Ordering::Release);
}

/// The proof of the running process's detached write holding ([`lock`]), lent to `f`: for
/// kernel code under the C's `NET_LOCK()` taken detached (kept across `bsd`'s driver requests,
/// which may let it go and take it again: wg(4)'s `wg_up`) that reads what the net lock
/// guards outside those requests. `panic(9)` unless the running process holds the lock
/// detached; while `f` runs the holding cannot be let go ([`unlock`] and [`sleep_nsec`] of it
/// are `panic(9)`), so the proof, which `f` only borrows, lives no longer than the holding.
pub fn lend_detached<R>(f: impl FnOnce(&NetHeld<'_>) -> R) -> R {
    if !held_detached() {
        not_held("lend", status());
    }
    lend_write(f)
}

/// The proof of the running process's write holding (detached, or a witness's), lent to `f`;
/// the caller has checked the holding. [`LENT`] counts the lend until `f` returns (or
/// unwinds: a host test's panic), and the holding's release paths refuse to run meanwhile.
fn lend_write<R>(f: impl FnOnce(&NetHeld<'_>) -> R) -> R {
    /// The lend's end, on the closure's return or its unwinding.
    struct Lend;
    impl Drop for Lend {
        fn drop(&mut self) {
            LENT.fetch_sub(1, Ordering::AcqRel);
        }
    }

    LENT.fetch_add(1, Ordering::AcqRel);
    let _lend = Lend;
    f(&NetHeld::new())
}

/// A proof of the holding the running process has, lent for the life of the guard it returns
/// ([`NetLend`]), for `bsd`'s holdings, which no witness exists of
/// (`hal::ds::link::SaToken::lend_net`, the C's `NET_ASSERT_LOCKED()` before a call into the
/// kernel): the write holding of the running process (`rw_status` `RW_WRITE`: detached, or a
/// witness's further up the stack), kept while lent (its release paths refuse to run); or,
/// with the lock read (`RW_READ`, whoever reads it: `rw_status` cannot say), a read share
/// joined for the guard (one more reader on the lock word, without waiting: a reader may not
/// wait behind a writer that waits for it), released by its drop. `None` when the lock is
/// neither written by the running process nor read. Costs a read-modify-write pair the C
/// lacks: on [`LENT`] for a write holding, on the lock word for a joined share.
pub(crate) fn lend_held() -> Option<NetLend> {
    match status() {
        RW_WRITE => {
            LENT.fetch_add(1, Ordering::AcqRel);
            Some(NetLend {
                share: None,
                held: NetHeld::new(),
            })
        }
        RW_READ => NETLOCK.join_read().map(|share| NetLend {
            share: Some(share),
            held: NetHeld::new(),
        }),
        _ => None,
    }
}

/// [`unlock`]'s and [`sleep_nsec`]'s refusal while a proof of the holding is lent.
#[inline]
fn not_lent(what: &str) {
    if LENT.load(Ordering::Acquire) != 0 {
        hal_panic(format_args!(
            "netlock: {what} while its holding is lent to a proof"
        ));
    }
}

/// [`unlock`]'s and [`sleep_nsec`]'s refusal while a witness of the net cells is out on the
/// write holding: its holder would go on reading or writing cells the next holder may change.
/// (The witnesses' borrows already prevent it for every proof; the check makes a broken
/// invariant a `panic(9)` at the release.)
#[inline]
fn not_borrowed(what: &str) {
    let n = NET_BORROW.load(Ordering::Relaxed);
    if n != 0 {
        hal_panic(format_args!(
            "netlock: {what} while a net cell witness is out ({n})"
        ));
    }
}

/// A write witness's release of `lock` (its drop, a sleep, `unlocked`, `downgrade`:
/// `hal::sync::rwlock::WriteWitness`), checked first: of [`NETLOCK`], `panic(9)` while a proof of
/// the holding is lent ([`lend_held`]) or a net cell witness is out; of any other lock, one
/// address compare.
#[inline]
pub(crate) fn before_write_release(lock: &RawRwlock, what: &str) {
    if core::ptr::eq(lock, &raw const NETLOCK) {
        not_lent(what);
        not_borrowed(what);
    }
}

/// `NET_UNLOCK()`: releases the running process's detached write holding (taken by [`lock`]).
/// `panic(9)` when it has none (not held, held by another, or held through a witness), while
/// it is lent ([`lend_detached`]), or while a witness of the net cells is out on it.
pub fn unlock() {
    let me = rw_self();
    if DETACHED_WRITER.load(Ordering::Acquire) != me {
        not_held("exit write", status());
    }
    not_lent("exit write");
    not_borrowed("exit write");
    if DETACHED_WRITER
        .compare_exchange(me, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        not_held("exit write", status());
    }
    NETLOCK.exit_write(me);
}

/// `NET_LOCK_SHARED()`: waits for the net lock and takes it shared, detached, until
/// [`unlock_shared`].
pub fn lock_shared() {
    NETLOCK.read_wait();
    DETACHED_READERS.fetch_add(1, Ordering::AcqRel);
}

/// `NET_UNLOCK_SHARED()`: releases one detached read holding (taken by [`lock_shared`];
/// whose, the lock cannot tell, as the C's `rw_exit_read` cannot). `panic(9)` when there is
/// none.
pub fn unlock_shared() {
    if take_reader().is_err() {
        not_held("exit read", status());
    }
    NETLOCK.exit_read();
}

/// One detached read holding claimed for its release.
fn take_reader() -> Result<u32, u32> {
    DETACHED_READERS.try_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1))
}

/// `rwsleep_nsec(ident, &netlock, prio, wmesg, nsecs)`: sleeps on `ident` with a detached
/// holding released, then takes it again in its mode before returning the sleep's result
/// (`EWOULDBLOCK` after `nsecs`; `EINTR`/`ERESTART` on a signal with `PCATCH`): the running
/// process's detached write holding if it has one, else one of the detached read holdings,
/// whoever took it (the readers are counted, not owned, as the C's `rw_exit_read` cannot tell
/// them apart; a witness's reading is never in the count). `panic(9)` when there is neither
/// (`rw_assert_anylock`), for a write holding lent to a proof ([`lend_detached`]), and while a
/// witness of the net cells is out.
pub fn sleep_nsec<I: ?Sized>(
    ident: *const I,
    prio: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let me = rw_self();
    let writer = DETACHED_WRITER.load(Ordering::Acquire) == me;
    if !writer && DETACHED_READERS.load(Ordering::Acquire) == 0 {
        not_held("rwsleep", status());
    }
    not_borrowed("rwsleep");
    if writer {
        not_lent("rwsleep");
    }
    if DETACHED_WRITER
        .compare_exchange(me, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        let r = NETLOCK.sleep_released(ident, prio, wmesg, nsecs, || NETLOCK.exit_write(me));
        lock();
        r
    } else if take_reader().is_ok() {
        let r = NETLOCK.sleep_released(ident, prio, wmesg, nsecs, || NETLOCK.exit_read());
        lock_shared();
        r
    } else {
        not_held("rwsleep", status())
    }
}

/// The `panic(9)` of a release or a sleep without the holding it needs.
#[cold]
fn not_held(what: &str, status: i32) -> ! {
    hal_panic(format_args!(
        "netlock rwlock {:p}: {what} when lock not held (status {status:#x})",
        &raw const NETLOCK
    ))
}

/// The identity the net cells are keyed to and their witnesses answer ([`NET_CELLS_ID`]'s).
#[inline]
pub(crate) fn net_cells_id() -> LockId {
    LockId::of_mutex(&NET_CELLS_ID)
}

/// The `panic(9)` of a net cell witness asked for while others are out on the write holding
/// (`n`: the readers counted, or -1 for the writer).
#[cold]
fn borrowed(what: &str, n: i32) -> ! {
    if n < 0 {
        hal_panic(format_args!(
            "netlock: a net cell {what} while the writer is out"
        ));
    }
    hal_panic(format_args!(
        "netlock: a net cell {what} while {n} readers are out"
    ))
}

/// The `panic(9)` of `NET_ASSERT_LOCKED()` (`needs` "held") or
/// `NET_ASSERT_LOCKED_EXCLUSIVE()` ("held exclusively") failing for a net cell witness.
#[cold]
fn unproved(what: &str, needs: &str, status: i32) -> ! {
    hal_panic(format_args!(
        "netlock: a net cell {what} without the net lock {needs} (status {status:#x})"
    ))
}

impl Drop for NetReader<'_> {
    fn drop(&mut self) {
        if self.registered {
            NET_BORROW.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

impl Drop for NetWriter<'_> {
    fn drop(&mut self) {
        NET_BORROW.store(0, Ordering::Relaxed);
    }
}

impl Drop for NetLend {
    fn drop(&mut self) {
        // A joined share is released by its own drop, after this.
        if self.share.is_none() {
            LENT.fetch_sub(1, Ordering::AcqRel);
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::thread;

    use super::*;
    use crate::sync::rwlock::{Guarded, RW_WRITE_OTHER};
    use crate::sync::sleep::INFSLP;
    use crate::testutil;
    use std::string::String;

    /// The tests take the one static lock: one at a time (`hal::ds::link`'s too).
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    pub(crate) fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn the_witness_forms_and_the_status() {
        testutil::install();
        let _s = serial();
        assert_eq!(status(), 0);
        let w = NETLOCK.write();
        assert_eq!(status(), RW_WRITE);
        assert!(
            write_held() && !held_detached(),
            "a witness is no detached holding"
        );
        thread::scope(|s| {
            s.spawn(|| assert_eq!(status(), RW_WRITE_OTHER));
        });
        drop(w);
        let r = NETLOCK.read();
        // Two readers at once, on two threads.
        thread::scope(|s| {
            s.spawn(|| {
                let r2 = NETLOCK.read();
                assert_eq!(status(), RW_READ);
                drop(r2);
            });
        });
        assert_eq!(status(), RW_READ);
        assert!(!write_held());
        drop(r);
        assert_eq!(status(), 0);
    }

    #[test]
    fn lock_and_unlock_round_trip() {
        testutil::install();
        let _s = serial();
        lock();
        assert!(write_held() && held_detached());
        unlock();
        assert_eq!(status(), 0);
        assert!(!held_detached());
        lock_shared();
        lock_shared();
        assert_eq!(status(), RW_READ);
        unlock_shared();
        unlock_shared();
        assert_eq!(status(), 0);
    }

    #[test]
    fn a_second_unlock_panics() {
        testutil::install();
        let _s = serial();
        lock();
        unlock();
        let r = std::panic::catch_unwind(unlock);
        assert!(r.is_err(), "NET_UNLOCK() of a lock not held");
        assert!(std::panic::catch_unwind(unlock_shared).is_err());
        assert_eq!(status(), 0);
    }

    #[test]
    fn an_unlock_by_another_process_panics() {
        testutil::install();
        let _s = serial();
        lock();
        let other = thread::scope(|s| s.spawn(|| std::panic::catch_unwind(unlock).is_err()).join());
        assert!(matches!(other, Ok(true)), "only the holder releases it");
        assert!(write_held(), "still ours");
        unlock();
    }

    #[test]
    fn a_holding_a_witness_owns_is_never_released_by_the_paired_forms() {
        testutil::install();
        let _s = serial();
        let w = NETLOCK.write();
        assert!(
            std::panic::catch_unwind(unlock).is_err(),
            "NET_UNLOCK under a witness"
        );
        assert!(
            std::panic::catch_unwind(|| sleep_nsec(&raw const SERIAL, 0, "nettest", 1)).is_err(),
            "a sleep under a witness"
        );
        assert!(write_held(), "the witness still holds it");
        drop(w);
        let r = NETLOCK.read();
        assert!(
            std::panic::catch_unwind(unlock_shared).is_err(),
            "NET_UNLOCK_SHARED of a witness's read holding"
        );
        assert_eq!(status(), RW_READ, "the witness's reading stays");
        // A detached reader beside the witness: only its holding is released.
        lock_shared();
        unlock_shared();
        assert!(std::panic::catch_unwind(unlock_shared).is_err());
        assert_eq!(status(), RW_READ);
        drop(r);
        assert_eq!(status(), 0);
    }

    #[test]
    fn a_sleep_keeps_the_mode_it_found() {
        testutil::install();
        let _s = serial();
        let ident = 0u8;
        lock();
        assert_eq!(
            sleep_nsec(&raw const ident, 0, "nettest", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        assert!(held_detached(), "taken again for writing, detached");
        unlock();
        lock_shared();
        assert_eq!(
            sleep_nsec(&raw const ident, 0, "nettest", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        assert_eq!(status(), RW_READ, "taken again shared");
        unlock_shared();
        assert_eq!(status(), 0);
        let r = std::panic::catch_unwind(|| sleep_nsec(&raw const ident, 0, "nettest", INFSLP));
        assert!(r.is_err(), "a sleep without the lock");
    }

    #[test]
    fn a_detached_holding_lends_a_proof_and_cannot_be_let_go_meanwhile() {
        let _s = serial();
        // Not held: no lend.
        assert!(std::panic::catch_unwind(|| lend_detached(|_| ())).is_err());

        lock();
        assert_eq!(lend_detached(|_net| 7), 7);
        // The release refused while lent; the lend ended by the unwinding.
        assert!(std::panic::catch_unwind(|| lend_detached(|_net| unlock())).is_err());
        assert!(held_detached());
        assert_eq!(LENT.load(Ordering::Relaxed), 0);
        unlock();

        // A witness's holding is not lent.
        let w = NETLOCK.write();
        assert!(std::panic::catch_unwind(|| lend_detached(|_| ())).is_err());
        drop(w);
    }

    /// The net cell of the tests (each test that opens it takes `serial()` and sets it first).
    static CELL: Guarded<u32> = Guarded::new_net(0);

    /// The net cells' witnesses counted now on the write holding.
    fn borrow() -> i32 {
        NET_BORROW.load(Ordering::Relaxed)
    }

    #[test]
    fn net_cells_open_through_the_witnesses_of_a_proof_on_every_holding() {
        testutil::install();
        let _s = serial();
        // A write witness's holding: the writer changes, then readers nest, each counted.
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock's witness");
        {
            let mut wr = net.writer();
            assert_eq!(borrow(), -1);
            *CELL.get_mut(&mut wr) = 1;
            assert_eq!(*CELL.get(&wr), 1, "the writer reads through itself");
        }
        assert_eq!(borrow(), 0);
        let r1 = net.reader();
        let r2 = net.reader();
        assert_eq!(borrow(), 2, "readers nest, counted on the write holding");
        assert_eq!((*CELL.get(&r1), *CELL.get(&r2)), (1, 1));
        drop((r1, r2));
        assert_eq!(borrow(), 0);
        *CELL.get_mut(&mut net.writer()) = 2;
        assert_eq!(borrow(), 0, "the writer's drop gives the cells back");
        drop(w);

        // The detached write holding, lent to a closure.
        lock();
        lend_detached(|net| {
            let r = net.reader();
            assert_eq!((*CELL.get(&r), borrow()), (2, 1));
            drop(r);
            *CELL.get_mut(&mut net.writer()) = 3;
        });
        assert_eq!(borrow(), 0);
        unlock();

        // A read holding: readers count nothing.
        let r = NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock's witness");
        let (a, b) = (net.reader(), net.reader());
        assert_eq!((*CELL.get(&a), *CELL.get(&b), borrow()), (3, 3, 0));
        drop((a, b));
        drop(r);
        assert_eq!(status(), 0);
    }

    #[test]
    fn two_processes_read_the_net_cells_under_shared_holdings_at_once() {
        testutil::install();
        let _s = serial();
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock's witness");
            *CELL.get_mut(&mut net.writer()) = 9;
        }
        let both = std::sync::Barrier::new(2);
        thread::scope(|s| {
            for _ in 0..2 {
                s.spawn(|| {
                    let r = NETLOCK.read();
                    let net = NetHeld::of(&r).expect("the net lock's witness");
                    let rd = net.reader();
                    // Both processes hold the lock and a reader here.
                    both.wait();
                    assert_eq!(*CELL.get(&rd), 9);
                    assert_eq!(borrow(), 0, "nothing counted on shared holdings");
                    both.wait();
                });
            }
        });
        assert_eq!(status(), 0);
    }

    #[test]
    #[should_panic(expected = "netlock: a net cell writer while 1 readers are out")]
    fn a_writer_with_a_reader_out_panics() {
        testutil::install();
        let _s = serial();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock's witness");
        let _r = net.reader();
        let _wr = net.writer();
    }

    #[test]
    #[should_panic(expected = "netlock: a net cell reader while the writer is out")]
    fn a_reader_with_the_writer_out_panics() {
        testutil::install();
        let _s = serial();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock's witness");
        let _wr = net.writer();
        let _r = net.reader();
    }

    #[test]
    #[should_panic(expected = "netlock: a net cell writer while the writer is out")]
    fn a_second_writer_panics() {
        testutil::install();
        let _s = serial();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock's witness");
        let _wr = net.writer();
        let _wr2 = net.writer();
    }

    #[test]
    #[should_panic(
        expected = "netlock: a net cell writer without the net lock held exclusively (status 0x2)"
    )]
    fn a_writer_on_a_read_holding_panics() {
        testutil::install();
        let _s = serial();
        let r = NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock's witness");
        let _wr = net.writer();
    }

    /// A proof without its holding (only `hal` can make one, `NetHeld::new`; every source
    /// keeps its holding): the witnesses' own checks refuse it all the same.
    #[test]
    #[should_panic(
        expected = "netlock: a net cell writer without the net lock held exclusively (status 0x0)"
    )]
    fn a_writer_on_no_holding_panics() {
        testutil::install();
        let _s = serial();
        let net = NetHeld::new();
        let _wr = net.writer();
    }

    #[test]
    #[should_panic(expected = "netlock: a net cell reader without the net lock held (status 0x0)")]
    fn a_reader_on_no_holding_panics() {
        testutil::install();
        let _s = serial();
        let net = NetHeld::new();
        let _r = net.reader();
    }

    #[test]
    #[should_panic(expected = "Guarded: a witness of another lock")]
    fn a_read_witness_of_the_net_lock_opens_no_net_cell() {
        testutil::install();
        let _s = serial();
        let r = NETLOCK.read();
        let _ = CELL.get(&r);
    }

    #[test]
    #[should_panic(expected = "Guarded: a witness of another lock")]
    fn a_write_witness_of_the_net_lock_opens_no_net_cell() {
        testutil::install();
        let _s = serial();
        let mut w = NETLOCK.write();
        let _ = CELL.get_mut(&mut w);
    }

    #[test]
    #[should_panic(expected = "Guarded: a witness of another lock")]
    fn a_net_cell_witness_opens_no_cell_of_the_net_lock_itself() {
        static MEMBER: Guarded<u32> = Guarded::new_static(&NETLOCK, 0);
        testutil::install();
        let _s = serial();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock's witness");
        let r = net.reader();
        let _ = MEMBER.get(&r);
    }

    /// A proof made without a lend (only `hal` can, `NetHeld::new`), whose witnesses no
    /// borrow ends: left out, they keep the detached write holding from being let go or slept
    /// on (a broken invariant becomes a `panic(9)` at the release).
    #[test]
    fn a_detached_holding_is_not_let_go_with_a_net_cell_witness_out() {
        testutil::install();
        let _s = serial();
        let ident = 0u8;
        let nap = || sleep_nsec(&raw const ident, 0, "nettest", 1);
        lock();
        let net = NetHeld::new();
        let r = net.reader();
        assert!(
            catch_unwind(unlock).is_err(),
            "NET_UNLOCK with a reader out"
        );
        assert!(catch_unwind(nap).is_err(), "a sleep with a reader out");
        assert!(held_detached() && borrow() == 1, "the holding stays");
        drop(r);
        let w = net.writer();
        assert!(
            catch_unwind(unlock).is_err(),
            "NET_UNLOCK with the writer out"
        );
        assert!(catch_unwind(nap).is_err(), "a sleep with the writer out");
        assert!(held_detached() && borrow() == -1, "the holding stays");
        drop(w);
        assert_eq!(
            sleep_nsec(&raw const ident, 0, "nettest", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        unlock();
        assert_eq!(status(), 0);
    }

    /// review 1's B1: a lend found the lock read by another process (the old proof by check
    /// took that holding for the caller's). The lend joins it, so after the other reader lets
    /// go the lock is still read, by the lend's share, and no writer gets in while a reader
    /// made from the lend reads a net cell.
    #[test]
    fn a_lend_on_another_process_read_holding_joins_it() {
        testutil::install();
        let _s = serial();
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock's witness");
            *CELL.get_mut(&mut net.writer()) = 11;
        }
        let q_reads = std::sync::Barrier::new(2);
        let q_may_go = std::sync::Barrier::new(2);
        let q_gone = std::sync::Barrier::new(2);
        thread::scope(|s| {
            s.spawn(|| {
                let r = NETLOCK.read();
                q_reads.wait();
                q_may_go.wait();
                drop(r);
                q_gone.wait();
            });
            q_reads.wait();
            // This thread holds nothing; the lock is read (by the other one).
            let lend = lend_held().expect("the lock is read");
            let rd = lend.net().reader();
            q_may_go.wait();
            q_gone.wait();
            assert_eq!(status(), RW_READ, "the lend's share still reads the lock");
            assert!(
                NETLOCK.try_write().is_none(),
                "no writer while a reader of the lend reads"
            );
            assert_eq!(*CELL.get(&rd), 11);
            drop(rd);
            drop(lend);
            assert_eq!(status(), 0, "the share went with the guard");
        });
        // Nothing to lend on a free lock, nor on another process's write holding.
        assert!(lend_held().is_none());
        let w = NETLOCK.write();
        thread::scope(|s| {
            s.spawn(|| assert!(lend_held().is_none()));
        });
        drop(w);
    }

    /// A lend of the running process's write holding through a witness further up the stack
    /// (review 1's B1): every release of that witness is refused while the guard lives, so the
    /// proof cannot outlive the holding whatever code holds the witness.
    #[test]
    fn a_write_witness_is_not_let_go_while_a_proof_of_it_is_lent() {
        testutil::install();
        let _s = serial();
        let ident = 0u8;
        let mut w = NETLOCK.write();
        let lend = lend_held().expect("written by this process");
        assert!(
            catch_unwind(AssertUnwindSafe(|| w.unlocked(|| ()))).is_err(),
            "unlocked while lent"
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| w.sleep_nsec(
                &raw const ident,
                0,
                "nettest",
                1
            )))
            .is_err(),
            "a sleep while lent"
        );
        assert!(
            write_held() && !held_detached(),
            "the witness's holding stays"
        );
        drop(lend);
        // The guard gone, the witness lets go and takes back as before.
        w.unlocked(|| assert_eq!(status(), 0));
        assert_eq!(status(), RW_WRITE);

        // Its drop while lent: refused, the holding kept (the test lets it go by hand).
        let lend = lend_held().expect("written by this process");
        assert!(catch_unwind(AssertUnwindSafe(move || drop(w))).is_err());
        assert!(write_held(), "the dropped witness's holding stays");
        drop(lend);
        NETLOCK.exit_write(rw_self());
        assert_eq!(status(), 0);

        // A net cell witness out at the drop is refused the same way.
        let w = NETLOCK.write();
        let net = NetHeld::new();
        let r = net.reader();
        assert!(catch_unwind(AssertUnwindSafe(move || drop(w))).is_err());
        drop(r);
        NETLOCK.exit_write(rw_self());
        assert_eq!(status(), 0);
    }

    /// A guard lent on the detached write holding keeps it: `NET_UNLOCK()` and the sleep are
    /// refused until the guard goes.
    #[test]
    fn a_detached_holding_is_not_let_go_while_a_guard_lends_it() {
        testutil::install();
        let _s = serial();
        let ident = 0u8;
        lock();
        let lend = lend_held().expect("written by this process");
        assert!(catch_unwind(unlock).is_err(), "NET_UNLOCK while lent");
        assert!(
            catch_unwind(|| sleep_nsec(&raw const ident, 0, "nettest", 1)).is_err(),
            "a sleep while lent"
        );
        assert!(held_detached());
        drop(lend);
        unlock();
        assert_eq!(status(), 0);
    }

    /// `NET_UNLOCK()` by a process that does not hold the lock says so, whatever the holder has
    /// lent (review 1's nit: the ownership is checked first).
    #[test]
    fn an_unlock_by_a_non_holder_names_the_holding() {
        testutil::install();
        let _s = serial();
        lock();
        let lend = lend_held().expect("written by this process");
        let msg = thread::scope(|s| {
            s.spawn(|| match catch_unwind(unlock) {
                Ok(()) => String::new(),
                Err(e) => e.downcast_ref::<String>().cloned().unwrap_or_default(),
            })
            .join()
        })
        .expect("the other thread");
        assert!(msg.contains("when lock not held"), "{msg}");
        drop(lend);
        unlock();
    }

    /// A lend refuses `downgrade` (review 2's NB-3): a downgraded holding could be let go as a
    /// read while the proof of the write holding lives.
    #[test]
    fn a_write_witness_does_not_downgrade_while_a_proof_of_it_is_lent() {
        testutil::install();
        let _s = serial();
        let w = NETLOCK.write();
        let lend = lend_held().expect("written by this process");
        assert!(catch_unwind(AssertUnwindSafe(move || drop(w.downgrade()))).is_err());
        assert!(write_held(), "the holding stays written");
        drop(lend);
        NETLOCK.exit_write(rw_self());
        assert_eq!(status(), 0);
    }

    /// A lend refuses `into_sleep_nsec` (`PNORELOCK`, review 2's NB-3).
    #[test]
    fn a_write_witness_does_not_sleep_away_while_a_proof_of_it_is_lent() {
        testutil::install();
        let _s = serial();
        let ident = 0u8;
        let w = NETLOCK.write();
        let lend = lend_held().expect("written by this process");
        assert!(
            catch_unwind(AssertUnwindSafe(move || w.into_sleep_nsec(
                &raw const ident,
                0,
                "nettest",
                1
            )))
            .is_err()
        );
        assert!(write_held(), "the holding stays written");
        drop(lend);
        NETLOCK.exit_write(rw_self());
        assert_eq!(status(), 0);
    }

    /// `NET_UNLOCK_SHARED()` cannot take a joined share (it releases only what
    /// `NET_LOCK_SHARED()` counted), and a lend on the caller's own read holding does not wait
    /// behind a writer that waits for that holding (review 2's NB-3).
    #[test]
    fn a_joined_share_is_the_lend_own() {
        testutil::install();
        let _s = serial();
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock's witness");
            *CELL.get_mut(&mut net.writer()) = 13;
        }
        lock_shared();
        let lend = lend_held().expect("the lock is read");
        let rd = lend.net().reader();
        unlock_shared();
        assert_eq!(status(), RW_READ, "the joined share still reads");
        assert!(
            catch_unwind(unlock_shared).is_err(),
            "no detached share is left"
        );
        assert_eq!((status(), *CELL.get(&rd)), (RW_READ, 13));
        drop(rd);
        drop(lend);
        assert_eq!(status(), 0);

        // A writer queued behind this thread's read: the lend joins at once. (Its answer is
        // checked after the read is let go, so a refusal fails the test instead of leaving the
        // writer waiting for ever.)
        lock_shared();
        let read = thread::scope(|s| {
            let writer = s.spawn(|| {
                let w = NETLOCK.write();
                let net = NetHeld::of(&w).expect("the net lock's witness");
                *CELL.get_mut(&mut net.writer()) = 14;
            });
            while NETLOCK.waiters_for_test() == 0 {
                thread::yield_now();
            }
            let read = lend_held().map(|lend| *CELL.get(&lend.net().reader()));
            unlock_shared();
            writer.join().expect("the writer");
            read
        });
        assert_eq!(read, Some(13), "joined without waiting for the writer");
        let r = NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock's witness");
        assert_eq!(*CELL.get(&net.reader()), 14);
    }
}
/* </TESTS> */
