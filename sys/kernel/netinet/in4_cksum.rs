/*	$KAME: in4_cksum.c,v 1.10 2001/11/30 10:06:15 itojun Exp $	*/
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
 * Copyright (C) 1999 WIDE Project.
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
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (c) 1988, 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)in_cksum.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The Internet checksum of an IPv4 payload, with or without the pseudo header:
//! `in4_cksum`.
//!
//! Upstream: sys/netinet/in4_cksum.c @ 3ce1f3f79392
//! LZ: sys/netinet/in4_cksum.rs@3c62ede68802
//!
//! Checksum routine for Internet Protocol family headers (portable version), for the IPv4
//! pseudo header checksum: no need to clear non-pseudo-header fields in the IPv4 header. `len`
//! is the actual payload size, not including the IPv4 header and skipped header chain (`off
//! + len` should be equal to the whole packet). With `nxt` 0 there is no pseudo header (ICMP).
//!
//! ## Deviations
//! - The word loop is `netinet/in_cksum.rs`'s [`cksum_add`] (the C file carries a copy of
//!   `in_cksum`'s loop); the pseudo header's words are summed from a byte array laid out as
//!   `struct ipovly`.
//! - The result is a `u16`.
//!
//! ## Redesign
//! - LZ's `read_unaligned` of the header at `mtod(m)` and its 12-byte pseudo header array
//!   -> the protocol and length words added directly and `ip_src`/`ip_dst` read in place
//!   from the kernel's `Mbuf::data()` as the four 16-bit words they sum to, with no `unsafe`
//!   and no copy (a whole-header `hal::pod::at::<Ip>` was a memmove call in the dev profile,
//!   +34 % on N6.4a's `in4.cksum.40`); the check that the first mbuf holds a header is the
//!   slice's bound (`m_len < sizeof(struct ip)`, the C's panic). `#[inline]`: a per-packet
//!   helper of `bsd`'s transports across the crate boundary (N6.3c1's R12).

use core::mem::size_of;

use hal::sys::endian::htons;
use hal::sys::netinet_ip::Ip;
use hal::sys::netinet_ip_var::Ipovly;

use crate::assert::kpanic;
use crate::net::mbuf::Mbuf;
use crate::netinet::in_cksum::{cksum_add, cksum_fold};
use crate::prf::printf;

/// `offsetof(struct ip, ip_src)`: the source, then the destination.
const IP_SRC: usize = core::mem::offset_of!(Ip, ip_src);

/// `in4_cksum`: the checksum of `len` bytes of `m` from `off`, over the pseudo header of
/// protocol `nxt` when it is not 0.
#[inline]
pub fn in4_cksum(m: &Mbuf, nxt: u8, off: i32, len: i32) -> u16 {
    let mut sum = 0u64;
    let mut odd = false;

    if nxt != 0 {
        // pseudo header
        if (off as usize) < size_of::<Ipovly>() {
            kpanic(format_args!("in4_cksum: offset too short"));
        }
        // `ip_src` and `ip_dst`, read in place as the four 16-bit words they sum to (no copy
        // of the whole header: a call to memmove in the dev profile, N6.4a's bench).
        let Some(addrs) = m.data().get(IP_SRC..IP_SRC + 8) else {
            kpanic(format_args!("in4_cksum: bad mbuf chain"));
        };
        // ih_x1[8] = 0, ih_pr = nxt, ih_len, ih_src, ih_dst: the words 4..9 of the overlay,
        // whose first 8 bytes are zeroes.
        sum += u64::from(u16::from_ne_bytes([0, nxt]));
        sum += u64::from(htons(len as u16));
        let (words, _) = addrs.as_chunks::<2>();
        for w in words {
            sum += u64::from(u16::from_ne_bytes(*w));
        }
    }

    // skip unnecessary part and sum the payload
    if cksum_add(Some(m), off as usize, len as usize, &mut sum, &mut odd) != 0 {
        printf(format_args!("cksum4: out of data\n"));
    }
    cksum_fold(sum)
}
/* </CODE> */
