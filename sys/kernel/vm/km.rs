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
//! What is still `bsd`'s is reached through two bridges (bridges.toml), registered once by
//! `bsd`'s `uvm_km_init` ([`install`]): the kernel maps ([`KmMaps`]: `uvm_map`/`uvm_unmap` of
//! `kernel_map`, `kmem_map`, `exec_map`, `phys_map`, N3.5) and the page system ([`KmPages`]:
//! `uvm_pglistalloc`, `uvm_pglistfree` and `uvm_pagefree`, N3.3), whose boundary is the C's
//! own: the physical address of a frame (`VM_PAGE_TO_PHYS`/`PHYS_TO_VM_PAGE`). The kernel pmap
//! changes only through `hal::pmap::KernelMapper`, the token `pmap_bootstrap` hands out once:
//! it is kept here ([`adopt_mapper`]), the owner of the fact it stands for (the caller owns the
//! kernel virtual addresses it maps), and lent to `machine::pmap` alone, the holder of the one
//! [`LegacyLend`], until the last caller of its kernel-pmap wrappers moves ([`legacy_mapper`],
//! N3.3 to N7).
//!
//! No lock of its own, as the C with `__HAVE_PMAP_DIRECT`: the map's lock is inside
//! [`KmMaps`], the page queues' inside [`KmPages`]; the callers raise the interrupt level
//! around `kv_intrsafe` (`pool(9)`). A `kv_wait` mode sleeps on the map's address
//! (`"km_allocva"`, `PVM`) until a [`km_free`] of a `kv_wait` mode wakes it.
//!
//! ## Deviations
//! - On a machine without an MMU (`PMAP_NOMMU`, only the host test double) every page-backed
//!   request is served through the direct map, physically contiguous, and `kp_nomem` or
//!   `kp_pageable` requests fail (`None`); LZ reported the latter with `unported!`, which the
//!   kernel crate does not have: the host's output is the only place it showed.
//! - `km_alloc`'s single-page allocator (`uvm_km_pages` and its thread, `!__HAVE_PMAP_DIRECT`)
//!   is not compiled: both machines have a direct map (arm64 by LZ's deviation: OpenBSD's
//!   arm64 has none), so a `kv_singlepage` request that reaches the map path panics as the C
//!   does with a direct map (`km_alloc: DIRECT single page`); `uvm_init` keeps its two
//!   `unported:` lines (`uvm_km_page_init`, `uvm_km_page_lateinit`).
//! - `kd_slowdown` is a value, not a pointer: only the single-page thread writes it.
//! - `km_free` collects the frames of at most [`FREE_CHUNK`] pages at a time (the kernel holds
//!   no list and no heap here), so a wired range larger than that has its mappings removed
//!   and its frames freed chunk by chunk, one `pmap_kremove` (one TLB shootdown) per chunk
//!   where the C has one per call; within a chunk the C's order holds (the mappings go, then
//!   the frames go back), so no frame is on a free list while it is still mapped. A frame no
//!   page describes panics once its chunk is unmapped, where the C panics before the
//!   `pmap_kremove`. Each frame goes back by itself ([`KmPages::free_list`]: one free-page
//!   lock and one wakeup per page, where the C's `uvm_pglistfree` takes one per list).
//! - `km_alloc` gives back the frames of a request that found no virtual space one at a time
//!   as the page list hands them over ([`KmPages::alloc`]'s callback), each with the C's
//!   `uvm_pglistfree` ([`KmPages::free_list`]): the same counter and wakeups, in more calls.
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
//! - The C allocates the pages first, then the virtual space, then enters each page. The page
//!   list is `bsd`'s, so [`KmPages::alloc`] hands the frames over one at a time, in order; the
//!   first frame takes the virtual space (sleeping for it under `kv_wait`, the pages held, as
//!   the C), every frame is entered at the next page, and when there is no space each frame is
//!   given back. The order the C has is kept and the kernel holds no list.
//! - The page provider of `hal::kmbuf::KmBuf` ([`page_memory`]) and the module loader's memory
//!   ([`module_memory`]) are implemented here and passed down as values; the loader's
//!   `release` is the C's `km_free` sequence for a writable alias: `pmap_kremove` of the range
//!   it entered, then `km_free(kp_none)`.
//! - `pmap_update` is nothing on both machines (every pmap call does its own TLB work), so it
//!   is not called.

use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use hal::arch::current::param::PAGE_SIZE;
use hal::bridges::AlreadyRegistered;
use hal::kmbuf::{PageKind, PageMemory};
use hal::module::ModuleMemory;
use hal::once::OnceStatic;
use hal::pmap::{HAVE_PMAP_DIRECT, KernelMapper, PMAP_NOMMU, PMAP_WIRED};
use hal::sleep::{INFSLP, PVM, tsleep_nsec, wakeup};
use hal::sys::errno::Errno;
use hal::sys::mman::{MADV_RANDOM, MAP_INHERIT_NONE, PROT_READ, PROT_WRITE};
use hal::sys::types::{Paddr, Vaddr, Vsize};
use hal::vm::param::round_page;

use super::types::{
    UVM_FLAG_TRYLOCK, UVM_PLA_NOWAIT, UVM_PLA_TRYCONTIG, UVM_PLA_WAITOK, UVM_PLA_ZERO,
    UVM_UNKNOWN_OFFSET, UvmConstraintRange, VmProt, Voff, uvm_mapflag,
};
use crate::assert::{kassert, kpanic};

/// The most frames `km_free` collects before giving them back: `FREE_CHUNK` physical
/// addresses on the stack, 256 bytes (the kernel stacks of the I/O paths are tight).
pub const FREE_CHUNK: usize = 32;

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

/// The two bridges, registered together by [`install`].
#[derive(Clone, Copy)]
struct Bridges {
    maps: &'static dyn KmMaps,
    pages: &'static dyn KmPages,
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
static BRIDGES: OnceStatic<Bridges> = OnceStatic::new();

/// The first free kernel virtual address after `uvm_km_init`'s reservation (see the module's
/// deviations).
static KERNEL_MAP_MIN: AtomicUsize = AtomicUsize::new(0);
/// `vm_map_max(kernel_map)`, kept for the selftests.
static KERNEL_MAP_MAX: AtomicUsize = AtomicUsize::new(0);

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

/// The page system, still `bsd`'s (`uvm_page`, N3.3): a bridge (bridges.toml), no logic.
/// Frames cross as physical addresses, the C's own boundary (`VM_PAGE_TO_PHYS`,
/// `PHYS_TO_VM_PAGE`).
pub trait KmPages: Sync {
    /// `uvm_pglistalloc(sz, constraint->ucr_low, constraint->ucr_high, align, boundary, &pgl,
    /// maxseg, flags)`, then each frame of the list, in order, handed to `each` as it is taken
    /// off the list; the frames are the caller's from then on. Nothing is handed over when the
    /// allocation fails.
    #[allow(
        clippy::too_many_arguments,
        reason = "uvm_pglistalloc's signature, and the callback"
    )]
    fn alloc(
        &self,
        sz: usize,
        constraint: &UvmConstraintRange,
        align: Paddr,
        boundary: Paddr,
        maxseg: i32,
        flags: i32,
        each: &mut dyn FnMut(Paddr),
    ) -> Result<(), Errno>;

    /// `uvm_pglistfree` of the pages behind `frames` (`km_free`'s and `km_alloc`'s way back):
    /// every page onto the free list (`uvm_pmr_freepageq`: `uvmexp.free` counts them, and
    /// `&uvmexp.free` and `&uvmexp.zeropages` are woken), never into a per-CPU cache. When no
    /// page describes a frame, nothing is freed and that frame is the error (the caller
    /// panics with the C's message).
    fn free_list(&self, frames: &[Paddr]) -> Result<(), Paddr>;

    /// `uvm_pagefree(PHYS_TO_VM_PAGE(pa))` (`uvm_km_pgremove_intrsafe`'s way back: the page
    /// cleaned, then the per-CPU page cache under `__HAVE_UVM_PERCPU`); `false`, and nothing
    /// done, when no page describes `pa`.
    fn free(&self, pa: Paddr) -> bool;
}

/// The one lend of the kernel map's token to `bsd`: [`adopt_mapper`] makes it, once, and the
/// holder (`machine::pmap`, in a private static) shows it to [`legacy_mapper`]. Not `Clone`,
/// not `Copy`, no public constructor: code without it cannot reach the token.
pub struct LegacyLend {
    _private: (),
}

/// Registers the kernel maps and the page system, once (`bsd`'s `uvm_km_init`, before its
/// first map entry is allocated; the host tests' set-up). A second call keeps the first and
/// names the bridge.
pub fn install(
    maps: &'static dyn KmMaps,
    pages: &'static dyn KmPages,
) -> Result<(), AlreadyRegistered> {
    BRIDGES
        .set(Bridges { maps, pages })
        .map_err(|_| AlreadyRegistered("KmMaps"))
}

/// The registered bridges; `None` before [`install`].
fn bridges() -> Option<Bridges> {
    BRIDGES.get().copied()
}

/// The registered bridges for a call that needs them; panics (with `who`) before [`install`],
/// which only a broken caller can hit: there is nothing to free before anything was allocated.
fn bridges_for(who: &str) -> Bridges {
    match bridges() {
        Some(b) => b,
        None => kpanic(format_args!("{who}: before uvm_km_init")),
    }
}

/// Adopts the kernel map's token from the boot glue's `pmap_bootstrap` (once, on the boot
/// CPU, before anything maps) and returns the one [`LegacyLend`] for the caller to keep; a
/// second token is handed back, and no second lend exists.
pub fn adopt_mapper(m: KernelMapper) -> Result<LegacyLend, KernelMapper> {
    MAPPER.set(m)?;
    Ok(LegacyLend { _private: () })
}

/// The kernel map's token; panics before [`adopt_mapper`]. On the host, where nothing boots,
/// whoever reaches a path that maps (the map path, `km_free`'s wired path,
/// [`uvm_km_pgremove_intrsafe`], `module_memory`'s release) has bootstrapped the pmap first:
/// the kernel's tests in their set-up, `bsd`'s in `uvm_km_test_init`.
fn mapper() -> &'static KernelMapper {
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

/// `uvm_km_pgremove_intrsafe`: like `uvm_km_pgremove()`, but for "intrsafe" objects.
///
/// - when you unmap a part of anonymous kernel memory you want to toss the pages right
///   away. (this gets called from uvm_unmap_...).
/// - none of the pages will ever be busy, and none of them will ever be on the active or
///   inactive queues (because these objects are never allowed to "page").
pub fn uvm_km_pgremove_intrsafe(start: Vaddr, end: Vaddr) {
    let b = bridges_for("uvm_km_pgremove_intrsafe");
    let mut va = start.as_usize();
    while va < end.as_usize() {
        if let Some(pa) = hal::pmap::kernel().extract(Vaddr::new(va))
            && !b.pages.free(pa)
        {
            kpanic(format_args!("uvm_km_pgremove_intrsafe: no page"));
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

/// Where a run of frames goes when it takes the map path (`km_alloc` after `alloc_va`).
#[derive(Clone, Copy)]
enum Fill {
    /// No frame yet: the first one takes the virtual space.
    Start,
    /// The space starts at `sva`; the next frame goes at `next`.
    Mapped { sva: Vaddr, next: Vaddr },
    /// No space: every frame goes back.
    Failed,
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
    let b = bridges()?;

    if kp.kp_nomem || kp.kp_pageable {
        if nommu {
            // Kernel virtual space without memory, or pageable memory, means nothing without
            // an MMU (the host double; module deviations).
            return None;
        }
        return map_va(b, sz, kv, kp, kd).ok();
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

    // __HAVE_PMAP_DIRECT: only use direct mappings for single page or single segment
    // allocations.
    if HAVE_PMAP_DIRECT && (kv.kv_singlepage || kp.kp_maxseg == 1 || nommu) {
        let mut sva = None;
        b.pages
            .alloc(
                sz,
                kp.kp_constraint,
                Paddr::new(pla_align),
                kp.kp_boundary,
                pla_maxseg,
                pla_flags,
                &mut |pa| {
                    if sva.is_none() {
                        sva = Some(hal::pmap::direct_map(pa));
                    }
                },
            )
            .ok()?;
        return sva;
    }

    let prot: VmProt = PROT_READ | PROT_WRITE;
    let mut fill = Fill::Start;
    b.pages
        .alloc(
            sz,
            kp.kp_constraint,
            Paddr::new(pla_align),
            kp.kp_boundary,
            pla_maxseg,
            pla_flags,
            &mut |pa| {
                if let Fill::Start = fill {
                    fill = match map_va(b, sz, kv, kp, kd) {
                        Ok(va) => Fill::Mapped { sva: va, next: va },
                        Err(_) => Fill::Failed,
                    };
                }
                match fill {
                    Fill::Mapped { sva, next } => {
                        if kp.kp_pageable {
                            // Never reached: pageable memory takes no pages here (it is
                            // faulted in from the kernel object), as in the C.
                            let _ = mapper().enter(next, pa, prot, prot | PMAP_WIRED);
                        } else {
                            mapper().kenter(next, pa, prot);
                        }
                        fill = Fill::Mapped {
                            sva,
                            next: Vaddr::new(next.as_usize() + PAGE_SIZE),
                        };
                    }
                    Fill::Failed | Fill::Start => {
                        if let Err(pa) = b.pages.free_list(&[pa]) {
                            kpanic(format_args!(
                                "km_alloc: unmanaged page {:#x}",
                                pa.as_usize()
                            ));
                        }
                    }
                }
            },
        )
        .ok()?;
    match fill {
        Fill::Mapped { sva, .. } => Some(sva),
        Fill::Failed | Fill::Start => None,
    }
}

/// `km_alloc`'s `alloc_va`: virtual space of `sz` bytes in the mode's map, sleeping for it on
/// the map under `kv_wait` with `kd_waitok`.
fn map_va(
    b: Bridges,
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
        match b.maps.map(
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
                let _ = tsleep_nsec(b.maps.chan(kv.kv_map), PVM, "km_allocva", INFSLP);
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
    let b = bridges_for("km_free");
    let sva = va.as_usize();
    let eva = sva + sz;

    if !kp.kp_nomem {
        // __HAVE_PMAP_DIRECT
        let mut frames = [Paddr::new(0); FREE_CHUNK];
        if HAVE_PMAP_DIRECT && (kv.kv_singlepage || kp.kp_maxseg == 1 || nommu) {
            let mut va = sva;
            while va < eva {
                let mut n = 0;
                while va < eva && n < FREE_CHUNK {
                    match hal::pmap::direct_unmap(Vaddr::new(va)) {
                        Some(pa) => frames[n] = pa,
                        None => kpanic(format_args!("km_free: unmanaged page at {va:#x}")),
                    }
                    n += 1;
                    va += PAGE_SIZE;
                }
                if let Err(pa) = b.pages.free_list(&frames[..n]) {
                    let at = hal::pmap::direct_map(pa).as_usize();
                    kpanic(format_args!("km_free: unmanaged page at {at:#x}"));
                }
            }
            return;
        }

        if kp.kp_pageable {
            mapper().remove(Vaddr::new(sva), Vaddr::new(eva));
        } else {
            // The C's order, per chunk: collect the frames, remove the mappings, then free
            // the frames (module deviations).
            let mut va = sva;
            while va < eva {
                let start = va;
                let mut n = 0;
                while va < eva && va - start < FREE_CHUNK * PAGE_SIZE {
                    if let Some(pa) = hal::pmap::kernel().extract(Vaddr::new(va)) {
                        frames[n] = pa;
                        n += 1;
                    }
                    va += PAGE_SIZE;
                }
                mapper().kremove(Vaddr::new(start), Vsize::new(va - start));
                if let Err(pa) = b.pages.free_list(&frames[..n]) {
                    kpanic(format_args!("km_free: unmanaged page {:#x}", pa.as_usize()));
                }
            }
        }
    }
    // free_va:
    b.maps.unmap(kv.kv_map, Vaddr::new(sva), Vaddr::new(eva));
    if kv.kv_wait {
        wakeup(b.maps.chan(kv.kv_map));
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
    // pmap a `BTreeMap`) and the kernel's doubles of the two bridges (`crate::testutil`). The
    // map path, which the host never takes, is exercised through `alloc_in`/`free_in` with
    // `nommu` false: the double's virtual space, the host pmap's mappings.

    use std::sync::atomic::AtomicBool;
    use std::sync::{Once, OnceLock};
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::testutil;
    use hal::kmbuf::KmBuf;
    use hal::pmap::BootstrapParams;

    /// The lend `adopt_mapper` returned to the set-up.
    static LEND: OnceLock<LegacyLend> = OnceLock::new();

    /// The process's set-up, once: the runtime's doubles, the km doubles, the host pmap
    /// bootstrapped with the page provider double and its token adopted here (the lend kept
    /// for the test that borrows it).
    fn setup() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            testutil::install();
            testutil::km_install();
            let b = hal::pmap::bootstrap(
                &BootstrapParams {
                    direct_base: Vaddr::new(0),
                    direct_end: Vaddr::new(usize::MAX),
                    nx: false,
                    first_avail: Paddr::new(0),
                    image: None,
                },
                testutil::phys_pages(),
            );
            match adopt_mapper(b.mapper) {
                Ok(lend) => assert!(LEND.set(lend).is_ok()),
                Err(_) => panic!("the token was adopted already"),
            }
        });
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
        setup();
        let before = testutil::km_frames_out();
        let va = km_alloc(2 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("two pages");
        assert_eq!(va.as_usize() % PAGE_SIZE, 0);
        assert_eq!(testutil::km_frames_out(), before + 2);
        assert_eq!(
            testutil::km_entry(va),
            None,
            "the direct map takes no space"
        );
        let listed = testutil::km_list_freed();
        km_free(va, 2 * PAGE_SIZE, &KV_ANY, &KP_ZERO);
        assert_eq!(testutil::km_frames_out(), before);
        assert_eq!(
            testutil::km_list_freed(),
            listed + 2,
            "back on the free list (uvm_pglistfree), not uvm_pagefree's per-CPU cache"
        );
    }

    #[test]
    fn km_free_removes_the_mappings_before_it_frees_the_frames() {
        setup();
        // More pages than one chunk, so two kremoves and two lists.
        let pages = FREE_CHUNK + 8;
        let len = pages * PAGE_SIZE;
        let before = testutil::km_frames_out();
        let listed = testutil::km_list_freed();
        let va = alloc_in(false, len, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("space");
        assert_eq!(testutil::km_frames_out(), before + pages as i64);
        testutil::km_watch(va.as_usize(), va.as_usize() + len);
        let mapped_before = testutil::km_freed_mapped();
        free_in(false, va, len, &KV_ANY, &KP_ZERO);
        assert_eq!(
            testutil::km_freed_mapped(),
            mapped_before,
            "no frame freed while mapped"
        );
        assert_eq!(testutil::km_frames_out(), before);
        assert_eq!(testutil::km_list_freed(), listed + pages as i64);
        assert!((0..pages).all(|i| extract(va.as_usize() + i * PAGE_SIZE).is_none()));
    }

    #[test]
    fn the_lend_reaches_the_token() {
        setup();
        let lend = LEND.get().expect("the set-up's lend");
        let va = Vaddr::new(testutil::KM_VA_END - PAGE_SIZE);
        let pa = Paddr::new(0x4321_0000);
        legacy_mapper(lend).kenter(va, pa, PROT_READ);
        assert_eq!(extract(va.as_usize()), Some(pa));
        legacy_mapper(lend).kremove(va, Vsize::new(PAGE_SIZE));
        assert_eq!(extract(va.as_usize()), None);
    }

    #[test]
    fn without_an_mmu_no_memory_and_pageable_are_refused() {
        setup();
        assert_eq!(km_alloc(PAGE_SIZE, &KV_ANY, &KP_NONE, &KD_NOWAIT), None);
        assert_eq!(km_alloc(PAGE_SIZE, &KV_ANY, &KP_PAGEABLE, &KD_WAITOK), None);
    }

    #[test]
    fn the_map_path_enters_each_page_and_km_free_removes_them() {
        setup();
        let before = testutil::km_frames_out();
        let va = alloc_in(false, 3 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("space");
        assert!(va.as_usize() >= testutil::KM_VA_BASE);
        assert_eq!(
            testutil::km_entry(va),
            Some((KvMap::Kernel, KmObject::None))
        );
        assert_eq!(testutil::km_frames_out(), before + 3);
        let frames: std::vec::Vec<Paddr> = (0..3)
            .map(|i| extract(va.as_usize() + i * PAGE_SIZE).expect("entered"))
            .collect();
        assert!(frames.windows(2).all(|w| w[0] != w[1]));
        free_in(false, va, 3 * PAGE_SIZE, &KV_ANY, &KP_ZERO);
        assert_eq!(testutil::km_frames_out(), before);
        assert!((0..3).all(|i| extract(va.as_usize() + i * PAGE_SIZE).is_none()));
        assert_eq!(testutil::km_entry(va), None);
    }

    #[test]
    fn no_space_gives_every_frame_back() {
        setup();
        let before = testutil::km_frames_out();
        testutil::km_fail_maps(1);
        assert_eq!(
            alloc_in(false, 2 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT),
            None
        );
        assert_eq!(testutil::km_frames_out(), before);
    }

    #[test]
    fn virtual_space_only_and_pageable_memory_take_no_frames() {
        setup();
        let before = testutil::km_frames_out();
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
        assert_eq!(testutil::km_frames_out(), before);
    }

    #[test]
    fn a_waiting_mode_sleeps_on_the_map_until_a_wakeup() {
        setup();
        static KV_WAITS: KmemVaMode = KmemVaMode {
            kv_map: KvMap::Exec,
            kv_align: 0,
            kv_wait: true,
            kv_singlepage: false,
        };
        static DONE: AtomicBool = AtomicBool::new(false);
        let (maps, _) = testutil::km_doubles();
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
        setup();
        let before = testutil::km_frames_out();
        let va = alloc_in(false, 2 * PAGE_SIZE, &KV_INTRSAFE, &KP_ZERO, &KD_NOWAIT).expect("space");
        assert_eq!(testutil::km_frames_out(), before + 2);
        let end = Vaddr::new(va.as_usize() + 2 * PAGE_SIZE);
        uvm_km_pgremove_intrsafe(va, end);
        assert_eq!(testutil::km_frames_out(), before);
        assert_eq!(extract(va.as_usize()), None);
        assert_eq!(extract(va.as_usize() + PAGE_SIZE), None);
        // The space itself goes back with the map entry (uvm_unmap's part).
        free_in(false, va, 2 * PAGE_SIZE, &KV_INTRSAFE, &KP_NONE);
        assert_eq!(testutil::km_entry(va), None);
    }

    #[test]
    fn a_second_install_is_refused() {
        setup();
        let (maps, pages) = testutil::km_doubles();
        assert_eq!(install(maps, pages), Err(AlreadyRegistered("KmMaps")));
    }

    #[test]
    fn the_bounds_are_kept() {
        set_bounds(Vaddr::new(0x1000_0000), Vaddr::new(0x2000_0000));
        assert_eq!(kernel_map_min(), Vaddr::new(0x1000_0000));
        assert_eq!(kernel_map_max(), Vaddr::new(0x2000_0000));
    }

    #[test]
    fn page_memory_backs_a_kmbuf_and_takes_it_back() {
        setup();
        let before = testutil::km_frames_out();
        let mut b = KmBuf::alloc_wired(page_memory(), 2 * PAGE_SIZE, false).expect("wired");
        assert_eq!(testutil::km_frames_out(), before + 2);
        assert!(b.iter().all(|&x| x == 0));
        b[0] = 1;
        b[2 * PAGE_SIZE - 1] = 2;
        drop(b);
        assert_eq!(testutil::km_frames_out(), before);
        // Pageable memory needs an MMU (module deviations).
        assert!(KmBuf::alloc_pageable(page_memory(), PAGE_SIZE).is_none());
        assert_eq!(page_memory().page_size(), PAGE_SIZE);
    }

    #[test]
    fn module_memory_keeps_its_pages_when_released() {
        setup();
        let before = testutil::km_frames_out();
        let mem = module_memory();
        let va = mem.alloc(PAGE_SIZE + 1).expect("two pages");
        assert_eq!(testutil::km_frames_out(), before + 2);
        mem.release(va, PAGE_SIZE + 1);
        assert_eq!(
            testutil::km_last_unmap(),
            Some((KvMap::Kernel, va.as_usize(), va.as_usize() + 2 * PAGE_SIZE))
        );
        // kp_none: the pages stay, the module window's mappings hold them.
        assert_eq!(testutil::km_frames_out(), before + 2);
    }
}
/* </TESTS> */
