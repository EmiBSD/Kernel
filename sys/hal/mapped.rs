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
//! `MappedBytes`: the bytes of physical pages mapped into kernel virtual space for I/O (the
//! pager's window, `uvm_pagermapin`), as a slice; its drop removes the mappings.
//!
//! The C's `uvm_pagermapin(pps, npages, flags)` enters each busy page at the next page of a
//! pager segment with `pmap_enter(pmap_kernel(), cva, pa, prot, PMAP_WIRED | PMAP_CANFAIL |
//! prot)` (or, for one page, returns its direct-map address), and returns the `vaddr_t` that
//! the vnode pager puts in an `iovec` and `memset`s the tail of; `uvm_pagermapout(kva,
//! npages)` removes the mappings (`pmap_remove`, or nothing for the direct map). LZ keeps the
//! shape: an address its callers cast. Here [`KernelMapper::map_frames`] enters the frames and
//! returns a [`MappedBytes`], whose [`MappedBytes::as_slice`] and
//! [`MappedBytes::as_slice_mut`] are the bytes and whose drop is the `pmap_remove`;
//! [`MappedBytes::direct`] is the one-page direct-map case. The pager segment the window
//! sits in stays the caller's (`kernel::vm::pager`, N3.4c), who releases it after the drop.
//!
//! **The trust boundary**, as `KmBuf`'s (`sys/hal/trust.rs`): every frame is checked to be a
//! managed page (the frame proof the C's `struct vm_page *` gave: RAM, never device memory)
//! and the window's address to be page-aligned; that the window's kernel virtual addresses
//! are the caller's (a pager segment it took) is the [`KernelMapper`]'s contract, and that
//! nothing else reads or writes the frames while the view lives is the caller's: they are its
//! busy pages (`PG_BUSY`), which only their owner touches, through this view. Until the pages
//! are typed (N3.5) both are the caller's word, documented here and at its call.
//!
//! On a machine without an MMU (`PMAP_NOMMU`, only the host test double) a virtual address
//! maps nothing, so the window's bytes are read through the direct map (the identity there):
//! the frames must then be contiguous, which the host double's memory frames are, and
//! anything else is `panic(9)`; the mappings are still entered at the window's address, so
//! `pmap_extract` answers as on a machine.
//!
//! ## Redesign
//! - Not a port of a file: `uvm_pagermapin`'s mapping loop and `uvm_pagermapout`'s removal
//!   (`sys/uvm/uvm_pager.c`) as a value; the segment handling and the `uvm_wait` retry stay
//!   in the pager (on a failure the entered frames are removed here and the error returned,
//!   so the caller retries the whole window, where the C retries the page that failed: the
//!   same mappings in the end).

use core::ptr;
use core::slice;

use crate::arch::current::param::PAGE_SIZE;
use crate::assert::hal_panic;
use crate::pmap::{
    self, HAVE_PMAP_DIRECT, KernelMapper, PMAP_CANFAIL, PMAP_NOMMU, PMAP_WIRED, VmProt,
};
use crate::sys::errno::Errno;
use crate::sys::mman::PROT_WRITE;
use crate::sys::types::{Paddr, Vaddr, Vsize};

/// The bytes of frames mapped into kernel virtual space, `len` of them from [`MappedBytes::va`]
/// (the pager's window), as a slice; the drop removes the mappings it entered.
///
/// Invariant: `view` is the address of `len` bytes (at most `isize::MAX`) of managed pages,
/// mapped for as long as this value lives: by the `mapper` that entered them at `va` (its
/// drop removes them), or by the direct map (`mapper` is `None`, nothing to remove); writable
/// when `writable`. By the caller's contract (the module's trust boundary) no other reference
/// to those bytes is made while the value lives.
pub struct MappedBytes<'m> {
    /// The window's address: where the frames were entered, or the direct-map address of the
    /// one frame.
    va: Vaddr,
    /// Where the bytes are read: `va`, or the direct map on a machine without an MMU.
    view: usize,
    /// The bytes in view, a whole number of pages.
    len: usize,
    /// Whether the frames were mapped `PROT_WRITE`.
    writable: bool,
    /// The kernel map's token that entered the frames at `va`; `None` for the direct map.
    mapper: Option<&'m KernelMapper>,
}

impl MappedBytes<'_> {
    /// `pmap_map_direct(pg)`: the one managed page `pa` through the direct map, read and
    /// write (`uvm_pagermapin` with one page and no aliasing risk). Panics on a machine
    /// without a direct map (none has one missing: amd64, arm64 by LZ's deviation, the host
    /// double), and when `pa` is not a managed page's address.
    pub fn direct(pa: Paddr) -> MappedBytes<'static> {
        if !HAVE_PMAP_DIRECT {
            hal_panic(format_args!("MappedBytes::direct: no direct map"));
        }
        managed_page("MappedBytes::direct", pa);
        let va = pmap::direct_map(pa);
        MappedBytes {
            va,
            view: va.as_usize(),
            len: PAGE_SIZE,
            writable: true,
            mapper: None,
        }
    }

    /// The window's address (`uvm_pagermapin`'s result): what the caller gives back to the
    /// pager's segments after the drop.
    pub fn va(&self) -> Vaddr {
        self.va
    }

    /// The bytes in view, a whole number of pages.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the view is empty (never: a window holds a page at least).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the frames were mapped writable (`UVMPAGER_MAPIN_READ`: a read from the
    /// backing store into the pages).
    pub fn is_writable(&self) -> bool {
        self.writable
    }

    /// The bytes (`iov_base = kva`, `iov_len = npages << PAGE_SHIFT` of a write to the backing
    /// store).
    pub fn as_slice(&self) -> &[u8] {
        let p = ptr::with_exposed_provenance::<u8>(self.view);
        // SAFETY: `view` names `len` bytes (at most `isize::MAX`, nonzero) of managed pages,
        // every frame checked by `managed_page`, so RAM, whose every byte is a valid `u8`;
        // they stay mapped while this value lives (the invariant: the token's holder owns the
        // window's addresses and removes them only through this value's drop, the direct map
        // never goes). No `&mut` to them exists beside this `&`: `as_slice_mut` needs `&mut
        // self`, and the frames are the caller's busy pages, which nothing else reads or
        // writes while the view lives (the module's trust boundary). The slice borrows `self`,
        // so it ends before the drop unmaps.
        unsafe { slice::from_raw_parts(p, self.len) }
    }

    /// The bytes, writable (`iov_base = kva` of a read from the backing store, and the
    /// `memset` of the tail past what it returned). Panics when the frames were mapped
    /// read-only (the C's write would fault).
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        if !self.writable {
            hal_panic(format_args!(
                "MappedBytes: a read-only window at {:#x} written",
                self.va.as_usize()
            ));
        }
        let p = ptr::with_exposed_provenance_mut::<u8>(self.view);
        // SAFETY: as for `as_slice`, and the mapping is writable (checked above); `&mut self`
        // makes this the only reference to the bytes through this value while it lives, and
        // the busy pages are reached through nothing else (the trust boundary).
        unsafe { slice::from_raw_parts_mut(p, self.len) }
    }

    /// `uvm_pagermapout(kva, npages)`'s `pmap_remove`, now: the window's address, for the
    /// caller to give its segment back. The same as the drop.
    pub fn unmap(self) -> Vaddr {
        self.va
    }
}

impl Drop for MappedBytes<'_> {
    /// `pmap_remove(pmap_kernel(), kva, kva + len)` for a window `map_frames` entered (the
    /// direct map has nothing to remove: `pmap_unmap_direct`).
    fn drop(&mut self) {
        if let Some(mapper) = self.mapper {
            mapper.remove(self.va, self.va + Vsize::new(self.len));
        }
    }
}

impl KernelMapper {
    /// `uvm_pagermapin`'s loop: enters the managed pages `frames` at `va`, `va + PAGE_SIZE`,
    /// ... in the kernel pmap with `prot`, wired, allowed to fail (`PMAP_WIRED |
    /// PMAP_CANFAIL | prot`), and returns the view of their bytes. When an entry fails the
    /// frames entered so far are removed and the error is returned (`ENOMEM`: the caller
    /// waits for memory and tries again, or gives its segment back). Panics on no frame, a
    /// window that is not page-aligned or wraps, a frame that is not a managed page, and on a
    /// machine without an MMU frames that are not contiguous (the module's docs).
    ///
    /// `va` must be kernel virtual space the caller owns (a pager segment): the token's
    /// contract.
    pub fn map_frames(
        &self,
        va: Vaddr,
        frames: &[Paddr],
        prot: VmProt,
    ) -> Result<MappedBytes<'_>, Errno> {
        let Some(&first) = frames.first() else {
            hal_panic(format_args!("map_frames: no frame"));
        };
        let len = frames
            .len()
            .checked_mul(PAGE_SIZE)
            .filter(|&l| l <= isize::MAX.unsigned_abs())
            .filter(|&l| va.as_usize().checked_add(l).is_some());
        let Some(len) = len.filter(|_| va.as_usize().is_multiple_of(PAGE_SIZE)) else {
            hal_panic(format_args!(
                "map_frames: a window of {} pages at {:#x}",
                frames.len(),
                va.as_usize()
            ));
        };
        for (i, &pa) in frames.iter().enumerate() {
            managed_page("map_frames", pa);
            if PMAP_NOMMU && pa.as_usize() != first.as_usize() + i * PAGE_SIZE {
                hal_panic(format_args!(
                    "map_frames: frames not contiguous on a machine without an MMU"
                ));
            }
        }
        for (i, &pa) in frames.iter().enumerate() {
            let cva = va + Vsize::new(i * PAGE_SIZE);
            if let Err(e) = self.enter(cva, pa, prot, PMAP_WIRED | PMAP_CANFAIL | prot) {
                self.remove(va, cva);
                return Err(e);
            }
        }
        let view = if PMAP_NOMMU {
            pmap::direct_map(first).as_usize()
        } else {
            va.as_usize()
        };
        Ok(MappedBytes {
            va,
            view,
            len,
            writable: prot & PROT_WRITE != 0,
            mapper: Some(self),
        })
    }
}

/// Panics unless `pa` is a page's address that the page provider knows as a managed page.
fn managed_page(what: &str, pa: Paddr) {
    if !pa.as_usize().is_multiple_of(PAGE_SIZE) || pmap::kernel().pages().md_of(pa).is_none() {
        hal_panic(format_args!(
            "{what}: {:#x} is not a managed page",
            pa.as_usize()
        ));
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::mman::PROT_READ;
    use crate::testutil;

    /// The kernel map's token of this test process, and a window `pages` long of a test's
    /// own (`slot` apart from the other tests' windows and the pmap tests' addresses).
    fn window(slot: usize) -> (&'static KernelMapper, Vaddr) {
        let mapper = &testutil::pmap_boot().mapper;
        let (start, _) = pmap::virtual_space();
        (
            mapper,
            start + Vsize::new((0x30_0000 + slot * 0x100) * PAGE_SIZE),
        )
    }

    #[test]
    fn a_window_maps_the_frames_and_its_bytes_are_theirs() {
        let (mapper, va) = window(0);
        let frames = testutil::memory_frames(2);
        let mut w = mapper
            .map_frames(va, &frames, PROT_READ | PROT_WRITE)
            .unwrap();
        assert_eq!(
            (w.va(), w.len(), w.is_writable()),
            (va, 2 * PAGE_SIZE, true)
        );
        assert_eq!(pmap::kernel().extract(va), Some(frames[0]));
        assert_eq!(
            pmap::kernel().extract(va + Vsize::new(PAGE_SIZE + 8)),
            Some(Paddr::new(frames[1].as_usize() + 8))
        );
        // A read from the backing store, then the tail zeroed (uvn_io).
        let bytes = w.as_slice_mut();
        bytes[..5].copy_from_slice(b"hello");
        bytes[PAGE_SIZE] = 7;
        bytes[PAGE_SIZE + 1..].fill(0);
        assert_eq!(&w.as_slice()[..5], b"hello");
        assert_eq!(w.unmap(), va);
        assert_eq!(pmap::kernel().extract(va), None);
        // The frames hold what the window wrote: the second, through the direct map.
        let d = MappedBytes::direct(frames[1]);
        assert_eq!((d.as_slice()[0], d.len()), (7, PAGE_SIZE));
    }

    #[test]
    fn a_read_only_window_reads() {
        let (mapper, va) = window(1);
        let frames = testutil::memory_frames(1);
        MappedBytes::direct(frames[0]).as_slice_mut()[..3].copy_from_slice(b"abc");
        let w = mapper.map_frames(va, &frames, PROT_READ).unwrap();
        assert!(!w.is_writable());
        assert_eq!(&w.as_slice()[..3], b"abc");
    }

    #[test]
    #[should_panic(expected = "read-only window")]
    fn a_read_only_window_is_not_written() {
        let (mapper, va) = window(2);
        let frames = testutil::memory_frames(1);
        let mut w = mapper.map_frames(va, &frames, PROT_READ).unwrap();
        w.as_slice_mut()[0] = 1;
    }

    #[test]
    #[should_panic(expected = "is not a managed page")]
    fn a_frame_that_is_not_a_managed_page_is_refused() {
        let (mapper, va) = window(3);
        let _ = mapper.map_frames(va, &[Paddr::new(0x30_0000)], PROT_READ);
    }

    #[test]
    #[should_panic(expected = "is not a managed page")]
    fn the_direct_view_refuses_a_frame_that_is_not_a_managed_page() {
        let _ = testutil::pmap_boot();
        let _ = MappedBytes::direct(Paddr::new(0x30_0000));
    }

    #[test]
    #[should_panic(expected = "a window of 1 pages")]
    fn a_window_that_is_not_page_aligned_is_refused() {
        let (mapper, va) = window(4);
        let frames = testutil::memory_frames(1);
        let _ = mapper.map_frames(va + Vsize::new(8), &frames, PROT_READ);
    }

    #[test]
    #[should_panic(expected = "not contiguous")]
    fn frames_apart_are_refused_without_an_mmu() {
        let (mapper, va) = window(5);
        let a = testutil::memory_frames(1);
        let b = testutil::memory_frames(1);
        let _ = mapper.map_frames(va, &[a[0], b[0]], PROT_READ);
    }
}
/* </TESTS> */
