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
//! Host microbenchmarks of the network stack's runtime on LZ's shape (decision 43, N6.0): the
//! net lock (`NET_LOCK`, `bsd`'s `rwlock(9)` over `sys/net/if_.rs`'s `netlock`) and `smr(9)`
//! (`sys/kern/kern_smr.rs`), the "before" rows of `sys/kernel/benches/net.rs`, by the same
//! names, over the host's machine double (`sys/arch/host`) and the kernel's doubles
//! (`kernel::testutil`). Created by N6.0's first commit and deleted by its last, once the
//! kernel's rows are measured (the N5.3 and N5.4 precedent). Never part of a kernel image; it
//! measures and changes nothing.
//!
//! `cargo bench -p bsd --bench net` (release: the `bench` profile) or with `--profile dev`.
//! Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is:
//! - `netlock.paired`: `NET_LOCK()` and `NET_UNLOCK()` (`net_lock`/`net_unlock`) of a free
//!   lock by the one running thread.
//! - `netlock.read`: `NET_LOCK_SHARED()` and `NET_UNLOCK_SHARED()`.
//! - `smr.read`: `smr_read_enter()` and `smr_read_leave()` (nothing without `DIAGNOSTIC`).
//! - `smr.defer`: `smr_call(entry, func, arg)` of an idle entry (`smr_init`) on a CPU that is
//!   not idle: the entry queued on the CPU's list at `splhigh`; the list emptied between
//!   rounds, untimed (since N6.0, the entries run through `hal::smr` and made idle again).

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::ptr;
use std::time::{Duration, Instant};
use std::vec::Vec;

use bsd::kern::kern_smr::{smr_read_enter, smr_read_leave};
use bsd::kern::proc_glue::{BsdProcGlue, BsdProcessG, BsdThreadG};
use bsd::machine::cpu::curcpu;
use bsd::machine::{Cpu, Machine};
use bsd::sys::proc::{Proc, ProcExt, Process, SONPROC};
use bsd::sys::smr::{SmrEntry, smr_call, smr_init};
use bsd::sys::systm::{net_lock, net_lock_shared, net_unlock, net_unlock_shared};

/// Ops per repetition.
const OPS: usize = 100_000;

/// Entries queued per round of `smr.defer`.
const ENTRIES: usize = 1000;

/// Repetitions per bench.
const REPS: usize = 7;

/// ns per op of `REPS` repetitions of `f` (which returns the time of `ops` ops): min, median,
/// max.
fn measure(ops: usize, mut f: impl FnMut() -> Duration) -> (f64, f64, f64) {
    black_box(f());
    let mut v: Vec<f64> = (0..REPS)
        .map(|_| f().as_nanos() as f64 / ops as f64)
        .collect();
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

/// The deferred function of `smr.defer`'s entries.
fn noop(_arg: *mut core::ffi::c_void) {}

/// The bench's grace periods: no CPU but the bench's own.
struct NoOtherCpu;

impl hal::smr::CpuWalk for NoOtherCpu {
    fn running(&self, _n: usize) -> bool {
        false
    }
    fn peg(&self, _n: usize) {}
    fn unpeg(&self) {}
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    kernel::testutil::install();
    kernel::sched::sched_init_cpu::<BsdProcGlue>(hal::cpu::cpu_number());

    let names = ["netlock.paired", "netlock.read", "smr.read", "smr.defer"];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    // A running thread, so the CPU is not its idle thread's (`smr_cpu_is_idle` would dispatch
    // every call to the system's queue).
    let ci = curcpu();
    let pr: &'static Process = Box::leak(Box::new(Process::new(1, BsdProcessG::new())));
    let p: &'static Proc = Box::leak(Box::new(Proc::new_static(pr, BsdThreadG::new())));
    p.place();
    p.set_stat(SONPROC);
    Machine::set_curproc(ci, Some(p));

    println!("# bench min median max ns/op, spread (max-min)/median, reps {REPS}");
    let report = |name: &str, (min, med, max): (f64, f64, f64)| {
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}%",
            name,
            min,
            med,
            max,
            (max - min) / med * 100.0
        );
    };

    if wanted(names[0]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                net_lock();
                net_unlock();
            }
            t.elapsed()
        });
        report(names[0], r);
    }
    if wanted(names[1]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                net_lock_shared();
                net_unlock_shared();
            }
            t.elapsed()
        });
        report(names[1], r);
    }
    if wanted(names[2]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                smr_read_enter();
                black_box(());
                smr_read_leave();
            }
            t.elapsed()
        });
        report(names[2], r);
    }
    if wanted(names[3]) {
        let entries: &'static [SmrEntry] = Box::leak(
            (0..ENTRIES)
                .map(|_| SmrEntry::new())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let rounds = OPS / ENTRIES;
        let r = measure(rounds * ENTRIES, || {
            let mut d = Duration::ZERO;
            for _ in 0..rounds {
                for e in entries {
                    smr_init(e);
                }
                let t = Instant::now();
                for e in entries {
                    smr_call(black_box(e), noop, ptr::null_mut());
                }
                d += t.elapsed();
                // The entries dispatched, run and made idle again, untimed (no SMR thread runs
                // on the host; since N6.0 the queues are hal::smr's).
                hal::smr::idle();
                let b = hal::smr::collect();
                hal::smr::grace_wait(&NoOtherCpu);
                let _ = b.run();
            }
            d
        });
        report(names[3], r);
    }
}
/* </CODE> */
