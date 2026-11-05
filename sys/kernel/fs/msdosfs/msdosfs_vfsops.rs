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
 * Copyright (C) 1994, 1995, 1997 Wolfgang Solfrank.
 * Copyright (C) 1994, 1995, 1997 TooLs GmbH.
 * All rights reserved.
 * Original code by Paul Popelka (paulp@uts.amdahl.com) (see below).
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by TooLs GmbH.
 * 4. The name of TooLs GmbH may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY TOOLS GMBH ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL TOOLS GMBH BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/*
 * Written by Paul Popelka (paulp@uts.amdahl.com)
 *
 * You can do anything you want with this software, just don't say you wrote
 * it, and don't remove this notice.
 *
 * This software is provided "as is".
 *
 * The author supplies this software to be publicly redistributed on the
 * understanding that the author is not responsible for the correct
 * functioning of this software in any circumstances and is not liable for
 * any damages caused by this software.
 *
 * October 1992
 */
/* </LICENSES> */

/* <CODE> */
//! The msdos file system's file-system-type operations (`msdosfs_vfsops`): mounting
//! (`msdosfs_mount`, and `msdosfs_mountfs`, which reads the boot sector's BIOS parameter
//! block, tells FAT12, FAT16 and FAT32 apart, checks the FAT32 FSInfo block and builds the
//! in-use cluster bitmap), unmounting, the root vnode, `statfs`, `sync` and file handles.
//!
//! Upstream: sys/msdosfs/msdosfs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/msdosfs/msdosfs_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - `msdosfs_mount` looks the device name up in a `Nameidata` of its own (`ndinit` builds
//!   one around the kernel copy of the name) instead of reinitialising the caller's `ndp`,
//!   whose lifetime cannot hold a local name, as `ffs_mount` does. A mount that is not an
//!   update and has no arguments (the C would dereference NULL) answers `EINVAL`.
//! - `disk_map`, `nblkdev`'s device switch, `rootvp` and the export list (`pm_export`) are
//!   `bsd`'s until N7 and N6, reached through the glue (`FileGlue::disk_map`,
//!   `fs::spec::devsw`, `FileGlue::rootvp`, `FileGlue::vfs_export`), as ffs's and ext2fs's:
//!   the export list is the mount's glue member, which [`msdosfs_mountfs`] empties
//!   (`FileGlue::ufs_export_init`) where the C's `malloc(M_ZERO)` of the `msdosfsmount` starts
//!   `pm_export` empty, and `msdosfs_check_export` is `VFS_CHECKEXP`'s `MOUNT_MSDOS` arm over
//!   `ufs_check_export` (`bsd`'s `sys/sys/mount.rs`), which reads only that list.
//! - `bcopy(args, &mp->mnt_stat.mount_info.msdosfs_args, ...)` copies the kernel copy of the
//!   arguments' bytes into `mount_info`.
//! - `struct msdosfs_sync_arg` is [`MsdosfsSyncArgs`] and its `allerror` a `Result`;
//!   `vfs_mount_foreach_vnode`'s callback is a closure over it. `msdosfs_sync_vnode` checks
//!   `VNON` before it looks at the denode (a vnode without a type has none) (LZ's).
//! - `ffs(x) - 1` of a power of two is `trailing_zeros` (libkern's `ffs` is core's).
//! - The `MSDOSFS_DEBUG` `printf`s and `vprint` are left out: the option is not in GENERIC
//!   and has no feature here.
//! - `vfs_quotactl` and `vfs_vget` are `eopnotsupp` in the C and answer `EOPNOTSUPP`;
//!   `vfs_sysctl` is the `Vfs` trait's default (`EOPNOTSUPP`, no `vfs.msdos` node).
//! - `msdosfs_init` is `msdosfs_denode.rs`'s, where the C defines it.
//! - [`msdosfs_mountfs`] refuses with `EINVAL`, before the read, a FAT32 file system whose
//!   FSInfo block (`fsi_size`) is larger than `MAXBSIZE` (sectors of 16 KiB or more), where
//!   OpenBSD's `bread` maps pages past its buffer's `MAXPHYS` slot (`buf_map`) and the
//!   kernel's would panic: the user's decision D-N5.7-U3 of 2026-11-05
//!   (`msdosfsmount.rs`'s `fsi_size_checked`). What userland sees: `mount_msdos` of such an
//!   image fails `EINVAL` where OpenBSD mounts it after corrupting its buffer cache.
//! - [`msdosfs_fhtovp`] answers `ESTALE`, before `deget`, for a handle whose directory entry
//!   does not lie whole inside the block `readep` would read for it (`bptoep(pmp, bp,
//!   dirofs) + 32 > blsize`: an offset within the last 31 bytes of a cluster, or past the
//!   FAT12/16 root directory's shortened last block), where the C reads past the buffer (a
//!   bogus file, or a page fault): the user's decision D-N5.7-U1 of 2026-11-05
//!   (`msdosfs_lookup.rs`'s `direntry_in_block`, after `deget`'s own conversion of FAT32's
//!   root cluster; the root directory's own position, `MSDOSFSROOT_OFS`, which `deget`
//!   manufactures, is not checked). A handle `msdosfs_vptofh` made is a real entry's position
//!   and is never refused; any other crafted handle (a cluster past `pm_maxcluster`, an
//!   offset inside the block but not on an entry) gets the C's answer. What an NFS client
//!   sees: `ESTALE` for such a handle, instead of a bogus file or a server crash.
//! - The structure is built in [`msdosfs_mountfs`]'s locals and published once filled (the
//!   module's redesign): a mount refused by a check of the boot sector, of the FSInfo block
//!   or of the bitmap, or by `fillinusemap`'s read, never allocates its `M_MSDOSFSMNT` block,
//!   where the C `malloc`s it after the boot sector's read and frees it on the error path,
//!   so `vmstat -m`'s `MSDOSFS mount` row counts one request fewer for each such refused
//!   mount; a mount that succeeds makes the C's request.
//!
//! ## Redesign
//! - LZ's `MSDOSFS_VFSOPS` (a `bsd` `Vfsops` of function pointers over `bsd`'s mount, its
//!   `vfs_checkexp` LZ's `msdosfs_check_export`) -> `impl Vfs<G> for MsdosfsVfs<G>` and its
//!   constant [`MsdosfsVfs::VFS`], which `bsd`'s `vfsconflist[]` names
//!   (`&MsdosfsVfs::<BsdProcGlue>::VFS`, N5.7c3); the functions are the C's, generic over the
//!   glue, in its order, each `Vfs` method calling its function (ext2fs's `Ext2fsVfs` shape).
//! - LZ's `msdosfs_mountfs` `malloc`ed the zeroed `struct msdosfsmount`, wrote it with
//!   `ptr::write`, made a `&'static` of it and filled it member by member through 31 `Cell`s,
//!   then cast it into `mnt_data`; its bitmap was a raw `mallocarray` freed by hand on the
//!   error path and in `msdosfs_unmount` (four `unsafe`, the boot sector and the FSInfo block
//!   read through lent buffers' `unsafe` `data()`). Here the boot sector's BPBs are read from
//!   the buffer's token by `pod_view!`'s `at` at their offset (`BSBPB_OFF`), the geometry is
//!   worked out in locals, with the C's checks in the C's order, errnos and `printf`, and in
//!   its unsigned widths (every addition, subtraction and product over an on-disk value
//!   wraps as the C's `uint32_t` does; each division is by a value an earlier check made
//!   non-zero), the bitmap is `msdosfsmount.rs`'s `inusemap_alloc` (an `MArray` of
//!   `M_MSDOSFSFAT`, `EINVAL` as the C's `M_CANFAIL`), and the structure is built whole,
//!   filled by `fillinusemap`, then published in its counted `M_MSDOSFSMNT` block
//!   (`Msdosfsmount::alloc`) and hung from the mount's typed slot where the C sets
//!   `mnt_data` (docs/IDIOMS.md, "built whole, published counted"); an error before that
//!   drops the locals (the bitmap freed with them). `msdosfs_unmount` takes the slot's handle
//!   back after `vflush` (every denode, and the handle each held, gone), and the last handle
//!   frees the block with its bitmap. 0 `unsafe` (LZ: 7 with its tests').
//! - `VFSTOMSDOSFS` is `msdosfsmount.rs`'s `vfstomsdosfs`, a counted handle per call (one
//!   count pair per `Vfs` operation reached from the mount, ROADMAP N12's row); the members
//!   `msdosfs_mount` writes after `msdosfs_mountfs` and on an update (`pm_uid`, `pm_gid`,
//!   `pm_mask`, `pm_flags`) are its relaxed words, written under the kernel lock.
//! - `msdosfs_sync_vnode` takes the C's one `splbio` pair per vnode around its dirty test,
//!   the denode's handle (a count pair) taken before it; the walk is the C's
//!   `vfs_mount_foreach_vnode`.

use core::marker::PhantomData;
use core::ptr;

use hal::intr::{IPL_BIO, splraise};
use hal::malloc::MArc;
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FREAD, FWRITE};
use hal::sys::mount::{
    Fid, MNAMELEN, MNT_FORCE, MNT_LAZY, MNT_LOCAL, MNT_RDONLY, MNT_RELOAD, MNT_SYNCHRONOUS,
    MNT_UPDATE, MNT_WAIT, MNT_WANTRDWR, MSDOSFSMNT_LONGNAME, MSDOSFSMNT_NOWIN95,
    MSDOSFSMNT_SHORTNAME, MsdosfsArgs, Statfs,
};
use hal::sys::types::{Ino, Uid, major};
use hal::user::UserPtr;

use crate::assert::kpanic;
use crate::file::{DM_OPENBLCK, FileGlue};
use crate::fs::bio::{bread, brelse};
use crate::fs::buf::{B_INVAL, BufRef};
use crate::fs::lookup::{namei, ndinit};
use crate::fs::mount::{Mount, Vfsconf};
use crate::fs::msdosfs::bootsect::BSBPB_OFF;
use crate::fs::msdosfs::bpb::{
    Bpb50, ByteBpb33, ByteBpb50, ByteBpb710, FATMIRROR, FATNUM, Fsinfo, getulong, getushort,
};
use crate::fs::msdosfs::denode::{
    DE_ACCESS, DE_CREATE, DE_MODIFIED, DE_UPDATE, Defid, MSDOSFSROOT_OFS, WIN_MAXLEN, vtode,
};
use crate::fs::msdosfs::direntry::Direntry;
use crate::fs::msdosfs::fat::{
    CLUST_FIRST, CLUST_RSRVD, FAT12_MASK, FAT16_MASK, FAT32_MASK, MSDOSFSROOT,
};
use crate::fs::msdosfs::msdosfs_denode::{deget, msdosfs_init};
use crate::fs::msdosfs::msdosfs_fat::fillinusemap;
use crate::fs::msdosfs::msdosfs_lookup::direntry_in_block;
use crate::fs::msdosfs::msdosfsmount::{
    MSDOSFS_FATMIRROR, MSDOSFSMNT_MNTOPT, MSDOSFSMNT_RONLY, MSDOSFSMNT_WAITONFAT, Msdosfsmount,
    fsi_size_checked, inusemap_alloc, vfstomsdosfs,
};
use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
use crate::fs::spec::devsw;
use crate::fs::subr::{
    copy_statfs_info, vcount, vflush, vfs_mount_foreach_vnode, vfs_mountedon, vget, vinvalbuf,
    vput, vrele,
};
use crate::fs::ufs::ffs::ffs_vfsops::mname_copy;
use crate::fs::vnode::{
    FORCECLOSE, LK_EXCLUSIVE, LK_NOWAIT, LK_RETRY, V_SAVE, VBLK, VNON, Vnode, WRITECLOSE, vn_lock,
};
use crate::fs::vops::{
    Cred, Nd, VFS_SYNC, Vfs, vop_close, vop_fsync, vop_ioctl, vop_open, vop_unlock,
};
use crate::fs::word::U32Word;
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};
use crate::vm::io::MAXBSIZE;

/// `DEV_BSIZE`: the size of a disk block, the unit of the geometry's block numbers.
const DEV_BSIZE: u32 = 512;

/// `DIOCCACHESYNC` (`<sys/dkio.h>`, `_IOW('d', 120, int)`): sync the disk's write cache,
/// which `msdosfs_mount` asks of the device when it goes read-only (the device may not
/// support it).
const DIOCCACHESYNC: u64 = hal::sys::ioccom::_iow::<i32>(b'd', 120);

/// The msdos file system of the glue `G`: `msdosfs_vfsops` ([`MsdosfsVfs::VFS`]). Its vnode
/// table is the glue's `MsdosfsTables::vops` (`msdosfs_vnops.rs`).
pub struct MsdosfsVfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> MsdosfsVfs<G> {
    /// `msdosfs_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"msdos"`).
    pub const VFS: MsdosfsVfs<G> = MsdosfsVfs(PhantomData);
}

/// `struct msdosfs_sync_arg`: what `msdosfs_sync` hands `msdosfs_sync_vnode` for each vnode.
pub struct MsdosfsSyncArgs<'a, G: ProcGlue> {
    /// `p`: the thread syncing.
    pub p: &'a Thread<G>,
    /// `cred`: its credentials.
    pub cred: Cred<'a>,
    /// `allerror`: the last error of a vnode's `VOP_FSYNC`.
    pub allerror: Result<(), Errno>,
    /// `waitfor`: `MNT_WAIT`, `MNT_NOWAIT` or `MNT_LAZY`.
    pub waitfor: i32,
}

/// `msdosfs_mount` (`vfs_mount`): mount a msdos file system at `path`. `data` is the kernel
/// copy of the user's `struct msdosfs_args` (empty for the C's NULL), whose `fspec` names
/// the block special file to treat as a filesystem.
pub fn msdosfs_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    _ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let args = MsdosfsArgs::from_bytes(data);
    let mut fname = [0u8; MNAMELEN];
    let mut fspec = [0u8; MNAMELEN];
    let mut updpmp = None;

    // If updating, check whether changing from read-only to read/write; if there is no
    // device name, that's all we do.
    if mp.flag() & MNT_UPDATE != 0 {
        let pmp = updpmp.insert(vfstomsdosfs(mp));
        let mut error = Ok(());
        if pmp.pm_flags.get() & MSDOSFSMNT_RONLY == 0 && mp.flag() & MNT_RDONLY != 0 {
            mp.set_flag(mp.flag() & !MNT_RDONLY);
            let _ = VFS_SYNC(mp, MNT_WAIT, 0, Cred::real(p.ucred()), p);
            mp.set_flag(mp.flag() | MNT_RDONLY);

            let mut flags = WRITECLOSE;
            if mp.flag() & MNT_FORCE != 0 {
                flags |= FORCECLOSE;
            }
            error = vflush(mp, None, flags);
            if error.is_ok() {
                let mut force = 0i32.to_ne_bytes();

                pmp.pm_flags.set(pmp.pm_flags.get() | MSDOSFSMNT_RONLY);
                // may be not supported, ignore error
                let _ = vop_ioctl(
                    pmp.pm_devvp,
                    DIOCCACHESYNC,
                    &mut force,
                    FWRITE,
                    Cred::FSCRED,
                    p,
                );
            }
        }
        if error.is_ok() && mp.flag() & MNT_RELOAD != 0 {
            // not yet implemented
            error = Err(Errno::EOPNOTSUPP);
        }
        error?;
        if pmp.pm_flags.get() & MSDOSFSMNT_RONLY != 0 && mp.flag() & MNT_WANTRDWR != 0 {
            pmp.pm_flags.set(pmp.pm_flags.get() & !MSDOSFSMNT_RONLY);
        }

        match &args {
            // Process export requests.
            Some(a) if a.fspec == 0 => {
                return <G::FileGlue as FileGlue>::vfs_export(mp, &a.export_info);
            }
            None => return Ok(()),
            Some(_) => {}
        }
    }

    // Not an update, or updating the name: look up the name and verify that it refers to a
    // sensible block device.
    let Some(args) = args else {
        return Err(Errno::EINVAL);
    };
    UserPtr::<u8>::new(args.fspec).copyinstr(&mut fspec)?;

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

        match &updpmp {
            None => {
                if let Err(e) = msdosfs_mountfs(devvp, mp, p, &args) {
                    break 'error_devvp e;
                }
            }
            Some(pmp) => {
                if !ptr::eq(devvp, pmp.pm_devvp) {
                    break 'error_devvp Errno::EINVAL; // XXX needs translation
                }
                vrele(devvp);
            }
        }

        let pmp = vfstomsdosfs(mp);
        pmp.pm_gid.set(args.gid);
        pmp.pm_uid.set(args.uid);
        pmp.pm_mask.set(args.mask);
        pmp.pm_flags
            .set(pmp.pm_flags.get() | (args.flags as u32 & MSDOSFSMNT_MNTOPT));

        if pmp.pm_flags.get() & MSDOSFSMNT_NOWIN95 as u32 != 0 {
            pmp.pm_flags
                .set(pmp.pm_flags.get() | MSDOSFSMNT_SHORTNAME as u32);
        } else if pmp.pm_flags.get() & (MSDOSFSMNT_SHORTNAME | MSDOSFSMNT_LONGNAME) as u32 == 0 {
            pmp.pm_flags
                .set(pmp.pm_flags.get() | MSDOSFSMNT_LONGNAME as u32);
        }

        let longname = pmp.pm_flags.get() & MSDOSFSMNT_LONGNAME as u32 != 0;
        mp.with_stat(|sp| {
            sp.f_namemax = if longname { WIN_MAXLEN as u32 } else { 12 };
            mname_copy(&mut sp.f_mntonname, path);
            mname_copy(&mut sp.f_mntfromname, &fname);
            mname_copy(&mut sp.f_mntfromspec, &fspec);
            sp.mount_info.__align[..MsdosfsArgs::SIZE].copy_from_slice(&data[..MsdosfsArgs::SIZE]);
        });

        return Ok(());
    };

    // error_devvp:
    vrele(devvp);
    Err(error)
}

/// `msdosfs_mountfs(devvp, mp, p, argp)`: mount the msdos file system on the block device
/// `devvp` at `mp`: read and check the boot sector's BIOS parameter block, work out the FAT
/// type and geometry, check the FAT32 FSInfo block, and fill the in-use cluster bitmap. On
/// success the mount's typed slot holds the new structure (`mp->mnt_data`) and `devvp`'s
/// reference is the mount's.
pub fn msdosfs_mountfs<G: ProcGlue>(
    devvp: &'static Vnode<G>,
    mp: &'static Mount<G>,
    p: &Thread<G>,
    _argp: &MsdosfsArgs,
) -> Result<(), Errno> {
    let dev = devvp.rdev();

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

    let ronly = mp.flag() & MNT_RDONLY != 0;
    let omode = if ronly { FREAD } else { FREAD | FWRITE };
    vop_open(devvp, omode, Cred::FSCRED, p)?;

    // Used in error_exit (the C's `bp`; its `pmp` is not made before the last step).
    let mut bp: Option<BufRef> = None;

    let error: Errno = 'error_exit: {
        // Read the boot sector of the filesystem, and then check the boot signature. If not
        // a dos boot sector then error out.
        let (t, error) = bread(&devvp.bufs, 0, 4096);
        let t = bp.insert(t);
        if let Err(e) = error {
            break 'error_exit e;
        }
        // The three BPBs of the boot sector's union, at `bsBPB`'s offset: a 4096-byte buffer
        // always holds them.
        let (Some(b33), Some(b50), Some(b710)) = (
            ByteBpb33::at(t.bytes(), BSBPB_OFF),
            ByteBpb50::at(t.bytes(), BSBPB_OFF),
            ByteBpb710::at(t.bytes(), BSBPB_OFF),
        ) else {
            kpanic(format_args!(
                "msdosfs_mountfs: a boot sector short of its BPB"
            ));
        };

        // pmp = malloc(sizeof *pmp, M_MSDOSFSMNT, M_WAITOK | M_ZERO): the structure is built
        // below; its export list, the mount's glue member, starts empty.
        <G::FileGlue as FileGlue>::ufs_export_init(mp);
        let mut pm_flags = 0;
        let mut pm_curfat = 0;
        let mut pm_fatmask = 0;
        let mut pm_fatmult = 0;
        let mut pm_fatdiv = 0;
        let mut pm_fsinfo = 0;
        let mut pm_rootdirsize = 0;

        // Compute several useful quantities from the bpb in the bootsector. Copy in the dos 5
        // variant of the bpb then fix up the fields that are different between dos 5 and dos
        // 3.3.
        let mut sec_per_clust = u32::from(b50.bpbSecPerClust);
        let mut bpb = Bpb50 {
            bpbBytesPerSec: getushort(&b50.bpbBytesPerSec),
            bpbResSectors: getushort(&b50.bpbResSectors),
            bpbFATs: b50.bpbFATs,
            bpbRootDirEnts: getushort(&b50.bpbRootDirEnts),
            bpbSectors: getushort(&b50.bpbSectors),
            bpbSecPerTrack: getushort(&b50.bpbSecPerTrack),
            bpbHeads: getushort(&b50.bpbHeads),
            bpbMedia: b50.bpbMedia,
            // `pm_bpb`'s `bpbSecPerClust` and `bpbFATsecs` stay zero, as the C's: it keeps
            // them in `SecPerClust` and `pm_FATsecs`.
            ..Bpb50::default()
        };
        let mut pm_fatsecs = u32::from(getushort(&b50.bpbFATsecs));

        // Determine the number of DEV_BSIZE blocks in a MSDOSFS sector
        let blkpersec = u32::from(bpb.bpbBytesPerSec) / DEV_BSIZE;

        if bpb.bpbBytesPerSec == 0 || sec_per_clust == 0 {
            break 'error_exit Errno::EINVAL;
        }

        if bpb.bpbSectors == 0 {
            bpb.bpbHiddenSecs = getulong(&b50.bpbHiddenSecs);
            bpb.bpbHugeSectors = getulong(&b50.bpbHugeSectors);
        } else {
            bpb.bpbHiddenSecs = u32::from(getushort(&b33.bpbHiddenSecs));
            bpb.bpbHugeSectors = u32::from(bpb.bpbSectors);
        }

        if bpb.bpbRootDirEnts == 0 {
            if bpb.bpbSectors != 0 || pm_fatsecs != 0 || getushort(&b710.bpbFSVers) != 0 {
                break 'error_exit Errno::EINVAL;
            }
            pm_fatmask = FAT32_MASK;
            pm_fatmult = 4;
            pm_fatdiv = 1;
            pm_fatsecs = getulong(&b710.bpbBigFATsecs);
            let extflags = getushort(&b710.bpbExtFlags);
            if extflags & FATMIRROR != 0 {
                pm_curfat = u32::from(extflags & FATNUM);
            } else {
                pm_flags |= MSDOSFS_FATMIRROR;
            }
        } else {
            pm_flags |= MSDOSFS_FATMIRROR;
        }

        // More sanity checks:
        //	MSDOSFS sectors per cluster: >0 && power of 2
        //	MSDOSFS sector size: >= DEV_BSIZE && power of 2
        //	HUGE sector count: >0
        //	FAT sectors: >0
        let bytes_per_sec = u32::from(bpb.bpbBytesPerSec);
        if sec_per_clust == 0
            || !sec_per_clust.is_power_of_two()
            || bytes_per_sec < DEV_BSIZE
            || !bytes_per_sec.is_power_of_two()
            || bpb.bpbHugeSectors == 0
            || pm_fatsecs == 0
            || sec_per_clust * blkpersec > MAXBSIZE as u32 / DEV_BSIZE
        {
            break 'error_exit Errno::EINVAL;
        }

        bpb.bpbHugeSectors = bpb.bpbHugeSectors.wrapping_mul(blkpersec);
        bpb.bpbHiddenSecs = bpb.bpbHiddenSecs.wrapping_mul(blkpersec);
        pm_fatsecs = pm_fatsecs.wrapping_mul(blkpersec);
        // At most 65,535 reserved sectors of 128 blocks: no wrap.
        let pm_fatblk = u32::from(bpb.bpbResSectors) * blkpersec;
        // At most MAXBSIZE / DEV_BSIZE (checked above), as the C's u_int8_t holds it.
        sec_per_clust *= blkpersec;

        let fats_size = u32::from(bpb.bpbFATs).wrapping_mul(pm_fatsecs);
        let (pm_rootdirblk, pm_firstcluster);
        if pm_fatmask == FAT32_MASK {
            pm_rootdirblk = getulong(&b710.bpbRootClust);
            pm_firstcluster = pm_fatblk.wrapping_add(fats_size);
            // At most 65,535 sectors of 128 blocks: no wrap.
            pm_fsinfo = u32::from(getushort(&b710.bpbFSInfo)) * blkpersec;
        } else {
            pm_rootdirblk = pm_fatblk.wrapping_add(fats_size);
            // At most 65,535 entries of 32 bytes: no wrap.
            pm_rootdirsize =
                (u32::from(bpb.bpbRootDirEnts) * Direntry::SIZE as u32).div_ceil(DEV_BSIZE);
            pm_firstcluster = pm_rootdirblk.wrapping_add(pm_rootdirsize);
        }

        let pm_nmbrofclusters = bpb.bpbHugeSectors.wrapping_sub(pm_firstcluster) / sec_per_clust;
        let mut pm_maxcluster = pm_nmbrofclusters.wrapping_add(1);
        let pm_fatsize = pm_fatsecs.wrapping_mul(DEV_BSIZE);

        if pm_fatmask == 0 {
            if pm_maxcluster <= (CLUST_RSRVD - CLUST_FIRST) & FAT12_MASK {
                // This will usually be a floppy disk. This size makes sure that one fat entry
                // will not be split across multiple blocks.
                pm_fatmask = FAT12_MASK;
                pm_fatmult = 3;
                pm_fatdiv = 2;
            } else {
                pm_fatmask = FAT16_MASK;
                pm_fatmult = 2;
                pm_fatdiv = 1;
            }
        }
        // At most three sectors of 32 KiB: no wrap.
        let pm_fatblocksize = if pm_fatmask == FAT12_MASK {
            3 * bytes_per_sec
        } else {
            MAXBSIZE as u32
        };

        // We now have the number of sectors in each FAT, so can work out how many clusters
        // can be represented in a FAT. Let's make sure the file system doesn't claim to have
        // more clusters than this.
        //
        // We perform the calculation like we do to avoid integer overflow.
        //
        // This will give us a count of clusters. They are numbered from 0, so the max cluster
        // value is one less than the value we end up with.
        let fat_max_clusters = (pm_fatsize / pm_fatmult).wrapping_mul(pm_fatdiv);
        if pm_maxcluster >= fat_max_clusters {
            // `%d` of the C's `uint32_t`s: their bits as an `int`.
            printf(format_args!(
                "msdosfs: reducing max cluster to {} from {} due to FAT size\n",
                fat_max_clusters.wrapping_sub(1) as i32,
                pm_maxcluster as i32
            ));
            pm_maxcluster = fat_max_clusters.wrapping_sub(1);
        }

        let pm_fatblocksec = pm_fatblocksize / DEV_BSIZE;
        let pm_bnshift = DEV_BSIZE.trailing_zeros();

        // Compute mask and shift value for isolating cluster relative byte offsets and
        // cluster numbers from a file offset.
        let pm_bpcluster = sec_per_clust * DEV_BSIZE;
        let pm_crbomask = pm_bpcluster - 1;
        let pm_cnshift = pm_bpcluster.trailing_zeros();

        // Check for valid cluster size; must be a power of 2
        if pm_bpcluster ^ (1 << pm_cnshift) != 0 {
            break 'error_exit Errno::EINVAL;
        }

        // Release the bootsector buffer.
        if let Some(t) = bp.take() {
            brelse(t);
        }

        // Check FSInfo
        if pm_fsinfo != 0 {
            // A block larger than the cache maps is refused (the module's deviations,
            // D-N5.7-U3).
            let size = match fsi_size_checked(blkpersec) {
                Ok(size) => size,
                Err(e) => break 'error_exit e,
            };
            let (t, error) = bread(&devvp.bufs, i64::from(pm_fsinfo), size);
            let t = bp.insert(t);
            if let Err(e) = error {
                break 'error_exit e;
            }
            // A block of at least 1024 bytes always holds the structure.
            let Some(fp) = Fsinfo::at(t.bytes(), 0) else {
                kpanic(format_args!(
                    "msdosfs_mountfs: an FSInfo block short of its 1024 bytes"
                ));
            };
            if &fp.fsisig1 == b"RRaA"
                && &fp.fsisig2 == b"rrAa"
                && fp.fsisig3 == [0, 0, 0o125, 0o252]
                && fp.fsisig4 == [0, 0, 0o125, 0o252]
            {
                // Valid FSInfo.
            } else {
                pm_fsinfo = 0;
            }
            // XXX make sure this tiny buf doesn't come back in fillinusemap!
            if let Some(t) = bp.take() {
                t.buf().set(B_INVAL);
                brelse(t);
            }
        }

        // Check and validate (or perhaps invalidate?) the fsinfo structure? XXX

        // Allocate memory for the bitmap of allocated clusters, and then fill it in.
        let pm_inusemap = match inusemap_alloc(pm_maxcluster) {
            Ok(map) => map,
            Err(e) => break 'error_exit e,
        };

        // fillinusemap() needs pm_devvp: the structure, whole but not yet published.
        let pm = Msdosfsmount {
            pm_mountp: mp,
            pm_dev: dev,
            pm_uid: U32Word::new(0),
            pm_gid: U32Word::new(0),
            pm_mask: U32Word::new(0),
            pm_devvp: devvp,
            pm_bpb: bpb,
            pm_BlkPerSec: blkpersec,
            pm_FATsecs: pm_fatsecs,
            pm_fatblk,
            pm_rootdirblk,
            pm_rootdirsize,
            pm_firstcluster,
            pm_nmbrofclusters,
            pm_maxcluster,
            pm_freeclustercount: U32Word::new(0),
            pm_cnshift,
            pm_crbomask,
            pm_bnshift,
            pm_bpcluster,
            pm_fmod: U32Word::new(0),
            pm_fatblocksize,
            pm_fatblocksec,
            pm_fatsize,
            pm_fatmask,
            pm_fsinfo: U32Word::new(pm_fsinfo),
            pm_fatmult,
            pm_fatdiv,
            pm_curfat,
            pm_inusemap,
            pm_flags: U32Word::new(pm_flags),
        };

        // Have the inuse map filled in.
        if let Err(e) = fillinusemap(&pm) {
            break 'error_exit e;
        }

        // If they want fat updates to be synchronous then let them suffer the performance
        // degradation in exchange for the on disk copy of the fat being correct just about
        // all the time. I suppose this would be a good thing to turn on if the kernel is
        // still flakey.
        if mp.flag() & MNT_SYNCHRONOUS != 0 {
            pm.pm_flags.set(pm.pm_flags.get() | MSDOSFSMNT_WAITONFAT);
        }

        // Finish up.
        if ronly {
            pm.pm_flags.set(pm.pm_flags.get() | MSDOSFSMNT_RONLY);
        } else {
            pm.pm_fmod.set(1);
        }
        // mp->mnt_data = pmp: the structure published, the slot's handle the mount's.
        mp.set_node(MArc::erase(pm.alloc()));
        mp.with_stat(|sp| {
            sp.f_fsid.val[0] = dev;
            sp.f_fsid.val[1] = mp.vfc().vfc_typenum;
        });
        // QUOTA: if we ever do quotas for DOS filesystems this would be a place to fill in
        // the info in the msdosfsmount structure. You dolt, quotas on dos filesystems make no
        // sense because files have no owners on dos filesystems. of course there is some
        // empty space in the directory entry where we could put uid's and gid's.
        devvp.set_specmountpoint(Some(mp));

        return Ok(());
    };

    // error_exit: the structure, if it was built, and its bitmap went with the block above.
    if devvp.has_specinfo() {
        devvp.set_specmountpoint(None);
    }
    if let Some(t) = bp {
        brelse(t);
    }

    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vop_close(devvp, omode, Cred::NOCRED, Some(p));
    let _ = vop_unlock(devvp);
    Err(error)
}

/// `msdosfs_start` (`vfs_start`): make a filesystem operational; nothing to do.
pub fn msdosfs_start<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _flags: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    Ok(())
}

/// `msdosfs_unmount` (`vfs_unmount`): unmount the filesystem described by `mp`.
pub fn msdosfs_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut flags = 0;
    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }
    vflush(mp, None, flags)?;
    let pmp = vfstomsdosfs(mp);
    pmp.pm_devvp.set_specmountpoint(None);
    let vp = pmp.pm_devvp;
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let omode = if pmp.pm_flags.get() & MSDOSFSMNT_RONLY != 0 {
        FREAD
    } else {
        FREAD | FWRITE
    };
    let _ = vop_close(vp, omode, Cred::NOCRED, Some(p));
    vput(vp);
    // free(pmp->pm_inusemap); free(pmp); mp->mnt_data = NULL: the mount's handle goes; the
    // denodes' went with `vflush`, so this call's, dropped here, is the last and frees the
    // structure with its bitmap.
    drop(mp.take_node());
    drop(pmp);
    mp.set_flag(mp.flag() & !MNT_LOCAL);
    Ok(())
}

/// `msdosfs_root` (`vfs_root`): the root directory's vnode, referenced and locked.
pub fn msdosfs_root<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    let pmp = vfstomsdosfs(mp);
    let ndep = deget(&pmp, MSDOSFSROOT, MSDOSFSROOT_OFS)?;
    Ok(ndep.detov())
}

/// `msdosfs_statfs` (`vfs_statfs`): get file system statistics.
pub fn msdosfs_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let pmp = vfstomsdosfs(mp);
    sbp.f_bsize = pmp.pm_bpcluster;
    sbp.f_iosize = pmp.pm_bpcluster;
    sbp.f_blocks = u64::from(pmp.pm_nmbrofclusters);
    sbp.f_bfree = u64::from(pmp.pm_freeclustercount.get());
    sbp.f_bavail = i64::from(pmp.pm_freeclustercount.get());
    sbp.f_files = u64::from(pmp.pm_RootDirEnts()); // XXX
    sbp.f_ffree = 0; // what to put in here?
    sbp.f_favail = 0;
    copy_statfs_info(sbp, mp);

    Ok(())
}

/// `msdosfs_sync_vnode(vp, arg)`: write back one (modified) denode for `msdosfs_sync`.
pub fn msdosfs_sync_vnode<G: ProcGlue>(
    vp: &'static Vnode<G>,
    msa: &mut MsdosfsSyncArgs<'_, G>,
) -> Result<(), Errno> {
    // A vnode without a type has no denode (the module's deviations).
    let dep = (vp.vtype() != VNON).then(|| vtode(vp));
    let s = splraise(IPL_BIO);
    let skip = dep.as_ref().is_none_or(|dep| {
        dep.de_flag.get() & (DE_ACCESS | DE_CREATE | DE_UPDATE | DE_MODIFIED) == 0
            && vp.bufs.dirty_is_empty()
    }) || msa.waitfor == MNT_LAZY;
    drop(s);
    drop(dep);

    if skip {
        return Ok(());
    }

    if vget(vp, LK_EXCLUSIVE | LK_NOWAIT).is_err() {
        return Ok(());
    }

    if let Err(error) = vop_fsync(vp, msa.cred, msa.waitfor, msa.p) {
        msa.allerror = Err(error);
    }
    let _ = vop_unlock(vp);
    vrele(vp);

    Ok(())
}

/// `msdosfs_sync` (`vfs_sync`): write back the modified denodes and the device's dirty
/// buffers.
pub fn msdosfs_sync<G: ProcGlue>(
    mp: &'static Mount<G>,
    waitfor: i32,
    _stall: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let pmp = vfstomsdosfs(mp);
    let mut msa = MsdosfsSyncArgs {
        p,
        cred,
        allerror: Ok(()),
        waitfor,
    };

    // If we ever switch to not updating all of the fats all the time, this would be the
    // place to update them from the first one.
    if pmp.pm_fmod.get() != 0 {
        if pmp.pm_flags.get() & MSDOSFSMNT_RONLY != 0 {
            kpanic(format_args!("msdosfs_sync: rofs mod"));
        } else {
            // update fats here
        }
    }
    // Write back each (modified) denode.
    let _ = vfs_mount_foreach_vnode(mp, &mut |vp| msdosfs_sync_vnode(vp, &mut msa));

    // Force stale file system control information to be flushed.
    if waitfor != MNT_LAZY {
        let devvp = pmp.pm_devvp;
        let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
        if let Err(error) = vop_fsync(devvp, cred, waitfor, p) {
            msa.allerror = Err(error);
        }
        let _ = vop_unlock(devvp);
    }

    msa.allerror
}

/// `msdosfs_fhtovp` (`vfs_fhtovp`): the vnode, locked, of the file handle `fhp`. `ESTALE` for
/// a handle whose entry does not lie inside the block `deget` would read for it (the
/// module's deviations, D-N5.7-U1).
pub fn msdosfs_fhtovp<G: ProcGlue>(
    mp: &'static Mount<G>,
    fhp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    let pmp = vfstomsdosfs(mp);
    let defhp = Defid::from_fid(fhp);

    // The entry `deget` would read: FAT32's root directory is its cluster there; the root
    // directory's own position has no entry (`deget` manufactures it).
    let fat32 = pmp.pm_fatmask == FAT32_MASK;
    let mut dirclust = defhp.defid_dirclust;
    if fat32 && dirclust == MSDOSFSROOT {
        dirclust = pmp.pm_rootdirblk;
    }
    let root = (dirclust == MSDOSFSROOT || (fat32 && dirclust == pmp.pm_rootdirblk))
        && defhp.defid_dirofs == MSDOSFSROOT_OFS;
    if !root && !direntry_in_block(&pmp, dirclust, defhp.defid_dirofs) {
        return Err(Errno::ESTALE);
    }

    let dep = deget(&pmp, defhp.defid_dirclust, defhp.defid_dirofs)?;
    Ok(dep.detov())
}

/// `msdosfs_vptofh` (`vfs_vptofh`): the file handle of `vp`, the position of its directory
/// entry.
pub fn msdosfs_vptofh<G: ProcGlue>(vp: &'static Vnode<G>, fhp: &mut Fid) -> Result<(), Errno> {
    let dep = vtode(vp);
    let mut defhp = Defid::from_fid(fhp);
    defhp.defid_len = size_of::<Defid>() as u16;
    defhp.defid_dirclust = dep.de_dirclust.get();
    defhp.defid_dirofs = dep.de_diroffset.get();
    // defhp->defid_gen = dep->de_gen;
    defhp.to_fid(fhp);
    Ok(())
}

/// `msdosfs_vfsops`: each slot the C's function of the same name; `vfs_quotactl` and
/// `vfs_vget` the C's `eopnotsupp`, `vfs_sysctl` the trait's default (`EOPNOTSUPP`).
impl<G: ProcGlue> Vfs<G> for MsdosfsVfs<G> {
    /// `vfs_mount`: [`msdosfs_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: [`msdosfs_start`].
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno> {
        msdosfs_start(mp, flags, p)
    }

    /// `vfs_unmount`: [`msdosfs_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        msdosfs_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: [`msdosfs_root`].
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        msdosfs_root(mp)
    }

    /// `vfs_quotactl`: `eopnotsupp`.
    fn quotactl(
        &self,
        _mp: &'static Mount<G>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }

    /// `vfs_statfs`: [`msdosfs_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        msdosfs_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`msdosfs_sync`].
    fn sync(
        &self,
        mp: &'static Mount<G>,
        waitfor: i32,
        stall: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        msdosfs_sync(mp, waitfor, stall, cred, p)
    }

    /// `vfs_vget`: `eopnotsupp`.
    fn vget(&self, _mp: &'static Mount<G>, _ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }

    /// `vfs_fhtovp`: [`msdosfs_fhtovp`].
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        msdosfs_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: [`msdosfs_vptofh`].
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        msdosfs_vptofh(vp, fid)
    }

    /// `vfs_init`: `msdosfs_init` (`msdosfs_denode.rs`).
    fn init(&self, _vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        msdosfs_init::<G>()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the mount over testmsdosfs (`mkfat`'s FAT12, FAT16 and FAT32 images on
    // testffs's disk, mounted by `msdosfs_mountfs`, moved from `bsd`'s tests): the geometry it
    // works out and `statfs`, the FAT32 FSInfo block and active FAT, the bad boot sectors, the
    // FAT size's clamp and its `printf`, the FSInfo block's size bound (D-N5.7-U3), the
    // `mount(2)` path of `msdosfs_mount` (its arguments, its errors, the updates, the export
    // update through the glue), file handles (D-N5.7-U1) and every `Vfs` method. User memory:
    // testfs's path slots and buffer (`testfs::up`, `USER_BASE + 0xe000..0xf000`), under the
    // kernel memory's guard every fixture holds.

    use core::cell::Cell;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::sys::mount::{MNT_EXPORTED, MSDOSFSMNT_LONGNAME, MSDOSFSMNT_NOWIN95};
    use hal::sys::types::{Daddr, makedev};

    use super::super::fat::{fat12, fat16, fat32};
    use super::super::msdosfsmount::MsdosfsmountRef;
    use super::super::testmsdosfs::{
        TEST_MSDOS_CONF, attach, full_root, mkfat, noargs, root, try_mount, unmount,
    };
    use super::*;
    use crate::fs::bio::biodone;
    use crate::fs::buf::B_ERROR;
    use crate::fs::mount::{vfs_mount_alloc, vfs_mount_free, vfs_unbusy};
    use crate::fs::testfs::{self, Kind, TestGlue, UBUF, set_bdev_strategy, up};
    use crate::fs::ufs::ffs::testffs::{self, devvp, set_disk};
    use crate::fs::ufs::ufs::testufs::TEST_DEV;
    use crate::fs::vnode::{VDIR, VROOT};

    /// A FAT image with a file in the root directory and an empty subdirectory.
    fn sample(params: mkfat::Params) -> (Vec<u8>, u32) {
        let mut img = mkfat::Image::new(params);
        let root = img.root();
        img.add_file(root, None, b"README  TXT", &[b'r'; 3000]);
        img.mkdir(root, b"SUBDIR     ", 1);
        let used = img.nclusters;
        (img.finish(), used)
    }

    /// The `Nameidata` `mount(2)` hands `VFS_MOUNT` (`msdosfs_mount` looks the device up in a
    /// `Nameidata` of its own).
    fn nd(p: &Thread<TestGlue>) -> Nd<'_, TestGlue> {
        ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/msdos"), p)
    }

    /// `args` as the kernel copy `mount(2)` makes of the user's `struct msdosfs_args`.
    fn data_of(args: &MsdosfsArgs) -> Vec<u8> {
        let mut data = vec![0u8; MsdosfsArgs::SIZE];
        assert!(hal::user::pod_to_bytes(args, &mut data));
        data
    }

    /// The mount's structure.
    fn pmp_of(mp: &'static Mount<TestGlue>) -> MsdosfsmountRef<TestGlue> {
        vfstomsdosfs(mp)
    }

    #[test]
    fn fat12_mountfs_works_out_the_geometry() {
        let (disk, nclusters) = sample(mkfat::FAT12_1M);
        let (_g, p) = attach(disk);

        let mp = try_mount(p, 0).unwrap();
        let pmp = pmp_of(mp);
        assert!(fat12(&pmp));
        assert_eq!(pmp.pm_fatmask, FAT12_MASK);
        assert_eq!((pmp.pm_fatmult, pmp.pm_fatdiv), (3, 2));
        assert_eq!(pmp.pm_BytesPerSec(), 512);
        assert_eq!(pmp.pm_BlkPerSec, 1);
        assert_eq!(pmp.pm_bpcluster, 512);
        assert_eq!(pmp.pm_crbomask, 511);
        assert_eq!(pmp.pm_cnshift, 9);
        assert_eq!(pmp.pm_bnshift, 9);
        assert_eq!(pmp.pm_fatblk, 1);
        // 1 reserved sector and two FATs of 6 sectors; 64 entries are 4 sectors.
        assert_eq!(pmp.pm_FATsecs, 6);
        assert_eq!(pmp.pm_rootdirblk, 13);
        assert_eq!(pmp.pm_rootdirsize, 4);
        assert_eq!(pmp.pm_firstcluster, 17);
        assert_eq!(pmp.pm_nmbrofclusters, nclusters);
        assert_eq!(pmp.pm_nmbrofclusters, 2031);
        assert_eq!(pmp.pm_maxcluster, 2032);
        assert_eq!(pmp.pm_fatblocksize, 3 * 512);
        assert_eq!(pmp.pm_fatsize, 6 * 512);
        assert_eq!(pmp.pm_fsinfo.get(), 0);
        assert_ne!(pmp.pm_flags.get() & MSDOSFS_FATMIRROR, 0);
        assert_eq!(pmp.pm_flags.get() & MSDOSFSMNT_RONLY, 0);
        assert_eq!(pmp.pm_fmod.get(), 1);
        // `pm_bpb` keeps the BPB's members but the two the C keeps elsewhere.
        assert_eq!((pmp.pm_bpb.bpbSecPerClust, pmp.pm_bpb.bpbFATsecs), (0, 0));
        assert_eq!((pmp.pm_FATs(), pmp.pm_RootDirEnts()), (2, 64));
        assert_eq!((pmp.pm_Sectors(), pmp.pm_HugeSectors()), (2048, 2048));
        // README.TXT has 6 clusters, SUBDIR 1.
        assert_eq!(pmp.pm_freeclustercount.get(), 2031 - 7);
        assert_eq!(pmp.pm_inusemap.len(), 2033usize.div_ceil(32));
        assert!(ptr::eq(pmp.pm_mountp, mp));
        assert_eq!(pmp.pm_dev, TEST_DEV);
        let st = mp.stat();
        assert_eq!(st.f_fsid.val, [TEST_DEV, 4]);
        assert!(
            pmp.pm_devvp
                .specmountpoint()
                .is_some_and(|m| ptr::eq(m, mp))
        );

        let mut sb = mp.stat();
        msdosfs_statfs(mp, &mut sb, p).unwrap();
        assert_eq!(sb.f_bsize, 512);
        assert_eq!(sb.f_iosize, 512);
        assert_eq!(sb.f_blocks, 2031);
        assert_eq!(sb.f_bfree, 2024);
        assert_eq!(sb.f_bavail, 2024);
        assert_eq!(sb.f_files, 64);
        assert_eq!(sb.f_ffree, 0);
        assert_eq!(&sb.f_fstypename[..6], b"msdos\0");

        // The root directory: VFS_ROOT's denode is the manufactured one.
        let vp = root(mp);
        assert_ne!(vp.flag() & VROOT, 0);
        assert_eq!(vp.vtype(), VDIR);
        vput(vp);
        drop(pmp);
        unmount(p, mp);
        assert!(devvp().specmountpoint().is_none());
        assert!(!mp.has_node(), "the structure went with the unmount");
        assert_eq!(mp.flag() & MNT_LOCAL, 0);
    }

    #[test]
    fn fat16_mount_read_only() {
        let (disk, nclusters) = sample(mkfat::FAT16_4M);
        let (_g, p) = attach(disk);
        let mp = try_mount(p, MNT_RDONLY).unwrap();
        let pmp = pmp_of(mp);
        assert!(fat16(&pmp));
        assert_eq!((pmp.pm_fatmult, pmp.pm_fatdiv), (2, 1));
        assert_eq!(pmp.pm_fatblocksize, MAXBSIZE as u32);
        assert_eq!(pmp.pm_rootdirsize, 32);
        assert_eq!(pmp.pm_nmbrofclusters, nclusters);
        assert_ne!(pmp.pm_flags.get() & MSDOSFSMNT_RONLY, 0);
        assert_eq!(pmp.pm_fmod.get(), 0);
        drop(pmp);
        unmount(p, mp);
    }

    #[test]
    fn fat32_mount() {
        let (disk, nclusters) = sample(mkfat::FAT32_1M);
        let (_g, p) = attach(disk);
        let mp = try_mount(p, 0).unwrap();
        let pmp = pmp_of(mp);
        assert!(fat32(&pmp));
        assert_eq!(pmp.pm_fatmask, FAT32_MASK);
        assert_eq!(pmp.pm_rootdirblk, 2);
        assert_eq!(pmp.pm_rootdirsize, 0);
        assert_eq!(pmp.pm_firstcluster, pmp.pm_fatblk + 2 * pmp.pm_FATsecs);
        assert_eq!(pmp.pm_fatblk, 32);
        assert_eq!(pmp.pm_fsinfo.get(), 1, "a valid FSInfo block is kept");
        assert_eq!(pmp.pm_curfat, 0);
        assert_ne!(pmp.pm_flags.get() & MSDOSFS_FATMIRROR, 0);
        assert_eq!(pmp.pm_nmbrofclusters, nclusters);
        // The root directory, README.TXT and SUBDIR.
        assert_eq!(pmp.pm_freeclustercount.get(), nclusters - 8);
        // FAT32's root is a directory of its cluster.
        let vp = root(mp);
        assert_eq!(vtode(vp).de_StartCluster.get(), 2);
        vput(vp);
        drop(pmp);
        unmount(p, mp);
    }

    #[test]
    fn fat32_fsinfo_and_active_fat() {
        let (mut disk, _) = sample(mkfat::FAT32_1M);
        // A bad FSInfo signature is ignored, and FAT 1 is the only active one.
        disk[512 + 484] = b'X';
        disk[40..42].copy_from_slice(&0x0081u16.to_le_bytes());
        let (_g, p) = attach(disk);
        let mp = try_mount(p, 0).unwrap();
        let pmp = pmp_of(mp);
        assert_eq!(pmp.pm_fsinfo.get(), 0);
        assert_eq!(pmp.pm_curfat, 1);
        assert_eq!(pmp.pm_flags.get() & MSDOSFS_FATMIRROR, 0);
        drop(pmp);
        unmount(p, mp);
    }

    #[test]
    fn mountfs_rejects_bad_boot_sectors() {
        let (good, _) = sample(mkfat::FAT12_1M);
        let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
        let mut d = good.clone();
        d[11..13].copy_from_slice(&0u16.to_le_bytes());
        cases.push(("no bytes per sector", d));
        let mut d = good.clone();
        d[13] = 0;
        cases.push(("no sectors per cluster", d));
        let mut d = good.clone();
        d[13] = 3;
        cases.push(("sectors per cluster not a power of 2", d));
        let mut d = good.clone();
        d[11..13].copy_from_slice(&768u16.to_le_bytes());
        cases.push(("sector size not a power of 2", d));
        let mut d = good.clone();
        d[11..13].copy_from_slice(&256u16.to_le_bytes());
        cases.push(("sector smaller than DEV_BSIZE", d));
        let mut d = good.clone();
        d[22..24].copy_from_slice(&0u16.to_le_bytes());
        cases.push(("no FAT sectors", d));
        let mut d = good.clone();
        d[13] = 0x80;
        d[11..13].copy_from_slice(&1024u16.to_le_bytes());
        cases.push(("clusters larger than MAXBSIZE", d));
        let mut d = good.clone();
        d[17..19].copy_from_slice(&0u16.to_le_bytes());
        cases.push(("FAT32 with 16-bit sector counts", d));
        let mut d = good.clone();
        d[19..21].copy_from_slice(&0u16.to_le_bytes());
        d[32..36].copy_from_slice(&0u32.to_le_bytes());
        cases.push(("no sectors at all", d));

        let (_g, p) = attach(good.clone());
        for (what, disk) in cases {
            set_disk(disk);
            let mp = vfs_mount_alloc(None, &TEST_MSDOS_CONF);
            assert_eq!(
                msdosfs_mountfs(devvp(), mp, p, &noargs()),
                Err(Errno::EINVAL),
                "{what}"
            );
            assert!(!mp.has_node(), "{what}");
            assert!(devvp().specmountpoint().is_none());
            vfs_unbusy(mp);
            vfs_mount_free(mp);
        }

        // The device is still usable.
        set_disk(good);
        let mp = try_mount(p, 0).unwrap();
        assert_eq!(pmp_of(mp).pm_nmbrofclusters, 2031);
        unmount(p, mp);
    }

    #[test]
    fn mkfat_long_names_match_the_kernel() {
        // The builder's long name entries are what unix2winfn writes.
        let name = b"m10c-fat.txt";
        let short = *b"M10C-FATTXT";
        let ents = mkfat::lfn_entries(name, &short);
        assert_eq!(ents.len(), 1);
        assert_eq!(
            mkfat::chksum(&short),
            crate::fs::msdosfs::msdosfs_conv::winChksum(&short)
        );
        let mut we = crate::fs::msdosfs::direntry::Winentry::default();
        let more =
            crate::fs::msdosfs::msdosfs_conv::unix2winfn(name, &mut we, 1, mkfat::chksum(&short));
        assert_eq!(more, 0);
        let mut bytes = [0u8; 32];
        assert!(crate::fs::msdosfs::direntry::Winentry::put(
            &mut bytes, 0, &we
        ));
        assert_eq!(bytes, ents[0]);
    }

    /// A file system that claims more clusters than its FAT holds is clamped to the FAT, with
    /// the C's `printf`; a FAT whose size wraps to 0 prints the C's `%d` of the wrapped values
    /// and is refused by the bitmap's size check (`EINVAL`), in the C's widths, no overflow.
    #[test]
    fn the_cluster_count_is_clamped_to_the_fat() {
        let (mut disk, _) = sample(mkfat::FAT12_1M);
        // One FAT sector: 512 bytes hold 340 FAT12 entries.
        disk[22..24].copy_from_slice(&1u16.to_le_bytes());
        let (_g, p) = attach(disk);
        let line = "msdosfs: reducing max cluster to 339 from 2042 due to FAT size\n";
        let before = crate::testutil::printf_count(line);
        let mp = try_mount(p, MNT_RDONLY).unwrap();
        assert_eq!(crate::testutil::printf_count(line), before + 1);
        let pmp = pmp_of(mp);
        assert_eq!((pmp.pm_firstcluster, pmp.pm_maxcluster), (7, 339));
        assert_eq!(pmp.pm_inusemap.len(), 340usize.div_ceil(32));
        drop(pmp);
        unmount(p, mp);

        // FAT32 with 2^23 sectors per FAT: 2^32 bytes, a `pm_fatsize` of 0.
        let (mut disk, _) = sample(mkfat::FAT32_1M);
        disk[36..40].copy_from_slice(&0x0080_0000u32.to_le_bytes());
        set_disk(disk);
        let line = "msdosfs: reducing max cluster to -1 from -16775199 due to FAT size\n";
        let before = crate::testutil::printf_count(line);
        assert_eq!(try_mount(p, MNT_RDONLY).err(), Some(Errno::EINVAL));
        assert_eq!(crate::testutil::printf_count(line), before + 1);
    }

    std::thread_local! {
        /// The disk block (`b_blkno`) [`failing_strategy`] fails with `EIO`.
        static FAIL_BLKNO: Cell<Daddr> = const { Cell::new(-1) };
    }

    /// testffs's strategy, except for [`FAIL_BLKNO`]'s block: `B_ERROR`, `EIO`, `biodone`.
    fn failing_strategy(t: BufRef) {
        if FAIL_BLKNO.with(Cell::get) == t.buf().blkno() {
            t.buf().set_error(Some(Errno::EIO));
            t.buf().set(B_ERROR);
            biodone(t);
            return;
        }
        testffs::disk_strategy(t);
    }

    /// The FSInfo block: a read that fails fails the mount after the boot sector's checks (the
    /// clamp's `printf` printed first), the device closed and nothing hung; a block larger than
    /// `MAXBSIZE` (sectors of 16 KiB and 32 KiB) is refused with `EINVAL` before any read
    /// (D-N5.7-U3); 4 KiB and 8 KiB sectors read theirs (4 KiB and 16 KiB) and mount.
    #[test]
    fn the_fsinfo_block_is_read_within_maxbsize() {
        let (_g, p) = attach(vec![0u8; 4096]);
        for (bps, want) in [
            (4096u16, Ok(4096)),
            (8192, Ok(16384)),
            (16384, Err(Errno::EINVAL)),
            (32768, Err(Errno::EINVAL)),
        ] {
            let params = mkfat::Params {
                fat: 32,
                size: 4 << 20,
                bps,
                spc: 1,
                rde: 0,
            };
            let mut img = mkfat::Image::new(params);
            let root = img.root();
            img.add_file(root, None, b"README  TXT", b"fat32\n");
            set_disk(img.finish());
            // A failing read of the FSInfo block (sector 1) would answer EIO: none is made
            // when the size is refused.
            FAIL_BLKNO.with(|c| c.set(Daddr::from(bps / 512)));
            set_bdev_strategy(Some(failing_strategy));
            let r = try_mount(p, MNT_RDONLY).map(|_| ());
            match want {
                Ok(_) => assert_eq!(r, Err(Errno::EIO), "{bps}: the FSInfo read"),
                Err(e) => assert_eq!(r, Err(e), "{bps}"),
            }
            assert!(devvp().specmountpoint().is_none());
            set_bdev_strategy(Some(testffs::disk_strategy));
            match want {
                Ok(size) => {
                    assert_eq!(fsi_size_checked(u32::from(bps) / 512), Ok(size));
                    let mp = try_mount(p, MNT_RDONLY).unwrap();
                    let pmp = pmp_of(mp);
                    assert_eq!(pmp.pm_fsinfo.get(), u32::from(bps) / 512, "{bps}");
                    assert_eq!(pmp.pm_BytesPerSec(), bps);
                    drop(pmp);
                    unmount(p, mp);
                }
                Err(e) => assert_eq!(try_mount(p, MNT_RDONLY).err(), Some(e), "{bps}"),
            }
        }
    }

    /// `mount(2)`'s path, `msdosfs_mount`: the arguments the C needs, a block device, the
    /// arguments applied after `msdosfs_mountfs` (the owner, the group, the mask, the name
    /// flags with `f_namemax`, the names and `mount_info`), an update by the same device,
    /// another device refused.
    #[test]
    fn mount_needs_a_block_device_and_applies_the_arguments() {
        let (disk, _) = sample(mkfat::FAT12_1M);
        let (_g, p) = attach(disk);
        let mp = vfs_mount_alloc(None, &TEST_MSDOS_CONF);
        let v = &MsdosfsVfs::<TestGlue>::VFS;
        assert_eq!(
            v.mount(mp, b"/msdos", &mut [], &mut nd(p), p),
            Err(Errno::EINVAL),
            "no msdosfs_args"
        );
        let mut args = noargs();
        // Block devices of testfs's disk major (2) whose strategy is testffs's disk, minors
        // of their own (a vnode of the mounted device's number would be its alias, which
        // `vcount` counts), and one of a major past `nblkdev` (8).
        testfs::tree_add(b"fd0", testfs::DEV, Kind::Blk, Some(makedev(2, 20)));
        testfs::tree_add(b"fd1", testfs::DEV, Kind::Blk, Some(makedev(2, 21)));
        testfs::tree_add(b"wd9", testfs::DEV, Kind::Blk, Some(makedev(9, 0)));
        for (fspec, want) in [
            (&b"/f"[..], Errno::ENOTBLK),
            (b"/dev/sd0c", Errno::ENOTBLK),
            (b"/nothere", Errno::ENOENT),
            (b"/dev/wd9", Errno::ENXIO),
        ] {
            args.fspec = up(0, fspec).addr();
            assert_eq!(
                v.mount(mp, b"/msdos", &mut data_of(&args), &mut nd(p), p),
                Err(want)
            );
        }
        // A name without its NUL within MNAMELEN.
        UserPtr::<u8>::new(UBUF)
            .copyout_slice(&[b'x'; MNAMELEN + 1])
            .unwrap();
        args.fspec = UBUF;
        assert_eq!(
            v.mount(mp, b"/msdos", &mut data_of(&args), &mut nd(p), p),
            Err(Errno::ENAMETOOLONG)
        );
        assert!(!mp.has_node());

        // mount -t msdos -s -u 5 -g 6 -m 750 /dev/fd0 /msdos
        args.fspec = up(0, b"/dev/fd0").addr();
        args.uid = 5;
        args.gid = 6;
        args.mask = 0o750;
        args.flags = MSDOSFSMNT_NOWIN95;
        let mut data = data_of(&args);
        v.mount(mp, b"/msdos", &mut data, &mut nd(p), p).unwrap();
        let pmp = pmp_of(mp);
        assert_eq!(
            (pmp.pm_uid.get(), pmp.pm_gid.get(), pmp.pm_mask.get()),
            (5, 6, 0o750)
        );
        // NOWIN95 implies short names.
        let names = (MSDOSFSMNT_NOWIN95 | hal::sys::mount::MSDOSFSMNT_SHORTNAME) as u32;
        assert_eq!(pmp.pm_flags.get() & MSDOSFSMNT_MNTOPT, names);
        assert_eq!(pmp.pm_dev, makedev(2, 20));
        let st = mp.stat();
        assert_eq!(st.f_namemax, 12);
        assert_eq!(&st.f_mntonname[..7], b"/msdos\0");
        assert_eq!(&st.f_mntfromname[..9], b"/dev/fd0\0");
        assert_eq!(&st.f_mntfromspec[..9], b"/dev/fd0\0");
        assert_eq!(&st.mount_info.__align[..MsdosfsArgs::SIZE], &data[..]);
        assert_eq!(st.f_fsid.val, [makedev(2, 20), 4]);

        // An update naming the mounted device applies the arguments again.
        mp.set_flag(mp.flag() | MNT_UPDATE);
        args.uid = 7;
        args.flags = 0;
        v.mount(mp, b"/msdos", &mut data_of(&args), &mut nd(p), p)
            .unwrap();
        assert_eq!(pmp.pm_uid.get(), 7);
        assert_eq!(mp.stat().f_namemax, 12, "the flags only accumulate");
        // Another device is refused.
        args.fspec = up(0, b"/dev/fd1").addr();
        assert_eq!(
            v.mount(mp, b"/msdos", &mut data_of(&args), &mut nd(p), p),
            Err(Errno::EINVAL)
        );
        mp.set_flag(mp.flag() & !MNT_UPDATE);
        drop(pmp);
        v.unmount(mp, 0, p).unwrap();
        vfs_unbusy(mp);
        vfs_mount_free(mp);

        // Without a name flag the mount takes long names.
        let mp = vfs_mount_alloc(None, &TEST_MSDOS_CONF);
        args.fspec = up(0, b"/dev/fd0").addr();
        v.mount(mp, b"/msdos", &mut data_of(&args), &mut nd(p), p)
            .unwrap();
        assert_eq!(
            pmp_of(mp).pm_flags.get() & MSDOSFSMNT_MNTOPT,
            MSDOSFSMNT_LONGNAME as u32
        );
        assert_eq!(mp.stat().f_namemax, 255);
        v.unmount(mp, 0, p).unwrap();
        vfs_unbusy(mp);
        vfs_mount_free(mp);
    }

    /// `mount -u`: to read-only (the files flushed, `MSDOSFSMNT_RONLY`, the device asked to
    /// sync its cache), back to read-write (`MNT_WANTRDWR`), a reload (`EOPNOTSUPP`), an
    /// export update without a device name (the glue's `vfs_export`: the test glue has no
    /// export list and answers `EOPNOTSUPP`, which `msdosfs_mount` returns), and an update
    /// without arguments.
    #[test]
    fn updates_switch_read_only_and_back_and_pass_exports_to_the_glue() {
        let (disk, _) = sample(mkfat::FAT12_1M);
        let (_g, p) = attach(disk);
        let mp = try_mount(p, 0).unwrap();
        let pmp = pmp_of(mp);
        let v = &MsdosfsVfs::<TestGlue>::VFS;

        mp.set_flag(mp.flag() | MNT_UPDATE | MNT_RDONLY);
        v.mount(mp, b"/msdos", &mut [], &mut nd(p), p).unwrap();
        assert_ne!(pmp.pm_flags.get() & MSDOSFSMNT_RONLY, 0);
        assert_ne!(mp.flag() & MNT_RDONLY, 0);

        mp.set_flag(mp.flag() | MNT_WANTRDWR);
        v.mount(mp, b"/msdos", &mut [], &mut nd(p), p).unwrap();
        assert_eq!(pmp.pm_flags.get() & MSDOSFSMNT_RONLY, 0);
        mp.set_flag(mp.flag() & !(MNT_WANTRDWR | MNT_RDONLY));

        mp.set_flag(mp.flag() | MNT_RELOAD);
        assert_eq!(
            v.mount(mp, b"/msdos", &mut [], &mut nd(p), p),
            Err(Errno::EOPNOTSUPP)
        );
        mp.set_flag(mp.flag() & !MNT_RELOAD);

        let mut args = noargs();
        args.export_info.ex_flags = MNT_EXPORTED;
        assert_eq!(
            v.mount(mp, b"/msdos", &mut data_of(&args), &mut nd(p), p),
            Err(Errno::EOPNOTSUPP),
            "the test glue's vfs_export"
        );
        assert_eq!(mp.flag() & MNT_EXPORTED, 0);
        mp.set_flag(mp.flag() & !MNT_UPDATE);
        drop(pmp);
        unmount(p, mp);
    }

    /// `msdosfs_vptofh` names a file by its entry's position, which `msdosfs_fhtovp` reads
    /// back; a handle whose entry does not lie inside the block `deget` would read is
    /// `ESTALE` before any read (D-N5.7-U1): the last 31 bytes of a cluster, past the FAT12
    /// root's shortened last block, and the root's own offset on another cluster; an offset
    /// inside the block that is not on an entry gets the C's answer (the bytes there).
    #[test]
    fn file_handles_name_the_entry_and_stale_ones_are_refused() {
        // 2 KB clusters and a root directory of one 512-byte block.
        let params = mkfat::Params {
            fat: 12,
            size: 1 << 20,
            bps: 512,
            spc: 4,
            rde: 16,
        };
        let mut img = mkfat::Image::new(params);
        img.add_file(mkfat::Dir::Root, None, b"ONLY    TXT", b"x");
        let sub = img.mkdir(mkfat::Dir::Root, b"SUBDIR     ", 1);
        let (_g, p) = attach(img.finish());
        let mp = try_mount(p, 0).unwrap();
        let v = &MsdosfsVfs::<TestGlue>::VFS;
        let vp = {
            let pmp = pmp_of(mp);
            let dep = deget(&pmp, MSDOSFSROOT, 0).unwrap();
            dep.detov()
        };
        let mut fid = Fid::default();
        v.vptofh(vp, &mut fid).unwrap();
        let fh = Defid::from_fid(&fid);
        assert_eq!(
            (fh.defid_len, fh.defid_dirclust, fh.defid_dirofs),
            (12, MSDOSFSROOT, 0)
        );
        vput(vp);
        let again = v.fhtovp(mp, &fid).unwrap();
        assert!(ptr::eq(again, vp));
        vput(again);
        // The root's own handle.
        let rvp = root(mp);
        v.vptofh(rvp, &mut fid).unwrap();
        assert_eq!(
            Defid::from_fid(&fid).defid_dirofs,
            MSDOSFSROOT_OFS,
            "the manufactured root"
        );
        vput(rvp);
        let rvp = v.fhtovp(mp, &fid).unwrap();
        assert_ne!(rvp.flag() & VROOT, 0);
        vput(rvp);

        let handle = |dirclust: u32, dirofs: u32| {
            let mut fid = Fid::default();
            Defid {
                defid_len: 12,
                defid_pad: 0,
                defid_dirclust: dirclust,
                defid_dirofs: dirofs,
            }
            .to_fid(&mut fid);
            fid
        };
        for (what, dirclust, dirofs) in [
            ("the last 31 bytes of a cluster", sub, 2048 - 16),
            ("past the root's 512-byte block", MSDOSFSROOT, 512),
            ("the root's offset on another cluster", sub, MSDOSFSROOT_OFS),
        ] {
            assert_eq!(
                v.fhtovp(mp, &handle(dirclust, dirofs)).err(),
                Some(Errno::ESTALE),
                "{what}"
            );
        }
        // The last whole entry of the subdirectory's cluster is read (an empty slot).
        let last = v.fhtovp(mp, &handle(sub, 2048 - 32)).unwrap();
        assert_eq!(vtode(last).de_Name[0].get(), 0);
        vput(last);
        unmount(p, mp);
    }

    /// The methods no other test calls: `vfs_start`, `vfs_quotactl`, `vfs_vget` and
    /// `vfs_sysctl` (the C's `eopnotsupp`, the last the trait's default), and the structure's
    /// type: the mount's slot answers only `Msdosfsmount`.
    #[test]
    fn the_other_vfs_methods_answer_as_the_c() {
        let (_g, p, pmp) = full_root(1024, 2, 208);
        let mp = pmp.pm_mountp;
        let v = &MsdosfsVfs::<TestGlue>::VFS;
        assert_eq!(v.start(mp, 0, p), Ok(()));
        assert_eq!(v.quotactl(mp, 0, 0, 0, p), Err(Errno::EOPNOTSUPP));
        assert_eq!(v.vget(mp, 2).err(), Some(Errno::EOPNOTSUPP));
        let mut len = 0;
        assert_eq!(v.sysctl(&[1], 0, &mut len, 0, 0, p), Err(Errno::EOPNOTSUPP));
        assert!(v.as_any().is_none());
        assert!(mp.node_arc::<u32>().is_none());
        // DIOCCACHESYNC is `_IOW('d', 120, int)`.
        assert_eq!(DIOCCACHESYNC, 0x8004_6478);
        drop(pmp);
        unmount(p, mp);
    }
}
/* </TESTS> */
