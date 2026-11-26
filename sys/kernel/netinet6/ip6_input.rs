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
 *	@(#)ip_input.c	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 input: header checks, the hop-by-hop options, local delivery through the protocol
//! switch, the hand-off to forwarding, extension header walking, the control messages of
//! received packets, the `net.inet6.ip6` sysctls and the deferred send queue:
//! `netinet6/ip6_input.c` (forwarding itself is [`crate::netinet6::ip6_forward`]).
//!
//! Upstream: sys/netinet6/ip6_input.c @ 3ce1f3f79392
//! LZ: sys/netinet6/ip6_input.rs@3c62ede68802
//!
//! [`ipv6_input`] is the entry point from the link layer (`ether_input`, `if_input_local`,
//! through [`IPV6_INPUT`]): [`ip6_input_if`] checks the header ([`ipv6_check`]), runs pf(4),
//! embeds the scope of link-local addresses, rejects type 0 routing headers
//! ([`ip6_check_rh0hdr`]), and then delivers the packet when it is for us (a multicast group
//! joined on the interface, or a local route), or forwards it. Local delivery processes the
//! hop-by-hop options and trims the packet to its payload length ([`ip6_hbhchcheck`]), then
//! walks the protocol switch with `kernel::netinet::ip_input`'s `ip_deliver`, queueing on
//! [`IP6INTRQ`] for [`ip6intr`] the protocols that need the exclusive net lock.
//!
//! Locks: the caller holds the net lock (`NetHeld`), shared on the input path, exclusive in
//! [`ip6intr`]; \[q\] [`IP6INTRQ`] and `IP6SEND_MQ` are their own mutex's (`IPL_SOFTNET`);
//! the tunables are atomics; `ip6_mtudisc_timeout`'s change is under `sysctl_lock`.
//!
//! ## Deviations
//! - `ip6_process_hopopts`, `ip6_nexthdr` and `ip6_lasthdr` return `bool` and `Option`s
//!   where the C returns `-1`; `ip6_unknown_opt` returns `true` to skip the option (the C
//!   returns its length, which its callers read again themselves). `ip6_hbhchcheck`'s
//!   `int *oursp` is an `Option<&mut bool>`.
//! - The C's `struct mbuf **` is `&mut Option<MbufBox>`: a consumed packet leaves `*mp`
//!   `None` (the C leaves a pointer its callers do not touch). [`ip6_savecontrol`] returns
//!   the control chain (the C's `*mp`, which its callers pass empty).
//! - The header the input path writes in place (the scope zones, the hop limit, reassembly's
//!   length and addresses) is written into the mbuf that holds it; [`ipv6_check`] gives a
//!   read-only first mbuf (a shared cluster, which the C writes anyway) a copy of its own,
//!   once, and a packet for which no buffer can be had there is dropped, counted as
//!   `ip6s_idropped` (the C has no such failure). The writes after it are in place.
//! - The C's `ip6_hbhchcheck` reads the hop-by-hop header's next header again after the
//!   options (a second `ip6_exthdr_get`, whose failure it counts as `ip6s_tooshort`): the
//!   kernel's keeps the byte [`ip6_hopopts_input`] read from the same contiguous region, so
//!   that second failure, which cannot happen, has no branch.
//! - `ip6_init`'s protocol switch (`ip6_protox[]` from `inet6sw[]`, IPv6's raw entry for
//!   the rest) is `kernel::netinet6::in6_proto::inet6sw_init`, the table's (N6.5d); the
//!   fragment identifiers' generator (`ip6_randomid_init`, `idgen32` over `bsd`'s crypto
//!   until N8) is seeded by `bsd`'s `domaininit` just before the domains' initialisations
//!   (the C seeds it in `ip6_init`, after the switch; nothing draws an identifier before the
//!   first output).
//! - `NPF` (pf(4)) is configured: `pf_test(AF_INET6, PF_IN, ..)` and `pf_ouraddr` are the
//!   `Pf` bridge's legs. `IPSEC` is configured: `ipsec_in_use` and `ipsec_forward_check`
//!   are the `Ipsec` bridge's (`bsd`'s until N8).
//! - Not configured, each a comment at its site: `NCARP` (`carp_lsdrop`,
//!   `carp_strict_addr_chk`) and `MROUTING` (`ip6_mforward`, `ip6_mrouter_active`,
//!   `mrt6_init`; the `mrt` sysctls answer `EOPNOTSUPP` as the C's `#else` does). The
//!   `SMALL_KERNEL` variant of `ip6_sysctl` is not built.
//!
//! ## Redesign
//! - LZ's packets parked as `&'static Mbuf`, the header read and written as copies through
//!   `bsd`'s `mtod_ip6`/`mtod_ip6_store` at each step (8 `unsafe` in `bsd`'s module, its
//!   extension header reads and control messages) -> the packet owned (`MbufBox`) and its
//!   header read once: [`ipv6_check`] answers the packet with the header it checked, which
//!   [`ip6_input_if`] passes down to [`ip6_ours`] and [`ip6_hbhchcheck`]; it is read again
//!   only after pf (which may rewrite or replace the packet) and written back
//!   (`ip6_output`'s `ip6_store`) only when a scope zone changed. **Header copies per
//!   packet**: one 40-byte read (two with pf registered), against the C's reads in place;
//!   `ip6intr` reads the next header byte alone; `ip6_forward` reads the header again (its
//!   callers include pf, which hands it none).
//! - The route cache: the softnet's (`ns_route`) or the call's own (`iproute`, given back by
//!   its drop: the C's `rtfree` at `out`), lent by `route_of`.
//! - The receive entry point is [`IPV6_INPUT`] (`IPV4_INPUT`'s shape), read by
//!   `ether_input` and `if_input_local`; `ip6intrq` is the kernel's `Niqueue`, run by the
//!   netisr with the exclusive lock's proof; `ip_deliver`'s IPv6 counters, `ip6_hdrnestlimit`
//!   and `ip6_ours_enqueue`, the routing table's `ip6_multipath` and route cache counters
//!   are the kernel's own: the `Inet` bridge's legs `ipv6_input`, `ip6intr`,
//!   `ip6_forwarding`, `ip6_multipath`, `ip6stat_rtcache`, `ip6stat_deliver`,
//!   `ip6_hdrnestlimit` and `ip6_ours_enqueue` are gone.
//! - [`ip6_savecontrol`] makes the control messages newest first, each in O(1), and
//!   reverses the chain once at the end (the C appends through a pointer to the last
//!   `m_next`): O(n) in the messages, as the C; their values are `hal::mem::pod` images.
//! - LZ's `unsafe fn ip6_process_hopopts(mp, opthead: *const u8, hbhlen, ..)` (a slice made
//!   from the caller's pointer) and `unsafe fn ip6_unknown_opt(mp, optp: *const u8, off)`
//!   -> functions over the packet: the options are named by their offset in the packet
//!   (`off`, the C's `opthead`), and each step copies the option bytes it reads
//!   (`m_copydata` of at most six, the C's reads in place) out of the chain before it acts, so
//!   no borrow of the packet outlives a step that may free it or send an ICMPv6 error;
//!   `ip6_unknown_opt` takes the option's type byte, the only byte of `optp` the C reads. The
//!   options' own length checks are the C's, made against the `hbhlen` the caller found
//!   (every read is inside those bytes); a read past the packet itself, which no caller can
//!   ask for (the output path's header is in its own mbuf, the input path's pulled up), is
//!   counted as `ip6s_toosmall` and the packet dropped.
//! - The header copies of the walkers are `pod` images of their bytes (`m_copydata` into a
//!   small array, as LZ's), the C's `m_copydata` into a stack structure.
//! - `ip6_exthdr_get` (LZ: `bsd`'s `netinet/ip6.rs`, a raw pointer into the region) answers
//!   where the region is (the chain back, the index of its mbuf and the offset there;
//!   `m_pulldown` when the first mbuf does not hold it), and [`ip6_exthdr_bytes`] lends its
//!   bytes; `nd6_nbr`'s `nd6_msg` of N6.5b is this shape.
//! - `ip6_sysctl` takes the request as `kernel::sysctl`'s `SysctlReq`; `net.inet6.ip6.stats`
//!   is summed straight into the heap image `rdstruct` copies out (the C's `malloc(M_TEMP)`
//!   and `counters_read`).
//!
//! **Lock and spl transitions** against the C: the input path takes none of its own (the
//! caller holds the net lock); queueing for [`ip6intr`] takes `ip6intrq`'s mutex once per
//! packet and [`ip6intr`] once per packet dequeued, as the C's `niq_enqueue` and
//! `niq_dequeue`; [`ip6_send`] takes the send queue's once and `ip6_send_dispatch` the shared
//! net lock once per batch, as the C.

use core::mem::{offset_of, size_of};
use core::sync::atomic::{AtomicI32, Ordering};

use hal::cpu::intr::IPL_SOFTNET;
use hal::mem::malloc::M_NOWAIT;
use hal::mem::pod;
use hal::sync::netlock::{NETLOCK, NetHeld};
use hal::sync::once::OnceStatic;
use hal::sys::errno::Errno;
use hal::sys::limits::INT_MAX;
use hal::sys::net_if::{IFF_LOOPBACK, IFT_ENC};
use hal::sys::net_route::RTF_LOCAL;
use hal::sys::netinet_icmp6::{ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, ICMP6_PARAMPROB_OPTION};
use hal::sys::netinet_in::{
    IPPROTO_AH, IPPROTO_DONE, IPPROTO_DSTOPTS, IPPROTO_FRAGMENT, IPPROTO_HOPOPTS, IPPROTO_IPV6,
    IPPROTO_ROUTING,
};
use hal::sys::netinet_ip::IPQ_MAXLEN;
use hal::sys::netinet_ip6::{
    IP6F_OFF_MASK, IP6OPT_JUMBO, IP6OPT_JUMBO_LEN, IP6OPT_MINLEN, IP6OPT_PAD1, IP6OPT_PADN,
    IP6OPT_ROUTER_ALERT, IP6OPT_RTALERT_LEN, IP6OPT_TYPE_DISCARD, IP6OPT_TYPE_FORCEICMP,
    IP6OPT_TYPE_ICMP, IP6OPT_TYPE_SKIP, IPV6_FLOWINFO_MASK, IPV6_MAXPACKET, IPV6_VERSION,
    IPV6_VERSION_MASK, Ip6Ext, Ip6Frag, Ip6Hbh, Ip6Hdr, Ip6Rthdr, ip6opt_type,
};
use hal::sys::netinet6_in6::{
    IPV6_DSTOPTS, IPV6_HOPLIMIT, IPV6_HOPOPTS, IPV6_PKTINFO, IPV6_RTHDR, IPV6_RTHDR_TYPE_0,
    IPV6_TCLASS, IPV6CTL_DAD_COUNT, IPV6CTL_DAD_PENDING, IPV6CTL_DEFHLIM, IPV6CTL_DEFMCASTHLIM,
    IPV6CTL_FORWARDING, IPV6CTL_HDRNESTLIMIT, IPV6CTL_IFQUEUE, IPV6CTL_MAXDYNROUTES,
    IPV6CTL_MAXFRAGPACKETS, IPV6CTL_MAXFRAGS, IPV6CTL_MCAST_PMTU, IPV6CTL_MFORWARDING,
    IPV6CTL_MRTMFC, IPV6CTL_MRTMIF, IPV6CTL_MRTPROTO, IPV6CTL_MRTSTATS, IPV6CTL_MTUDISCTIMEOUT,
    IPV6CTL_MULTIPATH, IPV6CTL_NEIGHBORGCTHRESH, IPV6CTL_SENDREDIRECTS, IPV6CTL_STATS, In6Pktinfo,
    in6_is_addr_loopback, in6_is_addr_mc_intfacelocal, in6_is_addr_mc_linklocal,
    in6_is_addr_multicast, in6_is_addr_unspecified, in6_is_addr_v4compat, in6_is_addr_v4mapped,
    in6_is_scope_embed,
};
use hal::sys::netinet6_in6_var::{IN6_IFF_DUPLICATED, IN6_IFF_TENTATIVE};
use hal::sys::netinet6_ip6_var::{IP6S_NCOUNTERS, Ip6stat, Ip6statCounters};
use hal::sys::socket::{AF_INET6, AF_UNSPEC, SCM_TIMESTAMP, SO_TIMESTAMP, SOL_SOCKET};
use hal::sys::sysctl::SysctlBoundedArgs;

use crate::net::if_::{niq_enqueue, sysctl_niq};
use crate::net::if_var::{Ifnet, Netstack, Niqueue, niq_dequeue};
use crate::net::ipsec_hooks::ipsec;
use crate::net::mbuf::{
    M_BCAST, M_DONTWAIT, M_EXT, M_LOOP, M_MCAST, M_PKTHDR, MLEN, MT_DATA, Mbuf, MbufBox,
    PACKET_TAG_IP6_OFFNXT, PF_TAG_PROCESSED, PF_TAG_TRANSLATE_LOCALHOST, m_adj, m_copyback,
    m_copydata, m_get, m_microtime, m_pullup, m_trailingspace, mclget,
};
use crate::net::mbuf::{MbufQueue, ml_dequeue, ml_empty, mq_delist, mq_enqueue};
use crate::net::mbuf2::{m_pulldown, m_tag_delete, m_tag_find, m_tag_get, m_tag_prepend};
use crate::net::netisr::NETISR_IPV6;
use crate::net::pf_hooks::{PF_IN, PF_PASS, pf};
use crate::net::proto::PRC_NCMDS;
use crate::net::route::{RTGENERATION, Route, rt_timer_queue_change};
use crate::net::sockbuf::sbcreatecontrol;
use crate::netinet::in_pcb::{
    IN6P_DSTOPTS, IN6P_HOPLIMIT, IN6P_HOPOPTS, IN6P_PKTINFO, IN6P_RTHDR, IN6P_TCLASS, Inpcb,
};
use crate::netinet::ip_input::ip_deliver;
use crate::netinet::ip_var::Ipoffnxt;
use crate::netinet6::frag6::frag6_init;
use crate::netinet6::icmp6::{ICMP6_MTUDISC_TIMEOUT_Q, icmp6_error};
use crate::netinet6::in6::in6_hasmulti;
use crate::netinet6::in6_proto::inet6sw_init;
use crate::netinet6::in6_var::In6Ifaddr;
use crate::netinet6::ip6_forward::ip6_forward;
use crate::netinet6::ip6_output::{ip6_load, ip6_store};
use crate::netinet6::ip6_var::{
    IP6_DAD_COUNT, IP6_DAD_PENDING, IP6_DEFHLIM, IP6_DEFMCASTHLIM, IP6_FORWARDING,
    IP6_HDRNESTLIMIT, IP6_MAXDYNROUTES, IP6_MAXFRAGPACKETS, IP6_MAXFRAGS, IP6_MCAST_PMTU,
    IP6_MFORWARDING, IP6_MTUDISC_TIMEOUT, IP6_MULTIPATH, IP6_NEIGHBORGCTHRESH, IP6_SENDREDIRECTS,
    IP6COUNTERS, IPV6_FORWARDING, IPV6_FORWARDING_IPSEC, IPV6_REDIRECT, ip6stat_inc,
    ip6stat_inc_idx,
};
use crate::netinet6::nd6::nd6_init;
use crate::proc::ProcGlue;
use crate::sysctl::{SYSCTL_LOCK, SysctlReq, bounded_arr, int_bounded, rdstruct};
use crate::task::{Task, task_add};

/// `sizeof(struct ip6_hdr)`.
const IP6_HDR_LEN: i32 = size_of::<Ip6Hdr>() as i32;

/// `inet6ctlerrmap[]`: the errno of each `PRC_*` control command (`in6_pcbnotify`'s, N6.5c;
/// `bsd`'s `ip6_input.rs` re-exports it for its protocols' `ctlinput`s).
pub static INET6CTLERRMAP: [Option<Errno>; PRC_NCMDS] = [
    None,
    None,
    None,
    None,
    None,
    Some(Errno::EMSGSIZE),
    Some(Errno::EHOSTDOWN),
    Some(Errno::EHOSTUNREACH),
    Some(Errno::EHOSTUNREACH),
    Some(Errno::EHOSTUNREACH),
    Some(Errno::ECONNREFUSED),
    Some(Errno::ECONNREFUSED),
    Some(Errno::EMSGSIZE),
    Some(Errno::EHOSTUNREACH),
    None,
    None,
    None,
    None,
    None,
    None,
    Some(Errno::ENOPROTOOPT),
];

/// `ip6intrq`: the packets for the protocols that need the exclusive net lock.
pub static IP6INTRQ: Niqueue = Niqueue::new(IPQ_MAXLEN as u32, NETISR_IPV6);

/// `ipv6_input` of the glue `ip6_init` ran for, as `if_input_proto` calls it (the receive
/// entry point `G`-erased: `ether_input` and `if_input_local` read it; `IPV4_INPUT`'s shape).
pub static IPV6_INPUT: OnceStatic<crate::net::if_::ProtoFn> = OnceStatic::new();

/// `ipv6ctl_vars[]`: the bounded integer sysctls of `net.inet6.ip6`.
static IPV6CTL_VARS: [SysctlBoundedArgs; 13] = [
    SysctlBoundedArgs::new(IPV6CTL_FORWARDING, &IP6_FORWARDING, 0, 2),
    SysctlBoundedArgs::new(IPV6CTL_SENDREDIRECTS, &IP6_SENDREDIRECTS, 0, 1),
    SysctlBoundedArgs::readonly(IPV6CTL_DAD_PENDING, &IP6_DAD_PENDING),
    // MROUTING: IPV6CTL_MRTPROTO, read only (ip6_mrtproto); not configured.
    SysctlBoundedArgs::new(IPV6CTL_DEFHLIM, &IP6_DEFHLIM, 0, 255),
    SysctlBoundedArgs::new(IPV6CTL_MAXFRAGPACKETS, &IP6_MAXFRAGPACKETS, 0, 1000),
    SysctlBoundedArgs::new(IPV6CTL_HDRNESTLIMIT, &IP6_HDRNESTLIMIT, 0, 100),
    SysctlBoundedArgs::new(IPV6CTL_DAD_COUNT, &IP6_DAD_COUNT, 0, 10),
    SysctlBoundedArgs::new(IPV6CTL_DEFMCASTHLIM, &IP6_DEFMCASTHLIM, 0, 255),
    SysctlBoundedArgs::new(IPV6CTL_MAXFRAGS, &IP6_MAXFRAGS, 0, 1000),
    SysctlBoundedArgs::new(IPV6CTL_MFORWARDING, &IP6_MFORWARDING, 0, 1),
    SysctlBoundedArgs::new(IPV6CTL_MCAST_PMTU, &IP6_MCAST_PMTU, 0, 1),
    SysctlBoundedArgs::new(IPV6CTL_NEIGHBORGCTHRESH, &IP6_NEIGHBORGCTHRESH, 0, 5 * 2048),
    SysctlBoundedArgs::new(IPV6CTL_MAXDYNROUTES, &IP6_MAXDYNROUTES, 0, 5 * 4096),
];

/// `ip6send_mq`: the packets `ip6_send` queued for `ip6send_task`.
static IP6SEND_MQ: MbufQueue = MbufQueue::new(64, IPL_SOFTNET);
/// `ip6send_task` (its function names its queue: no argument).
static IP6SEND_TASK: Task = Task::new(ip6_send_dispatch, core::ptr::null_mut());

/// `erroff`: where the options of a hop-by-hop header right after the IPv6 header start, the
/// base of the parameter problem offsets (RFC 2460 p7: the hop-by-hop header follows the IPv6
/// header).
const HBH_ERROFF: i32 = IP6_HDR_LEN + size_of::<Ip6Hbh>() as i32;

/// `N` bytes of `m` from offset `off`, `None` when the packet does not hold them.
#[inline]
fn bytes_at<const N: usize>(m: &Mbuf, off: i32) -> Option<[u8; N]> {
    if off < 0 || i64::from(m.m_pkthdr().len.get()) < i64::from(off) + N as i64 {
        return None;
    }
    let mut b = [0u8; N];
    m_copydata(m, off, &mut b);
    Some(b)
}

/// `ip6_exthdr_get(&m, off, len)` (`<netinet/ip6.h>`): the `len` bytes at `off` of `m` made
/// contiguous, in place when the first mbuf holds them, else by `m_pulldown`: the chain back
/// with the index of the mbuf they are in and their offset there; `None` (the chain freed,
/// the C's NULL with `*mp` cleared) when the chain does not hold them or memory runs out.
pub fn ip6_exthdr_get(m: MbufBox, off: i32, len: i32) -> Option<(MbufBox, usize, usize)> {
    let (Ok(o), Ok(l)) = (usize::try_from(off), usize::try_from(len)) else {
        return None;
    };
    if m.data().len() >= o + l {
        return Some((m, 0, o));
    }
    let (m, at, o) = m_pulldown(m, off, len, true)?;
    Some((m, at, usize::try_from(o).ok()?))
}

/// The `len` bytes [`ip6_exthdr_get`] made contiguous in mbuf `at` of `m` at `o`.
#[inline]
pub fn ip6_exthdr_bytes(m: &Mbuf, at: usize, o: usize, len: usize) -> Option<&[u8]> {
    let mut n = m;
    for _ in 0..at {
        n = n.m_next.get()?;
    }
    n.data().get(o..o + len)
}

/// The byte at offset `off` of the packet set to `v` (a next header field: the C's
/// `*prvnxtp = nxt` through `ip6_exthdr_get`'s pointer, frag6's), in place in the mbuf of the
/// chain that holds it: found by the walk `off >= m_len`, never through `m_copyback` (whose
/// walk stops on the mbuf before when `off` is that mbuf's length and makes it writable for a
/// write of none of its bytes, a copy of a shared cluster). A read-only mbuf is given a
/// cluster of its own first (the C writes into the shared one); `false` when the packet does
/// not hold the byte or no cluster can be had.
#[must_use]
pub fn ip6_set_byte(m: &mut Mbuf, off: i32, v: u8) -> bool {
    let Ok(mut off) = usize::try_from(off) else {
        return false;
    };
    let mut p = m;
    while off >= p.m_len().get() as usize {
        off -= p.m_len().get() as usize;
        p = match p.m_next.get_mut() {
            Some(n) => n,
            None => return false,
        };
    }
    if let Some(d) = p.data_mut() {
        d[off] = v;
        return true;
    }
    // A shared cluster: the byte's own mbuf copied (`off` is inside it, never at its end).
    m_copyback(p, off as i32, &[v], M_DONTWAIT).is_ok()
}

/// The bytes at offset `off` of the packet set to `v` (headers the C writes in place through a
/// pointer into the packet), in the mbuf of the chain that holds `off`, found as
/// [`ip6_set_byte`] finds it; bytes that run past that mbuf, or a read-only one (a shared
/// cluster, which the C writes anyway), go through `m_copyback` from that mbuf on (`off` is
/// inside it: no copy of the mbuf before). `false` when the packet does not hold them or no
/// buffer can be had.
#[must_use]
pub fn ip6_set_bytes(m: &mut Mbuf, off: i32, v: &[u8]) -> bool {
    let Ok(mut off) = usize::try_from(off) else {
        return false;
    };
    let mut p = m;
    while off >= p.m_len().get() as usize {
        off -= p.m_len().get() as usize;
        p = match p.m_next.get_mut() {
            Some(n) => n,
            None => return false,
        };
    }
    if let Some(d) = p.data_mut()
        && let Some(dst) = d.get_mut(off..off + v.len())
    {
        dst.copy_from_slice(v);
        return true;
    }
    m_copyback(p, off as i32, v, M_DONTWAIT).is_ok()
}

/// `ip6_init`: IP6 initialization: fill in the IP6 protocol switch table (the glue's
/// `inet6sw[]`; all protocols not implemented in the kernel go to the raw IP6 protocol
/// handler), ND's and reassembly's initialisation and the receive entry point published (a
/// second call keeps the pools, the timeout and the entry point). The fragment identifiers'
/// generator is `idgen32`'s, seeded by the glue's boot (`bsd`'s `domaininit`).
pub fn ip6_init<G: ProcGlue>() {
    inet6sw_init::<G>();
    // ip6_randomid_init(): bsd's domaininit, before the domains' initialisations.
    nd6_init();
    frag6_init();
    // mq_init(&ip6send_mq, 64, IPL_SOFTNET): its initialiser.
    // ip6counters = counters_alloc(ip6s_ncounters): IP6COUNTERS.
    // MROUTING: mrt6_init(); not configured.
    let _ = IPV6_INPUT.set(ipv6_input::<G>);
}

/// `ip6_ours(mp, offp, nxt, af, flags, ns)`: enqueue packet for local delivery. Queuing is
/// used as a boundary between the network layer (input/forward path) running with
/// `NET_LOCK_SHARED()` and the transport layer needing it exclusively. `ip6` is the packet's
/// header as its caller read it.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's arguments, the header read once and the net lock's proof"
)]
pub fn ip6_ours<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    nxt: i32,
    af: i32,
    flags: i32,
    ip6: &Ip6Hdr,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let mut nxt = nxt;

    // ip6_hbhchcheck() may be run before, then off and nxt are set
    if *offp == 0 {
        nxt = ip6_hbhchcheck(mp, offp, None, flags, ip6, net);
        if nxt == IPPROTO_DONE {
            return IPPROTO_DONE;
        }
    }

    // We are already in a IPv4/IPv6 local deliver loop.
    if af != i32::from(AF_UNSPEC) {
        return nxt;
    }

    let nxt = ip_deliver::<G>(mp, offp, nxt, i32::from(AF_INET6), true, ns, net);
    if nxt == IPPROTO_DONE {
        return IPPROTO_DONE;
    }

    ip6_ours_enqueue::<G>(mp, offp, nxt)
}

/// `ip6_ours_enqueue(mp, offp, nxt)`: the packet on `ip6intrq` for `ip6intr`, with its offset
/// and next header in a tag when extension headers were processed; `IPPROTO_DONE`.
pub fn ip6_ours_enqueue<G: ProcGlue>(mp: &mut Option<MbufBox>, offp: &mut i32, nxt: i32) -> i32 {
    let Some(mut m) = mp.take() else {
        return IPPROTO_DONE;
    };

    // save values for later, use after dequeue
    if *offp != IP6_HDR_LEN {
        // mbuf tags are expensive, but only used for header options
        let Some(mut mtag) = m_tag_get(
            PACKET_TAG_IP6_OFFNXT,
            size_of::<Ipoffnxt>() as i32,
            M_NOWAIT,
        ) else {
            ip6stat_inc(Ip6statCounters::Ip6sIdropped);
            return IPPROTO_DONE;
        };
        let ion = Ipoffnxt {
            ion_off: *offp,
            ion_nxt: nxt,
        };
        let _ = pod::put(mtag.data_mut(), 0, &ion);
        m_tag_prepend(&mut m, mtag);
    }

    let _ = niq_enqueue::<G>(&IP6INTRQ, m);
    IPPROTO_DONE
}

/// `ip6intr()`: dequeue and process locally delivered packets. This is called with exclusive
/// `NET_LOCK()` (`net`, the netisr's).
pub fn ip6intr<G: ProcGlue>(net: &NetHeld<'_>) {
    while let Some(mut m) = niq_dequeue(&IP6INTRQ) {
        // DIAGNOSTIC: "ip6intr no HDR".
        crate::assert::kassert!(m.flags() & M_PKTHDR != 0);
        let (mut off, nxt);
        if let Some(mtag) = m_tag_find(&m, PACKET_TAG_IP6_OFFNXT, None) {
            let ion = pod::at::<Ipoffnxt>(&mtag.bytes(), 0).unwrap_or_default();
            off = ion.ion_off;
            nxt = ion.ion_nxt;

            let t = core::ptr::from_ref(mtag);
            m_tag_delete(&mut m, t);
        } else {
            off = IP6_HDR_LEN;
            nxt = i32::from(m.data()[offset_of!(Ip6Hdr, ip6_nxt)]);
        }
        let mut mp = Some(m);
        let nxt = ip_deliver::<G>(
            &mut mp,
            &mut off,
            nxt,
            i32::from(AF_INET6),
            false,
            None,
            net,
        );
        crate::assert::kassert!(nxt == IPPROTO_DONE);
    }
}

/// `ipv6_input(ifp, m, ns)`: an IPv6 packet from `ifp` (`ether_input`, `if_input_local`),
/// under the net lock (`net`).
pub fn ipv6_input<G: ProcGlue>(
    ifp: &'static Ifnet,
    m: MbufBox,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) {
    let mut off = 0;
    let mut mp = Some(m);
    let nxt = ip6_input_if::<G>(
        &mut mp,
        &mut off,
        IPPROTO_IPV6,
        i32::from(AF_UNSPEC),
        ifp,
        ns,
        net,
    );
    crate::assert::kassert!(nxt == IPPROTO_DONE);
}

/// `ipv6_check(ifp, m)`: the header checks of `ipv6_input`: version, address spoofing and
/// scope violations; the packet with its header as read for the checks (the one copy of it the
/// input path makes before pf: the C reads it in place, `ip6_input_if` passes this one down),
/// or `None` (freed) when it is bad.
pub fn ipv6_check(ifp: &Ifnet, m: MbufBox) -> Option<(MbufBox, Ip6Hdr)> {
    let mut m = m;

    if (m.m_len().get() as usize) < size_of::<Ip6Hdr>() {
        let Some(mm) = m_pullup(m, IP6_HDR_LEN) else {
            ip6stat_inc(Ip6statCounters::Ip6sToosmall);
            return None;
        };
        m = mm;
    }

    let ip6 = ip6_load(&m);
    let loopback = ifp.if_flags.get() & IFF_LOOPBACK != 0;

    if ip6.ip6_vfc() & IPV6_VERSION_MASK != IPV6_VERSION {
        ip6stat_inc(Ip6statCounters::Ip6sBadvers);
        return None;
    }

    // Check against address spoofing/corruption.
    if in6_is_addr_multicast(&ip6.ip6_src) || in6_is_addr_unspecified(&ip6.ip6_dst) {
        // XXX: "badscope" is not very suitable for a multicast source.
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return None;
    }
    if (in6_is_addr_loopback(&ip6.ip6_src) || in6_is_addr_loopback(&ip6.ip6_dst)) && !loopback {
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return None;
    }
    // Drop packets if interface ID portion is already filled.
    if ((in6_is_scope_embed(&ip6.ip6_src) && ip6.ip6_src.s6_addr16(1) != 0)
        || (in6_is_scope_embed(&ip6.ip6_dst) && ip6.ip6_dst.s6_addr16(1) != 0))
        && !loopback
    {
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return None;
    }
    if in6_is_addr_mc_intfacelocal(&ip6.ip6_dst) && m.flags() & M_LOOP == 0 {
        // In this case, the packet should come from the loopback interface. However, we
        // cannot just check the if_flags, because ip6_mloopback() passes the "actual"
        // interface as the outgoing/incoming interface.
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return None;
    }

    // The following check is not documented in specs. A malicious party may be able to use
    // IPv4 mapped addr to confuse tcp/udp stack and bypass security checks (act as if it was
    // from 127.0.0.1 by using IPv6 src ::ffff:127.0.0.1). Be cautious.
    //
    // This check chokes if we are in an SIIT cloud. As none of BSDs support IPv4-less kernel
    // compilation, we cannot support SIIT environment at all. So, it makes more sense for us
    // to reject any malicious packets for non-SIIT environment, than try to do a partial
    // support for SIIT environment.
    if in6_is_addr_v4mapped(&ip6.ip6_src) || in6_is_addr_v4mapped(&ip6.ip6_dst) {
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return None;
    }

    // Reject packets with IPv4 compatible addresses (auto tunnel).
    //
    // The code forbids automatic tunneling as per RFC4213.
    if in6_is_addr_v4compat(&ip6.ip6_src) || in6_is_addr_v4compat(&ip6.ip6_dst) {
        ip6stat_inc(Ip6statCounters::Ip6sBadscope);
        return None;
    }

    // The header made writable: the input path writes it in place (the scope zones, the hop
    // limit, reassembly's length and addresses), as the C does through `mtod`. A shared
    // cluster (a copy's) gets one of its own here, once; without one the packet is dropped,
    // counted as lost for want of a buffer (`ip6s_idropped`).
    if m.is_readonly() {
        let mut b = [0u8; IP6_HDR_LEN as usize];
        m_copydata(&m, 0, &mut b);
        if m_copyback(&mut m, 0, &b, M_DONTWAIT).is_err() {
            ip6stat_inc(Ip6statCounters::Ip6sIdropped);
            return None;
        }
    }

    Some((m, ip6))
}

/// The route cache `ip6_input_if` uses: the softnet's (`ns->ns_route`), or the call's own
/// (`iproute`, given back when it ends).
fn route_of<'a>(ns: &'a mut Option<&mut Netstack>, iproute: &'a mut Route) -> &'a mut Route {
    match ns {
        Some(n) => &mut n.ns_route,
        None => iproute,
    }
}

/// The IPv6 header's scope zones: the source's and the destination's embedded zone words.
#[inline]
fn zones(ip6: &Ip6Hdr) -> (u16, u16) {
    (ip6.ip6_src.s6_addr16(1), ip6.ip6_dst.s6_addr16(1))
}

/// `ip6_input_if(mp, offp, nxt, af, ifp, ns)`: process the IPv6 packet `*mp` received on
/// `ifp` (`nxt` is `IPPROTO_IPV6`, `af` `AF_INET6` or `AF_UNSPEC` for a forwarded one); the
/// next protocol or `IPPROTO_DONE`, `*mp` cleared when the packet was consumed.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's arguments and the net lock's proof"
)]
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function with its labels"
)]
pub fn ip6_input_if<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    nxt: i32,
    af: i32,
    ifp: &'static Ifnet,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let mut ns = ns;
    let mut iproute = Route::new();
    let mut flags = 0;
    let loopback = ifp.if_flags.get() & IFF_LOOPBACK != 0;

    crate::assert::kassert!(*offp == 0);

    ip6stat_inc(Ip6statCounters::Ip6sTotal);
    'bad: {
        let Some(m0) = mp.take() else {
            break 'bad;
        };
        // The header is read once (the C reads it in place): here, by the checks, and again
        // only after pf, which may rewrite or replace the packet.
        let Some((mut m, mut ip6)) = ipv6_check(ifp, m0) else {
            break 'bad;
        };

        // NCARP > 0: carp_lsdrop; not configured.
        ip6stat_inc_idx(Ip6statCounters::Ip6sNxthist, usize::from(ip6.ip6_nxt));

        // If the packet has been received on a loopback interface it can be destined to any
        // local address, not necessarily to an address configured on `ifp'.
        let mut scope = (0, 0);
        if loopback && (in6_is_scope_embed(&ip6.ip6_src) || in6_is_scope_embed(&ip6.ip6_dst)) {
            if in6_is_scope_embed(&ip6.ip6_src) {
                scope.0 = ip6.ip6_src.s6_addr16(1);
                ip6.ip6_src.set_s6_addr16(1, 0);
            }
            if in6_is_scope_embed(&ip6.ip6_dst) {
                scope.1 = ip6.ip6_dst.s6_addr16(1);
                ip6.ip6_dst.set_s6_addr16(1, 0);
            }
            if !ip6_store(&mut m, &ip6) {
                ip6stat_inc(Ip6statCounters::Ip6sIdropped);
                break 'bad;
            }
        }

        // Packet filter
        let odst = ip6.ip6_dst;
        if let Some(p) = pf() {
            let (action, mm) = p.test(AF_INET6, PF_IN, ifp, m);
            if action != PF_PASS {
                drop(mm);
                break 'bad;
            }
            let Some(mm) = mm else {
                break 'bad;
            };
            m = mm;
            ip6 = ip6_load(&m);
        }
        if odst != ip6.ip6_dst {
            flags |= IPV6_REDIRECT;
        }

        match IP6_FORWARDING.load(Ordering::Relaxed) {
            2 => flags |= IPV6_FORWARDING_IPSEC | IPV6_FORWARDING,
            1 => flags |= IPV6_FORWARDING,
            _ => {}
        }

        // Without embedded scope ID we cannot find link-local addresses in the routing table.
        let before = zones(&ip6);
        if loopback {
            if in6_is_scope_embed(&ip6.ip6_src) {
                ip6.ip6_src.set_s6_addr16(1, scope.0);
            }
            if in6_is_scope_embed(&ip6.ip6_dst) {
                ip6.ip6_dst.set_s6_addr16(1, scope.1);
            }
        } else {
            let idx = (ifp.if_index.get() as u16).to_be();
            if in6_is_scope_embed(&ip6.ip6_src) {
                ip6.ip6_src.set_s6_addr16(1, idx);
            }
            if in6_is_scope_embed(&ip6.ip6_dst) {
                ip6.ip6_dst.set_s6_addr16(1, idx);
            }
        }
        // Written back only when a zone changed (the C writes the words in place).
        if zones(&ip6) != before && !ip6_store(&mut m, &ip6) {
            ip6stat_inc(Ip6statCounters::Ip6sIdropped);
            break 'bad;
        }

        // Be more secure than RFC5095 and scan for type 0 routing headers. If pf has already
        // scanned the header chain, do not do it twice.
        if m.m_pkthdr().pf.flags.get() & PF_TAG_PROCESSED == 0 && ip6_check_rh0hdr(&m, &ip6, offp) {
            ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
            icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, *offp, net);
            break 'bad;
        }

        // NPF > 0
        if pf().is_some_and(|p| p.ouraddr(&m) == 1) {
            *mp = Some(m);
            return ip6_ours::<G>(mp, offp, nxt, af, flags, &ip6, ns, net);
        }

        // Multicast check
        if in6_is_addr_multicast(&ip6.ip6_dst) {
            // Make sure M_MCAST is set. It should theoretically already be there, but let's
            // play safe because upper layers check for this flag.
            m.m_flags().set(m.flags() | M_MCAST);

            // See if we belong to the destination multicast group on the arrival interface.
            let ours = in6_hasmulti(&ip6.ip6_dst, ifp);

            // MROUTING: when ip6_mforwarding and ip6_mrouter_active(ifp->if_rdomain),
            // ip6_hbhchcheck and ip6_mforward under KERNEL_LOCK (the packet is passed to the
            // kernel-level multicast forwarding function); not configured.

            if !ours {
                ip6stat_inc(Ip6statCounters::Ip6sNotmember);
                if !in6_is_addr_mc_linklocal(&ip6.ip6_dst) {
                    ip6stat_inc(Ip6statCounters::Ip6sCantforward);
                }
                break 'bad;
            }
            *mp = Some(m);
            return ip6_ours::<G>(mp, offp, nxt, af, flags, &ip6, ns, net);
        }

        // Unicast check
        let local = {
            let ro = route_of(&mut ns, &mut iproute);
            let rt = ro.mpath6(
                &ip6.ip6_dst.s6_addr,
                Some(&ip6.ip6_src.s6_addr),
                m.m_pkthdr().ph_rtableid.get(),
                net,
            );
            // Accept the packet if the route to the destination is marked as local (the
            // address's flags read under one reader).
            rt.filter(|rt| rt.rt_flags.get() & RTF_LOCAL != 0)
                .map(|rt| {
                    let flags6 = rt
                        .ifa(&net.reader())
                        .and_then(In6Ifaddr::of)
                        .map(|ia6| ia6.ia6_flags.get());
                    (rt.rt_ifidx.get(), flags6)
                })
        };
        if let Some((rt_ifidx, flags6)) = local {
            let Some(flags6) = flags6 else {
                break 'bad;
            };
            if flags & IPV6_FORWARDING == 0
                && rt_ifidx != ifp.if_index.get()
                && !(loopback
                    || ifp.if_type.get() == IFT_ENC
                    || m.m_pkthdr().pf.flags.get() & PF_TAG_TRANSLATE_LOCALHOST != 0)
            {
                // received on wrong interface
                // NCARP > 0: virtual IPs on carp interfaces are also checked against the
                // parent interface (carp_strict_addr_chk); not configured.
                ip6stat_inc(Ip6statCounters::Ip6sWrongif);
                break 'bad;
            }
            // packets to a tentative, duplicated, or somehow invalid address must not be
            // accepted.
            if flags6 & (IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED) != 0 {
                break 'bad;
            }
            *mp = Some(m);
            return ip6_ours::<G>(mp, offp, nxt, af, flags, &ip6, ns, net);
        }

        // NCARP > 0: carp_lsdrop for ICMPv6; not configured.

        // Now there is no reason to process the packet if it's not our own and we're not a
        // router.
        if flags & IPV6_FORWARDING == 0 {
            ip6stat_inc(Ip6statCounters::Ip6sCantforward);
            break 'bad;
        }

        let mut ours = false;
        *mp = Some(m);
        let nxt = ip6_hbhchcheck(mp, offp, Some(&mut ours), flags, &ip6, net);
        if nxt == IPPROTO_DONE {
            return IPPROTO_DONE;
        }

        if ours {
            if af == i32::from(AF_UNSPEC) {
                return ip6_ours::<G>(mp, offp, nxt, af, flags, &ip6, ns, net);
            }
            return nxt;
        }

        let Some(m) = mp.take() else {
            break 'bad;
        };
        // IPSEC
        if let Some(i) = ipsec()
            && i.in_use()
            && i.forward_check(&m, *offp, i32::from(AF_INET6))
        {
            ip6stat_inc(Ip6statCounters::Ip6sCantforward);
            break 'bad;
        }
        // Fall through, forward packet. Outbound IPsec policy checking will occur in
        // ip6_forward().

        ip6_forward(m, Some(route_of(&mut ns, &mut iproute)), flags, net);
        return IPPROTO_DONE;
    }
    // bad: the packet, if any is left, freed; iproute's route given back with it.
    drop(mp.take());
    IPPROTO_DONE
}

/// `ip6_hbhchcheck(mp, offp, oursp, flags)`: process the hop-by-hop options header, if any,
/// and trim the packet to its payload length; the next header with `*offp` after the
/// hop-by-hop options. On error frees the mbuf and returns `IPPROTO_DONE`. `ip6` is the
/// packet's header as its caller read it.
fn ip6_hbhchcheck(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    oursp: Option<&mut bool>,
    flags: i32,
    ip6: &Ip6Hdr,
    net: &NetHeld<'_>,
) -> i32 {
    let mut rtalert: u32 = !0;
    let nxt: u8;

    // Process Hop-by-Hop options header if it's contained. m may be modified in
    // ip6_hopopts_input(). If a JumboPayload option is included, plen will also be modified.
    let mut plen = u32::from(u16::from_be(ip6.ip6_plen));
    *offp = IP6_HDR_LEN;
    if i32::from(ip6.ip6_nxt) == IPPROTO_HOPOPTS {
        let Some(h) = ip6_hopopts_input(mp, offp, &mut plen, &mut rtalert, net) else {
            return IPPROTO_DONE; // m have already been freed
        };
        nxt = h;

        // if the payload length field is 0 and the next header field indicates Hop-by-Hop
        // Options header, then a Jumbo Payload option MUST be included.
        if ip6.ip6_plen == 0 && plen == 0 {
            // Note that if a valid jumbo payload option is contained, ip6_hopopts_input() must
            // set a valid (non-zero) payload length to the variable plen.
            ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
            if let Some(m) = mp.take() {
                icmp6_error(
                    m,
                    ICMP6_PARAM_PROB,
                    ICMP6_PARAMPROB_HEADER,
                    offset_of!(Ip6Hdr, ip6_plen) as i32,
                    net,
                );
            }
            return IPPROTO_DONE;
        }

        // accept the packet if a router alert option is included and we act as an IPv6
        // router.
        if rtalert != !0
            && flags & IPV6_FORWARDING != 0
            && let Some(ours) = oursp
        {
            *ours = true;
        }
    } else {
        nxt = ip6.ip6_nxt;
    }

    // Check that the amount of data in the buffers is as at least much as the IPv6 header
    // would have us expect. Trim mbufs if longer than we expect. Drop packet if shorter than
    // we expect.
    let Some(m) = mp.as_ref() else {
        return IPPROTO_DONE;
    };
    let pktlen = i64::from(m.m_pkthdr().len.get());
    let want = i64::from(IP6_HDR_LEN) + i64::from(plen);
    if pktlen - i64::from(IP6_HDR_LEN) < i64::from(plen) {
        ip6stat_inc(Ip6statCounters::Ip6sTooshort);
        drop(mp.take());
        return IPPROTO_DONE;
    }
    if pktlen > want {
        if i64::from(m.m_len().get()) == pktlen {
            m.m_len().set(want as u32);
            m.m_pkthdr().len.set(want as i32);
        } else {
            m_adj(m, (want - pktlen) as i32);
        }
    }

    i32::from(nxt)
}

/// `ip6_check_rh0hdr(m, offp)`: scan the packet (header `ip6`) for an RH0 routing header
/// (mostly stolen from pf.c's `pf_test()`); `true` with `*offp` at the offending field when
/// the packet must be rejected.
fn ip6_check_rh0hdr(m: &Mbuf, ip6: &Ip6Hdr, offp: &mut i32) -> bool {
    let mut proto = i32::from(ip6.ip6_nxt);
    let mut rh_cnt = 0;

    // The header is at the start of the data.
    let mut off = IP6_HDR_LEN;
    let lim = m
        .m_pkthdr()
        .len
        .get()
        .min(i32::from(u16::from_be(ip6.ip6_plen)) + IP6_HDR_LEN);
    loop {
        match proto {
            IPPROTO_ROUTING => {
                if rh_cnt > 0 {
                    // more than one rh header present
                    *offp = off;
                    return true;
                }
                rh_cnt += 1;

                if off + size_of::<Ip6Rthdr>() as i32 > lim {
                    // packet to short to make sense
                    *offp = off;
                    return true;
                }

                let mut b = [0u8; size_of::<Ip6Rthdr>()];
                m_copydata(m, off, &mut b);

                if i32::from(b[offset_of!(Ip6Rthdr, ip6r_type)]) == IPV6_RTHDR_TYPE_0 {
                    *offp = off + offset_of!(Ip6Rthdr, ip6r_type) as i32;
                    return true;
                }

                off += (i32::from(b[offset_of!(Ip6Rthdr, ip6r_len)]) + 1) * 8;
                proto = i32::from(b[offset_of!(Ip6Rthdr, ip6r_nxt)]);
            }
            IPPROTO_AH | IPPROTO_HOPOPTS | IPPROTO_DSTOPTS => {
                // get next header and header length
                if off + size_of::<Ip6Ext>() as i32 > lim {
                    // Packet to short to make sense, we could reject the packet but as a
                    // router we should not do that so forward it.
                    return false;
                }

                let mut b = [0u8; size_of::<Ip6Ext>()];
                m_copydata(m, off, &mut b);

                let len = i32::from(b[offset_of!(Ip6Ext, ip6e_len)]);
                if proto == IPPROTO_AH {
                    off += (len + 2) * 4;
                } else {
                    off += (len + 1) * 8;
                }
                proto = i32::from(b[offset_of!(Ip6Ext, ip6e_nxt)]);
            }
            // IPPROTO_FRAGMENT and the rest: end of header stack
            _ => return false,
        }
    }
}

/// `ip6_hopopts_input(mp, offp, plenp, rtalertp)`: Hop-by-Hop options header processing. If
/// a valid jumbo payload option is included, the real payload length will be stored in
/// `plenp`. The header's next header with `*offp` past it; `None` when the packet was freed
/// (the C's -1). rtalertp - XXX: should be stored in a more smart way
fn ip6_hopopts_input(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    plenp: &mut u32,
    rtalertp: &mut u32,
    net: &NetHeld<'_>,
) -> Option<u8> {
    const HBH: usize = size_of::<Ip6Hbh>();
    let mut off = *offp;

    // validation of the length of the header
    let Some((m, at, o)) = ip6_exthdr_get(mp.take()?, IP6_HDR_LEN, HBH as i32) else {
        ip6stat_inc(Ip6statCounters::Ip6sTooshort);
        return None;
    };
    let (nxt, len) = {
        let h = ip6_exthdr_bytes(&m, at, o, HBH)?;
        (h[0], h[1])
    };
    let mut hbhlen = (i32::from(len) + 1) << 3;
    let Some((m, _, _)) = ip6_exthdr_get(m, IP6_HDR_LEN, hbhlen) else {
        ip6stat_inc(Ip6statCounters::Ip6sTooshort);
        return None;
    };
    off += hbhlen;
    hbhlen -= HBH as i32;

    // The `hbhlen` bytes of options follow the 2-byte header right after the IPv6 header
    // (`ip6_exthdr_get` made them contiguous; the parser reads them by offset).
    *mp = Some(m);
    if !ip6_process_hopopts(mp, IP6_HDR_LEN + HBH as i32, hbhlen, rtalertp, plenp, net) {
        return None;
    }

    *offp = off;
    Some(nxt)
}

/// `ip6_hopopts_input`'s second half, `ip6_process_hopopts(mp, opthead, hbhlen, rtalertp,
/// plenp)`: searches the `hbhlen` bytes of hop-by-hop options at offset `off` of the packet
/// `*mp` (the C's `opthead`) and processes each option (the router alert into `rtalertp`, the
/// jumbo payload length into `plenp`); `true` on success (the C's 0), `false` when the packet
/// was dropped or an ICMPv6 error sent for it (the C's -1, `*mp` cleared then). This function
/// is separate from `ip6_hopopts_input()` in order to handle a case where the sending node
/// itself process its hop-by-hop options header: it is called from `ip6_output()` then. The
/// hop-by-hop header is located right after the IPv6 header (RFC2460 p7), which the error
/// offsets assume.
pub fn ip6_process_hopopts(
    mp: &mut Option<MbufBox>,
    off: i32,
    hbhlen: i32,
    rtalertp: &mut u32,
    plenp: &mut u32,
    net: &NetHeld<'_>,
) -> bool {
    let mut hbhlen = hbhlen;
    // `opt - opthead`: the current option's offset among the options.
    let mut i: i32 = 0;

    // The `N` bytes of the current option, the packet dropped as too small when it does not
    // hold them (no caller lets that happen: the options were made or pulled up whole).
    macro_rules! opt {
        ($n:literal) => {{
            match mp.as_deref().and_then(|m| bytes_at::<$n>(m, off + i)) {
                Some(b) => b,
                None => {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
            }
        }};
    }

    while hbhlen > 0 {
        let [ty] = opt!(1);
        let optlen: i32 = match ty {
            IP6OPT_PAD1 => 1,
            IP6OPT_PADN => {
                if hbhlen < IP6OPT_MINLEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [_, len] = opt!(2);
                i32::from(len) + 2
            }
            IP6OPT_ROUTER_ALERT => {
                // XXX may need check for alignment
                if hbhlen < IP6OPT_RTALERT_LEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [_, len, a0, a1] = opt!(4);
                if usize::from(len) != IP6OPT_RTALERT_LEN - 2 {
                    // XXX stat
                    if let Some(m) = mp.take() {
                        icmp6_error(
                            m,
                            ICMP6_PARAM_PROB,
                            ICMP6_PARAMPROB_HEADER,
                            HBH_ERROFF + i + 1,
                            net,
                        );
                    }
                    return false;
                }
                *rtalertp = u32::from(u16::from_be_bytes([a0, a1]));
                IP6OPT_RTALERT_LEN as i32
            }
            IP6OPT_JUMBO => {
                // XXX may need check for alignment
                if hbhlen < IP6OPT_JUMBO_LEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [_, len, j0, j1, j2, j3] = opt!(6);
                if usize::from(len) != IP6OPT_JUMBO_LEN - 2 {
                    // XXX stat
                    if let Some(m) = mp.take() {
                        icmp6_error(
                            m,
                            ICMP6_PARAM_PROB,
                            ICMP6_PARAMPROB_HEADER,
                            HBH_ERROFF + i + 1,
                            net,
                        );
                    }
                    return false;
                }

                // IPv6 packets that have non 0 payload length must not contain a jumbo
                // payload option.
                let Some(m) = mp.take() else {
                    return false;
                };
                let plen = match bytes_at::<2>(&m, offset_of!(Ip6Hdr, ip6_plen) as i32) {
                    Some(p) => u16::from_ne_bytes(p),
                    None => kpanic_short(),
                };
                if plen != 0 {
                    ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
                    icmp6_error(
                        m,
                        ICMP6_PARAM_PROB,
                        ICMP6_PARAMPROB_HEADER,
                        HBH_ERROFF + i,
                        net,
                    );
                    return false;
                }

                // We may see jumbolen in unaligned location, so we'd need to perform memcpy().
                let jumboplen = u32::from_be_bytes([j0, j1, j2, j3]);

                // if there are multiple jumbo payload options, *plenp will be non-zero and the
                // packet will be rejected. the behavior may need some debate in ipngwg -
                // multiple options does not make sense, however, there's no explicit mention in
                // specification.
                if *plenp != 0 {
                    ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
                    icmp6_error(
                        m,
                        ICMP6_PARAM_PROB,
                        ICMP6_PARAMPROB_HEADER,
                        HBH_ERROFF + i + 2,
                        net,
                    );
                    return false;
                }

                // jumbo payload length must be larger than 65535.
                if jumboplen as usize <= IPV6_MAXPACKET {
                    ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
                    icmp6_error(
                        m,
                        ICMP6_PARAM_PROB,
                        ICMP6_PARAMPROB_HEADER,
                        HBH_ERROFF + i + 2,
                        net,
                    );
                    return false;
                }
                *plenp = jumboplen;
                *mp = Some(m);
                IP6OPT_JUMBO_LEN as i32
            }
            // unknown option
            _ => {
                if hbhlen < IP6OPT_MINLEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [ty, len] = opt!(2);
                if !ip6_unknown_opt(mp, ty, HBH_ERROFF + i, net) {
                    return false;
                }
                i32::from(len) + 2
            }
        };
        hbhlen -= optlen;
        i += optlen;
    }

    true
}

/// The IPv6 header the jumbo option's check reads is not in the packet: no caller passes such
/// a packet (both put the hop-by-hop header after one), so this is `panic(9)`, as reading past
/// the first mbuf would be in the C.
#[cold]
fn kpanic_short() -> ! {
    crate::assert::kpanic(format_args!(
        "ip6_process_hopopts: a jumbo option without an IPv6 header"
    ))
}

/// `ip6_unknown_opt(mp, optp, off)`: unknown option processing, by the action bits of the
/// option's type `ty` (`*optp`). `off` is the offset from the IPv6 header to the option,
/// which allows returning an ICMPv6 error even if the IPv6 header and the option header are
/// not continuous. `true` to skip the option (the C returns its length), `false` when the
/// packet was dropped or an ICMPv6 error sent (the C's -1, `*mp` cleared).
pub fn ip6_unknown_opt(mp: &mut Option<MbufBox>, ty: u8, off: i32, net: &NetHeld<'_>) -> bool {
    match ip6opt_type(ty) {
        // ignore the option
        IP6OPT_TYPE_SKIP => true,
        // silently discard
        IP6OPT_TYPE_DISCARD => {
            drop(mp.take());
            false
        }
        // send ICMP even if multicasted
        IP6OPT_TYPE_FORCEICMP => {
            ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
            if let Some(m) = mp.take() {
                icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_OPTION, off, net);
            }
            false
        }
        // send ICMP if not multicasted
        IP6OPT_TYPE_ICMP => {
            ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
            if let Some(m) = mp.take() {
                let dst = bytes_at::<16>(&m, offset_of!(Ip6Hdr, ip6_dst) as i32);
                let mcast = dst.is_some_and(|d| {
                    in6_is_addr_multicast(&hal::sys::netinet6_in6::In6Addr { s6_addr: d })
                });
                if mcast || m.flags() & (M_BCAST | M_MCAST) != 0 {
                    drop(m);
                } else {
                    icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_OPTION, off, net);
                }
            }
            false
        }
        _ => {
            drop(mp.take()); // XXX: NOTREACHED
            false
        }
    }
}

/// `ip6_savecontrol(inp, m, mp)`: create the "control" list for this pcb: the control
/// messages `inp` asked for about the received packet `m`, chained in the C's order (`*mp`,
/// which its callers pass empty). The routine will be called from upper layer handlers like
/// `udp_input()`.
pub fn ip6_savecontrol<G: ProcGlue>(inp: &Inpcb<G>, m: &Mbuf) -> Option<MbufBox> {
    let ip6 = ip6_load(m);
    // The messages chained newest first (each made one in O(1)), reversed at the end.
    let mut rev: Option<MbufBox> = None;
    let mut put = |c: Option<MbufBox>| {
        if let Some(mut c) = c {
            if let Some(r) = rev.take() {
                drop(c.m_next.set(r));
            }
            rev = Some(c);
        }
    };

    if inp.socket().has_options(SO_TIMESTAMP) {
        let tv = m_microtime(m);
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&tv.tv_sec.to_ne_bytes());
        b[8..].copy_from_slice(&tv.tv_usec.to_ne_bytes());
        put(sbcreatecontrol(&b, SCM_TIMESTAMP, SOL_SOCKET));
    }

    // RFC 2292 sec. 5
    if inp.has_flags(IN6P_PKTINFO) {
        let mut pi6 = In6Pktinfo {
            ipi6_addr: ip6.ip6_dst,
            ipi6_ifindex: m.m_pkthdr().ph_ifidx.get(),
        };
        if in6_is_scope_embed(&pi6.ipi6_addr) {
            pi6.ipi6_addr.set_s6_addr16(1, 0);
        }
        let mut b = [0u8; size_of::<In6Pktinfo>()];
        let _ = pod::put(&mut b, 0, &pi6);
        put(sbcreatecontrol(&b, IPV6_PKTINFO, IPPROTO_IPV6));
    }

    if inp.has_flags(IN6P_HOPLIMIT) {
        let hlim = i32::from(ip6.ip6_hlim);
        put(sbcreatecontrol(
            &hlim.to_ne_bytes(),
            IPV6_HOPLIMIT,
            IPPROTO_IPV6,
        ));
    }

    if inp.has_flags(IN6P_TCLASS) {
        let flowinfo = u32::from_be(ip6.ip6_flow & IPV6_FLOWINFO_MASK) >> 20;
        let tclass = (flowinfo & 0xff) as i32;
        put(sbcreatecontrol(
            &tclass.to_ne_bytes(),
            IPV6_TCLASS,
            IPPROTO_IPV6,
        ));
    }

    // IPV6_HOPOPTS socket option. Recall that we required super-user privilege for the
    // option (see ip6_ctloutput), but it might be too strict, since there might be some
    // hop-by-hop options which can be returned to normal user. See also RFC 2292 section 6
    // (or RFC 3542 section 8).
    //
    // Check if a hop-by-hop options header is contained in the received packet, and if so,
    // store the options as ancillary data. Note that a hop-by-hop options header must be just
    // after the IPv6 header, which is assured through the IPv6 input processing.
    'done: {
        if inp.has_flags(IN6P_HOPOPTS) && i32::from(ip6.ip6_nxt) == IPPROTO_HOPOPTS {
            let Some(ext) = ip6_pullexthdr(m, IP6_HDR_LEN as usize, i32::from(ip6.ip6_nxt)) else {
                ip6stat_inc(Ip6statCounters::Ip6sTooshort);
                break 'done;
            };
            let hbh = ext.data();
            let hbhlen = (usize::from(hbh[offset_of!(Ip6Hbh, ip6h_len)]) + 1) << 3;
            if hbhlen != hbh.len() {
                ip6stat_inc(Ip6statCounters::Ip6sTooshort);
                break 'done;
            }

            // XXX: We copy the whole header even if a jumbo payload option is included, the
            // option which is to be removed before returning according to RFC2292. Note: this
            // constraint is removed in RFC3542.
            put(sbcreatecontrol(hbh, IPV6_HOPOPTS, IPPROTO_IPV6));
        }

        // IPV6_DSTOPTS and IPV6_RTHDR socket options
        if inp.has_flags(IN6P_RTHDR | IN6P_DSTOPTS) {
            let mut nxt = i32::from(ip6.ip6_nxt);
            let mut off = IP6_HDR_LEN as usize;

            // Search for destination options headers or routing header(s) through the header
            // chain, and stores each header as ancillary data. Note that the order of the
            // headers remains in the chain of ancillary data. (Is explicit loop prevention
            // necessary?) If it is not an extension header, don't try to pull it from the
            // chain: the loop's condition.
            while let IPPROTO_DSTOPTS | IPPROTO_ROUTING | IPPROTO_HOPOPTS | IPPROTO_AH = nxt {
                let Some(ext) = ip6_pullexthdr(m, off, nxt) else {
                    ip6stat_inc(Ip6statCounters::Ip6sTooshort);
                    break 'done;
                };
                let ip6e = ext.data();
                let len = usize::from(ip6e[offset_of!(Ip6Ext, ip6e_len)]);
                let elen = if nxt == IPPROTO_AH {
                    (len + 2) << 2
                } else {
                    (len + 1) << 3
                };
                if elen != ip6e.len() {
                    ip6stat_inc(Ip6statCounters::Ip6sTooshort);
                    break 'done;
                }

                match nxt {
                    IPPROTO_DSTOPTS if inp.has_flags(IN6P_DSTOPTS) => {
                        put(sbcreatecontrol(ip6e, IPV6_DSTOPTS, IPPROTO_IPV6));
                    }
                    IPPROTO_ROUTING if inp.has_flags(IN6P_RTHDR) => {
                        put(sbcreatecontrol(ip6e, IPV6_RTHDR, IPPROTO_IPV6));
                    }
                    // IPPROTO_HOPOPTS, IPPROTO_AH (is it possible?), or not asked for
                    _ => {}
                }

                // proceed with the next header.
                off += elen;
                nxt = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_nxt)]);
            }
        }
    }

    // The chain in the order made.
    let mut head: Option<MbufBox> = None;
    while let Some(mut c) = rev {
        rev = c.m_next.take();
        if let Some(h) = head.take() {
            drop(c.m_next.set(h));
        }
        head = Some(c);
    }
    head
}

/// `ip6_pullexthdr(m, off, nxt)`: pull the single extension header of type `nxt` at `off`
/// from the mbuf chain: a single mbuf that contains it, or `None` on error.
fn ip6_pullexthdr(m: &Mbuf, off: usize, nxt: i32) -> Option<MbufBox> {
    // DIAGNOSTIC: an invalid nxt is printed.
    #[cfg(feature = "diagnostic")]
    if !matches!(
        nxt,
        IPPROTO_DSTOPTS | IPPROTO_ROUTING | IPPROTO_HOPOPTS | IPPROTO_AH
    ) {
        crate::printf::printf(format_args!("ip6_pullexthdr: invalid nxt={nxt}\n"));
    }

    let pktlen = m.m_pkthdr().len.get().max(0) as usize;
    if off + size_of::<Ip6Ext>() > pktlen {
        return None;
    }

    let mut ip6e = [0u8; size_of::<Ip6Ext>()];
    m_copydata(m, off as i32, &mut ip6e);
    let len = usize::from(ip6e[offset_of!(Ip6Ext, ip6e_len)]);
    let elen = if nxt == IPPROTO_AH {
        (len + 2) << 2
    } else {
        (len + 1) << 3
    };

    if off + elen > pktlen {
        return None;
    }

    let mut n = m_get(M_DONTWAIT, MT_DATA);
    if let Some(nn) = n.as_mut()
        && elen >= MLEN
    {
        mclget(nn, M_DONTWAIT);
        if nn.flags() & M_EXT == 0 {
            n = None;
        }
    }
    let Some(mut n) = n else {
        ip6stat_inc(Ip6statCounters::Ip6sIdropped);
        return None;
    };

    n.m_len().set(0);
    if elen as i32 >= m_trailingspace(&n) {
        return None;
    }

    n.m_len().set(elen as u32);
    m_copydata(m, off as i32, n.data_mut()?);
    Some(n)
}

/// `ip6_get_prevhdr(m, off)`: the offset of the next header field that precedes offset
/// `off` (the field of the header before the one at `off`). `panic(9)` for an offset inside
/// the IPv6 header, as the C.
pub fn ip6_get_prevhdr(m: &Mbuf, off: i32) -> i32 {
    if off == IP6_HDR_LEN {
        return offset_of!(Ip6Hdr, ip6_nxt) as i32;
    } else if off < IP6_HDR_LEN {
        crate::assert::kpanic(format_args!(
            "ip6_get_prevhdr: off < sizeof(struct ip6_hdr)"
        ));
    }

    let mut nxt = i32::from(m.data()[offset_of!(Ip6Hdr, ip6_nxt)]);
    let mut len = IP6_HDR_LEN;
    let mut nlen = 0;
    while len < off {
        let mut ip6e = [0u8; size_of::<Ip6Ext>()];
        m_copydata(m, len, &mut ip6e);
        let elen = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_len)]);

        nlen = match nxt {
            IPPROTO_FRAGMENT => size_of::<Ip6Frag>() as i32,
            IPPROTO_AH => (elen + 2) << 2,
            _ => (elen + 1) << 3,
        };
        len += nlen;
        nxt = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_nxt)]);
    }

    len - nlen
}

/// `ip6_nexthdr(m, off, proto, nxtp)`: the offset of the header after the one of protocol
/// `proto` at `off`, its protocol stored in `nxtp`; `m` is retained. `None` where the C
/// returns -1.
pub fn ip6_nexthdr(m: &Mbuf, off: i32, proto: i32, nxtp: &mut i32) -> Option<i32> {
    let pktlen = m.m_pkthdr().len.get();
    if m.flags() & M_PKTHDR == 0 || pktlen < off {
        return None;
    }

    match proto {
        IPPROTO_IPV6 => {
            if pktlen < off + IP6_HDR_LEN {
                return None;
            }
            let ip6 = bytes_at::<{ size_of::<Ip6Hdr>() }>(m, off)?;
            *nxtp = i32::from(ip6[offset_of!(Ip6Hdr, ip6_nxt)]);
            Some(off + IP6_HDR_LEN)
        }
        IPPROTO_FRAGMENT => {
            // terminate parsing if it is not the first fragment, it does not make sense to
            // parse through it.
            if pktlen < off + size_of::<Ip6Frag>() as i32 {
                return None;
            }
            let fh = bytes_at::<{ size_of::<Ip6Frag>() }>(m, off)?;
            let o = offset_of!(Ip6Frag, ip6f_offlg);
            if u16::from_ne_bytes([fh[o], fh[o + 1]]) & IP6F_OFF_MASK != 0 {
                return None;
            }
            *nxtp = i32::from(fh[offset_of!(Ip6Frag, ip6f_nxt)]);
            Some(off + size_of::<Ip6Frag>() as i32)
        }
        IPPROTO_AH | IPPROTO_HOPOPTS | IPPROTO_ROUTING | IPPROTO_DSTOPTS => {
            if pktlen < off + size_of::<Ip6Ext>() as i32 {
                return None;
            }
            let ip6e = bytes_at::<{ size_of::<Ip6Ext>() }>(m, off)?;
            *nxtp = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_nxt)]);
            let len = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_len)]);
            let off = if proto == IPPROTO_AH {
                off + ((len + 2) << 2)
            } else {
                off + ((len + 1) << 3)
            };
            if pktlen < off {
                return None;
            }
            Some(off)
        }
        // give up: IPPROTO_NONE, IPPROTO_ESP, IPPROTO_IPCOMP and any other
        _ => None,
    }
}

/// `ip6_lasthdr(m, off, proto, nxtp)`: the offset of the last header in the chain (the
/// upper-layer protocol's), its protocol stored in `nxtp`; `m` is kept untainted. `None`
/// where the C returns -1 (an invalid chain).
pub fn ip6_lasthdr(m: &Mbuf, off: i32, proto: i32, nxtp: &mut i32) -> Option<i32> {
    let mut off = off;
    let mut proto = proto;

    loop {
        let Some(newoff) = ip6_nexthdr(m, off, proto, nxtp) else {
            return Some(off);
        };
        if newoff < off {
            return None; // invalid
        } else if newoff == off {
            return Some(newoff);
        }

        off = newoff;
        proto = *nxtp;
    }
}
/// `ip6_sysctl_ip6stat(oldp, oldlenp, newp)`: `net.inet6.ip6.stats`, the counters as a
/// `struct ip6stat` (summed straight into the heap image: the C's `malloc(M_TEMP)` and
/// `counters_read`).
fn ip6_sysctl_ip6stat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    const _: () = assert!(size_of::<Ip6stat>() == IP6S_NCOUNTERS * size_of::<u64>());
    let mut ip6stat = alloc::vec![0u8; size_of::<Ip6stat>()];
    IP6COUNTERS.read_into(&mut ip6stat);
    rdstruct(req, &ip6stat)
}

/// `ip6_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: system control for IP6, the
/// `net.inet6.ip6` sysctls.
pub fn ip6_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // Almost all sysctl names at this level are terminal.
    let Some(&name0) = name.first() else {
        return Err(Errno::ENOTDIR);
    };
    if name.len() != 1 && name0 != IPV6CTL_IFQUEUE {
        return Err(Errno::ENOTDIR);
    }

    match name0 {
        IPV6CTL_STATS => ip6_sysctl_ip6stat(req),
        // !MROUTING:
        IPV6CTL_MRTSTATS | IPV6CTL_MRTPROTO | IPV6CTL_MRTMIF | IPV6CTL_MRTMFC => {
            Err(Errno::EOPNOTSUPP)
        }
        IPV6CTL_MTUDISCTIMEOUT => {
            let oldval = IP6_MTUDISC_TIMEOUT.load(Ordering::Relaxed);
            let newval = AtomicI32::new(oldval);
            let error = int_bounded(req, &newval, 0, INT_MAX);
            let newval = newval.into_inner();
            if error.is_ok() && oldval != newval {
                let sysctl_lock = SYSCTL_LOCK.write();
                IP6_MTUDISC_TIMEOUT.store(newval, Ordering::Relaxed);
                rt_timer_queue_change(&ICMP6_MTUDISC_TIMEOUT_Q, newval);
                drop(sysctl_lock);
            }
            error
        }
        IPV6CTL_IFQUEUE => sysctl_niq(&name[1..], req, &IP6INTRQ),
        IPV6CTL_MULTIPATH => {
            let oldval = IP6_MULTIPATH.load(Ordering::Relaxed);
            let newval = AtomicI32::new(oldval);
            let error = int_bounded(req, &newval, 0, 1);
            let newval = newval.into_inner();
            if error.is_ok() && oldval != newval {
                IP6_MULTIPATH.store(newval, Ordering::Relaxed);
                core::sync::atomic::fence(Ordering::Release);
                RTGENERATION.fetch_add(1, Ordering::Relaxed);
            }
            error
        }
        _ => bounded_arr(&IPV6CTL_VARS, name, req),
    }
}

/// `ip6_send_dispatch(xmq)`: `ip6send_task`: sends the packets queued with `ip6_output`, under
/// the shared net lock.
fn ip6_send_dispatch(_xmq: *mut core::ffi::c_void) {
    let mut ml = mq_delist(&IP6SEND_MQ);
    if ml_empty(&ml) {
        return;
    }

    let shared = NETLOCK.read();
    let Some(net) = NetHeld::of(&shared) else {
        crate::assert::kpanic(format_args!("ip6_send: the net lock"));
    };
    while let Some(m) = ml_dequeue(&mut ml) {
        let _ = crate::netinet6::ip6_output::ip6_output(m, None, None, 0, None, None, &net);
    }
}

/// `ip6_send(m)`: queues `m` for `ip6_output` from the softnet task (ICMPv6's replies and
/// errors, made where `ip6_output` cannot be called).
pub fn ip6_send(m: MbufBox) {
    let _ = mq_enqueue(&IP6SEND_MQ, m);
    if let Some(tq) = crate::net::if_::net_tq(0) {
        let _ = task_add(tq, core::pin::Pin::static_ref(&IP6SEND_TASK));
    }
}

/// The packets `ip6_send` queued, taken (the host tests look at what ICMPv6 sent).
#[cfg(any(test, feature = "doubles"))]
pub fn ip6_send_test_take() -> crate::net::mbuf::MbufList {
    mq_delist(&IP6SEND_MQ)
}

/// The send and input queues forgotten for the host tests (their packets live in an earlier
/// test's memory).
#[cfg(any(test, feature = "doubles"))]
pub fn ip6_test_reset() {
    core::mem::forget(mq_delist(&IP6SEND_MQ));
    core::mem::forget(crate::net::if_var::niq_delist(&IP6INTRQ));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the walkers and the hop-by-hop parser over kernel packets: the chain walk
    // (LZ's `the_extension_header_chain_is_walked`, with `ip6_get_prevhdr` since N6.5d), the
    // option walk and the unknown option actions (LZ's
    // `hop_by_hop_options_are_walked`' parser half and `unknown_option_actions`), and one
    // crafted option string per check of `ip6_process_hopopts`, each passing every earlier
    // check, with the C's answer (the counter, the ICMPv6 error and its offset).

    use std::vec::Vec;

    use hal::sys::netinet_in::{IPPROTO_ESP, IPPROTO_UDP};
    use hal::sys::netinet_ip6::IPV6_VERSION;
    use hal::sys::netinet6_ip6_var::Ip6statCounters as C;

    use super::*;
    use crate::net::mbuf::m_devget;
    use crate::net::route::tests::{take_icmp6_errors, take_icmp6_params, testroutes_setup};
    use crate::netinet6::ip6_var::IP6COUNTERS;

    fn stat(c: C) -> u64 {
        IP6COUNTERS.read()[c as usize]
    }

    /// fd00::`last`.
    fn ula(last: u8) -> [u8; 16] {
        let mut a = [0u8; 16];
        a[0] = 0xfd;
        a[15] = last;
        a
    }

    /// An IPv6 header (`nxt`, payload length `plen`, to `dst`) followed by `rest`.
    fn packet(nxt: i32, plen: u16, dst: [u8; 16], rest: &[u8]) -> MbufBox {
        let mut b = Vec::new();
        b.extend_from_slice(&u32::from(IPV6_VERSION).wrapping_shl(24).to_be_bytes());
        b.extend_from_slice(&plen.to_be_bytes());
        b.push(nxt as u8);
        b.push(64);
        b.extend_from_slice(&ula(1));
        b.extend_from_slice(&dst);
        b.extend_from_slice(rest);
        m_devget(&b, 0).expect("packet")
    }

    /// The hop-by-hop options `opts` of a packet to `dst` with payload length `plen` (the
    /// options after the IPv6 header and the 2-byte hop-by-hop header, then 8 bytes of UDP, so
    /// that an option read past the options' length finds bytes, as in the C's packets, rather
    /// than the packet's end), run through the parser: its answer, the router alert and jumbo
    /// length it stored, whether the packet is still the caller's.
    fn run(opts: &[u8], plen: u16, dst: [u8; 16]) -> (bool, u32, u32, bool) {
        let mut rest = std::vec![IPPROTO_UDP as u8, 0];
        rest.extend_from_slice(opts);
        rest.extend_from_slice(&[0; 8]);
        let mut mp = Some(packet(IPPROTO_HOPOPTS, plen, dst, &rest));
        let (mut rtalert, mut jumbo) = (!0, 0);
        let r = hal::sync::netlock::NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        let ok = ip6_process_hopopts(
            &mut mp,
            HBH_ERROFF,
            opts.len() as i32,
            &mut rtalert,
            &mut jumbo,
            &net,
        );
        (ok, rtalert, jumbo, mp.is_some())
    }

    #[test]
    fn hop_by_hop_options_are_walked() {
        let _g = testroutes_setup();
        let _ = take_icmp6_errors();

        // Pad1, a router alert for MLD (0x0000 as `MLD6_HBH`), PadN of 0.
        let opts = [IP6OPT_PAD1, IP6OPT_ROUTER_ALERT, 2, 0, 1, IP6OPT_PADN, 0, 0];
        assert_eq!(run(&opts, 8, ula(2)), (true, 1, 0, true));

        // An unknown option whose action is "skip" (type 0x1e) between PadN options.
        let opts = [0x1e, 2, 0xaa, 0xbb, IP6OPT_PADN, 0, 0, 0];
        assert_eq!(run(&opts[..6], 6, ula(2)), (true, !0, 0, true));

        // A valid jumbo option on a packet whose payload length is 0.
        let opts = [IP6OPT_PADN, 0, IP6OPT_JUMBO, 4, 0, 1, 0, 0];
        assert_eq!(run(&opts, 0, ula(2)), (true, !0, 0x1_0000, true));
        assert!(take_icmp6_errors().is_empty());
    }

    /// One option string per check of `ip6_process_hopopts` (each passes every earlier check)
    /// and the C's answer: the counter it bumps, the parameter problem it sends (its offset
    /// from the IPv6 header) or none (dropped silently, or counted only).
    #[test]
    fn each_bound_of_the_option_walk_answers_as_the_c() {
        let _g = testroutes_setup();
        struct Case {
            what: &'static str,
            opts: &'static [u8],
            plen: u16,
            counter: Option<C>,
            error: Option<(u8, u8, i32)>,
        }
        let prob = |code, param| Some((ICMP6_PARAM_PROB, code, param));
        let cases = [
            Case {
                what: "PadN with one byte left",
                opts: &[IP6OPT_PAD1, IP6OPT_PADN],
                plen: 8,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "router alert with three bytes left",
                opts: &[IP6OPT_ROUTER_ALERT, 2, 0],
                plen: 8,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "router alert of a wrong length",
                opts: &[IP6OPT_ROUTER_ALERT, 3, 0, 0, 0, 0],
                plen: 8,
                counter: None,
                error: prob(ICMP6_PARAMPROB_HEADER, 43),
            },
            Case {
                what: "jumbo with five bytes left",
                opts: &[IP6OPT_JUMBO, 4, 0, 1, 0],
                plen: 0,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "jumbo of a wrong length",
                opts: &[IP6OPT_JUMBO, 5, 0, 1, 0, 0, 0, 0],
                plen: 0,
                counter: None,
                error: prob(ICMP6_PARAMPROB_HEADER, 43),
            },
            Case {
                what: "jumbo with a payload length",
                opts: &[IP6OPT_JUMBO, 4, 0, 1, 0, 0],
                plen: 8,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_HEADER, 42),
            },
            Case {
                what: "two jumbo options",
                opts: &[IP6OPT_JUMBO, 4, 0, 1, 0, 0, IP6OPT_JUMBO, 4, 0, 1, 0, 0],
                plen: 0,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_HEADER, 50),
            },
            Case {
                what: "jumbo of at most 65535",
                opts: &[IP6OPT_JUMBO, 4, 0, 0, 0xff, 0xff],
                plen: 0,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_HEADER, 44),
            },
            Case {
                what: "unknown option with one byte left",
                opts: &[IP6OPT_PAD1, 0x1e],
                plen: 8,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "unknown option, discard",
                opts: &[0x5e, 0],
                plen: 8,
                counter: None,
                error: None,
            },
            Case {
                what: "unknown option, ICMP even to multicast",
                opts: &[0x9e, 0],
                plen: 8,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_OPTION, 42),
            },
            Case {
                what: "unknown option, ICMP unless multicast",
                opts: &[0xde, 0],
                plen: 8,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_OPTION, 42),
            },
        ];
        for c in &cases {
            let _ = take_icmp6_errors();
            let _ = take_icmp6_params();
            let before = c.counter.map(stat);
            let (ok, _, _, kept) = run(c.opts, c.plen, ula(2));
            assert!(!ok && !kept, "{}: refused, the packet gone", c.what);
            if let (Some(k), Some(b)) = (c.counter, before) {
                assert_eq!(stat(k), b + 1, "{}: counted", c.what);
            }
            let errors = take_icmp6_errors();
            let params = take_icmp6_params();
            match c.error {
                Some((t, code, param)) => {
                    assert_eq!(errors, [(t, code)], "{}: the parameter problem", c.what);
                    assert_eq!(params, [param], "{}: its offset", c.what);
                }
                None => assert!(errors.is_empty(), "{}: no error", c.what),
            }
        }
        // "ICMP unless multicast" to a multicast destination: dropped without an error.
        let mut mcast = ula(2);
        mcast[0] = 0xff;
        let _ = take_icmp6_errors();
        assert!(!run(&[0xde, 0], 8, mcast).0);
        assert!(
            take_icmp6_errors().is_empty(),
            "no error to a multicast group"
        );
    }

    #[test]
    fn unknown_option_actions() {
        let _g = testroutes_setup();
        let mut mp = Some(packet(IPPROTO_UDP, 0, ula(2), &[0x1e, 0]));
        let r = hal::sync::netlock::NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        assert!(ip6_unknown_opt(&mut mp, 0x1e, 42, &net));
        assert!(mp.is_some());

        assert!(!ip6_unknown_opt(&mut mp, 0x7f, 42, &net));
        assert!(mp.is_none());
    }

    /// An IPv6 header, a hop-by-hop header (8), destination options (16), a fragment header
    /// (`offlg`) and 8 bytes of UDP.
    fn chain(offlg: u16) -> MbufBox {
        let mut rest = Vec::new();
        rest.extend_from_slice(&[IPPROTO_DSTOPTS as u8, 0, 1, 4, 0, 0, 0, 0]);
        rest.extend_from_slice(&[IPPROTO_FRAGMENT as u8, 1, 1, 12]);
        rest.extend_from_slice(&[0; 12]);
        rest.extend_from_slice(&[IPPROTO_UDP as u8, 0]);
        rest.extend_from_slice(&offlg.to_be_bytes());
        rest.extend_from_slice(&[0, 0, 0, 7]);
        rest.extend_from_slice(&[0; 8]);
        packet(IPPROTO_HOPOPTS, rest.len() as u16, ula(2), &rest)
    }

    #[test]
    fn the_extension_header_chain_is_walked() {
        let _g = testroutes_setup();

        let m = chain(0x0001); // first fragment, more fragments
        let mut nxt = -1;
        assert_eq!(ip6_nexthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(40));
        assert_eq!(nxt, IPPROTO_HOPOPTS);
        assert_eq!(ip6_nexthdr(&m, 40, IPPROTO_HOPOPTS, &mut nxt), Some(48));
        assert_eq!(nxt, IPPROTO_DSTOPTS);
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(72));
        assert_eq!(nxt, IPPROTO_UDP);
        // The next header field before each header: the IPv6 header's, the hop-by-hop
        // header's, the destination options'.
        assert_eq!(ip6_get_prevhdr(&m, 40), offset_of!(Ip6Hdr, ip6_nxt) as i32);
        assert_eq!(ip6_get_prevhdr(&m, 64), 48);
        assert_eq!(ip6_get_prevhdr(&m, 72), 64);

        // A later fragment: the walk stops at the fragment header.
        let m = chain(0x0040); // byte offset 64
        let mut nxt = -1;
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(64));
        assert_eq!(nxt, IPPROTO_FRAGMENT);
        // An ESP header is never walked into.
        assert_eq!(ip6_nexthdr(&m, 64, IPPROTO_ESP, &mut nxt), None);
        // A header past the end of the packet.
        assert_eq!(ip6_nexthdr(&m, 1000, IPPROTO_HOPOPTS, &mut nxt), None);

        // AH counts its length in 4-byte words, plus 2.
        let ah = [IPPROTO_UDP as u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let m = packet(IPPROTO_AH, ah.len() as u16, ula(2), &ah);
        let mut nxt = -1;
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(52));
        assert_eq!(nxt, IPPROTO_UDP);

        // A header whose length runs past the packet ends the walk where it starts; its next
        // header is stored all the same (the C stores `*nxtp` before the length test).
        let m = packet(
            IPPROTO_DSTOPTS,
            8,
            ula(2),
            &[IPPROTO_UDP as u8, 4, 0, 0, 0, 0, 0, 0],
        );
        let mut nxt = -1;
        assert_eq!(ip6_nexthdr(&m, 40, IPPROTO_DSTOPTS, &mut nxt), None);
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(40));
        assert_eq!(nxt, IPPROTO_UDP);
    }

    // The input path (N6.5d): ipv6_check's header checks (LZ's `ipv6_check_rejects_bad_headers`
    // as a table, each check alone), the shared header's copy, ip6_hbhchcheck's payload length
    // and trimming (LZ's) and its hop-by-hop bounds, the type 0 routing header scan (LZ's and
    // its bounds), ip6_input_if's decisions over ND's test interface (delivered with `af`
    // AF_INET6, as IP-in-IP calls it: the protocol switch is bsd's until the table moves),
    // ip6_savecontrol's messages and the sysctls (LZ's `deep_names_are_not_directories`).

    use hal::sys::netinet_icmp6::{ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOROUTE};
    use hal::sys::netinet_in::IPPROTO_TCP;
    use hal::sys::netinet6_in6::In6Addr;
    use hal::user::UserPtr;

    use crate::net::mbuf::{M_COPYALL, m_copym, m_split};
    use crate::netinet6::icmp6::tests::{net6, packet as packet6, with_net};
    use crate::netinet6::nd6::tests::{ether_bare, ours6, peer6};

    type G = crate::fs::testfs::TestGlue;

    /// The user-copy window of these tests' sysctls (beside ICMPv6's, under the same lock).
    const USYS6: usize = crate::testutil::USER_BASE + 0x3_9000;

    fn in6(b: [u8; 16]) -> In6Addr {
        let mut a = In6Addr::default();
        a.s6_addr = b;
        a
    }

    /// `pfx` followed by zeros and `last`.
    fn addr6(pfx: &[u8], last: u8) -> In6Addr {
        let mut b = [0u8; 16];
        b[..pfx.len()].copy_from_slice(pfx);
        b[15] |= last;
        in6(b)
    }

    /// An IPv6 packet from `src` to `dst` (next header `nxt`) with payload `rest`, received on
    /// `ifp`.
    fn rx(ifp: &Ifnet, src: In6Addr, dst: In6Addr, nxt: i32, rest: &[u8]) -> MbufBox {
        let m = m_devget(&packet6(src, dst, nxt as u8, 64, rest), 0).expect("a packet");
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        m
    }

    #[test]
    fn ipv6_check_answers_each_header_as_the_c() {
        let (_g, ifp) = net6();
        let good = ula(2);
        let mc = addr6(&[0xff, 0x02], 1);
        let lo = addr6(&[], 1);
        let mut v4m = [0u8; 16];
        v4m[10..].copy_from_slice(&[0xff, 0xff, 10, 0, 0, 1]);
        let v4m = in6(v4m);
        let v4c = addr6(&[], 0);
        let mut v4c = v4c;
        v4c.s6_addr[12..].copy_from_slice(&[10, 0, 0, 1]);
        let emb = addr6(&[0xfe, 0x80, 0, 7], 1);
        let ll = addr6(&[0xfe, 0x80], 1);
        let ifl = addr6(&[0xff, 0x01], 1);
        // (what, source, destination, version field, on a loopback interface, M_LOOP, the
        // counter it bumps: none when it passes)
        #[rustfmt::skip]
        let cases: [(&str, In6Addr, In6Addr, u8, bool, bool, Option<C>); 19] = [
            ("good", in6(ula(1)), in6(good), IPV6_VERSION, false, false, None),
            ("version 4", in6(ula(1)), in6(good), 0x40, false, false, Some(C::Ip6sBadvers)),
            ("multicast source", mc, in6(good), IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("unspecified destination", in6(ula(1)), in6([0; 16]), IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("::1 source", lo, in6(good), IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("::1 destination", in6(ula(1)), lo, IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("::1 on loopback", lo, lo, IPV6_VERSION, true, false, None),
            ("embedded zone source", emb, in6(good), IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("embedded zone destination", in6(ula(1)), emb, IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("embedded zone on loopback", emb, emb, IPV6_VERSION, true, false, None),
            ("link-local without a zone", ll, ll, IPV6_VERSION, false, false, None),
            ("interface-local multicast", in6(ula(1)), ifl, IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("interface-local looped back", in6(ula(1)), ifl, IPV6_VERSION, false, true, None),
            ("v4-mapped source", v4m, in6(good), IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("v4-mapped destination", in6(ula(1)), v4m, IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("v4-compatible source", v4c, in6(good), IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("v4-compatible destination", in6(ula(1)), v4c, IPV6_VERSION, false, false, Some(C::Ip6sBadscope)),
            ("v4-mapped on loopback", v4m, in6(good), IPV6_VERSION, true, false, Some(C::Ip6sBadscope)),
            ("multicast source on loopback", mc, lo, IPV6_VERSION, true, false, Some(C::Ip6sBadscope)),
        ];
        let flags = ifp.if_flags.get();
        for (what, src, dst, vfc, lo, mloop, counter) in cases {
            let mut b = packet6(src, dst, IPPROTO_UDP as u8, 64, &[0; 8]);
            b[0] = vfc;
            let m = m_devget(&b, 0).expect("a packet");
            if mloop {
                m.m_flags().set(m.flags() | M_LOOP);
            }
            ifp.if_flags
                .set(if lo { flags | IFF_LOOPBACK } else { flags });
            let (vers, scope) = (stat(C::Ip6sBadvers), stat(C::Ip6sBadscope));
            let r = ipv6_check(ifp, m);
            ifp.if_flags.set(flags);
            match counter {
                None => {
                    let (_, ip6) = r.expect(what);
                    assert_eq!((ip6.ip6_src, ip6.ip6_dst), (src, dst), "{what}");
                    assert_eq!((stat(C::Ip6sBadvers), stat(C::Ip6sBadscope)), (vers, scope));
                }
                Some(c) => {
                    assert!(r.is_none(), "{what}");
                    let n = if c == C::Ip6sBadvers { vers } else { scope };
                    assert_eq!(stat(c), n + 1, "{what}");
                }
            }
        }
        // A packet shorter than the header.
        let toosmall = stat(C::Ip6sToosmall);
        assert!(ipv6_check(ifp, m_devget(&[0x60; 20], 0).expect("a packet")).is_none());
        assert_eq!(stat(C::Ip6sToosmall), toosmall + 1);
    }

    #[test]
    fn a_shared_header_gets_its_own_copy_before_the_input_path_writes_it() {
        let (_g, ifp) = net6();
        let mut m = rx(ifp, peer6(), ours6(), IPPROTO_UDP, &[0; 600]);
        let c = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT).expect("a copy");
        assert!(c.is_readonly(), "the copy shares the cluster");
        let (mut c, mut ip6) = ipv6_check(ifp, c).expect("a good header");
        assert!(!c.is_readonly());
        ip6.ip6_hlim = 1;
        assert!(ip6_store(&mut c, &ip6));
        assert_eq!(ip6_load(&c).ip6_hlim, 1);
        assert_eq!(ip6_load(&m).ip6_hlim, 64, "the other copy untouched");
    }

    /// `ip6_hbhchcheck` of `m` as `ip6_input_if` runs it (its header as read), under the net
    /// lock: the next header, the offset, whether a router alert made it ours, the packet.
    fn hbhchcheck(m: MbufBox, flags: i32) -> (i32, i32, bool, Option<MbufBox>) {
        let ip6 = ip6_load(&m);
        let mut mp = Some(m);
        let (mut off, mut ours) = (0, false);
        let nxt =
            with_net(|net| ip6_hbhchcheck(&mut mp, &mut off, Some(&mut ours), flags, &ip6, net));
        (nxt, off, ours, mp)
    }

    #[test]
    fn the_payload_length_is_checked_and_the_packet_trimmed() {
        let _g = testroutes_setup();

        // The header announces 16 bytes, only 8 follow.
        let tooshort = stat(C::Ip6sTooshort);
        let (nxt, _, _, m) = hbhchcheck(packet(IPPROTO_UDP, 16, ula(2), &[0; 8]), 0);
        assert_eq!(nxt, IPPROTO_DONE);
        assert!(m.is_none());
        assert_eq!(stat(C::Ip6sTooshort), tooshort + 1);

        // Exactly as announced: kept whole.
        let (nxt, off, _, m) = hbhchcheck(packet(IPPROTO_UDP, 8, ula(2), &[0; 8]), 0);
        assert_eq!((nxt, off), (IPPROTO_UDP, 40));
        assert_eq!(m.expect("kept").m_pkthdr().len.get(), 48);

        // The header announces 8 bytes, 24 follow (link-layer padding): trimmed to 48, in the
        // one mbuf and across a chain.
        let (nxt, off, _, m) = hbhchcheck(packet(IPPROTO_UDP, 8, ula(2), &[0; 24]), 0);
        assert_eq!((nxt, off), (IPPROTO_UDP, 40));
        let m = m.expect("kept");
        assert_eq!((m.m_len().get(), m.m_pkthdr().len.get()), (48, 48));
        let mut m = packet(IPPROTO_UDP, 8, ula(2), &[0; 24]);
        let tail = m_split(&mut m, 44, M_DONTWAIT).expect("a split");
        drop(m.m_next.set(tail));
        crate::net::mbuf::m_calchdrlen(&m);
        let (nxt, _, _, m) = hbhchcheck(m, 0);
        assert_eq!(nxt, IPPROTO_UDP);
        let m = m.expect("kept");
        assert_eq!(m.m_pkthdr().len.get(), 48);
        let mut total = 0;
        let mut p: Option<&Mbuf> = Some(&m);
        while let Some(q) = p {
            total += q.m_len().get();
            p = q.m_next.get();
        }
        assert_eq!(total, 48, "the chain trimmed with its header");
    }

    #[test]
    fn each_bound_of_the_hop_by_hop_check_answers_as_the_c() {
        let _g = testroutes_setup();
        let _ = take_icmp6_errors();
        let alert = [
            IPPROTO_UDP as u8,
            0,
            IP6OPT_ROUTER_ALERT,
            2,
            0,
            0,
            IP6OPT_PADN,
            0,
        ];

        // A router alert makes a router take the packet; a host does not.
        let (nxt, off, ours, _) =
            hbhchcheck(packet(IPPROTO_HOPOPTS, 8, ula(2), &alert), IPV6_FORWARDING);
        assert_eq!((nxt, off, ours), (IPPROTO_UDP, 48, true));
        let (nxt, _, ours, _) = hbhchcheck(packet(IPPROTO_HOPOPTS, 8, ula(2), &alert), 0);
        assert_eq!((nxt, ours), (IPPROTO_UDP, false));
        let pads = [IPPROTO_UDP as u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0];
        let (_, _, ours, _) =
            hbhchcheck(packet(IPPROTO_HOPOPTS, 8, ula(2), &pads), IPV6_FORWARDING);
        assert!(!ours, "no router alert");

        // A zero payload length needs a jumbo option.
        let bad = stat(C::Ip6sBadoptions);
        let (nxt, _, _, m) = hbhchcheck(packet(IPPROTO_HOPOPTS, 0, ula(2), &pads), 0);
        assert_eq!(nxt, IPPROTO_DONE);
        assert!(m.is_none());
        assert_eq!(stat(C::Ip6sBadoptions), bad + 1);
        assert_eq!(
            take_icmp6_errors(),
            [(ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER)]
        );
        assert_eq!(take_icmp6_params(), [offset_of!(Ip6Hdr, ip6_plen) as i32]);

        // A jumbo length the packet does not hold.
        // (the option's value 0x1_0000, past IPV6_MAXPACKET, in a 16-byte header)
        let jumbo = [
            IPPROTO_UDP as u8,
            1,
            IP6OPT_PADN,
            0,
            IP6OPT_JUMBO,
            4,
            0,
            1,
            0,
            0,
            IP6OPT_PADN,
            4,
            0,
            0,
            0,
            0,
        ];
        let tooshort = stat(C::Ip6sTooshort);
        let (nxt, _, _, _) = hbhchcheck(packet(IPPROTO_HOPOPTS, 0, ula(2), &jumbo), 0);
        assert_eq!(nxt, IPPROTO_DONE);
        assert_eq!(stat(C::Ip6sTooshort), tooshort + 1);

        // The hop-by-hop header cut short: its first two bytes, then its announced length.
        for rest in [
            &[IPPROTO_UDP as u8][..],
            &[IPPROTO_UDP as u8, 1, IP6OPT_PADN, 4, 0, 0, 0, 0],
        ] {
            let tooshort = stat(C::Ip6sTooshort);
            let (nxt, _, _, m) =
                hbhchcheck(packet(IPPROTO_HOPOPTS, rest.len() as u16, ula(2), rest), 0);
            assert_eq!(nxt, IPPROTO_DONE);
            assert!(m.is_none());
            assert_eq!(stat(C::Ip6sTooshort), tooshort + 1, "{rest:?}");
        }
        assert!(take_icmp6_errors().is_empty());
    }

    /// `ip6_check_rh0hdr` of a packet with next header `nxt`, payload length `plen` and the
    /// extension headers `rest`: the verdict and the offset it names.
    fn rh0(nxt: i32, plen: u16, rest: &[u8]) -> (bool, i32) {
        let m = packet(nxt, plen, ula(2), rest);
        let mut off = 0;
        let r = ip6_check_rh0hdr(&m, &ip6_load(&m), &mut off);
        (r, off)
    }

    #[test]
    fn type_0_routing_headers_are_rejected() {
        let _g = testroutes_setup();
        let rthdr = |ty: u8, nxt: i32| [nxt as u8, 0, ty, 0, 0, 0, 0, 0];
        let ty = offset_of!(Ip6Rthdr, ip6r_type) as i32;

        assert_eq!(
            rh0(IPPROTO_ROUTING, 8, &rthdr(0, IPPROTO_UDP)),
            (true, 40 + ty)
        );
        assert!(!rh0(IPPROTO_ROUTING, 8, &rthdr(2, IPPROTO_UDP)).0);

        // Two routing headers, behind a destination options header.
        let mut rest = std::vec![IPPROTO_ROUTING as u8, 0, 1, 4, 0, 0, 0, 0];
        rest.extend_from_slice(&rthdr(2, IPPROTO_ROUTING));
        rest.extend_from_slice(&rthdr(2, IPPROTO_UDP));
        assert_eq!(rh0(IPPROTO_DSTOPTS, rest.len() as u16, &rest), (true, 56));

        // A routing header whose 4 fixed bytes the payload length cuts short (its bytes in
        // the packet past the payload) is rejected at its start; at the length, it is read.
        assert_eq!(rh0(IPPROTO_ROUTING, 3, &rthdr(2, IPPROTO_UDP)), (true, 40));
        assert!(!rh0(IPPROTO_ROUTING, 4, &rthdr(2, IPPROTO_UDP)).0);
        // A destination options header cut short is let through (a router forwards it), even
        // when the bytes past the payload would name a routing header.
        assert!(!rh0(IPPROTO_DSTOPTS, 1, &rthdr(0, IPPROTO_UDP)).0);
        let mut rest = std::vec![IPPROTO_ROUTING as u8, 0, 1, 4, 0, 0, 0, 0];
        rest.extend_from_slice(&rthdr(0, IPPROTO_UDP));
        assert!(!rh0(IPPROTO_DSTOPTS, 1, &rest).0);
        // AH counts in 4-byte words plus 2: the type 0 header right after its 12 bytes.
        let mut rest = std::vec![IPPROTO_ROUTING as u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        rest.extend_from_slice(&rthdr(0, IPPROTO_UDP));
        assert_eq!(rh0(IPPROTO_AH, rest.len() as u16, &rest), (true, 52 + ty));
        // Behind hop-by-hop options, 8-byte units.
        let mut rest = std::vec![IPPROTO_ROUTING as u8, 0, 1, 4, 0, 0, 0, 0];
        rest.extend_from_slice(&rthdr(0, IPPROTO_UDP));
        assert_eq!(
            rh0(IPPROTO_HOPOPTS, rest.len() as u16, &rest),
            (true, 48 + ty)
        );
        // A fragment header ends the scan.
        let mut rest = std::vec![IPPROTO_ROUTING as u8, 0, 0, 1, 0, 0, 0, 7];
        rest.extend_from_slice(&rthdr(0, IPPROTO_UDP));
        assert!(!rh0(IPPROTO_FRAGMENT, rest.len() as u16, &rest).0);
    }

    /// `ip6_input_if` of `m` received on `ifp` with `af` AF_INET6 (IP-in-IP's call, which
    /// stops before the protocol switch), under the shared net lock: the next header, the
    /// offset, the packet if it is still the caller's.
    fn input_if(ifp: &'static Ifnet, m: MbufBox) -> (i32, i32, Option<MbufBox>) {
        let r = NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        let mut mp = Some(m);
        let mut off = 0;
        let nxt = ip6_input_if::<G>(
            &mut mp,
            &mut off,
            IPPROTO_IPV6,
            i32::from(AF_INET6),
            ifp,
            None,
            &net,
        );
        (nxt, off, mp)
    }

    #[test]
    fn the_input_path_delivers_what_is_ours_and_drops_the_rest() {
        let (_g, ifp) = net6();
        let _ = take_icmp6_errors();
        let other = ether_bare(b"tin1");

        // Ours, on the interface that has the address.
        let hist = |n: i32| IP6COUNTERS.read()[C::Ip6sNxthist as usize + n as usize];
        let (total, udp) = (stat(C::Ip6sTotal), hist(IPPROTO_UDP));
        let (nxt, off, m) = input_if(ifp, rx(ifp, peer6(), ours6(), IPPROTO_UDP, &[0; 8]));
        assert_eq!((nxt, off), (IPPROTO_UDP, 40));
        assert!(m.is_some());
        assert_eq!(
            (stat(C::Ip6sTotal), hist(IPPROTO_UDP)),
            (total + 1, udp + 1)
        );

        // Ours, on another interface: refused by a host, taken by a router, by any interface
        // from the loopback or enc(4), and for a packet pf rewrote to localhost.
        let wrongif = stat(C::Ip6sWrongif);
        let (nxt, _, m) = input_if(other, rx(other, peer6(), ours6(), IPPROTO_UDP, &[0; 8]));
        assert_eq!((nxt, m.is_none()), (IPPROTO_DONE, true));
        assert_eq!(stat(C::Ip6sWrongif), wrongif + 1);
        IP6_FORWARDING.store(1, Ordering::Relaxed);
        let (nxt, _, _) = input_if(other, rx(other, peer6(), ours6(), IPPROTO_UDP, &[0; 8]));
        IP6_FORWARDING.store(0, Ordering::Relaxed);
        assert_eq!(nxt, IPPROTO_UDP, "a router");
        other.if_type.set(IFT_ENC);
        let (nxt, _, _) = input_if(other, rx(other, peer6(), ours6(), IPPROTO_UDP, &[0; 8]));
        other.if_type.set(hal::sys::net_if::IFT_ETHER);
        assert_eq!(nxt, IPPROTO_UDP, "enc(4)");
        let m = rx(other, peer6(), ours6(), IPPROTO_UDP, &[0; 8]);
        m.m_pkthdr().pf.flags.set(PF_TAG_TRANSLATE_LOCALHOST);
        assert_eq!(input_if(other, m).0, IPPROTO_UDP, "pf's localhost");
        let flags = other.if_flags.get();
        other.if_flags.set(flags | IFF_LOOPBACK);
        let (nxt, _, _) = input_if(other, rx(other, peer6(), ours6(), IPPROTO_UDP, &[0; 8]));
        other.if_flags.set(flags);
        assert_eq!(nxt, IPPROTO_UDP, "the loopback");
        assert_eq!(stat(C::Ip6sWrongif), wrongif + 1);

        // A tentative or duplicated address takes nothing, silently.
        let ifa = crate::netinet6::in6::tests::withaddr(ifp, &ours6()).expect("ours");
        let ia6 = crate::netinet6::in6_var::ifatoia6(&ifa);
        for f in [IN6_IFF_TENTATIVE, IN6_IFF_DUPLICATED] {
            let old = ia6.ia6_flags.get();
            ia6.ia6_flags.set(old | f);
            let (nxt, _, m) = input_if(ifp, rx(ifp, peer6(), ours6(), IPPROTO_UDP, &[0; 8]));
            ia6.ia6_flags.set(old);
            assert_eq!((nxt, m.is_none()), (IPPROTO_DONE, true), "{f:#x}");
        }

        // Not ours: a host does not forward.
        let cant = stat(C::Ip6sCantforward);
        let far = addr6(&[0x20, 0x01, 0x0d, 0xb8], 9);
        assert_eq!(
            input_if(ifp, rx(ifp, peer6(), far, IPPROTO_UDP, &[0; 8])).0,
            IPPROTO_DONE
        );
        assert_eq!(stat(C::Ip6sCantforward), cant + 1);
        // A router hands it to ip6_forward, which finds no route; a router alert makes it ours.
        IP6_FORWARDING.store(1, Ordering::Relaxed);
        let noroute = stat(C::Ip6sNoroute);
        assert_eq!(
            input_if(ifp, rx(ifp, peer6(), far, IPPROTO_UDP, &[0; 8])).0,
            IPPROTO_DONE
        );
        assert_eq!(stat(C::Ip6sNoroute), noroute + 1);
        let alert = [
            IPPROTO_UDP as u8,
            0,
            IP6OPT_ROUTER_ALERT,
            2,
            0,
            0,
            IP6OPT_PADN,
            0,
        ];
        let (nxt, off, _) = input_if(ifp, rx(ifp, peer6(), far, IPPROTO_HOPOPTS, &alert));
        IP6_FORWARDING.store(0, Ordering::Relaxed);
        assert_eq!((nxt, off), (IPPROTO_UDP, 48));
        assert_eq!(
            take_icmp6_errors(),
            [(ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOROUTE)]
        );

        // Multicast: a group joined on the interface is ours; another is not, counted as not
        // forwarded but for link-local scope.
        let all = addr6(&[0xff, 0x02], 1);
        assert_eq!(
            input_if(ifp, rx(ifp, peer6(), all, IPPROTO_UDP, &[0; 8])).0,
            IPPROTO_UDP
        );
        let (notmember, cant) = (stat(C::Ip6sNotmember), stat(C::Ip6sCantforward));
        let link = addr6(&[0xff, 0x02], 9);
        assert_eq!(
            input_if(ifp, rx(ifp, peer6(), link, IPPROTO_UDP, &[0; 8])).0,
            IPPROTO_DONE
        );
        assert_eq!(
            (stat(C::Ip6sNotmember), stat(C::Ip6sCantforward)),
            (notmember + 1, cant)
        );
        let global = addr6(&[0xff, 0x0e], 9);
        let m = rx(ifp, peer6(), global, IPPROTO_UDP, &[0; 8]);
        assert_eq!(input_if(ifp, m).0, IPPROTO_DONE);
        assert_eq!(
            (stat(C::Ip6sNotmember), stat(C::Ip6sCantforward)),
            (notmember + 2, cant + 1)
        );
    }

    #[test]
    fn the_input_path_embeds_the_zone_and_refuses_type_0_routing_headers() {
        let (_g, ifp) = net6();
        let _ = take_icmp6_errors();
        // A link-local source gets the arrival interface's zone in its header.
        let ll = addr6(&[0xfe, 0x80], 2);
        let (nxt, _, m) = input_if(ifp, rx(ifp, ll, ours6(), IPPROTO_UDP, &[0; 8]));
        assert_eq!(nxt, IPPROTO_UDP);
        let ip6 = ip6_load(&m.expect("ours"));
        assert_eq!(
            ip6.ip6_src.s6_addr16(1),
            (ifp.if_index.get() as u16).to_be()
        );

        // A type 0 routing header: a parameter problem at its type; not scanned again once
        // pf scanned the chain.
        let rh = [IPPROTO_UDP as u8, 0, 0, 0, 0, 0, 0, 0];
        let bad = stat(C::Ip6sBadoptions);
        let (nxt, _, m) = input_if(ifp, rx(ifp, peer6(), ours6(), IPPROTO_ROUTING, &rh));
        assert_eq!((nxt, m.is_none()), (IPPROTO_DONE, true));
        assert_eq!(stat(C::Ip6sBadoptions), bad + 1);
        assert_eq!(
            take_icmp6_errors(),
            [(ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER)]
        );
        assert_eq!(
            take_icmp6_params(),
            [40 + offset_of!(Ip6Rthdr, ip6r_type) as i32]
        );
        let m = rx(ifp, peer6(), ours6(), IPPROTO_ROUTING, &rh);
        m.m_pkthdr().pf.flags.set(PF_TAG_PROCESSED);
        assert_eq!(input_if(ifp, m).0, IPPROTO_ROUTING);

        // A bad version never reaches the checks after ipv6_check.
        let mut b = packet6(peer6(), ours6(), IPPROTO_UDP as u8, 64, &[0; 8]);
        b[0] = 0x40;
        let vers = stat(C::Ip6sBadvers);
        assert_eq!(
            input_if(ifp, m_devget(&b, 0).expect("a packet")).0,
            IPPROTO_DONE
        );
        assert_eq!(stat(C::Ip6sBadvers), vers + 1);
    }

    #[test]
    fn ip6_ours_enqueue_keeps_the_offset_of_the_extension_headers() {
        let (_g, ifp) = net6();
        let _ = crate::net::if_var::niq_delist(&IP6INTRQ);
        let mut mp = Some(rx(
            ifp,
            peer6(),
            ours6(),
            IPPROTO_HOPOPTS,
            &[IPPROTO_TCP as u8, 0, 1, 4, 0, 0, 0, 0],
        ));
        let mut off = 48;
        assert_eq!(
            ip6_ours_enqueue::<G>(&mut mp, &mut off, IPPROTO_TCP),
            IPPROTO_DONE
        );
        assert!(mp.is_none());
        let mut mp = Some(rx(ifp, peer6(), ours6(), IPPROTO_TCP, &[0; 20]));
        let mut off = 40;
        assert_eq!(
            ip6_ours_enqueue::<G>(&mut mp, &mut off, IPPROTO_TCP),
            IPPROTO_DONE
        );
        let m = niq_dequeue(&IP6INTRQ).expect("the first");
        let t = m_tag_find(&m, PACKET_TAG_IP6_OFFNXT, None).expect("its offset tagged");
        let ion = pod::at::<Ipoffnxt>(&t.bytes(), 0).expect("an ipoffnxt");
        assert_eq!((ion.ion_off, ion.ion_nxt), (48, IPPROTO_TCP));
        let m = niq_dequeue(&IP6INTRQ).expect("the second");
        assert!(
            m_tag_find(&m, PACKET_TAG_IP6_OFFNXT, None).is_none(),
            "no tag after the header alone"
        );
    }

    #[test]
    fn ip6_savecontrol_makes_the_messages_asked_for_in_order() {
        use crate::netinet::in_pcb::{INP_IPV6, testinet};
        let (_g, _p) = testinet::setup();
        let (so, inp) = testinet::pcb(testinet::table(16));
        inp.set_flags(
            INP_IPV6
                | IN6P_PKTINFO
                | IN6P_HOPLIMIT
                | IN6P_TCLASS
                | IN6P_DSTOPTS
                | IN6P_RTHDR
                | IN6P_HOPOPTS,
        );
        // Hop-by-hop options, destination options, a routing header, then UDP.
        let mut rest = std::vec![IPPROTO_DSTOPTS as u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0];
        rest.extend_from_slice(&[IPPROTO_ROUTING as u8, 0, IP6OPT_PADN, 4, 0, 0, 0, 0]);
        rest.extend_from_slice(&[IPPROTO_UDP as u8, 0, 2, 0, 0, 0, 0, 0]);
        rest.extend_from_slice(&[0; 8]);
        let mut b = packet6(peer6(), ours6(), IPPROTO_HOPOPTS as u8, 7, &rest);
        b[0] = 0x6a; // traffic class 0xa5
        b[1] = 0x50;
        let m = m_devget(&b, 0).expect("a packet");
        m.m_pkthdr().ph_ifidx.set(3);
        let mut c = ip6_savecontrol::<G>(&inp, &m);
        let mut types = std::vec::Vec::new();
        while let Some(mut x) = c {
            let d = x.data();
            types.push(i32::from_ne_bytes([d[8], d[9], d[10], d[11]]));
            // The data after the 12-byte cmsghdr, aligned to 16.
            match types.len() {
                1 => {
                    let pi = pod::at::<In6Pktinfo>(d, 16).expect("an in6_pktinfo");
                    assert_eq!((pi.ipi6_addr, pi.ipi6_ifindex), (ours6(), 3));
                }
                2 => assert_eq!(
                    i32::from_ne_bytes([d[16], d[17], d[18], d[19]]),
                    7,
                    "the hop limit"
                ),
                3 => assert_eq!(
                    i32::from_ne_bytes([d[16], d[17], d[18], d[19]]),
                    0xa5,
                    "the class"
                ),
                n => assert_eq!(d.len(), 16 + 8, "header {n}: its 8 bytes"),
            }
            c = x.m_next.take();
        }
        assert_eq!(
            types,
            [
                IPV6_PKTINFO,
                IPV6_HOPLIMIT,
                IPV6_TCLASS,
                IPV6_HOPOPTS,
                IPV6_DSTOPTS,
                IPV6_RTHDR
            ]
        );
        // Asked for the routing header alone: that message alone.
        inp.clear_flags(IN6P_PKTINFO | IN6P_HOPLIMIT | IN6P_TCLASS | IN6P_DSTOPTS | IN6P_HOPOPTS);
        let c = ip6_savecontrol::<G>(&inp, &m).expect("one message");
        assert!(c.m_next.get().is_none());
        testinet::release(so, inp);
    }

    /// `ip6_sysctl` of `name`, writing `new` when given: the value read and the answer.
    fn sysctl6(name: &[i32], new: Option<i32>) -> (i32, Result<(), Errno>) {
        let mut oldlen = size_of::<i32>();
        if let Some(v) = new {
            assert!(UserPtr::<i32>::new(USYS6 + 0x100).copyout(&v).is_ok());
        }
        let mut req = SysctlReq {
            oldp: USYS6,
            oldlenp: &mut oldlen,
            newp: if new.is_some() { USYS6 + 0x100 } else { 0 },
            newlen: if new.is_some() { size_of::<i32>() } else { 0 },
        };
        let r = ip6_sysctl(name, &mut req);
        let v = UserPtr::<i32>::new(USYS6).copyin().unwrap_or(-7);
        (v, r)
    }

    #[test]
    fn the_sysctls_read_and_write_the_variables() {
        let _g = testroutes_setup();
        // Deep names are not directories, but for the input queue's.
        for name in [
            IPV6CTL_DEFHLIM,
            IPV6CTL_STATS,
            IPV6CTL_MULTIPATH,
            IPV6CTL_MTUDISCTIMEOUT,
        ] {
            assert_eq!(sysctl6(&[name, 1], None).1, Err(Errno::ENOTDIR), "{name}");
        }
        assert_eq!(sysctl6(&[], None).1, Err(Errno::ENOTDIR));
        assert_ne!(sysctl6(&[IPV6CTL_IFQUEUE, 1], None).1, Err(Errno::ENOTDIR));
        for name in [
            IPV6CTL_MRTSTATS,
            IPV6CTL_MRTPROTO,
            IPV6CTL_MRTMIF,
            IPV6CTL_MRTMFC,
        ] {
            assert_eq!(sysctl6(&[name], None).1, Err(Errno::EOPNOTSUPP));
        }
        // The bounded names: in bounds taken, out of bounds EINVAL.
        for (name, var, lo, hi) in [
            (IPV6CTL_FORWARDING, &IP6_FORWARDING, 0, 2),
            (IPV6CTL_SENDREDIRECTS, &IP6_SENDREDIRECTS, 0, 1),
            (IPV6CTL_DEFHLIM, &IP6_DEFHLIM, 0, 255),
            (IPV6CTL_MAXFRAGPACKETS, &IP6_MAXFRAGPACKETS, 0, 1000),
            (IPV6CTL_HDRNESTLIMIT, &IP6_HDRNESTLIMIT, 0, 100),
            (IPV6CTL_DAD_COUNT, &IP6_DAD_COUNT, 0, 10),
            (IPV6CTL_DEFMCASTHLIM, &IP6_DEFMCASTHLIM, 0, 255),
            (IPV6CTL_MAXFRAGS, &IP6_MAXFRAGS, 0, 1000),
            (IPV6CTL_MFORWARDING, &IP6_MFORWARDING, 0, 1),
            (IPV6CTL_MCAST_PMTU, &IP6_MCAST_PMTU, 0, 1),
            (IPV6CTL_NEIGHBORGCTHRESH, &IP6_NEIGHBORGCTHRESH, 0, 5 * 2048),
            (IPV6CTL_MAXDYNROUTES, &IP6_MAXDYNROUTES, 0, 5 * 4096),
            (IPV6CTL_MTUDISCTIMEOUT, &IP6_MTUDISC_TIMEOUT, 0, INT_MAX),
            (IPV6CTL_MULTIPATH, &IP6_MULTIPATH, 0, 1),
        ] {
            let old = var.load(Ordering::Relaxed);
            assert_eq!(sysctl6(&[name], None), (old, Ok(())), "{name}");
            assert_eq!(sysctl6(&[name], Some(lo)).1, Ok(()));
            assert_eq!(var.load(Ordering::Relaxed), lo);
            assert_eq!(sysctl6(&[name], Some(hi)).1, Ok(()));
            assert_eq!(var.load(Ordering::Relaxed), hi);
            assert_eq!(
                sysctl6(&[name], Some(lo - 1)).1,
                Err(Errno::EINVAL),
                "{name}"
            );
            if hi < INT_MAX {
                assert_eq!(
                    sysctl6(&[name], Some(hi + 1)).1,
                    Err(Errno::EINVAL),
                    "{name}"
                );
            }
            assert_eq!(var.load(Ordering::Relaxed), hi, "unchanged by a refusal");
            var.store(old, Ordering::Relaxed);
        }
        // The DAD count in progress is read only.
        assert_eq!(
            sysctl6(&[IPV6CTL_DAD_PENDING], Some(1)).1,
            Err(Errno::EPERM)
        );
        // A change of the multipath switch makes the cached routes stale; the same value not.
        let generation = RTGENERATION.load(Ordering::Relaxed);
        assert_eq!(sysctl6(&[IPV6CTL_MULTIPATH], Some(1)).1, Ok(()));
        assert_eq!(RTGENERATION.load(Ordering::Relaxed), generation + 1);
        assert_eq!(sysctl6(&[IPV6CTL_MULTIPATH], Some(1)).1, Ok(()));
        assert_eq!(RTGENERATION.load(Ordering::Relaxed), generation + 1);
        IP6_MULTIPATH.store(0, Ordering::Relaxed);
        // The statistics: the whole struct ip6stat, ip6s_total first.
        let mut len = size_of::<Ip6stat>();
        let mut req = SysctlReq {
            oldp: USYS6,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(ip6_sysctl(&[IPV6CTL_STATS], &mut req), Ok(()));
        assert_eq!(len, size_of::<Ip6stat>());
        let total = UserPtr::<u64>::new(USYS6).copyin().expect("ip6s_total");
        assert_eq!(total, stat(C::Ip6sTotal));
    }
}
/* </TESTS> */
