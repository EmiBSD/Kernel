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
//! Host microbenchmarks of `bsd`'s run and sleep queues (decision 43; `sys/kern/kern_sched.rs`,
//! `sys/kern/kern_synch.rs`): what putting a thread on and off a queue costs over the host's
//! machine double (`sys/arch/host`) and the kernel's doubles (`kernel::testutil`). Never part
//! of a kernel image; it measures and changes nothing.
//!
//! `cargo bench -p bsd --bench legacy_sched` (release: the `bench` profile) or with
//! `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is:
//! - `prim.setrunqueue`: `setrunqueue(ci, p, prio)` of a thread on no queue, under the
//!   scheduler lock taken once per round of 1,000 threads (one priority: the tail insert).
//! - `prim.remrunqueue`: `remrunqueue(p)` of those 1,000 threads, first to last, under the
//!   same lock hold.
//! - `prim.sleep-queue-insert`: `sleep_setup(ident, PWAIT, ..)` of the current thread (which
//!   takes and releases the scheduler lock itself) and its `unsleep` under the lock: a
//!   thread's trip through a sleep queue without the switch.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::ptr;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use std::vec::Vec;

use bsd::kern::kern_sched::{remrunqueue, setrunqueue};
use bsd::kern::kern_synch::{sleep_setup, unsleep};
use bsd::kern::proc_glue::{BsdProcessG, BsdThreadG};
use bsd::kern::sched_bsd::{sched_lock, sched_lock_init, sched_unlock};
use bsd::machine::cpu::curcpu;
use bsd::machine::{Cpu, Machine};
use bsd::sys::param::PWAIT;
use bsd::sys::proc::{P_INSCHED, Proc, ProcExt, Process, SONPROC};

/// Threads per round.
const N: usize = 1000;

/// Rounds per repetition.
const ROUNDS: usize = 50;

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

/// Min, median and max of `v`.
fn spread(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

/// A thread on no queue, placed, as `thread_new` leaves one, of a process of its own
/// (leaked: the bench's threads live for the run).
fn thread() -> &'static Proc {
    let pr: &'static Process = Box::leak(Box::new(Process::new(1, BsdProcessG::new())));
    let p: &'static Proc = Box::leak(Box::new(Proc::new_static(
        pr,
        hal::list::ListEntry::new(),
        BsdThreadG::new(),
    )));
    p.place();
    p
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    kernel::testutil::install();
    sched_lock_init();

    let names = [
        "prim.setrunqueue",
        "prim.remrunqueue",
        "prim.sleep-queue-insert",
    ];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    let ci = curcpu();
    let threads: Vec<&'static Proc> = (0..N).map(|_| thread()).collect();
    let prio = 50u8;

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

    if wanted(names[0]) || wanted(names[1]) {
        let mut ons = Vec::new();
        let mut offs = Vec::new();
        // One warm-up repetition, then `REPS` timed.
        for rep in 0..=REPS {
            let mut on = Duration::ZERO;
            let mut off = Duration::ZERO;
            for _ in 0..ROUNDS {
                sched_lock();
                let t = Instant::now();
                for &p in &threads {
                    setrunqueue(Some(ci), black_box(p), prio);
                }
                on += t.elapsed();
                let t = Instant::now();
                for &p in &threads {
                    remrunqueue(black_box(p));
                }
                off += t.elapsed();
                sched_unlock();
            }
            if rep > 0 {
                ons.push(on.as_nanos() as f64 / (N * ROUNDS) as f64);
                offs.push(off.as_nanos() as f64 / (N * ROUNDS) as f64);
            }
        }
        if wanted(names[0]) {
            report(names[0], spread(ons));
        }
        if wanted(names[1]) {
            report(names[1], spread(offs));
        }
    }

    if wanted(names[2]) {
        let p = threads[0];
        p.set_stat(SONPROC);
        Machine::set_curproc(ci, p);
        let ident = ptr::from_ref(&threads).cast::<core::ffi::c_void>();
        let r = measure(N * ROUNDS, || {
            let t = Instant::now();
            for _ in 0..N * ROUNDS {
                sleep_setup(black_box(ident), PWAIT, "bench");
                sched_lock();
                unsleep(p);
                p.set_stat(SONPROC);
                p.p_flag.fetch_and(!P_INSCHED, Ordering::Relaxed);
                sched_unlock();
            }
            t.elapsed()
        });
        Machine::set_curproc(ci, ptr::null());
        report(names[2], r);
    }
}
/* </CODE> */
