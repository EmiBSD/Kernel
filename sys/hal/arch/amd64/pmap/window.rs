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
//! amd64: the window of the kernel image's own slot the ABI module `abi_openbsd.ko` runs
//! from.
//!
//! Upstream: sys/arch/amd64/amd64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/amd64/amd64/pmap.rs@3c62ede68802
//!
//! OpenBSD never executes from `kernel_map`: `pmap_alloc_level` gives every page-directory
//! entry it creates for the managed kernel range `pg_nx`, so a leaf entered `PROT_EXEC` there
//! still faults on instruction fetch. The only executable kernel memory is the image the boot
//! loader maps in the `KERNBASE` slot (`L4_SLOT_KERNBASE`, 511), whose directory entries
//! carry no NX (`locore0.S`'s "level 2 pages (RWX)", or Limine's tables). The module loader
//! (`hal::module::load`, decision 22) needs executable pages for the module's text; rather
//! than relax `kernel_map`'s directories (the user's decision of 2026-10-16, option C), the
//! module runs from a window of the kernel's own slot:
//!
//! - The bootstrap calls [`init`] once, at its end, on the boot CPU, with the kernel pmap in
//!   `%cr3`; both boot paths (boot(8) through `locore0.S`, and Limine) end there. It finds the
//!   last entry the boot loader used in the level-2 table of `KERNBASE`'s gigabyte (under
//!   boot(8): the image, its symbols and the module file up to `esym`, locore0's tables and
//!   the ISA hole's pages, all under locore0's `NKL2_KIMG_ENTRIES + 1` entries; under Limine:
//!   the image's segments), and links the [`MODULE_WINDOW_SIZE`] bytes after it, 2 MiB
//!   aligned, to level-1 tables the window owns ([`MODULE_WINDOW_PTPS`], in the kernel's
//!   `.bss`). Their directory entries have the flags locore0 gives the image's (`PG_V |
//!   PG_RW`, and `pg_crypt`), no `pg_nx`: the leaves decide.
//! - The window is outside `kernel_map` (slot 511; `kernel_map` is `pmap_virtual_space`'s
//!   range, in the slots after the direct map, which is checked), so uvm never hands it out,
//!   `pmap_growkernel` never reaches it and no directory above it is ever freed.
//! - [`enter`] enters one page in it, its leaf composed as `pmap_kenter_pa` composes one
//!   (`pg_nx` unless `PROT_EXEC`, read-only unless `PROT_WRITE`, `pg_crypt`, `pg_g_kern`),
//!   and refuses a writable executable page, a page entered already, an address outside the
//!   window and a physical address that is not a page's. The module is never unloaded, so no
//!   leaf is ever removed: the window lives for the kernel's lifetime.
//!
//! What executes after this: the kernel's text, and in the window only the leaves entered
//! `PROT_EXEC` (the module's text). What never does: `kernel_map`, which keeps `pg_nx` at
//! every directory level as in OpenBSD; the loader's writable copy (a `kernel_map` range,
//! unmapped before the module's entry point is called). A stray kernel write to the module's
//! text or read-only data faults on a read-only leaf outside every uvm map: a kernel panic,
//! the W^X behaviour wanted.
//!
//! ## Redesign
//! - New in native (the N2 close, the user's decision of 2026-10-16), first written as `bsd`'s
//!   `sys/arch/amd64/amd64/pmap_module.rs` beside LZ's pmap; it moves into `hal` with the
//!   pmap. OpenBSD has no executable kernel memory outside its image: the window re-expresses
//!   for the module what `locore0.S` and `pmap_bootstrap` do for the image in `KERNBASE`'s
//!   slot, level-1 tables under directory entries without NX and leaves with the protection of
//!   `pmap_kenter_pa`'s leaf.
//! - The level-1 tables are a typed static the window owns ([`Ptp`], `[AtomicU64; NPDPG]` per
//!   page), written with atomics, not pages taken from the boot memory map: the bootstrap runs
//!   before any memory is loaded into uvm on either boot path, and the `.bss` is the memory
//!   both boot loaders hand over zeroed, mapped and outside every free region.
//! - The boot loader's level-2 table is reached through the frame view of the direct map
//!   (`tables.rs`), as every table now is, where `pmap_module.rs` read it through the
//!   recursive mapping in an `unsafe` block of its own: the window has no `unsafe`.
//! - The link is attempted once ([`MODULE_WINDOW_TRIED`]): a refused first attempt leaves the
//!   window empty for good, so no later caller touches the boot loader's level-2 table once
//!   other CPUs run.

use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use super::super::include::param::{KERNBASE, PAGE_SIZE};
use super::super::include::pte::{NBPD_L2, NBPD_L3, PG_FRAME, PG_RO, PG_RW, PG_V};
use super::backend::Backend;
use super::layout::{NPDPG, PMAP_PA_MASK};
use super::tables::{frame, pg_crypt, pg_g_kern, pg_nx, table_from};
use super::tlb::{invlpg, rcr3};
use crate::pmap::{PmapBackend, VmProt};
use crate::sys::mman::{PROT_EXEC, PROT_WRITE};
use crate::sys::types::{Paddr, Vaddr};

/// The window's size: the largest image the module loader takes
/// (`hal::module::elf::MAX_SPAN`), so the window never refuses a module the loader accepts.
pub const MODULE_WINDOW_SIZE: usize = crate::module::elf::MAX_SPAN as usize;

/// The level-1 tables the window needs: one per 2 MiB.
const MODULE_WINDOW_NPTP: usize = MODULE_WINDOW_SIZE / NBPD_L2;

/// One level-1 page-table page, as the window owns it: the MMU reads it by its physical
/// address, the window writes it through its kernel address, with atomics.
#[repr(C, align(4096))]
struct Ptp([AtomicU64; NPDPG]);

impl Ptp {
    /// An empty table: every entry invalid.
    const fn new() -> Self {
        Self([const { AtomicU64::new(0) }; NPDPG])
    }
}

/// The window's level-1 tables, in the kernel's `.bss`: part of the image on both boot
/// paths, so never memory uvm manages.
static MODULE_WINDOW_PTPS: [Ptp; MODULE_WINDOW_NPTP] = [const { Ptp::new() }; MODULE_WINDOW_NPTP];

/// The window's first address, published once the window is linked; 0 while there is none.
static MODULE_WINDOW_VA: AtomicUsize = AtomicUsize::new(0);

/// Set by the first call of [`init`], whatever it achieved (see the redesign).
static MODULE_WINDOW_TRIED: AtomicBool = AtomicBool::new(false);

/// Links the module window into the kernel image's slot (see the module's documentation).
/// Once: the first call, from the end of the bootstrap on the boot CPU, is the only one that
/// may link; every later call returns at once. A first call with another page table in
/// `%cr3`, or with a slot that has no room, links nothing: the window stays empty, and the
/// module loader reports it has no memory for the module (fail-soft, decision 29); it is never
/// placed in `kernel_map` instead.
pub(super) fn init() {
    if MODULE_WINDOW_TRIED.swap(true, Ordering::AcqRel) {
        return;
    }
    let kpm = Backend::kernel();
    let root = kpm.md.root();
    if root != rcr3() & PMAP_PA_MASK as u64 {
        return;
    }

    // The gigabyte KERNBASE starts (L4 slot 511, its L3 entry): its level-2 table, which the
    // boot loader built for the image the kernel runs from and nothing frees.
    let gib = KERNBASE & !(NBPD_L3 - 1);
    let Some(l2pa) = table_from(Paddr::new(root as usize), gib, 2) else {
        return;
    };
    let l2 = frame(l2pa);

    // The window starts after the last entry the boot loader used, whatever it maps.
    let Some(last) = l2.iter().rposition(|e| e.load(Ordering::Relaxed) != 0) else {
        return;
    };
    let first = last + 1;
    let Some(slots) = l2.get(first..first + MODULE_WINDOW_NPTP) else {
        return;
    };
    let base = gib + first * NBPD_L2;

    // Outside kernel_map, which uvm manages.
    let (kva_start, kva_end) = Backend::virtual_space();
    if base < kva_end.as_usize() && kva_start.as_usize() < base + MODULE_WINDOW_SIZE {
        return;
    }

    // The tables' physical addresses (each page on its own: the image may not be physically
    // contiguous under Limine), then each one emptied before the MMU can see it.
    let mut ptp_pa = [0u64; MODULE_WINDOW_NPTP];
    for (pa, ptp) in ptp_pa.iter_mut().zip(&MODULE_WINDOW_PTPS) {
        let Some(p) = Backend::extract(kpm, Vaddr::new(ptr::from_ref(ptp).addr())) else {
            return;
        };
        *pa = p.as_usize() as u64 & PG_FRAME;
        for e in &ptp.0 {
            e.store(0, Ordering::Relaxed);
        }
    }

    // Link them: each entry must still be empty (the boot CPU is alone, so it is), else the
    // window is not published; an entry linked before the refusal maps nothing (its table is
    // empty) and stays so.
    let flags = PG_V | PG_RW | pg_crypt();
    for (slot, pa) in slots.iter().zip(ptp_pa) {
        if slot
            .compare_exchange(0, pa | flags, Ordering::Release, Ordering::Relaxed)
            .is_err()
        {
            return;
        }
    }
    // The entries were invalid, and the MMU caches no invalid entry: nothing to flush.
    MODULE_WINDOW_VA.store(base, Ordering::Release);
}

/// The module window: its first address and its size; empty (size 0) until the bootstrap
/// linked it, or when the kernel slot had no room for it. amd64 places the module nowhere
/// else.
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
/// page is entered already, `pa` is not a page's physical address, or `prot` is both
/// writable and executable.
pub(super) fn enter(va: Vaddr, pa: Paddr, prot: VmProt) -> bool {
    let base = MODULE_WINDOW_VA.load(Ordering::Acquire);
    let (va, pa) = (va.as_usize(), pa.as_usize() as u64);
    if base == 0 || !va.is_multiple_of(PAGE_SIZE) || pa & !PG_FRAME != 0 {
        return false;
    }
    if !w_xor_x(prot) {
        return false;
    }
    let Some(page) = va.checked_sub(base).map(|off| off / PAGE_SIZE) else {
        return false;
    };
    let Some(pte) = MODULE_WINDOW_PTPS
        .get(page / NPDPG)
        .and_then(|ptp| ptp.0.get(page % NPDPG))
    else {
        return false;
    };

    // pmap_kenter_pa's leaf: the window is above the first 2 MB, so pg_g_kern applies.
    let mut npte =
        pa | PG_V | if prot & PROT_WRITE != 0 { PG_RW } else { PG_RO } | pg_crypt() | pg_g_kern();
    if prot & PROT_EXEC == 0 {
        npte |= pg_nx();
    }
    // Only an empty entry is taken, so no CPU can hold a translation of the page: no TLB
    // shootdown is needed, and this CPU's entry is dropped for good measure.
    if pte
        .compare_exchange(0, npte, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return false;
    }
    invlpg(va);
    true
}

/// W^X: the loader asks for read, read-write or read-execute only.
const fn w_xor_x(prot: VmProt) -> bool {
    !(prot & PROT_WRITE != 0 && prot & PROT_EXEC != 0)
}

const _: () = {
    assert!(MODULE_WINDOW_SIZE.is_multiple_of(NBPD_L2));
    assert!(MODULE_WINDOW_NPTP <= NPDPG);
    assert!(size_of::<Ptp>() == PAGE_SIZE);
    assert!(w_xor_x(crate::sys::mman::PROT_READ | PROT_EXEC));
    assert!(w_xor_x(crate::sys::mman::PROT_READ | PROT_WRITE));
    assert!(!w_xor_x(PROT_WRITE | PROT_EXEC));
};
/* </CODE> */
