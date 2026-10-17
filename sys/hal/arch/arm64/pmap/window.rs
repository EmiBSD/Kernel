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
//! arm64: the window of the kernel image's own gigabyte the ABI module `abi_openbsd.ko` runs
//! from, as amd64's (`sys/hal/arch/amd64/pmap/window.rs`, decision 37).
//!
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//!
//! The module loader (`hal::module::load`, decision 22) needs executable pages for the
//! module's text. OpenBSD's arm64 has no executable kernel memory but its image: nothing
//! enters a kernel page `PROT_EXEC` (its `ap_bits_kern[PROT_EXEC]` leaves out `ATTR_PXN`,
//! `pmap.c`, but no caller asks for it). Rather than execute from `kernel_map`, the module
//! runs, on both machines, from a window beside the image:
//!
//! - The bootstrap calls [`init`] once, at its end, on the boot CPU, with the kernel pmap's
//!   copied level-0 and level-1 tables in `TTBR1_EL1`; both boot paths (boot(8)/efiboot
//!   through `locore0.S`, and Limine) end there. In hal's level-1 copy it finds the entry of
//!   the gigabyte the image starts in (`kernel.ld`'s `0xffffffff80000000`): the level-2 table
//!   the bootstrap's `pmap_map_stolen` built for the image (N3.1 commit 8), which no vp table
//!   of ours describes and nothing frees, one level-3 table per 2 MiB of the image, the same
//!   on both boot paths. It links the [`MODULE_WINDOW_SIZE`] bytes after the image's last
//!   entry to level-3 tables the window owns ([`MODULE_WINDOW_PTPS`], in the kernel's
//!   `.bss`), by table descriptors without `Lx_PT_PXN`: the leaves decide.
//! - The window is outside the kernel's managed range (`VM_MIN_KERNEL_ADDRESS` to
//!   `VM_MAX_KERNEL_ADDRESS`: `vmmap` and `kernel_map`, checked), so uvm never hands it out,
//!   `pmap_growkernel` never reaches it, and no table above it is ever freed.
//! - [`enter`] enters one page in it, its leaf composed as `pmap_pte_update` composes a
//!   write-back kernel mapping (`ap_bits_kern`: `ATTR_PXN` unless `PROT_EXEC`, `ATTR_UXN`,
//!   read-only unless `PROT_WRITE`; the kernel pmap's `ATTR_GP`), the instruction cache made
//!   coherent with the page first when it is executable (`pmap_icache_sync_page`, as
//!   `pmap_enter` does for an executable page); it refuses a writable executable page, a page
//!   entered already, an address outside the window and a physical address that is not a
//!   page's. The module is never unloaded, so no leaf is ever removed: the window lives for
//!   the kernel's lifetime.
//!
//! What executes after this: in the window, only the leaves entered `PROT_EXEC` (the
//! module's text); in the managed range (`vmmap`, `kernel_map`) nothing, since the kernel
//! pmap refuses `PROT_EXEC` and the range's table descriptors carry `Lx_PT_PXN`
//! (`backend.rs`, deviations); in the image, its text alone, read-only (`pmap_map_stolen`,
//! `backend.rs`, redesign: the read-only data and the data are never executable, and
//! nothing past the image is mapped there). The loader's writable copy is a `kernel_map`
//! range, unmapped before the module's entry point is called. A stray kernel write to the
//! module's text or read-only data faults on a read-only leaf outside every uvm map: a
//! kernel panic, the W^X behaviour wanted for the module.
//!
//! `pmap_extract` of a window address on the kernel pmap answers `None`: the window's leaves
//! have no software descriptor, and the image's level-1 entry has no vp table of ours below
//! it (`backend.rs`'s `extract`), where amd64 walks the hardware tables. No caller extracts a
//! window address today: the loader extracts the writable copy's `kernel_map` address and
//! enters that page here.
//!
//! ## Redesign
//! - New in native (N3.1 commit 7, decision 37's symmetry at N3): until it, arm64 placed the
//!   module in `kernel_map` (`bsd`'s `ModulePlatform::reserve` and `enter`, `pmap_enter` of
//!   a kernel range `PROT_READ | PROT_EXEC`). The window re-expresses for the module what the
//!   C's `pmap_set_l2`/`pmap_set_l3` and `pmap_pte_update` do for a kernel mapping: level-3
//!   tables hung under level-2 table descriptors, leaves with `ap_bits_kern`'s bits.
//! - The level-3 tables are a typed static the window owns ([`Ptp`], `[AtomicU64; 512]` per
//!   page), written with atomics, not vp tables of the page provider: they need no software
//!   page (nothing walks them but the MMU and [`enter`]), and the `.bss` is memory both boot
//!   loaders hand over zeroed, mapped and outside every free region.
//! - The image's level-2 table is reached through the frame view of the direct map
//!   (`vp.rs`), as the bootstrap reaches the tables it builds: the window has no `unsafe`.
//! - The link is attempted once ([`MODULE_WINDOW_TRIED`]): a refused first attempt leaves the
//!   window empty for good, and the module loader reports it has no memory for the module
//!   (decision 29's fail-soft); it is never placed in `kernel_map` instead. Two limits of
//!   that promise: reaching the image's level-2 table panics (`vp.rs`'s `check_frame`)
//!   rather than refusing when that table lies outside the direct map (a stolen frame, it
//!   lies inside on both boot paths); and a refused link reaches the boot console as the
//!   same `NoMemory` line as a failed `km_alloc` of the writable copy, with no line of its
//!   own.

use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use super::super::include::param::PAGE_SIZE;
use super::super::include::pte::{L1_SIZE, Lx_TYPE_MASK, Lx_TYPE_PT, PTE_RPGN};
use super::backend::{Backend, kernel_leaf, pmap_icache_sync_page};
use super::cpufunc::{cpu_tlb_flush, cpu_tlb_flush_all_asid};
use super::layout::{
    MODULE_WINDOW_NPTP, MODULE_WINDOW_SIZE, VP_ENTRIES, module_prot_ok, module_window_place,
    tlbi_page, vp_idx,
};
use super::vp::words;
use crate::pmap::{PmapBackend, VmProt};
use crate::sys::mman::PROT_EXEC;
use crate::sys::types::{Paddr, Vaddr};

/// One level-3 page-table page, as the window owns it: the MMU reads it by its physical
/// address, the window writes it through its kernel address, with atomics.
#[repr(C, align(4096))]
struct Ptp([AtomicU64; VP_ENTRIES]);

impl Ptp {
    /// An empty table: every entry invalid.
    const fn new() -> Self {
        Self([const { AtomicU64::new(0) }; VP_ENTRIES])
    }
}

/// The window's level-3 tables, in the kernel's `.bss`: part of the image on both boot
/// paths, so never memory uvm manages.
static MODULE_WINDOW_PTPS: [Ptp; MODULE_WINDOW_NPTP] = [const { Ptp::new() }; MODULE_WINDOW_NPTP];

/// The window's first address, published once the window is linked; 0 while there is none.
static MODULE_WINDOW_VA: AtomicUsize = AtomicUsize::new(0);

/// Set by the first call of [`init`], whatever it achieved (see the redesign).
static MODULE_WINDOW_TRIED: AtomicBool = AtomicBool::new(false);

/// Links the module window into the level-2 table of the gigabyte `image` (the kernel
/// image's first address, `BootstrapParams::image`) starts in (see the module's
/// documentation). Once: the first call, from the end of the bootstrap on the boot CPU, is
/// the only one that may link; every later call returns at once. A first call without an
/// image, with the image's gigabyte outside the kernel's level-1 table or mapped by a block,
/// or with no room after its last entry, links nothing: the window stays empty. A level-2
/// table outside the direct map panics instead (`words`, see the redesign).
pub(super) fn init(image: Option<Vaddr>) {
    if MODULE_WINDOW_TRIED.swap(true, Ordering::AcqRel) {
        return;
    }
    let Some(image) = image else {
        return;
    };
    let kpm = Backend::kernel();
    let gib = image.as_usize() & !(L1_SIZE - 1);

    // hal's level-1 copy (the bootstrap's `vp1`), and in it the image's level-2 table
    // (`pmap_map_stolen`'s): a table descriptor no vp table of ours hangs from.
    let Some(l1) = kpm.md.root().and_then(|r| r.child(vp_idx(0, gib))) else {
        return;
    };
    let i1 = vp_idx(1, gib);
    if l1.child(i1).is_some() {
        return;
    }
    let d = l1.entry(i1).load(Ordering::Acquire);
    if d & Lx_TYPE_MASK != Lx_TYPE_PT {
        return;
    }
    let l2 = words(Paddr::new((d & PTE_RPGN) as usize));

    // The window starts after the image's last entry and stays out of the kernel's managed
    // range.
    let Some(last) = l2.iter().rposition(|e| e.load(Ordering::Relaxed) != 0) else {
        return;
    };
    let Some((first, base)) = module_window_place(gib, last) else {
        return;
    };
    let Some(slots) = l2.get(first..first + MODULE_WINDOW_NPTP) else {
        return;
    };

    // The tables' physical addresses (each page on its own, through the image's offset),
    // then each one emptied before the MMU can see it.
    let mut ptp_pa = [0u64; MODULE_WINDOW_NPTP];
    for (pa, ptp) in ptp_pa.iter_mut().zip(&MODULE_WINDOW_PTPS) {
        let Some(p) = Backend::extract(kpm, Vaddr::new(ptr::from_ref(ptp).addr())) else {
            return;
        };
        let p = p.as_usize() as u64;
        if p & !PTE_RPGN != 0 {
            return;
        }
        *pa = p;
        for e in &ptp.0 {
            e.store(0, Ordering::Relaxed);
        }
    }

    // Link them: each entry must still be empty (the boot CPU is alone, so it is), else the
    // window is not published; an entry linked before the refusal maps nothing (its table is
    // empty) and stays so. `VP_Lx(pa)`, without `Lx_PT_PXN`: the leaves decide.
    for (slot, pa) in slots.iter().zip(ptp_pa) {
        if slot
            .compare_exchange(0, pa | Lx_TYPE_PT, Ordering::Release, Ordering::Relaxed)
            .is_err()
        {
            return;
        }
    }
    // The entries were invalid, and the TLB caches no invalid entry; the flush's `dsb ishst`
    // makes the table writes visible to the walkers before the window is published.
    cpu_tlb_flush();
    MODULE_WINDOW_VA.store(base, Ordering::Release);
}

/// The module window: its first address and its size; empty (size 0) until the bootstrap
/// linked it, or when the image's gigabyte had no room for it. arm64 places the module
/// nowhere else.
pub(super) fn window() -> (Vaddr, usize) {
    let va = MODULE_WINDOW_VA.load(Ordering::Acquire);
    if va == 0 {
        (Vaddr::new(0), 0)
    } else {
        (Vaddr::new(va), MODULE_WINDOW_SIZE)
    }
}

/// Enters the page at `pa` at `va` in the module window, with `prot`, wired and for the
/// kernel's lifetime; `false`, entering nothing, when `va` is not a page of the window, the
/// page is entered already, `pa` is not a page's physical address, or `prot` is `PROT_NONE`
/// or both writable and executable.
pub(super) fn enter(va: Vaddr, pa: Paddr, prot: VmProt) -> bool {
    let base = MODULE_WINDOW_VA.load(Ordering::Acquire);
    let (va, pa) = (va.as_usize(), pa.as_usize() as u64);
    if base == 0 || !va.is_multiple_of(PAGE_SIZE) || pa & !PTE_RPGN != 0 {
        return false;
    }
    if !module_prot_ok(prot) {
        return false;
    }
    let Some(page) = va.checked_sub(base).map(|off| off / PAGE_SIZE) else {
        return false;
    };
    let Some(pte) = MODULE_WINDOW_PTPS
        .get(page / VP_ENTRIES)
        .and_then(|ptp| ptp.0.get(page % VP_ENTRIES))
    else {
        return false;
    };
    if pte.load(Ordering::Acquire) != 0 {
        return false;
    }

    // What was written through the loader's copy reaches instruction fetch before the page
    // can be executed (`pmap_enter`'s `pmap_icache_sync_page` for an executable page).
    if prot & PROT_EXEC != 0 {
        pmap_icache_sync_page(pa as usize);
    }
    // Only an empty entry is taken, so no CPU can hold a translation of the page; the flush
    // orders the write before any walk (`ttlb_flush` of the kernel pmap).
    if pte
        .compare_exchange(
            0,
            kernel_leaf(pa, prot, Backend::kernel().md.guarded()),
            Ordering::AcqRel,
            Ordering::Relaxed,
        )
        .is_err()
    {
        return false;
    }
    cpu_tlb_flush_all_asid(tlbi_page(va));
    true
}

const _: () = {
    assert!(size_of::<Ptp>() == PAGE_SIZE);
    assert!(MODULE_WINDOW_NPTP * VP_ENTRIES * PAGE_SIZE == MODULE_WINDOW_SIZE);
};
/* </CODE> */
