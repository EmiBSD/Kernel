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
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
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
 *	@(#)vm_kern.c   8.3 (Berkeley) 1/12/94
 * from: Id: uvm_km.c,v 1.1.2.14 1998/02/06 05:19:27 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
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
//! Kernel memory allocation: `km_alloc(9)` and `km_free(9)` of `uvm/uvm_km.c`, with the
//! allocation modes of `<uvm/uvm_extern.h>`.
//!
//! Upstream: sys/uvm/uvm_km.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_extern.h @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_km.rs@3c62ede68802
//! LZ: sys/uvm/uvm_extern.rs@3c62ede68802
//!
//! A request names three modes: where the virtual space comes from ([`KmemVaMode`]: which
//! kernel map, its alignment, whether to wait for room, the single-page allocator), what backs
//! it ([`KmemPaMode`]: no pages, pageable memory of the kernel object, or wired pages within a
//! physical range, alignment, boundary and segment count, zeroed or not) and how the call may
//! behave ([`KmemDynMode`]: sleep or not, a `PMAP_PREFER` offset, try-lock the map). With a
//! direct map (`__HAVE_PMAP_DIRECT`, both machines) a single page or a single physically
//! contiguous segment is returned through the direct map and touches no map at all; anything
//! else takes virtual space from the map and enters each page.
//!
//! What is still `bsd`'s is reached through one bridge (bridges.toml), registered once by
//! `bsd`'s `uvm_km_init` ([`install`]): the kernel maps ([`KmMaps`]: `uvm_map`/`uvm_unmap` of
//! `kernel_map`, `kmem_map`, `exec_map`, `phys_map`, N3.5). The pages are the kernel's own
//! (`super::page`: `uvm_pglistalloc`, `uvm_pglistfree`, `uvm_pagefree`, since N3.3). The kernel pmap
//! changes only through `hal::pmap::KernelMapper`, the token `pmap_bootstrap` hands out once:
//! it is kept here ([`adopt_mapper`]), the owner of the fact it stands for (the caller owns the
//! kernel virtual addresses it maps), and lent to `machine::pmap` alone, the holder of the one
//! [`LegacyLend`], until the last caller of its kernel-pmap wrappers moves ([`legacy_mapper`],
//! N3.3 to N7).
//!
//! No lock of its own, as the C with `__HAVE_PMAP_DIRECT`: the map's lock is inside
//! [`KmMaps`], the free pages' inside the page system; the callers raise the interrupt level
//! around `kv_intrsafe` (`pool(9)`). A `kv_wait` mode sleeps on the map's address
//! (`"km_allocva"`, `PVM`) until a [`km_free`] of a `kv_wait` mode wakes it.
//!
//! ## Deviations
//! - On a machine without an MMU (`PMAP_NOMMU`, only the host test double) every page-backed
//!   request is served through the direct map, physically contiguous, and `kp_nomem` or
//!   `kp_pageable` requests fail (`None`), reported once as LZ did (`unported: km_alloc:
//!   kp_nomem/kp_pageable on a machine without an MMU`, through `printf(9)`).
//! - `km_alloc`'s single-page allocator (`uvm_km_pages` and its thread, `!__HAVE_PMAP_DIRECT`)
//!   is not compiled: both machines have a direct map (arm64 by LZ's deviation: OpenBSD's
//!   arm64 has none), so a `kv_singlepage` request that reaches the map path panics as the C
//!   does with a direct map (`km_alloc: DIRECT single page`); `uvm_init` keeps its two
//!   `unported:` lines (`uvm_km_page_init`, `uvm_km_page_lateinit`).
//! - `kd_slowdown` is a value, not a pointer: only the single-page thread writes it.
//! - [`kernel_map_min`] is the first address the kernel map can hand out after the bootstrap
//!   reservation (the selftests probe it), not `vm_map_min(kernel_map)`.
//!
//! ## Redesign
//! - LZ's `uvm_km.rs` is the allocator, the kernel map itself and `uvm_km_init`'s map and
//!   object set-up, all in `bsd`. The allocator moves here (N3.2): `km_alloc`, `km_free`,
//!   `uvm_km_pgremove_intrsafe` and the modes; the map (`kernel_map_store`, `uvm_km_init`,
//!   `uvm_km_suballoc`, `uvm_km_pgremove`) stays in `bsd`'s `sys/uvm/uvm_km.rs` with `uvm_map`
//!   and the aobj, reached through [`KmMaps`]. A result is a [`Vaddr`], never dereferenced
//!   here; `bsd`'s adapter turns it into the legacy `NonNull<u8>` for its callers.
//! - A mode's `kv_map` is a [`KvMap`] name, not a pointer to a map pointer; `kp_object` is a
//!   `bool` that means `&uvm.kernel_object` ([`KmObject::Kernel`] at the bridge).
//! - N3.3: the page system is the kernel's (`super::page`), so the `KmPages` bridge is gone and
//!   the C's shape is back: `km_alloc` takes the page list first, then the virtual space
//!   (sleeping for it under `kv_wait`, the pages held), then enters each page, or gives the
//!   whole list back (`uvm_pglistfree`) when there is no space; `km_free` collects the pages
//!   of the range on one list, removes the mappings and frees the list. N3.2's per-chunk
//!   frees and its frame-at-a-time callback (two deviations) are gone with it.
//! - The page provider of `hal::kmbuf::KmBuf` ([`page_memory`]) and the module loader's memory
//!   ([`module_memory`]) are implemented here and passed down as values; the loader's
//!   `release` is the C's `km_free` sequence for a writable alias: `pmap_kremove` of the range
//!   it entered, then `km_free(kp_none)`.
//! - `pmap_update` is nothing on both machines (every pmap call does its own TLB work), so it
//!   is not called.
//! - N3.5 S1: the C's four map pointers (`kernel_map`, `kmem_map`, `exec_map`, `phys_map`,
//!   assigned once each during boot) are `OnceStatic<&'static VmMap>` slots here, set by
//!   [`install_submap`] and read by [`kernel_map`], [`kmem_map`], [`exec_map`], [`phys_map`]
//!   and [`map_of`] (`KvMap` names the slot); `map == kernel_map` is [`is_kernel_map`], a
//!   pointer compare. [`uvm_km_pgremove`] moved here from `bsd`'s `uvm_km.rs` with the kernel
//!   object (the kernel's since N3.4e); its write witness is the C's `KASSERT(rw_write_held)`.
//!   Nothing calls them in a kernel image until the flip (S2): `bsd`'s copies and the
//!   [`KmMaps`] bridge stay live until then.
//! - N3.5 S1: the windows of `uvm_io` and `sys_kbind` are minted only from the range
//!   `uvm_map_extract` returned: [`uvm_map_extract_window`] wraps that range in an
//!   [`Extracted`] (no other constructor), which [`foreign_window`] consumes into a
//!   [`KernelWindow`]; the window is dropped before the range, and the range's drop unmaps it
//!   from `kernel_map` (`uvm_unmap_remove`, `uvm_unmap_detach(AMAP_REFALL)`). The token's
//!   contract ("a window only over your own extract's range, unmapped only after the window
//!   is gone") is that type. [`dma_memory`] is `KmBuf`'s provider of DMA-reachable wired
//!   pages (`uvm_vslock_device`'s bounce).

use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::bridges::AlreadyRegistered;
use hal::kmbuf::{PageKind, PageMemory};
use hal::module::ModuleMemory;
use hal::once::OnceStatic;
use hal::pmap::{HAVE_PMAP_DIRECT, KernelMapper, PMAP_NOMMU, PMAP_WIRED};
use hal::rwlock::{Witness, WriteWitness};
use hal::sleep::{INFSLP, PVM, tsleep_nsec, wakeup};
use hal::sys::errno::Errno;
use hal::sys::mman::{MADV_RANDOM, MAP_INHERIT_NONE, PROT_READ, PROT_WRITE};
use hal::sys::types::{Paddr, Vaddr, Vsize};
use hal::user::ForeignWindow;
use hal::vm::param::round_page;
use hal::vm::uvmexp::UVMEXP;

use super::amap::AMAP_REFALL;
use super::aobj::uao_dropswap;
use super::map::{DeadQueue, VmMap, uvm_map_extract, uvm_unmap_detach, uvm_unmap_remove};
use super::object::ObjRef;
use super::page::{
    Owner, PG_BUSY, PG_WANTED, PHYS_TO_VM_PAGE, Pglist, uvm_pagefree, uvm_pagelookup,
    uvm_pglistalloc, uvm_pglistfree,
};
use super::types::{
    UVM_FLAG_TRYLOCK, UVM_PLA_NOWAIT, UVM_PLA_TRYCONTIG, UVM_PLA_WAITOK, UVM_PLA_ZERO,
    UVM_UNKNOWN_OFFSET, UvmConstraintRange, VmProt, Voff, uvm_mapflag,
};
use crate::assert::{kassert, kpanic};
use crate::prf::unported;

/// Which kernel map a [`KmemVaMode`] allocates from (`kv_map` in C is a pointer to the map
/// pointer; the maps are `bsd`'s, named here, reached through [`KmMaps`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KvMap {
    /// `kernel_map`.
    Kernel,
    /// `kmem_map`.
    Kmem,
    /// `exec_map` (`kv_exec`, `kern_exec.c`).
    Exec,
    /// `phys_map` (`kv_physwait`, the machine's `vm_machdep.c`).
    Phys,
    /// No map: the single page allocator (`kv_singlepage`).
    None,
}

/// `struct kmem_va_mode`: allocation mode for virtual space.
#[derive(Clone, Copy)]
pub struct KmemVaMode {
    /// The map we're allocating from.
    pub kv_map: KvMap,
    /// Alignment.
    pub kv_align: usize,
    /// Wait for free space in the map if it's full. The default allocators don't wait since
    /// running out of space in kernel_map and kmem_map is usually fatal. Special maps like
    /// exec_map are specifically limited, so waiting for space in them is necessary.
    pub kv_wait: bool,
    /// Use the single page allocator.
    pub kv_singlepage: bool,
}

/// `struct kmem_pa_mode`: allocation mode for physical pages.
#[derive(Clone, Copy)]
pub struct KmemPaMode {
    /// Allocation constraint for physical pages.
    pub kp_constraint: &'static UvmConstraintRange,
    /// If the pages should be allocated from an object: the kernel object (`kp_pageable`).
    pub kp_object: bool,
    /// Physical alignment of the first page in the allocation.
    pub kp_align: Paddr,
    /// Boundary that the physical addresses can't cross if the allocation is contiguous.
    pub kp_boundary: Paddr,
    /// Maximal amount of contiguous segments.
    pub kp_maxseg: i32,
    /// Don't allocate any backing pages.
    pub kp_nomem: bool,
    /// Zero the returned memory.
    pub kp_zero: bool,
    /// Allocate pageable memory.
    pub kp_pageable: bool,
}

/// `struct kmem_dyn_mode`: dynamic allocation parameters. Stuff that changes too often or too
/// much to create separate va and pa modes for.
#[derive(Clone, Copy)]
pub struct KmemDynMode {
    /// Offset to feed to `PMAP_PREFER`.
    pub kd_prefer: Voff,
    /// Special parameter for the singlepage va allocator that tells the caller to sleep if
    /// possible to let the singlepage allocator catch up (`kd_slowdown` in C is a pointer the
    /// allocator writes through).
    pub kd_slowdown: bool,
    /// Is it ok to sleep?
    pub kd_waitok: bool,
    /// Don't sleep on map locks.
    pub kd_trylock: bool,
}

/// `KMEM_DYN_INITIALIZER`.
pub const KMEM_DYN_INITIALIZER: KmemDynMode = KmemDynMode {
    kd_prefer: UVM_UNKNOWN_OFFSET,
    kd_slowdown: false,
    kd_waitok: false,
    kd_trylock: false,
};

/// Which object a map request carries: none, or the kernel object (`uvm.kernel_object`,
/// `kp_object`), whose offsets are the map's (`va - vm_map_min(kernel_map)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KmObject {
    /// No object (`NULL`).
    None,
    /// `uvm.kernel_object`, for pageable kernel memory.
    Kernel,
}

/// `no_constraint`: unconstrained range.
pub static NO_CONSTRAINT: UvmConstraintRange = UvmConstraintRange {
    ucr_low: Paddr::new(0),
    ucr_high: Paddr::new(usize::MAX),
};

/// `kv_any`: any kernel virtual address, from `kernel_map`.
pub static KV_ANY: KmemVaMode = KmemVaMode {
    kv_map: KvMap::Kernel,
    kv_align: 0,
    kv_wait: false,
    kv_singlepage: false,
};

/// `kv_intrsafe`: from `kmem_map`, usable from interrupt handlers.
pub static KV_INTRSAFE: KmemVaMode = KmemVaMode {
    kv_map: KvMap::Kmem,
    kv_align: 0,
    kv_wait: false,
    kv_singlepage: false,
};

/// `kv_page`: the single page allocator.
pub static KV_PAGE: KmemVaMode = KmemVaMode {
    kv_map: KvMap::None,
    kv_align: 0,
    kv_wait: false,
    kv_singlepage: true,
};

/// A physical-page mode within `constraint`, every other field off (the C's designated
/// initialisers leave them zero): the base of the `kp_*` modes, here and in `bsd`'s
/// `kp_dma*`.
pub const fn pa_mode(constraint: &'static UvmConstraintRange) -> KmemPaMode {
    KmemPaMode {
        kp_constraint: constraint,
        kp_object: false,
        kp_align: Paddr::new(0),
        kp_boundary: Paddr::new(0),
        kp_maxseg: 0,
        kp_nomem: false,
        kp_zero: false,
        kp_pageable: false,
    }
}

/// `kp_dirty`: any physical pages, not zeroed.
pub static KP_DIRTY: KmemPaMode = pa_mode(&NO_CONSTRAINT);

/// `kp_zero`: any physical pages, zeroed.
pub static KP_ZERO: KmemPaMode = KmemPaMode {
    kp_zero: true,
    ..pa_mode(&NO_CONSTRAINT)
};

/// `kp_mbuf_contig`: one physically contiguous segment.
pub static KP_MBUF_CONTIG: KmemPaMode = KmemPaMode {
    kp_maxseg: 1,
    ..pa_mode(&NO_CONSTRAINT)
};

/// `kp_pageable`: pageable memory from the kernel object. XXX - kp_nomem, maybe, but we'll need
/// to fix km_free.
pub static KP_PAGEABLE: KmemPaMode = KmemPaMode {
    kp_object: true,
    kp_pageable: true,
    ..pa_mode(&NO_CONSTRAINT)
};

/// `kp_none`: virtual space only, no backing pages.
pub static KP_NONE: KmemPaMode = KmemPaMode {
    kp_nomem: true,
    ..pa_mode(&NO_CONSTRAINT)
};

/// `kd_waitok`: may sleep.
pub static KD_WAITOK: KmemDynMode = KmemDynMode {
    kd_waitok: true,
    ..KMEM_DYN_INITIALIZER
};

/// `kd_nowait`: must not sleep.
pub static KD_NOWAIT: KmemDynMode = KMEM_DYN_INITIALIZER;

/// `kd_trylock`: must not sleep on map locks.
pub static KD_TRYLOCK: KmemDynMode = KmemDynMode {
    kd_trylock: true,
    ..KMEM_DYN_INITIALIZER
};

/// The kernel map's token, adopted once from `pmap_bootstrap` ([`adopt_mapper`]).
static MAPPER: OnceStatic<KernelMapper> = OnceStatic::new();

/// The maps and the page system, registered once by `bsd`'s `uvm_km_init` ([`install`]).
static MAPS: OnceStatic<&'static dyn KmMaps> = OnceStatic::new();

/// The first free kernel virtual address after `uvm_km_init`'s reservation (see the module's
/// deviations).
static KERNEL_MAP_MIN: AtomicUsize = AtomicUsize::new(0);
/// `vm_map_max(kernel_map)`, kept for the selftests.
static KERNEL_MAP_MAX: AtomicUsize = AtomicUsize::new(0);

/// `kernel_map`: set once by `uvm_km_init` ([`install_submap`]).
static KERNEL_MAP: OnceStatic<&'static VmMap> = OnceStatic::new();
/// `kmem_map`: set once by `kmeminit`'s `uvm_km_suballoc` ([`install_submap`]).
static KMEM_MAP: OnceStatic<&'static VmMap> = OnceStatic::new();
/// `exec_map`: set once by the machine's `cpu_startup`.
static EXEC_MAP: OnceStatic<&'static VmMap> = OnceStatic::new();
/// `phys_map`: set once by the machine's `cpu_startup`.
static PHYS_MAP: OnceStatic<&'static VmMap> = OnceStatic::new();

/// The kernel maps, still `bsd`'s (`uvm_map`, N3.5): a bridge (bridges.toml), no logic.
///
/// What it answers is trusted as the kernel map's allocation record (a documented boundary
/// until `kernel::vm::map` owns the record, N3.5): a range [`KmMaps::map`] returns is free
/// kernel virtual space nobody else maps until [`KmMaps::unmap`] gives it back, which is the
/// fact the kernel map's token stands for when [`km_alloc`] enters pages there.
pub trait KmMaps: Sync {
    /// `uvm_map(*kv_map, &va, sz, uobj, prefer, align, mapflags)` with `va` starting at
    /// `vm_map_min(map)`: the address of `sz` bytes of new space in map `which`, carrying
    /// `object` (`uvm.kernel_object` at offset `UVM_UNKNOWN_OFFSET`, which the map turns into
    /// `va - vm_map_min(kernel_map)`), or why there was none (`ENOMEM`, or the try-lock's
    /// failure under `UVM_FLAG_TRYLOCK`).
    fn map(
        &self,
        which: KvMap,
        sz: usize,
        object: KmObject,
        prefer: Voff,
        align: usize,
        mapflags: u32,
    ) -> Result<Vaddr, Errno>;

    /// `uvm_unmap(*kv_map, sva, eva)`: gives `[sva, eva)` back to map `which` (the map's own
    /// teardown removes what it entered: the kernel object's pages, an intrsafe map's pages).
    fn unmap(&self, which: KvMap, sva: Vaddr, eva: Vaddr);

    /// The map's address, the C's wait channel for `"km_allocva"` (compared, never
    /// dereferenced).
    fn chan(&self, which: KvMap) -> *const ();
}

/// A range of `kernel_map` that `uvm_map_extract` filled with another map's pages
/// ([`uvm_map_extract_window`] makes the only ones; no other constructor): owned, so it is
/// given back once, by its drop (`uvm_unmap_remove(kernel_map, va, va + len, &dead, FALSE,
/// TRUE, FALSE)` under the map's write lock, then `uvm_unmap_detach(&dead, AMAP_REFALL)`,
/// as `uvm_io` and `sys_kbind` do).
pub struct Extracted {
    va: Vaddr,
    len: usize,
}

impl Extracted {
    /// The range's first kernel address.
    pub fn va(&self) -> Vaddr {
        self.va
    }

    /// The range's length.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the range is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Drop for Extracted {
    fn drop(&mut self) {
        let kmap = kernel_map();
        let mut dead = DeadQueue::new();
        {
            let mut g = kmap.lock();
            let va = self.va.as_usize();
            // uvm_unmap_remove cannot fail on kernel_map (no immutable check): the C ignores
            // its result too. XXX
            let _ = uvm_unmap_remove(&mut g, va, va + self.len, &mut dead, false, true, false);
        }
        uvm_unmap_detach(&mut dead, AMAP_REFALL);
    }
}

/// An extracted range and the one window that copies through it ([`foreign_window`]). The
/// window is dropped before the range (the fields' order), so the range is unmapped only
/// once nothing can copy through it.
pub struct KernelWindow {
    window: ForeignWindow<'static>,
    range: Extracted,
}

impl KernelWindow {
    /// The window: `read_at`/`write_at` within the extracted range.
    pub fn window(&self) -> &ForeignWindow<'static> {
        &self.window
    }

    /// The range it copies through.
    pub fn range(&self) -> &Extracted {
        &self.range
    }
}

/// The one lend of the kernel map's token to `bsd`: [`adopt_mapper`] makes it, once, and the
/// holder (`machine::pmap`, in a private static) shows it to [`legacy_mapper`]. Not `Clone`,
/// not `Copy`, no public constructor: code without it cannot reach the token.
pub struct LegacyLend {
    _private: (),
}

/// Registers the kernel maps, once (`bsd`'s `uvm_km_init`, before its first map entry is
/// allocated; the host tests' set-up). A second call keeps the first and names the bridge.
pub fn install(maps: &'static dyn KmMaps) -> Result<(), AlreadyRegistered> {
    MAPS.set(maps).map_err(|_| AlreadyRegistered("KmMaps"))
}

/// The registered maps; `None` before [`install`].
fn maps() -> Option<&'static dyn KmMaps> {
    MAPS.get().copied()
}

/// The registered maps for a call that needs them; panics (with `who`) before [`install`],
/// which only a broken caller can hit: there is nothing to free before anything was allocated.
fn maps_for(who: &str) -> &'static dyn KmMaps {
    match maps() {
        Some(m) => m,
        None => kpanic(format_args!("{who}: before uvm_km_init")),
    }
}

/// `uvm_pglistfree` of the pages behind `frames`, panicking with `who`'s message (the C's) at
/// the first frame no page describes, before any page goes back.
fn free_frames(who: &str, frames: impl Iterator<Item = Paddr>) {
    let pgl = Pglist::new();
    for pa in frames {
        match PHYS_TO_VM_PAGE(pa) {
            Some(pg) => pgl.insert_tail(pg),
            None => kpanic(format_args!("{who}: unmanaged page {:#x}", pa.as_usize())),
        }
    }
    uvm_pglistfree(&pgl);
}

/// Adopts the kernel map's token from the boot glue's `pmap_bootstrap` (once, on the boot
/// CPU, before anything maps) and returns the one [`LegacyLend`] for the caller to keep; a
/// second token is handed back, and no second lend exists.
pub fn adopt_mapper(m: KernelMapper) -> Result<LegacyLend, KernelMapper> {
    MAPPER.set(m)?;
    Ok(LegacyLend { _private: () })
}

/// The kernel map's token (also the pager's, for its window: `vm::pager::uvm_pagermapin`);
/// panics before [`adopt_mapper`]. On the host, where nothing boots,
/// whoever reaches a path that maps (the map path, `km_free`'s wired path,
/// [`uvm_km_pgremove_intrsafe`], `module_memory`'s release, the fault handler's
/// `pmap_enter` on `kernel_map`, `kernel::vm::fault`) has bootstrapped the pmap first: the
/// kernel's tests in their set-up, `bsd`'s in `uvm_km_test_init`.
///
/// The holders keep the token's contract (`hal::mapped`): no live pager window is overlaid.
/// The fault handler enters pages only at addresses of an entry with an amap or an object, and
/// a pager window lies in space `km_alloc` reserved with neither (`kv_any`, `kp_none`: the
/// pager segments), so a fault on a window's address is case 0 and fails with `EFAULT` before
/// any `pmap_enter`.
pub(super) fn mapper() -> &'static KernelMapper {
    match MAPPER.get() {
        Some(m) => m,
        None => kpanic(format_args!(
            "pmap: the kernel pmap used before pmap_bootstrap"
        )),
    }
}

/// The token, lent to the holder of the [`LegacyLend`]: `machine::pmap`, for its wrappers
/// over the kernel pmap. Those are the three `unsafe fn`s (`pmap_kenter_pa`, `pmap_kremove`,
/// arm64's `pmap_kenter_cache`), whose callers keep their `unsafe` blocks as the proof the
/// token stands for, and the safe `pmap_enter`, `pmap_remove` and `pmap_protect` on
/// `pmap_kernel()` and amd64's `pmap_prealloc_lowmem_ptps`, whose callers are the kernel map's
/// own paths (`uvm_map`, `uvm_fault`, `uvm_glue`, the boot glue). A documented trust boundary
/// that shrinks as they move (N3.3 `uvm_page`, N3.5 `uvm_glue` and `uvm_map`, N4 the
/// machines' `cpu` and `vm_machdep`, N7 `bus_space`, `rnd`, `vfs_biomem`) and goes with the
/// last one. Panics before [`adopt_mapper`].
pub fn legacy_mapper(_lend: &LegacyLend) -> &'static KernelMapper {
    mapper()
}

/// `uvm_km_init`'s bounds of the kernel map: the first address past its bootstrap reservation
/// and its end (see the module's deviations).
pub fn set_bounds(start: Vaddr, end: Vaddr) {
    KERNEL_MAP_MIN.store(start.as_usize(), Ordering::Relaxed);
    KERNEL_MAP_MAX.store(end.as_usize(), Ordering::Relaxed);
}

/// The first kernel virtual address past the bootstrap reservation of `uvm_km_init` (see the
/// module's deviations).
pub fn kernel_map_min() -> Vaddr {
    Vaddr::new(KERNEL_MAP_MIN.load(Ordering::Relaxed))
}

/// `vm_map_max(kernel_map)`: the end of kernel virtual space.
pub fn kernel_map_max() -> Vaddr {
    Vaddr::new(KERNEL_MAP_MAX.load(Ordering::Relaxed))
}

/// The slot of the kernel map `which` names; `None` for the single page mode, which has none.
fn slot_of(which: KvMap) -> Option<&'static OnceStatic<&'static VmMap>> {
    match which {
        KvMap::Kernel => Some(&KERNEL_MAP),
        KvMap::Kmem => Some(&KMEM_MAP),
        KvMap::Exec => Some(&EXEC_MAP),
        KvMap::Phys => Some(&PHYS_MAP),
        KvMap::None => None,
    }
}

/// Sets the kernel map `which` names, once: `kernel_map` (`uvm_km_init`), `kmem_map`
/// (`kmeminit`'s `uvm_km_suballoc`), `exec_map` and `phys_map` (the machine's
/// `cpu_startup`). The C assigns a global pointer; a second set keeps the first and names the
/// map, and `KvMap::None` (the single page mode) has no map to set: both panic, as only a
/// broken boot path can reach them.
pub fn install_submap(which: KvMap, map: &'static VmMap) {
    let Some(slot) = slot_of(which) else {
        kpanic(format_args!(
            "install_submap: the single page mode has no map"
        ));
    };
    if slot.set(map).is_err() {
        kpanic(format_args!("install_submap: {which:?} set twice"));
    }
}

/// The kernel map `which` names, if it is set yet.
pub fn map_of(which: KvMap) -> Option<&'static VmMap> {
    slot_of(which).and_then(|slot| slot.get().copied())
}

/// The kernel map `which` names, for a caller that needs it; panics (with `who`) before it
/// is set, which only a broken boot order can hit.
fn map_for(who: &str, which: KvMap) -> &'static VmMap {
    match map_of(which) {
        Some(map) => map,
        None => kpanic(format_args!(
            "{who}: {which:?} map used before it is set up"
        )),
    }
}

/// `kernel_map`: the kernel's map; panics before `uvm_km_init`.
pub fn kernel_map() -> &'static VmMap {
    map_for("kernel_map", KvMap::Kernel)
}

/// `kmem_map`: the submap `malloc(9)` allocates from; panics before `kmeminit`.
pub fn kmem_map() -> &'static VmMap {
    map_for("kmem_map", KvMap::Kmem)
}

/// `exec_map`: the submap `exec` copies its arguments through; panics before `cpu_startup`.
pub fn exec_map() -> &'static VmMap {
    map_for("exec_map", KvMap::Exec)
}

/// `phys_map`: the submap `vmapbuf` maps physio buffers in; panics before `cpu_startup`.
pub fn phys_map() -> &'static VmMap {
    map_for("phys_map", KvMap::Phys)
}

/// `map == kernel_map`: whether `map` is the kernel map (false before `uvm_km_init`, as the
/// C's null `kernel_map` compares).
pub fn is_kernel_map(map: &VmMap) -> bool {
    map_of(KvMap::Kernel).is_some_and(|k| core::ptr::eq(k, map))
}

/// `uvm_map_extract(src, start, len, &kva, flags)` as a value: the pages of `[start, start +
/// len)` of `src`, mapped in `kernel_map` at the address the extract chose, wrapped in the
/// only window that may copy through them ([`KernelWindow`]); its drop unmaps them. The one
/// way to an [`Extracted`]: the range is the extract's result, never a caller's number.
pub fn uvm_map_extract_window(
    src: &VmMap,
    start: usize,
    len: usize,
    flags: i32,
) -> Result<KernelWindow, Errno> {
    let kva = uvm_map_extract(src, start, len, flags)?;
    Ok(foreign_window(Extracted {
        va: Vaddr::new(kva),
        len,
    }))
}

/// The window onto an extracted range, minted by the kernel map's token: the window
/// consumes the range, so it is the range's only window, and the range is unmapped only
/// after the window is gone (the token's contract, `hal::pmap::KernelMapper::foreign_window`).
pub fn foreign_window(range: Extracted) -> KernelWindow {
    let window = mapper().foreign_window(range.va, range.len);
    KernelWindow { window, range }
}

/// `uvm_km_pgremove`: remove pages from a kernel uvm_object.
///
/// - when you unmap a part of anonymous kernel memory you want to toss the pages right
///   away. (this gets called from uvm_unmap_...).
///
/// `w` is the write witness of `uobj`'s lock (the C's `KASSERT(rw_write_held(
/// uobj->vmobjlock))` as a parameter: a witness of another lock panics); a busy page is
/// waited for asleep on it with that lock released and taken again (`uvm_pagewait` then
/// `rw_enter(RW_WRITE)`), `"km_pgrm"`, `PVM`.
pub fn uvm_km_pgremove(uobj: &ObjRef, w: &mut WriteWitness<'_>, startva: Vaddr, endva: Vaddr) {
    let base = kernel_map().min_offset();
    let start = (startva.as_usize() - base) as Voff;
    let end = (endva.as_usize() - base) as Voff;
    let mut swpgonlydelta = 0;

    let Some(aobj) = uobj.aobj() else {
        kpanic(format_args!("uvm_km_pgremove: not an aobj"));
    };
    if w.lock_id() != uobj.lock().id() {
        kpanic(format_args!("uvm_km_pgremove: a witness of another lock"));
    }

    mapper().remove(startva, endva);
    let mut curoff = start;
    while curoff < end {
        let pp = uvm_pagelookup(&**uobj, curoff);
        if let Some(pg) = pp.filter(|pg| pg.flags() & PG_BUSY != 0) {
            pg.set_bits(PG_WANTED);
            let _ = w.sleep_nsec(pg.chan(), PVM, "km_pgrm", INFSLP);
            continue; // loop back to us
        }

        // free the swap slot, then the page
        let slot = uao_dropswap(aobj, w, (curoff >> PAGE_SHIFT) as i32);

        match pp {
            Some(pg) => uvm_pagefree(pg, Owner::Object(&**uobj)),
            None if slot != 0 => swpgonlydelta += 1,
            None => {}
        }
        curoff += PAGE_SIZE as Voff;
    }

    if swpgonlydelta > 0 {
        kassert!(UVMEXP.swpgonly.load(Ordering::Relaxed) >= swpgonlydelta);
        UVMEXP.swpgonly.fetch_sub(swpgonlydelta, Ordering::Relaxed);
    }
}

/// `uvm_km_pgremove_intrsafe`: like `uvm_km_pgremove()`, but for "intrsafe" objects.
///
/// - when you unmap a part of anonymous kernel memory you want to toss the pages right
///   away. (this gets called from uvm_unmap_...).
/// - none of the pages will ever be busy, and none of them will ever be on the active or
///   inactive queues (because these objects are never allowed to "page").
pub fn uvm_km_pgremove_intrsafe(start: Vaddr, end: Vaddr) {
    let mut va = start.as_usize();
    while va < end.as_usize() {
        if let Some(pa) = hal::pmap::kernel().extract(Vaddr::new(va)) {
            match PHYS_TO_VM_PAGE(pa) {
                Some(pg) => uvm_pagefree(pg, Owner::None),
                None => kpanic(format_args!("uvm_km_pgremove_intrsafe: no page")),
            }
        }
        va += PAGE_SIZE;
    }
    mapper().kremove(start, Vsize::new(end.as_usize() - start.as_usize()));
}

/// `km_alloc`: allocate `sz` bytes of kernel memory with the given virtual, physical and
/// dynamic modes; its kernel virtual address, `None` when there is none (or before
/// [`install`]).
pub fn km_alloc(sz: usize, kv: &KmemVaMode, kp: &KmemPaMode, kd: &KmemDynMode) -> Option<Vaddr> {
    alloc_in(PMAP_NOMMU, sz, kv, kp, kd)
}

/// [`km_alloc`] on a machine with (`nommu` false) or without an MMU.
fn alloc_in(
    nommu: bool,
    sz: usize,
    kv: &KmemVaMode,
    kp: &KmemPaMode,
    kd: &KmemDynMode,
) -> Option<Vaddr> {
    kassert!(sz == round_page(sz));
    let maps = maps()?;

    if kp.kp_nomem || kp.kp_pageable {
        if nommu {
            // Kernel virtual space without memory, or pageable memory, means nothing without
            // an MMU (the host double; module deviations).
            let _ = unported!("km_alloc: kp_nomem/kp_pageable on a machine without an MMU");
            return None;
        }
        return map_va(maps, sz, kv, kp, kd).ok();
    }

    let mut pla_flags = if kd.kd_waitok {
        UVM_PLA_WAITOK
    } else {
        UVM_PLA_NOWAIT
    };
    pla_flags |= UVM_PLA_TRYCONTIG;
    if kp.kp_zero {
        pla_flags |= UVM_PLA_ZERO;
    }
    let mut pla_align = kp.kp_align.as_usize();
    if HAVE_PMAP_DIRECT && pla_align < kv.kv_align {
        pla_align = kv.kv_align;
    }
    let mut pla_maxseg = kp.kp_maxseg;
    if pla_maxseg == 0 {
        pla_maxseg = (sz / PAGE_SIZE) as i32;
    }
    if nommu {
        // No MMU (the host double): the direct map must serve, so one segment.
        pla_maxseg = 1;
    }

    let pgl = Pglist::new();
    uvm_pglistalloc(
        sz,
        kp.kp_constraint.ucr_low,
        kp.kp_constraint.ucr_high,
        Paddr::new(pla_align),
        kp.kp_boundary,
        &pgl,
        pla_maxseg,
        pla_flags,
    )
    .ok()?;

    // __HAVE_PMAP_DIRECT: only use direct mappings for single page or single segment
    // allocations.
    if HAVE_PMAP_DIRECT && (kv.kv_singlepage || kp.kp_maxseg == 1 || nommu) {
        let first = pgl.first().map(|pg| hal::pmap::direct_map(pg.phys_addr()));
        // The pages are the caller's now: off this function's list.
        while pgl.pop_head().is_some() {}
        return first;
    }

    let Ok(sva) = map_va(maps, sz, kv, kp, kd) else {
        uvm_pglistfree(&pgl);
        return None;
    };
    let prot: VmProt = PROT_READ | PROT_WRITE;
    let mut va = sva;
    while let Some(pg) = pgl.pop_head() {
        let pa = pg.phys_addr();
        if kp.kp_pageable {
            // Never reached: pageable memory takes no pages here (it is faulted in from the
            // kernel object), as in the C.
            let _ = mapper().enter(va, pa, prot, prot | PMAP_WIRED);
        } else {
            mapper().kenter(va, pa, prot);
        }
        va = Vaddr::new(va.as_usize() + PAGE_SIZE);
    }
    Some(sva)
}

/// `km_alloc`'s `alloc_va`: virtual space of `sz` bytes in the mode's map, sleeping for it on
/// the map under `kv_wait` with `kd_waitok`.
fn map_va(
    maps: &'static dyn KmMaps,
    sz: usize,
    kv: &KmemVaMode,
    kp: &KmemPaMode,
    kd: &KmemDynMode,
) -> Result<Vaddr, Errno> {
    let prot: VmProt = PROT_READ | PROT_WRITE;

    if kp.kp_pageable {
        kassert!(kp.kp_object);
        kassert!(!kv.kv_singlepage);
    } else {
        kassert!(!kp.kp_object);
    }

    if kv.kv_singlepage {
        kassert!(sz == PAGE_SIZE);
        // __HAVE_PMAP_DIRECT on both machines: the single-page allocator is not compiled
        // (module deviations).
        kpanic(format_args!("km_alloc: DIRECT single page"));
    }

    let mapflags = if kd.kd_trylock { UVM_FLAG_TRYLOCK } else { 0 };
    let object = if kp.kp_object {
        KmObject::Kernel
    } else {
        KmObject::None
    };
    loop {
        // try_map:
        match maps.map(
            kv.kv_map,
            sz,
            object,
            kd.kd_prefer,
            kv.kv_align,
            uvm_mapflag(prot, prot, MAP_INHERIT_NONE, MADV_RANDOM, mapflags),
        ) {
            Ok(va) => return Ok(va),
            Err(e) => {
                if !(kv.kv_wait && kd.kd_waitok) {
                    return Err(e);
                }
                let _ = tsleep_nsec(maps.chan(kv.kv_map), PVM, "km_allocva", INFSLP);
            }
        }
    }
}

/// `km_free`: returns what [`km_alloc`] gave, with the same modes.
pub fn km_free(va: Vaddr, sz: usize, kv: &KmemVaMode, kp: &KmemPaMode) {
    free_in(PMAP_NOMMU, va, sz, kv, kp);
}

/// [`km_free`] on a machine with (`nommu` false) or without an MMU.
fn free_in(nommu: bool, va: Vaddr, sz: usize, kv: &KmemVaMode, kp: &KmemPaMode) {
    let maps = maps_for("km_free");
    let sva = va.as_usize();
    let eva = sva + sz;

    if !kp.kp_nomem {
        // __HAVE_PMAP_DIRECT
        if HAVE_PMAP_DIRECT && (kv.kv_singlepage || kp.kp_maxseg == 1 || nommu) {
            let frames = (sva..eva).step_by(PAGE_SIZE).map(|va| {
                match hal::pmap::direct_unmap(Vaddr::new(va)) {
                    Some(pa) => pa,
                    None => kpanic(format_args!("km_free: unmanaged page at {va:#x}")),
                }
            });
            free_frames("km_free", frames);
            return;
        }

        if kp.kp_pageable {
            mapper().remove(Vaddr::new(sva), Vaddr::new(eva));
        } else {
            // The C's order: collect the pages, remove the mappings, then free the list (no
            // page is on a free list while it is still mapped).
            let pgl = Pglist::new();
            for va in (sva..eva).step_by(PAGE_SIZE) {
                if let Some(pa) = hal::pmap::kernel().extract(Vaddr::new(va)) {
                    match PHYS_TO_VM_PAGE(pa) {
                        Some(pg) => pgl.insert_tail(pg),
                        None => {
                            kpanic(format_args!("km_free: unmanaged page {:#x}", pa.as_usize()))
                        }
                    }
                }
            }
            mapper().kremove(Vaddr::new(sva), Vsize::new(sz));
            uvm_pglistfree(&pgl);
        }
    }
    // free_va:
    maps.unmap(kv.kv_map, Vaddr::new(sva), Vaddr::new(eva));
    if kv.kv_wait {
        wakeup(maps.chan(kv.kv_map));
    }
}

/// The C's physical-page mode of a [`KmBuf`](hal::kmbuf::KmBuf)'s kind.
fn kp_of(kind: PageKind) -> &'static KmemPaMode {
    match kind {
        PageKind::Pageable => &KP_PAGEABLE,
        PageKind::Wired => &KP_ZERO,
    }
}

/// `hal::kmbuf::KmBuf`'s pages: `km_alloc`/`km_free` with `&kv_any`.
struct KmPageMemory;

// The provider's contract (`hal::kmbuf::PageMemory`): a block `km_alloc` returns is fresh
// kernel memory of `len` bytes, zeroed (`kp_zero`, or `kp_pageable`'s zero-fill faults),
// page-aligned, which nothing else references until `free` hands it to `km_free` with the
// same modes.
impl PageMemory for KmPageMemory {
    fn alloc(&self, len: usize, kind: PageKind, wait: bool) -> Option<(NonNull<u8>, usize)> {
        let kd = if wait { &KD_WAITOK } else { &KD_NOWAIT };
        let va = km_alloc(len, &KV_ANY, kp_of(kind), kd)?;
        let p = NonNull::new(core::ptr::with_exposed_provenance_mut::<u8>(va.as_usize()))?;
        Some((p, len))
    }

    fn free(&self, ptr: NonNull<u8>, len: usize, kind: PageKind) {
        km_free(Vaddr::new(ptr.as_ptr().addr()), len, &KV_ANY, kp_of(kind));
    }

    fn page_size(&self) -> usize {
        PAGE_SIZE
    }
}

static KM_PAGE_MEMORY: KmPageMemory = KmPageMemory;

/// `hal::kmbuf::KmBuf`'s wired pages within the machine's DMA range (`dma_constraint`, the
/// `kp_dma_zero` mode): `uvm_vslock_device`'s bounce buffer.
struct KmDmaMemory;

impl KmDmaMemory {
    /// `kp_dma_zero` over the machine's `dma_constraint` (`vm::init::machine()`).
    fn kp() -> KmemPaMode {
        KmemPaMode {
            kp_zero: true,
            ..pa_mode(super::init::machine().dma_constraint)
        }
    }
}

// The provider's contract (`hal::kmbuf::PageMemory`), as `KmPageMemory`'s: a block is fresh,
// zeroed, page-aligned kernel memory nothing else references until `free`. Only wired
// blocks: a pageable one has no physical range to keep to.
impl PageMemory for KmDmaMemory {
    fn alloc(&self, len: usize, kind: PageKind, wait: bool) -> Option<(NonNull<u8>, usize)> {
        if kind != PageKind::Wired {
            return None;
        }
        let kd = if wait { &KD_WAITOK } else { &KD_NOWAIT };
        let va = km_alloc(len, &KV_ANY, &Self::kp(), kd)?;
        let p = NonNull::new(core::ptr::with_exposed_provenance_mut::<u8>(va.as_usize()))?;
        Some((p, len))
    }

    fn free(&self, ptr: NonNull<u8>, len: usize, _kind: PageKind) {
        km_free(Vaddr::new(ptr.as_ptr().addr()), len, &KV_ANY, &Self::kp());
    }

    fn page_size(&self) -> usize {
        PAGE_SIZE
    }
}

static KM_DMA_MEMORY: KmDmaMemory = KmDmaMemory;

/// The kernel map's DMA-reachable wired pages as `KmBuf` takes them (`uvm_vslock_device`'s
/// bounce), a provider passed as a value.
pub const fn dma_memory() -> &'static dyn PageMemory {
    &KM_DMA_MEMORY
}

/// The kernel map's pages as `hal::kmbuf::KmBuf` takes them: `km_alloc(&kv_any, &kp_pageable
/// or &kp_zero, &kd_waitok or &kd_nowait)`, a provider passed as a value.
pub const fn page_memory() -> &'static dyn PageMemory {
    &KM_PAGE_MEMORY
}

/// The module loader's writable copy (`hal::module::load`): `km_alloc(&kv_any, &kp_zero,
/// &kd_waitok)`, released as the C's `km_free` sequence for a writable alias.
struct KmModuleMemory;

// The loader's premise (`hal::module::ModuleMemory`): `alloc` is fresh, zeroed, wired kernel
// memory mapped writable at the address returned; `release` removes those writable mappings
// and gives back the virtual space only (`kp_none`), so the pages stay, held by the module
// window's own mappings.
impl ModuleMemory for KmModuleMemory {
    fn alloc(&self, len: usize) -> Option<Vaddr> {
        km_alloc(round_page(len), &KV_ANY, &KP_ZERO, &KD_WAITOK)
    }

    fn release(&self, va: Vaddr, len: usize) {
        let len = round_page(len);
        mapper().kremove(va, Vsize::new(len));
        km_free(va, len, &KV_ANY, &KP_NONE);
    }
}

static KM_MODULE_MEMORY: KmModuleMemory = KmModuleMemory;

/// The kernel memory the module loader takes, a provider passed to `hal::module::load` as a
/// value.
pub const fn module_memory() -> &'static dyn ModuleMemory {
    &KM_MODULE_MEMORY
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Over hal's host pmap (no MMU: `PMAP_NOMMU`, the direct map the identity, the kernel
    // pmap a `BTreeMap`), the kernel's own page system over a heap block
    // (`testutil::pages_install`, which holds `testutil::MEMORY`) and the double of the maps
    // bridge (`crate::testutil`). The map path, which the host never takes, is exercised
    // through `alloc_in`/`free_in` with `nommu` false: the double's virtual space, the host
    // pmap's mappings.

    use std::sync::MutexGuard;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::testutil;

    use hal::kmbuf::KmBuf;
    use hal::vm::uvmexp::UVMEXP;

    /// 16 MiB of pages for each test.
    const BYTES: usize = 16 << 20;

    /// The maps double, the host pmap (once per process) and fresh pages, `MEMORY` held.
    fn setup() -> MutexGuard<'static, ()> {
        testutil::km_install();
        testutil::pages_install(BYTES)
    }

    /// The free pages, those in this CPU's page cache (`MULTIPROCESSOR`) included.
    fn avail() -> i64 {
        i64::from(UVMEXP.free.load(Ordering::Relaxed))
            + i64::from(UVMEXP.percpucaches.load(Ordering::Relaxed))
    }

    fn extract(va: usize) -> Option<Paddr> {
        hal::pmap::kernel().extract(Vaddr::new(va))
    }

    #[test]
    fn the_modes_are_the_cs() {
        assert_eq!(NO_CONSTRAINT.ucr_low, Paddr::new(0));
        assert_eq!(NO_CONSTRAINT.ucr_high, Paddr::new(usize::MAX));
        assert_eq!(KV_ANY.kv_map, KvMap::Kernel);
        assert!(!KV_ANY.kv_wait && !KV_ANY.kv_singlepage && KV_ANY.kv_align == 0);
        assert_eq!(KV_INTRSAFE.kv_map, KvMap::Kmem);
        assert!(!KV_INTRSAFE.kv_singlepage);
        assert_eq!(KV_PAGE.kv_map, KvMap::None);
        assert!(KV_PAGE.kv_singlepage);
        for (name, kp, object, maxseg, nomem, zero, pageable) in [
            ("kp_dirty", &KP_DIRTY, false, 0, false, false, false),
            ("kp_zero", &KP_ZERO, false, 0, false, true, false),
            (
                "kp_mbuf_contig",
                &KP_MBUF_CONTIG,
                false,
                1,
                false,
                false,
                false,
            ),
            ("kp_pageable", &KP_PAGEABLE, true, 0, false, false, true),
            ("kp_none", &KP_NONE, false, 0, true, false, false),
        ] {
            assert!(core::ptr::eq(kp.kp_constraint, &NO_CONSTRAINT), "{name}");
            assert_eq!(kp.kp_object, object, "{name}");
            assert_eq!(kp.kp_maxseg, maxseg, "{name}");
            assert_eq!(kp.kp_nomem, nomem, "{name}");
            assert_eq!(kp.kp_zero, zero, "{name}");
            assert_eq!(kp.kp_pageable, pageable, "{name}");
            assert_eq!(kp.kp_align, Paddr::new(0), "{name}");
            assert_eq!(kp.kp_boundary, Paddr::new(0), "{name}");
        }
        for (name, kd, waitok, trylock) in [
            ("kd_waitok", &KD_WAITOK, true, false),
            ("kd_nowait", &KD_NOWAIT, false, false),
            ("kd_trylock", &KD_TRYLOCK, false, true),
        ] {
            assert_eq!(kd.kd_prefer, UVM_UNKNOWN_OFFSET, "{name}");
            assert!(!kd.kd_slowdown, "{name}");
            assert_eq!(kd.kd_waitok, waitok, "{name}");
            assert_eq!(kd.kd_trylock, trylock, "{name}");
        }
    }

    #[test]
    fn without_an_mmu_the_direct_map_serves_and_gives_the_frames_back() {
        let _g = setup();
        let before = avail();
        let va = km_alloc(2 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("two pages");
        assert_eq!(va.as_usize() % PAGE_SIZE, 0);
        assert_eq!(avail(), before - 2);
        assert_eq!(
            testutil::km_entry(va),
            None,
            "the direct map takes no space"
        );
        // The pages are the page system's, theirs in the direct map, off the free lists.
        let pg = PHYS_TO_VM_PAGE(Paddr::new(va.as_usize())).expect("a managed page");
        assert!(!pg.is_free());
        let free = UVMEXP.free.load(Ordering::Relaxed);
        km_free(va, 2 * PAGE_SIZE, &KV_ANY, &KP_ZERO);
        assert_eq!(avail(), before);
        assert_eq!(
            UVMEXP.free.load(Ordering::Relaxed),
            free + 2,
            "back on the free list (uvm_pglistfree), not uvm_pagefree's per-CPU cache"
        );
    }

    #[test]
    fn km_free_removes_the_mappings_then_frees_the_list() {
        let _g = setup();
        let pages = 40;
        let len = pages * PAGE_SIZE;
        let before = avail();
        let va = alloc_in(false, len, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("space");
        assert_eq!(avail(), before - pages as i64);
        let frames: std::vec::Vec<Paddr> = (0..pages)
            .map(|i| extract(va.as_usize() + i * PAGE_SIZE).expect("entered"))
            .collect();
        free_in(false, va, len, &KV_ANY, &KP_ZERO);
        assert_eq!(avail(), before);
        assert!((0..pages).all(|i| extract(va.as_usize() + i * PAGE_SIZE).is_none()));
        assert!(
            frames
                .iter()
                .all(|&pa| PHYS_TO_VM_PAGE(pa).is_some_and(|pg| pg.is_free()))
        );
    }

    /// A map slot is set once and read back; a second set and the single page mode's
    /// (which names no map) panic. `phys_map` only: no other test sets it.
    #[test]
    fn a_submap_is_installed_once() {
        assert!(map_of(KvMap::None).is_none());
        let m: &'static VmMap =
            std::boxed::Box::leak(std::boxed::Box::new(VmMap::new(0x1000_0000, 0x2000_0000)));
        install_submap(KvMap::Phys, m);
        assert!(core::ptr::eq(phys_map(), m));
        assert!(map_of(KvMap::Phys).is_some_and(|p| core::ptr::eq(p, m)));
        assert!(!is_kernel_map(m));
        assert_eq!(phys_map().min_offset(), 0x1000_0000);
        assert_eq!(phys_map().max_offset(), 0x2000_0000);
        assert!(std::panic::catch_unwind(|| install_submap(KvMap::Phys, m)).is_err());
        assert!(std::panic::catch_unwind(|| install_submap(KvMap::None, m)).is_err());
    }

    #[test]
    fn the_lend_reaches_the_token() {
        let _g = setup();
        let lend = testutil::pmap_boot();
        let va = Vaddr::new(testutil::KM_VA_END - PAGE_SIZE);
        let pa = Paddr::new(0x4321_0000);
        legacy_mapper(lend).kenter(va, pa, PROT_READ);
        assert_eq!(extract(va.as_usize()), Some(pa));
        legacy_mapper(lend).kremove(va, Vsize::new(PAGE_SIZE));
        assert_eq!(extract(va.as_usize()), None);
    }

    #[test]
    fn without_an_mmu_no_memory_and_pageable_are_refused() {
        let _g = setup();
        assert_eq!(km_alloc(PAGE_SIZE, &KV_ANY, &KP_NONE, &KD_NOWAIT), None);
        assert_eq!(km_alloc(PAGE_SIZE, &KV_ANY, &KP_PAGEABLE, &KD_WAITOK), None);
    }

    #[test]
    fn the_map_path_enters_each_page_and_km_free_removes_them() {
        let _g = setup();
        let before = avail();
        let va = alloc_in(false, 3 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("space");
        assert!(va.as_usize() >= testutil::KM_VA_BASE);
        assert_eq!(
            testutil::km_entry(va),
            Some((KvMap::Kernel, KmObject::None))
        );
        assert_eq!(avail(), before - 3);
        let frames: std::vec::Vec<Paddr> = (0..3)
            .map(|i| extract(va.as_usize() + i * PAGE_SIZE).expect("entered"))
            .collect();
        assert!(frames.windows(2).all(|w| w[0] != w[1]));
        free_in(false, va, 3 * PAGE_SIZE, &KV_ANY, &KP_ZERO);
        assert_eq!(avail(), before);
        assert!((0..3).all(|i| extract(va.as_usize() + i * PAGE_SIZE).is_none()));
        assert_eq!(testutil::km_entry(va), None);
    }

    #[test]
    fn no_space_gives_every_frame_back() {
        let _g = setup();
        let before = avail();
        testutil::km_fail_maps(1);
        assert_eq!(
            alloc_in(false, 2 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT),
            None
        );
        assert_eq!(avail(), before);
    }

    #[test]
    fn virtual_space_only_and_pageable_memory_take_no_frames() {
        let _g = setup();
        let before = avail();
        let va = alloc_in(false, PAGE_SIZE, &KV_ANY, &KP_NONE, &KD_NOWAIT).expect("space");
        assert_eq!(
            testutil::km_entry(va),
            Some((KvMap::Kernel, KmObject::None))
        );
        assert_eq!(extract(va.as_usize()), None);
        free_in(false, va, PAGE_SIZE, &KV_ANY, &KP_NONE);
        assert_eq!(testutil::km_entry(va), None);

        let va = alloc_in(false, 2 * PAGE_SIZE, &KV_ANY, &KP_PAGEABLE, &KD_WAITOK).expect("space");
        assert_eq!(
            testutil::km_entry(va),
            Some((KvMap::Kernel, KmObject::Kernel))
        );
        free_in(false, va, 2 * PAGE_SIZE, &KV_ANY, &KP_PAGEABLE);
        assert_eq!(testutil::km_entry(va), None);
        assert_eq!(avail(), before);
    }

    #[test]
    fn a_waiting_mode_sleeps_on_the_map_until_a_wakeup() {
        let _g = setup();
        static KV_WAITS: KmemVaMode = KmemVaMode {
            kv_map: KvMap::Exec,
            kv_align: 0,
            kv_wait: true,
            kv_singlepage: false,
        };
        static DONE: AtomicBool = AtomicBool::new(false);
        let maps = testutil::km_maps();
        let chan = maps.chan(KvMap::Exec).addr();
        // The waker: a km_free of a kv_wait mode elsewhere, as often as needed (a wakeup
        // before the sleep is set up wakes nobody, as in the C).
        let waker = thread::spawn(move || {
            while !DONE.load(Ordering::Acquire) {
                wakeup(core::ptr::without_provenance::<()>(chan));
                thread::sleep(Duration::from_millis(5));
            }
        });
        let calls = testutil::km_map_calls();
        testutil::km_fail_maps(1);
        let va = alloc_in(false, PAGE_SIZE, &KV_WAITS, &KP_NONE, &KD_WAITOK);
        DONE.store(true, Ordering::Release);
        waker.join().unwrap();
        let va = va.expect("space after the wakeup");
        assert_eq!(
            testutil::km_map_calls(),
            calls + 2,
            "failed once, then retried"
        );
        assert_eq!(testutil::km_entry(va), Some((KvMap::Exec, KmObject::None)));
        free_in(false, va, PAGE_SIZE, &KV_WAITS, &KP_NONE);
        assert_eq!(
            testutil::km_last_unmap(),
            Some((KvMap::Exec, va.as_usize(), va.as_usize() + PAGE_SIZE))
        );

        // Without kd_waitok the same failure returns at once.
        testutil::km_fail_maps(1);
        assert_eq!(
            alloc_in(false, PAGE_SIZE, &KV_WAITS, &KP_NONE, &KD_NOWAIT),
            None
        );
    }

    #[test]
    fn pgremove_intrsafe_frees_what_is_mapped() {
        let _g = setup();
        let before = avail();
        let va = alloc_in(false, 2 * PAGE_SIZE, &KV_INTRSAFE, &KP_ZERO, &KD_NOWAIT).expect("space");
        assert_eq!(avail(), before - 2);
        let end = Vaddr::new(va.as_usize() + 2 * PAGE_SIZE);
        uvm_km_pgremove_intrsafe(va, end);
        assert_eq!(avail(), before);
        assert_eq!(extract(va.as_usize()), None);
        assert_eq!(extract(va.as_usize() + PAGE_SIZE), None);
        // The space itself goes back with the map entry (uvm_unmap's part).
        free_in(false, va, 2 * PAGE_SIZE, &KV_INTRSAFE, &KP_NONE);
        assert_eq!(testutil::km_entry(va), None);
    }

    #[test]
    fn a_second_install_is_refused() {
        testutil::km_install();
        assert_eq!(
            install(testutil::km_maps()),
            Err(AlreadyRegistered("KmMaps"))
        );
    }

    #[test]
    fn the_bounds_are_kept() {
        set_bounds(Vaddr::new(0x1000_0000), Vaddr::new(0x2000_0000));
        assert_eq!(kernel_map_min(), Vaddr::new(0x1000_0000));
        assert_eq!(kernel_map_max(), Vaddr::new(0x2000_0000));
    }

    #[test]
    fn page_memory_backs_a_kmbuf_and_takes_it_back() {
        let _g = setup();
        let before = avail();
        let mut b = KmBuf::alloc_wired(page_memory(), 2 * PAGE_SIZE, false).expect("wired");
        assert_eq!(avail(), before - 2);
        assert!(b.iter().all(|&x| x == 0));
        b[0] = 1;
        b[2 * PAGE_SIZE - 1] = 2;
        drop(b);
        assert_eq!(avail(), before);
        // Pageable memory needs an MMU (module deviations).
        assert!(KmBuf::alloc_pageable(page_memory(), PAGE_SIZE).is_none());
        assert_eq!(page_memory().page_size(), PAGE_SIZE);
    }

    #[test]
    fn module_memory_keeps_its_pages_when_released() {
        let _g = setup();
        let before = avail();
        let mem = module_memory();
        let va = mem.alloc(PAGE_SIZE + 1).expect("two pages");
        assert_eq!(avail(), before - 2);
        mem.release(va, PAGE_SIZE + 1);
        assert_eq!(
            testutil::km_last_unmap(),
            Some((KvMap::Kernel, va.as_usize(), va.as_usize() + 2 * PAGE_SIZE))
        );
        // kp_none: the pages stay, the module window's mappings hold them.
        assert_eq!(avail(), before - 2);
    }
}
/* </TESTS> */
