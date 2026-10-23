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
/* </LICENSES> */

/* <CODE> */
//! Kernel lock and mutexes: `kern/kern_lock.c`, `bsd`'s names over `hal`.
//!
//! Upstream: sys/kern/kern_lock.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_lock.rs@3c62ede68802
//!
//! The kernel lock (`kernel_lock`, the ticket lock `__mp_lock`) is `hal::klock`'s, the
//! mutexes (`struct mutex` with its parking lots, `struct db_mutex`) `hal::mutex`'s since
//! N4.5c; this module keeps the C's functions for `bsd` (`mtx_enter(&m)` in 128 files) and
//! the producer/consumer generation lock (`pc_lock_init`, `pc_sprod_*`, `pc_mprod_*`,
//! `pc_cons_*`), which stays here. `mtx_print_parks` (a debugging printer of the parking
//! lots that nothing calls) is not here; `WITNESS` and `MP_LOCKDEBUG` are not configured.
//!
//! ## Deviations
//! - `membar_producer`/`membar_consumer`/`membar_exit` are `fence`s of the matching
//!   ordering; `membar_enter_after_atomic` is an `Acquire` fence and `membar_enter` a
//!   `SeqCst` one.
//! - `KERNEL_LOCK()`/`KERNEL_UNLOCK()` are `sys::systm::kernel_lock()`/`kernel_unlock()`.
//! - On the host the kernel lock is `hal::klock`'s per-thread double (every test thread is a
//!   CPU of its own there): `__mp_release_all`/`__mp_acquire_count` of `kernel_lock` reach
//!   it; `__mp_lock`/`__mp_unlock` of it (amd64's interrupt entry) take the machine's ticket
//!   lock.
//!
//! ## Redesign
//! - N4.5c: the kernel lock, the mutex and its parking lots, and the debugger's mutex moved
//!   into `hal` (`hal::klock`, `hal::mutex`), where they are written without `unsafe` (LZ: 8
//!   here, 2 in `<sys/mutex.h>`): the interrupt masking of the ticket lock and of the lots is
//!   `hal::cpu`'s guard, a parked waiter a pinned stack element of a list bound to its lot's
//!   lock. The functions below forward to their methods; the uniprocessor branches LZ kept
//!   unbuilt are gone (decision 40); the release checks of N4.2 are `hal::klock`'s.

use core::sync::atomic::{Ordering, fence};

use crate::sys::mutex::{DbMutex, Mutex, mutex_ipl};
use crate::sys::pclock::PcLock;

/// The name `kernel_lock` the C's `__mp_*(&kernel_lock)` calls pass: the lock itself is
/// `hal::klock`'s, reached only through its functions (which refuse a release by a CPU that
/// does not hold it); this value only says which lock is meant.
pub struct KernelLockName;

/// `kernel_lock`.
pub static KERNEL_LOCK: KernelLockName = KernelLockName;

/// `_kernel_lock_init`: `KERNEL_LOCK_INIT()`, once, from `main`: the ticket lock reset and
/// the parking lots checked empty.
pub fn _kernel_lock_init() {
    hal::klock::init();
    hal::mutex::init_parking();
}

/// `_kernel_lock`: acquire the kernel lock. Intended for use in the scheduler and the lower
/// half of the kernel.
pub fn _kernel_lock() {
    hal::klock::kernel_lock();
}

/// `_kernel_unlock`: release the kernel lock.
pub fn _kernel_unlock() {
    hal::klock::kernel_unlock();
}

/// `_kernel_lock_held`: this CPU holds the kernel lock (always true after a panic or in ddb).
pub fn _kernel_lock_held() -> bool {
    hal::klock::kernel_lock_held()
}

/// `__mp_lock(&kernel_lock)`: takes the kernel lock, recursively (an interrupt handler's
/// entry: no scheduler assertion).
pub fn __mp_lock(_mpl: &KernelLockName) {
    hal::klock::mp_lock();
}

/// `__mp_unlock(&kernel_lock)`: drops one level of this CPU's hold (`panic(9)` unless it
/// holds the lock; the last level refuses a witness out).
pub fn __mp_unlock(_mpl: &KernelLockName) {
    hal::klock::kernel_unlock();
}

/// `__mp_release_all(&kernel_lock)`: drops every level of this CPU's hold; returns how many
/// there were (0, releasing nothing, when it holds none).
pub fn __mp_release_all(_mpl: &KernelLockName) -> u32 {
    hal::klock::release_all()
}

/// `__mp_acquire_count(&kernel_lock, count)`: takes it `count` times (what
/// `__mp_release_all` returned).
pub fn __mp_acquire_count(_mpl: &KernelLockName, count: u32) {
    hal::klock::acquire_count(count);
}

/// `__mtx_init`: a free mutex raising to `wantipl`.
pub fn __mtx_init(mtx: &Mutex, wantipl: i32) {
    mtx.init(wantipl);
}

/// `_mtx_init(mtx, ipl)`: `__mtx_init` at `__MUTEX_IPL(ipl)`.
pub fn _mtx_init(mtx: &Mutex, ipl: i32) {
    __mtx_init(mtx, mutex_ipl(ipl));
}

/// `mtx_init_flags(m, ipl, name, flags)`: without `WITNESS`, `name` and `flags` are unused.
pub fn mtx_init_flags(mtx: &Mutex, ipl: i32, _name: Option<&'static str>, _flags: i32) {
    _mtx_init(mtx, ipl);
}

/// `mtx_init(m, ipl)`.
pub fn mtx_init(mtx: &Mutex, ipl: i32) {
    mtx_init_flags(mtx, ipl, None, 0);
}

/// `mtx_enter_try`: takes the mutex if it is free; `false` (at the old level) otherwise.
pub fn mtx_enter_try(mtx: &Mutex) -> bool {
    mtx.enter_try()
}

/// `mtx_enter`: takes the mutex, raising to its level; spins a little, then parks in the
/// mutex's lot until the owner hands it over.
pub fn mtx_enter(mtx: &Mutex) {
    mtx.enter();
}

/// `mtx_leave`: releases the mutex; if CPUs are parked on it, wakes the first one waiting
/// for this mutex, then restores the level it was entered at.
pub fn mtx_leave(mtx: &Mutex) {
    mtx.leave();
}

/// `db_mtx_enter`: takes the debugger's mutex with interrupts off, spinning with exponential
/// back-off while another CPU holds it. Unlike `mtx_enter` it works while `db_active`.
pub fn db_mtx_enter(mtx: &DbMutex) {
    mtx.enter();
}

/// `db_mtx_leave`: releases the debugger's mutex and restores this CPU's interrupt state.
pub fn db_mtx_leave(mtx: &DbMutex) {
    mtx.leave();
}

/// `pc_lock_init`.
pub fn pc_lock_init(pcl: &PcLock) {
    pcl.pcl_gen.store(0, Ordering::Relaxed);
}

/// `pc_sprod_enter`: a single (non-interlocking) producer enters; returns the generation.
pub fn pc_sprod_enter(pcl: &PcLock) -> u32 {
    let generation = pcl.pcl_gen.load(Ordering::Relaxed).wrapping_add(1);
    pcl.pcl_gen.store(generation, Ordering::Relaxed);
    fence(Ordering::Release); // membar_producer()

    generation
}

/// `pc_sprod_leave`.
pub fn pc_sprod_leave(pcl: &PcLock, generation: u32) {
    fence(Ordering::Release); // membar_producer()
    pcl.pcl_gen
        .store(generation.wrapping_add(1), Ordering::Relaxed);
}

/// `pc_mprod_enter`: multiple (interlocking) producers: waits for an even generation and
/// makes it odd with a compare-and-swap; returns the generation entered.
pub fn pc_mprod_enter(pcl: &PcLock) -> u32 {
    let mut generation = pcl.pcl_gen.load(Ordering::Relaxed);
    let next = loop {
        while generation & 1 != 0 {
            core::hint::spin_loop(); // CPU_BUSY_CYCLE()
            generation = pcl.pcl_gen.load(Ordering::Relaxed);
        }

        let next = generation.wrapping_add(1);
        match pcl
            .pcl_gen
            .compare_exchange(generation, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => break next,
            Err(found) => {
                core::hint::spin_loop(); // CPU_BUSY_CYCLE()
                generation = found;
            }
        }
    };

    fence(Ordering::Acquire); // membar_enter_after_atomic()
    next
}

/// `pc_mprod_leave`.
pub fn pc_mprod_leave(pcl: &PcLock, generation: u32) {
    fence(Ordering::Release); // membar_exit()
    pcl.pcl_gen
        .store(generation.wrapping_add(1), Ordering::Relaxed);
}

/// `pc_cons_enter`: a consumer waits for a quiescent generation and records it in `genp`.
pub fn pc_cons_enter(pcl: &PcLock, genp: &mut u32) {
    let mut generation = pcl.pcl_gen.load(Ordering::Relaxed);
    while generation & 1 != 0 {
        core::hint::spin_loop(); // CPU_BUSY_CYCLE()
        generation = pcl.pcl_gen.load(Ordering::Relaxed);
    }

    fence(Ordering::Acquire); // membar_consumer()
    *genp = generation;
}

/// `pc_cons_leave`: `false` if the read was consistent; `true` (with `genp` updated) if a
/// producer intervened and the consumer must retry.
#[must_use]
pub fn pc_cons_leave(pcl: &PcLock, genp: &mut u32) -> bool {
    fence(Ordering::Acquire); // membar_consumer()

    let mut generation = pcl.pcl_gen.load(Ordering::Relaxed);
    if generation & 1 != 0 {
        loop {
            core::hint::spin_loop(); // CPU_BUSY_CYCLE()
            generation = pcl.pcl_gen.load(Ordering::Relaxed);
            if generation & 1 == 0 {
                break;
            }
        }
    } else if generation == *genp {
        return false;
    }

    *genp = generation;
    true
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::intr::IPL_HIGH;
    use crate::sys::mutex::{mtx_curcpu, mtx_owner};

    #[test]
    fn pc_lock_generations() {
        let pcl = PcLock::new();
        let mut g = 0;
        pc_cons_enter(&pcl, &mut g);
        assert_eq!(g, 0);
        assert!(!pc_cons_leave(&pcl, &mut g), "nothing changed");
        let generation = pc_sprod_enter(&pcl);
        assert_eq!(generation, 1);
        pc_sprod_leave(&pcl, generation);
        assert_eq!(pcl.pcl_gen.load(Ordering::Relaxed), 2);
        assert!(pc_cons_leave(&pcl, &mut g), "a producer intervened");
        assert_eq!(g, 2);
        let generation = pc_mprod_enter(&pcl);
        pc_mprod_leave(&pcl, generation);
        pc_lock_init(&pcl);
        assert_eq!(pcl.pcl_gen.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn enter_and_leave_track_the_owner() {
        static M: Mutex = Mutex::new(IPL_HIGH);
        assert_eq!(mtx_owner(&M), 0);
        mtx_enter(&M);
        assert_eq!(mtx_owner(&M), mtx_curcpu());
        mtx_leave(&M);
        assert_eq!(mtx_owner(&M), 0);
        assert!(mtx_enter_try(&M));
        mtx_leave(&M);
        assert_eq!(mtx_owner(&M), 0);
    }

    #[test]
    fn db_mutex_tracks_the_owner() {
        static M: DbMutex = DbMutex::new();
        db_mtx_enter(&M);
        assert_eq!(M.owner(), mtx_curcpu());
        db_mtx_leave(&M);
        assert_eq!(M.owner(), 0);
    }

    #[test]
    fn init_resets() {
        let m = Mutex::new(3);
        mtx_enter(&m);
        mtx_init(&m, 5);
        assert_eq!(mtx_owner(&m), 0);
        // `__MUTEX_IPL`: a level below `IPL_MPFLOOR` is raised to it.
        assert!(5 < crate::machine::intr::IPL_MPFLOOR);
        assert_eq!(m.wantipl(), crate::machine::intr::IPL_MPFLOOR);
    }

    /// The last `KERNEL_UNLOCK` with a witness out panics before the lock is released; the
    /// witness dropped, it releases (`hal::klock`'s check, through `bsd`'s names).
    #[test]
    fn the_last_kernel_unlock_with_a_witness_out_panics() {
        std::thread::spawn(|| {
            _kernel_lock();
            _kernel_lock();
            let r = hal::klock::reader();
            _kernel_unlock();
            assert_eq!(hal::klock::depth(), 1);
            let got = std::panic::catch_unwind(_kernel_unlock);
            let msg = got
                .err()
                .and_then(|e| e.downcast_ref::<std::string::String>().cloned())
                .unwrap_or_default();
            assert!(
                msg.contains("KERNEL_UNLOCK: the kernel lock released with a witness out (1)"),
                "{msg}"
            );
            assert_eq!(hal::klock::depth(), 1, "nothing was released");
            drop(r);
            _kernel_unlock();
            assert_eq!(hal::klock::depth(), 0);
        })
        .join()
        .unwrap();
    }

    /// `__mp_release_all(&kernel_lock)` (the switch, the sleeps) refuses before it touches the
    /// lock.
    #[test]
    fn a_whole_release_with_a_witness_out_panics() {
        std::thread::spawn(|| {
            _kernel_lock();
            let w = hal::klock::witness();
            let got = std::panic::catch_unwind(|| __mp_release_all(&KERNEL_LOCK));
            let msg = got
                .err()
                .and_then(|e| e.downcast_ref::<std::string::String>().cloned())
                .unwrap_or_default();
            assert!(
                msg.contains("__mp_release_all: the kernel lock released with a witness out"),
                "{msg}"
            );
            drop(w);
            assert_eq!(__mp_release_all(&KERNEL_LOCK), 1);
            __mp_acquire_count(&KERNEL_LOCK, 1);
            _kernel_unlock();
            assert_eq!(hal::klock::depth(), 0);
        })
        .join()
        .unwrap();
    }
}
/* </TESTS> */
