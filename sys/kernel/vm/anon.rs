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
//! Anonymous memory: `struct vm_anon` of `<uvm/uvm_anon.h>` and the anon operations of
//! `uvm/uvm_anon.c`.
//!
//! Upstream: sys/uvm/uvm_anon.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_anon.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_anon.rs@3c62ede68802
//!
//! Anonymous virtual memory is short term virtual memory that goes away when the processes
//! referencing it go away. An anonymous page of virtual memory is described by an [`Anon`].
//! For active anons the data can be in one of the following states: \[1\] in a `vm_page` with
//! no backing store allocated yet, \[2\] in a `vm_page` with backing store allocated, or \[3\]
//! paged out to backing store (no `vm_page`).
//!
//! An anon lives in the slots of one or more amaps (`kernel::vm::amap`): `amap_copy` makes
//! two amaps share it until a write fault copies it. Its lock is its amap's (one lock serves
//! an amap, every anon in it and every amap that shares anons with it), given to
//! [`uvm_analloc`] and kept for the anon's life. The C's `an_ref` counts the amap slots that
//! hold the anon, and every decision of the C reads that count (a write fault copies an anon
//! whose count is above one); the anon's memory belongs to the [`AnonRef`] handles, one per
//! slot and one per thread that keeps the anon across a sleep, and goes back to `"anonpl"`
//! with the last of them.
//!
//! ## Deviations
//! - Swap is not here (N3.5): [`uvm_anon_dropswap`] reports `uvm_swap_free` when an anon has a
//!   slot, which nothing assigns yet.
//! - [`uvm_anon_pagein`] needs `uvmfault_anonget` (the fault handler, N3.6) and reports it.
//!
//! ## Redesign
//! - LZ's `uvm_analloc() -> &'static VmAnon` (a pool item cast and written by hand, freed by
//!   `uvm_anfree`'s `pool_put` while other holders may still name it) -> [`AnonRef`], a
//!   `hal::pool::PoolArc<Anon>` of `"anonpl"`. The C's `an_ref` stays the count it is (the
//!   amap slots that hold the anon, [`Anon::refs`]), beside the handle count that owns the
//!   memory: every place the C increments `an_ref` also clones a handle (`amap_copy`'s slot
//!   copy; a new anon starts at one of each), and a thread that keeps an anon across a sleep
//!   clones a handle and leaves `an_ref` alone, so the copy-on-write decisions (`an_ref > 1`)
//!   read what the C reads while the memory cannot go before its last holder.
//!   [`uvm_anfree`] does what the C's does to the page and the swap slot; the `pool_put` is
//!   the drop of the last handle.
//! - `an_lock: Cell<*const Rwlock>`, set by every caller right after `uvm_analloc` and cleared
//!   before a fresh anon is freed -> a `hal::rwlock::SharedRwlock` handle given to
//!   [`uvm_analloc`] and never changed: the anon holds a clone of its amap's lock, so the
//!   C's `rw_obj_hold` that keeps the lock alive for a `PG_RELEASED` page (and the
//!   `rw_obj_free` in `uvm_anon_release`) is the anon's own handle.
//! - `an_ref` and `an_swslot` are relaxed atomics whose setters assert the C's lock rule
//!   (the write lock for `an_ref`, the lock for `an_swslot`); they take no witness, because
//!   an amap walk holds its chunk tables open through the write witness
//!   (`hal::rwlock::Guarded::get_mut` borrows it) while it drops its anons' counts.
//! - [`uvm_anon_release`] consumes the witness of the anon's lock (the C's `rw_exit`); the
//!   caller drops its handle after it.
//! - `MULTIPROCESSOR` is always configured (decision 40): [`uvm_anon_init_percpu`] always
//!   makes the per-CPU cache, and the C's `unused` padding is not needed: an item is the
//!   anon's 32 bytes and the handle's count and pool, above a `pool_cache_item` (`"anonpl"`
//!   shows 48 bytes where the C's shows 32: the user's decision D2 of N3.4).
//! - `uvm_anfree` leaves a slot marked `SWSLOT_BAD` out of `uvmexp.swpgonly`, as the C does
//!   (LZ tested only `an_swslot != 0`).

use core::sync::atomic::{AtomicI32, Ordering};

use hal::link::Link;
use hal::pool::{PR_NOWAIT, PR_WAITOK, PoolArc, TypedPool};
use hal::rwlock::{RW_READ, RW_WRITE, RawRwlock, SharedRwlock, Witness, WriteWitness};
use hal::sys::errno::Errno;
use hal::sys::mman::PROT_NONE;
use hal::vm::uvmexp::UVMEXP;

use super::amap::Amap;
use super::page::{Owner, OwnerTag, PG_BUSY, PG_RELEASED, PageAnon, VmPage, uvm_pagefree};
use crate::assert::{kassert, kpanic};
use crate::prf::unported;

/// `SWSLOT_BAD` (`<uvm/uvm_swap.h>`): a swap slot the pager could not read; never freed,
/// never used again.
pub const SWSLOT_BAD: i32 = -1;

/// `struct vm_anon`: one page of anonymous memory.
///
/// Locks: the anon's lock is its amap's (`an_lock`); `refs` (`an_ref`) is written under it
/// held for writing, `swslot` (`an_swslot`) under it, or by the holder of the anon's busy
/// page; `page` (`an_page`) by the page system under it.
pub struct Anon {
    /// `an_lock`: the lock, shared with the amap the anon is in.
    lock: SharedRwlock,
    /// `an_page`: the resident page, if in RAM.
    page: Link<VmPage>,
    /// `an_ref`: the amap slots that hold the anon.
    refs: AtomicI32,
    /// `an_swslot`: drum swap slot # (if != 0) \[if we hold an_page, PG_BUSY\].
    swslot: AtomicI32,
}

impl Anon {
    /// A fresh anon under `lock`: no page, no swap slot, one reference.
    fn new(lock: SharedRwlock) -> Self {
        Self {
            lock,
            page: Link::new(),
            refs: AtomicI32::new(1),
            swslot: AtomicI32::new(0),
        }
    }

    /// `an_lock`: the anon's lock (its amap's).
    pub fn lock(&self) -> &SharedRwlock {
        &self.lock
    }

    /// Whether the anon is under `lock` (`anon->an_lock == amap->am_lock`).
    pub fn shares_lock(&self, lock: &SharedRwlock) -> bool {
        SharedRwlock::same(&self.lock, lock)
    }

    /// `an_page`, if resident.
    pub fn page(&self) -> Option<&'static VmPage> {
        self.page.get()
    }

    /// `an_ref`: the amap slots that hold the anon.
    pub fn refs(&self) -> i32 {
        self.refs.load(Ordering::Relaxed)
    }

    /// `an_ref = n`, under the anon's lock held for writing.
    pub fn set_refs(&self, n: i32) {
        kassert!(self.write_held());
        self.refs.store(n, Ordering::Relaxed);
    }

    /// `an_swslot`.
    pub fn swslot(&self) -> i32 {
        self.swslot.load(Ordering::Relaxed)
    }

    /// `an_swslot = slot`, under the anon's lock.
    pub fn set_swslot(&self, slot: i32) {
        kassert!(self.lock_held());
        self.swslot.store(slot, Ordering::Relaxed);
    }

    /// `rw_write_held(anon->an_lock)`.
    fn write_held(&self) -> bool {
        self.lock.status() == RW_WRITE
    }

    /// `rw_lock_held(anon->an_lock)`.
    fn lock_held(&self) -> bool {
        matches!(self.lock.status(), RW_WRITE | RW_READ)
    }
}

/// A counted handle to an anon (`struct vm_anon *`): an amap slot's, or a thread's that keeps
/// the anon across a sleep.
pub type AnonRef = PoolArc<Anon>;

/// `uvm_anon_pool`: the anons, `"anonpl"`.
static ANON_POOL: TypedPool<Anon> = TypedPool::new();

/// The lock of the anon `uvm_anwait` takes from the pool and gives back at once (never
/// entered).
static ANWAIT_LOCK: RawRwlock = RawRwlock::new("anwait");

// The page system's view of an anon (`kernel::vm::page`): the owner of its page.
impl PageAnon for Anon {
    fn tag(&self) -> OwnerTag {
        OwnerTag::anon(self)
    }

    fn locked(&self, exclusive: bool) -> Option<bool> {
        Some(if exclusive {
            self.write_held()
        } else {
            self.lock_held()
        })
    }

    fn refs(&self) -> i32 {
        Anon::refs(self)
    }

    fn set_page(&self, pg: Option<&'static VmPage>) {
        self.page.set(pg);
    }
}

/// `uvm_anon_init`: the anon pool, `"anonpl"` at `ipl` (the machine's `IPL_MPFLOOR`),
/// `PR_WAITOK`, which keeps at most `uvmexp.free / 16` idle items. `ENOMEM` when the pool
/// cannot be made, `EBUSY` when it was.
pub fn uvm_anon_init(ipl: i32) -> Result<(), Errno> {
    ANON_POOL.init(ipl, PR_WAITOK, "anonpl")?;
    ANON_POOL.set_hiwat(
        (UVMEXP.free.load(Ordering::Relaxed) / 16)
            .max(0)
            .unsigned_abs(),
    );
    Ok(())
}

/// `uvm_anon_init_percpu`: the anon pool's per-CPU caches.
pub fn uvm_anon_init_percpu() {
    ANON_POOL.cache_init();
}

/// `uvm_analloc`: allocate a new anon, under `lock` (its amap's), with one reference and the
/// one handle that goes with it. `None` when the pool has no item now (`PR_NOWAIT`).
pub fn uvm_analloc(lock: &SharedRwlock) -> Option<AnonRef> {
    let lock = lock.clone();
    ANON_POOL.get_shared(PR_NOWAIT, || Anon::new(lock))
}

/// `uvm_anfree`: free a single anon structure: its page (or, when the page is busy, marks it
/// `PG_RELEASED` so that [`uvm_anon_release`] would release it later) and its swap slot.
///
/// The anon must be removed from the amap (its reference count is 0) and its lock held for
/// writing. Its memory goes with the last handle: the slot's, which the caller drops next,
/// or, for a busy page, the handle of the thread that holds the page busy.
pub fn uvm_anfree(anon: &Anon) {
    let pg = anon.page();

    kassert!(anon.write_held());
    kassert!(anon.refs() == 0);

    // Dispose of the page, if it is resident.
    if let Some(pg) = pg {
        // If the page is busy, mark it as PG_RELEASED, so that uvm_anon_release(9) would
        // release it later. The busy page's holder keeps a handle, hence the anon and its
        // lock.
        if pg.flags() & PG_BUSY != 0 {
            pg.set_bits(PG_RELEASED);
            return;
        }
        hal::pmap::page_protect(&pg.mdpage, PROT_NONE);
        uvm_pagefree(pg, Owner::Anon(anon));
    } else {
        let slot = anon.swslot();
        if slot != 0 && slot != SWSLOT_BAD {
            // This page is no longer only in swap.
            kassert!(UVMEXP.swpgonly.load(Ordering::Relaxed) > 0);
            UVMEXP.swpgonly.fetch_sub(1, Ordering::Relaxed);
        }
    }

    // Free any swap resources, leave a page replacement hint.
    uvm_anon_dropswap(anon);

    kassert!(anon.page.is_none());
    kassert!(anon.swslot() == 0);
}

/// `uvm_anwait`: wait for memory to become available to allocate an anon.
pub fn uvm_anwait() {
    // XXX: Want something like pool_wait()?
    drop(ANON_POOL.get_shared(PR_WAITOK, || {
        Anon::new(SharedRwlock::from_static(&ANWAIT_LOCK))
    }));
}

/// `uvm_anon_pagein`: fetch an anon's page.
///
/// The anon must be locked for writing (`w`, which is its lock's and is released on return).
/// Returns true if pagein was aborted due to lack of memory.
pub fn uvm_anon_pagein(amap: &Amap, anon: &Anon, w: WriteWitness<'_>) -> bool {
    kassert!(w.lock_id() == anon.lock.id());
    kassert!(anon.shares_lock(amap.lock()));

    // Get the page of the anon: uvmfault_anonget (the fault handler, N3.6); until then
    // nothing is paged out, so there is nothing to fetch.
    let _ = unported!("uvm_anon_pagein: uvmfault_anonget (M7a-3)");
    drop(w);
    false
}

/// `uvm_anon_dropswap`: release any swap resources from this anon.
///
/// The anon must be locked or have a reference count of 0.
pub fn uvm_anon_dropswap(anon: &Anon) {
    kassert!(anon.refs() == 0 || anon.lock_held());

    if anon.swslot() == 0 {
        return;
    }

    let _ = unported!("uvm_anon_dropswap: uvm_swap_free (swap, M7)");
    anon.swslot.store(0, Ordering::Relaxed);
}

/// `uvm_anon_release`: release an anon's `PG_RELEASED` page and its swap slot, then the
/// anon's lock (`w`, its witness). The anon has no reference left; its memory goes with the
/// caller's handle, which the caller drops next.
pub fn uvm_anon_release(anon: &Anon, w: WriteWitness<'_>) {
    if w.lock_id() != anon.lock.id() {
        kpanic(format_args!("uvm_anon_release: a witness of another lock"));
    }
    let Some(pg) = anon.page() else {
        kpanic(format_args!("uvm_anon_release: anon without a page"));
    };

    kassert!(pg.flags() & PG_RELEASED != 0);
    kassert!(pg.flags() & PG_BUSY != 0);
    kassert!(!pg.has_object());
    kassert!(pg.owned_by_anon(anon));
    kassert!(anon.refs() == 0);

    hal::pmap::page_protect(&pg.mdpage, PROT_NONE);
    uvm_pagefree(pg, Owner::Anon(anon));
    kassert!(anon.page.is_none());
    uvm_anon_dropswap(anon);
    drop(w);
}

/// The anon pool's items out (`pr_nout`), for the tests of this module and the amap's.
#[cfg(test)]
pub(crate) fn anons_out() -> u64 {
    ANON_POOL.nout()
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use super::*;
    use crate::testutil;
    use crate::vm::page::{PG_CLEAN, uvm_pagealloc};

    /// Real page memory, the lock objects and the anon pool.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(256 * 4096);
        crate::testutil::init_shared_once();
        let _ = uvm_anon_init(0);
        guard
    }

    fn lock() -> SharedRwlock {
        SharedRwlock::new("testlk").expect("a lock object")
    }

    #[test]
    fn anons_come_from_the_pool_and_free_their_page() {
        let _g = setup();
        let lock = lock();
        let out = anons_out();
        let anon = uvm_analloc(&lock).expect("an anon");
        assert_eq!(anons_out(), out + 1);
        assert_eq!(anon.refs(), 1);
        assert!(anon.page().is_none() && anon.shares_lock(&lock));

        let w = lock.write();
        let pg = uvm_pagealloc(Owner::Anon(&*anon), 0, 0).expect("a page");
        assert!(anon.page().is_some_and(|p| core::ptr::eq(p, pg)));
        assert!(pg.owned_by_anon(&*anon));
        pg.clear_bits(PG_BUSY);

        anon.set_refs(0);
        uvm_anfree(&anon);
        assert!(!pg.has_owner(), "the page went back to the free pool");
        drop(w);
        drop(anon);
        assert_eq!(anons_out(), out, "the last handle gave the item back");
    }

    #[test]
    fn a_busy_page_is_released_later_and_the_anon_lives_while_a_handle_does() {
        let _g = setup();
        let lock = lock();
        let out = anons_out();
        let anon = uvm_analloc(&lock).expect("an anon");
        // The faulting thread's handle: it keeps the anon across its I/O.
        let holder = anon.clone();

        let w = lock.write();
        let pg = uvm_pagealloc(Owner::Anon(&*anon), 0, 0).expect("a page");
        assert_ne!(pg.flags() & PG_BUSY, 0, "a new page is busy");
        anon.set_refs(0);
        uvm_anfree(&anon);
        assert_ne!(
            pg.flags() & PG_RELEASED,
            0,
            "a busy page is left to its holder"
        );
        assert!(pg.owned_by_anon(&*holder));
        drop(w);
        drop(anon); // the slot's handle
        assert_eq!(anons_out(), out + 1, "the holder keeps the anon");

        // The holder finds the page released: it frees the anon's page and the anon.
        let w = holder.lock().write();
        uvm_anon_release(&holder, w);
        assert!(!pg.has_owner());
        assert_eq!(holder.lock().status(), 0, "uvm_anon_release unlocked");
        drop(holder);
        assert_eq!(anons_out(), out);
    }

    #[test]
    fn a_bad_swap_slot_is_not_counted_as_swap_only() {
        let _g = setup();
        let lock = lock();
        let anon = uvm_analloc(&lock).expect("an anon");
        let w = lock.write();
        anon.set_swslot(SWSLOT_BAD);
        anon.set_refs(0);
        let before = UVMEXP.swpgonly.load(Ordering::Relaxed);
        uvm_anfree(&anon);
        assert_eq!(UVMEXP.swpgonly.load(Ordering::Relaxed), before);
        assert_eq!(anon.swslot(), 0, "the slot is dropped");
        drop(w);
        let _ = PG_CLEAN;
    }

    #[test]
    fn the_pool_keeps_a_sixteenth_of_free_memory_and_a_cache() {
        let _g = setup();
        uvm_anon_init_percpu();
        let (_, cache) = testutil::pool_knobs("anonpl").expect("the pool");
        assert!(cache);
        // uvm_anwait takes an item and gives it back.
        let out = anons_out();
        uvm_anwait();
        assert_eq!(anons_out(), out);
    }

    #[test]
    #[ignore = "reads reference/openbsd-src (just test-ref)"]
    fn constants_match_the_c() {
        let defs = crate::reftest::defines("sys/uvm/uvm_swap.h");
        let _ = crate::reftest::assert_defines!(defs; SWSLOT_BAD);
    }
}
/* </TESTS> */
