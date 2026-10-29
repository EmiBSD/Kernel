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
 * Copyright (c) 2007 Artur Grabowski <art@openbsd.org>
 * Copyright (c) 2012-2016,2019 Bob Beck <beck@openbsd.org>
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
//! The memory behind the buffer cache (`vfs_biomem.c`): each buffer's pages live in its own
//! (or its cluster's) `uvm_object`, and a `MAXPHYS` slot of a kernel virtual arena maps them
//! while the buffer is busy; idle buffers keep their mapping on an LRU (`buf_valist`) until a
//! busy one needs the slot.
//!
//! Upstream: sys/kern/vfs_biomem.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_biomem.rs@3c62ede68802
//!
//! Every function runs with the cache's lock held ([`super::bio::Cache`], the C's `splbio`),
//! which guards `buf_valist`, the arena and the counters it moves.
//!
//! ## Deviations
//! - On a machine without an MMU (`PMAP_NOMMU`: only the host test double) the arena
//!   reserves no kernel virtual space (its slots are fake addresses, `kernel::vm::km`), the
//!   pages are physically contiguous (`uvm_pagealloc_multi`) and a buffer's bytes are read
//!   through the direct map (`hal::mapped`), LZ's deviation.
//!
//! ## Redesign
//! - LZ's `buf_kva_start`/`buf_kva_end` atomics and slot addresses as `usize`s -> the
//!   arena `kernel::vm::km::BufArena` in the cache's state, handing out `BufSlot`s, the only
//!   addresses a buffer's pages are entered at; `buf_map`'s `pmap_kenter_pa` loop through
//!   `bsd`'s legacy `unsafe fn` -> `BufSlot::map`, a `hal::mapped::MappedBytes` over an
//!   unmanaged mapping that the buffer's token holds while it is busy and the buffer keeps
//!   while idle (`kernel::fs::buf::Kva`); `buf_unmap`'s `pmap_kremove` -> `BufArena::unmap`
//!   of that view; `buf_fix_mapping` -> `MappedBytes::truncate`.
//! - `buf_dealloc_mem`'s "keep the kva, free the pages, `B_RELEASED`" -> the view dropped,
//!   the pages freed, the bare slot kept by the idle buffer at the head of `buf_valist`; the
//!   buffer stays on `bufhead` (which owns it) until `buf_unmap` takes the slot and gives the
//!   buffer back to `bufpl` (the C took it off `bufhead` in `buf_put` and freed it from the
//!   list it was still on).
//! - `buf_acquire`/`buf_release` take and give back the token (`kernel::fs::buf::BufRef`),
//!   whose view is the mapping; `b_data` follows it.

use core::sync::atomic::Ordering;

use hal::sleep::{INFSLP, PRIBIO, curproc_word, wakeup};
use hal::sys::param::PAGE_SIZE;
use hal::sys::types::Paddr;
use hal::vm::param::{atop, ptoa, round_page};

use super::bio::{BCSTATS, Cache, bufbackoff_locked, cleaner_word, syncer_word};
use super::buf::{B_BUSY, B_RELEASED, Buf, BufRef, Kva, MAXPHYS, Pages, RESERVE_SLOTS};
use crate::assert::{kassert, kpanic};
use crate::vm::km::{BufArena, BufSlot};
use crate::vm::object::uvm_obj_free;
use crate::vm::page::{uvm_pagealloc_multi, uvm_pagelookup, vm_page_to_phys};
use crate::vm::types::{UVM_PLA_NOWAIT, UVM_PLA_NOWAKE, UVM_PLA_WAITOK, Voff};
use hal::owned::Member;

/// At most a slot's worth of pages.
const SLOT_PAGES: usize = MAXPHYS / PAGE_SIZE;

/// The frames of a buffer's pages, up to a slot's worth (`buf_map`'s loop over them).
struct Frames {
    f: [Paddr; SLOT_PAGES],
    n: usize,
}

impl Frames {
    const fn new() -> Self {
        Self {
            f: [Paddr::new(0); SLOT_PAGES],
            n: 0,
        }
    }

    fn push(&mut self, pa: Paddr) {
        if self.n == SLOT_PAGES {
            kpanic(format_args!("buf_map: more pages than a slot holds"));
        }
        self.f[self.n] = pa;
        self.n += 1;
    }
}

impl core::ops::Deref for Frames {
    type Target = [Paddr];

    fn deref(&self) -> &[Paddr] {
        &self.f[..self.n]
    }
}

/// `buf_needva`: threads sleeping for a slot (its address is the channel).
pub(crate) static BUF_NEEDVA: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(0);

/// `buf_mem_init(size)`: reserves the arena of `size` bytes in `kernel_map` and counts its
/// slots (`kvaslots`).
pub(crate) fn buf_mem_init(g: &mut Cache, size: usize) {
    let arena = BufArena::reserve(size);
    // Contiguous mapping
    let slots = arena.slots() as i64;
    BCSTATS.kvaslots.store(slots, Ordering::Relaxed);
    BCSTATS.kvaslots_avail.store(slots, Ordering::Relaxed);
    g.arena = Some(arena);
}

/// `buf_acquire`: sets `B_BUSY` on the cache buffer `r` names and gives its token, mapped in
/// the kvm (sleeping for a slot, the cache's lock left meanwhile, when none is free).
pub(crate) fn buf_acquire(
    g: &mut Cache,
    r: hal::owned::SecondaryRef<super::buf::ByBufhead>,
) -> BufRef {
    let Some(bp) = r.get() else {
        kpanic(format_args!("buf_acquire: no buffer"));
    };
    kassert!(!bp.isset(B_BUSY));
    // Busy before waiting for kvm.
    bp.set(B_BUSY);
    let mut t = BufRef::acquire(g, r);
    buf_map(g, &mut t);
    t
}

/// `buf_acquire_nomap`: sets `B_BUSY` and gives the token, keeping any mapping the buffer
/// had (off `buf_valist`).
pub(crate) fn buf_acquire_nomap(
    g: &mut Cache,
    r: hal::owned::SecondaryRef<super::buf::ByBufhead>,
) -> BufRef {
    let Some(bp) = r.get() else {
        kpanic(format_args!("buf_acquire_nomap: no buffer"));
    };
    bp.set(B_BUSY);
    let t = BufRef::acquire(g, r);
    if t.is_mapped() {
        drop(g.valist.remove(t.buf()));
        BCSTATS.kvaslots_avail.fetch_sub(1, Ordering::Relaxed);
        BCSTATS.busymapped.fetch_add(1, Ordering::Relaxed);
    }
    t
}

/// `buf_map(bp)`: gives a busy buffer a mapping: a fresh slot of the arena, or the slot of the
/// least recently used idle buffer, which loses its own (`buf_unmap`); sleeps on
/// `buf_needva` (the cache's lock left) while none may be stolen.
pub(crate) fn buf_map(g: &mut Cache, t: &mut BufRef) {
    if t.is_mapped() {
        drop(g.valist.remove(t.buf()));
        BCSTATS.kvaslots_avail.fetch_sub(1, Ordering::Relaxed);
    } else {
        // First, just use the pre-allocated space until we run out.
        let slot = if let Some(slot) = g.arena.as_mut().and_then(BufArena::take) {
            BCSTATS.kvaslots_avail.fetch_sub(1, Ordering::Relaxed);
            slot
        } else {
            // Find some buffer we can steal the space from.
            loop {
                let cur = curproc_word();
                let reserved = cur != syncer_word()
                    && cur != cleaner_word()
                    && BCSTATS.kvaslots_avail.load(Ordering::Relaxed) <= RESERVE_SLOTS;
                if !reserved && !g.valist.is_empty() {
                    break;
                }
                BUF_NEEDVA.fetch_add(1, Ordering::Relaxed);
                let _ = g.sleep_nsec(&raw const BUF_NEEDVA, PRIBIO, "buf_needva", INFSLP);
            }
            buf_unmap(g)
        };
        let frames = pages_of(g, t.buf());
        t.set_view(slot.map(&frames));
    }
    BCSTATS.busymapped.fetch_add(1, Ordering::Relaxed);
}

/// The frames of `bp`'s pages, `b_bufsize` bytes of its `b_pobj` from `b_poffs` (`buf_map`'s
/// `uvm_pagelookup` loop); `panic(9)` for a page that is gone (the C's `KASSERT`).
fn pages_of(g: &Cache, bp: &Buf) -> Frames {
    let n = atop(bp.bufsize() as usize);
    let off = bp.poffs();
    bp.with_pobj(g, |obj| {
        let Some(obj) = obj else {
            kpanic(format_args!("buf_map: buffer {bp:p} has no pages"));
        };
        let mut frames = Frames::new();
        for i in 0..n {
            let Some(pg) = uvm_pagelookup(obj, off + ptoa(i) as Voff) else {
                kpanic(format_args!("buf_map: page {i} of {bp:p} is gone"));
            };
            frames.push(vm_page_to_phys(pg));
        }
        frames
    })
}

/// `buf_release`: clears `B_BUSY` and lets the buffer's mapping be stolen (on the tail of
/// `buf_valist`, the view kept by the idle buffer); the token is consumed.
pub(crate) fn buf_release(g: &mut Cache, t: BufRef) {
    let bp = t.buf();
    kassert!(bp.isset(B_BUSY));
    if t.is_mapped() {
        BCSTATS.busymapped.fetch_sub(1, Ordering::Relaxed);
        g.valist.push_tail(Member::counted(bp));
        BCSTATS.kvaslots_avail.fetch_add(1, Ordering::Relaxed);
        if BUF_NEEDVA.load(Ordering::Relaxed) != 0 {
            BUF_NEEDVA.store(0, Ordering::Relaxed);
            wakeup(&raw const BUF_NEEDVA);
        }
    }
    bp.clr(B_BUSY);
    t.release(g);
}

/// Deallocates all memory resources of the buffer `t` holds (`buf_dealloc_mem`). The kvm is
/// not dropped (there is no way to reclaim it): a mapped buffer keeps its slot, idle, at the
/// head of `buf_valist` (`B_RELEASED`), its token consumed and the buffer freed later by
/// `buf_unmap`: `None`. An unmapped one gives its token back for the caller to free: `Some`.
/// The token may be a busy one (`B_BUSY`, from `brelse`) or one taken of an idle cache buffer
/// (`discard_buffer`), whose mapping was on `buf_valist`.
pub(crate) fn buf_dealloc_mem(g: &mut Cache, mut t: BufRef) -> Option<BufRef> {
    let busy = t.buf().isset(B_BUSY);
    let slot = t.take_view().map(|v| {
        if busy {
            BCSTATS.busymapped.fetch_sub(1, Ordering::Relaxed);
        }
        unmap(g, v)
    });

    if t.buf().has_pages(g) {
        buf_free_pages(g, t.buf());
    }

    let Some(slot) = slot else {
        return Some(t);
    };

    let bp = t.buf();
    if busy {
        bp.clr(B_BUSY);
        if BUF_NEEDVA.load(Ordering::Relaxed) != 0 {
            BUF_NEEDVA.store(0, Ordering::Relaxed);
            wakeup(&raw const BUF_NEEDVA);
        }
    } else {
        // XXX - need better test
        drop(g.valist.remove(bp));
        BCSTATS.kvaslots_avail.fetch_sub(1, Ordering::Relaxed);
    }
    bp.set(B_RELEASED);
    let m = Member::counted(bp);
    t.release_bare(g, slot);
    g.valist.push_head(m);
    BCSTATS.kvaslots_avail.fetch_add(1, Ordering::Relaxed);
    None
}

/// The view `v` of a slot of the cache's arena taken back to its slot (`pmap_kremove`).
fn unmap(g: &mut Cache, v: hal::mapped::MappedBytes<'static>) -> BufSlot {
    match &g.arena {
        Some(a) => a.unmap(v),
        None => kpanic(format_args!("buf_unmap: no buffer arena")),
    }
}

/// `buf_fix_mapping(bp, newsize)` (only `bread_cluster_callback`): the mapping of the busy
/// buffer `t` shortened to `newsize` bytes; the rest of its pages go to the other buffers of
/// the cluster.
pub(crate) fn buf_fix_mapping(t: &mut BufRef, newsize: usize) {
    if newsize < t.buf().bufsize() as usize {
        if let Some(v) = t.view_mut() {
            v.truncate(newsize);
        }
        // Note: the size we lost is actually with the other buffers read in by
        // bread_cluster
        t.buf().set_bufsize(newsize as i64);
    }
}

/// `buf_unmap()`: takes the least recently used idle buffer's kva off `buf_valist` and gives
/// its slot (for internal use only); a `B_RELEASED` buffer, which kept only the slot, is freed
/// (`pool_put`).
pub(crate) fn buf_unmap(g: &mut Cache) -> BufSlot {
    let Some(m) = g.valist.pop_head() else {
        kpanic(format_args!("buf_unmap: buf_valist is empty"));
    };
    BCSTATS.kvaslots_avail.fetch_sub(1, Ordering::Relaxed);
    let bp = m.get();
    kassert!(!bp.isset(B_BUSY));
    let slot = match bp.take_idle_kva(g) {
        Kva::Mapped(v) => unmap(g, v),
        Kva::Bare(slot) => slot,
        Kva::None => kpanic(format_args!("buf_unmap: buffer {bp:p} has no kva")),
    };
    if bp.isset(B_RELEASED) {
        match m.into_ref() {
            // pool_put(&bufpool, bp)
            Ok(r) => drop(g.bufhead.take(r)),
            Err(_) => kpanic(format_args!(
                "buf_unmap: a released buffer is not the cache's"
            )),
        }
    }
    slot
}

/// `buf_alloc_pages(bp, size)`: `size` bytes (a whole number of pages) of DMA-reachable
/// pages for the busy buffer `t`, in its own object; throws clean buffers away
/// (`bufbackoff`) while they are short, then waits for them, and panics when even that fails
/// (the C's "should not happen").
pub(crate) fn buf_alloc_pages(g: &mut Cache, t: &BufRef, size: usize) {
    let bp = t.buf();
    kassert!(size == round_page(size));
    kassert!(!bp.has_pages(g));
    kassert!(!t.is_mapped());

    bp.uobj().reinit_dummy(1); // uvm_obj_init(&bp->b_uobj, &bufcache_pager, 1)

    // Attempt to allocate with NOWAIT. if we can't, then throw away some clean pages and try
    // again. Finally, if that fails, do a WAITOK allocation so the page daemon can find memory
    // for us.
    let npages = atop(size) as i64;
    let mut r;
    loop {
        r = uvm_pagealloc_multi(bp.uobj(), 0, size, UVM_PLA_NOWAIT | UVM_PLA_NOWAKE);
        if r.is_ok() {
            break;
        }
        if bufbackoff_locked(g, npages) < npages as u64 {
            break;
        }
    }
    if r.is_err() {
        r = uvm_pagealloc_multi(bp.uobj(), 0, size, UVM_PLA_WAITOK);
    }
    // should not happen
    if r.is_err() {
        kpanic(format_args!(
            "uvm_pagealloc_multi unable to allocate an buf_object of size {size}"
        ));
    }

    BCSTATS.numbufpages.fetch_add(npages, Ordering::Relaxed);
    bp.set_pages(g, Pages::Own);
    bp.set_poffs(0);
    bp.set_bufsize(size as i64);
}

/// `buf_free_pages(bp)`: frees the buffer's pages (it has no mapping): unwired, then its object
/// emptied (`uvm_obj_free`).
pub(crate) fn buf_free_pages(g: &mut Cache, bp: &Buf) {
    kassert!(bp.data_addr() == 0);

    let off = bp.poffs();
    let n = atop(bp.bufsize() as usize);
    bp.with_pobj(g, |uobj| {
        let Some(uobj) = uobj else {
            kpanic(format_args!("buf_free_pages: buffer {bp:p} has no pages"));
        };
        for i in 0..n {
            let Some(pg) = uvm_pagelookup(uobj, off + ptoa(i) as Voff) else {
                kpanic(format_args!("buf_free_pages: page {i} of {bp:p} is gone"));
            };
            kassert!(pg.wire_count() == 1);
            pg.set_wire_count(0);
            BCSTATS.numbufpages.fetch_sub(1, Ordering::Relaxed);
        }
        // XXX refactor to do this without splbio later
        uvm_obj_free(uobj);
    });
    bp.set_poffs(0);
    bp.set_pages(g, Pages::None);
}

const _: () = assert!(size_of::<Paddr>() == size_of::<usize>());
/* </CODE> */
