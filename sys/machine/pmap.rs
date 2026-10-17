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
/* </LICENSES> */

/* <CODE> */
//! `<machine/pmap.h>` and the machine-dependent half of `<uvm/uvm_pmap.h>`: the physical map,
//! what `uvm` asks the MMU code to do, over `hal::pmap` (N3.1).
//!
//! The pmaps themselves are `hal`'s (`sys/hal/pmap.rs`, the backends under
//! `sys/hal/arch/<machine>/pmap/`); this module is `bsd`'s adapter to them, so that the
//! callers of the contract keep its free functions by name: `pmap_kernel`, `pmap_enter`,
//! `pmap_page_protect(pg, ..)`, `pmap_activate(p)` and the rest. What changed shape:
//!
//! - A user pmap is a [`PmapRef`], `hal`'s counted handle: [`pmap_create`] makes one,
//!   [`pmap_reference`] clones it, [`pmap_destroy`] drops it (the last drop frees the page
//!   tables, and panics while a mapping is left, where the C freed silently). The kernel
//!   pmap is `PmapRef::Kernel`; [`pmap_kernel`] still answers `&'static MachinePmap`.
//! - The kernel pmap changes only through `hal::pmap::KernelMapper`, the token
//!   `pmap_bootstrap` hands out once; [`pmap_bootstrap`] (the machine's boot glue) gives it to
//!   its owner, the kernel's allocator (`kernel::vm::km::adopt_mapper`, N3.2), which lends it
//!   back here, to the holder of the one `kernel::vm::km::LegacyLend` (a private static:
//!   `kernel::vm::km::legacy_mapper` takes it), for [`pmap_enter`]/[`pmap_remove`]/
//!   [`pmap_protect`] on the kernel pmap and the two legacy `unsafe fn`s
//!   ([`pmap_kenter_pa`], [`pmap_kremove`], with arm64's [`pmap_kenter_cache`]), whose
//!   callers keep their `unsafe` blocks until their owners move (N3.5 `uvm_glue`, N4, N7).
//!   Their bodies hold no `unsafe` block. `pmap_steal_memory`'s wrapper went with its caller,
//!   `uvm_page` (N3.3): the kernel's boot allocations are `hal::bootmem`'s.
//! - Loading a pmap's root takes `hal::pmap::Switcher`, the other token: private here too,
//!   used only by [`pmap_activate`] and the machine's switch path ([`pmap_setttb`]), each of
//!   which records the switch (`ci_proc_pmap`, `ci_curpm`) where the shootdowns and the ASID
//!   allocator look, through the machine's hooks of [`Pmap`].
//! - The page allocator the pmap needs (`uvm_pagealloc`, `uvm_pagefree`, `PHYS_TO_VM_PAGE`,
//!   `uvm_page_physsteal`, `uvm.page_init_done`) is the kernel's page system's
//!   `hal::pmap::PhysPages`, `kernel::vm::page::PHYS_PAGES` (N3.3), given to
//!   `hal::pmap::bootstrap` as a value (D16).
//! - [`Pmap`] is what stays machine-dependent in `bsd`: the constants and the machine's own
//!   steps around `hal` (arm64's console remap in `pmap_init`, the switch records).
//!
//! The host double (`sys/arch/host`) runs the same adapter over `hal`'s host backend; having
//! no boot path, it bootstraps the pmap on first use, once per process
//! ([`Pmap::pmap_bootstrap_lazy`]).

use core::sync::atomic::{AtomicBool, Ordering};

use hal::once::OnceStatic;
use hal::pmap::{BootstrapParams, KernelMapper, Switcher};
pub use hal::pmap::{KernelImage, PmapRef};
use kernel::vm::km::LegacyLend;

use crate::kern::subr_prf::panic;
use crate::machine::Machine;
use crate::sys::errno::Errno;
use crate::sys::proc::{Proc, Process};
use crate::sys::types::{Paddr, Vaddr, Vsize};
use crate::uvm::uvm_extern::{UvmConstraintRange, VmProt, Vmspace};
use crate::uvm::uvm_page::{PHYS_TO_VM_PAGE, VmPage, vm_page_to_phys};

/// `struct vm_page_md`: the pmap's per-page data inside `struct vm_page` (`hal`'s).
pub type VmPageMd = hal::pmap::PageMd;

/// `struct pmap` (what a `pmap_t` points at; `hal`'s).
pub type MachinePmap = hal::pmap::Pmap;

/// What stays machine-dependent of the physical map in `bsd`: the constants and the
/// machine's steps around `hal::pmap`.
pub trait Pmap {
    /// `__HAVE_PMAP_DIRECT`: physical memory is direct-mapped, so `pmap_map_direct` works.
    const HAVE_PMAP_DIRECT: bool;
    /// `PMAP_STEAL_MEMORY`: `uvm_pageboot_alloc` defers to `pmap_steal_memory`.
    const PMAP_STEAL_MEMORY: bool;
    /// `uvm_md_constraints[]`: the DMA constraint ranges of the machine, lowest first.
    const UVM_MD_CONSTRAINTS: &'static [&'static UvmConstraintRange];
    /// `dma_constraint`: the range every DMA-capable device can reach.
    const DMA_CONSTRAINT: &'static UvmConstraintRange;
    /// `PMAP_WC`: the physical-address flag of a write-combining mapping (0 where the pmap
    /// has none, as `<uvm/uvm_pmap.h>` defaults it).
    const PMAP_WC: usize;
    /// `PMAP_NOCACHE`: the physical-address flag of an uncached mapping (a frame buffer's
    /// `mmap`, M13; 0 where the pmap has none).
    const PMAP_NOCACHE: usize;
    /// No MMU behind the pmap: only the direct map is addressable (the `host` test double,
    /// whose "physical" pages are the test process's memory). `km_alloc` and `kmeminit` then
    /// serve every request through the direct map. False on every real machine.
    const PMAP_NOMMU: bool;

    /// `pmap_init`'s machine sequence: `hal::pmap::init()` and what the machine does around
    /// it (arm64: the console's remap, then the lower half's translation registers).
    fn pmap_init_md();

    /// The free kernel virtual range `pmap_steal_memory` reports to its caller (amd64 and
    /// the host: `pmap_virtual_space`'s; arm64: the bounds without `pmap_virtual_space`'s
    /// side effect, which would stop the vp tables being stolen).
    fn pmap_steal_bounds() -> (Vaddr, Vaddr);

    /// `pmap_activate(p)`'s records before any switch (amd64: `pcb_pmap`, `pcb_cr3` and, for
    /// the running thread of a user process, `ci_proc_pmap`; arm64: `pm_active`); whether
    /// `pm` is to be loaded on this CPU now.
    fn pmap_activate_md(p: &Proc, pm: &MachinePmap) -> bool;

    /// The record of a switch, right after `pm`'s root was loaded on this CPU (arm64:
    /// `ci_curpm` and the branch-predictor flush; nothing on amd64).
    fn pmap_switched_md(pm: &MachinePmap);

    /// `pmap_deactivate(p)` on the running CPU (amd64: `ci_proc_pmap` cleared; arm64:
    /// `TTBR0_EL1` back to the kernel's empty table, `pm_active` one less).
    fn pmap_deactivate_md(p: &Proc, pm: &MachinePmap);

    /// `pmap_purge(p)` (`__HAVE_PMAP_PURGE`, arm64; nothing elsewhere).
    fn pmap_purge_md(p: &Proc, pm: &MachinePmap);

    /// Bootstraps the pmap on first use where no boot path does (the host double, once per
    /// process); nothing on a machine, whose boot glue calls [`pmap_bootstrap`].
    fn pmap_bootstrap_lazy();
}

/// The lend of the kernel map's token, from `kernel::vm::km::adopt_mapper`: only this module
/// can borrow the token.
static LEND: OnceStatic<LegacyLend> = OnceStatic::new();

/// The context switch's token, from `pmap_bootstrap`.
static SWITCHER: OnceStatic<Switcher> = OnceStatic::new();

/// Whether [`pmap_bootstrap`] ran (the lazy path's quick test).
static BOOTSTRAPPED: AtomicBool = AtomicBool::new(false);

/// Bootstraps the pmap where the machine's boot path has not (the host double).
fn booted() {
    if !BOOTSTRAPPED.load(Ordering::Acquire) {
        Machine::pmap_bootstrap_lazy();
    }
}

/// The kernel map's token, lent by its owner (`kernel::vm::km`); panics before
/// [`pmap_bootstrap`].
fn mapper() -> &'static KernelMapper {
    booted();
    match LEND.get() {
        Some(lend) => kernel::vm::km::legacy_mapper(lend),
        None => panic(format_args!(
            "pmap: the kernel pmap used before pmap_bootstrap"
        )),
    }
}

/// The context switch's token; panics before [`pmap_bootstrap`].
fn switcher() -> &'static Switcher {
    booted();
    match SWITCHER.get() {
        Some(s) => s,
        None => panic(format_args!("pmap: a pmap loaded before pmap_bootstrap")),
    }
}

/// `pmap_bootstrap`, the boot glue's call (once, on the boot CPU, before anything maps):
/// `hal::pmap::bootstrap` with the boot glue's facts and the kernel's page provider; the
/// kernel map's token goes to the kernel's allocator (`kernel::vm::km::adopt_mapper`), the
/// context switch's is kept here. The first free physical page and the first free kernel
/// virtual address (amd64's and arm64's answers).
pub fn pmap_bootstrap(params: &BootstrapParams) -> (Paddr, Vaddr) {
    let b = hal::pmap::bootstrap(params, &kernel::vm::page::PHYS_PAGES);
    let lent = kernel::vm::km::adopt_mapper(b.mapper).is_ok_and(|lend| LEND.set(lend).is_ok());
    if !lent || SWITCHER.set(b.switcher).is_err() {
        panic(format_args!("pmap_bootstrap: the tokens were kept already"));
    }
    BOOTSTRAPPED.store(true, Ordering::Release);
    (b.first_avail, b.virtual_avail)
}

/// `pmap_prealloc_lowmem_ptps` (amd64, after a boot by boot(8)): the PTPs that map the
/// first 2 MB in the kernel pmap, taken from the low pages at `first_avail`; the next free
/// page. The pages are the caller's (reserved in the boot memory map, below 4 GB).
#[cfg(machine_x86)]
pub fn pmap_prealloc_lowmem_ptps(first_avail: Paddr) -> Paddr {
    mapper().prealloc_lowmem_ptps(first_avail)
}

/// `pmap_kernel()`: the kernel's pmap.
pub fn pmap_kernel() -> &'static MachinePmap {
    hal::pmap::kernel()
}

/// `pmap_create()`: a new, empty user pmap with one reference; its top-level table maps the
/// kernel half. The C's `pool_get(PR_WAITOK)` never fails: no pmap is a panic.
pub fn pmap_create() -> PmapRef {
    booted();
    match hal::pmap::create() {
        Some(pm) => pm,
        None => panic(format_args!("pmap_create: no pmap")),
    }
}

/// `pmap_destroy(pmap)`: drops a reference; the last one frees the page tables and the pmap
/// itself (the caller removed every mapping: `hal` panics otherwise).
pub fn pmap_destroy(pmap: PmapRef) {
    drop(pmap);
}

/// `pmap_reference(pmap)`: one more reference.
pub fn pmap_reference(pmap: &PmapRef) -> PmapRef {
    pmap.clone()
}

/// `pmap_enter(pmap, va, pa, prot, flags)`: maps `pa` at `va` with `prot`; `flags` carries
/// `PMAP_WIRED`, `PMAP_CANFAIL` and the access type. Fails with `ENOMEM` only under
/// `PMAP_CANFAIL`. The kernel pmap through the kernel map's token.
pub fn pmap_enter(
    pmap: &MachinePmap,
    va: Vaddr,
    pa: Paddr,
    prot: VmProt,
    flags: i32,
) -> Result<(), Errno> {
    if pmap.is_kernel() {
        mapper().enter(va, pa, prot, flags)
    } else {
        pmap.enter(va, pa, prot, flags)
    }
}

/// `pmap_remove(pmap, sva, eva)`: removes the mappings in `[sva, eva)`.
pub fn pmap_remove(pmap: &MachinePmap, sva: Vaddr, eva: Vaddr) {
    if pmap.is_kernel() {
        mapper().remove(sva, eva);
    } else {
        pmap.remove(sva, eva);
    }
}

/// `pmap_protect(pmap, sva, eva, prot)`: lowers the protection of the mappings in
/// `[sva, eva)` to `prot`; `PROT_NONE` removes them.
pub fn pmap_protect(pmap: &MachinePmap, sva: Vaddr, eva: Vaddr, prot: VmProt) {
    if pmap.is_kernel() {
        mapper().protect(sva, eva, prot);
    } else {
        pmap.protect(sva, eva, prot);
    }
}

/// `pmap_wired_count(pmap)`: the number of wired pages in the pmap (`pm_stats`).
pub fn pmap_wired_count(pmap: &MachinePmap) -> i64 {
    pmap.wired_count()
}

/// `pmap_resident_count(pmap)`: the number of resident pages in the pmap (`pm_stats`).
pub fn pmap_resident_count(pmap: &MachinePmap) -> i64 {
    pmap.resident_count()
}

/// `pmap_unwire(pmap, va)`: clears the wired bit of the mapping at `va`.
pub fn pmap_unwire(pmap: &MachinePmap, va: Vaddr) {
    pmap.unwire(va);
}

/// `pmap_page_protect(pg, prot)`: lowers every mapping of `pg` to `prot`; `PROT_NONE`
/// removes them all (what the object layer does before freeing a page).
pub fn pmap_page_protect(pg: &VmPage, prot: VmProt) {
    hal::pmap::page_protect(&pg.mdpage, prot);
}

/// `pmap_clear_modify(pg)`: clears the modified bit of every mapping of `pg`; whether any
/// was set.
pub fn pmap_clear_modify(pg: &VmPage) -> bool {
    hal::pmap::clear_modify(&pg.mdpage)
}

/// `pmap_clear_reference(pg)`: clears the referenced bit of every mapping of `pg`; whether
/// any was set.
pub fn pmap_clear_reference(pg: &VmPage) -> bool {
    hal::pmap::clear_reference(&pg.mdpage)
}

/// `pmap_is_modified(pg)`: whether any mapping of `pg` has its modified bit set.
pub fn pmap_is_modified(pg: &VmPage) -> bool {
    hal::pmap::is_modified(&pg.mdpage)
}

/// `pmap_is_referenced(pg)`: whether any mapping of `pg` has its referenced bit set.
pub fn pmap_is_referenced(pg: &VmPage) -> bool {
    hal::pmap::is_referenced(&pg.mdpage)
}

/// `pmap_remove_holes(vm)`: makes the MMU's unmappable holes unavailable in the map
/// (nothing on amd64 and arm64: the C's `pmap_remove_holes` is empty there).
pub fn pmap_remove_holes(_vm: &Vmspace) {}

/// `pmap_proc_iflush(pr, va, len)`: makes instructions just written to `[va, va+len)` of
/// `pr`'s address space visible to instruction fetch (an I-cache sync where the caches are
/// not coherent).
pub fn pmap_proc_iflush(pr: &Process, va: Vaddr, len: Vsize) {
    pr.vmspace().vm_map.pmap().proc_iflush(va, len);
}

/// `pmap_zero_page`: zero-fills the page.
pub fn pmap_zero_page(pg: &VmPage) {
    booted();
    hal::pmap::zero_page(vm_page_to_phys(pg));
}

/// `pmap_copy_page`: copies `src` into `dst`.
pub fn pmap_copy_page(src: &VmPage, dst: &VmPage) {
    booted();
    hal::pmap::copy_page(vm_page_to_phys(src), vm_page_to_phys(dst));
}

/// `pmap_virtual_space`: the kernel virtual range still free, on machines without
/// `PMAP_STEAL_MEMORY`.
pub fn pmap_virtual_space(start: &mut Vaddr, end: &mut Vaddr) {
    (*start, *end) = hal::pmap::virtual_space();
}

/// `pmap_kenter_pa`: maps `pa` at `va` in the kernel pmap, wired, with `prot`.
///
/// # Safety
///
/// `va` must be kernel virtual space the caller owns and `pa` a page it may map there (the
/// proof the kernel map's token stands for, which these legacy callers give by their own
/// `unsafe` block until their owners move).
pub unsafe fn pmap_kenter_pa(va: Vaddr, pa: Paddr, prot: VmProt) {
    mapper().kenter(va, pa, prot);
}

/// `pmap_kremove`: removes `len` bytes of mappings made by `pmap_kenter_pa` from `va`.
///
/// # Safety
///
/// The range must have been mapped by `pmap_kenter_pa` and nothing may use it afterwards.
pub unsafe fn pmap_kremove(va: Vaddr, len: Vsize) {
    mapper().kremove(va, len);
}

/// `pmap_kenter_cache(va, pa, prot, cacheable)` (arm64): as [`pmap_kenter_pa`] with the
/// memory type `cacheable` (`PMAP_CACHE_*`).
///
/// # Safety
///
/// As for [`pmap_kenter_pa`].
#[cfg(target_arch = "aarch64")]
pub unsafe fn pmap_kenter_cache(va: Vaddr, pa: Paddr, prot: VmProt, cacheable: i32) {
    mapper().kenter_cache(va, pa, prot, cacheable);
}

/// `pmap_extract`: the physical address `va` maps to in `pmap`, if any.
pub fn pmap_extract(pmap: &MachinePmap, va: Vaddr) -> Option<Paddr> {
    pmap.extract(va)
}

/// `pmap_update`: makes pending mapping changes visible (nothing: every call does its own
/// TLB work, on amd64 and arm64 alike).
pub fn pmap_update(_pmap: &MachinePmap) {}

/// Loads `pm`'s root on this CPU and records the switch (the context switch's token).
fn switch_to(pm: &MachinePmap) {
    pm.load(switcher());
    Machine::pmap_switched_md(pm);
}

/// `pmap_activate(p)`: `p`'s address space is in use by one more thread; switched in now
/// when `p` is the running thread.
pub fn pmap_activate(p: &Proc) {
    let pm = p.vmspace().vm_map.pmap();
    if Machine::pmap_activate_md(p, pm) {
        switch_to(pm);
    }
}

/// `pmap_setttb(p)` (arm64): the switch path's load of `p`'s address space, for every
/// context switch (`cpu_switchto`), and `pmap_activate`'s.
pub fn pmap_setttb(p: &Proc) {
    switch_to(p.vmspace().vm_map.pmap());
}

/// `pmap_deactivate(p)`: the inverse, at exit.
pub fn pmap_deactivate(p: &Proc) {
    Machine::pmap_deactivate_md(p, p.vmspace().vm_map.pmap());
}

/// `pmap_purge(p)` (`__HAVE_PMAP_PURGE`): the last thread of a dying process is about to
/// tear its address space down (`uvm_purge`). arm64 drops the process's ASID first; a
/// machine without `__HAVE_PMAP_PURGE` (amd64, host) does nothing.
pub fn pmap_purge(p: &Proc) {
    Machine::pmap_purge_md(p, p.vmspace().vm_map.pmap());
}

/// `efi_enter`'s pmap half (arm64): the EFI runtime pmap's tables on this CPU (interrupts
/// are the caller's to disable around the call into the runtime).
#[cfg(target_arch = "aarch64")]
pub fn pmap_efi_enter(pm: &MachinePmap) {
    pm.load_efi(switcher());
}

/// `efi_leave`'s pmap half (arm64): the user address-space width back and `to`, the pmap
/// this CPU ran on before [`pmap_efi_enter`], loaded again.
#[cfg(target_arch = "aarch64")]
pub fn pmap_efi_leave(to: &MachinePmap) {
    hal::pmap::leave_efi(to, switcher());
}

/// `pmap_growkernel`: grows the kernel page tables to cover `maxkvaddr`; returns how far
/// they now reach.
pub fn pmap_growkernel(maxkvaddr: Vaddr) -> Vaddr {
    hal::pmap::growkernel(maxkvaddr)
}

/// `pmap_init`: the pmap module's own initialisation, once the page system is up.
pub fn pmap_init() {
    booted();
    Machine::pmap_init_md();
}

/// `pmap_map_direct`: the direct-map address of a page (`__HAVE_PMAP_DIRECT`).
pub fn pmap_map_direct(pg: &VmPage) -> Vaddr {
    hal::pmap::direct_map(vm_page_to_phys(pg))
}

/// `pmap_unmap_direct`: the page behind a direct-map address.
pub fn pmap_unmap_direct(va: Vaddr) -> Option<&'static VmPage> {
    hal::pmap::direct_unmap(va).and_then(PHYS_TO_VM_PAGE)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The adapter over hal's host backend: the dispatch of the kernel pmap to the token, the
    // counted handles, the page provider over the test's memory.

    use std::assert;
    use std::assert_eq;

    use super::*;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::mman::{PROT_READ, PROT_WRITE};
    use crate::sys::param::PAGE_SIZE;
    use crate::uvm::uvm_pmap::PMAP_WIRED;

    #[test]
    fn the_kernel_pmap_goes_through_the_token() {
        let _g = setup_real_memory();
        let va = Vaddr::new(hal::arch::current::vmparam::VM_MIN_KERNEL_ADDRESS + 0x7000_0000);
        let pa = Paddr::new(0x4000_0000);
        pmap_enter(pmap_kernel(), va, pa, PROT_READ, PROT_READ | PMAP_WIRED).unwrap();
        assert_eq!(pmap_extract(pmap_kernel(), va), Some(pa));
        pmap_remove(pmap_kernel(), va, Vaddr::new(va.as_usize() + PAGE_SIZE));
        assert_eq!(pmap_extract(pmap_kernel(), va), None);
    }

    #[test]
    fn a_user_pmap_is_counted_and_freed_with_its_last_handle() {
        let _g = setup_real_memory();
        let pm = pmap_create();
        let other = pmap_reference(&pm);
        assert!(PmapRef::ptr_eq(&pm, &other));
        let va = Vaddr::new(0x1000_0000);
        let pa = Paddr::new(0x4000_1000);
        pmap_enter(&pm, va, pa, PROT_READ | PROT_WRITE, PROT_READ).unwrap();
        assert_eq!(pmap_extract(&other, va), Some(pa));
        assert_eq!(pmap_resident_count(&pm), 1);
        pmap_remove(&pm, va, Vaddr::new(va.as_usize() + PAGE_SIZE));
        pmap_destroy(other);
        assert_eq!(pmap_extract(&pm, va), None);
        pmap_destroy(pm);
    }

    #[test]
    fn the_provider_knows_the_managed_pages() {
        use hal::pmap::PhysPages;
        let _g = setup_real_memory();
        let pages = &kernel::vm::page::PHYS_PAGES;
        let pa = pages.alloc_ptp().expect("a page-table page");
        let md = pages.md_of(pa).expect("a managed page");
        assert!(core::ptr::eq(md, &PHYS_TO_VM_PAGE(pa).unwrap().mdpage));
        assert_eq!(PHYS_TO_VM_PAGE(pa).unwrap().wire_count(), 1);
        assert!(pages.page_init_done());
        pages.free_ptp(pa);
        assert_eq!(pages.md_of(Paddr::new(0)).map(|_| ()), None);
    }
}
/* </TESTS> */
