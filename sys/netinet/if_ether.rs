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
 *	@(#)if_ether.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)if_ether.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Ethernet: `<netinet/if_ether.h>` and `netinet/if_ether.c` under LZ's path for `bsd`'s
//! importers. The userland half of the header is `hal::sys::netinet_if_ether` (re-exported
//! here), `struct arpcom` and the Ethernet layer are `kernel::net::if_ethersubr`, and ARP
//! itself is `kernel::netinet::if_ether` since N6.4b (re-exported here); what is left of
//! `bsd`'s own is the pointer view of `struct ether_extracted` (N6.3c2's R5) and ND's
//! `ETHER_MAP_IPV6_MULTICAST` over `bsd`'s `struct in6_addr` (N6.5).
//!
//! Upstream: sys/netinet/if_ether.h @ 3ce1f3f79392
//! Upstream: sys/netinet/if_ether.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `struct ether_extracted` points into the mbuf with raw pointers, as the C does (the
//!   packet data has no alignment guarantee, so they are read unaligned); `tcphdr` and
//!   `udphdr` are not ported, so `tcp` and `udp` are byte pointers.
//! - `ETHER_MAP_IPV6_MULTICAST(ip6addr, enaddr)` is a function that returns the Ethernet
//!   address.
//! - `arpintr()` is the kernel's under the net lock `bsd` holds (its proof made through the
//!   `SaToken`), for `bsd`'s tests that drive the ARP queue by hand.

pub use hal::sys::netinet_if_ether::*;
pub use kernel::net::if_ethersubr::{Arpcom, EtherMulti, EtherPort, arpcom_of};
pub use kernel::netinet::if_ether::{
    ARPINQ, ARPT_DOWN, ARPT_KEEP, LA_HOLD_QUEUE, LA_HOLD_TOTAL, LlinfoArp, SDL_ALEN, arpinit,
    gw_alen_family, gw_family_len, gw_lladdr, gw_sdl_copy, gw_sdl_index, gw_set_lladdr,
    gw_set_type_index, if_sadl_copy, la_hold_total,
};
#[cfg(feature = "nfsclient")]
pub use kernel::netinet::if_ether::{
    REVARP_FINISHED, REVARP_IFIDX, REVARP_MYIP, REVARP_SRVIP, revarprequest, revarpwhoami,
    revarpwhoarewe,
};

use crate::netinet::in_::InAddr;
use crate::netinet::ip::Ip;
use crate::netinet::ip6::Ip6Hdr;
use crate::netinet6::in6::In6Addr;

/// `struct ether_extracted`: a quick view of the TCP/IP headers inside an Ethernet frame
/// (`ether_extract_headers`); NULL members were not found.
pub struct EtherExtracted {
    /// `eh`.
    pub eh: *mut EtherHeader,
    /// `evh`.
    pub evh: *mut EtherVlanHeader,
    /// `ip4`.
    pub ip4: *mut Ip,
    /// `ip6`.
    pub ip6: *mut Ip6Hdr,
    /// `tcp` (`struct tcphdr *`, not ported).
    pub tcp: *mut u8,
    /// `udp` (`struct udphdr *`, not ported).
    pub udp: *mut u8,
    /// `iplen`.
    pub iplen: u32,
    /// `iphlen`.
    pub iphlen: u32,
    /// `tcphlen`.
    pub tcphlen: u32,
    /// `paylen`.
    pub paylen: u32,
}

impl EtherExtracted {
    /// `memset(ext, 0, sizeof(*ext))`.
    pub const fn new() -> Self {
        Self {
            eh: core::ptr::null_mut(),
            evh: core::ptr::null_mut(),
            ip4: core::ptr::null_mut(),
            ip6: core::ptr::null_mut(),
            tcp: core::ptr::null_mut(),
            udp: core::ptr::null_mut(),
            iplen: 0,
            iphlen: 0,
            tcphlen: 0,
            paylen: 0,
        }
    }
}

impl Default for EtherExtracted {
    fn default() -> Self {
        Self::new()
    }
}

/// `ETHER_MAP_IPV6_MULTICAST(ip6addr, enaddr)`: maps an IPv6 multicast address to an
/// Ethernet multicast address. The high-order 16 bits of the Ethernet address are statically
/// assigned, and the low-order 32 bits are taken from the low end of the IPv6 address.
pub const fn ether_map_ipv6_multicast(ip6addr: &In6Addr) -> [u8; ETHER_ADDR_LEN] {
    let a = &ip6addr.s6_addr;
    [0x33, 0x33, a[12], a[13], a[14], a[15]]
}

/// `arpintr()`: the kernel's ARP soft interrupt, under the net lock `bsd`'s caller holds
/// (`if_netisr`'s, or a test's `net_lock()`).
pub fn arpintr() {
    kernel::netinet::if_ether::arpintr(&crate::net::if_::net_held("arpintr"));
}

/// `arpproxy(in, rtableid)`: the kernel's, under the net lock `bsd`'s caller holds.
pub fn arpproxy(in_: InAddr, rtableid: u32) -> bool {
    kernel::netinet::if_ether::arpproxy(in_, rtableid, &crate::net::if_::net_held("arpproxy"))
}

// Sizes of the C structures.
const _: () = {
    assert!(core::mem::size_of::<EtherAddr>() == 6);
    assert!(core::mem::size_of::<EtherHeader>() == ETHER_HDR_LEN);
    assert!(core::mem::size_of::<EtherVlanHeader>() == ETHER_HDR_LEN + EVL_ENCAPLEN);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::ethertypes::ETHERTYPE_ARP;
    use crate::reftest::{assert_complete, assert_defines};
    use crate::sys::endian::htons;

    #[test]
    fn frame_layout_and_predicates() {
        let eh = EtherHeader {
            ether_dhost: [0xff; 6],
            ether_shost: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
            ether_type: htons(ETHERTYPE_ARP),
        };
        let mut bytes = [0u8; 14];
        assert!(hal::pod::put(&mut bytes, 0, &eh));
        assert_eq!(&bytes[12..], &[0x08, 0x06]);
        assert!(ether_is_broadcast(&eh.ether_dhost));
        assert!(ether_is_multicast(&eh.ether_dhost));
        assert!(!ether_is_multicast(&eh.ether_shost));
        assert!(ether_is_anyaddr(&[0; 6]));
        assert!(ether_is_eq(
            &eh.ether_shost,
            &[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
        ));
        assert!(eth64_is_multicast(0x0100_5e00_0001));
        assert!(eth64_is_broadcast(0xffff_ffff_ffff));
        assert!(eth64_is_8021_rsvd(0x0180_c200_000e));
        assert_eq!(ETHERMTU, 1500);
        assert_eq!(ETHERMIN, 46);
        assert_eq!(evl_vlanoftag(0x6064), 0x064);
        assert_eq!(evl_prioftag(0x6064), 3);

        let mut ip6 = In6Addr::default();
        ip6.s6_addr[12..].copy_from_slice(&[0xff, 0x00, 0x00, 0x01]);
        assert_eq!(
            ether_map_ipv6_multicast(&ip6),
            [0x33, 0x33, 0xff, 0x00, 0x00, 0x01]
        );
        assert_eq!(core::mem::offset_of!(EtherArp, arp_sha), 8);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/if_ether.h");
        let ether = assert_defines!(defs;
        ETHER_ADDR_LEN, ETHER_TYPE_LEN, ETHER_CRC_LEN, ETHER_HDR_LEN, ETHER_MIN_LEN,
        ETHER_MAX_LEN, ETHER_MAX_DIX_LEN, ETHER_VLAN_ENCAP_LEN, ETHER_ALIGN,
        ETHER_MAX_HARDMTU_LEN, ETHER_CRC_POLY_LE, ETHER_CRC_POLY_BE);
        assert_complete(&defs, "ETHER_", &ether);
        let evl = assert_defines!(defs;
        EVL_VLID_MASK, EVL_VLID_NULL, EVL_VLID_MIN, EVL_VLID_MAX, EVL_PRIO_MAX, EVL_PRIO_BITS,
        EVL_ENCAPLEN);
        assert_complete(&defs, "EVL_", &evl);
        let eth64 = assert_defines!(defs; ETH64_8021_RSVD_PREFIX, ETH64_8021_RSVD_MASK);
        assert_complete(&defs, "ETH64_", &eth64);
        assert_defines!(defs; ETHERMTU, ETHERMIN);
    }
}
/* </TESTS> */
