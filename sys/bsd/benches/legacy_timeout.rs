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
//! Host microbenchmarks of the legacy timeout(9) adapter's paths (decision 43;
//! `sys/sys/timeout.rs`): what `timeout_set` costs a `struct timeout` that `bsd` embeds, over
//! the kernel's host doubles (`kernel::testutil`). Never part of a kernel image; it measures
//! and changes nothing.
//!
//! `cargo bench -p bsd --bench legacy_timeout` (release: the `bench` profile) or with
//! `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is:
//! - `timeout.set-served`: `timeout_set(to, fn, arg)` of a wrapper whose link names its node
//!   (the path of every call after the first).
//! - `timeout.set-fresh`: `timeout_set` of a zeroed wrapper at an address the registry knows
//!   (a structure made again in a reused pool item: a fork's three timeouts, a tcpcb's); 1,000
//!   wrappers, zeroed between rounds outside the timing.
//! - `timeout.set-new-address`: `timeout_set` of a wrapper at an address never seen, which
//!   makes its node (the pool's get, the registry's insert); 1,000 per repetition, the
//!   registry growing as the kernel's does.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::ffi::c_void;
use std::hint::black_box;
use std::ptr;
use std::time::{Duration, Instant};
use std::vec::Vec;

use bsd::sys::timeout::{Timeout, timeout_set, timeout_startup};

/// A timeout function that does nothing.
fn noop(_arg: *mut c_void) {}

/// Wrappers per round.
const N: usize = 1000;

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

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    kernel::testutil::install();
    timeout_startup();

    let names = [
        "timeout.set-served",
        "timeout.set-fresh",
        "timeout.set-new-address",
    ];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    // The wrappers of the first two benches, at fixed addresses (boxes never moved).
    let mut wrappers: Vec<Box<Timeout>> = (0..N).map(|_| Box::new(Timeout::zeroed())).collect();
    for w in &wrappers {
        timeout_set(w, noop, ptr::null_mut());
    }

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
        const ROUNDS: usize = 200;
        let r = measure(N * ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                for w in &wrappers {
                    timeout_set(w, noop, black_box(ptr::null_mut()));
                }
            }
            t.elapsed()
        });
        report(names[0], r);
    }

    if wanted(names[1]) {
        const ROUNDS: usize = 50;
        let r = measure(N * ROUNDS, || {
            let mut d = Duration::ZERO;
            for _ in 0..ROUNDS {
                // Made again, zeroed, at the same addresses (outside the timing).
                for w in wrappers.iter_mut() {
                    **w = Timeout::zeroed();
                }
                let t = Instant::now();
                for w in &wrappers {
                    timeout_set(w, noop, black_box(ptr::null_mut()));
                }
                d += t.elapsed();
            }
            d
        });
        report(names[1], r);
    }

    if wanted(names[2]) {
        let r = measure(N, || {
            // New addresses (the boxes are leaked: an address is never seen twice).
            let fresh: Vec<&'static Timeout> = (0..N)
                .map(|_| &*Box::leak(Box::new(Timeout::zeroed())))
                .collect();
            let t = Instant::now();
            for w in &fresh {
                timeout_set(w, noop, black_box(ptr::null_mut()));
            }
            t.elapsed()
        });
        report(names[2], r);
    }
}
/* </CODE> */
