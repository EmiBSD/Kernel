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
//! `<msdosfs/denode.h>`: the pc filesystem specific portion of the vnode structure (`struct
//! denode`), its fat cache, the transfer of directory entries between the internal and the
//! external form, and the msdosfs file handle.
//!
//! Upstream: sys/msdosfs/denode.h @ 3ce1f3f79392
//! LZ: sys/msdosfs/denode.rs@3c62ede68802
//!
//! To describe a file uniquely the `de_dirclust`, `de_diroffset`, and `de_StartCluster` fields
//! are used. `de_dirclust` contains the cluster number of the directory cluster containing the
//! entry for a file or directory. `de_diroffset` is the index into the cluster for the entry
//! describing a file or directory. `de_StartCluster` is the number of the first cluster of the
//! file or directory.
//!
//! Now to describe the quirks of the pc filesystem.
//! - Clusters 0 and 1 are reserved.
//! - The first allocatable cluster is 2.
//! - The root directory is of fixed size and all blocks that make it up are contiguous.
//! - Cluster 0 refers to the root directory when it is found in the startcluster field of a
//!   directory entry that points to another directory.
//! - Cluster 0 implies a 0 length file when found in the start cluster field of a directory
//!   entry that points to a file.
//! - You can't use the cluster number 0 to derive the address of the root directory.
//! - Multiple directory entries can point to a directory. The entry in the parent directory
//!   points to a child directory. Any directories in the child directory contain a ".." entry
//!   that points back to the parent. The child directory itself contains a "." entry that
//!   points to itself.
//! - The root directory does not contain a "." or ".." entry.
//! - Directory entries for directories are never changed once they are created (except when
//!   removed). The size stays 0, and the last modification time is never changed. This is
//!   because so many directory entries can point to the physical clusters that make up a
//!   directory. It would lead to an update nightmare.
//! - The length field in a directory entry pointing to a directory contains 0 (always). The
//!   only way to find the end of a directory is to follow the cluster chain until the "last
//!   cluster" marker is found.
//!
//! My extensions to make this house of cards work. These apply only to the in memory copy of
//! the directory entry.
//! - A reference count for each denode will be kept since dos doesn't keep such things.
//!
//! ## Deviations
//! - The hand-made hash chain (`de_next`, and `de_prev` pointing at the previous link) is a
//!   pinned list entry, `de_hash`, of the denode hash's chains (`msdosfs_denode.rs`, N5.7c2),
//!   with the chain's index (`de_chain`, what `de_prev` knows of its chain); `de_prev ==
//!   NULL` (a denode on no chain) is the entry's own state.
//! - `de_Name` is eleven byte words read and written whole ([`Denode::name`],
//!   [`Denode::set_name`]); each `de_fc` slot is one 64-bit word holding `fc_frcn` and
//!   `fc_fsrcn` ([`Denode::fc`], [`fc_setcache`]).
//! - `fc_setcache`, `DE_INTERNALIZE32`, `DE_INTERNALIZE`, `DE_EXTERNALIZE`, `DETIMES`, `VTODE`
//!   are functions with the macros' names in lower case; `DETOV` is [`Denode::detov`]. The
//!   on-disk entry is the `Direntry` value `direntry.rs` copies out of the directory block
//!   and its caller writes back.
//! - `de_forw`/`de_back` name a `de_chain` member the structure does not have; they are dead
//!   macros and are left out.
//! - `de_lock` is a `hal::rwlock::RecursiveRwlock` (the C's `rrw_init_flags(.., RWL_DUPOK |
//!   RWL_IS_VNODE)`: both are `WITNESS` flags, which the kernel's lock has none of, as the
//!   UFS inode's).
//! - The vnode operation prototypes (`msdosfs_lookup`, ...) are the functions of
//!   `msdosfs_vnops.rs`, except `msdosfs_inactive` and `msdosfs_reclaim`, which the C defines
//!   in `msdosfs_denode.c`; the internal service routines are in `msdosfs_denode.rs`
//!   (`deget`, `deupdat`, `detrunc`, `deextend`, `reinsert`) and `msdosfs_lookup.rs` (the
//!   rest).
//!
//! ## Redesign
//! - LZ's `Denode` (`Cell` members under an `unsafe impl Sync`, `malloc(M_MSDOSFSNODE)`
//!   written with `ptr::write` into `v_data` by `deget` and reached by `vtode`'s `unsafe`
//!   deref behind a tag check as a `&'static Denode`, freed by `msdosfs_reclaim` without a
//!   drop, so `de_lockf` sat in a `ManuallyDrop`) -> a [`Denode`] made whole where the C fills
//!   it ([`Denode::new`]: the identity `de_vnode`, `de_dev`, `de_pmp` and the entry's position
//!   given, the rest zero as `M_ZERO` leaves it) and published in a counted
//!   `malloc(M_MSDOSFSNODE)` block ([`DenodeRef`], `hal::malloc::MArc`; `vmstat -m`'s `MSDOSFS
//!   node` row, the C's requests; a typed pool would add a pool row userland sees, D-N5.7-2)
//!   erased into the vnode's typed slot (`Vnode::set_node` in `deget`, where the C sets
//!   `v_data`; `take_node` in `msdosfs_reclaim`, mfs's precedent): the last handle frees the
//!   block and drops its members, so a handle kept across `bread` never dangles and
//!   `de_lockf` is a plain `LockfSlot` (docs/IDIOMS.md row 191 loses its one user).
//!   [`vtode`] is the slot's typed read and a count (the type replaces LZ's tag check; cost
//!   against the C's one load: the slot's read, two atomics, and a count pair per operation,
//!   ROADMAP N12's row).
//! - The members the C changes under the denode's lock and the kernel lock are relaxed words
//!   (`fs::word`; docs/IDIOMS.md's counters' row: every msdosfs path is `KERNEL_LOCK`ed, as
//!   ufs's): `de_flag`, `de_dirclust`/`de_diroffset` (`reinsert` moves a renamed denode),
//!   the lookup's `de_fndoffset`/`de_fndcnt`, `de_refcnt`, the entry's members, the fat cache.
//!   `de_devvp` is a `hal::link::Link` (`None` until `deget` has hashed the denode, taken by
//!   `msdosfs_reclaim` for its `vrele`). `de_pmp` is a counted handle of the mount structure
//!   held for the denode's life, as `i_ump` ([`Denode::pmp`] borrows it: one load, the C's).
//! - The template denode `msdosfs_create` and `msdosfs_mkdir` fill on the stack (`struct
//!   denode ndirent`, `bzero`ed) is a [`Denode::new`] value with no vnode, never hashed nor
//!   published; `createde` reads it as the C does (a count pair of `de_pmp` per create,
//!   ROADMAP N12's row).

use hal::link::Link;
use hal::malloc::{M_WAITOK, MArc};
use hal::pinlist::PinEntry;
use hal::rwlock::RecursiveRwlock;
use hal::sys::mount::{Fid, MSDOSFSMNT_NOWIN95};
use hal::sys::types::Dev;

use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::fs::lockf::LockfSlot;
use crate::fs::msdosfs::bpb::{getulong, getushort, putulong, putushort};
use crate::fs::msdosfs::direntry::{
    ATTR_ARCHIVE, ATTR_DIRECTORY, CASE_LOWER_BASE, CASE_LOWER_EXT, Direntry,
};
use crate::fs::msdosfs::fat::fat32;
use crate::fs::msdosfs::msdosfs_conv::unix2dostime;
use crate::fs::msdosfs::msdosfsmount::{Msdosfsmount, MsdosfsmountRef};
use crate::fs::vnode::{VT_MSDOSFS, Vnode};
use crate::fs::word::{I32Word, I64Word, U8Word, U16Word, U32Word, U64Word};
use crate::proc::ProcGlue;

/// `M_MSDOSFSNODE`: the malloc type of the denodes (`<sys/malloc.h>`; `vmstat -m`'s
/// `MSDOSFS node`).
pub const M_MSDOSFSNODE: i32 = 61;

/// `MSDOSFSROOT_OFS`: internal pseudo-offset for (nonexistent) directory entry for the root
/// dir in the root dir.
pub const MSDOSFSROOT_OFS: u32 = 0x1fffffff;

/// `FC_SIZE`: number of entries in the cache.
pub const FC_SIZE: usize = 3;
/// `FC_LASTMAP`: entry the last call to `pcbmap()` resolved to.
pub const FC_LASTMAP: usize = 0;
/// `FC_LASTFC`: entry for the last cluster in the file.
pub const FC_LASTFC: usize = 1;
/// `FC_OLASTFC`: entry for the previous last cluster.
pub const FC_OLASTFC: usize = 2;

/// `FCE_EMPTY`: doesn't represent an actual cluster #.
pub const FCE_EMPTY: u32 = 0xffffffff;

/// `DE_UPDATE`: modification time update request.
pub const DE_UPDATE: u32 = 0x0004;
/// `DE_CREATE`: creation time update.
pub const DE_CREATE: u32 = 0x0008;
/// `DE_ACCESS`: access time update.
pub const DE_ACCESS: u32 = 0x0010;
/// `DE_MODIFIED`: denode has been modified.
pub const DE_MODIFIED: u32 = 0x0020;
/// `DE_RENAME`: denode is in the process of being renamed.
pub const DE_RENAME: u32 = 0x0040;

/// `WIN_MAXLEN`: maximum filename length in Win95. Note: must be < `sizeof(dirent.d_name)`.
pub const WIN_MAXLEN: usize = 255;

/// `MSDOSFS_FILESIZE_MAX`: maximum size of a file on a FAT filesystem.
pub const MSDOSFS_FILESIZE_MAX: i64 = 0xFFFFFFFF;

/// `struct fatcache`: the fat cache structure. `fc_fsrcn` is the filesystem relative cluster
/// number that corresponds to the file relative cluster number in this structure (`fc_frcn`).
///
/// The fat entry cache as it stands helps make extending files a "quick" operation by avoiding
/// having to scan the fat to discover the last cluster of the file. The cache also helps
/// sequential reads by remembering the last cluster read from the file. This also prevents us
/// from having to rescan the fat to find the next cluster to read. This cache is probably
/// pretty worthless if a file is opened by multiple processes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fatcache {
    /// `fc_frcn`: file relative cluster number.
    pub fc_frcn: u32,
    /// `fc_fsrcn`: filesystem relative cluster number.
    pub fc_fsrcn: u32,
}

impl Fatcache {
    /// The slot's word: `fc_frcn` in the high half, `fc_fsrcn` in the low one.
    const fn word(self) -> u64 {
        (self.fc_frcn as u64) << 32 | self.fc_fsrcn as u64
    }

    /// The slot that `word` holds.
    const fn from_word(w: u64) -> Self {
        Self {
            fc_frcn: (w >> 32) as u32,
            fc_fsrcn: w as u32,
        }
    }
}

/// `struct denode`: the in memory variant of a dos directory entry. It is usually contained
/// within a vnode.
///
/// Protected by: the denode's lock (`de_lock`), held through its vnode, and the kernel lock
/// (every word's writer holds it); the hash chain by the kernel lock. The identity (`de_vnode`,
/// `de_dev`, `de_pmp`) is set when the value is made and never changes.
#[allow(non_snake_case)] // the C field names
pub struct Denode<G: ProcGlue> {
    /// `de_next`/`de_prev`: hash chain.
    pub de_hash: PinEntry<Denode<G>>,
    /// The chain `msdosfs_hashins` put `de_hash` on (the C's `de_prev` points into it): the
    /// entry's position changes before `reinsert` takes the denode off its old chain.
    pub de_chain: U32Word,
    /// `de_vnode`: addr of vnode we are part of (`None` for the template a create fills).
    pub de_vnode: Option<&'static Vnode<G>>,
    /// `de_devvp`: vnode of blk dev we live on.
    pub de_devvp: Link<Vnode<G>>,
    /// `de_flag`: flag bits (`DE_*`).
    pub de_flag: U32Word,
    /// `de_dev`: device where direntry lives.
    pub de_dev: Dev,
    /// `de_dirclust`: cluster of the directory file containing this entry.
    pub de_dirclust: U32Word,
    /// `de_diroffset`: offset of this entry in the directory cluster.
    pub de_diroffset: U32Word,
    /// `de_fndoffset`: offset of found dir entry.
    pub de_fndoffset: U32Word,
    /// `de_fndcnt`: number of slots before `de_fndoffset`.
    pub de_fndcnt: I32Word,
    /// `de_refcnt`: reference count.
    pub de_refcnt: I64Word,
    /// `de_pmp`: addr of our mount struct, a counted handle held for the denode's life.
    pub de_pmp: MsdosfsmountRef<G>,
    /// `de_lockf`: byte level lock list.
    pub de_lockf: LockfSlot,
    /// `de_lock`: denode lock.
    pub de_lock: RecursiveRwlock,
    /// `de_Name`: name, from DOS directory entry ([`Denode::name`]).
    pub de_Name: [U8Word; 11],
    /// `de_Attributes`: attributes, from directory entry.
    pub de_Attributes: U8Word,
    /// `de_CTimeHundredth`: creation time, 1/100th of a sec.
    pub de_CTimeHundredth: U8Word,
    /// `de_CTime`: creation time.
    pub de_CTime: U16Word,
    /// `de_CDate`: creation date.
    pub de_CDate: U16Word,
    /// `de_ADate`: access date.
    pub de_ADate: U16Word,
    /// `de_MTime`: modification time.
    pub de_MTime: U16Word,
    /// `de_MDate`: modification date.
    pub de_MDate: U16Word,
    /// `de_StartCluster`: starting cluster of file.
    pub de_StartCluster: U32Word,
    /// `de_FileSize`: size of file in bytes.
    pub de_FileSize: U32Word,
    /// `de_fc`: fat cache, a slot a word ([`Denode::fc`]).
    pub de_fc: [U64Word; FC_SIZE],
}

/// `struct denode *` held: a counted handle to a denode, in a block of
/// `malloc(M_MSDOSFSNODE)` that the last handle frees. The vnode's typed slot holds one until
/// `msdosfs_reclaim`, each operation reached from the vnode one ([`vtode`]).
pub type DenodeRef<G> = MArc<Denode<G>>;

impl<G: ProcGlue> Denode<G> {
    /// A denode of the mount `pmp` on the device `dev` for the directory entry at `dirclust`,
    /// `diroffset`, hung on the vnode `vp` (none for the template of a create), every other
    /// member zero as the C's `malloc(M_ZERO)` (`deget`) or `bzero` (`struct denode ndirent`)
    /// leaves it; the caller fills the rest as the C does (`deget`'s `fc_purge`, the entry's
    /// members).
    pub fn new(
        vp: Option<&'static Vnode<G>>,
        dev: Dev,
        pmp: MsdosfsmountRef<G>,
        dirclust: u32,
        diroffset: u32,
    ) -> Self {
        Self {
            de_hash: PinEntry::new(),
            de_chain: U32Word::new(0),
            de_vnode: vp,
            de_devvp: Link::new(),
            de_flag: U32Word::new(0),
            de_dev: dev,
            de_dirclust: U32Word::new(dirclust),
            de_diroffset: U32Word::new(diroffset),
            de_fndoffset: U32Word::new(0),
            de_fndcnt: I32Word::new(0),
            de_refcnt: I64Word::new(0),
            de_pmp: pmp,
            de_lockf: LockfSlot::new(),
            de_lock: RecursiveRwlock::new("denode"),
            de_Name: [const { U8Word::new(0) }; 11],
            de_Attributes: U8Word::new(0),
            de_CTimeHundredth: U8Word::new(0),
            de_CTime: U16Word::new(0),
            de_CDate: U16Word::new(0),
            de_ADate: U16Word::new(0),
            de_MTime: U16Word::new(0),
            de_MDate: U16Word::new(0),
            de_StartCluster: U32Word::new(0),
            de_FileSize: U32Word::new(0),
            de_fc: [const { U64Word::new(0) }; FC_SIZE],
        }
    }

    /// `ldep = malloc(sizeof(*ldep), M_MSDOSFSNODE, M_WAITOK | M_ZERO)` holding `self`, one
    /// handle, which `deget` hangs from the vnode's typed slot (`Vnode::set_node`). `M_WAITOK`
    /// sleeps for the memory and never fails, as the C's.
    pub fn alloc(self) -> DenodeRef<G> {
        match MArc::new(M_MSDOSFSNODE, M_WAITOK, self) {
            Some(dep) => dep,
            None => kpanic(format_args!("deget: malloc(M_MSDOSFSNODE) failed")),
        }
    }

    /// `DETOV(de)`: the denode's vnode. `panic(9)` for the template of a create, which has
    /// none.
    pub fn detov(&self) -> &'static Vnode<G> {
        match self.de_vnode {
            Some(vp) => vp,
            None => kpanic(format_args!("denode {self:p}: no vnode")),
        }
    }

    /// `dep->de_pmp`: the mount structure, borrowed from the denode's handle.
    #[inline]
    pub fn pmp(&self) -> &Msdosfsmount<G> {
        &self.de_pmp
    }

    /// `dep->de_flag |= f`.
    pub fn set_flag(&self, f: u32) {
        self.de_flag.set(self.de_flag.get() | f);
    }

    /// `dep->de_flag &= ~f`.
    pub fn clr_flag(&self, f: u32) {
        self.de_flag.set(self.de_flag.get() & !f);
    }

    /// `dep->de_fc[slot]`.
    pub fn fc(&self, slot: usize) -> Fatcache {
        Fatcache::from_word(self.de_fc[slot].get())
    }

    /// `dep->de_Name`: the 11 bytes of the name and the extension.
    pub fn name(&self) -> [u8; 11] {
        core::array::from_fn(|i| self.de_Name[i].get())
    }

    /// `bcopy(name, dep->de_Name, 11)`.
    pub fn set_name(&self, n: &[u8; 11]) {
        for (w, &b) in self.de_Name.iter().zip(n) {
            w.set(b);
        }
    }
}

/// `struct defid`: this overlays the fid structure (see mount.h).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Defid {
    /// `defid_len`: length of structure.
    pub defid_len: u16,
    /// `defid_pad`: force long alignment.
    pub defid_pad: u16,
    /// `defid_dirclust`: cluster this dir entry came from.
    pub defid_dirclust: u32,
    /// `defid_dirofs`: offset of entry within the cluster.
    pub defid_dirofs: u32,
    // `defid_gen` (generation number) is `#if 0` in the C.
}

impl Defid {
    /// The `struct defid` a `struct fid` holds (`(struct defid *)fhp`).
    pub fn from_fid(fid: &Fid) -> Self {
        let d = &fid.fid_data;
        Self {
            defid_len: fid.fid_len,
            defid_pad: fid.fid_reserved,
            defid_dirclust: u32::from_ne_bytes([d[0], d[1], d[2], d[3]]),
            defid_dirofs: u32::from_ne_bytes([d[4], d[5], d[6], d[7]]),
        }
    }

    /// Writes the `struct defid` over `fid`.
    pub fn to_fid(&self, fid: &mut Fid) {
        fid.fid_len = self.defid_len;
        fid.fid_reserved = self.defid_pad;
        fid.fid_data[0..4].copy_from_slice(&self.defid_dirclust.to_ne_bytes());
        fid.fid_data[4..8].copy_from_slice(&self.defid_dirofs.to_ne_bytes());
    }
}

/// `fc_setcache(dep, slot, frcn, fsrcn)`: set a slot in the fat cache.
pub fn fc_setcache<G: ProcGlue>(dep: &Denode<G>, slot: usize, frcn: u32, fsrcn: u32) {
    dep.de_fc[slot].set(
        Fatcache {
            fc_frcn: frcn,
            fc_fsrcn: fsrcn,
        }
        .word(),
    );
}

/// `DE_INTERNALIZE32(dep, dp)`: the high half of a FAT32 start cluster.
pub fn de_internalize32<G: ProcGlue>(dep: &Denode<G>, dp: &Direntry) {
    dep.de_StartCluster
        .set(dep.de_StartCluster.get() | (u32::from(getushort(&dp.deHighClust)) << 16));
}

/// `DE_INTERNALIZE(dep, dp)`: transfer directory entry `dp` (external form) into denode `dep`
/// (internal form).
pub fn de_internalize<G: ProcGlue>(dep: &Denode<G>, dp: &Direntry) {
    dep.set_name(&dp.name11());
    dep.de_Attributes.set(dp.deAttributes);
    dep.de_CTimeHundredth.set(dp.deCTimeHundredth);
    dep.de_CTime.set(getushort(&dp.deCTime));
    dep.de_CDate.set(getushort(&dp.deCDate));
    dep.de_ADate.set(getushort(&dp.deADate));
    dep.de_MTime.set(getushort(&dp.deMTime));
    dep.de_MDate.set(getushort(&dp.deMDate));
    dep.de_StartCluster
        .set(u32::from(getushort(&dp.deStartCluster)));
    dep.de_FileSize.set(getulong(&dp.deFileSize));
    if fat32(dep.pmp()) {
        de_internalize32(dep, dp);
    }
}

/// `DE_EXTERNALIZE(dp, dep)`: transfer denode `dep` (internal form) into directory entry `dp`
/// (external form).
pub fn de_externalize<G: ProcGlue>(dp: &mut Direntry, dep: &Denode<G>) {
    dp.set_name11(&dep.name());
    dp.deAttributes = dep.de_Attributes.get();
    dp.deLowerCase = CASE_LOWER_BASE | CASE_LOWER_EXT;
    dp.deCTimeHundredth = dep.de_CTimeHundredth.get();
    putushort(&mut dp.deCTime, dep.de_CTime.get());
    putushort(&mut dp.deCDate, dep.de_CDate.get());
    putushort(&mut dp.deADate, dep.de_ADate.get());
    putushort(&mut dp.deMTime, dep.de_MTime.get());
    putushort(&mut dp.deMDate, dep.de_MDate.get());
    putushort(&mut dp.deStartCluster, dep.de_StartCluster.get() as u16);
    let size = if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        0
    } else {
        dep.de_FileSize.get()
    };
    putulong(&mut dp.deFileSize, size);
    let high = if fat32(dep.pmp()) {
        (dep.de_StartCluster.get() >> 16) as u16
    } else {
        0
    };
    putushort(&mut dp.deHighClust, high);
}

/// `VTODE(vp)`: the denode of a msdosfs vnode, held. `panic(9)` when the vnode is not a
/// msdosfs vnode with its denode (LZ's check, made by the slot's type and the tag).
pub fn vtode<G: ProcGlue>(vp: &Vnode<G>) -> DenodeRef<G> {
    match vp.node_arc::<Denode<G>>() {
        Some(dep) if vp.tag() == VT_MSDOSFS => dep,
        _ => kpanic(format_args!("VTODE: vnode {vp:p} has no denode")),
    }
}

/// `DETIMES(dep, acc, mod, cre)`: turn the pending time update requests of `dep` into its
/// DOS times (`acc` for the access, `mod` for the modification, `cre` for the creation).
pub fn detimes<G: ProcGlue>(dep: &Denode<G>, acc: &Timespec, r#mod: &Timespec, cre: &Timespec) {
    if dep.de_flag.get() & (DE_UPDATE | DE_CREATE | DE_ACCESS) == 0 {
        return;
    }
    dep.set_flag(DE_MODIFIED);
    if dep.de_flag.get() & DE_UPDATE != 0 {
        let (dd, dt, _) = unix2dostime(r#mod);
        dep.de_MDate.set(dd);
        dep.de_MTime.set(dt);
        dep.de_Attributes
            .set(dep.de_Attributes.get() | ATTR_ARCHIVE);
    }
    if dep.pmp().pm_flags.get() & MSDOSFSMNT_NOWIN95 as u32 == 0 {
        if dep.de_flag.get() & DE_ACCESS != 0 {
            let (dd, _, _) = unix2dostime(acc);
            dep.de_ADate.set(dd);
        }
        if dep.de_flag.get() & DE_CREATE != 0 {
            let (dd, dt, dh) = unix2dostime(cre);
            dep.de_CDate.set(dd);
            dep.de_CTime.set(dt);
            dep.de_CTimeHundredth.set(dh);
        }
    }
    dep.clr_flag(DE_UPDATE | DE_CREATE | DE_ACCESS);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of `denode.rs`: the transfer of directory entries between the on-disk and the
    // in-core form, `DETIMES`, the fat cache's words, the published denode in its vnode's slot,
    // and the file handle.

    use super::*;
    use crate::fs::msdosfs::direntry::ATTR_READONLY;
    use crate::fs::msdosfs::fat::{FAT16_MASK, FAT32_MASK};
    use crate::fs::msdosfs::msdosfsmount::{M_MSDOSFSMNT, test_pmp};
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs;

    /// A denode of a mount of the given FAT width, on no vnode.
    fn denode(fatmask: u32) -> Denode<TestGlue> {
        let pmp = test_pmp(fatmask, 100, |_| {});
        let dev = pmp.pm_dev;
        Denode::new(None, dev, pmp, 7, 64)
    }

    /// A directory entry for "HELLO.TXT", 1234 bytes from cluster 0x12345.
    fn entry() -> [u8; 32] {
        let mut e = [0u8; 32];
        e[..11].copy_from_slice(b"HELLO   TXT");
        e[11] = ATTR_READONLY;
        e[13] = 57;
        e[14..16].copy_from_slice(&0x6000u16.to_le_bytes()); // CTime
        e[16..18].copy_from_slice(&0x5021u16.to_le_bytes()); // CDate
        e[18..20].copy_from_slice(&0x5022u16.to_le_bytes()); // ADate
        e[20..22].copy_from_slice(&0x0001u16.to_le_bytes()); // HighClust
        e[22..24].copy_from_slice(&0x6001u16.to_le_bytes()); // MTime
        e[24..26].copy_from_slice(&0x5023u16.to_le_bytes()); // MDate
        e[26..28].copy_from_slice(&0x2345u16.to_le_bytes()); // StartCluster
        e[28..32].copy_from_slice(&1234u32.to_le_bytes());
        e
    }

    #[test]
    fn internalize_fat16_ignores_the_high_cluster() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let dep = denode(FAT16_MASK);
        let e = entry();
        de_internalize(&dep, &Direntry::at(&e, 0).unwrap());
        assert_eq!(&dep.name(), b"HELLO   TXT");
        assert_eq!(dep.de_Attributes.get(), ATTR_READONLY);
        assert_eq!(dep.de_CTimeHundredth.get(), 57);
        assert_eq!(dep.de_CTime.get(), 0x6000);
        assert_eq!(dep.de_CDate.get(), 0x5021);
        assert_eq!(dep.de_ADate.get(), 0x5022);
        assert_eq!(dep.de_MTime.get(), 0x6001);
        assert_eq!(dep.de_MDate.get(), 0x5023);
        assert_eq!(dep.de_StartCluster.get(), 0x2345);
        assert_eq!(dep.de_FileSize.get(), 1234);

        let mut out = [0xaau8; 32];
        let mut dp = Direntry::at(&out, 0).unwrap();
        de_externalize(&mut dp, &dep);
        assert!(Direntry::put(&mut out, 0, &dp));
        let mut want = entry();
        want[12] = CASE_LOWER_BASE | CASE_LOWER_EXT;
        want[20..22].copy_from_slice(&[0, 0]);
        assert_eq!(out, want);
    }

    #[test]
    fn internalize_fat32_round_trip() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let dep = denode(FAT32_MASK);
        let e = entry();
        de_internalize(&dep, &Direntry::at(&e, 0).unwrap());
        assert_eq!(dep.de_StartCluster.get(), 0x12345);
        let mut dp = Direntry::default();
        de_externalize(&mut dp, &dep);
        let mut out = [0u8; 32];
        assert!(Direntry::put(&mut out, 0, &dp));
        let mut want = entry();
        want[12] = CASE_LOWER_BASE | CASE_LOWER_EXT;
        assert_eq!(out, want);

        // A directory's entry has size 0 on disk.
        dep.de_Attributes.set(ATTR_DIRECTORY);
        de_externalize(&mut dp, &dep);
        assert_eq!(dp.deFileSize, [0, 0, 0, 0]);
    }

    #[test]
    fn detimes_turns_requests_into_dos_times() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let dep = denode(FAT16_MASK);
        // 2026-10-04 12:34:56, and 1980-01-01 00:00:00.
        let now = Timespec {
            tv_sec: 1_791_117_296,
            tv_nsec: 0,
        };
        let epoch = Timespec {
            tv_sec: 315_532_800,
            tv_nsec: 0,
        };

        // No request: nothing changes.
        detimes(&dep, &now, &now, &now);
        assert_eq!(dep.de_flag.get(), 0);

        dep.set_flag(DE_UPDATE | DE_ACCESS | DE_CREATE);
        detimes(&dep, &epoch, &now, &epoch);
        assert_eq!(dep.de_flag.get(), DE_MODIFIED);
        assert_eq!(dep.de_MDate.get(), 4 | 10 << 5 | 46 << 9);
        assert_eq!(dep.de_MTime.get(), 28 | 34 << 5 | 12 << 11);
        assert_eq!(dep.de_Attributes.get(), ATTR_ARCHIVE);
        assert_eq!(dep.de_ADate.get(), 0x21);
        assert_eq!(dep.de_CDate.get(), 0x21);
        assert_eq!((dep.de_CTime.get(), dep.de_CTimeHundredth.get()), (0, 0));

        // Without Win95 there are no access and creation times.
        dep.pmp().pm_flags.set(MSDOSFSMNT_NOWIN95 as u32);
        dep.set_flag(DE_ACCESS | DE_CREATE);
        detimes(&dep, &now, &now, &now);
        assert_eq!(dep.de_ADate.get(), 0x21);
        assert_eq!(dep.de_CDate.get(), 0x21);
        assert_eq!(dep.de_flag.get(), DE_MODIFIED);
    }

    #[test]
    fn fat_cache_slots_are_words_of_both_numbers() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let dep = denode(FAT16_MASK);
        assert_eq!(dep.fc(FC_LASTFC), Fatcache::default());
        fc_setcache(&dep, FC_LASTFC, FCE_EMPTY, 0x1234_5678);
        assert_eq!(
            dep.fc(FC_LASTFC),
            Fatcache {
                fc_frcn: FCE_EMPTY,
                fc_fsrcn: 0x1234_5678
            }
        );
        assert_eq!(dep.fc(FC_LASTMAP), Fatcache::default(), "the other slots");
        assert_eq!(dep.fc(FC_OLASTFC), Fatcache::default());
        fc_setcache(&dep, FC_OLASTFC, 3, u32::MAX);
        assert_eq!(
            (dep.fc(FC_OLASTFC).fc_frcn, dep.fc(FC_OLASTFC).fc_fsrcn),
            (3, u32::MAX)
        );
        // The identity given, the rest zero.
        assert_eq!((dep.de_dirclust.get(), dep.de_diroffset.get()), (7, 64));
        assert!(dep.de_vnode.is_none() && dep.de_devvp.get().is_none());
        assert!(!dep.de_hash.is_linked() && dep.de_lockf.is_empty());
        dep.set_name(b"TEMPLATEDOS");
        assert_eq!(&dep.name(), b"TEMPLATEDOS");
    }

    #[test]
    fn a_published_denode_is_its_vnodes_node() {
        let (_g, _p) = testffs::attach(std::vec![0u8; 4096]);
        let pmp = test_pmp(FAT16_MASK, 100, |_| {});
        assert_eq!(MArc::ty(&pmp), M_MSDOSFSMNT);
        let vp = crate::fs::subr::getnewvnode::<TestGlue>(
            VT_MSDOSFS,
            Some(pmp.pm_mountp),
            &crate::fs::testfs::TEST_VOPS,
        )
        .unwrap();
        let dep = Denode::new(Some(vp), pmp.pm_dev, pmp.clone(), 0, MSDOSFSROOT_OFS).alloc();
        assert_eq!(MArc::ty(&dep), M_MSDOSFSNODE);
        vp.set_node(MArc::erase(dep.clone()));
        let again = vtode(vp);
        assert!(MArc::ptr_eq(&again, &dep));
        assert!(core::ptr::eq(again.detov(), vp));
        assert!(MArc::ptr_eq(&again.de_pmp, &pmp));
        drop(again);
        assert!(vp.take_node().is_some());
    }

    #[test]
    fn defid_overlays_fid() {
        let defid = Defid {
            defid_len: size_of::<Defid>() as u16,
            defid_pad: 0,
            defid_dirclust: 7,
            defid_dirofs: 96,
        };
        let mut fid = Fid::default();
        defid.to_fid(&mut fid);
        assert_eq!(fid.fid_len, 12);
        assert_eq!(Defid::from_fid(&fid), defid);
    }
}
/* </TESTS> */
