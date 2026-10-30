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
//! changes nothing. Until N5.2's last bench commit `sys/benches/fs.rs` timed the same op names
//! on LZ's shape (and, at the switch, on `kernel::fs` with `bsd`'s glue); its numbers are in
//! the commits b4c27cee and the one that removed it.
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
//!
//! N5.3's rows (`kernel::fs::{lookup, cache, lockf, getcwd, vnops}`), the op names and the
//! file system of `sys/benches/fs.rs` (`/a/b/c` and `/f`, `ufs_lookup`'s protocol over the
//! name cache, a lock word per node), mounted as root after the rows above, over the test
//! glue and a thread of its own:
//! - `namei.hit.1`, `namei.hit.3`: `namei("/a")`, `namei("/a/b/c")` with the names cached,
//!   and the `vrele`.
//! - `namei.miss.1`: `namei("/a", NOCACHE | FOLLOW)`.
//! - `cache.lookup.hit`: `cache_lookup` of `a` in the locked root (`LOCKPARENT | ISLASTCN`)
//!   and the `vput`.
//! - `cache.enter.zap`: `cache_enter` of a name not cached in a full cache.
//! - `lf.setlk.unlck`: `lf_advlock` `F_SETLK` and `F_UNLCK` of one owner's write lock.
//! - `getcwd.cached`: `vfs_getcwd_common` of `/a/b/c` through the reverse cache.
//! - `vn_open.close`: `vn_open("/f", FREAD)`, the unlock, `vn_close`.
//! - `prim.namecache-mtx`: the name cache's lock (D-N5.3-2's leaf) taken and left.
//!
//! N5.4's rows (`kernel::fs::syscalls`), the op names of `sys/benches/fs.rs` (N5.4's: the
//! same system calls on LZ's shape and, at the switch, through `bsd`'s adapter), each called
//! as its `KernelApi` entry calls it, the paths and the `struct stat` in the user memory of
//! the host's double, over the names bench's file system and thread:
//! - `open.close`: `open("/f", O_RDONLY)` and `close` of the descriptor.
//! - `fstat`: `fstat` of a descriptor of `/f` (`kernel::descrip`'s, the vnode file's stat).
//! - `lseek`: `lseek(fd, 0, SEEK_SET)`.
//! - `stat.3`: `stat("/a/b/c")` with the three names cached.
//!
//! Since N5.3 the bench registers `proc.rs`'s recycling pool backend before the doubles (as
//! `vm.rs` and `proc.rs` do): the doubles' pool hands out a fresh zeroed page per item and
//! never takes one back, which timed the host's `malloc` and `memset` (a `namei`'s path
//! buffer, a name cache entry, a lock), not the kernel; LZ's bench (`sys/benches/fs.rs`)
//! gets its items from `pool(9)`'s free lists. `getnewvnode.recycle`'s figure moves with it.

#![forbid(unsafe_code)]

use core::ptr::NonNull;
use std::boxed::Box;
use std::hint::black_box;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::sys::fcntl::{F_POSIX, F_SETLK, F_UNLCK, F_WRLCK, FREAD, Flock, O_RDONLY};
use hal::sys::unistd::SEEK_SET;
use hal::user::UserPtr;
use kernel::file::LockOwner;
use kernel::fs::cache::{NCLOCK, cache_counts, cache_enter, cache_lookup, nchinit};
use kernel::fs::getcwd::vfs_getcwd_common;
use kernel::fs::lockf::{LockfSlot, lf_advlock};
use kernel::fs::lookup::{namei, ndinit};
use kernel::fs::mount::{Mount, Vfsconf, mountlist_insert_tail, vfs_mount_alloc, vfs_unbusy};
use kernel::fs::namei::{
    CREATE, Componentname, FOLLOW, ISDOTDOT, ISLASTCN, LOCKPARENT, LOOKUP, MAKEENTRY, MAXPATHLEN,
    NOCACHE, NiDirp, PDIRUNLOCK, RENAME,
};
use kernel::fs::syscalls::{sys_lseek, sys_open, sys_stat};
use kernel::fs::vnode::{GETCWD_CHECK_ACCESS, LK_NOWAIT, VDIR, VROOT, Vattr};
use kernel::fs::vnops::{vn_close, vn_open};
use kernel::fs::vops::{Nd, VFS_ROOT, Vfs};

use hal::pool::{PoolBackend, RawPool};
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

// ---------------------------------------------------------------------------------------
// A pool backend that recycles its items (proc.rs's, itself vm.rs's), registered before the
// doubles: the doubles' pool hands out a fresh zeroed page per item and never takes one back.

/// A page of the host.
const PAGE: usize = 4096;

/// A free list of items of one size, as addresses (the provenance exposed when they were
/// carved or given back).
struct FreeList {
    stride: usize,
    free: Mutex<Vec<usize>>,
}

impl FreeList {
    const fn new(stride: usize) -> Self {
        Self {
            stride,
            free: Mutex::new(Vec::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<usize>> {
        self.free
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// An item: one given back, or a fresh slab carved into items.
    fn get(&self) -> Option<NonNull<u8>> {
        let mut free = self.lock();
        if free.is_empty() {
            let per = (64 * 1024 / self.stride).max(1);
            let mut rest: &'static mut [u8] = fresh(per * self.stride);
            while rest.len() >= self.stride {
                let (item, tail) = rest.split_at_mut(self.stride);
                free.push(
                    NonNull::from(item)
                        .cast::<u8>()
                        .as_ptr()
                        .expose_provenance(),
                );
                rest = tail;
            }
        }
        let addr = free.pop()?;
        NonNull::new(std::ptr::with_exposed_provenance_mut::<u8>(addr))
    }

    fn put(&self, item: NonNull<u8>) {
        self.lock().push(item.as_ptr().expose_provenance());
    }
}

/// `len` bytes of fresh, zeroed, page-aligned memory, leaked.
fn fresh(len: usize) -> &'static mut [u8] {
    // A byte block one page longer, cut at its first page boundary.
    let block: &'static mut [u8] = vec![0u8; len + PAGE].leak();
    let skip = block.as_ptr().addr().next_multiple_of(PAGE) - block.as_ptr().addr();
    let (_, tail) = block.split_at_mut(skip);
    let (run, _) = tail.split_at_mut(len);
    run
}

/// One pool of the recycling double.
struct BenchPool {
    items: FreeList,
    size: usize,
    nout: AtomicU64,
}

impl RawPool for BenchPool {
    fn get(&self, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        let p = self.items.get()?;
        self.nout.fetch_add(1, Ordering::Relaxed);
        Some((p, self.size))
    }

    fn put(&self, item: NonNull<u8>) {
        self.nout.fetch_sub(1, Ordering::Relaxed);
        self.items.put(item);
    }

    fn nout(&self) -> u64 {
        self.nout.load(Ordering::Relaxed)
    }

    fn size(&self) -> usize {
        self.size
    }

    fn set_hiwat(&self, _n: u32) {}

    fn cache_init(&self) {}
}

/// The recycling pool backend.
struct BenchPools;

impl PoolBackend for BenchPools {
    fn create(
        &self,
        size: usize,
        align: usize,
        _ipl: i32,
        _flags: i32,
        _wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        if align > PAGE {
            return None;
        }
        let size = size.max(1);
        let stride = size.next_multiple_of(align.max(16));
        Some(Box::leak(Box::new(BenchPool {
            items: FreeList::new(stride),
            size,
            nout: AtomicU64::new(0),
        })))
    }
}

static BENCH_POOLS: BenchPools = BenchPools;

// ---------------------------------------------------------------------------------------
// The file systems of the bench.

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

/// `/`, `/a`, `/a/b`, `/a/b/c` (directories) and `/f` (a file) of the names bench's file
/// system.
const NROOT: usize = 0;
/// `/a/b/c`.
const NC: usize = 3;
/// `/f`.
const NF: usize = 4;
/// The names bench's nodes.
const NN: usize = 5;
/// Each node's name and parent.
const NNODES: [(&[u8], usize); NN] = [(b"/", 0), (b"a", 0), (b"b", 1), (b"c", 2), (b"f", 0)];

/// Each node's vnode, once made (the "inode hash").
static NVREFS: Mutex<[Option<&'static Vnode<TestGlue>>; NN]> = Mutex::new([None; NN]);
/// Each node's lock: 1 while held.
static NLOCKED: [AtomicU32; NN] = [const { AtomicU32::new(0) }; NN];
/// The names bench's mount.
static NMP: Mutex<Option<&'static Mount<TestGlue>>> = Mutex::new(None);

/// The node a names bench vnode stands for (`v_data` is its index plus one).
fn nnode(vp: &Vnode<TestGlue>) -> usize {
    (vp.data().addr()).wrapping_sub(1)
}

/// The vnode of node `idx`, referenced and locked (`VFS_VGET`).
fn nvget(mp: &'static Mount<TestGlue>, idx: usize) -> Result<&'static Vnode<TestGlue>, Errno> {
    if let Some(vp) = NVREFS.lock().ok().and_then(|t| t[idx]) {
        vget(vp, LK_EXCLUSIVE)?;
        return Ok(vp);
    }
    let vp = getnewvnode(VT_TMPFS, Some(mp), &NAMES_VOPS)?;
    vp.set_vtype(if idx == NF { VREG } else { VDIR });
    vp.set_data(core::ptr::without_provenance_mut(idx + 1));
    if idx == NROOT {
        vp.set_flag(VROOT);
    }
    vn_lock(vp, LK_EXCLUSIVE)?;
    if let Ok(mut t) = NVREFS.lock() {
        t[idx] = Some(vp);
    }
    Ok(vp)
}

/// The names bench's vnode operations: a lock word per node, `ufs_lookup`'s protocol over
/// the name cache, the attributes `getcwd` and `vn_open` ask (a field, so the table has an
/// address of its own).
struct NamesVops {
    _id: u8,
}

static NAMES_VOPS: NamesVops = NamesVops { _id: 3 };

impl Vnops<TestGlue> for NamesVops {
    fn lock(&self, vp: &'static Vnode<TestGlue>, flags: i32) -> Result<(), Errno> {
        let l = &NLOCKED[nnode(vp)];
        if l.load(Ordering::Relaxed) != 0 {
            if flags & LK_NOWAIT != 0 {
                return Err(Errno::EBUSY);
            }
            panic!("namesfs: node {} locked twice", nnode(vp));
        }
        l.store(1, Ordering::Relaxed);
        Ok(())
    }
    fn unlock(&self, vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        NLOCKED[nnode(vp)].store(0, Ordering::Relaxed);
        Ok(())
    }
    fn islocked(&self, vp: &'static Vnode<TestGlue>) -> i32 {
        NLOCKED[nnode(vp)].load(Ordering::Relaxed) as i32
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
    fn lookup(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vpp: &mut Option<&'static Vnode<TestGlue>>,
        cnp: &mut Componentname<'_, TestGlue>,
    ) -> Result<(), Errno> {
        *vpp = None;
        if dvp.vtype() != VDIR {
            return Err(Errno::ENOTDIR);
        }
        let lockparent = cnp.cn_flags & LOCKPARENT != 0;
        let islast = cnp.cn_flags & ISLASTCN != 0;
        if let Some(vp) = cache_lookup(dvp, cnp)? {
            *vpp = Some(vp);
            return Ok(());
        }
        let dir = nnode(dvp);
        let name = cnp.name();
        let found = if name == b"." {
            Some(dir)
        } else if cnp.cn_flags & ISDOTDOT != 0 {
            Some(NNODES[dir].1)
        } else {
            (1..NN).find(|&i| NNODES[i].1 == dir && NNODES[i].0 == name)
        };
        let Some(idx) = found else {
            if (cnp.cn_nameiop == CREATE || cnp.cn_nameiop == RENAME) && islast {
                return Err(Errno::EJUSTRETURN);
            }
            return Err(Errno::ENOENT);
        };
        let Some(mp) = NMP.lock().ok().and_then(|m| *m) else {
            return Err(Errno::ENOENT);
        };
        let vp = if idx == dir {
            vref(dvp);
            dvp
        } else if cnp.cn_flags & ISDOTDOT != 0 {
            let _ = vop_unlock(dvp);
            cnp.cn_flags |= PDIRUNLOCK;
            let vp = nvget(mp, idx)?;
            if lockparent && islast {
                vn_lock(dvp, LK_EXCLUSIVE)?;
                cnp.cn_flags &= !PDIRUNLOCK;
            }
            vp
        } else {
            let vp = nvget(mp, idx)?;
            if !lockparent || !islast {
                let _ = vop_unlock(dvp);
                cnp.cn_flags |= PDIRUNLOCK;
            }
            vp
        };
        if cnp.cn_flags & MAKEENTRY != 0 {
            cache_enter(dvp, Some(vp), cnp.name());
        }
        *vpp = Some(vp);
        Ok(())
    }
    fn getattr(
        &self,
        vp: &'static Vnode<TestGlue>,
        vap: &mut Vattr,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        *vap = Vattr::new();
        vap.va_type = vp.vtype();
        vap.va_mode = 0o755;
        vap.va_nlink = 1;
        vap.va_fsid = 98;
        vap.va_fileid = 100 + nnode(vp) as u64;
        vap.va_size = 512;
        vap.va_blocksize = 512;
        Ok(())
    }
    fn access(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn open(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
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
    fn print(&self, _vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        Ok(())
    }
}

/// The names bench's file system operations: the root and the inode lookup.
struct NamesFs;

static NAMES_FS: NamesFs = NamesFs;

impl Vfs<TestGlue> for NamesFs {
    fn mount(
        &self,
        _mp: &'static Mount<TestGlue>,
        _path: &[u8],
        _data: &mut [u8],
        _ndp: &mut Nd<'_, TestGlue>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn start(
        &self,
        _mp: &'static Mount<TestGlue>,
        _f: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn unmount(
        &self,
        _mp: &'static Mount<TestGlue>,
        _f: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn root(&self, mp: &'static Mount<TestGlue>) -> Result<&'static Vnode<TestGlue>, Errno> {
        nvget(mp, NROOT)
    }
    fn quotactl(
        &self,
        _mp: &'static Mount<TestGlue>,
        _cmds: i32,
        _uid: hal::sys::types::Uid,
        _arg: usize,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn statfs(
        &self,
        _mp: &'static Mount<TestGlue>,
        _sbp: &mut hal::sys::mount::Statfs,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn sync(
        &self,
        _mp: &'static Mount<TestGlue>,
        _waitfor: i32,
        _stall: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn vget(
        &self,
        mp: &'static Mount<TestGlue>,
        ino: hal::sys::types::Ino,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        nvget(mp, ino as usize)
    }
    fn fhtovp(
        &self,
        _mp: &'static Mount<TestGlue>,
        _fid: &hal::sys::mount::Fid,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn vptofh(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _fid: &mut hal::sys::mount::Fid,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
}

/// The names bench's configured type.
static NAMES_CONF: Vfsconf<TestGlue> =
    Vfsconf::new(&NAMES_FS, b"namesfs", 98, hal::sys::mount::MNT_LOCAL, 0);

/// The names bench's file system as root, and a thread of a process of its own with
/// credentials and a descriptor table whose current directory is that root (what
/// `testfs::test_thread` makes for its own file system).
fn names_root() -> &'static Thread<TestGlue> {
    let mp = vfs_mount_alloc(None, &NAMES_CONF);
    vfs_unbusy(mp);
    if let Ok(mut m) = NMP.lock() {
        *m = Some(mp);
    }
    mountlist_insert_tail(mp);
    mp.flag_set(hal::sys::mount::MNT_ROOTFS);
    let root = need(VFS_ROOT(mp), "VFS_ROOT");
    kernel::fs::init::set_rootvnode(Some(root));
    vref(root);
    let _ = vop_unlock(root);
    kernel::proc::procinit::<TestGlue>(64, 64);
    let _ = kernel::descrip::filedesc_init::<kernel::fs::testfs::TestFiles>();
    kernel::testutil::set_test_maxfiles(4096);
    let pr = kernel::proc::process_alloc::<TestGlue>(1, ());
    drop(pr.set_ucred(kernel::cred::CredRef::new(kernel::cred::Ucred::new())));
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
    let t = kernel::proc::thread_alloc::<TestGlue>(&pr, ());
    drop(t.set_cred(pr.ucred()));
    let p: &'static Thread<TestGlue> = hal::pool::PoolBox::leak(t);
    core::mem::forget(pr);
    vref(root);
    p.fd().set_cdir(Some(root));
    TestGlue::set_curproc(Some(p));
    // The glue's record of namei's hooks is the tests', not the names'.
    kernel::testutil::set_file_log(false);
    p
}

/// `namei(path, op, flags)`, its vnode released (`vrele`).
fn lookup_rele(p: &'static Thread<TestGlue>, path: &'static [u8], flags: u64) {
    let mut nd = ndinit(LOOKUP, flags, NiDirp::Sys(path), p);
    need(namei(&mut nd), "namei");
    if let Some(vp) = nd.ni_vp {
        let _ = vrele(vp);
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
        "namei.hit.1",
        "namei.hit.3",
        "namei.miss.1",
        "cache.lookup.hit",
        "cache.enter.zap",
        "lf.setlk.unlck",
        "getcwd.cached",
        "vn_open.close",
        "prim.namecache-mtx",
        "open.close",
        "fstat",
        "lseek",
        "stat.3",
    ];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    // The recycling pools, the doubles, the kernel lock, the test file system mounted as root.
    let _ = hal::bridges::register_pool_backend(&BENCH_POOLS);
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
    if !names[10..].iter().any(|n| wanted(n)) {
        return;
    }

    // N5.3's rows: the names bench's file system as root, every name cached once.
    let p = names_root();
    let root = need(
        kernel::fs::init::rootvnode::<TestGlue>().ok_or(Errno::ENOENT),
        "root",
    );
    lookup_rele(p, b"/a/b/c", FOLLOW);
    lookup_rele(p, b"/f", FOLLOW);
    let refs = NVREFS.lock().map(|t| *t).unwrap_or([None; NN]);
    let cvp = need(refs[NC].ok_or(Errno::ENOENT), "c");
    let fvp = need(refs[NF].ok_or(Errno::ENOENT), "f");
    vref(cvp);
    vref(fvp);
    let cred = Cred::real(p.ucred());

    if wanted(names[10]) {
        report(
            names[10],
            measure(|| rounds(|| lookup_rele(p, black_box(b"/a"), FOLLOW))),
        );
    }
    if wanted(names[11]) {
        report(
            names[11],
            measure(|| rounds(|| lookup_rele(p, black_box(b"/a/b/c"), FOLLOW))),
        );
    }
    if wanted(names[12]) {
        report(
            names[12],
            measure(|| rounds(|| lookup_rele(p, black_box(b"/a"), NOCACHE | FOLLOW))),
        );
        // Cache `a` again for the rows below.
        lookup_rele(p, b"/a", FOLLOW);
    }
    if wanted(names[13]) {
        let _ = vn_lock(root, LK_EXCLUSIVE);
        report(
            names[13],
            measure(|| {
                rounds(|| {
                    let mut cn =
                        Componentname::new(LOOKUP, MAKEENTRY | ISLASTCN | LOCKPARENT, p, cred);
                    cn.set_name_lit(b"a");
                    if let Ok(Some(vp)) = cache_lookup(black_box(root), &mut cn) {
                        vput(vp);
                    }
                })
            }),
        );
        let _ = vop_unlock(root);
    }
    if wanted(names[14]) {
        // Eight entries kept (`initialvnodes`), sixteen names entered in turn: each entry is
        // of a name that is not cached and recycles the LRU's head.
        let names16: Vec<Vec<u8>> = (0..16).map(|i| format!("z{i:02}").into_bytes()).collect();
        let (cached, _) = cache_counts::<TestGlue>();
        nchinit::<TestGlue>(cached as i32 + 8);
        let mut i = 0;
        report(
            names[14],
            measure(|| {
                rounds(|| {
                    let name = &names16[i % 16];
                    i += 1;
                    cache_enter(black_box(root), Some(fvp), name);
                })
            }),
        );
        nchinit::<TestGlue>(64);
    }
    if wanted(names[15]) {
        let slot: &'static LockfSlot = Box::leak(Box::new(LockfSlot::new()));
        let id = LockOwner::File(core::ptr::from_ref(p).addr());
        report(
            names[15],
            measure(|| {
                rounds(|| {
                    let mut fl = Flock {
                        l_start: 0,
                        l_len: 100,
                        l_pid: 0,
                        l_type: F_WRLCK,
                        l_whence: SEEK_SET as i16,
                    };
                    let _ =
                        lf_advlock::<TestGlue>(black_box(slot), 0, id, F_SETLK, &mut fl, F_POSIX);
                    fl.l_type = F_UNLCK;
                    let _ =
                        lf_advlock::<TestGlue>(slot, 0, id, i32::from(F_UNLCK), &mut fl, F_POSIX);
                })
            }),
        );
    }
    if wanted(names[16]) {
        // cache.enter.zap's row recycled the LRU's oldest entries (a, b and c among
        // them) and getcwd enters no name: the path cached again.
        lookup_rele(p, b"/a/b/c", FOLLOW);
        let mut path = [0u8; MAXPATHLEN];
        report(
            names[16],
            measure(|| {
                rounds(|| {
                    let mut bp = path.len() - 1;
                    path[bp] = 0;
                    let _ = vfs_getcwd_common(
                        black_box(cvp),
                        None,
                        Some((&mut path, &mut bp)),
                        (MAXPATHLEN / 2) as i32,
                        GETCWD_CHECK_ACCESS,
                        p,
                    );
                    black_box(bp);
                })
            }),
        );
    }
    if wanted(names[17]) {
        report(
            names[17],
            measure(|| {
                rounds(|| {
                    let mut nd = ndinit(0, 0, NiDirp::Sys(black_box(b"/f")), p);
                    if vn_open(&mut nd, FREAD, 0).is_ok()
                        && let Some(vp) = nd.ni_vp
                    {
                        let _ = vop_unlock(vp);
                        let _ = vn_close(vp, FREAD, cred, Some(p));
                    }
                })
            }),
        );
    }
    if wanted(names[18]) {
        report(
            names[18],
            measure(|| {
                rounds(|| {
                    let g = NCLOCK.lock();
                    black_box(&g);
                })
            }),
        );
    }

    // N5.4's rows: the system calls, their paths and `struct stat` in user memory.
    let upath_f = UserPtr::<u8>::new(kernel::testutil::USER_BASE + 0x1200);
    let upath_abc = UserPtr::<u8>::new(kernel::testutil::USER_BASE + 0x1240);
    let ustat = UserPtr::<u8>::new(kernel::testutil::USER_BASE + 0x1280);
    need(upath_f.copyout_slice(b"/f\0"), "copyout");
    need(upath_abc.copyout_slice(b"/a/b/c\0"), "copyout");
    lookup_rele(p, b"/a/b/c", FOLLOW);
    let fd = need(sys_open(p, upath_f, O_RDONLY, 0), "open");
    need(kernel::descrip::sys_fstat(p, fd, ustat.addr()), "fstat");
    need(sys_lseek(p, fd, 0, SEEK_SET), "lseek");
    need(sys_stat(p, upath_abc, ustat), "stat");
    if wanted(names[19]) {
        report(
            names[19],
            measure(|| {
                rounds(|| {
                    if let Ok(fd) = sys_open(p, black_box(upath_f), O_RDONLY, 0) {
                        let _ = kernel::descrip::sys_close(p, fd);
                    }
                })
            }),
        );
    }
    if wanted(names[20]) {
        report(
            names[20],
            measure(|| {
                rounds(|| {
                    let _ = black_box(kernel::descrip::sys_fstat(p, black_box(fd), ustat.addr()));
                })
            }),
        );
    }
    if wanted(names[21]) {
        report(
            names[21],
            measure(|| {
                rounds(|| {
                    let _ = black_box(sys_lseek(p, black_box(fd), 0, SEEK_SET));
                })
            }),
        );
    }
    if wanted(names[22]) {
        report(
            names[22],
            measure(|| {
                rounds(|| {
                    let _ = black_box(sys_stat(p, black_box(upath_abc), ustat));
                })
            }),
        );
    }
    let _ = kernel::descrip::sys_close(p, fd);
}
/* </CODE> */
