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
 * Copyright (c) 1982, 1986, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)fs.h	8.10 (Berkeley) 10/27/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ffs/fs.h>`: the layout of a fast file system: the super-block (`struct fs`), the
//! cylinder group block (`struct cg`, and the old `struct ocg`), the cylinder group summaries
//! (`struct csum`), and the macros that turn inode numbers, logical blocks and fragments into
//! disk addresses; and the super-block in core ([`Superblock`]) and a cylinder group in a busy
//! buffer ([`CgView`]).
//!
//! Upstream: sys/ufs/ffs/fs.h @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/fs.rs@3c62ede68802
//!
//! Each disk drive contains some number of file systems. A file system consists of a number
//! of cylinder groups. Each cylinder group has inodes and data. A file system is described
//! by its super-block, which in turn describes the cylinder groups. The super-block is
//! critical data and is replicated in each cylinder group to protect against catastrophic
//! loss. This is done at `newfs` time and the critical super-block data does not change, so
//! the copies need not be referenced further unless disaster strikes.
//!
//! For file system fs, the offsets of the various blocks of interest are given in the super
//! block as: `[fs->fs_sblkno]` super-block, `[fs->fs_cblkno]` cylinder group block,
//! `[fs->fs_iblkno]` inode blocks, `[fs->fs_dblkno]` data blocks. The beginning of cylinder
//! group cg in fs, is given by the `cgbase(fs, cg)` macro.
//!
//! ## Deviations
//! - The in-core-only slots of `struct fs` hold 0 in the in-core copy where the C stores
//!   kernel addresses: `fs_csp` always, `fs_maxcluster` when the cluster summary is kept
//!   (`fs_contigsumsize > 0`), `fs_contigdirs` (the C's NULL on a read-only mount, its array's
//!   address otherwise). So the super-block `ffs_sbupdate` writes carries 0 in those slots
//!   (fsck ignores them); the arrays they pointed at are [`Superblock`]'s own. `fs_ocsp` and
//!   `fs_active` keep the bytes the disk had, as in the C. The pointer slots of the on-disk
//!   structure are `u64`: both machines are LP64, and their offsets and sizes are the C's
//!   (asserted below).
//! - A super-block whose `fs_sbsize` is smaller than `struct fs` (no `newfs` writes one) is
//!   held in `struct fs`'s size, as LZ held it; the C's copy would end inside the structure.
//! - The other macros are functions with the macros' names in lower case (`fsbtodb`,
//!   `cgtod`, `ino_to_fsba`, `blkoff`, `lblkno`, `fragroundup`, `blksize`, ...), and
//!   `NINDIR`/`INOPB`/`INOPF`/`NSPB`/`NSPF`/`CGSIZE`/`FS_KERNMAXFILESIZE` are `nindir`,
//!   `inopb`, `inopf`, `nspb`, `nspf`, `cgsize`, `fs_kernmaxfilesize`; `<sys/param.h>`'s
//!   `setbit`/`clrbit`/`isset`/`isclr` are here for the maps.
//! - `inside[]`, `around[]` and `fragtbl[]` are `ffs_tables.c`'s, re-exported here.
//!
//! ## Redesign
//! - LZ's `#[repr(C)]` `Fs` with every member a `Cell` (`unsafe impl Sync`), `malloc`ed and
//!   reached as `&'static Fs` through an `unsafe` cast (sys/ufs/ffs/ffs_vfsops.rs:897-911),
//!   its in-core pointers raw and dereferenced behind bounds checks (`Fs::fs_cs`,
//!   `contigdirs`, `set_maxcluster`: four `unsafe` blocks, fs.rs:400-445) -> two types (D-N5.5b-2):
//!   [`Fs`], the on-disk structure as a plain `hal::abi_pod!` (read out of a buffer by
//!   `pod_from_bytes`, laid out by `newfs`, the layout asserted against the C's offsets), and
//!   [`Superblock`], the in-core copy: the C's `fs_sbsize` bytes as relaxed atomic words
//!   (the dinode's idiom, `DinWords`, over a `malloc(M_UFSMNT)` array of the C's
//!   size: the same word core, `inode.rs`'s `words_get`/`words_set`), each member read and
//!   written at `offset_of!(Fs, member)` by the accessors [`fs_field!`] writes
//!   (`fs.fs_bsize()`, `fs.set_fs_fmod(1)`, `fs.add_fs_cstotal_nbfree(-1)`); the C's
//!   `malloc`ed arrays its pointers name are owned members: `space` (the cylinder group
//!   summaries and `fs_maxcluster`, the C's one `malloc`, so the `UFS mount` row's requests
//!   and bytes are the C's), and `fs_contigdirs` (read-mostly, replaced by a mount update).
//!   Cost against the C: a member read is a load and a shift of its word, a change a load
//!   and a store, no lock and no interrupt-level change (the C changes these members under
//!   the kernel lock, checked under `diagnostic`); `fs_cs(fs, cg)` a bounds check; a
//!   `fs_contigdirs` access a read section of its slot (two atomics), one per walk.
//! - LZ's `CgBuf` (the buffer's address and length, the header cast from it and the maps cut
//!   as raw slices: three `unsafe` sites and a `pub unsafe fn new`) -> [`CgView`], a view over
//!   the busy buffer's bytes (`BufRef::data`): the header's members read and written at
//!   `offset_of!(Cg, member)` by [`cg_field!`]'s accessors, the maps as tails of the bytes,
//!   each offset checked against the block (a panic where the C would read past it), the
//!   old layout's offsets (`struct ocg`) as LZ's. The C's pointer arithmetic is a bounds check
//!   per map access, over the same bytes.
//! - `cg_frsum` is the C's `u_int32_t` again (LZ's `int32_t` deviation): `ffs_fragacct`'s
//!   signed counts are added modulo 2^32, the C's arithmetic through its `int32_t *`.

use core::mem::offset_of;
use core::sync::atomic::{AtomicI32, AtomicU8, AtomicU64, Ordering};

use hal::malloc::{M_WAITOK, MArray};
use hal::slot::ReadMostly;
use hal::sys::types::{Daddr, Off};
use hal::user::pod_from_bytes;

use crate::assert::kpanic;
use crate::fs::bio::DEV_BSHIFT;
use crate::fs::ufs::ufs::dinode::{NDADDR, Ufsino};
use crate::fs::ufs::ufs::inode::{
    DinField, Inode, words_get, words_read_bytes, words_set, words_write_bytes,
};
use crate::fs::ufs::ufs::ufsmount::M_UFSMNT;
use crate::proc::ProcGlue;
use crate::select::howmany;

pub use super::ffs_tables::{AROUND, FRAGTBL, INSIDE};

/// `BBSIZE`: the boot block's size.
pub const BBSIZE: usize = 8192;
/// `SBSIZE`: the super-block's size.
pub const SBSIZE: usize = 8192;
/// `BBOFF`.
pub const BBOFF: Off = 0;
/// `SBOFF`.
pub const SBOFF: Off = BBOFF + BBSIZE as Off;
/// `BBLOCK`.
pub const BBLOCK: Daddr = 0;
/// `SBLOCK`.
pub const SBLOCK: Daddr = BBLOCK + (BBSIZE >> DEV_BSHIFT) as Daddr;
/// `SBLOCK_UFS1`: the byte offset of the FFS1 super-block.
pub const SBLOCK_UFS1: i32 = 8192;
/// `SBLOCK_UFS2`: the byte offset of the FFS2 super-block.
pub const SBLOCK_UFS2: i32 = 65536;
/// `SBLOCK_PIGGY`.
pub const SBLOCK_PIGGY: i32 = 262144;
/// `SBLOCKSIZE`.
pub const SBLOCKSIZE: usize = 8192;
/// `SBLOCKSEARCH`: where to look for a super-block, in order, `-1` terminated.
pub const SBLOCKSEARCH: [i32; 4] = [SBLOCK_UFS2, SBLOCK_UFS1, SBLOCK_PIGGY, -1];

/// `MAXFRAG`: blocks may be broken into at most this many fragments.
pub const MAXFRAG: usize = 8;

/// `MINBSIZE`: the smallest allowable block size. In order to insure that it is possible to
/// create files of size 2^32 with only two levels of indirection, MINBSIZE is set to 4096.
pub const MINBSIZE: usize = 4096;

/// `MAXMNTLEN`: the space in the super-block for the name it is mounted on.
pub const MAXMNTLEN: usize = 468;

/// `MAXVOLLEN`: the length of the volume name buffer.
pub const MAXVOLLEN: usize = 32;

/// `NOCSPTRS`: the padding left of the old in-core summary pointers (128 bytes less the
/// four pointers that remain).
pub const NOCSPTRS: usize = (128 / size_of::<*mut u8>()) - 4;

/// `FS_MAXCONTIG`: a maximum summary size of contiguous blocks.
pub const FS_MAXCONTIG: usize = 16;

/// `MINFREE`: the minimum acceptable percentage of file system blocks which may be free.
pub const MINFREE: i32 = 5;
/// `DEFAULTOPT`.
pub const DEFAULTOPT: i32 = FS_OPTTIME;

/// `AVFILESIZ`: expected average file size.
pub const AVFILESIZ: u32 = 16384;
/// `AFPDIR`: expected number of files per directory.
pub const AFPDIR: u32 = 64;

/// `FSMAXSNAP`: size of superblock space reserved for snapshots.
pub const FSMAXSNAP: usize = 20;

/// `struct csum`: per cylinder group information; summarized in blocks allocated from first
/// cylinder group data blocks.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Csum {
    /// `cs_ndir`: number of directories.
    pub cs_ndir: i32,
    /// `cs_nbfree`: number of free blocks.
    pub cs_nbfree: i32,
    /// `cs_nifree`: number of free inodes.
    pub cs_nifree: i32,
    /// `cs_nffree`: number of free frags.
    pub cs_nffree: i32,
}

hal::abi_pod!(Csum {
    cs_ndir: i32,
    cs_nbfree: i32,
    cs_nifree: i32,
    cs_nffree: i32,
});

/// `struct csum_total`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CsumTotal {
    /// `cs_ndir`: number of directories.
    pub cs_ndir: i64,
    /// `cs_nbfree`: number of free blocks.
    pub cs_nbfree: i64,
    /// `cs_nifree`: number of free inodes.
    pub cs_nifree: i64,
    /// `cs_nffree`: number of free frags.
    pub cs_nffree: i64,
    /// `cs_spare`: future expansion.
    pub cs_spare: [i64; 4],
}

hal::abi_pod!(CsumTotal {
    cs_ndir: i64,
    cs_nbfree: i64,
    cs_nifree: i64,
    cs_nffree: i64,
    cs_spare: [i64; 4],
});

/// `struct fs`: super block for an FFS file system, as the disk holds it. The kernel reads
/// it in core through [`Superblock`]; this is the layout its accessors name and what
/// `newfs` and the tests write.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fs {
    /// `fs_firstfield`: historic file system linked list, used for incore super blocks.
    pub fs_firstfield: i32,
    /// `fs_unused_1`.
    pub fs_unused_1: i32,
    /// `fs_sblkno`: addr of super-block / frags.
    pub fs_sblkno: i32,
    /// `fs_cblkno`: offset of cyl-block / frags.
    pub fs_cblkno: i32,
    /// `fs_iblkno`: offset of inode-blocks / frags.
    pub fs_iblkno: i32,
    /// `fs_dblkno`: offset of first data / frags.
    pub fs_dblkno: i32,
    /// `fs_cgoffset`: cylinder group offset in cylinder.
    pub fs_cgoffset: i32,
    /// `fs_cgmask`: used to calc mod fs_ntrak.
    pub fs_cgmask: i32,
    /// `fs_ffs1_time`: last time written.
    pub fs_ffs1_time: i32,
    /// `fs_ffs1_size`: # of blocks in fs / frags.
    pub fs_ffs1_size: i32,
    /// `fs_ffs1_dsize`: # of data blocks in fs.
    pub fs_ffs1_dsize: i32,
    /// `fs_ncg`: # of cylinder groups.
    pub fs_ncg: u32,
    /// `fs_bsize`: size of basic blocks / bytes.
    pub fs_bsize: i32,
    /// `fs_fsize`: size of frag blocks / bytes.
    pub fs_fsize: i32,
    /// `fs_frag`: # of frags in a block in fs.
    pub fs_frag: i32,
    /// `fs_minfree`: minimum percentage of free blocks.
    pub fs_minfree: i32,
    /// `fs_rotdelay`: # of ms for optimal next block.
    pub fs_rotdelay: i32,
    /// `fs_rps`: disk revolutions per second.
    pub fs_rps: i32,
    /// `fs_bmask`: `blkoff` calc of blk offsets.
    pub fs_bmask: i32,
    /// `fs_fmask`: `fragoff` calc of frag offsets.
    pub fs_fmask: i32,
    /// `fs_bshift`: `lblkno` calc of logical blkno.
    pub fs_bshift: i32,
    /// `fs_fshift`: `numfrags` calc # of frags.
    pub fs_fshift: i32,
    /// `fs_maxcontig`: max # of contiguous blks.
    pub fs_maxcontig: i32,
    /// `fs_maxbpg`: max # of blks per cyl group.
    pub fs_maxbpg: i32,
    /// `fs_fragshift`: block to frag shift.
    pub fs_fragshift: i32,
    /// `fs_fsbtodb`: fsbtodb and dbtofsb shift constant.
    pub fs_fsbtodb: i32,
    /// `fs_sbsize`: actual size of super block.
    pub fs_sbsize: i32,
    /// `fs_csmask`: csum block offset (now unused).
    pub fs_csmask: i32,
    /// `fs_csshift`: csum block number (now unused).
    pub fs_csshift: i32,
    /// `fs_nindir`: value of NINDIR.
    pub fs_nindir: i32,
    /// `fs_inopb`: inodes per file system block.
    pub fs_inopb: u32,
    /// `fs_nspf`: DEV_BSIZE sectors per frag.
    pub fs_nspf: i32,
    /// `fs_optim`: optimization preference, see below.
    pub fs_optim: i32,
    /// `fs_npsect`: DEV_BSIZE sectors/track + spares.
    pub fs_npsect: i32,
    /// `fs_interleave`: DEV_BSIZE sector interleave.
    pub fs_interleave: i32,
    /// `fs_trackskew`: sector 0 skew, per track.
    pub fs_trackskew: i32,
    /// `fs_id`: unique filesystem id (the space of the unused `fs_headswitch`, `fs_trkseek`).
    pub fs_id: [i32; 2],
    /// `fs_ffs1_csaddr`: blk addr of cyl grp summary area.
    pub fs_ffs1_csaddr: i32,
    /// `fs_cssize`: cyl grp summary area size / bytes.
    pub fs_cssize: i32,
    /// `fs_cgsize`: cyl grp block size / bytes.
    pub fs_cgsize: i32,
    /// `fs_ntrak`: tracks per cylinder.
    pub fs_ntrak: i32,
    /// `fs_nsect`: DEV_BSIZE sectors per track.
    pub fs_nsect: i32,
    /// `fs_spc`: DEV_BSIZE sectors per cylinder.
    pub fs_spc: i32,
    /// `fs_ncyl`: cylinders in file system.
    pub fs_ncyl: i32,
    /// `fs_cpg`: cylinders per group.
    pub fs_cpg: i32,
    /// `fs_ipg`: inodes per group.
    pub fs_ipg: u32,
    /// `fs_fpg`: blocks per group * fs_frag.
    pub fs_fpg: i32,
    /// `fs_ffs1_cstotal`: cylinder summary information.
    pub fs_ffs1_cstotal: Csum,
    /// `fs_fmod`: super block modified flag.
    pub fs_fmod: i8,
    /// `fs_clean`: file system is clean flag.
    pub fs_clean: i8,
    /// `fs_ronly`: mounted read-only flag.
    pub fs_ronly: i8,
    /// `fs_ffs1_flags`: see `FS_` below.
    pub fs_ffs1_flags: i8,
    /// `fs_fsmnt`: name mounted on.
    pub fs_fsmnt: [u8; MAXMNTLEN],
    /// `fs_volname`: volume name.
    pub fs_volname: [u8; MAXVOLLEN],
    /// `fs_swuid`: system-wide uid.
    pub fs_swuid: u64,
    /// `fs_pad`: due to alignment of fs_swuid.
    pub fs_pad: i32,
    /// `fs_cgrotor`: last cg searched.
    pub fs_cgrotor: i32,
    /// `fs_ocsp`: padding; was list of fs_cs buffers (`void *`).
    pub fs_ocsp: [u64; NOCSPTRS],
    /// `fs_contigdirs`: # of contiguously allocated dirs (an in-core `u_int8_t *`).
    pub fs_contigdirs: u64,
    /// `fs_csp`: cg summary info buffer for fs_cs (an in-core `struct csum *`).
    pub fs_csp: u64,
    /// `fs_maxcluster`: max cluster in each cyl group (an in-core `int32_t *`).
    pub fs_maxcluster: u64,
    /// `fs_active`: reserved for snapshots (`u_char *`).
    pub fs_active: u64,
    /// `fs_cpc`: cyl per cycle in postbl.
    pub fs_cpc: i32,
    /// `fs_maxbsize`: maximum blocking factor permitted.
    pub fs_maxbsize: i32,
    /// `fs_spareconf64`: old rotation block list head.
    pub fs_spareconf64: [i64; 17],
    /// `fs_sblockloc`: offset of standard super block.
    pub fs_sblockloc: i64,
    /// `fs_cstotal`: cylinder summary information.
    pub fs_cstotal: CsumTotal,
    /// `fs_time`: time last written.
    pub fs_time: i64,
    /// `fs_size`: number of blocks in fs.
    pub fs_size: i64,
    /// `fs_dsize`: number of data blocks in fs.
    pub fs_dsize: i64,
    /// `fs_csaddr`: blk addr of cyl grp summary area.
    pub fs_csaddr: i64,
    /// `fs_pendingblocks`: blocks in process of being freed.
    pub fs_pendingblocks: i64,
    /// `fs_pendinginodes`: inodes in process of being freed.
    pub fs_pendinginodes: u32,
    /// `fs_snapinum`: space reserved for snapshots.
    pub fs_snapinum: [u32; FSMAXSNAP],
    /// `fs_avgfilesize`: expected average file size.
    pub fs_avgfilesize: u32,
    /// `fs_avgfpdir`: expected # of files per directory.
    pub fs_avgfpdir: u32,
    /// `fs_sparecon`: reserved for future constants.
    pub fs_sparecon: [i32; 26],
    /// `fs_flags`: see `FS_` flags below.
    pub fs_flags: u32,
    /// `fs_fscktime`: last time fsck(8)ed.
    pub fs_fscktime: i32,
    /// `fs_contigsumsize`: size of cluster summary array.
    pub fs_contigsumsize: i32,
    /// `fs_maxsymlinklen`: max length of an internal symlink.
    pub fs_maxsymlinklen: i32,
    /// `fs_inodefmt`: format of on-disk inodes.
    pub fs_inodefmt: i32,
    /// `fs_maxfilesize`: maximum representable file size.
    pub fs_maxfilesize: u64,
    /// `fs_qbmask`: `~fs_bmask` - for use with quad size.
    pub fs_qbmask: i64,
    /// `fs_qfmask`: `~fs_fmask` - for use with quad size.
    pub fs_qfmask: i64,
    /// `fs_state`: validate fs_clean field.
    pub fs_state: i32,
    /// `fs_postblformat`: format of positional layout tables.
    pub fs_postblformat: i32,
    /// `fs_nrpos`: number of rotational positions.
    pub fs_nrpos: i32,
    /// `fs_postbloff`: (u_int16) rotation block list head.
    pub fs_postbloff: i32,
    /// `fs_rotbloff`: (u_int8) blocks for each rotation.
    pub fs_rotbloff: i32,
    /// `fs_magic`: magic number.
    pub fs_magic: i32,
    /// `fs_space[1]`: list of blocks for each rotation (actually longer), and the seven bytes
    /// of padding the C's `sizeof(struct fs)` ends with.
    pub fs_space: [u8; 8],
}

hal::abi_pod!(Fs {
    fs_firstfield: i32,
    fs_unused_1: i32,
    fs_sblkno: i32,
    fs_cblkno: i32,
    fs_iblkno: i32,
    fs_dblkno: i32,
    fs_cgoffset: i32,
    fs_cgmask: i32,
    fs_ffs1_time: i32,
    fs_ffs1_size: i32,
    fs_ffs1_dsize: i32,
    fs_ncg: u32,
    fs_bsize: i32,
    fs_fsize: i32,
    fs_frag: i32,
    fs_minfree: i32,
    fs_rotdelay: i32,
    fs_rps: i32,
    fs_bmask: i32,
    fs_fmask: i32,
    fs_bshift: i32,
    fs_fshift: i32,
    fs_maxcontig: i32,
    fs_maxbpg: i32,
    fs_fragshift: i32,
    fs_fsbtodb: i32,
    fs_sbsize: i32,
    fs_csmask: i32,
    fs_csshift: i32,
    fs_nindir: i32,
    fs_inopb: u32,
    fs_nspf: i32,
    fs_optim: i32,
    fs_npsect: i32,
    fs_interleave: i32,
    fs_trackskew: i32,
    fs_id: [i32; 2],
    fs_ffs1_csaddr: i32,
    fs_cssize: i32,
    fs_cgsize: i32,
    fs_ntrak: i32,
    fs_nsect: i32,
    fs_spc: i32,
    fs_ncyl: i32,
    fs_cpg: i32,
    fs_ipg: u32,
    fs_fpg: i32,
    fs_ffs1_cstotal: Csum,
    fs_fmod: i8,
    fs_clean: i8,
    fs_ronly: i8,
    fs_ffs1_flags: i8,
    fs_fsmnt: [u8; MAXMNTLEN],
    fs_volname: [u8; MAXVOLLEN],
    fs_swuid: u64,
    fs_pad: i32,
    fs_cgrotor: i32,
    fs_ocsp: [u64; NOCSPTRS],
    fs_contigdirs: u64,
    fs_csp: u64,
    fs_maxcluster: u64,
    fs_active: u64,
    fs_cpc: i32,
    fs_maxbsize: i32,
    fs_spareconf64: [i64; 17],
    fs_sblockloc: i64,
    fs_cstotal: CsumTotal,
    fs_time: i64,
    fs_size: i64,
    fs_dsize: i64,
    fs_csaddr: i64,
    fs_pendingblocks: i64,
    fs_pendinginodes: u32,
    fs_snapinum: [u32; FSMAXSNAP],
    fs_avgfilesize: u32,
    fs_avgfpdir: u32,
    fs_sparecon: [i32; 26],
    fs_flags: u32,
    fs_fscktime: i32,
    fs_contigsumsize: i32,
    fs_maxsymlinklen: i32,
    fs_inodefmt: i32,
    fs_maxfilesize: u64,
    fs_qbmask: i64,
    fs_qfmask: i64,
    fs_state: i32,
    fs_postblformat: i32,
    fs_nrpos: i32,
    fs_postbloff: i32,
    fs_rotbloff: i32,
    fs_magic: i32,
    fs_space: [u8; 8],
});

impl Fs {
    /// A super-block of zeros (`newfs`'s starting point, a test's).
    pub fn zeroed() -> Self {
        match pod_from_bytes(&[0u8; size_of::<Fs>()]) {
            Some(fs) => fs,
            None => kpanic(format_args!("Fs::zeroed")),
        }
    }
}

impl Default for Fs {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// `fs->fs_fsmnt` as `%s` prints it: the bytes up to the first NUL, or all 468 of them
/// when the disk's copy has none (the C's `%s` would read on past the member).
pub struct Fsmnt([u8; MAXMNTLEN]);

impl Fsmnt {
    /// The name's bytes, up to the first NUL.
    pub fn as_bytes(&self) -> &[u8] {
        let n = self.0.iter().position(|&b| b == 0).unwrap_or(MAXMNTLEN);
        &self.0[..n]
    }
}

impl core::fmt::Display for Fsmnt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &b in self.as_bytes() {
            core::fmt::Write::write_char(f, char::from(b))?;
        }
        Ok(())
    }
}

/// `struct fs *` in core (`ump->um_fs`): the super-block's words and the arrays the C's
/// in-core pointers name, owned. Set once by `ffs_mountfs` into its mount's UFS structure
/// (`Ufsmount::set_um_fs`), freed with it.
pub struct Superblock {
    /// `*fs`: the disk's `fs_sbsize` bytes (at least `struct fs`) as relaxed atomic words in
    /// one `malloc(M_UFSMNT)` array, read and written as `DinWords` reads and writes the
    /// dinode (`inode.rs`'s word core): a member at its `offset_of!`, a change a load and a
    /// store by a holder of the kernel lock (checked under `diagnostic`).
    words: MArray<AtomicU64>,
    /// `fs_csp` and `fs_maxcluster`: the C's one `space` of `malloc(M_UFSMNT)`, the cylinder
    /// group summaries (`fs_cssize` bytes, whole fragments) and, when `fs_contigsumsize > 0`,
    /// one `int32_t` per cylinder group.
    space: MArray<AtomicI32>,
    /// The words of `space` the summaries take (`fs_maxcluster` starts there).
    ncsum: usize,
    /// The `fs_maxcluster` words after them: `fs_ncg`, or 0 without a cluster summary.
    nmaxcluster: usize,
    /// `fs_contigdirs`: `fs_ncg` counters of `malloc(M_UFSMNT)` on a read-write mount, none
    /// on a read-only one; replaced by a mount update between the two.
    contigdirs: ReadMostly<Option<MArray<AtomicU8>>>,
}

impl Superblock {
    /// `fs = malloc(fs_sbsize, M_UFSMNT, M_WAITOK); memcpy(fs, sb, fs_sbsize)` and `space =
    /// malloc(size, M_UFSMNT, M_WAITOK)` of `ffs_mountfs`: the in-core copy of `sb` (the
    /// disk's `fs_sbsize` bytes, which `ffs_validate` accepted) and its summary space,
    /// zeroed (the caller reads the summaries in, [`Superblock::write_csum_bytes`]), with
    /// `fs_maxcluster`'s counters at `fs_contigsumsize` as the C sets them. No
    /// `fs_contigdirs` yet ([`Superblock::contigdirs_alloc`]). `panic(9)` for a super-block
    /// with no fragment size (the C divides by it).
    pub fn new(sb: &[u8]) -> Self {
        // In words, at least struct fs (the module's deviations).
        let n = sb.len().max(size_of::<Fs>()).div_ceil(8);
        let Some(words) = MArray::new(M_UFSMNT, M_WAITOK, n, |_| AtomicU64::new(0)) else {
            kpanic(format_args!(
                "ffs: malloc(M_UFSMNT) of the super-block failed"
            ));
        };
        words_write_bytes(&words, "fs", 0, sb);
        let get = |off: usize| words_get::<i32>(&words, "fs", off);
        let cssize = get(offset_of!(Fs, fs_cssize)).max(0) as usize;
        let fsize = get(offset_of!(Fs, fs_fsize));
        let ncg = words_get::<u32>(&words, "fs", offset_of!(Fs, fs_ncg)) as usize;
        let contigsumsize = get(offset_of!(Fs, fs_contigsumsize));
        if fsize <= 0 {
            kpanic(format_args!("ffs: fs_fsize {fsize}"));
        }
        // The summaries are read and written in whole fragments.
        let csbytes = howmany(cssize, fsize as usize) * fsize as usize;
        let ncsum = csbytes.div_ceil(4);
        let nmaxcluster = if contigsumsize > 0 { ncg } else { 0 };
        let init = |i: usize| AtomicI32::new(if i < ncsum { 0 } else { contigsumsize });
        let Some(space) = MArray::new(M_UFSMNT, M_WAITOK, ncsum + nmaxcluster, init) else {
            kpanic(format_args!(
                "ffs: malloc(M_UFSMNT) of the summaries failed"
            ));
        };
        let fs = Self {
            words,
            space,
            ncsum,
            nmaxcluster,
            contigdirs: ReadMostly::new(None),
        };
        // fs->fs_csp = space; fs->fs_maxcluster = lp: in-core addresses, 0 here.
        fs.set_fs_csp(0);
        if nmaxcluster > 0 {
            fs.set_fs_maxcluster(0);
        }
        fs
    }

    /// The member at byte `off` of the copy.
    #[inline]
    fn get<T: DinField>(&self, off: usize) -> T {
        words_get(&self.words, "fs", off)
    }

    /// The member at byte `off` of the copy set to `v`.
    #[inline]
    fn set<T: DinField>(&self, off: usize, v: T) {
        words_set(&self.words, "fs", off, v);
    }

    /// The copy's bytes `off..off + out.len()` into `out` (`ffs_sbupdate`'s copy).
    pub fn read_bytes(&self, off: usize, out: &mut [u8]) {
        words_read_bytes(&self.words, "fs", off, out);
    }

    /// The copy's bytes `off..off + src.len()` set to `src` (`ffs_reload`'s copy).
    pub fn write_bytes(&self, off: usize, src: &[u8]) {
        words_write_bytes(&self.words, "fs", off, src);
    }

    /// The copy's size in bytes: `fs_sbsize` rounded up to a word, at least `struct fs`
    /// (`ffs_sbupdate` writes `fs_sbsize` of them).
    pub fn copy_len(&self) -> usize {
        self.words.len() * 8
    }

    /// `fs->fs_id`.
    pub fn fs_id(&self) -> [i32; 2] {
        let off = offset_of!(Fs, fs_id);
        [self.get(off), self.get(off + 4)]
    }

    /// `fs->fs_fsmnt`.
    pub fn fsmnt(&self) -> Fsmnt {
        let mut name = [0u8; MAXMNTLEN];
        self.read_bytes(offset_of!(Fs, fs_fsmnt), &mut name);
        Fsmnt(name)
    }

    /// `memset(fs->fs_fsmnt, 0, sizeof(fs->fs_fsmnt)); strlcpy(fs->fs_fsmnt, name, ..)`: the
    /// name up to its NUL, cut to `MAXMNTLEN - 1` bytes, the rest zero.
    pub fn set_fsmnt(&self, name: &[u8]) {
        let n = name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name.len())
            .min(MAXMNTLEN - 1);
        let mut buf = [0u8; MAXMNTLEN];
        buf[..n].copy_from_slice(&name[..n]);
        self.write_bytes(offset_of!(Fs, fs_fsmnt), &buf);
    }

    /// `fs->fs_cs(fs, indx)`: the summary of cylinder group `indx`; `panic(9)` past the
    /// summary area, where the C would read past `fs_csp`'s array.
    #[inline]
    pub fn fs_cs(&self, indx: u32) -> CsumWords<'_> {
        let i = indx as usize * 4;
        match self.space[..self.ncsum]
            .get(i..i + 4)
            .and_then(|w| <&[AtomicI32; 4]>::try_from(w).ok())
        {
            Some(w) => CsumWords(w),
            None => kpanic(format_args!("fs_cs: cg {indx} of {}", self.fs_ncg())),
        }
    }

    /// The summary area's bytes `off..off + out.len()` into `out` (`ffs_sbupdate`'s writes);
    /// `panic(9)` for a range outside it or not of whole `int32_t`s.
    pub fn read_csum_bytes(&self, off: usize, out: &mut [u8]) {
        let words = self.csum_range(off, out.len());
        for (dst, w) in out.as_chunks_mut::<4>().0.iter_mut().zip(words) {
            *dst = w.load(Ordering::Relaxed).to_ne_bytes();
        }
    }

    /// The summary area's bytes `off..off + src.len()` set to `src` (`ffs_mountfs`'s and
    /// `ffs_reload`'s reads), by a holder of the kernel lock.
    pub fn write_csum_bytes(&self, off: usize, src: &[u8]) {
        #[cfg(feature = "diagnostic")]
        hal::klock::kernel_assert_locked();
        let words = self.csum_range(off, src.len());
        for (part, w) in src.as_chunks::<4>().0.iter().zip(words) {
            w.store(i32::from_ne_bytes(*part), Ordering::Relaxed);
        }
    }

    /// The words of the summary area's bytes `off..off + len`.
    fn csum_range(&self, off: usize, len: usize) -> &[AtomicI32] {
        if !off.is_multiple_of(4) || !len.is_multiple_of(4) || off + len > self.ncsum * 4 {
            kpanic(format_args!(
                "fs_csp: bytes {off}+{len} of {}",
                self.ncsum * 4
            ));
        }
        &self.space[off / 4..(off + len) / 4]
    }

    /// The summary area's size in bytes (`fs_cssize` in whole fragments).
    pub fn csum_len(&self) -> usize {
        self.ncsum * 4
    }

    /// `fs->fs_maxcluster[cg]`; `panic(9)` without the array or past it.
    pub fn maxcluster(&self, cg: u32) -> i32 {
        self.maxcluster_word(cg).load(Ordering::Relaxed)
    }

    /// `fs->fs_maxcluster[cg] = v`, by a holder of the kernel lock.
    pub fn set_maxcluster(&self, cg: u32, v: i32) {
        #[cfg(feature = "diagnostic")]
        hal::klock::kernel_assert_locked();
        self.maxcluster_word(cg).store(v, Ordering::Relaxed);
    }

    /// `&fs->fs_maxcluster[cg]`.
    fn maxcluster_word(&self, cg: u32) -> &AtomicI32 {
        let cg = cg as usize;
        if cg >= self.nmaxcluster {
            kpanic(format_args!(
                "fs_maxcluster: cg {cg} of {}",
                self.nmaxcluster
            ));
        }
        &self.space[self.ncsum + cg]
    }

    /// `f` over `fs->fs_contigdirs` (one read section of the slot for a whole walk; `f` must
    /// not sleep); `panic(9)` on a read-only mount, which has none (the C's NULL).
    pub fn with_contigdirs<R>(&self, f: impl FnOnce(&[AtomicU8]) -> R) -> R {
        let r = self.contigdirs.read();
        match &*r {
            Some(a) => f(a),
            None => kpanic(format_args!("fs_contigdirs: a read-only mount")),
        }
    }

    /// `fs->fs_contigdirs = malloc(fs->fs_ncg, M_UFSMNT, M_WAITOK|M_ZERO)` (a read-write
    /// mount, or an update to one); the slot in the copy is 0 (the module's deviations).
    pub fn contigdirs_alloc(&self) {
        let n = self.fs_ncg() as usize;
        let Some(a) = MArray::new(M_UFSMNT, M_WAITOK, n, |_| AtomicU8::new(0)) else {
            kpanic(format_args!(
                "ffs: malloc(M_UFSMNT) of fs_contigdirs failed"
            ));
        };
        drop(self.contigdirs.replace(Some(a)));
        self.set_fs_contigdirs(0);
    }

    /// `free(fs->fs_contigdirs, M_UFSMNT, fs->fs_ncg); fs->fs_contigdirs = NULL` (an update
    /// to read-only).
    pub fn contigdirs_free(&self) {
        drop(self.contigdirs.replace(None));
        self.set_fs_contigdirs(0);
    }
}

/// Generates the accessors of the in-core super-block's members: `$get` reads `fs->member`
/// at `offset_of!(Fs, member)` (a load and a shift of its word), `$set` writes it (a load
/// and a store, by a holder of the kernel lock), `$add` (counters) adds to it, as the C's
/// `fs->member += d` under that lock.
macro_rules! fs_field {
    ($($get:ident, $set:ident $(, $add:ident)?: $t:ty = $($f:ident).+;)+) => {
        impl Superblock {
            $(
                #[doc = concat!("`fs->", stringify!($($f).+), "`.")]
                #[inline]
                pub fn $get(&self) -> $t {
                    self.get::<$t>(offset_of!(Fs, $($f).+))
                }

                #[doc = concat!("`fs->", stringify!($($f).+), " = v`.")]
                #[inline]
                pub fn $set(&self, v: $t) {
                    self.set::<$t>(offset_of!(Fs, $($f).+), v);
                }

                $(
                    #[doc = concat!("`", stringify!($get), "() += d`.")]
                    #[inline]
                    pub fn $add(&self, d: $t) {
                        self.$set(self.$get().wrapping_add(d));
                    }
                )?
            )+
        }
    };
}

fs_field! {
    fs_sblkno, set_fs_sblkno: i32 = fs_sblkno;
    fs_cblkno, set_fs_cblkno: i32 = fs_cblkno;
    fs_iblkno, set_fs_iblkno: i32 = fs_iblkno;
    fs_dblkno, set_fs_dblkno: i32 = fs_dblkno;
    fs_cgoffset, set_fs_cgoffset: i32 = fs_cgoffset;
    fs_cgmask, set_fs_cgmask: i32 = fs_cgmask;
    fs_ffs1_time, set_fs_ffs1_time: i32 = fs_ffs1_time;
    fs_ffs1_size, set_fs_ffs1_size: i32 = fs_ffs1_size;
    fs_ffs1_dsize, set_fs_ffs1_dsize: i32 = fs_ffs1_dsize;
    fs_ncg, set_fs_ncg: u32 = fs_ncg;
    fs_bsize, set_fs_bsize: i32 = fs_bsize;
    fs_fsize, set_fs_fsize: i32 = fs_fsize;
    fs_frag, set_fs_frag: i32 = fs_frag;
    fs_minfree, set_fs_minfree: i32 = fs_minfree;
    fs_rotdelay, set_fs_rotdelay: i32 = fs_rotdelay;
    fs_rps, set_fs_rps: i32 = fs_rps;
    fs_bmask, set_fs_bmask: i32 = fs_bmask;
    fs_fmask, set_fs_fmask: i32 = fs_fmask;
    fs_bshift, set_fs_bshift: i32 = fs_bshift;
    fs_fshift, set_fs_fshift: i32 = fs_fshift;
    fs_maxcontig, set_fs_maxcontig: i32 = fs_maxcontig;
    fs_maxbpg, set_fs_maxbpg: i32 = fs_maxbpg;
    fs_fragshift, set_fs_fragshift: i32 = fs_fragshift;
    fs_fsbtodb, set_fs_fsbtodb: i32 = fs_fsbtodb;
    fs_sbsize, set_fs_sbsize: i32 = fs_sbsize;
    fs_nindir, set_fs_nindir: i32 = fs_nindir;
    fs_inopb, set_fs_inopb: u32 = fs_inopb;
    fs_nspf, set_fs_nspf: i32 = fs_nspf;
    fs_optim, set_fs_optim: i32 = fs_optim;
    fs_npsect, set_fs_npsect: i32 = fs_npsect;
    fs_interleave, set_fs_interleave: i32 = fs_interleave;
    fs_trackskew, set_fs_trackskew: i32 = fs_trackskew;
    fs_ffs1_csaddr, set_fs_ffs1_csaddr: i32 = fs_ffs1_csaddr;
    fs_cssize, set_fs_cssize: i32 = fs_cssize;
    fs_cgsize, set_fs_cgsize: i32 = fs_cgsize;
    fs_ntrak, set_fs_ntrak: i32 = fs_ntrak;
    fs_nsect, set_fs_nsect: i32 = fs_nsect;
    fs_spc, set_fs_spc: i32 = fs_spc;
    fs_ncyl, set_fs_ncyl: i32 = fs_ncyl;
    fs_cpg, set_fs_cpg: i32 = fs_cpg;
    fs_ipg, set_fs_ipg: u32 = fs_ipg;
    fs_fpg, set_fs_fpg: i32 = fs_fpg;
    fs_ffs1_cstotal_ndir, set_fs_ffs1_cstotal_ndir: i32 = fs_ffs1_cstotal.cs_ndir;
    fs_ffs1_cstotal_nbfree, set_fs_ffs1_cstotal_nbfree: i32 = fs_ffs1_cstotal.cs_nbfree;
    fs_ffs1_cstotal_nifree, set_fs_ffs1_cstotal_nifree: i32 = fs_ffs1_cstotal.cs_nifree;
    fs_ffs1_cstotal_nffree, set_fs_ffs1_cstotal_nffree: i32 = fs_ffs1_cstotal.cs_nffree;
    fs_fmod, set_fs_fmod: i8 = fs_fmod;
    fs_clean, set_fs_clean: i8 = fs_clean;
    fs_ronly, set_fs_ronly: i8 = fs_ronly;
    fs_ffs1_flags, set_fs_ffs1_flags: i8 = fs_ffs1_flags;
    fs_cgrotor, set_fs_cgrotor: i32 = fs_cgrotor;
    fs_contigdirs_slot, set_fs_contigdirs: u64 = fs_contigdirs;
    fs_csp_slot, set_fs_csp: u64 = fs_csp;
    fs_maxcluster_slot, set_fs_maxcluster: u64 = fs_maxcluster;
    fs_cpc, set_fs_cpc: i32 = fs_cpc;
    fs_maxbsize, set_fs_maxbsize: i32 = fs_maxbsize;
    fs_sblockloc, set_fs_sblockloc: i64 = fs_sblockloc;
    fs_cstotal_ndir, set_fs_cstotal_ndir, add_fs_cstotal_ndir: i64 = fs_cstotal.cs_ndir;
    fs_cstotal_nbfree, set_fs_cstotal_nbfree, add_fs_cstotal_nbfree: i64 = fs_cstotal.cs_nbfree;
    fs_cstotal_nifree, set_fs_cstotal_nifree, add_fs_cstotal_nifree: i64 = fs_cstotal.cs_nifree;
    fs_cstotal_nffree, set_fs_cstotal_nffree, add_fs_cstotal_nffree: i64 = fs_cstotal.cs_nffree;
    fs_time, set_fs_time: i64 = fs_time;
    fs_size, set_fs_size: i64 = fs_size;
    fs_dsize, set_fs_dsize: i64 = fs_dsize;
    fs_csaddr, set_fs_csaddr: i64 = fs_csaddr;
    fs_avgfilesize, set_fs_avgfilesize: u32 = fs_avgfilesize;
    fs_avgfpdir, set_fs_avgfpdir: u32 = fs_avgfpdir;
    fs_flags, set_fs_flags: u32 = fs_flags;
    fs_contigsumsize, set_fs_contigsumsize: i32 = fs_contigsumsize;
    fs_maxsymlinklen, set_fs_maxsymlinklen: i32 = fs_maxsymlinklen;
    fs_inodefmt, set_fs_inodefmt: i32 = fs_inodefmt;
    fs_maxfilesize, set_fs_maxfilesize: u64 = fs_maxfilesize;
    fs_qbmask, set_fs_qbmask: i64 = fs_qbmask;
    fs_qfmask, set_fs_qfmask: i64 = fs_qfmask;
    fs_state, set_fs_state: i32 = fs_state;
    fs_postblformat, set_fs_postblformat: i32 = fs_postblformat;
    fs_nrpos, set_fs_nrpos: i32 = fs_nrpos;
    fs_postbloff, set_fs_postbloff: i32 = fs_postbloff;
    fs_rotbloff, set_fs_rotbloff: i32 = fs_rotbloff;
    fs_magic, set_fs_magic: i32 = fs_magic;
}

/// `fs->fs_cs(fs, cg)` in core: the four counters of a cylinder group's `struct csum`, words
/// of the super-block's summary space (relaxed, changed by holders of the kernel lock, as
/// the C's).
#[derive(Clone, Copy)]
pub struct CsumWords<'a>(&'a [AtomicI32; 4]);

impl CsumWords<'_> {
    /// Counter `i` (`cs_ndir`, `cs_nbfree`, `cs_nifree`, `cs_nffree`).
    #[inline]
    fn get(self, i: usize) -> i32 {
        self.0[i].load(Ordering::Relaxed)
    }

    /// Counter `i` `+= d`.
    #[inline]
    fn add(self, i: usize, d: i32) {
        #[cfg(feature = "diagnostic")]
        hal::klock::kernel_assert_locked();
        let w = &self.0[i];
        w.store(w.load(Ordering::Relaxed).wrapping_add(d), Ordering::Relaxed);
    }

    /// `cs_ndir`.
    pub fn cs_ndir(self) -> i32 {
        self.get(0)
    }

    /// `cs_nbfree`.
    pub fn cs_nbfree(self) -> i32 {
        self.get(1)
    }

    /// `cs_nifree`.
    pub fn cs_nifree(self) -> i32 {
        self.get(2)
    }

    /// `cs_nffree`.
    pub fn cs_nffree(self) -> i32 {
        self.get(3)
    }

    /// `cs_ndir += d`.
    pub fn add_cs_ndir(self, d: i32) {
        self.add(0, d);
    }

    /// `cs_nbfree += d`.
    pub fn add_cs_nbfree(self, d: i32) {
        self.add(1, d);
    }

    /// `cs_nifree += d`.
    pub fn add_cs_nifree(self, d: i32) {
        self.add(2, d);
    }

    /// `cs_nffree += d`.
    pub fn add_cs_nffree(self, d: i32) {
        self.add(3, d);
    }
}

/// `FS_MAGIC`: the fast filesystem magic number.
pub const FS_MAGIC: i32 = 0x011954;
/// `FS_UFS1_MAGIC`: the fast filesystem magic number.
pub const FS_UFS1_MAGIC: i32 = 0x011954;
/// `FS_UFS2_MAGIC`: UFS fast filesystem magic number.
pub const FS_UFS2_MAGIC: i32 = 0x19540119;
/// `FS_OKAY`: superblock checksum.
pub const FS_OKAY: i32 = 0x7c269d38;
/// `FS_42INODEFMT`: 4.2BSD inode format.
pub const FS_42INODEFMT: i32 = -1;
/// `FS_44INODEFMT`: 4.4BSD inode format.
pub const FS_44INODEFMT: i32 = 2;

/// `FS_ISCLEAN`.
pub const FS_ISCLEAN: i8 = 0x01;
/// `FS_WASCLEAN`.
pub const FS_WASCLEAN: i8 = 0x02;

/// `FS_OPTTIME`: minimize allocation time.
pub const FS_OPTTIME: i32 = 0;
/// `FS_OPTSPACE`: minimize disk fragmentation.
pub const FS_OPTSPACE: i32 = 1;

/// `FS_UNCLEAN`: filesystem not clean at mount.
pub const FS_UNCLEAN: u32 = 0x01;
/// `FS_FLAGS_UPDATED`: an FFS1 file system that had its flags moved to the new (FFS2)
/// location for compatibility.
pub const FS_FLAGS_UPDATED: u32 = 0x80;

/// `FS_42POSTBLFMT`: 4.2BSD rotational table format.
pub const FS_42POSTBLFMT: i32 = -1;
/// `FS_DYNAMICPOSTBLFMT`: dynamic rotational table format.
pub const FS_DYNAMICPOSTBLFMT: i32 = 1;

/// `fs_rotbl(fs)`: the byte offset in the super-block of the rotational table.
pub fn fs_rotbl(fs: &Superblock) -> usize {
    if fs.fs_postblformat() == FS_42POSTBLFMT {
        offset_of!(Fs, fs_space)
    } else {
        fs.fs_rotbloff() as usize
    }
}

/// `CGSIZE(fs)`: the size of a cylinder group block: the `struct cg`, the block totals,
/// the rotational positions, the inode and block maps and, if present, the cluster summary
/// and map.
pub fn cgsize(fs: &Superblock) -> usize {
    let cpg = fs.fs_cpg() as usize;
    let mut size = size_of::<Cg>()
        + size_of::<i32>()
        + cpg * size_of::<i32>()
        + cpg * fs.fs_nrpos() as usize * size_of::<i16>()
        + howmany(fs.fs_ipg() as usize, 8)
        + howmany(fs.fs_fpg() as usize, 8);
    if fs.fs_contigsumsize() > 0 {
        size += fs.fs_contigsumsize() as usize * size_of::<i32>()
            + howmany(fragstoblks(fs, i64::from(fs.fs_fpg())) as usize, 8);
    }
    size
}

/// `CG_MAGIC`.
pub const CG_MAGIC: i32 = 0x090255;

/// `struct cg`: cylinder group block for a file system (actually longer: the maps follow),
/// as the disk holds it; [`CgView`] reads and writes it in a buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cg {
    /// `cg_firstfield`: historic cyl groups linked list.
    pub cg_firstfield: i32,
    /// `cg_magic`: magic number.
    pub cg_magic: i32,
    /// `cg_time`: time last written.
    pub cg_time: i32,
    /// `cg_cgx`: we are the cgx'th cylinder group.
    pub cg_cgx: u32,
    /// `cg_ncyl`: number of cyl's this cg.
    pub cg_ncyl: i16,
    /// `cg_niblk`: number of inode blocks this cg.
    pub cg_niblk: i16,
    /// `cg_ndblk`: number of data blocks this cg.
    pub cg_ndblk: u32,
    /// `cg_cs`: cylinder summary information.
    pub cg_cs: Csum,
    /// `cg_rotor`: position of last used block.
    pub cg_rotor: u32,
    /// `cg_frotor`: position of last used frag.
    pub cg_frotor: u32,
    /// `cg_irotor`: position of last used inode.
    pub cg_irotor: u32,
    /// `cg_frsum`: counts of available frags.
    pub cg_frsum: [u32; MAXFRAG],
    /// `cg_btotoff`: (int32) block totals per cylinder.
    pub cg_btotoff: i32,
    /// `cg_boff`: (u_int16) free block positions.
    pub cg_boff: i32,
    /// `cg_iusedoff`: (u_int8) used inode map.
    pub cg_iusedoff: u32,
    /// `cg_freeoff`: (u_int8) free block map.
    pub cg_freeoff: u32,
    /// `cg_nextfreeoff`: (u_int8) next available space.
    pub cg_nextfreeoff: u32,
    /// `cg_clustersumoff`: (u_int32) counts of avail clusters.
    pub cg_clustersumoff: u32,
    /// `cg_clusteroff`: (u_int8) free cluster map.
    pub cg_clusteroff: u32,
    /// `cg_nclusterblks`: number of clusters this cg.
    pub cg_nclusterblks: u32,
    /// `cg_ffs2_niblk`: number of inode blocks this cg.
    pub cg_ffs2_niblk: u32,
    /// `cg_initediblk`: last initialized inode.
    pub cg_initediblk: u32,
    /// `cg_sparecon32`: reserved for future use.
    pub cg_sparecon32: [i32; 3],
    /// `cg_ffs2_time`: time last written.
    pub cg_ffs2_time: i64,
    /// `cg_sparecon64`: reserved for future use.
    pub cg_sparecon64: [i64; 3],
}

hal::abi_pod!(Cg {
    cg_firstfield: i32,
    cg_magic: i32,
    cg_time: i32,
    cg_cgx: u32,
    cg_ncyl: i16,
    cg_niblk: i16,
    cg_ndblk: u32,
    cg_cs: Csum,
    cg_rotor: u32,
    cg_frotor: u32,
    cg_irotor: u32,
    cg_frsum: [u32; MAXFRAG],
    cg_btotoff: i32,
    cg_boff: i32,
    cg_iusedoff: u32,
    cg_freeoff: u32,
    cg_nextfreeoff: u32,
    cg_clustersumoff: u32,
    cg_clusteroff: u32,
    cg_nclusterblks: u32,
    cg_ffs2_niblk: u32,
    cg_initediblk: u32,
    cg_sparecon32: [i32; 3],
    cg_ffs2_time: i64,
    cg_sparecon64: [i64; 3],
});

/// `struct ocg`, the layout of old file systems' cylinder groups: the offsets of its arrays,
/// which [`CgView`] uses when `cg_magic` is not where `struct cg` has it. Its header up to
/// `cg_frsum` is `struct cg`'s.
pub mod ocg {
    /// `offsetof(struct ocg, cg_btot)`: `int32_t cg_btot[32]`.
    pub const CG_BTOT: usize = 84;
    /// `offsetof(struct ocg, cg_b)`: `int16_t cg_b[32][8]`.
    pub const CG_B: usize = 212;
    /// `offsetof(struct ocg, cg_iused)`: `u_int8_t cg_iused[256]`.
    pub const CG_IUSED: usize = 724;
    /// `offsetof(struct ocg, cg_magic)`.
    pub const CG_MAGIC: usize = 980;
    /// `offsetof(struct ocg, cg_free)`: the free block map.
    pub const CG_FREE: usize = 984;
}

/// A cylinder group block in a busy buffer: `(struct cg *)bp->b_data` and the macros that
/// locate its arrays, over the token's bytes (`BufRef::data`) for as long as the caller
/// holds the buffer (the C's `cgp`, valid until `brelse`/`bdwrite`). The header's members
/// are read and written at their `offset_of!(Cg, ..)` ([`cg_field!`]); the maps are tails
/// of the bytes, each start checked against the block.
pub struct CgView<'a> {
    b: &'a mut [u8],
}

impl<'a> CgView<'a> {
    /// The cylinder group in `b` (the buffer's `b_bcount` bytes): `panic(9)` for a block
    /// shorter than `struct cg`, whose header the C would read past it.
    pub fn new(b: &'a mut [u8]) -> Self {
        if b.len() < size_of::<Cg>() {
            kpanic(format_args!("CgView: short buffer {}", b.len()));
        }
        Self { b }
    }

    /// The header member at byte `off` (inside the header, which `new` checked).
    #[inline]
    fn get<T: DinField>(&self, off: usize) -> T {
        get_ne(self.b, off)
    }

    /// The header member at byte `off` set to `v`.
    #[inline]
    fn set<T: DinField>(&mut self, off: usize, v: T) {
        set_ne(self.b, off, v);
    }

    /// Whether the header is the new `struct cg` (`cgp->cg_magic == CG_MAGIC`).
    #[inline]
    fn is_cg(&self) -> bool {
        self.cg_magic() == CG_MAGIC
    }

    /// `cg_chkmagic(cgp)`: either layout's magic number is there.
    pub fn cg_chkmagic(&self) -> bool {
        self.is_cg()
            || (self.b.len() >= ocg::CG_MAGIC + 4
                && get_ne::<i32>(self.b, ocg::CG_MAGIC) == CG_MAGIC)
    }

    /// The bytes from `off` to the end of the block, which must start past the header (the
    /// new layout's whole `struct cg`, the old one's up to `cg_frsum`): `panic(9)` for an
    /// offset outside the block, where the C would read or write past it.
    fn tail(&mut self, off: usize) -> &mut [u8] {
        let hdr = if self.is_cg() {
            size_of::<Cg>()
        } else {
            ocg::CG_BTOT
        };
        if off < hdr || off > self.b.len() {
            kpanic(format_args!(
                "cg map offset {off} out of range ({} bytes)",
                self.b.len()
            ));
        }
        &mut self.b[off..]
    }

    /// The `n` bytes at `off` of a map: `panic(9)` outside the block.
    fn at(&mut self, off: usize, n: usize) -> &mut [u8] {
        let t = self.tail(off);
        if t.len() < n {
            kpanic(format_args!("cg map entry at {off} past the block"));
        }
        &mut t[..n]
    }

    /// `cg_inosused(cgp)`: the used inode map.
    pub fn cg_inosused(&mut self) -> &mut [u8] {
        let off = if self.is_cg() {
            self.cg_iusedoff() as usize
        } else {
            ocg::CG_IUSED
        };
        self.tail(off)
    }

    /// `cg_blksfree(cgp)`: the free block (fragment) map.
    pub fn cg_blksfree(&mut self) -> &mut [u8] {
        let off = if self.is_cg() {
            self.cg_freeoff() as usize
        } else {
            ocg::CG_FREE
        };
        self.tail(off)
    }

    /// `cg_clustersfree(cgp)`: the free cluster map.
    pub fn cg_clustersfree(&mut self) -> &mut [u8] {
        let off = self.cg_clusteroff() as usize;
        self.tail(off)
    }

    /// The byte offset of `cg_clustersum(cgp)[i]`.
    fn clustersum_off(&self, i: usize) -> usize {
        (self.cg_clustersumoff() as usize).saturating_add(i.saturating_mul(4))
    }

    /// `cg_clustersum(cgp)[i]`.
    pub fn cg_clustersum(&mut self, i: usize) -> i32 {
        let off = self.clustersum_off(i);
        get_ne(self.at(off, 4), 0)
    }

    /// `cg_clustersum(cgp)[i] += d`.
    pub fn add_cg_clustersum(&mut self, i: usize, d: i32) {
        let off = self.clustersum_off(i);
        let b = self.at(off, 4);
        let v = get_ne::<i32>(b, 0).wrapping_add(d);
        set_ne(b, 0, v);
    }

    /// `cg_blktot(cgp)[cylno] += d` (FFS1's block totals per cylinder).
    pub fn add_cg_blktot(&mut self, cylno: usize, d: i32) {
        let base = if self.is_cg() {
            self.cg_btotoff() as usize
        } else {
            ocg::CG_BTOT
        };
        let b = self.at(base.saturating_add(cylno.saturating_mul(4)), 4);
        let v = get_ne::<i32>(b, 0).wrapping_add(d);
        set_ne(b, 0, v);
    }

    /// `cg_blks(fs, cgp, cylno)[pos] += d` (FFS1's free block positions).
    pub fn add_cg_blks(&mut self, fs: &Superblock, cylno: usize, pos: usize, d: i16) {
        let off = if self.is_cg() {
            let i = cylno
                .saturating_mul(fs.fs_nrpos() as usize)
                .saturating_add(pos);
            (self.cg_boff() as usize).saturating_add(i.saturating_mul(2))
        } else {
            ocg::CG_B + (cylno * 8 + pos) * 2
        };
        let b = self.at(off, 2);
        let v = get_ne::<i16>(b, 0).wrapping_add(d);
        set_ne(b, 0, v);
    }

    /// `cgp->cg_frsum[i]`; `panic(9)` past `MAXFRAG`.
    pub fn cg_frsum(&self, i: usize) -> u32 {
        if i >= MAXFRAG {
            kpanic(format_args!("cg_frsum[{i}]"));
        }
        self.get(offset_of!(Cg, cg_frsum) + i * 4)
    }

    /// `cgp->cg_frsum[i] += d`, the C's arithmetic modulo 2^32.
    pub fn add_cg_frsum(&mut self, i: usize, d: i32) {
        let v = self.cg_frsum(i).wrapping_add_signed(d);
        self.set(offset_of!(Cg, cg_frsum) + i * 4, v);
    }
}

/// Generates the accessors of a cylinder group's header members: `$get` reads `cgp->member`
/// at `offset_of!(Cg, member)`, `$set` writes it, `$add` (counters) adds to it.
macro_rules! cg_field {
    ($($get:ident, $set:ident $(, $add:ident)?: $t:ty = $($f:ident).+;)+) => {
        impl CgView<'_> {
            $(
                #[doc = concat!("`cgp->", stringify!($($f).+), "`.")]
                #[inline]
                pub fn $get(&self) -> $t {
                    self.get::<$t>(offset_of!(Cg, $($f).+))
                }

                #[doc = concat!("`cgp->", stringify!($($f).+), " = v`.")]
                #[inline]
                pub fn $set(&mut self, v: $t) {
                    self.set::<$t>(offset_of!(Cg, $($f).+), v);
                }

                $(
                    #[doc = concat!("`", stringify!($get), "() += d`.")]
                    #[inline]
                    pub fn $add(&mut self, d: $t) {
                        let v = self.$get().wrapping_add(d);
                        self.$set(v);
                    }
                )?
            )+
        }
    };
}

cg_field! {
    cg_magic, set_cg_magic: i32 = cg_magic;
    cg_time, set_cg_time: i32 = cg_time;
    cg_cgx, set_cg_cgx: u32 = cg_cgx;
    cg_ndblk, set_cg_ndblk: u32 = cg_ndblk;
    cg_cs_ndir, set_cg_cs_ndir, add_cg_cs_ndir: i32 = cg_cs.cs_ndir;
    cg_cs_nbfree, set_cg_cs_nbfree, add_cg_cs_nbfree: i32 = cg_cs.cs_nbfree;
    cg_cs_nifree, set_cg_cs_nifree, add_cg_cs_nifree: i32 = cg_cs.cs_nifree;
    cg_cs_nffree, set_cg_cs_nffree, add_cg_cs_nffree: i32 = cg_cs.cs_nffree;
    cg_rotor, set_cg_rotor: u32 = cg_rotor;
    cg_frotor, set_cg_frotor: u32 = cg_frotor;
    cg_irotor, set_cg_irotor: u32 = cg_irotor;
    cg_btotoff, set_cg_btotoff: i32 = cg_btotoff;
    cg_boff, set_cg_boff: i32 = cg_boff;
    cg_iusedoff, set_cg_iusedoff: u32 = cg_iusedoff;
    cg_freeoff, set_cg_freeoff: u32 = cg_freeoff;
    cg_clustersumoff, set_cg_clustersumoff: u32 = cg_clustersumoff;
    cg_clusteroff, set_cg_clusteroff: u32 = cg_clusteroff;
    cg_nclusterblks, set_cg_nclusterblks: u32 = cg_nclusterblks;
    cg_ffs2_niblk, set_cg_ffs2_niblk: u32 = cg_ffs2_niblk;
    cg_initediblk, set_cg_initediblk: u32 = cg_initediblk;
    cg_ffs2_time, set_cg_ffs2_time: i64 = cg_ffs2_time;
}

/// The member of `T::SIZE` bytes at `off` of `b`, native-endian; `panic(9)` past `b`.
#[inline]
fn get_ne<T: DinField>(b: &[u8], off: usize) -> T {
    let Some(src) = b.get(off..off + T::SIZE) else {
        kpanic(format_args!("cg: member at {off} past the block"));
    };
    let mut w = [0u8; 8];
    if cfg!(target_endian = "little") {
        w[..T::SIZE].copy_from_slice(src);
    } else {
        w[8 - T::SIZE..].copy_from_slice(src);
    }
    T::from_bits(u64::from_ne_bytes(w))
}

/// The member of `T::SIZE` bytes at `off` of `b` set to `v`, native-endian.
#[inline]
fn set_ne<T: DinField>(b: &mut [u8], off: usize, v: T) {
    let w = v.to_bits().to_ne_bytes();
    let Some(dst) = b.get_mut(off..off + T::SIZE) else {
        kpanic(format_args!("cg: member at {off} past the block"));
    };
    if cfg!(target_endian = "little") {
        dst.copy_from_slice(&w[..T::SIZE]);
    } else {
        dst.copy_from_slice(&w[8 - T::SIZE..]);
    }
}

/// `setbit(a, i)` (`<sys/param.h>`).
#[inline]
pub fn setbit(a: &mut [u8], i: usize) {
    a[i / 8] |= 1 << (i % 8);
}

/// `clrbit(a, i)`.
#[inline]
pub fn clrbit(a: &mut [u8], i: usize) {
    a[i / 8] &= !(1 << (i % 8));
}

/// `isset(a, i)`.
#[inline]
pub fn isset(a: &[u8], i: usize) -> bool {
    a[i / 8] & (1 << (i % 8)) != 0
}

/// `isclr(a, i)`.
#[inline]
pub fn isclr(a: &[u8], i: usize) -> bool {
    !isset(a, i)
}

/// `fsbtodb(fs, b)`: file system blocks (fragments) to `DEV_BSIZE` disk blocks.
#[inline]
pub fn fsbtodb(fs: &Superblock, b: Daddr) -> Daddr {
    b << fs.fs_fsbtodb()
}

/// `dbtofsb(fs, b)`.
#[inline]
pub fn dbtofsb(fs: &Superblock, b: Daddr) -> Daddr {
    b >> fs.fs_fsbtodb()
}

/// `cgbase(fs, c)`: the first fragment of cylinder group `c`.
#[inline]
pub fn cgbase(fs: &Superblock, c: u32) -> Daddr {
    Daddr::from(fs.fs_fpg()) * Daddr::from(c)
}

/// `cgdata(fs, c)`: data zone.
#[inline]
pub fn cgdata(fs: &Superblock, c: u32) -> Daddr {
    cgdmin(fs, c) + Daddr::from(fs.fs_minfree())
}

/// `cgmeta(fs, c)`: meta data.
#[inline]
pub fn cgmeta(fs: &Superblock, c: u32) -> Daddr {
    cgdmin(fs, c)
}

/// `cgdmin(fs, c)`: 1st data.
#[inline]
pub fn cgdmin(fs: &Superblock, c: u32) -> Daddr {
    cgstart(fs, c) + Daddr::from(fs.fs_dblkno())
}

/// `cgimin(fs, c)`: inode blk.
#[inline]
pub fn cgimin(fs: &Superblock, c: u32) -> Daddr {
    cgstart(fs, c) + Daddr::from(fs.fs_iblkno())
}

/// `cgsblock(fs, c)`: super blk.
#[inline]
pub fn cgsblock(fs: &Superblock, c: u32) -> Daddr {
    cgstart(fs, c) + Daddr::from(fs.fs_sblkno())
}

/// `cgtod(fs, c)`: cg block.
#[inline]
pub fn cgtod(fs: &Superblock, c: u32) -> Daddr {
    cgstart(fs, c) + Daddr::from(fs.fs_cblkno())
}

/// `cgstart(fs, c)`.
#[inline]
pub fn cgstart(fs: &Superblock, c: u32) -> Daddr {
    cgbase(fs, c) + Daddr::from(fs.fs_cgoffset()) * Daddr::from(c & !(fs.fs_cgmask() as u32))
}

/// `ino_to_cg(fs, x)`: inode number to cylinder group number.
#[inline]
pub fn ino_to_cg(fs: &Superblock, x: Ufsino) -> u32 {
    x / fs.fs_ipg()
}

/// `ino_to_fsba(fs, x)`: inode number to file system block address.
#[inline]
pub fn ino_to_fsba(fs: &Superblock, x: Ufsino) -> Daddr {
    cgimin(fs, ino_to_cg(fs, x)) + blkstofrags(fs, Daddr::from((x % fs.fs_ipg()) / inopb(fs)))
}

/// `ino_to_fsbo(fs, x)`: inode number to file system block offset.
#[inline]
pub fn ino_to_fsbo(fs: &Superblock, x: Ufsino) -> usize {
    (x % inopb(fs)) as usize
}

/// `dtog(fs, d)`: the cylinder group number of a file system block.
#[inline]
pub fn dtog(fs: &Superblock, d: Daddr) -> u32 {
    (d / Daddr::from(fs.fs_fpg())) as u32
}

/// `dtogd(fs, d)`: the frag block number in its cylinder group of a file system block.
#[inline]
pub fn dtogd(fs: &Superblock, d: Daddr) -> Daddr {
    d % Daddr::from(fs.fs_fpg())
}

/// `blkmap(fs, map, loc)`: the bits for the block at fragment `loc` in a map.
#[inline]
pub fn blkmap(fs: &Superblock, map: &[u8], loc: Daddr) -> i32 {
    let loc = loc as usize;
    (i32::from(map[loc / 8]) >> (loc % 8)) & (0xff >> (8 - fs.fs_frag()))
}

/// `cbtocylno(fs, bno)`: the cylinder of a cylinder group block address.
#[inline]
pub fn cbtocylno(fs: &Superblock, bno: Daddr) -> usize {
    (fsbtodb(fs, bno) / Daddr::from(fs.fs_spc())) as usize
}

/// `cbtorpos(fs, bno)`: the rotational position of a cylinder group block address.
pub fn cbtorpos(fs: &Superblock, bno: Daddr) -> usize {
    if fs.fs_nrpos() <= 1 {
        return 0;
    }
    let db = fsbtodb(fs, bno);
    let spc = Daddr::from(fs.fs_spc());
    let nsect = Daddr::from(fs.fs_nsect());
    ((db % spc / nsect * Daddr::from(fs.fs_trackskew())
        + db % spc % nsect * Daddr::from(fs.fs_interleave()))
        % nsect
        * Daddr::from(fs.fs_nrpos())
        / Daddr::from(fs.fs_npsect())) as usize
}

/// `blkoff(fs, loc)`: calculates `loc % fs->fs_bsize`.
#[inline]
pub fn blkoff(fs: &Superblock, loc: Off) -> Off {
    loc & fs.fs_qbmask()
}

/// `fragoff(fs, loc)`: calculates `loc % fs->fs_fsize`.
#[inline]
pub fn fragoff(fs: &Superblock, loc: Off) -> Off {
    loc & fs.fs_qfmask()
}

/// `lblktosize(fs, blk)`: calculates `(off_t)blk * fs->fs_bsize`.
#[inline]
pub fn lblktosize(fs: &Superblock, blk: Daddr) -> Off {
    blk << fs.fs_bshift()
}

/// `lblkno(fs, loc)`: calculates `loc / fs->fs_bsize`.
#[inline]
pub fn lblkno(fs: &Superblock, loc: Off) -> Daddr {
    loc >> fs.fs_bshift()
}

/// `numfrags(fs, loc)`: calculates `loc / fs->fs_fsize`.
#[inline]
pub fn numfrags(fs: &Superblock, loc: i64) -> i64 {
    loc >> fs.fs_fshift()
}

/// `blkroundup(fs, size)`: calculates `roundup(size, fs->fs_bsize)`.
#[inline]
pub fn blkroundup(fs: &Superblock, size: i64) -> i64 {
    (size + fs.fs_qbmask()) & i64::from(fs.fs_bmask())
}

/// `fragroundup(fs, size)`: calculates `roundup(size, fs->fs_fsize)`.
#[inline]
pub fn fragroundup(fs: &Superblock, size: i64) -> i64 {
    (size + fs.fs_qfmask()) & i64::from(fs.fs_fmask())
}

/// `fragstoblks(fs, frags)`: calculates `frags / fs->fs_frag`.
#[inline]
pub fn fragstoblks(fs: &Superblock, frags: Daddr) -> Daddr {
    frags >> fs.fs_fragshift()
}

/// `blkstofrags(fs, blks)`: calculates `blks * fs->fs_frag`.
#[inline]
pub fn blkstofrags(fs: &Superblock, blks: Daddr) -> Daddr {
    blks << fs.fs_fragshift()
}

/// `fragnum(fs, fsb)`: calculates `fsb % fs->fs_frag`.
#[inline]
pub fn fragnum(fs: &Superblock, fsb: Daddr) -> Daddr {
    fsb & Daddr::from(fs.fs_frag() - 1)
}

/// `blknum(fs, fsb)`: calculates `rounddown(fsb, fs->fs_frag)`.
#[inline]
pub fn blknum(fs: &Superblock, fsb: Daddr) -> Daddr {
    fsb & !Daddr::from(fs.fs_frag() - 1)
}

/// `freespace(fs, percentreserved)`: the number of available frags given a percentage to
/// hold in reserve.
pub fn freespace(fs: &Superblock, percentreserved: i32) -> i64 {
    blkstofrags(fs, fs.fs_cstotal_nbfree()) + fs.fs_cstotal_nffree()
        - (fs.fs_dsize() * i64::from(percentreserved) / 100)
}

/// `blksize(fs, ip, lbn)`: the size of a file block in the file system: a whole block, or
/// the fragments the last direct block of a small file holds.
#[inline]
pub fn blksize<G: ProcGlue>(fs: &Superblock, ip: &Inode<G>, lbn: Daddr) -> u64 {
    sblksize(fs, ip.dip_size(), lbn)
}

/// `dblksize(fs, dip, lbn)`: as `blksize` for a dinode's `di_size`.
#[inline]
pub fn dblksize(fs: &Superblock, di_size: u64, lbn: Daddr) -> u64 {
    sblksize(fs, di_size, lbn)
}

/// `sblksize(fs, size, lbn)`: as `blksize` for a file of `size` bytes.
pub fn sblksize(fs: &Superblock, size: u64, lbn: Daddr) -> u64 {
    if lbn >= NDADDR as Daddr || size >= ((lbn + 1) << fs.fs_bshift()) as u64 {
        fs.fs_bsize() as u64
    } else {
        fragroundup(fs, blkoff(fs, size as Off)) as u64
    }
}

/// `NSPB(fs)`: number of disk sectors per block; assumes `DEV_BSIZE` byte sector size.
#[inline]
pub fn nspb(fs: &Superblock) -> i32 {
    fs.fs_nspf() << fs.fs_fragshift()
}

/// `NSPF(fs)`: number of disk sectors per fragment.
#[inline]
pub fn nspf(fs: &Superblock) -> i32 {
    fs.fs_nspf()
}

/// `INOPB(fs)`: number of inodes per file system block (`fs->fs_bsize`).
#[inline]
pub fn inopb(fs: &Superblock) -> u32 {
    fs.fs_inopb()
}

/// `INOPF(fs)`: number of inodes per file system fragment (`fs->fs_fsize`).
#[inline]
pub fn inopf(fs: &Superblock) -> u32 {
    fs.fs_inopb() >> fs.fs_fragshift()
}

/// `NINDIR(fs)`: number of indirects in a file system block.
#[inline]
pub fn nindir(fs: &Superblock) -> i64 {
    i64::from(fs.fs_nindir())
}

/// `FS_KERNMAXFILESIZE(pgsiz, fs)`: the maximum file size the kernel allows. Even though ffs
/// can handle files up to 16TB, we do limit the max file to 2^31 pages to prevent overflow of
/// a 32-bit unsigned int.
pub fn fs_kernmaxfilesize(pgsiz: u64, fs: &Superblock) -> u64 {
    0x8000_0000u64 * pgsiz.min(fs.fs_bsize() as u64) - 1
}

// The amd64/arm64 layout of `struct fs` and `struct cg` (both LP64), as the C compiler lays
// them out: `sizeof(struct fs)` is 1,384 (`fs_space[1]` at 1,376 and the padding).
const _: () = {
    assert!(size_of::<Csum>() == 16);
    assert!(size_of::<CsumTotal>() == 64);
    assert!(NOCSPTRS == 12);
    assert!(offset_of!(Fs, fs_sblkno) == 8);
    assert!(offset_of!(Fs, fs_ncg) == 44);
    assert!(offset_of!(Fs, fs_bsize) == 48);
    assert!(offset_of!(Fs, fs_minfree) == 60);
    assert!(offset_of!(Fs, fs_bmask) == 72);
    assert!(offset_of!(Fs, fs_fragshift) == 96);
    assert!(offset_of!(Fs, fs_sbsize) == 104);
    assert!(offset_of!(Fs, fs_nindir) == 116);
    assert!(offset_of!(Fs, fs_optim) == 128);
    assert!(offset_of!(Fs, fs_id) == 144);
    assert!(offset_of!(Fs, fs_ffs1_csaddr) == 152);
    assert!(offset_of!(Fs, fs_cgsize) == 160);
    assert!(offset_of!(Fs, fs_ipg) == 184);
    assert!(offset_of!(Fs, fs_fpg) == 188);
    assert!(offset_of!(Fs, fs_ffs1_cstotal) == 192);
    assert!(offset_of!(Fs, fs_fmod) == 208);
    assert!(offset_of!(Fs, fs_clean) == 209);
    assert!(offset_of!(Fs, fs_ronly) == 210);
    assert!(offset_of!(Fs, fs_ffs1_flags) == 211);
    assert!(offset_of!(Fs, fs_fsmnt) == 212);
    assert!(offset_of!(Fs, fs_volname) == 680);
    assert!(offset_of!(Fs, fs_swuid) == 712);
    assert!(offset_of!(Fs, fs_cgrotor) == 724);
    assert!(offset_of!(Fs, fs_ocsp) == 728);
    assert!(offset_of!(Fs, fs_contigdirs) == 824);
    assert!(offset_of!(Fs, fs_csp) == 832);
    assert!(offset_of!(Fs, fs_maxcluster) == 840);
    assert!(offset_of!(Fs, fs_active) == 848);
    assert!(offset_of!(Fs, fs_cpc) == 856);
    assert!(offset_of!(Fs, fs_maxbsize) == 860);
    assert!(offset_of!(Fs, fs_spareconf64) == 864);
    assert!(offset_of!(Fs, fs_sblockloc) == 1000);
    assert!(offset_of!(Fs, fs_cstotal) == 1008);
    assert!(offset_of!(Fs, fs_time) == 1072);
    assert!(offset_of!(Fs, fs_size) == 1080);
    assert!(offset_of!(Fs, fs_dsize) == 1088);
    assert!(offset_of!(Fs, fs_csaddr) == 1096);
    assert!(offset_of!(Fs, fs_pendinginodes) == 1112);
    assert!(offset_of!(Fs, fs_snapinum) == 1116);
    assert!(offset_of!(Fs, fs_avgfilesize) == 1196);
    assert!(offset_of!(Fs, fs_sparecon) == 1204);
    assert!(offset_of!(Fs, fs_flags) == 1308);
    assert!(offset_of!(Fs, fs_contigsumsize) == 1316);
    assert!(offset_of!(Fs, fs_maxsymlinklen) == 1320);
    assert!(offset_of!(Fs, fs_inodefmt) == 1324);
    assert!(offset_of!(Fs, fs_maxfilesize) == 1328);
    assert!(offset_of!(Fs, fs_qbmask) == 1336);
    assert!(offset_of!(Fs, fs_qfmask) == 1344);
    assert!(offset_of!(Fs, fs_state) == 1352);
    assert!(offset_of!(Fs, fs_postblformat) == 1356);
    assert!(offset_of!(Fs, fs_nrpos) == 1360);
    assert!(offset_of!(Fs, fs_rotbloff) == 1368);
    assert!(offset_of!(Fs, fs_magic) == 1372);
    assert!(offset_of!(Fs, fs_space) == 1376);
    assert!(size_of::<Fs>() == 1384);
    assert!(offset_of!(Cg, cg_magic) == 4);
    assert!(offset_of!(Cg, cg_cs) == 24);
    assert!(offset_of!(Cg, cg_rotor) == 40);
    assert!(offset_of!(Cg, cg_frsum) == 52);
    assert!(offset_of!(Cg, cg_btotoff) == ocg::CG_BTOT);
    assert!(offset_of!(Cg, cg_iusedoff) == 92);
    assert!(offset_of!(Cg, cg_ffs2_niblk) == 116);
    assert!(offset_of!(Cg, cg_initediblk) == 120);
    assert!(offset_of!(Cg, cg_ffs2_time) == 136);
    assert!(size_of::<Cg>() == 168);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use hal::user::pod_to_bytes;

    use super::*;
    use crate::testutil;

    /// A super-block of FFS2's usual geometry with a distinct value in each member the
    /// tests read.
    fn fs2() -> Fs {
        let mut fs = Fs::zeroed();
        fs.fs_sblkno = 40;
        fs.fs_cblkno = 48;
        fs.fs_iblkno = 56;
        fs.fs_dblkno = 312;
        fs.fs_ncg = 3;
        fs.fs_bsize = 8192;
        fs.fs_fsize = 1024;
        fs.fs_frag = 8;
        fs.fs_minfree = 5;
        fs.fs_bmask = !8191;
        fs.fs_fmask = !1023;
        fs.fs_bshift = 13;
        fs.fs_fshift = 10;
        fs.fs_fragshift = 3;
        fs.fs_fsbtodb = 1;
        fs.fs_sbsize = 2048;
        fs.fs_nindir = 1024;
        fs.fs_inopb = 32;
        fs.fs_id = [7, -8];
        fs.fs_cssize = 1024;
        fs.fs_cgsize = 8192;
        fs.fs_ipg = 512;
        fs.fs_fpg = 2048;
        fs.fs_fmod = -2;
        fs.fs_clean = 1;
        fs.fs_fsmnt[..4].copy_from_slice(b"/mnt");
        fs.fs_contigdirs = 0xdead_beef;
        fs.fs_csp = 0xfeed_face;
        fs.fs_maxcluster = 0x1234;
        fs.fs_active = 0x5678;
        fs.fs_ocsp[3] = 0x99;
        fs.fs_cstotal.cs_nbfree = 1 << 40;
        fs.fs_cstotal.cs_nffree = 77;
        fs.fs_time = -3;
        fs.fs_size = 6144;
        fs.fs_dsize = 6000;
        fs.fs_csaddr = 312;
        fs.fs_contigsumsize = 0;
        fs.fs_maxsymlinklen = 120;
        fs.fs_maxfilesize = u64::MAX - 5;
        fs.fs_qbmask = 8191;
        fs.fs_qfmask = 1023;
        fs.fs_magic = FS_UFS2_MAGIC;
        fs.fs_space = [1, 2, 3, 4, 5, 6, 7, 8];
        fs
    }

    /// `fs`'s bytes followed by `extra` bytes of a rotational table, as `fs_sbsize` covers
    /// them.
    fn sb_bytes(fs: &Fs, sbsize: usize) -> Vec<u8> {
        let mut b = vec![0u8; sbsize.max(size_of::<Fs>())];
        assert!(pod_to_bytes(fs, &mut b));
        for (i, x) in b.iter_mut().enumerate().skip(size_of::<Fs>()) {
            *x = i as u8;
        }
        b.truncate(sbsize);
        b
    }

    #[test]
    fn the_accessors_read_and_write_each_member_at_its_offset() {
        testutil::install();
        let want = fs2();
        let b = sb_bytes(&want, 2048);
        let fs = Superblock::new(&b);
        assert_eq!(fs.copy_len(), 2048);
        assert_eq!(fs.fs_bsize(), 8192);
        assert_eq!(fs.fs_ncg(), 3);
        assert_eq!(fs.fs_inopb(), 32);
        assert_eq!(fs.fs_id(), [7, -8]);
        assert_eq!(fs.fs_fmod(), -2);
        assert_eq!(fs.fs_cstotal_nbfree(), 1 << 40);
        assert_eq!(fs.fs_maxfilesize(), u64::MAX - 5);
        assert_eq!(fs.fs_magic(), FS_UFS2_MAGIC);
        assert_eq!(fs.fsmnt().as_bytes(), b"/mnt");
        assert_eq!(std::format!("{}", fs.fsmnt()), "/mnt");

        // A member changed alone keeps its word's neighbours.
        fs.set_fs_fmod(1);
        fs.set_fs_ronly(1);
        fs.add_fs_cstotal_nbfree(-1);
        fs.add_fs_cstotal_nffree(3);
        fs.set_fs_cgrotor(2);
        fs.set_fsmnt(b"/usr\0junk");
        let mut out = vec![0u8; 2048];
        fs.read_bytes(0, &mut out);
        let got: Fs = pod_from_bytes(&out).unwrap();
        let mut w = want;
        w.fs_fmod = 1;
        w.fs_ronly = 1;
        w.fs_cstotal.cs_nbfree -= 1;
        w.fs_cstotal.cs_nffree += 3;
        w.fs_cgrotor = 2;
        w.fs_fsmnt = [0; MAXMNTLEN];
        w.fs_fsmnt[..4].copy_from_slice(b"/usr");
        // The in-core address slot of the summaries is 0; the others the disk's.
        w.fs_csp = 0;
        assert_eq!(got, w);
        // The bytes past struct fs (the rotational table) are the disk's.
        assert_eq!(out[size_of::<Fs>()..], b[size_of::<Fs>()..]);
    }

    #[test]
    fn a_name_without_its_nul_stops_at_the_member() {
        testutil::install();
        let mut f = fs2();
        f.fs_fsmnt = [b'a'; MAXMNTLEN];
        let fs = Superblock::new(&sb_bytes(&f, 2048));
        assert_eq!(fs.fsmnt().as_bytes().len(), MAXMNTLEN);
        fs.set_fsmnt(&[b'b'; 600]);
        assert_eq!(fs.fsmnt().as_bytes(), &[b'b'; MAXMNTLEN - 1][..]);
    }

    #[test]
    fn a_short_super_block_is_held_in_struct_fs() {
        testutil::install();
        let f = fs2();
        let fs = Superblock::new(&sb_bytes(&f, 1024));
        assert_eq!(fs.copy_len(), size_of::<Fs>());
        assert_eq!(fs.fs_bsize(), 8192);
        assert_eq!(fs.fs_magic(), 0, "past the disk's bytes: zero");
    }

    #[test]
    fn the_summaries_and_the_cluster_counters_share_one_space() {
        testutil::install();
        let mut f = fs2();
        f.fs_contigsumsize = 4;
        let fs = Superblock::new(&sb_bytes(&f, 2048));
        // fs_cssize 1024 bytes in 1 KB fragments, then fs_ncg cluster counters.
        assert_eq!(fs.csum_len(), 1024);
        assert_eq!(fs.space.len(), 256 + 3);
        assert_eq!(fs.fs_maxcluster_slot(), 0);
        for cg in 0..3 {
            assert_eq!(fs.maxcluster(cg), 4);
        }
        let mut cs = vec![0u8; 48];
        for (i, c) in cs.chunks_exact_mut(4).enumerate() {
            c.copy_from_slice(&(i as i32 * 10).to_ne_bytes());
        }
        fs.write_csum_bytes(0, &cs);
        assert_eq!(fs.fs_cs(1).cs_ndir(), 40);
        assert_eq!(fs.fs_cs(2).cs_nffree(), 110);
        fs.fs_cs(1).add_cs_nbfree(-51);
        let mut back = vec![0u8; 48];
        fs.read_csum_bytes(0, &mut back);
        assert_eq!(back[20..24], (-1i32).to_ne_bytes());
        fs.set_maxcluster(2, 1);
        assert_eq!(fs.maxcluster(2), 1);
        assert_eq!(fs.fs_cs(2).cs_ndir(), 80, "the counters past the summaries");
    }

    #[test]
    #[should_panic(expected = "fs_cs: cg 256")]
    fn a_summary_past_the_area_is_refused() {
        testutil::install();
        let fs = Superblock::new(&sb_bytes(&fs2(), 2048));
        let _ = fs.fs_cs(256);
    }

    #[test]
    #[should_panic(expected = "fs_maxcluster: cg 0 of 0")]
    fn no_cluster_counters_without_a_cluster_summary() {
        testutil::install();
        let fs = Superblock::new(&sb_bytes(&fs2(), 2048));
        fs.set_maxcluster(0, 1);
    }

    #[test]
    fn contigdirs_come_and_go_with_a_read_write_mount() {
        testutil::install();
        let fs = Superblock::new(&sb_bytes(&fs2(), 2048));
        fs.contigdirs_alloc();
        assert_eq!(fs.fs_contigdirs_slot(), 0);
        fs.with_contigdirs(|c| {
            assert_eq!(c.len(), 3);
            c[1].store(9, Ordering::Relaxed);
        });
        assert_eq!(fs.with_contigdirs(|c| c[1].load(Ordering::Relaxed)), 9);
        fs.contigdirs_free();
        fs.contigdirs_alloc();
        assert_eq!(fs.with_contigdirs(|c| c[1].load(Ordering::Relaxed)), 0);
    }

    /// A new-layout cylinder group block of `len` bytes: magic, cgx 2, the maps at 168
    /// (inodes, 8 bytes), 176 (fragments, 16 bytes), the cluster summary at 192 and the
    /// cluster map at 208, the FFS1 totals at 220 and positions at 228.
    fn cg_block(len: usize) -> Vec<u8> {
        let c = Cg {
            cg_magic: CG_MAGIC,
            cg_cgx: 2,
            cg_cs: Csum {
                cs_ndir: 1,
                cs_nbfree: 2,
                cs_nifree: 3,
                cs_nffree: 4,
            },
            cg_frsum: [0, 5, 0, 0, 0, 0, 0, 0],
            cg_btotoff: 220,
            cg_boff: 228,
            cg_iusedoff: 168,
            cg_freeoff: 176,
            cg_clustersumoff: 192,
            cg_clusteroff: 208,
            ..Cg::default()
        };
        let mut b = vec![0u8; len];
        assert!(pod_to_bytes(&c, &mut b));
        b
    }

    #[test]
    fn a_cylinder_group_is_read_and_changed_in_its_buffer() {
        let mut b = cg_block(256);
        {
            let mut cgp = CgView::new(&mut b);
            assert!(cgp.cg_chkmagic());
            assert_eq!(cgp.cg_cgx(), 2);
            assert_eq!(cgp.cg_cs_nifree(), 3);
            cgp.add_cg_cs_nbfree(-1);
            cgp.add_cg_frsum(1, -6);
            assert_eq!(cgp.cg_frsum(1), u32::MAX, "the C's u_int32_t arithmetic");
            setbit(cgp.cg_inosused(), 9);
            setbit(cgp.cg_blksfree(), 0);
            cgp.add_cg_clustersum(1, 7);
            setbit(cgp.cg_clustersfree(), 3);
            cgp.add_cg_blktot(1, 1);
            let fs = Superblock::new(&sb_bytes(
                &{
                    let mut f = fs2();
                    f.fs_nrpos = 1;
                    f
                },
                2048,
            ));
            cgp.add_cg_blks(&fs, 2, 0, -1);
            cgp.set_cg_irotor(9);
            cgp.set_cg_ffs2_time(-1);
        }
        let got: Cg = pod_from_bytes(&b).unwrap();
        assert_eq!(got.cg_cs.cs_nbfree, 1);
        assert_eq!(got.cg_frsum[1], u32::MAX);
        assert_eq!(got.cg_irotor, 9);
        assert_eq!(got.cg_ffs2_time, -1);
        assert_eq!(b[168 + 1], 0x02, "inode 9's bit");
        assert_eq!(b[176], 0x01);
        assert_eq!(b[196..200], 7i32.to_ne_bytes());
        assert_eq!(b[208], 0x08);
        assert_eq!(b[224..228], 1i32.to_ne_bytes());
        assert_eq!(b[232..234], (-1i16).to_ne_bytes());
    }

    #[test]
    fn an_old_cylinder_group_is_found_by_its_other_magic() {
        let mut b = vec![0u8; 2048];
        b[ocg::CG_MAGIC..ocg::CG_MAGIC + 4].copy_from_slice(&CG_MAGIC.to_ne_bytes());
        let mut cgp = CgView::new(&mut b);
        assert!(cgp.cg_chkmagic());
        setbit(cgp.cg_inosused(), 0);
        setbit(cgp.cg_blksfree(), 8);
        cgp.add_cg_blktot(0, 2);
        drop(cgp);
        assert_eq!(b[ocg::CG_IUSED], 1);
        assert_eq!(b[ocg::CG_FREE + 1], 1);
        assert_eq!(b[ocg::CG_BTOT..ocg::CG_BTOT + 4], 2i32.to_ne_bytes());
        // Neither magic: not a cylinder group.
        let mut z = vec![0u8; 2048];
        assert!(!CgView::new(&mut z).cg_chkmagic());
    }

    #[test]
    #[should_panic(expected = "cg map offset 300 out of range")]
    fn a_map_past_the_block_is_refused() {
        let mut b = cg_block(256);
        b[offset_of!(Cg, cg_freeoff)..offset_of!(Cg, cg_freeoff) + 4]
            .copy_from_slice(&300u32.to_ne_bytes());
        let mut cgp = CgView::new(&mut b);
        let _ = cgp.cg_blksfree();
    }

    #[test]
    #[should_panic(expected = "cg map offset 100 out of range")]
    fn a_map_inside_the_header_is_refused() {
        let mut b = cg_block(256);
        b[offset_of!(Cg, cg_iusedoff)..offset_of!(Cg, cg_iusedoff) + 4]
            .copy_from_slice(&100u32.to_ne_bytes());
        let mut cgp = CgView::new(&mut b);
        let _ = cgp.cg_inosused();
    }

    #[test]
    #[should_panic(expected = "cg map entry at 254 past the block")]
    fn a_summary_entry_across_the_end_is_refused() {
        let mut b = cg_block(256);
        b[offset_of!(Cg, cg_clustersumoff)..offset_of!(Cg, cg_clustersumoff) + 4]
            .copy_from_slice(&254u32.to_ne_bytes());
        let mut cgp = CgView::new(&mut b);
        let _ = cgp.cg_clustersum(0);
    }

    #[test]
    #[should_panic(expected = "CgView: short buffer 100")]
    fn a_block_shorter_than_the_header_is_refused() {
        let mut b = vec![0u8; 100];
        let _ = CgView::new(&mut b);
    }

    #[test]
    fn the_macros_compute_the_c_addresses() {
        testutil::install();
        let fs = Superblock::new(&sb_bytes(&fs2(), 2048));
        assert_eq!(fsbtodb(&fs, 10), 20);
        assert_eq!(dbtofsb(&fs, 20), 10);
        assert_eq!(cgbase(&fs, 2), 4096);
        assert_eq!(cgtod(&fs, 1), 2048 + 48);
        assert_eq!(cgdmin(&fs, 1), 2048 + 312);
        assert_eq!(cgdata(&fs, 1), 2048 + 312 + 5);
        assert_eq!(ino_to_cg(&fs, 1030), 2);
        // Inode 1030: group 2, the 6th of its block 0.
        assert_eq!(ino_to_fsba(&fs, 1030), 4096 + 56);
        assert_eq!(ino_to_fsbo(&fs, 1030), 6);
        assert_eq!(ino_to_fsba(&fs, 1024 + 40), 4096 + 56 + 8);
        assert_eq!(dtog(&fs, 4100), 2);
        assert_eq!(dtogd(&fs, 4100), 4);
        assert_eq!(blkoff(&fs, 8193), 1);
        assert_eq!(lblkno(&fs, 8193), 1);
        assert_eq!(lblktosize(&fs, 3), 24576);
        assert_eq!(numfrags(&fs, 3072), 3);
        assert_eq!(fragroundup(&fs, 1025), 2048);
        assert_eq!(blkroundup(&fs, 1), 8192);
        assert_eq!(fragstoblks(&fs, 17), 2);
        assert_eq!(blkstofrags(&fs, 2), 16);
        assert_eq!(fragnum(&fs, 17), 1);
        assert_eq!(blknum(&fs, 17), 16);
        assert_eq!(sblksize(&fs, 3000, 0), 3072);
        assert_eq!(sblksize(&fs, 3000, 1), 3072, "the last block's fragments");
        assert_eq!(sblksize(&fs, 9000, 0), 8192);
        assert_eq!(sblksize(&fs, 1 << 20, 0), 8192);
        assert_eq!(nindir(&fs), 1024);
        assert_eq!(inopf(&fs), 4);
        assert_eq!(fs_kernmaxfilesize(4096, &fs), 0x8000_0000u64 * 4096 - 1);
        // freespace: 8 frags per free block plus the free frags, less 5 % of 6000.
        assert_eq!(freespace(&fs, 5), (1i64 << 40) * 8 + 77 - 300);
        let map = [0b1111_0000u8, 0xff];
        assert_eq!(blkmap(&fs, &map, 0), 0xf0);
        assert_eq!(blkmap(&fs, &map, 8), 0xff);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn fs_h_constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ffs/fs.h");
        for (name, value) in [
            ("BBSIZE", BBSIZE as i64),
            ("SBSIZE", SBSIZE as i64),
            ("SBLOCK_UFS1", i64::from(SBLOCK_UFS1)),
            ("SBLOCK_UFS2", i64::from(SBLOCK_UFS2)),
            ("SBLOCK_PIGGY", i64::from(SBLOCK_PIGGY)),
            ("SBLOCKSIZE", SBLOCKSIZE as i64),
            ("MAXFRAG", MAXFRAG as i64),
            ("MINBSIZE", MINBSIZE as i64),
            ("MAXMNTLEN", MAXMNTLEN as i64),
            ("MAXVOLLEN", MAXVOLLEN as i64),
            ("FS_MAXCONTIG", FS_MAXCONTIG as i64),
            ("MINFREE", i64::from(MINFREE)),
            ("AVFILESIZ", i64::from(AVFILESIZ)),
            ("AFPDIR", i64::from(AFPDIR)),
            ("FSMAXSNAP", FSMAXSNAP as i64),
            ("FS_MAGIC", i64::from(FS_MAGIC)),
            ("FS_UFS1_MAGIC", i64::from(FS_UFS1_MAGIC)),
            ("FS_UFS2_MAGIC", i64::from(FS_UFS2_MAGIC)),
            ("FS_OKAY", i64::from(FS_OKAY)),
            ("FS_44INODEFMT", i64::from(FS_44INODEFMT)),
            ("FS_ISCLEAN", i64::from(FS_ISCLEAN)),
            ("FS_WASCLEAN", i64::from(FS_WASCLEAN)),
            ("FS_OPTTIME", i64::from(FS_OPTTIME)),
            ("FS_OPTSPACE", i64::from(FS_OPTSPACE)),
            ("FS_UNCLEAN", i64::from(FS_UNCLEAN)),
            ("FS_FLAGS_UPDATED", i64::from(FS_FLAGS_UPDATED)),
            ("FS_DYNAMICPOSTBLFMT", i64::from(FS_DYNAMICPOSTBLFMT)),
            ("CG_MAGIC", i64::from(CG_MAGIC)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
        // The negative ones are written `-1`.
        assert_eq!(defs.get("FS_42INODEFMT").map(|s| s.as_str()), Some("-1"));
        assert_eq!(defs.get("FS_42POSTBLFMT").map(|s| s.as_str()), Some("-1"));
    }
}
/* </TESTS> */
