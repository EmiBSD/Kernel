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
//! A UFS test double: the UFS layer of this module's parent run over a fake file system, so
//! the kernel's `ufs_lookup` and vnode operations are tested without `bsd`'s ffs (host tests
//! and the host benches only, `cfg(any(test, feature = "doubles"))`; not an OpenBSD file).
//!
//! [`TestUfs`] is the file system's half: its `InodeOps` (truncate, update, the inode's
//! allocation and free, a block's allocation and read) and its `UfsFs` (read, write, fsync),
//! over inodes of a pool of its own whose dinodes (FFS2) live in a per-thread table, and
//! blocks of [`BSIZE`] bytes (one `DIRBLKSIZ`, the mount's `f_iosize`) read and written
//! through the buffer cache. The device under the mount is a block device of testfs's device
//! switch (`bdevvp`) whose strategy, hooked here ([`disk_strategy`]), keeps each block's bytes
//! by its disk address. The mount's `vget` makes the inode of a number
//! from its table (`ffs_vget`'s steps: the inode hash, a new vnode of [`TEST_UFS_VOPS`], the
//! inode, its dinode, `ufs_ihashins`). [`setup`] mounts a fresh one with its root directory
//! (inode 2, `.` and `..`).

use core::cell::{Cell, RefCell};
use std::boxed::Box;
use std::collections::HashMap;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::intr::IPL_NONE;
use hal::pool::{BoxPool, PR_WAITOK, PoolArc, TypedPool};
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, MNT_LOCAL, Statfs};
use hal::sys::types::{Daddr, Ino, Mode, Off, Uid};

use super::dinode::{IFDIR, Ufs2Dinode, Ufsino};
use super::dir::DIRBLKSIZ;
use super::inode::{Dinode, Inode, InodeInit, InodeOps, Ufs2Din, vtoi};
use super::ufs_ihash::{ufs_ihashget, ufs_ihashinit, ufs_ihashins};
use super::ufs_vnops::{MASTERTEMPLATE, UfsFs, UfsSpecVnops, UfsVnops};
use super::ufsmount::{UM_UFS2, Ufsmount};
use crate::file::FileGlue;
use crate::fs::bio::{biodone, bread, brelse, bwrite, getblk};
use crate::fs::buf::{B_CLRBUF, B_READ, BufRef};
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc, vfs_unbusy};
use crate::fs::namei::{CREATE, Componentname, ISLASTCN, LOCKPARENT, LOOKUP};
use crate::fs::subr::{bdevvp, getnewvnode};
use crate::fs::testfs::{TestGlue, set_bdev_strategy, setup_root_cached, test_thread_cached};
use crate::fs::vnode::{VBLK, VCHR, VREG, VT_UFS, Vattr, Vnode, iftovt};
use crate::fs::vops::{Cred, IoBuf, Nd, Vfs, vop_create, vop_lookup};
use crate::proc::Thread;
use crate::uio::Uio;

/// The block size: one directory block, the mount's `f_iosize`.
pub const BSIZE: usize = DIRBLKSIZ;

/// The device number of the double's disk (`i_dev`): a disk of testfs's device switch.
pub const TEST_DEV: hal::sys::types::Dev = hal::sys::types::makedev(2, 14);

/// The file system's half of the double.
pub struct TestUfs;

/// The double's file system.
pub static TEST_UFS: TestUfs = TestUfs;

/// `ffs_vops` of the double: the kernel's UFS table over [`TestUfs`].
pub static TEST_UFS_VOPS: UfsVnops<TestGlue, TestUfs> = UfsVnops::new(&TEST_UFS);

/// `ffs_specvops` of the double.
pub static TEST_UFS_SPECVOPS: UfsSpecVnops<TestGlue, TestUfs> = UfsSpecVnops::new(&TEST_UFS);

/// The double's `vfsops`: `vget` and `root`, the rest `EOPNOTSUPP`.
struct TestUfsVfs;

/// The double's `vfsops`.
static TEST_UFS_VFS: TestUfsVfs = TestUfsVfs;

/// The double's configuration entry.
static TEST_UFS_CONF: Vfsconf<TestGlue> = Vfsconf::new(&TEST_UFS_VFS, b"testufs", 98, MNT_LOCAL, 0);

/// `ffs_ino_pool` and `ffs_dinode2_pool` of the double.
static INODES: TypedPool<Inode<TestGlue>> = TypedPool::new();
/// `ffs_dinode2_pool`.
static DINO2: BoxPool<Ufs2Din> = BoxPool::new();

/// The mount's device vnode and its UFS structure.
#[derive(Clone, Copy)]
struct Fsys {
    /// `um_devvp`.
    devvp: &'static Vnode<TestGlue>,
    /// The mount's UFS structure.
    ump: &'static Ufsmount<TestGlue>,
}

std::thread_local! {
    /// The disk's blocks by disk address.
    static DISK: RefCell<HashMap<Daddr, Vec<u8>>> = RefCell::new(HashMap::new());
    /// The inode table: each inode's dinode as last written (`UFS_UPDATE`).
    static DINODES: RefCell<HashMap<Ufsino, Ufs2Dinode>> = RefCell::new(HashMap::new());
    /// The next inode number and the next disk block `inode_alloc` and `buf_alloc` hand out.
    static NEXT: Cell<(Ufsino, Daddr)> = const { Cell::new((3, 101)) };
    /// The `UFS_UPDATE`s and the inodes freed (number, mode), in order.
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    /// The inodes `inode_free` gave back.
    static FREED: RefCell<Vec<(Ufsino, Mode)>> = const { RefCell::new(Vec::new()) };
    /// The error the next `update` returns.
    static UPDATE_ERROR: Cell<Option<Errno>> = const { Cell::new(None) };
    /// The mounted double.
    static FSYS: Cell<Option<Fsys>> = const { Cell::new(None) };
}

/// The mounted double.
fn fsys() -> Fsys {
    match FSYS.with(Cell::get) {
        Some(f) => f,
        None => std::panic!("testufs: not mounted"),
    }
}

/// The `UFS_UPDATE`s since [`setup`].
pub fn updates() -> usize {
    UPDATES.with(Cell::get)
}

/// The inodes freed since [`setup`] (number, mode).
pub fn freed() -> Vec<(Ufsino, Mode)> {
    FREED.with(|f| f.borrow().clone())
}

/// Makes the next `update` fail with `e`.
pub fn set_update_error(e: Errno) {
    UPDATE_ERROR.with(|c| c.set(Some(e)));
}

/// The dinode inode `ino` was last written with.
pub fn dinode(ino: Ufsino) -> Option<Ufs2Dinode> {
    DINODES.with(|d| d.borrow().get(&ino).copied())
}

/// A test thread (root's credentials) over fresh VFS tables with page memory and a buffer
/// cache, and the double mounted (not on the mount list) with its root directory, inode 2:
/// `.` and `..` in its one block. The guard of the kernel's memory, the thread, the mount.
pub fn setup() -> (
    MutexGuard<'static, ()>,
    &'static Thread<TestGlue>,
    &'static Mount<TestGlue>,
) {
    let (g, p) = test_thread_cached(1 << 21, 64);
    (g, p, mount())
}

/// [`setup`] for a caller that has made its test thread already (the host benches, whose
/// `procinit` ran): fresh VFS tables with page memory and a buffer cache, and the double
/// mounted. The guard of the kernel's memory and the mount.
pub fn setup_cached() -> (MutexGuard<'static, ()>, &'static Mount<TestGlue>) {
    let (g, _mp) = setup_root_cached(1 << 21, 64);
    (g, mount())
}

/// The double mounted over fresh tables: its pools, the inode hash, the disk, the root.
fn mount() -> &'static Mount<TestGlue> {
    // readdir's and readlink's uiomove copy into the tests' kernel buffers.
    crate::testutil::set_copy_kernel(true);
    let _ = INODES.init_recorded(IPL_NONE, PR_WAITOK, "ffsino");
    let _ = DINO2.init(IPL_NONE, PR_WAITOK, "dino2pl");
    ufs_ihashinit::<TestGlue>(64);
    DISK.with(|d| d.borrow_mut().clear());
    DINODES.with(|d| d.borrow_mut().clear());
    NEXT.with(|n| n.set((3, 101)));
    UPDATES.with(|u| u.set(0));
    FREED.with(|f| f.borrow_mut().clear());
    UPDATE_ERROR.with(|c| c.set(None));

    let devvp = match bdevvp::<TestGlue>(TEST_DEV) {
        Ok(Some(vp)) => vp,
        Ok(None) => std::panic!("testufs: no device vnode"),
        Err(e) => std::panic!("testufs: no device vnode: {e:?}"),
    };
    set_bdev_strategy(Some(disk_strategy));
    let mp = vfs_mount_alloc(None, &TEST_UFS_CONF);
    vfs_unbusy(mp);
    mp.with_stat(|st| {
        st.f_iosize = BSIZE as u32;
        st.f_bsize = BSIZE as u32;
    });
    let ump: &'static Ufsmount<TestGlue> = Box::leak(Box::new(Ufsmount::new()));
    ump.um_maxsymlinklen.set(120);
    FSYS.with(|f| f.set(Some(Fsys { devvp, ump })));

    // The root directory: "." and ".." of inode 2 in disk block 100.
    let mut t = MASTERTEMPLATE;
    t.dot_ino = 2;
    t.dotdot_ino = 2;
    let mut blk = std::vec![0u8; BSIZE];
    blk[..t.to_bytes().len()].copy_from_slice(&t.to_bytes());
    DISK.with(|d| d.borrow_mut().insert(100, blk));
    let mut root = Ufs2Dinode {
        di_mode: (IFDIR | 0o755) as u16,
        di_nlink: 2,
        di_size: BSIZE as u64,
        ..Ufs2Dinode::default()
    };
    root.di_db[0] = 100;
    DINODES.with(|d| d.borrow_mut().insert(2, root));
    mp
}

/// `ffs_vget` of the double: the vnode of inode `ino`, locked and referenced.
fn vget(mp: &'static Mount<TestGlue>, ino: Ufsino) -> Result<&'static Vnode<TestGlue>, Errno> {
    if let Some(vp) = ufs_ihashget::<TestGlue>(TEST_DEV, ino) {
        return Ok(vp);
    }
    let f = fsys();
    let vp = getnewvnode(VT_UFS, Some(mp), &TEST_UFS_VOPS)?;
    let Some(ip) = INODES.get_homed(PR_WAITOK, || {
        Inode::new(InodeInit {
            vnode: Some(vp),
            mount: Some(mp),
            ump: Some(f.ump),
            devvp: Some(f.devvp),
            dev: TEST_DEV,
            number: ino,
            fstype: UM_UFS2,
            nindir: (BSIZE / size_of::<i64>()) as u64,
            seqinc: 1,
            vtbl: Some(&TEST_UFS),
            ..InodeInit::default()
        })
    }) else {
        return Err(Errno::ENOMEM);
    };
    let d = dinode(ino).unwrap_or_default();
    let Some(din) = DINO2.get(PR_WAITOK, || Ufs2Din::new(&d)) else {
        return Err(Errno::ENOMEM);
    };
    ip.set_din(Dinode::Ufs2(din));
    ip.i_effnlink.set(i32::from(d.di_nlink));
    vp.set_node(PoolArc::erase(ip.clone()));
    let _ = ufs_ihashins(ip.as_pin());
    vp.set_vtype(iftovt(Mode::from(d.di_mode)));
    if vp.vtype() == VCHR || vp.vtype() == VBLK {
        vp.set_ops_id(crate::fs::subr::vops_id::<TestGlue>(&TEST_UFS_SPECVOPS));
    }
    Ok(vp)
}

impl Vfs<TestGlue> for TestUfsVfs {
    fn mount(
        &self,
        _: &'static Mount<TestGlue>,
        _: &[u8],
        _: &mut [u8],
        _: &mut Nd<'_, TestGlue>,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn start(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn unmount(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn root(&self, mp: &'static Mount<TestGlue>) -> Result<&'static Vnode<TestGlue>, Errno> {
        vget(mp, 2)
    }
    fn quotactl(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: Uid,
        _: usize,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn statfs(
        &self,
        _: &'static Mount<TestGlue>,
        _: &mut Statfs,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn sync(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: i32,
        _: Cred<'_>,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn vget(
        &self,
        mp: &'static Mount<TestGlue>,
        ino: Ino,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        vget(mp, ino as Ufsino)
    }
    fn fhtovp(
        &self,
        _: &'static Mount<TestGlue>,
        _: &Fid,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn vptofh(&self, _: &'static Vnode<TestGlue>, _: &mut Fid) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
}

/// The disk's `d_strategy`: the block's bytes in or out by disk address, then `biodone`.
fn disk_strategy(mut t: BufRef) {
    {
        let blkno = t.buf().blkno();
        let n = t.buf().bcount() as usize;
        if t.buf().isset(B_READ) {
            let data = t.data();
            DISK.with(|d| match d.borrow().get(&blkno) {
                Some(b) => data[..n].copy_from_slice(&b[..n]),
                None => data[..n].fill(0),
            });
        } else {
            let b = t.bytes()[..n].to_vec();
            DISK.with(|d| d.borrow_mut().insert(blkno, b));
        }
    }
    biodone(t);
}

/// The logical block of `off` and the offset in it.
fn blkof(off: Off) -> (Daddr, usize) {
    (off / BSIZE as Off, (off % BSIZE as Off) as usize)
}

impl InodeOps<TestGlue> for TestUfs {
    /// `ffs_truncate` (direct blocks only): the size, and the blocks past it unmapped.
    fn truncate(&self, ip: &Inode<TestGlue>, off: Off, _: i32, _: Cred<'_>) -> Result<(), Errno> {
        let keep = (off as usize).div_ceil(BSIZE);
        for lbn in keep..super::dinode::NDADDR {
            ip.dip_set_db(lbn, 0);
        }
        ip.dip_set_size(off as u64);
        Ok(())
    }
    /// `ffs_update`: the dinode into the inode table.
    fn update(&self, ip: &Inode<TestGlue>, _waitfor: i32) -> Result<(), Errno> {
        if let Some(e) = UPDATE_ERROR.with(Cell::take) {
            return Err(e);
        }
        UPDATES.with(|u| u.set(u.get() + 1));
        let d = ip.din2();
        DINODES.with(|t| t.borrow_mut().insert(ip.i_number, d));
        Ok(())
    }
    /// `ffs_inode_alloc`: the next inode number, a zeroed dinode of `mode`, its vnode locked.
    fn inode_alloc(
        &self,
        pip: &Inode<TestGlue>,
        mode: Mode,
        _: Cred<'_>,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        let (ino, blk) = NEXT.with(Cell::get);
        NEXT.with(|n| n.set((ino + 1, blk)));
        let d = Ufs2Dinode {
            di_mode: mode as u16,
            ..Ufs2Dinode::default()
        };
        DINODES.with(|t| t.borrow_mut().insert(ino, d));
        vget(pip.mount(), ino)
    }
    /// `ffs_inode_free`: recorded.
    fn inode_free(&self, _: &Inode<TestGlue>, ino: Ufsino, mode: Mode) -> Result<(), Errno> {
        FREED.with(|f| f.borrow_mut().push((ino, mode)));
        Ok(())
    }
    /// `ffs_balloc` (direct blocks only): a block of the file, allocated (and cleared with
    /// `B_CLRBUF`) when it has none, or read.
    fn buf_alloc(
        &self,
        ip: &Inode<TestGlue>,
        startoffset: Off,
        _size: i32,
        _: Cred<'_>,
        flags: i32,
    ) -> Result<IoBuf, Errno> {
        let (lbn, _) = blkof(startoffset);
        let vp = ip.itov();
        if ip.dip_db(lbn as usize) == 0 {
            let (ino, blk) = NEXT.with(Cell::get);
            NEXT.with(|n| n.set((ino, blk + 1)));
            ip.dip_set_db(lbn as usize, blk);
            let Some(mut t) = getblk(&vp.bufs, lbn, BSIZE as i32, 0, 0) else {
                return Err(Errno::EIO);
            };
            t.buf().set_blkno(blk);
            if flags & B_CLRBUF != 0 {
                t.clear();
            }
            return Ok(IoBuf::Token(t));
        }
        let (t, error) = bread(&vp.bufs, lbn, BSIZE as i32);
        if let Err(e) = error {
            brelse(t);
            return Err(e);
        }
        Ok(IoBuf::Token(t))
    }
    /// `ffs_bufatoff`: the block holding `offset`, read.
    fn bufatoff(&self, ip: &Inode<TestGlue>, offset: Off) -> Result<(IoBuf, usize), Errno> {
        let (lbn, on) = blkof(offset);
        let (t, error) = bread(&ip.itov().bufs, lbn, BSIZE as i32);
        if let Err(e) = error {
            brelse(t);
            return Err(e);
        }
        Ok((IoBuf::Token(t), on))
    }
}

impl UfsFs<TestGlue> for TestUfs {
    /// `ffs_read`: block by block through the cache, up to the file's size.
    fn read(
        &self,
        vp: &'static Vnode<TestGlue>,
        uio: &mut Uio<'_, TestGlue>,
        _: i32,
        _: Cred<'_>,
    ) -> Result<(), Errno> {
        let ip = vtoi(vp);
        while uio.uio_resid > 0 && (uio.uio_offset as u64) < ip.dip_size() {
            let (lbn, on) = blkof(uio.uio_offset);
            let left = (ip.dip_size() - uio.uio_offset as u64) as usize;
            let n = (BSIZE - on).min(uio.uio_resid).min(left);
            let (mut t, error) = bread(&vp.bufs, lbn, BSIZE as i32);
            if let Err(e) = error {
                brelse(t);
                return Err(e);
            }
            let r =
                <crate::fs::testfs::TestFiles as FileGlue>::uiomove(&mut t.data()[on..on + n], uio);
            brelse(t);
            r?;
        }
        Ok(())
    }
    /// `ffs_write`: block by block, each allocated or read, filled and written.
    fn write(
        &self,
        vp: &'static Vnode<TestGlue>,
        uio: &mut Uio<'_, TestGlue>,
        _: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        let ip = vtoi(vp);
        while uio.uio_resid > 0 {
            let (_, on) = blkof(uio.uio_offset);
            let n = (BSIZE - on).min(uio.uio_resid);
            let off = uio.uio_offset;
            let Some(mut t) = self
                .buf_alloc(&ip, off, BSIZE as i32, cred, B_CLRBUF)?
                .into_token()
            else {
                return Err(Errno::EIO);
            };
            let r =
                <crate::fs::testfs::TestFiles as FileGlue>::uiomove(&mut t.data()[on..on + n], uio);
            if (off as u64) + n as u64 > ip.dip_size() {
                ip.dip_set_size(off as u64 + n as u64);
            }
            bwrite(t)?;
            r?;
        }
        Ok(())
    }
    /// `ffs_fsync`: the blocks are written as they change.
    fn fsync(
        &self,
        _: &'static Vnode<TestGlue>,
        _: Cred<'_>,
        _: i32,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
}
/// A componentname of `p` for `op` with `flags` (and `ISLASTCN`) naming `name`, under the
/// credential `cred`, as `namei` leaves it for the last component.
pub fn cn<'a>(
    p: &'a Thread<TestGlue>,
    cred: &'a crate::cred::Ucred,
    op: u64,
    flags: u64,
    name: &'static [u8],
) -> Componentname<'a, TestGlue> {
    let mut c = Componentname::new(op, flags | ISLASTCN, p, Cred::real(cred));
    c.set_name_lit(name);
    c
}

/// A `struct vattr` of type `t` and mode `mode`, the rest `VNOVAL` (`vattr_null`).
pub fn vattr(t: crate::fs::vnode::Vtype, mode: Mode) -> Vattr {
    let mut va = Vattr::new();
    crate::fs::subr::vattr_null(&mut va);
    va.va_type = t;
    va.va_mode = mode;
    va
}

/// `open(name, O_CREAT)` in the locked directory `dvp` of the double: the new file's vnode,
/// locked and referenced; `dvp` stays locked.
pub fn create(
    p: &Thread<TestGlue>,
    cred: &crate::cred::Ucred,
    dvp: &'static Vnode<TestGlue>,
    name: &'static [u8],
) -> Result<&'static Vnode<TestGlue>, Errno> {
    let mut c = cn(p, cred, CREATE, LOCKPARENT, name);
    let mut vpp = None;
    match vop_lookup(dvp, &mut vpp, &mut c) {
        Err(Errno::EJUSTRETURN) => {}
        Ok(()) => return Err(Errno::EEXIST),
        Err(e) => return Err(e),
    }
    let mut va = vattr(VREG, 0o644);
    vop_create(dvp, &mut vpp, &mut c, &mut va)?;
    vpp.ok_or(Errno::ENOENT)
}

/// The vnode `name` names in the locked directory `dvp` (`LOOKUP`, `LOCKPARENT`): locked
/// and referenced (`dvp` itself, referenced again, for ".").
pub fn lookup(
    p: &Thread<TestGlue>,
    cred: &crate::cred::Ucred,
    dvp: &'static Vnode<TestGlue>,
    name: &'static [u8],
) -> Result<&'static Vnode<TestGlue>, Errno> {
    let mut c = cn(p, cred, LOOKUP, LOCKPARENT, name);
    let mut vpp = None;
    vop_lookup(dvp, &mut vpp, &mut c)?;
    vpp.ok_or(Errno::ENOENT)
}
/* </CODE> */
