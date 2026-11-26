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
 * Copyright (c) 2001-2003 Opsycon AB  (www.opsycon.se / www.opsycon.com)
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
//! Device registers: `bus_space(9)`'s memory space as a value, [`Mmio<R>`], read and written
//! through typed offsets, [`Reg<R, T>`].
//!
//! Upstream: sys/arch/arm64/arm64/bus_space.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/include/bus.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/bus_space.rs@e8249449d905
//! LZ: sys/arch/arm64/include/bus.rs@e8249449d905
//!
//! A mapping is made only by the kernel map's token ([`KernelMapper::map_device`]): whole pages
//! from a page-aligned physical address, entered read and write, never executable, with the
//! memory type the caller names ([`DevCache`]), and the register block `off` bytes into the
//! first page. Every frame is checked not to be a managed page (`PhysPages::md_of`) nor to hold
//! the kernel image: RAM the page system hands out, and the kernel's own text and data, are
//! never device memory. The value is the mapping: [`Mmio::read`] and
//! [`Mmio::write`] are `bus_space_read_N`/`bus_space_write_N`, one volatile access each, at an
//! offset a [`Reg`] carries; [`Mmio::read_at`]/[`Mmio::write_at`] take a computed offset and
//! check it. Only [`KernelMapper::unmap_device`] consumes it (`pmap_kremove`), so no access
//! outlives the mapping; a dropped `Mmio` leaves its mapping, as a softc that never detaches.
//!
//! A register layout is a marker type ([`RegBlock`], its `SIZE`), and a register a `const`
//! of [`Reg<R, T>`]: [`Reg::new`] asserts the offset is aligned for `T` and the register lies
//! inside the block, which a `const` item evaluates at compile time, so a fixed-offset access
//! has no check at run time. The block's size is checked against the mapping once, by
//! `map_device`, and the block's first byte is aligned for every register width there.
//!
//! The trust boundary (`sys/hal/mem/trust.rs`): that the physical range is the device's registers,
//! and the virtual range the token holder's own, is the caller's word, as for `kenter`. A
//! frame of RAM that is neither a managed page nor the kernel image (the pages stolen before
//! `uvm_page_init`, the boot page tables among them, of which nothing keeps a record) cannot
//! be told from a device's by any check here.
//!
//! ## Deviations
//! - None of its own. [`DevCache`] names the four memory types the C's `generic_space_map`
//!   chooses from; the translation of the `BUS_SPACE_MAP_*` flags, and LZ's deviation in it,
//!   are `bsd`'s caller's (`sys/arch/arm64/arm64/bus_space.rs`, its `## Deviations`).
//!
//! ## Redesign
//! - LZ's handle is a bare address (`BusSpaceHandle(usize)`), read through the tag's function
//!   table with `h + o` dereferenced in an `unsafe` block per width, any offset accepted
//!   (`generic_space_read_1` .. `_8`, `generic_space_write_1` .. `_8`). Here the handle is the
//!   mapping itself, its accesses two methods generic over the width ([`MmioWord`]: `u8` to
//!   `u64`), inlined: one load or store where the C makes an indirect call and then the load
//!   (`arm64/include/bus.h`'s `bus_space_read_4` through `t->_space_read_4`). A computed offset
//!   costs one compare and branch more than the C, named for its first user (the GIC's per-IRQ
//!   registers, N7.2a).
//! - LZ's `generic_space_map` entered each page with the legacy `pmap_kenter_cache` `unsafe fn`
//!   in `bsd`; the mapping loop is [`KernelMapper::map_device`] here (with the managed-frame
//!   check the C does not make), its virtual range chosen by the token's holder
//!   (`kernel::vm::km::map_device`, the C's `km_alloc(kv_any, kp_none)` and LZ's `vmmap`
//!   window). `bsd`'s `generic_space_map` keeps its name and signature over it and takes the
//!   address out ([`Mmio::into_legacy_va`]); its reads and writes stay its own (163 modules name
//!   `machine::bus`) until the drivers move.
//! - Not here, each with its first kernel user: `bus_space_subregion` (`generic_space_region`),
//!   `bus_space_vaddr`, the region and multi accessors, `bus_space_mmap` and amd64's port space
//!   (a `PortIo<R>` with the same four methods at N10).

use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr;

use crate::arch::current::param::PAGE_SIZE;
use crate::assert::hal_panic;
use crate::vm::pmap::{self, KernelMapper, PMAP_NOMMU};
use sysdefs::errno::Errno;
use sysdefs::types::{Paddr, Vaddr, Vsize};

/// The alignment of a typed block's first byte: the widest register's (`u64`).
const BLOCK_ALIGN: usize = align_of::<u64>();

/// The widths a register is read and written in: `u8`, `u16`, `u32` and `u64`, every bit
/// pattern of which is a value. Sealed: no other type can be one.
pub trait MmioWord: sealed::Word {}

impl MmioWord for u8 {}
impl MmioWord for u16 {}
impl MmioWord for u32 {}
impl MmioWord for u64 {}

/// The seal of [`MmioWord`].
mod sealed {
    /// An integer of a register's width.
    pub trait Word: Copy + 'static {}
    impl Word for u8 {}
    impl Word for u16 {}
    impl Word for u32 {}
    impl Word for u64 {}
}

/// A device's register layout: one marker type per kind of register block, its size the
/// bytes every register of the layout lies within. A mapping shorter than `SIZE` is refused.
pub trait RegBlock: 'static {
    /// The block's size in bytes.
    const SIZE: usize;
}

/// The block with no layout (`SIZE` 0): no [`Reg`] can name a register of it, so it is read
/// and written only at computed offsets ([`Mmio::read_at`]); `bsd`'s handles.
#[derive(Clone, Copy, Debug)]
pub struct Untyped;

impl RegBlock for Untyped {
    const SIZE: usize = 0;
}

/// A register of layout `R`, `T` wide, at a fixed offset of the block.
pub struct Reg<R: RegBlock, T: MmioWord> {
    off: usize,
    _layout: PhantomData<fn() -> (R, T)>,
}

impl<R: RegBlock, T: MmioWord> Reg<R, T> {
    /// The register at byte `off` of the block. Panics unless `off` is aligned for `T` and the
    /// register ends inside `R::SIZE`: at compile time in a `const` item, the form a driver's
    /// layout takes.
    ///
    /// ```
    /// use hal::dev::mmio::{Reg, RegBlock};
    /// struct Regs;
    /// impl RegBlock for Regs {
    ///     const SIZE: usize = 0x10;
    /// }
    /// const STATUS: Reg<Regs, u32> = Reg::new(0x0c);
    /// fn main() {
    ///     assert_eq!(STATUS.offset(), 0x0c);
    /// }
    /// ```
    ///
    /// A register past its block does not compile:
    ///
    /// ```compile_fail
    /// use hal::dev::mmio::{Reg, RegBlock};
    /// struct Regs;
    /// impl RegBlock for Regs {
    ///     const SIZE: usize = 0x10;
    /// }
    /// const PAST: Reg<Regs, u32> = Reg::new(0x10);
    /// fn main() {
    ///     let _ = PAST.offset();
    /// }
    /// ```
    ///
    /// Nor does one not aligned for its width:
    ///
    /// ```compile_fail
    /// use hal::dev::mmio::{Reg, RegBlock};
    /// struct Regs;
    /// impl RegBlock for Regs {
    ///     const SIZE: usize = 0x10;
    /// }
    /// const ODD: Reg<Regs, u16> = Reg::new(0x03);
    /// fn main() {
    ///     let _ = ODD.offset();
    /// }
    /// ```
    pub const fn new(off: usize) -> Self {
        assert!(
            off.is_multiple_of(align_of::<T>()),
            "Reg::new: a register not aligned for its width"
        );
        assert!(
            off <= R::SIZE && R::SIZE - off >= size_of::<T>(),
            "Reg::new: a register past its block"
        );
        Self {
            off,
            _layout: PhantomData,
        }
    }

    /// Its byte offset in the block.
    pub const fn offset(self) -> usize {
        self.off
    }
}

impl<R: RegBlock, T: MmioWord> Clone for Reg<R, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: RegBlock, T: MmioWord> Copy for Reg<R, T> {}

/// A memory type of a device mapping, the four `generic_space_map` chooses from:
/// `PMAP_CACHE_DEV_NGNRNE`, `PMAP_CACHE_DEV_NGNRE`, `PMAP_CACHE_CI` and `PMAP_CACHE_WB` on arm64
/// (amd64's `pmap_kenter_pa` flags at N10: `PMAP_NOCACHE` for the two device types, `PMAP_WC`,
/// none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevCache {
    /// Device memory, non-gathering, non-reordering, no early write acknowledgement
    /// (Device-nGnRnE): the default.
    Device,
    /// Device memory with early write acknowledgement (Device-nGnRE): the C's
    /// `BUS_SPACE_MAP_POSTED`.
    Posted,
    /// Normal memory, not cacheable: the C's `BUS_SPACE_MAP_PREFETCHABLE` (`PMAP_CACHE_CI`).
    Uncached,
    /// Normal write-back memory: `BUS_SPACE_MAP_CACHEABLE`.
    Cacheable,
}

/// A device's registers mapped in the kernel pmap: `bus_space_handle_t` with the mapping it
/// stands for. Made only by [`KernelMapper::map_device`] and consumed only by
/// [`KernelMapper::unmap_device`], so every access lies inside a live mapping of device
/// frames. `Send` and `Sync` by its fields: every access is one volatile load or store of
/// memory outside every Rust allocation, which threads may make at once (as the C's CPUs do).
pub struct Mmio<R: RegBlock> {
    /// The block's first byte: the mapping plus the block's offset in its first page (the
    /// direct map's address on a machine without an MMU, the host double).
    base: usize,
    /// The block's length (`bus_space_map`'s `size`).
    len: usize,
    /// The mapping's first page.
    va: Vaddr,
    /// The mapping's length, whole pages.
    size: usize,
    _layout: PhantomData<fn() -> R>,
}

impl<R: RegBlock> Mmio<R> {
    /// `bus_space_read_N(t, h, o)` of the register `r`.
    #[inline(always)]
    pub fn read<T: MmioWord>(&self, r: Reg<R, T>) -> T {
        self.load(r.off)
    }

    /// `bus_space_write_N(t, h, o, v)` of the register `r`.
    #[inline(always)]
    pub fn write<T: MmioWord>(&self, r: Reg<R, T>, v: T) {
        self.store(r.off, v);
    }

    /// `bus_space_read_N(t, h, off)` at a computed offset; `panic(9)` when the word does not
    /// lie inside the block or is not aligned for `T`.
    #[inline]
    pub fn read_at<T: MmioWord>(&self, off: usize) -> T {
        self.check::<T>(off);
        self.load(off)
    }

    /// `bus_space_write_N(t, h, off, v)` at a computed offset; `panic(9)` as
    /// [`Mmio::read_at`].
    #[inline]
    pub fn write_at<T: MmioWord>(&self, off: usize, v: T) {
        self.check::<T>(off);
        self.store(off, v);
    }

    /// The block's length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the block has no byte (an `Untyped` mapping of size 0).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The block's address, for `bsd`'s `BusSpaceHandle`, whose readers dereference it in
    /// their own `unsafe` blocks: the mapping stays, never unmapped through this value again
    /// (`bsd`'s `generic_space_unmap` removes it). Scaffolding (docs/SCAFFOLDING.md): goes with
    /// `bsd`'s last `bus_space` reader (N7.14).
    pub fn into_legacy_va(self) -> usize {
        self.base
    }

    /// Panics unless a `T` at `off` lies inside the block and its address is aligned for `T`.
    #[inline]
    fn check<T: MmioWord>(&self, off: usize) {
        let inside = off <= self.len && self.len - off >= size_of::<T>();
        if !inside || !(self.base + off).is_multiple_of(align_of::<T>()) {
            hal_panic(format_args!(
                "bus_space: a {}-byte access at {off:#x} of a {:#x}-byte block at {:#x}",
                size_of::<T>(),
                self.len,
                self.base
            ));
        }
    }

    /// The one load: a `T` at `off`, which the caller has proved inside the block and aligned
    /// (a [`Reg`] of `R`, or [`Mmio::check`]).
    #[inline(always)]
    fn load<T: MmioWord>(&self, off: usize) -> T {
        let p = ptr::with_exposed_provenance::<T>(self.base + off);
        // SAFETY: the invariant: `[base, base + len)` lies in a mapping `map_device` entered
        // onto frames none of which is a managed page or the kernel image, read and write,
        // which stays while this
        // value lives (only `unmap_device`, which consumes it, removes it; the token's contract
        // keeps every other remover away from its pages, as for any `kenter` mapping). The
        // caller proved `off` (a `Reg::new` of `R`: aligned and inside `R::SIZE`, which
        // `map_device` checked is at most `len`, with `base` aligned for every width; or
        // `check`), so `p` is aligned and the `T` inside the block. Device frames hold no Rust
        // object (the trust paragraph of `sys/hal/mem/trust.rs`: firmware's register ranges), so
        // the volatile load touches memory outside every Rust allocation, which needs no
        // provenance and races nothing the language sees. On the host double the block is a
        // Rust allocation of the test's own whose address it exposed, valid for the read; there
        // the volatile access is an ordinary one, so a store from another thread at the same
        // time would race it: `Mmio` is `Sync`, but the host tests touch each mapping from
        // their own thread alone. Any bit pattern is a `T` (`MmioWord`).
        unsafe { p.read_volatile() }
    }

    /// The one store: `v` at `off`, proved as for [`Mmio::load`].
    #[inline(always)]
    fn store<T: MmioWord>(&self, off: usize, v: T) {
        let p = ptr::with_exposed_provenance_mut::<T>(self.base + off);
        // SAFETY: as for `load`: `p` is aligned and its `T` inside a live, writable mapping of
        // device frames, outside every Rust allocation, so the volatile store changes nothing a
        // Rust reference covers and races nothing the language sees, whichever thread runs it.
        // On the host double the block is the test's own Rust allocation, which no reference
        // covers either; there the store is an ordinary write, sound because the host tests use
        // each mapping from one thread (`Mmio` is `Sync`: a test that shared one across threads
        // would race).
        unsafe { p.write_volatile(v) }
    }
}

#[cfg(target_arch = "aarch64")]
impl KernelMapper {
    /// `bus_space_map`'s mapping loop (`generic_space_map`'s `pmap_kenter_cache` per page):
    /// the whole pages that hold `[pa + off, pa + off + len)` entered at `va` in the kernel
    /// pmap, read and write, never executable, with the memory type `cache`, and the block
    /// `off` bytes into the first page as a value. `va` is kernel virtual space the caller
    /// owns ([`KernelMapper::device_va`], or its `km_alloc(kv_any, kp_none)`), unmapped.
    ///
    /// `EINVAL` when `va` or `pa` is not page-aligned, `off` is not inside the first page, the
    /// range wraps, `len` is shorter than `R::SIZE`, or a typed block (`R::SIZE` above 0)
    /// starts at an address not aligned for a `u64`. `panic(9)` before `pmap_init` (the boot
    /// path maps the early console through its own identity window then) and when a frame is
    /// a managed page or holds the kernel image, before any page is entered. An empty block
    /// (`len` 0) still takes the page it starts in.
    pub fn map_device<R: RegBlock>(
        &self,
        va: Vaddr,
        pa: Paddr,
        off: usize,
        len: usize,
        cache: DevCache,
    ) -> Result<Mmio<R>, Errno> {
        if !pmap::initialized() {
            hal_panic(format_args!(
                "map_device: {:#x} mapped before pmap_init",
                pa.as_usize()
            ));
        }
        // One page at least: an empty block still takes the page it starts in, so every block
        // is a mapped address.
        let size = off
            .checked_add(len.max(1))
            .and_then(|end| end.checked_next_multiple_of(PAGE_SIZE))
            .filter(|&size| {
                off < PAGE_SIZE
                    && va.as_usize().checked_add(size).is_some()
                    && pa.as_usize().checked_add(size).is_some()
            })
            .ok_or(Errno::EINVAL)?;
        if !va.as_usize().is_multiple_of(PAGE_SIZE)
            || !pa.as_usize().is_multiple_of(PAGE_SIZE)
            || len < R::SIZE
        {
            return Err(Errno::EINVAL);
        }
        let base = if PMAP_NOMMU {
            pmap::direct_map(pa).as_usize() + off
        } else {
            va.as_usize() + off
        };
        if R::SIZE > 0 && !base.is_multiple_of(BLOCK_ALIGN) {
            return Err(Errno::EINVAL);
        }
        let pages = pmap::kernel().pages();
        for i in (0..size).step_by(PAGE_SIZE) {
            let frame = Paddr::new(pa.as_usize() + i);
            if pages.md_of(frame).is_some() {
                hal_panic(format_args!(
                    "map_device: {:#x} is a managed page",
                    frame.as_usize()
                ));
            }
            if self.in_kernel_image(frame) {
                hal_panic(format_args!(
                    "map_device: {:#x} holds the kernel image",
                    frame.as_usize()
                ));
            }
        }
        for i in (0..size).step_by(PAGE_SIZE) {
            self.kenter_device(va + Vsize::new(i), Paddr::new(pa.as_usize() + i), cache);
        }
        Ok(Mmio {
            base,
            len,
            va,
            size,
            _layout: PhantomData,
        })
    }

    /// `bus_space_unmap`'s `pmap_kremove`: the mapping `m` stands for removed, and its first
    /// page and length (whole pages) handed back for the caller to give the space back
    /// (`km_free`, or nothing for the device window).
    pub fn unmap_device<R: RegBlock>(&self, m: Mmio<R>) -> (Vaddr, usize) {
        self.kremove(m.va, Vsize::new(m.size));
        (m.va, m.size)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;

    use super::*;
    use crate::testutil;

    /// A test device's layout: 0x40 bytes of registers.
    struct TestRegs;

    impl RegBlock for TestRegs {
        const SIZE: usize = 0x40;
    }

    const R8: Reg<TestRegs, u8> = Reg::new(0x01);
    const R16: Reg<TestRegs, u16> = Reg::new(0x02);
    const R32: Reg<TestRegs, u32> = Reg::new(0x04);
    const R64: Reg<TestRegs, u64> = Reg::new(0x08);
    const LAST: Reg<TestRegs, u64> = Reg::new(0x38);

    /// A page of host memory standing for a device's registers: no managed page of the test
    /// provider, its address exposed (the host's direct map is the identity).
    #[repr(C, align(4096))]
    struct DevPage([u8; 4096]);

    fn device_frame() -> Paddr {
        let p = Box::into_raw(Box::new(DevPage([0; 4096])));
        Paddr::new(p.expose_provenance())
    }

    #[cfg(target_arch = "aarch64")]
    fn mapper() -> &'static KernelMapper {
        &testutil::pmap_boot().mapper
    }

    #[cfg(target_arch = "aarch64")]
    fn map<R: RegBlock>(off: usize, len: usize) -> Result<Mmio<R>, Errno> {
        let m = mapper();
        let va = m.device_va(PAGE_SIZE).expect("the host's device window");
        m.map_device::<R>(va, device_frame(), off, len, DevCache::Device)
    }

    #[test]
    fn a_registers_offset_and_width_are_checked() {
        assert_eq!(
            (R8.offset(), R16.offset(), R32.offset(), R64.offset()),
            (1, 2, 4, 8)
        );
        assert_eq!(LAST.offset() + 8, TestRegs::SIZE);
    }

    #[test]
    #[should_panic(expected = "past its block")]
    fn a_register_past_its_block_panics_at_run_time() {
        // Aligned for its width, so only the end is wrong.
        let off = std::hint::black_box(TestRegs::SIZE);
        let _ = Reg::<TestRegs, u32>::new(off);
    }

    #[test]
    #[should_panic(expected = "not aligned")]
    fn a_misaligned_register_panics_at_run_time() {
        let off = std::hint::black_box(2);
        let _ = Reg::<TestRegs, u32>::new(off);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn each_width_round_trips_through_the_mapping() {
        let m = map::<TestRegs>(0x100, TestRegs::SIZE).unwrap();
        assert_eq!(m.len(), TestRegs::SIZE);
        m.write(R64, 0x1122_3344_5566_7788);
        assert_eq!(m.read(R64), 0x1122_3344_5566_7788);
        m.write(R32, 0xdead_beef);
        assert_eq!(m.read(R32), 0xdead_beef);
        m.write(R16, 0xa55a);
        assert_eq!(m.read(R16), 0xa55a);
        m.write(R8, 0x7e);
        assert_eq!(m.read(R8), 0x7e);
        // The same bytes, little-endian, at computed offsets.
        assert_eq!(m.read_at::<u8>(0x08), 0x88);
        assert_eq!(m.read_at::<u16>(0x04), 0xbeef);
        m.write_at::<u32>(0x38, 0x0102_0304);
        assert_eq!(m.read(LAST) & 0xffff_ffff, 0x0102_0304);
        assert_eq!(m.read_at::<u8>(0x01), 0x7e);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn the_mapping_is_entered_and_unmap_removes_it() {
        let mapper = mapper();
        let va = mapper.device_va(2 * PAGE_SIZE).unwrap();
        assert!(mapper.in_device_window(va));
        let pa = device_frame();
        // A block across a page boundary takes two pages.
        let m = mapper
            .map_device::<Untyped>(va, pa, PAGE_SIZE - 8, 16, DevCache::Posted)
            .unwrap();
        assert_eq!(pmap::kernel().extract(va), Some(pa));
        assert!(pmap::kernel().extract(va + Vsize::new(PAGE_SIZE)).is_some());
        assert_eq!(mapper.unmap_device(m), (va, 2 * PAGE_SIZE));
        assert_eq!(pmap::kernel().extract(va), None);
        assert_eq!(pmap::kernel().extract(va + Vsize::new(PAGE_SIZE)), None);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    #[should_panic(expected = "bus_space: a 4-byte access")]
    fn a_computed_offset_past_the_block_panics() {
        let m = map::<Untyped>(0, 0x10).unwrap();
        let _ = m.read_at::<u32>(0x0d);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    #[should_panic(expected = "bus_space: a 8-byte access")]
    fn a_misaligned_computed_offset_panics() {
        let m = map::<Untyped>(0, 0x40).unwrap();
        m.write_at::<u64>(0x04, 0);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn a_mapping_shorter_than_its_layout_or_misplaced_is_einval() {
        assert!(matches!(
            map::<TestRegs>(0, TestRegs::SIZE - 1),
            Err(Errno::EINVAL)
        ));
        assert!(matches!(
            map::<TestRegs>(4, TestRegs::SIZE),
            Err(Errno::EINVAL)
        ));
        assert!(matches!(map::<Untyped>(PAGE_SIZE, 4), Err(Errno::EINVAL)));
        let m = mapper();
        let va = m.device_va(PAGE_SIZE).unwrap();
        let odd = Paddr::new(device_frame().as_usize() + 8);
        assert!(matches!(
            m.map_device::<Untyped>(va, odd, 0, 8, DevCache::Device),
            Err(Errno::EINVAL)
        ));
        // An untyped block may start anywhere in its page.
        assert!(map::<Untyped>(3, 5).is_ok());
    }

    #[cfg(all(target_arch = "aarch64", feature = "diagnostic"))]
    #[test]
    #[should_panic(expected = "kernel diagnostic assertion")]
    fn an_empty_window_request_fails_the_assertion() {
        let _ = mapper().device_va(0);
    }

    #[cfg(all(target_arch = "aarch64", feature = "diagnostic"))]
    #[test]
    #[should_panic(expected = "kernel diagnostic assertion")]
    fn a_ragged_window_request_fails_the_assertion() {
        let _ = mapper().device_va(PAGE_SIZE + 8);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    #[should_panic(expected = "is a managed page")]
    fn a_managed_frame_is_never_device_memory() {
        let m = mapper();
        let va = m.device_va(PAGE_SIZE).unwrap();
        let frames = testutil::memory_frames(1);
        let _ = m.map_device::<Untyped>(va, frames[0], 0, 8, DevCache::Device);
    }
}
/* </TESTS> */
