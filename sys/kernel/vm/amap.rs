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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! The amap: `struct vm_amap` of `<uvm/uvm_amap.h>` and the amap operations of
//! `uvm/uvm_amap.c`.
//!
//! Upstream: sys/uvm/uvm_amap.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_amap.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_amap.rs@3c62ede68802
//!
//! An amap holds the anons (`kernel::vm::anon`) that are mapped together in virtual memory,
//! one per page: its entries are called slots, clustered into chunks of [`UVM_AMAP_CHUNK`]
//! slots each ([`Chunk`]: the anons and a bitmap of the slots in use). An amap of up to
//! `UVM_AMAP_CHUNK` slots embeds its one chunk; a larger one keeps its chunks in a hash
//! table of buckets, a chunk per run of `UVM_AMAP_CHUNK` slots, created as anons arrive
//! (`lazyalloc`) or all at once. The per-page reference counts (`ppref`) are an optional
//! array, made when only part of an amap gains or loses a reference ([`amap_pp_adjref`]).
//!
//! An amap is referenced by map entries (`am_ref`, [`Amap::refs`]): `fork` adds one, the
//! clip routines split one in two ([`amap_splitref`]), `amap_copy` gives a map entry a copy
//! of its own ([`amap_copy_from`]) and the last [`amap_unref`] wipes the amap out. Its lock is
//! shared with its anons and with every amap that shares anons with it: a copy takes the
//! source's lock and keeps it while any anon is shared.
//!
//! ## Deviations
//! - A chunk always has `UVM_AMAP_CHUNK` anon slots: the C's flexible `ac_anon[]` and the
//!   sixteen `uvm_small_amap_pool[]`s sized by slot count (`"amappl1"` .. `"amappl16"`) are
//!   one amap pool, `"amappl"`, and one chunk pool, `"amapchunkpl"` (LZ's deviation, kept).
//! - Swap off is not here (N3.5, the user's decision G7 of N3.4): there is no `amap_list`
//!   (and no `amap_list_lock` taken on every allocation and free), and [`amap_swap_off`]
//!   reports `unported: amap_swap_off (uvm_swap, N3.5)` once and answers that nothing failed,
//!   which is what LZ's walk answered with no anon on swap (`swapctl` is `sys_nosys` at the
//!   pin). `AMAP_SWAPOFF` is kept and tested where the C tests it; nothing sets it.
//! - `UVM_PAGE_OWN` is not configured.
//!
//! ## Redesign
//! - LZ's `amap_alloc() -> &'static VmAmap` (a pool item cast and written by hand, freed by
//!   `amap_free`'s `pool_put` while map entries could still name it) -> [`AmapRef`], a
//!   `hal::pool::PoolArc<Amap>` of `"amappl"`. `am_ref` stays the C's count of the map
//!   entries that reference the amap ([`Amap::refs`], read without the lock where the C reads
//!   it so, a relaxed atomic), beside the handle count that owns the memory: the map entries'
//!   handles and those of threads that keep the amap across a sleep. Each anon of a slot is
//!   held by an [`AnonRef`] in the slot, and every place the C increments `an_ref` clones the
//!   handle into the slot (the copy of [`amap_copy_from`]); [`amap_add`] takes the caller's
//!   handle into the slot, [`amap_unadd`] hands the slot's back.
//! - `am_lock: Cell<*const Rwlock>` (`rw_obj_alloc`, `rw_obj_hold` by a copy, `rw_obj_free` by
//!   each) -> the amap's `hal::rwlock::SharedRwlock`, which its anons and its copies clone;
//!   what the C keeps under it (the slot count in use, the `ppref` array, the chunk tables)
//!   is an [`AmapState`] inside a `hal::rwlock::Guarded` keyed to that lock: the functions
//!   that read it take a witness of the lock (`&impl Witness`), those that change it the write
//!   witness (`&mut WriteWitness`), and a witness of another lock is `panic(9)`, so the C's
//!   `KASSERT(anon->an_lock == amap->am_lock)` is the check that opens the state.
//!   `amap_lock(amap, rwlt)` / `amap_unlock(amap)` are the witness's creation and drop
//!   (`amap.lock().write()`, `.read()`): [`amap_wipeout`] and [`amap_adjref_anons`] consume the
//!   witness where the C unlocks. `am_nslot` and `am_hashshift` never change after
//!   [`amap_alloc1`] and live outside the lock ([`Amap::nslot`]).
//! - `am_ppref: Cell<*mut i32>` with `PPREF_NONE` (`(int *)-1`) for "the array could not be
//!   had" -> [`Ppref`]: `None` (`NULL`), `Failed` (`PPREF_NONE`), `Some` of a
//!   `hal::malloc::MArray<i32>` of `M_UVMAMAP`; `pp_getreflen`/`pp_setreflen` index a slice.
//! - The normal amap's `am_buckets` array of chunk pointers, with every chunk also on the
//!   `am_chunks` `TAILQ` (a bucket's chunks consecutive on it), and the chunks' `ac_list`
//!   links -> an `MArray` of buckets, each the head of an owned chain of the bucket's chunks
//!   (`hal::pool::PoolBox<Chunk>` items of `"amapchunkpl"`, each owning the next: the C's run
//!   of a bucket on the list is the chain). A new chunk joins the end of its bucket's chain,
//!   where the C inserts it after the bucket's last; `AMAP_CHUNK_FOREACH` walks the buckets in
//!   order, and [`amap_wiperange`] always walks the buckets of the range (the C walks the
//!   whole list instead when that is cheaper: the same chunks). A chunk is 152 bytes, as the
//!   C's item.
//! - `amap_copy(map, entry, waitf, canchunk, startva, endva)` is split: the map's half (the
//!   chunking decision, the clips, the install into the entry and `UVM_ET_NEEDSCOPY`) stays
//!   with the map (`bsd`'s `sys/uvm/uvm_amap.rs`), and [`amap_copy_from`] is the amap's: the
//!   take-over checks, the copy of the slots and the lock it shares or replaces, answered as
//!   a [`CopyOutcome`]. The C's `amap_ref`, `amap_unref` and `amap_splitref` keep their
//!   counting; the handle a new map entry holds is the caller's clone.
//! - The C's unlock, `amap_list_remove` and lock again around the last `amap_unref` go with
//!   the list (see the deviations): [`amap_unref`] wipes the amap out under the witness it
//!   took. A wiped-out amap's state is gone ([`amap_wipeout`] frees its tables); its memory
//!   goes with the last handle.
//! - `amap_cow_now(map, entry)`, `amap_lookup(aref, offset)` and the other aref forms take
//!   the amap and the reference's page offset (`aref->ar_pageoff`): the aref is the map's
//!   (`VmAref`, "locked by vm_map lock"), whose forms stay in `bsd`'s adapter.
//! - `amap_init(ipl)` takes the machine's `IPL_MPFLOOR` from its caller (the pools' level).
//! - N3.4d: [`amap_populate`]'s allocation can sleep (`PR_WAITOK`), and the fault calls it
//!   with nothing held (`uvm_fault.c`'s `amap_add` failure); the state opens only through the
//!   write witness, so the fault splits it: [`amap_chunk_alloc`] makes the chunk with nothing
//!   held (the C's sleep point), [`amap_populate_with`] installs it under a write witness
//!   taken afterwards, unless the slot's chunk exists by then (the spare goes back).

use core::mem;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray};
use hal::pool::{BoxPool, PR_NOWAIT, PR_WAITOK, PoolArc, PoolBox, TypedPool};
use hal::rwlock::{Guarded, RW_WRITE, SharedRwlock, Witness, WriteWitness};
use hal::sys::errno::Errno;
use hal::sys::limits::INT_MAX;
use hal::sys::mman::PROT_NONE;

use super::anon::{AnonRef, uvm_analloc, uvm_anfree, uvm_anon_pagein};
use super::page::{
    Owner, PG_BUSY, PG_FAKE, VmPage, uvm_pageactivate, uvm_pagealloc, uvm_pagecopy, uvm_pagewait,
};
use super::pmemrange::uvm_wait;
use crate::assert::{kassert, kpanic};
use crate::prf::unported;

/// `AMAP_SHARED`: amap is shared.
pub const AMAP_SHARED: i32 = 0x1;
/// `AMAP_REFALL`: amap_ref: reference entire amap.
pub const AMAP_REFALL: i32 = 0x2;
/// `AMAP_SWAPOFF`: amap_swap_off() is in progress.
pub const AMAP_SWAPOFF: i32 = 0x4;

/// `UVM_AMAP_LARGE`: # of slots in "large" amap.
pub const UVM_AMAP_LARGE: usize = 256;
/// `UVM_AMAP_CHUNK`: # of slots to chunk large amaps in.
pub const UVM_AMAP_CHUNK: usize = 16;

/// `M_UVMAMAP` (`<sys/malloc.h>`): the malloc type of the bucket and `ppref` arrays.
pub const M_UVMAMAP: i32 = 98;

/// `struct vm_amap_chunk`: `UVM_AMAP_CHUNK` slots of an amap.
pub struct Chunk {
    /// The next chunk of the bucket (the C's `ac_list` within the bucket's run).
    next: Option<PoolBox<Chunk>>,
    /// `ac_baseslot`: the amap slot of `anons[0]`.
    baseslot: i32,
    /// `ac_usedmap`: a bit per slot in use.
    usedmap: u16,
    /// `ac_nslot`: the slots of the amap this chunk covers.
    nslot: u16,
    /// `ac_anon[]`.
    anons: [Option<AnonRef>; UVM_AMAP_CHUNK],
}

impl Chunk {
    /// An empty chunk for `nslot` slots from `baseslot`.
    const fn new(baseslot: i32, nslot: u16) -> Self {
        Self {
            next: None,
            baseslot,
            usedmap: 0,
            nslot,
            anons: [const { None }; UVM_AMAP_CHUNK],
        }
    }

    /// `ac_baseslot`.
    pub fn baseslot(&self) -> i32 {
        self.baseslot
    }

    /// `ac_nslot`.
    pub fn nslot(&self) -> u16 {
        self.nslot
    }

    /// `ac_usedmap`.
    pub fn usedmap(&self) -> u16 {
        self.usedmap
    }

    /// `ac_anon[i]`.
    pub fn anon(&self, i: usize) -> Option<&AnonRef> {
        self.anons[i].as_ref()
    }
}

/// The chunk tables of an amap (the `am_impl` union).
enum Tables {
    /// `ami_small`: the one chunk of an amap of at most `UVM_AMAP_CHUNK` slots.
    Small(Chunk),
    /// `ami_normal`: the buckets, each the chain of its chunks, and `am_ncused`.
    Normal {
        /// `am_buckets` (its length is `am_nbuckets`).
        buckets: MArray<Option<PoolBox<Chunk>>>,
        /// `am_ncused`: # of chunks currently in use.
        ncused: i32,
    },
    /// The tables of an amap [`amap_wipeout`] freed.
    Gone,
}

/// `am_ppref`: the optional per-page reference counts.
enum Ppref {
    /// `NULL`: not made yet.
    None,
    /// `PPREF_NONE`: the array could not be had; not used.
    Failed,
    /// The array, of `am_nslot` ints.
    Some(MArray<i32>),
}

/// What an amap keeps under its lock.
pub struct AmapState {
    /// `am_nused`: # of slots currently in use.
    nused: i32,
    /// `am_ppref`.
    ppref: Ppref,
    /// The chunk tables.
    tables: Tables,
}

impl AmapState {
    /// `am_nused`.
    pub fn nused(&self) -> i32 {
        self.nused
    }

    /// `AMAP_CHUNK_FOREACH`: `f` on each chunk until it answers `Some`.
    fn try_each_chunk<R>(&mut self, mut f: impl FnMut(&mut Chunk) -> Option<R>) -> Option<R> {
        match &mut self.tables {
            Tables::Small(c) => f(c),
            Tables::Normal { buckets, .. } => buckets.iter_mut().find_map(|head| {
                let mut cur = head.as_mut();
                while let Some(c) = cur {
                    if let Some(r) = f(c) {
                        return Some(r);
                    }
                    cur = c.next.as_mut();
                }
                None
            }),
            Tables::Gone => gone(),
        }
    }
}

/// The geometry of an amap, fixed by [`amap_alloc1`].
#[derive(Clone, Copy)]
struct Geom {
    /// `am_nslot`: # of slots currently in map.
    nslot: i32,
    /// `am_hashshift`: shift count to hash slot to bucket.
    hashshift: i32,
}

impl Geom {
    /// `UVM_AMAP_SMALL(amap)`.
    fn is_small(self) -> bool {
        self.nslot as usize <= UVM_AMAP_CHUNK
    }

    /// `UVM_AMAP_BUCKET(amap, slot)`.
    fn bucket(self, slot: usize) -> usize {
        (slot / UVM_AMAP_CHUNK) >> self.hashshift
    }
}

/// `struct vm_amap`.
///
/// Locks: `lock` (`am_lock`) guards `state` and the writes of `refs` and `flags`; `geom` is
/// immutable after [`amap_alloc1`].
pub struct Amap {
    /// `am_lock`: lock for all vm_amap flags, shared with the anons and the copies.
    lock: SharedRwlock,
    /// `am_ref`: the map entries that reference the amap.
    refs: AtomicI32,
    /// `am_flags`.
    flags: AtomicI32,
    /// `am_nslot`, `am_hashshift`.
    geom: Geom,
    /// The slots and their tables.
    state: Guarded<AmapState>,
}

impl Amap {
    /// `am_lock`.
    pub fn lock(&self) -> &SharedRwlock {
        &self.lock
    }

    /// `amap_refs(amap)`: `am_ref`, which the C also reads without the lock where it can only
    /// be one if the reader holds the only reference.
    pub fn refs(&self) -> i32 {
        self.refs.load(Ordering::Relaxed)
    }

    /// `amap_flags(amap)`: `am_flags`.
    pub fn flags(&self) -> i32 {
        self.flags.load(Ordering::Relaxed)
    }

    /// `am_nslot`.
    pub fn nslot(&self) -> i32 {
        self.geom.nslot
    }

    /// `UVM_AMAP_SMALL(amap)`.
    pub fn is_small(&self) -> bool {
        self.geom.is_small()
    }

    /// What the amap keeps under its lock, for the holder of `w` (a witness of its lock).
    pub fn state<'a, W: Witness>(&'a self, w: &'a W) -> &'a AmapState {
        self.state.get(w)
    }

    /// `am_ref = n`, under the lock held for writing.
    fn set_refs(&self, n: i32) {
        kassert!(self.lock.status() == RW_WRITE);
        self.refs.store(n, Ordering::Relaxed);
    }

    /// `am_flags = f`, under the lock held for writing.
    fn set_flags(&self, f: i32) {
        kassert!(self.lock.status() == RW_WRITE);
        self.flags.store(f, Ordering::Relaxed);
    }
}

/// A counted handle to an amap (`struct vm_amap *`): a map entry's, or a thread's that keeps
/// the amap across a sleep.
pub type AmapRef = PoolArc<Amap>;

/// What [`amap_copy_from`] did.
pub enum CopyOutcome {
    /// The source has one reference, the caller's: it takes the source over (no copy).
    TakeOver,
    /// A copy with one reference, for the caller's map entry.
    New(AmapRef),
    /// No memory for the copy: nothing changed (the C leaves `UVM_ET_NEEDSCOPY` set).
    NoMemory,
}

/// Pools for allocation of vm_amap structures. Note that in order to avoid an endless loop,
/// the amap pool's allocator cannot allocate memory from an amap (it currently goes through
/// the kernel uobj, so we are ok).
static AMAP_POOL: TypedPool<Amap> = TypedPool::new();
/// `uvm_amap_chunk_pool`.
static CHUNK_POOL: BoxPool<Chunk> = BoxPool::new();

/// The use of an amap whose tables [`amap_wipeout`] freed: a C use after free, `panic(9)`.
#[cold]
fn gone() -> ! {
    kpanic(format_args!("amap: used after amap_wipeout"))
}

/// `UVM_AMAP_SLOTIDX(slot)`.
pub const fn uvm_amap_slotidx(slot: usize) -> usize {
    slot % UVM_AMAP_CHUNK
}

/// `AMAP_B2SLOT(S, B)`: convert byte offset to slot.
fn amap_b2slot(b: usize) -> usize {
    kassert!(b & (PAGE_SIZE - 1) == 0);
    b >> PAGE_SHIFT
}

/// `AMAP_BASE_SLOT(slot)`.
pub const fn amap_base_slot(slot: usize) -> usize {
    (slot / UVM_AMAP_CHUNK) * UVM_AMAP_CHUNK
}

/// The pool wait flag for a malloc wait flag.
fn pool_waitf(waitf: i32) -> i32 {
    if waitf & M_WAITOK != 0 {
        PR_WAITOK
    } else {
        PR_NOWAIT
    }
}

/// `amap_chunk_get`: lookup a chunk for slot. If `create` is set, the chunk is created if it
/// does not yet exist (from the chunk pool with `waitf`). `None` when it does not exist and
/// cannot be (or may not be) made.
fn amap_chunk_get(
    geom: Geom,
    tables: &mut Tables,
    slot: usize,
    create: bool,
    waitf: i32,
) -> Option<&mut Chunk> {
    let (buckets, ncused) = match tables {
        Tables::Small(c) => return Some(c),
        Tables::Normal { buckets, ncused } => (buckets, ncused),
        Tables::Gone => gone(),
    };

    let baseslot = amap_base_slot(slot) as i32;
    let head = &mut buckets[geom.bucket(slot)];
    let at = chain_position(head, baseslot);
    let cur = chain_at(head, at);
    if cur.is_some() {
        return cur.as_deref_mut();
    }
    if !create {
        return None;
    }

    let n = if geom.nslot - baseslot >= UVM_AMAP_CHUNK as i32 {
        UVM_AMAP_CHUNK as i32
    } else {
        geom.nslot - baseslot
    };
    let chunk = CHUNK_POOL.get(waitf, || Chunk::new(baseslot, n as u16))?;
    *cur = Some(chunk);
    *ncused += 1;
    cur.as_deref_mut()
}

/// The chunk for `slot`, if it exists (`amap_chunk_get(amap, slot, 0, PR_NOWAIT)` on a
/// shared view).
fn amap_chunk_find(geom: Geom, tables: &Tables, slot: usize) -> Option<&Chunk> {
    match tables {
        Tables::Small(c) => Some(c),
        Tables::Normal { buckets, .. } => {
            let baseslot = amap_base_slot(slot) as i32;
            let mut cur = buckets[geom.bucket(slot)].as_deref();
            while let Some(c) = cur {
                if c.baseslot == baseslot {
                    return Some(c);
                }
                cur = c.next.as_deref();
            }
            None
        }
        Tables::Gone => gone(),
    }
}

/// `amap_chunk_free`: frees the (empty) chunk of a normal amap whose first slot is
/// `baseslot`; nothing for a small amap.
fn amap_chunk_free(geom: Geom, tables: &mut Tables, baseslot: i32) {
    let Tables::Normal { buckets, ncused } = tables else {
        return;
    };

    let head = &mut buckets[geom.bucket(baseslot as usize)];
    let at = chain_position(head, baseslot);
    let cur = chain_at(head, at);
    let Some(mut chunk) = cur.take() else {
        kpanic(format_args!("amap_chunk_free: no chunk at slot {baseslot}"));
    };
    *cur = chunk.next.take();
    kassert!(chunk.usedmap == 0);
    drop(chunk);
    *ncused -= 1;
}

/// The position in a bucket's chain of the chunk whose first slot is `baseslot`, or the
/// chain's length when it has none.
fn chain_position(head: &Option<PoolBox<Chunk>>, baseslot: i32) -> usize {
    let mut at = 0;
    let mut cur = head.as_deref();
    while let Some(c) = cur {
        if c.baseslot == baseslot {
            break;
        }
        at += 1;
        cur = c.next.as_deref();
    }
    at
}

/// The link at position `at` of a bucket's chain (`at` from [`chain_position`]): the chunk
/// there, or the empty end of the chain.
fn chain_at(head: &mut Option<PoolBox<Chunk>>, at: usize) -> &mut Option<PoolBox<Chunk>> {
    let mut cur = head;
    for _ in 0..at {
        match cur {
            Some(c) => cur = &mut c.next,
            None => break,
        }
    }
    cur
}

/// Empties a bucket's chain chunk by chunk (a whole chain dropped at once would recurse).
fn drain_chain(head: &mut Option<PoolBox<Chunk>>) {
    while let Some(mut c) = head.take() {
        *head = c.next.take();
    }
}

/// `pp_getreflen`: get the reference and length for a specific offset.
///
/// ppref's amap must be locked.
fn pp_getreflen(ppref: &[i32], offset: usize) -> (i32, i32) {
    let v = ppref[offset];
    if v > 0 {
        // chunk size must be 1
        (v - 1, 1) // don't forget to adjust
    } else {
        (-v - 1, ppref[offset + 1])
    }
}

/// `pp_setreflen`: set the reference and length for a specific offset.
///
/// ppref's amap must be locked.
fn pp_setreflen(ppref: &mut [i32], offset: usize, r#ref: i32, len: i32) {
    if len == 1 {
        ppref[offset] = r#ref + 1;
    } else {
        ppref[offset] = -(r#ref + 1);
        ppref[offset + 1] = len;
    }
}

/// `amap_init`: called at boot time to init global amap data structures: the amap pool and
/// the chunk pool at `ipl` (the machine's `IPL_MPFLOOR`), `PR_WAITOK`, each keeping at most
/// 4096 idle items. `ENOMEM` when a pool cannot be made, `EBUSY` when they were.
pub fn amap_init(ipl: i32) -> Result<(), Errno> {
    // Initialize the vm_amap pool.
    AMAP_POOL.init(ipl, PR_WAITOK, "amappl")?;
    AMAP_POOL.set_hiwat(4096);

    // initialize small amap pools: one pool serves every size (see the module's deviations).

    CHUNK_POOL.init(ipl, PR_WAITOK, "amapchunkpl")?;
    CHUNK_POOL.set_hiwat(4096);
    Ok(())
}

/// `amap_alloc1`: allocate an amap of `slots` slots under `lock`, with one reference, but do
/// not initialise the overlay. `None` when memory is short (`waitf` without `M_WAITOK`).
fn amap_alloc1(slots: usize, waitf: i32, lazyalloc: bool, lock: SharedRwlock) -> Option<AmapRef> {
    let mut hashshift = 0u32;
    let mut chunkperbucket = 1usize;
    let pwaitf = pool_waitf(waitf);

    kassert!(slots > 0);

    // Cast to unsigned so that rounding up cannot cause integer overflow if slots is large.
    let chunks = slots.div_ceil(UVM_AMAP_CHUNK);

    if lazyalloc {
        // Basically, the amap is a hash map where the number of buckets is fixed. We select
        // the number of buckets using the following strategy:
        //
        // 1. The maximal number of entries to search in a bucket upon a collision should be
        // less than or equal to log2(slots / UVM_AMAP_CHUNK). This is the worst-case number
        // of lookups we would have if we could chunk the amap. The log2(n) comes from the
        // fact that amaps are chunked by splitting up their vm_map_entries and organizing
        // those in a binary search tree.
        //
        // 2. The maximal number of entries in a bucket must be a power of two.
        //
        // The maximal number of entries per bucket is used to hash a slot to a bucket.
        //
        // In the future, this strategy could be refined to make it even harder/impossible
        // that the total amount of KVA needed for the hash buckets of all amaps to exceed
        // the maximal amount of KVA memory reserved for amaps.
        let mut log_chunks = 1;
        while (chunks >> log_chunks) > 0 {
            log_chunks += 1;
        }

        chunkperbucket = 1usize << hashshift;
        while chunkperbucket + 1 < log_chunks {
            hashshift += 1;
            chunkperbucket = 1usize << hashshift;
        }
    }

    let geom = Geom {
        nslot: slots as i32,
        hashshift: hashshift as i32,
    };
    let small = geom.is_small();
    let mut amap = AMAP_POOL.get_shared(pwaitf, || Amap {
        state: Guarded::new(
            &lock,
            AmapState {
                nused: 0,
                ppref: Ppref::None,
                tables: if small {
                    Tables::Small(Chunk::new(0, slots as u16))
                } else {
                    Tables::Gone
                },
            },
        ),
        lock,
        refs: AtomicI32::new(1),
        flags: AtomicI32::new(0),
        geom,
    })?;
    if small {
        return Some(amap);
    }

    // The buckets; on a failure the amap goes back with its handle (fail1).
    let nbuckets = chunks.div_ceil(chunkperbucket);
    let mut buckets = MArray::new(M_UVMAMAP, waitf, nbuckets, |_| None)?;
    let mut ncused = 0;

    if !lazyalloc {
        for (i, bucket) in buckets.iter_mut().enumerate() {
            let n = if i == nbuckets - 1 {
                match slots % UVM_AMAP_CHUNK {
                    0 => UVM_AMAP_CHUNK,
                    n => n,
                }
            } else {
                UVM_AMAP_CHUNK
            };
            // fail1: the chunks made so far go with the buckets.
            let chunk =
                CHUNK_POOL.get(pwaitf, || Chunk::new((i * UVM_AMAP_CHUNK) as i32, n as u16))?;
            *bucket = Some(chunk);
            ncused += 1;
        }
    }

    let Some(a) = PoolArc::get_mut(&mut amap) else {
        kpanic(format_args!("amap_alloc1: a fresh amap is shared"));
    };
    a.state.get_mut_owned().tables = Tables::Normal { buckets, ncused };
    Some(amap)
}

/// `amap_lock_alloc`: a lock object of its own for an amap, `"amaplk"`.
fn amap_lock_alloc() -> SharedRwlock {
    let Some(lock) = SharedRwlock::new("amaplk") else {
        kpanic(format_args!("rw_obj_alloc: rwlock_obj_pool is empty"));
    };
    lock
}

/// `amap_alloc`: allocate an amap to manage `sz` bytes of anonymous VM.
///
/// The caller should ensure `sz` is a multiple of `PAGE_SIZE`. The amap has one reference
/// (the caller's handle) and is returned unlocked.
pub fn amap_alloc(sz: usize, waitf: i32, lazyalloc: bool) -> Option<AmapRef> {
    let slots = amap_b2slot(sz); // load slots
    if slots > INT_MAX as usize {
        return None;
    }

    amap_alloc1(slots, waitf, lazyalloc, amap_lock_alloc())
}

/// `amap_wipeout`: wipeout all anon's in an amap; then free the amap's tables (its memory
/// goes with the last handle).
///
/// Called from [`amap_unref`], when the reference count drops to zero. The amap must be
/// locked for writing (`w`, released here as the C's `amap_unlock`).
pub fn amap_wipeout(amap: &Amap, mut w: WriteWitness<'_>) {
    kassert!(amap.refs() == 0);

    if amap.flags() & AMAP_SWAPOFF != 0 {
        // Note: amap_swap_off() will call us again.
        drop(w);
        return;
    }

    let lock = amap.lock();
    let st = amap.state.get_mut(&mut w);
    let mut dead = mem::replace(
        st,
        AmapState {
            nused: 0,
            ppref: Ppref::None,
            tables: Tables::Gone,
        },
    );
    let _ = dead.try_each_chunk(|c| {
        let mut map = c.usedmap;
        while map != 0 {
            let slot = map.trailing_zeros() as usize;
            map ^= 1 << slot;

            let Some(anon) = c.anons[slot].take().filter(|a| a.refs() != 0) else {
                kpanic(format_args!("amap_wipeout: corrupt amap"));
            };
            kassert!(anon.shares_lock(lock));

            // Drop the reference.
            let refs = anon.refs() - 1;
            anon.set_refs(refs);
            if refs == 0 {
                uvm_anfree(&anon);
            }
        }
        c.usedmap = 0;
        None::<()>
    });

    // Finally, destroy the amap.
    amap.set_refs(0); // ... was one
    dead.nused = 0;
    drop(w);
    amap_free(dead);
}

/// `amap_free`: frees a wiped-out amap's tables (its `ppref` array, its chunks and buckets).
fn amap_free(mut st: AmapState) {
    kassert!(st.nused == 0);
    if let Tables::Normal { buckets, .. } = &mut st.tables {
        for head in buckets.iter_mut() {
            drain_chain(head);
        }
    }
}

/// `amap_copy` (the amap's half): a copy of `slots` slots of `src` from page `src_pageoff`,
/// for the map entry whose reference to `src` it replaces.
///
/// When the caller's map entry holds the only reference (checked without the lock first, as
/// the C reads `am_ref`, then again under it) the entry takes `src` over. Otherwise the copy
/// shares the source's anons (each one more reference, and one more handle in the copy's
/// slot) and its lock while any anon is shared, and `src` loses the entry's reference. `src`
/// must be unlocked.
pub fn amap_copy_from(src: &Amap, src_pageoff: i32, slots: usize, waitf: i32) -> CopyOutcome {
    // First check and see if we are the only map entry referencing the amap we currently
    // have. If so, then just take it over instead of copying it. Note that we are reading
    // am_ref without lock held as the value can only be one if we have the only reference
    // to the amap (via our locked map). If the value is greater than one, then allocate
    // amap and re-check the value.
    if src.refs() == 1 {
        return CopyOutcome::TakeOver;
    }

    // Allocate a new amap (note: not initialised, etc), sharing the source amap's lock.
    let lazyalloc = !src.is_small() && src.geom.hashshift != 0;
    let Some(mut amap) = amap_alloc1(slots, waitf, lazyalloc, src.lock.clone()) else {
        return CopyOutcome::NoMemory;
    };

    // Lock the source (hence the new amap: the same lock).
    let mut w = src.lock.write();

    // Re-check the reference count with the lock held. If it has dropped to one - we can
    // take over the existing map.
    if src.refs() == 1 {
        // Just take over the existing amap.
        drop(w);
        // Destroy the new (unused) amap: its memory and its lock reference go with the
        // handle.
        amap.refs.store(0, Ordering::Relaxed);
        drop(amap);
        return CopyOutcome::TakeOver;
    }

    // Copy the slots, into the new amap through its one handle.
    let copied = {
        let Some(dst) = PoolArc::get_mut(&mut amap) else {
            kpanic(format_args!("amap_copy: a fresh amap is shared"));
        };
        let dst_geom = dst.geom;
        let dst_st = dst.state.get_mut_owned();
        let src_st = src.state.get(&w);
        let mut nused = 0;
        let mut ok = true;
        let mut lcv = 0;
        while lcv < slots {
            let srcslot = src_pageoff as usize + lcv;
            let i = uvm_amap_slotidx(lcv);
            let j = uvm_amap_slotidx(srcslot);
            let mut n = UVM_AMAP_CHUNK - i.max(j);
            if lcv + n > slots {
                n = slots - lcv;
            }

            let Some(srcchunk) = amap_chunk_find(src.geom, &src_st.tables, srcslot) else {
                lcv += n;
                continue;
            };

            let Some(chunk) = amap_chunk_get(dst_geom, &mut dst_st.tables, lcv, true, PR_NOWAIT)
            else {
                ok = false;
                break;
            };

            for k in 0..n {
                let (i, j) = (i + k, j + k);
                chunk.anons[i] = srcchunk.anons[j].clone();
                let Some(anon) = chunk.anons[i].as_ref() else {
                    continue;
                };
                kassert!(anon.shares_lock(&src.lock));
                kassert!(anon.refs() > 0);
                chunk.usedmap |= 1 << i;
                anon.set_refs(anon.refs() + 1);
                kassert!(anon.refs() as usize <= PoolArc::count(anon));
                nused += 1;
            }
            lcv += n;
        }
        dst_st.nused += nused;
        ok.then_some(dst_st.nused)
    };

    let Some(nused) = copied else {
        // No chunk for the copy: amap_wipeout() drops what was copied and releases the
        // shared lock.
        amap.refs.store(0, Ordering::Relaxed);
        amap_wipeout(&amap, w);
        return CopyOutcome::NoMemory;
    };

    // Drop our reference to the old amap (srcamap) and unlock. Since the reference count on
    // srcamap is greater than one, (we checked above), it cannot drop to zero while it is
    // locked.
    src.set_refs(src.refs() - 1);
    kassert!(src.refs() > 0);

    if src.refs() == 1 && src.flags() & AMAP_SHARED != 0 {
        src.set_flags(src.flags() & !AMAP_SHARED); // clear shared flag
    }
    if matches!(src.state.get(&w).ppref, Ppref::Some(_)) {
        amap_pp_adjref(src, &mut w, src_pageoff as usize, slots, -1);
    }

    // If we referenced any anons, then share the source amap's lock. Otherwise, we have
    // nothing in common, so allocate a new one.
    kassert!(SharedRwlock::same(&amap.lock, &src.lock));
    drop(w);

    if nused == 0 {
        let lock = amap_lock_alloc();
        let Some(dst) = PoolArc::get_mut(&mut amap) else {
            kpanic(format_args!("amap_copy: a fresh amap is shared"));
        };
        dst.lock = lock;
        dst.state.rekey(&dst.lock);
    }

    CopyOutcome::New(amap)
}

/// What [`amap_cow_now`]'s walk stopped for.
enum CowStep {
    /// Every shared anon was copied.
    Done,
    /// A busy page of the anon held by the handle: wait for it and restart.
    Wait(&'static VmPage, AnonRef),
    /// No anon or page for a copy (the anon made, if any): wait for memory and restart.
    NoMemory(Option<AnonRef>),
}

/// `amap_cow_now`: resolve all copy-on-write faults in an amap now for fork(2).
///
/// Called during fork(2) when the parent process has a wired map entry. In that case we want
/// to avoid write-protecting pages in the parent's map (e.g. like what you'd do for a COW
/// page) so we resolve the COW here.
///
/// Assume parent's entry was wired, thus all pages are resident. The parent and child vm_map
/// must both be locked. Caller passes the child's entry's amap in to us. XXXCDC: out of
/// memory should cause fork to fail, but there is currently no easy way to do this (needs
/// fix).
pub fn amap_cow_now(amap: &Amap) {
    // note that if we unlock the amap then we must ReStart the "lcv" for loop because some
    // other process could reorder the anon's in the am_anon[] array on us while the lock is
    // dropped.
    loop {
        // ReStart:
        let mut w = amap.lock.write();
        let lock = amap.lock();
        let step = amap.state.get_mut(&mut w).try_each_chunk(|c| {
            let mut map = c.usedmap;
            while map != 0 {
                let slot = map.trailing_zeros() as usize;
                map ^= 1 << slot;
                let Some(anon) = c.anons[slot].as_ref() else {
                    kpanic(format_args!("amap_cow_now: corrupt amap"));
                };
                kassert!(anon.shares_lock(lock));

                // The old page must be resident since the parent is wired.
                let Some(pg) = anon.page() else {
                    kpanic(format_args!("amap_cow_now: anon without a page"));
                };

                // if the anon ref count is one, we are safe (the child has exclusive access
                // to the page).
                if anon.refs() <= 1 {
                    continue;
                }

                // If the page is busy, then we have to unlock, wait for it and then restart.
                if pg.flags() & PG_BUSY != 0 {
                    return Some(CowStep::Wait(pg, anon.clone()));
                }

                // Perform a copy-on-write. First - get a new anon (it will share the amap's
                // lock) and a page.
                let nanon = uvm_analloc(lock);
                let npg = nanon
                    .as_ref()
                    .and_then(|n| uvm_pagealloc(Owner::Anon(&**n), 0, 0));
                let (nanon, npg) = match (nanon, npg) {
                    (Some(nanon), Some(npg)) => (nanon, npg),
                    // out of memory
                    (nanon, _) => return Some(CowStep::NoMemory(nanon)),
                };

                // Copy the data and replace anon with the new one; the old anon keeps its
                // other slots' handles.
                uvm_pagecopy(pg, npg);
                anon.set_refs(anon.refs() - 1);
                kassert!(anon.refs() > 0);
                c.anons[slot] = Some(nanon);

                // Drop PG_BUSY on new page. Since its owner was write locked all this time -
                // it cannot be PG_RELEASED or PG_WANTED.
                npg.clear_bits(PG_BUSY | PG_FAKE);
                // UVM_PAGE_OWN(npg, NULL): not configured.
                if let Some(nanon) = c.anons[slot].as_ref() {
                    uvm_pageactivate(npg, Owner::Anon(&**nanon));
                }
            }
            None
        });

        match step.unwrap_or(CowStep::Done) {
            CowStep::Done => {
                drop(w);
                return;
            }
            CowStep::Wait(pg, anon) => {
                uvm_pagewait(pg, Owner::Anon(&*anon), w, "cownow");
            }
            CowStep::NoMemory(nanon) => {
                // The anon no amap saw goes first, under the lock it shares (the C frees it
                // just after the unlock, its lock cleared: nothing else ever named it).
                if let Some(nanon) = nanon {
                    nanon.set_refs(nanon.refs() - 1);
                    kassert!(nanon.refs() == 0);
                    uvm_anfree(&nanon);
                }
                drop(w);
                uvm_wait("cownowpage");
            }
        }
    }
}

/// `amap_splitref`: split a single reference to `amap`, at page `pageoff`, into two separate
/// references, the second `offset` bytes in: one more reference to the amap (not a share
/// reference), whose page offset is returned. The caller's second map entry holds the
/// handle it already cloned.
///
/// Called from uvm_map's clip routines. The map should be locked; the amap should be
/// unlocked (we will lock).
pub fn amap_splitref(amap: &Amap, pageoff: i32, offset: usize) -> i32 {
    let leftslots = amap_b2slot(offset) as i32;
    if leftslots == 0 {
        kpanic(format_args!("amap_splitref: split at zero offset"));
    }

    let mut w = amap.lock.write();

    if amap.nslot() - pageoff - leftslots <= 0 {
        kpanic(format_args!("amap_splitref: map size check failed"));
    }

    // Establish ppref before we add a duplicate reference to the amap.
    if matches!(amap.state.get(&w).ppref, Ppref::None) {
        amap_pp_establish(amap, &mut w);
    }

    // Note: not a share reference.
    amap.set_refs(amap.refs() + 1);
    drop(w);
    pageoff + leftslots
}

/// `amap_pp_establish`: add a ppref array to an amap, if possible.
///
/// The amap should be locked for writing by the caller (`w`).
fn amap_pp_establish(amap: &Amap, w: &mut WriteWitness<'_>) {
    let refs = amap.refs();
    let nslot = amap.nslot();
    let st = amap.state.get_mut(w);
    st.ppref = match MArray::new(M_UVMAMAP, M_NOWAIT, nslot as usize, |_| 0) {
        // Failure - just do not use ppref.
        None => Ppref::Failed,
        Some(mut ppref) => {
            pp_setreflen(&mut ppref, 0, refs, nslot);
            Ppref::Some(ppref)
        }
    };
}

/// `amap_pp_adjref`: adjust reference count to a part of an amap using the per-page
/// reference count array; a part whose count drops to zero is wiped out.
///
/// The caller must check that the array is in use. The map and the amap must be locked
/// (`w`).
fn amap_pp_adjref(
    amap: &Amap,
    w: &mut WriteWitness<'_>,
    curslot: usize,
    slotlen: usize,
    adjval: i32,
) {
    let lock = amap.lock();
    let geom = amap.geom;
    let AmapState {
        nused,
        ppref,
        tables,
    } = amap.state.get_mut(w);
    let Ppref::Some(ppref) = ppref else {
        kpanic(format_args!("amap_pp_adjref: no ppref"));
    };
    let ppref: &mut [i32] = ppref;

    let stopslot = curslot + slotlen;
    let mut prevlcv = 0;

    // Advance to the correct place in the array, fragment if needed.
    let mut lcv = 0;
    let mut len;
    while lcv < curslot {
        let (r#ref, l) = pp_getreflen(ppref, lcv);
        len = l as usize;
        if lcv + len > curslot {
            // goes past start?
            pp_setreflen(ppref, lcv, r#ref, (curslot - lcv) as i32);
            pp_setreflen(ppref, curslot, r#ref, (len - (curslot - lcv)) as i32);
            len = curslot - lcv; // new length of entry @ lcv
        }
        prevlcv = lcv;
        lcv += len;
    }
    let (prevref, prevlen) = if lcv != 0 {
        pp_getreflen(ppref, prevlcv)
    } else {
        // Ensure that the "prevref == ref" test below always fails, since we are starting
        // from the beginning of the ppref array; that is, there is no previous chunk.
        (-1, 0)
    };

    // Now adjust reference counts in range. Merge the first changed entry with the last
    // unchanged entry if possible.
    if lcv != curslot {
        kpanic(format_args!("amap_pp_adjref: overshot target"));
    }

    while lcv < stopslot {
        let (mut r#ref, l) = pp_getreflen(ppref, lcv);
        len = l as usize;
        if lcv + len > stopslot {
            // goes past end?
            pp_setreflen(ppref, lcv, r#ref, (stopslot - lcv) as i32);
            pp_setreflen(ppref, stopslot, r#ref, (len - (stopslot - lcv)) as i32);
            len = stopslot - lcv;
        }
        r#ref += adjval;
        if r#ref < 0 {
            kpanic(format_args!("amap_pp_adjref: negative reference count"));
        }
        if lcv == prevlcv + prevlen as usize && r#ref == prevref {
            pp_setreflen(ppref, prevlcv, r#ref, prevlen + len as i32);
        } else {
            pp_setreflen(ppref, lcv, r#ref, len as i32);
        }
        if r#ref == 0 {
            amap_wiperange(geom, lock, tables, nused, lcv, len);
        }
        lcv += len;
    }
}

/// `amap_wiperange_chunk`: drops the anons of `[slotoff, slotoff + slots)` that `chunk`
/// holds.
fn amap_wiperange_chunk(
    lock: &SharedRwlock,
    nused: &mut i32,
    chunk: &mut Chunk,
    slotoff: usize,
    slots: usize,
) {
    let startbase = amap_base_slot(slotoff) as i32;
    let endbase = amap_base_slot(slotoff + slots - 1) as i32;

    let mut map = chunk.usedmap;
    if startbase == chunk.baseslot {
        map &= !(((1u32 << (slotoff - startbase as usize)) - 1) as u16);
    }
    if endbase == chunk.baseslot {
        map &= ((1u32 << (slotoff + slots - endbase as usize)) - 1) as u16;
    }

    while map != 0 {
        let curslot = map.trailing_zeros() as usize;
        map ^= 1 << curslot;
        chunk.usedmap ^= 1 << curslot;

        // remove it from the amap
        let Some(anon) = chunk.anons[curslot].take() else {
            kpanic(format_args!("amap_wiperange_chunk: empty slot in use"));
        };
        kassert!(anon.shares_lock(lock));

        *nused -= 1;

        // drop anon reference count
        let refs = anon.refs() - 1;
        anon.set_refs(refs);
        if refs == 0 {
            uvm_anfree(&anon);
        }

        // done with this anon, next ...!
    }
}

/// `amap_wiperange`: wipe out a range of an amap. Note: different from amap_wipeout because
/// the amap is kept intact.
///
/// Both map and amap must be locked by the caller.
fn amap_wiperange(
    geom: Geom,
    lock: &SharedRwlock,
    tables: &mut Tables,
    nused: &mut i32,
    slotoff: usize,
    slots: usize,
) {
    kassert!(lock.status() == RW_WRITE);

    let (buckets, ncused) = match tables {
        Tables::Small(c) => {
            amap_wiperange_chunk(lock, nused, c, slotoff, slots);
            return;
        }
        Tables::Normal { buckets, ncused } => (buckets, ncused),
        Tables::Gone => gone(),
    };

    // The chunks of the range are in the buckets of the range: walk each one's chain, the
    // emptied chunks freed on the way.
    let startbucket = geom.bucket(slotoff);
    let endbucket = geom.bucket(slotoff + slots - 1);
    for head in &mut buckets[startbucket..=endbucket] {
        let mut cur = head;
        loop {
            let emptied = match cur.as_deref_mut() {
                None => break,
                Some(c) => {
                    let base = c.baseslot as usize;
                    if base + c.nslot as usize > slotoff && base < slotoff + slots {
                        amap_wiperange_chunk(lock, nused, c, slotoff, slots);
                        c.usedmap == 0
                    } else {
                        false
                    }
                }
            };
            if emptied {
                // amap_chunk_free: the chunk leaves its chain, its next takes its place.
                if let Some(mut dead) = cur.take() {
                    *cur = dead.next.take();
                    *ncused -= 1;
                }
                continue;
            }
            match cur {
                Some(c) => cur = &mut c.next,
                None => break,
            }
        }
    }
}

/// `amap_swap_off`: pagein anonymous pages in amaps and drop swap slots. Not here (N3.5, see
/// the module's deviations): reported once; nothing failed.
///
/// Returns true if failed.
pub fn amap_swap_off(startslot: i32, endslot: i32) -> bool {
    let _ = (startslot, endslot, uvm_anon_pagein);
    let _ = unported!("amap_swap_off (uvm_swap, N3.5)");
    false
}

/// `amap_lookup`: look up the anon of the page `offset` bytes into the reference to `amap`
/// at page `pageoff`.
///
/// The amap should be locked by the caller (`w`, either kind); the reference borrows the
/// witness (a caller that keeps the anon across a sleep clones the handle).
pub fn amap_lookup<'a, W: Witness>(
    amap: &'a Amap,
    w: &'a W,
    pageoff: i32,
    offset: usize,
) -> Option<&'a AnonRef> {
    let slot = amap_b2slot(offset) + pageoff as usize;
    kassert!(slot < amap.nslot() as usize);

    let chunk = amap_chunk_find(amap.geom, &amap.state.get(w).tables, slot)?;

    chunk.anons[uvm_amap_slotidx(slot)].as_ref()
}

/// `amap_lookups`: look up the anons of `anons.len()` pages from `offset` bytes into the
/// reference to `amap` at page `pageoff`.
///
/// The amap should be locked by the caller (`w`). XXXCDC: this interface is biased toward
/// array-based amaps. fix.
pub fn amap_lookups<'a, W: Witness>(
    amap: &'a Amap,
    w: &'a W,
    pageoff: i32,
    offset: usize,
    anons: &mut [Option<&'a AnonRef>],
) {
    let npages = anons.len();
    let slot = amap_b2slot(offset) + pageoff as usize;

    kassert!(slot + (npages - 1) < amap.nslot() as usize);

    let tables = &amap.state.get(w).tables;
    let mut i = 0;
    let mut lcv = slot;
    while lcv < slot + npages {
        let mut n = UVM_AMAP_CHUNK - uvm_amap_slotidx(lcv);
        if lcv + n > slot + npages {
            n = slot + npages - lcv;
        }

        match amap_chunk_find(amap.geom, tables, lcv) {
            None => anons[i..i + n].fill(None),
            Some(chunk) => {
                let base = uvm_amap_slotidx(lcv);
                for (k, a) in anons[i..i + n].iter_mut().enumerate() {
                    *a = chunk.anons[base + k].as_ref();
                }
            }
        }
        i += n;
        lcv += n;
    }
}

/// `amap_populate`: ensure that the amap can store an anon for the page `offset` bytes into
/// the reference at page `pageoff`. This function can sleep until memory to store the anon
/// is available. The amap must be locked for writing (`w`).
pub fn amap_populate(amap: &Amap, w: &mut WriteWitness<'_>, pageoff: i32, offset: usize) {
    let slot = amap_b2slot(offset) + pageoff as usize;
    kassert!(slot < amap.nslot() as usize);

    let chunk = amap_chunk_get(
        amap.geom,
        &mut amap.state.get_mut(w).tables,
        slot,
        true,
        PR_WAITOK,
    );
    kassert!(chunk.is_some());
}

/// A chunk made with no lock held ([`amap_chunk_alloc`]), for [`amap_populate_with`].
pub struct SpareChunk(PoolBox<Chunk>);

/// `amap_populate`'s allocation, with nothing held (it may sleep with `PR_WAITOK`): a chunk
/// for the page `offset` bytes into the reference at page `pageoff`; `None` for a small amap
/// (its chunk is embedded) or when the pool has no item now (`PR_NOWAIT`).
pub fn amap_chunk_alloc(
    amap: &Amap,
    pageoff: i32,
    offset: usize,
    waitf: i32,
) -> Option<SpareChunk> {
    let slot = amap_b2slot(offset) + pageoff as usize;
    kassert!(slot < amap.nslot() as usize);
    if amap.is_small() {
        return None;
    }
    let baseslot = amap_base_slot(slot) as i32;
    let n = (amap.geom.nslot - baseslot).min(UVM_AMAP_CHUNK as i32);
    CHUNK_POOL
        .get(waitf, || Chunk::new(baseslot, n as u16))
        .map(SpareChunk)
}

/// `amap_populate`'s insertion: the slot of the page `offset` bytes into the reference at
/// page `pageoff` gets the `spare` chunk, unless its chunk exists already (made meanwhile by
/// another fault: the spare goes back to its pool). The amap must be locked for writing (`w`)
/// and not wiped out (`am_ref` above 0, which the caller checks under `w`).
pub fn amap_populate_with(
    amap: &Amap,
    w: &mut WriteWitness<'_>,
    pageoff: i32,
    offset: usize,
    spare: SpareChunk,
) {
    let slot = amap_b2slot(offset) + pageoff as usize;
    kassert!(slot < amap.nslot() as usize);
    let geom = amap.geom;

    match &mut amap.state.get_mut(w).tables {
        // the embedded chunk serves every slot
        Tables::Small(_) => {}
        Tables::Normal { buckets, ncused } => {
            let baseslot = amap_base_slot(slot) as i32;
            kassert!(spare.0.baseslot == baseslot);
            let head = &mut buckets[geom.bucket(slot)];
            let at = chain_position(head, baseslot);
            let cur = chain_at(head, at);
            if cur.is_none() {
                *cur = Some(spare.0);
                *ncused += 1;
            }
        }
        Tables::Gone => gone(),
    }
}

/// `amap_add`: add (or `replace`) the anon of the page `offset` bytes into the reference at
/// page `pageoff`, the caller's handle moving into the slot.
///
/// The amap must be locked for writing (`w`); the anon must have the lock associated with
/// this amap. `Ok` with the replaced anon's handle (`None` when adding), whose reference
/// count the caller drops; `Err` with the anon back when no chunk could be had (the C's 1).
pub fn amap_add(
    amap: &Amap,
    w: &mut WriteWitness<'_>,
    pageoff: i32,
    offset: usize,
    anon: AnonRef,
    replace: bool,
) -> Result<Option<AnonRef>, AnonRef> {
    let slot = amap_b2slot(offset) + pageoff as usize;
    kassert!(slot < amap.nslot() as usize);
    kassert!(anon.shares_lock(&amap.lock));

    let shared = amap.flags() & AMAP_SHARED != 0;
    let AmapState { nused, tables, .. } = amap.state.get_mut(w);
    let Some(chunk) = amap_chunk_get(amap.geom, tables, slot, true, PR_NOWAIT) else {
        return Err(anon);
    };

    let slot = uvm_amap_slotidx(slot);
    if replace {
        let Some(oanon) = chunk.anons[slot].take() else {
            kpanic(format_args!("amap_add: replacing an empty slot"));
        };
        if let Some(opg) = oanon.page()
            && shared
        {
            hal::pmap::page_protect(&opg.mdpage, PROT_NONE);
            // XXX: suppose page is supposed to be wired somewhere?
        }
        chunk.anons[slot] = Some(anon);
        Ok(Some(oanon))
    } else {
        // !replace
        if chunk.anons[slot].is_some() {
            kpanic(format_args!("amap_add: slot in use"));
        }

        chunk.usedmap |= 1 << slot;
        *nused += 1;
        chunk.anons[slot] = Some(anon);
        Ok(None)
    }
}

/// `amap_unadd`: remove the anon of the page `offset` bytes into the reference at page
/// `pageoff` from the amap, its handle handed back (the caller drops its reference count).
///
/// The amap must be locked for writing (`w`).
pub fn amap_unadd(amap: &Amap, w: &mut WriteWitness<'_>, pageoff: i32, offset: usize) -> AnonRef {
    let slot = amap_b2slot(offset) + pageoff as usize;
    kassert!(slot < amap.nslot() as usize);

    let AmapState { nused, tables, .. } = amap.state.get_mut(w);
    let Some(chunk) = amap_chunk_get(amap.geom, tables, slot, false, PR_NOWAIT) else {
        kpanic(format_args!("amap_unadd: no chunk"));
    };

    let slot = uvm_amap_slotidx(slot);
    let Some(anon) = chunk.anons[slot].take() else {
        kpanic(format_args!("amap_unadd: empty slot"));
    };
    chunk.usedmap &= !(1 << slot);
    *nused -= 1;

    if chunk.usedmap == 0 {
        let baseslot = chunk.baseslot;
        amap_chunk_free(amap.geom, tables, baseslot);
    }
    anon
}

/// `amap_adjref_anons`: adjust the reference count(s) on amap and its anons, then unlock
/// (`w` is consumed).
fn amap_adjref_anons(
    amap: &Amap,
    mut w: WriteWitness<'_>,
    offset: usize,
    len: usize,
    refv: i32,
    all: bool,
) {
    // We must establish the ppref array before changing am_ref so that the ppref values
    // match the current amap refcount.
    if matches!(amap.state.get(&w).ppref, Ppref::None) && !all && len != amap.nslot() as usize {
        amap_pp_establish(amap, &mut w);
    }
    amap.set_refs(amap.refs() + refv);

    if matches!(amap.state.get(&w).ppref, Ppref::Some(_)) {
        if all {
            amap_pp_adjref(amap, &mut w, 0, amap.nslot() as usize, refv);
        } else {
            amap_pp_adjref(amap, &mut w, offset, len, refv);
        }
    }
    drop(w);
}

/// `amap_ref`: gain a reference to an amap, `len` pages from page `offset` (all of it with
/// `AMAP_REFALL`), shared with `AMAP_SHARED`. The caller's new map entry holds a clone of the
/// caller's handle.
///
/// The amap must not be locked (we will lock). Called at fork time to gain the child's
/// reference.
pub fn amap_ref(amap: &Amap, offset: usize, len: usize, flags: i32) {
    let w = amap.lock.write();
    if flags & AMAP_SHARED != 0 {
        amap.set_flags(amap.flags() | AMAP_SHARED);
    }
    amap_adjref_anons(amap, w, offset, len, 1, flags & AMAP_REFALL != 0);
}

/// `amap_unref`: remove a reference to an amap, `len` pages from page `offset` (`all` of
/// it); the last one wipes the amap out. The caller drops its map entry's handle after it.
///
/// All pmap-level references to this amap must be already removed. Called from
/// `uvm_unmap_detach()`; the entry is already removed from the map. We will lock the amap, so
/// it must be unlocked.
pub fn amap_unref(amap: &Amap, offset: usize, len: usize, all: bool) {
    let w = amap.lock.write();

    kassert!(amap.refs() > 0);

    if amap.refs() == 1 {
        // If the last reference - wipeout and destroy the amap (no list to leave first: see
        // the module's notes).
        amap.set_refs(0);
        amap_wipeout(amap, w);
        return;
    }

    // Otherwise, drop the reference count(s) on anons.
    if amap.refs() == 2 && amap.flags() & AMAP_SHARED != 0 {
        amap.set_flags(amap.flags() & !AMAP_SHARED);
    }
    amap_adjref_anons(amap, w, offset, len, -1, all);
}

/// The amap pool's and the chunk pool's items out (`pr_nout`), for the tests.
#[cfg(test)]
pub(crate) fn amaps_out() -> (u64, u64) {
    (AMAP_POOL.nout(), CHUNK_POOL.nout())
}

const _: () = {
    // A chunk is the C's pool item: the 24 bytes of `struct vm_amap_chunk` (its `TAILQ` entry
    // and three fields, where the chain needs one link and the handle's pool) and 16 anons.
    assert!(size_of::<Chunk>() == 24 + UVM_AMAP_CHUNK * 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use super::*;
    use crate::testutil;
    use crate::vm::anon::{Anon, anons_out, uvm_anon_init};
    use crate::vm::page::PG_CLEAN;

    /// Real page memory, the lock objects, the anon pool and the amap pools.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(256 * 4096);
        let _ = hal::rwlock::init_shared();
        let _ = uvm_anon_init(0);
        let _ = amap_init(0);
        guard
    }

    /// An anon that shares `amap`'s lock, as the fault handler makes them.
    fn anon_for(amap: &Amap) -> AnonRef {
        uvm_analloc(amap.lock()).expect("an anon")
    }

    /// An anon of `amap` with a resident, unbusied page.
    fn anon_with_page(amap: &Amap, w: &WriteWitness<'_>) -> AnonRef {
        let _ = w;
        let anon = anon_for(amap);
        let pg = uvm_pagealloc(Owner::Anon(&*anon), 0, 0).expect("a page");
        pg.clear_bits(PG_BUSY | PG_FAKE);
        anon
    }

    fn the_page(anon: &Anon) -> &'static VmPage {
        anon.page().expect("a page")
    }

    #[test]
    fn small_amap_add_lookup_unadd() {
        let _g = setup();
        let amap = amap_alloc(4 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        assert!(amap.is_small());
        assert_eq!(amap.nslot(), 4);
        assert_eq!(amap.refs(), 1);

        let anon = anon_for(&amap);
        let a = anon.clone();
        let mut w = amap.lock().write();
        assert!(amap_add(&amap, &mut w, 0, 2 * PAGE_SIZE, anon, false).is_ok_and(|o| o.is_none()));
        assert_eq!(amap.state(&w).nused(), 1);
        assert!(amap_lookup(&amap, &w, 0, 2 * PAGE_SIZE).is_some_and(|x| PoolArc::ptr_eq(x, &a)));
        assert!(amap_lookup(&amap, &w, 0, 0).is_none());

        let mut anons: [Option<&AnonRef>; 4] = [None; 4];
        amap_lookups(&amap, &w, 0, 0, &mut anons);
        assert!(anons[0].is_none() && anons[1].is_none() && anons[3].is_none());
        assert!(anons[2].is_some_and(|x| PoolArc::ptr_eq(x, &a)));

        let back = amap_unadd(&amap, &mut w, 0, 2 * PAGE_SIZE);
        assert!(PoolArc::ptr_eq(&back, &a));
        assert_eq!(amap.state(&w).nused(), 0);
        assert!(amap_lookup(&amap, &w, 0, 2 * PAGE_SIZE).is_none());
        // the anon left the amap with its reference: drop it under the shared lock
        back.set_refs(0);
        uvm_anfree(&back);
        drop(w);

        amap_unref(&amap, 0, 4, true);
        assert_eq!(amap.refs(), 0);
    }

    #[test]
    fn large_amap_chunks_and_buckets() {
        let _g = setup();
        let slots = UVM_AMAP_LARGE + 5;
        let (_, chunks0) = amaps_out();

        // eager: every chunk exists, the last one short
        let eager = amap_alloc(slots * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        assert!(!eager.is_small());
        assert_eq!(eager.geom.hashshift, 0);
        {
            let w = eager.lock().read();
            let Tables::Normal { buckets, ncused } = &eager.state.get(&w).tables else {
                panic!("a normal amap");
            };
            assert_eq!(buckets.len(), 17);
            assert_eq!(*ncused, 17);
            let last = buckets[16].as_deref().expect("a chunk");
            assert_eq!(last.baseslot(), 256);
            assert_eq!(last.nslot(), 5);
        }
        assert_eq!(amaps_out().1, chunks0 + 17);
        amap_unref(&eager, 0, slots, true);
        assert_eq!(amaps_out().1, chunks0, "the wipeout freed the chunks");
        drop(eager);

        // lazy: 17 chunks hash into 5 buckets of 4 (log2(17) rounds to 5, so 4 + 1 < 5 fails)
        let lazy = amap_alloc(slots * PAGE_SIZE, M_WAITOK, true).expect("an amap");
        assert_eq!(lazy.geom.hashshift, 2);
        let ncused = |w: &WriteWitness<'_>| match &lazy.state.get(w).tables {
            Tables::Normal { buckets, ncused } => {
                assert_eq!(buckets.len(), 5);
                *ncused
            }
            _ => panic!("a normal amap"),
        };
        let (a0, a1) = (anon_for(&lazy), anon_for(&lazy));
        let a1c = a1.clone();
        let mut w = lazy.lock().write();
        assert_eq!(ncused(&w), 0);
        assert!(amap_add(&lazy, &mut w, 0, 0, a0, false).is_ok());
        assert!(amap_add(&lazy, &mut w, 0, 260 * PAGE_SIZE, a1, false).is_ok());
        assert_eq!(ncused(&w), 2);
        assert_eq!(lazy.state(&w).nused(), 2);
        assert!(
            amap_lookup(&lazy, &w, 0, 260 * PAGE_SIZE).is_some_and(|x| PoolArc::ptr_eq(x, &a1c))
        );
        assert!(amap_lookup(&lazy, &w, 0, 100 * PAGE_SIZE).is_none());
        let a0 = amap_unadd(&lazy, &mut w, 0, 0);
        assert_eq!(ncused(&w), 1, "an emptied chunk goes back to the pool");
        a0.set_refs(0);
        uvm_anfree(&a0);
        drop(w);
        amap_unref(&lazy, 0, slots, true);
    }

    #[test]
    fn two_chunks_of_one_bucket_chain_in_order_and_leave_it() {
        let _g = setup();
        // 64 chunks hash with a shift of 3: chunks 0..8 share bucket 0.
        let slots = 64 * UVM_AMAP_CHUNK;
        let amap = amap_alloc(slots * PAGE_SIZE, M_WAITOK, true).expect("an amap");
        assert_eq!(amap.geom.hashshift, 3);
        let mut w = amap.lock().write();
        for chunk in [3, 1, 5] {
            let a = anon_for(&amap);
            let off = chunk * UVM_AMAP_CHUNK * PAGE_SIZE;
            assert!(amap_add(&amap, &mut w, 0, off, a, false).is_ok());
        }
        let chain = |w: &WriteWitness<'_>| {
            let Tables::Normal { buckets, .. } = &amap.state.get(w).tables else {
                panic!("a normal amap");
            };
            let mut v = std::vec::Vec::new();
            let mut cur = buckets[0].as_deref();
            while let Some(c) = cur {
                v.push(c.baseslot());
                cur = c.next.as_deref();
            }
            v
        };
        assert_eq!(
            chain(&w),
            [48, 16, 80],
            "a new chunk joins the end of its bucket"
        );
        let a = amap_unadd(&amap, &mut w, 0, 16 * PAGE_SIZE);
        assert_eq!(
            chain(&w),
            [48, 80],
            "the middle chunk leaves, its next takes its place"
        );
        a.set_refs(0);
        uvm_anfree(&a);
        drop(w);
        amap_unref(&amap, 0, slots, true);
    }

    #[test]
    fn ppref_counts_partial_references() {
        let _g = setup();
        let amap = amap_alloc(8 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        amap_ref(&amap, 0, 8, AMAP_REFALL);
        assert_eq!(amap.refs(), 2);
        let ppref = |amap: &Amap| {
            let w = amap.lock().read();
            match &amap.state.get(&w).ppref {
                Ppref::Some(p) => Some(p.to_vec()),
                _ => None,
            }
        };
        assert!(ppref(&amap).is_none(), "a whole reference needs no ppref");

        amap_ref(&amap, 2, 4, 0);
        assert_eq!(amap.refs(), 3);
        let p = ppref(&amap).expect("a ppref array");
        assert_eq!(pp_getreflen(&p, 0), (2, 2));
        assert_eq!(pp_getreflen(&p, 2), (3, 4));
        assert_eq!(pp_getreflen(&p, 6), (2, 2));

        amap_unref(&amap, 2, 4, false);
        assert_eq!(amap.refs(), 2);
        let p = ppref(&amap).expect("a ppref array");
        assert_eq!(
            pp_getreflen(&p, 0),
            (2, 6),
            "the first changed entry merges with the unchanged one before it"
        );
        assert_eq!(pp_getreflen(&p, 6), (2, 2));

        amap_unref(&amap, 0, 8, true);
        assert_eq!(amap.refs(), 1);
        amap_unref(&amap, 0, 8, true);
    }

    #[test]
    fn ppref_matches_the_c_example() {
        // The C's comment: REFS 2 2 2 2 3 1 1 0 0 0 4 4 0 1 1 1.
        let mut p = [0i32; 16];
        for (off, r, len) in [
            (0, 2, 4),
            (4, 3, 1),
            (5, 1, 2),
            (7, 0, 3),
            (10, 4, 2),
            (12, 0, 1),
            (13, 1, 3),
        ] {
            pp_setreflen(&mut p, off, r, len);
        }
        assert_eq!(p[0], -3);
        assert_eq!(p[1], 4);
        assert_eq!(p[4], 4);
        assert_eq!((p[5], p[6]), (-2, 2));
        assert_eq!((p[7], p[8]), (-1, 3));
        assert_eq!((p[10], p[11]), (-5, 2));
        assert_eq!(p[12], 1);
        assert_eq!((p[13], p[14]), (-2, 3));
        assert_eq!(pp_getreflen(&p, 12), (0, 1));
        assert_eq!(pp_getreflen(&p, 10), (4, 2));
    }

    #[test]
    fn a_partial_reference_dropped_to_zero_wipes_its_range() {
        let _g = setup();
        let amap = amap_alloc(8 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        let (a2, a5) = (anon_for(&amap), anon_for(&amap));
        let (k2, k5) = (a2.clone(), a5.clone());
        {
            let mut w = amap.lock().write();
            assert!(amap_add(&amap, &mut w, 0, 2 * PAGE_SIZE, a2, false).is_ok());
            assert!(amap_add(&amap, &mut w, 0, 5 * PAGE_SIZE, a5, false).is_ok());
        }
        // The entry gives up pages 0..4 (an unmap of the front): their anons go.
        amap_unref(&amap, 0, 4, false);
        assert_eq!(amap.refs(), 0, "the C's am_ref counts entries, not pages");
        // am_ref 1 -> 0 above wiped the whole amap; redo with two references.
        let amap = amap_alloc(8 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        let (a2, a5) = (anon_for(&amap), anon_for(&amap));
        let (k2b, k5b) = (a2.clone(), a5.clone());
        {
            let mut w = amap.lock().write();
            assert!(amap_add(&amap, &mut w, 0, 2 * PAGE_SIZE, a2, false).is_ok());
            assert!(amap_add(&amap, &mut w, 0, 5 * PAGE_SIZE, a5, false).is_ok());
        }
        amap_splitref(&amap, 0, 4 * PAGE_SIZE); // two entries: pages 0..4 and 4..8
        assert_eq!(amap.refs(), 2);
        amap_unref(&amap, 0, 4, false); // the first entry goes
        assert_eq!(amap.refs(), 1);
        {
            let w = amap.lock().read();
            assert!(
                amap_lookup(&amap, &w, 0, 2 * PAGE_SIZE).is_none(),
                "page 2 wiped"
            );
            assert!(
                amap_lookup(&amap, &w, 0, 5 * PAGE_SIZE).is_some(),
                "page 5 kept"
            );
            assert_eq!(amap.state(&w).nused(), 1);
        }
        assert_eq!(k2b.refs(), 0, "the wiped anon lost its reference");
        assert_eq!(k5b.refs(), 1);
        amap_unref(&amap, 4, 4, false);
        assert_eq!((k2.refs(), k5.refs()), (0, 0));
    }

    #[test]
    fn amap_copy_takes_over_or_copies() {
        let _g = setup();
        let amap = amap_alloc(4 * PAGE_SIZE, M_WAITOK, false).expect("an amap");

        // a sole reference is taken over
        assert!(matches!(
            amap_copy_from(&amap, 0, 4, M_WAITOK),
            CopyOutcome::TakeOver
        ));

        // a shared amap is copied: the anons gain a reference, the source loses one
        let anon = anon_for(&amap);
        let a = anon.clone();
        {
            let mut w = amap.lock().write();
            assert!(amap_add(&amap, &mut w, 0, PAGE_SIZE, anon, false).is_ok());
        }
        amap_ref(&amap, 0, 4, AMAP_REFALL); // the other map entry, as fork makes it
        let CopyOutcome::New(copy) = amap_copy_from(&amap, 0, 4, M_WAITOK) else {
            panic!("a copy");
        };
        assert!(!PoolArc::ptr_eq(&copy, &amap));
        assert_eq!(amap.refs(), 1);
        assert_eq!(copy.refs(), 1);
        assert_eq!(a.refs(), 2);
        assert_eq!(
            PoolArc::count(&a),
            3,
            "two slots and this test hold the anon"
        );
        assert!(
            SharedRwlock::same(copy.lock(), amap.lock()),
            "a copy with shared anons shares the lock"
        );
        {
            let w = copy.lock().read();
            assert_eq!(copy.state(&w).nused(), 1);
            assert!(amap_lookup(&copy, &w, 0, PAGE_SIZE).is_some_and(|x| PoolArc::ptr_eq(x, &a)));
        }

        amap_unref(&copy, 0, 4, true);
        drop(copy);
        assert_eq!(a.refs(), 1);
        assert_eq!(PoolArc::count(&a), 2);
        amap_unref(&amap, 0, 4, true);
        assert_eq!(a.refs(), 0);
    }

    #[test]
    fn a_copy_that_shares_no_anon_has_a_lock_of_its_own() {
        let _g = setup();
        let amap = amap_alloc(8 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        let anon = anon_for(&amap);
        {
            let mut w = amap.lock().write();
            assert!(amap_add(&amap, &mut w, 0, 6 * PAGE_SIZE, anon, false).is_ok());
        }
        amap_ref(&amap, 0, 8, AMAP_REFALL);
        // Copy only pages 0..4, which hold no anon.
        let CopyOutcome::New(copy) = amap_copy_from(&amap, 0, 4, M_WAITOK) else {
            panic!("a copy");
        };
        assert!(!SharedRwlock::same(copy.lock(), amap.lock()));
        let w = copy.lock().write();
        assert_eq!(
            copy.state(&w).nused(),
            0,
            "its state opens with its own lock"
        );
        drop(w);
        assert_eq!(amap.lock().status(), 0, "the source is unlocked");
        amap_unref(&copy, 0, 4, true);
        amap_unref(&amap, 0, 8, true);
    }

    #[test]
    fn cow_now_copies_the_shared_anons() {
        let _g = setup();
        let amap = amap_alloc(2 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        let out = anons_out();
        let (shared, alone) = {
            let mut w = amap.lock().write();
            let shared = anon_with_page(&amap, &w);
            let alone = anon_with_page(&amap, &w);
            let (s, a) = (shared.clone(), alone.clone());
            assert!(amap_add(&amap, &mut w, 0, 0, shared, false).is_ok());
            assert!(amap_add(&amap, &mut w, 0, PAGE_SIZE, alone, false).is_ok());
            // another amap's slot holds `shared` too
            s.set_refs(2);
            (s, a)
        };
        assert_eq!(anons_out(), out + 2);

        amap_cow_now(&amap);

        let w = amap.lock().read();
        let now = amap_lookup(&amap, &w, 0, 0).expect("an anon").clone();
        assert!(
            !PoolArc::ptr_eq(&now, &shared),
            "the shared anon was replaced"
        );
        assert_eq!(now.refs(), 1);
        assert_eq!(shared.refs(), 1, "the old anon keeps its other reference");
        assert!(SharedRwlock::same(now.lock(), amap.lock()));
        let npg = the_page(&now);
        assert_eq!(npg.flags() & (PG_BUSY | PG_FAKE), 0);
        assert_eq!(npg.flags() & PG_CLEAN, 0, "the copy is dirty");
        assert!(amap_lookup(&amap, &w, 0, PAGE_SIZE).is_some_and(|x| PoolArc::ptr_eq(x, &alone)));
        assert_eq!(anons_out(), out + 3);
        drop(w);
        drop(now);

        // The other amap's reference, gone: the old anon is freed with it.
        {
            let w = amap.lock().write();
            shared.set_refs(0);
            uvm_anfree(&shared);
            drop(w);
        }
        drop(shared);
        drop(alone);
        amap_unref(&amap, 0, 2, true);
        assert_eq!(anons_out(), out);
    }

    #[test]
    fn a_large_amap_without_its_buckets_is_not_made() {
        let _g = setup();
        let before = amaps_out();
        testutil::fail_pages(0, 1); // the bucket array's malloc
        let none = amap_alloc((UVM_AMAP_LARGE + 1) * PAGE_SIZE, M_NOWAIT, true);
        testutil::fail_pages(0, 0);
        assert!(none.is_none());
        assert_eq!(amaps_out(), before, "the amap went back to its pool");
        assert_eq!(testutil::malloc_out(M_UVMAMAP), 0);
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_witness_of_another_lock_opens_nothing() {
        let _g = setup();
        let amap = amap_alloc(4 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        let other = SharedRwlock::new("other").expect("a lock");
        let w = other.read();
        let _ = amap_lookup(&amap, &w, 0, 0);
    }

    #[test]
    #[should_panic(expected = "amap_wipeout: corrupt amap")]
    fn a_wipeout_of_an_anon_without_references_panics() {
        let _g = setup();
        let amap = amap_alloc(4 * PAGE_SIZE, M_WAITOK, false).expect("an amap");
        let anon = anon_for(&amap);
        let mut w = amap.lock().write();
        anon.set_refs(0);
        assert!(amap_add(&amap, &mut w, 0, 0, anon, false).is_ok());
        amap.set_refs(0);
        amap_wipeout(&amap, w);
    }

    #[test]
    fn swap_off_is_reported_and_nothing_fails() {
        let _g = setup();
        assert!(!amap_swap_off(0, 100));
        assert_eq!(
            testutil::printf_count("unported: amap_swap_off (uvm_swap, N3.5)\n"),
            1
        );
        assert!(!amap_swap_off(0, 100));
        assert_eq!(
            testutil::printf_count("unported: amap_swap_off (uvm_swap, N3.5)\n"),
            1
        );
    }

    #[test]
    #[ignore = "reads reference/openbsd-src (just test-ref)"]
    fn constants_match_the_c() {
        let defs = crate::reftest::defines("sys/uvm/uvm_amap.h");
        let _ = crate::reftest::assert_defines!(defs;
            AMAP_SHARED, AMAP_REFALL, AMAP_SWAPOFF, UVM_AMAP_LARGE, UVM_AMAP_CHUNK);
        let defs = crate::reftest::defines("sys/sys/malloc.h");
        let _ = crate::reftest::assert_defines!(defs; M_UVMAMAP);
    }

    #[test]
    fn a_chunk_made_unlocked_is_installed_once() {
        let _g = setup();
        let amap = amap_alloc(32 * PAGE_SIZE, M_WAITOK, true).expect("a normal amap");
        let (_, chunks) = amaps_out();

        // Made with nothing held, installed under the write witness.
        let spare = amap_chunk_alloc(&amap, 0, 17 * PAGE_SIZE, PR_WAITOK).expect("a chunk");
        assert_eq!(amaps_out().1, chunks + 1);
        let mut w = amap.lock().write();
        assert!(amap_chunk_find(amap.geom, &amap.state.get(&w).tables, 17).is_none());
        amap_populate_with(&amap, &mut w, 0, 17 * PAGE_SIZE, spare);
        let c = amap_chunk_find(amap.geom, &amap.state.get(&w).tables, 17).expect("installed");
        assert_eq!(c.baseslot(), 16);
        drop(w);

        // A second one for the same slot finds the chunk there and goes back to its pool.
        let again = amap_chunk_alloc(&amap, 0, 20 * PAGE_SIZE, PR_WAITOK).expect("a chunk");
        let mut w = amap.lock().write();
        amap_populate_with(&amap, &mut w, 0, 20 * PAGE_SIZE, again);
        drop(w);
        assert_eq!(amaps_out().1, chunks + 1);

        // A small amap's chunk is embedded: nothing to make.
        let small = amap_alloc(4 * PAGE_SIZE, M_WAITOK, true).expect("a small amap");
        assert!(amap_chunk_alloc(&small, 0, PAGE_SIZE, PR_WAITOK).is_none());
    }
}
/* </TESTS> */
