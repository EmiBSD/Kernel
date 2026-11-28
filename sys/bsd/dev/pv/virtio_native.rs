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
//! The legacy edge of the kernel's virtio core (N7.3a): a kernel virtio child
//! (`kernel::dev::pv`) under `bsd`'s transports, `virtio_mmio` and `virtio_pci`. Not an
//! OpenBSD file; scaffolding (docs/SCAFFOLDING.md, "virtio's legacy edge"), gone when the
//! transports are the kernel's (N7.3d; the DMA half with the last legacy tag, N7.5/N7.11b).
//!
//! A transport offers its child a kernel [`VirtioArgs`] ([`virtio_config_found`], the one
//! line each transport's attach changed): its `transport` makes a [`LegacyTransport`] for a
//! kernel child, its `legacy` is the transport's own arguments, which [`present`] lends a
//! legacy child as the `void *aux` it always read. [`LegacyTransport`] is the kernel's
//! `VirtioTransport` over the transport's `struct virtio_ops` and softc members, no logic of
//! its own: each method one call of the table's entry or one member written. Its
//! `attach_finish` gives the transport what a legacy child gives it: `sc_vqs`/`sc_nvqs` (an
//! array of shadow queues, the index, size and offsets of each child queue and no ring, which
//! `virtio_pci` reads for its notify offsets and MSI-X vectors), `sc_config_change`, and
//! `sc_native`, the child's handle, to which `virtio_check_vq` hands each queue's interrupt.
//! [`LegacyDmaMemory`] is the transport's `sc_dmat` as the kernel's `DmaMemory`.
//!
//! ## Deviations
//! - A shadow queue exists from `attach_finish` on: a queue freed before it (an attach's
//!   error path) is not told to the device (`setup_queue` of address 0, which the C writes to
//!   the registers of a queue the device never had). No kernel child frees a queue in N7.3a.
//! - A kernel child's queues are its transport's 0 to n - 1, in order, as the C's
//!   transports assume (`virtio_pci_kick` reads `sc_vqs[idx]`): another order is `panic(9)`.
//! - The shadow array is one `mallocarray(9)` of `M_DEVBUF` at `attach_finish` (the C's
//!   child lends its own `struct virtqueue`s); its failure is `ENOMEM`, the child's attach
//!   failing as for the transport's.
//! - A kernel child whose `attach_finish` fails keeps its state alive (the transport's
//!   `sc_native` holds it, and nothing detaches a virtio child): its buffers are not freed
//!   where the C frees viornd's (`dma_free`).
//! - The tag's coherence is not visible to machine-independent code, so the memory answers
//!   not coherent: a sync does a non-coherent tag's cache operations where a coherent tag's
//!   C does a barrier (correct either way; slower on a coherent bus, never on the smokes'
//!   paths that matter).

use core::any::Any;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::{self, NonNull};

use alloc::boxed::Box;
use hal::dev::dma::{DmaBlock, DmaMemory, DmaReq, DmaSeg};
use hal::mem::malloc::{M_NOWAIT as H_NOWAIT, MBox};
use kernel::dev::device::DevRef;
use kernel::dev::pv::virtio::{
    ChildRef, Features, QueueSetup, VirtioArgs, VirtioFeatureName as NativeFeatureName,
    VirtioTransport,
};

use crate::dev::fdt::virtio_mmio::{VIRTIO_MMIO_CA, VIRTIO_MMIO_FDT_CA};
use crate::dev::pci::virtio_pci::VIRTIO_PCI_CA;
use crate::dev::pv::virtiovar::{
    VIRTIO_CHILD_ERROR, VirtioAttachArgs, VirtioFeatureName, VirtioSoftc, virtio_get_status,
    virtio_intr_barrier, virtio_negotiate_features, virtio_poll_intr, virtio_read_device_config_1,
    virtio_read_device_config_2, virtio_read_device_config_4, virtio_read_device_config_8,
    virtio_read_queue_size, virtio_set_status, virtio_setup_queue, virtio_write_device_config_1,
    virtio_write_device_config_2, virtio_write_device_config_4, virtio_write_device_config_8,
};
use crate::kern::kern_malloc::mallocarray;
use crate::kern::subr_autoconf::{config_found_typed, legacy_device};
use crate::kern::subr_prf::panic;
use crate::machine::bus::{
    BUS_DMA_64BIT, BUS_DMA_COHERENT, BUS_DMA_NOCACHE, BUS_DMA_NOWAIT, BUS_DMA_WAITOK, BUS_DMA_ZERO,
    BusDmaSegment, BusDmaTag, BusDmamap, bus_dmamap_create, bus_dmamap_destroy, bus_dmamap_load,
    bus_dmamap_unload, bus_dmamem_alloc, bus_dmamem_alloc_range, bus_dmamem_free, bus_dmamem_map,
    bus_dmamem_unmap,
};
use crate::sys::device::Device;
use crate::sys::errno::Errno;
use crate::sys::malloc::{M_DEVBUF, M_NOWAIT, M_ZERO};

/// The kernel's `VirtioTransport` over a legacy transport: its node, which holds its softc
/// (the transport never detaches while it has a child, fact 6 of N7.3a's brief), and its
/// DMA memory.
pub struct LegacyTransport {
    /// The transport's node (`virtio28`).
    dev: DevRef,
    /// `sc_dmat`'s memory half, made once for this child.
    dma: &'static LegacyDmaMemory,
}

impl LegacyTransport {
    /// The transport's `struct virtio_softc`. `panic(9)` for a node that is not a virtio
    /// transport's.
    fn sc(&self) -> &VirtioSoftc {
        softc_of(&self.dev)
    }
}

/// `sc_dmat` as the kernel's `DmaMemory`: `bus_dmamem_alloc_range` (or `bus_dmamem_alloc`
/// for the tag's own range), `bus_dmamem_map`, `bus_dmamap_create` and `bus_dmamap_load` of
/// one segment, the device's address the loaded map's segment; `free` the four in reverse.
pub struct LegacyDmaMemory {
    /// The transport's tag.
    tag: BusDmaTag,
}

/// What `free` needs back of a block: the map (its cookie's address) and the segment
/// `bus_dmamem_alloc` gave, which a translating tag's loaded map does not show.
struct DmaRecord {
    /// The loaded map.
    map: &'static BusDmamap,
    /// `bus_dmamem_alloc`'s segment.
    seg: BusDmaSegment,
}

/// `(struct virtio_softc *)dev` of a virtio transport's node; `panic(9)` for any other.
fn softc_of(dev: &DevRef) -> &VirtioSoftc {
    let d = legacy_device(dev);
    let ca = d.cfdata().cf_attach;
    if !(ptr::eq(ca, &VIRTIO_MMIO_CA)
        || ptr::eq(ca, &VIRTIO_MMIO_FDT_CA)
        || ptr::eq(ca, &VIRTIO_PCI_CA))
    {
        panic(format_args!(
            "virtio: {} is not a virtio transport",
            dev.xname()
        ));
    }
    // SAFETY: `d` is the legacy header of a node whose row attaches `virtio_mmio` or
    // `virtio_pci` (checked above), so `config_make_softc` made it as that attachment's
    // softc, `ca_devsize` bytes of a `VirtioMmioSoftc` or a `VirtioPciSoftc`, each
    // `#[repr(C)]` with its `struct virtio_softc` first, which `softc` checks against the
    // block's size besides. The header lives as long as the node (its `LegacySoftc` holds a
    // `dv_ref`), and `dev` holds the node for the reference's lifetime.
    unsafe { d.softc::<VirtioSoftc>() }
}

/// [`VirtioArgs::transport`] of a legacy transport: the kernel child's `VirtioTransport`
/// over `parent`, the transport's node (`(struct virtio_softc *)parent`), with its DMA
/// memory (leaked: one per child, as the C never frees a tag).
pub fn legacy_transport(parent: &DevRef) -> Box<dyn VirtioTransport> {
    let tag = softc_of(parent).dmat();
    let dma: &'static LegacyDmaMemory = Box::leak(Box::new(LegacyDmaMemory { tag }));
    Box::new(LegacyTransport {
        dev: parent.clone(),
        dma,
    })
}

/// The transport's `config_found(self, &vma, NULL)`: its child found with a kernel
/// [`VirtioArgs`] (`va`'s id and vectors, the legacy transport's constructor and its own
/// arguments `aux`, which a legacy child gets through [`present`]).
pub fn virtio_config_found(
    parent: &Device,
    va: VirtioAttachArgs,
    aux: *mut c_void,
) -> Option<NonNull<Device>> {
    let args = VirtioArgs {
        devid: va.va_devid,
        nintr: va.va_nintr,
        transport: legacy_transport,
        legacy: aux.expose_provenance(),
    };
    config_found_typed(parent, &args, None)
}

/// The virtio presenter of the shim's door (`subr_autoconf.rs`'s `present_aux`): a kernel
/// [`VirtioArgs`] of a legacy transport lent to a legacy child as the `void *` its
/// transport passed before (`VirtioArgs::legacy`); `false`, nothing lent, for any other
/// argument, and for a kernel transport's (whose `legacy` is 0).
pub fn present(aux: &(dyn Any + Send + Sync), f: &mut dyn FnMut(*mut c_void)) -> bool {
    match aux.downcast_ref::<VirtioArgs>() {
        Some(va) if va.legacy != 0 => {
            f(ptr::with_exposed_provenance_mut(va.legacy));
            true
        }
        _ => false,
    }
}

/// `sc_config_change` of a kernel child: its handler.
fn native_config_change(vsc: &VirtioSoftc) -> i32 {
    vsc.sc_native
        .get()
        .and_then(ChildRef::config_change)
        .unwrap_or(0)
}

impl VirtioTransport for LegacyTransport {
    fn kick(&self, index: u16) {
        let sc = self.sc();
        (sc.ops().kick)(sc, index);
    }
    fn read_config_1(&self, off: usize) -> u8 {
        virtio_read_device_config_1(self.sc(), off as i32)
    }
    fn read_config_2(&self, off: usize) -> u16 {
        virtio_read_device_config_2(self.sc(), off as i32)
    }
    fn read_config_4(&self, off: usize) -> u32 {
        virtio_read_device_config_4(self.sc(), off as i32)
    }
    fn read_config_8(&self, off: usize) -> u64 {
        virtio_read_device_config_8(self.sc(), off as i32)
    }
    fn write_config_1(&self, off: usize, v: u8) {
        virtio_write_device_config_1(self.sc(), off as i32, v);
    }
    fn write_config_2(&self, off: usize, v: u16) {
        virtio_write_device_config_2(self.sc(), off as i32, v);
    }
    fn write_config_4(&self, off: usize, v: u32) {
        virtio_write_device_config_4(self.sc(), off as i32, v);
    }
    fn write_config_8(&self, off: usize, v: u64) {
        virtio_write_device_config_8(self.sc(), off as i32, v);
    }
    fn read_queue_size(&self, index: u16) -> u16 {
        virtio_read_queue_size(self.sc(), index)
    }
    /// The shadow at `q.index` given the queue's members, then the transport's entry; no
    /// shadow yet (before `attach_finish`): nothing (the module's deviations).
    fn setup_queue(&self, q: &QueueSetup) {
        let sc = self.sc();
        if let Some(vq) = sc.vqs().get(usize::from(q.index)) {
            shadow_fill(vq, q);
            virtio_setup_queue(sc, vq, q.addr);
        }
    }
    fn setup_intrs(&self) {
        let sc = self.sc();
        (sc.ops().setup_intrs)(sc);
    }
    fn get_status(&self) -> i32 {
        virtio_get_status(self.sc())
    }
    fn set_status(&self, status: i32) {
        virtio_set_status(self.sc(), status);
    }
    /// `sc_driver_features = driver`, `neg_features` (the names as an empty table when
    /// given: `VIRTIO_DEBUG` is 0, so the transports read only whether there is one), then the
    /// three members the transport set.
    fn negotiate_features(
        &self,
        driver: u64,
        names: Option<&[NativeFeatureName]>,
    ) -> Result<Features, Errno> {
        let sc = self.sc();
        let none: [VirtioFeatureName; 0] = [];
        sc.sc_driver_features.set(driver);
        virtio_negotiate_features(sc, names.map(|_| &none[..]))?;
        Ok(Features {
            active: sc.sc_active_features.get(),
            indirect: sc.sc_indirect.get() != 0,
            version_1: sc.sc_version_1.get() != 0,
        })
    }
    /// What a legacy child sets before `virtio_attach_finish`: the shadow queues
    /// (`sc_vqs`, `sc_nvqs`), `sc_config_change` when the child has a handler, `sc_native`;
    /// then the transport's entry with its own arguments.
    fn attach_finish(
        &self,
        va: &VirtioArgs,
        child: ChildRef,
        queues: &[QueueSetup],
    ) -> Result<(), Errno> {
        let sc = self.sc();
        for (i, q) in queues.iter().enumerate() {
            if usize::from(q.index) != i {
                panic(format_args!(
                    "{}: virtqueue {} of a native child at {i}",
                    self.dev.xname(),
                    q.index
                ));
            }
        }
        if !queues.is_empty() {
            let Some(vqs) = mallocarray(
                queues.len(),
                size_of::<crate::dev::pv::virtiovar::Virtqueue>(),
                M_DEVBUF,
                M_NOWAIT | M_ZERO,
            ) else {
                return Err(Errno::ENOMEM);
            };
            sc.sc_vqs.set(vqs.as_ptr().cast());
            sc.sc_nvqs.set(queues.len() as i32);
            for (vq, q) in sc.vqs().iter().zip(queues) {
                vq.vq_owner.set(ptr::from_ref(sc));
                shadow_fill(vq, q);
            }
        }
        if child.has_config_change() {
            sc.sc_config_change.set(Some(native_config_change));
        }
        if sc.sc_native.set(child).is_err() {
            panic(format_args!("{}: a second native child", self.dev.xname()));
        }
        (sc.ops().attach_finish)(sc, ptr::with_exposed_provenance_mut(va.legacy))
    }
    fn poll_intr(&self) -> i32 {
        virtio_poll_intr(self.sc())
    }
    fn intr_barrier(&self) {
        virtio_intr_barrier(self.sc());
    }
    fn dma(&self) -> &'static dyn DmaMemory {
        self.dma
    }
    /// `sc_child = self` (the child's legacy header: a kernel node's shadow, whose
    /// `dv_cfdata` the transport's negotiation reads for the child's flags), `sc_ipl`.
    fn claim(&self, child: &DevRef, ipl: i32) -> bool {
        let sc = self.sc();
        if !sc.sc_child.get().is_null() {
            return false;
        }
        sc.sc_child.set(ptr::from_ref(legacy_device(child)));
        sc.sc_ipl.set(ipl);
        true
    }
    fn child_failed(&self) {
        self.sc().sc_child.set(VIRTIO_CHILD_ERROR);
    }
    fn xname(&self) -> &str {
        self.dev.xname()
    }
}

/// A shadow queue given a kernel queue's index, size and ring offsets (no ring: the kernel
/// queue's).
fn shadow_fill(vq: &crate::dev::pv::virtiovar::Virtqueue, q: &QueueSetup) {
    vq.vq_index.set(i32::from(q.index));
    vq.vq_num.set(u32::from(q.num));
    vq.vq_mask.set(u32::from(q.num).saturating_sub(1));
    vq.vq_availoffset.set(q.avail_off as i32);
    vq.vq_usedoffset.set(q.used_off as i32);
}

impl LegacyDmaMemory {
    /// What [`DmaMemory::alloc`] made before a step failed, given back in reverse:
    /// `bus_dmamap_destroy`, `bus_dmamem_unmap`, `bus_dmamem_free`.
    fn release(
        &self,
        seg: BusDmaSegment,
        va: Option<(NonNull<u8>, usize)>,
        map: Option<&'static BusDmamap>,
    ) {
        if let Some(map) = map {
            // SAFETY: the map came from `bus_dmamap_create` of this tag and is unloaded; this
            // is its last use (the caller gives it up: no record of it remains).
            unsafe { bus_dmamap_destroy(self.tag, NonNull::from(map)) };
        }
        if let Some((kva, len)) = va {
            // SAFETY: `kva` is `bus_dmamem_map`'s mapping of `len` bytes of this tag's `seg`,
            // which nothing reaches any more (its map is gone or never loaded it, and the
            // `DmaBuf` that read it is dropped).
            unsafe { bus_dmamem_unmap(self.tag, kva, len) };
        }
        // SAFETY: `seg` is `bus_dmamem_alloc`'s segment of this tag, unmapped above (or never
        // mapped), given back once.
        unsafe { bus_dmamem_free(self.tag, &[seg]) };
    }
}

// The provider's contract (`hal::dev::dma::DmaMemory`): a block is `bus_dmamem_map`'s fresh
// mapping of `bus_dmamem_alloc`'s segment, read and write, holding no Rust object, the
// caller's alone until `free`; its `seg` is the loaded map's, what the device is told.
impl DmaMemory for LegacyDmaMemory {
    fn alloc(&self, req: &DmaReq) -> Result<DmaBlock, Errno> {
        let wait = if req.nowait {
            BUS_DMA_NOWAIT
        } else {
            BUS_DMA_WAITOK
        };
        let mut flags = wait;
        if req.zero {
            flags |= BUS_DMA_ZERO;
        }
        if req.coherent {
            flags |= BUS_DMA_COHERENT;
        }
        if req.nocache {
            flags |= BUS_DMA_NOCACHE;
        }
        let mut segs = [BusDmaSegment::default(); 1];
        match req.range {
            Some((low, high)) => bus_dmamem_alloc_range(
                self.tag,
                req.size,
                req.align,
                req.boundary,
                &mut segs,
                flags,
                low as usize,
                high as usize,
            )?,
            None => bus_dmamem_alloc(
                self.tag,
                req.size,
                req.align,
                req.boundary,
                &mut segs,
                flags,
            )?,
        };
        let seg = segs[0];
        let kva = match bus_dmamem_map(self.tag, &mut segs, req.size, flags) {
            Ok(kva) => kva,
            Err(e) => {
                self.release(seg, None, None);
                return Err(e);
            }
        };
        let mapped = Some((kva, req.size));
        let map_flags = if req.dma64 {
            wait | BUS_DMA_64BIT
        } else {
            wait
        };
        let map = match bus_dmamap_create(self.tag, req.size, 1, req.size, req.boundary, map_flags)
        {
            Ok(map) => map,
            Err(e) => {
                self.release(seg, mapped, None);
                return Err(e);
            }
        };
        // SAFETY: `kva` maps `req.size` bytes of this tag's memory, which stay mapped until
        // `free` unloads the map before unmapping them.
        let load =
            unsafe { bus_dmamap_load(self.tag, map, kva.as_ptr(), req.size, None, BUS_DMA_NOWAIT) };
        if let Err(e) = load {
            self.release(seg, mapped, Some(map));
            return Err(e);
        }
        let loaded = map
            .dm_segs()
            .first()
            .map(core::cell::Cell::get)
            .unwrap_or_default();
        let Some(record) = MBox::new(M_DEVBUF, H_NOWAIT, DmaRecord { map, seg }) else {
            bus_dmamap_unload(self.tag, map);
            self.release(seg, mapped, Some(map));
            return Err(Errno::ENOMEM);
        };
        let record: &'static DmaRecord = MBox::leak(record);
        Ok(DmaBlock {
            va: kva,
            len: seg.ds_len,
            seg: DmaSeg {
                addr: loaded.ds_addr as u64,
                len: loaded.ds_len,
            },
            coherent: false,
            cookie: ptr::from_ref(record).expose_provenance(),
        })
    }

    fn free(&self, block: DmaBlock) {
        // SAFETY: the cookie is the exposed address of the `DmaRecord` `alloc` leaked for this
        // block (`MBox::leak`: never freed, so the reference is valid for ever), handed back
        // unchanged by the block's `DmaBuf` and only once.
        let record: &DmaRecord = unsafe { &*ptr::with_exposed_provenance(block.cookie) };
        bus_dmamap_unload(self.tag, record.map);
        self.release(
            record.seg,
            Some((block.va, block.len.min(record.seg.ds_len))),
            Some(record.map),
        );
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the presenter (the adapter's members are the shim's tests,
    // `kern/subr_autoconf.rs`, which have the test machine's tables).

    use std::{assert, assert_eq};

    use super::*;

    #[test]
    fn the_presenter_lends_the_legacy_arguments_unchanged() {
        let mut vma = 0u64;
        let addr = ptr::from_mut(&mut vma).expose_provenance();
        let args = VirtioArgs {
            devid: 4,
            nintr: 1,
            transport: kernel::dev::pv::virtio::fake_transport,
            legacy: addr,
        };
        let mut lent = None;
        assert!(present(&args, &mut |a| lent = Some(a.addr())));
        assert_eq!(lent, Some(addr));

        // A kernel transport's arguments, and another bus's, lend nothing.
        let mut lent = None;
        let native = VirtioArgs { legacy: 0, ..args };
        assert!(!present(&native, &mut |a| lent = Some(a.addr())));
        assert!(!present(&5u32, &mut |a| lent = Some(a.addr())));
        assert_eq!(lent, None);
    }
}
/* </TESTS> */
