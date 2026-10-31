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
 * Copyright (c) 2001 Ian Dowse.  All rights reserved.
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
 * $FreeBSD: src/sys/ufs/ufs/dirhash.h,v 1.4 2003/01/01 18:48:59 schweikh Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ufs/dirhash.h>`: the hash of a large directory. For fast operations on large
//! directories, we maintain a hash that maps the file name to the offset of the directory
//! entry within the directory file.
//!
//! Upstream: sys/ufs/ufs/dirhash.h @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/dirhash.rs@3c62ede68802
//!
//! The hashing uses a dumb spillover to the next free slot on collisions, so we must keep
//! the utilisation low to avoid long linear searches. Deleted entries that are not the last
//! in a chain must be marked [`DIRHASH_DEL`].
//!
//! We also maintain information about free space in each block to speed up creations.
//!
//! Dirhash uses a score mechanism to achieve a hybrid between a least-recently-used and a
//! least-often-used algorithm for entry recycling. The score is incremented when a directory
//! is used, and decremented when the directory is a candidate for recycling. When the score
//! reaches zero, the hash is recycled. Hashes are linked together on a TAILQ list, and hashes
//! with higher scores filter towards the tail (most recently used) end of the list.
//!
//! New hash entries are given an initial score of [`DH_SCOREINIT`] and are placed at the
//! most-recently-used end of the list. This helps a lot in the worst-case case scenario where
//! every directory access is to a directory that is not hashed (i.e. the working set of hash
//! candidates is much larger than the configured memory limit). In this case it limits the
//! number of hash builds to `1/DH_SCOREINIT` of the number of accesses.
//!
//! The main hash table has 2 levels. It is an array of pointers to blocks of [`DH_NBLKOFF`]
//! offsets.
//!
//! ## Deviations
//! - The prototypes and the `ufs_mindirhashsize`/`ufs_dirhashmaxmem`/`ufs_dirhashmem`
//!   `extern`s are the definitions in `ufs_dirhash.rs` (feature `ufs_dirhash`, the C's
//!   `option UFS_DIRHASH`); this header is compiled always, as `inode.h` names `struct dirhash
//!   *i_dirhash` without the option.
//! - `dh_score`, `dh_seqopt` and `dh_seqoff` are relaxed words beside the lock-owned tables:
//!   the C reads the next hash's `dh_score` without its lock (a hint for the list's order)
//!   and writes `dh_seqopt`/`dh_seqoff` after `DIRHASH_UNLOCK`, under the inode's lock alone.
//!   `dh_onlist` is the list entry's own `is_linked`; `ufsdirhash_build`'s unlocked read of
//!   `dh_hash` is a relaxed flag beside the tables, written under `dh_mtx`.
//! - The second level of `dh_hash` is an array of 16-byte pool handles where the C's holds
//!   8-byte pointers, so a hash's `M_DIRHASH` block for it is twice the C's (`vmstat -m`'s
//!   "dirhash"); the memory a hash is accounted for (`ufs_dirhashmem`, `vfs.ffs.dirhash_mem`)
//!   keeps the C's formula.
//!
//! ## Redesign
//! - LZ's `Dirhash` of `Cell`s under one `unsafe impl Sync`, whose `dh_hash` and
//!   `dh_blkfree` were raw `malloc`/`pool_get` pointers read through `unsafe` derefs behind
//!   index checks (sys/ufs/ufs/dirhash.rs:150-210 at the pin) -> [`Dirhash`]: the members
//!   `dh_mtx` protects are its data ([`DhTables`] in a `hal::rwlock::Rwlocked`, `dh_mtx`
//!   itself), the two levels an `MArray` of `M_DIRHASH` holding the `ufsdirhash_pool` blocks
//!   (`PoolBox`es) and the free counts an `MArray` of bytes, both in [`DhArrays`], `None` once
//!   recycled (the C's `dh_hash == NULL`); `DH_ENTRY` and `dh_blkfree[]` are slice indexing
//!   (a bad slot a bounds panic, where the C reads out of bounds).
//! - `dh_list` is a `hal::pinlist::PinEntry` of `ufsdirhash_list` (the dirhash is pinned in
//!   its `MBox`, and the entry's drop panics while linked: the C's "remove before free").

use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use hal::malloc::MArray;
use hal::pinlist::PinEntry;
use hal::pool::PoolBox;
use hal::rwlock::Rwlocked;

use super::dir::{Doff, MAXNAMLEN, directsiz};
use crate::assert::kpanic;

/// `DIRHASH_EMPTY`: entry unused.
pub const DIRHASH_EMPTY: Doff = -1;
/// `DIRHASH_DEL`: deleted entry; may be part of chain.
pub const DIRHASH_DEL: Doff = -2;

/// `DIRALIGN`.
pub const DIRALIGN: i32 = 4;
/// `DH_NFSTATS`: max `DIRALIGN` words in a directory entry.
pub const DH_NFSTATS: usize = directsiz(MAXNAMLEN + 1) / DIRALIGN as usize;

/// `DH_SCOREINIT`: initial `dh_score` when dirhash built.
pub const DH_SCOREINIT: i32 = 8;
/// `DH_SCOREMAX`: max `dh_score` value.
pub const DH_SCOREMAX: i32 = 64;

/// `DH_BLKOFFSHIFT`.
pub const DH_BLKOFFSHIFT: i32 = 8;
/// `DH_NBLKOFF`: offsets in one block of the hash array.
pub const DH_NBLKOFF: i32 = 1 << DH_BLKOFFSHIFT;
/// `DH_BLKOFFMASK`.
pub const DH_BLKOFFMASK: i32 = DH_NBLKOFF - 1;

/// A block of the hash array: `DH_NBLKOFF` offsets, an item of `ufsdirhash_pool`.
pub type DhBlock = [Doff; DH_NBLKOFF as usize];

/// `dh_hash` and `dh_blkfree`: the two arrays a recycle takes away.
pub struct DhArrays {
    /// `dh_hash`: the hash array (2-level), `dh_narrays` blocks (`None` until allocated).
    pub hash: MArray<Option<PoolBox<DhBlock>>>,
    /// `dh_blkfree`: free `DIRALIGN` words in each dir block (`dh_nblk` bytes). XXX assumes
    /// `DIRBLKSIZ` is 512.
    pub blkfree: MArray<u8>,
}

impl DhArrays {
    /// The block of `slot`; `panic(9)` for one never allocated (the C's NULL).
    fn block(&self, slot: i32) -> &DhBlock {
        match self.hash.get((slot >> DH_BLKOFFSHIFT) as usize) {
            Some(Some(b)) => b,
            _ => kpanic(format_args!("dirhash: slot {slot} has no block")),
        }
    }

    /// `DH_ENTRY(dh, slot)`.
    pub fn entry(&self, slot: i32) -> Doff {
        self.block(slot)[(slot & DH_BLKOFFMASK) as usize]
    }

    /// `DH_ENTRY(dh, slot) = v`.
    pub fn set_entry(&mut self, slot: i32, v: Doff) {
        match self.hash.get_mut((slot >> DH_BLKOFFSHIFT) as usize) {
            Some(Some(b)) => b[(slot & DH_BLKOFFMASK) as usize] = v,
            _ => kpanic(format_args!("dirhash: slot {slot} has no block")),
        }
    }
}

/// The members of `struct dirhash` that `dh_mtx` protects.
pub struct DhTables {
    /// `dh_hash` and `dh_blkfree`; `None` once recycled.
    pub arrays: Option<DhArrays>,
    /// `dh_narrays`: number of entries in `dh_hash`.
    pub narrays: i32,
    /// `dh_hlen`: total slots in the 2-level hash array.
    pub hlen: i32,
    /// `dh_hused`: entries in use.
    pub hused: i32,
    /// `dh_nblk`: size of `dh_blkfree` array.
    pub nblk: i32,
    /// `dh_dirblks`: number of `DIRBLKSIZ` blocks in dir.
    pub dirblks: i32,
    /// `dh_firstfree`: first blk with N words free.
    pub firstfree: [i32; DH_NFSTATS + 1],
}

impl DhTables {
    /// The arrays of a hash that was not recycled; `panic(9)` otherwise (the C would follow
    /// NULL).
    pub fn arrays(&self) -> &DhArrays {
        match &self.arrays {
            Some(a) => a,
            None => kpanic(format_args!("dirhash: recycled")),
        }
    }

    /// [`DhTables::arrays`], writable.
    pub fn arrays_mut(&mut self) -> &mut DhArrays {
        match &mut self.arrays {
            Some(a) => a,
            None => kpanic(format_args!("dirhash: recycled")),
        }
    }

    /// `dh->dh_blkfree[block]`.
    pub fn blkfree(&self, block: i32) -> u8 {
        self.arrays().blkfree[block as usize]
    }

    /// `dh->dh_blkfree[block] = v`.
    pub fn set_blkfree(&mut self, block: i32, v: u8) {
        self.arrays_mut().blkfree[block as usize] = v;
    }
}

/// `struct dirhash`.
///
/// Locks: `dh_mtx` ([`Dirhash::tables`]) for every member but `dh_list`, which
/// `ufsdirhash_mtx` protects, and the three hints (the module's deviations). The `dh_mtx`
/// lock is acquired either via the inode lock, or via `ufsdirhash_mtx`. Only the owner of the
/// inode may free the associated dirhash, but anything can steal its memory and set `dh_hash`
/// to NULL.
pub struct Dirhash {
    /// `dh_mtx` and the members it protects.
    pub tables: Rwlocked<DhTables>,
    /// `dh_score`: access count for this dirhash.
    pub dh_score: AtomicI32,
    /// `dh_seqopt`: sequential access optimisation enabled.
    pub dh_seqopt: AtomicI32,
    /// `dh_seqoff`: sequential access optimisation offset.
    pub dh_seqoff: AtomicI32,
    /// `dh_hash != NULL`, read without `dh_mtx` (`ufsdirhash_build`'s unlocked read):
    /// cleared under it when a recycle takes the arrays, set by `ufsdirhash_build` under the
    /// list's lock when it publishes them (no other thread sees the dirhash before).
    dh_hashed: AtomicBool,
    /// `dh_list`: chain of all dirhashes. Protected by `ufsdirhash_mtx`.
    pub dh_list: PinEntry<Dirhash>,
}

impl Dirhash {
    /// A dirhash over `tables`, with the score of a new one, on no list.
    pub fn new(tables: DhTables) -> Self {
        Self {
            tables: Rwlocked::new("dirhash", tables),
            dh_score: AtomicI32::new(DH_SCOREINIT),
            dh_seqopt: AtomicI32::new(0),
            dh_seqoff: AtomicI32::new(0),
            dh_hashed: AtomicBool::new(false),
            dh_list: PinEntry::new(),
        }
    }

    /// `dh_score`.
    pub fn score(&self) -> i32 {
        self.dh_score.load(Ordering::Relaxed)
    }

    /// `dh_hash != NULL`, as an unlocked read sees it.
    pub fn hashed(&self) -> bool {
        self.dh_hashed.load(Ordering::Relaxed)
    }

    /// Records whether the arrays are there (under `dh_mtx`).
    pub fn set_hashed(&self, v: bool) {
        self.dh_hashed.store(v, Ordering::Relaxed);
    }

    /// `dh_onlist`: whether the dirhash is on `ufsdirhash_list`.
    pub fn onlist(&self) -> bool {
        self.dh_list.is_linked()
    }
}

hal::pin_adapter!(
    /// `ufsdirhash_list`'s link: dirhashes through their `dh_list`.
    pub DirhashList: Dirhash, dh_list
);

const _: () = {
    assert!(DH_NFSTATS == 67);
    assert!(DH_NBLKOFF == 256);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ufs/dirhash.h");
        for (name, v) in [
            ("DIRHASH_EMPTY", i64::from(DIRHASH_EMPTY)),
            ("DIRHASH_DEL", i64::from(DIRHASH_DEL)),
            ("DIRALIGN", i64::from(DIRALIGN)),
            ("DH_SCOREINIT", i64::from(DH_SCOREINIT)),
            ("DH_SCOREMAX", i64::from(DH_SCOREMAX)),
            ("DH_BLKOFFSHIFT", i64::from(DH_BLKOFFSHIFT)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(v), "{name}");
        }
    }
}
/* </TESTS> */
