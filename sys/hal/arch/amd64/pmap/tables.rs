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
//! amd64's page tables as the pmap reaches them: through the direct map, one view function
//! for every frame, the walks over its atomic words, and the PTE bits the boot path decides.
//!
//! Upstream: sys/arch/amd64/amd64/pmap.c @ 3ce1f3f79392
//! LZ: sys/arch/amd64/amd64/pmap.rs@3c62ede68802
//!
//! **The view.** [`frame`] is the only place a frame of RAM becomes a Rust reference: 512
//! `AtomicU64`s at the frame's direct-map address, checked to lie inside the direct map the
//! boot glue described (`BootstrapParams`). The pmap reaches through it the root and the
//! page-table pages (the boot loader's, `pmap_growkernel`'s, a user pmap's), and the frames
//! `pmap_zero_page`/`pmap_copy_page`/`pmap_steal_memory` are given. Every access is atomic, so
//! the MMU's accessed and dirty bit updates and the other CPUs' writes never race with a
//! plain access. A table is viewed only while it is held: a user pmap's under its `pm_mtx`
//! (or, for the page-level calls, under the `pv_mtx` of a page it maps, which keeps every
//! table above that mapping), the kernel pmap's always (its tables are never freed); a table
//! goes back to the page provider only after the TLB shootdown's wait, when no walk can be in
//! it (`backend.rs`).
//!
//! ## Deviations
//! - `pg_xo` (PKU), `pg_crypt` (SEV) other than what `locore0.S` writes, `pmap_pg_wc` (PAT) and
//!   `pg_frame`/`pg_lgframe` narrowed by the encryption bit keep the "not available" values
//!   they had in LZ, as constants here: CPU identification does not set them (N4).
//!
//! ## Redesign
//! - LZ reads and writes entries through raw pointers (`pde_at`, `pde_set`,
//!   `pmap_pte_set`/`setbits`/`clearbits`, the recursive mapping's `kvtopte`), each an `unsafe`
//!   block on the caller's word. Here every entry is an `&AtomicU64` of a [`frame`] view: the
//!   C's `atomic_swap_64`/`atomic_setbits_u64`/`atomic_clearbits_u64` are `swap`,
//!   `fetch_or`, `fetch_and` on it, and the walks are safe code. `pmap_find_pte_direct` is
//!   [`find_pte`] returning the entry's word itself.
//! - Nothing reads through the recursive slot (`pmap_pdes_valid`, `normal_pdes` and
//!   `kvtopte` are gone): every walk starts at a pmap's root in the direct map.
//! - `PG_NX_BIT`, `PG_CRYPT` and `PG_G_KERN` stay `pub static AtomicU64`s: `locore0.S` writes
//!   and reads them by symbol (`bsd`'s `sys/arch/amd64/amd64/locore0.rs`), the trap code reads
//!   `pg_nx`. `pmap_direct_base`/`_end` are this module's, set once by the bootstrap.

use core::ptr;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::super::include::param::{PAGE_MASK, PAGE_SIZE};
use super::super::include::pte::{
    L4_MASK, L4_SHIFT, PG_FRAME, PG_LGFRAME, PG_PS, PG_UCMINUS, PG_V,
};
use super::layout::{NPDPG, PTP_LEVELS, va_sign_pos};
use crate::sys::types::{Paddr, Vaddr};
use crate::trust::check_frame;

/// `pmap_direct_base`: where physical address 0 is mapped (the boot glue's, at bootstrap).
static DIRECT_BASE: AtomicUsize = AtomicUsize::new(0);
/// `pmap_direct_end`: the end of the direct map.
static DIRECT_END: AtomicUsize = AtomicUsize::new(0);

/// `pg_nx`: NX PTE bit (if CPU supports). `locore0.S` sets it by symbol after a boot by
/// boot(8); the bootstrap sets it from `BootstrapParams::nx`.
pub static PG_NX_BIT: AtomicU64 = AtomicU64::new(0);
/// `pg_g_kern`: PG_G if global pages should be used in kernel mappings, 0 otherwise (for
/// insecure CPUs). Read by `locore0.S` by symbol.
pub static PG_G_KERN: AtomicU64 = AtomicU64::new(0);
/// `pg_crypt`: the memory encryption bit (SEV), 0 without it. Read by `locore0.S` by symbol.
pub static PG_CRYPT: AtomicU64 = AtomicU64::new(0);

/// `pg_xo`: XO PTE bits, set to PKU key1 (if cpu supports PKU): none here (see the
/// deviations).
pub(super) const PG_XO: u64 = 0;
/// `pmap_pg_wc`: the PTE bits of a write-combining mapping without PAT: UC- so the MTRRs can
/// override the cacheability.
pub(super) const PMAP_PG_WC: u64 = PG_UCMINUS;

/// `pg_nx`.
pub(super) fn pg_nx() -> u64 {
    PG_NX_BIT.load(Ordering::Relaxed)
}

/// `pg_crypt`.
pub(super) fn pg_crypt() -> u64 {
    PG_CRYPT.load(Ordering::Relaxed)
}

/// `pg_g_kern`.
pub(super) fn pg_g_kern() -> u64 {
    PG_G_KERN.load(Ordering::Relaxed)
}

/// `pg_frame`: the page frame bits of an entry.
pub(super) const fn pg_frame() -> u64 {
    PG_FRAME
}

/// The frame an entry points to.
pub(super) const fn entry_pa(e: u64) -> Paddr {
    Paddr::new((e & pg_frame()) as usize)
}

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
    (base <= va && va < end).then(|| Paddr::new(va - base))
}

/// The frame at `pa` as 512 atomic words, through the direct map (see the module docs).
/// Panics when `pa` is not a page's address or the page is not inside the direct map.
pub(super) fn frame(pa: Paddr) -> &'static [AtomicU64; NPDPG] {
    let base = DIRECT_BASE.load(Ordering::Relaxed);
    let end = DIRECT_END.load(Ordering::Relaxed);
    let va = check_frame(pa.as_usize(), PAGE_SIZE, base, end);
    // SAFETY: `va` is the direct-map address of the page-aligned frame `pa`, inside
    // `[DIRECT_BASE, DIRECT_END)` (`check_frame` above): the range the boot glue gave the bootstrap
    // (`BootstrapParams`, the boot contract of decision 30), which the boot loader maps
    // read-write to RAM for the kernel's lifetime and nothing unmaps. So the 4096 bytes are
    // dereferenceable for `'static` and aligned for `[AtomicU64; 512]` (a page is). The frames
    // viewed are page-table pages and frames handed to the pmap to fill (the trust of
    // `PhysPages` and of the caller's `Paddr`, `sys/hal/pmap.rs`): no plain Rust reference to
    // them is alive while they are viewed, and every access through the view is atomic, so it
    // races with nothing (the MMU's A/D updates are atomic too). The view is used only while
    // the frame is held (module docs); the reference never outlives that use in a way that
    // reads a frame after it is given back.
    unsafe { &*ptr::with_exposed_provenance::<[AtomicU64; NPDPG]>(va) }
}

/// The entry `index` of the table at `pa`.
pub(super) fn entry(pa: Paddr, index: usize) -> &'static AtomicU64 {
    &frame(pa)[index % NPDPG]
}

/// Zeroes the frame at `pa` (`pagezero`).
pub(super) fn zero_frame(pa: Paddr) {
    for w in frame(pa) {
        w.store(0, Ordering::Relaxed);
    }
}

/// Copies the frame at `src` into the frame at `dst`.
pub(super) fn copy_frame(src: Paddr, dst: Paddr) {
    for (d, s) in frame(dst).iter().zip(frame(src)) {
        d.store(s.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

/// `pmap_find_pte_direct`: walks the tables from `root` down to `va`'s entry. The level the
/// walk stopped at (0: the level-1 entry was reached; 1: a 2M page or an invalid level-2
/// entry; ...), the table's frame there and the index in it.
pub(super) fn find_pte_from(root: Paddr, va: usize) -> (usize, Paddr, usize) {
    let mut table = root;
    let mut shift = L4_SHIFT;
    let mut mask = L4_MASK;
    let mut offs = 0;
    for lev in (1..=PTP_LEVELS).rev() {
        offs = (va_sign_pos(va) & mask) >> shift;
        let pde = entry(table, offs).load(Ordering::Relaxed);
        // Large pages are different, break early if we run into one.
        if pde & (PG_PS | PG_V) != PG_V {
            return (lev - 1, table, offs);
        }
        if lev == 1 {
            break;
        }
        table = entry_pa(pde);
        // 4096/8 == 512 == 2^9 entries per level
        shift -= 9;
        mask >>= 9;
    }
    (0, table, offs)
}

/// The frame of the level-`level` table of `va` under `root` (the root for level 4), `None`
/// where a level above is invalid or a large page.
pub(super) fn table_from(root: Paddr, va: usize, level: usize) -> Option<Paddr> {
    let mut table = root;
    for lvl in ((level + 1)..=PTP_LEVELS).rev() {
        let pde = entry(table, super::layout::pl_pi(va, lvl)).load(Ordering::Relaxed);
        if pde & (PG_PS | PG_V) != PG_V {
            return None;
        }
        table = entry_pa(pde);
    }
    Some(table)
}

/// The physical address a leaf maps `va` to: a level-1 entry, or a 2M page met at level 2.
pub(super) fn leaf_pa(level: usize, pte: u64, va: usize) -> Option<Paddr> {
    if level == 0 && pte & PG_V != 0 {
        return Some(Paddr::new((pte & pg_frame()) as usize | (va & PAGE_MASK)));
    }
    if level == 1 && pte & (PG_PS | PG_V) == (PG_PS | PG_V) {
        let off = va & super::super::include::pte::PAGE_MASK_L2;
        return Some(Paddr::new((pte & PG_LGFRAME) as usize | off));
    }
    None
}
/* </CODE> */
