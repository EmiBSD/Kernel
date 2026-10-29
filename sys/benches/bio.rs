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
//! Host microbenchmarks of the buffer cache and the disk queues on LZ's shape (decision 43,
//! N5.1: `sys/kern/vfs_bio.rs`, `sys/kern/vfs_biomem.rs`, `sys/kern/kern_bufq.rs`): what
//! `fileread`'s path costs in `getblk`, `bread` and `brelse` before the cache moves into
//! `kernel::fs`, over the host's machine double (`sys/arch/host`, `PMAP_NOMMU`: a buffer's
//! pages are reached through the direct map) and the kernel's doubles, on real page memory as
//! the host tests load it. Never part of a kernel image; it measures and changes nothing.
//! `sys/kernel/benches/bio.rs` (N5.1's commit 7) times the same op names on the kernel's
//! shape, and this file goes with it.
//!
//! `cargo bench -p bsd --bench bio` (release: the `bench` profile) or with `--profile dev`.
//! Arguments: `list`; the names of the benches to run (all by default).
//!
//! The set-up: 64 MiB of host memory as the physical memory, `bufinit` with `bufpages` 4,096
//! (256 kva slots of `MAXPHYS`), a vnode of its own whose strategy completes a read at once
//! (`biodone`, no data copied: no timed op reaches it), and `CACHED` one-page blocks read
//! into the cache, released, each idle and mapped. What an op is:
//! - `getblk.hit`: `getblk(vp, blkno, PAGE_SIZE, 0, INFSLP)` of a cached, idle, mapped block:
//!   the tree lookup, `bufcache_take` and `buf_acquire` (its mapping kept). Timed over the
//!   `CACHED` blocks in a row; they are released afterwards, untimed.
//! - `brelse`: `brelse` of those busy blocks: back to the 2Q queues and `buf_valist`.
//! - `bread.hit`: `bread` of a cached block holding valid data (`B_DONE`): `getblk.hit` and
//!   `biowait`'s check, no I/O. Released afterwards, untimed.
//! - `getblk.miss-alloc`: `getblk` of a block the cache does not hold: `buf_get`'s
//!   allocation of the header from `bufpl`, of its page (`uvm_pagealloc_multi`) and of its
//!   mapping (a kva slot: once the arena is used up, the one an invalidated buffer left at the
//!   head of `buf_valist`), and its links. `MISSES` fresh blocks a round, invalidated and
//!   released afterwards, untimed (which frees them).
//! - `bufcache.take-release`: `bufcache_take` and `bufcache_release` of a cached idle buffer
//!   at `splbio`: the 2Q queues' half of an acquire and a release.
//! - `bufq.nscan.queue-dequeue.128`: a disk's trip through an nscan queue: 128 buffers with
//!   scattered block numbers `bufq_queue`d, then `bufq_dequeue`d and `bufq_done` each (the
//!   sort of a segment included); an op is one buffer's three calls.
//! - `bufq.fifo.queue-dequeue`: the same trip through a fifo queue, one buffer at a time.
//! - `prim.splbio`: `splbio` and `splx`, the legacy cache's lock (the kernel's shape times
//!   its own lock beside it).

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use std::vec;
use std::vec::Vec;

use bsd::kern::kern_bufq::{bufq_dequeue, bufq_done, bufq_init, bufq_queue};
use bsd::kern::vfs_bio::{
    biodone, bread, brelse, bufcache_release, bufcache_take, bufinit, getblk,
};
use bsd::kern::vm_bsd::{uvm_page_init, uvm_page_physload};
use bsd::machine::intr::{splbio, splx};
use bsd::sys::buf::{B_INVAL, BUFQ_FIFO, BUFQ_NSCAN, Buf, Bufq};
use bsd::sys::errno::Errno;
use bsd::sys::param::PAGE_SIZE;
use bsd::sys::systm::{INFSLP, PHYSMEM};
use bsd::sys::types::{Daddr, Vaddr};
use bsd::sys::vnode::{VREG, Vnode, VopStrategyArgs, Vops};
use hal::vm::param::atop;
use hal::vm::uvmexp::UVMEXP;

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

/// The strategy of the bench's vnode: a read or write completes at once (`b_resid` 0,
/// `biodone`); no byte moves (the timed ops never reach it).
fn strategy(ap: &mut VopStrategyArgs) -> Result<(), Errno> {
    let bp = ap.a_bp;
    bp.b_resid.set(0);
    let s = splbio();
    biodone(bp);
    splx(s);
    Ok(())
}

/// The bench vnode's operations: the strategy alone.
static BENCH_VOPS: Vops = Vops {
    vop_strategy: Some(strategy),
    ..Vops::EMPTY
};

/// What `subr_pool.rs`'s `setup_real_memory` does for the host tests (a bench target is not
/// built with `cfg(test)`): host memory as the physical memory, the page system up, the
/// host pmap's bootstrap and the kernel's doubles of `hal`'s bridges, `kmeminit`.
fn setup_memory() {
    bsd::kern::hal_bridges::hal_bridges_register_vm();
    kernel::testutil::install();
    kernel::vm::page::uvm_page_test_reset();
    UVMEXP.pagesize.store(PAGE_SIZE as i32, Ordering::Relaxed);
    kernel::vm::page::uvm_setpagesize();

    let block: &'static mut [u8] = vec![0u8; BLOCK_BYTES + PAGE_SIZE].leak();
    let base = (block.as_ptr() as usize + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let (start, end) = (atop(base), atop(base + BLOCK_BYTES));
    uvm_page_physload(start, end, start, end, 0);
    PHYSMEM.store(end - start, Ordering::Relaxed);
    let (mut s, mut e) = (Vaddr::new(0), Vaddr::new(0));
    uvm_page_init(&mut s, &mut e);
    hal::pmap::init();
    bsd::kern::kern_malloc::kmeminit();
}

/// The bench's vnode: a regular file in use (so `vhold`/`vdrop` leave the free lists alone),
/// leaked.
fn vnode() -> &'static Vnode {
    let vp: &'static Vnode = Box::leak(Box::new(Vnode::new()));
    vp.v_op.set(Some(&BENCH_VOPS));
    vp.v_type.set(VREG);
    vp.v_usecount.set(1);
    vp
}

/// The block number of the `i`th cached block.
fn cached(i: usize) -> Daddr {
    i as Daddr * STEP
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
    ];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    setup_memory();
    // The buffer cache runs under the kernel lock (`uvm_pagealloc_multi` asserts it).
    bsd::sys::systm::kernel_lock();
    bsd::conf::param::bufpages.store(BUFPAGES, Ordering::Relaxed);
    bufinit();
    let vp = vnode();
    let size = PAGE_SIZE as i32;
    for i in 0..CACHED {
        let (bp, r) = bread(vp, cached(i), size);
        if r.is_err() {
            eprintln!("bio: the read of block {} failed", cached(i));
            std::process::exit(1);
        }
        brelse(bp);
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
    let held = |bp: Option<&'static Buf>| match bp {
        Some(bp) => bp,
        None => {
            eprintln!("bio: getblk gave no buffer");
            std::process::exit(1);
        }
    };

    if wanted(names[0]) || wanted(names[1]) {
        const ROUNDS: usize = 200;
        let mut gets = Vec::new();
        let mut rels = Vec::new();
        let mut bps: Vec<&'static Buf> = Vec::with_capacity(CACHED);
        for rep in 0..=REPS {
            let mut get = Duration::ZERO;
            let mut rel = Duration::ZERO;
            for _ in 0..ROUNDS {
                let t = Instant::now();
                for i in 0..CACHED {
                    bps.push(held(getblk(vp, black_box(cached(i)), size, 0, INFSLP)));
                }
                get += t.elapsed();
                let t = Instant::now();
                for bp in bps.drain(..) {
                    brelse(black_box(bp));
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
        let mut bps: Vec<&'static Buf> = Vec::with_capacity(CACHED);
        let r = measure(CACHED * ROUNDS, || {
            let mut d = Duration::ZERO;
            for _ in 0..ROUNDS {
                let t = Instant::now();
                for i in 0..CACHED {
                    let (bp, r) = bread(vp, black_box(cached(i)), size);
                    black_box(r).ok();
                    bps.push(bp);
                }
                d += t.elapsed();
                for bp in bps.drain(..) {
                    brelse(bp);
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
        let mut bps: Vec<&'static Buf> = Vec::with_capacity(MISSES);
        let r = measure(MISSES * ROUNDS, || {
            let mut d = Duration::ZERO;
            for _ in 0..ROUNDS {
                let t = Instant::now();
                for _ in 0..MISSES {
                    bps.push(held(getblk(vp, black_box(next), size, 0, INFSLP)));
                    next += STEP;
                }
                d += t.elapsed();
                for bp in bps.drain(..) {
                    bp.set(B_INVAL);
                    brelse(bp);
                }
            }
            d
        });
        report(names[3], r);
    }

    if wanted(names[4]) {
        const ROUNDS: usize = 500;
        let bps: Vec<&'static Buf> = (0..CACHED)
            .map(|i| match bsd::kern::vfs_bio::incore(vp, cached(i)) {
                Some(bp) => bp,
                None => {
                    eprintln!("bio: block {} is not cached", cached(i));
                    std::process::exit(1);
                }
            })
            .collect();
        let r = measure(CACHED * ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                let s = splbio();
                for &bp in &bps {
                    bufcache_take(black_box(bp));
                    bufcache_release(bp);
                }
                splx(s);
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
        let bufs: Vec<&'static Buf> = (0..QN)
            .map(|i| {
                let bp: &'static Buf = Box::leak(Box::new(Buf::new()));
                bp.b_blkno.set(((i * 97) % QN) as Daddr * STEP);
                bp
            })
            .collect();
        let r = measure(QN * ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                for &bp in &bufs {
                    bufq_queue(bq, black_box(bp));
                }
                while let Some(bp) = bufq_dequeue(bq) {
                    bufq_done(bq, black_box(bp));
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
        let r = measure(ROUNDS, || {
            let t = Instant::now();
            for _ in 0..ROUNDS {
                bufq_queue(bq, black_box(bp));
                if let Some(bp) = bufq_dequeue(bq) {
                    bufq_done(bq, black_box(bp));
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
                let s = splbio();
                splx(black_box(s));
            }
            t.elapsed()
        });
        report(names[7], r);
    }
}
/* </CODE> */
