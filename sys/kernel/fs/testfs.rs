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
//! The kernel's test file system: a glue of the tests (`TestGlue`, `TestFiles`, each test
//! thread with its own VFS tables), a fixed tree of nodes with the lock discipline of a real
//! file system (a [`RecursiveRwlock`] per node, as ufs's inode), a device switch that records
//! its calls, and the set-up the VFS core's tests and benches share. Host tests and benches
//! only (`cfg(any(test, feature = "doubles"))`), never a kernel image.

use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::boxed::Box;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::rwlock::{RW_NOSLEEP, RW_READ, RW_RECURSEFAIL, RW_WRITE, RecursiveRwlock};
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, MNT_LOCAL, Statfs};
use hal::sys::types::{Dev, Ino, Uid};

use super::mount::{Mount, Vfsconf, mountlist_insert_tail, vfs_mount_alloc, vfs_unbusy};
use super::spec::{D_CLONE, D_DISK, D_TTY, Devsw, register_devsw};
use super::subr::{getnewvnode, vfs_test_reset, vget};
use super::vnode::{
    LK_EXCLUSIVE, LK_NOWAIT, LK_RECURSEFAIL, LK_SHARED, VDIR, VREG, VROOT, VT_TMPFS, Vattr, Vnode,
    vn_lock,
};
use super::vops::{Cred, IoBuf, Nd, Vfs, Vnops};
use crate::event::Knote;
use crate::proc::Thread;
use crate::uio::Uio;

crate::test_file_glue!(TestFiles, TestGlue);
crate::test_proc_glue!(TestGlue, TestFiles);

/// What a node is: a directory or a regular file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A directory.
    Dir,
    /// A regular file.
    Reg,
}

/// `/`.
pub const ROOT: usize = 0;
/// `/a`.
pub const A: usize = 1;
/// `/a/b`, a file.
pub const B: usize = 2;
/// `/a/c`.
pub const C: usize = 3;
/// The number of nodes.
pub const N: usize = 4;

/// The tree's kinds.
pub const KINDS: [Kind; N] = [Kind::Dir, Kind::Dir, Kind::Reg, Kind::Dir];

/// A node's in-core state: its lock and the vnode it has.
pub struct TestNode {
    /// `i_lock`.
    pub lock: RecursiveRwlock,
    /// The node's index.
    pub idx: usize,
}

std::thread_local! {
    /// This thread's nodes, by vnode address (the "inode hash").
    static NODES: RefCell<Vec<(usize, &'static TestNode, &'static Vnode<TestGlue>)>> =
        const { RefCell::new(Vec::new()) };
    /// This thread's mount.
    static MP: RefCell<Option<&'static Mount<TestGlue>>> = const { RefCell::new(None) };
}

/// How many times `VOP_INACTIVE` ran, on any test thread.
pub static INACTIVE: AtomicUsize = AtomicUsize::new(0);
/// How many times `VOP_RECLAIM` ran.
pub static RECLAIM: AtomicUsize = AtomicUsize::new(0);
/// How many times a node was unlocked.
pub static UNLOCKS: AtomicUsize = AtomicUsize::new(0);

/// The node of a test vnode, if it has one.
pub fn node_of(vp: &Vnode<TestGlue>) -> Option<&'static TestNode> {
    NODES.with(|n| n.borrow().iter().find(|e| e.0 == vp.addr()).map(|e| e.1))
}

/// The vnode of node `idx`, if it has one.
pub fn vnode_of(idx: usize) -> Option<&'static Vnode<TestGlue>> {
    NODES.with(|n| n.borrow().iter().find(|e| e.1.idx == idx).map(|e| e.2))
}

/// The vnode of node `idx`, referenced and locked (`VFS_VGET`).
pub fn vget_node(
    mp: &'static Mount<TestGlue>,
    idx: usize,
) -> Result<&'static Vnode<TestGlue>, Errno> {
    if let Some(vp) = vnode_of(idx) {
        vget(vp, LK_EXCLUSIVE)?;
        return Ok(vp);
    }
    let vp = getnewvnode(VT_TMPFS, Some(mp), &TEST_VOPS)?;
    vp.set_vtype(match KINDS[idx] {
        Kind::Dir => VDIR,
        Kind::Reg => VREG,
    });
    if idx == ROOT {
        vp.set_flag(VROOT);
    }
    let node: &'static TestNode = Box::leak(Box::new(TestNode {
        lock: RecursiveRwlock::new("testnode"),
        idx,
    }));
    NODES.with(|n| n.borrow_mut().push((vp.addr(), node, vp)));
    vn_lock(vp, LK_EXCLUSIVE)?;
    Ok(vp)
}

/// `lk` flags as the node's rrwlock takes them (`ufs_lock`'s `ap->a_flags & LK_RWFLAGS`).
fn rw_flags(flags: i32) -> i32 {
    let mut rw = 0;
    if flags & LK_EXCLUSIVE != 0 {
        rw |= RW_WRITE;
    }
    if flags & LK_SHARED != 0 {
        rw |= RW_READ;
    }
    if flags & LK_NOWAIT != 0 {
        rw |= RW_NOSLEEP;
    }
    if flags & LK_RECURSEFAIL != 0 {
        rw |= RW_RECURSEFAIL;
    }
    rw
}

/// The test file system's vnode operations (a field, so its address is its own).
pub struct TestVops {
    _id: u8,
}

/// The table.
pub static TEST_VOPS: TestVops = TestVops { _id: 1 };

impl Vnops<TestGlue> for TestVops {
    fn lock(&self, vp: &'static Vnode<TestGlue>, flags: i32) -> Result<(), Errno> {
        match node_of(vp) {
            Some(n) => n.lock.enter(rw_flags(flags)),
            None => Ok(()),
        }
    }
    fn unlock(&self, vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        if let Some(n) = node_of(vp) {
            UNLOCKS.fetch_add(1, Ordering::SeqCst);
            n.lock.exit();
        }
        Ok(())
    }
    fn islocked(&self, vp: &'static Vnode<TestGlue>) -> i32 {
        node_of(vp).map_or(0, |n| n.lock.status())
    }
    fn inactive(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        INACTIVE.fetch_add(1, Ordering::SeqCst);
        self.unlock(vp)
    }
    fn reclaim(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        RECLAIM.fetch_add(1, Ordering::SeqCst);
        NODES.with(|n| n.borrow_mut().retain(|e| e.0 != vp.addr()));
        vp.set_data(core::ptr::null_mut());
        Ok(())
    }
    fn getattr(
        &self,
        vp: &'static Vnode<TestGlue>,
        vap: &mut Vattr,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        vap.va_type = vp.vtype();
        vap.va_size = 512;
        Ok(())
    }
    fn read(
        &self,
        _vp: &'static Vnode<TestGlue>,
        uio: &mut Uio<'_, TestGlue>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        // A file of 'x' bytes.
        let mut buf = [b'x'; 64];
        let n = uio.uio_resid.min(buf.len());
        <TestFiles as crate::file::FileGlue>::uiomove(&mut buf[..n], uio)
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
    fn print(&self, _vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
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
    fn strategy(&self, _vp: &'static Vnode<TestGlue>, bp: IoBuf) -> Result<(), Errno> {
        if let Some(t) = bp.into_token() {
            super::bio::biodone(t);
        }
        Ok(())
    }
}

/// The test file system's operations: the root and the inode lookup do anything.
pub struct TestFs;

/// The table.
pub static TEST_VFS: TestFs = TestFs;

impl Vfs<TestGlue> for TestFs {
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
        _flags: i32,
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
        vget_node(mp, ROOT)
    }
    fn quotactl(
        &self,
        _mp: &'static Mount<TestGlue>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn statfs(
        &self,
        _mp: &'static Mount<TestGlue>,
        _sbp: &mut Statfs,
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
        ino: Ino,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        vget_node(mp, ino as usize)
    }
    fn fhtovp(
        &self,
        _mp: &'static Mount<TestGlue>,
        _fid: &Fid,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn vptofh(&self, _vp: &'static Vnode<TestGlue>, _fid: &mut Fid) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
}

/// The configured types of the tests: the test file system alone.
pub static TEST_CONF: [Vfsconf<TestGlue>; 1] =
    [Vfsconf::new(&TEST_VFS, b"testfs", 99, MNT_LOCAL, 0)];

/// The device switch of the tests: majors 1 (a tty), 2 (a disk), 3 (a cloning device);
/// opens and closes succeed and are counted.
pub struct TestDevsw;

/// Opens the test device switch saw.
pub static OPENS: AtomicUsize = AtomicUsize::new(0);
/// Closes the test device switch saw.
pub static CLOSES: AtomicUsize = AtomicUsize::new(0);

impl Devsw<TestGlue> for TestDevsw {
    fn nchrdev(&self) -> u32 {
        8
    }
    fn nblkdev(&self) -> u32 {
        8
    }
    fn cdev_type(&self, maj: u32) -> u32 {
        match maj {
            1 => D_TTY,
            2 => D_DISK,
            _ => 0,
        }
    }
    fn cdev_flags(&self, maj: u32) -> u32 {
        if maj == 3 { D_CLONE } else { 0 }
    }
    fn bdev_type(&self, maj: u32) -> u32 {
        if maj == 2 { D_DISK } else { 0 }
    }
    fn cdev_open(
        &self,
        _dev: Dev,
        _oflags: i32,
        _devtype: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        OPENS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn cdev_close(
        &self,
        _dev: Dev,
        _f: i32,
        _t: i32,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        CLOSES.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn cdev_read(
        &self,
        _dev: Dev,
        _uio: &mut Uio<'_, TestGlue>,
        _ioflag: i32,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn cdev_write(
        &self,
        _dev: Dev,
        _uio: &mut Uio<'_, TestGlue>,
        _ioflag: i32,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn cdev_ioctl(
        &self,
        _dev: Dev,
        _cmd: u64,
        _data: &mut [u8],
        _fflag: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::ENOTTY)
    }
    fn cdev_kqfilter(&self, _dev: Dev, _kn: &Knote<TestGlue>) -> Option<Result<(), Errno>> {
        None
    }
    fn bdev_open(&self, _dev: Dev, _o: i32, _t: i32, _p: &Thread<TestGlue>) -> Result<(), Errno> {
        OPENS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn bdev_close(
        &self,
        _dev: Dev,
        _f: i32,
        _t: i32,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        CLOSES.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn bdev_ioctl(
        &self,
        _dev: Dev,
        _cmd: u64,
        _data: &mut [u8],
        _fflag: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::ENOTTY)
    }
    fn bdev_strategy(&self, bp: IoBuf) {
        if let Some(t) = bp.into_token() {
            super::bio::biodone(t);
        }
    }
    fn chrtoblk(&self, _dev: Dev) -> Dev {
        super::bio::NODEV
    }
    fn iskmemdev(&self, _dev: Dev) -> bool {
        false
    }
    fn iszerodev(&self, _dev: Dev) -> bool {
        false
    }
}

/// The device switch.
pub static TEST_DEVSW: TestDevsw = TestDevsw;

/// The VFS tables of this test thread made afresh: the doubles, the kernel lock (the VOPs
/// run under it, as the system calls that reach them take it), `vfsinit` of the test file
/// system, the device switch; the old lists forgotten. The guard of the kernel's memory.
pub fn setup() -> MutexGuard<'static, ()> {
    let g = crate::testutil::pages_reset();
    hal::klock::kernel_lock();
    vfs_test_reset::<TestGlue>();
    NODES.with(|n| n.borrow_mut().clear());
    super::init::vfsinit::<TestGlue>(&TEST_CONF, 64, || {});
    let _ = register_devsw::<TestGlue>(&TEST_DEVSW);
    g
}

/// [`setup`] plus the test file system mounted as root: on the mount list with its root
/// vnode, which is the thread's `rootvnode`, unlocked and referenced twice (the mount's
/// root and the caller's).
pub fn setup_root() -> (MutexGuard<'static, ()>, &'static Mount<TestGlue>) {
    let g = setup();
    let mp = vfs_mount_alloc(None, &TEST_CONF[0]);
    vfs_unbusy(mp);
    mountlist_insert_tail(mp);
    MP.with(|m| *m.borrow_mut() = Some(mp));
    let Ok(root) = super::vops::VFS_ROOT(mp) else {
        std::panic!("testfs: no root");
    };
    super::init::set_rootvnode(Some(root));
    super::subr::vref(root);
    let _ = super::vops::vop_unlock(root);
    (g, mp)
}
/* </CODE> */
