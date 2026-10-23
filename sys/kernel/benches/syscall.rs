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
//! Host microbenchmarks of the system call path (N4.8b, decision 43: the syscall entry and
//! `copyin`/`copyout` are hot paths), over the host's doubles (`hal`'s user-copy double since
//! N4.8b, `kernel::testutil`'s for the rest). Never part of a kernel image.
//!
//! `cargo bench -p kernel --features doubles --bench syscall` (release) or with `--profile
//! dev`. Arguments: the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop at the parent commit,
//! whose copies went through the `UserCopy` bridge to the kernel's double):
//! - `prim.copyin-64`: 64 bytes copied in from user memory (`UserPtr::copyin_slice`).
//! - `prim.copyout-64`: 64 bytes copied out.
//! - `prim.copyinstr-16`: a 16-byte path (15 characters and the NUL) copied in.
//!
//! The copies themselves are the host's double (a lock over a byte array, the bytes checked
//! against its bounds), not `copy.S`: the numbers measure the path into the machine's copy,
//! which the arm64 kernel shares, not the copy's own cost there.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::user::UserPtr;
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

/// Where the bench's user buffers are in the user-copy double (a range no test uses).
const UBUF: usize = testutil::USER_BASE + 0xD000;

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();

    let path = *b"/usr/lib/libc.\0\0";
    if UserPtr::<u8>::new(UBUF + 0x100)
        .copyout_slice(&path)
        .is_err()
    {
        eprintln!("bench: the bench's buffer is not in the user-copy double");
        std::process::exit(1);
    }
    let src = [0x5au8; 64];
    let mut dst = [0u8; 64];
    let mut name = [0u8; 64];

    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "prim.copyin-64",
        ops: PRIM,
        f: Box::new(prim(|| {
            let r = black_box(UserPtr::<u8>::new(black_box(UBUF))).copyin_slice(&mut dst);
            let _ = black_box(r);
        })),
    });
    benches.push(Bench {
        name: "prim.copyout-64",
        ops: PRIM,
        f: Box::new(prim(|| {
            let r = black_box(UserPtr::<u8>::new(black_box(UBUF))).copyout_slice(&src);
            let _ = black_box(r);
        })),
    });
    benches.push(Bench {
        name: "prim.copyinstr-16",
        ops: PRIM,
        f: Box::new(prim(|| {
            let r = black_box(UserPtr::<u8>::new(black_box(UBUF + 0x100))).copyinstr(&mut name);
            let _ = black_box(r);
        })),
    });

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    for bench in benches.iter_mut() {
        if !args.is_empty() && !args.iter().any(|x| x == bench.name) {
            continue;
        }
        let (min, med, max, calls) = measure(bench, reps, target);
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}% {:>8}",
            bench.name,
            min,
            med,
            max,
            (max - min) / med * 100.0,
            calls
        );
    }
    std::process::exit(0);
}
/* </CODE> */
