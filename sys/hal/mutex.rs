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
 * Copyright (c) 2017 Visa Hankala
 * Copyright (c) 2014 David Gwynne <dlg@openbsd.org>
 * Copyright (c) 2004 Artur Grabowski <art@openbsd.org>
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
 * Copyright (c) 2004 Artur Grabowski <art@openbsd.org>
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
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
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
 *	@(#)kern_synch.c	8.6 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `Mutexed<T>`: the spinning mutex of `mutex(9)` that owns the data it guards.
//!
//! Upstream: sys/kern/kern_lock.c @ 3ce1f3f79392
//! Upstream: sys/sys/mutex.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_synch.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_lock.rs@3c62ede68802
//! LZ: sys/sys/mutex.rs@3c62ede68802
//! LZ: sys/kern/kern_synch.rs@3c62ede68802
//!
//! The behaviour of `mtx_enter(9)`: a mutex is owned by a CPU, never by a process; it is not
//! recursive; entering it raises the interrupt level to the mutex's (at least `IPL_MPFLOOR`
//! under `MULTIPROCESSOR`, `__MUTEX_IPL`) before taking the owner word, so an interrupt
//! handler of that level or below cannot run on this CPU and try to take it again; a CPU that
//! finds it held spins, counted in its scheduler's `spc_spinning`; leaving it releases the
//! owner word and then restores the level it replaced. It never sleeps. `ci_mutex_level` is
//! counted under `DIAGNOSTIC`.
//!
//! The machine's `spl(9)` and the CPU's identity arrive through the `IntrLevel` and `CpuId`
//! bridges (`intr.rs`, `cpu.rs`); before they are registered there are no interrupts and one
//! CPU, and a mutex is a compare-and-swap of its owner word, which is all its exclusion ever
//! rests on. The level a mutex raises to is fixed when it is entered, and the guard restores
//! only a level it raised: a mutex entered before the `IntrLevel` bridge exists and left
//! after it was registered restores nothing (the C fixes `mtx_wantipl` at init; this
//! mutex's level, `IPL_MPFLOOR`'s floor included, is read once, at entry).
//!
//! ## Deviations
//! - The parking lots of the C's slow path (`mtx_park`, `mtx_enter_park`: a CPU that spun 40
//!   times publishes itself in one of 128 lots with interrupts disabled and waits to be
//!   handed the mutex) need `intr_disable`/`intr_restore`, which `hal` gets with the hardware
//!   at N4 (intr, N4.1): until then a contended `Mutexed` keeps spinning on its owner word with
//!   `CPU_BUSY_CYCLE`, the C's behaviour before the lots. Mutual exclusion, the levels and the
//!   counters are the C's; only who wins a contended release may differ.
//! - After a panic or inside ddb (`panicstr || db_active`) the C's `mtx_enter` returns
//!   without the mutex and `mtx_leave` without releasing it, so the debugger and the dump do
//!   not deadlock on a lock a stopped CPU holds. A `Mutexed` cannot hand out its data without
//!   holding the lock (that would be a second `&mut`), so it takes the lock as usual then;
//!   code that must run after a panic (the console output, `cons.rs`) does not take one.
//!   `mtx_owned` keeps the C's answer: true after a panic or in ddb, whoever holds the mutex
//!   (`<sys/mutex.h>`), since it only feeds assertions.
//! - "locking against myself" (entering a mutex this CPU holds) panics in every
//!   configuration, where the C checks it only under `DIAGNOSTIC` (and otherwise spins for
//!   ever under `MULTIPROCESSOR`, or carries on with two owners without it).
//! - `mtx_oldipl` is not kept in the mutex: the guard holds the level to restore, which only
//!   the owner ever read.
//!
//! ## Redesign
//! - LZ's `Mutex` is the C's `struct mutex` and free functions (`mtx_enter(&m)`, then the
//!   protected fields as `Cell`s beside it, each access trusting the caller to hold the
//!   mutex). `Mutexed<T>` holds the data: [`Mutexed::lock`] is `mtx_enter` and returns a
//!   [`MutexedGuard`] through which alone the data is reached, and the guard's drop is
//!   `mtx_leave`. Holding the guard is the proof of the lock; it is not `Send`, as the
//!   mutex belongs to the CPU that entered it.
//! - The owner word is never 0 for an owner (the CPU's word with a flag bit set), so the
//!   compare-and-swap excludes even when the `CpuId` bridge answers wrongly, and a machine
//!   with one CPU takes the same path: there the swap never fails but on a recursion, which
//!   panics.
//! - The kernel is always `MULTIPROCESSOR` (decision 40): the level is always raised to at
//!   least `IPL_MPFLOOR`; LZ's uniprocessor `__MUTEX_IPL` (the level unchanged) is gone.
//! - `mtx_enter_try` is [`Mutexed::try_lock`], `None` when held elsewhere.
//! - The legacy `Mutex` (129 files) keeps its API (the timing rule; decision 8 names the
//!   owning type `Mutexed` beside it, the rename is N9's).
//! - N3.4a: `msleep_nsec(ident, &mtx, prio, wmesg, nsecs)` is [`MutexedGuard::sleep_nsec`]:
//!   `&mut self`, so no reference into the data lives across the sleep, and the mutex is held
//!   again when it returns (on whichever CPU the process wakes), `Ok` or `Err`, as
//!   `rwsleep`'s guards (`uvm_pseg_get`'s wait for a pager segment). Its `cold || panicstr`
//!   early return is the C's: the `Sleep` bridge answers `cold()`, and then the mutex is left,
//!   `cold_yield()` gives interrupts their chance at `safepri` and bounces the kernel lock (the
//!   C lowers through the mutex's saved level to the same effect), the mutex is entered again
//!   and 0 returned without sleeping. `msleep_nsec` is `kern_synch.c`'s, hence the third
//!   source.
//! - N3.5 S0: a [`MutexedGuard`] is a witness (`hal::rwlock::Witness`, `Exclusive`,
//!   `Divisible`): the one holder of the mutex, as a writer is of a reader-writer lock, so a
//!   `Guarded` cell keyed to the mutex ([`Mutexed::id`], `Guarded::new_mutexed`) opens through
//!   it (an `INTRSAFE` map, whose lock is `vm_map.mtx` at `IPL_VM` instead of an rwlock).
//! - N4.1: a lock whose data lives elsewhere, in the cells of the objects it guards (the C's
//!   `[T]` fields of every timeout under `timeout_mutex`, `[S]` under `SCHED_LOCK`), is a
//!   [`SharedMutex`], a `Mutexed<()>` whose guard is the cells' witness; a `static` one keys
//!   cells made in a `const` context (`Guarded::new_static_mutexed`, `TIMEOUT_INITIALIZER`).

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::assert::kassert;
use crate::bridges;
use crate::cpu;
use crate::intr::{self, IPL_NONE};
use crate::rwlock::LockId;
use crate::sleep::{INFSLP, PCATCH, PRIMASK, chan};
use crate::sys::errno::Errno;

/// The flag bit every owner word carries, so an owner's word is never 0 (the free value).
const MTX_OWNED: usize = 0x2;

/// A spinning mutex that owns the `T` it protects (`struct mutex` with its data).
///
/// Invariant: `data` is reached only through a [`MutexedGuard`], and a guard can reach it
/// only between the compare-and-swap that moved `owner` from 0 to its CPU's word and the
/// store of 0 that releases it (the guard's drop, or the release inside the guard's own
/// [`MutexedGuard::sleep_nsec`], which holds the guard borrowed mutably until it has taken
/// the mutex again); so at most one guard, hence at most one `&mut T`, can reach the data.
pub struct Mutexed<T> {
    /// `mtx_owner`: 0 when free, else the owning CPU's word.
    owner: AtomicUsize,
    /// `mtx_wantipl`: the level entering raises to (before `__MUTEX_IPL`'s floor).
    wantipl: i32,
    data: UnsafeCell<T>,
}

impl<T> Mutexed<T> {
    /// `MUTEX_INITIALIZER(ipl)` around `value`: a free mutex that raises to `ipl`.
    pub const fn new(ipl: i32, value: T) -> Self {
        Self {
            owner: AtomicUsize::new(0),
            wantipl: ipl,
            data: UnsafeCell::new(value),
        }
    }

    /// `mtx_enter(9)`: raises the level, then takes the mutex, spinning while another CPU
    /// holds it. Panics when this CPU holds it already ("locking against myself").
    pub fn lock(&self) -> MutexedGuard<'_, T> {
        let restore = self.enter();
        self.held(restore)
    }

    /// `mtx_enter`'s work: the level raised, the owner word taken (spinning while another CPU
    /// holds it); the level to restore.
    fn enter(&self) -> Option<i32> {
        let restore = self.raise();
        let me = self_word();
        let found = self.swap_in(me);
        if found != 0 {
            self.against_myself(found, me);
            cpu::spinning(1);
            while !(self.owner.load(Ordering::Relaxed) == 0 && self.swap_in(me) == 0) {
                core::hint::spin_loop(); // CPU_BUSY_CYCLE()
            }
            cpu::spinning(-1);
        }
        restore
    }

    /// `mtx_leave`'s work: `ci_mutex_level` uncounted, the owner word released (Release, so
    /// the next owner's swap sees every write made under the mutex), then the level `restore`
    /// says put back.
    fn leave(&self, restore: Option<i32>) {
        #[cfg(feature = "diagnostic")]
        cpu::mutex_level(-1);
        self.owner.store(0, Ordering::Release);
        intr::restore(restore);
    }

    /// `mtx_enter_try(9)`: the guard when the mutex was free; `None` (at the old level) when
    /// another CPU holds it. Panics when this CPU holds it.
    pub fn try_lock(&self) -> Option<MutexedGuard<'_, T>> {
        let restore = self.raise();
        let me = self_word();
        let found = self.swap_in(me);
        if found == 0 {
            return Some(self.held(restore));
        }
        intr::restore(restore);
        self.against_myself(found, me);
        None
    }

    /// The data, through the exclusive reference that proves no guard is alive.
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// The data, consuming the mutex.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// The mutex's identity (the address of its owner word, as a mutex's), for the
    /// [`crate::rwlock::Guarded`] cells keyed to it: its guard is their witness.
    pub fn id(&self) -> LockId {
        LockId::of_mutex(&self.owner)
    }

    /// The owner word, whose address is the mutex's identity: what a `const`-made
    /// [`crate::rwlock::Guarded`] keeps of a `static` mutex.
    pub(crate) const fn owner_word(&self) -> &AtomicUsize {
        &self.owner
    }

    /// Whether some CPU holds the mutex (`mtx_owner(mtx) != 0`): a snapshot, for assertions.
    pub fn is_locked(&self) -> bool {
        self.owner.load(Ordering::Relaxed) != 0
    }

    /// `mtx_owned(mtx)`: whether this CPU holds the mutex, or the kernel is past caring (after
    /// a panic, in ddb), as the C answers for its assertions.
    pub fn owned(&self) -> bool {
        self.owner.load(Ordering::Relaxed) == self_word() || cpu::past_caring()
    }

    /// Raises to `__MUTEX_IPL(mtx_wantipl)` (`MULTIPROCESSOR`'s: at least `IPL_MPFLOOR`),
    /// read once, here; what [`intr::raise`] says to restore.
    fn raise(&self) -> Option<i32> {
        let want = self.wantipl.max(intr::mpfloor());
        if want == IPL_NONE {
            None
        } else {
            intr::raise(want)
        }
    }

    /// One attempt at the owner word, free to `me`; the value found (0 when taken). Acquire
    /// on success, so the data the previous owner wrote is visible.
    fn swap_in(&self, me: usize) -> usize {
        match self
            .owner
            .compare_exchange(0, me, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(found) | Err(found) => found,
        }
    }

    /// The `DIAGNOSTIC` check of the C, kept in every configuration: entering a mutex this
    /// CPU holds would make two guards.
    fn against_myself(&self, found: usize, me: usize) {
        if found == me {
            crate::assert::hal_panic(format_args!(
                "mtx {:p}: locking against myself",
                core::ptr::from_ref(self)
            ));
        }
    }

    /// The owner word is ours: `ci_mutex_level` counted, the guard made with what to restore.
    fn held(&self, restore: Option<i32>) -> MutexedGuard<'_, T> {
        #[cfg(feature = "diagnostic")]
        cpu::mutex_level(1);
        MutexedGuard {
            lock: self,
            restore,
            _not_send: PhantomData,
        }
    }
}

/// A mutex that guards no data of its own but the [`crate::rwlock::Guarded`] cells keyed to it
/// (`Guarded::new_mutexed`, `Guarded::new_static_mutexed`): its guard is their witness. The
/// C's `struct mutex` beside the fields it guards in other objects (`timeout_mutex`,
/// `SCHED_LOCK`).
pub type SharedMutex = Mutexed<()>;

impl<T: Default> Default for Mutexed<T> {
    fn default() -> Self {
        Self::new(IPL_NONE, T::default())
    }
}

// SAFETY: the mutex hands its `T` to one CPU at a time (the invariant), so sharing
// `&Mutexed<T>` across CPUs moves access to the `T` between them, which needs `T: Send` and
// nothing more: no two CPUs ever see the `T` at once.
unsafe impl<T: Send> Sync for Mutexed<T> {}

/// The proof of holding a [`Mutexed`]: the data through `Deref`/`DerefMut`; its drop is
/// `mtx_leave`. Not `Send`: the mutex belongs to the CPU that entered it, at its level.
#[must_use = "dropping the guard at once leaves the mutex"]
pub struct MutexedGuard<'a, T> {
    lock: &'a Mutexed<T>,
    /// The level entering replaced, `None` when it raised none.
    restore: Option<i32>,
    _not_send: PhantomData<*const ()>,
}

impl<T> MutexedGuard<'_, T> {
    /// `msleep_nsec(ident, mtx, prio, wmesg, nsecs)`: queued on `ident` while the mutex is
    /// still held (so the wakeup that follows a change under it cannot be missed), the mutex
    /// left, asleep, and the mutex entered again before the sleep's result is returned:
    /// `EWOULDBLOCK` after `nsecs` ([`INFSLP`]: never), `EINTR`/`ERESTART` on a signal with
    /// `PCATCH` in `prio`. `&mut self`, so no reference into the data lives across the sleep.
    /// While `cold || panicstr` it does not sleep: the mutex is left, interrupts get their
    /// chance at `safepri`, the kernel lock is bounced, the mutex is entered again and `Ok`
    /// returned. Before the sleep queues exist no one could wake a sleeper either: the mutex
    /// is left, the CPU let breathe and the mutex entered again.
    pub fn sleep_nsec<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        kassert!(prio & !(PRIMASK | PCATCH) == 0);
        let m = self.lock;
        let result = match bridges::sleep() {
            Some(b) if b.cold() => {
                // After a panic, or during autoconfiguration: give interrupts a chance, then
                // return; no other process may run, in case this is the idle one.
                m.leave(self.restore);
                b.cold_yield();
                Ok(())
            }
            Some(b) => {
                b.sleep_setup(chan(ident), prio, wmesg);
                // Signal may stop the process: the mutex is left before the sleep.
                m.leave(self.restore);
                b.sleep_finish(nsecs, true)
            }
            None => {
                m.leave(self.restore);
                core::hint::spin_loop();
                Ok(())
            }
        };
        // Entered again on whichever CPU the process woke on, at that CPU's level.
        self.restore = m.enter();
        #[cfg(feature = "diagnostic")]
        cpu::mutex_level(1);
        result
    }

    /// The identity of the mutex this guard holds.
    #[inline]
    pub(crate) fn mutex_id(&self) -> LockId {
        self.lock.id()
    }

    /// `msleep_nsec` with no timeout.
    pub fn sleep<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.sleep_nsec(ident, prio, wmesg, INFSLP)
    }
}

impl<T> Deref for MutexedGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: `&self` is a borrow of the guard outside its own `sleep_nsec` (which holds
        // it mutably borrowed while the mutex is released), so the guard's CPU holds the
        // mutex (the type's invariant) and no other guard, hence no other reference to the
        // data, can reach it; the shared reference borrows the guard and cannot outlive it.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexedGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as for `deref`: the one guard of the held mutex, borrowed mutably, so this
        // is the only reference to the data while it lives (and `sleep_nsec` cannot run
        // while it does).
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MutexedGuard<'_, T> {
    /// `mtx_leave(9)`: the owner word released, then the level this guard raised put back.
    fn drop(&mut self) {
        self.lock.leave(self.restore);
    }
}

/// `mtx_curcpu()` as the owner word: never 0.
fn self_word() -> usize {
    cpu::curcpu_word() | MTX_OWNED
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use std::vec::Vec;

    use super::*;
    use crate::sleep::PWAIT;
    use crate::testutil;

    #[test]
    fn the_guard_reaches_the_data_and_releases_on_drop() {
        testutil::install();
        let m = Mutexed::new(5, 0u32);
        {
            let mut g = m.lock();
            *g += 1;
            assert!(m.is_locked());
            assert!(m.owned());
        }
        assert!(!m.is_locked());
        assert_eq!(*m.lock(), 1);
    }

    #[test]
    fn entering_raises_the_level_and_leaving_restores_it() {
        testutil::install();
        let base = testutil::spl_level();
        let m = Mutexed::new(base + 4, ());
        {
            let _g = m.lock();
            assert!(testutil::spl_level() >= base + 4);
        }
        assert_eq!(testutil::spl_level(), base);
    }

    #[test]
    fn try_lock_fails_while_another_cpu_holds_it() {
        testutil::install();
        let m = Arc::new(Mutexed::new(IPL_NONE, 0u32));
        let g = m.lock();
        let m2 = Arc::clone(&m);
        let other = thread::spawn(move || m2.try_lock().is_none());
        assert!(other.join().unwrap());
        drop(g);
        assert!(m.try_lock().is_some());
    }

    #[test]
    #[should_panic(expected = "locking against myself")]
    fn entering_twice_on_one_cpu_panics() {
        testutil::install();
        let m = Mutexed::new(IPL_NONE, ());
        let _a = m.lock();
        let _b = m.lock();
    }

    #[test]
    fn contending_cpus_exclude_each_other() {
        testutil::install();
        let m = Arc::new(Mutexed::new(IPL_NONE, 0u64));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let m = Arc::clone(&m);
                thread::spawn(move || {
                    for _ in 0..10_000 {
                        let mut g = m.lock();
                        // A non-atomic read-modify-write: a lost update would show a race.
                        let v = *g;
                        *g = v + 1;
                    }
                    testutil::spinning()
                })
            })
            .collect();
        for t in threads {
            // Every spin a thread counted, it uncounted.
            assert_eq!(t.join().unwrap(), 0);
        }
        assert_eq!(*m.lock(), 40_000);
    }

    #[test]
    fn the_guard_sleeps_without_the_mutex_and_wakes_holding_it() {
        testutil::install();
        let m = Arc::new(Mutexed::new(IPL_NONE, false));
        let chan_word = Arc::new(0u8);
        let (m2, c2) = (Arc::clone(&m), Arc::clone(&chan_word));
        let sleeper = thread::spawn(move || {
            let mut g = m2.lock();
            while !*g {
                g.sleep(Arc::as_ptr(&c2), PWAIT, "testslp").unwrap();
            }
            // The mutex is held again on return, by this thread.
            m2.owned()
        });
        // The sleeper left the mutex: this thread gets in.
        thread::sleep(Duration::from_millis(20));
        *m.lock() = true;
        crate::sleep::wakeup(Arc::as_ptr(&chan_word));
        assert!(sleeper.join().unwrap());
        assert!(!m.is_locked());
    }

    #[test]
    fn a_timed_guard_sleep_expires_holding_the_mutex_at_its_level() {
        testutil::install();
        let base = testutil::spl_level();
        let m = Mutexed::new(base + 4, 0u8);
        let x = 0u8;
        let mut g = m.lock();
        assert_eq!(
            g.sleep_nsec(&raw const x, PWAIT, "testslp", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        assert!(m.is_locked() && m.owned());
        assert!(testutil::spl_level() >= base + 4);
        *g = 1;
        drop(g);
        assert!(!m.is_locked());
        assert_eq!(testutil::spl_level(), base);
        assert_eq!(*m.lock(), 1);
    }

    #[test]
    fn a_cold_guard_sleep_yields_and_returns_without_sleeping() {
        testutil::install();
        let base = testutil::spl_level();
        let m = Mutexed::new(base + 4, 0u8);
        let x = 0u8;
        let before = testutil::cold_yields();
        testutil::set_cold(true);
        let mut g = m.lock();
        // No timeout and no wakeup: only the early return ends this sleep.
        let slept = g.sleep(&raw const x, PWAIT, "testslp");
        testutil::set_cold(false);
        assert_eq!(slept, Ok(()));
        assert_eq!(testutil::cold_yields(), before + 1);
        // Held again, at the mutex's level, and the old level put back on the leave.
        assert!(m.is_locked() && m.owned());
        assert!(testutil::spl_level() >= base + 4);
        *g = 1;
        drop(g);
        assert_eq!(testutil::spl_level(), base);
        assert_eq!(*m.lock(), 1);
    }

    #[test]
    fn get_mut_and_into_inner_need_no_lock() {
        let mut m = Mutexed::new(IPL_NONE, 3u8);
        *m.get_mut() = 4;
        assert_eq!(m.into_inner(), 4);
    }
}
/* </TESTS> */
