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
 * Copyright (c) 2003-2004 Opsycon AB  (www.opsycon.se / www.opsycon.com)
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS
 * OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
 * DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */
/*-
 * Copyright (c) 1996, 1997, 1998 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jason R. Thorpe of the Numerical Aerospace Simulation Facility,
 * NASA Ames Research Center.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright (c) 2003-2004 Opsycon AB Sweden.  All rights reserved.
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
 */
/* </LICENSES> */

/* <CODE> */
//! Memory shared with a device: `bus_dmamem_alloc(9)`'s memory as a value, [`DmaBuf`], over a
//! bus's provider of it, [`DmaMemory`].
//!
//! Upstream: sys/arch/arm64/arm64/bus_dma.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/include/bus.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/bus_dma.rs@e8249449d905
//! LZ: sys/arch/arm64/include/bus.rs@e8249449d905
//!
//! A buffer is one segment of bus addresses ([`DmaSeg`]) mapped for the CPU, the provider's
//! (`bus_dmamem_alloc_range` of one segment, then `bus_dmamem_map`; the kernel's
//! `kernel::vm::km::bus_dma_memory` for a bus with no IOMMU) and given back to it by the drop
//! (`bus_dmamem_unmap`, then `bus_dmamem_free`). A device reads and writes those bytes while
//! the kernel holds the buffer, so no Rust reference to them is ever made, and no slice of
//! them handed out: the CPU reaches them only by volatile loads and stores ([`DmaBuf::read`],
//! [`DmaBuf::write`], [`DmaBuf::copy_out`], [`DmaBuf::copy_in`], [`DmaBuf::zero`]), every
//! range checked against the buffer, of types every bit pattern of which is a value
//! (`AbiPod`). A write takes `&mut self`: the owner (a virtqueue under its driver's lock) is
//! exclusive, as in the C. [`DmaBuf::sync`] is `bus_dmamap_sync` over the buffer's own bytes:
//! on a coherent bus a memory barrier, otherwise the cache operations of the C's
//! `_dmamap_sync_segment` (arm64's `dc civac`, `dc cvac`, `dc ivac`).
//!
//! The trust boundary (`sys/hal/mem/trust.rs`): that the block is mapped kernel memory holding no
//! Rust object, the frames its segment names, the provider's grant to this buffer alone, is
//! the provider's contract. `hal` checks the answer against the request: the mapping
//! page-aligned and as long as asked, the segment as long, aligned, inside the range and not
//! across the boundary the request names.
//!
//! ## Deviations
//! - One segment per buffer (`nsegs` 1), what the first kernel users ask (virtio's rings,
//!   `virtio.c`'s `bus_dmamem_alloc_range(.., 1, ..)`); a multi-segment `bus_dmamem_alloc` is
//!   `bsd`'s until a kernel driver needs one.
//! - `BUS_DMA_64BIT` travels in the request ([`DmaReq::dma64`]) for amd64's provider (its
//!   `no_constraint`); arm64's always allocates within `dma_constraint`, as the C.
//!
//! ## Redesign
//! - LZ's DMA memory is the C's: `_dmamem_alloc_range` fills a `bus_dma_segment_t` array,
//!   `_dmamem_map` returns a `NonNull<u8>` the driver dereferences in `unsafe` blocks (rings,
//!   command blocks) while the device writes the same bytes, `_dmamem_unmap` and
//!   `_dmamem_free` take the pointer and the segments back on every exit path, and the tag's
//!   coherence is read at each `_dmamap_sync`. Here the buffer owns all of it: the mapping, its
//!   one segment, the tag's coherence and the provider that frees it, so nothing leaks or is
//!   freed twice, and the bytes are reached only by checked volatile accesses. A range check
//!   costs one compare and branch per access more than the C's raw pointer (equal to `bsd`'s
//!   `virtiovar.rs` `used_ring` today); N7.3a's bench decides on a masked ring view.
//! - The tag's memory half is a trait object, [`DmaMemory`], passed as a value (decision 39, as
//!   `hal::mem::kmbuf::PageMemory`): amd64's tags wrap one another (`acpidmar`'s IOMMU, ISA's
//!   16 MB bound, a SEV guest's bounce) and arm64's SMMU will (N7.11), so it is not a `cfg`.
//!   The request's flags are named fields ([`DmaReq`]): `BUS_DMA_*`'s values differ between
//!   amd64 and arm64 (`include/bus.h`), and only the kernel reads them. [`DmaSync`]'s values
//!   are `hal`'s own for the same reason.
//! - `_dmamap_sync`'s segment walk and bounce pages belong to a loaded map (the streaming
//!   half, `kernel::dev::bus_dma` at N7.3a); a buffer of its own memory has neither, so its
//!   sync is the C's per-segment switch over one range. `dc ivac` (`cpu_dcache_inv_range`,
//!   `bsd`'s `unsafe fn`, it discards dirty lines) is `hal`'s for this caller alone
//!   (`arch/arm64/cpufunc.rs`'s `cpu_dcache_inv_dma`), over lines that lie inside the
//!   buffer's own page-aligned mapping.

use core::mem::{align_of, size_of};
use core::num::NonZeroUsize;
use core::ops::BitOr;
use core::ptr::{self, NonNull};
use core::sync::atomic::{Ordering, fence};

use crate::arch::current::param::PAGE_SIZE;
use crate::assert::hal_panic;
use crate::mem::trust::check_block;
use crate::sys::errno::Errno;
use crate::user::{AbiPod, bytes_of, bytes_of_mut, zeroed};

/// The word a copy moves at a time where the buffer's address is aligned for it.
type Word = u64;

/// `bus_dma_segment_t`: a run of bus addresses (`ds_addr`, `ds_len`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DmaSeg {
    /// `ds_addr`: the first bus address (`bus_addr_t`, a `u_long` on both machines).
    pub addr: u64,
    /// `ds_len`: its length in bytes.
    pub len: usize,
}

/// What `bus_dmamem_alloc_range` and `bus_dmamem_map` are asked for one segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DmaReq {
    /// The bytes wanted (rounded up to pages by the provider).
    pub size: usize,
    /// The bus-address alignment, a power of two (0: none).
    pub align: usize,
    /// A power of two the segment may not cross (0: none).
    pub boundary: usize,
    /// `alloc_range`'s `low` and `high` (inclusive); `None`: the tag's own range
    /// (`dma_constraint`).
    pub range: Option<(u64, u64)>,
    /// `BUS_DMA_NOWAIT`: fail rather than sleep for memory or for kernel virtual space.
    pub nowait: bool,
    /// `BUS_DMA_ZERO`: zeroed memory.
    pub zero: bool,
    /// `BUS_DMA_COHERENT`: a mapping the device and the CPU see alike; a tag that is not
    /// coherent maps it uncached (`_dmamem_map`).
    pub coherent: bool,
    /// `BUS_DMA_NOCACHE`: an uncached mapping.
    pub nocache: bool,
    /// `BUS_DMA_64BIT`: the device reaches every bus address (amd64's provider; see the
    /// module's deviations).
    pub dma64: bool,
}

/// What a provider answers: the mapping, its one segment, whether the tag keeps DMA
/// coherent, and a word of its own given back with the block to `free`.
#[derive(Debug)]
pub struct DmaBlock {
    /// The mapping's first byte, page-aligned.
    pub va: NonNull<u8>,
    /// The mapping's length, whole pages.
    pub len: usize,
    /// The segment of bus addresses the mapping's frames are.
    pub seg: DmaSeg,
    /// The tag keeps DMA coherent (`BUS_DMA_COHERENT` in its `_flags`): a sync is a memory
    /// barrier.
    pub coherent: bool,
    /// The provider's own word (an IOMMU's mapping, ...), back to it unchanged.
    pub cookie: usize,
}

/// `bus_dmamap_sync`'s operations: `BUS_DMASYNC_PREREAD`, `PREWRITE`, `POSTREAD`,
/// `POSTWRITE`, combined with `|` (the values are `hal`'s own: the machines' differ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DmaSync(u8);

impl DmaSync {
    /// `BUS_DMASYNC_POSTREAD`: after the device wrote memory.
    pub const POSTREAD: Self = Self(0x1);
    /// `BUS_DMASYNC_POSTWRITE`: after the device read memory.
    pub const POSTWRITE: Self = Self(0x2);
    /// `BUS_DMASYNC_PREREAD`: before the device writes memory.
    pub const PREREAD: Self = Self(0x4);
    /// `BUS_DMASYNC_PREWRITE`: before the device reads memory.
    pub const PREWRITE: Self = Self(0x8);
}

impl BitOr for DmaSync {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// `BUS_DMASYNC_PREREAD | BUS_DMASYNC_PREWRITE`, a case of `_dmamap_sync_segment`'s switch.
const PRE_BOTH: DmaSync = DmaSync(DmaSync::PREREAD.0 | DmaSync::PREWRITE.0);
/// `BUS_DMASYNC_POSTREAD | BUS_DMASYNC_POSTWRITE`, another.
const POST_BOTH: DmaSync = DmaSync(DmaSync::POSTREAD.0 | DmaSync::POSTWRITE.0);

/// Memory owned by the kernel and shared with a device (`bus_dmamem_alloc(9)` and
/// `bus_dmamem_map(9)`), one segment, freed by the drop. `Send` and `Sync` by its fields: a
/// shared one only reads (volatile loads of memory no reference covers) and syncs, a write
/// needs `&mut`.
pub struct DmaBuf {
    /// The mapping's first byte, its address exposed by the provider.
    va: NonZeroUsize,
    /// The bytes asked for (`DmaReq::size`): every access lies below.
    len: usize,
    /// The mapping's length, whole pages.
    mapped: usize,
    seg: DmaSeg,
    coherent: bool,
    cookie: usize,
    mem: &'static dyn DmaMemory,
}

impl DmaBuf {
    /// `bus_dmamem_alloc_range` of one segment and `bus_dmamem_map` through `mem`: `EINVAL`
    /// for no byte or an alignment or boundary that is not a power of two, the provider's
    /// error otherwise (`ENOMEM`). `panic(9)` when the provider's answer does not keep the
    /// request (a mapping not page-aligned or short, a segment short, misaligned, outside the
    /// range or across the boundary): an allocator that lies about its memory cannot go on.
    pub fn alloc(mem: &'static dyn DmaMemory, req: &DmaReq) -> Result<DmaBuf, Errno> {
        let pow2 = |x: usize| x == 0 || x.is_power_of_two();
        if req.size == 0 || !pow2(req.align) || !pow2(req.boundary) {
            return Err(Errno::EINVAL);
        }
        let block = mem.alloc(req)?;
        check_block("DmaMemory", block.va, block.len, req.size, PAGE_SIZE);
        // Whole pages: a cache line `sync` discards never reaches past the mapping
        // (`arch/arm64/cpufunc.rs`'s `cpu_dcache_inv_dma`).
        if !block.len.is_multiple_of(PAGE_SIZE) {
            hal_panic(format_args!(
                "DmaMemory: block {:p} of {:#x} bytes is not whole pages",
                block.va, block.len
            ));
        }
        let seg = block.seg;
        let last = seg.addr.checked_add(req.size as u64 - 1);
        let kept = seg.len >= req.size
            && seg.addr.is_multiple_of(req.align.max(1) as u64)
            && last.is_some_and(|last| {
                req.range
                    .is_none_or(|(low, high)| seg.addr >= low && last <= high)
                    && (req.boundary == 0
                        || seg.addr / req.boundary as u64 == last / req.boundary as u64)
            });
        if !kept {
            hal_panic(format_args!(
                "DmaMemory: segment {:#x}+{:#x} for {:#x} bytes aligned {:#x}, boundary {:#x}, range {:x?}",
                seg.addr, seg.len, req.size, req.align, req.boundary, req.range
            ));
        }
        Ok(DmaBuf {
            va: block.va.expose_provenance(),
            len: req.size,
            mapped: block.len,
            seg,
            coherent: block.coherent,
            cookie: block.cookie,
            mem,
        })
    }

    /// The bytes asked for.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Never: a buffer holds a byte at least.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The segment (`dm_segs[0]` of the map that loads it): what the device is told.
    pub fn seg(&self) -> DmaSeg {
        self.seg
    }

    /// Whether the tag keeps DMA coherent (a sync is a barrier only).
    pub fn is_coherent(&self) -> bool {
        self.coherent
    }

    /// The `T` at byte `off`, one volatile load where its address is aligned for `T`, byte by
    /// byte otherwise; `panic(9)` past the end.
    pub fn read<T: AbiPod>(&self, off: usize) -> T {
        let a = self.range("read", off, size_of::<T>());
        if a.is_multiple_of(align_of::<T>()) {
            self.load(a)
        } else {
            let mut v = zeroed::<T>();
            self.copy_out(off, bytes_of_mut(core::slice::from_mut(&mut v)));
            v
        }
    }

    /// `v` written at byte `off`, as [`DmaBuf::read`] reads it.
    pub fn write<T: AbiPod>(&mut self, off: usize, v: T) {
        let a = self.range("write", off, size_of::<T>());
        if a.is_multiple_of(align_of::<T>()) {
            self.store(a, v);
        } else {
            self.copy_in(off, bytes_of(core::slice::from_ref(&v)));
        }
    }

    /// `dst.len()` bytes from byte `off` into `dst`; `panic(9)` past the end.
    pub fn copy_out(&self, off: usize, dst: &mut [u8]) {
        let mut a = self.range("copy_out", off, dst.len());
        let (head, rest) = dst.split_at_mut(head_len(a, dst.len()));
        for b in head {
            *b = self.load(a);
            a += 1;
        }
        let mut words = rest.chunks_exact_mut(size_of::<Word>());
        for w in &mut words {
            w.copy_from_slice(&self.load::<Word>(a).to_ne_bytes());
            a += size_of::<Word>();
        }
        for b in words.into_remainder() {
            *b = self.load(a);
            a += 1;
        }
    }

    /// `src` written from byte `off`; `panic(9)` past the end.
    pub fn copy_in(&mut self, off: usize, src: &[u8]) {
        let mut a = self.range("copy_in", off, src.len());
        let (head, rest) = src.split_at(head_len(a, src.len()));
        for &b in head {
            self.store(a, b);
            a += 1;
        }
        let mut words = rest.chunks_exact(size_of::<Word>());
        for w in &mut words {
            let mut bytes = [0; size_of::<Word>()];
            bytes.copy_from_slice(w);
            self.store(a, Word::from_ne_bytes(bytes));
            a += size_of::<Word>();
        }
        for &b in words.remainder() {
            self.store(a, b);
            a += 1;
        }
    }

    /// `len` zero bytes from byte `off`; `panic(9)` past the end.
    pub fn zero(&mut self, off: usize, len: usize) {
        let start = self.range("zero", off, len);
        let end = start + len;
        let mut a = start;
        let body = start + head_len(start, len);
        while a < body {
            self.store(a, 0u8);
            a += 1;
        }
        while end - a >= size_of::<Word>() {
            self.store::<Word>(a, 0);
            a += size_of::<Word>();
        }
        while a < end {
            self.store(a, 0u8);
            a += 1;
        }
    }

    /// `bus_dmamap_sync(t, map, off, len, ops)` over the buffer's own bytes: on a coherent tag
    /// a memory barrier (`membar_sync`); otherwise `_dmamap_sync_segment`'s cache operations
    /// for `ops`: `PREREAD` (with or without `PREWRITE`) writes back and invalidates,
    /// `PREWRITE` writes back, `POSTREAD` (with or without `POSTWRITE`) orders memory and
    /// invalidates the lines a speculative load may have filled, anything else nothing.
    /// `panic(9)` past the end (the C's `ran off map`).
    pub fn sync(&self, off: usize, len: usize, ops: DmaSync) {
        let va = self.range("sync", off, len);
        if self.coherent {
            fence(Ordering::SeqCst);
            return;
        }
        if len == 0 {
            return;
        }
        match ops {
            DmaSync::PREREAD | PRE_BOTH => cache::wbinv(va, len),
            DmaSync::PREWRITE => cache::wb(va, len),
            DmaSync::POSTREAD | POST_BOTH => {
                fence(Ordering::SeqCst);
                cache::inv(self, off, len);
            }
            _ => {}
        }
    }

    /// The lines a cache operation over `[off, off + len)` may touch, as the address and length
    /// of that range: `None` for no byte (no line), `panic(9)` past the end. Every line holding
    /// a byte of it lies inside the mapping, whose start is page-aligned and whose length is
    /// whole pages (`alloc` checked both) since a cache line never crosses a page. The
    /// machine's `dc ivac` takes its range from here (`arch/arm64/cpufunc.rs`).
    pub(crate) fn lines(&self, off: usize, len: usize) -> Option<(usize, usize)> {
        let va = self.range("sync", off, len);
        (len != 0).then_some((va, len))
    }

    /// The address of byte `off`, once `[off, off + len)` is checked inside the buffer;
    /// `panic(9)` naming `what` otherwise.
    fn range(&self, what: &str, off: usize, len: usize) -> usize {
        if off > self.len || self.len - off < len {
            hal_panic(format_args!(
                "DmaBuf: {what} of {len:#x} bytes at {off:#x} past {:#x}",
                self.len
            ));
        }
        self.va.get() + off
    }

    /// The one load: a `T` at address `a`, which the caller took from [`DmaBuf::range`] for at
    /// least `size_of::<T>()` bytes and checked aligned for `T`.
    #[inline(always)]
    fn load<T: AbiPod>(&self, a: usize) -> T {
        let p = ptr::with_exposed_provenance::<T>(a);
        // SAFETY: the invariant: `[va, va + mapped)` is the block the provider granted this
        // value alone, mapped read and write for the CPU until the drop gives it back, holding
        // no Rust object (`DmaMemory`'s contract, the trust paragraph of `sys/hal/mem/trust.rs`;
        // `alloc` checked the mapping page-aligned and at least `len` long), and `len` is at
        // most `mapped`. The caller proved the `T` at `a` inside `[va, va + len)` and `a`
        // aligned for `T`. On a machine the block is kernel pages outside every Rust
        // allocation, so the volatile load needs no provenance and races nothing the language
        // sees, the device's writes included; on the host double it is the double's own
        // allocation, whose address it exposed. No reference to the bytes exists (none is
        // ever made), and any bit pattern is a `T` (`AbiPod`).
        unsafe { p.read_volatile() }
    }

    /// The one store: `v` at address `a`, proved as for [`DmaBuf::load`]; `&mut self`, so no
    /// other access through this value runs beside it.
    #[inline(always)]
    fn store<T: AbiPod>(&mut self, a: usize, v: T) {
        let p = ptr::with_exposed_provenance_mut::<T>(a);
        // SAFETY: as for `load`: the `T` at `a` lies inside the block, aligned, in memory no
        // Rust reference covers, mapped writable; the device may read it at the same time,
        // which changes nothing the language sees.
        unsafe { p.write_volatile(v) }
    }
}

impl Drop for DmaBuf {
    /// `bus_dmamem_unmap`, then `bus_dmamem_free`: the block back to its provider unchanged.
    fn drop(&mut self) {
        self.mem.free(DmaBlock {
            va: NonNull::with_exposed_provenance(self.va),
            len: self.mapped,
            seg: self.seg,
            coherent: self.coherent,
            cookie: self.cookie,
        });
    }
}

/// A bus's DMA tag, the half that makes owned memory (`_dmamem_alloc_range`, `_dmamem_map`,
/// `_dmamem_unmap`, `_dmamem_free` of `struct machine_bus_dma_tag`): implemented by the
/// kernel and passed as a value, as `PageMemory`.
///
/// The contract (`sys/hal/mem/trust.rs`): a block `alloc` returns is `len` bytes at `va` of
/// kernel memory mapped read and write for the CPU, holding no Rust object, whose frames are
/// the bus addresses of `seg`, the caller's alone until `free` takes it back; `hal` checks
/// what it can (`DmaBuf::alloc`).
pub trait DmaMemory: Sync {
    /// One segment of `req.size` bytes (rounded up to pages) and its mapping; `ENOMEM` when
    /// there is none now.
    fn alloc(&self, req: &DmaReq) -> Result<DmaBlock, Errno>;
    /// Takes back a block `alloc` gave, unchanged: the mapping removed, then the memory freed.
    fn free(&self, block: DmaBlock);
}

/// How many bytes from `a` go one by one before an address aligned for a [`Word`], at most
/// `len`.
fn head_len(a: usize, len: usize) -> usize {
    a.next_multiple_of(size_of::<Word>())
        .wrapping_sub(a)
        .min(len)
}

/// The data-cache operations a sync asks for, by address: arm64's.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
mod cache {
    pub(super) use crate::arch::arm64::cpufunc::{
        cpu_dcache_inv_dma as inv, cpu_dcache_wb_range as wb, cpu_dcache_wbinv_range as wbinv,
    };
}

/// None where the machine keeps DMA coherent or has no device: amd64 (its tags are coherent,
/// its sync only orders memory) and the host double, which counts the calls for its tests.
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
mod cache {
    use super::DmaBuf;

    #[cfg(test)]
    std::thread_local! {
        /// The cache operations this thread's syncs asked for.
        static OPS: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    }

    /// Counts one operation (tests only).
    fn count() {
        #[cfg(test)]
        OPS.with(|n| n.set(n.get() + 1));
    }

    /// The cache operations this thread asked for so far.
    #[cfg(test)]
    pub(super) fn ops() -> u32 {
        OPS.with(core::cell::Cell::get)
    }

    pub(super) fn wb(_va: usize, _len: usize) {
        count();
    }

    pub(super) fn wbinv(_va: usize, _len: usize) {
        count();
    }

    /// The machine's contract kept: no line for no byte.
    pub(super) fn inv(buf: &DmaBuf, off: usize, len: usize) {
        if buf.lines(off, len).is_some() {
            count();
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::vec::Vec;

    use super::*;

    /// A page of the double's memory.
    #[derive(Clone, Copy)]
    #[repr(C, align(4096))]
    struct Page([u8; 4096]);

    /// What the double answers.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Grant {
        /// What was asked.
        Honest,
        /// A mapping a page short of the request.
        Short,
        /// A segment one page off the alignment asked for.
        Misaligned,
        /// A mapping as long as asked but not whole pages.
        Ragged,
    }

    /// A provider of leaked host pages (never reused, so no test sees another's), counting
    /// what it hands out and takes back.
    struct Double {
        grant: Grant,
        coherent: bool,
        allocs: AtomicUsize,
        frees: AtomicUsize,
        last_cookie: AtomicUsize,
    }

    impl DmaMemory for Double {
        fn alloc(&self, req: &DmaReq) -> Result<DmaBlock, Errno> {
            let pages = req.size.div_ceil(PAGE_SIZE);
            let extra = usize::from(self.grant == Grant::Misaligned);
            let align = req.align.max(PAGE_SIZE);
            // Room for the pages, the misaligned grant's page and the alignment's skip.
            let count = pages + extra + align / PAGE_SIZE;
            let mem: &'static mut [Page] =
                Vec::from_iter(core::iter::repeat_n(Page([0; 4096]), count)).leak();
            let all = mem.as_mut_ptr().cast::<u8>();
            // The bus address is the host address (an IOMMU-less bus), and the misaligned
            // grant starts one page in.
            let start = all.expose_provenance().next_multiple_of(align) + extra * PAGE_SIZE;
            let off = start - all.addr();
            let va = NonNull::new(all.wrapping_add(off)).expect("leaked memory");
            let len = match self.grant {
                Grant::Short => (pages - 1) * PAGE_SIZE,
                Grant::Ragged => req.size.next_multiple_of(8),
                Grant::Honest | Grant::Misaligned => pages * PAGE_SIZE,
            };
            self.allocs.fetch_add(1, Ordering::Relaxed);
            Ok(DmaBlock {
                va,
                len,
                seg: DmaSeg {
                    addr: start as u64,
                    len,
                },
                coherent: self.coherent,
                cookie: 0x5eed,
            })
        }

        fn free(&self, block: DmaBlock) {
            self.last_cookie.store(block.cookie, Ordering::Relaxed);
            self.frees.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn double(grant: Grant, coherent: bool) -> &'static Double {
        Box::leak(Box::new(Double {
            grant,
            coherent,
            allocs: AtomicUsize::new(0),
            frees: AtomicUsize::new(0),
            last_cookie: AtomicUsize::new(0),
        }))
    }

    fn req(size: usize) -> DmaReq {
        DmaReq {
            size,
            ..DmaReq::default()
        }
    }

    fn buf(size: usize) -> DmaBuf {
        DmaBuf::alloc(double(Grant::Honest, false), &req(size)).expect("allocated")
    }

    #[test]
    fn a_buffer_is_freed_once_by_its_drop_with_its_cookie() {
        let d = double(Grant::Honest, true);
        let b = DmaBuf::alloc(d, &req(3 * PAGE_SIZE / 2)).unwrap();
        assert_eq!((b.len(), b.seg().len), (3 * PAGE_SIZE / 2, 2 * PAGE_SIZE));
        assert!(b.is_coherent() && !b.is_empty());
        assert_eq!(d.frees.load(Ordering::Relaxed), 0);
        drop(b);
        assert_eq!(d.allocs.load(Ordering::Relaxed), 1);
        assert_eq!(d.frees.load(Ordering::Relaxed), 1);
        assert_eq!(d.last_cookie.load(Ordering::Relaxed), 0x5eed);
    }

    #[test]
    fn a_bad_request_is_einval_and_asks_the_provider_nothing() {
        let d = double(Grant::Honest, false);
        let bad = [
            req(0),
            DmaReq {
                align: 3 * PAGE_SIZE,
                ..req(PAGE_SIZE)
            },
            DmaReq {
                boundary: 12,
                ..req(PAGE_SIZE)
            },
        ];
        for r in bad {
            assert!(matches!(DmaBuf::alloc(d, &r), Err(Errno::EINVAL)));
        }
        assert_eq!(d.allocs.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn the_request_is_kept() {
        let d = double(Grant::Honest, false);
        let r = DmaReq {
            align: 4 * PAGE_SIZE,
            boundary: 8 * PAGE_SIZE,
            ..req(2 * PAGE_SIZE)
        };
        let b = DmaBuf::alloc(d, &r).unwrap();
        assert_eq!(b.seg().addr % (4 * PAGE_SIZE) as u64, 0);
    }

    #[test]
    #[should_panic(expected = "DmaMemory: block")]
    fn an_undersized_grant_is_refused() {
        let _ = DmaBuf::alloc(double(Grant::Short, false), &req(2 * PAGE_SIZE));
    }

    #[test]
    #[should_panic(expected = "is not whole pages")]
    fn a_grant_that_is_not_whole_pages_is_refused() {
        // As long as asked (100 bytes, 104 granted), page-aligned, but its last cache line
        // could reach past the mapping.
        let _ = DmaBuf::alloc(double(Grant::Ragged, false), &req(100));
    }

    #[test]
    fn no_byte_is_no_cache_line() {
        let b = buf(PAGE_SIZE);
        assert_eq!(b.lines(PAGE_SIZE, 0), None, "at the end, nothing");
        assert_eq!(b.lines(0, 0), None);
        assert_eq!(b.lines(8, 16), Some((b.va.get() + 8, 16)));
        let before = cache::ops();
        cache::inv(&b, PAGE_SIZE, 0);
        b.sync(PAGE_SIZE, 0, DmaSync::POSTREAD);
        b.sync(0, 0, DmaSync::PREREAD);
        assert_eq!(cache::ops(), before);
    }

    #[test]
    #[should_panic(expected = "DmaBuf: sync")]
    fn the_lines_past_the_end_panic() {
        let b = buf(PAGE_SIZE);
        let _ = b.lines(PAGE_SIZE, 1);
    }

    #[test]
    #[should_panic(expected = "DmaMemory: segment")]
    fn a_misaligned_grant_is_refused() {
        let r = DmaReq {
            align: 2 * PAGE_SIZE,
            ..req(PAGE_SIZE)
        };
        let _ = DmaBuf::alloc(double(Grant::Misaligned, false), &r);
    }

    #[test]
    #[should_panic(expected = "DmaMemory: segment")]
    fn a_grant_outside_the_range_is_refused() {
        let r = DmaReq {
            range: Some((0, 0xfff)),
            ..req(PAGE_SIZE)
        };
        let _ = DmaBuf::alloc(double(Grant::Honest, false), &r);
    }

    #[test]
    fn words_round_trip_aligned_or_not() {
        let mut b = buf(PAGE_SIZE);
        b.write::<u64>(0, 0x0102_0304_0506_0708);
        assert_eq!(b.read::<u64>(0), 0x0102_0304_0506_0708);
        assert_eq!(b.read::<u8>(0), 0x08, "little-endian, as the machines");
        b.write::<u16>(0x101, 0xbeef);
        assert_eq!(b.read::<u16>(0x101), 0xbeef);
        assert_eq!(b.read::<u8>(0x101), 0xef);
        b.write::<u32>(PAGE_SIZE - 4, 0xfeed_f00d);
        assert_eq!(b.read::<u32>(PAGE_SIZE - 4), 0xfeed_f00d);
        b.write::<[u16; 3]>(0x203, [1, 2, 3]);
        assert_eq!(b.read::<[u16; 3]>(0x203), [1, 2, 3]);
    }

    #[test]
    fn copies_and_zero_move_every_byte() {
        let mut b = buf(PAGE_SIZE);
        let src: Vec<u8> = (0..100u8).collect();
        for off in [0, 3, 8, 13] {
            b.copy_in(off, &src);
            let mut dst = [0u8; 100];
            b.copy_out(off, &mut dst);
            assert_eq!(&dst[..], &src[..], "at {off}");
        }
        b.zero(5, 50);
        let mut dst = [0xffu8; 60];
        b.copy_out(0, &mut dst);
        assert!(dst[5..55].iter().all(|&x| x == 0));
        // The copies at 0, 3, 8 and 13 left 0, 1, 2 (from 0), then 0, 1 (from 3) below 5,
        // and the last copy's bytes from 13 on.
        assert_eq!(dst[55..60], [42, 43, 44, 45, 46]);
        assert_eq!(dst[..5], [0, 1, 2, 0, 1]);
    }

    #[test]
    #[should_panic(expected = "DmaBuf: read")]
    fn a_read_past_the_end_panics() {
        let b = buf(100);
        let _ = b.read::<u32>(97);
    }

    #[test]
    #[should_panic(expected = "DmaBuf: write")]
    fn a_write_past_the_end_panics() {
        let mut b = buf(100);
        b.write::<u8>(100, 1);
    }

    #[test]
    #[should_panic(expected = "DmaBuf: copy_out")]
    fn a_copy_out_past_the_end_panics() {
        let b = buf(100);
        let mut dst = [0u8; 10];
        b.copy_out(91, &mut dst);
    }

    #[test]
    #[should_panic(expected = "DmaBuf: copy_in")]
    fn a_copy_in_past_the_end_panics() {
        let mut b = buf(100);
        b.copy_in(usize::MAX, &[1]);
    }

    #[test]
    #[should_panic(expected = "DmaBuf: zero")]
    fn a_zero_past_the_end_panics() {
        let mut b = buf(100);
        b.zero(1, 100);
    }

    #[test]
    #[should_panic(expected = "DmaBuf: sync")]
    fn a_sync_past_the_end_panics() {
        let b = buf(100);
        b.sync(0, 101, DmaSync::PREWRITE);
    }

    #[test]
    fn a_coherent_sync_touches_no_cache_line_and_the_others_do_the_cs() {
        let coherent = DmaBuf::alloc(double(Grant::Honest, true), &req(PAGE_SIZE)).unwrap();
        let before = cache::ops();
        for ops in [
            DmaSync::PREREAD | DmaSync::PREWRITE,
            DmaSync::POSTREAD,
            DmaSync::PREWRITE,
        ] {
            coherent.sync(0, PAGE_SIZE, ops);
        }
        assert_eq!(cache::ops(), before);
        let b = buf(PAGE_SIZE);
        // One operation each, as `_dmamap_sync_segment`'s switch: PREREAD, PREREAD|PREWRITE,
        // PREWRITE, POSTREAD, POSTREAD|POSTWRITE; none for POSTWRITE, a mix or no byte.
        for ops in [
            DmaSync::PREREAD,
            DmaSync::PREREAD | DmaSync::PREWRITE,
            DmaSync::PREWRITE,
            DmaSync::POSTREAD,
            DmaSync::POSTREAD | DmaSync::POSTWRITE,
        ] {
            b.sync(8, 16, ops);
        }
        assert_eq!(cache::ops(), before + 5);
        b.sync(8, 16, DmaSync::POSTWRITE);
        b.sync(8, 16, DmaSync::PREWRITE | DmaSync::POSTREAD);
        b.sync(8, 0, DmaSync::PREWRITE);
        assert_eq!(cache::ops(), before + 5);
    }
}
/* </TESTS> */
