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
//! Host microbenchmarks of the VFS system calls as `bsd` reaches them (decision 43, N5.4):
//! `open(2)` and `close(2)` of a file, `fstat(2)` and `lseek(2)` of an open descriptor and
//! `stat(2)` of a cached path of three components, each called as its `sysent[]` row calls
//! it (`sys_open(p, args, retval)`, the arguments in registers, the paths and the `struct
//! stat` in the user memory of the host's double). Before the switch the rows run LZ's
//! `vfs_syscalls.rs`; at the switch the same names run `kernel::fs::syscalls` through `bsd`'s
//! adapter; over the host's machine double and the kernel's doubles, on real page memory as
//! the host tests load it. Never part of a kernel image; it measures and changes nothing.
//! `sys/kernel/benches/fs.rs` times the same op names over the kernel's test glue, and this
//! file goes with N5.4's last bench commit (N5.3's names bench was this file before, removed
//! at that slice's end: c9c643ac).
//!
//! `cargo bench -p bsd --bench fs` (release: the `bench` profile) or with `--profile dev`.
//! Arguments: `list`; the names of the benches to run (all by default).
//!
//! The file system is a fixed tree, `/a/b/c` (directories) and `/f` (a file), whose lookup is
//! `ufs_lookup`'s protocol over the name cache (`cache_lookup` first, `cache_enter` after the
//! directory), each node with a lock word of its own (`LK_NOWAIT` honoured, a second lock of a
//! held node a panic, as testfs).
//!
//! What an op is (`ROUNDS` of `BATCH` ops per timed sample):
//! - `open.close`: `open("/f", O_RDONLY)` and `close` of the descriptor it returned.
//! - `fstat`: `fstat` of a descriptor of `/f` into a user `struct stat`.
//! - `lseek`: `lseek(fd, 0, SEEK_SET)` of a descriptor of `/f`.
//! - `stat.3`: `stat("/a/b/c")` with the three names cached, into a user `struct stat`.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use std::vec;
use std::vec::Vec;

use bsd::kern::kern_descrip::{filedesc_init, sys_close, sys_fstat};
use bsd::kern::proc_glue::{BsdProcessG, BsdThreadG, procinit};
use bsd::kern::vfs_init::{set_rootvnode, vfsinit};
use bsd::kern::vfs_syscalls::{sys_lseek, sys_open, sys_stat};
use bsd::kern::vfs_vops::VOP_UNLOCK;
use bsd::kern::vm_bsd::{uvm_page_init, uvm_page_physload};
use bsd::machine::Machine;
use bsd::machine::cpu::Cpu;
use bsd::sys::errno::Errno;
use bsd::sys::fcntl::O_RDONLY;
use bsd::sys::lock::{LK_EXCLUSIVE, LK_NOWAIT};
use bsd::sys::lockf::lf_init;
use bsd::sys::mount::{
    MNT_LOCAL, MNT_ROOTFS, Mount, VFS_ROOT, Vfsconf, Vfsops, mountlist_insert_tail,
};
use bsd::sys::namei::cache_enter;
use bsd::sys::namei::cache_lookup;
use bsd::sys::namei::{
    CREATE, FOLLOW, ISDOTDOT, ISLASTCN, LOCKPARENT, LOOKUP, MAKEENTRY, NiDirp, PDIRUNLOCK, RENAME,
};
use bsd::sys::namei::{namei, ndinit};
use bsd::sys::param::PAGE_SIZE;
use bsd::sys::proc::{Proc, Process};
use bsd::sys::systm::{PHYSMEM, SysArgs};
use bsd::sys::types::{Register, Vaddr};
use bsd::sys::unistd::SEEK_SET;
use bsd::sys::vnode::{
    VDIR, VREG, VROOT, VT_TMPFS, Vattr, Vnode, VopAccessArgs, VopGetattrArgs, VopInactiveArgs,
    VopIslockedArgs, VopLockArgs, VopLookupArgs, VopUnlockArgs, Vops,
};
use bsd::sys::vnode::{getnewvnode, vfs_mount_alloc, vfs_unbusy, vget, vn_lock, vref, vrele};
use hal::user::UserPtr;
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

/// `/`.
const ROOT: usize = 0;
/// `/a`.
const A: usize = 1;
/// `/a/b`.
const B: usize = 2;
/// `/f`, a file (`/a/b/c` is node 3).
const F: usize = 4;
/// The number of nodes.
const N: usize = 5;
/// Each node's name and parent.
const NODES: [(&[u8], usize); N] = [
    (b"/", ROOT),
    (b"a", ROOT),
    (b"b", A),
    (b"c", B),
    (b"f", ROOT),
];

/// Each node's vnode, while it has one (the "inode hash").
static VREFS: std::sync::Mutex<[Option<&'static Vnode>; N]> = std::sync::Mutex::new([None; N]);
/// Each node's lock: 1 while held.
static LOCKED: [AtomicU32; N] = [const { AtomicU32::new(0) }; N];
/// The mount.
static MP: std::sync::Mutex<Option<&'static Mount>> = std::sync::Mutex::new(None);

/// The node a bench vnode stands for (`v_data` is its index plus one).
fn node_of(vp: &Vnode) -> usize {
    (vp.data() as usize) - 1
}

/// The node's vnode, if it has one.
fn vnode_of(idx: usize) -> Option<&'static Vnode> {
    VREFS.lock().map_or(None, |t| t[idx])
}

/// The vnode of node `idx`, referenced and locked (`VFS_VGET`).
fn vget_node(mp: &'static Mount, idx: usize) -> Result<&'static Vnode, Errno> {
    if let Some(vp) = vnode_of(idx) {
        vget(vp, LK_EXCLUSIVE)?;
        return Ok(vp);
    }
    let vp = getnewvnode(VT_TMPFS, Some(mp), &BENCH_VOPS)?;
    vp.set_vtype(if idx == F { VREG } else { VDIR });
    vp.set_data(core::ptr::without_provenance_mut(idx + 1));
    if idx == ROOT {
        vp.set_flag(VROOT);
    }
    vn_lock(vp, LK_EXCLUSIVE)?;
    if let Ok(mut t) = VREFS.lock() {
        t[idx] = Some(vp);
    }
    Ok(vp)
}

fn bench_lock(ap: &mut VopLockArgs) -> Result<(), Errno> {
    let l = &LOCKED[node_of(ap.a_vp)];
    if l.load(Ordering::Relaxed) != 0 {
        if ap.a_flags & LK_NOWAIT != 0 {
            return Err(Errno::EBUSY);
        }
        panic!("benchfs: node {} locked twice", node_of(ap.a_vp));
    }
    l.store(1, Ordering::Relaxed);
    Ok(())
}

fn bench_unlock(ap: &mut VopUnlockArgs) -> Result<(), Errno> {
    LOCKED[node_of(ap.a_vp)].store(0, Ordering::Relaxed);
    Ok(())
}

fn bench_islocked(ap: &mut VopIslockedArgs) -> i32 {
    LOCKED[node_of(ap.a_vp)].load(Ordering::Relaxed) as i32
}

fn bench_inactive(ap: &mut VopInactiveArgs<'_>) -> Result<(), Errno> {
    VOP_UNLOCK(ap.a_vp)
}

/// `ufs_lookup`'s protocol over the fixed tree: the name cache first, then the directory;
/// the parent stays locked only for `LOCKPARENT` on the last component.
fn bench_lookup(ap: &mut VopLookupArgs<'_, '_>) -> Result<(), Errno> {
    let dvp = ap.a_dvp;
    let cnp = &mut *ap.a_cnp;
    *ap.a_vpp = None;
    if dvp.vtype() != VDIR {
        return Err(Errno::ENOTDIR);
    }
    let lockparent = cnp.cn_flags & LOCKPARENT != 0;
    let islast = cnp.cn_flags & ISLASTCN != 0;
    match cache_lookup(dvp, cnp) {
        Ok(Some(vp)) => {
            *ap.a_vpp = Some(vp);
            return Ok(());
        }
        Ok(None) => {}
        Err(e) => return Err(e),
    }
    let dir = node_of(dvp);
    let name = cnp.name();
    let found = if name == b"." {
        Some(dir)
    } else if cnp.cn_flags & ISDOTDOT != 0 {
        Some(NODES[dir].1)
    } else {
        (1..N).find(|&i| NODES[i].1 == dir && NODES[i].0 == name)
    };
    let Some(idx) = found else {
        if (cnp.cn_nameiop == CREATE || cnp.cn_nameiop == RENAME) && islast {
            return Err(Errno::EJUSTRETURN);
        }
        return Err(Errno::ENOENT);
    };
    let Some(mp) = MP.lock().ok().and_then(|m| *m) else {
        return Err(Errno::ENOENT);
    };
    let vp = if idx == dir {
        vref(dvp);
        dvp
    } else if cnp.cn_flags & ISDOTDOT != 0 {
        let _ = VOP_UNLOCK(dvp);
        cnp.cn_flags |= PDIRUNLOCK;
        let vp = vget_node(mp, idx)?;
        if lockparent && islast {
            vn_lock(dvp, LK_EXCLUSIVE)?;
            cnp.cn_flags &= !PDIRUNLOCK;
        }
        vp
    } else {
        let vp = vget_node(mp, idx)?;
        if !lockparent || !islast {
            let _ = VOP_UNLOCK(dvp);
            cnp.cn_flags |= PDIRUNLOCK;
        }
        vp
    };
    if cnp.cn_flags & MAKEENTRY != 0 {
        cache_enter(dvp, Some(vp), cnp.name());
    }
    *ap.a_vpp = Some(vp);
    Ok(())
}

fn bench_getattr(ap: &mut VopGetattrArgs<'_>) -> Result<(), Errno> {
    let idx = node_of(ap.a_vp);
    let va = &mut *ap.a_vap;
    *va = Vattr::new();
    va.va_type = ap.a_vp.vtype();
    va.va_mode = 0o755;
    va.va_nlink = 1;
    va.va_fsid = 98;
    va.va_fileid = 100 + idx as u64;
    va.va_size = 512;
    va.va_blocksize = 512;
    Ok(())
}

fn bench_access(_ap: &mut VopAccessArgs<'_>) -> Result<(), Errno> {
    Ok(())
}

/// The bench file system's vnode operations.
static BENCH_VOPS: Vops = Vops {
    vop_lock: Some(bench_lock),
    vop_unlock: Some(bench_unlock),
    vop_islocked: Some(bench_islocked),
    vop_inactive: Some(bench_inactive),
    vop_reclaim: Some(|_| Ok(())),
    vop_lookup: Some(bench_lookup),
    vop_getattr: Some(bench_getattr),
    vop_access: Some(bench_access),
    vop_open: Some(|_| Ok(())),
    vop_close: Some(|_| Ok(())),
    vop_fsync: Some(|_| Ok(())),
    vop_print: Some(|_| Ok(())),
    ..Vops::EMPTY
};

/// The bench file system's operations: the root and the inode lookup do anything.
static BENCH_VFSOPS: Vfsops = Vfsops {
    vfs_mount: |_, _, _, _, _| Ok(()),
    vfs_start: |_, _, _| Ok(()),
    vfs_unmount: |_, _, _| Ok(()),
    vfs_root: |mp| vget_node(mp, ROOT),
    vfs_quotactl: |_, _, _, _, _| Err(Errno::EOPNOTSUPP),
    vfs_statfs: |_, _, _| Ok(()),
    vfs_sync: |_, _, _, _, _| Ok(()),
    vfs_vget: |mp, ino| vget_node(mp, ino as usize),
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
    // The kernel's pools on bsd's pool(9), as the kernel boots (before the doubles'
    // registration, which then changes nothing).
    bsd::kern::hal_bridges::hal_bridges_register_pools();
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

/// A thread of a process of its own with credentials and a descriptor table, `curproc` (what
/// `vfs_subr.rs`'s test `setup` makes, which a bench cannot reach).
fn the_proc() -> &'static Proc {
    procinit();
    filedesc_init();
    let pr: &'static Process = Box::leak(Box::new(Process::new(1, BsdProcessG::new())));
    let p: &'static Proc = Box::leak(Box::new(Proc::new_static(pr, BsdThreadG::new())));
    p.p_self.set(Some(p));
    pr.set_main_static(hal::pin::pin_static(p));
    drop(p.set_cred(kernel::cred::crget()));
    drop(pr.set_ucred(kernel::cred::crget()));
    // The limits of `lim_startup` that matter here: descriptors to open.
    let mut limits = kernel::resource::Plimit::new();
    for l in &mut limits.pl_rlimit {
        l.rlim_cur = kernel::resource::RLIM_INFINITY;
        l.rlim_max = kernel::resource::RLIM_INFINITY;
    }
    limits.pl_rlimit[kernel::resource::RLIMIT_NOFILE].rlim_cur = 512;
    limits.pl_rlimit[kernel::resource::RLIMIT_NOFILE].rlim_max = 512;
    drop(pr.set_limit(kernel::resource::PlimitRef::new(limits)));
    pr.set_fd(kernel::descrip::fdinit());
    Machine::set_curproc(Machine::curcpu(), Some(p));
    p
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

/// `namei(path, op, flags)`, its vnode released (`vrele`).
fn lookup_rele(p: &Proc, path: &'static [u8], flags: u64) {
    let mut nd = ndinit(LOOKUP, flags, NiDirp::Sys(path), p);
    need(namei(&mut nd), "namei");
    if let Some(vp) = nd.ni_vp {
        vrele(vp);
    }
}

/// Where the bench keeps its user data in the user memory of the host's double: the paths,
/// then a `struct stat`.
const UPATH_F: usize = kernel::testutil::USER_BASE + 0x1200;
/// `"/a/b/c"`.
const UPATH_ABC: usize = kernel::testutil::USER_BASE + 0x1240;
/// The `struct stat` of `fstat` and `stat`.
const USTAT: usize = kernel::testutil::USER_BASE + 0x1280;

/// The six argument registers of a system call.
fn args(a: [usize; 3]) -> SysArgs {
    [
        a[0] as Register,
        a[1] as Register,
        a[2] as Register,
        0,
        0,
        0,
    ]
}

/// `open(path, flags)` as its row calls it: the descriptor.
fn open(p: &Proc, path: usize) -> i32 {
    let mut rv = [0 as Register; 2];
    need(
        sys_open(p, &args([path, O_RDONLY as usize, 0]), &mut rv),
        "open",
    );
    rv[0] as i32
}

/// `close(fd)` as its row calls it.
fn close(p: &Proc, fd: i32) {
    let mut rv = [0 as Register; 2];
    need(sys_close(p, &args([fd as usize, 0, 0]), &mut rv), "close");
}

fn main() {
    let args_: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let names = ["open.close", "fstat", "lseek", "stat.3"];
    if args_.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args_.is_empty() || args_.iter().any(|a| a == n);

    setup_memory();
    // The rows' system calls run under the kernel lock where their rows take it, and the
    // file system's lookups under it, as the system calls that reach them take it.
    bsd::sys::systm::kernel_lock();
    let p = the_proc();
    vfsinit();
    lf_init();
    let _ = kernel::fs::spec::register_devsw::<bsd::kern::proc_glue::BsdProcGlue>(
        &bsd::sys::specdev::BSD_DEVSW,
    );
    // The bench file system mounted as root, as `main` does after `mountroot`.
    let mp = vfs_mount_alloc(None, &BENCH_CONF);
    vfs_unbusy(mp);
    if let Ok(mut m) = MP.lock() {
        *m = Some(mp);
    }
    mountlist_insert_tail(mp);
    mp.set_flag(mp.flag() | MNT_ROOTFS);
    let root = need(VFS_ROOT(mp), "VFS_ROOT");
    set_rootvnode(Some(root));
    p.fd().set_cdir(Some(root));
    vref(root);
    let _ = VOP_UNLOCK(root);
    // Every name cached once.
    lookup_rele(p, b"/a/b/c", FOLLOW);
    lookup_rele(p, b"/f", FOLLOW);
    // The paths in user memory.
    need(
        UserPtr::<u8>::new(UPATH_F).copyout_slice(b"/f\0"),
        "copyout",
    );
    need(
        UserPtr::<u8>::new(UPATH_ABC).copyout_slice(b"/a/b/c\0"),
        "copyout",
    );
    // A descriptor of `/f` the fstat and lseek rows use; each row's call succeeds once
    // before it is timed.
    let fd = open(p, UPATH_F);
    {
        let mut rv = [0 as Register; 2];
        need(
            sys_fstat(p, &args([fd as usize, USTAT, 0]), &mut rv),
            "fstat",
        );
        need(
            sys_lseek(p, &args([fd as usize, 0, SEEK_SET as usize]), &mut rv),
            "lseek",
        );
        need(sys_stat(p, &args([UPATH_ABC, USTAT, 0]), &mut rv), "stat");
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
        report(
            names[0],
            measure(|| {
                rounds(|| {
                    let fd = open(p, black_box(UPATH_F));
                    close(p, fd);
                })
            }),
        );
    }
    if wanted(names[1]) {
        report(
            names[1],
            measure(|| {
                rounds(|| {
                    let mut rv = [0 as Register; 2];
                    let a = args([black_box(fd) as usize, USTAT, 0]);
                    let _ = black_box(sys_fstat(p, &a, &mut rv));
                })
            }),
        );
    }
    if wanted(names[2]) {
        report(
            names[2],
            measure(|| {
                rounds(|| {
                    let mut rv = [0 as Register; 2];
                    let a = args([black_box(fd) as usize, 0, SEEK_SET as usize]);
                    let _ = black_box(sys_lseek(p, &a, &mut rv));
                })
            }),
        );
    }
    if wanted(names[3]) {
        report(
            names[3],
            measure(|| {
                rounds(|| {
                    let mut rv = [0 as Register; 2];
                    let a = args([black_box(UPATH_ABC), USTAT, 0]);
                    let _ = black_box(sys_stat(p, &a, &mut rv));
                })
            }),
        );
    }
    close(p, fd);
}
/* </CODE> */
