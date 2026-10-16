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
/* </LICENSES> */

/* <CODE> */
//! `Mutexed<T>`: the spinning mutex of `mutex(9)` that owns the data it guards.
//!
//! Upstream: sys/kern/kern_lock.c @ 3ce1f3f79392
//! Upstream: sys/sys/mutex.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_lock.rs@3c62ede68802
//! LZ: sys/sys/mutex.rs@3c62ede68802
//!
//! The behaviour of `mtx_enter(9)`: a mutex is owned by a CPU, never by a process; it is not
//! recursive; entering it raises the interrupt level to the mutex's (at least `IPL_MPFLOOR`
//! under `MULTIPROCESSOR`, `__MUTEX_IPL`) before taking the owner word, so an interrupt
//! handler of that level or below cannot run on this CPU and try to take it again; a CPU that
//! finds it held spins, counted in its scheduler's `spc_spinning`; leaving it releases the
//! owner word and then restores the level saved in the mutex (`mtx_oldipl`). It never
//! sleeps. `ci_mutex_level` is counted under `DIAGNOSTIC`.
//!
//! The machine's `spl(9)` and the CPU's identity arrive through the `IntrLevel` and `CpuId`
//! bridges (`intr.rs`, `cpu.rs`); before they are registered there are no interrupts and one
//! CPU, and a mutex is a compare-and-swap of its owner word, which is all its exclusion ever
//! rests on.
//!
//! ## Deviations
//! - The parking lots of the C's slow path (`mtx_park`, `mtx_enter_park`: a CPU that spun 40
//!   times publishes itself in one of 128 lots with interrupts disabled and waits to be
//!   handed the mutex) need `intr_disable`/`intr_restore`, which `hal` gets with the hardware
//!   at N2.2: until then a contended `Mutexed` keeps spinning on its owner word with
//!   `CPU_BUSY_CYCLE`, the C's behaviour before the lots. Mutual exclusion, the levels and the
//!   counters are the C's; only who wins a contended release may differ.
//! - After a panic or inside ddb (`panicstr || db_active`) the C's `mtx_enter` returns
//!   without the mutex and `mtx_leave` without releasing it, so the debugger and the dump do
//!   not deadlock on a lock a stopped CPU holds. A `Mutexed` cannot hand out its data without
//!   holding the lock (that would be a second `&mut`), so it takes the lock as usual then;
//!   code that must run after a panic (the console output, `cons.rs`) does not take one.
//! - "locking against myself" (entering a mutex this CPU holds) panics in every
//!   configuration, where the C checks it only under `DIAGNOSTIC` (and otherwise spins for
//!   ever under `MULTIPROCESSOR`, or carries on with two owners without it).
//!
//! ## Redesign
//! - LZ's `Mutex` is the C's `struct mutex` and free functions (`mtx_enter(&m)`, then the
//!   protected fields as `Cell`s beside it, each access trusting the caller to hold the
//!   mutex). `Mutexed<T>` holds the data: [`Mutexed::lock`] is `mtx_enter` and returns a
//!   [`MutexedGuard`] through which alone the data is reached, and the guard's drop is
//!   `mtx_leave`. Holding the guard is the proof of the lock; it is not `Send`, as the
//!   mutex belongs to the CPU that entered it.
//! - The owner word is never 0 for an owner (the CPU's word with a flag bit set), so the
//!   compare-and-swap excludes even when the `CpuId` bridge answers wrongly, and the
//!   uniprocessor build takes the same path: on one CPU the swap never fails but on a
//!   recursion, which panics.
//! - `mtx_oldipl` is an atomic written by the owner only, so the type needs no `Cell`.
//! - `mtx_enter_try` is [`Mutexed::try_lock`], `None` when held elsewhere.
//! - The legacy `Mutex` (129 files) keeps its API (the timing rule; decision 8 names the
//!   owning type `Mutexed` beside it, the rename is N9's).

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use crate::cpu;
use crate::intr::{self, IPL_NONE};

/// The flag bit every owner word carries, so an owner's word is never 0 (the free value).
const MTX_OWNED: usize = 0x2;

/// A spinning mutex that owns the `T` it protects (`struct mutex` with its data).
///
/// Invariant: `data` is reached only through a [`MutexedGuard`], and a guard exists only
/// between the compare-and-swap that moved `owner` from 0 to this CPU's word and the store of
/// 0 in the guard's drop; so at most one guard, hence at most one `&mut T`, is alive.
pub struct Mutexed<T> {
    /// `mtx_owner`: 0 when free, else the owning CPU's word.
    owner: AtomicUsize,
    /// `mtx_wantipl`: the level entering raises to (before `__MUTEX_IPL`'s floor).
    wantipl: i32,
    /// `mtx_oldipl`: the level to restore on leave; written by the owner after taking the
    /// mutex, read by the owner before releasing it.
    oldipl: AtomicI32,
    data: UnsafeCell<T>,
}

impl<T> Mutexed<T> {
    /// `MUTEX_INITIALIZER(ipl)` around `value`: a free mutex that raises to `ipl`.
    pub const fn new(ipl: i32, value: T) -> Self {
        Self {
            owner: AtomicUsize::new(0),
            wantipl: ipl,
            oldipl: AtomicI32::new(IPL_NONE),
            data: UnsafeCell::new(value),
        }
    }

    /// `mtx_enter(9)`: raises the level, then takes the mutex, spinning while another CPU
    /// holds it. Panics when this CPU holds it already ("locking against myself").
    pub fn lock(&self) -> MutexedGuard<'_, T> {
        let want = self.want();
        let s = raise(want);
        let me = self_word();
        let found = self.cas(0, me);
        if found != 0 {
            self.against_myself(found, me);
            cpu::spinning(1);
            loop {
                core::hint::spin_loop(); // CPU_BUSY_CYCLE()
                if self.owner.load(Ordering::Relaxed) == 0 && self.cas(0, me) == 0 {
                    break;
                }
            }
            cpu::spinning(-1);
        }
        self.locked(want, s)
    }

    /// `mtx_enter_try(9)`: the guard when the mutex was free; `None` (at the old level) when
    /// another CPU holds it. Panics when this CPU holds it.
    pub fn try_lock(&self) -> Option<MutexedGuard<'_, T>> {
        let want = self.want();
        let s = raise(want);
        let me = self_word();
        let found = self.cas(0, me);
        if found == 0 {
            return Some(self.locked(want, s));
        }
        if want != IPL_NONE {
            intr::splx(s);
        }
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

    /// Whether some CPU holds the mutex (`mtx_owner(mtx) != 0`): a snapshot, for assertions.
    pub fn is_locked(&self) -> bool {
        self.owner.load(Ordering::Relaxed) != 0
    }

    /// `mtx_owned(mtx)`: whether this CPU holds the mutex.
    pub fn owned(&self) -> bool {
        self.owner.load(Ordering::Relaxed) == self_word()
    }

    /// `__MUTEX_IPL(mtx_wantipl)`: under `MULTIPROCESSOR` at least `IPL_MPFLOOR`.
    fn want(&self) -> i32 {
        #[cfg(feature = "multiprocessor")]
        {
            self.wantipl.max(intr::mpfloor())
        }
        #[cfg(not(feature = "multiprocessor"))]
        {
            self.wantipl
        }
    }

    /// `mtx_cas`: the compare-and-swap of the owner word; the value found. Acquire on
    /// success, so the data the previous owner wrote is visible (`membar_enter_after_atomic`).
    fn cas(&self, expect: usize, new: usize) -> usize {
        match self
            .owner
            .compare_exchange(expect, new, Ordering::Acquire, Ordering::Relaxed)
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

    /// The tail of `mtx_enter` once the owner word is ours: the old level saved in the mutex,
    /// `ci_mutex_level` counted, the guard made.
    fn locked(&self, want: i32, s: i32) -> MutexedGuard<'_, T> {
        if want != IPL_NONE {
            self.oldipl.store(s, Ordering::Relaxed);
        }
        #[cfg(feature = "diagnostic")]
        cpu::mutex_level(1);
        MutexedGuard {
            lock: self,
            _not_send: PhantomData,
        }
    }

    /// `mtx_leave(9)`: called by the guard's drop only, so the owner word is ours.
    fn unlock(&self) {
        #[cfg(feature = "diagnostic")]
        cpu::mutex_level(-1);
        let want = self.want();
        let s = self.oldipl.load(Ordering::Relaxed);
        // membar_exit_before_atomic(): the data written under the mutex is visible to the
        // next owner, whose compare-and-swap acquires.
        self.owner.store(0, Ordering::Release);
        if want != IPL_NONE {
            intr::splx(s);
        }
    }
}

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
    _not_send: PhantomData<*const ()>,
}

impl<T> Deref for MutexedGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this guard exists, so its CPU holds the mutex (the type's invariant) and no
        // other guard, hence no other reference to the data, is alive; the shared reference
        // borrows the guard and cannot outlive it.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexedGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as for `deref`: the one guard of the held mutex, borrowed mutably, so this
        // is the only reference to the data while it lives.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MutexedGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

/// Raises to `want` unless it is `IPL_NONE`; the old level.
fn raise(want: i32) -> i32 {
    if want != IPL_NONE {
        intr::splraise_raw(want)
    } else {
        IPL_NONE
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
    use std::vec::Vec;

    use super::*;
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
    fn get_mut_and_into_inner_need_no_lock() {
        let mut m = Mutexed::new(IPL_NONE, 3u8);
        *m.get_mut() = 4;
        assert_eq!(m.into_inner(), 4);
    }
}
/* </TESTS> */
