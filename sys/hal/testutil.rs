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

use std::cell::Cell;
use std::sync::Once;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::bridges;
use crate::cpu::CpuId;
use crate::intr::IntrLevel;

std::thread_local! {
    static SPL: Cell<i32> = const { Cell::new(0) };
    static SPINNING: Cell<isize> = const { Cell::new(0) };
    static MUTEX_LEVEL: Cell<i32> = const { Cell::new(0) };
    static WORD: u8 = const { 0 };
    static CPU: usize = NEXT_CPU.fetch_add(1, Ordering::Relaxed);
}

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

static TEST_INTR: TestIntr = TestIntr;
static TEST_CPU: TestCpu = TestCpu;

/// Registers every double, once per process.
pub(crate) fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = bridges::register_intr_level(&TEST_INTR);
        let _ = bridges::register_cpu_id(&TEST_CPU);
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
