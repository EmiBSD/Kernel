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
 *	@(#)route.h	8.3 (Berkeley) 4/19/94
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
 * Copyright (c) 1980, 1986, 1991, 1993
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
 *	@(#)route.c	8.2 (Berkeley) 11/15/93
 */

/*
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! The routing ABI: `<net/route.h>`'s flags, priorities, message types and address bits, and
//! the images the routing socket and `sysctl(2)`'s `net.route` share with userland (route(8),
//! netstat(1), routed daemons): `struct rt_msghdr` and its `struct rt_metrics`, `struct
//! rtstat`, `struct rt_tableinfo`, and the label, DNS, static-route and search-path socket
//! addresses. The kernel's routing (`kernel::net::route`, `rtable`, `art`, `rtsock`) and
//! `bsd`'s legacy callers share them from here.
//!
//! Upstream: sys/net/route.h @ 3ce1f3f79392
//! Upstream: sys/net/route.c @ 3ce1f3f79392
//! LZ: sys/net/route.rs@3c62ede68802
//!
//! The constants take the type of the field that stores them: `RTF_*` are `u32` (`rt_flags`),
//! `RTM_*` message types `u8` (`rtm_type`), `RTA_*` `i32` (`rtm_addrs`), `RTAX_*` `usize`
//! (indices into `rti_info`), `RTP_*` `u8` (`rt_priority`), `RTV_*` `u32` (`rtm_inits`),
//! `ROUTE_*` socket options `i32`. The structures are the C's images on LP64 (amd64 and arm64
//! alike), `AbiPod` by [`crate::abi_pod!`] (no padding: each is tiled by its fields) and their
//! sizes asserted at compile time.
//!
//! ## Deviations
//! - `ROUTE_FILTER(m)` is the `const fn` [`route_filter`].
//! - `srtdnstosa(sdns)` is a view of the address's bytes (`hal::pod::view`), not a cast.
//! - `struct rt_kmetrics`, `struct rtentry`, `struct route`, `struct rt_addrinfo`, `struct
//!   rttimer_queue`, `enum rtstat_counters` and `RT_RESOLVE` are the kernel's
//!   (`kernel::net::route`): `#ifdef _KERNEL`, never seen by userland.
//!
//! ## Redesign
//! - LZ's `sys/net/route.rs` held route.h's ABI beside route.c's code; the ABI is here, in
//!   `hal`, so the kernel's routing and `bsd`'s callers name one set of constants and images
//!   (N6.3b; the `net_if` precedent of N6.3a). The `NET_RT_*` names and `RT_TABLEID_*` are
//!   `<sys/socket.h>`'s ([`crate::sys::socket`]).
//! - LZ's `#[repr(C)]` structures with a runtime size check -> `AbiPod` images, so a message
//!   is built and read in a byte buffer through `hal::pod` with no padding byte left
//!   uninitialised; LZ's size asserts (route.rs's) are `const` here.

use core::mem::size_of;

use crate::sys::types::{Pid, SaFamily};

/// `RTF_UP`: route usable.
pub const RTF_UP: u32 = 0x1;
/// `RTF_GATEWAY`: destination is a gateway.
pub const RTF_GATEWAY: u32 = 0x2;
/// `RTF_HOST`: host entry (net otherwise).
pub const RTF_HOST: u32 = 0x4;
/// `RTF_REJECT`: host or net unreachable.
pub const RTF_REJECT: u32 = 0x8;
/// `RTF_DYNAMIC`: created dynamically (by redirect).
pub const RTF_DYNAMIC: u32 = 0x10;
/// `RTF_MODIFIED`: modified dynamically (by redirect).
pub const RTF_MODIFIED: u32 = 0x20;
/// `RTF_DONE`: message confirmed.
pub const RTF_DONE: u32 = 0x40;
/// `RTF_CLONING`: generate new routes on use.
pub const RTF_CLONING: u32 = 0x100;
/// `RTF_MULTICAST`: route associated to a multicast address.
pub const RTF_MULTICAST: u32 = 0x200;
/// `RTF_LLINFO`: generated by ARP or ND.
pub const RTF_LLINFO: u32 = 0x400;
/// `RTF_STATIC`: manually added.
pub const RTF_STATIC: u32 = 0x800;
/// `RTF_BLACKHOLE`: just discard packets (during updates).
pub const RTF_BLACKHOLE: u32 = 0x1000;
/// `RTF_PROTO3`: protocol specific routing flag.
pub const RTF_PROTO3: u32 = 0x2000;
/// `RTF_PROTO2`: protocol specific routing flag.
pub const RTF_PROTO2: u32 = 0x4000;
/// `RTF_ANNOUNCE`: announce L2 entry.
pub const RTF_ANNOUNCE: u32 = RTF_PROTO2;
/// `RTF_PROTO1`: protocol specific routing flag.
pub const RTF_PROTO1: u32 = 0x8000;
/// `RTF_CLONED`: this is a cloned route.
pub const RTF_CLONED: u32 = 0x10000;
/// `RTF_CACHED`: cached by a `RTF_GATEWAY` entry.
pub const RTF_CACHED: u32 = 0x20000;
/// `RTF_MPATH`: multipath route or operation.
pub const RTF_MPATH: u32 = 0x40000;
/// `RTF_MPLS`: MPLS additional infos.
pub const RTF_MPLS: u32 = 0x100000;
/// `RTF_LOCAL`: route to a local address.
pub const RTF_LOCAL: u32 = 0x200000;
/// `RTF_BROADCAST`: route associated to a broadcast address.
pub const RTF_BROADCAST: u32 = 0x400000;
/// `RTF_CONNECTED`: interface route.
pub const RTF_CONNECTED: u32 = 0x800000;
/// `RTF_BFD`: link state controlled by BFD.
pub const RTF_BFD: u32 = 0x1000000;

/// `RTF_FMASK`: the flags `RTM_CHANGE` may modify.
pub const RTF_FMASK: u32 = RTF_LLINFO
    | RTF_PROTO1
    | RTF_PROTO2
    | RTF_PROTO3
    | RTF_BLACKHOLE
    | RTF_REJECT
    | RTF_STATIC
    | RTF_MPLS
    | RTF_BFD;

/// `RTP_NONE`: unset priority, use a sane default.
pub const RTP_NONE: u8 = 0;
/// `RTP_LOCAL`: local address routes (must be the highest).
pub const RTP_LOCAL: u8 = 1;
/// `RTP_CONNECTED`: directly connected routes.
pub const RTP_CONNECTED: u8 = 4;
/// `RTP_STATIC`: static routes base priority.
pub const RTP_STATIC: u8 = 8;
/// `RTP_EIGRP`: EIGRP routes.
pub const RTP_EIGRP: u8 = 28;
/// `RTP_OSPF`: OSPF routes.
pub const RTP_OSPF: u8 = 32;
/// `RTP_ISIS`: IS-IS routes.
pub const RTP_ISIS: u8 = 36;
/// `RTP_RIP`: RIP routes.
pub const RTP_RIP: u8 = 40;
/// `RTP_BGP`: BGP routes.
pub const RTP_BGP: u8 = 48;
/// `RTP_DEFAULT`: routes that have nothing set.
pub const RTP_DEFAULT: u8 = 56;
/// `RTP_PROPOSAL_STATIC`.
pub const RTP_PROPOSAL_STATIC: u8 = 57;
/// `RTP_PROPOSAL_DHCLIENT`.
pub const RTP_PROPOSAL_DHCLIENT: u8 = 58;
/// `RTP_PROPOSAL_SLAAC`.
pub const RTP_PROPOSAL_SLAAC: u8 = 59;
/// `RTP_PROPOSAL_UMB`.
pub const RTP_PROPOSAL_UMB: u8 = 60;
/// `RTP_PROPOSAL_PPP`.
pub const RTP_PROPOSAL_PPP: u8 = 61;
/// `RTP_PROPOSAL_SOLICIT`: request reply of all `RTM_PROPOSAL`.
pub const RTP_PROPOSAL_SOLICIT: u8 = 62;
/// `RTP_MAX`: maximum priority.
pub const RTP_MAX: u8 = 63;
/// `RTP_ANY`: any of the above.
pub const RTP_ANY: u8 = 64;
/// `RTP_MASK`.
pub const RTP_MASK: u8 = 0x7f;
/// `RTP_DOWN`: route/link is down.
pub const RTP_DOWN: u8 = 0x80;

/// `RTM_VERSION`: up the ante and ignore older versions.
pub const RTM_VERSION: u8 = 5;

/// `RTM_MAXSIZE`: maximum size of an accepted route message.
pub const RTM_MAXSIZE: usize = 2048;

/// `RTM_ADD`: add route.
pub const RTM_ADD: u8 = 0x1;
/// `RTM_DELETE`: delete route.
pub const RTM_DELETE: u8 = 0x2;
/// `RTM_CHANGE`: change metrics or flags.
pub const RTM_CHANGE: u8 = 0x3;
/// `RTM_GET`: report metrics.
pub const RTM_GET: u8 = 0x4;
/// `RTM_LOSING`: kernel suspects partitioning.
pub const RTM_LOSING: u8 = 0x5;
/// `RTM_REDIRECT`: told to use different route.
pub const RTM_REDIRECT: u8 = 0x6;
/// `RTM_MISS`: lookup failed on this address.
pub const RTM_MISS: u8 = 0x7;
/// `RTM_RESOLVE`: request to resolve dst to LL addr.
pub const RTM_RESOLVE: u8 = 0xb;
/// `RTM_NEWADDR`: address being added to iface.
pub const RTM_NEWADDR: u8 = 0xc;
/// `RTM_DELADDR`: address being removed from iface.
pub const RTM_DELADDR: u8 = 0xd;
/// `RTM_IFINFO`: iface going up/down etc.
pub const RTM_IFINFO: u8 = 0xe;
/// `RTM_IFANNOUNCE`: iface arrival/departure.
pub const RTM_IFANNOUNCE: u8 = 0xf;
/// `RTM_DESYNC`: route socket buffer overflow.
pub const RTM_DESYNC: u8 = 0x10;
/// `RTM_INVALIDATE`: invalidate cache of L2 route.
pub const RTM_INVALIDATE: u8 = 0x11;
/// `RTM_BFD`: bidirectional forwarding detection.
pub const RTM_BFD: u8 = 0x12;
/// `RTM_PROPOSAL`: proposal for resolvd(8).
pub const RTM_PROPOSAL: u8 = 0x13;
/// `RTM_CHGADDRATTR`: address attribute change.
pub const RTM_CHGADDRATTR: u8 = 0x14;
/// `RTM_80211INFO`: 80211 iface change.
pub const RTM_80211INFO: u8 = 0x15;
/// `RTM_SOURCE`: set source address.
pub const RTM_SOURCE: u8 = 0x16;

/// `RTV_MTU`: init or lock `_mtu`.
pub const RTV_MTU: u32 = 0x1;
/// `RTV_HOPCOUNT`: init or lock `_hopcount`.
pub const RTV_HOPCOUNT: u32 = 0x2;
/// `RTV_EXPIRE`: init or lock `_expire`.
pub const RTV_EXPIRE: u32 = 0x4;
/// `RTV_RPIPE`: init or lock `_recvpipe`.
pub const RTV_RPIPE: u32 = 0x8;
/// `RTV_SPIPE`: init or lock `_sendpipe`.
pub const RTV_SPIPE: u32 = 0x10;
/// `RTV_SSTHRESH`: init or lock `_ssthresh`.
pub const RTV_SSTHRESH: u32 = 0x20;
/// `RTV_RTT`: init or lock `_rtt`.
pub const RTV_RTT: u32 = 0x40;
/// `RTV_RTTVAR`: init or lock `_rttvar`.
pub const RTV_RTTVAR: u32 = 0x80;

/// `RTA_DST`: destination sockaddr present.
pub const RTA_DST: i32 = 0x1;
/// `RTA_GATEWAY`: gateway sockaddr present.
pub const RTA_GATEWAY: i32 = 0x2;
/// `RTA_NETMASK`: netmask sockaddr present.
pub const RTA_NETMASK: i32 = 0x4;
/// `RTA_GENMASK`: cloning mask sockaddr present.
pub const RTA_GENMASK: i32 = 0x8;
/// `RTA_IFP`: interface name sockaddr present.
pub const RTA_IFP: i32 = 0x10;
/// `RTA_IFA`: interface addr sockaddr present.
pub const RTA_IFA: i32 = 0x20;
/// `RTA_AUTHOR`: sockaddr for author of redirect.
pub const RTA_AUTHOR: i32 = 0x40;
/// `RTA_BRD`: for `NEWADDR`, broadcast or p-p dest addr.
pub const RTA_BRD: i32 = 0x80;
/// `RTA_SRC`: source sockaddr present.
pub const RTA_SRC: i32 = 0x100;
/// `RTA_SRCMASK`: source netmask present.
pub const RTA_SRCMASK: i32 = 0x200;
/// `RTA_LABEL`: route label present.
pub const RTA_LABEL: i32 = 0x400;
/// `RTA_BFD`: bfd present.
pub const RTA_BFD: i32 = 0x800;
/// `RTA_DNS`: DNS servers sockaddr present.
pub const RTA_DNS: i32 = 0x1000;
/// `RTA_STATIC`: RFC 3442 encoded static routes present.
pub const RTA_STATIC: i32 = 0x2000;
/// `RTA_SEARCH`: RFC 3397 encoded search path present.
pub const RTA_SEARCH: i32 = 0x4000;

/// `RTAX_DST`: destination sockaddr present.
pub const RTAX_DST: usize = 0;
/// `RTAX_GATEWAY`: gateway sockaddr present.
pub const RTAX_GATEWAY: usize = 1;
/// `RTAX_NETMASK`: netmask sockaddr present.
pub const RTAX_NETMASK: usize = 2;
/// `RTAX_GENMASK`: cloning mask sockaddr present.
pub const RTAX_GENMASK: usize = 3;
/// `RTAX_IFP`: interface name sockaddr present.
pub const RTAX_IFP: usize = 4;
/// `RTAX_IFA`: interface addr sockaddr present.
pub const RTAX_IFA: usize = 5;
/// `RTAX_AUTHOR`: sockaddr for author of redirect.
pub const RTAX_AUTHOR: usize = 6;
/// `RTAX_BRD`: for `NEWADDR`, broadcast or p-p dest addr.
pub const RTAX_BRD: usize = 7;
/// `RTAX_SRC`: source sockaddr present.
pub const RTAX_SRC: usize = 8;
/// `RTAX_SRCMASK`: source netmask present.
pub const RTAX_SRCMASK: usize = 9;
/// `RTAX_LABEL`: route label present.
pub const RTAX_LABEL: usize = 10;
/// `RTAX_BFD`: bfd present.
pub const RTAX_BFD: usize = 11;
/// `RTAX_DNS`: DNS server(s) sockaddr present.
pub const RTAX_DNS: usize = 12;
/// `RTAX_STATIC`: RFC 3442 encoded static routes present.
pub const RTAX_STATIC: usize = 13;
/// `RTAX_SEARCH`: RFC 3397 encoded search path present.
pub const RTAX_SEARCH: usize = 14;
/// `RTAX_MAX`: size of array to allocate.
pub const RTAX_MAX: usize = 15;

/// `ROUTE_MSGFILTER`: bitmask to specify which types should be sent to the client.
pub const ROUTE_MSGFILTER: i32 = 1;
/// `ROUTE_TABLEFILTER`: change routing table the socket is listening on, `RTABLE_ANY`
/// listens on all tables.
pub const ROUTE_TABLEFILTER: i32 = 2;
/// `ROUTE_PRIOFILTER`: only pass updates with a priority higher or equal (actual value
/// lower) to the specified priority.
pub const ROUTE_PRIOFILTER: i32 = 3;
/// `ROUTE_FLAGFILTER`: do not pass updates for routes with flags in this bitmask.
pub const ROUTE_FLAGFILTER: i32 = 4;

/// `RTABLE_ANY`: every routing table (`ROUTE_TABLEFILTER`).
pub const RTABLE_ANY: u32 = 0xffff_ffff;

/// `RTLABEL_LEN`: a route label's buffer, its NUL included.
pub const RTLABEL_LEN: usize = 32;
/// `RTDNS_LEN`.
pub const RTDNS_LEN: usize = 128;
/// `RTSTATIC_LEN`.
pub const RTSTATIC_LEN: usize = 128;
/// `RTSEARCH_LEN`.
pub const RTSEARCH_LEN: usize = 128;

/// `struct rt_metrics`: the huge version of a route's metrics, for userland compatibility.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RtMetrics {
    /// Packets sent using this route.
    pub rmx_pksent: u64,
    /// Lifetime for route, e.g. redirect.
    pub rmx_expire: i64,
    /// Kernel must leave these values.
    pub rmx_locks: u32,
    /// MTU for this path.
    pub rmx_mtu: u32,
    /// References held (the routing table's not counted).
    pub rmx_refcnt: u32,
    /// Max hops expected (no longer used; some apps may still need it).
    pub rmx_hopcount: u32,
    /// Inbound delay-bandwidth product.
    pub rmx_recvpipe: u32,
    /// Outbound delay-bandwidth product.
    pub rmx_sendpipe: u32,
    /// Outbound gateway buffer limit.
    pub rmx_ssthresh: u32,
    /// Estimated round trip time.
    pub rmx_rtt: u32,
    /// Estimated rtt variance.
    pub rmx_rttvar: u32,
    /// Padding.
    pub rmx_pad: u32,
}

crate::abi_pod!(RtMetrics {
    rmx_pksent: u64,
    rmx_expire: i64,
    rmx_locks: u32,
    rmx_mtu: u32,
    rmx_refcnt: u32,
    rmx_hopcount: u32,
    rmx_recvpipe: u32,
    rmx_sendpipe: u32,
    rmx_ssthresh: u32,
    rmx_rtt: u32,
    rmx_rttvar: u32,
    rmx_pad: u32,
});

/// `struct rtstat`: routing statistics (`NET_RT_STATS`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rtstat {
    /// Bogus redirect calls.
    pub rts_badredirect: u32,
    /// Routes created by redirects.
    pub rts_dynamic: u32,
    /// Routes modified by redirects.
    pub rts_newgateway: u32,
    /// Lookups which failed.
    pub rts_unreach: u32,
    /// Lookups satisfied by a wildcard.
    pub rts_wildcard: u32,
}

crate::abi_pod!(Rtstat {
    rts_badredirect: u32,
    rts_dynamic: u32,
    rts_newgateway: u32,
    rts_unreach: u32,
    rts_wildcard: u32,
});

/// `struct rt_tableinfo`: routing table info (`NET_RT_TABLE`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RtTableinfo {
    /// Routing table id.
    pub rti_tableid: u16,
    /// Routing domain id.
    pub rti_domainid: u16,
}

crate::abi_pod!(RtTableinfo {
    rti_tableid: u16,
    rti_domainid: u16,
});

/// `struct rt_msghdr`: the header of a routing message; its socket addresses follow it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RtMsghdr {
    /// To skip over non-understood messages.
    pub rtm_msglen: u16,
    /// Future binary compatibility.
    pub rtm_version: u8,
    /// Message type.
    pub rtm_type: u8,
    /// `sizeof(rt_msghdr)` to skip over the header.
    pub rtm_hdrlen: u16,
    /// Index for associated ifp.
    pub rtm_index: u16,
    /// Routing table id.
    pub rtm_tableid: u16,
    /// Routing priority.
    pub rtm_priority: u8,
    /// MPLS additional infos.
    pub rtm_mpls: u8,
    /// Bitmask identifying sockaddrs in msg.
    pub rtm_addrs: i32,
    /// Flags, incl. kern & message, e.g. `DONE`.
    pub rtm_flags: i32,
    /// Bitmask used in `RTM_CHANGE` message.
    pub rtm_fmask: i32,
    /// Identify sender.
    pub rtm_pid: Pid,
    /// For sender to identify action.
    pub rtm_seq: i32,
    /// Why failed.
    pub rtm_errno: i32,
    /// Which metrics we are initializing.
    pub rtm_inits: u32,
    /// Metrics themselves.
    pub rtm_rmx: RtMetrics,
}

impl RtMsghdr {
    /// `rtm_use`: the no longer used `rtm_rmx.rmx_pksent`, overloaded.
    pub fn rtm_use(&self) -> u64 {
        self.rtm_rmx.rmx_pksent
    }
}

crate::abi_pod!(RtMsghdr {
    rtm_msglen: u16,
    rtm_version: u8,
    rtm_type: u8,
    rtm_hdrlen: u16,
    rtm_index: u16,
    rtm_tableid: u16,
    rtm_priority: u8,
    rtm_mpls: u8,
    rtm_addrs: i32,
    rtm_flags: i32,
    rtm_fmask: i32,
    rtm_pid: Pid,
    rtm_seq: i32,
    rtm_errno: i32,
    rtm_inits: u32,
    rtm_rmx: RtMetrics,
});

/// `struct sockaddr_rtlabel`: a route label as a socket address.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SockaddrRtlabel {
    /// Total length.
    pub sr_len: u8,
    /// Address family.
    pub sr_family: SaFamily,
    /// The label, NUL-terminated.
    pub sr_label: [u8; RTLABEL_LEN],
}

crate::abi_pod!(SockaddrRtlabel {
    sr_len: u8,
    sr_family: SaFamily,
    sr_label: [u8; RTLABEL_LEN],
});

/// `struct sockaddr_rtdns`: DNS servers as a socket address.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SockaddrRtdns {
    /// Total length.
    pub sr_len: u8,
    /// Address family.
    pub sr_family: SaFamily,
    /// The servers.
    pub sr_dns: [u8; RTDNS_LEN],
}

impl SockaddrRtdns {
    /// Every byte zero.
    pub const fn new() -> Self {
        Self {
            sr_len: 0,
            sr_family: 0,
            sr_dns: [0; RTDNS_LEN],
        }
    }
}

impl Default for SockaddrRtdns {
    fn default() -> Self {
        Self::new()
    }
}

crate::abi_pod!(SockaddrRtdns {
    sr_len: u8,
    sr_family: SaFamily,
    sr_dns: [u8; RTDNS_LEN],
});

/// `struct sockaddr_rtstatic`: RFC 3442 static routes as a socket address.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SockaddrRtstatic {
    /// Total length.
    pub sr_len: u8,
    /// Address family.
    pub sr_family: SaFamily,
    /// The routes.
    pub sr_static: [u8; RTSTATIC_LEN],
}

impl SockaddrRtstatic {
    /// Every byte zero.
    pub const fn new() -> Self {
        Self {
            sr_len: 0,
            sr_family: 0,
            sr_static: [0; RTSTATIC_LEN],
        }
    }
}

impl Default for SockaddrRtstatic {
    fn default() -> Self {
        Self::new()
    }
}

crate::abi_pod!(SockaddrRtstatic {
    sr_len: u8,
    sr_family: SaFamily,
    sr_static: [u8; RTSTATIC_LEN],
});

/// `struct sockaddr_rtsearch`: an RFC 3397 search path as a socket address.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SockaddrRtsearch {
    /// Total length.
    pub sr_len: u8,
    /// Address family.
    pub sr_family: SaFamily,
    /// The search path.
    pub sr_search: [u8; RTSEARCH_LEN],
}

impl SockaddrRtsearch {
    /// Every byte zero.
    pub const fn new() -> Self {
        Self {
            sr_len: 0,
            sr_family: 0,
            sr_search: [0; RTSEARCH_LEN],
        }
    }
}

impl Default for SockaddrRtsearch {
    fn default() -> Self {
        Self::new()
    }
}

crate::abi_pod!(SockaddrRtsearch {
    sr_len: u8,
    sr_family: SaFamily,
    sr_search: [u8; RTSEARCH_LEN],
});

/// `ROUTE_FILTER(m)`: the bit of message type `m` in a `ROUTE_MSGFILTER` mask.
pub const fn route_filter(m: u8) -> u32 {
    1 << m
}

// The C's sizes on LP64 (amd64 and arm64 alike).
const _: () = {
    assert!(size_of::<RtMetrics>() == 56);
    assert!(size_of::<Rtstat>() == 20);
    assert!(size_of::<RtTableinfo>() == 4);
    assert!(size_of::<RtMsghdr>() == 96);
    assert!(size_of::<SockaddrRtlabel>() == 34);
    assert!(size_of::<SockaddrRtdns>() == 130);
    assert!(size_of::<SockaddrRtstatic>() == 130);
    assert!(size_of::<SockaddrRtsearch>() == 130);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pod;

    #[test]
    fn a_message_header_is_read_and_written_in_its_bytes() {
        // A route(8) RTM_GET as userland writes it: the C's offsets (rtm_addrs at 12,
        // rtm_pid at 24, rtm_rmx at 40, rmx_refcnt at 40 + 24).
        let mut buf = [0u8; 96];
        let mut h = RtMsghdr {
            rtm_msglen: 96,
            rtm_version: RTM_VERSION,
            rtm_type: RTM_GET,
            rtm_hdrlen: 96,
            rtm_addrs: RTA_DST | RTA_NETMASK,
            rtm_pid: 42,
            ..RtMsghdr::default()
        };
        h.rtm_rmx.rmx_refcnt = 3;
        assert!(pod::put(&mut buf, 0, &h));
        assert_eq!(&buf[0..4], &[96, 0, RTM_VERSION, RTM_GET]);
        assert_eq!(buf[12], 0x5);
        assert_eq!(buf[24], 42);
        assert_eq!(buf[64], 3);
        let back: RtMsghdr = pod::at(&buf, 0).expect("the header");
        assert_eq!(back, h);
        assert_eq!(back.rtm_use(), 0);
    }

    #[test]
    fn the_filter_bits_are_the_message_types() {
        assert_eq!(route_filter(RTM_ADD), 0x2);
        assert_eq!(route_filter(RTM_IFINFO), 1 << 14);
        assert_eq!(route_filter(RTM_SOURCE), 1 << 22);
    }
}
/* </TESTS> */
