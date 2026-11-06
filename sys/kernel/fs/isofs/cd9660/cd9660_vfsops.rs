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

/*-
 * Copyright (c) 1994
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley
 * by Pace Willisson (pace@blitz.com).  The Rock Ridge Extension
 * Support code is derived from software contributed to Berkeley
 * by Atsushi Murai (amurai@spec.co.jp).
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
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
 *	@(#)cd9660_vfsops.c	8.9 (Berkeley) 12/5/94
 */
/* </LICENSES> */

/* <CODE> */
//! The ISO 9660 file system's file-system-type operations (`cd9660_vfsops`): mounting
//! (`cd9660_mountroot`, `cd9660_mount`, `iso_mountfs` with the volume descriptor search, the
//! Rock Ridge and Joliet detection), the volume label a CD's spoofed disk label is made of
//! (`iso_volume_label`), unmounting, `statfs`, the node cache's `cd9660_vget_internal`, and
//! file handles.
//!
//! Upstream: sys/isofs/cd9660/cd9660_vfsops.c @ 3ce1f3f79392
//! LZ: sys/isofs/cd9660/cd9660_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - `cd9660_mount` looks the device name up in a `Nameidata` of its own (`ndinit` around
//!   the kernel copy of the name) instead of reinitialising the caller's `ndp`, as
//!   `ffs_mount` does. A mount without arguments (the C's NULL `data`, which it would
//!   dereference) is `EINVAL`. The mount arguments copied into `mnt_stat.mount_info` are the
//!   caller's bytes with the `flags` that `iso_mountfs` changed (the C changes the kernel
//!   copy in place) (LZ's).
//! - `iso_mountfs`'s error path also releases the primary descriptor's buffer, which the C
//!   leaks when the logical block size is bad (LZ's).
//! - `cd9660_mountroot`: a `rootdev` without a device (`bdevvp` answering no vnode, which the
//!   C would pass on as NULL) is `ENODEV`, as in `ffs_mountroot` (LZ's).
//! - `option FIFO` is in GENERIC but `miscfs/fifofs` is not ported: `cd9660_fifovops` comes
//!   with it, and `cd9660_vget_internal` takes the C's `#else` branch for a fifo meanwhile
//!   (`EOPNOTSUPP`), as `ffs_vinit` does (LZ's).
//! - `struct ifid` is read from and written to the `struct fid` it overlays by
//!   [`Ifid::from_fid`]/[`Ifid::to_fid`] (LZ's).
//! - `CDIOREADMSADDR`'s `int` travels as its four bytes through `VOP_IOCTL` (LZ's); the
//!   command is a private constant until N7 brings `<sys/cdio.h>` to the kernel, as
//!   msdosfs's `DIOCCACHESYNC`.
//! - `ISOFS_DBG` is not defined.
//! - [`cd9660_fhtovp`] reads `ifid_ino` as the `cdino_t` that [`cd9660_vptofh`] stored in it
//!   (zero-extended), the user's decision D-N5.8-U2 of 2026-11-05: the C keeps it in an `int`
//!   that `VFS_VGET`'s `ino_t` sign-extends, so `cd9660_vget` panics ("alien ino_t") for every
//!   handle whose top bit is set: the handle of a file whose directory record lies 2 GiB or
//!   more into the disc, or a crafted one. Here such a handle names the record at its offset,
//!   which `cd9660_vget_internal`'s own checks refuse with `ESTALE` when it is no record of
//!   the volume (past `volume_space_size`, across its block); `cd9660_vget`'s check is kept
//!   for an `ino` no handle can give. What an NFS client of an exported disc sees: the files
//!   past 2 GiB open, and a crafted handle gets `ESTALE` or the record it names, where the
//!   OpenBSD server crashes.
//! - `cd9660_check_export` is `VFS_CHECKEXP`'s `MOUNT_CD9660` arm (`bsd`'s
//!   `sys/sys/mount.rs`): `ufs_check_export`, the same body over the mount's export list,
//!   which is the mount's glue member (`im_export`, `cd9660_extern.rs`).
//! - `iso_disklabelspoof` is `bsd`'s (`sys/kern/subr_disk.rs`) over [`iso_volume_label`]:
//!   the disk label and `dkcksum` are `bsd`'s until N7.
//!
//! ## Redesign
//! - LZ's `CD9660_VFSOPS` (a `bsd` `Vfsops` of function pointers over `bsd`'s mount, its
//!   `vfs_checkexp` LZ's `cd9660_check_export`) -> `impl Vfs<G> for Cd9660Vfs<G>` and its
//!   constant [`Cd9660Vfs::VFS`], which `bsd`'s `vfsconflist[]` names; the functions are the
//!   C's, generic over the glue, in its order (msdosfs's `MsdosfsVfs` shape).
//! - `iso_mountfs` reads each volume descriptor's members where they are in the buffer's
//!   token (`offset_of!` of `iso.rs`'s images, no 2,048-byte copy), checks them in the C's
//!   order with the C's errnos, builds the `struct iso_mnt` whole in its locals (D-N5.8-6:
//!   the C writes no member after the mount) and publishes it in its counted `M_ISOFSMNT`
//!   block (`IsoMnt::alloc`) hung from the mount's typed slot, where LZ `malloc`ed it and
//!   wrote it with `ptr::write` (five `unsafe` with the descriptors' views). The
//!   supplementary descriptor's buffer is released with the primary's, before the Rock Ridge
//!   check, its root record copied (D-N5.8-R1; the C holds it across that check,
//!   cd9660_vfsops.c:288-289 to :415-416), so the Rock Ridge walk runs with one device block
//!   held, the root directory's, which it is told (`held`, D-N5.8-U1). A mount refused after
//!   the C's `malloc` (the root block's read) makes that request on its error path: one
//!   `M_ISOFSMNT` block of the published block's size, dropped at once, so `vmstat -m`'s
//!   `ISOFS mount` row keeps the C's requests (D-N5.7-U5's shim, msdosfs's).
//! - `cd9660_vget_internal` makes the node with its identity and the mount structure's
//!   handle (`IsoNode::new`), publishes it in its counted `M_ISOFSNODE` block and hangs it
//!   from the vnode's typed slot where the C sets `v_data`, hashes it, then reads the record
//!   (the caller's view, or a token of the device's buffer whose block the Rock Ridge walk
//!   is told, D-N5.8-U1), fills the node's words and attributes (`IsoNode::update_inode`'s
//!   one writer), and on a device alias moves the node's slot to the alias as ext2fs's
//!   `ext2fs_vinit` does. LZ's two `unsafe` (the node written into its block, its
//!   `&'static`) and its lent buffers' views are gone.
//! - [`iso_volume_label`] is the C's descriptor scan of `iso_disklabelspoof` over a reader
//!   of a descriptor's first bytes (the shell's driver strategy, `bsd`'s, or a test's), its
//!   checks in the C's order; the shell copies [`ISO_LABEL_HEAD`] bytes of each descriptor
//!   where LZ viewed the whole buffer in place (two `unsafe`).
//! - `VFSTOISOFS` is a counted handle per call (one count pair per `Vfs` operation reached
//!   from the mount, ROADMAP N12's row). 0 `unsafe` (LZ: 11).

use core::marker::PhantomData;
use core::mem::{MaybeUninit, offset_of};
use core::ptr;

use hal::malloc::{M_WAITOK, MArc};
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FREAD, FWRITE};
use hal::sys::mount::{
    Fid, ISOFSMNT_EXTATT, ISOFSMNT_GENS, ISOFSMNT_NOJOLIET, ISOFSMNT_NORRIP, ISOFSMNT_SESS,
    IsoArgs, MNAMELEN, MNT_FORCE, MNT_LOCAL, MNT_RDONLY, MNT_UPDATE, Statfs,
};
use hal::sys::syslimits::NAME_MAX;
use hal::sys::types::{Daddr, Dev, Ino, Uid, major};
use hal::user::UserPtr;

use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::fs::bio::{DEV_BSHIFT, bread, brelse};
use crate::fs::buf::BufRef;
use crate::fs::isofs::cd9660::cd9660_extern::{
    ISO_FTYPE_9660, ISO_FTYPE_DEFAULT, ISO_FTYPE_RRIP, ISOFSMNT_ROOT, IsoMnt, M_ISOFSMNT, blkoff,
    lblkno, vfstoisofs,
};
use crate::fs::isofs::cd9660::cd9660_lookup::cd9660_bufatoff;
use crate::fs::isofs::cd9660::cd9660_node::{
    IsoNode, cd9660_defattr, cd9660_deftstamp, cd9660_ihashget, cd9660_ihashins, cd9660_init,
    extattr_buf, isodirino, vtoi,
};
use crate::fs::isofs::cd9660::cd9660_rrip::{cd9660_rrip_analyze, cd9660_rrip_offset};
use crate::fs::isofs::cd9660::iso::{
    Cdino, ISO_DEFAULT_BLOCK_SIZE, ISO_DIRECTORY_RECORD_SIZE, ISO_STANDARD_ID, ISO_VD_END,
    ISO_VD_PRIMARY, ISO_VD_SUPPLEMENTARY, IsoDirectoryRecord, IsoPrimaryDescriptor,
    IsoSupplementaryDescriptor, IsoVolumeDescriptor, isonum_711, isonum_723, isonum_733,
};
use crate::fs::lookup::{namei, ndinit};
use crate::fs::mount::{Mount, Vfsconf, mountlist_insert_tail, vfs_mount_free, vfs_unbusy};
use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
use crate::fs::spec::devsw;
use crate::fs::subr::{
    bdevvp, checkalias, copy_statfs_info, getnewvnode, vcount, vflush, vfs, vfs_mountedon,
    vfs_rootmountalloc, vgone, vinvalbuf, vops_id, vput, vref, vrele,
};
use crate::fs::ufs::ffs::ffs_vfsops::mname_copy;
use crate::fs::vnode::{
    FORCECLOSE, LK_EXCLUSIVE, LK_RETRY, V_SAVE, VBAD, VBLK, VCHR, VDIR, VFIFO, VLNK, VNON, VREG,
    VROOT, VSOCK, VT_ISOFS, Vnode, VopsId, iftovt, vn_lock,
};
use crate::fs::vops::{Cred, Nd, VFS_VGET, Vfs, vop_close, vop_ioctl, vop_open, vop_unlock};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};
use crate::time::inittodr;
use crate::vm::io::MAXBSIZE;
use crate::vm::vnode::uvm_vnp_setsize;

/// `DEV_BSIZE`: the size of a disk block, the unit of a buffer's block numbers.
const DEV_BSIZE: usize = 1 << DEV_BSHIFT;

/// `btodb(ISO_DEFAULT_BLOCK_SIZE)`: the disk blocks of a volume descriptor.
const VD_DBLOCKS: i32 = (ISO_DEFAULT_BLOCK_SIZE >> DEV_BSHIFT) as i32;

/// `CDIOREADMSADDR` (`<sys/cdio.h>`, `_IOWR('c', 6, int)`): the address of the last session
/// of a multi-session disc, which `iso_mountfs` asks of the device when the mount names
/// none (the device may not support it). A private copy until N7 brings `<sys/cdio.h>`'s
/// constants to the kernel, which replace it.
const CDIOREADMSADDR: u64 = hal::sys::ioccom::_iowr::<i32>(b'c', 6);

/// `offsetof(struct iso_volume_descriptor, type)`.
const VD_TYPE: usize = offset_of!(IsoVolumeDescriptor, type_);
/// `offsetof(struct iso_volume_descriptor, id)`.
const VD_ID: usize = offset_of!(IsoVolumeDescriptor, id);
/// `offsetof(struct iso_primary_descriptor, volume_id)`.
const PVD_VOLUME_ID: usize = offset_of!(IsoPrimaryDescriptor, volume_id);
/// `offsetof(struct iso_primary_descriptor, volume_space_size)`.
const PVD_VOLUME_SPACE_SIZE: usize = offset_of!(IsoPrimaryDescriptor, volume_space_size);
/// `offsetof(struct iso_primary_descriptor, logical_block_size)`.
const PVD_LOGICAL_BLOCK_SIZE: usize = offset_of!(IsoPrimaryDescriptor, logical_block_size);
/// `offsetof(struct iso_primary_descriptor, root_directory_record)`.
const PVD_ROOT: usize = offset_of!(IsoPrimaryDescriptor, root_directory_record);
/// `offsetof(struct iso_supplementary_descriptor, flags)`.
const SVD_FLAGS: usize = offset_of!(IsoSupplementaryDescriptor, flags);
/// `offsetof(struct iso_supplementary_descriptor, escape)`.
const SVD_ESCAPE: usize = offset_of!(IsoSupplementaryDescriptor, escape);
/// `offsetof(struct iso_supplementary_descriptor, root_directory_record)`.
const SVD_ROOT: usize = offset_of!(IsoSupplementaryDescriptor, root_directory_record);

/// `sizeof(iso_mnt.root)`: a root directory record.
const ROOT_RECORD: usize = 34;

/// The bytes of a volume descriptor [`iso_volume_label`] reads: from its start through the
/// primary descriptor's `logical_block_size`.
pub const ISO_LABEL_HEAD: usize = PVD_LOGICAL_BLOCK_SIZE + 4;

/// `sizeof(struct ifid)`.
pub const IFID_SIZE: u16 = 16;

/// `struct ifid`: an ISO 9660 file identifier, overlaid on `struct fid`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifid {
    /// `ifid_len`.
    pub ifid_len: u16,
    /// `ifid_pad`.
    pub ifid_pad: u16,
    /// `ifid_ino`: the file's `cdino_t` in the C's `int` (the module's deviations).
    pub ifid_ino: i32,
    /// `ifid_start`.
    pub ifid_start: i64,
}

impl Ifid {
    /// `(struct ifid *)fhp`: the file identifier read as an ISO 9660 one (`ifid_ino` at
    /// offset 4, `ifid_start` aligned at offset 8).
    pub fn from_fid(fid: &Fid) -> Self {
        let d = &fid.fid_data;
        let mut start = [0u8; 8];
        start.copy_from_slice(&d[4..12]);
        Self {
            ifid_len: fid.fid_len,
            ifid_pad: fid.fid_reserved,
            ifid_ino: i32::from_ne_bytes([d[0], d[1], d[2], d[3]]),
            ifid_start: i64::from_ne_bytes(start),
        }
    }

    /// Stores the identifier into the `struct fid` it overlays.
    pub fn to_fid(&self, fid: &mut Fid) {
        fid.fid_len = self.ifid_len;
        fid.fid_reserved = self.ifid_pad;
        fid.fid_data[0..4].copy_from_slice(&self.ifid_ino.to_ne_bytes());
        fid.fid_data[4..12].copy_from_slice(&self.ifid_start.to_ne_bytes());
    }
}

/// The ISO 9660 file system of the glue `G`: `cd9660_vfsops` ([`Cd9660Vfs::VFS`]). Its vnode
/// tables are the glue's `IsofsTables::vops` and `specvops` (`cd9660_vnops.rs`).
pub struct Cd9660Vfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Cd9660Vfs<G> {
    /// `cd9660_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"cd9660"`).
    pub const VFS: Cd9660Vfs<G> = Cd9660Vfs(PhantomData);
}

/// The `N` bytes at `off` of a descriptor's buffer, `None` past its end (a buffer of a
/// descriptor's 2,048 bytes always holds them).
fn field<const N: usize>(b: &[u8], off: usize) -> Option<[u8; N]> {
    let mut a = [0u8; N];
    a.copy_from_slice(b.get(off..off + N)?);
    Some(a)
}

/// `cd9660_mountroot`: called by `vfs_mountroot` (`dk_mountroot`) when iso is going to be
/// mounted as root: the vnodes of `swapdev` and `rootdev` (into `swapdev_vp` and `rootvp`),
/// the root mount of `"cd9660"`, the mount list, the clock.
pub fn cd9660_mountroot<G: ProcGlue>(rootdev: Dev, swapdev: Dev) -> Result<(), Errno> {
    G::with_curproc(|p| {
        let Some(p) = p else {
            kpanic(format_args!("cd9660_mountroot: no curproc"));
        };

        // Get vnodes for swapdev and rootdev.
        let made = bdevvp::<G>(swapdev).and_then(|svp| {
            <G::FileGlue as FileGlue>::set_swapdev_vp(svp);
            bdevvp::<G>(rootdev)
        });
        let rvp = match made {
            Ok(Some(rvp)) => rvp,
            Ok(None) => {
                printf(format_args!("cd9660_mountroot: can't setup bdevvp's"));
                return Err(Errno::ENODEV);
            }
            Err(e) => {
                printf(format_args!("cd9660_mountroot: can't setup bdevvp's"));
                return Err(e);
            }
        };
        <G::FileGlue as FileGlue>::set_rootvp(Some(rvp));

        let mp = vfs_rootmountalloc::<G>(b"cd9660", b"root_device")?;
        let Some(mut args) = IsoArgs::from_bytes(&[0u8; IsoArgs::SIZE]) else {
            kpanic(format_args!("cd9660_mountroot: no iso_args"));
        };
        args.flags = ISOFSMNT_ROOT;
        if let Err(e) = iso_mountfs(rvp, mp, p, &mut args) {
            vfs_unbusy(mp);
            vfs_mount_free(mp);
            return Err(e);
        }

        mountlist_insert_tail(mp);
        let mut st = mp.stat();
        let _ = cd9660_statfs(mp, &mut st, p);
        mp.with_stat(|sp| *sp = st);
        vfs_unbusy(mp);
        inittodr(0);

        Ok(())
    })
}

/// `cd9660_mount` (`vfs_mount`): mount system call. `data` is the kernel copy of the user's
/// `struct iso_args` (empty for the C's NULL).
pub fn cd9660_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    _ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let args = IsoArgs::from_bytes(data);

    if mp.flag() & MNT_RDONLY == 0 {
        return Err(Errno::EROFS);
    }

    // If updating, check whether changing from read-only to read/write; if there is no
    // device name, that's all we do.
    if mp.flag() & MNT_UPDATE != 0 {
        if let Some(a) = &args
            && a.fspec == 0
        {
            return <G::FileGlue as FileGlue>::vfs_export(mp, &a.export_info);
        }
        return Ok(());
    }

    // Not an update, or updating the name: look up the name and verify that it refers to a
    // sensible block device.
    let Some(mut args) = args else {
        return Err(Errno::EINVAL);
    };
    let mut fspec = [0u8; MNAMELEN];
    UserPtr::<u8>::new(args.fspec).copyinstr(&mut fspec)?;
    let flen = fspec.iter().position(|&c| c == 0).unwrap_or(MNAMELEN);
    let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(&fspec[..flen]), p);
    namei(&mut nd)?;
    let Some(devvp) = nd.ni_vp else {
        return Err(Errno::ENOENT);
    };

    if devvp.vtype() != VBLK {
        vrele(devvp);
        return Err(Errno::ENOTBLK);
    }
    if major(devvp.rdev()) >= devsw::<G>().nblkdev() {
        vrele(devvp);
        return Err(Errno::ENXIO);
    }

    let error = if mp.flag() & MNT_UPDATE == 0 {
        iso_mountfs(devvp, mp, p, &mut args)
    } else if !ptr::eq(devvp, vfstoisofs(mp).im_devvp) {
        Err(Errno::EINVAL) // needs translation
    } else {
        vrele(devvp);
        Ok(())
    };
    if let Err(e) = error {
        vrele(devvp);
        return Err(e);
    }

    let flags = offset_of!(IsoArgs, flags);
    mp.with_stat(|sp| {
        mname_copy(&mut sp.f_mntonname, path);
        mname_copy(&mut sp.f_mntfromname, &fspec);
        mname_copy(&mut sp.f_mntfromspec, &fspec);
        let info = &mut sp.mount_info.__align[..IsoArgs::SIZE];
        info.copy_from_slice(&data[..IsoArgs::SIZE]);
        info[flags..flags + 4].copy_from_slice(&args.flags.to_ne_bytes());
    });

    let mut st = mp.stat();
    let _ = cd9660_statfs(mp, &mut st, p);
    mp.with_stat(|sp| *sp = st);

    Ok(())
}

/// `iso_mountfs`: common code for mount and mountroot: find the primary (and a Joliet)
/// volume descriptor, check the logical block size, probe Rock Ridge, and hang the mount
/// structure from the mount's typed slot (the C's `mp->mnt_data`); `devvp`'s reference is
/// the mount's on success.
pub fn iso_mountfs<G: ProcGlue>(
    devvp: &'static Vnode<G>,
    mp: &'static Mount<G>,
    p: &Thread<G>,
    argp: &mut IsoArgs,
) -> Result<(), Errno> {
    let dev = devvp.rdev();
    let ronly = mp.flag() & MNT_RDONLY != 0;

    if !ronly {
        return Err(Errno::EROFS);
    }

    // Disallow multiple mounts of the same device. Disallow mounting of a device that is
    // currently in use (except for root, which might share swap device for miniroot). Flush
    // out any old buffers remaining from a previous use.
    vfs_mountedon(devvp)?;
    if vcount(devvp) > 1 && !<G::FileGlue as FileGlue>::rootvp().is_some_and(|r| ptr::eq(r, devvp))
    {
        return Err(Errno::EBUSY);
    }
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let error = vinvalbuf(devvp, V_SAVE, Cred::real(p.ucred()), Some(p), 0, INFSLP);
    let _ = vop_unlock(devvp);
    error?;

    let omode = if ronly { FREAD } else { FREAD | FWRITE };
    vop_open(devvp, omode, Cred::FSCRED, p)?;

    // This is the "logical sector size". The standard says this should be 2048 or the
    // physical sector size on the device, whichever is greater. For now, we'll just use a
    // constant.
    let iso_bsize = ISO_DEFAULT_BLOCK_SIZE as i32;

    let sess = if argp.flags & ISOFSMNT_SESS != 0 {
        argp.sess.max(0)
    } else {
        let mut b = 0i32.to_ne_bytes();
        match vop_ioctl(devvp, CDIOREADMSADDR, &mut b, 0, Cred::FSCRED, p) {
            Ok(()) => i32::from_ne_bytes(b),
            Err(_) => 0,
        }
    };

    // The C's `bp`, `pribp` and `supbp`; `malloced` says whether the C would have
    // `malloc`ed `isomp` (the module's redesign).
    let mut bp: Option<BufRef> = None;
    let mut pribp: Option<BufRef> = None;
    let mut supbp: Option<BufRef> = None;
    let mut malloced = false;

    let error: Errno = 'out: {
        let mut joliet_level = 0;
        for iso_blknum in 16..100i32 {
            // `(iso_blknum + sess) * btodb(iso_bsize)`, the C's `int` arithmetic.
            let blk = Daddr::from(iso_blknum.wrapping_add(sess).wrapping_mul(VD_DBLOCKS));
            let (t, error) = bread(&devvp.bufs, blk, iso_bsize);
            let t = bp.insert(t);
            if let Err(e) = error {
                break 'out e;
            }

            let vdp = t.bytes();
            if vdp.get(VD_ID..VD_ID + ISO_STANDARD_ID.len()) != Some(&ISO_STANDARD_ID[..]) {
                break 'out Errno::EINVAL;
            }

            match vdp.get(VD_TYPE).copied() {
                Some(ISO_VD_PRIMARY) => {
                    if pribp.is_none() {
                        pribp = bp.take();
                    }
                }
                Some(ISO_VD_SUPPLEMENTARY) => {
                    if supbp.is_none() {
                        if argp.flags & ISOFSMNT_NOJOLIET == 0 {
                            match vdp.get(SVD_ESCAPE..SVD_ESCAPE + 3) {
                                Some(b"%/@") => joliet_level = 1,
                                Some(b"%/C") => joliet_level = 2,
                                Some(b"%/E") => joliet_level = 3,
                                _ => {}
                            }

                            if vdp.get(SVD_FLAGS).is_some_and(|f| f & 1 != 0) {
                                joliet_level = 0;
                            }
                        }
                        supbp = bp.take();
                    }
                }
                Some(ISO_VD_END) => break, // goto vd_end
                _ => {}
            }
            if let Some(t) = bp.take() {
                brelse(t);
            }
        }
        // vd_end:
        if let Some(t) = bp.take() {
            brelse(t);
        }

        let Some(pri) = pribp.as_ref() else {
            break 'out Errno::EINVAL;
        };
        let (Some(lbs), Some(vss), Some(root)) = (
            field::<4>(pri.bytes(), PVD_LOGICAL_BLOCK_SIZE),
            field::<8>(pri.bytes(), PVD_VOLUME_SPACE_SIZE),
            field::<ROOT_RECORD>(pri.bytes(), PVD_ROOT),
        ) else {
            break 'out Errno::EINVAL;
        };

        let logical_block_size = i32::from(isonum_723(&lbs));

        if logical_block_size < DEV_BSIZE as i32
            || logical_block_size > MAXBSIZE as i32
            || logical_block_size & (logical_block_size - 1) != 0
        {
            break 'out Errno::EINVAL;
        }

        let Some(rootp) = IsoDirectoryRecord::new(&root) else {
            break 'out Errno::EINVAL;
        };

        // isomp = malloc(sizeof *isomp, M_ISOFSMNT, M_WAITOK): the structure is built below;
        // its export list, the mount's glue member, starts empty.
        malloced = true;
        <G::FileGlue as FileGlue>::ufs_export_init(mp);
        let mut isomp = IsoMnt {
            im_flags: 0,
            im_mountp: mp,
            im_dev: dev,
            im_devvp: devvp,
            logical_block_size,
            im_bshift: logical_block_size.trailing_zeros() as i32,
            im_bmask: logical_block_size - 1,
            // Since an ISO9660 multi-session CD can also access previous sessions, we have
            // to include them into the space considerations.
            volume_space_size: (isonum_733(&vss) as i32).wrapping_add(sess),
            root,
            root_extent: isonum_733(rootp.extent()),
            root_size: isonum_733(rootp.size()),
            iso_ftype: ISO_FTYPE_DEFAULT,
            rr_skip: 0,
            rr_skip0: 0,
            joliet_level: 0,
        };
        let root_eal = u32::from(isonum_711(rootp.ext_attr_length()));

        if let Some(t) = pribp.take() {
            brelse(t);
        }
        // The supplementary descriptor's root record, and its buffer released before the
        // Rock Ridge check (D-N5.8-R1: the module's redesign).
        let sup_root = supbp.take().and_then(|t| {
            let r = field::<ROOT_RECORD>(t.bytes(), SVD_ROOT);
            brelse(t);
            r
        });

        let typenum = mp.vfc().vfc_typenum;
        mp.with_stat(|sp| {
            sp.f_fsid.val[0] = dev;
            sp.f_fsid.val[1] = typenum;
            sp.f_namemax = NAME_MAX as u32;
        });
        mp.set_flag(mp.flag() | MNT_LOCAL);

        // Check the Rock Ridge Extension support
        if argp.flags & ISOFSMNT_NORRIP == 0 {
            let blk = Daddr::from(isomp.root_extent.wrapping_add(root_eal))
                << (isomp.im_bshift - DEV_BSHIFT as i32);
            let (t, error) = bread(&devvp.bufs, blk, isomp.logical_block_size);
            let t = bp.insert(t);
            if let Err(e) = error {
                break 'out e;
            }

            // A logical block holds a record's fixed part (`DEV_BSIZE` at least, above).
            let Some(rootp) = IsoDirectoryRecord::new(t.bytes()) else {
                break 'out Errno::EINVAL;
            };

            isomp.rr_skip = cd9660_rrip_offset(&rootp, &mut isomp, Some(blk));
            if isomp.rr_skip < 0 {
                argp.flags |= ISOFSMNT_NORRIP;
            } else {
                argp.flags &= !ISOFSMNT_GENS;
            }

            // The contents are valid, but they will get reread as part of another vnode,
            // so...
            if let Some(t) = bp.take() {
                brelse(t);
            }
        }
        isomp.im_flags =
            argp.flags & (ISOFSMNT_NORRIP | ISOFSMNT_GENS | ISOFSMNT_EXTATT | ISOFSMNT_NOJOLIET);
        isomp.iso_ftype = match isomp.im_flags & (ISOFSMNT_NORRIP | ISOFSMNT_GENS) {
            f if f == ISOFSMNT_GENS | ISOFSMNT_NORRIP => ISO_FTYPE_9660,
            0 => ISO_FTYPE_RRIP,
            _ => ISO_FTYPE_DEFAULT,
        };

        // Decide whether to use the Joliet descriptor
        if isomp.iso_ftype != ISO_FTYPE_RRIP
            && joliet_level != 0
            && let Some(root) = sup_root
            && let Some(rootp) = IsoDirectoryRecord::new(&root)
        {
            isomp.root_extent = isonum_733(rootp.extent());
            isomp.root_size = isonum_733(rootp.size());
            isomp.joliet_level = joliet_level;
            isomp.root = root;
        }

        // mp->mnt_data = isomp: the structure published, the slot's handle the mount's.
        mp.set_node(MArc::erase(isomp.alloc()));
        devvp.set_specmountpoint(Some(mp));

        return Ok(());
    };

    // out:
    if devvp.has_specinfo() {
        devvp.set_specmountpoint(None);
    }
    for t in [bp, supbp, pribp].into_iter().flatten() {
        brelse(t);
    }

    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vop_close(devvp, omode, Cred::NOCRED, Some(p));
    let _ = vop_unlock(devvp);

    // free(isomp, M_ISOFSMNT, 0): the C's request, made and freed here (the module's
    // redesign, D-N5.7-U5).
    if malloced {
        drop(MArc::new(
            M_ISOFSMNT,
            M_WAITOK,
            MaybeUninit::<IsoMnt<G>>::uninit(),
        ));
    }
    Err(error)
}

/// The volume label of an ISO 9660 disc, `iso_disklabelspoof`'s test of the device: the
/// first primary volume descriptor among blocks 16 to 99 (each a standard identifier, none
/// the terminator before it), with a valid logical block size; its `volume_id`, whose halves
/// are the spoofed label's `d_typename` and `d_packname`. `read(blkno, buf)` fills `buf`
/// ([`ISO_LABEL_HEAD`] bytes) with the first bytes of the 2,048-byte block at the disk block
/// `blkno`; any error, as any check that fails, is `EINVAL` (the C's).
pub fn iso_volume_label(
    read: &mut dyn FnMut(Daddr, &mut [u8]) -> Result<(), Errno>,
) -> Result<[u8; 32], Errno> {
    let mut vdp = [0u8; ISO_LABEL_HEAD];
    let mut vdtype = 0;
    for iso_blknum in 16..100i32 {
        if read(Daddr::from(iso_blknum * VD_DBLOCKS), &mut vdp).is_err() {
            return Err(Errno::EINVAL);
        }

        vdtype = isonum_711(&vdp[VD_TYPE..VD_TYPE + 1]);
        if vdp[VD_ID..VD_ID + ISO_STANDARD_ID.len()] != ISO_STANDARD_ID[..] || vdtype == ISO_VD_END
        {
            return Err(Errno::EINVAL);
        }

        if vdtype == ISO_VD_PRIMARY {
            break;
        }
    }

    if vdtype != ISO_VD_PRIMARY {
        return Err(Errno::EINVAL);
    }

    let logical_block_size = usize::from(isonum_723(
        &vdp[PVD_LOGICAL_BLOCK_SIZE..PVD_LOGICAL_BLOCK_SIZE + 4],
    ));
    if !(DEV_BSIZE..=MAXBSIZE).contains(&logical_block_size)
        || logical_block_size & (logical_block_size - 1) != 0
    {
        return Err(Errno::EINVAL);
    }

    let mut volume_id = [0u8; 32];
    volume_id.copy_from_slice(&vdp[PVD_VOLUME_ID..PVD_VOLUME_ID + 32]);
    Ok(volume_id)
}

/// `cd9660_start` (`vfs_start`): make a filesystem operational. Nothing to do at the
/// moment.
pub fn cd9660_start<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _flags: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    Ok(())
}

/// `cd9660_unmount` (`vfs_unmount`): unmount system call.
pub fn cd9660_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut flags = 0;
    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }
    // #if 0: mntflushbuf(mp, 0); if (mntinvalbuf(mp)) return (EBUSY);
    vflush(mp, None, flags)?;

    let isomp = vfstoisofs(mp);
    let devvp = isomp.im_devvp;

    devvp.set_specmountpoint(None);
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vop_close(devvp, FREAD, Cred::NOCRED, Some(p));
    vput(devvp);
    // free(isomp, M_ISOFSMNT, 0); mp->mnt_data = NULL: the mount's handle goes; the nodes'
    // went with `vflush`, so this call's, dropped here, is the last and frees the block.
    drop(mp.take_node());
    drop(isomp);
    mp.set_flag(mp.flag() & !MNT_LOCAL);
    Ok(())
}

/// `cd9660_root` (`vfs_root`): return root of a filesystem.
pub fn cd9660_root<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    let imp = vfstoisofs(mp);
    // A copy of 34 bytes: always a record's fixed part.
    let Some(dp) = IsoDirectoryRecord::new(&imp.root) else {
        kpanic(format_args!("cd9660_root: no root record"));
    };
    let ino = isodirino(&dp, &imp);

    // With RRIP we must use the `.' entry of the root directory. Simply tell vget, that
    // it's a relocated directory.
    cd9660_vget_internal(mp, ino, imp.iso_ftype == ISO_FTYPE_RRIP, Some(dp))
}

/// `cd9660_quotactl` (`vfs_quotactl`): do operations associated with quotas, not supported.
pub fn cd9660_quotactl() -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `cd9660_statfs` (`vfs_statfs`): get file system statistics.
pub fn cd9660_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let isomp = vfstoisofs(mp);

    sbp.f_bsize = isomp.logical_block_size as u32;
    sbp.f_iosize = sbp.f_bsize; // XXX
    sbp.f_blocks = i64::from(isomp.volume_space_size) as u64;
    sbp.f_bfree = 0; // total free blocks
    sbp.f_bavail = 0; // blocks free for non superuser
    sbp.f_files = 0; // total files
    sbp.f_ffree = 0; // free file nodes
    sbp.f_favail = 0; // file nodes free for non superuser
    copy_statfs_info(sbp, mp);

    Ok(())
}

/// `cd9660_sync` (`vfs_sync`): nothing to write back.
pub fn cd9660_sync() -> Result<(), Errno> {
    Ok(())
}

/// `cd9660_fhtovp` (`vfs_fhtovp`): file handle to vnode.
///
/// Have to be really careful about stale file handles:
/// - check that the inode number is in range
/// - call iget() to get the locked inode
/// - check for an unallocated inode (i_mode == 0)
/// - check that the generation number matches
///
/// `ifid_ino` is the `cdino_t` `cd9660_vptofh` stored, zero-extended (D-N5.8-U2, the
/// module's deviations).
pub fn cd9660_fhtovp<G: ProcGlue>(
    mp: &'static Mount<G>,
    fhp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    let ifhp = Ifid::from_fid(fhp);

    let nvp = VFS_VGET(mp, Ino::from(ifhp.ifid_ino.cast_unsigned()))?;
    let ip = vtoi(nvp);
    if ip.inode().iso_mode == 0 {
        drop(ip);
        vput(nvp);
        return Err(Errno::ESTALE);
    }
    Ok(nvp)
}

/// `cd9660_vget` (`vfs_vget`).
pub fn cd9660_vget<G: ProcGlue>(
    mp: &'static Mount<G>,
    ino: Ino,
) -> Result<&'static Vnode<G>, Errno> {
    if ino > Ino::from(Cdino::MAX) {
        kpanic(format_args!("cd9660_vget: alien ino_t {ino}"));
    }

    // XXXX
    // It would be nice if we didn't always set the `relocated' flag and force the extra
    // read, but I don't want to think about fixing that right now.
    // (#if 0: VFSTOISOFS(mp)->iso_ftype == ISO_FTYPE_RRIP.)
    cd9660_vget_internal(mp, ino as Cdino, false, None)
}

/// `cd9660_vget_internal`: the vnode of the node `ino`, referenced and locked, from the
/// node cache or read in from its directory record (`isodir`, a view the caller holds, or
/// read here). `relocated` directories take their attributes from their own `.` entry.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its error exits kept in place"
)]
pub fn cd9660_vget_internal<G: ProcGlue>(
    mp: &'static Mount<G>,
    ino: Cdino,
    relocated: bool,
    isodir: Option<IsoDirectoryRecord<'_>>,
) -> Result<&'static Vnode<G>, Errno> {
    let (imp, ip, vp) = loop {
        // retry:
        let imp = vfstoisofs(mp);
        let dev = imp.im_dev;
        if let Some(vp) = cd9660_ihashget::<G>(dev, ino) {
            return Ok(vp);
        }

        // Allocate a new vnode/iso_node.
        let vp = getnewvnode(VT_ISOFS, Some(mp), &vfs::<G>().cd9660.vops)?;
        let ip = IsoNode::new(vp, dev, ino, imp.clone()).alloc();
        vp.set_node(MArc::erase(ip.clone()));

        // Put it onto its hash chain and lock it so that other requests for this inode will
        // block if they arrive while we are sleeping waiting for old data structures to be
        // purged or for the contents of the disk portion of this inode to be read.
        match cd9660_ihashins(ip.as_pin()) {
            Ok(()) => break (imp, ip, vp),
            Err(error) => {
                drop(ip);
                vrele(vp);

                if error == Errno::EEXIST {
                    continue;
                }

                return Err(error);
            }
        }
    };

    let lbs = imp.logical_block_size as usize;
    // The C's `bp`: the device's buffer of the record, or the directory's of its `.` entry;
    // `off`, the record's offset in it; `held`, its device block when it is the device's
    // (D-N5.8-U1).
    let mut bp: Option<BufRef> = None;
    let mut off = 0;
    let mut held = None;
    if isodir.is_none() {
        let lbn = lblkno(&imp, i64::from(ino));
        if lbn >= i64::from(imp.volume_space_size) {
            drop(ip);
            vput(vp);
            printf(format_args!("fhtovp: lbn exceed volume space {lbn}\n"));
            return Err(Errno::ESTALE);
        }

        off = blkoff(&imp, i64::from(ino)) as usize;
        if off + ISO_DIRECTORY_RECORD_SIZE > lbs {
            drop(ip);
            vput(vp);
            printf(format_args!(
                "fhtovp: crosses block boundary {}\n",
                off + ISO_DIRECTORY_RECORD_SIZE
            ));
            return Err(Errno::ESTALE);
        }

        let blk = lbn << (imp.im_bshift - DEV_BSHIFT as i32);
        let (t, error) = bread(&imp.im_devvp.bufs, blk, imp.logical_block_size);
        if let Err(e) = error {
            drop(ip);
            vput(vp);
            brelse(t);
            printf(format_args!("fhtovp: bread error {}\n", e as i32));
            return Err(e);
        }

        let reclen = usize::from(t.bytes().get(off).copied().unwrap_or(0));
        if off + reclen > lbs {
            drop(ip);
            vput(vp);
            brelse(t);
            printf(format_args!(
                "fhtovp: directory crosses block boundary {}[off={off}/len={reclen}]\n",
                off + reclen
            ));
            return Err(Errno::ESTALE);
        }

        // #if 0: compare the record's start with the file handle's ifid_start.
        bp = Some(t);
        held = Some(blk);
    }

    ip.i_devvp.set(Some(imp.im_devvp));
    vref(imp.im_devvp);

    if relocated {
        // On relocated directories we must read the `.' entry out of a dir.
        ip.iso_start.set(u64::from(ino >> imp.im_bshift));
        if let Some(t) = bp.take() {
            brelse(t);
        }
        match cd9660_bufatoff(&ip, 0) {
            Ok((t, _)) => {
                bp = Some(t);
                off = 0;
                held = None;
            }
            Err(e) => {
                drop(ip);
                vput(vp);
                return Err(e);
            }
        }
    }
    let isodir = match bp.as_ref() {
        Some(t) => t.bytes().get(off..).and_then(IsoDirectoryRecord::new),
        None => isodir,
    };
    // The block's tail holds a record's fixed part: checked above, or the start of a block.
    let Some(isodir) = isodir else {
        kpanic(format_args!("cd9660_vget_internal: no directory record"));
    };

    ip.iso_extent.set(u64::from(isonum_733(isodir.extent())));
    ip.i_size.set(u64::from(isonum_733(isodir.size())));
    ip.iso_start
        .set(u64::from(isonum_711(isodir.ext_attr_length())) + ip.iso_extent.get());

    // Setup time stamp, attribute
    vp.set_vtype(VNON);
    if imp.iso_ftype == ISO_FTYPE_RRIP {
        cd9660_rrip_analyze(&isodir, &ip, &imp, held);
    } else {
        // ISO_FTYPE_9660 (and the default)
        let bp2 = extattr_buf(&isodir, &ip);
        ip.update_inode(|ino| {
            cd9660_defattr(&isodir, &ip, ino, bp2.as_ref());
            cd9660_deftstamp(&isodir, &ip, ino, bp2.as_ref());
        });
        if let Some(b2) = bp2 {
            brelse(b2);
        }
    }

    if let Some(t) = bp {
        brelse(t);
    }

    // Initialize the associated vnode
    let mut vp = vp;
    let attrs = ip.inode();
    vp.set_vtype(iftovt(u32::from(attrs.iso_mode)));
    match vp.vtype() {
        // FIFO: vp->v_op = &cd9660_fifovops (miscfs/fifofs, not ported); without FIFO:
        VFIFO => {
            drop(ip);
            vput(vp);
            return Err(Errno::EOPNOTSUPP);
        }
        VCHR | VBLK => {
            // if device, look at device number table for translation
            vp.set_ops_id(vops_id::<G>(&vfs::<G>().cd9660.specvops));
            if let Some(nvp) = checkalias(vp, attrs.iso_rdev, Some(mp)) {
                // Discard unneeded vnode, but save its iso_node. Note that the lock is
                // carried over in the iso_node
                if let Some(node) = vp.take_node() {
                    nvp.set_node(node);
                }
                vp.set_ops_id(VopsId::SPEC);
                vrele(vp);
                vgone(vp);
                // Reinitialize aliased inode.
                vp = nvp;
                ip.i_vnode.set(Some(vp));
            }
        }
        VLNK | VNON | VSOCK | VDIR | VBAD => {}
        VREG => uvm_vnp_setsize(vp, ip.i_size.get() as i64),
    }

    if ip.iso_extent.get() == u64::from(imp.root_extent) {
        vp.set_flag(vp.flag() | VROOT);
    }

    // XXX need generation number?

    Ok(vp)
}

/// `cd9660_vptofh` (`vfs_vptofh`): vnode pointer to file handle.
pub fn cd9660_vptofh<G: ProcGlue>(vp: &'static Vnode<G>, fhp: &mut Fid) -> Result<(), Errno> {
    let ip = vtoi(vp);
    Ifid {
        ifid_len: IFID_SIZE,
        ifid_pad: fhp.fid_reserved,
        ifid_ino: ip.i_number.cast_signed(),
        ifid_start: ip.iso_start.get() as i64,
    }
    .to_fid(fhp);

    Ok(())
}

/// `cd9660_vfsops`: each slot the C's function of the same name; `vfs_sysctl` the trait's
/// default (`EOPNOTSUPP`, the C's `eopnotsupp`); `vfs_checkexp` is `VFS_CHECKEXP`'s arm (the
/// module's deviations).
impl<G: ProcGlue> Vfs<G> for Cd9660Vfs<G> {
    /// `vfs_mount`: [`cd9660_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        cd9660_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: [`cd9660_start`].
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno> {
        cd9660_start(mp, flags, p)
    }

    /// `vfs_unmount`: [`cd9660_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        cd9660_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: [`cd9660_root`].
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        cd9660_root(mp)
    }

    /// `vfs_quotactl`: [`cd9660_quotactl`].
    fn quotactl(
        &self,
        _mp: &'static Mount<G>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        cd9660_quotactl()
    }

    /// `vfs_statfs`: [`cd9660_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        cd9660_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`cd9660_sync`].
    fn sync(
        &self,
        _mp: &'static Mount<G>,
        _waitfor: i32,
        _stall: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        cd9660_sync()
    }

    /// `vfs_vget`: [`cd9660_vget`].
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        cd9660_vget(mp, ino)
    }

    /// `vfs_fhtovp`: [`cd9660_fhtovp`].
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        cd9660_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: [`cd9660_vptofh`].
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        cd9660_vptofh(vp, fid)
    }

    /// `vfs_init`: `cd9660_init` (`cd9660_node.rs`).
    fn init(&self, _vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        cd9660_init::<G>()
    }
}

const _: () = {
    assert!(size_of::<Ifid>() == IFID_SIZE as usize);
    assert!(IFID_SIZE as usize <= size_of::<Fid>());
    assert!(ISO_LABEL_HEAD == 132);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the mount over testcd9660 (the makefs image and mkiso's, on testffs's
    // disk, mounted by `iso_mountfs`; LZ's moved from `bsd`): the Rock Ridge, plain ISO 9660
    // and `GENS` mounts read through the system calls, the refusals (no descriptors, a wrong
    // identifier, a bad block size), the C's `malloc` request of a refused mount (D-N5.7-U5's
    // shim), a Joliet tree, a later session, the `mount(2)` path of `cd9660_mount`, file
    // handles (D-N5.8-U2: a record past 2 GiB on a sparse disk, crafted handles), every `Vfs`
    // method, `cd9660_mountroot`'s refusals, and the label parser. User memory: testfs's path
    // slots and buffer (`testfs::up`, `UBUF`), under the kernel memory's guard every fixture
    // holds.

    use core::cell::Cell;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::sys::dirent::Dirent;
    use hal::sys::fcntl::O_RDONLY;
    use hal::sys::types::makedev;

    use super::*;
    use crate::descrip::{sys_close, sys_read};
    use crate::fs::bio::{NODEV, biodone};
    use crate::fs::buf::B_ERROR;
    use crate::fs::isofs::cd9660::testcd9660::mkiso::{Iso, record, ucs2};
    use crate::fs::isofs::cd9660::testcd9660::{
        TEST_CD9660_CONF, args, attach_image, attach_sparse, image, mount_root, try_mount,
    };
    use crate::fs::mount::vfs_mount_alloc;
    use crate::fs::namei::LOCKLEAF;
    use crate::fs::syscalls::{sys_getdents, sys_open, sys_readlink};
    use crate::fs::testfs::{self, Kind, TestGlue, UBUF, set_bdev_strategy, up};
    use crate::fs::ufs::ffs::testffs::{self, set_disk, unmount_root};
    use crate::fs::ufs::ufs::testufs::TEST_DEV;
    use crate::fs::vnode::Vattr;
    use crate::fs::vops::vop_getattr;
    use crate::testutil::{malloc_out, malloc_requests, printf_count};

    /// The whole file `path`, read through `open(2)` and `read(2)` in small pieces.
    fn read_file(p: &Thread<TestGlue>, path: &[u8]) -> Result<Vec<u8>, Errno> {
        let fd = sys_open(p, up(0, path), O_RDONLY, 0)?;
        let mut out = Vec::new();
        loop {
            let n = sys_read(p, fd, UBUF, 5)?;
            if n == 0 {
                break;
            }
            let mut b = vec![0u8; n];
            UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
            out.extend_from_slice(&b);
        }
        sys_close(p, fd)?;
        Ok(out)
    }

    /// The names `getdents(2)` gives for the directory `path`, read with a buffer of
    /// `bufsize` bytes.
    fn list_dir(p: &Thread<TestGlue>, path: &[u8], bufsize: usize) -> Vec<Vec<u8>> {
        let fd = sys_open(p, up(0, path), O_RDONLY, 0).unwrap();
        let mut names = Vec::new();
        loop {
            let n = sys_getdents(p, fd, UBUF, bufsize).unwrap();
            if n == 0 {
                break;
            }
            let mut b = vec![0u8; n];
            UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
            let mut off = 0;
            while off < n {
                let d = Dirent::from_bytes(&b[off..]).unwrap();
                let at = off + Dirent::NAME_OFFSET;
                names.push(b[at..at + usize::from(d.d_namlen)].to_vec());
                off += usize::from(d.d_reclen);
            }
        }
        sys_close(p, fd).unwrap();
        names
    }

    /// `stat(path)`: the attributes of the vnode `namei` gives.
    fn stat(p: &Thread<TestGlue>, path: &[u8]) -> Result<Vattr, Errno> {
        let mut nd = ndinit(LOOKUP, LOCKLEAF | FOLLOW, NiDirp::Sys(path), p);
        namei(&mut nd)?;
        let vp = nd.ni_vp.unwrap();
        let mut va = Vattr::new();
        let error = vop_getattr(vp, &mut va, Cred::real(p.ucred()), p);
        vput(vp);
        error.map(|()| va)
    }

    /// The vnode `path` names, locked and referenced.
    fn lookup(p: &Thread<TestGlue>, path: &[u8]) -> &'static Vnode<TestGlue> {
        let mut nd = ndinit(LOOKUP, LOCKLEAF | FOLLOW, NiDirp::Sys(path), p);
        namei(&mut nd).unwrap();
        nd.ni_vp.unwrap()
    }

    #[test]
    fn a_rock_ridge_disc_mounts_and_reads() {
        let (_g, p) = attach_image(image::image_bytes());
        let (mp, flags) = mount_root(p, 0);
        // Rock Ridge found: generation numbers are off, the format is RRIP
        assert_eq!(flags & (ISOFSMNT_NORRIP | ISOFSMNT_GENS), 0);
        let imp = vfstoisofs(mp);
        assert_eq!(imp.iso_ftype, ISO_FTYPE_RRIP);
        assert_eq!((imp.logical_block_size, imp.im_bshift), (2048, 11));
        assert_eq!((imp.root_extent, imp.rr_skip, imp.rr_skip0), (20, 0, 0));
        assert_eq!((imp.im_flags, imp.joliet_level), (0, 0));
        drop(imp);
        let st = mp.stat();
        assert_eq!((st.f_blocks, st.f_bsize, st.f_iosize), (25, 2048, 2048));
        assert_eq!(st.f_namemax, 255);
        assert_eq!(st.f_fsid.val, [TEST_DEV, 14]);
        assert_ne!(mp.flag() & MNT_LOCAL, 0);

        assert_eq!(read_file(p, b"/m10c-iso.txt").unwrap(), b"m10c-iso-42\n");
        assert_eq!(
            read_file(p, b"sub/Mixed_Case-name.long.txt").unwrap(),
            b"hello\n"
        );
        // the symbolic link, followed by namei through "..", and read by readlink(2)
        assert_eq!(read_file(p, b"/sub/link").unwrap(), b"m10c-iso-42\n");
        let n = sys_readlink(p, up(0, b"/sub/link"), UBUF, 64).unwrap();
        let mut b = vec![0u8; n];
        UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
        assert_eq!(b, b"../m10c-iso.txt");
        assert_eq!(read_file(p, b"/M10C_ISO.TXT"), Err(Errno::ENOENT));

        assert_eq!(
            list_dir(p, b"/", 4096),
            [
                b".".to_vec(),
                b"..".to_vec(),
                b"m10c-iso.txt".to_vec(),
                b"sub".to_vec()
            ]
        );
        // a buffer that holds two short entries, or the long one, at a time
        assert_eq!(list_dir(p, b"/sub", 64).len(), 4);

        let va = stat(p, b"/m10c-iso.txt").unwrap();
        assert_eq!(
            (va.va_type, va.va_mode, va.va_size, va.va_nlink),
            (VREG, 0o644, 12, 1)
        );
        assert_eq!(va.va_mtime.tv_sec, 1_791_110_208);
        let va = stat(p, b"/sub").unwrap();
        assert_eq!((va.va_type, va.va_fileid), (VDIR, 21 << 11));
        let va = stat(p, b"/").unwrap();
        assert_eq!((va.va_mode, va.va_fileid), (0o755, 20 << 11));

        unmount_root(p, mp);
    }

    #[test]
    fn a_disc_mounted_without_rock_ridge_shows_iso_names() {
        let (_g, p) = attach_image(image::image_bytes());
        let (mp, flags) = mount_root(p, ISOFSMNT_NORRIP);
        assert_eq!(flags & ISOFSMNT_NORRIP, ISOFSMNT_NORRIP);
        assert_eq!(vfstoisofs(mp).iso_ftype, ISO_FTYPE_DEFAULT);

        // ISO names, case-insensitively and without the version
        assert_eq!(read_file(p, b"/m10c_iso.txt").unwrap(), b"m10c-iso-42\n");
        assert_eq!(read_file(p, b"/M10C_ISO.TXT;1").unwrap(), b"m10c-iso-42\n");
        assert_eq!(read_file(p, b"/m10c-iso.txt"), Err(Errno::ENOENT));
        assert_eq!(
            list_dir(p, b"/SUB", 4096),
            [
                b".".to_vec(),
                b"..".to_vec(),
                b"LINK".to_vec(),
                b"MIXED_CASE_NAME.LONG_TXT".to_vec()
            ]
        );
        // plain ISO 9660 has no owners or modes: everything is readable and executable
        let va = stat(p, b"/m10c_iso.txt").unwrap();
        assert_eq!((va.va_type, va.va_mode), (VREG, 0o555));
        assert_eq!(va.va_mtime.tv_sec, 1_791_110_208);

        unmount_root(p, mp);
    }

    #[test]
    fn a_disc_mounted_with_gens_keeps_versions() {
        let (_g, p) = attach_image(image::image_bytes());
        let (mp, _) = mount_root(p, ISOFSMNT_NORRIP | ISOFSMNT_GENS);
        assert_eq!(vfstoisofs(mp).iso_ftype, ISO_FTYPE_9660);
        assert_eq!(
            list_dir(p, b"/", 4096),
            [
                b".".to_vec(),
                b"..".to_vec(),
                b"M10C_ISO.TXT;1".to_vec(),
                b"SUB".to_vec()
            ]
        );
        unmount_root(p, mp);
    }

    /// No volume descriptor, a wrong standard identifier after the primary one, a logical
    /// block size that is not a power of two: `EINVAL`, nothing hung on the mount.
    #[test]
    fn discs_without_a_valid_primary_descriptor_do_not_mount() {
        let (_g, p) = attach_image(vec![0u8; image::IMAGE_SIZE]);
        assert_eq!(try_mount(p, &mut args(0, 0)).err(), Some(Errno::EINVAL));
        let mut img = image::image_bytes();
        img[17 * 2048 + 1] = b'X';
        set_disk(img);
        assert_eq!(try_mount(p, &mut args(0, 0)).err(), Some(Errno::EINVAL));
        let mut img = image::image_bytes();
        img[16 * 2048 + 128] = 0x03; // 2051, not a power of two
        set_disk(img);
        assert_eq!(try_mount(p, &mut args(0, 0)).err(), Some(Errno::EINVAL));
        // the same disc, its block size fixed, mounts
        set_disk(image::image_bytes());
        assert!(try_mount(p, &mut args(0, 0)).is_ok());
    }

    std::thread_local! {
        /// The disk block [`failing_strategy`] fails.
        static FAIL_BLKNO: Cell<Daddr> = const { Cell::new(-1) };
    }

    /// testffs's disk, but `EIO` for the block `FAIL_BLKNO`.
    fn failing_strategy(t: BufRef) {
        if t.buf().blkno() == FAIL_BLKNO.with(Cell::get) {
            t.buf().set_error(Some(Errno::EIO));
            t.buf().set(B_ERROR);
            biodone(t);
            return;
        }
        testffs::disk_strategy(t);
    }

    /// A mount refused after the C's `malloc` of its `struct iso_mnt` (the root directory's
    /// block, read for the Rock Ridge check, fails) makes the C's one `M_ISOFSMNT` request,
    /// of the published structure's block size, and frees it (D-N5.7-U5); one refused before
    /// it (a bad block size) makes none; a mount's block is freed by its unmount.
    #[test]
    fn a_refused_mount_makes_the_c_s_malloc_request() {
        let (_g, p) = attach_image(image::image_bytes());
        let (n0, _) = malloc_requests(M_ISOFSMNT);
        let out0 = malloc_out(M_ISOFSMNT);
        let mp = try_mount(p, &mut args(0, 0)).unwrap();
        let (n1, size) = malloc_requests(M_ISOFSMNT);
        assert_eq!(n1, n0 + 1);
        assert_eq!(malloc_out(M_ISOFSMNT) - out0, size as i64);
        Cd9660Vfs::<TestGlue>::VFS.unmount(mp, 0, p).unwrap();
        assert_eq!(malloc_out(M_ISOFSMNT), out0);
        // Refused at the root block's read: the same request, of the same size, freed.
        FAIL_BLKNO.with(|c| c.set(20 << 2));
        set_bdev_strategy(Some(failing_strategy));
        assert_eq!(try_mount(p, &mut args(0, 0)).err(), Some(Errno::EIO));
        assert_eq!(malloc_requests(M_ISOFSMNT), (n1 + 1, size));
        assert_eq!(malloc_out(M_ISOFSMNT), out0);
        // Without Rock Ridge the root block is not read: the mount is made.
        let mp = try_mount(p, &mut args(ISOFSMNT_NORRIP, 0)).unwrap();
        Cd9660Vfs::<TestGlue>::VFS.unmount(mp, 0, p).unwrap();
        set_bdev_strategy(Some(testffs::disk_strategy));
        // Refused before it (a bad block size): no request.
        let mut img = image::image_bytes();
        img[16 * 2048 + 128] = 0x03;
        set_disk(img);
        assert_eq!(try_mount(p, &mut args(0, 0)).err(), Some(Errno::EINVAL));
        assert_eq!(malloc_requests(M_ISOFSMNT).0, n1 + 2);
        assert_eq!(malloc_out(M_ISOFSMNT), out0);
    }

    /// A disc with a Joliet tree (`%/E`, level 3) beside its ISO 9660 one: the primary
    /// descriptor at 16, the supplementary one at 17, the terminator at 18; the ISO root at
    /// block 20, the Joliet root at 21, the file's data at 22. `svd_flags` the
    /// supplementary descriptor's flags.
    fn joliet_image(svd_flags: u8) -> Vec<u8> {
        let mut iso = Iso::new(24);
        iso.dir(
            20,
            &[
                record(20, 2048, 2, &[0], &[]),
                record(20, 2048, 2, &[1], &[]),
                record(22, 6, 0, b"LONGNA~1.TXT;1", &[]),
            ],
        );
        iso.dir(
            21,
            &[
                record(21, 2048, 2, &[0], &[]),
                record(21, 2048, 2, &[1], &[]),
                record(22, 6, 0, &ucs2(b"Long Name.txt;1"), &[]),
            ],
        );
        iso.put(22, 0, b"jolie\n");
        iso.pvd(16, b"JOLIET", 24, &record(20, 2048, 2, &[0], &[]));
        iso.svd(17, b"%/E", svd_flags, 24, &record(21, 2048, 2, &[0], &[]));
        iso.term(18);
        iso.finish()
    }

    /// Without Rock Ridge (none on the disc: the probe sets `ISOFSMNT_NORRIP`) the Joliet
    /// tree is the root, its names the UCS-2 ones; `ISOFSMNT_NOJOLIET`, or a supplementary
    /// descriptor whose flags say its escapes are not ISO 2375's, keep the ISO 9660 tree.
    #[test]
    fn a_joliet_tree_gives_its_names() {
        let (_g, p) = attach_image(joliet_image(0));
        let (mp, flags) = mount_root(p, 0);
        assert_eq!(flags & ISOFSMNT_NORRIP, ISOFSMNT_NORRIP);
        let imp = vfstoisofs(mp);
        assert_eq!((imp.joliet_level, imp.root_extent), (3, 21));
        assert_eq!(imp.iso_ftype, ISO_FTYPE_DEFAULT);
        drop(imp);
        assert_eq!(
            list_dir(p, b"/", 4096),
            [b".".to_vec(), b"..".to_vec(), b"Long Name.txt".to_vec()]
        );
        assert_eq!(read_file(p, b"/Long Name.txt").unwrap(), b"jolie\n");
        unmount_root(p, mp);

        for (img, flags) in [(joliet_image(0), ISOFSMNT_NOJOLIET), (joliet_image(1), 0)] {
            set_disk(img);
            let (mp, _) = mount_root(p, flags);
            let imp = vfstoisofs(mp);
            assert_eq!((imp.joliet_level, imp.root_extent), (0, 20));
            drop(imp);
            assert_eq!(
                list_dir(p, b"/", 4096),
                [b".".to_vec(), b"..".to_vec(), b"LONGNA~1.TXT".to_vec()]
            );
            unmount_root(p, mp);
        }
    }

    /// A disc whose last session starts at block `sess`: its descriptors at `16 + sess`,
    /// its root directory at `18 + sess`, a file at `19 + sess`.
    fn session_image(sess: u32) -> Vec<u8> {
        let mut iso = Iso::new(sess + 20);
        iso.dir(
            sess + 18,
            &[
                record(sess + 18, 2048, 2, &[0], &[]),
                record(sess + 18, 2048, 2, &[1], &[]),
                record(sess + 19, 5, 0, b"SESS.TXT;1", &[]),
            ],
        );
        iso.put(sess + 19, 0, b"sess\n");
        iso.pvd(
            sess + 16,
            b"SESS",
            20,
            &record(sess + 18, 2048, 2, &[0], &[]),
        );
        iso.term(sess + 17);
        iso.finish()
    }

    /// `ISOFSMNT_SESS` reads the descriptors of the session at `iso_args.sess` and counts the
    /// earlier sessions in the volume's size; without it the device is asked
    /// (`CDIOREADMSADDR`, which testfs's disk does not answer: session 0); a negative
    /// session is 0.
    #[test]
    fn a_later_session_is_mounted_where_the_arguments_say() {
        let (_g, p) = attach_image(session_image(8));
        let mut a = args(ISOFSMNT_SESS, 8);
        let mp = try_mount(p, &mut a).unwrap();
        assert_eq!(mp.stat().f_blocks, 28);
        assert_eq!(vfstoisofs(mp).volume_space_size, 28);
        testffs::make_root(p, mp);
        assert_eq!(read_file(p, b"/sess.txt").unwrap(), b"sess\n");
        unmount_root(p, mp);
        assert_eq!(try_mount(p, &mut args(0, 8)).err(), Some(Errno::EINVAL));
        assert_eq!(
            try_mount(p, &mut args(ISOFSMNT_SESS, -5)).err(),
            Some(Errno::EINVAL)
        );
    }

    /// The `Nameidata` `mount(2)` hands `VFS_MOUNT` (`cd9660_mount` looks the device up in a
    /// `Nameidata` of its own).
    fn nd(p: &Thread<TestGlue>) -> Nd<'_, TestGlue> {
        ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/cd"), p)
    }

    /// `a` as the kernel copy `mount(2)` makes of the user's `struct iso_args`.
    fn data_of(a: &IsoArgs) -> Vec<u8> {
        let mut data = vec![0u8; IsoArgs::SIZE];
        assert!(hal::user::pod_to_bytes(a, &mut data));
        data
    }

    /// `mount(2)`'s path: a read-only mount only, the arguments, the device's checks, the
    /// names and `mount_info` (the flags `iso_mountfs` left), then the updates: without a
    /// device name an export update (the test glue has no export list: `EOPNOTSUPP`, which
    /// `cd9660_mount` returns), without arguments nothing.
    #[test]
    fn mount_needs_a_read_only_block_device_and_keeps_the_arguments() {
        let (_g, p) = attach_image(image::image_bytes());
        // Block devices of testfs's disk major (2), a minor of their own (a vnode of the
        // mounted device's number would be its alias, which `vcount` counts), and one of a
        // major past `nblkdev` (8).
        testfs::tree_add(b"cd1", testfs::DEV, Kind::Blk, Some(makedev(2, 22)));
        testfs::tree_add(b"wd9", testfs::DEV, Kind::Blk, Some(makedev(9, 0)));
        let mp = vfs_mount_alloc(None, &TEST_CD9660_CONF);
        let v = &Cd9660Vfs::<TestGlue>::VFS;
        let mut a = args(ISOFSMNT_GENS, 0);
        a.fspec = up(0, b"/dev/cd1").addr();
        assert_eq!(
            v.mount(mp, b"/cd", &mut data_of(&a), &mut nd(p), p),
            Err(Errno::EROFS)
        );
        mp.set_flag(mp.flag() | MNT_RDONLY);
        assert_eq!(
            v.mount(mp, b"/cd", &mut [], &mut nd(p), p),
            Err(Errno::EINVAL),
            "no iso_args"
        );
        for (fspec, want) in [
            (&b"/f"[..], Errno::ENOTBLK),
            (b"/nothere", Errno::ENOENT),
            (b"/dev/wd9", Errno::ENXIO),
        ] {
            let mut b = a;
            b.fspec = up(0, fspec).addr();
            assert_eq!(
                v.mount(mp, b"/cd", &mut data_of(&b), &mut nd(p), p),
                Err(want)
            );
        }
        assert!(!mp.has_node());

        // mount_cd9660 -g /dev/cd1 /cd: Rock Ridge found, GENS dropped
        let mut data = data_of(&a);
        v.mount(mp, b"/cd", &mut data, &mut nd(p), p).unwrap();
        let st = mp.stat();
        assert_eq!(&st.f_mntonname[..4], b"/cd\0");
        assert_eq!(&st.f_mntfromname[..9], b"/dev/cd1\0");
        assert_eq!(&st.f_mntfromspec[..9], b"/dev/cd1\0");
        let flags = offset_of!(IsoArgs, flags);
        let info = &st.mount_info.__align[..IsoArgs::SIZE];
        assert_eq!(&info[..flags], &data[..flags]);
        assert_eq!(&info[flags..flags + 4], &0i32.to_ne_bytes());
        assert_eq!(&info[flags + 4..], &data[flags + 4..]);
        assert_eq!(st.f_fsid.val, [makedev(2, 22), 14]);
        assert_eq!(st.f_blocks, 25);

        // The updates.
        mp.set_flag(mp.flag() | MNT_UPDATE);
        assert_eq!(v.mount(mp, b"/cd", &mut [], &mut nd(p), p), Ok(()));
        let mut e = args(0, 0);
        e.fspec = 0;
        assert_eq!(
            v.mount(mp, b"/cd", &mut data_of(&e), &mut nd(p), p),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(v.mount(mp, b"/cd", &mut data, &mut nd(p), p), Ok(()));
        mp.set_flag(mp.flag() & !MNT_UPDATE);
        v.unmount(mp, 0, p).unwrap();
        assert!(!mp.has_node());
        assert_eq!(mp.flag() & MNT_LOCAL, 0);
        vfs_unbusy(mp);
        vfs_mount_free(mp);
    }

    #[test]
    fn file_handles_overlay_struct_fid() {
        let ifid = Ifid {
            ifid_len: IFID_SIZE,
            ifid_pad: 0,
            ifid_ino: 41284,
            ifid_start: 23,
        };
        let mut fid = Fid::default();
        ifid.to_fid(&mut fid);
        assert_eq!(fid.fid_len, 16);
        assert_eq!(&fid.fid_data[..4], &41284i32.to_ne_bytes());
        assert_eq!(&fid.fid_data[4..12], &23i64.to_ne_bytes());
        assert_eq!(Ifid::from_fid(&fid), ifid);
    }

    /// A handle's round trip through the node cache, and crafted handles on the 25-block
    /// image: one with the top bit set names a block past the volume (`ESTALE` and the C's
    /// line, where OpenBSD panics "alien ino_t"), one a record across its block (`ESTALE`).
    #[test]
    fn file_handles_name_their_records_and_stale_ones_are_refused() {
        let (_g, p) = attach_image(image::image_bytes());
        let (mp, _) = mount_root(p, 0);
        let vp = lookup(p, b"/m10c-iso.txt");
        let mut fid = Fid::default();
        cd9660_vptofh(vp, &mut fid).unwrap();
        let ifid = Ifid::from_fid(&fid);
        assert_eq!(
            (ifid.ifid_len, ifid.ifid_ino, ifid.ifid_start),
            (16, 20 * 2048 + 324, 23)
        );
        let _ = vop_unlock(vp);
        let same = cd9660_fhtovp(mp, &fid).unwrap();
        assert!(ptr::eq(same, vp));
        vput(same);
        vrele(vp);

        let line = "fhtovp: lbn exceed volume space 1048577\n";
        let before = printf_count(line);
        let mut bad = ifid;
        bad.ifid_ino = 0x8000_0800_u32.cast_signed();
        bad.to_fid(&mut fid);
        assert_eq!(cd9660_fhtovp(mp, &fid).err(), Some(Errno::ESTALE));
        assert_eq!(printf_count(line), before + 1);
        let line = "fhtovp: crosses block boundary 2073\n";
        let before = printf_count(line);
        bad.ifid_ino = 20 * 2048 + 2040;
        bad.to_fid(&mut fid);
        assert_eq!(cd9660_fhtovp(mp, &fid).err(), Some(Errno::ESTALE));
        assert_eq!(printf_count(line), before + 1);
        unmount_root(p, mp);
    }

    /// The block 2 GiB into the disc (byte 2^31).
    const FAR: u32 = 1 << 20;

    /// A disc of `FAR + 2` blocks: the root (block 18) holds the directory `FAR`, whose data
    /// is block `FAR`; it holds `F.TXT;1`, its record at byte 68 of the block, its data the
    /// last block.
    fn far_image() -> Iso {
        let mut iso = Iso::new(FAR + 2);
        iso.dir(
            18,
            &[
                record(18, 2048, 2, &[0], &[]),
                record(18, 2048, 2, &[1], &[]),
                record(FAR, 2048, 2, b"FAR", &[]),
            ],
        );
        iso.dir(
            FAR,
            &[
                record(FAR, 2048, 2, &[0], &[]),
                record(18, 2048, 2, &[1], &[]),
                record(FAR + 1, 4, 0, b"F.TXT;1", &[]),
            ],
        );
        iso.put(FAR + 1, 0, b"far\n");
        iso.pvd(16, b"FAR", FAR + 2, &record(18, 2048, 2, &[0], &[]));
        iso.term(17);
        iso
    }

    /// D-N5.8-U2: the handle of a file whose record lies past 2 GiB carries an `ifid_ino`
    /// with the top bit set; on a fresh mount (no node cached) it names that record again,
    /// read from the disc; a handle with the top bit set past the volume is `ESTALE`.
    #[test]
    fn a_handle_past_two_gib_names_its_record() {
        let (_g, p) = attach_sparse(&far_image());
        let (mp, _) = mount_root(p, ISOFSMNT_NORRIP);
        assert_eq!(read_file(p, b"/FAR/F.TXT").unwrap(), b"far\n");
        let vp = lookup(p, b"/FAR/F.TXT");
        let mut fid = Fid::default();
        cd9660_vptofh(vp, &mut fid).unwrap();
        let ifid = Ifid::from_fid(&fid);
        assert!(ifid.ifid_ino < 0, "the C's int, negative");
        assert_eq!(ifid.ifid_ino.cast_unsigned(), (FAR << 11) + 68);
        vput(vp);
        unmount_root(p, mp);

        let (mp, _) = mount_root(p, ISOFSMNT_NORRIP);
        let vp = cd9660_fhtovp(mp, &fid).unwrap();
        let ip = vtoi(vp);
        assert_eq!((ip.i_number, ip.i_size.get()), ((FAR << 11) + 68, 4));
        assert_eq!(ip.iso_extent.get(), u64::from(FAR + 1));
        drop(ip);
        vput(vp);
        let mut bad = ifid;
        bad.ifid_ino = ((FAR + 5) << 11).cast_signed();
        bad.to_fid(&mut fid);
        assert_eq!(cd9660_fhtovp(mp, &fid).err(), Some(Errno::ESTALE));
        unmount_root(p, mp);
    }

    /// Every slot of `cd9660_vfsops` once: `start`, `quotactl`, `statfs`, `sync`, `sysctl`
    /// (the trait's default), `root`, `vget` (and its alien `ino_t`), `unmount` (forced).
    #[test]
    fn the_other_vfs_methods_answer_as_the_c() {
        let (_g, p) = attach_image(image::image_bytes());
        let mp = try_mount(p, &mut args(0, 0)).unwrap();
        let v = &Cd9660Vfs::<TestGlue>::VFS;
        assert_eq!(v.start(mp, 0, p), Ok(()));
        assert_eq!(v.quotactl(mp, 0, 0, 0, p), Err(Errno::EOPNOTSUPP));
        assert_eq!(v.sync(mp, 0, 0, Cred::NOCRED, p), Ok(()));
        let mut len = 0;
        assert_eq!(v.sysctl(&[1], 0, &mut len, 0, 0, p), Err(Errno::EOPNOTSUPP));
        let mut sb = mp.stat();
        sb.f_bfree = 7;
        v.statfs(mp, &mut sb, p).unwrap();
        assert_eq!(
            (sb.f_bsize, sb.f_iosize, sb.f_blocks, sb.f_bfree, sb.f_files),
            (2048, 2048, 25, 0, 0)
        );
        let rvp = v.root(mp).unwrap();
        assert_ne!(rvp.flag() & VROOT, 0);
        assert_eq!(vtoi(rvp).i_number, 20 << 11);
        vput(rvp);
        let vp = v.vget(mp, 20 * 2048 + 324).unwrap();
        assert_eq!(vp.vtype(), VREG);
        vput(vp);
        let alien = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = v.vget(mp, 1 << 32);
        }));
        assert!(alien.is_err(), "cd9660_vget: alien ino_t");
        assert_eq!(v.unmount(mp, MNT_FORCE, p), Ok(()));
        assert!(!mp.has_node());
    }

    /// `cd9660_mountroot` without a device for `rootdev` (`ENODEV` and the C's line), and
    /// with one but no `"cd9660"` configured (testfs's table): `vfs_rootmountalloc`'s
    /// `ENODEV`. A root mount itself is `bsd`'s test (`sys/ufs/ffs/ffs_vfsops.rs`).
    #[test]
    fn mountroot_needs_a_device_and_the_configured_type() {
        let (_g, _p) = attach_image(image::image_bytes());
        let line = "cd9660_mountroot: can't setup bdevvp's";
        let before = printf_count(line);
        assert_eq!(
            cd9660_mountroot::<TestGlue>(NODEV, NODEV),
            Err(Errno::ENODEV)
        );
        assert_eq!(printf_count(line), before + 1);
        // A device number of its own (testffs's is the disk's vnode, which bdevvp would
        // take over as an alias).
        assert_eq!(
            cd9660_mountroot::<TestGlue>(makedev(2, 23), NODEV),
            Err(Errno::ENODEV)
        );
    }

    /// The label parser over a reader of `img`: the volume id of the first primary
    /// descriptor, the blocks asked for; `EINVAL` for no descriptor, a terminator first, a
    /// bad block size and a failed read.
    #[test]
    fn the_volume_label_is_the_primary_descriptor_s() {
        let label = |img: &[u8], fail: bool| {
            let mut asked = Vec::new();
            let r = iso_volume_label(&mut |blkno, buf: &mut [u8]| {
                asked.push(blkno);
                if fail {
                    return Err(Errno::EIO);
                }
                let at = blkno as usize * 512;
                buf.copy_from_slice(&img[at..at + buf.len()]);
                Ok(())
            });
            (r, asked)
        };
        let img = image::image_bytes();
        let (r, asked) = label(&img, false);
        let id = r.unwrap();
        assert_eq!(&id[..16], b"M10C            ");
        assert_eq!(&id[16..], b"                ");
        assert_eq!(asked, [64]);
        // a supplementary descriptor first: the primary one after it
        let (r, asked) = label(&joliet_image(0)[..], false);
        assert_eq!(&r.unwrap()[..6], b"JOLIET");
        assert_eq!(asked, [64]);
        let mut svd_first = joliet_image(0);
        svd_first.copy_within(17 * 2048..18 * 2048, 15 * 2048);
        svd_first.copy_within(16 * 2048..17 * 2048, 17 * 2048);
        svd_first.copy_within(15 * 2048..16 * 2048, 16 * 2048);
        let (r, asked) = label(&svd_first, false);
        assert_eq!(&r.unwrap()[..6], b"JOLIET");
        assert_eq!(asked, [64, 68]);
        // no descriptor, the terminator first, a bad block size, a failed read
        assert_eq!(
            label(&vec![0u8; image::IMAGE_SIZE], false).0,
            Err(Errno::EINVAL)
        );
        let mut term = img.clone();
        term[16 * 2048] = 255;
        assert_eq!(label(&term, false).0, Err(Errno::EINVAL));
        let mut bad = img.clone();
        bad[16 * 2048 + 128] = 0x03;
        assert_eq!(label(&bad, false).0, Err(Errno::EINVAL));
        let (r, asked) = label(&img, true);
        assert_eq!((r, asked), (Err(Errno::EINVAL), vec![64]));
    }
}
/* </TESTS> */
