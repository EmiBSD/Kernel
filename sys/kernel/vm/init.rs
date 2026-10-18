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
 *
 * from: Id: uvm_init.c,v 1.1.2.3 1998/02/06 05:15:27 chs Exp
 */
/* </LICENSES> */

/* <CODE> */
//! Init the vm system: `uvm/uvm_init.c`, the kernel's steps of `uvm_init`.
//!
//! Upstream: sys/uvm/uvm_init.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_init.rs@3c62ede68802
//!
//! `uvm_init` is three steps here, around what `bsd` still owns (the C's order kept):
//! `bsd`'s `uvm_init` registers the bridges and hands down the machine's constants, runs the
//! page system's init over the machine's constraints (`uvm_page_init`), then
//! [`uvm_init_early`] (the map sub-system, `kernel_map` and the kernel object, the fault's
//! tuning), then `pmap_init`, `kmeminit` and `dma_alloc_init`, then [`uvm_init_late`] (the
//! pagers, the amaps, the kernel's swap object, the anons, the best-fit switches).
//!
//! ## Deviations
//! - `uvmexp_counters` (the C's per-CPU `COUNTERS_BOOT_MEMORY`) are the static atomics of
//!   `hal::vm::uvmexp`, so `uvm_init_percpu` does not call `counters_alloc_ncpus` (LZ's).
//! - `uvm_km_page_init` and `uvm_km_page_lateinit` (`!__HAVE_PMAP_DIRECT`'s single-page
//!   allocator) report themselves `unported`, as in LZ: both machines have a direct map.
//! - `averunnable.fscale = FSCALE` is `bsd`'s scheduler's (`sched_bsd`), set there (LZ's).
//!
//! ## Redesign
//! - The C switches both `kernel_map` and `kmem_map` to a best-fit allocator; LZ switched
//!   `kernel_map` only ("kmem_map waits for kmeminit's suballoc", though `kmeminit` had run).
//!   Here both, as the C (`uvmexp.kmapent` may count a few entries more than LZ's under
//!   `kmem_map`'s best-fit; the main session's decision of 2026-10-18).
//! - The machine's numbers the virtual memory system needs that `hal` does not carry
//!   (`<machine/param.h>`'s `USPACE`,
//!   `USPACE_ALIGN` and `HAVE_USPACE_GUARD`, `<machine/exec.h>`'s `__LDPGSZ`,
//!   `<machine/pmap.h>`'s `dma_constraint`) are the machine's traits in `bsd`'s arch: `bsd`'s
//!   `uvm_init` hands them down once as a [`VmMachine`] value ([`uvm_machine_init`]), which
//!   the kernel's modules read through [`machine`]. `<machine/vmparam.h>`'s values
//!   (`VM_MAXUSER_ADDRESS`, `DFLDSIZ`, `MAXSSIZ`, ...) are `hal`'s
//!   (`hal::arch::current::vmparam`).

use core::sync::atomic::Ordering;

use hal::once::OnceStatic;
use hal::sys::types::{Vaddr, Vsize};
use hal::vm::uvmexp::UVMEXP;

use super::addr::uaddr_bestfit_create;
use super::amap::amap_init;
use super::anon::{uvm_anon_init, uvm_anon_init_percpu};
use super::aobj::{UAO_FLAG_KERNSWAP, uao_create};
use super::fault::uvmfault_init;
use super::km::{VM_KERNEL_SPACE_SIZE, kernel_map, kmem_map, uvm_km_init};
use super::map::{UVM_MAXKADDR, UaddrSlot, uvm_map_init, uvm_map_set_uaddr};
use super::pager::uvm_pager_init;
use super::types::UvmConstraintRange;
use crate::assert::kpanic;
use crate::prf::unported;

/// The machine's constants of the virtual memory system, handed down once by `bsd`'s
/// `uvm_init` ([`uvm_machine_init`]).
#[derive(Clone, Copy, Debug)]
pub struct VmMachine {
    /// `__LDPGSZ`: the page size the linker assumes (`mmap(2)`'s alignment of a large
    /// anonymous mapping).
    pub ldpgsz: usize,
    /// `USPACE`: a thread's u-area (PCB and kernel stack).
    pub uspace: usize,
    /// `USPACE_ALIGN`: the u-area's alignment.
    pub uspace_align: usize,
    /// `__HAVE_USPACE_GUARD`: a guard page between the PCB and the stack.
    pub have_uspace_guard: bool,
    /// `dma_constraint`: the physical range every DMA-capable device reaches.
    pub dma_constraint: &'static UvmConstraintRange,
}

/// The machine's constants, once `uvm_init` handed them down.
static MACHINE: OnceStatic<&'static VmMachine> = OnceStatic::new();

/// Records the machine's constants, once (`bsd`'s `uvm_init`, first; the tests' set-up); a
/// second call keeps the first.
pub fn uvm_machine_init(m: &'static VmMachine) {
    let _ = MACHINE.set(m);
}

/// The machine's constants; panics before [`uvm_machine_init`] (nothing that reads them runs
/// before `uvm_init`).
pub fn machine() -> &'static VmMachine {
    match MACHINE.get() {
        Some(m) => m,
        None => kpanic(format_args!(
            "uvm: the machine's constants used before uvm_init"
        )),
    }
}

/// `uvm_init`'s first part, after `bsd`'s page system init (`uvm_page_init` with the
/// machine's constraints, whose `kvm_start`/`kvm_end` are `kvm`) and before `pmap_init`:
/// the map sub-system and the kernel's virtual memory (`kernel_map`, the kernel object), and
/// the fault's tuning. `base` is `vm_min_kernel_address`.
pub fn uvm_init_early(base: Vaddr, kvm: (Vaddr, Vaddr)) {
    // Ensure that the hardware set the page size.
    if UVMEXP.pagesize.load(Ordering::Relaxed) == 0 {
        kpanic(format_args!("uvm_init: page size not set"));
    }

    // Init the map sub-system. Allocates the static pool of vm_map_entry structures that are
    // used for "special" kernel maps (e.g. kernel_map, kmem_map, etc...).
    uvm_map_init();

    // Setup the kernel's virtual memory data structures. This includes setting up the
    // kernel_map/kernel_object. uvm_maxkaddr's static initialiser is VM_MIN_KERNEL_ADDRESS;
    // where that base is a runtime value (amd64 above its direct map) it starts there too.
    UVM_MAXKADDR.fetch_max(base.as_usize(), Ordering::Relaxed);
    uvm_km_init(base, kvm.0, kvm.1);

    // step 4.5: init (tune) the fault recovery code.
    uvmfault_init();
}

/// `uvm_init`'s last part, after `bsd`'s `pmap_init`, `kmeminit` and `dma_alloc_init` (the
/// C's order): the pagers, the amaps, the kernel's swap object, the anons, and the switch of
/// `kernel_map` and `kmem_map` to a best-fit allocator. `ipl` is `IPL_MPFLOOR`, the pools'.
pub fn uvm_init_late(ipl: i32) {
    // Init all pagers and the pager_map.
    if uvm_pager_init().is_err() {
        kpanic(format_args!(
            "uvm_pager_init: cannot make the pager's pools"
        ));
    }

    // step 9: init anonymous memory system
    if amap_init(ipl).is_err() {
        // init amap module
        kpanic(format_args!("amap_init: cannot make the amap pools"));
    }

    // step 10: start uvm_km_page allocator thread.
    let _ = unported!("uvm_km_page_lateinit");

    // the VM system is now up! now that malloc is up we can enable paging of kernel objects.
    let _ = uao_create(Vsize::new(VM_KERNEL_SPACE_SIZE), UAO_FLAG_KERNSWAP);

    // DEADBEEF0 / DEADBEEF1: not configured.

    // Init anonymous memory systems.
    if uvm_anon_init(ipl).is_err() {
        kpanic(format_args!("uvm_anon_init: cannot make anonpl"));
    }

    // Switch kernel and kmem_map over to a best-fit allocator, instead of walking the tree
    // (SMALL_KERNEL is not configured).
    for map in [kernel_map(), kmem_map()] {
        uvm_map_set_uaddr(
            map,
            UaddrSlot::Any(3),
            Some(uaddr_bestfit_create(map.min_offset(), map.max_offset())),
        );
    }
}

/// `uvm_init_percpu`: the per-CPU parts, once the CPUs are known.
pub fn uvm_init_percpu() {
    // uvmexp_counters = counters_alloc_ncpus(uvmexp_counters, exp_ncounters): the uvmexp
    // counters are static atomics (see the module's deviations), so there is nothing to move
    // to per-CPU memory.
    uvm_anon_init_percpu();
}
/* </CODE> */
