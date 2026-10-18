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
//! `KmBuf`: an owned run of kernel pages, `km_alloc(9)` to `km_free(9)` as a value.
//!
//! The C keeps a `km_alloc` result as a `void *` and a size beside it, and must `km_free`
//! both, with the same modes, on every path; LZ keeps the shape (`pipe_buffer.buffer`, a
//! `NonNull<u8>` and three `unsafe` slices over it in `sys_pipe.rs`). A `KmBuf` owns the
//! pages: it derefs to `[u8]` of its length, moves like any value (pipe(2) moves its buffer
//! out of the pair's lock while `uiomove` runs, decision 11), and its drop is the `km_free`.
//!
//! The pages come from the provider the caller passes, a [`PageMemory`]: the kernel's own
//! allocator, `kernel::vm::km` (`page_memory()`, N3.2), passed down as a value as the pmap's
//! page provider is (`PhysPages`, D16), never a registered slot. It is a safe trait (decision
//! 30): it returns a pointer and the size granted, and builds nothing unsafe. Before the
//! kernel map is up, the provider refuses (`None`), as `km_alloc` with `KD_NOWAIT` may.
//!
//! ## Redesign
//! - `km_alloc(len, &kv_any, &kp_pageable, &kd_waitok)` -> [`KmBuf::alloc_pageable`];
//!   `km_alloc(len, &kv_any, &kp_zero, &kd_*)` -> [`KmBuf::alloc_wired`]; the `km_free` with
//!   the same modes -> the drop. The modes the C passes beside the pointer are part of the
//!   value, so they cannot disagree.
//! - The slices over the pages rest on the provider's contract ([`PageMemory`]; the kernel's
//!   `km`, forbid-crate code since N3.2, keeps it; `sys/hal/trust.rs`): a documented trust
//!   boundary, not a compile-time proof, until the kernel map's record is typed (N3.5). `hal`
//!   checks every answer (page alignment, the size granted; a probe of the first and last
//!   byte under `diagnostic`) and panics naming the provider when one fails.
//! - N3.5 S0: [`KmBuf::leak_slice`] gives the run up for good as typed values that live for
//!   ever (`uvm_mapent_alloc` carving a `km_alloc`'d page into map entries it never frees:
//!   `hal::pool::StaticArena::extend`), as `bootmem::steal_slice` does with stolen memory.

use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::assert::{hal_panic, kassert};

use crate::trust::check_block;

/// What backs a [`KmBuf`]: the C's `kp_pageable` or `kp_zero` physical-page mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageKind {
    /// `&kp_pageable`: backed by an anonymous object, faulted in (zero-filled) on first
    /// touch, may be paged out; touching it may sleep.
    Pageable,
    /// `&kp_zero`: wired, zeroed pages; touching them never faults.
    Wired,
}

/// The kernel map's pages, as the kernel provides them (`km_alloc`/`km_free` with `&kv_any`,
/// `kernel::vm::km::page_memory()`), passed to [`KmBuf`]'s constructors as a value.
///
/// The provider's contract (`sys/hal/trust.rs`): `alloc(len, kind, wait)` returns `None` or
/// a pointer to the size granted (at least `len`) of zeroed kernel memory, page-aligned, that
/// nothing else references until `free` gets that pointer back with the same `len` and
/// `kind`; `page_size` is the machine's `PAGE_SIZE`. `hal` checks the alignment and the size
/// of every answer.
pub trait PageMemory: Sync {
    /// `km_alloc(len, &kv_any, kind's mode, wait ? &kd_waitok : &kd_nowait)`: the block and
    /// the size granted.
    fn alloc(&self, len: usize, kind: PageKind, wait: bool) -> Option<(NonNull<u8>, usize)>;
    /// `km_free(ptr, len, &kv_any, kind's mode)`: the contract's other half. The block given
    /// back is one this bridge granted, with its `len` and `kind`, and no view of it is
    /// alive. Only `KmBuf`'s drop calls it for a block `KmBuf` made.
    fn free(&self, ptr: NonNull<u8>, len: usize, kind: PageKind);
    /// `PAGE_SIZE`.
    fn page_size(&self) -> usize;
}

/// An owned run of kernel pages: `len` bytes, freed on drop.
///
/// Invariant: `ptr` came from `mem`'s `alloc(len, kind, _)`, passed [`check_block`]
/// (page-aligned, at least `len` bytes) and has not been freed; by the provider's contract
/// this value is the only reference to the memory, so it may hand out `&[u8]` and `&mut [u8]`
/// of it under the usual borrow rules.
pub struct KmBuf {
    ptr: NonNull<u8>,
    len: usize,
    kind: PageKind,
    mem: &'static dyn PageMemory,
}

impl KmBuf {
    /// `km_alloc(len, &kv_any, &kp_pageable, &kd_waitok)` from `mem`: `len` bytes (a multiple
    /// of the page size) of pageable memory, sleeping for it; `None` when the kernel map cannot
    /// hold it (or is not up yet).
    pub fn alloc_pageable(mem: &'static dyn PageMemory, len: usize) -> Option<KmBuf> {
        Self::alloc(mem, len, PageKind::Pageable, true)
    }

    /// `km_alloc(len, &kv_any, &kp_zero, wait ? &kd_waitok : &kd_nowait)` from `mem`: `len`
    /// bytes (a multiple of the page size) of wired, zeroed memory.
    pub fn alloc_wired(mem: &'static dyn PageMemory, len: usize, wait: bool) -> Option<KmBuf> {
        Self::alloc(mem, len, PageKind::Wired, wait)
    }

    fn alloc(
        mem: &'static dyn PageMemory,
        len: usize,
        kind: PageKind,
        wait: bool,
    ) -> Option<KmBuf> {
        kassert!(len != 0 && len.is_multiple_of(mem.page_size())); // km_alloc: sz == round_page(sz)
        let (ptr, granted) = mem.alloc(len, kind, wait)?;
        check_block("PageMemory", ptr, granted, len, mem.page_size());
        Some(KmBuf {
            ptr,
            len,
            kind,
            mem,
        })
    }

    /// The length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the run is empty (never, for a run `km_alloc` made).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// What backs the run.
    pub fn kind(&self) -> PageKind {
        self.kind
    }

    /// Gives the run up for good as `len / size_of::<T>()` values, element `i` set to
    /// `init(i)`: they live for ever and the pages are never freed. `T` must fit the page
    /// alignment (a larger alignment panics) and not be zero-sized (a compile-time error).
    pub fn leak_slice<T: 'static>(self, mut init: impl FnMut(usize) -> T) -> &'static [T] {
        const {
            assert!(
                size_of::<T>() != 0,
                "KmBuf::leak_slice of a zero-sized type"
            );
        }
        let this = ManuallyDrop::new(self);
        if align_of::<T>() > this.mem.page_size() {
            hal_panic(format_args!(
                "KmBuf::leak_slice: an alignment of {} past the page's",
                align_of::<T>()
            ));
        }
        let n = this.len / size_of::<T>();
        let base = this.ptr.as_ptr().cast::<T>();
        // SAFETY: `ptr` names `len` bytes of kernel memory that only this value references
        // and that live until `mem.free` (the provider's contract, checked page-aligned and
        // at least `len` bytes by `check_block`, `sys/hal/trust.rs`); the value is never
        // dropped (`ManuallyDrop`, not returned), so `free` is never called and the memory
        // lives for ever, referenced by nothing else. `base` is aligned for `T` (page-aligned,
        // and `T`'s alignment is at most the page's, checked above) and `n` values of `T` end
        // within the `len` bytes; each is written whole before the shared slice over the `n`
        // of them is made, and that slice is the only reference to the memory from now on.
        // A panic in `init` leaks the run and the values made so far; nothing reads them.
        unsafe {
            for i in 0..n {
                base.add(i).write(init(i));
            }
            core::slice::from_raw_parts(base, n)
        }
    }
}

impl Deref for KmBuf {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // SAFETY: `ptr` names at least `len` bytes (checked by `check_block` when the run was
        // made) of zeroed, initialised kernel memory that only this value references and that
        // lives until the drop: the contract of the `PageMemory` it came from, which the caller
        // of `alloc_*` chose (in a kernel image only `sys/kern/pipe_glue.rs`'s `PIPES`, with
        // the kernel's own `km`, which hands each block out once and takes it back only from
        // this drop; `sys/hal/trust.rs`), a documented trust boundary. The slice borrows
        // `self`, so it ends before the drop.
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl DerefMut for KmBuf {
    fn deref_mut(&mut self) -> &mut [u8] {
        // SAFETY: as for `deref`, and `&mut self` makes this the only reference to the bytes
        // while it lives.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for KmBuf {
    fn drop(&mut self) {
        self.mem.free(self.ptr, self.len, self.kind);
    }
}

// SAFETY: a `KmBuf` owns its bytes outright (no other reference exists, the invariant, which
// rests on the provider's contract) and its provider is `Sync`, so moving it to another CPU
// moves the only access to them.

unsafe impl Send for KmBuf {}

// SAFETY: `&KmBuf` gives only `&[u8]` (shared reads of plain bytes) and the length and kind,
// which never change; reads from several CPUs at once are sound.
unsafe impl Sync for KmBuf {}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn a_buffer_is_zeroed_writable_and_freed_on_drop() {
        testutil::install();
        let before = testutil::pages_out();
        let mut b = KmBuf::alloc_pageable(testutil::page_memory(), 2 * 4096).unwrap();
        assert_eq!(b.len(), 8192);
        assert_eq!(b.kind(), PageKind::Pageable);
        assert!(b.iter().all(|&x| x == 0));
        b[100] = 7;
        b[8191] = 9;
        assert_eq!((b[100], b[8191]), (7, 9));
        assert!(testutil::pages_out() > before);
        drop(b);
        assert_eq!(testutil::pages_out(), before);
    }

    #[test]
    fn a_buffer_moves_out_of_a_lock_and_back() {
        testutil::install();
        let l = crate::rwlock::Rwlocked::new(
            "test",
            KmBuf::alloc_wired(testutil::page_memory(), 4096, true),
        );
        let mut g = l.write();
        let mut mem = g.take().unwrap();
        g.unlocked(|| mem[0] = 1);
        *g = Some(mem);
        assert_eq!(g.as_ref().map(|m| m[0]), Some(1));
    }

    #[test]
    fn a_leaked_buffer_holds_its_values_for_ever() {
        testutil::install();
        let before = testutil::pages_out();
        let b = KmBuf::alloc_wired(testutil::page_memory(), 4096, true).unwrap();
        let v: &'static [u64] = b.leak_slice(|i| i as u64 * 3);
        assert_eq!(v.len(), 4096 / 8);
        assert!(v.iter().enumerate().all(|(i, &x)| x == i as u64 * 3));
        // Never freed.
        assert_eq!(testutil::pages_out(), before + 4096);
    }
}
/* </TESTS> */
