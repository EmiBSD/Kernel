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
 *  Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ext2fs/ext2fs.h>`: the layout of a second extended file system: the super-block
//! (`struct ext2fs`), its in-core form (`struct m_ext2fs`, [`Ext2Superblock`]), the block
//! group descriptor (`struct ext2_gd`), the feature flags, and the macros that turn inode
//! numbers, logical blocks and fragments into disk addresses.
//!
//! Upstream: sys/ufs/ext2fs/ext2fs.h @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs.rs@3c62ede68802
//!
//! Each disk drive contains some number of file systems. A file system consists of a number
//! of block groups ("cylinder groups"). Each group has inodes and data. A file system is
//! described by its super-block, which in turn describes the groups. The super-block is
//! critical data and is replicated in the groups to protect against catastrophic loss.
//!
//! Inodes are, like in UFS, 32-bit unsigned integers ([`Ufsino`]). Disk blocks are 32-bit if
//! the file system is not operating in 64-bit mode (the incompatible ext4 64BIT flag); the C
//! says "XXX disk blocks are simply `u_int32_t` for now", and so does this port.
//!
//! Ext2 metadata is stored in little-endian byte order (the JBD2 journal of ext3 and ext4 is
//! big-endian).
//!
//! ## Deviations
//! - Every macro is a function with the macro's name ([`fsbtodb`], [`dbtofsb`], [`ino_to_cg`],
//!   [`ino_to_fsba`], [`ino_to_fsbo`], [`dtog`], [`dtogd`], [`blkoff`], [`lblktosize`],
//!   [`lblkno`], [`blkroundup`], [`fragroundup`], [`freespace`]), and `NINDIR` is [`nindir`].
//!   `fsbtodb`/`dbtofsb`/`lblktosize` work on `daddr_t`/`off_t` here, so a block number times
//!   the sectors per block does not wrap at 32 bits as the C macros do on a `u_int32_t`.
//!   `ino_to_*` wrap `x - 1` at 32 bits as the C does for inode 0.
//! - `e2fs_sbload`/`e2fs_sbsave`/`e2fs_cgload`/`e2fs_cgsave` are functions that are always
//!   compiled: they copy on a little-endian machine and call the byte swappers of
//!   `ext2fs_bswap.rs` on a big-endian one (the C picks one or the other with `#if
//!   BYTE_ORDER`); both machines are little-endian, so the swappers are not reached.
//! - The `static const` feature name tables are the constants `RO_COMPAT` and `INCOMPAT`.
//! - `cg_has_sb` computes in 64 bits, where the C's `int` powers overflow for huge `i`.
//! - `e2fs_overflow` is a free function over the in-core super-block.
//! - A group descriptor read past `e2fs_ncg` panics ([`Ext2Superblock::gd`]), where the C
//!   would read past its array.
//! - The in-core super-block's `malloc(M_UFSMNT)` holds `struct ext2fs` and `e2fs_fsmnt` (the
//!   C's `struct m_ext2fs` without its scalars, which live in the mount's UFS structure with
//!   the rest of [`Ext2Superblock`]): the `UFS mount` row's requests are the C's, its bytes
//!   about 70 fewer per ext2fs mount; an FFS mount's block grows by the size of the larger
//!   variant of `ufsmount_u` (`ufsmount.rs`).
//!
//! ## Redesign
//! - LZ's `MExt2fs`, the `malloc`ed `struct m_ext2fs` cast to `&'static MExt2fs` (its
//!   `struct ext2fs` in an `UnsafeCell` read and written through `with_e2fs`/`with_e2fs_mut`,
//!   its scalars `Cell`s, `e2fs_gd` a raw `Cell<*mut Ext2Gd>` dereferenced behind a bounds
//!   check: ten `unsafe` sites and an `unsafe impl Sync`) -> [`Ext2Superblock`] (D-N5.6-3),
//!   held by the mount's UFS structure (`Ufsmount`'s `UfsSuper::Ext2`, the C's `ufsmount_u`)
//!   and freed with its last handle: `struct ext2fs` and `e2fs_fsmnt` as relaxed atomic words
//!   in one `malloc(M_UFSMNT)` array (the FFS super-block's idiom, `ffs/fs.rs`, over
//!   `inode.rs`'s word core), each member read and written at `offset_of!(Ext2fs, member)` by
//!   the accessors [`e2fs_field!`] writes (`fs.e2fs_ipg()`, `fs.set_e2fs_fbcount(v)`); the
//!   scalars relaxed words (`fs.e2fs_bsize.get()`, LZ's shape); the group descriptors
//!   `e2fs_ngdb * e2fs_bsize` bytes of words in a second array (the C's one `mallocarray`),
//!   in a read-mostly slot that the mount fills and `ext2fs_reload` replaces, read four
//!   words a descriptor ([`Ext2Superblock::gd`]) and changed one member at a time
//!   (`set_gd_nbfree`, ...). Every writer holds the kernel lock, as in the C (every ext2fs
//!   path is `KERNEL_LOCK`ed), checked under `diagnostic` by the word core. Cost against the
//!   C: a member read is a load and a shift of its word; a descriptor read four loads, the
//!   slot's read section and a bounds compare where the C indexes a pointer; no lock and no
//!   interrupt-level change.
//! - [`Ext2fs`] and [`Ext2Gd`] are `hal::abi_pod!` images (`pod_from_bytes`/`pod_to_bytes`):
//!   LZ's `read_unaligned`/`write_unaligned` of the super-block (two `unsafe`) and its
//!   `mem::zeroed` (one) go, and so do `Ext2Gd`'s byte-by-byte conversions: the descriptors
//!   are copied as the bytes they are on a little-endian machine, as the C's `memcpy`.

use core::mem::{offset_of, size_of};
use core::sync::atomic::AtomicU64;

use hal::malloc::{M_WAITOK, MArray};
use hal::slot::ReadMostly;
use hal::sys::types::{Daddr, Off};
use hal::user::{pod_from_bytes, pod_to_bytes, zeroed};

use super::ext2fs_bswap::{e2fs_cg_bswap, e2fs_sb_bswap};
use super::ext2fs_dinode::EXT2_REV0_DINODE_SIZE;
use crate::assert::kpanic;
use crate::fs::bio::DEV_BSHIFT;
use crate::fs::ufs::ufs::dinode::Ufsino;
use crate::fs::ufs::ufs::inode::{
    DinField, I8Word, I32Word, I64Word, words_get, words_read_bytes, words_set, words_write_bytes,
};
use crate::fs::ufs::ufs::ufsmount::M_UFSMNT;

/// `BBSIZE`: the boot block's size.
pub const BBSIZE: usize = 1024;
/// `SBSIZE`: the super-block's size.
pub const SBSIZE: usize = 1024;
/// `BBOFF`.
pub const BBOFF: Off = 0;
/// `SBOFF`.
pub const SBOFF: Off = BBOFF + BBSIZE as Off;
/// `BBLOCK`.
pub const BBLOCK: Daddr = 0;
/// `SBLOCK`.
pub const SBLOCK: Daddr = BBLOCK + (BBSIZE >> DEV_BSHIFT) as Daddr;

/// `LOG_MINBSIZE`.
pub const LOG_MINBSIZE: u32 = 10;
/// `MINBSIZE`: the smallest allowable block size.
pub const MINBSIZE: usize = 1 << LOG_MINBSIZE;
/// `LOG_MINFSIZE`.
pub const LOG_MINFSIZE: u32 = 10;
/// `MINFSIZE`.
pub const MINFSIZE: usize = 1 << LOG_MINFSIZE;

/// `MAXMNTLEN`: the space allocated in `struct m_ext2fs` for the name mounted on.
pub const MAXMNTLEN: usize = 512;

/// `MINFREE`: the minimum acceptable percentage of file system blocks which may be free.
pub const MINFREE: i32 = 5;

/// `E2FS_MAGIC`: the ext2fs magic number.
pub const E2FS_MAGIC: u16 = 0xef53;
/// `E2FS_REV0`: revision level 0.
pub const E2FS_REV0: u32 = 0;
/// `E2FS_REV1`: revision level 1.
pub const E2FS_REV1: u32 = 1;

/// `EXT2F_COMPAT_PREALLOC`.
pub const EXT2F_COMPAT_PREALLOC: u32 = 0x0001;
/// `EXT2F_COMPAT_IMAGIC_INODES`.
pub const EXT2F_COMPAT_IMAGIC_INODES: u32 = 0x0002;
/// `EXT2F_COMPAT_HAS_JOURNAL`.
pub const EXT2F_COMPAT_HAS_JOURNAL: u32 = 0x0004;
/// `EXT2F_COMPAT_EXT_ATTR`.
pub const EXT2F_COMPAT_EXT_ATTR: u32 = 0x0008;
/// `EXT2F_COMPAT_RESIZE`.
pub const EXT2F_COMPAT_RESIZE: u32 = 0x0010;
/// `EXT2F_COMPAT_DIR_INDEX`.
pub const EXT2F_COMPAT_DIR_INDEX: u32 = 0x0020;
/// `EXT2F_COMPAT_SPARSE_SUPER2`.
pub const EXT2F_COMPAT_SPARSE_SUPER2: u32 = 0x0200;

/// `EXT2F_ROCOMPAT_SPARSE_SUPER`.
pub const EXT2F_ROCOMPAT_SPARSE_SUPER: u32 = 0x0001;
/// `EXT2F_ROCOMPAT_LARGE_FILE`.
pub const EXT2F_ROCOMPAT_LARGE_FILE: u32 = 0x0002;
/// `EXT2F_ROCOMPAT_BTREE_DIR`.
pub const EXT2F_ROCOMPAT_BTREE_DIR: u32 = 0x0004;
/// `EXT2F_ROCOMPAT_HUGE_FILE`.
pub const EXT2F_ROCOMPAT_HUGE_FILE: u32 = 0x0008;
/// `EXT2F_ROCOMPAT_GDT_CSUM`.
pub const EXT2F_ROCOMPAT_GDT_CSUM: u32 = 0x0010;
/// `EXT2F_ROCOMPAT_DIR_NLINK`.
pub const EXT2F_ROCOMPAT_DIR_NLINK: u32 = 0x0020;
/// `EXT2F_ROCOMPAT_EXTRA_ISIZE`.
pub const EXT2F_ROCOMPAT_EXTRA_ISIZE: u32 = 0x0040;
/// `EXT2F_ROCOMPAT_QUOTA`.
pub const EXT2F_ROCOMPAT_QUOTA: u32 = 0x0100;
/// `EXT2F_ROCOMPAT_BIGALLOC`.
pub const EXT2F_ROCOMPAT_BIGALLOC: u32 = 0x0200;
/// `EXT2F_ROCOMPAT_METADATA_CKSUM`.
pub const EXT2F_ROCOMPAT_METADATA_CKSUM: u32 = 0x0400;
/// `EXT2F_ROCOMPAT_READONLY`.
pub const EXT2F_ROCOMPAT_READONLY: u32 = 0x1000;
/// `EXT2F_ROCOMPAT_PROJECT`.
pub const EXT2F_ROCOMPAT_PROJECT: u32 = 0x2000;

/// `EXT2F_INCOMPAT_COMP`.
pub const EXT2F_INCOMPAT_COMP: u32 = 0x0001;
/// `EXT2F_INCOMPAT_FTYPE`.
pub const EXT2F_INCOMPAT_FTYPE: u32 = 0x0002;
/// `EXT2F_INCOMPAT_RECOVER`.
pub const EXT2F_INCOMPAT_RECOVER: u32 = 0x0004;
/// `EXT2F_INCOMPAT_JOURNAL_DEV`.
pub const EXT2F_INCOMPAT_JOURNAL_DEV: u32 = 0x0008;
/// `EXT2F_INCOMPAT_META_BG`.
pub const EXT2F_INCOMPAT_META_BG: u32 = 0x0010;
/// `EXT2F_INCOMPAT_EXTENTS`.
pub const EXT2F_INCOMPAT_EXTENTS: u32 = 0x0040;
/// `EXT2F_INCOMPAT_64BIT`.
pub const EXT2F_INCOMPAT_64BIT: u32 = 0x0080;
/// `EXT2F_INCOMPAT_MMP`.
pub const EXT2F_INCOMPAT_MMP: u32 = 0x0100;
/// `EXT2F_INCOMPAT_FLEX_BG`.
pub const EXT2F_INCOMPAT_FLEX_BG: u32 = 0x0200;
/// `EXT2F_INCOMPAT_EA_INODE`.
pub const EXT2F_INCOMPAT_EA_INODE: u32 = 0x0400;
/// `EXT2F_INCOMPAT_DIRDATA`.
pub const EXT2F_INCOMPAT_DIRDATA: u32 = 0x1000;
/// `EXT2F_INCOMPAT_CSUM_SEED`.
pub const EXT2F_INCOMPAT_CSUM_SEED: u32 = 0x2000;
/// `EXT2F_INCOMPAT_LARGEDIR`.
pub const EXT2F_INCOMPAT_LARGEDIR: u32 = 0x4000;
/// `EXT2F_INCOMPAT_INLINE_DATA`.
pub const EXT2F_INCOMPAT_INLINE_DATA: u32 = 0x8000;
/// `EXT2F_INCOMPAT_ENCRYPT`.
pub const EXT2F_INCOMPAT_ENCRYPT: u32 = 0x10000;

/// `struct ext2_feature`: a feature flag and its name.
#[derive(Clone, Copy, Debug)]
pub struct Ext2Feature {
    /// `mask`.
    pub mask: u32,
    /// `name`.
    pub name: &'static str,
}

/// `ro_compat[]`: the read-only compatible features and their names.
pub const RO_COMPAT: [Ext2Feature; 12] = [
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_SPARSE_SUPER,
        name: "sparse_super",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_LARGE_FILE,
        name: "large_file",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_BTREE_DIR,
        name: "btree_dir",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_HUGE_FILE,
        name: "huge_file",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_GDT_CSUM,
        name: "uninit_bg",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_DIR_NLINK,
        name: "dir_nlink",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_EXTRA_ISIZE,
        name: "extra_isize",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_QUOTA,
        name: "quota",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_BIGALLOC,
        name: "bigalloc",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_METADATA_CKSUM,
        name: "metadata_csum",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_READONLY,
        name: "read-only",
    },
    Ext2Feature {
        mask: EXT2F_ROCOMPAT_PROJECT,
        name: "project",
    },
];

/// `incompat[]`: the incompatible features and their names.
pub const INCOMPAT: [Ext2Feature; 15] = [
    Ext2Feature {
        mask: EXT2F_INCOMPAT_COMP,
        name: "compression",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_FTYPE,
        name: "filetype",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_RECOVER,
        name: "needs_recovery",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_JOURNAL_DEV,
        name: "journal_dev",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_META_BG,
        name: "meta_bg",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_EXTENTS,
        name: "extents",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_64BIT,
        name: "64bit",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_MMP,
        name: "mmp",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_FLEX_BG,
        name: "flex_bg",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_EA_INODE,
        name: "ea_inode",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_DIRDATA,
        name: "dirdata",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_CSUM_SEED,
        name: "metadata_csum_seed",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_LARGEDIR,
        name: "large_dir",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_INLINE_DATA,
        name: "inline_data",
    },
    Ext2Feature {
        mask: EXT2F_INCOMPAT_ENCRYPT,
        name: "encrypt",
    },
];

/// `EXT2F_COMPAT_SUPP`: compatible features supported in this implementation.
pub const EXT2F_COMPAT_SUPP: u32 = 0x0000;
/// `EXT2F_ROCOMPAT_SUPP`: read-only compatible features supported in this implementation.
pub const EXT2F_ROCOMPAT_SUPP: u32 = EXT2F_ROCOMPAT_SPARSE_SUPER | EXT2F_ROCOMPAT_LARGE_FILE;
/// `EXT2F_INCOMPAT_SUPP`: incompatible features supported in this implementation.
pub const EXT2F_INCOMPAT_SUPP: u32 = EXT2F_INCOMPAT_FTYPE;
/// `EXT4F_RO_INCOMPAT_SUPP`: incompatible features supported when mounted read-only.
pub const EXT4F_RO_INCOMPAT_SUPP: u32 = EXT2F_INCOMPAT_EXTENTS
    | EXT2F_INCOMPAT_FLEX_BG
    | EXT2F_INCOMPAT_META_BG
    | EXT2F_INCOMPAT_RECOVER;

/// `E2FS_BEH_CONTINUE`: continue operation.
pub const E2FS_BEH_CONTINUE: u16 = 1;
/// `E2FS_BEH_READONLY`: remount the file system read only.
pub const E2FS_BEH_READONLY: u16 = 2;
/// `E2FS_BEH_PANIC`: cause a panic.
pub const E2FS_BEH_PANIC: u16 = 3;
/// `E2FS_BEH_DEFAULT`.
pub const E2FS_BEH_DEFAULT: u16 = E2FS_BEH_CONTINUE;

/// `E2FS_OS_LINUX`.
pub const E2FS_OS_LINUX: u32 = 0;
/// `E2FS_OS_HURD`.
pub const E2FS_OS_HURD: u32 = 1;
/// `E2FS_OS_MASIX`.
pub const E2FS_OS_MASIX: u32 = 2;

/// `E2FS_ISCLEAN`: the file system is clean.
pub const E2FS_ISCLEAN: u16 = 0x01;
/// `E2FS_ERRORS`: the file system has errors.
pub const E2FS_ERRORS: u16 = 0x02;

/// `struct ext2fs`: the super block of an ext2fs file system, as it is on disk (little-endian
/// on disk; the in-core [`Ext2Superblock`] holds its bytes in native order, and this is the
/// layout its accessors name and what the tests' `newfs` writes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Ext2fs {
    /// `e2fs_icount`: inode count.
    pub e2fs_icount: u32,
    /// `e2fs_bcount`: blocks count.
    pub e2fs_bcount: u32,
    /// `e2fs_rbcount`: reserved blocks count.
    pub e2fs_rbcount: u32,
    /// `e2fs_fbcount`: free blocks count.
    pub e2fs_fbcount: u32,
    /// `e2fs_ficount`: free inodes count.
    pub e2fs_ficount: u32,
    /// `e2fs_first_dblock`: first data block.
    pub e2fs_first_dblock: u32,
    /// `e2fs_log_bsize`: block size = 1024*(2^e2fs_log_bsize).
    pub e2fs_log_bsize: u32,
    /// `e2fs_log_fsize`: fragment size log2.
    pub e2fs_log_fsize: u32,
    /// `e2fs_bpg`: blocks per group.
    pub e2fs_bpg: u32,
    /// `e2fs_fpg`: frags per group.
    pub e2fs_fpg: u32,
    /// `e2fs_ipg`: inodes per group.
    pub e2fs_ipg: u32,
    /// `e2fs_mtime`: mount time.
    pub e2fs_mtime: u32,
    /// `e2fs_wtime`: write time.
    pub e2fs_wtime: u32,
    /// `e2fs_mnt_count`: mount count.
    pub e2fs_mnt_count: u16,
    /// `e2fs_max_mnt_count`: max mount count.
    pub e2fs_max_mnt_count: u16,
    /// `e2fs_magic`: magic number.
    pub e2fs_magic: u16,
    /// `e2fs_state`: file system state.
    pub e2fs_state: u16,
    /// `e2fs_beh`: behavior on errors.
    pub e2fs_beh: u16,
    /// `e2fs_minrev`: minor revision level.
    pub e2fs_minrev: u16,
    /// `e2fs_lastfsck`: time of last fsck.
    pub e2fs_lastfsck: u32,
    /// `e2fs_fsckintv`: max time between fscks.
    pub e2fs_fsckintv: u32,
    /// `e2fs_creator`: creator OS.
    pub e2fs_creator: u32,
    /// `e2fs_rev`: revision level.
    pub e2fs_rev: u32,
    /// `e2fs_ruid`: default uid for reserved blocks.
    pub e2fs_ruid: u16,
    /// `e2fs_rgid`: default gid for reserved blocks.
    pub e2fs_rgid: u16,
    // EXT2_DYNAMIC_REV superblocks.
    /// `e2fs_first_ino`: first non-reserved inode.
    pub e2fs_first_ino: u32,
    /// `e2fs_inode_size`: size of inode structure.
    pub e2fs_inode_size: u16,
    /// `e2fs_block_group_nr`: block group number of this super block.
    pub e2fs_block_group_nr: u16,
    /// `e2fs_features_compat`: compatible feature set.
    pub e2fs_features_compat: u32,
    /// `e2fs_features_incompat`: incompatible feature set.
    pub e2fs_features_incompat: u32,
    /// `e2fs_features_rocompat`: RO-compatible feature set.
    pub e2fs_features_rocompat: u32,
    /// `e2fs_uuid`: 128-bit uuid for volume.
    pub e2fs_uuid: [u8; 16],
    /// `e2fs_vname`: volume name.
    pub e2fs_vname: [u8; 16],
    /// `e2fs_fsmnt`: name mounted on.
    pub e2fs_fsmnt: [u8; 64],
    /// `e2fs_algo`: for compression.
    pub e2fs_algo: u32,
    /// `e2fs_prealloc`: number of blocks to preallocate.
    pub e2fs_prealloc: u8,
    /// `e2fs_dir_prealloc`: number of blocks to preallocate for dir.
    pub e2fs_dir_prealloc: u8,
    /// `e2fs_reserved_ngdb`: number of reserved gd blocks for resize.
    pub e2fs_reserved_ngdb: u16,
    // Ext3 JBD2 journaling.
    /// `e2fs_journal_uuid`.
    pub e2fs_journal_uuid: [u8; 16],
    /// `e2fs_journal_ino`.
    pub e2fs_journal_ino: u32,
    /// `e2fs_journal_dev`.
    pub e2fs_journal_dev: u32,
    /// `e2fs_last_orphan`: start of list of inodes to delete.
    pub e2fs_last_orphan: u32,
    /// `e2fs_hash_seed`: htree hash seed.
    pub e2fs_hash_seed: [u32; 4],
    /// `e2fs_def_hash_version`.
    pub e2fs_def_hash_version: u8,
    /// `e2fs_journal_backup_type`.
    pub e2fs_journal_backup_type: u8,
    /// `e2fs_gdesc_size`.
    pub e2fs_gdesc_size: u16,
    /// `e2fs_default_mount_opts`.
    pub e2fs_default_mount_opts: u32,
    /// `e2fs_first_meta_bg`.
    pub e2fs_first_meta_bg: u32,
    /// `e2fs_mkfs_time`.
    pub e2fs_mkfs_time: u32,
    /// `e2fs_journal_backup`.
    pub e2fs_journal_backup: [u32; 17],
    /// `e2fs_bcount_hi`: high bits of blocks count.
    pub e2fs_bcount_hi: u32,
    /// `e2fs_rbcount_hi`: high bits of reserved blocks count.
    pub e2fs_rbcount_hi: u32,
    /// `e2fs_fbcount_hi`: high bits of free blocks count.
    pub e2fs_fbcount_hi: u32,
    /// `e2fs_min_extra_isize`: all inodes have some bytes.
    pub e2fs_min_extra_isize: u16,
    /// `e2fs_want_extra_isize`: inodes must reserve some bytes.
    pub e2fs_want_extra_isize: u16,
    /// `e2fs_flags`: miscellaneous flags.
    pub e2fs_flags: u32,
    /// `e2fs_raid_stride`: RAID stride.
    pub e2fs_raid_stride: u16,
    /// `e2fs_mmpintv`: seconds to wait in MMP checking.
    pub e2fs_mmpintv: u16,
    /// `e2fs_mmpblk`: block for multi-mount protection.
    pub e2fs_mmpblk: u64,
    /// `e2fs_raid_stripe_wid`: blocks on data disks (N * stride).
    pub e2fs_raid_stripe_wid: u32,
    /// `e2fs_log_gpf`: FLEX_BG group size.
    pub e2fs_log_gpf: u8,
    /// `e2fs_chksum_type`: metadata checksum algorithm used.
    pub e2fs_chksum_type: u8,
    /// `e2fs_encrypt`: versioning level for encryption.
    pub e2fs_encrypt: u8,
    /// `e2fs_reserved_pad`.
    pub e2fs_reserved_pad: u8,
    /// `e2fs_kbytes_written`: number of lifetime kilobytes.
    pub e2fs_kbytes_written: u64,
    /// `e2fs_snapinum`: inode number of active snapshot.
    pub e2fs_snapinum: u32,
    /// `e2fs_snapid`: sequential ID of active snapshot.
    pub e2fs_snapid: u32,
    /// `e2fs_snaprbcount`: reserved blocks for active snapshot.
    pub e2fs_snaprbcount: u64,
    /// `e2fs_snaplist`: inode number for on-disk snapshot.
    pub e2fs_snaplist: u32,
    /// `e2fs_errcount`: number of file system errors.
    pub e2fs_errcount: u32,
    /// `e2fs_first_errtime`: first time an error happened.
    pub e2fs_first_errtime: u32,
    /// `e2fs_first_errino`: inode involved in first error.
    pub e2fs_first_errino: u32,
    /// `e2fs_first_errblk`: block involved of first error.
    pub e2fs_first_errblk: u64,
    /// `e2fs_first_errfunc`: function where error happened.
    pub e2fs_first_errfunc: [u8; 32],
    /// `e2fs_first_errline`: line number where error happened.
    pub e2fs_first_errline: u32,
    /// `e2fs_last_errtime`: most recent time of an error.
    pub e2fs_last_errtime: u32,
    /// `e2fs_last_errino`: inode involved in last error.
    pub e2fs_last_errino: u32,
    /// `e2fs_last_errline`: line number where error happened.
    pub e2fs_last_errline: u32,
    /// `e2fs_last_errblk`: block involved of last error.
    pub e2fs_last_errblk: u64,
    /// `e2fs_last_errfunc`: function where error happened.
    pub e2fs_last_errfunc: [u8; 32],
    /// `e2fs_mount_opts`.
    pub e2fs_mount_opts: [u8; 64],
    /// `e2fs_usrquota_inum`: inode for tracking user quota.
    pub e2fs_usrquota_inum: u32,
    /// `e2fs_grpquota_inum`: inode for tracking group quota.
    pub e2fs_grpquota_inum: u32,
    /// `e2fs_overhead_clusters`: overhead blocks/clusters.
    pub e2fs_overhead_clusters: u32,
    /// `e2fs_backup_bgs`: groups with sparse_super2 super blocks.
    pub e2fs_backup_bgs: [u32; 2],
    /// `e2fs_encrypt_algos`: encryption algorithms in use.
    pub e2fs_encrypt_algos: [u8; 4],
    /// `e2fs_encrypt_pw_salt`: salt used for string2key.
    pub e2fs_encrypt_pw_salt: [u8; 16],
    /// `e2fs_lpf_ino`: location of the lost+found inode.
    pub e2fs_lpf_ino: u32,
    /// `e2fs_proj_quota_inum`: inode for tracking project quota.
    pub e2fs_proj_quota_inum: u32,
    /// `e2fs_chksum_seed`: checksum seed.
    pub e2fs_chksum_seed: u32,
    /// `e2fs_reserved`: padding to the end of the block.
    pub e2fs_reserved: [u32; 98],
    /// `e2fs_sbchksum`: super block checksum.
    pub e2fs_sbchksum: u32,
}

impl Ext2fs {
    /// A zeroed super block.
    pub fn new() -> Self {
        zeroed()
    }

    /// The super block in the first `SBSIZE` bytes of `bytes`, little-endian as on disk, in
    /// native byte order (`e2fs_sbload` into a structure of its own: `e2fs_sbcheck`'s copy).
    pub fn from_disk(bytes: &[u8]) -> Self {
        let Some(disk) = bytes.get(..SBSIZE).and_then(pod_from_bytes::<Ext2fs>) else {
            kpanic(format_args!(
                "e2fs_sbload: {} bytes, want {}",
                bytes.len(),
                SBSIZE
            ));
        };
        if cfg!(target_endian = "big") {
            let mut sb = Self::new();
            e2fs_sb_bswap(&disk, &mut sb);
            sb
        } else {
            disk
        }
    }

    /// The super block as the `SBSIZE` little-endian bytes it is on disk, into the first
    /// `SBSIZE` bytes of `out` (`e2fs_sbsave` from a structure of its own).
    pub fn to_disk(&self, out: &mut [u8]) {
        let mut disk = *self;
        if cfg!(target_endian = "big") {
            e2fs_sb_bswap(self, &mut disk);
        }
        if !out
            .get_mut(..SBSIZE)
            .is_some_and(|o| pod_to_bytes(&disk, o))
        {
            kpanic(format_args!(
                "e2fs_sbsave: {} bytes, want {}",
                out.len(),
                SBSIZE
            ));
        }
    }
}

impl Default for Ext2fs {
    fn default() -> Self {
        Self::new()
    }
}

hal::abi_pod!(Ext2fs {
    e2fs_icount: u32,
    e2fs_bcount: u32,
    e2fs_rbcount: u32,
    e2fs_fbcount: u32,
    e2fs_ficount: u32,
    e2fs_first_dblock: u32,
    e2fs_log_bsize: u32,
    e2fs_log_fsize: u32,
    e2fs_bpg: u32,
    e2fs_fpg: u32,
    e2fs_ipg: u32,
    e2fs_mtime: u32,
    e2fs_wtime: u32,
    e2fs_mnt_count: u16,
    e2fs_max_mnt_count: u16,
    e2fs_magic: u16,
    e2fs_state: u16,
    e2fs_beh: u16,
    e2fs_minrev: u16,
    e2fs_lastfsck: u32,
    e2fs_fsckintv: u32,
    e2fs_creator: u32,
    e2fs_rev: u32,
    e2fs_ruid: u16,
    e2fs_rgid: u16,
    e2fs_first_ino: u32,
    e2fs_inode_size: u16,
    e2fs_block_group_nr: u16,
    e2fs_features_compat: u32,
    e2fs_features_incompat: u32,
    e2fs_features_rocompat: u32,
    e2fs_uuid: [u8; 16],
    e2fs_vname: [u8; 16],
    e2fs_fsmnt: [u8; 64],
    e2fs_algo: u32,
    e2fs_prealloc: u8,
    e2fs_dir_prealloc: u8,
    e2fs_reserved_ngdb: u16,
    e2fs_journal_uuid: [u8; 16],
    e2fs_journal_ino: u32,
    e2fs_journal_dev: u32,
    e2fs_last_orphan: u32,
    e2fs_hash_seed: [u32; 4],
    e2fs_def_hash_version: u8,
    e2fs_journal_backup_type: u8,
    e2fs_gdesc_size: u16,
    e2fs_default_mount_opts: u32,
    e2fs_first_meta_bg: u32,
    e2fs_mkfs_time: u32,
    e2fs_journal_backup: [u32; 17],
    e2fs_bcount_hi: u32,
    e2fs_rbcount_hi: u32,
    e2fs_fbcount_hi: u32,
    e2fs_min_extra_isize: u16,
    e2fs_want_extra_isize: u16,
    e2fs_flags: u32,
    e2fs_raid_stride: u16,
    e2fs_mmpintv: u16,
    e2fs_mmpblk: u64,
    e2fs_raid_stripe_wid: u32,
    e2fs_log_gpf: u8,
    e2fs_chksum_type: u8,
    e2fs_encrypt: u8,
    e2fs_reserved_pad: u8,
    e2fs_kbytes_written: u64,
    e2fs_snapinum: u32,
    e2fs_snapid: u32,
    e2fs_snaprbcount: u64,
    e2fs_snaplist: u32,
    e2fs_errcount: u32,
    e2fs_first_errtime: u32,
    e2fs_first_errino: u32,
    e2fs_first_errblk: u64,
    e2fs_first_errfunc: [u8; 32],
    e2fs_first_errline: u32,
    e2fs_last_errtime: u32,
    e2fs_last_errino: u32,
    e2fs_last_errline: u32,
    e2fs_last_errblk: u64,
    e2fs_last_errfunc: [u8; 32],
    e2fs_mount_opts: [u8; 64],
    e2fs_usrquota_inum: u32,
    e2fs_grpquota_inum: u32,
    e2fs_overhead_clusters: u32,
    e2fs_backup_bgs: [u32; 2],
    e2fs_encrypt_algos: [u8; 4],
    e2fs_encrypt_pw_salt: [u8; 16],
    e2fs_lpf_ino: u32,
    e2fs_proj_quota_inum: u32,
    e2fs_chksum_seed: u32,
    e2fs_reserved: [u32; 98],
    e2fs_sbchksum: u32,
});

/// `struct ext2_gd`: an ext2 file system block group descriptor.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ext2Gd {
    /// `ext2bgd_b_bitmap`: blocks bitmap block.
    pub ext2bgd_b_bitmap: u32,
    /// `ext2bgd_i_bitmap`: inodes bitmap block.
    pub ext2bgd_i_bitmap: u32,
    /// `ext2bgd_i_tables`: inodes table block.
    pub ext2bgd_i_tables: u32,
    /// `ext2bgd_nbfree`: number of free blocks.
    pub ext2bgd_nbfree: u16,
    /// `ext2bgd_nifree`: number of free inodes.
    pub ext2bgd_nifree: u16,
    /// `ext2bgd_ndirs`: number of directories.
    pub ext2bgd_ndirs: u16,
    /// `reserved`.
    pub reserved: u16,
    /// `reserved2`.
    pub reserved2: [u32; 3],
}

hal::abi_pod!(Ext2Gd {
    ext2bgd_b_bitmap: u32,
    ext2bgd_i_bitmap: u32,
    ext2bgd_i_tables: u32,
    ext2bgd_nbfree: u16,
    ext2bgd_nifree: u16,
    ext2bgd_ndirs: u16,
    reserved: u16,
    reserved2: [u32; 3],
});

/// `sizeof(struct ext2_gd)`.
pub const EXT2_GD_SIZE: usize = size_of::<Ext2Gd>();

/// The bytes of the in-core super-block's array: `struct ext2fs`, then `e2fs_fsmnt`.
const SB_WORDS: usize = (SBSIZE + MAXMNTLEN) / 8;

/// `struct m_ext2fs` (`ump->um_e2fs`): the in-core super-block of an ext2fs mount, set once
/// by `ext2fs_mountfs` into its mount's UFS structure (`Ufsmount::set_um_e2fs`) and freed
/// with it.
///
/// Locks: every writer holds the kernel lock (each ext2fs path runs `KERNEL_LOCK`ed, as in
/// the C), which orders the writes: a member's change is a load and a store of its word,
/// checked under `diagnostic`. Readers need no lock: a relaxed load returns a whole word.
pub struct Ext2Superblock {
    /// `e2fs` and `e2fs_fsmnt`: `struct ext2fs`'s `SBSIZE` bytes followed by the `MAXMNTLEN`
    /// of the name mounted on, as relaxed words of one `malloc(M_UFSMNT)` array (the C's
    /// `struct m_ext2fs`'s first members), read and written as `DinWords` reads the dinode.
    words: MArray<AtomicU64>,
    /// `e2fs_gd`: the group descriptors, `e2fs_ngdb * e2fs_bsize` bytes of words in one
    /// `malloc(M_UFSMNT)` array (none until the mount reads them), replaced by
    /// `ext2fs_reload` under the kernel lock.
    gd: ReadMostly<Option<MArray<AtomicU64>>>,
    /// `e2fs_ronly`: mounted read-only flag.
    pub e2fs_ronly: I8Word,
    /// `e2fs_fmod`: super block modified flag.
    pub e2fs_fmod: I8Word,
    /// `e2fs_fsize`: fragment size.
    pub e2fs_fsize: I32Word,
    /// `e2fs_bsize`: block size.
    pub e2fs_bsize: I32Word,
    /// `e2fs_bshift`: `lblkno` calc of logical blkno.
    pub e2fs_bshift: I32Word,
    /// `e2fs_bmask`: `blkoff` calc of blk offsets.
    pub e2fs_bmask: I32Word,
    /// `e2fs_qbmask`: `~fs_bmask`, for use with quad size.
    pub e2fs_qbmask: I64Word,
    /// `e2fs_fsbtodb`: `fsbtodb` and `dbtofsb` shift constant.
    pub e2fs_fsbtodb: I32Word,
    /// `e2fs_ncg`: number of cylinder groups.
    pub e2fs_ncg: I32Word,
    /// `e2fs_ngdb`: number of group descriptor blocks.
    pub e2fs_ngdb: I32Word,
    /// `e2fs_ipb`: number of inodes per block.
    pub e2fs_ipb: I32Word,
    /// `e2fs_itpg`: number of inode tables per group.
    pub e2fs_itpg: I32Word,
    /// `e2fs_maxfilesize`: depends on LARGE/HUGE flags.
    pub e2fs_maxfilesize: I64Word,
}

impl Ext2Superblock {
    /// `malloc(sizeof(struct m_ext2fs), M_UFSMNT, M_WAITOK | M_ZERO)` (`ext2fs_mountfs`): a
    /// zeroed super-block with no group descriptors. `M_WAITOK` sleeps for the memory and
    /// never fails, as the C's.
    pub fn new() -> Self {
        let Some(words) = MArray::new(M_UFSMNT, M_WAITOK, SB_WORDS, |_| AtomicU64::new(0)) else {
            kpanic(format_args!("ext2fs: malloc(M_UFSMNT) of m_ext2fs failed"));
        };
        Self {
            words,
            gd: ReadMostly::new(None),
            e2fs_ronly: I8Word::new(0),
            e2fs_fmod: I8Word::new(0),
            e2fs_fsize: I32Word::new(0),
            e2fs_bsize: I32Word::new(0),
            e2fs_bshift: I32Word::new(0),
            e2fs_bmask: I32Word::new(0),
            e2fs_qbmask: I64Word::new(0),
            e2fs_fsbtodb: I32Word::new(0),
            e2fs_ncg: I32Word::new(0),
            e2fs_ngdb: I32Word::new(0),
            e2fs_ipb: I32Word::new(0),
            e2fs_itpg: I32Word::new(0),
            e2fs_maxfilesize: I64Word::new(0),
        }
    }

    /// The member at byte `off` of `struct ext2fs` (an `offset_of!(Ext2fs, ..)`).
    #[inline]
    fn get<T: DinField>(&self, off: usize) -> T {
        words_get(&self.words, "m_ext2fs", off)
    }

    /// The member at byte `off` of `struct ext2fs` set to `v`, the word's other members kept.
    #[inline]
    fn set<T: DinField>(&self, off: usize, v: T) {
        words_set(&self.words, "m_ext2fs", off, v);
    }

    /// The bytes `off..off + out.len()` of `struct ext2fs` (its arrays: `e2fs_uuid`,
    /// `e2fs_vname`, `e2fs_fsmnt`, ...) into `out`; `panic(9)` past the structure.
    pub fn read_bytes(&self, off: usize, out: &mut [u8]) {
        if off + out.len() > SBSIZE {
            kpanic(format_args!("m_ext2fs: e2fs bytes {off}+{}", out.len()));
        }
        words_read_bytes(&self.words, "m_ext2fs", off, out);
    }

    /// The bytes `off..off + src.len()` of `struct ext2fs` set to `src`; `panic(9)` past the
    /// structure.
    pub fn write_bytes(&self, off: usize, src: &[u8]) {
        if off + src.len() > SBSIZE {
            kpanic(format_args!("m_ext2fs: e2fs bytes {off}+{}", src.len()));
        }
        words_write_bytes(&self.words, "m_ext2fs", off, src);
    }

    /// `fs->e2fs_fsmnt`: a copy of the name mounted on.
    pub fn fsmnt(&self) -> Fsmnt {
        let mut name = [0u8; MAXMNTLEN];
        words_read_bytes(&self.words, "m_ext2fs", SBSIZE, &mut name);
        Fsmnt(name)
    }

    /// `fs->e2fs_fsmnt` set to the first `MAXMNTLEN` bytes of `name` (the rest zero).
    pub fn set_fsmnt(&self, name: &[u8]) {
        let mut buf = [0u8; MAXMNTLEN];
        let n = name.len().min(MAXMNTLEN);
        buf[..n].copy_from_slice(&name[..n]);
        words_write_bytes(&self.words, "m_ext2fs", SBSIZE, &buf);
    }

    /// `EXT2_DINODE_SIZE(fs)`: the size of an on-disk inode.
    pub fn dinode_size(&self) -> usize {
        if self.e2fs_rev() > E2FS_REV0 {
            usize::from(self.e2fs_inode_size())
        } else {
            EXT2_REV0_DINODE_SIZE
        }
    }

    /// `fs->e2fs_gd = mallocarray(ngdb, bsize, M_UFSMNT, M_WAITOK)`: a new, zeroed array of
    /// `ngdb * bsize` bytes of group descriptors in place of the old one, which is freed
    /// (`ext2fs_reload`'s `free` of the old array), by the holder of the kernel lock.
    pub fn gd_alloc(&self, ngdb: usize, bsize: usize) {
        let n = (ngdb * bsize).div_ceil(8);
        let Some(a) = MArray::new(M_UFSMNT, M_WAITOK, n, |_| AtomicU64::new(0)) else {
            kpanic(format_args!("ext2fs: malloc(M_UFSMNT) of e2fs_gd failed"));
        };
        drop(self.gd.replace(Some(a)));
    }

    /// `free(fs->e2fs_gd, M_UFSMNT, ..)`: the descriptors freed (a failed mount, the unmount).
    pub fn gd_free(&self) {
        drop(self.gd.replace(None));
    }

    /// `f` over the descriptors' words (one read section of the slot); `panic(9)` before the
    /// mount read them (the C's NULL `e2fs_gd`).
    fn with_gd<R>(&self, f: impl FnOnce(&[AtomicU64]) -> R) -> R {
        let r = self.gd.read();
        match &*r {
            Some(a) => f(a),
            None => kpanic(format_args!("m_ext2fs: no group descriptors")),
        }
    }

    /// The byte offset of member `off` of descriptor `i`: `panic(9)` past `e2fs_ncg`, where
    /// the C would index past its array.
    #[inline]
    fn gd_off(&self, i: usize, off: usize) -> usize {
        let ncg = self.e2fs_ncg.get();
        if i >= ncg.max(0) as usize {
            kpanic(format_args!("m_ext2fs: group descriptor {i} of {ncg}"));
        }
        i * EXT2_GD_SIZE + off
    }

    /// `fs->e2fs_gd[i]`, copied (four word loads).
    pub fn gd(&self, i: usize) -> Ext2Gd {
        let at = self.gd_off(i, 0);
        let mut b = [0u8; EXT2_GD_SIZE];
        self.with_gd(|w| words_read_bytes(w, "e2fs_gd", at, &mut b));
        pod_from_bytes(&b).unwrap_or_default()
    }

    /// The member at `off` of descriptor `i` (one word load).
    #[inline]
    fn gd_get<T: DinField>(&self, i: usize, off: usize) -> T {
        let at = self.gd_off(i, off);
        self.with_gd(|w| words_get(w, "e2fs_gd", at))
    }

    /// The member at `off` of descriptor `i` set to `v`, by the holder of the kernel lock.
    #[inline]
    fn gd_set<T: DinField>(&self, i: usize, off: usize, v: T) {
        let at = self.gd_off(i, off);
        self.with_gd(|w| words_set(w, "e2fs_gd", at, v));
    }

    /// `fs->e2fs_gd[i].ext2bgd_b_bitmap`.
    pub fn gd_b_bitmap(&self, i: usize) -> u32 {
        self.gd_get(i, offset_of!(Ext2Gd, ext2bgd_b_bitmap))
    }

    /// `fs->e2fs_gd[i].ext2bgd_i_bitmap`.
    pub fn gd_i_bitmap(&self, i: usize) -> u32 {
        self.gd_get(i, offset_of!(Ext2Gd, ext2bgd_i_bitmap))
    }

    /// `fs->e2fs_gd[i].ext2bgd_i_tables`.
    pub fn gd_i_tables(&self, i: usize) -> u32 {
        self.gd_get(i, offset_of!(Ext2Gd, ext2bgd_i_tables))
    }

    /// `fs->e2fs_gd[i].ext2bgd_nbfree`.
    pub fn gd_nbfree(&self, i: usize) -> u16 {
        self.gd_get(i, offset_of!(Ext2Gd, ext2bgd_nbfree))
    }

    /// `fs->e2fs_gd[i].ext2bgd_nbfree = v`.
    pub fn set_gd_nbfree(&self, i: usize, v: u16) {
        self.gd_set(i, offset_of!(Ext2Gd, ext2bgd_nbfree), v);
    }

    /// `fs->e2fs_gd[i].ext2bgd_nifree`.
    pub fn gd_nifree(&self, i: usize) -> u16 {
        self.gd_get(i, offset_of!(Ext2Gd, ext2bgd_nifree))
    }

    /// `fs->e2fs_gd[i].ext2bgd_nifree = v`.
    pub fn set_gd_nifree(&self, i: usize, v: u16) {
        self.gd_set(i, offset_of!(Ext2Gd, ext2bgd_nifree), v);
    }

    /// `fs->e2fs_gd[i].ext2bgd_ndirs`.
    pub fn gd_ndirs(&self, i: usize) -> u16 {
        self.gd_get(i, offset_of!(Ext2Gd, ext2bgd_ndirs))
    }

    /// `fs->e2fs_gd[i].ext2bgd_ndirs = v`.
    pub fn set_gd_ndirs(&self, i: usize, v: u16) {
        self.gd_set(i, offset_of!(Ext2Gd, ext2bgd_ndirs), v);
    }
}

impl Default for Ext2Superblock {
    fn default() -> Self {
        Self::new()
    }
}

/// `fs->e2fs_fsmnt` as `%s` prints it: the bytes up to the first NUL, or all `MAXMNTLEN` of
/// them when there is none.
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

/// Generates the accessors of the in-core `struct ext2fs`'s members: `$get` reads
/// `fs->e2fs.member` at `offset_of!(Ext2fs, member)` (a load and a shift of its word), `$set`
/// writes it (a load and a store, by a holder of the kernel lock).
macro_rules! e2fs_field {
    ($($get:ident, $set:ident: $t:ty;)+) => {
        impl Ext2Superblock {
            $(
                #[doc = concat!("`fs->e2fs.", stringify!($get), "`.")]
                #[inline]
                pub fn $get(&self) -> $t {
                    self.get::<$t>(offset_of!(Ext2fs, $get))
                }

                #[doc = concat!("`fs->e2fs.", stringify!($get), " = v`.")]
                #[inline]
                pub fn $set(&self, v: $t) {
                    self.set::<$t>(offset_of!(Ext2fs, $get), v);
                }
            )+
        }
    };
}

e2fs_field! {
    e2fs_icount, set_e2fs_icount: u32;
    e2fs_bcount, set_e2fs_bcount: u32;
    e2fs_rbcount, set_e2fs_rbcount: u32;
    e2fs_fbcount, set_e2fs_fbcount: u32;
    e2fs_ficount, set_e2fs_ficount: u32;
    e2fs_first_dblock, set_e2fs_first_dblock: u32;
    e2fs_log_bsize, set_e2fs_log_bsize: u32;
    e2fs_log_fsize, set_e2fs_log_fsize: u32;
    e2fs_bpg, set_e2fs_bpg: u32;
    e2fs_fpg, set_e2fs_fpg: u32;
    e2fs_ipg, set_e2fs_ipg: u32;
    e2fs_mtime, set_e2fs_mtime: u32;
    e2fs_wtime, set_e2fs_wtime: u32;
    e2fs_mnt_count, set_e2fs_mnt_count: u16;
    e2fs_max_mnt_count, set_e2fs_max_mnt_count: u16;
    e2fs_magic, set_e2fs_magic: u16;
    e2fs_state, set_e2fs_state: u16;
    e2fs_beh, set_e2fs_beh: u16;
    e2fs_minrev, set_e2fs_minrev: u16;
    e2fs_lastfsck, set_e2fs_lastfsck: u32;
    e2fs_fsckintv, set_e2fs_fsckintv: u32;
    e2fs_creator, set_e2fs_creator: u32;
    e2fs_rev, set_e2fs_rev: u32;
    e2fs_ruid, set_e2fs_ruid: u16;
    e2fs_rgid, set_e2fs_rgid: u16;
    e2fs_first_ino, set_e2fs_first_ino: u32;
    e2fs_inode_size, set_e2fs_inode_size: u16;
    e2fs_block_group_nr, set_e2fs_block_group_nr: u16;
    e2fs_features_compat, set_e2fs_features_compat: u32;
    e2fs_features_incompat, set_e2fs_features_incompat: u32;
    e2fs_features_rocompat, set_e2fs_features_rocompat: u32;
    e2fs_reserved_ngdb, set_e2fs_reserved_ngdb: u16;
    e2fs_gdesc_size, set_e2fs_gdesc_size: u16;
    e2fs_first_meta_bg, set_e2fs_first_meta_bg: u32;
    e2fs_flags, set_e2fs_flags: u32;
}

/// `e2fs_overflow(fs, lower, value)`: whether `value` is outside `lower..=e2fs_maxfilesize`.
pub fn e2fs_overflow(fs: &Ext2Superblock, lower: Off, value: Off) -> bool {
    value < lower || value > fs.e2fs_maxfilesize.get()
}

/// `cg_has_sb(i)`: if the `EXT2F_ROCOMPAT_SPARSE_SUPER` flag is set, the cylinder group has a
/// copy of the super and cylinder group descriptors blocks only if it is a power of 3, 5 or 7.
pub fn cg_has_sb(i: i32) -> bool {
    if i == 0 || i == 1 {
        return true;
    }
    let i = i64::from(i);
    let (mut a3, mut a5, mut a7) = (3i64, 5i64, 7i64);
    while a3 <= i || a5 <= i || a7 <= i {
        if i == a3 || i == a5 || i == a7 {
            return true;
        }
        a3 *= 3;
        a5 *= 5;
        a7 *= 7;
    }
    false
}

/// `e2fs_sbload(old, new)`: loads the super block from the first `SBSIZE` bytes of `old`
/// (little-endian on disk) into the in-core `new`: the C's `memcpy` on a little-endian
/// machine, `e2fs_sb_bswap` on a big-endian one.
pub fn e2fs_sbload(old: &[u8], new: &Ext2Superblock) {
    let Some(src) = old.get(..SBSIZE) else {
        kpanic(format_args!(
            "e2fs_sbload: {} bytes, want {}",
            old.len(),
            SBSIZE
        ));
    };
    if cfg!(target_endian = "big") {
        let mut b = [0u8; SBSIZE];
        let _ = pod_to_bytes(&Ext2fs::from_disk(src), &mut b);
        new.write_bytes(0, &b);
    } else {
        new.write_bytes(0, src);
    }
}

/// `e2fs_sbsave(old, new)`: stores the in-core super block `old` in the first `SBSIZE`
/// bytes of `new` (little-endian on disk).
pub fn e2fs_sbsave(old: &Ext2Superblock, new: &mut [u8]) {
    let Some(dst) = new.get_mut(..SBSIZE) else {
        kpanic(format_args!(
            "e2fs_sbsave: {} bytes, want {}",
            new.len(),
            SBSIZE
        ));
    };
    old.read_bytes(0, dst);
    if cfg!(target_endian = "big") {
        let sb = pod_from_bytes::<Ext2fs>(dst).unwrap_or_default();
        sb.to_disk(dst);
    }
}

/// `e2fs_cgload(old, &fs->e2fs_gd[gdesc], size)`: loads `size` bytes of group descriptors
/// (little-endian on disk) from `old` into the in-core descriptors from number `gdesc` on:
/// the bytes as they are on a little-endian machine (the C's `memcpy`), `e2fs_cg_bswap` of the
/// whole descriptors on a big-endian one. `panic(9)` past `old` or the descriptors' array.
pub fn e2fs_cgload(old: &[u8], fs: &Ext2Superblock, gdesc: usize, size: usize) {
    let Some(src) = old.get(..size) else {
        kpanic(format_args!("e2fs_cgload: {size} bytes from {}", old.len()));
    };
    let at = gdesc * EXT2_GD_SIZE;
    if cfg!(target_endian = "big") {
        for (k, d) in src.as_chunks::<EXT2_GD_SIZE>().0.iter().enumerate() {
            let disk = [pod_from_bytes::<Ext2Gd>(d).unwrap_or_default()];
            let mut gd = [Ext2Gd::default()];
            e2fs_cg_bswap(&disk, &mut gd, EXT2_GD_SIZE);
            let mut b = [0u8; EXT2_GD_SIZE];
            let _ = pod_to_bytes(&gd[0], &mut b);
            fs.with_gd(|w| words_write_bytes(w, "e2fs_gd", at + k * EXT2_GD_SIZE, &b));
        }
    } else {
        fs.with_gd(|w| words_write_bytes(w, "e2fs_gd", at, src));
    }
}

/// `e2fs_cgsave(&fs->e2fs_gd[gdesc], new, size)`: stores `size` bytes of the in-core group
/// descriptors from number `gdesc` on into `new` (little-endian on disk).
pub fn e2fs_cgsave(fs: &Ext2Superblock, gdesc: usize, new: &mut [u8], size: usize) {
    let Some(dst) = new.get_mut(..size) else {
        kpanic(format_args!("e2fs_cgsave: {size} bytes into {}", new.len()));
    };
    let at = gdesc * EXT2_GD_SIZE;
    fs.with_gd(|w| words_read_bytes(w, "e2fs_gd", at, dst));
    if cfg!(target_endian = "big") {
        for d in dst.as_chunks_mut::<EXT2_GD_SIZE>().0 {
            let gd = [pod_from_bytes::<Ext2Gd>(d).unwrap_or_default()];
            let mut disk = [Ext2Gd::default()];
            e2fs_cg_bswap(&gd, &mut disk, EXT2_GD_SIZE);
            let _ = pod_to_bytes(&disk[0], d);
        }
    }
}

/// `fsbtodb(fs, b)`: turns a file system block number into a disk block address (maps file
/// system blocks to device size blocks).
#[inline]
pub fn fsbtodb(fs: &Ext2Superblock, b: Daddr) -> Daddr {
    b << fs.e2fs_fsbtodb.get()
}

/// `dbtofsb(fs, b)`: the inverse of [`fsbtodb`].
#[inline]
pub fn dbtofsb(fs: &Ext2Superblock, b: Daddr) -> Daddr {
    b >> fs.e2fs_fsbtodb.get()
}

/// `ino_to_cg(fs, x)`: inode number to cylinder group number.
#[inline]
pub fn ino_to_cg(fs: &Ext2Superblock, x: Ufsino) -> u32 {
    x.wrapping_sub(1) / fs.e2fs_ipg()
}

/// `ino_to_fsba(fs, x)`: inode number to file system block address.
#[inline]
pub fn ino_to_fsba(fs: &Ext2Superblock, x: Ufsino) -> u32 {
    fs.gd_i_tables(ino_to_cg(fs, x) as usize)
        .wrapping_add((x.wrapping_sub(1) % fs.e2fs_ipg()) / fs.e2fs_ipb.get() as u32)
}

/// `ino_to_fsbo(fs, x)`: inode number to file system block offset.
#[inline]
pub fn ino_to_fsbo(fs: &Ext2Superblock, x: Ufsino) -> u32 {
    x.wrapping_sub(1) % fs.e2fs_ipb.get() as u32
}

/// `dtog(fs, d)`: the cylinder group number for a file system block.
#[inline]
pub fn dtog(fs: &Ext2Superblock, d: u32) -> u32 {
    d.wrapping_sub(fs.e2fs_first_dblock()) / fs.e2fs_fpg()
}

/// `dtogd(fs, d)`: the cylinder group block number for a file system block.
#[inline]
pub fn dtogd(fs: &Ext2Superblock, d: u32) -> u32 {
    d.wrapping_sub(fs.e2fs_first_dblock()) % fs.e2fs_fpg()
}

/// `blkoff(fs, loc)`: calculates `loc % fs->e2fs_bsize`.
#[inline]
pub fn blkoff(fs: &Ext2Superblock, loc: Off) -> Off {
    loc & fs.e2fs_qbmask.get()
}

/// `lblktosize(fs, blk)`: calculates `blk * fs->e2fs_bsize`.
#[inline]
pub fn lblktosize(fs: &Ext2Superblock, blk: Daddr) -> Off {
    blk << fs.e2fs_bshift.get()
}

/// `lblkno(fs, loc)`: calculates `loc / fs->e2fs_bsize`.
#[inline]
pub fn lblkno(fs: &Ext2Superblock, loc: Off) -> Daddr {
    loc >> fs.e2fs_bshift.get()
}

/// `blkroundup(fs, size)`: calculates `roundup(size, fs->e2fs_bsize)`.
#[inline]
pub fn blkroundup(fs: &Ext2Superblock, size: Off) -> Off {
    (size + fs.e2fs_qbmask.get()) & i64::from(fs.e2fs_bmask.get())
}

/// `fragroundup(fs, size)`: calculates `roundup(size, fs->e2fs_bsize)` (as the C does: there
/// are no fragments in ext2fs).
#[inline]
pub fn fragroundup(fs: &Ext2Superblock, size: Off) -> Off {
    (size + fs.e2fs_qbmask.get()) & i64::from(fs.e2fs_bmask.get())
}

/// `freespace(fs)`: the number of available blocks given the ones held in reserve.
#[inline]
pub fn freespace(fs: &Ext2Superblock) -> u32 {
    fs.e2fs_fbcount().wrapping_sub(fs.e2fs_rbcount())
}

/// `NINDIR(fs)`: the number of indirects in a file system block.
#[inline]
pub fn nindir(fs: &Ext2Superblock) -> i64 {
    i64::from(fs.e2fs_bsize.get()) / size_of::<u32>() as i64
}

const _: () = {
    assert!(size_of::<Ext2fs>() == SBSIZE);
    assert!(size_of::<Ext2Gd>() == 32);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_magic) == 56);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_rev) == 76);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_features_compat) == 92);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_uuid) == 104);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_journal_uuid) == 208);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_bcount_hi) == 336);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_mmpblk) == 360);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_kbytes_written) == 376);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_chksum_seed) == 624);
    assert!(core::mem::offset_of!(Ext2fs, e2fs_sbchksum) == 1020);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::{assert, assert_eq};

    use super::*;
    use crate::testutil;

    /// A super-block of 3 groups of 100 blocks (from block 1) and 8 inodes, 1 KiB blocks of 4
    /// inodes, 50 free blocks of which 5 are reserved; the descriptors' array allocated.
    fn fs() -> Ext2Superblock {
        testutil::install();
        let fs = Ext2Superblock::new();
        fs.set_e2fs_ipg(8);
        fs.set_e2fs_fpg(100);
        fs.set_e2fs_first_dblock(1);
        fs.set_e2fs_fbcount(50);
        fs.set_e2fs_rbcount(5);
        fs.e2fs_ipb.set(4);
        fs.e2fs_bsize.set(1024);
        fs.e2fs_bshift.set(10);
        fs.e2fs_bmask.set(!1023);
        fs.e2fs_qbmask.set(1023);
        fs.e2fs_fsbtodb.set(1);
        fs.e2fs_ncg.set(3);
        fs.e2fs_ngdb.set(1);
        fs.gd_alloc(1, 1024);
        fs
    }

    /// The disk bytes of descriptors `gds`, a block of `size` bytes.
    fn gd_bytes(gds: &[Ext2Gd], size: usize) -> std::vec::Vec<u8> {
        let mut b = std::vec![0u8; size];
        for (i, g) in gds.iter().enumerate() {
            assert!(pod_to_bytes(
                g,
                &mut b[i * EXT2_GD_SIZE..(i + 1) * EXT2_GD_SIZE]
            ));
        }
        b
    }

    #[test]
    fn sector_and_block_arithmetic() {
        let fs = fs();
        assert_eq!(fsbtodb(&fs, 7), 14);
        assert_eq!(dbtofsb(&fs, 14), 7);
        assert_eq!(lblktosize(&fs, 3), 3072);
        assert_eq!(lblkno(&fs, 3073), 3);
        assert_eq!(blkoff(&fs, 3073), 1);
        assert_eq!(blkroundup(&fs, 1025), 2048);
        assert_eq!(blkroundup(&fs, 1024), 1024);
        assert_eq!(fragroundup(&fs, 1), 1024);
        assert_eq!(freespace(&fs), 45);
        assert_eq!(nindir(&fs), 256);
        assert_eq!(dtog(&fs, 101), 1);
        assert_eq!(dtogd(&fs, 101), 0);
        assert_eq!(dtog(&fs, 100), 0);
        fs.e2fs_maxfilesize.set(10);
        assert!(!e2fs_overflow(&fs, 0, 5));
        assert!(e2fs_overflow(&fs, 0, 11));
        assert!(e2fs_overflow(&fs, 0, -1));
    }

    #[test]
    fn inode_numbers_map_to_groups_and_blocks() {
        let fs = fs();
        let mut gds = [Ext2Gd::default(); 3];
        gds[0].ext2bgd_i_tables = 10;
        gds[1].ext2bgd_i_tables = 20;
        gds[2].ext2bgd_i_tables = 30;
        e2fs_cgload(&gd_bytes(&gds, 1024), &fs, 0, 1024);
        // Inode 1 is the first of group 0; inode 9 the first of group 1; inode 20 the 4th
        // of group 2 (index 3, in the second inode block: ipb is 4).
        assert_eq!(ino_to_cg(&fs, 1), 0);
        assert_eq!(ino_to_cg(&fs, 8), 0);
        assert_eq!(ino_to_cg(&fs, 9), 1);
        assert_eq!(ino_to_fsba(&fs, 9), 20);
        assert_eq!(ino_to_fsba(&fs, 13), 21);
        assert_eq!(ino_to_cg(&fs, 20), 2);
        assert_eq!(ino_to_fsba(&fs, 20), 30);
        assert_eq!(ino_to_fsbo(&fs, 20), 3);
        fs.set_gd_nbfree(1, 9);
        assert_eq!(fs.gd(1).ext2bgd_nbfree, 9);
        assert_eq!(fs.gd(1).ext2bgd_i_tables, 20);
    }

    #[test]
    fn sparse_super_groups() {
        let has: [i32; 12] = [0, 1, 3, 5, 7, 9, 25, 27, 49, 81, 125, 243];
        for i in 0..250 {
            assert_eq!(cg_has_sb(i), has.contains(&i) || i == 343, "{i}");
        }
        assert!(cg_has_sb(343));
        assert!(!cg_has_sb(2));
        assert!(!cg_has_sb(15));
    }

    #[test]
    fn super_block_load_save_round_trip() {
        let mut img = [0u8; SBSIZE];
        img[0..4].copy_from_slice(&1234u32.to_le_bytes());
        img[56..58].copy_from_slice(&E2FS_MAGIC.to_le_bytes());
        img[76..80].copy_from_slice(&E2FS_REV1.to_le_bytes());
        img[104..120].copy_from_slice(&[0xa5; 16]);
        img[1020..1024].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        let sb = Ext2fs::from_disk(&img);
        assert_eq!(sb.e2fs_icount, 1234);
        assert_eq!(sb.e2fs_magic, E2FS_MAGIC);
        assert_eq!(sb.e2fs_rev, E2FS_REV1);
        assert_eq!(sb.e2fs_sbchksum, 0xdead_beef);
        let mut out = [0u8; SBSIZE];
        sb.to_disk(&mut out);
        assert_eq!(out, img);

        // In core: every byte back as it came, the name mounted on untouched.
        let fs = fs();
        fs.set_fsmnt(b"/mnt\0");
        e2fs_sbload(&img, &fs);
        assert_eq!((fs.e2fs_icount(), fs.e2fs_magic()), (1234, E2FS_MAGIC));
        let mut uuid = [0u8; 16];
        fs.read_bytes(offset_of!(Ext2fs, e2fs_uuid), &mut uuid);
        assert_eq!(uuid, [0xa5; 16]);
        let mut out = [0xffu8; SBSIZE + 8];
        e2fs_sbsave(&fs, &mut out);
        assert_eq!(out[..SBSIZE], img);
        assert_eq!(out[SBSIZE..], [0xff; 8], "nothing past SBSIZE");
        assert_eq!(fs.fsmnt().as_bytes(), b"/mnt");
    }

    #[test]
    fn the_accessors_read_and_write_each_member_at_its_offset() {
        let fs = fs();
        let mut want = Ext2fs::new();
        macro_rules! check {
            ($($get:ident, $set:ident, $v:expr;)+) => {
                $(
                    fs.$set($v);
                    want.$get = $v;
                )+
                let mut b = [0u8; SBSIZE];
                e2fs_sbsave(&fs, &mut b);
                let got = Ext2fs::from_disk(&b);
                $(assert_eq!(got.$get, want.$get, stringify!($get));)+
                $(assert_eq!(fs.$get(), $v, stringify!($get));)+
            };
        }
        check! {
            e2fs_icount, set_e2fs_icount, 0x0102_0304;
            e2fs_bcount, set_e2fs_bcount, 0x1112_1314;
            e2fs_rbcount, set_e2fs_rbcount, 3;
            e2fs_fbcount, set_e2fs_fbcount, 4;
            e2fs_ficount, set_e2fs_ficount, 5;
            e2fs_first_dblock, set_e2fs_first_dblock, 6;
            e2fs_log_bsize, set_e2fs_log_bsize, 7;
            e2fs_log_fsize, set_e2fs_log_fsize, 8;
            e2fs_bpg, set_e2fs_bpg, 9;
            e2fs_fpg, set_e2fs_fpg, 10;
            e2fs_ipg, set_e2fs_ipg, 11;
            e2fs_mtime, set_e2fs_mtime, 12;
            e2fs_wtime, set_e2fs_wtime, 13;
            e2fs_mnt_count, set_e2fs_mnt_count, 0x0e0f;
            e2fs_max_mnt_count, set_e2fs_max_mnt_count, 0x1011;
            e2fs_magic, set_e2fs_magic, E2FS_MAGIC;
            e2fs_state, set_e2fs_state, 0x1213;
            e2fs_beh, set_e2fs_beh, 0x1415;
            e2fs_minrev, set_e2fs_minrev, 0x1617;
            e2fs_lastfsck, set_e2fs_lastfsck, 18;
            e2fs_fsckintv, set_e2fs_fsckintv, 19;
            e2fs_creator, set_e2fs_creator, 20;
            e2fs_rev, set_e2fs_rev, 21;
            e2fs_ruid, set_e2fs_ruid, 0x1617;
            e2fs_rgid, set_e2fs_rgid, 0x1819;
            e2fs_first_ino, set_e2fs_first_ino, 24;
            e2fs_inode_size, set_e2fs_inode_size, 0x1a1b;
            e2fs_block_group_nr, set_e2fs_block_group_nr, 0x1c1d;
            e2fs_features_compat, set_e2fs_features_compat, 27;
            e2fs_features_incompat, set_e2fs_features_incompat, 28;
            e2fs_features_rocompat, set_e2fs_features_rocompat, 29;
            e2fs_reserved_ngdb, set_e2fs_reserved_ngdb, 0x1e1f;
            e2fs_gdesc_size, set_e2fs_gdesc_size, 0x2021;
            e2fs_first_meta_bg, set_e2fs_first_meta_bg, 32;
            e2fs_flags, set_e2fs_flags, 33;
        }
        assert_eq!(
            fs.dinode_size(),
            0x1a1b,
            "a revision above 0 names its inode size"
        );
        fs.set_e2fs_rev(E2FS_REV0);
        assert_eq!(fs.dinode_size(), 128);
    }

    #[test]
    fn group_descriptors_load_save_round_trip() {
        let fs = fs();
        let mut img = [0u8; 1024];
        for (i, b) in img.iter_mut().enumerate() {
            *b = i as u8;
        }
        e2fs_cgload(&img, &fs, 0, 1024);
        let gd = fs.gd(0);
        assert_eq!(gd.ext2bgd_b_bitmap, 0x0302_0100);
        assert_eq!(gd.ext2bgd_nbfree, 0x0d0c);
        assert_eq!(fs.gd(1).ext2bgd_i_tables, 0x2b2a_2928);
        assert_eq!(fs.gd_i_tables(1), 0x2b2a_2928);
        assert_eq!(
            (
                fs.gd_b_bitmap(2),
                fs.gd_i_bitmap(2),
                fs.gd_nifree(2),
                fs.gd_ndirs(2)
            ),
            (0x4342_4140, 0x4746_4544, 0x4f4e, 0x5150)
        );
        let mut out = [0u8; 1024];
        e2fs_cgsave(&fs, 0, &mut out, 1024);
        assert_eq!(out, img);

        // One member changed, its neighbours kept, the bytes saved as the disk holds them.
        fs.set_gd_nifree(1, 0xbeef);
        fs.set_gd_ndirs(1, 7);
        e2fs_cgsave(&fs, 0, &mut out, 1024);
        assert_eq!(out[32 + 14..32 + 18], [0xef, 0xbe, 7, 0]);
        assert_eq!(out[32 + 12..32 + 14], img[32 + 12..32 + 14]);
        assert_eq!(out[32 + 18..], img[32 + 18..]);

        // A second block of descriptors goes after the first block's.
        fs.gd_alloc(2, 1024);
        fs.e2fs_ncg.set(40);
        e2fs_cgload(&img, &fs, 32, 1024);
        assert_eq!(fs.gd(33).ext2bgd_i_tables, 0x2b2a_2928);
        assert_eq!(fs.gd(31), Ext2Gd::default());
    }

    #[test]
    #[should_panic(expected = "group descriptor 3 of 3")]
    fn a_descriptor_past_the_groups_panics() {
        let fs = fs();
        let _ = fs.gd(3);
    }

    #[test]
    fn feature_tables_name_every_flag() {
        assert_eq!(RO_COMPAT.iter().fold(0, |m, f| m | f.mask), 0x377f);
        assert_eq!(INCOMPAT.iter().fold(0, |m, f| m | f.mask), 0x1f7df);
        assert_eq!(EXT2F_ROCOMPAT_SUPP, 3);
        assert_eq!(EXT2F_INCOMPAT_SUPP, 2);
        assert_eq!(EXT4F_RO_INCOMPAT_SUPP, 0x254);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ext2fs/ext2fs.h");
        for (name, value) in [
            ("BBSIZE", BBSIZE as i64),
            ("SBSIZE", SBSIZE as i64),
            ("LOG_MINBSIZE", i64::from(LOG_MINBSIZE)),
            ("MINBSIZE", MINBSIZE as i64),
            ("LOG_MINFSIZE", i64::from(LOG_MINFSIZE)),
            ("MINFSIZE", MINFSIZE as i64),
            ("MAXMNTLEN", MAXMNTLEN as i64),
            ("MINFREE", i64::from(MINFREE)),
            ("E2FS_MAGIC", i64::from(E2FS_MAGIC)),
            ("E2FS_REV0", i64::from(E2FS_REV0)),
            ("E2FS_REV1", i64::from(E2FS_REV1)),
            ("EXT2F_COMPAT_PREALLOC", i64::from(EXT2F_COMPAT_PREALLOC)),
            (
                "EXT2F_COMPAT_IMAGIC_INODES",
                i64::from(EXT2F_COMPAT_IMAGIC_INODES),
            ),
            (
                "EXT2F_COMPAT_HAS_JOURNAL",
                i64::from(EXT2F_COMPAT_HAS_JOURNAL),
            ),
            ("EXT2F_COMPAT_EXT_ATTR", i64::from(EXT2F_COMPAT_EXT_ATTR)),
            ("EXT2F_COMPAT_RESIZE", i64::from(EXT2F_COMPAT_RESIZE)),
            ("EXT2F_COMPAT_DIR_INDEX", i64::from(EXT2F_COMPAT_DIR_INDEX)),
            (
                "EXT2F_COMPAT_SPARSE_SUPER2",
                i64::from(EXT2F_COMPAT_SPARSE_SUPER2),
            ),
            (
                "EXT2F_ROCOMPAT_SPARSE_SUPER",
                i64::from(EXT2F_ROCOMPAT_SPARSE_SUPER),
            ),
            (
                "EXT2F_ROCOMPAT_LARGE_FILE",
                i64::from(EXT2F_ROCOMPAT_LARGE_FILE),
            ),
            (
                "EXT2F_ROCOMPAT_BTREE_DIR",
                i64::from(EXT2F_ROCOMPAT_BTREE_DIR),
            ),
            (
                "EXT2F_ROCOMPAT_HUGE_FILE",
                i64::from(EXT2F_ROCOMPAT_HUGE_FILE),
            ),
            (
                "EXT2F_ROCOMPAT_GDT_CSUM",
                i64::from(EXT2F_ROCOMPAT_GDT_CSUM),
            ),
            (
                "EXT2F_ROCOMPAT_DIR_NLINK",
                i64::from(EXT2F_ROCOMPAT_DIR_NLINK),
            ),
            (
                "EXT2F_ROCOMPAT_EXTRA_ISIZE",
                i64::from(EXT2F_ROCOMPAT_EXTRA_ISIZE),
            ),
            ("EXT2F_ROCOMPAT_QUOTA", i64::from(EXT2F_ROCOMPAT_QUOTA)),
            (
                "EXT2F_ROCOMPAT_BIGALLOC",
                i64::from(EXT2F_ROCOMPAT_BIGALLOC),
            ),
            (
                "EXT2F_ROCOMPAT_METADATA_CKSUM",
                i64::from(EXT2F_ROCOMPAT_METADATA_CKSUM),
            ),
            (
                "EXT2F_ROCOMPAT_READONLY",
                i64::from(EXT2F_ROCOMPAT_READONLY),
            ),
            ("EXT2F_ROCOMPAT_PROJECT", i64::from(EXT2F_ROCOMPAT_PROJECT)),
            ("EXT2F_INCOMPAT_COMP", i64::from(EXT2F_INCOMPAT_COMP)),
            ("EXT2F_INCOMPAT_FTYPE", i64::from(EXT2F_INCOMPAT_FTYPE)),
            ("EXT2F_INCOMPAT_RECOVER", i64::from(EXT2F_INCOMPAT_RECOVER)),
            (
                "EXT2F_INCOMPAT_JOURNAL_DEV",
                i64::from(EXT2F_INCOMPAT_JOURNAL_DEV),
            ),
            ("EXT2F_INCOMPAT_META_BG", i64::from(EXT2F_INCOMPAT_META_BG)),
            ("EXT2F_INCOMPAT_EXTENTS", i64::from(EXT2F_INCOMPAT_EXTENTS)),
            ("EXT2F_INCOMPAT_64BIT", i64::from(EXT2F_INCOMPAT_64BIT)),
            ("EXT2F_INCOMPAT_MMP", i64::from(EXT2F_INCOMPAT_MMP)),
            ("EXT2F_INCOMPAT_FLEX_BG", i64::from(EXT2F_INCOMPAT_FLEX_BG)),
            (
                "EXT2F_INCOMPAT_EA_INODE",
                i64::from(EXT2F_INCOMPAT_EA_INODE),
            ),
            ("EXT2F_INCOMPAT_DIRDATA", i64::from(EXT2F_INCOMPAT_DIRDATA)),
            (
                "EXT2F_INCOMPAT_CSUM_SEED",
                i64::from(EXT2F_INCOMPAT_CSUM_SEED),
            ),
            (
                "EXT2F_INCOMPAT_LARGEDIR",
                i64::from(EXT2F_INCOMPAT_LARGEDIR),
            ),
            (
                "EXT2F_INCOMPAT_INLINE_DATA",
                i64::from(EXT2F_INCOMPAT_INLINE_DATA),
            ),
            ("EXT2F_INCOMPAT_ENCRYPT", i64::from(EXT2F_INCOMPAT_ENCRYPT)),
            ("EXT2F_COMPAT_SUPP", i64::from(EXT2F_COMPAT_SUPP)),
            ("EXT2F_INCOMPAT_SUPP", i64::from(EXT2F_INCOMPAT_SUPP)),
            ("E2FS_BEH_CONTINUE", i64::from(E2FS_BEH_CONTINUE)),
            ("E2FS_BEH_READONLY", i64::from(E2FS_BEH_READONLY)),
            ("E2FS_BEH_PANIC", i64::from(E2FS_BEH_PANIC)),
            ("E2FS_BEH_DEFAULT", i64::from(E2FS_BEH_DEFAULT)),
            ("E2FS_OS_LINUX", i64::from(E2FS_OS_LINUX)),
            ("E2FS_OS_HURD", i64::from(E2FS_OS_HURD)),
            ("E2FS_OS_MASIX", i64::from(E2FS_OS_MASIX)),
            ("E2FS_ISCLEAN", i64::from(E2FS_ISCLEAN)),
            ("E2FS_ERRORS", i64::from(E2FS_ERRORS)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
    }
}
/* </TESTS> */
