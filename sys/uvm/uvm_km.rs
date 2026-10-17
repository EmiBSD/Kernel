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
/* </LICENSES> */

/* <CODE> */
//! Kernel memory allocation and management: `uvm/uvm_km.c`, `bsd`'s half since N3.2.
//!
//! Upstream: sys/uvm/uvm_km.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_km.rs@3c62ede68802
//!
//! The allocator (`km_alloc`, `km_free`, `uvm_km_pgremove_intrsafe` and the modes) is the
//! kernel crate's, `kernel::vm::km`. What stays here is what belongs to the kernel map and the
//! kernel object, which are still `bsd`'s (`uvm_map`, `uvm_aobj`): `kernel_map` itself, its
//! set-up in [`uvm_km_init`], [`uvm_km_suballoc`] and [`uvm_km_pgremove`]; the bridge the
//! kernel's allocator reaches the maps through ([`BsdKmMaps`]; its pages are the kernel's own
//! page system since N3.3); `kp_dma*` (over
//! the machine's `dma_constraint`); and the legacy signatures every caller of `km_alloc` and
//! `km_free` keeps (`Option<NonNull<u8>>`).
//!
//! ## Deviations
//! - `kernel_map` is [`kernel_map()`] (a panic before `uvm_km_init`) and the C's
//!   `map == kernel_map` is [`is_kernel_map`]; `kmem_map` is `kern_malloc.rs`'s
//!   [`kmem_map()`](crate::kern::kern_malloc::kmem_map).
//! - The allocator's own deviations are `kernel::vm::km`'s (the host's `PMAP_NOMMU`, no
//!   single-page allocator, `km_free`'s order).
//!
//! ## Redesign
//! - N3.2 (docs/ZERO_UNSAFE.md 0.8): the allocator moved into the kernel crate; this module is
//!   its adapter and the kernel map's owner until N3.5, with no `unsafe` (LZ's 9: the kernel
//!   map's `AtomicPtr` is a `OnceStatic`, the kernel object comes through `Uvm`'s safe
//!   accessor, the page list is drained with `TailqHead::pop_head`, and the pmap calls are the
//!   kernel's, through its `KernelMapper` token).
//! - `kv_map_of` is [`BsdKmMaps::map_of`], the `KmMaps` bridge's dispatch from a `KvMap` to
//!   the map.
//! - [`uvm_km_init`] installs the maps bridge first: the kernel object and the reservation's
//!   map entries are allocated through `km_alloc` (`kv_page`) before the map is up.
//! - N3.3: `BsdKmPages` (the `KmPages` bridge over `uvm_pglistalloc`, `uvm_pmr_freepages` and
//!   `uvm_pagefree`) is gone with the bridge: the kernel's allocator calls the kernel's page
//!   system.

use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering;

use hal::once::OnceStatic;
pub use kernel::vm::km::{
    KD_NOWAIT, KD_TRYLOCK, KD_WAITOK, KP_DIRTY, KP_MBUF_CONTIG, KP_NONE, KP_PAGEABLE, KP_ZERO,
    KV_ANY, KV_INTRSAFE, KV_PAGE, NO_CONSTRAINT, kernel_map_max, kernel_map_min,
    uvm_km_pgremove_intrsafe,
};
use kernel::vm::km::{KmMaps, KmObject, KvMap, install, pa_mode, set_bounds};

use crate::kassert;
use crate::kern::kern_malloc::kmem_map;
use crate::kern::kern_rwlock::rw_enter_write;
use crate::kern::subr_prf::panic;
use crate::machine::pmap::{PmapRef, pmap_kernel, pmap_reference, pmap_remove};
use crate::machine::{Machine, Pmap};
use crate::sys::errno::Errno;
use crate::sys::mman::{MADV_RANDOM, MAP_INHERIT_NONE, PROT_READ, PROT_WRITE};
use crate::sys::param::{PAGE_SHIFT, PAGE_SIZE};
use crate::sys::rwlock::rw_write_held;
use crate::sys::types::{Vaddr, Vsize};
use crate::uvm::uvm_aobj::{UAO_FLAG_KERNOBJ, uao_create, uao_dropswap, uao_init};
use crate::uvm::uvm_extern::{
    KmemDynMode, KmemPaMode, KmemVaMode, UVM_FLAG_FIXED, UVM_FLAG_NOMERGE, UVM_UNKNOWN_OFFSET,
    Voff, uvm_mapflag,
};
use crate::uvm::uvm_init::{UVM, UVMEXP};
use crate::uvm::uvm_map::{
    VM_MAP_PAGEABLE, VmMap, uvm_map, uvm_map_create, uvm_map_setup, uvm_map_submap, uvm_unmap,
};
use crate::uvm::uvm_object::{UvmObject, uvm_obj_is_aobj};
use crate::uvm::uvm_page::{Owner, PG_BUSY, uvm_pagefree, uvm_pagelookup, uvm_pagewait};
use crate::uvm::uvm_param::{VM_KERNEL_SPACE_SIZE, round_page};

/// The kernel maps of `bsd`, as the kernel's `km` asks for them (`kernel::vm::km::KmMaps`).
pub struct BsdKmMaps;

impl BsdKmMaps {
    /// The map a `kmem_va_mode` allocates from (`*kv->kv_map`).
    pub fn map_of(which: KvMap) -> &'static VmMap {
        match which {
            KvMap::Kernel => kernel_map(),
            KvMap::Kmem => kmem_map(),
            KvMap::Exec => crate::uvm::uvm_extern::exec_map(),
            KvMap::Phys => crate::uvm::uvm_extern::phys_map(),
            KvMap::None => panic(format_args!("km_alloc: single page mode has no map")),
        }
    }
}

/// `kernel_map_store`: the kernel map.
static KERNEL_MAP_STORE: VmMap = VmMap::new();
/// `kernel_map`: unset until `uvm_km_init` set the store up.
static KERNEL_MAP: OnceStatic<&'static VmMap> = OnceStatic::new();

/// The bridge's implementation, installed by [`uvm_km_init`].
static BSD_KM_MAPS: BsdKmMaps = BsdKmMaps;

/// `kernel_map`: the kernel's map, once `uvm_km_init` set it up.
pub fn kernel_map() -> &'static VmMap {
    match KERNEL_MAP.get() {
        Some(map) => map,
        None => panic(format_args!("kernel_map used before uvm_km_init")),
    }
}

/// `map == kernel_map`: whether `map` is the kernel map (false before `uvm_km_init`, as
/// the C's null `kernel_map` compares).
pub fn is_kernel_map(map: &VmMap) -> bool {
    KERNEL_MAP.get().is_some_and(|k| ptr::eq(*k, map))
}

/// `kp_dma`: pages every DMA-capable device can reach.
pub static KP_DMA: KmemPaMode = pa_mode(<Machine as Pmap>::DMA_CONSTRAINT);

/// `kp_dma_contig`: one physically contiguous DMA-reachable segment.
pub static KP_DMA_CONTIG: KmemPaMode = KmemPaMode {
    kp_maxseg: 1,
    ..pa_mode(<Machine as Pmap>::DMA_CONSTRAINT)
};

/// `kp_dma_zero`: zeroed DMA-reachable pages.
pub static KP_DMA_ZERO: KmemPaMode = KmemPaMode {
    kp_zero: true,
    ..pa_mode(<Machine as Pmap>::DMA_CONSTRAINT)
};

/// `uvm_km_init`: init kernel maps and objects to reflect reality (i.e. KVM already
/// allocated for text, data, bss, and static data structures).
///
/// - KVM is defined by `[base.. base + VM_KERNEL_SPACE_SIZE]`. we assume that
///   `[base -> start]` has already been allocated and that "end" is the end of the kernel
///   image span.
pub fn uvm_km_init(base: Vaddr, start: Vaddr, end: Vaddr) {
    // The kernel's allocator reaches the maps through this; the kernel object's and the
    // reservation's allocations below already go through it.
    if install(&BSD_KM_MAPS).is_err() {
        panic(format_args!("uvm_km_init: called twice"));
    }

    // kernel_object: for pageable anonymous kernel memory
    uao_init();
    let Some(kernel_object) = uao_create(Vsize::new(VM_KERNEL_SPACE_SIZE), UAO_FLAG_KERNOBJ) else {
        panic(format_args!("uvm_km_init: no kernel object"));
    };
    UVM.kernel_object.set(kernel_object);

    // init the map and reserve already allocated kernel space before installing.
    // KVA_GUARDPAGES is not configured.
    uvm_map_setup(
        &KERNEL_MAP_STORE,
        PmapRef::Kernel,
        base.as_usize(),
        end.as_usize(),
        VM_MAP_PAGEABLE,
    );
    let mut reserve = base.as_usize();
    if base != start
        && uvm_map(
            &KERNEL_MAP_STORE,
            &mut reserve,
            start.as_usize() - base.as_usize(),
            None,
            UVM_UNKNOWN_OFFSET,
            0,
            uvm_mapflag(
                PROT_READ | PROT_WRITE,
                PROT_READ | PROT_WRITE,
                MAP_INHERIT_NONE,
                MADV_RANDOM,
                UVM_FLAG_FIXED,
            ),
        )
        .is_err()
    {
        panic(format_args!(
            "uvm_km_init: could not reserve space for kernel"
        ));
    }

    if KERNEL_MAP.set(&KERNEL_MAP_STORE).is_err() {
        panic(format_args!("uvm_km_init: called twice"));
    }
    set_bounds(start, end);

    // __HAVE_PMAP_DIRECT: no uvm_km_pages.mtx.
}

/// `uvm_km_suballoc`: allocate a submap in the kernel map. once a submap is allocated all
/// references to that area of VM must go through it. this allows the locking of VAs in
/// kernel_map to be broken up into regions.
///
/// - if `fixed` is true, `*min` specifies where the region described by the submap must
///   start
/// - if submap is given we use that as the submap, otherwise we alloc a new map
pub fn uvm_km_suballoc(
    map: &VmMap,
    min: &mut usize,
    max: &mut usize,
    size: usize,
    flags: i32,
    fixed: bool,
    submap: Option<&'static VmMap>,
) -> &'static VmMap {
    let mapflags = UVM_FLAG_NOMERGE | if fixed { UVM_FLAG_FIXED } else { 0 };

    let size = round_page(size); // round up to pagesize

    // first allocate a blank spot in the parent map
    if uvm_map(
        map,
        min,
        size,
        None,
        UVM_UNKNOWN_OFFSET,
        0,
        uvm_mapflag(
            PROT_READ | PROT_WRITE,
            PROT_READ | PROT_WRITE,
            MAP_INHERIT_NONE,
            MADV_RANDOM,
            mapflags,
        ),
    )
    .is_err()
    {
        panic(format_args!(
            "uvm_km_suballoc: unable to allocate space in parent map"
        ));
    }

    // set VM bounds (min is filled in by uvm_map)
    *max = *min + size;

    // add references to pmap and create or init the submap
    let pmap = pmap_reference(map.pmap_ref());
    let submap = match submap {
        None => match uvm_map_create(pmap, *min, *max, flags) {
            Some(submap) => submap,
            None => panic(format_args!("uvm_km_suballoc: unable to create submap")),
        },
        Some(submap) => {
            uvm_map_setup(submap, pmap, *min, *max, flags);

            submap
        }
    };

    // now let uvm_map_submap plug in it...
    if uvm_map_submap(map, *min, *max, submap).is_err() {
        panic(format_args!("uvm_km_suballoc: submap allocation failed"));
    }

    submap
}

/// `uvm_km_pgremove`: remove pages from a kernel uvm_object.
///
/// - when you unmap a part of anonymous kernel memory you want to toss the pages right
///   away. (this gets called from uvm_unmap_...).
pub fn uvm_km_pgremove(uobj: &UvmObject, startva: Vaddr, endva: Vaddr) {
    let start = (startva.as_usize() - kernel_map().min_offset.get()) as Voff;
    let end = (endva.as_usize() - kernel_map().min_offset.get()) as Voff;
    let mut swpgonlydelta = 0;

    kassert!(uvm_obj_is_aobj(uobj));
    kassert!(rw_write_held(uobj.vmobjlock()));

    pmap_remove(pmap_kernel(), startva, endva);
    let mut curoff = start;
    while curoff < end {
        let pp = uvm_pagelookup(uobj, curoff);
        if let Some(pg) = pp.filter(|pg| pg.flags() & PG_BUSY != 0) {
            uvm_pagewait(pg, Owner::Object(uobj), uobj.vmobjlock(), "km_pgrm");
            rw_enter_write(uobj.vmobjlock());
            continue; // loop back to us
        }

        // free the swap slot, then the page
        let slot = uao_dropswap(uobj, (curoff >> PAGE_SHIFT) as i32);

        match pp {
            Some(pg) => uvm_pagefree(pg, Owner::Object(uobj)),
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

impl KmMaps for BsdKmMaps {
    fn map(
        &self,
        which: KvMap,
        sz: usize,
        object: KmObject,
        prefer: Voff,
        align: usize,
        mapflags: u32,
    ) -> Result<Vaddr, Errno> {
        let map = Self::map_of(which);
        let uobj = match object {
            KmObject::None => None,
            KmObject::Kernel => UVM.kernel_object(),
        };
        let mut va = map.min_offset.get();
        uvm_map(map, &mut va, sz, uobj, prefer, align, mapflags)?;
        Ok(Vaddr::new(va))
    }

    fn unmap(&self, which: KvMap, sva: Vaddr, eva: Vaddr) {
        uvm_unmap(Self::map_of(which), sva.as_usize(), eva.as_usize());
    }

    fn chan(&self, which: KvMap) -> *const () {
        ptr::from_ref(Self::map_of(which)).cast::<()>()
    }
}

/// `km_alloc`: allocate `sz` bytes of kernel memory with the given virtual, physical and
/// dynamic modes (`kernel::vm::km::km_alloc`, its address as the legacy pointer).
pub fn km_alloc(
    sz: usize,
    kv: &KmemVaMode,
    kp: &KmemPaMode,
    kd: &KmemDynMode,
) -> Option<NonNull<u8>> {
    let va = kernel::vm::km::km_alloc(sz, kv, kp, kd)?;
    NonNull::new(ptr::with_exposed_provenance_mut::<u8>(va.as_usize()))
}

/// `km_free`: returns what `km_alloc` gave, with the same modes (`kernel::vm::km::km_free`).
pub fn km_free(v: NonNull<u8>, sz: usize, kv: &KmemVaMode, kp: &KmemPaMode) {
    kernel::vm::km::km_free(Vaddr::new(v.as_ptr().addr()), sz, kv, kp);
}

/// The host tests' stand-in for [`uvm_km_init`]'s first step, which they never run: installs
/// the maps bridge so the kernel's allocator serves the pools and `malloc` over the test's
/// memory (a second call keeps the first), and bootstraps the host pmap, so the kernel map's
/// token is adopted before any path of the kernel's allocator that maps can run (a machine's
/// boot glue does it before `uvm_init`).
#[cfg(test)]
pub(crate) fn uvm_km_test_init() {
    let _ = install(&BSD_KM_MAPS);
    <Machine as Pmap>::pmap_bootstrap_lazy();
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The adapter over the host's memory (`setup_real_memory`, which runs uvm_km_test_init).

    use std::assert_eq;

    use super::*;
    use crate::kern::subr_pool::tests::setup_real_memory;

    fn free_pages() -> i32 {
        UVMEXP.free.load(Ordering::Relaxed)
    }

    #[test]
    fn km_free_puts_the_pages_back_on_the_free_list() {
        let _g = setup_real_memory();
        let before = free_pages();
        let p = km_alloc(2 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("two pages");
        assert_eq!(free_pages(), before - 2);
        km_free(p, 2 * PAGE_SIZE, &KV_ANY, &KP_ZERO);
        // uvm_pglistfree's way: counted free at once. On the host (no `multiprocessor`) the
        // per-CPU cache is the free list itself, so this cannot tell the two paths apart; the
        // kernel crate's km tests do (`UVMEXP.free` against the cache's count).
        assert_eq!(free_pages(), before);
    }

    #[test]
    fn pgremove_intrsafe_runs_after_the_test_set_up() {
        let _g = setup_real_memory();
        // The kernel's token is reachable once uvm_km_test_init ran: an empty range is a
        // kremove of nothing, not a panic.
        let va = Vaddr::new(0xffff_9800_0000_0000);
        uvm_km_pgremove_intrsafe(va, va);
    }
}
/* </TESTS> */
