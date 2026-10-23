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
//! Host microbenchmarks of N4.7b's hot paths (decision 43): the clists (`kernel::clist`)
//! and, from the tty's commit on, the line discipline's per-character input, its output
//! processing and the tty's lock, over the kernel's host doubles (`kernel::testutil`,
//! feature `doubles`). Never part of a kernel image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench tty` (release: the `bench` profile) or
//! with `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop over LZ's functions, a
//! scratch test of `bsd` at the parent commit):
//! - `prim.clist-putc-getc`: one `putc` then one `getc` on a 1,024-character quoted queue.
//! - `prim.clist-b_to_q-64`: `b_to_q` of 64 bytes then `q_to_b` of them back.
//! - `prim.tty-lock`: a `Mutexed` at `IPL_TTY` taken and given back (a queue's lock, the
//!   tty's); LZ's "before" is a `spltty()`/`splx()` pair, what its queues paid instead.
//! - `prim.spltty`: `splraise(IPL_TTY)` and its restore, the level alone.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::string::String;
use std::time::{Duration, Instant};
use std::vec::Vec;

use kernel::clist::Clist;
use kernel::testutil;

// ---------------------------------------------------------------------------------------
// The harness (vm.rs's).

/// One bench: its name, the ops one call of `f` makes, and `f`, which returns the time of
/// the part it measures.
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

/// A queue of `size` characters, set up by `clalloc`, for the run.
fn queue(size: i32, quot: bool) -> &'static Clist {
    let q: &'static Clist = Box::leak(Box::new(Clist::new()));
    q.clalloc(size, quot);
    q
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();

    let q = queue(1024, true);
    let q2 = queue(1024, true);
    let src = [b'x'; 64];
    let mut dst = [0u8; 64];

    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "prim.clist-putc-getc",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(q.putc(black_box(0x61)));
            black_box(q.getc());
        })),
    });
    benches.push(Bench {
        name: "prim.clist-b_to_q-64",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(q2.b_to_q(black_box(&src)));
            black_box(q2.q_to_b(black_box(&mut dst)));
        })),
    });

    // The primitives the ops are made of: the queue's lock (a `Mutexed` at IPL_TTY), and
    // what LZ's queues paid instead (`spltty()`/`splx()`, a raise of the level alone).
    static TTY_MTX: hal::mutex::Mutexed<u64> = hal::mutex::Mutexed::new(hal::intr::IPL_TTY, 0);
    benches.push(Bench {
        name: "prim.tty-lock",
        ops: PRIM,
        f: Box::new(prim(|| {
            *TTY_MTX.lock() += 1;
        })),
    });
    benches.push(Bench {
        name: "prim.spltty",
        ops: PRIM,
        f: Box::new(prim(|| {
            let s = hal::intr::splraise(hal::intr::IPL_TTY);
            drop(black_box(s));
        })),
    });

    if args.first().map(String::as_str) == Some("list") {
        for b in &benches {
            println!("{}", b.name);
        }
        std::process::exit(0);
    }

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    for b in benches.iter_mut() {
        if !args.is_empty() && !args.iter().any(|a| a == b.name) {
            continue;
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
    }
}
/* </CODE> */
