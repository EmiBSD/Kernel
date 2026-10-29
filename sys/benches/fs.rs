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
//! Host microbenchmarks of the vnode layer on LZ's shape (decision 43, N5.2:
//! `sys/kern/vfs_subr.rs`, `vfs_vnops.rs`, `vfs_vops.rs`): what `namei`'s, `open`'s and
//! `exec`'s paths cost in `vref`/`vrele`, `vget`/`vput`, `vn_lock`, `vhold`/`vdrop`, one
//! operation's dispatch and a vnode's recycling before the vnode layer moves into
//! `kernel::fs`, over the host's machine double and the kernel's doubles, on real page memory
//! as the host tests load it. Never part of a kernel image; it measures and changes nothing.
//! `sys/kernel/benches/fs.rs` times the same op names on the kernel's shape, and this file
//! goes with the switch (N5.2's last bench commit).
//!
//! `cargo bench -p bsd --bench fs` (release: the `bench` profile) or with `--profile dev`.
//! Arguments: `list`; the names of the benches to run (all by default).
//!
//! The ops are the kernel bench's (see its notes). The file system's lock is one `Rrwlock`
//! the bench's vnodes share (`rrw_enter`/`rrw_exit`/`rrw_status`, ufs's), its device vnode
//! `spec_vops`'s (nullops); `prim.vnode-lists` is `splbio`/`splx`, the free lists' lock of
//! LZ's shape.

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use std::vec;
use std::vec::Vec;

use bsd::kern::kern_lock::{mtx_enter, mtx_leave};
use bsd::kern::kern_rwlock::{rrw_enter, rrw_exit, rrw_status};
use bsd::kern::vfs_subr::{
    MAXVNODES, VNODE_MTX, cdevvp, getnewvnode, vdrop, vfinddev, vfs_mount_alloc, vfs_unbusy, vget,
    vhold, vput, vref, vrele,
};
use bsd::kern::vfs_vnops::vn_lock;
use bsd::kern::vfs_vops::{VOP_ISLOCKED, VOP_UNLOCK};
use bsd::kern::vm_bsd::{uvm_page_init, uvm_page_physload};
use bsd::machine::intr::{splbio, splx};
use bsd::sys::errno::Errno;
use bsd::sys::lock::{LK_EXCLUSIVE, LK_RWFLAGS};
use bsd::sys::mount::{MNT_LOCAL, Vfsconf, Vfsops};
use bsd::sys::param::PAGE_SIZE;
use bsd::sys::rwlock::Rrwlock;
use bsd::sys::systm::PHYSMEM;
use bsd::sys::types::{Vaddr, makedev};
use bsd::sys::vnode::{
    VCHR, VREG, VT_NON, VT_TMPFS, Vnode, VopInactiveArgs, VopIslockedArgs, VopLockArgs,
    VopUnlockArgs, Vops,
};
use hal::vm::param::atop;
use hal::vm::uvmexp::UVMEXP;

/// The host memory loaded as the physical memory.
const BLOCK_BYTES: usize = 64 << 20;
/// Repetitions per bench.
const REPS: usize = 7;
/// Timed batches per sample.
const ROUNDS: usize = 200;
/// Ops per timed batch.
const BATCH: usize = 100;

/// The bench file system's node lock (every bench vnode's: one is locked at a time).
static NODE_LOCK: Rrwlock = Rrwlock::new("benchnode");

fn bench_lock(ap: &mut VopLockArgs) -> Result<(), Errno> {
    rrw_enter(&NODE_LOCK, ap.a_flags & LK_RWFLAGS)
}

fn bench_unlock(_ap: &mut VopUnlockArgs) -> Result<(), Errno> {
    rrw_exit(&NODE_LOCK);
    Ok(())
}

fn bench_islocked(_ap: &mut VopIslockedArgs) -> i32 {
    rrw_status(&NODE_LOCK)
}

fn bench_inactive(ap: &mut VopInactiveArgs<'_>) -> Result<(), Errno> {
    let _ = VOP_UNLOCK(ap.a_vp);
    Ok(())
}

/// The bench file system's vnode operations: ufs's lock discipline, the rest empty.
static BENCH_VOPS: Vops = Vops {
    vop_lock: Some(bench_lock),
    vop_unlock: Some(bench_unlock),
    vop_islocked: Some(bench_islocked),
    vop_inactive: Some(bench_inactive),
    vop_reclaim: Some(|_| Ok(())),
    vop_close: Some(|_| Ok(())),
    vop_fsync: Some(|_| Ok(())),
    ..Vops::EMPTY
};

/// The bench file system's operations: nothing is mounted through them.
static BENCH_VFSOPS: Vfsops = Vfsops {
    vfs_mount: |_, _, _, _, _| Ok(()),
    vfs_start: |_, _, _| Ok(()),
    vfs_unmount: |_, _, _| Ok(()),
    vfs_root: |_| Err(Errno::EOPNOTSUPP),
    vfs_quotactl: |_, _, _, _, _| Err(Errno::EOPNOTSUPP),
    vfs_statfs: |_, _, _| Ok(()),
    vfs_sync: |_, _, _, _, _| Ok(()),
    vfs_vget: |_, _| Err(Errno::EOPNOTSUPP),
    vfs_fhtovp: |_, _| Err(Errno::EOPNOTSUPP),
    vfs_vptofh: |_, _| Err(Errno::EOPNOTSUPP),
    vfs_init: None,
    vfs_sysctl: None,
    vfs_checkexp: |_, _, _, _| Err(Errno::EOPNOTSUPP),
};

/// The bench file system's configuration entry.
static BENCH_CONF: Vfsconf = Vfsconf::new(&BENCH_VFSOPS, b"benchfs", 98, MNT_LOCAL, 0);

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

    setup_memory();
    // The VOPs run under the kernel lock, as the system calls that reach them take it.
    bsd::sys::systm::kernel_lock();
    bsd::kern::vfs_init::vfsinit();
    let mp = vfs_mount_alloc(None, &BENCH_CONF);
    vfs_unbusy(mp);
    // A file system's vnode in use (one reference kept), unlocked.
    let vp: &'static Vnode = need(getnewvnode(VT_TMPFS, Some(mp), &BENCH_VOPS), "getnewvnode");
    vp.v_type.set(VREG);
    // A device vnode in use.
    let dev = makedev(5, 1);
    let dvp = match cdevvp(dev) {
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
                    let _ = VOP_UNLOCK(vp);
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
                    black_box(VOP_ISLOCKED(black_box(vp)));
                })
            }),
        );
    }
    if wanted(names[6]) {
        // One free vnode, and no new one allowed: each getnewvnode recycles it.
        let first = need(getnewvnode(VT_NON, None, &BENCH_VOPS), "getnewvnode");
        first.v_type.set(VREG);
        let _ = vrele(first);
        let saved = MAXVNODES.load(Ordering::Relaxed);
        MAXVNODES.store(1, Ordering::Relaxed);
        report(
            names[6],
            measure(|| {
                rounds(|| {
                    let nvp = need(getnewvnode(VT_NON, None, &BENCH_VOPS), "getnewvnode");
                    nvp.v_type.set(VREG);
                    let _ = vrele(black_box(nvp));
                })
            }),
        );
        MAXVNODES.store(saved, Ordering::Relaxed);
    }
    if wanted(names[7]) {
        report(
            names[7],
            measure(|| {
                rounds(|| {
                    black_box(vfinddev(black_box(dev), VCHR));
                })
            }),
        );
    }
    if wanted(names[8]) {
        report(
            names[8],
            measure(|| {
                rounds(|| {
                    mtx_enter(&VNODE_MTX);
                    mtx_leave(&VNODE_MTX);
                })
            }),
        );
    }
    if wanted(names[9]) {
        report(
            names[9],
            measure(|| {
                rounds(|| {
                    let s = splbio();
                    splx(black_box(s));
                })
            }),
        );
    }
}
/* </CODE> */
