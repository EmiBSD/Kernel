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
//! file system (a [`RecursiveRwlock`] per node, as ufs's inode), names looked up as
//! `ufs_lookup` does (the name cache first, then the directory), a device switch that records
//! its calls, and the set-up the VFS core's tests and benches share. Host tests and benches
//! only (`cfg(any(test, feature = "doubles"))`), never a kernel image.
//!
//! The test glue's `uiomove` moves no byte of a kernel buffer (`testutil::test_uiomove`: a
//! `UIO_SYSSPACE` iovec is accounted only), so the tree has no symbolic link and `readdir`
//! no entry: the links and the directory scan are tested over `bsd`'s glue, whose `uiomove`
//! copies.

use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::boxed::Box;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::rwlock::{RW_NOSLEEP, RW_READ, RW_RECURSEFAIL, RW_WRITE, RecursiveRwlock};
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, MNT_LOCAL, Statfs};
use hal::sys::types::{Dev, Ino, Uid};

use super::cache::{cache_enter, cache_lookup};
use super::mount::{Mount, Vfsconf, mountlist_insert_tail, vfs_mount_alloc, vfs_unbusy};
use super::namei::{
    CREATE, Componentname, ISDOTDOT, ISLASTCN, LOCKPARENT, MAKEENTRY, PDIRUNLOCK, RENAME,
};
use super::spec::{D_CLONE, D_DISK, D_TTY, Devsw, register_devsw};
use super::subr::{getnewvnode, vfs_test_reset, vget};
use super::vnode::{
    LK_EXCLUSIVE, LK_NOWAIT, LK_RECURSEFAIL, LK_SHARED, VDIR, VREG, VROOT, VT_TMPFS, Vattr, Vnode,
    vn_lock,
};
use super::vops::vop_unlock;
use super::vops::{Cred, IoBuf, Nd, Vfs, Vnops};
use crate::event::Knote;
use crate::proc::Thread;
use crate::uio::Uio;

crate::test_file_glue!(TestFiles, TestGlue);
crate::test_proc_glue!(TestGlue, TestFiles);

/// What a node is: a directory, a regular file or a socket.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A directory.
    Dir,
    /// A regular file.
    Reg,
    /// A socket (`VSOCK`: what `vn_open` refuses).
    Sock,
    /// A character device (`VCHR`: `/dev/console`).
    Chr,
}

/// `/`.
pub const ROOT: usize = 0;
/// `/a`.
pub const A: usize = 1;
/// `/a/b`, a file.
pub const B: usize = 2;
/// `/a/c`.
pub const C: usize = 3;
/// `/a/c/d`: a directory whose missing names cannot be created (`EACCES`).
pub const D: usize = 4;
/// `/f`, a file.
pub const F: usize = 5;
/// `/a/xxx...x` (40 bytes, longer than `NAMECACHE_MAXLEN`), a file.
pub const LONG: usize = 6;
/// `/s`, a socket.
pub const S: usize = 7;
/// `/dev`.
pub const DEV: usize = 8;
/// `/dev/console`, a character device (`TIOCCONS`'s lookup; what `acct(2)` refuses).
pub const CONSOLE: usize = 9;
/// The number of nodes.
pub const N: usize = 10;

/// The tree's kinds.
pub const KINDS: [Kind; N] = [
    Kind::Dir,
    Kind::Dir,
    Kind::Reg,
    Kind::Dir,
    Kind::Dir,
    Kind::Reg,
    Kind::Reg,
    Kind::Sock,
    Kind::Dir,
    Kind::Chr,
];

/// The tree's names and parents (the root is its own parent, as `..` of `/` is `/`).
pub const NAMES: [(&[u8], usize); N] = [
    (b"/", ROOT),
    (b"a", ROOT),
    (b"b", A),
    (b"c", A),
    (b"d", C),
    (b"f", ROOT),
    (b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", A),
    (b"s", ROOT),
    (b"dev", ROOT),
    (b"console", DEV),
];

/// A node's in-core state: its lock, its index and its record locks.
pub struct TestNode {
    /// `i_lock`.
    pub lock: RecursiveRwlock,
    /// The node's index.
    pub idx: usize,
    /// `i_lockf`: the node's advisory locks.
    pub lockf: super::lockf::LockfSlot,
}

std::thread_local! {
    /// This thread's nodes, by vnode address (the "inode hash"), with their mounts' addresses.
    static NODES: RefCell<Vec<(usize, &'static TestNode, &'static Vnode<TestGlue>, usize)>> =
        const { RefCell::new(Vec::new()) };
    /// This thread's mount.
    static MP: RefCell<Option<&'static Mount<TestGlue>>> = const { RefCell::new(None) };
    /// Whether this thread's `VOP_ACCESS` refuses ([`set_deny_access`]).
    static DENY_ACCESS: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    /// This thread's last `VOP_WRITE`: its size and `ioflag` ([`last_write`]).
    static LAST_WRITE: core::cell::Cell<Option<(usize, i32)>> =
        const { core::cell::Cell::new(None) };
    /// This thread's last `VOP_CLOSE`: whether its credentials were `NOCRED` and whether it
    /// named a thread ([`last_close`]).
    static LAST_CLOSE: core::cell::Cell<Option<(bool, bool)>> =
        const { core::cell::Cell::new(None) };
}

/// This thread's last `VOP_CLOSE` (taken): `(cred is NOCRED, a thread given)`.
pub fn last_close() -> Option<(bool, bool)> {
    LAST_CLOSE.with(core::cell::Cell::take)
}

/// This thread's last `VOP_WRITE` (taken): the bytes it was asked to write and its `ioflag`.
pub fn last_write() -> Option<(usize, i32)> {
    LAST_WRITE.with(core::cell::Cell::take)
}

/// Makes this thread's `VOP_ACCESS` refuse every request (`EACCES`), or allow it again (the
/// default, which [`setup`] restores).
pub fn set_deny_access(on: bool) {
    DENY_ACCESS.with(|d| d.set(on));
}

/// How many times `VOP_INACTIVE` ran, on any test thread.
pub static INACTIVE: AtomicUsize = AtomicUsize::new(0);
/// How many times `VOP_RECLAIM` ran.
pub static RECLAIM: AtomicUsize = AtomicUsize::new(0);
/// How many times a node was unlocked.
pub static UNLOCKS: AtomicUsize = AtomicUsize::new(0);
/// How many times `VOP_SETATTR` ran (`vn_open`'s `O_TRUNC`).
pub static SETATTRS: AtomicUsize = AtomicUsize::new(0);
/// How many times `VOP_OPEN` ran.
pub static VOPENS: AtomicUsize = AtomicUsize::new(0);

/// The node of a test vnode, if it has one.
pub fn node_of(vp: &Vnode<TestGlue>) -> Option<&'static TestNode> {
    NODES.with(|n| n.borrow().iter().find(|e| e.0 == vp.addr()).map(|e| e.1))
}

/// Whether any node of this thread holds its lock.
pub fn any_locked() -> bool {
    NODES.with(|n| n.borrow().iter().any(|e| e.1.lock.status() != 0))
}

/// The use count of every node's vnode in the root's mount (0 for none).
pub fn usecounts() -> [u32; N] {
    let root = super::init::rootvnode::<TestGlue>().and_then(Vnode::mount);
    core::array::from_fn(|i| {
        root.and_then(|mp| vnode_in(mp, i))
            .map_or(0, |vp| vp.usecount())
    })
}

/// The vnode of node `idx`, if it has one (in the first mount that has it).
pub fn vnode_of(idx: usize) -> Option<&'static Vnode<TestGlue>> {
    NODES.with(|n| n.borrow().iter().find(|e| e.1.idx == idx).map(|e| e.2))
}

/// The vnode of node `idx` of the mount `mp`, if it has one.
pub fn vnode_in(mp: &Mount<TestGlue>, idx: usize) -> Option<&'static Vnode<TestGlue>> {
    let m = core::ptr::from_ref(mp).addr();
    NODES.with(|n| {
        n.borrow()
            .iter()
            .find(|e| e.1.idx == idx && e.3 == m)
            .map(|e| e.2)
    })
}

/// The vnode of node `idx` of `mp`, referenced and locked (`VFS_VGET`).
pub fn vget_node(
    mp: &'static Mount<TestGlue>,
    idx: usize,
) -> Result<&'static Vnode<TestGlue>, Errno> {
    if let Some(vp) = vnode_in(mp, idx) {
        vget(vp, LK_EXCLUSIVE)?;
        return Ok(vp);
    }
    let vp = getnewvnode(VT_TMPFS, Some(mp), &TEST_VOPS)?;
    vp.set_vtype(match KINDS[idx] {
        Kind::Dir => VDIR,
        Kind::Reg => VREG,
        Kind::Sock => super::vnode::VSOCK,
        Kind::Chr => super::vnode::VCHR,
    });
    if idx == ROOT {
        vp.set_flag(VROOT);
    }
    let node: &'static TestNode = Box::leak(Box::new(TestNode {
        lock: RecursiveRwlock::new("testnode"),
        idx,
        lockf: super::lockf::LockfSlot::new(),
    }));
    let m = core::ptr::from_ref(mp).addr();
    NODES.with(|n| n.borrow_mut().push((vp.addr(), node, vp, m)));
    vn_lock(vp, LK_EXCLUSIVE)?;
    Ok(vp)
}

/// The vnode `ufs_lookup` gives for `name` in the directory `dvp` (locked): the name cache
/// first, then the tree; the parent stays locked only for `LOCKPARENT` on the last
/// component (`PDIRUNLOCK` otherwise). A missing name is `ENOENT` (cached negatively), or
/// `EJUSTRETURN` for the last component of a `CREATE` or `RENAME`, but in `D`, which
/// refuses it (`EACCES`).
fn test_lookup(
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

    let (Some(mp), Some(dir)) = (dvp.mount(), node_of(dvp).map(|n| n.idx)) else {
        return Err(Errno::ENOENT);
    };
    let name = cnp.name();
    let found = if name == b"." {
        Some(dir)
    } else if cnp.cn_flags & ISDOTDOT != 0 {
        Some(NAMES[dir].1)
    } else {
        (1..N).find(|&i| NAMES[i].1 == dir && NAMES[i].0 == name)
    };
    let Some(idx) = found else {
        if (cnp.cn_nameiop == CREATE || cnp.cn_nameiop == RENAME) && islast {
            if dir == D {
                return Err(Errno::EACCES);
            }
            if !lockparent {
                let _ = vop_unlock(dvp);
                cnp.cn_flags |= PDIRUNLOCK;
            }
            return Err(Errno::EJUSTRETURN);
        }
        if cnp.cn_flags & MAKEENTRY != 0 && cnp.cn_nameiop != CREATE {
            cache_enter(dvp, None, cnp.name());
        }
        return Err(Errno::ENOENT);
    };

    let vp = if idx == dir {
        super::subr::vref(dvp);
        dvp
    } else if cnp.cn_flags & ISDOTDOT != 0 {
        let _ = vop_unlock(dvp);
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
        vap.va_mode = 0o755;
        vap.va_nlink = 1;
        vap.va_fsid = 99;
        vap.va_fileid = 100 + node_of(vp).map_or(0, |n| n.idx as u64);
        vap.va_blocksize = 512;
        vap.va_bytes = 1024;
        Ok(())
    }
    fn lookup(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vpp: &mut Option<&'static Vnode<TestGlue>>,
        cnp: &mut Componentname<'_, TestGlue>,
    ) -> Result<(), Errno> {
        test_lookup(dvp, vpp, cnp)
    }
    fn access(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        if DENY_ACCESS.with(core::cell::Cell::get) {
            Err(Errno::EACCES)
        } else {
            Ok(())
        }
    }
    fn open(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        VOPENS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn setattr(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _vap: &mut Vattr,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        SETATTRS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn abortop(
        &self,
        _dvp: &'static Vnode<TestGlue>,
        cnp: &mut Componentname<'_, TestGlue>,
    ) -> Result<(), Errno> {
        super::default::vop_generic_abortop::<TestGlue>(cnp)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<TestGlue>,
        id: crate::file::LockOwner,
        op: i32,
        fl: &mut hal::sys::fcntl::Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        let Some(node) = node_of(vp) else {
            return Err(Errno::EOPNOTSUPP);
        };
        super::lockf::lf_advlock::<TestGlue>(&node.lockf, 512, id, op, fl, flags)
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
    fn write(
        &self,
        _vp: &'static Vnode<TestGlue>,
        uio: &mut Uio<'_, TestGlue>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        // The bytes go nowhere; the call is recorded (`last_write`).
        let n = uio.uio_resid;
        LAST_WRITE.with(|w| w.set(Some((n, ioflag))));
        let mut buf = std::vec![0u8; n];
        <TestFiles as crate::file::FileGlue>::uiomove(&mut buf, uio)
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
        cred: Cred<'_>,
        p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        LAST_CLOSE.with(|c| c.set(Some((cred.is_nocred(), p.is_some()))));
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
    /// A file system of 100 blocks, 50 of them free.
    fn statfs(
        &self,
        _mp: &'static Mount<TestGlue>,
        sbp: &mut Statfs,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        sbp.f_blocks = 100;
        sbp.f_bavail = 50;
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
    DENY_ACCESS.with(|d| d.set(false));
    super::init::vfsinit::<TestGlue>(&TEST_CONF, 64, || {});
    super::lockf::lf_init();
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

/// A second mount of the test file system on the directory `covered` (unlocked, referenced
/// by the caller): `mnt_vnodecovered` and `v_mountedhere`, as `sys_mount` links them; on the
/// mount list.
pub fn mount_on(covered: &'static Vnode<TestGlue>) -> &'static Mount<TestGlue> {
    let mp = vfs_mount_alloc(Some(covered), &TEST_CONF[0]);
    vfs_unbusy(mp);
    mountlist_insert_tail(mp);
    covered.set_mountedhere(Some(mp));
    mp
}

/// [`setup_root`] plus a thread of a process of its own (pid 1, credentials of root, a
/// descriptor table whose current directory is the root), `curproc` of the test thread; for
/// the tests that look names up. Leaked: the tests' threads live for the run.
pub fn test_thread() -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    std::thread_local! {
        static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    }
    let (g, _mp) = setup_root();
    if !MADE.with(|m| m.replace(true)) {
        crate::proc::procinit::<TestGlue>(64, 64);
        let _ = crate::descrip::filedesc_init::<TestFiles>();
    }
    crate::testutil::set_test_maxfiles(4096);
    let pr = crate::proc::process_alloc::<TestGlue>(1, ());
    drop(pr.set_ucred(crate::cred::CredRef::new(crate::cred::Ucred::new())));
    let mut limits = crate::resource::Plimit::new();
    for l in &mut limits.pl_rlimit {
        l.rlim_cur = crate::resource::RLIM_INFINITY;
        l.rlim_max = crate::resource::RLIM_INFINITY;
    }
    limits.pl_rlimit[crate::resource::RLIMIT_NOFILE].rlim_cur = 512;
    limits.pl_rlimit[crate::resource::RLIMIT_NOFILE].rlim_max = 512;
    drop(pr.set_limit(crate::resource::PlimitRef::new(limits)));
    pr.set_fd(crate::descrip::fdinit());
    let t = crate::proc::thread_alloc::<TestGlue>(&pr, ());
    drop(t.set_cred(pr.ucred()));
    let p: &'static Thread<TestGlue> = hal::pool::PoolBox::leak(t);
    core::mem::forget(pr);
    if let Some(root) = super::init::rootvnode::<TestGlue>() {
        super::subr::vref(root);
        p.fd().set_cdir(Some(root));
    }
    TestGlue::set_curproc(Some(p));
    (g, p)
}
/* </CODE> */
