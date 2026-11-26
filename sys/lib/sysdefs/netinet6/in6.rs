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
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
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
 * Copyright (c) 1982, 1986, 1990, 1993
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
 *	@(#)in.h	8.3 (Berkeley) 1/3/94
 */

/*
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
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
 * Copyright (c) 1982, 1986, 1991, 1993
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
 *	@(#)in.c	8.2 (Berkeley) 11/15/93
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6's userland ABI, `<netinet6/in6.h>`: the address and socket address
//! (`struct in6_addr`, `struct sockaddr_in6`), the `[gs]etsockopt` names and values of the
//! IPv6 level and their structures (`struct ipv6_mreq`, `struct in6_pktinfo`, `struct
//! ip6_mtuinfo`), the well-known addresses and masks (`in6.c`'s objects), the address tests
//! (`IN6_IS_ADDR_*`, `IN6_ARE_ADDR_EQUAL`, `IN6_IS_SCOPE_*`) and the `net.inet6` sysctl
//! names. The kernel's IPv6 (`in6.c`) is `kernel::netinet6::in6`'s.
//!
//! Upstream: sys/netinet6/in6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/in6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/in6.rs@3c62ede68802
//!
//! Byte order: as in the C kernel, an [`In6Addr`] holds the address bytes as they are on the
//! wire, and the 16- and 32-bit views (`s6_addr16`, `s6_addr32`) are the native-endian
//! readings of those bytes, so `__IPV6_ADDR_INT32_ONE` is `htonl(1)` and a comparison of
//! words is a comparison of the network-order words, as in C.
//!
//! ## Deviations
//! - `struct in6_addr`'s `__u6_addr` union is the byte array `s6_addr`; `s6_addr8`,
//!   `s6_addr16` and `s6_addr32` are `const fn` accessors (and setters) reading the bytes in
//!   native order. The structure has alignment 1 (4 in C) so that it can sit in the
//!   `__packed` protocol headers; every structure holding one keeps the C size and offsets
//!   (asserted below).
//! - The address macros (`IN6_IS_ADDR_*`, `IN6_ARE_ADDR_EQUAL`, `IN6_IS_SCOPE_*`,
//!   `__IPV6_ADDR_MC_SCOPE`) are `const fn`s of the same name in lower case.
//! - The `*_INIT` initializers and the `in6addr_*`/`in6mask*` objects (defined in `in6.c`)
//!   are `pub const` values: `IN6ADDR_ANY_INIT` and `IN6ADDR_ANY` are the same address. The
//!   kernel masks `IN6MASK0`..`IN6MASK128` stand for both the C macros and the `in6mask*`
//!   objects, and `sa6_any` is [`SA6_ANY`].
//! - `CTL_IPV6PROTO_NAMES` and `IPV6CTL_NAMES` are `Ctlname` tables.
//! - The userland part (`SIN6_LEN`, `socklen_t`, the `inet6_opt_*` and `inet6_rth_*`
//!   prototypes of libc) is not kernel material. `__KAME__` is not mirrored.
//!
//! ## Redesign
//! - N6.3c2: `struct in6_addr`, `struct sockaddr_in6` and `IN6_IS_ADDR_UNSPECIFIED` copied
//!   from LZ's `bsd` `sys/netinet6/in6.rs` for the kernel's Ethernet layer. N6.5a: the whole
//!   userland half of the header moved here from LZ's module, which re-exports it, so one
//!   `In6Addr` and one `SockaddrIn6` exist (N6.4c1's `Addr6` and `Hdr6` traits keep reading
//!   both images until `bsd`'s last copy goes); `struct ipv6_mreq`, `struct in6_pktinfo` and
//!   `struct ip6_mtuinfo` are `abi_pod!` images (copied in and out of option buffers by
//!   `hal::mem::pod`); the address tests and the views are `#[inline]` (read per packet across the
//!   crates, N6.4a's R12).

use core::mem::size_of;

use crate::base::endian::{htonl, htons};
use crate::base::types::{InPort, SaFamily};
use crate::net::socket::AF_INET6;
use crate::netinet::in_::IPPROTO_DIVERT;
use crate::proc::sysctl::{CTLTYPE_INT, CTLTYPE_NODE, CTLTYPE_STRUCT, Ctlname};

/// Buffer length for strings containing printable IPv6 addresses.
pub const INET6_ADDRSTRLEN: usize = 46;

// Macros started with IPV6_ADDR is KAME local: network-order words and half-words.

/// `__IPV6_ADDR_INT32_ONE`: 1 as a network-order word.
pub const __IPV6_ADDR_INT32_ONE: u32 = htonl(1);
/// `__IPV6_ADDR_INT32_TWO`: 2 as a network-order word.
pub const __IPV6_ADDR_INT32_TWO: u32 = htonl(2);
/// `__IPV6_ADDR_INT32_MNL`: ff01:: (node-local multicast), first word.
pub const __IPV6_ADDR_INT32_MNL: u32 = htonl(0xff01_0000);
/// `__IPV6_ADDR_INT32_MLL`: ff02:: (link-local multicast), first word.
pub const __IPV6_ADDR_INT32_MLL: u32 = htonl(0xff02_0000);
/// `__IPV6_ADDR_INT32_SMP`: the third word of a v4-mapped address.
pub const __IPV6_ADDR_INT32_SMP: u32 = htonl(0x0000_ffff);
/// `__IPV6_ADDR_INT16_ULL`: fe80 (unicast link-local).
pub const __IPV6_ADDR_INT16_ULL: u16 = htons(0xfe80);
/// `__IPV6_ADDR_INT16_USL`: fec0 (unicast site-local).
pub const __IPV6_ADDR_INT16_USL: u16 = htons(0xfec0);
/// `__IPV6_ADDR_INT16_MLL`: ff02 (multicast link-local).
pub const __IPV6_ADDR_INT16_MLL: u16 = htons(0xff02);

/// `__IPV6_ADDR_SCOPE_NODELOCAL`.
pub const __IPV6_ADDR_SCOPE_NODELOCAL: u8 = 0x01;
/// `__IPV6_ADDR_SCOPE_INTFACELOCAL`.
pub const __IPV6_ADDR_SCOPE_INTFACELOCAL: u8 = 0x01;
/// `__IPV6_ADDR_SCOPE_LINKLOCAL`.
pub const __IPV6_ADDR_SCOPE_LINKLOCAL: u8 = 0x02;
/// `__IPV6_ADDR_SCOPE_SITELOCAL`.
pub const __IPV6_ADDR_SCOPE_SITELOCAL: u8 = 0x05;
/// `__IPV6_ADDR_SCOPE_ORGLOCAL`: just used in this file.
pub const __IPV6_ADDR_SCOPE_ORGLOCAL: u8 = 0x08;
/// `__IPV6_ADDR_SCOPE_GLOBAL`.
pub const __IPV6_ADDR_SCOPE_GLOBAL: u8 = 0x0e;

// Options for use with [gs]etsockopt at the IPV6 level. First word of comment is data type;
// bool is stored in int.

/// int; IP6 hops.
pub const IPV6_UNICAST_HOPS: i32 = 4;
/// u_int; set/get IP6 multicast i/f.
pub const IPV6_MULTICAST_IF: i32 = 9;
/// u_int; set/get IP6 multicast hops.
pub const IPV6_MULTICAST_HOPS: i32 = 10;
/// u_int; set/get IP6 multicast loopback.
pub const IPV6_MULTICAST_LOOP: i32 = 11;
/// ip6_mreq; join a group membership.
pub const IPV6_JOIN_GROUP: i32 = 12;
/// ip6_mreq; leave a group membership.
pub const IPV6_LEAVE_GROUP: i32 = 13;
/// int; range to choose for unspec port.
pub const IPV6_PORTRANGE: i32 = 14;
/// icmp6_filter; icmp6 filter.
pub const ICMP6_FILTER: i32 = 18;

/// int; checksum offset for raw socket.
pub const IPV6_CHECKSUM: i32 = 26;
/// bool; make `AF_INET6` sockets v6 only.
pub const IPV6_V6ONLY: i32 = 27;

// new socket options introduced in RFC3542

/// ip6_dest; send dst option before rthdr.
pub const IPV6_RTHDRDSTOPTS: i32 = 35;

/// bool; recv if, dst addr.
pub const IPV6_RECVPKTINFO: i32 = 36;
/// bool; recv hop limit.
pub const IPV6_RECVHOPLIMIT: i32 = 37;
/// bool; recv routing header.
pub const IPV6_RECVRTHDR: i32 = 38;
/// bool; recv hop-by-hop option.
pub const IPV6_RECVHOPOPTS: i32 = 39;
/// bool; recv dst option after rthdr.
pub const IPV6_RECVDSTOPTS: i32 = 40;

/// bool; send packets at the minimum MTU.
pub const IPV6_USE_MIN_MTU: i32 = 42;
/// bool; notify an according MTU.
pub const IPV6_RECVPATHMTU: i32 = 43;

/// mtuinfo; get the current path MTU (sopt), 4 bytes int; MTU notification (cmsg).
pub const IPV6_PATHMTU: i32 = 44;

// More new socket options introduced in RFC3542

/// in6_pktinfo; send if, src addr.
pub const IPV6_PKTINFO: i32 = 46;
/// int; send hop limit.
pub const IPV6_HOPLIMIT: i32 = 47;
/// sockaddr; next hop addr.
pub const IPV6_NEXTHOP: i32 = 48;
/// ip6_hbh; send hop-by-hop option.
pub const IPV6_HOPOPTS: i32 = 49;
/// ip6_dest; send dst option before rthdr.
pub const IPV6_DSTOPTS: i32 = 50;
/// ip6_rthdr; send routing header.
pub const IPV6_RTHDR: i32 = 51;

/// int; authentication used.
pub const IPV6_AUTH_LEVEL: i32 = 53;
/// int; transport encryption.
pub const IPV6_ESP_TRANS_LEVEL: i32 = 54;
/// int; full-packet encryption.
pub const IPV6_ESP_NETWORK_LEVEL: i32 = 55;
/// Set the outbound SA for a socket.
pub const IPSEC6_OUTSA: i32 = 56;
/// bool; recv traffic class values.
pub const IPV6_RECVTCLASS: i32 = 57;

/// bool; attach flowlabel automagically.
pub const IPV6_AUTOFLOWLABEL: i32 = 59;
/// int; compression.
pub const IPV6_IPCOMP_LEVEL: i32 = 60;

/// int; send traffic class value.
pub const IPV6_TCLASS: i32 = 61;
/// bool; disable IPv6 fragmentation.
pub const IPV6_DONTFRAG: i32 = 62;
/// bool; using PIPEX.
pub const IPV6_PIPEX: i32 = 63;

/// bool; receive IP dst port w/dgram.
pub const IPV6_RECVDSTPORT: i32 = 64;
/// int; minimum recv hop limit.
pub const IPV6_MINHOPCOUNT: i32 = 65;

/// int; routing table, see `SO_RTABLE`.
pub const IPV6_RTABLE: i32 = 0x1021;

/// This hop need not be a neighbor.
pub const IPV6_RTHDR_LOOSE: i32 = 0;
/// IPv6 routing header type 0.
pub const IPV6_RTHDR_TYPE_0: i32 = 0;

// Defaults and limits for options

/// Normally limit m'casts to 1 hop.
pub const IPV6_DEFAULT_MULTICAST_HOPS: i32 = 1;
/// Normally hear sends if a member.
pub const IPV6_DEFAULT_MULTICAST_LOOP: i32 = 1;

// Argument for IPV6_PORTRANGE: which range to search when port is unspecified at bind() or
// connect().

/// Default range.
pub const IPV6_PORTRANGE_DEFAULT: i32 = 0;
/// "high" - request firewall bypass.
pub const IPV6_PORTRANGE_HIGH: i32 = 1;
/// "low" - vouchsafe security.
pub const IPV6_PORTRANGE_LOW: i32 = 2;

// Definitions for inet6 sysctl operations. Third level is protocol number; fourth level is
// desired variable within that protocol.

/// Don't list to `IPV6PROTO_MAX`.
pub const IPV6PROTO_MAXID: i32 = IPPROTO_DIVERT + 1;

/// `CTL_IPV6PROTO_NAMES`: the protocols under `net.inet6`.
pub const CTL_IPV6PROTO_NAMES: [Ctlname; IPV6PROTO_MAXID as usize] = {
    let mut n = [Ctlname::NONE; IPV6PROTO_MAXID as usize];
    n[6] = Ctlname::new(b"tcp6", CTLTYPE_NODE);
    n[17] = Ctlname::new(b"udp6", CTLTYPE_NODE);
    n[41] = Ctlname::new(b"ip6", CTLTYPE_NODE);
    n[51] = Ctlname::new(b"ipsec6", CTLTYPE_NODE);
    n[58] = Ctlname::new(b"icmp6", CTLTYPE_NODE);
    n[258] = Ctlname::new(b"divert", CTLTYPE_NODE);
    n
};

// Names for IP sysctl objects

/// Act as router.
pub const IPV6CTL_FORWARDING: i32 = 1;
/// May send redirects when forwarding.
pub const IPV6CTL_SENDREDIRECTS: i32 = 2;
/// Default Hop-Limit.
pub const IPV6CTL_DEFHLIM: i32 = 3;
/// Forward source-routed dgrams.
pub const IPV6CTL_FORWSRCRT: i32 = 5;
/// Stats.
pub const IPV6CTL_STATS: i32 = 6;
/// Multicast forwarding stats.
pub const IPV6CTL_MRTSTATS: i32 = 7;
/// Multicast routing protocol.
pub const IPV6CTL_MRTPROTO: i32 = 8;
/// Max packets reassembly queue.
pub const IPV6CTL_MAXFRAGPACKETS: i32 = 9;
/// Verify source route and intf.
pub const IPV6CTL_SOURCECHECK: i32 = 10;
/// Minimum logging interval.
pub const IPV6CTL_SOURCECHECK_LOGINT: i32 = 11;
/// `IPV6CTL_ACCEPT_RTADV`.
pub const IPV6CTL_ACCEPT_RTADV: i32 = 12;
/// `IPV6CTL_LOG_INTERVAL`.
pub const IPV6CTL_LOG_INTERVAL: i32 = 14;
/// `IPV6CTL_HDRNESTLIMIT`.
pub const IPV6CTL_HDRNESTLIMIT: i32 = 15;
/// `IPV6CTL_DAD_COUNT`.
pub const IPV6CTL_DAD_COUNT: i32 = 16;
/// `IPV6CTL_AUTO_FLOWLABEL`.
pub const IPV6CTL_AUTO_FLOWLABEL: i32 = 17;
/// `IPV6CTL_DEFMCASTHLIM`.
pub const IPV6CTL_DEFMCASTHLIM: i32 = 18;
// 24 to 40: reserved
/// Max fragments.
pub const IPV6CTL_MAXFRAGS: i32 = 41;
/// `IPV6CTL_MFORWARDING`.
pub const IPV6CTL_MFORWARDING: i32 = 42;
/// `IPV6CTL_MULTIPATH`.
pub const IPV6CTL_MULTIPATH: i32 = 43;
/// Path MTU discovery for multicast.
pub const IPV6CTL_MCAST_PMTU: i32 = 44;
/// `IPV6CTL_NEIGHBORGCTHRESH`.
pub const IPV6CTL_NEIGHBORGCTHRESH: i32 = 45;
/// `IPV6CTL_MAXDYNROUTES`.
pub const IPV6CTL_MAXDYNROUTES: i32 = 48;
/// `IPV6CTL_DAD_PENDING`.
pub const IPV6CTL_DAD_PENDING: i32 = 49;
/// `IPV6CTL_MTUDISCTIMEOUT`.
pub const IPV6CTL_MTUDISCTIMEOUT: i32 = 50;
/// `IPV6CTL_IFQUEUE`.
pub const IPV6CTL_IFQUEUE: i32 = 51;
/// `IPV6CTL_MRTMIF`.
pub const IPV6CTL_MRTMIF: i32 = 52;
/// `IPV6CTL_MRTMFC`.
pub const IPV6CTL_MRTMFC: i32 = 53;
/// `IPV6CTL_MAXID`.
pub const IPV6CTL_MAXID: i32 = 54;

/// `IPV6CTL_NAMES`: the names under `net.inet6.ip6`.
pub const IPV6CTL_NAMES: [Ctlname; IPV6CTL_MAXID as usize] = {
    let mut n = [Ctlname::NONE; IPV6CTL_MAXID as usize];
    n[1] = Ctlname::new(b"forwarding", CTLTYPE_INT);
    n[2] = Ctlname::new(b"redirect", CTLTYPE_INT);
    n[3] = Ctlname::new(b"hlim", CTLTYPE_INT);
    n[5] = Ctlname::new(b"forwsrcrt", CTLTYPE_INT);
    n[8] = Ctlname::new(b"mrtproto", CTLTYPE_INT);
    n[9] = Ctlname::new(b"maxfragpackets", CTLTYPE_INT);
    n[10] = Ctlname::new(b"sourcecheck", CTLTYPE_INT);
    n[11] = Ctlname::new(b"sourcecheck_logint", CTLTYPE_INT);
    n[15] = Ctlname::new(b"hdrnestlimit", CTLTYPE_INT);
    n[16] = Ctlname::new(b"dad_count", CTLTYPE_INT);
    n[18] = Ctlname::new(b"defmcasthlim", CTLTYPE_INT);
    n[41] = Ctlname::new(b"maxfrags", CTLTYPE_INT);
    n[42] = Ctlname::new(b"mforwarding", CTLTYPE_INT);
    n[43] = Ctlname::new(b"multipath", CTLTYPE_INT);
    n[44] = Ctlname::new(b"multicast_mtudisc", CTLTYPE_INT);
    n[45] = Ctlname::new(b"neighborgcthresh", CTLTYPE_INT);
    n[48] = Ctlname::new(b"maxdynroutes", CTLTYPE_INT);
    n[49] = Ctlname::new(b"dad_pending", CTLTYPE_INT);
    n[50] = Ctlname::new(b"mtudisctimeout", CTLTYPE_INT);
    n[51] = Ctlname::new(b"ifq", CTLTYPE_NODE);
    n[52] = Ctlname::new(b"mrtmif", CTLTYPE_STRUCT);
    n[53] = Ctlname::new(b"mrtmfc", CTLTYPE_STRUCT);
    n
};

/// `struct in6_addr`: an IPv6 address, its 16 bytes in network order (see the module's
/// deviations for the views).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct In6Addr {
    /// `s6_addr`: the 128-bit address.
    pub s6_addr: [u8; 16],
}
crate::abi_pod!(In6Addr { s6_addr: [u8; 16] });

impl In6Addr {
    /// The address of the 16 bytes `b`.
    pub const fn new(b: [u8; 16]) -> Self {
        Self { s6_addr: b }
    }

    /// The address whose `s6_addr32` words are `w` (network-order words).
    pub const fn from_s6_addr32(w: [u32; 4]) -> Self {
        let mut a = Self { s6_addr: [0; 16] };
        let mut i = 0;
        while i < 4 {
            a.set_s6_addr32(i, w[i]);
            i += 1;
        }
        a
    }

    /// `s6_addr8[i]`.
    #[inline]
    pub const fn s6_addr8(&self, i: usize) -> u8 {
        self.s6_addr[i]
    }

    /// `s6_addr16[i]`: bytes `2i`, `2i + 1` read in native order.
    #[inline]
    pub const fn s6_addr16(&self, i: usize) -> u16 {
        u16::from_ne_bytes([self.s6_addr[2 * i], self.s6_addr[2 * i + 1]])
    }

    /// `s6_addr16[i] = v`.
    #[inline]
    pub const fn set_s6_addr16(&mut self, i: usize, v: u16) {
        let b = v.to_ne_bytes();
        self.s6_addr[2 * i] = b[0];
        self.s6_addr[2 * i + 1] = b[1];
    }

    /// `s6_addr32[i]`: bytes `4i` to `4i + 3` read in native order.
    #[inline]
    pub const fn s6_addr32(&self, i: usize) -> u32 {
        let a = &self.s6_addr;
        u32::from_ne_bytes([a[4 * i], a[4 * i + 1], a[4 * i + 2], a[4 * i + 3]])
    }

    /// `s6_addr32[i] = v`.
    #[inline]
    pub const fn set_s6_addr32(&mut self, i: usize, v: u32) {
        let b = v.to_ne_bytes();
        self.s6_addr[4 * i] = b[0];
        self.s6_addr[4 * i + 1] = b[1];
        self.s6_addr[4 * i + 2] = b[2];
        self.s6_addr[4 * i + 3] = b[3];
    }
}

/// `struct sockaddr_in6`: socket address for IPv6.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SockaddrIn6 {
    /// Length of this struct (`sa_family_t`).
    pub sin6_len: u8,
    /// `AF_INET6`.
    pub sin6_family: SaFamily,
    /// Transport layer port #, network order.
    pub sin6_port: InPort,
    /// IP6 flow information.
    pub sin6_flowinfo: u32,
    /// IP6 address.
    pub sin6_addr: In6Addr,
    /// Interface scope id.
    pub sin6_scope_id: u32,
}
crate::abi_pod!(SockaddrIn6 {
    sin6_len: u8,
    sin6_family: SaFamily,
    sin6_port: InPort,
    sin6_flowinfo: u32,
    sin6_addr: In6Addr,
    sin6_scope_id: u32,
});

impl SockaddrIn6 {
    /// An all-zero socket address, usable in `const` contexts.
    pub const fn zeroed() -> Self {
        Self {
            sin6_len: 0,
            sin6_family: 0,
            sin6_port: 0,
            sin6_flowinfo: 0,
            sin6_addr: In6Addr { s6_addr: [0; 16] },
            sin6_scope_id: 0,
        }
    }

    /// A socket address of `addr` with its length and family set, as the C code fills one
    /// (`sin6_len = sizeof(sin6); sin6_family = AF_INET6; sin6_addr = addr`).
    pub const fn with_addr(addr: In6Addr) -> Self {
        Self {
            sin6_len: size_of::<SockaddrIn6>() as u8,
            sin6_family: AF_INET6,
            sin6_addr: addr,
            ..Self::zeroed()
        }
    }
}

/// `struct ipv6_mreq`: argument structure for `IPV6_JOIN_GROUP` and `IPV6_LEAVE_GROUP`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ipv6Mreq {
    /// `ipv6mr_multiaddr`.
    pub ipv6mr_multiaddr: In6Addr,
    /// `ipv6mr_interface`.
    pub ipv6mr_interface: u32,
}

crate::abi_pod!(Ipv6Mreq {
    ipv6mr_multiaddr: In6Addr,
    ipv6mr_interface: u32,
});

/// `struct in6_pktinfo`: `IPV6_PKTINFO`, packet information (RFC3542 sec 6).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct In6Pktinfo {
    /// src/dst IPv6 address.
    pub ipi6_addr: In6Addr,
    /// send/recv interface index.
    pub ipi6_ifindex: u32,
}

crate::abi_pod!(In6Pktinfo {
    ipi6_addr: In6Addr,
    ipi6_ifindex: u32,
});

/// `struct ip6_mtuinfo`: control structure for the `IPV6_RECVPATHMTU` socket option.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ip6Mtuinfo {
    /// Or sockaddr_storage?
    pub ip6m_addr: SockaddrIn6,
    /// The path MTU.
    pub ip6m_mtu: u32,
}

crate::abi_pod!(Ip6Mtuinfo {
    ip6m_addr: SockaddrIn6,
    ip6m_mtu: u32,
});

/// `IN6ADDR_ANY_INIT`: `::`.
pub const IN6ADDR_ANY_INIT: In6Addr = In6Addr::new([0; 16]);
/// `IN6ADDR_LOOPBACK_INIT`: `::1`.
pub const IN6ADDR_LOOPBACK_INIT: In6Addr =
    In6Addr::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
/// `IN6ADDR_NODELOCAL_ALLNODES_INIT`: `ff01::1`.
pub const IN6ADDR_NODELOCAL_ALLNODES_INIT: In6Addr =
    In6Addr::new([0xff, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
/// `IN6ADDR_INTFACELOCAL_ALLNODES_INIT`: `ff01::1`.
pub const IN6ADDR_INTFACELOCAL_ALLNODES_INIT: In6Addr =
    In6Addr::new([0xff, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
/// `IN6ADDR_LINKLOCAL_ALLNODES_INIT`: `ff02::1`.
pub const IN6ADDR_LINKLOCAL_ALLNODES_INIT: In6Addr =
    In6Addr::new([0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
/// `IN6ADDR_LINKLOCAL_ALLROUTERS_INIT`: `ff02::2`.
pub const IN6ADDR_LINKLOCAL_ALLROUTERS_INIT: In6Addr =
    In6Addr::new([0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);

/// `in6addr_any` (`in6.c`).
pub const IN6ADDR_ANY: In6Addr = IN6ADDR_ANY_INIT;
/// `in6addr_loopback` (`in6.c`).
pub const IN6ADDR_LOOPBACK: In6Addr = IN6ADDR_LOOPBACK_INIT;
/// `in6addr_intfacelocal_allnodes` (`in6.c`).
pub const IN6ADDR_INTFACELOCAL_ALLNODES: In6Addr = IN6ADDR_INTFACELOCAL_ALLNODES_INIT;
/// `in6addr_linklocal_allnodes` (`in6.c`).
pub const IN6ADDR_LINKLOCAL_ALLNODES: In6Addr = IN6ADDR_LINKLOCAL_ALLNODES_INIT;
/// `in6addr_linklocal_allrouters` (`in6.c`).
pub const IN6ADDR_LINKLOCAL_ALLROUTERS: In6Addr = IN6ADDR_LINKLOCAL_ALLROUTERS_INIT;

/// `IN6MASK0` / `in6mask0`: the /0 mask.
pub const IN6MASK0: In6Addr = In6Addr::new([0; 16]);
/// `IN6MASK32` / `in6mask32`: the /32 mask.
pub const IN6MASK32: In6Addr =
    In6Addr::new([0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
/// `IN6MASK64` / `in6mask64`: the /64 mask.
pub const IN6MASK64: In6Addr = In6Addr::new([
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0,
]);
/// `IN6MASK96` / `in6mask96`: the /96 mask.
pub const IN6MASK96: In6Addr = In6Addr::new([
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0,
]);
/// `IN6MASK128` / `in6mask128`: the /128 mask.
pub const IN6MASK128: In6Addr = In6Addr::new([0xff; 16]);

/// `sa6_any` (`in6.c`): the unspecified address as a socket address.
pub const SA6_ANY: SockaddrIn6 = SockaddrIn6::with_addr(IN6ADDR_ANY_INIT);

/// `IN6_ARE_ADDR_EQUAL(a, b)`.
#[inline]
pub const fn in6_are_addr_equal(a: &In6Addr, b: &In6Addr) -> bool {
    let mut i = 0;
    while i < 16 {
        if a.s6_addr[i] != b.s6_addr[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// `IN6_IS_ADDR_UNSPECIFIED(a)`: `::`.
#[inline]
pub const fn in6_is_addr_unspecified(a: &In6Addr) -> bool {
    a.s6_addr32(0) == 0 && a.s6_addr32(1) == 0 && a.s6_addr32(2) == 0 && a.s6_addr32(3) == 0
}

/// `IN6_IS_ADDR_LOOPBACK(a)`: `::1`.
#[inline]
pub const fn in6_is_addr_loopback(a: &In6Addr) -> bool {
    a.s6_addr32(0) == 0
        && a.s6_addr32(1) == 0
        && a.s6_addr32(2) == 0
        && a.s6_addr32(3) == __IPV6_ADDR_INT32_ONE
}

/// `IN6_IS_ADDR_V4COMPAT(a)`: IPv4 compatible (`::a.b.c.d`, not `::` or `::1`).
#[inline]
pub const fn in6_is_addr_v4compat(a: &In6Addr) -> bool {
    a.s6_addr32(0) == 0
        && a.s6_addr32(1) == 0
        && a.s6_addr32(2) == 0
        && a.s6_addr32(3) != 0
        && a.s6_addr32(3) != __IPV6_ADDR_INT32_ONE
}

/// `IN6_IS_ADDR_V4MAPPED(a)`: `::ffff:a.b.c.d`.
#[inline]
pub const fn in6_is_addr_v4mapped(a: &In6Addr) -> bool {
    a.s6_addr32(0) == 0 && a.s6_addr32(1) == 0 && a.s6_addr32(2) == __IPV6_ADDR_INT32_SMP
}

/// `IN6_IS_ADDR_LINKLOCAL(a)`: unicast link-local, `fe80::/10` (the topmost 10 bits only,
/// RFC2373).
#[inline]
pub const fn in6_is_addr_linklocal(a: &In6Addr) -> bool {
    a.s6_addr[0] == 0xfe && (a.s6_addr[1] & 0xc0) == 0x80
}

/// `IN6_IS_ADDR_SITELOCAL(a)`: unicast site-local, `fec0::/10`.
#[inline]
pub const fn in6_is_addr_sitelocal(a: &In6Addr) -> bool {
    a.s6_addr[0] == 0xfe && (a.s6_addr[1] & 0xc0) == 0xc0
}

/// `__IPV6_ADDR_MC_SCOPE(a)`: the scope of a multicast address.
#[inline]
pub const fn __ipv6_addr_mc_scope(a: &In6Addr) -> u8 {
    a.s6_addr[1] & 0x0f
}

/// `IN6_IS_ADDR_MULTICAST(a)`: `ff00::/8`.
#[inline]
pub const fn in6_is_addr_multicast(a: &In6Addr) -> bool {
    a.s6_addr[0] == 0xff
}

/// `IN6_IS_ADDR_MC_NODELOCAL(a)`.
#[inline]
pub const fn in6_is_addr_mc_nodelocal(a: &In6Addr) -> bool {
    in6_is_addr_multicast(a) && __ipv6_addr_mc_scope(a) == __IPV6_ADDR_SCOPE_NODELOCAL
}

/// `IN6_IS_ADDR_MC_INTFACELOCAL(a)`.
#[inline]
pub const fn in6_is_addr_mc_intfacelocal(a: &In6Addr) -> bool {
    in6_is_addr_multicast(a) && __ipv6_addr_mc_scope(a) == __IPV6_ADDR_SCOPE_INTFACELOCAL
}

/// `IN6_IS_ADDR_MC_LINKLOCAL(a)`.
#[inline]
pub const fn in6_is_addr_mc_linklocal(a: &In6Addr) -> bool {
    in6_is_addr_multicast(a) && __ipv6_addr_mc_scope(a) == __IPV6_ADDR_SCOPE_LINKLOCAL
}

/// `IN6_IS_ADDR_MC_SITELOCAL(a)`.
#[inline]
pub const fn in6_is_addr_mc_sitelocal(a: &In6Addr) -> bool {
    in6_is_addr_multicast(a) && __ipv6_addr_mc_scope(a) == __IPV6_ADDR_SCOPE_SITELOCAL
}

/// `IN6_IS_ADDR_MC_ORGLOCAL(a)`.
#[inline]
pub const fn in6_is_addr_mc_orglocal(a: &In6Addr) -> bool {
    in6_is_addr_multicast(a) && __ipv6_addr_mc_scope(a) == __IPV6_ADDR_SCOPE_ORGLOCAL
}

/// `IN6_IS_ADDR_MC_GLOBAL(a)`.
#[inline]
pub const fn in6_is_addr_mc_global(a: &In6Addr) -> bool {
    in6_is_addr_multicast(a) && __ipv6_addr_mc_scope(a) == __IPV6_ADDR_SCOPE_GLOBAL
}

/// `IN6_IS_SCOPE_LINKLOCAL(a)`: a unicast or multicast link-local address.
#[inline]
pub const fn in6_is_scope_linklocal(a: &In6Addr) -> bool {
    in6_is_addr_linklocal(a) || in6_is_addr_mc_linklocal(a)
}

/// `IN6_IS_SCOPE_EMBED(a)`: an address whose scope the kernel embeds in it (link-local
/// unicast or multicast, interface-local multicast).
#[inline]
pub const fn in6_is_scope_embed(a: &In6Addr) -> bool {
    in6_is_addr_linklocal(a) || in6_is_addr_mc_linklocal(a) || in6_is_addr_mc_intfacelocal(a)
}

// LP64 sizes of the C structures.
const _: () = {
    assert!(size_of::<In6Addr>() == 16);
    assert!(size_of::<SockaddrIn6>() == 28);
    assert!(core::mem::offset_of!(SockaddrIn6, sin6_addr) == 8);
    assert!(size_of::<Ipv6Mreq>() == 20);
    assert!(size_of::<In6Pktinfo>() == 20);
    assert!(size_of::<Ip6Mtuinfo>() == 32);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;

    use super::*;
    use crate::base::reftest::{assert_complete, assert_defines};

    /// `a:b:c:d:e:f:g:h` from its eight 16-bit groups.
    fn addr(g: [u16; 8]) -> In6Addr {
        let mut a = In6Addr::default();
        for (i, w) in g.iter().enumerate() {
            a.s6_addr[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
        }
        a
    }

    #[test]
    fn views_are_network_order_words() {
        let lo = IN6ADDR_LOOPBACK;
        assert_eq!(lo.s6_addr32(3), __IPV6_ADDR_INT32_ONE);
        assert_eq!(lo.s6_addr32(3), htonl(1));
        let ll = addr([0xfe80, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(ll.s6_addr16(0), __IPV6_ADDR_INT16_ULL);
        let mut a = In6Addr::default();
        a.set_s6_addr32(0, __IPV6_ADDR_INT32_MLL);
        a.set_s6_addr16(7, htons(2));
        assert_eq!(a, IN6ADDR_LINKLOCAL_ALLROUTERS);
        let w = [1, 2, 3, 4];
        let b = In6Addr::from_s6_addr32(w);
        assert_eq!(
            [
                b.s6_addr32(0),
                b.s6_addr32(1),
                b.s6_addr32(2),
                b.s6_addr32(3)
            ],
            w
        );
        assert_eq!(b.s6_addr8(0), b.s6_addr[0]);
    }

    #[test]
    fn address_classes() {
        let any = IN6ADDR_ANY;
        let lo = IN6ADDR_LOOPBACK;
        let ll = addr([0xfe80, 0, 0, 0, 0x0200, 0x5eff, 0xfe00, 0x5301]);
        let ll_wide = addr([0xfebf, 0, 0, 0, 0, 0, 0, 1]);
        let sl = addr([0xfec0, 0, 0, 0, 0, 0, 0, 1]);
        let global = addr([0x2001, 0xdb8, 0, 0, 0, 0, 0, 1]);
        let mapped = addr([0, 0, 0, 0, 0, 0xffff, 0xc000, 0x0201]);
        let compat = addr([0, 0, 0, 0, 0, 0, 0xc000, 0x0201]);

        assert!(in6_is_addr_unspecified(&any) && !in6_is_addr_unspecified(&lo));
        assert!(in6_is_addr_loopback(&lo) && !in6_is_addr_loopback(&any));
        assert!(in6_is_addr_linklocal(&ll) && in6_is_addr_linklocal(&ll_wide));
        assert!(!in6_is_addr_linklocal(&sl) && !in6_is_addr_linklocal(&global));
        assert!(in6_is_addr_sitelocal(&sl) && !in6_is_addr_sitelocal(&ll));
        assert!(in6_is_addr_v4mapped(&mapped) && !in6_is_addr_v4mapped(&compat));
        assert!(in6_is_addr_v4compat(&compat) && !in6_is_addr_v4compat(&mapped));
        assert!(!in6_is_addr_v4compat(&any) && !in6_is_addr_v4compat(&lo));
        assert!(!in6_is_addr_multicast(&global));
        assert!(in6_are_addr_equal(&ll, &ll) && !in6_are_addr_equal(&ll, &ll_wide));
    }

    #[test]
    fn multicast_scopes() {
        let nodelocal = IN6ADDR_NODELOCAL_ALLNODES_INIT;
        let linklocal = IN6ADDR_LINKLOCAL_ALLNODES;
        let site = addr([0xff05, 0, 0, 0, 0, 0, 0, 2]);
        let org = addr([0xff08, 0, 0, 0, 0, 0, 0, 2]);
        let global = addr([0xff0e, 0, 0, 0, 0, 0, 0, 0x101]);

        assert!(in6_is_addr_multicast(&nodelocal));
        assert!(in6_is_addr_mc_nodelocal(&nodelocal) && in6_is_addr_mc_intfacelocal(&nodelocal));
        assert!(in6_is_addr_mc_linklocal(&linklocal) && !in6_is_addr_mc_linklocal(&site));
        assert!(in6_is_addr_mc_sitelocal(&site));
        assert!(in6_is_addr_mc_orglocal(&org));
        assert!(in6_is_addr_mc_global(&global));
        assert_eq!(__ipv6_addr_mc_scope(&site), __IPV6_ADDR_SCOPE_SITELOCAL);

        let ll = addr([0xfe80, 0, 0, 0, 0, 0, 0, 1]);
        assert!(in6_is_scope_linklocal(&ll) && in6_is_scope_linklocal(&linklocal));
        assert!(!in6_is_scope_linklocal(&nodelocal));
        assert!(in6_is_scope_embed(&ll) && in6_is_scope_embed(&nodelocal));
        assert!(!in6_is_scope_embed(&site) && !in6_is_scope_embed(&IN6ADDR_LOOPBACK));
    }

    #[test]
    fn masks_and_socket_addresses() {
        assert_eq!(IN6MASK64.s6_addr32(1), 0xffff_ffff);
        assert_eq!(IN6MASK64.s6_addr32(2), 0);
        assert_eq!(IN6MASK96.s6_addr32(2), 0xffff_ffff);
        assert_eq!(IN6MASK128, In6Addr::new([0xff; 16]));
        assert_eq!(SA6_ANY.sin6_len as usize, size_of::<SockaddrIn6>());
        assert_eq!(SA6_ANY.sin6_family, AF_INET6);
        assert!(in6_is_addr_unspecified(&SA6_ANY.sin6_addr));
        assert_eq!(offset_of!(SockaddrIn6, sin6_flowinfo), 4);
        assert_eq!(offset_of!(SockaddrIn6, sin6_addr), 8);
        assert_eq!(offset_of!(SockaddrIn6, sin6_scope_id), 24);
        assert_eq!(CTL_IPV6PROTO_NAMES[41].ctl_name, Some(&b"ip6"[..]));
        assert_eq!(IPV6CTL_NAMES[3].ctl_name, Some(&b"hlim"[..]));
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::base::reftest::defines("sys/netinet6/in6.h");
        assert_defines!(defs; INET6_ADDRSTRLEN, ICMP6_FILTER, IPSEC6_OUTSA);
        let scope = assert_defines!(defs;
        __IPV6_ADDR_SCOPE_NODELOCAL, __IPV6_ADDR_SCOPE_INTFACELOCAL, __IPV6_ADDR_SCOPE_LINKLOCAL,
        __IPV6_ADDR_SCOPE_SITELOCAL, __IPV6_ADDR_SCOPE_ORGLOCAL, __IPV6_ADDR_SCOPE_GLOBAL);
        assert_complete(&defs, "__IPV6_ADDR_SCOPE_", &scope);
        let ipv6 = assert_defines!(defs;
        IPV6_UNICAST_HOPS, IPV6_MULTICAST_IF, IPV6_MULTICAST_HOPS, IPV6_MULTICAST_LOOP,
        IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP, IPV6_PORTRANGE, IPV6_CHECKSUM, IPV6_V6ONLY,
        IPV6_RTHDRDSTOPTS, IPV6_RECVPKTINFO, IPV6_RECVHOPLIMIT, IPV6_RECVRTHDR,
        IPV6_RECVHOPOPTS, IPV6_RECVDSTOPTS, IPV6_USE_MIN_MTU, IPV6_RECVPATHMTU, IPV6_PATHMTU,
        IPV6_PKTINFO, IPV6_HOPLIMIT, IPV6_NEXTHOP, IPV6_HOPOPTS, IPV6_DSTOPTS, IPV6_RTHDR,
        IPV6_AUTH_LEVEL, IPV6_ESP_TRANS_LEVEL, IPV6_ESP_NETWORK_LEVEL, IPV6_RECVTCLASS,
        IPV6_AUTOFLOWLABEL, IPV6_IPCOMP_LEVEL, IPV6_TCLASS, IPV6_DONTFRAG, IPV6_PIPEX,
        IPV6_RECVDSTPORT, IPV6_MINHOPCOUNT, IPV6_RTABLE, IPV6_RTHDR_LOOSE, IPV6_RTHDR_TYPE_0,
        IPV6_DEFAULT_MULTICAST_HOPS, IPV6_DEFAULT_MULTICAST_LOOP, IPV6_PORTRANGE_DEFAULT,
        IPV6_PORTRANGE_HIGH, IPV6_PORTRANGE_LOW);
        assert_complete(&defs, "IPV6_", &ipv6);
        let ctl = assert_defines!(defs;
        IPV6CTL_FORWARDING, IPV6CTL_SENDREDIRECTS, IPV6CTL_DEFHLIM, IPV6CTL_FORWSRCRT,
        IPV6CTL_STATS, IPV6CTL_MRTSTATS, IPV6CTL_MRTPROTO, IPV6CTL_MAXFRAGPACKETS,
        IPV6CTL_SOURCECHECK, IPV6CTL_SOURCECHECK_LOGINT, IPV6CTL_ACCEPT_RTADV,
        IPV6CTL_LOG_INTERVAL, IPV6CTL_HDRNESTLIMIT, IPV6CTL_DAD_COUNT, IPV6CTL_AUTO_FLOWLABEL,
        IPV6CTL_DEFMCASTHLIM, IPV6CTL_MAXFRAGS, IPV6CTL_MFORWARDING, IPV6CTL_MULTIPATH,
        IPV6CTL_MCAST_PMTU, IPV6CTL_NEIGHBORGCTHRESH, IPV6CTL_MAXDYNROUTES,
        IPV6CTL_DAD_PENDING, IPV6CTL_MTUDISCTIMEOUT, IPV6CTL_IFQUEUE, IPV6CTL_MRTMIF,
        IPV6CTL_MRTMFC, IPV6CTL_MAXID);
        assert_complete(&defs, "IPV6CTL_", &[&ctl[..], &["IPV6CTL_NAMES"]].concat());
        assert_eq!(defs["IPV6PROTO_MAXID"], "(IPPROTO_DIVERT + 1)");
    }
}
/* </TESTS> */
