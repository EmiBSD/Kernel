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
 * Copyright (c) 2008-2009,2014-2016 Dale Rahn <drahn@dalerahn.com>
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
/* </LICENSES> */

/* <CODE> */
//! arm64's vp tables (`struct pmapvp0`..`pmapvp3`) as the pmap reaches them: through the
//! direct map, one view function for every frame, the walks over atomic words, and the
//! mapping descriptors (`struct pte_desc`) the level-3 tables own.
//!
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//!
//! **A vp table** is two pages, as the C's `struct pmapvp*` (8 KiB): the hardware table the
//! MMU walks (512 descriptors) and the software page beside it. Here they are two frames from
//! the page provider, not one contiguous item: [`Table`] names both. A level-0 to 2 table's
//! software page holds, per entry, the physical address of the child's software page (0: no
//! child of ours; the child's hardware page is the descriptor's output address); a level-3
//! table's holds the entry's `pte_desc`, a [`Link`].
//!
//! **The view.** [`view`] is the only place a frame becomes a Rust reference: a page of
//! atomics at the frame's direct-map address, after `trust::check_frame` placed the whole page
//! inside the direct map the boot glue described (`BootstrapParams`). OpenBSD's arm64 has no
//! direct map; LZ's (Limine's HHDM, or `locore0.S`'s `DIRECT_BASE`) stands in for it (the
//! backend's deviations), so this check is the one amd64's view makes. The frames viewed are
//! vp table pages, the kernel's pages of `pte_desc`s, and the frames the pmap is asked to
//! zero, copy or sync (the trust of `PhysPages` and of the caller's `Paddr`). A table is
//! viewed only while it is held: a user pmap's while the pmap is (its tables are freed by its
//! last drop alone), the kernel pmap's always (never freed).
//!
//! **The links.** A [`Link`] is a `pte_desc` pointer: a level-3 slot, a page's pv head, or a
//! descriptor's `pv_next`. Its one dereference is [`Link::target`], which takes the witness
//! that keeps the target alive ([`Holds`]): the pmap's lock (or the kernel pmap, whose
//! descriptors are never freed) for a slot, the page's `pv_mtx` for a pv link.
//!
//! ## Redesign
//! - LZ's `vp_table!` types hold the hardware table in an `UnsafeCell<[u64; 512]>` read and
//!   written by volatile accesses (each an `unsafe` block) and the children as
//!   `Cell<*mut Child>` turned into `&'static` by `vp()` (another); the tables came from
//!   `pmap_vp_pages`, two contiguous pages through `uvm_page_physsteal`/`uvm_pglistalloc`.
//!   Here a table is two frames of the provider, each viewed as 512 atomic words, the
//!   children are physical addresses (no pointer, no dereference), and the walks
//!   ([`lookup`], [`enter`]) are safe code over the views.
//! - `struct pte_desc` is [`PteDesc`], all atomics (its fields change while other CPUs read
//!   them under the page's `pv_mtx`, as the C's do), 32 bytes: the pv list is singly linked
//!   (`pv_next`; the C's `LIST_ENTRY` has a back pointer too), unlinked by a walk from the
//!   head under `pv_mtx`, as amd64's `pv_entry`. A user mapping's descriptor is a
//!   `SharedBox<PteDesc>` of the pool `"pted"`, given up to its slot (`SharedBox::into_raw`)
//!   and taken back to be freed ([`Link::take`]); the kernel's come from pages of
//!   descriptors `pmap_growkernel` carves (`pmap_kpted_alloc`), never freed.

use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, AtomicU64, AtomicUsize, Ordering};

use super::super::include::param::{PAGE_MASK, PAGE_SIZE};
use super::super::include::pte::{Lx_TYPE_MASK, Lx_TYPE_PT, PTE_RPGN};
use super::backend::PvHead;
use super::layout::{PTED_VA_MANAGED_M, VP_ENTRIES, vp_idx};
use crate::assert::hal_panic;
use crate::mutex::MutexedGuard;
use crate::pmap::Pmap;
use crate::pool::{BoxPool, SharedBox};
use crate::sys::types::{Paddr, Vaddr};
use crate::trust::check_frame;

/// The kernel's `pte_desc`s one page holds (`PAGE_SIZE / sizeof(struct pte_desc)`).
pub(super) const PTEDS_PER_PAGE: usize = PAGE_SIZE / size_of::<PteDesc>();

/// A `pte_desc` pointer (see the module docs): null, or a live descriptor while the witness
/// of its kind is held.
///
/// Invariant: a link is a level-3 slot of a pmap's table, a page's pv head
/// ([`PvHead`]) or a descriptor's `pv_next`, and is null or names a live descriptor: (1) a
/// kernel slot names one that is never freed (carved from a page of descriptors the kernel
/// keeps, or a pool item given up for good); (2) a user slot owns its descriptor (put there by
/// `SharedBox::into_raw`), which only [`Link::take`] frees, under the pmap's lock held
/// exclusively, after the descriptor left its pv list; (3) a descriptor is on a pv list only
/// between `pmap_enter_pv` and `pmap_remove_pv`, both under that page's `pv_mtx`.
#[repr(transparent)]
pub(super) struct Link(AtomicPtr<PteDesc>);

impl Link {
    /// A null link.
    pub(super) const fn new() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }

    /// The descriptor's address, compared and never dereferenced.
    pub(super) fn get(&self) -> Option<NonNull<PteDesc>> {
        NonNull::new(self.0.load(Ordering::Acquire))
    }

    /// Points the link at `to` (with the provenance `to` carries) or nowhere.
    pub(super) fn set(&self, to: Option<NonNull<PteDesc>>) {
        self.0.store(
            to.map_or(ptr::null_mut(), NonNull::as_ptr),
            Ordering::Release,
        );
    }

    /// The descriptor the link names, alive as long as `held` is: `held` is the witness that
    /// keeps this link's kind of target (the module docs).
    pub(super) fn target<'w, W: Holds>(&self, held: &'w W) -> Option<&'w PteDesc> {
        let _ = held;
        let p = self.get()?;
        // SAFETY: by the type's invariant `p` names a live descriptor, and it stays alive while
        // `held` is borrowed: for a slot, `held` is that pmap's `Locked` (the kernel's, whose
        // descriptors are never freed; a user pmap's, under whose lock alone `take` frees, and
        // `take` needs it mutably, so not while this borrow lives; another CPU cannot take
        // the lock meanwhile); for a pv link, `held` is the page's `pv_mtx` guard, which
        // `pmap_remove_pv` needs before the descriptor can be freed. The pointer carries the
        // descriptor's provenance (from `SharedBox::into_raw`, or from a reference into its
        // page's view). Only shared references are made, and every field is atomic, so the
        // CPUs reading it under `pv_mtx` while its pmap's CPU updates it under `pm_mtx` race
        // with nothing.
        Some(unsafe { p.as_ref() })
    }

    /// A user slot's descriptor taken back from the slot, to be freed or moved: the slot is
    /// null after. `held` is the slot's pmap's lock, held exclusively.
    pub(super) fn take(
        &self,
        held: &mut Locked<'_>,
        pool: &'static BoxPool<PteDesc>,
    ) -> Option<SharedBox<PteDesc>> {
        if matches!(held, Locked::Kernel) {
            hal_panic(format_args!("pmap: a kernel pted is never freed"));
        }
        let p = NonNull::new(self.0.swap(ptr::null_mut(), Ordering::AcqRel))?;
        // SAFETY: a user slot holds only what `SharedBox::into_raw` returned for an item of
        // `pool` (the pool `"pted"`, `pmap_enter`'s only source of user descriptors), and the
        // swap above cleared it, so this pointer is rebuilt once. `held` is borrowed mutably,
        // so no `Link::target` reference made under it is alive, and the descriptor is off its
        // pv list (the caller's `pmap_remove_pv` ran first, checked below), so no pv reader
        // can reach it.
        let b = unsafe { SharedBox::from_raw(pool, p) };
        if b.va.load(Ordering::Relaxed) & PTED_VA_MANAGED_M as usize != 0 {
            hal_panic(format_args!("pmap: a pted freed while on its pv list"));
        }
        Some(b)
    }
}

/// `pm_mtx` held for a user pmap, or the kernel pmap, which is never locked: the witness the
/// walks of a pmap's slots take (`pmap_lock`/`pmap_unlock`).
pub(super) enum Locked<'a> {
    /// The kernel pmap: its descriptors are never freed.
    Kernel,
    /// A user pmap's `pm_mtx`.
    User(#[allow(dead_code, reason = "held for its drop")] MutexedGuard<'a, ()>),
}

/// What keeps a [`Link`]'s target alive: the pmap's [`Locked`], or a page's `pv_mtx` guard.
pub(super) trait Holds {}

impl Holds for Locked<'_> {}

impl Holds for MutexedGuard<'_, PvHead> {}

/// `struct pte_desc`: one mapping (see the module's redesign).
///
/// Every bit pattern is a valid value (atomics only): the kernel's descriptors are viewed in
/// zeroed pages.
#[repr(C)]
pub(super) struct PteDesc {
    /// `pted_pv_list`: the next mapping of the same page.
    pub(super) pv_next: Link,
    /// `pted_pte`: the PTE as it should be: the frame and the `PROT_*` bits now granted.
    pub(super) pte: AtomicU64,
    /// `pted_pmap`: the pmap of the mapping (its address).
    pub(super) pmap: AtomicPtr<Pmap>,
    /// `pted_va`: the virtual address, with the `PMAP_CACHE_*`, `PROT_*` and `PTED_VA_*` flags
    /// in its low bits.
    pub(super) va: AtomicUsize,
}

impl PteDesc {
    /// An empty descriptor (the C's `PR_ZERO`).
    pub(super) const fn new() -> Self {
        Self {
            pv_next: Link::new(),
            pte: AtomicU64::new(0),
            pmap: AtomicPtr::new(ptr::null_mut()),
            va: AtomicUsize::new(0),
        }
    }
}

/// A vp table: its hardware page and its software page (see the module docs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Table {
    /// The page the MMU walks.
    pub(super) hw: Paddr,
    /// The page of children (levels 0 to 2) or descriptors (level 3).
    pub(super) sw: Paddr,
}

impl Table {
    /// The hardware descriptor `i`.
    pub(super) fn entry(&self, i: usize) -> &'static AtomicU64 {
        &words(self.hw)[i % VP_ENTRIES]
    }

    /// The child table at `i` (levels 0 to 2), `None` where no table of ours hangs (an invalid
    /// entry, or one the boot loader made).
    pub(super) fn child(&self, i: usize) -> Option<Table> {
        let sw = words(self.sw)[i % VP_ENTRIES].load(Ordering::Acquire);
        if sw == 0 {
            return None;
        }
        let d = self.entry(i).load(Ordering::Acquire);
        if d & Lx_TYPE_MASK != Lx_TYPE_PT {
            hal_panic(format_args!(
                "pmap: vp entry {i} has a child but descriptor {d:#x}"
            ));
        }
        Some(Table {
            hw: Paddr::new((d & PTE_RPGN) as usize),
            sw: Paddr::new(sw as usize),
        })
    }

    /// `pmap_set_l1`/`_l2`/`_l3`: hangs `child` at `i`, its table descriptor first, then the
    /// software link a walk reads first.
    pub(super) fn set_child(&self, i: usize, child: Table) {
        // `VP_Lx(pa)`: the table's physical address as a table descriptor.
        self.entry(i)
            .store(child.hw.as_usize() as u64 | Lx_TYPE_PT, Ordering::Release);
        words(self.sw)[i % VP_ENTRIES].store(child.sw.as_usize() as u64, Ordering::Release);
    }

    /// Unhooks the child at `i` (the software link first): `pmap_vp_destroy`'s
    /// `vp->vp[i] = NULL`.
    pub(super) fn clear_child(&self, i: usize) {
        words(self.sw)[i % VP_ENTRIES].store(0, Ordering::Release);
        self.entry(i).store(0, Ordering::Release);
    }

    /// The level-3 slot `i` (this table is a level-3 one).
    pub(super) fn slot(&self, i: usize) -> &'static Link {
        &slots(self.sw)[i % VP_ENTRIES]
    }
}

/// A level-3 position: the table and the index of a virtual address in it (`pl3`).
#[derive(Clone, Copy, Debug)]
pub(super) struct L3 {
    /// The level-3 table.
    pub(super) table: Table,
    /// `VP_IDX3(va)`.
    pub(super) idx: usize,
}

impl L3 {
    /// The hardware PTE (`*pl3`).
    pub(super) fn pte(&self) -> &'static AtomicU64 {
        self.table.entry(self.idx)
    }

    /// The descriptor slot (`vp3->vp[VP_IDX3(va)]`).
    pub(super) fn slot(&self) -> &'static Link {
        self.table.slot(self.idx)
    }
}

/// `pmap_direct_base`: where physical address 0 is mapped (the boot glue's, at bootstrap).
static DIRECT_BASE: AtomicUsize = AtomicUsize::new(0);
/// The end of the direct map.
static DIRECT_END: AtomicUsize = AtomicUsize::new(0);

/// What a frame may be viewed as: a page of atomics for which every bit pattern is valid.
/// Implemented here alone, for the three types `view` lists.
trait FrameView {}

impl FrameView for [AtomicU64; VP_ENTRIES] {}

impl FrameView for [Link; VP_ENTRIES] {}

impl FrameView for [PteDesc; PTEDS_PER_PAGE] {}

/// Sets the direct map's bounds, once, from the bootstrap.
pub(super) fn set_direct_map(base: Vaddr, end: Vaddr) {
    DIRECT_BASE.store(base.as_usize(), Ordering::Relaxed);
    DIRECT_END.store(end.as_usize(), Ordering::Relaxed);
}

/// `PMAP_DIRECT_MAP(pa)`: the direct-map address of a physical address.
pub(super) fn direct_map(pa: Paddr) -> Vaddr {
    Vaddr::new(
        DIRECT_BASE
            .load(Ordering::Relaxed)
            .wrapping_add(pa.as_usize()),
    )
}

/// `PMAP_DIRECT_UNMAP(va)`, `None` outside the direct map.
pub(super) fn direct_unmap(va: Vaddr) -> Option<Paddr> {
    let base = DIRECT_BASE.load(Ordering::Relaxed);
    let end = DIRECT_END.load(Ordering::Relaxed);
    let va = va.as_usize();
    (base != 0 && base <= va && va < end).then(|| Paddr::new(va - base))
}

/// The frame at `pa` as a `T` (see the module docs). Panics when `pa` is not a page's address
/// or the page is not inside the direct map.
fn view<T: FrameView>(pa: Paddr) -> &'static T {
    let base = DIRECT_BASE.load(Ordering::Relaxed);
    let end = DIRECT_END.load(Ordering::Relaxed);
    let va = check_frame(pa.as_usize(), PAGE_SIZE, base, end);
    // SAFETY: `va` is the direct-map address of the page-aligned frame `pa`, the whole page
    // inside `[DIRECT_BASE, DIRECT_END)` (`check_frame` above): the range the boot glue gave
    // the bootstrap (`BootstrapParams`, the boot contract of decision 30), which the boot
    // loader maps read-write to RAM for the kernel's lifetime and nothing unmaps. `T` is one
    // of the three `FrameView` types, each exactly one page (the compile-time checks below)
    // with an alignment a page satisfies, made of atomics alone (`AtomicU64`, `Link`'s
    // transparent `AtomicPtr`, `PteDesc`'s `repr(C)` of atomics), so the 4096 bytes are a valid
    // `T` whatever they hold, and every access through the view is atomic: it races with
    // nothing (the MMU's walks only read). No plain Rust reference to a viewed frame exists
    // (the frames are vp table pages, the kernel's descriptor pages and frames handed to the
    // pmap to fill, the trust of `PhysPages` and of the caller's `Paddr`, `sys/hal/pmap.rs`),
    // and a view is used only while its frame is held (module docs).
    unsafe { &*ptr::with_exposed_provenance::<T>(va) }
}

/// The frame at `pa` as 512 atomic words (a hardware table, a software page of levels 0 to 2,
/// a frame to zero or copy).
pub(super) fn words(pa: Paddr) -> &'static [AtomicU64; VP_ENTRIES] {
    view(pa)
}

/// The software page of a level-3 table: its 512 descriptor slots.
fn slots(pa: Paddr) -> &'static [Link; VP_ENTRIES] {
    view(pa)
}

/// A page of the kernel's descriptors (`pmap_kpted_alloc`'s page).
pub(super) fn pted_page(pa: Paddr) -> &'static [PteDesc; PTEDS_PER_PAGE] {
    view(pa)
}

/// Zeroes the frame at `pa` (`pagezero_cache`, the C's `dc zva` loop).
pub(super) fn zero_frame(pa: Paddr) {
    for w in words(pa) {
        w.store(0, Ordering::Relaxed);
    }
}

/// Copies the frame at `src` into the frame at `dst`.
pub(super) fn copy_frame(src: Paddr, dst: Paddr) {
    for (d, s) in words(dst).iter().zip(words(src)) {
        d.store(s.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

/// The levels a walk descends from the root to reach a level-3 table: from level 0 on a
/// four-level pmap (the kernel's, the EFI pmap), from level 1 on a three-level one.
fn levels(four_level: bool) -> core::ops::Range<usize> {
    if four_level { 0..3 } else { 1..3 }
}

/// `pmap_vp_lookup`: the level-3 position of `va` under `root`, `None` where a table above is
/// missing.
pub(super) fn lookup(root: Table, four_level: bool, va: usize) -> Option<L3> {
    let mut t = root;
    for level in levels(four_level) {
        t = t.child(vp_idx(level, va))?;
    }
    Some(L3 {
        table: t,
        idx: vp_idx(3, va),
    })
}

/// `pmap_vp_enter`'s walk: the level-3 position of `va` under `root`, hanging the missing
/// tables `alloc` makes (`Err(level)`, the level of the table that could not be had).
pub(super) fn enter(
    root: Table,
    four_level: bool,
    va: usize,
    mut alloc: impl FnMut() -> Option<Table>,
) -> Result<L3, usize> {
    let mut t = root;
    for level in levels(four_level) {
        let i = vp_idx(level, va);
        t = match t.child(i) {
            Some(c) => c,
            None => {
                let Some(c) = alloc() else {
                    return Err(level + 1);
                };
                t.set_child(i, c);
                c
            }
        };
    }
    Ok(L3 {
        table: t,
        idx: vp_idx(3, va),
    })
}

/// Every child table under `t` (a table of `level`), deepest first, each given to `free`
/// after its own children; the level-3 tables' slots to `slot` first. `pmap_vp_destroy`'s
/// walk.
pub(super) fn destroy(
    t: Table,
    level: usize,
    slot: &mut impl FnMut(&'static Link),
    free: &mut impl FnMut(Table),
) {
    if level == 3 {
        for i in 0..VP_ENTRIES {
            slot(t.slot(i));
        }
        return;
    }
    for i in 0..VP_ENTRIES {
        let Some(c) = t.child(i) else {
            continue;
        };
        t.clear_child(i);
        destroy(c, level + 1, slot, free);
        free(c);
    }
}

const _: () = {
    assert!(size_of::<[AtomicU64; VP_ENTRIES]>() == PAGE_SIZE);
    assert!(size_of::<[Link; VP_ENTRIES]>() == PAGE_SIZE);
    assert!(size_of::<[PteDesc; PTEDS_PER_PAGE]>() == PAGE_SIZE);
    assert!(size_of::<PteDesc>() == 32);
    assert!(align_of::<PteDesc>() <= PAGE_SIZE);
    assert!(PAGE_MASK == PAGE_SIZE - 1);
};

// The descriptors are read by every CPU (`pv_mtx` readers beside `pm_mtx` writers).
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<PteDesc>();
    shared::<Link>();
};
/* </CODE> */
