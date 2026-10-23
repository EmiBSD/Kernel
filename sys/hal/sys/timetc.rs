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
 * Copyright (c) 2000 Poul-Henning Kamp <phk@FreeBSD.org>
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
 * If we meet some day, and you think this stuff is worth it, you
 * can buy me a beer in return. Poul-Henning Kamp
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/timetc.h>`'s page shared with userland: `struct timekeep` and `TK_VERSION`.
//!
//! Upstream: sys/sys/timetc.h @ 3ce1f3f79392
//! LZ: sys/sys/timetc.rs@3c62ede68802
//!
//! libc reads the timehands from this page (`tk_scale`, `tk_offset_count`, the three
//! bintimes) and the hardware counter itself (`tk_user`, `tk_counter_mask`), retrying while
//! `tk_generation` is 0 or changed under it, so `gettimeofday(2)` and `clock_gettime(2)` need
//! no system call.
//!
//! ## Redesign
//! - LZ's `struct timekeep` was a plain `#[repr(C)]` structure the kernel wrote through a raw
//!   `&mut` of the wired page while userland read it (`tc_update_timekeep`'s one `unsafe`).
//!   [`Timekeep`] keeps that layout as the ABI's record and [`TimekeepAtomic`] is the same
//!   bytes as atomics (each offset asserted equal), the view the kernel writes through: the
//!   page is shared with every process, so each store is an atomic one, and the generation
//!   protocol is the C's (`tk_generation = 0`, release, the fields, release, the
//!   generation). The padding of `struct timekeep` after `tk_version`, `tk_offset_count` and
//!   `tk_counter_mask` is spelled out as words nobody writes.

use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering};

/// `TK_VERSION`.
pub const TK_VERSION: u32 = 0;

/// `struct bintime` as the page lays it out: seconds, then the fraction in units of 2^-64.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TkBintime {
    /// `sec`.
    pub sec: i64,
    /// `frac`.
    pub frac: u64,
}

/// `struct timekeep`: the timehands and timecounter state shared with userland, as the ABI
/// lays it out (the record; the kernel writes the page through [`TimekeepAtomic`]).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timekeep {
    // set at initialization
    /// `tk_version`: version number.
    pub tk_version: u32,

    // timehands members
    /// `tk_scale`.
    pub tk_scale: u64,
    /// `tk_offset_count`.
    pub tk_offset_count: u32,
    /// `tk_offset`.
    pub tk_offset: TkBintime,
    /// `tk_naptime`.
    pub tk_naptime: TkBintime,
    /// `tk_boottime`.
    pub tk_boottime: TkBintime,
    /// `tk_generation` (volatile).
    pub tk_generation: u32,

    // timecounter members
    /// `tk_user`.
    pub tk_user: i32,
    /// `tk_counter_mask`.
    pub tk_counter_mask: u32,
}

/// A `struct bintime` of the page, as two atomic words.
#[repr(C)]
#[derive(Debug, Default)]
pub struct AtomicTkBintime {
    sec: AtomicI64,
    frac: AtomicU64,
}

impl AtomicTkBintime {
    /// Zero.
    pub const fn new() -> Self {
        Self {
            sec: AtomicI64::new(0),
            frac: AtomicU64::new(0),
        }
    }

    /// Stores `sec` and `frac` (relaxed: the generation's release orders them).
    #[inline]
    pub fn set(&self, sec: i64, frac: u64) {
        self.sec.store(sec, Ordering::Relaxed);
        self.frac.store(frac, Ordering::Relaxed);
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> TkBintime {
        TkBintime {
            sec: self.sec.load(Ordering::Relaxed),
            frac: self.frac.load(Ordering::Relaxed),
        }
    }
}

/// `struct timekeep` as the kernel writes it: [`Timekeep`]'s bytes, every field an atomic of
/// the same size at the same offset (the padding as words nobody writes), so the page
/// userland reads while the kernel writes is only ever accessed atomically on the kernel's
/// side.
#[repr(C)]
#[derive(Debug, Default)]
pub struct TimekeepAtomic {
    /// `tk_version`.
    pub tk_version: AtomicU32,
    _pad0: AtomicU32,
    /// `tk_scale`.
    pub tk_scale: AtomicU64,
    /// `tk_offset_count`.
    pub tk_offset_count: AtomicU32,
    _pad1: AtomicU32,
    /// `tk_offset`.
    pub tk_offset: AtomicTkBintime,
    /// `tk_naptime`.
    pub tk_naptime: AtomicTkBintime,
    /// `tk_boottime`.
    pub tk_boottime: AtomicTkBintime,
    /// `tk_generation`.
    pub tk_generation: AtomicU32,
    /// `tk_user`.
    pub tk_user: AtomicI32,
    /// `tk_counter_mask`.
    pub tk_counter_mask: AtomicU32,
    _pad2: AtomicU32,
}

impl TimekeepAtomic {
    /// A zeroed page's view (`tk_version` 0, which is [`TK_VERSION`]).
    pub const fn new() -> Self {
        Self {
            tk_version: AtomicU32::new(0),
            _pad0: AtomicU32::new(0),
            tk_scale: AtomicU64::new(0),
            tk_offset_count: AtomicU32::new(0),
            _pad1: AtomicU32::new(0),
            tk_offset: AtomicTkBintime::new(),
            tk_naptime: AtomicTkBintime::new(),
            tk_boottime: AtomicTkBintime::new(),
            tk_generation: AtomicU32::new(0),
            tk_user: AtomicI32::new(0),
            tk_counter_mask: AtomicU32::new(0),
            _pad2: AtomicU32::new(0),
        }
    }

    /// The page as a reader would see it now, field by field and without the generation
    /// check (the tests, the debugger).
    pub fn snapshot(&self) -> Timekeep {
        Timekeep {
            tk_version: self.tk_version.load(Ordering::Relaxed),
            tk_scale: self.tk_scale.load(Ordering::Relaxed),
            tk_offset_count: self.tk_offset_count.load(Ordering::Relaxed),
            tk_offset: self.tk_offset.get(),
            tk_naptime: self.tk_naptime.get(),
            tk_boottime: self.tk_boottime.get(),
            tk_generation: self.tk_generation.load(Ordering::Relaxed),
            tk_user: self.tk_user.load(Ordering::Relaxed),
            tk_counter_mask: self.tk_counter_mask.load(Ordering::Relaxed),
        }
    }
}

// The atomic view has the record's layout, field for field (the C's `struct timekeep`, 88
// bytes on both LP64 machines), and fits in the one page `exec_timekeep_map` maps.
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<Timekeep>() == 88);
    assert!(size_of::<TimekeepAtomic>() == size_of::<Timekeep>());
    assert!(align_of::<TimekeepAtomic>() == align_of::<Timekeep>());
    assert!(offset_of!(TimekeepAtomic, tk_version) == offset_of!(Timekeep, tk_version));
    assert!(offset_of!(TimekeepAtomic, tk_scale) == offset_of!(Timekeep, tk_scale));
    assert!(offset_of!(TimekeepAtomic, tk_offset_count) == offset_of!(Timekeep, tk_offset_count));
    assert!(offset_of!(TimekeepAtomic, tk_offset) == offset_of!(Timekeep, tk_offset));
    assert!(offset_of!(TimekeepAtomic, tk_naptime) == offset_of!(Timekeep, tk_naptime));
    assert!(offset_of!(TimekeepAtomic, tk_boottime) == offset_of!(Timekeep, tk_boottime));
    assert!(offset_of!(TimekeepAtomic, tk_generation) == offset_of!(Timekeep, tk_generation));
    assert!(offset_of!(TimekeepAtomic, tk_user) == offset_of!(Timekeep, tk_user));
    assert!(offset_of!(TimekeepAtomic, tk_counter_mask) == offset_of!(Timekeep, tk_counter_mask));
    assert!(offset_of!(Timekeep, tk_scale) == 8);
    assert!(offset_of!(Timekeep, tk_offset) == 24);
    assert!(offset_of!(Timekeep, tk_generation) == 72);
    assert!(offset_of!(Timekeep, tk_counter_mask) == 80);
    assert!(size_of::<TimekeepAtomic>() <= crate::sys::param::PAGE_SIZE);
};
/* </CODE> */
