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
 * Copyright (c) 1998 Chuck Silvers, Charles D. Cranor and
 *                    Washington University.
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
 *
 * from: Id: uvm_aobj.h,v 1.1.2.4 1998/02/06 05:19:28 chs Exp
 * from: Id: uvm_aobj.c,v 1.1.2.5 1998/02/06 05:14:38 chs Exp
 */
/* </LICENSES> */

/* <CODE> */
//! The anonymous memory object pager: `<uvm/uvm_aobj.h>` and `uvm/uvm_aobj.c`.
//!
//! Upstream: sys/uvm/uvm_aobj.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_aobj.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_aobj.rs@3c62ede68802
//!
//! author: Chuck Silvers, started: Jan-1998, design mostly from Chuck Cranor.
//!
//! An anonymous UVM object (aobj) manages anonymous-memory. In addition to keeping the list
//! of resident pages, it may also keep a list of allocated swap blocks. Depending on the
//! size of the object, this list is either stored in an array (small objects) or in a hash
//! table (large objects).
//!
//! ## Deviations
//! - Swap does not exist yet (N3.5): `uao_get` reports `uvm_swap_get` and treats a slot as an
//!   I/O error (the `SWSLOT_BAD` marking and `uvm_swap_markbad` are reported with it);
//!   [`uao_dropswap`] and [`uao_dropswap_range`] report `uvm_swap_free`. Nothing assigns a
//!   slot, so none of these paths run.
//! - [`uao_swap_off`] reports itself and pages nothing in: the global list of aobjs it walks
//!   (`uao_list`, `uao_list_lock`) arrives with its caller, `uvm_swap`'s swapoff (N3.5;
//!   `swapctl` is `sys_nosys` at the pin). [`uao_pagein`] pages in one aobj's range.
//! - `uao_shrink`/`uao_grow` and their helpers are `#ifdef TMPFS`: feature `tmpfs`.
//! - `uao_shrink_hash`/`uao_grow_hash` move the swap-hash elements to the new table by
//!   rehashing each by its tag (`uao_swhash_move`). The C moves bucket `i` of the old table
//!   to bucket `i` of the new one (`uvm_aobj.c` 425-434, 582-588) on the premise that the
//!   elements' hash does not change, which does not hold: `hashinit` rounds the bucket count
//!   to a power of two (`kern_subr.c` 226-230) and a tag can equal `pages >> 4`, so a shrink
//!   (200 -> 65 pages) writes past the new table and a grow (65 -> 200) leaves an element in a
//!   bucket `UAO_SWHASH_HASH` no longer looks in (its swap slot lost). A slip of the C the
//!   port does not have; latent until swap exists (N3.5, with tmpfs).
//! - `UVM_PAGE_OWN` is not configured.
//!
//! ## Redesign
//! - LZ's `uao_create() -> &'static UvmObject` (a pool item written by hand, freed by
//!   `uao_free`'s `pool_put` while map entries may still name it) -> an
//!   `ObjRef::Aobj(PoolArc<Aobj>)` of `"aobjpl"`, the kernel object an `ObjRef::Kernel` of the
//!   static [`Aobj`] (`kernel_object_store`). `uo_refs` stays the C's count of references
//!   ([`uao_reference`], [`uao_detach`], atomic as in the C); the memory belongs to the handles,
//!   and goes back to the pool with the last of them after the last [`uao_detach`] destroyed
//!   the object (its pages and its swap bookkeeping).
//! - The pager functions take the [`Aobj`], not an object cast back from its first member.
//! - The swap bookkeeping (the `u_swap` union of `u_swslots`, a `mallocarray` of slots, and
//!   `u_swhash`, a `hashinit` table of `LIST_HEAD`s of `uao_swhash_elt` pool items) -> a
//!   [`Swap`] inside a `hal::rwlock::Guarded` keyed to the object's lock: `None` (`NOSWAP`, or
//!   nothing made), `Array` of a `hal::malloc::MArray<i32>`, `Hash` of an `MArray` of buckets,
//!   each the head of an owned chain of [`SwhashElt`] (`hal::pool::PoolBox` items of
//!   `"uaoeltpl"`, each owning the next: the C's bucket list). Reading a slot takes a witness
//!   of the object's lock, changing one the write witness; a witness of another lock is
//!   `panic(9)`. An element is 96 bytes, as the C's item; a bucket head is 16 bytes where the
//!   C's is 8 (the handle's pool beside the pointer), so a hash table's `M_UVMAOBJ` block is
//!   twice the C's (as `amap`'s buckets, N3.4b).
//! - `uao_get` is two functions: [`uao_get_locked`] (`PGO_LOCKED`: a witness of either mode,
//!   kept) and [`uao_get_sync`] (the write witness, consumed: the C returns unlocked); the
//!   object's `pgo_get` (`kernel::vm::object::ObjRef::get`, `get_sync`) dispatches to them.
//!   A wait on a busy page under a borrowed witness ([`uao_flush`]) sleeps with the lock
//!   released and takes it again (`rwsleep`), the C's `uvm_pagewait` and `rw_enter`.
//! - `u_pages` and `u_flags` are relaxed atomics (the C's `u_pages` changes only under the
//!   lock, in [`uao_grow`] and [`uao_shrink`]); the global list and `u_list` are not kept (see
//!   the deviations).
//! - [`uao_init`] returns whether the pools were made (pipe's precedent): `ENOMEM` before the
//!   pool bridge, `EBUSY` the second time.

use core::mem;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::intr::IPL_NONE;
#[cfg(feature = "tmpfs")]
use hal::malloc::M_CANFAIL;
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray};
use hal::pool::{BoxPool, PR_NOWAIT, PR_WAITOK, PoolArc, PoolBox, TypedPool};
use hal::rwlock::{Guarded, RawRwlock, Witness, WriteWitness};
use hal::sys::errno::Errno;
use hal::sys::mman::{PROT_NONE, PROT_READ, PROT_WRITE};
use hal::sys::types::Vsize;
use hal::vm::param::{round_page, trunc_page};
use hal::vm::uvmexp::UVMEXP;

use super::object::{ObjKind, ObjRef, Object, UVM_OBJ_KERN, pagewait_relock, uvm_obj_destroy};
use super::page::{
    Owner, PG_BUSY, PG_CLEAN, PG_FAKE, PG_WANTED, PQ_AOBJ, VmPage, uvm_pagealloc,
    uvm_pagedeactivate, uvm_pagefree, uvm_pagelookup, uvm_pagewait, uvm_pagezero,
};
use super::pager::{
    PGO_ALLPAGES, PGO_CLEANIT, PGO_DEACTIVATE, PGO_FREE, PGO_LOCKED, PageSlot, VM_PAGER_BAD,
    VM_PAGER_ERROR, VM_PAGER_OK, VM_PAGER_REFAULT, VM_PAGER_UNLOCK,
};
use super::pmemrange::uvm_wait;
use super::types::{VmProt, Voff};
use crate::assert::{kassert, kpanic};
use crate::prf::{printf, unported};

/// `M_UVMAOBJ` (`<sys/malloc.h>`): the malloc type of the swap slot arrays and hash tables.
pub const M_UVMAOBJ: i32 = 99;

// flags

/// `UAO_FLAG_KERNOBJ`: create kernel object (`uao_create`; can only be used one time, at
/// bootup).
pub const UAO_FLAG_KERNOBJ: i32 = 0x1;
/// `UAO_FLAG_KERNSWAP`: enable kernel swap (`uao_create`, once).
pub const UAO_FLAG_KERNSWAP: i32 = 0x2;
/// `UAO_FLAG_CANFAIL`: creation can fail.
pub const UAO_FLAG_CANFAIL: i32 = 0x4;
/// `UAO_FLAG_NOSWAP`: internal: aobj can't swap (kernel obj only!).
pub const UAO_FLAG_NOSWAP: i32 = 0x8;

/// `UAO_SWHASH_CLUSTER_SHIFT`: for hash tables, we break the address space of the aobj into
/// blocks of `UAO_SWHASH_CLUSTER_SIZE` pages, which shall be a power of two.
const UAO_SWHASH_CLUSTER_SHIFT: u32 = 4;
/// `UAO_SWHASH_CLUSTER_SIZE`.
pub const UAO_SWHASH_CLUSTER_SIZE: usize = 1 << UAO_SWHASH_CLUSTER_SHIFT;

/// `UAO_SWHASH_ELT_TAG(idx)`: the "tag" for this page index.
const fn uao_swhash_elt_tag(idx: Voff) -> Voff {
    idx >> UAO_SWHASH_CLUSTER_SHIFT
}

/// `UAO_SWHASH_ELT_PAGESLOT_IDX(idx)`.
const fn uao_swhash_elt_pageslot_idx(idx: Voff) -> usize {
    (idx as usize) & (UAO_SWHASH_CLUSTER_SIZE - 1)
}

/// `UAO_SWHASH_THRESHOLD`: the threshold which determines whether we will use an array or a
/// hash table to store the list of allocated swap blocks.
pub const UAO_SWHASH_THRESHOLD: i32 = (UAO_SWHASH_CLUSTER_SIZE * 4) as i32;

/// `UAO_SWHASH_MAXBUCKETS`: the number of buckets in a hash, with an upper bound.
const UAO_SWHASH_MAXBUCKETS: i32 = 256;

/// `UAO_SWHASH_BUCKETS(pages)`.
const fn uao_swhash_buckets(pages: i32) -> i32 {
    let n = pages >> UAO_SWHASH_CLUSTER_SHIFT;
    if n < UAO_SWHASH_MAXBUCKETS {
        n
    } else {
        UAO_SWHASH_MAXBUCKETS
    }
}

/// `struct uao_swhash_elt`: when a hash table is being used, this structure defines the
/// format of an entry in the bucket list.
pub struct SwhashElt {
    /// `list`: the next element of the bucket.
    next: Option<PoolBox<SwhashElt>>,
    /// `tag`: our 'tag'.
    tag: Voff,
    /// `count`: our number of active slots.
    count: i32,
    /// `slots`: the slots.
    slots: [i32; UAO_SWHASH_CLUSTER_SIZE],
}

impl SwhashElt {
    /// An element for `tag` with no slots (what `pool_get` with `PR_ZERO` hands out, tagged).
    const fn new(tag: Voff) -> Self {
        Self {
            next: None,
            tag,
            count: 0,
            slots: [0; UAO_SWHASH_CLUSTER_SIZE],
        }
    }

    /// `UAO_SWHASH_ELT_PAGEIDX_BASE(elt)`: given an ELT, return its pageidx base.
    fn pageidx_base(&self) -> Voff {
        self.tag << UAO_SWHASH_CLUSTER_SHIFT
    }
}

/// A bucket of the swap hash table: the head of its chain.
type Bucket = Option<PoolBox<SwhashElt>>;

/// The swap bookkeeping of an aobj (the `u_swap` union): only one of the two is used in any
/// given aobj, as `UAO_USES_SWHASH` says.
pub enum Swap {
    /// No slots kept (`UAO_FLAG_NOSWAP`, or before `uao_create` made them).
    None,
    /// `u_swslots`: offset -> swapslot mappings of a small aobj.
    Array(MArray<i32>),
    /// `u_swhash`: the hash table of a large aobj (`u_swhashmask` is its length minus one).
    Hash(MArray<Bucket>),
}

/// `struct uvm_aobj`: the actual anon-backed uvm_object.
///
/// Locks: the object's lock guards `swap` (a cell keyed to it) and the writes of `pages`.
pub struct Aobj {
    /// `u_obj`: has: pgops, memt, #pages, #refs.
    obj: Object,
    /// `u_pages`: number of pages in entire object.
    pages: AtomicI32,
    /// `u_flags`: the flags (see above).
    flags: AtomicI32,
    /// `u_swap`: the swap slots.
    swap: Guarded<Swap>,
}

impl Aobj {
    /// A fresh aobj of `pages` pages: one reference, a lock of its own, no slots yet.
    fn new(pages: i32) -> Self {
        let obj = Object::new(ObjKind::Aobj, 1);
        let swap = Guarded::new(obj.lock(), Swap::None);
        Self {
            obj,
            pages: AtomicI32::new(pages),
            flags: AtomicI32::new(0),
            swap,
        }
    }

    /// The kernel object before `uao_create`: under the bootstrap lock, which lives for ever.
    const fn kernel() -> Self {
        Self {
            obj: Object::with_static_lock(ObjKind::Aobj, UVM_OBJ_KERN, &BOOTSTRAP_LOCK),
            pages: AtomicI32::new(0),
            flags: AtomicI32::new(0),
            swap: Guarded::new_static(&BOOTSTRAP_LOCK, Swap::None),
        }
    }

    /// `u_obj`.
    pub fn object(&self) -> &Object {
        &self.obj
    }

    /// `u_pages`: number of pages in entire object.
    pub fn pages(&self) -> i32 {
        self.pages.load(Ordering::Relaxed)
    }

    /// `u_flags`.
    pub fn flags(&self) -> i32 {
        self.flags.load(Ordering::Relaxed)
    }

    /// `UAO_USES_SWHASH(aobj)`.
    pub fn uses_swhash(&self) -> bool {
        self.pages() > UAO_SWHASH_THRESHOLD
    }

    /// `u_pages = pages`, under the object's lock held for writing (`uao_grow`, `uao_shrink`).
    #[cfg(feature = "tmpfs")]
    fn set_pages(&self, pages: i32) {
        kassert!(self.obj.write_held());
        self.pages.store(pages, Ordering::Relaxed);
    }
}

/// `uao_swhash_elt_pool`: pool of uao_swhash_elt structures.
static SWHASH_ELT_POOL: BoxPool<SwhashElt> = BoxPool::new();
/// `uvm_aobj_pool`.
static AOBJ_POOL: TypedPool<Aobj> = TypedPool::new();

/// `bootstrap_kernel_object_lock`: the kernel object's lock, which lives for ever.
static BOOTSTRAP_LOCK: RawRwlock = RawRwlock::new("kobjlk");
/// `kernel_object_store`: the kernel object (`uao_create`).
static KERNEL_OBJECT: Aobj = Aobj::kernel();
/// `kobj_alloced`: how far the kernel object's creation got.
static KOBJ_ALLOCED: AtomicI32 = AtomicI32::new(0);

// hash table/array related functions

/// The bucket of `table` that page `idx` hashes to (`UAO_SWHASH_HASH(aobj, idx)`).
fn bucket_of(table: &[Bucket], idx: Voff) -> usize {
    ((idx >> UAO_SWHASH_CLUSTER_SHIFT) as usize) & (table.len() - 1)
}

/// The element of the chain `head` for `tag`.
fn chain_find(head: &Bucket, tag: Voff) -> Option<&SwhashElt> {
    let mut cur = head.as_deref();
    while let Some(e) = cur {
        if e.tag == tag {
            return Some(e);
        }
        cur = e.next.as_deref();
    }
    None
}

/// The element of the chain `head` for `tag`, mutably.
fn chain_find_mut(head: &mut Bucket, tag: Voff) -> Option<&mut SwhashElt> {
    let mut cur = head.as_deref_mut();
    while let Some(e) = cur {
        if e.tag == tag {
            return Some(e);
        }
        cur = e.next.as_deref_mut();
    }
    None
}

/// Keeps the elements of the chain `head` for which `keep` answers true, in order; the others
/// go back to the pool (`LIST_REMOVE`, `pool_put`).
fn chain_retain(head: &mut Bucket, mut keep: impl FnMut(&mut SwhashElt) -> bool) {
    let mut rest = head.take();
    let mut tail = head;
    while let Some(mut e) = rest {
        rest = e.next.take();
        if keep(&mut e) {
            tail = &mut tail.insert(e).next;
        }
    }
}

/// `uao_find_swhash_elt`: find (or create, waiting for the pool when `wait`) a hash table
/// entry for a page offset.
fn uao_find_swhash_elt(
    table: &mut [Bucket],
    pageidx: Voff,
    create: bool,
    wait: bool,
) -> Option<&mut SwhashElt> {
    let b = bucket_of(table, pageidx); // first hash to get bucket
    let page_tag = uao_swhash_elt_tag(pageidx); // tag to search for
    let head = &mut table[b];

    // now search the bucket for the requested tag
    if chain_find(head, page_tag).is_some() {
        return chain_find_mut(head, page_tag);
    }

    if !create {
        return None;
    }

    // allocate a new entry for the bucket and init/insert it in
    let waitf = if wait { PR_WAITOK } else { PR_NOWAIT };
    let mut elt = SWHASH_ELT_POOL.get(waitf, || SwhashElt::new(page_tag))?;
    elt.next = head.take();
    Some(&mut **head.insert(elt))
}

/// `hashinit(elements, M_UVMAOBJ, flags, &mask)`: a table of the next power of two of
/// `elements` empty buckets.
fn hashinit(elements: i32, flags: i32) -> Option<MArray<Bucket>> {
    if elements <= 0 {
        kpanic(format_args!("hashinit: bad cnt"));
    }
    let mut hashsize = 1usize;
    while hashsize < elements as usize {
        hashsize <<= 1;
    }
    MArray::new(M_UVMAOBJ, flags, hashsize, |_| None)
}

/// A zeroed array of `n` swap slots (`mallocarray(n, sizeof(int), M_UVMAOBJ, flags | M_ZERO)`).
fn slot_array(n: i32, flags: i32) -> Option<MArray<i32>> {
    MArray::new(M_UVMAOBJ, flags, n.max(0) as usize, |_| 0)
}

/// `uao_find_swslot`: find the swap slot number for an aobj/pageidx; `w` is a witness of the
/// object's lock.
pub fn uao_find_swslot(aobj: &Aobj, w: &impl Witness, pageidx: i32) -> i32 {
    // if noswap flag is set, then we never return a slot
    if aobj.flags() & UAO_FLAG_NOSWAP != 0 {
        return 0;
    }

    let idx = Voff::from(pageidx);
    match aobj.swap.get(w) {
        // if hashing, look in hash table.
        Swap::Hash(table) => chain_find(&table[bucket_of(table, idx)], uao_swhash_elt_tag(idx))
            .map_or(0, |elt| elt.slots[uao_swhash_elt_pageslot_idx(idx)]),
        // otherwise, look in the array
        Swap::Array(slots) => slots[pageidx as usize],
        Swap::None => 0,
    }
}

/// `uao_set_swslot`: set the swap slot for a page in an aobj.
///
/// Setting a slot to zero frees the slot. Object must be locked by caller (`w`). We return
/// the old slot number, or -1 if we failed to allocate memory to record the new slot number.
pub fn uao_set_swslot(aobj: &Aobj, w: &mut WriteWitness<'_>, pageidx: i32, slot: i32) -> i32 {
    // if noswap flag is set, then we can't set a slot
    if aobj.flags() & UAO_FLAG_NOSWAP != 0 {
        if slot == 0 {
            return 0; // a clear is ok
        }

        // but a set is not
        printf(format_args!(
            "uao_set_swslot: uobj = {:p}\n",
            core::ptr::from_ref(&aobj.obj)
        ));
        kpanic(format_args!(
            "uao_set_swslot: attempt to set a slot on a NOSWAP object"
        ));
    }

    let idx = Voff::from(pageidx);
    match aobj.swap.get_mut(w) {
        // are we using a hash table?  if so, add it in the hash.
        Swap::Hash(table) => {
            // Avoid allocating an entry just to free it again if the page had not swap slot
            // in the first place, and we are freeing.
            let Some(elt) = uao_find_swhash_elt(table, idx, slot != 0, false) else {
                return if slot != 0 { -1 } else { 0 };
            };

            let oldslot = mem::replace(&mut elt.slots[uao_swhash_elt_pageslot_idx(idx)], slot);

            // now adjust the elt's reference counter and free it if we've dropped it to zero.
            if slot != 0 {
                if oldslot == 0 {
                    elt.count += 1;
                }
            } else {
                if oldslot != 0 {
                    elt.count -= 1;
                }

                if elt.count == 0 {
                    let b = bucket_of(table, idx);
                    chain_retain(&mut table[b], |e| e.count != 0);
                }
            }
            oldslot
        }
        // we are using an array
        Swap::Array(slots) => mem::replace(&mut slots[pageidx as usize], slot),
        Swap::None => kpanic(format_args!("uao_set_swslot: no swap slots")),
    }
}

// end of hash/array functions

/// `uao_free`: free all resources held by an aobj, and then free the aobj.
///
/// The aobj should be dead; its lock is held (`w`) and released here. The memory goes back to
/// the pool with the last handle.
fn uao_free(aobj: &Aobj, mut w: WriteWitness<'_>) {
    kassert!(aobj.obj.is_aobj());
    kassert!(aobj.obj.write_held());
    uao_dropswap_range(aobj, &mut w, 0, 0);

    // free the hash table or the array itself.
    *aobj.swap.get_mut(&mut w) = Swap::None;
    drop(w);

    // finally free the aobj itself
    uvm_obj_destroy(&aobj.obj);
}

// pager functions

/// `uao_shrink_flush`: free the pages and drop the swap slots of the pages
/// `[startpg, endpg)` of an aobj that is about to shrink.
///
/// Shrinking an aobj to a given number of pages is always the same procedure: assess the
/// necessity of data structure conversion (hash to array), secure resources, flush pages and
/// drop swap slots.
#[cfg(feature = "tmpfs")]
pub fn uao_shrink_flush(aobj: &Aobj, w: &mut WriteWitness<'_>, startpg: i32, endpg: i32) {
    kassert!(startpg < endpg);
    kassert!(aobj.obj.refs() == 1);
    let _ = uao_flush(
        aobj,
        w,
        Voff::from(startpg) << PAGE_SHIFT,
        Voff::from(endpg) << PAGE_SHIFT,
        PGO_FREE,
    );
    uao_dropswap_range(aobj, w, Voff::from(startpg), Voff::from(endpg));
}

/// Moves every swap-hash element of `old` into `new`, each to the bucket its tag hashes to
/// in `new` (where the C's bucket-to-bucket move goes wrong: the module's deviations).
#[cfg(feature = "tmpfs")]
fn uao_swhash_move(old: &mut [Bucket], new: &mut [Bucket]) {
    let newmask = new.len() - 1;
    for bucket in old {
        let mut rest = bucket.take();
        while let Some(mut elt) = rest {
            rest = elt.next.take();
            let b = (elt.tag as usize) & newmask;
            elt.next = new[b].take();
            new[b] = Some(elt);
        }
    }
}

/// The hash table of a large aobj, for its resizers.
#[cfg(feature = "tmpfs")]
fn swhash_of<'s>(swap: &'s mut Swap, who: &str) -> &'s mut MArray<Bucket> {
    match swap {
        Swap::Hash(t) => t,
        Swap::Array(_) | Swap::None => kpanic(format_args!("{who}: no swap hash")),
    }
}

/// The slot array of a small aobj, for its resizers.
#[cfg(feature = "tmpfs")]
fn swslots_of<'s>(swap: &'s mut Swap, who: &str) -> &'s mut MArray<i32> {
    match swap {
        Swap::Array(s) => s,
        Swap::Hash(_) | Swap::None => kpanic(format_args!("{who}: no swap slots")),
    }
}

/// `uao_shrink_hash`: shrink an aobj that keeps a hash table and still needs one.
#[cfg(feature = "tmpfs")]
pub fn uao_shrink_hash(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    kassert!(aobj.uses_swhash());

    // If the size of the hash table doesn't change, all we need to do is to adjust the
    // page count.
    if uao_swhash_buckets(aobj.pages()) == uao_swhash_buckets(pages) {
        uao_shrink_flush(aobj, w, pages, aobj.pages());
        aobj.set_pages(pages);
        return Ok(());
    }

    let mut new_swhash =
        hashinit(uao_swhash_buckets(pages), M_WAITOK | M_CANFAIL).ok_or(Errno::ENOMEM)?;

    uao_shrink_flush(aobj, w, pages, aobj.pages());

    // Even though the hash table size is changing, the hash of the buckets we are
    // interested in copying should not change.
    let swap = aobj.swap.get_mut(w);
    uao_swhash_move(swhash_of(swap, "uao_shrink_hash"), &mut new_swhash);
    *swap = Swap::Hash(new_swhash);
    aobj.set_pages(pages);

    Ok(())
}

/// `uao_shrink_convert`: shrink an aobj that keeps a hash table below the threshold, so
/// that its swap slots move to an array.
#[cfg(feature = "tmpfs")]
pub fn uao_shrink_convert(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    let mut new_swslots = slot_array(pages, M_WAITOK | M_CANFAIL).ok_or(Errno::ENOMEM)?;

    uao_shrink_flush(aobj, w, pages, aobj.pages());

    // Convert swap slots from hash to array.
    let swap = aobj.swap.get_mut(w);
    let table = swhash_of(swap, "uao_shrink_convert");
    for (i, out) in new_swslots.iter_mut().enumerate() {
        let idx = i as Voff;
        let Some(elt) = uao_find_swhash_elt(table, idx, false, false) else {
            continue;
        };
        *out = elt.slots[uao_swhash_elt_pageslot_idx(idx)];
        if *out != 0 {
            elt.count -= 1;
        }
        if elt.count == 0 {
            let b = bucket_of(table, idx);
            chain_retain(&mut table[b], |e| e.count != 0);
        }
    }

    // The flush and the loop above emptied the table (every element of `[0, pages)` lost
    // its last slot), and the object now uses the array.
    *swap = Swap::Array(new_swslots);
    aobj.set_pages(pages);

    Ok(())
}

/// `uao_shrink_array`: shrink an aobj that keeps its swap slots in an array.
#[cfg(feature = "tmpfs")]
pub fn uao_shrink_array(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    let mut new_swslots = slot_array(pages, M_WAITOK | M_CANFAIL).ok_or(Errno::ENOMEM)?;

    uao_shrink_flush(aobj, w, pages, aobj.pages());

    let swap = aobj.swap.get_mut(w);
    let old = swslots_of(swap, "uao_shrink_array");
    let n = new_swslots.len();
    new_swslots.copy_from_slice(&old[..n]);

    *swap = Swap::Array(new_swslots);
    aobj.set_pages(pages);

    Ok(())
}

/// `uao_shrink`: shrink an aobj to `pages` pages, freeing the pages and swap slots past
/// the new end. The object is locked by the caller (`w`).
#[cfg(feature = "tmpfs")]
pub fn uao_shrink(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    kassert!(pages < aobj.pages());

    // Distinguish between three possible cases:
    // 1. aobj uses hash and must be converted to array.
    // 2. aobj uses array and array size needs to be adjusted.
    // 3. aobj uses hash and hash size needs to be adjusted.
    if pages > UAO_SWHASH_THRESHOLD {
        uao_shrink_hash(aobj, w, pages) // case 3
    } else if aobj.pages() > UAO_SWHASH_THRESHOLD {
        uao_shrink_convert(aobj, w, pages) // case 1
    } else {
        uao_shrink_array(aobj, w, pages) // case 2
    }
}

/// `uao_grow_array`: grow an aobj whose swap slots stay in an array.
///
/// Growing an aobj only adjusts the swap slots; the pages themselves come later through
/// `uvm_fault()`. It is thus mandatory that the caller of these functions does not allow
/// faults to happen in case of growth error.
#[cfg(feature = "tmpfs")]
pub fn uao_grow_array(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    kassert!(aobj.pages() <= UAO_SWHASH_THRESHOLD);

    let mut new_swslots = slot_array(pages, M_WAITOK | M_CANFAIL).ok_or(Errno::ENOMEM)?;

    let swap = aobj.swap.get_mut(w);
    let old = swslots_of(swap, "uao_grow_array");
    new_swslots[..old.len()].copy_from_slice(&old[..]);

    *swap = Swap::Array(new_swslots);
    aobj.set_pages(pages);

    Ok(())
}

/// `uao_grow_hash`: grow an aobj that keeps a hash table.
#[cfg(feature = "tmpfs")]
pub fn uao_grow_hash(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    kassert!(pages > UAO_SWHASH_THRESHOLD);

    // If the size of the hash table doesn't change, all we need to do is to adjust the
    // page count.
    if uao_swhash_buckets(aobj.pages()) == uao_swhash_buckets(pages) {
        aobj.set_pages(pages);
        return Ok(());
    }

    kassert!(uao_swhash_buckets(aobj.pages()) < uao_swhash_buckets(pages));

    let mut new_swhash =
        hashinit(uao_swhash_buckets(pages), M_WAITOK | M_CANFAIL).ok_or(Errno::ENOMEM)?;

    let swap = aobj.swap.get_mut(w);
    uao_swhash_move(swhash_of(swap, "uao_grow_hash"), &mut new_swhash);
    *swap = Swap::Hash(new_swhash);
    aobj.set_pages(pages);

    Ok(())
}

/// `uao_grow_convert`: grow an aobj past the threshold, so that its swap slots move from
/// an array to a hash table.
#[cfg(feature = "tmpfs")]
pub fn uao_grow_convert(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    let mut new_swhash =
        hashinit(uao_swhash_buckets(pages), M_WAITOK | M_CANFAIL).ok_or(Errno::ENOMEM)?;

    let swap = aobj.swap.get_mut(w);
    let old = swslots_of(swap, "uao_grow_convert");
    for (i, &slot) in old.iter().enumerate() {
        if slot != 0 {
            let idx = i as Voff;
            let Some(elt) = uao_find_swhash_elt(&mut new_swhash, idx, true, true) else {
                kpanic(format_args!("uao_grow_convert: no swap hash element"));
            };
            elt.count += 1;
            elt.slots[uao_swhash_elt_pageslot_idx(idx)] = slot;
        }
    }

    *swap = Swap::Hash(new_swhash);
    aobj.set_pages(pages);

    Ok(())
}

/// `uao_grow`: grow an aobj to `pages` pages. The object is locked by the caller (`w`).
#[cfg(feature = "tmpfs")]
pub fn uao_grow(aobj: &Aobj, w: &mut WriteWitness<'_>, pages: i32) -> Result<(), Errno> {
    kassert!(pages > aobj.pages());

    // Distinguish between three possible cases:
    // 1. aobj uses hash and hash size needs to be adjusted.
    // 2. aobj uses array and array size needs to be adjusted.
    // 3. aobj uses array and must be converted to hash.
    if pages <= UAO_SWHASH_THRESHOLD {
        uao_grow_array(aobj, w, pages) // case 2
    } else if aobj.pages() > UAO_SWHASH_THRESHOLD {
        uao_grow_hash(aobj, w, pages) // case 1
    } else {
        uao_grow_convert(aobj, w, pages)
    }
}

/// `uao_create`: create an aobj of the given size and return its uvm_object.
///
/// For normal use, flags are zero or `UAO_FLAG_CANFAIL`. For the kernel object, the flags
/// are: `UAO_FLAG_KERNOBJ`, allocate the kernel object (can only happen once);
/// `UAO_FLAG_KERNSWAP`, enable swapping of kernel object ("           ").
pub fn uao_create(size: Vsize, flags: i32) -> Option<ObjRef> {
    let pages = (round_page(size.as_usize()) >> PAGE_SHIFT) as i32;

    // Allocate a new aobj, unless kernel object is requested.
    let mut handle = if flags & UAO_FLAG_KERNOBJ != 0 {
        kassert!(KOBJ_ALLOCED.load(Ordering::Relaxed) == 0);
        KERNEL_OBJECT.pages.store(pages, Ordering::Relaxed);
        KERNEL_OBJECT
            .flags
            .store(UAO_FLAG_NOSWAP, Ordering::Relaxed);
        KOBJ_ALLOCED.store(UAO_FLAG_KERNOBJ, Ordering::Relaxed);
        None
    } else if flags & UAO_FLAG_KERNSWAP != 0 {
        kassert!(KOBJ_ALLOCED.load(Ordering::Relaxed) == UAO_FLAG_KERNOBJ);
        KOBJ_ALLOCED.store(UAO_FLAG_KERNSWAP, Ordering::Relaxed);
        None
    } else {
        match AOBJ_POOL.get_shared(PR_WAITOK, || Aobj::new(pages)) {
            Some(h) => Some(h),
            None => kpanic(format_args!("uao_create: pool_get failed")),
        }
    };

    // allocate hash/array if necessary
    if flags == 0 || flags & (UAO_FLAG_KERNSWAP | UAO_FLAG_CANFAIL) != 0 {
        let mflags = if flags != 0 { M_NOWAIT } else { M_WAITOK };
        let aobj: &Aobj = handle.as_deref().unwrap_or(&KERNEL_OBJECT);

        // allocate hash table or array depending on object size
        let swap = if aobj.uses_swhash() {
            hashinit(uao_swhash_buckets(pages), mflags).map(Swap::Hash)
        } else {
            slot_array(pages, mflags).map(Swap::Array)
        };
        let Some(swap) = swap else {
            if flags & UAO_FLAG_CANFAIL != 0 {
                // The item goes back to the pool with the handle.
                return None;
            }
            if aobj.uses_swhash() {
                kpanic(format_args!("uao_create: hashinit swhash failed"));
            }
            kpanic(format_args!("uao_create: malloc swslots failed"));
        };

        if let Some(h) = handle.as_mut() {
            // A fresh aobj, still its maker's alone.
            match PoolArc::get_mut(h) {
                Some(a) => *a.swap.get_mut_owned() = swap,
                None => kpanic(format_args!("uao_create: a shared new aobj")),
            }
        } else {
            // The kernel object, under its lock.
            let mut w = KERNEL_OBJECT.obj.lock().write();
            *KERNEL_OBJECT.swap.get_mut(&mut w) = swap;
        }

        if flags & UAO_FLAG_KERNSWAP != 0 {
            // clear noswap
            KERNEL_OBJECT
                .flags
                .fetch_and(!UAO_FLAG_NOSWAP, Ordering::Relaxed);
            return Some(ObjRef::Kernel(&KERNEL_OBJECT));
            // done!
        }
    }

    // The object was made by `Aobj::new` (`uvm_obj_init` with a lock of its own) or is the
    // kernel object under its bootstrap lock (`uvm_obj_setlock`). No global list (see the
    // module's deviations).
    Some(match handle {
        Some(h) => ObjRef::Aobj(h),
        None => ObjRef::Kernel(&KERNEL_OBJECT),
    })
}

/// `uvm.kernel_object`: the kernel object, once `uao_create(.., UAO_FLAG_KERNOBJ)` made it.
pub fn kernel_object() -> Option<ObjRef> {
    (KOBJ_ALLOCED.load(Ordering::Relaxed) != 0).then_some(ObjRef::Kernel(&KERNEL_OBJECT))
}

/// `uao_init`: set up aobj pager subsystem: the pools `"uaoeltpl"` and `"aobjpl"`, at
/// `IPL_NONE`, `PR_WAITOK`. `ENOMEM` when a pool cannot be made, `EBUSY` when it was.
pub fn uao_init() -> Result<(), Errno> {
    // NOTE: Pages for this pool must not come from a pageable kernel map!
    SWHASH_ELT_POOL.init(IPL_NONE, PR_WAITOK, "uaoeltpl")?;
    AOBJ_POOL.init(IPL_NONE, PR_WAITOK, "aobjpl")
}

/// `uao_reference`: hold a reference to an anonymous UVM object.
pub fn uao_reference(aobj: &Aobj) {
    // Kernel object is persistent.
    if aobj.obj.is_kern_object() {
        return;
    }

    aobj.obj.refs_inc();
}

/// `uao_detach`: drop a reference to an anonymous UVM object.
pub fn uao_detach(aobj: &Aobj) {
    let uobj = &aobj.obj;

    // Detaching from kernel_object is a NOP.
    if uobj.is_kern_object() {
        return;
    }

    // Drop the reference.  If it was the last one, destroy the object.
    if uobj.refs_dec_nv() > 0 {
        return;
    }

    // Free all the pages left in the aobj.  For each page, when the page is no longer busy
    // (and thus after any disk I/O that it is involved in is complete), release any swap
    // resources and free the page itself.
    let lock = uobj.lock();
    let mut w = lock.write();
    while let Some(pg) = uobj.pages().root() {
        hal::pmap::page_protect(&pg.mdpage, PROT_NONE);
        if pg.flags() & PG_BUSY != 0 {
            uvm_pagewait(pg, Owner::Object(uobj), w, "uao_det");
            w = lock.write();
            continue;
        }
        let _ = uao_dropswap(aobj, &mut w, (pg.offset() >> PAGE_SHIFT) as i32);
        uvm_pagefree(pg, Owner::Object(uobj));
    }

    // Finally, free the anonymous UVM object itself.
    uao_free(aobj, w);
}

/// `uao_flush`: flush pages out of a uvm object.
///
/// If `PGO_CLEANIT` is not set, then we will not block. If `PGO_ALLPAGE` is set, then all
/// pages in the object are valid targets for flushing. NOTE: we are allowed to lock the page
/// queues, so the caller must not be holding the lock on them (e.g. pagedaemon had better
/// not call us with the queues locked). We return `true` unless we encountered some sort of
/// I/O error (XXXJRT currently never happens, as we never directly initiate I/O). The object
/// is locked for writing (`w`).
pub fn uao_flush(
    aobj: &Aobj,
    w: &mut WriteWitness<'_>,
    start: Voff,
    stop: Voff,
    flags: i32,
) -> bool {
    let uobj = &aobj.obj;
    let end = Voff::from(aobj.pages()) << PAGE_SHIFT;

    kassert!(uobj.write_held());

    let (start, mut stop) = if flags & PGO_ALLPAGES != 0 {
        (0, end)
    } else {
        (
            trunc_page(start as usize) as Voff,
            round_page(stop as usize) as Voff,
        )
    };
    if flags & PGO_ALLPAGES == 0 && stop > end {
        printf(format_args!(
            "uao_flush: strange, got an out of range flush (fixed)\n"
        ));
        stop = end;
    }

    // Don't need to do any work here if we're not freeing or deactivating pages.
    if flags & (PGO_DEACTIVATE | PGO_FREE) == 0 {
        return true;
    }

    let owner = Owner::Object(uobj);
    let mut curoff = start;
    while curoff < stop {
        let Some(pg) = uvm_pagelookup(uobj, curoff) else {
            curoff += PAGE_SIZE as Voff;
            continue;
        };

        // Make sure page is unbusy, else wait for it.
        if pg.flags() & PG_BUSY != 0 {
            pagewait_relock(pg, uobj, w, "uaoflsh");
            continue; // the same offset again
        }
        curoff += PAGE_SIZE as Voff;

        let what = flags & (PGO_CLEANIT | PGO_FREE | PGO_DEACTIVATE);
        // XXX In the first 3 cases, we always just deactivate the page. We may want to
        // handle the different cases more specifically in the future.
        let deactivate = what == PGO_CLEANIT | PGO_FREE
            || what == PGO_CLEANIT | PGO_DEACTIVATE
            || what == PGO_DEACTIVATE
            // If there are multiple references to the object, just deactivate the page.
            || (what == PGO_FREE && uobj.refs() > 1);
        if deactivate {
            // deactivate_it:
            uvm_pagedeactivate(pg, owner);
            continue;
        }
        if what != PGO_FREE {
            kpanic(format_args!("uao_flush: weird flags"));
        }

        // XXX skip the page if it's wired
        if pg.wire_count() != 0 {
            continue;
        }

        // free the swap slot and the page.
        hal::pmap::page_protect(&pg.mdpage, PROT_NONE);

        // freeing swapslot here is not strictly necessary. however, leaving it here doesn't
        // save much because we need to update swap accounting anyway.
        let _ = uao_dropswap(aobj, w, (pg.offset() >> PAGE_SHIFT) as i32);
        uvm_pagefree(pg, owner);
    }

    true
}

/// `uao_get_validate`: `PGO_ALLPAGES` makes every requested page mandatory; otherwise only
/// centeridx must remain within the object, because adjacent pages are fault clustering
/// candidates rather than required fetches. Validate this distinction before allocation
/// because pageidx directly indexes swap metadata; an out of range value can therefore
/// access storage beyond the object's extent.
fn uao_get_validate(
    aobj: &Aobj,
    firstpage: Voff,
    maxpages: i32,
    centeridx: i32,
    flags: i32,
) -> bool {
    let u_pages = Voff::from(aobj.pages());
    if maxpages <= 0 || firstpage < 0 || firstpage >= u_pages {
        return false;
    }

    if flags & PGO_ALLPAGES != 0 {
        return Voff::from(maxpages) <= u_pages - firstpage;
    }

    centeridx >= 0 && centeridx < maxpages && Voff::from(centeridx) < u_pages - firstpage
}

/// `uao_get` with `PGO_LOCKED`: the resident, unbusy pages (case 1 below) into `pps`, the
/// object's lock held in either mode (`w`) and kept. `VM_PAGER_UNLOCK` when the centre page
/// (or, with `PGO_ALLPAGES`, any page) is not there: the caller unlocks and calls
/// [`uao_get_sync`].
///
/// We have three cases: 1: page is resident -> just return the page. 2: page is zero-fill ->
/// allocate a new page and zero it. 3: page is swapped out -> fetch the page from swap.
/// NOTE: offset is the offset of pps\[0\], _NOT_ pps\[centeridx\]. NOTE: caller must check for
/// released pages!!
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn uao_get_locked(
    aobj: &Aobj,
    w: &impl Witness,
    offset: Voff,
    pps: &mut [PageSlot],
    npagesp: &mut i32,
    centeridx: i32,
    _advice: i32,
    flags: i32,
) -> i32 {
    let uobj = &aobj.obj;
    uobj.check_witness(w.lock_id(), "uao_get");
    kassert!(uobj.is_aobj());
    kassert!(flags & PGO_LOCKED != 0);

    // get number of pages
    let maxpages = *npagesp;
    let firstpage = offset >> PAGE_SHIFT;
    if !uao_get_validate(aobj, firstpage, maxpages, centeridx, flags) {
        *npagesp = 0;
        return VM_PAGER_BAD;
    }
    kassert!(pps.len() >= maxpages as usize);

    // step 1a: get pages that are already resident.   only do this if the data structures
    // are locked (i.e. the first time through).
    let mut done = true; // be optimistic
    let mut gotpages = 0; // # of pages we got so far

    for (lcv, slot) in pps.iter_mut().enumerate().take(maxpages as usize) {
        let current_offset = offset + ((lcv as Voff) << PAGE_SHIFT);
        // do we care about this page?  if not, skip it
        if slot.is_dontcare() {
            continue;
        }

        // lookup page; to be useful must get a non-busy page
        let Some(ptmp) = uvm_pagelookup(uobj, current_offset).filter(|p| p.flags() & PG_BUSY == 0)
        else {
            if lcv as i32 == centeridx {
                // need to do a wait or I/O!
                done = false;
            }
            if flags & PGO_ALLPAGES != 0 {
                done = false;
                break;
            }
            continue;
        };

        // useful page: plug it in our result array
        *slot = PageSlot::Page(ptmp);
        gotpages += 1;
    }

    // step 1b: now we've either done everything needed or we to unlock and do some waiting
    // or I/O.
    *npagesp = gotpages;
    if done { VM_PAGER_OK } else { VM_PAGER_UNLOCK }
}

/// `uao_get` without `PGO_LOCKED`: the pages the caller must have (the centre one, or with
/// `PGO_ALLPAGES` every empty slot), resident (busied) or made (zero-filled, case 2; read
/// from swap, case 3: see the module's deviations), busy for the caller, into `pps`. Entered
/// with the object's lock held for writing (`w`), returns with it released.
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn uao_get_sync(
    aobj: &Aobj,
    w: WriteWitness<'_>,
    offset: Voff,
    pps: &mut [PageSlot],
    npagesp: &mut i32,
    centeridx: i32,
    _access_type: VmProt,
    _advice: i32,
    flags: i32,
) -> i32 {
    let uobj = &aobj.obj;
    let lock = uobj.lock();
    uobj.check_witness(w.lock_id(), "uao_get");
    kassert!(uobj.is_aobj());
    kassert!(uobj.write_held());
    kassert!(flags & PGO_LOCKED == 0);

    // get number of pages
    let maxpages = *npagesp;
    let firstpage = offset >> PAGE_SHIFT;
    if !uao_get_validate(aobj, firstpage, maxpages, centeridx, flags) {
        *npagesp = 0;
        drop(w);
        return VM_PAGER_BAD;
    }
    kassert!(pps.len() >= maxpages as usize);

    let owner = Owner::Object(uobj);
    let mut w = w;
    // step 2: get non-resident or busy pages. data structures are unlocked.
    for (lcv, slot) in pps.iter_mut().enumerate().take(maxpages as usize) {
        let page_offset = offset + ((lcv as Voff) << PAGE_SHIFT);

        // - skip over pages we've already gotten or don't want
        // - skip over pages we don't _have_ to get
        if !slot.is_empty() || (lcv as i32 != centeridx && flags & PGO_ALLPAGES == 0) {
            continue;
        }

        let pageidx = (firstpage + lcv as Voff) as i32;

        // we have yet to locate the current page (pps[lcv]).   we first look for a page that
        // is already at the current offset. if we find a page, we check to see if it is busy
        // or released.  if that is the case, then we sleep on the page until it is no longer
        // busy or released and repeat the lookup. if the page we found is neither busy nor
        // released, then we busy it (so we own it) and plug it into pps[lcv].   this
        // 'break's the following while loop and indicates we are ready to move on to the
        // next page in the "lcv" loop above.
        //
        // if we exit the while loop with pps[lcv] still empty, then it means that we
        // allocated a new busy/fake/clean page ptmp in the object and we need to do I/O to
        // fill in the data.

        // top of "pps" while loop
        let ptmp: &'static VmPage = loop {
            // look for a resident page
            let Some(ptmp) = uvm_pagelookup(uobj, page_offset) else {
                // not resident?   allocate one now (if we can)
                let Some(ptmp) = uvm_pagealloc(owner, page_offset, 0) else {
                    // out of RAM?
                    drop(w);
                    uvm_wait("uao_getpage");
                    w = lock.write();
                    // goto top of pps while loop
                    continue;
                };

                // safe with PQ's unlocked: because we just alloc'd the page
                ptmp.set_bits(PQ_AOBJ);

                // got new page ready for I/O.  break pps while loop.  pps[lcv] is still empty.
                break ptmp;
            };

            // page is there, see if we need to wait on it
            if ptmp.flags() & PG_BUSY != 0 {
                uvm_pagewait(ptmp, owner, w, "uao_get");
                w = lock.write();
                continue; // goto top of pps while loop
            }

            // if we get here then the page is resident and unbusy.  we busy it now (so we
            // own it).
            // we own it, caller must un-busy
            ptmp.set_bits(PG_BUSY);
            // UVM_PAGE_OWN(ptmp, "uao_get2"): not configured.
            *slot = PageSlot::Page(ptmp);
            break ptmp;
        };

        // if we own the valid page at the correct offset, pps[lcv] will point to it.
        // nothing more to do except go to the next page.
        if !slot.is_empty() {
            continue; // next lcv
        }

        // we have a "fake/busy/clean" page that we just allocated. do the needed "i/o",
        // either reading from swap or zeroing.
        let swslot = uao_find_swslot(aobj, &w, pageidx);

        // just zero the page if there's nothing in swap.
        if swslot == 0 {
            // page hasn't existed before, just zero it.
            uvm_pagezero(ptmp);
        } else {
            // page in the swapped-out page. unlock object for i/o, relock when done.
            drop(w);
            // rv = uvm_swap_get(ptmp, swslot, PGO_SYNCIO): swap (N3.5); an error here.
            let _ = unported!("uao_get: uvm_swap_get (swap, M7)");
            let rv = VM_PAGER_ERROR;
            w = lock.write();

            // I/O done.  check for errors.
            if rv != VM_PAGER_OK {
                // remove the swap slot from the aobj and mark the aobj as having no real
                // slot.  don't free the swap slot, thus preventing it from being used again:
                // uao_set_swslot(&aobj->u_obj, pageidx, SWSLOT_BAD); uvm_swap_markbad(swslot,
                // 1) (swap, N3.5).
                let _ = unported!("uao_get: SWSLOT_BAD / uvm_swap_markbad (swap, M7)");

                if ptmp.flags() & PG_WANTED != 0 {
                    hal::sleep::wakeup(ptmp.chan());
                }
                ptmp.clear_bits(PG_WANTED | PG_BUSY);
                // UVM_PAGE_OWN(ptmp, NULL): not configured.
                uvm_pagefree(ptmp, owner);
                drop(w);

                return rv;
            }
        }

        // we got the page!   clear the fake flag (indicates valid data now in page) and plug
        // into our result array.   note that page is still busy.
        //
        // it is the callers job to: check if the page is released, unbusy the page, activate
        // the page.
        ptmp.clear_bits(PG_FAKE);
        let _ = hal::pmap::clear_modify(&ptmp.mdpage); // ... and clean
        *slot = PageSlot::Page(ptmp);
    } // lcv loop

    drop(w);
    VM_PAGER_OK
}

/// `uao_dropswap`: release any swap resources from this aobj page. The object's lock is held
/// for writing (`w`).
pub fn uao_dropswap(aobj: &Aobj, w: &mut WriteWitness<'_>, pageidx: i32) -> i32 {
    kassert!(aobj.obj.is_aobj());

    let slot = uao_set_swslot(aobj, w, pageidx, 0);
    if slot != 0 {
        // uvm_swap_free(slot, 1): swap (N3.5).
        let _ = unported!("uao_dropswap: uvm_swap_free (swap, M7)");
    }
    slot
}

/// `uao_swap_off`: page in every page in every aobj that is paged-out to a range of swslots.
/// Returns `true` if pagein was aborted due to lack of memory.
///
/// The walk of every aobj needs the global list, which arrives with swapoff (see the
/// module's deviations): reported, nothing paged in.
pub fn uao_swap_off(startslot: i32, endslot: i32) -> bool {
    let _ = (startslot, endslot);
    let _ = unported!("uao_swap_off: the aobj list (uvm_swap, N3.5)");
    false
}

/// The first page of `aobj` from page `from` (array) or from the top (hash) whose swap slot is
/// in `[startslot, endslot)`.
fn next_slot_in_range(
    aobj: &Aobj,
    w: &impl Witness,
    from: usize,
    startslot: i32,
    endslot: i32,
) -> Option<i32> {
    let in_range = |slot: i32| slot >= startslot && slot < endslot;
    match aobj.swap.get(w) {
        Swap::Hash(table) => table.iter().rev().find_map(|head| {
            let mut cur = head.as_deref();
            while let Some(elt) = cur {
                if let Some(i) = elt.slots.iter().position(|&s| in_range(s)) {
                    return Some((elt.pageidx_base() as i32) + i as i32);
                }
                cur = elt.next.as_deref();
            }
            None
        }),
        Swap::Array(slots) => slots
            .iter()
            .enumerate()
            .skip(from)
            .find(|&(_, &s)| in_range(s))
            .map(|(i, _)| i as i32),
        Swap::None => None,
    }
}

/// `uao_pagein`: page in any pages from aobj in the given range. The object's lock is held for
/// writing (`w`) and comes back held.
///
/// Returns `true` if pagein was aborted due to lack of memory.
pub fn uao_pagein<'a>(
    aobj: &'a Aobj,
    w: WriteWitness<'a>,
    startslot: i32,
    endslot: i32,
) -> (bool, WriteWitness<'a>) {
    let mut w = w;
    let mut from = 0;
    loop {
        // process the page, then start over on this object since the swhash elt may have been
        // freed (the array goes on with the next page).
        let Some(pageidx) = next_slot_in_range(aobj, &w, from, startslot, endslot) else {
            return (false, w);
        };
        let (rv, w2) = uao_pagein_page(aobj, w, pageidx);
        w = w2;
        if rv {
            return (true, w);
        }
        if !aobj.uses_swhash() {
            from = pageidx as usize + 1;
        }
    }
}

/// `uao_pagein_page`: page in a single page from an anonymous UVM object. The object's lock
/// is held for writing (`w`) and comes back held.
///
/// Returns `true` if pagein was aborted due to lack of memory.
#[allow(
    clippy::match_same_arms,
    reason = "the C keeps the cases apart; OpenBSD's shape is kept so it can be grepped"
)]
pub fn uao_pagein_page<'a>(
    aobj: &'a Aobj,
    w: WriteWitness<'a>,
    pageidx: i32,
) -> (bool, WriteWitness<'a>) {
    let uobj = &aobj.obj;
    let mut pg = [PageSlot::Empty];
    let mut npages = 1;

    kassert!(uobj.write_held());
    let rv = uao_get_sync(
        aobj,
        w,
        Voff::from(pageidx) << PAGE_SHIFT,
        &mut pg,
        &mut npages,
        0,
        PROT_READ | PROT_WRITE,
        0,
        0,
    );

    // relock and finish up.
    let mut w = uobj.lock().write();
    match rv {
        VM_PAGER_OK => {}
        // nothing more to do on errors. VM_PAGER_REFAULT can only mean that the anon was
        // freed, so again there's nothing to do.
        VM_PAGER_ERROR | VM_PAGER_REFAULT => return (false, w),
        _ => {}
    }
    let Some(pg) = pg[0].page() else {
        kpanic(format_args!("uao_pagein_page: no page"));
    };

    // ok, we've got the page now. mark it as dirty, clear its swslot and un-busy it.
    let _ = uao_dropswap(aobj, &mut w, pageidx);
    pg.clear_bits(PG_BUSY | PG_CLEAN | PG_FAKE);
    // UVM_PAGE_OWN(pg, NULL): not configured.

    // deactivate the page (to put it on a page queue).
    uvm_pagedeactivate(pg, Owner::Object(uobj));

    (false, w)
}

/// `uao_dropswap_range`: drop swapslots in the range.
///
/// Aobj must be locked (`w`) and is returned locked. `start` is inclusive, `end` is exclusive
/// (0: the end of the object).
pub fn uao_dropswap_range(aobj: &Aobj, w: &mut WriteWitness<'_>, start: Voff, end: Voff) {
    let mut swpgonlydelta = 0;
    let uobj = &aobj.obj;

    kassert!(uobj.write_held());

    let end = if end == 0 { Voff::MAX } else { end };

    match aobj.swap.get_mut(w) {
        Swap::Hash(table) => {
            let taglo = uao_swhash_elt_tag(start);
            let taghi = uao_swhash_elt_tag(end);

            for head in table.iter_mut() {
                chain_retain(head, |e| {
                    if e.tag < taglo || taghi < e.tag {
                        return true;
                    }

                    let startidx = if e.tag == taglo {
                        uao_swhash_elt_pageslot_idx(start)
                    } else {
                        0
                    };

                    let endidx = if e.tag == taghi {
                        uao_swhash_elt_pageslot_idx(end)
                    } else {
                        UAO_SWHASH_CLUSTER_SIZE
                    };

                    for j in startidx..endidx {
                        let slot = e.slots[j];

                        kassert!(
                            uvm_pagelookup(uobj, (e.pageidx_base() + j as Voff) << PAGE_SHIFT)
                                .is_none()
                        );

                        if slot > 0 {
                            // uvm_swap_free(slot, 1): swap (N3.5).
                            let _ = unported!("uao_dropswap_range: uvm_swap_free (swap, M7)");
                            swpgonlydelta += 1;
                            kassert!(e.count > 0);
                            e.slots[j] = 0;
                            e.count -= 1;
                        }
                    }

                    e.count != 0
                });
            }
        }
        Swap::Array(slots) => {
            let end = end.min(Voff::from(aobj.pages()));
            for i in start..end {
                let slot = slots[i as usize];

                if slot > 0 {
                    // uvm_swap_free(slot, 1): swap (N3.5).
                    let _ = unported!("uao_dropswap_range: uvm_swap_free (swap, M7)");
                    swpgonlydelta += 1;
                }
            }
        }
        Swap::None => {}
    }

    // adjust the counter of pages only in swap for all the swap slots we've freed.
    if swpgonlydelta > 0 {
        kassert!(UVMEXP.swpgonly.load(Ordering::Relaxed) >= swpgonlydelta);
        UVMEXP.swpgonly.fetch_sub(swpgonlydelta, Ordering::Relaxed);
    }
}

/// The aobj pool's and the swap-hash element pool's items out (`pr_nout`), for the tests.
#[cfg(test)]
pub(crate) fn aobjs_out() -> (u64, u64) {
    (AOBJ_POOL.nout(), SWHASH_ELT_POOL.nout())
}

const _: () = {
    // An element is the C's pool item: `struct uao_swhash_elt`'s `LIST_ENTRY` (where the chain
    // needs one link and the handle's pool), tag, count and 16 slots.
    assert!(size_of::<SwhashElt>() == 96);
};

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use super::*;
    use crate::testutil;
    use crate::vm::page::uvm_page_unbusy;

    /// Real page memory, the lock objects and the aobj pools.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(256 * 4096);
        crate::testutil::init_shared_once();
        let _ = uao_init();
        guard
    }

    /// The slot count of the swap-hash element of page `idx`, if there is one.
    fn elt_count(aobj: &Aobj, w: &impl Witness, idx: Voff) -> Option<i32> {
        match aobj.swap.get(w) {
            Swap::Hash(t) => {
                chain_find(&t[bucket_of(t, idx)], uao_swhash_elt_tag(idx)).map(|e| e.count)
            }
            _ => None,
        }
    }

    fn create(pages: usize) -> (ObjRef, PoolArc<Aobj>) {
        let obj = uao_create(Vsize::new(pages * PAGE_SIZE), 0).expect("an aobj");
        let ObjRef::Aobj(a) = &obj else {
            panic!("not an aobj");
        };
        let a = a.clone();
        (obj, a)
    }

    #[test]
    fn small_aobj_keeps_swap_slots_in_an_array() {
        let _g = setup();
        let (uobj, aobj) = create(4);
        assert!(uobj.is_aobj());
        assert!(!aobj.uses_swhash());
        assert_eq!(aobj.pages(), 4);
        assert_eq!(uobj.refs(), 1);

        let mut w = uobj.lock().write();
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 0);
        assert_eq!(uao_set_swslot(&aobj, &mut w, 1, 7), 0);
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 7);
        assert_eq!(uao_set_swslot(&aobj, &mut w, 1, 9), 7);
        assert_eq!(uao_dropswap(&aobj, &mut w, 1), 9);
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 0);
        drop(w);

        uobj.reference();
        assert_eq!(uobj.refs(), 2);
        uobj.detach();
        assert_eq!(uobj.refs(), 1);
        uobj.detach();
    }

    #[test]
    fn large_aobj_hashes_swap_slots() {
        let _g = setup();
        let pages = (UAO_SWHASH_THRESHOLD + 1) as usize;
        let (uobj, aobj) = create(pages);
        assert!(aobj.uses_swhash());

        let mut w = uobj.lock().write();
        match aobj.swap.get(&w) {
            Swap::Hash(t) => assert_eq!(t.len(), 4, "65 pages hash into 4 buckets"),
            _ => panic!("no hash"),
        }
        let (_, elts) = aobjs_out();
        assert_eq!(uao_set_swslot(&aobj, &mut w, 3, 11), 0);
        assert_eq!(uao_set_swslot(&aobj, &mut w, 5, 12), 0); // the same cluster
        assert_eq!(uao_set_swslot(&aobj, &mut w, 40, 13), 0); // another one
        assert_eq!(aobjs_out().1, elts + 2);
        assert_eq!(elt_count(&aobj, &w, 3), Some(2));
        assert_eq!(uao_find_swslot(&aobj, &w, 5), 12);
        assert_eq!(uao_find_swslot(&aobj, &w, 40), 13);
        assert_eq!(uao_find_swslot(&aobj, &w, 41), 0);

        assert_eq!(uao_set_swslot(&aobj, &mut w, 3, 0), 11);
        assert_eq!(aobjs_out().1, elts + 2, "one slot left in the element");
        assert_eq!(elt_count(&aobj, &w, 3), Some(1));
        assert_eq!(uao_set_swslot(&aobj, &mut w, 5, 0), 12);
        assert_eq!(
            aobjs_out().1,
            elts + 1,
            "an element with no slots left goes back to the pool"
        );

        // the slot at 40 stands for a page only in swap; dropping the range accounts for it
        UVMEXP.swpgonly.fetch_add(1, Ordering::Relaxed);
        uao_dropswap_range(&aobj, &mut w, 0, 0);
        assert_eq!(uao_find_swslot(&aobj, &w, 40), 0);
        assert_eq!(UVMEXP.swpgonly.load(Ordering::Relaxed), 0);
        assert_eq!(aobjs_out().1, elts);
        drop(w);

        uobj.detach();
    }

    #[test]
    fn uao_get_zero_fills_and_finds_resident_pages() {
        let _g = setup();
        let (uobj, _aobj) = create(4);
        let off = PAGE_SIZE as Voff;
        let mut pps = [PageSlot::Empty; 1];
        let mut npages = 1;

        // PGO_LOCKED on a page that is not resident: the caller must unlock and come back
        let w = uobj.lock().write();
        let rv = uobj.get(&w, off, &mut pps, &mut npages, 0, PROT_READ, 0, PGO_LOCKED);
        assert_eq!(rv, VM_PAGER_UNLOCK);
        assert_eq!(npages, 0);
        assert!(pps[0].is_empty());

        // the full path allocates a zero-filled page, busy for the caller, and drops the lock
        npages = 1;
        let rv = uobj.get_sync(
            w,
            off,
            &mut pps,
            &mut npages,
            0,
            PROT_READ | PROT_WRITE,
            0,
            0,
        );
        assert_eq!(rv, VM_PAGER_OK);
        assert!(!uobj.lock_held());
        let pg = pps[0].page().expect("a page");
        assert!(pg.flags() & PG_BUSY != 0);
        assert_eq!(pg.flags() & PG_FAKE, 0);
        assert!(pg.flags() & PQ_AOBJ != 0);
        assert_eq!(pg.offset(), off);
        assert!(pg.owned_by_object(&*uobj));
        assert_eq!(uobj.npages(), 1);

        // once unbusied, PGO_LOCKED finds it, a read witness enough, and leaves the
        // PGO_DONTCARE slot alone
        {
            let _w = uobj.lock().write();
            uvm_page_unbusy(&[Some(pg)], Owner::Object(&*uobj));
        }
        let r = uobj.lock().read();
        let mut pps2 = [PageSlot::DontCare, PageSlot::Empty];
        let mut n2 = 2;
        let rv = uobj.get(&r, 0, &mut pps2, &mut n2, 1, PROT_READ, 0, PGO_LOCKED);
        assert_eq!(rv, VM_PAGER_OK);
        assert_eq!(n2, 1);
        assert!(pps2[1].page().is_some_and(|p| core::ptr::eq(p, pg)));
        assert!(pps2[0].is_dontcare());
        drop(r);

        // past the object: VM_PAGER_BAD, and the lock is dropped
        let mut pps3 = [PageSlot::Empty; 1];
        let mut n3 = 1;
        let w = uobj.lock().write();
        let rv = uobj.get_sync(w, 4 * off, &mut pps3, &mut n3, 0, PROT_READ, 0, 0);
        assert_eq!(rv, VM_PAGER_BAD);
        assert_eq!(n3, 0);
        assert!(!uobj.lock_held());

        // the last reference frees the page with the object
        uobj.detach();
        assert_eq!(uobj.npages(), 0);
    }

    #[test]
    fn the_memory_goes_with_the_last_handle_after_the_last_detach() {
        let _g = setup();
        let (out, _) = aobjs_out();
        let (uobj, aobj) = create(2);
        let entry = uobj.clone(); // a second map entry's handle, with its reference
        entry.reference();
        uobj.detach();
        assert_eq!(aobjs_out().0, out + 1);
        entry.detach(); // the last reference: the object is destroyed
        assert_eq!(aobj.object().refs(), 0);
        drop((uobj, entry));
        assert_eq!(aobjs_out().0, out + 1, "a handle is left");
        drop(aobj);
        assert_eq!(aobjs_out().0, out, "the last handle gave the item back");
    }

    #[test]
    fn detach_waits_for_a_busy_page() {
        let _g = setup();
        let (uobj, _aobj) = create(1);
        let mut pps = [PageSlot::Empty; 1];
        let mut npages = 1;
        let w = uobj.lock().write();
        assert_eq!(
            uobj.get_sync(w, 0, &mut pps, &mut npages, 0, PROT_READ | PROT_WRITE, 0, 0),
            VM_PAGER_OK
        );
        let pg = pps[0].page().expect("a page");
        let holder = uobj.clone();
        let t = std::thread::spawn(move || {
            holder.detach(); // sleeps until the page is unbusied
            holder.npages()
        });
        // Let the detach find the busy page, then unbusy it.
        while pg.flags() & PG_WANTED == 0 {
            std::thread::yield_now();
        }
        {
            let _w = uobj.lock().write();
            uvm_page_unbusy(&[Some(pg)], Owner::Object(&*uobj));
        }
        assert_eq!(t.join().expect("the detach"), 0);
    }

    #[test]
    fn the_kernel_object_is_static_and_never_detached() {
        let _g = setup();
        if kernel_object().is_none() {
            let k = uao_create(Vsize::new(16 * PAGE_SIZE), UAO_FLAG_KERNOBJ).expect("kobj");
            assert!(matches!(k, ObjRef::Kernel(_)));
        }
        let k = kernel_object().expect("the kernel object");
        assert!(k.is_kern_object());
        assert_eq!(k.lock().name(), "kobjlk");
        k.reference();
        k.detach();
        assert_eq!(k.refs(), UVM_OBJ_KERN);
    }

    #[cfg(feature = "tmpfs")]
    #[test]
    fn grow_and_shrink_keep_the_swap_slots_below_the_end() {
        let _g = setup();
        let (uobj, aobj) = create(4);
        let mut w = uobj.lock().write();
        assert_eq!(uao_set_swslot(&aobj, &mut w, 1, 7), 0);
        assert_eq!(uao_set_swslot(&aobj, &mut w, 3, 9), 0);

        // case 2: the array grows
        uao_grow(&aobj, &mut w, 8).expect("grow the array");
        assert!(!aobj.uses_swhash());
        assert_eq!(aobj.pages(), 8);
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 7);
        assert_eq!(uao_find_swslot(&aobj, &w, 7), 0);

        // case 3: past the threshold the slots move to a hash table
        uao_grow(&aobj, &mut w, 100).expect("convert to a hash");
        assert!(aobj.uses_swhash());
        assert!(matches!(aobj.swap.get(&w), Swap::Hash(_)));
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 7);
        assert_eq!(uao_find_swslot(&aobj, &w, 3), 9);

        // case 1: a bigger table, the elements rehashed by their tags
        uao_grow(&aobj, &mut w, 300).expect("grow the hash");
        match aobj.swap.get(&w) {
            Swap::Hash(t) => assert_eq!(t.len(), 32, "18 buckets round up to 32"),
            _ => panic!("no hash"),
        }
        assert_eq!(uao_set_swslot(&aobj, &mut w, 290, 11), 0);
        assert_eq!(uao_set_swslot(&aobj, &mut w, 70, 12), 0);
        assert_eq!(uao_find_swslot(&aobj, &w, 3), 9);

        // shrinking the hash drops the slots past the end (a page only in swap each)
        UVMEXP.swpgonly.fetch_add(1, Ordering::Relaxed);
        uao_shrink(&aobj, &mut w, 200).expect("shrink the hash");
        assert_eq!(aobj.pages(), 200);
        assert_eq!(UVMEXP.swpgonly.load(Ordering::Relaxed), 0);
        assert_eq!(uao_find_swslot(&aobj, &w, 70), 12);
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 7);

        // case 1 of shrink: back to an array below the threshold
        UVMEXP.swpgonly.fetch_add(1, Ordering::Relaxed);
        uao_shrink(&aobj, &mut w, 10).expect("convert to an array");
        assert!(!aobj.uses_swhash());
        assert!(matches!(aobj.swap.get(&w), Swap::Array(_)));
        assert_eq!(UVMEXP.swpgonly.load(Ordering::Relaxed), 0);
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 7);
        assert_eq!(uao_find_swslot(&aobj, &w, 3), 9);

        // case 2 of shrink: a smaller array (the slot at 3 goes)
        UVMEXP.swpgonly.fetch_add(1, Ordering::Relaxed);
        uao_shrink(&aobj, &mut w, 2).expect("shrink the array");
        assert_eq!(UVMEXP.swpgonly.load(Ordering::Relaxed), 0);
        assert_eq!(aobj.pages(), 2);
        assert_eq!(uao_find_swslot(&aobj, &w, 1), 7);
        assert_eq!(uao_set_swslot(&aobj, &mut w, 1, 0), 7);
        drop(w);

        uobj.detach();
    }

    #[cfg(feature = "tmpfs")]
    #[test]
    fn shrink_frees_the_resident_pages_past_the_end() {
        let _g = setup();
        let (uobj, aobj) = create(4);
        for idx in [0, 3] {
            let mut pps = [PageSlot::Empty; 1];
            let mut npages = 1;
            let w = uobj.lock().write();
            let rv = uobj.get_sync(
                w,
                (idx * PAGE_SIZE) as Voff,
                &mut pps,
                &mut npages,
                0,
                PROT_READ | PROT_WRITE,
                0,
                0,
            );
            assert_eq!(rv, VM_PAGER_OK);
            let pg = pps[0].page().expect("a page");
            let _w = uobj.lock().write();
            uvm_page_unbusy(&[Some(pg)], Owner::Object(&*uobj));
        }
        assert_eq!(uobj.npages(), 2);

        let mut w = uobj.lock().write();
        uao_shrink(&aobj, &mut w, 1).expect("shrink");
        assert_eq!(uobj.npages(), 1, "the page at index 3 is gone");
        assert!(uvm_pagelookup(&*uobj, 0).is_some());
        uao_grow(&aobj, &mut w, 4).expect("grow");
        assert_eq!(uobj.npages(), 1);
        drop(w);

        uobj.detach();
    }
}
/* </TESTS> */
