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

/*-
 * Copyright (c) 1991, 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)vm_extern.h	8.5 (Berkeley) 5/3/95
 */
/* </LICENSES> */

/* <CODE> */
//! The external interface of `uvm`: `<uvm/uvm_extern.h>`, `bsd`'s half since N3.2.
//!
//! Upstream: sys/uvm/uvm_extern.h @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_extern.rs@3c62ede68802
//!
//! The types and flags that name only the machine's own types are the kernel crate's
//! (`kernel::vm::types`), and the kernel memory allocation modes are its allocator's
//! (`kernel::vm::km`); both are re-exported here by name. What stays is what embeds or names
//! the map, still `bsd`'s (`uvm_map`, N3.5): `struct vmspace` and the `exec_map`/`phys_map`
//! submaps.
//!
//! ## Deviations
//! - `UVM_MAPFLAG` and its extractors are `const fn`s (`kernel::vm::types`).
//! - In `kmem_va_mode`, `kv_map` (a pointer to a map pointer) says which map by name
//!   (`kernel::vm::km::KvMap`).
//! - `exec_map`, which every machine's `machdep.c` defines and makes identically in
//!   `cpu_startup`, is one static here ([`EXEC_MAP`]) that the machines set. The same holds
//!   for `phys_map` ([`PHYS_MAP`]), the physio submap.
//!
//! ## Redesign
//! - N3.2 (DECISIONS G1): the header is split by owner. `kernel::vm::types` holds `voff_t`,
//!   `vm_prot_t` (one type with `hal::pmap::VmProt`), `vm_inherit_t`, `vm_fault_t`, the map
//!   flags and their encoding, the page allocation and lock flags, `uvm_constraint_range` and
//!   the core dump callbacks; `kernel::vm::km` holds `kmem_va_mode`, `kmem_pa_mode`,
//!   `kmem_dyn_mode` and `KMEM_DYN_INITIALIZER`. Their 49 users compile unchanged through the
//!   re-exports below.

use core::cell::Cell;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicPtr, Ordering};

pub use kernel::vm::km::{KMEM_DYN_INITIALIZER, KmemDynMode, KmemPaMode, KmemVaMode, KvMap};
pub use kernel::vm::types::{
    MADV_MASK, MAP_INHERIT_MASK, PHYSLOAD_DEVICE, PROT_MASK, UVM_FLAG_CONCEAL, UVM_FLAG_COPYONW,
    UVM_FLAG_FIXED, UVM_FLAG_HOLE, UVM_FLAG_NOFAULT, UVM_FLAG_NOMERGE, UVM_FLAG_OVERLAY,
    UVM_FLAG_QUERY, UVM_FLAG_SIGALTSTACK, UVM_FLAG_STACK, UVM_FLAG_TRYLOCK, UVM_FLAG_UNMAP,
    UVM_FLAG_WC, UVM_IO_FIXPROT, UVM_LK_ENTER, UVM_LK_EXIT, UVM_PGA_USERESERVE, UVM_PGA_ZERO,
    UVM_PLA_NOWAIT, UVM_PLA_NOWAKE, UVM_PLA_TRYCONTIG, UVM_PLA_USERESERVE, UVM_PLA_WAITOK,
    UVM_PLA_ZERO, UVM_UNKNOWN_OFFSET, UvmConstraintRange, UvmCoredumpSetupCb, UvmCoredumpWalkCb,
    VmFault, VmInherit, VmProt, Voff, uvm_advice, uvm_inherit, uvm_mapflag, uvm_maxprotection,
    uvm_protection,
};

use crate::sys::types::Segsz;
use crate::uvm::uvm_map::VmMap;

/// `struct vmspace`: shareable process virtual address space. May eventually be merged with
/// `vm_map`. Several fields are temporary (text, data stuff).
///
/// Locks used to protect struct members: `K` kernel lock, `I` immutable after creation, `a`
/// atomic operations, `v` `vm_map`'s lock.
///
/// `#[repr(C)]` so that `vm_map` is at offset 0, as the C relies on (`(struct vmspace *)map`).
#[repr(C)]
pub struct Vmspace {
    /// `vm_map`: VM address map.
    pub vm_map: VmMap,
    /// \[a\] `vm_refcnt`: number of references.
    pub vm_refcnt: AtomicI32,
    // vm_shm: SYSVSHM is not configured.
    // We copy from vm_startcopy (= vm_rssize) to the end of the structure on fork.
    /// `vm_rssize`: current resident set size in pages.
    pub vm_rssize: Cell<Segsz>,
    /// `vm_swrss`: resident set size before last swap.
    pub vm_swrss: Cell<Segsz>,
    /// `vm_tsize`: text size (pages) XXX.
    pub vm_tsize: Cell<Segsz>,
    /// `vm_dsize`: data size (pages) XXX.
    pub vm_dsize: Cell<Segsz>,
    /// `vm_dused`: data segment length (pages) XXX.
    pub vm_dused: Cell<Segsz>,
    /// \[v\] `vm_ssize`: stack size (pages).
    pub vm_ssize: Cell<Segsz>,
    /// \[I\] `vm_taddr`: user virtual address of text.
    pub vm_taddr: Cell<usize>,
    /// \[I\] `vm_daddr`: user virtual address of data.
    pub vm_daddr: Cell<usize>,
    /// \[I\] `vm_maxsaddr`: user VA at max stack growth.
    pub vm_maxsaddr: Cell<usize>,
    /// \[I\] `vm_minsaddr`: user VA at top of stack.
    pub vm_minsaddr: Cell<usize>,
}

impl Vmspace {
    /// A zero vmspace, before `uvmspace_init` (what a `static struct vmspace` holds).
    pub const fn new() -> Self {
        Self {
            vm_map: VmMap::new(),
            vm_refcnt: AtomicI32::new(0),
            vm_rssize: Cell::new(0),
            vm_swrss: Cell::new(0),
            vm_tsize: Cell::new(0),
            vm_dsize: Cell::new(0),
            vm_dused: Cell::new(0),
            vm_ssize: Cell::new(0),
            vm_taddr: Cell::new(0),
            vm_daddr: Cell::new(0),
            vm_maxsaddr: Cell::new(0),
            vm_minsaddr: Cell::new(0),
        }
    }

    /// `memcpy(&vm2->vm_startcopy, &vm1->vm_startcopy, ...)`: copies the statistics and
    /// boundaries from `other` (fork).
    pub fn copy_startcopy_from(&self, other: &Vmspace) {
        self.vm_rssize.set(other.vm_rssize.get());
        self.vm_swrss.set(other.vm_swrss.get());
        self.vm_tsize.set(other.vm_tsize.get());
        self.vm_dsize.set(other.vm_dsize.get());
        self.vm_dused.set(other.vm_dused.get());
        self.vm_ssize.set(other.vm_ssize.get());
        self.vm_taddr.set(other.vm_taddr.get());
        self.vm_daddr.set(other.vm_daddr.get());
        self.vm_maxsaddr.set(other.vm_maxsaddr.get());
        self.vm_minsaddr.set(other.vm_minsaddr.get());
    }

    /// `memset(&vm->vm_startcopy, 0, ...)`: nukes the statistics and boundaries (exec).
    pub fn clear_startcopy(&self) {
        self.copy_startcopy_from(&Vmspace::new());
    }
}

impl Default for Vmspace {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: a vmspace is shared between the threads of its process; the fields are `Cell`s and
// atomics whose protection the field docs name (the map's lock, the kernel lock).
unsafe impl Sync for Vmspace {}

/// `exec_map`: the submap of `kernel_map` that holds `execve`'s argument buffers (16
/// `NCARGS`, which effectively limits the number of processes exec'ing at any time); null
/// until the machine's `cpu_startup` made it.
pub static EXEC_MAP: AtomicPtr<crate::uvm::uvm_map::VmMap> = AtomicPtr::new(ptr::null_mut());

/// `exec_map`, once `cpu_startup` made it.
pub fn exec_map() -> &'static crate::uvm::uvm_map::VmMap {
    let map = EXEC_MAP.load(Ordering::Acquire);
    // SAFETY: a non-null pointer is the submap `uvm_km_suballoc` returned, never freed.
    match unsafe { map.as_ref() } {
        Some(map) => map,
        None => crate::kern::subr_prf::panic(format_args!("exec_map used before cpu_startup")),
    }
}

/// `phys_map`: the submap of `kernel_map` that `vmapbuf` maps physio's user buffers into
/// (`VM_PHYS_SIZE`); null until the machine's `cpu_startup` made it.
pub static PHYS_MAP: AtomicPtr<crate::uvm::uvm_map::VmMap> = AtomicPtr::new(ptr::null_mut());

/// `phys_map`, once `cpu_startup` made it.
pub fn phys_map() -> &'static crate::uvm::uvm_map::VmMap {
    let map = PHYS_MAP.load(Ordering::Acquire);
    // SAFETY: a non-null pointer is the submap `uvm_km_suballoc` returned, never freed.
    match unsafe { map.as_ref() } {
        Some(map) => map,
        None => crate::kern::subr_prf::panic(format_args!("phys_map used before cpu_startup")),
    }
}
/* </CODE> */
