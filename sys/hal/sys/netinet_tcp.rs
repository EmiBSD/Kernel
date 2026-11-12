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
 *	@(#)tcp.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The TCP header, `<netinet/tcp.h>`'s `struct tcphdr` (N6.3c2, D-N6.3c-21): what the
//! kernel's Ethernet layer reads (`th_off`). N6.6 completes it.
//!
//! Upstream: sys/netinet/tcp.h @ 3ce1f3f79392
//! LZ: sys/netinet/tcp.rs@3c62ede68802
//!
//! ## Deviations
//! - `th_x2:4` and `th_off:4` are one byte, `th_x2_off` (`th_off` the high nibble), with
//!   [`Tcphdr::th_off`] (LZ's).
//!
//! ## Redesign
//! - LZ's image in `bsd`'s `sys/netinet/tcp.rs` -> `hal`, where `kernel::net` names it; `bsd`
//!   keeps its copy until N6.6.

use core::mem::size_of;

/// `struct tcphdr`: the TCP header, per RFC 793; multi-byte members in network order.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tcphdr {
    /// Source port.
    pub th_sport: u16,
    /// Destination port.
    pub th_dport: u16,
    /// Sequence number (`tcp_seq`).
    pub th_seq: u32,
    /// Acknowledgement number.
    pub th_ack: u32,
    /// `th_off:4` (the high nibble, the data offset in 32-bit words) and `th_x2:4`.
    pub th_x2_off: u8,
    /// Flags (`TH_*`).
    pub th_flags: u8,
    /// Window.
    pub th_win: u16,
    /// Checksum.
    pub th_sum: u16,
    /// Urgent pointer.
    pub th_urp: u16,
}

crate::abi_pod!(Tcphdr {
    th_sport: u16,
    th_dport: u16,
    th_seq: u32,
    th_ack: u32,
    th_x2_off: u8,
    th_flags: u8,
    th_win: u16,
    th_sum: u16,
    th_urp: u16,
});

impl Tcphdr {
    /// `th_off`: the data offset (header length), in 32-bit words.
    #[inline]
    pub const fn th_off(&self) -> u8 {
        self.th_x2_off >> 4
    }
}

const _: () = assert!(size_of::<Tcphdr>() == 20);
/* </CODE> */
