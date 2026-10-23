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
//! Host microbenchmarks of N4.8's clock hot paths (decision 43): the timecounter readers and
//! the windup (`kernel::tc`), the clock interrupt's dispatch and hardclock as they land, over
//! the kernel's host doubles (`kernel::testutil`, feature `doubles`). Never part of a kernel
//! image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench clock` (release: the `bench` profile)
//! or with `--profile dev`. Arguments: the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop over LZ's functions, a
//! scratch test of `bsd`'s `kern_tc.rs` at the parent commit):
//! - `prim.binuptime`: `binuptime()` on the dummy counter (one atomic increment per read, as
//!   LZ's dummy).
//! - `prim.getnanotime`: `getnanotime()`.
//! - `prim.nanouptime`: `nanouptime()`.
//! - `prim.tc-windup`: `tc_windup()` under `windup_mtx`, no timekeep page.

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::time::{Duration, Instant};
use std::vec::Vec;

use kernel::tc;
use kernel::testutil;

/// The calls of a primitive per timed call.
const PRIM: usize = 1000;

/// ns per op of `reps` repetitions of about `target` of wall time each: min, median, max.
fn measure(mut f: impl FnMut(), reps: usize, target: Duration) -> (f64, f64, f64) {
    let mut calls = 1usize;
    loop {
        let t = Instant::now();
        for _ in 0..calls * PRIM {
            f();
        }
        let wall = t.elapsed();
        if wall >= target / 4 || calls >= 1 << 20 {
            let per = wall.as_secs_f64() / calls as f64;
            calls = ((target.as_secs_f64() / per.max(1e-9)) as usize).max(3);
            break;
        }
        calls *= 4;
    }
    let mut v = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        for _ in 0..calls * PRIM {
            f();
        }
        v.push(t.elapsed().as_nanos() as f64 / (calls * PRIM) as f64);
    }
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();
    tc::inittimecounter();

    let benches: [(&str, &dyn Fn()); 4] = [
        ("prim.binuptime", &|| {
            black_box(tc::binuptime());
        }),
        ("prim.getnanotime", &|| {
            black_box(tc::getnanotime());
        }),
        ("prim.nanouptime", &|| {
            black_box(tc::nanouptime());
        }),
        ("prim.tc-windup", &|| {
            tc::tc_windup(&mut tc::WINDUP.lock(), None, None, None, None);
        }),
    ];

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, reps {reps}");
    for (name, f) in benches {
        if !args.is_empty() && !args.iter().any(|x| x == name) {
            continue;
        }
        let (min, med, max) = measure(f, reps, target);
        println!("{name:<30} {min:>12.1} {med:>12.1} {max:>12.1}");
    }
    std::process::exit(0);
}
/* </CODE> */
