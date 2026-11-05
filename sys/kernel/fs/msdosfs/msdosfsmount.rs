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
//! `<msdosfs/msdosfsmount.h>`: the mount control block of a msdos file system (`struct
//! msdosfsmount`), hung from the mount's typed slot (`mnt_data`), the `MSDOSFSMNT_*` flags and
//! the macros that convert between byte offsets, blocks and clusters.
//!
//! Upstream: sys/msdosfs/msdosfsmount.h @ 3ce1f3f79392
//! LZ: sys/msdosfs/msdosfsmount.rs@3c62ede68802
//!
//! ## Deviations
//! - `pm_export` (`struct netexport`) is not a member: the export list of every kernel file
//!   system's mount is the mount's glue member (`MountG`, `bsd`'s `BsdMountG.export`, the UFS
//!   mounts' since N5.5b0), which `msdosfs_mount` updates through `FileGlue::vfs_export` and
//!   `msdosfs_mountfs` starts empty through `FileGlue::ufs_export_init` (N5.7c3); the C keeps
//!   it here whether or not `NFSSERVER` is configured.
//! - The `pm_BytesPerSec`, ... shorthands of `pm_bpb` are methods with the macros' names
//!   ([`bpb_shorthand!`]); `msdosfs_mountfs` assigns the members of `pm_bpb` before the
//!   structure is published.
//! - The flags are `u32`, the type of `pm_flags`; `MSDOSFSMNT_SHORTNAME`, `_LONGNAME` and
//!   `_NOWIN95` are the `<sys/mount.h>` ones (`hal::sys::mount`), as the C's `#if 0` says.
//! - The arithmetic macros (`de_cluster`, `cntobn`, ...) are generic functions over the
//!   [`DeArith`] integers, so each computes in the type of its argument as the macro does
//!   (an `off_t` offset stays 64-bit, a cluster number wraps at 32 bits); the macros' unsigned
//!   additions and subtractions wrap. `bptoep` returns the entry's offset in the buffer (the
//!   caller reads it with `Direntry::at`). `FATOFS` computes in `u32`, the type of `cn` and of
//!   the multipliers.
//! - `fsi_size(pmp)` shifts `1024` by `pm_BlkPerSec >> 2` with the machines' masking of a
//!   32-bit shift count (`wrapping_shl`). The count is at most 16 from a disk (`bpbBytesPerSec`
//!   is 16-bit and `msdosfs_mountfs` requires a power of two of at least 512, so 32 KiB
//!   sectors at most); a count of 32 (the C's undefined `1024 << 32`) is unreachable, and
//!   would give 1024, as both machines' shift instructions (x86's `shl`, arm64's `lslv`
//!   W-form) take the count modulo 32. [`fsi_size_checked`] refuses with `EINVAL` an FSInfo size
//!   above `MAXBSIZE` (FAT32 with sectors of 16 KiB or more: 256 KiB, 64 MiB), the user's
//!   decision D-N5.7-U3 of 2026-11-05: OpenBSD's `bread` of such a size maps pages past its
//!   buffer's `MAXPHYS` slot (`buf_map`, `vfs_biomem.c`) where the kernel's panics;
//!   `msdosfs_mountfs` calls it before the FSInfo read (N5.7c3).
//! - The prototypes of the virtual filesystem operations (`msdosfs_mount`, ...) are the
//!   functions of `msdosfs_vfsops.rs`; `msdosfs_init` is in `msdosfs_denode.rs`, where the C
//!   defines it.
//!
//! ## Redesign
//! - LZ's `Msdosfsmount` (31 `Cell`s, `malloc(M_MSDOSFSMNT)` written with `ptr::write` into
//!   `mnt_data` and cast back by `vfstomsdosfs`'s `unsafe` deref behind a name check, every
//!   denode holding a `&'static`; `pm_inusemap` a raw `Cell<*mut u32>` to a `mallocarray`,
//!   viewed through `from_raw_parts`; an `unsafe impl Sync`) -> a [`Msdosfsmount`] built whole
//!   by `msdosfs_mountfs` before it is published (D-N5.7-5): the identity (`pm_mountp`,
//!   `pm_devvp`, `pm_dev`), the BPB and the geometry are plain members the C never changes
//!   after the mount, read with one load as the C reads them; the members the C writes after
//!   the structure is hung (`pm_uid`, `pm_gid`, `pm_mask` by `msdosfs_mount` after
//!   `msdosfs_mountfs` and on an update; `pm_flags` by RW<->RO and `WAITONFAT`; `pm_fmod`;
//!   `pm_freeclustercount`; `pm_fsinfo`, which `updatefats` clears after a failed read) are
//!   relaxed words written under the kernel lock (`fs::word`, docs/IDIOMS.md's counters' row:
//!   every msdosfs path is `KERNEL_LOCK`ed); the bitmap is an `MArray<AtomicU32>` of
//!   `M_MSDOSFSFAT` ([`inusemap_alloc`], the C's `mallocarray(M_CANFAIL)`, `None` -> `EINVAL`),
//!   its words changed by a load and a store under the same lock. The structure is published
//!   in a counted `malloc(M_MSDOSFSMNT)` block ([`MsdosfsmountRef`], `hal::malloc::MArc`;
//!   `vmstat -m`'s `MSDOSFS mount` row, 16 bytes more per block for the count and the type, as
//!   the `UFS mount` row) erased into the mount's typed slot, where the C sets `mnt_data`; the
//!   unmount takes it back after `vflush` and the last handle frees the block. The structure
//!   is a C in-core structure `malloc(M_ZERO)`ed, filled member by member, then hung: built
//!   whole, published counted, the members changed after publication words (docs/IDIOMS.md
//!   row, Mfsnode the first instance).
//! - `VFSTOMSDOSFS(mp)` ([`vfstomsdosfs`]) is the slot's typed read, a counted handle (the
//!   type replaces LZ's `vfc_name` check; `panic(9)` on another file system's mount). Cost
//!   against the C's one load: the slot's read (two atomics) and a count pair, once per `Vfs`
//!   call (ROADMAP N12's row); a denode holds one handle for its life and borrows it
//!   (`dep.pmp()`, one load, the C's `dep->de_pmp`).
//! - LZ's `mountp()`/`devvp()` (an `Option` checked on every use) and `inusemap()` (the raw
//!   view) -> the members themselves; LZ's zeroed `new()` and its `Default` go: nothing makes
//!   a structure that `msdosfs_mountfs` has not filled.

use core::ops::{Shl, Shr};
use core::sync::atomic::AtomicU32;

use hal::malloc::{M_CANFAIL, M_WAITOK, MArc, MArray};
use hal::sys::errno::Errno;
use hal::sys::mount::{MSDOSFSMNT_LONGNAME, MSDOSFSMNT_NOWIN95, MSDOSFSMNT_SHORTNAME};
use hal::sys::types::Dev;

use crate::assert::kpanic;
use crate::fs::mount::Mount;
use crate::fs::msdosfs::bpb::Bpb50;
use crate::fs::msdosfs::fat::{CLUST_FIRST, MSDOSFSROOT};
use crate::fs::vnode::Vnode;
use crate::fs::word::U32Word;
use crate::proc::ProcGlue;
use crate::vm::io::MAXBSIZE;

/// `M_MSDOSFSMNT`: the malloc type of the msdos mount structures (`<sys/malloc.h>`;
/// `vmstat -m`'s `MSDOSFS mount`).
pub const M_MSDOSFSMNT: i32 = 59;
/// `M_MSDOSFSFAT`: the malloc type of the in-use bitmaps (`vmstat -m`'s `MSDOSFS fat`).
pub const M_MSDOSFSFAT: i32 = 60;

/// `MSDOSFSMNT_MNTOPT`: all the `<sys/mount.h>` flags.
pub const MSDOSFSMNT_MNTOPT: u32 =
    (MSDOSFSMNT_SHORTNAME | MSDOSFSMNT_LONGNAME | MSDOSFSMNT_NOWIN95) as u32;
/// `MSDOSFSMNT_RONLY`: mounted read-only.
pub const MSDOSFSMNT_RONLY: u32 = 0x8000_0000;
/// `MSDOSFSMNT_WAITONFAT`: mounted synchronous.
pub const MSDOSFSMNT_WAITONFAT: u32 = 0x4000_0000;
/// `MSDOSFS_FATMIRROR`: FAT is mirrored.
pub const MSDOSFS_FATMIRROR: u32 = 0x2000_0000;

/// `N_INUSEBITS`: number of bits in one `pm_inusemap` item.
pub const N_INUSEBITS: u32 = 8 * size_of::<u32>() as u32;

/// Generates a `pm_bpb` shorthand: `pmp->pm_x`, read.
macro_rules! bpb_shorthand {
    ($(#[$doc:meta])* $get:ident, $field:ident, $t:ty) => {
        $(#[$doc])*
        #[allow(non_snake_case)] // the C macro's name
        #[inline]
        pub fn $get(&self) -> $t {
            self.pm_bpb.$field
        }
    };
}

/// `struct msdosfsmount`: layout of the mount control block for a msdos file system, built
/// whole by `msdosfs_mountfs` and published as a [`MsdosfsmountRef`] (the module's redesign).
///
/// Protected by: nothing but the kernel lock, as in the C; the plain members never change
/// once the structure is built, the words are written under the kernel lock.
#[allow(non_snake_case)] // the C field names
pub struct Msdosfsmount<G: ProcGlue> {
    /// `pm_mountp`: vfs mount struct for this fs.
    pub pm_mountp: &'static Mount<G>,
    /// `pm_dev`: block special device mounted.
    pub pm_dev: Dev,
    /// `pm_uid`: uid to set as owner of the files (`msdosfs_mount`, after `mountfs`).
    pub pm_uid: U32Word,
    /// `pm_gid`: gid to set as owner of the files.
    pub pm_gid: U32Word,
    /// `pm_mask`: mask to and with file protection bits.
    pub pm_mask: U32Word,
    /// `pm_devvp`: vnode for block device mntd.
    pub pm_devvp: &'static Vnode<G>,
    /// `pm_bpb`: BIOS parameter blk for this fs.
    pub pm_bpb: Bpb50,
    /// `pm_BlkPerSec`: # of `DEV_BSIZE` blocks in MSDOSFS sector.
    pub pm_BlkPerSec: u32,
    /// `pm_FATsecs`: actual number of fat sectors.
    pub pm_FATsecs: u32,
    /// `pm_fatblk`: block # of first FAT.
    pub pm_fatblk: u32,
    /// `pm_rootdirblk`: block # (cluster # for FAT32) of root directory number.
    pub pm_rootdirblk: u32,
    /// `pm_rootdirsize`: size in blocks (not clusters).
    pub pm_rootdirsize: u32,
    /// `pm_firstcluster`: block number of first cluster.
    pub pm_firstcluster: u32,
    /// `pm_nmbrofclusters`: # of clusters in filesystem.
    pub pm_nmbrofclusters: u32,
    /// `pm_maxcluster`: maximum cluster number.
    pub pm_maxcluster: u32,
    /// `pm_freeclustercount`: number of free clusters.
    pub pm_freeclustercount: U32Word,
    /// `pm_cnshift`: shift file offset right this amount to get a cluster number.
    pub pm_cnshift: u32,
    /// `pm_crbomask`: and a file offset with this mask to get cluster rel offset.
    pub pm_crbomask: u32,
    /// `pm_bnshift`: shift file offset right this amount to get a block number.
    pub pm_bnshift: u32,
    /// `pm_bpcluster`: bytes per cluster.
    pub pm_bpcluster: u32,
    /// `pm_fmod`: ~0 if fs is modified, this can rollover to 0.
    pub pm_fmod: U32Word,
    /// `pm_fatblocksize`: size of fat blocks in bytes.
    pub pm_fatblocksize: u32,
    /// `pm_fatblocksec`: size of fat blocks in sectors.
    pub pm_fatblocksec: u32,
    /// `pm_fatsize`: size of fat in bytes.
    pub pm_fatsize: u32,
    /// `pm_fatmask`: mask to use for fat numbers.
    pub pm_fatmask: u32,
    /// `pm_fsinfo`: fsinfo block number (0 once `updatefats` failed to read it).
    pub pm_fsinfo: U32Word,
    /// `pm_fatmult`: these 2 values are used in fat offset computation.
    pub pm_fatmult: u32,
    /// `pm_fatdiv`: (see `pm_fatmult`).
    pub pm_fatdiv: u32,
    /// `pm_curfat`: current fat for FAT32 (0 otherwise).
    pub pm_curfat: u32,
    /// `pm_inusemap`: bitmap of in-use clusters, `howmany(pm_maxcluster + 1, N_INUSEBITS)`
    /// words of `M_MSDOSFSFAT` ([`inusemap_alloc`]).
    pub pm_inusemap: MArray<AtomicU32>,
    /// `pm_flags`: the `MSDOSFSMNT_*` flags above.
    pub pm_flags: U32Word,
}

/// `struct msdosfsmount *` held: a counted handle to a mount's msdos structure, in a block of
/// `malloc(M_MSDOSFSMNT)` that the last handle frees. The mount's typed slot holds one while
/// it is mounted, each denode one for its life, each operation reached from the mount one
/// ([`vfstomsdosfs`]).
pub type MsdosfsmountRef<G> = MArc<Msdosfsmount<G>>;

impl<G: ProcGlue> Msdosfsmount<G> {
    /// `pmp = malloc(sizeof *pmp, M_MSDOSFSMNT, M_WAITOK | M_ZERO)` holding the structure
    /// `msdosfs_mountfs` built: one handle, which the mount hangs from its typed slot
    /// (`Mount::set_node`). `M_WAITOK` sleeps for the memory and never fails, as the C's; no
    /// `M_ZERO`: the block is written whole.
    pub fn alloc(self) -> MsdosfsmountRef<G> {
        match MArc::new(M_MSDOSFSMNT, M_WAITOK, self) {
            Some(pmp) => pmp,
            None => kpanic(format_args!("msdosfs: malloc(M_MSDOSFSMNT) failed")),
        }
    }

    bpb_shorthand!(
        /// `pm_BytesPerSec` (`pm_bpb.bpbBytesPerSec`).
        pm_BytesPerSec, bpbBytesPerSec, u16
    );
    bpb_shorthand!(
        /// `pm_ResSectors` (`pm_bpb.bpbResSectors`).
        pm_ResSectors, bpbResSectors, u16
    );
    bpb_shorthand!(
        /// `pm_FATs` (`pm_bpb.bpbFATs`).
        pm_FATs, bpbFATs, u8
    );
    bpb_shorthand!(
        /// `pm_RootDirEnts` (`pm_bpb.bpbRootDirEnts`).
        pm_RootDirEnts, bpbRootDirEnts, u16
    );
    bpb_shorthand!(
        /// `pm_Sectors` (`pm_bpb.bpbSectors`).
        pm_Sectors, bpbSectors, u16
    );
    bpb_shorthand!(
        /// `pm_Media` (`pm_bpb.bpbMedia`).
        pm_Media, bpbMedia, u8
    );
    bpb_shorthand!(
        /// `pm_SecPerTrack` (`pm_bpb.bpbSecPerTrack`).
        pm_SecPerTrack, bpbSecPerTrack, u16
    );
    bpb_shorthand!(
        /// `pm_Heads` (`pm_bpb.bpbHeads`).
        pm_Heads, bpbHeads, u16
    );
    bpb_shorthand!(
        /// `pm_HiddenSects` (`pm_bpb.bpbHiddenSecs`).
        pm_HiddenSects, bpbHiddenSecs, u32
    );
    bpb_shorthand!(
        /// `pm_HugeSectors` (`pm_bpb.bpbHugeSectors`).
        pm_HugeSectors, bpbHugeSectors, u32
    );
}

/// The integer types the offset and cluster macros are applied to (`uint32_t` cluster and
/// block numbers, `daddr_t` and `off_t` offsets): each macro computes in its argument's type,
/// with C's unsigned wrapping.
pub trait DeArith: Copy + Shl<u32, Output = Self> + Shr<u32, Output = Self> {
    /// The `uint32_t` value `v` in this type (the C's usual arithmetic conversion).
    fn from_u32(v: u32) -> Self;
    /// `self + o`, wrapping.
    #[must_use]
    fn wadd(self, o: Self) -> Self;
    /// `self - o`, wrapping.
    #[must_use]
    fn wsub(self, o: Self) -> Self;
}

/// `pmp->pm_inusemap = mallocarray(howmany(maxcluster + 1, N_INUSEBITS), sizeof(u_int),
/// M_MSDOSFSFAT, M_WAITOK | M_CANFAIL)` (`msdosfs_mountfs`), every word 0: `EINVAL` for an
/// empty bitmap (`maxcluster + 1` wrapped), a size that overflows (the C's `SIZE_MAX /
/// bmapsiz` check) or a `NULL` from `malloc`, as the C answers each.
pub fn inusemap_alloc(maxcluster: u32) -> Result<MArray<AtomicU32>, Errno> {
    // `pm_maxcluster + 1` wraps at its `uint32_t`; `howmany` then computes in `size_t`, the
    // type of `N_INUSEBITS` (`8 * sizeof(u_int)`), where its `+ 31` cannot wrap: the
    // `usize` `div_ceil`, which never wraps, is that value.
    let bmapsiz = (maxcluster.wrapping_add(1) as usize).div_ceil(N_INUSEBITS as usize);
    if bmapsiz == 0 || usize::MAX / bmapsiz < size_of::<u32>() {
        return Err(Errno::EINVAL);
    }
    MArray::new(M_MSDOSFSFAT, M_WAITOK | M_CANFAIL, bmapsiz, |_| {
        AtomicU32::new(0)
    })
    .ok_or(Errno::EINVAL)
}

/// `FATOFS(pmp, cn)`: byte offset in FAT on filesystem `pmp`, cluster `cn`. `pm_fatdiv` is 1
/// or 2, set by `msdosfs_mountfs` with the FAT's width, never read from the disk.
pub fn fatofs<G: ProcGlue>(pmp: &Msdosfsmount<G>, cn: u32) -> u32 {
    cn.wrapping_mul(pmp.pm_fatmult) / pmp.pm_fatdiv
}

/// `VFSTOMSDOSFS(mp)`: the msdos mount structure of `mp`, held. `panic(9)` when the mount is
/// not a mounted msdos file system (the slot holds none, or another file system's).
pub fn vfstomsdosfs<G: ProcGlue>(mp: &Mount<G>) -> MsdosfsmountRef<G> {
    match mp.node_arc::<Msdosfsmount<G>>() {
        Some(pmp) => pmp,
        None => kpanic(format_args!(
            "VFSTOMSDOSFS: mount {mp:p} is not a mounted msdos file system"
        )),
    }
}

/// Implements [`DeArith`] for integer types.
macro_rules! de_arith {
    ($($t:ty),*) => {
        $(impl DeArith for $t {
            fn from_u32(v: u32) -> Self {
                v as $t
            }
            fn wadd(self, o: Self) -> Self {
                self.wrapping_add(o)
            }
            fn wsub(self, o: Self) -> Self {
                self.wrapping_sub(o)
            }
        })*
    };
}

de_arith!(u32, u64, i64);

/// `bptoep(pmp, bp, dirofs)`: convert pointer to buffer -> pointer to direntry; here the
/// offset of the entry in the buffer's data.
pub fn bptoep<G: ProcGlue>(pmp: &Msdosfsmount<G>, dirofs: u32) -> usize {
    (dirofs & pmp.pm_crbomask) as usize
}

/// `de_bn2cn(pmp, bn)`: convert block number to cluster number.
pub fn de_bn2cn<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, bn: T) -> T {
    bn >> pmp.pm_cnshift.wrapping_sub(pmp.pm_bnshift)
}

/// `de_cn2bn(pmp, cn)`: convert cluster number to block number.
pub fn de_cn2bn<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, cn: T) -> T {
    cn << pmp.pm_cnshift.wrapping_sub(pmp.pm_bnshift)
}

/// `de_cluster(pmp, off)`: convert file offset to cluster number.
pub fn de_cluster<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, off: T) -> T {
    off >> pmp.pm_cnshift
}

/// `de_clcount(pmp, size)`: clusters required to hold size bytes.
pub fn de_clcount<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, size: T) -> T {
    size.wadd(T::from_u32(pmp.pm_bpcluster))
        .wsub(T::from_u32(1))
        >> pmp.pm_cnshift
}

/// `de_blk(pmp, off)`: convert file offset to block number.
pub fn de_blk<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, off: T) -> T {
    de_cn2bn(pmp, de_cluster(pmp, off))
}

/// `de_cn2off(pmp, cn)`: convert cluster number to file offset.
pub fn de_cn2off<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, cn: T) -> T {
    cn << pmp.pm_cnshift
}

/// `de_bn2off(pmp, bn)`: convert block number to file offset.
pub fn de_bn2off<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, bn: T) -> T {
    bn << pmp.pm_bnshift
}

/// `cntobn(pmp, cn)`: map a cluster number into a filesystem relative block number.
pub fn cntobn<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, cn: T) -> T {
    de_cn2bn(pmp, cn.wsub(T::from_u32(CLUST_FIRST))).wadd(T::from_u32(pmp.pm_firstcluster))
}

/// `roottobn(pmp, dirofs)`: calculate block number for directory entry in root dir, offset
/// `dirofs`.
pub fn roottobn<G: ProcGlue, T: DeArith>(pmp: &Msdosfsmount<G>, dirofs: T) -> T {
    de_blk(pmp, dirofs).wadd(T::from_u32(pmp.pm_rootdirblk))
}

/// `detobn(pmp, dirclu, dirofs)`: calculate block number for directory entry at cluster
/// `dirclu`, offset `dirofs`.
pub fn detobn<G: ProcGlue>(pmp: &Msdosfsmount<G>, dirclu: u32, dirofs: u32) -> u32 {
    if dirclu == MSDOSFSROOT {
        roottobn(pmp, dirofs)
    } else {
        cntobn(pmp, dirclu)
    }
}

/// `fsi_size(pmp)`: calculate size of fsinfo block (the module's deviations: the shift count
/// taken modulo 32, as both machines' instructions take it).
pub fn fsi_size<G: ProcGlue>(pmp: &Msdosfsmount<G>) -> i32 {
    fsi_size_of(pmp.pm_BlkPerSec)
}

/// The macro's value for a sector of `blkpersec` blocks (`pm_BlkPerSec`).
fn fsi_size_of(blkpersec: u32) -> i32 {
    1024i32.wrapping_shl(blkpersec >> 2)
}

/// [`fsi_size`] for the FSInfo read of `msdosfs_mountfs`, from the sector's `blkpersec`
/// blocks (`pm_BlkPerSec`: the structure is built after that read): `EINVAL` when it exceeds
/// `MAXBSIZE`, the largest buffer the cache maps (the module's deviations, the user's
/// decision D-N5.7-U3).
pub fn fsi_size_checked(blkpersec: u32) -> Result<i32, Errno> {
    let size = fsi_size_of(blkpersec);
    if size > MAXBSIZE as i32 {
        return Err(Errno::EINVAL);
    }
    Ok(size)
}

/// A mount structure of the test glue over testffs's device (`testffs::attach` first) and a
/// fresh mount of testfs's configuration: the FAT width `fatmask` (with its `pm_fatmult` and
/// `pm_fatdiv`), `maxcluster` clusters, the bitmap with every cluster free except 0, 1 and the
/// bits past `maxcluster`, the rest zero but for what `f` sets; published.
#[cfg(test)]
pub(crate) fn test_pmp(
    fatmask: u32,
    maxcluster: u32,
    f: impl FnOnce(&mut Msdosfsmount<crate::fs::testfs::TestGlue>),
) -> MsdosfsmountRef<crate::fs::testfs::TestGlue> {
    use crate::fs::msdosfs::fat::{FAT12_MASK, FAT16_MASK};
    use core::sync::atomic::Ordering;

    let (fatmult, fatdiv) = match fatmask {
        FAT12_MASK => (3, 2),
        FAT16_MASK => (2, 1),
        _ => (4, 1),
    };
    let Ok(map) = inusemap_alloc(maxcluster) else {
        std::panic!("test_pmp: no bitmap");
    };
    for cn in 0..map.len() as u32 * N_INUSEBITS {
        if cn < CLUST_FIRST || cn > maxcluster {
            let w = &map[(cn / N_INUSEBITS) as usize];
            w.store(
                w.load(Ordering::Relaxed) | 1 << (cn % N_INUSEBITS),
                Ordering::Relaxed,
            );
        }
    }
    let devvp = crate::fs::ufs::ffs::testffs::devvp();
    let mut pm = Msdosfsmount {
        pm_mountp: crate::fs::mount::vfs_mount_alloc(None, &crate::fs::testfs::TEST_CONF[0]),
        pm_dev: devvp.rdev(),
        pm_uid: U32Word::new(0),
        pm_gid: U32Word::new(0),
        pm_mask: U32Word::new(0),
        pm_devvp: devvp,
        pm_bpb: Bpb50::default(),
        pm_BlkPerSec: 0,
        pm_FATsecs: 0,
        pm_fatblk: 0,
        pm_rootdirblk: 0,
        pm_rootdirsize: 0,
        pm_firstcluster: 0,
        pm_nmbrofclusters: maxcluster.wrapping_sub(1),
        pm_maxcluster: maxcluster,
        pm_freeclustercount: U32Word::new(maxcluster.wrapping_sub(1)),
        pm_cnshift: 0,
        pm_crbomask: 0,
        pm_bnshift: 0,
        pm_bpcluster: 0,
        pm_fmod: U32Word::new(0),
        pm_fatblocksize: 0,
        pm_fatblocksec: 0,
        pm_fatsize: 0,
        pm_fatmask: fatmask,
        pm_fsinfo: U32Word::new(0),
        pm_fatmult: fatmult,
        pm_fatdiv: fatdiv,
        pm_curfat: 0,
        pm_inusemap: map,
        pm_flags: U32Word::new(0),
    };
    f(&mut pm);
    pm.alloc()
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::msdosfs::fat::{FAT16_MASK, FAT32_MASK};
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs;
    use core::sync::atomic::Ordering;

    /// A FAT16 geometry: 512-byte sectors, 4 sectors per cluster, root directory at block 9
    /// for 32 blocks, first cluster at block 41.
    fn fat16(pm: &mut Msdosfsmount<TestGlue>) {
        pm.pm_bpb.bpbBytesPerSec = 512;
        pm.pm_bnshift = 9;
        pm.pm_bpcluster = 2048;
        pm.pm_cnshift = 11;
        pm.pm_crbomask = 2047;
        pm.pm_rootdirblk = 9;
        pm.pm_rootdirsize = 32;
        pm.pm_firstcluster = 41;
    }

    #[test]
    fn conversions() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let pmp = test_pmp(FAT16_MASK, 100, fat16);
        let pmp = &*pmp;
        assert_eq!(pmp.pm_BytesPerSec(), 512);
        assert_eq!(de_cluster(pmp, 4095u32), 1);
        assert_eq!(de_cluster(pmp, 0x1_0000_0000i64), 0x20_0000);
        assert_eq!(de_clcount(pmp, 0u32), 0);
        assert_eq!(de_clcount(pmp, 1u32), 1);
        assert_eq!(de_clcount(pmp, 2049u32), 2);
        // The macro's addition wraps in the argument's type.
        assert_eq!(de_clcount(pmp, u32::MAX), 0);
        assert_eq!(de_cn2bn(pmp, 3u32), 12);
        assert_eq!(de_bn2cn(pmp, 12u32), 3);
        assert_eq!(de_blk(pmp, 5000u32), 8);
        assert_eq!(de_cn2off(pmp, 3u32), 6144);
        assert_eq!(de_bn2off(pmp, 3u32), 1536);
        assert_eq!(cntobn(pmp, 2u32), 41);
        assert_eq!(cntobn(pmp, 5i64), 53);
        // A cluster below CLUST_FIRST wraps as the C's unsigned subtraction.
        assert_eq!(cntobn(pmp, 0u32), 41u32.wrapping_sub(8));
        assert_eq!(roottobn(pmp, 64u32), 9);
        assert_eq!(detobn(pmp, MSDOSFSROOT, 2048), 13);
        assert_eq!(detobn(pmp, 3, 2048), 45);
        assert_eq!(bptoep(pmp, 2048 + 96), 96);
        assert_eq!(fatofs(pmp, 7), 14);
        assert_eq!(N_INUSEBITS, 32);
    }

    #[test]
    fn fsi_size_for_every_sector_size() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        // Sector size, the FSInfo read, the checked size (D-N5.7-U3: above MAXBSIZE refused).
        for (bps, size, checked) in [
            (512u32, 1024, Ok(1024)),
            (1024, 1024, Ok(1024)),
            (2048, 2048, Ok(2048)),
            (4096, 4096, Ok(4096)),
            (8192, 16384, Ok(16384)),
            (16384, 256 * 1024, Err(Errno::EINVAL)),
            (32768, 64 * 1024 * 1024, Err(Errno::EINVAL)),
            // Unreachable: bpbBytesPerSec is 16-bit. 1024 << 32 gives 1024, the shift count
            // modulo 32, as both machines' instructions.
            (65536, 1024, Ok(1024)),
        ] {
            let pmp = test_pmp(FAT32_MASK, 100, |pm| pm.pm_BlkPerSec = bps / 512);
            assert_eq!(fsi_size(&pmp), size, "{bps}");
            assert_eq!(fsi_size_checked(bps / 512), checked, "{bps}");
        }
    }

    #[test]
    fn built_whole_and_read_back_through_the_slot() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let pmp = test_pmp(FAT16_MASK, 1000, |pm| {
            fat16(pm);
            pm.pm_FATsecs = 4;
        });
        assert_eq!(MArc::ty(&pmp), M_MSDOSFSMNT);
        assert_eq!(MArray::ty(&pmp.pm_inusemap), M_MSDOSFSFAT);
        assert_eq!(pmp.pm_inusemap.len(), 1001usize.div_ceil(32));
        // Clusters 0 and 1 and the bits past 1000 are in use, the rest free.
        assert_eq!(pmp.pm_inusemap[0].load(Ordering::Relaxed), 0b11);
        assert_eq!(pmp.pm_inusemap[31].load(Ordering::Relaxed), !0u32 << 9);
        assert_eq!(pmp.pm_freeclustercount.get(), 999);
        // The words the C writes after the mount.
        pmp.pm_flags.set(MSDOSFSMNT_RONLY);
        pmp.pm_uid.set(1000);
        assert_eq!(
            (pmp.pm_flags.get(), pmp.pm_uid.get()),
            (MSDOSFSMNT_RONLY, 1000)
        );
        // Hung from the mount's slot (`mnt_data`), read back as the same structure.
        let mp = pmp.pm_mountp;
        mp.set_node(MArc::erase(pmp.clone()));
        let again = vfstomsdosfs(mp);
        assert!(MArc::ptr_eq(&again, &pmp));
        assert_eq!(again.pm_FATsecs, 4);
        drop(again);
        assert!(mp.take_node().is_some());
    }

    #[test]
    fn inusemap_alloc_refuses_what_the_c_refuses() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        // maxcluster + 1 wraps to 0: an empty bitmap.
        assert_eq!(inusemap_alloc(u32::MAX).err(), Some(Errno::EINVAL));
        assert_eq!(inusemap_alloc(0).map(|m| m.len()), Ok(1));
        assert_eq!(inusemap_alloc(31).map(|m| m.len()), Ok(1));
        assert_eq!(inusemap_alloc(32).map(|m| m.len()), Ok(2));
    }

    #[test]
    fn vfstomsdosfs_of_another_mount_panics() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let mp = crate::fs::mount::vfs_mount_alloc(None, &crate::fs::testfs::TEST_CONF[0]);
        // No lock is taken inside: the panic unwinds out of the slot's read alone.
        let r = catch_unwind(AssertUnwindSafe(|| drop(vfstomsdosfs::<TestGlue>(mp))));
        assert!(r.is_err());
    }
}
/* </TESTS> */
