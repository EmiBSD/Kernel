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
//! The machine's boot names the blob twice, as the C does (`arm64/machdep.c`'s `initarm`):
//! first [`probe`], with only the header mapped, for the size to map (`fdt_get_size`), then
//! [`adopt`], once the whole blob is mapped, for the bytes the parser reads (`fdt_init`). The
//! first address given is the device tree for the kernel's whole run: another address later
//! is `panic(9)`, and so is a first one after the boot's bridges are registered (the
//! [`crate::user::KcopyToken`] taken), when code of the `forbid` crates may run. `adopt`
//! makes the slice once and hands out the same one ever after. No call writes the blob.
//!
//! What is checked before a byte is viewed: the address is not null, the range does not wrap
//! and spans at most `isize::MAX` bytes, the header's magic is FDT's. What the machine's boot
//! vouches for is the registration contract of `sys/hal/mem/trust.rs` (the device tree): the
//! header mapped at `probe`, the whole blob at `adopt`, for the kernel's run, never written,
//! its frames given to no allocator (arm64's `initarm` refuses a blob in usable memory).
//!
//! amd64 has no device tree: its boot never calls `probe`, `adopt` answers `None`.
//!
//! ## Redesign
//! - LZ's `Fdt` was five raw pointers into the blob in a `StaticCell`, read by
//!   `ptr::read_unaligned` at every token (`be32`, `cstr`, the header's fields, a property's
//!   value): eleven `unsafe` that each rested on the blob being well formed. Here the blob is
//!   viewed once, as bytes, behind one private `bytes` (one `unsafe`), and the kernel parses
//!   it in safe code with every read bounded, so a malformed tree reads as no tree instead of
//!   reading past it.
//! - The C (and LZ) never relocate nor check the blob; the size read of `probe` checks the
//!   magic only, as the C's first look at the header before the blob is mapped
//!   (`arm64/machdep.c`).

use core::sync::atomic::{AtomicUsize, Ordering};

use sysdefs::types::Vaddr;

use crate::assert::hal_panic;
use crate::sync::once::OnceStatic;

/// `FDT_MAGIC`.
const FDT_MAGIC: u32 = 0xd00d_feed;
/// The length of `struct fdt_head`: ten big-endian words.
const FDT_HEAD_LEN: usize = 40;

/// The device tree the boot named: its address (0 until the first [`probe`]) and the blob's
/// bytes, made once by the first [`adopt`].
struct Blob {
    /// The address the first `probe` recorded, 0 before.
    va: AtomicUsize,
    /// The whole blob, once adopted.
    whole: OnceStatic<&'static [u8]>,
}

impl Blob {
    /// No tree named yet.
    const fn new() -> Self {
        Self {
            va: AtomicUsize::new(0),
            whole: OnceStatic::new(),
        }
    }

    /// [`probe`] over this record; `sealed` is whether the boot's first phase is over.
    fn probe(&self, va: usize, sealed: bool) -> usize {
        if va == 0 {
            // The C's `fdt_get_size(NULL)`: no tree, nothing recorded.
            return 0;
        }
        let prev = self.va.load(Ordering::Acquire);
        if prev != va {
            if sealed || prev != 0 {
                hal_panic(format_args!(
                    "fdtblob: device tree at {va:#x} refused (recorded {prev:#x}, sealed {sealed})"
                ));
            }
            if let Err(now) = self
                .va
                .compare_exchange(0, va, Ordering::AcqRel, Ordering::Acquire)
                && now != va
            {
                hal_panic(format_args!(
                    "fdtblob: device tree at {va:#x} refused (recorded {now:#x})"
                ));
            }
        }
        head_size(bytes(va, FDT_HEAD_LEN))
    }

    /// [`adopt`] over this record.
    fn adopt(&self) -> Option<&'static [u8]> {
        if let Some(&b) = self.whole.get() {
            return Some(b);
        }
        let va = self.va.load(Ordering::Acquire);
        if va == 0 {
            return None;
        }
        let size = head_size(bytes(va, FDT_HEAD_LEN));
        if size == 0 {
            return None;
        }
        // A racing first adopt made the same slice: either one is kept.
        let _ = self.whole.set(bytes(va, size));
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

/// The `len` bytes at `va`, under the registration contract (the module's docs).
fn bytes(va: usize, len: usize) -> &'static [u8] {
    if va == 0 || va.checked_add(len).is_none() || len > isize::MAX as usize {
        hal_panic(format_args!(
            "fdtblob: {len} bytes at {va:#x} are no device tree"
        ));
    }
    let p = va as *const u8;
    // SAFETY: `va` is the address the boot recorded through `probe` (the first one, before the
    // boot's bridges are registered: no `forbid` crate's code can have named another, and a
    // second address panics above us), and `len` is the header's length or the blob's own
    // `fh_size`. The registration contract (`sys/hal/mem/trust.rs`, the device tree), which the
    // machine's boot upholds: those bytes are mapped at that address for the kernel's whole
    // run (the header from `probe`, all `fh_size` bytes from `adopt`, the direct map or the
    // kernel image, never unmapped), never written by anyone after the boot named them, and
    // part of no allocation (their frames are outside every usable region, which arm64's
    // `initarm` checks). Checked here: `p` is not null, the range does not wrap and is at most
    // `isize::MAX` bytes; `u8` has no alignment and no invalid value. So a shared `'static`
    // slice of them aliases no `&mut` and reads only initialised, live bytes.
    unsafe { core::slice::from_raw_parts(p, len) }
}

/// The device tree at `va`, as the machine's boot names it with only its header mapped: its
/// size (the header's `fh_size`) when the magic is FDT's, 0 otherwise or when `va` is null.
/// The first address recorded is the kernel's tree: another one later, or a first one after
/// the boot's bridges are registered, is `panic(9)`.
pub fn probe(va: Vaddr) -> usize {
    BLOB.probe(va.as_usize(), crate::user::KcopyToken::taken())
}

/// The whole device tree [`probe`] recorded, once the machine's boot has mapped it all; the
/// same slice ever after. `None` when no tree was named or its magic is not FDT's.
pub fn adopt() -> Option<&'static [u8]> {
    BLOB.adopt()
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
    fn leaked(magic: u32, len: usize) -> usize {
        let mut v: Vec<u8> = vec![0; len.max(FDT_HEAD_LEN)];
        v[..4].copy_from_slice(&magic.to_be_bytes());
        v[4..8].copy_from_slice(&(len as u32).to_be_bytes());
        v[len.max(FDT_HEAD_LEN) - 1] = 0xa5;
        Box::leak(v.into_boxed_slice()).as_ptr() as usize
    }

    #[test]
    fn the_size_then_the_same_bytes() {
        let b = Blob::new();
        assert!(b.adopt().is_none(), "nothing named yet");
        let va = leaked(FDT_MAGIC, 96);
        assert_eq!(b.probe(va, false), 96);
        assert_eq!(
            b.probe(va, true),
            96,
            "the recorded address, also after the seal"
        );
        let whole = b.adopt().unwrap();
        assert_eq!(whole.len(), 96);
        assert_eq!(whole.as_ptr() as usize, va);
        assert_eq!(whole[95], 0xa5);
        assert!(core::ptr::eq(b.adopt().unwrap(), whole), "the same slice");
    }

    #[test]
    fn a_wrong_magic_is_no_tree() {
        let b = Blob::new();
        let va = leaked(0x1234_5678, 64);
        assert_eq!(b.probe(va, false), 0);
        assert!(b.adopt().is_none());
    }

    #[test]
    fn null_is_no_tree_and_records_nothing() {
        let b = Blob::new();
        assert_eq!(b.probe(0, false), 0);
        assert!(b.adopt().is_none());
        let va = leaked(FDT_MAGIC, 48);
        assert_eq!(b.probe(va, false), 48, "a tree may still be named");
    }

    #[test]
    #[should_panic(expected = "refused")]
    fn a_second_address_is_refused() {
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48), false);
        let _ = b.probe(leaked(FDT_MAGIC, 48), false);
    }

    #[test]
    #[should_panic(expected = "refused")]
    fn a_first_address_after_the_seal_is_refused() {
        let b = Blob::new();
        let _ = b.probe(leaked(FDT_MAGIC, 48), true);
    }

    #[test]
    #[should_panic(expected = "no device tree")]
    fn a_range_that_wraps_is_refused() {
        let _ = bytes(usize::MAX - 8, FDT_HEAD_LEN);
    }

    #[test]
    #[should_panic(expected = "no device tree")]
    fn a_range_past_isize_max_is_refused() {
        let _ = bytes(16, isize::MAX as usize + 1);
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
