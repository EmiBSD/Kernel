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
//! Host microbenchmarks of N4.1's hot paths (decision 43): timeout(9)'s `timeout_add`/
//! `timeout_del`, `timeout_set`, `timeout_hardclock_update` and `softclock`, over the kernel's
//! host doubles (`kernel::testutil`, feature `doubles`). Never part of a kernel image; it
//! measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench timeout` (release: the `bench` profile)
//! or with `--profile dev` (the kernel's opt-level 1). Arguments: `list`; the names of the
//! benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop over LZ's functions, a
//! scratch test of `bsd` at the parent commit):
//! - `timeout.add-del`: `timeout_add(to, 100)` then `timeout_del(to)` of one timeout.
//! - `timeout.set`: `timeout_set(to, fn, arg)` of an idle timeout.
//! - `timeout.hardclock.idle`: `ticks++` and `timeout_hardclock_update()`, the wheel empty.
//! - `timeout.hardclock.1k`: `timeout_hardclock_update()` alone, with 1,000 timeouts on the
//!   wheel, each re-adding itself 100 to 999 ticks out when it runs (the `softclock` that
//!   follows each update is not timed).
//! - `timeout.softclock.100`: `softclock()` running 100 timeouts due now, per timeout (the
//!   100 `timeout_add(to, 0)` before it are not timed).

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::ffi::c_void;
use std::hint::black_box;
use std::pin::Pin;
use std::ptr;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::pin::pin_static;
use kernel::clock::TICKS;
use kernel::testutil;
use kernel::timeout::{self, KCLOCK_NONE, Timeout};

// ---------------------------------------------------------------------------------------
// The harness (vm.rs's).

/// One bench: its name, the ops one call of `f` makes, and `f`, which returns the time of
/// the part it measures (all of it, or a phase).
struct Bench<'a> {
    name: &'static str,
    ops: usize,
    f: Box<dyn FnMut() -> Duration + 'a>,
}

/// The calls of a primitive per timed call.
const PRIM: usize = 1000;

/// Times `PRIM` calls of `f`.
fn prim(mut f: impl FnMut()) -> impl FnMut() -> Duration {
    move || {
        let t = Instant::now();
        for _ in 0..PRIM {
            f();
        }
        t.elapsed()
    }
}

/// ns per op of `reps` repetitions of about `target` of wall time each (after a
/// calibration that is the warm-up): min, median, max, and the calls per repetition.
fn measure(b: &mut Bench<'_>, reps: usize, target: Duration) -> (f64, f64, f64, usize) {
    let mut calls = 1usize;
    loop {
        let t = Instant::now();
        for _ in 0..calls {
            black_box((b.f)());
        }
        let wall = t.elapsed();
        if wall >= target / 4 || calls >= 1 << 24 {
            let per = wall.as_secs_f64() / calls as f64;
            calls = ((target.as_secs_f64() / per.max(1e-9)) as usize).max(3);
            break;
        }
        calls *= 4;
    }
    let mut v = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut t = Duration::ZERO;
        for _ in 0..calls {
            t += (b.f)();
        }
        v.push(t.as_nanos() as f64 / (calls * b.ops) as f64);
    }
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1], calls)
}

// ---------------------------------------------------------------------------------------
// timeout(9).

/// A timeout function that does nothing.
fn noop(_arg: *mut c_void) {}

/// The loaded wheel's timeouts.
static LOADED: [Timeout; 1000] = [const { Timeout::new(refire, ptr::null_mut()) }; 1000];

/// A loaded timeout's function: re-adds itself, 100 to 999 ticks out by its index (its
/// argument, a number, never an address).
fn refire(arg: *mut c_void) {
    let i = arg.addr();
    pin_static(&LOADED[i]).add(100 + (i % 900) as i32);
}

/// The timeouts `softclock.100` runs.
static DUE: [Timeout; 100] = [const { Timeout::new(noop, ptr::null_mut()) }; 100];

/// One tick: `ticks++`, the update, the soft interrupt's work.
fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
    timeout::hardclock_update();
    timeout::softclock();
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();
    timeout::startup(100, 10_000, 10_000_000);

    static ONE: Timeout = Timeout::new(noop, ptr::null_mut());
    let one: Pin<&'static Timeout> = pin_static(&ONE);

    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "timeout.add-del",
        ops: PRIM,
        f: Box::new(prim(move || {
            black_box(one.add(100));
            black_box(one.del());
        })),
    });
    benches.push(Bench {
        name: "timeout.set",
        ops: PRIM,
        f: Box::new(prim(move || {
            one.set(Some(noop), black_box(ptr::null_mut()), KCLOCK_NONE, 0);
        })),
    });
    benches.push(Bench {
        name: "timeout.hardclock.idle",
        ops: PRIM,
        f: Box::new(prim(|| {
            TICKS.fetch_add(1, Ordering::Relaxed);
            timeout::hardclock_update();
        })),
    });
    benches.push(Bench {
        name: "timeout.hardclock.1k",
        ops: 1,
        f: Box::new(|| {
            TICKS.fetch_add(1, Ordering::Relaxed);
            let t = Instant::now();
            timeout::hardclock_update();
            let d = t.elapsed();
            timeout::softclock();
            d
        }),
    });
    benches.push(Bench {
        name: "timeout.softclock.100",
        ops: DUE.len(),
        f: Box::new(|| {
            for to in &DUE {
                pin_static(to).add(0);
            }
            let t = Instant::now();
            timeout::softclock();
            t.elapsed()
        }),
    });

    // The primitives the ops are made of, to tell the wheel's cost from the lock's.
    static PRIM_MTX: hal::mutex::Mutexed<u64> = hal::mutex::Mutexed::new(hal::intr::IPL_HIGH, 0);
    benches.push(Bench {
        name: "prim.mutex",
        ops: PRIM,
        f: Box::new(prim(|| {
            *PRIM_MTX.lock() += 1;
        })),
    });
    benches.push(Bench {
        name: "prim.nanouptime",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(kernel::clock::nanouptime());
        })),
    });
    benches.push(Bench {
        name: "prim.ticks",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(kernel::clock::ticks());
        })),
    });

    if args.first().map(String::as_str) == Some("list") {
        for b in &benches {
            println!("{}", b.name);
        }
        std::process::exit(0);
    }
    if args.first().map(String::as_str) == Some("--profile") {
        // Loops one bench for SECS seconds, for a sampling profiler attached to the process
        // (`sample <pid>`), as vm.rs.
        let name = args.get(1).cloned().unwrap_or_default();
        let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
        let Some(b) = benches.iter_mut().find(|b| b.name == name.as_str()) else {
            eprintln!("timeout bench: no bench {name}");
            std::process::exit(2);
        };
        eprintln!(
            "pid {} profiling {} for {secs} s",
            std::process::id(),
            b.name
        );
        let end = Instant::now() + Duration::from_secs(secs);
        let mut calls = 0u64;
        let mut t = Duration::ZERO;
        while Instant::now() < end {
            t += (b.f)();
            calls += 1;
        }
        println!(
            "{} {:.1} ns/op over {calls} calls",
            b.name,
            t.as_nanos() as f64 / (calls as f64 * b.ops as f64)
        );
        std::process::exit(0);
    }

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    for b in benches.iter_mut() {
        if !args.is_empty() && !args.iter().any(|a| a == b.name) {
            continue;
        }
        if b.name == "timeout.hardclock.1k" {
            // The 1,000 timeouts on the wheel, spread, then a steady state reached.
            for (i, to) in LOADED.iter().enumerate() {
                to.set(Some(refire), ptr::without_provenance_mut(i), KCLOCK_NONE, 0);
                pin_static(to).add(1 + (i % 999) as i32);
            }
            for _ in 0..2000 {
                tick();
            }
        }
        let (min, med, max, calls) = measure(b, reps, target);
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}% {:>8}",
            b.name,
            min,
            med,
            max,
            (max - min) / med * 100.0,
            calls
        );
        if b.name == "timeout.hardclock.1k" {
            for to in &LOADED {
                to.del();
            }
        }
    }
    std::process::exit(0);
}
/* </CODE> */
