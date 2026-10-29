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
//! Host microbenchmarks of the buffer cache and the disk queues on the kernel's shape
//! (decision 43, N5.1: `kernel::fs::bio`, `kernel::fs::biomem`, `kernel::fs::bufq`): the ops
//! `sys/benches/bio.rs` timed on LZ's shape, by the same names, over the kernel's host
//! doubles (`kernel::testutil`, feature `doubles`; `PMAP_NOMMU`: a buffer's pages are reached
//! through the direct map) on real page memory as the host tests load it. Never part of a
//! kernel image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench bio` (release: the `bench` profile) or
//! with `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! The set-up: 64 MiB of host memory as the physical memory, `bufinit` with `bufpages` 4,096
//! (256 kva slots of `MAXPHYS`), a vnode double (`testutil::BufTestVnode`, whose strategy
//! completes a transfer at once; no timed op reaches it), and `CACHED` one-page blocks read
//! into the cache, released, each idle and mapped. What an op is (the same work as LZ's,
//! the token's part included):
//! - `getblk.hit`: `getblk(vb, blkno, PAGE_SIZE, 0, INFSLP)` of a cached, idle, mapped block:
//!   the tree lookup, `bufcache_take` and the acquire (the token made, the view taken from the
//!   idle buffer). Timed over the `CACHED` blocks in a row; released afterwards, untimed.
//! - `brelse`: `brelse` of those tokens: back to the 2Q queues and `buf_valist`, the view
//!   kept by the idle buffer.
//! - `bread.hit`: `bread` of a cached block holding valid data (`B_DONE`): `getblk.hit` and
//!   the doread's check, no I/O. Released afterwards, untimed.
//! - `getblk.miss-alloc`: `getblk` of a block the cache does not hold: `buf_get`'s
//!   allocation of the header from `bufpl`, of its page (`uvm_pagealloc_multi`) and of its
//!   view (a kva slot: once the arena is used up, the one an invalidated buffer left at the
//!   head of `buf_valist`), and its links. `MISSES` fresh blocks a round, invalidated and
//!   released afterwards, untimed (which frees them).
//! - `bufcache.take-release`: `bufcache_take` and `bufcache_release` of a cached idle buffer
//!   under the cache's lock, taken once a round as LZ's `splbio`: the 2Q queues' half of an
//!   acquire and a release.
//! - `bufq.nscan.queue-dequeue.128`: a disk's trip through an nscan queue: 128 buffers of a
//!   driver's own (their tokens kept across rounds, as LZ's buffers stayed busy) with
//!   scattered block numbers `bufq_queue`d (the token parked), then `bufq_dequeue`d (the
//!   token taken back) and `bufq_done` each (the sort of a segment included); an op is one
//!   buffer's three calls.
//! - `bufq.fifo.queue-dequeue`: the same trip through a fifo queue, one buffer at a time.
//! - `prim.splbio`: `splraise(IPL_BIO)` and its `splx`, the legacy cache's lock, beside
//! - `prim.cache-lock`: the kernel's cache lock (`CACHE`, a `Mutexed` at `IPL_BIO`) taken and
//!   dropped.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::intr::{IPL_BIO, splraise};
use hal::owned::SecondaryRef;
use hal::sleep::INFSLP;
use hal::sys::param::PAGE_SIZE;
use hal::sys::types::Daddr;
use kernel::fs::bio::{CACHE, VnodeBufs, bread, brelse, bufcache_release, bufcache_take, getblk};
use kernel::fs::buf::{B_INVAL, BUFQ_FIFO, BUFQ_NSCAN, Buf, BufRef, ByBufhead};
use kernel::fs::bufq::{Bufq, bufq_dequeue, bufq_done, bufq_init, bufq_queue};
use kernel::testutil::{BufTestVnode, bufcache_install};

/// The host memory loaded as the physical memory.
const BLOCK_BYTES: usize = 64 << 20;

/// The cache's size in pages (`bufpages`), so 256 kva slots of `MAXPHYS`.
const BUFPAGES: i64 = 4096;

/// The cached blocks the hit benches go over.
const CACHED: usize = 128;

/// The fresh blocks a round of `getblk.miss-alloc` takes.
const MISSES: usize = 64;

/// A block's sectors (`PAGE_SIZE / DEV_BSIZE`): consecutive blocks do not overlap.
const STEP: Daddr = 8;

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

/// The block number of the `i`th cached block.
fn cached(i: usize) -> Daddr {
    i as Daddr * STEP
}

/// Exits with `what` when `x` is `None`.
fn need<T>(x: Option<T>, what: &str) -> T {
    match x {
        Some(x) => x,
        None => {
            eprintln!("bio: {what}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let names = [
        "getblk.hit",
        "brelse",
        "bread.hit",
        "getblk.miss-alloc",
        "bufcache.take-release",
        "bufq.nscan.queue-dequeue.128",
        "bufq.fifo.queue-dequeue",
        "prim.splbio",
        "prim.cache-lock",
    ];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    // The page memory, the kernel lock (the cache runs under it, as a system call), bufinit.
    let _memory = bufcache_install(BLOCK_BYTES, BUFPAGES);
    let tv = BufTestVnode::new(CACHED * STEP as usize);
    let vb: &'static VnodeBufs = &tv.bufs;
    let size = PAGE_SIZE as i32;
    for i in 0..CACHED {
        let (t, r) = bread(vb, cached(i), size);
        if r.is_err() {
            eprintln!("bio: the read of block {} failed", cached(i));
            std::process::exit(1);
        }
        brelse(t);
    }

    // `profile SECS`: getblk.hit and brelse in a loop for SECS seconds, for a sampler
    // (`sample <pid>`) to attribute their cost.
    if args.first().map(String::as_str) == Some("profile") {
        let secs: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
        println!("profile: pid {} for {secs} s", std::process::id());
        let end = Instant::now() + Duration::from_secs(secs);
        let mut ts: Vec<BufRef> = Vec::with_capacity(CACHED);
        while Instant::now() < end {
            for _ in 0..100 {
                for i in 0..CACHED {
                    ts.push(need(
                        getblk(vb, black_box(cached(i)), size, 0, INFSLP),
                        "getblk gave no buffer",
                    ));
                }
                for b in ts.drain(..) {
                    brelse(black_box(b));
                }
            }
        }
        return;
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

    if wanted(names[0]) || wanted(names[1]) {
        const ROUNDS: usize = 200;
        let mut gets = Vec::new();
        let mut rels = Vec::new();
        let mut ts: Vec<BufRef> = Vec::with_capacity(CACHED);
        for rep in 0..=REPS {
            let mut get = Duration::ZERO;
            let mut rel = Duration::ZERO;
            for _ in 0..ROUNDS {
                let t = Instant::now();
                for i in 0..CACHED {
                    ts.push(need(
                        getblk(vb, black_box(cached(i)), size, 0, INFSLP),
                        "getblk gave no buffer",
                    ));
                }
                get += t.elapsed();
                let t = Instant::now();
                for b in ts.drain(..) {
                    brelse(black_box(b));
                }
                rel += t.elapsed();
            }
            if rep > 0 {
                gets.push(get.as_nanos() as f64 / (CACHED * ROUNDS) as f64);
                rels.push(rel.as_nanos() as f64 / (CACHED * ROUNDS) as f64);
            }
        }
        let spread = |mut v: Vec<f64>| {
            v.sort_by(f64::total_cmp);
            (v[0], v[v.len() / 2], v[v.len() - 1])
        };
        if wanted(names[0]) {
            report(names[0], spread(gets));
        }
        if wanted(names[1]) {
            report(names[1], spread(rels));
        }
    }

    if wanted(names[2]) {
        const ROUNDS: usize = 200;
        let mut ts: Vec<BufRef> = Vec::with_capacity(CACHED);
        let r = measure(CACHED * ROUNDS, || {
            let mut d = Duration::ZERO;
            for _ in 0..ROUNDS {
                let t = Instant::now();
                for i in 0..CACHED {
                    let (b, r) = bread(vb, black_box(cached(i)), size);
                    black_box(r).ok();
                    ts.push(b);
                }
                d += t.elapsed();
                for b in ts.drain(..) {
                    brelse(b);
                }
            }
            d
        });
        report(names[2], r);
    }

    if wanted(names[3]) {
        const ROUNDS: usize = 50;
        // Fresh blocks past the cached ones, never asked for twice.
        let mut next = cached(CACHED) + 1024 * STEP;
        let mut ts: Vec<BufRef> = Vec::with_capacity(MISSES);
        let r = measure(MISSES * ROUNDS, || {
            let mut d = Duration::ZERO;
            for _ in 0..ROUNDS {
                let t = Instant::now();
                for _ in 0..MISSES {
                    ts.push(need(
                        getblk(vb, black_box(next), size, 0, INFSLP),
                        "getblk gave no buffer",
                    ));
                    next += STEP;
                }
                d += t.elapsed();
                for b in ts.drain(..) {
                    b.buf().set(B_INVAL);
                    brelse(b);
                }
            }
            d
        });
        report(names[3], r);
    }

    if wanted(names[4]) {
        const ROUNDS: usize = 500;
        // Counted links to the cached buffers (each taken through a token, released at once).
        let refs: Vec<SecondaryRef<ByBufhead>> = (0..CACHED)
            .map(|i| {
                let t = need(
                    getblk(vb, cached(i), size, 0, INFSLP),
                    "a block is not cached",
                );
                let r = need(t.counted(), "a cached buffer without its link");
                brelse(t);
                r
            })
            .collect();
        let r = measure(CACHED * ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                let mut g = CACHE.lock();
                for r in &refs {
                    let bp = need(r.get(), "a link points at nothing");
                    bufcache_take(&mut g, black_box(bp));
                    bufcache_release(&mut g, bp);
                }
                drop(g);
            }
            t.elapsed()
        });
        report(names[4], r);
    }

    if wanted(names[5]) {
        const ROUNDS: usize = 500;
        const QN: usize = 128;
        let bq: &'static Bufq = Box::leak(Box::new(Bufq::new()));
        if bufq_init(bq, BUFQ_NSCAN).is_err() {
            eprintln!("bio: bufq_init failed");
            std::process::exit(1);
        }
        // Scattered block numbers: a multiplicative walk over the segment.
        let mut ts: Vec<BufRef> = (0..QN)
            .map(|i| {
                let bp: &'static Buf = Box::leak(Box::new(Buf::new()));
                bp.set_blkno(((i * 97) % QN) as Daddr * STEP);
                BufRef::foreign(bp)
            })
            .collect();
        let r = measure(QN * ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                for b in ts.drain(..) {
                    bufq_queue(bq, black_box(b));
                }
                while let Some(b) = bufq_dequeue(bq) {
                    bufq_done(bq, black_box(b.buf()));
                    ts.push(b);
                }
            }
            t.elapsed()
        });
        report(names[5], r);
    }

    if wanted(names[6]) {
        const ROUNDS: usize = 100_000;
        let bq: &'static Bufq = Box::leak(Box::new(Bufq::new()));
        if bufq_init(bq, BUFQ_FIFO).is_err() {
            eprintln!("bio: bufq_init failed");
            std::process::exit(1);
        }
        let bp: &'static Buf = Box::leak(Box::new(Buf::new()));
        let mut held = Some(BufRef::foreign(bp));
        let r = measure(ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                bufq_queue(bq, black_box(need(held.take(), "the token is gone")));
                if let Some(b) = bufq_dequeue(bq) {
                    bufq_done(bq, black_box(b.buf()));
                    held = Some(b);
                }
            }
            t.elapsed()
        });
        report(names[6], r);
    }

    if wanted(names[7]) {
        const ROUNDS: usize = 1_000_000;
        let r = measure(ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                let s = splraise(IPL_BIO);
                drop(black_box(s));
            }
            t.elapsed()
        });
        report(names[7], r);
    }

    if wanted(names[8]) {
        const ROUNDS: usize = 1_000_000;
        let r = measure(ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                let g = CACHE.lock();
                drop(black_box(g));
            }
            t.elapsed()
        });
        report(names[8], r);
    }
}
/* </CODE> */
