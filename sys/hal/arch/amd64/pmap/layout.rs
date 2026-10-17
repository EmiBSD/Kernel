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

/*
 * Copyright (c) 2001 Wasabi Systems, Inc.
 * All rights reserved.
 *
 * Written by Frank van der Linden for Wasabi Systems, Inc.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed for the NetBSD Project by
 *      Wasabi Systems, Inc.
 * 4. The name of Wasabi Systems, Inc. may not be used to endorse
 *    or promote products derived from this software without specific prior
 *    written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY WASABI SYSTEMS, INC. ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL WASABI SYSTEMS, INC
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
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

/*
 * Copyright 2001 (c) Wasabi Systems, Inc.
 * All rights reserved.
 *
 * Written by Frank van der Linden for Wasabi Systems, Inc.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed for the NetBSD Project by
 *      Wasabi Systems, Inc.
 * 4. The name of Wasabi Systems, Inc. may not be used to endorse
 *    or promote products derived from this software without specific prior
 *    written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY WASABI SYSTEMS, INC. ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL WASABI SYSTEMS, INC
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! amd64 `<machine/pmap.h>`'s layout: the PML4 slots, the page-table geometry, the recursive
//! mapping's addresses, the PTE bits the pmap adds and the composition of a leaf.
//!
//! Upstream: sys/arch/amd64/include/pmap.h @ 3ce1f3f79392
//! Upstream: sys/arch/amd64/amd64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/amd64/include/pmap.rs@3c62ede68802
//! LZ: sys/arch/amd64/amd64/pmap.rs@3c62ede68802
//!
//! Pure constants and `const fn`s, so the host tests compile this file by `#[path]`
//! (`sys/hal/arch/host/mod.rs`, D15): its two neighbours are reached as
//! `super::super::include`, the same path on the machine and on the host.
//!
//! ## Deviations
//! - `pm_pdir_intel` (the Meltdown PML4), PCID and the nested (EPT, RVI) pmaps are not
//!   configured: their constants are here, as the header's, and nothing uses them.
//!
//! ## Redesign
//! - LZ's `include/pmap.rs` holds this layout with `struct pmap`, `struct pv_entry`,
//!   `struct vm_page_md` and the inline frontends; those are `hal::pmap`'s and the backend's
//!   (`backend.rs`) now, and this file keeps the layout alone.
//! - `kvtopte` (a pointer through the recursive mapping) and `ptp_va2o` (the offset of a PTP
//!   in `pm_obj`) are gone: the backend walks the tables through the direct map and the walk
//!   is the PTPs' index. The recursive slot stays installed (`locore0.S`'s protocol) and its
//!   addresses (`PTE_BASE`, `L2_BASE`, ...) stay, for the TLB entries they name.
//! - `protection_codes[]`, which `pmap_bootstrap` filled once, is [`protection_code`], a
//!   `const fn` of the protection and the machine's `pg_nx`/`pg_xo`; `pl_pi` (the index inside
//!   one table, the C's `pl1_pi`..`pl4_pi` by level) comes from the C file.

use super::super::include::param::{PAGE_MASK, PAGE_SIZE};
use super::super::include::pte::{
    L1_FRAME, L1_MASK, L1_SHIFT, L2_FRAME, L2_MASK, L2_SHIFT, L3_FRAME, L3_MASK, L3_SHIFT,
    L4_FRAME, L4_MASK, L4_SHIFT, NBPD_L1, NBPD_L2, NBPD_L3, NBPD_L4, PG_AVAIL1, PG_AVAIL2, PG_RO,
    PG_RW, PG_V, PdEntry,
};
use super::super::include::vmparam::{VM_MAX_KERNEL_ADDRESS, VM_MIN_KERNEL_ADDRESS};
use crate::pmap::{PG_PMAP2, PMAP_MD0, PMAP_MD1, VmProt};

/// `VA_SIGN_MASK`: the sign-extension bits of a canonical address.
pub const VA_SIGN_MASK: usize = 0xffff_0000_0000_0000;

/// `VA_SIGN_NEG(va)`: sign-extends a kernel virtual address.
pub const fn va_sign_neg(va: usize) -> usize {
    va | VA_SIGN_MASK
}

/// `VA_SIGN_POS(va)`: strips the sign extension.
pub const fn va_sign_pos(va: usize) -> usize {
    va & !VA_SIGN_MASK
}

/// `L4_SLOT_PTE`: the PML4 slot of the recursive mapping.
pub const L4_SLOT_PTE: usize = 255;
/// `L4_SLOT_KERN`: the first PML4 slot of kernel virtual space.
pub const L4_SLOT_KERN: usize = 256;
/// `L4_SLOT_KERNBASE`: the PML4 slot of the kernel image.
pub const L4_SLOT_KERNBASE: usize = 511;

/// `DIRECT_MAP_PML4_SLOTS`: PML4 slots the direct map takes.
pub const DIRECT_MAP_PML4_SLOTS: usize = 4;
/// `DIRECT_MAP_START_CHOICES`: how many PML4 slots the direct map may start at.
pub const DIRECT_MAP_START_CHOICES: usize = 32;
/// `DIRECT_MAP_START_MASK`.
pub const DIRECT_MAP_START_MASK: usize = DIRECT_MAP_START_CHOICES - 1;
/// `DIRECT_MAP_RESERVED_PML4_SLOTS`.
pub const DIRECT_MAP_RESERVED_PML4_SLOTS: usize =
    DIRECT_MAP_PML4_SLOTS + DIRECT_MAP_START_CHOICES - 1;
/// `DIRECT_MAP_SIZE`: bytes the direct map covers.
pub const DIRECT_MAP_SIZE: usize = DIRECT_MAP_PML4_SLOTS * NBPD_L4;
/// `L4_SLOT_DIRECT`: the lowest PML4 slot of the direct map.
pub const L4_SLOT_DIRECT: usize = L4_SLOT_KERNBASE - DIRECT_MAP_RESERVED_PML4_SLOTS;
/// `L4_SLOT_EARLY`: the PML4 slot of the early mapping.
pub const L4_SLOT_EARLY: usize = L4_SLOT_DIRECT - 1;

/// `PDIR_SLOT_KERN`.
pub const PDIR_SLOT_KERN: usize = L4_SLOT_KERN;
/// `PDIR_SLOT_PTE`.
pub const PDIR_SLOT_PTE: usize = L4_SLOT_PTE;
/// `PDIR_SLOT_DIRECT`.
pub const PDIR_SLOT_DIRECT: usize = L4_SLOT_DIRECT;
/// `PDIR_SLOT_EARLY`.
pub const PDIR_SLOT_EARLY: usize = L4_SLOT_EARLY;

// The virtual addresses of the page tables and the page directory through the recursive
// mapping in slot L4_SLOT_PTE: PTE_BASE is the level-1 table of every address, L2_BASE the
// level-2 tables, and so on up to the PML4 itself at L4_BASE. Nothing reads through them; the
// TLB shootdown of a freed table names its page there, as the C does.

/// `PTE_BASE`: where every level-1 table is visible.
pub const PTE_BASE: usize = L4_SLOT_PTE * NBPD_L4;
/// `L1_BASE`.
pub const L1_BASE: usize = PTE_BASE;
/// `L2_BASE`: the level-2 tables.
pub const L2_BASE: usize = L1_BASE + L4_SLOT_PTE * NBPD_L3;
/// `L3_BASE`: the level-3 tables.
pub const L3_BASE: usize = L2_BASE + L4_SLOT_PTE * NBPD_L2;
/// `L4_BASE`: the PML4.
pub const L4_BASE: usize = L3_BASE + L4_SLOT_PTE * NBPD_L1;
/// `PDP_PDE`: the recursive entry itself.
pub const PDP_PDE: usize = L4_BASE + L4_SLOT_PTE * size_of::<PdEntry>();
/// `PDP_BASE`.
pub const PDP_BASE: usize = L4_BASE;

/// `pl1_pi(VA)`: the index inside the level-1 table.
pub const fn pl1_pi(va: usize) -> usize {
    (va_sign_pos(va) & L1_MASK) >> L1_SHIFT
}

/// `pl2_pi(VA)`.
pub const fn pl2_pi(va: usize) -> usize {
    (va_sign_pos(va) & L2_MASK) >> L2_SHIFT
}

/// `pl3_pi(VA)`.
pub const fn pl3_pi(va: usize) -> usize {
    (va_sign_pos(va) & L3_MASK) >> L3_SHIFT
}

/// `pl4_pi(VA)`.
pub const fn pl4_pi(va: usize) -> usize {
    (va_sign_pos(va) & L4_MASK) >> L4_SHIFT
}

/// `pl1_i(VA)`: the index of `va`'s level-1 entry counted from `PTE_BASE`.
pub const fn pl1_i(va: usize) -> usize {
    (va_sign_pos(va) & L1_FRAME) >> L1_SHIFT
}

/// `pl2_i(VA)`.
pub const fn pl2_i(va: usize) -> usize {
    (va_sign_pos(va) & L2_FRAME) >> L2_SHIFT
}

/// `pl3_i(VA)`.
pub const fn pl3_i(va: usize) -> usize {
    (va_sign_pos(va) & L3_FRAME) >> L3_SHIFT
}

/// `pl4_i(VA)`.
pub const fn pl4_i(va: usize) -> usize {
    (va_sign_pos(va) & L4_FRAME) >> L4_SHIFT
}

/// `PTP_MASK_INITIALIZER`: `ptp_masks[]`.
pub const PTP_MASKS: [usize; PTP_LEVELS] = [L1_FRAME, L2_FRAME, L3_FRAME, L4_FRAME];
/// `PTP_SHIFT_INITIALIZER`: `ptp_shifts[]`.
pub const PTP_SHIFTS: [u32; PTP_LEVELS] = [L1_SHIFT, L2_SHIFT, L3_SHIFT, L4_SHIFT];
/// `NKPTP_INITIALIZER`: `nkptp[]` at boot.
pub const NKPTP_INITIALIZER: [usize; PTP_LEVELS] = [
    NKL1_START_ENTRIES,
    NKL2_START_ENTRIES,
    NKL3_START_ENTRIES,
    NKL4_START_ENTRIES,
];
/// `NKPTPMAX_INITIALIZER`: `nkptpmax[]`.
pub const NKPTPMAX_INITIALIZER: [usize; PTP_LEVELS] = [
    NKL1_MAX_ENTRIES,
    NKL2_MAX_ENTRIES,
    NKL3_MAX_ENTRIES,
    NKL4_MAX_ENTRIES,
];
/// `NBPD_INITIALIZER`: `nbpd[]`.
pub const NBPD_INITIALIZER: [usize; PTP_LEVELS] = [NBPD_L1, NBPD_L2, NBPD_L3, NBPD_L4];
/// `PDES_INITIALIZER`: `normal_pdes[]`, the level 2, 3 and 4 tables through the recursive map.
pub const PDES_INITIALIZER: [usize; PTP_LEVELS - 1] = [L2_BASE, L3_BASE, L4_BASE];

/// `pl_i(va, lvl)`: the index of `va`'s level-`lvl` entry counted from that level's base.
pub const fn pl_i(va: usize, lvl: usize) -> usize {
    (va_sign_pos(va) & PTP_MASKS[lvl - 1]) >> PTP_SHIFTS[lvl - 1]
}

/// `pl<lvl>_pi(va)`: the index of `va`'s entry within one level-`lvl` table (`pl_i` counts
/// across the whole level).
pub const fn pl_pi(va: usize, lvl: usize) -> usize {
    pl_i(va, lvl) & (NTOPLEVEL_PDES - 1)
}

// NK*_MAX_ENTRIES: the maximum number of PTPs per level the kernel may use. NK*_KIMG_ENTRIES
// and ND*_ENTRIES: how many the kernel image and the direct map take at boot. NK*_START_ENTRIES:
// how many are there to begin with.

/// `NKL4_MAX_ENTRIES`.
pub const NKL4_MAX_ENTRIES: usize = 1;
/// `NKL3_MAX_ENTRIES`.
pub const NKL3_MAX_ENTRIES: usize = NKL4_MAX_ENTRIES * 512;
/// `NKL2_MAX_ENTRIES`.
pub const NKL2_MAX_ENTRIES: usize = NKL3_MAX_ENTRIES * 512;
/// `NKL1_MAX_ENTRIES`.
pub const NKL1_MAX_ENTRIES: usize = NKL2_MAX_ENTRIES * 512;

/// `NKL4_KIMG_ENTRIES`.
pub const NKL4_KIMG_ENTRIES: usize = 1;
/// `NKL3_KIMG_ENTRIES`.
pub const NKL3_KIMG_ENTRIES: usize = 1;
/// `NKL2_KIMG_ENTRIES`.
pub const NKL2_KIMG_ENTRIES: usize = 64;

/// `NDML4_ENTRIES`.
pub const NDML4_ENTRIES: usize = 1;
/// `NDML3_ENTRIES`.
pub const NDML3_ENTRIES: usize = 1;
/// `NDML2_ENTRIES`: 4GB.
pub const NDML2_ENTRIES: usize = 4;

/// `NKL4_START_ENTRIES`.
pub const NKL4_START_ENTRIES: usize = 0;
/// `NKL3_START_ENTRIES`.
pub const NKL3_START_ENTRIES: usize = 0;
/// `NKL2_START_ENTRIES`.
pub const NKL2_START_ENTRIES: usize = 0;
/// `NKL1_START_ENTRIES`: XXX.
pub const NKL1_START_ENTRIES: usize = 0;

/// `NTOPLEVEL_PDES`: entries in the top-level table.
pub const NTOPLEVEL_PDES: usize = PAGE_SIZE / size_of::<PdEntry>();
/// `NPDPG`: entries in a page of page-table entries.
pub const NPDPG: usize = PAGE_SIZE / size_of::<PdEntry>();

/// `PTP_LEVELS`: how many levels of page tables.
pub const PTP_LEVELS: usize = 4;

/// `PG_W`: "wired" mapping.
pub const PG_W: u64 = PG_AVAIL1;
/// `PG_PVLIST`: mapping has entry on pvlist.
pub const PG_PVLIST: u64 = PG_AVAIL2;

/// `PCID_KERN`: for `pmap_kernel()`.
pub const PCID_KERN: u64 = 0;
/// `PCID_PROC`: non-`pmap_kernel()`, U+K.
pub const PCID_PROC: u64 = 1;
/// `PCID_PROC_INTEL`: non-`pmap_kernel()`, U-K (meltdown).
pub const PCID_PROC_INTEL: u64 = 2;
/// `PCID_TEMP`: temp mapping of another non-`pmap_kernel()`.
pub const PCID_TEMP: u64 = 3;
/// `PCID_EFI`: EFI runtime services.
pub const PCID_EFI: u64 = 4;

/// `NPTECL`: number of PTEs per cache line.
pub const NPTECL: usize = 8;

/// `PMAP_TYPE_NORMAL`: a plain pmap.
pub const PMAP_TYPE_NORMAL: i32 = 1;
/// `PMAP_TYPE_EPT`: an Intel nested pmap (vmm).
pub const PMAP_TYPE_EPT: i32 = 2;
/// `PMAP_TYPE_RVI`: an AMD nested pmap (vmm).
pub const PMAP_TYPE_RVI: i32 = 3;

/// `PMAP_EFI`: an EFI runtime mapping.
pub const PMAP_EFI: i32 = PMAP_MD0;
/// `PMAP_NOCRYPT`: no memory encryption.
pub const PMAP_NOCRYPT: i32 = PMAP_MD1;

/// `PMAP_PA_MASK`: to remove the flags.
pub const PMAP_PA_MASK: usize = !PAGE_MASK;
/// `PMAP_NOCACHE`: set the non-cacheable bit.
pub const PMAP_NOCACHE: i32 = 0x1;
/// `PMAP_WC`: set page write combining.
pub const PMAP_WC: i32 = 0x2;

/// `PG_PMAP_WC`: the page is mapped write-combining (`PG_PMAP_MOD` and `PG_PMAP_REF` are
/// `hal::pmap`'s, the same on every machine).
pub const PG_PMAP_WC: u32 = PG_PMAP2;

/// `pmap_valid_entry(E)`: is PDE or PTE valid?
pub const fn pmap_valid_entry(e: PdEntry) -> bool {
    e & PG_V != 0
}

/// `protection_codes[prot & 7]`: the PTE protection bits of a combination of `PROT_READ`
/// (1), `PROT_WRITE` (2) and `PROT_EXEC` (4), as `pmap_bootstrap` fills the C's table:
/// non-executable (`pg_nx`) unless `PROT_EXEC`, writable when `PROT_WRITE`, execute-only
/// (`pg_xo`, the PKU key) for `PROT_EXEC` alone.
pub const fn protection_code(prot: VmProt, pg_nx: u64, pg_xo: u64) -> u64 {
    match prot & 7 {
        0 => pg_nx,         // ---
        1 => PG_RO | pg_nx, // -r-
        2 => PG_RW | pg_nx, // w--
        3 => PG_RW | pg_nx, // wr-
        4 => pg_xo,         // --x
        5 => PG_RO,         // -rx
        _ => PG_RW,         // w-x, wrx
    }
}

/// The managed kernel window `[virtual_avail, end)`: `pmap_bootstrap`'s `virtual_avail` and
/// the end `pmap_virtual_space` reports, for a direct map `[direct_base, direct_end)`.
///
/// The C's window is `[VM_MIN_KERNEL_ADDRESS, VM_MAX_KERNEL_ADDRESS)`, PML4 slot
/// `L4_SLOT_KERN`, and so is this one while the direct map lies elsewhere (`locore0.S`'s
/// `L4_SLOT_DIRECT`). When the direct map overlaps that slot (Limine's HHDM starts there),
/// the window is the first whole PML4 slot after the direct map's end
/// (`roundup(direct_end, NBPD_L4)`): always exactly the C's size (one slot, asserted below,
/// which `VM_KERNEL_SPACE_SIZE` and the sizes derived from it assume), never a second
/// top-level entry (`NKL4_MAX_ENTRIES`), never over the direct map. `None` when no window is
/// left: an empty or wrapping direct map, or a slot outside the kernel slots before
/// `KERNBASE`'s (the recursive slot 255 below them; slot 511, the image and the module
/// window, above).
pub const fn kernel_window(direct_base: usize, direct_end: usize) -> Option<(usize, usize)> {
    if direct_end <= direct_base {
        return None;
    }
    let start = if direct_base < VM_MAX_KERNEL_ADDRESS && direct_end > VM_MIN_KERNEL_ADDRESS {
        // roundup(direct_end, NBPD_L4); a wrap to 0 lands below the kernel slots, refused.
        let down = direct_end & !(NBPD_L4 - 1);
        if down == direct_end {
            down
        } else {
            down.wrapping_add(NBPD_L4)
        }
    } else {
        VM_MIN_KERNEL_ADDRESS
    };
    let slot = pl_i(start, PTP_LEVELS);
    if slot < L4_SLOT_KERN || slot >= L4_SLOT_KERNBASE || slot == L4_SLOT_PTE {
        return None;
    }
    let end = start + NBPD_L4;
    if direct_base < end && start < direct_end {
        return None;
    }
    Some((start, end))
}

// The C's kernel window is exactly one PML4 slot, and so is every window [`kernel_window`]
// returns.
const _: () = assert!(VM_MAX_KERNEL_ADDRESS - VM_MIN_KERNEL_ADDRESS == NBPD_L4);
const _: () = assert!(pl_i(VM_MIN_KERNEL_ADDRESS, PTP_LEVELS) == L4_SLOT_KERN);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::super::super::include::pte::PG_NX;
    use super::*;

    #[test]
    fn slot_layout() {
        assert_eq!(L4_SLOT_DIRECT, 511 - 35);
        assert_eq!(L4_SLOT_EARLY, L4_SLOT_DIRECT - 1);
        assert_eq!(DIRECT_MAP_SIZE, 4 << 39);
        assert_eq!(NTOPLEVEL_PDES, 512);
        assert!(DIRECT_MAP_START_CHOICES & DIRECT_MAP_START_MASK == 0);
    }

    #[test]
    fn recursive_mapping_indices() {
        // The PML4 maps itself in slot 255: its own address is L4_BASE + 255 * 8.
        assert_eq!(PTE_BASE, 0x0000_7f80_0000_0000);
        assert_eq!(
            L4_BASE,
            PTE_BASE + 255 * NBPD_L3 + 255 * NBPD_L2 + 255 * NBPD_L1
        );
        assert_eq!(PDP_PDE, L4_BASE + 255 * 8);
        let va = 0xffff_8000_0040_1000;
        assert_eq!(pl4_pi(va), 256);
        assert_eq!(pl3_pi(va), 0);
        assert_eq!(pl2_pi(va), 2);
        assert_eq!(pl1_pi(va), 1);
        assert_eq!(pl1_i(va), (256 << 27) | (2 << 9) | 1);
        assert_eq!(pl_i(va, 4), pl4_i(va));
        assert_eq!(pl4_i(va), 256);
        for lvl in 1..=PTP_LEVELS {
            assert!(pl_pi(va, lvl) < NTOPLEVEL_PDES);
        }
        assert_eq!(pl_pi(va, 1), pl1_pi(va));
        assert_eq!(pl_pi(va, 2), pl2_pi(va));
        assert_eq!(pl_pi(va, 4), pl4_pi(va));
    }

    #[test]
    fn protection_codes_are_the_c_table() {
        let codes: [u64; 8] = core::array::from_fn(|p| protection_code(p as VmProt, PG_NX, 0));
        assert_eq!(
            codes,
            [
                PG_NX,
                PG_RO | PG_NX,
                PG_RW | PG_NX,
                PG_RW | PG_NX,
                0,
                PG_RO,
                PG_RW,
                PG_RW
            ]
        );
        // Without NX the machine cannot refuse execution: no code carries the bit.
        assert!((0..8).all(|p| protection_code(p, 0, 0) & PG_NX == 0));
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/amd64/include/pmap.h");
        let ours: &[(&str, i64)] = &[
            ("L4_SLOT_PTE", L4_SLOT_PTE as i64),
            ("L4_SLOT_KERN", L4_SLOT_KERN as i64),
            ("L4_SLOT_KERNBASE", L4_SLOT_KERNBASE as i64),
            ("DIRECT_MAP_PML4_SLOTS", DIRECT_MAP_PML4_SLOTS as i64),
            ("DIRECT_MAP_START_CHOICES", DIRECT_MAP_START_CHOICES as i64),
            ("NKL2_KIMG_ENTRIES", NKL2_KIMG_ENTRIES as i64),
            ("NDML2_ENTRIES", NDML2_ENTRIES as i64),
            ("PTP_LEVELS", PTP_LEVELS as i64),
            ("PCID_KERN", PCID_KERN as i64),
            ("PCID_PROC", PCID_PROC as i64),
            ("PCID_PROC_INTEL", PCID_PROC_INTEL as i64),
            ("PCID_TEMP", PCID_TEMP as i64),
            ("PCID_EFI", PCID_EFI as i64),
            ("NPTECL", NPTECL as i64),
            ("PMAP_TYPE_NORMAL", i64::from(PMAP_TYPE_NORMAL)),
            ("PMAP_TYPE_EPT", i64::from(PMAP_TYPE_EPT)),
            ("PMAP_TYPE_RVI", i64::from(PMAP_TYPE_RVI)),
            ("PMAP_NOCACHE", i64::from(PMAP_NOCACHE)),
            ("PMAP_WC", i64::from(PMAP_WC)),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }

    #[test]
    fn the_kernel_window_is_one_slot_after_the_direct_map() {
        const SLOT: usize = NBPD_L4;
        let kern = VM_MIN_KERNEL_ADDRESS;
        // (direct_base, direct_end) -> the window.
        let cases: [((usize, usize), Option<(usize, usize)>); 13] = [
            // boot(8): locore0.S's direct map at L4_SLOT_DIRECT, the C's window.
            (
                (
                    va_sign_neg(L4_SLOT_DIRECT * SLOT),
                    va_sign_neg(L4_SLOT_DIRECT * SLOT) + (4 << 30),
                ),
                Some((kern, VM_MAX_KERNEL_ADDRESS)),
            ),
            // Limine, 4 GiB of memory: the HHDM ends inside slot 256, the window is 257.
            (
                (kern, kern + (4 << 30)),
                Some((kern + SLOT, kern + 2 * SLOT)),
            ),
            // Limine, memory ending at 511 GiB (QEMU -m 509G): a 1 GiB rest of slot 256
            // would starve kmeminit; the window is the whole of 257.
            (
                (kern, kern + (511 << 30)),
                Some((kern + SLOT, kern + 2 * SLOT)),
            ),
            // A direct map ending exactly at a slot boundary: the next slot, whole.
            ((kern, kern + SLOT), Some((kern + SLOT, kern + 2 * SLOT))),
            // Limine, QEMU's map reaching 1 TiB: the HHDM ends where slot 258 starts, and the
            // window is that whole slot (the N3 integration's boot, inverted by 4b's clamp).
            (
                (kern, 0xffff_8100_0000_0000),
                Some((0xffff_8100_0000_0000, 0xffff_8180_0000_0000)),
            ),
            // An HHDM ending inside slot 257: the window is 258.
            (
                (kern, kern + SLOT + (8 << 30)),
                Some((kern + 2 * SLOT, kern + 3 * SLOT)),
            ),
            // A direct map starting above slot 256 leaves the C's window whole.
            (
                (kern + SLOT, kern + 2 * SLOT),
                Some((kern, VM_MAX_KERNEL_ADDRESS)),
            ),
            // A direct map inside slot 256, not at its start: the window is the next slot.
            (
                (kern + (1 << 30), kern + (5 << 30)),
                Some((kern + SLOT, kern + 2 * SLOT)),
            ),
            // A direct map up to KERNBASE's slot, or ending inside slot 510 (rounded up to
            // 511), leaves nothing.
            ((kern, va_sign_neg(L4_SLOT_KERNBASE * SLOT)), None),
            ((kern, va_sign_neg(510 * SLOT) + (1 << 30)), None),
            // A direct map to the top of the address space wraps the round-up: refused.
            ((kern, usize::MAX), None),
            // Empty and wrapping direct maps are refused.
            ((kern, kern), None),
            ((kern + SLOT, 0), None),
        ];
        for ((base, end), want) in cases {
            let got = kernel_window(base, end);
            assert_eq!(got, want, "direct map [{base:#x}, {end:#x})");
            if let Some((s, e)) = got {
                assert_eq!(s % SLOT, 0, "slot-aligned");
                assert_eq!(
                    e - s,
                    VM_MAX_KERNEL_ADDRESS - VM_MIN_KERNEL_ADDRESS,
                    "the C's size"
                );
                assert!(!(base < e && s < end), "no overlap with the direct map");
                let slot = pl_i(s, PTP_LEVELS);
                assert!(
                    (L4_SLOT_KERN..L4_SLOT_KERNBASE).contains(&slot),
                    "a kernel slot below KERNBASE's"
                );
            }
        }
    }
}
/* </TESTS> */
