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
/* </LICENSES> */

/* <CODE> */
//! `<sys/extent.h>`: `extent(9)`, the general purpose extent manager. The extent itself is the
//! kernel crate's (`kernel::extent`, N4.1); this is the legacy API the callers in `bsd` keep,
//! over it.
//!
//! Upstream: sys/sys/extent.h @ 3ce1f3f79392
//! Upstream: sys/kern/subr_extent.c @ 3ce1f3f79392
//! LZ: sys/sys/extent.rs@3c62ede68802
//! LZ: sys/kern/subr_extent.rs@3c62ede68802
//!
//! `crate::kern::subr_extent` names this module, so the callers' paths are LZ's.
//!
//! ## Deviations
//! - `extent_create` gives the extent up for good (`MBox::leak`): the callers keep LZ's
//!   `&'static Extent` (in `Cell`s they read with `get()`), so `extent_destroy` frees the
//!   regions and takes the extent off the list of all extents, but the extent's own
//!   `malloc(9)` block stays, since references to it may remain (one destroy site on arm64:
//!   `ppb_detach`). A deviation of this adapter, until those callers hold an
//!   `kernel::extent::ExtentRef`.
//!
//! ## Redesign
//! - LZ's header held the `#[repr(C)]` records of `Cell`s (`struct extent`, `struct
//!   extent_region`, `struct extent_fixed`) and `subr_extent.rs` the functions over them, with
//!   `extent_destroy` and the `*_with_descr` functions `unsafe fn`s. The types and the
//!   constants are the kernel's now, re-exported; the functions keep their C names and
//!   arguments here over the kernel's methods, all safe: a fixed extent's storage is a
//!   `kernel::extent::ExtentFixed` (`storage` names it as a `FixedRegions`), and a caller's
//!   descriptor (`*_with_descr`) is a [`Descr`] made in advance by [`extent_descr`] and given
//!   to the extent, where the C links the caller's own `struct extent_region` (the SMMU's map
//!   state embedded one).
//! - `extent_print_all` prints through ddb's `db_printf`, as the C.

use crate::kern::subr_prf::db_printf;
use crate::sys::errno::Errno;

pub use kernel::extent::{
    Descr, ER_ALLOC, ER_DISCARD, EX_BOUNDZERO, EX_CATCH, EX_CONFLICTOK, EX_FAST, EX_FILLED,
    EX_MALLOCOK, EX_NOALIGN, EX_NOBOUNDARY, EX_NOCOALESCE, EX_NOWAIT, EX_WAITOK, EX_WAITSPACE,
    EXF_BITS, EXF_FIXED, EXF_FLWANTED, EXF_NOCOALESCE, EXF_WANTED, Extent, ExtentFixed, ExtentList,
    ExtentRegion, ExtentRegionList, FixedRegions,
};

/// `extent_print_all()`: every extent registered (DDB `show extents`).
pub fn extent_print_all() {
    kernel::extent::extent_print_all(|a| {
        db_printf(a);
    });
}

/// `extent_create(name, start, end, mtype, storage, storagesize, flags)`: an extent over
/// `[start, end]`; with `storage` a fixed extent whose descriptors live there. `None` when
/// memory is short (`EX_WAITOK` not given). For good (the module's deviations).
pub fn extent_create(
    name: &'static [u8],
    start: u64,
    end: u64,
    mtype: i32,
    storage: Option<&'static dyn FixedRegions>,
    flags: i32,
) -> Option<&'static Extent> {
    match storage {
        Some(fex) => fex.create(name, start, end, mtype, flags),
        None => Extent::create(name, start, end, mtype, flags).map(hal::malloc::MBox::leak),
    }
}

/// `extent_destroy(ex)`: the extent's regions freed and the extent off the list of all
/// extents; its memory stays (the module's deviations).
pub fn extent_destroy(ex: &'static Extent) {
    ex.destroy();
}

/// `extent_alloc_region(ex, start, size, flags)`: allocates `[start, start + size - 1]`.
pub fn extent_alloc_region(ex: &Extent, start: u64, size: u64, flags: i32) -> Result<(), Errno> {
    ex.alloc_region(start, size, flags)
}

/// `extent_alloc_region_with_descr(ex, start, size, flags, rp)`: as `extent_alloc_region`,
/// with the caller's descriptor ([`extent_descr`]), in an `EX_NOCOALESCE` extent.
pub fn extent_alloc_region_with_descr(
    ex: &Extent,
    start: u64,
    size: u64,
    flags: i32,
    rp: Descr,
) -> Result<(), Errno> {
    ex.alloc_region_with(start, size, flags, rp)
}

/// `extent_alloc_subregion(ex, substart, subend, size, alignment, skew, boundary, flags,
/// result)`: the start.
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn extent_alloc_subregion(
    ex: &Extent,
    substart: u64,
    subend: u64,
    size: u64,
    alignment: u64,
    skew: u64,
    boundary: u64,
    flags: i32,
) -> Result<u64, Errno> {
    ex.alloc_subregion(substart, subend, size, alignment, skew, boundary, flags)
}

/// `extent_alloc_subregion_with_descr(...)`: as `extent_alloc_subregion`, with the caller's
/// descriptor, in an `EX_NOCOALESCE` extent.
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn extent_alloc_subregion_with_descr(
    ex: &Extent,
    substart: u64,
    subend: u64,
    size: u64,
    alignment: u64,
    skew: u64,
    boundary: u64,
    flags: i32,
    rp: Descr,
) -> Result<u64, Errno> {
    ex.alloc_subregion_with(substart, subend, size, alignment, skew, boundary, flags, rp)
}

/// `extent_alloc(ex, size, alignment, skew, boundary, flags, result)`: the simple case of
/// `extent_alloc_subregion`, over the whole extent.
pub fn extent_alloc(
    ex: &Extent,
    size: u64,
    alignment: u64,
    skew: u64,
    boundary: u64,
    flags: i32,
) -> Result<u64, Errno> {
    ex.alloc(size, alignment, skew, boundary, flags)
}

/// `extent_alloc_with_descr(ex, size, alignment, skew, boundary, flags, region, result)`: the
/// simple case of `extent_alloc_subregion_with_descr`.
pub fn extent_alloc_with_descr(
    ex: &Extent,
    size: u64,
    alignment: u64,
    skew: u64,
    boundary: u64,
    flags: i32,
    rp: Descr,
) -> Result<u64, Errno> {
    ex.alloc_with(size, alignment, skew, boundary, flags, rp)
}

/// `extent_free(ex, start, size, flags)`: gives `[start, start + size - 1]` back. Panics when
/// the range is not allocated, unless `EX_CONFLICTOK` allows partial overlaps.
pub fn extent_free(ex: &Extent, start: u64, size: u64, flags: i32) -> Result<(), Errno> {
    ex.free(start, size, flags)
}

/// `extent_print(ex)`.
pub fn extent_print(ex: &Extent) {
    ex.print();
}

/// A descriptor for the `*_with_descr` allocations, made in advance (`EX_WAITOK`: it may
/// sleep); `None` when memory is short. The C's caller embeds a `struct extent_region` in a
/// structure of its own instead.
pub fn extent_descr(flags: i32) -> Option<Descr> {
    kernel::extent::descr(flags)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// `M_DEVBUF`.
    const M_DEVBUF: i32 = 2;

    #[test]
    fn the_legacy_api_keeps_an_extent_for_good_and_destroys_its_regions() {
        kernel::testutil::install();
        let Some(ex) = extent_create(b"legacy", 0, 0xff, M_DEVBUF, None, EX_WAITOK) else {
            panic!("extent_create");
        };
        assert_eq!(extent_alloc_region(ex, 0x10, 0x10, EX_NOWAIT), Ok(()));
        // Best fit: the gap before it, exactly; the two coalesce.
        assert_eq!(
            extent_alloc(ex, 0x10, EX_NOALIGN, 0, EX_NOBOUNDARY, EX_NOWAIT),
            Ok(0)
        );
        assert_eq!(extent_free(ex, 0, 0x20, EX_NOWAIT), Ok(()));
        assert_eq!(extent_alloc_region(ex, 0x40, 0x10, EX_NOWAIT), Ok(()));
        extent_destroy(ex);
        // Empty and still usable memory: a new region fits where the old one was.
        assert_eq!(extent_alloc_region(ex, 0x40, 0x10, EX_NOWAIT), Ok(()));
    }

    #[test]
    fn a_caller_descriptor_goes_through_the_legacy_names() {
        kernel::testutil::install();
        let Some(ex) = extent_create(b"iova", 0, 0xffff, M_DEVBUF, None, EX_NOCOALESCE) else {
            panic!("extent_create");
        };
        let Some(d) = extent_descr(EX_NOWAIT) else {
            panic!("extent_descr");
        };
        assert_eq!(
            extent_alloc_with_descr(ex, 0x1000, 0x1000, 0, 0, EX_NOWAIT, d),
            Ok(0)
        );
        assert_eq!(extent_free(ex, 0, 0x1000, EX_NOWAIT), Ok(()));
    }
}
/* </TESTS> */
