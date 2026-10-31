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
//! `<ufs/ufs/inode.h>`: the in-core inode is the kernel's (`kernel::fs::ufs::ufs::inode`,
//! N5.5a1): this module keeps LZ's names for `bsd`'s file systems (`ffs`, `mfs`, `ext2fs`)
//! and the UFS files still here (lookup, the vnode operations, quotas, dirhash), and the
//! members of `bsd`'s types an inode carries ([`BsdInodeG`], the glue's `InodeG`).
//!
//! Upstream: sys/ufs/ufs/inode.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - `inode_u` (`i_fs`, `i_e2fs`), ext2fs's `inode_ext` ([`Ext2fsInodeExt`]) and ext2fs's
//!   dinode (`i_e2din`) are [`BsdInodeG`]'s, reached as `ip.g`: ffs's super block is `bsd`'s
//!   until N5.5b, ext2fs's members until N5.6 (`i_dquot[]` and the dirhash are the kernel
//!   inode's since N5.5a2). `i_fs` and `i_e2fs` are read through [`InodeExt`] (`ip.fs()`,
//!   `ip.e2fs()`; `i_ump` is the kernel's `ip.ump()`); the
//!   `i_e2fs_*` shorthands are [`BsdInodeG`]'s methods (`ip.g.i_e2fs_mode()`, ...), over
//!   `i_e2din` ([`BsdInodeG::with_e2din`], the one dinode pointer cast to `ext2fs_dinode`);
//!   `i_e2fs_blocks` is `i_e2fs_blocks()` (a copy of the 15 pointers), `i_e2fs_block(i)` and
//!   `set_i_e2fs_block(i, v)`. The C's `i_e2fs_faddr_hi` names a member that does not exist
//!   and is left out. `EXT2FS_ITIMES` is [`InodeExt::ext2fs_itimes`].
//! - The `struct inode_vtbl` calls (`UFS_TRUNCATE`, `UFS_UPDATE`, ...) are functions with the
//!   macros' names over the kernel's `InodeOps` ([`Inode::vtbl`]); `iv_inode_alloc` and
//!   `iv_buf_alloc` return the vnode or buffer the C passes back through `vpp`/`bpp`,
//!   `iv_bufatoff` the buffer and the offset of `res` in it, lent (`IoBuf::Lent`) as `bsd`'s
//!   buffers are.

use core::cell::Cell;

#[cfg(feature = "ext2fs")]
use crate::kern::kern_tc::gettime;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::sys::buf::Buf;
use crate::sys::errno::Errno;
use crate::sys::types::{Mode, Off};
use crate::sys::vnode::Vnode;
#[cfg(feature = "ext2fs")]
use crate::ufs::ext2fs::ext2fs::MExt2fs;
#[cfg(feature = "ext2fs")]
use crate::ufs::ext2fs::ext2fs_dinode::Ext2fsDinode;
#[cfg(feature = "ext2fs")]
use crate::ufs::ext2fs::ext2fs_extents::Ext4ExtentCache;
use crate::ufs::ffs::fs::Fs;
use crate::ufs::ufs::dinode::Ufsino;
#[cfg(feature = "ext2fs")]
use crate::ufs::ufs::dinode::{NDADDR, NIADDR};
#[cfg(feature = "ext2fs")]
use hal::link::PtrWord;
pub use kernel::fs::ufs::ufs::inode::{
    CiCell, DinField, DinWords, Dinode, I32Word, IN_ACCESS, IN_CHANGE, IN_EXLOCK, IN_HASHED,
    IN_LAZYMOD, IN_MODIFIED, IN_RENAME, IN_SHLOCK, IN_UPDATE, Indir, InodeOps, U32Word, U64Word,
    Ufid, Ufs1Din, Ufs2Din, daddr32_at, daddr64_at, dinode1_at, dinode2_at, doingasync,
    set_daddr32_at, set_daddr64_at, set_dinode1_at, set_dinode2_at, vtoi,
};
use kernel::fs::vops::{Cred, IoBuf};

/// `struct ext2fs_inode_ext`: the ext2fs members of an inode.
#[cfg(feature = "ext2fs")]
pub struct Ext2fsInodeExt {
    /// `ext2fs_last_lblk`: last logical blk allocated.
    pub ext2fs_last_lblk: Cell<u32>,
    /// `ext2fs_last_blk`: last blk allocated on disk.
    pub ext2fs_last_blk: Cell<u32>,
    /// `ext2fs_effective_uid`: effective inode uid.
    pub ext2fs_effective_uid: Cell<u32>,
    /// `ext2fs_effective_gid`: effective inode gid.
    pub ext2fs_effective_gid: Cell<u32>,
    /// `ext2fs_extent_cache`.
    pub ext2fs_extent_cache: Cell<Ext4ExtentCache>,
}

#[cfg(feature = "ext2fs")]
impl Ext2fsInodeExt {
    /// A zeroed structure.
    pub const fn new() -> Self {
        Self {
            ext2fs_last_lblk: Cell::new(0),
            ext2fs_last_blk: Cell::new(0),
            ext2fs_effective_uid: Cell::new(0),
            ext2fs_effective_gid: Cell::new(0),
            ext2fs_extent_cache: Cell::new(Ext4ExtentCache {
                ec_start: 0,
                ec_blk: 0,
                ec_len: 0,
                ec_type: 0,
            }),
        }
    }
}

#[cfg(feature = "ext2fs")]
impl Default for Ext2fsInodeExt {
    fn default() -> Self {
        Self::new()
    }
}

/// The members of `bsd`'s types an inode carries (the glue's `InodeG`, docs/SCAFFOLDING.md).
#[derive(Default)]
pub struct BsdInodeG {
    /// `i_fs` (`inode_u.fs`): associated filesystem.
    pub i_fs: Cell<Option<&'static Fs>>,
    /// `i_e2fs` (`inode_u.e2fs`): associated ext2fs file system (feature `ext2fs`), in place of
    /// `i_fs` for an ext2fs inode.
    #[cfg(feature = "ext2fs")]
    pub i_e2fs: Cell<Option<&'static MExt2fs>>,
    /// `inode_ext.e2fs`: the ext2fs members (feature `ext2fs`), in place of the dirhash for an
    /// ext2fs inode.
    #[cfg(feature = "ext2fs")]
    pub i_e2fs_ext: Ext2fsInodeExt,
    /// `i_e2din` (`dinode_u.e2fs`): an ext2fs inode's dinode, an item of
    /// `ext2fs_dinode_pool` (NULL for an FFS inode, whose dinode is the kernel's).
    #[cfg(feature = "ext2fs")]
    pub i_e2din: PtrWord<Ext2fsDinode>,
}

// SAFETY: LZ's argument for `struct inode` (sys/ufs/ufs/inode.rs:244-245 at the pin), moved
// with the members it covered (the same site, not a new one, as `VnodeG`'s at N5.2): they are
// changed under the inode's lock or the kernel lock, as in C. Paid member by member: the quota
// and dirhash ones went at N5.5a2 (the kernel inode's), `i_fs` at N5.5b, the ext2fs ones at
// N5.6.
unsafe impl Sync for BsdInodeG {}

/// Generates a getter and a setter of a member of the ext2fs dinode (feature `ext2fs`).
macro_rules! e2din_field {
    ($(#[$doc:meta])* $get:ident, $set:ident, $t:ty, $field:ident) => {
        $(#[$doc])*
        #[cfg(feature = "ext2fs")]
        pub fn $get(&self) -> $t {
            self.with_e2din(|d| d.$field)
        }

        $(#[$doc])*
        #[cfg(feature = "ext2fs")]
        pub fn $set(&self, v: $t) {
            self.with_e2din(|d| d.$field = v)
        }
    };
}

impl BsdInodeG {
    /// `f(ip->i_e2din)`: the ext2fs dinode, for the duration of `f`.
    #[cfg(feature = "ext2fs")]
    pub fn with_e2din<R>(&self, f: impl FnOnce(&mut Ext2fsDinode) -> R) -> R {
        let p = self.i_e2din.get_mut();
        if p.is_null() {
            panic(format_args!("inode {:p}: no ext2fs dinode", self));
        }
        // SAFETY: `ext2fs_vget` points `i_e2din` at the inode's own pool `ext2fs_dinode`
        // before the inode is used, and `ext2fs_reclaim` frees it with the inode; the
        // reference does not outlive `f`, and no other one is alive meanwhile (the accessors
        // are the only way in, none of them nests, and they run under the kernel lock).
        f(unsafe { &mut *p })
    }

    /// `ip->i_e2din == NULL`: an ext2fs inode without its dinode (yet, or any more).
    #[cfg(feature = "ext2fs")]
    pub fn e2din_is_null(&self) -> bool {
        self.i_e2din.get().is_null()
    }

    /// `ip->i_e2fs_last_lblk` (`inode_ext.e2fs.ext2fs_last_lblk`).
    #[cfg(feature = "ext2fs")]
    pub fn i_e2fs_last_lblk(&self) -> &Cell<u32> {
        &self.i_e2fs_ext.ext2fs_last_lblk
    }

    /// `ip->i_e2fs_last_blk`.
    #[cfg(feature = "ext2fs")]
    pub fn i_e2fs_last_blk(&self) -> &Cell<u32> {
        &self.i_e2fs_ext.ext2fs_last_blk
    }

    /// `ip->i_e2fs_uid`: the effective uid.
    #[cfg(feature = "ext2fs")]
    pub fn i_e2fs_uid(&self) -> &Cell<u32> {
        &self.i_e2fs_ext.ext2fs_effective_uid
    }

    /// `ip->i_e2fs_gid`: the effective gid.
    #[cfg(feature = "ext2fs")]
    pub fn i_e2fs_gid(&self) -> &Cell<u32> {
        &self.i_e2fs_ext.ext2fs_effective_gid
    }

    /// `ip->i_e2fs_ext_cache`: the extent cache.
    #[cfg(feature = "ext2fs")]
    pub fn i_e2fs_ext_cache(&self) -> &Cell<Ext4ExtentCache> {
        &self.i_e2fs_ext.ext2fs_extent_cache
    }

    e2din_field!(
        /// `ip->i_e2fs_mode`.
        i_e2fs_mode, set_i_e2fs_mode, u16, e2di_mode
    );
    e2din_field!(
        /// `ip->i_e2fs_size`.
        i_e2fs_size, set_i_e2fs_size, u32, e2di_size
    );
    e2din_field!(
        /// `ip->i_e2fs_atime`.
        i_e2fs_atime, set_i_e2fs_atime, u32, e2di_atime
    );
    e2din_field!(
        /// `ip->i_e2fs_ctime`.
        i_e2fs_ctime, set_i_e2fs_ctime, u32, e2di_ctime
    );
    e2din_field!(
        /// `ip->i_e2fs_mtime`.
        i_e2fs_mtime, set_i_e2fs_mtime, u32, e2di_mtime
    );
    e2din_field!(
        /// `ip->i_e2fs_dtime`.
        i_e2fs_dtime, set_i_e2fs_dtime, u32, e2di_dtime
    );
    e2din_field!(
        /// `ip->i_e2fs_nlink`.
        i_e2fs_nlink, set_i_e2fs_nlink, u16, e2di_nlink
    );
    e2din_field!(
        /// `ip->i_e2fs_nblock`.
        i_e2fs_nblock, set_i_e2fs_nblock, u32, e2di_nblock
    );
    e2din_field!(
        /// `ip->i_e2fs_flags`.
        i_e2fs_flags, set_i_e2fs_flags, u32, e2di_flags
    );
    e2din_field!(
        /// `ip->i_e2fs_gen`.
        i_e2fs_gen, set_i_e2fs_gen, u32, e2di_gen
    );
    e2din_field!(
        /// `ip->i_e2fs_facl`.
        i_e2fs_facl, set_i_e2fs_facl, u32, e2di_facl
    );
    e2din_field!(
        /// `ip->i_e2fs_size_hi`.
        i_e2fs_size_hi, set_i_e2fs_size_hi, u32, e2di_size_hi
    );
    e2din_field!(
        /// `ip->i_e2fs_faddr`.
        i_e2fs_faddr, set_i_e2fs_faddr, u32, e2di_faddr
    );
    e2din_field!(
        /// `ip->i_e2fs_nblock_hi`.
        i_e2fs_nblock_hi, set_i_e2fs_nblock_hi, u16, e2di_nblock_hi
    );
    e2din_field!(
        /// `ip->i_e2fs_uid_low`.
        i_e2fs_uid_low, set_i_e2fs_uid_low, u16, e2di_uid_low
    );
    e2din_field!(
        /// `ip->i_e2fs_gid_low`.
        i_e2fs_gid_low, set_i_e2fs_gid_low, u16, e2di_gid_low
    );
    e2din_field!(
        /// `ip->i_e2fs_uid_high`.
        i_e2fs_uid_high, set_i_e2fs_uid_high, u16, e2di_uid_high
    );
    e2din_field!(
        /// `ip->i_e2fs_gid_high`.
        i_e2fs_gid_high, set_i_e2fs_gid_high, u16, e2di_gid_high
    );

    /// `ip->i_e2fs_blocks`: a copy of the block pointers (`NDADDR + NIADDR` of them).
    #[cfg(feature = "ext2fs")]
    pub fn i_e2fs_blocks(&self) -> [u32; NDADDR + NIADDR] {
        self.with_e2din(|d| d.e2di_blocks)
    }

    /// `ip->i_e2fs_blocks[i]`.
    #[cfg(feature = "ext2fs")]
    pub fn i_e2fs_block(&self, i: usize) -> u32 {
        self.with_e2din(|d| d.e2di_blocks[i])
    }

    /// `ip->i_e2fs_blocks[i] = v`.
    #[cfg(feature = "ext2fs")]
    pub fn set_i_e2fs_block(&self, i: usize, v: u32) {
        self.with_e2din(|d| d.e2di_blocks[i] = v)
    }
}

/// `struct inode` of `bsd`'s threads: the kernel's.
pub type Inode = kernel::fs::ufs::ufs::inode::Inode<BsdProcGlue>;
/// A counted handle to one of `bsd`'s inodes ([`vtoi`]'s).
pub type InodeRef = kernel::fs::ufs::ufs::inode::InodeRef<BsdProcGlue>;
/// What [`Inode::new`] takes.
pub type InodeInit = kernel::fs::ufs::ufs::inode::InodeInit<BsdProcGlue>;

/// The members of `struct inode` that `bsd`'s types answer: `i_ump`, `inode_u`, and
/// `EXT2FS_ITIMES` (over the kernel's flags and ext2fs's dinode).
pub trait InodeExt {
    /// `ip->i_fs`: the FFS super block.
    fn fs(&self) -> &'static Fs;
    /// `ip->i_e2fs`: the inode's in-core ext2fs super block (feature `ext2fs`).
    #[cfg(feature = "ext2fs")]
    fn e2fs(&self) -> &'static MExt2fs;
    /// `EXT2FS_ITIMES(ip)`: updates the access, modification and change times the flags ask
    /// for, as `UFS_ITIMES` does for an FFS inode.
    #[cfg(feature = "ext2fs")]
    fn ext2fs_itimes(&self);
}

impl InodeExt for Inode {
    fn fs(&self) -> &'static Fs {
        match self.g.i_fs.get() {
            Some(fs) => fs,
            None => panic(format_args!("inode {:p}: no fs", self)),
        }
    }

    #[cfg(feature = "ext2fs")]
    fn e2fs(&self) -> &'static MExt2fs {
        match self.g.i_e2fs.get() {
            Some(fs) => fs,
            None => panic(format_args!("inode {:p}: no e2fs", self)),
        }
    }

    #[cfg(feature = "ext2fs")]
    fn ext2fs_itimes(&self) {
        let flag = self.i_flag.get();
        if flag & (IN_ACCESS | IN_CHANGE | IN_UPDATE) == 0 {
            return;
        }
        self.i_flag.set(flag | IN_MODIFIED);
        if flag & IN_ACCESS != 0 {
            self.g.set_i_e2fs_atime(gettime() as u32);
        }
        if flag & IN_UPDATE != 0 {
            self.g.set_i_e2fs_mtime(gettime() as u32);
        }
        if flag & IN_CHANGE != 0 {
            self.g.set_i_e2fs_ctime(gettime() as u32);
            self.i_modrev.set(self.i_modrev.get().wrapping_add(1));
        }
        self.i_flag
            .set(self.i_flag.get() & !(IN_ACCESS | IN_CHANGE | IN_UPDATE));
    }
}

/// `UFS_TRUNCATE(ip, off, flags, cred)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_TRUNCATE(ip: &Inode, off: Off, flags: i32, cred: Cred<'_>) -> Result<(), Errno> {
    ip.vtbl().truncate(ip, off, flags, cred)
}

/// `UFS_UPDATE(ip, sync)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_UPDATE(ip: &Inode, sync: i32) -> Result<(), Errno> {
    ip.vtbl().update(ip, sync)
}

/// `UFS_INODE_ALLOC(pip, mode, cred, vpp)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_INODE_ALLOC(pip: &Inode, mode: Mode, cred: Cred<'_>) -> Result<&'static Vnode, Errno> {
    pip.vtbl().inode_alloc(pip, mode, cred)
}

/// `UFS_INODE_FREE(pip, ino, mode)`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_INODE_FREE(pip: &Inode, ino: Ufsino, mode: Mode) -> Result<(), Errno> {
    pip.vtbl().inode_free(pip, ino, mode)
}

/// The buffer an `InodeOps` method gave, as `bsd`'s callers hold it: lent (its token parked
/// in it), as `bsd`'s file systems give theirs.
fn lent(b: IoBuf) -> &'static Buf {
    match b {
        IoBuf::Lent(bp) => bp,
        IoBuf::Token(t) => crate::sys::buf::lend(t),
    }
}

/// `UFS_BUF_ALLOC(ip, startoffset, size, cred, flags, bpp)`: the buffer the C returns in
/// `*bpp`.
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_BUF_ALLOC(
    ip: &Inode,
    startoffset: Off,
    size: i32,
    cred: Cred<'_>,
    flags: i32,
) -> Result<&'static Buf, Errno> {
    ip.vtbl()
        .buf_alloc(ip, startoffset, size, cred, flags)
        .map(lent)
}

/// `UFS_BUFATOFF(ip, offset, res, bpp)`: the buffer and the offset of `res` in its data, lent
/// as `bsd`'s buffers are (no `bsd` caller is left since N5.5a3: the kernel's `ufs_lookup` and
/// `ufs_readdir` take the inode's token).
#[allow(non_snake_case)] // the C macro's name
pub fn UFS_BUFATOFF(ip: &Inode, offset: Off) -> Result<(&'static Buf, usize), Errno> {
    ip.vtbl()
        .bufatoff(ip, offset)
        .map(|(b, off)| (lent(b), off))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ext2fs")]
    #[test]
    fn the_ext2fs_members_read_and_write_the_glues_dinode() {
        let g = BsdInodeG::default();
        assert!(g.e2din_is_null());
        let din: &'static mut Ext2fsDinode =
            std::boxed::Box::leak(std::boxed::Box::new(Ext2fsDinode::default()));
        g.i_e2din.set(core::ptr::from_mut(din));
        assert!(!g.e2din_is_null());
        g.set_i_e2fs_mode(0o100600);
        g.set_i_e2fs_nlink(2);
        g.set_i_e2fs_block(3, 77);
        assert_eq!((g.i_e2fs_mode(), g.i_e2fs_nlink()), (0o100600, 2));
        assert_eq!(g.i_e2fs_blocks()[3], 77);
    }
}
/* </TESTS> */
