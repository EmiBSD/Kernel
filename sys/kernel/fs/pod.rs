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
//! The views of the file systems' on-disk structures over a buffer's bytes: a structure made
//! of bytes only (a `hal::abi_pod!` image, which the compiler lays out without padding) gets
//! from [`pod_view!`] its `SIZE`, `at(buf, off)` (the structure that starts at byte `off`,
//! copied out; `None` when the buffer ends first) and `put(buf, off, &v)` (written back
//! there; `false`, nothing written, when the buffer is too short): the C's cast of a pointer
//! into `b_data`, `(struct foo *)(bp->b_data + off)`.
//!
//! N5.8c1 lifted the macro and its two functions out of `fs::msdosfs::bpb` (N5.7c1, which
//! the boot sectors, the BPBs and the directory entries use) for their second and third
//! users, cd9660 and udf (D-N5.8-4). Cost against the C's in-place access: one copy of the
//! structure per view; a caller that needs one member of a large structure reads it at its
//! offset instead (docs/IDIOMS.md, the row of the on-disk structures of byte fields).

use hal::user::{AbiPod, pod_from_bytes, pod_to_bytes};

/// Gives an on-disk structure (a `hal::abi_pod!` of bytes) its views over a buffer's bytes:
/// `SIZE`, `at(buf, off)` (the structure that starts at byte `off`, copied out; `None` when
/// `buf` ends before it does) and `put(buf, off, &v)` (written back there; `false`, nothing
/// written, when `buf` is too short): the C's `(struct foo *)(bp->b_data + off)`.
macro_rules! pod_view {
    ($t:ty) => {
        impl $t {
            /// The size of the structure on disk.
            pub const SIZE: usize = ::core::mem::size_of::<$t>();

            /// The structure that starts at byte `off` of `buf`, or `None` when `buf` is too
            /// short for it.
            pub fn at(buf: &[u8], off: usize) -> Option<$t> {
                $crate::fs::pod::pod_at(buf, off)
            }

            /// Writes `v` at byte `off` of `buf`; `false` (nothing written) when `buf` is too
            /// short for it.
            pub fn put(buf: &mut [u8], off: usize, v: &$t) -> bool {
                $crate::fs::pod::pod_put(buf, off, v)
            }
        }

        const _: () = assert!(::core::mem::align_of::<$t>() == 1);
    };
}
pub(crate) use pod_view;

/// [`pod_view!`]'s `at`: the `T` at byte `off` of `buf`, copied out.
pub fn pod_at<T: AbiPod>(buf: &[u8], off: usize) -> Option<T> {
    pod_from_bytes(buf.get(off..)?)
}

/// [`pod_view!`]'s `put`: `v` written at byte `off` of `buf`.
pub fn pod_put<T: AbiPod>(buf: &mut [u8], off: usize, v: &T) -> bool {
    match buf.get_mut(off..) {
        Some(b) => pod_to_bytes(v, b),
        None => false,
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// A three-byte image, as the file systems declare theirs.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct Three {
        a: u8,
        b: [u8; 2],
    }

    hal::abi_pod!(Three { a: u8, b: [u8; 2] });
    pod_view!(Three);

    #[test]
    fn a_view_reads_and_writes_inside_its_buffer_only() {
        let mut buf = [1u8, 2, 3, 4];
        assert_eq!(Three::SIZE, 3);
        assert_eq!(Three::at(&buf, 1), Some(Three { a: 2, b: [3, 4] }));
        // the buffer ends before the structure does, or before its start
        assert_eq!(Three::at(&buf, 2), None);
        assert_eq!(Three::at(&buf, 9), None);
        let v = Three { a: 9, b: [8, 7] };
        assert!(Three::put(&mut buf, 0, &v));
        assert_eq!(buf, [9, 8, 7, 4]);
        assert!(!Three::put(&mut buf, 2, &v));
        assert!(!Three::put(&mut buf, 5, &v));
        assert_eq!(buf, [9, 8, 7, 4]);
        assert_eq!(pod_at::<Three>(&buf, 1), Some(Three { a: 8, b: [7, 4] }));
    }
}
/* </TESTS> */
