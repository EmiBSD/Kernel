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
//! Ethernet's userland ABI, `<netinet/if_ether.h>` outside `#ifdef _KERNEL`: the frame's
//! lengths and limits, `struct ether_addr`, `struct ether_header`, `struct
//! ether_vlan_header`, the 802.1Q tag fields, the address tests on six bytes and on the
//! 48-bit integer form (`ETH64_*`), and the CRC polynomials. The kernel's bpf and Ethernet
//! layer (`kernel::net`) and `bsd`'s drivers share them.
//!
//! Upstream: sys/netinet/if_ether.h @ 3ce1f3f79392
//! Upstream: sys/netinet/if_ether.c @ 3ce1f3f79392
//! LZ: sys/netinet/if_ether.rs@3c62ede68802
//!
//! The headers are the frames as they are on the wire: `ether_type`, `evl_encap_proto`,
//! `evl_tag` and `evl_proto` hold network-order values (`u16::from_be(eh.ether_type) ==
//! ETHERTYPE_IP`), as in C. An Ethernet header in a received packet starts two bytes past a
//! word ([`ETHER_ALIGN`]): the images are of alignment 2, which keeps that start aligned for
//! them.
//!
//! ## Deviations
//! - `ETHER_IS_MULTICAST(addr)`, `ETHER_IS_BROADCAST`, `ETHER_IS_ANYADDR`, `ETHER_IS_EQ`,
//!   `EVL_VLANOFTAG`, `EVL_PRIOFTAG` and the `ETH64_IS_*` tests are `fn`s with the lowercase
//!   names over six-byte arrays and `u64`s.
//!
//! ## Redesign
//! - LZ's `sys/netinet/if_ether.rs` held the header beside if_ether.c's ARP; the userland
//!   part of the header is here, in `hal` (N6.3c), so `kernel::net`'s bpf and Ethernet layer
//!   name it; LZ's `#[repr(C)]` images are `abi_pod!`s with `const` size asserts. The
//!   `#ifdef _KERNEL` part (`struct arpcom`, `struct ether_multi`, `ETHER_MAP_IP_MULTICAST`,
//!   `etherbroadcastaddr`, ...) is the kernel's Ethernet layer's (N6.3c2).
//! - N6.4b: the ARP part joins it with the kernel's ARP, its first user here: `struct
//!   ether_arp` (28 bytes, `struct arphdr` of `hal::sys::net_if_arp` first, with the C's
//!   `arp_hrd`.. `#define`s as accessors), `struct sockaddr_inarp` (16), `SIN_PROXY`,
//!   `RTF_USETRAILERS` and `RTF_PERMANENT_ARP` (`arp(8)` reads them); `bsd`'s copies went.

use core::mem::size_of;

use crate::sys::net_if_arp::Arphdr;
use crate::sys::net_route::{RTF_PROTO1, RTF_PROTO3};
use crate::sys::netinet_in::InAddr;

/// `ETHER_ADDR_LEN`: an Ethernet address's length.
pub const ETHER_ADDR_LEN: usize = 6;
/// `ETHER_TYPE_LEN`: the type field's length.
pub const ETHER_TYPE_LEN: usize = 2;
/// `ETHER_CRC_LEN`: the CRC's length.
pub const ETHER_CRC_LEN: usize = 4;
/// `ETHER_HDR_LEN`: the header's length.
pub const ETHER_HDR_LEN: usize = (ETHER_ADDR_LEN * 2) + ETHER_TYPE_LEN;
/// `ETHER_MIN_LEN`: the shortest frame, CRC included.
pub const ETHER_MIN_LEN: usize = 64;
/// `ETHER_MAX_LEN`: the longest frame, CRC included.
pub const ETHER_MAX_LEN: usize = 1518;
/// `ETHER_MAX_DIX_LEN`: the longest DIX frame.
pub const ETHER_MAX_DIX_LEN: usize = 1536;

/// `ETHER_VLAN_ENCAP_LEN`: the length of 802.1Q's encapsulation.
pub const ETHER_VLAN_ENCAP_LEN: usize = 4;

/// `ETHER_ALIGN`: what a driver `m_adj`s off a receive buffer so the IP header past the
/// 14-byte Ethernet header lands on a word.
pub const ETHER_ALIGN: usize = 2;

/// `ETHER_MAX_HARDMTU_LEN`: the longest Ethernet length supported, with room for
/// encapsulation.
pub const ETHER_MAX_HARDMTU_LEN: usize = 65435;

/// `EVL_VLID_MASK`: a tag's VLAN id bits.
pub const EVL_VLID_MASK: u16 = 0xFFF;
/// `EVL_VLID_NULL`: the null VLAN id (0x000 and 0xfff are reserved).
pub const EVL_VLID_NULL: u16 = 0x000;
/// `EVL_VLID_MIN`: the lowest VLAN id.
pub const EVL_VLID_MIN: u16 = 0x001;
/// `EVL_VLID_MAX`: the highest VLAN id.
pub const EVL_VLID_MAX: u16 = 0xFFE;

/// `EVL_PRIO_MAX`: a tag's highest priority.
pub const EVL_PRIO_MAX: u16 = 7;
/// `EVL_PRIO_BITS`: where a tag's priority starts.
pub const EVL_PRIO_BITS: u16 = 13;

/// `EVL_ENCAPLEN`: the encapsulation's length in octets.
pub const EVL_ENCAPLEN: usize = 4;

/// `ETH64_8021_RSVD_PREFIX`: the IEEE 802.1 reserved multicast range, as a 48-bit integer.
pub const ETH64_8021_RSVD_PREFIX: u64 = 0x0180_c200_0000;
/// `ETH64_8021_RSVD_MASK`.
pub const ETH64_8021_RSVD_MASK: u64 = 0xffff_ffff_fff0;

/// `ETHERMTU`: the Ethernet MTU.
pub const ETHERMTU: usize = ETHER_MAX_LEN - ETHER_HDR_LEN - ETHER_CRC_LEN;
/// `ETHERMIN`: the shortest payload.
pub const ETHERMIN: usize = ETHER_MIN_LEN - ETHER_HDR_LEN - ETHER_CRC_LEN;

/// `ETHER_CRC_POLY_LE`: the CRC32 polynomial, little-endian form.
pub const ETHER_CRC_POLY_LE: u32 = 0xedb8_8320;
/// `ETHER_CRC_POLY_BE`: the big-endian form.
pub const ETHER_CRC_POLY_BE: u32 = 0x04c1_1db6;

/// `struct ether_addr`: an Ethernet address, six octets.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EtherAddr {
    /// The octets.
    pub ether_addr_octet: [u8; ETHER_ADDR_LEN],
}

crate::abi_pod!(EtherAddr {
    ether_addr_octet: [u8; ETHER_ADDR_LEN],
});

/// `struct ether_header`: the Ethernet header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EtherHeader {
    /// The destination address.
    pub ether_dhost: [u8; ETHER_ADDR_LEN],
    /// The source address.
    pub ether_shost: [u8; ETHER_ADDR_LEN],
    /// The type, network order.
    pub ether_type: u16,
}

crate::abi_pod!(EtherHeader {
    ether_dhost: [u8; ETHER_ADDR_LEN],
    ether_shost: [u8; ETHER_ADDR_LEN],
    ether_type: u16,
});

/// `struct ether_vlan_header`: an 802.1Q tagged Ethernet header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EtherVlanHeader {
    /// The destination address.
    pub evl_dhost: [u8; ETHER_ADDR_LEN],
    /// The source address.
    pub evl_shost: [u8; ETHER_ADDR_LEN],
    /// `ETHERTYPE_VLAN`, network order.
    pub evl_encap_proto: u16,
    /// The priority and the VLAN id, network order.
    pub evl_tag: u16,
    /// The payload's type, network order.
    pub evl_proto: u16,
}

crate::abi_pod!(EtherVlanHeader {
    evl_dhost: [u8; ETHER_ADDR_LEN],
    evl_shost: [u8; ETHER_ADDR_LEN],
    evl_encap_proto: u16,
    evl_tag: u16,
    evl_proto: u16,
});

/// `struct ether_arp`: Ethernet Address Resolution Protocol. See RFC 826 for protocol
/// description; this structure is adapted to resolving internet addresses. Field names used
/// correspond to RFC 826.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EtherArp {
    /// Fixed-size header.
    pub ea_hdr: Arphdr,
    /// Sender hardware address.
    pub arp_sha: [u8; ETHER_ADDR_LEN],
    /// Sender protocol address.
    pub arp_spa: [u8; 4],
    /// Target hardware address.
    pub arp_tha: [u8; ETHER_ADDR_LEN],
    /// Target protocol address.
    pub arp_tpa: [u8; 4],
}

crate::abi_pod!(EtherArp {
    ea_hdr: Arphdr,
    arp_sha: [u8; ETHER_ADDR_LEN],
    arp_spa: [u8; 4],
    arp_tha: [u8; ETHER_ADDR_LEN],
    arp_tpa: [u8; 4],
});

impl EtherArp {
    /// `arp_hrd`: format of hardware address, network order.
    pub const fn arp_hrd(&self) -> u16 {
        self.ea_hdr.ar_hrd
    }

    /// `arp_pro`: format of protocol address, network order.
    pub const fn arp_pro(&self) -> u16 {
        self.ea_hdr.ar_pro
    }

    /// `arp_hln`: length of hardware address.
    pub const fn arp_hln(&self) -> u8 {
        self.ea_hdr.ar_hln
    }

    /// `arp_pln`: length of protocol address.
    pub const fn arp_pln(&self) -> u8 {
        self.ea_hdr.ar_pln
    }

    /// `arp_op`: operation, network order.
    pub const fn arp_op(&self) -> u16 {
        self.ea_hdr.ar_op
    }
}

/// `struct sockaddr_inarp`: the socket address of an ARP entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SockaddrInarp {
    /// Total length.
    pub sin_len: u8,
    /// `AF_INET`.
    pub sin_family: u8,
    /// Port.
    pub sin_port: u16,
    /// Address.
    pub sin_addr: InAddr,
    /// Source address.
    pub sin_srcaddr: InAddr,
    /// Type of service.
    pub sin_tos: u16,
    /// `SIN_PROXY`.
    pub sin_other: u16,
}

crate::abi_pod!(SockaddrInarp {
    sin_len: u8,
    sin_family: u8,
    sin_port: u16,
    sin_addr: InAddr,
    sin_srcaddr: InAddr,
    sin_tos: u16,
    sin_other: u16,
});

/// `sockaddr_inarp` flag: proxy entry.
pub const SIN_PROXY: u16 = 1;

/// Use trailers (IP and ethernet specific routing flag).
pub const RTF_USETRAILERS: u32 = RTF_PROTO1;
/// Only manual overwrite of entry.
pub const RTF_PERMANENT_ARP: u32 = RTF_PROTO3;

/// `EVL_VLANOFTAG(tag)`: the VLAN id of a (host-order) tag.
pub const fn evl_vlanoftag(tag: u16) -> u16 {
    tag & EVL_VLID_MASK
}

/// `EVL_PRIOFTAG(tag)`: the priority of a (host-order) tag.
pub const fn evl_prioftag(tag: u16) -> u16 {
    (tag >> EVL_PRIO_BITS) & 7
}

/// `ETHER_IS_MULTICAST(addr)`: whether the address is multicast (or broadcast).
pub const fn ether_is_multicast(addr: &[u8; ETHER_ADDR_LEN]) -> bool {
    addr[0] & 0x01 != 0
}

/// `ETHER_IS_BROADCAST(addr)`: whether the address is ff:ff:ff:ff:ff:ff.
pub const fn ether_is_broadcast(addr: &[u8; ETHER_ADDR_LEN]) -> bool {
    (addr[0] & addr[1] & addr[2] & addr[3] & addr[4] & addr[5]) == 0xff
}

/// `ETHER_IS_ANYADDR(addr)`: whether the address is 00:00:00:00:00:00.
pub const fn ether_is_anyaddr(addr: &[u8; ETHER_ADDR_LEN]) -> bool {
    (addr[0] | addr[1] | addr[2] | addr[3] | addr[4] | addr[5]) == 0x00
}

/// `ETHER_IS_EQ(a1, a2)`: whether the two addresses are equal.
pub fn ether_is_eq(a1: &[u8; ETHER_ADDR_LEN], a2: &[u8; ETHER_ADDR_LEN]) -> bool {
    a1 == a2
}

/// `ETH64_IS_MULTICAST(e64)`: whether the address, as a 48-bit integer, is multicast.
pub const fn eth64_is_multicast(e64: u64) -> bool {
    e64 & 0x0100_0000_0000 != 0
}

/// `ETH64_IS_BROADCAST(e64)`.
pub const fn eth64_is_broadcast(e64: u64) -> bool {
    e64 == 0xffff_ffff_ffff
}

/// `ETH64_IS_ANYADDR(e64)`.
pub const fn eth64_is_anyaddr(e64: u64) -> bool {
    e64 == 0x0000_0000_0000
}

/// `ETH64_IS_8021_RSVD(e64)`: whether the address is in the IEEE 802.1 reserved range.
pub const fn eth64_is_8021_rsvd(e64: u64) -> bool {
    (e64 & ETH64_8021_RSVD_MASK) == ETH64_8021_RSVD_PREFIX
}

const _: () = {
    assert!(size_of::<EtherAddr>() == 6);
    assert!(size_of::<EtherHeader>() == 14);
    assert!(align_of::<EtherHeader>() == 2);
    assert!(size_of::<EtherVlanHeader>() == 18);
    assert!(size_of::<EtherArp>() == 28);
    assert!(core::mem::offset_of!(EtherArp, arp_sha) == 8);
    assert!(size_of::<SockaddrInarp>() == 16);
    assert!(ETHER_HDR_LEN == 14);
    assert!(ETHERMTU == 1500);
    assert!(ETHERMIN == 46);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_tests_read_the_c_bits() {
        let bcast = [0xff; ETHER_ADDR_LEN];
        let any = [0u8; ETHER_ADDR_LEN];
        let mcast = [0x01, 0x00, 0x5e, 0x00, 0x00, 0x01];
        let ucast = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        assert!(ether_is_broadcast(&bcast) && ether_is_multicast(&bcast));
        assert!(ether_is_anyaddr(&any) && !ether_is_multicast(&any));
        assert!(ether_is_multicast(&mcast) && !ether_is_broadcast(&mcast));
        assert!(!ether_is_multicast(&ucast) && !ether_is_anyaddr(&ucast));
        assert!(ether_is_eq(&ucast, &[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]));
        assert!(eth64_is_broadcast(0xffff_ffff_ffff));
        assert!(eth64_is_multicast(0x0100_5e00_0001) && !eth64_is_multicast(0x5254_0012_3456));
        assert!(eth64_is_anyaddr(0));
        assert!(eth64_is_8021_rsvd(0x0180_c200_000e) && !eth64_is_8021_rsvd(0x0180_c200_0010));
        // 802.1Q: priority 5, VLAN 100.
        let tag = (5 << EVL_PRIO_BITS) | 100;
        assert_eq!((evl_prioftag(tag), evl_vlanoftag(tag)), (5, 100));
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn lengths_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/if_ether.h");
        let _ = crate::reftest::assert_defines!(defs;
            ETHER_ADDR_LEN, ETHER_TYPE_LEN, ETHER_CRC_LEN, ETHER_MIN_LEN, ETHER_MAX_LEN,
            ETHER_MAX_DIX_LEN, ETHER_VLAN_ENCAP_LEN, ETHER_ALIGN, ETHER_MAX_HARDMTU_LEN,
            EVL_VLID_MASK, EVL_VLID_NULL, EVL_VLID_MIN, EVL_VLID_MAX, EVL_PRIO_MAX, EVL_PRIO_BITS,
            EVL_ENCAPLEN, ETHER_CRC_POLY_LE, ETHER_CRC_POLY_BE,
        );
        let _ = crate::reftest::assert_defines!(defs; ETHERMTU, ETHERMIN, SIN_PROXY);
        assert_eq!(defs["RTF_USETRAILERS"], "RTF_PROTO1");
        assert_eq!(defs["RTF_PERMANENT_ARP"], "RTF_PROTO3");
    }
}
/* </TESTS> */
