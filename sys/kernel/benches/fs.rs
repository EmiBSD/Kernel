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
//! Host microbenchmarks of the vnode layer on the kernel's shape (decision 43, N5.2:
//! `kernel::fs::subr`, `vnode`, `vops`): what `namei`'s, `open`'s and `exec`'s paths cost in
//! `vref`/`vrele`, `vget`/`vput`, `vn_lock`, `vhold`/`vdrop`, one operation's dispatch and
//! a vnode's recycling, over the kernel's doubles and its test file system
//! (`kernel::fs::testfs`, feature `doubles`). Never part of a kernel image; it measures and
//! changes nothing. `sys/benches/fs.rs` times the same op names on LZ's shape.
//!
//! `cargo bench -p kernel --features doubles --bench fs` (release: the `bench` profile) or
//! with `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! The file system's lock is one hal `RecursiveRwlock` the bench's vnodes share (as LZ's bench
//! shares one `Rrwlock`: ufs's discipline, `rrw_enter`/`rrw_exit`/`rrw_status`), its device
//! vnode `spec_vops`'s (nullops).
//!
//! What an op is (`ROUNDS` of `BATCH` ops per timed sample):
//! - `vref.vrele`: `vref` and `vrele` of a vnode in use (the count never reaches 0).
//! - `vget.vput.nolock`: `vget(vp, 0)` and `vput` of a device vnode in use (`spec_vops`:
//!   the lock operations are nullops).
//! - `vget.vput.rrw`: `vget(vp, LK_EXCLUSIVE)` and `vput` of a file system's vnode in use
//!   (its node's rrwlock taken and left).
//! - `vn_lock.unlock.rrw`: `vn_lock(vp, LK_EXCLUSIVE)` and `VOP_UNLOCK`.
//! - `vhold.vdrop`: `vhold` and `vdrop` of a vnode in use.
//! - `vop.dispatch`: one `VOP_ISLOCKED`.
//! - `getnewvnode.recycle`: `getnewvnode` recycling the one free vnode (`vgonel` of its old
//!   identity) and the `vrele` that frees it again.
//! - `vfinddev.hit`: `vfinddev` of a device vnode on its chain.
//! - `prim.vnode-mtx`: `vnode_mtx` taken and left.
//! - `prim.vnode-lists`: the free lists' lock taken and left.

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::rwlock::RecursiveRwlock;
use hal::sys::errno::Errno;
use hal::sys::types::makedev;
use kernel::fs::subr::{cdevvp, getnewvnode, vdrop, vfinddev, vfs, vget, vhold, vput, vref, vrele};
use kernel::fs::testfs::{TestGlue, setup_root};
use kernel::fs::vnode::{
    LK_EXCLUSIVE, LK_RWFLAGS, VCHR, VNODE_MTX, VREG, VT_NON, VT_TMPFS, Vnode, vn_lock,
};
use kernel::fs::vops::{Cred, Vnops, vop_islocked, vop_unlock};
use kernel::proc::Thread;

/// Repetitions per bench.
const REPS: usize = 7;
/// Timed batches per sample.
const ROUNDS: usize = 200;
/// Ops per timed batch.
const BATCH: usize = 100;

/// The bench file system's node lock (every bench vnode's: one is locked at a time).
static NODE_LOCK: RecursiveRwlock = RecursiveRwlock::new("benchnode");

/// The bench file system's vnode operations: ufs's lock discipline, the rest empty (a field,
/// so the table has an address of its own).
struct BenchVops {
    _id: u8,
}

static BENCH_VOPS: BenchVops = BenchVops { _id: 2 };

impl Vnops<TestGlue> for BenchVops {
    fn lock(&self, _vp: &'static Vnode<TestGlue>, flags: i32) -> Result<(), Errno> {
        NODE_LOCK.enter(flags & LK_RWFLAGS)
    }
    fn unlock(&self, _vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        NODE_LOCK.exit();
        Ok(())
    }
    fn islocked(&self, _vp: &'static Vnode<TestGlue>) -> i32 {
        NODE_LOCK.status()
    }
    fn inactive(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        let _ = vop_unlock(vp);
        Ok(())
    }
    fn reclaim(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn close(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn fsync(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _cred: Cred<'_>,
        _waitfor: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
}

/// ns per op of `REPS` samples of `f` (which runs `ROUNDS * BATCH` ops and returns the time
/// they took): min, median, max.
fn measure(mut f: impl FnMut() -> Duration) -> (f64, f64, f64) {
    black_box(f());
    let mut v: Vec<f64> = (0..REPS)
        .map(|_| f().as_nanos() as f64 / (ROUNDS * BATCH) as f64)
        .collect();
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

/// `ROUNDS` batches of `BATCH` calls of `op`, each batch timed alone.
fn rounds(mut op: impl FnMut()) -> Duration {
    let mut total = Duration::ZERO;
    for _ in 0..ROUNDS {
        let t = Instant::now();
        for _ in 0..BATCH {
            op();
        }
        total += t.elapsed();
    }
    total
}

/// Exits with `what` when `x` fails.
fn need<T, E: core::fmt::Debug>(x: Result<T, E>, what: &str) -> T {
    match x {
        Ok(x) => x,
        Err(e) => {
            eprintln!("fs: {what}: {e:?}");
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
        "vref.vrele",
        "vget.vput.nolock",
        "vget.vput.rrw",
        "vn_lock.unlock.rrw",
        "vhold.vdrop",
        "vop.dispatch",
        "getnewvnode.recycle",
        "vfinddev.hit",
        "prim.vnode-mtx",
        "prim.vnode-lists",
    ];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    // The doubles, the kernel lock, the test file system mounted as root.
    kernel::testutil::install();
    let (_memory, mp) = setup_root();
    // A file system's vnode in use (one reference kept), unlocked.
    let vp: &'static Vnode<TestGlue> =
        need(getnewvnode(VT_TMPFS, Some(mp), &BENCH_VOPS), "getnewvnode");
    vp.set_vtype(VREG);
    // A device vnode in use.
    let dev = makedev(5, 1);
    let dvp = match cdevvp::<TestGlue>(dev) {
        Ok(Some(v)) => v,
        _ => {
            eprintln!("fs: cdevvp failed");
            std::process::exit(1);
        }
    };

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
        report(
            names[0],
            measure(|| {
                rounds(|| {
                    vref(black_box(vp));
                    black_box(vrele(vp));
                })
            }),
        );
    }
    if wanted(names[1]) {
        report(
            names[1],
            measure(|| {
                rounds(|| {
                    let _ = vget(black_box(dvp), 0);
                    vput(dvp);
                })
            }),
        );
    }
    if wanted(names[2]) {
        report(
            names[2],
            measure(|| {
                rounds(|| {
                    let _ = vget(black_box(vp), LK_EXCLUSIVE);
                    vput(vp);
                })
            }),
        );
    }
    if wanted(names[3]) {
        report(
            names[3],
            measure(|| {
                rounds(|| {
                    let _ = vn_lock(black_box(vp), LK_EXCLUSIVE);
                    let _ = vop_unlock(vp);
                })
            }),
        );
    }
    if wanted(names[4]) {
        report(
            names[4],
            measure(|| {
                rounds(|| {
                    vhold(black_box(vp));
                    vdrop(vp);
                })
            }),
        );
    }
    if wanted(names[5]) {
        report(
            names[5],
            measure(|| {
                rounds(|| {
                    black_box(vop_islocked(black_box(vp)));
                })
            }),
        );
    }
    if wanted(names[6]) {
        // One free vnode, and no new one allowed: each getnewvnode recycles it.
        let t = vfs::<TestGlue>();
        let first = need(getnewvnode(VT_NON, None, &BENCH_VOPS), "getnewvnode");
        first.set_vtype(VREG);
        let _ = vrele(first);
        let saved = t.maxvnodes.load(core::sync::atomic::Ordering::Relaxed);
        t.maxvnodes.store(1, core::sync::atomic::Ordering::Relaxed);
        report(
            names[6],
            measure(|| {
                rounds(|| {
                    let nvp = need(getnewvnode(VT_NON, None, &BENCH_VOPS), "getnewvnode");
                    nvp.set_vtype(VREG);
                    let _ = vrele(black_box(nvp));
                })
            }),
        );
        t.maxvnodes
            .store(saved, core::sync::atomic::Ordering::Relaxed);
    }
    if wanted(names[7]) {
        report(
            names[7],
            measure(|| {
                rounds(|| {
                    black_box(vfinddev::<TestGlue>(black_box(dev), VCHR));
                })
            }),
        );
    }
    if wanted(names[8]) {
        report(
            names[8],
            measure(|| {
                rounds(|| {
                    let g = VNODE_MTX.lock();
                    black_box(&g);
                })
            }),
        );
    }
    if wanted(names[9]) {
        let t = vfs::<TestGlue>();
        report(
            names[9],
            measure(|| {
                rounds(|| {
                    let g = t.lists.lock();
                    black_box(&g);
                })
            }),
        );
    }
}
/* </CODE> */
