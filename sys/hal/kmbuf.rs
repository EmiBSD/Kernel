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
//! The pages come from `uvm_km` through the [`PageMemory`] bridge (bridges.toml; paid by N3,
//! when `kernel::vm` owns the kernel map). The bridge is an `unsafe trait`: what it returns
//! is memory this module hands out as `&mut [u8]`, so its implementer vouches for the
//! contract, as `GlobalAlloc`'s does.
//!
//! **Before registration** (early boot, host tests that register none) there is no kernel
//! map to take pages from: an allocation fails (`None`), as `km_alloc` with `KD_NOWAIT` may.
//!
//! ## Redesign
//! - `km_alloc(len, &kv_any, &kp_pageable, &kd_waitok)` -> [`KmBuf::alloc_pageable`];
//!   `km_alloc(len, &kv_any, &kp_zero, &kd_*)` -> [`KmBuf::alloc_wired`]; the `km_free` with
//!   the same modes -> the drop. The modes the C passes beside the pointer are part of the
//!   value, so they cannot disagree.

use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::assert::kassert;
use crate::bridges;

/// What backs a [`KmBuf`]: the C's `kp_pageable` or `kp_zero` physical-page mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageKind {
    /// `&kp_pageable`: backed by an anonymous object, faulted in (zero-filled) on first
    /// touch, may be paged out; touching it may sleep.
    Pageable,
    /// `&kp_zero`: wired, zeroed pages; touching them never faults.
    Wired,
}

/// The kernel map's pages, as `bsd` registers them (`km_alloc`/`km_free` of
/// `sys/uvm/uvm_km.rs`, with `&kv_any`).
///
/// # Safety
///
/// An implementation upholds, for every call: `alloc(len, kind, wait)` returns `None` or a
/// pointer to `len` bytes of kernel memory, aligned to the page size, zeroed, readable and
/// writable, that nothing else references until `free` is called with that pointer, the same
/// `len` and the same `kind`; `free` is called once per such pointer and takes the memory
/// back; `page_size` is the machine's `PAGE_SIZE`, constant.
pub unsafe trait PageMemory: Sync {
    /// `km_alloc(len, &kv_any, kind's mode, wait ? &kd_waitok : &kd_nowait)`.
    fn alloc(&self, len: usize, kind: PageKind, wait: bool) -> Option<NonNull<u8>>;
    /// `km_free(ptr, len, &kv_any, kind's mode)`.
    fn free(&self, ptr: NonNull<u8>, len: usize, kind: PageKind);
    /// `PAGE_SIZE`.
    fn page_size(&self) -> usize;
}

/// An owned run of kernel pages: `len` bytes, freed on drop.
///
/// Invariant: `ptr` came from the registered [`PageMemory`]'s `alloc(len, kind, _)` and has
/// not been freed; this value is the only reference to the memory (the trait's contract), so
/// it may hand out `&[u8]` and `&mut [u8]` of it under the usual borrow rules.
pub struct KmBuf {
    ptr: NonNull<u8>,
    len: usize,
    kind: PageKind,
    mem: &'static dyn PageMemory,
}

impl KmBuf {
    /// `km_alloc(len, &kv_any, &kp_pageable, &kd_waitok)`: `len` bytes (a multiple of the page
    /// size) of pageable memory, sleeping for it; `None` when the kernel map cannot hold it
    /// (or before the bridge is registered).
    pub fn alloc_pageable(len: usize) -> Option<KmBuf> {
        Self::alloc(len, PageKind::Pageable, true)
    }

    /// `km_alloc(len, &kv_any, &kp_zero, wait ? &kd_waitok : &kd_nowait)`: `len` bytes (a
    /// multiple of the page size) of wired, zeroed memory.
    pub fn alloc_wired(len: usize, wait: bool) -> Option<KmBuf> {
        Self::alloc(len, PageKind::Wired, wait)
    }

    fn alloc(len: usize, kind: PageKind, wait: bool) -> Option<KmBuf> {
        let mem = bridges::page_memory()?;
        kassert!(len != 0 && len.is_multiple_of(mem.page_size())); // km_alloc: sz == round_page(sz)
        let ptr = mem.alloc(len, kind, wait)?;
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
}

impl Deref for KmBuf {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // SAFETY: `ptr` names `len` zeroed, initialised bytes that only this value
        // references (the invariant, from `PageMemory`'s contract) and that live until the
        // drop; the slice borrows `self`, so it ends before the drop frees them.
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

// SAFETY: a `KmBuf` owns its bytes outright (no other reference exists, the invariant) and
// its bridge is `Sync`, so moving it to another CPU moves the only access to them.
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
        let mut b = KmBuf::alloc_pageable(2 * 4096).unwrap();
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
        let l = crate::rwlock::Rwlocked::new("test", KmBuf::alloc_wired(4096, true));
        let mut g = l.write();
        let mut mem = g.take().unwrap();
        g.unlocked(|| mem[0] = 1);
        *g = Some(mem);
        assert_eq!(g.as_ref().map(|m| m[0]), Some(1));
    }
}
/* </TESTS> */
