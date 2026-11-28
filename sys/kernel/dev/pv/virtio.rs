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
 * Copyright (c) 2012 Stefan Fritsch, Alexander Fiveg.
 * Copyright (c) 2010 Minoura Makoto.
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
 */

/*
 * Copyright (c) 2012 Stefan Fritsch.
 * Copyright (c) 2010 Minoura Makoto.
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
 */

/*
 * Part of the file derived from `Virtio PCI Card Specification v0.8.6 DRAFT'
 * Appendix A.
 */
/* An interface for efficient virtio implementation.
 *
 * This header is BSD licensed so anyone can use the definitions
 * to implement compatible drivers/servers.
 *
 * Copyright 2007, 2009, IBM Corporation
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
 * 3. Neither the name of IBM nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL IBM OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! The virtio core in the kernel, `virtio(4)`: a child's virtqueue ([`Virtqueue`]: the
//! descriptor rings, enqueue and dequeue, interrupt suppression), what a child asks of its
//! transport ([`VirtioTransport`]: `struct virtio_ops` and the softc members a child sets
//! through its parent), what the transport's interrupt calls back ([`VirtioChild`],
//! [`ChildRef`]), and the device-independent half of attach and reset.
//!
//! Upstream: sys/dev/pv/virtio.c @ 3ce1f3f79392
//! Upstream: sys/dev/pv/virtiovar.h @ 3ce1f3f79392
//! LZ: sys/dev/pv/virtio.rs@e8249449d905
//! LZ: sys/dev/pv/virtiovar.rs@e8249449d905
//!
//! A transport (`virtio_mmio`, `virtio_pci`, `bsd`'s until N7.3d) finds a device and offers
//! its child a [`VirtioArgs`] ([`Aux::Typed`]); the child makes its transport from it
//! ([`VirtioArgs::transport`]), claims the device ([`VirtioTransport::claim`]: `sc_child`,
//! `sc_ipl`), negotiates features, allocates its queues ([`Virtqueue::alloc`]) and finishes
//! with [`virtio_attach_finish`], which hands the transport the child's [`ChildRef`]: the
//! transport's interrupt then calls [`ChildRef::check_vq`] for each queue (the C's
//! `virtio_check_vq` and the queue's `vq_done`) and [`ChildRef::config_change`].
//!
//! A queue is a plain value inside the cell that guards its driver's state (the C has no
//! lock in it: the child's mutex, or the kernel lock and an `spl`, guards it). Its rings are
//! one [`DmaBuf`] of the C's size and layout, reached field by field at the offsets of
//! [`crate::dev::pv::virtioreg`]; the free list is threaded through the entries by index.
//!
//! Typical usage (the C's comment, with this module's names):
//! ```text
//!     let slot = vq.enqueue_prep()?;               // allocate a slot (EAGAIN: full)
//!     vq.enqueue_reserve(slot, nsegs)?;            // the descriptor chain (EAGAIN)
//!     <sync the command and the payload>
//!     vq.enqueue(slot, &cmd_segs, true);           // the device reads these
//!     vq.enqueue(slot, &payload_segs, false);      // the device writes these
//!     vq.enqueue_commit(t, Some(slot), true);      // publish, kick unless told not to
//!
//!     <interrupt> vq.check(), then while let Ok((slot, len)) = vq.dequeue() {
//!         ..; vq.dequeue_commit(slot);             // the slots back on the free list
//!     }
//! ```
//! To reset a device to a known state: [`virtio_reset`] (the device stops; `dequeue` still
//! works), revoke the pending requests, [`virtio_reinit_start`] (dequeue prohibited), the
//! child's own initialization, [`virtio_reinit_end`] (the device active, enqueue allowed).
//! Once attached, features are assumed not to change again.
//!
//! ## Deviations
//! - The barriers (`virtio_membar_producer`, `_consumer`, `_sync`) are
//!   `core::sync::atomic::fence(Release)`, `(Acquire)` and `(SeqCst)` (P35): on arm64 LLVM
//!   emits `dmb ish`/`dmb ishld` where the C has `dmb st`/`dmb ld`/`dmb sy`, at least as
//!   strong towards another CPU, which is where QEMU's device model runs (`DmaBuf::sync`
//!   stands on the same choice); on amd64 nothing, nothing and `mfence`, as the C.
//! - One `DmaBuf::alloc` stands for the C's `bus_dmamem_alloc_range`, `bus_dmamem_map`,
//!   `bus_dmamap_create` and `bus_dmamap_load` (U13, the user's decision of 2026-11-28): the
//!   four failure messages are the first, `virtqueue %d for %s allocation failed, error %d`,
//!   whichever step failed. They print only when memory runs out during an attach.
//! - Stricter than the C: a slot the device hands back that is not a slot of the queue
//!   (`vq_entries[slot]` unchecked, `virtio.c:867`), a chain walked onto one, `enqueue` or
//!   `enqueue_p` of a slot `enqueue_reserve` never built, and `enqueue_trim` of a direct chain
//!   past the queue are `panic(9)` naming the queue; the C's `VIRTIO_ASSERT`s are
//!   `kdassert!` (`option DEBUG`).
//! - [`virtio_reinit_start`] after a renegotiation that fails (the transport then marks the
//!   device `FAILED`) initialises the queues without features, where the C keeps what the
//!   transport set before it failed. No child resets its device in N7.3a.
//! - `virtio_check_vqs`, `virtio_log_features`, `virtio_vq_dump`, `virtio_intr_name` and
//!   `virtio_cd` are the transports' and stay `bsd`'s (`dev/pv/virtio.rs`) until a transport
//!   is the kernel's (N7.3d); `virtio_intr_establish` comes with its first caller (vio's
//!   per-queue vectors, N7.3b). `VIRTIO_DEBUG` is 0, as the C's, and its messages are
//!   compiled behind `if VIRTIO_DEBUG > 0`.
//!
//! ## Redesign
//! - LZ's `struct virtqueue` was the C's, a zeroed record of `Cell`s and raw pointers into the
//!   ring (`vq_desc`, `vq_avail`, `vq_used`, `vq_indirect`), read and written by volatile
//!   helpers in `unsafe` blocks, with `vq_owner` back to the softc and a `queue.rs` free list
//!   through the entries. Here [`Virtqueue`] owns its ring (a [`DmaBuf`], so the mapping,
//!   the segment and the provider that frees it go together), its entries (an `MArray` of
//!   `M_DEVBUF`) and its free list (an index threaded through the entries, `free` its head),
//!   and every method that changes it takes `&mut self`; the transport is a parameter where
//!   the C follows `vq_owner`, so a queue holds no pointer back. `event_idx` is copied at
//!   `alloc` and `init` (the C reads `virtio_has_feature(vq_owner, ..)` at each commit).
//!   `vq_free_entry`'s list count at `virtio_free_vq` is a counter (`nfree`), O(1).
//! - `struct virtio_ops` and the softc members the child sets (`sc_child`, `sc_ipl`,
//!   `sc_driver_features`, `sc_nvqs`/`sc_vqs`, `sc_config_change`) -> the trait
//!   [`VirtioTransport`], received as a value (`Box<dyn VirtioTransport>`, made for the child
//!   that attaches, P38); `bsd`'s implementation over the legacy transports is
//!   `dev/pv/virtio_native.rs` (a `bridges.toml` row, N7.3d). `sc_active_features`,
//!   `sc_indirect`, `sc_version_1` -> [`Features`], the answer of the negotiation, which the
//!   child keeps.
//! - `vq_done` and `sc_config_change` (function pointers the child writes into its parent's
//!   softc, which cast the softc back to the child's) -> the trait [`VirtioChild`] of the
//!   child's state, handed to the transport as a [`ChildRef`] (the state's counted handle
//!   with its type erased and the two functions monomorphised: one `TypeId` compare a call).
//! - `struct virtio_attach_args` -> [`VirtioArgs`], `'static` under [`Aux::Typed`], which also
//!   carries the transport's constructor and, while the transports are `bsd`'s, their own
//!   arguments for a legacy child ([`VirtioArgs::legacy`], scaffolding).
//! - A queue the device was told about ([`Virtqueue::setup`]) and not told to forget
//!   ([`Virtqueue::free`]) keeps its ring when it is dropped (the pages are never freed, and
//!   the drop says so once), where the C's error paths leak the whole queue; the request a
//!   queue holds while the device has it (the C's `qe_vr_index`) is N7.4c's, with vioblk
//!   (P43).

use alloc::boxed::Box;
use core::any::Any;
use core::sync::atomic::{AtomicBool, Ordering, fence};

use hal::dev::dma::{DmaBuf, DmaMemory, DmaReq, DmaSeg, DmaSync};
use hal::mem::malloc::{M_NOWAIT, MArc, MArray};
use hal::mem::pool::AnyArc;
use sysdefs::base::errno::Errno;

use crate::assert::{kdassert, kpanic};
use crate::base::prf::printf;
use crate::dev::device::{Aux, DevRef, M_DEVBUF};
use crate::dev::pv::virtioreg::{
    VIRTIO_CONFIG_DEVICE_STATUS_ACK, VIRTIO_CONFIG_DEVICE_STATUS_DRIVER,
    VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK, VIRTIO_CONFIG_DEVICE_STATUS_RESET,
    VIRTIO_F_RING_EVENT_IDX, VIRTIO_F_RING_INDIRECT_DESC, VIRTIO_F_VERSION_1, VIRTIO_PAGE_SIZE,
    VRING_AVAIL_F_NO_INTERRUPT, VRING_AVAIL_FLAGS, VRING_AVAIL_IDX, VRING_AVAIL_RING,
    VRING_DESC_ADDR, VRING_DESC_F_INDIRECT, VRING_DESC_F_NEXT, VRING_DESC_F_WRITE,
    VRING_DESC_FLAGS, VRING_DESC_LEN, VRING_DESC_NEXT, VRING_DESC_SIZE, VRING_USED_ELEM_ID,
    VRING_USED_ELEM_LEN, VRING_USED_ELEM_SIZE, VRING_USED_F_NO_NOTIFY, VRING_USED_FLAGS,
    VRING_USED_IDX, VRING_USED_RING,
};

/// `VIRTIO_DEBUG`: 0, as the C defines it unless the build says otherwise.
pub const VIRTIO_DEBUG: i32 = 0;

// Flags for config(8).

/// `VIRTIO_CF_NO_INDIRECT`.
pub const VIRTIO_CF_NO_INDIRECT: i32 = 1;
/// `VIRTIO_CF_NO_EVENT_IDX`.
pub const VIRTIO_CF_NO_EVENT_IDX: i32 = 2;
/// `VIRTIO_CF_PREFER_VERSION_09`.
pub const VIRTIO_CF_PREFER_VERSION_09: i32 = 8;

/// `VIRTQUEUE_ALIGN(n)`: rounds up to the virtio page size.
const fn virtqueue_align(n: usize) -> usize {
    (n + (VIRTIO_PAGE_SIZE - 1)) & !(VIRTIO_PAGE_SIZE - 1)
}

/// The rings' memory: below `VIRTIO_PAGE_SIZE << 32` (virtio 0.9's 32-bit page frame number;
/// the C keeps the limit for 1.0 too).
const RING_HIGH: u64 = ((VIRTIO_PAGE_SIZE as u64) << 32) - 1;

/// `struct virtio_attach_args`, with what the child reaches its parent by.
pub struct VirtioArgs {
    /// `va_devid`: the virtio device id (`PCI_PRODUCT_VIRTIO_*`).
    pub devid: i32,
    /// `va_nintr`: the number of interrupt vectors.
    pub nintr: u32,
    /// `(struct virtio_softc *)parent`: the parent's transport, made for the child that
    /// attaches, given the parent's node.
    pub transport: fn(&DevRef) -> Box<dyn VirtioTransport>,
    /// Scaffolding (docs/SCAFFOLDING.md): the legacy transport's own arguments
    /// (`struct virtio_mmio_attach_args`, `struct virtio_pci_attach_args`) as an address whose
    /// provenance was exposed, lent to a legacy child by `bsd`'s presenter and never read
    /// here; 0 from a transport of the kernel's.
    pub legacy: usize,
}

impl VirtioArgs {
    /// The bus's arguments, when they are a virtio transport's.
    pub fn of_aux<'a>(aux: &'a Aux<'_>) -> Option<&'a VirtioArgs> {
        match aux {
            Aux::Typed(a) => {
                let a: &'a (dyn Any + Send + Sync) = *a;
                a.downcast_ref::<VirtioArgs>()
            }
            Aux::Legacy(_) => None,
        }
    }
}

/// `struct virtio_feature_name`: a feature bit's name, for `VIRTIO_DEBUG`'s listing.
#[derive(Clone, Copy, Debug)]
pub struct VirtioFeatureName {
    /// `bit`.
    pub bit: u64,
    /// `name`.
    pub name: &'static str,
}

/// What the negotiation settled: `sc_active_features`, `sc_indirect`, `sc_version_1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Features {
    /// `sc_active_features`: the bits both sides have.
    pub active: u64,
    /// `sc_indirect`: indirect descriptors are on.
    pub indirect: bool,
    /// `sc_version_1`: the device speaks virtio 1.0.
    pub version_1: bool,
}

impl Features {
    /// `virtio_has_feature(sc, fbit)`.
    #[inline]
    pub fn has(&self, fbit: u64) -> bool {
        self.active & fbit != 0
    }

    /// The features `active` makes (a transport's answer: indirect descriptors and version 1
    /// as their bits say).
    pub fn of_active(active: u64) -> Features {
        Features {
            active,
            indirect: active & VIRTIO_F_RING_INDIRECT_DESC != 0,
            version_1: active & VIRTIO_F_VERSION_1 != 0,
        }
    }
}

/// What `setup_queue` reads of a queue: its index, size, ring address and the two rings'
/// offsets (`vq_index`, `vq_num`, `vq_dmamap->dm_segs[0].ds_addr`, `vq_availoffset`,
/// `vq_usedoffset`); `addr` 0 tells the device the queue is gone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueueSetup {
    /// `vq_index`.
    pub index: u16,
    /// `vq_num`.
    pub num: u16,
    /// The ring's bus address; 0: no queue any more.
    pub addr: u64,
    /// `vq_availoffset`: the avail ring's offset in the ring.
    pub avail_off: u32,
    /// `vq_usedoffset`: the used ring's offset in the ring.
    pub used_off: u32,
}

/// Which descriptor table a request's chain is in (`qe_indirect` and `qe_desc_base`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Chain {
    /// The queue's own table (`vq_desc`).
    #[default]
    Direct,
    /// The slot's part of the indirect area (`vq_indirect + vq_maxnsegs * slot`).
    Indirect,
}

/// `struct vq_entry`: one slot of a queue; its index in `entries` is `qe_index`.
#[derive(Clone, Copy, Debug, Default)]
struct VqEntry {
    /// `qe_list`: the next free slot, while this one is free.
    next_free: Option<u16>,
    /// `qe_next`: the next descriptor `enqueue` fills; `None` until `enqueue_reserve` (the C's
    /// -1).
    next: Option<u16>,
    /// `qe_indirect` with `qe_desc_base`: the table of the chain, for a head.
    chain: Chain,
}

/// `struct virtqueue`: one queue of a device, owned by its child's state.
pub struct Virtqueue {
    /// `vq_vaddr`, `vq_segs`, `vq_dmamap`, `vq_bytesize`: the rings, one segment. `None` only
    /// inside the drop of a queue the device still knows.
    ring: Option<DmaBuf>,
    /// `vq_num`: the queue size, a power of two.
    num: u16,
    /// `vq_mask`: `vq_num - 1`.
    mask: u16,
    /// `vq_index`: the queue number (0, 1, ...).
    index: u16,
    /// `vq_availoffset`: `sizeof(struct vring_desc) * vq_num`.
    avail_off: u32,
    /// `vq_usedoffset`.
    used_off: u32,
    /// `vq_indirectoffset`, when `vq_indirect` is not NULL.
    indirect_off: Option<u32>,
    /// `vq_maxnsegs`: the descriptors of one slot's indirect table.
    maxnsegs: usize,
    /// `vq_entries`.
    entries: MArray<VqEntry>,
    /// `vq_freelist`'s head.
    free: Option<u16>,
    /// The slots on the free list (the C counts them at `virtio_free_vq`).
    nfree: u16,
    /// `vq_avail_idx`: the next avail ring entry this side fills.
    avail_idx: u16,
    /// `vq_used_idx`: the next used ring entry this side reads.
    used_idx: u16,
    /// `vq_queued`: the avail ring changed since the last `check`.
    queued: i32,
    /// `virtio_has_feature(vq_owner, VIRTIO_F_RING_EVENT_IDX)`, copied at `alloc` and `init`.
    event_idx: bool,
    /// The device was told the ring's address (`setup`) and not told to forget it (`free`).
    live: bool,
}

/// The drop of a queue the device still knows has said so (once for the kernel's life).
static DROPPED_LIVE: AtomicBool = AtomicBool::new(false);

impl Virtqueue {
    /// `virtio_alloc_vq`: queue `index` of the device behind `t`, its rings laid out as the
    /// C's (the descriptors, the avail ring and its event word, padding, the used ring and its
    /// event word, padding, then `maxnsegs` indirect descriptors per slot when indirect
    /// descriptors are on and `maxnsegs > 1`), initialised. `maxnsegs` 1 disables indirect
    /// descriptors for this queue. `ENXIO` when the device has no such queue, the allocation's
    /// error otherwise, each with the C's message; `panic(9)` for a size that is not a power
    /// of two.
    pub fn alloc(
        t: &dyn VirtioTransport,
        f: &Features,
        index: u16,
        maxnsegs: usize,
        name: &str,
    ) -> Result<Virtqueue, Errno> {
        let vq_size = t.read_queue_size(index);
        if vq_size == 0 {
            printf(format_args!(
                "virtqueue not exist, index {index} for {name}\n"
            ));
            return Err(Errno::ENXIO);
        }
        if !vq_size.is_power_of_two() {
            kpanic(format_args!("vq_size not power of two: {vq_size}"));
        }
        let n = usize::from(vq_size);

        let hdrlen = if f.has(VIRTIO_F_RING_EVENT_IDX) { 3 } else { 2 };
        // allocsize1: descriptor table + avail ring + pad
        let allocsize1 = virtqueue_align(VRING_DESC_SIZE * n + 2 * (hdrlen + n));
        // allocsize2: used ring + pad
        let allocsize2 = virtqueue_align(2 * hdrlen + VRING_USED_ELEM_SIZE * n);
        // allocsize3: indirect table
        let allocsize3 = if f.indirect && maxnsegs > 1 {
            VRING_DESC_SIZE * maxnsegs * n
        } else {
            0
        };
        let allocsize = allocsize1 + allocsize2 + allocsize3;

        // alloc and map the memory
        //
        // With virtio 0.9, the ring memory must be in the lowest 2^32 pages. For simplicity,
        // we use this limit even for virtio 1.0.
        let req = DmaReq {
            size: allocsize,
            align: VIRTIO_PAGE_SIZE,
            range: Some((0, RING_HIGH)),
            nowait: true,
            dma64: true,
            ..DmaReq::default()
        };
        let ring = match DmaBuf::alloc(t.dma(), &req) {
            Ok(ring) => ring,
            Err(e) => {
                printf(format_args!(
                    "virtqueue {index} for {name} allocation failed, error {}\n",
                    e as i32
                ));
                return Err(e);
            }
        };

        // free slot management
        let entries =
            MArray::new(M_DEVBUF, M_NOWAIT, n, |_| VqEntry::default()).ok_or(Errno::ENOMEM)?;

        let mut vq = Virtqueue {
            ring: Some(ring),
            num: vq_size,
            mask: vq_size - 1,
            index,
            avail_off: (VRING_DESC_SIZE * n) as u32,
            used_off: allocsize1 as u32,
            indirect_off: (allocsize3 > 0).then_some((allocsize1 + allocsize2) as u32),
            maxnsegs,
            entries,
            free: None,
            nfree: 0,
            avail_idx: 0,
            used_idx: 0,
            queued: 0,
            event_idx: false,
            live: false,
        };
        vq.init(f);

        if VIRTIO_DEBUG > 0 {
            printf(format_args!(
                "\nallocated {allocsize} byte for virtqueue {index} for {name}, size {vq_size}\n"
            ));
            if allocsize3 > 0 {
                printf(format_args!(
                    "using {allocsize3} byte ({} entries) indirect descriptors\n",
                    maxnsegs * n
                ));
            }
        }
        Ok(vq)
    }

    /// `virtio_free_vq`: the device told the queue is gone (`setup_queue` of address 0), then
    /// its memory given back. The device must be deactivated already; a queue with a request
    /// still out is handed back with `EBUSY` and the C's message.
    #[allow(
        clippy::result_large_err,
        reason = "a busy queue goes back to its owner whole, as the C leaves it in place"
    )]
    pub fn free(mut self, t: &dyn VirtioTransport) -> Result<(), (Virtqueue, Errno)> {
        // device must be already deactivated
        // confirm the vq is empty
        if self.nfree != self.num {
            printf(format_args!(
                "{}: freeing non-empty vq, index {}\n",
                t.xname(),
                self.index
            ));
            return Err((self, Errno::EBUSY));
        }

        // tell device that there's no virtqueue any longer
        let mut q = self.queue_setup();
        q.addr = 0;
        t.setup_queue(&q);
        self.live = false;
        Ok(())
    }

    /// `virtio_init_vq`: the rings zeroed, every slot's indirect table chained, every slot on
    /// the free list in rising order (`enqueue_trim` needs monotonely rising entries, so they
    /// are linked in reverse), both indexes 0. `f` gives the event index feature again (a
    /// reset may have renegotiated).
    pub fn init(&mut self, f: &Features) {
        let vq_size = self.num;
        kdassert!(vq_size > 0);
        self.event_idx = f.has(VIRTIO_F_RING_EVENT_IDX);
        let len = self.ring().len();
        self.ring_mut().zero(0, len);

        // build the indirect descriptor chain
        if self.indirect_off.is_some() {
            for i in 0..vq_size {
                let base = self.table(i, Chain::Indirect);
                for j in 0..self.maxnsegs.saturating_sub(1) {
                    self.set_d_next(base, j as u16, (j + 1) as u16);
                }
            }
        }

        // free slot management
        self.free = None;
        for i in (0..vq_size).rev() {
            let head = self.free;
            self.entry_mut(i).next_free = head;
            self.free = Some(i);
        }
        self.nfree = vq_size;

        self.ring().sync(0, len, DmaSync::PREWRITE);
        // enqueue/dequeue status
        self.avail_idx = 0;
        self.used_idx = 0;
        self.sync_uring(DmaSync::PREREAD);
        self.queued = 1;
    }

    /// What the transport's `setup_queue` reads of this queue, the device being told of it:
    /// from now on a drop keeps the ring (see the module's redesign) until [`Virtqueue::free`].
    pub fn setup(&mut self) -> QueueSetup {
        self.live = true;
        self.queue_setup()
    }

    /// The queue's [`QueueSetup`], its ring's bus address included.
    fn queue_setup(&self) -> QueueSetup {
        QueueSetup {
            index: self.index,
            num: self.num,
            addr: self.ring().seg().addr,
            avail_off: self.avail_off,
            used_off: self.used_off,
        }
    }

    /// `vq_index`.
    pub fn index(&self) -> u16 {
        self.index
    }

    /// `vq_num`.
    pub fn num(&self) -> u16 {
        self.num
    }

    /// `vq_maxnsegs`.
    pub fn maxnsegs(&self) -> usize {
        self.maxnsegs
    }

    /// The slots on the free list.
    pub fn nfree(&self) -> u16 {
        self.nfree
    }

    /// `vq_queued`.
    pub fn queued(&self) -> i32 {
        self.queued
    }

    /// `virtio_enqueue_prep`: a slot for a new request, its chain not built yet; `EAGAIN`
    /// when every slot is out.
    pub fn enqueue_prep(&mut self) -> Result<u16, Errno> {
        let slot = self.alloc_entry().ok_or(Errno::EAGAIN)?;
        // next slot is not allocated yet
        self.entry_mut(slot).next = None;
        Ok(slot)
    }

    /// `virtio_enqueue_reserve`: the remaining slots and the descriptor chain of `nsegs`
    /// descriptors for the request at `slot`: one ring slot pointing at the slot's indirect
    /// table when indirect descriptors are on and `1 < nsegs <= maxnsegs`, else a chain of
    /// `nsegs` slots in the queue's table. `EAGAIN` when the slots run out, the request then
    /// aborted (its slots back on the free list).
    pub fn enqueue_reserve(&mut self, slot: u16, nsegs: usize) -> Result<(), Errno> {
        kdassert!(self.entry(slot).next.is_none());
        kdassert!(1 <= nsegs && nsegs <= usize::from(self.num));

        if let Some(ind) = self.indirect_off
            && nsegs > 1
            && nsegs <= self.maxnsegs
        {
            self.entry_mut(slot).chain = Chain::Indirect;

            let tbl = self.table(slot, Chain::Indirect);
            let addr = self.ring().seg().addr
                + u64::from(ind)
                + (VRING_DESC_SIZE * self.maxnsegs * usize::from(slot)) as u64;
            self.set_d_buf(0, slot, addr, (VRING_DESC_SIZE * nsegs) as u32);
            self.set_d_flags(0, slot, VRING_DESC_F_INDIRECT);

            for i in 0..nsegs - 1 {
                self.set_d_flags(tbl, i as u16, VRING_DESC_F_NEXT);
            }
            self.set_d_flags(tbl, (nsegs - 1) as u16, 0);
            self.entry_mut(slot).next = Some(0);
        } else {
            let e = self.entry_mut(slot);
            e.chain = Chain::Direct;
            e.next = Some(slot);
            let mut s = slot;
            for _ in 0..nsegs - 1 {
                let Some(qe) = self.alloc_entry() else {
                    self.set_d_flags(0, s, 0);
                    self.enqueue_abort(slot);
                    return Err(Errno::EAGAIN);
                };
                self.set_d_flags(0, s, VRING_DESC_F_NEXT);
                self.set_d_next(0, s, qe);
                s = qe;
            }
            self.set_d_flags(0, s, 0);
        }
        Ok(())
    }

    /// `virtio_enqueue` of one loaded buffer (`dm_segs`): its segments fill the next
    /// descriptors of `slot`'s chain; `write` is true when the device only reads them (the
    /// others get `VRING_DESC_F_WRITE`). More segments than `maxnsegs` is `panic(9)`, as the
    /// C's.
    pub fn enqueue(&mut self, slot: u16, segs: &[DmaSeg], write: bool) {
        let (base, mut s) = self.cursor(slot, "virtio_enqueue");
        kdassert!(!segs.is_empty());
        if segs.len() > self.maxnsegs {
            if VIRTIO_DEBUG > 0 {
                for (i, seg) in segs.iter().enumerate() {
                    printf(format_args!(
                        " {i} ({}): {:#x} {:x} \n",
                        i32::from(write),
                        seg.addr,
                        seg.len
                    ));
                }
            }
            kpanic(format_args!(
                "dmamap->dm_nseg {} > vq->vq_maxnsegs {}",
                segs.len(),
                self.maxnsegs
            ));
        }

        for seg in segs {
            self.set_d_buf(base, s, seg.addr, seg.len as u32);
            if !write {
                let flags = self.d_flags(base, s);
                self.set_d_flags(base, s, flags | VRING_DESC_F_WRITE);
            }
            s = self.d_next(base, s);
        }
        self.entry_mut(slot).next = Some(s);
    }

    /// `virtio_enqueue_p`: `[start, start + len)` of one segment as the next descriptor of
    /// `slot`'s chain.
    pub fn enqueue_p(&mut self, slot: u16, seg: DmaSeg, start: u64, len: u32, write: bool) {
        let (base, s) = self.cursor(slot, "virtio_enqueue_p");
        // XXX todo: handle more segments
        kdassert!(seg.len as u64 > start && seg.len as u64 >= start + u64::from(len));

        self.set_d_buf(base, s, seg.addr + start, len);
        if !write {
            let flags = self.d_flags(base, s);
            self.set_d_flags(base, s, flags | VRING_DESC_F_WRITE);
        }
        let next = self.d_next(base, s);
        self.entry_mut(slot).next = Some(next);
    }

    /// `publish_avail_idx`: the avail ring's entries, then its index, made visible to the
    /// device.
    fn publish_avail_idx(&mut self) {
        // first make sure the avail ring entries are visible to the device
        self.sync_aring(DmaSync::PREWRITE);

        fence(Ordering::Release); // virtio_membar_producer
        let off = self.avail_off as usize + VRING_AVAIL_IDX;
        let idx = self.avail_idx;
        self.ring_mut().write::<u16>(off, idx);
        // make the avail idx visible to the device
        self.sync_aring(DmaSync::PREWRITE);
        self.queued = 1;
    }

    /// `virtio_enqueue_commit`: the request at `slot` put on the avail ring (`None`: nothing
    /// new, `virtio_notify`); with `notify` the index is published and the device kicked
    /// unless it said it does not need it (its event index, or `VRING_USED_F_NO_NOTIFY`).
    pub fn enqueue_commit(&mut self, t: &dyn VirtioTransport, slot: Option<u16>, notify: bool) {
        if let Some(slot) = slot {
            self.sync_descs(DmaSync::PREWRITE);
            if self.entry(slot).chain == Chain::Indirect {
                self.sync_indirect(slot, DmaSync::PREWRITE);
            }
            let i = self.avail_idx & self.mask;
            self.avail_idx = self.avail_idx.wrapping_add(1);
            let off = self.avail_off as usize + VRING_AVAIL_RING + 2 * usize::from(i);
            self.ring_mut().write::<u16>(off, slot);
        }

        // notify:
        if notify {
            if self.event_idx {
                let o = self.avail_idx_published();
                let n = self.avail_idx;
                self.publish_avail_idx();

                fence(Ordering::SeqCst); // virtio_membar_sync
                self.sync_uring_avail_event(DmaSync::POSTREAD);
                if vring_need_event(self.avail_event(), n, o) {
                    t.kick(self.index);
                }
            } else {
                self.publish_avail_idx();

                fence(Ordering::SeqCst); // virtio_membar_sync
                self.sync_uring(DmaSync::POSTREAD);
                let off = self.used_off as usize + VRING_USED_FLAGS;
                if self.ring().read::<u16>(off) & VRING_USED_F_NO_NOTIFY == 0 {
                    t.kick(self.index);
                }
            }
        }
    }

    /// `virtio_notify(sc, vq)`: `virtio_enqueue_commit(sc, vq, -1, 1)`.
    pub fn notify(&mut self, t: &dyn VirtioTransport) {
        self.enqueue_commit(t, None, true);
    }

    /// `virtio_enqueue_abort`: the request at `slot` rolled back, its slots on the free list.
    pub fn enqueue_abort(&mut self, slot: u16) {
        if self.entry(slot).next.is_none() {
            self.free_entry(slot);
            return;
        }
        self.free_chain(slot);
    }

    /// `virtio_enqueue_trim`: the chain of a request on statically reserved slots cut to
    /// `nsegs` descriptors. A direct chain is assumed to be a contiguous block from `slot`
    /// (the C's N.B.).
    pub fn enqueue_trim(&mut self, slot: u16, nsegs: usize) {
        let (base, mut i) = if self.d_flags(0, slot) & VRING_DESC_F_INDIRECT == 0 {
            // N.B.: the entries are ASSUMED to be a contiguous block with slot being the
            // index to the first one.
            if usize::from(slot) + nsegs > usize::from(self.num) {
                kpanic(format_args!(
                    "virtqueue {}: trim of slot {slot} to {nsegs} past {}",
                    self.index, self.num
                ));
            }
            self.entry_mut(slot).next = Some(slot);
            (0, slot)
        } else {
            self.entry_mut(slot).next = Some(0);
            self.set_d_len(0, slot, (VRING_DESC_SIZE * nsegs) as u32);
            (self.table(slot, Chain::Indirect), 0)
        };

        for _ in 0..nsegs.saturating_sub(1) {
            self.set_d_flags(base, i, VRING_DESC_F_NEXT);
            i += 1;
        }
        self.set_d_flags(base, i, 0);
    }

    /// `virtio_check_vq` but the call of `vq_done`: the rings synced for reading, and whether
    /// the device used an entry since the last [`Virtqueue::dequeue`].
    pub fn check(&mut self) -> bool {
        if self.queued != 0 {
            self.queued = 0;
            self.sync_aring(DmaSync::POSTWRITE);
        }
        self.sync_uring(DmaSync::POSTREAD);
        self.used_idx != self.used_idx_device()
    }

    /// `virtio_dequeue`: the next request the device used, its slot and the length it wrote;
    /// `ENOENT` when there is none. The used ring must have been synced already, usually by
    /// [`Virtqueue::check`] in the interrupt: polling `dequeue` until it answers does not
    /// work.
    pub fn dequeue(&mut self) -> Result<(u16, u32), Errno> {
        if self.used_idx == self.used_idx_device() {
            return Err(Errno::ENOENT);
        }
        let usedidx = self.used_idx & self.mask;
        self.used_idx = self.used_idx.wrapping_add(1);

        fence(Ordering::Acquire); // virtio_membar_consumer
        self.sync_uring(DmaSync::POSTREAD);
        let elem =
            self.used_off as usize + VRING_USED_RING + VRING_USED_ELEM_SIZE * usize::from(usedidx);
        let id = self.ring().read::<u32>(elem + VRING_USED_ELEM_ID);
        let len = self.ring().read::<u32>(elem + VRING_USED_ELEM_LEN);
        let slot = match u16::try_from(id) {
            Ok(slot) if slot < self.num => slot,
            _ => kpanic(format_args!(
                "virtqueue {}: the device used slot {id} of {}",
                self.index, self.num
            )),
        };

        if self.entry(slot).chain == Chain::Indirect {
            self.sync_indirect(slot, DmaSync::POSTWRITE);
        }
        Ok((slot, len))
    }

    /// `virtio_dequeue_commit`: the request at `slot` complete, its slots on the free list
    /// again (forgetting it leaks them); the number of slots freed. Not for statically
    /// reserved slots trimmed with [`Virtqueue::enqueue_trim`].
    pub fn dequeue_commit(&mut self, slot: u16) -> usize {
        self.free_chain(slot)
    }

    /// `virtio_postpone_intr`: the used event index moved `nslots` past the last entry read,
    /// to delay the interrupt; `true` when the used ring has already advanced too far and the
    /// caller must process the queue again (else no more interrupts will happen).
    pub fn postpone_intr(&mut self, nslots: u16) -> bool {
        let idx = self.used_idx.wrapping_add(nslots);

        // set the new event index: avail_ring->used_event = idx
        self.set_used_event(idx);
        fence(Ordering::SeqCst); // virtio_membar_sync

        self.sync_aring_used_event(DmaSync::PREWRITE);
        self.queued += 1;

        nslots < self.nused()
    }

    /// `virtio_postpone_intr_smart`: the interrupt postponed until 3/4 of the available
    /// descriptors have been consumed.
    pub fn postpone_intr_smart(&mut self) -> bool {
        let pending = self.avail_idx_published().wrapping_sub(self.used_idx);
        let nslots = (u32::from(pending) * 3 / 4) as u16;

        self.postpone_intr(nslots)
    }

    /// `virtio_postpone_intr_far`: the interrupt postponed until all of the available
    /// descriptors have been consumed.
    pub fn postpone_intr_far(&mut self) -> bool {
        let nslots = self.avail_idx_published().wrapping_sub(self.used_idx);

        self.postpone_intr(nslots)
    }

    /// `virtio_stop_vq_intr`: the device asked not to interrupt for this queue; no guarantee.
    /// With event indexes there is no way to disable the interrupt completely: the used event
    /// moves half the index space away, which won't happen soon and is far enough in the past
    /// not to trigger a spurious interrupt.
    pub fn stop_intr(&mut self) {
        if self.event_idx {
            self.set_used_event(self.used_idx.wrapping_add(0x8000));
            self.sync_aring_used_event(DmaSync::PREWRITE);
        } else {
            let flags = self.avail_flags();
            self.set_avail_flags(flags | VRING_AVAIL_F_NO_INTERRUPT);
        }
        self.sync_aring(DmaSync::PREWRITE);
        self.queued += 1;
    }

    /// `virtio_start_vq_intr`: the device asked to interrupt again (with event indexes, the
    /// used event is the last entry read); `true` when it used entries meanwhile, which the
    /// caller must process.
    pub fn start_intr(&mut self) -> bool {
        if self.event_idx {
            self.set_used_event(self.used_idx);
            self.sync_aring_used_event(DmaSync::PREWRITE);
        } else {
            let flags = self.avail_flags();
            self.set_avail_flags(flags & !VRING_AVAIL_F_NO_INTERRUPT);
            self.sync_aring(DmaSync::PREWRITE);
        }

        fence(Ordering::SeqCst); // virtio_membar_sync

        self.queued += 1;

        self.sync_uring(DmaSync::POSTREAD);
        self.used_idx != self.used_idx_device()
    }

    /// `virtio_nused`: the entries of the used ring not read yet, which can be given to the
    /// avail ring again.
    pub fn nused(&self) -> u16 {
        self.sync_uring(DmaSync::POSTREAD);
        let n = self.used_idx_device().wrapping_sub(self.used_idx);
        kdassert!(n <= self.num);
        n
    }

    // Free descriptor management.

    /// `vq_alloc_entry`: the free list's head, taken off it.
    fn alloc_entry(&mut self) -> Option<u16> {
        let slot = self.free?;
        self.free = self.entry(slot).next_free;
        self.nfree -= 1;
        Some(slot)
    }

    /// `vq_free_entry`: `slot` back at the free list's head.
    fn free_entry(&mut self, slot: u16) {
        let head = self.free;
        self.entry_mut(slot).next_free = head;
        self.free = Some(slot);
        self.nfree += 1;
    }

    /// The slots of the direct chain that starts at `slot` (one, for an indirect request,
    /// whose head carries no `NEXT`) back on the free list, walked by the descriptors' `next`
    /// as the C's `virtio_enqueue_abort` and `virtio_dequeue_commit`; how many.
    fn free_chain(&mut self, slot: u16) -> usize {
        let mut qe = slot;
        let mut s = slot;
        let mut r = 1;
        while self.d_flags(0, s) & VRING_DESC_F_NEXT != 0 {
            s = self.d_next(0, s);
            self.free_entry(qe);
            qe = s;
            r += 1;
        }
        self.free_entry(qe);
        r
    }

    /// `&vq->vq_entries[slot]`; `panic(9)` naming the queue for a slot that is not one of it.
    fn entry(&self, slot: u16) -> &VqEntry {
        match self.entries.get(usize::from(slot)) {
            Some(e) => e,
            None => kpanic(format_args!(
                "virtqueue {}: slot {slot} of {}",
                self.index, self.num
            )),
        }
    }

    /// As [`Virtqueue::entry`], to change it.
    fn entry_mut(&mut self, slot: u16) -> &mut VqEntry {
        let (index, num) = (self.index, self.num);
        match self.entries.get_mut(usize::from(slot)) {
            Some(e) => e,
            None => kpanic(format_args!("virtqueue {index}: slot {slot} of {num}")),
        }
    }

    /// The descriptor table and the next descriptor of `slot`'s chain, which
    /// `enqueue_reserve` must have built.
    fn cursor(&self, slot: u16, what: &str) -> (usize, u16) {
        let e = self.entry(slot);
        match e.next {
            Some(s) => (self.table(slot, e.chain), s),
            None => kpanic(format_args!(
                "{what}: virtqueue {} slot {slot} not reserved",
                self.index
            )),
        }
    }

    // The rings, at the offsets of `virtioreg.rs`: every access a volatile load or store of
    // the `DmaBuf`, its range checked.

    /// The rings.
    fn ring(&self) -> &DmaBuf {
        match &self.ring {
            Some(r) => r,
            None => kpanic(format_args!("virtqueue {}: no ring", self.index)),
        }
    }

    /// The rings, to write them.
    fn ring_mut(&mut self) -> &mut DmaBuf {
        let index = self.index;
        match &mut self.ring {
            Some(r) => r,
            None => kpanic(format_args!("virtqueue {index}: no ring")),
        }
    }

    /// The byte offset of the descriptor table of `slot`'s chain (`qe_desc_base`): 0 for the
    /// queue's own, the slot's part of the indirect area otherwise.
    fn table(&self, slot: u16, chain: Chain) -> usize {
        match (chain, self.indirect_off) {
            (Chain::Indirect, Some(ind)) => {
                ind as usize + VRING_DESC_SIZE * self.maxnsegs * usize::from(slot)
            }
            _ => 0,
        }
    }

    /// `vd[i].flags` of the table at `base`.
    fn d_flags(&self, base: usize, i: u16) -> u16 {
        self.ring()
            .read(base + VRING_DESC_SIZE * usize::from(i) + VRING_DESC_FLAGS)
    }

    /// `vd[i].flags = v`.
    fn set_d_flags(&mut self, base: usize, i: u16, v: u16) {
        self.ring_mut().write(
            base + VRING_DESC_SIZE * usize::from(i) + VRING_DESC_FLAGS,
            v,
        );
    }

    /// `vd[i].next`.
    fn d_next(&self, base: usize, i: u16) -> u16 {
        self.ring()
            .read(base + VRING_DESC_SIZE * usize::from(i) + VRING_DESC_NEXT)
    }

    /// `vd[i].next = v`.
    fn set_d_next(&mut self, base: usize, i: u16, v: u16) {
        self.ring_mut()
            .write(base + VRING_DESC_SIZE * usize::from(i) + VRING_DESC_NEXT, v);
    }

    /// `vd[i].addr = addr; vd[i].len = len`.
    fn set_d_buf(&mut self, base: usize, i: u16, addr: u64, len: u32) {
        let d = base + VRING_DESC_SIZE * usize::from(i);
        let ring = self.ring_mut();
        ring.write(d + VRING_DESC_ADDR, addr);
        ring.write(d + VRING_DESC_LEN, len);
    }

    /// `vd[i].len = len`.
    fn set_d_len(&mut self, base: usize, i: u16, len: u32) {
        self.ring_mut().write(
            base + VRING_DESC_SIZE * usize::from(i) + VRING_DESC_LEN,
            len,
        );
    }

    /// `vq->vq_avail->idx`: the index the device last saw published.
    fn avail_idx_published(&self) -> u16 {
        self.ring().read(self.avail_off as usize + VRING_AVAIL_IDX)
    }

    /// `vq->vq_avail->flags`.
    fn avail_flags(&self) -> u16 {
        self.ring()
            .read(self.avail_off as usize + VRING_AVAIL_FLAGS)
    }

    /// `vq->vq_avail->flags = v`.
    fn set_avail_flags(&mut self, v: u16) {
        let off = self.avail_off as usize + VRING_AVAIL_FLAGS;
        self.ring_mut().write(off, v);
    }

    /// The offset of `VQ_USED_EVENT(vq)`: `avail->ring[vq_num]`.
    fn used_event_off(&self) -> usize {
        self.avail_off as usize + VRING_AVAIL_RING + 2 * usize::from(self.num)
    }

    /// `VQ_USED_EVENT(vq) = v`.
    fn set_used_event(&mut self, v: u16) {
        let off = self.used_event_off();
        self.ring_mut().write(off, v);
    }

    /// `vq->vq_used->idx`: the device's free-running index.
    fn used_idx_device(&self) -> u16 {
        self.ring().read(self.used_off as usize + VRING_USED_IDX)
    }

    /// The offset of `VQ_AVAIL_EVENT(vq)`: `used->ring[vq_num]`.
    fn avail_event_off(&self) -> usize {
        self.used_off as usize + VRING_USED_RING + VRING_USED_ELEM_SIZE * usize::from(self.num)
    }

    /// `VQ_AVAIL_EVENT(vq)`: the avail index the device asks to be kicked at.
    fn avail_event(&self) -> u16 {
        self.ring().read(self.avail_event_off())
    }

    // dmamap sync operations for a virtqueue.
    //
    // XXX These should be more fine grained. Syncing the whole ring if we only need a few
    // bytes is inefficient if we use bounce buffers.

    /// `vq_sync_descs`: the descriptor table (`availoffset == sizeof(vring_desc) * vq_num`).
    fn sync_descs(&self, ops: DmaSync) {
        self.ring().sync(0, self.avail_off as usize, ops);
    }

    /// `vq_sync_aring`: the avail ring.
    fn sync_aring(&self, ops: DmaSync) {
        self.ring().sync(
            self.avail_off as usize,
            VRING_AVAIL_RING + 2 * usize::from(self.num),
            ops,
        );
    }

    /// `vq_sync_aring_used_event`: the used event word after the avail ring.
    fn sync_aring_used_event(&self, ops: DmaSync) {
        self.ring().sync(self.used_event_off(), 2, ops);
    }

    /// `vq_sync_uring`: the used ring.
    fn sync_uring(&self, ops: DmaSync) {
        self.ring().sync(
            self.used_off as usize,
            VRING_USED_RING + VRING_USED_ELEM_SIZE * usize::from(self.num),
            ops,
        );
    }

    /// `vq_sync_uring_avail_event`: the avail event word after the used ring.
    fn sync_uring_avail_event(&self, ops: DmaSync) {
        self.ring().sync(self.avail_event_off(), 2, ops);
    }

    /// `vq_sync_indirect`: the indirect table of `slot`.
    fn sync_indirect(&self, slot: u16, ops: DmaSync) {
        self.ring().sync(
            self.table(slot, Chain::Indirect),
            VRING_DESC_SIZE * self.maxnsegs,
            ops,
        );
    }
}

impl Drop for Virtqueue {
    /// The entries freed; the ring given back, unless the device still knows it: then its
    /// pages are never freed (the device may write them yet), and the first such drop says
    /// so.
    fn drop(&mut self) {
        if self.live
            && let Some(ring) = self.ring.take()
        {
            core::mem::forget(ring);
            if !DROPPED_LIVE.swap(true, Ordering::Relaxed) {
                printf(format_args!(
                    "virtqueue {}: dropped while the device knows it; its ring is kept\n",
                    self.index
                ));
            }
        }
    }
}

/// The child's side of an interrupt for one of its children, as `ChildRef` calls it: the
/// C's `vq_done` and `sc_config_change`.
type VqIntrFn = fn(&AnyArc, usize) -> i32;

/// `sc_config_change`, monomorphised for the child's state.
type ConfigChangeFn = fn(&AnyArc) -> i32;

/// A virtio child's state as its transport's interrupt sees it: the queues' `vq_done`s and
/// `sc_config_change`, which the C writes into its parent's softc (`ChildRef` erases it).
pub trait VirtioChild: Send + Sync + 'static {
    /// `sc_config_change != NULL`: the child has a configuration change handler.
    const CONFIG_CHANGE: bool = false;

    /// `virtio_check_vq` of the child's queue `index` (`sc_vqs[index]`), then its `vq_done`
    /// when the device used an entry: the work done (1) or not (0).
    fn vq_intr(&self, index: usize) -> i32;

    /// `sc_config_change`'s call: the device's configuration changed.
    fn config_change(&self) -> i32 {
        0
    }
}

/// A child's state with its type erased, as its transport keeps it: what the transport's
/// interrupt calls ([`ChildRef::check_vq`], [`ChildRef::config_change`]), and the child's
/// node (`sc_child`).
#[derive(Clone)]
pub struct ChildRef {
    /// The child's state, a counted handle with its type erased.
    state: AnyArc,
    /// The child's node.
    dev: DevRef,
    /// `vq_done`, through `virtio_check_vq`.
    vq_intr: VqIntrFn,
    /// `sc_config_change`; `None` for the C's NULL.
    config_change: Option<ConfigChangeFn>,
}

impl ChildRef {
    /// The handle a child gives its transport: its state and its node.
    pub fn of<C: VirtioChild>(state: &MArc<C>, dev: &DevRef) -> Self {
        ChildRef {
            state: MArc::erase(state.clone()),
            dev: dev.clone(),
            vq_intr: child_vq_intr::<C>,
            config_change: C::CONFIG_CHANGE.then_some(child_config_change::<C>),
        }
    }

    /// `virtio_check_vq` of the child's queue `index`, and its `vq_done`.
    #[inline]
    pub fn check_vq(&self, index: usize) -> i32 {
        (self.vq_intr)(&self.state, index)
    }

    /// `sc_config_change(vsc)`; `None` when the child has none.
    pub fn config_change(&self) -> Option<i32> {
        self.config_change.map(|f| f(&self.state))
    }

    /// `sc_config_change != NULL`.
    pub fn has_config_change(&self) -> bool {
        self.config_change.is_some()
    }

    /// `sc_child`: the child's node.
    pub fn dev(&self) -> &DevRef {
        &self.dev
    }
}

/// What a child asks of its transport: `struct virtio_ops`, entry by entry, then the softc
/// members a child sets through its parent (`sc_child`, `sc_ipl`, `sc_dmat`). A trait
/// received as a value ([`VirtioArgs::transport`]); `bsd`'s implementation over the legacy
/// transports is a `bridges.toml` row (N7.3d).
pub trait VirtioTransport: Send + Sync + 'static {
    /// `kick`: the device notified of new buffers in queue `index`.
    fn kick(&self, index: u16);
    /// `read_dev_cfg_1`: byte `off` of the device's configuration.
    fn read_config_1(&self, off: usize) -> u8;
    /// `read_dev_cfg_2`.
    fn read_config_2(&self, off: usize) -> u16;
    /// `read_dev_cfg_4`.
    fn read_config_4(&self, off: usize) -> u32;
    /// `read_dev_cfg_8`.
    fn read_config_8(&self, off: usize) -> u64;
    /// `write_dev_cfg_1`.
    fn write_config_1(&self, off: usize, v: u8);
    /// `write_dev_cfg_2`.
    fn write_config_2(&self, off: usize, v: u16);
    /// `write_dev_cfg_4`.
    fn write_config_4(&self, off: usize, v: u32);
    /// `write_dev_cfg_8`.
    fn write_config_8(&self, off: usize, v: u64);
    /// `read_queue_size`: the size of queue `index`, 0 for none.
    fn read_queue_size(&self, index: u16) -> u16;
    /// `setup_queue`: the device told where queue `q.index`'s rings are (`q.addr` 0: no queue
    /// any more).
    fn setup_queue(&self, q: &QueueSetup);
    /// `setup_intrs`: each queue routed to its interrupt vector.
    fn setup_intrs(&self);
    /// `get_status`.
    fn get_status(&self) -> i32;
    /// `set_status`.
    fn set_status(&self, status: i32);
    /// `neg_features` with `sc_driver_features` = `driver`: what both sides have. `names`
    /// given, the transport may print what it disables (`VIRTIO_DEBUG`'s listing besides).
    fn negotiate_features(
        &self,
        driver: u64,
        names: Option<&[VirtioFeatureName]>,
    ) -> Result<Features, Errno>;
    /// `attach_finish`: the transport's part of [`virtio_attach_finish`] (the interrupts),
    /// with the child that handles them (`sc_vqs`, `sc_nvqs`, each `vq_done`,
    /// `sc_config_change`) and its queues.
    fn attach_finish(
        &self,
        va: &VirtioArgs,
        child: ChildRef,
        queues: &[QueueSetup],
    ) -> Result<(), Errno>;
    /// `poll_intr`: the interrupt handler, called by a child that polls.
    fn poll_intr(&self) -> i32;
    /// `intr_barrier`: a handler running elsewhere has returned.
    fn intr_barrier(&self);
    /// `sc_dmat`, its memory half: what the child's rings and buffers come from.
    fn dma(&self) -> &'static dyn DmaMemory;
    /// `sc_child = child`, `sc_ipl = ipl`; `false` when a child is there already (the C's
    /// "already attached to something else").
    fn claim(&self, child: &DevRef, ipl: i32) -> bool;
    /// `sc_child = VIRTIO_CHILD_ERROR`: the child's attach failed.
    fn child_failed(&self);
    /// `sc_dev.dv_xname`: the transport's name, for the core's messages.
    fn xname(&self) -> &str;
}

/// `vq_done` through `virtio_check_vq`, for a child whose state is a `C`; 0 for a state of
/// another type (a handle that does not name `C`'s cannot exist: `ChildRef::of` makes both).
fn child_vq_intr<C: VirtioChild>(state: &AnyArc, index: usize) -> i32 {
    state.downcast_ref::<C>().map_or(0, |c| c.vq_intr(index))
}

/// `sc_config_change`, for a child whose state is a `C`.
fn child_config_change<C: VirtioChild>(state: &AnyArc) -> i32 {
    state
        .downcast_ref::<C>()
        .map_or(0, VirtioChild::config_change)
}

/// `virtio_device_name[]`'s entries, `None` for the C's NULL.
static VIRTIO_DEVICE_NAME: [Option<&str>; 17] = [
    Some("Unknown (0)"),    // 0
    Some("Network"),        // 1
    Some("Block"),          // 2
    Some("Console"),        // 3
    Some("Entropy"),        // 4
    Some("Memory Balloon"), // 5
    Some("IO Memory"),      // 6
    Some("Rpmsg"),          // 7
    Some("SCSI host"),      // 8
    Some("9P Transport"),   // 9
    Some("mac80211 wlan"),  // 10
    None,                   // 11
    None,                   // 12
    None,                   // 13
    None,                   // 14
    None,                   // 15
    Some("GPU"),            // 16
];

/// `virtio_device_string(id)`: the name of virtio device `id`; a NULL entry as the kernel's
/// `printf` prints it, `(null)`.
pub fn virtio_device_string(id: i32) -> &'static str {
    match usize::try_from(id)
        .ok()
        .and_then(|i| VIRTIO_DEVICE_NAME.get(i))
    {
        Some(Some(name)) => name,
        Some(None) => "(null)",
        None => "Unknown",
    }
}

/// `virtio_reset`: the device reset (`virtio_device_reset`), which stops its activity; no
/// feature active.
pub fn virtio_reset(t: &dyn VirtioTransport, f: &mut Features) {
    t.set_status(VIRTIO_CONFIG_DEVICE_STATUS_RESET);
    f.active = 0;
}

/// `virtio_attach_finish`: the transport establishes the interrupts for `child`, every queue
/// is handed to the device, and the device is told the driver is ready. The transport's error
/// comes before any queue is handed over.
pub fn virtio_attach_finish(
    t: &dyn VirtioTransport,
    va: &VirtioArgs,
    child: ChildRef,
    queues: &[QueueSetup],
) -> Result<(), Errno> {
    t.attach_finish(va, child, queues)?;

    t.setup_intrs();
    for q in queues {
        if q.num == 0 {
            continue;
        }
        t.setup_queue(q);
    }
    t.set_status(VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK);
    Ok(())
}

/// `virtio_reinit_start`: the features negotiated again (`driver` the child's), the
/// interrupts routed and each queue initialised and handed to the device again; a queue whose
/// size changed is `panic(9)`. The new features, which the child keeps.
pub fn virtio_reinit_start(
    t: &dyn VirtioTransport,
    driver: u64,
    vqs: &mut [&mut Virtqueue],
) -> Features {
    t.set_status(VIRTIO_CONFIG_DEVICE_STATUS_ACK);
    t.set_status(VIRTIO_CONFIG_DEVICE_STATUS_DRIVER);
    let f = t.negotiate_features(driver, None).unwrap_or_default();
    t.setup_intrs();
    for vq in vqs {
        let n = t.read_queue_size(vq.index);
        if n != vq.num {
            kpanic(format_args!(
                "{}: virtqueue size changed, vq index {}",
                t.xname(),
                vq.index
            ));
        }
        vq.init(&f);
        let q = vq.setup();
        t.setup_queue(&q);
    }
    f
}

/// `virtio_reinit_end`: the device activated; enqueue allowed.
pub fn virtio_reinit_end(t: &dyn VirtioTransport) {
    t.set_status(VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK);
}

/// The event index rule of `virtio_enqueue_commit`: the device, which asked to be kicked when
/// the avail index passes `event`, needs a kick for the entries `[o, n)` just published:
/// `(uint16_t)(n - (event + 1)) < (uint16_t)(n - o)`, in 16-bit arithmetic across the wrap.
#[inline]
fn vring_need_event(event: u16, n: u16, o: u16) -> bool {
    n.wrapping_sub(event.wrapping_add(1)) < n.wrapping_sub(o)
}

/// Host tests and benches: the device's side of a queue's rings, as QEMU's model reads and
/// writes them.
#[cfg(any(test, feature = "doubles"))]
pub struct DeviceView<'a> {
    /// The queue.
    vq: &'a mut Virtqueue,
}

#[cfg(any(test, feature = "doubles"))]
impl Virtqueue {
    /// Host tests and benches: the device's side of the rings.
    pub fn device(&mut self) -> DeviceView<'_> {
        DeviceView { vq: self }
    }
}

#[cfg(any(test, feature = "doubles"))]
impl DeviceView<'_> {
    /// Descriptor `i` of the queue's own table: `(addr, len, flags, next)`.
    pub fn desc(&self, i: u16) -> (u64, u32, u16, u16) {
        self.desc_at(0, i)
    }

    /// Descriptor `j` of `slot`'s indirect table.
    pub fn indirect(&self, slot: u16, j: u16) -> (u64, u32, u16, u16) {
        self.desc_at(self.vq.table(slot, Chain::Indirect), j)
    }

    /// The descriptor `i` of the table at `base`.
    fn desc_at(&self, base: usize, i: u16) -> (u64, u32, u16, u16) {
        let d = base + VRING_DESC_SIZE * usize::from(i);
        let r = self.vq.ring();
        (
            r.read(d + VRING_DESC_ADDR),
            r.read(d + VRING_DESC_LEN),
            r.read(d + VRING_DESC_FLAGS),
            r.read(d + VRING_DESC_NEXT),
        )
    }

    /// `avail->idx` as published.
    pub fn avail_idx(&self) -> u16 {
        self.vq.avail_idx_published()
    }

    /// `avail->ring[i]`.
    pub fn avail_ring(&self, i: u16) -> u16 {
        let off = self.vq.avail_off as usize + VRING_AVAIL_RING + 2 * usize::from(i);
        self.vq.ring().read(off)
    }

    /// `avail->flags`.
    pub fn avail_flags(&self) -> u16 {
        self.vq.avail_flags()
    }

    /// `VQ_USED_EVENT`: the used index the driver asks to be interrupted at.
    pub fn used_event(&self) -> u16 {
        self.vq.ring().read(self.vq.used_event_off())
    }

    /// `used->idx`.
    pub fn used_idx(&self) -> u16 {
        self.vq.used_idx_device()
    }

    /// The device used the request at `id` and wrote `len` bytes: the used ring's next
    /// element, then its index.
    pub fn use_slot(&mut self, id: u32, len: u32) {
        let idx = self.vq.used_idx_device();
        let elem = self.vq.used_off as usize
            + VRING_USED_RING
            + VRING_USED_ELEM_SIZE * usize::from(idx & self.vq.mask);
        let off = self.vq.used_off as usize + VRING_USED_IDX;
        let r = self.vq.ring_mut();
        r.write(elem + VRING_USED_ELEM_ID, id);
        r.write(elem + VRING_USED_ELEM_LEN, len);
        r.write(off, idx.wrapping_add(1));
    }

    /// `used->flags = v` (`VRING_USED_F_NO_NOTIFY`).
    pub fn set_used_flags(&mut self, v: u16) {
        let off = self.vq.used_off as usize + VRING_USED_FLAGS;
        self.vq.ring_mut().write(off, v);
    }

    /// `VQ_AVAIL_EVENT = v`: the avail index the device asks to be kicked at.
    pub fn set_avail_event(&mut self, v: u16) {
        let off = self.vq.avail_event_off();
        self.vq.ring_mut().write(off, v);
    }

    /// The ring's bus address and length, and the avail, used and indirect areas' offsets.
    pub fn layout(&self) -> (u64, usize, u32, u32, Option<u32>) {
        let r = self.vq.ring();
        (
            r.seg().addr,
            r.len(),
            self.vq.avail_off,
            self.vq.used_off,
            self.vq.indirect_off,
        )
    }
}

/// Host tests and benches: DMA memory of leaked host pages (never reused, so no test sees
/// another's), at bus addresses of its own below the rings' limit, counting what it hands out
/// and takes back; the next `fail` allocations answer `ENOMEM`.
#[cfg(any(test, feature = "doubles"))]
pub struct FakeDma {
    /// The next bus address handed out.
    next: core::sync::atomic::AtomicU64,
    /// Blocks handed out.
    pub allocs: core::sync::atomic::AtomicUsize,
    /// Blocks taken back.
    pub frees: core::sync::atomic::AtomicUsize,
    /// Allocations still to refuse.
    pub fail: core::sync::atomic::AtomicUsize,
}

#[cfg(any(test, feature = "doubles"))]
impl FakeDma {
    /// A provider whose first block is at bus address 1 MiB.
    pub const fn new() -> Self {
        Self {
            next: core::sync::atomic::AtomicU64::new(0x10_0000),
            allocs: core::sync::atomic::AtomicUsize::new(0),
            frees: core::sync::atomic::AtomicUsize::new(0),
            fail: core::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[cfg(any(test, feature = "doubles"))]
impl Default for FakeDma {
    fn default() -> Self {
        Self::new()
    }
}

// The provider's contract: each block is fresh, leaked, page-aligned host memory of whole
// pages that only this block names (never handed out again), holding no Rust object but the
// bytes.
#[cfg(any(test, feature = "doubles"))]
impl DmaMemory for FakeDma {
    fn alloc(&self, req: &DmaReq) -> Result<hal::dev::dma::DmaBlock, Errno> {
        use sysdefs::base::param::PAGE_SIZE;

        /// A page of host memory.
        #[derive(Clone, Copy)]
        #[repr(C, align(4096))]
        struct Page([u8; 4096]);

        if self
            .fail
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1))
            .is_ok()
        {
            return Err(Errno::ENOMEM);
        }
        let pages = req.size.div_ceil(PAGE_SIZE);
        let align = req.align.max(PAGE_SIZE) as u64;
        let mem: &'static mut [Page] =
            std::vec::Vec::from_iter(core::iter::repeat_n(Page([0; 4096]), pages)).leak();
        let va = core::ptr::NonNull::from(mem).cast::<u8>();
        let len = pages * PAGE_SIZE;
        let step = (len as u64).next_multiple_of(align) + align;
        let addr = self
            .next
            .fetch_add(step, Ordering::Relaxed)
            .next_multiple_of(align);
        self.allocs.fetch_add(1, Ordering::Relaxed);
        Ok(hal::dev::dma::DmaBlock {
            va,
            len,
            seg: DmaSeg { addr, len },
            coherent: false,
            cookie: 0,
        })
    }

    fn free(&self, _block: hal::dev::dma::DmaBlock) {
        self.frees.fetch_add(1, Ordering::Relaxed);
    }
}

/// Host tests and benches: one call a child made of a [`FakeTransport`].
#[cfg(any(test, feature = "doubles"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakeEvent {
    /// `attach_finish` of so many queues.
    AttachFinish(usize),
    /// `setup_intrs`.
    SetupIntrs,
    /// `setup_queue`.
    SetupQueue(QueueSetup),
    /// `set_status`.
    SetStatus(i32),
    /// `negotiate_features` with these driver features.
    Negotiate(u64),
}

/// Host tests and benches: a transport over nothing, which records what its child asks
/// (kicks counted, the attach's calls and the `setup_queue`s in order, a configuration space
/// of 64 bytes) and whose interrupt calls the child as the C's transports do
/// (`virtio_check_vqs`: the queues backwards).
#[cfg(any(test, feature = "doubles"))]
pub struct FakeTransport {
    /// The DMA memory.
    pub dma: &'static FakeDma,
    /// The size of queues 0 to 3 (`read_queue_size`), 0 for none.
    pub qsize: [core::sync::atomic::AtomicU16; 4],
    /// The device's features; the negotiation answers those the driver asked for, with
    /// indirect descriptors and `ANY_LAYOUT` as the C's transports add them.
    pub host_features: core::sync::atomic::AtomicU64,
    /// `kick`s.
    pub kicks: core::sync::atomic::AtomicU32,
    /// What the child asked, in order.
    pub events: std::sync::Mutex<std::vec::Vec<FakeEvent>>,
    /// The device's configuration space.
    pub config: std::sync::Mutex<[u8; 64]>,
    /// `attach_finish` answers `EIO`.
    pub fail_finish: AtomicBool,
    /// The negotiation answers `ENXIO`.
    pub fail_negotiate: AtomicBool,
    /// The child and its queues' indexes, from `attach_finish`.
    pub child: std::sync::Mutex<Option<(ChildRef, std::vec::Vec<u16>)>>,
    /// `claim`'s child and level.
    pub claimed: std::sync::Mutex<Option<(DevRef, i32)>>,
    /// `child_failed` was called.
    pub failed: AtomicBool,
}

#[cfg(any(test, feature = "doubles"))]
std::thread_local! {
    /// The transport `fake_transport` makes for this thread's children.
    static FAKE_CURRENT: core::cell::Cell<Option<&'static FakeTransport>> =
        const { core::cell::Cell::new(None) };
}

#[cfg(any(test, feature = "doubles"))]
impl FakeTransport {
    /// A leaked transport whose queue 0 has `qsize` entries, the one [`fake_transport`] makes
    /// for this thread from now on.
    pub fn make(qsize: u16, host_features: u64) -> &'static FakeTransport {
        use core::sync::atomic::{AtomicU16, AtomicU32, AtomicU64};
        let t: &'static FakeTransport = Box::leak(Box::new(FakeTransport {
            dma: Box::leak(Box::new(FakeDma::new())),
            qsize: [
                AtomicU16::new(qsize),
                AtomicU16::new(0),
                AtomicU16::new(0),
                AtomicU16::new(0),
            ],
            host_features: AtomicU64::new(host_features),
            kicks: AtomicU32::new(0),
            events: std::sync::Mutex::new(std::vec::Vec::new()),
            config: std::sync::Mutex::new([0; 64]),
            fail_finish: AtomicBool::new(false),
            fail_negotiate: AtomicBool::new(false),
            child: std::sync::Mutex::new(None),
            claimed: std::sync::Mutex::new(None),
            failed: AtomicBool::new(false),
        }));
        FAKE_CURRENT.with(|c| c.set(Some(t)));
        t
    }

    /// The interrupt: `virtio_check_vqs` over the child's queues, backwards.
    pub fn interrupt(&self) -> i32 {
        let child = self
            .child
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let Some((c, queues)) = child else {
            return 0;
        };
        let mut r = 0;
        for i in (0..queues.len()).rev() {
            r |= c.check_vq(i);
        }
        r
    }

    /// The calls recorded so far.
    pub fn events(&self) -> std::vec::Vec<FakeEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Records one call.
    fn log(&self, e: FakeEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(e);
    }

    /// `n` bytes of the configuration space from `off`, little-endian.
    fn cfg_read(&self, off: usize, n: usize) -> u64 {
        let c = self
            .config
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (0..n).fold(0, |v, i| v | (u64::from(c[off + i]) << (8 * i)))
    }

    /// `n` bytes of `v` into the configuration space from `off`, little-endian.
    fn cfg_write(&self, off: usize, n: usize, v: u64) {
        let mut c = self
            .config
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for i in 0..n {
            c[off + i] = (v >> (8 * i)) as u8;
        }
    }
}

/// Host tests and benches: [`VirtioArgs::transport`] for this thread's [`FakeTransport`]
/// (the last [`FakeTransport::make`]); `panic` when there is none.
#[cfg(any(test, feature = "doubles"))]
pub fn fake_transport(_parent: &DevRef) -> Box<dyn VirtioTransport> {
    match FAKE_CURRENT.with(core::cell::Cell::get) {
        Some(t) => Box::new(t),
        None => kpanic(format_args!("fake_transport: no FakeTransport made")),
    }
}

#[cfg(any(test, feature = "doubles"))]
impl VirtioTransport for &'static FakeTransport {
    fn kick(&self, _index: u16) {
        self.kicks.fetch_add(1, Ordering::Relaxed);
    }
    fn read_config_1(&self, off: usize) -> u8 {
        self.cfg_read(off, 1) as u8
    }
    fn read_config_2(&self, off: usize) -> u16 {
        self.cfg_read(off, 2) as u16
    }
    fn read_config_4(&self, off: usize) -> u32 {
        self.cfg_read(off, 4) as u32
    }
    fn read_config_8(&self, off: usize) -> u64 {
        self.cfg_read(off, 8)
    }
    fn write_config_1(&self, off: usize, v: u8) {
        self.cfg_write(off, 1, v.into());
    }
    fn write_config_2(&self, off: usize, v: u16) {
        self.cfg_write(off, 2, v.into());
    }
    fn write_config_4(&self, off: usize, v: u32) {
        self.cfg_write(off, 4, v.into());
    }
    fn write_config_8(&self, off: usize, v: u64) {
        self.cfg_write(off, 8, v);
    }
    fn read_queue_size(&self, index: u16) -> u16 {
        self.qsize
            .get(usize::from(index))
            .map_or(0, |q| q.load(Ordering::Relaxed))
    }
    fn setup_queue(&self, q: &QueueSetup) {
        self.log(FakeEvent::SetupQueue(*q));
    }
    fn setup_intrs(&self) {
        self.log(FakeEvent::SetupIntrs);
    }
    fn get_status(&self) -> i32 {
        0
    }
    fn set_status(&self, status: i32) {
        self.log(FakeEvent::SetStatus(status));
    }
    fn negotiate_features(
        &self,
        driver: u64,
        _names: Option<&[VirtioFeatureName]>,
    ) -> Result<Features, Errno> {
        use crate::dev::pv::virtioreg::VIRTIO_F_ANY_LAYOUT;
        self.log(FakeEvent::Negotiate(driver));
        if self.fail_negotiate.load(Ordering::Relaxed) {
            return Err(Errno::ENXIO);
        }
        let want = driver | VIRTIO_F_RING_INDIRECT_DESC | VIRTIO_F_ANY_LAYOUT;
        Ok(Features::of_active(
            want & self.host_features.load(Ordering::Relaxed),
        ))
    }
    fn attach_finish(
        &self,
        _va: &VirtioArgs,
        child: ChildRef,
        queues: &[QueueSetup],
    ) -> Result<(), Errno> {
        self.log(FakeEvent::AttachFinish(queues.len()));
        if self.fail_finish.load(Ordering::Relaxed) {
            return Err(Errno::EIO);
        }
        let idx = queues.iter().map(|q| q.index).collect();
        *self
            .child
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((child, idx));
        Ok(())
    }
    fn poll_intr(&self) -> i32 {
        self.interrupt()
    }
    fn intr_barrier(&self) {}
    fn dma(&self) -> &'static dyn DmaMemory {
        self.dma
    }
    fn claim(&self, child: &DevRef, ipl: i32) -> bool {
        let mut c = self
            .claimed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if c.is_some() {
            return false;
        }
        *c = Some((child.clone(), ipl));
        true
    }
    fn child_failed(&self) {
        self.failed.store(true, Ordering::Relaxed);
    }
    fn xname(&self) -> &str {
        "virtio0"
    }
}

/// Host tests and benches: a node of `row` named `name` (unit 0, no parent) with an empty
/// softc, outside the device tree, for a child's attach called by hand.
#[cfg(any(test, feature = "doubles"))]
pub fn test_node(row: &'static dyn crate::dev::device::ConfigRow, name: &str) -> DevRef {
    let mut xname = [0u8; 16];
    xname[..name.len()].copy_from_slice(name.as_bytes());
    let id = crate::dev::device::Ident {
        class: crate::dev::device::DV_DULL,
        unit: 0,
        xname: &xname,
        row,
        parent: None,
    };
    let softc = MArc::new(M_DEVBUF, M_NOWAIT, ()).map(MArc::erase);
    match softc {
        Some(sc) => crate::dev::device::Device::new(&id, 0, sc),
        None => kpanic(format_args!("test_node: no memory")),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the virtio core: queues over a FakeTransport and leaked host pages
    // (FakeDma), the device's side written through DeviceView.

    use core::sync::atomic::AtomicU32;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;
    use crate::dev::pv::virtioreg::VIRTIO_F_ANY_LAYOUT;

    /// Indirect descriptors and event indexes as the device offers them.
    const ALL: u64 = VIRTIO_F_RING_INDIRECT_DESC | VIRTIO_F_RING_EVENT_IDX;

    /// A transport with queue 0 of `n` slots, and that queue, allocated with `features`
    /// (`maxnsegs` indirect descriptors a slot when they are on).
    fn queue(n: u16, active: u64, maxnsegs: usize) -> (&'static FakeTransport, Virtqueue) {
        crate::testutil::install();
        let t = FakeTransport::make(n, ALL);
        let f = Features::of_active(active);
        let vq = Virtqueue::alloc(&t, &f, 0, maxnsegs, "test").expect("a queue");
        (t, vq)
    }

    /// The free list from its head.
    fn free_list(vq: &Virtqueue) -> Vec<u16> {
        let mut v = Vec::new();
        let mut s = vq.free;
        while let Some(i) = s {
            v.push(i);
            s = vq.entries[usize::from(i)].next_free;
        }
        v
    }

    /// The page-rounded size of the C's `virtio_alloc_vq` for `n` slots.
    fn c_sizes(
        n: usize,
        event_idx: bool,
        maxnsegs: usize,
        indirect: bool,
    ) -> (usize, usize, usize) {
        let hdrlen = if event_idx { 3 } else { 2 };
        let a1 = (16 * n + 2 * (hdrlen + n)).next_multiple_of(4096);
        let a2 = (2 * hdrlen + 8 * n).next_multiple_of(4096);
        let a3 = if indirect && maxnsegs > 1 {
            16 * maxnsegs * n
        } else {
            0
        };
        (a1, a2, a3)
    }

    #[test]
    fn the_rings_are_laid_out_as_the_c_lays_them_out() {
        for n in [8u16, 256, 1024] {
            for (active, maxnsegs) in [
                (0, 1),
                (VIRTIO_F_RING_EVENT_IDX, 1),
                (VIRTIO_F_RING_INDIRECT_DESC, 4),
                (ALL, 4),
                (VIRTIO_F_RING_INDIRECT_DESC, 1),
            ] {
                let (_t, mut vq) = queue(n, active, maxnsegs);
                let event = active & VIRTIO_F_RING_EVENT_IDX != 0;
                let ind = active & VIRTIO_F_RING_INDIRECT_DESC != 0;
                let (a1, a2, a3) = c_sizes(usize::from(n), event, maxnsegs, ind);
                let (addr, len, avail, used, indirect) = vq.device().layout();
                assert_eq!(len, a1 + a2 + a3, "{n} {active:#x} {maxnsegs}");
                assert_eq!(avail as usize, 16 * usize::from(n));
                assert_eq!(used as usize, a1);
                assert_eq!(indirect.map(|o| o as usize), (a3 > 0).then_some(a1 + a2));
                assert_eq!(addr % 4096, 0, "VIRTIO_PAGE_SIZE aligned");
                assert!(addr + len as u64 - 1 <= RING_HIGH);
                assert_eq!(vq.num(), n);
                assert_eq!(vq.event_idx, event);
                let q = vq.setup();
                assert_eq!(
                    q,
                    QueueSetup {
                        index: 0,
                        num: n,
                        addr,
                        avail_off: avail,
                        used_off: used
                    }
                );
            }
        }
    }

    #[test]
    fn init_puts_every_slot_on_the_free_list_rising_and_chains_the_indirect_tables() {
        let (_t, mut vq) = queue(8, VIRTIO_F_RING_INDIRECT_DESC, 4);
        assert_eq!(free_list(&vq), (0..8).collect::<Vec<_>>());
        assert_eq!(vq.nfree(), 8);
        assert_eq!(vq.queued(), 1);
        let d = vq.device();
        for slot in 0..8 {
            for j in 0..3u16 {
                assert_eq!(d.indirect(slot, j).3, j + 1);
            }
            assert_eq!(d.indirect(slot, 3).3, 0);
        }
    }

    #[test]
    fn a_direct_chain_is_built_filled_committed_and_dequeued() {
        let (t, mut vq) = queue(8, 0, 1);
        let slot = vq.enqueue_prep().unwrap();
        assert_eq!(slot, 0);
        vq.enqueue_reserve(slot, 3).unwrap();
        assert_eq!(vq.nfree(), 5);
        {
            let d = vq.device();
            assert_eq!((d.desc(0).2, d.desc(0).3), (VRING_DESC_F_NEXT, 1));
            assert_eq!((d.desc(1).2, d.desc(1).3), (VRING_DESC_F_NEXT, 2));
            assert_eq!(d.desc(2).2, 0);
        }

        let hdr = DmaSeg {
            addr: 0x2000,
            len: 64,
        };
        vq.enqueue_p(slot, hdr, 16, 12, true);
        vq.enqueue(
            slot,
            &[DmaSeg {
                addr: 0x3000,
                len: 1500,
            }],
            true,
        );
        vq.enqueue(
            slot,
            &[DmaSeg {
                addr: 0x4000,
                len: 2048,
            }],
            false,
        );
        {
            let d = vq.device();
            assert_eq!((d.desc(0).0, d.desc(0).1), (0x2010, 12));
            assert_eq!(
                (d.desc(1).0, d.desc(1).1, d.desc(1).2),
                (0x3000, 1500, VRING_DESC_F_NEXT)
            );
            assert_eq!((d.desc(2).0, d.desc(2).2), (0x4000, VRING_DESC_F_WRITE));
        }

        vq.enqueue_commit(&t, Some(slot), true);
        assert_eq!(vq.device().avail_ring(0), 0);
        assert_eq!(vq.device().avail_idx(), 1);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 1);

        assert_eq!(vq.dequeue(), Err(Errno::ENOENT), "nothing used yet");
        assert!(!vq.check());
        assert_eq!(vq.queued(), 0, "check clears vq_queued");
        vq.device().use_slot(0, 100);
        assert!(vq.check());
        assert_eq!(vq.dequeue(), Ok((0, 100)));
        assert!(!vq.check(), "the index has been read");
        assert_eq!(vq.dequeue_commit(0), 3);
        assert_eq!(vq.nfree(), 8);
        assert_eq!(vq.dequeue(), Err(Errno::ENOENT));
    }

    #[test]
    fn an_indirect_request_takes_one_ring_slot() {
        let (t, mut vq) = queue(8, VIRTIO_F_RING_INDIRECT_DESC, 4);
        let s0 = vq.enqueue_prep().unwrap();
        let slot = vq.enqueue_prep().unwrap();
        assert_eq!(slot, 1);
        vq.enqueue_reserve(slot, 3).unwrap();
        assert_eq!(vq.nfree(), 6);
        let (addr, _, _, _, ind) = vq.device().layout();
        let d = vq.device().desc(1);
        assert_eq!(d.2, VRING_DESC_F_INDIRECT);
        assert_eq!(d.1, 48, "sizeof(struct vring_desc) * nsegs");
        assert_eq!(
            d.0,
            addr + u64::from(ind.unwrap()) + 16 * 4,
            "slot 1's table"
        );
        {
            let v = vq.device();
            assert_eq!(v.indirect(1, 0).2, VRING_DESC_F_NEXT);
            assert_eq!(v.indirect(1, 1).2, VRING_DESC_F_NEXT);
            assert_eq!(v.indirect(1, 2).2, 0);
        }
        vq.enqueue(
            slot,
            &[
                DmaSeg {
                    addr: 0x5000,
                    len: 16,
                },
                DmaSeg {
                    addr: 0x6000,
                    len: 512,
                },
            ],
            true,
        );
        vq.enqueue(
            slot,
            &[DmaSeg {
                addr: 0x7000,
                len: 1,
            }],
            false,
        );
        {
            let v = vq.device();
            assert_eq!((v.indirect(1, 0).0, v.indirect(1, 1).0), (0x5000, 0x6000));
            assert_eq!(v.indirect(1, 2).2, VRING_DESC_F_WRITE);
        }

        vq.enqueue_trim(slot, 2);
        assert_eq!(vq.device().desc(1).1, 32);
        assert_eq!(vq.device().indirect(1, 1).2, 0);

        vq.enqueue_commit(&t, Some(slot), false);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 0, "no notify asked");
        assert_eq!(vq.device().avail_idx(), 0, "not published without notify");
        vq.notify(&t);
        assert_eq!(vq.device().avail_idx(), 1);
        vq.device().use_slot(1, 0);
        assert!(vq.check());
        assert_eq!(vq.dequeue(), Ok((1, 0)));
        assert_eq!(vq.dequeue_commit(1), 1, "one ring slot");
        vq.enqueue_abort(s0);
        assert_eq!(vq.nfree(), 8);
    }

    #[test]
    fn reserve_without_enough_slots_aborts_and_abort_frees() {
        let (_t, mut vq) = queue(8, 0, 1);
        for _ in 0..6 {
            let s = vq.enqueue_prep().unwrap();
            vq.enqueue_reserve(s, 1).unwrap();
        }
        let s = vq.enqueue_prep().unwrap();
        assert_eq!(vq.nfree(), 1);
        assert_eq!(vq.enqueue_reserve(s, 3), Err(Errno::EAGAIN));
        assert_eq!(
            vq.nfree(),
            2,
            "the head and the one extra slot it took are back"
        );

        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_abort(s);
        assert_eq!(vq.nfree(), 2, "an unreserved slot");
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 2).unwrap();
        assert_eq!(vq.nfree(), 0);
        vq.enqueue_abort(s);
        assert_eq!(vq.nfree(), 2, "a reserved chain");
        assert_eq!(vq.enqueue_prep().map(|_| ()), Ok(()));
        assert_eq!(vq.enqueue_prep().map(|_| ()), Ok(()));
        assert_eq!(vq.enqueue_prep(), Err(Errno::EAGAIN));
    }

    #[test]
    fn trim_cuts_a_static_direct_chain() {
        let (_t, mut vq) = queue(8, 0, 1);
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 4).unwrap();
        vq.enqueue_trim(s, 2);
        let d = vq.device();
        assert_eq!(d.desc(0).2, VRING_DESC_F_NEXT);
        assert_eq!(d.desc(1).2, 0);
        assert_eq!(vq.entry(s).next, Some(s));
    }

    #[test]
    fn enqueue_past_maxnsegs_panics() {
        let (_t, mut vq) = queue(8, 0, 1);
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 2).unwrap();
        let two = [
            DmaSeg {
                addr: 0x1000,
                len: 1,
            },
            DmaSeg {
                addr: 0x2000,
                len: 1,
            },
        ];
        assert!(catch_unwind(AssertUnwindSafe(|| vq.enqueue(s, &two, true))).is_err());
    }

    #[test]
    fn a_slot_the_queue_does_not_have_panics() {
        let (_t, mut vq) = queue(8, 0, 1);
        assert!(catch_unwind(AssertUnwindSafe(|| vq.dequeue_commit(8))).is_err());
        let (_t, mut vq) = queue(8, 0, 1);
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 1).unwrap();
        vq.device().use_slot(9, 0);
        vq.check();
        assert!(catch_unwind(AssertUnwindSafe(|| vq.dequeue())).is_err());
        let (_t, mut vq) = queue(8, 0, 1);
        let s = vq.enqueue_prep().unwrap();
        assert!(
            catch_unwind(AssertUnwindSafe(|| vq.enqueue(
                s,
                &[DmaSeg::default()],
                true
            )))
            .is_err(),
            "not reserved"
        );
    }

    #[test]
    fn the_kick_rule_without_event_indexes_is_the_used_flag() {
        let (t, mut vq) = queue(8, 0, 1);
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 1).unwrap();
        vq.enqueue_commit(&t, Some(s), true);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 1);
        vq.device().set_used_flags(VRING_USED_F_NO_NOTIFY);
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 1).unwrap();
        vq.enqueue_commit(&t, Some(s), true);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 1, "NO_NOTIFY");
        assert_eq!(vq.device().avail_idx(), 2, "published all the same");
    }

    #[test]
    fn the_kick_rule_with_event_indexes_is_the_event_window() {
        // (event, new, old, kick): the device asked to be kicked once the index passes
        // `event`; [old, new) was just published. Across the 16-bit wrap.
        let table: &[(u16, u16, u16, bool)] = &[
            (0, 1, 0, true),
            (0, 2, 1, false),
            (1, 2, 1, true),
            (4, 8, 2, true),
            (9, 8, 2, false),
            (0xffff, 0, 0xffff, true),
            (0xfffe, 1, 0xffff, true),
            (0, 1, 0xffff, true),
            (1, 1, 0xffff, false),
            (0x7fff, 0x8000, 0x7fff, true),
        ];
        for &(event, n, o, kick) in table {
            assert_eq!(vring_need_event(event, n, o), kick, "{event} {n} {o}");
        }

        let (t, mut vq) = queue(8, VIRTIO_F_RING_EVENT_IDX, 1);
        // avail_event 0: kicked for entry 0.
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 1).unwrap();
        vq.enqueue_commit(&t, Some(s), true);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 1);
        // Still 0: entry 1 needs no kick.
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 1).unwrap();
        vq.enqueue_commit(&t, Some(s), true);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 1);
        // The device moves its event to 2: entry 2 is kicked.
        vq.device().set_avail_event(2);
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 1).unwrap();
        vq.enqueue_commit(&t, Some(s), true);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn interrupts_stop_and_start_in_both_modes() {
        let (_t, mut vq) = queue(8, 0, 1);
        let q0 = vq.queued();
        vq.stop_intr();
        assert_eq!(vq.device().avail_flags(), VRING_AVAIL_F_NO_INTERRUPT);
        assert_eq!(vq.queued(), q0 + 1);
        assert!(!vq.start_intr());
        assert_eq!(vq.device().avail_flags(), 0);
        vq.device().use_slot(0, 0);
        assert!(vq.start_intr(), "used meanwhile");

        let (_t, mut vq) = queue(8, VIRTIO_F_RING_EVENT_IDX, 1);
        vq.stop_intr();
        assert_eq!(vq.device().used_event(), 0x8000);
        assert_eq!(vq.device().avail_flags(), 0);
        assert!(!vq.start_intr());
        assert_eq!(vq.device().used_event(), 0);
    }

    #[test]
    fn postpone_moves_the_used_event_and_nused_counts() {
        let (_t, mut vq) = queue(8, VIRTIO_F_RING_EVENT_IDX, 1);
        assert!(!vq.postpone_intr(3));
        assert_eq!(vq.device().used_event(), 3);
        vq.device().use_slot(0, 0);
        vq.device().use_slot(1, 0);
        assert_eq!(vq.nused(), 2);
        assert!(vq.postpone_intr(1), "the ring went past the new event");

        let (t, mut vq) = queue(8, VIRTIO_F_RING_EVENT_IDX, 1);
        for _ in 0..4 {
            let s = vq.enqueue_prep().unwrap();
            vq.enqueue_reserve(s, 1).unwrap();
            vq.enqueue_commit(&t, Some(s), true);
        }
        assert!(!vq.postpone_intr_smart());
        assert_eq!(vq.device().used_event(), 3, "3/4 of 4 pending");
        assert!(!vq.postpone_intr_far());
        assert_eq!(vq.device().used_event(), 4);
    }

    #[test]
    fn free_refuses_a_busy_queue_and_tells_the_device_otherwise() {
        let (t, mut vq) = queue(8, 0, 1);
        let frees = t.dma.frees.load(Ordering::Relaxed);
        let s = vq.enqueue_prep().unwrap();
        let (mut vq, e) = vq.free(&t).err().expect("busy");
        assert_eq!(e, Errno::EBUSY);
        assert!(crate::testutil::printf_count("virtio0: freeing non-empty vq, index 0\n") >= 1);
        vq.enqueue_abort(s);
        let q = vq.setup();
        vq.free(&t).ok().expect("freed");
        assert_eq!(
            t.events().last(),
            Some(&FakeEvent::SetupQueue(QueueSetup { addr: 0, ..q }))
        );
        assert_eq!(
            t.dma.frees.load(Ordering::Relaxed),
            frees + 1,
            "the ring went back"
        );
    }

    #[test]
    fn a_queue_the_device_knows_keeps_its_ring_when_dropped() {
        let (t, mut vq) = queue(8, 0, 1);
        let frees = t.dma.frees.load(Ordering::Relaxed);
        let _ = vq.setup();
        drop(vq);
        assert_eq!(t.dma.frees.load(Ordering::Relaxed), frees, "kept");
        let (t, vq) = queue(8, 0, 1);
        let frees = t.dma.frees.load(Ordering::Relaxed);
        drop(vq);
        assert_eq!(
            t.dma.frees.load(Ordering::Relaxed),
            frees + 1,
            "never set up: freed"
        );
    }

    #[test]
    fn seventy_thousand_requests_wrap_the_indexes_of_a_queue_of_eight() {
        let (t, mut vq) = queue(8, VIRTIO_F_RING_EVENT_IDX, 1);
        for i in 0..70_000u32 {
            let s = vq.enqueue_prep().unwrap();
            vq.enqueue_reserve(s, 1).unwrap();
            vq.enqueue(
                s,
                &[DmaSeg {
                    addr: 0x9000,
                    len: 16,
                }],
                false,
            );
            vq.enqueue_commit(&t, Some(s), true);
            // The device keeps asking for the next one.
            let next = vq.device().avail_idx();
            vq.device().set_avail_event(next);
            vq.device().use_slot(u32::from(s), i & 0xff);
            assert!(vq.check());
            assert_eq!(vq.dequeue(), Ok((s, i & 0xff)));
            assert_eq!(vq.dequeue_commit(s), 1);
        }
        assert_eq!(vq.nfree(), 8);
        assert_eq!(vq.device().avail_idx(), (70_000u32 % 65_536) as u16);
        assert_eq!(vq.device().used_idx(), (70_000u32 % 65_536) as u16);
        assert_eq!(t.kicks.load(Ordering::Relaxed), 70_000);
    }

    #[test]
    fn alloc_says_why_it_fails() {
        crate::testutil::install();
        let t = FakeTransport::make(0, 0);
        let f = Features::default();
        assert_eq!(
            Virtqueue::alloc(&t, &f, 0, 1, "nosuchq").err(),
            Some(Errno::ENXIO)
        );
        assert!(crate::testutil::printf_count("virtqueue not exist, index 0 for nosuchq\n") >= 1);
        t.qsize[1].store(8, Ordering::Relaxed);
        t.dma.fail.store(1, Ordering::Relaxed);
        assert_eq!(
            Virtqueue::alloc(&t, &f, 1, 1, "nomemq").err(),
            Some(Errno::ENOMEM)
        );
        assert!(
            crate::testutil::printf_count("virtqueue 1 for nomemq allocation failed, error 12\n")
                >= 1
        );
        t.qsize[2].store(6, Ordering::Relaxed);
        assert!(catch_unwind(AssertUnwindSafe(|| Virtqueue::alloc(&t, &f, 2, 1, "odd"))).is_err());
    }

    /// A child of one queue, counting its interrupts.
    struct TestChild {
        vq: hal::sync::mutex::Mutexed<Virtqueue>,
        done: AtomicU32,
    }

    impl VirtioChild for TestChild {
        const CONFIG_CHANGE: bool = true;
        fn vq_intr(&self, index: usize) -> i32 {
            assert_eq!(index, 0);
            let mut vq = self.vq.lock();
            if !vq.check() {
                return 0;
            }
            while let Ok((slot, _)) = vq.dequeue() {
                vq.dequeue_commit(slot);
                self.done.fetch_add(1, Ordering::Relaxed);
            }
            1
        }
        fn config_change(&self) -> i32 {
            7
        }
    }

    /// A child without a configuration handler.
    struct Plain;

    impl VirtioChild for Plain {
        fn vq_intr(&self, _index: usize) -> i32 {
            5
        }
    }

    /// A row of constants for the tests' nodes.
    struct TestRow;

    impl crate::dev::device::ConfigRow for TestRow {
        fn driver(&'static self) -> &'static dyn crate::dev::device::DriverOps {
            panic!("never attached")
        }
        fn parents(&self) -> &'static [i16] {
            &[]
        }
        fn starunit1(&self) -> i16 {
            0
        }
        fn unit(&self) -> i16 {
            0
        }
        fn set_unit(&self, _unit: i16) {}
        fn fstate(&self) -> i16 {
            0
        }
        fn set_fstate(&self, _fstate: i16) {}
    }

    static TEST_ROW: TestRow = TestRow;

    /// A node for the tests' children.
    fn node() -> DevRef {
        test_node(&TEST_ROW, "vtest0")
    }

    #[test]
    fn a_child_ref_dispatches_to_its_state() {
        let (t, vq) = queue(8, 0, 1);
        let dev = node();
        let st = MArc::new(
            M_DEVBUF,
            M_NOWAIT,
            TestChild {
                vq: hal::sync::mutex::Mutexed::new(hal::cpu::intr::IPL_NET, vq),
                done: AtomicU32::new(0),
            },
        )
        .unwrap();
        let c = ChildRef::of(&st, &dev);
        assert!(MArc::ptr_eq(c.dev(), &dev));
        assert!(c.has_config_change());
        assert_eq!(c.config_change(), Some(7));
        assert_eq!(c.check_vq(0), 0, "an empty queue");
        {
            let mut vq = st.vq.lock();
            let s = vq.enqueue_prep().unwrap();
            vq.enqueue_reserve(s, 1).unwrap();
            vq.enqueue_commit(&t, Some(s), true);
            vq.device().use_slot(u32::from(s), 0);
        }
        assert_eq!(c.check_vq(0), 1);
        assert_eq!(st.done.load(Ordering::Relaxed), 1);

        let p = ChildRef::of(&MArc::new(M_DEVBUF, M_NOWAIT, Plain).unwrap(), &dev);
        assert!(!p.has_config_change());
        assert_eq!(p.config_change(), None);
        assert_eq!(p.check_vq(0), 5);
        // A state of another type answers 0.
        assert_eq!(child_vq_intr::<TestChild>(&p.state, 0), 0);
        assert_eq!(child_config_change::<TestChild>(&p.state), 0);
    }

    #[test]
    fn attach_finish_hands_the_queues_over_in_the_c_s_order() {
        let (t, mut vq) = queue(8, 0, 1);
        let dev = node();
        let va = VirtioArgs {
            devid: 4,
            nintr: 1,
            transport: fake_transport,
            legacy: 0,
        };
        let q = vq.setup();
        let st = MArc::new(M_DEVBUF, M_NOWAIT, Plain).unwrap();
        virtio_attach_finish(&t, &va, ChildRef::of(&st, &dev), &[q]).unwrap();
        assert_eq!(
            t.events(),
            [
                FakeEvent::AttachFinish(1),
                FakeEvent::SetupIntrs,
                FakeEvent::SetupQueue(q),
                FakeEvent::SetStatus(VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK),
            ]
        );
        assert_eq!(t.interrupt(), 5);

        let (t, mut vq) = queue(8, 0, 1);
        t.fail_finish.store(true, Ordering::Relaxed);
        let q = vq.setup();
        assert_eq!(
            virtio_attach_finish(&t, &va, ChildRef::of(&st, &dev), &[q]),
            Err(Errno::EIO)
        );
        assert_eq!(
            t.events(),
            [FakeEvent::AttachFinish(1)],
            "nothing handed over"
        );
    }

    #[test]
    fn reinit_starts_again_and_refuses_a_changed_size() {
        let (t, mut vq) = queue(8, 0, 1);
        let s = vq.enqueue_prep().unwrap();
        vq.enqueue_reserve(s, 1).unwrap();
        let mut f = Features::of_active(VIRTIO_F_ANY_LAYOUT);
        virtio_reset(&t, &mut f);
        assert_eq!(f.active, 0);
        let f = virtio_reinit_start(&t, VIRTIO_F_RING_EVENT_IDX, &mut [&mut vq]);
        assert!(f.has(VIRTIO_F_RING_EVENT_IDX) && f.indirect);
        assert!(vq.event_idx);
        assert_eq!(vq.nfree(), 8, "every slot free again");
        virtio_reinit_end(&t);
        let ev = t.events();
        assert_eq!(
            ev[0],
            FakeEvent::SetStatus(VIRTIO_CONFIG_DEVICE_STATUS_RESET)
        );
        assert_eq!(ev[3], FakeEvent::Negotiate(VIRTIO_F_RING_EVENT_IDX));
        assert_eq!(ev[4], FakeEvent::SetupIntrs);
        assert!(matches!(ev[5], FakeEvent::SetupQueue(q) if q.num == 8 && q.addr != 0));
        assert_eq!(
            ev[6],
            FakeEvent::SetStatus(VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK)
        );

        t.qsize[0].store(16, Ordering::Relaxed);
        assert!(
            catch_unwind(AssertUnwindSafe(|| virtio_reinit_start(
                &t,
                0,
                &mut [&mut vq]
            )))
            .is_err()
        );
    }

    #[test]
    fn the_args_are_read_back_from_a_typed_aux_only() {
        let va = VirtioArgs {
            devid: 4,
            nintr: 1,
            transport: fake_transport,
            legacy: 0x1234,
        };
        let aux = Aux::Typed(&va);
        let got = VirtioArgs::of_aux(&aux).expect("a virtio argument");
        assert_eq!((got.devid, got.nintr, got.legacy), (4, 1, 0x1234));
        assert!(VirtioArgs::of_aux(&Aux::Legacy(0x1234)).is_none());
        assert!(VirtioArgs::of_aux(&Aux::Typed(&5u32)).is_none());
    }

    #[test]
    fn device_names() {
        assert_eq!(virtio_device_string(1), "Network");
        assert_eq!(virtio_device_string(4), "Entropy");
        assert_eq!(virtio_device_string(5), "Memory Balloon");
        assert_eq!(virtio_device_string(0), "Unknown (0)");
        assert_eq!(virtio_device_string(12), "(null)");
        assert_eq!(virtio_device_string(16), "GPU");
        assert_eq!(virtio_device_string(17), "Unknown");
        assert_eq!(virtio_device_string(-1), "Unknown");
    }
}
/* </TESTS> */
