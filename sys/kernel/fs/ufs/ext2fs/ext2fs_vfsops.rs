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

/*
 * Copyright (c) 1997 Manuel Bouyer.
 * Copyright (c) 1989, 1991, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *	notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *	notice, this list of conditions and the following disclaimer in the
 *	documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *	may be used to endorse or promote products derived from this software
 *	without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)ffs_vfsops.c	8.14 (Berkeley) 11/28/94
 * Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! The second extended file system's file-system-type operations: mounting
//! (`ext2fs_mountroot`, `ext2fs_mount`, `ext2fs_mountfs` with the super block checks of
//! `e2fs_sbcheck` and the in-core set-up of `e2fs_sbfill`), reloading, unmounting,
//! `statfs`, `sync`, the inode cache's `ext2fs_vget`, file handles, the write-back of the
//! super block and the group descriptors (`ext2fs_sbupdate`, `ext2fs_cgupdate`) and
//! `ext2fs_init`.
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - `ext2fs_vfsops` has no `vfs_sysctl` (the C has none either: the trait's default answers
//!   `EOPNOTSUPP`, as `VFS_SYSCTL`'s NULL slot does); `vfs_quotactl` is the UFS layer's
//!   (`option QUOTA`), as in C.
//! - The group descriptors are allocated zeroed (the C's `mallocarray` leaves them
//!   uninitialised past the last group); the inode's dinode is zeroed before `e2fs_iload`
//!   fills it, so the large-inode fields of a revision 0 file system (128-byte inodes) are
//!   zero, not left over (LZ's).
//! - [`ext2fs_mount`] looks the device name up in a `Nameidata` of its own (`ndinit` around
//!   the kernel copy of the name), as `ffs_mount` does; a mount that is not an update and has
//!   no arguments is `EINVAL` (the C reads through the NULL `args`; only in-kernel callers
//!   pass none). `disk_map` and `vfs_export` (with the export list, `um_export`) are `bsd`'s
//!   until N7 and N6, reached through the glue (`FileGlue::disk_map`, `FileGlue::vfs_export`;
//!   [`ext2fs_mountfs`] empties the recycled mount's export list with
//!   `FileGlue::ufs_export_init` where the C's `malloc(M_ZERO)` of the `ufsmount` starts it
//!   empty), as ffs's.
//! - [`ext2fs_reload`] releases the super block's buffer before `e2fs_sbfill` reads the
//!   descriptors again, and the old descriptors go with the new array; the C keeps the buffer
//!   busy and loses the descriptors. `ext2fs_reload_vnode` releases the buffer of a failed
//!   `bread`, which the C returns without (LZ's).
//! - [`ext2fs_vget`] takes its reference on the device vnode as soon as the inode points at
//!   the mount, before `ufs_ihashins`, as `ffs_vget` does (ffs_vfsops.c:1217-1218), instead of
//!   after `ext2fs_vinit` (ext2fs_vfsops.c:940): `i_devvp` is `i_ump->um_devvp` (inode.h:259),
//!   so `ext2fs_reclaim`'s `vrele(ip->i_devvp)` (ext2fs_vnops.c:1224-1225) runs for an inode
//!   whose `vget` failed half-way too (the `EEXIST` retry and a failed `bread` recycle it, a
//!   `VFIFO` without fifofs leaves it cached), and the C releases there a reference it never
//!   took (LZ's deviation, kept by the user's decision of 2026-11-05).
//! - `rootdev` and `swapdev` are the caller's ([`ext2fs_mountroot`]'s arguments: `bsd`'s
//!   `ROOTDEV` and the machine's `conf.c` until N7); `rootvp` and `swapdev_vp` are `bsd`'s
//!   `init_main.c` globals, reached through the glue (`FileGlue::rootvp`, `set_rootvp`,
//!   `set_swapdev_vp`, docs/SCAFFOLDING.md), as ffs's.
//! - The `struct ext2fs_reload_args`/`struct ext2fs_sync_args` callbacks of
//!   `vfs_mount_foreach_vnode` are closures over those structures.
//!
//! ## Redesign
//! - LZ's `EXT2FS_VFSOPS` (a `bsd` `Vfsops` of function pointers over `bsd`'s mount) ->
//!   `impl Vfs<G> for Ext2fsVfs<G>` and its constant [`Ext2fsVfs::VFS`], which `bsd`'s
//!   `vfsconflist[]` names (`&Ext2fsVfs::<BsdProcGlue>::VFS`); the functions are the C's,
//!   generic over the glue, in its order, each `Vfs` method calling its function (the
//!   `Ffs<G>` shape, named `Ext2fsVfs` since the on-disk `struct ext2fs` is `Ext2fs`; no `VTBL`: the C has no `inode_vtbl` for ext2fs).
//! - The in-core super block is the UFS mount's own `UfsSuper::Ext2` ([`Ext2Superblock`],
//!   `ext2fs.rs`), set once by [`ext2fs_mountfs`] and freed with the counted `Ufsmount`'s last
//!   handle: LZ's `malloc`ed `MExt2fs` cast to a `&'static` (an `unsafe` reference whose
//!   soundness rested on `vflush` reclaiming every inode before the unmount's `free`) and its
//!   link in the glue's `BsdUfsmountG` are gone; `e2fs_sbload`, `e2fs_cgload`, `e2fs_sbsave` and
//!   `e2fs_cgsave` copy between the buffers' tokens and its words, LZ's `e2fs_gd_area` (an
//!   `unsafe` slice over the descriptors' `malloc`) gone.
//! - LZ's `EXT2FS_INODE_POOL` and `EXT2FS_DINODE_POOL` statics (the second a `bsd` `Pool` of
//!   raw items) -> the glue's `UfsTables::ext2fs_ino` and `ext2fs_dino`, typed pools, made by
//!   [`ext2fs_init`] (the C's `pool_init` on each call; a second `init` answers `EBUSY`, kept
//!   as a no-op: the C has no `done` flag).
//! - [`ext2fs_vget`] gives the new vnode the kernel's table (`UfsTables::ext2fs_vops`) and
//!   its inode the UFS mount's handle; the dinode is an `ext2dinopl` item set once into the
//!   inode's `Dinode::Ext2` (`ext2fs_dinode.rs`), dying with the inode's last handle; LZ's
//!   `ptr::write` into a raw pool item and its lent-buffer slice are gone.
//! - LZ's `strlcpy_zero` -> libkern's `strlcpy` over a zeroed array (b2's `mname_copy`).
//! - [`e2fs_sbfill`] computes in the C's widths (32-bit unsigned `howmany` of the block
//!   count, the shift of `1024 << e2fs_log_fsize` as the machine's 32-bit shift), so a corrupt
//!   super block gives the C's geometry instead of a Rust overflow panic.
//! - Cost against the C: `VFSTOUFS` is a read of the mount's slot and one count pair per
//!   operation reached from the mount (`statfs`, `sync`, `vget`, `fhtovp`, `unmount`,
//!   `quotactl`), none on a read or a write; a member of the super block is one relaxed load,
//!   a descriptor four (ROADMAP N12's row); the walks of `ext2fs_sync` and `ext2fs_reload` are
//!   the C's over `vfs_mount_foreach_vnode`, `vtoi` a count pair per vnode as ffs's;
//!   `ext2fs_sync_vnode` takes the C's one `splbio` pair per vnode.

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::intr::{IPL_BIO, IPL_NONE, splraise};
use hal::malloc::MArc;
use hal::pool::{PR_WAITOK, PoolArc};
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FREAD, FWRITE};
use hal::sys::mount::{
    Fid, MNAMELEN, MNT_FORCE, MNT_LAZY, MNT_LOCAL, MNT_RDONLY, MNT_RELOAD, MNT_UPDATE, MNT_WAIT,
    MNT_WANTRDWR, Statfs, UfsArgs,
};
use hal::sys::types::{Daddr, Dev, Ino, Off, Uid, major};
use hal::user::UserPtr;

use super::ext2fs::{
    E2FS_ERRORS, E2FS_ISCLEAN, E2FS_MAGIC, E2FS_REV0, E2FS_REV1, EXT2_GD_SIZE,
    EXT2F_INCOMPAT_EXTENTS, EXT2F_INCOMPAT_RECOVER, EXT2F_INCOMPAT_SUPP, EXT2F_ROCOMPAT_HUGE_FILE,
    EXT2F_ROCOMPAT_LARGE_FILE, EXT2F_ROCOMPAT_SPARSE_SUPER, EXT2F_ROCOMPAT_SUPP,
    EXT4F_RO_INCOMPAT_SUPP, Ext2Superblock, Ext2fs, INCOMPAT, LOG_MINBSIZE, MAXMNTLEN, RO_COMPAT,
    SBLOCK, SBOFF, SBSIZE, cg_has_sb, e2fs_cgload, e2fs_cgsave, e2fs_sbload, e2fs_sbsave, fsbtodb,
    ino_to_fsba, ino_to_fsbo, nindir,
};
use super::ext2fs_alloc::next_gennumber;
use super::ext2fs_dinode::{
    EXT2_FIRSTINO, EXT2_MAXSYMLINKLEN, EXT2_ROOTINO, Ext2Inode, e2fs_iload,
};
use super::ext2fs_inode::ext2fs_setsize;
use super::ext2fs_subr::ext2fs_vinit;
use crate::assert::kpanic;
use crate::file::{DM_OPENBLCK, FileGlue};
use crate::fs::bio::{DEV_BSHIFT, bawrite, bread, brelse, bwrite, getblk_wait};
use crate::fs::lookup::{namei, ndinit};
use crate::fs::mount::{Mount, Vfsconf, mountlist_insert_tail, vfs_mount_free, vfs_unbusy};
use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
use crate::fs::spec::devsw;
use crate::fs::subr::{
    bdevvp, copy_statfs_info, getnewvnode, vcount, vflush, vfs, vfs_mount_foreach_vnode,
    vfs_mountedon, vfs_rootmountalloc, vget, vgonel, vinvalbuf, vput, vref, vrele,
};
use crate::fs::ufs::ffs::ffs_vfsops::mname_copy;
use crate::fs::ufs::ufs::dinode::Ufsino;
use crate::fs::ufs::ufs::dir::MAXNAMLEN;
use crate::fs::ufs::ufs::inode::{
    Dinode, IN_ACCESS, IN_CHANGE, IN_MODIFIED, IN_UPDATE, Inode, InodeInit, Ufid, vtoi,
};
use crate::fs::ufs::ufs::quota::ufs_quotactl;
use crate::fs::ufs::ufs::ufs_ihash::{ufs_ihashget, ufs_ihashins};
use crate::fs::ufs::ufs::ufs_vfsops::{ufs_init, ufs_root, ufs_start};
use crate::fs::ufs::ufs::ufsmount::{UM_EXT2FS, Ufsmount, UfsmountRef, vfstoufs};
use crate::fs::vnode::{
    FORCECLOSE, LK_EXCLUSIVE, LK_NOWAIT, LK_RETRY, V_SAVE, VBAD, VBLK, VNON, VT_EXT2FS, Vnode,
    WRITECLOSE, vn_lock,
};
use crate::fs::vops::{Cred, Nd, VFS_VGET, Vfs, vop_close, vop_fsync, vop_open, vop_unlock};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};
use crate::tc::gettime;
use crate::time::inittodr;

/// `DEV_BSIZE`.
const DEV_BSIZE: Off = 1 << DEV_BSHIFT;

/// The second extended file system of the glue `G`: `ext2fs_vfsops` ([`Ext2fsVfs::VFS`]). Its
/// vnode tables are the glue's `UfsTables::ext2fs_vops` and `ext2fs_specvops`
/// (`ext2fs_vnops.rs`); the C has no `inode_vtbl` for ext2fs.
pub struct Ext2fsVfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Ext2fsVfs<G> {
    /// `ext2fs_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"ext2fs"`).
    pub const VFS: Ext2fsVfs<G> = Ext2fsVfs(PhantomData);
}

/// A device name as `%s` prints it: its bytes, one character each.
struct Name<'a>(&'a [u8]);

impl core::fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &b in self.0 {
            core::fmt::Write::write_char(f, char::from(b))?;
        }
        Ok(())
    }
}

/// `struct ext2fs_sync_args`.
struct Ext2fsSyncArgs<'a, G: ProcGlue> {
    /// `allerror`.
    allerror: Result<(), Errno>,
    /// `waitfor`.
    waitfor: i32,
    /// `nlink0`.
    nlink0: i32,
    /// `inflight`.
    inflight: i32,
    /// `p`.
    p: &'a Thread<G>,
    /// `cred`.
    cred: Cred<'a>,
}

/// `ext2fs_init` (`vfs_init`): the inode and dinode pools, then the UFS layer. A second
/// call (a host test's) keeps the pools made first: their `init` answers `EBUSY`.
pub fn ext2fs_init<G: ProcGlue>(_vfsp: &'static Vfsconf<G>) -> Result<(), Errno> {
    let t = &vfs::<G>().ufs;
    let _ = t.ext2fs_ino.init_recorded(IPL_NONE, PR_WAITOK, "ext2inopl");
    let _ = t.ext2fs_dino.init(IPL_NONE, PR_WAITOK, "ext2dinopl");

    ufs_init::<G>(vfs::<G>().initialvnodes.load(Ordering::Relaxed))
}

/// `memset(fs->e2fs_fsmnt, 0, sizeof(fs->e2fs_fsmnt)); strlcpy(fs->e2fs_fsmnt, name, ..)`.
fn e2fs_set_fsmnt(fs: &Ext2Superblock, name: &[u8]) {
    let mut m = [0u8; MAXMNTLEN];
    libkern::strlcpy(&mut m, name);
    fs.set_fsmnt(&m);
}

/// `memset(fs->e2fs.e2fs_fsmnt, 0, ..); strlcpy(fs->e2fs.e2fs_fsmnt, name, ..)`: the "last
/// mounted on" name in the super block, kept by revision 1 file systems.
fn e2fs_set_sb_fsmnt(fs: &Ext2Superblock, name: &[u8]) {
    let mut m = [0u8; 64];
    libkern::strlcpy(&mut m, name);
    fs.write_bytes(offset_of!(Ext2fs, e2fs_fsmnt), &m);
}

/// `ext2fs_mountroot`: called by `main()` when ext2fs is going to be mounted as root: the
/// vnodes of `swapdev` and `rootdev` (into `swapdev_vp` and `rootvp`), the root mount of
/// `"ext2fs"`, the mount list, and the clock set from the super block's write time.
pub fn ext2fs_mountroot<G: ProcGlue>(rootdev: Dev, swapdev: Dev) -> Result<(), Errno> {
    G::with_curproc(|p| {
        let Some(p) = p else {
            kpanic(format_args!("ext2fs_mountroot: no curproc"));
        };

        // Get vnodes for swapdev and rootdev.
        <G::FileGlue as FileGlue>::set_swapdev_vp(None);
        let made = bdevvp::<G>(swapdev).and_then(|vp| {
            <G::FileGlue as FileGlue>::set_swapdev_vp(vp);
            bdevvp::<G>(rootdev)
        });
        let Ok(Some(rvp)) = made else {
            kpanic(format_args!("ext2fs_mountroot: can't setup bdevvp's"));
        };
        <G::FileGlue as FileGlue>::set_rootvp(Some(rvp));

        let mp = match vfs_rootmountalloc::<G>(b"ext2fs", b"root_device") {
            Ok(mp) => mp,
            Err(e) => {
                vrele(rvp);
                return Err(e);
            }
        };

        if let Err(e) = ext2fs_mountfs(rvp, mp, p) {
            vfs_unbusy(mp);
            vfs_mount_free(mp);
            vrele(rvp);
            return Err(e);
        }

        mountlist_insert_tail(mp);
        let ump = vfstoufs(mp);
        let fs = ump.um_e2fs();
        let (name, len) = mp.mntonname();
        e2fs_set_fsmnt(fs, &name[..len]);
        if fs.e2fs_rev() > E2FS_REV0 {
            e2fs_set_sb_fsmnt(fs, &name[..len]);
        }
        let mut st = mp.stat();
        let _ = ext2fs_statfs(mp, &mut st, p);
        mp.with_stat(|sp| *sp = st);
        vfs_unbusy(mp);
        inittodr(i64::from(fs.e2fs_wtime()));
        Ok(())
    })
}

/// `ext2fs_mount` (`vfs_mount`): mount system call. `data` is the kernel copy of the user's
/// `struct ufs_args` (empty for the C's NULL).
pub fn ext2fs_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let args = UfsArgs::from_bytes(data);
    let mut ump: Option<UfsmountRef<G>> = None;
    let mut fname = [0u8; MNAMELEN];
    let mut fspec = [0u8; MNAMELEN];

    // If updating, check whether changing from read-only to read/write; if there is no
    // device name, that's all we do.
    if mp.flag() & MNT_UPDATE != 0 {
        let u: &UfsmountRef<G> = ump.insert(vfstoufs(mp));
        let fs = u.um_e2fs();
        if fs.e2fs_ronly.get() == 0 && mp.flag() & MNT_RDONLY != 0 {
            let mut flags = WRITECLOSE;
            if mp.flag() & MNT_FORCE != 0 {
                flags |= FORCECLOSE;
            }
            let error = ext2fs_flushfiles(mp, flags, p);
            if error.is_ok()
                && ext2fs_cgupdate(u, MNT_WAIT).is_ok()
                && fs.e2fs_state() & E2FS_ERRORS == 0
            {
                fs.set_e2fs_state(E2FS_ISCLEAN);
                let _ = ext2fs_sbupdate(u, MNT_WAIT);
            }
            error?;
            fs.e2fs_ronly.set(1);
        }
        if mp.flag() & MNT_RELOAD != 0 {
            ext2fs_reload(mp, ndp.ni_cnd.cn_cred, p)?;
        }
        if fs.e2fs_ronly.get() != 0 && mp.flag() & MNT_WANTRDWR != 0 {
            fs.e2fs_ronly.set(0);
            if fs.e2fs_state() == E2FS_ISCLEAN {
                fs.set_e2fs_state(0);
            } else {
                fs.set_e2fs_state(E2FS_ERRORS);
            }
            fs.e2fs_fmod.set(1);
        }
        match &args {
            // Process export requests.
            Some(a) if a.fspec == 0 => {
                return <G::FileGlue as FileGlue>::vfs_export(mp, &a.export_info);
            }
            None => return Ok(()), // success
            Some(_) => {}
        }
    }

    // Not an update, or updating the name: look up the name and verify that it refers to a
    // sensible block device.
    let Some(a) = &args else {
        return Err(Errno::EINVAL);
    };
    UserPtr::<u8>::new(a.fspec).copyinstr(&mut fspec)?;

    if !<G::FileGlue as FileGlue>::disk_map(&fspec, &mut fname, DM_OPENBLCK) {
        fname = fspec;
    }

    let flen = fname.iter().position(|&c| c == 0).unwrap_or(MNAMELEN);
    let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(&fname[..flen]), p);
    namei(&mut nd)?;
    let Some(devvp) = nd.ni_vp else {
        return Err(Errno::ENOENT);
    };

    let error: Errno = 'error_devvp: {
        if devvp.vtype() != VBLK {
            break 'error_devvp Errno::ENOTBLK;
        }
        if major(devvp.rdev()) >= devsw::<G>().nblkdev() {
            break 'error_devvp Errno::ENXIO;
        }
        if mp.flag() & MNT_UPDATE == 0 {
            if let Err(e) = ext2fs_mountfs(devvp, mp, p) {
                break 'error_devvp e;
            }
        } else {
            let Some(u) = &ump else {
                kpanic(format_args!("ext2fs_mount: update without ufsmount"));
            };
            if !ptr::eq(devvp, u.devvp()) {
                break 'error_devvp Errno::EINVAL; // XXX needs translation
            }
            vrele(devvp);
        }
        let u = vfstoufs(mp);
        let fs = u.um_e2fs();

        e2fs_set_fsmnt(fs, path);
        if fs.e2fs_rev() > E2FS_REV0 {
            let (name, len) = mp.mntonname();
            e2fs_set_sb_fsmnt(fs, &name[..len]);
        }
        let fsmnt = fs.fsmnt();
        let mut onname = [0u8; MNAMELEN];
        let n = fsmnt.as_bytes().len().min(MNAMELEN);
        onname[..n].copy_from_slice(&fsmnt.as_bytes()[..n]);
        mp.with_stat(|sp| {
            sp.f_mntonname = onname;
            mname_copy(&mut sp.f_mntfromname, &fname);
            mname_copy(&mut sp.f_mntfromspec, &fspec);
            sp.mount_info.__align[..UfsArgs::SIZE].copy_from_slice(&data[..UfsArgs::SIZE]);
        });

        if fs.e2fs_fmod.get() != 0 {
            // XXX
            fs.e2fs_fmod.set(0);
            if fs.e2fs_state() == 0 {
                fs.set_e2fs_wtime(gettime() as u32);
            } else {
                let from = mp.stat().f_mntfromname;
                let n = from.iter().position(|&c| c == 0).unwrap_or(MNAMELEN);
                printf(format_args!(
                    "{}: file system not clean; please fsck(8)\n",
                    Name(&from[..n])
                ));
            }
            let _ = ext2fs_cgupdate(&u, MNT_WAIT);
        }

        return Ok(());
    };

    // error_devvp: error with devvp held.
    vrele(devvp);
    Err(error)
}

/// `ext2fs_reload_vnode`: steps 4 to 6 of [`ext2fs_reload`] for one vnode.
fn ext2fs_reload_vnode<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fs: &Ext2Superblock,
    p: &Thread<G>,
    cred: Cred<'_>,
    devvp: &'static Vnode<G>,
) -> Result<(), Errno> {
    // Step 4: invalidate all inactive vnodes.
    if vp.usecount() == 0 {
        vgonel(vp, Some(p));
        return Ok(());
    }

    // Step 5: invalidate all cached file data.
    if vget(vp, LK_EXCLUSIVE).is_err() {
        return Ok(());
    }

    if vinvalbuf(vp, 0, cred, Some(p), 0, INFSLP).is_err() {
        kpanic(format_args!("ext2fs_reload: dirty2"));
    }
    // Step 6: re-read inode data for all active vnodes.
    let ip = vtoi(vp);
    let (t, error) = bread(
        &devvp.bufs,
        fsbtodb(fs, Daddr::from(ino_to_fsba(fs, ip.i_number))),
        fs.e2fs_bsize.get(),
    );
    if let Err(e) = error {
        brelse(t);
        drop(ip);
        vput(vp);
        return Err(e);
    }
    let off = ino_to_fsbo(fs, ip.i_number) as usize * fs.dinode_size();
    e2fs_iload(fs, &t.bytes()[off..], ip.i_e2din());
    brelse(t);
    drop(ip);
    vput(vp);
    Ok(())
}

/// `ext2fs_maxfilesize`: the largest file the block pointers can map, and the disk addresses
/// can reach.
fn ext2fs_maxfilesize(fs: &Ext2Superblock) -> Off {
    let huge = fs.e2fs_features_rocompat() & EXT2F_ROCOMPAT_HUGE_FILE != 0;
    let bsize = Off::from(fs.e2fs_bsize.get());
    let b = bsize / 4;

    let sectors: u64 = if huge {
        (1 << 48) - 1
    } else {
        u64::from(u32::MAX)
    };
    let physically = (sectors << DEV_BSHIFT) as Off;
    let logically = (12 + b + b * b + b * b * b) * bsize;

    logically.min(physically)
}

/// `e2fs_sbfill`: compute the in-memory values of the super block just loaded into `fs`, and
/// load the group descriptors (a new array in place of the old one: `ext2fs_reload`'s).
pub(crate) fn e2fs_sbfill<G: ProcGlue>(
    devvp: &'static Vnode<G>,
    fs: &Ext2Superblock,
) -> Result<(), Errno> {
    // XXX assume hardware block size == 512
    let bpg = fs.e2fs_bpg();
    let blocks = fs.e2fs_bcount().wrapping_sub(fs.e2fs_first_dblock());
    fs.e2fs_ncg
        .set((blocks.wrapping_add(bpg.wrapping_sub(1)) / bpg) as i32);
    let log_bsize = fs.e2fs_log_bsize();
    fs.e2fs_fsbtodb.set(log_bsize as i32 + 1);
    fs.e2fs_bsize.set(1024 << log_bsize);
    fs.e2fs_bshift.set((LOG_MINBSIZE + log_bsize) as i32);
    fs.e2fs_fsize.set(1024i32.wrapping_shl(fs.e2fs_log_fsize()));

    fs.e2fs_qbmask.set(i64::from(fs.e2fs_bsize.get() - 1));
    fs.e2fs_bmask.set(!(fs.e2fs_qbmask.get() as i32));

    fs.e2fs_ipb
        .set(fs.e2fs_bsize.get() / fs.dinode_size() as i32);
    fs.e2fs_itpg
        .set((fs.e2fs_ipg() / fs.e2fs_ipb.get() as u32) as i32);

    // Re-read group descriptors from the disk.
    let bsize = fs.e2fs_bsize.get();
    let per_block = bsize as usize / EXT2_GD_SIZE;
    fs.e2fs_ngdb
        .set((fs.e2fs_ncg.get() as usize).div_ceil(per_block) as i32);
    let ngdb = fs.e2fs_ngdb.get() as usize;
    fs.gd_alloc(ngdb, bsize as usize);

    for i in 0..ngdb {
        let dblk = Daddr::from(i32::from(bsize <= 1024)) + i as Daddr + 1;
        let gdesc = i * per_block;

        let (t, error) = bread(&devvp.bufs, fsbtodb(fs, dblk), bsize);
        if let Err(e) = error {
            fs.gd_free();
            brelse(t);
            return Err(e);
        }

        e2fs_cgload(t.bytes(), fs, gdesc, bsize as usize);
        brelse(t);
    }

    if fs.e2fs_features_rocompat() & EXT2F_ROCOMPAT_LARGE_FILE == 0 || fs.e2fs_rev() == E2FS_REV0 {
        fs.e2fs_maxfilesize.set(Off::from(i32::MAX));
    } else {
        fs.e2fs_maxfilesize.set(ext2fs_maxfilesize(fs));
    }

    if fs.e2fs_features_incompat() & EXT2F_INCOMPAT_EXTENTS != 0 {
        fs.e2fs_maxfilesize.set(fs.e2fs_maxfilesize.get() * 4);
    }

    Ok(())
}

/// `ext2fs_reload`: reload all incore data for a filesystem (used after running fsck on the
/// root filesystem and finding things to fix). The filesystem must be mounted read-only.
///
/// Things to do to update the mount:
///  1) invalidate all cached meta-data.
///  2) re-read superblock from disk.
///  3) re-read summary information from disk.
///  4) invalidate all inactive vnodes.
///  5) invalidate all cached file data.
///  6) re-read inode data for all active vnodes.
pub fn ext2fs_reload<G: ProcGlue>(
    mountp: &'static Mount<G>,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if mountp.flag() & MNT_RDONLY == 0 {
        return Err(Errno::EINVAL);
    }
    // Step 1: invalidate all cached meta-data.
    let ump = vfstoufs(mountp);
    let devvp = ump.devvp();
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let error = vinvalbuf(devvp, 0, cred, Some(p), 0, INFSLP);
    let _ = vop_unlock(devvp);
    if error.is_err() {
        kpanic(format_args!("ext2fs_reload: dirty1"));
    }

    // Step 2: re-read superblock from disk.
    let (t, error) = bread(&devvp.bufs, SBOFF / DEV_BSIZE, SBSIZE as i32);
    if let Err(e) = error {
        brelse(t);
        return Err(e);
    }
    if let Err(e) = e2fs_sbcheck(t.bytes(), mountp.flag() & MNT_RDONLY != 0) {
        brelse(t);
        return Err(e);
    }

    // Copy in the new superblock, compute in-memory values and load group descriptors.
    let fs = ump.um_e2fs();
    e2fs_sbload(t.bytes(), fs);
    brelse(t);
    e2fs_sbfill(devvp, fs)?;

    vfs_mount_foreach_vnode(mountp, &mut |vp| {
        ext2fs_reload_vnode(vp, fs, p, cred, devvp)
    })
}

/// `ext2fs_mountfs`: common code for mount and mountroot.
pub fn ext2fs_mountfs<G: ProcGlue>(
    devvp: &'static Vnode<G>,
    mp: &'static Mount<G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let dev = devvp.rdev();
    let cred = Cred::real(p.ucred());
    // Disallow multiple mounts of the same device. Disallow mounting of a device that is
    // currently in use (except for root, which might share swap device for miniroot). Flush
    // out any old buffers remaining from a previous use.
    vfs_mountedon(devvp)?;
    if vcount(devvp) > 1 && !<G::FileGlue as FileGlue>::rootvp().is_some_and(|r| ptr::eq(r, devvp))
    {
        return Err(Errno::EBUSY);
    }
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let error = vinvalbuf(devvp, V_SAVE, cred, Some(p), 0, INFSLP);
    let _ = vop_unlock(devvp);
    error?;

    let ronly = mp.flag() & MNT_RDONLY != 0;
    let omode = if ronly { FREAD } else { FREAD | FWRITE };
    vop_open(devvp, omode, Cred::FSCRED, p)?;

    let mut bp = None;
    let mut ump: Option<UfsmountRef<G>> = None;

    let error: Errno = 'out: {
        // Read the superblock from disk.
        let (t, error) = bread(&devvp.bufs, SBOFF / DEV_BSIZE, SBSIZE as i32);
        let t = bp.insert(t);
        if let Err(e) = error {
            break 'out e;
        }
        if let Err(e) = e2fs_sbcheck(t.bytes(), ronly) {
            break 'out e;
        }

        let u: &UfsmountRef<G> = ump.insert(Ufsmount::alloc());
        // The zeroed structure's export list (the mount's glue member).
        <G::FileGlue as FileGlue>::ufs_export_init(mp);
        // ump->um_e2fs = malloc(sizeof(struct m_ext2fs), M_UFSMNT, M_WAITOK | M_ZERO).
        let fs = Ext2Superblock::new();

        // Copy in the superblock, compute in-memory values and load group descriptors.
        e2fs_sbload(t.bytes(), &fs);
        if let Err(e) = e2fs_sbfill(devvp, &fs) {
            break 'out e;
        }
        if let Some(t) = bp.take() {
            brelse(t);
        }
        fs.e2fs_ronly.set(i8::from(ronly));
        u.um_fstype.set(UM_EXT2FS);

        if !ronly {
            if fs.e2fs_state() == E2FS_ISCLEAN {
                fs.set_e2fs_state(0);
            } else {
                fs.set_e2fs_state(E2FS_ERRORS);
            }
            fs.e2fs_fmod.set(1);
        }

        u.set_um_e2fs(fs);
        let fs = u.um_e2fs();
        mp.set_node(MArc::erase(u.clone()));
        mp.with_stat(|sp| {
            sp.f_fsid.val[0] = dev;
            sp.f_fsid.val[1] = mp.vfc().vfc_typenum;
            sp.f_namemax = MAXNAMLEN as u32;
        });
        mp.set_flag(mp.flag() | MNT_LOCAL);
        u.um_mountp.set(Some(mp));
        u.um_dev.set(dev);
        u.um_devvp.set(Some(devvp));
        u.um_nindir.set(nindir(fs) as u64);
        u.um_bptrtodb.set(fs.e2fs_fsbtodb.get() as u64);
        u.um_seqinc.set(1); // no frags
        u.um_maxsymlinklen.set(EXT2_MAXSYMLINKLEN as u32);
        devvp.set_specmountpoint(Some(mp));
        return Ok(());
    };

    // out:
    if devvp.has_specinfo() {
        devvp.set_specmountpoint(None);
    }
    if let Some(t) = bp {
        brelse(t);
    }
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vop_close(devvp, omode, cred, Some(p));
    let _ = vop_unlock(devvp);
    if ump.is_some() {
        // `free(ump->um_e2fs); free(ump); mp->mnt_data = NULL`: the last handle, `ump`, frees
        // the structure and the super block in it.
        drop(mp.take_node());
    }
    drop(ump);
    Err(error)
}

/// `ext2fs_unmount` (`vfs_unmount`): unmount system call.
pub fn ext2fs_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut flags = 0;
    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }
    ext2fs_flushfiles(mp, flags, p)?;
    let ump = vfstoufs(mp);
    let fs = ump.um_e2fs();

    if fs.e2fs_ronly.get() == 0
        && ext2fs_cgupdate(&ump, MNT_WAIT).is_ok()
        && fs.e2fs_state() & E2FS_ERRORS == 0
    {
        fs.set_e2fs_state(E2FS_ISCLEAN);
        let _ = ext2fs_sbupdate(&ump, MNT_WAIT);
    }

    let devvp = ump.devvp();
    if devvp.vtype() != VBAD {
        devvp.set_specmountpoint(None);
    }
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let omode = if fs.e2fs_ronly.get() != 0 {
        FREAD
    } else {
        FREAD | FWRITE
    };
    let _ = vop_close(devvp, omode, Cred::NOCRED, Some(p));
    vput(devvp);
    // free(fs->e2fs_gd); free(fs); free(ump); mp->mnt_data = NULL: the mount's handle goes;
    // the inodes' went with `vflush`, so this call's, dropped on return, is the last and frees
    // the structure with its super block and descriptors.
    drop(mp.take_node());
    mp.set_flag(mp.flag() & !MNT_LOCAL);
    Ok(())
}

/// `ext2fs_flushfiles`: flush out all the files in a filesystem.
pub fn ext2fs_flushfiles<G: ProcGlue>(
    mp: &'static Mount<G>,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = vfstoufs(mp);
    // Flush all the files.
    vflush(mp, None, flags)?;
    // Flush filesystem metadata.
    let devvp = ump.devvp();
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let error = vop_fsync(devvp, Cred::real(p.ucred()), MNT_WAIT, p);
    let _ = vop_unlock(devvp);
    error
}

/// `ext2fs_statfs` (`vfs_statfs`): get file system statistics.
pub fn ext2fs_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = vfstoufs(mp);
    let fs = ump.um_e2fs();
    if fs.e2fs_magic() != E2FS_MAGIC {
        kpanic(format_args!("ext2fs_statfs"));
    }

    // Compute the overhead (FS structures)
    let ncg = fs.e2fs_ncg.get();
    let overhead_per_group: u32 = 1 /* block bitmap */ + 1 /* inode bitmap */
        + fs.e2fs_itpg.get() as u32;
    let mut overhead = fs
        .e2fs_first_dblock()
        .wrapping_add((ncg as u32).wrapping_mul(overhead_per_group));
    let ngroups = if fs.e2fs_rev() > E2FS_REV0
        && fs.e2fs_features_rocompat() & EXT2F_ROCOMPAT_SPARSE_SUPER != 0
    {
        (0..ncg).filter(|&i| cg_has_sb(i)).count() as u32
    } else {
        ncg as u32
    };
    overhead = overhead.wrapping_add(ngroups.wrapping_mul(1 + fs.e2fs_ngdb.get() as u32));

    sbp.f_bsize = fs.e2fs_bsize.get() as u32;
    sbp.f_iosize = fs.e2fs_bsize.get() as u32;
    sbp.f_blocks = u64::from(fs.e2fs_bcount().wrapping_sub(overhead));
    sbp.f_bfree = u64::from(fs.e2fs_fbcount());
    sbp.f_bavail = sbp.f_bfree as i64 - i64::from(fs.e2fs_rbcount());
    sbp.f_files = u64::from(fs.e2fs_icount());
    sbp.f_ffree = u64::from(fs.e2fs_ficount());
    sbp.f_favail = i64::from(fs.e2fs_ficount());
    copy_statfs_info(sbp, mp);

    Ok(())
}

/// `ext2fs_sync_vnode`: write back one (modified) inode for [`ext2fs_sync`] (the C's callback
/// always answers 0).
fn ext2fs_sync_vnode<G: ProcGlue>(vp: &'static Vnode<G>, esa: &mut Ext2fsSyncArgs<'_, G>) {
    if vp.vtype() == VNON {
        return;
    }

    let ip = vtoi(vp);
    let mut nlink0 = 0;

    if ip.i_e2din().i_e2fs_nlink() == 0 {
        nlink0 = 1;
    }

    let s = splraise(IPL_BIO);
    let skip = ip.i_flag.get() & (IN_ACCESS | IN_CHANGE | IN_MODIFIED | IN_UPDATE) == 0
        && vp.bufs.dirty_is_empty();
    drop(s);
    drop(ip);

    'end: {
        if skip {
            break 'end;
        }

        if vget(vp, LK_EXCLUSIVE | LK_NOWAIT).is_err() {
            esa.inflight = (esa.inflight + 1).min(65536);
            break 'end;
        }

        if let Err(e) = vop_fsync(vp, esa.cred, esa.waitfor, esa.p) {
            esa.allerror = Err(e);
        }
        vput(vp);
    }
    // end:
    esa.nlink0 = (esa.nlink0 + nlink0).min(65536);
}

/// `ext2fs_sync` (`vfs_sync`): go through the disk queues to initiate sandbagged IO; go
/// through the inodes to write those that have been modified; initiate the writing of the
/// super block if it has been modified.
///
/// Should always be called with the mount point locked.
pub fn ext2fs_sync<G: ProcGlue>(
    mp: &'static Mount<G>,
    waitfor: i32,
    stall: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = vfstoufs(mp);
    let fs = ump.um_e2fs();
    let mut allerror = Ok(());
    if fs.e2fs_ronly.get() != 0 {
        // XXX
        printf(format_args!("fs = {}\n", fs.fsmnt()));
        kpanic(format_args!("update: rofs mod"));
    }

    // Write back each (modified) inode.
    let mut esa = Ext2fsSyncArgs {
        allerror: Ok(()),
        waitfor,
        nlink0: 0,
        inflight: 0,
        p,
        cred,
    };

    let _ = vfs_mount_foreach_vnode(mp, &mut |vp| {
        ext2fs_sync_vnode(vp, &mut esa);
        Ok(())
    });
    if esa.allerror.is_err() {
        allerror = esa.allerror;
    }

    // Force stale file system control information to be flushed.
    if waitfor != MNT_LAZY {
        let devvp = ump.devvp();
        let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
        if let Err(e) = vop_fsync(devvp, cred, waitfor, p) {
            allerror = Err(e);
        }
        let _ = vop_unlock(devvp);
    }
    // Write back modified superblock.
    let state = fs.e2fs_state();
    let fmod = fs.e2fs_fmod.get();
    if stall != 0 && fs.e2fs_ronly.get() == 0 {
        fs.e2fs_fmod.set(1);
        if allerror.is_ok() && esa.nlink0 == 0 && esa.inflight == 0 {
            if fs.e2fs_state() & E2FS_ERRORS == 0 {
                fs.set_e2fs_state(E2FS_ISCLEAN);
            }
        } else {
            fs.set_e2fs_state(0);
        }
    }
    if fs.e2fs_fmod.get() != 0 {
        fs.e2fs_fmod.set(0);
        fs.set_e2fs_wtime(gettime() as u32);
        if let Err(e) = ext2fs_cgupdate(&ump, waitfor) {
            allerror = Err(e);
        }
    }
    fs.set_e2fs_state(state);
    fs.e2fs_fmod.set(fmod);
    allerror
}

/// `ext2fs_vget` (`vfs_vget`): look up an EXT2FS dinode number to find its incore vnode,
/// otherwise read it in from disk. If it is in core, wait for the lock bit to clear, then
/// return the inode locked. Detection and handling of mount points must be done by the
/// calling routine.
pub fn ext2fs_vget<G: ProcGlue>(
    mp: &'static Mount<G>,
    ino: Ino,
) -> Result<&'static Vnode<G>, Errno> {
    if ino > u64::from(Ufsino::MAX) {
        kpanic(format_args!("ext2fs_vget: alien ino_t {ino}"));
    }
    let ino = ino as Ufsino;

    let ump = vfstoufs(mp);
    let dev = ump.um_dev.get();
    let tables = &vfs::<G>().ufs;
    loop {
        // retry:
        if let Some(vp) = ufs_ihashget::<G>(dev, ino) {
            return Ok(vp);
        }

        // Allocate a new vnode/inode.
        let vp = getnewvnode(VT_EXT2FS, Some(mp), &tables.ext2fs_vops)?;

        let fs = ump.um_e2fs();
        let Some(ip) = tables.ext2fs_ino.get_homed(PR_WAITOK, || {
            Inode::new(InodeInit {
                vnode: Some(vp),
                mount: Some(mp),
                ump: Some(ump.clone()),
                devvp: Some(ump.devvp()),
                dev,
                number: ino,
                fstype: ump.um_fstype.get(),
                nindir: ump.um_nindir.get(),
                bptrtodb: ump.um_bptrtodb.get(),
                seqinc: ump.um_seqinc.get(),
                ..InodeInit::default()
            })
        }) else {
            kpanic(format_args!("ext2fs_vget: no inode"));
        };
        // The C takes this reference after ext2fs_vinit (the module's deviations).
        vref(ump.devvp());
        vp.set_node(PoolArc::erase(ip.clone()));

        // Put it onto its hash chain and lock it so that other requests for this inode will
        // block if they arrive while we are sleeping waiting for old data structures to be
        // purged or for the contents of the disk portion of this inode to be read.
        if let Err(error) = ufs_ihashins(ip.as_pin()) {
            drop(ip);
            vrele(vp);

            if error == Errno::EEXIST {
                continue;
            }

            return Err(error);
        }

        // Read in the disk contents for the inode, copy into the inode.
        let (t, error) = bread(
            &ump.devvp().bufs,
            fsbtodb(fs, Daddr::from(ino_to_fsba(fs, ino))),
            fs.e2fs_bsize.get(),
        );
        if let Err(e) = error {
            // The inode does not contain anything useful, so it would be misleading to leave
            // it on its hash chain. With mode still zero, it will be unlinked and returned to
            // the free list by vput().
            drop(ip);
            vput(vp);
            brelse(t);
            return Err(e);
        }

        let Some(din) = tables.ext2fs_dino.get(PR_WAITOK, Ext2Inode::new) else {
            kpanic(format_args!("ext2fs_vget: no dinode"));
        };
        let off = fs.dinode_size() * ino_to_fsbo(fs, ino) as usize;
        e2fs_iload(fs, &t.bytes()[off..], &din);
        ip.set_din(Dinode::Ext2(din));
        brelse(t);
        let d = ip.i_e2din();

        ip.i_effnlink.set(i32::from(d.i_e2fs_nlink()));

        // The fields for storing the UID and GID of an ext2fs inode are limited to 16 bits.
        // To overcome this limitation, Linux decided to scatter the highest bits of these
        // values into a previously reserved area on the disk inode. We deal with this
        // situation by having two 32-bit fields *out* of the disk inode to hold the complete
        // values. Now that we are reading in the inode, compute these fields.
        d.i_e2fs_uid
            .set(u32::from(d.i_e2fs_uid_low()) | (u32::from(d.i_e2fs_uid_high()) << 16));
        d.i_e2fs_gid
            .set(u32::from(d.i_e2fs_gid_low()) | (u32::from(d.i_e2fs_gid_high()) << 16));

        // If the inode was deleted, reset all fields
        if d.i_e2fs_dtime() != 0 {
            d.set_i_e2fs_mode(0);
            d.set_i_e2fs_nblock(0);
            let _ = ext2fs_setsize(&ip, 0);
        }

        // Initialize the vnode from the inode, check for aliases. Note that the underlying
        // vnode may have changed.
        let vp = match ext2fs_vinit(mp, vp) {
            Ok(vp) => vp,
            Err(e) => {
                drop(ip);
                vput(vp);
                return Err(e);
            }
        };

        // Set up a generation number for this inode if it does not already have one. This
        // should only happen on old filesystems.
        let d = ip.i_e2din();
        if d.i_e2fs_gen() == 0 {
            d.set_i_e2fs_gen(next_gennumber() as u32);
            if vp.mount().is_some_and(|m| m.flag() & MNT_RDONLY == 0) {
                ip.set_flag(IN_MODIFIED);
            }
        }

        return Ok(vp);
    }
}

/// `ext2fs_fhtovp` (`vfs_fhtovp`): file handle to vnode.
///
/// Have to be really careful about stale file handles:
/// - check that the inode number is valid
/// - call `ext2fs_vget()` to get the locked inode
/// - check for an unallocated inode (`i_mode == 0`)
/// - check that the given client host has export rights and return those rights via
///   `exflagsp` and `credanonp`
pub fn ext2fs_fhtovp<G: ProcGlue>(
    mp: &'static Mount<G>,
    fhp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    let ufhp = Ufid::from_fid(fhp);
    {
        let ump = vfstoufs(mp);
        let fs = ump.um_e2fs();
        if (ufhp.ufid_ino < EXT2_FIRSTINO && ufhp.ufid_ino != EXT2_ROOTINO)
            || ufhp.ufid_ino > (fs.e2fs_ncg.get() as u32).wrapping_mul(fs.e2fs_ipg())
        {
            return Err(Errno::ESTALE);
        }
    }

    let nvp = VFS_VGET(mp, u64::from(ufhp.ufid_ino))?;
    let ip = vtoi(nvp);
    let d = ip.i_e2din();
    if d.i_e2fs_mode() == 0 || d.i_e2fs_dtime() != 0 || d.i_e2fs_gen() != ufhp.ufid_gen {
        drop(ip);
        vput(nvp);
        return Err(Errno::ESTALE);
    }
    Ok(nvp)
}

/// `ext2fs_vptofh` (`vfs_vptofh`): vnode pointer to file handle.
pub fn ext2fs_vptofh<G: ProcGlue>(vp: &'static Vnode<G>, fhp: &mut Fid) -> Result<(), Errno> {
    let ip = vtoi(vp);
    Ufid {
        ufid_len: size_of::<Ufid>() as u16,
        ufid_pad: fhp.fid_reserved,
        ufid_ino: ip.i_number,
        ufid_gen: ip.i_e2din().i_e2fs_gen(),
    }
    .to_fid(fhp);
    Ok(())
}

/// `ext2fs_sbupdate`: write a superblock back to disk.
pub fn ext2fs_sbupdate<G: ProcGlue>(mp: &Ufsmount<G>, waitfor: i32) -> Result<(), Errno> {
    let fs = mp.um_e2fs();

    let mut t = getblk_wait(&mp.devvp().bufs, SBLOCK, SBSIZE as i32);
    e2fs_sbsave(fs, t.data());
    let error = if waitfor == MNT_WAIT {
        bwrite(t)
    } else {
        bawrite(t);
        Ok(())
    };
    fs.e2fs_fmod.set(0);
    error
}

/// `ext2fs_cgupdate`: write the superblock and the group descriptors back to disk.
pub fn ext2fs_cgupdate<G: ProcGlue>(mp: &Ufsmount<G>, waitfor: i32) -> Result<(), Errno> {
    let fs = mp.um_e2fs();
    let mut error = Ok(());

    let allerror = ext2fs_sbupdate(mp, waitfor);
    let bsize = fs.e2fs_bsize.get();
    let per_block = bsize as usize / EXT2_GD_SIZE;
    for i in 0..fs.e2fs_ngdb.get().max(0) as usize {
        let dblk = Daddr::from(i32::from(bsize <= 1024)) + i as Daddr + 1;
        let mut t = getblk_wait(&mp.devvp().bufs, fsbtodb(fs, dblk), bsize);
        e2fs_cgsave(fs, i * per_block, t.data(), bsize as usize);
        if waitfor == MNT_WAIT {
            error = bwrite(t);
        } else {
            bawrite(t);
        }
    }

    if allerror.is_ok() && error.is_err() {
        return error;
    }
    allerror
}

/// `letoh16` of the super block member at `off` of the buffer's bytes.
fn sb_le16(sb: &[u8], off: usize) -> u32 {
    u32::from(u16::from_le_bytes([sb[off], sb[off + 1]]))
}

/// `letoh32` of the super block member at `off` of the buffer's bytes.
fn sb_le32(sb: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([sb[off], sb[off + 1], sb[off + 2], sb[off + 3]])
}

/// `e2fs_sbcheck`: whether the super block in `sb` (the buffer's bytes, before the copy:
/// watch out for endianness!) can be mounted, read-only if `ronly`.
pub(crate) fn e2fs_sbcheck(sb: &[u8], ronly: bool) -> Result<(), Errno> {
    if sb.len() < SBSIZE {
        kpanic(format_args!("e2fs_sbcheck: {} bytes", sb.len()));
    }
    let tmp = sb_le16(sb, offset_of!(Ext2fs, e2fs_magic));
    if tmp != u32::from(E2FS_MAGIC) {
        printf(format_args!("ext2fs: wrong magic number 0x{tmp:x}\n"));
        return Err(Errno::EIO); // XXX needs translation
    }

    let mut tmp = sb_le32(sb, offset_of!(Ext2fs, e2fs_log_bsize));
    if tmp > 2 {
        // skewed log(block size): 1024 -> 0 | 2048 -> 1 | 4096 -> 2
        tmp = tmp.wrapping_add(10);
        printf(format_args!(
            "ext2fs: wrong log2(block size) {}\n",
            tmp as i32
        ));
        return Err(Errno::EIO); // XXX needs translation
    }

    if sb_le32(sb, offset_of!(Ext2fs, e2fs_bpg)) == 0 {
        printf(format_args!("ext2fs: zero blocks per group\n"));
        return Err(Errno::EIO);
    }

    let tmp = sb_le32(sb, offset_of!(Ext2fs, e2fs_rev));
    if tmp > E2FS_REV1 {
        printf(format_args!("ext2fs: wrong revision number 0x{tmp:x}\n"));
        return Err(Errno::EIO); // XXX needs translation
    } else if tmp == E2FS_REV0 {
        return Ok(());
    }

    let tmp = sb_le32(sb, offset_of!(Ext2fs, e2fs_first_ino));
    if tmp != EXT2_FIRSTINO {
        printf(format_args!("ext2fs: first inode at 0x{tmp:x}\n"));
        return Err(Errno::EINVAL); // XXX needs translation
    }

    let tmp = sb_le32(sb, offset_of!(Ext2fs, e2fs_features_incompat));
    let mask = tmp & !(EXT2F_INCOMPAT_SUPP | EXT4F_RO_INCOMPAT_SUPP);
    if mask != 0 {
        printf(format_args!("ext2fs: unsupported incompat features: "));
        for f in INCOMPAT.iter().filter(|f| mask & f.mask != 0) {
            printf(format_args!("{} ", f.name));
        }
        printf(format_args!("\n"));
        return Err(Errno::EINVAL); // XXX needs translation
    }

    if !ronly && tmp & EXT4F_RO_INCOMPAT_SUPP != 0 {
        printf(format_args!("ext4fs: only read-only support right now\n"));
        return Err(Errno::EROFS); // XXX needs translation
    }

    if tmp & EXT2F_INCOMPAT_RECOVER != 0 {
        printf(format_args!(
            "ext2fs: your file system says it needs recovery\n"
        ));
        if !ronly {
            return Err(Errno::EROFS); // XXX needs translation
        }
    }

    let tmp = sb_le32(sb, offset_of!(Ext2fs, e2fs_features_rocompat)) & !EXT2F_ROCOMPAT_SUPP;
    if !ronly && tmp != 0 {
        printf(format_args!("ext2fs: unsupported R/O compat features: "));
        for f in RO_COMPAT.iter().filter(|f| tmp & f.mask != 0) {
            printf(format_args!("{} ", f.name));
        }
        printf(format_args!("\n"));
        return Err(Errno::EROFS); // XXX needs translation
    }

    Ok(())
}

/// `ext2fs_vfsops`: each slot the C's function of the same name; `vfs_sysctl` is NULL
/// (the trait's default, `EOPNOTSUPP`).
impl<G: ProcGlue> Vfs<G> for Ext2fsVfs<G> {
    /// `vfs_mount`: [`ext2fs_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ext2fs_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: `ufs_start`.
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno> {
        ufs_start(mp, flags, p)
    }

    /// `vfs_unmount`: [`ext2fs_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        ext2fs_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: `ufs_root`.
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        ufs_root(mp)
    }

    /// `vfs_quotactl`: `ufs_quotactl`.
    fn quotactl(
        &self,
        mp: &'static Mount<G>,
        cmds: i32,
        uid: Uid,
        arg: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ufs_quotactl(mp, cmds, uid, arg, p)
    }

    /// `vfs_statfs`: [`ext2fs_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        ext2fs_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`ext2fs_sync`].
    fn sync(
        &self,
        mp: &'static Mount<G>,
        waitfor: i32,
        stall: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ext2fs_sync(mp, waitfor, stall, cred, p)
    }

    /// `vfs_vget`: [`ext2fs_vget`].
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        ext2fs_vget(mp, ino)
    }

    /// `vfs_fhtovp`: [`ext2fs_fhtovp`].
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        ext2fs_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: [`ext2fs_vptofh`].
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        ext2fs_vptofh(vp, fid)
    }

    /// `vfs_init`: [`ext2fs_init`].
    fn init(&self, vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        ext2fs_init(vfc)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests over testext2fs's image (a block device of testfs whose strategy serves the
    // image), mounted by `ext2fs_mountfs`: the geometry `e2fs_sbfill` computes, statfs, reads
    // through the block pointers, the indirect block and the extent tree, the block maps, file
    // handles, a read-write mount that allocates and truncates and unmounts clean, the super
    // block checks name by name, the largest file, the super block and descriptors written
    // back byte for byte, the mount updates and the reload, a stalled sync, vget's
    // generation and deleted inodes, and the device vnode's count across a vget that fails
    // half-way (the user's decision of 2026-11-05). User memory: USER_BASE + 0xe000..0xf000
    // (four path slots, then a buffer), which no other test of the binary uses (the tests
    // hold MEMORY: one at a time).

    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::sys::mount::MNT_RDONLY;

    use super::super::ext2fs_dinode::{EXT4_EXTENTS, Ext2fsDinode};
    use super::super::ext2fs_inode::{ext2fs_size, ext2fs_truncate};
    use super::super::ext2fs_subr::ext2fs_bufatoff;
    use super::super::testext2fs::newfs::{
        self, B, BBITMAP, BIG_SIZE, BIG0, EXT_SIZE, GDT, HELLO, HELLO_DATA, IBITMAP, IPG, ITABLE,
        Image, NBLK, NFREE, NIFREE, pattern,
    };
    use super::super::testext2fs::{attach, fail_block, new_file, try_mount};
    use super::*;

    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs::{devvp, disk, set_disk};
    use crate::fs::ufs::ufs::dinode::{IFREG, ROOTINO};
    use crate::fs::vnode::{VDIR, VREG, VROOT};
    use crate::fs::vops::{VFS_ROOT, vop_bmap, vop_read, vop_write};
    use crate::testutil::{USER_BASE, printf_count};
    use crate::uio::{Iovec, Uio};

    /// The path slots: four of 256 bytes.
    const UPATH: usize = USER_BASE + 0xe000;
    /// The buffer, to USER_BASE + 0xf000.
    const UBUF: usize = USER_BASE + 0xe400;

    /// The NUL-terminated `s` in user memory, in slot `i`.
    fn up(i: usize, s: &[u8]) -> UserPtr<u8> {
        assert!(s.len() < 256 && i < 4);
        let at = UPATH + i * 256;
        let mut b = s.to_vec();
        b.push(0);
        UserPtr::<u8>::new(at).copyout_slice(&b).unwrap();
        UserPtr::new(at)
    }

    /// A `Nameidata` for `ext2fs_mount`'s `ndp` (an update reads its credential).
    fn nd(p: &Thread<TestGlue>) -> Nd<'_, TestGlue> {
        ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/ext"), p)
    }

    /// The whole file through `VOP_READ`, `chunk` bytes at a time.
    fn read_all(p: &Thread<TestGlue>, vp: &'static Vnode<TestGlue>, chunk: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut buf = vec![0u8; chunk];
            let mut iov = [Iovec::new()];
            let mut u = Uio::kernel_read(&mut iov, &mut buf, Some(p));
            u.uio_offset = out.len() as Off;
            vop_read(vp, &mut u, 0, Cred::real(p.ucred())).unwrap();
            let n = chunk - u.uio_resid;
            if n == 0 {
                return out;
            }
            out.extend_from_slice(&buf[..n]);
        }
    }

    /// `VOP_WRITE` of `data` at `offset`.
    fn write_at(p: &Thread<TestGlue>, vp: &'static Vnode<TestGlue>, offset: Off, data: &[u8]) {
        let mut iov = [Iovec::new()];
        let mut u = Uio::kernel_write(&mut iov, data, Some(p));
        u.uio_offset = offset;
        vop_write(vp, &mut u, 0, Cred::real(p.ucred())).unwrap();
        assert_eq!(u.uio_resid, 0);
    }

    /// `VOP_BMAP`: the disk block and run of logical block `bn`.
    fn bmap(vp: &'static Vnode<TestGlue>, bn: Daddr) -> (Daddr, i32) {
        let mut bnp = 0;
        let mut run = 0;
        vop_bmap(vp, bn, None, Some(&mut bnp), Some(&mut run)).unwrap();
        (bnp, run)
    }

    /// A little-endian `u32` of the disk image.
    fn disk_u32(off: usize) -> u32 {
        let d = disk();
        u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
    }

    /// A little-endian `u16` of the disk image.
    fn disk_u16(off: usize) -> u16 {
        let d = disk();
        u16::from_le_bytes([d[off], d[off + 1]])
    }

    /// Whether bit `bit` of the bitmap in block `blk` is set on the disk.
    fn disk_bit(blk: usize, bit: usize) -> bool {
        disk()[blk * B + bit / 8] & (1 << (bit % 8)) != 0
    }

    /// The super block's byte at member offset `off`, on the disk.
    const SB: usize = 1024;

    #[test]
    fn mount_read_map_statfs_and_unmount() {
        let (_g, p) = attach(newfs::image(true));
        let mp = try_mount(p, MNT_RDONLY).unwrap();
        let ump = vfstoufs(mp);
        let fs = ump.um_e2fs();
        assert_eq!(ump.um_fstype.get(), UM_EXT2FS);
        assert_eq!(ump.um_nindir.get(), 256);
        assert_eq!(ump.um_bptrtodb.get(), 1);
        assert_eq!(ump.um_seqinc.get(), 1);
        assert_eq!(ump.um_maxsymlinklen.get(), 60);
        assert_eq!(fs.e2fs_ncg.get(), 1);
        assert_eq!(fs.e2fs_ngdb.get(), 1);
        assert_eq!(fs.e2fs_bsize.get(), 1024);
        assert_eq!(fs.e2fs_fsbtodb.get(), 1);
        assert_eq!(fs.e2fs_bshift.get(), 10);
        assert_eq!((fs.e2fs_qbmask.get(), fs.e2fs_bmask.get()), (1023, !1023));
        assert_eq!(fs.e2fs_ipb.get(), 8);
        assert_eq!(fs.e2fs_itpg.get(), 4);
        assert_eq!(fs.gd(0).ext2bgd_i_tables, ITABLE as u32);
        assert_eq!(fs.e2fs_ronly.get(), 1);
        assert_eq!(fs.e2fs_maxfilesize.get(), Off::from(i32::MAX) * 4);
        assert_eq!(mp.flag() & MNT_LOCAL, MNT_LOCAL);
        assert!(core::ptr::eq(devvp().specmountpoint().unwrap(), mp));

        // statfs: the overhead is the boot block, two bitmaps and four inode table blocks, and
        // one super block and one descriptor block.
        let st = mp.stat();
        assert_eq!(st.f_bsize, 1024);
        assert_eq!(st.f_iosize, 1024);
        assert_eq!(st.f_blocks, u64::from(NBLK) - 9);
        assert_eq!(st.f_bfree, u64::from(NFREE));
        assert_eq!(st.f_bavail, i64::from(NFREE) - 3);
        assert_eq!(st.f_files, u64::from(IPG));
        assert_eq!(st.f_ffree, u64::from(NIFREE));
        assert_eq!(&st.f_fstypename[..7], b"ext2fs\0");
        assert_eq!(st.f_fsid.val[1], 17);
        assert_eq!(st.f_namemax, 255);

        // The root directory.
        let root = VFS_ROOT(mp).unwrap();
        assert_eq!(root.vtype(), VDIR);
        assert_eq!(root.flag() & VROOT, VROOT);
        let rip = vtoi(root);
        assert_eq!(rip.i_number, ROOTINO);
        assert_eq!(rip.i_e2din().i_e2fs_uid.get(), 1000 | 1 << 16);
        assert_eq!(rip.i_e2din().i_e2fs_gid.get(), 20);
        assert_eq!(rip.i_effnlink.get(), 2);
        drop(rip);
        let dir = read_all(p, root, 300);
        assert_eq!(dir.len(), B);
        assert_eq!(&dir[24..28], &12u32.to_le_bytes());
        assert_eq!(&dir[32..41], b"hello.txt");

        // hello.txt
        let hello = VFS_VGET(mp, 12).unwrap();
        assert_eq!(hello.vtype(), VREG);
        assert_eq!(read_all(p, hello, 5), HELLO_DATA);
        assert_eq!(bmap(hello, 0), (2 * HELLO as Daddr, 0));
        assert_eq!(bmap(hello, 1).0, -1);

        // big: direct blocks in one run, then the two behind the indirect block.
        let big = VFS_VGET(mp, 13).unwrap();
        assert_eq!(read_all(p, big, 1000), pattern(BIG_SIZE, 13));
        assert_eq!(bmap(big, 0), (2 * BIG0 as Daddr, 11));
        assert_eq!(bmap(big, 12), (48, 1));
        assert_eq!(bmap(big, 13), (50, 0));
        assert_eq!(bmap(big, 14).0, -1);

        // ext: through the extent tree (and its cache).
        let ext = VFS_VGET(mp, 14).unwrap();
        assert_ne!(vtoi(ext).i_e2din().i_e2fs_flags() & EXT4_EXTENTS, 0);
        assert_eq!(read_all(p, ext, 700), pattern(EXT_SIZE, 14));
        assert_eq!(bmap(ext, 1).0, 2 * 28);
        assert_eq!(bmap(ext, 2).0, 2 * 30);
        let (t, off) = ext2fs_bufatoff(&vtoi(ext), 2 * B as Off + 5).unwrap();
        assert_eq!(t.bytes()[off], pattern(EXT_SIZE, 14)[2 * B + 5]);
        brelse(t);

        // File handles.
        let mut fid = Fid::default();
        ext2fs_vptofh(hello, &mut fid).unwrap();
        let u = Ufid::from_fid(&fid);
        assert_eq!((u.ufid_len, u.ufid_ino, u.ufid_gen), (12, 12, 19));
        for ino in [5, IPG + 1] {
            let bad = Ufid { ufid_ino: ino, ..u };
            bad.to_fid(&mut fid);
            assert_eq!(ext2fs_fhtovp(mp, &fid).err(), Some(Errno::ESTALE));
        }

        vput(ext);
        vput(big);
        vput(hello);
        vput(root);
        drop(ump);
        ext2fs_unmount(mp, 0, p).unwrap();
        assert!(!mp.has_node(), "the mount's handle went");
        assert!(devvp().specmountpoint().is_none());
        // A read-only unmount writes nothing.
        assert_eq!(disk(), newfs::image(true));
    }

    #[test]
    fn a_read_write_mount_allocates_truncates_and_unmounts_clean() {
        let (_g, p) = attach(newfs::image(false));
        let mp = try_mount(p, 0).unwrap();
        let ump = vfstoufs(mp);
        let fs = ump.um_e2fs();
        assert_eq!(fs.e2fs_state(), 0);
        assert_eq!(fs.e2fs_fmod.get(), 1);
        assert_eq!(fs.e2fs_maxfilesize.get(), Off::from(i32::MAX));
        // Nothing written by the mount itself: the disk still says clean.
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);

        let root = VFS_ROOT(mp).unwrap();
        let hello = VFS_VGET(mp, 12).unwrap();
        let ip = vtoi(hello);
        let cred = Cred::real(p.ucred());

        // Grow hello.txt to three blocks: two new ones, the first 8 free ones of the group.
        let data = pattern(3000, 1);
        write_at(p, hello, 0, &data);
        assert_eq!(ext2fs_size(&ip), 3000);
        assert_eq!(fs.e2fs_fbcount(), NFREE - 2);
        assert_eq!(u32::from(fs.gd(0).ext2bgd_nbfree), NFREE - 2);
        let d = ip.i_e2din();
        assert_eq!(d.i_e2fs_nblock(), 6);
        assert_eq!(d.i_e2fs_block(1), 33);
        assert_eq!(d.i_e2fs_block(2), 34);
        assert_eq!(read_all(p, hello, 512), data);

        // Truncate it back into its first block.
        ext2fs_truncate(&ip, 12, 0, cred).unwrap();
        assert_eq!(ext2fs_size(&ip), 12);
        assert_eq!(fs.e2fs_fbcount(), NFREE);
        assert_eq!(d.i_e2fs_nblock(), 2);
        assert_eq!(d.i_e2fs_blocks()[1..], [0; 14]);
        assert_eq!(read_all(p, hello, 512), data[..12]);

        // A byte at logical block 13: an indirect block and a data block, holes before. With
        // no block to follow, both go to the first wholly free byte of the bitmap: blocks
        // 33..=40, then 41..=48.
        write_at(p, hello, 13 * B as Off, b"Z");
        assert_eq!(fs.e2fs_fbcount(), NFREE - 2);
        assert_eq!(d.i_e2fs_block(12), 33);
        assert_eq!(bmap(hello, 13).0, 2 * 41);
        let all = read_all(p, hello, 4096);
        assert_eq!(all.len(), 13 * B + 1);
        assert_eq!(all[..12], data[..12]);
        assert!(all[12..13 * B].iter().all(|&b| b == 0));
        assert_eq!(all[13 * B], b'Z');

        // Truncate to nothing: the data block under the indirect block, the indirect block and
        // the first block go back.
        ext2fs_truncate(&ip, 0, 0, cred).unwrap();
        assert_eq!(ext2fs_size(&ip), 0);
        assert_eq!(fs.e2fs_fbcount(), NFREE + 1);
        assert_eq!(d.i_e2fs_nblock(), 0);
        assert_eq!(d.i_e2fs_blocks(), [0; 15]);

        // A new inode: the first free one, 15; let go with no links, it is freed again.
        let nvp = new_file(mp, p);
        let nip = vtoi(nvp);
        assert_eq!(nip.i_number, 15);
        assert_eq!(fs.e2fs_ficount(), NIFREE - 1);
        assert_ne!(nip.i_e2din().i_e2fs_gen(), 0);
        nip.i_e2din().set_i_e2fs_nlink(0);
        nip.i_effnlink.set(0);
        drop(nip);
        vput(nvp);
        assert_eq!(fs.e2fs_ficount(), NIFREE);

        drop(ip);
        vput(hello);
        vput(root);
        drop(ump);
        ext2fs_unmount(mp, 0, p).unwrap();

        // On the disk: clean, the counts and bitmaps back, hello.txt empty, inode 15 free.
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);
        assert_eq!(disk_u32(SB + offset_of!(Ext2fs, e2fs_fbcount)), NFREE + 1);
        assert_eq!(disk_u32(SB + offset_of!(Ext2fs, e2fs_ficount)), NIFREE);
        assert_eq!(disk_u16(GDT * B + 12), (NFREE + 1) as u16);
        assert_eq!(disk_u16(GDT * B + 14), NIFREE as u16);
        assert!(!disk_bit(BBITMAP, HELLO - 1));
        assert!(!disk_bit(BBITMAP, 32) && !disk_bit(BBITMAP, 33) && !disk_bit(BBITMAP, 40));
        assert!(!disk_bit(IBITMAP, 14));
        let (blk, off) = Image::ioff(12);
        let at = blk * B + off;
        assert_eq!(disk_u32(at + offset_of!(Ext2fsDinode, e2di_size)), 0);
        assert_eq!(disk_u32(at + offset_of!(Ext2fsDinode, e2di_nblock)), 0);
        assert_eq!(disk_u32(at + offset_of!(Ext2fsDinode, e2di_blocks)), 0);
        assert_eq!(disk_u16(at + offset_of!(Ext2fsDinode, e2di_uid_high)), 1);
        // Inode 15 went to its block as it was let go, mode kept, no links.
        let (blk, off) = Image::ioff(15);
        let at = blk * B + off;
        assert_eq!(
            disk_u16(at + offset_of!(Ext2fsDinode, e2di_mode)),
            (IFREG | 0o644) as u16
        );
        assert_eq!(disk_u16(at + offset_of!(Ext2fsDinode, e2di_nlink)), 0);
        assert_ne!(disk_u32(at + offset_of!(Ext2fsDinode, e2di_gen)), 0);
    }

    /// The test image with `f` applied to its super block (as a copy read out and written
    /// back in the disk's byte order).
    fn with_sb(f: &dyn Fn(&mut Ext2fs)) -> Vec<u8> {
        let mut img = newfs::image(false);
        let mut s = Ext2fs::from_disk(&img[SB..SB + SBSIZE]);
        f(&mut s);
        s.to_disk(&mut img[SB..SB + SBSIZE]);
        img
    }

    #[test]
    fn ext4_needs_a_read_only_mount_and_bad_super_blocks_are_refused() {
        let (_g, p) = attach(newfs::image(true));
        let line = "ext4fs: only read-only support right now\n";
        let n = printf_count(line);
        assert_eq!(try_mount(p, 0).err(), Some(Errno::EROFS));
        assert_eq!(printf_count(line), n + 1);
        assert!(
            devvp().specmountpoint().is_none(),
            "nothing left of the mount"
        );

        assert_eq!(
            e2fs_sbcheck(&with_sb(&|_| {})[SB..SB + SBSIZE], false),
            Ok(())
        );
        // Each check, in the C's order, with its errno and its line (the first whose
        // condition holds is the one reported).
        type Edit = dyn Fn(&mut Ext2fs);
        let cases: [(&Edit, bool, Errno, &str); 9] = [
            (
                &|s| s.e2fs_magic = 0x1234,
                true,
                Errno::EIO,
                "ext2fs: wrong magic number 0x1234\n",
            ),
            (
                &|s| s.e2fs_log_bsize = 3,
                true,
                Errno::EIO,
                "ext2fs: wrong log2(block size) 13\n",
            ),
            (
                &|s| s.e2fs_bpg = 0,
                true,
                Errno::EIO,
                "ext2fs: zero blocks per group\n",
            ),
            (
                &|s| s.e2fs_rev = 2,
                true,
                Errno::EIO,
                "ext2fs: wrong revision number 0x2\n",
            ),
            (
                &|s| s.e2fs_first_ino = 12,
                true,
                Errno::EINVAL,
                "ext2fs: first inode at 0xc\n",
            ),
            (
                &|s| s.e2fs_features_incompat |= 0x0008,
                true,
                Errno::EINVAL,
                "ext2fs: unsupported incompat features: ",
            ),
            (
                &|s| s.e2fs_features_incompat |= EXT2F_INCOMPAT_EXTENTS,
                false,
                Errno::EROFS,
                "ext4fs: only read-only support right now\n",
            ),
            (
                // RECOVER is one of EXT4F_RO_INCOMPAT_SUPP's: read-write, the ext4 check
                // answers first.
                &|s| s.e2fs_features_incompat |= EXT2F_INCOMPAT_RECOVER,
                false,
                Errno::EROFS,
                "ext4fs: only read-only support right now\n",
            ),
            (
                &|s| s.e2fs_features_rocompat |= 0x0004,
                false,
                Errno::EROFS,
                "ext2fs: unsupported R/O compat features: ",
            ),
        ];
        for (f, ronly, e, line) in cases {
            let n = printf_count(line);
            assert_eq!(
                e2fs_sbcheck(&with_sb(f)[SB..SB + SBSIZE], ronly),
                Err(e),
                "{line}"
            );
            assert_eq!(printf_count(line), n + 1, "{line}");
        }
        // A file system to recover mounts read-only (with its line); an unknown read-only
        // feature too, and extents.
        let recover = with_sb(&|s| s.e2fs_features_incompat |= EXT2F_INCOMPAT_RECOVER);
        let line = "ext2fs: your file system says it needs recovery\n";
        let n = printf_count(line);
        assert_eq!(e2fs_sbcheck(&recover[SB..SB + SBSIZE], true), Ok(()));
        assert_eq!(printf_count(line), n + 1, "said, and mounted read-only");
        let btree = with_sb(&|s| s.e2fs_features_rocompat |= 0x0004);
        assert_eq!(e2fs_sbcheck(&btree[SB..SB + SBSIZE], true), Ok(()));
        // Revision 0 skips the feature checks.
        let rev0 = with_sb(&|s| {
            s.e2fs_rev = 0;
            s.e2fs_first_ino = 0;
            s.e2fs_features_incompat = 0x8000;
        });
        assert_eq!(e2fs_sbcheck(&rev0[SB..SB + SBSIZE], false), Ok(()));
        // A bad super block on the disk: the mount fails EIO and leaves nothing.
        set_disk(with_sb(&|s| s.e2fs_magic = 0x5555));
        assert_eq!(try_mount(p, MNT_RDONLY).err(), Some(Errno::EIO));
        assert!(devvp().specmountpoint().is_none());
    }

    #[test]
    fn the_largest_file_follows_the_block_size_and_huge_files() {
        let fs = Ext2Superblock::new();
        fs.e2fs_bsize.set(1024);
        assert_eq!(
            ext2fs_maxfilesize(&fs),
            (12 + 256 + 256 * 256 + 256 * 256 * 256) * 1024
        );
        fs.e2fs_bsize.set(4096);
        // Logically 4 TiB and more; the 32-bit sector count of a disk address stops at 2 TiB.
        assert_eq!(ext2fs_maxfilesize(&fs), Off::from(u32::MAX) * 512);
        fs.set_e2fs_features_rocompat(EXT2F_ROCOMPAT_HUGE_FILE);
        assert_eq!(
            ext2fs_maxfilesize(&fs),
            (12 + 1024 + 1024 * 1024 + 1024 * 1024 * 1024) * 4096
        );
    }

    #[test]
    fn sbfill_takes_the_c_widths_and_large_files() {
        // A large-file revision 1 file system: the largest file of its block size.
        let (_g, _p) = attach(with_sb(&|s| {
            s.e2fs_features_rocompat |= EXT2F_ROCOMPAT_LARGE_FILE;
        }));
        let fs = super::super::testext2fs::superblock();
        assert_eq!(fs.e2fs_maxfilesize.get(), ext2fs_maxfilesize(&fs));
        assert_eq!(fs.gd(0).ext2bgd_b_bitmap, BBITMAP as u32);
        // The C's 32-bit arithmetic: a block count below the first data block wraps (0 - 1 +
        // 8191, divided by 8192, is 0 groups, so no descriptor block is read), and an
        // impossible fragment shift is the machine's 32-bit shift (33 is 1), not a panic.
        fs.set_e2fs_bcount(0);
        fs.set_e2fs_log_fsize(33);
        e2fs_sbfill(devvp(), &fs).unwrap();
        assert_eq!((fs.e2fs_ncg.get(), fs.e2fs_ngdb.get()), (0, 0));
        assert_eq!(fs.e2fs_fsize.get(), 2048);
        // Revision 0 has no large files.
        fs.set_e2fs_rev(E2FS_REV0);
        e2fs_sbfill(devvp(), &fs).unwrap();
        assert_eq!(fs.e2fs_maxfilesize.get(), Off::from(i32::MAX));
    }

    #[test]
    fn cgupdate_writes_the_copy_in_core_byte_for_byte() {
        let (_g, p) = attach(newfs::image(false));
        let mp = try_mount(p, 0).unwrap();
        let ump = vfstoufs(mp);
        let fs = ump.um_e2fs();
        fs.set_gd_nbfree(0, 5);
        fs.set_gd_ndirs(0, 3);
        fs.set_e2fs_fbcount(77);
        fs.e2fs_fmod.set(1);
        ext2fs_cgupdate(&ump, MNT_WAIT).unwrap();
        assert_eq!(fs.e2fs_fmod.get(), 0, "sbupdate clears fmod");
        let d = disk();
        let mut sb = vec![0u8; SBSIZE];
        e2fs_sbsave(fs, &mut sb);
        assert_eq!(&d[SB..SB + SBSIZE], &sb[..], "the super block");
        let mut gd = vec![0u8; B];
        e2fs_cgsave(fs, 0, &mut gd, B);
        assert_eq!(&d[GDT * B..(GDT + 1) * B], &gd[..], "the descriptor block");
        assert_eq!(disk_u16(GDT * B + 12), 5);
        assert_eq!(disk_u16(GDT * B + 16), 3);
        assert_eq!(disk_u32(SB + offset_of!(Ext2fs, e2fs_fbcount)), 77);
        // The bytes the image had past the descriptors are the copy in core's too (zero).
        assert!(
            d[GDT * B + EXT2_GD_SIZE..(GDT + 1) * B]
                .iter()
                .all(|&b| b == 0)
        );
        fs.set_gd_nbfree(0, NFREE as u16);
        fs.set_gd_ndirs(0, 1);
        fs.set_e2fs_fbcount(NFREE);
        drop(ump);
        ext2fs_unmount(mp, 0, p).unwrap();
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);
    }

    #[test]
    fn a_new_mount_needs_arguments_and_a_block_device() {
        let (_g, p) = attach(newfs::image(false));
        let mp = crate::fs::mount::vfs_mount_alloc(None, &super::super::testext2fs::TEST_EXT2_CONF);
        assert_eq!(
            ext2fs_mount(mp, b"/ext", &mut [], &mut nd(p), p),
            Err(Errno::EINVAL),
            "no ufs_args"
        );
        let mut data = vec![0u8; UfsArgs::SIZE];
        let mut args = UfsArgs::from_bytes(&data).unwrap();
        for (fspec, want) in [
            (&b"/f"[..], Errno::ENOTBLK),
            (b"/dev/sd0c", Errno::ENOTBLK),
            (b"/nothere", Errno::ENOENT),
        ] {
            args.fspec = up(0, fspec).addr();
            assert!(hal::user::pod_to_bytes(&args, &mut data));
            assert_eq!(
                ext2fs_mount(mp, b"/ext", &mut data, &mut nd(p), p),
                Err(want)
            );
        }
        // A name without its NUL within MNAMELEN.
        UserPtr::<u8>::new(UBUF)
            .copyout_slice(&[b'x'; MNAMELEN + 1])
            .unwrap();
        args.fspec = UBUF;
        assert!(hal::user::pod_to_bytes(&args, &mut data));
        assert_eq!(
            ext2fs_mount(mp, b"/ext", &mut data, &mut nd(p), p),
            Err(Errno::ENAMETOOLONG)
        );
        vfs_unbusy(mp);
        vfs_mount_free(mp);
    }

    #[test]
    fn an_update_to_read_only_and_back_writes_the_clean_state() {
        let (_g, p) = attach(newfs::image(false));
        let mp = try_mount(p, 0).unwrap();
        let fs_of = || vfstoufs(mp);
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);

        // Read-only: the files flushed, the super block and descriptors written clean.
        mp.set_flag(mp.flag() | MNT_UPDATE | MNT_RDONLY);
        ext2fs_mount(mp, b"/ext", &mut [], &mut nd(p), p).unwrap();
        assert_eq!(fs_of().um_e2fs().e2fs_ronly.get(), 1);
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);

        // Back to read-write: dirty in core, to be written.
        mp.set_flag(mp.flag() | MNT_WANTRDWR);
        ext2fs_mount(mp, b"/ext", &mut [], &mut nd(p), p).unwrap();
        let fs = fs_of();
        assert_eq!(fs.um_e2fs().e2fs_ronly.get(), 0);
        assert_eq!(fs.um_e2fs().e2fs_state(), 0);
        assert_eq!(fs.um_e2fs().e2fs_fmod.get(), 1);
        drop(fs);

        // A reload needs a read-only mount.
        mp.set_flag((mp.flag() & !(MNT_RDONLY | MNT_WANTRDWR)) | MNT_UPDATE | MNT_RELOAD);
        assert_eq!(
            ext2fs_mount(mp, b"/ext", &mut [], &mut nd(p), p),
            Err(Errno::EINVAL)
        );
        mp.set_flag(mp.flag() & !(MNT_UPDATE | MNT_RELOAD));
        ext2fs_unmount(mp, 0, p).unwrap();
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);
    }

    #[test]
    fn a_reload_reads_the_super_block_descriptors_and_inodes_again() {
        let (_g, p) = attach(newfs::image(false));
        let mp = try_mount(p, MNT_RDONLY).unwrap();
        let ump = vfstoufs(mp);
        let fs = ump.um_e2fs();
        // An active vnode (referenced, unlocked) and an inactive one.
        let hello = VFS_VGET(mp, 12).unwrap();
        let _ = vop_unlock(hello);
        let big = VFS_VGET(mp, 13).unwrap();
        vput(big);

        // fsck changed the disk under the read-only mount.
        let mut img = disk();
        img[SB + offset_of!(Ext2fs, e2fs_fbcount)..][..4]
            .copy_from_slice(&(NFREE - 1).to_le_bytes());
        img[GDT * B + 12..][..2].copy_from_slice(&((NFREE - 1) as u16).to_le_bytes());
        let (blk, off) = Image::ioff(12);
        img[blk * B + off + offset_of!(Ext2fsDinode, e2di_size)..][..4]
            .copy_from_slice(&5u32.to_le_bytes());
        set_disk(img);

        mp.set_flag(mp.flag() | MNT_UPDATE | MNT_RELOAD);
        ext2fs_mount(mp, b"/ext", &mut [], &mut nd(p), p).unwrap();
        mp.set_flag(mp.flag() & !(MNT_UPDATE | MNT_RELOAD));
        assert_eq!(fs.e2fs_fbcount(), NFREE - 1);
        assert_eq!(u32::from(fs.gd_nbfree(0)), NFREE - 1);
        assert_eq!(fs.e2fs_ronly.get(), 1, "kept");
        assert_eq!(ext2fs_size(&vtoi(hello)), 5, "the active inode read again");

        // A bad super block is refused before anything changes.
        set_disk(with_sb(&|s| s.e2fs_magic = 0x4242));
        mp.set_flag(mp.flag() | MNT_UPDATE | MNT_RELOAD);
        assert_eq!(
            ext2fs_mount(mp, b"/ext", &mut [], &mut nd(p), p),
            Err(Errno::EIO)
        );
        mp.set_flag(mp.flag() & !(MNT_UPDATE | MNT_RELOAD));
        assert_eq!(fs.e2fs_fbcount(), NFREE - 1);
        vrele(hello);
        drop(ump);
        ext2fs_unmount(mp, 0, p).unwrap();
    }

    #[test]
    fn a_stalled_sync_writes_clean_only_without_dangling_inodes() {
        let (_g, p) = attach(newfs::image(false));
        let mp = try_mount(p, 0).unwrap();
        let ump = vfstoufs(mp);
        let fs = ump.um_e2fs();
        let cred = Cred::real(p.ucred());
        ext2fs_sync(mp, MNT_WAIT, 1, cred, p).unwrap();
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);
        assert_eq!(
            (fs.e2fs_state(), fs.e2fs_fmod.get()),
            (0, 1),
            "restored in core"
        );

        // A file removed while open: its inode dangles (nlink0), so the disk stays dirty.
        let vp = new_file(mp, p);
        let ip = vtoi(vp);
        ip.i_e2din().set_i_e2fs_nlink(0);
        ip.i_effnlink.set(0);
        ip.set_flag(IN_CHANGE);
        drop(ip);
        let _ = vop_unlock(vp);
        ext2fs_sync(mp, MNT_WAIT, 1, cred, p).unwrap();
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), 0);
        vrele(vp);
        // A lazy sync flushes no device and still writes the modified super block.
        let w = fs.e2fs_wtime();
        fs.e2fs_fmod.set(1);
        ext2fs_sync(mp, MNT_LAZY, 0, cred, p).unwrap();
        assert!(fs.e2fs_wtime() >= w);
        drop(ump);
        ext2fs_unmount(mp, 0, p).unwrap();
        assert_eq!(disk_u16(SB + offset_of!(Ext2fs, e2fs_state)), E2FS_ISCLEAN);
    }

    #[test]
    fn file_handles_name_inodes_and_refuse_stale_ones() {
        let (_g, p) = attach(newfs::image(false));
        let mp = try_mount(p, 0).unwrap();
        let vp = ext2fs_vget(mp, 12).unwrap();
        let mut fid = Fid::default();
        ext2fs_vptofh(vp, &mut fid).unwrap();
        vput(vp);
        let ufid = Ufid::from_fid(&fid);
        let back = ext2fs_fhtovp(mp, &fid).unwrap();
        assert!(ptr::eq(back, vp), "the hashed vnode");
        vput(back);

        let with = |f: &dyn Fn(&mut Ufid)| {
            let mut u = ufid;
            f(&mut u);
            let mut h = Fid::default();
            u.to_fid(&mut h);
            ext2fs_fhtovp(mp, &h).map(vput)
        };
        assert_eq!(
            with(&|u| {
                u.ufid_ino = EXT2_ROOTINO;
                u.ufid_gen = 7 + EXT2_ROOTINO;
            }),
            Ok(()),
            "the root is below the first inode and valid"
        );
        assert_eq!(
            with(&|u| u.ufid_ino = 1),
            Err(Errno::ESTALE),
            "a reserved inode"
        );
        assert_eq!(
            with(&|u| u.ufid_ino = 33),
            Err(Errno::ESTALE),
            "past the inodes"
        );
        assert_eq!(
            with(&|u| u.ufid_gen = u.ufid_gen.wrapping_add(1)),
            Err(Errno::ESTALE),
            "another generation"
        );
        assert_eq!(
            with(&|u| u.ufid_ino = 20),
            Err(Errno::ESTALE),
            "a free inode"
        );
        ext2fs_unmount(mp, 0, p).unwrap();
    }

    #[test]
    fn vget_gives_an_old_inode_a_generation_and_resets_a_deleted_one() {
        let mut img = Image(newfs::image(false));
        let (blk, off) = Image::ioff(12);
        img.u32(blk, off + offset_of!(Ext2fsDinode, e2di_gen), 0);
        let (blk, off) = Image::ioff(13);
        img.u32(blk, off + offset_of!(Ext2fsDinode, e2di_dtime), 5);
        let (_g, p) = attach(img.0);
        let mp = try_mount(p, 0).unwrap();
        let vp = ext2fs_vget(mp, 12).unwrap();
        let ip = vtoi(vp);
        assert_ne!(
            ip.i_e2din().i_e2fs_gen(),
            0,
            "a generation for an old inode"
        );
        assert_ne!(
            ip.i_flag.get() & IN_MODIFIED,
            0,
            "to be written on a R/W mount"
        );
        assert!(ptr::eq(ip.itov(), vp));
        assert_eq!(ip.i_effnlink.get(), 1);
        drop(ip);
        vput(vp);
        let again = ext2fs_vget(mp, 12).unwrap();
        assert!(ptr::eq(again, vp), "from the inode hash");
        vput(again);

        // A deleted inode reads as nothing: mode, blocks and size zero.
        let vp = ext2fs_vget(mp, 13).unwrap();
        let ip = vtoi(vp);
        let d = ip.i_e2din();
        assert_eq!((d.i_e2fs_mode(), d.i_e2fs_nblock()), (0, 0));
        assert_eq!(ext2fs_size(&ip), 0);
        drop(ip);
        vput(vp);
        ext2fs_unmount(mp, 0, p).unwrap();
    }

    /// The user's decision of 2026-11-05 (D-c3-1): a vget that fails after the inode points
    /// at the mount leaves the device vnode's count where it was once the inode is reclaimed
    /// (the C's late `vref` releases one it never took there).
    #[test]
    fn a_vget_failing_half_way_keeps_the_device_vnodes_count() {
        let mut img = Image(newfs::image(false));
        // Inode 15: a fifo, which `ext2fs_vinit` refuses without fifofs.
        img.inode(15, 0o10644, 1, 0, &[], 0);
        let (_g, p) = attach(img.0);
        let mp = try_mount(p, MNT_RDONLY).unwrap();
        let base = devvp().usecount();

        // The inode's block cannot be read: the vnode is recycled at once (no dinode).
        let (blk, _) = Image::ioff(12);
        fail_block(Some(2 * blk as Daddr));
        assert_eq!(ext2fs_vget(mp, 12).err(), Some(Errno::EIO));
        assert_eq!(devvp().usecount(), base, "after the recycle");
        fail_block(None);
        let vp = ext2fs_vget(mp, 12).unwrap();
        assert_eq!(devvp().usecount(), base + 1, "the inode's own");
        vput(vp);

        // The fifo: the inode stays hashed, its reference with it, until it is reclaimed.
        assert_eq!(ext2fs_vget(mp, 15).err(), Some(Errno::EOPNOTSUPP));
        vflush(mp, None, 0).unwrap();
        assert_eq!(devvp().usecount(), base, "every inode reclaimed");
        ext2fs_unmount(mp, 0, p).unwrap();
    }

    #[test]
    fn the_table_answers_as_the_c_and_sysctl_is_eopnotsupp() {
        let (_g, p) = attach(newfs::image(false));
        let vfs = &Ext2fsVfs::<TestGlue>::VFS;
        let mut len = 0;
        assert_eq!(
            vfs.sysctl(&[1], 0, &mut len, 0, 0, p),
            Err(Errno::EOPNOTSUPP)
        );
        assert!(vfs.as_any().is_none());
        let mp = try_mount(p, 0).unwrap();
        assert_eq!(vfs.start(mp, 0, p), Ok(()));
        let root = vfs.root(mp).unwrap();
        assert_eq!(vtoi(root).i_number, EXT2_ROOTINO);
        vput(root);
        let mut st = mp.stat();
        vfs.statfs(mp, &mut st, p).unwrap();
        assert_eq!(st.f_ffree, u64::from(NIFREE));
        vfs.sync(mp, MNT_WAIT, 0, Cred::real(p.ucred()), p).unwrap();
        vfs.unmount(mp, 0, p).unwrap();
    }
}
/* </TESTS> */
