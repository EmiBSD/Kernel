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
//! `<dev/pv/virtioreg.h>` in the kernel: the virtio device ids, feature and status bits, and
//! the ring layout, from the "Virtio PCI Card Specification v0.8.6 DRAFT" Appendix A.
//!
//! Upstream: sys/dev/pv/virtioreg.h @ 3ce1f3f79392
//! LZ: sys/dev/pv/virtioreg.rs@e8249449d905
//!
//! The standard layout for the ring is a continuous chunk of memory which looks like this.
//! We assume num is a power of 2.
//!
//! ```text
//! struct vring {
//!      // The actual descriptors (16 bytes each)
//!      struct vring_desc desc[num];
//!
//!      // A ring of available descriptor heads with free-running index.
//!      __u16 avail_flags;
//!      __u16 avail_idx;
//!      __u16 available[num];
//!      __u16 used_event_idx
//!
//!      // Padding to the next align boundary.
//!      char pad[];
//!
//!      // A ring of used descriptor heads with free-running index.
//!      __u16 used_flags;
//!      __u16 used_idx;
//!      struct vring_used_elem used[num];
//!      __u16 avail_event_idx;
//! };
//! ```
//! Note: for virtio PCI, align is 4096.
//!
//! ## Redesign
//! - LZ's ring structures were reached through raw pointers into the ring with volatile
//!   accesses; here the rings are a `hal::dev::dma::DmaBuf` read and written member by member,
//!   so the layout is the byte offsets below (`VRING_DESC_*`, `VRING_AVAIL_*`,
//!   `VRING_USED_*`, the elements' `VRING_USED_ELEM_*`), each taken from the `#[repr(C)]`
//!   structures with `offset_of!` and checked against the C's numbers. The structures stay
//!   as the layout's definition; nothing makes one. `VQ_USED_EVENT` and `VQ_AVAIL_EVENT` are
//!   `ring[vq_num]` of the avail and used rings (`dev/pv/virtio.rs`).
//! - The kernel's copy beside `bsd`'s `dev/pv/virtioreg.rs`, which the legacy core and its
//!   five legacy children read until N7.3d (docs/SCAFFOLDING.md).

use core::mem::offset_of;

/// `PCI_PRODUCT_VIRTIO_NETWORK`: virtio product id (subsystem).
pub const PCI_PRODUCT_VIRTIO_NETWORK: i32 = 1;
/// `PCI_PRODUCT_VIRTIO_BLOCK`.
pub const PCI_PRODUCT_VIRTIO_BLOCK: i32 = 2;
/// `PCI_PRODUCT_VIRTIO_CONSOLE`.
pub const PCI_PRODUCT_VIRTIO_CONSOLE: i32 = 3;
/// `PCI_PRODUCT_VIRTIO_ENTROPY`.
pub const PCI_PRODUCT_VIRTIO_ENTROPY: i32 = 4;
/// `PCI_PRODUCT_VIRTIO_BALLOON`.
pub const PCI_PRODUCT_VIRTIO_BALLOON: i32 = 5;
/// `PCI_PRODUCT_VIRTIO_IOMEM`.
pub const PCI_PRODUCT_VIRTIO_IOMEM: i32 = 6;
/// `PCI_PRODUCT_VIRTIO_RPMSG`.
pub const PCI_PRODUCT_VIRTIO_RPMSG: i32 = 7;
/// `PCI_PRODUCT_VIRTIO_SCSI`.
pub const PCI_PRODUCT_VIRTIO_SCSI: i32 = 8;
/// `PCI_PRODUCT_VIRTIO_9P`.
pub const PCI_PRODUCT_VIRTIO_9P: i32 = 9;
/// `PCI_PRODUCT_VIRTIO_MAC80211`.
pub const PCI_PRODUCT_VIRTIO_MAC80211: i32 = 10;
/// `PCI_PRODUCT_VIRTIO_GPU`.
pub const PCI_PRODUCT_VIRTIO_GPU: i32 = 16;
/// `PCI_PRODUCT_VIRTIO_VMMCI`: private id.
pub const PCI_PRODUCT_VIRTIO_VMMCI: i32 = 65535;

// Device-independent feature bits.

/// `VIRTIO_F_NOTIFY_ON_EMPTY`.
pub const VIRTIO_F_NOTIFY_ON_EMPTY: u64 = 1 << 24;
/// `VIRTIO_F_ANY_LAYOUT`.
pub const VIRTIO_F_ANY_LAYOUT: u64 = 1 << 27;
/// `VIRTIO_F_RING_INDIRECT_DESC`.
pub const VIRTIO_F_RING_INDIRECT_DESC: u64 = 1 << 28;
/// `VIRTIO_F_RING_EVENT_IDX`.
pub const VIRTIO_F_RING_EVENT_IDX: u64 = 1 << 29;
/// `VIRTIO_F_BAD_FEATURE`.
pub const VIRTIO_F_BAD_FEATURE: u64 = 1 << 30;
/// `VIRTIO_F_VERSION_1`.
pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;
/// `VIRTIO_F_ACCESS_PLATFORM`.
pub const VIRTIO_F_ACCESS_PLATFORM: u64 = 1 << 33;
/// `VIRTIO_F_RING_PACKED`.
pub const VIRTIO_F_RING_PACKED: u64 = 1 << 34;
/// `VIRTIO_F_IN_ORDER`.
pub const VIRTIO_F_IN_ORDER: u64 = 1 << 35;
/// `VIRTIO_F_ORDER_PLATFORM`.
pub const VIRTIO_F_ORDER_PLATFORM: u64 = 1 << 36;
/// `VIRTIO_F_SR_IOV`.
pub const VIRTIO_F_SR_IOV: u64 = 1 << 37;
/// `VIRTIO_F_NOTIFICATION_DATA`.
pub const VIRTIO_F_NOTIFICATION_DATA: u64 = 1 << 38;
/// `VIRTIO_F_NOTIF_CONFIG_DATA`.
pub const VIRTIO_F_NOTIF_CONFIG_DATA: u64 = 1 << 39;
/// `VIRTIO_F_RING_RESET`.
pub const VIRTIO_F_RING_RESET: u64 = 1 << 40;

// Device status bits.

/// `VIRTIO_CONFIG_DEVICE_STATUS_RESET`.
pub const VIRTIO_CONFIG_DEVICE_STATUS_RESET: i32 = 0;
/// `VIRTIO_CONFIG_DEVICE_STATUS_ACK`.
pub const VIRTIO_CONFIG_DEVICE_STATUS_ACK: i32 = 1;
/// `VIRTIO_CONFIG_DEVICE_STATUS_DRIVER`.
pub const VIRTIO_CONFIG_DEVICE_STATUS_DRIVER: i32 = 2;
/// `VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK`.
pub const VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK: i32 = 4;
/// `VIRTIO_CONFIG_DEVICE_STATUS_FEATURES_OK`.
pub const VIRTIO_CONFIG_DEVICE_STATUS_FEATURES_OK: i32 = 8;
/// `VIRTIO_CONFIG_DEVICE_STATUS_DEVICE_NEEDS_RESET`.
pub const VIRTIO_CONFIG_DEVICE_STATUS_DEVICE_NEEDS_RESET: i32 = 64;
/// `VIRTIO_CONFIG_DEVICE_STATUS_FAILED`.
pub const VIRTIO_CONFIG_DEVICE_STATUS_FAILED: i32 = 128;

// Virtqueue

/// `VRING_DESC_F_NEXT`: this marks a buffer as continuing via the next field.
pub const VRING_DESC_F_NEXT: u16 = 1;
/// `VRING_DESC_F_WRITE`: this marks a buffer as write-only (otherwise read-only).
pub const VRING_DESC_F_WRITE: u16 = 2;
/// `VRING_DESC_F_INDIRECT`: this means the buffer contains a list of buffer descriptors.
pub const VRING_DESC_F_INDIRECT: u16 = 4;

/// `VRING_USED_F_NO_NOTIFY`: the Host uses this in `used->flags` to advise the Guest: don't
/// kick me when you add a buffer. It's unreliable, so it's simply an optimization. Guest
/// will still kick if it's out of buffers.
pub const VRING_USED_F_NO_NOTIFY: u16 = 1;
/// `VRING_AVAIL_F_NO_INTERRUPT`: the Guest uses this in `avail->flags` to advise the Host:
/// don't interrupt me when you consume a buffer. It's unreliable, so it's simply an
/// optimization.
pub const VRING_AVAIL_F_NO_INTERRUPT: u16 = 1;

/// `VIRTIO_PAGE_SIZE`.
pub const VIRTIO_PAGE_SIZE: usize = 4096;

/// `struct vring_desc`: virtio ring descriptors, 16 bytes. These can chain together via
/// `next`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VringDesc {
    /// Address (guest-physical).
    pub addr: u64,
    /// Length.
    pub len: u32,
    /// The flags as indicated above.
    pub flags: u16,
    /// We chain unused descriptors via this, too.
    pub next: u16,
}

/// `struct vring_avail`: the driver's ring of available descriptor heads; `ring[]` follows
/// at [`VRING_AVAIL_RING`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VringAvail {
    /// `flags`.
    pub flags: u16,
    /// `idx`: free-running index of the next slot the driver fills.
    pub idx: u16,
}

/// `struct vring_used_elem`. `u32` is used here for ids for padding reasons.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VringUsedElem {
    /// Index of start of used descriptor chain.
    pub id: u32,
    /// Total length of the descriptor chain which was written to.
    pub len: u32,
}

/// `struct vring_used`: the device's ring of used descriptor chains; `ring[]` follows at
/// [`VRING_USED_RING`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VringUsed {
    /// `flags`.
    pub flags: u16,
    /// `idx`: free-running index of the next slot the device fills.
    pub idx: u16,
}

/// `sizeof(struct vring_desc)`.
pub const VRING_DESC_SIZE: usize = size_of::<VringDesc>();
/// `offsetof(struct vring_desc, addr)`.
pub const VRING_DESC_ADDR: usize = offset_of!(VringDesc, addr);
/// `offsetof(struct vring_desc, len)`.
pub const VRING_DESC_LEN: usize = offset_of!(VringDesc, len);
/// `offsetof(struct vring_desc, flags)`.
pub const VRING_DESC_FLAGS: usize = offset_of!(VringDesc, flags);
/// `offsetof(struct vring_desc, next)`.
pub const VRING_DESC_NEXT: usize = offset_of!(VringDesc, next);
/// `offsetof(struct vring_avail, flags)`.
pub const VRING_AVAIL_FLAGS: usize = offset_of!(VringAvail, flags);
/// `offsetof(struct vring_avail, idx)`.
pub const VRING_AVAIL_IDX: usize = offset_of!(VringAvail, idx);
/// `offsetof(struct vring_avail, ring)`.
pub const VRING_AVAIL_RING: usize = size_of::<VringAvail>();
/// `offsetof(struct vring_used, flags)`.
pub const VRING_USED_FLAGS: usize = offset_of!(VringUsed, flags);
/// `offsetof(struct vring_used, idx)`.
pub const VRING_USED_IDX: usize = offset_of!(VringUsed, idx);
/// `offsetof(struct vring_used, ring)`.
pub const VRING_USED_RING: usize = size_of::<VringUsed>();
/// `sizeof(struct vring_used_elem)`.
pub const VRING_USED_ELEM_SIZE: usize = size_of::<VringUsedElem>();
/// `offsetof(struct vring_used_elem, id)`.
pub const VRING_USED_ELEM_ID: usize = offset_of!(VringUsedElem, id);
/// `offsetof(struct vring_used_elem, len)`.
pub const VRING_USED_ELEM_LEN: usize = offset_of!(VringUsedElem, len);

// The structures are __packed in C; the natural layout here has no padding, so it is the
// same, and the offsets are the C's.
const _: () = {
    assert!(VRING_DESC_SIZE == 16);
    assert!(VRING_DESC_ADDR == 0);
    assert!(VRING_DESC_LEN == 8);
    assert!(VRING_DESC_FLAGS == 12);
    assert!(VRING_DESC_NEXT == 14);
    assert!(VRING_AVAIL_FLAGS == 0 && VRING_AVAIL_IDX == 2 && VRING_AVAIL_RING == 4);
    assert!(VRING_USED_FLAGS == 0 && VRING_USED_IDX == 2 && VRING_USED_RING == 4);
    assert!(VRING_USED_ELEM_SIZE == 8 && VRING_USED_ELEM_ID == 0 && VRING_USED_ELEM_LEN == 4);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/dev/pv/virtioreg.h");
        crate::reftest::assert_defines!(defs;
            PCI_PRODUCT_VIRTIO_NETWORK, PCI_PRODUCT_VIRTIO_BLOCK, PCI_PRODUCT_VIRTIO_CONSOLE,
            PCI_PRODUCT_VIRTIO_ENTROPY, PCI_PRODUCT_VIRTIO_BALLOON, PCI_PRODUCT_VIRTIO_IOMEM,
            PCI_PRODUCT_VIRTIO_RPMSG, PCI_PRODUCT_VIRTIO_SCSI, PCI_PRODUCT_VIRTIO_9P,
            PCI_PRODUCT_VIRTIO_MAC80211, PCI_PRODUCT_VIRTIO_GPU, PCI_PRODUCT_VIRTIO_VMMCI,
            VIRTIO_F_NOTIFY_ON_EMPTY, VIRTIO_F_ANY_LAYOUT, VIRTIO_F_RING_INDIRECT_DESC,
            VIRTIO_F_RING_EVENT_IDX, VIRTIO_F_BAD_FEATURE, VIRTIO_F_VERSION_1,
            VIRTIO_F_ACCESS_PLATFORM, VIRTIO_F_RING_PACKED, VIRTIO_F_IN_ORDER,
            VIRTIO_F_ORDER_PLATFORM, VIRTIO_F_SR_IOV, VIRTIO_F_NOTIFICATION_DATA,
            VIRTIO_F_NOTIF_CONFIG_DATA, VIRTIO_F_RING_RESET,
            VIRTIO_CONFIG_DEVICE_STATUS_RESET, VIRTIO_CONFIG_DEVICE_STATUS_ACK,
            VIRTIO_CONFIG_DEVICE_STATUS_DRIVER, VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK,
            VIRTIO_CONFIG_DEVICE_STATUS_FEATURES_OK,
            VIRTIO_CONFIG_DEVICE_STATUS_DEVICE_NEEDS_RESET, VIRTIO_CONFIG_DEVICE_STATUS_FAILED,
            VRING_DESC_F_NEXT, VRING_DESC_F_WRITE, VRING_DESC_F_INDIRECT,
            VRING_USED_F_NO_NOTIFY, VRING_AVAIL_F_NO_INTERRUPT, VIRTIO_PAGE_SIZE,
        );
    }

    #[test]
    fn feature_bits_are_the_spec_bit_numbers() {
        assert_eq!(VIRTIO_F_RING_INDIRECT_DESC.trailing_zeros(), 28);
        assert_eq!(VIRTIO_F_RING_EVENT_IDX.trailing_zeros(), 29);
        assert_eq!(VIRTIO_F_VERSION_1.trailing_zeros(), 32);
        assert_eq!(VIRTIO_F_RING_RESET.trailing_zeros(), 40);
    }

    /// The ring as a `__packed` C mirror lays it out, `ring[]` included.
    #[repr(C, packed)]
    #[allow(dead_code, reason = "a layout mirror, never made")]
    struct PackedAvail {
        flags: u16,
        idx: u16,
        ring: [u16; 4],
    }

    /// The used ring, as [`PackedAvail`].
    #[repr(C, packed)]
    #[allow(dead_code, reason = "a layout mirror, never made")]
    struct PackedUsed {
        flags: u16,
        idx: u16,
        ring: [VringUsedElem; 4],
    }

    #[test]
    fn the_offsets_are_a_packed_mirror_s() {
        assert_eq!(offset_of!(PackedAvail, flags), VRING_AVAIL_FLAGS);
        assert_eq!(offset_of!(PackedAvail, idx), VRING_AVAIL_IDX);
        assert_eq!(offset_of!(PackedAvail, ring), VRING_AVAIL_RING);
        assert_eq!(offset_of!(PackedUsed, flags), VRING_USED_FLAGS);
        assert_eq!(offset_of!(PackedUsed, idx), VRING_USED_IDX);
        assert_eq!(offset_of!(PackedUsed, ring), VRING_USED_RING);
        assert_eq!(
            size_of::<PackedUsed>(),
            VRING_USED_RING + 4 * VRING_USED_ELEM_SIZE
        );
    }
}
/* </TESTS> */
