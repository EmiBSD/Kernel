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
 * from: Id: uvm_pager.h,v 1.1.2.14 1998/01/13 19:00:50 chuck Exp
 */

/*
 * Copyright (c) 1990 University of Utah.
 * Copyright (c) 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
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
 *	@(#)vm_pager.h	8.5 (Berkeley) 7/7/94
 */
/* </LICENSES> */

/* <CODE> */
//! The pager interface: the `PGO_*` flags, the `VM_PAGER_*` results and the page slots of
//! `<uvm/uvm_pager.h>`, and `uvm/uvm_pager.c`, the generic functions that assist the pagers:
//! the pager map (`uvm_pseg_*`, `uvm_pagermapin`/`out`) and the cluster building
//! (`uvm_mk_pcluster`, `uvm_pager_put`, `uvm_pager_dropcluster`).
//!
//! Upstream: sys/uvm/uvm_pager.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_pager.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_pager.rs@3c62ede68802
//!
//! The pager operations themselves are the methods of `kernel::vm::object::ObjRef`.
//!
//! ## Deviations
//! - `pgo_put`'s last argument is the `PGO_*` flags as an `i32`: the C's prototype says
//!   `boolean_t`, but `uvm_pager_put` passes its flags and `uvn_put` reads them.
//! - `pgo_mk_pcluster` and [`uvm_mk_pcluster`] return the index in `pps` where the cluster
//!   starts, and [`uvm_pager_put`] takes `pps` and a `&mut usize` for the C's `struct
//!   vm_page ***ppsp_ptr`.
//! - The device pager (`uvm_device.c`) is not ported. Nothing is the page daemon yet
//!   (`uvm_pdaemon.c`, N3.5), so [`uvm_pseg_get`] never digs into the reserve and
//!   [`uvm_pseg_reserve_available`] asserts a caller that cannot exist.
//! - [`uvm_pager_dropcluster`] refuses (`panic(9)`) a released anon page, where the C calls
//!   `uvm_anon_release`: an anon page is a swap cluster's (`uobj == NULL` in the C), which this
//!   function, for a non-swap-backed object's cluster, never gets (there is no swap pageout,
//!   N3.5), and its anon is not reachable from the page (`kernel::vm::page` keeps no back
//!   pointer).
//! - The async pageout (`uvm_aio_biodone`, `uvm_swap_dropcluster`, `uvm_aio_aiodone`,
//!   `uvm.aio_done`) is the swap pager's (N3.5) and not here.
//!
//! ## Redesign
//! - `PGO_DONTCARE` (`(struct vm_page *)-1`, a pointer the pagers compare with and never
//!   follow) in arrays of `struct vm_page *` -> [`PageSlot`]: `Empty` (`NULL`), `DontCare`,
//!   `Page(&'static VmPage)`; a pager's page array is `&mut [PageSlot]`.
//! - `struct uvm_pagerops` and `uvmpagerops[]` -> the methods of `ObjRef` (`object.rs`);
//!   [`uvm_pager_init`] calls the pagers' init functions by name, and returns the error of
//!   one that could not make its pool (pipe's precedent).
//! - `uvm_pseg_lck` and `psegs[]` beside it, `msleep_nsec(&psegs, &uvm_pseg_lck, ..)` -> the
//!   segments inside a `hal::mutex::Mutexed` at `IPL_VM` ([`PSEGS`]), slept on with
//!   `MutexedGuard::sleep_nsec`; the wait channel is still the segments' address.
//! - `uvm_pagermapin(pps, npages, flags) -> vaddr_t` and `uvm_pagermapout(kva, npages)` ->
//!   [`uvm_pagermapin`] returning a [`PagerWindow`]: the `hal::mapped::MappedBytes` view of the
//!   pages (made with the kernel map's token, `kernel::vm::km`), and the pager segment it sits
//!   in, given back after the mappings go; [`uvm_pagermapout`] is its drop. A window the
//!   kernel map cannot enter at once is retried whole after `uvm_wait`, where the C retries the
//!   page that failed (the same mappings in the end).
//! - The cluster functions take the object's write witness (`&WriteWitness`, `&mut` where the
//!   pager's I/O releases it): the C's `KASSERT(rw_write_held(uobj->vmobjlock))` as a type.

use core::ptr;

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::intr::IPL_VM;
use hal::mapped::MappedBytes;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::pmap::HAVE_PMAP_DIRECT;
use hal::rwlock::WriteWitness;
use hal::sleep::{INFSLP, PVM, wakeup};
use hal::sys::errno::Errno;
use hal::sys::mman::{PROT_READ, PROT_WRITE};
use hal::sys::types::{Paddr, Vaddr};

use super::km::{KD_TRYLOCK, KP_NONE, KV_ANY, km_alloc, km_free, mapper};
use super::object::{ObjRef, slot_page};
use super::page::{
    PG_BUSY, PG_CLEAN, PG_CLEANCHK, PG_FAKE, PG_RELEASED, PG_WANTED, PQ_ANON, PQ_INACTIVE,
    PageQueues, VmPage, uvm_pagelookup,
};
use super::pmemrange::uvm_wait;
use super::types::Voff;
use super::vnode::uvn_init;
use crate::assert::{kassert, kdassert, kpanic};

// pager flags [mostly for flush]

/// `PGO_CLEANIT`: write dirty pages to backing store.
pub const PGO_CLEANIT: i32 = 0x001;
/// `PGO_SYNCIO`: if PGO_CLEANIT: use sync I/O?
pub const PGO_SYNCIO: i32 = 0x002;
/// `PGO_DEACTIVATE`: deactivate flushed pages.
pub const PGO_DEACTIVATE: i32 = 0x004;
/// `PGO_FREE`: free flushed pages (if not set then the pages stay where they are).
pub const PGO_FREE: i32 = 0x008;
/// `PGO_ALLPAGES`: flush whole object/get all pages.
pub const PGO_ALLPAGES: i32 = 0x010;
/// `PGO_DOACTCLUST`: flag to mk_pcluster to include active.
pub const PGO_DOACTCLUST: i32 = 0x020;
/// `PGO_LOCKED`: fault data structures are locked \[get\].
pub const PGO_LOCKED: i32 = 0x040;
/// `PGO_PDFREECLUST`: daemon's free cluster flag \[uvm_pager_put\].
pub const PGO_PDFREECLUST: i32 = 0x080;
/// `PGO_NOWAIT`: do not wait for inode lock.
pub const PGO_NOWAIT: i32 = 0x200;

/// A slot of a pager's page array (`struct vm_page *`): a page, none yet (`NULL`), or a page
/// the caller is not interested in getting (`PGO_DONTCARE`, \[get only\]).
#[derive(Clone, Copy, Default)]
pub enum PageSlot {
    /// `NULL`: no page (yet).
    #[default]
    Empty,
    /// `PGO_DONTCARE`: page we are not interested in getting.
    DontCare,
    /// A page.
    Page(&'static VmPage),
}

impl PageSlot {
    /// The page in the slot.
    pub fn page(self) -> Option<&'static VmPage> {
        match self {
            PageSlot::Page(pg) => Some(pg),
            PageSlot::Empty | PageSlot::DontCare => None,
        }
    }

    /// `pps[i] == NULL`.
    pub fn is_empty(self) -> bool {
        matches!(self, PageSlot::Empty)
    }

    /// `pps[i] == PGO_DONTCARE`.
    pub fn is_dontcare(self) -> bool {
        matches!(self, PageSlot::DontCare)
    }
}

// Two slots are equal when they hold the same page (by address, as the C compares pointers).
impl PartialEq for PageSlot {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PageSlot::Empty, PageSlot::Empty) | (PageSlot::DontCare, PageSlot::DontCare) => true,
            (PageSlot::Page(a), PageSlot::Page(b)) => ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl core::fmt::Debug for PageSlot {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PageSlot::Empty => f.write_str("Empty"),
            PageSlot::DontCare => f.write_str("DontCare"),
            PageSlot::Page(pg) => write!(f, "Page({})", pg.index()),
        }
    }
}

/// `UVMPAGER_MAPIN_WAITOK`: it's okay to wait.
pub const UVMPAGER_MAPIN_WAITOK: i32 = 0x01;
/// `UVMPAGER_MAPIN_READ`: host <- device.
pub const UVMPAGER_MAPIN_READ: i32 = 0x02;
/// `UVMPAGER_MAPIN_WRITE`: device -> host (pseudo flag).
pub const UVMPAGER_MAPIN_WRITE: i32 = 0x00;

// get/put return values

/// `VM_PAGER_OK`: operation was successful.
pub const VM_PAGER_OK: i32 = 0;
/// `VM_PAGER_BAD`: specified data was out of the accepted range.
pub const VM_PAGER_BAD: i32 = 1;
/// `VM_PAGER_FAIL`: specified data was in range, but doesn't exist.
pub const VM_PAGER_FAIL: i32 = 2;
/// `VM_PAGER_PEND`: operations was initiated but not completed.
pub const VM_PAGER_PEND: i32 = 3;
/// `VM_PAGER_ERROR`: error while accessing data that is in range and exists.
pub const VM_PAGER_ERROR: i32 = 4;
/// `VM_PAGER_AGAIN`: temporary resource shortage prevented operation from happening.
pub const VM_PAGER_AGAIN: i32 = 5;
/// `VM_PAGER_UNLOCK`: unlock the map and try again.
pub const VM_PAGER_UNLOCK: i32 = 6;
/// `VM_PAGER_REFAULT`: \[uvm_fault internal use only!\] unable to relock data structures,
/// thus the mapping needs to be reverified before we can proceed.
pub const VM_PAGER_REFAULT: i32 = 7;

/// `PAGER_MAP_SIZE`: XXX this is needed until the device strategy interface is changed to do
/// physically-addressed i/o.
pub const PAGER_MAP_SIZE: usize = 16 * 1024 * 1024;

/// `MAXBSIZE` (`<sys/param.h>`): the largest file system block, the largest pager I/O.
pub const MAXBSIZE: usize = 64 * 1024;

/// `PMAP_PREFER_ALIGN()`: the alignment the pmap prefers for a mapping; 0 where `PMAP_PREFER`
/// is not defined (amd64, arm64: no virtual cache aliasing).
const PMAP_PREFER_ALIGN: usize = 0;

// the pager map: provides KVA for I/O
//
// Each uvm_pseg has room for MAX_PAGERMAP_SEGS pager io space of MAXBSIZE bytes.
//
// The number of uvm_pseg instances is dynamic using an array segs. At most UVM_PSEG_COUNT
// instances can exist.
//
// psegs[0/1] always exist (so that the pager can always map in pages). psegs[0/1] element 0/1
// are always reserved for the pagedaemon.
//
// Any other pseg is automatically created when no space is available and automatically
// destroyed when it is no longer in use.

/// `MAX_PAGER_SEGS`.
pub const MAX_PAGER_SEGS: usize = 16;
/// `PSEG_NUMSEGS`.
pub const PSEG_NUMSEGS: usize = PAGER_MAP_SIZE / MAX_PAGER_SEGS / MAXBSIZE;

/// `struct uvm_pseg`.
#[derive(Clone, Copy)]
pub struct Pseg {
    /// `start`: start of virtual space; 0 if not inited.
    start: usize,
    /// `use`: bitmap of the segments in use in this pseg.
    use_: i32,
}

impl Pseg {
    /// An uninitialised pseg.
    const fn new() -> Self {
        Self { start: 0, use_: 0 }
    }

    /// `UVM_PSEG_FULL(pseg)`.
    fn full(&self) -> bool {
        self.use_ == (1 << MAX_PAGER_SEGS) - 1
    }

    /// `UVM_PSEG_EMPTY(pseg)`.
    fn empty(&self) -> bool {
        self.use_ == 0
    }

    /// `UVM_PSEG_INUSE(pseg, id)`.
    fn inuse(&self, id: usize) -> bool {
        self.use_ & (1 << id) != 0
    }
}

/// `psegs[PSEG_NUMSEGS]` under `uvm_pseg_lck`, the pager map's lock (`IPL_VM`).
static PSEGS: Mutexed<[Pseg; PSEG_NUMSEGS]> = Mutexed::new(IPL_VM, [Pseg::new(); PSEG_NUMSEGS]);

/// The wait channel of the pager map (`&psegs`).
fn psegs_chan() -> *const () {
    ptr::from_ref(&PSEGS).cast()
}

/// `uvm_pager_init`: init pagers (at boot time): the pager map's first two segments, then
/// the pagers' init functions (`uvmpagerops[]`: the aobj pager has none, the vnode pager's is
/// `uvn_init`). The error of a pager's init (its pool could not be made) is returned.
pub fn uvm_pager_init() -> Result<(), Errno> {
    // init pager map
    let mut psegs = PSEGS.lock();
    uvm_pseg_init(&mut psegs[0]);
    uvm_pseg_init(&mut psegs[1]);
    drop(psegs);

    // init ASYNC I/O queue: TAILQ_INIT(&uvm.aio_done): the async pageout path (uvm_aio_*,
    // the swap pager) is not here yet.

    // call pager init functions
    uvn_init()
}

/// Initialize a uvm_pseg.
///
/// May fail, in which case seg->start == 0.
///
/// Caller locks uvm_pseg_lck.
fn uvm_pseg_init(pseg: &mut Pseg) {
    kassert!(pseg.start == 0);
    kassert!(pseg.use_ == 0);
    pseg.start = km_alloc(MAX_PAGER_SEGS * MAXBSIZE, &KV_ANY, &KP_NONE, &KD_TRYLOCK)
        .map_or(0, Vaddr::as_usize);
}

/// Whether the caller is the page daemon (`uvm_pdaemon.c` is not here, so never).
fn curproc_is_pagedaemon() -> bool {
    false
}

/// `uvm_pseg_reserve_available()`: the pagedaemon's reserved segments that are free.
pub fn uvm_pseg_reserve_available() -> i32 {
    let mut count = 0;

    kassert!(curproc_is_pagedaemon());

    let psegs = PSEGS.lock();
    for pseg in &psegs[..2] {
        for j in 0..2 {
            if !pseg.inuse(j) {
                count += 1;
            }
        }
    }

    count
}

/// Acquire a pager map segment.
///
/// Returns a vaddr for paging; `None` on failure (the C's 0): no segment could be made, or
/// none is free and `UVMPAGER_MAPIN_WAITOK` was not given (with it, sleeps until one is).
///
/// Caller does not lock.
pub fn uvm_pseg_get(flags: i32) -> Option<Vaddr> {
    let mut use_reserve = false;

    let mut psegs = PSEGS.lock();

    'restart: loop {
        // pager_seg_restart:
        let fail = 'search: {
            // Find first pseg that has room.
            for (n, pseg) in psegs.iter_mut().enumerate() {
                if pseg.full() {
                    continue;
                }

                if pseg.start == 0 {
                    // Need initialization.
                    uvm_pseg_init(pseg);
                    if pseg.start == 0 {
                        break 'search true;
                    }
                }

                // Keep indexes 0,1 reserved for pagedaemon.
                let first = if n < 2 && !use_reserve { 2 } else { 0 };

                for i in first..MAX_PAGER_SEGS {
                    if !pseg.inuse(i) {
                        pseg.use_ |= 1 << i;
                        return Some(Vaddr::new(pseg.start + i * MAXBSIZE));
                    }
                }
            }
            false
        };

        // Let the pagdeamon dig into its reserve if we couldn't get a "normal" slot.
        if !fail && curproc_is_pagedaemon() && !use_reserve {
            use_reserve = true;
            continue 'restart;
        }

        // pager_seg_fail:
        if flags & UVMPAGER_MAPIN_WAITOK != 0 {
            let _ = psegs.sleep_nsec(psegs_chan(), PVM, "pagerseg", INFSLP);
            continue 'restart;
        }

        return None;
    }
}

/// Release a pager map segment.
///
/// Caller does not lock.
///
/// Deallocates pseg if it is no longer in use.
pub fn uvm_pseg_release(segaddr: Vaddr) {
    let segaddr = segaddr.as_usize();
    let mut va = 0;

    let mut psegs = PSEGS.lock();
    let Some((n, pseg)) = psegs.iter_mut().enumerate().find(|(_, p)| {
        p.start != 0 && p.start <= segaddr && segaddr < p.start + MAX_PAGER_SEGS * MAXBSIZE
    }) else {
        kpanic(format_args!(
            "uvm_pseg_release: {segaddr:#x} is no pager segment"
        ));
    };

    let id = (segaddr - pseg.start) / MAXBSIZE;
    kassert!(id < MAX_PAGER_SEGS);

    // test for no remainder
    kdassert!(segaddr == pseg.start + id * MAXBSIZE);

    kassert!(pseg.inuse(id));

    pseg.use_ &= !(1 << id);
    wakeup(psegs_chan());

    if n >= 2 && pseg.empty() {
        va = pseg.start;
        pseg.start = 0;
    }

    drop(psegs);

    if va != 0 {
        km_free(Vaddr::new(va), MAX_PAGER_SEGS * MAXBSIZE, &KV_ANY, &KP_NONE);
    }
}

/// A pager segment taken by [`uvm_pagermapin`], given back when dropped.
struct SegHold(Vaddr);

impl Drop for SegHold {
    fn drop(&mut self) {
        uvm_pseg_release(self.0);
    }
}

/// The pages of a pager I/O mapped into kernel virtual space ([`uvm_pagermapin`]): their bytes
/// as a slice. The drop removes the mappings, then gives the pager segment back
/// ([`uvm_pagermapout`]). The pages stay busy while the window lives (`hal::mapped`'s rule).
pub struct PagerWindow {
    /// The view (first, so it is dropped first: the mappings go before the segment).
    bytes: MappedBytes<'static>,
    /// The segment the view sits in; `None` for the direct map.
    _seg: Option<SegHold>,
}

impl PagerWindow {
    /// The window's address (the C's `kva`).
    pub fn va(&self) -> Vaddr {
        self.bytes.va()
    }

    /// The pages' bytes.
    pub fn as_slice(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    /// The pages' bytes, to write (a window mapped for `UVMPAGER_MAPIN_READ`, or the direct
    /// map).
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        self.bytes.as_slice_mut()
    }
}

/// uvm_pagermapin: map pages into KVA for I/O that needs mappings
///
/// We basically just km_valloc a blank map entry to reserve the space in the kernel map and
/// then use pmap_enter() to put the mappings in by hand. The pages of `pps` are busy (and stay
/// so while the window lives). `None` when no window could be had and `UVMPAGER_MAPIN_WAITOK`
/// was not given.
pub fn uvm_pagermapin(pps: &[PageSlot], flags: i32) -> Option<PagerWindow> {
    let npages = pps.len();

    // __HAVE_PMAP_DIRECT: use direct mappings for single page, unless there is a risk of
    // aliasing.
    if HAVE_PMAP_DIRECT && npages == 1 && PMAP_PREFER_ALIGN == 0 {
        let pg = slot_page(pps[0], "uvm_pagermapin");
        kassert!(pg.flags() & PG_BUSY != 0);
        return Some(PagerWindow {
            bytes: mapper().map_direct(pg.phys_addr()),
            _seg: None,
        });
    }

    let mut prot = PROT_READ;
    if flags & UVMPAGER_MAPIN_READ != 0 {
        prot |= PROT_WRITE;
    }
    let size = npages * PAGE_SIZE;

    if size > MAXBSIZE {
        kpanic(format_args!("uvm_pagermapin: {npages} pages"));
    }

    let mut frames = [Paddr::new(0); MAXBSIZE / PAGE_SIZE];
    for (pa, &slot) in frames.iter_mut().zip(pps) {
        let pp = slot_page(slot, "uvm_pagermapin");
        kassert!(pp.flags() & PG_BUSY != 0);
        *pa = pp.phys_addr();
    }

    let kva = uvm_pseg_get(flags)?;
    let seg = SegHold(kva);

    loop {
        match mapper().map_frames(kva, &frames[..npages], prot) {
            Ok(bytes) => {
                return Some(PagerWindow {
                    bytes,
                    _seg: Some(seg),
                });
            }
            Err(_) => {
                if flags & UVMPAGER_MAPIN_WAITOK != 0 {
                    uvm_wait("pgrmapin");
                } else {
                    // The entered frames were removed; the segment goes back with `seg`.
                    return None;
                }
            }
        }
    }
}

/// uvm_pagermapout: remove KVA mapping
///
/// We remove our mappings by hand and then remove the mapping.
pub fn uvm_pagermapout(win: PagerWindow) {
    drop(win);
}

/// uvm_mk_pcluster
///
/// generic "make 'pager put' cluster" function. a pager can either \[1\] set pgo_mk_pcluster
/// to NULL (never cluster), \[2\] set it to this generic function, or \[3\] set it to a pager
/// specific function.
///
/// => caller must lock object (`w`) _and_ pagequeues (since we need to look at active vs.
///    inactive bits, etc.)
/// => caller must make center page busy and write-protect it
/// => we mark all cluster pages busy for the caller
/// => the caller must unbusy all pages (and check wanted/released status if it drops the
///    object lock)
/// => flags:
///      PGO_ALLPAGES:  all pages in object are valid targets
///      !PGO_ALLPAGES: use "lo" and "hi" to limit range of cluster
///      PGO_DOACTCLUST: include active pages in cluster.
///      PGO_FREE: set the PG_RELEASED bits on the cluster so they'll be freed in async io
///      (caller must clean on error).
///        NOTE: the caller should clear PG_CLEANCHK bits if PGO_DOACTCLUST. PG_CLEANCHK is
///        only a hint, but clearing will help reduce the number of calls we make to the pmap
///        layer.
///
/// Returns the index in `pps` where the cluster starts (the C returns that element's
/// address); `*npages` is its length.
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn uvm_mk_pcluster(
    uobj: &ObjRef,
    w: &WriteWitness<'_>,
    pps: &mut [PageSlot],
    npages: &mut i32,
    center: &'static VmPage,
    flags: i32,
    mlo: Voff,
    mhi: Voff,
) -> usize {
    // center page should already be busy and write protected. XXX: suppose page is wired? if
    // we lock, then a process could fault/block on it. if we don't lock, a process could write
    // the pages in the middle of an I/O. (consider an msync()). let's lock it for now (better
    // to delay than corrupt data?).
    // get cluster boundaries, check sanity, and apply our limits as well.
    let (mut lo, mut hi) = uobj.cluster(w, center.offset());
    if flags & PGO_ALLPAGES == 0 {
        lo = lo.max(mlo);
        hi = hi.min(mhi);
    }
    if (hi - lo) >> PAGE_SHIFT > Voff::from(*npages) {
        // pps too small, bail out!
        pps[0] = PageSlot::Page(center);
        *npages = 1;
        return 0;
    }

    // now determine the center and attempt to cluster around the edges
    let center_idx = ((center.offset() - lo) >> PAGE_SHIFT) as usize;
    pps[center_idx] = PageSlot::Page(center); // plug in the center page
    let mut ppsp = center_idx;
    *npages = 1;

    // attempt to cluster around the left [backward], and then the right side [forward].
    //
    // note that for inactive pages (pages that have been deactivated) there are no valid
    // mappings and PG_CLEAN should be up to date. [i.e. there is no need to query the pmap
    // with pmap_is_modified since there are no mappings].
    for forward in [false, true] {
        let incr: Voff = if forward {
            PAGE_SIZE as Voff
        } else {
            -(PAGE_SIZE as Voff)
        };
        let mut curoff = center.offset() + incr;
        while (!forward && curoff >= lo) || (forward && curoff < hi) {
            let Some(pclust) = uvm_pagelookup(&**uobj, curoff) else {
                break; // no page
            };
            // handle active pages
            // NOTE: inactive pages don't have pmap mappings
            if pclust.flags() & PQ_INACTIVE == 0 {
                if flags & PGO_DOACTCLUST == 0 {
                    // dont want mapped pages at all
                    break;
                }

                // make sure "clean" bit is sync'd
                if pclust.flags() & PG_CLEANCHK == 0 {
                    if pclust.flags() & (PG_CLEAN | PG_BUSY) == PG_CLEAN
                        && hal::pmap::is_modified(&pclust.mdpage)
                    {
                        pclust.clear_bits(PG_CLEAN);
                    }
                    // now checked
                    pclust.set_bits(PG_CLEANCHK);
                }
            }

            // is page available for cleaning and does it need it
            if pclust.flags() & (PG_CLEAN | PG_BUSY) != 0 {
                break; // page is already clean or is busy
            }

            // yes! enroll the page in our array
            pclust.set_bits(PG_BUSY);
            // UVM_PAGE_OWN(pclust, "uvm_mk_pcluster"): not configured.

            // If we want to free after io is done, and we're async, set the released flag
            if flags & (PGO_FREE | PGO_SYNCIO) == PGO_FREE {
                pclust.set_bits(PG_RELEASED);
            }

            // XXX: protect wired page? see above comment.
            hal::pmap::page_protect(&pclust.mdpage, PROT_READ);
            if !forward {
                ppsp -= 1; // back up one page
                pps[ppsp] = PageSlot::Page(pclust);
            } else {
                // move forward one page
                pps[ppsp + *npages as usize] = PageSlot::Page(pclust);
            }
            *npages += 1;
            curoff += incr;
        }
    }

    // done! return the cluster array to the caller!!!
    ppsp
}

/// uvm_pager_put: high level pageout routine
///
/// we want to pageout page "pg" to backing store, clustering if possible.
///
/// => page queues must be locked by caller: `pageq` is their guard, dropped (unlocked) once
///    the cluster is built
/// => "uobj" is the object backing it, its lock held for writing (`w`, released and taken
///    again around the I/O)
/// => "pg" should be PG_BUSY (by caller), and !PG_CLEAN
/// => "pps" is an array of `*npages` slots for possible cluster building; the cluster's start
///    index comes back in `ppsp`
/// => flags
///    PGO_ALLPAGES: all pages in uobj are valid targets
///    PGO_DOACTCLUST: include "PQ_ACTIVE" pages as valid targets
///    PGO_SYNCIO: do SYNC I/O (no async)
///    PGO_PDFREECLUST: pagedaemon: drop cluster on successful I/O
///    PGO_FREE: tell the aio daemon to free pages in the async case.
/// => start/stop: if !PGO_ALLPAGES limit targets to this range
/// => return state:
///    1. we return the VM_PAGER status code of the pageout
///    2. we return with the page queues unlocked
///    3. on errors we always drop the cluster. thus, if we return !PEND, !OK, then the caller
///       only has to worry about un-busying the main page (not the cluster pages).
///    4. on success, if !PGO_PDFREECLUST, we return the cluster with all pages busy (caller
///       must un-busy and check wanted/released flags).
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn uvm_pager_put(
    uobj: &ObjRef,
    w: &mut WriteWitness<'_>,
    pg: &'static VmPage,
    pps: &mut [PageSlot],
    ppsp: &mut usize,
    npages: &mut i32,
    flags: i32,
    start: Voff,
    stop: Voff,
    pageq: MutexedGuard<'static, PageQueues>,
) -> i32 {
    // note that the page queues must be locked to cluster.

    // attempt to build a cluster for pageout using its make-put-cluster function (if it has
    // one).
    match uobj.mk_pcluster(w, pps, npages, pg, flags, start, stop) {
        Some(at) => *ppsp = at, // update caller's index
        None => {
            *ppsp = 0;
            pps[0] = PageSlot::Page(pg);
            *npages = 1;
        }
    }

    // now that we've clustered we can unlock the page queues
    drop(pageq);

    // now attempt the I/O. if we have a failure and we are clustered, we will drop the
    // cluster and try again.
    let n = *npages as usize;
    let result = uobj.put(w, &mut pps[*ppsp..*ppsp + n], *npages, flags);

    // we have attempted the I/O.
    //
    // if the I/O was a success then:
    //	if !PGO_PDFREECLUST, we return the cluster to the caller (who must un-busy all pages)
    //	else we un-busy cluster pages for the pagedaemon
    //
    // if I/O is pending (async i/o) then we return the pending code. [in this case the async
    // i/o done function must clean up when i/o is done...]
    if result == VM_PAGER_PEND || result == VM_PAGER_OK {
        if result == VM_PAGER_OK && flags & PGO_PDFREECLUST != 0 {
            // drop cluster
            uvm_pager_dropcluster(w, &pps[*ppsp..*ppsp + n], npages, PGO_PDFREECLUST);
        }
        return result;
    }

    // a pager error occurred (even after dropping the cluster, if there was one).
    uvm_pager_dropcluster(w, &pps[*ppsp..*ppsp + n], npages, 0);

    result
}

/// uvm_pager_dropcluster: drop a cluster we have built (because we got an error, or, if
/// PGO_PDFREECLUST we are un-busying the cluster pages on behalf of the pagedaemon).
///
/// => uobj is a non-swap-backed object, locked for writing (`w`)
/// => page queues are not locked
/// => ppsp/npages is our current cluster
/// => flags: PGO_PDFREECLUST: pageout was a success: un-busy cluster pages on behalf of the
///    pagedaemon.
pub fn uvm_pager_dropcluster(
    w: &WriteWitness<'_>,
    ppsp: &[PageSlot],
    npages: &mut i32,
    flags: i32,
) {
    let _ = w;

    for &slot in ppsp.iter().take(*npages as usize) {
        // skip empty slot
        let Some(pg) = slot.page() else {
            continue;
        };

        // did someone want the page while we had it busy-locked?
        if pg.flags() & PG_WANTED != 0 {
            wakeup(pg.chan());
        }

        // if page was released, release it. otherwise un-busy it
        if pg.flags() & PG_RELEASED != 0 && pg.flags() & PQ_ANON != 0 {
            // kills anon and frees pg: see the module's deviations.
            kpanic(format_args!(
                "uvm_pager_dropcluster: anon page without anon"
            ));
        } else {
            // if we were planning on async io then we would have PG_RELEASED set, clear that
            // with the others.
            pg.clear_bits(PG_BUSY | PG_WANTED | PG_FAKE | PG_RELEASED);
            // UVM_PAGE_OWN(ppsp[lcv], NULL): not configured.
        }

        // if we are operating on behalf of the pagedaemon and we had a successful pageout
        // update the page!
        if flags & PGO_PDFREECLUST != 0 {
            let _ = hal::pmap::clear_reference(&pg.mdpage);
            let _ = hal::pmap::clear_modify(&pg.mdpage);
            pg.set_bits(PG_CLEAN);
        }
    }
}

// uvm_aio_biodone, uvm_swap_dropcluster, uvm_aio_aiodone: the async (swap) pageout, which
// needs uvm_swap.c (N3.5): not here yet.
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use super::*;
    use crate::testutil;

    /// The address the tests give the first two psegs: the host's `km_alloc` has no kernel
    /// virtual space without pages (`PMAP_NOMMU`), so those two, which a booted kernel makes
    /// in `uvm_pager_init`, are set by hand, and no other pseg can be made (as a full kernel
    /// map).
    const SEEDED: usize = 0x7000_0000;

    /// Real page memory, the lock objects, and psegs 0 and 1 seeded and free.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(256 * 4096);
        let _ = hal::rwlock::init_shared();
        let mut psegs = PSEGS.lock();
        for (i, p) in psegs.iter_mut().enumerate().take(2) {
            p.start = SEEDED + i * MAX_PAGER_SEGS * MAXBSIZE;
        }
        assert!(
            psegs.iter().all(|p| p.use_ == 0),
            "a test left a segment in use"
        );
        drop(psegs);
        guard
    }

    #[test]
    fn a_slot_is_a_page_or_not() {
        let slots = [PageSlot::Empty, PageSlot::DontCare];
        assert!(slots[0].is_empty() && !slots[0].is_dontcare() && slots[0].page().is_none());
        assert!(slots[1].is_dontcare() && slots[1].page().is_none());
        assert_eq!(PageSlot::default(), PageSlot::Empty);
        assert_ne!(PageSlot::Empty, PageSlot::DontCare);
    }

    #[test]
    fn pager_segments_are_handed_out_and_taken_back() {
        let _g = setup();
        // Nothing is the page daemon: the first two segments of psegs[0] and [1] stay reserved.
        let a = uvm_pseg_get(0).expect("a segment");
        let b = uvm_pseg_get(0).expect("a second segment");
        assert_eq!(b.as_usize(), a.as_usize() + MAXBSIZE);
        uvm_pseg_release(a);
        let c = uvm_pseg_get(0).expect("the first segment again");
        assert_eq!(c.as_usize(), a.as_usize());
        uvm_pseg_release(b);
        uvm_pseg_release(c);
    }

    #[test]
    fn a_waiting_segment_request_sleeps_until_a_release() {
        let _g = setup();
        // Take every segment there is (psegs 0 and 1 but their reserved two each; no other
        // pseg can be made), then wait for one from another thread.
        let mut held = std::vec::Vec::new();
        while let Some(va) = uvm_pseg_get(0) {
            held.push(va);
        }
        assert_eq!(held.len(), 2 * (MAX_PAGER_SEGS - 2));
        let last = held.pop().expect("a segment");
        let t = std::thread::spawn(move || uvm_pseg_get(UVMPAGER_MAPIN_WAITOK));
        std::thread::sleep(std::time::Duration::from_millis(20));
        uvm_pseg_release(last);
        let got = t.join().expect("the waiter").expect("a segment");
        assert_eq!(got.as_usize(), last.as_usize());
        uvm_pseg_release(got);
        for va in held {
            uvm_pseg_release(va);
        }
    }

    #[test]
    fn a_two_page_window_maps_both_pages_and_unmaps_before_its_segment() {
        use super::super::page::{Pglist, uvm_pglistalloc, uvm_pglistfree};
        use super::super::types::UVM_PLA_NOWAIT;

        let _g = setup();
        // Two contiguous pages (the host has no MMU: a window is the frames' own addresses),
        // busy as a pager's I/O pages.
        let pgl = Pglist::new();
        uvm_pglistalloc(
            2 * PAGE_SIZE,
            Paddr::new(0),
            Paddr::new(usize::MAX),
            Paddr::new(PAGE_SIZE),
            Paddr::new(0),
            &pgl,
            1,
            UVM_PLA_NOWAIT,
        )
        .expect("two pages");
        let p0 = pgl.pop_head().expect("a page");
        let p1 = pgl.pop_head().expect("a second page");
        let (p0, p1) = if p0.phys_addr() < p1.phys_addr() {
            (p0, p1)
        } else {
            (p1, p0)
        };
        p0.set_bits(PG_BUSY);
        p1.set_bits(PG_BUSY);
        let pps = [PageSlot::Page(p0), PageSlot::Page(p1)];

        // More than one page: a pager segment's window (map_frames), writable for a read.
        let mut win = uvm_pagermapin(&pps, UVMPAGER_MAPIN_READ).expect("a window");
        let va = win.va();
        assert!(va.as_usize() >= SEEDED, "in a pager segment");
        let kernel = hal::pmap::kernel();
        assert_eq!(kernel.extract(va), Some(p0.phys_addr()));
        assert_eq!(
            kernel.extract(va + hal::sys::types::Vsize::new(PAGE_SIZE)),
            Some(p1.phys_addr())
        );
        assert_eq!(win.as_slice().len(), 2 * PAGE_SIZE);
        win.as_slice_mut()[PAGE_SIZE] = 0xa5;
        assert_eq!(win.as_slice()[PAGE_SIZE], 0xa5);

        // The drop: the mappings go first, then the segment, free again for the next get.
        uvm_pagermapout(win);
        assert!(kernel.extract(va).is_none());
        assert!(
            kernel
                .extract(va + hal::sys::types::Vsize::new(PAGE_SIZE))
                .is_none()
        );
        let again = uvm_pseg_get(0).expect("the segment back");
        assert_eq!(again.as_usize(), va.as_usize());
        uvm_pseg_release(again);

        p0.clear_bits(PG_BUSY);
        p1.clear_bits(PG_BUSY);
        pgl.insert_tail(p0);
        pgl.insert_tail(p1);
        uvm_pglistfree(&pgl); // takes them off the list
    }
}
/* </TESTS> */
