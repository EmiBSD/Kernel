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

/*
 * Copyright (c) 2008,2009,2014 Dale Rahn <drahn@dalerahn.com>
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
//! arm64's pmap behind `hal::pmap`: `pmap.c`'s pmaps, pte descriptors, pv lists and mapping
//! functions over the vp tables of `vp.rs`, the ASIDs of `asid.rs` and the TLB and cache
//! operations of `cpufunc.rs`.
//!
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/include/pmap.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//! LZ: sys/arch/arm64/include/pmap.rs@3c62ede68802
//!
//! **Locks** (as the C): a user pmap's `pm_mtx` (`IPL_VM`) is taken before a page's `pv_mtx`
//! (`IPL_VM`); the kernel pmap is never locked (its [`Locked`] witness takes no lock); vp
//! tables and descriptors are allocated under `pm_mtx` (`PR_NOWAIT`), so the page allocator's
//! locks nest inside it; a managed mapping is removed inside its page's `pv_mtx`, taken under
//! `pm_mtx` (the removal rule, `vp.rs`: no walker holding `pv_mtx` ever sees a linked
//! descriptor whose PTE was zeroed); `pmap_page_protect` takes a reference to the pmap under
//! `pv_mtx`, drops it, takes `pm_mtx`, re-takes `pv_mtx` and re-checks; `pmap_asid_mtx`
//! (`IPL_HIGH`) is taken on
//! the context switch's path (`Backend::load`). There is no TLB shootdown IPI: every
//! invalidation is the inner-shareable broadcast `tlbi`, so arm64 registers no `TlbCpus`.
//!
//! ## Deviations
//! - OpenBSD arm64 has no direct map and no `PMAP_STEAL_MEMORY`: boot allocations go through
//!   `pmap_steal_avail` (a bump allocator over `pmap_avail[]`, mapped by `pmap_map_stolen`)
//!   and `uvm_pageboot_alloc` maps page by page with `pmap_kenter_pa`. As in LZ, the boot
//!   loader's higher-half direct map (Limine's HHDM, or `locore0.S`'s `DIRECT_BASE` after
//!   boot(8)) serves as `__HAVE_PMAP_DIRECT`, boot memory is stolen through the page
//!   provider (`PhysPages::steal`, `uvm_page_physsteal`), and vp tables, descriptor pages and
//!   `pmap_zero_page`/`pmap_copy_page`/the I-cache syncs reach their frames through it,
//!   without the C's per-CPU `zero_page`/`copy_*_page` windows.
//! - The kernel's level-0 table and the level-1 table of the kernel range's slot start as
//!   copies of the boot loader's (`pmap_bootstrap` copies them into fresh vp tables), so the
//!   direct map's tables (another level-0 slot) stay the boot loader's: they have no
//!   descriptor, and `pmap_extract` answers for them through the direct map. The C builds
//!   the kernel tables from scratch (`pmap_setup_avail`) and pre-populates the first GiB;
//!   here the boot glue's `pmap_growkernel` populates it, so `pmap_maxkvaddr` starts at
//!   `VM_MIN_KERNEL_ADDRESS`, and the memory is loaded into uvm before `pmap_bootstrap`.
//! - The kernel image is mapped as the C's `pmap_map_stolen` maps it (see the redesign), on
//!   both boot paths, with these differences: only the image is mapped so (the C maps every
//!   stolen region; here those are reached through the direct map), from its first page to
//!   the page holding `_end` (the C maps up to `esym`: boot(8)'s symbol table past `_end` is
//!   not mapped, since `ddb` loads no symbols and the ABI module efiboot puts there is read
//!   through the direct map); the padding between two sections is not mapped (the C maps it
//!   read-write); the image's leaves carry no `ATTR_GP` (the C's kernel pmap guards its text
//!   for branch target identification; this kernel is built without BTI landing pads, which
//!   stable rustc cannot emit); and they have no software descriptor, so `pmap_extract`
//!   answers for the image through its virtual-to-physical offset (`BootstrapParams::image`).
//!   The same holds for the module window's leaves (`window.rs`), which the C does not have:
//!   no kernel leaf `kernel_leaf` composes is guarded. The C prints `removing ...` twice
//!   (`pmap_remove_avail`) and `all mapped` after the switch; native prints neither, as LZ
//!   did not: printing them would add boot lines the smokes and dmesg never had, and the
//!   memory `removing` reports is handed to uvm before the bootstrap here.
//! - The direct map (another level-0 slot, the boot loader's tables below hal's copy of its
//!   level-0 entries) gets `Lx_PT_PXN` and `Lx_PT_XN` on those entries, so nothing below them
//!   executes whatever the boot loader's leaves say (Limine's were never checked; `locore0.S`'s
//!   blocks are `ATTR_XN`), and `pmap_init` clears `TCR_EL1.HPD0`/`HPD1` so those table bits
//!   count (both are clear under both boot loaders today: `locore.rs`'s `tcr` word, and the
//!   `TCR_EL1` Limine 12.9.3 writes, `0xb5003500` with the sizes, `IPS` and `DS`); the C's
//!   `locore.S` never sets them. The direct map still maps every physical page read-write,
//!   the image's included: the kernel text has a writable alias there, as on amd64, whose
//!   OpenBSD direct map maps the kernel's physical pages too; the C's arm64 has no direct map.
//! - The kernel pmap is four-level (`TCR_EL1.T1SZ` = 16, as both boot paths leave it); the
//!   C's is three-level (a 39-bit kernel), so where the C reaches `pm_vp.l1` directly the walks
//!   go through level 0.
//! - `MAIR_EL1` indices 2, 3 and 4 are programmed by `pmap_bootstrap` around the boot loader's
//!   0 and 1 (`include/pte.rs`, deviations); the C's `locore.S` writes the whole register.
//! - `pmap_init`'s `TTBR0_EL1`/`TCR_EL1` switch is its own function, [`init_lower_half`]: the
//!   console is remapped into the kernel half (`pluartcn_remap`, `bsd`'s, which needs
//!   `pmap_initialized`) between the pools and the switch, since the boot glue's device map
//!   lives in the lower half (the C's locore never had one there). It also resizes `T0SZ`
//!   to `USER_SPACE_BITS`, as the C's.
//! - `vmmap` is the kernel virtual window below `virtual_avail` that `bus_space`'s
//!   `generic_space_map` takes device mappings from ([`VMMAP`], [`VMMAP_SIZE`]), 1024 pages
//!   where the C keeps one page (`pmap_bootstrap_bs_map`).
//! - Pointer authentication (`pm_api*key`, `pmap_setpauthkeys`) and the trampoline pmap
//!   (`pmap_tramp`, `pmap_postinit`'s `pmap_kvp_map` submap) are not configured, as in LZ;
//!   `pmap_vp_populate`/`__HAVE_PMAP_POPULATE` stays unconfigured (uvm_fault's slice decides,
//!   N3.6): `uvm_fault` never calls it here.
//! - `pmap_proc_iflush` syncs every page through its direct-map alias (the C syncs the user
//!   range directly for the current process, through `zero_page` otherwise), and skips a
//!   frame that is not a managed page (a device page mapped into a traced process): it may lie
//!   outside the direct map, and device memory is not instruction-cached.
//! - `pool_setlowat` (the pteds' 20, the pmaps' 2) is not called, as in LZ: the pool bridge has
//!   no low-water mark (`hal::pool`).
//! - `pmap_copy_page`'s `splbio` (the buffer flipper's interrupt-context use) is not needed:
//!   the copy maps nothing.
//! - Stricter than OpenBSD's arm64, whose kernel leaves may execute (`ap_bits_kern[PROT_EXEC]`
//!   carries no `ATTR_PXN`, `pmap.c`) and whose tables carry no `PXNTable`, though nothing
//!   asks a kernel page to: the kernel pmap refuses `PROT_EXEC` (`pmap_enter`,
//!   `pmap_kenter_pa` and `pmap_kenter_cache` panic), and the table descriptors
//!   `pmap_growkernel` makes for the managed range (`vmmap` and `kernel_map`, the level-1
//!   slots [`kernel_map_slot`] names) carry `Lx_PT_PXN`, so nothing below them executes at
//!   EL1 whatever a leaf says: amd64's posture, whose directories carry `pg_nx`. The image's
//!   tables (`pmap_map_stolen`'s, under the image's own level-1 entry, never set by
//!   `pmap_growkernel`), the direct map's (another level-0 slot) and the EFI runtime pmap's
//!   (its own root) are not touched; the only executable kernel memory is the image's text
//!   and the module window (`window.rs`), whose tables hang from the image's level-2 table.
//!   A table `pmap_vp_enter` would hang for the kernel pmap beyond `pmap_growkernel`'s reach
//!   (uvm never asks) has no `Lx_PT_PXN`, but its leaves are refused `PROT_EXEC` all the
//!   same.
//!
//! ## Redesign
//! - `struct pmap` is `hal::pmap::Pmap` around [`PmapMd`]: `pm_mtx` as a `Mutexed<()>` whose
//!   guard is the [`Locked`] witness of every walk of a user pmap's descriptors, the root
//!   table (`pm_vp`: its two pages), `have_4_level_pt`, `pm_privileged`, `pm_pt0pa`,
//!   `pm_asid`, `pm_active`, `pm_guarded` as atomics, and the limit (`VM_MAXUSER_ADDRESS`, or
//!   the EFI pmap's `1 << EFI_SPACE_BITS`). `pm_refs` is the pool handle's count
//!   (`PmapRef`); the descriptors in user slots are counted ([`PmapMd`]'s `mapped`) for the
//!   last drop's check.
//! - `struct vm_page_md` is `hal::pmap::PageMd`: the pv list (`vp.rs`'s `PvHead`, a pv link
//!   under `pv_mtx`) and the `PG_PMAP_MOD`/`REF`/`EXE` bits the C keeps in `pg_flags`.
//! - A user mapping's `struct pte_desc` is a `SharedBox<PteDesc>` of the pool `"pted"`, owned
//!   by its level-3 slot; the kernel's come from pages of descriptors (`pmap_kpted_alloc`).
//!   Its pmap is a pointer dereferenced in one place ([`pted_pmap`]): a pmap with a mapping is
//!   alive, since `hal::pmap`'s last drop panics while one is left; who must hold the pmap
//!   after dropping `pv_mtx` (`pmap_page_protect`) rebuilds a counted handle
//!   ([`pmap_reference_pted`], `PoolArc::from_value`), dropped once both locks are released.
//! - `pmap_setttb`'s body is `Backend::load` (the ASID generation check, the empty table,
//!   `cpu_setttb`); its unmangled symbol, `ci_curpm`, `ci_flush_bp` stay in `bsd`'s
//!   `cpuswitch.rs` (the caller). `pmap_activate`'s and `pmap_deactivate`'s process halves are
//!   the caller's too; `efi_enter`'s and `efi_leave`'s register switches are
//!   `Backend::load_efi`/`leave_efi`.
//! - `pmap_destroy` + `pmap_release` are the last handle's drop: the ASIDs freed (their TLB
//!   entries dropped on every CPU) before the tables go back to the provider, where the C
//!   frees the tables to its pool first.
//! - `pmap_steal_memory`, `pmap_kvp_alloc` and `pmap_kpted_alloc` take their frames from the
//!   page provider (`steal` before the page system is up, `alloc_ptp` after), where LZ called
//!   `uvm_page_physsteal`/`uvm_pglistalloc`.
//! - The ABI module's window (`window.rs`, N3.1 commit 7, decision 37's symmetry): the
//!   bootstrap links it at its end, `module_window`/`module_enter` are its, and the window's
//!   leaves are [`kernel_leaf`]s, `pmap_pte_update`'s write-back kernel descriptor; until
//!   then arm64 placed the module in `kernel_map`.
//! - The image by section and the switch (N3.1 commit 8; LZ kept the boot loader's tables
//!   below its level-1 copy, so under boot(8)/efiboot the image ran on `locore0.S`'s 32
//!   level-2 blocks over the whole 64 MB block, read-write and executable at EL1, the block's
//!   tail past the image, uvm's memory, aliased there too): [`pmap_map_stolen`] maps the image
//!   in tables of its own, stolen frames through the direct map, as the C's maps it with
//!   `pmap_kenter_cache`: the text (`__text_start` to `_etext`) read-only and executable at
//!   EL1, the read-only data (`__rodata_start` to `_erodata`) read-only, the data and BSS
//!   (`__data_start` to `_end`) read-write, every leaf `ATTR_UXN` and the last two
//!   `ATTR_PXN`, nothing after the image (`layout.rs`'s `image_page_prot`, the bounds checked
//!   by `image_layout_ok`); their level-2 table replaces the boot loader's entry for the
//!   image's gigabyte in the level-1 copy, whatever the boot loader built there (Limine's
//!   mapping or `locore0.S`'s blocks). [`switch_mmu_kernel_table`] then loads the copy into
//!   `TTBR1_EL1` as the C's `switch_mmu_kernel_table(pt1pa)` does: at the switch routine's
//!   physical address, through a four-page identity table in `TTBR0_EL1` that maps that page
//!   alone, so no instruction runs through `TTBR1_EL1` while the kernel's live entries change
//!   size and attributes (break-before-make); `TTBR0_EL1` and its `TCR_EL1` fields go back
//!   as they were after it. The identity table stays (Limine's application processors switch
//!   through it too, `cpu.rs`'s `cpu_hatch_entry`), as do the boot loader's tables, which
//!   nothing walks any more: under boot(8) `locore.S`'s `pagetable` pages in the kernel's
//!   `.data` (the direct map's level-1 table among them, still in use), under Limine its
//!   bootloader-reclaimable pages (the application processors start on them). N3.1 8b: the
//!   switch builds its `TTBR1_EL1` value itself (the kernel root, ASID 0, `CnP` clear) and
//!   refuses a CPU already on the kernel's tables with another value, so no caller can load
//!   the kernel's tables under a user pmap's ASID.

use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};

use super::super::include::param::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use super::super::include::pte::{
    ATTR_AF, ATTR_GP, ATTR_PXN, ATTR_UXN, ATTR_nG, L2_SIZE, L3_P, Lx_PT_PXN, Lx_PT_XN,
    Lx_TYPE_MASK, Lx_TYPE_PT, Lx_TYPE_S, MAIR_CI, MAIR_DEV_NGNRNE, MAIR_WT, PTE_ATTR_CI,
    PTE_ATTR_DEV_NGNRE, PTE_ATTR_DEV_NGNRNE, PTE_ATTR_WB, PTE_ATTR_WT, PTE_RPGN, SH_INNER, attr_ap,
    attr_idx, attr_sh, mair_attr,
};
use super::super::include::vmparam::{
    USER_SPACE_BITS, VM_MAX_KERNEL_ADDRESS, VM_MAXUSER_ADDRESS, VM_MIN_KERNEL_ADDRESS,
};
use super::asid::{self, pmap_allocate_asid, pmap_free_asid};
use super::cpufunc::{
    SWITCH_ROUTINE_LEN, cpu_dcache_wbinv_range, cpu_icache_sync_range, cpu_idcache_wbinv_range,
    cpu_setttb, cpu_tlb_flush, cpu_tlb_flush_all_asid, cpu_tlb_flush_asid, cpu_tlb_flush_asid_all,
    cpu_ttbr1, switch_routine_va, ttbr0_empty,
};
use super::layout::{
    AP_BITS_KERN, ASID_USER, EFI_SPACE_BITS, PG_PMAP_EXE, PMAP_ASID_MASK, PMAP_CACHE_BITS,
    PMAP_CACHE_CI, PMAP_CACHE_DEV_NGNRE, PMAP_CACHE_DEV_NGNRNE, PMAP_CACHE_WB, PMAP_CACHE_WT,
    PMAP_NOCACHE, PMAP_PA_MASK, PMAP_WC, PROT_MASK, PTED_VA_MANAGED_M, PTED_VA_WIRED_M,
    T1SZ_4LEVEL, TCR_A1, TCR_HPD, TCR_IDENTITY_TTBR0, TCR_T0SZ_MASK, TCR_TTBR0_FIELDS,
    TTBR_BADDR_MASK, VP_IDX2_CNT, VP_IDX3_CNT, cache_of_pa, cache_valid, image_layout_ok,
    image_page_prot, image_ptps, kernel_leaf, kernel_map_slot, tcr_t1sz, tlbi_asid, tlbi_page,
    vp_idx,
};
use super::vp::{
    self, L3, Link, Locked, PTEDS_PER_PAGE, PteDesc, PvHead, PvKind, Table, copy_frame, direct_map,
    direct_unmap, pted_page, pv_insert, pv_unlink, set_direct_map, words, zero_frame,
};
use super::window;
use crate::assert::{hal_panic, kassert};
use crate::intr::{IPL_NONE, IPL_VM, splraise};
use crate::mutex::{Mutexed, MutexedGuard};
use crate::pmap::{
    BootstrapParams, KernelImage, PG_PMAP_MOD, PG_PMAP_REF, PMAP_CANFAIL, PMAP_WIRED, PageMd,
    PhysPages, Pmap, PmapBackend, PmapCounter, PmapRef, PmapStatistics, VmProt,
};
use crate::pool::{BoxPool, PR_NOWAIT, PoolArc};
use crate::sys::errno::Errno;
use crate::sys::mman::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};
use crate::sys::types::{Paddr, Vaddr, Vsize};
use crate::vm::param::{atop, round_page, trunc_page};

/// The size of the `vmmap` window (see the deviations): large enough for the devices, wholly
/// below `virtual_avail`, out of `kernel_map`'s way.
pub const VMMAP_SIZE: usize = 1024 * PAGE_SIZE;

/// `ap_bits_user[]`: the access bits of a user mapping, by `PROT_*`.
const AP_BITS_USER: [u64; 8] = [
    0,                                          // PROT_NONE
    ATTR_PXN | ATTR_UXN | ATTR_AF | attr_ap(3), // PROT_READ
    ATTR_PXN | ATTR_UXN | ATTR_AF | attr_ap(1), // PROT_WRITE
    ATTR_PXN | ATTR_UXN | ATTR_AF | attr_ap(1), // PROT_WRITE|PROT_READ
    ATTR_PXN | ATTR_AF | attr_ap(2),            // PROT_EXEC
    ATTR_PXN | ATTR_AF | attr_ap(3),            // PROT_EXEC|PROT_READ
    ATTR_PXN | ATTR_AF | attr_ap(1),            // PROT_EXEC|PROT_WRITE
    ATTR_PXN | ATTR_AF | attr_ap(1),            // PROT_EXEC|PROT_WRITE|PROT_READ
];

/// The `MAIR_EL1` fields `pmap_bootstrap` writes (indices 2, 3, 4) and what it keeps.
const MAIR_SET: u64 = mair_attr(MAIR_DEV_NGNRNE, 2) | mair_attr(MAIR_CI, 3) | mair_attr(MAIR_WT, 4);
/// The `MAIR_EL1` fields of the boot loader that stay (indices 0, 1, 5, 6, 7).
const MAIR_KEEP: u64 = !(mair_attr(0xff, 2) | mair_attr(0xff, 3) | mair_attr(0xff, 4));

/// The machine part of `struct pmap`.
///
/// Invariant: the root (both pages) is set before the pmap is shared (`pmap_bootstrap` for
/// the kernel pmap, `create`/`create_efi` for a user one) and stays until the pmap is freed;
/// `pt0pa` is the root's hardware page or the kernel's empty table.
pub(crate) struct PmapMd {
    /// `pm_mtx`: a user pmap's tables, descriptors and statistics; the kernel pmap never
    /// takes it (`vp::Locked::of` alone takes it).
    pub(super) mtx: Mutexed<()>,
    /// `pm_vp`'s hardware page (the top table, level 0 or 1).
    root_hw: AtomicU64,
    /// `pm_vp`'s software page.
    root_sw: AtomicU64,
    /// `have_4_level_pt`.
    four_level: AtomicBool,
    /// `pm_privileged`: the kernel's access bits (`ap_bits_kern`).
    privileged: AtomicBool,
    /// Made by `create_efi`: the EFI runtime pmap, loaded only by `load_efi`.
    efi: bool,
    /// `pm_pt0pa`: the table `TTBR0_EL1` gets.
    pt0pa: AtomicU64,
    /// `pm_asid`: the ASID pair's even member and its generation.
    asid: AtomicU64,
    /// `pm_active`: the threads using it.
    active: AtomicI32,
    /// `pm_guarded`: `ATTR_GP` on executable pages (BTI), or 0.
    guarded: AtomicU64,
    /// The first address the pmap may not map.
    limit: usize,
    /// The descriptors in a user pmap's slots (`Backend::mappings`).
    mapped: AtomicUsize,
}

impl PmapMd {
    /// The kernel pmap's part before the bootstrap.
    pub(crate) const fn new() -> Self {
        Self::with(0, 0, true, true, false, usize::MAX)
    }

    /// A pmap's part over the root `(hw, sw)`.
    const fn with(
        hw: u64,
        sw: u64,
        four_level: bool,
        privileged: bool,
        efi: bool,
        limit: usize,
    ) -> Self {
        Self {
            mtx: Mutexed::new(IPL_VM, ()),
            root_hw: AtomicU64::new(hw),
            root_sw: AtomicU64::new(sw),
            four_level: AtomicBool::new(four_level),
            privileged: AtomicBool::new(privileged),
            efi,
            pt0pa: AtomicU64::new(hw),
            asid: AtomicU64::new(0),
            active: AtomicI32::new(0),
            guarded: AtomicU64::new(0),
            limit,
            mapped: AtomicUsize::new(0),
        }
    }

    /// `pm_vp`: the root table, `None` before the bootstrap (the kernel's) or after the
    /// release.
    pub(super) fn root(&self) -> Option<Table> {
        let hw = self.root_hw.load(Ordering::Acquire);
        let sw = self.root_sw.load(Ordering::Acquire);
        (hw != 0 && sw != 0).then(|| Table {
            hw: Paddr::new(hw as usize),
            sw: Paddr::new(sw as usize),
        })
    }

    /// `have_4_level_pt`.
    pub(super) fn four_level(&self) -> bool {
        self.four_level.load(Ordering::Relaxed)
    }

    /// `pm_pt0pa`.
    pub(super) fn pt0pa(&self) -> u64 {
        self.pt0pa.load(Ordering::Acquire)
    }

    /// `pm_asid`.
    pub(super) fn asid(&self) -> u64 {
        self.asid.load(Ordering::Acquire)
    }

    /// Sets `pm_asid` (under `pmap_asid_mtx`).
    pub(super) fn set_asid(&self, asid: u64) {
        self.asid.store(asid, Ordering::Release);
    }
}

/// What `pmap_growkernel` keeps between calls, under its own lock (the C's `splvm` and the
/// statics of `pmap_kpted_alloc`).
struct Grow {
    /// `pmap_maxkvaddr`: how far the kernel's vp tables and descriptors reach.
    maxkvaddr: usize,
    /// The page `pmap_kpted_alloc` carves (`pted`)...
    kpted: Paddr,
    /// ...and how many descriptors are left in it (`npted`).
    npted: usize,
}

/// `kernel_pmap_`: the kernel's pmap.
static KERNEL_PMAP: Pmap = Pmap::kernel_with(PmapMd::new());
/// `pmap_pted_pool`: the pool of user mappings' descriptors.
static PMAP_PTED_POOL: BoxPool<PteDesc> = BoxPool::new();
/// `pmap_initialized`.
static PMAP_INITIALIZED: AtomicBool = AtomicBool::new(false);
/// `virtual_avail`: the first free kernel virtual address.
static VIRTUAL_AVAIL: AtomicUsize = AtomicUsize::new(0);
/// `pmap_virtual_space_called`: prevent further KVA stealing.
static PMAP_VIRTUAL_SPACE_CALLED: AtomicBool = AtomicBool::new(false);
/// The kernel image's first virtual address, its end and its physical-minus-virtual offset
/// (`BootstrapParams::image`, see the deviations).
static IMAGE: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
/// `mappings_allocated`: vp tables allocated for the kernel.
static MAPPINGS_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
/// `pted_allocated`: pages of descriptors allocated for the kernel.
static PTED_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
/// `pmap_growkernel`'s state.
static GROW: Mutexed<Grow> = Mutexed::new(
    IPL_VM,
    Grow {
        maxkvaddr: VM_MIN_KERNEL_ADDRESS,
        kpted: Paddr::new(0),
        npted: 0,
    },
);
/// `vmmap` (see the deviations): the next free address of the device window, `bus_space`'s.
pub static VMMAP: AtomicUsize = AtomicUsize::new(0);
/// The identity table [`switch_mmu_kernel_table`] runs from (its `TTBR0_EL1` value), built
/// once by the bootstrap; 0 before.
static SWITCH_IDENTITY: AtomicU64 = AtomicU64::new(0);
/// The physical address of the switch routine (`cpufunc.rs`'s `hal_switch_mmu_kernel`), the
/// one page the identity table maps; 0 before the bootstrap.
static SWITCH_ROUTINE_PA: AtomicU64 = AtomicU64::new(0);

/// arm64's pmap backend.
pub(crate) struct Backend;

/// `+= delta` on a statistics counter (the C's `++`/`--` on a `long`).
fn add(c: &PmapCounter, delta: i64) {
    c.set(c.get() + delta);
}

/// `ttlb_flush`: drops the TLB entries of one page of `pm` on every CPU; nothing while no
/// thread uses `pm`.
fn ttlb_flush(pm: &Pmap, va: usize) {
    if pm.md.active.load(Ordering::Relaxed) == 0 {
        return;
    }
    let resva = tlbi_page(va);
    if pm.is_kernel() {
        cpu_tlb_flush_all_asid(resva);
    } else {
        let asid = pm.md.asid();
        cpu_tlb_flush_asid(resva | tlbi_asid(asid));
        cpu_tlb_flush_asid(resva | tlbi_asid(asid | ASID_USER));
    }
}

/// `pmap_lock`: takes a user pmap's `pm_mtx`; the kernel pmap has no lock.
fn pmap_lock(pm: &Pmap) -> Locked<'_> {
    Locked::of(pm)
}

/// `pmap_vp_lookup`: the level-3 position of `va` in `pm`, `None` where a table is missing.
/// This is used for `pmap_kernel()` mappings, they are not to be removed from the vp table
/// because they were statically initialized at the initial pmap initialization.
fn pmap_vp_lookup(pm: &Pmap, va: usize) -> Option<L3> {
    vp::lookup(pm, va)
}

/// The pmap of a filled descriptor.
fn pted_pmap(pted: &PteDesc) -> &Pmap {
    let p = pted.pmap.load(Ordering::Acquire);
    if p.is_null() {
        hal_panic(format_args!("pmap: a pted without a pmap"));
    }
    // SAFETY: `pmap_fill_pte` stored the address of the pmap whose slot holds the descriptor
    // (from a `&Pmap`: the kernel's static, or a user pmap's value in its pool item, which
    // never moves), and the descriptor is reached under a witness that keeps it alive (its
    // slot under that pmap's lock, or its pv list under the page's `pv_mtx`). A pmap with a
    // descriptor is not freed: a user one's last handle's drop panics while `mapped` (its
    // occupied slots) is not 0, leaving its pool item in place, and the kernel's is a static.
    // The reference borrows the descriptor, so the pmap outlives it; it is shared, and every
    // field the backend reads through it is atomic or behind its lock.
    unsafe { &*p }
}

/// `pmap_reference(pted->pted_pmap)`: a counted handle to the pmap of a linked descriptor,
/// taken under the page's `pv_mtx` (`pmap_page_protect`, before it drops the lock); the
/// kernel pmap's is its static arm.
fn pmap_reference_pted(pted: &PteDesc) -> PmapRef {
    let p = pted.pmap.load(Ordering::Acquire);
    if ptr::eq(p, &KERNEL_PMAP) {
        return PmapRef::Kernel;
    }
    let Some(p) = NonNull::new(p) else {
        hal_panic(format_args!("pmap: a linked pted without a pmap"));
    };
    // SAFETY: `p` is the address of a user pmap's value inside its item of `hal::pmap`'s pool
    // (`pmap_fill_pte` stored `ptr::from_ref` of the `&Pmap` a `PmapRef` derefs to, and every
    // pmap but the kernel's is one `create`/`create_efi` put in that pool). The descriptor is
    // linked and the caller holds the page's `pv_mtx`, so the mapping exists: every removal
    // unlinks it under this lock first. A user pmap with a mapping has a handle (its last drop
    // panics while one is left, `hal::pmap`'s `Drop for Pmap`, and the panic leaves the item
    // in place), so the count is at least 1 for the whole call.
    PmapRef::User(unsafe { PoolArc::from_value(p) })
}

/// `PTED_MANAGED(pted)`.
fn pted_managed(pted: &PteDesc) -> bool {
    pted.va.load(Ordering::Relaxed) & PTED_VA_MANAGED_M as usize != 0
}

/// `PTED_WIRED(pted)`.
fn pted_wired(pted: &PteDesc) -> bool {
    pted.va.load(Ordering::Relaxed) & PTED_VA_WIRED_M as usize != 0
}

/// `PTED_VALID(pted)`.
fn pted_valid(pted: &PteDesc) -> bool {
    pted.pte.load(Ordering::Relaxed) != 0
}

/// The page-aligned virtual address of a descriptor.
fn pted_va(pted: &PteDesc) -> usize {
    pted.va.load(Ordering::Relaxed) & !PAGE_MASK
}

/// Two zeroed frames for a vp table: stolen while the page system is not up and the virtual
/// space was not handed to uvm, else from the provider (`pmap_kvp_alloc`, `pool_get(&pmap_vp_pool,
/// PR_NOWAIT | PR_ZERO)`). `None` when a frame is not there now.
fn pmap_kvp_alloc(pages: &dyn PhysPages) -> Option<Table> {
    let steal = !pages.page_init_done() && !PMAP_VIRTUAL_SPACE_CALLED.load(Ordering::Relaxed);
    let frame = || -> Option<Paddr> {
        if steal {
            let Some(pa) = pages.steal(1) else {
                hal_panic(format_args!("pmap_kvp_alloc: out of memory"));
            };
            zero_frame(pa);
            Some(pa)
        } else {
            pages.alloc_ptp()
        }
    };
    let hw = frame()?;
    let Some(sw) = frame() else {
        if !steal {
            pages.free_ptp(hw);
        }
        return None;
    };
    MAPPINGS_ALLOCATED.fetch_add(1, Ordering::Relaxed);
    Some(Table { hw, sw })
}

/// A zeroed frame the bootstrap steals for a table of its own (the image's, the switch's
/// identity table, the empty lower half), never given back.
fn pmap_steal_table(pages: &dyn PhysPages) -> Paddr {
    let Some(pa) = pages.steal(1) else {
        hal_panic(format_args!("pmap_bootstrap: out of memory"));
    };
    zero_frame(pa);
    pa
}

/// `pmap_map_stolen` for the kernel image (see the redesign): a level-2 table, and a level-3
/// table per 2 MiB the image touches, mapping each page of the image with its section's
/// protection (`image_page_prot`) through a write-back kernel leaf ([`kernel_leaf`], without
/// `ATTR_GP`, see the deviations); the level-2 table's frame. `img` passed
/// `image_layout_ok`: the image lies in one gigabyte, below 48 physical bits.
fn pmap_map_stolen(pages: &dyn PhysPages, img: &KernelImage) -> Paddr {
    let l2 = pmap_steal_table(pages);
    let start = img.start.as_usize();
    let end = round_page(img.end.as_usize());
    let first = start & !(L2_SIZE - 1);
    for i in 0..image_ptps(img) {
        let block = first + i * L2_SIZE;
        let l3 = pmap_steal_table(pages);
        let ptes = words(l3);
        let pages_of_block = block.max(start)..block.saturating_add(L2_SIZE).min(end);
        for va in pages_of_block.step_by(PAGE_SIZE) {
            if let Some(prot) = image_page_prot(img, va) {
                let pa = va.wrapping_add(img.phys_off) as u64;
                ptes[vp_idx(3, va)].store(kernel_leaf(pa, prot), Ordering::Relaxed);
            }
        }
        let table = l3.as_usize() as u64 | Lx_TYPE_PT;
        words(l2)[vp_idx(2, block)].store(table, Ordering::Relaxed);
    }
    l2
}

/// The identity table [`switch_mmu_kernel_table`] runs from: four stolen tables, levels 0 to
/// 3 of a 48-bit lower half, whose one leaf maps the page at `pa` (the switch routine's) at
/// its own address, read-only and executable at EL1 ([`kernel_leaf`] of `PROT_READ |
/// PROT_EXEC`, not global). Its level-0 frame, the `TTBR0_EL1` value.
fn pmap_identity_table(pages: &dyn PhysPages, pa: usize) -> Paddr {
    let [l0, l1, l2, l3] = [(); 4].map(|()| pmap_steal_table(pages));
    words(l3)[vp_idx(3, pa)].store(
        kernel_leaf(pa as u64, PROT_READ | PROT_EXEC),
        Ordering::Relaxed,
    );
    for (table, level, child) in [(l2, 2, l3), (l1, 1, l2), (l0, 0, l1)] {
        words(table)[vp_idx(level, pa)]
            .store(child.as_usize() as u64 | Lx_TYPE_PT, Ordering::Relaxed);
    }
    l0
}

/// `switch_mmu_kernel_table(pt1pa)`: loads the kernel pmap's root into `TTBR1_EL1` with the
/// kernel's ASID 0 and `CnP` clear (`pmap_kernel()->pm_asid`, as the bootstrap sets it) and
/// drops every TLB entry (the C's `switch_mmu_kernel`), running the switch routine at its
/// physical address through the bootstrap's identity table (see the redesign), then puts
/// `TTBR0_EL1` and its `TCR_EL1` fields back as they were. For a CPU that is not on the
/// kernel's tables yet (the boot CPU in the bootstrap, an application processor Limine started
/// on its own), or one on exactly that value already (a boot(8) application processor, which
/// `locore.S`'s `start_mmu` put there: a TLB flush); panics on a CPU on the kernel's tables
/// with another ASID or `CnP` (a user pmap's ASID loaded: its translations would be cached
/// under the kernel's ASID, out of `pmap_free_asid`'s reach), or before the bootstrap built
/// the identity table.
pub fn switch_mmu_kernel_table() {
    let id = SWITCH_IDENTITY.load(Ordering::Acquire);
    let routine = SWITCH_ROUTINE_PA.load(Ordering::Acquire);
    let root = KERNEL_PMAP.md.root_hw.load(Ordering::Acquire);
    if id == 0 || routine == 0 || root == 0 {
        hal_panic(format_args!("switch_mmu_kernel: before pmap_bootstrap"));
    }
    // ASID 0 (bits 63:48) and CnP (bit 0) clear: the kernel's root alone.
    let ttbr1 = root;
    let live = cpu_ttbr1();
    if live & TTBR_BADDR_MASK == root && live != ttbr1 {
        hal_panic(format_args!(
            "switch_mmu_kernel: the kernel's table is loaded as {live:#x}"
        ));
    }
    // SAFETY: the block changes the CPU's translation registers only, and leaves every
    // translation the kernel uses valid:
    // - `TTBR0_EL1`: the identity table (`pmap_identity_table`, stolen zeroed frames written
    //   through their views, the `dsb ishst` completing the writes before any walk) maps one
    //   page, the switch routine's, at its physical address, read-only and executable at EL1,
    //   under the lower-half walk `TCR_IDENTITY_TTBR0` sets (48 bits, 4 KiB granule, the four
    //   levels it was built for, write-back walks). Nothing else is reached through the lower
    //   half inside the block, which has no memory operand, and with interrupts masked
    //   (`daifset`, restored at the end) no handler runs inside it. The old `TTBR0_EL1` and
    //   `TCR_EL1` are put back before the block ends, and `tlbi vmalle1` drops what this CPU
    //   cached through the identity table, so the lower half translates afterwards as before
    //   (the boot glue's device map, the kernel's empty table, or Limine's).
    // - `TTBR1_EL1`: the routine (`cpufunc.rs`) runs from the identity page, so between its
    //   `msr ttbr1_el1` and the completion of its `tlbi vmalle1is; dsb ish; isb` nothing is
    //   fetched or accessed through the upper half (break-before-make for the kernel's live
    //   entries, which change size and attributes here). `ttbr1` is the kernel pmap's root with
    //   the kernel's ASID 0 and `CnP` clear, built above, never a caller's value, and the CPU
    //   was either off the kernel's tables or on that very value (checked above), so no user
    //   pmap's ASID is replaced while its table sits in `TTBR0_EL1` (whose translations would
    //   otherwise be cached under ASID 0, beyond `pmap_free_asid`'s `tlbi aside1is` of the
    //   user's ASID), and no TLB is shared (`CnP`). Its tables map, at the physical addresses
    //   every table a CPU of this kernel runs on maps them (the boot loader's on both paths,
    //   Limine's on an application processor), the image, the direct map (the copied level-0
    //   slots, Limine's stacks there included) and the kernel map (the copied level-1 slots),
    //   with `Lx_PT_PXN`/`Lx_PT_XN` on the direct map's slots removing only execution, which
    //   nothing does there (the code after the block is the image's text). So the return address, the
    //   code after the block, the stack and every Rust reference stay mapped; only the image's
    //   permissions change, tighter: no Rust code writes the text or the read-only data (the
    //   boot seed in `.openbsd.randomdata` is wiped through another mapping), and nothing
    //   executes from the data.
    // - Registers: `x0` carries `ttbr1` and the routine leaves it; `blr` clobbers `x30`; the
    //   routine touches no other register, no stack and no condition flag. Memory is a
    //   clobber: the translations change.
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "mrs {daif}, daif",
            "msr daifset, #0xf",
            "mrs {ttbr0}, ttbr0_el1",
            "mrs {tcr}, tcr_el1",
            "bic {t}, {tcr}, {fields}",
            "orr {t}, {t}, {idtcr}",
            "msr tcr_el1, {t}",
            "msr ttbr0_el1, {id}",
            "isb",
            "tlbi vmalle1",
            "dsb nsh",
            "isb",
            "blr {routine}",
            "msr ttbr0_el1, {ttbr0}",
            "msr tcr_el1, {tcr}",
            "isb",
            "tlbi vmalle1",
            "dsb nsh",
            "isb",
            "msr daif, {daif}",
            daif = out(reg) _,
            ttbr0 = out(reg) _,
            tcr = out(reg) _,
            t = out(reg) _,
            fields = in(reg) TCR_TTBR0_FIELDS,
            idtcr = in(reg) TCR_IDENTITY_TTBR0,
            id = in(reg) id,
            routine = in(reg) routine,
            in("x0") ttbr1,
            out("x30") _,
            options(nostack, preserves_flags),
        )
    };
}

/// `pmap_kpted_alloc`: a zeroed descriptor for the kernel, a page of them at a time.
fn pmap_kpted_alloc(g: &mut Grow, pages: &dyn PhysPages) -> Option<NonNull<PteDesc>> {
    if g.npted == 0 {
        let pa = if !pages.page_init_done() && !PMAP_VIRTUAL_SPACE_CALLED.load(Ordering::Relaxed) {
            let Some(pa) = pages.steal(1) else {
                hal_panic(format_args!("pmap_kpted_alloc: out of memory"));
            };
            zero_frame(pa);
            pa
        } else {
            pages.alloc_ptp()?
        };
        g.kpted = pa;
        g.npted = PTEDS_PER_PAGE;
        PTED_ALLOCATED.fetch_add(1, Ordering::Relaxed);
    }
    let i = PTEDS_PER_PAGE - g.npted;
    g.npted -= 1;
    Some(NonNull::from(&pted_page(g.kpted)[i]))
}

/// `pmap_vp_enter`: the level-3 position of `va` in `pm`, the missing tables allocated
/// (`PR_NOWAIT`); without `PMAP_CANFAIL` a missing frame panics.
fn pmap_vp_enter(pm: &Pmap, locked: &Locked<'_>, va: usize, flags: i32) -> Result<L3, Errno> {
    let pages = pm.pages();
    match vp::enter(pm, locked, va, || pmap_kvp_alloc(pages)) {
        Ok(l3) => Ok(l3),
        Err(level) => {
            if flags & PMAP_CANFAIL == 0 {
                hal_panic(format_args!("pmap_vp_enter: unable to allocate L{level}"));
            }
            Err(Errno::ENOMEM)
        }
    }
}

/// `pmap_enter_pv`: puts the mapping on its page's pv list. Too soon (before `pmap_init`),
/// the page is not managed yet.
fn pmap_enter_pv(pted: &PteDesc, md: &PageMd) {
    if !PMAP_INITIALIZED.load(Ordering::Acquire) {
        return;
    }
    let head = md.pv.lock();
    pv_insert(&head, pted);
    pted.va
        .fetch_or(PTED_VA_MANAGED_M as usize, Ordering::Relaxed);
}

/// The page a managed descriptor maps (`PHYS_TO_VM_PAGE(pted->pted_pte & PTE_RPGN)`).
fn pted_md(pted: &PteDesc, pages: &dyn PhysPages) -> &'static PageMd {
    let pa = Paddr::new((pted.pte.load(Ordering::Relaxed) & PTE_RPGN) as usize);
    match pages.md_of(pa) {
        Some(md) => md,
        None => hal_panic(format_args!("pmap_remove_pv: {pa:?} is not a managed page")),
    }
}

/// `pmap_pte_remove` + `pmap_remove_pv` + the descriptor cleared (`pted_pte = 0`,
/// `pted_va = 0`): the PTE zeroed and its TLB entries dropped; for a managed descriptor all of
/// it inside its page's `pv_mtx`, which the caller does not hold (the removal rule, `vp.rs`:
/// a pv walker revalidates from `pted.pte` and would otherwise write the zeroed PTE back).
fn pmap_pte_remove(pm: &Pmap, pted: &PteDesc, l3: L3) {
    let va = pted_va(pted);
    let pv = pted_managed(pted).then(|| pted_md(pted, pm.pages()).pv.lock());
    l3.pte().store(0, Ordering::Release);
    ttlb_flush(pm, va);
    if let Some(head) = &pv {
        pv_unlink(head, pted);
    }
    pted.pte.store(0, Ordering::Relaxed);
    pted.va.store(0, Ordering::Relaxed);
    drop(pv);
}

/// `pmap_icache_sync_page`: the page's contents made visible to instruction fetch, through
/// the direct map.
pub(super) fn pmap_icache_sync_page(pa: usize) {
    cpu_icache_sync_range(
        direct_map(Paddr::new(pa & PMAP_PA_MASK)).as_usize(),
        PAGE_SIZE,
    );
}

/// `pmap_enter`: enter a mapping into a pmap.
fn pmap_enter(pm: &Pmap, va: usize, pa: usize, prot: VmProt, flags: i32) -> Result<(), Errno> {
    let cache = cache_of_pa(pa);
    let pg = pm.pages().md_of(Paddr::new(pa & PMAP_PA_MASK));

    let mut locked = pmap_lock(pm);
    let mut l3 = pmap_vp_lookup(pm, va);
    let old = l3.and_then(|l| {
        let t = l.pted(&locked)?;
        pted_valid(t).then(|| {
            let v = t.va.load(Ordering::Relaxed);
            t.pte.load(Ordering::Relaxed) & PTE_RPGN == pa as u64 & PTE_RPGN
                && v & PROT_MASK as usize == (prot & PROT_MASK) as usize
                && v & PMAP_CACHE_BITS as usize == cache as usize
        })
    });
    match (old, l3) {
        (Some(true), _) => return Ok(()),
        (Some(false), Some(l)) => {
            pmap_remove_pted(pm, &mut locked, l);
            // we lost our pted if it was user
        }
        _ => {}
    }

    add(&pm.stats.resident_count, 1);

    // Do not have pted for this, get one and put it in VP
    if l3.is_none_or(|l| !l.occupied()) {
        let Some(pted) = PMAP_PTED_POOL.get_linked(PR_NOWAIT, PteDesc::new) else {
            if flags & PMAP_CANFAIL == 0 {
                hal_panic(format_args!("pmap_enter: failed to allocate pted"));
            }
            return Err(Errno::ENOMEM);
        };
        let l = match pmap_vp_enter(pm, &locked, va, flags) {
            Ok(l) => l,
            Err(e) => {
                drop(pted);
                return Err(e);
            }
        };
        // The slot owns it from now on (`L3::take` frees it); a kernel slot keeps it for
        // good, as the C's kernel pmap never frees a descriptor.
        l.put(&locked, pted);
        if !pm.is_kernel() {
            pm.md.mapped.fetch_add(1, Ordering::Relaxed);
        }
        l3 = Some(l);
    }
    let Some(l3) = l3 else {
        hal_panic(format_args!("pmap_enter: no level-3 table"));
    };
    let Some(pted) = l3.pted(&locked) else {
        hal_panic(format_args!("pmap_enter: no pted"));
    };

    // If it should be enabled _right now_, we can skip doing ref/mod emulation. Any access
    // includes reference, modified only by write.
    if let Some(md) = pg
        && (flags & PROT_MASK != 0 || md.attrs.load(Ordering::Relaxed) & PG_PMAP_REF != 0)
    {
        md.attrs.fetch_or(PG_PMAP_REF, Ordering::Relaxed);
        if prot & PROT_WRITE != 0 && flags & PROT_WRITE != 0 {
            md.attrs.fetch_or(PG_PMAP_MOD, Ordering::Relaxed);
            md.attrs.fetch_and(!PG_PMAP_EXE, Ordering::Relaxed);
        }
    }

    pmap_fill_pte(pm, va, pa, pted, prot, flags, cache);

    if let Some(md) = pg {
        pmap_enter_pv(pted, md); // only managed mem
    }

    if let Some(md) = pg
        && flags & PROT_EXEC != 0
    {
        if md.attrs.load(Ordering::Relaxed) & PG_PMAP_EXE == 0 {
            pmap_icache_sync_page(pa);
        }
        md.attrs.fetch_or(PG_PMAP_EXE, Ordering::Relaxed);
    }

    // Insert into table, if this mapping said it needed to be mapped now.
    if flags & (PROT_MASK | PMAP_WIRED) != 0 {
        pmap_pte_update(pm, pted, l3.pte());
        ttlb_flush(pm, va & !PAGE_MASK);
    }
    Ok(())
}

/// `pmap_remove`: remove the given range of mapping entries.
fn pmap_remove(pm: &Pmap, sva: usize, eva: usize) {
    let mut locked = pmap_lock(pm);
    for va in (sva..eva).step_by(PAGE_SIZE) {
        let Some(l3) = pmap_vp_lookup(pm, va) else {
            continue;
        };
        let Some(pted) = l3.pted(&locked) else {
            continue;
        };
        if pted_wired(pted) {
            add(&pm.stats.wired_count, -1);
            pted.va
                .fetch_and(!(PTED_VA_WIRED_M as usize), Ordering::Relaxed);
        }
        if pted_valid(pted) {
            pmap_remove_pted(pm, &mut locked, l3);
        }
    }
}

/// `pmap_remove_pted`: remove a single mapping, notice that this code is O(1). A user pmap's
/// descriptor is freed; the kernel's stays in its slot. Under `pm_mtx` (`locked`); a managed
/// one is torn down inside its page's `pv_mtx` ([`pmap_pte_remove`]).
fn pmap_remove_pted(pm: &Pmap, locked: &mut Locked<'_>, l3: L3) {
    let Some(pted) = l3.pted(locked) else {
        return;
    };
    add(&pm.stats.resident_count, -1);

    if pted_wired(pted) {
        add(&pm.stats.wired_count, -1);
        pted.va
            .fetch_and(!(PTED_VA_WIRED_M as usize), Ordering::Relaxed);
    }

    pmap_pte_remove(pm, pted, l3);

    if !pm.is_kernel() {
        drop(l3.take(locked, &PMAP_PTED_POOL));
        pm.md.mapped.fetch_sub(1, Ordering::Relaxed);
    }
}

/// `_pmap_kenter_pa`: enter a kernel mapping for the given page. Kernel mappings have a larger
/// set of prerequisites than normal mappings: no memory should be allocated to create a kernel
/// mapping; a vp mapping should already exist, even if invalid; all vp tree mappings should
/// already exist.
fn pmap_kenter_pa_cache(va: usize, pa: usize, prot: VmProt, flags: i32, cache: i32) {
    let pm = &KERNEL_PMAP;
    let locked = Locked::of(pm);
    let l3 = pmap_vp_lookup(pm, va);
    let Some((l3, pted)) = l3.and_then(|l| Some((l, l.pted(&locked)?))) else {
        hal_panic(format_args!(
            "pted not preallocated in pmap_kernel() va {va:#x} pa {pa:#x}"
        ));
    };

    if pted_valid(pted) {
        pmap_kremove_pg(va); // pted is reused
    }

    add(&pm.stats.resident_count, 1);

    let flags = flags | PMAP_WIRED; // kernel mappings are always wired.
    // Calculate PTE
    pmap_fill_pte(pm, va, pa, pted, prot, flags, cache);

    // Insert into table
    pmap_pte_update(pm, pted, l3.pte());
    ttlb_flush(pm, va & !PAGE_MASK);

    let pg = pm.pages().md_of(Paddr::new(
        (pted.pte.load(Ordering::Relaxed) & PTE_RPGN) as usize,
    ));
    if pg.is_some() && (cache == PMAP_CACHE_CI || cache == PMAP_CACHE_DEV_NGNRNE) {
        cpu_idcache_wbinv_range(va & !PAGE_MASK, PAGE_SIZE);
    }
}

/// `pmap_kremove_pg`: remove kernel (`pmap_kernel()`) mapping, one page.
fn pmap_kremove_pg(va: usize) {
    let pm = &KERNEL_PMAP;
    let locked = Locked::of(pm);
    let Some(l3) = pmap_vp_lookup(pm, va) else {
        return;
    };
    let Some(pted) = l3.pted(&locked) else {
        return;
    };
    if !pted_valid(pted) {
        return; // not mapped
    }

    let _s = splraise(IPL_VM);

    add(&pm.stats.resident_count, -1);
    if pted_wired(pted) {
        add(&pm.stats.wired_count, -1);
    }

    // pmap_pte_remove(pted, 0), pmap_remove_pv, invalidate pted (the removal rule).
    pmap_pte_remove(pm, pted, l3);
}

/// `pmap_fill_pte`: computes the descriptor of a mapping.
fn pmap_fill_pte(
    pm: &Pmap,
    va: usize,
    pa: usize,
    pted: &PteDesc,
    prot: VmProt,
    flags: i32,
    cache: i32,
) {
    if !cache_valid(cache) {
        hal_panic(format_args!("pmap_fill_pte: invalid cache mode"));
    }
    let mut v = va | cache as usize | (prot & PROT_MASK) as usize;
    if flags & PMAP_WIRED != 0 {
        v |= PTED_VA_WIRED_M as usize;
        add(&pm.stats.wired_count, 1);
    }
    pted.pmap
        .store(ptr::from_ref(pm).cast_mut(), Ordering::Release);
    pted.va.store(v, Ordering::Relaxed);
    pted.pte.store(
        (pa as u64 & PTE_RPGN) | (flags & PROT_MASK) as u64,
        Ordering::Relaxed,
    );
}

/// `pmap_pte_update`: the hardware PTE of a descriptor, stored at `pl3`.
fn pmap_pte_update(pm: &Pmap, pted: &PteDesc, pl3: &AtomicU64) {
    // see mair in locore.S (and include/pte.rs for the indices)
    let idx = match pted.va.load(Ordering::Relaxed) as i32 & PMAP_CACHE_BITS {
        PMAP_CACHE_WB => PTE_ATTR_WB, // inner and outer writeback
        PMAP_CACHE_WT => PTE_ATTR_WT, // inner and outer writethrough
        PMAP_CACHE_CI => PTE_ATTR_CI,
        PMAP_CACHE_DEV_NGNRNE => PTE_ATTR_DEV_NGNRNE,
        PMAP_CACHE_DEV_NGNRE => PTE_ATTR_DEV_NGNRE,
        _ => hal_panic(format_args!("pmap_pte_update: invalid cache mode")),
    };
    let attr = ATTR_nG | attr_idx(idx) | attr_sh(SH_INNER);

    let pte = pted.pte.load(Ordering::Relaxed);
    let prot = (pte & PROT_MASK as u64) as usize;
    let mut access_bits = if pm.md.privileged.load(Ordering::Relaxed) {
        AP_BITS_KERN[prot]
    } else {
        AP_BITS_USER[prot]
    };

    // !SMALL_KERNEL
    access_bits |= pm.md.guarded.load(Ordering::Relaxed);

    let pte = (pte & PTE_RPGN) | attr | access_bits | L3_P;
    pl3.store(if access_bits != 0 { pte } else { 0 }, Ordering::Release);
}

/// `pmap_pte_insert`: writes a descriptor's PTE into its level-3 table.
fn pmap_pte_insert(pm: &Pmap, pted: &PteDesc) {
    let va = pted.va.load(Ordering::Relaxed);
    let Some(l3) = pmap_vp_lookup(pm, va) else {
        hal_panic(format_args!(
            "pmap_pte_insert: have a pted, but missing a vp for {va:#x} va pmap {:p}",
            pm.as_ptr()
        ));
    };
    pmap_pte_update(pm, pted, l3.pte());
}

/// `pmap_page_ro`: lowers the mapping `pted` of `pm` (whose PTE is `pl3`) to read-only, and to
/// `prot`.
fn pmap_page_ro(pm: &Pmap, pted: &PteDesc, pl3: &AtomicU64, prot: VmProt) {
    if !pted_valid(pted) {
        return;
    }
    let mut clear = PROT_WRITE;
    if prot & PROT_READ == 0 {
        clear |= PROT_READ;
    }
    if prot & PROT_EXEC == 0 {
        clear |= PROT_EXEC;
    }
    pted.va.fetch_and(!(clear as usize), Ordering::Relaxed);
    pted.pte.fetch_and(!(clear as u64), Ordering::Relaxed);
    pmap_pte_update(pm, pted, pl3);
    ttlb_flush(pm, pted_va(pted));
}

/// `pmap_page_protect`: lower the protection on the specified physical page.
/// The first descriptor of the page's pv list that `pmap_page_protect` removes: the first
/// that is not a live byte view's own mapping (a pager window's entry in the kernel pmap,
/// `PageMd::spares`; a frame has at most one).
fn first_unspared<'w>(md: &PageMd, head: &'w MutexedGuard<'_, PvHead>) -> Option<&'w PteDesc> {
    let mut cur = head.0.target(head);
    while let Some(pted) = cur {
        let kernel = ptr::eq(pted.pmap.load(Ordering::Acquire), &KERNEL_PMAP);
        if !md.spares(kernel, pted_va(pted)) {
            return Some(pted);
        }
        cur = pted.pv_next.target(head);
    }
    None
}

fn pmap_page_protect(md: &PageMd, prot: VmProt) {
    if prot != PROT_NONE {
        let head = md.pv.lock();
        let mut cur: &Link<PvKind> = &head.0;
        while let Some(pted) = cur.target(&head) {
            let pm = pted_pmap(pted);
            // Every VA needs a pted, even unmanaged ones.
            if let Some(l3) = pmap_vp_lookup(pm, pted_va(pted)) {
                pmap_page_ro(pm, pted, l3.pte(), prot);
            }
            cur = &pted.pv_next;
        }
        return;
    }

    loop {
        let pmref = {
            let head = md.pv.lock();
            let Some(pted) = first_unspared(md, &head) else {
                // page is being reclaimed, sync icache next use (a live window's own mapping,
                // spared, is all that may be left: the page is not reclaimed then)
                if head.0.target(&head).is_none() {
                    md.attrs.fetch_and(!PG_PMAP_EXE, Ordering::Relaxed);
                }
                return;
            };
            pmap_reference_pted(pted)
        };
        let pm: &Pmap = &pmref;
        let mut locked = pmap_lock(pm);

        // We dropped the pvlist lock before grabbing the pmap lock to avoid lock ordering
        // problems. This means we have to check the pvlist again since somebody else might
        // have modified it. All we care about is that the pvlist entry matches the pmap we
        // just locked. If it doesn't, unlock the pmap and try again.
        let va = {
            let head = md.pv.lock();
            first_unspared(md, &head)
                .filter(|t| ptr::eq(t.pmap.load(Ordering::Acquire), pm))
                .map(pted_va)
        };
        if let Some(va) = va {
            // The first mapping not spared is `pm`'s at `va`, which only `pm`'s lock (held)
            // removes; the head guard is dropped (the removal takes `pv_mtx` itself).
            let Some(l3) = pmap_vp_lookup(pm, va) else {
                hal_panic(format_args!(
                    "pmap_page_protect: a linked pted without a vp for {va:#x}"
                ));
            };
            pmap_remove_pted(pm, &mut locked, l3);
        }
        drop(locked);
        // pmap_destroy(pm): outside both locks, as the drop may be the last.
        drop(pmref);
    }
}

/// `pmap_protect`: lowers the protection of the mappings in `[sva, eva)` of `pm`;
/// `PROT_NONE` removes them.
fn pmap_protect(pm: &Pmap, sva: usize, eva: usize, prot: VmProt) {
    if prot & (PROT_READ | PROT_EXEC) == 0 {
        pmap_remove(pm, sva, eva);
        return;
    }
    let locked = pmap_lock(pm);
    for va in (sva..eva).step_by(PAGE_SIZE) {
        // Every VA needs a pted, even unmanaged ones.
        let Some(l3) = pmap_vp_lookup(pm, va) else {
            continue;
        };
        if let Some(pted) = l3.pted(&locked) {
            pmap_page_ro(pm, pted, l3.pte(), prot);
        }
    }
}

/// `pmap_fault_fixup`'s body under `pm_mtx`: software referenced/modified emulation; whether
/// the fault was one (then it is handled) rather than a legit fault.
fn pmap_fault_fixup(pm: &Pmap, va: usize, ftype: VmProt) -> bool {
    let locked = pmap_lock(pm);
    // Every VA needs a pted, even unmanaged ones.
    let Some(l3) = pmap_vp_lookup(pm, va) else {
        return false;
    };
    let Some(pted) = l3.pted(&locked) else {
        return false;
    };
    if !pted_valid(pted) {
        return false;
    }

    // There has to be a PA for the VA, get it.
    let pa = (pted.pte.load(Ordering::Relaxed) & PTE_RPGN) as usize;

    // If it's unmanaged, it must not fault.
    let Some(md) = pm.pages().md_of(Paddr::new(pa)) else {
        return false;
    };

    let pte_prot = pted.pte.load(Ordering::Relaxed) as usize;
    let va_prot = pted.va.load(Ordering::Relaxed);
    let (rd, wr, ex) = (PROT_READ as usize, PROT_WRITE as usize, PROT_EXEC as usize);

    // Check the fault types to find out if we were doing any mod/ref emulation and fixup the
    // PTE if we were.
    if ftype & PROT_WRITE != 0 && pte_prot & wr == 0 && va_prot & wr != 0 {
        // Page modified emulation. A write always includes a reference. This means that we
        // can enable read and exec as well, akin to the page reference emulation.
        md.attrs
            .fetch_or(PG_PMAP_MOD | PG_PMAP_REF, Ordering::Relaxed);
        md.attrs.fetch_and(!PG_PMAP_EXE, Ordering::Relaxed);

        // Thus, enable read, write and exec.
        pted.pte
            .fetch_or((va_prot & (rd | wr | ex)) as u64, Ordering::Relaxed);
    } else if ftype & PROT_EXEC != 0 && pte_prot & ex == 0 && va_prot & ex != 0 {
        // Exec always includes a reference. Since we now know the page has been accessed, we
        // can enable read as well if UVM allows it.
        md.attrs.fetch_or(PG_PMAP_REF, Ordering::Relaxed);

        // Thus, enable read and exec.
        pted.pte
            .fetch_or((va_prot & (rd | ex)) as u64, Ordering::Relaxed);
    } else if ftype & PROT_READ != 0 && pte_prot & rd == 0 && va_prot & rd != 0 {
        // Page referenced emulation. Since we now know the page has been accessed, we can
        // enable exec as well if UVM allows it.
        md.attrs.fetch_or(PG_PMAP_REF, Ordering::Relaxed);

        // Thus, enable read and exec.
        pted.pte
            .fetch_or((va_prot & (rd | ex)) as u64, Ordering::Relaxed);
    } else {
        // didn't catch it, so probably broken
        return false;
    }

    // If this is a page that can be executed, make sure to invalidate the instruction cache if
    // the page has been modified or not used yet.
    if va_prot & ex != 0 {
        if md.attrs.load(Ordering::Relaxed) & PG_PMAP_EXE == 0 {
            pmap_icache_sync_page(pa);
        }
        md.attrs.fetch_or(PG_PMAP_EXE, Ordering::Relaxed);
    }

    // We actually made a change, so flush it and sync.
    pmap_pte_update(pm, pted, l3.pte());
    ttlb_flush(pm, va & !PAGE_MASK);
    true
}

/// `pmap_clear_modify` (`clear` = `PROT_WRITE`) and `pmap_clear_reference` (`PROT_MASK`: when
/// this turns off read permissions it also disables write permissions so that mod is
/// correctly tracked after clear_ref): the bits taken from every mapping of the page, so the
/// next access is seen by `pmap_fault_fixup`. Always false, as the C.
fn pmap_clear_attrs(md: &PageMd, attr: u32, clear: i32) -> bool {
    md.attrs.fetch_and(!attr, Ordering::Relaxed);

    let head = md.pv.lock();
    let mut cur: &Link<PvKind> = &head.0;
    while let Some(pted) = cur.target(&head) {
        let pm = pted_pmap(pted);
        pted.pte.fetch_and(!(clear as u64), Ordering::Relaxed);
        pmap_pte_insert(pm, pted);
        ttlb_flush(pm, pted_va(pted));
        cur = &pted.pv_next;
    }
    false
}

/// `pmap_kenter_cache`'s and `pmap_kenter_pa`'s cache-mode check, then `_pmap_kenter_pa`.
/// Panics on `PROT_EXEC` (see the deviations).
fn pmap_kenter(va: Vaddr, pa: Paddr, prot: VmProt, cache: i32) {
    refuse_kernel_exec("pmap_kenter_pa", va, prot);
    pmap_kenter_pa_cache(va.as_usize(), pa.as_usize(), prot, prot, cache);
}

/// The kernel pmap's `PROT_EXEC` refusal (see the deviations): no kernel page outside the image
/// and the module window (`window.rs`, entered by `module_enter` alone) executes.
fn refuse_kernel_exec(what: &str, va: Vaddr, prot: VmProt) {
    if prot & PROT_EXEC != 0 {
        hal_panic(format_args!(
            "{what}: an executable kernel mapping at {:#x}",
            va.as_usize()
        ));
    }
}

/// The EFI pmap's and the user's half of `efi_enter`/`efi_leave`: the empty table, `T0SZ`
/// for `bits`, then `pm`'s ASID and table (`cpu_setttb`).
fn switch_space(pm: &Pmap, bits: u32) {
    let pt0pa = pm.md.pt0pa();
    let empty = KERNEL_PMAP.md.pt0pa();
    if pt0pa == 0 || empty == 0 {
        hal_panic(format_args!("pmap: a TTBR0_EL1 switch without tables"));
    }
    let t0sz = 64 - u64::from(bits);
    // SAFETY: the lower half goes to the kernel's empty table (`pmap_bootstrap`'s zeroed page)
    // while its walk is resized, then to `pm`'s root (the machine part's invariant: a table
    // `create`/`create_efi` built, alive while the caller's handle is, or the kernel's empty
    // table), with `pm`'s ASID in `TTBR1_EL1`, whose table base `bfi` leaves alone. Nothing of
    // the kernel lives in the lower half (no Rust reference points there; the EFI runtime is
    // called through it by `bsd`'s `efi_call` with interrupts disabled, its own trust), and
    // the kernel's translations through `TTBR1_EL1` stay; the `isb`s order the switches before
    // the next fetch. Memory is a clobber.
    unsafe {
        core::arch::asm!(
            "msr ttbr0_el1, {empty}",
            "isb",
            "mrs {tcr}, tcr_el1",
            "and {tcr}, {tcr}, {keep}",
            "orr {tcr}, {tcr}, {t0sz}",
            "msr tcr_el1, {tcr}",
            "mrs {tmp}, ttbr1_el1",
            "bfi {tmp}, {asid}, #48, #16",
            "msr ttbr1_el1, {tmp}",
            "isb",
            "msr ttbr0_el1, {pt0pa}",
            "isb",
            empty = in(reg) empty,
            keep = in(reg) !TCR_T0SZ_MASK,
            t0sz = in(reg) t0sz,
            asid = in(reg) pm.md.asid(),
            pt0pa = in(reg) pt0pa,
            tcr = out(reg) _,
            tmp = out(reg) _,
            options(nostack, preserves_flags),
        )
    };
}

/// `pmap_init`'s second half (see the deviations): now that we are in virtual address space we
/// don't need the bootstrap mapping in `TTBR0_EL1` and can set the TCR to a more useful value:
/// the lower half to the kernel's empty table, `T0SZ` for `USER_SPACE_BITS`, the ASID from
/// `TTBR1_EL1` (`TCR_A1`), every TLB flushed. Called by the boot glue once the console was
/// remapped into the kernel half, on the boot CPU.
pub fn init_lower_half() {
    let pt0pa = KERNEL_PMAP.md.pt0pa();
    if pt0pa == 0 {
        hal_panic(format_args!("pmap_init: before pmap_bootstrap"));
    }
    let set = (64 - u64::from(USER_SPACE_BITS)) | TCR_A1;
    // SAFETY: the kernel runs entirely in the upper half (`TTBR1_EL1`): the lower half goes to
    // the empty table (`pmap_bootstrap`'s zeroed page) and its walk is resized under it; the
    // boot glue's device map there is no longer used (the console was remapped, the caller's
    // order), and no Rust reference points into the lower half. `TCR_A1` takes the ASID from
    // `TTBR1_EL1`, whose ASID field is the kernel's 0 here, so the kernel's translations do not
    // change; clearing `TCR_HPD` (a no-op under both boot loaders today, see the deviations)
    // makes the table descriptors' `PXNTable`/`UXNTable` count, which only take execution away
    // from the direct map and `kernel_map`, where nothing executes, and the kernel's tables
    // carry no `APTable`. The TLB is flushed right after. Memory is a clobber.
    unsafe {
        core::arch::asm!(
            "msr ttbr0_el1, {pt0}",
            "isb",
            "mrs {tcr}, tcr_el1",
            "and {tcr}, {tcr}, {keep}",
            "orr {tcr}, {tcr}, {set}",
            "msr tcr_el1, {tcr}",
            pt0 = in(reg) pt0pa,
            keep = in(reg) !(TCR_T0SZ_MASK | TCR_HPD),
            set = in(reg) set,
            tcr = out(reg) _,
            options(nostack, preserves_flags),
        )
    };
    cpu_tlb_flush();
    asid::set_curpm(&KERNEL_PMAP);
}

/// The kernel's empty lower-half table (`pmap_kernel()->pm_pt0pa`), for the CPU code that
/// loads it itself (`cpu_init`, the secondary processors' start).
pub fn kernel_pt0pa() -> u64 {
    KERNEL_PMAP.md.pt0pa()
}

/// `pmap_steal_memory`'s out-parameters without `pmap_virtual_space`'s side effect: the free
/// kernel virtual range, for the boot glue's `uvm_pageboot_alloc`, which steals before uvm
/// takes the range (`pmap_virtual_space` forbids further stealing of vp tables).
pub fn stolen_bounds() -> (Vaddr, Vaddr) {
    (
        Vaddr::new(VIRTUAL_AVAIL.load(Ordering::Relaxed)),
        Vaddr::new(VM_MAX_KERNEL_ADDRESS),
    )
}

impl Pmap {
    /// `setregs`' `pm_guarded`: `ATTR_GP` on this user pmap's executable pages (branch target
    /// identification), or not (`EXEC_NOBTCFI`). Panics on the kernel pmap.
    pub fn set_guarded(&self, guarded: bool) {
        if self.is_kernel() {
            hal_panic(format_args!("setregs: the kernel pmap"));
        }
        self.md
            .guarded
            .store(if guarded { ATTR_GP } else { 0 }, Ordering::Relaxed);
    }
}

impl PmapBackend for Backend {
    /// The boot loader's direct map (see the deviations).
    const HAVE_PMAP_DIRECT: bool = true;
    const PMAP_STEAL_MEMORY: bool = true;
    const PMAP_WC: usize = PMAP_WC as usize;
    const PMAP_NOCACHE: usize = PMAP_NOCACHE as usize;
    const PMAP_NOMMU: bool = false;
    const POOL_WCHAN: &'static str = "pmap";
    const POOL_IPL: i32 = IPL_NONE;
    const POOL_FLAGS: i32 = 0;

    fn kernel() -> &'static Pmap {
        &KERNEL_PMAP
    }

    /// `pmap_bootstrap`: copies the boot loader's top kernel tables into the kernel pmap, maps
    /// the image by section in tables of its own and switches to them (see the redesign and
    /// the deviations), probes the ASIDs, reserves `vmmap`; the first free kernel virtual
    /// address.
    fn bootstrap(p: &BootstrapParams) -> (Paddr, Vaddr) {
        set_direct_map(p.direct_base, p.direct_end);
        let Some(img) = p.image else {
            hal_panic(format_args!("pmap_bootstrap: no kernel image"));
        };
        if !image_layout_ok(&img) {
            hal_panic(format_args!(
                "pmap_bootstrap: the image's sections cannot be mapped ({img:x?})"
            ));
        }
        IMAGE[0].store(img.start.as_usize(), Ordering::Relaxed);
        IMAGE[1].store(img.end.as_usize(), Ordering::Relaxed);
        IMAGE[2].store(img.phys_off, Ordering::Relaxed);
        let pages = KERNEL_PMAP.pages();

        let (tcr, ttbr1, mmfr0): (u64, u64, u64);
        // SAFETY: reading system registers has no side effect and touches no memory; all
        // three are readable at EL1.
        unsafe {
            core::arch::asm!(
                "mrs {tcr}, tcr_el1",
                "mrs {ttbr1}, ttbr1_el1",
                "mrs {mmfr0}, id_aa64mmfr0_el1",
                tcr = out(reg) tcr,
                ttbr1 = out(reg) ttbr1,
                mmfr0 = out(reg) mmfr0,
                options(nomem, nostack, preserves_flags),
            )
        };
        if tcr_t1sz(tcr) != T1SZ_4LEVEL {
            hal_panic(format_args!(
                "pmap_bootstrap: TTBR1_EL1 walk is not 4-level (TCR_EL1.T1SZ != 16)"
            ));
        }
        let l0_pa = Paddr::new((ttbr1 & TTBR_BADDR_MASK) as usize);

        // KERNEL IS ASSUMED TO BE 39 bits (or less) in the C; here the boot loader's four-level
        // tables are copied into vp tables (see the deviations).
        let Some(vp0) = pmap_kvp_alloc(pages) else {
            hal_panic(format_args!("pmap_bootstrap: out of memory"));
        };
        copy_frame(l0_pa, vp0.hw);

        let kslot = vp_idx(0, VM_MIN_KERNEL_ADDRESS);
        // The direct map's level-0 slots (the boot loader's tables below them, see the
        // deviations): never executable, at EL1 or EL0, whatever its leaves say (Limine's
        // were never checked; locore0.S's blocks are ATTR_XN already). Nothing executes from
        // the direct map: the image, the module window and kernel_map are slot 511's, the
        // switch's identity page is the lower half's.
        let (first, last) = (
            vp_idx(0, p.direct_base.as_usize()),
            vp_idx(0, p.direct_end.as_usize() - 1),
        );
        if first > last || last >= kslot {
            hal_panic(format_args!(
                "pmap_bootstrap: the direct map overlaps the kernel's level-0 slot"
            ));
        }
        for i in first..=last {
            let e = vp0.entry(i);
            let d = e.load(Ordering::Relaxed);
            if d & Lx_TYPE_MASK == Lx_TYPE_PT {
                e.store(d | Lx_PT_PXN | Lx_PT_XN, Ordering::Relaxed);
            } else if d & Lx_TYPE_MASK == Lx_TYPE_S {
                // A valid level-0 entry that is not a table is a block (only with TCR.DS): its
                // execute rights could not be taken away here, so refuse it.
                hal_panic(format_args!(
                    "pmap_bootstrap: the direct map's level-0 entry {i} is a block"
                ));
            }
        }
        let l1_desc = vp0.entry(kslot).load(Ordering::Relaxed);
        if l1_desc & Lx_TYPE_MASK != Lx_TYPE_PT {
            hal_panic(format_args!(
                "pmap_bootstrap: level-0 entry {kslot} is not a table"
            ));
        }
        let Some(vp1) = pmap_kvp_alloc(pages) else {
            hal_panic(format_args!("pmap_bootstrap: out of memory"));
        };
        copy_frame(Paddr::new((l1_desc & PTE_RPGN) as usize), vp1.hw);
        vp0.set_child(kslot, vp1);

        // pmap_map_stolen: the image by section, in tables of its own that replace the boot
        // loader's entry for its gigabyte in the copy (a table descriptor with no software
        // child: no walk of ours descends there; the leaves decide).
        let image = img.start.as_usize();
        if vp_idx(0, image) != kslot {
            hal_panic(format_args!(
                "pmap_bootstrap: the image is outside the kernel's level-0 slot"
            ));
        }
        let l2 = pmap_map_stolen(pages, &img);
        vp1.entry(vp_idx(1, image))
            .store(l2.as_usize() as u64 | Lx_TYPE_PT, Ordering::Release);

        let pm = &KERNEL_PMAP;
        pm.md
            .root_sw
            .store(vp0.sw.as_usize() as u64, Ordering::Release);
        pm.md
            .root_hw
            .store(vp0.hw.as_usize() as u64, Ordering::Release);
        pm.md.four_level.store(true, Ordering::Relaxed);
        pm.md.privileged.store(true, Ordering::Relaxed);
        pm.md.active.store(1, Ordering::Relaxed);
        pm.md.guarded.store(ATTR_GP, Ordering::Relaxed);
        pm.md.set_asid(0);
        // pmap_tramp: not configured (see the deviations). ASID 0 is the kernel's; the width
        // from id_aa64mmfr0_el1.
        asid::probe(mmfr0, tcr);

        // SAFETY: the fields written (indices 2, 3, 4) are the ones no mapping uses yet: the
        // boot loader's and the image's use 0 and 1 (`include/pte.rs`, deviations), the boot
        // glue's device map 2 with the value written here; the `isb` makes the new attributes
        // visible before any mapping with them is entered.
        unsafe {
            core::arch::asm!(
                "mrs {m}, mair_el1",
                "and {m}, {m}, {keep}",
                "orr {m}, {m}, {set}",
                "msr mair_el1, {m}",
                "isb",
                m = out(reg) _,
                keep = in(reg) MAIR_KEEP,
                set = in(reg) MAIR_SET,
                options(nostack, preserves_flags),
            )
        };

        // switch_mmu_kernel_table(pt1pa): the switch routine at its physical address, through
        // an identity table that maps its page alone (see the redesign), with the kernel's
        // ASID 0 and CnP clear. No "all mapped" line (see the deviations).
        let routine = switch_routine_va().wrapping_add(img.phys_off);
        if routine & PAGE_MASK > PAGE_SIZE - SWITCH_ROUTINE_LEN || routine >> 48 != 0 {
            hal_panic(format_args!(
                "pmap_bootstrap: the switch routine at {routine:#x} is not on one page"
            ));
        }
        let id = pmap_identity_table(pages, routine);
        SWITCH_ROUTINE_PA.store(routine as u64, Ordering::Release);
        SWITCH_IDENTITY.store(id.as_usize() as u64, Ordering::Release);
        switch_mmu_kernel_table();

        // The empty lower-half table the C keeps in pm_pt0pa (`init_lower_half` loads it).
        let pt0pa = pmap_steal_table(pages);
        pm.md
            .pt0pa
            .store(pt0pa.as_usize() as u64, Ordering::Release);

        // pmap_avail_fixup: nothing to fix up, uvm has the memory already (see the
        // deviations).
        asid::set_curpm(pm);

        let vstart = VM_MIN_KERNEL_ADDRESS;
        VMMAP.store(vstart, Ordering::Relaxed);
        let vstart = vstart + VMMAP_SIZE;
        VIRTUAL_AVAIL.store(vstart, Ordering::Relaxed);

        // The ABI module's window beside the image, on the kernel's tables now loaded.
        window::init(Some(img.start));
        (p.first_avail, Vaddr::new(vstart))
    }

    /// `pmap_init`'s pools (the user pmaps' is `hal::pmap::init`'s); the `TTBR0_EL1` switch is
    /// [`init_lower_half`]'s (see the deviations).
    fn init() {
        if PMAP_PTED_POOL.init(IPL_VM, 0, "pted").is_err() {
            hal_panic(format_args!("pmap_init: the pted pool"));
        }
        // pmap_vp_pool: the vp tables come from the page provider (see the redesign).
        PMAP_INITIALIZED.store(true, Ordering::Release);
    }

    fn initialized() -> bool {
        PMAP_INITIALIZED.load(Ordering::Acquire)
    }

    /// `VM_MAXUSER_ADDRESS`, or the EFI pmap's `1 << EFI_SPACE_BITS`.
    fn va_limit(pmap: &Pmap) -> Vaddr {
        Vaddr::new(pmap.md.limit)
    }

    /// `pmap_create`'s machine part (`pmap_pinit`): a full level-1 table (user pmaps are
    /// three-level, `USER_SPACE_BITS` 39), which `pm_pt0pa` names.
    fn create(_stats: &PmapStatistics, pages: &'static dyn PhysPages) -> Option<PmapMd> {
        let root = pmap_kvp_alloc(pages)?;
        Some(PmapMd::with(
            root.hw.as_usize() as u64,
            root.sw.as_usize() as u64,
            false,
            false,
            false,
            VM_MAXUSER_ADDRESS,
        ))
    }

    /// `efi_map_runtime`'s pmap: `pmap_create`'s table read as level 0 (`have_4_level_pt`),
    /// privileged.
    fn create_efi(_stats: &PmapStatistics, pages: &'static dyn PhysPages) -> Option<PmapMd> {
        let root = pmap_kvp_alloc(pages)?;
        Some(PmapMd::with(
            root.hw.as_usize() as u64,
            root.sw.as_usize() as u64,
            true,
            true,
            true,
            1 << EFI_SPACE_BITS,
        ))
    }

    fn mappings(pmap: &Pmap) -> usize {
        pmap.md.mapped.load(Ordering::Acquire)
    }

    /// `pmap_release` + `pmap_free_asid`: the ASIDs back (their TLB entries dropped on every
    /// CPU) and every table to the provider (`pmap_vp_destroy`). No reference to the pmap is
    /// left: this is its last handle's drop, and `pmap_page_protect` holds a counted one.
    fn destroy(pmap: &mut Pmap) {
        if asid::loaded_anywhere(pmap) {
            hal_panic(format_args!("pmap_destroy: the pmap is loaded on a CPU"));
        }
        // This order carries the ASID rollover's soundness (`asid.rs`): the check above (its
        // Acquire loads pair with `set_curpm`'s Release stores) and then `pmap_free_asid`, which
        // takes `pmap_asid_mtx`, both come before any table or the pool item is freed, so a
        // rollover that read this pmap from a CPU's record still holds the lock until it is
        // done with it. Never swap it back to the C's release-then-free.
        pmap_free_asid(pmap);
        let Some(&pages) = pmap.pages.get() else {
            return;
        };
        let Some(root) = pmap.md.root() else {
            return;
        };
        let top = if pmap.md.four_level() { 0 } else { 1 };
        let mut locked = pmap_lock(pmap);
        vp::destroy(
            root,
            top,
            &mut |slot| drop(slot.take(&mut locked, &PMAP_PTED_POOL)),
            &mut |t| {
                pages.free_ptp(t.hw);
                pages.free_ptp(t.sw);
            },
        );
        drop(locked);
        pages.free_ptp(root.hw);
        pages.free_ptp(root.sw);
        pmap.md.root_hw.store(0, Ordering::Release);
        pmap.md.root_sw.store(0, Ordering::Release);
    }

    /// `pmap_enter`; on the kernel pmap, `PROT_EXEC` panics (see the deviations).
    fn enter(pmap: &Pmap, va: Vaddr, pa: Paddr, prot: VmProt, flags: i32) -> Result<(), Errno> {
        if pmap.is_kernel() {
            refuse_kernel_exec("pmap_enter", va, prot);
        }
        pmap_enter(pmap, va.as_usize(), pa.as_usize(), prot, flags)
    }

    fn remove(pmap: &Pmap, sva: Vaddr, eva: Vaddr) {
        pmap_remove(pmap, sva.as_usize(), eva.as_usize());
    }

    fn protect(pmap: &Pmap, sva: Vaddr, eva: Vaddr, prot: VmProt) {
        pmap_protect(pmap, sva.as_usize(), eva.as_usize(), prot);
    }

    /// `pmap_extract`: get the physical page address for the given pmap/virtual address (the
    /// kernel's direct map and image through their offsets, see the deviations).
    fn extract(pmap: &Pmap, va: Vaddr) -> Option<Paddr> {
        if pmap.is_kernel() {
            if let Some(pa) = direct_unmap(va) {
                return Some(pa);
            }
            let (start, end) = (
                IMAGE[0].load(Ordering::Relaxed),
                IMAGE[1].load(Ordering::Relaxed),
            );
            if start <= va.as_usize() && va.as_usize() < end {
                let off = IMAGE[2].load(Ordering::Relaxed);
                return Some(Paddr::new(va.as_usize().wrapping_add(off)));
            }
        }
        let locked = pmap_lock(pmap);
        let va = va.as_usize();
        let l3 = pmap_vp_lookup(pmap, va)?;
        let pted = l3.pted(&locked).filter(|t| pted_valid(t))?;
        Some(Paddr::new(
            (pted.pte.load(Ordering::Relaxed) & PTE_RPGN) as usize | (va & PAGE_MASK),
        ))
    }

    /// `pmap_unwire`: clears the wired bit of the mapping at `va`.
    fn unwire(pmap: &Pmap, va: Vaddr) {
        let locked = pmap_lock(pmap);
        let Some(l3) = pmap_vp_lookup(pmap, va.as_usize()) else {
            return;
        };
        if let Some(pted) = l3.pted(&locked).filter(|t| pted_wired(t)) {
            add(&pmap.stats.wired_count, -1);
            pted.va
                .fetch_and(!(PTED_VA_WIRED_M as usize), Ordering::Relaxed);
        }
    }

    /// `pmap_kenter_pa`: write-back unless `pa` carries `PMAP_NOCACHE` or `PMAP_DEVICE`.
    fn kenter(va: Vaddr, pa: Paddr, prot: VmProt) {
        pmap_kenter(va, pa, prot, cache_of_pa(pa.as_usize()));
    }

    /// `pmap_kremove`: remove kernel (`pmap_kernel()`) mappings.
    fn kremove(va: Vaddr, len: Vsize) {
        let va = va.as_usize();
        for i in 0..len.as_usize() >> PAGE_SHIFT {
            pmap_kremove_pg(va + i * PAGE_SIZE);
        }
    }

    /// `pmap_setttb`'s body: if the generation of the ASID for the new pmap doesn't match the
    /// current generation, allocate a new ASID; then the empty table first, so no translation
    /// of the old address space is cached under the new ASID, and `cpu_setttb`.
    fn load(pmap: &Pmap) {
        if pmap.md.root().is_none() {
            hal_panic(format_args!("pmap_setttb: a pmap without tables"));
        }
        if pmap.md.efi {
            hal_panic(format_args!("pmap_setttb: the EFI pmap"));
        }
        if !pmap.is_kernel() && pmap.md.asid() & !PMAP_ASID_MASK != asid::generation() {
            pmap_allocate_asid(pmap);
        }
        // pmap_setpauthkeys: not configured (see the deviations).
        ttbr0_empty();
        cpu_setttb(pmap);
        asid::set_curpm(pmap);
    }

    /// `efi_enter`'s switch: the EFI pmap's tables with `T0SZ` for `EFI_SPACE_BITS`.
    fn load_efi(pmap: &Pmap) {
        if !pmap.md.efi {
            hal_panic(format_args!("efi_enter: not the EFI pmap"));
        }
        switch_space(pmap, EFI_SPACE_BITS);
    }

    /// `efi_leave`'s switch: `to` (the CPU's `ci_curpm`) with `T0SZ` for `USER_SPACE_BITS`.
    fn leave_efi(to: &Pmap) {
        if to.md.efi {
            hal_panic(format_args!("efi_leave: back to the EFI pmap"));
        }
        switch_space(to, USER_SPACE_BITS);
    }

    /// `pmap_activate`'s count: one more thread uses it.
    fn activate(pmap: &Pmap) {
        pmap.md.active.fetch_add(1, Ordering::Relaxed);
    }

    /// `pmap_deactivate`: the lower half back to the empty table and one thread less; nothing
    /// when none was counted.
    fn deactivate(pmap: &Pmap) {
        if pmap.md.active.load(Ordering::Relaxed) == 0 {
            return;
        }
        ttbr0_empty();
        pmap.md.active.fetch_sub(1, Ordering::Relaxed);
    }

    /// `pmap_purge`: called by the last thread of an exiting process. There is a theoretical
    /// chance that our sibling threads are still making their way through the tail end of
    /// exit1(); make absolutely sure they have made it past the point where they disable their
    /// userland page tables. Then its ASIDs out of every TLB, so the teardown that follows
    /// needs no more flushes.
    fn purge(pmap: &Pmap) {
        while pmap.md.active.load(Ordering::Acquire) != 1 {
            core::hint::spin_loop();
        }
        ttbr0_empty();
        let asid = pmap.md.asid();
        cpu_tlb_flush_asid_all(tlbi_asid(asid));
        cpu_tlb_flush_asid_all(tlbi_asid(asid | ASID_USER));
        pmap.md
            .pt0pa
            .store(KERNEL_PMAP.md.pt0pa(), Ordering::Release);
        pmap.md.active.store(0, Ordering::Release);
    }

    /// `pmap_proc_iflush`: each page of `[va, va + len)` synced through its direct-map alias
    /// (see the deviations).
    fn proc_iflush(pmap: &Pmap, va: Vaddr, len: Vsize) {
        let mut va = va.as_usize();
        let mut len = len.as_usize();
        while len > 0 {
            // add one to always round up to the next page
            let clen = (round_page(va + 1) - va).min(len);
            let off = va - trunc_page(va);
            // A frame that is not a managed page (device memory) is not instruction-cached and
            // may lie outside the direct map (see the deviations).
            if let Some(pa) = Self::extract(pmap, Vaddr::new(trunc_page(va)))
                && pmap
                    .pages()
                    .md_of(Paddr::new(pa.as_usize() & PMAP_PA_MASK))
                    .is_some()
            {
                cpu_icache_sync_range(direct_map(pa).as_usize() + off, clen);
            }

            len -= clen;
            va += clen;
        }
    }

    fn page_protect(md: &PageMd, prot: VmProt) {
        pmap_page_protect(md, prot);
    }

    /// `pmap_clear_modify`: drops write access from every mapping, so the next write is seen.
    fn clear_modify(md: &PageMd) -> bool {
        pmap_clear_attrs(md, PG_PMAP_MOD, PROT_WRITE)
    }

    fn clear_reference(md: &PageMd) -> bool {
        pmap_clear_attrs(md, PG_PMAP_REF, PROT_MASK)
    }

    fn is_modified(md: &PageMd) -> bool {
        md.attrs.load(Ordering::Relaxed) & PG_PMAP_MOD != 0
    }

    fn is_referenced(md: &PageMd) -> bool {
        md.attrs.load(Ordering::Relaxed) & PG_PMAP_REF != 0
    }

    /// `pmap_zero_page`: `pagezero_cache` through the direct map.
    fn zero_page(pa: Paddr) {
        zero_frame(pa);
    }

    /// `pmap_copy_page`: through the direct map.
    fn copy_page(src: Paddr, dst: Paddr) {
        copy_frame(src, dst);
    }

    /// `pmap_steal_memory` (not in the C, see the deviations): `size` bytes of free physical
    /// memory, zeroed, through the direct map.
    fn steal_memory(pages: &'static dyn PhysPages, size: Vsize) -> Vaddr {
        let size = round_page(size.as_usize());
        let npg = atop(size);
        if npg == 0 {
            // No page taken would still hand out an address (the front refuses the sizes
            // that round to none; this keeps the backend's own contract).
            hal_panic(format_args!("pmap_steal_memory: no page"));
        }
        let Some(pa) = pages.steal(npg) else {
            hal_panic(format_args!(
                "pmap_steal_memory: unable to allocate region with size {size:#x}"
            ));
        };
        for i in 0..npg {
            zero_frame(Paddr::new(pa.as_usize() + i * PAGE_SIZE));
        }
        direct_map(pa)
    }

    /// `pmap_virtual_space`: the free kernel virtual range; prevents further KVA stealing.
    fn virtual_space() -> (Vaddr, Vaddr) {
        PMAP_VIRTUAL_SPACE_CALLED.store(true, Ordering::Relaxed);
        stolen_bounds()
    }

    /// `pmap_growkernel`: makes the kernel's vp tables and descriptors reach `maxkvaddr`, the
    /// table descriptors of the managed range with `Lx_PT_PXN` (see the deviations).
    fn growkernel(maxkvaddr: Vaddr) -> Vaddr {
        let maxkvaddr = maxkvaddr.as_usize();
        // Not strictly necessary, but we use an interrupt-safe map and uvm asserts that we're
        // at IPL_VM: the lock's level.
        let mut g = GROW.lock();
        if maxkvaddr <= g.maxkvaddr {
            return Vaddr::new(g.maxkvaddr);
        }
        let pm = &KERNEL_PMAP;
        let pages = pm.pages();
        let Some(vp1) = pm.md.root().and_then(|r| r.child(vp_idx(0, g.maxkvaddr))) else {
            hal_panic(format_args!(
                "pmap_growkernel: no level-1 table for the kernel range"
            ));
        };

        'fail: {
            for i in vp_idx(1, g.maxkvaddr)..=vp_idx(1, maxkvaddr - 1) {
                // Never executable at EL1 below a level-1 slot of the managed range.
                let pxn = if kernel_map_slot(i) { Lx_PT_PXN } else { 0 };
                let vp2 = match vp1.child(i) {
                    Some(t) => t,
                    None => {
                        let Some(t) = pmap_kvp_alloc(pages) else {
                            break 'fail;
                        };
                        vp1.set_child_with(i, t, pxn);
                        t
                    }
                };

                let lb_idx2 = if i == vp_idx(1, g.maxkvaddr) {
                    vp_idx(2, g.maxkvaddr)
                } else {
                    0
                };
                let ub_idx2 = if i == vp_idx(1, maxkvaddr - 1) {
                    vp_idx(2, maxkvaddr - 1)
                } else {
                    VP_IDX2_CNT - 1
                };

                for j in lb_idx2..=ub_idx2 {
                    let vp3 = match vp2.child(j) {
                        Some(t) => t,
                        None => {
                            let Some(t) = pmap_kvp_alloc(pages) else {
                                break 'fail;
                            };
                            vp2.set_child_with(j, t, pxn);
                            t
                        }
                    };

                    for k in 0..VP_IDX3_CNT {
                        if !vp3.kernel_slot_full(k) {
                            let Some(pted) = pmap_kpted_alloc(&mut g, pages) else {
                                break 'fail;
                            };
                            vp3.set_kernel_slot(k, pted);
                            g.maxkvaddr += PAGE_SIZE;
                        }
                    }
                }
            }
            kassert!(g.maxkvaddr >= maxkvaddr);
        }
        Vaddr::new(g.maxkvaddr)
    }

    fn direct_map(pa: Paddr) -> Vaddr {
        direct_map(pa)
    }

    fn direct_unmap(va: Vaddr) -> Option<Paddr> {
        direct_unmap(va)
    }

    /// No `pmap_flush_cache` in the C's arm64: the data cache written back and invalidated over
    /// the range (`cpu_dcache_wbinv_range`), for the callers amd64's has.
    fn flush_cache(va: Vaddr, len: Vsize) {
        cpu_dcache_wbinv_range(va.as_usize(), len.as_usize());
    }

    /// The window beside the image (`window.rs`), the module's only place, as on amd64.
    fn module_window() -> Option<(Vaddr, usize)> {
        Some(window::window())
    }

    fn module_enter(va: Vaddr, pa: Paddr, prot: VmProt) -> bool {
        window::enter(va, pa, prot)
    }

    /// `pmap_kenter_cache`: enter a kernel mapping with the given cache mode.
    fn kenter_cache(va: Vaddr, pa: Paddr, prot: VmProt, cacheable: i32) {
        pmap_kenter(va, pa, prot, cacheable);
    }

    /// `pmap_fault_fixup`: this function exists to do software referenced/modified emulation.
    /// Its purpose is to tell the caller that a fault was generated either for this emulation,
    /// or to tell the caller that it's a legit fault.
    fn fault_fixup(pmap: &Pmap, va: Vaddr, ftype: VmProt) -> bool {
        pmap_fault_fixup(pmap, va.as_usize(), ftype)
    }
}

// The PTE bits (`ap_bits_user`, `ap_bits_kern`): what the tables grant by construction.
const _: () = {
    let mut prot = 1;
    while prot < 8 {
        let user = AP_BITS_USER[prot];
        let kern = AP_BITS_KERN[prot];
        // The kernel never executes a user page; EL0 never executes a kernel page.
        assert!(user & ATTR_PXN != 0);
        assert!(kern & ATTR_UXN != 0);
        // Every granted page is accessed already (no access-flag fault).
        assert!(user & ATTR_AF != 0 && kern & ATTR_AF != 0);
        // EL0 reads user pages (AP[1] set) but the execute-only ones (`PROT_EXEC` alone:
        // AP 0b10, EL0 may fetch, not load), never kernel ones (AP[1] clear).
        assert!((user & attr_ap(1) != 0) == (prot as i32 != PROT_EXEC));
        assert!(kern & attr_ap(1) == 0);
        // Writable exactly when PROT_WRITE (AP[2], the read-only bit, clear).
        let writable = prot as i32 & PROT_WRITE != 0;
        assert!((user & attr_ap(2) == 0) == writable);
        assert!((kern & attr_ap(2) == 0) == writable);
        // Executable exactly when PROT_EXEC: UXN for EL0, PXN for EL1.
        let exec = prot as i32 & PROT_EXEC != 0;
        assert!((user & ATTR_UXN == 0) == exec);
        assert!((kern & ATTR_PXN == 0) == exec);
        prot += 1;
    }
    assert!(AP_BITS_USER[0] == 0 && AP_BITS_KERN[0] == 0);
    // The MAIR indices the cache modes select are the ones the bootstrap writes or keeps.
    assert!(MAIR_SET & !MAIR_KEEP == MAIR_SET);
    assert!(PTE_ATTR_DEV_NGNRNE == 2 && PTE_ATTR_CI == 3 && PTE_ATTR_WT == 4);
    assert!(PTE_ATTR_WB == 0 && PTE_ATTR_DEV_NGNRE == 1);
    // The kernel leaves (`layout.rs`'s `kernel_leaf`: the image, the identity page, the module
    // window) carry no branch-target guard; their host tests check the rest.
    let pa = 0x4000_0000;
    assert!(kernel_leaf(pa, PROT_READ | PROT_EXEC) & ATTR_GP == 0);
    assert!(kernel_leaf(pa, PROT_READ) & ATTR_GP == 0);
    assert!(kernel_leaf(pa, PROT_READ | PROT_WRITE) & ATTR_GP == 0);
    // `Lx_PT_PXN` and `Lx_PT_XN` are table descriptors' bits the walks ignore: above the output
    // address (`Table::child`'s `PTE_RPGN`), outside the type bits.
    assert!((Lx_PT_PXN | Lx_PT_XN) & (PTE_RPGN | Lx_TYPE_MASK) == 0);
};

/* </CODE> */
