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
 * Copyright (c) 2001, 2002 Scott Long <scottl@freebsd.org>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: src/sys/fs/udf/udf_vfsops.c,v 1.25 2005/01/25 15:52:03 phk Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! UDF file-system-type operations (`udf_vfsops`): the pools (`udf_init`), the mount (the
//! anchor, the volume descriptor sequence, the partition maps, the file set descriptor and the
//! root's file entry), unmount, root, `statfs`, `udf_vget` (a file entry into a vnode), file
//! handles, `udf_checktag`, and the partition maps' handlers.
//!
//! Upstream: sys/isofs/udf/udf_vfsops.c @ 3ce1f3f79392
//! LZ: sys/isofs/udf/udf_vfsops.rs@3c62ede68802
//!
//! Ported to OpenBSD by Pedro Martelletto in February 2005.
//!
//! Ok, here's how it goes. The UDF specs are pretty clear on how each data structure is
//! made up, but not very clear on how they relate to each other. Here is the skinny... This
//! demonstrates a filesystem with one file in the root directory. Subdirectories are treated
//! just as normal files, but they have File Id Descriptors of their children as their file
//! data. As for the Anchor Volume Descriptor Pointer, it can exist in two of the following
//! three places: sector 256, sector n (the max sector of the disk), or sector n - 256. It's
//! a pretty good bet that one will exist at sector 256 though. One caveat is unclosed CD
//! media. For that, sector 256 cannot be written, so the Anchor Volume Descriptor Pointer
//! can exist at sector 512 until the media is closed.
//!
//! ## Deviations
//! - `udf_init` takes no `struct vfsconf *` (the C's ignores it); a second call (a host
//!   test's, a second `vfsinit`) keeps the pools made first: their `init` answers `EBUSY`.
//! - `udf_mount` reads its `struct udf_args` out of the kernel copy of the mount arguments
//!   (`UdfArgs::from_bytes`), `EINVAL` when they are short; it looks the device up with a
//!   `Nameidata` of its own (`ndinit`), as `ffs_mount` does, instead of reinitialising the
//!   caller's. The `UDF_DEBUG` message (no such option here) is left out (LZ's).
//! - `vfsconflist[]` gives UDF `UdfArgs::SIZE` as `vfc_datasize` (the C writes
//!   `sizeof(struct iso_args)`, larger than the `struct udf_args` that `mount_udf(8)`
//!   passes, so `sys_mount` would copy in bytes past it) (LZ's).
//! - `udf_root`, `udf_vget` and `udf_fhtovp` return the vnode (`Vfs`'s shape).
//! - A descriptor member that does not fit in what was read (a partition map past the
//!   logical volume descriptor's 2,048-byte buffer, the Metadata File's allocation
//!   descriptor, a sparing table shorter than its header, a sparing table entry) is `EINVAL`,
//!   or ends the count of sparing table entries, where the C reads past the buffer (LZ's).
//! - `udf_vget` copies at most what the buffer holds of the file entry (the C copies
//!   `l_ea + l_ad` and the header's size from a one-block buffer); the rest of the copy stays
//!   zero (`M_ZERO`) (LZ's).
//! - [`udf_mountfs`] refuses with `EINVAL` a logical volume descriptor whose `lb_size` is
//!   outside `DEV_BSIZE..=MAXBSIZE` or not a power of two (`udf_lb_size`), where the C
//!   computes `um_bshift = ffs(um_bsize) - 1` of any size (a shift by -1 for 0, a division by
//!   zero in `udf_get_spartmap`, the wrong blocks for a size not a power of two: the user's
//!   D-N5.8-U3); every real medium is 512, 2,048 or 4,096. A root ICB or a sparing table
//!   longer than `MAXBSIZE` is `EINVAL` before its read (`udf_readlblks`, `udf.rs`).
//! - The error path of `udf_mountfs` frees the sparing table and the VAT block, and
//!   `udf_get_spartmap` frees the table a later failure orphans, where the C leaks them
//!   (udf_vfsops.c:430-433 free only the hash and the structure; :762 overwrites `um_stbl`
//!   before the read, so a failing map frees the new table and leaks the old): the user's U4
//!   of 2026-11-06 ("free them"). `vmstat -m`'s `UDF mount` requests are the C's; its `InUse`
//!   is lower than OpenBSD's after a failed mount or a mount with more than one sparing table.
//! - On that error path, once the VAT was read, the mount's vnodes are flushed (`vflush`,
//!   not forced: the VAT's) before the mount structure goes (D-u2-1; the user's U6 of
//!   2026-11-06, in U4's class): the C's `udf_vat_get` `vput`s the VAT's vnode, which stays
//!   cached on the freed mount, its unode naming the freed structure and holding the
//!   device's `vref` (udf_subr.c:196-209, udf_vfsops.c:637; the bail at :430 frees the
//!   structure under it). So after a failed mount of a VAT medium OpenBSD keeps one more
//!   `udfndpl` item and one more `UDF file entry` block in use (`vmstat -m`'s `InUse`), and a
//!   retry of the mount answers `EBUSY` (`vcount(devvp) > 1`, :237) until the vnode is
//!   recycled, which then reads freed memory; here both come back and a retry mounts (the
//!   same `EINVAL` for the same medium). No choice is the C's: without the flush the unode
//!   would keep the mount structure and its hash alive instead (`UDF mount`'s `InUse` higher
//!   than OpenBSD's).
//! - [`udf_fhtovp`] reads `ifid_ino` as the `udfino_t` [`udf_vptofh`] stored (zero-extended),
//!   as cd9660's (the user's D-N5.8-U2): the C's `int` is sign-extended into `VFS_VGET`'s
//!   `ino_t` and `udf_vget` panics ("alien ino_t") for a handle whose top bit is set.
//!   Unreachable: `udf_checkexp` refuses every export (`EACCES`), so no NFS client holds a
//!   UDF handle.
//! - `udf_checkexp` is `VFS_CHECKEXP`'s `MOUNT_UDF` arm (`bsd`'s `sys/sys/mount.rs`), the
//!   C's `EACCES` ("for the time being").
//! - `udf_vget` makes the unode after it read the file entry (the C's `pool_get` comes first,
//!   udf_vfsops.c:576): a `vget` refused before it (a read error, no file entry, no memory
//!   for its copy) makes and frees one item of `unode_pool` (D-N5.7-U5's shim), so
//!   `vmstat -m`'s `udfndpl` requests are the C's.
//! - `udf_disklabelspoof` is `bsd`'s (`sys/kern/subr_disk.rs`) over `udf_volume_id`
//!   (`udf_subr.rs`): the disk label and `dkcksum` are `bsd`'s until N7.
//!
//! ## Redesign
//! - LZ's `UDF_VFSOPS` (a `bsd` `Vfsops` of function pointers over `bsd`'s mount) ->
//!   `impl Vfs<G> for UdfVfs<G>` and its constant [`UdfVfs::VFS`], which `bsd`'s
//!   `vfsconflist[]` names; the functions are the C's, generic over the glue, in its order
//!   (cd9660's `Cd9660Vfs` shape). LZ's three `Pool` statics and `pool_init`s -> the glue's
//!   `UdfTables` (`udf_extern.rs`), made by [`udf_init`] with the C's names (`unode_pool`
//!   recorded, so `VTOU` counts a handle back from the unode the vnode lends).
//! - [`udf_mountfs`] publishes the mount structure where the C `malloc`s it (`Umount::alloc`,
//!   hung from the mount's typed slot at the C's `mp->mnt_data = ump`: `udf_vat_get` reaches
//!   it through the mount during the mount, D-N5.8-6), reads each descriptor's members at
//!   their offsets in the buffer's token (no copy of the 440-byte logical volume descriptor
//!   or the 512-byte file set descriptor), and takes every check in the C's order with the
//!   C's errnos and messages; its error path takes back what the mount made (the VAT block,
//!   the sparing table, the structure from the slot: the cycle unode -> mount -> VAT block ->
//!   unode is broken there and at the unmount). LZ wrote the structure into a raw `malloc`
//!   block and freed it by hand, and viewed the buffers in place (16 `unsafe`).
//! - [`udf_vget`] reads the file entry, copies it into its `M_UDFFENTRY` `MArray` (`M_NOWAIT`:
//!   `None` is `ENOMEM`), makes the unode with it (`Unode::new`) and publishes it in the
//!   glue's recorded `unode_pool`, gets the vnode and sets the unode's `u_vnode` (a `Link`
//!   set once, D-u2-2), hangs the unode from the vnode's typed slot where the C sets
//!   `v_data`, hashes it (which locks the vnode), and on a device alias moves the slot to the
//!   alias as cd9660's `cd9660_vget_internal` does. LZ wrote the unode into a raw
//!   `unode_pool` item.
//! - `struct ifid` is [`Ifid`], read from the `struct fid` it overlays field by field.
//!   `VFSTOUDFFS` is a counted handle per call (one count pair per `Vfs` operation reached
//!   from the mount, ROADMAP N12's row). 0 `unsafe` (LZ: 16).

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr;

use hal::intr::IPL_NONE;
use hal::malloc::{M_NOWAIT, M_WAITOK, MArc, MArray};
use hal::pool::{PR_WAITOK, PoolArc};
use hal::sleep::INFSLP;
use hal::sys::endian::{letoh16, letoh32};
use hal::sys::errno::Errno;
use hal::sys::fcntl::FREAD;
use hal::sys::mount::{
    Fid, MNAMELEN, MNT_FORCE, MNT_LOCAL, MNT_RDONLY, MNT_ROOTFS, MNT_UPDATE, Statfs, UdfArgs,
};
use hal::sys::syslimits::NAME_MAX;
use hal::sys::types::{Daddr, Ino, Uid, major};
use hal::user::{UserPtr, zeroed};

use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::fs::bio::{DEV_BSHIFT, bread, brelse};
use crate::fs::buf::BufRef;
use crate::fs::isofs::udf::ecma167_udf::{
    AnchorVdp, DescTag, ExtentAd, ExtfileEntry, FileEntry, FilesetDesc, LogvolDesc, LongAd,
    PartDesc, PartMap1, PartMap2, PartMapMeta, PartMapSpare, SpareMapEntry, TAGID_ANCHOR,
    TAGID_EXTFENTRY, TAGID_FENTRY, TAGID_FSD, TAGID_LOGVOL, TAGID_PARTITION, UDF_EXTFENTRY_SIZE,
    UDF_FENTRY_SIZE, UDF_ICB_FILETYPE_BLOCKDEVICE, UDF_ICB_FILETYPE_CHARDEVICE,
    UDF_ICB_FILETYPE_DIRECTORY, UDF_ICB_FILETYPE_FIFO, UDF_ICB_FILETYPE_RANDOMACCESS,
    UDF_ICB_FILETYPE_REALTIME, UDF_ICB_FILETYPE_SOCKET, UDF_ICB_FILETYPE_SYMLINK,
    UDF_ICB_FILETYPE_UNKNOWN, UdfSparingTable,
};
use crate::fs::isofs::udf::udf::{
    M_UDFFENTRY, M_UDFMOUNT, UDF_HASHTBLSIZE, UDF_MNT_FIND_VAT, UDF_MNT_USES_META, UdfHash, Udfino,
    Umount, Unode, VFSTOUDFFS, VTOU, lblk_bytes, rdsector, udf_getid, udf_readlblks,
};
use crate::fs::isofs::udf::udf_subr::{udf_lb_size, udf_vat_get, udf_vat_map};
use crate::fs::isofs::udf::udf_vnops::{udf_allocv, udf_hashins, udf_hashlookup};
use crate::fs::lookup::{namei, ndinit};
use crate::fs::mount::{Mount, Vfsconf};
use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
use crate::fs::pod::pod_at;
use crate::fs::spec::devsw;
use crate::fs::subr::{
    checkalias, copy_statfs_info, hashinit, siphash_key, vcount, vflush, vfs, vfs_mountedon, vgone,
    vinvalbuf, vref, vrele,
};
use crate::fs::ufs::ffs::ffs_vfsops::mname_copy;
use crate::fs::vnode::{
    FORCECLOSE, LK_EXCLUSIVE, LK_RETRY, V_SAVE, VBLK, VCHR, VDIR, VFIFO, VLNK, VREG, VROOT, VSOCK,
    Vnode, VopsId, vn_lock,
};
use crate::fs::vops::{Cred, Nd, VFS_VGET, Vfs, vop_close, vop_open, vop_unlock};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};

/// The block size the mount reads the anchor and the volume descriptor sequence in
/// (`bsize = 2048`: "Should probe the media for its size.").
const VDS_BSIZE: i32 = 2048;

/// `btodb(bsize)`: the disk blocks of one.
const VDS_DBLOCKS: u32 = (VDS_BSIZE >> DEV_BSHIFT) as u32;

/// `sizeof(struct ifid)`.
pub const IFID_SIZE: u16 = 16;

/// `struct ifid`: a UDF file identifier, overlaid on `struct fid`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifid {
    /// `ifid_len`.
    pub ifid_len: u16,
    /// `ifid_pad`.
    pub ifid_pad: u16,
    /// `ifid_ino`: the file's `udfino_t` in the C's `int` (the module's deviations).
    pub ifid_ino: i32,
    /// `ifid_start`.
    pub ifid_start: i64,
}

impl Ifid {
    /// `(struct ifid *)fhp`: the file identifier read as a UDF one (`ifid_ino` at offset 4,
    /// `ifid_start` aligned at offset 8).
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
}

/// The UDF file system of the glue `G`: `udf_vfsops` ([`UdfVfs::VFS`]). Its vnode table is
/// the glue's `UdfTables::vops` (`udf_vnops.rs`).
pub struct UdfVfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> UdfVfs<G> {
    /// `udf_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"udf"`).
    pub const VFS: UdfVfs<G> = UdfVfs(PhantomData);
}

/// `udf_init(vfsp)` (`vfs_init`): the pools of the name buffers, the unodes and the
/// directory streams.
pub fn udf_init<G: ProcGlue>() -> Result<(), Errno> {
    let t = &vfs::<G>().udf;
    for r in [
        t.trans_pool.init(IPL_NONE, PR_WAITOK, "udftrpl"),
        t.unode_pool.init_recorded(IPL_NONE, PR_WAITOK, "udfndpl"),
        t.ds_pool.init(IPL_NONE, PR_WAITOK, "udfdspl"),
    ] {
        match r {
            Ok(()) | Err(Errno::EBUSY) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

/// `udf_start` (`vfs_start`).
pub fn udf_start<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _flags: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    Ok(())
}

/// `udf_mount` (`vfs_mount`): mount system call. `data` is the kernel copy of the user's
/// `struct udf_args`.
pub fn udf_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    _ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if mp.flag() & MNT_RDONLY == 0 {
        mp.set_flag(mp.flag() | MNT_RDONLY);
    }

    // No root filesystem support. Probably not a big deal, since the bootloader doesn't
    // understand UDF.
    if mp.flag() & MNT_ROOTFS != 0 {
        return Err(Errno::EOPNOTSUPP);
    }

    // If updating, check whether changing from read-only to read/write; if there is no
    // device name, that's all we do.
    if mp.flag() & MNT_UPDATE != 0 {
        return Ok(());
    }

    let Some(args) = UdfArgs::from_bytes(data) else {
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

    if let Err(e) = udf_mountfs(devvp, mp, args.lastblock, p) {
        vrele(devvp);
        return Err(e);
    }

    // Keep a copy of the mount information.
    mp.with_stat(|sp| {
        mname_copy(&mut sp.f_mntonname, path);
        mname_copy(&mut sp.f_mntfromname, &fspec);
        mname_copy(&mut sp.f_mntfromspec, &fspec);
    });

    Ok(())
}

/// Check the descriptor tag for both the correct id and correct checksum. Return zero if
/// all is good, `EINVAL` if not.
pub fn udf_checktag(tag: &DescTag, id: u16) -> Result<(), Errno> {
    if letoh16(tag.id) != id {
        return Err(Errno::EINVAL);
    }

    // The sum of the tag's first 15 bytes but the checksum's own (byte 4), as a `uint8_t`.
    let itag = tag.as_bytes();
    let mut cksum: u8 = 0;
    for &b in &itag[..15] {
        cksum = cksum.wrapping_add(b);
    }
    cksum = cksum.wrapping_sub(itag[4]);

    if cksum == tag.cksum {
        return Ok(());
    }

    Err(Errno::EINVAL)
}

/// `udf_checktag` of the descriptor at the start of `b` (`EINVAL` when `b` is too short for
/// its tag).
fn checktag_at(b: &[u8], id: u16) -> Result<(), Errno> {
    match DescTag::at(b, 0) {
        Some(tag) => udf_checktag(&tag, id),
        None => Err(Errno::EINVAL),
    }
}

/// The `T` member at `off` of the descriptor `b` (zero when `b` ends first, which a block of
/// `um_bsize` bytes holding one of these descriptors never does).
fn member<T: hal::user::AbiPod>(b: &[u8], off: usize) -> T {
    pod_at(b, off).unwrap_or_else(zeroed)
}

/// `udf_mountfs`: read the volume's structures from `devvp` and set up its umount, hung from
/// the mount's typed slot (the C's `mp->mnt_data`); `devvp`'s reference is the mount's on
/// success.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its exits to bail kept in place"
)]
pub fn udf_mountfs<G: ProcGlue>(
    devvp: &'static Vnode<G>,
    mp: &'static Mount<G>,
    lb: u32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut fsd_offset: u32 = 0;
    let mut part_num: u16 = 0;
    let mut fsd_part: u16 = 0;
    let mut logvol_found = false;
    let mut part_found = false;
    let mut fsd_found = false;

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

    vop_open(devvp, FREAD, Cred::FSCRED, p)?;

    // ump = malloc(sizeof(*ump), M_UDFMOUNT, M_WAITOK | M_ZERO); mp->mnt_data = ump: the
    // structure published at the C's point, the slot's handle the mount's (D-N5.8-6).
    let ump = Umount::new(mp, devvp).alloc();
    mp.set_node(MArc::erase(ump.clone()));
    let typenum = mp.vfc().vfc_typenum;
    mp.with_stat(|sp| {
        sp.f_fsid.val[0] = devvp.rdev();
        sp.f_fsid.val[1] = typenum;
        sp.f_namemax = NAME_MAX as u32;
    });
    mp.set_flag(mp.flag() | MNT_LOCAL);

    // The C's `bp`: the buffer the mount holds, released on every path.
    let mut bp: Option<BufRef> = None;

    let error: Errno = 'bail: {
        // Get the Anchor Volume Descriptor Pointer from sector 256. Should also check sector
        // n - 256, n, and 512.
        let mut sector: u32 = 256;
        let (t, e) = bread(
            &devvp.bufs,
            Daddr::from(sector.wrapping_mul(VDS_DBLOCKS)),
            VDS_BSIZE,
        );
        let t = bp.insert(t);
        if let Err(e) = e {
            break 'bail e;
        }
        if let Err(e) = checktag_at(lblk_bytes(t), TAGID_ANCHOR) {
            break 'bail e;
        }
        let main_vds_ex: ExtentAd = member(lblk_bytes(t), offset_of!(AnchorVdp, main_vds_ex));
        if let Some(t) = bp.take() {
            brelse(t);
        }

        // Extract the Partition Descriptor and Logical Volume Descriptor from the Volume
        // Descriptor Sequence. Should we care about the partition type right now? What
        // about multiple partitions?
        let mvds_start = letoh32(main_vds_ex.loc);
        let mvds_end =
            mvds_start.wrapping_add(letoh32(main_vds_ex.len).wrapping_sub(1) / VDS_BSIZE as u32);
        sector = mvds_start;
        while sector < mvds_end {
            let (t, e) = bread(
                &devvp.bufs,
                Daddr::from(sector.wrapping_mul(VDS_DBLOCKS)),
                VDS_BSIZE,
            );
            let t = bp.insert(t);
            if let Err(e) = e {
                printf(format_args!("Can't read sector {} of VDS\n", sector as i32));
                break 'bail e;
            }
            let lvd = lblk_bytes(t);
            if checktag_at(lvd, TAGID_LOGVOL).is_ok() {
                let lb_size: u32 = member(lvd, offset_of!(LogvolDesc, lb_size));
                // The block size is checked where the C takes it (the module's deviations).
                let bs = match udf_lb_size(letoh32(lb_size)) {
                    Ok(bs) => bs,
                    Err(e) => break 'bail e,
                };
                ump.um_bsize.set(bs);
                ump.um_bmask.set(bs - 1);
                ump.um_bshift.set(bs.trailing_zeros() as i32);
                let fsd_loc: LongAd = member(lvd, offset_of!(LogvolDesc, _lvd_use));
                fsd_part = letoh16(fsd_loc.loc.part_num);
                fsd_offset = letoh32(fsd_loc.loc.lb_num);
                if udf_find_partmaps(&ump, lvd).is_err() {
                    break;
                }
                logvol_found = true;
            }
            let pd = lvd;
            if checktag_at(pd, TAGID_PARTITION).is_ok() {
                part_found = true;
                part_num = letoh16(member(pd, offset_of!(PartDesc, part_num)));
                let len = letoh32(member(pd, offset_of!(PartDesc, part_len)));
                ump.um_len.set(len);
                ump.um_reallen.set(len);
                let start = letoh32(member(pd, offset_of!(PartDesc, start_loc)));
                ump.um_start.set(start);
                ump.um_realstart.set(start);
            }

            if let Some(t) = bp.take() {
                brelse(t);
            }
            if part_found && logvol_found {
                break;
            }
            sector += 1;
        }

        if !part_found || !logvol_found {
            break 'bail Errno::EINVAL;
        }

        if ump.um_flags.get() & UDF_MNT_USES_META != 0 {
            // Read Metadata File 'File Entry' to find Metadata file.
            sector = ump.um_start.get().wrapping_add(ump.um_meta_start.get()); // Set in udf_get_mpartmap()
            let (t, e) = rdsector(&ump, devvp, Daddr::from(sector), ump.um_bsize.get());
            let t = bp.insert(t);
            if e.is_err() {
                printf(format_args!(
                    "Cannot read sector {} for Metadata File Entry\n",
                    sector as i32
                ));
                break 'bail Errno::EINVAL;
            }
            let fe = lblk_bytes(t);
            // The allocation descriptor at `l_ea` past the entry's header, when it fits.
            let la_at = |hdr: usize, l_ea_at: usize| {
                let l_ea: u32 = member(fe, l_ea_at);
                hdr.checked_add(letoh32(l_ea) as usize)
                    .and_then(|off| LongAd::at(fe, off))
            };
            let la = if checktag_at(fe, TAGID_EXTFENTRY).is_ok() {
                la_at(UDF_EXTFENTRY_SIZE, offset_of!(ExtfileEntry, l_ea))
            } else if checktag_at(fe, TAGID_FENTRY).is_ok() {
                la_at(UDF_FENTRY_SIZE, offset_of!(FileEntry, l_ea))
            } else {
                let tag: DescTag = member(fe, 0);
                printf(format_args!(
                    "Invalid Metadata File FE @ sector {}! (tag.id {})\n",
                    sector as i32,
                    { tag.id }
                ));
                break 'bail Errno::EINVAL;
            };
            let Some(la) = la else {
                break 'bail Errno::EINVAL;
            };
            ump.um_meta_start.set(letoh32(la.loc.lb_num));
            ump.um_meta_len.set(letoh32(la.len));
            if let Some(t) = bp.take() {
                brelse(t);
            }
        } else if fsd_part != part_num {
            printf(format_args!("FSD does not lie within the partition!\n"));
            break 'bail Errno::EINVAL;
        }

        // mtx_init(&ump->um_hashmtx, IPL_NONE): made with the structure.
        let Some((tbl, mask)) = hashinit(UDF_HASHTBLSIZE, M_UDFMOUNT, M_WAITOK) else {
            kpanic(format_args!("udf_mountfs: hashinit(M_WAITOK) failed"));
        };
        ump.set_hashtbl(UdfHash::new(tbl, mask, siphash_key()));

        // Get the VAT, if needed
        if ump.um_flags.get() & UDF_MNT_FIND_VAT != 0
            && let Err(e) = udf_vat_get(&ump, lb)
        {
            break 'bail e;
        }

        // Grab the Fileset Descriptor. Thanks to Chuck McCrobie <mccrobie@cablespeed.com>
        // for pointing me in the right direction here.
        sector = if ump.um_flags.get() & UDF_MNT_USES_META != 0 {
            ump.um_meta_start.get()
        } else {
            fsd_offset
        };
        let _ = udf_vat_map(&ump, &mut sector);
        let (t, e) = rdsector(&ump, devvp, Daddr::from(sector), ump.um_bsize.get());
        let t = bp.insert(t);
        if let Err(e) = e {
            printf(format_args!(
                "Cannot read sector {} of FSD\n",
                sector as i32
            ));
            break 'bail e;
        }
        let fsd = lblk_bytes(t);
        if checktag_at(fsd, TAGID_FSD).is_ok() {
            fsd_found = true;
            let mut icb: LongAd = member(fsd, offset_of!(FilesetDesc, rootdir_icb));
            if ump.um_flags.get() & UDF_MNT_USES_META != 0 {
                icb.loc.lb_num = { icb.loc.lb_num }.wrapping_add(ump.um_meta_start.get());
                icb.loc.part_num = part_num;
            }
            ump.set_root_icb(icb);
        }

        if let Some(t) = bp.take() {
            brelse(t);
        }

        if !fsd_found {
            printf(format_args!("Couldn't find the fsd\n"));
            break 'bail Errno::EINVAL;
        }

        // Find the file entry for the root directory.
        let root_icb = ump.root_icb();
        sector = letoh32(root_icb.loc.lb_num);
        let size = letoh32(root_icb.len);
        let _ = udf_vat_map(&ump, &mut sector);
        let t = match udf_readlblks(&ump, sector as i32, size as i32) {
            Ok(t) => bp.insert(t),
            Err(e) => {
                printf(format_args!("Cannot read sector {}\n", sector as i32));
                break 'bail e;
            }
        };

        let fe = lblk_bytes(t);
        if checktag_at(fe, TAGID_EXTFENTRY).is_err()
            && let Err(e) = checktag_at(fe, TAGID_FENTRY)
        {
            printf(format_args!("Invalid root file entry!\n"));
            break 'bail e;
        }

        if let Some(t) = bp.take() {
            brelse(t);
        }

        devvp.set_specmountpoint(Some(mp));

        return Ok(());
    };

    // bail: the VAT's vnode flushed (the module's deviations, D-u2-1), then the VAT block,
    // the sparing table and the structure freed (hashfree, free(ump)): the user's U4.
    if ump.um_vat.read().is_some() {
        let _ = vflush(mp, None, 0);
    }
    drop(ump.um_vat.replace(None));
    drop(ump.take_stbl());
    drop(mp.take_node());
    drop(ump);
    mp.set_flag(mp.flag() & !MNT_LOCAL);
    if devvp.has_specinfo() {
        devvp.set_specmountpoint(None);
    }
    if let Some(t) = bp {
        brelse(t);
    }

    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vop_close(devvp, FREAD, Cred::FSCRED, Some(p));
    let _ = vop_unlock(devvp);

    Err(error)
}

/// `udf_unmount` (`vfs_unmount`).
pub fn udf_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = VFSTOUDFFS(mp);
    let devvp = ump.devvp();
    let mut flags = 0;

    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }

    vflush(mp, None, flags)?;

    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vinvalbuf(devvp, V_SAVE, Cred::NOCRED, Some(p), 0, INFSLP);
    let _ = vop_close(devvp, FREAD, Cred::NOCRED, Some(p));
    let _ = vop_unlock(devvp);

    devvp.set_specmountpoint(None);
    vrele(devvp);

    // if (ump->um_flags & UDF_MNT_USES_VAT) free(ump->um_vat): the block is there only on
    // such a volume; taking it gives the VAT's unode back (the vnodes went with vflush).
    drop(ump.um_vat.replace(None));

    drop(ump.take_stbl());

    // hashfree(); free(ump, M_UDFMOUNT); mp->mnt_data = NULL: the mount's handle goes; the
    // unodes' went with vflush and the VAT block, so this call's, dropped here, is the last.
    drop(mp.take_node());
    drop(ump);

    mp.set_flag(mp.flag() & !MNT_LOCAL);

    Ok(())
}

/// `udf_root` (`vfs_root`): the root directory's vnode, locked.
pub fn udf_root<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    let ump = VFSTOUDFFS(mp);

    let id = udf_getid(&ump.root_icb());
    drop(ump);

    let vp = udf_vget(mp, Ino::from(id))?;

    vp.set_flag(vp.flag() | VROOT);

    Ok(vp)
}

/// `udf_quotactl` (`vfs_quotactl`).
pub fn udf_quotactl() -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `udf_statfs` (`vfs_statfs`).
pub fn udf_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = VFSTOUDFFS(mp);

    sbp.f_bsize = ump.um_bsize.get() as u32;
    sbp.f_iosize = ump.um_bsize.get() as u32;
    sbp.f_blocks = u64::from(ump.um_len.get());
    sbp.f_bfree = 0;
    sbp.f_bavail = 0;
    sbp.f_files = 0;
    sbp.f_ffree = 0;
    sbp.f_favail = 0;
    copy_statfs_info(sbp, mp);

    Ok(())
}

/// `udf_sync` (`vfs_sync`).
pub fn udf_sync() -> Result<(), Errno> {
    Ok(())
}

/// The `pool_get(&unode_pool)` of a `vget` refused before its unode was made, and its
/// `pool_put` (the module's deviations: D-N5.7-U5's shim): an item of no file entry and no
/// vnode, the C's zeroed item.
fn udf_vget_refused<G: ProcGlue>(ump: &crate::fs::isofs::udf::udf::UmountRef<G>, ino: Udfino) {
    drop(Unode::new(ump.clone(), ino, None).alloc());
}

/// `udf_vget` (`vfs_vget`): the vnode of the file entry at logical block `ino`, locked.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its error exits kept in place"
)]
pub fn udf_vget<G: ProcGlue>(mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
    if ino > Ino::from(Udfino::MAX) {
        kpanic(format_args!("udf_vget: alien ino_t {ino}"));
    }
    let ino = ino as Udfino;

    let ump = VFSTOUDFFS(mp);

    // See if we already have this in the cache
    if let Some(vp) = udf_hashlookup(&ump, ino, LK_EXCLUSIVE)? {
        return Ok(vp);
    }

    // Allocate memory and check the tag id's before grabbing a new vnode, since it's hard to
    // roll back if there is a problem. (The C's `pool_get` of the unode is here; the unode
    // is made below with the file entry's copy, and each exit before it makes and frees one
    // item: the module's deviations.)

    // Copy in the file entry. Per the spec, the size can only be 1 block.
    let mut sector = ino;
    let devvp = ump.devvp();
    let _ = udf_vat_map(&ump, &mut sector);
    let (bp, error) = rdsector(&ump, devvp, Daddr::from(sector), ump.um_bsize.get());
    if let Err(e) = error {
        printf(format_args!("Cannot read sector {}\n", sector as i32));
        udf_vget_refused(&ump, ino);
        brelse(bp);
        return Err(e);
    }

    let fe = lblk_bytes(&bp);
    // The C's `int size`: the `uint32_t` sum of `l_ea` and `l_ad`; the copy's length the
    // header's size past it.
    let (size, hdr) = if checktag_at(fe, TAGID_EXTFENTRY).is_ok() {
        let l_ea: u32 = member(fe, offset_of!(ExtfileEntry, l_ea));
        let l_ad: u32 = member(fe, offset_of!(ExtfileEntry, l_ad));
        (
            letoh32(l_ea).wrapping_add(letoh32(l_ad)) as i32,
            UDF_EXTFENTRY_SIZE,
        )
    } else if checktag_at(fe, TAGID_FENTRY).is_ok() {
        let l_ea: u32 = member(fe, offset_of!(FileEntry, l_ea));
        let l_ad: u32 = member(fe, offset_of!(FileEntry, l_ad));
        (
            letoh32(l_ea).wrapping_add(letoh32(l_ad)) as i32,
            UDF_FENTRY_SIZE,
        )
    } else {
        printf(format_args!("Invalid file entry!\n"));
        udf_vget_refused(&ump, ino);
        brelse(bp);
        return Err(Errno::ENOMEM);
    };

    // Allocate max size of FE/XFE: `size + UDF_EXTFENTRY_SIZE`, the C's `int` passed to
    // malloc's `size_t` (sign-extended, as the C's).
    let len = size.wrapping_add(UDF_EXTFENTRY_SIZE as i32) as isize as usize;
    let Some(mut fentry) = MArray::new(M_UDFFENTRY, M_NOWAIT, len, |_| 0u8) else {
        udf_vget_refused(&ump, ino);
        brelse(bp);
        return Err(Errno::ENOMEM); // Cannot allocate file entry block
    };

    // bcopy of `size` and the header, at most what the buffer holds (the module's
    // deviations).
    let n = (i64::from(size) + hdr as i64).clamp(0, fe.len().min(len) as i64) as usize;
    fentry[..n].copy_from_slice(&fe[..n]);

    brelse(bp);

    let up = Unode::new(ump.clone(), ino, Some(fentry)).alloc();

    let vp = match udf_allocv(mp) {
        Ok(vp) => vp,
        Err(e) => {
            // free(up->u_fentry); pool_put(&unode_pool, up)
            drop(up);
            return Err(e); // Error from udf_allocv()
        }
    };

    up.u_vnode.set(Some(vp));
    up.u_devvp.set(Some(ump.devvp()));
    vp.set_node(PoolArc::erase(up.clone()));
    vref(ump.devvp());

    // udf_hashins() will lock the vnode for us.
    let _ = udf_hashins(&up);

    let vtype = match up.fe.icbtag.file_type {
        UDF_ICB_FILETYPE_DIRECTORY => VDIR,
        UDF_ICB_FILETYPE_BLOCKDEVICE => VBLK,
        UDF_ICB_FILETYPE_CHARDEVICE => VCHR,
        UDF_ICB_FILETYPE_FIFO => VFIFO,
        UDF_ICB_FILETYPE_SOCKET => VSOCK,
        UDF_ICB_FILETYPE_SYMLINK => VLNK,
        UDF_ICB_FILETYPE_RANDOMACCESS | UDF_ICB_FILETYPE_REALTIME | UDF_ICB_FILETYPE_UNKNOWN => {
            VREG
        }
        _ => {
            printf(format_args!(
                "Unrecognized file type ({})\n",
                vp.vtype() as i32
            ));
            VREG
        }
    };
    vp.set_vtype(vtype);

    // check if this is a vnode alias
    let mut vp = vp;
    if let Some(nvp) = checkalias(vp, up.u_dev, Some(ump.um_mountp)) {
        printf(format_args!("found a vnode alias\n"));
        // Discard unneeded vnode, but save its udf_node. Note that the lock is carried over
        // in the udf_node
        if let Some(node) = vp.take_node() {
            nvp.set_node(node);
        }
        vp.set_ops_id(VopsId::SPEC);
        vrele(vp);
        vgone(vp);
        // Reinitialize aliased inode.
        vp = nvp;
        ump.um_devvp.set(Some(vp));
    }

    Ok(vp)
}

/// `udf_fhtovp` (`vfs_fhtovp`): the vnode a file handle names, locked; `ifid_ino` is the
/// `udfino_t` [`udf_vptofh`] stored, zero-extended (D-N5.8-U2, the module's deviations).
pub fn udf_fhtovp<G: ProcGlue>(
    mp: &'static Mount<G>,
    fhp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    let ifhp = Ifid::from_fid(fhp);

    VFS_VGET(mp, Ino::from(ifhp.ifid_ino.cast_unsigned()))
}

/// `udf_vptofh` (`vfs_vptofh`): sets the length and the file number of `fhp` (the rest of it
/// is left as it is, as in C).
pub fn udf_vptofh<G: ProcGlue>(vp: &'static Vnode<G>, fhp: &mut Fid) -> Result<(), Errno> {
    let up = VTOU(vp);
    fhp.fid_len = IFID_SIZE;
    fhp.fid_data[0..4].copy_from_slice(&up.u_ino.cast_signed().to_ne_bytes());

    Ok(())
}

/// Handle a virtual partition map.
pub fn udf_get_vpartmap<G: ProcGlue>(ump: &Umount<G>) -> Result<(), Errno> {
    ump.um_flags.set(ump.um_flags.get() | UDF_MNT_FIND_VAT); // Should do more than this
    Ok(())
}

/// Handle a sparable partition map. `st_loc` is the bytes that follow the map's fixed part
/// (the C's `pms->st_loc[]`). The table read is the mount's (`Umount::set_stbl`, which frees
/// the one it replaces); a table refused is freed (the module's deviations).
pub fn udf_get_spartmap<G: ProcGlue>(
    ump: &Umount<G>,
    pms: &PartMapSpare,
    st_loc: &[u8],
) -> Result<(), Errno> {
    let st_size = letoh32(pms.st_size);
    // ump->um_stbl = malloc(st_size, M_UDFMOUNT, M_NOWAIT); bzero(..)
    let Some(mut stbl) = MArray::new(M_UDFMOUNT, M_NOWAIT, st_size as usize, |_| 0u8) else {
        return Err(Errno::ENOMEM);
    };

    // Calculate the number of sectors per packet (`um_bsize` is a checked size: not 0).
    ump.um_psecs
        .set(i32::from(letoh16(pms.packet_len)) / ump.um_bsize.get());

    let Some(loc0) = pod_at::<u32>(st_loc, 0) else {
        return Err(Errno::EINVAL);
    };
    // Failed to read sparing table: the buffer released, the table freed.
    let bp = udf_readlblks(ump, letoh32(loc0) as i32, st_size as i32)?;

    let data = lblk_bytes(&bp);
    let n = stbl.len().min(data.len());
    stbl[..n].copy_from_slice(&data[..n]);
    brelse(bp);

    let Some(hdr) = UdfSparingTable::at(&stbl, 0) else {
        return Err(Errno::EINVAL);
    };
    if udf_checktag(&hdr.tag, 0).is_err() {
        return Err(Errno::EINVAL); // Invalid sparing table found
    }

    // See how many valid entries there are here. The list is supposed to be sorted,
    // 0xfffffff0 and higher are not valid. (A `rt_l` of 0 leaves the count of the table read
    // before: the C's.)
    for i in 0..usize::from(letoh16(hdr.rt_l)) {
        ump.um_stbl_len.set(i as i32);
        let Some(e) = SpareMapEntry::at(&stbl, UdfSparingTable::SIZE + i * SpareMapEntry::SIZE)
        else {
            break;
        };
        if letoh32(e.org) >= 0xffff_fff0 {
            break;
        }
    }

    ump.set_stbl(stbl);

    Ok(())
}

/// Handle a metadata partition map.
pub fn udf_get_mpartmap<G: ProcGlue>(ump: &Umount<G>, pmm: &PartMapMeta) -> Result<(), Errno> {
    ump.um_flags.set(ump.um_flags.get() | UDF_MNT_USES_META);
    ump.um_meta_start.set(pmm.meta_file_lbn);
    Ok(())
}

/// Scan the partition maps of the logical volume descriptor `lvd` (its bytes: the maps follow
/// its fixed part, the C's `lvd->maps[]`).
pub fn udf_find_partmaps<G: ProcGlue>(ump: &Umount<G>, lvd: &[u8]) -> Result<(), Errno> {
    let n_pm: u32 = member(lvd, offset_of!(LogvolDesc, n_pm));
    let maps = lvd.get(LogvolDesc::SIZE..).unwrap_or(&[]);
    let mut pmap = 0usize;

    for _ in 0..letoh32(n_pm) {
        let (Some(&ptype), Some(&psize)) = (maps.get(pmap), maps.get(pmap + 1)) else {
            return Err(Errno::EINVAL);
        };
        let psize = usize::from(psize);

        if ptype != 1 && ptype != 2 {
            return Err(Errno::EINVAL); // Invalid partition map type
        }

        if psize != PartMap1::SIZE && psize != PartMap2::SIZE {
            return Err(Errno::EINVAL); // Invalid partition map size
        }

        if ptype == 1 {
            pmap += PartMap1::SIZE;
            continue;
        }

        // Type 2 map. Find out the details (a map of 64 bytes, as each of the three kinds).
        let Some(pm2) = PartMap2::at(maps, pmap) else {
            return Err(Errno::EINVAL);
        };
        let regid_id = pm2.part_id.id;

        // The literals compared as the C's bcmp of UDF_REGID_ID_SIZE bytes does: "*UDF
        // Virtual Partition" is one byte shorter, its NUL takes part.
        let error = if &regid_id == b"*UDF Virtual Partition\0" {
            udf_get_vpartmap(ump)
        } else if &regid_id == b"*UDF Sparable Partition" {
            match PartMapSpare::at(maps, pmap) {
                Some(pms) => udf_get_spartmap(ump, &pms, &maps[pmap + PartMapSpare::SIZE..]),
                None => Err(Errno::EINVAL),
            }
        } else if &regid_id == b"*UDF Metadata Partition" {
            match PartMapMeta::at(maps, pmap) {
                Some(pmm) => udf_get_mpartmap(ump, &pmm),
                None => Err(Errno::EINVAL),
            }
        } else {
            return Err(Errno::EINVAL); // Unsupported partition map
        };

        error?; // Error getting partition

        pmap += PartMap2::SIZE;
    }

    Ok(())
}

/// `udf_vfsops`: each slot the C's function of the same name; `vfs_sysctl` the trait's
/// default (`EOPNOTSUPP`, the C's NULL); `vfs_checkexp` is `VFS_CHECKEXP`'s arm (the module's
/// deviations).
impl<G: ProcGlue> Vfs<G> for UdfVfs<G> {
    /// `vfs_mount`: [`udf_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        udf_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: [`udf_start`].
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno> {
        udf_start(mp, flags, p)
    }

    /// `vfs_unmount`: [`udf_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        udf_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: [`udf_root`].
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        udf_root(mp)
    }

    /// `vfs_quotactl`: [`udf_quotactl`].
    fn quotactl(
        &self,
        _mp: &'static Mount<G>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        udf_quotactl()
    }

    /// `vfs_statfs`: [`udf_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        udf_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`udf_sync`].
    fn sync(
        &self,
        _mp: &'static Mount<G>,
        _waitfor: i32,
        _stall: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        udf_sync()
    }

    /// `vfs_vget`: [`udf_vget`].
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        udf_vget(mp, ino)
    }

    /// `vfs_fhtovp`: [`udf_fhtovp`].
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        udf_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: [`udf_vptofh`].
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        udf_vptofh(vp, fid)
    }

    /// `vfs_init`: [`udf_init`].
    fn init(&self, _vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        udf_init::<G>()
    }
}

const _: () = {
    assert!(size_of::<Ifid>() == IFID_SIZE as usize);
    assert!(IFID_SIZE as usize <= size_of::<Fid>());
    assert!(VDS_DBLOCKS == 4);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the mount over testudf (mkudf's images on testffs's disk, mounted by
    // `udf_mountfs`; LZ's moved from `bsd`'s `udf_vnops.rs`): the test volume read through
    // the system calls and the vnode operations, the refusals in the C's order, D-N5.8-U3's
    // block sizes, a VAT volume, sparing tables, a metadata partition, what a failed mount
    // frees (the user's U4, D-u2-1), the pool request of a refused `vget` (D-N5.7-U5's
    // shim), file handles (D-N5.8-U2), every other `Vfs` and `Vnops` method, and the tag
    // check. User memory: testfs's path slots and buffer (`testfs::up`, `UBUF`), under the
    // kernel memory's guard every fixture holds.

    use core::cell::Cell;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::sys::dirent::{DT_DIR, DT_UNKNOWN, Dirent};
    use hal::sys::fcntl::O_RDONLY;
    use hal::sys::types::Register;
    use hal::sys::unistd::{_PC_LINK_MAX, _PC_NAME_MAX};

    use super::*;
    use crate::descrip::{sys_close, sys_read};
    use crate::fs::bio::biodone;
    use crate::fs::buf::{B_ERROR, BufRef};
    use crate::fs::isofs::udf::ecma167_udf::UDF_FILE_CHAR_PAR;
    use crate::fs::isofs::udf::testudf::mkudf::{
        self, B, META_START, PLEN, PSTART, TEXT, VAT_LASTBLOCK, fid, file_entry, short_ad,
        sparing_table, sub_name,
    };
    use crate::fs::isofs::udf::testudf::{
        TEST_UDF_CONF, attach, mount_root, root, try_mount, unmount,
    };
    use crate::fs::isofs::udf::udf::M_UDFMOUNT;
    use crate::fs::isofs::udf::udf_vnops::UDF_INVALID_BMAP;
    use crate::fs::mount::vfs_mount_alloc;
    use crate::fs::namei::LOCKLEAF;
    use crate::fs::subr::vput;
    use crate::fs::syscalls::{sys_getdents, sys_open};
    use crate::fs::testfs::{TestGlue, UBUF, set_bdev_strategy, up};
    use crate::fs::ufs::ffs::testffs::{self, set_disk};
    use crate::fs::vnode::{VWRITE, Vattr};
    use crate::fs::vops::{Vnops, vop_access, vop_bmap, vop_getattr, vop_read, vop_readdir};
    use crate::testutil::{malloc_out, malloc_requests, pool_requests, printf_count, watch_pool};
    use crate::uio::{Iovec, Uio};

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

    /// `getdents(2)` of the directory `path` with a buffer of `bufsize` bytes: (name, fileno,
    /// type) of each entry.
    fn list_dir(
        p: &Thread<TestGlue>,
        path: &[u8],
        bufsize: usize,
    ) -> Result<Vec<(Vec<u8>, u64, u8)>, Errno> {
        let fd = sys_open(p, up(0, path), O_RDONLY, 0)?;
        let mut out = Vec::new();
        let r = loop {
            let n = match sys_getdents(p, fd, UBUF, bufsize) {
                Ok(0) => break Ok(()),
                Ok(n) => n,
                Err(e) => break Err(e),
            };
            let mut b = vec![0u8; n];
            UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
            let mut off = 0;
            while off < n {
                let d = Dirent::from_bytes(&b[off..]).unwrap();
                let at = off + Dirent::NAME_OFFSET;
                let name = b[at..at + usize::from(d.d_namlen)].to_vec();
                assert_eq!(b[at + usize::from(d.d_namlen)], 0);
                out.push((name, d.d_fileno, d.d_type));
                off += usize::from(d.d_reclen);
            }
        };
        sys_close(p, fd).unwrap();
        r.map(|()| out)
    }

    /// The names of [`list_dir`].
    fn names(p: &Thread<TestGlue>, path: &[u8], bufsize: usize) -> Vec<Vec<u8>> {
        let ents = list_dir(p, path, bufsize).unwrap();
        ents.into_iter().map(|(n, _, _)| n).collect()
    }

    /// The vnode `path` names, locked and referenced.
    fn lookup(p: &Thread<TestGlue>, path: &[u8]) -> Result<&'static Vnode<TestGlue>, Errno> {
        let mut nd = ndinit(LOOKUP, LOCKLEAF | FOLLOW, NiDirp::Sys(path), p);
        namei(&mut nd)?;
        Ok(nd.ni_vp.unwrap())
    }

    /// `vop_getattr` of a locked vnode.
    fn getattr(p: &Thread<TestGlue>, vp: &'static Vnode<TestGlue>) -> Vattr {
        let mut va = Vattr::new();
        vop_getattr(vp, &mut va, Cred::real(p.ucred()), p).unwrap();
        va
    }

    /// `stat(path)`: the attributes of the vnode `namei` gives.
    fn stat(p: &Thread<TestGlue>, path: &[u8]) -> Result<Vattr, Errno> {
        let vp = lookup(p, path)?;
        let va = getattr(p, vp);
        vput(vp);
        Ok(va)
    }

    /// `VOP_READ` of `len` bytes at `offset` of a vnode, through a kernel `uio`.
    fn read_at(
        p: &Thread<TestGlue>,
        vp: &'static Vnode<TestGlue>,
        offset: i64,
        len: usize,
    ) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf[..], Some(p));
        uio.uio_offset = offset;
        vop_read(vp, &mut uio, 0, Cred::NOCRED).unwrap();
        let n = len - uio.uio_resid;
        buf.truncate(n);
        buf
    }

    std::thread_local! {
        /// The block the failing strategy refuses.
        static FAIL_BLKNO: Cell<Daddr> = const { Cell::new(-1) };
        /// The largest transfer the recording strategy saw.
        static MAX_BCOUNT: Cell<i64> = const { Cell::new(0) };
    }

    /// testffs's disk, but `EIO` for the block `FAIL_BLKNO` and for any block past the disk
    /// (a crafted sector), a transfer of no bytes done at once (testffs's disk would map
    /// the buffer), and the largest transfer recorded.
    fn checked_strategy(t: BufRef) {
        let b = t.buf();
        MAX_BCOUNT.with(|m| m.set(m.get().max(b.bcount())));
        if b.bcount() == 0 {
            biodone(t);
            return;
        }
        let past = (b.blkno() as u64).saturating_mul(512) >= ((PSTART + PLEN) * B) as u64;
        if b.blkno() == FAIL_BLKNO.with(Cell::get) || past {
            b.set_error(Some(Errno::EIO));
            b.set(B_ERROR);
            biodone(t);
            return;
        }
        testffs::disk_strategy(t);
    }

    /// What a mount holds that it gives back: `M_UDFMOUNT`'s and `M_UDFFENTRY`'s bytes out,
    /// the unodes out, the device vnode's references.
    fn held() -> (i64, i64, u64, u32) {
        (
            malloc_out(M_UDFMOUNT),
            malloc_out(M_UDFFENTRY),
            vfs::<TestGlue>().udf.unode_pool.nout(),
            testffs::devvp().usecount(),
        )
    }

    /// The test volume (LZ's): the mount's structure, the root, a file read through the
    /// system calls in pieces, its block map, a file recorded in its extended file entry,
    /// names that are not there, the directories (one recorded in its entry, one whose
    /// descriptors straddle its extents), "..", and an unmount that gives everything back.
    #[test]
    fn the_test_volume_mounts_reads_and_unmounts() {
        let (_g, p) = attach(mkudf::image().0);
        let before = held();
        let mp = mount_root(p, 0);
        let ump = VFSTOUDFFS(mp);
        assert_eq!(
            (ump.um_bsize.get(), ump.um_bshift.get(), ump.um_bmask.get()),
            (2048, 11, 2047)
        );
        assert_eq!(ump.um_start.get(), PSTART as u32);
        assert_eq!(ump.um_len.get(), PLEN as u32);
        assert_eq!(mp.flag() & MNT_LOCAL, MNT_LOCAL);
        let f_fsid = mp.stat().f_fsid;
        assert_eq!(f_fsid.val, [testffs::devvp().rdev(), 13]);
        assert!(core::ptr::eq(ump.devvp(), testffs::devvp()));
        drop(ump);

        let rvp = root(mp);
        assert_eq!(rvp.vtype(), VDIR);
        assert_ne!(rvp.flag() & VROOT, 0);
        assert_eq!(VTOU(rvp).u_ino, 2);
        let va = getattr(p, rvp);
        // A directory of two recorded blocks: 2 x 2048 bytes.
        assert_eq!((va.va_mode, va.va_nlink, va.va_size), (0o755, 2, 4096));
        vput(rvp);

        // The file: attributes, data in pieces, the block map, the cached vnode.
        let va = stat(p, b"/m10c-udf.txt").unwrap();
        assert_eq!((va.va_mode, va.va_uid, va.va_gid), (0o644, 501, 20));
        assert_eq!((va.va_size, va.va_bytes, va.va_fileid), (12, 12, 4));
        assert_eq!((va.va_gen, va.va_blocksize, va.va_type), (1, 2048, VREG));
        // The C's arithmetic (one day more than the date: it counts the day of the month).
        assert_eq!(va.va_mtime.tv_sec, 1_791_203_696);
        assert_eq!(read_file(p, b"/m10c-udf.txt").unwrap(), TEXT);
        let vp = lookup(p, b"/m10c-udf.txt").unwrap();
        for (off, len) in [(0, 12), (5, 4), (11, 9), (12, 3)] {
            assert_eq!(
                read_at(p, vp, off, len),
                &TEXT[off as usize..(off as usize + len).min(12)]
            );
        }
        let mut bn: Daddr = 0;
        let mut devvp = None;
        vop_bmap(vp, 0, Some(&mut devvp), Some(&mut bn), None).unwrap();
        assert_eq!(bn, ((PSTART + 5) * (B / 512)) as Daddr);
        assert!(devvp.is_some_and(|d| core::ptr::eq(d, testffs::devvp())));
        let vp2 = lookup(p, b"/m10c-udf.txt").unwrap();
        assert!(core::ptr::eq(vp, vp2));
        vput(vp2);
        vput(vp);

        // A file recorded in its extended file entry, owner -1 (shown as 0), time zone -60.
        assert_eq!(read_file(p, b"/emb.txt").unwrap(), b"hello");
        let va = stat(p, b"/emb.txt").unwrap();
        assert_eq!((va.va_size, va.va_uid), (5, 0));
        assert_eq!(va.va_mtime.tv_sec, 1_791_203_696 + 3600);
        let vp = lookup(p, b"/emb.txt").unwrap();
        assert_eq!(read_at(p, vp, 2, 10), b"llo");
        let mut bn: Daddr = 0;
        assert_eq!(
            vop_bmap(vp, 0, None, Some(&mut bn), None),
            Err(UDF_INVALID_BMAP)
        );
        vput(vp);

        // Names that are not there, the deleted one.
        assert_eq!(stat(p, b"/gone").err(), Some(Errno::ENOENT));
        assert_eq!(stat(p, b"/m10c-udf.tx").err(), Some(Errno::ENOENT));

        // The root directory.
        let ents = list_dir(p, b"/", 4096).unwrap();
        let want: Vec<(Vec<u8>, u64, u8)> = vec![
            (b".".to_vec(), 2, DT_DIR),
            (b"..".to_vec(), 2, DT_DIR),
            (b"m10c-udf.txt".to_vec(), 4, DT_UNKNOWN),
            (b"sub".to_vec(), 7, DT_DIR),
            (b"emb.txt".to_vec(), 6, DT_UNKNOWN),
            (b"edir".to_vec(), 13, DT_DIR),
        ];
        assert_eq!(ents, want);

        // The directory recorded in its file entry, whole and resumed.
        for len in [4096, 72] {
            assert_eq!(
                names(p, b"/edir", len),
                vec![&b"."[..], b"..", b"a1", b"b2", b"c3"],
                "buffer {len}"
            );
        }
        assert_eq!(read_file(p, b"/edir/c3").unwrap(), TEXT);

        // The directory whose descriptors straddle its extents (at 2040 and 4040), read whole
        // and in small pieces; lookups of the straddling names; ".." back to the root.
        for len in [8192, 100] {
            let ents = list_dir(p, b"/sub", len).unwrap();
            assert_eq!(ents.len(), 2 + mkudf::NSUB, "buffer {len}");
            for (i, (name, ino, _)) in ents[2..].iter().enumerate() {
                assert_eq!(name, &sub_name(i));
                assert_eq!(*ino, 4);
            }
        }
        for i in [0, 25, 26, 50, 51, mkudf::NSUB - 1] {
            let mut path = b"/sub/".to_vec();
            path.extend(sub_name(i));
            assert_eq!(read_file(p, &path).unwrap(), TEXT);
        }
        assert_eq!(stat(p, b"/sub").unwrap().va_size, 4096);
        assert_eq!(stat(p, b"/sub/..").unwrap().va_fileid, 2);

        unmount(p, mp);
        assert_eq!(held(), before);
    }

    /// The refusals of `udf_mountfs`, each in the C's order with its message, and what each
    /// gives back.
    #[test]
    fn damaged_volumes_are_not_mounted() {
        let (_g, p) = attach(mkudf::image().0);
        let before = held();
        let refused = |img: mkudf::Img, line: Option<&str>| {
            set_disk(img.0);
            let n = line.map(printf_count);
            assert_eq!(try_mount(p, 0).err(), Some(Errno::EINVAL), "{line:?}");
            if let (Some(line), Some(n)) = (line, n) {
                assert_eq!(printf_count(line), n + 1, "{line}");
            }
            assert_eq!(held(), before);
        };
        // The anchor's checksum.
        let mut img = mkudf::image();
        img.0[256 * B + 4] ^= 1;
        refused(img, None);
        // No partition descriptor.
        let mut img = mkudf::image();
        img.0[33 * B..34 * B].fill(0);
        refused(img, None);
        // A partition map of no known type, or of a type 2 name nobody knows.
        let mut img = mkudf::image();
        img.put(mkudf::LVD, LogvolDesc::SIZE, &[3, 6]);
        refused(img, None);
        let mut img = mkudf::image();
        img.map2(b"*UDF Unknown Partition\0", &[]);
        refused(img, None);
        // The file set descriptor in another partition.
        let mut img = mkudf::image();
        img.u16(mkudf::LVD, offset_of!(LogvolDesc, _lvd_use) + 8, 1);
        refused(img, Some("FSD does not lie within the partition!\n"));
        // No file set descriptor.
        let mut img = mkudf::image();
        img.lb(0, &[0u8; B]);
        refused(img, Some("Couldn't find the fsd\n"));
        // The root is no file entry.
        let mut img = mkudf::image();
        img.lb(2, &[0u8; B]);
        refused(img, Some("Invalid root file entry!\n"));
        // The root's ICB of no bytes: a buffer of none, the entry missing (D-u2-4).
        let mut img = mkudf::image();
        img.root_len(0);
        set_bdev_strategy(Some(checked_strategy));
        refused(img, Some("Invalid root file entry!\n"));
        set_bdev_strategy(Some(testffs::disk_strategy));
    }

    /// D-N5.8-U3: a logical block size the mount cannot use is `EINVAL` where the C takes
    /// it, and a root ICB longer than `MAXBSIZE` is `EINVAL` before its read.
    #[test]
    fn a_block_size_the_mount_cannot_use_is_einval() {
        let (_g, p) = attach(mkudf::image().0);
        let before = held();
        for lb in [0, 1000, 65_536 * 2] {
            let mut img = mkudf::image();
            img.lb_size(lb);
            set_disk(img.0);
            assert_eq!(try_mount(p, 0).err(), Some(Errno::EINVAL), "lb_size {lb}");
            assert_eq!(held(), before);
        }
        let mut img = mkudf::image();
        img.root_len(128 * 1024);
        set_disk(img.0);
        set_bdev_strategy(Some(checked_strategy));
        MAX_BCOUNT.with(|m| m.set(0));
        let n = printf_count("Cannot read sector 262\n");
        assert_eq!(try_mount(p, 0).err(), Some(Errno::EINVAL));
        assert_eq!(printf_count("Cannot read sector 262\n"), n + 1);
        assert_eq!(MAX_BCOUNT.with(Cell::get), B as i64);
        set_bdev_strategy(Some(testffs::disk_strategy));
        assert_eq!(held(), before);
    }

    /// A VAT volume (a virtual partition map): the VAT's file entry read at the mount, the
    /// root found only through the VAT, the VAT block given back by the unmount.
    #[test]
    fn a_vat_volume_finds_its_blocks_through_the_vat() {
        let (_g, p) = attach(mkudf::vat_image().0);
        let before = held();
        let (n, _) = malloc_requests(M_UDFMOUNT);
        let mp = mount_root(p, VAT_LASTBLOCK);
        // The structure, the hash and the VAT block: the C's three requests.
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n + 3);
        let ump = VFSTOUDFFS(mp);
        assert_eq!(
            ump.um_flags.get(),
            crate::fs::isofs::udf::udf::UDF_MNT_USES_VAT
        );
        assert!(ump.vat().is_some_and(|(_, len)| len == 16));
        drop(ump);
        assert_eq!(read_file(p, b"/m10c-udf.txt").unwrap(), TEXT);
        assert_eq!(names(p, b"/", 4096).len(), 6);
        unmount(p, mp);
        assert_eq!(held(), before);
    }

    /// What a failed mount made it gives back (the user's U4 and U6): the VAT block and the
    /// VAT's node, its file entry and its device reference (its vnode flushed, D-u2-1) when
    /// the file set descriptor is missing, where the C leaks the block and leaves the vnode
    /// on the freed mount; so a retry of the same medium fails as the first did (`EINVAL`),
    /// where OpenBSD answers `EBUSY` until that vnode is recycled.
    #[test]
    fn a_failed_mount_frees_the_vat() {
        let mut img = mkudf::vat_image();
        img.lb(0, &[0u8; B]);
        let (_g, p) = attach(img.0);
        let before = held();
        let (n, _) = malloc_requests(M_UDFMOUNT);
        assert_eq!(try_mount(p, VAT_LASTBLOCK).err(), Some(Errno::EINVAL));
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n + 3);
        assert_eq!(held(), before);
        // The same medium again: the mount reaches the same check, not the device's `EBUSY`.
        assert_eq!(try_mount(p, VAT_LASTBLOCK).err(), Some(Errno::EINVAL));
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n + 6);
        assert_eq!(held(), before);
        // The device is free again: a second mount of the fixed volume is made.
        set_disk(mkudf::vat_image().0);
        let mp = mount_root(p, VAT_LASTBLOCK);
        unmount(p, mp);
        assert_eq!(held(), before);
    }

    /// Sparing tables: one remaps the packet of the file's data (the C's `um_psecs` is
    /// `packet_len / um_bsize`); a second table replaces the first, which is freed (U4: the
    /// C's requests, one table in use); a refused second table frees both.
    #[test]
    fn sparing_tables_remap_and_replace() {
        const EOT: (u32, u32) = (0xffff_fff0, 0);
        let one = mkudf::sparing_image(4096, &[sparing_table(50, &[(4, 70), EOT])]);
        let (_g, p) = attach(one.0.clone());
        let before = held();
        let mut img = one;
        img.put(71, 0, b"m10c-spare!\n");
        set_disk(img.0);
        let (n0, _) = malloc_requests(M_UDFMOUNT);
        let mp = mount_root(p, 0);
        let one_out = malloc_out(M_UDFMOUNT) - before.0;
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n0 + 3);
        let ump = VFSTOUDFFS(mp);
        // The C's count: the index of the first entry not valid.
        assert_eq!((ump.um_psecs.get(), ump.um_stbl_len.get()), (2, 1));
        drop(ump);
        assert_eq!(read_file(p, b"/m10c-udf.txt").unwrap(), b"m10c-spare!\n");
        unmount(p, mp);
        assert_eq!(held(), before);

        // Two tables of the same size: the second is the mount's.
        let a = sparing_table(50, &[(4, 70), EOT]);
        let b = sparing_table(52, &[(4, 80), EOT]);
        let mut img = mkudf::sparing_image(4096, &[a.clone(), b]);
        img.put(71, 0, b"m10c-first\n\n");
        img.put(81, 0, b"m10c-later\n\n");
        set_disk(img.0);
        let (n1, _) = malloc_requests(M_UDFMOUNT);
        let mp = mount_root(p, 0);
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n1 + 4);
        assert_eq!(malloc_out(M_UDFMOUNT) - before.0, one_out);
        assert_eq!(read_file(p, b"/m10c-udf.txt").unwrap(), b"m10c-later\n\n");
        unmount(p, mp);
        assert_eq!(held(), before);

        // The second table's tag is wrong: refused, both tables freed.
        let mut c = sparing_table(52, &[(4, 80), EOT]);
        c[4] ^= 1;
        set_disk(mkudf::sparing_image(4096, &[a, c]).0);
        let (n2, _) = malloc_requests(M_UDFMOUNT);
        assert_eq!(try_mount(p, 0).err(), Some(Errno::EINVAL));
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n2 + 3);
        assert_eq!(held(), before);
    }

    /// A metadata partition: the file set read at its start, the directories walked from it
    /// (`meta_enter`), the start restored after every walk, a failing read of a directory's
    /// block included.
    #[test]
    fn a_metadata_partition_is_walked_from_its_start() {
        let (_g, p) = attach(mkudf::meta_image().0);
        let before = held();
        let mp = mount_root(p, 0);
        let ump = VFSTOUDFFS(mp);
        assert_eq!(
            (ump.um_meta_start.get(), ump.um_meta_len.get()),
            (META_START, 6 * B as u32)
        );
        assert_eq!(udf_getid(&ump.root_icb()), META_START + 1);
        assert_eq!(names(p, b"/", 4096), vec![&b"."[..], b"..", b"meta.txt"]);
        assert_eq!(ump.um_start.get(), PSTART as u32);
        assert_eq!(read_file(p, b"/meta.txt").unwrap(), TEXT);
        assert_eq!(
            (ump.um_start.get(), ump.um_len.get()),
            (PSTART as u32, PLEN as u32)
        );
        drop(ump);
        unmount(p, mp);

        // The root directory's block fails: EIO, the start restored.
        let blk = ((PSTART as u32 + META_START + 2) * 4) as Daddr;
        FAIL_BLKNO.with(|c| c.set(blk));
        set_bdev_strategy(Some(checked_strategy));
        let mp = mount_root(p, 0);
        assert_eq!(list_dir(p, b"/", 4096).err(), Some(Errno::EIO));
        let ump = VFSTOUDFFS(mp);
        assert_eq!(
            (ump.um_start.get(), ump.um_len.get()),
            (PSTART as u32, PLEN as u32)
        );
        drop(ump);
        FAIL_BLKNO.with(|c| c.set(-1));
        set_bdev_strategy(Some(testffs::disk_strategy));
        unmount(p, mp);
        assert_eq!(held(), before);
    }

    /// A `vget` refused before its unode is made still makes and frees one item of
    /// `udfndpl`, as the C's `pool_get` first (D-N5.7-U5's shim); a `vget` that makes its
    /// unode makes one.
    #[test]
    fn a_refused_vget_makes_the_c_s_pool_request() {
        let (_g, p) = attach(mkudf::image().0);
        let mp = mount_root(p, 0);
        let items = vfs::<TestGlue>().udf.unode_pool.nout();
        watch_pool("udfndpl");
        // lb 1 holds no file entry: the C's ENOMEM.
        let n = printf_count("Invalid file entry!\n");
        assert_eq!(VFS_VGET(mp, 1).err(), Some(Errno::ENOMEM));
        assert_eq!(printf_count("Invalid file entry!\n"), n + 1);
        assert_eq!(pool_requests(), 1);
        assert_eq!(vfs::<TestGlue>().udf.unode_pool.nout(), items);
        // A read error.
        FAIL_BLKNO.with(|c| c.set(((PSTART + 9) * 4) as Daddr));
        set_bdev_strategy(Some(checked_strategy));
        assert_eq!(VFS_VGET(mp, 9).err(), Some(Errno::EIO));
        assert_eq!(pool_requests(), 2);
        assert_eq!(vfs::<TestGlue>().udf.unode_pool.nout(), items);
        FAIL_BLKNO.with(|c| c.set(-1));
        set_bdev_strategy(Some(testffs::disk_strategy));
        // A file entry: its unode, out until the vnode is reclaimed.
        let vp = VFS_VGET(mp, 4).unwrap();
        assert_eq!(pool_requests(), 3);
        assert_eq!(vfs::<TestGlue>().udf.unode_pool.nout(), items + 1);
        vput(vp);
        unmount(p, mp);
    }

    /// A handle's round trip; a handle whose `ifid_ino` has its top bit set is the `udfino_t`
    /// zero-extended (D-N5.8-U2): a block past the disk, `EIO`, where OpenBSD panics "alien
    /// ino_t".
    #[test]
    fn file_handles_name_their_file_entries() {
        let (_g, p) = attach(mkudf::image().0);
        let mp = mount_root(p, 0);
        let vp = lookup(p, b"/m10c-udf.txt").unwrap();
        let mut fid = Fid::default();
        fid.fid_reserved = 7;
        udf_vptofh(vp, &mut fid).unwrap();
        let ifid = Ifid::from_fid(&fid);
        assert_eq!((ifid.ifid_len, ifid.ifid_pad, ifid.ifid_ino), (16, 7, 4));
        vput(vp);
        let nvp = udf_fhtovp(mp, &fid).unwrap();
        assert!(core::ptr::eq(nvp, vp));
        vput(nvp);
        set_bdev_strategy(Some(checked_strategy));
        fid.fid_data[0..4].copy_from_slice(&(-0x7fff_fffci32).to_ne_bytes());
        assert_eq!(udf_fhtovp(mp, &fid).err(), Some(Errno::EIO));
        set_bdev_strategy(Some(testffs::disk_strategy));
        unmount(p, mp);
    }

    /// `udf_mount`'s own checks, and the `Vfs` and `Vnops` methods the other tests do not
    /// reach, as the C answers them.
    #[test]
    fn the_other_operations_answer_as_the_c() {
        let (_g, p) = attach(mkudf::image().0);
        let vfs_ops = &UdfVfs::<TestGlue>::VFS;
        // mount(2): read-only always; no root file system; an update does nothing; short
        // arguments.
        let mp = vfs_mount_alloc(None, &TEST_UDF_CONF);
        mp.set_flag(mp.flag() | MNT_ROOTFS);
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/"), p);
        assert_eq!(
            vfs_ops.mount(mp, b"/", &mut [0u8; UdfArgs::SIZE], &mut nd, p),
            Err(Errno::EOPNOTSUPP)
        );
        assert_ne!(mp.flag() & MNT_RDONLY, 0);
        mp.set_flag((mp.flag() & !MNT_ROOTFS) | MNT_UPDATE);
        assert_eq!(vfs_ops.mount(mp, b"/", &mut [], &mut nd, p), Ok(()));
        mp.set_flag(mp.flag() & !MNT_UPDATE);
        assert_eq!(
            vfs_ops.mount(mp, b"/", &mut [0u8; 4], &mut nd, p),
            Err(Errno::EINVAL)
        );
        crate::fs::mount::vfs_unbusy(mp);
        crate::fs::mount::vfs_mount_free(mp);

        let mp = mount_root(p, 0);
        let mut st = mp.stat();
        vfs_ops.statfs(mp, &mut st, p).unwrap();
        assert_eq!(
            (st.f_bsize, st.f_iosize, st.f_blocks),
            (2048, 2048, PLEN as u64)
        );
        assert_eq!((st.f_bfree, st.f_files, st.f_ffree), (0, 0, 0));
        assert_eq!(vfs_ops.start(mp, 0, p), Ok(()));
        assert_eq!(vfs_ops.quotactl(mp, 0, 0, 0, p), Err(Errno::EOPNOTSUPP));
        assert_eq!(vfs_ops.sync(mp, 0, 0, Cred::NOCRED, p), Ok(()));

        let rvp = root(mp);
        let ops = &vfs::<TestGlue>().udf.vops;
        let mut r: Register = 0;
        for (name, want) in [(_PC_LINK_MAX, 65535), (_PC_NAME_MAX, 255)] {
            ops.pathconf(rvp, name, &mut r).unwrap();
            assert_eq!(r, want);
        }
        assert_eq!(ops.pathconf(rvp, 999, &mut r), Err(Errno::EINVAL));
        assert_eq!(
            vop_access(rvp, VWRITE, Cred::real(p.ucred()), p),
            Err(Errno::EROFS)
        );
        assert_eq!(
            ops.ioctl(rvp, 0, &mut [], 0, Cred::NOCRED, p),
            Err(Errno::ENOTTY)
        );
        let mut eof = 0;
        let mut buf = [0u8; 8];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf[..], Some(p));
        // A buffer too small for ".": the C's -1 out of the loop (ERESTART's value).
        assert_eq!(
            vop_readdir(rvp, &mut uio, Cred::NOCRED, &mut eof),
            Err(Errno::ERESTART)
        );
        assert_eq!(eof, 0);
        let mut uio = Uio::kernel_read(&mut iov, &mut buf[..], Some(p));
        assert_eq!(
            ops.readlink(rvp, &mut uio, Cred::NOCRED),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(ops.open(rvp, 0, Cred::NOCRED, p), Ok(()));
        assert_eq!(ops.close(rvp, 0, Cred::NOCRED, Some(p)), Ok(()));
        assert_eq!(ops.print(rvp), Ok(()));
        vput(rvp);
        unmount(p, mp);
    }

    /// A file recorded in its extended file entry read from offset 0 and 100; a directory
    /// whose descriptor ends past its chunk is an invalid fragment (`EINVAL`, LZ's bound).
    #[test]
    fn in_entry_data_and_a_fragment_past_its_chunk() {
        let mut img = mkudf::image();
        let data: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
        img.lb(6, &file_entry(6, true, 5, 3, 0x1884, 0, 300, &data));
        // lb 14: a directory of 80 bytes whose first extent holds 42: a parent descriptor of
        // 41 bytes (l_iu 3), padded to 44, past it.
        let mut parent = fid(UDF_FILE_CHAR_PAR, b"", 2);
        parent.resize(44, 0);
        parent[36..38].copy_from_slice(&3u16.to_le_bytes());
        let mut ads = Vec::new();
        ads.extend(short_ad(42, 9));
        ads.extend(short_ad(38, 11));
        img.lb(14, &file_entry(14, false, 4, 0, 0x1ca5, 0, 80, &ads));
        img.lb(9, &parent);
        let (_g, p) = attach(img.0);
        let mp = mount_root(p, 0);
        let vp = lookup(p, b"/emb.txt").unwrap();
        assert_eq!(read_at(p, vp, 0, 400), data);
        assert_eq!(read_at(p, vp, 100, 400), &data[100..]);
        vput(vp);
        let dvp = VFS_VGET(mp, 14).unwrap();
        let n = printf_count("udf: invalid FID fragment\n");
        let mut eof = 0;
        let mut buf = [0u8; 512];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf[..], Some(p));
        assert_eq!(
            vop_readdir(dvp, &mut uio, Cred::NOCRED, &mut eof),
            Err(Errno::EINVAL)
        );
        assert_eq!(printf_count("udf: invalid FID fragment\n"), n + 1);
        vput(dvp);
        unmount(p, mp);
    }

    /// A tag: `id`, the checksum `sum` (the right one when `None`), the rest of LZ's test's.
    fn tag(id: u16, sum: Option<u8>) -> DescTag {
        let mut t = [0u8; 16];
        t[0..2].copy_from_slice(&id.to_le_bytes());
        t[2..4].copy_from_slice(&2u16.to_le_bytes());
        t[12..16].copy_from_slice(&0xfedc_ba98u32.to_le_bytes());
        let good = t
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 4 && i < 15)
            .fold(0u8, |s, (_, &b)| s.wrapping_add(b));
        t[4] = sum.unwrap_or(good);
        DescTag::at(&t, 0).unwrap()
    }

    #[test]
    fn checktag_wants_the_id_and_the_sum() {
        assert_eq!(udf_checktag(&tag(2, None), 2), Ok(()));
        assert_eq!(udf_checktag(&tag(2, None), 1), Err(Errno::EINVAL));
        let good = tag(2, None).cksum;
        assert_eq!(
            udf_checktag(&tag(2, Some(good.wrapping_add(1))), 2),
            Err(Errno::EINVAL)
        );
        // Byte 15 (the high byte of tag_loc) is outside the sum, as in the C (i < 15).
        let mut b = tag(261, None).as_bytes();
        b[15] ^= 0xff;
        assert_eq!(udf_checktag(&DescTag::at(&b, 0).unwrap(), 261), Ok(()));
        b[14] ^= 0xff;
        assert_eq!(
            udf_checktag(&DescTag::at(&b, 0).unwrap(), 261),
            Err(Errno::EINVAL)
        );
    }
}
/* </TESTS> */
