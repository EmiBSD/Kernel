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

use super::super::include::param::{PAGE_MASK, PAGE_SHIFT};
use super::super::include::vmparam::{VM_MAX_KERNEL_ADDRESS, VM_MIN_KERNEL_ADDRESS};
use crate::pmap::{PG_PMAP2, PMAP_MD0, PMAP_MD1, PMAP_MD2, PMAP_MD3, VmProt};
use crate::sys::mman::{PROT_EXEC, PROT_NONE, PROT_WRITE};

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
/// `TTBRx_EL1.BADDR`: bits 47:1 hold the table's physical address.
pub const TTBR_BADDR_MASK: u64 = 0x0000_ffff_ffff_fffe;
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
        // VM_MIN_KERNEL_ADDRESS..=VM_MAX_KERNEL_ADDRESS: 16 GiB, slots 0 to 3.
        for (l1, managed) in [(0, true), (3, true), (4, false), (510, false), (511, false)] {
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
