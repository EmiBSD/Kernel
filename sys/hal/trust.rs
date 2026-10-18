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
//! The memory bridges (`PoolBackend`/`RawPool`, `Malloc`) and the kernel map's page provider
//! (`PageMemory`) are safe traits (decision 30): their implementations (the bridges registered
//! once by the kernel's boot path with the kernel's own `pool(9)` and `malloc(9)`; the
//! provider the kernel's own `km` is and passes down as a value, N3.2) build nothing unsafe;
//! they return a pointer and the size they granted. That the block is the kernel's memory,
//! exclusively the caller's until it is freed through the same implementation, is the
//! **registration contract** (`sys/hal/bridges.rs`; for the page provider, the contract of
//! `crate::kmbuf::PageMemory`), on which the `// SAFETY:` of every site that makes a
//! reference or a slice of such a block rests. It is a documented trust boundary, not a
//! compile-time proof, and it goes with the bridges (N9) and, for the pages, when the kernel
//! map's record is typed (N3.5). Since the page provider is a value and not a slot, the
//! boundary is chosen at each call: every caller of `KmBuf::alloc_pageable`/`alloc_wired` and
//! of `kernel::pipe::Pipes::new` names its provider, and in a kernel image the one production
//! choice is `sys/kern/pipe_glue.rs`'s `PIPES`, which passes the kernel's
//! `kernel::vm::km::page_memory()` (the test doubles are `cfg(test)` code).
//!
//! What can be checked is checked here, on every answer: the pointer is not null (it is a
//! `NonNull`), it is aligned as the request needs, and the size granted covers the request.
//! A failed check is `panic(9)` naming the bridge: a kernel whose allocator lies about its
//! memory cannot continue. Under `diagnostic` a fresh block is also probed: its first and
//! last bytes are written and read back (then restored), so a block that is not memory, or
//! not writable, fails at its first use instead of corrupting something later.
//!
//! The machine's `kcopy` (`hal::user::UserCopy::kcopy_in`/`kcopy_out`, N3.5 S0) writes and
//! reads kernel memory named by an address. `hal` calls it only for a range inside a
//! `hal::user::ForeignWindow`, which the kernel map's token alone mints
//! (`KernelMapper::foreign_window`): nothing checks the range, so a window is a write to any
//! kernel address its maker names, the strongest primitive the token gives. The token's
//! contract (as for `kenter`'s addresses, which no check here could establish either): its
//! holder mints a window only over a range its own `uvm_map_extract` returned into the
//! kernel map, onto foreign pages no Rust object occupies, and unmaps that range only after
//! the window is gone (N3.5 S1 makes this a type: windows minted only from the extract's
//! result). The window bounds every copy (a range past it is `panic(9)`); the fault on an
//! unmapped page is the machine's `pcb_onfault`, an `EFAULT`.
//!
//! The pmap's page provider (`hal::pmap::PhysPages`, N3.1, passed to `pmap_bootstrap` as a
//! value by the same boot path) is under the same contract for the frames it hands out, and a
//! machine's pmap views a frame through the direct map only after [`check_frame`]: the frame
//! page-aligned, its whole page inside the direct map the boot glue described.
//!
//! ## Redesign
//! - Not a port: the C trusts `km_alloc`, `pool_get` and `malloc` silently. Here the trust is
//!   one function every memory piece calls, so it is audited in one place.

use core::ptr::NonNull;

use crate::assert::hal_panic;

/// Checks a frame `pa` (from `PhysPages`, or a caller's `Paddr`) before a pmap views it through
/// the direct map `[base, end)` that the boot glue described: `pa` page-aligned (`page` a
/// power of two), the map set (`base` not 0), and the whole page inside it. Its direct-map
/// address; panics naming the frame otherwise.
#[cfg_attr(
    not(all(target_os = "none", target_arch = "x86_64")),
    allow(dead_code, reason = "amd64's pmap views frames through its direct map")
)]
pub(crate) fn check_frame(pa: usize, page: usize, base: usize, end: usize) -> usize {
    let va = base.checked_add(pa).filter(|va| {
        base != 0 && pa & (page - 1) == 0 && va.checked_add(page).is_some_and(|e| e <= end)
    });
    match va {
        Some(va) => va,
        None => hal_panic(format_args!(
            "PhysPages: frame {pa:#x} outside the direct map [{base:#x}, {end:#x})"
        )),
    }
}

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
    if ptr.as_ptr().addr().checked_add(granted).is_none() {
        hal_panic(format_args!(
            "{bridge}: block {ptr:p} of {granted} bytes ends past the address space"
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
    #[should_panic(expected = "ends past the address space")]
    fn a_block_past_the_end_of_memory_panics() {
        let mut b = [0u64; 4];
        let p = NonNull::from(&mut b).cast::<u8>();
        check_block("test", p, usize::MAX, 32, 8);
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
