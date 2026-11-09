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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)if.h	8.1 (Berkeley) 6/10/93
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
 * Copyright (c) 1980, 1986, 1993
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
 *	@(#)if.c	8.3 (Berkeley) 1/4/94
 */

/*
 * Copyright (c) 1989, 1993, 1994
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
 *	@(#)if_types.h	8.3 (Berkeley) 4/28/95
 */

/*
 * Copyright (c) 1990, 1993
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
 *	@(#)if_dl.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The interface ABI: `<net/if.h>`'s flags, constants and request images, `<net/if_types.h>`'s
//! interface types and `<net/if_dl.h>`'s link-level address, what userland, the routing
//! socket and the drivers share with the kernel's interface layer (`kernel::net::if_`).
//!
//! Upstream: sys/net/if.h @ 3ce1f3f79392
//! Upstream: sys/net/if.c @ 3ce1f3f79392
//! Upstream: sys/net/if_types.h @ 3ce1f3f79392
//! Upstream: sys/net/if_dl.h @ 3ce1f3f79392
//! LZ: sys/net/if_.rs@3c62ede68802
//! LZ: sys/net/if_types.rs@3c62ede68802
//! LZ: sys/net/if_dl.rs@3c62ede68802
//!
//! The structures cross the user/kernel boundary (`ioctl(2)`'s argument, the routing
//! socket's messages, `sysctl(2)`'s `NET_RT_IFLIST`): each is an image whose bytes are the
//! C's on LP64 (amd64 and arm64 alike), made `AbiPod` by [`crate::abi_pod!`] with the holes
//! the C compiler leaves named (`_pad0`, ...), and its size asserted at compile time. A user
//! or kernel address inside one (`ifr_data`, `ifc_buf`, `ifm_ulist`, ...) is a `u64` word,
//! never a pointer: the kernel copies through it (`hal::user::UserPtr`), it never follows it.
//!
//! The C's unnamed unions (`ifr_ifru`, `ifra_ifrau`, `ifgr_ifgru`, `ifgrq_ifgrqu`,
//! `ifc_ifcu`) are byte arrays of the union's size, the structure aligned as the union is,
//! and the C's `#define ifr_flags ifr_ifru.ifru_flags` shorthands are accessors that read and
//! write the member's bytes at the union's start in the machine's byte order: what the C's
//! member access does, with no union and no `unsafe`.
//!
//! ## Deviations
//! - `CTL_IFQ_NAMES` (a `struct ctlname` table) comes with `<sys/sysctl.h>`'s tables.
//! - `LINK_STATE_DESCRIPTIONS` is a slice of [`IfStatusDescription`] whose strings are byte
//!   slices; the C's terminating `{ 0, 0, NULL }` entry is the slice's end.
//! - `IFG_ALL` and `IFG_EGRESS` are byte strings without the NUL.
//! - `struct if_nameindex` (`if_nameindex(3)`'s, a userland pointer to the name) is
//!   userland's: the kernel never builds one.
//! - `LLADDR(s)` is [`SockaddrDl::lladdr`], over the bytes of a `sockaddr_dl` that may be
//!   longer than the structure (`if_alloc_sadl`'s), so it takes the whole slice; `satosdl`,
//!   `sdltosa` and the other casts are views of an address's bytes (`hal::pod::view`).
//! - `link_ntoa(3)` is userland.
//!
//! ## Redesign
//! - LZ's `#[repr(C)]` structures with `#[repr(C)] union` members read through `unsafe`
//!   accessors (`IfrIfru`, `IfraIfrau`, `IfgrIfgru`, `IfgrqIfgrqu`, `IfcIfcu`) and raw pointer
//!   members -> `AbiPod` images: the unions are byte arrays, the pointers `u64` words, so the
//!   kernel views a request in its `ioctl(2)` buffer with `hal::pod::view_mut` and copies a
//!   message out as its bytes, with no `unsafe` and no padding byte left uninitialised
//!   (N6.3a, decision 47's ABI edge; `bsd`'s `sys/net/if_.rs` keeps LZ's structures for its
//!   `ifioctl` until N6.3a2).
//! - `struct if_data`'s `ifi_lastchange` is an [`IfTimeval`]: `struct timeval`'s image, as
//!   the kernel's `Timeval` is `kernel::clock`'s and `hal` names nothing of the kernel.
//! - `if.c`'s functions are the kernel's (`kernel::net::if_`, N6.3a); only the header's part
//!   of LZ's `sys/net/if_.rs` is here.
//! - The three headers are one module: `if_types.h` and `if_dl.h` are the constants and the
//!   one structure `if.h`'s users need beside it (`if_alloc_sadl`, the routing socket).

use crate::sys::socket::{Sockaddr, SockaddrStorage};
use crate::sys::types::SaFamily;

/// Length of interface external name, including terminating '\0'. Note: this is the same size
/// as a generic device's external name.
pub const IF_NAMESIZE: usize = 16;

/// Number of cluster pools.
pub const MCLPOOLS: usize = 8;

/// `IFQ_NQUEUES`: priority queues per interface queue.
pub const IFQ_NQUEUES: u32 = 8;
/// `IFQ_MINPRIO`.
pub const IFQ_MINPRIO: u32 = 0;
/// `IFQ_MAXPRIO`.
pub const IFQ_MAXPRIO: u32 = IFQ_NQUEUES - 1;
/// `IFQ_DEFPRIO`.
pub const IFQ_DEFPRIO: u32 = 3;

// Values for if_link_state.

/// Link unknown.
pub const LINK_STATE_UNKNOWN: u8 = 0;
/// Link invalid.
pub const LINK_STATE_INVALID: u8 = 1;
/// Link is down.
pub const LINK_STATE_DOWN: u8 = 2;
/// Keepalive reports down.
pub const LINK_STATE_KALIVE_DOWN: u8 = 3;
/// Link is up.
pub const LINK_STATE_UP: u8 = 4;
/// Link is up and half duplex.
pub const LINK_STATE_HALF_DUPLEX: u8 = 5;
/// Link is up and full duplex.
pub const LINK_STATE_FULL_DUPLEX: u8 = 6;

/// Traditional BSD name for length of interface external name.
pub const IFNAMSIZ: usize = IF_NAMESIZE;

/// Length of interface description, including terminating '\0'.
pub const IFDESCRSIZE: usize = 64;

// Interface flags can be either owned by the stack or the driver. The comments say who toggles
// which flag: [I] immutable after creation, [N] written by the stack (upon user request), [d]
// written by the driver, [c] for userland compatibility only.

/// [N] interface is up.
pub const IFF_UP: i32 = 0x1;
/// [I] broadcast address valid.
pub const IFF_BROADCAST: i32 = 0x2;
/// [N] turn on debugging.
pub const IFF_DEBUG: i32 = 0x4;
/// [I] is a loopback net.
pub const IFF_LOOPBACK: i32 = 0x8;
/// [I] is point-to-point link.
pub const IFF_POINTOPOINT: i32 = 0x10;
/// [N] only static ARP.
pub const IFF_STATICARP: i32 = 0x20;
/// [d] resources allocated.
pub const IFF_RUNNING: i32 = 0x40;
/// [N] no address resolution protocol.
pub const IFF_NOARP: i32 = 0x80;
/// [N] receive all packets.
pub const IFF_PROMISC: i32 = 0x100;
/// [d] receive all multicast packets.
pub const IFF_ALLMULTI: i32 = 0x200;
/// [c] transmission in progress.
pub const IFF_OACTIVE: i32 = 0x400;
/// [I] can't hear own transmissions.
pub const IFF_SIMPLEX: i32 = 0x800;
/// [N] per link layer defined bit.
pub const IFF_LINK0: i32 = 0x1000;
/// [N] per link layer defined bit.
pub const IFF_LINK1: i32 = 0x2000;
/// [N] per link layer defined bit.
pub const IFF_LINK2: i32 = 0x4000;
/// [I] supports multicast.
pub const IFF_MULTICAST: i32 = 0x8000;

/// Flags set internally only.
pub const IFF_CANTCHANGE: i32 = IFF_BROADCAST
    | IFF_LOOPBACK
    | IFF_POINTOPOINT
    | IFF_RUNNING
    | IFF_OACTIVE
    | IFF_SIMPLEX
    | IFF_MULTICAST
    | IFF_ALLMULTI;

/// [I] `if_start` is mpsafe.
pub const IFXF_MPSAFE: i32 = 0x1;
/// [I] pseudo interface.
pub const IFXF_CLONED: i32 = 0x2;
/// [N] v6 temporary addrs enabled.
pub const IFXF_AUTOCONF6TEMP: i32 = 0x4;
/// [N] supports MPLS.
pub const IFXF_MPLS: i32 = 0x8;
/// [N] wake on lan enabled.
pub const IFXF_WOL: i32 = 0x10;
/// [N] v6 autoconf enabled.
pub const IFXF_AUTOCONF6: i32 = 0x20;
/// [N] don't do RFC 7217.
pub const IFXF_INET6_NOSOII: i32 = 0x40;
/// [N] v4 autoconf (aka dhcp) enabled.
pub const IFXF_AUTOCONF4: i32 = 0x80;
/// [N] only used for bpf.
pub const IFXF_MONITOR: i32 = 0x100;
/// [N] TCP large recv offload.
pub const IFXF_LRO: i32 = 0x200;
/// [I] mbuf with 64 bit DMA supported.
pub const IFXF_MBUF_64BIT: i32 = 0x400;

/// Extended flags the user cannot change.
pub const IFXF_CANTCHANGE: i32 = IFXF_MPSAFE | IFXF_CLONED;

// Capabilities that interfaces can advertise.

/// Can do IPv4 header csum.
#[allow(non_upper_case_globals)] // the C name
pub const IFCAP_CSUM_IPv4: u32 = 0x0000_0001;
/// Can do IPv4/TCP csum.
#[allow(non_upper_case_globals)] // the C name
pub const IFCAP_CSUM_TCPv4: u32 = 0x0000_0002;
/// Can do IPv4/UDP csum.
#[allow(non_upper_case_globals)] // the C name
pub const IFCAP_CSUM_UDPv4: u32 = 0x0000_0004;
/// VLAN-compatible MTU.
pub const IFCAP_VLAN_MTU: u32 = 0x0000_0010;
/// Hardware VLAN tag support.
pub const IFCAP_VLAN_HWTAGGING: u32 = 0x0000_0020;
/// HW offload w/ inline tag.
pub const IFCAP_VLAN_HWOFFLOAD: u32 = 0x0000_0040;
/// Can do IPv6/TCP checksums.
#[allow(non_upper_case_globals)] // the C name
pub const IFCAP_CSUM_TCPv6: u32 = 0x0000_0080;
/// Can do IPv6/UDP checksums.
#[allow(non_upper_case_globals)] // the C name
pub const IFCAP_CSUM_UDPv6: u32 = 0x0000_0100;
/// IPv4/TCP segment offload.
#[allow(non_upper_case_globals)] // the C name
pub const IFCAP_TSOv4: u32 = 0x0000_1000;
/// IPv6/TCP segment offload.
#[allow(non_upper_case_globals)] // the C name
pub const IFCAP_TSOv6: u32 = 0x0000_2000;
/// TCP large recv offload.
pub const IFCAP_LRO: u32 = 0x0000_4000;
/// Can do wake on lan.
pub const IFCAP_WOL: u32 = 0x0000_8000;

/// The checksum offload capabilities.
pub const IFCAP_CSUM_MASK: u32 =
    IFCAP_CSUM_IPv4 | IFCAP_CSUM_TCPv4 | IFCAP_CSUM_UDPv4 | IFCAP_CSUM_TCPv6 | IFCAP_CSUM_UDPv6;

// Symbolic names for terminal (per-protocol) CTL_IFQ_ nodes.

/// `IFQCTL_LEN`.
pub const IFQCTL_LEN: i32 = 1;
/// `IFQCTL_MAXLEN`.
pub const IFQCTL_MAXLEN: i32 = 2;
/// `IFQCTL_DROPS`.
pub const IFQCTL_DROPS: i32 = 3;
/// `IFQCTL_CONGESTION`.
pub const IFQCTL_CONGESTION: i32 = 4;
/// `IFQCTL_MAXID`.
pub const IFQCTL_MAXID: i32 = 5;

/// Interface arrival.
pub const IFAN_ARRIVAL: u16 = 0;
/// Interface departure.
pub const IFAN_DEPARTURE: u16 = 1;

/// Group contains all interfaces.
pub const IFG_ALL: &[u8] = b"all";
/// If(s) default route(s) point to.
pub const IFG_EGRESS: &[u8] = b"egress";

/// `IF_HDRPRIO_MIN`.
pub const IF_HDRPRIO_MIN: i32 = IFQ_MINPRIO as i32;
/// `IF_HDRPRIO_MAX`.
pub const IF_HDRPRIO_MAX: i32 = IFQ_MAXPRIO as i32;
/// Use mbuf prio.
pub const IF_HDRPRIO_PACKET: i32 = -1;
/// Copy payload prio.
pub const IF_HDRPRIO_PAYLOAD: i32 = -2;
/// Use outer prio.
pub const IF_HDRPRIO_OUTER: i32 = -3;

/// Ethernet or ethernet tagged.
pub const IF_PWE3_ETHERNET: i32 = 1;
/// IP layer 2.
pub const IF_PWE3_IP: i32 = 2;

/// In: prefix given; out: kernel fills id.
pub const IFLR_PREFIX: u32 = 0x8000;

/// `IFSFF_ADDR_EEPROM`.
pub const IFSFF_ADDR_EEPROM: u8 = 0xa0;
/// `IFSFF_ADDR_DDM`.
pub const IFSFF_ADDR_DDM: u8 = 0xa2;

/// `IFSFF_DATA_LEN`.
pub const IFSFF_DATA_LEN: usize = 256;

/// `IF_MAX_VECTORS`: most interrupt vectors (and queues) one interface uses.
pub const IF_MAX_VECTORS: usize = 8;

// Interface types (`<net/if_types.h>`).

/// None of the following.
pub const IFT_OTHER: u8 = 0x01;
/// Old-style arpanet imp.
pub const IFT_1822: u8 = 0x02;
/// HDH arpanet imp.
pub const IFT_HDH1822: u8 = 0x03;
/// X25 to imp.
pub const IFT_X25DDN: u8 = 0x04;
/// PDN X25 interface (RFC877)
pub const IFT_X25: u8 = 0x05;
/// Ethernet CSMA/CD.
pub const IFT_ETHER: u8 = 0x06;
/// CSMA/CD.
pub const IFT_ISO88023: u8 = 0x07;
/// Token Bus.
pub const IFT_ISO88024: u8 = 0x08;
/// Token Ring.
pub const IFT_ISO88025: u8 = 0x09;
/// MAN.
pub const IFT_ISO88026: u8 = 0x0a;
/// `IFT_STARLAN`.
pub const IFT_STARLAN: u8 = 0x0b;
/// Proteon 10MBit ring.
pub const IFT_P10: u8 = 0x0c;
/// Proteon 80MBit ring.
pub const IFT_P80: u8 = 0x0d;
/// Hyperchannel.
pub const IFT_HY: u8 = 0x0e;
/// `IFT_FDDI`.
pub const IFT_FDDI: u8 = 0x0f;
/// `IFT_LAPB`.
pub const IFT_LAPB: u8 = 0x10;
/// `IFT_SDLC`.
pub const IFT_SDLC: u8 = 0x11;
/// `IFT_T1`.
pub const IFT_T1: u8 = 0x12;
/// E1 - european T1.
pub const IFT_CEPT: u8 = 0x13;
/// `IFT_ISDNBASIC`.
pub const IFT_ISDNBASIC: u8 = 0x14;
/// `IFT_ISDNPRIMARY`.
pub const IFT_ISDNPRIMARY: u8 = 0x15;
/// Proprietary PTP serial.
pub const IFT_PTPSERIAL: u8 = 0x16;
/// RFC 1331.
pub const IFT_PPP: u8 = 0x17;
/// Loopback.
pub const IFT_LOOP: u8 = 0x18;
/// ISO over IP.
pub const IFT_EON: u8 = 0x19;
/// Obsolete 3MB experimental ethernet.
pub const IFT_XETHER: u8 = 0x1a;
/// XNS over IP.
pub const IFT_NSIP: u8 = 0x1b;
/// IP over generic TTY.
pub const IFT_SLIP: u8 = 0x1c;
/// Ultra Technologies.
pub const IFT_ULTRA: u8 = 0x1d;
/// Generic T3.
pub const IFT_DS3: u8 = 0x1e;
/// SMDS.
pub const IFT_SIP: u8 = 0x1f;
/// Frame Relay DTE only.
pub const IFT_FRELAY: u8 = 0x20;
/// `IFT_RS232`.
pub const IFT_RS232: u8 = 0x21;
/// Parallel-port.
pub const IFT_PARA: u8 = 0x22;
/// `IFT_ARCNET`.
pub const IFT_ARCNET: u8 = 0x23;
/// `IFT_ARCNETPLUS`.
pub const IFT_ARCNETPLUS: u8 = 0x24;
/// ATM cells.
pub const IFT_ATM: u8 = 0x25;
/// `IFT_MIOX25`.
pub const IFT_MIOX25: u8 = 0x26;
/// SONET or SDH.
pub const IFT_SONET: u8 = 0x27;
/// `IFT_X25PLE`.
pub const IFT_X25PLE: u8 = 0x28;
/// `IFT_ISO88022LLC`.
pub const IFT_ISO88022LLC: u8 = 0x29;
/// `IFT_LOCALTALK`.
pub const IFT_LOCALTALK: u8 = 0x2a;
/// `IFT_SMDSDXI`.
pub const IFT_SMDSDXI: u8 = 0x2b;
/// Frame Relay DCE.
pub const IFT_FRELAYDCE: u8 = 0x2c;
/// `IFT_V35`.
pub const IFT_V35: u8 = 0x2d;
/// `IFT_HSSI`.
pub const IFT_HSSI: u8 = 0x2e;
/// `IFT_HIPPI`.
pub const IFT_HIPPI: u8 = 0x2f;
/// Generic Modem.
pub const IFT_MODEM: u8 = 0x30;
/// AAL5 over ATM.
pub const IFT_AAL5: u8 = 0x31;
/// `IFT_SONETPATH`.
pub const IFT_SONETPATH: u8 = 0x32;
/// `IFT_SONETVT`.
pub const IFT_SONETVT: u8 = 0x33;
/// SMDS InterCarrier Interface.
pub const IFT_SMDSICIP: u8 = 0x34;
/// Proprietary Virtual/internal.
pub const IFT_PROPVIRTUAL: u8 = 0x35;
/// Proprietary Multiplexing.
pub const IFT_PROPMUX: u8 = 0x36;
/// 100BaseVG.
pub const IFT_IEEE80212: u8 = 0x37;
/// Fibre Channel.
pub const IFT_FIBRECHANNEL: u8 = 0x38;
/// HIPPI interfaces.
pub const IFT_HIPPIINTERFACE: u8 = 0x39;
/// Obsolete, use either 0x20 or 0x2c.
pub const IFT_FRAMERELAYINTERCONNECT: u8 = 0x3a;
/// ATM Emulated LAN for 802.3.
pub const IFT_AFLANE8023: u8 = 0x3b;
/// ATM Emulated LAN for 802.5.
pub const IFT_AFLANE8025: u8 = 0x3c;
/// ATM Emulated circuit.
pub const IFT_CCTEMUL: u8 = 0x3d;
/// Fast Ethernet (100BaseT)
pub const IFT_FASTETHER: u8 = 0x3e;
/// ISDN and X.25.
pub const IFT_ISDN: u8 = 0x3f;
/// CCITT V.11/X.21.
pub const IFT_V11: u8 = 0x40;
/// CCITT V.36.
pub const IFT_V36: u8 = 0x41;
/// CCITT G703 at 64Kbps.
pub const IFT_G703AT64K: u8 = 0x42;
/// Obsolete see DS1-MIB.
pub const IFT_G703AT2MB: u8 = 0x43;
/// SNA QLLC.
pub const IFT_QLLC: u8 = 0x44;
/// Fast Ethernet (100BaseFX)
pub const IFT_FASTETHERFX: u8 = 0x45;
/// Channel.
pub const IFT_CHANNEL: u8 = 0x46;
/// Radio spread spectrum.
pub const IFT_IEEE80211: u8 = 0x47;
/// IBM System 360/370 OEMI Channel.
pub const IFT_IBM370PARCHAN: u8 = 0x48;
/// IBM Enterprise Systems Connection.
pub const IFT_ESCON: u8 = 0x49;
/// Data Link Switching.
pub const IFT_DLSW: u8 = 0x4a;
/// ISDN S/T interface.
pub const IFT_ISDNS: u8 = 0x4b;
/// ISDN U interface.
pub const IFT_ISDNU: u8 = 0x4c;
/// Link Access Protocol D.
pub const IFT_LAPD: u8 = 0x4d;
/// IP Switching Objects.
pub const IFT_IPSWITCH: u8 = 0x4e;
/// Remote Source Route Bridging.
pub const IFT_RSRB: u8 = 0x4f;
/// ATM Logical Port.
pub const IFT_ATMLOGICAL: u8 = 0x50;
/// Digital Signal Level 0.
pub const IFT_DS0: u8 = 0x51;
/// Group of ds0s on the same ds1.
pub const IFT_DS0BUNDLE: u8 = 0x52;
/// Bisynchronous Protocol.
pub const IFT_BSC: u8 = 0x53;
/// Asynchronous Protocol.
pub const IFT_ASYNC: u8 = 0x54;
/// Combat Net Radio.
pub const IFT_CNR: u8 = 0x55;
/// ISO 802.5r DTR.
pub const IFT_ISO88025DTR: u8 = 0x56;
/// Ext Pos Loc Report Sys.
pub const IFT_EPLRS: u8 = 0x57;
/// Appletalk Remote Access Protocol.
pub const IFT_ARAP: u8 = 0x58;
/// Proprietary Connectionless Protocol.
pub const IFT_PROPCNLS: u8 = 0x59;
/// CCITT-ITU X.29 PAD Protocol.
pub const IFT_HOSTPAD: u8 = 0x5a;
/// CCITT-ITU X.3 PAD Facility.
pub const IFT_TERMPAD: u8 = 0x5b;
/// Multiproto Interconnect over FR.
pub const IFT_FRAMERELAYMPI: u8 = 0x5c;
/// CCITT-ITU X213.
pub const IFT_X213: u8 = 0x5d;
/// Asymmetric Digital Subscriber Loop.
pub const IFT_ADSL: u8 = 0x5e;
/// Rate-Adapt. Digital Subscriber Loop.
pub const IFT_RADSL: u8 = 0x5f;
/// Symmetric Digital Subscriber Loop.
pub const IFT_SDSL: u8 = 0x60;
/// Very H-Speed Digital Subscrib. Loop.
pub const IFT_VDSL: u8 = 0x61;
/// ISO 802.5 CRFP.
pub const IFT_ISO88025CRFPINT: u8 = 0x62;
/// Myricom Myrinet.
pub const IFT_MYRINET: u8 = 0x63;
/// Voice recEive and transMit.
pub const IFT_VOICEEM: u8 = 0x64;
/// Voice Foreign Exchange Office.
pub const IFT_VOICEFXO: u8 = 0x65;
/// Voice Foreign Exchange Station.
pub const IFT_VOICEFXS: u8 = 0x66;
/// Voice encapsulation.
pub const IFT_VOICEENCAP: u8 = 0x67;
/// Voice over IP encapsulation.
pub const IFT_VOICEOVERIP: u8 = 0x68;
/// ATM DXI.
pub const IFT_ATMDXI: u8 = 0x69;
/// ATM FUNI.
pub const IFT_ATMFUNI: u8 = 0x6a;
/// ATM IMA.
pub const IFT_ATMIMA: u8 = 0x6b;
/// PPP Multilink Bundle.
pub const IFT_PPPMULTILINKBUNDLE: u8 = 0x6c;
/// IBM ipOverCdlc.
pub const IFT_IPOVERCDLC: u8 = 0x6d;
/// IBM Common Link Access to Workstn.
pub const IFT_IPOVERCLAW: u8 = 0x6e;
/// IBM stackToStack.
pub const IFT_STACKTOSTACK: u8 = 0x6f;
/// IBM VIPA.
pub const IFT_VIRTUALIPADDRESS: u8 = 0x70;
/// IBM multi-protocol channel support.
pub const IFT_MPC: u8 = 0x71;
/// IBM ipOverAtm.
pub const IFT_IPOVERATM: u8 = 0x72;
/// ISO 802.5j Fiber Token Ring.
pub const IFT_ISO88025FIBER: u8 = 0x73;
/// IBM twinaxial data link control.
pub const IFT_TDLC: u8 = 0x74;
/// Gigabit Ethernet.
pub const IFT_GIGABITETHERNET: u8 = 0x75;
/// HDLC.
pub const IFT_HDLC: u8 = 0x76;
/// LAP F.
pub const IFT_LAPF: u8 = 0x77;
/// V.37.
pub const IFT_V37: u8 = 0x78;
/// Multi-Link Protocol.
pub const IFT_X25MLP: u8 = 0x79;
/// X25 Hunt Group.
pub const IFT_X25HUNTGROUP: u8 = 0x7a;
/// Transp HDLC.
pub const IFT_TRANSPHDLC: u8 = 0x7b;
/// Interleave channel.
pub const IFT_INTERLEAVE: u8 = 0x7c;
/// Fast channel.
pub const IFT_FAST: u8 = 0x7d;
/// IP (for APPN HPR in IP networks)
pub const IFT_IP: u8 = 0x7e;
/// CATV Mac Layer.
pub const IFT_DOCSCABLEMACLAYER: u8 = 0x7f;
/// CATV Downstream interface.
pub const IFT_DOCSCABLEDOWNSTREAM: u8 = 0x80;
/// CATV Upstream interface.
pub const IFT_DOCSCABLEUPSTREAM: u8 = 0x81;
/// Avalon Parallel Processor.
pub const IFT_A12MPPSWITCH: u8 = 0x82;
/// Encapsulation interface.
pub const IFT_TUNNEL: u8 = 0x83;
/// Coffee pot.
pub const IFT_COFFEE: u8 = 0x84;
/// Circuit Emulation Service.
pub const IFT_CES: u8 = 0x85;
/// (x)  ATM Sub Interface.
pub const IFT_ATMSUBINTERFACE: u8 = 0x86;
/// Layer 2 Virtual LAN using 802.1Q.
pub const IFT_L2VLAN: u8 = 0x87;
/// Layer 3 Virtual LAN - IP Protocol.
pub const IFT_L3IPVLAN: u8 = 0x88;
/// Layer 3 Virtual LAN - IPX Prot.
pub const IFT_L3IPXVLAN: u8 = 0x89;
/// IP over Power Lines.
pub const IFT_DIGITALPOWERLINE: u8 = 0x8a;
/// (xxx)  Multimedia Mail over IP.
pub const IFT_MEDIAMAILOVERIP: u8 = 0x8b;
/// Dynamic synchronous Transfer Mode.
pub const IFT_DTM: u8 = 0x8c;
/// Data Communications Network.
pub const IFT_DCN: u8 = 0x8d;
/// IP Forwarding Interface.
pub const IFT_IPFORWARD: u8 = 0x8e;
/// Multi-rate Symmetric DSL.
pub const IFT_MSDSL: u8 = 0x8f;
/// IEEE1394 High Performance SerialBus.
pub const IFT_IEEE1394: u8 = 0x90;
/// HIPPI-6400.
pub const IFT_IFGSN: u8 = 0x91;
/// DVB-RCC MAC Layer.
pub const IFT_DVBRCCMACLAYER: u8 = 0x92;
/// DVB-RCC Downstream Channel.
pub const IFT_DVBRCCDOWNSTREAM: u8 = 0x93;
/// DVB-RCC Upstream Channel.
pub const IFT_DVBRCCUPSTREAM: u8 = 0x94;
/// ATM Virtual Interface.
pub const IFT_ATMVIRTUAL: u8 = 0x95;
/// MPLS Tunnel Virtual Interface.
pub const IFT_MPLSTUNNEL: u8 = 0x96;
/// Spatial Reuse Protocol.
pub const IFT_SRP: u8 = 0x97;
/// Voice over ATM.
pub const IFT_VOICEOVERATM: u8 = 0x98;
/// Voice Over Frame Relay.
pub const IFT_VOICEOVERFRAMERELAY: u8 = 0x99;
/// Digital Subscriber Loop over ISDN.
pub const IFT_IDSL: u8 = 0x9a;
/// Avici Composite Link Interface.
pub const IFT_COMPOSITELINK: u8 = 0x9b;
/// SS7 Signaling Link.
pub const IFT_SS7SIGLINK: u8 = 0x9c;
/// Prop. P2P wireless interface.
pub const IFT_PROPWIRELESSP2P: u8 = 0x9d;
/// Frame forward Interface.
pub const IFT_FRFORWARD: u8 = 0x9e;
/// Multiprotocol over ATM AAL5.
pub const IFT_RFC1483: u8 = 0x9f;
/// USB Interface.
pub const IFT_USB: u8 = 0xa0;
/// IEEE 802.3ad Link Aggregate.
pub const IFT_IEEE8023ADLAG: u8 = 0xa1;
/// BGP Policy Accounting.
pub const IFT_BGPPOLICYACCOUNTING: u8 = 0xa2;
/// FRF.16 Multilink Frame Relay.
pub const IFT_FRF16MFRBUNDLE: u8 = 0xa3;
/// H323 Gatekeeper.
pub const IFT_H323GATEKEEPER: u8 = 0xa4;
/// H323 Voice and Video Proxy.
pub const IFT_H323PROXY: u8 = 0xa5;
/// MPLS.
pub const IFT_MPLS: u8 = 0xa6;
/// Multi-frequency signaling link.
pub const IFT_MFSIGLINK: u8 = 0xa7;
/// High Bit-Rate DSL, 2nd gen.
pub const IFT_HDSL2: u8 = 0xa8;
/// Multirate HDSL2.
pub const IFT_SHDSL: u8 = 0xa9;
/// Facility Data Link (4Kbps) on a DS1.
pub const IFT_DS1FDL: u8 = 0xaa;
/// Packet over SONET/SDH Interface.
pub const IFT_POS: u8 = 0xab;
/// DVB-ASI Input.
pub const IFT_DVBASILN: u8 = 0xac;
/// DVB-ASI Output.
pub const IFT_DVBASIOUT: u8 = 0xad;
/// Power Line Communications.
pub const IFT_PLC: u8 = 0xae;
/// Non-Facility Associated Signaling.
pub const IFT_NFAS: u8 = 0xaf;
/// TROO8.
pub const IFT_TR008: u8 = 0xb0;
/// Remote Digital Terminal.
pub const IFT_GR303RDT: u8 = 0xb1;
/// Integrated Digital Terminal.
pub const IFT_GR303IDT: u8 = 0xb2;
/// ISUP.
pub const IFT_ISUP: u8 = 0xb3;
/// Prop/Wireless MAC Layer.
pub const IFT_PROPDOCSWIRELESSMACLAYER: u8 = 0xb4;
/// Prop/Wireless Downstream.
pub const IFT_PROPDOCSWIRELESSDOWNSTREAM: u8 = 0xb5;
/// Prop/Wireless Upstream.
pub const IFT_PROPDOCSWIRELESSUPSTREAM: u8 = 0xb6;
/// HIPERLAN Type 2 Radio Interface.
pub const IFT_HIPERLAN2: u8 = 0xb7;
/// PropBroadbandWirelessAccess P2MP.
pub const IFT_PROPBWAP2MP: u8 = 0xb8;
/// SONET Overhead Channel.
pub const IFT_SONETOVERHEADCHANNEL: u8 = 0xb9;
/// Digital Wrapper Overhead.
pub const IFT_DIGITALWRAPPEROVERHEADCHANNEL: u8 = 0xba;
/// ATM adaptation layer 2.
pub const IFT_AAL2: u8 = 0xbb;
/// MAC layer over radio links.
pub const IFT_RADIOMAC: u8 = 0xbc;
/// ATM over radio links.
pub const IFT_ATMRADIO: u8 = 0xbd;
/// Inter-Machine Trunks.
pub const IFT_IMT: u8 = 0xbe;
/// Multiple Virtual Lines DSL.
pub const IFT_MVL: u8 = 0xbf;
/// Long Reach DSL.
pub const IFT_REACHDSL: u8 = 0xc0;
/// Frame Relay DLCI End Point.
pub const IFT_FRDLCIENDPT: u8 = 0xc1;
/// ATM VCI End Point.
pub const IFT_ATMVCIENDPT: u8 = 0xc2;
/// Optical Channel.
pub const IFT_OPTICALCHANNEL: u8 = 0xc3;
/// Optical Transport.
pub const IFT_OPTICALTRANSPORT: u8 = 0xc4;
/// Proprietary ATM.
pub const IFT_PROPATM: u8 = 0xc5;
/// Voice Over Cable Interface.
pub const IFT_VOICEOVERCABLE: u8 = 0xc6;
/// Infiniband.
pub const IFT_INFINIBAND: u8 = 0xc7;
/// TE Link.
pub const IFT_TELINK: u8 = 0xc8;
/// Q.2931.
pub const IFT_Q2931: u8 = 0xc9;
/// Virtual Trunk Group.
pub const IFT_VIRTUALTG: u8 = 0xca;
/// SIP Trunk Group.
pub const IFT_SIPTG: u8 = 0xcb;
/// SIP Signaling.
pub const IFT_SIPSIG: u8 = 0xcc;
/// CATV Upstream Channel.
pub const IFT_DOCSCABLEUPSTREAMCHANNEL: u8 = 0xcd;
/// Acorn Econet.
pub const IFT_ECONET: u8 = 0xce;
/// FSAN 155Mb Symmetrical PON interface.
pub const IFT_PON155: u8 = 0xcf;
/// FSAN 622Mb Symmetrical PON interface.
pub const IFT_PON622: u8 = 0xd0;
/// Transparent bridge interface.
pub const IFT_BRIDGE: u8 = 0xd1;
/// Interface common to multiple lines.
pub const IFT_LINEGROUP: u8 = 0xd2;
/// Voice E&M Feature Group D.
pub const IFT_VOICEEMFGD: u8 = 0xd3;
/// Voice FGD Exchange Access North American.
pub const IFT_VOICEFGDEANA: u8 = 0xd4;
/// Voice Direct Inward Dialing.
pub const IFT_VOICEDID: u8 = 0xd5;
/// `IFT_GIF`.
pub const IFT_GIF: u8 = 0xf0;
/// `IFT_DUMMY`.
pub const IFT_DUMMY: u8 = 0xf1;
/// `IFT_PVC`.
pub const IFT_PVC: u8 = 0xf2;
/// `IFT_FAITH`.
pub const IFT_FAITH: u8 = 0xf3;
/// Encapsulation.
pub const IFT_ENC: u8 = 0xf4;
/// Packet filter logging.
pub const IFT_PFLOG: u8 = 0xf5;
/// Packet filter state syncing.
pub const IFT_PFSYNC: u8 = 0xf6;
/// Common Address Redundancy Protocol.
pub const IFT_CARP: u8 = 0xf7;
/// Bluetooth.
pub const IFT_BLUETOOTH: u8 = 0xf8;
/// Pflow.
pub const IFT_PFLOW: u8 = 0xf9;
/// Mobile Broadband Interface Model.
pub const IFT_MBIM: u8 = 0xfa;
/// WireGuard tunnel.
pub const IFT_WIREGUARD: u8 = 0xfb;

/// The `i16` at the start of a union's bytes.
const fn ru_i16(b: &[u8]) -> i16 {
    i16::from_ne_bytes([b[0], b[1]])
}

/// The `i32` at the start of a union's bytes.
const fn ru_i32(b: &[u8]) -> i32 {
    i32::from_ne_bytes([b[0], b[1], b[2], b[3]])
}

/// The `u32` at the start of a union's bytes.
const fn ru_u32(b: &[u8]) -> u32 {
    u32::from_ne_bytes([b[0], b[1], b[2], b[3]])
}

/// The `i64` at the start of a union's bytes.
const fn ru_i64(b: &[u8]) -> i64 {
    i64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// The `u64` at the start of a union's bytes.
const fn ru_u64(b: &[u8]) -> u64 {
    u64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// The first 16 bytes of a union as a `struct sockaddr`.
const fn ru_sockaddr(b: &[u8]) -> Sockaddr {
    let mut sa_data = [0u8; 14];
    let mut i = 0;
    while i < 14 {
        sa_data[i] = b[2 + i];
        i += 1;
    }
    Sockaddr {
        sa_len: b[0],
        sa_family: b[1],
        sa_data,
    }
}

/// `sa`'s 16 bytes written at the start of a union's bytes.
fn put_sockaddr(b: &mut [u8], sa: &Sockaddr) {
    b[0] = sa.sa_len;
    b[1] = sa.sa_family;
    b[2..16].copy_from_slice(&sa.sa_data);
}

/// `struct if_clonereq`: the cloners' names (`SIOCIFGCLONERS`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfClonereq {
    /// Total cloners (out).
    pub ifcr_total: i32,
    /// Room for this many in user buffer.
    pub ifcr_count: i32,
    /// Buffer for cloner names (a user address).
    pub ifcr_buffer: u64,
}

crate::abi_pod!(IfClonereq {
    ifcr_total: i32,
    ifcr_count: i32,
    ifcr_buffer: u64,
});

/// `struct if_rxring`: the accounting of a driver's receive ring (embedded in a softc, so its
/// all-zero value is an unused ring).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfRxring {
    /// `rxr_adjusted`.
    pub rxr_adjusted: i32,
    /// `rxr_alive`: number of descriptors on the ring.
    pub rxr_alive: u32,
    /// `rxr_cwm`: current water mark.
    pub rxr_cwm: u32,
    /// `rxr_lwm`: low water mark.
    pub rxr_lwm: u32,
    /// `rxr_hwm`: high water mark.
    pub rxr_hwm: u32,
}

crate::abi_pod!(IfRxring {
    rxr_adjusted: i32,
    rxr_alive: u32,
    rxr_cwm: u32,
    rxr_lwm: u32,
    rxr_hwm: u32,
});

/// `struct if_rxring_info`: one ring of `SIOCGIFRXR`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfRxringInfo {
    /// `ifr_name`: name of the ring.
    pub ifr_name: [u8; 16],
    /// `ifr_size`: size of the packets on the ring.
    pub ifr_size: u32,
    /// `ifr_info`.
    pub ifr_info: IfRxring,
}

crate::abi_pod!(IfRxringInfo {
    ifr_name: [u8; 16],
    ifr_size: u32,
    ifr_info: IfRxring,
});

/// `struct if_rxrinfo`: the rings of `SIOCGIFRXR`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfRxrinfo {
    /// `ifri_total`.
    pub ifri_total: u32,
    /// The hole before the pointer.
    pub _pad0: u32,
    /// `ifri_entries` (a user address of `ifri_total` [`IfRxringInfo`]s).
    pub ifri_entries: u64,
}

crate::abi_pod!(IfRxrinfo {
    ifri_total: u32,
    _pad0: u32,
    ifri_entries: u64,
});

/// `struct timeval`'s image inside [`IfData`] (`time_t`, `suseconds_t`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfTimeval {
    /// `tv_sec`.
    pub tv_sec: i64,
    /// `tv_usec`.
    pub tv_usec: i64,
}

crate::abi_pod!(IfTimeval {
    tv_sec: i64,
    tv_usec: i64,
});

/// `struct if_data`: the interface's statistics and information, `SIOCGIFDATA`'s and the
/// routing socket's.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfData {
    /// Ethernet, tokenring, etc.
    pub ifi_type: u8,
    /// Media address length.
    pub ifi_addrlen: u8,
    /// Media header length.
    pub ifi_hdrlen: u8,
    /// Current link state.
    pub ifi_link_state: u8,
    /// Maximum transmission unit.
    pub ifi_mtu: u32,
    /// Routing metric (external only).
    pub ifi_metric: u32,
    /// Routing instance.
    pub ifi_rdomain: u32,
    /// Linespeed.
    pub ifi_baudrate: u64,
    /// Packets received on interface.
    pub ifi_ipackets: u64,
    /// Input errors on interface.
    pub ifi_ierrors: u64,
    /// Packets sent on interface.
    pub ifi_opackets: u64,
    /// Output errors on interface.
    pub ifi_oerrors: u64,
    /// Collisions on csma interfaces.
    pub ifi_collisions: u64,
    /// Total number of octets received.
    pub ifi_ibytes: u64,
    /// Total number of octets sent.
    pub ifi_obytes: u64,
    /// Packets received via multicast.
    pub ifi_imcasts: u64,
    /// Packets sent via multicast.
    pub ifi_omcasts: u64,
    /// Dropped on input, this interface.
    pub ifi_iqdrops: u64,
    /// Dropped on output, this interface.
    pub ifi_oqdrops: u64,
    /// Destined for unsupported protocol.
    pub ifi_noproto: u64,
    /// Interface capabilities.
    pub ifi_capabilities: u32,
    /// The hole before the time.
    pub _pad0: u32,
    /// Last operational state change.
    pub ifi_lastchange: IfTimeval,
}

crate::abi_pod!(IfData {
    ifi_type: u8,
    ifi_addrlen: u8,
    ifi_hdrlen: u8,
    ifi_link_state: u8,
    ifi_mtu: u32,
    ifi_metric: u32,
    ifi_rdomain: u32,
    ifi_baudrate: u64,
    ifi_ipackets: u64,
    ifi_ierrors: u64,
    ifi_opackets: u64,
    ifi_oerrors: u64,
    ifi_collisions: u64,
    ifi_ibytes: u64,
    ifi_obytes: u64,
    ifi_imcasts: u64,
    ifi_omcasts: u64,
    ifi_iqdrops: u64,
    ifi_oqdrops: u64,
    ifi_noproto: u64,
    ifi_capabilities: u32,
    _pad0: u32,
    ifi_lastchange: IfTimeval,
});

/// `struct if_status_description`: the text of a link state for a type of interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IfStatusDescription {
    /// `ifs_type`: the interface type, 0 for any.
    pub ifs_type: u8,
    /// `ifs_state`: the link state.
    pub ifs_state: u8,
    /// `ifs_string`.
    pub ifs_string: &'static [u8],
}

/// `struct if_msghdr`: the routing socket's message about an interface (`RTM_IFINFO`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfMsghdr {
    /// To skip over non-understood messages.
    pub ifm_msglen: u16,
    /// Future binary compatibility.
    pub ifm_version: u8,
    /// Message type.
    pub ifm_type: u8,
    /// Sizeof(if_msghdr).
    pub ifm_hdrlen: u16,
    /// Index for associated ifp.
    pub ifm_index: u16,
    /// Routing table id.
    pub ifm_tableid: u16,
    /// `ifm_pad1`.
    pub ifm_pad1: u8,
    /// `ifm_pad2`.
    pub ifm_pad2: u8,
    /// Like rtm_addrs.
    pub ifm_addrs: i32,
    /// Value of if_flags.
    pub ifm_flags: i32,
    /// Extra flags.
    pub ifm_xflags: i32,
    /// Statistics and other data about if.
    pub ifm_data: IfData,
}

crate::abi_pod!(IfMsghdr {
    ifm_msglen: u16,
    ifm_version: u8,
    ifm_type: u8,
    ifm_hdrlen: u16,
    ifm_index: u16,
    ifm_tableid: u16,
    ifm_pad1: u8,
    ifm_pad2: u8,
    ifm_addrs: i32,
    ifm_flags: i32,
    ifm_xflags: i32,
    ifm_data: IfData,
});

/// `struct ifa_msghdr`: the routing socket's message about an interface address
/// (`RTM_NEWADDR`, `RTM_DELADDR`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfaMsghdr {
    /// To skip over non-understood messages.
    pub ifam_msglen: u16,
    /// Future binary compatibility.
    pub ifam_version: u8,
    /// Message type.
    pub ifam_type: u8,
    /// Sizeof(ifa_msghdr).
    pub ifam_hdrlen: u16,
    /// Index for associated ifp.
    pub ifam_index: u16,
    /// Routing table id.
    pub ifam_tableid: u16,
    /// `ifam_pad1`.
    pub ifam_pad1: u8,
    /// `ifam_pad2`.
    pub ifam_pad2: u8,
    /// Like rtm_addrs.
    pub ifam_addrs: i32,
    /// Value of ifa_flags.
    pub ifam_flags: i32,
    /// Value of ifa_metric.
    pub ifam_metric: i32,
}

crate::abi_pod!(IfaMsghdr {
    ifam_msglen: u16,
    ifam_version: u8,
    ifam_type: u8,
    ifam_hdrlen: u16,
    ifam_index: u16,
    ifam_tableid: u16,
    ifam_pad1: u8,
    ifam_pad2: u8,
    ifam_addrs: i32,
    ifam_flags: i32,
    ifam_metric: i32,
});

/// `struct if_announcemsghdr`: the routing socket's message about an interface's arrival or
/// departure (`RTM_IFANNOUNCE`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfAnnouncemsghdr {
    /// To skip over non-understood messages.
    pub ifan_msglen: u16,
    /// Future binary compatibility.
    pub ifan_version: u8,
    /// Message type.
    pub ifan_type: u8,
    /// Sizeof(if_announcemsghdr).
    pub ifan_hdrlen: u16,
    /// Index for associated ifp.
    pub ifan_index: u16,
    /// What type of announcement.
    pub ifan_what: u16,
    /// If name, e.g. "en0".
    pub ifan_name: [u8; IFNAMSIZ],
}

crate::abi_pod!(IfAnnouncemsghdr {
    ifan_msglen: u16,
    ifan_version: u8,
    ifan_type: u8,
    ifan_hdrlen: u16,
    ifan_index: u16,
    ifan_what: u16,
    ifan_name: [u8; IFNAMSIZ],
});

/// `struct if_ieee80211_data`: the 802.11 part of `RTM_80211INFO`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfIeee80211Data {
    /// Channel number.
    pub ifie_channel: u8,
    /// Length of the network id.
    pub ifie_nwid_len: u8,
    /// The hole before the flags.
    pub _pad0: [u8; 2],
    /// `ifie_flags`.
    pub ifie_flags: u32,
    /// `ifie_xflags`.
    pub ifie_xflags: u32,
    /// Network id.
    pub ifie_nwid: [u8; 32],
    /// Access point's address.
    pub ifie_addr: [u8; 6],
    /// The hole at the end.
    pub _pad1: [u8; 2],
}

crate::abi_pod!(IfIeee80211Data {
    ifie_channel: u8,
    ifie_nwid_len: u8,
    _pad0: [u8; 2],
    ifie_flags: u32,
    ifie_xflags: u32,
    ifie_nwid: [u8; 32],
    ifie_addr: [u8; 6],
    _pad1: [u8; 2],
});

/// `struct if_ieee80211_msghdr`: `RTM_80211INFO`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfIeee80211Msghdr {
    /// To skip over non-understood messages.
    pub ifim_msglen: u16,
    /// Future binary compatibility.
    pub ifim_version: u8,
    /// Message type.
    pub ifim_type: u8,
    /// Sizeof(if_ieee80211_msghdr).
    pub ifim_hdrlen: u16,
    /// Index for associated ifp.
    pub ifim_index: u16,
    /// Routing table id.
    pub ifim_tableid: u16,
    /// The hole before the data.
    pub _pad0: u16,
    /// `ifim_ifie`.
    pub ifim_ifie: IfIeee80211Data,
}

crate::abi_pod!(IfIeee80211Msghdr {
    ifim_msglen: u16,
    ifim_version: u8,
    ifim_type: u8,
    ifim_hdrlen: u16,
    ifim_index: u16,
    ifim_tableid: u16,
    _pad0: u16,
    ifim_ifie: IfIeee80211Data,
});

/// `struct if_nameindex_msg`: an index and its name, `NET_RT_IFNAMES`'s entries.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfNameindexMsg {
    /// `if_index`.
    pub if_index: u32,
    /// `if_name`.
    pub if_name: [u8; IFNAMSIZ],
}

crate::abi_pod!(IfNameindexMsg {
    if_index: u32,
    if_name: [u8; IFNAMSIZ],
});

/// `struct ifg_req`: one group or one member of `SIOCGIFGROUP`/`SIOCGIFGMEMB`; the union of
/// `ifgrq_group` and `ifgrq_member` (one name either way) is `ifgrq_ifgrqu`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfgReq {
    /// `ifgrq_ifgrqu`: the group's or the member's name.
    pub ifgrq_ifgrqu: [u8; IFNAMSIZ],
}

crate::abi_pod!(IfgReq {
    ifgrq_ifgrqu: [u8; IFNAMSIZ],
});

impl IfgReq {
    /// `ifgrq_group`.
    pub fn ifgrq_group(&self) -> &[u8; IFNAMSIZ] {
        &self.ifgrq_ifgrqu
    }

    /// `ifgrq_member`.
    pub fn ifgrq_member(&self) -> &[u8; IFNAMSIZ] {
        &self.ifgrq_ifgrqu
    }
}

/// `struct ifg_attrib`: a group's attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfgAttrib {
    /// `ifg_carp_demoted`.
    pub ifg_carp_demoted: i32,
}

crate::abi_pod!(IfgAttrib {
    ifg_carp_demoted: i32,
});

/// `struct ifgroupreq`: the group requests (`SIOCAIFGROUP`, `SIOCGIFGROUP`, ...); the union of
/// `ifgru_group`, `ifgru_groups` and `ifgru_attrib` is `ifgr_ifgru`.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifgroupreq {
    /// If name, e.g. "en0".
    pub ifgr_name: [u8; IFNAMSIZ],
    /// `ifgr_len`.
    pub ifgr_len: u32,
    /// The hole before the union.
    pub _pad0: u32,
    /// `ifgr_ifgru`.
    pub ifgr_ifgru: [u8; 16],
}

crate::abi_pod!(Ifgroupreq {
    ifgr_name: [u8; IFNAMSIZ],
    ifgr_len: u32,
    _pad0: u32,
    ifgr_ifgru: [u8; 16],
});

impl Ifgroupreq {
    /// `ifgr_group`.
    pub fn ifgr_group(&self) -> &[u8; IFNAMSIZ] {
        &self.ifgr_ifgru
    }

    /// `ifgr_group`, to write.
    pub fn ifgr_group_mut(&mut self) -> &mut [u8; IFNAMSIZ] {
        &mut self.ifgr_ifgru
    }

    /// `ifgr_groups`: the user address of the [`IfgReq`] array.
    pub fn ifgr_groups(&self) -> u64 {
        ru_u64(&self.ifgr_ifgru)
    }

    /// `ifgr_attrib`.
    pub fn ifgr_attrib(&self) -> IfgAttrib {
        IfgAttrib {
            ifg_carp_demoted: ru_i32(&self.ifgr_ifgru),
        }
    }

    /// Sets `ifgr_attrib`.
    pub fn set_ifgr_attrib(&mut self, a: IfgAttrib) {
        self.ifgr_ifgru[..4].copy_from_slice(&a.ifg_carp_demoted.to_ne_bytes());
    }
}

/// Defines the `ifr_*` shorthands for a scalar member of `ifr_ifru`.
macro_rules! ifr_accessor {
    ($($(#[$doc:meta])* $get:ident, $set:ident => $ty:ty, $read:ident;)*) => {
        $(
            $(#[$doc])*
            pub fn $get(&self) -> $ty {
                $read(&self.ifr_ifru)
            }

            $(#[$doc])*
            pub fn $set(&mut self, v: $ty) {
                let b = v.to_ne_bytes();
                self.ifr_ifru[..b.len()].copy_from_slice(&b);
            }
        )*
    };
}

/// `struct ifreq`: the interface request of most `SIOC*` commands; the union `ifr_ifru` (an
/// address, the flags, the metric, a media word, a user pointer, an index) is its 16 bytes.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifreq {
    /// If name, e.g. "en0".
    pub ifr_name: [u8; IFNAMSIZ],
    /// `ifr_ifru`.
    pub ifr_ifru: [u8; 16],
}

crate::abi_pod!(Ifreq {
    ifr_name: [u8; IFNAMSIZ],
    ifr_ifru: [u8; 16],
});

/// A union's first 16 bytes as the `sockaddr` its address members are (a `sockaddr` is 16
/// bytes of alignment 1, so the view always fits).
fn sockaddr_mut(u: &mut [u8]) -> &mut Sockaddr {
    match crate::pod::view_mut::<Sockaddr>(u, 0) {
        Some(sa) => sa,
        None => crate::assert::hal_panic(format_args!("a union shorter than a sockaddr")),
    }
}

impl Ifreq {
    /// An all-zero request.
    pub const fn zeroed() -> Self {
        Self {
            ifr_name: [0; IFNAMSIZ],
            ifr_ifru: [0; 16],
        }
    }

    /// `ifr_addr`: address.
    pub const fn ifr_addr(&self) -> Sockaddr {
        ru_sockaddr(&self.ifr_ifru)
    }

    /// Sets `ifr_addr`.
    pub fn set_ifr_addr(&mut self, sa: &Sockaddr) {
        put_sockaddr(&mut self.ifr_ifru, sa);
    }

    /// `ifr_dstaddr`: other end of p-to-p link.
    pub const fn ifr_dstaddr(&self) -> Sockaddr {
        ru_sockaddr(&self.ifr_ifru)
    }

    /// Sets `ifr_dstaddr`.
    pub fn set_ifr_dstaddr(&mut self, sa: &Sockaddr) {
        put_sockaddr(&mut self.ifr_ifru, sa);
    }

    /// `ifr_broadaddr`: broadcast address.
    pub const fn ifr_broadaddr(&self) -> Sockaddr {
        ru_sockaddr(&self.ifr_ifru)
    }

    /// Sets `ifr_broadaddr`.
    pub fn set_ifr_broadaddr(&mut self, sa: &Sockaddr) {
        put_sockaddr(&mut self.ifr_ifru, sa);
    }

    /// `&ifr->ifr_addr`, to write in place (the union's 16 bytes viewed as a `sockaddr`).
    pub fn ifr_addr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifr_ifru)
    }

    /// `&ifr->ifr_dstaddr`, to write in place.
    pub fn ifr_dstaddr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifr_ifru)
    }

    /// `&ifr->ifr_broadaddr`, to write in place.
    pub fn ifr_broadaddr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifr_ifru)
    }

    ifr_accessor! {
        /// `ifr_flags`: flags.
        ifr_flags, set_ifr_flags => i16, ru_i16;
        /// `ifr_metric`: metric.
        ifr_metric, set_ifr_metric => i32, ru_i32;
        /// `ifr_mtu`: mtu (overload).
        ifr_mtu, set_ifr_mtu => i32, ru_i32;
        /// `ifr_hardmtu`: hardmtu (overload).
        ifr_hardmtu, set_ifr_hardmtu => i32, ru_i32;
        /// `ifr_media`: media options.
        ifr_media, set_ifr_media => u64, ru_u64;
        /// `ifr_rdomainid`: VRF instance (overload).
        ifr_rdomainid, set_ifr_rdomainid => i32, ru_i32;
        /// `ifr_vnetid`: Virtual Net Id.
        ifr_vnetid, set_ifr_vnetid => i64, ru_i64;
        /// `ifr_ttl`: tunnel TTL (overload).
        ifr_ttl, set_ifr_ttl => i32, ru_i32;
        /// `ifr_df`: tunnel DF (overload).
        ifr_df, set_ifr_df => i32, ru_i32;
        /// `ifr_data`: for use by interface (a user address).
        ifr_data, set_ifr_data => u64, ru_u64;
        /// `ifr_index`: interface index.
        ifr_index, set_ifr_index => u32, ru_u32;
        /// `ifr_llprio`: link layer priority.
        ifr_llprio, set_ifr_llprio => i32, ru_i32;
        /// `ifr_hdrprio`: header prio field config.
        ifr_hdrprio, set_ifr_hdrprio => i32, ru_i32;
        /// `ifr_pwe3`: PWE3 type.
        ifr_pwe3, set_ifr_pwe3 => i32, ru_i32;
    }
}

/// `struct ifaliasreq`: `SIOCAIFADDR`'s request; the union of `ifra_addr` and `ifra_align`
/// is `ifra_ifrau`.
#[repr(C, align(4))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifaliasreq {
    /// If name, e.g. "en0".
    pub ifra_name: [u8; IFNAMSIZ],
    /// `ifra_ifrau`.
    pub ifra_ifrau: [u8; 16],
    /// `ifra_dstaddr` (`ifra_broadaddr`).
    pub ifra_dstaddr: Sockaddr,
    /// `ifra_mask`.
    pub ifra_mask: Sockaddr,
}

crate::abi_pod!(Ifaliasreq {
    ifra_name: [u8; IFNAMSIZ],
    ifra_ifrau: [u8; 16],
    ifra_dstaddr: Sockaddr,
    ifra_mask: Sockaddr,
});

impl Ifaliasreq {
    /// `ifra_addr`.
    pub const fn ifra_addr(&self) -> Sockaddr {
        ru_sockaddr(&self.ifra_ifrau)
    }

    /// Sets `ifra_addr`.
    pub fn set_ifra_addr(&mut self, sa: &Sockaddr) {
        put_sockaddr(&mut self.ifra_ifrau, sa);
    }

    /// `&ifra->ifra_addr`, to write in place.
    pub fn ifra_addr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifra_ifrau)
    }

    /// `ifra_broadaddr`: the `ifra_dstaddr` member.
    pub const fn ifra_broadaddr(&self) -> &Sockaddr {
        &self.ifra_dstaddr
    }
}

/// `struct ifmediareq`: `SIOCGIFMEDIA`'s request.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifmediareq {
    /// If name, e.g. "en0".
    pub ifm_name: [u8; IFNAMSIZ],
    /// Current media options.
    pub ifm_current: u64,
    /// Don't care mask.
    pub ifm_mask: u64,
    /// Media status.
    pub ifm_status: u64,
    /// Active options.
    pub ifm_active: u64,
    /// # entries in ifm_ulist array.
    pub ifm_count: i32,
    /// The hole before the pointer.
    pub _pad0: u32,
    /// `ifm_ulist`: media words (a user address).
    pub ifm_ulist: u64,
}

crate::abi_pod!(Ifmediareq {
    ifm_name: [u8; IFNAMSIZ],
    ifm_current: u64,
    ifm_mask: u64,
    ifm_status: u64,
    ifm_active: u64,
    ifm_count: i32,
    _pad0: u32,
    ifm_ulist: u64,
});

/// `struct ifkalivereq`: `SIOCSETKALIVE`/`SIOCGETKALIVE`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifkalivereq {
    /// If name, e.g. "en0".
    pub ikar_name: [u8; IFNAMSIZ],
    /// `ikar_timeo`.
    pub ikar_timeo: i32,
    /// `ikar_cnt`.
    pub ikar_cnt: i32,
}

crate::abi_pod!(Ifkalivereq {
    ikar_name: [u8; IFNAMSIZ],
    ikar_timeo: i32,
    ikar_cnt: i32,
});

/// `struct ifconf`: `SIOCGIFCONF`'s request; the union of `ifc_buf` and `ifc_req` (one user
/// address either way) is `ifc_ifcu`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ifconf {
    /// Size of associated buffer.
    pub ifc_len: i32,
    /// The hole before the pointer.
    pub _pad0: u32,
    /// `ifc_ifcu`.
    pub ifc_ifcu: u64,
}

crate::abi_pod!(Ifconf {
    ifc_len: i32,
    _pad0: u32,
    ifc_ifcu: u64,
});

impl Ifconf {
    /// `ifc_buf`: buffer address.
    pub const fn ifc_buf(&self) -> u64 {
        self.ifc_ifcu
    }

    /// `ifc_req`: array of structures returned (the same address).
    pub const fn ifc_req(&self) -> u64 {
        self.ifc_ifcu
    }
}

/// `struct if_laddrreq`: `SIOCSLIFPHYADDR`/`SIOCGLIFPHYADDR`, the tunnel's addresses.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IfLaddrreq {
    /// If name, e.g. "gre1".
    pub iflr_name: [u8; IFNAMSIZ],
    /// In/out parameter.
    pub flags: u32,
    /// In/out parameter.
    pub prefixlen: u32,
    /// In/out parameter.
    pub addr: SockaddrStorage,
    /// Out parameter.
    pub dstaddr: SockaddrStorage,
}

crate::abi_pod!(IfLaddrreq {
    iflr_name: [u8; IFNAMSIZ],
    flags: u32,
    prefixlen: u32,
    addr: SockaddrStorage,
    dstaddr: SockaddrStorage,
});

/// `struct if_afreq`: `SIOCIFAFATTACH`/`SIOCIFAFDETACH`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfAfreq {
    /// `ifar_name`.
    pub ifar_name: [u8; IFNAMSIZ],
    /// `ifar_af`.
    pub ifar_af: SaFamily,
}

crate::abi_pod!(IfAfreq {
    ifar_name: [u8; IFNAMSIZ],
    ifar_af: SaFamily,
});

/// `struct if_parent`: `SIOCSIFPARENT`/`SIOCGIFPARENT`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfParent {
    /// `ifp_name`.
    pub ifp_name: [u8; IFNAMSIZ],
    /// `ifp_parent`.
    pub ifp_parent: [u8; IFNAMSIZ],
}

crate::abi_pod!(IfParent {
    ifp_name: [u8; IFNAMSIZ],
    ifp_parent: [u8; IFNAMSIZ],
});

/// `struct if_sffpage`: `SIOCGIFSFFPAGE`, a page of a transceiver's EEPROM.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IfSffpage {
    /// `sff_ifname`.
    pub sff_ifname: [u8; IFNAMSIZ],
    /// `sff_addr`: i2c address of the module.
    pub sff_addr: u8,
    /// `sff_page`: the page.
    pub sff_page: u8,
    /// `sff_data`.
    pub sff_data: [u8; IFSFF_DATA_LEN],
}

crate::abi_pod!(IfSffpage {
    sff_ifname: [u8; IFNAMSIZ],
    sff_addr: u8,
    sff_page: u8,
    sff_data: [u8; IFSFF_DATA_LEN],
});

/// `struct sockaddr_dl`: structure of a link-level sockaddr. `sdl_data` is a minimum work
/// area: an interface's own (`if_alloc_sadl`) is longer, the name and the address after the
/// header, so [`SockaddrDl::lladdr`] reads the whole address's bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SockaddrDl {
    /// Total length of sockaddr.
    pub sdl_len: u8,
    /// `AF_LINK`.
    pub sdl_family: u8,
    /// If != 0, system given index for interface.
    pub sdl_index: u16,
    /// Interface type.
    pub sdl_type: u8,
    /// Interface name length, no trailing 0 reqd.
    pub sdl_nlen: u8,
    /// Link level address length.
    pub sdl_alen: u8,
    /// Link layer selector length, mostly 0.
    pub sdl_slen: u8,
    /// Minimum work area, can be larger; contains both if name and ll address; big enough for
    /// `IFNAMSIZ` plus 8byte ll addr.
    pub sdl_data: [u8; 24],
}

crate::abi_pod!(SockaddrDl {
    sdl_len: u8,
    sdl_family: u8,
    sdl_index: u16,
    sdl_type: u8,
    sdl_nlen: u8,
    sdl_alen: u8,
    sdl_slen: u8,
    sdl_data: [u8; 24],
});

/// The offset of `sdl_data` in a `sockaddr_dl`.
pub const SDL_DATA_OFFSET: usize = core::mem::offset_of!(SockaddrDl, sdl_data);

impl SockaddrDl {
    /// `LLADDR(s)`: the link-level address of the `sockaddr_dl` whose bytes `sdl` holds (its
    /// `sdl_alen` bytes after the name); `None` when `sdl` is shorter than the header says.
    pub fn lladdr(sdl: &[u8]) -> Option<&[u8]> {
        let nlen = usize::from(*sdl.get(5)?);
        let alen = usize::from(*sdl.get(6)?);
        let start = SDL_DATA_OFFSET + nlen;
        sdl.get(start..start + alen)
    }

    /// `LLADDR(s)`, to write.
    pub fn lladdr_mut(sdl: &mut [u8]) -> Option<&mut [u8]> {
        let nlen = usize::from(*sdl.get(5)?);
        let alen = usize::from(*sdl.get(6)?);
        let start = SDL_DATA_OFFSET + nlen;
        sdl.get_mut(start..start + alen)
    }
}

/// `LINK_STATE_DESCRIPTIONS`: the text of each link state, the type-specific ones first.
pub const LINK_STATE_DESCRIPTIONS: &[IfStatusDescription] = &[
    lsd(IFT_ETHER, LINK_STATE_DOWN, b"no carrier"),
    lsd(IFT_IEEE80211, LINK_STATE_DOWN, b"no network"),
    lsd(IFT_PPP, LINK_STATE_DOWN, b"no carrier"),
    lsd(IFT_CARP, LINK_STATE_DOWN, b"backup"),
    lsd(IFT_CARP, LINK_STATE_UP, b"master"),
    lsd(IFT_CARP, LINK_STATE_HALF_DUPLEX, b"master"),
    lsd(IFT_CARP, LINK_STATE_FULL_DUPLEX, b"master"),
    lsd(0, LINK_STATE_UP, b"active"),
    lsd(0, LINK_STATE_HALF_DUPLEX, b"active"),
    lsd(0, LINK_STATE_FULL_DUPLEX, b"active"),
    lsd(0, LINK_STATE_UNKNOWN, b"unknown"),
    lsd(0, LINK_STATE_INVALID, b"invalid"),
    lsd(0, LINK_STATE_DOWN, b"down"),
    lsd(0, LINK_STATE_KALIVE_DOWN, b"keepalive down"),
];

/// One entry of [`LINK_STATE_DESCRIPTIONS`].
const fn lsd(ifs_type: u8, ifs_state: u8, ifs_string: &'static [u8]) -> IfStatusDescription {
    IfStatusDescription {
        ifs_type,
        ifs_state,
        ifs_string,
    }
}

/// `IFQ_PRIO2TOS(p)`: a queue priority as the precedence bits of an IP type of service.
pub const fn ifq_prio2tos(p: u32) -> u32 {
    p << 5
}

/// `IFQ_TOS2PRIO(t)`: the queue priority of an IP type of service.
pub const fn ifq_tos2prio(t: u32) -> u32 {
    t >> 5
}

/// `LINK_STATE_IS_UP(s)`: whether link state `s` counts as up (unknown does).
pub const fn link_state_is_up(s: u8) -> bool {
    s >= LINK_STATE_UP || s == LINK_STATE_UNKNOWN
}

/// `LINK_STATE_DESC_MATCH(ifs, t, s)`: whether description `ifs` applies to type `t` in state
/// `s`.
pub const fn link_state_desc_match(ifs: &IfStatusDescription, t: u8, s: u8) -> bool {
    (ifs.ifs_type == t || ifs.ifs_type == 0) && ifs.ifs_state == s
}

/// `IF_Kbps(x)`: kilobits/sec.
pub const fn if_kbps(x: u64) -> u64 {
    x * 1000
}

/// `IF_Mbps(x)`: megabits/sec.
pub const fn if_mbps(x: u64) -> u64 {
    if_kbps(x * 1000)
}

/// `IF_Gbps(x)`: gigabits/sec.
pub const fn if_gbps(x: u64) -> u64 {
    if_mbps(x * 1000)
}

// LP64 sizes of the C structures (amd64 and arm64 alike).
const _: () = {
    assert!(size_of::<IfClonereq>() == 16);
    assert!(size_of::<IfRxring>() == 20);
    assert!(size_of::<IfRxringInfo>() == 40);
    assert!(size_of::<IfRxrinfo>() == 16);
    assert!(size_of::<IfData>() == 144);
    assert!(core::mem::offset_of!(IfData, ifi_lastchange) == 128);
    assert!(size_of::<IfMsghdr>() == 168);
    assert!(size_of::<IfaMsghdr>() == 24);
    assert!(size_of::<IfAnnouncemsghdr>() == 26);
    assert!(size_of::<IfIeee80211Data>() == 52);
    assert!(size_of::<IfIeee80211Msghdr>() == 64);
    assert!(size_of::<IfNameindexMsg>() == 20);
    assert!(size_of::<IfgReq>() == 16);
    assert!(size_of::<Ifgroupreq>() == 40);
    assert!(align_of::<Ifgroupreq>() == 8);
    assert!(size_of::<Ifreq>() == 32);
    assert!(align_of::<Ifreq>() == 8);
    assert!(size_of::<Ifaliasreq>() == 64);
    assert!(size_of::<Ifmediareq>() == 64);
    assert!(size_of::<Ifkalivereq>() == 24);
    assert!(size_of::<Ifconf>() == 16);
    assert!(size_of::<IfLaddrreq>() == 536);
    assert!(size_of::<IfAfreq>() == 17);
    assert!(size_of::<IfParent>() == 32);
    assert!(size_of::<IfSffpage>() == 274);
    assert!(size_of::<SockaddrDl>() == 32);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ifreq_shorthands_share_the_unions_first_bytes() {
        let mut ifr = Ifreq::zeroed();
        ifr.set_ifr_flags(0x0843);
        assert_eq!(ifr.ifr_flags(), 0x0843);
        assert_eq!(&ifr.ifr_ifru[..2], &0x0843i16.to_ne_bytes());
        ifr.set_ifr_metric(-7);
        assert_eq!(
            (ifr.ifr_metric(), ifr.ifr_mtu(), ifr.ifr_rdomainid()),
            (-7, -7, -7)
        );
        ifr.set_ifr_media(0x1234_5678_9abc_def0);
        assert_eq!(ifr.ifr_media(), 0x1234_5678_9abc_def0);
        assert_eq!(ifr.ifr_data(), 0x1234_5678_9abc_def0);
        ifr.set_ifr_index(3);
        assert_eq!(ifr.ifr_index(), 3);
        let sa = Sockaddr {
            sa_len: 16,
            sa_family: 2,
            sa_data: [9; 14],
        };
        ifr.set_ifr_addr(&sa);
        assert_eq!(ifr.ifr_addr(), sa);
        assert_eq!(ifr.ifr_dstaddr(), sa);
        assert_eq!(ifr.ifr_broadaddr(), sa);
        assert_eq!(ifr.ifr_flags(), i16::from_ne_bytes([16, 2]));
    }

    #[test]
    fn the_other_unions_are_their_members_bytes() {
        let mut ifra = Ifaliasreq::default();
        let sa = Sockaddr {
            sa_len: 16,
            sa_family: 2,
            sa_data: [1; 14],
        };
        ifra.set_ifra_addr(&sa);
        assert_eq!(ifra.ifra_addr(), sa);
        ifra.ifra_dstaddr.sa_len = 5;
        assert_eq!(ifra.ifra_broadaddr().sa_len, 5);

        let mut ifgr = Ifgroupreq::default();
        ifgr.ifgr_group_mut()[..3].copy_from_slice(b"all");
        assert_eq!(&ifgr.ifgr_group()[..3], b"all");
        ifgr.set_ifgr_attrib(IfgAttrib {
            ifg_carp_demoted: 42,
        });
        assert_eq!(ifgr.ifgr_attrib().ifg_carp_demoted, 42);

        let ifc = Ifconf {
            ifc_len: 64,
            _pad0: 0,
            ifc_ifcu: 0x7f7f_0000,
        };
        assert_eq!((ifc.ifc_buf(), ifc.ifc_req()), (0x7f7f_0000, 0x7f7f_0000));
    }

    #[test]
    fn lladdr_follows_the_name() {
        let mut sdl = SockaddrDl {
            sdl_len: 32,
            sdl_family: 18,
            sdl_nlen: 4,
            sdl_alen: 6,
            ..SockaddrDl::default()
        };
        sdl.sdl_data[..10].copy_from_slice(b"vio0\x52\x54\x00\x12\x34\x56");
        let mut b = [0u8; 32];
        assert!(crate::pod::put(&mut b, 0, &sdl));
        assert_eq!(
            SockaddrDl::lladdr(&b),
            Some(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56][..])
        );
        SockaddrDl::lladdr_mut(&mut b)
            .unwrap()
            .copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(
            &b[SDL_DATA_OFFSET + 4..SDL_DATA_OFFSET + 10],
            &[1, 2, 3, 4, 5, 6]
        );
        // A header that names more bytes than there are.
        b[6] = 200;
        assert_eq!(SockaddrDl::lladdr(&b), None);
        assert_eq!(SockaddrDl::lladdr(&b[..3]), None);
    }

    #[test]
    fn link_state_descriptions_match_as_the_c() {
        let desc = |t, s| {
            LINK_STATE_DESCRIPTIONS
                .iter()
                .find(|d| link_state_desc_match(d, t, s))
                .map(|d| d.ifs_string)
        };
        assert_eq!(desc(IFT_ETHER, LINK_STATE_DOWN), Some(&b"no carrier"[..]));
        assert_eq!(desc(IFT_LOOP, LINK_STATE_DOWN), Some(&b"down"[..]));
        assert_eq!(desc(IFT_CARP, LINK_STATE_UP), Some(&b"master"[..]));
        assert_eq!(
            desc(IFT_ETHER, LINK_STATE_FULL_DUPLEX),
            Some(&b"active"[..])
        );
        assert!(link_state_is_up(LINK_STATE_UNKNOWN) && link_state_is_up(LINK_STATE_UP));
        assert!(!link_state_is_up(LINK_STATE_DOWN) && !link_state_is_up(LINK_STATE_INVALID));
        assert_eq!(if_gbps(1), 1_000_000_000);
        assert_eq!((ifq_prio2tos(7), ifq_tos2prio(0xe0)), (0xe0, 7));
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn interface_constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/net/if.h");
        let ifq = crate::reftest::assert_defines!(defs; IFQ_NQUEUES, IFQ_MINPRIO, IFQ_MAXPRIO, IFQ_DEFPRIO);
        crate::reftest::assert_complete(&defs, "IFQ_", &ifq);
        let link = crate::reftest::assert_defines!(defs;
        LINK_STATE_UNKNOWN, LINK_STATE_INVALID, LINK_STATE_DOWN, LINK_STATE_KALIVE_DOWN,
        LINK_STATE_UP, LINK_STATE_HALF_DUPLEX, LINK_STATE_FULL_DUPLEX);
        // LINK_STATE_DESCRIPTIONS spans lines in the C.
        crate::reftest::assert_complete(
            &defs,
            "LINK_STATE_",
            &[&link[..], &["LINK_STATE_DESCRIPTIONS"]].concat(),
        );
        let iff = crate::reftest::assert_defines!(defs;
        IFF_UP, IFF_BROADCAST, IFF_DEBUG, IFF_LOOPBACK, IFF_POINTOPOINT, IFF_STATICARP,
        IFF_RUNNING, IFF_NOARP, IFF_PROMISC, IFF_ALLMULTI, IFF_OACTIVE, IFF_SIMPLEX, IFF_LINK0,
        IFF_LINK1, IFF_LINK2, IFF_MULTICAST);
        // IFF_CANTCHANGE spans lines in the C.
        assert_eq!(IFF_CANTCHANGE, 0x8e5a);
        crate::reftest::assert_complete(&defs, "IFF_", &[&iff[..], &["IFF_CANTCHANGE"]].concat());
        let ifxf = crate::reftest::assert_defines!(defs;
        IFXF_MPSAFE, IFXF_CLONED, IFXF_AUTOCONF6TEMP, IFXF_MPLS, IFXF_WOL, IFXF_AUTOCONF6,
        IFXF_INET6_NOSOII, IFXF_AUTOCONF4, IFXF_MONITOR, IFXF_LRO, IFXF_MBUF_64BIT);
        assert_eq!(IFXF_CANTCHANGE, 0x3);
        crate::reftest::assert_complete(
            &defs,
            "IFXF_",
            &[&ifxf[..], &["IFXF_CANTCHANGE"]].concat(),
        );
        let ifcap = crate::reftest::assert_defines!(defs;
        IFCAP_CSUM_IPv4, IFCAP_CSUM_TCPv4, IFCAP_CSUM_UDPv4, IFCAP_VLAN_MTU,
        IFCAP_VLAN_HWTAGGING, IFCAP_VLAN_HWOFFLOAD, IFCAP_CSUM_TCPv6, IFCAP_CSUM_UDPv6,
        IFCAP_TSOv4, IFCAP_TSOv6, IFCAP_LRO, IFCAP_WOL);
        assert_eq!(IFCAP_CSUM_MASK, 0x187);
        crate::reftest::assert_complete(
            &defs,
            "IFCAP_",
            &[&ifcap[..], &["IFCAP_CSUM_MASK"]].concat(),
        );
        let ifqctl = crate::reftest::assert_defines!(defs;
        IFQCTL_LEN, IFQCTL_MAXLEN, IFQCTL_DROPS, IFQCTL_CONGESTION, IFQCTL_MAXID);
        crate::reftest::assert_complete(&defs, "IFQCTL_", &ifqctl);
        let ifan = crate::reftest::assert_defines!(defs; IFAN_ARRIVAL, IFAN_DEPARTURE);
        crate::reftest::assert_complete(&defs, "IFAN_", &ifan);
        let hdrprio = crate::reftest::assert_defines!(defs;
        IF_HDRPRIO_MIN, IF_HDRPRIO_MAX, IF_HDRPRIO_PACKET, IF_HDRPRIO_PAYLOAD,
        IF_HDRPRIO_OUTER, IF_PWE3_ETHERNET, IF_PWE3_IP, IF_NAMESIZE, IF_MAX_VECTORS);
        crate::reftest::assert_complete(&defs, "IF_", &hdrprio);
        let sff = crate::reftest::assert_defines!(defs; IFSFF_ADDR_EEPROM, IFSFF_ADDR_DDM, IFSFF_DATA_LEN);
        crate::reftest::assert_complete(&defs, "IFSFF_", &sff);
        crate::reftest::assert_defines!(defs; MCLPOOLS, IFNAMSIZ, IFDESCRSIZE, IFLR_PREFIX);
        assert_eq!(defs["IFG_ALL"], "\"all\"");
        assert_eq!(defs["IFG_EGRESS"], "\"egress\"");
    }

    // net/if.c

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn interface_types_match_the_c_header() {
        let defs = crate::reftest::defines("sys/net/if_types.h");
        let ours = crate::reftest::assert_defines!(defs;
            IFT_OTHER, IFT_1822, IFT_HDH1822, IFT_X25DDN, IFT_X25, IFT_ETHER, IFT_ISO88023,
            IFT_ISO88024, IFT_ISO88025, IFT_ISO88026, IFT_STARLAN, IFT_P10, IFT_P80, IFT_HY,
            IFT_FDDI, IFT_LAPB, IFT_SDLC, IFT_T1, IFT_CEPT, IFT_ISDNBASIC, IFT_ISDNPRIMARY,
            IFT_PTPSERIAL, IFT_PPP, IFT_LOOP, IFT_EON, IFT_XETHER, IFT_NSIP, IFT_SLIP, IFT_ULTRA,
            IFT_DS3, IFT_SIP, IFT_FRELAY, IFT_RS232, IFT_PARA, IFT_ARCNET, IFT_ARCNETPLUS, IFT_ATM,
            IFT_MIOX25, IFT_SONET, IFT_X25PLE, IFT_ISO88022LLC, IFT_LOCALTALK, IFT_SMDSDXI,
            IFT_FRELAYDCE, IFT_V35, IFT_HSSI, IFT_HIPPI, IFT_MODEM, IFT_AAL5, IFT_SONETPATH,
            IFT_SONETVT, IFT_SMDSICIP, IFT_PROPVIRTUAL, IFT_PROPMUX, IFT_IEEE80212,
            IFT_FIBRECHANNEL, IFT_HIPPIINTERFACE, IFT_FRAMERELAYINTERCONNECT, IFT_AFLANE8023,
            IFT_AFLANE8025, IFT_CCTEMUL, IFT_FASTETHER, IFT_ISDN, IFT_V11, IFT_V36, IFT_G703AT64K,
            IFT_G703AT2MB, IFT_QLLC, IFT_FASTETHERFX, IFT_CHANNEL, IFT_IEEE80211,
            IFT_IBM370PARCHAN, IFT_ESCON, IFT_DLSW, IFT_ISDNS, IFT_ISDNU, IFT_LAPD, IFT_IPSWITCH,
            IFT_RSRB, IFT_ATMLOGICAL, IFT_DS0, IFT_DS0BUNDLE, IFT_BSC, IFT_ASYNC, IFT_CNR,
            IFT_ISO88025DTR, IFT_EPLRS, IFT_ARAP, IFT_PROPCNLS, IFT_HOSTPAD, IFT_TERMPAD,
            IFT_FRAMERELAYMPI, IFT_X213, IFT_ADSL, IFT_RADSL, IFT_SDSL, IFT_VDSL,
            IFT_ISO88025CRFPINT, IFT_MYRINET, IFT_VOICEEM, IFT_VOICEFXO, IFT_VOICEFXS,
            IFT_VOICEENCAP, IFT_VOICEOVERIP, IFT_ATMDXI, IFT_ATMFUNI, IFT_ATMIMA,
            IFT_PPPMULTILINKBUNDLE, IFT_IPOVERCDLC, IFT_IPOVERCLAW, IFT_STACKTOSTACK,
            IFT_VIRTUALIPADDRESS, IFT_MPC, IFT_IPOVERATM, IFT_ISO88025FIBER, IFT_TDLC,
            IFT_GIGABITETHERNET, IFT_HDLC, IFT_LAPF, IFT_V37, IFT_X25MLP, IFT_X25HUNTGROUP,
            IFT_TRANSPHDLC, IFT_INTERLEAVE, IFT_FAST, IFT_IP, IFT_DOCSCABLEMACLAYER,
            IFT_DOCSCABLEDOWNSTREAM, IFT_DOCSCABLEUPSTREAM, IFT_A12MPPSWITCH, IFT_TUNNEL,
            IFT_COFFEE, IFT_CES, IFT_ATMSUBINTERFACE, IFT_L2VLAN, IFT_L3IPVLAN, IFT_L3IPXVLAN,
            IFT_DIGITALPOWERLINE, IFT_MEDIAMAILOVERIP, IFT_DTM, IFT_DCN, IFT_IPFORWARD, IFT_MSDSL,
            IFT_IEEE1394, IFT_IFGSN, IFT_DVBRCCMACLAYER, IFT_DVBRCCDOWNSTREAM, IFT_DVBRCCUPSTREAM,
            IFT_ATMVIRTUAL, IFT_MPLSTUNNEL, IFT_SRP, IFT_VOICEOVERATM, IFT_VOICEOVERFRAMERELAY,
            IFT_IDSL, IFT_COMPOSITELINK, IFT_SS7SIGLINK, IFT_PROPWIRELESSP2P, IFT_FRFORWARD,
            IFT_RFC1483, IFT_USB, IFT_IEEE8023ADLAG, IFT_BGPPOLICYACCOUNTING, IFT_FRF16MFRBUNDLE,
            IFT_H323GATEKEEPER, IFT_H323PROXY, IFT_MPLS, IFT_MFSIGLINK, IFT_HDSL2, IFT_SHDSL,
            IFT_DS1FDL, IFT_POS, IFT_DVBASILN, IFT_DVBASIOUT, IFT_PLC, IFT_NFAS, IFT_TR008,
            IFT_GR303RDT, IFT_GR303IDT, IFT_ISUP, IFT_PROPDOCSWIRELESSMACLAYER,
            IFT_PROPDOCSWIRELESSDOWNSTREAM, IFT_PROPDOCSWIRELESSUPSTREAM, IFT_HIPERLAN2,
            IFT_PROPBWAP2MP, IFT_SONETOVERHEADCHANNEL, IFT_DIGITALWRAPPEROVERHEADCHANNEL, IFT_AAL2,
            IFT_RADIOMAC, IFT_ATMRADIO, IFT_IMT, IFT_MVL, IFT_REACHDSL, IFT_FRDLCIENDPT,
            IFT_ATMVCIENDPT, IFT_OPTICALCHANNEL, IFT_OPTICALTRANSPORT, IFT_PROPATM,
            IFT_VOICEOVERCABLE, IFT_INFINIBAND, IFT_TELINK, IFT_Q2931, IFT_VIRTUALTG, IFT_SIPTG,
            IFT_SIPSIG, IFT_DOCSCABLEUPSTREAMCHANNEL, IFT_ECONET, IFT_PON155, IFT_PON622,
            IFT_BRIDGE, IFT_LINEGROUP, IFT_VOICEEMFGD, IFT_VOICEFGDEANA, IFT_VOICEDID, IFT_GIF,
            IFT_DUMMY, IFT_PVC, IFT_FAITH, IFT_ENC, IFT_PFLOG, IFT_PFSYNC, IFT_CARP, IFT_BLUETOOTH,
            IFT_PFLOW, IFT_MBIM, IFT_WIREGUARD,
        );
        crate::reftest::assert_complete(&defs, "IFT_", &ours);
    }
}
/* </TESTS> */
