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
/* </LICENSES> */

/* <CODE> */
//! The host tests' doubles of the bridges (`cfg(test)` only, declared so in `lib.rs`).
//!
//! The tests share one process and the registry is set once, so [`install`] registers every
//! double the first time any test calls it and does nothing afterwards; every test that
//! reaches a bridge calls it first. The doubles keep their state per test thread where a test
//! asserts on it (the interrupt level, the spin count), so tests running in parallel do not
//! see each other's.
//!
//! - `IntrLevel`: a per-thread level; `splraise` never lowers it, `splx` restores; the
//!   `IPL_MPFLOOR` of the host double of `bsd` (9).
//! - `CpuId`: each test thread is a CPU of its own (its index from a counter, its word the
//!   address of a per-thread static), so two test threads contend for a lock and update
//!   per-CPU counters as two CPUs do; `ncpusfound` is `MAXCPUS`.
//! - `Sleep`: each test thread is a process (its word the address of another per-thread
//!   static); a wait channel has a generation number that every wakeup bumps, `sleep_setup`
//!   records it and `sleep_finish` waits on one condition variable until it moves (so a wakeup
//!   between the two is never lost, as with the C's sleep queue lock) or the timeout
//!   (`EWOULDBLOCK`); no signals.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, Once};
use std::time::{Duration, Instant};

use crate::bridges;
use crate::cpu::CpuId;
use crate::errno::Errno;
use crate::intr::IntrLevel;
use crate::sleep::{INFSLP, Sleep};

std::thread_local! {
    static SPL: Cell<i32> = const { Cell::new(0) };
    static SPINNING: Cell<isize> = const { Cell::new(0) };
    static MUTEX_LEVEL: Cell<i32> = const { Cell::new(0) };
    static WORD: u8 = const { 0 };
    static CPU: usize = NEXT_CPU.fetch_add(1, Ordering::Relaxed);
    static PROC: u8 = const { 0 };
    static PENDING: Cell<Option<(usize, u64)>> = const { Cell::new(None) };
}

/// The generation number of every wait channel a wakeup touched.
static QUEUES: Mutex<BTreeMap<usize, u64>> = Mutex::new(BTreeMap::new());
/// Signalled by every wakeup.
static WAKE: Condvar = Condvar::new();

/// The index the next test thread gets.
static NEXT_CPU: AtomicUsize = AtomicUsize::new(0);

/// The interrupt-level double.
struct TestIntr;

impl IntrLevel for TestIntr {
    fn splraise(&self, ipl: i32) -> i32 {
        SPL.with(|l| {
            let old = l.get();
            l.set(old.max(ipl));
            old
        })
    }

    fn splx(&self, s: i32) {
        SPL.with(|l| l.set(s));
    }

    fn mpfloor(&self) -> i32 {
        9
    }
}

/// The CPU double.
struct TestCpu;

impl CpuId for TestCpu {
    fn cpu_number(&self) -> usize {
        CPU.with(|c| *c) % crate::cpu::MAXCPUS
    }

    fn curcpu_word(&self) -> usize {
        // The address of a per-thread static: distinct per live thread, never 0, and with
        // the two low bits clear (the locks' flag bits) once aligned down.
        WORD.with(|w| core::ptr::from_ref(w).addr() & !0xf) | 0x10
    }

    fn spinning(&self, delta: isize) {
        SPINNING.with(|s| s.set(s.get() + delta));
    }

    fn past_caring(&self) -> bool {
        false
    }

    fn mutex_level(&self, delta: i32) {
        MUTEX_LEVEL.with(|m| m.set(m.get() + delta));
    }

    fn ncpus(&self) -> usize {
        crate::cpu::MAXCPUS
    }
}

/// The sleep queue double.
struct TestSleep;

/// The queues, a panicking test's poison ignored (the map is consistent at every unlock).
fn queues() -> MutexGuard<'static, BTreeMap<usize, u64>> {
    QUEUES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Sleep for TestSleep {
    fn curproc_word(&self) -> usize {
        PROC.with(|p| core::ptr::from_ref(p).addr() & !0xf) | 0x10
    }

    fn sleep_setup(&self, ident: *const (), _prio: i32, _wmesg: &'static str) {
        let q = queues();
        let chan = ident.addr();
        let generation = q.get(&chan).copied().unwrap_or(0);
        PENDING.with(|p| p.set(Some((chan, generation))));
    }

    fn sleep_finish(&self, nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
        let Some((chan, generation)) = PENDING.with(Cell::take) else {
            return Ok(());
        };
        if !do_sleep {
            return Ok(());
        }
        let deadline = (nsecs != INFSLP).then(|| Instant::now() + Duration::from_nanos(nsecs));
        let mut q = queues();
        loop {
            if q.get(&chan).copied().unwrap_or(0) != generation {
                return Ok(());
            }
            match deadline {
                None => {
                    q = WAKE
                        .wait(q)
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                }
                Some(d) => {
                    let now = Instant::now();
                    if now >= d {
                        return Err(Errno::EWOULDBLOCK);
                    }
                    q = WAKE
                        .wait_timeout(q, d - now)
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .0;
                }
            }
        }
    }

    fn tsleep_nsec(
        &self,
        ident: *const (),
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        self.sleep_setup(ident, prio, wmesg);
        self.sleep_finish(nsecs, true)
    }

    fn wakeup(&self, ident: *const ()) {
        let mut q = queues();
        *q.entry(ident.addr()).or_insert(0) += 1;
        WAKE.notify_all();
    }

    fn wakeup_one(&self, ident: *const ()) {
        // Waking every sleeper is a correct wakeup_one for the callers: they all retry.
        self.wakeup(ident);
    }

    fn kernel_lock_held(&self) -> bool {
        false
    }
}

static TEST_INTR: TestIntr = TestIntr;
static TEST_CPU: TestCpu = TestCpu;
static TEST_SLEEP: TestSleep = TestSleep;

/// Registers every double, once per process.
pub(crate) fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = bridges::register_intr_level(&TEST_INTR);
        let _ = bridges::register_cpu_id(&TEST_CPU);
        let _ = bridges::register_sleep(&TEST_SLEEP);
    });
}

/// This thread's interrupt level, as the double keeps it.
pub(crate) fn spl_level() -> i32 {
    SPL.with(Cell::get)
}

/// This thread's spin count, as the double keeps it.
pub(crate) fn spinning() -> isize {
    SPINNING.with(Cell::get)
}
/* </CODE> */
