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
 * Copyright (c) 2009 Dariusz Swiderski <sfires@sfires.net>
 * Copyright (c) 2009 Mark Kettenis <kettenis@openbsd.org>
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
//! The bootloader's flattened device tree as bytes: the one piece of `dev/ofw/fdt.c` that is
//! hardware, the blob at the address the bootloader names, handed to the kernel's parser
//! (`kernel::dev::ofw::fdt`) as a `&'static [u8]`.
//!
//! Upstream: sys/dev/ofw/fdt.c @ 3ce1f3f79392
//! Upstream: sys/dev/ofw/fdt.h @ 3ce1f3f79392
//! LZ: sys/dev/ofw/fdt.rs@e8249449d905
//!
//! The machine's boot names the blob as the C does (`arm64/machdep.c`'s `initarm`): first
//! [`probe`], with only the header mapped, for the size to map (`fdt_get_size`); then, on
//! boot(8)'s path only, [`with_boot_mut`], once, with the whole blob mapped, for the C's
//! `explicit_bzero` of the softraid boot key and UUID in `/chosen`; then [`adopt`], for the
//! bytes the parser reads for the rest of the run (`fdt_init`). The first address given is the
//! device tree for the kernel's whole run: another address later is `panic(9)`, and so is a
//! first one after [`seal`] (the first step of `bsd`'s `hal_bridges_register`, before which no
//! code of the `forbid` crates picks an address). `adopt` makes the shared slice once and hands
//! out the same one ever after; once it has run, or once the seal is set, a mutable view is
//! `panic(9)`, and while the mutable view lives `probe` and `adopt` are.
//!
//! What is checked before a byte is viewed: the address is not null, the range does not wrap
//! and spans at most `isize::MAX` bytes, the header's magic is FDT's. What the machine's boot
//! vouches for is the registration contract of `sys/hal/mem/trust.rs` (the device tree): the
//! header mapped at `probe`, the whole blob (writable) at `with_boot_mut` and at `adopt`, for
//! the kernel's run, never written after `adopt`, its frames given to no allocator (arm64's
//! `initarm` refuses a blob in usable memory).
//!
//! amd64 has no device tree: its boot never calls `probe`, `adopt` answers `None`.
//!
//! ## Redesign
//! - LZ's `Fdt` was five raw pointers into the blob in a `StaticCell`, read by
//!   `ptr::read_unaligned` at every token (`be32`, `cstr`, the header's fields, a property's
//!   value): eleven `unsafe` that each rested on the blob being well formed, and boot(8)'s
//!   wipe of the softraid key wrote through a `&[u8]` of it turned `&mut` (`bootarg_bzero`).
//!   Here the blob is viewed by two private functions, `bytes` (shared, from `adopt` on) and
//!   `with_bytes_mut` (the boot's one mutable view, before `adopt`), one `unsafe` each, and the
//!   kernel parses it in safe code with every read bounded, so a malformed tree reads as no
//!   tree instead of reading past it.
//! - The C (and LZ) never relocate nor check the blob; the size read of `probe` checks the
//!   magic only, as the C's first look at the header before the blob is mapped
//!   (`arm64/machdep.c`).

use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering};

use crate::assert::hal_panic;
use crate::sync::once::OnceStatic;

/// `FDT_MAGIC`.
const FDT_MAGIC: u32 = 0xd00d_feed;
/// The length of `struct fdt_head`: ten big-endian words.
const FDT_HEAD_LEN: usize = 40;

/// No view of the blob but `probe`'s header reads yet.
const FREE: u8 = 0;
/// The boot's mutable view is lent ([`with_boot_mut`] is running).
const LENT: u8 = 1;
/// The boot's mutable view was lent and has ended.
const DONE: u8 = 2;
/// [`adopt`] has run: shared views only, for ever.
const ADOPTED: u8 = 3;

/// The device tree the boot named: its address (null until the first [`probe`]), where its
/// views stand, the seal, and the blob's bytes, made once by the first [`adopt`].
struct Blob {
    /// The address the first `probe` recorded, null before.
    va: AtomicPtr<u8>,
    /// `FREE`, `LENT`, `DONE` or `ADOPTED`.
    state: AtomicU8,
    /// Set by [`seal`]: no first address after it.
    sealed: AtomicBool,
    /// The whole blob, once adopted.
    whole: OnceStatic<&'static [u8]>,
}

impl Blob {
    /// No tree named yet.
    const fn new() -> Self {
        Self {
            va: AtomicPtr::new(ptr::null_mut()),
            state: AtomicU8::new(FREE),
            sealed: AtomicBool::new(false),
            whole: OnceStatic::new(),
        }
    }

    /// [`probe`] over this record.
    fn probe(&self, p: *const u8) -> usize {
        if p.is_null() {
            // The C's `fdt_get_size(NULL)`: no tree, nothing recorded.
            return 0;
        }
        if self.state.load(Ordering::Acquire) == LENT {
            hal_panic(format_args!(
                "fdtblob: probe inside the boot's mutable view"
            ));
        }
        let prev = self.va.load(Ordering::Acquire).cast_const();
        if prev != p {
            let sealed = self.sealed.load(Ordering::Acquire);
            if sealed || !prev.is_null() {
                hal_panic(format_args!(
                    "fdtblob: device tree at {p:p} refused (recorded {prev:p}, sealed {sealed})"
                ));
            }
            if let Err(now) = self.va.compare_exchange(
                ptr::null_mut(),
                p.cast_mut(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) && now.cast_const() != p
            {
                hal_panic(format_args!(
                    "fdtblob: device tree at {p:p} refused (recorded {now:p})"
                ));
            }
        }
        head_size(bytes(p, FDT_HEAD_LEN))
    }

    /// [`with_boot_mut`] over this record.
    fn with_boot_mut<R>(&self, f: impl FnOnce(&mut [u8]) -> R) -> Option<R> {
        let p = self.va.load(Ordering::Acquire);
        if p.is_null() {
            return None;
        }
        if self.sealed.load(Ordering::Acquire) {
            hal_panic(format_args!("fdtblob: a mutable view after the seal"));
        }
        if let Err(s) = self
            .state
            .compare_exchange(FREE, LENT, Ordering::AcqRel, Ordering::Acquire)
        {
            hal_panic(format_args!(
                "fdtblob: a mutable view refused (state {s}: one only, before adopt)"
            ));
        }
        let size = head_size(bytes(p, FDT_HEAD_LEN));
        let r = (size != 0).then(|| with_bytes_mut(p, size, f));
        self.state.store(DONE, Ordering::Release);
        r
    }

    /// [`adopt`] over this record.
    fn adopt(&self) -> Option<&'static [u8]> {
        if let Some(&b) = self.whole.get() {
            return Some(b);
        }
        let p = self.va.load(Ordering::Acquire).cast_const();
        if p.is_null() {
            return None;
        }
        // LENT is never overwritten: a refused adopt leaves the state as it found it, so a
        // later one (after an unwinding test, or from another CPU) is refused too.
        if self
            .state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |s| {
                (s != LENT).then_some(ADOPTED)
            })
            .is_err()
        {
            hal_panic(format_args!(
                "fdtblob: adopt inside the boot's mutable view"
            ));
        }
        let size = head_size(bytes(p, FDT_HEAD_LEN));
        if size == 0 {
            return None;
        }
        // A racing first adopt made the same slice: either one is kept.
        let _ = self.whole.set(bytes(p, size));
        self.whole.get().copied()
    }
}

/// The device tree of this kernel.
static BLOB: Blob = Blob::new();

/// The header's `fh_size` when its magic is FDT's, 0 otherwise.
fn head_size(head: &[u8]) -> usize {
    let word = |i: usize| {
        head.get(4 * i..4 * i + 4)
            .and_then(|w| <[u8; 4]>::try_from(w).ok())
            .map_or(0, u32::from_be_bytes)
    };
    if word(0) == FDT_MAGIC {
        word(1) as usize
    } else {
        0
    }
}

/// Refuses a range no view may span: a null address, a range that wraps, one past
/// `isize::MAX` bytes.
fn check(p: *const u8, len: usize) {
    if p.is_null() || p.addr().checked_add(len).is_none() || len > isize::MAX as usize {
        hal_panic(format_args!(
            "fdtblob: {len} bytes at {p:p} are no device tree"
        ));
    }
}

/// The `len` bytes at `p`, shared, under the registration contract (the module's docs).
fn bytes(p: *const u8, len: usize) -> &'static [u8] {
    check(p, len);
    // SAFETY: `p` is the pointer the boot recorded through `probe` (the first one, before the
    // seal: no `forbid` crate's code can have named another, and a second one panics there),
    // with the provenance the boot gave it, and `len` is the header's length or the blob's own
    // `fh_size`. The registration contract (`sys/hal/mem/trust.rs`, the device tree), which
    // the machine's boot upholds: those bytes are mapped at that address for the kernel's
    // whole run (the header from `probe`, all `fh_size` bytes from `adopt`: the direct map,
    // never unmapped, or `stand`'s `FDT_COPY` static in the kernel image), and are part of no
    // allocation the kernel hands out (their frames are outside every usable region, which
    // arm64's `initarm` checks; `FDT_COPY`'s one `&mut`, `stand`'s `get_mut` in `_start`, ended
    // before the boot named the tree). The only write after the boot named them is the one
    // mutable view of `with_bytes_mut`, and `Blob`'s state keeps the two apart: no shared view
    // lives while the mutable one does (`probe` and `adopt` panic while the state is `LENT`,
    // and `adopt`'s refusal leaves it `LENT`; the header reads of `probe` and `with_boot_mut`
    // are dropped before the mutable view is made), and once `adopt` has run no mutable view
    // is made again (`ADOPTED`). Checked here: `p` is not null, the range does not wrap and
    // is at most `isize::MAX` bytes; `u8` has no alignment and no invalid value. So the slice
    // aliases no live `&mut` and reads only initialised bytes that live for the run.
    unsafe { core::slice::from_raw_parts(p, len) }
}

/// `f` over the `len` bytes at `p`, mutably: the boot's one view, under the registration
/// contract (the module's docs).
fn with_bytes_mut<R>(p: *mut u8, len: usize, f: impl FnOnce(&mut [u8]) -> R) -> R {
    check(p, len);
    // SAFETY: `p` and `len` as in `bytes` (the recorded pointer, the blob's own `fh_size`),
    // reached only from `Blob::with_boot_mut`, which moved the state from `FREE` to `LENT`
    // (once ever: no path moves it back), so this is the one mutable view of the blob and it
    // lives only for `f` (the closure cannot keep it: its lifetime is `f`'s argument's). No
    // other reference to the bytes is alive: `adopt` has not run (it moves the state to
    // `ADOPTED`, from which the compare-exchange fails), so no `'static` shared view exists;
    // `probe`'s header view was dropped before; while the state is `LENT`, `probe` and `adopt`
    // panic before making one. The machine's boot upholds the rest (`sys/hal/mem/trust.rs`):
    // the call is boot(8)'s, on the boot CPU, before any other CPU or the `forbid` crates run
    // (a call after the seal panics), and the whole blob is mapped read-write there (the
    // direct map over efiboot's `EfiLoaderData` copy). Checked here: `p` not null, the range
    // not wrapping and at most `isize::MAX` bytes; `u8` has no alignment and no invalid value.
    f(unsafe { core::slice::from_raw_parts_mut(p, len) })
}

/// The device tree at `p`, as the machine's boot names it with only its header mapped: its
/// size (the header's `fh_size`) when the magic is FDT's, 0 otherwise or when `p` is null.
/// The first pointer recorded is the kernel's tree: another one later, or a first one after
/// [`seal`], is `panic(9)`. The pointer keeps the provenance the boot made it with, which
/// must allow writes (never one made from a `&[u8]`): [`with_boot_mut`] writes through it.
pub fn probe(p: *const u8) -> usize {
    BLOB.probe(p)
}

/// `f` over the whole device tree [`probe`] recorded, mutably: the boot's one view, for
/// boot(8)'s wipe of the softraid boot key and UUID (`arm64/machdep.c`), with the whole blob
/// mapped and before [`adopt`]. `None` (and `f` not called) when no tree was named or its
/// magic is not FDT's. A second call, a call after `adopt` or after [`seal`], and a `probe` or
/// `adopt` from inside `f`, are `panic(9)`.
pub fn with_boot_mut<R>(f: impl FnOnce(&mut [u8]) -> R) -> Option<R> {
    BLOB.with_boot_mut(f)
}

/// The whole device tree [`probe`] recorded, once the machine's boot has mapped it all; the
/// same slice ever after. `None` when no tree was named or its magic is not FDT's.
pub fn adopt() -> Option<&'static [u8]> {
    BLOB.adopt()
}

/// Ends the boot's naming of the tree: a first address given to [`probe`] afterwards, and a
/// [`with_boot_mut`], are `panic(9)`. Called first by `bsd`'s `hal_bridges_register`, before
/// any code of the `forbid` crates may pick an address; sealing can only refuse.
pub fn seal() {
    BLOB.sealed.store(true, Ordering::Release);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::vec;
    use std::vec::Vec;

    use super::*;

    /// A blob of `len` bytes (at least a header) with `magic`, leaked: the host's stand-in for
    /// the bootloader's memory, which lives for the process.
    fn leaked(magic: u32, len: usize) -> *const u8 {
        let mut v: Vec<u8> = vec![0; len.max(FDT_HEAD_LEN)];
        v[..4].copy_from_slice(&magic.to_be_bytes());
        v[4..8].copy_from_slice(&(len as u32).to_be_bytes());
        v[len.max(FDT_HEAD_LEN) - 1] = 0xa5;
        Box::leak(v.into_boxed_slice()).as_mut_ptr().cast_const()
    }

    #[test]
    fn the_size_then_the_same_bytes() {
        let b = Blob::new();
        assert!(b.adopt().is_none(), "nothing named yet");
        let p = leaked(FDT_MAGIC, 96);
        assert_eq!(b.probe(p), 96);
        b.sealed.store(true, Ordering::Release);
        assert_eq!(b.probe(p), 96, "the recorded pointer, also after the seal");
        let whole = b.adopt().unwrap();
        assert_eq!(whole.len(), 96);
        assert!(core::ptr::eq(whole.as_ptr(), p));
        assert_eq!(whole[95], 0xa5);
        assert!(core::ptr::eq(b.adopt().unwrap(), whole), "the same slice");
    }

    #[test]
    fn a_wrong_magic_is_no_tree() {
        let b = Blob::new();
        let p = leaked(0x1234_5678, 64);
        assert_eq!(b.probe(p), 0);
        assert!(b.with_boot_mut(|_| ()).is_none());
        assert!(b.adopt().is_none());
    }

    #[test]
    fn null_is_no_tree_and_records_nothing() {
        let b = Blob::new();
        assert_eq!(b.probe(ptr::null()), 0);
        assert!(b.with_boot_mut(|_| ()).is_none());
        assert!(b.adopt().is_none());
        assert_eq!(
            b.probe(leaked(FDT_MAGIC, 48)),
            48,
            "a tree may still be named"
        );
    }

    #[test]
    fn the_boot_view_writes_before_adopt() {
        let b = Blob::new();
        let p = leaked(FDT_MAGIC, 64);
        assert_eq!(b.probe(p), 64);
        assert_eq!(
            b.with_boot_mut(|blob| {
                blob[50] = 7;
                blob.len()
            }),
            Some(64)
        );
        let whole = b.adopt().unwrap();
        assert_eq!(whole[50], 7);
        assert_eq!(whole[63], 0xa5);
    }

    #[test]
    #[should_panic(expected = "mutable view refused")]
    fn a_mutable_view_after_adopt_is_refused() {
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48));
        let _ = b.adopt();
        let _ = b.with_boot_mut(|_| ());
    }

    #[test]
    #[should_panic(expected = "mutable view refused")]
    fn a_second_mutable_view_is_refused() {
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48));
        let _ = b.with_boot_mut(|_| ());
        let _ = b.with_boot_mut(|_| ());
    }

    #[test]
    #[should_panic(expected = "after the seal")]
    fn a_mutable_view_after_the_seal_is_refused() {
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48));
        b.sealed.store(true, Ordering::Release);
        let _ = b.with_boot_mut(|_| ());
    }

    #[test]
    #[should_panic(expected = "inside the boot's mutable view")]
    fn a_probe_inside_the_mutable_view_is_refused() {
        let b = Blob::new();
        let p = leaked(FDT_MAGIC, 48);
        let _ = b.probe(p);
        let _ = b.with_boot_mut(|_| b.probe(p));
    }

    #[test]
    #[should_panic(expected = "inside the boot's mutable view")]
    fn an_adopt_inside_the_mutable_view_is_refused() {
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48));
        let _ = b.with_boot_mut(|_| b.adopt().map(<[u8]>::len));
    }

    #[test]
    fn an_adopt_refused_inside_the_view_stays_refused() {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48));
        let _ = b.with_boot_mut(|_| {
            let first = catch_unwind(AssertUnwindSafe(|| b.adopt().map(<[u8]>::len)));
            assert!(first.is_err(), "the first adopt is refused");
            let second = catch_unwind(AssertUnwindSafe(|| b.adopt().map(<[u8]>::len)));
            assert!(
                second.is_err(),
                "and so is the second: LENT was not overwritten"
            );
            assert_eq!(b.state.load(Ordering::Acquire), LENT);
        });
        assert_eq!(
            b.adopt().map(<[u8]>::len),
            Some(48),
            "after the view, as usual"
        );
    }

    #[test]
    #[should_panic(expected = "refused")]
    fn a_second_address_is_refused() {
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48));
        let _ = b.probe(leaked(FDT_MAGIC, 48));
    }

    #[test]
    #[should_panic(expected = "refused")]
    fn a_first_address_after_the_seal_is_refused() {
        let b = Blob::new();
        b.sealed.store(true, Ordering::Release);
        let _ = b.probe(leaked(FDT_MAGIC, 48));
    }

    #[test]
    #[should_panic(expected = "no device tree")]
    fn a_range_that_wraps_is_refused() {
        let _ = bytes(ptr::without_provenance(usize::MAX - 8), FDT_HEAD_LEN);
    }

    #[test]
    #[should_panic(expected = "no device tree")]
    fn a_range_past_isize_max_is_refused() {
        let _ = bytes(ptr::without_provenance(16), isize::MAX as usize + 1);
    }

    #[test]
    fn the_header_size_reads_only_what_is_there() {
        assert_eq!(head_size(&[]), 0);
        assert_eq!(head_size(&FDT_MAGIC.to_be_bytes()), 0, "no size word");
        let mut h = [0u8; 8];
        h[..4].copy_from_slice(&FDT_MAGIC.to_be_bytes());
        h[4..].copy_from_slice(&0x2000u32.to_be_bytes());
        assert_eq!(head_size(&h), 0x2000);
    }
}
/* </TESTS> */
