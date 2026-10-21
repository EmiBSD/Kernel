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

/*-
 * Copyright (c) 1996, 1998 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jason R. Thorpe and Matthias Drochner.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/*-
 * Copyright (c) 1996 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jason R. Thorpe.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `extent(9)`: the general purpose extent manager.
//!
//! Upstream: sys/kern/subr_extent.c @ 3ce1f3f79392
//! Upstream: sys/sys/extent.h @ 3ce1f3f79392
//! LZ: sys/kern/subr_extent.rs@3c62ede68802
//! LZ: sys/sys/extent.rs@3c62ede68802
//!
//! An extent is a range of unsigned numbers (bus numbers, I/O ports, memory or I/O virtual
//! addresses) and the regions allocated in it, kept sorted and, unless the extent was created
//! `EX_NOCOALESCE`, merged with their neighbours. A region is allocated at a given place
//! ([`Extent::alloc_region`]) or wherever it fits in a subrange, aligned (with a skew) and
//! without crossing a boundary ([`Extent::alloc_subregion`]: the smallest gap that holds it, or
//! the first with `EX_FAST`); [`Extent::free`] gives a range back, splitting a region when
//! needed. A filled extent (`EX_FILLED`) starts with everything allocated, so what is freed is
//! what is available (the PCI host bridges' windows). A region's descriptor comes from the pool
//! `extentpl`, or from a fixed extent's own storage ([`ExtentFixed`]), or from the caller, made
//! in advance ([`descr`]: the `*_with_descr` functions, for an IOMMU's map).
//!
//! The list of every extent and [`extent_print_all`] are kept (`DIAGNOSTIC || DDB`; GENERIC has
//! `option DDB`). The argument checks that `panic(9)` are under feature `diagnostic`; without
//! it a region outside the extent is `EINVAL`, as in the C.
//!
//! ## Deviations
//! - Errors are `Err(Errno)` and an allocated start the `Ok` value (the C's `int` and
//!   `u_long *result`); the C's checks of NULL pointers and of "storage provided for
//!   non-fixed" have no counterpart: the types rule them out.
//! - A fixed extent's storage ([`ExtentFixed`]) holds the extent itself and its descriptors,
//!   counted in descriptors (`EXTENT_FIXED_STORAGE_SIZE`'s role), and serves one extent for the
//!   life of the kernel: the C's `memset` and reuse of a storage after `extent_destroy` is not
//!   kept (no caller destroys a fixed extent).
//! - A descriptor the extent no longer needs on a path where the C forgets it (an
//!   `EX_CONFLICTOK` allocation that the extent already covers, a sleep interrupted by a
//!   signal) goes back where it came from; the C leaks it there.
//! - ddb's `show extents` reports an extent whose lock is held (`<locked>`, or the list's)
//!   instead of taking it: a panic may strike under it, and the C has no lock.
//! - A caller's descriptor (`ER_DISCARD`) goes back to `extentpl` when its region is freed,
//!   where the C leaves it to the structure that embeds it: the caller made it from the pool
//!   ([`descr`]) and does not get it back.
//!
//! ## Redesign
//! - LZ's `struct extent`, `struct extent_region` and `struct extent_fixed` were `#[repr(C)]`
//!   records of `Cell`s linked by `queue.h` lists in `unsafe` blocks, the descriptors pool items
//!   and storage bytes handed out as `&'static`, a fixed extent found by casting the extent
//!   back to its storage, `extent_destroy` and the `*_with_descr` functions `unsafe fn`s with
//!   the caller's promises. Now the regions are a `hal::owned::OwnedTree` ordered by start,
//!   then end, then the descriptor's address, which owns each descriptor as its `PoolBox` and
//!   gives it back when the region goes (a freed descriptor cannot stay linked), and a fixed
//!   extent's freelist an `OwnedList` of the same boxes. The C's walks of the sorted list keep
//!   their order, two regions with one start included (the `EX_CONFLICTOK` case-1 quirk's
//!   `[s, s - 1]` before `[s, e]`, where the C's `LIST_INSERT_AFTER(last)` puts it; equal
//!   regions, which no walk tells apart, by address where the C puts the newest first); a
//!   region is inserted by its key where the C inserts after `last`, and a coalesced or freed
//!   region's start or end moves in place without changing the order. The walks act on the
//!   region in hand (its fields are atomics written under the lock): the coalescing of
//!   `extent_insert_and_optimize` on the region the walk held and its successor, the cases of
//!   `extent_free` on the region found; a removal or an insertion follows once the walk's
//!   borrow has ended, by the region's exact key.
//! - The extent's mutable state (its regions, the freelist, `EXF_WANTED` and `EXF_FLWANTED`)
//!   is inside a `hal::mutex::Mutexed` at `IPL_VM`, the descriptors' pool's level; the C has no
//!   lock (its callers serialise; the IOMMUs take a mutex around it). `EXF_FIXED` and
//!   `EXF_NOCOALESCE` are set at creation and read without it. The C's sleeps (`EX_WAITSPACE`,
//!   a fixed extent's empty freelist) sleep on that mutex, on the same channels and with the
//!   same priority and wait message (`extnt`); a descriptor from the pool is made before the
//!   lock is taken (`PR_WAITOK` may sleep).
//! - A non-fixed extent is a `hal::malloc::MBox` of the caller's `mtype` (the C's `malloc`,
//!   the same `vmstat -m` row), whose drop is `extent_destroy`; a fixed one lives in its
//!   [`ExtentFixed`] (a caller's `static`). Every extent is linked, pinned, on the list of all
//!   extents (`hal::pinlist`), and leaves it when destroyed.
//! - A caller's descriptor is a [`Descr`], a box of `extentpl` made in advance and given to the
//!   extent, which owns it from then on (the C's `struct extent_region` embedded in the
//!   caller's structure, linked raw).

use core::fmt;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering as Mem};

use hal::intr::{IPL_NONE, IPL_VM};
use hal::malloc::{M_NOWAIT, M_WAITOK, MBox};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::owned::{OwnedEntry, OwnedList, OwnedTree};
use hal::pin::pin_static;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{BoxPool, PR_NOWAIT, PR_WAITOK, PoolBox, StaticArena};
use hal::rwlock::Rwlocked;
use hal::sleep::{PCATCH, PRIBIO, wakeup};

use crate::assert::kpanic;
use crate::prf::printf;
use crate::sys::errno::Errno;

/// `ER_ALLOC`: region descriptor dynamically allocated.
pub const ER_ALLOC: i32 = 0x01;
/// `ER_DISCARD`: discard region descriptor after use (the caller's).
pub const ER_DISCARD: i32 = 0x02;

/// `EXF_FIXED`: extent uses fixed storage.
pub const EXF_FIXED: i32 = 0x01;
/// `EXF_NOCOALESCE`: coalescing of regions not allowed.
pub const EXF_NOCOALESCE: i32 = 0x02;
/// `EXF_WANTED`: someone asleep on extent.
pub const EXF_WANTED: i32 = 0x04;
/// `EXF_FLWANTED`: someone asleep on freelist.
pub const EXF_FLWANTED: i32 = 0x08;

/// `EXF_BITS`: the `%b` description of `ex_flags` (base 16, then each bit's number from 1 and
/// its name).
pub const EXF_BITS: &[u8] = b"\x10\x04FLWANTED\x03WANTED\x02NOCOALESCE\x01FIXED";

/// `EX_NOWAIT`: not safe to sleep.
pub const EX_NOWAIT: i32 = 0x0000;
/// `EX_WAITOK`: safe to sleep.
pub const EX_WAITOK: i32 = 0x0001;
/// `EX_FAST`: take first fit in `extent_alloc()`.
pub const EX_FAST: i32 = 0x0002;
/// `EX_CATCH`: catch signals while sleeping.
pub const EX_CATCH: i32 = 0x0004;
/// `EX_NOCOALESCE`: create a non-coalescing extent.
pub const EX_NOCOALESCE: i32 = 0x0008;
/// `EX_MALLOCOK`: safe to call `malloc()`.
pub const EX_MALLOCOK: i32 = 0x0010;
/// `EX_WAITSPACE`: wait for space to become free.
pub const EX_WAITSPACE: i32 = 0x0020;
/// `EX_BOUNDZERO`: boundary lines start at 0.
pub const EX_BOUNDZERO: i32 = 0x0040;
/// `EX_CONFLICTOK`: allow conflicts.
pub const EX_CONFLICTOK: i32 = 0x0080;
/// `EX_FILLED`: create a filled extent.
pub const EX_FILLED: i32 = 0x0100;

/// `EX_NOALIGN`: don't do alignment (the "alignment" argument's place holder).
pub const EX_NOALIGN: u64 = 1;
/// `EX_NOBOUNDARY`: don't do boundary checking (the "boundary" argument's place holder).
pub const EX_NOBOUNDARY: u64 = 0;

/// `struct extent_region`: one allocated region `[er_start, er_end]`, or a free descriptor of
/// a fixed extent. Its fields are written under its extent's lock.
pub struct ExtentRegion {
    /// `er_link`: the extent's region tree or a fixed extent's freelist.
    er_link: OwnedEntry<ExtentRegion>,
    /// `er_start`: start of region.
    er_start: AtomicU64,
    /// `er_end`: end of region (inclusive).
    er_end: AtomicU64,
    /// `er_flags`: `ER_*`.
    er_flags: AtomicI32,
}

impl ExtentRegion {
    /// A descriptor in no extent, covering nothing, with `flags`.
    pub const fn new(flags: i32) -> Self {
        Self {
            er_link: OwnedEntry::new(),
            er_start: AtomicU64::new(0),
            er_end: AtomicU64::new(0),
            er_flags: AtomicI32::new(flags),
        }
    }

    /// `er_start`.
    pub fn start(&self) -> u64 {
        self.er_start.load(Mem::Relaxed)
    }

    /// `er_end`.
    pub fn end(&self) -> u64 {
        self.er_end.load(Mem::Relaxed)
    }

    /// `er_flags`.
    pub fn flags(&self) -> i32 {
        self.er_flags.load(Mem::Relaxed)
    }

    fn set_start(&self, v: u64) {
        self.er_start.store(v, Mem::Relaxed);
    }

    fn set_end(&self, v: u64) {
        self.er_end.store(v, Mem::Relaxed);
    }

    fn set_flags(&self, v: i32) {
        self.er_flags.store(v, Mem::Relaxed);
    }
}

/// A region's place in the region tree: its start, then its end, then its descriptor's
/// address. Two regions may share a start (the `EX_CONFLICTOK` case-1 quirk inserts `[s, s -
/// 1]` before a region that starts at `s`, in an `EX_NOCOALESCE` extent, as the C's list
/// admits); the address tells apart two that share both, which no walk can tell apart.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RKey {
    start: u64,
    end: u64,
    at: usize,
}

impl RKey {
    /// `r`'s key now (its start and end move in place: see [`ExtentRegionList`]).
    fn of(r: &ExtentRegion) -> Self {
        Self {
            start: r.start(),
            end: r.end(),
            at: ptr::from_ref(r).addr(),
        }
    }
}

hal::owned_adapter!(
    /// `LIST_HEAD(, extent_region)`: the regions by ([`RKey`]) start, end and address, and the
    /// freelist. A start or end moved in place keeps the order: a coalesced or freed region
    /// moves within the gap its neighbours leave, and where two regions share a start (only in
    /// an `EX_NOCOALESCE` extent, which moves nothing in place) the quirk's `[s, s - 1]` sorts
    /// before the region `[s, e]`, `e >= s`, as the C's list has them.
    pub ExtentRegionList: ExtentRegion, er_link, |a, b| RKey::of(a).cmp(&RKey::of(b))
);

/// A caller's region descriptor, made in advance ([`descr`]) and given to the extent by the
/// `*_with_descr` allocations: a box of `extentpl`.
pub type Descr = PoolBox<ExtentRegion>;

/// What an extent's lock keeps.
struct ExState {
    /// `ex_regions`: allocated regions in extent, by start, then end, then address
    /// ([`RKey`]).
    regions: OwnedTree<ExtentRegionList>,
    /// `fex_freelist`: a fixed extent's free descriptors.
    freelist: OwnedList<ExtentRegionList>,
    /// `EXF_WANTED` and `EXF_FLWANTED` of `ex_flags`.
    flags: i32,
}

/// `struct extent`.
pub struct Extent {
    /// `ex_name`: name of extent.
    pub ex_name: &'static [u8],
    /// `ex_start`: start of extent.
    pub ex_start: u64,
    /// `ex_end`: end of extent (inclusive).
    pub ex_end: u64,
    /// `ex_mtype`: memory type (`malloc(9)`).
    pub ex_mtype: i32,
    /// `EXF_FIXED` of `ex_flags` (set at creation).
    fixed: bool,
    /// `EXF_NOCOALESCE` of `ex_flags` (set at creation).
    nocoalesce: bool,
    /// The regions, the freelist and the waiters' flags.
    st: Mutexed<ExState>,
    /// The freelist's wait channel (`&fex->fex_freelist`): its address is distinct from the
    /// extent's own (`ex`, the `EX_WAITSPACE` channel) only because this byte is not the
    /// struct's first field; a compile-time check below keeps it so.
    fl_chan: u8,
    /// `ex_link`: the list of all extents.
    ex_link: PinEntry<Extent>,
}

hal::pin_adapter!(
    /// `LIST_HEAD(listhead, extent)`: through `ex_link`.
    pub ExtentList: Extent, ex_link
);

/// A non-fixed extent: its `malloc(9)` block of the caller's `mtype`; dropping it is
/// `extent_destroy`.
pub type ExtentRef = MBox<Extent>;

impl Extent {
    /// `ex_flags`, for a holder of the lock.
    fn flags_of(&self, st: &ExState) -> i32 {
        st.flags
            | if self.fixed { EXF_FIXED } else { 0 }
            | if self.nocoalesce { EXF_NOCOALESCE } else { 0 }
    }

    /// `extent_create(name, start, end, mtype, NULL, 0, flags)`: an extent over `[start,
    /// end]`, in a `malloc(9)` block of `mtype` (`EX_WAITOK`: `M_WAITOK`). `None` when memory
    /// is short.
    pub fn create(
        name: &'static [u8],
        start: u64,
        end: u64,
        mtype: i32,
        flags: i32,
    ) -> Option<ExtentRef> {
        create_check(name, start, end);
        extent_pool_init();
        let mflags = if flags & EX_WAITOK != 0 {
            M_WAITOK
        } else {
            M_NOWAIT
        };
        let ex = MBox::new(
            mtype,
            mflags,
            extent_new(name, start, end, mtype, false, flags & EX_NOCOALESCE != 0),
        )?;
        if flags & EX_FILLED != 0 {
            // A descriptor that cannot be had frees the extent (its drop: it is not
            // registered yet).
            let rp = extent_alloc_region_descriptor(&ex, flags)?;
            fill(&ex, rp);
        }
        extent_register(ex.as_pin());
        Some(ex)
    }

    /// `extent_destroy(ex)` of an extent that is not dropped (a fixed one, or one the legacy
    /// callers hold as `&'static`): its regions freed and the extent off the list of all
    /// extents; it is empty and unregistered afterwards.
    pub fn destroy(&self) {
        let mut g = self.st.lock();
        while let Some(k) = g.regions.min().map(RKey::of) {
            let Some(rp) = take(&mut g, k) else {
                break;
            };
            extent_free_region_descriptor(self, &mut g, rp);
        }
        drop(g);
        extent_unregister(self);
    }

    /// `extent_alloc_region(ex, start, size, flags)`: allocates `[start, start + size - 1]`.
    pub fn alloc_region(&self, start: u64, size: u64, flags: i32) -> Result<(), Errno> {
        // Allocate the region descriptor. It will be freed later if we can coalesce with
        // another region.
        let Some(rp) = extent_alloc_region_descriptor(self, flags) else {
            if cfg!(feature = "diagnostic") {
                printf(format_args!(
                    "extent_alloc_region: can't allocate region descriptor\n"
                ));
            }
            return Err(Errno::ENOMEM);
        };
        extent_do_alloc_region(self, &mut self.st.lock(), start, size, flags, rp)
    }

    /// `extent_alloc_region_with_descr(ex, start, size, flags, rp)`: as
    /// [`Extent::alloc_region`], with the caller's descriptor, in an `EX_NOCOALESCE` extent.
    pub fn alloc_region_with(
        &self,
        start: u64,
        size: u64,
        flags: i32,
        rp: Descr,
    ) -> Result<(), Errno> {
        self.check_nocoalesce("extent_alloc_region_with_descr");
        rp.set_flags(ER_DISCARD);
        extent_do_alloc_region(self, &mut self.st.lock(), start, size, flags, rp)
    }

    /// `extent_alloc_subregion(ex, substart, subend, size, alignment, skew, boundary, flags,
    /// result)`: `size` in `[substart, subend]`, aligned to `alignment` (a power of 2) after
    /// `skew`, not crossing a multiple of `boundary` (`EX_NOBOUNDARY`); the smallest gap that
    /// holds it, or the first with `EX_FAST`. The start.
    #[allow(clippy::too_many_arguments, reason = "the C's signature")]
    pub fn alloc_subregion(
        &self,
        substart: u64,
        subend: u64,
        size: u64,
        alignment: u64,
        skew: u64,
        boundary: u64,
        flags: i32,
    ) -> Result<u64, Errno> {
        let Some(rp) = extent_alloc_region_descriptor(self, flags) else {
            if cfg!(feature = "diagnostic") {
                printf(format_args!(
                    "extent_alloc_subregion: can't allocate region descriptor\n"
                ));
            }
            return Err(Errno::ENOMEM);
        };
        let req = Req {
            substart,
            subend,
            size,
            alignment,
            skew,
            boundary,
            flags,
        };
        extent_do_alloc(self, &mut self.st.lock(), &req, rp)
    }

    /// `extent_alloc_subregion_with_descr(...)`: as [`Extent::alloc_subregion`], with the
    /// caller's descriptor, in an `EX_NOCOALESCE` extent.
    #[allow(clippy::too_many_arguments, reason = "the C's signature")]
    pub fn alloc_subregion_with(
        &self,
        substart: u64,
        subend: u64,
        size: u64,
        alignment: u64,
        skew: u64,
        boundary: u64,
        flags: i32,
        rp: Descr,
    ) -> Result<u64, Errno> {
        self.check_nocoalesce("extent_alloc_subregion_with_descr");
        rp.set_flags(ER_DISCARD);
        let req = Req {
            substart,
            subend,
            size,
            alignment,
            skew,
            boundary,
            flags,
        };
        extent_do_alloc(self, &mut self.st.lock(), &req, rp)
    }

    /// `extent_alloc(ex, size, alignment, skew, boundary, flags, result)`: the simple case of
    /// [`Extent::alloc_subregion`], over the whole extent.
    pub fn alloc(
        &self,
        size: u64,
        alignment: u64,
        skew: u64,
        boundary: u64,
        flags: i32,
    ) -> Result<u64, Errno> {
        self.alloc_subregion(
            self.ex_start,
            self.ex_end,
            size,
            alignment,
            skew,
            boundary,
            flags,
        )
    }

    /// `extent_alloc_with_descr(...)`: the simple case of [`Extent::alloc_subregion_with`].
    pub fn alloc_with(
        &self,
        size: u64,
        alignment: u64,
        skew: u64,
        boundary: u64,
        flags: i32,
        rp: Descr,
    ) -> Result<u64, Errno> {
        self.alloc_subregion_with(
            self.ex_start,
            self.ex_end,
            size,
            alignment,
            skew,
            boundary,
            flags,
            rp,
        )
    }

    /// `extent_free(ex, start, size, flags)`: gives `[start, start + size - 1]` back. Panics
    /// when the range is not allocated, unless `EX_CONFLICTOK` allows partial overlaps.
    pub fn free(&self, start: u64, size: u64, flags: i32) -> Result<(), Errno> {
        extent_free(self, start, size, flags)
    }

    /// `extent_print(ex)`.
    pub fn print(&self) {
        extent_print1(self, printf);
    }

    /// The `DIAGNOSTIC` check of the `*_with_descr` functions.
    fn check_nocoalesce(&self, what: &str) {
        if cfg!(feature = "diagnostic") && !self.nocoalesce {
            kpanic(format_args!("{what}: EX_NOCOALESCE not set"));
        }
    }
}

impl Drop for Extent {
    /// `extent_destroy`'s list removal (the regions are freed by the tree's drop).
    fn drop(&mut self) {
        extent_unregister(self);
    }
}

/// `struct extent_fixed`: a fixed extent's storage, a caller's `static` that lives for ever:
/// the extent and `N` region descriptors (`EXTENT_FIXED_STORAGE_SIZE(N)`'s room).
pub struct ExtentFixed<const N: usize> {
    /// `fex_extent`.
    fex_extent: OnceStatic<Extent>,
    /// `fex_storage`: the descriptors' room.
    fex_storage: StaticArena<ExtentRegion, N>,
    /// The pool over `fex_storage` the descriptors are boxes of.
    pool: BoxPool<ExtentRegion>,
}

impl<const N: usize> ExtentFixed<N> {
    /// Storage for one fixed extent of `N` descriptors.
    pub const fn new() -> Self {
        Self {
            fex_extent: OnceStatic::new(),
            fex_storage: StaticArena::new(),
            pool: BoxPool::new(),
        }
    }
}

impl<const N: usize> Default for ExtentFixed<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// A fixed extent's storage, whatever its size (`extent_create`'s `storage` argument).
pub trait FixedRegions: Sync {
    /// `extent_create(name, start, end, mtype, storage, storagesize, flags)` in this storage:
    /// the extent, every descriptor on its freelist. `None` when `EX_FILLED`'s descriptor
    /// cannot be had; `panic(9)` when the storage served an extent already.
    fn create(
        &'static self,
        name: &'static [u8],
        start: u64,
        end: u64,
        mtype: i32,
        flags: i32,
    ) -> Option<&'static Extent>;
}

impl<const N: usize> FixedRegions for ExtentFixed<N> {
    fn create(
        &'static self,
        name: &'static [u8],
        start: u64,
        end: u64,
        mtype: i32,
        flags: i32,
    ) -> Option<&'static Extent> {
        create_check(name, start, end);
        extent_pool_init();
        let fresh = extent_new(name, start, end, mtype, true, flags & EX_NOCOALESCE != 0);
        if self.pool.init_arena(&self.fex_storage).is_err() || self.fex_extent.set(fresh).is_err() {
            kpanic(format_args!(
                "extent_create: the fixed storage of `{}' served an extent already",
                Name(name)
            ));
        }
        let ex = self.fex_extent.get()?;
        {
            // In a fixed extent, we have to pre-allocate region descriptors and place them in
            // the extent's freelist.
            let mut g = ex.st.lock();
            while let Some(rp) = self.pool.get(PR_NOWAIT, || ExtentRegion::new(0)) {
                g.freelist.push(rp);
            }
        }
        if flags & EX_FILLED != 0 {
            let rp = extent_alloc_region_descriptor(ex, flags)?;
            fill(ex, rp);
        }
        extent_register(pin_static(ex));
        Some(ex)
    }
}

/// `ext_list`: every extent, for `extent_print_all`.
static EXT_LIST: PinList<ExtentList> = PinList::new();
/// The list's lock (the C's list is unlocked: autoconfiguration serialises).
static EXT_LOCK: Mutexed<()> = Mutexed::new(IPL_NONE, ());

/// `ex_region_pl`: the pool `extentpl` of dynamically allocated region descriptors.
static EX_REGION_PL: BoxPool<ExtentRegion> = BoxPool::new();
/// `extent_pool_init`'s `inited`.
static EX_REGION_PL_INITED: AtomicBool = AtomicBool::new(false);
/// Serialises the making of [`EX_REGION_PL`], once (`pool_init` may sleep).
static EX_REGION_PL_INIT: Rwlocked<()> = Rwlocked::new("extentpl", ());

/// What `extent_do_alloc` is asked for.
struct Req {
    substart: u64,
    subend: u64,
    size: u64,
    alignment: u64,
    skew: u64,
    boundary: u64,
    flags: i32,
}

/// The end of `extent_do_alloc`'s search.
enum Fit<'r> {
    /// `found`: insert at this start, after this region (`None`: first).
    Found(u64, Option<&'r ExtentRegion>),
    /// `fail`: no exact (or, with `EX_FAST`, first) fit.
    Fail,
}

/// What `extent_insert_and_optimize`'s coalescing ([`coalesce`]) leaves to the tree.
enum Coalesced {
    /// Merged with a neighbour: the new descriptor is not needed.
    Merged,
    /// Merged with both neighbours, filling the gap: the next region's descriptor goes too.
    MergedTakeNext(RKey),
    /// Not merged: the new descriptor is filled and linked.
    Insert,
}

/// Where `extent_do_alloc_region`'s walk of the regions ended.
enum Placed {
    /// No conflict: the coalescing done on the regions in hand, and what is left.
    Insert(Coalesced),
    /// A conflict, and `EX_WAITSPACE`: sleep and try again.
    Sleep,
    /// `EX_CONFLICTOK`: try again with this range (cases 1 and 3).
    Retry(u64, u64),
    /// `EX_CONFLICTOK`: this region lies inside the request; free it and try again (case 2).
    Drop(RKey),
    /// `EX_CONFLICTOK`: the request lies inside an allocated region (case 4).
    Covered,
    /// A conflict: `EAGAIN`.
    Conflict,
}

/// What `extent_free`'s walk of the regions left to the tree.
enum Freed {
    /// Nothing (done in place, or not found).
    Walked,
    /// Case 1: this region goes.
    Take(RKey),
    /// Case 4: the new region `[end + 1, er_end]` is linked.
    Insert(PoolBox<ExtentRegion>),
    /// Case 1a: this region goes and the walk goes on.
    TakeAndGoOn(RKey),
}

/// An extent's name as the C prints it (`%s`: up to its NUL, if any; each byte as a char, as
/// LZ's `Str`, so a non-ASCII byte prints as its Latin-1 character).
struct Name<'a>(&'a [u8]);

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &c in self.0.iter().take_while(|&&c| c != 0) {
            fmt::Write::write_char(f, char::from(c))?;
        }
        Ok(())
    }
}

/// `ex_flags` as `%b` prints it with [`EXF_BITS`]: hex, then the names of the bits set in
/// angle brackets.
struct ExfBits(i32);

impl fmt::Display for ExfBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)?;
        let mut sep = '<';
        let mut bits = &EXF_BITS[1..];
        while let Some((&bit, rest)) = bits.split_first() {
            let n = rest.iter().take_while(|&&c| c > b' ').count();
            let (name, rest) = rest.split_at(n);
            bits = rest;
            if self.0 & (1 << (bit - 1)) != 0 {
                fmt::Write::write_char(f, sep)?;
                f.write_str(core::str::from_utf8(name).unwrap_or("?"))?;
                sep = ',';
            }
        }
        if sep == ',' {
            fmt::Write::write_char(f, '>')?;
        }
        Ok(())
    }
}

/// `extent_align(start, align, skew)`: shortcut to align to an arbitrary power-of-two
/// boundary.
const fn extent_align(start: u64, align: u64, skew: u64) -> u64 {
    (start.wrapping_sub(skew).wrapping_add(align.wrapping_sub(1)) & align.wrapping_neg())
        .wrapping_add(skew)
}

/// `LE_OV(x, y, z)`: whether `x + y <= z`, failing if the sum overflows.
const fn le_ov(x: u64, y: u64, z: u64) -> bool {
    let sum = x.wrapping_add(y);
    sum >= x && sum <= z
}

/// `extent_register(ex)`: puts the extent on the list of all extents (an extent linked
/// already panics: the C's `DIAGNOSTIC` "already registered").
fn extent_register(ex: Pin<&Extent>) {
    EXT_LIST.insert_head(&mut EXT_LOCK.lock(), ex);
}

/// `extent_destroy`'s `LIST_REMOVE(ex, ex_link)`, once.
fn extent_unregister(ex: &Extent) {
    if ex.ex_link.is_linked() {
        EXT_LIST.remove(&mut EXT_LOCK.lock(), ex);
    }
}

/// `extent_pool_init()`: makes `extentpl` once.
fn extent_pool_init() -> &'static BoxPool<ExtentRegion> {
    if !EX_REGION_PL_INITED.load(Mem::Acquire) {
        let _w = EX_REGION_PL_INIT.write();
        if !EX_REGION_PL_INITED.load(Mem::Relaxed) {
            if EX_REGION_PL.init(IPL_VM, 0, "extentpl").is_err() {
                kpanic(format_args!("extent_pool_init: cannot make extentpl"));
            }
            EX_REGION_PL_INITED.store(true, Mem::Release);
        }
    }
    &EX_REGION_PL
}

/// `extent_print_all()`: every extent registered (ddb's `show extents`), through `pr`.
pub fn extent_print_all(pr: fn(fmt::Arguments<'_>)) {
    // ddb runs after a panic that may have struck under these locks: a held one is reported,
    // never taken (`Mutexed` panics "locking against myself" when this CPU holds it).
    let Some(g) = try_lock_free(&EXT_LOCK) else {
        pr(format_args!("extent list <locked>\n"));
        return;
    };
    for ep in EXT_LIST.iter(&g) {
        match try_lock_free(&ep.st) {
            Some(st) => print_locked(ep, &st, pr),
            None => pr(format_args!("extent `{}' <locked>\n", Name(ep.ex_name))),
        }
    }
}

/// `m`'s guard when no CPU holds it (ddb's path: a lock this CPU holds is not taken again).
fn try_lock_free<T>(m: &Mutexed<T>) -> Option<MutexedGuard<'_, T>> {
    if m.is_locked() {
        return None;
    }
    m.try_lock()
}

/// A new extent over `[start, end]`, with no regions.
fn extent_new(
    name: &'static [u8],
    start: u64,
    end: u64,
    mtype: i32,
    fixed: bool,
    nocoalesce: bool,
) -> Extent {
    Extent {
        ex_name: name,
        ex_start: start,
        ex_end: end,
        ex_mtype: mtype,
        fixed,
        nocoalesce,
        st: Mutexed::new(
            IPL_VM,
            ExState {
                regions: OwnedTree::new(),
                freelist: OwnedList::new(),
                flags: 0,
            },
        ),
        fl_chan: 0,
        ex_link: PinEntry::new(),
    }
}

/// `extent_create`'s `DIAGNOSTIC` argument check.
fn create_check(name: &[u8], start: u64, end: u64) {
    if cfg!(feature = "diagnostic") && end < start {
        printf(format_args!(
            "extent_create: extent `{}', start 0x{start:x}, end 0x{end:x}\n",
            Name(name)
        ));
        kpanic(format_args!("extent_create: end < start"));
    }
}

/// `EX_FILLED`: everything allocated, one region.
fn fill(ex: &Extent, rp: PoolBox<ExtentRegion>) {
    rp.set_start(ex.ex_start);
    rp.set_end(ex.ex_end);
    insert(&mut ex.st.lock(), rp);
}

/// `extent_destroy(ex)` of a [`ExtentRef`] (its drop does it) or of an extent held as
/// `&'static` ([`Extent::destroy`]).
pub fn extent_destroy(ex: ExtentRef) {
    drop(ex);
}

/// The region of key `key` (the one the caller had in hand, under the same lock), taken off
/// the tree.
fn take(g: &mut MutexedGuard<'_, ExState>, key: RKey) -> Option<PoolBox<ExtentRegion>> {
    g.regions.take_by(|r| key.cmp(&RKey::of(r)))
}

/// `rp` linked in the region tree. Keys are unique (the descriptor's address is part of
/// them), so an equal key is the same descriptor linked twice, which its `PoolBox` rules
/// out: never seen.
fn insert(g: &mut MutexedGuard<'_, ExState>, rp: PoolBox<ExtentRegion>) {
    let at = rp.start();
    if g.regions.insert(rp).is_some() {
        kpanic(format_args!("extent: a region at 0x{at:x} linked twice"));
    }
}

/// `extent_insert_and_optimize`'s coalescing, on the regions in hand: `[start, start + size -
/// 1]` goes after `after` (`None`: at the head) and is merged with its neighbours when the
/// extent allows it, their start or end moved in place (one pointer step, as the C's: the
/// region before is the one the caller's walk held, the one after its successor); what is
/// left for the tree, which the caller does once the walk's borrow has ended
/// ([`extent_insert_and_optimize`]).
fn coalesce(
    ex: &Extent,
    regions: &OwnedTree<ExtentRegionList>,
    start: u64,
    size: u64,
    after: Option<&ExtentRegion>,
) -> Coalesced {
    if ex.nocoalesce {
        return Coalesced::Insert;
    }
    let end = start.wrapping_add(size.wrapping_sub(1));
    match after {
        None => {
            // We're the first in the region list. If there's a region after us, attempt to
            // coalesce to save descriptor overhead.
            if let Some(first) = regions.min()
                && start.wrapping_add(size) == first.start()
            {
                // We can coalesce. Prepend us to the first region.
                first.set_start(start);
                return Coalesced::Merged;
            }
        }
        Some(a) => {
            // Attempt to coalesce with the region before us, then with the one after.
            let next = regions.next(a);
            let appended = a.end().wrapping_add(1) == start;
            if appended {
                // We can coalesce. Append ourselves and make note of it.
                a.set_end(end);
            }
            if let Some(n) = next
                && start.wrapping_add(size) == n.start()
            {
                if appended {
                    // We exactly fit the gap: the "next" region's descriptor goes.
                    a.set_end(n.end());
                    return Coalesced::MergedTakeNext(RKey::of(n));
                }
                // Just prepend us to the next region.
                n.set_start(start);
                return Coalesced::Merged;
            }
            // We don't need a region descriptor if we appended ourselves to the previous
            // region.
            if appended {
                return Coalesced::Merged;
            }
        }
    }
    Coalesced::Insert
}

/// `extent_insert_and_optimize(ex, start, size, after, rp)`, the tree's part once
/// [`coalesce`] has merged what it could: `rp` freed when it is not needed, or filled with
/// `[start, start + size - 1]` and linked (`LIST_INSERT_AFTER(after)`: the key puts it where
/// the C's list has it).
fn extent_insert_and_optimize(
    ex: &Extent,
    g: &mut MutexedGuard<'_, ExState>,
    start: u64,
    size: u64,
    c: Coalesced,
    rp: PoolBox<ExtentRegion>,
) {
    match c {
        Coalesced::Merged => extent_free_region_descriptor(ex, g, rp),
        Coalesced::MergedTakeNext(next) => {
            if let Some(nextr) = take(g, next) {
                extent_free_region_descriptor(ex, g, nextr);
            }
            extent_free_region_descriptor(ex, g, rp);
        }
        Coalesced::Insert => {
            // cant_coalesce: fill in the region descriptor and insert ourselves.
            rp.set_start(start);
            rp.set_end(start.wrapping_add(size.wrapping_sub(1)));
            insert(g, rp);
        }
    }
}

/// The sleep `EX_WAITSPACE` (and a fixed extent's empty freelist) takes on the extent's
/// lock: `tsleep_nsec(chan, PRIBIO | (EX_CATCH ? PCATCH : 0), "extnt", INFSLP)`.
fn extent_sleep<T: ?Sized>(
    g: &mut MutexedGuard<'_, ExState>,
    chan: *const T,
    flags: i32,
) -> Result<(), Errno> {
    let pri = PRIBIO | if flags & EX_CATCH != 0 { PCATCH } else { 0 };
    g.sleep(chan, pri, "extnt")
}

/// `extent_do_alloc_region(ex, start, size, flags, myrp)`: a specific region, with the
/// descriptor `myrp`.
fn extent_do_alloc_region(
    ex: &Extent,
    g: &mut MutexedGuard<'_, ExState>,
    start: u64,
    size: u64,
    flags: i32,
    myrp: PoolBox<ExtentRegion>,
) -> Result<(), Errno> {
    let mut start = start;
    let mut size = size;
    let mut end = start.wrapping_add(size.wrapping_sub(1));

    if cfg!(feature = "diagnostic") {
        // Check arguments.
        if size < 1 {
            printf(format_args!(
                "extent_do_alloc_region: extent `{}', size 0x{size:x}\n",
                Name(ex.ex_name)
            ));
            kpanic(format_args!("extent_do_alloc_region: bad size"));
        }
        if end < start {
            printf(format_args!(
                "extent_do_alloc_region: extent `{}', start 0x{start:x}, size 0x{size:x}\n",
                Name(ex.ex_name)
            ));
            kpanic(format_args!("extent_do_alloc_region: overflow"));
        }
        if flags & EX_CONFLICTOK != 0 && flags & EX_WAITSPACE != 0 {
            kpanic(format_args!(
                "extent_do_alloc_region: EX_CONFLICTOK and EX_WAITSPACE are mutually exclusive"
            ));
        }
    }

    // Make sure the requested region lies within the extent.
    if start < ex.ex_start || end > ex.ex_end {
        if cfg!(feature = "diagnostic") {
            printf(format_args!(
                "extent_do_alloc_region: extent `{}' (0x{:x} - 0x{:x})\n",
                Name(ex.ex_name),
                ex.ex_start,
                ex.ex_end
            ));
            printf(format_args!(
                "extent_do_alloc_region: start 0x{start:x}, end 0x{end:x}\n"
            ));
            kpanic(format_args!(
                "extent_do_alloc_region: region lies outside extent"
            ));
        }
        extent_free_region_descriptor(ex, g, myrp);
        return Err(Errno::EINVAL);
    }

    loop {
        // alloc_start: attempt to place ourselves in the desired area of the extent. The
        // regions are sorted: once one starts past our end, there is nothing more to look at.
        // "last" is the region we fall after, `None` at the head.
        let placed = 'walk: {
            let mut last: Option<&ExtentRegion> = None;
            for rp in g.regions.iter() {
                if rp.start() > end {
                    // We lie before this region and don't conflict.
                    break;
                }
                // The current region begins before we end. Check for a conflict.
                if rp.end() >= start {
                    // We conflict. If we can (and want to) wait, do so.
                    if flags & EX_WAITSPACE != 0 {
                        break 'walk Placed::Sleep;
                    }
                    if flags & EX_CONFLICTOK == 0 {
                        break 'walk Placed::Conflict;
                    }
                    // We tolerate conflicts: adjust things such that all space in the
                    // requested region is allocated. 1. The current region overlaps our
                    // start: start at its end. 2. It falls completely within the request:
                    // free it. 3. It overlaps our end: end at its start. 4. The request falls
                    // completely within it: done.
                    if rp.start() <= start {
                        if rp.end() < ex.ex_end {
                            break 'walk Placed::Retry(rp.end() + 1, end);
                        }
                    } else if rp.end() < end {
                        break 'walk Placed::Drop(RKey::of(rp));
                    } else if rp.start() < end && rp.start() > ex.ex_start {
                        break 'walk Placed::Retry(start, rp.start() - 1);
                    }
                    break 'walk Placed::Covered;
                }
                // We don't conflict, but this region lies before us.
                last = Some(rp);
            }
            // We don't conflict with any regions: insert ourselves after "last" (the
            // coalescing on the regions in hand, the tree's part below).
            Placed::Insert(coalesce(ex, &g.regions, start, size, last))
        };

        match placed {
            Placed::Insert(c) => {
                extent_insert_and_optimize(ex, g, start, size, c, myrp);
                return Ok(());
            }
            Placed::Sleep => {
                g.flags |= EXF_WANTED;
                if let Err(e) = extent_sleep(g, ptr::from_ref(ex), flags) {
                    extent_free_region_descriptor(ex, g, myrp);
                    return Err(e);
                }
            }
            Placed::Retry(s, e) => {
                start = s;
                end = e;
                size = end.wrapping_sub(start).wrapping_add(1);
            }
            Placed::Drop(key) => {
                if let Some(rp) = take(g, key) {
                    extent_free_region_descriptor(ex, g, rp);
                }
            }
            Placed::Covered => {
                extent_free_region_descriptor(ex, g, myrp);
                return Ok(());
            }
            Placed::Conflict => {
                extent_free_region_descriptor(ex, g, myrp);
                return Err(Errno::EAGAIN);
            }
        }
    }
}

/// `extent_do_alloc`'s boundary check of a candidate at `newstart` (written twice in the C):
/// `Ok` with the (possibly moved) start, `Err(true)` when the moved candidate no longer fits
/// (`fits` says; the C's `goto skip` in the loop, `goto fail` after it), `Err(false)` when the
/// request can't fit (`goto fail`).
fn extent_boundary_check(
    ex: &Extent,
    newstart: u64,
    req: &Req,
    fits: impl Fn(u64) -> bool,
) -> Result<u64, bool> {
    let mut newstart = newstart;
    let size = req.size;
    let newend = newstart.wrapping_add(size.wrapping_sub(1));

    // Calculate the next boundary after the start of this region.
    let skew = if req.flags & EX_BOUNDZERO != 0 {
        0
    } else {
        ex.ex_start
    };
    let mut dontcross = extent_align(newstart.wrapping_add(1), req.boundary, skew).wrapping_sub(1);

    // Check for overflow
    if dontcross < ex.ex_start {
        dontcross = ex.ex_end;
    } else if newend > dontcross {
        // Candidate region crosses boundary. Throw away the leading part and see if we
        // still fit.
        newstart = dontcross.wrapping_add(1);
        dontcross = dontcross.wrapping_add(req.boundary);
        if !fits(newstart) {
            return Err(true);
        }
    }

    // If we run past the end of the extent or the boundary overflows, then the request
    // can't fit.
    if newstart.wrapping_add(size).wrapping_sub(1) > ex.ex_end || dontcross < newstart {
        return Err(false);
    }

    Ok(newstart)
}

/// `extent_do_alloc(ex, substart, subend, size, alignment, skew, boundary, flags, myrp,
/// result)`: the search of [`Extent::alloc_subregion`], with the descriptor `myrp`.
fn extent_do_alloc(
    ex: &Extent,
    g: &mut MutexedGuard<'_, ExState>,
    req: &Req,
    myrp: PoolBox<ExtentRegion>,
) -> Result<u64, Errno> {
    let Req {
        substart,
        subend,
        size,
        alignment,
        skew,
        boundary,
        flags,
    } = *req;

    if cfg!(feature = "diagnostic") {
        // Check arguments.
        if substart < ex.ex_start
            || substart > ex.ex_end
            || subend > ex.ex_end
            || subend < ex.ex_start
        {
            printf(format_args!(
                "extent_do_alloc: extent `{}', ex_start 0x{:x}, ex_end 0x{:x}\n",
                Name(ex.ex_name),
                ex.ex_start,
                ex.ex_end
            ));
            printf(format_args!(
                "extent_do_alloc: substart 0x{substart:x}, subend 0x{subend:x}\n"
            ));
            kpanic(format_args!("extent_do_alloc: bad subregion"));
        }
        if size < 1 || size - 1 > subend.wrapping_sub(substart) {
            printf(format_args!(
                "extent_do_alloc: extent `{}', size 0x{size:x}\n",
                Name(ex.ex_name)
            ));
            kpanic(format_args!("extent_do_alloc: bad size"));
        }
        if alignment == 0 {
            kpanic(format_args!("extent_do_alloc: bad alignment"));
        }
        if boundary != 0 && boundary < size {
            printf(format_args!(
                "extent_do_alloc: extent `{}', size 0x{size:x}, boundary 0x{boundary:x}\n",
                Name(ex.ex_name)
            ));
            kpanic(format_args!("extent_do_alloc: bad boundary"));
        }
    }

    loop {
        // alloc_start: "last" is the region we looked at last, so that we need not walk the
        // list again to insert ourselves; `None` puts us at the head. The smallest chunk we
        // fit in so far: its overhead ("bestovh", 0 for none: an exact match is inserted at
        // once), start and "last". "exend" is the end of the free region: the end of the
        // extent or the start of a region past the subend.
        let mut last: Option<&ExtentRegion> = None;
        let mut bestovh: u64 = 0;
        let mut beststart: u64 = 0;
        let mut bestlast: Option<&ExtentRegion> = None;
        let mut exend = ex.ex_end;

        // For N allocated regions, we must make (N + 1) checks for unallocated space. The
        // first chunk we check is the area from the beginning of the subregion to the first
        // allocated region after that point.
        let mut newstart = extent_align(substart, alignment, skew);
        if newstart < ex.ex_start {
            if cfg!(feature = "diagnostic") {
                printf(format_args!(
                    "extent_do_alloc: extent `{}' (0x{:x} - 0x{:x}), alignment 0x{alignment:x}\n",
                    Name(ex.ex_name),
                    ex.ex_start,
                    ex.ex_end
                ));
                kpanic(format_args!("extent_do_alloc: overflow after alignment"));
            }
            extent_free_region_descriptor(ex, g, myrp);
            return Err(Errno::EINVAL);
        }

        let fit = 'scan: {
            let regions = &g.regions;
            // Find the first allocated region that begins on or after the subregion start,
            // advancing "last" along the way.
            // (One walk of the tree in order, the C's walk of the list.)
            let mut it = regions.iter().peekable();
            let mut last_end = None;
            while let Some(r) = it.next_if(|r| r.start() < newstart) {
                last = Some(r);
                last_end = Some(r.end());
            }
            let mut rp = it.next();

            // Relocate the start of our candidate region to the end of the last allocated
            // region (if there was one overlapping our subrange).
            if let Some(le) = last_end
                && le >= newstart
            {
                newstart = extent_align(le.wrapping_add(1), alignment, skew);
            }

            while let Some(r) = rp {
                // If the region pasts the subend, bail out and see if we fit against the
                // subend.
                if r.start() > subend {
                    exend = r.start();
                    break;
                }

                // Check the chunk before "rp". Note that our comparison is safe from
                // overflow conditions.
                if le_ov(newstart, size, r.start()) {
                    // Do a boundary check, if necessary. Note that a region may *begin* on
                    // the boundary, but it must end before the boundary.
                    let checked = if boundary != 0 {
                        extent_boundary_check(ex, newstart, req, |ns| le_ov(ns, size, r.start()))
                    } else {
                        Ok(newstart)
                    };
                    match checked {
                        Err(false) => break 'scan Fit::Fail,
                        // skip
                        Err(true) => {}
                        Ok(ns) => {
                            newstart = ns;
                            // We would fit into this space. Calculate the overhead (wasted
                            // space). If we exactly fit, or we're taking the first fit,
                            // insert ourselves into the region list.
                            let ovh = r.start() - newstart - size;
                            if flags & EX_FAST != 0 || ovh == 0 {
                                break 'scan Fit::Found(newstart, last);
                            }
                            // Don't exactly fit, but check to see if we're better than any
                            // current choice.
                            if bestovh == 0 || ovh < bestovh {
                                bestovh = ovh;
                                beststart = newstart;
                                bestlast = last;
                            }
                        }
                    }
                }

                // skip: skip past the current region and check again.
                newstart = extent_align(r.end().wrapping_add(1), alignment, skew);
                if newstart < r.end() {
                    // Overflow condition. Don't error out, since we might have a chunk of
                    // space that we can use.
                    break 'scan Fit::Fail;
                }

                last = Some(r);
                rp = it.next();
            }

            // The final check is from the current starting point to the end of the
            // subregion. If there were no allocated regions, "newstart" is set to the
            // beginning of the subregion, or just past the end of the last allocated region,
            // adjusted for alignment in either case.
            if le_ov(newstart, size.wrapping_sub(1), subend) {
                // Do a boundary check, if necessary. Note that a region may *begin* on the
                // boundary, but it must end before the boundary.
                if boundary != 0 {
                    match extent_boundary_check(ex, newstart, req, |ns| {
                        le_ov(ns, size.wrapping_sub(1), subend)
                    }) {
                        Ok(ns) => newstart = ns,
                        Err(_) => break 'scan Fit::Fail,
                    }
                }

                // We would fit into this space. Calculate the overhead (wasted space). If we
                // exactly fit, or we're taking the first fit, insert ourselves into the region
                // list.
                let ovh = exend
                    .wrapping_sub(newstart)
                    .wrapping_sub(size.wrapping_sub(1));
                if flags & EX_FAST != 0 || ovh == 0 {
                    break 'scan Fit::Found(newstart, last);
                }

                // Don't exactly fit, but check to see if we're better than any current choice.
                if bestovh == 0 || ovh < bestovh {
                    bestovh = ovh;
                    beststart = newstart;
                    bestlast = last;
                }
            }

            Fit::Fail
        };

        let (start, after) = match fit {
            Fit::Found(start, after) => (start, after),
            // fail: there is no chunk large enough to hold the request, or, without EX_FAST,
            // no exact match for it (with EX_FAST, no space at all).
            Fit::Fail if flags & EX_FAST == 0 && bestovh != 0 => {
                // We have a match that's "good enough".
                (beststart, bestlast)
            }
            Fit::Fail => {
                // No space currently available. Wait for it to free up, if possible.
                if flags & EX_WAITSPACE != 0 {
                    g.flags |= EXF_WANTED;
                    if let Err(e) = extent_sleep(g, ptr::from_ref(ex), flags) {
                        extent_free_region_descriptor(ex, g, myrp);
                        return Err(e);
                    }
                    continue;
                }
                extent_free_region_descriptor(ex, g, myrp);
                return Err(Errno::EAGAIN);
            }
        };

        // found: insert ourselves into the region list (after "last", the region in hand).
        let c = coalesce(ex, &g.regions, start, size, after);
        extent_insert_and_optimize(ex, g, start, size, c, myrp);
        return Ok(start);
    }
}

/// `extent_free(ex, start, size, flags)`.
fn extent_free(ex: &Extent, start: u64, size: u64, flags: i32) -> Result<(), Errno> {
    let end = start.wrapping_add(size.wrapping_sub(1));

    if cfg!(feature = "diagnostic") {
        // Check arguments.
        if start < ex.ex_start || end > ex.ex_end {
            ex.print();
            printf(format_args!(
                "extent_free: extent `{}', start 0x{start:x}, size 0x{size:x}\n",
                Name(ex.ex_name)
            ));
            kpanic(format_args!(
                "extent_free: extent `{}', region not within extent",
                Name(ex.ex_name)
            ));
        }
        // Check for an overflow.
        if end < start {
            ex.print();
            printf(format_args!(
                "extent_free: extent `{}', start 0x{start:x}, size 0x{size:x}\n",
                Name(ex.ex_name)
            ));
            kpanic(format_args!("extent_free: overflow"));
        }
    }

    // If we're allowing coalescing, we must allocate a region descriptor now, since it might
    // block.
    let mut nrp = if ex.nocoalesce {
        None
    } else {
        Some(extent_alloc_region_descriptor(ex, flags).ok_or(Errno::ENOMEM)?)
    };

    // Find region and deallocate. 1. start == er_start && end == er_end: free the
    // descriptor. 2. start == er_start && end < er_end: adjust er_start. 3. start > er_start
    // && end == er_end: adjust er_end. 4. start > er_start && end < er_end: fragment the
    // region (needs the new descriptor). Cases 2, 3 and 4 require that the extent coalesces.
    // EX_CONFLICTOK allows partially overlapping regions: cases 1a, 2a and 3a below.
    let mut g = ex.st.lock();
    let mut done = false;
    loop {
        // One walk of the regions in order, acting on the region in hand: cases 2 to 4, 2a and
        // 3a move its start or end in place; what changes the tree (case 1's or 1a's removal,
        // case 4's new region) is done once the walk's borrow has ended.
        let act = 'walk: {
            for r in g.regions.iter() {
                let (rs, re) = (r.start(), r.end());

                // Does the current region end before the chunk to be freed begins?
                if re < start {
                    continue;
                }
                // Does it begin after the chunk ends? Then we've passed every region that
                // might hold it.
                if rs > end {
                    break;
                }

                // Case 1.
                if start == rs && end == re {
                    done = true;
                    break 'walk Freed::Take(RKey::of(r));
                }

                // The following cases all require that EXF_NOCOALESCE is not set.
                if ex.nocoalesce {
                    continue;
                }

                // Case 2.
                if start == rs && end < re {
                    r.set_start(end + 1);
                    done = true;
                    break 'walk Freed::Walked;
                }

                // Case 3.
                if start > rs && end == re {
                    r.set_end(start - 1);
                    done = true;
                    break 'walk Freed::Walked;
                }

                // Case 4.
                if start > rs
                    && end < re
                    && let Some(n) = nrp.take()
                {
                    // Fill in the new descriptor, adjust the current one, insert the new one
                    // after it; we used the new descriptor, so don't free it below.
                    n.set_start(end + 1);
                    n.set_end(re);
                    r.set_end(start - 1);
                    done = true;
                    break 'walk Freed::Insert(n);
                }

                if flags & EX_CONFLICTOK == 0 {
                    continue;
                }

                // Case 1a: the region goes, and the walk goes on.
                if start <= rs && end >= re {
                    break 'walk Freed::TakeAndGoOn(RKey::of(r));
                }

                // Case 2a.
                if start <= rs && end >= rs {
                    r.set_start(end.wrapping_add(1));
                }

                // Case 3a.
                if start <= r.end() && end >= r.end() {
                    r.set_end(start.wrapping_sub(1));
                }
            }
            Freed::Walked
        };
        match act {
            Freed::Walked => break,
            Freed::Take(k) => {
                if let Some(rp) = take(&mut g, k) {
                    extent_free_region_descriptor(ex, &mut g, rp);
                }
                break;
            }
            Freed::Insert(n) => {
                insert(&mut g, n);
                break;
            }
            Freed::TakeAndGoOn(k) => {
                // The C's walk goes on from the next region; walking again from the head
                // reaches it after the same regions, each of which the walk left ending
                // before the chunk (3a) or starting past it (2a, the last it can touch).
                if let Some(rp) = take(&mut g, k) {
                    extent_free_region_descriptor(ex, &mut g, rp);
                }
            }
        }
    }

    if !done && flags & EX_CONFLICTOK == 0 {
        // Region not found, or request otherwise invalid. Printed under the lock, which is
        // given up before the panic: ddb's `show extents` takes it.
        print_locked(ex, &g, printf);
        drop(g);
        printf(format_args!(
            "extent_free: start 0x{start:x}, end 0x{end:x}\n"
        ));
        kpanic(format_args!("extent_free: region not found"));
    }

    // done:
    if let Some(n) = nrp {
        extent_free_region_descriptor(ex, &mut g, n);
    }
    if g.flags & EXF_WANTED != 0 {
        g.flags &= !EXF_WANTED;
        wakeup(ptr::from_ref(ex));
    }
    Ok(())
}

/// `extent_alloc_region_descriptor(ex, flags)`: a descriptor from a fixed extent's freelist
/// (waiting for one with `EX_WAITOK`, or from the pool with `EX_MALLOCOK`), or from the pool
/// (`ER_ALLOC`). `None` when none can be had.
fn extent_alloc_region_descriptor(ex: &Extent, flags: i32) -> Option<PoolBox<ExtentRegion>> {
    if ex.fixed {
        let mut g = ex.st.lock();
        loop {
            // Don't muck with the flags of a descriptor pulled off the freelist: it may be a
            // dynamically allocated one that was kindly given to us.
            if let Some(rp) = g.freelist.pop() {
                return Some(rp);
            }
            if flags & EX_MALLOCOK != 0 {
                break;
            }
            if flags & EX_WAITOK == 0 {
                return None;
            }
            g.flags |= EXF_FLWANTED;
            if extent_sleep(&mut g, ptr::from_ref(&ex.fl_chan), flags).is_err() {
                return None;
            }
        }
    }

    // alloc:
    let pr = if flags & EX_WAITOK != 0 {
        PR_WAITOK
    } else {
        PR_NOWAIT
    };
    extent_pool_init().get(pr, || ExtentRegion::new(ER_ALLOC))
}

/// `extent_free_region_descriptor(ex, rp)`: a fixed extent's descriptor back on its freelist
/// (waking a waiter), and a pool one given to a waiter for the freelist or back to the pool;
/// a caller's (`ER_DISCARD`) back to the pool it was made from.
fn extent_free_region_descriptor(
    ex: &Extent,
    g: &mut MutexedGuard<'_, ExState>,
    rp: PoolBox<ExtentRegion>,
) {
    if rp.flags() & ER_DISCARD != 0 {
        drop(rp);
        return;
    }

    if ex.fixed {
        if rp.flags() & ER_ALLOC != 0 {
            if g.flags & EXF_FLWANTED == 0 {
                drop(rp);
                return;
            }
            // Someone waits for a region descriptor: be nice and give them this one, rather
            // than just free'ing it back to the system (clear all but ER_ALLOC).
            rp.set_flags(ER_ALLOC);
        } else {
            // Clear all flags.
            rp.set_flags(0);
        }
        g.freelist.push(rp);
        if g.flags & EXF_FLWANTED != 0 {
            // wake_em_up:
            g.flags &= !EXF_FLWANTED;
            wakeup(ptr::from_ref(&ex.fl_chan));
        }
        return;
    }

    // We know it's dynamically allocated if we get here.
    drop(rp);
}

/// `descr(flags)`: a caller's descriptor for the `*_with_descr` allocations, made in
/// advance from `extentpl` (`EX_WAITOK`: it may sleep). `None` when memory is short.
pub fn descr(flags: i32) -> Option<Descr> {
    let pr = if flags & EX_WAITOK != 0 {
        PR_WAITOK
    } else {
        PR_NOWAIT
    };
    extent_pool_init().get(pr, || ExtentRegion::new(ER_DISCARD))
}

/// `extent_print1(ex, pr)`: the extent's name, range and flags, then a line per region.
fn extent_print1(ex: &Extent, pr: fn(fmt::Arguments<'_>)) {
    print_locked(ex, &ex.st.lock(), pr);
}

/// [`extent_print1`] for a holder of the extent's lock.
fn print_locked(ex: &Extent, st: &ExState, pr: fn(fmt::Arguments<'_>)) {
    pr(format_args!(
        "extent `{}' (0x{:x} - 0x{:x}), flags={}\n",
        Name(ex.ex_name),
        ex.ex_start,
        ex.ex_end,
        ExfBits(ex.flags_of(st))
    ));
    for rp in st.regions.iter() {
        pr(format_args!("     0x{:x} - 0x{:x}\n", rp.start(), rp.end()));
    }
}

/// `extentpl`'s items out now (tests and the host benches).
#[cfg(any(test, feature = "doubles"))]
pub fn extentpl_nout() -> u64 {
    EX_REGION_PL.nout()
}

const _: () = {
    assert!(extent_align(0x1001, 0x1000, 0) == 0x2000);
    assert!(le_ov(1, 2, 3) && !le_ov(u64::MAX, 2, u64::MAX));
    // The freelist's channel is not the extent's (`fl_chan` is not at offset 0).
    assert!(core::mem::offset_of!(Extent, fl_chan) != 0);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // LZ's twelve cases (creation, placed and searched allocations with alignment, skew,
    // boundary, best and first fit, coalescing, the four freeing cases and the
    // conflict-tolerant ones, fixed storage, caller descriptors, the list of all extents), over
    // the kernel's doubles: `extentpl` and `malloc(9)` are host memory.

    use std::sync::{Mutex as StdMutex, MutexGuard};
    use std::vec::Vec;

    use super::*;
    use crate::testutil;

    /// `extentpl` is global: the tests that count its items serialise.
    static LOCK: StdMutex<()> = StdMutex::new(());

    fn setup() -> MutexGuard<'static, ()> {
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        testutil::install();
        g
    }

    /// `ex_flags`.
    fn flags(ex: &Extent) -> i32 {
        ex.flags_of(&ex.st.lock())
    }

    /// `M_DEVBUF`.
    const M_DEVBUF: i32 = 2;

    /// The extent's regions, in order.
    fn regions(ex: &Extent) -> Vec<(u64, u64)> {
        ex.st
            .lock()
            .regions
            .iter()
            .map(|r| (r.start(), r.end()))
            .collect()
    }

    fn new_extent(start: u64, end: u64, flags: i32) -> ExtentRef {
        match Extent::create(b"test", start, end, M_DEVBUF, EX_WAITOK | flags) {
            Some(ex) => ex,
            None => panic!("extent_create"),
        }
    }

    #[test]
    fn align_and_overflow_helpers() {
        assert_eq!(extent_align(0x1001, 0x1000, 0), 0x2000);
        assert_eq!(extent_align(0x1000, 0x1000, 0), 0x1000);
        // The skew shifts the grid.
        assert_eq!(extent_align(0x1001, 0x1000, 0x10), 0x1010);
        assert_eq!(extent_align(5, EX_NOALIGN, 0), 5);
        assert!(le_ov(1, 2, 3));
        assert!(!le_ov(2, 2, 3));
        assert!(!le_ov(u64::MAX, 2, u64::MAX));
    }

    #[test]
    fn create_empty_and_filled() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        assert!(regions(&ex).is_empty());
        assert_eq!(flags(&ex), 0);
        assert_eq!((ex.ex_start, ex.ex_end), (0, 0xff));

        let ex = new_extent(0, u64::MAX, EX_FILLED | EX_NOCOALESCE);
        assert_eq!(regions(&ex), [(0, u64::MAX)]);
        assert_eq!(flags(&ex), EXF_NOCOALESCE);
        extent_destroy(ex);
    }

    /// A filled extent with the host bridge's windows freed, as acpipci builds its memory
    /// extent from `_CRS`.
    #[test]
    fn filled_extent_freed_windows() {
        let _g = setup();
        let ex = new_extent(0, u64::MAX, EX_FILLED);
        assert_eq!(ex.free(0x1000_0000, 0x2eff_0000, EX_WAITOK), Ok(()));
        assert_eq!(ex.free(0x80_0000_0000, 0x80_0000_0000, EX_WAITOK), Ok(()));
        assert_eq!(
            regions(&ex),
            [
                (0, 0x0fff_ffff),
                (0x3eff_0000, 0x7f_ffff_ffff),
                (0x100_0000_0000, u64::MAX)
            ]
        );

        // A BAR in the low window, 64 KiB aligned.
        let a = ex.alloc(0x4000, 0x1_0000, 0, EX_NOBOUNDARY, EX_NOWAIT);
        assert_eq!(a, Ok(0x1000_0000));
        // It coalesced with the region before the window.
        assert_eq!(regions(&ex)[0], (0, 0x1000_3fff));

        // Giving it back splits nothing: case 3 (end of a region).
        assert_eq!(ex.free(0x1000_0000, 0x4000, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex)[0], (0, 0x0fff_ffff));
    }

    #[test]
    fn alloc_region_conflicts_and_coalescing() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        assert_eq!(ex.alloc_region(0x10, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x30, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x10, 0x1f), (0x30, 0x3f)]);

        // Overlap: EAGAIN.
        assert_eq!(ex.alloc_region(0x18, 0x10, EX_NOWAIT), Err(Errno::EAGAIN));
        // Outside the extent: EINVAL (no DIAGNOSTIC).
        #[cfg(not(feature = "diagnostic"))]
        assert_eq!(ex.alloc_region(0xf8, 0x10, EX_NOWAIT), Err(Errno::EINVAL));

        // Exactly the gap: both neighbours merge into one region.
        assert_eq!(ex.alloc_region(0x20, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x10, 0x3f)]);
        // Before the first region, touching it: prepended.
        assert_eq!(ex.alloc_region(0x0, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x0, 0x3f)]);
        // Touching the next only.
        assert_eq!(ex.alloc_region(0x50, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x48, 0x8, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x0, 0x3f), (0x48, 0x5f)]);
    }

    #[test]
    fn alloc_region_conflictok() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        assert_eq!(ex.alloc_region(0x10, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x40, 0x10, EX_NOWAIT), Ok(()));
        // Covers the first region's tail, the gap, the whole second region and more: the
        // whole range ends up allocated.
        assert_eq!(
            ex.alloc_region(0x18, 0x48, EX_NOWAIT | EX_CONFLICTOK),
            Ok(())
        );
        assert_eq!(regions(&ex), [(0x10, 0x5f)]);
        // Inside a region that reaches the extent's end (case 4; one that does not is
        // moved by case 1, the C's order of the tests): nothing to do, and the descriptor
        // goes back.
        assert_eq!(ex.alloc_region(0xc0, 0x40, EX_NOWAIT), Ok(()));
        let out = extentpl_nout();
        assert_eq!(
            ex.alloc_region(0xd0, 0x10, EX_NOWAIT | EX_CONFLICTOK),
            Ok(())
        );
        assert_eq!(regions(&ex), [(0x10, 0x5f), (0xc0, 0xff)]);
        assert_eq!(extentpl_nout(), out);
    }

    #[test]
    fn conflictok_case1_quirk_admits_a_region_at_a_taken_start() {
        let _g = setup();
        // The C's case 1 restarts at the conflicting region's end + 1 with the request's end
        // unchanged: here `[0x2000, 0x1fff]`, inserted after the first region, before the one
        // that starts at 0x2000 (subr_extent.c's list admits both; the C returns 0).
        let ex = new_extent(0, 0xffff, EX_NOCOALESCE);
        assert_eq!(ex.alloc_region(0x1000, 0x1000, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x2000, 0x1000, EX_NOWAIT), Ok(()));
        assert_eq!(
            ex.alloc_region(0x1000, 0x1000, EX_NOWAIT | EX_CONFLICTOK),
            Ok(())
        );
        assert_eq!(
            regions(&ex),
            [(0x1000, 0x1fff), (0x2000, 0x1fff), (0x2000, 0x2fff)]
        );
        // Twice (an SMMU window reserved for two requester IDs): two equal regions, which
        // no walk tells apart.
        assert_eq!(
            ex.alloc_region(0x1000, 0x1000, EX_NOWAIT | EX_CONFLICTOK),
            Ok(())
        );
        assert_eq!(
            regions(&ex),
            [
                (0x1000, 0x1fff),
                (0x2000, 0x1fff),
                (0x2000, 0x1fff),
                (0x2000, 0x2fff)
            ]
        );
        // The region that shares their start is freed alone (case 1, the region in hand).
        let out = extentpl_nout();
        assert_eq!(ex.free(0x2000, 0x1000, EX_NOWAIT), Ok(()));
        assert_eq!(
            regions(&ex),
            [(0x1000, 0x1fff), (0x2000, 0x1fff), (0x2000, 0x1fff)]
        );
        assert_eq!(extentpl_nout(), out - 1);
        // And placed again after them.
        assert_eq!(ex.alloc_region(0x2000, 0x1000, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex).last(), Some(&(0x2000, 0x2fff)));
        assert_eq!(ex.alloc_region(0x2800, 0x10, EX_NOWAIT), Err(Errno::EAGAIN));
    }

    #[test]
    fn a_held_extent_prints_as_locked_in_ddb() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        let held = ex.st.lock();
        assert!(try_lock_free(&ex.st).is_none());
        drop(held);
        assert!(try_lock_free(&ex.st).is_some());
        // The whole list, with nothing held, prints (and takes no lock it holds).
        extent_print_all(|_| {});
    }

    #[test]
    fn alloc_best_fit_and_first_fit() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        // Gaps: [0x10, 0x2f] (32), [0x40, 0x4f] (16), [0x60, 0xff] (160).
        assert_eq!(ex.alloc_region(0x0, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x30, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x50, 0x10, EX_NOWAIT), Ok(()));

        // Best fit: the 16-byte gap, an exact match.
        assert_eq!(
            ex.alloc(0x10, EX_NOALIGN, 0, EX_NOBOUNDARY, EX_NOWAIT),
            Ok(0x40)
        );
        // First fit takes the first gap that holds it.
        assert_eq!(
            ex.alloc(0x8, EX_NOALIGN, 0, EX_NOBOUNDARY, EX_NOWAIT | EX_FAST),
            Ok(0x10)
        );
        // Best fit for 8 bytes: the 24-byte rest of the first gap beats the tail.
        assert_eq!(
            ex.alloc(0x8, EX_NOALIGN, 0, EX_NOBOUNDARY, EX_NOWAIT),
            Ok(0x18)
        );
        // Too big: EAGAIN.
        assert_eq!(
            ex.alloc(0x100, EX_NOALIGN, 0, EX_NOBOUNDARY, EX_NOWAIT),
            Err(Errno::EAGAIN)
        );
    }

    #[test]
    fn alloc_alignment_skew_subregion() {
        let _g = setup();
        let ex = new_extent(0, 0xffff, 0);
        assert_eq!(
            ex.alloc_subregion(0x101, 0xffff, 0x10, 0x100, 0, 0, EX_NOWAIT),
            Ok(0x200)
        );
        assert_eq!(
            ex.alloc_subregion(0x101, 0xffff, 0x10, 0x100, 0x8, 0, EX_NOWAIT),
            Ok(0x108)
        );
        // Only up to subend.
        assert_eq!(
            ex.alloc_subregion(0x0, 0xf, 0x20, 1, 0, 0, EX_NOWAIT),
            Err(Errno::EAGAIN)
        );
    }

    #[test]
    fn alloc_boundary() {
        let _g = setup();
        let ex = new_extent(0, 0xffff, 0);
        assert_eq!(ex.alloc_region(0, 0xf0, EX_NOWAIT), Ok(()));
        // 0x20 bytes at 0xf0 would cross 0x100: moved to the boundary.
        assert_eq!(
            ex.alloc(0x20, EX_NOALIGN, 0, 0x100, EX_NOWAIT | EX_FAST),
            Ok(0x100)
        );
        // Without a boundary it fits right after the first region.
        assert_eq!(
            ex.alloc(0x10, EX_NOALIGN, 0, EX_NOBOUNDARY, EX_NOWAIT | EX_FAST),
            Ok(0xf0)
        );
    }

    #[test]
    fn free_cases() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        assert_eq!(ex.alloc_region(0x10, 0x40, EX_NOWAIT), Ok(()));
        // Case 4: the middle, the region splits.
        assert_eq!(ex.free(0x20, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x10, 0x1f), (0x30, 0x4f)]);
        // Case 2: the start.
        assert_eq!(ex.free(0x30, 0x8, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x10, 0x1f), (0x38, 0x4f)]);
        // Case 3: the end.
        assert_eq!(ex.free(0x48, 0x8, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x10, 0x1f), (0x38, 0x47)]);
        // Case 1: a whole region.
        assert_eq!(ex.free(0x10, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x38, 0x47)]);
        // Conflict-tolerant: across a region's start (2a) and past what is allocated.
        assert_eq!(ex.alloc_region(0x60, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.free(0x30, 0x10, EX_NOWAIT | EX_CONFLICTOK), Ok(()));
        assert_eq!(regions(&ex), [(0x40, 0x47), (0x60, 0x6f)]);
        // 1a and 2a: a range covering one region and the second's head.
        assert_eq!(ex.free(0x3c, 0x2c, EX_NOWAIT | EX_CONFLICTOK), Ok(()));
        assert_eq!(regions(&ex), [(0x68, 0x6f)]);
        // Nothing there at all is fine with EX_CONFLICTOK.
        assert_eq!(ex.free(0x0, 0x10, EX_NOWAIT | EX_CONFLICTOK), Ok(()));
        assert_eq!(regions(&ex), [(0x68, 0x6f)]);
        // 3a, then 1a twice: the walk goes on past each region it frees.
        assert_eq!(ex.alloc_region(0x10, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x30, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x50, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.free(0x14, 0x5c, EX_NOWAIT | EX_CONFLICTOK), Ok(()));
        assert_eq!(regions(&ex), [(0x10, 0x13)]);
    }

    #[test]
    #[should_panic(expected = "extent_free: region not found")]
    fn freeing_what_is_not_allocated_panics() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        let _ = ex.free(0x10, 0x10, EX_NOWAIT);
    }

    #[test]
    fn nocoalesce_with_descr() {
        let _g = setup();
        let ex = new_extent(0, 0xff, EX_NOCOALESCE);
        let out = extentpl_nout();
        let Some(d1) = descr(EX_NOWAIT) else {
            panic!("descr");
        };
        let Some(d2) = descr(EX_NOWAIT) else {
            panic!("descr");
        };
        assert_eq!(ex.alloc_region_with(0x10, 0x10, EX_NOWAIT, d1), Ok(()));
        assert_eq!(
            ex.alloc_with(0x10, EX_NOALIGN, 0, EX_NOBOUNDARY, EX_FAST, d2),
            Ok(0)
        );
        // Neighbours stay apart; the descriptors are the extent's now, marked the caller's.
        assert_eq!(regions(&ex), [(0x0, 0xf), (0x10, 0x1f)]);
        assert!(ex.st.lock().regions.iter().all(|r| r.flags() == ER_DISCARD));
        assert_eq!(extentpl_nout(), out + 2);
        // Only whole regions can be freed; a caller's descriptor goes back to the pool.
        assert_eq!(ex.free(0x10, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(regions(&ex), [(0x0, 0xf)]);
        assert_eq!(extentpl_nout(), out + 1);
    }

    #[test]
    fn fixed_storage() {
        let _g = setup();
        // Room for one descriptor.
        static FIX: ExtentFixed<1> = ExtentFixed::new();
        let Some(ex) = FIX.create(b"fixed", 0, 0xff, M_DEVBUF, EX_NOWAIT) else {
            panic!("extent_create");
        };
        assert_ne!(flags(ex) & EXF_FIXED, 0);
        assert_eq!(ex.st.lock().freelist.iter().count(), 1);

        // One descriptor: the second allocation has none.
        assert_eq!(ex.alloc_region(0x0, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x80, 0x10, EX_NOWAIT), Err(Errno::ENOMEM));
        // EX_MALLOCOK falls back to the pool.
        assert_eq!(ex.alloc_region(0x80, 0x10, EX_NOWAIT | EX_MALLOCOK), Ok(()));
        assert_eq!(regions(ex), [(0x0, 0xf), (0x80, 0x8f)]);
        // A storage descriptor goes back to the freelist; a pool one to the pool.
        assert_eq!(ex.free(0x80, 0x10, EX_NOWAIT | EX_MALLOCOK), Ok(()));
        assert_eq!(ex.free(0x0, 0x10, EX_NOWAIT | EX_MALLOCOK), Ok(()));
        assert!(regions(ex).is_empty());
        assert!(!ex.st.lock().freelist.is_empty());
    }

    #[test]
    fn registered_and_printed() {
        let _g = setup();
        let ex = new_extent(0x100, 0x1ff, EX_FILLED);
        let p: *const Extent = &*ex;
        let listed = |p: *const Extent| {
            let g = EXT_LOCK.lock();
            EXT_LIST.iter(&g).any(|e| ptr::eq(e, p))
        };
        assert!(listed(p));
        let before = testutil::printf_count("extent `test' (0x100 - 0x1ff), flags=0\n");
        ex.print();
        assert_eq!(
            testutil::printf_count("extent `test' (0x100 - 0x1ff), flags=0\n"),
            before + 1
        );
        extent_destroy(ex);
        assert!(!listed(p));
    }

    #[test]
    fn the_flags_print_as_percent_b() {
        let s = |v| std::format!("{}", ExfBits(v));
        assert_eq!(s(0), "0");
        assert_eq!(s(EXF_NOCOALESCE), "2<NOCOALESCE>");
        assert_eq!(s(EXF_FIXED | EXF_FLWANTED), "9<FLWANTED,FIXED>");
    }

    #[test]
    fn a_destroyed_extent_gives_its_descriptors_back() {
        let _g = setup();
        let ex = new_extent(0, 0xff, 0);
        let out = extentpl_nout();
        assert_eq!(ex.alloc_region(0x10, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(ex.alloc_region(0x30, 0x10, EX_NOWAIT), Ok(()));
        assert_eq!(extentpl_nout(), out + 2);
        ex.destroy();
        assert!(regions(&ex).is_empty());
        assert_eq!(extentpl_nout(), out);
    }
}
/* </TESTS> */
