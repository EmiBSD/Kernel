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
 *
 * from: Id: uvm_fault.h,v 1.1.2.2 1997/12/08 16:07:12 chuck Exp
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
 *
 * from: Id: uvm_fault.c,v 1.1.2.23 1998/02/06 05:29:05 chs Exp
 */
/* </LICENSES> */

/* <CODE> */
//! The fault handler: `uvm_fault.h` and `uvm_fault.c`.
//!
//! Upstream: sys/uvm/uvm_fault.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_fault.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_fault.rs@3c62ede68802
//!
//! A word on page faults. Types of page faults we handle:
//!
//! - case 0: layerless fault: no amap or uobj is present. This is an error.
//! - case 1: upper layer fault (anon active): 1A, read or write with `an_ref == 1`: I/O
//!   takes place in upper level anon and uobj is not touched; 1B, write with `an_ref > 1`:
//!   new anon is alloc'd and data is copied off (COW).
//! - case 2: lower layer fault (uobj): 2A, read on non-NULL uobj or write to
//!   non-copy_on_write area: I/O takes place directly in object; 2B, write to copy_on_write
//!   or read on NULL uobj: data is "promoted" from uobj to a new anon; if uobj is null, then
//!   we zero fill.
//!
//! We follow the standard UVM locking protocol ordering: MAPS => AMAP => UOBJ => ANON =>
//! PAGE QUEUES (PQ). We hold a `PG_BUSY` page if we unlock for I/O.
//!
//! The code is structured as follows: init the "IN" params in the ufi structure; ReFault
//! (`ERESTART` returned to the loop in [`uvm_fault`]): do lookups (locks maps), check
//! protection, handle needs_copy; check for case 0 fault (error); establish "range" of
//! fault; if we have an amap lock it and extract the anons; if sequential advice deactivate
//! pages behind us; at the same time check pmap for unmapped areas and anon for pages that
//! we could map in (and do map it if found); check object for resident pages that we could
//! map in; if (case 2) goto Case2; handle case 1: ensure source anon is resident in RAM, if
//! case 1B alloc new anon and copy from source, map the correct page in; Case2: ensure
//! source page is resident (if uobj), if case 2B alloc new anon and copy from source (could
//! be zero fill if uobj == NULL), map the correct page in; done!
//!
//! Note on paging: if we have to do I/O we place a `PG_BUSY` page in the correct object,
//! unlock everything, and do the I/O. When I/O is done we must reverify the state of the
//! world before assuming that our data structures are valid (because mappings could change
//! while the map is unlocked). The system keeps a "version" number of a map (every time you
//! write-lock a map you bump it), so we save the version number before we release the lock
//! and start I/O, then relock and check the version numbers to see if anything changed.
//!
//! The traps reach it through `bsd`'s `sys/uvm/uvm_fault.rs` (N3.4e), whose `VmMap` implements
//! [`FaultMap`]; the kernel's objects implement [`FaultObject`].
//!
//! ## Deviations
//! - `uvmadvice[]` is a constant table: `PAGE_SIZE` is a constant here, so the values
//!   `uvmfault_init` computes are too; [`uvmfault_init`] only checks them (LZ's).
//! - Out parameters become return values: [`uvmfault_promote`] yields the anon and the page,
//!   [`uvm_fault_check`] the entry it read and the amap's witness, [`uvm_fault_lower_io`]
//!   the object page (LZ's).
//! - `uvm_swap_get` and `uvm_swap_markbad` (`uvmfault_anonget`) are reported: nothing has a
//!   swap slot before `uvm_swap.c` (N3.5; LZ's).
//! - The fault starts with shared (`RW_READ`) upper and lower locks, as the C does on amd64
//!   and arm64 (LZ's).
//! - `pmap_nested` is 0 (no vmm); both machines define `__HAVE_PMAP_MPSAFE_ENTER_COW` (so the
//!   `MULTIPROCESSOR` COW paths that need its absence are not compiled); `UVM_PAGE_OWN` and
//!   `TRACEPOINT` (dt) are not configured (LZ's).
//! - arm64 defines `__HAVE_PMAP_POPULATE` (`arch/arm64/include/pmap.h`): after a failed
//!   `pmap_enter` its C calls `pmap_populate(pmap, va)` (`arm64/pmap.c`) where amd64's waits in
//!   `uvm_wait`. `pmap_populate` is not ported (inherited from LZ, which compiled the amd64
//!   branch on both machines; `hal`'s arm64 pmap has no `pmap_vp_populate`): both machines
//!   take the `uvm_wait` branch, then `ERESTART`, as LZ.
//! - No pager implements `pgo_fault` (`uvm_device.c`, the only one, is not ported; DECISIONS
//!   G3 of N3.4): the fault's branch that hands the fault to it goes with the operation.
//! - `uvmfault_anonget`, `uvmfault_relock` and `uvmfault_unlockmaps` always have a fault info:
//!   the C's `ufi == NULL` (paging an anon in without a fault, `uvm_anon_pagein` for swapoff)
//!   arrives with swap (N3.5); `kernel::vm::anon::uvm_anon_pagein` reports it until then (an
//!   inherited gap, kept visible).
//! - The lower layer's `amap_add` failure (`uvm_fault_lower`, `uvm_fault.c` 1468-1485): the C
//!   unlocks everything, then calls `uvm_anfree` on the fresh anon with `an_ref` still 1 and
//!   its lock released (under `DIAGNOSTIC`, GENERIC's option, its two `KASSERT`s panic), and
//!   `amap_populate(&ufi->entry->aref, ..)` reads the entry after the map was unlocked and
//!   writes the amap unlocked (another process may unmap and free it meanwhile: a use after
//!   free, and a race with any other writer of the amap). Here the object and the maps are
//!   released first, the anon (which no one else ever saw) has its reference dropped and is
//!   freed under the amap's write witness, and the amap is released: nothing is held, as in
//!   the C. `amap_populate` is split: the chunk is made with nothing held (the C's sleep
//!   point, `PR_WAITOK`), then installed under the amap's write lock taken afresh, skipped
//!   when the amap was wiped out meanwhile (`am_ref` 0) and left to another fault's chunk if
//!   one is there; the amap is the entry's as read before the unlock, kept alive by its
//!   handle. What differs: no panic where a `DIAGNOSTIC` kernel panics, no access to freed or
//!   unlocked memory; the retry (`ERESTART`) and the `ENOMEM` when `uvm_swapisfull()` are the
//!   C's.
//! - `uvm_swapisfull` reads the two swap counters without `uvm_swap_data_lock` (`uvm_swap`'s,
//!   `bsd`'s until N3.5): nothing writes them while `swapctl(2)` is `sys_nosys` at the pin, as
//!   `bsd`'s copy says. With no swap configured it answers true (0 >= 0), as the C.
//!
//! ## Redesign
//! - LZ's `UvmFaultinfo { map: *const VmMap, entry: *const VmMapEntry, .. }` with
//!   `ufi.entry()` making a `&'static` from the raw pointer, and the amap and object locks
//!   taken in one function and released in another (`amap_lock` in `uvm_fault_check`,
//!   `rw_enter(lock, rw_status(lock))` in `uvmfault_anonget`, `rw_enter(RW_UPGRADE |
//!   RW_NOSLEEP)` in the upgrades, `uvmfault_unlockall(ufi, amap, uobj)` on fifteen paths,
//!   `KASSERT(rw_status(lock) == flt->upper_lock_type)` at each step) -> the locks are
//!   witnesses that travel: [`FaultInfo`] holds the map's guard (`Option<M::Guard>`: `Some`
//!   exactly while the maps are locked), and the amap's and the object's witnesses
//!   (`hal::rwlock::RwWitness`, the mode the C keeps in `flt->upper_lock_type` is the
//!   witness's arm) are values in [`uvm_fault`]'s loop frame, returned by the function that
//!   takes the lock and consumed by the one that releases it. [`uvmfault_unlockall`] takes
//!   the witnesses it releases; [`uvmfault_anonget`] consumes the amap's and gives one back,
//!   taken again in the same mode, or none ([`AnonGet`]); the upgrades consume a witness and
//!   give the write witness or the reader back. No witness can outlive its round of the loop
//!   (it borrows that round's entry snapshot) and none can be released twice (it is moved),
//!   so the locks held at each point are the live witnesses. The order of the
//!   acquisitions is the code's, as the C's, and each one says where it stands in MAPS =>
//!   AMAP => UOBJ => ANON => PQ.
//! - The map is `bsd`'s until N3.5: the fault is generic over [`FaultMap`] (implemented by
//!   `bsd`'s `VmMap` since the flip, N3.4e; bridges.toml `FaultMap`), whose guard is a GAT that borrows
//!   the map and whose entries borrow the guard ([`FaultMapGuard::lookup`]), so no entry is
//!   read after its map was unlocked. `uvmfault_relock`'s "same version, else nothing held" is
//!   [`FaultMap::relock`] answering `Option<Guard>`. What the fault reads of an entry is taken
//!   once under the lock into an [`EntryView`] (the C reads `ufi->entry` while the map is
//!   locked, or after a relock that found the same version: the same values), holding the
//!   amap and the object by counted handles, which keep them alive across the unlocks (the
//!   C's pointers, valid because the map version did not change).
//! - The anon and page arrays (`anons_store[]` of raw pointers read by `anon_at`, `pages[]`
//!   with `PGO_DONTCARE` read by `page_at`): the neighbour anons are looked up under the
//!   amap's witness where they are used ([`uvm_fault_upper_lookup`], `uvmfault_anonflush`),
//!   and the centre anon that `uvmfault_anonget` keeps across its sleeps is an `AnonRef`
//!   clone; the pages are [`PageSlot`]s.
//! - The objects are reached through [`FaultObject`], the shape of the kernel's `ObjRef`
//!   (N3.4c: `get` with the object's witness, `get_sync` consuming the write witness, since
//!   the pagers return from a `PGO_SYNCIO` get without the object's lock, `flush` with the
//!   write witness), which implements it; the trait lets the tests script a pager (relock
//!   failures, `VM_PAGER_AGAIN`) besides running over the real aobjs.
//! - The C reads the I/O page's object back from the page (`uobj = pg->uobject`): a page here
//!   keeps an owner tag, never a pointer, and every pager returns its own pages, so the fault
//!   checks that the page is the asked object's and goes on with that object (a check, not a
//!   `KASSERT`: the fault would otherwise lock one object and touch another's page). The C's
//!   comment that a tmpfs vnode's pages are its UAO's describes NetBSD's tmpfs; OpenBSD's
//!   shares neither the pages nor the lock (`tmpfs_subr.c`'s `#if 0`), so the wired path's
//!   `KASSERT(uobj->vmobjlock == pg->uobject->vmobjlock)` is the page's owner being `uobj`.
//! - `curproc->p_ru` (`ru_minflt`, `ru_majflt`, `ru_maxrss`) is `bsd`'s process until N4: the
//!   map's notes ([`FaultMap::note_minflt`], `note_majflt`, `note_maxrss`).
//! - `pmap_enter` on `kernel_map`'s pmap goes through the kernel map's token (`vm::km`), a
//!   user pmap's through the handle (`hal::pmap::PmapRef`); `pmap_update` has nothing to do
//!   (`hal`'s pmap does its TLB work in each call).
//! - `uvm_fault_unwire_locked(map, start, end)` takes the map's guard as the proof of the
//!   lock (`vm_map_assert_anylock`), and `uvm_map_lock_entry`/`uvm_map_unlock_entry` around
//!   each entry are an [`EntryLocks`] whose drop releases the object's lock, then the amap's.
//!   An entry without an amap has its page's owner looked for in its object only (N3.3's K1
//!   rule: the amap is looked up only when the entry has one).

use core::ptr;
use core::sync::atomic::Ordering;

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::malloc::M_NOWAIT;
use hal::pmap::{PMAP_CANFAIL, PMAP_WC, PMAP_WIRED, PmapRef};
use hal::pool::PR_WAITOK;
use hal::rwlock::{RwMode, RwWitness, SharedRwlock, Witness, WriteWitness};
use hal::sleep::{PVM, tsleep_nsec, wakeup};
use hal::sys::errno::Errno;
use hal::sys::mman::{MADV_NORMAL, MADV_SEQUENTIAL, PROT_NONE, PROT_WRITE};
use hal::sys::types::{Paddr, Vaddr};
use hal::vm::param::{atop, trunc_page};
use hal::vm::uvmexp::{UVMEXP, UvmExpCounters, counters_inc};

use super::amap::{
    AMAP_SHARED, Amap, AmapRef, amap_add, amap_chunk_alloc, amap_lookup, amap_lookups,
    amap_populate_with,
};
use super::anon::{
    AnonRef, SWSLOT_BAD, uvm_analloc, uvm_anfree, uvm_anon_dropswap, uvm_anon_release, uvm_anwait,
};
use super::aobj::uao_dropswap;
use super::km;
use super::object::ObjRef;
use super::page::{
    Owner, PG_BUSY, PG_CLEAN, PG_FAKE, PG_RELEASED, PG_WANTED, PHYS_TO_VM_PAGE, PQ_ANON, PQ_AOBJ,
    VmPage, uvm_pageactivate, uvm_pagealloc, uvm_pagecopy, uvm_pagedeactivate, uvm_pagefree,
    uvm_pageunwire, uvm_pagewait, uvm_pagewire,
};
use super::pager::{
    PGO_DEACTIVATE, PGO_LOCKED, PGO_SYNCIO, PageSlot, VM_PAGER_AGAIN, VM_PAGER_ERROR, VM_PAGER_OK,
    VM_PAGER_PEND,
};
use super::pmemrange::uvm_wait;
use super::types::{MADV_MASK, PROT_MASK, UVM_PGA_ZERO, VmFault, VmProt, Voff};
use crate::assert::{kassert, kpanic};
use crate::prf::unported;

/// `VM_FAULT_INVALID`: invalid mapping.
pub const VM_FAULT_INVALID: VmFault = 0x0;
/// `VM_FAULT_PROTECT`: protection.
pub const VM_FAULT_PROTECT: VmFault = 0x1;
/// `VM_FAULT_WIRE`: wire mapping.
pub const VM_FAULT_WIRE: VmFault = 0x2;

/*
 * The lower layer, as the fault sees it: the kernel's objects and pagers
 * (kernel::vm::{object, pager}, N3.4c).
 */

/// A uvm object with its pager, as the fault reaches it (`struct uvm_object` and its
/// `pgops`): the kernel's [`ObjRef`] (below), and the tests' double. `Clone` is a handle that
/// keeps the object alive, as the entry's (no `uo_refs` is taken: the map entry's reference
/// stands, as in the C).
pub trait FaultObject: Clone {
    /// `uobj->vmobjlock`.
    fn vmobjlock(&self) -> &SharedRwlock;
    /// The object as the owner of its pages, for the page functions.
    fn owner(&self) -> Owner<'_>;
    /// `pgo_get(uobj, off, pps, npages, centeridx, access, advice, PGO_LOCKED)`: the resident,
    /// neither busy nor released pages of `[off, off + npages)` into the slots that are not
    /// `DontCare`, their count in `npages`; the object stays locked (`w`, either mode).
    #[allow(clippy::too_many_arguments, reason = "pgo_get's signature")]
    fn get(
        &self,
        w: &RwWitness<'_>,
        off: Voff,
        pps: &mut [PageSlot],
        npages: &mut i32,
        centeridx: i32,
        access_type: VmProt,
        advice: i32,
        flags: i32,
    ) -> i32;
    /// `pgo_get(.., PGO_SYNCIO)`: the page at `off`, read in if need be, busy, in `pps[0]`;
    /// entered with the object locked for writing, returns without the lock (`w` consumed:
    /// the pagers' `KASSERT(rw_write_held)` and their unlock).
    #[allow(clippy::too_many_arguments, reason = "pgo_get's signature")]
    fn get_sync(
        &self,
        w: WriteWitness<'_>,
        off: Voff,
        pps: &mut [PageSlot],
        npages: &mut i32,
        centeridx: i32,
        access_type: VmProt,
        advice: i32,
        flags: i32,
    ) -> i32;
    /// `pgo_flush(uobj, start, stop, flags)`, the object locked for writing.
    fn flush(&self, w: &mut WriteWitness<'_>, start: Voff, stop: Voff, flags: i32) -> bool;
    /// `uao_dropswap(uobj, pageidx)` of an aobj: the page's swap slot released.
    fn dropswap(&self, w: &mut WriteWitness<'_>, pageidx: i32);
}

// The kernel's objects: each operation is the object's own (one call); the fault's other
// implementation is its tests' double.
impl FaultObject for ObjRef {
    fn vmobjlock(&self) -> &SharedRwlock {
        // A mapped object (an aobj, the kernel object, a vnode's) always has its lock; a dummy
        // (the pmap's, the buffer cache's) is never mapped: `Object::lock` panics on one.
        self.lock()
    }

    fn owner(&self) -> Owner<'_> {
        Owner::Object(&**self)
    }

    fn get(
        &self,
        w: &RwWitness<'_>,
        off: Voff,
        pps: &mut [PageSlot],
        npages: &mut i32,
        centeridx: i32,
        access_type: VmProt,
        advice: i32,
        flags: i32,
    ) -> i32 {
        ObjRef::get(
            self,
            w,
            off,
            pps,
            npages,
            centeridx,
            access_type,
            advice,
            flags,
        )
    }

    fn get_sync(
        &self,
        w: WriteWitness<'_>,
        off: Voff,
        pps: &mut [PageSlot],
        npages: &mut i32,
        centeridx: i32,
        access_type: VmProt,
        advice: i32,
        flags: i32,
    ) -> i32 {
        ObjRef::get_sync(
            self,
            w,
            off,
            pps,
            npages,
            centeridx,
            access_type,
            advice,
            flags,
        )
    }

    fn flush(&self, w: &mut WriteWitness<'_>, start: Voff, stop: Voff, flags: i32) -> bool {
        ObjRef::flush(self, w, start, stop, flags)
    }

    fn dropswap(&self, w: &mut WriteWitness<'_>, pageidx: i32) {
        // KASSERT(UVM_OBJ_IS_AOBJ(uobj)): only an aobj page is PQ_AOBJ.
        match self.aobj() {
            Some(aobj) => {
                let _ = uao_dropswap(aobj, w, pageidx);
            }
            None => kpanic(format_args!("uao_dropswap: not an aobj")),
        }
    }
}

/*
 * The map, as the fault sees it: bsd's VmMap until N3.5 (bridges.toml FaultMap), the
 * kernel's own map after.
 */

/// A map the fault resolves addresses in (`struct vm_map`): `bsd`'s `VmMap` implements it
/// (bridges.toml `FaultMap`, paid by N3.5, when the map moves into the kernel), the tests a
/// double. Every method is one call of the map's; none decides anything.
pub trait FaultMap: Sized + 'static {
    /// The objects its entries map.
    type Object: FaultObject;
    /// `struct vm_map_entry`.
    type Entry: FaultEntry<Map = Self, Object = Self::Object>;
    /// The map locked (`vm_map_lock`, `vm_map_lock_read`): dropped is `vm_map_unlock` or
    /// `vm_map_unlock_read`; the entries are read through it.
    type Guard<'a>: FaultMapGuard<Entry = Self::Entry, Map = Self>
    where
        Self: 'a;

    /// `vm_map_min(map)`.
    fn min_offset(&self) -> usize;
    /// `vm_map_max(map)`.
    fn max_offset(&self) -> usize;
    /// `vm_map_lock(map)` (`RwMode::Write`, which bumps the version) or `vm_map_lock_read`.
    fn lock(&self, mode: RwMode) -> Self::Guard<'_>;
    /// `uvmfault_relock`'s map half: `vm_map_lock_read(map)`, kept when the map's version is
    /// still `version`; otherwise `vm_map_unlock_read` and `None`, nothing held.
    fn relock(&self, version: u32) -> Option<Self::Guard<'_>>;
    /// `vm_map_pmap(map)`.
    fn pmap(&self) -> &PmapRef;
    /// `map->flags & VM_MAP_ISVMSPACE`.
    fn is_vmspace(&self) -> bool;
    /// `map->flags & VM_MAP_INTRSAFE`.
    fn is_intrsafe(&self) -> bool;
    /// `map->flags & VM_MAP_PAGEABLE`.
    fn is_pageable(&self) -> bool;
    /// `curproc->p_ru.ru_minflt++`.
    fn note_minflt(&self);
    /// `curproc->p_ru.ru_majflt++`.
    fn note_majflt(&self);
    /// `if (p->p_ru.ru_maxrss < kb) p->p_ru.ru_maxrss = kb` for `curproc`, whose vmspace is
    /// this map (the C's `KASSERT(p != NULL && &p->p_vmspace->vm_map == map)`).
    fn note_maxrss(&self, kb: usize);
}

/// A locked map ([`FaultMap::lock`]): the proof the entries it hands out are valid.
pub trait FaultMapGuard: Sized {
    /// The map's entries.
    type Entry;
    /// The map it locks.
    type Map;

    /// The map this guard locks.
    fn map(&self) -> &Self::Map;
    /// How the map is locked.
    fn mode(&self) -> RwMode;
    /// `map->timestamp`, the version a relock compares.
    fn version(&self) -> u32;
    /// `uvm_map_lookup_entry(map, va, &entry)`: the entry holding `va`.
    fn lookup(&self, va: usize) -> Option<&Self::Entry>;
    /// `RBT_NEXT(uvm_map_addr, entry)`: the next entry by address.
    fn next<'a>(&'a self, entry: &'a Self::Entry) -> Option<&'a Self::Entry>;
    /// `vm_map_upgrade(map)` of a read-locked map (which bumps the version): the map locked
    /// for writing, or still read-locked in the `Err` (the C's `FALSE`).
    fn upgrade(self) -> Result<Self, Self>;
    /// `vm_map_downgrade(map)`: write-locked to read-locked, without a window.
    fn downgrade(self) -> Self;
    /// `amap_copy(map, entry, waitf, canchunk, startva, endva)` (its map half, `bsd`'s
    /// `uvm_amap.rs`): the map is write-locked; the entry may be clipped, so the caller looks
    /// it up again.
    fn copy_amap(&self, entry: &Self::Entry, waitf: i32, canchunk: bool, start: usize, end: usize);
}

/// A map entry, as the fault reads it under its map's lock.
pub trait FaultEntry {
    /// The map a submap entry names.
    type Map: 'static;
    /// The object an entry maps.
    type Object;

    /// `entry->start`.
    fn start(&self) -> usize;
    /// `entry->end`.
    fn end(&self) -> usize;
    /// `entry->offset`.
    fn offset(&self) -> Voff;
    /// `entry->protection`.
    fn protection(&self) -> VmProt;
    /// `entry->advice`.
    fn advice(&self) -> i32;
    /// `VM_MAPENT_ISWIRED(entry)`: `wired_count != 0`.
    fn is_wired(&self) -> bool;
    /// `UVM_ET_ISSUBMAP(entry)`.
    fn is_submap(&self) -> bool;
    /// `UVM_ET_ISCOPYONWRITE(entry)`.
    fn is_copyonwrite(&self) -> bool;
    /// `UVM_ET_ISNEEDSCOPY(entry)`.
    fn is_needscopy(&self) -> bool;
    /// `UVM_ET_ISNOFAULT(entry)`.
    fn is_nofault(&self) -> bool;
    /// `UVM_ET_ISSTACK(entry)`.
    fn is_stack(&self) -> bool;
    /// `UVM_ET_ISWC(entry)`.
    fn is_wc(&self) -> bool;
    /// `entry->object.sub_map` of a submap entry (submaps live for ever).
    fn submap(&self) -> Option<&'static Self::Map>;
    /// `entry->aref`: the amap (a handle) and the reference's page offset, if any.
    fn aref(&self) -> Option<(AmapRef, i32)>;
    /// `entry->object.uvm_obj` of an object entry (`UVM_ET_ISOBJ`), a handle.
    fn object(&self) -> Option<Self::Object>;
}

/// `struct uvm_faultinfo`: the fault's address and the map it is resolved in.
pub struct FaultInfo<'m, M: FaultMap> {
    /// IN: original map.
    orig_map: &'m M,
    /// IN: original rounded VA.
    orig_rvaddr: usize,
    /// IN: original size of interest.
    orig_size: usize,
    /// The map [`uvmfault_lookup`] found (a submap or the original).
    map: &'m M,
    /// `map` locked: `Some` exactly while the maps are.
    guard: Option<M::Guard<'m>>,
    /// The map's version number when it was looked up (`mapv`).
    version: u32,
    /// Size of interest.
    size: usize,
}

impl<'m, M: FaultMap> FaultInfo<'m, M> {
    /// The IN parameters (`orig_*`), before [`uvmfault_lookup`].
    fn new(orig_map: &'m M, orig_rvaddr: usize, orig_size: usize) -> Self {
        Self {
            orig_map,
            orig_rvaddr,
            orig_size,
            map: orig_map,
            guard: None,
            version: 0,
            size: 0,
        }
    }

    /// The lock of the maps, which the caller holds.
    fn guard(&self) -> &M::Guard<'m> {
        match &self.guard {
            Some(g) => g,
            None => kpanic(format_args!("uvm_fault: the maps are not locked")),
        }
    }

    /// The lock of the maps, taken out to be changed or released.
    fn take_guard(&mut self) -> M::Guard<'m> {
        match self.guard.take() {
            Some(g) => g,
            None => kpanic(format_args!("uvm_fault: the maps are not locked")),
        }
    }
}

/// An entry's `aref`: its amap (a counted handle) and the page offset of its reference.
struct Aref {
    /// `ar_amap`.
    amap: AmapRef,
    /// `ar_pageoff`.
    pageoff: i32,
}

/// What the fault reads of the faulting entry, under the map lock: the C reads `ufi->entry`
/// while the map is locked, or after a relock found the same version, which are the values
/// taken here. The amap and the object are handles that keep them alive across the unlocks.
struct EntryView<O> {
    start: usize,
    end: usize,
    offset: Voff,
    protection: VmProt,
    advice: i32,
    wired: bool,
    wc: bool,
    copyonwrite: bool,
    needscopy: bool,
    nofault: bool,
    stack: bool,
    /// The upper layer.
    aref: Option<Aref>,
    /// The lower layer.
    object: Option<O>,
}

impl<O: FaultObject> EntryView<O> {
    /// The entry's fields now (its map locked).
    fn of<E: FaultEntry<Object = O>>(e: &E) -> Self {
        Self {
            start: e.start(),
            end: e.end(),
            offset: e.offset(),
            protection: e.protection(),
            advice: e.advice(),
            wired: e.is_wired(),
            wc: e.is_wc(),
            copyonwrite: e.is_copyonwrite(),
            needscopy: e.is_needscopy(),
            nofault: e.is_nofault(),
            stack: e.is_stack(),
            aref: e.aref().map(|(amap, pageoff)| Aref { amap, pageoff }),
            object: e.object(),
        }
    }

    /// `MASK(entry)`: the protection a copy-on-write entry may be entered with.
    fn mask(&self) -> VmProt {
        if self.copyonwrite {
            !PROT_WRITE
        } else {
            PROT_MASK
        }
    }
}

/*
 * local data structures
 */

/// `struct uvm_advice`: how many pages behind and ahead of the fault to map in.
#[derive(Clone, Copy)]
struct UvmAdvice {
    nback: usize,
    nforw: usize,
}

/// `UVM_MAXRANGE`: must be max() of nback+nforw+1.
pub const UVM_MAXRANGE: usize = 16;

/// `uvmadvice[MADV_MASK + 1]`: page range array (the values `uvmfault_init` computes).
const UVMADVICE: [UvmAdvice; MADV_MASK as usize + 1] = uvmadvice_table();

/// `uvmfault_init`'s computation, as a constant.
const fn uvmadvice_table() -> [UvmAdvice; MADV_MASK as usize + 1] {
    let mut table = [UvmAdvice { nback: 0, nforw: 0 }; MADV_MASK as usize + 1];

    let npages = atop(16384);
    if npages > 0 {
        assert!(npages <= UVM_MAXRANGE / 2);
        table[MADV_NORMAL as usize].nforw = npages;
        table[MADV_NORMAL as usize].nback = npages - 1;
    }

    let npages = atop(32768);
    if npages > 0 {
        assert!(npages <= UVM_MAXRANGE / 2);
        table[MADV_SEQUENTIAL as usize].nforw = npages - 1;
        table[MADV_SEQUENTIAL as usize].nback = npages;
    }
    table
}

/// `struct uvm_faultctx`: the following members are set up by [`uvm_fault_check`] and
/// read-only after that (but the lock modes, which the upgrades raise).
struct FaultCtx {
    enter_prot: VmProt,
    access_type: VmProt,
    startva: usize,
    npages: usize,
    centeridx: usize,
    narrow: bool,
    wired: bool,
    pa_flags: usize,
    promote: bool,
    /// `upper_lock_type`: the mode the amap's lock is (to be) taken in.
    upper: RwMode,
    /// `lower_lock_type`: the mode the object's lock is (to be) taken in.
    lower: RwMode,
}

/// `MSEC_TO_NSEC(5)`: `uvm_fault_lower_io`'s pause before it retries a pager's `AGAIN`.
const FLTAGAIN_NSEC: u64 = 5_000_000;

/// `nowake`: a wait channel no one ever wakes (a timed sleep's).
static NOWAKE: u8 = 0;

/// The physical address with the pmap flags OR'ed in (`VM_PAGE_TO_PHYS(pg) | pa_flags`).
fn pa_with_flags(pg: &VmPage, pa_flags: usize) -> Paddr {
    Paddr::new(pg.phys_addr().as_usize() | pa_flags)
}

/// `pmap_enter(pmap, va, pa, prot, flags)`: the kernel pmap through the kernel map's token
/// (the fault is one of the kernel map's own paths), a user pmap through its handle.
fn pmap_enter(pmap: &PmapRef, va: usize, pa: Paddr, prot: VmProt, flags: i32) -> Result<(), Errno> {
    match pmap {
        PmapRef::Kernel => km::mapper().enter(Vaddr::new(va), pa, prot, flags),
        PmapRef::User(_) => pmap.enter(Vaddr::new(va), pa, prot, flags),
    }
}

/// `uvm_swapisfull`: whether the pages only in swap account for 99% of the swap space or
/// more (see the module's deviations: no lock, and true with no swap).
fn uvm_swapisfull() -> bool {
    let swpgonly = i64::from(UVMEXP.swpgonly.load(Ordering::Relaxed));
    let swpages = i64::from(UVMEXP.swpages.load(Ordering::Relaxed));
    kassert!(swpgonly <= swpages);
    swpgonly >= swpages * 99 / 100
}

/// `uvm_swap_get(pg, swslot, PGO_SYNCIO)`: no swap before `uvm_swap` (N3.5): reported, and the
/// read fails (LZ's). The tests stand in for the pager here.
fn uvm_swap_get(pg: &'static VmPage, swslot: i32) -> i32 {
    #[cfg(test)]
    if let Some(result) = tests::swap_get_hook(pg, swslot) {
        return result;
    }
    let _ = (pg, swslot);
    let _ = unported!("uvmfault_anonget: uvm_swap_get (uvm_swap.c, M7)");
    VM_PAGER_ERROR
}

/*
 * inline functions
 */

/// `uvmfault_anonflush`: try and deactivate pages in the `n` anons of `aref` from `off` bytes
/// into the entry, under the amap's witness `w`.
///
/// - does not have to deactivate page if it is busy
fn uvmfault_anonflush(aref: &Aref, w: &RwWitness<'_>, off: usize, n: usize) {
    let mut anons: [Option<&AnonRef>; UVM_MAXRANGE] = [None; UVM_MAXRANGE];
    amap_lookups(&aref.amap, w, aref.pageoff, off, &mut anons[..n]);
    for anon in anons[..n].iter().flatten() {
        // KASSERT(rw_lock_held(anons[lcv]->an_lock)): `w` holds the amap's, which it shares.
        kassert!(anon.shares_lock(aref.amap.lock()));
        if let Some(pg) = anon.page()
            && pg.flags() & PG_BUSY == 0
        {
            uvm_pagedeactivate(pg, Owner::Anon(&***anon));
        }
    }
}

/*
 * normal functions
 */

/// `uvmfault_init`: compute proper values for the `uvmadvice[]` array (a constant here, see
/// the module's deviations).
pub fn uvmfault_init() {
    kassert!(
        UVMADVICE[MADV_NORMAL as usize].nforw + UVMADVICE[MADV_NORMAL as usize].nback
            < UVM_MAXRANGE
    );
    kassert!(
        UVMADVICE[MADV_SEQUENTIAL as usize].nforw + UVMADVICE[MADV_SEQUENTIAL as usize].nback
            < UVM_MAXRANGE
    );
}

/// `amap_populate(&ufi->entry->aref, ufi->orig_rvaddr - ufi->entry->start)` of the lower
/// layer's `amap_add` failure, with nothing held (see the module's deviations): the chunk is
/// made unlocked (the sleep is the C's), then installed under the amap's write lock taken
/// again, unless the amap was wiped out meanwhile (`am_ref` 0: its tables are gone) or the
/// slot's chunk was made by another fault; the spare goes back to its pool otherwise.
fn uvmfault_populate(amap: &Amap, pageoff: i32, off: usize) {
    let Some(spare) = amap_chunk_alloc(amap, pageoff, off, PR_WAITOK) else {
        return;
    };
    let mut w = amap.lock().write();
    if amap.refs() > 0 {
        amap_populate_with(amap, &mut w, pageoff, off, spare);
    } else {
        drop(spare);
    }
    drop(w);
}

/// What [`uvmfault_anonget`] did.
enum AnonGet<'w> {
    /// The anon's page is resident, neither busy nor released: everything still locked, the
    /// amap's witness given back (in the mode it was taken).
    Resident(RwWitness<'w>),
    /// The anon has no page and the amap is only read-locked: everything still locked, the
    /// witness given back (the C's `ENOLCK`: the caller upgrades and retries).
    NeedWrite(RwWitness<'w>),
    /// `ERESTART` (refault) or the I/O's error: everything unlocked.
    Unlocked(Errno),
}

/// `uvmfault_anonget`: get data in an anon into a non-busy, non-released page in that anon.
///
/// - Map, amap and thus anon should be locked by caller: the maps in `ufi`, the amap of
///   `aref` by `w`, whose lock `anon` (an entry `off` bytes into the entry) shares.
/// - If we fail, we unlock everything and error is returned.
/// - If we are successful, return with everything still locked.
/// - We do not move the page on the queues (gets moved later). If we allocate a new page
///   (we_own), it gets put on the queues. Either way, the result is that the page is on the
///   queues at return time.
fn uvmfault_anonget<'w, M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    aref: &'w Aref,
    off: usize,
    anon: &AnonRef,
    w: RwWitness<'w>,
) -> AnonGet<'w> {
    let amap: &'w Amap = &aref.amap;
    // KASSERT(rw_lock_held(anon->an_lock)) and KASSERT(anon->an_lock == amap->am_lock): `w`
    // holds the amap's lock, which the anon shares.
    kassert!(w.lock_id() == amap.lock().id());
    kassert!(anon.shares_lock(amap.lock()));

    // Increment the counters.
    counters_inc(UvmExpCounters::FltAnget);
    if anon.page().is_some() {
        ufi.orig_map.note_minflt();
    } else {
        ufi.orig_map.note_majflt();
    }
    let mut w = w;

    // Loop until we get the anon data, or fail.
    loop {
        // Note: 'we_own' (`owned`) will be the page if we set PG_BUSY on it.
        let mut owned: Option<&'static VmPage> = None;
        let mut error = VM_PAGER_OK;

        // Is page resident? Make sure it is not busy/released.
        let lock_type = w.mode(); // rw_status(anon->an_lock)
        match anon.page() {
            Some(pg) => {
                kassert!(pg.flags() & PQ_ANON != 0);
                kassert!(pg.owned_by_anon(&**anon));

                // if the page is busy, we drop all the locks and try again.
                if pg.flags() & (PG_BUSY | PG_RELEASED) == 0 {
                    return AnonGet::Resident(w);
                }
                counters_inc(UvmExpCounters::FltPgwait);

                // The last unlock must be an atomic unlock and wait on the owner of page.
                kassert!(!pg.has_object());
                uvmfault_unlockall(ufi, None, None);
                uvm_pagewait(pg, Owner::Anon(&**anon), w, "anonget");
            }
            None => {
                // No page, therefore allocate one. A write lock is required for this. If the
                // caller didn't supply one, fail now and have them retry.
                if lock_type == RwMode::Read {
                    return AnonGet::NeedWrite(w);
                }
                match uvm_pagealloc(Owner::Anon(&**anon), 0, 0) {
                    None => {
                        // Out of memory. Wait a little.
                        uvmfault_unlockall(ufi, Some(w), None);
                        counters_inc(UvmExpCounters::FltNoram);
                        uvm_wait("flt_noram1");
                    }
                    Some(pg) => {
                        // PG_BUSY bit is set.
                        owned = Some(pg);
                        uvmfault_unlockall(ufi, Some(w), None);

                        // Pass a PG_BUSY+PG_FAKE+PG_CLEAN page into the uvm_swap_get()
                        // function with all data structures unlocked. Note that it is OK to
                        // read an_swslot here, because we hold PG_BUSY on the page.
                        counters_inc(UvmExpCounters::Pageins);
                        error = uvm_swap_get(pg, anon.swslot());

                        // We clean up after the I/O below in the 'we_own' case.
                    }
                }
            }
        }

        // Re-lock the map and anon (MAPS => AMAP: the map first).
        let locked = uvmfault_relock(ufi);
        let relocked = (locked || owned.is_some()).then(|| amap.lock().enter(lock_type));

        // If we own the page (i.e. we set PG_BUSY), then we need to clean up after the I/O.
        // There are three cases to consider:
        //
        // 1) Page was released during I/O: free anon and ReFault.
        // 2) I/O not OK. Free the page and cause the fault to fail.
        // 3) I/O OK! Activate the page and sync with the non-we_own case (i.e. drop anon
        //    lock if not locked).
        let relocked = match (owned, relocked) {
            (Some(pg), Some(again)) => {
                // The page was allocated under the write lock, taken again in that mode.
                let RwWitness::Write(ww) = again else {
                    kpanic(format_args!(
                        "uvmfault_anonget: a page owned without the write lock"
                    ));
                };
                if pg.flags() & PG_WANTED != 0 {
                    wakeup(pg.chan());
                }

                // if we were RELEASED during I/O, then our anon is no longer part of an amap.
                // we need to free the anon and try again.
                if pg.flags() & PG_RELEASED != 0 {
                    kassert!(anon.refs() == 0);
                    // Released while we had unlocked amap.
                    if locked {
                        uvmfault_unlockall(ufi, None, None);
                    }
                    uvm_anon_release(anon, ww); // frees page for us
                    counters_inc(UvmExpCounters::FltPgrele);
                    return AnonGet::Unlocked(Errno::ERESTART); // refault!
                }

                if error != VM_PAGER_OK {
                    kassert!(error != VM_PAGER_PEND);

                    // Remove the swap slot from the anon and mark the anon as having no real
                    // slot. Do not free the swap slot, thus preventing it from being used
                    // again.
                    let _ = unported!("uvmfault_anonget: uvm_swap_markbad (uvm_swap.c, M7)");
                    anon.set_swslot(SWSLOT_BAD);

                    // Note: page was never !PG_BUSY, so it cannot be mapped and thus no need
                    // to pmap_page_protect() it. Freeing it removes it from the anon.
                    uvm_pagefree(pg, Owner::Anon(&**anon));

                    if locked {
                        uvmfault_unlockall(ufi, None, None);
                    }
                    drop(ww); // rw_exit(anon->an_lock)
                    // An error occurred while trying to bring in the page -- this is the only
                    // error we return right now.
                    return AnonGet::Unlocked(Errno::EACCES); // XXX
                }

                // We have successfully read the page, activate it.
                let _ = hal::pmap::clear_modify(&pg.mdpage);
                uvm_pageactivate(pg, Owner::Anon(&**anon));
                pg.clear_bits(PG_WANTED | PG_BUSY | PG_FAKE);
                // UVM_PAGE_OWN(pg, NULL): not configured.
                Some(RwWitness::Write(ww))
            }
            (_, relocked) => relocked,
        };

        // We were not able to re-lock the map - restart the fault (the anon's lock, taken
        // again when we owned the page, goes with `relocked`).
        let again = match relocked {
            Some(again) if locked => again,
            _ => return AnonGet::Unlocked(Errno::ERESTART),
        };

        // Verify that no one has touched the amap and moved the anon on us.
        let moved = amap_lookup(amap, &again, aref.pageoff, off)
            .is_none_or(|found| !AnonRef::ptr_eq(found, anon));
        if moved {
            uvmfault_unlockall(ufi, Some(again), None);
            return AnonGet::Unlocked(Errno::ERESTART);
        }

        // Retry..
        counters_inc(UvmExpCounters::FltAnretry);
        w = again;
    }
}

/// What [`uvmfault_promote`] made.
struct Promoted<'v> {
    /// The new anon, with one reference (not yet in the amap).
    anon: AnonRef,
    /// Its page, busy.
    pg: &'static VmPage,
    /// The amap's write witness, given back.
    upper: WriteWitness<'v>,
    /// The object's witness, given back.
    lower: Option<RwWitness<'v>>,
}

/// `uvmfault_promote`: promote data to a new anon. Used for 1B and 2B.
///
/// 1. allocate an anon and a page.
/// 2. fill its contents (`uobjpage`, or zeroes for `None`: the C's `PGO_DONTCARE`).
///
/// - if we fail we unlock everything (the maps, `uw` and `lower`).
/// - on success, return a new locked anon and its page, and the witnesses.
/// - it's caller's responsibility to put the promoted page to the page queue.
fn uvmfault_promote<'v, M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    aref: &'v Aref,
    uw: WriteWitness<'v>,
    lower: Option<RwWitness<'v>>,
    uobjpage: Option<&'static VmPage>,
) -> Result<Promoted<'v>, Errno> {
    let amap = &aref.amap;
    // KASSERT(rw_write_held(amap->am_lock)): `uw`, of the amap's lock.
    kassert!(uw.lock_id() == amap.lock().id());
    // KASSERT(uobj == NULL || rw_lock_held(uobj->vmobjlock)): an object's page comes with the
    // object's witness.
    kassert!(uobjpage.is_none_or(|p| !p.has_object() || lower.is_some()));

    let anon = uvm_analloc(amap.lock());
    let pg = anon.as_ref().and_then(|anon| {
        uvm_pagealloc(
            Owner::Anon(&**anon),
            0,
            if uobjpage.is_none() { UVM_PGA_ZERO } else { 0 },
        )
    });

    // check for out of RAM
    let (anon, pg) = match (anon, pg) {
        (Some(anon), Some(pg)) => (anon, pg),
        (anon, _) => {
            let had_anon = anon.is_some();
            match anon {
                None => counters_inc(UvmExpCounters::FltNoanon),
                Some(anon) => {
                    // The anon no amap saw goes under the amap's write lock it shares (the C
                    // frees it just after the unlock, its lock cleared: nothing else ever
                    // named it).
                    anon.set_refs(anon.refs() - 1);
                    uvm_anfree(&anon);
                    counters_inc(UvmExpCounters::FltNoram);
                }
            }
            uvmfault_unlockall(ufi, Some(RwWitness::Write(uw)), lower);

            if uvm_swapisfull() {
                return Err(Errno::ENOMEM);
            }

            // out of RAM, wait for more
            if had_anon {
                uvm_wait("flt_noram3");
            } else {
                uvm_anwait();
            }
            return Err(Errno::ERESTART);
        }
    };

    // copy the page (pg now dirty)
    if let Some(src) = uobjpage {
        uvm_pagecopy(src, pg);
    }

    Ok(Promoted {
        anon,
        pg,
        upper: uw,
        lower,
    })
}

/// `uvmfault_update_stats`: update statistics after fault resolution: maxrss.
fn uvmfault_update_stats<M: FaultMap>(ufi: &FaultInfo<'_, M>) {
    let map = ufi.orig_map;

    // If this is a nested pmap (eg, a virtual machine pmap managed by vmm(4) on amd64/i386),
    // don't do any updating, just return. pmap_nested() is 0 here (no vmm).

    // Update the maxrss for the process.
    if map.is_vmspace() {
        let res = usize::try_from(map.pmap().resident_count()).unwrap_or(0);
        // Convert res from pages to kilobytes.
        map.note_maxrss(res << (PAGE_SHIFT - 10));
    }
}

/*
 *   F A U L T   -   m a i n   e n t r y   p o i n t
 */

/// `uvm_fault`: page fault handler.
///
/// - called from MD code to resolve a page fault
/// - VM data structures usually should be unlocked. However, it is possible to call here with
///   the main map locked if the caller gets a write lock, sets it recursive, and then calls
///   us (c.f. `uvm_map_pageable`). This should be avoided because it keeps the map locked off
///   during I/O.
/// - MUST NEVER BE CALLED IN INTERRUPT CONTEXT
pub fn uvm_fault<M: FaultMap>(
    orig_map: &M,
    vaddr: usize,
    fault_type: VmFault,
    access_type: VmProt,
) -> Result<(), Errno> {
    counters_inc(UvmExpCounters::Faults);
    // TRACEPOINT(uvm, fault, ...): dt is not configured.

    // init the IN parameters in the ufi
    let mut ufi = FaultInfo::new(orig_map, trunc_page(vaddr), PAGE_SIZE);
    let mut flt = FaultCtx {
        enter_prot: 0,
        access_type,
        startva: 0,
        npages: 0,
        centeridx: 0,
        narrow: false, // assume normal fault for now
        wired: false,  // assume non-wired fault for now
        pa_flags: 0,
        promote: false,
        // shared lock for now (amd64, arm64)
        upper: RwMode::Read,
        lower: RwMode::Read,
    };

    loop {
        // ReFault: what this round reads of the entry, which its witnesses borrow, so none
        // outlives the round.
        let mut entry = None;
        let error = match uvm_fault_check(&mut ufi, &mut flt, &mut entry, fault_type) {
            Err(e) => Err(e),
            Ok((view, upper)) => {
                let mut pages = [PageSlot::Empty; UVM_MAXRANGE];
                // Some if there is an anon at the faulting address
                match uvm_fault_upper_lookup(&ufi, &flt, view, upper.as_ref(), &mut pages) {
                    // case 1: fault on an anon in our amap
                    Some(anon) => match upper {
                        Some(uw) => uvm_fault_upper(&mut ufi, &mut flt, view, uw, anon),
                        None => kpanic(format_args!("uvm_fault: an anon without its amap")),
                    },
                    // case 2: fault on backing obj or zero fill (no pager has a pgo_fault:
                    // see the module's deviations)
                    None => uvm_fault_lower(&mut ufi, &mut flt, view, upper, &mut pages),
                }
            }
        };
        if !matches!(error, Err(Errno::ERESTART)) {
            kassert!(ufi.guard.is_none());
            return error;
        }
    }
}

/// `uvm_fault_check`: check prot, handle needs-copy, etc.
///
/// 1. lookup entry.
/// 2. check protection.
/// 3. adjust fault condition (mainly for simulated fault).
/// 4. handle needs-copy (lazy amap copy).
/// 5. establish range of interest for neighbor fault (aka pre-fault).
/// 6. look up anons (if amap exists): the amap locked (the lookups are
///    [`uvm_fault_upper_lookup`]'s, under the same witness).
/// 7. flush pages (if MADV_SEQUENTIAL)
///
/// - called with nothing locked.
/// - if we fail we unlock everything.
/// - initialize/adjust many members of flt.
/// - on success the maps are locked (`ufi`); returns the entry as read, kept in `slot` for
///   the round, and the amap's witness, which borrows it.
fn uvm_fault_check<'v, M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    flt: &mut FaultCtx,
    slot: &'v mut Option<EntryView<M::Object>>,
    fault_type: VmFault,
) -> Result<(&'v EntryView<M::Object>, Option<RwWitness<'v>>), Errno> {
    let mut write_locked = false;

    // lookup and lock the maps
    let view = loop {
        // lookup:
        let Some(mut view) = uvmfault_lookup(ufi, write_locked) else {
            return Err(Errno::EFAULT);
        };

        #[cfg(feature = "diagnostic")]
        if !ufi.map.is_pageable() {
            kpanic(format_args!(
                "uvm_fault: fault on non-pageable map ({:p}, {:#x})",
                ufi.map, ufi.orig_rvaddr
            ));
        }

        // check protection
        if view.protection & flt.access_type != flt.access_type {
            uvmfault_unlockmaps(ufi);
            return Err(Errno::EACCES);
        }

        // "enter_prot" is the protection we want to enter the page in at. for certain pages
        // (e.g. copy-on-write pages) this protection can be more strict than
        // ufi->entry->protection. "wired" means either the entry is wired or we are
        // fault-wiring the pg.
        flt.enter_prot = view.protection;
        flt.pa_flags = if view.wc { PMAP_WC } else { 0 };
        if view.wired || fault_type == VM_FAULT_WIRE {
            flt.wired = true;
            flt.access_type = flt.enter_prot; // full access for wired
            // don't look for neighborhood pages on "wire" fault
            flt.narrow = true;
            // wiring pages requires a write lock.
            flt.upper = RwMode::Write;
            flt.lower = RwMode::Write;
        }

        // handle "needs_copy" case.
        if view.needscopy {
            if flt.access_type & PROT_WRITE != 0 || view.object.is_none() {
                // modifying `ufi->entry' requires write lock
                if !write_locked {
                    write_locked = true;
                    match ufi.take_guard().upgrade() {
                        Ok(g) => ufi.guard = Some(g),
                        Err(g) => {
                            ufi.guard = Some(g);
                            uvmfault_unlockmaps(ufi);
                            continue; // goto lookup
                        }
                    }
                }

                let g = ufi.guard();
                kassert!(g.mode() == RwMode::Write);
                let Some(e) = g.lookup(ufi.orig_rvaddr) else {
                    kpanic(format_args!(
                        "uvm_fault_check: the entry left the locked map"
                    ));
                };
                g.copy_amap(
                    e,
                    M_NOWAIT,
                    !view.stack,
                    ufi.orig_rvaddr,
                    ufi.orig_rvaddr + 1,
                );
                // The entry was copied (and perhaps clipped): read it again.
                let Some(e) = g.lookup(ufi.orig_rvaddr) else {
                    kpanic(format_args!("uvm_fault_check: amap_copy lost the entry"));
                };
                view = EntryView::of(e);

                // didn't work? must be out of RAM.
                if view.needscopy {
                    uvmfault_unlockmaps(ufi);
                    uvm_wait("fltamapcopy");
                    return Err(Errno::ERESTART);
                }

                counters_inc(UvmExpCounters::FltAmcopy);
            } else {
                // ensure that we pmap_enter page R/O since needs_copy is still true
                flt.enter_prot &= !PROT_WRITE;
            }
        }
        break view;
    };

    if write_locked {
        let g = ufi.take_guard().downgrade();
        ufi.guard = Some(g);
    }

    // identify the players: view.aref (upper layer), view.object (lower layer)

    // check for a case 0 fault. if nothing backing the entry then error now.
    if view.aref.is_none() && view.object.is_none() {
        uvmfault_unlockmaps(ufi);
        return Err(Errno::EFAULT);
    }

    // for a case 2B fault waste no time on adjacent pages because they are likely already
    // entered.
    if view.object.is_some() && view.aref.is_some() && flt.access_type & PROT_WRITE != 0 {
        // wide fault (!narrow)
        flt.narrow = true;
    }

    // establish range of interest based on advice from mapper and then clip to fit map
    // entry. note that we only want to do this the first time through the fault. if we
    // ReFault we will disable this by setting "narrow" to true.
    let nback;
    if !flt.narrow {
        // wide fault (!narrow)
        let advice = UVMADVICE[(view.advice & MADV_MASK) as usize];
        nback = advice
            .nback
            .min((ufi.orig_rvaddr - view.start) >> PAGE_SHIFT);
        flt.startva = ufi.orig_rvaddr - (nback << PAGE_SHIFT);
        let nforw = advice
            .nforw
            .min(((view.end - ufi.orig_rvaddr) >> PAGE_SHIFT) - 1);
        // note: "-1" because we don't want to count the faulting page as forw
        flt.npages = nback + nforw + 1;
        flt.centeridx = nback;

        flt.narrow = true; // ensure only once per-fault
    } else {
        // narrow fault!
        nback = 0;
        flt.startva = ufi.orig_rvaddr;
        flt.npages = 1;
        flt.centeridx = 0;
    }

    let view: &'v EntryView<M::Object> = slot.insert(view);

    // if we've got an amap then lock it (MAPS => AMAP: the maps are held) and extract current
    // anons.
    let upper = view.aref.as_ref().map(|aref| {
        if flt.access_type & PROT_WRITE != 0 {
            // assume we're about to COW.
            flt.upper = RwMode::Write;
        }
        aref.amap.lock().enter(flt.upper)
    });

    if flt.access_type & PROT_WRITE != 0 {
        // we are about to dirty the object and that requires a write lock.
        flt.lower = RwMode::Write;
    }

    // for MADV_SEQUENTIAL mappings we want to deactivate the back pages now and then forget
    // about them (for the rest of the fault).
    if view.advice == MADV_SEQUENTIAL && nback != 0 {
        // flush back-page anons?
        if let (Some(aref), Some(w)) = (&view.aref, &upper) {
            uvmfault_anonflush(aref, w, flt.startva - view.start, nback);
        }

        // flush object? change lock type to RW_WRITE, to avoid excessive competition between
        // read/write locks if many threads doing "sequential access".
        if let Some(uobj) = &view.object {
            let uoff = (flt.startva - view.start) as Voff + view.offset;
            flt.lower = RwMode::Write;
            // AMAP => UOBJ: the amap (if any) is held.
            let mut w = uobj.vmobjlock().write();
            let _ = uobj.flush(
                &mut w,
                uoff,
                uoff + (nback << PAGE_SHIFT) as Voff,
                PGO_DEACTIVATE,
            );
            drop(w);
        }

        // now forget about the backpages (the anons are looked up from startva)
        flt.startva += nback << PAGE_SHIFT;
        flt.npages -= nback;
        flt.centeridx = 0;
    }

    Ok((view, upper))
}

/// `uvm_fault_upper_upgrade`: upgrade upper lock, reader -> writer. The write witness, or
/// the reader back when another reader holds the lock too (`ERESTART`: the caller unlocks
/// everything, and the next round takes a writer lock).
fn uvm_fault_upper_upgrade<'w>(
    flt: &mut FaultCtx,
    w: RwWitness<'w>,
) -> Result<WriteWitness<'w>, RwWitness<'w>> {
    kassert!(flt.upper == w.mode());

    // fast path.
    let fast = w.mode() == RwMode::Write;

    // otherwise try for the upgrade. if we don't get it, unlock everything, restart the fault
    // and next time around get a writer lock.
    flt.upper = RwMode::Write;
    match w.try_upgrade() {
        Ok(ww) => {
            if !fast {
                counters_inc(UvmExpCounters::FltUp);
            }
            Ok(ww)
        }
        Err(w) => {
            counters_inc(UvmExpCounters::FltNoup);
            Err(w)
        }
    }
}

/// `uvm_fault_upper_lookup`: look up existing h/w mapping and amap.
///
/// Iterate range of interest:
/// 1. check if h/w mapping exists. If yes, we don't care.
/// 2. check if anon exists. If not, page is lower.
/// 3. if anon exists, enter h/w mapping for neighbors.
///
/// - called with amap locked (if exists): `upper`.
/// - returns the anon at the faulting address (the C's `shadowed`), a handle the upper fault
///   keeps across its sleeps.
fn uvm_fault_upper_lookup<M: FaultMap>(
    ufi: &FaultInfo<'_, M>,
    flt: &FaultCtx,
    view: &EntryView<M::Object>,
    upper: Option<&RwWitness<'_>>,
    pages: &mut [PageSlot; UVM_MAXRANGE],
) -> Option<AnonRef> {
    let pmap = ufi.orig_map.pmap();
    let mut entered = 0;

    kassert!(upper.is_none_or(|w| w.mode() == flt.upper));

    // the anons of the range, under the amap's witness
    let mut anons: [Option<&AnonRef>; UVM_MAXRANGE] = [None; UVM_MAXRANGE];
    if let (Some(aref), Some(w)) = (&view.aref, upper) {
        amap_lookups(
            &aref.amap,
            w,
            aref.pageoff,
            flt.startva - view.start,
            &mut anons[..flt.npages],
        );
    }

    // map in the backpages and frontpages we found in the amap in hopes of preventing future
    // faults. we also init the pages[] array as we go.
    let mut shadowed = None;
    for (lcv, (slot, anon)) in pages.iter_mut().zip(anons).enumerate().take(flt.npages) {
        let currva = flt.startva + (lcv << PAGE_SHIFT);

        // unmapped or center page. check if any anon at this level.
        let Some(anon) = anon else {
            *slot = PageSlot::Empty;
            continue;
        };

        // check for present page and map if possible.
        *slot = PageSlot::DontCare;
        if lcv == flt.centeridx {
            // save center for later!
            shadowed = Some(anon.clone());
            continue;
        }

        kassert!(
            view.aref
                .as_ref()
                .is_some_and(|a| anon.shares_lock(a.amap.lock()))
        );

        // ignore busy pages. don't play with VAs that are already mapped.
        if let Some(pg) = anon.page()
            && pg.flags() & (PG_RELEASED | PG_BUSY) == 0
            && pmap.extract(Vaddr::new(currva)).is_none()
        {
            uvm_pageactivate(pg, Owner::Anon(&**anon)); // reactivate
            counters_inc(UvmExpCounters::FltNamap);

            // No fault-ahead when wired.
            kassert!(!flt.wired);

            // Since this isn't the page that's actually faulting, ignore pmap_enter() failures;
            // it's not critical that we enter these right now.
            let prot = if anon.refs() > 1 {
                flt.enter_prot & !PROT_WRITE
            } else {
                flt.enter_prot
            };
            let _ = pmap_enter(
                pmap,
                currva,
                pa_with_flags(pg, flt.pa_flags),
                prot,
                PMAP_CANFAIL,
            );
            entered += 1;
        }
    }
    // if entered > 0: pmap_update(pmap), which has nothing to do (module notes).
    let _ = entered;

    shadowed
}

/// `uvm_fault_upper`: handle upper fault.
///
/// 1. acquire anon lock (the amap's: `uw`).
/// 2. get anon. Let [`uvmfault_anonget`] do the dirty work.
/// 3. if COW, promote data to new anon.
/// 4. enter h/w mapping.
fn uvm_fault_upper<'v, M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    flt: &mut FaultCtx,
    view: &'v EntryView<M::Object>,
    uw: RwWitness<'v>,
    anon: AnonRef,
) -> Result<(), Errno> {
    let Some(aref) = &view.aref else {
        kpanic(format_args!("uvm_fault_upper: entry without an amap"));
    };
    let amap = &aref.amap;
    let off = ufi.orig_rvaddr - view.start;
    let pmap = ufi.orig_map.pmap();

    kassert!(uw.mode() == flt.upper);
    kassert!(anon.shares_lock(amap.lock()));

    // no matter if we have case 1A or case 1B we are going to need to have the anon's memory
    // resident. ensure that now.
    //
    // let uvmfault_anonget do the dirty work. if it fails it will unlock everything for us.
    // if it succeeds, locks are still valid and locked. also, if it is OK, then the anon's
    // page is on the queues.
    let mut uw = uw;
    let uw = loop {
        // retry:
        match uvmfault_anonget(ufi, aref, off, &anon, uw) {
            AnonGet::Resident(w) => break w,
            AnonGet::Unlocked(e) => return Err(e), // ERESTART, or the I/O's error
            AnonGet::NeedWrite(w) => {
                // it needs a write lock: retry (KASSERT(rw_write_held): the type)
                match uvm_fault_upper_upgrade(flt, w) {
                    Ok(ww) => uw = RwWitness::Write(ww),
                    Err(w) => {
                        uvmfault_unlockall(ufi, Some(w), None);
                        return Err(Errno::ERESTART);
                    }
                }
            }
        }
    };

    kassert!(uw.mode() == flt.upper);
    kassert!(anon.shares_lock(amap.lock()));

    // if we are case 1B then we will need to allocate a new blank anon to transfer the data
    // into. note that we have a lock on anon, so no one can busy or release the page until we
    // are done. also note that the ref count can't drop to zero here because it is > 1 and
    // we are only dropping one ref.
    //
    // in the (hopefully very rare) case that we are out of RAM we will unlock, wait for more
    // RAM, and refault.
    //
    // if we are out of anon VM we wait for RAM to become available.
    let (anon, pg, upper) = if flt.access_type & PROT_WRITE != 0 && anon.refs() > 1 {
        // promoting requires a write lock.
        let ww = match uvm_fault_upper_upgrade(flt, uw) {
            Ok(ww) => ww,
            Err(w) => {
                uvmfault_unlockall(ufi, Some(w), None);
                return Err(Errno::ERESTART);
            }
        };

        counters_inc(UvmExpCounters::FltAcow);
        let oanon = anon; // oanon = old
        let Some(opg) = oanon.page() else {
            kpanic(format_args!(
                "uvm_fault_upper: anon without a page after anonget"
            ));
        };

        let promoted = uvmfault_promote(ufi, aref, ww, None, Some(opg))?;
        let (nanon, pg, mut ww) = (promoted.anon, promoted.pg, promoted.upper);

        // un-busy! new page
        pg.clear_bits(PG_BUSY | PG_FAKE);
        // UVM_PAGE_OWN(pg, NULL): not configured.
        let replaced = match amap_add(amap, &mut ww, aref.pageoff, off, nanon.clone(), true) {
            Ok(Some(replaced)) => replaced,
            _ => kpanic(format_args!(
                "uvm_fault_upper: amap_add could not replace the anon"
            )),
        };
        kassert!(AnonRef::ptr_eq(&replaced, &oanon));

        kassert!(SharedRwlock::same(nanon.lock(), oanon.lock()));

        // deref: can not drop to zero here by defn!
        kassert!(oanon.refs() > 1);
        oanon.set_refs(oanon.refs() - 1);
        drop(replaced); // the slot's handle; ours (oanon) goes with this function

        // note: oanon is still locked, as is the new anon (one lock, `ww`).
        kassert!(nanon.shares_lock(amap.lock()));
        kassert!(oanon.shares_lock(amap.lock()));

        // MULTIPROCESSOR && !__HAVE_PMAP_MPSAFE_ENTER_COW: both machines define the latter.
        (nanon, pg, RwWitness::Write(ww))
    } else {
        counters_inc(UvmExpCounters::FltAnon);
        let Some(pg) = anon.page() else {
            kpanic(format_args!(
                "uvm_fault_upper: anon without a page after anonget"
            ));
        };
        if anon.refs() > 1 {
            // disallow writes to ref > 1 anons
            flt.enter_prot &= !PROT_WRITE;
        }
        (anon, pg, uw)
    };

    // now map the page in .
    if pmap_enter(
        pmap,
        ufi.orig_rvaddr,
        pa_with_flags(pg, flt.pa_flags),
        flt.enter_prot,
        flt.access_type | PMAP_CANFAIL | if flt.wired { PMAP_WIRED } else { 0 },
    )
    .is_err()
    {
        // No need to undo what we did; we can simply think of this as the pmap throwing away
        // the mapping information.
        //
        // We do, however, have to go through the ReFault path, as the map may change while
        // we're asleep.
        uvmfault_unlockall(ufi, Some(upper), None);
        if uvm_swapisfull() {
            // XXX instrumentation
            return Err(Errno::ENOMEM);
        }
        // arm64's __HAVE_PMAP_POPULATE (pmap_populate) is not ported: uvm_wait on both
        // machines (see the module's deviations).
        // XXX instrumentation
        uvm_wait("flt_pmfail1");
        return Err(Errno::ERESTART);
    }

    // ... update the page queues.
    if flt.wired {
        uvm_pagewire(pg, Owner::Anon(&*anon));
    } else {
        uvm_pageactivate(pg, Owner::Anon(&*anon));
    }

    if flt.wired {
        // since the now-wired page cannot be paged out, release its swap resources for others
        // to use. since an anon with no swap cannot be PG_CLEAN, clear its clean flag now.
        pg.clear_bits(PG_CLEAN);
        uvm_anon_dropswap(&anon);
    }

    // done case 1! finish up by unlocking everything and returning success
    uvmfault_unlockall(ufi, Some(upper), None);
    // pmap_update(pmap): nothing to do (module notes).
    Ok(())
}

/// `uvm_fault_lower_lookup`: look up on-memory uobj pages.
///
/// 1. get on-memory pages.
/// 2. if failed, give up (get only center page later).
/// 3. if succeeded, enter h/w mapping of neighbor pages.
///
/// Returns the object's witness (the object stays locked, as in the C) and the centre page
/// when it was resident.
fn uvm_fault_lower_lookup<'v, M: FaultMap>(
    ufi: &FaultInfo<'_, M>,
    flt: &FaultCtx,
    view: &EntryView<M::Object>,
    uobj: &'v M::Object,
    pages: &mut [PageSlot; UVM_MAXRANGE],
) -> (RwWitness<'v>, Option<&'static VmPage>) {
    let pmap = ufi.orig_map.pmap();

    // AMAP => UOBJ: the amap (if any) is held.
    let w = uobj.vmobjlock().enter(flt.lower);

    counters_inc(UvmExpCounters::FltLget);
    let mut gotpages = flt.npages as i32;
    let _ = uobj.get(
        &w,
        view.offset + (flt.startva - view.start) as Voff,
        &mut pages[..flt.npages],
        &mut gotpages,
        flt.centeridx as i32,
        flt.access_type & view.mask(),
        view.advice,
        PGO_LOCKED,
    );

    // check for pages to map, if we got any
    if gotpages == 0 {
        return (w, None);
    }

    let mut entered = 0;
    let mut uobjpage = None;
    for (lcv, slot) in pages.iter().enumerate().take(flt.npages) {
        let currva = flt.startva + (lcv << PAGE_SHIFT);
        let PageSlot::Page(pg) = *slot else {
            continue;
        };

        kassert!(pg.flags() & PG_BUSY == 0);
        kassert!(pg.flags() & PG_RELEASED == 0);

        // if center page is resident and not PG_BUSY, then pgo_get gave us a handle to it.
        // remember this page as "uobjpage." (for later use).
        if lcv == flt.centeridx {
            uobjpage = Some(pg);
            continue;
        }

        if pmap.extract(Vaddr::new(currva)).is_some() {
            continue;
        }

        // calling pgo_get with PGO_LOCKED returns us pages which are neither busy nor
        // released, so we don't need to check for this. we can just directly enter the pages.
        uvm_pageactivate(pg, uobj.owner());
        counters_inc(UvmExpCounters::FltNomap);

        // No fault-ahead when wired.
        kassert!(!flt.wired);

        // Since this page isn't the page that's actually faulting, ignore pmap_enter()
        // failures; it's not critical that we enter these right now. NOTE: page can't be
        // PG_WANTED or PG_RELEASED because we've held the lock the whole time we've had the
        // handle.
        let _ = pmap_enter(
            pmap,
            currva,
            pa_with_flags(pg, flt.pa_flags),
            flt.enter_prot & view.mask(),
            PMAP_CANFAIL,
        );
        entered += 1;
    }
    // if entered > 0: pmap_update(pmap), which has nothing to do (module notes).
    let _ = entered;

    (w, uobjpage)
}

/// `uvm_fault_lower_upgrade`: upgrade lower lock, reader -> writer, as
/// [`uvm_fault_upper_upgrade`].
fn uvm_fault_lower_upgrade<'w>(
    flt: &mut FaultCtx,
    w: RwWitness<'w>,
) -> Result<WriteWitness<'w>, RwWitness<'w>> {
    kassert!(flt.lower == w.mode());

    // fast path.
    let fast = w.mode() == RwMode::Write;

    // otherwise try for the upgrade. if we don't get it, unlock everything, restart the fault
    // and next time around get a writer lock.
    flt.lower = RwMode::Write;
    match w.try_upgrade() {
        Ok(ww) => {
            if !fast {
                counters_inc(UvmExpCounters::FltUp);
            }
            Ok(ww)
        }
        Err(w) => {
            counters_inc(UvmExpCounters::FltNoup);
            Err(w)
        }
    }
}

/// `uvm_fault_lower`: handle lower fault.
///
/// 1. check uobj: 1.1 if null, ZFOD; 1.2 if not null, look up unmapped neighbor pages.
/// 2. for center page, check if promote: 2.1 ZFOD always needs promotion; 2.2 other uobjs,
///    when entry is marked COW (usually `MAP_PRIVATE` vnode).
/// 3. if uobj is not ZFOD and page is not found, do i/o.
/// 4. dispatch either direct / promote fault.
fn uvm_fault_lower<'v, M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    flt: &mut FaultCtx,
    view: &'v EntryView<M::Object>,
    upper: Option<RwWitness<'v>>,
    pages: &mut [PageSlot; UVM_MAXRANGE],
) -> Result<(), Errno> {
    let aref = view.aref.as_ref();
    let off = ufi.orig_rvaddr - view.start;
    let mut uobj = view.object.as_ref();
    let mut upper = upper;
    let mut lower: Option<RwWitness<'v>> = None;
    let mut dropswap = false;

    // now, if the desired page is not shadowed by the amap and we have a backing object that
    // does not have a special fault routine, then we ask (with pgo_get) the object for
    // resident pages that we care about and attempt to map them in. we do not let pgo_get
    // block (PGO_LOCKED).
    let mut uobjpage = PageSlot::Empty;
    if let Some(o) = uobj {
        let (w, pg) = uvm_fault_lower_lookup(ufi, flt, view, o, pages);
        lower = Some(w);
        if let Some(pg) = pg {
            uobjpage = PageSlot::Page(pg);
        }
    } else {
        // zero fill; don't care neighbor pages
    }

    // note that at this point we are done with any front or back pages. we are now going to
    // focus on the center page (i.e. the one we've faulted on). if we have faulted on the
    // bottom (uobj) layer [i.e. case 2] and the page was both present and available, then
    // we've got a pointer to it as "uobjpage".
    kassert!(upper.as_ref().is_none_or(|w| w.mode() == flt.upper));
    kassert!(lower.as_ref().is_none_or(|w| w.mode() == flt.lower));

    // note that uobjpage can not be PGO_DONTCARE at this point. we now set uobjpage to
    // PGO_DONTCARE if we are doing a zero fill. if we have a backing object, check and see if
    // we are going to promote the data up to an anon during the fault.
    if uobj.is_none() {
        uobjpage = PageSlot::DontCare;
        flt.promote = true; // always need anon here
    } else {
        kassert!(!matches!(uobjpage, PageSlot::DontCare));
        flt.promote = flt.access_type & PROT_WRITE != 0 && view.copyonwrite;
    }

    // if uobjpage is not null then we do not need to do I/O to get the uobjpage.
    //
    // if uobjpage is null, then we need to ask the pager to get the data for us. once we have
    // the data, we need to reverify the state the world. we are currently not holding any
    // resources.
    match (uobjpage, uobj, lower.take()) {
        (PageSlot::Empty, Some(o), Some(w)) => {
            let io = uvm_fault_lower_io(ufi, flt, view, upper.take(), o, w)?;
            uobjpage = io.page;
            uobj = io.uobj;
            upper = io.upper;
            lower = io.lower;
        }
        (PageSlot::Empty, _, _) => {
            kpanic(format_args!("uvm_fault_lower: no object to read from"));
        }
        (_, _, w) => {
            lower = w;
            // update rusage counters
            ufi.orig_map.note_minflt();
            if let (PageSlot::Page(pg), Some(o)) = (uobjpage, uobj) {
                uvm_pageactivate(pg, o.owner());
            }
        }
    }

    // notes:
    //  - at this point uobjpage can not be NULL
    //  - at this point uobjpage could be PG_WANTED (handle later)
    let pg: &'static VmPage;
    let mut anon: Option<AnonRef> = None;
    if !flt.promote {
        // we are not promoting. if the mapping is COW ensure that we don't give more access
        // than we should (e.g. when doing a read fault on a COPYONWRITE mapping we want to map
        // the COW page in R/O even though the entry protection could be R/W).
        //
        // set "pg" to the page we want to map in (uobjpage, usually)
        counters_inc(UvmExpCounters::FltObj);
        if view.copyonwrite {
            flt.enter_prot &= !PROT_WRITE;
        }
        let PageSlot::Page(page) = uobjpage else {
            kpanic(format_args!("uvm_fault_lower: no object page to map"));
        };
        pg = page; // map in the actual object

    // we are faulting directly on the page.
    } else {
        // KASSERT(amap != NULL), and the amap's witness taken with it.
        let (Some(aref), Some(uw)) = (aref, upper.take()) else {
            kpanic(format_args!("uvm_fault_lower: promoting without an amap"));
        };

        // promoting requires a write lock.
        let ww = match uvm_fault_upper_upgrade(flt, uw) {
            Ok(ww) => ww,
            Err(w) => {
                uvmfault_unlockall(ufi, Some(w), lower);
                return Err(Errno::ERESTART);
            }
        };
        kassert!(lower.as_ref().is_none_or(|w| w.mode() == flt.lower));

        // if we are going to promote the data to an anon we allocate a blank anon here and
        // plug it into our amap.
        let src = match uobjpage {
            PageSlot::Page(p) => Some(p),
            _ => None, // PGO_DONTCARE: zero fill
        };
        let promoted = uvmfault_promote(ufi, aref, ww, lower.take(), src)?;
        let mut ww = promoted.upper;
        lower = promoted.lower;
        pg = promoted.pg;
        let nanon = anon.insert(promoted.anon);

        // fill in the data
        if let Some(src) = src {
            counters_inc(UvmExpCounters::FltPrcopy);

            // promote to shared amap? make sure all sharing procs see it
            if aref.amap.flags() & AMAP_SHARED != 0 {
                hal::pmap::page_protect(&src.mdpage, PROT_NONE);
            }
            // MULTIPROCESSOR && !__HAVE_PMAP_MPSAFE_ENTER_COW: both machines define the latter.
            // done with copied uobjpage.
            drop(lower.take());
            uobj = None;
        } else {
            counters_inc(UvmExpCounters::FltPrzero);
            // Page is zero'd and marked dirty by uvm_pagealloc(), called in
            // uvmfault_promote() above.
        }

        if amap_add(&aref.amap, &mut ww, aref.pageoff, off, nanon.clone(), false).is_err() {
            if pg.flags() & PG_WANTED != 0 {
                wakeup(pg.chan());
            }

            pg.clear_bits(PG_BUSY | PG_FAKE | PG_WANTED);
            // UVM_PAGE_OWN(pg, NULL): not configured.
            // The object and the maps go first; the anon no amap saw is freed under the
            // amap's write witness (its reference dropped: see the module's deviations), then
            // the amap is unlocked: everything is, as the C's uvmfault_unlockall.
            drop(lower.take());
            uvmfault_unlockmaps(ufi);
            nanon.set_refs(nanon.refs() - 1);
            uvm_anfree(nanon);
            drop(anon);
            drop(ww);
            counters_inc(UvmExpCounters::FltNoamap);

            if uvm_swapisfull() {
                return Err(Errno::ENOMEM);
            }

            uvmfault_populate(&aref.amap, aref.pageoff, off);
            return Err(Errno::ERESTART);
        }
        upper = Some(RwWitness::Write(ww));
    }

    // anon must be write locked (promotion). uobj can be either.
    //
    // Note: pg is either the uobjpage or the new page in the new anon.
    kassert!(upper.as_ref().is_none_or(|w| w.mode() == flt.upper));
    kassert!(lower.as_ref().is_none_or(|w| w.mode() == flt.lower));
    kassert!(
        anon.as_ref()
            .is_none_or(|a| aref.is_some_and(|r| a.shares_lock(r.amap.lock())))
    );

    // all resources are present. we can now map it in and free our resources.
    let pmap = ufi.orig_map.pmap();
    if pmap_enter(
        pmap,
        ufi.orig_rvaddr,
        pa_with_flags(pg, flt.pa_flags),
        flt.enter_prot,
        flt.access_type | PMAP_CANFAIL | if flt.wired { PMAP_WIRED } else { 0 },
    )
    .is_err()
    {
        // No need to undo what we did; we can simply think of this as the pmap throwing away
        // the mapping information.
        //
        // We do, however, have to go through the ReFault path, as the map may change while
        // we're asleep.
        if pg.flags() & PG_WANTED != 0 {
            wakeup(pg.chan());
        }

        pg.clear_bits(PG_BUSY | PG_FAKE | PG_WANTED);
        // UVM_PAGE_OWN(pg, NULL): not configured.
        uvmfault_unlockall(ufi, upper, lower);
        if uvm_swapisfull() {
            // XXX instrumentation
            return Err(Errno::ENOMEM);
        }
        // arm64's __HAVE_PMAP_POPULATE (pmap_populate) is not ported: uvm_wait on both
        // machines (see the module's deviations).
        // XXX instrumentation
        uvm_wait("flt_pmfail2");
        return Err(Errno::ERESTART);
    }

    // The page's owner: the new anon, or the object.
    let owner = match (&anon, uobj) {
        (Some(a), _) => Owner::Anon(&**a),
        (None, Some(o)) => o.owner(),
        (None, None) => kpanic(format_args!("uvm_fault_lower: a page without an owner")),
    };
    if flt.wired {
        uvm_pagewire(pg, owner);
        if pg.flags() & PQ_AOBJ != 0 {
            // since the now-wired page cannot be paged out, release its swap resources for
            // others to use. since an aobj page with no swap cannot be clean, mark it dirty
            // now.
            //
            // use pg->uobject here (the page's object is the one asked: module notes).
            kassert!(uobj.is_some());
            kassert!(uobj.is_some_and(|o| pg.owner_tag() == o.owner().tag()));
            pg.clear_bits(PG_CLEAN);
            dropswap = true;
        }
    } else {
        uvm_pageactivate(pg, owner);
    }

    if dropswap {
        let (Some(o), Some(w)) = (uobj, lower.as_mut().and_then(RwWitness::as_write)) else {
            kpanic(format_args!(
                "uvm_fault_lower: a wired aobj page without its write lock"
            ));
        };
        o.dropswap(w, (pg.offset() >> PAGE_SHIFT) as i32);
    }

    if pg.flags() & PG_WANTED != 0 {
        wakeup(pg.chan());
    }

    pg.clear_bits(PG_BUSY | PG_FAKE | PG_WANTED);
    // UVM_PAGE_OWN(pg, NULL): not configured.
    uvmfault_unlockall(ufi, upper, lower);
    // pmap_update(pmap): nothing to do (module notes).

    Ok(())
}

/// What [`uvm_fault_lower_io`] brought back, everything locked again.
struct LowerIo<'v, O> {
    /// The object page (busy no more), or `DontCare` when a `UVM_ET_NOFAULT` entry's read
    /// failed (the caller promotes a zero page).
    page: PageSlot,
    /// The page's object (`None` with `DontCare`).
    uobj: Option<&'v O>,
    /// The amap's witness, taken again.
    upper: Option<RwWitness<'v>>,
    /// The object's witness, taken again for the page.
    lower: Option<RwWitness<'v>>,
}

/// `uvm_fault_lower_io`: get lower page from backing store.
///
/// 1. unlock everything, because i/o will block.
/// 2. call pgo_get.
/// 3. if failed, recover.
/// 4. if succeeded, relock everything and verify things.
fn uvm_fault_lower_io<'v, M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    flt: &mut FaultCtx,
    view: &'v EntryView<M::Object>,
    upper: Option<RwWitness<'v>>,
    uobj: &'v M::Object,
    lower: RwWitness<'v>,
) -> Result<LowerIo<'v, M::Object>, Errno> {
    let aref = view.aref.as_ref();

    // grab everything we need from the entry before we unlock
    let uoff = (ufi.orig_rvaddr - view.start) as Voff + view.offset;
    let access_type = flt.access_type & view.mask();
    let advice = view.advice;

    // Upgrade to a write lock if needed.
    let lw = match uvm_fault_lower_upgrade(flt, lower) {
        Ok(lw) => lw,
        Err(w) => {
            uvmfault_unlockall(ufi, upper, Some(w));
            return Err(Errno::ERESTART);
        }
    };
    uvmfault_unlockall(ufi, upper, None);

    // update rusage counters
    ufi.orig_map.note_majflt();

    // KASSERT(rw_write_held(uobj->vmobjlock)): `lw`.
    counters_inc(UvmExpCounters::FltGet);
    let mut gotpages = 1;
    let mut pgs = [PageSlot::Empty];
    // The pager returns without the object's lock (`lw` is consumed).
    let result = uobj.get_sync(
        lw,
        uoff,
        &mut pgs,
        &mut gotpages,
        0,
        access_type,
        advice,
        PGO_SYNCIO,
    );
    let mut pg = pgs[0];
    let mut uobj = Some(uobj);

    // recover from I/O
    if result != VM_PAGER_OK {
        kassert!(result != VM_PAGER_PEND);

        if result == VM_PAGER_AGAIN {
            let _ = tsleep_nsec(ptr::from_ref(&NOWAKE), PVM, "fltagain2", FLTAGAIN_NSEC);
            return Err(Errno::ERESTART);
        }

        if !view.nofault {
            return Err(Errno::EIO);
        }

        pg = PageSlot::DontCare;
        uobj = None;
        flt.promote = true;
    } else if !matches!(pg, PageSlot::Page(_)) {
        kpanic(format_args!(
            "uvm_fault_lower_io: the pager returned no page"
        ));
    }

    // re-verify the state of the world (MAPS => AMAP => UOBJ).
    let mut locked = uvmfault_relock(ufi);
    let mut upper = None;
    if locked && let Some(aref) = aref {
        upper = Some(aref.amap.lock().enter(flt.upper));
    }

    // might be changed: the page's object is the one asked (module notes)
    let mut lower = None;
    if let (PageSlot::Page(page), Some(o)) = (pg, uobj) {
        if page.owner_tag() != o.owner().tag() {
            kpanic(format_args!(
                "uvm_fault_lower_io: the pager returned another object's page"
            ));
        }
        lower = Some(o.vmobjlock().enter(flt.lower));
        kassert!(page.flags() & PG_BUSY != 0);
        kassert!(flt.lower == RwMode::Write);
    }

    // Re-verify that amap slot is still free. if there is a problem, we clean up.
    let taken = match (locked, aref, &upper) {
        (true, Some(aref), Some(w)) => {
            amap_lookup(&aref.amap, w, aref.pageoff, ufi.orig_rvaddr - view.start).is_some()
        }
        _ => false,
    };
    if taken {
        uvmfault_unlockall(ufi, upper.take(), None);
        locked = false;
    }

    // release the page now, still holding object lock
    if let (PageSlot::Page(page), Some(o)) = (pg, uobj) {
        uvm_pageactivate(page, o.owner());

        if page.flags() & PG_WANTED != 0 {
            wakeup(page.chan());
        }
        page.clear_bits(PG_BUSY | PG_WANTED);
        // UVM_PAGE_OWN(pg, NULL): not configured.
    }

    if !locked {
        drop(lower); // the object's lock, if the page took it
        return Err(Errno::ERESTART);
    }

    // we have the data in pg. we are holding object lock (so the page can't be released on
    // us).
    Ok(LowerIo {
        page: pg,
        uobj,
        upper,
        lower,
    })
}

/// `uvm_fault_wire`: wire down a range of virtual addresses in a map.
///
/// - map may be read-locked by caller, but MUST NOT be write-locked.
/// - if map is read-locked, any operations which may cause map to be write-locked in
///   [`uvm_fault`] must be taken care of by the caller. See `uvm_map_pageable()`.
pub fn uvm_fault_wire<M: FaultMap>(
    map: &M,
    start: usize,
    end: usize,
    access_type: VmProt,
) -> Result<(), Errno> {
    // now fault it in a page at a time. if the fault fails then we have to undo what we have
    // done. note that in uvm_fault PROT_NONE is replaced with the max protection if
    // fault_type is VM_FAULT_WIRE.
    let mut va = start;
    while va < end {
        if let Err(rv) = uvm_fault(map, va, VM_FAULT_WIRE, access_type) {
            if va != start {
                uvm_fault_unwire(map, start, va);
            }
            return Err(rv);
        }
        va += PAGE_SIZE;
    }

    Ok(())
}

/// `uvm_fault_unwire`: unwire range of virtual space.
pub fn uvm_fault_unwire<M: FaultMap>(map: &M, start: usize, end: usize) {
    let guard = map.lock(RwMode::Read);
    uvm_fault_unwire_locked(&guard, start, end);
    drop(guard);
}

/// `uvm_map_lock_entry(entry)` / `uvm_map_unlock_entry(entry)`: an entry's amap and object
/// locked for writing (MAPS => AMAP => UOBJ: the caller holds the map); the drop releases the
/// object's lock, then the amap's (the fields drop in this order).
pub struct EntryLocks<'a> {
    /// The object's (`rw_enter(uobj->vmobjlock, RW_WRITE)`).
    lower: Option<WriteWitness<'a>>,
    /// The amap's (`amap_lock(amap, RW_WRITE)`).
    upper: Option<WriteWitness<'a>>,
}

impl<'a> EntryLocks<'a> {
    /// `uvm_map_lock_entry`: the amap's lock first, then the object's.
    pub fn new(upper: Option<&'a SharedRwlock>, lower: Option<&'a SharedRwlock>) -> Self {
        let upper = upper.map(SharedRwlock::write);
        let lower = lower.map(SharedRwlock::write);
        Self { lower, upper }
    }

    /// The amap's write witness, if the entry has an amap.
    pub fn upper(&self) -> Option<&WriteWitness<'a>> {
        self.upper.as_ref()
    }

    /// The object's write witness, if the entry maps an object.
    pub fn lower(&self) -> Option<&WriteWitness<'a>> {
        self.lower.as_ref()
    }

    /// The object's write witness, to sleep on or to change the object's pages with (the
    /// kernel object's `uvm_km_pgremove` under `uvm_map_lock_entry`).
    pub fn lower_mut(&mut self) -> Option<&mut WriteWitness<'a>> {
        self.lower.as_mut()
    }
}

/// The owner of `pg`, mapped `off` bytes into an entry with `aref` (whose amap `locks` holds)
/// and `uobj`: the anon at that offset when `pg` is its page, else the object when `pg` is one
/// of its pages, else none (`uvm_pageunwire` asserts the owner is the page's and locked; the
/// C reads `pg->uanon`/`pg->uobject`). An entry without an amap is ordinary (a shared file
/// mapping, a private one only read so far): the amap is looked up only when there is one.
fn entry_page_owner<'a, O: FaultObject>(
    aref: Option<&'a Aref>,
    locks: &'a EntryLocks<'_>,
    uobj: Option<&'a O>,
    off: usize,
    pg: &VmPage,
) -> Owner<'a> {
    if let (Some(aref), Some(w)) = (aref, locks.upper())
        && let Some(anon) = amap_lookup(&aref.amap, w, aref.pageoff, off)
        && pg.owned_by_anon(&**anon)
    {
        return Owner::Anon(&**anon);
    }
    match uobj {
        Some(o) if pg.owner_tag() == o.owner().tag() => o.owner(),
        _ => Owner::None,
    }
}

/// `uvm_fault_unwire_locked`: the guts of [`uvm_fault_unwire`].
///
/// - map must be at least read-locked: `guard`, the map's lock, which names the map (the C's
///   `vm_map_assert_anylock(map)` is the type: no guard of another map can be passed).
pub fn uvm_fault_unwire_locked<G>(guard: &G, start: usize, end: usize)
where
    G: FaultMapGuard,
    G::Map: FaultMap<Entry = G::Entry>,
    G::Entry: FaultEntry<Object = <G::Map as FaultMap>::Object>,
{
    let map = guard.map();
    let pmap = map.pmap();

    kassert!(!map.is_intrsafe());

    // we assume that the area we are unwiring has actually been wired in the first place.
    // this means that we should be able to extract the PAs from the pmap.

    // find the beginning map entry for the region.
    kassert!(start >= map.min_offset() && end <= map.max_offset());
    let Some(mut entry) = guard.lookup(start) else {
        kpanic(format_args!("uvm_fault_unwire_locked: address not in map"));
    };

    let mut va = start;
    while va < end {
        // find the map entry for the current address.
        kassert!(va >= entry.start());
        while va >= entry.end() {
            let next = guard.next(entry);
            kassert!(next.is_some_and(|n| n.start() <= entry.end()));
            let Some(next) = next else {
                kpanic(format_args!("uvm_fault_unwire_locked: ran off the map"));
            };
            entry = next;
        }

        // lock it (the entry's addresses up to `end`, then it is unlocked: the C unlocks the
        // previous entry when the walk reaches the next one, and the last at the end).
        let aref = entry.aref().map(|(amap, pageoff)| Aref { amap, pageoff });
        let uobj = entry.object();
        let locks = EntryLocks::new(
            aref.as_ref().map(|a| a.amap.lock()),
            uobj.as_ref().map(|o| o.vmobjlock()),
        );
        let wired = entry.is_wired();
        let estart = entry.start();
        let seg_end = end.min(entry.end());
        while va < seg_end {
            if let Some(pa) = pmap.extract(Vaddr::new(va)) {
                // if the entry is no longer wired, tell the pmap.
                if !wired {
                    pmap.unwire(Vaddr::new(va));
                }

                if let Some(pg) = PHYS_TO_VM_PAGE(pa) {
                    // Owner::None only for a page neither the entry's anon at this address nor
                    // its object owns, which a wired mapping of the entry cannot be:
                    // uvm_pageunwire's owner assertion then fails under DIAGNOSTIC, as the C's
                    // KASSERT(uvm_page_owner_locked_p(pg, TRUE)) on such a page.
                    let owner =
                        entry_page_owner(aref.as_ref(), &locks, uobj.as_ref(), va - estart, pg);
                    uvm_pageunwire(pg, owner);
                }
            }
            va += PAGE_SIZE;
        }
        drop(locks);
    }
}

/// `uvmfault_unlockmaps`: unlock the maps (the guard knows how it was locked).
fn uvmfault_unlockmaps<M: FaultMap>(ufi: &mut FaultInfo<'_, M>) {
    uvmfault_update_stats(ufi);
    drop(ufi.take_guard());
}

/// `uvmfault_unlockall`: unlock everything passed in: the object's witness, then the amap's,
/// then the maps.
///
/// - maps must be read-locked (not write-locked).
fn uvmfault_unlockall<M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    upper: Option<RwWitness<'_>>,
    lower: Option<RwWitness<'_>>,
) {
    kassert!(ufi.guard().mode() == RwMode::Read);
    drop(lower);
    drop(upper);
    uvmfault_unlockmaps(ufi);
}

/// `uvmfault_lookup`: lookup a virtual address in a map.
///
/// - caller must provide a `uvm_faultinfo` structure with the IN params properly filled in
/// - we will lookup the map entry (handling submaps) as we go
/// - if the lookup is a success we will return with the maps locked, and the entry as read
/// - if "write_lock" is true, we write_lock the map, otherwise we only get a read lock.
/// - note that submaps can only appear in the kernel and they are required to use the same
///   virtual addresses as the map they are referenced by (thus address translation between
///   the main map and the submap is unnecessary).
fn uvmfault_lookup<M: FaultMap>(
    ufi: &mut FaultInfo<'_, M>,
    write_lock: bool,
) -> Option<EntryView<M::Object>> {
    // init ufi values for lookup.
    ufi.map = ufi.orig_map;
    ufi.size = ufi.orig_size;

    // keep going down levels until we are done. note that there can only be two levels so we
    // won't loop very long.
    loop {
        let map = ufi.map;
        if ufi.orig_rvaddr < map.min_offset() || ufi.orig_rvaddr >= map.max_offset() {
            return None;
        }

        // lock map
        let guard = map.lock(if write_lock {
            RwMode::Write
        } else {
            RwMode::Read
        });

        // lookup
        let found = guard.lookup(ufi.orig_rvaddr).map(|entry| {
            // reduce size if necessary
            if entry.end() - ufi.orig_rvaddr < ufi.size {
                ufi.size = entry.end() - ufi.orig_rvaddr;
            }
            if entry.is_submap() {
                Err(entry.submap())
            } else {
                Ok(EntryView::of(entry))
            }
        });
        let version = guard.version();
        ufi.guard = Some(guard);

        match found {
            None => {
                uvmfault_unlockmaps(ufi);
                return None;
            }
            // submap? replace map with the submap and lookup again. note: VAs in submaps must
            // match VAs in main map.
            Some(Err(submap)) => {
                let Some(tmpmap) = submap else {
                    kpanic(format_args!("uvmfault_lookup: submap entry without a map"));
                };
                uvmfault_unlockmaps(ufi);
                ufi.map = tmpmap;
            }
            // got it!
            Some(Ok(view)) => {
                ufi.version = version;
                return Some(view);
            }
        }
    }
}

/// `uvmfault_relock`: attempt to relock the same version of the map.
///
/// - fault data structures should be unlocked before calling.
/// - if a success (true) maps will be locked after call.
fn uvmfault_relock<M: FaultMap>(ufi: &mut FaultInfo<'_, M>) -> bool {
    kassert!(ufi.guard.is_none());

    // relock map. fail if version mismatch (in which case nothing gets locked).
    match ufi.map.relock(ufi.version) {
        None => {
            counters_inc(UvmExpCounters::FltNorelck);
            false
        }
        Some(guard) => {
            kassert!(guard.mode() == RwMode::Read);
            ufi.guard = Some(guard);
            counters_inc(UvmExpCounters::FltRelck);
            true // got it!
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The fault handler over doubles of its two bridges: `FakeMap` (a map of a few entries
    // behind a hal `Rwlocked`, with a version every write lock bumps) and `FakeObj` (an object
    // with a real lock and page tree whose pager is scripted). The anons, the amaps and the
    // pages are the kernel's own, over the page system of `testutil::pages_install`.

    use std::boxed::Box;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::sync::MutexGuard;
    use std::sync::atomic::{AtomicI32, AtomicUsize};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};
    use std::vec;
    use std::vec::Vec;

    use hal::rwlock::{RW_READ, RW_WRITE, ReadGuard, Rwlocked, WriteGuard};
    use hal::sys::mman::{PROT_EXEC, PROT_READ};
    use hal::sys::types::Vsize;
    use hal::tree::Tree;
    use hal::vm::uvmexp::counters_read;

    use super::*;
    use crate::testutil;
    use crate::vm::amap::{
        AMAP_REFALL, CopyOutcome, amap_alloc, amap_copy_from, amap_init, amap_ref, amap_unadd,
        amap_unref, amaps_out,
    };
    use crate::vm::anon::{anons_out, uvm_anon_init};
    use crate::vm::aobj::{uao_create, uao_find_swslot, uao_init, uao_set_swslot};
    use crate::vm::page::{
        OwnerTag, PQ_ACTIVE, PQ_INACTIVE, PageObject, UvmObjtree, uvm_pagelookup,
    };
    use crate::vm::vnode::{uvn_attach, uvn_init};

    const RW: VmProt = PROT_READ | PROT_WRITE;
    /// The double map's range.
    const VA_MIN: usize = 0x1000_0000;
    const VA_MAX: usize = 0x8000_0000;
    /// Where the tests map.
    const VA: usize = 0x2000_0000;

    /// Real page memory, the lock objects, the anon and amap pools and the pmap pool.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(256 * 4096);
        let _ = hal::rwlock::init_shared();
        let _ = uvm_anon_init(0);
        let _ = amap_init(0);
        let _ = uao_init();
        hal::pmap::init();
        no_swap_get();
        guard
    }

    fn count(c: UvmExpCounters) -> u64 {
        counters_read(c)
    }

    /*
     * The swap pager's stand-in (uvm_swap_get, N3.5): what the test thread says it does.
     */

    type SwapGet = Box<dyn FnMut(&'static VmPage, i32) -> i32>;

    std::thread_local! {
        static SWAP_GET: RefCell<Option<SwapGet>> = const { RefCell::new(None) };
    }

    /// The answer of the test thread's stand-in, if it set one.
    pub(super) fn swap_get_hook(pg: &'static VmPage, swslot: i32) -> Option<i32> {
        SWAP_GET.with(|h| h.borrow_mut().as_mut().map(|f| f(pg, swslot)))
    }

    fn on_swap_get(f: impl FnMut(&'static VmPage, i32) -> i32 + 'static) {
        SWAP_GET.with(|h| *h.borrow_mut() = Some(Box::new(f)));
    }

    fn no_swap_get() {
        SWAP_GET.with(|h| *h.borrow_mut() = None);
    }

    /*
     * The object double.
     */

    struct ObjInner {
        lock: SharedRwlock,
        memt: Tree<UvmObjtree>,
        npages: AtomicI32,
        /// What the next `get_sync`s answer instead of reading (front first).
        script: RefCell<VecDeque<i32>>,
        /// Run by `get_sync` once the object is unlocked, as another process might.
        meanwhile: RefCell<Option<Box<dyn FnMut()>>>,
        gets: AtomicUsize,
        sync_gets: AtomicUsize,
        flushes: RefCell<Vec<(Voff, Voff, i32)>>,
        dropswaps: RefCell<Vec<i32>>,
    }

    impl PageObject for ObjInner {
        fn tag(&self) -> OwnerTag {
            OwnerTag::object(self)
        }
        fn is_dummy(&self) -> bool {
            false
        }
        fn is_kern_object(&self) -> bool {
            false
        }
        fn locked(&self, exclusive: bool) -> bool {
            let s = self.lock.status();
            s == RW_WRITE || (!exclusive && s == RW_READ)
        }
        fn pages(&self) -> &Tree<UvmObjtree> {
            &self.memt
        }
        fn npages(&self) -> i32 {
            self.npages.load(Ordering::Relaxed)
        }
        fn set_npages(&self, n: i32) {
            self.npages.store(n, Ordering::Relaxed);
        }
    }

    #[derive(Clone)]
    struct FakeObj(&'static ObjInner);

    fn obj() -> FakeObj {
        FakeObj(Box::leak(Box::new(ObjInner {
            lock: SharedRwlock::new("uobjlk").expect("a lock object"),
            memt: Tree::new(),
            npages: AtomicI32::new(0),
            script: RefCell::new(VecDeque::new()),
            meanwhile: RefCell::new(None),
            gets: AtomicUsize::new(0),
            sync_gets: AtomicUsize::new(0),
            flushes: RefCell::new(Vec::new()),
            dropswaps: RefCell::new(Vec::new()),
        })))
    }

    impl FakeObj {
        fn gets(&self) -> usize {
            self.0.gets.load(Ordering::Relaxed)
        }
        fn sync_gets(&self) -> usize {
            self.0.sync_gets.load(Ordering::Relaxed)
        }
        fn page(&self, off: usize) -> Option<&'static VmPage> {
            let _w = self.0.lock.read();
            uvm_pagelookup(self.0, off as Voff)
        }
    }

    impl FaultObject for FakeObj {
        fn vmobjlock(&self) -> &SharedRwlock {
            &self.0.lock
        }

        fn owner(&self) -> Owner<'_> {
            Owner::Object(self.0)
        }

        fn get(
            &self,
            w: &RwWitness<'_>,
            off: Voff,
            pps: &mut [PageSlot],
            npages: &mut i32,
            _centeridx: i32,
            _access_type: VmProt,
            _advice: i32,
            flags: i32,
        ) -> i32 {
            assert_eq!(w.lock_id(), self.0.lock.id());
            assert_eq!(flags, PGO_LOCKED);
            self.0.gets.fetch_add(1, Ordering::Relaxed);
            let mut got = 0;
            for (i, slot) in pps.iter_mut().enumerate() {
                if matches!(slot, PageSlot::DontCare) {
                    continue;
                }
                *slot = match uvm_pagelookup(self.0, off + (i * PAGE_SIZE) as Voff) {
                    Some(pg) if pg.flags() & (PG_BUSY | PG_RELEASED) == 0 => {
                        got += 1;
                        PageSlot::Page(pg)
                    }
                    _ => PageSlot::Empty,
                };
            }
            *npages = got;
            VM_PAGER_OK
        }

        fn get_sync(
            &self,
            w: WriteWitness<'_>,
            off: Voff,
            pps: &mut [PageSlot],
            npages: &mut i32,
            _centeridx: i32,
            _access_type: VmProt,
            _advice: i32,
            flags: i32,
        ) -> i32 {
            assert_eq!(w.lock_id(), self.0.lock.id());
            assert_eq!(flags, PGO_SYNCIO);
            assert_eq!(self.0.lock.status(), RW_WRITE);
            self.0.sync_gets.fetch_add(1, Ordering::Relaxed);
            let scripted = self.0.script.borrow_mut().pop_front();
            if let Some(r) = scripted.filter(|&r| r != VM_PAGER_OK) {
                drop(w);
                return r;
            }
            let pg = match uvm_pagelookup(self.0, off) {
                Some(pg) => {
                    pg.set_bits(PG_BUSY);
                    pg
                }
                None => {
                    let pg = uvm_pagealloc(Owner::Object(self.0), off, 0).expect("a page");
                    pg.clear_bits(PG_FAKE); // read in
                    pg
                }
            };
            pps[0] = PageSlot::Page(pg);
            *npages = 1;
            drop(w); // the pager returns unlocked
            if let Some(f) = self.0.meanwhile.borrow_mut().as_mut() {
                f();
            }
            VM_PAGER_OK
        }

        fn flush(&self, w: &mut WriteWitness<'_>, start: Voff, stop: Voff, flags: i32) -> bool {
            assert_eq!(w.lock_id(), self.0.lock.id());
            self.0.flushes.borrow_mut().push((start, stop, flags));
            true
        }

        fn dropswap(&self, w: &mut WriteWitness<'_>, pageidx: i32) {
            assert_eq!(w.lock_id(), self.0.lock.id());
            self.0.dropswaps.borrow_mut().push(pageidx);
        }
    }

    /// What the double map's entries map: the pager double or a real object (an aobj).
    #[derive(Clone)]
    enum TestObj {
        Fake(FakeObj),
        Real(ObjRef),
    }

    impl From<FakeObj> for TestObj {
        fn from(o: FakeObj) -> Self {
            TestObj::Fake(o)
        }
    }

    impl From<ObjRef> for TestObj {
        fn from(o: ObjRef) -> Self {
            TestObj::Real(o)
        }
    }

    impl FaultObject for TestObj {
        fn vmobjlock(&self) -> &SharedRwlock {
            match self {
                TestObj::Fake(o) => o.vmobjlock(),
                TestObj::Real(o) => FaultObject::vmobjlock(o),
            }
        }
        fn owner(&self) -> Owner<'_> {
            match self {
                TestObj::Fake(o) => o.owner(),
                TestObj::Real(o) => FaultObject::owner(o),
            }
        }
        fn get(
            &self,
            w: &RwWitness<'_>,
            off: Voff,
            pps: &mut [PageSlot],
            npages: &mut i32,
            centeridx: i32,
            access_type: VmProt,
            advice: i32,
            flags: i32,
        ) -> i32 {
            match self {
                TestObj::Fake(o) => {
                    o.get(w, off, pps, npages, centeridx, access_type, advice, flags)
                }
                TestObj::Real(o) => FaultObject::get(
                    o,
                    w,
                    off,
                    pps,
                    npages,
                    centeridx,
                    access_type,
                    advice,
                    flags,
                ),
            }
        }
        fn get_sync(
            &self,
            w: WriteWitness<'_>,
            off: Voff,
            pps: &mut [PageSlot],
            npages: &mut i32,
            centeridx: i32,
            access_type: VmProt,
            advice: i32,
            flags: i32,
        ) -> i32 {
            match self {
                TestObj::Fake(o) => {
                    o.get_sync(w, off, pps, npages, centeridx, access_type, advice, flags)
                }
                TestObj::Real(o) => FaultObject::get_sync(
                    o,
                    w,
                    off,
                    pps,
                    npages,
                    centeridx,
                    access_type,
                    advice,
                    flags,
                ),
            }
        }
        fn flush(&self, w: &mut WriteWitness<'_>, start: Voff, stop: Voff, flags: i32) -> bool {
            match self {
                TestObj::Fake(o) => o.flush(w, start, stop, flags),
                TestObj::Real(o) => FaultObject::flush(o, w, start, stop, flags),
            }
        }
        fn dropswap(&self, w: &mut WriteWitness<'_>, pageidx: i32) {
            match self {
                TestObj::Fake(o) => o.dropswap(w, pageidx),
                TestObj::Real(o) => FaultObject::dropswap(o, w, pageidx),
            }
        }
    }

    /*
     * The map double.
     */

    struct FakeEntry {
        start: usize,
        end: usize,
        offset: Voff,
        protection: VmProt,
        advice: i32,
        wired: bool,
        cow: bool,
        needscopy: Cell<bool>,
        nofault: bool,
        aref: RefCell<Option<(AmapRef, i32)>>,
        object: Option<TestObj>,
        submap: Option<&'static FakeMap>,
    }

    impl FakeEntry {
        /// Read-write memory of `npages` pages at `start`, backed by nothing yet.
        fn new(start: usize, npages: usize) -> Self {
            Self {
                start,
                end: start + npages * PAGE_SIZE,
                offset: 0,
                protection: RW,
                advice: MADV_NORMAL,
                wired: false,
                cow: false,
                needscopy: Cell::new(false),
                nofault: false,
                aref: RefCell::new(None),
                object: None,
                submap: None,
            }
        }

        /// Private anonymous memory: copy-on-write, its amap made by the first fault.
        fn anon(start: usize, npages: usize) -> Self {
            let e = Self::new(start, npages);
            e.needscopy.set(true);
            Self { cow: true, ..e }
        }

        /// Anonymous memory over `amap`, whose copy is done.
        fn with_amap(start: usize, npages: usize, amap: &AmapRef) -> Self {
            let e = Self::new(start, npages);
            *e.aref.borrow_mut() = Some((amap.clone(), 0));
            e
        }

        /// A shared mapping of `obj` (the double or a real object) from offset 0.
        fn with_object<T: Clone + Into<TestObj>>(start: usize, npages: usize, obj: &T) -> Self {
            Self {
                object: Some(obj.clone().into()),
                ..Self::new(start, npages)
            }
        }
    }

    struct FakeState {
        entries: Vec<FakeEntry>,
        timestamp: u32,
    }

    struct FakeMap {
        state: Rwlocked<FakeState>,
        pmap: PmapRef,
        minflt: AtomicUsize,
        majflt: AtomicUsize,
        maxrss: AtomicUsize,
    }

    /// A map of `entries` (in address order) over a new user pmap, leaked (its pmap keeps the
    /// mappings the test made).
    fn map_of(entries: Vec<FakeEntry>) -> &'static FakeMap {
        Box::leak(Box::new(FakeMap {
            state: Rwlocked::new(
                "fakemap",
                FakeState {
                    entries,
                    timestamp: 0,
                },
            ),
            pmap: hal::pmap::create().expect("a pmap"),
            minflt: AtomicUsize::new(0),
            majflt: AtomicUsize::new(0),
            maxrss: AtomicUsize::new(0),
        }))
    }

    impl FakeMap {
        /// Another process changes the map (a write lock).
        fn bump(&self) {
            self.state.write().timestamp += 1;
        }

        /// No lock of the map is held.
        fn is_free(&self) -> bool {
            self.state.try_write().is_some()
        }

        fn pa(&self, va: usize) -> Option<Paddr> {
            self.pmap.extract(Vaddr::new(va))
        }

        /// The anon at `va`, as a handle.
        fn anon(&self, va: usize) -> Option<AnonRef> {
            let g = self.lock(RwMode::Read);
            let e = g.lookup(va)?;
            let (amap, pageoff) = e.aref()?;
            let start = e.start;
            let w = amap.lock().read();
            let anon = amap_lookup(&amap, &w, pageoff, va - start).cloned();
            drop(w);
            anon
        }

        fn amap(&self, va: usize) -> Option<AmapRef> {
            let g = self.lock(RwMode::Read);
            g.lookup(va)?.aref().map(|(a, _)| a)
        }
    }

    /// The double map locked: the map and its lock's guard.
    enum FakeGuard<'a> {
        Read(&'a FakeMap, ReadGuard<'a, FakeState>),
        Write(&'a FakeMap, WriteGuard<'a, FakeState>),
    }

    impl FakeGuard<'_> {
        fn state(&self) -> &FakeState {
            match self {
                FakeGuard::Read(_, g) => g,
                FakeGuard::Write(_, g) => g,
            }
        }
    }

    impl FaultMap for FakeMap {
        type Object = TestObj;
        type Entry = FakeEntry;
        type Guard<'a> = FakeGuard<'a>;

        fn min_offset(&self) -> usize {
            VA_MIN
        }
        fn max_offset(&self) -> usize {
            VA_MAX
        }
        fn lock(&self, mode: RwMode) -> FakeGuard<'_> {
            match mode {
                RwMode::Read => FakeGuard::Read(self, self.state.read()),
                RwMode::Write => {
                    let mut g = self.state.write();
                    g.timestamp += 1;
                    FakeGuard::Write(self, g)
                }
            }
        }
        fn relock(&self, version: u32) -> Option<FakeGuard<'_>> {
            let g = self.state.read();
            (g.timestamp == version).then_some(FakeGuard::Read(self, g))
        }
        fn pmap(&self) -> &PmapRef {
            &self.pmap
        }
        fn is_vmspace(&self) -> bool {
            true
        }
        fn is_intrsafe(&self) -> bool {
            false
        }
        fn is_pageable(&self) -> bool {
            true
        }
        fn note_minflt(&self) {
            self.minflt.fetch_add(1, Ordering::Relaxed);
        }
        fn note_majflt(&self) {
            self.majflt.fetch_add(1, Ordering::Relaxed);
        }
        fn note_maxrss(&self, kb: usize) {
            self.maxrss.fetch_max(kb, Ordering::Relaxed);
        }
    }

    impl FaultMapGuard for FakeGuard<'_> {
        type Entry = FakeEntry;
        type Map = FakeMap;

        fn map(&self) -> &FakeMap {
            match self {
                FakeGuard::Read(m, _) | FakeGuard::Write(m, _) => m,
            }
        }

        fn mode(&self) -> RwMode {
            match self {
                FakeGuard::Read(..) => RwMode::Read,
                FakeGuard::Write(..) => RwMode::Write,
            }
        }
        fn version(&self) -> u32 {
            self.state().timestamp
        }
        fn lookup(&self, va: usize) -> Option<&FakeEntry> {
            self.state()
                .entries
                .iter()
                .find(|e| e.start <= va && va < e.end)
        }
        fn next<'a>(&'a self, entry: &'a FakeEntry) -> Option<&'a FakeEntry> {
            let entries = &self.state().entries;
            let i = entries.iter().position(|e| ptr::eq(e, entry))?;
            entries.get(i + 1)
        }
        fn upgrade(self) -> Result<Self, Self> {
            match self {
                FakeGuard::Read(m, g) => match g.try_upgrade() {
                    Ok(mut w) => {
                        w.timestamp += 1;
                        Ok(FakeGuard::Write(m, w))
                    }
                    Err(g) => Err(FakeGuard::Read(m, g)),
                },
                w => Ok(w),
            }
        }
        fn downgrade(self) -> Self {
            match self {
                FakeGuard::Write(m, w) => FakeGuard::Read(m, w.downgrade()),
                r => r,
            }
        }
        /// The double copies the whole entry (no clipping): a new amap for an entry without
        /// one, else the amap's own copy.
        fn copy_amap(
            &self,
            e: &FakeEntry,
            waitf: i32,
            _canchunk: bool,
            _start: usize,
            _end: usize,
        ) {
            assert_eq!(self.mode(), RwMode::Write);
            let old = e.aref.borrow().clone();
            let slots = (e.end - e.start) / PAGE_SIZE;
            match old {
                None => {
                    if let Some(a) = amap_alloc(e.end - e.start, waitf, true) {
                        *e.aref.borrow_mut() = Some((a, 0));
                        e.needscopy.set(false);
                    }
                }
                Some((src, pageoff)) => match amap_copy_from(&src, pageoff, slots, waitf) {
                    CopyOutcome::TakeOver => e.needscopy.set(false),
                    CopyOutcome::New(a) => {
                        *e.aref.borrow_mut() = Some((a, 0));
                        e.needscopy.set(false);
                    }
                    CopyOutcome::NoMemory => {}
                },
            }
        }
    }

    impl FaultEntry for FakeEntry {
        type Map = FakeMap;
        type Object = TestObj;

        fn start(&self) -> usize {
            self.start
        }
        fn end(&self) -> usize {
            self.end
        }
        fn offset(&self) -> Voff {
            self.offset
        }
        fn protection(&self) -> VmProt {
            self.protection
        }
        fn advice(&self) -> i32 {
            self.advice
        }
        fn is_wired(&self) -> bool {
            self.wired
        }
        fn is_submap(&self) -> bool {
            self.submap.is_some()
        }
        fn is_copyonwrite(&self) -> bool {
            self.cow
        }
        fn is_needscopy(&self) -> bool {
            self.needscopy.get()
        }
        fn is_nofault(&self) -> bool {
            self.nofault
        }
        fn is_stack(&self) -> bool {
            false
        }
        fn is_wc(&self) -> bool {
            false
        }
        fn submap(&self) -> Option<&'static FakeMap> {
            self.submap
        }
        fn aref(&self) -> Option<(AmapRef, i32)> {
            self.aref.borrow().clone()
        }
        fn object(&self) -> Option<TestObj> {
            self.object.clone()
        }
    }

    /// An amap of `npages` pages (a normal one, chunks made as anons arrive, past 16).
    fn amap(npages: usize) -> AmapRef {
        amap_alloc(npages * PAGE_SIZE, hal::malloc::M_WAITOK, true).expect("an amap")
    }

    /// An anon of `amap` at page `slot` with a resident, active page.
    fn anon_with_page(amap: &AmapRef, w: &mut WriteWitness<'_>, slot: usize) -> AnonRef {
        let anon = uvm_analloc(amap.lock()).expect("an anon");
        let pg = uvm_pagealloc(Owner::Anon(&*anon), 0, 0).expect("a page");
        pg.clear_bits(PG_BUSY | PG_FAKE);
        uvm_pageactivate(pg, Owner::Anon(&*anon));
        assert!(matches!(
            amap_add(amap, w, 0, slot * PAGE_SIZE, anon.clone(), false),
            Ok(None)
        ));
        anon
    }

    /// An anon of `amap` at page `slot` that is only in swap (slot 5).
    fn anon_paged_out(amap: &AmapRef, w: &mut WriteWitness<'_>, slot: usize) -> AnonRef {
        let anon = uvm_analloc(amap.lock()).expect("an anon");
        anon.set_swslot(5);
        assert!(matches!(
            amap_add(amap, w, 0, slot * PAGE_SIZE, anon.clone(), false),
            Ok(None)
        ));
        anon
    }

    #[test]
    fn uvmadvice_is_what_uvmfault_init_computes() {
        uvmfault_init();
        // (advice, nback, nforw) with 4 KiB pages: 16 KiB around a normal fault, 32 KiB behind
        // a sequential one.
        let table = [
            (MADV_NORMAL, 3, 4),
            (hal::sys::mman::MADV_RANDOM, 0, 0),
            (MADV_SEQUENTIAL, 8, 7),
            (hal::sys::mman::MADV_WILLNEED, 0, 0),
        ];
        for (advice, nback, nforw) in table {
            let a = UVMADVICE[advice as usize];
            assert_eq!((a.nback, a.nforw), (nback, nforw), "advice {advice}");
            assert!(a.nback + a.nforw < UVM_MAXRANGE);
        }
    }

    #[test]
    fn a_zero_fill_fault_promotes_a_zeroed_anon() {
        let _g = setup();
        let map = map_of(vec![FakeEntry::anon(VA, 4)]);
        let (faults, amcopy, przero, up) = (
            count(UvmExpCounters::Faults),
            count(UvmExpCounters::FltAmcopy),
            count(UvmExpCounters::FltPrzero),
            count(UvmExpCounters::FltUp),
        );

        // A read fault: the amap is made (needs-copy without an object), the read lock on it
        // upgraded for the promotion, and a zeroed anon page mapped (case 2B, zero fill).
        assert_eq!(uvm_fault(map, VA + 8, VM_FAULT_INVALID, PROT_READ), Ok(()));
        let anon = map.anon(VA).expect("an anon");
        let pg = anon.page().expect("its page");
        assert_eq!(map.pa(VA), Some(pg.phys_addr()));
        assert_ne!(pg.flags() & PQ_ANON, 0);
        assert_ne!(pg.flags() & PQ_ACTIVE, 0);
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE), 0);
        assert_eq!(anon.refs(), 1);
        assert_eq!(count(UvmExpCounters::Faults), faults + 1);
        assert_eq!(count(UvmExpCounters::FltAmcopy), amcopy + 1);
        assert_eq!(count(UvmExpCounters::FltPrzero), przero + 1);
        assert_eq!(count(UvmExpCounters::FltUp), up + 1);
        assert_eq!(map.minflt.load(Ordering::Relaxed), 1);
        assert!(map.maxrss.load(Ordering::Relaxed) >= 4);

        // A write fault on it is case 1A: the same anon and page.
        let anon1a = count(UvmExpCounters::FltAnon);
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE), Ok(()));
        assert!(map.anon(VA).is_some_and(|a| AnonRef::ptr_eq(&a, &anon)));
        assert_eq!(map.pa(VA), Some(pg.phys_addr()));
        assert_eq!(count(UvmExpCounters::FltAnon), anon1a + 1);

        // Nothing is left locked.
        assert!(map.is_free());
        assert_eq!(map.amap(VA).expect("the amap").lock().status(), 0);
    }

    #[test]
    fn faults_outside_the_entries_or_their_protection_fail() {
        let _g = setup();
        let o = obj();
        let map = map_of(vec![
            FakeEntry::anon(VA, 1),
            // Nothing backs it and nothing will: case 0.
            FakeEntry::new(VA + 4 * PAGE_SIZE, 1),
            FakeEntry::with_object(VA + 8 * PAGE_SIZE, 1, &o),
        ]);
        assert_eq!(
            uvm_fault(map, 0x1000, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EFAULT),
            "outside the map"
        );
        assert_eq!(
            uvm_fault(map, VA + 2 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EFAULT),
            "no entry"
        );
        assert_eq!(
            uvm_fault(map, VA + 4 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EFAULT),
            "case 0"
        );
        assert_eq!(
            uvm_fault(map, VA, VM_FAULT_INVALID, PROT_EXEC),
            Err(Errno::EACCES),
            "more than the entry allows"
        );
        // The pager fails a read of an ordinary entry: EIO.
        o.0.script.borrow_mut().push_back(VM_PAGER_ERROR);
        assert_eq!(
            uvm_fault(map, VA + 8 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EIO)
        );
        assert!(map.is_free());
        assert_eq!(o.0.lock.status(), 0);
    }

    #[test]
    fn a_write_after_a_shared_amap_copies_the_anon() {
        let _g = setup();
        // The parent's amap, with one anon, referenced by two entries as after fork(2).
        let x = amap(1);
        let parent_anon = {
            let mut w = x.lock().write();
            anon_with_page(&x, &mut w, 0)
        };
        amap_ref(&x, 0, PAGE_SIZE, AMAP_REFALL);
        assert_eq!(x.refs(), 2);
        let parent = map_of(vec![FakeEntry::with_amap(VA, 1, &x)]);
        let child_entry = FakeEntry::with_amap(VA, 1, &x);
        child_entry.needscopy.set(true);
        let child = map_of(vec![FakeEntry {
            cow: true,
            ..child_entry
        }]);
        assert_eq!(uvm_fault(parent, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        let parent_pa = parent.pa(VA).expect("the parent's page");

        // The child's read fault copies the amap (no object: needs-copy is resolved) and maps
        // the parent's page read-only (case 1A with an_ref 2).
        assert_eq!(uvm_fault(child, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        assert_eq!(child.pa(VA), Some(parent_pa));
        assert_eq!(parent_anon.refs(), 2);
        assert_eq!(x.refs(), 1);

        // Its write fault promotes a copy into a new anon (case 1B).
        let acow = count(UvmExpCounters::FltAcow);
        let out = anons_out();
        assert_eq!(uvm_fault(child, VA, VM_FAULT_INVALID, PROT_WRITE), Ok(()));
        let child_anon = child.anon(VA).expect("the child's anon");
        assert!(!AnonRef::ptr_eq(&child_anon, &parent_anon));
        assert_eq!((parent_anon.refs(), child_anon.refs()), (1, 1));
        let child_pa = child.pa(VA).expect("the child's page");
        assert_ne!(child_pa, parent_pa);
        assert_eq!(parent.pa(VA), Some(parent_pa));
        assert!(
            parent
                .anon(VA)
                .is_some_and(|a| AnonRef::ptr_eq(&a, &parent_anon))
        );
        assert_eq!(count(UvmExpCounters::FltAcow), acow + 1);
        assert_eq!(anons_out(), out + 1);
        assert!(child.is_free() && parent.is_free());
    }

    #[test]
    fn a_paged_out_anon_under_a_read_lock_upgrades_and_is_read_in() {
        let _g = setup();
        let x = amap(4);
        let anon = {
            let mut w = x.lock().write();
            anon_paged_out(&x, &mut w, 0)
        };
        let map = map_of(vec![FakeEntry::with_amap(VA, 4, &x)]);
        let slots = std::rc::Rc::new(RefCell::new(Vec::new()));
        let seen = std::rc::Rc::clone(&slots);
        on_swap_get(move |pg, swslot| {
            // Everything is unlocked during the read; the page is busy and new.
            assert_ne!(pg.flags() & PG_BUSY, 0);
            seen.borrow_mut().push(swslot);
            VM_PAGER_OK
        });
        let (up, pageins, anretry, relck) = (
            count(UvmExpCounters::FltUp),
            count(UvmExpCounters::Pageins),
            count(UvmExpCounters::FltAnretry),
            count(UvmExpCounters::FltRelck),
        );

        // A read fault: no page and only a read lock (ENOLCK), the upgrade, the page read in
        // with everything unlocked, the relock, the retry, case 1A.
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        no_swap_get();
        assert_eq!(*slots.borrow(), [5]);
        let pg = anon.page().expect("the page read in");
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE | PG_WANTED), 0);
        assert_eq!(map.pa(VA), Some(pg.phys_addr()));
        assert_eq!(count(UvmExpCounters::FltUp), up + 1);
        assert_eq!(count(UvmExpCounters::Pageins), pageins + 1);
        assert_eq!(count(UvmExpCounters::FltAnretry), anretry + 1);
        assert_eq!(count(UvmExpCounters::FltRelck), relck + 1);
        // uvmfault_anonget counts each call, the retry after the upgrade too (the C's
        // `goto retry` calls it again).
        assert_eq!(map.majflt.load(Ordering::Relaxed), 2);
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);
    }

    #[test]
    fn anonget_restarts_when_the_map_changed_during_its_read() {
        let _g = setup();
        let x = amap(1);
        let anon = {
            let mut w = x.lock().write();
            anon_paged_out(&x, &mut w, 0)
        };
        let map = map_of(vec![FakeEntry::with_amap(VA, 1, &x)]);
        let reads = std::rc::Rc::new(Cell::new(0));
        let seen = std::rc::Rc::clone(&reads);
        on_swap_get(move |_, _| {
            // While the page is read in (everything unlocked), another process changes the map.
            seen.set(seen.get() + 1);
            map.bump();
            VM_PAGER_OK
        });
        let (norelck, pageins) = (
            count(UvmExpCounters::FltNorelck),
            count(UvmExpCounters::Pageins),
        );

        // The relock fails while anonget owns the page: it takes the anon's lock again to
        // finish the read (the page activated, unbusied), lets it go and restarts; the next
        // round finds the page resident.
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE), Ok(()));
        no_swap_get();
        assert_eq!(reads.get(), 1);
        assert_eq!(count(UvmExpCounters::FltNorelck), norelck + 1);
        assert_eq!(count(UvmExpCounters::Pageins), pageins + 1);
        let pg = anon.page().expect("the page read in");
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE | PG_WANTED), 0);
        assert_eq!(map.pa(VA), Some(pg.phys_addr()));
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);
    }

    #[test]
    fn anonget_restarts_when_the_anon_moved_while_it_slept() {
        let _g = setup();
        let x = amap(1);
        let (old, newer) = {
            let mut w = x.lock().write();
            let old = anon_with_page(&x, &mut w, 0);
            let newer = uvm_analloc(x.lock()).expect("an anon");
            let npg = uvm_pagealloc(Owner::Anon(&*newer), 0, 0).expect("a page");
            npg.clear_bits(PG_BUSY | PG_FAKE);
            uvm_pageactivate(npg, Owner::Anon(&*newer));
            (old, newer)
        };
        // Someone holds the old anon's page busy (an I/O in progress).
        let opg = old.page().expect("its page");
        opg.set_bits(PG_BUSY);
        let map = map_of(vec![FakeEntry::with_amap(VA, 1, &x)]);

        // Another process: once the fault sleeps on the busy page, it moves a new anon into
        // the slot, then ends the I/O and wakes the sleeper.
        let (x2, old2, newer2) = (x.clone(), old.clone(), newer.clone());
        let mover = thread::spawn(move || {
            while opg.flags() & PG_WANTED == 0 {
                thread::sleep(Duration::from_millis(1));
            }
            let mut w = x2.lock().write();
            let out = amap_unadd(&x2, &mut w, 0, 0);
            assert!(AnonRef::ptr_eq(&out, &old2));
            out.set_refs(0);
            assert!(matches!(
                amap_add(&x2, &mut w, 0, 0, newer2, false),
                Ok(None)
            ));
            drop(w);
            opg.clear_bits(PG_BUSY | PG_WANTED);
            wakeup(opg.chan());
        });
        let (pgwait, relck) = (
            count(UvmExpCounters::FltPgwait),
            count(UvmExpCounters::FltRelck),
        );

        // The fault waits for the page, relocks, finds another anon in the slot (ERESTART),
        // and the next round maps the new anon's page.
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        mover.join().unwrap();
        assert_eq!(count(UvmExpCounters::FltPgwait), pgwait + 1);
        assert_eq!(count(UvmExpCounters::FltRelck), relck + 1);
        assert!(map.anon(VA).is_some_and(|a| AnonRef::ptr_eq(&a, &newer)));
        assert_eq!(map.pa(VA), newer.page().map(VmPage::phys_addr));
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);
    }

    #[test]
    fn a_failed_swap_read_frees_the_page_and_marks_the_slot_bad() {
        let _g = setup();
        let x = amap(1);
        let anon = {
            let mut w = x.lock().write();
            anon_paged_out(&x, &mut w, 0)
        };
        let map = map_of(vec![FakeEntry::with_amap(VA, 1, &x)]);
        // No stand-in: uvm_swap_get is reported and fails, as without swap.
        assert_eq!(
            uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE),
            Err(Errno::EACCES)
        );
        assert!(anon.page().is_none());
        assert_eq!(anon.swslot(), SWSLOT_BAD);
        assert!(map.pa(VA).is_none());
        assert!(
            testutil::printf_count("unported: uvmfault_anonget: uvm_swap_get (uvm_swap.c, M7)\n")
                >= 1
        );
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);
    }

    #[test]
    fn a_failed_upgrade_restarts_and_takes_the_write_lock() {
        let _g = setup();
        let x = amap(1);
        let map = map_of(vec![FakeEntry::with_amap(VA, 1, &x)]);
        let lock = x.lock().clone();
        let (held, go) = (mpsc::channel(), mpsc::channel::<()>());
        let (held_tx, held_rx) = held;
        let (go_tx, go_rx) = go;
        let reader = thread::spawn(move || {
            // Another process reads the amap while the fault wants to promote into it.
            let r = lock.read();
            held_tx.send(()).unwrap();
            let _ = go_rx.recv_timeout(Duration::from_millis(50));
            drop(r);
        });
        held_rx.recv().unwrap();
        let (noup, up) = (count(UvmExpCounters::FltNoup), count(UvmExpCounters::FltUp));

        // The read fault's promotion cannot upgrade beside the other reader: ERESTART, and the
        // next round takes the amap for writing at once (no upgrade), once the reader left.
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        let _ = go_tx.send(());
        reader.join().unwrap();
        assert_eq!(count(UvmExpCounters::FltNoup), noup + 1);
        assert_eq!(count(UvmExpCounters::FltUp), up);
        assert!(map.anon(VA).is_some());
        assert!(map.pa(VA).is_some());
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);
    }

    #[test]
    fn a_relock_after_the_map_changed_restarts_holding_nothing() {
        let _g = setup();
        let o = obj();
        let map = map_of(vec![FakeEntry::with_object(VA, 1, &o)]);
        // While the pager reads, another process changes the map.
        *o.0.meanwhile.borrow_mut() = Some(Box::new(move || map.bump()));
        let (norelck, obj2a) = (
            count(UvmExpCounters::FltNorelck),
            count(UvmExpCounters::FltObj),
        );

        // The read fault reads the page in, finds a new map version (nothing is locked then:
        // ERESTART), and the next round maps the page it finds resident (case 2A).
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        *o.0.meanwhile.borrow_mut() = None;
        assert_eq!(count(UvmExpCounters::FltNorelck), norelck + 1);
        assert_eq!(count(UvmExpCounters::FltObj), obj2a + 1);
        assert_eq!((o.gets(), o.sync_gets()), (2, 1));
        let pg = o.page(0).expect("the object's page");
        assert_eq!(pg.flags() & (PG_BUSY | PG_WANTED), 0);
        assert_eq!(map.pa(VA), Some(pg.phys_addr()));
        assert_eq!(map.majflt.load(Ordering::Relaxed), 1);
        assert_eq!(map.minflt.load(Ordering::Relaxed), 1);
        assert!(map.is_free());
        assert_eq!(o.0.lock.status(), 0);
    }

    #[test]
    fn an_anon_released_during_anonget_restarts_the_fault() {
        let _g = setup();
        let x = amap(1);
        let anon = {
            let mut w = x.lock().write();
            anon_paged_out(&x, &mut w, 0)
        };
        let map = map_of(vec![FakeEntry::with_amap(VA, 1, &x)]);
        let x2 = x.clone();
        on_swap_get(move |pg, _| {
            // While the page is read in, another process unmaps the anon: its last reference
            // goes, and with its page busy uvm_anfree only marks it released.
            let mut w = x2.lock().write();
            let gone = amap_unadd(&x2, &mut w, 0, 0);
            gone.set_refs(0);
            uvm_anfree(&gone);
            assert_ne!(pg.flags() & PG_RELEASED, 0);
            drop(gone);
            drop(w);
            VM_PAGER_OK
        });
        let pgrele = count(UvmExpCounters::FltPgrele);

        // The fault finds the page released: uvm_anon_release frees it, ERESTART, and the
        // next round zero-fills a new anon in the empty slot.
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE), Ok(()));
        no_swap_get();
        assert_eq!(count(UvmExpCounters::FltPgrele), pgrele + 1);
        assert!(anon.page().is_none());
        assert_eq!(
            hal::pool::PoolArc::count(&anon),
            1,
            "only the test's handle is left"
        );
        let new = map.anon(VA).expect("a new anon");
        assert!(!AnonRef::ptr_eq(&new, &anon));
        assert_eq!(map.pa(VA), new.page().map(VmPage::phys_addr));
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);
    }

    #[test]
    fn madv_sequential_deactivates_and_flushes_the_pages_behind() {
        let _g = setup();
        let o = obj();
        let x = amap(16);
        let behind: Vec<AnonRef> = {
            let mut w = x.lock().write();
            (0..8).map(|i| anon_with_page(&x, &mut w, i)).collect()
        };
        let mut e = FakeEntry::with_object(VA, 16, &o);
        e.advice = MADV_SEQUENTIAL;
        *e.aref.borrow_mut() = Some((x.clone(), 0));
        let map = map_of(vec![e]);

        // A read fault on page 8: 8 pages behind (deactivated, flushed from the object), 7
        // ahead; the page itself comes from the object (case 2A).
        let up = count(UvmExpCounters::FltUp);
        assert_eq!(
            uvm_fault(map, VA + 8 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Ok(())
        );
        assert_eq!(
            *o.0.flushes.borrow(),
            [(0, (8 * PAGE_SIZE) as Voff, PGO_DEACTIVATE)]
        );
        for (i, anon) in behind.iter().enumerate() {
            let pg = anon.page().expect("a page");
            assert_ne!(pg.flags() & PQ_INACTIVE, 0, "page {i} deactivated");
        }
        let pg = o.page(8 * PAGE_SIZE).expect("the object's page");
        assert_eq!(map.pa(VA + 8 * PAGE_SIZE), Some(pg.phys_addr()));
        // The object was locked for writing from the start (no upgrade).
        assert_eq!(count(UvmExpCounters::FltUp), up);
        assert!(map.is_free());
        assert_eq!((o.0.lock.status(), x.lock().status()), (0, 0));
    }

    #[test]
    fn wire_and_unwire_over_two_entries() {
        let _g = setup();
        let o = obj();
        let map = map_of(vec![
            FakeEntry::anon(VA, 2),
            FakeEntry::with_object(VA + 2 * PAGE_SIZE, 2, &o),
        ]);
        let end = VA + 4 * PAGE_SIZE;

        assert_eq!(uvm_fault_wire(map, VA, end, RW), Ok(()));
        for i in 0..4 {
            let pa = map.pa(VA + i * PAGE_SIZE).expect("mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 1, "page {i} wired");
            assert_eq!(pg.flags() & (PQ_ACTIVE | PQ_INACTIVE), 0);
            if i < 2 {
                assert_eq!(pg.flags() & PG_CLEAN, 0, "anon page {i} dirty");
            }
        }
        assert!(map.anon(VA + PAGE_SIZE).is_some());
        assert!(
            map.amap(VA + 2 * PAGE_SIZE).is_none(),
            "the object entry has no amap"
        );

        // The object entry has no amap: its pages' owner is found in its object alone.
        uvm_fault_unwire(map, VA, end);
        for i in 0..4 {
            let pa = map.pa(VA + i * PAGE_SIZE).expect("still mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
            assert_ne!(pg.flags() & PQ_ACTIVE, 0, "page {i} active again");
        }
        assert!(map.is_free());
        assert_eq!(o.0.lock.status(), 0);
    }

    #[test]
    fn a_pager_again_sleeps_and_retries() {
        let _g = setup();
        let o = obj();
        let map = map_of(vec![FakeEntry::with_object(VA, 1, &o)]);
        o.0.script.borrow_mut().push_back(VM_PAGER_AGAIN);

        let t = Instant::now();
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        assert!(
            t.elapsed() >= Duration::from_millis(5),
            "the fltagain2 pause"
        );
        assert_eq!((o.gets(), o.sync_gets()), (2, 2));
        assert_eq!(map.pa(VA), o.page(0).map(VmPage::phys_addr));
        assert!(map.is_free());
        assert_eq!(o.0.lock.status(), 0);
    }

    #[test]
    fn a_nofault_entry_zero_fills_what_the_pager_cannot_read() {
        let _g = setup();
        let o = obj();
        let x = amap(1);
        let mut e = FakeEntry::with_object(VA, 1, &o);
        e.nofault = true;
        *e.aref.borrow_mut() = Some((x.clone(), 0));
        let map = map_of(vec![e]);
        o.0.script.borrow_mut().push_back(VM_PAGER_ERROR);

        let przero = count(UvmExpCounters::FltPrzero);
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        assert_eq!(count(UvmExpCounters::FltPrzero), przero + 1);
        let anon = map.anon(VA).expect("a zero-filled anon");
        assert_eq!(map.pa(VA), anon.page().map(VmPage::phys_addr));
        assert!(o.page(0).is_none());
        assert!(map.is_free());
        assert_eq!((o.0.lock.status(), x.lock().status()), (0, 0));
    }

    #[test]
    fn a_write_to_a_private_object_mapping_promotes_a_copy() {
        let _g = setup();
        let o = obj();
        let mut e = FakeEntry::with_object(VA, 1, &o);
        e.cow = true;
        e.needscopy.set(true);
        let map = map_of(vec![e]);

        // A read maps the object's page read-only (case 2A on a copy-on-write entry; the
        // needs-copy stays, there is an object).
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        let opg = o.page(0).expect("the object's page");
        assert_eq!(map.pa(VA), Some(opg.phys_addr()));
        assert!(map.amap(VA).is_none());

        // A write copies it into an anon (case 2B with a copy).
        let prcopy = count(UvmExpCounters::FltPrcopy);
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE), Ok(()));
        assert_eq!(count(UvmExpCounters::FltPrcopy), prcopy + 1);
        let anon = map.anon(VA).expect("the copy");
        let pg = anon.page().expect("its page");
        assert_ne!(pg.phys_addr(), opg.phys_addr());
        assert_eq!(map.pa(VA), Some(pg.phys_addr()));
        assert!(map.is_free());
        assert_eq!(o.0.lock.status(), 0);
    }

    #[test]
    fn the_neighbour_anons_are_mapped_ahead() {
        let _g = setup();
        let map = map_of(vec![FakeEntry::anon(VA, 8)]);
        for i in 0..8 {
            assert_eq!(
                uvm_fault(map, VA + i * PAGE_SIZE, VM_FAULT_INVALID, PROT_WRITE),
                Ok(())
            );
        }
        map.pmap
            .remove(Vaddr::new(VA), Vaddr::new(VA + 8 * PAGE_SIZE));

        // One read fault on page 4 maps the MADV_NORMAL window (3 behind, 3 ahead to the
        // entry's end) too.
        let namap = count(UvmExpCounters::FltNamap);
        assert_eq!(
            uvm_fault(map, VA + 4 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Ok(())
        );
        for i in 1..8 {
            assert!(map.pa(VA + i * PAGE_SIZE).is_some(), "page {i} mapped");
        }
        assert!(map.pa(VA).is_none(), "page 0 is behind the window");
        assert_eq!(count(UvmExpCounters::FltNamap), namap + 6);
        assert!(map.is_free());
    }

    #[test]
    fn a_submap_entry_is_resolved_in_its_submap() {
        let _g = setup();
        let sub = map_of(vec![FakeEntry::anon(VA, 2)]);
        let mut e = FakeEntry::new(VA, 2);
        e.submap = Some(sub);
        let map = map_of(vec![e]);

        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE), Ok(()));
        let anon = sub.anon(VA).expect("the submap's anon");
        // The original map's pmap gets the mapping.
        assert_eq!(map.pa(VA), anon.page().map(VmPage::phys_addr));
        assert!(sub.pa(VA).is_none());
        assert!(map.is_free() && sub.is_free());
    }

    /// A new aobj of `npages` pages.
    fn aobj(npages: usize) -> ObjRef {
        uao_create(Vsize::new(npages * PAGE_SIZE), 0).expect("an aobj")
    }

    /// The object's page at `off`, under its lock.
    fn aobj_page(o: &ObjRef, off: usize) -> Option<&'static VmPage> {
        let _w = o.lock().read();
        uvm_pagelookup(&**o, off as Voff)
    }

    #[test]
    fn a_shared_aobj_mapping_reads_in_zeroed_pages() {
        let _g = setup();
        let o = aobj(4);
        let map = map_of(vec![FakeEntry::with_object(VA, 4, &o)]);
        let (get, obj2a) = (count(UvmExpCounters::FltGet), count(UvmExpCounters::FltObj));

        // A read fault: the aobj has no page yet, uao_get makes a zeroed one (case 2A).
        assert_eq!(
            uvm_fault(map, VA + PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Ok(())
        );
        let pg = aobj_page(&o, PAGE_SIZE).expect("the aobj's page");
        assert_eq!(map.pa(VA + PAGE_SIZE), Some(pg.phys_addr()));
        assert_ne!(pg.flags() & PQ_AOBJ, 0);
        assert_ne!(pg.flags() & PQ_ACTIVE, 0);
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE | PG_WANTED), 0);
        assert_eq!(count(UvmExpCounters::FltGet), get + 1);
        assert_eq!(count(UvmExpCounters::FltObj), obj2a + 1);

        // A write fault on it finds it resident (no I/O): the same page, writable.
        assert_eq!(
            uvm_fault(map, VA + PAGE_SIZE, VM_FAULT_INVALID, PROT_WRITE),
            Ok(())
        );
        assert_eq!(count(UvmExpCounters::FltGet), get + 1);
        assert_eq!(map.pa(VA + PAGE_SIZE), Some(pg.phys_addr()));
        assert!(map.is_free());
        assert_eq!(o.lock().status(), 0);
    }

    #[test]
    fn a_private_aobj_mapping_copies_on_write() {
        let _g = setup();
        let o = aobj(1);
        let mut e = FakeEntry::with_object(VA, 1, &o);
        e.cow = true;
        e.needscopy.set(true);
        let map = map_of(vec![e]);

        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        let opg = aobj_page(&o, 0).expect("the aobj's page");
        assert_eq!(map.pa(VA), Some(opg.phys_addr()));

        let prcopy = count(UvmExpCounters::FltPrcopy);
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE), Ok(()));
        assert_eq!(count(UvmExpCounters::FltPrcopy), prcopy + 1);
        let anon = map.anon(VA).expect("the copy");
        let pg = anon.page().expect("its page");
        assert_ne!(pg.phys_addr(), opg.phys_addr());
        assert_eq!(map.pa(VA), Some(pg.phys_addr()));
        // The aobj keeps its page.
        assert!(aobj_page(&o, 0).is_some_and(|p| ptr::eq(p, opg)));
        assert!(map.is_free());
        assert_eq!(o.lock().status(), 0);
    }

    #[test]
    fn wiring_an_aobj_page_dirties_it_and_drops_its_swap() {
        let _g = setup();
        let o = aobj(2);
        let map = map_of(vec![FakeEntry::with_object(VA, 2, &o)]);

        // Page 0 resident with a swap slot (as after a swap-in: the slot stays; slots exist
        // only by uao_set_swslot until uvm_swap, N3.5), page 1 not resident.
        assert_eq!(uvm_fault(map, VA, VM_FAULT_INVALID, PROT_READ), Ok(()));
        let aobj = o.aobj().expect("an aobj");
        {
            let mut w = o.lock().write();
            assert_eq!(uao_set_swslot(aobj, &mut w, 0, 7), 0);
        }

        // The wired PQ_AOBJ pages go through uao_dropswap: page 0's slot is released (its
        // uvm_swap_free reported), page 1 had none.
        assert_eq!(uvm_fault_wire(map, VA, VA + 2 * PAGE_SIZE, RW), Ok(()));
        {
            let w = o.lock().read();
            assert_eq!(uao_find_swslot(aobj, &w, 0), 0, "the slot was dropped");
            assert_eq!(uao_find_swslot(aobj, &w, 1), 0);
        }
        for i in 0..2 {
            let pg = aobj_page(&o, i * PAGE_SIZE).expect("the aobj's page");
            assert_eq!(map.pa(VA + i * PAGE_SIZE), Some(pg.phys_addr()));
            assert_eq!(pg.wire_count(), 1, "page {i} wired");
            assert_eq!(pg.flags() & PG_CLEAN, 0, "page {i} dirty");
            assert_ne!(pg.flags() & PQ_AOBJ, 0);
        }
        uvm_fault_unwire(map, VA, VA + 2 * PAGE_SIZE);
        for i in 0..2 {
            let pg = aobj_page(&o, i * PAGE_SIZE).expect("the aobj's page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
        }
        assert!(map.is_free());
        assert_eq!(o.lock().status(), 0);
    }

    #[test]
    fn a_file_mapping_reads_the_file_and_fails_past_its_end() {
        let _g = setup();
        let _ = uvn_init();
        // uvn_attach runs under the kernel lock; the fault takes it in uvn_io.
        hal::klock::kernel_lock();
        let vp = testutil::mem_vnode((0..PAGE_SIZE + 100).map(|i| (i % 251) as u8).collect());
        let o = uvn_attach(vp, PROT_READ | PROT_WRITE).expect("attached");
        hal::klock::kernel_unlock();
        let map = map_of(vec![FakeEntry::with_object(VA, 3, &o)]);

        // The file's second page (100 bytes, then zeroes) is read in and mapped (case 2A).
        assert_eq!(
            uvm_fault(map, VA + PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Ok(())
        );
        let pg = aobj_page(&o, PAGE_SIZE).expect("the vnode's page");
        assert_eq!(map.pa(VA + PAGE_SIZE), Some(pg.phys_addr()));
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE | PG_WANTED), 0);
        assert_eq!(map.majflt.load(Ordering::Relaxed), 1);
        // Its bytes: the file's (i % 251) up to its end, zeroes after (a view wants the page
        // busy, as the pager's).
        let bytes = {
            let _w = o.lock().write();
            pg.set_bits(PG_BUSY);
            let view = super::super::km::mapper().map_direct(pg.phys_addr());
            let b = view.as_slice().to_vec();
            drop(view);
            pg.clear_bits(PG_BUSY);
            b
        };
        for (i, &b) in bytes[..100].iter().enumerate() {
            assert_eq!(b, ((PAGE_SIZE + i) % 251) as u8, "byte {i} of the page");
        }
        assert!(
            bytes[100..].iter().all(|&b| b == 0),
            "zeroes past the file's end"
        );

        // Past the file's end the pager answers VM_PAGER_BAD and the fault EIO (the trap's
        // SIGBUS, diff-openbsd's child_sigbus).
        assert_eq!(
            uvm_fault(map, VA + 2 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EIO)
        );
        assert!(map.pa(VA + 2 * PAGE_SIZE).is_none());
        assert!(map.is_free());
        assert_eq!(o.lock().status(), 0);
    }

    #[test]
    fn an_amap_without_room_frees_the_anon_and_populates_before_the_retry() {
        let _g = setup();
        // A normal amap (past 16 slots) whose chunks are made as anons arrive.
        let x = amap(32);
        let map = map_of(vec![FakeEntry::with_amap(VA, 32, &x)]);
        let (noamap, out) = (count(UvmExpCounters::FltNoamap), anons_out());

        // The chunk's pool item does not come (amap_add fails): the anon goes back, and with
        // no swap configured uvm_swapisfull() is true (0 of 0 pages), so ENOMEM, as the C.
        testutil::fail_pool("amapchunkpl", 1);
        assert_eq!(
            uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE),
            Err(Errno::ENOMEM)
        );
        assert_eq!(count(UvmExpCounters::FltNoamap), noamap + 1);
        assert_eq!(anons_out(), out, "the anon went back");
        assert!(map.anon(VA).is_none() && map.pa(VA).is_none());
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);

        // With swap room, amap_populate makes the chunk, ERESTART, and the next round adds a
        // new anon.
        UVMEXP.swpages.store(100, Ordering::Relaxed);
        testutil::fail_pool("amapchunkpl", 1);
        let result = uvm_fault(map, VA, VM_FAULT_INVALID, PROT_WRITE);
        testutil::fail_pool("amapchunkpl", 0);
        UVMEXP.swpages.store(0, Ordering::Relaxed);
        assert_eq!(result, Ok(()));
        assert_eq!(count(UvmExpCounters::FltNoamap), noamap + 2);
        assert_eq!(
            anons_out(),
            out + 1,
            "the anon of the failed round went back"
        );
        let anon = map.anon(VA).expect("the anon of the second round");
        assert_eq!(map.pa(VA), anon.page().map(VmPage::phys_addr));
        assert!(map.is_free());
        assert_eq!(x.lock().status(), 0);
    }

    #[test]
    fn populating_a_wiped_out_amap_is_skipped() {
        let _g = setup();
        // A normal amap whose last reference went while the fault slept for its chunk.
        let x = amap(32);
        amap_unref(&x, 0, 32 * PAGE_SIZE, true);
        assert_eq!(x.refs(), 0);
        let (_, chunks) = amaps_out();

        // The chunk made unlocked goes back: no table to install it in (no use after wipeout).
        uvmfault_populate(&x, 0, 17 * PAGE_SIZE);
        assert_eq!(amaps_out().1, chunks);
        assert_eq!(x.lock().status(), 0);

        // A live one gets its chunk installed.
        let y = amap(32);
        uvmfault_populate(&y, 0, 17 * PAGE_SIZE);
        assert_eq!(amaps_out().1, chunks + 1);
        // ... once: a second populate finds it and gives its spare back.
        uvmfault_populate(&y, 0, 20 * PAGE_SIZE);
        assert_eq!(amaps_out().1, chunks + 1);
    }
}
/* </TESTS> */
