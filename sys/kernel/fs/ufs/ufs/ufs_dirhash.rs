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
 * Copyright (c) 2001, 2002 Ian Dowse.  All rights reserved.
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
 */
/* </LICENSES> */

/* <CODE> */
//! A hash-based lookup scheme for UFS directories (`option UFS_DIRHASH`, cargo feature
//! `ufs_dirhash`): `ufsdirhash_build` hashes a directory of at least `ufs_mindirhashsize`
//! bytes the first time `ufs_lookup` searches it, mapping each name to the offset of its
//! entry and keeping the free space of every `DIRBLKSIZ` block, so that lookups and
//! creations in large directories do not scan the whole directory. `ufs_direnter`,
//! `ufs_dirremove` and the truncation keep the hash in step (`ufsdirhash_add`, `_remove`,
//! `_move`, `_newblk`, `_dirtrunc`). The memory of all hashes is bounded by
//! `ufs_dirhashmaxmem`: a hash that does not fit recycles the least used ones
//! (`ufsdirhash_recycle`), whose owners rebuild them on their next lookup.
//!
//! Upstream: sys/ufs/ufs/ufs_dirhash.c @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufs_dirhash.rs@3c62ede68802
//!
//! Locking order: `ufsdirhash_mtx`, then `dh_mtx`. The `dh_mtx` lock should be acquired
//! either via the inode lock, or via `ufsdirhash_mtx`. Only the owner of the inode may free
//! the associated dirhash, but anything can steal its memory and set `dh_hash` to NULL.
//!
//! ## Deviations
//! - The `ufs_mindirhashsize`, `ufs_dirhashmaxmem`, `ufs_dirhashmem` and `ufs_dirhashcheck`
//!   globals are the atomics [`UFS_MINDIRHASHSIZE`], [`UFS_DIRHASHMAXMEM`],
//!   [`UFS_DIRHASHMEM`] and [`UFS_DIRHASHCHECK`] (`ffs_vars[]` hands the first three to
//!   `sysctl_bounded_arr`); `ufs_dirhashmem` is still changed only under `ufsdirhash_mtx`.
//!   `ufsdirhash_key` is two atomic words, written by `ufsdirhash_init`.
//! - The memory accounted per hash uses `size_of::<Dirhash>()`, the Rust structure's size,
//!   for the C's `sizeof(struct dirhash)` (LZ's deviation, kept).
//! - A `struct direct *` argument (`ufsdirhash_add`, `_remove`, `_move`) is the entry's
//!   name: all the C reads from it is `d_name`, `d_namlen` and `DIRSIZ(dp)`, which follows
//!   from `d_namlen`. A directory block (`ufsdirhash_checkblock`'s `buf`) is a byte slice,
//!   and `ufsdirhash_getprev` takes the block and the entry's place in it.
//! - Return values: `ufsdirhash_build` is `bool` (`true` for the C's 0); `ufsdirhash_findfree`
//!   and `ufsdirhash_enduseful` return `Option` (`None` for the C's -1), `ufsdirhash_findfree`
//!   with the slot size the C stores in `*slotsize`; `ufsdirhash_lookup` returns the offset
//!   and the buffer's token (the C's `*offp`, `*bpp`), and `ENOENT` or `EJUSTRETURN` as
//!   errors; `ufsdirhash_recycle` returns the list's witness (the C's 0 with the list
//!   locked).
//! - `ufsdirhash_build` allocates `struct dirhash` after the arrays are filled from the
//!   directory (the C allocates it first): nothing else can see the tables meanwhile, so they
//!   are filled without `dh_mtx`; both answer -1 to an `M_NOWAIT` failure, the C before
//!   reading the directory.
//! - `ufsdirhash_lookup`'s unlocked `TAILQ_NEXT(dh, dh_list) != NULL` is read as "on the list
//!   and not its last element", the last one's address kept in a word written under
//!   `ufsdirhash_mtx` (a hint, as the C's read is).
//! - `ufsdirhash_checkblock` runs, as in C, only when `ufs_dirhashcheck` is set (0 by
//!   default; no sysctl sets it in OpenBSD); the host tests set it.
//! - `ufsdirhash_uninit` is left out: `dirhash.h` declares it and nothing calls it, in the C
//!   or in LZ, and `ufsdirhash_pool` lives as long as the kernel (a `hal` pool is never
//!   destroyed).
//!
//! ## Redesign
//! - LZ's `i_dirhash` pointer word and `unsafe` deref of the `malloc`ed `Dirhash` (the
//!   `i_dirhash` helper and its 13 callers' `unsafe` blocks, sys/ufs/ufs/ufs_dirhash.rs at
//!   the pin) -> the inode's `i_dirhash` cell, a `ReadMostly<Option<MBox<Dirhash>>>`: the
//!   dirhash is an `MBox` of `M_DIRHASH` the inode owns, its tables the data of `dh_mtx`
//!   (`dirhash.rs`). Each function reads the cell for its whole run, across the directory
//!   block reads that sleep, where `hal::slot` asks a reader not to sleep: its reason is a
//!   writer left spinning on a sleeping reader, and this cell's writers (`ufsdirhash_build`
//!   and `ufsdirhash_free`) run only under the inode's lock, which every reader's caller
//!   holds, so no writer can wait on one; a function drops its read before it calls
//!   `ufsdirhash_free` itself.
//! - LZ's `ufsdirhash_list` (a `sys/queue.rs` tail queue in a static, `unsafe` links) and
//!   `ufsdirhash_mtx` -> a `hal::pinlist::PinList` bound to the lock (`SharedRwlock` over a
//!   static `RawRwlock`): the list's witness is the write lock's; `TAILQ_REMOVE` +
//!   `TAILQ_INSERT_AFTER(dh_next)` is `PinList::step_over`.
//! - LZ's `pool_get` blocks behind raw pointers -> `ufsdirhash_pool` a `hal::pool::BoxPool`
//!   of `[doff_t; DH_NBLKOFF]`, its items `PoolBox`es in the `MArray`; a recycle takes the
//!   arrays out of the tables under both locks and drops them after, as the C frees them.
//! - The directory blocks come from `UFS_BUFATOFF` as the busy buffer's token (`IoBuf`'s,
//!   the file system's `InodeOps::bufatoff`), read through its bytes and released by
//!   `brelse`; `ufsdirhash_lookup` hands it to its caller.

use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicU64, AtomicUsize, Ordering};

use hal::intr::IPL_NONE;
use hal::malloc::{M_NOWAIT, MArray, MBox};
use hal::pinlist::PinList;
use hal::pool::{BoxPool, PR_WAITOK};
use hal::random::arc4random;
use hal::rwlock::{RawRwlock, SharedRwlock, WriteWitness};
use hal::sys::errno::Errno;
use hal::sys::types::Off;
use libkern::siphash::{SipHash24, SiphashKey};

use super::dir::{DIRBLKSIZ, Doff, d_ino, d_name, d_namlen, d_reclen, directsiz, dirsiz};
use super::dirhash::{
    DH_NBLKOFF, DH_NFSTATS, DH_SCOREMAX, DIRALIGN, DIRHASH_DEL, DIRHASH_EMPTY, DhArrays, DhBlock,
    DhTables, Dirhash, DirhashList,
};
use super::inode::Inode;
use crate::assert::{kassert, kpanic};
use crate::fs::bio::brelse;
use crate::fs::buf::BufRef;
use crate::proc::ProcGlue;

/// `M_DIRHASH`: the malloc type of the directory hashes (`<sys/malloc.h>`; `vmstat -m`'s
/// "dirhash").
pub const M_DIRHASH: i32 = 32;

/// `DIRBLKSIZ` as an `int`.
const DIRBLK: i32 = DIRBLKSIZ as i32;

/// `WRAPINCR(val, limit)`.
const fn wrapincr(val: i32, limit: i32) -> i32 {
    if val + 1 == limit { 0 } else { val + 1 }
}

/// `WRAPDECR(val, limit)`.
const fn wrapdecr(val: i32, limit: i32) -> i32 {
    if val == 0 { limit - 1 } else { val - 1 }
}

/// `BLKFREE2IDX(n)`.
const fn blkfree2idx(n: i32) -> usize {
    if n > DH_NFSTATS as i32 {
        DH_NFSTATS
    } else {
        n as usize
    }
}

/// `ufs_mindirhashsize`: the smallest directory, in bytes, that is hashed
/// (`vfs.ffs.dirhash_dirsize`).
pub static UFS_MINDIRHASHSIZE: AtomicI32 = AtomicI32::new(0);
/// `ufs_dirhashmaxmem`: the memory all hashes may use, in bytes (`vfs.ffs.dirhash_maxmem`).
pub static UFS_DIRHASHMAXMEM: AtomicI32 = AtomicI32::new(0);
/// `ufs_dirhashmem`: the memory the hashes use, in bytes (`vfs.ffs.dirhash_mem`). Changed
/// under `ufsdirhash_mtx`.
pub static UFS_DIRHASHMEM: AtomicI32 = AtomicI32::new(0);
/// `ufs_dirhashcheck`: run `ufsdirhash_checkblock`'s consistency checks.
pub static UFS_DIRHASHCHECK: AtomicI32 = AtomicI32::new(0);

/// `ufsdirhash_key`: the SipHash key of every hash (`k0`, `k1`), set by `ufsdirhash_init`.
static UFSDIRHASH_KEY: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];

/// `ufsdirhash_pool`: the blocks of `DH_NBLKOFF` offsets of the hash arrays.
static UFSDIRHASH_POOL: BoxPool<DhBlock> = BoxPool::new();

/// `ufsdirhash_mtx`'s lock: protects `ufsdirhash_list`, the `dh_list` field, `ufs_dirhashmem`.
static UFSDIRHASH_RW: RawRwlock = RawRwlock::new("dirhash_list");
/// `ufsdirhash_mtx`.
static UFSDIRHASH_MTX: SharedRwlock = SharedRwlock::from_static(&UFSDIRHASH_RW);
/// `ufsdirhash_list`: dirhash list; recently-used entries are near the tail.
static UFSDIRHASH_LIST: PinList<DirhashList> = PinList::new();
/// The address of `ufsdirhash_list`'s last element (0: empty), written under
/// `ufsdirhash_mtx` after every change, read without it by `ufsdirhash_lookup`.
static UFSDIRHASH_LAST: AtomicUsize = AtomicUsize::new(0);

/// `DIRHASHLIST_LOCK()`: the list's witness.
fn dirhashlist_lock() -> WriteWitness<'static> {
    UFSDIRHASH_MTX.write()
}

/// Records the list's last element after a change (`w` is the list's witness).
fn note_last(w: &WriteWitness<'_>) {
    let last = UFSDIRHASH_LIST
        .last(w)
        .map_or(0, |d| ptr::from_ref(d).addr());
    UFSDIRHASH_LAST.store(last, Ordering::Relaxed);
}

/// `TAILQ_NEXT(dh, dh_list) != NULL`, read without the list's lock (a hint).
fn has_next(dh: &Dirhash) -> bool {
    dh.onlist() && UFSDIRHASH_LAST.load(Ordering::Relaxed) != ptr::from_ref(dh).addr()
}

/// `UFS_BUFATOFF(ip, offset, NULL, &bp)`: the directory block holding `offset`, as its
/// token, and the offset's place in it.
fn bufatoff<G: ProcGlue>(ip: &Inode<G>, offset: Doff) -> Result<(BufRef, usize), Errno> {
    let (b, off) = ip.vtbl().bufatoff(ip, Off::from(offset))?;
    match b.into_token() {
        Some(t) => Ok((t, off)),
        None => kpanic(format_args!("UFS_BUFATOFF: a buffer without its token")),
    }
}

/// The bytes of the arrays of a hash of `narrays` blocks and `nblk` block counters, without
/// the structure: what `ufsdirhash_free` and `ufsdirhash_recycle` give back.
const fn arrays_mem(narrays: i32, nblk: i32) -> i32 {
    narrays * size_of::<usize>() as i32
        + narrays * DH_NBLKOFF * size_of::<Doff>() as i32
        + nblk * size_of::<u8>() as i32
}

/// `ufsdirhash_mtx`'s account: `ufs_dirhashmem -= mem`.
fn unaccount(mem: i32) {
    let _w = dirhashlist_lock();
    UFS_DIRHASHMEM.fetch_sub(mem, Ordering::Relaxed);
}

/// `ufsdirhash_build`: attempt to build up a hash table for the directory contents in
/// inode `ip`. Returns `true` on success (the C's 0), or `false` (-1) if the operation
/// failed or the directory should not be hashed.
pub fn ufsdirhash_build<G: ProcGlue>(ip: &Inode<G>) -> bool {
    let size = ip.dip_size() as i64;

    // Check if we can/should use dirhash.
    let intact = ip.i_dirhash.read().as_deref().map(Dirhash::hashed);
    match intact {
        None => {
            if size < i64::from(UFS_MINDIRHASHSIZE.load(Ordering::Relaxed)) {
                return false;
            }
        }
        Some(intact) => {
            // Hash exists, but sysctls could have changed.
            if size < i64::from(UFS_MINDIRHASHSIZE.load(Ordering::Relaxed))
                || UFS_DIRHASHMEM.load(Ordering::Relaxed)
                    > UFS_DIRHASHMAXMEM.load(Ordering::Relaxed)
            {
                ufsdirhash_free(ip);
                return false;
            }
            // Check if hash exists and is intact (note: unlocked read).
            if intact {
                return true;
            }
            // Free the old, recycled hash and build a new one.
            ufsdirhash_free(ip);
        }
    }

    // Don't hash removed directories.
    if ip.i_effnlink.get() == 0 {
        return false;
    }

    // Allocate 50% more entries than this dir size could ever need.
    kassert!(size >= DIRBLKSIZ as i64);
    let mut nslots = (size / directsiz(1) as i64) as i32;
    nslots = (nslots * 3 + 1) / 2;
    let narrays = (nslots + DH_NBLKOFF - 1) / DH_NBLKOFF;
    nslots = narrays * DH_NBLKOFF;
    let dirblocks = ((size + DIRBLKSIZ as i64 - 1) / DIRBLKSIZ as i64) as i32;
    let nblocks = (dirblocks * 3 + 1) / 2;

    let memreqd = size_of::<Dirhash>() as i32 + arrays_mem(narrays, nblocks);
    let w = dirhashlist_lock();
    let w = if memreqd + UFS_DIRHASHMEM.load(Ordering::Relaxed)
        > UFS_DIRHASHMAXMEM.load(Ordering::Relaxed)
    {
        drop(w);
        if memreqd > UFS_DIRHASHMAXMEM.load(Ordering::Relaxed) / 2 {
            return false;
        }

        // Try to free some space.
        match ufsdirhash_recycle(memreqd) {
            Some(w) => w,
            None => return false,
        }
        // Enough was freed, and list has been locked.
    } else {
        w
    };
    UFS_DIRHASHMEM.fetch_add(memreqd, Ordering::Relaxed);
    drop(w);

    // Use non-blocking mallocs so that we will revert to a linear lookup on failure rather
    // than potentially blocking forever.
    if let Some(t) = build_tables(ip, narrays, nslots, dirblocks, nblocks)
        && let Some(dh) = MBox::new(M_DIRHASH, M_NOWAIT, Dirhash::new(t))
    {
        let old = ip.i_dirhash.replace(Some(dh));
        kassert!(old.is_none());
        drop(old);
        let r = ip.i_dirhash.read();
        if let Some(dh) = r.as_ref() {
            let mut w = dirhashlist_lock();
            UFSDIRHASH_LIST.insert_tail(&mut w, dh.as_pin());
            note_last(&w);
            dh.set_hashed(true);
        }
        return true;
    }
    // fail: the arrays went with the tables.
    unaccount(memreqd);
    false
}

/// The tables of a new hash of `ip`, filled from the directory's blocks; `None` when an
/// array cannot be allocated, a block cannot be read, or the directory is corrupt.
fn build_tables<G: ProcGlue>(
    ip: &Inode<G>,
    narrays: i32,
    nslots: i32,
    dirblocks: i32,
    nblocks: i32,
) -> Option<DhTables> {
    let hash = MArray::new(M_DIRHASH, M_NOWAIT, narrays as usize, |_| None);
    let blkfree = MArray::new(M_DIRHASH, M_NOWAIT, nblocks as usize, |_| 0u8);
    let (Some(mut hash), Some(blkfree)) = (hash, blkfree) else {
        return None;
    };
    for b in hash.iter_mut() {
        *b = Some(UFSDIRHASH_POOL.get(PR_WAITOK, || [DIRHASH_EMPTY; DH_NBLKOFF as usize])?);
    }

    // Initialise the hash table and block statistics.
    let mut firstfree = [-1; DH_NFSTATS + 1];
    firstfree[DH_NFSTATS] = 0;
    let mut t = DhTables {
        arrays: Some(DhArrays { hash, blkfree }),
        narrays,
        hlen: nslots,
        hused: 0,
        nblk: nblocks,
        dirblks: dirblocks,
        firstfree,
    };
    for i in 0..dirblocks {
        t.set_blkfree(i, (DIRBLK / DIRALIGN) as u8);
    }

    let bmask = ip.mount().iosize() as i32 - 1;
    let mut bp: Option<BufRef> = None;
    let mut pos: Doff = 0;
    while u64::from(pos as u32) < ip.dip_size() {
        // If necessary, get the next directory block.
        if pos & bmask == 0 {
            if let Some(b) = bp.take() {
                brelse(b);
            }
            bp = Some(bufatoff(ip, pos).ok()?.0);
        }
        let Some(b) = bp.as_ref() else {
            kpanic(format_args!("ufsdirhash_build: no directory block"));
        };
        let data = b.bytes();
        // Add this entry to the hash.
        let ep = (pos & bmask) as usize;
        let reclen = i32::from(d_reclen(data, ep));
        if reclen == 0 || reclen > DIRBLK - (pos & (DIRBLK - 1)) {
            // Corrupted directory.
            if let Some(b) = bp.take() {
                brelse(b);
            }
            return None;
        }
        if d_ino(data, ep) != 0 {
            // Add the entry (simplified ufsdirhash_add).
            let namlen = d_namlen(data, ep);
            let mut slot = ufsdirhash_hash(t.hlen, d_name(data, ep, usize::from(namlen)));
            while t.arrays().entry(slot) != DIRHASH_EMPTY {
                slot = wrapincr(slot, t.hlen);
            }
            t.hused += 1;
            t.arrays_mut().set_entry(slot, pos);
            ufsdirhash_adjfree(&mut t, pos, -(dirsiz(namlen) as i32));
        }
        pos += reclen;
    }

    if let Some(b) = bp {
        brelse(b);
    }
    Some(t)
}

/// `ufsdirhash_free`: free any hash table associated with inode `ip`.
pub fn ufsdirhash_free<G: ProcGlue>(ip: &Inode<G>) {
    let Some(dh) = ip.i_dirhash.replace(None) else {
        return;
    };
    let mut mem = size_of::<Dirhash>() as i32;
    {
        let mut w = dirhashlist_lock();
        let t = dh.tables.write();
        if dh.onlist() {
            UFSDIRHASH_LIST.remove(&mut w, &dh);
            note_last(&w);
        }
        if t.arrays.is_some() {
            mem += arrays_mem(t.narrays, t.nblk);
        }
    }

    // The dirhash pointed to by 'dh' is exclusively ours now: its arrays, then the
    // structure, freed with it.
    drop(dh);

    unaccount(mem);
}

/// `ufsdirhash_lookup`: find the offset of the specified name within the given inode.
/// Returns the offset and the buffer holding the entry on success, `ENOENT` if the entry
/// does not exist, or `EJUSTRETURN` if the caller should revert to a linear search.
///
/// If `prevoffp` is given, the offset of the previous entry within the `DIRBLKSIZ`-sized
/// block is stored in it (if the entry is the first in a block, the start of the block is
/// used).
pub fn ufsdirhash_lookup<G: ProcGlue>(
    ip: &Inode<G>,
    name: &[u8],
    mut prevoffp: Option<&mut Doff>,
) -> Result<(Doff, BufRef), Errno> {
    let r = ip.i_dirhash.read();
    let Some(dh) = r.as_deref() else {
        return Err(Errno::EJUSTRETURN);
    };
    // Move this dirhash towards the end of the list if it has a score higher than the next
    // entry, and acquire the dh_mtx. Optimise the case where it's already the last by
    // performing an unlocked read of the TAILQ_NEXT pointer.
    //
    // In both cases, end up holding just dh_mtx.
    let mut t = if has_next(dh) {
        let mut w = dirhashlist_lock();
        let t = dh.tables.write();
        // If the new score will be greater than that of the next entry, then move this
        // entry past it. With both mutexes held, dh_next won't go away, but its dh_score
        // could change; that's not important since it is just a hint.
        if t.arrays.is_some()
            && dh.onlist()
            && UFSDIRHASH_LIST
                .next(&w, dh)
                .is_some_and(|n| dh.score() >= n.score())
        {
            UFSDIRHASH_LIST.step_over(&mut w, dh);
            note_last(&w);
        }
        drop(w);
        t
    } else {
        // Already the last, though that could change as we wait.
        dh.tables.write()
    };
    if t.arrays.is_none() {
        drop(t);
        drop(r);
        ufsdirhash_free(ip);
        return Err(Errno::EJUSTRETURN);
    }

    // Update the score.
    if dh.score() < DH_SCOREMAX {
        dh.dh_score.fetch_add(1, Ordering::Relaxed);
    }

    let bmask = ip.mount().iosize() as i32 - 1;
    let mut blkoff: Doff = -1;
    let mut bp: Option<BufRef> = None;
    'restart: loop {
        let mut slot = ufsdirhash_hash(t.hlen, name);

        if dh.dh_seqopt.load(Ordering::Relaxed) != 0 {
            // Sequential access optimisation. dh_seqoff contains the offset of the directory
            // entry immediately following the last entry that was looked up. Check if this
            // offset appears in the hash chain for the name we are looking for.
            let seqoff = dh.dh_seqoff.load(Ordering::Relaxed);
            let mut i = slot;
            let mut offset;
            loop {
                offset = t.arrays().entry(i);
                if offset == DIRHASH_EMPTY || offset == seqoff {
                    break;
                }
                i = wrapincr(i, t.hlen);
            }
            if offset == seqoff {
                // We found an entry with the expected offset. This is probably the entry we
                // want, but if not, the code below will turn off seqopt and retry.
                slot = i;
            } else {
                dh.dh_seqopt.store(0, Ordering::Relaxed);
            }
        }

        loop {
            let offset = t.arrays().entry(slot);
            if offset == DIRHASH_EMPTY {
                break 'restart;
            }
            if offset == DIRHASH_DEL {
                slot = wrapincr(slot, t.hlen);
                continue;
            }
            drop(t);

            if offset < 0 || u64::from(offset as u32) >= ip.dip_size() {
                kpanic(format_args!("ufsdirhash_lookup: bad offset in hash array"));
            }
            if offset & !bmask != blkoff {
                if let Some(b) = bp.take() {
                    brelse(b);
                }
                blkoff = offset & !bmask;
                match bufatoff(ip, blkoff) {
                    Ok((b, _)) => bp = Some(b),
                    Err(_) => return Err(Errno::EJUSTRETURN),
                }
            }
            let Some(b) = bp.as_ref() else {
                kpanic(format_args!("ufsdirhash_lookup: no directory block"));
            };
            let data = b.bytes();
            let dp = (offset & bmask) as usize;
            let reclen = i32::from(d_reclen(data, dp));
            if reclen == 0 || reclen > DIRBLK - (offset & (DIRBLK - 1)) {
                // Corrupted directory.
                if let Some(b) = bp.take() {
                    brelse(b);
                }
                return Err(Errno::EJUSTRETURN);
            }
            let namlen = d_namlen(data, dp);
            if usize::from(namlen) == name.len() && d_name(data, dp, name.len()) == name {
                // Found. Get the prev offset if needed.
                if let Some(prevoffp) = prevoffp.take() {
                    let prevoff = if offset & (DIRBLK - 1) != 0 {
                        let Some(prevoff) = ufsdirhash_getprev(data, dp, offset) else {
                            if let Some(b) = bp.take() {
                                brelse(b);
                            }
                            return Err(Errno::EJUSTRETURN);
                        };
                        prevoff
                    } else {
                        offset
                    };
                    *prevoffp = prevoff;
                }

                // Check for sequential access, and update offset.
                if dh.dh_seqopt.load(Ordering::Relaxed) == 0
                    && dh.dh_seqoff.load(Ordering::Relaxed) == offset
                {
                    dh.dh_seqopt.store(1, Ordering::Relaxed);
                }
                dh.dh_seqoff
                    .store(offset + dirsiz(namlen) as Doff, Ordering::Relaxed);

                match bp {
                    Some(b) => return Ok((offset, b)),
                    None => kpanic(format_args!("ufsdirhash_lookup: no directory block")),
                }
            }

            t = dh.tables.write();
            if t.arrays.is_none() {
                drop(t);
                if let Some(b) = bp.take() {
                    brelse(b);
                }
                drop(r);
                ufsdirhash_free(ip);
                return Err(Errno::EJUSTRETURN);
            }
            // When the name doesn't match in the seqopt case, go back and search normally.
            if dh.dh_seqopt.load(Ordering::Relaxed) != 0 {
                dh.dh_seqopt.store(0, Ordering::Relaxed);
                continue 'restart;
            }
            slot = wrapincr(slot, t.hlen);
        }
    }
    drop(t);
    if let Some(b) = bp {
        brelse(b);
    }
    Err(Errno::ENOENT)
}

/// Runs `f` over `ip`'s dirhash and its tables locked (`DIRHASH_LOCK`); a recycled hash is
/// freed (`ufsdirhash_free`) and `f` not run. `None` when there is no hash, when it was
/// recycled, or when `f` answers `None`, which frees the hash too (the callers' "let it be
/// rebuilt later").
fn locked<G: ProcGlue, R>(
    ip: &Inode<G>,
    f: impl FnOnce(&Dirhash, &mut DhTables) -> Option<R>,
) -> Option<R> {
    let r = ip.i_dirhash.read();
    let dh = r.as_deref()?;
    let out = {
        let mut t = dh.tables.write();
        if t.arrays.is_none() {
            None
        } else {
            f(dh, &mut t)
        }
    };
    if out.is_none() {
        drop(r);
        ufsdirhash_free(ip);
    }
    out
}

/// `ufsdirhash_findfree`: find a directory block with room for `slotneeded` bytes. Returns
/// the offset of the directory entry that begins the free space. This will either be the
/// offset of an existing entry that has free space at the end, or the offset of an entry
/// with `d_ino == 0` at the start of a `DIRBLKSIZ` block.
///
/// To use the space, the caller may need to compact existing entries in the directory. The
/// total number of bytes in all of the entries involved in the compaction is returned with
/// the offset (the C's `*slotsize`). In other words, all of the entries that must be
/// compacted are exactly contained in the region beginning at the returned offset and
/// spanning that many bytes.
///
/// Returns `None` (-1) if no space was found, indicating that the directory must be
/// extended.
pub fn ufsdirhash_findfree<G: ProcGlue>(ip: &Inode<G>, slotneeded: i32) -> Option<(Doff, i32)> {
    // Find a directory block with the desired free space.
    let need = (slotneeded as usize).div_ceil(DIRALIGN as usize);
    let dirblock = locked(ip, |_, t| {
        let b = (need..=DH_NFSTATS)
            .map(|i| t.firstfree[i])
            .find(|&b| b != -1);
        if let Some(b) = b {
            kassert!(b < t.nblk && usize::from(t.blkfree(b)) >= need);
        }
        Some(b)
    })??;

    let pos = dirblock * DIRBLK;
    let (bp, dpoff) = bufatoff(ip, pos).ok()?;
    let data = bp.bytes();

    // Find the first entry with free space.
    let mut i: i32 = 0;
    let mut bad = false;
    while i < DIRBLK {
        let dp = dpoff + i as usize;
        let reclen = i32::from(d_reclen(data, dp));
        if reclen == 0 {
            bad = true;
            break;
        }
        if d_ino(data, dp) == 0 || reclen > dirsiz(d_namlen(data, dp)) as i32 {
            break;
        }
        i += reclen;
    }
    if bad || i > DIRBLK {
        brelse(bp);
        return None;
    }
    let slotstart = pos + i;

    // Find the range of entries needed to get enough space
    let mut freebytes = 0;
    while i < DIRBLK && freebytes < slotneeded {
        let dp = dpoff + i as usize;
        let reclen = i32::from(d_reclen(data, dp));
        freebytes += reclen;
        if d_ino(data, dp) != 0 {
            freebytes -= dirsiz(d_namlen(data, dp)) as i32;
        }
        if reclen == 0 {
            bad = true;
            break;
        }
        i += reclen;
    }
    if bad || i > DIRBLK {
        brelse(bp);
        return None;
    }
    if freebytes < slotneeded {
        kpanic(format_args!("ufsdirhash_findfree: free mismatch"));
    }
    brelse(bp);
    Some((slotstart, pos + i - slotstart))
}

/// `ufsdirhash_enduseful`: return the start of the unused space at the end of a directory,
/// or `None` (-1) if there are no trailing unused blocks.
pub fn ufsdirhash_enduseful<G: ProcGlue>(ip: &Inode<G>) -> Option<Doff> {
    locked(ip, |_, t| {
        let empty = (DIRBLK / DIRALIGN) as u8;
        if t.blkfree(t.dirblks - 1) != empty {
            return Some(None);
        }
        let mut i = t.dirblks - 1;
        while i >= 0 && t.blkfree(i) == empty {
            i -= 1;
        }
        Some(Some((i + 1) * DIRBLK))
    })?
}

/// `ufsdirhash_add`: insert information into the hash about a new directory entry, the
/// entry named `name` at `offset`.
pub fn ufsdirhash_add<G: ProcGlue>(ip: &Inode<G>, name: &[u8], offset: Doff) {
    let _ = locked(ip, |_, t| {
        kassert!(offset < t.dirblks * DIRBLK);
        // Normal hash usage is < 66%. If the usage gets too high then remove the hash
        // entirely and let it be rebuilt later.
        if t.hused >= (t.hlen * 3) / 4 {
            return None;
        }

        // Find a free hash slot (empty or deleted), and add the entry.
        let mut slot = ufsdirhash_hash(t.hlen, name);
        while t.arrays().entry(slot) >= 0 {
            slot = wrapincr(slot, t.hlen);
        }
        if t.arrays().entry(slot) == DIRHASH_EMPTY {
            t.hused += 1;
        }
        t.arrays_mut().set_entry(slot, offset);

        // Update the per-block summary info.
        ufsdirhash_adjfree(t, offset, -(namesiz(name) as i32));
        Some(())
    });
}

/// `DIRSIZ(dirp)` of the entry named `name`.
fn namesiz(name: &[u8]) -> usize {
    dirsiz(name.len() as u8)
}

/// `ufsdirhash_remove`: remove the specified directory entry from the hash. The entry to
/// remove is defined by the name `name`, which must exist at the specified `offset` within
/// the directory.
pub fn ufsdirhash_remove<G: ProcGlue>(ip: &Inode<G>, name: &[u8], offset: Doff) {
    let _ = locked(ip, |_, t| {
        kassert!(offset < t.dirblks * DIRBLK);
        // Find the entry
        let slot = ufsdirhash_findslot(t, name, offset);

        // Remove the hash entry.
        ufsdirhash_delslot(t, slot);

        // Update the per-block summary info.
        ufsdirhash_adjfree(t, offset, namesiz(name) as i32);
        Some(())
    });
}

/// `ufsdirhash_move`: change the offset associated with a directory entry in the hash. Used
/// when compacting directory blocks.
pub fn ufsdirhash_move<G: ProcGlue>(ip: &Inode<G>, name: &[u8], oldoff: Doff, newoff: Doff) {
    let _ = locked(ip, |_, t| {
        kassert!(oldoff < t.dirblks * DIRBLK && newoff < t.dirblks * DIRBLK);
        // Find the entry, and update the offset.
        let slot = ufsdirhash_findslot(t, name, oldoff);
        t.arrays_mut().set_entry(slot, newoff);
        Some(())
    });
}

/// `ufsdirhash_newblk`: inform dirhash that the directory has grown by one block that
/// begins at `offset` (i.e. the new length is `offset + DIRBLKSIZ`).
pub fn ufsdirhash_newblk<G: ProcGlue>(ip: &Inode<G>, offset: Doff) {
    let _ = locked(ip, |_, t| {
        kassert!(offset == t.dirblks * DIRBLK);
        let block = offset / DIRBLK;
        if block >= t.nblk {
            // Out of space; must rebuild.
            return None;
        }
        t.dirblks = block + 1;

        // Account for the new free block.
        t.set_blkfree(block, (DIRBLK / DIRALIGN) as u8);
        if t.firstfree[DH_NFSTATS] == -1 {
            t.firstfree[DH_NFSTATS] = block;
        }
        Some(())
    });
}

/// `ufsdirhash_dirtrunc`: inform dirhash that the directory is being truncated.
pub fn ufsdirhash_dirtrunc<G: ProcGlue>(ip: &Inode<G>, offset: Doff) {
    let _ = locked(ip, |_, t| {
        kassert!(offset <= t.dirblks * DIRBLK);
        let block = (offset as usize).div_ceil(DIRBLKSIZ) as i32;
        // If the directory shrinks to less than 1/8 of dh_nblk blocks (about 20% of its
        // original size due to the 50% extra added in ufsdirhash_build) then free it, and
        // let the caller rebuild if necessary.
        if block < t.nblk / 8 && t.narrays > 1 {
            return None;
        }

        // Remove any `first free' information pertaining to the truncated blocks. All
        // blocks we're removing should be completely unused.
        if t.firstfree[DH_NFSTATS] >= block {
            t.firstfree[DH_NFSTATS] = -1;
        }
        for i in block..t.dirblks {
            if t.blkfree(i) != (DIRBLK / DIRALIGN) as u8 {
                kpanic(format_args!("ufsdirhash_dirtrunc: blocks in use"));
            }
        }
        for i in 0..DH_NFSTATS {
            if t.firstfree[i] >= block {
                kpanic(format_args!("ufsdirhash_dirtrunc: first free corrupt"));
            }
        }
        t.dirblks = block;
        Some(())
    });
}

/// `ufsdirhash_checkblock`: debugging function to check that the dirhash information about
/// a directory block matches its actual contents. Panics if a mismatch is detected.
///
/// `buf` is the in-core `DIRBLKSIZ`-sized directory block, and `offset` the offset from the
/// start of the directory of that block.
pub fn ufsdirhash_checkblock<G: ProcGlue>(ip: &Inode<G>, buf: &[u8], offset: Doff) {
    if UFS_DIRHASHCHECK.load(Ordering::Relaxed) == 0 {
        return;
    }
    let _ = locked(ip, |_, t| {
        let block = offset / DIRBLK;
        if offset & (DIRBLK - 1) != 0 || block >= t.dirblks {
            kpanic(format_args!("ufsdirhash_checkblock: bad offset"));
        }

        let mut nfree = 0;
        let mut i: i32 = 0;
        while i < DIRBLK {
            let dp = i as usize;
            let reclen = i32::from(d_reclen(buf, dp));
            if reclen == 0 || i + reclen > DIRBLK {
                kpanic(format_args!("ufsdirhash_checkblock: bad dir"));
            }

            if d_ino(buf, dp) == 0 {
                // XXX entries with d_ino == 0 should only occur at the start of a
                // DIRBLKSIZ block. However the ufs code is tolerant of such entries at other
                // offsets, and fsck does not fix them (the C's check is under #if 0).
                nfree += reclen;
                i += reclen;
                continue;
            }

            // Check that the entry exists (will panic if it doesn't).
            let namlen = d_namlen(buf, dp);
            ufsdirhash_findslot(t, d_name(buf, dp, usize::from(namlen)), offset + i);

            nfree += reclen - dirsiz(namlen) as i32;
            i += reclen;
        }
        if i != DIRBLK {
            kpanic(format_args!("ufsdirhash_checkblock: bad dir end"));
        }

        if i32::from(t.blkfree(block)) * DIRALIGN != nfree {
            kpanic(format_args!("ufsdirhash_checkblock: bad free count"));
        }

        let ffslot = blkfree2idx(nfree / DIRALIGN);
        for i in 0..=DH_NFSTATS {
            if t.firstfree[i] == block && i != ffslot {
                kpanic(format_args!("ufsdirhash_checkblock: bad first-free"));
            }
        }
        if t.firstfree[ffslot] == -1 {
            kpanic(format_args!(
                "ufsdirhash_checkblock: missing first-free entry"
            ));
        }
        Some(())
    });
}

/// `ufsdirhash_hash`: hash the specified filename into a slot of a hash of `hlen` slots.
fn ufsdirhash_hash(hlen: i32, name: &[u8]) -> i32 {
    let key = SiphashKey {
        k0: UFSDIRHASH_KEY[0].load(Ordering::Relaxed),
        k1: UFSDIRHASH_KEY[1].load(Ordering::Relaxed),
    };
    (SipHash24(&key, name) % hlen as u64) as i32
}

/// `ufsdirhash_adjfree`: adjust the number of free bytes in the block containing `offset`
/// by the value specified by `diff`.
///
/// The caller must ensure we have exclusive access to the tables: `dh_mtx` held, or a hash
/// `ufsdirhash_build` is filling.
fn ufsdirhash_adjfree(t: &mut DhTables, offset: Doff, diff: i32) {
    // Update the per-block summary info.
    let block = offset / DIRBLK;
    kassert!(block < t.nblk && block < t.dirblks);
    let ofidx = blkfree2idx(i32::from(t.blkfree(block)));
    t.set_blkfree(block, (i32::from(t.blkfree(block)) + diff / DIRALIGN) as u8);
    let nfidx = blkfree2idx(i32::from(t.blkfree(block)));

    // Update the `first free' list if necessary.
    if ofidx != nfidx {
        // If removing, scan forward for the next block.
        if t.firstfree[ofidx] == block {
            let next =
                (block + 1..t.dirblks).find(|&i| blkfree2idx(i32::from(t.blkfree(i))) == ofidx);
            t.firstfree[ofidx] = next.unwrap_or(-1);
        }

        // Make this the new `first free' if necessary
        if t.firstfree[nfidx] > block || t.firstfree[nfidx] == -1 {
            t.firstfree[nfidx] = block;
        }
    }
}

/// `ufsdirhash_findslot`: find the specified name which should have the specified offset.
/// Returns a slot number, and panics on failure. The tables are locked.
fn ufsdirhash_findslot(t: &DhTables, name: &[u8], offset: Doff) -> i32 {
    // Find the entry.
    kassert!(t.hused < t.hlen);
    let a = t.arrays();
    let mut slot = ufsdirhash_hash(t.hlen, name);
    while a.entry(slot) != offset && a.entry(slot) != DIRHASH_EMPTY {
        slot = wrapincr(slot, t.hlen);
    }
    if a.entry(slot) != offset {
        kpanic(format_args!(
            "ufsdirhash_findslot: '{}' not found",
            core::str::from_utf8(name).unwrap_or("?")
        ));
    }

    slot
}

/// `ufsdirhash_delslot`: remove the entry corresponding to the specified slot from the hash
/// array. The tables are locked.
fn ufsdirhash_delslot(t: &mut DhTables, slot: i32) {
    let hlen = t.hlen;
    // Mark the entry as deleted.
    t.arrays_mut().set_entry(slot, DIRHASH_DEL);

    // If this is the end of a chain of DIRHASH_DEL slots, remove them.
    let mut i = slot;
    while t.arrays().entry(i) == DIRHASH_DEL {
        i = wrapincr(i, hlen);
    }
    if t.arrays().entry(i) == DIRHASH_EMPTY {
        i = wrapdecr(i, hlen);
        while t.arrays().entry(i) == DIRHASH_DEL {
            t.arrays_mut().set_entry(i, DIRHASH_EMPTY);
            t.hused -= 1;
            i = wrapdecr(i, hlen);
        }
        kassert!(t.hused >= 0);
    }
}

/// `ufsdirhash_getprev`: given a directory entry at `dpos` in `data` and its directory
/// offset, find the offset of the previous entry in the same `DIRBLKSIZ`-sized block.
/// Returns an offset, or `None` (-1) if there is no previous entry in the block or some
/// other problem occurred.
fn ufsdirhash_getprev(data: &[u8], dpos: usize, offset: Doff) -> Option<Doff> {
    let blkoff = offset & !(DIRBLK - 1); // offset of start of block
    let entrypos = offset & (DIRBLK - 1); // entry relative to block
    let blkbuf = dpos.checked_sub(entrypos as usize)?;
    let mut prevoff = blkoff;

    // If `offset' is the start of a block, there is no previous entry.
    if entrypos == 0 {
        return None;
    }

    // Scan from the start of the block until we get to the entry.
    let mut i = 0;
    while i < entrypos {
        let reclen = i32::from(d_reclen(data, blkbuf + i as usize));
        if reclen == 0 || i + reclen > entrypos {
            return None; // Corrupted directory.
        }
        prevoff = blkoff + i;
        i += reclen;
    }
    Some(prevoff)
}

/// `ufsdirhash_recycle`: try to free up `wanted` bytes by stealing memory from existing
/// dirhashes. Returns the list's witness (the C's 0 with the list locked) if successful.
fn ufsdirhash_recycle(wanted: i32) -> Option<WriteWitness<'static>> {
    let mut w = dirhashlist_lock();
    while wanted + UFS_DIRHASHMEM.load(Ordering::Relaxed)
        > UFS_DIRHASHMAXMEM.load(Ordering::Relaxed)
    {
        // Find a dirhash, and lock it.
        let (arrays, mem) = {
            let dh = UFSDIRHASH_LIST.first(&w)?;
            let mut t = dh.tables.write();
            kassert!(t.arrays.is_some());

            // Decrement the score; only recycle if it becomes zero.
            if dh.dh_score.fetch_sub(1, Ordering::Relaxed) - 1 > 0 {
                return None;
            }

            // Detach its memory (remove it from the list below, both locks still held).
            dh.set_hashed(false);
            (t.arrays.take(), arrays_mem(t.narrays, t.nblk))
        };
        UFSDIRHASH_LIST.remove_first(&mut w);
        note_last(&w);

        // Unlock everything, free the detached memory.
        drop(w);
        drop(arrays);

        // Account for the returned memory, and repeat if necessary.
        w = dirhashlist_lock();
        UFS_DIRHASHMEM.fetch_sub(mem, Ordering::Relaxed);
    }
    // Success; return with list locked.
    Some(w)
}

/// `ufsdirhash_init`: the pool, the hash key and the default limits (the list and its lock
/// are statics, empty from the start).
pub fn ufsdirhash_init() {
    // Made once; a host test's second `ufs_init` finds it made (EBUSY) and keeps it.
    let _ = UFSDIRHASH_POOL.init(IPL_NONE, PR_WAITOK, "dirhash");
    let word = || u64::from(arc4random()) << 32 | u64::from(arc4random());
    for k in &UFSDIRHASH_KEY {
        k.store(word(), Ordering::Relaxed);
    }
    UFS_DIRHASHMAXMEM.store(5 * 1024 * 1024, Ordering::Relaxed);
    UFS_MINDIRHASHSIZE.store(5 * DIRBLK, Ordering::Relaxed);
}

/// The dirhash state forgotten (a host test's file systems start afresh, testing.md): the
/// list emptied (its dirhashes are the old test's inodes', never freed), the memory count
/// cleared.
#[cfg(any(test, feature = "doubles"))]
pub fn dirhash_test_reset() {
    let mut w = dirhashlist_lock();
    while UFSDIRHASH_LIST.remove_first(&mut w) {}
    note_last(&w);
    UFS_DIRHASHMEM.store(0, Ordering::Relaxed);
    UFS_DIRHASHCHECK.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::format;
    use std::vec;
    use std::vec::Vec;

    use hal::malloc::M_WAITOK;

    use super::*;
    use crate::fs::testfs::setup;

    /// Tables of `hlen` slots and `dirblks` directory blocks, every block empty, as
    /// `ufsdirhash_build` initialises them.
    fn tables(hlen: i32, dirblks: i32) -> DhTables {
        let _ = UFSDIRHASH_POOL.init(IPL_NONE, PR_WAITOK, "dirhash");
        let narrays = (hlen + DH_NBLKOFF - 1) / DH_NBLKOFF;
        let nblk = (dirblks * 3 + 1) / 2;
        let hash = MArray::new(M_DIRHASH, M_WAITOK, narrays as usize, |_| {
            UFSDIRHASH_POOL.get(PR_WAITOK, || [DIRHASH_EMPTY; DH_NBLKOFF as usize])
        })
        .unwrap();
        let blkfree = MArray::new(M_DIRHASH, M_WAITOK, nblk as usize, |_| {
            (DIRBLK / DIRALIGN) as u8
        })
        .unwrap();
        let mut firstfree = [-1; DH_NFSTATS + 1];
        firstfree[DH_NFSTATS] = 0;
        DhTables {
            arrays: Some(DhArrays { hash, blkfree }),
            narrays,
            hlen,
            hused: 0,
            nblk,
            dirblks,
            firstfree,
        }
    }

    #[test]
    fn hash_stays_in_range_and_depends_on_the_name() {
        let _g = setup();
        let hlen = 3 * DH_NBLKOFF;
        let mut seen = BTreeSet::new();
        for i in 0..200 {
            let name = format!("name{i}");
            let slot = ufsdirhash_hash(hlen, name.as_bytes());
            assert!((0..hlen).contains(&slot));
            assert_eq!(slot, ufsdirhash_hash(hlen, name.as_bytes()));
            seen.insert(slot);
        }
        // 200 names over 768 slots: a hash worth the name spreads them.
        assert!(seen.len() > 120, "only {} distinct slots", seen.len());
    }

    #[test]
    fn slots_chain_on_collisions_and_deleted_chains_collapse() {
        let _g = setup();
        // Four slots, so that names collide; the chain wraps around the end.
        let mut t = tables(4, 4);
        let add = |t: &mut DhTables, name: &[u8], off: Doff| {
            let mut slot = ufsdirhash_hash(t.hlen, name);
            while t.arrays().entry(slot) >= 0 {
                slot = wrapincr(slot, t.hlen);
            }
            if t.arrays().entry(slot) == DIRHASH_EMPTY {
                t.hused += 1;
            }
            t.arrays_mut().set_entry(slot, off);
            slot
        };
        let a = add(&mut t, b"a", 0);
        let b = add(&mut t, b"b", 12);
        let c = add(&mut t, b"c", 24);
        assert_eq!(t.hused, 3);
        for (n, off, slot) in [(&b"a"[..], 0, a), (b"b", 12, b), (b"c", 24, c)] {
            assert_eq!(ufsdirhash_findslot(&t, n, off), slot);
        }

        // Deleting an entry inside a chain leaves a DIRHASH_DEL marker that keeps the chain
        // walkable; deleting the chain's last live entries empties the markers too.
        let empty = (0..4)
            .find(|&s| t.arrays().entry(s) == DIRHASH_EMPTY)
            .unwrap();
        let chain: Vec<i32> = (1..4).map(|k| (empty + k) % 4).collect();
        ufsdirhash_delslot(&mut t, chain[0]);
        assert_eq!(t.arrays().entry(chain[0]), DIRHASH_DEL);
        assert_eq!(t.hused, 3);
        for &s in &chain[1..] {
            let off = t.arrays().entry(s);
            let name: &[u8] = match off {
                0 => b"a",
                12 => b"b",
                _ => b"c",
            };
            assert_eq!(ufsdirhash_findslot(&t, name, off), s);
        }
        ufsdirhash_delslot(&mut t, chain[2]);
        assert_eq!(t.arrays().entry(chain[2]), DIRHASH_EMPTY);
        assert_eq!(t.hused, 2);
        ufsdirhash_delslot(&mut t, chain[1]);
        // The whole chain collapses: chain[1], then the marker left at chain[0].
        assert!((0..4).all(|s| t.arrays().entry(s) == DIRHASH_EMPTY));
        assert_eq!(t.hused, 0);
    }

    #[test]
    fn adjfree_keeps_the_first_free_lists() {
        let _g = setup();
        let mut t = tables(DH_NBLKOFF, 4);
        let full = DH_NFSTATS;
        assert_eq!(t.firstfree[full], 0);

        // Block 0 loses 400 bytes: 28 words free, so it is the first block with 28 free, and
        // block 1 becomes the first entirely free one.
        ufsdirhash_adjfree(&mut t, 0, -400);
        assert_eq!(t.blkfree(0), 28);
        assert_eq!(t.firstfree[28], 0);
        assert_eq!(t.firstfree[full], 1);
        // Block 2 likewise; block 0 stays the first with 28 free.
        ufsdirhash_adjfree(&mut t, 2 * DIRBLK + 100, -400);
        assert_eq!(t.firstfree[28], 0);
        // Block 0 empties again: block 2 is now the first with 28 free, block 0 the first free.
        ufsdirhash_adjfree(&mut t, 12, 400);
        assert_eq!(t.blkfree(0), (DIRBLK / DIRALIGN) as u8);
        assert_eq!(t.firstfree[28], 2);
        assert_eq!(t.firstfree[full], 0);
        // Small changes within the top bucket (more than DH_NFSTATS words free) move nothing.
        ufsdirhash_adjfree(&mut t, 3 * DIRBLK, -16);
        assert_eq!(t.blkfree(3), 124);
        assert_eq!(t.firstfree[full], 0);
        assert_eq!(blkfree2idx(124), full);
    }

    #[test]
    fn getprev_finds_the_previous_entry_in_the_block() {
        // A block at directory offset 512 with entries at 0 (12 bytes), 12 (16) and 28 (rest).
        let mut blk = vec![0u8; DIRBLKSIZ];
        let mut put = |off: usize, reclen: u16| {
            blk[off..off + 4].copy_from_slice(&5u32.to_ne_bytes());
            blk[off + 4..off + 6].copy_from_slice(&reclen.to_ne_bytes());
            blk[off + 7] = 1;
        };
        put(0, 12);
        put(12, 16);
        put(28, (DIRBLKSIZ - 28) as u16);
        assert_eq!(ufsdirhash_getprev(&blk, 28, DIRBLK + 28), Some(DIRBLK + 12));
        assert_eq!(ufsdirhash_getprev(&blk, 12, DIRBLK + 12), Some(DIRBLK));
        assert_eq!(ufsdirhash_getprev(&blk, 0, DIRBLK), None);
        // An entry offset that does not fall on an entry boundary is corruption.
        assert_eq!(ufsdirhash_getprev(&blk, 20, DIRBLK + 20), None);
    }

    #[test]
    fn a_recycled_hash_has_no_arrays_and_its_memory_is_the_c_formula() {
        let _g = setup();
        let mut t = tables(DH_NBLKOFF * 2, 8);
        assert_eq!(arrays_mem(t.narrays, t.nblk), 2 * 8 + 2 * 256 * 4 + 12);
        let taken = t.arrays.take();
        assert!(taken.is_some() && t.arrays.is_none());
        // A dirhash on no list has no next; a new one has the initial score, no arrays seen.
        let dh = Dirhash::new(t);
        assert!(!dh.onlist() && !has_next(&dh));
        assert_eq!(dh.score(), super::super::dirhash::DH_SCOREINIT);
        assert!(!dh.hashed());
    }
}
/* </TESTS> */
