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
//! Typed memory stolen at boot: what `uvm_pageboot_alloc` hands to the page system before the
//! page system exists (the `vm_page` array, the pmemranges), as `&'static [T]`.
//!
//! Not an OpenBSD file. The C takes the bytes from `pmap_steal_memory` and casts them
//! (`pagearray = (struct vm_page *)uvm_pageboot_alloc(...)`, `uvm_pmr_allocpmr`); the memory is
//! never given back. [`steal_slice`] does the same over [`crate::pmap::steal_memory`] and builds
//! each `T` in place, so the kernel gets a typed slice that lives for ever and writes no
//! `unsafe`. [`steal_one`] is the one-element form.
//!
//! The premise, the page provider's contract (`hal::pmap::PhysPages::steal`, a documented trust
//! boundary as the memory bridges', `sys/hal/trust.rs`): the frames `steal_memory` returns are
//! taken out of the page system for good, mapped (through the direct map) and zeroed by the
//! backend, and nothing else knows them. `steal_memory` panics once the page system is up, so
//! these functions are boot-time only, as the C's.
//!
//! ## Redesign
//! - `(struct T *)uvm_pageboot_alloc(n * sizeof(T))` and a loop that initialises each entry
//!   through the pointer -> `steal_slice(n, |i| T)`: the cast and the writes are here, once.

use core::mem::{align_of, size_of};

use crate::arch::current::param::PAGE_SIZE;
use crate::assert::hal_panic;
use crate::sys::types::Vsize;

/// `n` values of `T`, `init(i)` the `i`th, in memory stolen from the page system before it is
/// up; never freed. Panics when `T` is zero-sized or aligned beyond a page, when the size
/// overflows, and (in `steal_memory`) after `uvm_page_init` or when the memory is not there.
pub fn steal_slice<T: Sync>(n: usize, mut init: impl FnMut(usize) -> T) -> &'static [T] {
    if n == 0 {
        return &[];
    }
    if size_of::<T>() == 0 || align_of::<T>() > PAGE_SIZE {
        hal_panic(format_args!(
            "steal_slice: a type of size {} and alignment {}",
            size_of::<T>(),
            align_of::<T>()
        ));
    }
    let Some(bytes) = n.checked_mul(size_of::<T>()) else {
        hal_panic(format_args!("steal_slice: {n} values overflow"));
    };
    let va = crate::pmap::steal_memory(Vsize::new(bytes));
    let base = core::ptr::with_exposed_provenance_mut::<T>(va.as_usize());
    if base.is_null() || !base.is_aligned() {
        hal_panic(format_args!(
            "steal_slice: stolen memory at {:#x}",
            va.as_usize()
        ));
    }
    for i in 0..n {
        let v = init(i);
        // SAFETY: `base` is the start of `bytes = n * size_of::<T>()` bytes the backend's
        // `steal_memory` took out of the page system for good and mapped (the provider's
        // contract, module docs): nothing else references them, they stay valid for the
        // kernel's whole run, and `base` is non-null and aligned for `T` (checked above: the
        // memory is page-aligned and `align_of::<T>()` is at most a page). `i < n`, so
        // `base.add(i)` stays inside the block and `write` stores a whole `T` there without
        // reading or dropping what was in the (zeroed) bytes.
        unsafe { base.add(i).write(v) };
    }
    // SAFETY: the `n` elements from `base` were each initialised by the loop above, in memory
    // that no other reference names and that is never freed or written again except through
    // the shared slice made here (whose `T: Sync` makes it shareable among CPUs); `bytes` did
    // not overflow, so the slice's size is within `isize::MAX` for any block the backend
    // could return.
    unsafe { core::slice::from_raw_parts(base, n) }
}

/// One `T`, built by `init`, in memory stolen from the page system before it is up (the C's
/// `uvm_pmr_allocpmr` during `uvm_page_init`); never freed. Panics as [`steal_slice`].
pub fn steal_one<T: Sync>(init: impl FnOnce() -> T) -> &'static T {
    let mut init = Some(init);
    let s = steal_slice(1, |_| match init.take() {
        Some(f) => f(),
        None => hal_panic(format_args!("steal_one: built twice")),
    });
    &s[0]
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::*;

    /// The host pmap, bootstrapped once per test process with a provider whose page system is
    /// never up (`testutil::pmap_boot`).
    fn booted() {
        let _ = crate::testutil::pmap_boot();
    }

    #[test]
    fn a_slice_is_built_in_place_and_shared() {
        booted();
        let s = steal_slice(1000, |i| (i as u64) * 3);
        assert_eq!(s.len(), 1000);
        assert!(s.iter().enumerate().all(|(i, &v)| v == i as u64 * 3));
        assert_eq!(s.as_ptr().addr() % PAGE_SIZE, 0, "page-aligned");
        let t = std::thread::spawn(move || s[999]);
        assert_eq!(t.join().unwrap(), 2997);
    }

    #[test]
    fn one_value_and_the_empty_slice() {
        booted();
        let v = steal_one(|| [7u8; 3]);
        assert_eq!(*v, [7, 7, 7]);
        let e: &[u32] = steal_slice(0, |_| 1);
        assert!(e.is_empty());
        let words: Vec<u32> = steal_slice(3, |i| i as u32 + 1).to_vec();
        assert_eq!(words, [1, 2, 3]);
    }

    #[test]
    #[should_panic(expected = "a type of size 0")]
    fn a_zero_sized_type_is_refused() {
        booted();
        let _ = steal_slice(4, |_| ());
    }
}
/* </TESTS> */
