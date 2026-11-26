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
 * Copyright (c) 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
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
 *	@(#)xdr_subs.h	8.3 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/xdr_subs.h>`: conversion to and from the XDR representation used by NFS, as defined
//! by "XDR: External Data Representation Standard" (RFC 1014).
//!
//! Upstream: sys/nfs/xdr_subs.h @ 3ce1f3f79392
//! LZ: sys/nfs/xdr_subs.rs@3c62ede68802
//!
//! A raw XDR word is a `u32` as it lies in an mbuf (network byte order, read with the
//! machine's order): `fxdr_unsigned(w)` is the host value, `txdr_unsigned(v)` the raw word.
//! The C's type argument (`fxdr_unsigned(int, *tl)`) is a cast at the call site;
//! `txdr_unsigned` of a negative `int` is `txdr_unsigned(x as u32)`, the same bits.
//!
//! ## Deviations
//! - The statement macros `fxdr_nfsv2time(f, t)`, `fxdr_nfsv3time(f, t)`,
//!   `txdr_nfsv3time(f, t)`, `txdr_hyper(f, t)` are functions that return the converted
//!   value; `fxdr_hyper(f)` takes the two raw words. `txdr_nfsv2time`, declared here, is
//!   defined in `nfs_subs` as in C (re-exported here).
//! - [`xdr_bytes`] and [`xdr_get`] are Rust helpers: the bytes of a wire structure, and a
//!   wire structure read from bytes (the C casts the pointer).
//!
//! ## Redesign
//! - LZ's `bsd` module -> the kernel's (N6.7a0): the client (`kernel::fs::nfs`) and `bsd`'s
//!   server name this one (`bsd`'s `nfs::xdr_subs` re-exports it). The byte views of a wire
//!   structure are `hal::user`'s `bytes_of` and `bytes_of_mut` over an `AbiPod` type (LZ: two
//!   raw-pointer `unsafe` blocks over its own `AbiPod`).

use core::slice;

use hal::user::{bytes_of, bytes_of_mut, zeroed};
use sysdefs::pod::AbiPod;

use crate::time::clock::Timespec;
use sysdefs::nfs_nfsproto::{Nfsv2Time, Nfsv3Time};
use sysdefs::types::Time;

pub use super::nfs_subs::txdr_nfsv2time;

/// `fxdr_unsigned(t, v)`: the host value of the raw XDR word `v` (`ntohl`).
#[inline]
pub const fn fxdr_unsigned(v: u32) -> u32 {
    u32::from_be(v)
}

/// `txdr_unsigned(v)`: the raw XDR word of `v` (`htonl`).
#[inline]
pub const fn txdr_unsigned(v: u32) -> u32 {
    v.to_be()
}

/// `fxdr_nfsv2time(f, t)`: the time of a version 2 `nfstime2`; a `nfsv2_usec` of all ones
/// means no microseconds.
pub const fn fxdr_nfsv2time(f: &Nfsv2Time) -> Timespec {
    Timespec {
        tv_sec: fxdr_unsigned(f.nfsv2_sec) as Time,
        tv_nsec: if f.nfsv2_usec == 0xffff_ffff {
            0
        } else {
            // The C multiplies in `u_int`.
            1000u32.wrapping_mul(fxdr_unsigned(f.nfsv2_usec)) as i64
        },
    }
}

/// `fxdr_nfsv3time(f, t)`: the time of a version 3 `nfstime3`.
pub const fn fxdr_nfsv3time(f: &Nfsv3Time) -> Timespec {
    Timespec {
        tv_sec: fxdr_unsigned(f.nfsv3_sec) as Time,
        tv_nsec: fxdr_unsigned(f.nfsv3_nsec) as i64,
    }
}

/// `txdr_nfsv3time(f, t)`: the version 3 `nfstime3` of a time (both members truncated to 32
/// bits, as `htonl` does).
pub const fn txdr_nfsv3time(f: &Timespec) -> Nfsv3Time {
    Nfsv3Time {
        nfsv3_sec: txdr_unsigned(f.tv_sec as u32),
        nfsv3_nsec: txdr_unsigned(f.tv_nsec as u32),
    }
}

/// `fxdr_hyper(f)`: the 64-bit value of two raw XDR words, most significant first.
pub const fn fxdr_hyper(f: [u32; 2]) -> u64 {
    ((fxdr_unsigned(f[0]) as u64) << 32) | fxdr_unsigned(f[1]) as u64
}

/// `txdr_hyper(f, t)`: the two raw XDR words of a 64-bit value.
pub const fn txdr_hyper(f: u64) -> [u32; 2] {
    [
        txdr_unsigned((f >> 32) as u32),
        txdr_unsigned((f & 0xffff_ffff) as u32),
    ]
}

/// The bytes of a wire structure.
pub fn xdr_bytes<T: AbiPod>(v: &T) -> &[u8] {
    bytes_of(slice::from_ref(v))
}

/// A wire structure read from the start of `b`; when `b` is shorter than the structure (a
/// version 2 reply read into a structure sized for version 3), the rest is zero.
pub fn xdr_get<T: AbiPod>(b: &[u8]) -> T {
    let mut v = zeroed::<T>();
    let dst = bytes_of_mut(slice::from_mut(&mut v));
    let n = b.len().min(dst.len());
    dst[..n].copy_from_slice(&b[..n]);
    v
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_network_order() {
        let w = txdr_unsigned(0x0102_0304);
        assert_eq!(w.to_ne_bytes(), [1, 2, 3, 4]);
        assert_eq!(fxdr_unsigned(w), 0x0102_0304);
        let h = txdr_hyper(0x1122_3344_5566_7788);
        assert_eq!(h[0].to_ne_bytes(), [0x11, 0x22, 0x33, 0x44]);
        assert_eq!(fxdr_hyper(h), 0x1122_3344_5566_7788);
    }

    #[test]
    fn times() {
        let t2 = Nfsv2Time::from_words([txdr_unsigned(5), txdr_unsigned(7)]);
        assert_eq!(fxdr_nfsv2time(&t2), Timespec::new(5, 7000));
        let none = Nfsv2Time::from_words([txdr_unsigned(5), 0xffff_ffff]);
        assert_eq!(fxdr_nfsv2time(&none), Timespec::new(5, 0));
        let ts = Timespec::new(1_700_000_000, 999);
        assert_eq!(fxdr_nfsv3time(&txdr_nfsv3time(&ts)), ts);
    }

    #[test]
    fn short_reads_are_zero_filled() {
        let t: Nfsv3Time = xdr_get(&[1, 2, 3, 4]);
        assert_eq!(xdr_bytes(&t), [1, 2, 3, 4, 0, 0, 0, 0]);
    }
}
/* </TESTS> */
