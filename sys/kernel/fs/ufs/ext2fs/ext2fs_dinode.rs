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
 * Copyright (c) 1982, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)dinode.h	8.6 (Berkeley) 9/13/94
 *  Modified for ext2fs by Manuel Bouyer.
 */

/*
 * Copyright (c) 1982, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)inode.h	8.5 (Berkeley) 7/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ext2fs/ext2fs_dinode.h>`: the on-disk inode of an ext2fs (`struct ext2fs_dinode`), its
//! file types, permissions and flags; and the inode's ext2fs state in core ([`Ext2Inode`]:
//! the dinode and `<ufs/ufs/inode.h>`'s `inode_ext.e2fs`).
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_dinode.h @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_dinode.rs@3c62ede68802
//! Upstream: sys/ufs/ufs/inode.h @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/inode.rs@3c62ede68802
//!
//! A dinode contains all the meta-data associated with a UFS file. This structure defines the
//! on-disk format of a dinode. Since this structure describes an on-disk structure, all its
//! fields are defined by types with precise widths.
//!
//! ## Deviations
//! - `EXT2_DINODE_SIZE(fs)` is the in-core super-block's `dinode_size()`
//!   (`ext2fs.rs`'s `Ext2Superblock`); `e2fs_iload`/`e2fs_isave` are functions
//!   that are always compiled: they copy `MIN(EXT2_DINODE_SIZE(fs), sizeof(*new))` bytes on a
//!   little-endian machine and go through `e2fs_i_bswap` (`ext2fs_bswap.rs`) on a big-endian
//!   one, where the C picks one with `#if BYTE_ORDER`. They take the buffer's bytes, not a
//!   `struct ext2fs_dinode *` into it.
//! - `NDADDR` and `NIADDR` (also defined by `ufs/ufs/dinode.h`) are re-exported from there.
//! - An inode's dinode is an item of `ext2dinopl` of about 200 bytes where the C's is
//!   `sizeof(struct ext2fs_dinode)`, 156: the item holds `inode_ext.e2fs` too (the C's
//!   union member of `struct inode`), and the dinode's 156 bytes in 20 words. `vmstat -m`'s
//!   and `systat pool`'s Size column for the pool shows it.
//!
//! ## Redesign
//! - LZ's in-core dinode, a raw pointer `i_e2din` to a `pool_get` item (`bsd`'s glue member,
//!   read and written through `with_e2din`'s `unsafe` `&mut`), and `inode_ext.e2fs` as five
//!   `Cell`s of the glue -> [`Ext2Inode`] (D-N5.6-2), one item of `ext2dinopl` set once by
//!   `ext2fs_vget` into the inode's dinode slot (`Dinode::Ext2`, `ufs/inode.rs`) and given
//!   back with the inode's last handle: the dinode as 20 relaxed words (`DinWords`, the FFS
//!   dinode's idiom), each `i_e2fs_*` macro of `<ufs/ufs/inode.h>` an accessor at
//!   `offset_of!(Ext2fsDinode, e2di_x)` (every member naturally aligned and at most four
//!   bytes, so none crosses a word), the in-core members relaxed words. Whole copies are
//!   bytes: `e2fs_iload`/`e2fs_isave` write and read `min(EXT2_DINODE_SIZE, 156)` of them
//!   (`DinWords::load_pod` wants a whole number of words: 156 is not one, and padding the
//!   structure would write four bytes of the disk's inode the C leaves alone). Every writer
//!   holds the kernel lock, as the C's (the inode's lock and the kernel lock).
//! - `e2di_rdev` and `e2di_shortlink` (macros for `e2di_blocks[0]` and `e2di_blocks`) are
//!   [`Ext2Inode`]'s accessors ([`Ext2Inode::e2di_rdev`], [`Ext2Inode::e2di_shortlink`]): the
//!   two byte views of LZ's `Ext2fsDinode` (two `unsafe` slices) and its `copy_nonoverlapping`
//!   copies (two more) go; [`Ext2fsDinode`] is a `hal::abi_pod!` image.

use core::cmp::min;
use core::mem::{offset_of, size_of};

use hal::user::{pod_from_bytes, pod_to_bytes};

use super::ext2fs::Ext2Superblock;
use super::ext2fs_bswap::e2fs_i_bswap;
use super::ext2fs_extents::Ext4ExtentCache;
use crate::assert::kpanic;
use crate::fs::ufs::ufs::dinode::Ufsino;
use crate::fs::ufs::ufs::inode::{DinField, DinWords, I64Word, U32Word};

pub use crate::fs::ufs::ufs::dinode::{NDADDR, NIADDR};

/// `EXT2_ROOTINO`: the root inode is the root of the file system. Inode 0 can not be used for
/// normal purposes and bad blocks are normally linked to inode 1, thus the root inode is 2.
/// Inode 3 to 10 are reserved in ext2fs.
pub const EXT2_ROOTINO: Ufsino = 2;
/// `EXT2_RESIZEINO`.
pub const EXT2_RESIZEINO: Ufsino = 7;
/// `EXT2_FIRSTINO`.
pub const EXT2_FIRSTINO: Ufsino = 11;

/// `EXT2_MAXSYMLINKLEN`.
pub const EXT2_MAXSYMLINKLEN: usize = (NDADDR + NIADDR) * size_of::<u32>();
/// `E2MAXSYMLINKLEN`.
pub const E2MAXSYMLINKLEN: usize = (NDADDR + NIADDR) * size_of::<u32>();

/// `struct ext2fs_dinode`: the on-disk inode (little-endian on disk).
#[repr(C)]
#[allow(non_snake_case)] // `e2di__reserved`, the C's name with its double underscore
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ext2fsDinode {
    /// `e2di_mode`: 0: IFMT, permissions; see below.
    pub e2di_mode: u16,
    /// `e2di_uid_low`: 2: owner UID, bits 15:0.
    pub e2di_uid_low: u16,
    /// `e2di_size`: 4: file size (bytes) bits 31:0.
    pub e2di_size: u32,
    /// `e2di_atime`: 8: access time.
    pub e2di_atime: u32,
    /// `e2di_ctime`: 12: change time.
    pub e2di_ctime: u32,
    /// `e2di_mtime`: 16: modification time.
    pub e2di_mtime: u32,
    /// `e2di_dtime`: 20: deletion time.
    pub e2di_dtime: u32,
    /// `e2di_gid_low`: 24: owner GID, lowest bits.
    pub e2di_gid_low: u16,
    /// `e2di_nlink`: 26: file link count.
    pub e2di_nlink: u16,
    /// `e2di_nblock`: 28: blocks count.
    pub e2di_nblock: u32,
    /// `e2di_flags`: 32: status flags (chflags).
    pub e2di_flags: u32,
    /// `e2di_version_lo`: 36: inode version, bits 31:0.
    pub e2di_version_lo: u32,
    /// `e2di_blocks`: 40: disk blocks.
    pub e2di_blocks: [u32; NDADDR + NIADDR],
    /// `e2di_gen`: 100: generation number.
    pub e2di_gen: u32,
    /// `e2di_facl`: 104: file ACL, bits 31:0.
    pub e2di_facl: u32,
    /// `e2di_size_hi`: 108: file size (bytes), bits 63:32.
    pub e2di_size_hi: u32,
    /// `e2di_faddr`: 112: fragment address (obsolete).
    pub e2di_faddr: u32,
    /// `e2di_nblock_hi`: 116: blocks count, bits 47:32.
    pub e2di_nblock_hi: u16,
    /// `e2di_facl_hi`: 118: file ACL, bits 47:32.
    pub e2di_facl_hi: u16,
    /// `e2di_uid_high`: 120: owner UID, bits 31:16.
    pub e2di_uid_high: u16,
    /// `e2di_gid_high`: 122: owner GID, bits 31:16.
    pub e2di_gid_high: u16,
    /// `e2di_chksum_lo`: 124: inode checksum, bits 15:0.
    pub e2di_chksum_lo: u16,
    /// `e2di__reserved`: 126: unused.
    pub e2di__reserved: u16,
    /// `e2di_isize`: 128: size of this inode.
    pub e2di_isize: u16,
    /// `e2di_chksum_hi`: 130: inode checksum, bits 31:16.
    pub e2di_chksum_hi: u16,
    /// `e2di_x_ctime`: 132: extra change time.
    pub e2di_x_ctime: u32,
    /// `e2di_x_mtime`: 136: extra modification time.
    pub e2di_x_mtime: u32,
    /// `e2di_x_atime`: 140: extra access time.
    pub e2di_x_atime: u32,
    /// `e2di_crtime`: 144: creation (birth) time.
    pub e2di_crtime: u32,
    /// `e2di_x_crtime`: 148: extra creation (birth) time.
    pub e2di_x_crtime: u32,
    /// `e2di_version_hi`: 152: inode version, bits 63:31.
    pub e2di_version_hi: u32,
}

hal::abi_pod!(Ext2fsDinode {
    e2di_mode: u16,
    e2di_uid_low: u16,
    e2di_size: u32,
    e2di_atime: u32,
    e2di_ctime: u32,
    e2di_mtime: u32,
    e2di_dtime: u32,
    e2di_gid_low: u16,
    e2di_nlink: u16,
    e2di_nblock: u32,
    e2di_flags: u32,
    e2di_version_lo: u32,
    e2di_blocks: [u32; NDADDR + NIADDR],
    e2di_gen: u32,
    e2di_facl: u32,
    e2di_size_hi: u32,
    e2di_faddr: u32,
    e2di_nblock_hi: u16,
    e2di_facl_hi: u16,
    e2di_uid_high: u16,
    e2di_gid_high: u16,
    e2di_chksum_lo: u16,
    e2di__reserved: u16,
    e2di_isize: u16,
    e2di_chksum_hi: u16,
    e2di_x_ctime: u32,
    e2di_x_mtime: u32,
    e2di_x_atime: u32,
    e2di_crtime: u32,
    e2di_x_crtime: u32,
    e2di_version_hi: u32,
});

/// `EXT2_IEXEC`: executable.
pub const EXT2_IEXEC: u16 = 0o000100;
/// `EXT2_IWRITE`: writeable.
pub const EXT2_IWRITE: u16 = 0o000200;
/// `EXT2_IREAD`: readable.
pub const EXT2_IREAD: u16 = 0o000400;
/// `EXT2_ISVTX`: sticky bit.
pub const EXT2_ISVTX: u16 = 0o001000;
/// `EXT2_ISGID`: set-gid.
pub const EXT2_ISGID: u16 = 0o002000;
/// `EXT2_ISUID`: set-uid.
pub const EXT2_ISUID: u16 = 0o004000;

/// `EXT2_IFMT`: mask of file type.
pub const EXT2_IFMT: u16 = 0o170000;
/// `EXT2_IFIFO`: named pipe (fifo).
pub const EXT2_IFIFO: u16 = 0o010000;
/// `EXT2_IFCHR`: character device.
pub const EXT2_IFCHR: u16 = 0o020000;
/// `EXT2_IFDIR`: directory file.
pub const EXT2_IFDIR: u16 = 0o040000;
/// `EXT2_IFBLK`: block device.
pub const EXT2_IFBLK: u16 = 0o060000;
/// `EXT2_IFREG`: regular file.
pub const EXT2_IFREG: u16 = 0o100000;
/// `EXT2_IFLNK`: symbolic link.
pub const EXT2_IFLNK: u16 = 0o120000;
/// `EXT2_IFSOCK`: UNIX domain socket.
pub const EXT2_IFSOCK: u16 = 0o140000;

/// `EXT2_SECRM`: secure deletion.
pub const EXT2_SECRM: u32 = 0x0000_0001;
/// `EXT2_UNRM`: undelete.
pub const EXT2_UNRM: u32 = 0x0000_0002;
/// `EXT2_COMPR`: compress file.
pub const EXT2_COMPR: u32 = 0x0000_0004;
/// `EXT2_SYNC`: synchronous updates.
pub const EXT2_SYNC: u32 = 0x0000_0008;
/// `EXT2_IMMUTABLE`: immutable file.
pub const EXT2_IMMUTABLE: u32 = 0x0000_0010;
/// `EXT2_APPEND`: writes to file may only append.
pub const EXT2_APPEND: u32 = 0x0000_0020;
/// `EXT2_NODUMP`: do not dump file.
pub const EXT2_NODUMP: u32 = 0x0000_0040;
/// `EXT2_NOATIME`: do not update access time.
pub const EXT2_NOATIME: u32 = 0x0000_0080;
/// `EXT4_INDEX`: hash-indexed directory.
pub const EXT4_INDEX: u32 = 0x0000_1000;
/// `EXT4_JOURNAL_DATA`: file data should be journaled.
pub const EXT4_JOURNAL_DATA: u32 = 0x0000_4000;
/// `EXT4_DIRSYNC`: all dirent updates done synchronously.
pub const EXT4_DIRSYNC: u32 = 0x0001_0000;
/// `EXT4_TOPDIR`: top of directory hierarchies.
pub const EXT4_TOPDIR: u32 = 0x0002_0000;
/// `EXT4_HUGE_FILE`: nblocks unit is fsb, not db.
pub const EXT4_HUGE_FILE: u32 = 0x0004_0000;
/// `EXT4_EXTENTS`: inode uses extents.
pub const EXT4_EXTENTS: u32 = 0x0008_0000;
/// `EXT4_EOFBLOCKS`: blocks allocated beyond EOF.
pub const EXT4_EOFBLOCKS: u32 = 0x0040_0000;

/// `EXT2_REV0_DINODE_SIZE`: size of the revision 0 on-disk inode.
pub const EXT2_REV0_DINODE_SIZE: usize = 128;

/// `sizeof(struct ext2fs_dinode)`.
pub const EXT2FS_DINODE_SIZE: usize = size_of::<Ext2fsDinode>();

/// `struct ext2fs_dinode` in core: its 156 bytes as 20 relaxed words.
pub type Ext2Din = DinWords<20>;

/// The ext2fs state of an inode in core: `*ip->i_e2din` (`dinode_u.e2fs_din`) and
/// `ip->inode_ext.e2fs` (`struct ext2fs_inode_ext`), one item of `ext2dinopl`.
///
/// Locks: as the inode's words (`ufs/inode.rs`): every writer holds the kernel lock, which
/// orders the writes; a reader gets whole members.
pub struct Ext2Inode {
    /// `*i_e2din`.
    din: Ext2Din,
    /// `i_e2fs_last_lblk` (`ext2fs_last_lblk`): last logical block allocated.
    pub i_e2fs_last_lblk: U32Word,
    /// `i_e2fs_last_blk` (`ext2fs_last_blk`): last block allocated on disk.
    pub i_e2fs_last_blk: U32Word,
    /// `i_e2fs_uid` (`ext2fs_effective_uid`): the 32-bit effective uid.
    pub i_e2fs_uid: U32Word,
    /// `i_e2fs_gid` (`ext2fs_effective_gid`): the 32-bit effective gid.
    pub i_e2fs_gid: U32Word,
    /// `i_e2fs_ext_cache.ec_start` (`ext2fs_extent_cache`).
    ec_start: I64Word,
    /// `i_e2fs_ext_cache.ec_blk`.
    ec_blk: U32Word,
    /// `i_e2fs_ext_cache.ec_len`.
    ec_len: U32Word,
    /// `i_e2fs_ext_cache.ec_type`.
    ec_type: U32Word,
}

impl Ext2Inode {
    /// `pool_get(&ext2fs_dinode_pool, PR_WAITOK | PR_ZERO)` and `inode_ext` zeroed: all zero.
    pub const fn new() -> Self {
        Self {
            din: DinWords::zeroed(),
            i_e2fs_last_lblk: U32Word::new(0),
            i_e2fs_last_blk: U32Word::new(0),
            i_e2fs_uid: U32Word::new(0),
            i_e2fs_gid: U32Word::new(0),
            ec_start: I64Word::new(0),
            ec_blk: U32Word::new(0),
            ec_len: U32Word::new(0),
            ec_type: U32Word::new(0),
        }
    }

    /// The dinode's member at `offset_of!(Ext2fsDinode, ..)`.
    #[inline]
    fn get<T: DinField>(&self, off: usize) -> T {
        self.din.get(off)
    }

    /// The dinode's member at `off` set to `v`, by a holder of the kernel lock.
    #[inline]
    fn set<T: DinField>(&self, off: usize, v: T) {
        self.din.set(off, v);
    }

    /// The byte offset of `e2di_blocks[i]`: `panic(9)` past the array, where the C would
    /// index past it.
    #[inline]
    fn block_off(i: usize) -> usize {
        if i >= NDADDR + NIADDR {
            kpanic(format_args!("ext2fs dinode: block pointer {i}"));
        }
        offset_of!(Ext2fsDinode, e2di_blocks) + 4 * i
    }

    /// `ip->i_e2fs_blocks[i]` as stored (little-endian on disk: the callers `letoh32` it).
    #[inline]
    pub fn i_e2fs_block(&self, i: usize) -> u32 {
        self.get(Self::block_off(i))
    }

    /// `ip->i_e2fs_blocks[i] = v` (`v` as stored: `htole32`ed by the caller).
    #[inline]
    pub fn set_i_e2fs_block(&self, i: usize, v: u32) {
        self.set(Self::block_off(i), v);
    }

    /// `ip->i_e2fs_blocks`: a copy of the block pointers, as stored.
    pub fn i_e2fs_blocks(&self) -> [u32; NDADDR + NIADDR] {
        core::array::from_fn(|i| self.i_e2fs_block(i))
    }

    /// `memcpy(ip->i_e2fs_blocks, blks, sizeof(ip->i_e2fs_blocks))`.
    pub fn set_i_e2fs_blocks(&self, blks: &[u32; NDADDR + NIADDR]) {
        for (i, &b) in blks.iter().enumerate() {
            self.set_i_e2fs_block(i, b);
        }
    }

    /// `e2di_rdev` (`e2di_blocks[0]`): block and character devices overlay the first data
    /// block with their `dev_t` value (as stored).
    pub fn e2di_rdev(&self) -> u32 {
        self.i_e2fs_block(0)
    }

    /// `e2di_rdev = v`.
    pub fn set_e2di_rdev(&self, v: u32) {
        self.set_i_e2fs_block(0, v);
    }

    /// `e2di_shortlink` (`e2di_blocks`): a short symbolic link keeps its path in the block
    /// pointers' area, `EXT2_MAXSYMLINKLEN` bytes; a copy of them.
    pub fn e2di_shortlink(&self) -> [u8; EXT2_MAXSYMLINKLEN] {
        let mut b = [0u8; EXT2_MAXSYMLINKLEN];
        self.din
            .read_bytes(offset_of!(Ext2fsDinode, e2di_blocks), &mut b);
        b
    }

    /// `memcpy(e2di_shortlink + off, src, len)`: `panic(9)` past the area.
    pub fn set_e2di_shortlink(&self, off: usize, src: &[u8]) {
        if off + src.len() > EXT2_MAXSYMLINKLEN {
            kpanic(format_args!(
                "ext2fs dinode: shortlink bytes {off}+{}",
                src.len()
            ));
        }
        self.din
            .write_bytes(offset_of!(Ext2fsDinode, e2di_blocks) + off, src);
    }

    /// `memset(ip->i_e2din, 0, sizeof(struct ext2fs_dinode))` (`ext2fs_inode_alloc`).
    pub fn clear(&self) {
        self.din.write_bytes(0, &[0u8; EXT2FS_DINODE_SIZE]);
    }

    /// `*ip->i_e2din`: a copy of the whole dinode.
    pub fn din(&self) -> Ext2fsDinode {
        let mut b = [0u8; EXT2FS_DINODE_SIZE];
        self.din.read_bytes(0, &mut b);
        pod_from_bytes(&b).unwrap_or_default()
    }

    /// `ip->i_e2fs_ext_cache`: a copy of the extent cache.
    pub fn i_e2fs_ext_cache(&self) -> Ext4ExtentCache {
        Ext4ExtentCache {
            ec_start: self.ec_start.get(),
            ec_blk: self.ec_blk.get(),
            ec_len: self.ec_len.get(),
            ec_type: self.ec_type.get(),
        }
    }

    /// `ip->i_e2fs_ext_cache = *ecp`, by a holder of the kernel lock.
    pub fn set_i_e2fs_ext_cache(&self, ecp: &Ext4ExtentCache) {
        self.ec_start.set(ecp.ec_start);
        self.ec_blk.set(ecp.ec_blk);
        self.ec_len.set(ecp.ec_len);
        self.ec_type.set(ecp.ec_type);
    }
}

impl Default for Ext2Inode {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates the `i_e2fs_*` accessors of `<ufs/ufs/inode.h>` over the in-core dinode: `$get`
/// reads `ip->i_e2din->$field` (a load and a shift of its word), `$set` writes it.
macro_rules! e2din_field {
    ($($get:ident, $set:ident: $t:ty = $field:ident;)+) => {
        impl Ext2Inode {
            $(
                #[doc = concat!("`ip->", stringify!($get), "` (`i_e2din->", stringify!($field), "`).")]
                #[inline]
                pub fn $get(&self) -> $t {
                    self.get::<$t>(offset_of!(Ext2fsDinode, $field))
                }

                #[doc = concat!("`ip->", stringify!($get), " = v`.")]
                #[inline]
                pub fn $set(&self, v: $t) {
                    self.set::<$t>(offset_of!(Ext2fsDinode, $field), v);
                }
            )+
        }
    };
}

e2din_field! {
    i_e2fs_mode, set_i_e2fs_mode: u16 = e2di_mode;
    i_e2fs_size, set_i_e2fs_size: u32 = e2di_size;
    i_e2fs_atime, set_i_e2fs_atime: u32 = e2di_atime;
    i_e2fs_ctime, set_i_e2fs_ctime: u32 = e2di_ctime;
    i_e2fs_mtime, set_i_e2fs_mtime: u32 = e2di_mtime;
    i_e2fs_dtime, set_i_e2fs_dtime: u32 = e2di_dtime;
    i_e2fs_nlink, set_i_e2fs_nlink: u16 = e2di_nlink;
    i_e2fs_nblock, set_i_e2fs_nblock: u32 = e2di_nblock;
    i_e2fs_flags, set_i_e2fs_flags: u32 = e2di_flags;
    i_e2fs_gen, set_i_e2fs_gen: u32 = e2di_gen;
    i_e2fs_facl, set_i_e2fs_facl: u32 = e2di_facl;
    i_e2fs_size_hi, set_i_e2fs_size_hi: u32 = e2di_size_hi;
    i_e2fs_faddr, set_i_e2fs_faddr: u32 = e2di_faddr;
    i_e2fs_nblock_hi, set_i_e2fs_nblock_hi: u16 = e2di_nblock_hi;
    i_e2fs_uid_low, set_i_e2fs_uid_low: u16 = e2di_uid_low;
    i_e2fs_gid_low, set_i_e2fs_gid_low: u16 = e2di_gid_low;
    i_e2fs_uid_high, set_i_e2fs_uid_high: u16 = e2di_uid_high;
    i_e2fs_gid_high, set_i_e2fs_gid_high: u16 = e2di_gid_high;
    i_e2fs_isize, set_i_e2fs_isize: u16 = e2di_isize;
}

/// `e2fs_iload(fs, old, new)`: loads the inode in the buffer bytes `old` into the in-core
/// `new`: `MIN(EXT2_DINODE_SIZE(fs), sizeof(*new))` bytes, little-endian on disk (the bytes
/// of `new` past them are left as they are).
pub fn e2fs_iload(fs: &Ext2Superblock, old: &[u8], new: &Ext2Inode) {
    let n = min(fs.dinode_size(), EXT2FS_DINODE_SIZE);
    let Some(src) = old.get(..n) else {
        kpanic(format_args!("e2fs_iload: {} bytes, want {}", old.len(), n));
    };
    if cfg!(target_endian = "big") {
        // The swapper writes the members it knows into `new`, which keeps the others.
        let mut b = [0u8; EXT2FS_DINODE_SIZE];
        b[..n].copy_from_slice(src);
        let disk = pod_from_bytes::<Ext2fsDinode>(&b).unwrap_or_default();
        let mut d = new.din();
        e2fs_i_bswap(fs, &disk, &mut d);
        let _ = pod_to_bytes(&d, &mut b);
        new.din.write_bytes(0, &b[..n]);
    } else {
        new.din.write_bytes(0, src);
    }
}

/// `e2fs_isave(fs, old, new)`: stores the in-core inode `old` in the buffer bytes `new`:
/// `MIN(EXT2_DINODE_SIZE(fs), sizeof(*old))` bytes, little-endian on disk (the bytes of the
/// buffer past them are left as they are).
pub fn e2fs_isave(fs: &Ext2Superblock, old: &Ext2Inode, new: &mut [u8]) {
    let n = min(fs.dinode_size(), EXT2FS_DINODE_SIZE);
    let Some(dst) = new.get_mut(..n) else {
        kpanic(format_args!("e2fs_isave: {} bytes, want {}", new.len(), n));
    };
    if cfg!(target_endian = "big") {
        // The swapper writes the members it knows into the buffer's inode, which keeps the
        // others.
        let mut b = [0u8; EXT2FS_DINODE_SIZE];
        b[..n].copy_from_slice(dst);
        let mut disk = pod_from_bytes::<Ext2fsDinode>(&b).unwrap_or_default();
        e2fs_i_bswap(fs, &old.din(), &mut disk);
        let _ = pod_to_bytes(&disk, &mut b);
        dst.copy_from_slice(&b[..n]);
    } else {
        old.din.read_bytes(0, dst);
    }
}

const _: () = {
    assert!(size_of::<Ext2fsDinode>() == 156);
    assert!(core::mem::offset_of!(Ext2fsDinode, e2di_blocks) == 40);
    assert!(core::mem::offset_of!(Ext2fsDinode, e2di_gen) == 100);
    assert!(core::mem::offset_of!(Ext2fsDinode, e2di_nblock_hi) == 116);
    assert!(core::mem::offset_of!(Ext2fsDinode, e2di_isize) == 128);
    assert!(core::mem::offset_of!(Ext2fsDinode, e2di_version_hi) == 152);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;
    use crate::testutil;

    fn fs(rev: u32, isize_: u16) -> Ext2Superblock {
        testutil::install();
        let fs = Ext2Superblock::new();
        fs.set_e2fs_rev(rev);
        fs.set_e2fs_inode_size(isize_);
        fs
    }

    #[test]
    fn dinode_size_follows_the_revision() {
        assert_eq!(fs(0, 256).dinode_size(), 128);
        assert_eq!(fs(1, 256).dinode_size(), 256);
        assert_eq!(fs(1, 128).dinode_size(), 128);
    }

    #[test]
    fn load_and_save_copy_the_smaller_of_the_two_sizes() {
        let mut img = [0u8; 256];
        for (i, b) in img.iter_mut().enumerate() {
            *b = i as u8;
        }
        let d = Ext2Inode::new();
        e2fs_iload(&fs(1, 128), &img, &d);
        assert_eq!(d.i_e2fs_mode(), 0x0100);
        assert_eq!(d.i_e2fs_block(0), 0x2b2a_2928);
        assert_eq!(d.i_e2fs_isize(), 0, "past 128 bytes: untouched");
        e2fs_iload(&fs(1, 256), &img, &d);
        assert_eq!(d.i_e2fs_isize(), 0x8180);
        assert_eq!(d.din().e2di_version_hi, 0x9b9a_9998);
        let mut out = [0xffu8; 256];
        e2fs_isave(&fs(1, 256), &d, &mut out);
        assert_eq!(out[..156], img[..156]);
        assert_eq!(
            out[156..],
            [0xff; 100],
            "the bytes past the structure: untouched"
        );
        let mut out = [0xffu8; 256];
        e2fs_isave(&fs(0, 256), &d, &mut out);
        assert_eq!(out[..128], img[..128]);
        assert_eq!(out[128..], [0xff; 128], "a revision 0 inode is 128 bytes");
    }

    #[test]
    fn rdev_and_shortlink_overlay_the_block_pointers() {
        let d = Ext2Inode::new();
        d.set_e2di_shortlink(0, b"/tmp\0");
        assert_eq!(&d.e2di_shortlink()[..5], b"/tmp\0");
        assert_eq!(d.e2di_shortlink().len(), 60);
        d.set_e2di_shortlink(57, b"xyz");
        assert_eq!(d.i_e2fs_block(14).to_le_bytes()[1..], *b"xyz");
        d.set_e2di_rdev(0x0102);
        assert_eq!(d.i_e2fs_block(0), 0x0102);
        assert_eq!(d.e2di_rdev(), 0x0102);
        assert_eq!(d.din().e2di_blocks[0], 0x0102);
    }

    #[test]
    fn every_member_is_read_and_written_at_its_offset() {
        let d = Ext2Inode::new();
        d.set_i_e2fs_mode(0o100644);
        d.set_i_e2fs_uid_low(0x0201);
        d.set_i_e2fs_size(0x0605_0403);
        d.set_i_e2fs_atime(8);
        d.set_i_e2fs_ctime(12);
        d.set_i_e2fs_mtime(16);
        d.set_i_e2fs_dtime(20);
        d.set_i_e2fs_gid_low(24);
        d.set_i_e2fs_nlink(26);
        d.set_i_e2fs_nblock(28);
        d.set_i_e2fs_flags(EXT4_EXTENTS);
        for i in 0..NDADDR + NIADDR {
            d.set_i_e2fs_block(i, 100 + i as u32);
        }
        d.set_i_e2fs_gen(100);
        d.set_i_e2fs_facl(104);
        d.set_i_e2fs_size_hi(108);
        d.set_i_e2fs_faddr(112);
        d.set_i_e2fs_nblock_hi(116);
        d.set_i_e2fs_uid_high(120);
        d.set_i_e2fs_gid_high(122);
        d.set_i_e2fs_isize(128);
        let c = d.din();
        assert_eq!(
            (
                c.e2di_mode,
                c.e2di_uid_low,
                c.e2di_size,
                c.e2di_atime,
                c.e2di_ctime
            ),
            (0o100644, 0x0201, 0x0605_0403, 8, 12)
        );
        assert_eq!(
            (
                c.e2di_mtime,
                c.e2di_dtime,
                c.e2di_gid_low,
                c.e2di_nlink,
                c.e2di_nblock
            ),
            (16, 20, 24, 26, 28)
        );
        assert_eq!(c.e2di_flags, EXT4_EXTENTS);
        assert_eq!(c.e2di_blocks, core::array::from_fn(|i| 100 + i as u32));
        assert_eq!(d.i_e2fs_blocks(), c.e2di_blocks);
        assert_eq!(
            (
                c.e2di_gen,
                c.e2di_facl,
                c.e2di_size_hi,
                c.e2di_faddr,
                c.e2di_nblock_hi
            ),
            (100, 104, 108, 112, 116)
        );
        assert_eq!(
            (c.e2di_uid_high, c.e2di_gid_high, c.e2di_isize),
            (120, 122, 128)
        );
        assert_eq!((c.e2di_version_lo, c.e2di_version_hi), (0, 0));
        assert_eq!(
            (d.i_e2fs_size(), d.i_e2fs_nlink(), d.i_e2fs_gen()),
            (0x0605_0403, 26, 100)
        );
        d.set_i_e2fs_blocks(&[0; NDADDR + NIADDR]);
        assert_eq!(d.i_e2fs_blocks(), [0; NDADDR + NIADDR]);
        assert_eq!(d.i_e2fs_gen(), 100, "the neighbours are kept");
        d.clear();
        assert_eq!(d.din(), Ext2fsDinode::default());

        // The in-core members beside the dinode.
        d.i_e2fs_last_lblk.set(3);
        d.i_e2fs_uid.set(70_000);
        let cache = Ext4ExtentCache {
            ec_start: 1 << 40,
            ec_blk: 5,
            ec_len: 6,
            ec_type: 2,
        };
        d.set_i_e2fs_ext_cache(&cache);
        assert_eq!(d.i_e2fs_ext_cache(), cache);
        assert_eq!((d.i_e2fs_last_lblk.get(), d.i_e2fs_uid.get()), (3, 70_000));
    }

    #[test]
    #[should_panic(expected = "block pointer 15")]
    fn a_block_pointer_past_the_array_panics() {
        let _ = Ext2Inode::new().i_e2fs_block(15);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ext2fs/ext2fs_dinode.h");
        for (name, value) in [
            ("NDADDR", NDADDR as i64),
            ("NIADDR", NIADDR as i64),
            ("EXT2_IEXEC", i64::from(EXT2_IEXEC)),
            ("EXT2_IWRITE", i64::from(EXT2_IWRITE)),
            ("EXT2_IREAD", i64::from(EXT2_IREAD)),
            ("EXT2_ISVTX", i64::from(EXT2_ISVTX)),
            ("EXT2_ISGID", i64::from(EXT2_ISGID)),
            ("EXT2_ISUID", i64::from(EXT2_ISUID)),
            ("EXT2_IFMT", i64::from(EXT2_IFMT)),
            ("EXT2_IFIFO", i64::from(EXT2_IFIFO)),
            ("EXT2_IFCHR", i64::from(EXT2_IFCHR)),
            ("EXT2_IFDIR", i64::from(EXT2_IFDIR)),
            ("EXT2_IFBLK", i64::from(EXT2_IFBLK)),
            ("EXT2_IFREG", i64::from(EXT2_IFREG)),
            ("EXT2_IFLNK", i64::from(EXT2_IFLNK)),
            ("EXT2_IFSOCK", i64::from(EXT2_IFSOCK)),
            ("EXT2_SECRM", i64::from(EXT2_SECRM)),
            ("EXT2_UNRM", i64::from(EXT2_UNRM)),
            ("EXT2_COMPR", i64::from(EXT2_COMPR)),
            ("EXT2_SYNC", i64::from(EXT2_SYNC)),
            ("EXT2_IMMUTABLE", i64::from(EXT2_IMMUTABLE)),
            ("EXT2_APPEND", i64::from(EXT2_APPEND)),
            ("EXT2_NODUMP", i64::from(EXT2_NODUMP)),
            ("EXT2_NOATIME", i64::from(EXT2_NOATIME)),
            ("EXT4_INDEX", i64::from(EXT4_INDEX)),
            ("EXT4_JOURNAL_DATA", i64::from(EXT4_JOURNAL_DATA)),
            ("EXT4_DIRSYNC", i64::from(EXT4_DIRSYNC)),
            ("EXT4_TOPDIR", i64::from(EXT4_TOPDIR)),
            ("EXT4_HUGE_FILE", i64::from(EXT4_HUGE_FILE)),
            ("EXT4_EXTENTS", i64::from(EXT4_EXTENTS)),
            ("EXT4_EOFBLOCKS", i64::from(EXT4_EOFBLOCKS)),
            ("EXT2_REV0_DINODE_SIZE", EXT2_REV0_DINODE_SIZE as i64),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
    }
}
/* </TESTS> */
