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
 * Copyright (c) 2024 Martin Pieuchot <mpi@openbsd.org>
 * Copyright (c) 2009, 2010 Ariane van der Steldt <ariane@stack.nl>
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
 * Copyright (c) 2024 Martin Pieuchot <mpi@openbsd.org>
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
//! Describe and manage free physical memory: `<uvm/uvm_pmemrange.h>`, `uvm/uvm_pmemrange.c`
//! and the per-CPU page cache of `<uvm/uvm_percpu.h>`.
//!
//! Upstream: sys/uvm/uvm_pmemrange.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_pmemrange.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_percpu.h @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_pmemrange.rs@3c62ede68802
//! LZ: sys/uvm/uvm_percpu.rs@3c62ede68802
//!
//! 2 trees: addr tree and size tree.
//!
//! The allocator keeps chunks of free pages (called a range). Two pages are part of the same
//! range if: all pages in between are part of that range, they are of the same memory type
//! (zeroed or non-zeroed), they are part of the same pmemrange. A pmemrange is a range of
//! memory which is part of the same vm_physseg and has a use-count.
//!
//! addr tree is `vm_page[0].objt`; size tree is `vm_page[1].objt`.
//!
//! The size tree is not used for memory ranges of 1 page, instead, single queue is
//! `vm_page[0].pageq`.
//!
//! `vm_page[0].fpgsz` describes the length of a free range. Two adjacent ranges are joined,
//! unless: they have pages in between them which are not free; they belong to different
//! memtypes (zeroed vs dirty memory); they are in different pmemrange areas (ISA vs non-ISA
//! memory for instance); they are not a continuation of the same array. The latter issue is
//! caused by vm_physseg ordering and splitting from the MD initialization machinery. The MD
//! code is dependant on freelists and happens to split ISA memory from non-ISA memory. (Note:
//! freelists die die die!)
//!
//! `uvm_page_init` guarantees that every vm_physseg contains an array of struct vm_page. This
//! code depends on that array. The array may break across vm_physsegs boundaries.
//!
//! Every range's trees and lists, its counters and its pages' `fpgsz` are guarded by
//! `uvm.fpageqlock`, here the mutex [`FREE`] whose data is the ranges' index ([`FreeState`]):
//! the functions that touch a range take the guarded state as their proof. The per-CPU page
//! caches (`MULTIPROCESSOR && __HAVE_UVM_PERCPU`, both machines) are each CPU's own, touched
//! at `splbio` only.
//!
//! ## Deviations
//! - Page numbers (`paddr_t` holding `atop()` results in C) are `usize`; byte addresses stay
//!   `Paddr`.
//! - `uvm_wait` is absent (no page daemon, N3.5), so a `UVM_PLA_WAITOK` request that cannot
//!   be met returns `ENOMEM` instead of sleeping for the page daemon (LZ's deviation, kept).
//!   LZ called its `uvm_wait`, which printed `unported: uvm_wait (uvm_pdaemon.c, M5)` once;
//!   the kernel crate has no `printf(9)` (N3.2's precedent for `km_alloc`), so this line is
//!   where the gap shows.
//! - `in_pagedaemon` answers false: there is no page daemon nor `curproc` here yet (N3.5, N4).
//! - The `goto`-driven search of `uvm_pmr_getpages` is written with labelled loops; the order
//!   of the tries, memtypes and ranges is the C's.
//! - `uvm_pmr_assertvalid` (`DEBUG`) is compiled for feature `debug` and for the host tests.
//! - `uvm_pagezero_thread` needs a kernel thread (N4) and is not here.
//! - The per-CPU page cache keeps each CPU's magazines in [`UVM_PMR_CACHES`] (a
//!   `hal::percpu::PerCpu`, the slot of `cpu_number()`) instead of `curcpu()->ci_uvm`: the
//!   uvm type stays out of the machine's `struct cpu_info`. Without `MULTIPROCESSOR` the
//!   single-CPU versions are used, as in C.
//! - `uvm_pmr_allocpmr` after `uvm_init` needs `malloc(9)` (only `uvm_pmr_use_inc`, called by
//!   `uvm_pmr_init` alone, reaches it, so the path is not reachable): it panics, as LZ did
//!   after reporting it unported.
//! - The buffer cache's statistics the allocator reads (`BUFPAGES_DEFICIT`,
//!   `BUFPAGES_INACT`, over `bcstats`) come through the [`BufCacheStats`] bridge (N5);
//!   before it is registered they are 0, their values at boot.
//! - The C's `printf` lines of this module ("Flags: ... will panic now.", "uvm_pmr_split: lost
//!   one pmr") go to the console through `hal::cons::Cons`, not through `printf(9)`'s message
//!   buffer, which is `bsd`'s.
//!
//! ## Redesign
//! - The ranges' and the pages' links are `hal::tree::Tree`/`hal::list::List` over `'static`
//!   elements (the page array and the pmemranges are stolen at boot and never freed), so the
//!   mutators are safe; `pmr_static` and `VmPage::forever` are gone.
//! - `pg + n` and `pg[1]` (the size tree's node) are array indices (`page::nth`,
//!   `page::nth_back`): the size tree's comparator reads the page before its node, and "the
//!   array broke" (`prev + prev->fpgsz != pg`) is `prev.index + prev.fpgsz != pg.index`.
//! - `uvm.fpageqlock` and `uvm.pmr_control` -> [`FREE`], a `Mutexed<FreeState>` the
//!   functions take as proof; the range counters are relaxed atomics under it.
//! - `uvm_pmr_allocpmr` builds the range in boot-stolen memory with `hal::bootmem::steal_one`
//!   (the C's `uvm_pageboot_alloc` and `memset`/`RBT_INIT`).
//! - The per-CPU magazines hold [`hal::link::Link`]s to pages instead of
//!   `Cell<*const VmPage>`, their counts atomics: `hal::percpu::PerCpu<UvmPmrCache>`.
//! - `splbio()`/`splx(s)` -> `hal::intr::splraise(IPL_BIO)`'s guard.

use core::cmp::Ordering as Cmp;
use core::fmt::Write;
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use hal::bootmem;
use hal::bridges::AlreadyRegistered;
use hal::intr::IPL_VM;
use hal::list::{List, ListEntry};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::sleep::wakeup;
use hal::sys::errno::Errno;
use hal::sys::types::Paddr;
use hal::tree::{Tree, TreeEntry};
use hal::vm::param::{atop, round_page, trunc_page};
use hal::vm::uvmexp::UVMEXP;

use super::page::{
    self, PG_ZERO, PQ_FREE, Pglist, VmPage, nth, nth_back, uvm_pagezero, uvm_pagezero_target,
    vm_page_to_phys,
};
use super::types::{
    UVM_PLA_NOWAIT, UVM_PLA_NOWAKE, UVM_PLA_TRYCONTIG, UVM_PLA_USERESERVE, UVM_PLA_WAITOK,
    UVM_PLA_ZERO, UvmConstraintRange,
};
use crate::assert::{kassert, kdassert, kpanic};
#[cfg(feature = "multiprocessor")]
use hal::{intr::IPL_BIO, link::Link, percpu::PerCpu, percpu::PerCpuInit};

/// Dirty memory.
pub const UVM_PMR_MEMTYPE_DIRTY: usize = 0;
/// Zeroed memory.
pub const UVM_PMR_MEMTYPE_ZERO: usize = 1;
/// Number of memory types.
pub const UVM_PMR_MEMTYPE_MAX: usize = 2;

/// `UVM_PMR_CACHEMAGSZ`: the number of pages per magazine should be large enough to get rid
/// of the contention in the pmemrange allocator during concurrent page faults and small enough
/// to limit fragmentation.
pub const UVM_PMR_CACHEMAGSZ: usize = 8;

hal::tree_adapter!(
    /// `uvm_pmr_addr`: free ranges by address (`vm_page[0].objt`).
    pub UvmPmrAddr: VmPage, objt, uvm_pmr_addr_cmp
);
hal::tree_adapter!(
    /// `uvm_pmr_size`: free ranges by size (`vm_page[1].objt`).
    pub UvmPmrSize: VmPage, objt, uvm_pmr_size_cmp
);
hal::tree_adapter!(
    /// `uvm_pmemrange_addr`: the ranges by address.
    pub UvmPmemrangeAddr: UvmPmemrange, pmr_addr, uvm_pmemrange_addr_cmp
);
hal::list_adapter!(
    /// `uvm_pmemrange_use`: the ranges by use count.
    pub PmrUse: UvmPmemrange, pmr_use
);

/// `struct uvm_pmemrange`: a range of physical memory with a use count and its free chunks.
/// Every field is \[F\].
pub struct UvmPmemrange {
    /// Free page chunks, sorted by addr.
    addr: Tree<UvmPmrAddr>,
    /// Free page chunks, sorted by size.
    size: [Tree<UvmPmrSize>; UVM_PMR_MEMTYPE_MAX],
    /// Single page regions (uses pageq).
    single: [Pglist; UVM_PMR_MEMTYPE_MAX],
    /// Start of address range (pgno).
    low: AtomicUsize,
    /// End +1 (pgno).
    high: AtomicUsize,
    /// Use counter.
    r#use: AtomicI32,
    /// Current range count.
    nsegs: AtomicUsize,
    /// pmr, sorted by use.
    pmr_use: ListEntry<UvmPmemrange>,
    /// pmr, sorted by address.
    pmr_addr: TreeEntry<UvmPmemrange>,
}

impl UvmPmemrange {
    /// An empty range (what `uvm_pmr_allocpmr` builds).
    pub const fn new() -> Self {
        Self {
            addr: Tree::new(),
            size: [Tree::new(), Tree::new()],
            single: [Pglist::new(), Pglist::new()],
            low: AtomicUsize::new(0),
            high: AtomicUsize::new(0),
            r#use: AtomicI32::new(0),
            nsegs: AtomicUsize::new(0),
            pmr_use: ListEntry::new(),
            pmr_addr: TreeEntry::new(),
        }
    }

    /// Start of address range (pgno).
    pub fn low(&self) -> usize {
        self.low.load(Ordering::Relaxed)
    }

    /// End +1 (pgno).
    pub fn high(&self) -> usize {
        self.high.load(Ordering::Relaxed)
    }

    /// Use counter.
    pub fn use_count(&self) -> i32 {
        self.r#use.load(Ordering::Relaxed)
    }

    /// Current range count.
    pub fn nsegs(&self) -> usize {
        self.nsegs.load(Ordering::Relaxed)
    }

    fn set_nsegs(&self, n: usize) {
        self.nsegs.store(n, Ordering::Relaxed);
    }
}

impl Default for UvmPmemrange {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct uvm_pmr_control`: all the ranges, by address and by use.
pub struct UvmPmrControl {
    /// The ranges, sorted by address.
    addr: Tree<UvmPmemrangeAddr>,
    /// The ranges, sorted by use.
    r#use: List<PmrUse>,
}

impl UvmPmrControl {
    /// No ranges.
    pub const fn new() -> Self {
        Self {
            addr: Tree::new(),
            r#use: List::new(),
        }
    }

    /// The ranges by address, in order.
    pub fn ranges(&self) -> impl Iterator<Item = &'static UvmPmemrange> {
        self.addr.iter()
    }

    /// The ranges by use, in order.
    pub fn by_use(&self) -> impl Iterator<Item = &'static UvmPmemrange> {
        self.r#use.iter()
    }
}

impl Default for UvmPmrControl {
    fn default() -> Self {
        Self::new()
    }
}

/// What `uvm.fpageqlock` guards: the ranges' index (`uvm.pmr_control`) and, through it, every
/// range and the free pages' links and sizes.
pub struct FreeState {
    /// `uvm.pmr_control`.
    pub control: UvmPmrControl,
}

impl FreeState {
    const fn new() -> Self {
        Self {
            control: UvmPmrControl::new(),
        }
    }
}

/// `struct uvm_pmr_cache_item`: a magazine.
#[cfg(feature = "multiprocessor")]
pub struct UvmPmrCacheItem {
    /// `upci_pages`: the pages; the first `upci_npages` are valid.
    upci_pages: [Link<VmPage>; UVM_PMR_CACHEMAGSZ],
    /// `upci_npages`: # of pages in magazine.
    upci_npages: AtomicUsize,
}

#[cfg(feature = "multiprocessor")]
impl UvmPmrCacheItem {
    /// An empty magazine.
    pub const fn new() -> Self {
        Self {
            upci_pages: [const { Link::new() }; UVM_PMR_CACHEMAGSZ],
            upci_npages: AtomicUsize::new(0),
        }
    }

    /// `upci_npages`.
    pub fn npages(&self) -> usize {
        self.upci_npages.load(Ordering::Relaxed)
    }
}

#[cfg(feature = "multiprocessor")]
impl Default for UvmPmrCacheItem {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct uvm_pmr_cache`: per-CPU cache of physical pages. Only the owning CPU touches its
/// cache, at `splbio`.
#[cfg(feature = "multiprocessor")]
pub struct UvmPmrCache {
    /// `upc_magz`: magazines.
    upc_magz: [UvmPmrCacheItem; 2],
    /// `upc_actv`: index of active magazine.
    upc_actv: AtomicUsize,
}

#[cfg(feature = "multiprocessor")]
impl UvmPmrCache {
    /// Two empty magazines, the first one active.
    pub const fn new() -> Self {
        Self {
            upc_magz: [UvmPmrCacheItem::new(), UvmPmrCacheItem::new()],
            upc_actv: AtomicUsize::new(0),
        }
    }

    /// The two magazines.
    pub fn magazines(&self) -> &[UvmPmrCacheItem; 2] {
        &self.upc_magz
    }
}

#[cfg(feature = "multiprocessor")]
impl Default for UvmPmrCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "multiprocessor")]
impl PerCpuInit for UvmPmrCache {
    const INIT: Self = UvmPmrCache::new();
}

/// `uvm.fpageqlock` and what it guards.
static FREE: Mutexed<FreeState> = Mutexed::new(IPL_VM, FreeState::new());

/// Every CPU's page cache (`ci_uvm`, see the module's deviations).
#[cfg(feature = "multiprocessor")]
static UVM_PMR_CACHES: PerCpu<UvmPmrCache> = PerCpu::new();

/// The buffer cache's statistics, once registered.
static STATS: OnceStatic<&'static dyn BufCacheStats> = OnceStatic::new();

/// The buffer cache's page statistics the allocator reads to wake the page daemon early
/// (`<sys/mount.h>`'s `BUFPAGES_DEFICIT` and `BUFPAGES_INACT` over `bcstats`), still `bsd`'s:
/// a bridge (bridges.toml `BufCacheStats`, N5), no logic.
pub trait BufCacheStats: Sync {
    /// `BUFPAGES_DEFICIT`: how far the buffer cache is below its low water mark (0 or more).
    fn bufpages_deficit(&self) -> i64;
    /// `BUFPAGES_INACT`: the buffer cache's clean pages above its low water mark (0 or more).
    fn bufpages_inact(&self) -> i64;
}

/// Registers the buffer cache's statistics, once (`bsd`'s `uvm_page_init`); a second
/// registration keeps the first.
pub fn uvm_pmr_install_stats(s: &'static dyn BufCacheStats) -> Result<(), AlreadyRegistered> {
    STATS.set(s).map_err(|_| AlreadyRegistered("BufCacheStats"))
}

/// `uvm_lock_fpageq()`: the free page state, locked until the guard drops.
pub fn uvm_lock_fpageq() -> MutexedGuard<'static, FreeState> {
    FREE.lock()
}

/// Validate the flags of the page (used in asserts). Any free page must have the PQ_FREE flag
/// set. Free pages may be zeroed. Pmap flags are left untouched (they live in
/// `hal::pmap::PageMd` since N3.1). The PQ_FREE flag is not checked here: by not checking, we
/// can easily use this check in pages which are freed.
fn valid_flags(pg_flags: u32) -> bool {
    pg_flags & !(PQ_FREE | PG_ZERO) == 0
}

/// `in_pagedaemon`: whether the caller is the page daemon (or the syncer, when allowed).
fn in_pagedaemon(_allowsyncer: bool) -> bool {
    // curcpu()->ci_idepth, uvm.pagedaemon_proc and syncerproc arrive with N3.5 and N4.
    false
}

/// `uvm_pmr_pg_to_memtype`: memory types. The page flags are used to derive what the current
/// memory type of a page is.
pub fn uvm_pmr_pg_to_memtype(pg: &VmPage) -> usize {
    if pg.flags() & PG_ZERO != 0 {
        return UVM_PMR_MEMTYPE_ZERO;
    }
    // Default: dirty memory.
    UVM_PMR_MEMTYPE_DIRTY
}

/// Page number of a page.
fn pgno(pg: &VmPage) -> usize {
    vm_page_to_phys(pg).atop()
}

/// `pg->fpgsz` (\[F\]).
fn fpgsz(pg: &VmPage) -> usize {
    pg.fpgsz.load(Ordering::Relaxed)
}

/// `pg->fpgsz = n` (\[F\]).
fn set_fpgsz(pg: &VmPage, n: usize) {
    pg.fpgsz.store(n, Ordering::Relaxed);
}

/// Whether the range `prev` ends where `pg` starts in the page array (not "array broke").
fn follows_in_array(prev: &VmPage, pg: &VmPage) -> bool {
    prev.index() + fpgsz(prev) == pg.index()
}

/// `pow2divide`: computes num/denom and rounds it up to the next power-of-2.
///
/// This is a division function which calculates an approximation of num/denom, with result =~
/// num/denom. It is meant to be fast and doesn't have to be accurate.
///
/// Providing too large a value makes the allocator slightly faster, at the risk of hitting the
/// failure case more often. Providing too small a value makes the allocator a bit slower, but
/// less likely to hit a failure case.
pub fn pow2divide(num: usize, denom: usize) -> usize {
    let mut rshift = 0;
    let mut denom = denom;
    while num > denom {
        rshift += 1;
        denom <<= 1;
    }
    1 << rshift
}

/// `PMR_IS_SUBRANGE_OF`: lhs is a subrange of rhs. If rhs_low == 0: don't care about lower
/// bound. If rhs_high == 0: don't care about upper bound.
fn pmr_is_subrange_of(lhs_low: usize, lhs_high: usize, rhs_low: usize, rhs_high: usize) -> bool {
    (rhs_low == 0 || lhs_low >= rhs_low) && (rhs_high == 0 || lhs_high <= rhs_high)
}

/// `PMR_INTERSECTS_WITH`: lhs intersects with rhs. If rhs_low == 0: don't care about lower
/// bound. If rhs_high == 0: don't care about upper bound. Ranges don't intersect if they don't
/// have any page in common, array semantics mean that < instead of <= should be used here.
fn pmr_intersects_with(lhs_low: usize, lhs_high: usize, rhs_low: usize, rhs_high: usize) -> bool {
    (rhs_low == 0 || rhs_low < lhs_high) && (rhs_high == 0 || lhs_low < rhs_high)
}

/// `PMR_ALIGN`: align to power-of-2 alignment.
fn pmr_align(pgno: usize, align: usize) -> usize {
    (pgno + (align - 1)) & !(align - 1)
}

/// `PMR_ALIGN_DOWN`.
fn pmr_align_down(pgno: usize, align: usize) -> usize {
    pgno & !(align - 1)
}

/// `uvm_pmemrange_addr_cmp`: sort by address ascending.
pub fn uvm_pmemrange_addr_cmp(lhs: &UvmPmemrange, rhs: &UvmPmemrange) -> Cmp {
    lhs.low().cmp(&rhs.low())
}

/// `uvm_pmemrange_use_cmp`: sort by use ascending. The higher the use value of a range, the
/// more devices need memory in this range. Therefore allocate from the range with the lowest
/// use first.
pub fn uvm_pmemrange_use_cmp(lhs: &UvmPmemrange, rhs: &UvmPmemrange) -> Cmp {
    lhs.use_count()
        .cmp(&rhs.use_count())
        .then_with(|| uvm_pmemrange_addr_cmp(lhs, rhs))
}

/// `uvm_pmr_addr_cmp`: pages by physical address.
pub fn uvm_pmr_addr_cmp(lhs: &VmPage, rhs: &VmPage) -> Cmp {
    vm_page_to_phys(lhs)
        .as_usize()
        .cmp(&vm_page_to_phys(rhs).as_usize())
}

/// `uvm_pmr_size_cmp`: ranges by size, then address. Using second tree, so we receive `pg[1]`
/// instead of `pg[0]`.
pub fn uvm_pmr_size_cmp(lhs: &VmPage, rhs: &VmPage) -> Cmp {
    let (lhs0, rhs0) = (nth_back(lhs, 1), nth_back(rhs, 1));
    fpgsz(lhs0)
        .cmp(&fpgsz(rhs0))
        .then_with(|| uvm_pmr_addr_cmp(lhs0, rhs0))
}

/// `uvm_pmr_nfindsz`: find the first range of free pages that is at least sz pages long.
pub fn uvm_pmr_nfindsz(
    _fs: &FreeState,
    pmr: &UvmPmemrange,
    sz: usize,
    mti: usize,
) -> Option<&'static VmPage> {
    kassert!(sz >= 1);

    if sz == 1 && !pmr.single[mti].is_empty() {
        return pmr.single[mti].first();
    }

    let mut node = pmr.size[mti].root();
    let mut best = None;
    while let Some(n) = node {
        let range = nth_back(n, 1);
        if fpgsz(range) >= sz {
            best = Some(range);
            node = Tree::<UvmPmrSize>::left(n);
        } else {
            node = Tree::<UvmPmrSize>::right(n);
        }
    }
    best
}

/// `uvm_pmr_nextsz`: finds the next range. The next range has a size >= pg->fpgsz. Returns
/// `None` if no more ranges are available.
pub fn uvm_pmr_nextsz(
    _fs: &FreeState,
    pmr: &UvmPmemrange,
    pg: &VmPage,
    mt: usize,
) -> Option<&'static VmPage> {
    let npg = if fpgsz(pg) == 1 {
        match Pglist::next(pg) {
            Some(next) => return Some(next),
            None => pmr.size[mt].min(),
        }
    } else {
        Tree::<UvmPmrSize>::next(nth(pg, 1))
    };

    npg.map(|n| nth_back(n, 1))
}

/// `uvm_pmr_pnaddr`: finds the previous and next ranges relative to the (uninserted) pg range.
/// `prev` is `None` if no previous range is available that can join with pg; `next` likewise.
pub fn uvm_pmr_pnaddr(
    _fs: &FreeState,
    pmr: &UvmPmemrange,
    pg: &VmPage,
) -> (Option<&'static VmPage>, Option<&'static VmPage>) {
    let mut pg_next = pmr.addr.nfind(pg);
    let mut pg_prev = match pg_next {
        None => pmr.addr.max(),
        Some(next) => Tree::<UvmPmrAddr>::prev(next),
    };

    kdassert!(
        pg_next.is_none_or(|n| vm_page_to_phys(n).as_usize() > vm_page_to_phys(pg).as_usize())
    );
    kdassert!(
        pg_prev.is_none_or(|p| vm_page_to_phys(p).as_usize() < vm_page_to_phys(pg).as_usize())
    );

    // Reset if not contig.
    if let Some(prev) = pg_prev
        && (pgno(prev) + fpgsz(prev) != pgno(pg)
            || !follows_in_array(prev, pg) // Array broke.
            || uvm_pmr_pg_to_memtype(prev) != uvm_pmr_pg_to_memtype(pg))
    {
        pg_prev = None;
    }
    if let Some(next) = pg_next
        && (pgno(pg) + fpgsz(pg) != pgno(next)
            || !follows_in_array(pg, next) // Array broke.
            || uvm_pmr_pg_to_memtype(next) != uvm_pmr_pg_to_memtype(pg))
    {
        pg_next = None;
    }
    (pg_prev, pg_next)
}

/// `uvm_pmr_remove_addr`: remove a range from the address tree. Address tree maintains pmr
/// counters.
pub fn uvm_pmr_remove_addr(_fs: &FreeState, pmr: &UvmPmemrange, pg: &VmPage) {
    kdassert!(pmr.addr.find(pg).is_some_and(|f| core::ptr::eq(f, pg)));
    kdassert!(pg.flags() & PQ_FREE != 0);
    pmr.addr.remove(pg);

    pmr.set_nsegs(pmr.nsegs() - 1);
}

/// `uvm_pmr_remove_size`: remove a range from the size tree.
pub fn uvm_pmr_remove_size(_fs: &FreeState, pmr: &UvmPmemrange, pg: &VmPage) {
    kdassert!(fpgsz(pg) >= 1);
    kdassert!(pg.flags() & PQ_FREE != 0);
    let memtype = uvm_pmr_pg_to_memtype(pg);

    if fpgsz(pg) == 1 {
        #[cfg(feature = "debug")]
        kdassert!(pmr.single[memtype].iter().any(|i| core::ptr::eq(i, pg)));
        pmr.single[memtype].remove(pg);
    } else {
        let node = nth(pg, 1);
        kdassert!(
            pmr.size[memtype]
                .find(node)
                .is_some_and(|f| core::ptr::eq(f, node))
        );
        pmr.size[memtype].remove(node);
    }
}

/// `uvm_pmr_remove`: remove from both trees.
pub fn uvm_pmr_remove(fs: &FreeState, pmr: &UvmPmemrange, pg: &VmPage) {
    uvm_pmr_assertvalid(fs, pmr);
    uvm_pmr_remove_size(fs, pmr, pg);
    uvm_pmr_remove_addr(fs, pmr, pg);
    uvm_pmr_assertvalid(fs, pmr);
}

/// `uvm_pmr_insert_addr`: insert the range described in pg. Returns the range thus created
/// (which may be joined with the previous and next ranges). If `no_join`, the caller
/// guarantees that the range cannot possibly join with adjacent ranges.
pub fn uvm_pmr_insert_addr(
    fs: &FreeState,
    pmr: &UvmPmemrange,
    pg: &'static VmPage,
    no_join: bool,
) -> &'static VmPage {
    kdassert!(pg.flags() & PQ_FREE != 0);
    kdassert!(fpgsz(pg) >= 1);

    #[cfg(feature = "debug")]
    for mt in 0..UVM_PMR_MEMTYPE_MAX {
        kdassert!(!pmr.single[mt].iter().any(|i| core::ptr::eq(i, pg)));
        if fpgsz(pg) > 1 {
            kdassert!(pmr.size[mt].find(nth(pg, 1)).is_none());
        }
        kdassert!(pmr.addr.find(pg).is_none());
    }

    if !no_join {
        let (prev, next) = uvm_pmr_pnaddr(fs, pmr, pg);
        if let Some(next) = next {
            uvm_pmr_remove_size(fs, pmr, next);
            uvm_pmr_remove_addr(fs, pmr, next);
            set_fpgsz(pg, fpgsz(pg) + fpgsz(next));
            set_fpgsz(next, 0);
        }
        if let Some(prev) = prev {
            uvm_pmr_remove_size(fs, pmr, prev);
            set_fpgsz(prev, fpgsz(prev) + fpgsz(pg));
            set_fpgsz(pg, 0);
            return prev;
        }
    }

    let dupe = pmr.addr.insert(pg);
    kassert!(dupe.is_none());
    let _ = dupe;

    pmr.set_nsegs(pmr.nsegs() + 1);

    pg
}

/// `uvm_pmr_insert_size`: insert the range described in pg into the size tree. Page must
/// already be in the address tree.
pub fn uvm_pmr_insert_size(_fs: &FreeState, pmr: &UvmPmemrange, pg: &'static VmPage) {
    kdassert!(fpgsz(pg) >= 1);
    kdassert!(pg.flags() & PQ_FREE != 0);

    let memtype = uvm_pmr_pg_to_memtype(pg);
    #[cfg(feature = "debug")]
    {
        for mti in 0..UVM_PMR_MEMTYPE_MAX {
            kdassert!(!pmr.single[mti].iter().any(|i| core::ptr::eq(i, pg)));
            if fpgsz(pg) > 1 {
                kdassert!(pmr.size[mti].find(nth(pg, 1)).is_none());
            }
            kdassert!(pmr.addr.find(pg).is_some_and(|f| core::ptr::eq(f, pg)));
        }
        for i in 0..fpgsz(pg) {
            kassert!(uvm_pmr_pg_to_memtype(nth(pg, i)) == memtype);
        }
    }

    if fpgsz(pg) == 1 {
        pmr.single[memtype].insert_tail(pg);
    } else {
        let dupe = pmr.size[memtype].insert(nth(pg, 1));
        kassert!(dupe.is_none());
        let _ = dupe;
    }
}

/// `uvm_pmr_insert`: insert in both trees.
pub fn uvm_pmr_insert(
    fs: &FreeState,
    pmr: &UvmPmemrange,
    pg: &'static VmPage,
    no_join: bool,
) -> &'static VmPage {
    uvm_pmr_assertvalid(fs, pmr);
    let pg = uvm_pmr_insert_addr(fs, pmr, pg, no_join);
    uvm_pmr_insert_size(fs, pmr, pg);
    uvm_pmr_assertvalid(fs, pmr);
    pg
}

/// `uvm_pmr_findnextsegment`: find the last page that is part of this segment.
///
/// - `pg`: the range at which to start the search.
/// - `boundary`: the page number boundary specification (0 = no boundary).
/// - `pmr`: the pmemrange of the page.
///
/// This function returns 1 before the next range, so if you want to have the next range, you
/// need to run `TAILQ_NEXT(result, pageq)` after calling. The reason is that this way, the
/// length of the segment is easily calculated using: `atop(result) - atop(pg) + 1`. Hence
/// this function also never returns NULL.
pub fn uvm_pmr_findnextsegment(
    pmr: &UvmPmemrange,
    pg: &'static VmPage,
    boundary: usize,
) -> &'static VmPage {
    kdassert!(pmr.low() <= pgno(pg) && pmr.high() > pgno(pg));
    let first_boundary = if boundary != 0 {
        pmr_align(pgno(pg) + 1, boundary)
    } else {
        0
    };

    // Increase next until it hits the first page of the next segment.
    //
    // While loop checks the following:
    // - next != NULL: we have not reached the end of pgl
    // - boundary == 0 || next < first_boundary: we do not cross a boundary
    // - atop(prev) + 1 == atop(next): still in the same segment
    // - low <= last, high > last: still in the same memory range
    // - memtype is equal: allocator is unable to view different memtypes as part of the same
    //   segment
    // - prev + 1 == next: no array breakage occurs
    let mut prev = pg;
    let mut next = Pglist::next(prev);
    while let Some(n) = next
        && (boundary == 0 || pgno(n) < first_boundary)
        && pgno(prev) + 1 == pgno(n)
        && pmr.low() <= pgno(n)
        && pmr.high() > pgno(n)
        && uvm_pmr_pg_to_memtype(prev) == uvm_pmr_pg_to_memtype(n)
        && prev.index() + 1 == n.index()
    {
        prev = n;
        next = Pglist::next(prev);
    }

    // End of this segment.
    prev
}

/// `uvm_pmr_findprevsegment`: find the first page that is part of this segment.
///
/// This function returns 1 after the previous range, so if you want to have the previous
/// range, you need to run `TAILQ_NEXT(result, pageq)` after calling. The reason is that this
/// way, the length of the segment is easily calculated using: `atop(pg) - atop(result) + 1`.
/// Hence this function also never returns NULL.
pub fn uvm_pmr_findprevsegment(
    pmr: &UvmPmemrange,
    pg: &'static VmPage,
    boundary: usize,
) -> &'static VmPage {
    kdassert!(pmr.low() <= pgno(pg) && pmr.high() > pgno(pg));
    let first_boundary = if boundary != 0 {
        pmr_align_down(pgno(pg), boundary)
    } else {
        0
    };

    // Increase next until it hits the first page of the previous segment (the checks mirror
    // uvm_pmr_findnextsegment's, walking down).
    let mut prev = pg;
    let mut next = Pglist::next(prev);
    while let Some(n) = next
        && (boundary == 0 || pgno(n) >= first_boundary)
        && pgno(prev).wrapping_sub(1) == pgno(n)
        && pmr.low() <= pgno(n)
        && pmr.high() > pgno(n)
        && uvm_pmr_pg_to_memtype(prev) == uvm_pmr_pg_to_memtype(n)
        && prev.index().wrapping_sub(1) == n.index()
    {
        prev = n;
        next = Pglist::next(prev);
    }

    // Start of this segment.
    prev
}

/// `uvm_pmr_remove_1strange`: remove the first segment of contiguous pages from pgl. A
/// segment ends if it crosses boundary (unless boundary = 0) or if it would enter a different
/// uvm_pmemrange.
///
/// `work`: the page range that the caller is currently working with. May be `None`.
///
/// If `is_desperate`, the smallest segment is erased. Otherwise, the first segment is erased
/// (which, if called by `uvm_pmr_getpages`, probably is the smallest or very close to it).
/// Returns the number of pages removed.
pub fn uvm_pmr_remove_1strange(
    fs: &FreeState,
    pgl: &Pglist,
    boundary: usize,
    work: Option<&mut Option<&'static VmPage>>,
    is_desperate: bool,
) -> usize {
    kassert!(!pgl.is_empty());

    // Initialize to first page. Unless desperate scan finds a better candidate, this is
    // what'll be erased.
    let Some(mut start) = pgl.first() else {
        return 0;
    };
    let Some(mut pmr) = uvm_pmemrange_find(fs, pgno(start)) else {
        return 0;
    };
    let mut end = uvm_pmr_findnextsegment(pmr, start, boundary);

    // If we are desperate, we _really_ want to get rid of the smallest element (rather than a
    // close match to the smallest element).
    if is_desperate {
        // Linear search for smallest segment.
        let mut pmr_iter = pmr;
        let mut iter = Pglist::next(end);
        while let Some(it) = iter
            && !core::ptr::eq(start, end)
        {
            // Only update pmr if it doesn't match current iteration.
            if pmr.low() > pgno(it) || pmr.high() <= pgno(it) {
                let Some(found) = uvm_pmemrange_find(fs, pgno(it)) else {
                    break;
                };
                pmr_iter = found;
            }

            let iter_end = uvm_pmr_findnextsegment(pmr_iter, it, boundary);

            // Current iteration is smaller than best match so far; update.
            if vm_page_to_phys(iter_end).as_usize() - vm_page_to_phys(it).as_usize()
                < vm_page_to_phys(end).as_usize() - vm_page_to_phys(start).as_usize()
            {
                start = it;
                end = iter_end;
                pmr = pmr_iter;
            }
            iter = Pglist::next(iter_end);
        }
    }

    // Calculate count and end of the list.
    let count = atop(vm_page_to_phys(end).as_usize() - vm_page_to_phys(start).as_usize()) + 1;
    let lowest = start;
    let stop = Pglist::next(end);

    // Actually remove the range of pages.
    //
    // Sadly, this cannot be done using pointer iteration: vm_physseg is not guaranteed to be
    // sorted on address, hence uvm_page_init() may not have initialized its array sorted by
    // page number.
    let mut iter = Some(start);
    while let Some(it) = iter
        && !stop.is_some_and(|s| core::ptr::eq(s, it))
    {
        let iter_end = Pglist::next(it);
        pgl.remove(it);
        iter = iter_end;
    }

    set_fpgsz(lowest, count);
    let inserted = uvm_pmr_insert(fs, pmr, lowest, false);

    // If the caller was working on a range and this function modified that range, update the
    // pointer.
    if let Some(work) = work
        && let Some(w) = *work
        && pgno(inserted) <= pgno(w)
        && pgno(inserted) + fpgsz(inserted) > pgno(w)
    {
        *work = Some(inserted);
    }
    count
}

/// `uvm_pmr_remove_1strange_reverse`: remove the first segment of contiguous pages from a pgl
/// with the list elements in reverse order of physaddr. A segment ends if it would enter a
/// different uvm_pmemrange. Returns the count and the starting physical address of the
/// segment.
pub fn uvm_pmr_remove_1strange_reverse(fs: &FreeState, pgl: &Pglist) -> (usize, Paddr) {
    kassert!(!pgl.is_empty());

    let Some(start) = pgl.first() else {
        return (0, Paddr::new(0));
    };
    let Some(pmr) = uvm_pmemrange_find(fs, pgno(start)) else {
        return (0, Paddr::new(0));
    };
    let end = uvm_pmr_findprevsegment(pmr, start, 0);

    kassert!(end.index() <= start.index());

    // Calculate count and end of the list.
    let count = atop(vm_page_to_phys(start).as_usize() - vm_page_to_phys(end).as_usize()) + 1;
    let lowest = end;
    let stop = Pglist::next(end);

    // Actually remove the range of pages (see uvm_pmr_remove_1strange for why it walks the
    // list).
    let mut iter = Some(start);
    while let Some(it) = iter
        && !stop.is_some_and(|s| core::ptr::eq(s, it))
    {
        let iter_end = Pglist::next(it);
        pgl.remove(it);
        iter = iter_end;
    }

    set_fpgsz(lowest, count);
    let _ = uvm_pmr_insert(fs, pmr, lowest, false);

    (count, vm_page_to_phys(lowest))
}

/// `uvm_pmr_extract_range`: extract a number of pages from a segment of free pages. Called by
/// `uvm_pmr_getpages`. Returns the segment that was created from pages left over at the tail
/// of the remove set of pages, or `None` if no pages were left at the tail.
pub fn uvm_pmr_extract_range(
    fs: &FreeState,
    pmr: &UvmPmemrange,
    pg: &'static VmPage,
    start: usize,
    end: usize,
    result: &Pglist,
) -> Option<&'static VmPage> {
    kdassert!(end > start);
    kdassert!(pmr.low() <= pgno(pg));
    kdassert!(pmr.high() >= pgno(pg) + fpgsz(pg));
    kdassert!(pgno(pg) <= start);
    kdassert!(pgno(pg) + fpgsz(pg) >= end);

    let before_sz = start - pgno(pg);
    let after_sz = pgno(pg) + fpgsz(pg) - end;
    kdassert!(before_sz + after_sz + (end - start) == fpgsz(pg));
    uvm_pmr_assertvalid(fs, pmr);

    uvm_pmr_remove_size(fs, pmr, pg);
    if before_sz == 0 {
        uvm_pmr_remove_addr(fs, pmr, pg);
    }

    // Add selected pages to result.
    for i in before_sz..before_sz + (end - start) {
        let pg_i = nth(pg, i);
        kassert!(pg_i.flags() & PQ_FREE != 0);
        set_fpgsz(pg_i, 0);
        result.insert_tail(pg_i);
    }

    // Before handling.
    if before_sz > 0 {
        set_fpgsz(pg, before_sz);
        uvm_pmr_insert_size(fs, pmr, pg);
    }

    // After handling.
    if after_sz > 0 {
        let after = nth(pg, before_sz + (end - start));
        #[cfg(feature = "debug")]
        for i in 0..after_sz {
            kassert!(!uvm_pmr_isfree(fs, nth(after, i)));
        }
        kdassert!(pgno(after) == end);
        set_fpgsz(after, after_sz);
        let after = uvm_pmr_insert_addr(fs, pmr, after, true);
        uvm_pmr_insert_size(fs, pmr, after);
        uvm_pmr_assertvalid(fs, pmr);
        return Some(after);
    }

    uvm_pmr_assertvalid(fs, pmr);
    None
}

/// `uvm_pmr_getpages`: acquire a number of pages.
///
/// - `count`: the number of pages returned
/// - `start`: lowest page number
/// - `end`: highest page number +1 (start = end = 0: no limitation)
/// - `align`: power-of-2 alignment constraint (align = 1: no alignment)
/// - `boundary`: power-of-2 boundary (boundary = 0: no boundary)
/// - `maxseg`: maximum number of segments to return
/// - `flags`: `UVM_PLA_*` flags
/// - `result`: returned pages storage (uses pageq)
#[allow(clippy::too_many_arguments, reason = "the C signature")]
pub fn uvm_pmr_getpages(
    count: usize,
    start: usize,
    end: usize,
    align: usize,
    boundary: usize,
    maxseg: i32,
    flags: i32,
    result: &Pglist,
) -> Result<(), Errno> {
    // Validate arguments.
    kassert!(count > 0);
    kassert!(start == 0 || end == 0 || start < end);
    kassert!(align >= 1);
    kassert!(align.is_power_of_two());
    kassert!(maxseg > 0);
    kassert!(boundary == 0 || boundary.is_power_of_two());
    kassert!(boundary == 0 || (maxseg as usize) * boundary >= count);
    kassert!(result.is_empty());
    kassert!((flags & UVM_PLA_WAITOK == 0) ^ (flags & UVM_PLA_NOWAIT == 0));

    let mut flags = flags;
    let maxseg = maxseg as usize;

    // TRYCONTIG is a noop if you only want a single segment. Remove it if that's the case:
    // otherwise it'll deny the fast allocation.
    if maxseg == 1 || count == 1 {
        flags &= !UVM_PLA_TRYCONTIG;
    }

    // Configure search.
    //
    // search[0] is one segment, only used in UVM_PLA_TRYCONTIG case.
    // search[1] is multiple segments, chosen to fulfill the search in approximately even-sized
    //   segments. This is a good trade-off between slightly reduced allocation speed and less
    //   fragmentation.
    // search[2] is the worst case, in which all segments are evaluated. This provides the
    //   least fragmentation, but makes the search possibly longer (although in the case it is
    //   selected, that no longer matters most).
    //
    // The exception is when maxseg == 1: since we can only fulfill that with one segment of
    // size pages, only a single search type has to be attempted.
    let mut search = [0usize; 3];
    let mut start_try;
    if maxseg == 1 || count == 1 {
        start_try = 2;
        search[2] = count;
    } else if maxseg >= count && flags & UVM_PLA_TRYCONTIG == 0 {
        start_try = 2;
        search[2] = 1;
    } else {
        start_try = 0;
        search[0] = count;
        search[1] = pow2divide(count, maxseg);
        search[2] = 1;
        if flags & UVM_PLA_TRYCONTIG == 0 {
            start_try = 1;
        }
        if search[1] >= search[0] {
            search[1] = search[0];
            start_try = 1;
        }
        if search[2] >= search[start_try] {
            start_try = 2;
        }
    }

    // Memory type: if zeroed memory is requested, traverse the zero set. Otherwise, traverse
    // the dirty set. The memtype iterator is reinitialized to memtype_init on entrance of a
    // pmemrange.
    let memtype_init = if flags & UVM_PLA_ZERO != 0 {
        UVM_PMR_MEMTYPE_ZERO
    } else {
        UVM_PMR_MEMTYPE_DIRTY
    };

    // Initially, we're not desperate. Note that if we return from a sleep, we are still
    // desperate. Chances are that memory pressure is still high, so resetting seems
    // over-optimistic to me.
    let mut desperate = false;

    let fs = FREE.lock();
    // retry: the return point after sleeping is not reachable (uvm_wait is not here).

    // check to see if we need to generate some free pages waking the pagedaemon.
    let (deficit, inact) = STATS
        .get()
        .map_or((0, 0), |s| (s.bufpages_deficit(), s.bufpages_inact()));
    let free = i64::from(UVMEXP.free.load(Ordering::Relaxed));
    if (free - deficit) < i64::from(UVMEXP.freemin.load(Ordering::Relaxed))
        || ((free - deficit) < i64::from(UVMEXP.freetarg.load(Ordering::Relaxed))
            && (i64::from(UVMEXP.inactive.load(Ordering::Relaxed)) + inact)
                < i64::from(UVMEXP.inactarg.load(Ordering::Relaxed)))
    {
        wakeup(page::pagedaemon_chan());
    }

    // fail if any of these conditions is true:
    // [1] there really are no free pages, or
    // [2] only kernel "reserved" pages remain and the UVM_PLA_USERESERVE flag wasn't used.
    // [3] only pagedaemon "reserved" pages remain and the requestor isn't the pagedaemon nor
    //     the syncer.
    let free = i64::from(UVMEXP.free.load(Ordering::Relaxed));
    if free <= i64::from(UVMEXP.reserve_kernel.load(Ordering::Relaxed)) + count as i64
        && flags & UVM_PLA_USERESERVE == 0
    {
        return Err(Errno::ENOMEM);
    }

    if free <= i64::from(UVMEXP.reserve_pagedaemon.load(Ordering::Relaxed)) + count as i64
        && !in_pagedaemon(true)
    {
        // UVM_PLA_WAITOK: uvm_wait("uvm_pmr_getpages") then goto retry; without a page
        // daemon the wait cannot be satisfied (module deviations).
        return Err(Errno::ENOMEM);
    }

    let mut fcount = 0usize;
    let mut fnsegs = 0usize;

    // retry_desperate / fail / out: the labelled block ends with the C's exit taken.
    let found_all = 'retry_desperate: loop {
        // If we just want any page(s), go for the really fast option.
        if count <= maxseg && align == 1 && boundary == 0 && flags & UVM_PLA_TRYCONTIG == 0 {
            fcount +=
                uvm_pmr_get1page(&fs, count - fcount, memtype_init, result, start, end, false);

            // If we found sufficient pages, go to the success exit code. Otherwise, go
            // immediately to fail, since we collected all we could anyway.
            break 'retry_desperate fcount == count;
        }

        // The heart of the contig case.
        //
        // foreach (struct pmemrange) {
        //	foreach (memtype) {
        //		foreach(try) {
        //			foreach (free range of memtype in pmemrange, starting at
        //			    search[try]) {
        //				while (range has space left)
        //					take from range
        //			}
        //		}
        //	}
        //	if next pmemrange has higher usecount than current:
        //		enter desperate case (which will drain the pmemranges until empty
        //		prior to moving to the next one)
        // }
        //
        // When desperate is activated, try always starts at the highest value.
        let mut pmr_iter = fs.control.r#use.first();
        'pmr: while let Some(pmr) = pmr_iter {
            pmr_iter = List::<PmrUse>::next(pmr);

            // Empty range.
            if pmr.nsegs() == 0 {
                continue;
            }

            // Outside requested range.
            if !pmr_intersects_with(pmr.low(), pmr.high(), start, end) {
                continue;
            }

            let mut memtype = memtype_init;

            'rescan_memtype: loop {
                // Return point at memtype++.
                let mut r#try = start_try;

                'rescan: loop {
                    // Return point at try++.
                    let mut found = uvm_pmr_nfindsz(&fs, pmr, search[r#try], memtype);
                    while let Some(mut f) = found {
                        let f_next = uvm_pmr_nextsz(&fs, pmr, f, memtype);

                        let mut fstart = pgno(f);
                        if start != 0 {
                            fstart = start.max(fstart);
                        }
                        'drain_found: loop {
                            // Throw away the first segment if fnsegs == maxseg.
                            //
                            // Note that f_next is still valid after this call, since we only
                            // allocated from entries before f_next. We don't revisit the
                            // entries we already extracted from unless we entered the
                            // desperate case.
                            if fnsegs == maxseg {
                                fnsegs -= 1;
                                let mut work = Some(f);
                                fcount -= uvm_pmr_remove_1strange(
                                    &fs,
                                    result,
                                    boundary,
                                    Some(&mut work),
                                    desperate,
                                );
                                if let Some(w) = work {
                                    f = w;
                                }
                            }

                            fstart = pmr_align(fstart, align);
                            let mut fend = pgno(f) + fpgsz(f);
                            if end != 0 {
                                fend = end.min(fend);
                            }
                            if boundary != 0 {
                                fend = fend.min(pmr_align(fstart + 1, boundary));
                            }
                            if fstart >= fend {
                                break 'drain_found;
                            }
                            if fend - fstart > count - fcount {
                                fend = fstart + (count - fcount);
                            }

                            fcount += fend - fstart;
                            fnsegs += 1;
                            let after = uvm_pmr_extract_range(&fs, pmr, f, fstart, fend, result);

                            if fcount == count {
                                break 'retry_desperate true;
                            }

                            // If there's still space left in found, try to fully drain it
                            // prior to continuing.
                            match after {
                                Some(a) => {
                                    f = a;
                                    fstart = fend;
                                }
                                None => break 'drain_found,
                            }
                        }
                        found = f_next;
                    }

                    // Try a smaller search now.
                    r#try += 1;
                    if r#try < search.len() {
                        continue 'rescan;
                    }
                    break 'rescan;
                }

                // Exhaust all memory types prior to going to the next memory segment. This
                // means that zero-vs-dirty are eaten prior to moving to a pmemrange with a
                // higher use-count.
                memtype += 1;
                if memtype == UVM_PMR_MEMTYPE_MAX {
                    memtype = 0;
                }
                if memtype != memtype_init {
                    continue 'rescan_memtype;
                }
                break 'rescan_memtype;
            }

            // If not desperate, enter desperate case prior to eating all the good stuff in
            // the next range.
            if !desperate
                && let Some(next) = pmr_iter
                && next.use_count() != pmr.use_count()
            {
                break 'pmr;
            }
        }

        // Not enough memory of the requested type available. Fall back to less good memory
        // that we'll clean up better later.
        //
        // This algorithm is not very smart though, it just starts scanning a different typed
        // range, but the nicer ranges of the previous iteration may fall out. Hence there is a
        // small chance of a false negative.
        //
        // When desperate: scan all sizes starting at the smallest (start_try = 1) and do not
        // consider UVM_PLA_TRYCONTIG (which may allow us to hit the fast path now).
        //
        // Also, because we will revisit entries we scanned before, we need to reset the page
        // queue, or we may end up releasing entries in such a way as to invalidate f_next.
        if !desperate {
            desperate = true;
            start_try = search.len() - 1;
            flags &= !UVM_PLA_TRYCONTIG;

            while !result.is_empty() {
                uvm_pmr_remove_1strange(&fs, result, 0, None, false);
            }
            fnsegs = 0;
            fcount = 0;
            continue 'retry_desperate;
        }
        break 'retry_desperate false;
    };

    if !found_all {
        // fail: allocation failed.
        // XXX: claim from memory reserve here

        while !result.is_empty() {
            uvm_pmr_remove_1strange(&fs, result, 0, None, false);
        }

        if flags & UVM_PLA_WAITOK != 0 {
            // uvm_wait("pmrwait") then goto retry: without a page daemon the wait cannot be
            // satisfied (module deviations).
            return Err(Errno::ENOMEM);
        }
        if flags & UVM_PLA_NOWAKE == 0 {
            wakeup(page::pagedaemon_chan());
        }
        return Err(Errno::ENOMEM);
    }

    // out: allocation successful.
    UVMEXP.free.fetch_sub(fcount as i32, Ordering::Relaxed);

    drop(fs);

    // Update statistics and zero pages if UVM_PLA_ZERO.
    #[cfg(feature = "diagnostic")]
    let (mut diag_fnsegs, mut diag_fcount, mut diag_prev): (usize, usize, Option<&VmPage>) =
        (0, 0, None);
    for found in result.iter() {
        found.mdpage.clear_attrs();

        if found.flags() & PG_ZERO != 0 {
            let fs = FREE.lock();
            UVMEXP.zeropages.fetch_sub(1, Ordering::Relaxed);
            if UVMEXP.zeropages.load(Ordering::Relaxed) < uvm_pagezero_target() {
                wakeup(core::ptr::from_ref(&UVMEXP.zeropages));
            }
            drop(fs);
        }
        if flags & UVM_PLA_ZERO != 0 {
            if found.flags() & PG_ZERO != 0 {
                UVMEXP.pga_zerohit.fetch_add(1, Ordering::Relaxed);
            } else {
                UVMEXP.pga_zeromiss.fetch_add(1, Ordering::Relaxed);
                uvm_pagezero(found);
            }
        }
        found.clear_bits(PG_ZERO | PQ_FREE);

        kassert!(!found.has_owner());
        found.pg_version.fetch_add(1, Ordering::Relaxed);

        // Validate that the page matches range criterium.
        kdassert!(start == 0 || pgno(found) >= start);
        kdassert!(end == 0 || pgno(found) < end);

        #[cfg(feature = "diagnostic")]
        {
            // Update fcount (# found pages) and fnsegs (# found segments) counters.
            if diag_prev.is_none_or(|prev| {
                // new segment if it contains a hole
                pgno(prev) + 1 != pgno(found)
                    // new segment if it crosses boundary
                    || (pgno(prev) & !boundary.wrapping_sub(1))
                        != (pgno(found) & !boundary.wrapping_sub(1))
            }) {
                diag_fnsegs += 1;
            }
            diag_fcount += 1;
            diag_prev = Some(found);
        }
    }

    #[cfg(feature = "diagnostic")]
    {
        // Panic on algorithm failure.
        if diag_fcount != count || diag_fnsegs > maxseg {
            kpanic(format_args!(
                "pmemrange allocation error: allocated {} pages in {} segments, but request was {} pages in {} segments",
                diag_fcount, diag_fnsegs, count, maxseg
            ));
        }
    }

    Ok(())
}

/// `uvm_pmr_getone`: acquire a single page.
pub fn uvm_pmr_getone(flags: i32) -> Option<&'static VmPage> {
    let pgl = Pglist::new();
    uvm_pmr_getpages(1, 0, 0, 1, 0, 1, flags, &pgl).ok()?;

    let pg = pgl.pop_head();
    kassert!(pg.is_some() && pgl.is_empty());

    pg
}

/// `uvm_pmr_freepages`: free a number of contig pages (invoked by `uvm_page_init`).
pub fn uvm_pmr_freepages(pg: &'static VmPage, count: usize) {
    for i in 0..count {
        let pgi = nth(pg, i);
        kassert!(!pgi.has_owner());

        kassert!(pgno(pgi) == pgno(pg) + i);

        if !(pgi.flags() & PQ_FREE == 0 && valid_flags(pgi.flags())) {
            let _ = writeln!(
                hal::cons::Cons,
                "Flags: {:#x}, will panic now.",
                pgi.flags()
            );
        }
        kassert!(pgi.flags() & PQ_FREE == 0 && valid_flags(pgi.flags()));
        pgi.set_bits(PQ_FREE);
        pgi.clear_bits(PG_ZERO);
    }

    let fs = FREE.lock();

    let mut pg = pg;
    let mut i = count;
    while i > 0 {
        let Some(pmr) = uvm_pmemrange_find(&fs, pgno(pg)) else {
            kassert!(false);
            break;
        };

        let pmr_count = i.min(pmr.high() - pgno(pg));
        set_fpgsz(pg, pmr_count);
        uvm_pmr_insert(&fs, pmr, pg, false);

        UVMEXP.free.fetch_add(pmr_count as i32, Ordering::Relaxed);
        i -= pmr_count;
        if i > 0 {
            pg = nth(pg, pmr_count);
        }
    }
    wakeup(core::ptr::from_ref(&UVMEXP.free));
    if UVMEXP.zeropages.load(Ordering::Relaxed) < uvm_pagezero_target() {
        wakeup(core::ptr::from_ref(&UVMEXP.zeropages));
    }
}

/// `uvm_pmr_freepageq`: free all pages in the queue.
pub fn uvm_pmr_freepageq(pgl: &Pglist) {
    for pg in pgl.iter() {
        kassert!(!pg.has_owner());

        if !(pg.flags() & PQ_FREE == 0 && valid_flags(pg.flags())) {
            let _ = writeln!(hal::cons::Cons, "Flags: {:#x}, will panic now.", pg.flags());
        }
        kassert!(pg.flags() & PQ_FREE == 0 && valid_flags(pg.flags()));
        pg.set_bits(PQ_FREE);
        pg.clear_bits(PG_ZERO);
    }

    let fs = FREE.lock();
    while let Some(pg) = pgl.first() {
        let plen = if Pglist::next(pg).is_some_and(|next| pg.index() == next.index() + 1) {
            // If pg is one behind the position of the next page in the list in the page
            // array, try going backwards instead of forward.
            uvm_pmr_remove_1strange_reverse(&fs, pgl).0
        } else {
            uvm_pmr_remove_1strange(&fs, pgl, 0, None, false)
        };
        UVMEXP.free.fetch_add(plen as i32, Ordering::Relaxed);
    }
    wakeup(core::ptr::from_ref(&UVMEXP.free));
    if UVMEXP.zeropages.load(Ordering::Relaxed) < uvm_pagezero_target() {
        wakeup(core::ptr::from_ref(&UVMEXP.zeropages));
    }
}

/// `uvm_pmemrange_use_insert`: store a pmemrange in the list. The list is sorted by use.
/// Returns the range already there when one compares equal.
pub fn uvm_pmemrange_use_insert(
    useq: &List<PmrUse>,
    pmr: &'static UvmPmemrange,
) -> Option<&'static UvmPmemrange> {
    let mut at = None;
    for iter in useq.iter() {
        match uvm_pmemrange_use_cmp(pmr, iter) {
            Cmp::Equal => return Some(iter),
            Cmp::Less => {
                at = Some(iter);
                break;
            }
            Cmp::Greater => {}
        }
    }

    match at {
        None => useq.insert_tail(pmr),
        Some(iter) => useq.insert_before(iter, pmr),
    }
    None
}

/// `uvm_pmr_assertvalid`: validation of the whole pmemrange. Called with fpageq locked.
#[cfg(any(feature = "debug", test))]
pub fn uvm_pmr_assertvalid(fs: &FreeState, pmr: &UvmPmemrange) {
    // Empty range
    if pmr.nsegs() == 0 {
        return;
    }

    // Validate address tree.
    for i in pmr.addr.iter() {
        // Validate the range.
        assert!(fpgsz(i) > 0);
        assert!(pgno(i) >= pmr.low());
        assert!(pgno(i) + fpgsz(i) <= pmr.high());

        // Validate each page in this range.
        for lcv in 0..fpgsz(i) {
            let p = nth(i, lcv);
            // Only the first page has a size specification. Rest is size 0.
            assert!(lcv == 0 || fpgsz(p) == 0);
            // Flag check.
            assert!(valid_flags(p.flags()) && p.flags() & PQ_FREE == PQ_FREE);
            // Free pages are: not wired, have no vm_anon, have no uvm_object.
            assert!(p.wire_count() == 0);
            assert!(!p.has_owner());
            // Pages in a single range always have the same memtype.
            assert!(uvm_pmr_pg_to_memtype(i) == uvm_pmr_pg_to_memtype(p));
        }

        // Check that it shouldn't be joined with its predecessor.
        if let Some(prev) = Tree::<UvmPmrAddr>::prev(i) {
            assert!(
                uvm_pmr_pg_to_memtype(i) != uvm_pmr_pg_to_memtype(prev)
                    || pgno(i) > pgno(prev) + fpgsz(prev)
                    || !follows_in_array(prev, i)
            );
        }

        // Assert i is in the size tree as well.
        if fpgsz(i) == 1 {
            assert!(
                pmr.single[uvm_pmr_pg_to_memtype(i)]
                    .iter()
                    .any(|xref| core::ptr::eq(xref, i))
            );
        } else {
            let node = nth(i, 1);
            assert!(
                pmr.size[uvm_pmr_pg_to_memtype(i)]
                    .find(node)
                    .is_some_and(|f| core::ptr::eq(f, node))
            );
        }
    }

    // Validate size tree.
    for mti in 0..UVM_PMR_MEMTYPE_MAX {
        let mut i = uvm_pmr_nfindsz(fs, pmr, 1, mti);
        while let Some(cur) = i {
            let next = uvm_pmr_nextsz(fs, pmr, cur, mti);
            if let Some(n) = next {
                assert!(fpgsz(cur) <= fpgsz(n));
            }

            // Assert i is in the addr tree as well.
            assert!(pmr.addr.find(cur).is_some_and(|f| core::ptr::eq(f, cur)));

            // Assert i is of the correct memory type.
            assert!(uvm_pmr_pg_to_memtype(cur) == mti);
            i = next;
        }
    }

    // Validate nsegs statistic.
    assert_eq!(pmr.nsegs(), pmr.addr.iter().count());
}

/// `uvm_pmr_assertvalid`: nothing without `DEBUG`.
#[cfg(not(any(feature = "debug", test)))]
pub fn uvm_pmr_assertvalid(_fs: &FreeState, _pmr: &UvmPmemrange) {}

/// `uvm_pmr_split`: split pmr at split point pageno. Called with fpageq unlocked. Split is
/// only applied if a pmemrange spans pageno.
pub fn uvm_pmr_split(pageno: usize) {
    let fs = FREE.lock();
    let needs_split = uvm_pmemrange_find(&fs, pageno).is_some_and(|pmr| pmr.low() < pageno);
    drop(fs);
    if !needs_split {
        // No split required.
        return;
    }

    // uvm_pmr_allocpmr() calls into malloc() which in turn calls into uvm_kmemalloc which
    // calls into pmemrange, making the locking a bit hard, so we just race!
    let drain = uvm_pmr_allocpmr();
    let fs = FREE.lock();
    let Some(pmr) = uvm_pmemrange_find(&fs, pageno).filter(|pmr| pmr.low() < pageno) else {
        // We lost the race since someone else ran this or a related function, however this
        // should be triggered very rarely so we just leak the pmr.
        let _ = writeln!(hal::cons::Cons, "uvm_pmr_split: lost one pmr");
        return;
    };

    kassert!(pmr.low() < pageno);
    kassert!(pmr.high() > pageno);

    drain.low.store(pageno, Ordering::Relaxed);
    drain.high.store(pmr.high(), Ordering::Relaxed);
    drain.r#use.store(pmr.use_count(), Ordering::Relaxed);

    uvm_pmr_assertvalid(&fs, pmr);
    uvm_pmr_assertvalid(&fs, drain);
    kassert!(drain.nsegs() == 0);

    let mut rebuild = pmr.addr.iter().find(|r| pgno(r) >= pageno);
    let prev = match rebuild {
        None => pmr.addr.max(),
        Some(r) => Tree::<UvmPmrAddr>::prev(r),
    };
    kassert!(prev.is_none_or(|p| pgno(p) < pageno));

    // Handle free chunk that spans the split point.
    if let Some(prev) = prev
        && pgno(prev) + fpgsz(prev) > pageno
    {
        kassert!(pgno(prev) < pageno);

        uvm_pmr_remove(&fs, pmr, prev);
        let prev_sz = fpgsz(prev);
        let before = pageno - pgno(prev);
        let after = pgno(prev) + prev_sz - pageno;

        kassert!(before > 0);
        kassert!(after > 0);

        set_fpgsz(prev, before);
        uvm_pmr_insert(&fs, pmr, prev, true);
        let tail = nth(prev, before);
        set_fpgsz(tail, after);
        uvm_pmr_insert(&fs, drain, tail, true);
    }

    // Move free chunks that no longer fall in the range.
    while let Some(r) = rebuild {
        let next = Tree::<UvmPmrAddr>::next(r);

        uvm_pmr_remove(&fs, pmr, r);
        uvm_pmr_insert(&fs, drain, r, true);
        rebuild = next;
    }

    pmr.high.store(pageno, Ordering::Relaxed);
    uvm_pmr_assertvalid(&fs, pmr);
    uvm_pmr_assertvalid(&fs, drain);

    let dupe = fs.control.addr.insert(drain);
    kassert!(dupe.is_none());
    let _ = dupe;
    uvm_pmemrange_use_insert(&fs.control.r#use, drain);
}

/// `uvm_pmr_use_inc`: increase the usage counter for the given range of memory. The more
/// usage counters a given range of memory has, the more will be attempted not to allocate
/// from it. Addresses here are in `paddr_t`, not page-numbers. The lowest and highest allowed
/// address are specified.
pub fn uvm_pmr_use_inc(low: Paddr, high: Paddr) {
    // pmr uses page numbers, translate low and high.
    let high = atop(trunc_page(high.as_usize().wrapping_add(1)));
    let low = atop(round_page(low.as_usize()));
    uvm_pmr_split(low);
    uvm_pmr_split(high);

    let mut sz = 0usize;
    let fs = FREE.lock();
    // Increase use count on segments in range.
    let mut it = fs.control.addr.min();
    while let Some(pmr) = it {
        it = Tree::<UvmPmemrangeAddr>::next(pmr);
        if pmr_is_subrange_of(pmr.low(), pmr.high(), low, high) {
            fs.control.r#use.remove(pmr);
            pmr.r#use.fetch_add(1, Ordering::Relaxed);
            sz += pmr.high() - pmr.low();
            uvm_pmemrange_use_insert(&fs.control.r#use, pmr);
        }
        uvm_pmr_assertvalid(&fs, pmr);
    }
    drop(fs);

    kassert!(sz >= high.wrapping_sub(low));
    let _ = sz;
}

/// `uvm_pmr_allocpmr`: allocate a pmemrange. If called from `uvm_page_init`, the boot
/// allocator is used (`hal::bootmem`, over `pmap_steal_memory`). If called after `uvm_init`,
/// malloc is used (and if called in between, you're dead): that path is not reachable here
/// (module deviations) and panics.
pub fn uvm_pmr_allocpmr() -> &'static UvmPmemrange {
    if page::page_init_done() {
        kpanic(format_args!(
            "uvm_pmr_allocpmr: no allocator after uvm_init yet"
        ));
    }
    // The C's memset + RBT_INIT/TAILQ_INIT: an empty range.
    bootmem::steal_one(UvmPmemrange::new)
}

/// `uvm_pmr_init`: initialization of pmr. Called by `uvm_page_init`. Sets up pmemranges: one
/// range for the entire address space, split and counted by the machine's DMA `constraints`.
pub fn uvm_pmr_init(constraints: &[&UvmConstraintRange]) {
    // By default, one range for the entire address space.
    let new_pmr = uvm_pmr_allocpmr();
    new_pmr.low.store(0, Ordering::Relaxed);
    new_pmr.high.store(atop(usize::MAX) + 1, Ordering::Relaxed);

    {
        let fs = FREE.lock();
        let dupe = fs.control.addr.insert(new_pmr);
        kassert!(dupe.is_none());
        let _ = dupe;
        uvm_pmemrange_use_insert(&fs.control.r#use, new_pmr);
    }

    for c in constraints {
        uvm_pmr_use_inc(c.ucr_low, c.ucr_high);
    }
}

/// `uvm_pmemrange_find`: find the pmemrange that contains the given page number. (Manually
/// traverses the binary tree, because that is cheaper on stack usage.)
pub fn uvm_pmemrange_find(fs: &FreeState, pageno: usize) -> Option<&'static UvmPmemrange> {
    let mut pmr = fs.control.addr.root();
    while let Some(p) = pmr {
        if p.low() > pageno {
            pmr = Tree::<UvmPmemrangeAddr>::left(p);
        } else if p.high() <= pageno {
            pmr = Tree::<UvmPmemrangeAddr>::right(p);
        } else {
            return Some(p);
        }
    }
    None
}

/// `uvm_pmr_isfree`: return true if the given page is in any of the free lists. Used by
/// `uvm_page_printit`. This function is safe, even if the page is not on the freeq. Note:
/// does not apply locking, only called from ddb (here: under the caller's guard).
pub fn uvm_pmr_isfree(fs: &FreeState, pg: &VmPage) -> bool {
    let Some(pmr) = uvm_pmemrange_find(fs, pgno(pg)) else {
        return false;
    };
    let r = match pmr.addr.nfind(pg) {
        None => pmr.addr.max(),
        Some(r) if !core::ptr::eq(r, pg) => Tree::<UvmPmrAddr>::prev(r),
        Some(r) => Some(r),
    };
    let Some(r) = r else {
        return false; // Empty tree.
    };

    kdassert!(pgno(r) <= pgno(pg));
    pgno(r) + fpgsz(r) > pgno(pg)
}

/// `uvm_pmr_rootupdate`: given a root of a tree, find a range which intersects start, end
/// and is of the same memtype. Page must be in the address tree.
pub fn uvm_pmr_rootupdate(
    _fs: &FreeState,
    init_root: &'static VmPage,
    start: usize,
    end: usize,
    memtype: usize,
) -> Option<&'static VmPage> {
    let mut root = Some(init_root);

    // Which direction to use for searching.
    let direction = if start != 0 && pgno(init_root) + fpgsz(init_root) <= start {
        1
    } else if end != 0 && pgno(init_root) >= end {
        -1
    } else {
        // nothing to do
        return Some(init_root);
    };

    // First, update root to fall within the chosen range.
    while let Some(r) = root
        && !pmr_intersects_with(pgno(r), pgno(r) + fpgsz(r), start, end)
    {
        root = if direction == 1 {
            Tree::<UvmPmrAddr>::right(r)
        } else {
            Tree::<UvmPmrAddr>::left(r)
        };
    }
    let root = root?;
    if uvm_pmr_pg_to_memtype(root) == memtype {
        return Some(root);
    }

    // Root is valid, but of the wrong memtype.
    //
    // Try to find a range that has the given memtype in the subtree (memtype mismatches are
    // costly, either because the conversion is expensive, or a later allocation will need to
    // do the opposite conversion, which will be expensive).
    //
    // First, simply increase address until we hit something we can use. Cache the upper page,
    // so we can page-walk later.
    let mut high = root;
    let mut high_next = Tree::<UvmPmrAddr>::right(high);
    while let Some(hn) = high_next
        && pmr_intersects_with(pgno(hn), pgno(hn) + fpgsz(hn), start, end)
    {
        high = hn;
        if uvm_pmr_pg_to_memtype(high) == memtype {
            return Some(high);
        }
        high_next = Tree::<UvmPmrAddr>::right(high);
    }

    // Second, decrease the address until we hit something we can use. Cache the lower page,
    // so we can page-walk later.
    let mut low = root;
    let mut low_next = Tree::<UvmPmrAddr>::left(low);
    while let Some(ln) = low_next
        && pmr_intersects_with(pgno(ln), pgno(ln) + fpgsz(ln), start, end)
    {
        low = ln;
        if uvm_pmr_pg_to_memtype(low) == memtype {
            return Some(low);
        }
        low_next = Tree::<UvmPmrAddr>::left(low);
    }

    if core::ptr::eq(low, high) {
        return None;
    }

    // No hits. Walk the address tree until we find something usable.
    let mut cur = Tree::<UvmPmrAddr>::next(low);
    while let Some(l) = cur
        && !core::ptr::eq(l, high)
    {
        kdassert!(pmr_is_subrange_of(pgno(l), pgno(l) + fpgsz(l), start, end));
        if uvm_pmr_pg_to_memtype(l) == memtype {
            return Some(l);
        }
        cur = Tree::<UvmPmrAddr>::next(l);
    }

    // Nothing found.
    None
}

/// `uvm_pmr_get1page`: allocate any page, the fastest way. Page number constraints only.
pub fn uvm_pmr_get1page(
    fs: &FreeState,
    count: usize,
    memtype_init: usize,
    result: &Pglist,
    start: usize,
    end: usize,
    memtype_only: bool,
) -> usize {
    let mut fcount = 0usize;
    let mut pmr_iter = fs.control.r#use.first();
    while let Some(pmr) = pmr_iter {
        pmr_iter = List::<PmrUse>::next(pmr);

        // We're done.
        if fcount == count {
            break;
        }

        // Outside requested range.
        if !(start == 0 && end == 0) && !pmr_intersects_with(pmr.low(), pmr.high(), start, end) {
            continue;
        }

        // Range is empty.
        if pmr.nsegs() == 0 {
            continue;
        }

        // Loop over all memtypes, starting at memtype_init.
        let mut memtype = memtype_init;
        while fcount != count {
            let mut found = pmr.single[memtype].first();
            // If found is outside the range, walk the list until we find something that
            // intersects with boundaries.
            while let Some(f) = found
                && !pmr_intersects_with(pgno(f), pgno(f) + 1, start, end)
            {
                found = Pglist::next(f);
            }

            if found.is_none() {
                // Check if the size tree contains a range that intersects with the
                // boundaries. As the allocation is for any page, try the smallest range so
                // that large ranges are preserved for more constrained cases. Only one entry
                // is checked here, to avoid a brute-force search.
                //
                // Note that a size tree gives pg[1] instead of pg[0].
                found = pmr.size[memtype].min().map(|n| nth_back(n, 1));
                if let Some(f) = found
                    && !pmr_intersects_with(pgno(f), pgno(f) + fpgsz(f), start, end)
                {
                    found = None;
                }
            }
            if found.is_none() {
                // Try address-guided search to meet the page number constraints.
                if let Some(root) = pmr.addr.root() {
                    found = uvm_pmr_rootupdate(fs, root, start, end, memtype);
                }
            }
            if let Some(f) = found {
                uvm_pmr_assertvalid(fs, pmr);
                uvm_pmr_remove_size(fs, pmr, f);

                // If the page intersects the end, then it'll need splitting.
                //
                // Note that we don't need to split if the page intersects start: the drain
                // function will simply stop on hitting start.
                if end != 0 && pgno(f) + fpgsz(f) > end {
                    let splitsz = pgno(f) + fpgsz(f) - end;

                    uvm_pmr_remove_addr(fs, pmr, f);
                    uvm_pmr_assertvalid(fs, pmr);
                    set_fpgsz(f, fpgsz(f) - splitsz);
                    let splitpg = nth(f, fpgsz(f));
                    set_fpgsz(splitpg, splitsz);
                    uvm_pmr_insert(fs, pmr, splitpg, true);

                    // At this point, splitpg and found actually should be joined. But we
                    // explicitly disable that, because we will start subtracting from found.
                    kassert!(start == 0 || pgno(f) + fpgsz(f) > start);
                    uvm_pmr_insert_addr(fs, pmr, f, true);
                }

                // Fetch pages from the end. If the range is larger than the requested number
                // of pages, this saves us an addr-tree update.
                //
                // Since we take from the end and insert at the head, any ranges keep
                // preserved.
                while fpgsz(f) > 0 && fcount < count && (start == 0 || pgno(f) + fpgsz(f) > start) {
                    set_fpgsz(f, fpgsz(f) - 1);
                    fcount += 1;
                    result.insert_head(nth(f, fpgsz(f)));
                }
                if fpgsz(f) > 0 {
                    uvm_pmr_insert_size(fs, pmr, f);
                    kdassert!(fcount == count);
                    uvm_pmr_assertvalid(fs, pmr);
                    return fcount;
                }

                // Delayed addr-tree removal.
                uvm_pmr_remove_addr(fs, pmr, f);
                uvm_pmr_assertvalid(fs, pmr);
            } else {
                if memtype_only {
                    break;
                }
                // Skip to the next memtype.
                memtype += 1;
                if memtype == UVM_PMR_MEMTYPE_MAX {
                    memtype = 0;
                }
                if memtype == memtype_init {
                    break;
                }
            }
        }
    }

    // Search finished.
    //
    // Ran out of ranges before enough pages were gathered, or we hit the case where
    // found->fpgsz == count - fcount, in which case the above exit condition didn't trigger.
    //
    // On failure, caller will free the pages.
    fcount
}

/// `uvm_pmr_print`: print information about pmemrange into `out` (ddb's `db_printf`, or the
/// console). Takes the free page lock.
pub fn uvm_pmr_print(out: &mut dyn Write) {
    let fs = FREE.lock();
    let _ = writeln!(out, "Ranges, use queue:");
    let mut useq_len = 0;
    for pmr in fs.control.r#use.iter() {
        useq_len += 1;
        let mut free = 0usize;
        let mut size = [0usize; UVM_PMR_MEMTYPE_MAX];
        for (mt, sz) in size.iter_mut().enumerate() {
            let pg = match pmr.size[mt].max() {
                Some(n) => Some(nth_back(n, 1)),
                None => pmr.single[mt].first(),
            };
            *sz = pg.map_or(0, fpgsz);

            for pg in pmr.addr.iter() {
                free += fpgsz(pg);
            }
        }

        let _ = write!(
            out,
            "* [{:#x}-{:#x}] use={} nsegs={}",
            pmr.low(),
            pmr.high(),
            pmr.use_count(),
            pmr.nsegs()
        );
        for (mt, sz) in size.iter().enumerate() {
            let _ = write!(out, " maxsegsz[{mt}]={sz:#x}");
        }
        let _ = writeln!(out, " free={free:#x}");
    }
    let _ = writeln!(out, "#ranges = {useq_len}");
}

/// This CPU's page cache (`&curcpu()->ci_uvm`, see the module's deviations).
#[cfg(feature = "multiprocessor")]
pub fn uvm_pmr_curcache() -> &'static UvmPmrCache {
    UVM_PMR_CACHES.get()
}

/// `uvm_pmr_cache_alloc`: fills the empty magazine `upci` from the allocator; false when the
/// allocator has no `UVM_PMR_CACHEMAGSZ` pages to spare.
#[cfg(feature = "multiprocessor")]
fn uvm_pmr_cache_alloc(upci: &UvmPmrCacheItem) -> bool {
    let flags = UVM_PLA_NOWAIT | UVM_PLA_NOWAKE;
    let npages = UVM_PMR_CACHEMAGSZ;

    hal::intr::splassert(IPL_BIO, "uvm_pmr_cache_alloc");
    kassert!(upci.npages() == 0);

    let pgl = Pglist::new();
    if uvm_pmr_getpages(npages, 0, 0, 1, 0, npages as i32, flags, &pgl).is_err() {
        return false;
    }

    while let Some(pg) = pgl.pop_head() {
        let n = upci.npages();
        upci.upci_pages[n].set(Some(pg));
        upci.upci_npages.store(n + 1, Ordering::Relaxed);
    }
    UVMEXP
        .percpucaches
        .fetch_add(npages as i32, Ordering::Relaxed);

    true
}

/// `uvm_pmr_cache_get`: a page from this CPU's cache, refilled from the allocator when both
/// magazines are empty; straight from the allocator when it cannot be refilled.
#[cfg(feature = "multiprocessor")]
pub fn uvm_pmr_cache_get(flags: i32) -> Option<&'static VmPage> {
    let upc = uvm_pmr_curcache();

    // XXX The buffer flipper (incorrectly?) allocates & frees pages (from
    // uvm_pagerealloc_multi()) from interrupt context!
    let s = hal::intr::splraise(IPL_BIO);
    let actv = upc.upc_actv.load(Ordering::Relaxed);
    let mut upci = &upc.upc_magz[actv];
    if upci.npages() == 0 {
        let prev = if actv == 0 { 1 } else { 0 };
        upci = &upc.upc_magz[prev];
        if upci.npages() == 0 {
            UVMEXP.pcpmiss.fetch_add(1, Ordering::Relaxed);
            if !uvm_pmr_cache_alloc(upci) {
                drop(s);
                return uvm_pmr_getone(flags);
            }
        }
        // Swap magazines
        upc.upc_actv.store(prev, Ordering::Relaxed);
    } else {
        UVMEXP.pcphit.fetch_add(1, Ordering::Relaxed);
    }

    UVMEXP.percpucaches.fetch_sub(1, Ordering::Relaxed);
    let n = upci.npages() - 1;
    upci.upci_npages.store(n, Ordering::Relaxed);
    let pg = upci.upci_pages[n].get();
    upci.upci_pages[n].set(None);
    drop(s);

    let Some(pg) = pg else {
        kpanic(format_args!("uvm_pmr_cache_get: an empty magazine slot"));
    };
    if flags & UVM_PLA_ZERO != 0 {
        uvm_pagezero(pg);
    }

    Some(pg)
}

/// `uvm_pmr_cache_free`: gives every page of the magazine back to the allocator; the number
/// of pages.
#[cfg(feature = "multiprocessor")]
fn uvm_pmr_cache_free(upci: &UvmPmrCacheItem) -> u32 {
    hal::intr::splassert(IPL_BIO, "uvm_pmr_cache_free");

    let pgl = Pglist::new();
    let n = upci.npages();
    for slot in &upci.upci_pages[..n] {
        if let Some(pg) = slot.get() {
            pgl.insert_tail(pg);
        }
    }

    uvm_pmr_freepageq(&pgl);

    UVMEXP.percpucaches.fetch_sub(n as i32, Ordering::Relaxed);
    upci.upci_npages.store(0, Ordering::Relaxed);
    for slot in &upci.upci_pages {
        slot.set(None);
    }

    n as u32
}

/// `uvm_pmr_cache_put`: returns a page to this CPU's cache; low pages always go back to the
/// allocator, so as not to accelerate their exhaustion.
#[cfg(feature = "multiprocessor")]
pub fn uvm_pmr_cache_put(pg: &'static VmPage) {
    let upc = uvm_pmr_curcache();

    // Always give back low pages to the allocator to not accelerate their exhaustion.
    let low = {
        let fs = FREE.lock();
        uvm_pmemrange_find(&fs, atop(vm_page_to_phys(pg).as_usize()))
            .is_none_or(|pmr| pmr.use_count() > 0)
    };
    if low {
        uvm_pmr_freepages(pg, 1);
        return;
    }

    kassert!(pg.wire_count() == 0);
    kassert!(!pg.has_owner());

    // XXX The buffer flipper (incorrectly?) allocates & frees pages (from
    // uvm_pagerealloc_multi()) from interrupt context!
    let s = hal::intr::splraise(IPL_BIO);
    let actv = upc.upc_actv.load(Ordering::Relaxed);
    let mut upci = &upc.upc_magz[actv];
    if upci.npages() >= UVM_PMR_CACHEMAGSZ {
        let prev = if actv == 0 { 1 } else { 0 };
        upci = &upc.upc_magz[prev];
        if upci.npages() > 0 {
            let _ = uvm_pmr_cache_free(upci);
        }

        // Swap magazines
        upc.upc_actv.store(prev, Ordering::Relaxed);
        kassert!(upci.npages() == 0);
    }

    let n = upci.npages();
    upci.upci_pages[n].set(Some(pg));
    upci.upci_npages.store(n + 1, Ordering::Relaxed);
    UVMEXP.percpucaches.fetch_add(1, Ordering::Relaxed);
    drop(s);
}

/// `uvm_pmr_cache_drain`: gives this CPU's cached pages back to the allocator; how many.
#[cfg(feature = "multiprocessor")]
pub fn uvm_pmr_cache_drain() -> u32 {
    let upc = uvm_pmr_curcache();
    let mut freed = 0;

    // XXX The buffer flipper (incorrectly?) allocates & frees pages (from
    // uvm_pagerealloc_multi()) from interrupt context!
    let s = hal::intr::splraise(IPL_BIO);
    freed += uvm_pmr_cache_free(&upc.upc_magz[0]);
    freed += uvm_pmr_cache_free(&upc.upc_magz[1]);
    drop(s);

    freed
}

// !(MULTIPROCESSOR && __HAVE_UVM_PERCPU)

/// `uvm_pmr_cache_get`: without a per-CPU cache, straight from the allocator.
#[cfg(not(feature = "multiprocessor"))]
pub fn uvm_pmr_cache_get(flags: i32) -> Option<&'static VmPage> {
    uvm_pmr_getone(flags)
}

/// `uvm_pmr_cache_put`: without a per-CPU cache, straight to the allocator.
#[cfg(not(feature = "multiprocessor"))]
pub fn uvm_pmr_cache_put(pg: &'static VmPage) {
    uvm_pmr_freepages(pg, 1);
}

/// `uvm_pmr_cache_drain`: nothing to drain without a per-CPU cache.
#[cfg(not(feature = "multiprocessor"))]
pub fn uvm_pmr_cache_drain() -> u32 {
    0
}

/// Frees the pages behind the physical addresses `frames` as one list (`uvm_pglistfree` of the
/// frames' pages: the free lists, `uvmexp.free`, the wakeups; never a per-CPU cache), for the
/// kernel's allocator, whose boundary is the frame (`kernel::vm::km`). When no page describes
/// a frame, nothing is freed and that frame is the error.
pub fn uvm_pmr_free_frames(frames: &[Paddr]) -> Result<(), Paddr> {
    for &pa in frames {
        if page::PHYS_TO_VM_PAGE(pa).is_none() {
            return Err(pa);
        }
    }
    let pgl = Pglist::new();
    for &pa in frames {
        if let Some(pg) = page::PHYS_TO_VM_PAGE(pa) {
            pgl.insert_tail(pg);
        }
    }
    if !pgl.is_empty() {
        uvm_pmr_freepageq(&pgl);
    }
    Ok(())
}

/// Forgets every range and every CPU's cached pages without freeing them, so a host test that
/// sets up its own memory (`uvm_page_test_reset`) is never handed a page of an earlier
/// test's. Tests only.
#[cfg(any(test, feature = "doubles"))]
pub fn uvm_pmr_cache_test_reset() {
    *FREE.lock() = FreeState::new();
    #[cfg(feature = "multiprocessor")]
    for upc in (0..hal::cpu::MAXCPUS).filter_map(|i| UVM_PMR_CACHES.cpu(i)) {
        for upci in &upc.upc_magz {
            upci.upci_npages.store(0, Ordering::Relaxed);
            for slot in &upci.upci_pages {
                slot.set(None);
            }
        }
        upc.upc_actv.store(0, Ordering::Relaxed);
    }
    UVMEXP.percpucaches.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the physical page allocator over synthetic physical segments (moved from
    // `bsd`'s `uvm_pmemrange.rs`, N3.3).
    //
    // The page system is global, so every test holds `testutil::MEMORY` (through
    // `testutil::pages_setup`), forgets the previous memory and loads two segments: one inside
    // the host's "ISA" range (use count 2 after `uvm_pmr_init`) and one above 4 GiB (use count
    // 0), so allocations prefer the second. The page array and the ranges are stolen from the
    // segments, so the free total is a little under the loaded size; [`total`] reads it from
    // the segments.

    use std::sync::MutexGuard;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use super::*;
    use crate::testutil;
    use crate::vm::page::{
        Owner, PG_BUSY, PHYS_TO_VM_PAGE, uvm_pagealloc, uvm_pagefree, uvm_pglistalloc,
        uvm_pglistfree, vm_physmem,
    };
    use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};

    /// A segment in the ISA range: pages 0x100..0x200 (1 MiB..2 MiB).
    const SEG_ISA: (usize, usize) = (0x100, 0x200);
    /// A segment above 4 GiB: pages 0x10_0000..0x10_1000 (16 MiB).
    const SEG_HIGH: (usize, usize) = (0x10_0000, 0x10_1000);

    fn pages(seg: (usize, usize)) -> usize {
        seg.1 - seg.0
    }

    /// The pages the segments hold after the boot-time stealing.
    fn total() -> usize {
        vm_physmem()
            .iter()
            .map(|seg| seg.avail_end - seg.avail_start)
            .sum()
    }

    fn free() -> usize {
        UVMEXP.free.load(Ordering::Relaxed) as usize
    }

    fn in_seg(pg: &VmPage, seg: (usize, usize)) -> bool {
        (seg.0..seg.1).contains(&pgno(pg))
    }

    fn assert_all_valid() {
        let fs = FREE.lock();
        for pmr in fs.control.addr.iter() {
            uvm_pmr_assertvalid(&fs, pmr);
        }
    }

    fn find(pageno: usize) -> Option<&'static UvmPmemrange> {
        uvm_pmemrange_find(&FREE.lock(), pageno)
    }

    fn isfree(pg: &VmPage) -> bool {
        uvm_pmr_isfree(&FREE.lock(), pg)
    }

    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_setup(&[SEG_HIGH, SEG_ISA], &testutil::HOST_CONSTRAINTS);
        assert!(
            total() > pages(SEG_ISA) + pages(SEG_HIGH) - 256,
            "little was stolen"
        );
        guard
    }

    #[test]
    fn init_frees_every_page_and_splits_the_ranges() {
        let _g = setup();
        assert_eq!(free(), total());
        assert_all_valid();

        // The host's constraints (ISA at 16 MiB, DMA at 4 GiB) split the single range in
        // three, sorted by use on the use queue: the unconstrained range first.
        let fs = FREE.lock();
        let uses: Vec<i32> = fs.control.by_use().map(UvmPmemrange::use_count).collect();
        assert_eq!(uses, vec![0, 1, 2]);
        assert_eq!(fs.control.ranges().count(), 3);
        drop(fs);

        let isa = find(SEG_ISA.0).expect("the ISA segment has a range");
        assert_eq!((isa.low(), isa.high()), (0, 0x1000));
        assert_eq!(isa.nsegs(), 1);
        let high = find(SEG_HIGH.0).expect("the high segment has a range");
        assert_eq!(high.low(), 0x10_0000);
        assert_eq!(high.nsegs(), 1);

        // Boot memory was stolen from a boundary of one segment (`uvm_page_physsteal`): what
        // is left of each segment is one free range.
        for seg in vm_physmem() {
            let pg = PHYS_TO_VM_PAGE(Paddr::new(seg.avail_start << PAGE_SHIFT))
                .expect("the first available page of a segment has a structure");
            assert!(isfree(pg));
            assert_eq!(fpgsz(pg), seg.avail_end - seg.avail_start);
        }
        assert!(
            total() < pages(SEG_ISA) + pages(SEG_HIGH),
            "the page array came out of a segment"
        );
    }

    #[test]
    fn getone_prefers_the_least_used_range() {
        let _g = setup();
        let pg = uvm_pmr_getone(UVM_PLA_NOWAIT).expect("a page");
        assert!(
            in_seg(pg, SEG_HIGH),
            "page {:#x} is not in the high segment",
            pgno(pg)
        );
        assert_eq!(pg.flags() & PQ_FREE, 0);
        assert_eq!(free(), total() - 1);
        assert!(!isfree(pg));
        assert_all_valid();

        uvm_pmr_freepages(pg, 1);
        assert_eq!(free(), total());
        assert!(isfree(pg));
        assert_all_valid();
        // The page joined its range again: one segment per range.
        let high = find(SEG_HIGH.0).expect("range");
        assert_eq!(high.nsegs(), 1);
    }

    #[test]
    fn contiguous_aligned_and_bounded() {
        let _g = setup();
        let list = Pglist::new();
        let npages = 16;
        let r = uvm_pglistalloc(
            npages * PAGE_SIZE,
            Paddr::new(SEG_HIGH.0 << PAGE_SHIFT),
            Paddr::new((SEG_HIGH.1 << PAGE_SHIFT) - 1),
            Paddr::new(16 * PAGE_SIZE),
            Paddr::new(64 * PAGE_SIZE),
            &list,
            1,
            UVM_PLA_NOWAIT,
        );
        assert_eq!(r, Ok(()));
        let got: Vec<usize> = list.iter().map(pgno).collect();
        assert_eq!(got.len(), npages);
        assert_eq!(got[0] % 16, 0, "aligned");
        assert_eq!(got[0] / 64, got[npages - 1] / 64, "no boundary crossing");
        for w in got.windows(2) {
            assert_eq!(w[1], w[0] + 1, "contiguous");
        }
        assert!(got.iter().all(|&p| (SEG_HIGH.0..SEG_HIGH.1).contains(&p)));
        assert_eq!(free(), total() - npages);
        assert_all_valid();

        uvm_pglistfree(&list);
        assert!(list.is_empty());
        assert_eq!(free(), total());
        assert_all_valid();
    }

    #[test]
    fn constrained_to_the_isa_range() {
        let _g = setup();
        let list = Pglist::new();
        let r = uvm_pglistalloc(
            4 * PAGE_SIZE,
            Paddr::new(0),
            Paddr::new(0x00ff_ffff),
            Paddr::new(0),
            Paddr::new(0),
            &list,
            4,
            UVM_PLA_NOWAIT,
        );
        assert_eq!(r, Ok(()));
        assert_eq!(list.iter().count(), 4);
        assert!(list.iter().all(|pg| in_seg(pg, SEG_ISA)));
        uvm_pglistfree(&list);
        assert_eq!(free(), total());
        assert_all_valid();
    }

    #[test]
    fn exhaustion_fails_cleanly() {
        let _g = setup();
        let list = Pglist::new();

        // More than exists.
        let r = uvm_pmr_getpages(total() + 1, 0, 0, 1, 0, 1, UVM_PLA_NOWAIT, &list);
        assert_eq!(r, Err(Errno::ENOMEM));
        assert!(list.is_empty());
        assert_eq!(free(), total());

        // Everything but the reserves, in as many segments as it takes.
        let reserve = UVMEXP.reserve_kernel.load(Ordering::Relaxed) as usize;
        let count = total() - reserve - 1;
        let r = uvm_pmr_getpages(count, 0, 0, 1, 0, count as i32, UVM_PLA_NOWAIT, &list);
        assert_eq!(r, Ok(()));
        assert_eq!(list.iter().count(), count);
        assert_eq!(free(), total() - count);
        assert!(list.iter().all(|pg| pg.flags() & PQ_FREE == 0));
        assert_all_valid();

        // Into the reserve: refused without UVM_PLA_USERESERVE.
        let more = Pglist::new();
        let r = uvm_pmr_getpages(2, 0, 0, 1, 0, 2, UVM_PLA_NOWAIT, &more);
        assert_eq!(r, Err(Errno::ENOMEM));
        assert!(more.is_empty());

        uvm_pmr_freepageq(&list);
        assert!(list.is_empty());
        assert_eq!(free(), total());
        assert_all_valid();
        for pmr in FREE.lock().control.ranges() {
            assert!(pmr.nsegs() <= 1, "every range is one free chunk again");
        }
    }

    #[test]
    fn freeing_a_descending_list_joins_the_range() {
        let _g = setup();
        let list = Pglist::new();
        let r = uvm_pmr_getpages(32, 0, 0, 1, 0, 1, UVM_PLA_NOWAIT, &list);
        assert_eq!(r, Ok(()));

        // Rebuild the list with the pages in descending order of physical address.
        let reversed = Pglist::new();
        while let Some(pg) = list.pop_head() {
            reversed.insert_head(pg);
        }
        let order: Vec<usize> = reversed.iter().map(pgno).collect();
        assert!(order.windows(2).all(|w| w[0] == w[1] + 1));

        uvm_pmr_freepageq(&reversed);
        assert!(reversed.is_empty());
        assert_eq!(free(), total());
        assert_all_valid();
        let high = find(SEG_HIGH.0).expect("range");
        assert_eq!(high.nsegs(), 1);
    }

    #[test]
    fn zeroed_pages_are_counted() {
        let _g = setup();
        let before = UVMEXP.pga_zeromiss.load(Ordering::Relaxed);
        let pg = uvm_pmr_getone(UVM_PLA_NOWAIT | UVM_PLA_ZERO).expect("a page");
        assert_eq!(UVMEXP.pga_zeromiss.load(Ordering::Relaxed), before + 1);
        assert_eq!(pg.flags() & (PG_ZERO | PQ_FREE), 0);
        uvm_pmr_freepages(pg, 1);
        assert_eq!(free(), total());
    }

    #[test]
    fn pagealloc_and_pagefree() {
        let _g = setup();
        // The free pages, those in this CPU's cache (MULTIPROCESSOR) included.
        let avail = || free() + UVMEXP.percpucaches.load(Ordering::Relaxed) as usize;
        let pg = uvm_pagealloc(Owner::None, 0, 0).expect("a page");
        assert_ne!(pg.flags() & PG_BUSY, 0);
        assert_eq!(pg.flags() & PQ_FREE, 0);
        assert!(!pg.has_owner());
        assert_eq!(avail(), total() - 1);

        uvm_pagefree(pg, Owner::None);
        assert_eq!(avail(), total());
        // On the free lists, or (MULTIPROCESSOR) in this CPU's page cache.
        assert!(isfree(pg) || cfg!(feature = "multiprocessor"));
        assert_all_valid();
    }

    #[test]
    fn frames_go_back_as_one_list() {
        let _g = setup();
        let list = Pglist::new();
        assert_eq!(
            uvm_pmr_getpages(3, 0, 0, 1, 0, 3, UVM_PLA_NOWAIT, &list),
            Ok(())
        );
        let frames: Vec<Paddr> = list.iter().map(vm_page_to_phys).collect();
        while list.pop_head().is_some() {}
        assert_eq!(free(), total() - 3);
        // A frame no page describes: nothing is freed.
        let mut bad = frames.clone();
        bad.push(Paddr::new(0x50 << PAGE_SHIFT));
        assert_eq!(
            uvm_pmr_free_frames(&bad),
            Err(Paddr::new(0x50 << PAGE_SHIFT))
        );
        assert_eq!(free(), total() - 3);
        assert_eq!(uvm_pmr_free_frames(&frames), Ok(()));
        assert_eq!(free(), total());
        assert_all_valid();
    }

    #[test]
    fn pow2divide_rounds_up_to_a_power_of_two() {
        assert_eq!(pow2divide(1, 8), 1);
        assert_eq!(pow2divide(8, 8), 1);
        assert_eq!(pow2divide(9, 8), 2);
        assert_eq!(pow2divide(100, 7), 16);
        assert_eq!(pow2divide(4096, 1), 4096);
    }

    #[test]
    fn range_predicates() {
        assert!(pmr_is_subrange_of(10, 20, 0, 0));
        assert!(pmr_is_subrange_of(10, 20, 10, 20));
        assert!(!pmr_is_subrange_of(10, 21, 10, 20));
        assert!(pmr_intersects_with(10, 20, 19, 0));
        assert!(!pmr_intersects_with(10, 20, 20, 0));
        assert!(pmr_intersects_with(10, 20, 0, 11));
        assert!(!pmr_intersects_with(10, 20, 0, 10));
        assert_eq!(pmr_align(17, 16), 32);
        assert_eq!(pmr_align_down(17, 16), 16);
    }

    #[test]
    fn the_ranges_print() {
        let _g = setup();
        let mut out = std::string::String::new();
        uvm_pmr_print(&mut out);
        assert!(out.starts_with("Ranges, use queue:\n"));
        assert!(out.contains("use=0") && out.contains("use=2"));
        assert!(out.ends_with("#ranges = 3\n"));
    }

    /// A test's memory is gone at the next setup: the per-CPU cache it filled must not hand
    /// its pages to the next test (hal's managed-page check refuses to zero them).
    #[cfg(feature = "multiprocessor")]
    #[test]
    fn a_new_setup_forgets_the_per_cpu_cache() {
        {
            let _g = setup();
            let pg = uvm_pmr_cache_get(UVM_PLA_NOWAIT).expect("a page");
            assert!(
                UVMEXP.percpucaches.load(Ordering::Relaxed) > 0,
                "the magazine filled"
            );
            assert!(in_seg(pg, SEG_HIGH));
        }
        let _g = setup();
        assert_eq!(UVMEXP.percpucaches.load(Ordering::Relaxed), 0);
        assert!(
            uvm_pmr_curcache()
                .magazines()
                .iter()
                .all(|m| m.npages() == 0)
        );
        assert_eq!(free(), total(), "every page of the new memory is free");
    }
}
/* </TESTS> */
