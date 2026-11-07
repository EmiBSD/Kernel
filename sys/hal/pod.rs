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
//! The views of an `AbiPod` structure over a byte buffer: a protocol header in a packet's
//! bytes, an on-disk structure in a buffer's. [`at`] and [`put`] copy it out and back (the
//! C's `m_copydata` and `m_copyback` of a header, a file system's read of a structure it then
//! writes back); [`view`] and [`view_mut`] read and write it in place (the C's
//! `mtod(m, struct ip *)`, `(struct foo *)(bp->b_data + off)`), when the bytes are long
//! enough and aligned for it: `None` otherwise, where the C relies on `ETHER_ALIGN` and the
//! buffer's own alignment (a caller then copies, as the C's `m_copydata` cases do).
//!
//! ## Redesign
//! - `kernel::fs::pod`'s `pod_at` and `pod_put` (N5.8c1, over `hal::user`'s
//!   `pod_from_bytes`/`pod_to_bytes`) moved here as [`at`] and [`put`], unchanged, beside the
//!   in-place forms the network stack needs (N6.0): one copy of a 20-byte IPv4 header out and
//!   back per change costs more than the C's pointer, and its fields are read and written in
//!   place by the C (`ip_input`'s `ip->ip_ttl--`, `in_cksum` over the header).

use crate::user::{AbiPod, pod_from_bytes, pod_to_bytes};

/// The `T` at byte `off` of `buf`, copied out; `None` when `buf` ends before it does.
pub fn at<T: AbiPod>(buf: &[u8], off: usize) -> Option<T> {
    pod_from_bytes(buf.get(off..)?)
}

/// `v` written at byte `off` of `buf`; `false` (nothing written) when `buf` is too short.
pub fn put<T: AbiPod>(buf: &mut [u8], off: usize, v: &T) -> bool {
    match buf.get_mut(off..) {
        Some(b) => pod_to_bytes(v, b),
        None => false,
    }
}

/// The `T` at byte `off` of `buf`, read in place: `None` when `buf` ends before it does or
/// the bytes are not aligned for a `T`.
pub fn view<T: AbiPod>(buf: &[u8], off: usize) -> Option<&T> {
    let b = buf.get(off..off.checked_add(size_of::<T>())?)?;
    if !b.as_ptr().addr().is_multiple_of(align_of::<T>()) {
        return None;
    }
    // SAFETY: `b` is `size_of::<T>()` initialised bytes of `buf`, aligned for `T` (both checked
    // above), so the cast names a place that holds a `T`'s bytes; every bit pattern of an
    // `AbiPod` type is a valid value and it has no padding and no interior mutability (the
    // trait's contract: `abi_pod!` checks every field is an integer or an array of `AbiPod`,
    // nothing else implements it), so those bytes are a valid `T` and reading them through a
    // shared reference changes nothing. The reference borrows `buf` for its lifetime, so no
    // `&mut` to those bytes can be alive beside it.
    Some(unsafe { &*b.as_ptr().cast::<T>() })
}

/// The `T` at byte `off` of `buf`, read and written in place: `None` as for [`view`].
pub fn view_mut<T: AbiPod>(buf: &mut [u8], off: usize) -> Option<&mut T> {
    let end = off.checked_add(size_of::<T>())?;
    let b = buf.get_mut(off..end)?;
    if !b.as_ptr().addr().is_multiple_of(align_of::<T>()) {
        return None;
    }
    // SAFETY: as for `view`, and since every bit pattern is a valid `T` (and a valid run of
    // bytes), whatever is written through the reference leaves `buf` valid; it borrows `buf`
    // mutably for its lifetime, so it is the only reference to those bytes while it lives.
    Some(unsafe { &mut *b.as_mut_ptr().cast::<T>() })
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// `struct ip` (netinet/ip.h): 20 bytes.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(C)]
    struct Ip {
        vhl: u8,
        tos: u8,
        len: u16,
        id: u16,
        off: u16,
        ttl: u8,
        p: u8,
        sum: u16,
        src: u32,
        dst: u32,
    }
    crate::abi_pod!(Ip {
        vhl: u8,
        tos: u8,
        len: u16,
        id: u16,
        off: u16,
        ttl: u8,
        p: u8,
        sum: u16,
        src: u32,
        dst: u32
    });

    /// 64 bytes aligned to 8.
    #[repr(C, align(8))]
    struct Pkt([u8; 64]);

    #[test]
    fn a_view_needs_the_bytes_and_their_alignment() {
        let mut pkt = Pkt([0; 64]);
        pkt.0[16] = 0x45;
        pkt.0[24] = 64;
        let v: &Ip = view(&pkt.0, 16).expect("aligned and long enough");
        assert_eq!((v.vhl, v.ttl), (0x45, 64));
        assert!(
            view::<Ip>(&pkt.0, 14).is_none(),
            "misaligned: the C's ETHER_ALIGN case"
        );
        assert!(view::<Ip>(&pkt.0, 48).is_none(), "the packet ends first");
        assert!(view::<Ip>(&pkt.0, usize::MAX).is_none());
        assert!(view::<u8>(&pkt.0, 63).is_some());
        assert!(at::<Ip>(&pkt.0, 14).is_some(), "a copy needs no alignment");
    }

    #[test]
    fn writes_in_place_are_seen_by_the_copies_and_back() {
        let mut pkt = Pkt([0; 64]);
        let v: &mut Ip = view_mut(&mut pkt.0, 16).expect("aligned");
        v.ttl = 63;
        v.id = 0x1234;
        let c: Ip = at(&pkt.0, 16).expect("long enough");
        assert_eq!((c.ttl, c.id), (63, 0x1234));
        let mut c2 = c;
        c2.p = 6;
        assert!(put(&mut pkt.0, 16, &c2));
        assert_eq!(view::<Ip>(&pkt.0, 16), Some(&c2));
        assert!(!put(&mut pkt.0, 50, &c2), "too short: nothing written");
        assert!(view_mut::<Ip>(&mut pkt.0, 18).is_none(), "misaligned");
    }
}
/* </TESTS> */
