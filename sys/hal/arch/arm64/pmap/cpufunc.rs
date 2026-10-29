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

/*-
 * Copyright (c) 2014 Robin Randhawa
 * Copyright (c) 2015 The FreeBSD Foundation
 * All rights reserved.
 *
 * Portions of this software were developed by Andrew Turner
 * under sponsorship from the FreeBSD Foundation
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*-
 * Copyright (c) 2012-2014 Andrew Turner
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: head/sys/arm64/arm64/locore.S 282867 2015-05-13 18:57:03Z zbb $
 */
/* </LICENSES> */

/* <CODE> */
//! arm64's TLB and cache maintenance as the pmap needs it: `cpufunc_asm.S`'s `cpu_setttb`,
//! the four `cpu_tlb_flush*`, `cpu_dcache_wbinv_range`, `cpu_idcache_wbinv_range` and
//! `cpu_icache_sync_range`, and `pmap.c`'s write of the kernel's empty table to `TTBR0_EL1`,
//! as private `asm!` of the backend.
//!
//! Upstream: sys/arch/arm64/arm64/cpufunc_asm.S @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/pmap.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/locore.S @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/cpufunc.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/pmap.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/locore.rs@3c62ede68802
//!
//! Every TLB invalidation is the inner-shareable broadcast (`tlbi ...is`): arm64 sends no IPI
//! for a shootdown, so the pmap registers no `TlbCpus` bridge. The barriers are the C's: a
//! `dsb ishst` so the table writes reach the walkers first, the `tlbi`, a `dsb ish` so every
//! CPU has done it, an `isb`.
//!
//! ## Deviations
//! - The `CPTAG_REPEAT_TLBI` code patch (a second `tlbi` on cores with that erratum) is not
//!   applied, as in LZ: `codepatch` arrives with the CPU code (N4).
//! - The cache line sizes are read from `CTR_EL0` inside each operation, as LZ does; the C
//!   loads the `dcache_line_size`/`idcache_line_size` that `cpu.c` probed from it at boot.
//!
//! ## Redesign
//! - Split from `bsd`'s `cpufunc.rs`, which keeps its own copies for `bus_dma`, `efi_machdep`
//!   and `cpu.c` until N4; there each was an `asm!` per instruction with its loop in Rust,
//!   here a whole `cache_handle_range` (the C's macro: the line loop of `dc`, then of `ic`)
//!   is one block.
//! - `cpu_setttb(asid, pt0pa)` takes the pmap ([`cpu_setttb`]): the ASID and the table come
//!   from its machine part, whose invariant is the soundness argument, instead of from two
//!   numbers any caller could pass.
//! - N3.1 commit 8: `locore.S`'s `switch_mmu_kernel`, which `pmap_bootstrap` calls at its
//!   physical address (`switch_mmu_kernel_table`, `pmap.c`), is `hal_switch_mmu_kernel`, a
//!   position-independent routine of its own section; the backend calls it the same way, at
//!   its physical address through an identity table it builds (`backend.rs`,
//!   `switch_mmu_kernel_table`), and finds it through [`switch_routine_va`]. RETGUARD and the
//!   `CPTAG_REPEAT_TLBI` code patch are left out, as everywhere in this kernel.

use core::arch::{asm, global_asm};
use core::ptr;

use crate::assert::hal_panic;
use crate::pmap::Pmap;

// `switch_mmu_kernel` (`locore.S`): loads `TTBR1_EL1` with `x0` and drops every TLB entry of
// every CPU (the broadcast `tlbi vmalle1is`, completed by `dsb ish`), then returns. Called
// only at its physical address through the backend's identity table, so none of its
// instructions is fetched through `TTBR1_EL1` while that register changes (the Arm ARM's
// break-before-make for the kernel's own global entries, which the switch to the kernel's
// tables changes in size and attributes). Position-independent: no literal, no absolute
// address, no stack; eight instructions after a 64-byte alignment, so the routine never
// crosses a page and one identity page maps it.
global_asm!(
    ".pushsection .text.hal_switch_mmu_kernel, \"ax\", %progbits",
    ".balign 64",
    ".globl hal_switch_mmu_kernel",
    ".hidden hal_switch_mmu_kernel",
    ".type hal_switch_mmu_kernel, %function",
    "hal_switch_mmu_kernel:",
    "dsb sy",
    "msr ttbr1_el1, x0",
    "isb",
    "dsb ishst",
    "tlbi vmalle1is",
    "dsb ish",
    "isb",
    "ret",
    ".size hal_switch_mmu_kernel, . - hal_switch_mmu_kernel",
    ".popsection",
);

/// The routine's length in bytes: its eight instruction words.
pub(super) const SWITCH_ROUTINE_LEN: usize = size_of::<[u32; 8]>();

// SAFETY: the one item names the routine above, defined in this crate by its symbol, as its
// eight instruction words (`.size` says the same). They lie in the kernel text, which both
// boot paths and the kernel's own tables map readable and nothing ever writes, so a shared
// view of them aliases no write; the kernel only takes its address (`switch_routine_va`).
unsafe extern "C" {
    /// The instructions of `hal_switch_mmu_kernel`.
    #[link_name = "hal_switch_mmu_kernel"]
    safe static HAL_SWITCH_MMU_KERNEL: [u32; 8];
}

/// The virtual address of `hal_switch_mmu_kernel` in the kernel text.
pub(super) fn switch_routine_va() -> usize {
    ptr::addr_of!(HAL_SWITCH_MMU_KERNEL).addr()
}

/// `READ_SPECIALREG(ttbr1_el1)`: the running CPU's `TTBR1_EL1` (its table base, ASID and
/// `CnP`).
pub(super) fn cpu_ttbr1() -> u64 {
    let ttbr1: u64;
    // SAFETY: reading a system register has no side effect and touches no memory;
    // `TTBR1_EL1` is readable at EL1.
    unsafe {
        asm!(
            "mrs {}, ttbr1_el1",
            out(reg) ttbr1,
            options(nomem, nostack, preserves_flags),
        );
    };
    ttbr1
}

/// `cache_handle_range dcop[, ic, icop]`: one cache operation over every line of
/// `[va, va + len)`, then `dsb ish`; with an instruction-cache operation, the same lines
/// through it, then `dsb ish; isb`. The line is `CTR_EL0.DminLine` for the data cache alone,
/// the smaller of `DminLine` and `IminLine` with the instruction cache (`idcache_line_size`).
macro_rules! cache_handle_range {
    ($va:expr, $len:expr, $dcop:literal) => {{
        let (va, len): (usize, usize) = ($va, $len);
        // SAFETY: cache maintenance by virtual address changes no data: `dc civac` writes a
        // dirty line back before dropping it, `dc cvac` and `dc cvau` write back (no caller
        // has a discarding operation: `dc ivac` stays `bsd`'s `unsafe fn`). An address that is not mapped faults like any kernel access (a
        // trap, then a panic) and corrupts nothing. The loop's registers are the block's
        // own; memory is a clobber (not `nomem`), so the operation stays between the
        // accesses it orders.
        unsafe {
            asm!(
                "mrs {t}, ctr_el0",
                "ubfx {t}, {t}, #16, #4",
                "mov {line}, #4",
                "lsl {line}, {line}, {t}",
                "sub {t}, {line}, #1",
                "and {lo}, {a}, {t}",
                "add {n}, {n}, {lo}",
                "bic {a}, {a}, {t}",
                "2:",
                concat!("dc ", $dcop, ", {a}"),
                "add {a}, {a}, {line}",
                "subs {n}, {n}, {line}",
                "b.hi 2b",
                "dsb ish",
                a = inout(reg) va => _,
                n = inout(reg) len => _,
                t = out(reg) _,
                line = out(reg) _,
                lo = out(reg) _,
                options(nostack),
            )
        }
    }};
    ($va:expr, $len:expr, $dcop:literal, $icop:literal) => {{
        let (va, len): (usize, usize) = ($va, $len);
        // SAFETY: as for the data-cache form; `ic ivau` drops instruction-cache lines, which
        // are refetched from memory, so nothing is lost either.
        unsafe {
            asm!(
                "mrs {t}, ctr_el0",
                "ubfx {lo}, {t}, #16, #4",
                "and {t}, {t}, #0xf",
                "cmp {lo}, {t}",
                "csel {t}, {lo}, {t}, lo",
                "mov {line}, #4",
                "lsl {line}, {line}, {t}",
                "sub {t}, {line}, #1",
                "and {lo}, {a}, {t}",
                "add {n}, {n}, {lo}",
                "bic {a}, {a}, {t}",
                "mov {ia}, {a}",
                "mov {cnt}, {n}",
                "2:",
                concat!("dc ", $dcop, ", {a}"),
                "add {a}, {a}, {line}",
                "subs {n}, {n}, {line}",
                "b.hi 2b",
                "dsb ish",
                "3:",
                concat!("ic ", $icop, ", {ia}"),
                "add {ia}, {ia}, {line}",
                "subs {cnt}, {cnt}, {line}",
                "b.hi 3b",
                "dsb ish",
                "isb",
                a = inout(reg) va => _,
                n = inout(reg) len => _,
                t = out(reg) _,
                line = out(reg) _,
                lo = out(reg) _,
                ia = out(reg) _,
                cnt = out(reg) _,
                options(nostack),
            )
        }
    }};
}

/// `cpu_setttb(pm->pm_asid, pm->pm_pt0pa)`: `pm`'s ASID into `TTBR1_EL1` (the ASID's register,
/// `TCR_A1`) and its table into `TTBR0_EL1`. The caller loaded the kernel's empty table first
/// ([`ttbr0_empty`]), so no translation of the old address space is cached under the new ASID.
pub(super) fn cpu_setttb(pm: &Pmap) {
    let asid = pm.md.asid();
    let pt0pa = pm.md.pt0pa();
    if pt0pa == 0 {
        hal_panic(format_args!("cpu_setttb: a pmap without tables"));
    }
    // SAFETY: `pt0pa` is a table this pmap's machine part names for the lower half: the root
    // `create`/`create_efi` built (a user pmap's, until its last drop, which is refused while
    // a CPU has it loaded), or the kernel's empty table (`pmap_bootstrap`'s zeroed page, and a
    // purged pmap's). `TTBR0_EL1` translates only the lower half, where no Rust reference of
    // the kernel points (the user copy reaches it through its own fault-checked routines);
    // `bfi` changes only the ASID field of `TTBR1_EL1`, whose table base stays (at EL1,
    // where this runs, the kernel's root: `trampoline.S` puts the trampoline's there only for
    // EL0 and restores the kernel's on every entry), so the kernel's code, stack and data
    // translate as before. The ASID written is the pair's even member, the one EL1 runs
    // under (`pm_asid`'s low bit is clear: `pmap_allocate_asid` hands out pairs, and the
    // kernel's is 0); `tramp_return` sets bit 48, the odd member, for EL0 alone. The `isb`s order the switch before
    // the next instruction fetch; memory is a clobber.
    unsafe {
        asm!(
            "mrs {tmp}, ttbr1_el1",
            "bfi {tmp}, {asid}, #48, #16",
            "msr ttbr1_el1, {tmp}",
            "isb",
            "msr ttbr0_el1, {pt0pa}",
            "isb",
            tmp = out(reg) _,
            asid = in(reg) asid,
            pt0pa = in(reg) pt0pa,
            options(nostack, preserves_flags),
        );
    };
}

/// `WRITE_SPECIALREG(ttbr0_el1, pmap_kernel()->pm_pt0pa); isb`: the lower half to the kernel's
/// empty table.
pub(super) fn ttbr0_empty() {
    let pt0pa = crate::pmap::kernel().md.pt0pa();
    if pt0pa == 0 {
        hal_panic(format_args!(
            "pmap: TTBR0_EL1 switched before pmap_bootstrap"
        ));
    }
    // SAFETY: the kernel pmap's `pt0pa` is the zeroed page `pmap_bootstrap` took (its every
    // descriptor invalid), set before any switch can run (checked above); the lower half
    // then maps nothing, and nothing of the kernel lives there (see `cpu_setttb`).
    unsafe {
        asm!(
            "msr ttbr0_el1, {}",
            "isb",
            in(reg) pt0pa,
            options(nostack, preserves_flags),
        );
    };
}

/// `cpu_tlb_flush`: drops every TLB entry of the inner-shareable domain (all CPUs).
pub(crate) fn cpu_tlb_flush() {
    // SAFETY: TLB invalidation changes no mapping and no data: every later access refetches
    // its translation from the tables, which the `dsb ishst` made visible first.
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags),
        );
    };
}

/// `cpu_tlb_flush_asid(va)`: drops one page of one ASID on every CPU (`va` carries the ASID in
/// bits 63:48 and the page number in 43:0).
pub(crate) fn cpu_tlb_flush_asid(va: u64) {
    // SAFETY: as for `cpu_tlb_flush`; any operand is a valid invalidation.
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vae1is, {}",
            "dsb ish",
            "isb",
            in(reg) va,
            options(nostack, preserves_flags),
        );
    };
}

/// `cpu_tlb_flush_all_asid(va)`: drops one page for every ASID on every CPU (kernel mappings).
pub(crate) fn cpu_tlb_flush_all_asid(va: u64) {
    // SAFETY: as for `cpu_tlb_flush`.
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vaale1is, {}",
            "dsb ish",
            "isb",
            in(reg) va,
            options(nostack, preserves_flags),
        );
    };
}

/// `cpu_tlb_flush_asid_all(asid)`: drops every entry of one ASID (bits 63:48) on every CPU.
pub(crate) fn cpu_tlb_flush_asid_all(asid: u64) {
    // SAFETY: as for `cpu_tlb_flush`.
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi aside1is, {}",
            "dsb ish",
            "isb",
            in(reg) asid,
            options(nostack, preserves_flags),
        );
    };
}

/// `cpu_dcache_wb_range(va, len)`: writes the dirty data cache lines of `[va, va + len)`
/// back to memory (`dc cvac`), for a device about to read it (`bus_dma`, the interrupt
/// controllers' tables; `hal::arch::arm64::cpufunc`).
pub(crate) fn cpu_dcache_wb_range(va: usize, len: usize) {
    cache_handle_range!(va, len, "cvac");
}

/// `cpu_dcache_wbinv_range(va, len)`: writes back and invalidates the data cache over
/// `[va, va + len)` (`dc civac`).
pub(crate) fn cpu_dcache_wbinv_range(va: usize, len: usize) {
    cache_handle_range!(va, len, "civac");
}

/// `cpu_idcache_wbinv_range(va, len)`: writes back and invalidates the data cache, then
/// invalidates the instruction cache, over `[va, va + len)` (a managed page about to be
/// mapped non-cacheable).
pub(crate) fn cpu_idcache_wbinv_range(va: usize, len: usize) {
    cache_handle_range!(va, len, "civac", "ivau");
}

/// `cpu_icache_sync_range(va, len)`: makes instructions written to `[va, va + len)` visible
/// to instruction fetch (data cache cleaned to the point of unification, instruction cache
/// invalidated).
pub(crate) fn cpu_icache_sync_range(va: usize, len: usize) {
    cache_handle_range!(va, len, "cvau", "ivau");
}

/* </CODE> */
