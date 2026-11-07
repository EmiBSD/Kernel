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
 * Copyright (c) 2019-2020 Visa Hankala
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
//! The SMR thread of `smr_call(9)` (`kern_smr.c`'s `smr_thread` and `smr_startup_thread`)
//! and the scheduler's walk of the CPUs a grace period waits for (`smr_grace_wait`'s
//! `CPU_INFO_FOREACH`, `CPU_IS_RUNNING` and `sched_peg_curproc`); the protocol it drives is
//! `hal::smr`'s.
//!
//! Upstream: sys/kern/kern_smr.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_smr.rs@3c62ede68802
//!
//! The thread runs without the kernel lock: it waits for dispatched calls
//! (`hal::smr::wait`: "bored", then "pause" for `SMR_PAUSE` unless expedited), takes them all
//! (`hal::smr::collect`), waits for a grace period over the scheduler's CPUs
//! ([`SchedCpus`]: the thread pegged to each running CPU whose view is behind, whose switch
//! makes it pass `hal::smr::idle`), runs them in their order, and reports a round that took 2
//! seconds or more, at most once every 5 minutes (`ratecheck`).
//!
//! ## Deviations
//! - `WITNESS` and `dt(4)` are not configured (`smr_lock_obj`'s checks, the `TRACEPOINT`s).
//!
//! ## Redesign
//! - LZ's `smr_thread` over `bsd`'s `msleep_nsec`, `SimpleqHead` and `getmicrouptime` (with
//!   an `unsafe` concatenation) -> [`thread`] over `hal::smr`'s `wait`, `collect` and
//!   `Batch::run`, the kernel's clock (`kernel::tc`) and `ratecheck` (`kernel::time`).
//! - `smr_startup_thread`'s `kthread_create(smr_thread, NULL, NULL, "smr")` -> [`start`], a
//!   kernel thread whose start is `kernel::fork::Start::Smr`.
//! - `smr_grace_wait`'s machine walk -> [`SchedCpus`], the `hal::smr::CpuWalk` the thread
//!   hands `hal::smr::grace_wait` (decision 39: a parameter, no bridge).
//! - `smr_startup` is gone: `hal::smr`'s queues are statics (no `SIMPLEQ_INIT`) and the
//!   wakeup timeout is gone with `smr_wakeup` (`hal::smr`'s notes).
//! - `smr_barrier()`/`smr_flush()` are `hal::smr::barrier`, which needs no walk.

use core::marker::PhantomData;

use hal::smr::{self, CpuWalk};

use crate::assert::kpanic;
use crate::clock::Timeval;
use crate::proc::ProcGlue;

/// `smr_logintvl`: how often a slow round is reported.
const SMR_LOGINTVL: Timeval = Timeval::new(300, 0);

/// The CPUs as the scheduler of the glue `G` knows them: the `hal::smr::CpuWalk` of
/// `smr_grace_wait`.
pub struct SchedCpus<G>(PhantomData<fn() -> G>);

impl<G> SchedCpus<G> {
    /// The walk (a value of no size).
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<G> Default for SchedCpus<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: ProcGlue> CpuWalk for SchedCpus<G> {
    fn running(&self, n: usize) -> bool {
        G::cpu_is_running(n)
    }

    fn peg(&self, n: usize) {
        crate::sched::sched_peg_curproc::<G>(n);
    }

    fn unpeg(&self) {
        crate::sched::sched_unpeg_curproc::<G>();
    }
}

/// `smr_startup_thread()`: `main` starts the SMR thread once every CPU partakes in
/// scheduling. `panic(9)` when it cannot be made.
pub fn start<G: ProcGlue>() {
    if crate::thread::kthread_create::<G>(b"smr", crate::fork::Start::Smr).is_err() {
        kpanic(format_args!("could not create smr thread"));
    }
}

/// `smr_thread()`: collects the dispatched calls, waits for a grace period and runs them, for
/// ever, without the kernel lock.
pub fn thread<G: ProcGlue>() -> ! {
    hal::klock::kernel_assert_locked(); // KERNEL_ASSERT_LOCKED()
    hal::klock::kernel_unlock(); // KERNEL_UNLOCK()

    let cpus = SchedCpus::<G>::new();
    let mut loglast = Timeval::new(0, 0);
    loop {
        let _count = round(&cpus, &mut loglast);
    }
}

/// One round of [`thread`]: the dispatched calls waited for, taken, a grace period over
/// `cpus`, the calls run (how many), a round of 2 seconds or more reported (`loglast`, the
/// `ratecheck` state).
fn round(cpus: &dyn CpuWalk, loglast: &mut Timeval) -> u64 {
    smr::wait();
    let batch = smr::collect();

    let start = crate::tc::getmicrouptime();

    smr::grace_wait(cpus);

    // WITNESS_CHECKORDER, WITNESS_LOCK: not configured.
    let count = batch.run();
    // WITNESS_UNLOCK: not configured.

    let end = crate::tc::getmicrouptime();
    let elapsed = end.sub(start);
    if elapsed.tv_sec >= 2 && crate::time::ratecheck(loglast, &SMR_LOGINTVL) {
        hal::prf::printf(format_args!(
            "smr: dispatch took {}.{:06}s\n",
            elapsed.tv_sec, elapsed.tv_usec
        ));
    }
    // TRACEPOINT(smr, thread, TIMEVAL_TO_NSEC(&elapsed), count): dt(4), not configured.
    count
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::ffi::c_void;
    use std::boxed::Box;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::testutil;

    /// No CPU but the caller's.
    struct Alone;

    impl CpuWalk for Alone {
        fn running(&self, _n: usize) -> bool {
            false
        }
        fn peg(&self, n: usize) {
            panic!("peg({n}) with no other CPU");
        }
        fn unpeg(&self) {}
    }

    static CALLS: AtomicU64 = AtomicU64::new(0);

    fn count(_arg: *mut c_void) {
        CALLS.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn a_round_runs_the_calls_dispatched_before_it_after_the_pause() {
        // The page system's lock keeps `hal::smr`'s queues from a test's reset meanwhile.
        let _mem = testutil::MEMORY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        testutil::install();
        let before = CALLS.load(Ordering::SeqCst);
        for _ in 0..3 {
            smr::defer_link(
                Box::leak(Box::new(hal::smr::SmrLink::new())),
                count,
                core::ptr::null_mut(),
            );
        }
        hal::smr::idle(); // this CPU's quiescent state: its calls dispatched
        let t = std::time::Instant::now();
        let mut loglast = Timeval::new(0, 0);
        assert_eq!(round(&Alone, &mut loglast), 3, "the batch, in one round");
        assert!(
            t.elapsed() >= std::time::Duration::from_millis(90),
            "SMR_PAUSE first"
        );
        assert_eq!(CALLS.load(Ordering::SeqCst), before + 3);
        assert_eq!(loglast.tv_sec, 0, "a fast round is not reported");
    }
}
/* </TESTS> */
