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
 *	@(#)ip_icmp.c	8.2 (Berkeley) 1/4/94
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
 *
 */

/*-
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
 *	@(#)protosw.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! ICMPv6: error generation ([`icmp6_error`]), input processing ([`icmp6_input`]: echo, the
//! MLD and ND dispatch, errors handed to the upper layers), reflection, redirects, path MTU
//! discovery and the `net.inet6.icmp6` sysctls: `netinet6/icmp6.c`; with `struct
//! ip6ctlparam`, the argument of an IPv6 protocol's control input ([`Ip6CtlParam`],
//! `<netinet6/ip6protosw.h>`).
//!
//! Upstream: sys/netinet6/icmp6.c @ 3ce1f3f79392
//! Upstream: sys/netinet6/ip6protosw.h @ 3ce1f3f79392
//! LZ: sys/netinet6/icmp6.rs@3c62ede68802
//! LZ: sys/netinet6/ip6protosw.rs@3c62ede68802
//!
//! Locks: \[a\] the tunables are atomics; the rate limiter's two words are under
//! `ppsratecheck_mtx`; the registered path MTU callbacks are read lock-free.
//!
//! ## Deviations
//! - `NPF` is configured: `PF_TAG_DIVERTED` is honoured on input and `pf_pkt_addr_changed`
//!   is the `Pf` bridge's leg. `NCARP` (`carp_lsdrop` on echo requests) is not configured.
//! - `icmp6_ctloutput` (`ICMP6_FILTER`) stays `bsd`'s with the raw sockets' filter word it
//!   reads and writes (`inp_icmp6filt`, N6.6b; `bsd`'s `netinet6/icmp6.rs`).
//! - The raw sockets' input (`rip6_input`, the C's tail call) is the IPv6 domain's raw
//!   entry's `Protocol::input` (`bsd`'s until N6.6b); the upper layer's control input
//!   (`inet6sw[ip6_protox[nxt]].pr_ctlinput`) is the entry's `Protocol::ctlinput`.
//! - `icmp6_mtudisc_timeout` is the C's: it does not tell TCP that the path MTU may have
//!   grown (the IPv4 twin does).
//! - The headers the C writes in place (the echo reply's type, the reflected header, the
//!   quoted header's cleared scope zones, `udp6_ctlinput`'s embedded zone in the quoted
//!   destination) are written into the mbuf that holds them; a read-only one (a shared
//!   cluster, which the C writes anyway) is copied first, and a packet whose bytes find no
//!   buffer is dropped, counted as the C counts its buffer failures there
//!   (`icp6s_tooshort`).
//!
//! ## Redesign
//! - LZ's reads of the message through the raw pointers `ip6_exthdr_get` returned (`read_at`,
//!   an `unsafe fn`; 23 `unsafe` over mbuf data) -> `pod` copies of the headers from the
//!   mbuf that holds them (`ip6_input::ip6_exthdr_get`, `ip6_exthdr_bytes`), the C's checks
//!   in their order before each.
//! - LZ's `struct ip6ctlparam` (raw pointers into the message and into `icmp6_notify_error`'s
//!   locals, read by `bsd`'s IPv6 `pr_ctlinput`s and `icmp6_mtudisc_update` through
//!   `unsafe`) -> [`Ip6CtlParam`]: the message by reference (`m`), the ICMPv6 and quoted IPv6
//!   headers and the source by value, the final destination a `Cell` (`udp6_ctlinput` embeds
//!   its zone, as the C writes `*ip6c_finaldst`; written back into the packet when it is the
//!   quoted header's own destination, as the C's pointer is), the command's argument an
//!   `Option<u32>` (the C's `void *` to `notifymtu`). It reaches a protocol through
//!   `Protocol::ctlinput`'s `CtlArg::Ip6` (D-8; `kernel::net::proto`); `bsd`'s IPv6
//!   `pr_ctlinput`s read the kernel's type (their one cast of `d`).
//! - `icmp6_mtudisc_callbacks` (a `LIST` of `malloc(M_PCB)` records; LZ: a `Vec` of function
//!   pointers in a `StaticCell` written without a lock) -> [`ICMP6_MTUDISC_CALLBACKS`], a
//!   `ReadMostly` of an `MArray` of `fn`s (D-15), grown by a copy at registration (boot time,
//!   one registrant at the pin: TCP), read lock-free per Packet Too Big; registering twice is
//!   a no-op as the C's.
//! - `icmp6errpps_count`/`icmp6errppslim_last` (LZ: a `StaticCell` read through
//!   `ppsratecheck_shared`'s raw pointers) -> [`ICMP6ERRPPS`], a `Guarded` cell keyed to
//!   `ppsratecheck_mtx`, read and written inside `ppsratecheck_locked`'s one section (D-14,
//!   N6.4c2's R7).
//! - The packets are owned (`MbufBox`): the C's `struct mbuf **` is `&mut Option<MbufBox>`,
//!   `m_freem` a drop.
//!
//! **Lock and spl transitions** against the C: `icmp6_error` takes `ppsratecheck_mtx` once
//! (one spl pair, the C's); the input path takes none of its own; the sysctls' changes of the
//! timer queues take `sysctl_lock` for writing, as the C.

use core::cell::Cell;
use core::mem::{offset_of, size_of};
use core::sync::atomic::Ordering;

use hal::malloc::{M_NOWAIT, MArray};
use hal::netlock::NetHeld;
use hal::pod;
use hal::pool::PoolArc;
use hal::rwlock::Guarded;
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::limits::INT_MAX;
use hal::sys::net_if::{IFF_UP, IFXF_AUTOCONF6, SDL_DATA_OFFSET};
use hal::sys::net_route::{
    RTAX_DST, RTAX_GATEWAY, RTAX_LABEL, RTF_BLACKHOLE, RTF_DYNAMIC, RTF_GATEWAY, RTF_HOST,
    RTF_LLINFO, RTF_LOCAL, RTF_REJECT, RTM_ADD, RTV_MTU, SockaddrRtlabel,
};
use hal::sys::netinet_icmp6::{
    ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_ADDR, ICMP6_DST_UNREACH_ADMIN,
    ICMP6_DST_UNREACH_BEYONDSCOPE, ICMP6_DST_UNREACH_NOPORT, ICMP6_DST_UNREACH_NOROUTE,
    ICMP6_ECHO_REPLY, ICMP6_ECHO_REQUEST, ICMP6_PACKET_TOO_BIG, ICMP6_PARAM_PROB,
    ICMP6_PARAMPROB_HEADER, ICMP6_PARAMPROB_NEXTHEADER, ICMP6_PARAMPROB_OPTION,
    ICMP6_ROUTER_RENUMBERING, ICMP6_ROUTER_RENUMBERING_COMMAND, ICMP6_ROUTER_RENUMBERING_RESULT,
    ICMP6_TIME_EXCEED_REASSEMBLY, ICMP6_TIME_EXCEED_TRANSIT, ICMP6_TIME_EXCEEDED, ICMP6_WRUREPLY,
    ICMP6_WRUREQUEST, ICMPV6_PLD_MAXLEN, ICMPV6CTL_ERRPPSLIMIT, ICMPV6CTL_MTUDISC_HIWAT,
    ICMPV6CTL_MTUDISC_LOWAT, ICMPV6CTL_ND6_DELAY, ICMPV6CTL_ND6_MMAXTRIES, ICMPV6CTL_ND6_QUEUED,
    ICMPV6CTL_ND6_UMAXTRIES, ICMPV6CTL_REDIRTIMEOUT, ICMPV6CTL_STATS, ICP6S_NCOUNTERS, Icmp6Hdr,
    Icmp6RouterRenum, Icmp6statCounters, MLD_LISTENER_DONE, MLD_LISTENER_QUERY,
    MLD_LISTENER_REPORT, MLD_MTRACE, MLD_MTRACE_RESP, MldHdr, ND_NEIGHBOR_ADVERT,
    ND_NEIGHBOR_SOLICIT, ND_OPT_REDIRECTED_HEADER, ND_OPT_TARGET_LINKADDR, ND_REDIRECT,
    ND_REDIRECT_ONLINK, ND_REDIRECT_ROUTER, ND_ROUTER_ADVERT, ND_ROUTER_SOLICIT, NdNeighborAdvert,
    NdNeighborSolicit, NdOptHdr, NdOptRdHdr, NdRedirect, NdRouterAdvert, NdRouterSolicit,
};
use hal::sys::netinet_in::{
    IPPROTO_AH, IPPROTO_DONE, IPPROTO_DSTOPTS, IPPROTO_FRAGMENT, IPPROTO_HOPOPTS, IPPROTO_ICMPV6,
    IPPROTO_IPV6, IPPROTO_RAW, IPPROTO_ROUTING,
};
use hal::sys::netinet_ip6::{
    IP6F_OFF_MASK, IPV6_FLOWLABEL_MASK, IPV6_MMTU, IPV6_VERSION, IPV6_VERSION_MASK, Ip6Ext,
    Ip6Frag, Ip6Hdr, Ip6Rthdr, Ip6Rthdr0,
};
use hal::sys::netinet6_in6::{
    IPV6_RTHDR_TYPE_0, In6Addr, SockaddrIn6, in6_is_addr_linklocal, in6_is_addr_multicast,
    in6_is_addr_unspecified, in6_is_scope_embed,
};
use hal::sys::netinet6_in6_var::{IN6_IFF_ANYCAST, IN6_IFF_DUPLICATED, IN6_IFF_TENTATIVE};
use hal::sys::socket::{AF_INET6, AF_LINK};
use hal::sys::sysctl::SysctlBoundedArgs;

use crate::assert::kpanic;
use crate::clock::Timeval;
use crate::net::domain::pfctlinput;
use crate::net::if_::{if_get, if_put, ifa_ifwithaddr};
use crate::net::if_var::Netstack;
use crate::net::mbuf::{
    M_BCAST, M_COPYALL, M_DONTWAIT, M_EXT, M_ICMP_CSUM_OUT, M_MAXLOOP, M_MCAST, MHLEN, MT_HEADER,
    Mbuf, MbufBox, PF_TAG_DIVERTED, PF_TAG_GENERATED, m_adj, m_cat, m_copym, m_gethdr,
    m_move_pkthdr, m_prepend, m_pullup, m_resethdr, m_trailingspace, mclget,
};
use crate::net::pf_hooks::pf;
use crate::net::proto::{
    CtlArg, PRC_HOSTDEAD, PRC_MSGSIZE, PRC_NCMDS, PRC_PARAMPROB, PRC_REDIRECT_HOST,
    PRC_TIMXCEED_INTRANS, PRC_TIMXCEED_REASS, PRC_UNREACH_NET, PRC_UNREACH_PORT,
    PRC_UNREACH_PROTOCOL, SockaddrView,
};
use crate::net::route::{
    RT_RESOLVE, RtAddrinfo, Rtentry, RttimerQueue, rt_timer_add, rt_timer_queue_change,
    rt_timer_queue_count, rt_timer_queue_init, rtalloc, rtdeletemsg, rtisvalid, rtlabel_id2sa,
    rtredirect, rtrequest,
};
use crate::net::rtable::rtable_getsource;
use crate::net::rtsock::rtm_send;
use crate::netinet::in_proto::inet6_protocol;
use crate::netinet6::in6::{in6_addr2scopeid, in6_ifawithscope, in6ifa_ifpforlinklocal};
use crate::netinet6::in6_cksum::in6_cksum;
use crate::netinet6::in6_src::in6_embedscope;
use crate::netinet6::in6_var::{In6Ifaddr, ia6_in6};
use crate::netinet6::ip6_input::{
    ip6_exthdr_bytes, ip6_exthdr_get, ip6_lasthdr, ip6_send, ip6_set_bytes,
};
use crate::netinet6::ip6_output::{ip6_load, ip6_output, ip6_store};
use crate::netinet6::ip6_var::{
    ICMP6_REDIRTIMEOUT, ICMP6COUNTERS, ICMP6ERRPPSLIM, IP6_DEFHLIM, IP6_FORWARDING,
    IP6_MAXDYNROUTES, IP6_MTUDISC_TIMEOUT, icmp6stat_inc, icmp6stat_inc_hist,
};
use crate::netinet6::mld6::{mld6_fasttimo, mld6_init, mld6_input};
use crate::netinet6::nd6::{
    ND6_DELAY, ND6_MMAXTRIES, ND6_UMAXTRIES, NdOpts, ln_hold_total, nd6_cache_lladdr,
    nd6_is_addr_neighbor, nd6_lookup, nd6_opt_lladdr, nd6_options,
};
use crate::netinet6::nd6_nbr::{nd6_na_input, nd6_ns_input};
use crate::netinet6::nd6_rtr::nd6_rtr_cache;
use crate::proc::ProcGlue;
use crate::sysctl::{SYSCTL_LOCK, SysctlReq, bounded_arr, int_bounded, rdint, rdstruct};
use crate::time::{PPSRATECHECK_MTX, ppsratecheck_held, ppsratecheck_locked};

/// `sizeof(struct ip6_hdr)`.
const IP6_HDR_LEN: usize = size_of::<Ip6Hdr>();
/// `sizeof(struct icmp6_hdr)`.
const ICMP6_HDR_LEN: usize = size_of::<Icmp6Hdr>();
/// `M_PCB`: the callbacks' malloc type.
const M_PCB: i32 = 4;

/// `void (*mc_func)(struct sockaddr_in6 *, u_int)`: a path MTU change callback (the
/// destination and the routing table).
pub type Mtudisc6Fn = fn(&SockaddrIn6, u32);

/// `struct ip6ctlparam`: the argument of an IPv6 protocol's control input (`pr_ctlinput`),
/// consulted only with the `AF_INET6` family, valid for the call.
///
/// ```text
/// IPv6 ICMP IPv6 [exthdrs] finalhdr payload
/// ^    ^    ^              ^
/// |    |    ip6c_ip6       ip6c_off
/// |    ip6c_icmp6
/// ip6c_m
/// ```
pub struct Ip6CtlParam<'a> {
    /// `ip6c_m`: start of mbuf chain.
    pub m: &'a Mbuf,
    /// `*ip6c_icmp6`: icmp6 header of target packet.
    pub icmp6: Icmp6Hdr,
    /// `*ip6c_ip6`: ip6 header of target packet.
    pub ip6: Ip6Hdr,
    /// `ip6c_off`: offset of the target proto header.
    pub off: i32,
    /// `*ip6c_src`: srcaddr w/ additional info (`ip6c_ip6->ip6_src` + scope info + flowlabel
    /// in `ip6c_ip6`: beware of flowlabel, if you try to compare it against others).
    pub src: SockaddrIn6,
    /// `*ip6c_finaldst`: final destination address (usually `ip6c_ip6->ip6_dst`; the last
    /// address of a type 0 routing header with segments left); the protocols embed its zone.
    pub finaldst: Cell<In6Addr>,
    /// `ip6c_cmdarg`: control command dependent data (`notifymtu` for a Packet Too Big).
    pub cmdarg: Option<u32>,
    /// `ip6c_nxt`: final next header field.
    pub nxt: u8,
}

/// `icmp6errppslim_last` and `icmp6errpps_count`.
struct Icmp6ErrPps {
    /// `icmp6errppslim_last`.
    last: Timeval,
    /// `icmp6errpps_count`.
    count: i32,
}

/// What `icmp6_input` does after its message-specific processing.
enum Next {
    /// `deliver`: the message is an error for the protocol it quotes; `PRC_*` code.
    Deliver(i32),
    /// `badcode`.
    Badcode,
    /// `badlen`.
    Badlen,
    /// `break` to `raw`: give the packet to the raw sockets.
    Raw,
}

/// `icmp6errpps_count`, `icmp6errppslim_last`, under `ppsratecheck_mtx`.
static ICMP6ERRPPS: Guarded<Icmp6ErrPps> = Guarded::new_static_mutexed(
    &PPSRATECHECK_MTX,
    Icmp6ErrPps {
        last: Timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        count: 0,
    },
);

/// `icmp6_mtudisc_callbacks`: the callbacks to notify when Path MTU changes are made, the
/// newest first (`LIST_INSERT_HEAD`).
pub static ICMP6_MTUDISC_CALLBACKS: ReadMostly<Option<MArray<Mtudisc6Fn>>> = ReadMostly::new(None);

/// `icmp6_mtudisc_timeout_q`: the routes cloned for path MTU discovery.
pub static ICMP6_MTUDISC_TIMEOUT_Q: RttimerQueue = RttimerQueue::new();
/// \[a\] `icmp6_mtudisc_hiwat`: XXX do these values make any sense?
pub static ICMP6_MTUDISC_HIWAT: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(1280);
/// \[a\] `icmp6_mtudisc_lowat`.
pub static ICMP6_MTUDISC_LOWAT: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(256);
/// `icmp6_redirect_timeout_q`: keeps track of the number of redirect routes.
pub static ICMP6_REDIRECT_TIMEOUT_Q: RttimerQueue = RttimerQueue::new();

/// `icmpv6ctl_vars[]`.
static ICMPV6CTL_VARS: [SysctlBoundedArgs; 6] = [
    SysctlBoundedArgs::new(ICMPV6CTL_ND6_DELAY, &ND6_DELAY, 0, INT_MAX),
    SysctlBoundedArgs::new(ICMPV6CTL_ND6_UMAXTRIES, &ND6_UMAXTRIES, 0, INT_MAX),
    SysctlBoundedArgs::new(ICMPV6CTL_ND6_MMAXTRIES, &ND6_MMAXTRIES, 0, INT_MAX),
    SysctlBoundedArgs::new(ICMPV6CTL_MTUDISC_HIWAT, &ICMP6_MTUDISC_HIWAT, 0, INT_MAX),
    SysctlBoundedArgs::new(ICMPV6CTL_MTUDISC_LOWAT, &ICMP6_MTUDISC_LOWAT, 0, INT_MAX),
    SysctlBoundedArgs::new(ICMPV6CTL_ERRPPSLIMIT, &ICMP6ERRPPSLIM, -1, 1000),
];

/// The bytes of a `sockaddr_in6`, for a view.
#[inline]
fn sin6_bytes(sin6: &SockaddrIn6) -> [u8; size_of::<SockaddrIn6>()] {
    let mut b = [0u8; size_of::<SockaddrIn6>()];
    let _ = pod::put(&mut b, 0, sin6);
    b
}

/// The view of a `sockaddr_in6`'s bytes.
#[inline]
fn view(b: &[u8]) -> SockaddrView<'_> {
    match SockaddrView::new(b) {
        Some(v) => v,
        None => kpanic(format_args!("icmp6: a sockaddr_in6 too short for a view")),
    }
}

/// A `sockaddr_in6` of `addr` (`bzero`, `sin6_len`, `sin6_family`, `sin6_addr`).
#[inline]
fn sin6_of(addr: In6Addr) -> SockaddrIn6 {
    SockaddrIn6::with_addr(addr)
}

/// `icmp6_init`: MLD, the path MTU and the redirect timeout queues (the counters are a
/// static).
pub fn icmp6_init() {
    mld6_init();
    rt_timer_queue_init(
        &ICMP6_MTUDISC_TIMEOUT_Q,
        IP6_MTUDISC_TIMEOUT.load(Ordering::Relaxed),
        Some(icmp6_mtudisc_timeout),
    );
    rt_timer_queue_init(
        &ICMP6_REDIRECT_TIMEOUT_Q,
        ICMP6_REDIRTIMEOUT.load(Ordering::Relaxed),
        None,
    );
    // icmp6counters = counters_alloc(icp6s_ncounters): ICMP6COUNTERS.
}

/// `icmp6_errcount(type, code)`: counts an error message of `type_` and `code` that we send.
pub fn icmp6_errcount(type_: u8, code: u8) {
    use Icmp6statCounters as C;
    let c = match (type_, code) {
        (ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOROUTE) => C::Icp6sOdstUnreachNoroute,
        (ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_ADMIN) => C::Icp6sOdstUnreachAdmin,
        (ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_BEYONDSCOPE) => C::Icp6sOdstUnreachBeyondscope,
        (ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_ADDR) => C::Icp6sOdstUnreachAddr,
        (ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT) => C::Icp6sOdstUnreachNoport,
        (ICMP6_PACKET_TOO_BIG, _) => C::Icp6sOpacketTooBig,
        (ICMP6_TIME_EXCEEDED, ICMP6_TIME_EXCEED_TRANSIT) => C::Icp6sOtimeExceedTransit,
        (ICMP6_TIME_EXCEEDED, ICMP6_TIME_EXCEED_REASSEMBLY) => C::Icp6sOtimeExceedReassembly,
        (ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER) => C::Icp6sOparamprobHeader,
        (ICMP6_PARAM_PROB, ICMP6_PARAMPROB_NEXTHEADER) => C::Icp6sOparamprobNextheader,
        (ICMP6_PARAM_PROB, ICMP6_PARAMPROB_OPTION) => C::Icp6sOparamprobOption,
        (ND_REDIRECT, _) => C::Icp6sOredirect,
        _ => C::Icp6sOunknown,
    };
    icmp6stat_inc(c);
}

/// `icmp6_mtudisc_callback_register(func)`: registers a function to call on path MTU changes
/// (once: registering it again does nothing). The array grows by a copy (the C's `malloc`
/// of a record, `panic(9)` when it fails as the C's), the newest first.
pub fn icmp6_mtudisc_callback_register(func: Mtudisc6Fn) {
    let old = ICMP6_MTUDISC_CALLBACKS.update(|v| {
        let have: &[Mtudisc6Fn] = v.as_deref().unwrap_or(&[]);
        if have.iter().any(|f| core::ptr::fn_addr_eq(*f, func)) {
            return None;
        }
        // LIST_INSERT_HEAD
        let Some(new) = MArray::new(M_PCB, M_NOWAIT, have.len() + 1, |i| {
            if i == 0 { func } else { have[i - 1] }
        }) else {
            kpanic(format_args!("icmp6_mtudisc_callback_register"));
        };
        v.replace(new)
    });
    // The old array freed outside the cell.
    drop(old);
}

/// The callbacks registered, called with `sin6` and `rtableid`.
fn icmp6_mtudisc_notify(sin6: &SockaddrIn6, rtableid: u32) {
    let cbs = ICMP6_MTUDISC_CALLBACKS.read();
    if let Some(a) = cbs.as_ref() {
        for f in a.iter() {
            f(sin6, rtableid);
        }
    }
}

/// `icmp6_do_error(m, type, code, param)`: the ICMPv6 error of `type_`/`code` (`param`: the
/// pointer or the MTU) about packet `m`, which it consumes: the error, ready for
/// `icmp6_reflect`, or `None` when none is sent.
pub fn icmp6_do_error(m: MbufBox, type_: u8, code: u8, param: i32) -> Option<MbufBox> {
    icmp6stat_inc(Icmp6statCounters::Icp6sError);

    // count per-type-code statistics
    icmp6_errcount(type_, code);

    let mut m = m;
    if (m.m_len().get() as usize) < IP6_HDR_LEN {
        m = m_pullup(m, IP6_HDR_LEN as i32)?;
    }
    let oip6 = ip6_load(&m);

    // If the destination address of the erroneous packet is a multicast address, or the
    // packet was sent using link-layer multicast, we should basically suppress sending an
    // error (RFC 2463, Section 2.4). We have two exceptions (the item e.2 in that section):
    // - the Packet Too Big message can be sent for path MTU discovery.
    // - the Parameter Problem Message that can be allowed an icmp6 error in the option type
    //   field. This check has been done in ip6_unknown_opt(), so we can just check the type
    //   and code.
    if (m.flags() & (M_BCAST | M_MCAST) != 0 || in6_is_addr_multicast(&oip6.ip6_dst))
        && (type_ != ICMP6_PACKET_TOO_BIG
            && (type_ != ICMP6_PARAM_PROB || code != ICMP6_PARAMPROB_OPTION))
    {
        // freeit
        return None;
    }

    // RFC 2463, 2.4 (e.5): source address check.
    // XXX: the case of anycast source?
    if in6_is_addr_unspecified(&oip6.ip6_src) || in6_is_addr_multicast(&oip6.ip6_src) {
        return None;
    }

    // If we are about to send ICMPv6 against ICMPv6 error/redirect, don't do it.
    let mut nxt = -1;
    if let Some(off) = ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt)
        && nxt == IPPROTO_ICMPV6
    {
        let Some((mm, at, o)) = ip6_exthdr_get(m, off, ICMP6_HDR_LEN as i32) else {
            icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
            return None;
        };
        m = mm;
        let Some(&icp_type) = ip6_exthdr_bytes(&m, at, o, ICMP6_HDR_LEN).and_then(|b| b.first())
        else {
            return None;
        };
        if icp_type < ICMP6_ECHO_REQUEST || icp_type == ND_REDIRECT {
            // ICMPv6 error. Special case: for redirect (which is informational) we must not
            // send icmp6 error.
            icmp6stat_inc(Icmp6statCounters::Icp6sCanterror);
            return None;
        }
        // ICMPv6 informational - send the error
    }
    // else: non-ICMPv6 - send the error

    // Finally, do rate limitation check.
    if icmp6_ratelimit(&oip6.ip6_src, type_, code) {
        icmp6stat_inc(Icmp6statCounters::Icp6sToofreq);
        return None;
    }

    // OK, ICMP6 can be generated.

    if m.m_pkthdr().len.get() >= ICMPV6_PLD_MAXLEN as i32 {
        m_adj(&m, ICMPV6_PLD_MAXLEN as i32 - m.m_pkthdr().len.get());
    }

    // The quoted header's embedded scope zones cleared (the C clears them through `oip6`
    // after the prepend, in the same bytes): only when it has one.
    if in6_is_scope_embed(&oip6.ip6_src) || in6_is_scope_embed(&oip6.ip6_dst) {
        let mut q = oip6;
        if in6_is_scope_embed(&q.ip6_src) {
            q.ip6_src.set_s6_addr16(1, 0);
        }
        if in6_is_scope_embed(&q.ip6_dst) {
            q.ip6_dst.set_s6_addr16(1, 0);
        }
        if !ip6_store(&mut m, &q) {
            icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
            return None;
        }
    }

    let preplen = IP6_HDR_LEN + ICMP6_HDR_LEN;
    let mut m = m_prepend(m, preplen as i32, M_DONTWAIT)?;
    if (m.m_len().get() as usize) < preplen {
        m = m_pullup(m, preplen as i32)?;
    }

    let mut hdr = [0u8; IP6_HDR_LEN + ICMP6_HDR_LEN];
    let nip6 = Ip6Hdr {
        ip6_src: oip6.ip6_src,
        ip6_dst: oip6.ip6_dst,
        ..Ip6Hdr::default()
    };
    let mut icmp6 = Icmp6Hdr::zeroed();
    icmp6.icmp6_type = type_;
    icmp6.icmp6_code = code;
    icmp6.set_icmp6_pptr((param as u32).to_be());
    let _ = pod::put(&mut hdr, 0, &nip6);
    let _ = pod::put(&mut hdr, IP6_HDR_LEN, &icmp6);
    // The prepended bytes are the first mbuf's own (m_prepend's or m_pullup's fresh room).
    if !ip6_set_bytes(&mut m, 0, &hdr) {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return None;
    }

    // icmp6_reflect() is designed to be in the input path. icmp6_error() can be called from
    // both input and output path, and if we are in output path rcvif could contain bogus
    // value. clear m->m_pkthdr.ph_ifidx for safety, we should have enough scope information in
    // ip header (nip6).
    m.m_pkthdr().ph_ifidx.set(0);

    icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, type_);

    Some(m)
}

/// `icmp6_error(m, type, code, param)`: generate an error packet of type error in response to
/// bad IP6 packet `m` (consumed), under the net lock (`net`).
pub fn icmp6_error(m: MbufBox, type_: u8, code: u8, param: i32, net: &NetHeld<'_>) {
    #[cfg(test)]
    seen::record(type_, code, param);

    if let Some(n) = icmp6_do_error(m, type_, code, param) {
        // header order: IPv6 - ICMPv6
        let mut n = Some(n);
        if icmp6_reflect(&mut n, IP6_HDR_LEN, None, net).is_ok()
            && let Some(n) = n
        {
            ip6_send(n);
        }
    }
}

/// `icmp6_input(mp, offp, proto, af, ns)`: process a received ICMP6 message, ICMPv6's
/// `pr_input`, under the exclusive net lock (`net`, `ip6intr`'s: ICMPv6 is not
/// `PR_MPINPUT`).
pub fn icmp6_input<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    proto: i32,
    af: i32,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let Some(m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let off = *offp;
    let icmp6len = m.m_pkthdr().len.get() - off;

    // Locate icmp6 structure in mbuf, and check that not corrupted and of at least minimum
    // length
    if icmp6len < ICMP6_HDR_LEN as i32 {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return IPPROTO_DONE;
    }

    // calculate the checksum
    let Some((mut m, at, o)) = ip6_exthdr_get(m, off, ICMP6_HDR_LEN as i32) else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return IPPROTO_DONE;
    };
    let Some(icmp6) =
        ip6_exthdr_bytes(&m, at, o, ICMP6_HDR_LEN).and_then(|b| pod::at::<Icmp6Hdr>(b, 0))
    else {
        return IPPROTO_DONE;
    };
    let icmp6_type = icmp6.icmp6_type;
    let code = icmp6.icmp6_code;

    if in6_cksum(&m, IPPROTO_ICMPV6 as u8, off as u32, icmp6len as u32) != 0 {
        icmp6stat_inc(Icmp6statCounters::Icp6sChecksum);
        return IPPROTO_DONE;
    }

    'raw: {
        // NPF > 0
        let pff = &m.m_pkthdr().pf.flags;
        if pff.get() & PF_TAG_DIVERTED != 0 {
            match icmp6_type {
                // These ICMP6 types map to other connections. They must be delivered to
                // pr_ctlinput() also for diverted connections.
                ICMP6_DST_UNREACH | ICMP6_PACKET_TOO_BIG | ICMP6_TIME_EXCEEDED
                | ICMP6_PARAM_PROB => {
                    // Do not use the divert-to property of the TCP or UDP rule when doing the
                    // PCB lookup for the raw socket.
                    pff.set(pff.get() & !PF_TAG_DIVERTED);
                }
                _ => break 'raw,
            }
        }

        // NCARP > 0: carp_lsdrop of echo requests; not configured.
        icmp6stat_inc_hist(Icmp6statCounters::Icp6sInhist, icmp6_type);

        // A copy of the packet for a handler that consumes it, the packet going on to the
        // raw sockets; without one, the handler gets the packet itself ("give up local").
        macro_rules! local {
            ($handler:expr) => {{
                match m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT) {
                    Some(n) => {
                        ($handler)(n);
                        // m stays.
                        Next::Raw
                    }
                    None => {
                        // give up local
                        ($handler)(m);
                        return IPPROTO_DONE;
                    }
                }
            }};
        }

        let next = match icmp6_type {
            ICMP6_DST_UNREACH => match code {
                ICMP6_DST_UNREACH_NOROUTE => Next::Deliver(PRC_UNREACH_NET),
                // is this a good code?
                ICMP6_DST_UNREACH_ADMIN => Next::Deliver(PRC_UNREACH_PROTOCOL),
                ICMP6_DST_UNREACH_ADDR => Next::Deliver(PRC_HOSTDEAD),
                // I mean "source address was incorrect."
                ICMP6_DST_UNREACH_BEYONDSCOPE => Next::Deliver(PRC_PARAMPROB),
                ICMP6_DST_UNREACH_NOPORT => Next::Deliver(PRC_UNREACH_PORT),
                _ => Next::Badcode,
            },

            // MTU is checked in icmp6_mtudisc_update. Updating the path MTU will be done after
            // examining intermediate extension headers.
            ICMP6_PACKET_TOO_BIG => Next::Deliver(PRC_MSGSIZE),

            ICMP6_TIME_EXCEEDED => match code {
                ICMP6_TIME_EXCEED_TRANSIT => Next::Deliver(PRC_TIMXCEED_INTRANS),
                ICMP6_TIME_EXCEED_REASSEMBLY => Next::Deliver(PRC_TIMXCEED_REASS),
                _ => Next::Badcode,
            },

            ICMP6_PARAM_PROB => match code {
                ICMP6_PARAMPROB_NEXTHEADER => Next::Deliver(PRC_UNREACH_PROTOCOL),
                ICMP6_PARAMPROB_HEADER | ICMP6_PARAMPROB_OPTION => Next::Deliver(PRC_PARAMPROB),
                _ => Next::Badcode,
            },

            ICMP6_ECHO_REQUEST => {
                if code != 0 {
                    Next::Badcode
                } else {
                    match icmp6_echo(m, off, &icmp6, net) {
                        Some(mm) => {
                            m = mm;
                            Next::Raw
                        }
                        // The packet went to the querier only.
                        None => return IPPROTO_DONE,
                    }
                }
            }

            ICMP6_ECHO_REPLY => {
                if code != 0 {
                    Next::Badcode
                } else {
                    Next::Raw
                }
            }

            MLD_LISTENER_QUERY | MLD_LISTENER_REPORT => {
                if icmp6len < size_of::<MldHdr>() as i32 {
                    Next::Badlen
                } else {
                    local!(|n| mld6_input(n, off, net))
                }
            }

            MLD_LISTENER_DONE => {
                if icmp6len < size_of::<MldHdr>() as i32 {
                    // necessary?
                    Next::Badlen
                } else {
                    // nothing to be done in kernel
                    Next::Raw
                }
            }

            // XXX: these two are experimental. not officially defined.
            // XXX: per-interface statistics?
            // just pass it to applications
            MLD_MTRACE_RESP | MLD_MTRACE => Next::Raw,

            // IPv6 Node Information Queries are not supported (ICMP6_FQDN_QUERY).
            ICMP6_WRUREQUEST | ICMP6_WRUREPLY => Next::Raw,

            ND_ROUTER_SOLICIT | ND_ROUTER_ADVERT => {
                if code != 0 {
                    Next::Badcode
                } else if (icmp6_type == ND_ROUTER_SOLICIT
                    && icmp6len < size_of::<NdRouterSolicit>() as i32)
                    || (icmp6_type == ND_ROUTER_ADVERT
                        && icmp6len < size_of::<NdRouterAdvert>() as i32)
                {
                    Next::Badlen
                } else {
                    local!(|n| nd6_rtr_cache(n, off, icmp6len, icmp6_type, net))
                }
            }

            ND_NEIGHBOR_SOLICIT => {
                if code != 0 {
                    Next::Badcode
                } else if icmp6len < size_of::<NdNeighborSolicit>() as i32 {
                    Next::Badlen
                } else {
                    local!(|n| nd6_ns_input(n, off, icmp6len, net))
                }
            }

            ND_NEIGHBOR_ADVERT => {
                if code != 0 {
                    Next::Badcode
                } else if icmp6len < size_of::<NdNeighborAdvert>() as i32 {
                    Next::Badlen
                } else {
                    local!(|n| nd6_na_input(n, off, icmp6len, net))
                }
            }

            ND_REDIRECT => {
                if code != 0 {
                    Next::Badcode
                } else if icmp6len < size_of::<NdRedirect>() as i32 {
                    Next::Badlen
                } else {
                    local!(|n| icmp6_redirect_input::<G>(n, off, net))
                }
            }

            ICMP6_ROUTER_RENUMBERING => {
                if code != ICMP6_ROUTER_RENUMBERING_COMMAND
                    && code != ICMP6_ROUTER_RENUMBERING_RESULT
                {
                    Next::Badcode
                } else if icmp6len < size_of::<Icmp6RouterRenum>() as i32 {
                    Next::Badlen
                } else {
                    Next::Raw
                }
            }

            _ => {
                if icmp6_type < ICMP6_ECHO_REQUEST {
                    // ICMPv6 error: MUST deliver it by spec...
                    Next::Deliver(PRC_NCMDS as i32)
                } else {
                    // ICMPv6 informational: MUST not deliver
                    Next::Raw
                }
            }
        };

        match next {
            Next::Deliver(prc) => match icmp6_notify_error::<G>(m, off, icmp6len, prc) {
                Some(mm) => m = mm,
                // In this case, m should've been freed.
                None => return IPPROTO_DONE,
            },
            Next::Badcode => icmp6stat_inc(Icmp6statCounters::Icp6sBadcode),
            Next::Badlen => icmp6stat_inc(Icmp6statCounters::Icp6sBadlen),
            Next::Raw => {}
        }
    }
    // raw: deliver the packet to appropriate sockets (rip6_input).
    *mp = Some(m);
    match inet6_protocol::<G>(IPPROTO_RAW) {
        Some(raw) => raw.input(mp, offp, proto, af, ns, net),
        None => {
            drop(mp.take());
            IPPROTO_DONE
        }
    }
}

/// The echo request case of `icmp6_input`: copy the packet `m` to send to two data paths,
/// userland socket(s) (`m`, given back for the raw sockets) and the querier (the echo reply,
/// sent); `None` when `m` itself was used up as the reply (no copy could be made).
fn icmp6_echo(m: MbufBox, off: i32, icmp6: &Icmp6Hdr, net: &NetHeld<'_>) -> Option<MbufBox> {
    let mut m = Some(m);
    if let Some((n, noff)) = icmp6_echo_reply(&mut m, off, icmp6) {
        icmp6stat_inc(Icmp6statCounters::Icp6sReflect);
        icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, ICMP6_ECHO_REPLY);
        let mut n = Some(n);
        if icmp6_reflect(&mut n, noff, None, net).is_ok()
            && let Some(n) = n
        {
            ip6_send(n);
        }
    }
    m
}

/// The echo reply of the request `*m` (its ICMPv6 header `icmp6` at `off`), with the offset
/// of its ICMPv6 header, turned into a reply but not yet reflected; `*m` left for the raw
/// sockets, or taken as the reply when no copy can be had ("give up local"). `None` when no
/// reply is sent ("give up remote", or no buffer for the reply's header).
fn icmp6_echo_reply(
    m: &mut Option<MbufBox>,
    off: i32,
    icmp6: &Icmp6Hdr,
) -> Option<(MbufBox, usize)> {
    let mm = m.as_mut()?;
    let (n, noff) = match m_copym(mm, 0, M_COPYALL, M_DONTWAIT) {
        // Give up local: the request itself becomes the reply.
        None => (m.take()?, off as usize),
        Some(n0) => {
            // If the first mbuf is shared, or the first mbuf is too short, copy the first
            // part of the data into a fresh mbuf. Otherwise, we will wrongly overwrite both
            // copies.
            if n0.flags() & M_EXT == 0 && n0.m_len().get() as usize >= off as usize + ICMP6_HDR_LEN
            {
                (n0, off as usize)
            } else {
                const MAXLEN: usize = IP6_HDR_LEN + ICMP6_HDR_LEN;
                // Prepare an internal mbuf. m_pullup() doesn't always copy the length we
                // specified. (MAXLEN < MCLBYTES: never "give up remote".)
                let mut nn = m_gethdr(M_DONTWAIT, i32::from(n0.m_type().get()));
                if let Some(x) = nn.as_mut()
                    && MAXLEN >= MHLEN as usize
                {
                    mclget(x, M_DONTWAIT);
                    if x.flags() & M_EXT == 0 {
                        nn = None;
                    }
                }
                match nn {
                    None => {
                        // Give up local
                        drop(n0);
                        (m.take()?, off as usize)
                    }
                    Some(mut n) => {
                        let mut n0 = n0;
                        m_move_pkthdr(&mut n, &mut n0);

                        // Copy IPv6 and ICMPv6 only.
                        let mut hdr = [0u8; MAXLEN];
                        let ip6 = ip6_load(m.as_deref()?);
                        let _ = pod::put(&mut hdr, 0, &ip6);
                        let _ = pod::put(&mut hdr, IP6_HDR_LEN, icmp6);
                        n.m_len().set(MAXLEN as u32);
                        if !ip6_set_bytes(&mut n, 0, &hdr) {
                            return None;
                        }

                        // Adjust mbuf. ip6_plen will be adjusted in ip6_output().
                        // n->m_pkthdr.len == n0->m_pkthdr.len at this point.
                        let ph = n.m_pkthdr();
                        ph.len
                            .set(ph.len.get() + MAXLEN as i32 - (off + ICMP6_HDR_LEN as i32));
                        m_adj(&n0, off + ICMP6_HDR_LEN as i32);
                        drop(n.m_next.set(n0));
                        (n, IP6_HDR_LEN)
                    }
                }
            }
        }
    };

    // deliverecho: the reply's type and code.
    let mut n = n;
    if !ip6_set_bytes(&mut n, noff as i32, &[ICMP6_ECHO_REPLY, 0]) {
        return None;
    }
    Some((n, noff))
}

/// The `len` bytes at `off` of `m` made contiguous ([`ip6_exthdr_get`]) and copied as a `T`;
/// `None` (the chain freed, `icp6s_tooshort` counted) when the chain does not hold them.
fn exthdr<T: hal::user::AbiPod>(m: MbufBox, off: i32) -> Option<(MbufBox, T)> {
    let len = size_of::<T>();
    let Some((m, at, o)) = ip6_exthdr_get(m, off, len as i32) else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return None;
    };
    let v = ip6_exthdr_bytes(&m, at, o, len).and_then(|b| pod::at::<T>(b, 0))?;
    Some((m, v))
}

/// `icmp6_notify_error(m, off, icmp6len, code)`: the ICMPv6 error at `off` of `m` handed to
/// the control input of the protocol of the packet it quotes, with `code` (a `PRC_*`). The
/// packet back when it is still ours (the C's 0), `None` when it was freed (-1).
fn icmp6_notify_error<G: ProcGlue>(
    m: MbufBox,
    off: i32,
    icmp6len: i32,
    code: i32,
) -> Option<MbufBox> {
    const HDRS: usize = ICMP6_HDR_LEN + IP6_HDR_LEN;
    if icmp6len < HDRS as i32 {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return None;
    }

    let (mut m, eip6) = exthdr::<Ip6Hdr>(m, off + ICMP6_HDR_LEN as i32)?;

    // Detect the upper level protocol
    let mut nxt = eip6.ip6_nxt;
    let mut eoff = off + HDRS as i32;
    let mut finaldst: Option<In6Addr> = None;

    // XXX: should avoid infinite loop explicitly?
    loop {
        match i32::from(nxt) {
            IPPROTO_HOPOPTS | IPPROTO_DSTOPTS | IPPROTO_AH => {
                let (mm, eh) = exthdr::<Ip6Ext>(m, eoff)?;
                m = mm;
                if i32::from(nxt) == IPPROTO_AH {
                    eoff += (i32::from(eh.ip6e_len) + 2) << 2;
                } else {
                    eoff += (i32::from(eh.ip6e_len) + 1) << 3;
                }
                nxt = eh.ip6e_nxt;
            }
            IPPROTO_ROUTING => {
                // When the erroneous packet contains a routing header, we should examine the
                // header to determine the final destination. Otherwise, we can't properly
                // update information that depends on the final destination (e.g. path MTU).
                let (mm, rth) = exthdr::<Ip6Rthdr>(m, eoff)?;
                m = mm;
                let rthlen = (i32::from(rth.ip6r_len) + 1) << 3;

                // XXX: currently there is no officially defined type other than type-0. Note
                // that if the segment left field is 0, all intermediate hops must have been
                // passed.
                if rth.ip6r_segleft != 0 && i32::from(rth.ip6r_type) == IPV6_RTHDR_TYPE_0 {
                    let Some((mm, at, o)) = ip6_exthdr_get(m, eoff, rthlen) else {
                        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
                        return None;
                    };
                    m = mm;
                    let rth0 = ip6_exthdr_bytes(&m, at, o, rthlen as usize)?;
                    let len = pod::at::<Ip6Rthdr0>(rth0, 0)?.ip6r0_len;
                    // just ignore a bogus header
                    let hops = usize::from(len / 2);
                    if len % 2 == 0 && hops != 0 {
                        // The header is `rthlen` = 8 + 16 * hops bytes long, so the last of
                        // its `hops` addresses lies inside it.
                        finaldst = pod::at::<In6Addr>(
                            rth0,
                            size_of::<Ip6Rthdr0>() + (hops - 1) * size_of::<In6Addr>(),
                        );
                    }
                }
                eoff += rthlen;
                nxt = rth.ip6r_nxt;
            }
            IPPROTO_FRAGMENT => {
                let (mm, fh) = exthdr::<Ip6Frag>(m, eoff)?;
                m = mm;
                // Data after a fragment header is meaningless unless it is the first fragment,
                // but we'll go to the notify label for path MTU discovery.
                if fh.ip6f_offlg & IP6F_OFF_MASK != 0 {
                    break;
                }

                eoff += size_of::<Ip6Frag>() as i32;
                nxt = fh.ip6f_nxt;
            }
            // This case includes ESP and the No Next Header. In such cases going to the notify
            // label does not have any meaning (i.e. ctlfunc will be NULL), but we go anyway
            // since we might have to update path MTU information.
            _ => break,
        }
    }

    // notify:
    let Some((m, at, o)) = ip6_exthdr_get(m, off, HDRS as i32) else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return None;
    };
    let (icmp6, eip6) = {
        let b = ip6_exthdr_bytes(&m, at, o, HDRS)?;
        (
            pod::at::<Icmp6Hdr>(b, 0)?,
            pod::at::<Ip6Hdr>(b, ICMP6_HDR_LEN)?,
        )
    };
    let ifidx = m.m_pkthdr().ph_ifidx.get();

    let mut icmp6dst = sin6_of(finaldst.unwrap_or(eip6.ip6_dst));
    icmp6dst.sin6_scope_id = in6_addr2scopeid(ifidx, &icmp6dst.sin6_addr) as u32;
    let mut a = icmp6dst.sin6_addr;
    if in6_embedscope(&mut a, &icmp6dst, 0, 0).is_err() {
        // interface went away
        return None;
    }
    icmp6dst.sin6_addr = a;

    // retrieve parameters from the inner IPv6 header, and convert them into sockaddr
    // structures.
    let mut icmp6src = sin6_of(eip6.ip6_src);
    icmp6src.sin6_scope_id = in6_addr2scopeid(ifidx, &icmp6src.sin6_addr) as u32;
    let mut a = icmp6src.sin6_addr;
    if in6_embedscope(&mut a, &icmp6src, 0, 0).is_err() {
        // interface went away
        return None;
    }
    icmp6src.sin6_addr = a;
    icmp6src.sin6_flowinfo = eip6.ip6_flow & IPV6_FLOWLABEL_MASK;

    // NPF > 0
    if let Some(p) = pf() {
        p.pkt_addr_changed(&m);
    }

    // The quoted header's destination is the final one unless a routing header named
    // another: then the protocols' changes of it are written back where it is (the C's
    // `finaldst = &eip6->ip6_dst`).
    let in_packet = finaldst.is_none();
    let finaldst0 = finaldst.unwrap_or(eip6.ip6_dst);
    let ip6cp = Ip6CtlParam {
        m: &m,
        icmp6,
        ip6: eip6,
        off: eoff,
        src: icmp6src,
        finaldst: Cell::new(finaldst0),
        cmdarg: (icmp6.icmp6_type == ICMP6_PACKET_TOO_BIG)
            .then(|| u32::from_be(icmp6.icmp6_data32())),
        nxt,
    };

    if let Some(pr) = inet6_protocol::<G>(i32::from(nxt)) {
        let dst = sin6_bytes(&icmp6dst);
        pr.ctlinput(
            code,
            view(&dst),
            m.m_pkthdr().ph_rtableid.get(),
            CtlArg::Ip6(&ip6cp),
        );
    }
    let fd = ip6cp.finaldst.get();
    let mut m = m;
    if in_packet && fd != finaldst0 {
        let at = off + (ICMP6_HDR_LEN + offset_of!(Ip6Hdr, ip6_dst)) as i32;
        if !ip6_set_bytes(&mut m, at, &fd.s6_addr) {
            icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
            return None;
        }
    }
    Some(m)
}

/// `icmp6_mtudisc_update(ip6cp, validated)`: a Packet Too Big about the destination of
/// `ip6cp` (validated by a protocol's matching control block, or not): the path MTU updated on
/// a cloned host route, the registered callbacks told. Under the net lock (`net`).
pub fn icmp6_mtudisc_update(ip6cp: &Ip6CtlParam<'_>, validated: bool, net: &NetHeld<'_>) {
    let dst = ip6cp.finaldst.get();
    let mtu = u32::from_be(ip6cp.icmp6.icmp6_data32());

    if mtu < IPV6_MMTU {
        return;
    }

    // allow non-validated cases if memory is plenty, to make traffic from non-connected pcb
    // happy.
    let rtcount = rt_timer_queue_count(&ICMP6_MTUDISC_TIMEOUT_Q);
    if validated {
        if rtcount > ICMP6_MTUDISC_HIWAT.load(Ordering::Relaxed) as u64 {
            return;
        }
        // else if rtcount > icmp6_mtudisc_lowat: XXX nuke a victim, install the new one.
    } else if rtcount > ICMP6_MTUDISC_LOWAT.load(Ordering::Relaxed) as u64 {
        return;
    }

    let m = ip6cp.m;
    let ifidx = m.m_pkthdr().ph_ifidx.get();
    let mut sin6 = sin6_of(dst);
    // XXX normally, this won't happen
    if in6_is_addr_linklocal(&dst) {
        sin6.sin6_addr.set_s6_addr16(1, (ifidx as u16).to_be());
    }
    sin6.sin6_scope_id = in6_addr2scopeid(ifidx, &sin6.sin6_addr) as u32;

    let rtableid = m.m_pkthdr().ph_rtableid.get();
    if let Some(rt) = icmp6_mtudisc_clone(&sin6, rtableid, false, net)
        && rt.rt_flags.get() & RTF_HOST != 0
        && rt.rt_locks().get() & RTV_MTU == 0
    {
        let rtmtu = rt.rt_mtu().load(Ordering::Relaxed);
        if rtmtu > mtu || rtmtu == 0 {
            let ifp = if_get(rt.rt_ifidx.get());
            if let Some(ifp) = ifp
                && mtu < ifp.if_mtu.get()
            {
                icmp6stat_inc(Icmp6statCounters::Icp6sPmtuchg);
                let _ =
                    rt.rt_mtu()
                        .compare_exchange(rtmtu, mtu, Ordering::Relaxed, Ordering::Relaxed);
            }
            if_put(ifp);
        }
    }
    // rtfree(rt): the clone's handle dropped above.

    // Notify protocols that the MTU for this destination has changed.
    icmp6_mtudisc_notify(&sin6, rtableid);
}

/// `icmp6_reflect(mp, off, sa)`: reflect the ip6 packet back to the source. `off` points to
/// the icmp6 header, counted from the top of the mbuf. The source address comes from `sa`
/// when given. The packet is consumed (`*mp` cleared) on error: `ELOOP` past the loop count,
/// `EMSGSIZE` without room for the headers, `EHOSTUNREACH` without a source.
pub fn icmp6_reflect(
    mp: &mut Option<MbufBox>,
    off: usize,
    sa: Option<&SockaddrIn6>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    const HDRLEN: usize = IP6_HDR_LEN + ICMP6_HDR_LEN;
    let Some(mut m) = mp.take() else {
        return Err(Errno::EHOSTUNREACH);
    };

    // too short to reflect
    if off < IP6_HDR_LEN {
        return Err(Errno::EHOSTUNREACH);
    }

    let loopcnt = m.m_pkthdr().ph_loopcnt.get();
    m.m_pkthdr().ph_loopcnt.set(loopcnt.wrapping_add(1));
    if loopcnt >= M_MAXLOOP {
        return Err(Errno::ELOOP);
    }
    let rtableid = m.m_pkthdr().ph_rtableid.get();
    let pfflags = m.m_pkthdr().pf.flags.get();
    m_resethdr(&mut m);
    m.m_pkthdr().ph_rtableid.set(rtableid);
    m.m_pkthdr().pf.flags.set(pfflags & PF_TAG_GENERATED);

    // If there are extra headers between IPv6 and ICMPv6, strip off that header first.
    let mut ip6 = ip6_load(&m);
    if off > IP6_HDR_LEN {
        let l = off - IP6_HDR_LEN;
        m_adj(&m, l as i32);
    }
    if (m.m_len().get() as usize) < HDRLEN {
        let Some(mm) = m_pullup(m, HDRLEN as i32) else {
            return Err(Errno::EMSGSIZE);
        };
        m = mm;
    }
    ip6.ip6_nxt = IPPROTO_ICMPV6 as u8;

    let t = ip6.ip6_dst;
    // ip6_input() drops a packet if its src is multicast. So, the src is never multicast.
    ip6.ip6_dst = ip6.ip6_src;

    // XXX: make sure to embed scope zone information, using already embedded IDs or the
    // received interface (if any). Note that rcvif may be NULL.
    // TODO: scoped routing case (XXX).
    let sa6_src = sin6_of(ip6.ip6_dst);
    let sa6_dst = sin6_of(t);

    let mut src: Option<In6Addr> = None;
    let key = match sa {
        Some(sa) => *sa,
        None => {
            // If the incoming packet was addressed directly to us (i.e. unicast), use dst as
            // the src for the reply. The IN6_IFF_TENTATIVE|IN6_IFF_DUPLICATED case would be
            // VERY rare, but is possible (for example) when we encounter an error while
            // forwarding procedure destined to a duplicated address of ours.
            let b = sin6_bytes(&sa6_dst);
            let rt = rtalloc(view(&b), 0, rtableid, net);
            if rtisvalid(rt.as_deref())
                && let Some(r) = rt.as_deref()
                && r.rt_flags.get() & RTF_LOCAL != 0
                && r.ifa().and_then(In6Ifaddr::of).is_some_and(|ia6| {
                    ia6.ia6_flags.get() & (IN6_IFF_ANYCAST | IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED)
                        == 0
                })
            {
                src = Some(t);
            }
            // rtfree(rt)
            drop(rt);
            sa6_src
        }
    };

    if src.is_none() {
        // This case matches to multicasts, our anycast, or unicasts that we do not own. Select
        // a source address based on the source address of the erroneous packet.
        let b = sin6_bytes(&key);
        let rt = rtalloc(view(&b), RT_RESOLVE, rtableid, net);
        let Some(r) = rt.as_deref().filter(|r| rtisvalid(Some(r))) else {
            return Err(Errno::EHOSTUNREACH);
        };
        // A valid route has its address (`rt_ifa`).
        let Some(rifa) = r.ifa() else {
            return Err(Errno::EHOSTUNREACH);
        };
        let ia6 = rifa
            .ifa_ifp
            .get()
            .and_then(|ifp| in6_ifawithscope(ifp, &t, rtableid, Some(r)));
        let mut s = match ia6.or_else(|| In6Ifaddr::of(rifa)) {
            Some(ia6) => ia6_in6(ia6),
            None => return Err(Errno::EHOSTUNREACH),
        };

        // route sourceaddr may override src address selection
        if r.rt_flags.get() & RTF_GATEWAY != 0
            && let Some(sourceaddr) = rtable_getsource(rtableid, AF_INET6, net)
            && let Some(ifa) = ifa_ifwithaddr(net, sourceaddr.bytes(), rtableid)
            && ifa
                .ifa_ifp
                .get()
                .is_some_and(|ifp| ifp.if_flags.get() & IFF_UP != 0)
        {
            // An `AF_INET6` source is a `sockaddr_in6`.
            s = pod::at::<SockaddrIn6>(sourceaddr.bytes(), 0)
                .unwrap_or_default()
                .sin6_addr;
        }
        src = Some(s);
        // rtfree(rt)
        drop(rt);
    }

    ip6.ip6_src = src.unwrap_or(t);

    ip6.ip6_flow = 0;
    ip6.set_ip6_vfc((ip6.ip6_vfc() & !IPV6_VERSION_MASK) | IPV6_VERSION);
    ip6.ip6_nxt = IPPROTO_ICMPV6 as u8;
    ip6.ip6_hlim = IP6_DEFHLIM.load(Ordering::Relaxed) as u8;

    // The header and the checksum's two bytes (icmp6->icmp6_cksum = 0), in the first mbuf.
    let mut hdr = [0u8; IP6_HDR_LEN];
    let _ = pod::put(&mut hdr, 0, &ip6);
    if !ip6_set_bytes(&mut m, 0, &hdr)
        || !ip6_set_bytes(
            &mut m,
            (IP6_HDR_LEN + offset_of!(Icmp6Hdr, icmp6_cksum)) as i32,
            &[0, 0],
        )
    {
        return Err(Errno::EMSGSIZE);
    }
    m.m_pkthdr().csum_flags.set(M_ICMP_CSUM_OUT);

    // XXX option handling

    m.m_flags().set(m.flags() & !(M_BCAST | M_MCAST));
    *mp = Some(m);
    Ok(())
}

/// `icmp6_fasttimo()`: the ICMPv6 fast timeout (MLD's timers).
pub fn icmp6_fasttimo() {
    mld6_fasttimo();
}

/// `icmp6_redirect_input(m, off)`: a received redirect at `off` of `m` (consumed), under the
/// exclusive net lock (`net`).
pub fn icmp6_redirect_input<G: ProcGlue>(m: MbufBox, off: i32, net: &NetHeld<'_>) {
    let ip6 = ip6_load(&m);
    let icmp6len = i32::from(u16::from_be(ip6.ip6_plen));
    let i_am_router = IP6_FORWARDING.load(Ordering::Relaxed) != 0;
    let src6 = ip6.ip6_src;
    let rtableid = m.m_pkthdr().ph_rtableid.get();

    // The receiving interface gone: the packet freed (the C returns without freeing it).
    let Some(ifp) = if_get(m.m_pkthdr().ph_ifidx.get()) else {
        return;
    };

    // `freeit` (false) and `bad` (true) of the C.
    let bad = 'out: {
        // if we are router, we don't update route by icmp6 redirect
        if i_am_router {
            break 'out false;
        }
        if ifp.if_xflags.get() & IFXF_AUTOCONF6 == 0 {
            break 'out false;
        }

        let Some((m, at, o)) = ip6_exthdr_get(m, off, icmp6len) else {
            icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
            if_put(Some(ifp));
            return;
        };
        let Some(msg) = ip6_exthdr_bytes(&m, at, o, icmp6len as usize) else {
            break 'out false;
        };
        let Some(nd_rd) = pod::at::<NdRedirect>(msg, 0) else {
            break 'out false;
        };
        let mut redtgt6 = nd_rd.nd_rd_target;
        let mut reddst6 = nd_rd.nd_rd_dst;

        let zone = (ifp.if_index.get() as u16).to_be();
        if in6_is_addr_linklocal(&redtgt6) {
            redtgt6.set_s6_addr16(1, zone);
        }
        if in6_is_addr_linklocal(&reddst6) {
            reddst6.set_s6_addr16(1, zone);
        }

        // validation
        if !in6_is_addr_linklocal(&src6) {
            break 'out true;
        }
        if ip6.ip6_hlim != 255 {
            break 'out true;
        }
        if in6_is_addr_multicast(&reddst6) {
            break 'out true;
        }

        {
            // ip6->ip6_src must be equal to gw for icmp6->icmp6_reddst
            let b = sin6_bytes(&sin6_of(reddst6));
            let Some(rt) = rtalloc(view(&b), 0, rtableid, net) else {
                break 'out true;
            };
            let gw6 = rt.with_gateway(|g| {
                g.filter(|g| g.get(1) == Some(&AF_INET6))
                    .and_then(|g| pod::at::<SockaddrIn6>(g, 0))
                    .map(|s| s.sin6_addr)
            });
            // rtfree(rt)
            drop(rt);
            if gw6 != Some(src6) {
                break 'out true;
            }
        }

        // router case; on-link destination case
        let is_router = in6_is_addr_linklocal(&redtgt6);
        let is_onlink = redtgt6 == reddst6;
        if !is_router && !is_onlink {
            break 'out true;
        }

        // validation passed

        let opts = &msg[size_of::<NdRedirect>()..];
        let mut ndopts = NdOpts::default();
        if !nd6_options(opts, &mut ndopts) {
            // nd6_options have incremented stats
            break 'out false;
        }

        let mut lladdr: Option<&[u8]> = None;
        let mut lladdrlen = 0;
        if let Some(o) = ndopts.nd_opts_tgt_lladdr {
            // An offset nd6_options found in `opts`.
            let (a, len) = nd6_opt_lladdr(opts, o);
            lladdr = Some(a);
            lladdrlen = len;
        }

        if lladdr.is_some() && ((i32::from(ifp.if_addrlen.get()) + 2 + 7) & !7) != lladdrlen {
            break 'out true;
        }

        // RFC 2461 8.3
        nd6_cache_lladdr(
            ifp,
            &redtgt6,
            lladdr,
            i32::from(ND_REDIRECT),
            i32::from(if is_onlink {
                ND_REDIRECT_ONLINK
            } else {
                ND_REDIRECT_ROUTER
            }),
            i_am_router,
            net,
        );

        if !is_onlink {
            // better router case. perform rtredirect.
            // do not install redirect route, if the number of entries is too much (> hiwat).
            // note that, the node (= host) will work just fine even if we do not install
            // redirect route (there will be additional hops, though).
            let rtcount = rt_timer_queue_count(&ICMP6_REDIRECT_TIMEOUT_Q);
            if rtcount >= IP6_MAXDYNROUTES.load(Ordering::Relaxed) as u64 {
                break 'out false;
            }

            let sdst = sin6_bytes(&sin6_of(reddst6));
            let sgw = sin6_bytes(&sin6_of(redtgt6));
            let ssrc = sin6_bytes(&sin6_of(src6));
            let mut newrt = None;
            rtredirect(
                view(&sdst),
                view(&sgw),
                view(&ssrc),
                Some(&mut newrt),
                rtableid,
                net,
            );
            if let Some(nrt) = newrt.as_ref()
                && ICMP6_REDIRTIMEOUT.load(Ordering::Relaxed) > 0
            {
                let _ = rt_timer_add(nrt, &ICMP6_REDIRECT_TIMEOUT_Q, rtableid);
            }
            // rtfree(newrt)
            drop(newrt);
        }
        // finally update cached route in each socket via pfctlinput
        let sdst = sin6_bytes(&sin6_of(reddst6));
        pfctlinput::<G>(PRC_REDIRECT_HOST, view(&sdst));
        false
    };

    // freeit / bad:
    if_put(Some(ifp));
    if bad {
        icmp6stat_inc(Icmp6statCounters::Icp6sBadredirect);
    }
}

/// `icmp6_redirect_output(m0, rt)`: a redirect about the forwarded packet `m0` (consumed)
/// along route `rt`, under the net lock (`net`).
pub fn icmp6_redirect_output(m0: MbufBox, rt: &Rtentry, net: &NetHeld<'_>) {
    let i_am_router = IP6_FORWARDING.load(Ordering::Relaxed) != 0;

    icmp6_errcount(ND_REDIRECT, 0);

    // if we are not router, we don't send icmp6 redirect
    if !i_am_router {
        return;
    }

    // sanity check
    if !rtisvalid(Some(rt)) {
        return;
    }

    let ifp = if_get(rt.rt_ifidx.get());
    let Some(ifp) = ifp else {
        return;
    };

    'fail: {
        // Address check: the source address must identify a neighbor, and the destination
        // address must not be a multicast address [RFC 2461, sec 8.2]
        let sip6 = ip6_load(&m0);
        let mut src_sa = sin6_of(sip6.ip6_src);
        // we don't currently use sin6_scope_id, but eventually use it
        src_sa.sin6_scope_id = in6_addr2scopeid(ifp.if_index.get(), &sip6.ip6_src) as u32;
        if !nd6_is_addr_neighbor(&src_sa, ifp, net) {
            break 'fail;
        }
        if in6_is_addr_multicast(&sip6.ip6_dst) {
            break 'fail; // what should we do here?
        }

        // rate limit
        if icmp6_ratelimit(&sip6.ip6_src, ND_REDIRECT, 0) {
            break 'fail;
        }

        // Since we are going to append up to 1280 bytes (= IPV6_MMTU), we almost always ask
        // for an mbuf cluster for simplicity. (MHLEN < IPV6_MMTU is almost always true)
        let Some(mut m) = m_gethdr(M_DONTWAIT, MT_HEADER) else {
            break 'fail;
        };
        if IPV6_MMTU as usize >= MHLEN as usize {
            mclget(&mut m, M_DONTWAIT);
        }
        m.m_pkthdr().ph_ifidx.set(0);
        m.m_len().set(0);
        let maxlen = (IPV6_MMTU as usize).min(m_trailingspace(&m).max(0) as usize);
        let addrlen = usize::from(ifp.if_addrlen.get());
        // just for safety
        if maxlen < IP6_HDR_LEN + ICMP6_HDR_LEN + ((size_of::<NdOptHdr>() + addrlen + 7) & !7) {
            break 'fail;
        }

        // get ip6 linklocal address for ifp(my outgoing interface).
        let Some(ia6) = in6ifa_ifpforlinklocal(
            ifp,
            IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED | IN6_IFF_ANYCAST,
        ) else {
            break 'fail;
        };
        let ifp_ll6 = ia6_in6(ia6);

        // get ip6 linklocal address for the router.
        let gateway_ll6 = if rt.rt_flags.get() & RTF_GATEWAY != 0 {
            // An inet6 gateway route's gateway is a `sockaddr_in6`.
            rt.with_gateway(|g| g.and_then(|g| pod::at::<SockaddrIn6>(g, 0)))
                .map(|s| s.sin6_addr)
                .filter(in6_is_addr_linklocal)
        } else {
            None
        };

        // ip6
        let mut ip6 = Ip6Hdr {
            // ip6->ip6_plen will be set later
            ip6_nxt: IPPROTO_ICMPV6 as u8,
            ip6_hlim: 255,
            // ip6->ip6_src must be linklocal addr for my outgoing if.
            ip6_src: ifp_ll6,
            ip6_dst: sip6.ip6_src,
            ..Ip6Hdr::default()
        };
        ip6.set_ip6_vfc(IPV6_VERSION);

        // ND Redirect
        let mut nd_rd = NdRedirect {
            nd_rd_hdr: Icmp6Hdr::zeroed(),
            nd_rd_target: In6Addr::default(),
            nd_rd_dst: In6Addr::default(),
        };
        nd_rd.nd_rd_hdr.icmp6_type = ND_REDIRECT;
        nd_rd.nd_rd_hdr.icmp6_code = 0;
        nd_rd.nd_rd_hdr.set_icmp6_data32(0); // nd_rd_reserved
        let nexthop = if rt.rt_flags.get() & RTF_GATEWAY != 0 {
            // nd_rd->nd_rd_target must be a link-local address in better router cases.
            let Some(nh) = gateway_ll6 else {
                break 'fail;
            };
            nd_rd.nd_rd_target = nh;
            nd_rd.nd_rd_dst = sip6.ip6_dst;
            nh
        } else {
            // make sure redtgt == reddst
            nd_rd.nd_rd_target = sip6.ip6_dst;
            nd_rd.nd_rd_dst = sip6.ip6_dst;
            sip6.ip6_dst
        };

        // The message up to the redirected header option, built here and written to the mbuf
        // in one go (the C writes through pointers into it). The target link-layer option is
        // at most 8 * ((2 + 255 + 7) / 8) = 264 bytes.
        let mut buf = [0u8; IP6_HDR_LEN + size_of::<NdRedirect>() + 264 + 8];
        let mut p = IP6_HDR_LEN + size_of::<NdRedirect>();

        // target lladdr option
        let len = (size_of::<NdOptHdr>() + addrlen + 7) & !7; // round by 8
        // safety check
        if len + p <= maxlen {
            let nrt = nd6_lookup(&nexthop, false, Some(ifp), ifp.if_rdomain.get(), net);
            if let Some(nrt) = nrt.as_deref()
                && nrt.rt_flags.get() & (RTF_GATEWAY | RTF_LLINFO) == RTF_LLINFO
            {
                // A link-layer gateway is a `sockaddr_dl`: its family, address length, name
                // length, and LLADDR(sdl), `if_addrlen` bytes (zeros past a short one).
                let lladdr = nrt.with_gateway(|g| {
                    let g = g?;
                    let (family, nlen, alen) = (*g.get(1)?, *g.get(5)?, *g.get(6)?);
                    if family != AF_LINK || alen == 0 {
                        return None;
                    }
                    let mut a = [0u8; 255];
                    let src = g.get(SDL_DATA_OFFSET + usize::from(nlen)..).unwrap_or(&[]);
                    let n = src.len().min(addrlen);
                    a[..n].copy_from_slice(&src[..n]);
                    Some(a)
                });
                if let Some(a) = lladdr {
                    let hdr = NdOptHdr {
                        nd_opt_type: ND_OPT_TARGET_LINKADDR,
                        nd_opt_len: (len >> 3) as u8,
                    };
                    let _ = pod::put(&mut buf, p, &hdr);
                    let at = p + size_of::<NdOptHdr>();
                    buf[at..at + addrlen].copy_from_slice(&a[..addrlen]);
                    p += len;
                }
            }
            // rtfree(nrt)
            drop(nrt);
        }

        m.m_pkthdr().len.set(p as i32);
        m.m_len().set(p as u32);

        // just to be safe
        let mut m0 = Some(m0);
        if p <= maxlen {
            // redirected header option
            //
            // compute the maximum size for icmp6 redirect header option.
            // XXX room for auth header?
            let mut len = (maxlen - p) & !7;

            // Redirected header option spec (RFC2461 4.6.3) talks nothing about padding/
            // truncate rule for the original IP packet. From the discussion on IPv6imp in
            // Feb 1999, the consensus was:
            // - "attach as much as possible" is the goal
            // - pad if not aligned (original size can be guessed by original ip6 header)
            // Following code adds the padding if it is simple enough, and truncates if not.
            let hdrlen = size_of::<NdOptRdHdr>();
            if let Some(o) = m0.as_ref() {
                let pl = o.m_pkthdr().len.get();
                if ((len - hdrlen) as i64) < i64::from(pl) {
                    // not enough room, truncate
                    m_adj(o, (len - hdrlen) as i32 - pl);
                } else {
                    // enough room, truncate if not aligned. we don't pad here for simplicity.
                    let extra = pl % 8;
                    if extra != 0 {
                        // truncate
                        m_adj(o, -extra);
                    }
                    len = o.m_pkthdr().len.get() as usize + hdrlen;
                }
            }

            let nd_opt_rh = NdOptRdHdr {
                nd_opt_rh_type: ND_OPT_REDIRECTED_HEADER,
                nd_opt_rh_len: (len >> 3) as u8,
                ..NdOptRdHdr::default()
            };
            let _ = pod::put(&mut buf, p, &nd_opt_rh);
            p += hdrlen;

            // connect m0 to m
            let rest = m0.as_ref().map_or(0, |o| o.m_pkthdr().len.get());
            m.m_pkthdr().len.set(p as i32 + rest);
            m.m_len().set(p as u32);
        } else {
            // noredhdropt: m_freem(m0)
            drop(m0.take());
        }

        // The scope zones of link-local addresses are meaningful only inside the node.
        for a in [
            &mut ip6.ip6_src,
            &mut ip6.ip6_dst,
            &mut nd_rd.nd_rd_target,
            &mut nd_rd.nd_rd_dst,
        ] {
            if in6_is_addr_linklocal(a) {
                a.set_s6_addr16(1, 0);
            }
        }

        ip6.ip6_plen = ((m.m_pkthdr().len.get() as usize - IP6_HDR_LEN) as u16).to_be();

        // nd_rd->nd_rd_cksum = 0: the message was built zeroed.
        let _ = pod::put(&mut buf, 0, &ip6);
        let _ = pod::put(&mut buf, IP6_HDR_LEN, &nd_rd);
        // A fresh mbuf: `p` bytes fit its `maxlen` (the options are added only when they do).
        if !ip6_set_bytes(&mut m, 0, &buf[..p]) {
            break 'fail;
        }
        if let Some(o) = m0.take() {
            m_cat(&mut m, Some(o));
        }
        m.m_pkthdr().csum_flags.set(M_ICMP_CSUM_OUT);

        // send the packet to outside...
        let _ = ip6_output(m, None, None, 0, None, None, net);

        icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, ND_REDIRECT);
    }
    // fail: (m0 and m freed with their boxes)
    if_put(Some(ifp));
}

/// `icmp6_ratelimit(dst, type, code)`: perform rate limit check. `false` if it is okay to
/// send the icmp6 packet, `true` if the router SHOULD NOT send this icmp6 packet due to rate
/// limitation. XXX per-destination/type check necessary? `dst`, `type` and `code` are not
/// used at this moment.
pub fn icmp6_ratelimit(_dst: &In6Addr, _type: u8, _code: u8) -> bool {
    let icmp6errppslim_local = ICMP6ERRPPSLIM.load(Ordering::Relaxed);
    // PPS limit
    let ok = ppsratecheck_locked(|l| {
        let (mut last, mut count) = {
            let st = ICMP6ERRPPS.get(l.witness());
            (st.last, st.count)
        };
        let ok = ppsratecheck_held(l, &mut last, &mut count, icmp6errppslim_local);
        let st = ICMP6ERRPPS.get_mut(l.witness_mut());
        st.last = last;
        st.count = count;
        ok
    });
    // The packet is subject to rate limit; okay to send.
    !ok
}

/// `icmp6_mtudisc_clone(dst, rtableid, ipsec)`: a host route to `dst` in `rtableid` that can
/// carry a path MTU (with a timer of `icmp6_mtudisc_timeout_q`), cloned when the route found
/// is not one; `None` when no usable route can hold one. `ipsec`: for an IPsec SA's
/// destination, which may use a reject route.
pub fn icmp6_mtudisc_clone(
    dst: &SockaddrIn6,
    rtableid: u32,
    ipsec: bool,
    net: &NetHeld<'_>,
) -> Option<PoolArc<Rtentry>> {
    let sin6 = sin6_bytes(dst);
    let mut rt = rtalloc(view(&sin6), RT_RESOLVE, rtableid, net)?;

    // Check if the route is actually usable
    if !rtisvalid(Some(&rt)) {
        return None;
    }
    // IPsec needs the route only for PMTU, it can use reject for that
    if !ipsec && rt.rt_flags.get() & (RTF_REJECT | RTF_BLACKHOLE) != 0 {
        return None;
    }

    // No PMTU for local routes and permanent neighbors, ARP and NDP use the same expire timer
    // as the route.
    if rt.rt_flags.get() & RTF_LOCAL != 0
        || (rt.rt_flags.get() & RTF_LLINFO != 0 && rt.rt_expire().get() == 0)
    {
        return None;
    }

    // If we didn't get a host route, allocate one
    if rt.rt_flags.get() & RTF_HOST == 0 {
        let mut info = RtAddrinfo::new();
        let mut sa_rl = SockaddrRtlabel::default();
        let gw = rt.gateway_copy();

        info.rti_ifa = rt.rt_ifa.get();
        info.rti_flags = RTF_GATEWAY | RTF_HOST | RTF_DYNAMIC;
        info.rti_info[RTAX_DST] = Some(view(&sin6));
        info.rti_info[RTAX_GATEWAY] = gw.as_ref().map(|g| g.view());
        info.rti_info[RTAX_LABEL] = rtlabel_id2sa(rt.rt_labelid.get(), &mut sa_rl);

        let mut nrt = None;
        rtrequest(
            RTM_ADD,
            &mut info,
            rt.rt_priority.get(),
            Some(&mut nrt),
            rtableid,
            net,
        )
        .ok()?;
        let n = nrt?;
        n.rt_rmx.assign(&rt.rt_rmx);
        rt = n;
        rtm_send(&rt, RTM_ADD, 0, rtableid, net);
    }
    rt_timer_add(&rt, &ICMP6_MTUDISC_TIMEOUT_Q, rtableid).ok()?;

    Some(rt)
}

/// `icmp6_mtudisc_timeout(rt, rtableid)`: a learned path MTU expired (`icmp6_mtudisc_timeout_q`'s
/// function): a cloned host route deleted, else the route's MTU forgotten. Under the net lock
/// (`net`, the timer's).
pub fn icmp6_mtudisc_timeout(rt: &PoolArc<Rtentry>, rtableid: u32, net: &NetHeld<'_>) {
    let Some(ifp) = if_get(rt.rt_ifidx.get()) else {
        return;
    };

    if rt.rt_flags.get() & (RTF_DYNAMIC | RTF_HOST) == (RTF_DYNAMIC | RTF_HOST) {
        let _ = rtdeletemsg(rt, ifp, rtableid, net);
    } else if rt.rt_locks().get() & RTV_MTU == 0 {
        rt.rt_mtu().store(0, Ordering::Relaxed);
    }

    if_put(Some(ifp));
}

/// `icmp6_sysctl_icmp6stat(oldp, oldlenp, newp)`: `net.inet6.icmp6.stats`, the counters as a
/// `struct icmp6stat` (summed straight into the heap image, the C's `malloc(M_TEMP)` and
/// `counters_read`).
fn icmp6_sysctl_icmp6stat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let mut bytes = alloc::vec![0u8; ICP6S_NCOUNTERS * size_of::<u64>()];
    ICMP6COUNTERS.read_into(&mut bytes);
    rdstruct(req, &bytes)
}

/// `icmp6_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the `net.inet6.icmp6` sysctls.
pub fn icmp6_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // All sysctl names at this level are terminal.
    let [n] = name else {
        return Err(Errno::ENOTDIR);
    };

    match *n {
        ICMPV6CTL_REDIRTIMEOUT => {
            let oldval = ICMP6_REDIRTIMEOUT.load(Ordering::Relaxed);
            let newval = core::sync::atomic::AtomicI32::new(oldval);
            let error = int_bounded(req, &newval, 0, INT_MAX);
            let newval = newval.into_inner();
            if error.is_ok() && oldval != newval {
                let sysctl_lock = SYSCTL_LOCK.write();
                ICMP6_REDIRTIMEOUT.store(newval, Ordering::Relaxed);
                rt_timer_queue_change(&ICMP6_REDIRECT_TIMEOUT_Q, newval);
                drop(sysctl_lock);
            }
            error
        }
        ICMPV6CTL_STATS => icmp6_sysctl_icmp6stat(req),
        ICMPV6CTL_ND6_QUEUED => rdint(req, ln_hold_total.load(Ordering::Relaxed) as i32),
        _ => bounded_arr(&ICMPV6CTL_VARS, name, req),
    }
}

/// The rate limiter's state, the path MTU callbacks and the two timer queues' timeouts
/// forgotten for the host tests (each test starts with the limit's window open and no
/// callback; the registered array lives in an earlier test's memory: forgotten, not freed).
#[cfg(any(test, feature = "doubles"))]
pub fn icmp6_test_reset() {
    ppsratecheck_locked(|l| {
        let st = ICMP6ERRPPS.get_mut(l.witness_mut());
        st.last = Timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        st.count = 0;
    });
    core::mem::forget(ICMP6_MTUDISC_CALLBACKS.replace(None));
}

/// The ICMPv6 errors `icmp6_error` was asked for in the host tests: their type, code and
/// parameter, in order (what the C's callers ask for, whether or not one is sent: the rate
/// limit, the routes and the reflection decide that).
#[cfg(test)]
pub(crate) mod seen {
    use std::sync::Mutex;
    use std::vec::Vec;

    static SEEN: Mutex<Vec<(u8, u8, i32)>> = Mutex::new(Vec::new());

    /// One error asked for.
    pub(crate) fn record(type_: u8, code: u8, param: i32) {
        SEEN.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((type_, code, param));
    }

    /// The errors asked for since the last call.
    pub(crate) fn take() -> Vec<(u8, u8, i32)> {
        core::mem::take(&mut *SEEN.lock().unwrap_or_else(|e| e.into_inner()))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of ICMPv6 over a test Ethernet interface with fd00:77::1/64 and its
    // link-local address (ND's fixture, every DAD run to its end): LZ's (the echo reply with
    // its copy and fresh-header paths, the input's refusals, icmp6_error's rules and the message
    // it builds, the rate limit, the truncation to the minimum MTU, the quoted scope zone,
    // icmp6_reflect's refusals and its stripping of extension headers, the redirect validation,
    // a host's silence about redirects, path MTU discovery, the sysctls) and one crafted message
    // per bound of icmp6_input (the per-type lengths and codes, each passing the earlier
    // checks). The ICMP6_FILTER option is bsd's (netinet6/icmp6.rs) with its word.

    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use hal::netlock::NETLOCK;
    use hal::sys::netinet_in::IPPROTO_UDP;
    use hal::sys::socket::AF_INET6;
    use hal::user::UserPtr;

    use super::*;
    use crate::net::if_var::Ifnet;
    use crate::net::mbuf::{m_copydata, m_devget, m_split, ml_dequeue};
    use crate::netinet6::ip6_input::{ip6_send_test_take, ip6_test_reset};
    use crate::netinet6::nd6::tests::{ether, ours6, peer6, setup};
    use crate::netinet6::nd6_nbr::{dad_counts, dad_fire, dad_reap};
    use crate::testutil::USER_BASE;

    type G = crate::fs::testfs::TestGlue;

    /// The user-copy window of these tests' sysctls (free at N6.5d).
    const USYS: usize = USER_BASE + 0x3_8000;

    fn icp6(c: Icmp6statCounters) -> u64 {
        ICMP6COUNTERS.read()[c as usize]
    }

    fn hist(h: Icmp6statCounters, t: u8) -> u64 {
        ICMP6COUNTERS.read()[h as usize + usize::from(t)]
    }

    /// ND's fixture with an interface whose addresses are all preferred (every DAD run), and
    /// ICMPv6's state fresh.
    fn net6() -> (MutexGuard<'static, ()>, &'static Ifnet) {
        let g = setup();
        icmp6_init();
        icmp6_test_reset();
        ip6_test_reset();
        let ifp = ether(b"tic0");
        ifp.if_mtu.set(1500);
        let w = NETLOCK.read();
        let ifas: Vec<&'static crate::net::if_var::Ifaddr> = ifp.if_addrlist.iter().collect();
        drop(w);
        for ifa in ifas {
            while dad_counts(ifa).is_some() {
                dad_fire(ifa);
            }
        }
        dad_reap();
        let _ = crate::netinet6::nd6::tests::sent(ifp);
        let _ = seen::take();
        (g, ifp)
    }

    /// An IPv6 packet from `src` to `dst` (next header `nxt`, hop limit `hlim`) whose payload
    /// is `rest`.
    fn packet(src: In6Addr, dst: In6Addr, nxt: u8, hlim: u8, rest: &[u8]) -> Vec<u8> {
        let mut ip6 = Ip6Hdr {
            ip6_plen: (rest.len() as u16).to_be(),
            ip6_nxt: nxt,
            ip6_hlim: hlim,
            ip6_src: src,
            ip6_dst: dst,
            ..Ip6Hdr::default()
        };
        ip6.set_ip6_vfc(IPV6_VERSION);
        let mut b = vec![0u8; IP6_HDR_LEN];
        assert!(pod::put(&mut b, 0, &ip6));
        b.extend_from_slice(rest);
        b
    }

    /// An ICMPv6 message `msg` (its checksum computed unless `bad`) from `src` to `dst`,
    /// received on `ifp`.
    fn icmp(ifp: &Ifnet, src: In6Addr, dst: In6Addr, msg: &[u8], bad: bool) -> MbufBox {
        let mut b = packet(src, dst, IPPROTO_ICMPV6 as u8, 255, msg);
        if b.len() >= IP6_HDR_LEN + 4 {
            let m = m_devget(&b, 0).expect("a packet");
            let sum = in6_cksum(&m, IPPROTO_ICMPV6 as u8, 40, msg.len() as u32);
            b[42..44].copy_from_slice(&sum.to_ne_bytes());
            if bad {
                b[42] ^= 0xff;
            }
        }
        let m = m_devget(&b, 0).expect("a packet");
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        m
    }

    /// `icmp6_input` of `m` under the exclusive net lock, as `ip6intr` runs it.
    fn input(m: MbufBox) -> i32 {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut mp = Some(m);
        let mut off = 40;
        icmp6_input::<G>(
            &mut mp,
            &mut off,
            IPPROTO_ICMPV6,
            i32::from(AF_INET6),
            None,
            &net,
        )
    }

    /// `f` under the exclusive net lock.
    fn with_net<R>(f: impl FnOnce(&NetHeld<'_>) -> R) -> R {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        f(&net)
    }

    /// What `ip6_send` queued since the last call: each packet's bytes.
    fn sent() -> Vec<Vec<u8>> {
        let mut ml = ip6_send_test_take();
        let mut v = Vec::new();
        while let Some(m) = ml_dequeue(&mut ml) {
            let mut b = vec![0u8; m.m_pkthdr().len.get() as usize];
            m_copydata(&m, 0, &mut b);
            v.push(b);
        }
        v
    }

    fn addr(b: &[u8], at: usize) -> In6Addr {
        pod::at::<In6Addr>(b, at).expect("an address")
    }

    /// An echo request (id 7, seq 1) with `data`.
    fn echo(data: &[u8]) -> Vec<u8> {
        let mut m = vec![ICMP6_ECHO_REQUEST, 0, 0, 0, 0, 7, 0, 1];
        m.extend_from_slice(data);
        m
    }

    #[test]
    fn echo_request_is_answered_with_an_echo_reply() {
        let (_g, ifp) = net6();
        let (reflect, out, inh) = (
            icp6(Icmp6statCounters::Icp6sReflect),
            hist(Icmp6statCounters::Icp6sOuthist, ICMP6_ECHO_REPLY),
            hist(Icmp6statCounters::Icp6sInhist, ICMP6_ECHO_REQUEST),
        );
        let _ = input(icmp(ifp, peer6(), ours6(), &echo(b"ping6 data"), false));
        let s = sent();
        assert_eq!(s.len(), 1, "one reply");
        let r = &s[0];
        assert_eq!(
            (addr(r, 8), addr(r, 24)),
            (ours6(), peer6()),
            "from us to the sender"
        );
        assert_eq!(
            (r[6], r[7]),
            (IPPROTO_ICMPV6 as u8, 64),
            "ICMPv6, the default hop limit"
        );
        assert_eq!((r[40], r[41]), (ICMP6_ECHO_REPLY, 0));
        assert_eq!(&r[44..], &echo(b"ping6 data")[4..], "id, seq and data kept");
        assert_eq!(icp6(Icmp6statCounters::Icp6sReflect), reflect + 1);
        assert_eq!(
            hist(Icmp6statCounters::Icp6sOuthist, ICMP6_ECHO_REPLY),
            out + 1
        );
        assert_eq!(
            hist(Icmp6statCounters::Icp6sInhist, ICMP6_ECHO_REQUEST),
            inh + 1
        );
    }

    #[test]
    fn echo_reply_of_a_split_packet_gets_its_own_header_mbuf() {
        let (_g, ifp) = net6();
        // The first mbuf holds the IPv6 header and half of the ICMPv6 one.
        let mut m = icmp(ifp, peer6(), ours6(), &echo(&[0x5a; 64]), false);
        let tail = m_split(&mut m, 44, M_DONTWAIT).expect("a split");
        drop(m.m_next.set(tail));
        crate::net::mbuf::m_calchdrlen(&m);
        let _ = input(m);
        let s = sent();
        assert_eq!(s.len(), 1);
        assert_eq!((s[0][40], s[0][41]), (ICMP6_ECHO_REPLY, 0));
        assert_eq!(&s[0][44..], &echo(&[0x5a; 64])[4..]);
    }

    #[test]
    fn echo_request_to_a_stranger_is_not_answered_without_a_route() {
        let (_g, ifp) = net6();
        // To all nodes from a sender beyond any route: the reply's source is chosen by the
        // route back (none).
        let mut far = ours6();
        far.s6_addr[0] = 0x20;
        far.s6_addr[1] = 0x01;
        let mut all = In6Addr::default();
        all.s6_addr[0] = 0xff;
        all.s6_addr[1] = 0x02;
        all.s6_addr[15] = 1;
        let _ = input(icmp(ifp, far, all, &echo(b"x"), false));
        assert!(sent().is_empty(), "no route back, no source");
        // From the neighbor: answered.
        let _ = input(icmp(ifp, peer6(), all, &echo(b"x"), false));
        assert_eq!(sent().len(), 1);
    }

    /// One message per check of `icmp6_input`, each passing the earlier ones: the counter
    /// that moves.
    #[test]
    fn each_check_of_the_input_answers_as_the_c() {
        let (_g, ifp) = net6();
        use Icmp6statCounters as C;
        let msg = |t: u8, c: u8, len: usize| {
            let mut m = vec![0u8; len];
            m[0] = t;
            if len > 1 {
                m[1] = c;
            }
            m
        };
        #[rustfmt::skip]
        let cases: [(Vec<u8>, bool, C); 22] = [
            (vec![ICMP6_ECHO_REQUEST, 0, 0, 0], false, C::Icp6sTooshort),
            (echo(b""), true, C::Icp6sChecksum),
            (msg(ICMP6_ECHO_REQUEST, 1, 8), false, C::Icp6sBadcode),
            (msg(ICMP6_ECHO_REPLY, 1, 8), false, C::Icp6sBadcode),
            (msg(ICMP6_DST_UNREACH, 7, 48), false, C::Icp6sBadcode),
            (msg(ICMP6_TIME_EXCEEDED, 2, 48), false, C::Icp6sBadcode),
            (msg(ICMP6_PARAM_PROB, 3, 48), false, C::Icp6sBadcode),
            (msg(MLD_LISTENER_QUERY, 0, 23), false, C::Icp6sBadlen),
            (msg(MLD_LISTENER_REPORT, 0, 23), false, C::Icp6sBadlen),
            (msg(MLD_LISTENER_DONE, 0, 23), false, C::Icp6sBadlen),
            (msg(ND_ROUTER_SOLICIT, 1, 8), false, C::Icp6sBadcode),
            (msg(ND_ROUTER_ADVERT, 0, 15), false, C::Icp6sBadlen),
            (msg(ND_ROUTER_ADVERT, 1, 16), false, C::Icp6sBadcode),
            (msg(ND_NEIGHBOR_SOLICIT, 0, 23), false, C::Icp6sBadlen),
            (msg(ND_NEIGHBOR_SOLICIT, 1, 24), false, C::Icp6sBadcode),
            (msg(ND_NEIGHBOR_ADVERT, 0, 23), false, C::Icp6sBadlen),
            (msg(ND_NEIGHBOR_ADVERT, 1, 24), false, C::Icp6sBadcode),
            (msg(ND_REDIRECT, 0, 39), false, C::Icp6sBadlen),
            (msg(ND_REDIRECT, 1, 40), false, C::Icp6sBadcode),
            (msg(ICMP6_ROUTER_RENUMBERING, 2, 16), false, C::Icp6sBadcode),
            (msg(ICMP6_ROUTER_RENUMBERING, 0, 15), false, C::Icp6sBadlen),
            // An error too short for the header it quotes (icmp6_notify_error's check).
            (msg(ICMP6_DST_UNREACH, 0, 47), false, C::Icp6sTooshort),
        ];
        for (m, bad, c) in cases {
            let before = icp6(c);
            assert_eq!(input(icmp(ifp, peer6(), ours6(), &m, bad)), IPPROTO_DONE);
            assert_eq!(icp6(c), before + 1, "{m:?} {c:?}");
        }
        // Each length at its bound passes that check.
        for (t, len) in [
            (MLD_LISTENER_DONE, 24),
            (ND_ROUTER_ADVERT, 16),
            (ICMP6_ROUTER_RENUMBERING, 16),
        ] {
            let before = icp6(C::Icp6sBadlen);
            let _ = input(icmp(ifp, peer6(), ours6(), &msg(t, 0, len), false));
            assert_eq!(icp6(C::Icp6sBadlen), before, "type {t} at {len}");
        }
    }

    /// A UDP packet from the neighbor to us, `len` bytes of payload.
    fn udp(len: usize) -> MbufBox {
        let b = packet(peer6(), ours6(), IPPROTO_UDP as u8, 64, &vec![0x33; len]);
        m_devget(&b, 0).expect("a packet")
    }

    fn error(m: MbufBox, t: u8, c: u8, p: i32) {
        with_net(|net| icmp6_error(m, t, c, p, net));
    }

    #[test]
    fn error_message_quotes_the_packet_and_counts_its_kind() {
        let (_g, _ifp) = net6();
        use Icmp6statCounters as C;
        let (e, k, o) = (
            icp6(C::Icp6sError),
            icp6(C::Icp6sOparamprobHeader),
            hist(C::Icp6sOuthist, ICMP6_PARAM_PROB),
        );
        error(udp(8), ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, 42);
        let s = sent();
        assert_eq!(s.len(), 1);
        let r = &s[0];
        assert_eq!((addr(r, 8), addr(r, 24)), (ours6(), peer6()));
        assert_eq!((r[40], r[41]), (ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER));
        assert_eq!(&r[44..48], &42u32.to_be_bytes(), "the pointer");
        assert_eq!(r.len(), 48 + 48, "the packet quoted whole");
        assert_eq!(addr(r, 48 + 8), peer6(), "the quoted header");
        assert_eq!(icp6(C::Icp6sError), e + 1);
        assert_eq!(icp6(C::Icp6sOparamprobHeader), k + 1);
        assert_eq!(hist(C::Icp6sOuthist, ICMP6_PARAM_PROB), o + 1);
        // A Packet Too Big carries the MTU there.
        error(udp(8), ICMP6_PACKET_TOO_BIG, 0, 1280);
        let s = sent();
        assert_eq!(&s[0][44..48], &1280u32.to_be_bytes(), "the MTU");
        // Each kind counted.
        for (t, c, k) in [
            (
                ICMP6_DST_UNREACH,
                ICMP6_DST_UNREACH_NOPORT,
                C::Icp6sOdstUnreachNoport,
            ),
            (
                ICMP6_DST_UNREACH,
                ICMP6_DST_UNREACH_ADDR,
                C::Icp6sOdstUnreachAddr,
            ),
            (
                ICMP6_TIME_EXCEEDED,
                ICMP6_TIME_EXCEED_TRANSIT,
                C::Icp6sOtimeExceedTransit,
            ),
            (
                ICMP6_TIME_EXCEEDED,
                ICMP6_TIME_EXCEED_REASSEMBLY,
                C::Icp6sOtimeExceedReassembly,
            ),
            (
                ICMP6_PARAM_PROB,
                ICMP6_PARAMPROB_OPTION,
                C::Icp6sOparamprobOption,
            ),
            (ICMP6_DST_UNREACH, 99, C::Icp6sOunknown),
        ] {
            let before = icp6(k);
            icmp6_errcount(t, c);
            assert_eq!(icp6(k), before + 1, "{t} {c}");
        }
    }

    #[test]
    fn no_error_about_an_error() {
        let (_g, _ifp) = net6();
        let cant = icp6(Icmp6statCounters::Icp6sCanterror);
        let mk = |t: u8| {
            let b = packet(
                peer6(),
                ours6(),
                IPPROTO_ICMPV6 as u8,
                64,
                &[t, 0, 0, 0, 0, 0, 0, 0],
            );
            m_devget(&b, 0).expect("a packet")
        };
        // About an ICMPv6 error, or a redirect: none.
        for t in [ICMP6_DST_UNREACH, ICMP6_PARAM_PROB, ND_REDIRECT] {
            error(mk(t), ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, 0);
        }
        assert_eq!(icp6(Icmp6statCounters::Icp6sCanterror), cant + 3);
        assert!(sent().is_empty());
        // About an informational message: sent.
        error(
            mk(ICMP6_ECHO_REQUEST),
            ICMP6_PARAM_PROB,
            ICMP6_PARAMPROB_HEADER,
            0,
        );
        assert_eq!(sent().len(), 1);
    }

    #[test]
    fn errors_about_multicast_and_unspecified_sources_are_suppressed() {
        let (_g, _ifp) = net6();
        let mut all = In6Addr::default();
        all.s6_addr[0] = 0xff;
        all.s6_addr[1] = 0x02;
        all.s6_addr[15] = 1;
        let to = |src: In6Addr, dst: In6Addr| {
            let b = packet(src, dst, IPPROTO_UDP as u8, 64, &[0; 8]);
            m_devget(&b, 0).expect("a packet")
        };
        // To a multicast group: none, but for the two exceptions.
        error(
            to(peer6(), all),
            ICMP6_PARAM_PROB,
            ICMP6_PARAMPROB_HEADER,
            0,
        );
        assert!(sent().is_empty());
        error(to(peer6(), all), ICMP6_PACKET_TOO_BIG, 0, 1280);
        assert_eq!(sent().len(), 1, "path MTU discovery");
        error(
            to(peer6(), all),
            ICMP6_PARAM_PROB,
            ICMP6_PARAMPROB_OPTION,
            0,
        );
        assert_eq!(sent().len(), 1, "an option's error");
        // Sent as a link-layer multicast: the same.
        let m = to(peer6(), ours6());
        m.m_flags().set(m.flags() | M_MCAST);
        error(m, ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT, 0);
        assert!(sent().is_empty());
        // From the unspecified or a multicast address: none.
        error(to(In6Addr::default(), ours6()), ICMP6_DST_UNREACH, 0, 0);
        error(to(all, ours6()), ICMP6_DST_UNREACH, 0, 0);
        assert!(sent().is_empty());
    }

    #[test]
    fn errors_are_rate_limited() {
        let (_g, _ifp) = net6();
        let lim = ICMP6ERRPPSLIM.load(Ordering::Relaxed);
        ICMP6ERRPPSLIM.store(1, Ordering::Relaxed);
        icmp6_test_reset();
        let toofreq = icp6(Icmp6statCounters::Icp6sToofreq);
        error(udp(8), ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT, 0);
        error(udp(8), ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT, 0);
        ICMP6ERRPPSLIM.store(lim, Ordering::Relaxed);
        assert_eq!(icp6(Icmp6statCounters::Icp6sToofreq), toofreq + 1);
        assert_eq!(sent().len(), 1);
    }

    #[test]
    fn a_long_packet_is_truncated_to_the_minimum_mtu() {
        let (_g, _ifp) = net6();
        error(udp(2000), ICMP6_PACKET_TOO_BIG, 0, 1280);
        let s = sent();
        assert_eq!(
            s[0].len(),
            IPV6_MMTU as usize,
            "IPv6 + ICMPv6 + ICMPV6_PLD_MAXLEN"
        );
        // Exactly the payload's room: kept whole.
        error(udp(ICMPV6_PLD_MAXLEN - 41), ICMP6_PACKET_TOO_BIG, 0, 1280);
        assert_eq!(sent()[0].len(), IPV6_MMTU as usize - 1);
    }

    #[test]
    fn error_message_does_not_quote_the_scope_zone() {
        let (_g, ifp) = net6();
        let mut ll = peer6();
        ll.s6_addr[..8].copy_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0]);
        let mut zoned = ll;
        zoned.set_s6_addr16(1, (ifp.if_index.get() as u16).to_be());
        let mut ours_ll = ours6();
        ours_ll.s6_addr[..8].copy_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0]);
        let b = packet(zoned, ours6(), IPPROTO_UDP as u8, 64, &[0; 8]);
        let m = m_devget(&b, 0).expect("a packet");
        error(m, ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT, 0);
        let s = sent();
        assert_eq!(
            addr(&s[0], 48 + 8),
            ll,
            "the quoted source without its zone"
        );
    }

    #[test]
    fn reflect_refuses_what_cannot_be_reflected() {
        let (_g, _ifp) = net6();
        with_net(|net| {
            let mut mp = Some(udp(8));
            assert_eq!(
                icmp6_reflect(&mut mp, 20, None, net),
                Err(Errno::EHOSTUNREACH)
            );
            let m = udp(8);
            m.m_pkthdr().ph_loopcnt.set(M_MAXLOOP);
            let mut mp = Some(m);
            assert_eq!(
                icmp6_reflect(&mut mp, IP6_HDR_LEN, None, net),
                Err(Errno::ELOOP)
            );
            assert!(mp.is_none(), "consumed");
        });
    }

    #[test]
    fn reflect_strips_extension_headers_and_picks_the_source() {
        let (_g, _ifp) = net6();
        // A request behind an 8-byte destination options header.
        let mut rest = vec![IPPROTO_ICMPV6 as u8, 0, 1, 4, 0, 0, 0, 0];
        rest.extend_from_slice(&[ICMP6_ECHO_REPLY, 0, 0x12, 0x34, 0, 1, 0, 2]);
        let b = packet(peer6(), ours6(), 60, 64, &rest);
        let m = m_devget(&b, 0).expect("a packet");
        with_net(|net| {
            let mut mp = Some(m);
            icmp6_reflect(&mut mp, 48, None, net).expect("reflected");
            let m = mp.expect("the reply");
            let mut r = vec![0u8; m.m_pkthdr().len.get() as usize];
            m_copydata(&m, 0, &mut r);
            assert_eq!(r.len(), 48, "the extension header gone");
            assert_eq!(r[6], IPPROTO_ICMPV6 as u8);
            assert_eq!((addr(&r, 8), addr(&r, 24)), (ours6(), peer6()));
            assert_eq!(&r[42..44], &[0, 0], "the checksum to compute");
            assert_ne!(m.m_pkthdr().csum_flags.get() & M_ICMP_CSUM_OUT, 0);
        });
    }

    /// A redirect from `src` (hop limit `hlim`) for `dst` to `tgt`, received on `ifp`.
    fn redirect(ifp: &Ifnet, src: In6Addr, hlim: u8, tgt: In6Addr, dst: In6Addr) -> MbufBox {
        let mut msg = vec![ND_REDIRECT, 0, 0, 0, 0, 0, 0, 0];
        msg.extend_from_slice(&tgt.s6_addr);
        msg.extend_from_slice(&dst.s6_addr);
        let mut b = packet(src, ours6(), IPPROTO_ICMPV6 as u8, hlim, &msg);
        b[7] = hlim;
        let m = m_devget(&b, 0).expect("a packet");
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        m
    }

    /// A host route to `dst` through the gateway `gw` on `ifp`.
    fn gw_route(ifp: &Ifnet, dst: In6Addr, gw: In6Addr) {
        let d = sin6_bytes(&sin6_of(dst));
        let g = sin6_bytes(&sin6_of(gw));
        let ifa = crate::netinet6::in6::in6ifa_ifpwithaddr(ifp, &ours6()).map(|ia| &ia.ia_ifa);
        with_net(|net| {
            let mut info = RtAddrinfo::new();
            info.rti_flags = RTF_GATEWAY | RTF_HOST | hal::sys::net_route::RTF_STATIC;
            info.rti_info[RTAX_DST] = Some(view(&d));
            info.rti_info[RTAX_GATEWAY] = Some(view(&g));
            info.rti_ifa = ifa;
            rtrequest(
                RTM_ADD,
                &mut info,
                hal::sys::net_route::RTP_STATIC,
                None,
                0,
                net,
            )
            .expect("the gateway route");
        });
    }

    #[test]
    fn redirects_are_validated() {
        let (_g, ifp) = net6();
        ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_AUTOCONF6);
        let bad = || icp6(Icmp6statCounters::Icp6sBadredirect);
        let zone = (ifp.if_index.get() as u16).to_be();
        // The neighbor's link-local address, with the zone ip6_input embeds.
        let mut ll = peer6();
        ll.s6_addr[..8].copy_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0]);
        ll.set_s6_addr16(1, zone);
        let far = |last: u8| {
            let mut a = In6Addr::default();
            a.s6_addr[..2].copy_from_slice(&[0x20, 0x01]);
            a.s6_addr[15] = last;
            a
        };
        let mut mc = In6Addr::default();
        mc.s6_addr[0] = 0xff;
        mc.s6_addr[1] = 0x0e;
        mc.s6_addr[15] = 1;
        // Each destination's route goes through the redirect's source, so a case fails only
        // by its own check.
        gw_route(ifp, far(1), peer6());
        gw_route(ifp, far(2), ll);
        gw_route(ifp, mc, ll);
        gw_route(ifp, far(3), ours6());
        for (what, m) in [
            ("a global source", redirect(ifp, peer6(), 255, ll, far(1))),
            ("a hop limit not 255", redirect(ifp, ll, 254, ll, far(2))),
            ("a multicast destination", redirect(ifp, ll, 255, ll, mc)),
            (
                "a gateway that is not the source",
                redirect(ifp, ll, 255, ll, far(3)),
            ),
            (
                "no route to the destination",
                redirect(ifp, ll, 255, ll, far(4)),
            ),
            (
                "a global target to another destination",
                redirect(ifp, ll, 255, ours6(), far(2)),
            ),
        ] {
            let b = bad();
            with_net(|net| icmp6_redirect_input::<G>(m, 40, net));
            assert_eq!(bad(), b + 1, "{what}");
        }
        // A valid one (a better router): no bad redirect.
        let b = bad();
        with_net(|net| icmp6_redirect_input::<G>(redirect(ifp, ll, 255, ll, far(2)), 40, net));
        assert_eq!(bad(), b, "a valid redirect");
        // As a router, or without autoconfiguration: ignored, uncounted, valid or not.
        IP6_FORWARDING.store(1, Ordering::Relaxed);
        with_net(|net| icmp6_redirect_input::<G>(redirect(ifp, peer6(), 1, ll, mc), 40, net));
        IP6_FORWARDING.store(0, Ordering::Relaxed);
        ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_AUTOCONF6);
        with_net(|net| icmp6_redirect_input::<G>(redirect(ifp, peer6(), 1, ll, mc), 40, net));
        assert_eq!(bad(), b);
    }

    #[test]
    fn a_host_sends_no_redirects() {
        let (_g, ifp) = net6();
        let o = icp6(Icmp6statCounters::Icp6sOredirect);
        let out = hist(Icmp6statCounters::Icp6sOuthist, ND_REDIRECT);
        let b = sin6_bytes(&sin6_of(peer6()));
        with_net(|net| {
            let rt = rtalloc(view(&b), RT_RESOLVE, 0, net).expect("the neighbor's route");
            icmp6_redirect_output(udp(8), &rt, net);
        });
        assert_eq!(
            icp6(Icmp6statCounters::Icp6sOredirect),
            o + 1,
            "counted first, as the C"
        );
        assert_eq!(
            hist(Icmp6statCounters::Icp6sOuthist, ND_REDIRECT),
            out,
            "none sent"
        );
        let _ = ifp;
    }

    /// A Packet Too Big parameter about `dst` with `mtu`, the message received on `ifp`.
    fn ptb(ifp: &Ifnet, dst: In6Addr, mtu: u32, f: impl FnOnce(&Ip6CtlParam<'_>)) {
        let m = udp(8);
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        let mut icmp6 = Icmp6Hdr::zeroed();
        icmp6.icmp6_type = ICMP6_PACKET_TOO_BIG;
        icmp6.set_icmp6_data32(mtu.to_be());
        let p = Ip6CtlParam {
            m: &m,
            icmp6,
            ip6: Ip6Hdr::default(),
            off: 48,
            src: sin6_of(ours6()),
            finaldst: Cell::new(dst),
            cmdarg: Some(mtu),
            nxt: IPPROTO_UDP as u8,
        };
        f(&p);
    }

    #[test]
    fn path_mtu_discovery_clones_a_host_route_and_lowers_its_mtu() {
        let (_g, ifp) = net6();
        static CALLED: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        fn cb(_: &SockaddrIn6, _: u32) {
            CALLED.fetch_add(1, Ordering::Relaxed);
        }
        icmp6_mtudisc_callback_register(cb);
        icmp6_mtudisc_callback_register(cb);
        let mut far = ours6();
        far.s6_addr[15] = 0x99;
        let chg = icp6(Icmp6statCounters::Icp6sPmtuchg);
        let before = CALLED.load(Ordering::Relaxed);
        ptb(ifp, far, 1400, |p| {
            with_net(|net| icmp6_mtudisc_update(p, true, net))
        });
        assert_eq!(icp6(Icmp6statCounters::Icp6sPmtuchg), chg + 1);
        assert_eq!(
            CALLED.load(Ordering::Relaxed),
            before + 1,
            "registered once, told once"
        );
        let b = sin6_bytes(&sin6_of(far));
        with_net(|net| {
            let rt = rtalloc(view(&b), 0, 0, net).expect("the cloned host route");
            assert_ne!(rt.rt_flags.get() & RTF_HOST, 0);
            assert_eq!(rt.rt_mtu().load(Ordering::Relaxed), 1400);
        });
        // Below the IPv6 minimum MTU, not below the route's, or (a fresh route) not below the
        // interface's: nothing changes.
        ptb(ifp, far, 1279, |p| {
            with_net(|net| icmp6_mtudisc_update(p, true, net))
        });
        ptb(ifp, far, 1500, |p| {
            with_net(|net| icmp6_mtudisc_update(p, true, net))
        });
        let mut fresh = far;
        fresh.s6_addr[15] = 0x9a;
        ptb(ifp, fresh, ifp.if_mtu.get(), |p| {
            with_net(|net| icmp6_mtudisc_update(p, true, net))
        });
        assert_eq!(icp6(Icmp6statCounters::Icp6sPmtuchg), chg + 1);
        let b = sin6_bytes(&sin6_of(fresh));
        with_net(|net| {
            let rt = rtalloc(view(&b), 0, 0, net).expect("the fresh host route");
            assert_eq!(
                rt.rt_mtu().load(Ordering::Relaxed),
                0,
                "the interface's MTU kept"
            );
        });
    }

    #[test]
    fn unvalidated_mtu_changes_need_room_in_the_table() {
        let (_g, ifp) = net6();
        let lowat = ICMP6_MTUDISC_LOWAT.load(Ordering::Relaxed);
        let chg = icp6(Icmp6statCounters::Icp6sPmtuchg);
        let mut far = ours6();
        far.s6_addr[15] = 0x98;
        // A validated change makes the queue's first route.
        ptb(ifp, far, 1400, |p| {
            with_net(|net| icmp6_mtudisc_update(p, true, net))
        });
        assert_eq!(icp6(Icmp6statCounters::Icp6sPmtuchg), chg + 1);
        // The low water mark under the queue's count: an unvalidated one is ignored.
        ICMP6_MTUDISC_LOWAT.store(0, Ordering::Relaxed);
        far.s6_addr[15] = 0x97;
        ptb(ifp, far, 1400, |p| {
            with_net(|net| icmp6_mtudisc_update(p, false, net))
        });
        assert_eq!(
            icp6(Icmp6statCounters::Icp6sPmtuchg),
            chg + 1,
            "over the low water mark"
        );
        ICMP6_MTUDISC_LOWAT.store(lowat, Ordering::Relaxed);
        ptb(ifp, far, 1400, |p| {
            with_net(|net| icmp6_mtudisc_update(p, false, net))
        });
        assert_eq!(icp6(Icmp6statCounters::Icp6sPmtuchg), chg + 2, "room again");
    }

    /// `icmp6_sysctl` of `name`, writing `new` when given: the value read and the answer.
    fn sysctl(name: &[i32], new: Option<i32>) -> (i32, Result<(), Errno>) {
        let mut oldlen = size_of::<i32>();
        if let Some(v) = new {
            assert!(UserPtr::<i32>::new(USYS + 0x100).copyout(&v).is_ok());
        }
        let mut req = SysctlReq {
            oldp: USYS,
            oldlenp: &mut oldlen,
            newp: if new.is_some() { USYS + 0x100 } else { 0 },
            newlen: if new.is_some() { size_of::<i32>() } else { 0 },
        };
        let r = icmp6_sysctl(name, &mut req);
        let v = UserPtr::<i32>::new(USYS).copyin().unwrap_or(-7);
        (v, r)
    }

    #[test]
    fn the_sysctls_read_and_write_the_variables() {
        let _g = crate::net::route::tests::testroutes_setup();
        // The bounded names: in bounds taken, out of bounds EINVAL.
        for (name, var, lo, hi) in [
            (ICMPV6CTL_ND6_DELAY, &ND6_DELAY, 0, INT_MAX),
            (ICMPV6CTL_ND6_UMAXTRIES, &ND6_UMAXTRIES, 0, INT_MAX),
            (ICMPV6CTL_ND6_MMAXTRIES, &ND6_MMAXTRIES, 0, INT_MAX),
            (ICMPV6CTL_MTUDISC_HIWAT, &ICMP6_MTUDISC_HIWAT, 0, INT_MAX),
            (ICMPV6CTL_MTUDISC_LOWAT, &ICMP6_MTUDISC_LOWAT, 0, INT_MAX),
            (ICMPV6CTL_ERRPPSLIMIT, &ICMP6ERRPPSLIM, -1, 1000),
        ] {
            let old = var.load(Ordering::Relaxed);
            assert_eq!(sysctl(&[name], None), (old, Ok(())));
            assert_eq!(sysctl(&[name], Some(lo)).1, Ok(()));
            assert_eq!(var.load(Ordering::Relaxed), lo);
            assert_eq!(sysctl(&[name], Some(hi)).1, Ok(()));
            assert_eq!(var.load(Ordering::Relaxed), hi);
            if lo > i32::MIN {
                assert_eq!(sysctl(&[name], Some(lo - 1)).1, Err(Errno::EINVAL));
            }
            if hi < INT_MAX {
                assert_eq!(sysctl(&[name], Some(hi + 1)).1, Err(Errno::EINVAL));
            }
            var.store(old, Ordering::Relaxed);
        }
        // The redirect routes' timeout, the queue's with it.
        let old = ICMP6_REDIRTIMEOUT.load(Ordering::Relaxed);
        assert_eq!(sysctl(&[ICMPV6CTL_REDIRTIMEOUT], Some(30)).1, Ok(()));
        assert_eq!(ICMP6_REDIRTIMEOUT.load(Ordering::Relaxed), 30);
        assert_eq!(
            sysctl(&[ICMPV6CTL_REDIRTIMEOUT], Some(-1)).1,
            Err(Errno::EINVAL)
        );
        assert_eq!(sysctl(&[ICMPV6CTL_REDIRTIMEOUT], Some(old)).1, Ok(()));
        // The queued packets of ND, read only.
        assert_eq!(sysctl(&[ICMPV6CTL_ND6_QUEUED], None).1, Ok(()));
        assert_eq!(
            sysctl(&[ICMPV6CTL_ND6_QUEUED], Some(1)).1,
            Err(Errno::EPERM)
        );
        // The statistics, a struct icmp6stat.
        let mut oldlen = ICP6S_NCOUNTERS * 8;
        let mut req = SysctlReq {
            oldp: USYS,
            oldlenp: &mut oldlen,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(icmp6_sysctl(&[ICMPV6CTL_STATS], &mut req), Ok(()));
        assert_eq!(oldlen, ICP6S_NCOUNTERS * 8);
        // Every name is terminal.
        assert_eq!(
            sysctl(&[ICMPV6CTL_ND6_DELAY, 1], None).1,
            Err(Errno::ENOTDIR)
        );
        assert_eq!(sysctl(&[], None).1, Err(Errno::ENOTDIR));
    }
}
/* </TESTS> */
