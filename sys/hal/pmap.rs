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
 * Copyright (c) 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)pmap.h	8.1 (Berkeley) 6/11/93
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Author: Avadis Tevanian, Jr.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */
/* </LICENSES> */

/* <CODE> */
//! `pmap(9)`: the physical map, what the virtual memory system asks the MMU code to do.
//!
//! Upstream: sys/uvm/uvm_pmap.h @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_pmap.rs@3c62ede68802
//!
//! The machine-independent half of the interface: the types every machine shares and one
//! function or method per `pmap(9)` entry point, each a call into the running machine's
//! backend (`crate::arch::current::pmap`: amd64's and arm64's at N3.1 commits 4 and 5, the
//! host double's `BTreeMap` in `sys/hal/arch/host/pmap.rs`). The backend implements the
//! crate-private [`PmapBackend`] contract; what only one machine's own code uses (amd64's TLB
//! shootdown words and `PG_NX_BIT`, arm64's `VMMAP`, ...) stays `pub` in its backend module
//! and is reached by path from that machine's code.
//!
//! The behaviour is the C's (`uvm_pmap.h`, `pmap(9)`, the two `pmap.c`): `pmap_enter` fails only
//! under `PMAP_CANFAIL`, `pmap_update` does nothing (each call does its TLB work), a user pmap
//! is locked by its `pm_mtx` (`IPL_VM`) before the page's `pv_mtx` (`IPL_VM`), the kernel pmap
//! is never locked, PTPs and pv entries are freed only after the shootdown's wait.
//!
//! **The trust boundary.** A physical address is trusted everywhere (`enter`, `kenter`,
//! `zero_page`, `copy_page`, `module_enter`): that `pa` is a page the caller may map or write
//! is the frame proof a typed page would bring (`kernel::vm::page::VmPage` exists since N3.3,
//! but the pmap's callers still pass frames); until then it is the caller's word, as in the C. What `hal` can check it checks: a user pmap refuses the kernel's half, the
//! kernel pmap is changed only through the [`KernelMapper`] token, and the page provider is
//! the one [`bootstrap`] was given, once, by the boot path (decision 30's pattern).
//!
//! ## Deviations
//! - The `pmap_statistics` counters are [`PmapCounter`]s, relaxed atomic words: a user
//!   pmap's are changed under its lock, but the kernel pmap's are changed by every CPU without
//!   one (the C's plain `long`s race there; a racing `Cell` would be undefined behaviour in
//!   Rust). Loads and stores stay separate, so the counts are as approximate as the C's.
//!
//! ## Redesign
//! - LZ's `machine::Pmap` trait (`sys/machine/pmap.rs`) is implemented in `bsd` by each
//!   machine over its `struct pmap`, handed out as `&'static Pmap` from a pool
//!   (`pmap_create`), freed by `pmap_destroy(&'static Pmap)`. Here a user pmap is a
//!   [`PmapRef`], a counted handle with a static arm for the kernel pmap
//!   (`pmap_reference` = `Clone`, `pmap_destroy` = the drop, `pmap_create` = [`create`]); the
//!   last drop frees the page tables, and panics while a leaf mapping is left (the backend's
//!   count, not `resident_count`, which on amd64 counts the PTPs too), where the C frees
//!   silently: a pv entry of a freed pmap is a use after free (uvm removes every entry's
//!   mappings before `pmap_destroy`, `uvm_map.c` `uvm_unmap_kill_entry`).
//! - `pmap_kenter_pa`, `pmap_kremove` and `pmap_enter`/`pmap_remove`/`pmap_protect` on the
//!   kernel pmap were `unsafe fn`s or unchecked (a safe call could remap a kernel stack): they
//!   are methods of [`KernelMapper`], a zero-sized token without a public constructor that
//!   [`bootstrap`] returns once; holding it is the proof the caller owns the kernel map's
//!   allocations (the boot path gives it to the kernel map's owner, `uvm_km`, N3.2). The
//!   token does not refuse the user half: amd64's MP trampoline is mapped at its low
//!   identity address in the kernel pmap (`cpu.c`, `machdep.c`), executable. arm64's
//!   kernel pmap, which has no such page, refuses `PROT_EXEC` through the token (its
//!   backend's deviations): there nothing executes but the boot loader's mapping of the image
//!   (under boot(8)/efiboot its whole 64 MB block, read-write, an inherited deviation of the
//!   backend) and the module window.
//! - Switching the running CPU to a pmap's root (`lcr3` in `pmap_activate`, arm64's
//!   `pmap_setttb`, `efi_enter`/`efi_leave`) takes a second token, [`Switcher`], from the
//!   same `bootstrap`: a safe `load` would let any caller run on a root whose pmap it then
//!   drops (freeing the tables under it) or leave a CPU out of the shootdowns.
//! - A user [`Pmap`]'s `enter`, `remove` and `protect` refuse the kernel half (`va` at or above
//!   the pmap's limit, `VM_MAXUSER_ADDRESS`; on amd64 the recursive mapping's slot starts right
//!   above it) and the kernel pmap (which needs the token): both panic, as uvm never asks
//!   (stricter than the C, which would write into the shared kernel tables). arm64's EFI
//!   runtime pmap ([`create_efi`]) has its own limit, `1 << EFI_SPACE_BITS`: the runtime
//!   services are identity-mapped and may sit above the user half (`efi_machdep.c`).
//! - A pv entry names its pmap by a pointer; who must hold the pmap after dropping the page's
//!   `pv_mtx` (`pmap_page_remove`, the C's `pmap_reference(pve->pv_pmap)`) rebuilds a counted
//!   handle from it with `PoolArc::from_value` (`sys/hal/pool.rs`), in the backend: this
//!   module forbids `unsafe`. The extra handle is dropped outside `pv_mtx` and `pm_mtx`, since
//!   its drop may be the last and free page-table pages.
//! - `pmap_zero_page`/`pmap_copy_page` took a `struct vm_page *`, so their frame was a
//!   managed page by type: [`zero_page`] and [`copy_page`] take a `Paddr` and panic unless the
//!   provider knows it as a managed page (`PhysPages::md_of`), the check that type made.
//! - `struct vm_page_md` is [`PageMd`], `hal`'s type, embedded by value in the kernel's page
//!   (`kernel::vm::page::VmPage`, N3.3): the pv chain under its own mutex, the
//!   pmap's attribute bits the C keeps in `pg_flags` (`PG_PMAP0`..`2`) in their own atomic
//!   word, and a PTP's count of valid entries (the C's `wire_count` of a page-table page).
//!   N3.4a: the word also holds `hal`'s claim (`PG_HAL_VIEW`, `PG_HAL_VIEW_W`, never in
//!   [`PageMd::attrs`]): a frame held alone by a byte view (`sys/hal/mapped.rs`) or for the
//!   length of [`zero_page`]/[`copy_page`], which panic on a held frame, as
//!   [`page_protect`] does when it would take a view's mapping away and
//!   [`PageMd::clear_attrs`] when the page system hands a held page out again;
//!   [`PhysPages::is_busy`] answers `PG_BUSY` for the views' check.
//! - The page allocator is a value passed down, [`PhysPages`] (D16), given to [`bootstrap`]
//!   and kept by the kernel pmap, copied into every pmap [`create`] makes: `uvm_pagealloc`,
//!   `uvm_pagefree`, `PHYS_TO_VM_PAGE`, `uvm_page_physsteal`. `pmap_steal_memory` reads the
//!   kernel pmap's, so no other provider can be handed in.
//! - `pmap_bootstrap`'s inputs, which machdep wrote into the pmap's statics (`PMAP_DIRECT_*`,
//!   `PMAP_KERNEL_IMAGE_*`, `EFER.NXE` read inside the pmap), are a [`BootstrapParams`]; its
//!   answers (amd64 the first free page, arm64 the first free kernel address) are both in
//!   [`Bootstrapped`], with the token.
//! - `pmap_virtual_space(&start, &end)` returns the pair; `pmap_steal_memory(size, &start,
//!   &end)` returns the address and the caller asks [`virtual_space`] for the bounds after it.
//! - The bridges the backends need from code still in `bsd`: [`TlbCpus`] (amd64's shootdown:
//!   the CPU list, `ci_proc_pmap`, the IPIs; N4) and [`Random`] (`arc4random` for arm64's ASID
//!   pick; N7), registered in `sys/hal/bridges.rs`.

use core::ops::Deref;
use core::sync::atomic::{AtomicI64, AtomicU32, Ordering};

use crate::arch::current::pmap::{Backend, PmapMd, PvHead};
use crate::assert::hal_panic;
use crate::intr::IPL_VM;
use crate::mutex::Mutexed;
use crate::once::OnceStatic;
use crate::pool::{PR_WAITOK, PoolArc, TypedPool};
use crate::sys::errno::Errno;
use crate::sys::mman::{PROT_READ, PROT_WRITE};
use crate::sys::types::{Paddr, Vaddr, Vsize};

/// `vm_prot_t`: a combination of `PROT_READ`, `PROT_WRITE`, `PROT_EXEC` (`<sys/mman.h>`), the
/// same type as `uvm_extern`'s.
pub type VmProt = i32;

/// Wired mapping.
pub const PMAP_WIRED: i32 = 0x0000_0010;
/// Can fail if resource shortage.
pub const PMAP_CANFAIL: i32 = 0x0000_0020;
/// Machine dependant.
pub const PMAP_MD0: i32 = 0x0000_0040;
/// Machine dependant.
pub const PMAP_MD1: i32 = 0x0000_0080;
/// Machine dependant.
pub const PMAP_MD2: i32 = 0x0000_0100;
/// Machine dependant.
pub const PMAP_MD3: i32 = 0x0000_0200;

/// `__HAVE_PMAP_DIRECT`: physical memory is direct-mapped, so [`direct_map`] works.
pub const HAVE_PMAP_DIRECT: bool = Backend::HAVE_PMAP_DIRECT;
/// `PMAP_STEAL_MEMORY`: `uvm_pageboot_alloc` defers to [`steal_memory`].
pub const PMAP_STEAL_MEMORY: bool = Backend::PMAP_STEAL_MEMORY;
/// `PMAP_WC`: the physical-address flag of a write-combining mapping (0 where the pmap has
/// none, as `<uvm/uvm_pmap.h>` defaults it).
pub const PMAP_WC: usize = Backend::PMAP_WC;
/// `PMAP_NOCACHE`: the physical-address flag of an uncached mapping (0 where the pmap has
/// none).
pub const PMAP_NOCACHE: usize = Backend::PMAP_NOCACHE;
/// No MMU behind the pmap: only the direct map is addressable (the host double, whose
/// "physical" pages are numbers). False on every real machine.
pub const PMAP_NOMMU: bool = Backend::PMAP_NOMMU;

/// `PG_PMAP0` of `pg_flags`, here a bit of [`PageMd`]'s attribute word: the page was
/// modified (`PG_PMAP_MOD` on both machines).
pub const PG_PMAP_MOD: u32 = 0x0100_0000;
/// `PG_PMAP1`: the page was referenced (`PG_PMAP_REF` on both machines).
pub const PG_PMAP_REF: u32 = 0x0200_0000;
/// `PG_PMAP2`: machine-dependent (amd64 `PG_PMAP_WC`, mapped write-combining; arm64
/// `PG_PMAP_EXE`, mapped executable and I-cache clean).
pub const PG_PMAP2: u32 = 0x0400_0000;
/// `hal`'s own bit of [`PageMd`]'s attribute word, outside the `PG_PMAP*` bits (and so never
/// reported by [`PageMd::attrs`]): the frame is held alone, by a byte view
/// (`hal::mapped::MappedBytes`) or for the length of `pmap_zero_page`/`pmap_copy_page`.
const PG_HAL_VIEW: u32 = 0x8000_0000;
/// With [`PG_HAL_VIEW`]: the view is writable, so its mapping may not lose `PROT_WRITE`.
const PG_HAL_VIEW_W: u32 = 0x4000_0000;
/// The bits `hal` keeps for itself.
const PG_HAL_MASK: u32 = PG_HAL_VIEW | PG_HAL_VIEW_W;

/// The pool of user pmaps (`pmap_pmap_pool`), named as the C names it on each machine
/// (`"pmappl"` on amd64, `"pmap"` on arm64), made by [`init`].
static PMAP_POOL: TypedPool<Pmap> = TypedPool::new();

/// A `long` of `struct pmap_statistics` (see the module's deviations).
pub struct PmapCounter(AtomicI64);

impl PmapCounter {
    /// A counter at `v`.
    pub const fn new(v: i64) -> Self {
        Self(AtomicI64::new(v))
    }

    /// Reads the counter.
    pub fn get(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }

    /// Writes the counter.
    pub fn set(&self, v: i64) {
        self.0.store(v, Ordering::Relaxed);
    }
}

/// `struct pmap_statistics`.
pub struct PmapStatistics {
    /// Number of pages mapped (total).
    pub resident_count: PmapCounter,
    /// Number of pages wired.
    pub wired_count: PmapCounter,
}

impl PmapStatistics {
    /// Zeroed statistics.
    pub const fn new() -> Self {
        Self {
            resident_count: PmapCounter::new(0),
            wired_count: PmapCounter::new(0),
        }
    }
}

impl Default for PmapStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct pmap`: one address space's page tables, the machine's (`md`), with what every
/// machine keeps beside them.
///
/// A user pmap lives in a pool item and is reached through [`PmapRef`]s; the kernel pmap is a
/// static of the backend ([`kernel`]). Invariant: `pages` is set before the pmap is shared
/// (by [`bootstrap`] for the kernel pmap, by [`create`] for a user one) and never changes.
pub struct Pmap {
    /// `pm_stats`.
    pub(crate) stats: PmapStatistics,
    /// The page provider, the one [`bootstrap`] was given.
    pub(crate) pages: OnceStatic<&'static dyn PhysPages>,
    /// The machine's part (`pm_mtx` and the tables, the root's address, arm64's ASID, ...).
    pub(crate) md: PmapMd,
}

impl Pmap {
    /// The kernel pmap around the backend's `md`, for the backend's static (the provider is
    /// set by [`bootstrap`]).
    pub(crate) const fn kernel_with(md: PmapMd) -> Self {
        Self {
            stats: PmapStatistics::new(),
            pages: OnceStatic::new(),
            md,
        }
    }

    /// The page provider of this pmap; panics before [`bootstrap`] gave the kernel pmap one
    /// (no page-table page can be needed before it).
    pub(crate) fn pages(&self) -> &'static dyn PhysPages {
        match self.pages.get() {
            Some(p) => *p,
            None => hal_panic(format_args!("pmap: used before pmap_bootstrap")),
        }
    }

    /// Whether this is the kernel pmap (`pmap == pmap_kernel()`).
    pub fn is_kernel(&self) -> bool {
        core::ptr::eq(self, kernel())
    }

    /// The pmap's address, its identity for what records which pmap a CPU runs on
    /// (`pcb_pmap`, `ci_proc_pmap`, `ci_curpm`). Compared, not dereferenced, with one exception:
    /// arm64's `efi_leave` dereferences `ci_curpm`, which names the kernel pmap (a static) or the
    /// user pmap of the thread running the EFI call with interrupts off, alive for that call.
    pub fn as_ptr(&self) -> *const Pmap {
        self
    }

    /// The first address this pmap may not map (`VM_MAXUSER_ADDRESS` for a user pmap; arm64's
    /// EFI pmap, `1 << EFI_SPACE_BITS`).
    pub fn va_limit(&self) -> Vaddr {
        Backend::va_limit(self)
    }

    /// Panics unless this is a user pmap and `[sva, eva)` lies below its limit.
    fn user_range(&self, what: &str, sva: Vaddr, eva: Vaddr) {
        if self.is_kernel() {
            hal_panic(format_args!(
                "{what}: the kernel pmap is changed through the KernelMapper"
            ));
        }
        if sva.as_usize() > eva.as_usize() || eva > self.va_limit() {
            hal_panic(format_args!(
                "{what}: [{sva:?}, {eva:?}) is not in the user half of a user pmap"
            ));
        }
    }

    /// `pmap_enter(pmap, va, pa, prot, flags)` on a user pmap: maps `pa` (with the machine's
    /// `PMAP_NOCACHE`/`PMAP_WC` flag bits) at `va` with `prot`; `flags` carries the access type
    /// in its low three bits, `PMAP_WIRED` and `PMAP_CANFAIL`. Fails with `ENOMEM` only under
    /// `PMAP_CANFAIL` (a pv entry or a page-table page could not be had). Panics on the kernel
    /// pmap or a `va` of the kernel half.
    pub fn enter(&self, va: Vaddr, pa: Paddr, prot: VmProt, flags: i32) -> Result<(), Errno> {
        self.user_range("pmap_enter", va, va);
        if va >= self.va_limit() {
            hal_panic(format_args!(
                "pmap_enter: {va:?} is not in the user half of a user pmap"
            ));
        }
        Backend::enter(self, va, pa, prot, flags)
    }

    /// `pmap_remove(pmap, sva, eva)` on a user pmap: removes the mappings in `[sva, eva)`.
    /// Panics on the kernel pmap or a range reaching the kernel half.
    pub fn remove(&self, sva: Vaddr, eva: Vaddr) {
        self.user_range("pmap_remove", sva, eva);
        Backend::remove(self, sva, eva);
    }

    /// `pmap_protect(pmap, sva, eva, prot)` on a user pmap: lowers the protection of the
    /// mappings in `[sva, eva)` to `prot`; `PROT_NONE` removes them. Panics on the kernel pmap
    /// or a range reaching the kernel half.
    pub fn protect(&self, sva: Vaddr, eva: Vaddr, prot: VmProt) {
        self.user_range("pmap_protect", sva, eva);
        Backend::protect(self, sva, eva, prot);
    }

    /// `pmap_extract(pmap, va)`: the physical address `va` maps to, if any (any pmap).
    pub fn extract(&self, va: Vaddr) -> Option<Paddr> {
        Backend::extract(self, va)
    }

    /// `pmap_unwire(pmap, va)`: clears the wired bit of the mapping at `va` (any pmap: the
    /// kernel's too, for `uvm_fault_unwire` on `kernel_map`; no translation changes).
    pub fn unwire(&self, va: Vaddr) {
        Backend::unwire(self, va);
    }

    /// `pmap_wired_count(pmap)`: the wired pages (`pm_stats`).
    pub fn wired_count(&self) -> i64 {
        self.stats.wired_count.get()
    }

    /// `pmap_resident_count(pmap)`: the resident pages (`pm_stats`; amd64 counts its
    /// page-table pages in it, as the C).
    pub fn resident_count(&self) -> i64 {
        self.stats.resident_count.get()
    }

    /// The pmap's half of `pmap_activate`'s switch on the running CPU: amd64 `lcr3` of
    /// [`Pmap::cr3`]; arm64 `pmap_setttb`'s body (a new ASID if its generation is old, under
    /// `pmap_asid_mtx`, then `TTBR0_EL1` to the empty table and `cpu_setttb(asid, pt0pa)`).
    /// The process half (`pcb_pmap`, `ci_proc_pmap`, `ci_curpm`, `ci_flush_bp`) is the
    /// caller's.
    ///
    /// The [`Switcher`] is the proof that the caller is the context switch (its token, handed
    /// out once by [`bootstrap`], lives with the scheduler's adapter): that the CPU's record
    /// of its pmap (`ci_proc_pmap`, which amd64's shootdowns pick their targets by,
    /// [`TlbCpus::for_each_other_running`]) is updated in the same switch, and that the pmap
    /// stays referenced (a `PmapRef` in the running process) while the CPU runs on it. Beside
    /// the token the machine refuses what it can see: a pmap without a root (the kernel pmap
    /// before `bootstrap`) panics, and a pmap's last drop panics while this CPU's `%cr3` or
    /// another running CPU's record still names it, so a loaded root is never freed.
    pub fn load(&self, _switch: &Switcher) {
        Backend::load(self);
    }

    /// `efi_enter`'s pmap half (arm64): this EFI pmap's tables in `TTBR0_EL1` with the
    /// address-space width of the EFI pmap (`TCR_EL1.T0SZ = 64 - EFI_SPACE_BITS`), the empty
    /// table first; interrupts are the caller's to disable around the call into the runtime.
    /// The [`Switcher`] as for [`Pmap::load`]; a pmap [`create_efi`] did not make panics.
    #[cfg(target_arch = "aarch64")]
    pub fn load_efi(&self, _switch: &Switcher) {
        Backend::load_efi(self);
    }

    /// The pmap's half of `pmap_activate(p)`: one more thread uses it (arm64 `pm_active`;
    /// nothing on amd64).
    pub fn activate(&self) {
        Backend::activate(self);
    }

    /// The pmap's half of `pmap_deactivate(p)` on the running CPU (arm64: nothing when not
    /// active, else `TTBR0_EL1` back to the kernel's empty table and `pm_active` one less;
    /// nothing on amd64, whose `ci_proc_pmap` is the caller's).
    pub fn deactivate(&self) {
        Backend::deactivate(self);
    }

    /// The pmap's half of `pmap_purge(p)` (`__HAVE_PMAP_PURGE`, arm64: wait for the other
    /// threads to leave it, drop its ASIDs from the TLB, point it at the kernel's empty
    /// table; nothing on amd64).
    pub fn purge(&self) {
        Backend::purge(self);
    }

    /// `pmap_proc_iflush(pr, va, len)`'s pmap half: makes instructions just written to
    /// `[va, va + len)` of this address space visible to instruction fetch (arm64 an I-cache
    /// sync over the mapped pages; nothing on amd64, whose caches are coherent).
    pub fn proc_iflush(&self, va: Vaddr, len: Vsize) {
        Backend::proc_iflush(self, va, len);
    }

    /// The `%cr3` value of this pmap (`pcb_cr3`: the root's physical address; PCID is off).
    #[cfg(target_arch = "x86_64")]
    pub fn cr3(&self) -> u64 {
        Backend::cr3(self)
    }

    /// `pmap_fault_fixup(pm, va, ftype)`: completes a fault the pmap emulates (the access
    /// flag, the modified bit, a first execution) by giving the mapping at `va` the
    /// protection its descriptor records; whether it did (then the fault is handled). Any
    /// pmap: it never grants more than the mapping was entered with.
    #[cfg(target_arch = "aarch64")]
    pub fn fault_fixup(&self, va: Vaddr, ftype: VmProt) -> bool {
        Backend::fault_fixup(self, va, ftype)
    }
}

impl Drop for Pmap {
    /// `pmap_destroy`'s last reference (`pmap_release`): the page tables go back to the
    /// provider. Panics while a leaf mapping is left (see the module's redesign).
    fn drop(&mut self) {
        let left = Backend::mappings(self);
        if left != 0 {
            hal_panic(format_args!("pmap_destroy: {left} mappings left"));
        }
        Backend::destroy(self);
    }
}

/// A `pmap_t`: the kernel pmap, or a counted handle to a user one.
///
/// `Clone` is `pmap_reference`; dropping the last handle of a user pmap is `pmap_destroy`.
#[derive(Clone)]
pub enum PmapRef {
    /// `pmap_kernel()`.
    Kernel,
    /// A pmap [`create`] made.
    User(PoolArc<Pmap>),
}

impl PmapRef {
    /// The pmap's identity (as [`Pmap::as_ptr`]).
    pub fn as_ptr(&self) -> *const Pmap {
        Pmap::as_ptr(self)
    }

    /// Whether two handles name the same pmap.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        core::ptr::eq(a.as_ptr(), b.as_ptr())
    }
}

impl Deref for PmapRef {
    type Target = Pmap;

    fn deref(&self) -> &Pmap {
        match self {
            PmapRef::Kernel => kernel(),
            PmapRef::User(p) => p,
        }
    }
}

/// `struct vm_page_md`: the pmap's data inside each managed page (see the module's redesign).
pub struct PageMd {
    /// `pv_mtx` (`IPL_VM`, taken after the pmap's `pm_mtx`) and the page's mappings.
    // The host double keeps no pv chain and no page-table pages: only the machines read
    // `pv` and `ptp_refs`.
    #[cfg_attr(not(target_os = "none"), allow(dead_code))]
    pub(crate) pv: Mutexed<PvHead>,
    /// The `PG_PMAP_*` bits (`PG_PMAP_MOD`, `PG_PMAP_REF`, `PG_PMAP2`), and `hal`'s
    /// `PG_HAL_VIEW`/`PG_HAL_VIEW_W` (the frame is held alone, by a byte view or a copy).
    pub(crate) attrs: AtomicU32,
    /// Of a page-table page: its valid entries plus one (the C's `wire_count` of a PTP;
    /// amd64's alone: arm64's vp tables are found by the walk and freed with their pmap).
    #[cfg_attr(not(all(target_os = "none", target_arch = "x86_64")), allow(dead_code))]
    pub(crate) ptp_refs: AtomicU32,
}

impl PageMd {
    /// `VM_MDPAGE_INIT`: no mapping, no attribute.
    pub const fn new() -> Self {
        Self {
            pv: Mutexed::new(IPL_VM, PvHead::new()),
            attrs: AtomicU32::new(0),
            ptp_refs: AtomicU32::new(0),
        }
    }

    /// Clears the pmap's attribute bits of a page the page system hands out again
    /// (`uvm_pmr_getpages`, at allocation: the C's `atomic_clearbits_int(&pg->pg_flags,
    /// PG_PMAPMASK)` there). Panics when a byte view still holds the page: it was freed under
    /// the view, and its next owner would write what the view reads.
    pub fn clear_attrs(&self) {
        if self.in_view() {
            hal_panic(format_args!(
                "PageMd: a page in a byte view handed out again"
            ));
        }
        self.attrs.store(0, Ordering::Relaxed);
    }

    /// The `PG_PMAP_*` bits now (a snapshot, for assertions and ddb).
    pub fn attrs(&self) -> u32 {
        self.attrs.load(Ordering::Relaxed) & !PG_HAL_MASK
    }

    /// Whether the frame is held alone now, by a byte view or a copy (`uvm_pageclean` refuses
    /// to free such a page).
    pub fn in_view(&self) -> bool {
        self.attrs.load(Ordering::Acquire) & PG_HAL_VIEW != 0
    }

    /// Claims the frame alone, for a view (`writable` when it may write) or a copy: `true`
    /// when nothing held it (Acquire, so the bytes the last holder wrote are visible), `false`
    /// when something does (and then nothing changes).
    pub(crate) fn claim_view(&self, writable: bool) -> bool {
        let bits = if writable {
            PG_HAL_VIEW | PG_HAL_VIEW_W
        } else {
            PG_HAL_VIEW
        };
        self.attrs
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |a| {
                (a & PG_HAL_VIEW == 0).then_some(a | bits)
            })
            .is_ok()
    }

    /// Gives a claim up (Release: what the holder wrote is seen by the next claim).
    pub(crate) fn release_view(&self) {
        self.attrs.fetch_and(!PG_HAL_MASK, Ordering::Release);
    }

    /// Panics when lowering the page's mappings to `prot` would pull a live byte view's
    /// mapping from under it: removing them (no `PROT_READ`), or write-protecting a writable
    /// view.
    fn check_protect(&self, prot: VmProt) {
        let a = self.attrs.load(Ordering::Acquire);
        if a & PG_HAL_VIEW != 0
            && (prot & PROT_READ == 0 || (a & PG_HAL_VIEW_W != 0 && prot & PROT_WRITE == 0))
        {
            hal_panic(format_args!(
                "pmap_page_protect: a page in a byte view, to {prot:#x}"
            ));
        }
    }
}

impl Default for PageMd {
    fn default() -> Self {
        Self::new()
    }
}

/// The kernel's physical pages, as the pmap needs them (D16): the kernel's page system
/// implements it (`kernel::vm::page::PHYS_PAGES`, N3.3; `bsd`'s `uvm_page` did before), and the
/// boot path passes it to [`bootstrap`] as a value (an API, no longer a bridge row).
///
/// The contract, on which the pmap's table views rest (a documented trust boundary, as the
/// memory bridges', `sys/hal/trust.rs`): a frame `alloc_ptp` or `steal` hands out is RAM inside
/// the direct map that nothing else uses until it is given back to `free_ptp` (stolen frames
/// are never given back); `md_of` answers the same `PageMd` for a frame for ever.
pub trait PhysPages: Sync {
    /// `uvm_pagealloc(NULL, 0, NULL, UVM_PGA_USERESERVE | UVM_PGA_ZERO)` for a page-table
    /// page: a zeroed frame, wired; `None` when there is none now (the caller fails under
    /// `PMAP_CANFAIL`, or panics as the C does).
    fn alloc_ptp(&self) -> Option<Paddr>;
    /// `uvm_pagefree`: gives back a frame `alloc_ptp` handed out, after the TLB shootdown's
    /// wait (no CPU can still walk it).
    fn free_ptp(&self, pa: Paddr);
    /// `PHYS_TO_VM_PAGE(pa)->mdpage`: the `PageMd` of a managed frame; `None` for a frame
    /// outside the managed pages (device memory, a frame stolen at boot).
    fn md_of(&self, pa: Paddr) -> Option<&'static PageMd>;
    /// `pg->pg_flags & PG_BUSY` of the managed frame `pa` (`false` for any other frame): the
    /// page is held by its owner for I/O, the only pages a pager maps for their bytes
    /// (`uvm_pagermapin`'s `KASSERT(pp->pg_flags & PG_BUSY)`).
    fn is_busy(&self, pa: Paddr) -> bool;
    /// `uvm_page_physsteal(npg)`: `npg` contiguous frames taken out of the page system before
    /// it is up, not zeroed; `None` when there are none. Panics once `page_init_done` (as the
    /// C does).
    fn steal(&self, npg: usize) -> Option<Paddr>;
    /// `uvm.page_init_done`: whether `uvm_page_init` has run (page-table pages then come from
    /// `alloc_ptp`, before it from `steal`).
    fn page_init_done(&self) -> bool;
}

/// The kernel image, for a machine whose pmap maps it by section and translates its addresses
/// (arm64's `pmap_map_stolen` and `pmap_extract` of the image outside the direct map).
#[derive(Clone, Copy, Debug)]
pub struct KernelImage {
    /// The image's first virtual address, where its text starts (`__text_start`,
    /// `PMAP_KERNEL_IMAGE_START`).
    pub start: Vaddr,
    /// The end of the text (`_etext`).
    pub text_end: Vaddr,
    /// The first address of the read-only data (`__rodata_start`).
    pub rodata_start: Vaddr,
    /// Its end (`_erodata`).
    pub rodata_end: Vaddr,
    /// The first address of the data (`__data_start`): data and BSS up to `end`.
    pub data_start: Vaddr,
    /// Its end (`_end`).
    pub end: Vaddr,
    /// Physical minus virtual (`PMAP_KERNEL_IMAGE_PHYS_OFF`).
    pub phys_off: usize,
}

/// What the boot glue knows that `pmap_bootstrap` needs (D20): what LZ's machdep wrote into the
/// pmap's statics before calling it.
#[derive(Clone, Copy, Debug)]
pub struct BootstrapParams {
    /// The direct map's first address (`PMAP_DIRECT_BASE`: Limine's HHDM, or `locore0`'s).
    pub direct_base: Vaddr,
    /// Its end (`PMAP_DIRECT_END`).
    pub direct_end: Vaddr,
    /// amd64: the boot loader enabled no-execute (`EFER.NXE`, so `pg_nx` is `PG_NX`); ignored
    /// elsewhere.
    pub nx: bool,
    /// The first free physical page (amd64's `first_avail` argument).
    pub first_avail: Paddr,
    /// The kernel image (arm64; `None` elsewhere).
    pub image: Option<KernelImage>,
}

/// What [`bootstrap`] answers: the two values the machines' `pmap_bootstrap` return, and the
/// kernel map's token.
pub struct Bootstrapped {
    /// The first free physical page after the bootstrap (amd64's return value).
    pub first_avail: Paddr,
    /// The first free kernel virtual address (arm64's return value; amd64's `virtual_avail`,
    /// which its boot glue stores as the kernel map's base).
    pub virtual_avail: Vaddr,
    /// The kernel map's authority, handed out this once.
    pub mapper: KernelMapper,
    /// The context switch's authority to load a pmap's root ([`Pmap::load`]), handed out this
    /// once.
    pub switcher: Switcher,
}

/// The authority to switch the running CPU's address space ([`Pmap::load`], arm64's
/// [`Pmap::load_efi`] and [`leave_efi`]): zero-sized, not `Clone`, without a public
/// constructor; [`bootstrap`] returns the only one, which the boot path gives to the
/// scheduler's context switch. Possession is the proof that the caller records the switch
/// where the shootdowns look (`ci_proc_pmap`) and keeps the pmap referenced while it runs.
pub struct Switcher {
    _private: (),
}

/// The authority to change the kernel pmap (`pmap_kenter_pa`, `pmap_kremove`, `pmap_enter`
/// and friends on `pmap_kernel()`): zero-sized, not `Clone`, without a public constructor;
/// [`bootstrap`] returns the only one. Possession is the proof that the caller owns the
/// kernel virtual addresses it maps (the kernel map's allocation record, `uvm_km`'s at N3.2),
/// which no check here could establish.
pub struct KernelMapper {
    _private: (),
}

impl KernelMapper {
    /// `pmap_kenter_pa(va, pa, prot)`: maps `pa` at `va` in the kernel pmap, wired, unmanaged
    /// (no pv entry), with `prot`; an old valid entry is replaced and shot down.
    pub fn kenter(&self, va: Vaddr, pa: Paddr, prot: VmProt) {
        Backend::kenter(va, pa, prot);
    }

    /// `pmap_kremove(va, len)`: removes `len` bytes of `kenter` mappings from `va`.
    pub fn kremove(&self, va: Vaddr, len: Vsize) {
        Backend::kremove(va, len);
    }

    /// `pmap_enter(pmap_kernel(), va, pa, prot, flags)`: a managed kernel mapping (as
    /// [`Pmap::enter`], in the kernel pmap).
    pub fn enter(&self, va: Vaddr, pa: Paddr, prot: VmProt, flags: i32) -> Result<(), Errno> {
        Backend::enter(kernel(), va, pa, prot, flags)
    }

    /// `pmap_remove(pmap_kernel(), sva, eva)`.
    pub fn remove(&self, sva: Vaddr, eva: Vaddr) {
        Backend::remove(kernel(), sva, eva);
    }

    /// `pmap_protect(pmap_kernel(), sva, eva, prot)`.
    pub fn protect(&self, sva: Vaddr, eva: Vaddr, prot: VmProt) {
        Backend::protect(kernel(), sva, eva, prot);
    }

    /// `pmap_kenter_cache(va, pa, prot, cacheable)`: as [`KernelMapper::kenter`] with the
    /// memory type `cacheable` (`PMAP_CACHE_*`: device, uncached, write-through, write-back).
    #[cfg(target_arch = "aarch64")]
    pub fn kenter_cache(&self, va: Vaddr, pa: Paddr, prot: VmProt, cacheable: i32) {
        Backend::kenter_cache(va, pa, prot, cacheable);
    }
}

/// What amd64's TLB shootdown sends through (`pmap_tlb_shootpage`, `_shootrange`,
/// `_shoottlb`): `LAPIC_IPI_INVLPG`, `LAPIC_IPI_INVLRANGE`, `LAPIC_IPI_INVLTLB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlbIpi {
    /// One page (`TLB_SHOOT_ADDR1`).
    Page,
    /// A range (`TLB_SHOOT_ADDR1` to `TLB_SHOOT_ADDR2`).
    Range,
    /// The whole TLB.
    Tlb,
}

/// The CPUs, as amd64's shootdown and cache flush need them (`CPU_INFO_FOREACH`,
/// `cpu_is_running`, `ci_proc_pmap`, `x86_fast_ipi`, `wbinvd_on_all_cpus`, `ci_cflushsz`):
/// `bsd`'s CPU code registers it (bridges.toml; paid by N4, when the CPUs are `hal`'s and the
/// kernel's own). arm64 broadcasts its TLB invalidations and registers none. Before
/// registration there is one CPU: nothing to shoot.
pub trait TlbCpus: Sync {
    /// Calls `f(cpuid, ci_proc_pmap)` for every running CPU other than the caller's; the
    /// pointer is the pmap that CPU runs user code on ([`Pmap::as_ptr`], null for none),
    /// only compared, never dereferenced.
    fn for_each_other_running(&self, f: &mut dyn FnMut(usize, *const Pmap));
    /// `x86_fast_ipi(ci, ipi)`: whether the IPI was sent. A CPU that could not take it is
    /// counted as done (`pmap_tlb_shootfail`: its count decremented, the lock released with
    /// the last one, `pmap.c`), not a panic.
    fn send(&self, cpuid: usize, ipi: TlbIpi) -> bool;
    /// `wbinvd_on_all_cpus()`: write back and invalidate every CPU's caches.
    fn wbinvd_all(&self);
    /// `curcpu()->ci_cflushsz`: the `clflush` line size, 0 when the CPU has none.
    fn cflushsz(&self) -> usize;
}

/// `arc4random(9)`, as arm64's ASID allocator draws its first guess (`pmap_allocate_asid`):
/// `bsd`'s `dev/rnd.rs` registers it (bridges.toml; paid by N7). Before registration the
/// allocator searches linearly from the start.
pub trait Random: Sync {
    /// `arc4random()`.
    fn random_u32(&self) -> u32;
}

/// The arch half of `pmap(9)`: what each backend (`crate::arch::current::pmap::Backend`)
/// implements, one function per entry point. Generic code here makes the checks and holds the
/// handles; the backend owns the tables, the locks' contents and the TLB.
pub(crate) trait PmapBackend {
    /// `__HAVE_PMAP_DIRECT`.
    const HAVE_PMAP_DIRECT: bool;
    /// `PMAP_STEAL_MEMORY`.
    const PMAP_STEAL_MEMORY: bool;
    /// `PMAP_WC`.
    const PMAP_WC: usize;
    /// `PMAP_NOCACHE`.
    const PMAP_NOCACHE: usize;
    /// No MMU (the host double).
    const PMAP_NOMMU: bool;
    /// The user pmaps' pool: its name (`vmstat -m`), level and flags, as the C's `pool_init`.
    const POOL_WCHAN: &'static str;
    /// The pool's `ipl`.
    const POOL_IPL: i32;
    /// The pool's `flags`.
    const POOL_FLAGS: i32;

    /// The kernel pmap, the backend's static (`Pmap::kernel_with`).
    fn kernel() -> &'static Pmap;
    /// `pmap_bootstrap`: once, on the boot CPU, the kernel pmap's provider already set; the
    /// first free page and the first free kernel address.
    fn bootstrap(p: &BootstrapParams) -> (Paddr, Vaddr);
    /// `pmap_init`'s own pools (pv entries, pteds, ...), after the user pmaps' pool.
    fn init();
    /// `pmap_initialized`.
    fn initialized() -> bool;
    /// The first address `pmap` may not map ([`Pmap::va_limit`]).
    fn va_limit(pmap: &Pmap) -> Vaddr;
    /// `pmap_create`'s machine part: an empty user pmap's tables with the kernel half
    /// (`stats` to account the root, as amd64's `resident_count = 1`); `None` when no page
    /// could be had.
    fn create(stats: &PmapStatistics, pages: &'static dyn PhysPages) -> Option<PmapMd>;
    /// The leaf mappings left in a user pmap (for the last drop's check).
    fn mappings(pmap: &Pmap) -> usize;
    /// `pmap_release`: frees a user pmap's tables (no mapping is left).
    fn destroy(pmap: &mut Pmap);
    /// `pmap_enter`.
    fn enter(pmap: &Pmap, va: Vaddr, pa: Paddr, prot: VmProt, flags: i32) -> Result<(), Errno>;
    /// `pmap_remove`.
    fn remove(pmap: &Pmap, sva: Vaddr, eva: Vaddr);
    /// `pmap_protect`.
    fn protect(pmap: &Pmap, sva: Vaddr, eva: Vaddr, prot: VmProt);
    /// `pmap_extract`.
    fn extract(pmap: &Pmap, va: Vaddr) -> Option<Paddr>;
    /// `pmap_unwire`.
    fn unwire(pmap: &Pmap, va: Vaddr);
    /// `pmap_kenter_pa`.
    fn kenter(va: Vaddr, pa: Paddr, prot: VmProt);
    /// `pmap_kremove`.
    fn kremove(va: Vaddr, len: Vsize);
    /// [`Pmap::load`].
    fn load(pmap: &Pmap);
    /// [`create_efi`]'s machine part: a privileged four-level pmap whose limit is
    /// `1 << EFI_SPACE_BITS`.
    #[cfg(target_arch = "aarch64")]
    fn create_efi(stats: &PmapStatistics, pages: &'static dyn PhysPages) -> Option<PmapMd>;
    /// [`Pmap::load_efi`].
    #[cfg(target_arch = "aarch64")]
    fn load_efi(pmap: &Pmap);
    /// [`leave_efi`].
    #[cfg(target_arch = "aarch64")]
    fn leave_efi(to: &Pmap);
    /// [`Pmap::activate`].
    fn activate(pmap: &Pmap);
    /// [`Pmap::deactivate`].
    fn deactivate(pmap: &Pmap);
    /// [`Pmap::purge`].
    fn purge(pmap: &Pmap);
    /// [`Pmap::proc_iflush`].
    fn proc_iflush(pmap: &Pmap, va: Vaddr, len: Vsize);
    /// `pmap_page_protect`.
    fn page_protect(md: &PageMd, prot: VmProt);
    /// `pmap_clear_modify`.
    fn clear_modify(md: &PageMd) -> bool;
    /// `pmap_clear_reference`.
    fn clear_reference(md: &PageMd) -> bool;
    /// `pmap_is_modified`.
    fn is_modified(md: &PageMd) -> bool;
    /// `pmap_is_referenced`.
    fn is_referenced(md: &PageMd) -> bool;
    /// `pmap_zero_page`.
    fn zero_page(pa: Paddr);
    /// `pmap_copy_page`.
    fn copy_page(src: Paddr, dst: Paddr);
    /// `pmap_steal_memory`'s work, with the kernel pmap's provider (the page system not up).
    fn steal_memory(pages: &'static dyn PhysPages, size: Vsize) -> Vaddr;
    /// `pmap_virtual_space`.
    fn virtual_space() -> (Vaddr, Vaddr);
    /// `pmap_growkernel`.
    fn growkernel(maxkvaddr: Vaddr) -> Vaddr;
    /// `PMAP_DIRECT_MAP`.
    fn direct_map(pa: Paddr) -> Vaddr;
    /// `PMAP_DIRECT_UNMAP`, `None` outside the direct map.
    fn direct_unmap(va: Vaddr) -> Option<Paddr>;
    /// `pmap_flush_cache`.
    fn flush_cache(va: Vaddr, len: Vsize);
    /// [`module_window`].
    fn module_window() -> Option<(Vaddr, usize)>;
    /// [`module_enter`].
    fn module_enter(va: Vaddr, pa: Paddr, prot: VmProt) -> bool;
    /// [`Pmap::cr3`].
    #[cfg(target_arch = "x86_64")]
    fn cr3(pmap: &Pmap) -> u64;
    /// [`KernelMapper::kenter_cache`].
    #[cfg(target_arch = "aarch64")]
    fn kenter_cache(va: Vaddr, pa: Paddr, prot: VmProt, cacheable: i32);
    /// [`Pmap::fault_fixup`].
    #[cfg(target_arch = "aarch64")]
    fn fault_fixup(pmap: &Pmap, va: Vaddr, ftype: VmProt) -> bool;
}

/// `pmap_kernel()`: the kernel's pmap.
pub fn kernel() -> &'static Pmap {
    Backend::kernel()
}

/// `pmap_bootstrap`: adopts the page tables the boot loader left (amd64 `%cr3`'s PML4 and its
/// recursive slot, arm64 `TTBR1_EL1`'s tables copied into the kernel's own), with `p` and the
/// page provider `pages`, which the kernel pmap keeps and every user pmap copies. Once, on
/// the boot CPU, before anything maps; a second call panics. The boot glue's facts in `p` are
/// trusted, as the bridges' boot registration is (decision 30).
pub fn bootstrap(p: &BootstrapParams, pages: &'static dyn PhysPages) -> Bootstrapped {
    static DONE: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
    if DONE.swap(true, Ordering::AcqRel) {
        hal_panic(format_args!("pmap_bootstrap: called twice"));
    }
    if kernel().pages.set(pages).is_err() {
        hal_panic(format_args!(
            "pmap_bootstrap: the kernel pmap has a provider"
        ));
    }
    let (first_avail, virtual_avail) = Backend::bootstrap(p);
    Bootstrapped {
        first_avail,
        virtual_avail,
        mapper: KernelMapper { _private: () },
        switcher: Switcher { _private: () },
    }
}

/// `pmap_init`: the pmap's pools, once the pool allocator is up (`uvm_init`); a second call
/// does nothing. Panics when the pool allocator is not registered (the C's `pool_init` cannot
/// fail).
pub fn init() {
    // Recorded: the pv entries name their pmap by address and give a handle back from it
    // (`PoolArc::from_value`), so the pool never gives a pmap mutably.
    match PMAP_POOL.init_recorded(Backend::POOL_IPL, Backend::POOL_FLAGS, Backend::POOL_WCHAN) {
        Ok(()) => Backend::init(),
        Err(Errno::EBUSY) => {}
        Err(e) => hal_panic(format_args!("pmap_init: the pmap pool: {e:?}")),
    }
}

/// `pmap_initialized`: whether [`init`] has run.
pub fn initialized() -> bool {
    Backend::initialized()
}

/// `pmap_create()`: a new, empty user pmap with one reference, whose top-level table maps the
/// kernel half. `None` before [`bootstrap`] and [`init`] (between them the provider has no
/// page system to allocate from), or when no page could be had (the C's
/// `pool_get(PR_WAITOK)` never fails: a caller after boot may treat `None` as fatal).
pub fn create() -> Option<PmapRef> {
    create_with(Backend::create)
}

/// `efi_map_runtime`'s `pmap_create` (arm64): a privileged pmap with four-level tables whose
/// limit is `1 << EFI_SPACE_BITS` instead of `VM_MAXUSER_ADDRESS`, for the identity mappings of
/// the EFI runtime services; loaded by [`Pmap::load_efi`], left by [`leave_efi`]. `None` as
/// for [`create`].
#[cfg(target_arch = "aarch64")]
pub fn create_efi() -> Option<PmapRef> {
    create_with(Backend::create_efi)
}

/// `efi_leave`'s pmap half (arm64): the user address-space width back in `TCR_EL1` and `to`
/// (the pmap the CPU ran on before `efi_enter`, `ci_curpm`) loaded again.
#[cfg(target_arch = "aarch64")]
pub fn leave_efi(to: &Pmap, _switch: &Switcher) {
    Backend::leave_efi(to);
}

/// A user pmap whose machine part `make` builds, in the pool of user pmaps.
fn create_with(
    make: fn(&PmapStatistics, &'static dyn PhysPages) -> Option<PmapMd>,
) -> Option<PmapRef> {
    if !initialized() {
        return None;
    }
    let pages = *kernel().pages.get()?;
    let stats = PmapStatistics::new();
    let md = make(&stats, pages)?;
    let pmap = Pmap {
        stats,
        pages: OnceStatic::new(),
        md,
    };
    // A fresh cell: the set cannot fail.
    let _ = pmap.pages.set(pages);
    PMAP_POOL.get_shared(PR_WAITOK, || pmap).map(PmapRef::User)
}

/// `pmap_page_protect(pg, prot)`: lowers every mapping of the page to `prot`; `PROT_NONE`
/// removes them all (what the object layer does before freeing a page).
///
/// Panics when a byte view holds the page and `prot` would take its mapping away (the
/// window is a managed mapping: freeing a page removes it): the view's owner gives the page
/// up first.
pub fn page_protect(md: &PageMd, prot: VmProt) {
    md.check_protect(prot);
    Backend::page_protect(md, prot);
}

/// `pmap_clear_modify(pg)`: clears the modified bit of the page and its mappings; whether it
/// was set (always false on arm64, which emulates it).
pub fn clear_modify(md: &PageMd) -> bool {
    Backend::clear_modify(md)
}

/// `pmap_clear_reference(pg)`: clears the referenced bit; whether it was set.
pub fn clear_reference(md: &PageMd) -> bool {
    Backend::clear_reference(md)
}

/// `pmap_is_modified(pg)`.
pub fn is_modified(md: &PageMd) -> bool {
    Backend::is_modified(md)
}

/// `pmap_is_referenced(pg)`.
pub fn is_referenced(md: &PageMd) -> bool {
    Backend::is_referenced(md)
}

/// The `PageMd` of `pa`; panics unless the provider knows it as a managed page (the C's
/// `struct vm_page *`).
fn managed(what: &str, pa: Paddr) -> &'static PageMd {
    match kernel().pages().md_of(pa) {
        Some(md) => md,
        None => hal_panic(format_args!("{what}: {pa:?} is not a managed page")),
    }
}

/// Claims `md` (the frame `pa`) alone for `what`'s write; panics when a byte view holds it.
fn hold(what: &str, md: &PageMd, pa: Paddr) {
    if !md.claim_view(true) {
        hal_panic(format_args!("{what}: {pa:?} is in a byte view"));
    }
}

/// `pmap_zero_page`: zero-fills the managed page at `pa` (through the direct map). Panics
/// when `pa` is not a managed page (see the module's redesign), and when a byte view holds
/// it (the page is held alone for the fill).
pub fn zero_page(pa: Paddr) {
    let md = managed("pmap_zero_page", pa);
    hold("pmap_zero_page", md, pa);
    Backend::zero_page(pa);
    md.release_view();
}

/// `pmap_copy_page`: copies the managed page at `src` into the managed page at `dst`. Panics
/// when either is not a managed page or a byte view holds either (both are held alone for
/// the copy).
pub fn copy_page(src: Paddr, dst: Paddr) {
    let s = managed("pmap_copy_page", src);
    let d = managed("pmap_copy_page", dst);
    hold("pmap_copy_page", s, src);
    if !d.claim_view(true) {
        s.release_view();
        hal_panic(format_args!("pmap_copy_page: {dst:?} is in a byte view"));
    }
    Backend::copy_page(src, dst);
    d.release_view();
    s.release_view();
}

/// `pmap_steal_memory(size, ..)`: `size` bytes of physical memory taken from the provider the
/// kernel pmap was given, before the page system is up, mapped and zeroed; their kernel
/// address. [`virtual_space`] then answers the kernel virtual space still free (the C's
/// `*vstartp`, `*vendp`). Panics before [`bootstrap`], once the page system is up, when
/// the memory is not there (the C's "out of memory" panic), and on a size of zero or past
/// `isize::MAX`: the first would take no page and still hand out an address, the second is
/// no object's size, and only below it can the backends' page rounding not wrap to 0.
pub fn steal_memory(size: Vsize) -> Vaddr {
    let pages = kernel().pages();
    if pages.page_init_done() {
        hal_panic(format_args!("pmap_steal_memory: after uvm_page_init"));
    }
    let bytes = size.as_usize();
    if bytes == 0 || bytes > isize::MAX.unsigned_abs() {
        hal_panic(format_args!("pmap_steal_memory: size {bytes:#x}"));
    }
    Backend::steal_memory(pages, size)
}

/// `pmap_virtual_space`: the kernel virtual range still free, `(start, end)`.
pub fn virtual_space() -> (Vaddr, Vaddr) {
    Backend::virtual_space()
}

/// `pmap_growkernel(maxkvaddr)`: grows the kernel page tables to cover `maxkvaddr`; how far
/// they now reach.
pub fn growkernel(maxkvaddr: Vaddr) -> Vaddr {
    Backend::growkernel(maxkvaddr)
}

/// `PMAP_DIRECT_MAP(pa)`: the direct-map address of a physical address (`HAVE_PMAP_DIRECT`).
pub fn direct_map(pa: Paddr) -> Vaddr {
    Backend::direct_map(pa)
}

/// `PMAP_DIRECT_UNMAP(va)`: the physical address behind a direct-map address; `None` when
/// `va` is not in the direct map (`pmap_unmap_direct`'s caller then has no page).
pub fn direct_unmap(va: Vaddr) -> Option<Paddr> {
    Backend::direct_unmap(va)
}

/// `pmap_flush_cache(va, len)`: writes back and invalidates the data cache over
/// `[va, va + len)` (amd64 `clflush` between two `mfence`s, or `wbinvd` on every CPU without
/// `clflush`; the arm64 data-cache operations).
pub fn flush_cache(va: Vaddr, len: Vsize) {
    Backend::flush_cache(va, len);
}

/// The kernel virtual window the ABI module `abi_openbsd.ko` must run from (decision 22): its
/// first address and its size, outside `kernel_map`, for the kernel's lifetime; `None` on a
/// machine without one (the host double; amd64 and arm64 have one, N3.1). Its size is 0 when
/// the bootstrap could not link it. Without a window, or with an empty one, the module is not
/// loaded. The module loader's (`sys/hal/module/load.rs`).
pub(crate) fn module_window() -> Option<(Vaddr, usize)> {
    Backend::module_window()
}

/// Maps `pa` at `va`, a page of [`module_window`], in the kernel pmap with `prot`, wired, for
/// the kernel's lifetime: the module's pages. `false`, mapping nothing, when `va` is not a page
/// of the window, is mapped already, `pa` is not a page's address, or `prot` is both writable
/// and executable (W^X); always `false` without a window. The module loader's, as
/// [`module_window`].
pub(crate) fn module_enter(va: Vaddr, pa: Paddr, prot: VmProt) -> bool {
    Backend::module_enter(va, pa, prot)
}

// The pmap and the page data are shared between CPUs: a pmap's handles cross them, every page
// is reached from any CPU.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<Pmap>();
    shared::<PageMd>();
    shared::<PmapRef>();
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;
    use crate::arch::current::param::PAGE_SIZE;
    use crate::sys::mman::{PROT_NONE, PROT_READ, PROT_WRITE};
    use crate::testutil;

    /// The tests that count the pmap pool's items run one at a time (the pool is shared).
    fn serial() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The kernel map's token of this test process.
    fn booted() -> &'static KernelMapper {
        &boot().mapper
    }

    /// The context switch's token of this test process.
    #[cfg_attr(not(target_arch = "aarch64"), allow(dead_code))]
    fn switcher() -> &'static Switcher {
        &boot().switcher
    }

    /// The bridges' doubles, the bootstrap and `pmap_init`, once per test process (shared
    /// with every test module of the crate, `testutil::pmap_boot`).
    fn boot() -> &'static Bootstrapped {
        testutil::pmap_boot()
    }

    #[test]
    fn statistics_start_at_zero_and_keep_what_is_stored() {
        let st = PmapStatistics::new();
        assert_eq!((st.resident_count.get(), st.wired_count.get()), (0, 0));
        st.resident_count.set(st.resident_count.get() + 3);
        st.wired_count.set(-1);
        assert_eq!((st.resident_count.get(), st.wired_count.get()), (3, -1));
    }

    #[test]
    fn a_user_pmap_is_freed_once_by_its_last_handle() {
        let _ = booted();
        let _s = serial();
        assert!(initialized());
        let before = PMAP_POOL.nout();
        let one = create().unwrap();
        let two = one.clone();
        assert!(PmapRef::ptr_eq(&one, &two));
        assert!(!one.is_kernel());
        assert_eq!(PMAP_POOL.nout(), before + 1);
        drop(one);
        assert_eq!(PMAP_POOL.nout(), before + 1);
        drop(two);
        assert_eq!(PMAP_POOL.nout(), before);
    }

    #[test]
    #[should_panic(expected = "mappings left")]
    fn the_last_drop_with_a_mapping_left_panics() {
        let _ = booted();
        let _s = serial();
        let pm = create().unwrap();
        pm.enter(Vaddr::new(0x1000), Paddr::new(0x5000), PROT_READ, PROT_READ)
            .unwrap();
        drop(pm);
    }

    #[test]
    fn the_kernel_handle_is_the_kernel_pmap() {
        let k = PmapRef::Kernel;
        assert!(k.is_kernel());
        assert!(core::ptr::eq(k.as_ptr(), kernel()));
        assert!(PmapRef::ptr_eq(&k, &k.clone()));
    }

    #[test]
    #[should_panic(expected = "called twice")]
    fn the_kernel_mapper_is_handed_out_once() {
        let _ = booted();
        let params = BootstrapParams {
            direct_base: Vaddr::new(0),
            direct_end: Vaddr::new(0),
            nx: false,
            first_avail: Paddr::new(0),
            image: None,
        };
        let _again = bootstrap(&params, &testutil::FAKE_PAGES);
    }

    #[test]
    fn a_user_pmap_maps_extracts_and_removes() {
        let _ = booted();
        let _s = serial();
        let pm = create().unwrap();

        let va = Vaddr::new(0x40_0000);
        pm.enter(va, Paddr::new(0x7000), PROT_READ | PROT_WRITE, PMAP_WIRED)
            .unwrap();
        assert_eq!(pm.extract(Vaddr::new(0x40_0123)), Some(Paddr::new(0x7123)));
        assert_eq!(pm.resident_count(), 1);
        pm.protect(va, va + Vsize::new(PAGE_SIZE), PROT_NONE);
        assert_eq!(pm.extract(va), None);
        pm.enter(va, Paddr::new(0x8000), PROT_READ, 0).unwrap();
        pm.remove(va, va + Vsize::new(PAGE_SIZE));
        assert_eq!(pm.resident_count(), 0);
    }

    #[test]
    #[should_panic(expected = "not in the user half")]
    fn a_user_pmap_refuses_the_kernel_half() {
        let _ = booted();
        let _s = serial();
        let pm = create().unwrap();

        let top = Vaddr::new(crate::arch::current::vmparam::VM_MAXUSER_ADDRESS);
        let _ = pm.enter(top, Paddr::new(0x9000), PROT_READ, 0);
    }

    #[test]
    #[should_panic(expected = "not in the user half")]
    fn a_user_removal_may_not_reach_the_kernel_half() {
        let _ = booted();
        let _s = serial();
        let pm = create().unwrap();

        let top = crate::arch::current::vmparam::VM_MAXUSER_ADDRESS;
        pm.remove(Vaddr::new(0), Vaddr::new(top + PAGE_SIZE));
    }

    #[test]
    #[should_panic(expected = "through the KernelMapper")]
    fn the_kernel_pmap_is_not_entered_without_the_token() {
        let _ = booted();
        let _ = kernel().enter(Vaddr::new(0x1000), Paddr::new(0x1000), PROT_READ, 0);
    }

    #[test]
    fn the_token_maps_the_kernel_pmap_low_addresses_included() {
        let mapper = booted();
        let (start, end) = virtual_space();
        assert!(start < end);
        let va = start + Vsize::new(0x10_0000 * PAGE_SIZE);
        mapper.kenter(va, Paddr::new(0xa000), PROT_READ | PROT_WRITE);
        assert_eq!(kernel().extract(va), Some(Paddr::new(0xa000)));
        mapper.kremove(va, Vsize::new(PAGE_SIZE));
        assert_eq!(kernel().extract(va), None);
        // amd64's MP trampoline is entered at its low identity address (cpu.c).
        let tramp = Vaddr::new(0x1000 * 7);
        mapper.kenter(tramp, Paddr::new(0x7000), PROT_READ);
        assert_eq!(kernel().extract(tramp), Some(Paddr::new(0x7000)));
        mapper.kremove(tramp, Vsize::new(PAGE_SIZE));
    }

    #[test]
    #[should_panic(expected = "not a managed page")]
    fn zero_page_refuses_a_frame_that_is_not_a_managed_page() {
        let _ = booted();
        // The fake provider manages no page.
        zero_page(Paddr::new(0x30_0000));
    }

    #[test]
    fn the_user_limit_is_the_user_half() {
        let _ = booted();
        let _s = serial();
        let pm = create().unwrap();
        assert_eq!(
            pm.va_limit(),
            Vaddr::new(crate::arch::current::vmparam::VM_MAXUSER_ADDRESS)
        );
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn the_efi_pmap_maps_above_the_user_half() {
        let _ = booted();
        let _s = serial();
        let pm = create_efi().unwrap();
        let top = Vaddr::new(crate::arch::current::vmparam::VM_MAXUSER_ADDRESS);
        assert!(pm.va_limit() > top);
        pm.enter(top, Paddr::new(0xb000), PROT_READ, 0).unwrap();
        pm.load_efi(switcher());
        leave_efi(kernel(), switcher());

        pm.remove(top, top + Vsize::new(PAGE_SIZE));
        assert_eq!(pm.extract(top), None);
    }

    #[test]
    fn a_page_starts_clean_and_clear_attrs_cleans_it() {
        let md = PageMd::new();
        assert_eq!(md.attrs(), 0);
        md.attrs
            .store(PG_PMAP_MOD | PG_PMAP_REF | PG_PMAP2, Ordering::Relaxed);
        md.clear_attrs();
        assert_eq!(md.attrs(), 0);
        assert!(!is_modified(&md));
        assert!(!clear_reference(&md));
    }

    #[test]
    fn a_byte_view_claims_a_frame_alone_and_the_pmap_bits_ignore_it() {
        let md = PageMd::new();
        assert!(md.claim_view(true));
        assert!(md.in_view());
        assert!(!md.claim_view(false));
        md.attrs.fetch_or(PG_PMAP_MOD, Ordering::Relaxed);
        assert_eq!(md.attrs(), PG_PMAP_MOD);
        md.release_view();
        assert!(!md.in_view());
        assert_eq!(md.attrs(), PG_PMAP_MOD);
        md.clear_attrs();
        assert_eq!(md.attrs(), 0);
        // A read-only view may be write-protected, not removed.
        assert!(md.claim_view(false));
        md.check_protect(PROT_READ);
        md.release_view();
        md.check_protect(PROT_NONE);
    }

    #[test]
    #[should_panic(expected = "handed out again")]
    fn a_page_in_a_view_is_not_handed_out_again() {
        let md = PageMd::new();
        assert!(md.claim_view(false));
        md.clear_attrs();
    }

    #[test]
    #[should_panic(expected = "a page in a byte view, to 0x0")]
    fn a_page_in_a_view_keeps_its_mappings() {
        let _ = booted();
        let md = PageMd::new();
        assert!(md.claim_view(false));
        page_protect(&md, PROT_NONE);
    }

    #[test]
    #[should_panic(expected = "a page in a byte view, to 0x1")]
    fn a_writable_view_is_not_write_protected() {
        let md = PageMd::new();
        assert!(md.claim_view(true));
        md.check_protect(PROT_READ);
    }
}
/* </TESTS> */
