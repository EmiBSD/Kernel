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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)ufsmount.h	8.4 (Berkeley) 10/27/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ufs/ufsmount.h>`: the UFS specific mount structure data (`struct ufsmount`), hung
//! from a UFS mount's `mnt_data`.
//!
//! Upstream: sys/ufs/ufs/ufsmount.h @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufsmount.rs@3c62ede68802
//!
//! ## Deviations
//! - `ufsmount_u` (the super-block pointer of FFS or EXT2FS): FFS's is the structure's own
//!   since N5.5b1, a `Superblock` (`fs::ufs::ffs::fs`) set once by the kernel's `ffs_mountfs`
//!   ([`Ufsmount::um_fs`], [`Ufsmount::set_um_fs`]), held inline as the C holds the pointer;
//!   unset on an EXT2FS mount, whose `um_e2fs` is the glue's member [`Ufsmount::g`]
//!   (`FileGlue::UfsmountG`, docs/SCAFFOLDING.md) until N5.6. Until N5.5b2 switches ffs,
//!   `bsd`'s ffs keeps its own `um_fs` in the glue's member and leaves this one unset.
//!   `um_export` (`struct netexport`, the
//!   NFS server's) is the mount's glue member since N5.5b0 (`Mount::g`, `bsd`'s `BsdMountG`,
//!   until N6): `bsd`'s `vfs_export` and `vfs_export_lookup` read it from the mount, which all
//!   three of its readers hold. The quota members (`um_quotas`, `um_cred`, `um_btime`,
//!   `um_itime`, `um_qflags`) are the structure's own since N5.5a2.
//! - The block is the structure and the count of its handles ([`UfsmountRef`], an
//!   `hal::malloc::MArc`): 16 bytes more per mount in `vmstat -m`'s `UFS mount` row than the
//!   C's (the request counts are the C's).
//! - `MNINDIR`, `blkptrtodb` and `is_sequential` are functions with the macros' names in lower
//!   case.
//!
//! ## Redesign
//! - LZ's `Cell` members under one `unsafe impl Sync` -> relaxed atomic words with LZ's
//!   `get`/`set` (`inode.rs`'s [`U64Word`], ...): `ffs_mountfs` fills them in the order the C
//!   does (some after the structure is hung from the mount), `ffs_reload` changes
//!   `um_maxsymlinklen`; `um_mountp` and `um_devvp` are `Link`s (the mount and the device
//!   vnode are never freed). The `unsafe impl Sync` goes with the members it covered, to the
//!   glue's member.
//! - N5.5b0 (D-N5.5b-1, the user's): LZ's raw `malloc(M_UFSMNT)` written with `ptr::write`
//!   into `mnt_data` (sys/ufs/ffs/ffs_vfsops.rs:894-901, sys/ufs/ext2fs/ext2fs_vfsops.rs:
//!   670-677) and cast back by `bsd`'s `vfstoufs` behind a type-name check (an `unsafe` deref
//!   giving every caller and every inode a `&'static`) -> [`Ufsmount::alloc`], a counted
//!   `malloc(M_UFSMNT)` block ([`UfsmountRef`]) that `ffs_mountfs`/`ext2fs_mountfs` erase into
//!   the mount's typed slot (`Mount::set_node`) where the C sets `mnt_data`, and that the
//!   unmount takes back after `vflush` (`Mount::take_node`) where the C frees it. [`vfstoufs`]
//!   counts a handle out of the slot (the type checked by the slot's downcast, LZ's panic
//!   kept); each inode holds one for its life (`i_ump`); the last handle frees the block. So
//!   the structure outlives every holder by its count, where LZ's `&'static` rested on the
//!   unmount's `vflush` reclaiming every inode first. The cost against the C's one load of
//!   `mnt_data`: the slot's read and a count pair per `VFSTOUFS` (the `Vfs` operations, not
//!   read, write or lookup), one count pair per inode life.
//! - N5.5b1: `ufsmount_u.fs` (`um_fs`, the C's pointer to a `malloc`ed super-block) -> a
//!   set-once `Superblock` held inline ([`Ufsmount::um_fs`]): the super-block's own `malloc`
//!   is its words' array (`ffs/fs.rs`), so the `UFS mount` row's requests stay the C's, and
//!   it is freed with the structure's last handle, as the unmount frees both.

use core::sync::atomic::{AtomicI64, AtomicU8};

use hal::link::Link;
use hal::malloc::{M_WAITOK, MArc};
use hal::once::OnceStatic;
use hal::slot::ReadMostly;
use hal::sys::types::Daddr;

use super::inode::{I32Word, U32Word, U64Word};
use super::quota::MAXQUOTAS;
use crate::assert::kpanic;
use crate::cred::CredRef;
use crate::file::FileGlue;
use crate::fs::mount::Mount;
use crate::fs::ufs::ffs::fs::Superblock;
use crate::fs::vnode::Vnode;
use crate::proc::ProcGlue;

/// `struct ufsmount`: the UFS specific mount structure data.
pub struct Ufsmount<G: ProcGlue> {
    /// `um_mountp`: filesystem vfs structure.
    pub um_mountp: Link<Mount<G>>,
    /// `um_dev`: device mounted.
    pub um_dev: I32Word,
    /// `um_devvp`: block device mounted vnode.
    pub um_devvp: Link<Vnode<G>>,
    /// `um_fstype`: type of file system (`UM_UFS1`, `UM_UFS2`, `UM_EXT2FS`).
    pub um_fstype: U64Word,
    /// `um_nindir`: indirect ptrs per block.
    pub um_nindir: U64Word,
    /// `um_bptrtodb`: indir ptr to disk block.
    pub um_bptrtodb: U64Word,
    /// `um_seqinc`: inc between seq blocks.
    pub um_seqinc: U64Word,
    /// `um_savedmaxfilesize`: XXX - limit maxfilesize.
    pub um_savedmaxfilesize: U64Word,
    /// `um_maxsymlinklen`: max size of short symlink.
    pub um_maxsymlinklen: U32Word,
    /// `um_quotas`: pointer to quota files.
    pub um_quotas: [Link<Vnode<G>>; MAXQUOTAS],
    /// `um_cred`: quota file access cred, the mount's reference while the quota file is open
    /// (`None`, the C's NULL or `NOCRED`, while it is not).
    pub um_cred: [ReadMostly<Option<CredRef>>; MAXQUOTAS],
    /// `um_btime`: block quota time limit.
    pub um_btime: [AtomicI64; MAXQUOTAS],
    /// `um_itime`: inode quota time limit.
    pub um_itime: [AtomicI64; MAXQUOTAS],
    /// `um_qflags`: quota specific flags.
    pub um_qflags: [AtomicU8; MAXQUOTAS],
    /// `ufsmount_u.fs`: FFS's in-core super-block, set once by `ffs_mountfs`.
    fs: OnceStatic<Superblock>,
    /// The members of `bsd`'s types (`ufsmount_u`; the module's deviations).
    pub g: <G::FileGlue as FileGlue>::UfsmountG,
}

/// `struct ufsmount *` held: a counted handle to a mount's UFS structure, in a block of
/// `malloc(M_UFSMNT)` that the last handle frees. The mount's typed slot holds one while it is
/// mounted, each inode one for its life, each operation reached from the mount one
/// ([`vfstoufs`]).
pub type UfsmountRef<G> = MArc<Ufsmount<G>>;

impl<G: ProcGlue> Ufsmount<G> {
    /// `ump = malloc(sizeof(*ump), M_UFSMNT, M_WAITOK | M_ZERO)` (`ffs_mountfs`,
    /// `ext2fs_mountfs`): a zeroed structure in its counted block, one handle, which the mount
    /// hangs from its typed slot (`Mount::set_node`). `M_WAITOK` sleeps for the memory and
    /// never fails, as the C's; no `M_ZERO`: the block is written whole with [`Ufsmount::new`].
    pub fn alloc() -> UfsmountRef<G> {
        match MArc::new(M_UFSMNT, M_WAITOK, Self::new()) {
            Some(ump) => ump,
            None => kpanic(format_args!("ufsmount: malloc(M_UFSMNT) failed")),
        }
    }

    /// A zeroed structure, as `malloc(M_ZERO)` returns it.
    pub fn new() -> Self {
        Self {
            um_mountp: Link::new(),
            um_dev: I32Word::new(0),
            um_devvp: Link::new(),
            um_fstype: U64Word::new(0),
            um_nindir: U64Word::new(0),
            um_bptrtodb: U64Word::new(0),
            um_seqinc: U64Word::new(0),
            um_savedmaxfilesize: U64Word::new(0),
            um_maxsymlinklen: U32Word::new(0),
            um_quotas: [const { Link::new() }; MAXQUOTAS],
            um_cred: [const { ReadMostly::new(None) }; MAXQUOTAS],
            um_btime: [const { AtomicI64::new(0) }; MAXQUOTAS],
            um_itime: [const { AtomicI64::new(0) }; MAXQUOTAS],
            um_qflags: [const { AtomicU8::new(0) }; MAXQUOTAS],
            fs: OnceStatic::new(),
            g: Default::default(),
        }
    }

    /// `ump->um_devvp`, which every mounted UFS has.
    pub fn devvp(&self) -> &'static Vnode<G> {
        match self.um_devvp.get() {
            Some(vp) => vp,
            None => kpanic(format_args!("ufsmount {self:p}: no um_devvp")),
        }
    }

    /// `ump->um_mountp`.
    pub fn mountp(&self) -> &'static Mount<G> {
        match self.um_mountp.get() {
            Some(mp) => mp,
            None => kpanic(format_args!("ufsmount {self:p}: no um_mountp")),
        }
    }

    /// `ump->um_fs`: the FFS super-block in core; `panic(9)` on a mount without one (EXT2FS's,
    /// whose `ufsmount_u` the C reads as `um_e2fs`).
    #[inline]
    pub fn um_fs(&self) -> &Superblock {
        match self.fs.get() {
            Some(fs) => fs,
            None => kpanic(format_args!("ufsmount {self:p}: no um_fs")),
        }
    }

    /// `ump->um_fs = fs`, once (`ffs_mountfs`); `panic(9)` on a second.
    pub fn set_um_fs(&self, fs: Superblock) {
        if self.fs.set(fs).is_err() {
            kpanic(format_args!("ufsmount {self:p}: a second um_fs"));
        }
    }
}

impl<G: ProcGlue> Default for Ufsmount<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `M_UFSMNT`: the malloc type of the UFS mount structures and the inode hash table
/// (`<sys/malloc.h>`; `vmstat -m`'s `UFS mount`).
pub const M_UFSMNT: i32 = 28;

/// `UM_UFS1`: filesystem type.
pub const UM_UFS1: u64 = 1;
/// `UM_UFS2`.
pub const UM_UFS2: u64 = 2;
/// `UM_EXT2FS`.
pub const UM_EXT2FS: u64 = 3;

/// `QTF_OPENING`: Q_QUOTAON in progress.
pub const QTF_OPENING: u8 = 0x01;
/// `QTF_CLOSING`: Q_QUOTAOFF in progress.
pub const QTF_CLOSING: u8 = 0x02;

/// `VFSTOUFS(mp)`: convert mount ptr to ufsmount ptr, held: a counted handle out of the
/// mount's typed slot, which the caller may keep across the operation's sleeps. `panic(9)`
/// when the mount holds no UFS structure (LZ's check: an unmounted or a non-UFS mount).
pub fn vfstoufs<G: ProcGlue>(mp: &Mount<G>) -> UfsmountRef<G> {
    match mp.node_arc::<Ufsmount<G>>() {
        Some(ump) => ump,
        None => kpanic(format_args!("VFSTOUFS: mount {mp:p} is not a mounted UFS")),
    }
}

/// `MNINDIR(ump)`: indirect pointers per block.
pub fn mnindir<G: ProcGlue>(ump: &Ufsmount<G>) -> u64 {
    ump.um_nindir.get()
}

/// `blkptrtodb(ump, b)`: a block pointer to a disk block.
pub fn blkptrtodb<G: ProcGlue>(ump: &Ufsmount<G>, b: Daddr) -> Daddr {
    b << ump.um_bptrtodb.get()
}

/// `is_sequential(ump, a, b)`: whether `b` follows `a` on the disk.
pub fn is_sequential<G: ProcGlue>(ump: &Ufsmount<G>, a: Daddr, b: Daddr) -> bool {
    b == a + ump.um_seqinc.get() as Daddr
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::testfs::TestGlue;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ufs/ufsmount.h");
        for (name, value) in [
            ("UM_UFS1", UM_UFS1),
            ("UM_UFS2", UM_UFS2),
            ("UM_EXT2FS", UM_EXT2FS),
            ("QTF_OPENING", u64::from(QTF_OPENING)),
            ("QTF_CLOSING", u64::from(QTF_CLOSING)),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(value as i64),
                "{name}"
            );
        }
    }

    /// `VFSTOUFS` of a mount whose slot holds the structure `ffs_mountfs` hung there: a
    /// counted handle to it; the unmount's take leaves the inodes' handles valid.
    #[test]
    fn vfstoufs_counts_a_handle_out_of_the_mounts_slot() {
        let _g = crate::fs::testfs::setup();
        let mp: &'static Mount<TestGlue> =
            std::boxed::Box::leak(std::boxed::Box::new(Mount::new_unpooled(None)));
        let ump = Ufsmount::<TestGlue>::alloc();
        assert_eq!(MArc::ty(&ump), M_UFSMNT);
        ump.um_nindir.set(4096);
        mp.set_node(MArc::erase(ump.clone()));
        let held = vfstoufs(mp);
        assert!(MArc::ptr_eq(&held, &ump));
        assert_eq!(mnindir(&held), 4096);
        drop(mp.take_node());
        assert_eq!(
            held.um_nindir.get(),
            4096,
            "a holder's handle outlives the unmount"
        );
    }

    #[test]
    #[should_panic(expected = "is not a mounted UFS")]
    fn vfstoufs_of_a_mount_without_the_structure_is_a_panic() {
        let _g = crate::fs::testfs::setup();
        let mp: &'static Mount<TestGlue> =
            std::boxed::Box::leak(std::boxed::Box::new(Mount::new_unpooled(None)));
        mp.set_node(MArc::erase(MArc::new(M_UFSMNT, M_WAITOK, 5u64).unwrap()));
        let _ = vfstoufs(mp);
    }

    #[test]
    fn the_words_give_the_macros() {
        let ump = Ufsmount::<TestGlue>::new();
        ump.um_nindir.set(2048);
        ump.um_bptrtodb.set(4);
        ump.um_seqinc.set(8);
        assert_eq!(mnindir(&ump), 2048);
        assert_eq!(blkptrtodb(&ump, 3), 48);
        assert!(is_sequential(&ump, 100, 108));
        assert!(!is_sequential(&ump, 100, 101));
        assert!(ump.um_mountp.get().is_none());
    }
}
/* </TESTS> */
