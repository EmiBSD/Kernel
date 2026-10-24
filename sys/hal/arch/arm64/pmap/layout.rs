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
//! arm64 `<machine/pmap.h>`'s layout and the arithmetic of `pmap.c` that needs no machine:
//! the vp tables' geometry, the cache and pted flags, the ASID pairs, the operands of the
//! TLB invalidations and the translation-control fields the pmap writes.
//!
//! Upstream: sys/arch/arm64/include/pmap.h @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/pmap.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//!
//! Pure constants and `const fn`s, so the host tests compile this file by `#[path]`
//! (`sys/hal/arch/host/mod.rs`, D15): its neighbours are reached as `super::super::include`,
//! the same path on the machine and on the host.
//!
//! ## Deviations
//! - The fields of `TCR_EL1`, `TTBR1_EL1` and `ID_AA64MMFR0_EL1` the pmap reads and writes
//!   (`TCR_T0SZ`, `TCR_T1SZ`, `TCR_A1`, `TCR_AS`, the table base, the ASID width) are written
//!   here from `<machine/armreg.h>`, whose port stays in `bsd` (it names `bsd`'s CPU types)
//!   until the CPU code moves (N4).
//!
//! ## Redesign
//! - LZ's `include/pmap.rs` holds this layout with `struct pmap`, `struct vm_page_md` and
//!   `VM_MDPAGE_INIT`; those are `hal::pmap`'s and the backend's now (`backend.rs`), and this
//!   file keeps the layout alone, with what `pmap.c` computes from it: `VP_IDX0`..`VP_IDX3` as
//!   [`vp_idx`] by level, `pmap_enter`'s and `pmap_kenter_pa`'s cache mode from the flag bits
//!   of the physical address ([`cache_of_pa`]), the bitmap word and bits of an ASID pair
//!   ([`asid_slot`]), the operands of `ttlb_flush`'s and `pmap_free_asid`'s `tlbi`
//!   ([`tlbi_page`], [`tlbi_asid`]), and the ASID width `pmap_bootstrap` probes
//!   ([`nasid_probe`], which also says why it stays at 8 bits).
//! - New in native (N3.1 commit 7): the ABI module window's geometry and its two pure
//!   checks, where it goes in the image's level-2 table ([`module_window_place`]) and which
//!   protections it maps ([`module_prot_ok`]); `window.rs` links and fills it. And the
//!   level-1 slots of the kernel's managed range ([`kernel_map_slot`]), whose table
//!   descriptors `pmap_growkernel` marks `Lx_PT_PXN` (the backend's deviations).
//! - N3.1 commit 8: `pmap_map_stolen`'s choice of protection by section, for the image's
//!   pages ([`image_page_prot`]), the checks the bootstrap makes of the section bounds the
//!   boot glue passes ([`image_layout_ok`]) and how many level-3 tables they take
//!   ([`image_ptps`]), and the `TCR_EL1` the switch to the kernel's tables runs under while
//!   its identity table is in `TTBR0_EL1` ([`tcr_identity`]). N3.1 commit 8b: `ap_bits_kern`
//!   ([`AP_BITS_KERN`]) and the kernel leaf every page the pmap maps outside `pmap_enter`
//!   takes ([`kernel_leaf`]: the image, the switch's identity page, the module window) moved
//!   here from the backend, so the host tests check them; the `TCR_EL1` bits `pmap_init`
//!   clears ([`TCR_HPD`]).
//! - N4.8d: the distance between the kernel's root and `pmap_tramp`'s
//!   ([`TRAMP_ROOT_OFFSET`]; the C's `2 * PAGE_SIZE` literal, written in `pmap.c` and twice in
//!   `trampoline.S`) is one constant here, which `trampoline.S` takes as a `const` operand of
//!   `global_asm!`, and the `TTBR1_EL1` arithmetic of the trampoline's two switches is
//!   mirrored by [`tramp_ttbr1`] and [`kernel_ttbr1`], which the host tests pin.

use super::super::include::param::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use super::super::include::pte::{
    ATTR_AF, ATTR_PXN, ATTR_UXN, ATTR_nG, L3_P, PTE_ATTR_WB, PTE_RPGN, SH_INNER, attr_ap, attr_idx,
    attr_sh,
};
use super::super::include::vmparam::{VM_MAX_KERNEL_ADDRESS, VM_MIN_KERNEL_ADDRESS};
use crate::pmap::{KernelImage, PG_PMAP2, PMAP_MD0, PMAP_MD1, PMAP_MD2, PMAP_MD3, VmProt};
use crate::sys::mman::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};

// V->P mapping data

/// `VP_IDX0_CNT`: entries in a level-0 table.
pub const VP_IDX0_CNT: usize = 512;
/// `VP_IDX0_MASK`.
pub const VP_IDX0_MASK: usize = VP_IDX0_CNT - 1;
/// `VP_IDX0_POS`: the level-0 index starts at this bit of the virtual address.
pub const VP_IDX0_POS: u32 = 39;
/// `VP_IDX1_CNT`: entries in a level-1 table.
pub const VP_IDX1_CNT: usize = 512;
/// `VP_IDX1_MASK`.
pub const VP_IDX1_MASK: usize = VP_IDX1_CNT - 1;
/// `VP_IDX1_POS`.
pub const VP_IDX1_POS: u32 = 30;
/// `VP_IDX2_CNT`: entries in a level-2 table.
pub const VP_IDX2_CNT: usize = 512;
/// `VP_IDX2_MASK`.
pub const VP_IDX2_MASK: usize = VP_IDX2_CNT - 1;
/// `VP_IDX2_POS`.
pub const VP_IDX2_POS: u32 = 21;
/// `VP_IDX3_CNT`: entries in a level-3 table.
pub const VP_IDX3_CNT: usize = 512;
/// `VP_IDX3_MASK`.
pub const VP_IDX3_MASK: usize = VP_IDX3_CNT - 1;
/// `VP_IDX3_POS`.
pub const VP_IDX3_POS: u32 = 12;

/// The entries of one vp table, every level.
pub const VP_ENTRIES: usize = 512;

// cache flags

/// `PMAP_CACHE_CI`: cache inhibit.
pub const PMAP_CACHE_CI: i32 = PMAP_MD0;
/// `PMAP_CACHE_WT`: writethru.
pub const PMAP_CACHE_WT: i32 = PMAP_MD1;
/// `PMAP_CACHE_WB`: writeback.
pub const PMAP_CACHE_WB: i32 = PMAP_MD1 | PMAP_MD0;
/// `PMAP_CACHE_DEV_NGNRNE`: device nGnRnE.
pub const PMAP_CACHE_DEV_NGNRNE: i32 = PMAP_MD2;
/// `PMAP_CACHE_DEV_NGNRE`: device nGnRE.
pub const PMAP_CACHE_DEV_NGNRE: i32 = PMAP_MD2 | PMAP_MD0;
/// `PMAP_CACHE_BITS`: the cache flags.
pub const PMAP_CACHE_BITS: i32 = PMAP_MD0 | PMAP_MD1 | PMAP_MD2;

/// `PTED_VA_MANAGED_M`: the mapping is of a managed page (on its pv list).
pub const PTED_VA_MANAGED_M: i32 = PMAP_MD3;
/// `PTED_VA_WIRED_M`: the mapping is wired.
pub const PTED_VA_WIRED_M: i32 = PMAP_MD3 << 1;

/// `PMAP_PA_MASK`: to remove the flags.
pub const PMAP_PA_MASK: usize = !PAGE_MASK;
/// `PMAP_NOCACHE`: non-cacheable memory.
pub const PMAP_NOCACHE: i32 = 0x1;
/// `PMAP_DEVICE`: device memory.
pub const PMAP_DEVICE: i32 = 0x2;
/// `PMAP_WC`: write combining is device memory here.
pub const PMAP_WC: i32 = PMAP_DEVICE;

/// `PG_PMAP_EXE`: the page was mapped executable and its instruction cache is clean.
pub const PG_PMAP_EXE: u32 = PG_PMAP2;

/// `PROT_MASK` (`uvm_extern.h`): the protection bits of a `vm_prot_t`.
pub const PROT_MASK: i32 = 0x07;

// ASIDs

/// `PMAP_MAX_NASID`: the widest ASID space, 16 bits.
pub const PMAP_MAX_NASID: usize = 1 << 16;
/// `PMAP_ASID_MASK`: the ASID bits of `pm_asid`; the generation sits above them.
pub const PMAP_ASID_MASK: u64 = PMAP_MAX_NASID as u64 - 1;
/// `ASID_USER`: we run userland code with ASIDs that have the low bit set.
pub const ASID_USER: u64 = 1;

// The translation registers (<machine/armreg.h>, see the deviations)

/// `TCR_T0SZ(0x3f)`: the size field of the lower half.
pub const TCR_T0SZ_MASK: u64 = 0x3f;
/// `TCR_T1SZ_SHIFT`: the size field of the upper half.
pub const TCR_T1SZ_SHIFT: u32 = 16;
/// `TCR_A1`: the ASID comes from `TTBR1_EL1`.
pub const TCR_A1: u64 = 1 << 22;
/// `TCR_AS`: 16-bit ASIDs.
pub const TCR_AS: u64 = 1 << 36;
/// `T1SZ` of a four-level upper half (48 bits), what both boot paths leave.
pub const T1SZ_4LEVEL: u64 = 16;
/// `TCR_EL1.HPD0` and `HPD1` (bits 41 and 42, FEAT_HPDS; not in the C's `armreg.h`, whose
/// `locore.S` leaves them clear): when set, the table descriptors' `PXNTable`, `UXNTable` and
/// `APTable` are ignored. `pmap_init` clears both, as the C's `TCR_EL1` has them, so the
/// `Lx_PT_PXN` and `Lx_PT_XN` the pmap puts on table descriptors are enforced.
pub const TCR_HPD: u64 = (1 << 41) | (1 << 42);
/// `TCR_EL1`'s `TTBR0_EL1` fields, bits 15:0: `T0SZ`, `EPD0`, `IRGN0`, `ORGN0`, `SH0`, `TG0`.
pub const TCR_TTBR0_FIELDS: u64 = 0xffff;
/// The `TTBR0_EL1` walk of the identity table the switch to the kernel's tables runs from
/// (the backend's `switch_mmu_kernel_table`): 48 bits (`T0SZ` 16, four levels), walks enabled
/// (`EPD0` clear), write-back write-allocate inner and outer (`IRGN0`, `ORGN0` 0b01), inner
/// shareable (`SH0` 0b11), 4 KiB granule (`TG0` 0b00).
pub const TCR_IDENTITY_TTBR0: u64 = 16 | (0b01 << 8) | (0b01 << 10) | (0b11 << 12);
/// `TTBRx_EL1.BADDR`: bits 47:1 hold the table's physical address.
pub const TTBR_BADDR_MASK: u64 = 0x0000_ffff_ffff_fffe;
/// How far `pmap_tramp`'s root lies above the kernel's (the backend's `pmap_steal_roots`
/// steals both hardware pages in one block): what `trampoline.S` adds to `TTBR1_EL1` on the
/// way out to user mode and takes away on the way in. The C's `2 * PAGE_SIZE` is a
/// `struct pmapvp1` (a hardware page and a software page); here a vp table's two pages are
/// paired by `Table`, not by adjacency, so only the hardware roots are, one page apart.
pub const TRAMP_ROOT_OFFSET: usize = PAGE_SIZE;

/// `tramp_return`'s `orr x18, x18, #(1 << 48); add x18, x18, #TRAMP_ROOT_OFFSET`: the
/// `TTBR1_EL1` user code runs under, from the one the kernel runs under (`kernel`: the
/// kernel's root and a pair's even ASID). The odd ASID of the pair, the trampoline's root;
/// `CnP` and the other ASID bits unchanged. `kernel`'s base is below 48 bits and a page below
/// the last page, so the `add` carries into nothing.
pub const fn tramp_ttbr1(kernel: u64) -> u64 {
    (kernel | ASID_USER << 48) + TRAMP_ROOT_OFFSET as u64
}

/// The EL0 vectors' `bic x18, x18, #(1 << 48); sub x18, x18, #TRAMP_ROOT_OFFSET`: back from
/// [`tramp_ttbr1`]'s value to the kernel's, before any memory access of the kernel.
pub const fn kernel_ttbr1(tramp: u64) -> u64 {
    (tramp & !(ASID_USER << 48)) - TRAMP_ROOT_OFFSET as u64
}
/// `ID_AA64MMFR0_ASID_BITS_MASK`.
pub const ID_AA64MMFR0_ASID_BITS_MASK: u64 = 0xf << 4;
/// `ID_AA64MMFR0_ASID_BITS_16`: the CPU has 16-bit ASIDs.
pub const ID_AA64MMFR0_ASID_BITS_16: u64 = 0x2 << 4;
/// `EFI_SPACE_BITS` (`efi_machdep.c`): the width of the EFI runtime pmap's address space.
pub const EFI_SPACE_BITS: u32 = 48;
/// The ABI module window's size (`window.rs`): the largest image the module loader takes
/// (`hal::module::elf::MAX_SPAN`), so the window never refuses a module the loader accepts.
pub const MODULE_WINDOW_SIZE: usize = crate::module::elf::MAX_SPAN as usize;
/// The level-3 tables the window needs: one per level-2 entry (2 MiB).
pub const MODULE_WINDOW_NPTP: usize = MODULE_WINDOW_SIZE >> VP_IDX2_POS;

/// `ap_bits_kern[]`: the access bits of a kernel mapping, by `PROT_*`.
pub const AP_BITS_KERN: [u64; 8] = [
    0,                                          // PROT_NONE
    ATTR_PXN | ATTR_UXN | ATTR_AF | attr_ap(2), // PROT_READ
    ATTR_PXN | ATTR_UXN | ATTR_AF | attr_ap(0), // PROT_WRITE
    ATTR_PXN | ATTR_UXN | ATTR_AF | attr_ap(0), // PROT_WRITE|PROT_READ
    ATTR_UXN | ATTR_AF | attr_ap(2),            // PROT_EXEC
    ATTR_UXN | ATTR_AF | attr_ap(2),            // PROT_EXEC|PROT_READ
    ATTR_UXN | ATTR_AF | attr_ap(0),            // PROT_EXEC|PROT_WRITE
    ATTR_UXN | ATTR_AF | attr_ap(0),            // PROT_EXEC|PROT_WRITE|PROT_READ
];

/// How wide the ASIDs are, and why ([`nasid_probe`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsidWidth {
    /// 16 bits: the CPU has them (`ID_AA64MMFR0_EL1.ASIDBits`) and `TCR_EL1.AS` selects them.
    Bits16,
    /// 8 bits: what the CPU implements.
    Bits8,
    /// 8 bits: the CPU has 16, but the boot loader left `TCR_EL1.AS` clear (Limine does not
    /// promise it; the C's `locore.S` sets it, as boot(8)'s path does here), so the TLB tags
    /// with 8 and a 16-bit ASID would alias.
    Bits8TcrAs,
}

impl AsidWidth {
    /// `pmap_nasid`: how many ASIDs there are.
    pub const fn nasid(self) -> usize {
        match self {
            AsidWidth::Bits16 => 1 << 16,
            AsidWidth::Bits8 | AsidWidth::Bits8TcrAs => 1 << 8,
        }
    }

    /// The probe's answer in words (for ddb and the tests; OpenBSD prints no boot line for it).
    pub const fn why(self) -> &'static str {
        match self {
            AsidWidth::Bits16 => "16-bit ASIDs",
            AsidWidth::Bits8 => "8-bit ASIDs (the CPU's)",
            AsidWidth::Bits8TcrAs => "8-bit ASIDs (TCR_EL1.AS clear at boot)",
        }
    }
}

/// `VP_IDX0(va)`..`VP_IDX3(va)`: the index of `va` in its table of `level` (0 to 3).
pub const fn vp_idx(level: usize, va: usize) -> usize {
    const POS: [u32; 4] = [VP_IDX0_POS, VP_IDX1_POS, VP_IDX2_POS, VP_IDX3_POS];
    (va >> POS[level]) & (VP_ENTRIES - 1)
}

/// `pmap_enter`'s and `pmap_kenter_pa`'s cache mode: write-back, or the one the physical
/// address's flag bits ask for (`PMAP_DEVICE` over `PMAP_NOCACHE`, as the C tests them in that
/// order).
pub const fn cache_of_pa(pa: usize) -> i32 {
    if pa & PMAP_DEVICE as usize != 0 {
        PMAP_CACHE_DEV_NGNRNE
    } else if pa & PMAP_NOCACHE as usize != 0 {
        PMAP_CACHE_CI
    } else {
        PMAP_CACHE_WB
    }
}

/// Whether `cache` is one of the five cache modes (`pmap_fill_pte` panics otherwise).
pub const fn cache_valid(cache: i32) -> bool {
    matches!(
        cache,
        PMAP_CACHE_WB
            | PMAP_CACHE_WT
            | PMAP_CACHE_CI
            | PMAP_CACHE_DEV_NGNRNE
            | PMAP_CACHE_DEV_NGNRE
    )
}

/// The word of `pmap_asid[]` and the two bits of the pair whose even member is `asid` (the
/// kernel's view and, with `ASID_USER`, the user's).
pub const fn asid_slot(asid: usize) -> (usize, u32) {
    (asid / 32, 3u32 << (asid & (32 - 1)))
}

/// The page operand of `tlbi vae1is`/`vaale1is` for `va`: its page number, bits 43:0.
pub const fn tlbi_page(va: usize) -> u64 {
    ((va >> PAGE_SHIFT) & ((1usize << 44) - 1)) as u64
}

/// The ASID field of a `tlbi` operand (bits 63:48), from a `pm_asid` (the generation above
/// the 16 bits falls off, as the C's shift drops it).
pub const fn tlbi_asid(asid: u64) -> u64 {
    asid << 48
}

/// `pmap_bootstrap`'s probe: 16-bit ASIDs when `ID_AA64MMFR0_EL1` has them, provided the boot
/// loader's `TCR_EL1` selects them (`TCR_AS`), else 8 (see [`AsidWidth`]).
pub const fn nasid_probe(id_aa64mmfr0: u64, tcr: u64) -> AsidWidth {
    if id_aa64mmfr0 & ID_AA64MMFR0_ASID_BITS_MASK != ID_AA64MMFR0_ASID_BITS_16 {
        AsidWidth::Bits8
    } else if tcr & TCR_AS == 0 {
        AsidWidth::Bits8TcrAs
    } else {
        AsidWidth::Bits16
    }
}

/// `tcr` with its lower half `bits` wide (`TCR_T0SZ(64 - bits)`).
pub const fn tcr_t0sz(tcr: u64, bits: u32) -> u64 {
    (tcr & !TCR_T0SZ_MASK) | (64 - bits as u64)
}

/// The `T1SZ` field of `tcr`.
pub const fn tcr_t1sz(tcr: u64) -> u64 {
    (tcr >> TCR_T1SZ_SHIFT) & 0x3f
}

/// `tcr` with the lower half's walk the identity table needs ([`TCR_IDENTITY_TTBR0`]); the
/// upper half's fields and the rest stay.
pub const fn tcr_identity(tcr: u64) -> u64 {
    (tcr & !TCR_TTBR0_FIELDS) | TCR_IDENTITY_TTBR0
}

/// `pmap_pte_update`'s descriptor of a write-back kernel mapping of `pa` with `prot`, never
/// guarded (no `ATTR_GP`: the kernel and the ABI module are built without BTI landing pads,
/// the backend's deviations): the image's pages (`pmap_map_stolen`), the switch's identity
/// page and the module window's leaves (`window.rs`). 0 for `PROT_NONE`, as
/// `pmap_pte_update` stores.
pub const fn kernel_leaf(pa: u64, prot: VmProt) -> u64 {
    let access_bits = AP_BITS_KERN[(prot & PROT_MASK) as usize];
    if access_bits == 0 {
        return 0;
    }
    let attr = ATTR_nG | attr_idx(PTE_ATTR_WB) | attr_sh(SH_INNER);
    (pa & PTE_RPGN) | attr | access_bits | L3_P
}

/// The highest physical address a descriptor holds (`PTE_RPGN`'s bits 47:12), plus one.
const PA_LIMIT: usize = 1 << 48;

/// Whether the image's section bounds are what `pmap_map_stolen` can map: `start` page-aligned
/// and before the end of the text, the read-only data and the data starting on pages, in that
/// order, without overlap (`text_end <= rodata_start <= rodata_end <= data_start <= end`), the
/// physical offset a whole number of pages, every page of the image below 48 physical bits,
/// and the whole image inside one gigabyte (one level-1 entry, one level-2 table).
pub fn image_layout_ok(img: &KernelImage) -> bool {
    let start = img.start.as_usize();
    let (text_end, end) = (img.text_end.as_usize(), img.end.as_usize());
    let (ro, ro_end) = (img.rodata_start.as_usize(), img.rodata_end.as_usize());
    let data = img.data_start.as_usize();
    let aligned = |a: usize| a & PAGE_MASK == 0;
    if !(aligned(start) && aligned(ro) && aligned(ro_end) && aligned(data))
        || !(start < text_end && text_end <= ro && ro <= ro_end && ro_end <= data && data < end)
        || !aligned(img.phys_off)
    {
        return false;
    }
    let Some(last) = end
        .checked_add(PAGE_MASK)
        .map(|e| (e & !PAGE_MASK) - PAGE_SIZE)
    else {
        return false;
    };
    let (pa_first, pa_last) = (
        start.wrapping_add(img.phys_off),
        last.wrapping_add(img.phys_off),
    );
    pa_first <= pa_last && pa_last < PA_LIMIT && start >> VP_IDX1_POS == last >> VP_IDX1_POS
}

/// `pmap_map_stolen`'s protection of the image's page at `va` (its page's first address):
/// read and execute in the text (`__text_start` to `_etext`, the page holding `_etext`
/// included), read in the read-only data (`__rodata_start` to `_erodata`), read and write in
/// the data and BSS (`__data_start` to the page holding `_end`). `None` for a page outside the
/// image or in the padding between two sections, which is not mapped (the C maps every stolen
/// page read-write but those two).
pub fn image_page_prot(img: &KernelImage, va: usize) -> Option<VmProt> {
    let va = va & !PAGE_MASK;
    let end = img.end.as_usize().checked_add(PAGE_MASK)? & !PAGE_MASK;
    if img.start.as_usize() <= va && va < img.text_end.as_usize() {
        Some(PROT_READ | PROT_EXEC)
    } else if img.rodata_start.as_usize() <= va && va < img.rodata_end.as_usize() {
        Some(PROT_READ)
    } else if img.data_start.as_usize() <= va && va < end {
        Some(PROT_READ | PROT_WRITE)
    } else {
        None
    }
}

/// The level-3 tables the image takes: one per level-2 entry its pages touch (an image that
/// passed [`image_layout_ok`]).
pub fn image_ptps(img: &KernelImage) -> usize {
    let first = img.start.as_usize() >> VP_IDX2_POS;
    let last = (img.end.as_usize() - 1) >> VP_IDX2_POS;
    last - first + 1
}

/// Where the module window goes in the level-2 table of the gigabyte at `gib` (the kernel
/// image's), whose last entry in use is `last`: the index of its first level-2 entry, the
/// one after `last`, and its first address. `None` when `gib` is not a gigabyte's address,
/// the [`MODULE_WINDOW_NPTP`] entries do not fit in the table after `last`, or the window
/// would meet the kernel's managed range (`VM_MIN_KERNEL_ADDRESS`..=`VM_MAX_KERNEL_ADDRESS`,
/// `vmmap` and `kernel_map`), whose tables `pmap_growkernel` makes.
pub const fn module_window_place(gib: usize, last: usize) -> Option<(usize, usize)> {
    if gib & ((1 << VP_IDX1_POS) - 1) != 0 || last >= VP_ENTRIES {
        return None;
    }
    let first = last + 1;
    if first + MODULE_WINDOW_NPTP > VP_ENTRIES {
        return None;
    }
    let base = gib + (first << VP_IDX2_POS);
    // Its last byte, inside the gigabyte (aligned, so no overflow even at the top one).
    let end = base + (MODULE_WINDOW_SIZE - 1);
    if base <= VM_MAX_KERNEL_ADDRESS && VM_MIN_KERNEL_ADDRESS <= end {
        return None;
    }
    Some((first, base))
}

/// Whether the level-1 slot `l1` of the kernel's level-0 slot is one of the managed range's
/// (`VM_MIN_KERNEL_ADDRESS`..=`VM_MAX_KERNEL_ADDRESS`: `vmmap` and `kernel_map`), whose
/// table descriptors `pmap_growkernel` makes never executable at EL1; the image's
/// gigabyte is not one.
pub const fn kernel_map_slot(l1: usize) -> bool {
    vp_idx(1, VM_MIN_KERNEL_ADDRESS) <= l1 && l1 <= vp_idx(1, VM_MAX_KERNEL_ADDRESS)
}

/// Whether the module window maps a page with `prot`: a protection the kernel's access bits
/// grant (`ap_bits_kern`, not `PROT_NONE`) and never both writable and executable (W^X; the
/// loader asks for read, read-write or read-execute only).
pub const fn module_prot_ok(prot: VmProt) -> bool {
    prot & !PROT_MASK == 0
        && prot != PROT_NONE
        && prot & (PROT_WRITE | PROT_EXEC) != (PROT_WRITE | PROT_EXEC)
}

const _: () = {
    assert!(PMAP_CACHE_BITS & (PTED_VA_MANAGED_M | PTED_VA_WIRED_M) == 0);
    assert!(
        (PMAP_CACHE_BITS | PTED_VA_MANAGED_M | PTED_VA_WIRED_M | PROT_MASK) as usize & !PAGE_MASK
            == 0
    );
    assert!(PMAP_ASID_MASK == 0xffff);
    assert!(TRAMP_ROOT_OFFSET > 0 && TRAMP_ROOT_OFFSET & PAGE_MASK == 0);
    assert!(MODULE_WINDOW_SIZE.is_multiple_of(1 << VP_IDX2_POS));
    assert!(MODULE_WINDOW_NPTP > 0 && MODULE_WINDOW_NPTP < VP_ENTRIES);
    // The managed range lies in one level-0 slot, so its level-1 slots name it alone.
    assert!(vp_idx(0, VM_MIN_KERNEL_ADDRESS) == vp_idx(0, VM_MAX_KERNEL_ADDRESS));
    assert!(VM_MIN_KERNEL_ADDRESS & ((1 << VP_IDX1_POS) - 1) == 0);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_indices_split_a_va_by_level() {
        let va = (3 << VP_IDX0_POS) | (5 << VP_IDX1_POS) | (7 << VP_IDX2_POS) | (9 << VP_IDX3_POS);
        assert_eq!(
            (vp_idx(0, va), vp_idx(1, va), vp_idx(2, va), vp_idx(3, va)),
            (3, 5, 7, 9)
        );
        // The kernel's first address is the last level-0 slot.
        assert_eq!(vp_idx(0, 0xffff_ff80_0000_0000), 511);
        assert_eq!(vp_idx(1, 0xffff_ff80_0000_0000), 0);
    }

    #[test]
    fn the_cache_mode_follows_the_flag_bits_device_first() {
        let cases: &[(usize, i32)] = &[
            (0x4000_0000, PMAP_CACHE_WB),
            (0x4000_0000 | PMAP_NOCACHE as usize, PMAP_CACHE_CI),
            (0x4000_0000 | PMAP_DEVICE as usize, PMAP_CACHE_DEV_NGNRNE),
            (
                0x4000_0000 | (PMAP_DEVICE | PMAP_NOCACHE) as usize,
                PMAP_CACHE_DEV_NGNRNE,
            ),
        ];
        for &(pa, cache) in cases {
            assert_eq!(cache_of_pa(pa), cache, "{pa:#x}");
            assert!(cache_valid(cache));
        }
        assert!(!cache_valid(0));
        assert!(!cache_valid(PMAP_CACHE_BITS));
    }

    #[test]
    fn the_flags_are_the_headers() {
        assert_eq!(
            (
                PMAP_CACHE_CI,
                PMAP_CACHE_WT,
                PMAP_CACHE_WB,
                PMAP_CACHE_DEV_NGNRNE,
                PMAP_CACHE_DEV_NGNRE
            ),
            (0x40, 0x80, 0xc0, 0x100, 0x140)
        );
        assert_eq!((PTED_VA_MANAGED_M, PTED_VA_WIRED_M), (0x200, 0x400));
        assert_eq!(PG_PMAP_EXE, 0x0400_0000);
    }

    #[test]
    fn an_asid_pair_is_two_bits_of_one_word() {
        assert_eq!(asid_slot(0), (0, 0b11));
        assert_eq!(asid_slot(30), (0, 0b11 << 30));
        assert_eq!(asid_slot(32), (1, 0b11));
        assert_eq!(
            asid_slot(PMAP_MAX_NASID - 2),
            (PMAP_MAX_NASID / 32 - 1, 0b11 << 30)
        );
    }

    #[test]
    fn the_tlbi_operands_drop_what_does_not_fit() {
        // The page number 0xf_ffff_f800_0123 keeps its low 44 bits.
        assert_eq!(tlbi_page(0xffff_ff80_0012_3000), 0xffff_8000_123);
        assert_eq!(tlbi_asid(0x1_0005), 5 << 48);
        assert_eq!(tlbi_asid(0x2_0005 | ASID_USER), (5 | ASID_USER) << 48);
    }

    #[test]
    fn the_probe_needs_the_cpu_and_the_tcr() {
        let has16 = ID_AA64MMFR0_ASID_BITS_16;
        assert_eq!(nasid_probe(has16, TCR_AS), AsidWidth::Bits16);
        assert_eq!(nasid_probe(has16, 0), AsidWidth::Bits8TcrAs);
        assert_eq!(nasid_probe(0, TCR_AS), AsidWidth::Bits8);
        assert_eq!(AsidWidth::Bits16.nasid(), 65536);
        assert_eq!(AsidWidth::Bits8TcrAs.nasid(), 256);
    }

    #[test]
    fn the_trampoline_ttbr_changes_the_base_and_bit_48_only() {
        // A realistic kernel root under ASID 0, then under a user pair's even ASID with CnP.
        for kernel in [0x4_0000_1000u64, (0x2a << 48) | 0x4_0000_1000 | 1] {
            let tramp = tramp_ttbr1(kernel);
            assert_eq!(
                tramp & TTBR_BADDR_MASK,
                (kernel & TTBR_BADDR_MASK) + PAGE_SIZE as u64
            );
            assert_eq!(tramp >> 48, (kernel >> 48) | ASID_USER, "{kernel:#x}");
            assert_eq!(tramp & 1, kernel & 1, "CnP");
            assert_eq!(kernel_ttbr1(tramp), kernel, "{kernel:#x}");
        }
        assert_eq!(tramp_ttbr1(0x4_0000_1000), 0x0001_0004_0000_2000);
    }

    #[test]
    fn the_trampoline_offset_is_an_add_immediate() {
        // A page multiple, encodable as `add`/`sub`'s 12-bit immediate, shifted by 12 or not.
        let off = TRAMP_ROOT_OFFSET as u64;
        assert!(off > 0 && off.is_multiple_of(PAGE_SIZE as u64));
        assert!(off < 1 << 12 || (off.is_multiple_of(1 << 12) && off < 1 << 24));
        // The kernel's table base is TTBR's BADDR: a page offset keeps it inside the mask.
        assert_eq!(off & !TTBR_BADDR_MASK, 0);
    }

    #[test]
    fn the_trampoline_ttbr_round_trips() {
        for base in [0x1000u64, 0x4_0000_1000, (1 << 48) - 2 * PAGE_SIZE as u64] {
            for asid in [0u64, 2, 0x100, 0xfffe] {
                let kernel = asid << 48 | base;
                assert_eq!(kernel_ttbr1(tramp_ttbr1(kernel)), kernel, "{kernel:#x}");
                let tramp = tramp_ttbr1(kernel);
                assert_eq!(tramp_ttbr1(kernel_ttbr1(tramp)), tramp, "{tramp:#x}");
            }
        }
    }

    #[test]
    fn the_tcr_fields() {
        let tcr = 0x0000_0035_b510_3510;
        assert_eq!(tcr_t1sz(tcr), 0x10);
        assert_eq!(tcr_t0sz(tcr, 39) & TCR_T0SZ_MASK, 25);
        assert_eq!(tcr_t0sz(tcr, 39) & !TCR_T0SZ_MASK, tcr & !TCR_T0SZ_MASK);
    }

    #[test]
    fn the_module_window_goes_after_the_last_entry() {
        // The image's gigabyte (`kernel.ld`'s `__kernel_base`).
        const GIB: usize = 0xffff_ffff_8000_0000;
        assert_eq!(MODULE_WINDOW_NPTP, 8);
        // boot(8)/efiboot: locore0's 32 blocks of 2 MiB (the 64 MB block).
        assert_eq!(module_window_place(GIB, 31), Some((32, GIB + (32 << 21))));
        // Limine: the image's own entries.
        assert_eq!(module_window_place(GIB, 10), Some((11, GIB + (11 << 21))));
        // The last that fits, and the first that does not.
        assert_eq!(
            module_window_place(GIB, 503),
            Some((504, GIB + (504 << 21)))
        );
        assert_eq!(module_window_place(GIB, 504), None);
        assert_eq!(module_window_place(GIB, 511), None);
        assert_eq!(module_window_place(GIB, 512), None);
        // The top gigabyte: the window's last byte is the address space's.
        const TOP: usize = 0xffff_ffff_c000_0000;
        assert_eq!(
            module_window_place(TOP, 503),
            Some((504, TOP + (504 << 21)))
        );
        // Never in the managed kernel range, never off a gigabyte.
        assert_eq!(module_window_place(VM_MIN_KERNEL_ADDRESS, 1), None);
        assert_eq!(
            module_window_place(VM_MIN_KERNEL_ADDRESS + (3 << 30), 1),
            None
        );
        assert_eq!(module_window_place(GIB + 4096, 1), None);
    }

    #[test]
    fn the_managed_range_has_its_own_level_1_slots() {
        // VM_MIN_KERNEL_ADDRESS..=VM_MAX_KERNEL_ADDRESS: 16 GiB, one GiB per slot: 0 to 15.
        for (l1, managed) in [
            (0, true),
            (15, true),
            (16, false),
            (510, false),
            (511, false),
        ] {
            assert_eq!(kernel_map_slot(l1), managed, "slot {l1}");
        }
        // The image's gigabyte (`kernel.ld`) and the boot(8) window's are not.
        assert!(!kernel_map_slot(vp_idx(1, 0xffff_ffff_8000_0000)));
        assert!(!kernel_map_slot(vp_idx(1, 0xffff_ffff_8400_0000)));
        assert!(kernel_map_slot(vp_idx(1, VM_MAX_KERNEL_ADDRESS)));
    }

    #[test]
    fn the_module_window_takes_w_xor_x() {
        use crate::sys::mman::PROT_READ;
        for (prot, ok) in [
            (PROT_READ, true),
            (PROT_READ | PROT_WRITE, true),
            (PROT_READ | PROT_EXEC, true),
            (PROT_EXEC, true),
            (PROT_WRITE, true),
            (PROT_NONE, false),
            (PROT_WRITE | PROT_EXEC, false),
            (PROT_READ | PROT_WRITE | PROT_EXEC, false),
            (0x08, false),
            (-1, false),
        ] {
            assert_eq!(module_prot_ok(prot), ok, "{prot:#x}");
        }
    }

    /// An image as both boot paths lay it out (`kernel.ld`): the text from the gigabyte's
    /// start, the read-only data and the data on 64 KiB boundaries.
    fn image() -> KernelImage {
        use crate::sys::types::Vaddr;
        KernelImage {
            start: Vaddr::new(0xffff_ffff_8000_0000),
            text_end: Vaddr::new(0xffff_ffff_80a1_2345),
            rodata_start: Vaddr::new(0xffff_ffff_80a2_0000),
            rodata_end: Vaddr::new(0xffff_ffff_80c4_3000),
            data_start: Vaddr::new(0xffff_ffff_80c5_0000),
            end: Vaddr::new(0xffff_ffff_8123_4568),
            phys_off: 0x4000_0000usize.wrapping_sub(0xffff_ffff_8000_0000),
        }
    }

    #[test]
    fn the_image_is_mapped_by_section() {
        let img = image();
        let rx = Some(PROT_READ | PROT_EXEC);
        let r = Some(PROT_READ);
        let rw = Some(PROT_READ | PROT_WRITE);
        for (va, prot) in [
            (0xffff_ffff_7fff_f000, None),
            (0xffff_ffff_8000_0000, rx),
            (0xffff_ffff_8000_0123, rx),
            // The page holding `_etext` is text; the padding after it is not mapped.
            (0xffff_ffff_80a1_2000, rx),
            (0xffff_ffff_80a1_3000, None),
            (0xffff_ffff_80a1_f000, None),
            (0xffff_ffff_80a2_0000, r),
            (0xffff_ffff_80c4_2fff, r),
            (0xffff_ffff_80c4_3000, None),
            (0xffff_ffff_80c4_f000, None),
            (0xffff_ffff_80c5_0000, rw),
            // The page holding `_end` is data; nothing after it.
            (0xffff_ffff_8123_4000, rw),
            (0xffff_ffff_8123_5000, None),
            (0xffff_ffff_8400_0000, None),
        ] {
            assert_eq!(image_page_prot(&img, va), prot, "{va:#x}");
        }
        assert!(image_layout_ok(&img));
        // 0xffffffff80000000 to 0xffffffff81234568: level-2 entries 0 to 9.
        assert_eq!(image_ptps(&img), 10);
    }

    #[test]
    fn the_image_layout_is_checked() {
        use crate::sys::types::Vaddr;
        let ok = image();
        let cases: [(&str, fn(&mut KernelImage)); 8] = [
            ("start off a page", |i| {
                i.start = Vaddr::new(0xffff_ffff_8000_0800)
            }),
            ("rodata off a page", |i| {
                i.rodata_start = Vaddr::new(0xffff_ffff_80a2_0010)
            }),
            ("text into rodata", |i| {
                i.text_end = Vaddr::new(0xffff_ffff_80a2_1000)
            }),
            ("rodata into data", |i| {
                i.rodata_end = Vaddr::new(0xffff_ffff_80c6_0000)
            }),
            ("no data", |i| i.end = Vaddr::new(0xffff_ffff_80c5_0000)),
            ("offset off a page", |i| {
                i.phys_off = i.phys_off.wrapping_add(8)
            }),
            ("past a gigabyte", |i| {
                i.end = Vaddr::new(0xffff_ffff_c000_0001)
            }),
            ("past 48 physical bits", |i| {
                i.phys_off = (1usize << 48).wrapping_sub(0xffff_ffff_8000_0000 + 0x1000)
            }),
        ];
        for (what, change) in cases {
            let mut img = ok;
            change(&mut img);
            assert!(!image_layout_ok(&img), "{what}");
        }
        // The top gigabyte's last page still fits.
        let mut top = ok;
        top.end = Vaddr::new(0xffff_ffff_c000_0000);
        assert!(image_layout_ok(&top));
        assert_eq!(image_ptps(&top), 512);
    }

    #[test]
    fn a_kernel_leaf_takes_its_sections_bits_and_no_guard() {
        use super::super::super::include::pte::{ATTR_GP, ATTR_IDX_MASK};
        let pa = 0x4012_3000u64;
        let rx = kernel_leaf(pa, PROT_READ | PROT_EXEC);
        let r = kernel_leaf(pa, PROT_READ);
        let rw = kernel_leaf(pa, PROT_READ | PROT_WRITE);
        // Text (and the identity page, and the module's text): executable at EL1 only,
        // read-only.
        assert_eq!(rx & (ATTR_PXN | ATTR_UXN), ATTR_UXN);
        assert_eq!(rx & attr_ap(3), attr_ap(2));
        // Read-only data: never executable, read-only.
        assert_eq!(r & (ATTR_PXN | ATTR_UXN), ATTR_PXN | ATTR_UXN);
        assert_eq!(r & attr_ap(3), attr_ap(2));
        // Data and BSS: never executable, writable at EL1, never at EL0.
        assert_eq!(rw & (ATTR_PXN | ATTR_UXN), ATTR_PXN | ATTR_UXN);
        assert_eq!(rw & attr_ap(3), attr_ap(0));
        for (prot, leaf) in [
            (PROT_READ | PROT_EXEC, rx),
            (PROT_READ, r),
            (PROT_READ | PROT_WRITE, rw),
        ] {
            assert_eq!(leaf & PTE_RPGN, pa, "{prot:#x}");
            assert_eq!(leaf & 0x3, L3_P, "{prot:#x}");
            assert_ne!(leaf & ATTR_AF, 0, "{prot:#x}");
            assert_ne!(leaf & ATTR_nG, 0, "{prot:#x}");
            assert_eq!(leaf & ATTR_IDX_MASK, attr_idx(PTE_ATTR_WB), "{prot:#x}");
            assert_eq!(leaf & attr_sh(3), attr_sh(SH_INNER), "{prot:#x}");
            // No branch-target guard on any kernel leaf (the window's included).
            assert_eq!(leaf & ATTR_GP, 0, "{prot:#x}");
        }
        // No access is no leaf; the flag bits of the address do not leak in.
        assert_eq!(kernel_leaf(pa, PROT_NONE), 0);
        assert_eq!(kernel_leaf(pa | 0xfff, PROT_READ) & PTE_RPGN, pa);
        // `ap_bits_kern` grants write and execute together, as the C's: W^X is the callers'
        // (the module window refuses it, the kernel pmap refuses any `PROT_EXEC`, the image
        // asks for one of the three above).
        assert_eq!(
            kernel_leaf(pa, PROT_READ | PROT_WRITE | PROT_EXEC) & (ATTR_PXN | attr_ap(2)),
            0
        );
    }

    #[test]
    // The constants only: `init_lower_half` writes TCR in `asm!`, which a host test cannot
    // run; the arm64 smokes prove the write.
    fn tcr_hpd_names_the_two_hierarchical_disable_bits() {
        assert_eq!(TCR_HPD, 0x0000_0600_0000_0000);
        assert_eq!(TCR_HPD & (TCR_TTBR0_FIELDS | TCR_A1 | TCR_AS), 0);
    }

    #[test]
    fn the_identity_walk_replaces_the_lower_half_fields_only() {
        let tcr = 0x0000_0035_b510_3519 | (1 << 7);
        let id = tcr_identity(tcr);
        assert_eq!(id & TCR_TTBR0_FIELDS, 0x3510);
        assert_eq!(id & !TCR_TTBR0_FIELDS, tcr & !TCR_TTBR0_FIELDS);
        assert_eq!(id & TCR_T0SZ_MASK, 16);
        assert_eq!(tcr_t1sz(id), tcr_t1sz(tcr));
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/arm64/include/pmap.h");
        let ours: &[(&str, i64)] = &[
            ("VP_IDX0_CNT", VP_IDX0_CNT as i64),
            ("VP_IDX0_POS", i64::from(VP_IDX0_POS)),
            ("VP_IDX1_CNT", VP_IDX1_CNT as i64),
            ("VP_IDX1_POS", i64::from(VP_IDX1_POS)),
            ("VP_IDX2_CNT", VP_IDX2_CNT as i64),
            ("VP_IDX2_POS", i64::from(VP_IDX2_POS)),
            ("VP_IDX3_CNT", VP_IDX3_CNT as i64),
            ("VP_IDX3_POS", i64::from(VP_IDX3_POS)),
            ("PMAP_NOCACHE", i64::from(PMAP_NOCACHE)),
            ("PMAP_DEVICE", i64::from(PMAP_DEVICE)),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
