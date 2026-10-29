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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * All rights reserved.
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_pager.h,v 1.1.2.14 1998/01/13 19:00:50 chuck Exp
 */

/*
 * Copyright (c) 1990 University of Utah.
 * Copyright (c) 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)vm_pager.h	8.5 (Berkeley) 7/7/94
 */

/*
 * Copyright (c) 2007 Artur Grabowski <art@openbsd.org>
 * Copyright (c) 2012-2016,2019 Bob Beck <beck@openbsd.org>
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
//! pager's window, `uvm_pagermapin`; a buffer's window, `buf_map`), as a slice; its drop
//! removes the mappings.
//!
//! Upstream: sys/uvm/uvm_pager.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_pager.c @ 3ce1f3f79392
//! Upstream: sys/kern/vfs_biomem.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_pager.rs@3c62ede68802
//! LZ: sys/kern/vfs_biomem.rs@3c62ede68802
//!
//! The C's `uvm_pagermapin(pps, npages, flags)` enters each busy page at the next page of a
//! pager segment with `pmap_enter(pmap_kernel(), cva, pa, prot, PMAP_WIRED | PMAP_CANFAIL |
//! prot)` (or, for one page, returns its direct-map address), and returns the `vaddr_t` that
//! the vnode pager puts in an `iovec` and `memset`s the tail of; `uvm_pagermapout(kva,
//! npages)` removes the mappings (`pmap_remove`, or nothing for the direct map). Here
//! [`KernelMapper::map_frames`] enters the frames and returns a [`MappedBytes`], whose
//! [`MappedBytes::as_slice`] and [`MappedBytes::as_slice_mut`] are the bytes and whose drop
//! is the `pmap_remove`; [`KernelMapper::map_direct`] is the one-page direct-map case. The
//! pager segment the window sits in stays the caller's (`kernel::vm::pager`, N3.4c), who
//! releases it after the drop. The buffer cache's window (`vfs_biomem.c`'s `buf_map`:
//! `pmap_kenter_pa` of each of the buffer's pages at its kva slot, `PROT_READ | PROT_WRITE`)
//! is [`KernelMapper::kenter_frames`]: the same view over an unmanaged mapping, which
//! [`MappedBytes::truncate`] shortens (`buf_fix_mapping`) and whose drop is the `pmap_kremove`
//! (`buf_unmap`, `buf_dealloc_mem`).
//!
//! **What makes a view the only reference to its bytes.**
//! - Each frame is checked to be a managed page (the frame proof the C's `struct vm_page *`
//!   gave: RAM, never device memory nor boot-stolen memory) and a busy one: the page
//!   system answers `PG_BUSY` through the page provider (`PhysPages::is_busy`, the C's
//!   `KASSERT(pp->pg_flags & PG_BUSY)`), the mark of a page its owner holds for I/O. Pages
//!   `uvm_pglistalloc` hands out (`km_alloc`'s, so the pools', `malloc`'s, a `KmBuf`'s) are
//!   never busy, and a page-table page is unbusied when it is allocated, so none of those
//!   can be viewed.
//! - Each frame is *claimed* for the view: a bit of its `PageMd` that `hal` alone sets and
//!   clears (outside the `PG_PMAP*` bits), set by the constructor and cleared by the drop.
//!   A frame already held is `panic(9)`, the frames claimed so far released first; so two
//!   views of one frame cannot exist, whoever asks. The other ways to the frame's bytes
//!   refuse a held frame: `pmap_zero_page` and `pmap_copy_page` claim it for their length,
//!   `pmap_page_protect` spares the view's own mapping (removing the page's other mappings,
//!   which freeing it does, or write-protecting a writable view), the page system refuses to
//!   free it (`uvm_pageclean`), to hand it out again (`PageMd::clear_attrs`) and to unbusy
//!   it (`VmPage::clear_bits` of `PG_BUSY`), so the frames stay busy until the drop.
//! - A window is entered only where nothing is mapped (each page `pmap_extract`ed first), so
//!   no live window is overlaid by another.
//! - Both constructors are methods of the [`KernelMapper`], the kernel map's token: only its
//!   holders (the kernel's `vm::km`, and those it lends the token to: `bsd`'s
//!   `machine::pmap`, the pager) can make a view, not any kernel code. The window's
//!   addresses are the holder's (a pager segment it took).
//! - A buffer's window ([`KernelMapper::kenter_frames`]) is entered with `kenter`: unmanaged,
//!   on no pv chain, so neither `pmap_page_protect` nor the reference and modify emulation
//!   (`pmap_clear_reference`, `pmap_clear_modify`, which walk the pv chain) ever touch it; it
//!   leaves only by [`MappedBytes::truncate`], which shortens the view first, and by the drop.
//!   Its frames are the buffer's own pages, `uvm_pagealloc_multi`'s, busy (`PG_BUSY |
//!   PG_FAKE`) for the buffer's life, claimed for the view like a pager window's.
//! - **The token holder's contract** (what `hal` cannot check): no live window is overlaid,
//!   removed or write-protected through the kernel map's token but by its drop (or
//!   `truncate`), and no frame of a busy page that backs kernel memory outside a view is
//!   given to [`KernelMapper::map_frames`] or [`KernelMapper::map_direct`]. The buffer cache's
//!   pages are such pages while a buffer is unmapped (the buffer's dummy object, which no
//!   pager walks, so no pager window is made of them); a mapped buffer's are claimed by its
//!   window (`kenter_frames`), so a pager window over them panics. Until the cache maps its
//!   buffers through `kenter_frames` (N5.1's `kernel::fs::biomem`), `bsd`'s `vfs_biomem`
//!   maps them with the legacy `kenter` and reaches them by raw pointers, and `bsd`'s drivers
//!   reach a busy buffer's bytes by its address (`b_data`, their own `unsafe fn data`, N7).
//!   The token's ways in are `KernelMapper::enter`/`kenter` (an overlay), `remove`/`kremove`
//!   and `protect` on `pmap_kernel()`, and `bsd`'s safe `pmap_enter`, `pmap_remove` and
//!   `pmap_protect` on `pmap_kernel()` (`sys/machine/pmap.rs`, the legacy trust
//!   `kernel::vm::km` records for its lend).
//!
//! On a machine without an MMU (`PMAP_NOMMU`, only the host test double) a virtual address
//! maps nothing, so the window's bytes are read through the direct map (the identity there):
//! the frames must then be contiguous, which the host double's memory frames are, and
//! anything else is `panic(9)`; the mappings are still entered at the window's address, so
//! `pmap_extract` answers as on a machine.
//!
//! ## Deviations
//! - A byte view's own mapping is spared by `pmap::page_protect` with `PROT_NONE`
//!   (N3.4c review): in the C a multi-page pager window is a managed kernel entry (`uvm_pager.c` 292,
//!   `pmap_enter(pmap_kernel(), .., PMAP_WIRED | PMAP_CANFAIL)`) that a deactivation's
//!   `pmap_page_protect(PROT_NONE)` removes like any other (`uvn_flush`'s `PGO_DEACTIVATE`
//!   during `uvn_io`'s `VOP_WRITE`, `madvise(MADV_DONTNEED)` against `msync`), so the
//!   `VOP_WRITE`'s copy then faults on `kp_none` pager space: `EFAULT`, a kernel trap, a
//!   panic. Here the window keeps its entry until its own drop (`PageMd::spares`), and every
//!   other mapping of the page goes. Only the one-page case, the direct map, behaves as the C
//!   (it is on no pv chain there either).
//!
//! ## Redesign
//! - LZ's `uvm_pagermapin(pps, npages, flags) -> usize` and `uvm_pagermapout(kva, npages)`
//!   (`sys/uvm/uvm_pager.rs`) hand the window out as an address its callers cast into an
//!   `iovec` and write through. Their pmap half is here as a value: the mapping loop is
//!   [`KernelMapper::map_frames`] (on a failure the entered frames are removed and the error
//!   returned, so the pager retries the whole window after `uvm_wait`, where the C retries
//!   the page that failed: the same mappings in the end), the one-page case
//!   [`KernelMapper::map_direct`], the removal the drop. The segment handling, `UVMPAGER_*`
//!   and the retry stay with `uvm_pagermapin` in the pager (`bsd`'s `uvm_pager.rs` now, the
//!   kernel's `vm::pager` at N3.4c).
//! - N4.8a: the timekeep page (`kern_exec.c`'s `timekeep`, written by `kern_tc.c`) is a third
//!   view, [`KernelMapper::timekeep_view`]: one per boot, over a wired managed page whose
//!   frame it claims for good in its `PageMd` with a claim of its own (`PG_HAL_KEEP`: held
//!   like a window's, but not resting on `PG_BUSY`), so the rules above keep every other
//!   kernel way to its bytes away (the N4.8a review: the token's word alone was the proof
//!   before).
//! - N5.1: the mapping half of `vfs_biomem.c`'s `buf_map` (LZ: `pmap_kenter_pa` of each page
//!   through `bsd`'s legacy `unsafe fn`, the buffer's bytes then reached through the raw
//!   `b_data`) is [`KernelMapper::kenter_frames`], a fourth view over an unmanaged mapping,
//!   and `buf_fix_mapping`'s `pmap_kremove` of the tail is [`MappedBytes::truncate`]; the
//!   checks every window makes before it claims (`window_of`, `check_frames`, `claim_all`)
//!   are shared with `map_frames`. The arena's slots and the buffer's pages stay the kernel's
//!   (`kernel::vm::km`, `kernel::fs::biomem`).

use core::ptr;
use core::slice;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::current::param::PAGE_SIZE;
use crate::assert::hal_panic;
use crate::pmap::{
    self, HAVE_PMAP_DIRECT, KernelMapper, PMAP_CANFAIL, PMAP_NOMMU, PMAP_WIRED, PageMd, VmProt,
};
use crate::sys::errno::Errno;
use crate::sys::mman::{PROT_READ, PROT_WRITE};
use crate::sys::timetc::TimekeepAtomic;
use crate::sys::types::{Paddr, Vaddr, Vsize};

/// The bytes of frames mapped into kernel virtual space, `len` of them from [`MappedBytes::va`]
/// (the pager's window), as a slice; the drop removes the mappings it entered and gives the
/// frames' claims up.
///
/// Invariant: `view` is the address of `len` bytes (at most `isize::MAX`) of managed pages,
/// each claimed by this value alone (`PageMd::claim_view`), mapped for as long as it lives:
/// entered at `va` by the token in `kind` (its drop removes them) or through the direct map;
/// writable when `writable`.
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
    /// How the frames are mapped, and so how they are given back.
    kind: Kind<'m>,
}

/// How a [`MappedBytes`]' frames are mapped.
enum Kind<'m> {
    /// Entered at `va` by this token, managed and wired (`pmap_enter`): removed by the drop
    /// (`pmap_remove`), the frames found by `extract`.
    Window(&'m KernelMapper),
    /// Entered at `va` by this token, unmanaged (`pmap_kenter_pa`): removed by the drop
    /// (`pmap_kremove`), the frames found by `extract`.
    Unmanaged(&'m KernelMapper),
    /// The direct map of this one frame: nothing to remove.
    Direct(Paddr),
}

impl MappedBytes<'_> {
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
        // SAFETY: `view` names `len` bytes (at most `isize::MAX`, nonzero) of managed, busy
        // pages (checked when the view was made), so RAM, whose every byte is a valid `u8`.
        // They stay mapped while this value lives, or are restored by `pmap_fault_fixup`,
        // arm64's reference/modify emulation (`uvm_pagedeactivate`'s `pmap_clear_reference`
        // clears the access bit of every pv entry, the window's included; the next kernel
        // access faults and the fixup re-validates the entry with the protection its pted
        // keeps): the window was entered where nothing was
        // mapped and `pmap_page_protect` spares a held page's window entry (`PageMd::spares`,
        // recorded before the entry is made, the entry removed by the identity checked; the
        // direct map and an unmanaged window (`kenter_frames`), on no pv chain, never go, nor
        // does the reference and modify emulation reach them; `page_protect` refuses a `prot`
        // that would take a view's read access, or a writable one's write access, away). Each
        // frame is claimed by this value alone, so no other view exists,
        // `pmap_zero_page`/`pmap_copy_page` refuse it, and the page system can neither
        // unbusy, free nor hand it out again while the claim lives (it panics), so
        // the frames stay busy, their owner's, until the drop; no `&mut` through this value
        // is alive beside this `&` (`as_slice_mut` needs `&mut self`). Pages of
        // `uvm_pglistalloc` and the page tables are never busy, so no reference to that
        // kernel memory reaches the frames. `truncate` (by `&mut self`, so with no slice
        // alive) shortens `len` before it gives frames up and removes their entries, so `len`
        // never covers a page this value no longer holds. What `hal` cannot check is the
        // token holder's contract (the module's docs): no live window is overlaid, removed or
        // write-protected through the kernel map's token but by its drop or `truncate`, and
        // no frame of a busy page that backs kernel memory outside a view is given to
        // `map_frames` or `map_direct`. The slice borrows `self`, so it ends before the drop
        // unmaps and releases.
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
        // SAFETY: as for `as_slice`, and the mapping is writable (checked above, and
        // `pmap_page_protect` may not write-protect a writable view's page); `&mut self` makes
        // this the only reference to the bytes through this value while it lives, and the
        // claims keep every other view and copy away.
        unsafe { slice::from_raw_parts_mut(p, self.len) }
    }

    /// `uvm_pagermapout(kva, npages)`'s `pmap_remove`, now: the window's address, for the
    /// caller to give its segment back. The same as the drop.
    pub fn unmap(self) -> Vaddr {
        self.va
    }

    /// `buf_fix_mapping(bp, newsize)`: shortens the view to its first `new_len` bytes (a whole
    /// number of pages, at least one): the frames past it are given up and their entries
    /// removed (`pmap_kremove(va + newsize, len - newsize)`, a pager window's by
    /// `pmap_remove`), so the pages can go to another owner (`bread_cluster_callback`'s
    /// `uvm_pagerealloc` of the pages the other buffers of a cluster read). Nothing happens
    /// when `new_len` is the length. Panics on a `new_len` that is 0, longer than the view or
    /// not a whole number of pages.
    pub fn truncate(&mut self, new_len: usize) {
        if new_len == 0 || new_len > self.len || !new_len.is_multiple_of(PAGE_SIZE) {
            hal_panic(format_args!(
                "MappedBytes: a window of {:#x} bytes truncated to {new_len:#x}",
                self.len
            ));
        }
        if new_len == self.len {
            return;
        }
        let old = self.len;
        // The view stops covering the cut pages before they are given up.
        self.len = new_len;
        let cut = self.va + Vsize::new(new_len);
        match self.kind {
            Kind::Window(mapper) => {
                give_up(cut, old - new_len);
                mapper.remove(cut, self.va + Vsize::new(old));
            }
            Kind::Unmanaged(mapper) => {
                give_up(cut, old - new_len);
                mapper.kremove(cut, Vsize::new(old - new_len));
            }
            // A direct view is one page, which no `new_len` above shortens.
            Kind::Direct(_) => hal_panic(format_args!("MappedBytes: a direct view truncated")),
        }
    }
}

impl Drop for MappedBytes<'_> {
    /// `pmap_remove(pmap_kernel(), kva, kva + len)` for a window `map_frames` entered (the
    /// direct map has nothing to remove: `pmap_unmap_direct`), and the frames' claims given
    /// up once no reference into the view is left.
    fn drop(&mut self) {
        match self.kind {
            Kind::Direct(pa) => md("MappedBytes", pa).release_view(),
            Kind::Window(mapper) => {
                give_up(self.va, self.len);
                mapper.remove(self.va, self.va + Vsize::new(self.len));
            }
            Kind::Unmanaged(mapper) => {
                give_up(self.va, self.len);
                mapper.kremove(self.va, Vsize::new(self.len));
            }
        }
    }
}

/// Gives up the claims of the frames a window maps at `[va, va + len)`, found through the
/// window's own entries (`extract`), before the caller removes them. No reference into the
/// view is alive (the caller holds it mutably), so a frame claimed anew between the release
/// and the removal meets no reader through this window.
fn give_up(va: Vaddr, len: usize) {
    for i in 0..len / PAGE_SIZE {
        let cva = va + Vsize::new(i * PAGE_SIZE);
        match pmap::kernel().extract(cva) {
            Some(pa) => md("MappedBytes", pa).release_view(),
            None => hal_panic(format_args!(
                "MappedBytes: the window page at {:#x} lost its mapping",
                cva.as_usize()
            )),
        }
    }
}

impl KernelMapper {
    /// `uvm_pagermapin`'s loop: enters the managed pages `frames` at `va`, `va + PAGE_SIZE`,
    /// ... in the kernel pmap with `prot`, wired, allowed to fail (`PMAP_WIRED |
    /// PMAP_CANFAIL | prot`), and returns the view of their bytes. When an entry fails the
    /// frames entered so far are removed, the claims given up and the error returned
    /// (`ENOMEM`: the caller waits for memory and tries again, or gives its segment back).
    ///
    /// Exclusivity: each frame must be a busy managed page (`PG_BUSY`) and is claimed for
    /// this view alone; a frame already held (this one included, twice in `frames`) is
    /// `panic(9)`, after the claims made so far are given up. A page of the window already
    /// mapped is `panic(9)` before anything is claimed; the frames stay busy until the drop
    /// (unbusying, freeing or reallocating a viewed page panics). `va` is kernel virtual
    /// space the caller owns (a pager segment). The gap until N3.5 (the token holder's
    /// contract, the module's docs): no live window is overlaid, removed or write-protected
    /// through the kernel map's token but by its drop, and no frame is a busy page that backs
    /// kernel memory (the buffer cache's, busy for the buffer's life).
    /// Panics on no frame, `prot` without `PROT_READ` (the pager maps `PROT_READ`, with
    /// `PROT_WRITE` for a read into the pages), a window that is not page-aligned or wraps, a
    /// frame that is not a managed page or not busy, and on a machine without an MMU frames
    /// that are not contiguous.
    pub fn map_frames(
        &self,
        va: Vaddr,
        frames: &[Paddr],
        prot: VmProt,
    ) -> Result<MappedBytes<'_>, Errno> {
        let (first, len) = window_of("map_frames", va, frames);
        if prot & PROT_READ == 0 {
            hal_panic(format_args!("map_frames: prot {prot:#x} without PROT_READ"));
        }
        check_frames("map_frames", va, frames, first);
        let writable = prot & PROT_WRITE != 0;
        claim_all("map_frames", frames, writable);
        for (i, &pa) in frames.iter().enumerate() {
            let cva = va + Vsize::new(i * PAGE_SIZE);
            // The window's own entry, which `page_protect` spares (recorded before it exists).
            md("map_frames", pa).set_view_va(cva.as_usize());
            if let Err(e) = self.enter(cva, pa, prot, PMAP_WIRED | PMAP_CANFAIL | prot) {
                self.remove(va, cva);
                release(frames);
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
            writable,
            kind: Kind::Window(self),
        })
    }

    /// `buf_map`'s loop: enters the managed pages `frames` at `va`, `va + PAGE_SIZE`, ... in
    /// the kernel pmap unmanaged, read and write (`pmap_kenter_pa(va, pa, PROT_READ |
    /// PROT_WRITE)`: no pv entry, which `pmap_kenter_pa` cannot fail to make), and returns the
    /// writable view of their bytes; the drop removes them (`pmap_kremove`), and
    /// [`MappedBytes::truncate`] the tail (`buf_fix_mapping`).
    ///
    /// Exclusivity: as [`KernelMapper::map_frames`]: each frame must be a busy managed page
    /// (a buffer's own, `uvm_pagealloc_multi`'s `PG_BUSY | PG_FAKE`) and is claimed for this
    /// view alone; a frame already held (this one included, twice in `frames`) is
    /// `panic(9)`, after the claims made so far are given up. A page of the window already
    /// mapped is `panic(9)` before anything is claimed; the frames stay busy until the drop.
    /// `va` is kernel virtual space the caller owns (a kva slot of the buffer arena,
    /// `kernel::vm::km`). The token holder's contract (the module's docs): the window is
    /// removed only by the drop or `truncate`. Panics on no frame, a window that is not
    /// page-aligned or wraps, a frame that is not a managed page or not busy, and on a
    /// machine without an MMU frames that are not contiguous.
    pub fn kenter_frames(&self, va: Vaddr, frames: &[Paddr]) -> MappedBytes<'_> {
        let (first, len) = window_of("kenter_frames", va, frames);
        check_frames("kenter_frames", va, frames, first);
        claim_all("kenter_frames", frames, true);
        for (i, &pa) in frames.iter().enumerate() {
            self.kenter(va + Vsize::new(i * PAGE_SIZE), pa, PROT_READ | PROT_WRITE);
        }
        let view = if PMAP_NOMMU {
            pmap::direct_map(first).as_usize()
        } else {
            va.as_usize()
        };
        MappedBytes {
            va,
            view,
            len,
            writable: true,
            kind: Kind::Unmanaged(self),
        }
    }

    /// `pmap_map_direct(pg)`: the one managed page `pa` through the direct map, read and
    /// write (`uvm_pagermapin` with one page and no aliasing risk).
    ///
    /// Exclusivity: as [`KernelMapper::map_frames`]: the frame must be a busy managed page
    /// and is claimed for this view alone (a frame already held is `panic(9)`); it stays busy
    /// until the drop. The gap until N3.5 (the token holder's contract, the module's docs):
    /// no frame is a busy page that backs kernel memory (the buffer cache's, busy for the
    /// buffer's life). Panics on a machine without a direct map (none lacks one: amd64,
    /// arm64 by LZ's deviation, the host double) and when `pa` is not a busy managed page.
    pub fn map_direct(&self, pa: Paddr) -> MappedBytes<'_> {
        if !HAVE_PMAP_DIRECT {
            hal_panic(format_args!("map_direct: no direct map"));
        }
        if !busy_md("map_direct", pa).claim_view(true) {
            hal_panic(format_args!(
                "map_direct: the frame {:#x} is in another view",
                pa.as_usize()
            ));
        }
        let va = pmap::direct_map(pa);
        MappedBytes {
            va,
            view: va.as_usize(),
            len: PAGE_SIZE,
            writable: true,
            kind: Kind::Direct(pa),
        }
    }
}

/// The `PageMd` of `pa`, which must be a page's address the page provider knows as a managed
/// page; `panic(9)` naming `what` otherwise.
fn md(what: &str, pa: Paddr) -> &'static PageMd {
    let md = pa
        .as_usize()
        .is_multiple_of(PAGE_SIZE)
        .then(|| pmap::kernel().pages().md_of(pa))
        .flatten();
    match md {
        Some(md) => md,
        None => hal_panic(format_args!(
            "{what}: {:#x} is not a managed page",
            pa.as_usize()
        )),
    }
}

/// The `PageMd` of `pa`, which must also be busy (`PG_BUSY`, `uvm_pagermapin`'s
/// `KASSERT(pp->pg_flags & PG_BUSY)`); `panic(9)` naming `what` otherwise.
fn busy_md(what: &str, pa: Paddr) -> &'static PageMd {
    let md = md(what, pa);
    if !pmap::kernel().pages().is_busy(pa) {
        hal_panic(format_args!(
            "{what}: {:#x} is not a busy page",
            pa.as_usize()
        ));
    }
    md
}

/// Gives up the claims of `frames` (all claimed by the caller).
fn release(frames: &[Paddr]) {
    for &pa in frames {
        md("map_frames", pa).release_view();
    }
}

/// The first frame of a window of `frames` at `va` and its length in bytes; `panic(9)` naming
/// `what` for no frame, a window that is not page-aligned, wraps, or is longer than
/// `isize::MAX`.
fn window_of(what: &str, va: Vaddr, frames: &[Paddr]) -> (Paddr, usize) {
    let Some(&first) = frames.first() else {
        hal_panic(format_args!("{what}: no frame"));
    };
    let len = frames
        .len()
        .checked_mul(PAGE_SIZE)
        .filter(|&l| l <= isize::MAX.unsigned_abs())
        .filter(|&l| va.as_usize().checked_add(l).is_some());
    let Some(len) = len.filter(|_| va.as_usize().is_multiple_of(PAGE_SIZE)) else {
        hal_panic(format_args!(
            "{what}: a window of {} pages at {:#x}",
            frames.len(),
            va.as_usize()
        ));
    };
    (first, len)
}

/// Checks, before anything is claimed, that every frame is a busy managed page (contiguous
/// from `first` on a machine without an MMU) and that no page of the window at `va` is mapped;
/// `panic(9)` naming `what` otherwise.
fn check_frames(what: &str, va: Vaddr, frames: &[Paddr], first: Paddr) {
    for (i, &pa) in frames.iter().enumerate() {
        let _ = busy_md(what, pa);
        if PMAP_NOMMU && pa.as_usize() != first.as_usize() + i * PAGE_SIZE {
            hal_panic(format_args!(
                "{what}: frames not contiguous on a machine without an MMU"
            ));
        }
        let cva = va + Vsize::new(i * PAGE_SIZE);
        if pmap::kernel().extract(cva).is_some() {
            hal_panic(format_args!(
                "{what}: the window page at {:#x} is mapped already",
                cva.as_usize()
            ));
        }
    }
}

/// Claims every frame for one view (`writable` when it may write); on a frame already held,
/// gives up the claims made so far and `panic(9)`s naming `what`.
fn claim_all(what: &str, frames: &[Paddr], writable: bool) {
    for (i, &pa) in frames.iter().enumerate() {
        if !md(what, pa).claim_view(writable) {
            release(&frames[..i]);
            hal_panic(format_args!(
                "{what}: the frame {:#x} is in another view",
                pa.as_usize()
            ));
        }
    }
}

/// Whether [`KernelMapper::timekeep_view`] has handed its one view out.
static TIMEKEEP_VIEWED: AtomicBool = AtomicBool::new(false);

impl KernelMapper {
    /// The page shared with userland for the timehands (`struct timekeep`), viewed at `va` as
    /// the kernel writes it (`hal::sys::timetc::TimekeepAtomic`): `exec_timekeep_map`'s
    /// first call maps the timekeep object into the kernel map, wires it, and hands the
    /// address over (N4.8a; LZ wrote it through a raw `&mut` of the address).
    ///
    /// What makes the view the only kernel reference to the page's bytes, for ever:
    /// - `None` for an address that is not page-aligned, not mapped in the kernel pmap, whose
    ///   frame is not a managed page (device or boot-stolen memory) or not wired
    ///   (`wire_count` 0: the page daemon could take it), and for every call after the first
    ///   that succeeded: the view is made once per boot.
    /// - The frame is claimed for good (`PageMd::claim_for_good`, writable) and never given
    ///   back: a frame some byte view, copy or zeroing holds already is `panic(9)` (so is a
    ///   second call racing the first on the same frame: it finds the frame claimed), and
    ///   from now on the other ways to its bytes refuse it (`map_direct` and `map_frames`
    ///   panic, `pmap_zero_page` and `pmap_copy_page` refuse it, the page system will not
    ///   free it nor hand it out again), while `pmap_page_protect` spares the view's own
    ///   mapping at `va` (recorded as the claim's address) and removes only the others
    ///   (userland's read-only ones) or panics before write-protecting it. Unlike a pager
    ///   window's, this claim does not rest on `PG_BUSY`: the fault path that maps the page
    ///   into a process ends by clearing a `PG_BUSY` it never set (`uvm_fault_lower`, the
    ///   page from the locked `uao_get`), and only the unlocked `uao_get` would busy it;
    ///   neither changes its bytes nor its owner, so the page system lets both
    ///   (`PageMd::in_busy_view` is false for this claim).
    ///
    /// The token holder's contract that stays (the module's docs): `va` is the kernel map's
    /// mapping of the timekeep object, never removed through the token (its entry is the
    /// kernel map's, which never unmaps it).
    pub fn timekeep_view(&self, va: Vaddr) -> Option<&'static TimekeepAtomic> {
        let addr = va.as_usize();
        if addr == 0 || !addr.is_multiple_of(PAGE_SIZE) {
            return None;
        }
        let pa = pmap::kernel().extract(va)?;
        let pages = pmap::kernel().pages();
        let md = pages.md_of(pa)?;
        if !pages.is_wired(pa) {
            return None;
        }
        if TIMEKEEP_VIEWED.load(Ordering::Acquire) {
            return None;
        }
        if !md.claim_for_good() {
            hal_panic(format_args!(
                "timekeep_view: the frame {:#x} is in another view",
                pa.as_usize()
            ));
        }
        if TIMEKEEP_VIEWED.swap(true, Ordering::AcqRel) {
            // Another call, over another frame, made the view in the meantime (one over this
            // frame would have found it claimed above): the claim just taken goes back, and
            // the frame is as it was.
            md.release_view();
            return None;
        }
        // The view's own entry, which `page_protect` spares.
        md.set_view_va(addr);
        let view = if PMAP_NOMMU {
            pmap::direct_map(pa).as_usize()
        } else {
            addr
        };
        let p = ptr::with_exposed_provenance::<TimekeepAtomic>(view);
        // SAFETY: the invariant: `view` addresses `size_of::<TimekeepAtomic>()` (88) bytes of
        // one managed, wired page that no other kernel reference reaches, for `'static`.
        // Established here: `addr` is page-aligned (checked) and the kernel pmap maps it to
        // `pa` (checked: `extract`), so `view` (that address, or `pa`'s direct-map address on a
        // machine without an MMU, the same frame) is page-aligned, hence aligned for
        // `TimekeepAtomic` (8), and its 88 bytes lie in the one page (`size_of` <= `PAGE_SIZE`,
        // asserted in `sys/timetc.rs`); the frame is a managed page (checked: `md_of`, so RAM,
        // never device memory) and wired (checked: `is_wired`); this is the one view
        // (`TIMEKEEP_VIEWED`, swapped once), and the frame is claimed for it and never
        // released. Upheld: the claim makes the page system refuse to free or hand out the
        // frame (`uvm_pageclean`, `PageMd::clear_attrs` panic on a claimed page), so the
        // bytes stay this page's for `'static` (busying and unbusying it, which the claim
        // allows: the fault's clear of a `PG_BUSY` it never set, an unlocked `uao_get`'s busy,
        // move neither the bytes nor the page); `map_direct`, `map_frames`, `zero_page` and
        // `copy_page` refuse a claimed frame, so no other kernel view or copy reaches them;
        // `page_protect` spares the mapping at `addr`, and the token holder never removes the
        // kernel map's entry (the contract above). Why the shared reference cannot race:
        // every field is an atomic integer, so any bit pattern (a fresh object's zeroes) is a
        // valid value and every access goes through the atomics' interior mutability;
        // userland reads the page through read-only mappings, outside Rust's model.
        Some(unsafe { &*p })
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::sys::mman::PROT_NONE;
    use crate::testutil;

    /// The kernel map's token of this test process, and a window of a test's own (`slot`
    /// apart from the other tests' windows and the pmap tests' addresses).
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
        // The frames hold what the window wrote, and its claims are gone: the second one,
        // through the direct map.
        let d = mapper.map_direct(frames[1]);
        assert_eq!((d.as_slice()[0], d.len()), (7, PAGE_SIZE));
    }

    #[test]
    fn a_read_only_window_reads() {
        let (mapper, va) = window(1);
        let frames = testutil::memory_frames(1);
        mapper.map_direct(frames[0]).as_slice_mut()[..3].copy_from_slice(b"abc");
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
    #[should_panic(expected = "is in another view")]
    fn a_frame_is_not_viewed_twice_directly() {
        let (mapper, _) = window(6);
        let frames = testutil::memory_frames(1);
        let _a = mapper.map_direct(frames[0]);
        let _b = mapper.map_direct(frames[0]);
    }

    #[test]
    fn a_window_over_a_viewed_frame_panics_and_gives_its_claims_back() {
        let (mapper, va) = window(7);
        let frames = testutil::memory_frames(2);
        let held = mapper.map_direct(frames[1]);
        let refused = catch_unwind(AssertUnwindSafe(|| {
            let _ = mapper.map_frames(va, &frames, PROT_READ);
        }));
        assert!(refused.is_err());
        // Nothing was entered, and the first frame's claim was given back.
        assert_eq!(pmap::kernel().extract(va), None);
        drop(mapper.map_direct(frames[0]));
        drop(held);
        // Both free again: one window over the two.
        drop(mapper.map_frames(va, &frames, PROT_READ).unwrap());
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
        let (mapper, _) = window(9);
        let _ = mapper.map_direct(Paddr::new(0x30_0000));
    }

    #[test]
    #[should_panic(expected = "is not a busy page")]
    fn a_frame_that_is_not_busy_is_refused() {
        let (mapper, va) = window(10);
        let frames = testutil::memory_frames(1);
        testutil::set_idle(frames[0]);
        let _ = mapper.map_frames(va, &frames, PROT_READ | PROT_WRITE);
    }

    #[test]
    #[should_panic(expected = "is not a busy page")]
    fn the_direct_view_refuses_a_frame_that_is_not_busy() {
        let (mapper, _) = window(11);
        let frames = testutil::memory_frames(1);
        testutil::set_idle(frames[0]);
        let _ = mapper.map_direct(frames[0]);
    }

    #[test]
    #[should_panic(expected = "is mapped already")]
    fn a_window_over_a_live_window_is_refused() {
        let (mapper, va) = window(12);
        let a = testutil::memory_frames(1);
        let b = testutil::memory_frames(1);
        let _first = mapper.map_frames(va, &a, PROT_READ | PROT_WRITE).unwrap();
        let _second = mapper.map_frames(va, &b, PROT_READ | PROT_WRITE);
    }

    #[test]
    #[should_panic(expected = "without PROT_READ")]
    fn a_window_without_prot_read_is_refused() {
        let (mapper, va) = window(13);
        let frames = testutil::memory_frames(1);
        let _ = mapper.map_frames(va, &frames, PROT_NONE);
    }

    #[test]
    #[should_panic(expected = "without PROT_READ")]
    fn a_write_only_window_is_refused() {
        let (mapper, va) = window(14);
        let frames = testutil::memory_frames(1);
        let _ = mapper.map_frames(va, &frames, PROT_WRITE);
    }

    #[test]
    #[should_panic(expected = "is in a byte view")]
    fn zero_page_refuses_a_viewed_frame() {
        let (mapper, _) = window(15);
        let frames = testutil::memory_frames(1);
        let _v = mapper.map_direct(frames[0]);
        pmap::zero_page(frames[0]);
    }

    #[test]
    #[should_panic(expected = "is in a byte view")]
    fn copy_page_refuses_a_viewed_destination() {
        let (mapper, _) = window(16);
        let frames = testutil::memory_frames(2);
        let _v = mapper.map_direct(frames[1]);
        pmap::copy_page(frames[0], frames[1]);
    }

    #[test]
    fn zero_and_copy_give_their_hold_back() {
        let (mapper, _) = window(17);
        let frames = testutil::memory_frames(2);
        pmap::zero_page(frames[0]);
        pmap::copy_page(frames[0], frames[1]);
        drop(mapper.map_direct(frames[0]));
        drop(mapper.map_direct(frames[1]));
    }

    #[test]
    fn removing_a_viewed_pages_mappings_spares_the_window() {
        let (mapper, va) = window(18);
        let frames = testutil::memory_frames(1);
        let w = mapper
            .map_frames(va, &frames, PROT_READ | PROT_WRITE)
            .unwrap();
        // What a deactivation does to a busy page under I/O (`uvn_flush`'s PGO_DEACTIVATE):
        // every mapping removed but the window's own, which keeps its bytes.
        let md = pmap::kernel().pages().md_of(frames[0]).unwrap();
        assert!(md.spares(true, va.as_usize()));
        pmap::page_protect(md, PROT_NONE);
        assert_eq!(pmap::kernel().extract(va), Some(frames[0]));
        assert_eq!(w.as_slice().len(), PAGE_SIZE);
        drop(w);
        assert!(!md.spares(true, va.as_usize()));
    }

    #[test]
    #[should_panic(expected = "handed out again")]
    fn a_viewed_page_is_not_handed_out_again() {
        let (mapper, _) = window(19);
        let frames = testutil::memory_frames(1);
        let _v = mapper.map_direct(frames[0]);
        pmap::kernel()
            .pages()
            .md_of(frames[0])
            .unwrap()
            .clear_attrs();
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

    /// The timekeep view's refusals and its one success share this test: the view is made
    /// once per process (`TIMEKEEP_VIEWED`), so no other test may make it.
    #[test]
    fn the_timekeep_view_is_made_once_over_a_wired_managed_page() {
        let (mapper, va) = window(20);
        let rw = PROT_READ | PROT_WRITE;
        assert!(
            mapper.timekeep_view(va + Vsize::new(8)).is_none(),
            "not page-aligned"
        );
        assert!(mapper.timekeep_view(va).is_none(), "nothing mapped");
        // A frame that is not a managed page (device memory, say).
        mapper.kenter(va, Paddr::new(0x30_0000), rw);
        assert!(mapper.timekeep_view(va).is_none(), "not a managed page");
        mapper.kremove(va, Vsize::new(PAGE_SIZE));
        // A managed page that is not wired.
        let unwired = testutil::memory_frames(1);
        mapper.enter(va, unwired[0], rw, PMAP_WIRED | rw).unwrap();
        assert!(mapper.timekeep_view(va).is_none(), "not wired");
        mapper.remove(va, va + Vsize::new(PAGE_SIZE));
        // A wired page another view holds: `panic(9)`, and the one view is still to be made.
        let (_, held_va) = window(22);
        let held = testutil::memory_frames(1);
        testutil::set_wired(held[0]);
        mapper.enter(held_va, held[0], rw, PMAP_WIRED | rw).unwrap();
        let direct = mapper.map_direct(held[0]);
        let held_md = pmap::kernel().pages().md_of(held[0]).unwrap();
        assert!(held_md.in_busy_view(), "a direct view rests on PG_BUSY");
        let refused = catch_unwind(AssertUnwindSafe(|| mapper.timekeep_view(held_va)));
        assert!(refused.is_err(), "a frame in another view");
        drop(direct);

        // The timekeep page: managed, wired, mapped in the kernel pmap.
        let frames = testutil::memory_frames(1);
        testutil::set_wired(frames[0]);
        mapper.enter(va, frames[0], rw, PMAP_WIRED | rw).unwrap();
        let tk = mapper.timekeep_view(va).unwrap();
        tk.tk_version.store(7, Ordering::Relaxed);
        // The view is the frame's own bytes (the host double has no MMU: its direct map).
        assert_eq!(
            core::ptr::from_ref(tk).addr(),
            pmap::direct_map(frames[0]).as_usize()
        );

        // Once: a second call, even over another wired page, has no view.
        assert!(mapper.timekeep_view(va).is_none(), "the same page again");
        let (_, other_va) = window(21);
        let other = testutil::memory_frames(1);
        testutil::set_wired(other[0]);
        mapper
            .enter(other_va, other[0], rw, PMAP_WIRED | rw)
            .unwrap();
        assert!(mapper.timekeep_view(other_va).is_none(), "another page");
        let other_md = pmap::kernel().pages().md_of(other[0]).unwrap();
        assert!(!other_md.in_view(), "a refused call claims nothing");

        // The frame is held for good: no direct view, no zeroing, never handed out again,
        // and removing its mappings spares the view's own.
        let md = pmap::kernel().pages().md_of(frames[0]).unwrap();
        assert!(catch_unwind(AssertUnwindSafe(|| drop(mapper.map_direct(frames[0])))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| pmap::zero_page(frames[0]))).is_err());
        let dst = testutil::memory_frames(1);
        assert!(catch_unwind(AssertUnwindSafe(|| pmap::copy_page(frames[0], dst[0]))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| md.clear_attrs())).is_err());
        assert!(md.in_view() && md.spares(true, va.as_usize()));
        // The claim is for good but not on PG_BUSY: the page system may busy and unbusy it.
        assert!(!md.in_busy_view());
        pmap::page_protect(md, PROT_NONE);
        assert_eq!(pmap::kernel().extract(va), Some(frames[0]));
        assert_eq!(tk.tk_version.load(Ordering::Relaxed), 7);
    }

    /// Whether `pa` is held by a view now (a direct view of it is refused).
    fn held(mapper: &KernelMapper, pa: Paddr) -> bool {
        catch_unwind(AssertUnwindSafe(|| drop(mapper.map_direct(pa)))).is_err()
    }

    #[test]
    fn a_buffer_window_is_entered_unmanaged_and_kremoved() {
        let (mapper, va) = window(30);
        let frames = testutil::memory_frames(3);
        let mut w = mapper.kenter_frames(va, &frames);
        assert_eq!(
            (w.va(), w.len(), w.is_writable()),
            (va, 3 * PAGE_SIZE, true)
        );
        for (i, &pa) in frames.iter().enumerate() {
            assert_eq!(
                pmap::kernel().extract(va + Vsize::new(i * PAGE_SIZE)),
                Some(pa)
            );
            assert!(held(mapper, pa));
        }
        w.as_slice_mut()[2 * PAGE_SIZE] = 9;
        drop(w);
        assert_eq!(pmap::kernel().extract(va), None);
        assert_eq!(pmap::kernel().extract(va + Vsize::new(2 * PAGE_SIZE)), None);
        // The claims are gone and the bytes stayed with the frames.
        assert!(frames.iter().all(|&pa| !held(mapper, pa)));
        assert_eq!(mapper.map_direct(frames[2]).as_slice()[0], 9);
    }

    #[test]
    fn truncate_gives_the_tail_up_and_keeps_the_head() {
        let (mapper, va) = window(31);
        let frames = testutil::memory_frames(3);
        let mut w = mapper.kenter_frames(va, &frames);
        w.as_slice_mut()[..4].copy_from_slice(b"head");
        w.truncate(3 * PAGE_SIZE);
        assert_eq!(w.len(), 3 * PAGE_SIZE);
        w.truncate(PAGE_SIZE);
        assert_eq!((w.len(), w.as_slice().len()), (PAGE_SIZE, PAGE_SIZE));
        assert_eq!(&w.as_slice()[..4], b"head");
        assert_eq!(pmap::kernel().extract(va), Some(frames[0]));
        assert_eq!(pmap::kernel().extract(va + Vsize::new(PAGE_SIZE)), None);
        assert_eq!(pmap::kernel().extract(va + Vsize::new(2 * PAGE_SIZE)), None);
        assert!(held(mapper, frames[0]));
        assert!(!held(mapper, frames[1]) && !held(mapper, frames[2]));
        drop(w);
        assert!(!held(mapper, frames[0]));
        assert_eq!(pmap::kernel().extract(va), None);
    }

    #[test]
    fn a_pager_window_truncates_too() {
        let (mapper, va) = window(32);
        let frames = testutil::memory_frames(2);
        let mut w = mapper
            .map_frames(va, &frames, PROT_READ | PROT_WRITE)
            .unwrap();
        w.truncate(PAGE_SIZE);
        assert_eq!(pmap::kernel().extract(va + Vsize::new(PAGE_SIZE)), None);
        assert!(!held(mapper, frames[1]));
        drop(w);
        assert!(!held(mapper, frames[0]));
    }

    #[test]
    #[should_panic(expected = "truncated to")]
    fn truncate_to_nothing_is_refused() {
        let (mapper, va) = window(33);
        let frames = testutil::memory_frames(1);
        let mut w = mapper.kenter_frames(va, &frames);
        w.truncate(0);
    }

    #[test]
    #[should_panic(expected = "truncated to")]
    fn truncate_to_part_of_a_page_is_refused() {
        let (mapper, va) = window(34);
        let frames = testutil::memory_frames(2);
        let mut w = mapper.kenter_frames(va, &frames);
        w.truncate(PAGE_SIZE + 1);
    }

    #[test]
    fn a_buffer_window_over_a_viewed_frame_panics_and_gives_its_claims_back() {
        let (mapper, va) = window(35);
        let frames = testutil::memory_frames(2);
        let pinned = mapper.map_direct(frames[1]);
        let refused = catch_unwind(AssertUnwindSafe(|| {
            drop(mapper.kenter_frames(va, &frames));
        }));
        assert!(refused.is_err());
        assert_eq!(pmap::kernel().extract(va), None);
        assert!(!held(mapper, frames[0]));
        drop(pinned);
        drop(mapper.kenter_frames(va, &frames));
    }

    #[test]
    #[should_panic(expected = "is not a busy page")]
    fn a_buffer_window_refuses_a_frame_that_is_not_busy() {
        let (mapper, va) = window(36);
        let frames = testutil::memory_frames(1);
        testutil::set_idle(frames[0]);
        let _ = mapper.kenter_frames(va, &frames);
    }
}
/* </TESTS> */
