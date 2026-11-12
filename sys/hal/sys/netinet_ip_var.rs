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
 *	@(#)ip_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<netinet/ip_var.h>`'s userland part: `MAX_IPOPTLEN`, the header overlay `struct ipovly`,
//! `struct ipstat` (`net.inet.ip.stats`, which netstat(1) reads) and `struct ipoption`.
//!
//! Upstream: sys/netinet/ip_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/ip_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `struct ipstat`'s `u_long`s are `u64` (LP64), the words the sysctl copies.
//!
//! ## Redesign
//! - LZ's images in `bsd`'s `sys/netinet/ip_var.rs` -> `hal`, `abi_pod!` images; the
//!   `_KERNEL` part (the counters, `struct ip_moptions`, the reassembly queues) is
//!   `kernel::netinet::ip_var`'s and `bsd`'s until their slices.

use core::mem::size_of;

use crate::sys::netinet_in::InAddr;

/// Structure stored in mbuf in `inpcb.ip_options` and passed to `ip_output` when ip options
/// are in use: the most option bytes. The actual length of the options (including
/// `ipopt_dst`) is in `m_len`.
pub const MAX_IPOPTLEN: usize = 40;

/// `struct ipovly`: overlay for ip header used by other protocols (tcp, udp).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ipovly {
    /// (unused).
    pub ih_x1: [u8; 9],
    /// Protocol.
    pub ih_pr: u8,
    /// Protocol length.
    pub ih_len: u16,
    /// Source internet address.
    pub ih_src: InAddr,
    /// Destination internet address.
    pub ih_dst: InAddr,
}

crate::abi_pod!(Ipovly {
    ih_x1: [u8; 9],
    ih_pr: u8,
    ih_len: u16,
    ih_src: InAddr,
    ih_dst: InAddr,
});

/// `struct ipstat`: IP statistics, as `sysctl(2)` returns them.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ipstat {
    /// Total packets received.
    pub ips_total: u64,
    /// Checksum bad.
    pub ips_badsum: u64,
    /// Packet too short.
    pub ips_tooshort: u64,
    /// Not enough data.
    pub ips_toosmall: u64,
    /// IP header length < data size.
    pub ips_badhlen: u64,
    /// IP length < ip header length.
    pub ips_badlen: u64,
    /// Fragments received.
    pub ips_fragments: u64,
    /// Frags dropped (dups, out of space).
    pub ips_fragdropped: u64,
    /// Fragments timed out.
    pub ips_fragtimeout: u64,
    /// Packets forwarded.
    pub ips_forward: u64,
    /// Packets rcvd for unreachable dest.
    pub ips_cantforward: u64,
    /// Packets forwarded on same net.
    pub ips_redirectsent: u64,
    /// Unknown or unsupported protocol.
    pub ips_noproto: u64,
    /// Datagrams delivered to upper level.
    pub ips_delivered: u64,
    /// Total ip packets generated here.
    pub ips_localout: u64,
    /// Lost output due to nobufs, etc.
    pub ips_odropped: u64,
    /// Total packets reassembled ok.
    pub ips_reassembled: u64,
    /// Datagrams successfully fragmented.
    pub ips_fragmented: u64,
    /// Output fragments created.
    pub ips_ofragments: u64,
    /// Don't fragment flag was set, etc.
    pub ips_cantfrag: u64,
    /// Error in option processing.
    pub ips_badoptions: u64,
    /// Packets discarded due to no route.
    pub ips_noroute: u64,
    /// IP version != 4.
    pub ips_badvers: u64,
    /// Total raw ip packets generated.
    pub ips_rawout: u64,
    /// Malformed fragments (bad length).
    pub ips_badfrags: u64,
    /// Frags dropped for lack of memory.
    pub ips_rcvmemdrop: u64,
    /// IP length > max ip packet size.
    pub ips_toolong: u64,
    /// No match gif found.
    pub ips_nogif: u64,
    /// Invalid address on header.
    pub ips_badaddr: u64,
    /// Software checksummed on input.
    pub ips_inswcsum: u64,
    /// Software checksummed on output.
    pub ips_outswcsum: u64,
    /// Multicasts for unregistered groups.
    pub ips_notmember: u64,
    /// Valid route found in cache.
    pub ips_rtcachehit: u64,
    /// Route cache with new destination.
    pub ips_rtcachemiss: u64,
    /// Packet received on wrong interface.
    pub ips_wrongif: u64,
    /// Lost input due to nobufs, etc.
    pub ips_idropped: u64,
}

crate::abi_pod!(Ipstat {
    ips_total: u64,
    ips_badsum: u64,
    ips_tooshort: u64,
    ips_toosmall: u64,
    ips_badhlen: u64,
    ips_badlen: u64,
    ips_fragments: u64,
    ips_fragdropped: u64,
    ips_fragtimeout: u64,
    ips_forward: u64,
    ips_cantforward: u64,
    ips_redirectsent: u64,
    ips_noproto: u64,
    ips_delivered: u64,
    ips_localout: u64,
    ips_odropped: u64,
    ips_reassembled: u64,
    ips_fragmented: u64,
    ips_ofragments: u64,
    ips_cantfrag: u64,
    ips_badoptions: u64,
    ips_noroute: u64,
    ips_badvers: u64,
    ips_rawout: u64,
    ips_badfrags: u64,
    ips_rcvmemdrop: u64,
    ips_toolong: u64,
    ips_nogif: u64,
    ips_badaddr: u64,
    ips_inswcsum: u64,
    ips_outswcsum: u64,
    ips_notmember: u64,
    ips_rtcachehit: u64,
    ips_rtcachemiss: u64,
    ips_wrongif: u64,
    ips_idropped: u64,
});

/// `struct ipoption`: the options of an outgoing packet.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipoption {
    /// First-hop dst if source routed.
    pub ipopt_dst: InAddr,
    /// Options proper.
    pub ipopt_list: [i8; MAX_IPOPTLEN],
}

crate::abi_pod!(Ipoption {
    ipopt_dst: InAddr,
    ipopt_list: [i8; MAX_IPOPTLEN],
});

// LP64 sizes of the C structures.
const _: () = {
    assert!(size_of::<Ipovly>() == 20);
    assert!(size_of::<Ipstat>() == 36 * 8);
    assert!(size_of::<Ipoption>() == 44);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipovly_overlays_the_ip_header() {
        use crate::sys::netinet_ip::Ip;
        assert_eq!(size_of::<Ipovly>(), size_of::<Ip>());
        assert_eq!(
            core::mem::offset_of!(Ipovly, ih_src),
            core::mem::offset_of!(Ip, ip_src)
        );
        assert_eq!(
            core::mem::offset_of!(Ipovly, ih_pr),
            core::mem::offset_of!(Ip, ip_p)
        );
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/ip_var.h");
        crate::reftest::assert_defines!(defs; MAX_IPOPTLEN);
    }
}
/* </TESTS> */
