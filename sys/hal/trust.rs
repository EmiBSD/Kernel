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
/* </LICENSES> */

/* <CODE> */
//! The trust boundary of the memory bridges: the checks `hal` makes on every block the
//! kernel's allocators hand it, before it trusts the block.
//!
//! The memory bridges (`PageMemory`, `PoolBackend`/`RawPool`, `Malloc`) are safe traits
//! (decision 30): their implementations, registered once by the kernel's boot path with the
//! kernel's own `uvm_km`, `pool(9)` and `malloc(9)`, build nothing unsafe; they return a
//! pointer and the size they granted. That the block is the kernel's memory, exclusively the
//! caller's until it is freed through the same bridge, is the **registration contract**
//! (`sys/hal/bridges.rs`), on which the `// SAFETY:` of every site that makes a reference or
//! a slice of such a block rests. It is a documented trust boundary, not a compile-time
//! proof, and it goes with the bridges (N3 for `PageMemory`, N9 for the others).
//!
//! What can be checked is checked here, on every answer: the pointer is not null (it is a
//! `NonNull`), it is aligned as the request needs, and the size granted covers the request.
//! A failed check is `panic(9)` naming the bridge: a kernel whose allocator lies about its
//! memory cannot continue. Under `diagnostic` a fresh block is also probed: its first and
//! last bytes are written and read back (then restored), so a block that is not memory, or
//! not writable, fails at its first use instead of corrupting something later.
//!
//! ## Redesign
//! - Not a port: the C trusts `km_alloc`, `pool_get` and `malloc` silently. Here the trust is
//!   one function every memory piece calls, so it is audited in one place.

use core::ptr::NonNull;

use crate::assert::hal_panic;

/// Checks a block `bridge` returned for a request of `requested` bytes aligned to `align`:
/// aligned, and `granted` covering the request; probes it under `diagnostic`. Panics naming
/// the bridge when a check fails.
pub(crate) fn check_block(
    bridge: &'static str,
    ptr: NonNull<u8>,
    granted: usize,
    requested: usize,
    align: usize,
) {
    if !ptr.as_ptr().addr().is_multiple_of(align.max(1)) {
        hal_panic(format_args!(
            "{bridge}: block {ptr:p} not aligned to {align} bytes"
        ));
    }
    if granted < requested {
        hal_panic(format_args!(
            "{bridge}: block of {granted} bytes for a request of {requested}"
        ));
    }
    #[cfg(feature = "diagnostic")]
    probe(bridge, ptr, requested);
}

/// Writes the complement of the first and the last byte of a fresh block, reads it back and
/// restores it: a block that is not writable memory fails here, at its first use.
#[cfg(feature = "diagnostic")]
fn probe(bridge: &'static str, ptr: NonNull<u8>, len: usize) {
    if len == 0 {
        return;
    }
    let first = ptr.as_ptr();
    let last = first.wrapping_add(len - 1);
    for p in [first, last] {
        // SAFETY: the registration contract (module docs): `ptr` is a fresh block of at least
        // `len` bytes of kernel memory that nothing else references yet, so its first and last
        // bytes are writable and owned by the caller; the byte is restored before the block is
        // used, so its contents (zeroed or not) are as the allocator left them.
        let ok = unsafe {
            let old = p.read_volatile();
            p.write_volatile(!old);
            let back = p.read_volatile();
            p.write_volatile(old);
            back == !old
        };
        if !ok {
            hal_panic(format_args!(
                "{bridge}: block {ptr:p} is not writable memory"
            ));
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_block_passes() {
        let mut b = [0u64; 4];
        let p = NonNull::from(&mut b).cast::<u8>();
        check_block("test", p, 32, 32, 8);
        assert_eq!(b, [0; 4]);
    }

    #[test]
    #[should_panic(expected = "test: block of 16 bytes for a request of 32")]
    fn a_short_block_panics() {
        let mut b = [0u64; 4];
        check_block("test", NonNull::from(&mut b).cast::<u8>(), 16, 32, 8);
    }

    #[test]
    #[should_panic(expected = "not aligned to 8 bytes")]
    fn a_misaligned_block_panics() {
        let mut b = [0u64; 4];
        let p = NonNull::from(&mut b).cast::<u8>();
        let odd = NonNull::new(p.as_ptr().wrapping_add(1)).unwrap();
        check_block("test", odd, 31, 8, 8);
    }
}
/* </TESTS> */
