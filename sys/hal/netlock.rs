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

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::assert::hal_panic;
use crate::rwlock::{RW_WRITE, RawRwlock, rw_self};
use crate::sys::errno::Errno;

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

/// `NET_UNLOCK()`: releases the running process's detached write holding (taken by [`lock`]).
/// `panic(9)` when it has none (not held, held by another, or held through a witness).
pub fn unlock() {
    let me = rw_self();
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
/// (`rw_assert_anylock`).
pub fn sleep_nsec<I: ?Sized>(
    ident: *const I,
    prio: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let me = rw_self();
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
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use std::thread;

    use super::*;
    use crate::rwlock::{RW_READ, RW_WRITE_OTHER};
    use crate::sleep::INFSLP;
    use crate::testutil;

    /// The tests take the one static lock: one at a time (`hal::link`'s too).
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
}
/* </TESTS> */
