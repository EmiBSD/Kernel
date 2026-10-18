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
//! direct map, one view function for every frame, the walks over atomic words, the mapping
//! descriptors (`struct pte_desc`) the level-3 tables own, and the pv lists that link them.
//!
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//!
//! **A vp table** is two pages, as the C's `struct pmapvp*` (8 KiB): the hardware table the
//! MMU walks (512 descriptors) and the software page beside it. Here they are two frames from
//! the page provider, not one contiguous item: [`Table`] names both. A level-0 to 2 table's
//! software page holds, per entry, the physical address of the child's software page (0: no
//! child of ours; the child's hardware page is the descriptor's output address); a level-3
//! table's holds the entry's `pte_desc`, a slot [`Link`].
//!
//! **The view.** [`view`] is the only place a frame becomes a Rust reference: atomics at the
//! frame's direct-map address, after `trust::check_frame` placed the whole page inside the
//! direct map the boot glue described (`BootstrapParams`). OpenBSD's arm64 has no direct map;
//! LZ's (Limine's HHDM, or `locore0.S`'s `DIRECT_BASE`) stands in for it (the backend's
//! deviations), so this check is the one amd64's view makes. The frames viewed are vp table
//! pages, the kernel's pages of `pte_desc`s, and the frames the pmap is asked to zero, copy or
//! sync (the trust of `PhysPages` and of the caller's `Paddr`). A table is viewed only while
//! it is held: a user pmap's while the pmap is (its tables are freed by its last drop alone),
//! the kernel pmap's always (never freed).
//!
//! **The links.** A [`Link`] is a `pte_desc` pointer of one kind: a level-3 slot
//! ([`SlotKind`]), or a pv link ([`PvKind`]: a page's head, a descriptor's `pv_next` or
//! `pv_prev`). Its one dereference is [`Link::target`], which takes the witness that keeps a
//! target of that kind alive ([`Holds`]): the slot's pmap's [`Locked`] (built only by
//! [`Locked::of`], which takes the lock of a user pmap; a level-3 position checks it is its
//! own pmap's), or the page's `pv_mtx` guard for a pv link.
//!
//! **The removal rule.** A managed descriptor's hardware PTE is zeroed, flushed, the descriptor
//! unlinked and its `pte`/`va` cleared inside one critical section of its page's `pv_mtx`
//! (`pmap_remove_pted`, `pmap_kremove_pg`): no walker holding `pv_mtx` ever sees a linked
//! descriptor whose hardware PTE a removal has zeroed, so none can write that PTE back.
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
//!   them under the page's `pv_mtx`, as the C's do), 40 bytes as the C's (so the pool
//!   `"pted"` shows the C's item size): `pted_pv_list`'s `LIST_ENTRY` is `pv_next` and
//!   `pv_prev` (the previous descriptor, `None` at the head, where the C points at the
//!   previous `le_next`), so [`pv_unlink`] is the C's O(1) `LIST_REMOVE` through the one
//!   dereference. A user mapping's descriptor is a `SharedBox<PteDesc>` of the pool `"pted"`,
//!   given up to its slot ([`L3::put`], `SharedBox::into_raw`) and taken back to be freed
//!   ([`L3::take`]); the kernel's come from pages of descriptors `pmap_growkernel` carves
//!   (`pmap_kpted_alloc`), never freed.

use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, AtomicU64, AtomicUsize, Ordering};

use super::super::include::param::{PAGE_MASK, PAGE_SIZE};
use super::super::include::pte::{Lx_TYPE_MASK, Lx_TYPE_PT, PTE_RPGN};
use super::layout::{PTED_VA_MANAGED_M, VP_ENTRIES, vp_idx};
use crate::assert::hal_panic;
use crate::mutex::MutexedGuard;
use crate::pmap::Pmap;
use crate::pool::{BoxPool, SharedBox};
use crate::sys::types::{Paddr, Vaddr};
use crate::trust::check_frame;

/// The kernel's `pte_desc`s one page holds (`PAGE_SIZE / sizeof(struct pte_desc)`).
pub(super) const PTEDS_PER_PAGE: usize = PAGE_SIZE / size_of::<PteDesc>();

/// The kind of a level-3 slot link: kept alive by its pmap's [`Locked`].
pub(super) struct SlotKind;

/// The kind of a pv link (a page's head, `pv_next`, `pv_prev`): kept alive by the page's
/// `pv_mtx`.
pub(super) struct PvKind;

/// A `pte_desc` pointer of kind `K` (see the module docs): null, or a live descriptor while
/// the witness of its kind is held.
///
/// Invariant: a slot link is a level-3 slot of a pmap's table and a pv link a page's head or a
/// descriptor's `pv_next`/`pv_prev`, and each is null or names a live descriptor: (1) a kernel
/// slot names one that is never freed (carved from a page of descriptors the kernel keeps, or
/// a pool item given up for good); (2) a user slot owns its descriptor (put there by
/// [`L3::put`]), which only [`Link::take`] frees, under that pmap's [`Locked`] held
/// exclusively, after the descriptor left its pv list; (3) a descriptor is on a pv list only
/// between [`pv_insert`] and [`pv_unlink`], both under that page's `pv_mtx`.
#[repr(transparent)]
pub(super) struct Link<K>(AtomicPtr<PteDesc>, PhantomData<K>);

impl<K> Link<K> {
    /// A null link.
    pub(super) const fn new() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()), PhantomData)
    }

    /// The descriptor's address, compared and never dereferenced.
    pub(super) fn get(&self) -> Option<NonNull<PteDesc>> {
        NonNull::new(self.0.load(Ordering::Acquire))
    }

    /// Points the link at `to` (with the provenance `to` carries) or nowhere.
    fn set(&self, to: Option<NonNull<PteDesc>>) {
        self.0.store(
            to.map_or(ptr::null_mut(), NonNull::as_ptr),
            Ordering::Release,
        );
    }

    /// The descriptor the link names, alive as long as `held` is: `held` is the witness that
    /// keeps a target of this link's kind (the module docs).
    pub(super) fn target<'w, W: Holds<K>>(&self, held: &'w W) -> Option<&'w PteDesc> {
        let _ = held;
        let p = self.get()?;
        // SAFETY: by the type's invariant `p` names a live descriptor, and it stays alive while
        // `held` is borrowed, because `Holds` is implemented only for the witness of `K`: for a
        // slot link, a `Locked` (the kernel's, whose descriptors are never freed; a user
        // pmap's, under which alone `take` frees, needing it mutably, so not while this borrow
        // lives, and which no other CPU can hold meanwhile; that it is this slot's pmap's
        // `Locked` is checked by `L3`, whose positions are the only way to a slot outside
        // this file and `pmap_vp_destroy`'s walk over a pmap its caller owns); for a pv link,
        // a page's `pv_mtx` guard, which `pv_unlink` needs before the descriptor can be freed
        // (every pv link is read from that page's head or from a descriptor reached from it).
        // The pointer carries the descriptor's provenance (from `SharedBox::into_raw`, or from
        // a reference into its page's view). Only shared references are made, and every field
        // is atomic, so the CPUs reading it under `pv_mtx` while its pmap's CPU updates it
        // under `pm_mtx` race with nothing.
        Some(unsafe { p.as_ref() })
    }
}

impl Link<SlotKind> {
    /// A user slot's descriptor taken back from the slot, to be freed or moved: the slot is
    /// null after. `held` is the slot's pmap's lock, held exclusively.
    pub(super) fn take(
        &self,
        held: &mut Locked<'_>,
        pool: &'static BoxPool<PteDesc>,
    ) -> Option<SharedBox<PteDesc>> {
        if held.is_kernel() {
            hal_panic(format_args!("pmap: a kernel pted is never freed"));
        }
        let p = NonNull::new(self.0.swap(ptr::null_mut(), Ordering::AcqRel))?;
        // SAFETY: a user slot holds only what `SharedBox::into_raw` returned for an item of
        // `pool` (the pool `"pted"`, `pmap_enter`'s only source of user descriptors, through
        // `L3::put`), and the swap above cleared it, so this pointer is rebuilt once. `held` is
        // borrowed mutably, so no `Link::target` reference made under it is alive, and the
        // descriptor is off its pv list (the caller's `pv_unlink` ran first, checked below),
        // so no pv reader can reach it.
        let b = unsafe { SharedBox::from_raw(pool, p) };
        if b.va.load(Ordering::Relaxed) & PTED_VA_MANAGED_M as usize != 0
            || b.pv_next.get().is_some()
            || b.pv_prev.get().is_some()
        {
            hal_panic(format_args!("pmap: a pted freed while on its pv list"));
        }
        Some(b)
    }
}

/// `pm_mtx` held for a user pmap, or the kernel pmap, which is never locked: the witness the
/// walks of a pmap's slots take (`pmap_lock`/`pmap_unlock`). Built only by [`Locked::of`].
pub(super) struct Locked<'a> {
    /// The pmap whose slots it opens.
    pm: &'a Pmap,
    /// A user pmap's `pm_mtx`; `None` for the kernel pmap.
    _guard: Option<MutexedGuard<'a, ()>>,
}

impl<'a> Locked<'a> {
    /// `pmap_lock(pm)`: takes a user pmap's `pm_mtx`; the kernel pmap has no lock.
    pub(super) fn of(pm: &'a Pmap) -> Self {
        let guard = if pm.is_kernel() {
            None
        } else {
            Some(pm.md.mtx.lock())
        };
        Self { pm, _guard: guard }
    }

    /// Whether this is the kernel pmap's witness.
    pub(super) fn is_kernel(&self) -> bool {
        self.pm.is_kernel()
    }

    /// Panics unless this witness is `owner`'s (a level-3 position's pmap).
    fn check(&self, owner: *const Pmap) {
        if !ptr::eq(self.pm, owner) {
            hal_panic(format_args!("pmap: a slot read under another pmap's lock"));
        }
    }
}

/// What keeps a [`Link`] of kind `K` alive: the pmap's [`Locked`] for a slot, a page's
/// `pv_mtx` guard for a pv link.
pub(super) trait Holds<K> {}

impl Holds<SlotKind> for Locked<'_> {}

impl Holds<PvKind> for MutexedGuard<'_, PvHead> {}

/// `struct pte_desc`: one mapping (see the module's redesign).
///
/// Every bit pattern is a valid value (atomics only): the kernel's descriptors are viewed in
/// zeroed pages.
#[repr(C)]
pub(super) struct PteDesc {
    /// `pted_pv_list.le_next`: the next mapping of the same page.
    pub(super) pv_next: Link<PvKind>,
    /// `pted_pv_list.le_prev`: the previous mapping of the same page, `None` at the head.
    pub(super) pv_prev: Link<PvKind>,
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
            pv_prev: Link::new(),
            pte: AtomicU64::new(0),
            pmap: AtomicPtr::new(ptr::null_mut()),
            va: AtomicUsize::new(0),
        }
    }
}

/// `struct vm_page_md`'s list: the page's managed mappings, under `pv_mtx` (`pv_list`).
pub(crate) struct PvHead(pub(super) Link<PvKind>);

impl PvHead {
    /// No mapping.
    pub(crate) const fn new() -> Self {
        Self(Link::new())
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
        self.set_child_with(i, child, 0);
    }

    /// [`Table::set_child`] with the table descriptor's attribute bits `attrs` (the kernel's
    /// managed range: `Lx_PT_PXN`, `pmap_growkernel`), which no walk reads back.
    pub(super) fn set_child_with(&self, i: usize, child: Table, attrs: u64) {
        // `VP_Lx(pa)`: the table's physical address as a table descriptor.
        self.entry(i).store(
            child.hw.as_usize() as u64 | attrs | Lx_TYPE_PT,
            Ordering::Release,
        );
        words(self.sw)[i % VP_ENTRIES].store(child.sw.as_usize() as u64, Ordering::Release);
    }

    /// Unhooks the child at `i` (the software link first): `pmap_vp_destroy`'s
    /// `vp->vp[i] = NULL`.
    pub(super) fn clear_child(&self, i: usize) {
        words(self.sw)[i % VP_ENTRIES].store(0, Ordering::Release);
        self.entry(i).store(0, Ordering::Release);
    }

    /// The level-3 slot `i` (this table is a level-3 one).
    fn slot(&self, i: usize) -> &'static Link<SlotKind> {
        &slots(self.sw)[i % VP_ENTRIES]
    }

    /// Whether the kernel's level-3 slot `i` holds a descriptor (`pmap_growkernel`).
    pub(super) fn kernel_slot_full(&self, i: usize) -> bool {
        self.slot(i).get().is_some()
    }

    /// `vp3->vp[k] = pted` for one of the kernel's carved descriptors, which is never freed
    /// (`pmap_growkernel`, the kernel pmap's tables).
    pub(super) fn set_kernel_slot(&self, i: usize, pted: NonNull<PteDesc>) {
        self.slot(i).set(Some(pted));
    }
}

/// A level-3 position: the table and the index of a virtual address in it (`pl3`), and the
/// pmap whose tables they are.
#[derive(Clone, Copy, Debug)]
pub(super) struct L3 {
    /// The level-3 table.
    table: Table,
    /// `VP_IDX3(va)`.
    idx: usize,
    /// The pmap the walk started from (compared, never dereferenced).
    owner: *const Pmap,
}

impl L3 {
    /// The hardware PTE (`*pl3`).
    pub(super) fn pte(&self) -> &'static AtomicU64 {
        self.table.entry(self.idx)
    }

    /// Whether the slot holds a descriptor.
    pub(super) fn occupied(&self) -> bool {
        self.table.slot(self.idx).get().is_some()
    }

    /// The descriptor in the slot (`vp3->vp[VP_IDX3(va)]`), under its pmap's witness.
    pub(super) fn pted<'w>(&self, held: &'w Locked<'_>) -> Option<&'w PteDesc> {
        held.check(self.owner);
        self.table.slot(self.idx).target(held)
    }

    /// `vp3->vp[VP_IDX3(va)] = pted`: the slot, empty, takes `pted` over (the kernel's for
    /// good, a user pmap's until [`L3::take`]).
    pub(super) fn put(&self, held: &Locked<'_>, pted: SharedBox<PteDesc>) {
        held.check(self.owner);
        let slot = self.table.slot(self.idx);
        if slot.get().is_some() {
            hal_panic(format_args!("pmap_vp_enter: the slot holds a pted"));
        }
        slot.set(Some(SharedBox::into_raw(pted)));
    }

    /// A user slot's descriptor taken back ([`Link::take`]).
    pub(super) fn take(
        &self,
        held: &mut Locked<'_>,
        pool: &'static BoxPool<PteDesc>,
    ) -> Option<SharedBox<PteDesc>> {
        held.check(self.owner);
        self.table.slot(self.idx).take(held, pool)
    }
}

/// `pmap_direct_base`: where physical address 0 is mapped (the boot glue's, at bootstrap).
static DIRECT_BASE: AtomicUsize = AtomicUsize::new(0);
/// The end of the direct map.
static DIRECT_END: AtomicUsize = AtomicUsize::new(0);

/// What a frame may be viewed as: at most a page of atomics, for which every bit pattern is
/// valid. Implemented here alone, for the three types `view` lists.
trait FrameView {}

impl FrameView for [AtomicU64; VP_ENTRIES] {}

impl FrameView for [Link<SlotKind>; VP_ENTRIES] {}

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
    // of the three `FrameView` types, each at most one page (the compile-time checks below)
    // with an alignment a page satisfies, made of atomics alone (`AtomicU64`, `Link`'s
    // transparent `AtomicPtr` beside a zero-sized marker, `PteDesc`'s `repr(C)` of atomics),
    // so the bytes are a valid `T` whatever they hold, and every access through the view is
    // atomic: it races with nothing (the MMU's walks only read). No plain Rust reference to a
    // viewed frame exists (the frames are vp table pages, the kernel's descriptor pages and
    // frames handed to the pmap to fill, the trust of `PhysPages` and of the caller's `Paddr`,
    // `sys/hal/pmap.rs`), and a view is used only while its frame is held (module docs).
    unsafe { &*ptr::with_exposed_provenance::<T>(va) }
}

/// The frame at `pa` as 512 atomic words (a hardware table, a software page of levels 0 to 2,
/// a frame to zero or copy).
pub(super) fn words(pa: Paddr) -> &'static [AtomicU64; VP_ENTRIES] {
    view(pa)
}

/// The software page of a level-3 table: its 512 descriptor slots.
fn slots(pa: Paddr) -> &'static [Link<SlotKind>; VP_ENTRIES] {
    view(pa)
}

/// A page of the kernel's descriptors (`pmap_kpted_alloc`'s page: 102 of them, the last 16
/// bytes unused, as the C's).
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

/// `pmap_vp_lookup`: the level-3 position of `va` in `pm`, `None` where a table above is
/// missing (or the pmap has none yet).
pub(super) fn lookup(pm: &Pmap, va: usize) -> Option<L3> {
    let mut t = pm.md.root()?;
    for level in levels(pm.md.four_level()) {
        t = t.child(vp_idx(level, va))?;
    }
    Some(L3 {
        table: t,
        idx: vp_idx(3, va),
        owner: pm,
    })
}

/// `pmap_vp_enter`'s walk: the level-3 position of `va` in `pm`, hanging the missing tables
/// `alloc` makes (`Err(level)`, the level of the table that could not be had). `held` is
/// `pm`'s witness (the tables of a user pmap grow under its lock).
pub(super) fn enter(
    pm: &Pmap,
    held: &Locked<'_>,
    va: usize,
    mut alloc: impl FnMut() -> Option<Table>,
) -> Result<L3, usize> {
    held.check(pm);
    let Some(mut t) = pm.md.root() else {
        hal_panic(format_args!("pmap_vp_enter: pmap without a top table"));
    };
    for level in levels(pm.md.four_level()) {
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
        owner: pm,
    })
}

/// Every child table under `t` (a table of `level`), deepest first, each given to `free`
/// after its own children; the level-3 tables' slots to `slot` first. `pmap_vp_destroy`'s
/// walk, over the tables of a pmap its caller holds exclusively (the last drop).
pub(super) fn destroy(
    t: Table,
    level: usize,
    slot: &mut impl FnMut(&'static Link<SlotKind>),
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

/// `LIST_INSERT_HEAD(&pg->mdpage.pv_list, pted, pted_pv_list)` under the page's `pv_mtx`.
pub(super) fn pv_insert(head: &MutexedGuard<'_, PvHead>, pted: &PteDesc) {
    let me = NonNull::from(pted);
    if let Some(first) = head.0.target(head) {
        first.pv_prev.set(Some(me));
    }
    pted.pv_prev.set(None);
    pted.pv_next.set(head.0.get());
    head.0.set(Some(me));
}

/// `LIST_REMOVE(pted, pted_pv_list)` under the page's `pv_mtx` (O(1), by `pv_prev`); panics
/// when `pted` is not on this page's list. The caller passes the guard of the page `pted.pte`
/// maps (`pted_md`): `pted.pv_prev` is read through it before the membership check, so a guard
/// of another page would read a link that page's lock does not cover.
pub(super) fn pv_unlink(head: &MutexedGuard<'_, PvHead>, pted: &PteDesc) {
    let me = Some(NonNull::from(pted));
    let next = pted.pv_next.get();
    match pted.pv_prev.target(head) {
        Some(prev) if prev.pv_next.get() == me => prev.pv_next.set(next),
        None if head.0.get() == me => head.0.set(next),
        _ => hal_panic(format_args!("pmap_remove_pv: pted not on its page's list")),
    }
    if let Some(n) = pted.pv_next.target(head) {
        n.pv_prev.set(pted.pv_prev.get());
    }
    pted.pv_next.set(None);
    pted.pv_prev.set(None);
    pted.va
        .fetch_and(!(PTED_VA_MANAGED_M as usize), Ordering::Relaxed);
}

const _: () = {
    assert!(size_of::<[AtomicU64; VP_ENTRIES]>() == PAGE_SIZE);
    assert!(size_of::<[Link<SlotKind>; VP_ENTRIES]>() == PAGE_SIZE);
    assert!(size_of::<[PteDesc; PTEDS_PER_PAGE]>() <= PAGE_SIZE);
    // The C's `struct pte_desc` (`LIST_ENTRY`, `pted_pte`, `pted_pmap`, `pted_va`): the pool
    // `"pted"` keeps the C's item size.
    assert!(size_of::<PteDesc>() == 40);
    assert!(PTEDS_PER_PAGE == 102);
    assert!(align_of::<PteDesc>() <= PAGE_SIZE);
    assert!(PAGE_MASK == PAGE_SIZE - 1);
};

// The descriptors are read by every CPU (`pv_mtx` readers beside `pm_mtx` writers).
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<PteDesc>();
    shared::<Link<SlotKind>>();
    shared::<PvHead>();
};
/* </CODE> */
