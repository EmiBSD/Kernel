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
//! The test glue's `uiomove` copies a user iovec through the user-copy double and accounts a
//! kernel (`UIO_SYSSPACE`) one only (`testutil::test_uiomove`), unless a test turns the
//! kernel copy on (`testutil::set_copy_kernel`): `readdir` and `readlink` into user memory
//! are tested here, a symbolic link namei follows with the kernel copy on.
//!
//! N5.4 (D-N5.4-9): the tree is each test thread's own copy of the fixed one ([`setup`]
//! makes it), which the namespace operations change: `create`, `mknod`, `mkdir`, `rmdir`,
//! `remove`, `link`, `symlink` and `rename` (each releasing what `ufs`'s releases),
//! `readdir` (`struct dirent` records of `.`, `..` and the live names), `pathconf`,
//! `revoke` (counted), `setattr` and `mknod` recording what they were given
//! ([`last_setattr`], [`last_mknod`]), `getattr`'s mode and owner set by the test
//! ([`set_attr`]); two character devices with a device number (`/dev/ttyp0`, a tty of
//! [`TestDevsw`]'s major 1, and `/dev/sd0c`, a disk of major 2); the file system's `mount`
//! records its arguments and may be made to fail ([`last_mount`], [`set_mount_error`]),
//! `unmount` flushes the mount's vnodes (`vflush`, `FORCECLOSE` under `MNT_FORCE`), `statfs`
//! and `sync` are counted, and file handles name a node of a mount (`vptofh`, `fhtovp`).

use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::boxed::Box;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::rwlock::{RW_NOSLEEP, RW_READ, RW_RECURSEFAIL, RW_WRITE, RecursiveRwlock};
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, MNT_LOCAL, Statfs};
use hal::sys::types::{Dev, Ino, Mode, Uid};

use super::cache::{cache_enter, cache_lookup};
use super::mount::{Mount, Vfsconf, mountlist_insert_tail, vfs_mount_alloc, vfs_unbusy};
use super::namei::{
    CREATE, Componentname, ISDOTDOT, ISLASTCN, LOCKPARENT, MAKEENTRY, PDIRUNLOCK, RENAME, SAVENAME,
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
    /// A symbolic link to the path (`VLNK`; its bytes reach namei while the test glue copies
    /// kernel bytes, `testutil::set_copy_kernel`).
    Lnk(&'static [u8]),
    /// A block device (`VBLK`: `mknod`'s `S_IFBLK`).
    Blk,
    /// A bad node (`VBAD`: `mknod`'s `S_IFMT`, badsect's).
    Bad,
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
/// `/a/abs -> /f`, an absolute symbolic link.
pub const ABS: usize = 10;
/// `/lnk -> a/c`, a relative symbolic link.
pub const LNK: usize = 11;
/// `/dev/ttyp0`, a character device of [`TestDevsw`]'s tty major 1 (`revoke(2)`'s).
pub const TTY: usize = 12;
/// `/dev/sd0c`, a character device of [`TestDevsw`]'s disk major 2 (not a tty).
pub const DISK: usize = 13;
/// The number of nodes of the fixed tree (a test thread's tree grows past it).
pub const N: usize = 14;

/// `/dev/ttyp0`'s device number.
pub const TTY_DEV: Dev = hal::sys::types::makedev(1, 0);
/// `/dev/sd0c`'s device number.
pub const DISK_DEV: Dev = hal::sys::types::makedev(2, 2);

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
    Kind::Lnk(b"/f"),
    Kind::Lnk(b"a/c"),
    Kind::Chr,
    Kind::Chr,
];

/// The tree's device numbers (the devices only).
pub const RDEVS: [Option<Dev>; N] = {
    let mut r = [None; N];
    r[TTY] = Some(TTY_DEV);
    r[DISK] = Some(DISK_DEV);
    r
};

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
    (b"abs", A),
    (b"lnk", ROOT),
    (b"ttyp0", DEV),
    (b"sd0c", DEV),
];

/// A node of a test thread's tree: its name in its parent, what it is, its device number,
/// whether it is still linked.
#[derive(Clone)]
pub struct TreeNode {
    /// The name.
    pub name: Vec<u8>,
    /// The directory it is in (the root is its own).
    pub parent: usize,
    /// What it is.
    pub kind: Kind,
    /// The device number of a device.
    pub rdev: Option<Dev>,
    /// Whether the name is linked (a removed node keeps its index).
    pub live: bool,
}

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
    /// This thread's tree ([`setup`] copies the fixed one).
    static TREE: RefCell<Vec<TreeNode>> = const { RefCell::new(Vec::new()) };
    /// This thread's last `VOP_SETATTR`'s attributes ([`last_setattr`]).
    static LAST_SETATTR: core::cell::Cell<Option<Vattr>> = const { core::cell::Cell::new(None) };
    /// This thread's last `VOP_MKNOD`'s attributes ([`last_mknod`]).
    static LAST_MKNOD: core::cell::Cell<Option<Vattr>> = const { core::cell::Cell::new(None) };
    /// What this thread's `VOP_GETATTR` answers for the mode and the owner ([`set_attr`]).
    static ATTR: core::cell::Cell<(Mode, Uid)> = const { core::cell::Cell::new((0o755, 0)) };
    /// This thread's last `VFS_MOUNT`'s arguments ([`last_mount`]).
    static LAST_MOUNT: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
    /// What this thread's `VFS_MOUNT` answers ([`set_mount_error`]).
    static MOUNT_ERROR: core::cell::Cell<Option<Errno>> = const { core::cell::Cell::new(None) };
    /// This thread's last `VOP_CREATE`'s, `VOP_MKDIR`'s or `VOP_SYMLINK`'s attributes
    /// ([`last_create`]).
    static LAST_CREATE: core::cell::Cell<Option<Vattr>> = const { core::cell::Cell::new(None) };
    /// What this thread's next `VOP_OPEN` answers, once, and the descriptor it names in
    /// `p_dupfd` first ([`set_open_error`]).
    static OPEN_ERROR: core::cell::Cell<Option<(Errno, Option<i32>)>> =
        const { core::cell::Cell::new(None) };
    /// The effective uid of the credentials of this thread's last `VOP_ACCESS`
    /// ([`last_access_uid`]).
    static LAST_ACCESS_UID: core::cell::Cell<Option<Uid>> = const { core::cell::Cell::new(None) };
    /// The mount's flags this thread's last `VFS_SYNC` saw ([`last_sync_flags`]).
    static LAST_SYNC_FLAGS: core::cell::Cell<Option<i32>> = const { core::cell::Cell::new(None) };
    /// What this thread's next `VOP_CREATE` answers, once ([`set_create_error`]).
    static CREATE_ERROR: core::cell::Cell<Option<Errno>> = const { core::cell::Cell::new(None) };
    /// The error this thread's next `VOP_STRATEGY` gives its buffer, once
    /// ([`set_strategy_error`]).
    static STRATEGY_ERROR: core::cell::Cell<Option<Errno>> = const { core::cell::Cell::new(None) };
}

/// Makes this thread's next `VOP_STRATEGY` complete its buffer with `e` (`B_ERROR`), as a
/// disk's failed read.
pub fn set_strategy_error(e: Errno) {
    STRATEGY_ERROR.with(|c| c.set(Some(e)));
}

/// Makes this thread's next `VOP_CREATE` fail with `e`.
pub fn set_create_error(e: Errno) {
    CREATE_ERROR.with(|c| c.set(Some(e)));
}

/// This thread's last `VOP_CREATE`'s, `VOP_MKDIR`'s or `VOP_SYMLINK`'s attributes (taken).
pub fn last_create() -> Option<Vattr> {
    LAST_CREATE.with(core::cell::Cell::take)
}

/// Makes this thread's next `VOP_OPEN` fail with `e`, after setting the thread's `p_dupfd` to
/// `dupfd` when given (a `/dev/fd/N` driver's open: `ENODEV` and the descriptor to dup).
pub fn set_open_error(e: Errno, dupfd: Option<i32>) {
    OPEN_ERROR.with(|c| c.set(Some((e, dupfd))));
}

/// The effective uid of the credentials of this thread's last `VOP_ACCESS` (taken): the
/// thread's own (`p_ucred`), which `access(2)` swaps for the real ids.
pub fn last_access_uid() -> Option<Uid> {
    LAST_ACCESS_UID.with(core::cell::Cell::take)
}

/// The mount's flags this thread's last `VFS_SYNC` saw (taken).
pub fn last_sync_flags() -> Option<i32> {
    LAST_SYNC_FLAGS.with(core::cell::Cell::take)
}

/// This thread's last `VOP_SETATTR`'s attributes (taken).
pub fn last_setattr() -> Option<Vattr> {
    LAST_SETATTR.with(core::cell::Cell::take)
}

/// This thread's last `VOP_MKNOD`'s attributes (taken).
pub fn last_mknod() -> Option<Vattr> {
    LAST_MKNOD.with(core::cell::Cell::take)
}

/// Makes this thread's `VOP_GETATTR` answer `mode` and the owner `uid` (`0o755` and root
/// until set; [`setup`] restores them).
pub fn set_attr(mode: Mode, uid: Uid) {
    ATTR.with(|a| a.set((mode, uid)));
}

/// This thread's last `VFS_MOUNT`'s argument bytes (taken).
pub fn last_mount() -> Option<Vec<u8>> {
    LAST_MOUNT.with(|m| m.borrow_mut().take())
}

/// Makes this thread's `VFS_MOUNT` fail with `e` (`None`: succeed).
pub fn set_mount_error(e: Option<Errno>) {
    MOUNT_ERROR.with(|m| m.set(e));
}

/// The node of this thread's tree named `name` in the directory `dir`, if it is linked.
pub fn tree_find(dir: usize, name: &[u8]) -> Option<usize> {
    TREE.with(|t| {
        t.borrow()
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, n)| n.live && n.parent == dir && n.name == name)
            .map(|(i, _)| i)
    })
}

/// A copy of node `idx` of this thread's tree.
pub fn tree_node(idx: usize) -> Option<TreeNode> {
    TREE.with(|t| t.borrow().get(idx).cloned())
}

/// Links a new node into this thread's tree: its index.
fn tree_add(name: &[u8], parent: usize, kind: Kind, rdev: Option<Dev>) -> usize {
    TREE.with(|t| {
        let mut t = t.borrow_mut();
        t.push(TreeNode {
            name: name.to_vec(),
            parent,
            kind,
            rdev,
            live: true,
        });
        t.len() - 1
    })
}

/// Unlinks node `idx` of this thread's tree.
fn tree_unlink(idx: usize) {
    TREE.with(|t| {
        if let Some(n) = t.borrow_mut().get_mut(idx) {
            n.live = false;
        }
    });
}

/// The linked nodes of the directory `dir`, in order.
fn tree_children(dir: usize) -> Vec<(usize, TreeNode)> {
    TREE.with(|t| {
        t.borrow()
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, n)| n.live && n.parent == dir)
            .map(|(i, n)| (i, n.clone()))
            .collect()
    })
}

/// The kind of node `idx` (a regular file for an index this thread's tree does not have).
fn tree_kind(idx: usize) -> Kind {
    tree_node(idx).map_or(Kind::Reg, |n| n.kind)
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
    vp.set_vtype(match tree_kind(idx) {
        Kind::Dir => VDIR,
        Kind::Reg => VREG,
        Kind::Sock => super::vnode::VSOCK,
        Kind::Chr => super::vnode::VCHR,
        Kind::Lnk(_) => super::vnode::VLNK,
        Kind::Blk => super::vnode::VBLK,
        Kind::Bad => super::vnode::VBAD,
    });
    if idx == ROOT {
        vp.set_flag(VROOT);
    }
    // A device of the tree gets its specinfo (`ufs`'s `checkalias` at `ffs_vget`); no other
    // vnode of the thread has its number.
    if let Some(rdev) = tree_node(idx).and_then(|n| n.rdev) {
        let _ = super::subr::checkalias(vp, rdev, Some(mp));
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
        tree_node(dir).map(|n| n.parent)
    } else {
        tree_find(dir, name)
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
            // The pathname buffer is saved so that the name can be obtained later
            // (ufs_lookup's).
            cnp.cn_flags |= SAVENAME;
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
    // A rename's existing target: the name kept for the rename (ufs_lookup's).
    if cnp.cn_nameiop == RENAME && lockparent && islast {
        cnp.cn_flags |= SAVENAME;
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
        let (mode, uid) = ATTR.with(core::cell::Cell::get);
        vap.va_type = vp.vtype();
        vap.va_size = 512;
        vap.va_mode = mode;
        vap.va_uid = uid;
        vap.va_gen = 7;
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
        p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        LAST_ACCESS_UID.with(|c| c.set(Some(p.ucred().cr_uid.get())));
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
        p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        VOPENS.fetch_add(1, Ordering::SeqCst);
        if let Some((e, dupfd)) = OPEN_ERROR.with(core::cell::Cell::take) {
            if let Some(fd) = dupfd {
                p.p_dupfd.set(fd);
            }
            return Err(e);
        }
        Ok(())
    }
    fn setattr(
        &self,
        _vp: &'static Vnode<TestGlue>,
        vap: &mut Vattr,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        SETATTRS.fetch_add(1, Ordering::SeqCst);
        LAST_SETATTR.with(|c| c.set(Some(*vap)));
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
    fn readlink(
        &self,
        vp: &'static Vnode<TestGlue>,
        uio: &mut Uio<'_, TestGlue>,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        let Some(Kind::Lnk(target)) = node_of(vp).map(|n| tree_kind(n.idx)) else {
            return Err(Errno::EINVAL);
        };
        let mut buf = target.to_vec();
        <TestFiles as crate::file::FileGlue>::uiomove(&mut buf, uio)
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
            if let Some(e) = STRATEGY_ERROR.with(core::cell::Cell::take) {
                t.buf().set(super::buf::B_ERROR);
                t.buf().set_error(Some(e));
            }
            super::bio::biodone(t);
        }
        Ok(())
    }
    /// `ufs_create`: a regular file named `cnp`'s name in `dvp`; the new vnode locked, `dvp`
    /// left as it came.
    fn create(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vpp: &mut Option<&'static Vnode<TestGlue>>,
        cnp: &mut Componentname<'_, TestGlue>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        if let Some(e) = CREATE_ERROR.with(core::cell::Cell::take) {
            return Err(e);
        }
        LAST_CREATE.with(|c| c.set(Some(*vap)));
        *vpp = Some(make_node(dvp, cnp.name(), Kind::Reg, None)?);
        Ok(())
    }
    /// `ufs_mknod`: a node of `vap`'s type, the attributes recorded ([`last_mknod`]); no
    /// vnode back (the C `vgone`s it for `VFS_VGET` to reload), `dvp` left as it came.
    fn mknod(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vpp: &mut Option<&'static Vnode<TestGlue>>,
        cnp: &mut Componentname<'_, TestGlue>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        LAST_MKNOD.with(|c| c.set(Some(*vap)));
        let kind = match vap.va_type {
            super::vnode::VCHR => Kind::Chr,
            super::vnode::VBLK => Kind::Blk,
            _ => Kind::Bad,
        };
        let (Some(dir), Some(_)) = (node_of(dvp).map(|n| n.idx), dvp.mount()) else {
            return Err(Errno::ENOENT);
        };
        tree_add(cnp.name(), dir, kind, None);
        *vpp = None;
        Ok(())
    }
    /// `ufs_mkdir`: a directory, its vnode locked; `dvp` released (`vput`), as the C's.
    fn mkdir(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vpp: &mut Option<&'static Vnode<TestGlue>>,
        cnp: &mut Componentname<'_, TestGlue>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        LAST_CREATE.with(|c| c.set(Some(*vap)));
        let r = make_node(dvp, cnp.name(), Kind::Dir, None);
        super::subr::vput(dvp);
        *vpp = Some(r?);
        Ok(())
    }
    /// `ufs_rmdir`: `ENOTEMPTY` for a directory with names in it; both vnodes released.
    fn rmdir(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        _cnp: &mut Componentname<'_, TestGlue>,
    ) -> Result<(), Errno> {
        let idx = node_of(vp).map(|n| n.idx);
        let error = match idx {
            Some(i) if !tree_children(i).is_empty() => Err(Errno::ENOTEMPTY),
            Some(i) => {
                tree_unlink(i);
                Ok(())
            }
            None => Err(Errno::ENOENT),
        };
        super::subr::vput(dvp);
        super::subr::vput(vp);
        error
    }
    /// `ufs_remove`: the name unlinked (`EPERM` for a directory); `vop_remove` releases.
    fn remove(
        &self,
        _dvp: &'static Vnode<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        _cnp: &mut Componentname<'_, TestGlue>,
    ) -> Result<(), Errno> {
        if vp.vtype() == VDIR {
            return Err(Errno::EPERM);
        }
        match node_of(vp) {
            Some(n) => {
                tree_unlink(n.idx);
                Ok(())
            }
            None => Err(Errno::ENOENT),
        }
    }
    /// `ufs_link`: a new name in `dvp` for `vp`'s kind; `dvp` released (`vput`).
    fn link(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        cnp: &mut Componentname<'_, TestGlue>,
    ) -> Result<(), Errno> {
        let kind = node_of(vp).map_or(Kind::Reg, |n| tree_kind(n.idx));
        let error = match node_of(dvp) {
            Some(d) => {
                tree_add(cnp.name(), d.idx, kind, None);
                Ok(())
            }
            None => Err(Errno::ENOENT),
        };
        super::subr::vput(dvp);
        error
    }
    /// `ufs_symlink`: a link to `target`; `dvp` and the new vnode released, as the C's.
    fn symlink(
        &self,
        dvp: &'static Vnode<TestGlue>,
        vpp: &mut Option<&'static Vnode<TestGlue>>,
        cnp: &mut Componentname<'_, TestGlue>,
        vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        LAST_CREATE.with(|c| c.set(Some(*vap)));
        let target: &'static [u8] = Box::leak(target.to_vec().into_boxed_slice());
        let r = make_node(dvp, cnp.name(), Kind::Lnk(target), None);
        super::subr::vput(dvp);
        let vp = r?;
        super::subr::vput(vp);
        *vpp = Some(vp);
        Ok(())
    }
    /// `ufs_rename`: `EXDEV` across mounts; the source's name moved to the target's (a target
    /// replaced); every vnode released, as the C's.
    fn rename(
        &self,
        fdvp: &'static Vnode<TestGlue>,
        fvp: &'static Vnode<TestGlue>,
        _fcnp: &mut Componentname<'_, TestGlue>,
        tdvp: &'static Vnode<TestGlue>,
        tvp: Option<&'static Vnode<TestGlue>>,
        tcnp: &mut Componentname<'_, TestGlue>,
    ) -> Result<(), Errno> {
        let error = if fvp.mount().map(Mount::addr) != tdvp.mount().map(Mount::addr) {
            Err(Errno::EXDEV)
        } else {
            match (node_of(fvp), node_of(tdvp)) {
                (Some(f), Some(td)) => {
                    if let Some(t) = tvp.and_then(node_of) {
                        tree_unlink(t.idx);
                    }
                    let name = tcnp.name().to_vec();
                    TREE.with(|t| {
                        if let Some(n) = t.borrow_mut().get_mut(f.idx) {
                            n.name = name;
                            n.parent = td.idx;
                        }
                    });
                    Ok(())
                }
                _ => Err(Errno::ENOENT),
            }
        };
        if let Some(t) = tvp {
            if core::ptr::eq(t, tdvp) {
                let _ = super::subr::vrele(t);
            } else {
                super::subr::vput(t);
            }
        }
        super::subr::vput(tdvp);
        let _ = super::subr::vrele(fdvp);
        let _ = super::subr::vrele(fvp);
        error
    }
    /// `ufs_readdir`: `struct dirent` records of `.`, `..` and the directory's linked names,
    /// each at its byte offset in the directory's stream, as many whole ones as the uio
    /// holds; `*eofflag` set past the last.
    fn readdir(
        &self,
        vp: &'static Vnode<TestGlue>,
        uio: &mut Uio<'_, TestGlue>,
        _cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        let Some(dir) = node_of(vp).map(|n| n.idx) else {
            return Err(Errno::ENOENT);
        };
        let parent = tree_node(dir).map_or(ROOT, |n| n.parent);
        let mut entries: Vec<(u64, u8, Vec<u8>)> = std::vec![
            (100 + dir as u64, hal::sys::dirent::DT_DIR, b".".to_vec()),
            (
                100 + parent as u64,
                hal::sys::dirent::DT_DIR,
                b"..".to_vec()
            ),
        ];
        for (i, n) in tree_children(dir) {
            let ty = match n.kind {
                Kind::Dir => hal::sys::dirent::DT_DIR,
                Kind::Reg => hal::sys::dirent::DT_REG,
                Kind::Sock => hal::sys::dirent::DT_SOCK,
                Kind::Chr => hal::sys::dirent::DT_CHR,
                Kind::Lnk(_) => hal::sys::dirent::DT_LNK,
                Kind::Blk => hal::sys::dirent::DT_BLK,
                Kind::Bad => hal::sys::dirent::DT_UNKNOWN,
            };
            entries.push((100 + i as u64, ty, n.name));
        }
        let mut at: i64 = 0;
        *eofflag = 0;
        for (fileno, ty, name) in entries {
            let reclen = hal::sys::dirent::dirent_recsize(name.len());
            let start = at;
            at += reclen as i64;
            if start < uio.uio_offset {
                continue;
            }
            if uio.uio_resid < reclen {
                return Ok(());
            }
            let mut rec = std::vec![0u8; reclen];
            rec[0..8].copy_from_slice(&fileno.to_ne_bytes());
            rec[8..16].copy_from_slice(&at.to_ne_bytes());
            rec[16..18].copy_from_slice(&(reclen as u16).to_ne_bytes());
            rec[18] = ty;
            rec[19] = name.len() as u8;
            let off = hal::sys::dirent::Dirent::NAME_OFFSET;
            rec[off..off + name.len()].copy_from_slice(&name);
            <TestFiles as crate::file::FileGlue>::uiomove(&mut rec, uio)?;
        }
        *eofflag = 1;
        Ok(())
    }
    /// `ufs_pathconf`'s answers for a few names (`_PC_LINK_MAX` 32767, `_PC_NAME_MAX` 255),
    /// `EINVAL` for the others.
    fn pathconf(
        &self,
        _vp: &'static Vnode<TestGlue>,
        name: i32,
        retval: &mut hal::sys::types::Register,
    ) -> Result<(), Errno> {
        match name {
            hal::sys::unistd::_PC_LINK_MAX => *retval = 32767,
            hal::sys::unistd::_PC_NAME_MAX => *retval = 255,
            _ => return Err(Errno::EINVAL),
        }
        Ok(())
    }
    /// `vop_generic_revoke`, counted ([`REVOKES`]): nothing to revoke here.
    fn revoke(&self, _vp: &'static Vnode<TestGlue>, _flags: i32) -> Result<(), Errno> {
        REVOKES.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// A new node named `name` in the directory `dvp` (locked), of `kind`: its vnode, referenced
/// and locked (`VFS_VGET` of the new inode).
fn make_node(
    dvp: &'static Vnode<TestGlue>,
    name: &[u8],
    kind: Kind,
    rdev: Option<Dev>,
) -> Result<&'static Vnode<TestGlue>, Errno> {
    let (Some(dir), Some(mp)) = (node_of(dvp).map(|n| n.idx), dvp.mount()) else {
        return Err(Errno::ENOENT);
    };
    let idx = tree_add(name, dir, kind, rdev);
    vget_node(mp, idx)
}

/// How many times `VOP_REVOKE` ran, on any test thread.
pub static REVOKES: AtomicUsize = AtomicUsize::new(0);
/// How many times `VFS_STATFS` ran, on any test thread.
pub static STATFSES: AtomicUsize = AtomicUsize::new(0);
/// How many times `VFS_SYNC` ran, on any test thread.
pub static SYNCS: AtomicUsize = AtomicUsize::new(0);
/// How many times `VFS_UNMOUNT` ran, on any test thread.
pub static UNMOUNTS: AtomicUsize = AtomicUsize::new(0);

/// The test file system's operations: the root and the inode lookup do anything.
pub struct TestFs;

/// The table.
pub static TEST_VFS: TestFs = TestFs;

impl Vfs<TestGlue> for TestFs {
    fn mount(
        &self,
        _mp: &'static Mount<TestGlue>,
        _path: &[u8],
        data: &mut [u8],
        _ndp: &mut Nd<'_, TestGlue>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        LAST_MOUNT.with(|m| *m.borrow_mut() = Some(data.to_vec()));
        MOUNT_ERROR.with(core::cell::Cell::get).map_or(Ok(()), Err)
    }
    fn start(
        &self,
        _mp: &'static Mount<TestGlue>,
        _flags: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    /// `ufs_unmount`'s `vflush` of the mount's vnodes (`FORCECLOSE` under `MNT_FORCE`):
    /// `EBUSY` while one is in use but for a forced unmount.
    fn unmount(
        &self,
        mp: &'static Mount<TestGlue>,
        f: i32,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        UNMOUNTS.fetch_add(1, Ordering::SeqCst);
        let flags = if f & hal::sys::mount::MNT_FORCE != 0 {
            super::vnode::FORCECLOSE
        } else {
            0
        };
        super::subr::vflush(mp, None, flags)
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
        STATFSES.fetch_add(1, Ordering::SeqCst);
        sbp.f_blocks = 100;
        sbp.f_bavail = 50;
        Ok(())
    }
    fn sync(
        &self,
        mp: &'static Mount<TestGlue>,
        _waitfor: i32,
        _stall: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        SYNCS.fetch_add(1, Ordering::SeqCst);
        LAST_SYNC_FLAGS.with(|c| c.set(Some(mp.flag())));
        Ok(())
    }
    fn vget(
        &self,
        mp: &'static Mount<TestGlue>,
        ino: Ino,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        vget_node(mp, ino as usize)
    }
    /// The node a handle names (`ESTALE` for one no longer linked), referenced and locked.
    fn fhtovp(
        &self,
        mp: &'static Mount<TestGlue>,
        fid: &Fid,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        let mut b = [0u8; 4];
        b.copy_from_slice(&fid.fid_data[..4]);
        let idx = u32::from_ne_bytes(b) as usize;
        if fid.fid_len != 4 || !tree_node(idx).is_some_and(|n| n.live || idx == ROOT) {
            return Err(Errno::ESTALE);
        }
        vget_node(mp, idx)
    }
    /// The handle of a node: its index in four bytes.
    fn vptofh(&self, vp: &'static Vnode<TestGlue>, fid: &mut Fid) -> Result<(), Errno> {
        let Some(n) = node_of(vp) else {
            return Err(Errno::ESTALE);
        };
        fid.fid_len = 4;
        fid.fid_data[..4].copy_from_slice(&(n.idx as u32).to_ne_bytes());
        Ok(())
    }
}

/// The configured types of the tests: the test file system alone.
pub static TEST_CONF: [Vfsconf<TestGlue>; 1] = [Vfsconf::new(
    &TEST_VFS,
    b"testfs",
    99,
    MNT_LOCAL,
    TEST_DATASIZE,
)];

/// The test file system's `vfc_datasize`: the bytes `mount(2)` copies in.
pub const TEST_DATASIZE: usize = 16;

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
/// system, the device switch; the old lists and the buffer cache forgotten. The guard of the
/// kernel's memory.
pub fn setup() -> MutexGuard<'static, ()> {
    setup_on(crate::testutil::pages_reset())
}

/// [`setup`] over the memory guard `g` the caller made.
fn setup_on(g: MutexGuard<'static, ()>) -> MutexGuard<'static, ()> {
    hal::klock::kernel_lock();
    vfs_test_reset::<TestGlue>();
    // The buffer cache is the process's, not the thread's: a cache test's buffers (and
    // `bcstats.numbufs`, which `getnewvnode` reads to raise `maxvnodes`) must not reach the
    // file system tests that run after it.
    super::bio::bio_test_reset();
    NODES.with(|n| n.borrow_mut().clear());
    DENY_ACCESS.with(|d| d.set(false));
    TREE.with(|t| {
        *t.borrow_mut() = (0..N)
            .map(|i| TreeNode {
                name: NAMES[i].0.to_vec(),
                parent: NAMES[i].1,
                kind: KINDS[i],
                rdev: RDEVS[i],
                live: true,
            })
            .collect();
    });
    LAST_SETATTR.with(|c| c.set(None));
    LAST_MKNOD.with(|c| c.set(None));
    ATTR.with(|a| a.set((0o755, 0)));
    LAST_MOUNT.with(|m| *m.borrow_mut() = None);
    MOUNT_ERROR.with(|m| m.set(None));
    LAST_CREATE.with(|c| c.set(None));
    OPEN_ERROR.with(|c| c.set(None));
    LAST_ACCESS_UID.with(|c| c.set(None));
    LAST_SYNC_FLAGS.with(|c| c.set(None));
    CREATE_ERROR.with(|c| c.set(None));
    STRATEGY_ERROR.with(|c| c.set(None));
    crate::testutil::unveil_done_reset();
    super::init::vfsinit::<TestGlue>(&TEST_CONF, 64, || {});
    super::lockf::lf_init();
    let _ = register_devsw::<TestGlue>(&TEST_DEVSW);
    g
}

/// [`setup`] plus the test file system mounted as root: on the mount list with its root
/// vnode, which is the thread's `rootvnode`, unlocked and referenced twice (the mount's
/// root and the caller's).
pub fn setup_root() -> (MutexGuard<'static, ()>, &'static Mount<TestGlue>) {
    root_on(setup())
}

/// [`setup_root`] over `bytes` of page memory and a buffer cache of `bufpages` pages
/// (`testutil::bufcache_install`'s), for the tests that read and write a file's blocks
/// through the cache: [`setup`]'s memory has no pages, so `getblk` would wait for buffer
/// space for ever.
pub fn setup_root_cached(
    bytes: usize,
    bufpages: i64,
) -> (MutexGuard<'static, ()>, &'static Mount<TestGlue>) {
    let g = setup_on(crate::testutil::pages_install(bytes));
    crate::testutil::bufcache_init(bytes, bufpages);
    root_on(g)
}

/// The test file system mounted as root over [`setup`]'s tables (`g` their guard).
fn root_on(g: MutexGuard<'static, ()>) -> (MutexGuard<'static, ()>, &'static Mount<TestGlue>) {
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
