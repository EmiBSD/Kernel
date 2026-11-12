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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)ip.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The IPv4 header, `<netinet/ip.h>`'s `struct ip` and the fragment bits the kernel's
//! Ethernet layer reads (N6.3c2, D-N6.3c-21). N6.4 completes it.
//!
//! Upstream: sys/netinet/ip.h @ 3ce1f3f79392
//! LZ: sys/netinet/ip.rs@3c62ede68802
//!
//! ## Deviations
//! - `ip_hl:4` and `ip_v:4` are one byte, `ip_vhl` (the low and high nibbles on the
//!   little-endian machines), with [`Ip::ip_hl`] and [`Ip::ip_v`] (LZ's).
//!
//! ## Redesign
//! - LZ's image in `bsd`'s `sys/netinet/ip.rs` -> `hal`, where `kernel::net` names it; `bsd`
//!   keeps its copy until N6.4.

use core::mem::size_of;

use crate::sys::netinet_in::InAddr;

/// `IP_MF`: more fragments flag (of `ip_off`, host order).
pub const IP_MF: u16 = 0x2000;
/// `IP_OFFMASK`: mask for fragmenting bits.
pub const IP_OFFMASK: u16 = 0x1fff;

/// `struct ip`: an internet header, naked of options; multi-byte members in network order.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ip {
    /// `ip_v:4` (the high nibble) and `ip_hl:4` (the low nibble).
    pub ip_vhl: u8,
    /// Type of service.
    pub ip_tos: u8,
    /// Total length.
    pub ip_len: u16,
    /// Identification.
    pub ip_id: u16,
    /// Fragment offset field.
    pub ip_off: u16,
    /// Time to live.
    pub ip_ttl: u8,
    /// Protocol.
    pub ip_p: u8,
    /// Checksum.
    pub ip_sum: u16,
    /// Source address.
    pub ip_src: InAddr,
    /// Destination address.
    pub ip_dst: InAddr,
}

crate::abi_pod!(Ip {
    ip_vhl: u8,
    ip_tos: u8,
    ip_len: u16,
    ip_id: u16,
    ip_off: u16,
    ip_ttl: u8,
    ip_p: u8,
    ip_sum: u16,
    ip_src: InAddr,
    ip_dst: InAddr,
});

impl Ip {
    /// `ip_hl`: the header length, in 32-bit words.
    #[inline]
    pub const fn ip_hl(&self) -> u8 {
        self.ip_vhl & 0x0f
    }

    /// `ip_v`: the version.
    #[inline]
    pub const fn ip_v(&self) -> u8 {
        self.ip_vhl >> 4
    }
}

const _: () = assert!(size_of::<Ip>() == 20);
/* </CODE> */
