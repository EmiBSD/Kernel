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
/* </LICENSES> */

/* <CODE> */
//! amd64's pmap behind `hal::pmap`: `pmap.c`'s pmaps, pv lists, page-table pages and
//! mapping functions over the direct-map tables of `tables.rs` and the shootdowns of `tlb.rs`.
//!
//! Upstream: sys/arch/amd64/amd64/pmap.c @ 3ce1f3f79392
//! Upstream: sys/arch/amd64/include/pmap.h @ 3ce1f3f79392
//! LZ: sys/arch/amd64/amd64/pmap.rs@3c62ede68802
//! LZ: sys/arch/amd64/include/pmap.rs@3c62ede68802
//!
//! This is the i386 pmap modified and generalized to support x86-64, as the C says: the upper
//! levels of the page tables are hidden inside the PTP functions and `pmap_growkernel`.
//!
//! **Locks** (as the C): a user pmap's `pm_mtx` (`IPL_VM`) is taken before a page's `pv_mtx`
//! (`IPL_VM`); the kernel pmap is never locked (a [`Locked::Kernel`] witness); page-table
//! pages are allocated under `pm_mtx`, so the page allocator's locks nest inside it;
//! `pmap_page_remove` takes a reference to the pmap under `pv_mtx`, drops it, takes `pm_mtx`,
//! re-takes `pv_mtx` and re-checks. PTPs and pv entries are freed only after the shootdown's
//! wait.
//!
//! ## Deviations
//! - After a boot by Limine the kernel runs on the boot loader's PML4 (`%cr3`), which the
//!   bootstrap adopts and in which it installs the recursive slot (`PDIR_SLOT_PTE`) where the
//!   C's `locore0.S` does; after a boot by boot(8) both are `locore0.S`'s, as in C. The direct
//!   map is the boot loader's (Limine's HHDM, or `locore0.S`'s first 4 GB): its page tables
//!   are not built here.
//! - The managed kernel range is one whole PML4 slot, the C's size, but not always the C's
//!   slot 256: when the direct map overlaps `[VM_MIN_KERNEL_ADDRESS, VM_MAX_KERNEL_ADDRESS)`,
//!   as Limine's default placement does, it is the first whole slot after the direct map's
//!   end (`kernel_window` in `layout.rs`: `virtual_avail = roundup(direct_end, NBPD_L4)`, its
//!   end one slot further, which `pmap_virtual_space` reports). Limine's HHDM over 4 GiB of
//!   memory gives slot 257; QEMU's map reaching 1 TiB puts it over slots 256-257 and the
//!   window in slot 258. So the window always equals the C's size (what `VM_KERNEL_SPACE_SIZE`
//!   and `kmeminit`'s clamp assume), never lies over the direct map, and never needs a second
//!   top-level entry (LZ's window could straddle two slots under Limine and grow into the
//!   second without one); a direct map that leaves no whole slot below `KERNBASE`'s panics at
//!   bootstrap. `pmap_growkernel` counts its PTPs from `virtual_avail` (`pmap_kva_start`),
//!   keeps the page-table pages the boot loader installed on the way, and panics on a large
//!   page there. The boot glue moves uvm's runtime `vm_min_kernel_address` to `virtual_avail`
//!   ([`crate::pmap::Bootstrapped`]).
//! - `pg_nx` comes from `EFER.NXE` as the boot loader left it (`BootstrapParams::nx`); the
//!   C's `locore0.S` probes CPUID. PCID is off; `cpu_meltdown` is not configured (no
//!   `pm_pdir_intel`); vmm's nested pmaps are not configured.
//! - The PDP of a user pmap comes from the page provider rather than `pmap_pdp_pool`
//!   (`pmap_pdp_ctor` copies the kernel's whole upper half of the PML4, where the C copies the
//!   kernel VM, direct-map and `KERNBASE` slots one by one).
//! - `pmap_remove_pte` (the single-page shortcut of `pmap_do_remove`) is folded into the
//!   block loop.
//! - The pmap list (`pmaps`, `pmaps_lock`) is not kept: `NKL4_MAX_ENTRIES` is 1, so
//!   `pmap_growkernel` makes the kernel's one top-level entry at boot, before any user pmap;
//!   a new top-level entry once a user pmap exists panics (G11) instead of copying it into
//!   every pmap. `splhigh()` around the growth is kept.
//! - `pmap_unwire` is the C's under `DIAGNOSTIC` (a panic on an unmapped `va` and on an
//!   invalid PDE, the "didn't change" message) and silent without it; the message goes to
//!   the console (`hal::cons`), not the message buffer. LZ printed both messages always and
//!   panicked on an invalid PDE in every configuration.
//! - `pool_sethiwat(&pmap_pv_pool, 32 * 1024)` (pmap.c) is not set (LZ did not either):
//!   `hal::pool` has no high-water mark yet, so `vmstat -m`'s Hiwat column of `pvpl` is the
//!   pool's default.
//! - `pmap_free_ptp` shoots the recursive view's page of a freed table on this CPU always,
//!   where the C asks whether `curpcb`'s pmap is loaded (it is, but in the middle of a
//!   switch): one `invlpg` more, never one less.
//! - `PMAP_EFI` is refused (a panic) by `pmap_enter`: amd64's `efi_machdep.c` is not in LZ at
//!   the pin, so nothing enters an EFI runtime mapping, whose PTE the C makes without `PG_u`.
//!
//! ## Redesign
//! - `struct pmap` is `hal::pmap::Pmap` around [`PmapMd`]: `pm_mtx` as a `Mutexed<()>` whose
//!   guard is the [`Locked`] witness every walk of a user pmap takes, the root
//!   (`pm_pdirpa`), the count of page-table pages in `resident_count` (so the last drop can
//!   tell mappings from tables). `pm_obj`, `pm_ptphint`,
//!   `pm_pdir` (a pointer into the direct map), `pm_type` and `eptp` are gone: a PTP is found
//!   by the walk, whose directory entry is the lookup (`pmap_find_ptp` dropped), and its
//!   count of valid entries plus one (the C's `wire_count`) is its `PageMd::ptp_refs`, reached
//!   through the page provider.
//! - `struct pv_entry` is [`PvEntry`] in a `PoolBox` (pool `"pvpl"`), the page's chain owned
//!   head to tail (`PvHead`, under `pv_mtx`); an entry leaves by a cursor, and dropping one
//!   frees its tail one entry at a time (its `Drop`), never by recursion. Its pmap is a
//!   pointer dereferenced in one place ([`PvEntry::pmap`]): a pmap with a mapping is alive,
//!   since `hal::pmap`'s last drop panics while one is left.
//! - `pmap_reference(pve->pv_pmap)` + `pmap_destroy` around `pmap_page_remove`'s lock dance is
//!   a counted handle rebuilt under `pv_mtx` ([`pmap_reference_pv`], `PoolArc::from_value`),
//!   dropped once both locks are released: its drop may be the last, which frees the tables.
//! - `pmap_kenter_pa`, `pmap_kremove` walk the kernel tables through the direct map (LZ used
//!   the recursive mapping's `kvtopte`); a missing page table is a panic (the C faults).
//! - `pmap_pdp_ctor`, `pmap_get_physpage` and `pmap_prealloc_lowmem_ptps` write through the
//!   frame view; `pmap_prealloc_lowmem_ptps` is a method of `KernelMapper`, the authority it
//!   needs.

use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};

use super::super::include::param::PAGE_SIZE;
use super::super::include::pte::{
    NBPD_L2, PG_M, PG_N, PG_NX, PG_PS, PG_RO, PG_RW, PG_U, PG_V, PG_u, x86_round_pdr,
};
use super::super::include::vmparam::{
    VM_MAX_ADDRESS, VM_MAX_KERNEL_ADDRESS, VM_MAXUSER_ADDRESS, VM_MIN_KERNEL_ADDRESS,
};
use super::layout::{
    L2_BASE, L3_BASE, L4_BASE, NKPTP_INITIALIZER, NKPTPMAX_INITIALIZER, NTOPLEVEL_PDES,
    PDIR_SLOT_PTE, PG_PMAP_WC, PG_PVLIST, PG_W, PMAP_EFI, PMAP_NOCACHE, PMAP_NOCRYPT, PMAP_PA_MASK,
    PMAP_WC, PTE_BASE, PTP_LEVELS, PTP_SHIFTS, kernel_window, pl_i, pl_pi, pmap_valid_entry,
    protection_code, va_sign_neg,
};
use super::tables::{
    PG_NX_BIT, PG_XO, PMAP_PG_WC, copy_frame, direct_map, direct_unmap, entry, entry_pa,
    find_pte_from, frame, leaf_pa, pg_crypt, pg_frame, pg_g_kern, pg_nx, set_direct_map,
    table_from, zero_frame,
};
use super::tlb::{
    cflushsz, clflush, lcr3, mfence, pmap_tlb_shootpage, pmap_tlb_shootrange, pmap_tlb_shoottlb,
    pmap_tlb_shootwait, rcr3, tlbflush, wbinvd_on_all_cpus,
};
use super::window;
use crate::assert::{hal_panic, kassert};
use crate::intr::IPL_VM;
use crate::mutex::{Mutexed, MutexedGuard};
use crate::pmap::{
    BootstrapParams, KernelMapper, PG_PMAP_MOD, PG_PMAP_REF, PMAP_CANFAIL, PMAP_WIRED, PageMd,
    PhysPages, Pmap, PmapBackend, PmapRef, PmapStatistics, VmProt,
};
use crate::pool::{BoxPool, PR_NOWAIT, PoolArc, PoolBox};
use crate::sys::errno::Errno;
use crate::sys::mman::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};
use crate::sys::types::{Paddr, Vaddr, Vsize};
use crate::vm::param::{atop, round_page};

/// `PMAP_REMOVE_ALL`: `pmap_do_remove` removes every mapping.
const PMAP_REMOVE_ALL: i32 = 0;

/// How many page-table pages one removal collects before it waits for the shootdowns and
/// frees them (the C's unbounded `empty_ptps` list, bounded on a small kernel stack).
const FREE_BATCH: usize = 32;

/// The machine part of `struct pmap`.
///
/// Invariant: `root` is set before the pmap is shared (the bootstrap's `%cr3` for the kernel
/// pmap, `create`'s PDP for a user one) and is the frame of a PML4 holding the kernel half
/// until the pmap is freed.
pub(crate) struct PmapMd {
    /// `pm_mtx`: a user pmap's tables and statistics; the kernel pmap never takes it.
    mtx: Mutexed<()>,
    /// `pm_pdirpa`: the PML4's physical address.
    root: AtomicU64,
    /// The page-table pages (the PDP included) counted in `resident_count`.
    ptps: AtomicI64,
}

impl PmapMd {
    /// A pmap's part with no root yet (the kernel pmap's static, set by the bootstrap).
    pub(crate) const fn new() -> Self {
        Self {
            mtx: Mutexed::new(IPL_VM, ()),
            root: AtomicU64::new(0),
            ptps: AtomicI64::new(0),
        }
    }

    /// The root's physical address, the `%cr3` value (PCID off).
    pub(super) fn root(&self) -> u64 {
        self.root.load(Ordering::Acquire)
    }

    /// The root as a frame.
    fn root_pa(&self) -> Paddr {
        Paddr::new(self.root() as usize & PMAP_PA_MASK)
    }
}

/// `pm_mtx` held for a user pmap, or the kernel pmap, which is never locked: the witness every
/// walk of a pmap's tables takes (`pmap_map_ptes`/`pmap_unmap_ptes`).
pub(super) enum Locked<'a> {
    /// The kernel pmap: its tables are never freed.
    Kernel,
    /// A user pmap's `pm_mtx`.
    User(#[allow(dead_code, reason = "held for its drop")] MutexedGuard<'a, ()>),
}

/// `pmap_map_ptes`: lock the target map before touching its page tables, to guarantee other
/// CPUs have finished changing the tables before we potentially start caching table and TLB
/// entries. The kernel's pmap is always accessible (and has no lock).
fn pmap_map_ptes(pmap: &Pmap) -> Locked<'_> {
    if pmap.is_kernel() {
        Locked::Kernel
    } else {
        Locked::User(pmap.md.mtx.lock())
    }
}

/// `struct pv_entry`: one managed mapping of a page, on the page's chain under its `pv_mtx`.
pub(crate) struct PvEntry {
    /// The next entry, owned.
    next: Option<PoolBox<PvEntry>>,
    /// The pmap (see [`PvEntry::pmap`]).
    pmap: NonNull<Pmap>,
    /// The virtual address.
    va: usize,
    /// The level-1 PTP that holds the mapping's PTE (`None` for the kernel pmap).
    ptp: Option<Paddr>,
}

impl PvEntry {
    /// The pmap of this mapping.
    fn pmap(&self) -> &Pmap {
        // SAFETY: an entry exists only while its mapping does: `pmap_enter` links it when it
        // enters the mapping, and every path that zaps the PTE unlinks it under the same
        // `pv_mtx` (`pmap_remove_ptes`, `pmap_page_remove`, `pmap_enter` changing the frame).
        // A pmap with a mapping is never freed: `hal::pmap`'s last drop panics while
        // `Backend::mappings` is not 0, leaving its pool item in place. The reference borrows
        // the entry, which the caller holds under the page's `pv_mtx` (or owns, unlinked from
        // its page's chain while it holds a counted handle to the pmap), so the pmap outlives
        // it.
        unsafe { self.pmap.as_ref() }
    }
}

impl Drop for PvEntry {
    /// Frees the entries this one owns one at a time: a chain of hundreds would otherwise
    /// recurse once per entry on the kernel stack.
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut e) = next {
            next = e.next.take();
            drop(e);
        }
    }
}

/// Panics when another running CPU runs on `pmap` (its `ci_proc_pmap`): its tables are about
/// to be freed. Nothing to look at while the CPU bridge is not registered (one CPU).
fn other_cpus_off(pmap: &Pmap) {
    if let Some(b) = crate::bridges::tlb_cpus() {
        b.for_each_other_running(&mut |id, active| {
            if ptr::eq(active, pmap.as_ptr()) {
                hal_panic(format_args!("pmap_destroy: the pmap is loaded on cpu {id}"));
            }
        });
    }
}

/// `pmap_reference(pve->pv_pmap)`: a counted handle to the pmap of a linked entry, taken
/// under the page's `pv_mtx` (`pmap_page_remove`, before it drops the lock); the kernel
/// pmap's is its static arm.
fn pmap_reference_pv(e: &PvEntry) -> PmapRef {
    if ptr::eq(e.pmap.as_ptr(), &KERNEL_PMAP) {
        return PmapRef::Kernel;
    }
    // SAFETY: `e.pmap` is the address of a user pmap's value inside its item of
    // `hal::pmap`'s pool (`pmap_enter` recorded `NonNull::from(pmap)` of the `&Pmap` a
    // `PmapRef` derefs to, and every pmap but the kernel's is one `create` put in that pool).
    // The entry is linked and the caller holds the page's `pv_mtx`, so the mapping exists
    // (every removal unlinks the entry under this lock first), and a pmap with a mapping is
    // never put back: `hal::pmap`'s `Drop for Pmap` panics while one is left, before
    // `PoolArc`'s drop reaches the put. So the item is not put back during the call; a count
    // of 0 (the last handle gone, that drop under way) is refused by `from_value` itself.
    PmapRef::User(unsafe { PoolArc::from_value(e.pmap) })
}

// SAFETY: a `PvEntry` holds plain values and a pointer used only as `PvEntry::pmap` says
// (under the page's `pv_mtx`, or by the owner of an unlinked entry); `Pmap` is `Sync`, so
// moving an entry to another CPU and reading its pmap there is sound.
unsafe impl Send for PvEntry {}

/// `struct vm_page_md`'s chain: the page's managed mappings.
pub(crate) struct PvHead(Option<PoolBox<PvEntry>>);

impl PvHead {
    /// No mapping.
    pub(crate) const fn new() -> Self {
        Self(None)
    }
}

/// `kernel_pmap_store`: the kernel's pmap (proc 0).
static KERNEL_PMAP: Pmap = Pmap::kernel_with(PmapMd::new());
/// `pmap_pv_pool`: pool for pv entries.
static PV_POOL: BoxPool<PvEntry> = BoxPool::new();
/// `pmap_initialized`: pmap_init done yet?
static PMAP_INITIALIZED: AtomicBool = AtomicBool::new(false);
/// A user pmap was created: from then on no new top-level kernel entry may appear (G11).
static USER_PMAP_CREATED: AtomicBool = AtomicBool::new(false);
/// `virtual_avail`: the first free kernel virtual address.
static VIRTUAL_AVAIL: AtomicUsize = AtomicUsize::new(0);
/// `pmap_maxkvaddr`: how far the kernel page tables reach.
static PMAP_MAXKVADDR: AtomicUsize = AtomicUsize::new(VM_MIN_KERNEL_ADDRESS);
/// Where the managed kernel range begins: `VM_MIN_KERNEL_ADDRESS` in the C (see the
/// deviations).
static PMAP_KVA_START: AtomicUsize = AtomicUsize::new(VM_MIN_KERNEL_ADDRESS);
/// Where the managed kernel range ends: `VM_MAX_KERNEL_ADDRESS` in the C, one PML4 slot
/// after `pmap_kva_start` here (see the deviations).
static PMAP_KVA_END: AtomicUsize = AtomicUsize::new(VM_MAX_KERNEL_ADDRESS);
/// `nkptp[]`: how many page-table pages the kernel has at each level.
static NKPTP: [AtomicUsize; PTP_LEVELS] = [
    AtomicUsize::new(NKPTP_INITIALIZER[0]),
    AtomicUsize::new(NKPTP_INITIALIZER[1]),
    AtomicUsize::new(NKPTP_INITIALIZER[2]),
    AtomicUsize::new(NKPTP_INITIALIZER[3]),
];

/// amd64's pmap backend.
pub(crate) struct Backend;

/// `pmap_pte2flags`: the `PG_PMAP_*` page flags matching a PTE's R/M bits.
fn pmap_pte2flags(pte: u64) -> u32 {
    (if pte & PG_U != 0 { PG_PMAP_REF } else { 0 })
        | (if pte & PG_M != 0 { PG_PMAP_MOD } else { 0 })
}

/// `pmap_sync_flags_pte`: copies a PTE's R/M bits into the page's flags.
fn pmap_sync_flags_pte(md: &PageMd, pte: u64) {
    if pte & (PG_U | PG_M) != 0 {
        md.attrs.fetch_or(pmap_pte2flags(pte), Ordering::Relaxed);
    }
}

/// `pmap_is_curpmap`: is this pmap the one currently loaded \[in `%cr3`\]? The kernel is
/// always loaded: its half of the address space is in every page directory.
fn pmap_is_curpmap(pm: &Pmap) -> bool {
    pm.is_kernel() || pm.md.root() == rcr3() & pg_frame()
}

/// The page provider's `PageMd` of a page-table page; a user PTP is always a managed page.
fn ptp_md(pages: &dyn PhysPages, ptp: Paddr) -> &'static PageMd {
    match pages.md_of(ptp) {
        Some(md) => md,
        None => hal_panic(format_args!("pmap: PTP {ptp:?} is not a managed page")),
    }
}

/// `+= delta` on a statistics counter (the C's `++`/`--` on a `long`).
fn add(c: &crate::pmap::PmapCounter, delta: i64) {
    c.set(c.get() + delta);
}

/// The PTPs one removal frees once the shootdown is waited for (`empty_ptps`).
struct FreePtps {
    pages: [Paddr; FREE_BATCH],
    n: usize,
}

impl FreePtps {
    /// None yet.
    const fn new() -> Self {
        Self {
            pages: [Paddr::new(0); FREE_BATCH],
            n: 0,
        }
    }

    /// Queues `pa`; when the batch is full, waits for this CPU's shootdowns and frees it.
    fn push(&mut self, pages: &dyn PhysPages, pa: Paddr) {
        if self.n == FREE_BATCH {
            self.free(pages);
        }
        self.pages[self.n] = pa;
        self.n += 1;
    }

    /// Waits for this CPU's shootdowns (no CPU can still walk a queued table), then gives the
    /// queued pages back.
    fn free(&mut self, pages: &dyn PhysPages) {
        if self.n == 0 {
            return;
        }
        pmap_tlb_shootwait();
        for &pa in &self.pages[..self.n] {
            pages.free_ptp(pa);
        }
        self.n = 0;
    }
}

/// Gives a page-table page back to the provider, its count cleared (the C's `wire_count = 0`
/// before `uvm_pagefree`).
fn free_table(pages: &dyn PhysPages, pa: Paddr) {
    ptp_md(pages, pa).ptp_refs.store(0, Ordering::Relaxed);
    pages.free_ptp(pa);
}

/// Frees removed pv entries (`pmap_free_pvs`): the head's drop frees the rest one at a time
/// (`PvEntry`'s `Drop`).
fn free_pvs(list: Option<PoolBox<PvEntry>>) {
    drop(list);
}

/// `pmap_enter_pv`: enter a mapping onto a pv list. The caller adjusted the PTP's count.
fn pmap_enter_pv(md: &PageMd, mut pve: PoolBox<PvEntry>) {
    let mut head = md.pv.lock();
    pve.next = head.0.take(); // add to ...
    head.0 = Some(pve); // ... list
}

/// `pmap_remove_pv`: removes the mapping of `pmap` at `va` from the page's pv list; the
/// removed entry, unlinked (its `next` is `None`).
fn pmap_remove_pv(md: &PageMd, pmap: &Pmap, va: usize) -> Option<PoolBox<PvEntry>> {
    let mut head = md.pv.lock();
    let mut cur = &mut head.0;
    while cur
        .as_ref()
        .is_some_and(|e| !(ptr::eq(e.pmap.as_ptr(), pmap) && e.va == va))
    {
        cur = &mut cur.as_mut()?.next;
    }
    let mut found = cur.take()?;
    *cur = found.next.take(); // remove it!
    Some(found)
}

/// `pmap_get_physpage`: a zeroed page for one of the kernel's page-table pages: stolen while
/// the page system is not up (from `uvm_pageboot_alloc`), else from the provider.
fn pmap_get_physpage() -> Paddr {
    let kpm = &KERNEL_PMAP;
    let pages = kpm.pages();
    let pa = if pages.page_init_done() {
        match pages.alloc_ptp() {
            Some(pa) => pa,
            None => hal_panic(format_args!("pmap_get_physpage: out of memory")),
        }
    } else {
        match pages.steal(1) {
            Some(pa) => {
                zero_frame(pa);
                pa
            }
            None => hal_panic(format_args!("pmap_get_physpage: out of memory")),
        }
    };
    add(&kpm.stats.resident_count, 1);
    pa
}

/// `pmap_alloc_level`: allocate the amount of specified ptps for a ptp level, and populate
/// all levels below accordingly, mapping virtual addresses starting at kva. Used by
/// `pmap_growkernel`.
fn pmap_alloc_level(kva: usize, lvl: usize, needed_ptps: &[isize; PTP_LEVELS]) {
    let root = KERNEL_PMAP.md.root_pa();
    for level in (2..=lvl).rev() {
        let mut index = pl_i(kva, level);
        let mut endindex = index as isize + needed_ptps[level - 1];
        // XXX special case for first time call.
        if NKPTP[level - 1].load(Ordering::Relaxed) != 0 {
            index += 1;
        } else {
            endindex -= 1;
        }

        let mut i = index;
        while (i as isize) <= endindex {
            // Entry `i` of the level counted across all its tables (the C's `pdep[i]` through
            // the recursive mapping) belongs to the kernel address `va`, whose level-`level`
            // table the levels above (made first) lead to.
            let va = va_sign_neg(i << PTP_SHIFTS[level - 1]);
            let Some(table) = table_from(root, va, level) else {
                hal_panic(format_args!(
                    "pmap_alloc_level: no level {level} table for {va:#x}"
                ));
            };
            let e = entry(table, pl_pi(va, level));
            let pde = e.load(Ordering::Relaxed);
            if pmap_valid_entry(pde) {
                // The boot loader's page-table pages are kept (see the deviations).
                if pde & PG_PS != 0 {
                    hal_panic(format_args!(
                        "pmap_alloc_level: large page at level {level} index {i}"
                    ));
                }
            } else {
                let pa = pmap_get_physpage();
                e.store(
                    pa.as_usize() as u64 | PG_RW | PG_V | pg_nx() | pg_crypt(),
                    Ordering::Release,
                );
            }
            NKPTP[level - 1].fetch_add(1, Ordering::Relaxed);
            i += 1;
        }
    }
}

/// The end of the managed kernel window, set by the bootstrap: one PML4 slot after
/// `pmap_kva_start` (see the deviations).
fn kernel_virtual_end() -> usize {
    PMAP_KVA_END.load(Ordering::Relaxed)
}

/// `pmap_freepage` + `pmap_free_ptp`: frees the level-1 PTP `ptp` of `va` (which no longer
/// maps anything) and, level by level, the tables above it that it was the last user of; the
/// pages go to `freed`, given back after the shootdown's wait.
fn pmap_free_ptp(pmap: &Pmap, _locked: &Locked<'_>, ptp: Paddr, va: usize, freed: &mut FreePtps) {
    let pages = pmap.pages();
    let root = pmap.md.root_pa();
    let mut ptp = ptp;
    let mut level = 1;
    loop {
        // pmap_freepage
        add(&pmap.stats.resident_count, -1);
        pmap.md.ptps.fetch_sub(1, Ordering::Relaxed);
        ptp_md(pages, ptp).ptp_refs.store(0, Ordering::Relaxed);
        freed.push(pages, ptp);

        let Some(table) = table_from(root, va, level + 1) else {
            hal_panic(format_args!(
                "pmap_free_ptp: level {} table of {va:#x} is gone",
                level + 1
            ));
        };
        entry(table, pl_pi(va, level + 1)).store(0, Ordering::Release);
        // The recursive mapping's page that showed the freed table: invlpg on it drops the
        // TLB entry of that view and the paging-structure caches that still point at the
        // table (this CPU always: the C's "is curpcb's pmap loaded" is true but in a switch).
        let invaladdr = match level {
            1 => PTE_BASE,
            2 => L2_BASE,
            3 => L3_BASE,
            _ => L4_BASE,
        };
        pmap_tlb_shootpage(pmap, invaladdr + pl_i(va, level + 1) * PAGE_SIZE, true);
        if level < PTP_LEVELS - 1 {
            // The parent is the table just changed.
            let refs = &ptp_md(pages, table).ptp_refs;
            if refs.fetch_sub(1, Ordering::Relaxed) - 1 > 1 {
                break;
            }
            ptp = table;
        }
        level += 1;
        if level >= PTP_LEVELS {
            break;
        }
    }
}

/// `pmap_get_ptp`: the level-1 PTP of `va` in a user pmap, the missing levels allocated
/// (zeroed, each counted in its parent); `None` when a page could not be had.
fn pmap_get_ptp(pmap: &Pmap, _locked: &Locked<'_>, va: usize) -> Option<Paddr> {
    let pages = pmap.pages();
    let mut table = pmap.md.root_pa();
    // Loop through all page table levels seeing if we need to add a new page to that level.
    for i in (2..=PTP_LEVELS).rev() {
        let e = entry(table, pl_pi(va, i));
        let pde = e.load(Ordering::Relaxed);
        if pmap_valid_entry(pde) {
            table = entry_pa(pde);
            continue;
        }
        let new = pages.alloc_ptp()?;
        ptp_md(pages, new).ptp_refs.store(1, Ordering::Relaxed);
        e.store(
            new.as_usize() as u64 | PG_u | PG_RW | PG_V | pg_crypt(),
            Ordering::Release,
        );
        add(&pmap.stats.resident_count, 1);
        pmap.md.ptps.fetch_add(1, Ordering::Relaxed);
        // If we're not in the top level, increase the wire count of the parent page.
        if i < PTP_LEVELS {
            ptp_md(pages, table)
                .ptp_refs
                .fetch_add(1, Ordering::Relaxed);
        }
        table = new;
    }
    Some(table)
}

/// `pmap_pdp_ctor`: the page directory at `pdirpa` gets the recursive entry and the kernel's
/// upper half (see the deviations); the lower half is the provider's zeroes.
fn pmap_pdp_ctor(pdirpa: Paddr) {
    let kpdir = frame(KERNEL_PMAP.md.root_pa());
    let pdir = frame(pdirpa);
    // put in recursive PDE to map the PTEs
    pdir[PDIR_SLOT_PTE].store(
        pdirpa.as_usize() as u64 | PG_V | PG_RW | pg_nx() | pg_crypt(),
        Ordering::Relaxed,
    );
    for i in (PDIR_SLOT_PTE + 1)..NTOPLEVEL_PDES {
        pdir[i].store(kpdir[i].load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

/// `pmap_remove_ptes`: removes the valid PTEs of `[startva, endva)` from the level-1 table
/// `table` (entry `offs` maps `startva`) while the PTP still counts some (its `ptp_refs` is
/// above 1); the removed pv entries are chained onto `free`.
#[allow(clippy::too_many_arguments, reason = "the C's argument list")]
fn pmap_remove_ptes(
    pmap: &Pmap,
    ptp: Option<&PageMd>,
    table: Paddr,
    offs: usize,
    startva: usize,
    endva: usize,
    free: &mut Option<PoolBox<PvEntry>>,
) {
    let pages = pmap.pages();
    let ptes = frame(table);
    let mut va = startva;
    let mut i = offs;
    while va < endva && ptp.is_none_or(|p| p.ptp_refs.load(Ordering::Relaxed) > 1) {
        let Some(pte) = ptes.get(i) else {
            break;
        };
        if !pmap_valid_entry(pte.load(Ordering::Relaxed)) {
            // VA not mapped
            i += 1;
            va += PAGE_SIZE;
            continue;
        }
        // atomically save the old PTE and zap! it
        let opte = pte.swap(0, Ordering::AcqRel);
        if opte & PG_W != 0 {
            add(&pmap.stats.wired_count, -1);
        }
        add(&pmap.stats.resident_count, -1);
        if let Some(p) = ptp {
            p.ptp_refs.fetch_sub(1, Ordering::Relaxed); // dropping a PTE
        }

        let md = pages.md_of(entry_pa(opte));
        // if we are not on a pv list we are done.
        if opte & PG_PVLIST == 0 {
            if cfg!(feature = "diagnostic") && md.is_some() {
                hal_panic(format_args!(
                    "pmap_remove_ptes: managed page without PG_PVLIST: va {va:#x}, opte {opte:#x}"
                ));
            }
        } else {
            let Some(md) = md else {
                hal_panic(format_args!(
                    "pmap_remove_ptes: unmanaged page marked PG_PVLIST: va {va:#x}, opte {opte:#x}"
                ));
            };
            // sync R/M bits
            pmap_sync_flags_pte(md, opte);
            if let Some(mut pve) = pmap_remove_pv(md, pmap, va) {
                pve.next = free.take();
                *free = Some(pve);
            }
        }
        i += 1;
        va += PAGE_SIZE;
    }
}

/// `pmap_do_remove`: mapping removal guts (`flags` `PMAP_REMOVE_ALL`; caller should not be
/// holding any pmap locks).
fn pmap_do_remove(pmap: &Pmap, sva: usize, eva: usize, _flags: i32) {
    let pages = pmap.pages();
    let mut free: Option<PoolBox<PvEntry>> = None;
    let mut freed = FreePtps::new();

    let locked = pmap_map_ptes(pmap);
    let shootself = pmap_is_curpmap(pmap);
    let is_kernel = pmap.is_kernel();
    let root = pmap.md.root_pa();

    let shootall = (eva - sva > 32 * PAGE_SIZE) && sva < VM_MIN_KERNEL_ADDRESS;

    let mut va = sva;
    while va < eva {
        // determine range of block
        let blkendva = x86_round_pdr(va + 1).min(eva);

        // XXXCDC: our PTE mappings should never be removed with pmap_remove!
        if pl_i(va, PTP_LEVELS) == PDIR_SLOT_PTE {
            va = blkendva;
            continue;
        }

        let (level, table, offs) = find_pte_from(root, va);
        if level != 0 {
            va = blkendva;
            continue;
        }

        // get PTP if non-kernel mapping; we never free kernel PTPs
        let ptp = (!is_kernel).then(|| ptp_md(pages, table));
        pmap_remove_ptes(pmap, ptp, table, offs, va, blkendva, &mut free);

        // if PTP is no longer being used, free it!
        if let Some(p) = ptp
            && p.ptp_refs.load(Ordering::Relaxed) <= 1
        {
            pmap_free_ptp(pmap, &locked, table, va, &mut freed);
        }
        va = blkendva;
    }

    if shootall {
        pmap_tlb_shoottlb(pmap, shootself);
    } else {
        pmap_tlb_shootrange(pmap, sva, eva, shootself);
    }

    drop(locked);
    pmap_tlb_shootwait();

    // cleanup:
    free_pvs(free);
    freed.free(pages);
}

/// `pmap_page_remove`: remove a managed vm_page from all pmaps that map it. R/M bits are
/// sync'd back to attrs.
fn pmap_page_remove(md: &PageMd) {
    let mut freed = FreePtps::new();
    loop {
        let pmref = {
            let head = md.pv.lock();
            let Some(e) = head.0.as_deref() else {
                break;
            };
            pmap_reference_pv(e)
        };
        let pm: &Pmap = &pmref;
        let locked = pmap_map_ptes(pm); // locks pmap
        let shootself = pmap_is_curpmap(pm);

        // We dropped the pvlist lock before grabbing the pmap lock to avoid lock ordering
        // problems. This means we have to check the pvlist again since somebody else might
        // have modified it. All we care about is that the pvlist entry matches the pmap we
        // just locked. If it doesn't, unlock the pmap and try again.
        let mut head = md.pv.lock();
        if !head
            .0
            .as_deref()
            .is_some_and(|e| ptr::eq(e.pmap.as_ptr(), pm))
        {
            drop(head);
            drop(locked);
            continue;
        }
        let Some(mut pve) = head.0.take() else {
            continue;
        };
        head.0 = pve.next.take();
        drop(head);

        let va = pve.va;
        let (level, table, offs) = find_pte_from(pm.md.root_pa(), va);
        if cfg!(feature = "diagnostic") && pve.ptp.is_some_and(|p| level == 0 && p != table) {
            hal_panic(format_args!(
                "pmap_page_remove: mapped managed page has invalid pv_ptp field: va {va:#x}"
            ));
        }
        if level != 0 {
            // A linked entry's mapping has its level-1 table: the C faults on the PTE here.
            hal_panic(format_args!(
                "pmap_page_remove: no PTE for the mapping at {va:#x} (level {level})"
            ));
        }

        // atomically save the old PTE and zap it
        let opte = entry(table, offs).swap(0, Ordering::AcqRel);
        if opte & PG_W != 0 {
            add(&pm.stats.wired_count, -1);
        }
        add(&pm.stats.resident_count, -1);

        pmap_tlb_shootpage(pm, va, shootself);
        pmap_sync_flags_pte(md, opte);

        // update the PTP reference count. free if last reference.
        if let Some(ptp) = pve.ptp {
            let refs = &ptp_md(pm.pages(), ptp).ptp_refs;
            if refs.fetch_sub(1, Ordering::Relaxed) - 1 <= 1 {
                pmap_free_ptp(pm, &locked, ptp, va, &mut freed);
            }
        }
        drop(locked); // unlocks pmap
        drop(pve);
        // pmap_destroy(pm): outside both locks, as the drop may be the last.
        drop(pmref);
    }

    pmap_tlb_shootwait();
    if freed.n != 0 {
        // Every pmap has the kernel pmap's provider (`hal::pmap::create`).
        freed.free(KERNEL_PMAP.pages());
    }
}

/// `pmap_test_attrs`: whether the page or one of its mappings has a bit of `testbits`.
fn pmap_test_attrs(md: &PageMd, testbits: u64) -> bool {
    let testflags = pmap_pte2flags(testbits);
    if md.attrs.load(Ordering::Relaxed) & testflags != 0 {
        return true;
    }

    let mut mybits = 0;
    {
        let head = md.pv.lock();
        let mut cur = head.0.as_deref();
        while let Some(pve) = cur
            && mybits == 0
        {
            let (_level, table, offs) = find_pte_from(pve.pmap().md.root_pa(), pve.va);
            mybits |= entry(table, offs).load(Ordering::Relaxed) & testbits;
            cur = pve.next.as_deref();
        }
    }
    if mybits == 0 {
        return false;
    }
    md.attrs.fetch_or(pmap_pte2flags(mybits), Ordering::Relaxed);
    true
}

/// `pmap_clear_attrs`: clears `clearbits` in the page and its mappings; whether one was set.
fn pmap_clear_attrs(md: &PageMd, clearbits: u64) -> bool {
    let clearflags = pmap_pte2flags(clearbits);
    let mut result = md.attrs.fetch_and(!clearflags, Ordering::Relaxed) & clearflags != 0;
    {
        let head = md.pv.lock();
        let mut cur = head.0.as_deref();
        while let Some(pve) = cur {
            let pm = pve.pmap();
            let (_level, table, offs) = find_pte_from(pm.md.root_pa(), pve.va);
            let e = entry(table, offs);
            let opte = e.load(Ordering::Relaxed);
            if opte & clearbits != 0 {
                result = true;
                e.fetch_and(!(opte & clearbits), Ordering::AcqRel);
                pmap_tlb_shootpage(pm, pve.va, pmap_is_curpmap(pm));
            }
            cur = pve.next.as_deref();
        }
    }
    pmap_tlb_shootwait();
    result
}

/// `pmap_write_protect`: write-protect pages in a pmap.
fn pmap_write_protect(pmap: &Pmap, sva: usize, eva: usize, prot: VmProt) {
    let mut clear: u64 = 0;
    let mut set: u64 = 0;

    let locked = pmap_map_ptes(pmap);
    let shootself = pmap_is_curpmap(pmap);
    let root = pmap.md.root_pa();

    if prot & PROT_READ == 0 {
        set |= PG_XO;
    }
    if prot & PROT_WRITE == 0 {
        clear = PG_RW;
    }
    if prot & PROT_EXEC == 0 {
        set |= pg_nx();
    }

    let shootall = (eva - sva > 32 * PAGE_SIZE) && sva < VM_MIN_KERNEL_ADDRESS;

    let mut va = sva;
    while va < eva {
        let blkendva = x86_round_pdr(va + 1).min(eva);
        // XXXCDC: our PTE mappings should never be write-protected!
        if pl_i(va, PTP_LEVELS) == PDIR_SLOT_PTE {
            va = blkendva;
            continue;
        }
        let (level, table, offs) = find_pte_from(root, va);
        if level != 0 {
            va = blkendva;
            continue;
        }
        if cfg!(feature = "diagnostic") && (VM_MAXUSER_ADDRESS..VM_MAX_ADDRESS).contains(&va) {
            hal_panic(format_args!("pmap_write_protect: PTE space"));
        }
        // The C's count of entries, `pl1_i(blkendva) - pl1_i(va)`: an unaligned `va` still
        // covers every page the range touches.
        let n = pl_i(blkendva, 1) - pl_i(va, 1);
        for e in frame(table).iter().skip(offs).take(n) {
            if pmap_valid_entry(e.load(Ordering::Relaxed)) {
                e.fetch_and(!clear, Ordering::AcqRel);
                e.fetch_or(set, Ordering::AcqRel);
            }
        }
        va = blkendva;
    }

    if shootall {
        pmap_tlb_shoottlb(pmap, shootself);
    } else {
        pmap_tlb_shootrange(pmap, sva, eva, shootself);
    }
    drop(locked);
    pmap_tlb_shootwait();
}

/// `pmap_enter`: enter a mapping into a pmap. `ENOMEM` under `PMAP_CANFAIL` when a pv entry
/// or a page-table page cannot be had; a panic otherwise, as the C.
fn pmap_enter(
    pmap: &Pmap,
    va: usize,
    pa_raw: usize,
    prot: VmProt,
    flags: i32,
) -> Result<(), Errno> {
    let pages = pmap.pages();
    let wired = flags & PMAP_WIRED != 0;
    let crypt = flags & PMAP_NOCRYPT == 0;
    let nocache = pa_raw & PMAP_NOCACHE as usize != 0;
    let mut wc = pa_raw & PMAP_WC as usize != 0;
    let is_kernel = pmap.is_kernel();

    kassert!(!(wc && nocache));
    if flags & PMAP_EFI != 0 {
        hal_panic(format_args!(
            "pmap_enter: PMAP_EFI (amd64's EFI runtime pmap is not ported)"
        ));
    }
    let pa = pa_raw & PMAP_PA_MASK;

    if cfg!(feature = "diagnostic") {
        if va == super::layout::PDP_BASE {
            hal_panic(format_args!("pmap_enter: trying to map over PDP!"));
        }
        // sanity check: kernel PTPs should already have been pre-allocated
        if va >= VM_MIN_KERNEL_ADDRESS
            && !pmap_valid_entry(
                frame(pmap.md.root_pa())[pl_i(va, PTP_LEVELS) % NTOPLEVEL_PDES]
                    .load(Ordering::Relaxed),
            )
        {
            hal_panic(format_args!(
                "pmap_enter: missing kernel PTP for va {va:#x}!"
            ));
        }
    }

    let new_pve = PV_POOL.get(PR_NOWAIT, || PvEntry {
        next: None,
        pmap: NonNull::from(pmap),
        va,
        ptp: None,
    });
    let Some(mut pve) = new_pve else {
        if flags & PMAP_CANFAIL != 0 {
            return Err(Errno::ENOMEM);
        }
        hal_panic(format_args!("pmap_enter: no pv entries available"));
    };
    let mut opve: Option<PoolBox<PvEntry>> = None;

    // map in ptes and get our PTP (unless we are the kernel)
    let locked = pmap_map_ptes(pmap);
    let shootself = pmap_is_curpmap(pmap);
    let ptp = if is_kernel {
        None
    } else {
        match pmap_get_ptp(pmap, &locked, va) {
            Some(p) => Some(p),
            None => {
                if flags & PMAP_CANFAIL != 0 {
                    drop(locked);
                    return Err(Errno::ENOMEM);
                }
                hal_panic(format_args!("pmap_enter: get ptp failed"));
            }
        }
    };
    let (level, table, offs) = find_pte_from(pmap.md.root_pa(), va);
    if level != 0 {
        hal_panic(format_args!("pmap_enter: no page table for va {va:#x}"));
    }
    let pte = entry(table, offs);
    let opte = pte.load(Ordering::Relaxed); // old PTE
    let ptp_refs = ptp.map(|p| &ptp_md(pages, p).ptp_refs);

    let resdelta: i64;
    let wireddelta: i64;
    let ptpdelta: u32;
    let mut pg: Option<&'static PageMd> = None;
    let mut enter_now = false;
    // is there currently a valid mapping at our VA?
    if pmap_valid_entry(opte) {
        // resident count will not change since we are replacing/changing a valid mapping;
        // wired count might change...
        resdelta = 0;
        wireddelta = match (wired, opte & PG_W != 0) {
            (true, false) => 1,
            (false, true) => -1,
            _ => 0,
        };
        ptpdelta = 0;

        // is the currently mapped PA the same as the one we want to map?
        if (opte & pg_frame()) as usize == pa {
            // if this is on the PVLIST, sync R/M bit
            if opte & PG_PVLIST != 0 {
                pg = pages.md_of(Paddr::new(pa));
                let Some(p) = pg else {
                    hal_panic(format_args!(
                        "pmap_enter: same pa, PG_PVLIST mapping with unmanaged page: \
                         va {va:#x}, opte {opte:#x}, pa {pa:#x}"
                    ));
                };
                pmap_sync_flags_pte(p, opte);
            } else if cfg!(feature = "diagnostic") && pages.md_of(Paddr::new(pa)).is_some() {
                hal_panic(format_args!(
                    "pmap_enter: same pa, no PG_PVLIST mapping with managed page: \
                     va {va:#x}, opte {opte:#x}, pa {pa:#x}"
                ));
            }
            enter_now = true;
        } else if opte & PG_PVLIST != 0 {
            // changing PAs: we must remove the old one first (sync R/M bits)
            let Some(opg) = pages.md_of(entry_pa(opte)) else {
                hal_panic(format_args!(
                    "pmap_enter: PG_PVLIST mapping with unmanaged page: \
                     va {va:#x}, opte {opte:#x}, pa {pa:#x}"
                ));
            };
            pmap_sync_flags_pte(opg, opte);
            opve = pmap_remove_pv(opg, pmap, va);
        }
    } else {
        // opte not valid
        resdelta = 1;
        wireddelta = i64::from(wired);
        ptpdelta = u32::from(ptp.is_some());
    }

    // if this entry is to be on a pvlist, enter it now; the entry left unused is freed at
    // the end.
    let pve_unused = if enter_now {
        Some(pve)
    } else {
        if PMAP_INITIALIZED.load(Ordering::Relaxed) {
            pg = pages.md_of(Paddr::new(pa));
        }
        if let Some(p) = pg {
            pve.ptp = ptp;
            pmap_enter_pv(p, pve);
            None
        } else {
            Some(pve)
        }
    };

    // enter_now: at this point pg is Some if we want the PG_PVLIST bit set
    add(&pmap.stats.resident_count, resdelta);
    add(&pmap.stats.wired_count, wireddelta);
    if let Some(r) = ptp_refs {
        r.fetch_add(ptpdelta, Ordering::Relaxed);
    }

    let mut npte = pa as u64 | protection_code(prot, pg_nx(), PG_XO) | PG_V;
    if let Some(p) = pg {
        npte |= PG_PVLIST;
        // make sure that if the page is write combined all instances of pmap_enter make it
        // so.
        if p.attrs.load(Ordering::Relaxed) & PG_PMAP_WC != 0 {
            kassert!(!nocache);
            wc = true;
        }
    }
    if wc {
        npte |= PMAP_PG_WC;
    }
    if wired {
        npte |= PG_W;
    }
    if nocache {
        npte |= PG_N;
    }
    if va < VM_MAXUSER_ADDRESS {
        npte |= PG_u; // PMAP_EFI was refused above
    } else if va < VM_MAX_ADDRESS {
        npte |= PG_u | PG_RW; // XXXCDC: no longer needed?
    }
    if is_kernel {
        npte |= pg_g_kern();
    }
    if crypt {
        npte |= pg_crypt();
    }

    // If the old entry wasn't valid, we can just update it and go. If it was valid, and this
    // isn't a read->write transition, then we can safely just update it and flush any old TLB
    // entries. If it _was_ valid and this _is_ a read->write transition, then this could be a
    // CoW resolution and we need to make sure no CPU can see the new writable mapping while
    // another still has the old mapping in its TLB, so insert a correct but unwritable
    // mapping, flush any old TLB entries, then make it writable.
    if !pmap_valid_entry(opte) {
        pte.store(npte, Ordering::Release);
    } else if (opte | (npte ^ PG_RW)) & PG_RW != 0 {
        // previously writable or not making writable
        pte.store(npte, Ordering::Release);
        if nocache && opte & PG_N == 0 {
            wbinvd_on_all_cpus();
        }
        pmap_tlb_shootpage(pmap, va, shootself);
    } else {
        pte.store(npte ^ PG_RW, Ordering::Release);
        if nocache && opte & PG_N == 0 {
            // XXX impossible?
            wbinvd_on_all_cpus();
        }
        pmap_tlb_shootpage(pmap, va, shootself);
        pmap_tlb_shootwait();
        pte.store(npte, Ordering::Release);
    }

    drop(locked);
    pmap_tlb_shootwait();

    // out:
    drop(pve_unused);
    drop(opve);

    Ok(())
}

/// `pmap_kenter_pa`'s leaf: `pa` (with its `PMAP_NOCACHE`/`PMAP_WC`/`PMAP_NOCRYPT` flag bits)
/// at `va`, wired, unmanaged.
fn kenter_pte(va: usize, pa: usize, prot: VmProt) -> u64 {
    let mut npte = (pa & PMAP_PA_MASK) as u64
        | if prot & PROT_WRITE != 0 { PG_RW } else { PG_RO }
        | if pa & PMAP_NOCACHE as usize != 0 {
            PG_N
        } else {
            0
        }
        | if pa & PMAP_WC as usize != 0 {
            PMAP_PG_WC
        } else {
            0
        }
        | PG_V
        | if pa & PMAP_NOCRYPT as usize != 0 {
            0
        } else {
            pg_crypt()
        };
    // special 1:1 mappings in the first 2MB must not be global
    if va >= NBPD_L2 {
        npte |= pg_g_kern();
    }
    if prot & PROT_EXEC == 0 {
        npte |= pg_nx();
    }
    npte
}

/// The level-1 entry of the kernel's `va`; a panic where its page table does not exist (the
/// C's `kvtopte` faults there).
fn kernel_pte(va: usize) -> &'static AtomicU64 {
    let (level, table, offs) = find_pte_from(KERNEL_PMAP.md.root_pa(), va);
    if level != 0 {
        hal_panic(format_args!("pmap_kenter_pa: no page table for va {va:#x}"));
    }
    entry(table, offs)
}

/// `pmap_find_pte_direct(pmap_kernel(), va, ..)` as the C's callers use it outside the pmap
/// (the ISA hole check of `bus_space`): the level the walk stopped at (0 for a level-1 entry)
/// and the entry there. The kernel pmap only: its tables are never freed, so the walk needs
/// no lock (a user pmap's would need its `pm_mtx`, and no caller walks one).
pub fn find_pte(va: Vaddr) -> (usize, u64) {
    let (level, table, offs) = find_pte_from(KERNEL_PMAP.md.root_pa(), va.as_usize());
    (level, entry(table, offs).load(Ordering::Relaxed))
}

impl KernelMapper {
    /// `pmap_prealloc_lowmem_ptps`: the PTPs that map the first 2 MB in the kernel pmap, taken
    /// from the low pages at `first_avail` (three: levels 3, 2 and 1), so that the trampoline
    /// code can be entered (the application processors' `mptramp.S`); the next free page.
    /// Only after a boot by boot(8): Limine starts the processors itself. The three pages
    /// are the caller's (Reserved in the boot memory map), below 4 GB, inside the direct map.
    pub fn prealloc_lowmem_ptps(&self, first_avail: Paddr) -> Paddr {
        let mut next = first_avail.as_usize();
        let mut table = KERNEL_PMAP.md.root_pa();
        for level in (2..=PTP_LEVELS).rev() {
            let newp = Paddr::new(next);
            next += PAGE_SIZE;
            zero_frame(newp);
            entry(table, pl_pi(0, level)).store(
                (newp.as_usize() as u64 & pg_frame()) | PG_V | PG_RW | pg_crypt(),
                Ordering::Release,
            );
            table = newp;
        }
        Paddr::new(next)
    }
}

impl PmapBackend for Backend {
    const HAVE_PMAP_DIRECT: bool = true;
    const PMAP_STEAL_MEMORY: bool = true;
    const PMAP_WC: usize = PMAP_WC as usize;
    const PMAP_NOCACHE: usize = PMAP_NOCACHE as usize;
    const PMAP_NOMMU: bool = false;
    const POOL_WCHAN: &'static str = "pmappl";
    const POOL_IPL: i32 = IPL_VM;
    const POOL_FLAGS: i32 = 0;

    fn kernel() -> &'static Pmap {
        &KERNEL_PMAP
    }

    /// `pmap_bootstrap`: get the system in a state where it can run with VM properly enabled
    /// (called before `main()`): the kernel pmap adopts `%cr3`'s PML4 (Limine's, or
    /// `locore0.S`'s after boot(8)), the recursive slot is installed or checked, the module
    /// window linked. Allocates nothing.
    fn bootstrap(p: &BootstrapParams) -> (Paddr, Vaddr) {
        set_direct_map(p.direct_base, p.direct_end);
        let kva_start = VM_MIN_KERNEL_ADDRESS;
        // define the boundaries of the managed kernel virtual address space: one whole PML4
        // slot, the first after the direct map when it overlaps the C's (see the deviations).
        let (direct_base, direct_end) = (p.direct_base.as_usize(), p.direct_end.as_usize());
        let Some((virtual_avail, virtual_end)) = kernel_window(direct_base, direct_end) else {
            hal_panic(format_args!(
                "pmap_bootstrap: no kernel virtual space beside the direct map \
                 [{direct_base:#x}, {direct_end:#x})"
            ));
        };
        VIRTUAL_AVAIL.store(virtual_avail, Ordering::Relaxed); // first free KVA
        PMAP_KVA_START.store(virtual_avail, Ordering::Relaxed);
        PMAP_KVA_END.store(virtual_end, Ordering::Relaxed);
        PMAP_MAXKVADDR.store(virtual_avail, Ordering::Relaxed);

        // pg_nx: whether the boot loader enabled NX (the C's locore0.S does, after CPUID).
        if p.nx {
            PG_NX_BIT.store(PG_NX, Ordering::Relaxed);
        }

        // now we init the kernel's pmap
        let kpm = &KERNEL_PMAP;
        let pdirpa = Paddr::new(rcr3() as usize & PMAP_PA_MASK);
        kpm.md
            .root
            .store(pdirpa.as_usize() as u64, Ordering::Release);
        let resident = atop(kva_start - VM_MIN_KERNEL_ADDRESS) as i64;
        kpm.stats.wired_count.set(resident);
        kpm.stats.resident_count.set(resident);
        // the above is just a rough estimate and not critical to the proper operation of the
        // system.

        // The recursive mapping: the C's locore0.S installs it, and so does ours after a boot
        // by boot(8); under Limine it is installed here (see the deviations).
        let slot = entry(pdirpa, PDIR_SLOT_PTE);
        let pde = slot.load(Ordering::Relaxed);
        if !pmap_valid_entry(pde) {
            slot.store(
                pdirpa.as_usize() as u64 | PG_V | PG_RW | pg_nx() | pg_crypt(),
                Ordering::Release,
            );
        } else if pde & pg_frame() != pdirpa.as_usize() as u64 {
            hal_panic(format_args!(
                "pmap_bootstrap: PML4 slot {PDIR_SLOT_PTE} is in use"
            ));
        }
        tlbflush();

        window::init();

        (p.first_avail, Vaddr::new(virtual_avail))
    }

    /// `pmap_init`: the pv entries' pool (the C's `pmap_bootstrap` makes it with the pmap
    /// pool, `hal::pmap::init`'s).
    fn init() {
        if PV_POOL.init(IPL_VM, 0, "pvpl").is_err() {
            hal_panic(format_args!("pmap_init: the pv pool"));
        }
        PMAP_INITIALIZED.store(true, Ordering::Release);
    }

    fn initialized() -> bool {
        PMAP_INITIALIZED.load(Ordering::Acquire)
    }

    /// `pmap_create`'s machine part: the PDP (counted in `resident_count`, as the C), with the
    /// recursive entry and the kernel half.
    /// A user pmap's limit: `VM_MAXUSER_ADDRESS` (the recursive slot starts right above it).
    fn va_limit(_pmap: &Pmap) -> Vaddr {
        Vaddr::new(VM_MAXUSER_ADDRESS)
    }

    fn create(stats: &PmapStatistics, pages: &'static dyn PhysPages) -> Option<PmapMd> {
        let pdirpa = pages.alloc_ptp()?;
        pmap_pdp_ctor(pdirpa);
        stats.wired_count.set(0);
        stats.resident_count.set(1); // count the PDP allocd below
        let md = PmapMd::new();
        md.root.store(pdirpa.as_usize() as u64, Ordering::Release);
        md.ptps.store(1, Ordering::Relaxed);
        USER_PMAP_CREATED.store(true, Ordering::Release);
        Some(md)
    }

    fn mappings(pmap: &Pmap) -> usize {
        let tables = pmap.md.ptps.load(Ordering::Relaxed);
        usize::try_from(pmap.stats.resident_count.get() - tables).unwrap_or(0)
    }

    /// `pmap_release`: frees the remaining PTPs of the user half (walked top-down: the walk
    /// is the index) and the PDP. No reference to the pmap is left (this is its last
    /// handle's drop, and `pmap_page_remove` holds a counted one).
    fn destroy(pmap: &mut Pmap) {
        let Some(&pages) = pmap.pages.get() else {
            return;
        };
        let root = pmap.md.root_pa();
        if root.as_usize() == 0 {
            return;
        }
        // A loaded root is never freed: not this CPU's `%cr3`, not another running CPU's pmap.
        if rcr3() & pg_frame() == root.as_usize() as u64 {
            hal_panic(format_args!("pmap_destroy: the pmap is loaded on this CPU"));
        }
        other_cpus_off(pmap);
        for e4 in frame(root).iter().take(PDIR_SLOT_PTE) {
            let p4 = e4.load(Ordering::Relaxed);
            if !pmap_valid_entry(p4) {
                continue;
            }
            for e3 in frame(entry_pa(p4)) {
                let p3 = e3.load(Ordering::Relaxed);
                if !pmap_valid_entry(p3) {
                    continue;
                }
                for e2 in frame(entry_pa(p3)) {
                    let p2 = e2.load(Ordering::Relaxed);
                    if pmap_valid_entry(p2) {
                        free_table(pages, entry_pa(p2));
                    }
                }
                free_table(pages, entry_pa(p3));
            }
            free_table(pages, entry_pa(p4));
        }
        pages.free_ptp(root);
        pmap.md.root.store(0, Ordering::Release);
    }

    fn enter(pmap: &Pmap, va: Vaddr, pa: Paddr, prot: VmProt, flags: i32) -> Result<(), Errno> {
        pmap_enter(pmap, va.as_usize(), pa.as_usize(), prot, flags)
    }

    fn remove(pmap: &Pmap, sva: Vaddr, eva: Vaddr) {
        if sva < eva {
            pmap_do_remove(pmap, sva.as_usize(), eva.as_usize(), PMAP_REMOVE_ALL);
        }
    }

    /// `pmap_protect`: a frontend for `pmap_remove`/`pmap_write_protect`; we only have to
    /// worry about making the page more protected.
    fn protect(pmap: &Pmap, sva: Vaddr, eva: Vaddr, prot: VmProt) {
        if sva >= eva {
            return;
        }
        if prot != PROT_NONE {
            pmap_write_protect(pmap, sva.as_usize(), eva.as_usize(), prot);
        } else {
            pmap_do_remove(pmap, sva.as_usize(), eva.as_usize(), PMAP_REMOVE_ALL);
        }
    }

    /// `pmap_extract`: extract a PA for the given VA.
    fn extract(pmap: &Pmap, va: Vaddr) -> Option<Paddr> {
        if pmap.is_kernel()
            && let Some(pa) = direct_unmap(va)
        {
            return Some(pa);
        }
        let _locked = pmap_map_ptes(pmap);
        let va = va.as_usize();
        let (level, table, offs) = find_pte_from(pmap.md.root_pa(), va);
        leaf_pa(level, entry(table, offs).load(Ordering::Relaxed), va)
    }

    /// `pmap_unwire`: clear the wired bit in the PTE (the C's checks under `DIAGNOSTIC`).
    fn unwire(pmap: &Pmap, va: Vaddr) {
        let _locked = pmap_map_ptes(pmap);
        let va = va.as_usize();
        let (level, table, offs) = find_pte_from(pmap.md.root_pa(), va);
        if level != 0 {
            if cfg!(feature = "diagnostic") {
                hal_panic(format_args!("pmap_unwire: invalid PDE"));
            }
            return;
        }
        let e = entry(table, offs);
        let pte = e.load(Ordering::Relaxed);
        if cfg!(feature = "diagnostic") && !pmap_valid_entry(pte) {
            hal_panic(format_args!("pmap_unwire: invalid (unmapped) va {va:#x}"));
        }
        if pte & PG_W != 0 {
            e.fetch_and(!PG_W, Ordering::AcqRel);
            add(&pmap.stats.wired_count, -1);
        } else if cfg!(feature = "diagnostic") {
            let _ = core::fmt::Write::write_fmt(
                &mut crate::cons::Cons,
                format_args!(
                    "pmap_unwire: wiring for pmap {:p} va {va:#x} didn't change!\n",
                    pmap.as_ptr()
                ),
            );
        }
    }

    /// `pmap_kenter_pa`: enter a kernel mapping without R/M (pv_entry) tracking. No need to
    /// lock anything, assume va is already allocated.
    fn kenter(va: Vaddr, pa: Paddr, prot: VmProt) {
        let (va, pa) = (va.as_usize(), pa.as_usize());
        let npte = kenter_pte(va, pa, prot);
        let opte = kernel_pte(va).swap(npte, Ordering::AcqRel);
        if pmap_valid_entry(opte) {
            if (pa & PMAP_NOCACHE as usize != 0 && opte & PG_N == 0)
                || pa & PMAP_NOCRYPT as usize != 0
            {
                wbinvd_on_all_cpus();
            }
            // This shouldn't happen
            pmap_tlb_shootpage(&KERNEL_PMAP, va, true);
            pmap_tlb_shootwait();
        }
    }

    /// `pmap_kremove`: remove kernel mappings without R/M (pv_entry) tracking; the kernel only
    /// unmaps valid addresses, so the valid bit is not checked before the TLB flush.
    fn kremove(va: Vaddr, len: Vsize) {
        let sva = va.as_usize();
        let eva = sva + len.as_usize();
        for va in (sva..eva).step_by(PAGE_SIZE) {
            let opte = kernel_pte(va).swap(0, Ordering::AcqRel);
            kassert!(opte & PG_PVLIST == 0);
        }
        pmap_tlb_shootrange(&KERNEL_PMAP, sva, eva, true);
        pmap_tlb_shootwait();
    }

    /// `pmap_activate`'s switch: `lcr3` of the pmap's root (PCID off); a pmap without a root
    /// (the kernel pmap before the bootstrap) panics.
    fn load(pmap: &Pmap) {
        if pmap.md.root() == 0 {
            hal_panic(format_args!("pmap_activate: a pmap without a root"));
        }
        lcr3(pmap);
    }

    /// amd64 counts no activations (`ci_proc_pmap` is the caller's).
    fn activate(_pmap: &Pmap) {}

    /// amd64 counts no activations.
    fn deactivate(_pmap: &Pmap) {}

    /// No `__HAVE_PMAP_PURGE` on amd64.
    fn purge(_pmap: &Pmap) {}

    /// Nothing on amd64: the instruction cache is coherent.
    fn proc_iflush(_pmap: &Pmap, _va: Vaddr, _len: Vsize) {}

    /// `pmap_page_protect`: a frontend for `pmap_page_remove`/`pmap_clear_attrs`.
    fn page_protect(md: &PageMd, prot: VmProt) {
        if prot == PROT_READ {
            let _ = pmap_clear_attrs(md, PG_RW);
        } else {
            kassert!(prot == PROT_NONE);
            pmap_page_remove(md);
        }
    }

    fn clear_modify(md: &PageMd) -> bool {
        pmap_clear_attrs(md, PG_M)
    }

    fn clear_reference(md: &PageMd) -> bool {
        pmap_clear_attrs(md, PG_U)
    }

    fn is_modified(md: &PageMd) -> bool {
        pmap_test_attrs(md, PG_M)
    }

    fn is_referenced(md: &PageMd) -> bool {
        pmap_test_attrs(md, PG_U)
    }

    /// `pmap_zero_page`: `pagezero` through the direct map.
    fn zero_page(pa: Paddr) {
        zero_frame(pa);
    }

    /// `pmap_copy_page`: through the direct map.
    fn copy_page(src: Paddr, dst: Paddr) {
        copy_frame(src, dst);
    }

    /// `pmap_steal_memory`: takes `size` bytes out of the page system, zeroed, through the
    /// direct map.
    fn steal_memory(pages: &'static dyn PhysPages, size: Vsize) -> Vaddr {
        let size = round_page(size.as_usize());
        let npg = atop(size);
        if npg == 0 {
            // No page taken would still hand out an address (the front refuses the sizes
            // that round to none; this keeps the backend's own contract).
            hal_panic(format_args!("pmap_steal_memory: no page"));
        }
        let Some(pa) = pages.steal(npg) else {
            hal_panic(format_args!("pmap_steal_memory: out of memory"));
        };
        for i in 0..npg {
            zero_frame(Paddr::new(pa.as_usize() + i * PAGE_SIZE));
        }
        direct_map(pa)
    }

    /// `pmap_virtual_space`: the free kernel virtual range (see the deviations).
    fn virtual_space() -> (Vaddr, Vaddr) {
        (
            Vaddr::new(VIRTUAL_AVAIL.load(Ordering::Relaxed)),
            Vaddr::new(kernel_virtual_end()),
        )
    }

    /// `pmap_growkernel`: increase usage of KVM space: new PTPs for the kernel.
    fn growkernel(maxkvaddr: Vaddr) -> Vaddr {
        let cur = PMAP_MAXKVADDR.load(Ordering::Relaxed);
        if maxkvaddr.as_usize() <= cur {
            return Vaddr::new(cur);
        }
        let maxkvaddr = x86_round_pdr(maxkvaddr.as_usize());
        let kva_start = PMAP_KVA_START.load(Ordering::Relaxed);
        // This loop could be optimized more, but pmap_growkernel() is called infrequently.
        let mut needed_kptp = [0isize; PTP_LEVELS];
        for i in (1..PTP_LEVELS).rev() {
            let target_nptp = pl_i(maxkvaddr, i + 1) - pl_i(kva_start, i + 1);
            // XXX only need to check toplevel.
            if target_nptp > NKPTPMAX_INITIALIZER[i] {
                hal_panic(format_args!("pmap_growkernel: out of KVA space"));
            }
            needed_kptp[i] = target_nptp as isize - NKPTP[i].load(Ordering::Relaxed) as isize + 1;
        }
        if needed_kptp[PTP_LEVELS - 1] != 0 && USER_PMAP_CREATED.load(Ordering::Acquire) {
            // No pmaps list: the new top-level entry would be missing from every user pmap.
            hal_panic(format_args!(
                "pmap_growkernel: a new top-level entry after the first user pmap"
            ));
        }

        let _s = crate::intr::splhigh(); // to be safe
        pmap_alloc_level(cur, PTP_LEVELS, &needed_kptp);
        PMAP_MAXKVADDR.store(maxkvaddr, Ordering::Relaxed);
        Vaddr::new(maxkvaddr)
    }

    fn direct_map(pa: Paddr) -> Vaddr {
        direct_map(pa)
    }

    fn direct_unmap(va: Vaddr) -> Option<Paddr> {
        direct_unmap(va)
    }

    /// `pmap_flush_cache(addr, len)`: `clflush` each line between two `mfence`s, or `wbinvd`
    /// on every CPU when the CPU has no `clflush` (`ci_cflushsz` 0).
    fn flush_cache(va: Vaddr, len: Vsize) {
        let sz = cflushsz();
        if sz == 0 {
            wbinvd_on_all_cpus();
            return;
        }
        // all cpus that have clflush also have mfence.
        mfence();
        let start = va.as_usize();
        for line in (start..start + len.as_usize()).step_by(sz) {
            clflush(line);
        }
        mfence();
    }

    fn module_window() -> Option<(Vaddr, usize)> {
        Some(window::window())
    }

    fn module_enter(va: Vaddr, pa: Paddr, prot: VmProt) -> bool {
        window::enter(va, pa, prot)
    }

    /// `pcb_cr3`: the root's physical address (PCID off: `cr3_pcid_proc` is 0).
    fn cr3(pmap: &Pmap) -> u64 {
        pmap.md.root()
    }
}
/* </CODE> */
