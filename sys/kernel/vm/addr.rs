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
 * Copyright (c) 2011 Ariane van der Steldt <ariane@stack.nl>
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
//! `uvm_addr.h` / `uvm_addr.c`: address selection logic.
//!
//! Upstream: sys/uvm/uvm_addr.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_addr.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_addr.rs@3c62ede68802
//!
//! Address selection is just that: selection. These functions may make no changes to the
//! map, except for their own state (a [`Selector`] the map's state owns).
//!
//! A map has up to six selectors ([`super::map::MapState`]: `uaddr_exe`, `uaddr_any[4]`,
//! `uaddr_brk_stack`); each is one of the C's algorithms: the randomised one
//! ([`Selector::Rnd`], `uaddr_rnd`), the kernel's bootstrap first fit
//! ([`Selector::KBootstrap`]), best fit over a free-space tree ([`Selector::Bestfit`]),
//! pivots over the same kind of tree ([`Selector::Pivot`]) and the brk/stack areas
//! ([`Selector::StackBrk`]). The selectors with free operations (rnd, bestfit, pivot) own
//! the free space of the entries in their range ([`super::map::Owners::uaddr`]); bestfit and
//! pivot index those entries in a free-space tree.
//!
//! ## Deviations
//! - The C's output parameters become return values: a selector yields the entry and the
//!   address, [`uvm_addr_fitspace`] the lowest and highest fitting address,
//!   [`uvm_addr_invoke`] the first and last entry (by their start addresses) and the address.
//! - `uaddr_lin_*` and `uaddr_rnd_print` are `#if 0` in the C and not here;
//!   `uvm_addr_print` and `uaddr_pivot_print` (`DEBUG`/`DDB`) wait for the ddb printers.
//! - `SMALL_KERNEL` is not configured: the bestfit, pivot and stack/brk selectors exist.
//! - `MACHINE_STACK_GROWS_UP` is not defined on amd64 or arm64.
//!
//! ## Redesign
//! - LZ's states are pool items written as `#[repr(C)]` structs with the base
//!   `UvmAddrState` first, handed out as `&'static UvmAddrState` and cast back to the
//!   bestfit or pivot state after a check of the function table (12 `unsafe`: the pool
//!   writes, the casts, `Sync`, the free trees' raw links, a pivot's raw entry pointer). Here
//!   a selector is the enum [`Selector`], one variant per algorithm, each owning its state as
//!   a [`PoolBox`] of the C's pool (`"uaddr"`, `"uaddrrnd"`, `"uaddrbest"`, `"uaddrpivot"`),
//!   and the function table (`struct uvm_addr_functions`) is a `match` over the variant:
//!   `uaddr_select` is [`Selector::select`], `uaddr_free_insert`/`uaddr_free_remove`
//!   [`Selector::free_insert`]/[`Selector::free_remove`] (whether the C's pointer is set:
//!   [`Selector::has_free_ops`]), `uaddr_destroy` the drop of the selector, `uaddr_name`
//!   [`Selector::name`]. The kernel bootstrap selector is one `static` (its range set by
//!   [`uvm_addr_init`]) that every kernel map shares, as in the C.
//! - The free-space trees (`uaddr_free_rbtree` on `dfree.rbtree`) are
//!   `hal::owned::SecondaryTree`s over the map's entries, which the map's address tree owns,
//!   and a pivot's `entry` is a `hal::owned::SecondaryRef`: both are counted in the entry, so
//!   the address tree refuses to give an entry back while a free tree or a pivot still names
//!   it (the C's rule that `uvm_mapent_free_remove` comes before `uvm_mapent_addr_remove`,
//!   and that `uaddr_pivot_remove` clears the pivots, as checked preconditions).
//! - A selector selects with `&mut self` over a [`SelectCx`]: the map's address tree read
//!   shared, the owners of free space (`uvm_map_uaddr_e`'s answer for the rnd selector), and
//!   the brk/stack areas. An entry found in a free tree is returned as the address tree's
//!   element (found again by its start), so the result borrows the tree only and the selector
//!   stays free to change its pivots.
//! - The pivot fields are plain values under the map's lock (the state is the map's).

use core::cmp::Ordering;
use core::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use hal::arch::current::param::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use hal::intr::IPL_VM;
use hal::owned::{OwnedTree, SecondaryRef, SecondaryTree};
use hal::pool::{BoxPool, PR_WAITOK, PoolBox};
use hal::random::{arc4random, arc4random_uniform};
use hal::sys::errno::Errno;

use super::map::{
    ByAddr, Owners, Slot, UVM_MAXKADDR, VM_MAP_GUARDPAGES, VM_MAP_ISVMSPACE, VmMap, VmMapEntry,
    uvm_map_entrybyaddr, uvm_map_hint, vmmap_free_end, vmmap_free_start,
};
use super::types::VmProt;
use crate::assert::{kassert, kpanic};

/// `NUM_PIVOTS`: number of pivots in pivot allocator.
pub const NUM_PIVOTS: usize = 16;
/// `PIVOT_RND`: max number (inclusive) of pages the pivot allocator will place between
/// allocations.
///
/// The `uaddr_pivot_random()` function attempts to bias towards small space between
/// allocations, so putting a large number here is fine.
pub const PIVOT_RND: u32 = 8;
/// `PIVOT_EXPIRE`: number of allocations that a pivot can supply before expiring. When a
/// pivot expires, a new pivot has to be found.
///
/// Must be at least 1.
pub const PIVOT_EXPIRE: i32 = 1024;

/// `struct uvm_addr_state`: UVM address selection base state: the lower bound address (page
/// aligned) and the upper bound address (page aligned) of what the selector manages. The map
/// that owns the selector guards it with its lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UvmAddrState {
    /// `uaddr_minaddr`.
    pub uaddr_minaddr: usize,
    /// `uaddr_maxaddr`.
    pub uaddr_maxaddr: usize,
}

/// `struct uaddr_rnd_state`: uvm_addr state for rnd selector (`ur_free` is `#if 0` in the C).
pub struct RndState {
    /// `ur_uaddr`.
    pub ur_uaddr: UvmAddrState,
}

/// `struct uaddr_bestfit_state`: uvm_addr state for bestfit selector.
pub struct BestfitState {
    /// `ubf_uaddr`.
    pub ubf_uaddr: UvmAddrState,
    /// `ubf_free`: the entries whose free space this selector owns, by size.
    ubf_free: SecondaryTree<ByFspace>,
}

/// `struct uaddr_pivot`: definition of a pivot in pivot selector.
pub struct Pivot {
    /// `addr`: end of prev. allocation.
    pub addr: usize,
    /// `expire`: best before date.
    pub expire: i32,
    /// `dir`: direction.
    pub dir: i32,
    /// `entry`: will contain next alloc (counted in the entry: see the module's notes).
    entry: SecondaryRef<ByAddr>,
}

impl Pivot {
    /// A zeroed pivot: expired.
    pub const fn new() -> Self {
        Self {
            addr: 0,
            expire: 0,
            dir: 0,
            entry: SecondaryRef::new(),
        }
    }
}

impl Default for Pivot {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct uaddr_pivot_state`: uvm_addr state for pivot selector.
pub struct PivotState {
    /// `up_uaddr`.
    pub up_uaddr: UvmAddrState,
    /// `up_free`: free space tree, for fast pivot selection.
    up_free: SecondaryTree<ByFspace>,
    /// `up_pivots`: list of pivots. The pointers point to after the last allocation.
    pub up_pivots: [Pivot; NUM_PIVOTS],
}

/// `uaddr_kbootstrap`: the kernel bootstrap allocator's state, which every kernel map
/// shares ([`uvm_addr_init`] sets its range).
pub struct KBootstrap {
    /// `uaddr_minaddr`.
    minaddr: AtomicUsize,
    /// `uaddr_maxaddr`.
    maxaddr: AtomicUsize,
}

impl KBootstrap {
    /// The range, before `uvm_addr_init` empty.
    const fn new() -> Self {
        Self {
            minaddr: AtomicUsize::new(0),
            maxaddr: AtomicUsize::new(0),
        }
    }

    /// The base state.
    fn state(&self) -> UvmAddrState {
        UvmAddrState {
            uaddr_minaddr: self.minaddr.load(AtomicOrdering::Relaxed),
            uaddr_maxaddr: self.maxaddr.load(AtomicOrdering::Relaxed),
        }
    }
}

/// One address selector of a map (`struct uvm_addr_state *` with its
/// `struct uvm_addr_functions`): the state of one algorithm.
pub enum Selector {
    /// `uaddr_rnd_functions`.
    Rnd(PoolBox<RndState>),
    /// `uaddr_kernel_functions` over the shared `uaddr_kbootstrap`.
    KBootstrap(&'static KBootstrap),
    /// `uaddr_bestfit_functions`.
    Bestfit(PoolBox<BestfitState>),
    /// `uaddr_pivot_functions`.
    Pivot(PoolBox<PivotState>),
    /// `uaddr_stack_brk_functions`.
    StackBrk(PoolBox<UvmAddrState>),
}

impl Selector {
    /// The base state: the range it manages.
    pub fn state(&self) -> UvmAddrState {
        match self {
            Selector::Rnd(s) => s.ur_uaddr,
            Selector::KBootstrap(k) => k.state(),
            Selector::Bestfit(s) => s.ubf_uaddr,
            Selector::Pivot(s) => s.up_uaddr,
            Selector::StackBrk(s) => **s,
        }
    }

    /// `[uaddr_minaddr, uaddr_maxaddr)`.
    pub fn range(&self) -> (usize, usize) {
        let s = self.state();
        (s.uaddr_minaddr, s.uaddr_maxaddr)
    }

    /// `uaddr_name`: name of the allocator.
    pub fn name(&self) -> &'static str {
        match self {
            Selector::Rnd(_) => "uaddr_rnd",
            Selector::KBootstrap(_) => "uaddr_kbootstrap",
            Selector::Bestfit(_) => "uaddr_bestfit",
            Selector::Pivot(_) => "uaddr_pivot",
            Selector::StackBrk(_) => "uaddr_stckbrk",
        }
    }

    /// `uaddr_free_insert != NULL` (and so `uaddr_free_remove`): whether the selector owns the
    /// free space in its range (`uvm_map_uaddr` names only those).
    pub fn has_free_ops(&self) -> bool {
        matches!(
            self,
            Selector::Rnd(_) | Selector::Bestfit(_) | Selector::Pivot(_)
        )
    }

    /// The selector's identity, as the C's `%p` of the state prints it.
    fn addr(&self) -> usize {
        match self {
            Selector::Rnd(s) => PoolBox::as_ptr(s).addr(),
            Selector::KBootstrap(k) => core::ptr::from_ref(*k).addr(),
            Selector::Bestfit(s) => PoolBox::as_ptr(s).addr(),
            Selector::Pivot(s) => PoolBox::as_ptr(s).addr(),
            Selector::StackBrk(s) => PoolBox::as_ptr(s).addr(),
        }
    }

    /// `uaddr_free_insert`: `entry`'s free space is now this selector's (the rnd selector's
    /// tailq is `#if 0`: nothing to do).
    pub fn free_insert(&mut self, entry: &VmMapEntry) {
        match self {
            Selector::Bestfit(s) => uaddr_bestfit_insert(s, entry),
            Selector::Pivot(s) => uaddr_pivot_insert(s, entry),
            Selector::Rnd(_) | Selector::KBootstrap(_) | Selector::StackBrk(_) => {}
        }
    }

    /// `uaddr_free_remove`: `entry`'s free space leaves this selector.
    pub fn free_remove(&mut self, entry: &VmMapEntry) {
        match self {
            Selector::Bestfit(s) => uaddr_bestfit_remove(s, entry),
            Selector::Pivot(s) => uaddr_pivot_remove(s, entry),
            Selector::Rnd(_) | Selector::KBootstrap(_) | Selector::StackBrk(_) => {}
        }
    }

    /// `uaddr_select`: an address for `sz` bytes (aligned to `align`, `offset` past it, for a
    /// mapping of `prot`, near `hint` when not 0), and the entry whose free space holds it.
    #[allow(clippy::too_many_arguments)] // the C's signature
    pub fn select<'t>(
        &mut self,
        cx: &SelectCx<'t>,
        sz: usize,
        align: usize,
        offset: usize,
        prot: VmProt,
        hint: usize,
    ) -> Result<(&'t VmMapEntry, usize), Errno> {
        match self {
            Selector::Rnd(s) => uaddr_rnd_select(cx, &s.ur_uaddr, sz, align, offset, prot, hint),
            Selector::KBootstrap(_) => uaddr_kbootstrap_select(cx, sz, align, offset),
            Selector::Bestfit(s) => uaddr_bestfit_select(cx, s, sz, align, offset),
            Selector::Pivot(s) => uaddr_pivot_select(cx, s, sz, align, offset, prot, hint),
            Selector::StackBrk(s) => uaddr_stack_brk_select(cx, s, sz, align, offset),
        }
    }
}

/// What a selector selects in: the map (its flags, its vmspace's data address), the address
/// tree, the owners of each address's free space and the map's brk/stack areas, all under
/// the map's lock.
pub struct SelectCx<'t> {
    /// The map.
    pub map: &'t VmMap,
    /// `map->addr`.
    pub tree: &'t OwnedTree<ByAddr>,
    /// Which selector owns which free space (`uvm_map_uaddr`).
    pub owners: Owners,
    /// The slot of the selector selecting (the rnd selector keeps to its own free space).
    pub me: Slot,
    /// `map->b_start`, `map->b_end`.
    pub brk: (usize, usize),
    /// `map->s_start`, `map->s_end`.
    pub stack: (usize, usize),
}

/// `uaddr_pool`: the stack/brk selectors.
static UADDR_POOL: BoxPool<UvmAddrState> = BoxPool::new();
/// `uaddr_bestfit_pool`.
static UADDR_BESTFIT_POOL: BoxPool<BestfitState> = BoxPool::new();
/// `uaddr_pivot_pool`.
static UADDR_PIVOT_POOL: BoxPool<PivotState> = BoxPool::new();
/// `uaddr_rnd_pool`.
static UADDR_RND_POOL: BoxPool<RndState> = BoxPool::new();
/// `uaddr_kbootstrap`.
static UADDR_KBOOTSTRAP: KBootstrap = KBootstrap::new();

/*
 * Support functions.
 */

/// `uvm_addr_entrybyspace`: find smallest entry in tree that will fit `sz` bytes.
fn uvm_addr_entrybyspace(free: &SecondaryTree<ByFspace>, sz: usize) -> Option<&VmMapEntry> {
    let mut tmp = free.root();
    let mut res = None;
    while let Some(e) = tmp {
        if e.fspace() >= sz {
            res = Some(e);
            tmp = free.left(e);
        } else {
            tmp = free.right(e);
        }
    }
    res
}

/// `uvm_addr_align_forward`: the first address at or after `addr` that is `offset` past a
/// multiple of `align` (wrapping, which the callers check for).
#[inline]
fn uvm_addr_align_forward(addr: usize, align: usize, offset: usize) -> usize {
    kassert!(offset < align || (align == 0 && offset == 0));
    kassert!(align & align.wrapping_sub(1) == 0);
    kassert!(offset & PAGE_MASK == 0);

    let align = align.max(PAGE_SIZE);
    let adjusted = (addr & !(align - 1)).wrapping_add(offset);
    if adjusted < addr {
        adjusted.wrapping_add(align)
    } else {
        adjusted
    }
}

/// `uvm_addr_align_backward`: the last address at or before `addr` that is `offset` past a
/// multiple of `align` (wrapping, which the callers check for).
#[inline]
fn uvm_addr_align_backward(addr: usize, align: usize, offset: usize) -> usize {
    kassert!(offset < align || (align == 0 && offset == 0));
    kassert!(align & align.wrapping_sub(1) == 0);
    kassert!(offset & PAGE_MASK == 0);

    let align = align.max(PAGE_SIZE);
    let adjusted = (addr & !(align - 1)).wrapping_add(offset);
    if adjusted > addr {
        adjusted.wrapping_sub(align)
    } else {
        adjusted
    }
}

/// `uvm_addr_fitspace`: try to fit the requested space into the entry. Yields the lowest
/// and the highest address at which `sz` bytes fit in `[low_addr, high_addr)` with the
/// alignment and the gaps.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn uvm_addr_fitspace(
    low_addr: usize,
    high_addr: usize,
    sz: usize,
    align: usize,
    offset: usize,
    before_gap: usize,
    after_gap: usize,
) -> Result<(usize, usize), Errno> {
    if low_addr > high_addr {
        return Err(Errno::ENOMEM);
    }
    let fspace = high_addr - low_addr;
    if fspace < before_gap + after_gap {
        return Err(Errno::ENOMEM);
    }
    if fspace - before_gap - after_gap < sz {
        return Err(Errno::ENOMEM);
    }

    // Calculate lowest address.
    let tmp = low_addr + before_gap;
    let low_addr = uvm_addr_align_forward(tmp, align, offset);
    if low_addr < tmp {
        // Overflow during alignment.
        return Err(Errno::ENOMEM);
    }
    if high_addr - after_gap - sz < low_addr {
        return Err(Errno::ENOMEM);
    }

    // Calculate highest address.
    let tmp = high_addr - (after_gap + sz);
    let high_addr = uvm_addr_align_backward(tmp, align, offset);
    if high_addr > tmp {
        // Overflow during alignment.
        return Err(Errno::ENOMEM);
    }
    if low_addr > high_addr {
        return Err(Errno::ENOMEM);
    }

    Ok((low_addr, high_addr))
}

/// `uvm_addr_init`: initialize uvm_addr: the four pools and the bootstrap selector's range.
pub fn uvm_addr_init() {
    let _ = UADDR_POOL.init(IPL_VM, PR_WAITOK, "uaddr");
    let _ = UADDR_BESTFIT_POOL.init(IPL_VM, PR_WAITOK, "uaddrbest");
    let _ = UADDR_PIVOT_POOL.init(IPL_VM, PR_WAITOK, "uaddrpivot");
    let _ = UADDR_RND_POOL.init(IPL_VM, PR_WAITOK, "uaddrrnd");

    UADDR_KBOOTSTRAP
        .minaddr
        .store(PAGE_SIZE, AtomicOrdering::Relaxed);
    UADDR_KBOOTSTRAP
        .maxaddr
        .store(0usize.wrapping_sub(PAGE_SIZE), AtomicOrdering::Relaxed);
}

/// `uvm_addr_destroy`: invoke destructor function of uaddr (the selector's drop: its state
/// back to its pool, its free tree and pivots unlinked; the bootstrap selector is shared and
/// stays).
pub fn uvm_addr_destroy(uaddr: Option<Selector>) {
    drop(uaddr);
}

/// `uvm_addr_linsearch`: directional first fit.
///
/// Do a linear search for free space, starting at `hint`. `direction == 1`: search
/// forward; `direction == -1`: search backward.
///
/// Output: `low <= addr <= high` and entry will contain addr. `ENOMEM` if no space is
/// available.
///
/// gap describes the space that must appear between the preceding entry.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn uvm_addr_linsearch<'t>(
    cx: &SelectCx<'t>,
    hint: usize,
    sz: usize,
    align: usize,
    offset: usize,
    direction: i32,
    low: usize,
    high: usize,
    before_gap: usize,
    after_gap: usize,
) -> Result<(&'t VmMapEntry, usize), Errno> {
    kassert!(direction == -1 || direction == 1);
    kassert!(
        hint & PAGE_MASK == 0
            && high & PAGE_MASK == 0
            && low & PAGE_MASK == 0
            && before_gap & PAGE_MASK == 0
            && after_gap & PAGE_MASK == 0
    );
    kassert!(high.wrapping_add(sz) > high); // Check for overflow.

    // Hint magic.
    let mut hint = hint;
    if hint == 0 {
        hint = if direction == 1 { low } else { high };
    } else if hint > high {
        if direction != -1 {
            return Err(Errno::ENOMEM);
        }
        hint = high;
    } else if hint < low {
        if direction != 1 {
            return Err(Errno::ENOMEM);
        }
        hint = low;
    }

    let tree = cx.tree;
    let mut entry = uvm_map_entrybyaddr(tree, hint.wrapping_sub(usize::from(direction == -1)));
    while let Some(e) = entry {
        if (direction == 1 && vmmap_free_start(e) > high)
            || (direction == -1 && vmmap_free_end(e) < low)
        {
            break;
        }

        if let Ok((low_addr, high_addr)) = uvm_addr_fitspace(
            low.max(vmmap_free_start(e)),
            high.min(vmmap_free_end(e)),
            sz,
            align,
            offset,
            before_gap,
            after_gap,
        ) {
            let addr = if hint >= low_addr && hint <= high_addr {
                hint
            } else if direction == 1 {
                low_addr
            } else {
                high_addr
            };
            return Ok((e, addr));
        }

        entry = if direction == 1 {
            tree.next(e)
        } else {
            tree.prev(e)
        };
    }

    Err(Errno::ENOMEM)
}

/// `uvm_addr_invoke`: invoke address selector `slot` of the map (`ENOMEM` when it has none).
///
/// Will invoke `uvm_map_isavail` to fill in the last entry: yields the first entry and the
/// last entry (their start addresses, the keys of the address tree) and the address.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn uvm_addr_invoke(
    map: &VmMap,
    st: &mut super::map::MapState,
    slot: Slot,
    sz: usize,
    align: usize,
    offset: usize,
    prot: VmProt,
    hint: usize,
) -> Result<(usize, usize, usize), Errno> {
    // An empty slot answers before the owners are computed (uvm_map_findspace tries every
    // slot of uaddr_any in turn).
    if st.selector(slot).is_none() {
        return Err(Errno::ENOMEM);
    }
    let owners = st.owners(map);
    let brk = (st.b_start, st.b_end);
    let stack = (st.s_start, st.s_end);
    let (tree, sel) = st.tree_and_selector(slot);
    let Some(uaddr) = sel else {
        return Err(Errno::ENOMEM);
    };

    let hint = hint & !PAGE_MASK;
    let (minaddr, maxaddr) = uaddr.range();
    if hint != 0 && !(hint >= minaddr && hint < maxaddr) {
        return Err(Errno::ENOMEM);
    }

    let cx = SelectCx {
        map,
        tree,
        owners,
        me: slot,
        brk,
        stack,
    };
    let (entry, addr) = uaddr.select(&cx, sz, align, offset, prot, hint)?;

    let mut first = Some(entry);
    let mut last = None;
    if !super::map::uvm_map_isavail(
        map,
        tree,
        &owners,
        Some(slot),
        &mut first,
        &mut last,
        addr,
        sz,
    ) {
        kpanic(format_args!(
            "uvm_addr_invoke: address selector {:#x} ({} {:#x}-{:#x}) returned unavailable address {:#x} sz {:#x}",
            uaddr.addr(),
            uaddr.name(),
            minaddr,
            maxaddr,
            addr,
            sz
        ));
    }
    match (first, last) {
        (Some(first), Some(last)) => Ok((first.start(), last.start(), addr)),
        _ => kpanic(format_args!(
            "uvm_addr_invoke: uvm_map_isavail left the range {addr:#x}+{sz:#x} unbounded"
        )),
    }
}

/*
 * Randomized allocator.
 * This allocator use uvm_map_hint to acquire a random address and searches
 * from there.
 */

/// `uaddr_rnd_create`.
pub fn uaddr_rnd_create(minaddr: usize, maxaddr: usize) -> Selector {
    let Some(s) = UADDR_RND_POOL.get(PR_WAITOK, || RndState {
        ur_uaddr: UvmAddrState {
            uaddr_minaddr: minaddr,
            uaddr_maxaddr: maxaddr,
        },
    }) else {
        kpanic(format_args!("uaddr_rnd_create: uaddr_rnd_pool is empty"));
    };
    Selector::Rnd(s)
}

/// `uaddr_rnd_select` over the base state `uaddr` (the rnd selector's, or a pivot
/// selector's given a hint).
#[allow(clippy::too_many_arguments)] // the C's signature
fn uaddr_rnd_select<'t>(
    cx: &SelectCx<'t>,
    uaddr: &UvmAddrState,
    sz: usize,
    align: usize,
    offset: usize,
    prot: VmProt,
    hint: usize,
) -> Result<(&'t VmMapEntry, usize), Errno> {
    let map = cx.map;
    kassert!(map.flags() & VM_MAP_ISVMSPACE != 0);
    let vm = map.vmspace_vars();

    // Deal with guardpages: search for space with one extra page.
    let guard_sz = if map.flags() & VM_MAP_GUARDPAGES == 0 {
        0
    } else {
        PAGE_SIZE
    };

    if uaddr.uaddr_maxaddr.wrapping_sub(guard_sz) < sz {
        return Err(Errno::ENOMEM);
    }
    let minaddr = uvm_addr_align_forward(uaddr.uaddr_minaddr, align, offset);
    let maxaddr = uvm_addr_align_backward(uaddr.uaddr_maxaddr - sz - guard_sz, align, offset);

    // Quick fail if the allocation won't fit.
    if minaddr >= maxaddr {
        return Err(Errno::ENOMEM);
    }

    // Select a hint.
    let mut hint = hint;
    if hint == 0 {
        hint = uvm_map_hint(vm, prot, minaddr, maxaddr);
    }
    // Clamp hint to uaddr range.
    hint = hint.max(minaddr).min(maxaddr);

    // Align hint to align,offset parameters.
    let tmp = hint;
    hint = uvm_addr_align_forward(tmp, align, offset);
    // Check for overflow during alignment.
    if hint < tmp || hint > maxaddr {
        return Err(Errno::ENOMEM); // Compatibility mode: never look backwards.
    }

    let before_gap = 0;
    let after_gap = guard_sz;
    hint -= hint.min(before_gap);
    let need = before_gap + after_gap + sz;

    // Use the augmented address tree to look up the first entry at or after hint with
    // sufficient space.
    //
    // This code is the original optimized code, but will fail if the subtree it looks at
    // does have sufficient space, but fails to meet the align constraint.
    //
    // Guard: subtree is not exhausted and max(fspace) >= required.
    let tree = cx.tree;
    let mut entry = uvm_map_entrybyaddr(tree, hint);

    // Walk up the tree, until there is at least sufficient space.
    while let Some(e) = entry {
        if e.fspace_augment() >= need {
            break;
        }
        entry = tree.parent(e);
    }

    while let Some(e) = entry {
        // Test if this fits.
        if vmmap_free_end(e) > hint
            && cx.owners.uaddr(vmmap_free_start(e)) == Some(cx.me)
            && let Ok((low_addr, high_addr)) = uvm_addr_fitspace(
                uaddr.uaddr_minaddr.max(vmmap_free_start(e)),
                uaddr.uaddr_maxaddr.min(vmmap_free_end(e)),
                sz,
                align,
                offset,
                before_gap,
                after_gap,
            )
        {
            let addr = if hint >= low_addr && hint <= high_addr {
                hint
            } else {
                low_addr
            };
            return Ok((e, addr));
        }

        // RBT_NEXT, but skip subtrees that cannot possible fit.
        match tree.right(e) {
            Some(right) if right.fspace_augment() >= need => {
                let mut leftmost = right;
                while let Some(next) = tree.left(leftmost) {
                    leftmost = next;
                }
                entry = Some(leftmost);
            }
            _ => {
                // do_parent: climb while we are a right child; stop at the first ancestor
                // we are a left child of.
                let mut cur = e;
                entry = loop {
                    match tree.parent(cur) {
                        None => break None,
                        Some(parent) => {
                            if tree.left(parent).is_some_and(|l| core::ptr::eq(l, cur)) {
                                break Some(parent);
                            }
                            cur = parent;
                        }
                    }
                };
            }
        }
    }

    // Lookup failed.
    Err(Errno::ENOMEM)
}

/*
 * Kernel allocation bootstrap logic.
 */

/// `uaddr_kbootstrap_create`: the bootstrap selector every kernel map shares
/// (`&uaddr_kbootstrap`).
pub fn uaddr_kbootstrap() -> Selector {
    Selector::KBootstrap(&UADDR_KBOOTSTRAP)
}

/// `uaddr_kbootstrap_select`: select an address from the map.
///
/// This function ignores the uaddr spec and instead uses the map directly. Because of that
/// property, the uaddr algorithm can be shared across all kernel maps.
fn uaddr_kbootstrap_select<'t>(
    cx: &SelectCx<'t>,
    sz: usize,
    align: usize,
    offset: usize,
) -> Result<(&'t VmMapEntry, usize), Errno> {
    let maxkaddr = UVM_MAXKADDR.load(AtomicOrdering::Relaxed);
    for entry in cx.tree.iter() {
        if vmmap_free_end(entry) <= maxkaddr
            && let Ok((addr, _)) = uvm_addr_fitspace(
                vmmap_free_start(entry),
                vmmap_free_end(entry),
                sz,
                align,
                offset,
                0,
                0,
            )
        {
            return Ok((entry, addr));
        }
    }

    Err(Errno::ENOMEM)
}

/*
 * Best fit algorithm.
 */

/// `uaddr_bestfit_create`.
pub fn uaddr_bestfit_create(minaddr: usize, maxaddr: usize) -> Selector {
    let Some(s) = UADDR_BESTFIT_POOL.get(PR_WAITOK, || BestfitState {
        ubf_uaddr: UvmAddrState {
            uaddr_minaddr: minaddr,
            uaddr_maxaddr: maxaddr,
        },
        ubf_free: SecondaryTree::new(),
    }) else {
        kpanic(format_args!(
            "uaddr_bestfit_create: uaddr_bestfit_pool is empty"
        ));
    };
    Selector::Bestfit(s)
}

/// `uaddr_bestfit_insert`.
fn uaddr_bestfit_insert(uaddr: &mut BestfitState, entry: &VmMapEntry) {
    let me = core::ptr::from_ref(uaddr).addr();
    if let Some(rb_rv) = uaddr.ubf_free.insert(entry) {
        kpanic(format_args!(
            "uaddr_bestfit_insert: duplicate insertion: state {:#x} inserting {:#x}, colliding with {:#x}",
            me,
            entry.start(),
            rb_rv.start()
        ));
    }
}

/// `uaddr_bestfit_remove` (an entry not in the tree is `panic(9)`, the C's
/// `"uaddr_bestfit_remove: entry was not in tree"`, by `SecondaryTree::remove`'s check).
fn uaddr_bestfit_remove(uaddr: &mut BestfitState, entry: &VmMapEntry) {
    if !uaddr.ubf_free.contains(entry) {
        kpanic(format_args!("uaddr_bestfit_remove: entry was not in tree"));
    }
    uaddr.ubf_free.remove(entry);
}

/// The address tree's element that `e` (an element of a free tree) is: the same entry,
/// borrowed from the tree the result outlives the selector's borrow with.
fn in_tree<'t>(tree: &'t OwnedTree<ByAddr>, e: &VmMapEntry) -> &'t VmMapEntry {
    let start = e.start();
    match tree.find_by(|n| start.cmp(&n.start())) {
        Some(found) => found,
        None => kpanic(format_args!(
            "uvm_addr: a free tree's entry {start:#x} is not in the map"
        )),
    }
}

/// `uaddr_bestfit_select`.
fn uaddr_bestfit_select<'t>(
    cx: &SelectCx<'t>,
    uaddr: &BestfitState,
    sz: usize,
    align: usize,
    offset: usize,
) -> Result<(&'t VmMapEntry, usize), Errno> {
    let guardsz = if cx.map.flags() & VM_MAP_GUARDPAGES != 0 {
        PAGE_SIZE
    } else {
        0
    };
    if sz.wrapping_add(guardsz) < sz {
        return Err(Errno::ENOMEM);
    }

    // Find smallest item on freelist capable of holding item. Deal with guardpages:
    // search for space with one extra page.
    let mut entry = uvm_addr_entrybyspace(&uaddr.ubf_free, sz + guardsz).ok_or(Errno::ENOMEM)?;

    // Walk the tree until we find an entry that fits.
    let (min, max) = loop {
        match uvm_addr_fitspace(
            vmmap_free_start(entry),
            vmmap_free_end(entry),
            sz,
            align,
            offset,
            0,
            guardsz,
        ) {
            Ok(fit) => break fit,
            Err(_) => {
                entry = uaddr.ubf_free.next(entry).ok_or(Errno::ENOMEM)?;
            }
        }
    };

    // Return the address that generates the least fragmentation.
    let addr = if min - vmmap_free_start(entry) <= vmmap_free_end(entry) - guardsz - sz - max {
        min
    } else {
        max
    };
    Ok((in_tree(cx.tree, entry), addr))
}

/*
 * A userspace allocator based on pivots.
 */

/// `uaddr_pivot_random`: a special random function for pivots.
///
/// This function will return: a random number, a multiple of `PAGE_SIZE`, at least
/// `PAGE_SIZE`.
///
/// The random function has a slightly higher change to return a small number.
pub fn uaddr_pivot_random() -> usize {
    // The sum of two six-sided dice will have a normal distribution. We map the highest
    // probable number to 1, by folding the curve (think of a graph on a piece of paper,
    // that you fold).
    //
    // Because the fold happens at PIVOT_RND - 1, the numbers 0 and 1 have the same and
    // highest probability of happening.
    let r = (arc4random_uniform(PIVOT_RND) + arc4random_uniform(PIVOT_RND)) as i32
        - (PIVOT_RND as i32 - 1);
    let r = r.unsigned_abs() as usize;

    // Make the returned value at least PAGE_SIZE and a multiple of PAGE_SIZE.
    (1 + r) << PAGE_SHIFT
}

/// `uaddr_pivot_newpivot`: select a new pivot, `uaddr.up_pivots[pivot]`.
///
/// A pivot must: be chosen random; have a randomly chosen gap before it, where the
/// uaddr_state starts; have a randomly chosen gap after it, before the uaddr_state ends.
///
/// Furthermore, the pivot must provide sufficient space for the allocation. The addr will
/// be set to the selected address.
///
/// Returns `ENOMEM` on failure.
#[allow(clippy::too_many_arguments)] // the C's signature
fn uaddr_pivot_newpivot<'t>(
    cx: &SelectCx<'t>,
    uaddr: &mut PivotState,
    pivot: usize,
    sz: usize,
    align: usize,
    offset: usize,
    before_gap: usize,
    after_gap: usize,
) -> Result<(&'t VmMapEntry, usize), Errno> {
    let mut minaddr = uaddr.up_uaddr.uaddr_minaddr;
    let mut maxaddr = uaddr.up_uaddr.uaddr_maxaddr;
    kassert!(minaddr < maxaddr);
    if cfg!(feature = "diagnostic") && minaddr + 2 * PAGE_SIZE > maxaddr {
        kpanic(format_args!(
            "uaddr_pivot_newpivot: cannot grant random pivot in area less than 2 pages (size = {:#x})",
            maxaddr - minaddr
        ));
    }

    // Gap calculation: 1/32 of the size of the managed area.
    //
    // At most: sufficient to not get truncated at arc4random. At least: 2 PAGE_SIZE.
    //
    // minaddr and maxaddr will be changed according to arc4random.
    let dist = ((maxaddr - minaddr) / 32).max(2 * PAGE_SIZE);
    if dist >> PAGE_SHIFT > 0xffff_ffff {
        minaddr += (arc4random() as usize) << PAGE_SHIFT;
        maxaddr -= (arc4random() as usize) << PAGE_SHIFT;
    } else {
        minaddr += (arc4random_uniform((dist >> PAGE_SHIFT) as u32) as usize) << PAGE_SHIFT;
        maxaddr -= (arc4random_uniform((dist >> PAGE_SHIFT) as u32) as usize) << PAGE_SHIFT;
    }

    // A very fast way to find an entry that will be large enough to hold the allocation,
    // but still is found more or less randomly: the tree path selector has a 50% chance to
    // go for a bigger or smaller entry.
    //
    // Note that the memory may actually be available, but the fragmentation may be so bad
    // and the gaps chosen so unfortunately, that the allocation will not succeed. Or the
    // alignment can only be satisfied by an entry that is not visited in the randomly
    // selected path.
    //
    // This code finds an entry with sufficient space in O(log n) time.
    let PivotState {
        up_free, up_pivots, ..
    } = uaddr;
    let mut path = arc4random();
    let mut found: Option<(&VmMapEntry, usize, usize)> = None;
    let mut entry = up_free.root();
    while let Some(e) = entry {
        let fit = uvm_addr_fitspace(
            vmmap_free_start(e).max(minaddr),
            vmmap_free_end(e).min(maxaddr),
            sz,
            align,
            offset,
            before_gap,
            after_gap,
        );

        // It fits, save this entry.
        if let Ok((min, max)) = fit {
            found = Some((e, min, max));
        }

        // Next.
        entry = if fit.is_err() {
            up_free.right(e)
        } else if path & 0x1 == 0 {
            path >>= 1;
            up_free.right(e)
        } else {
            path >>= 1;
            up_free.left(e)
        };
    }
    let Some((found, found_minaddr, found_maxaddr)) = found else {
        return Err(Errno::ENOMEM); // Not found a large enough region.
    };

    // Calculate a random address within found.
    //
    // found_minaddr and found_maxaddr are already aligned, so be sure to select a multiple
    // of align as the offset in the entry. Preferably, arc4random_uniform is used to
    // provide no bias within the entry. However if the size of the entry exceeds
    // arc4random_uniforms argument limit, we simply use arc4random (thus limiting
    // ourselves to 4G * PAGE_SIZE bytes offset).
    let addr = if found_maxaddr == found_minaddr {
        found_minaddr
    } else {
        kassert!(align >= PAGE_SIZE && align & (align - 1) == 0);
        let arc4_arg = found_maxaddr - found_minaddr;
        if arc4_arg > 0xffff_ffff {
            found_minaddr + (arc4random() as usize & !(align - 1))
        } else {
            found_minaddr + (arc4random_uniform(arc4_arg as u32) as usize & !(align - 1))
        }
    };

    // Set up new pivot and return selected address.
    //
    // Depending on the direction of the pivot, the pivot must be placed at the bottom or
    // the top of the allocation: if the pivot moves upwards, place the pivot at the top of
    // the allocation; if the pivot moves downwards, place the pivot at the bottom of the
    // allocation.
    let p = &mut up_pivots[pivot];
    p.entry.set(Some(found));
    p.dir = if arc4random() & 0x1 != 0 { 1 } else { -1 };
    p.addr = if p.dir > 0 { addr + sz } else { addr };
    p.expire = PIVOT_EXPIRE - 1; // First use is right now.
    Ok((in_tree(cx.tree, found), addr))
}

/// `uaddr_pivot_select`: pivot selector.
///
/// Each time the selector is invoked, it will select a random pivot, which it will use to
/// select memory with. The memory will be placed at the pivot, with a randomly sized gap
/// between the allocation and the pivot. The pivot will then move so it will never revisit
/// this address.
///
/// Each allocation, the pivot expiry timer ticks. Once the pivot becomes expired, it will
/// be replaced with a newly created pivot. Pivots also automatically expire if they fail to
/// provide memory for an allocation.
///
/// Expired pivots are replaced using the `uaddr_pivot_newpivot()` function, which will
/// ensure the pivot points at memory in such a way that the allocation will succeed. As an
/// added bonus, the `uaddr_pivot_newpivot()` function will perform the allocation
/// immediately and move the pivot as appropriate.
///
/// If `uaddr_pivot_newpivot()` fails to find a new pivot that will allow the allocation to
/// succeed, it will not create a new pivot and the allocation will fail.
///
/// A pivot running into used memory will automatically expire (because it will fail to
/// allocate).
///
/// Characteristics of the allocator: best case, an allocation is O(log N) (it would be
/// O(1), if it weren't for the need to check if the memory is free; although that can be
/// avoided...); worst case, an allocation is O(log N) (the `uaddr_pivot_newpivot()`
/// function has that complexity); failed allocations always take O(log N) (the
/// `uaddr_pivot_newpivot()` function will walk that deep into the tree).
#[allow(clippy::too_many_arguments)] // the C's signature
fn uaddr_pivot_select<'t>(
    cx: &SelectCx<'t>,
    uaddr: &mut PivotState,
    sz: usize,
    align: usize,
    offset: usize,
    prot: VmProt,
    hint: usize,
) -> Result<(&'t VmMapEntry, usize), Errno> {
    // When we have a hint, use the rnd allocator that finds the area that is closest to
    // the hint, if there is such an area.
    if hint != 0 {
        return uaddr_rnd_select(cx, &uaddr.up_uaddr, sz, align, offset, prot, hint)
            .map_err(|_| Errno::ENOMEM);
    }

    // Select a random pivot and a random gap sizes around the allocation.
    let pivot = arc4random_uniform(NUM_PIVOTS as u32) as usize;
    let before_gap = uaddr_pivot_random();
    let after_gap = uaddr_pivot_random();
    let p = &uaddr.up_pivots[pivot];
    if p.addr != 0
        && p.expire != 0
        && let Some(entry) = p.entry.get()
    {
        let (fstart, fend) = (vmmap_free_start(entry), vmmap_free_end(entry));
        let entry = in_tree(cx.tree, entry);
        let p = &mut uaddr.up_pivots[pivot];
        // Attempt to use the pivot to map the entry.
        if p.dir > 0 {
            if let Ok((min, _)) = uvm_addr_fitspace(
                fstart.max(p.addr),
                fend,
                sz,
                align,
                offset,
                before_gap,
                after_gap,
            ) {
                p.addr = min + sz;
                p.expire -= 1;
                return Ok((entry, min));
            }
        } else if let Ok((_, max)) = uvm_addr_fitspace(
            fstart,
            fend.min(p.addr),
            sz,
            align,
            offset,
            before_gap,
            after_gap,
        ) {
            p.addr = max;
            p.expire -= 1;
            return Ok((entry, max));
        }
    }

    // Pivot expired or allocation failed. Use pivot selector to do the allocation and
    // find a new pivot.
    uaddr_pivot_newpivot(cx, uaddr, pivot, sz, align, offset, before_gap, after_gap)
}

/// `uaddr_pivot_insert`: insert an entry with free space in the space tree.
fn uaddr_pivot_insert(uaddr: &mut PivotState, entry: &VmMapEntry) {
    let me = core::ptr::from_ref(uaddr).addr();
    if let Some(rb_rv) = uaddr.up_free.insert(entry) {
        kpanic(format_args!(
            "uaddr_pivot_insert: duplicate insertion: state {:#x} inserting entry {:#x} which collides with {:#x}",
            me,
            entry.start(),
            rb_rv.start()
        ));
    }

    let start = vmmap_free_start(entry);
    let end = vmmap_free_end(entry);

    // Update all pivots that are contained in this entry.
    for p in &mut uaddr.up_pivots {
        let mut check_addr = p.addr;
        if check_addr == 0 {
            continue;
        }
        if p.dir < 0 {
            check_addr -= 1;
        }

        if start <= check_addr && check_addr < end {
            kassert!(!p.entry.is_some());
            p.entry.set(Some(entry));
        }
    }
}

/// `uaddr_pivot_remove`: remove an entry with free space from the space tree.
fn uaddr_pivot_remove(uaddr: &mut PivotState, entry: &VmMapEntry) {
    if !uaddr.up_free.contains(entry) {
        kpanic(format_args!("uaddr_pivot_remove: entry was not in tree"));
    }
    uaddr.up_free.remove(entry);

    // Inform any pivot with this entry that the entry is gone. Note that this does not
    // automatically invalidate the pivot.
    for p in &mut uaddr.up_pivots {
        if p.entry.points_to(entry) {
            p.entry.set(None);
        }
    }
}

/// `uaddr_pivot_create`: create a new pivot selector.
///
/// Initially, all pivots are in the expired state. Two reasons for this: it means this
/// allocator will not take a huge amount of time; pivots select better on demand, because
/// the pivot selection will be affected by preceding allocations: the next pivots will
/// likely end up in different segments of free memory, that was segmented by an earlier
/// allocation; better spread.
pub fn uaddr_pivot_create(minaddr: usize, maxaddr: usize) -> Selector {
    let Some(s) = UADDR_PIVOT_POOL.get(PR_WAITOK, || PivotState {
        up_uaddr: UvmAddrState {
            uaddr_minaddr: minaddr,
            uaddr_maxaddr: maxaddr,
        },
        up_free: SecondaryTree::new(),
        up_pivots: [const { Pivot::new() }; NUM_PIVOTS],
    }) else {
        kpanic(format_args!(
            "uaddr_pivot_create: uaddr_pivot_pool is empty"
        ));
    };
    Selector::Pivot(s)
}

/*
 * Stack/break allocator.
 *
 * Stack area is grown into in the opposite direction of the stack growth,
 * brk area is grown downward (because sbrk() grows upward).
 *
 * Both areas are grown into proportially: a weighted chance is used to
 * select which one (stack or brk area) to try. If the allocation fails,
 * the other one is tested.
 */

/// `uaddr_stack_brk_select`: stack/brk address selector.
fn uaddr_stack_brk_select<'t>(
    cx: &SelectCx<'t>,
    uaddr: &UvmAddrState,
    sz: usize,
    align: usize,
    offset: usize,
) -> Result<(&'t VmMapEntry, usize), Errno> {
    // Set up brk search strategy.
    let start = cx.brk.0.max(uaddr.uaddr_minaddr);
    let end = cx.brk.1.min(uaddr.uaddr_maxaddr);
    let before_gap = 0;
    let after_gap = 0;
    let dir = -1; // Opposite of brk() growth.

    if end.wrapping_sub(start) >= sz
        && let Ok(found) = uvm_addr_linsearch(
            cx,
            0,
            sz,
            align,
            offset,
            dir,
            start,
            end.wrapping_sub(sz),
            before_gap,
            after_gap,
        )
    {
        return Ok(found);
    }

    // Set up stack search strategy.
    let start = cx.stack.0.max(uaddr.uaddr_minaddr);
    let end = cx.stack.1.min(uaddr.uaddr_maxaddr);
    let before_gap = ((arc4random() & 0x3) as usize + 1) << PAGE_SHIFT;
    let after_gap = ((arc4random() & 0x3) as usize + 1) << PAGE_SHIFT;
    let dir = 1; // MACHINE_STACK_GROWS_UP is not defined.
    let span = end.wrapping_sub(start);
    if span >= before_gap + after_gap
        && span - before_gap - after_gap >= sz
        && let Ok(found) = uvm_addr_linsearch(
            cx,
            0,
            sz,
            align,
            offset,
            dir,
            start,
            end.wrapping_sub(sz),
            before_gap,
            after_gap,
        )
    {
        return Ok(found);
    }

    Err(Errno::ENOMEM)
}

/// `uaddr_stack_brk_create`.
pub fn uaddr_stack_brk_create(minaddr: usize, maxaddr: usize) -> Selector {
    let Some(s) = UADDR_POOL.get(PR_WAITOK, || UvmAddrState {
        uaddr_minaddr: minaddr,
        uaddr_maxaddr: maxaddr,
    }) else {
        kpanic(format_args!("uaddr_stack_brk_create: uaddr_pool is empty"));
    };
    Selector::StackBrk(s)
}

/// `uvm_mapent_fspace_cmp`: free space comparison. Compares smaller free-space before
/// larger free-space.
pub fn uvm_mapent_fspace_cmp(e1: &VmMapEntry, e2: &VmMapEntry) -> Ordering {
    e1.fspace()
        .cmp(&e2.fspace())
        .then(e1.start().cmp(&e2.start()))
}

hal::secondary_adapter!(
    /// `uaddr_free_rbtree`: the free-space tree of the bestfit and pivot selectors
    /// (`dfree.rbtree`), ordered by `uvm_mapent_fspace_cmp`, over the entries the map's
    /// address tree owns.
    pub ByFspace: ByAddr, free, uvm_mapent_fspace_cmp
);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;
    use std::sync::atomic::Ordering as Mem;
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use hal::arch::current::vmparam::{
        BRKSIZ, MAXSSIZ, USRSTACK, VM_MAXUSER_ADDRESS, VM_MIN_ADDRESS, VM_MIN_KERNEL_ADDRESS,
    };
    use hal::pmap::PmapRef;
    use hal::sys::mman::{
        MADV_NORMAL, MADV_RANDOM, MAP_INHERIT_COPY, MAP_INHERIT_NONE, PROT_READ, PROT_WRITE,
    };

    use super::*;
    use crate::testutil;
    use crate::vm::amap::amap_init;
    use crate::vm::anon::uvm_anon_init;
    use crate::vm::map::{
        VmspaceRef, test_entry, uvm_map, uvm_map_create, uvm_map_deallocate, uvm_map_init,
        uvm_map_set_uaddr, uvm_mapanon, uvm_unmap, uvmspace_alloc, uvmspace_free,
    };
    use crate::vm::types::{UVM_FLAG_COPYONW, UVM_FLAG_FIXED, UVM_UNKNOWN_OFFSET, uvm_mapflag};

    const RW: VmProt = PROT_READ | PROT_WRITE;

    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(256 * 4096);
        testutil::init_shared_once();
        let _ = uvm_anon_init(0);
        let _ = amap_init(0);
        hal::pmap::init();
        uvm_map_init();
        guard
    }

    fn user_vmspace() -> VmspaceRef {
        let vm = uvmspace_alloc(VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS, true, false);
        vm.vm_daddr.store(256 << 20, Mem::Relaxed);
        vm.vm_minsaddr.store(USRSTACK, Mem::Relaxed);
        vm.vm_maxsaddr.store(USRSTACK - MAXSSIZ, Mem::Relaxed);
        vm
    }

    /// The augment of every node is the largest free space of its subtree.
    fn augmented(tree: &OwnedTree<ByAddr>) -> bool {
        tree.iter().all(|e| {
            let mut want = e.fspace();
            for c in [tree.left(e), tree.right(e)].into_iter().flatten() {
                want = want.max(c.fspace_augment());
            }
            e.fspace_augment() == want
        })
    }

    #[test]
    fn fitspace_finds_the_bounds() {
        let p = PAGE_SIZE;
        // Four pages, two wanted: lowest and highest start.
        assert_eq!(
            uvm_addr_fitspace(10 * p, 14 * p, 2 * p, 0, 0, 0, 0),
            Ok((10 * p, 12 * p))
        );
        // Gaps eat into both ends.
        assert_eq!(
            uvm_addr_fitspace(10 * p, 14 * p, 2 * p, 0, 0, p, p),
            Ok((11 * p, 11 * p))
        );
        // Too small, with and without gaps.
        assert_eq!(
            uvm_addr_fitspace(10 * p, 11 * p, 2 * p, 0, 0, 0, 0),
            Err(Errno::ENOMEM)
        );
        assert_eq!(
            uvm_addr_fitspace(10 * p, 14 * p, 2 * p, 0, 0, 2 * p, p),
            Err(Errno::ENOMEM)
        );
        // Alignment moves the lowest start up and the highest down.
        assert_eq!(
            uvm_addr_fitspace(9 * p, 20 * p, p, 4 * p, 0, 0, 0),
            Ok((12 * p, 16 * p))
        );
        assert_eq!(
            uvm_addr_fitspace(9 * p, 20 * p, p, 4 * p, p, 0, 0),
            Ok((9 * p, 17 * p))
        );
        // Inverted range.
        assert_eq!(
            uvm_addr_fitspace(14 * p, 10 * p, p, 0, 0, 0, 0),
            Err(Errno::ENOMEM)
        );
    }

    #[test]
    fn pivot_random_is_pages() {
        testutil::install();
        for _ in 0..256 {
            let r = uaddr_pivot_random();
            assert!(r >= PAGE_SIZE && r & PAGE_MASK == 0);
            assert!(r <= (PIVOT_RND as usize) << PAGE_SHIFT);
        }
    }

    #[test]
    fn fspace_cmp_orders_by_size_then_address() {
        let a = test_entry(0x1000, PAGE_SIZE);
        let b = test_entry(0x2000, 2 * PAGE_SIZE);
        assert_eq!(uvm_mapent_fspace_cmp(&a, &b), Ordering::Less);
        let b = test_entry(0x2000, PAGE_SIZE);
        assert_eq!(uvm_mapent_fspace_cmp(&a, &b), Ordering::Less);
        assert_eq!(uvm_mapent_fspace_cmp(&b, &a), Ordering::Greater);
        assert_eq!(uvm_mapent_fspace_cmp(&a, &a), Ordering::Equal);
    }

    #[test]
    fn bestfit_takes_the_smallest_gap_that_fits() {
        let _g = setup();
        let p = PAGE_SIZE;
        let min = VM_MIN_KERNEL_ADDRESS;
        let max = min + (1 << 30);
        crate::vm::map::UVM_MAXKADDR.store(min, Mem::Relaxed);
        let map = uvm_map_create(PmapRef::Kernel, min, max, crate::vm::map::VM_MAP_PAGEABLE);
        uvm_map_set_uaddr(map, Slot::Any(3), Some(uaddr_bestfit_create(min, max)));

        // [0, 2p) used, a 4-page gap, [6p, 8p) used, an 8-page gap, [16p, 18p) used.
        let fixed = uvm_mapflag(RW, RW, MAP_INHERIT_NONE, MADV_RANDOM, UVM_FLAG_FIXED);
        for (s, e) in [(0, 2), (6, 8), (16, 18)] {
            let mut va = min + s * p;
            uvm_map(
                map,
                &mut va,
                (e - s) * p,
                None,
                UVM_UNKNOWN_OFFSET,
                0,
                fixed,
            )
            .expect("fixed");
        }
        let any = uvm_mapflag(RW, RW, MAP_INHERIT_NONE, MADV_RANDOM, 0);
        let mut va = 0;
        uvm_map(map, &mut va, 3 * p, None, UVM_UNKNOWN_OFFSET, 0, any).expect("bestfit");
        assert!(
            va >= min + 2 * p && va + 3 * p <= min + 6 * p,
            "{va:#x} in the 4-page gap"
        );
        let mut vb = 0;
        uvm_map(map, &mut vb, 5 * p, None, UVM_UNKNOWN_OFFSET, 0, any).expect("bestfit 2");
        assert!(
            vb >= min + 8 * p && vb + 5 * p <= min + 16 * p,
            "{vb:#x} in the 8-page gap"
        );
        {
            let g = map.lock_read();
            assert!(augmented(&g.state().addr));
        }
        for (s, e) in [(0, 2), (6, 8), (16, 18)] {
            uvm_unmap(map, min + s * p, min + e * p);
        }
        uvm_unmap(map, va, va + 3 * p);
        uvm_unmap(map, vb, vb + 5 * p);
        assert_eq!(map.size(), 0);
        uvm_map_deallocate(map);
    }

    #[test]
    fn the_brk_selector_takes_the_top_of_the_brk_area() {
        // With no uaddr_any selector, uvm_map_findspace falls back to uaddr_brk_stack, which
        // searches the brk area downward (the opposite of brk()'s growth).
        let _g = setup();
        let vm = user_vmspace();
        uvm_map_set_uaddr(&vm.vm_map, Slot::Any(0), None);
        let len = 4 * PAGE_SIZE;
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_COPY, MADV_NORMAL, UVM_FLAG_COPYONW);
        let mut a = 0;
        uvm_mapanon(&vm.vm_map, &mut a, len, 0, flags).expect("brk area");
        // The C's: linsearch's `high` (the last start, `end - sz`) is given to
        // uvm_addr_fitspace as the end of the space, so the top fit ends `sz` below it.
        assert_eq!(
            a + 2 * len,
            (256 << 20) + BRKSIZ,
            "{a:#x}: the brk area's top"
        );
        uvm_unmap(&vm.vm_map, a, a + len);
        uvmspace_free(vm);
    }

    #[test]
    fn rnd_addresses_keep_out_of_the_brk_and_stack_areas_and_the_tree_stays_augmented() {
        let _g = setup();
        testutil::set_random_seed(42);
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_COPY, MADV_NORMAL, UVM_FLAG_COPYONW);
        let (b0, b1) = (256 << 20, (256 << 20) + BRKSIZ);
        let (s0, s1) = (USRSTACK - MAXSSIZ, USRSTACK);
        let mut got = Vec::new();
        for i in 0..32 {
            let len = (1 + i % 5) * PAGE_SIZE;
            let mut a = 0;
            uvm_mapanon(map, &mut a, len, 0, flags).expect("rnd");
            assert!(a + len <= b0 || a >= b1, "{a:#x} is out of the brk area");
            assert!(a + len <= s0 || a >= s1, "{a:#x} is out of the stack area");
            got.push((a, len));
            if i % 3 == 2 {
                let (u, l) = got.remove(0);
                uvm_unmap(map, u, u + l);
            }
            let g = map.lock_read();
            assert!(augmented(&g.state().addr), "augmented after {i}");
        }
        for (u, l) in got {
            uvm_unmap(map, u, u + l);
        }
        assert_eq!(map.size(), 0);
        uvmspace_free(vm);
    }
}
/* </TESTS> */
