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
//! The host double's pmap behind `hal::pmap`: each pmap's mappings in a `BTreeMap`, page by
//! page. Not an OpenBSD file; it must not grow logic (`sys/hal/arch/host/mod.rs`).
//!
//! It keeps the behaviour of `bsd`'s host double (`sys/arch/host/mod.rs`, LZ's), which `bsd`'s
//! host tests were written against: physical pages are numbers, not memory, so zeroing and
//! copying them do nothing and the "direct map" is the identity; mappings have no protection
//! (`PROT_NONE` removes them, as the C does) and nothing is wired or emulated; a page has no
//! pv chain, so the page-level calls change nothing and report nothing set; boot memory comes
//! from the host allocator, its frames taken out of the provider (`steal`) so the page count
//! adds up. `resident_count` is the number of mappings. A pmap's limit is
//! `VM_MAXUSER_ADDRESS`, an EFI pmap's (aarch64 hosts) `1 << EFI_SPACE_BITS` as arm64's.

// The host double alone links std outside the tests (rust-kernel.md: the host doubles).
extern crate std;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use super::param::{
    PAGE_MASK, PAGE_SIZE, VM_MAX_KERNEL_ADDRESS, VM_MAXUSER_ADDRESS, VM_MIN_KERNEL_ADDRESS,
};
use crate::intr::IPL_VM;
use crate::pmap::{BootstrapParams, PageMd, PhysPages, Pmap, PmapBackend, PmapStatistics, VmProt};
use crate::pool::PR_WAITOK;
use crate::sys::errno::Errno;
use crate::sys::mman::PROT_NONE;
use crate::sys::types::{Paddr, Vaddr, Vsize};
use crate::vm::param::round_page;

/// arm64's `EFI_SPACE_BITS`: the width of the EFI runtime pmap's address space.
#[cfg(target_arch = "aarch64")]
const EFI_SPACE_BITS: u32 = 48;

/// The host's `struct pmap`: the mappings, virtual page to physical page, and the limit.
pub(crate) struct PmapMd {
    mappings: Mutex<BTreeMap<usize, usize>>,
    limit: usize,
}

impl PmapMd {
    /// No mapping, the user half's limit.
    pub(crate) const fn new() -> Self {
        Self {
            mappings: Mutex::new(BTreeMap::new()),
            limit: VM_MAXUSER_ADDRESS,
        }
    }

    /// The mappings, a poisoned lock taken over (a test that panicked holding it).
    fn map(&self) -> std::sync::MutexGuard<'_, BTreeMap<usize, usize>> {
        self.mappings.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The host's pv chain: none.
pub(crate) struct PvHead;

impl PvHead {
    /// No mapping.
    pub(crate) const fn new() -> Self {
        Self
    }
}

/// The kernel pmap.
static KERNEL_PMAP: Pmap = Pmap::kernel_with(PmapMd::new());

/// `pmap_initialized`.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// The host double's backend.
pub(crate) struct Backend;

impl Backend {
    /// Writes the pmap's statistics after a change of its map.
    fn account(pmap: &Pmap, map: &BTreeMap<usize, usize>) {
        pmap.stats.resident_count.set(map.len() as i64);
    }

    /// Removes the pages of `[sva, eva)` from `pmap`.
    fn remove_range(pmap: &Pmap, sva: Vaddr, eva: Vaddr) {
        let mut map = pmap.md.map();
        let from = sva.trunc_page().as_usize();
        map.retain(|&va, _| va < from || va >= eva.as_usize());
        Self::account(pmap, &map);
    }

    /// Maps the page of `pa` at the page of `va` in `pmap`.
    fn insert(pmap: &Pmap, va: Vaddr, pa: Paddr) {
        let mut map = pmap.md.map();
        map.insert(va.trunc_page().as_usize(), pa.trunc_page().as_usize());
        Self::account(pmap, &map);
    }
}

impl PmapBackend for Backend {
    const HAVE_PMAP_DIRECT: bool = true;
    const PMAP_STEAL_MEMORY: bool = true;
    const PMAP_WC: usize = 0;
    const PMAP_NOCACHE: usize = 0;
    const PMAP_NOMMU: bool = true;
    const POOL_WCHAN: &'static str = "pmappl";
    const POOL_IPL: i32 = IPL_VM;
    const POOL_FLAGS: i32 = PR_WAITOK;

    fn kernel() -> &'static Pmap {
        &KERNEL_PMAP
    }

    fn bootstrap(p: &BootstrapParams) -> (Paddr, Vaddr) {
        (p.first_avail, Vaddr::new(VM_MIN_KERNEL_ADDRESS))
    }

    fn init() {
        INITIALIZED.store(true, Ordering::Relaxed);
    }

    fn initialized() -> bool {
        INITIALIZED.load(Ordering::Relaxed)
    }

    fn va_limit(pmap: &Pmap) -> Vaddr {
        Vaddr::new(pmap.md.limit)
    }

    fn create(_stats: &PmapStatistics, _pages: &'static dyn PhysPages) -> Option<PmapMd> {
        Some(PmapMd::new())
    }

    #[cfg(target_arch = "aarch64")]
    fn create_efi(_stats: &PmapStatistics, _pages: &'static dyn PhysPages) -> Option<PmapMd> {
        Some(PmapMd {
            mappings: Mutex::new(BTreeMap::new()),
            limit: 1 << EFI_SPACE_BITS,
        })
    }

    /// The host has no translation registers.
    #[cfg(target_arch = "aarch64")]
    fn load_efi(_pmap: &Pmap) {}

    /// The host has no translation registers.
    #[cfg(target_arch = "aarch64")]
    fn leave_efi(_to: &Pmap) {}

    fn mappings(pmap: &Pmap) -> usize {
        pmap.md.map().len()
    }

    fn destroy(_pmap: &mut Pmap) {}

    fn enter(pmap: &Pmap, va: Vaddr, pa: Paddr, _prot: VmProt, _flags: i32) -> Result<(), Errno> {
        Self::insert(pmap, va, pa);
        Ok(())
    }

    fn remove(pmap: &Pmap, sva: Vaddr, eva: Vaddr) {
        Self::remove_range(pmap, sva, eva);
    }

    /// Mappings have no protection on the host; `PROT_NONE` removes them, as the C does.
    fn protect(pmap: &Pmap, sva: Vaddr, eva: Vaddr, prot: VmProt) {
        if prot == PROT_NONE {
            Self::remove_range(pmap, sva, eva);
        }
    }

    fn extract(pmap: &Pmap, va: Vaddr) -> Option<Paddr> {
        pmap.md
            .map()
            .get(&va.trunc_page().as_usize())
            .map(|pa| Paddr::new(pa + (va.as_usize() & PAGE_MASK)))
    }

    /// Nothing is wired on the host.
    fn unwire(_pmap: &Pmap, _va: Vaddr) {}

    fn kenter(va: Vaddr, pa: Paddr, _prot: VmProt) {
        Self::insert(&KERNEL_PMAP, va, pa);
    }

    fn kremove(va: Vaddr, len: Vsize) {
        let end = va.as_usize() + len.as_usize();
        Self::remove_range(&KERNEL_PMAP, va, Vaddr::new(end));
    }

    fn load(_pmap: &Pmap) {}

    fn activate(_pmap: &Pmap) {}

    fn deactivate(_pmap: &Pmap) {}

    fn purge(_pmap: &Pmap) {}

    fn proc_iflush(_pmap: &Pmap, _va: Vaddr, _len: Vsize) {}

    /// Mappings have no protection and pages no pv chain on the host.
    fn page_protect(_md: &PageMd, _prot: VmProt) {}

    /// Nothing is ever modified on the host.
    fn clear_modify(_md: &PageMd) -> bool {
        false
    }

    /// Nothing is ever referenced on the host.
    fn clear_reference(_md: &PageMd) -> bool {
        false
    }

    /// Nothing is ever modified on the host.
    fn is_modified(_md: &PageMd) -> bool {
        false
    }

    /// Nothing is ever referenced on the host.
    fn is_referenced(_md: &PageMd) -> bool {
        false
    }

    /// Page contents are not modelled.
    fn zero_page(_pa: Paddr) {}

    /// Page contents are not modelled.
    fn copy_page(_src: Paddr, _dst: Paddr) {}

    /// Boot memory comes from the host allocator and is never returned; the frames it stands
    /// for leave the page system through the provider, as on a real machine (a missing frame
    /// is the C's "out of memory" panic; the tests load enough).
    fn steal_memory(pages: &'static dyn PhysPages, size: Vsize) -> Vaddr {
        let size = round_page(size.as_usize());
        let _ = pages.steal(size / PAGE_SIZE);
        let words = size.div_ceil(size_of::<u64>());
        let block: &'static mut [u64] = std::vec![0u64; words].leak();

        Vaddr::new(block.as_mut_ptr().addr())
    }

    fn virtual_space() -> (Vaddr, Vaddr) {
        (
            Vaddr::new(VM_MIN_KERNEL_ADDRESS),
            Vaddr::new(VM_MAX_KERNEL_ADDRESS),
        )
    }

    /// The host's page tables never run out.
    fn growkernel(maxkvaddr: Vaddr) -> Vaddr {
        maxkvaddr
    }

    /// The "direct map" is the identity: a frame's address is its number.
    fn direct_map(pa: Paddr) -> Vaddr {
        Vaddr::new(pa.as_usize())
    }

    fn direct_unmap(va: Vaddr) -> Option<Paddr> {
        Some(Paddr::new(va.as_usize()))
    }

    fn flush_cache(_va: Vaddr, _len: Vsize) {}

    fn module_window() -> Option<(Vaddr, usize)> {
        None
    }

    fn module_enter(_va: Vaddr, _pa: Paddr, _prot: VmProt) -> bool {
        false
    }

    /// The host has no `%cr3`.
    #[cfg(target_arch = "x86_64")]
    fn cr3(_pmap: &Pmap) -> u64 {
        0
    }

    /// The host has no memory types: a plain `kenter`.
    #[cfg(target_arch = "aarch64")]
    fn kenter_cache(va: Vaddr, pa: Paddr, prot: VmProt, _cacheable: i32) {
        Self::kenter(va, pa, prot);
    }

    /// The host emulates nothing.
    #[cfg(target_arch = "aarch64")]
    fn fault_fixup(_pmap: &Pmap, _va: Vaddr, _ftype: VmProt) -> bool {
        false
    }
}
/* </CODE> */
