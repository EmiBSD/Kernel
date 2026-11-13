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
//! IPv4 input: header checks, options, local delivery through the protocol switch,
//! reassembly, forwarding and the `net.inet.ip` sysctls.
//!
//! Upstream: sys/netinet/ip_input.c @ 3ce1f3f79392
//! LZ: sys/netinet/ip_input.rs@3c62ede68802
//!
//! [`ipv4_input`] (from `ether_input` or `if_input_local`, through [`IPV4_INPUT`]) checks
//! and checksums the header ([`ipv4_check`]), runs pf, processes the options
//! ([`ip_dooptions`]) and either delivers the datagram ([`in_ouraddr`] says it is ours:
//! [`ip_ours`] reassembles fragments and runs the protocols that take the shared net lock,
//! the rest go through `ipintrq` to [`ipintr`]) or forwards it ([`ip_forward`], only with
//! `net.inet.ip.forwarding`). [`ip_deliver`] walks the protocol switch
//! (`inetsw[ip_protox[nxt]]`).
//!
//! Locks: \[I\] immutable after creation, \[N\] net lock, \[Q\] `ipq_mutex`, \[a\] atomic
//! operations.
//!
//! ## Deviations
//! - `NPF` (pf(4)) is configured: `pf_test`, `pf_ouraddr` and `pf_find_divert` are the `Pf`
//!   bridge's legs, which answer as the C without pf before pf registers.
//! - `IPSEC` is configured: `ipsec_in_use`, `ipsec_forward_check`, `ipsec_local_check`,
//!   `ipsec_init` and `ipsec_sysctl` are the `Ipsec` bridge's legs (`bsd`'s until N8); any
//!   refusal of the checks drops the packet, as the C's non-zero.
//! - `INET6` is configured and IPv6 is `bsd`'s until N6.5: `ip_deliver`'s IPv6 steps
//!   (`inet6sw[ip6_protox[nxt]]`, the `ip6stat` counters, `ip6_hdrnestlimit`,
//!   `ip6_ours_enqueue`) are the `Inet` bridge's legs.
//! - Not configured, each a comment at its site: `NCARP` (`carp_lsdrop`,
//!   `carp_strict_addr_chk`) and `MROUTING` (`ip_mforward`, `ip_mrouter_active`, `mrt_init`;
//!   the `mrt` sysctls answer `EOPNOTSUPP` as the C's `#else` does).
//! - `ip_init` fills `in_pcb.c`'s `baddynamicports`/`rootonlyports` from the default lists
//!   (slices without the C's terminating 0).
//! - `ipsend_mq`/`ipsendraw_mq` are made with the C's 64 and `IPL_SOFTNET` in their
//!   initialisers, so `ip_init` has no `mq_init` to run.
//! - The C writes the header, the options and ICMP's reply in place through `mtod`, into a
//!   cluster it may share with a copy. The kernel cannot lend a shared cluster's bytes
//!   mutably: `ipv4_check` gives a read-only first mbuf a cluster of its own, once, and
//!   drops the packet when none can be had (`ips_idropped`); every later write is then in
//!   place. The writers (`ip_store`, `pkt_put`, `ip_stripoptions`) still answer `false` for
//!   data they cannot write, and their callers drop the packet, counted as the C counts its
//!   buffer failures there (`ips_rcvmemdrop`, `ips_idropped`, `ips_cantforward`,
//!   `ips_odropped`, ICMP's `icps_tooshort`, IGMP's `igps_rcv_tooshort`): never taken while
//!   the first mbuf is writable.
//!
//! ## Redesign
//! - LZ's `struct mbuf **` functions over `&mut Option<&'static Mbuf>` (parked mbufs) ->
//!   `&mut Option<MbufBox>`: the packet is owned, `m_freemp` is a `take` and a drop.
//!   `ip_dooptions` and `ip_forward` take the packet by value (`ip_dooptions` gives it back
//!   when it should be processed further); `ipv4_check` gives back the trimmed packet.
//! - The IP header: LZ's `mtod_ip`/`mtod_ip_store` copies through `unsafe` ->
//!   `kernel::netinet::ip_output`'s `ip_load`/`ip_store` (a view in place when aligned, a
//!   copy otherwise, copy-on-write for a read-only cluster). The input path reads the
//!   header once, in `ipv4_check`, and passes that copy down (`in_ouraddr`, `ip_ours`,
//!   `ip_fragcheck`; 20 bytes where the C reads in place), reading it again only after pf
//!   (which may rewrite or replace the packet) and after the options (source routes rewrite
//!   `ip_dst`). The options `ip_dooptions`
//!   walks (LZ: a mutable slice made with `from_raw_parts_mut`) are copied out of the
//!   header (at most 40 bytes), walked with the C's bounds, and written back when an option
//!   changed them (`IPOPT_RR`, `IPOPT_LSRR`/`SSRR`'s next hop and the timestamp).
//! - The reassembly queue (LZ: `IPQ`, a `bsd` `ListHead` made `Sync` by hand, with the
//!   `ipq_mutex` beside it and its entries written whole into pool items through `unsafe`)
//!   -> [`IPQ`], a `Mutexed` (`IPL_SOFTNET`) owning [`IpqState`]: the datagrams on an
//!   `OwnedSlist` of "ipq" items, each owning its fragments on an `OwnedSlist` of "ipqe"
//!   items (`kernel::netinet::ip_var::{Ipq, Ipqent}`, D-15), and `ip_frags`. The C's
//!   `MUTEX_ASSERT_LOCKED(&ipq_mutex)` is the `&mut IpqState` parameter of `ip_reass`,
//!   `ip_freef` and `ip_flush`, which only the guard lends. The datagram being completed
//!   is taken off the queue for `ip_reass` (the walk keeps its predecessor, `remove_next`)
//!   and put back in its place (`split_after`, `push_tail`, `concat`), all O(1): the C's
//!   order of the queue, which `ip_flush` frees from its head, is kept. A fragment inserted
//!   in the middle of its datagram goes in the same way. See `kernel::netinet::ip_var` for
//!   the records.
//! - `ip_protox[]`/`inetsw[]` are `kernel::netinet::in_proto`'s; a protocol's `pr_input`
//!   is `Protocol::input`, the packet owned.
//! - `ipintrq` is the kernel's `Niqueue`; [`ipintr`] is called by the kernel's netisr with
//!   the proof of its exclusive net lock (the `Inet` legs `ipintr` and `ipv4_input` go);
//!   `ipv4_input`'s glue-generic instance is published once by [`ip_init`] in the `G`-erased
//!   slot [`IPV4_INPUT`] that `ether_input` and `if_input_local` read (D-9, the
//!   `BPF_REVOKE` and `ARPINPUT` shape).
//! - `NET_ASSERT_LOCKED()` along the input path is the `net` proof every function takes:
//!   `if_input_process`'s shared hold lends it to [`ipv4_input`] through the protocol
//!   function's new argument (`ProtoFn`), `ipintr` the netisr's exclusive one.
//! - `ipsend_task`/`ipsendraw_task` have no argument: their functions name their queues.
//! - `ip_savecontrol` answers the control chain it made (`*mp` in the C, which its callers
//!   pass empty).
//! - `struct ip_srcrt` is [`IpSrcrt`], the `PACKET_TAG_SRCROUTE` tag's image (`abi_pod!`).
//!
//! **Lock and spl transitions** against the C: `ipv4_input`..`ip_deliver` run under the
//! caller's shared net lock and take none of their own on the unfragmented path, as the C;
//! `ip_fragcheck` takes `ipq_mutex` once per fragment (one spl pair), as the C; `ip_dooptions`
//! takes the kernel lock C-style around the option walk, as the C; `ip_send`'s task takes the
//! net lock shared. `in_ouraddr`'s route cache hit is `Route::mpath4`'s (N6.3b2).

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::mutex::Mutexed;
use hal::netlock::{NETLOCK, NetHeld};
use hal::once::OnceStatic;
use hal::owned::{OwnedSlist, SlistAdapter};
use hal::pod;
use hal::pool::{BoxPool, PR_NOWAIT, PoolBox};
use hal::sys::errno::Errno;
use hal::sys::limits::INT_MAX;
use hal::sys::net_if::{IFF_LOOPBACK, IFT_ENC};
use hal::sys::netinet_in::{
    IN_CLASSA_NSHIFT, IN_LOOPBACKNET, INADDR_ANY, INADDR_BROADCAST, IP_RECVDSTADDR, IP_RECVIF,
    IP_RECVRTABLE, IP_RECVTTL, IPCTL_ARPDOWN, IPCTL_ARPQUEUE, IPCTL_ARPQUEUED, IPCTL_ARPTIMEOUT,
    IPCTL_DEFTTL, IPCTL_DIRECTEDBCAST, IPCTL_ENCDEBUG, IPCTL_FORWARDING, IPCTL_IFQUEUE,
    IPCTL_IPPORT_FIRSTAUTO, IPCTL_IPPORT_HIFIRSTAUTO, IPCTL_IPPORT_HILASTAUTO,
    IPCTL_IPPORT_LASTAUTO, IPCTL_IPPORT_MAXQUEUE, IPCTL_IPSEC_ALLOCATIONS,
    IPCTL_IPSEC_AUTH_ALGORITHM, IPCTL_IPSEC_BYTES, IPCTL_IPSEC_EMBRYONIC_SA_TIMEOUT,
    IPCTL_IPSEC_ENC_ALGORITHM, IPCTL_IPSEC_EXPIRE_ACQUIRE, IPCTL_IPSEC_FIRSTUSE,
    IPCTL_IPSEC_IPCOMP_ALGORITHM, IPCTL_IPSEC_REQUIRE_PFS, IPCTL_IPSEC_SOFT_ALLOCATIONS,
    IPCTL_IPSEC_SOFT_BYTES, IPCTL_IPSEC_SOFT_FIRSTUSE, IPCTL_IPSEC_SOFT_TIMEOUT, IPCTL_IPSEC_STATS,
    IPCTL_IPSEC_TIMEOUT, IPCTL_MFORWARDING, IPCTL_MRTMFC, IPCTL_MRTPROTO, IPCTL_MRTSTATS,
    IPCTL_MRTVIF, IPCTL_MTUDISC, IPCTL_MTUDISCTIMEOUT, IPCTL_MULTIPATH, IPCTL_SENDREDIRECTS,
    IPCTL_SOURCEROUTE, IPCTL_STATS, IPPROTO_DONE, IPPROTO_IP, IPPROTO_IPV4, IPPROTO_IPV6, InAddr,
    SockaddrIn, in_classfulbroadcast, in_local_group, in_multicast,
};
use hal::sys::netinet_ip::{
    IP_MAXPACKET, IP_MF, IP_OFFMASK, IPFRAGTTL, IPOPT_EOL, IPOPT_LSRR, IPOPT_MINOFF, IPOPT_NOP,
    IPOPT_OFFSET, IPOPT_OLEN, IPOPT_OPTVAL, IPOPT_RR, IPOPT_SSRR, IPOPT_TS, IPOPT_TS_PRESPEC,
    IPOPT_TS_TSANDADDR, IPOPT_TS_TSONLY, IPQ_MAXLEN, IPTOS_ECN_CE, IPTOS_ECN_MASK,
    IPTOS_ECN_NOTECT, IPTTLDEC, IPVERSION, Ip, IpTimestamp,
};
use hal::sys::netinet_ip_icmp::{
    ICMP_PARAMPROB, ICMP_REDIRECT, ICMP_REDIRECT_HOST, ICMP_TIMXCEED, ICMP_TIMXCEED_INTRANS,
    ICMP_UNREACH, ICMP_UNREACH_HOST, ICMP_UNREACH_NEEDFRAG, ICMP_UNREACH_SRCFAIL,
};
use hal::sys::netinet_ip_var::{Ipstat, MAX_IPOPTLEN};
use hal::sys::socket::{
    AF_INET, AF_INET6, AF_LINK, AF_UNSPEC, SCM_TIMESTAMP, SO_TIMESTAMP, SOL_SOCKET,
};
use hal::sys::sysctl::SysctlBoundedArgs;

use crate::assert::{kassert, kpanic};
use crate::net::if_::{
    Ip6Deliver, if_get, if_put, ifa_ifwithaddr, ifaof_ifpforaddr, inet, net_tq, niq_enqueue,
    sysctl_niq,
};
use crate::net::if_var::{Ifnet, Netstack, Niqueue, niq_dequeue};
use crate::net::ipsec_hooks::ipsec;
use crate::net::mbuf::{
    M_BCAST, M_COPYFLAGS, M_DONTWAIT, M_EXT, M_IPV4_CSUM_IN_BAD, M_IPV4_CSUM_IN_OK, M_MCAST,
    M_PKTHDR, MT_DATA, MT_SOOPTS, Mbuf, MbufBox, MbufQueue, PACKET_TAG_IP_OFFNXT,
    PACKET_TAG_IPSEC_FLOWINFO, PACKET_TAG_SRCROUTE, PF_TAG_DIVERTED, PF_TAG_GENERATED,
    PF_TAG_TRANSLATE_LOCALHOST, m_adj, m_calchdrlen, m_cat, m_copyback, m_copydata, m_get,
    m_gethdr, m_microtime, m_pullup, m_removehdr, ml_dequeue, ml_empty, mq_delist, mq_enqueue,
};
use crate::net::mbuf2::{m_tag_delete, m_tag_find, m_tag_get, m_tag_prepend};
use crate::net::netisr::NETISR_IP;
use crate::net::pf_hooks::{PF_IN, PF_PASS, pf};
use crate::net::proto::{PR_MPINPUT, PRC_NCMDS};
use hal::sys::net_route::{RTF_BROADCAST, RTF_DYNAMIC, RTF_GATEWAY, RTF_LOCAL, RTF_MODIFIED};

use crate::net::proto::SockaddrView;
use crate::net::route::{
    RT_RESOLVE, RTGENERATION, Route, rt_timer_queue_change, rt_timer_queue_flush, rtalloc,
    rtisvalid,
};
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::sbcreatecontrol;
use crate::netinet::if_ether::{ARPINQ, ARPT_DOWN, ARPT_KEEP, arpinit, arpproxy, la_hold_total};
use crate::netinet::in_::{in_canforward, in_hasmulti};
use crate::netinet::in_cksum::in_cksum;
use crate::netinet::in_pcb::{
    BADDYNAMICPORTS, DEFBADDYNAMICPORTS_TCP, DEFBADDYNAMICPORTS_UDP, DEFROOTONLYPORTS_TCP,
    DEFROOTONLYPORTS_UDP, INP_RECVDSTADDR, INP_RECVIF, INP_RECVRTABLE, INP_RECVTTL,
    IPPORT_FIRSTAUTO, IPPORT_LASTAUTO, Inpcb, ROOTONLYPORTS, dp_set, ipport_hifirstauto,
    ipport_hilastauto,
};
use crate::netinet::in_proto::{inet6_protocol, inetsw_init, ip_protocol};
use crate::netinet::in_var::InIfaddr;
use crate::netinet::ip_icmp::{IP_MTUDISC_TIMEOUT_Q, icmp_error, iptime};
use crate::netinet::ip_id::ip_randomid_init;
use crate::netinet::ip_output::{ip_load, ip_output, ip_store};
use crate::netinet::ip_var::{
    IP_ALLOWBROADCAST, IP_DEFTTL, IP_FORWARDING, IP_FORWARDING_IPSEC, IP_RAWOUTPUT, IP_REDIRECT,
    IPCOUNTERS, IPMTUDISCTIMEOUT, IPMULTIPATH, IPS_NCOUNTERS, Ipoffnxt, Ipq, IpqList, Ipqent,
    IpqentList, IpstatCounters, ip_mtudisc, ipstat_dec, ipstat_inc,
};
use crate::proc::ProcGlue;
use crate::sysctl::{
    SYSCTL_LOCK, SysctlReq, bounded_arr, int_bounded, rdint, rdstruct, securelevel_int,
};
use crate::task::{Task, task_add};

/// `sizeof(struct ip)`.
const IP_HDR_LEN: usize = size_of::<Ip>();

/// `M_NOWAIT` of `m_tag_get`.
const M_NOWAIT: i32 = 0x0002;

/// `struct ip_srcrt`: the IP options of an incoming packet saved in case a protocol wants to
/// respond to it over the same route if it got here using IP source routing. This allows
/// connection establishment and maintenance when the remote end is on a network that is not
/// known to us. The `PACKET_TAG_SRCROUTE` tag's image.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpSrcrt {
    /// `isr_nhops`: number of hops.
    pub isr_nhops: i32,
    /// `isr_dst`: final destination.
    pub isr_dst: InAddr,
    /// `isr_nop`: one NOP to align.
    pub isr_nop: u8,
    /// `isr_hdr`: OPTVAL, OLEN & OFFSET.
    pub isr_hdr: [u8; IPOPT_OFFSET + 1],
    /// `isr_routes`.
    pub isr_routes: [InAddr; MAX_IPOPTLEN / size_of::<InAddr>()],
}

hal::abi_pod!(IpSrcrt {
    isr_nhops: i32,
    isr_dst: InAddr,
    isr_nop: u8,
    isr_hdr: [u8; IPOPT_OFFSET + 1],
    isr_routes: [InAddr; MAX_IPOPTLEN / size_of::<InAddr>()],
});

/// Where `isr_hdr` is in the tag's image (`save_rte`'s `memcpy` runs on into `isr_routes`).
const ISR_HDR: usize = core::mem::offset_of!(IpSrcrt, isr_hdr);

/// \[Q\] What `ipq_mutex` guards: the reassembly queue and the fragments it holds.
pub struct IpqState {
    /// `ipq`: the datagrams being reassembled, the newest first.
    q: OwnedSlist<IpqList>,
    /// `ip_frags`: the fragments queued.
    frags: i32,
}

#[cfg(test)]
impl IpqState {
    /// `ip_frags`, for the tests.
    pub(crate) fn frags(&self) -> i32 {
        self.frags
    }

    /// The datagrams being reassembled, for the tests.
    pub(crate) fn queues(&self) -> usize {
        self.q.len()
    }
}

/// \[a\] `ip_forwarding`: act as router (2: only IPsec processed packets).
#[allow(non_upper_case_globals)] // the C name; `IP_FORWARDING` is `ip_var.h`'s output flag
pub static ip_forwarding: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ipmforwarding`.
pub static IPMFORWARDING: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip_sendredirects`.
pub static IP_SENDREDIRECTS: AtomicI32 = AtomicI32::new(1);
/// \[a\] `ip_dosourceroute`.
pub static IP_DOSOURCEROUTE: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip_mtudisc_timeout`: seconds to timeout mtu discovery.
pub static IP_MTUDISC_TIMEOUT: AtomicI32 = AtomicI32::new(IPMTUDISCTIMEOUT);
/// \[a\] `ip_directedbcast`: accept all broadcast packets.
pub static IP_DIRECTEDBCAST: AtomicI32 = AtomicI32::new(0);
/// \[a\] `ip_maxqueue`: keep track of memory used for reassembly.
pub static IP_MAXQUEUE: AtomicI32 = AtomicI32::new(300);

/// `ipq_mutex` and the reassembly queue it guards.
pub static IPQ: Mutexed<IpqState> = Mutexed::new(
    IPL_SOFTNET,
    IpqState {
        q: OwnedSlist::new(),
        frags: 0,
    },
);

/// `ipctl_vars[]`.
static IPCTL_VARS: [SysctlBoundedArgs; 12] = [
    SysctlBoundedArgs::new(IPCTL_FORWARDING, &ip_forwarding, 0, 2),
    SysctlBoundedArgs::new(IPCTL_SENDREDIRECTS, &IP_SENDREDIRECTS, 0, 1),
    SysctlBoundedArgs::new(IPCTL_DIRECTEDBCAST, &IP_DIRECTEDBCAST, 0, 1),
    // MROUTING: IPCTL_MRTPROTO, read only; not configured.
    SysctlBoundedArgs::new(IPCTL_DEFTTL, &IP_DEFTTL, 0, 255),
    SysctlBoundedArgs::new(IPCTL_IPPORT_FIRSTAUTO, &IPPORT_FIRSTAUTO, 0, 65535),
    SysctlBoundedArgs::new(IPCTL_IPPORT_LASTAUTO, &IPPORT_LASTAUTO, 0, 65535),
    SysctlBoundedArgs::new(IPCTL_IPPORT_HIFIRSTAUTO, &ipport_hifirstauto, 0, 65535),
    SysctlBoundedArgs::new(IPCTL_IPPORT_HILASTAUTO, &ipport_hilastauto, 0, 65535),
    SysctlBoundedArgs::new(IPCTL_IPPORT_MAXQUEUE, &IP_MAXQUEUE, 0, 10000),
    SysctlBoundedArgs::new(IPCTL_MFORWARDING, &IPMFORWARDING, 0, 1),
    SysctlBoundedArgs::new(IPCTL_ARPTIMEOUT, &ARPT_KEEP, 0, INT_MAX),
    SysctlBoundedArgs::new(IPCTL_ARPDOWN, &ARPT_DOWN, 0, INT_MAX),
];

/// `ipintrq`.
pub static IPINTRQ: Niqueue = Niqueue::new(IPQ_MAXLEN as u32, NETISR_IP);

/// `ipqent_pool`: the fragments ("ipqe").
pub static IPQENT_POOL: BoxPool<Ipqent> = BoxPool::new();
/// `ipq_pool`: the datagrams being reassembled ("ipq").
pub static IPQ_POOL: BoxPool<Ipq> = BoxPool::new();

/// `ipsend_mq`.
static IPSEND_MQ: MbufQueue = MbufQueue::new(64, IPL_SOFTNET);
/// `ipsendraw_mq`.
static IPSENDRAW_MQ: MbufQueue = MbufQueue::new(64, IPL_SOFTNET);

/// `ipsend_task`.
static IPSEND_TASK: Task = Task::new(ip_send_dispatch, core::ptr::null_mut());
/// `ipsendraw_task`.
static IPSENDRAW_TASK: Task = Task::new(ip_sendraw_dispatch, core::ptr::null_mut());

/// `ipv4_input` of the glue `ip_init` ran for, as `if_input_proto` calls it (D-9: the
/// receive entry point is `G`-erased; `ether_input` and `if_input_local` read it).
pub static IPV4_INPUT: OnceStatic<crate::net::if_::ProtoFn> = OnceStatic::new();

/// `inetctlerrmap[]`: the errno of each `PRC_*` control command.
pub static INETCTLERRMAP: [Option<Errno>; PRC_NCMDS] = [
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

/// `ip_init`: IP initialization: fill in IP protocol switch table (the glue's `inetsw[]`). All
/// protocols not implemented in kernel go to raw IP protocol handler. A second call (a
/// test's) leaves the pools and the published entry point as they are.
pub fn ip_init<G: ProcGlue>() {
    ip_randomid_init();

    // ipcounters = counters_alloc(ips_ncounters): kernel::netinet::ip_var::IPCOUNTERS.

    let _ = IPQENT_POOL.init(IPL_SOFTNET, 0, "ipqe");
    let _ = IPQ_POOL.init(IPL_SOFTNET, 0, "ipq");

    inetsw_init::<G>();
    // LIST_INIT(&ipq): the queue is made empty with its mutex.

    // Fill in list of ports not to allocate dynamically.
    for m in [&BADDYNAMICPORTS.tcp, &BADDYNAMICPORTS.udp] {
        for w in m {
            w.store(0, Ordering::Relaxed);
        }
    }
    for &p in DEFBADDYNAMICPORTS_TCP {
        dp_set(&BADDYNAMICPORTS.tcp, p);
    }
    for &p in DEFBADDYNAMICPORTS_UDP {
        dp_set(&BADDYNAMICPORTS.udp, p);
    }

    // Fill in list of ports only root can bind to.
    for m in [&ROOTONLYPORTS.tcp, &ROOTONLYPORTS.udp] {
        for w in m {
            w.store(0, Ordering::Relaxed);
        }
    }
    for &p in DEFROOTONLYPORTS_TCP {
        dp_set(&ROOTONLYPORTS.tcp, p);
    }
    for &p in DEFROOTONLYPORTS_UDP {
        dp_set(&ROOTONLYPORTS.udp, p);
    }

    // mq_init(&ipsend_mq, 64, IPL_SOFTNET) and ipsendraw_mq's: their initialisers.

    // NETHER > 0
    arpinit::<G>();
    // IPSEC
    if let Some(i) = ipsec() {
        i.init();
    }
    // MROUTING: mrt_init(); not configured.

    let _ = IPV4_INPUT.set(ipv4_input::<G>);
}

/// `ip_ours(mp, offp, nxt, af, ns)`: enqueue packet for local delivery. Queuing is used as a
/// boundary between the network layer (input/forward path) running with `NET_LOCK_SHARED()`
/// and the transport layer needing it exclusively. `ip` is the packet's header as its caller
/// read it (the C's `mtod(m, struct ip *)`, read once per packet: see `ip_input_if`).
#[allow(
    clippy::too_many_arguments,
    reason = "the C's arguments, the header read once and the net lock's proof"
)]
pub fn ip_ours<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    _nxt: i32,
    af: i32,
    ip: &Ip,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let nxt = ip_fragcheck(mp, offp, ip);
    if nxt == IPPROTO_DONE {
        return IPPROTO_DONE;
    }

    // We are already in a IPv4/IPv6 local deliver loop.
    if af != i32::from(AF_UNSPEC) {
        return nxt;
    }

    let nxt = ip_deliver::<G>(mp, offp, nxt, i32::from(AF_INET), true, ns, net);
    if nxt == IPPROTO_DONE {
        return IPPROTO_DONE;
    }

    ip_ours_enqueue::<G>(mp, offp, nxt)
}

/// `ip_ours_enqueue(mp, offp, nxt)`: the packet on `ipintrq` for `ipintr`, with its offset and
/// next protocol in a tag when the header has options.
pub fn ip_ours_enqueue<G: ProcGlue>(mp: &mut Option<MbufBox>, offp: &mut i32, nxt: i32) -> i32 {
    let Some(mut m) = mp.take() else {
        return IPPROTO_DONE;
    };

    // save values for later, use after dequeue
    if *offp != IP_HDR_LEN as i32 {
        // mbuf tags are expensive, but only used for header options
        let Some(mut mtag) =
            m_tag_get(PACKET_TAG_IP_OFFNXT, size_of::<Ipoffnxt>() as i32, M_NOWAIT)
        else {
            ipstat_inc(IpstatCounters::IpsIdropped);
            return IPPROTO_DONE;
        };
        let ion = Ipoffnxt {
            ion_off: *offp,
            ion_nxt: nxt,
        };
        let _ = pod::put(mtag.data_mut(), 0, &ion);
        m_tag_prepend(&mut m, mtag);
    }

    let _ = niq_enqueue::<G>(&IPINTRQ, m);
    IPPROTO_DONE
}

/// `ipintr()`: dequeue and process locally delivered packets. This is called with exclusive
/// `NET_LOCK()` (`net`, the netisr's).
pub fn ipintr<G: ProcGlue>(net: &NetHeld<'_>) {
    while let Some(mut m) = niq_dequeue(&IPINTRQ) {
        // DIAGNOSTIC: "ipintr no HDR".
        kassert!(m.flags() & M_PKTHDR != 0);
        let (mut off, nxt);
        if let Some(mtag) = m_tag_find(&m, PACKET_TAG_IP_OFFNXT, None) {
            let ion = pod::at::<Ipoffnxt>(&mtag.bytes(), 0).unwrap_or_default();
            off = ion.ion_off;
            nxt = ion.ion_nxt;

            let t = core::ptr::from_ref(mtag);
            m_tag_delete(&mut m, t);
        } else {
            let ip = ip_load(&m);
            off = i32::from(ip.ip_hl()) << 2;
            nxt = i32::from(ip.ip_p);
        }

        let mut mp = Some(m);
        let nxt = ip_deliver::<G>(&mut mp, &mut off, nxt, i32::from(AF_INET), false, None, net);
        kassert!(nxt == IPPROTO_DONE);
    }
}

/// `ipv4_input(ifp, m, ns)`: IPv4 input routine. Checksum and byte swap header. Process
/// options. Forward or deliver.
pub fn ipv4_input<G: ProcGlue>(
    ifp: &'static Ifnet,
    m: MbufBox,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) {
    let mut off = 0;
    let mut mp = Some(m);
    let nxt = ip_input_if::<G>(
        &mut mp,
        &mut off,
        IPPROTO_IPV4,
        i32::from(AF_UNSPEC),
        ifp,
        ns,
        net,
    );
    kassert!(nxt == IPPROTO_DONE);
}

/// `ipv4_check(ifp, m)`: the header checks of `ipv4_input`; the packet trimmed to `ip_len`
/// with its header as read for the checks (the one copy of it the input path makes: the C
/// reads it in place, `ip_input_if` passes this one down), or `None` (freed) when it is bad.
pub fn ipv4_check(ifp: &Ifnet, m: MbufBox) -> Option<(MbufBox, Ip)> {
    let mut m = m;

    if (m.m_len().get() as usize) < IP_HDR_LEN {
        let Some(mm) = m_pullup(m, IP_HDR_LEN as i32) else {
            ipstat_inc(IpstatCounters::IpsToosmall);
            return None;
        };
        m = mm;
    }

    let ip = ip_load(&m);
    if ip.ip_v() != IPVERSION {
        ipstat_inc(IpstatCounters::IpsBadvers);
        return None;
    }

    let hlen = usize::from(ip.ip_hl()) << 2;
    if hlen < IP_HDR_LEN {
        // minimum header length
        ipstat_inc(IpstatCounters::IpsBadhlen);
        return None;
    }
    if hlen > m.m_len().get() as usize {
        let Some(mm) = m_pullup(m, hlen as i32) else {
            ipstat_inc(IpstatCounters::IpsBadhlen);
            return None;
        };
        m = mm;
    }

    // 127/8 must not appear on wire - RFC1122
    if ((u32::from_be(ip.ip_dst.s_addr) >> IN_CLASSA_NSHIFT) == IN_LOOPBACKNET
        || (u32::from_be(ip.ip_src.s_addr) >> IN_CLASSA_NSHIFT) == IN_LOOPBACKNET)
        && ifp.if_flags.get() & IFF_LOOPBACK == 0
    {
        ipstat_inc(IpstatCounters::IpsBadaddr);
        return None;
    }

    let csum = &m.m_pkthdr().csum_flags;
    if csum.get() & M_IPV4_CSUM_IN_OK == 0 {
        if csum.get() & M_IPV4_CSUM_IN_BAD != 0 {
            ipstat_inc(IpstatCounters::IpsBadsum);
            return None;
        }

        ipstat_inc(IpstatCounters::IpsInswcsum);
        if in_cksum(&m, hlen as i32) != 0 {
            ipstat_inc(IpstatCounters::IpsBadsum);
            return None;
        }

        csum.set(csum.get() | M_IPV4_CSUM_IN_OK);
    }

    // Retrieve the packet length.
    let len = usize::from(u16::from_be(ip.ip_len));

    // Convert fields to host representation.
    if len < hlen {
        ipstat_inc(IpstatCounters::IpsBadlen);
        return None;
    }

    // Check that the amount of data in the buffers is at least as much as the IP header would
    // have us expect. Trim mbufs if longer than we expect. Drop packet if shorter than we
    // expect.
    let pktlen = m.m_pkthdr().len.get() as usize;
    if pktlen < len {
        ipstat_inc(IpstatCounters::IpsTooshort);
        return None;
    }
    if pktlen > len {
        if m.m_len().get() as usize == pktlen {
            m.m_len().set(len as u32);
            m.m_pkthdr().len.set(len as i32);
        } else {
            m_adj(&m, len as i32 - pktlen as i32);
        }
    }

    // The header made writable: the input path writes it (and the options, and ICMP's reply)
    // in place, as the C does through `mtod`. A shared cluster (a copy's) gets one of its own
    // here, once; without one the packet is dropped, counted as lost for want of a buffer
    // (`ips_idropped`). Every later write lands in this first mbuf (ICMP's `m_pullup` keeps
    // it or makes a fresh one), so they are made in place.
    if m.is_readonly() {
        let mut b = [0u8; 60];
        m_copydata(&m, 0, &mut b[..hlen]);
        if m_copyback(&mut m, 0, &b[..hlen], M_DONTWAIT).is_err() {
            ipstat_inc(IpstatCounters::IpsIdropped);
            return None;
        }
    }

    // The header: `m_pullup` moved its bytes, never changed them, and trimming the tail
    // leaves them as they were.
    Some((m, ip))
}

/// The route cache `ip_input_if` uses: the softnet's (`ns->ns_route`), or the call's own
/// (`iproute`, given back when it ends).
fn route_of<'a>(ns: &'a mut Option<&mut Netstack>, iproute: &'a mut Route) -> &'a mut Route {
    match ns {
        Some(n) => &mut n.ns_route,
        None => iproute,
    }
}

/// `ip_input_if(mp, offp, nxt, af, ifp, ns)`: the IPv4 input of a packet received on `ifp`.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's arguments and the net lock's proof"
)]
pub fn ip_input_if<G: ProcGlue>(
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

    kassert!(*offp == 0);

    ipstat_inc(IpstatCounters::IpsTotal);
    'bad: {
        let Some(m0) = mp.take() else {
            break 'bad;
        };
        // The header is read once (the C reads it in place, `mtod(m, struct ip *)`): here, by
        // the checks, and again only where the packet may have changed under it: after pf,
        // which may rewrite or replace the packet, and after the options, whose source
        // routes rewrite `ip_dst`.
        let Some((m, mut ip)) = ipv4_check(ifp, m0) else {
            break 'bad;
        };

        // NCARP > 0: carp_lsdrop; not configured.

        // Packet filter.
        let odst = ip.ip_dst;
        let mut m = match pf() {
            Some(p) => {
                let (action, m) = p.test(AF_INET, PF_IN, ifp, m);
                if action != PF_PASS {
                    drop(m);
                    break 'bad;
                }
                let Some(m) = m else {
                    break 'bad;
                };
                ip = ip_load(&m);
                m
            }
            None => m,
        };

        if odst.s_addr != ip.ip_dst.s_addr {
            flags |= IP_REDIRECT;
        }

        match ip_forwarding.load(Ordering::Relaxed) {
            2 => flags |= IP_FORWARDING_IPSEC | IP_FORWARDING,
            1 => flags |= IP_FORWARDING,
            _ => {}
        }
        if IP_DIRECTEDBCAST.load(Ordering::Relaxed) != 0 {
            flags |= IP_ALLOWBROADCAST;
        }

        let hlen = usize::from(ip.ip_hl()) << 2;

        // Process options and, if not destined for us, ship it on. ip_dooptions answers None
        // when an error was detected (causing an icmp message to be sent and the original
        // packet to be freed) or the packet was forwarded.
        if hlen > IP_HDR_LEN {
            match ip_dooptions(m, ifp, flags, net) {
                Some(mm) => {
                    m = mm;
                    ip = ip_load(&m);
                }
                None => break 'bad,
            }
        }

        let ours = in_ouraddr(&m, &ip, ifp, route_of(&mut ns, &mut iproute), flags, net);
        match ours {
            2 => break 'bad,
            1 => {
                *mp = Some(m);
                return ip_ours::<G>(mp, offp, nxt, af, &ip, ns, net);
            }
            _ => {}
        }

        if in_multicast(ip.ip_dst.s_addr) {
            // Make sure M_MCAST is set. It should theoretically already be there, but let's
            // play safe because upper layers check for this flag.
            m.m_flags().set(m.flags() | M_MCAST);

            // MROUTING: multicast forwarding through ip_mforward when ip_mrouter_active (the
            // IGMP packets delivered whatever the membership); not configured.

            // See if we belong to the destination multicast group on the arrival interface.
            if !in_hasmulti(&ip.ip_dst, ifp) {
                ipstat_inc(IpstatCounters::IpsNotmember);
                if !in_local_group(ip.ip_dst.s_addr) {
                    ipstat_inc(IpstatCounters::IpsCantforward);
                }
                break 'bad;
            }
            *mp = Some(m);
            return ip_ours::<G>(mp, offp, nxt, af, &ip, ns, net);
        }

        // NCARP > 0: carp_lsdrop for ICMP; not configured.

        // Not for us; forward if possible and desirable.
        if flags & IP_FORWARDING == 0 {
            ipstat_inc(IpstatCounters::IpsCantforward);
            break 'bad;
        }
        if let Some(i) = ipsec()
            && i.in_use()
            && i.forward_check(&m, hlen as i32, i32::from(AF_INET))
        {
            ipstat_inc(IpstatCounters::IpsCantforward);
            break 'bad;
        }
        // Fall through, forward packet. Outbound IPsec policy checking will occur in
        // ip_output().

        ip_forward(m, ifp, Some(route_of(&mut ns, &mut iproute)), flags, net);
        return IPPROTO_DONE;
    }
    // bad: the packet, if any is left, freed; iproute's route given back with it.
    drop(mp.take());
    IPPROTO_DONE
}

/// `ip_fragcheck(mp, offp)`: reassembles a fragment (the datagram, once complete, replaces
/// `*mp`); answers the next protocol with `*offp` at its header, or `IPPROTO_DONE`. `ip0` is
/// `*mp`'s header as its caller read it.
pub fn ip_fragcheck(mp: &mut Option<MbufBox>, offp: &mut i32, ip0: &Ip) -> i32 {
    if mp.is_none() {
        return IPPROTO_DONE;
    }
    let mut ip = *ip0;
    let mut hlen = usize::from(ip.ip_hl()) << 2;

    // If offset or more fragments are set, must reassemble. Otherwise, nothing need be done.
    // (We could look in the reassembly queue to see if the packet was previously fragmented,
    // but it's not worth the time; just let them time out.)
    if ip.ip_off & (IP_OFFMASK | IP_MF).to_be() != 0 {
        let Some(mut m) = mp.take() else {
            return IPPROTO_DONE;
        };
        if m.flags() & M_EXT != 0 {
            // XXX
            let Some(mm) = m_pullup(m, hlen as i32) else {
                ipstat_inc(IpstatCounters::IpsToosmall);
                return IPPROTO_DONE;
            };
            m = mm;
            ip = ip_load(&m);
        }

        // Adjust ip_len to not reflect header, set ipqe_mff if more fragments are expected,
        // convert offset of this to bytes.
        ip.ip_len = u16::from_be(ip.ip_len).wrapping_sub(hlen as u16).to_be();
        let mff = ip.ip_off & IP_MF.to_be() != 0;
        if mff {
            // Make sure that fragments have a data length that's a non-zero multiple of 8
            // bytes.
            let len = u16::from_be(ip.ip_len);
            if len == 0 || len & 0x7 != 0 {
                ipstat_inc(IpstatCounters::IpsBadfrags);
                return IPPROTO_DONE;
            }
        }
        ip.ip_off = (u16::from_be(ip.ip_off) << 3).to_be();
        if !ip_store(&mut m, &ip) {
            // No buffer for a shared cluster's copy (not taken: the header is writable since
            // ipv4_check): a memory failure, counted as the C's in ip_fragcheck.
            ipstat_inc(IpstatCounters::IpsRcvmemdrop);
            return IPPROTO_DONE;
        }

        let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());
        let out = {
            let mut g = IPQ.lock();
            ip_fragcheck_locked(&mut g, m, &ip, hlen, mff, rdomain)
        };
        match out {
            Frag::Done => return IPPROTO_DONE,
            Frag::Whole(m) => *mp = Some(m),
            Frag::Reassembled(mut m) => {
                ipstat_inc(IpstatCounters::IpsReassembled);
                ip = ip_load(&m);
                hlen = usize::from(ip.ip_hl()) << 2;
                ip.ip_len = u16::from_be(ip.ip_len).wrapping_add(hlen as u16).to_be();
                if !ip_store(&mut m, &ip) {
                    ipstat_inc(IpstatCounters::IpsRcvmemdrop);
                    return IPPROTO_DONE;
                }
                *mp = Some(m);
            }
        }
    }

    *offp = hlen as i32;
    i32::from(ip.ip_p)
}

/// What `ip_fragcheck`'s section under `ipq_mutex` did with a fragment.
enum Frag {
    /// It was queued or dropped (`*mp` is NULL).
    Done,
    /// It needed no reassembly (neither an offset nor more fragments): it goes on as it is.
    Whole(MbufBox),
    /// It completed its datagram, which goes on.
    Reassembled(MbufBox),
}

/// `ip_fragcheck`'s section under `ipq_mutex` (`st`, its guard's): the fragment `m` (header
/// `ip`, `hlen` bytes, more fragments after it if `mff`) of the datagram of `rdomain` queued
/// on its datagram's queue or dropped.
fn ip_fragcheck_locked(
    st: &mut IpqState,
    m: MbufBox,
    ip: &Ip,
    hlen: usize,
    mff: bool,
    rdomain: u32,
) -> Frag {
    let off = u16::from_be(ip.ip_off);
    let matches = |fp: &Ipq| {
        ip.ip_id == fp.ipq_id
            && ip.ip_src.s_addr == fp.ipq_src.s_addr
            && ip.ip_dst.s_addr == fp.ipq_dst.s_addr
            && ip.ip_p == fp.ipq_p
            && rdomain == fp.ipq_rdomain
    };

    // If datagram marked as having more fragments or if this is not the first fragment,
    // attempt reassembly; if it succeeds, proceed. (The C makes these checks after the walk
    // below, which changes nothing.)
    let mut whole = None;
    let mut ipqe = None;
    if mff || off != 0 {
        let ip_maxqueue_local = IP_MAXQUEUE.load(Ordering::Relaxed);

        ipstat_inc(IpstatCounters::IpsFragments);
        if st.frags + 1 > ip_maxqueue_local {
            ip_flush(st, ip_maxqueue_local);
            ipstat_inc(IpstatCounters::IpsRcvmemdrop);
            return Frag::Done;
        }

        let len = u16::from_be(ip.ip_len);
        let mut m = Some(m);
        let e = IPQENT_POOL.get(PR_NOWAIT, || {
            let Some(m) = m.take() else {
                kpanic(format_args!("ip_fragcheck: no fragment"));
            };
            Ipqent::new(m, mff, hlen as u8, off, len, ip.ip_tos)
        });
        let Some(e) = e else {
            ipstat_inc(IpstatCounters::IpsRcvmemdrop);
            return Frag::Done;
        };
        st.frags += 1;
        ipqe = Some(e);
    } else {
        whole = Some(m);
    }

    // Look for queue of fragments of this datagram: LIST_FOREACH, the cursor kept on the
    // predecessor of the one it finds, so the queue comes off the list and goes back in O(1).
    let IpqState { q, frags } = st;
    let at_head = q.first().is_some_and(matches);
    let mut c = q.cursor_mut();
    let after = !at_head
        && loop {
            let Some(cur) = c.current() else {
                break false;
            };
            match IpqList::link(cur).get() {
                None => break false,
                Some(n) if matches(n) => break true,
                Some(_) => {
                    c.move_next();
                }
            }
        };

    let Some(ipqe) = ipqe else {
        if after {
            if let Some(fp) = c.remove_next() {
                ip_freef(frags, fp);
            }
        } else if at_head && let Some(fp) = q.pop_head() {
            ip_freef(frags, fp);
        }
        return match whole {
            Some(m) => Frag::Whole(m),
            None => Frag::Done,
        };
    };

    if after {
        let fp = c.remove_next();
        let (datagram, back) = ip_reass(frags, ipqe, fp, ip, rdomain);
        if let Some(fp) = back {
            // Back after its predecessor: LIST_INSERT_AFTER of the C's untouched place.
            let mut rest = c.split_after();
            q.push_tail(fp);
            q.concat(&mut rest);
        }
        return datagram.map_or(Frag::Done, Frag::Reassembled);
    }
    let fp = if at_head { q.pop_head() } else { None };
    let (datagram, back) = ip_reass(frags, ipqe, fp, ip, rdomain);
    if let Some(fp) = back {
        // The head's place again, or a new queue's (LIST_INSERT_HEAD).
        q.push_head(fp);
    }
    datagram.map_or(Frag::Done, Frag::Reassembled)
}

/// `IPSTAT_INC(name)` of `ip_deliver`: the IPv4 counter `v4`, or the IPv6 one `v6` (the `Inet`
/// bridge's, IPv6 being `bsd`'s until N6.5) when `af` is not `AF_INET`.
#[inline]
fn ipstat_af(af: i32, v4: IpstatCounters, v6: Ip6Deliver) {
    if af == i32::from(AF_INET) {
        ipstat_inc(v4);
    } else if let Some(i) = inet() {
        i.ip6stat_deliver(v6);
    }
}

/// `ip_deliver(mp, offp, nxt, af, shared, ns)`: hands the packet to the protocols, walking the
/// protocol switch until one consumes it. With `shared` (the shared net lock is held), a
/// protocol that needs the exclusive lock gets the packet through `ipintrq` (`ip6intrq` for
/// IPv6) instead.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's arguments and the net lock's proof"
)]
pub fn ip_deliver<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    nxt: i32,
    af: i32,
    shared: bool,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let mut ns = ns;
    let mut nxt = nxt;
    let mut af = af;
    let mut nest = 0;

    // Tell launch routine the next header
    ipstat_af(af, IpstatCounters::IpsDelivered, Ip6Deliver::Delivered);

    'bad: {
        while nxt != IPPROTO_DONE {
            let psw = if af == i32::from(AF_INET) {
                ip_protocol::<G>(nxt)
            } else if af == i32::from(AF_INET6) {
                inet6_protocol::<G>(nxt)
            } else {
                None
            };
            let Some(psw) = psw else {
                kpanic(format_args!("ip_deliver: unknown af {af}"));
            };
            if shared && psw.flags() & PR_MPINPUT == 0 {
                // delivery not finished, decrement counter, queue
                if af == i32::from(AF_INET) {
                    ipstat_dec(IpstatCounters::IpsDelivered);
                    return ip_ours_enqueue::<G>(mp, offp, nxt);
                }
                if let Some(i) = inet() {
                    i.ip6stat_deliver(Ip6Deliver::Undelivered);
                    if let Some(m) = mp.take() {
                        i.ip6_ours_enqueue(m, *offp, nxt);
                    }
                }
                return IPPROTO_DONE;
            }

            if af == i32::from(AF_INET6) {
                nest += 1;
                if nest > inet().map_or(0, crate::net::if_::Inet::ip6_hdrnestlimit) {
                    if let Some(i) = inet() {
                        i.ip6stat_deliver(Ip6Deliver::Toomanyhdr);
                    }
                    break 'bad;
                }
            }

            // protection against faulty packet - there should be more sanity checks in header
            // chain processing.
            let Some(m) = mp.as_ref() else {
                return IPPROTO_DONE;
            };
            if m.m_pkthdr().len.get() < *offp {
                ipstat_af(af, IpstatCounters::IpsTooshort, Ip6Deliver::Tooshort);
                break 'bad;
            }

            if let Some(i) = ipsec()
                && i.in_use()
                && i.local_check(m, *offp, nxt, af)
            {
                ipstat_af(af, IpstatCounters::IpsCantforward, Ip6Deliver::Cantforward);
                break 'bad;
            }
            // Otherwise, just fall through and deliver the packet

            let naf = match nxt {
                IPPROTO_IPV4 => {
                    ipstat_inc(IpstatCounters::IpsDelivered);
                    i32::from(AF_INET)
                }
                IPPROTO_IPV6 => {
                    if let Some(i) = inet() {
                        i.ip6stat_deliver(Ip6Deliver::Delivered);
                    }
                    i32::from(AF_INET6)
                }
                _ => af,
            };
            nxt = psw.input(mp, offp, nxt, af, ns.as_deref_mut(), net);
            af = naf;
        }
        return nxt;
    }
    // bad:
    drop(mp.take());
    IPPROTO_DONE
}

/// `in_ouraddr(m, ifp, ro, flags)`: 1 if the packet is for one of our addresses (or a
/// broadcast we accept), 2 if it was received on the wrong interface, 0 otherwise. `ip` is
/// `m`'s header as its caller read it.
pub fn in_ouraddr(
    m: &Mbuf,
    ip: &Ip,
    ifp: &Ifnet,
    ro: &mut Route,
    flags: i32,
    net: &NetHeld<'_>,
) -> i32 {
    let mut match_ = 0;

    // NPF > 0
    match pf().map_or(-1, |p| p.ouraddr(m)) {
        0 => return 0,
        1 => return 1,
        _ => {} // pf does not know it
    }

    if ip.ip_dst.s_addr == INADDR_BROADCAST || ip.ip_dst.s_addr == INADDR_ANY {
        m.m_flags().set(m.flags() | M_BCAST);
        return 1;
    }

    let rtableid = m.m_pkthdr().ph_rtableid.get();
    let rt = ro.mpath4(
        ip.ip_dst.s_addr.to_ne_bytes(),
        Some(ip.ip_src.s_addr.to_ne_bytes()),
        rtableid,
        net,
    );
    if let Some(rt) = rt {
        if rt.rt_flags.isset(RTF_LOCAL) {
            match_ = 1;
        }

        // If directedbcast is enabled we only consider it local if it is received on the
        // interface with that address.
        if rt.rt_flags.isset(RTF_BROADCAST)
            && (flags & IP_ALLOWBROADCAST == 0 || rt.rt_ifidx.get() == ifp.if_index.get())
        {
            match_ = 1;

            // Make sure M_BCAST is set
            m.m_flags().set(m.flags() | M_BCAST);
        }
    }

    if match_ == 0 {
        // No local address or broadcast address found, so check for ancient classful
        // broadcast addresses. It must have been broadcast on the link layer, and for an
        // address on the interface it was received on.
        if m.flags() & M_BCAST == 0 || !in_classfulbroadcast(ip.ip_dst.s_addr, ip.ip_dst.s_addr) {
            return 0;
        }

        if ifp.if_rdomain.get() != rtable_l2(rtableid) {
            return 0;
        }
        // The check in the loop assumes you only rx a packet on an UP interface, and that
        // M_BCAST will only be set on a BROADCAST interface. (NET_ASSERT_LOCKED: `net`.)
        let _ = net;
        if ifp
            .if_addrlist
            .iter()
            .filter_map(InIfaddr::of)
            .any(|ia| in_classfulbroadcast(ip.ip_dst.s_addr, ia.ia_addr.addr().s_addr))
        {
            match_ = 1;
        }
    } else if let Some(rt) = rt
        && flags & IP_FORWARDING == 0
        && rt.rt_ifidx.get() != ifp.if_index.get()
        && !(ifp.if_flags.get() & IFF_LOOPBACK != 0
            || ifp.if_type.get() == IFT_ENC
            || m.m_pkthdr().pf.flags.get() & PF_TAG_TRANSLATE_LOCALHOST != 0)
    {
        // received on wrong interface.
        // NCARP > 0: carp_strict_addr_chk of the outgoing interface; not configured.
        ipstat_inc(IpstatCounters::IpsWrongif);
        match_ = 2;
    }

    match_
}

/// `ip_reass(ipqe, fp, rdomain)`: takes incoming datagram fragment `ipqe` and tries to
/// reassemble it into a whole datagram. If a chain for reassembly of this datagram already
/// exists, it is given as `fp` (off the queue); otherwise one is made. Answers the datagram
/// when complete, and the queue to put back in its place when it is not done (`ip` is the
/// fragment's header, for a new queue). `ipq_mutex` held: `frags` is its `ip_frags`.
fn ip_reass(
    frags: &mut i32,
    ipqe: PoolBox<Ipqent>,
    fp: Option<PoolBox<Ipq>>,
    ip: &Ip,
    rdomain: u32,
) -> (Option<MbufBox>, Option<PoolBox<Ipq>>) {
    let hlen = usize::from(ipqe.ipqe_hlen);

    // Presence of header sizes in mbufs would confuse code below.
    if let Some(m) = ipqe.ipqe_m.as_ref() {
        m.data_add(hlen as isize);
        m.m_len().set(m.m_len().get() - hlen as u32);
    }

    // dropfrag: the fragment and its entry freed.
    let dropfrag = |frags: &mut i32, ipqe: PoolBox<Ipqent>| {
        ipstat_inc(IpstatCounters::IpsFragdropped);
        drop(ipqe);
        *frags -= 1;
    };

    let Some(mut fp) = fp else {
        // If first fragment to arrive, create a reassembly queue.
        let Some(mut fp) = IPQ_POOL.get(PR_NOWAIT, || {
            Ipq::new(rdomain, ip.ip_id, ip.ip_p, ip.ip_src, ip.ip_dst, IPFRAGTTL)
        }) else {
            dropfrag(frags, ipqe);
            return (None, None);
        };
        // goto insert
        fp.ipq_fragq.push_head(ipqe);
        return reass_done(frags, fp);
    };

    // Handle ECN by comparing this segment with the first one; if CE is set, do not lose CE.
    // drop if CE and not-ECT are mixed for the same packet.
    let Some(first) = fp.ipq_fragq.first() else {
        kpanic(format_args!("ip_reass: an empty queue"));
    };
    let ecn = ipqe.ipqe_tos.get() & IPTOS_ECN_MASK;
    let ecn0 = first.ipqe_tos.get() & IPTOS_ECN_MASK;
    if ecn == IPTOS_ECN_CE {
        if ecn0 == IPTOS_ECN_NOTECT {
            dropfrag(frags, ipqe);
            return (None, Some(fp));
        }
        if ecn0 != IPTOS_ECN_CE {
            first.ipqe_tos.set(first.ipqe_tos.get() | IPTOS_ECN_CE);
        }
    }
    if ecn == IPTOS_ECN_NOTECT && ecn0 != IPTOS_ECN_NOTECT {
        dropfrag(frags, ipqe);
        return (None, Some(fp));
    }

    let ioff = i32::from(ipqe.ipqe_off.get());
    let fragq = &mut fp.ipq_fragq;
    // Find a segment which begins after this one does: `p` the last one that does not (the
    // cursor's element), `q` the one after it.
    let head_after = fragq
        .first()
        .is_none_or(|q| i32::from(q.ipqe_off.get()) > ioff);
    if head_after {
        // No preceding segment: trim or dequeue the ones this one covers from the head.
        while let Some(q) = fragq.first() {
            if ipqe.end() <= i32::from(q.ipqe_off.get()) {
                break;
            }
            let i = ipqe.end() - i32::from(q.ipqe_off.get());
            if i < i32::from(q.ipqe_len.get()) {
                q.ipqe_len.set(q.ipqe_len.get().wrapping_sub(i as u16));
                q.ipqe_off.set(q.ipqe_off.get().wrapping_add(i as u16));
                if let Some(m) = q.ipqe_m.as_ref() {
                    m_adj(m, i);
                }
                break;
            }
            drop(fragq.pop_head());
            *frags -= 1;
        }
        fragq.push_head(ipqe);
    } else {
        let mut c = fragq.cursor_mut();
        while let Some(cur) = c.current() {
            match IpqentList::link(cur).get() {
                Some(q) if i32::from(q.ipqe_off.get()) <= ioff => {
                    c.move_next();
                }
                _ => break,
            }
        }

        // If there is a preceding segment, it may provide some of our data already. If so,
        // drop the data from the incoming segment. If it provides all of our data, drop us.
        if let Some(p) = c.current() {
            let i = p.end() - ioff;
            if i > 0 {
                if i >= i32::from(ipqe.ipqe_len.get()) {
                    dropfrag(frags, ipqe);
                    return (None, Some(fp));
                }
                if let Some(m) = ipqe.ipqe_m.as_ref() {
                    m_adj(m, i);
                }
                ipqe.ipqe_off
                    .set(ipqe.ipqe_off.get().wrapping_add(i as u16));
                ipqe.ipqe_len
                    .set(ipqe.ipqe_len.get().wrapping_sub(i as u16));
            }
        }

        // While we overlap succeeding segments trim them or, if they are completely
        // covered, dequeue them.
        while let Some(q) = c.current().and_then(|p| IpqentList::link(p).get()) {
            if ipqe.end() <= i32::from(q.ipqe_off.get()) {
                break;
            }
            let i = ipqe.end() - i32::from(q.ipqe_off.get());
            if i < i32::from(q.ipqe_len.get()) {
                q.ipqe_len.set(q.ipqe_len.get().wrapping_sub(i as u16));
                q.ipqe_off.set(q.ipqe_off.get().wrapping_add(i as u16));
                if let Some(m) = q.ipqe_m.as_ref() {
                    m_adj(m, i);
                }
                break;
            }
            drop(c.remove_next());
            *frags -= 1;
        }

        // insert: stick new segment in its place (after p).
        let mut rest = c.split_after();
        fragq.push_tail(ipqe);
        fragq.concat(&mut rest);
    }

    reass_done(frags, fp)
}

/// `ip_reass`'s end, the new fragment in its place: the datagram when it is complete (its
/// queue then dropped, the C's `LIST_REMOVE` and `pool_put`), or the queue to put back; a
/// queue emptied by a bogus message size (`ip_freef`) is dropped too.
fn reass_done(frags: &mut i32, fp: PoolBox<Ipq>) -> (Option<MbufBox>, Option<PoolBox<Ipq>>) {
    let mut fp = fp;
    let datagram = reass_check(frags, &mut fp);
    if datagram.is_some() || fp.ipq_fragq.is_empty() {
        drop(fp);
        return (datagram, None);
    }
    (None, Some(fp))
}

/// `ip_reass`'s `insert:` tail: whether the fragments of `fp` are complete, and if so the
/// datagram made of them (the queue left empty, its fragments' entries freed); a bogus
/// message size frees them all (`ip_freef`, the queue left empty for the caller to drop).
fn reass_check(frags: &mut i32, fp: &mut PoolBox<Ipq>) -> Option<MbufBox> {
    let mut next: i32 = 0;
    let mut last: Option<&Ipqent> = None;
    for q in &fp.ipq_fragq {
        if i32::from(q.ipqe_off.get()) != next {
            return None;
        }
        next += i32::from(q.ipqe_len.get());
        last = Some(q);
    }
    if last.is_none_or(|l| l.ipqe_mff) {
        return None;
    }

    // Reassembly is complete. Check for a bogus message size and concatenate fragments.
    let first = fp.ipq_fragq.first()?;
    let hl = usize::from(first.ipqe_hlen);
    if next as usize + hl > IP_MAXPACKET {
        ipstat_inc(IpstatCounters::IpsToolong);
        *frags -= fp.ipq_fragq.len() as i32;
        while let Some(q) = fp.ipq_fragq.pop_head() {
            drop(q);
        }
        return None;
    }
    let mut q = fp.ipq_fragq.pop_head()?;
    let tos = q.ipqe_tos.get();
    let mut m = q.ipqe_m.take()?;
    drop(q);
    *frags -= 1;
    let t = m.m_next.take();
    m_cat(&mut m, t);
    while let Some(mut q) = fp.ipq_fragq.pop_head() {
        let t = q.ipqe_m.take();
        drop(q);
        *frags -= 1;
        let t = t.map(|mut t| {
            m_removehdr(&mut t);
            t
        });
        m_cat(&mut m, t);
    }

    // Create header for new ip packet by modifying header of first packet; dequeue and
    // discard fragment reassembly header. Make header visible.
    m.m_len().set(m.m_len().get() + hl as u32);
    m.data_add(-(hl as isize));
    let mut ip = ip_load(&m);
    ip.ip_tos = tos;
    ip.ip_len = (next as u16).to_be();
    ip.ip_src = fp.ipq_src;
    ip.ip_dst = fp.ipq_dst;
    if !ip_store(&mut m, &ip) {
        // The first fragment's header, writable since ipv4_check: not taken; a memory
        // failure, counted as the C's in ip_fragcheck.
        ipstat_inc(IpstatCounters::IpsRcvmemdrop);
        return None;
    }
    m_calchdrlen(&m);
    Some(m)
}

/// `ip_freef(fp)`: frees a fragment reassembly header and all associated datagrams (`fp`
/// off the queue; `frags` the guard's `ip_frags`).
fn ip_freef(frags: &mut i32, fp: PoolBox<Ipq>) {
    let mut fp = fp;
    while let Some(q) = fp.ipq_fragq.pop_head() {
        drop(q);
        *frags -= 1;
    }
    drop(fp);
}

/// `ip_slowtimo()`: IP timer processing; if a timer expires on a reassembly queue, discard
/// it.
pub fn ip_slowtimo() {
    let mut g = IPQ.lock();
    let IpqState { q, frags } = &mut *g;
    // The head first, then the cursor on each one's predecessor (LIST_FOREACH_SAFE).
    loop {
        let Some(fp) = q.first() else {
            return;
        };
        let ttl = fp.ipq_ttl.get().wrapping_sub(1);
        fp.ipq_ttl.set(ttl);
        if ttl != 0 {
            break;
        }
        ipstat_inc(IpstatCounters::IpsFragtimeout);
        if let Some(fp) = q.pop_head() {
            ip_freef(frags, fp);
        }
    }
    let mut c = q.cursor_mut();
    while let Some(fp) = c.current().and_then(|p| IpqList::link(p).get()) {
        let ttl = fp.ipq_ttl.get().wrapping_sub(1);
        fp.ipq_ttl.set(ttl);
        if ttl == 0 {
            ipstat_inc(IpstatCounters::IpsFragtimeout);
            if let Some(fp) = c.remove_next() {
                ip_freef(frags, fp);
            }
        } else {
            c.move_next();
        }
    }
}

/// `ip_flush(maxqueue)`: flush a bunch of datagram fragments, till we are down to 75%.
fn ip_flush(st: &mut IpqState, maxqueue: i32) {
    let mut max = 50;

    let IpqState { q, frags } = st;
    while !q.is_empty() && *frags > maxqueue * 3 / 4 && {
        max -= 1;
        max != 0
    } {
        ipstat_inc(IpstatCounters::IpsFragdropped);
        if let Some(fp) = q.pop_head() {
            ip_freef(frags, fp);
        }
    }
}

/// The `sockaddr_in` of `a`, as bytes (ICMP's too).
#[inline]
pub(crate) fn sin_bytes(a: InAddr) -> [u8; size_of::<SockaddrIn>()] {
    let sin = SockaddrIn {
        sin_len: size_of::<SockaddrIn>() as u8,
        sin_family: AF_INET,
        sin_addr: a,
        ..SockaddrIn::default()
    };
    let mut b = [0u8; size_of::<SockaddrIn>()];
    let _ = pod::put(&mut b, 0, &sin);
    b
}

/// The address of the Internet record of `ifa` (`ifatoia(ifa)->ia_addr.sin_addr`).
#[inline]
fn ia_addr_of(ifa: Option<&'static crate::net::if_var::Ifaddr>) -> InAddr {
    ifa.and_then(InIfaddr::of)
        .map_or(InAddr { s_addr: 0 }, |ia| ia.ia_addr.addr())
}

/// The four bytes of an address at `off` of the options.
#[inline]
fn opt_addr(opts: &[u8], off: usize) -> InAddr {
    let mut a = [0u8; 4];
    if let Some(b) = opts.get(off..off + 4) {
        a.copy_from_slice(b);
    }
    InAddr {
        s_addr: u32::from_ne_bytes(a),
    }
}

/// `memcpy(cp + off, &a, 4)`.
#[inline]
fn opt_put(opts: &mut [u8], off: usize, a: u32) {
    if let Some(b) = opts.get_mut(off..off + 4) {
        b.copy_from_slice(&a.to_ne_bytes());
    }
}

/// `bytes` written at `off` of `m`'s data, in place when the data is writable, through
/// `m_copyback` (a copy of a shared cluster first) when it is not; `false`, nothing written,
/// when no cluster can be had for the copy (the caller drops the packet).
#[must_use]
pub(crate) fn pkt_put(m: &mut Mbuf, off: usize, bytes: &[u8]) -> bool {
    if let Some(d) = m.data_mut()
        && let Some(dst) = d.get_mut(off..off + bytes.len())
    {
        dst.copy_from_slice(bytes);
        return true;
    }
    m_copyback(m, off as i32, bytes, M_DONTWAIT).is_ok()
}

/// What `ip_dooptions`' walk found wrong: the ICMP error's type and code (the parameter
/// problem's pointer).
struct BadOpt {
    type_: u8,
    code: i32,
}

/// `ip_dooptions(m, ifp, flags)`: do option processing on a datagram, possibly discarding it
/// if bad options are encountered, or forwarding it if source-routed. Answers the packet when
/// it should be processed further, `None` when it has been forwarded or freed.
pub fn ip_dooptions(
    m: MbufBox,
    ifp: &'static Ifnet,
    flags: i32,
    net: &NetHeld<'_>,
) -> Option<MbufBox> {
    let mut m = m;
    let mut ip = ip_load(&m);
    let rtableid = m.m_pkthdr().ph_rtableid.get();
    let mut forward = false;

    let dst = ip.ip_dst;
    let cnt0 = (usize::from(ip.ip_hl()) << 2) - IP_HDR_LEN;
    // `ipv4_check` made the whole header (with its options) contiguous in the first mbuf.
    let mut opts = [0u8; MAX_IPOPTLEN];
    let Some(b) = m.data().get(IP_HDR_LEN..IP_HDR_LEN + cnt0) else {
        kpanic(format_args!("ip_dooptions: {cnt0} bytes of options"));
    };
    opts[..cnt0].copy_from_slice(b);
    let opts = &mut opts[..cnt0];
    let mut changed = false;

    hal::klock::kernel_lock();
    let res = dooptions_walk(
        opts,
        &mut ip,
        &mut m,
        dst,
        ifp,
        rtableid,
        &mut forward,
        &mut changed,
        net,
    );
    hal::klock::kernel_unlock();
    if changed && !(ip_store(&mut m, &ip) && pkt_put(&mut m, IP_HDR_LEN, opts)) {
        // The header and its options, writable since ipv4_check: not taken.
        ipstat_inc(IpstatCounters::IpsIdropped);
        return None;
    }
    if let Err(bad) = res {
        icmp_error(m, bad.type_, bad.code as u8, 0, 0, net);
        ipstat_inc(IpstatCounters::IpsBadoptions);
        return None;
    }
    if forward && flags & IP_FORWARDING != 0 {
        ip_forward(m, ifp, None, flags | IP_REDIRECT, net);
        return None;
    }
    Some(m)
}

/// `ip_dooptions`' walk over the options `opts` (the header's, copied) of the packet `m`
/// (header `ip`), under the kernel lock: `Err` with the ICMP error to send for a bad option.
/// The bounds are the C's, each one where the C checks it; `code` is the offset of the bad
/// byte in the header (`&cp[..] - (u_char *)ip`).
#[allow(
    clippy::too_many_arguments,
    reason = "the walk's state, shared with the caller"
)]
fn dooptions_walk(
    opts: &mut [u8],
    ip: &mut Ip,
    m: &mut Mbuf,
    dst: InAddr,
    ifp: &'static Ifnet,
    rtableid: u32,
    forward: &mut bool,
    changed: &mut bool,
    net: &NetHeld<'_>,
) -> Result<(), BadOpt> {
    let hdr = IP_HDR_LEN as i32;
    let param = |code: usize| BadOpt {
        type_: ICMP_PARAMPROB,
        code: hdr + code as i32,
    };
    let unreach = |code: u8| BadOpt {
        type_: ICMP_UNREACH,
        code: i32::from(code),
    };
    let mut cp = 0usize;
    let mut cnt = opts.len() as i32;
    'opts: while cnt > 0 {
        let opt = opts[cp + IPOPT_OPTVAL];
        if opt == IPOPT_EOL {
            break;
        }
        let optlen: i32;
        if opt == IPOPT_NOP {
            optlen = 1;
        } else {
            if cnt < (IPOPT_OLEN + 1) as i32 {
                return Err(param(cp + IPOPT_OLEN));
            }
            optlen = i32::from(opts[cp + IPOPT_OLEN]);
            if optlen < (IPOPT_OLEN + 1) as i32 || optlen > cnt {
                return Err(param(cp + IPOPT_OLEN));
            }
        }
        // `cp .. cp + optlen` lies within the options from here on.
        let o = cp;

        match opt {
            // Source routing with record. Find interface with current destination address. If
            // none on this machine then drop if strictly routed, or do nothing if loosely
            // routed. Record interface address and bring up next address component. If
            // strictly routed make sure next address is on directly accessible net.
            IPOPT_LSRR | IPOPT_SSRR => 'sr: {
                if IP_DOSOURCEROUTE.load(Ordering::Relaxed) == 0 {
                    return Err(unreach(ICMP_UNREACH_SRCFAIL));
                }
                if optlen < (IPOPT_OFFSET + 1) as i32 {
                    return Err(param(o + IPOPT_OLEN));
                }
                let mut off = i32::from(opts[o + IPOPT_OFFSET]);
                if off < i32::from(IPOPT_MINOFF) {
                    return Err(param(o + IPOPT_OFFSET));
                }
                let ipaddr = sin_bytes(ip.ip_dst);
                if ifa_ifwithaddr(net, &ipaddr, rtableid).is_none() {
                    if opt == IPOPT_SSRR {
                        return Err(unreach(ICMP_UNREACH_SRCFAIL));
                    }
                    // Loose routing, and not at next destination yet; nothing to do except
                    // forward.
                    break 'sr;
                }
                off -= 1; // 0 origin
                if off + size_of::<InAddr>() as i32 > optlen {
                    // End of source route. Should be for us.
                    save_rte(m, &opts[o..o + optlen as usize], ip.ip_src);
                    break 'sr;
                }

                // locate outgoing interface
                let off = off as usize;
                let next = opt_addr(opts, o + off);
                let ipaddr = sin_bytes(next);
                // keep packet in the virtual instance
                let rt = rtalloc(
                    SockaddrView::new(&ipaddr)
                        .unwrap_or_else(|| kpanic(format_args!("ip_dooptions"))),
                    RT_RESOLVE,
                    rtableid,
                    net,
                );
                let Some(r) = rt.as_deref().filter(|r| {
                    rtisvalid(Some(r)) && !(opt == IPOPT_SSRR && r.rt_flags.isset(RTF_GATEWAY))
                }) else {
                    return Err(unreach(ICMP_UNREACH_SRCFAIL));
                };
                let a = ia_addr_of(r.ifa());
                drop(rt);
                opt_put(opts, o + off, a.s_addr);
                opts[o + IPOPT_OFFSET] =
                    opts[o + IPOPT_OFFSET].wrapping_add(size_of::<InAddr>() as u8);
                ip.ip_dst = next;
                *changed = true;
                // Let ip_intr's mcast routing check handle mcast pkts
                *forward = !in_multicast(ip.ip_dst.s_addr);
            }

            IPOPT_RR => 'rr: {
                if optlen < (IPOPT_OFFSET + 1) as i32 {
                    return Err(param(o + IPOPT_OLEN));
                }
                let off = i32::from(opts[o + IPOPT_OFFSET]);
                if off < i32::from(IPOPT_MINOFF) {
                    return Err(param(o + IPOPT_OFFSET));
                }

                // If no space remains, ignore.
                let off = off - 1; // 0 origin
                if off + size_of::<InAddr>() as i32 > optlen {
                    break 'rr;
                }
                let ipaddr = sin_bytes(ip.ip_dst);
                // locate outgoing interface; if we're the destination, use the incoming
                // interface (should be same). Again keep the packet inside the virtual
                // instance.
                let rt = rtalloc(
                    SockaddrView::new(&ipaddr)
                        .unwrap_or_else(|| kpanic(format_args!("ip_dooptions"))),
                    RT_RESOLVE,
                    rtableid,
                    net,
                );
                let Some(r) = rt.as_deref().filter(|r| rtisvalid(Some(r))) else {
                    return Err(unreach(ICMP_UNREACH_HOST));
                };
                let a = ia_addr_of(r.ifa());
                drop(rt);
                opt_put(opts, o + off as usize, a.s_addr);
                opts[o + IPOPT_OFFSET] =
                    opts[o + IPOPT_OFFSET].wrapping_add(size_of::<InAddr>() as u8);
                *changed = true;
            }

            IPOPT_TS => 'ts: {
                let code = param(o);
                if (optlen as usize) < size_of::<IpTimestamp>() {
                    return Err(code);
                }
                let Some(mut ipt) = pod::at::<IpTimestamp>(&opts[o..], 0) else {
                    return Err(code);
                };
                if ipt.ipt_ptr < 5 || ipt.ipt_len < 5 {
                    return Err(code);
                }
                let ptr = i32::from(ipt.ipt_ptr);
                let tlen = i32::from(ipt.ipt_len);
                if ptr - 1 + size_of::<u32>() as i32 > tlen {
                    // The overflow count of the local copy, as the C's (not written back).
                    let oflw = (ipt.ipt_oflw() + 1) & 0x0f;
                    ipt.set_ipt_oflw(oflw);
                    if oflw == 0 {
                        return Err(code);
                    }
                    break 'ts;
                }
                // `ipt_len` is the option's own length byte, at most `optlen`: the stamp's slot
                // `ptr - 1 .. ptr + 3` lies within the option.
                let sin = opt_addr(opts, o + (ptr - 1) as usize);
                match ipt.ipt_flg() {
                    IPOPT_TS_TSONLY => {}

                    IPOPT_TS_TSANDADDR => {
                        if ptr - 1 + (size_of::<u32>() + size_of::<InAddr>()) as i32 > tlen {
                            return Err(code);
                        }
                        let ipaddr = sin_bytes(dst);
                        if ifaof_ifpforaddr(net, &ipaddr, ifp).is_none() {
                            // continue: the next option.
                            cnt -= optlen;
                            cp += optlen as usize;
                            continue 'opts;
                        }
                        ipt.ipt_ptr = ipt.ipt_ptr.wrapping_add(size_of::<InAddr>() as u8);
                    }

                    IPOPT_TS_PRESPEC => {
                        if ptr - 1 + (size_of::<u32>() + size_of::<InAddr>()) as i32 > tlen {
                            return Err(code);
                        }
                        let ipaddr = sin_bytes(sin);
                        if ifa_ifwithaddr(net, &ipaddr, rtableid).is_none() {
                            cnt -= optlen;
                            cp += optlen as usize;
                            continue 'opts;
                        }
                        ipt.ipt_ptr = ipt.ipt_ptr.wrapping_add(size_of::<InAddr>() as u8);
                    }

                    _ => {
                        return Err(param(o + IPOPT_OFFSET + 1));
                    }
                }
                let ntime = iptime();
                // The C stores the time at the (local) pointer and moves it on, without
                // writing the pointer back into the option.
                opt_put(opts, o + usize::from(ipt.ipt_ptr) - 1, ntime);
                *changed = true;
            }

            _ => {}
        }
        cnt -= optlen;
        cp += optlen as usize;
    }
    Ok(())
}

/// `save_rte(m, option, dst)`: saves incoming source route `option` for use in replies, to be
/// picked up later by `ip_srcroute` if the receiver is interested.
fn save_rte(m: &mut Mbuf, option: &[u8], dst: InAddr) {
    let olen = usize::from(option[IPOPT_OLEN]);
    if olen > (IPOPT_OFFSET + 1) + MAX_IPOPTLEN / size_of::<InAddr>() * size_of::<InAddr>() {
        return;
    }

    let Some(mut mtag) = m_tag_get(PACKET_TAG_SRCROUTE, size_of::<IpSrcrt>() as i32, M_NOWAIT)
    else {
        ipstat_inc(IpstatCounters::IpsIdropped);
        return;
    };
    let mut isr = pod::at::<IpSrcrt>(mtag.data_mut().as_slice(), 0).unwrap_or_default();
    isr.isr_nhops = ((olen - IPOPT_OFFSET - 1) / size_of::<InAddr>()) as i32;
    isr.isr_dst = dst;
    let d = mtag.data_mut();
    let _ = pod::put(d, 0, &isr);
    // memcpy(isr->isr_hdr, option, olen): the header, then on into isr_routes.
    d[ISR_HDR..ISR_HDR + olen].copy_from_slice(&option[..olen]);
    m_tag_prepend(m, mtag);
}

/// `ip_srcroute(m0)`: retrieves the incoming source route for use in replies, in the same form
/// used by setsockopt. The first hop is placed before the options, will be removed later.
pub fn ip_srcroute(m0: &mut Mbuf) -> Option<MbufBox> {
    // sizeof(isr_nop) + sizeof(isr_hdr)
    const OPTSIZ: usize = 1 + IPOPT_OFFSET + 1;

    if IP_DOSOURCEROUTE.load(Ordering::Relaxed) == 0 {
        return None;
    }

    let mtag = m_tag_find(m0, PACKET_TAG_SRCROUTE, None)?;
    let mut isr = pod::at::<IpSrcrt>(&mtag.bytes(), 0)?;
    let t = core::ptr::from_ref(mtag);

    if isr.isr_nhops == 0 {
        return None;
    }
    let Some(mut m) = m_get(M_DONTWAIT, MT_SOOPTS) else {
        ipstat_inc(IpstatCounters::IpsIdropped);
        return None;
    };

    // length is (nhops+1)*sizeof(addr) + sizeof(nop + header)
    let nhops = isr.isr_nhops as usize;
    let len = (nhops + 1) * size_of::<InAddr>() + OPTSIZ;
    m.m_len().set(len as u32);

    let mut out = [0u8; (MAX_IPOPTLEN / 4 + 1) * 4 + OPTSIZ];
    let route = |i: usize| match isr.isr_routes.get(i) {
        Some(r) => r.s_addr,
        None => kpanic(format_args!("ip_srcroute: {nhops} hops")),
    };
    // First save first hop for return route
    out[..4].copy_from_slice(&route(nhops - 1).to_ne_bytes());

    // Copy option fields and padding (nop) to mbuf.
    isr.isr_nop = IPOPT_NOP;
    isr.isr_hdr[IPOPT_OFFSET] = IPOPT_MINOFF;
    out[4] = isr.isr_nop;
    out[5..4 + OPTSIZ].copy_from_slice(&isr.isr_hdr);
    // Record return path as an IP source route, reversing the path (pointers are now
    // aligned).
    let mut q = 4 + OPTSIZ;
    for p in (0..nhops - 1).rev() {
        out[q..q + 4].copy_from_slice(&route(p).to_ne_bytes());
        q += 4;
    }
    // Last hop goes to final destination.
    out[q..q + 4].copy_from_slice(&isr.isr_dst.s_addr.to_ne_bytes());
    // Invariant: `m` is m_get's, its data in its own storage (no cluster), which is never
    // read-only (`Mbuf::is_readonly` answers so only for a cluster): `data_mut` is `Some`.
    let Some(d) = m.data_mut() else {
        kpanic(format_args!(
            "ip_srcroute: an mbuf without a cluster read-only"
        ));
    };
    d.copy_from_slice(&out[..len]);
    m_tag_delete(m0, t);
    Some(m)
}

/// `ip_stripoptions(m)`: strips out IP options, at higher level protocol in the kernel.
/// `false`, nothing changed, when `m`'s data is read-only (a shared cluster: the input path
/// made its first mbuf writable in `ipv4_check`, so not taken): the caller drops it.
#[must_use]
pub fn ip_stripoptions(m: &mut Mbuf) -> bool {
    let mut ip = ip_load(m);
    let olen = (usize::from(ip.ip_hl()) << 2) - IP_HDR_LEN;
    let mlen = m.m_len().get() as usize;
    let i = mlen - (IP_HDR_LEN + olen);
    let Some(d) = m.data_mut() else {
        return false;
    };
    d.copy_within(IP_HDR_LEN + olen..IP_HDR_LEN + olen + i, IP_HDR_LEN);
    m.m_len().set((mlen - olen) as u32);
    if m.flags() & M_PKTHDR != 0 {
        let ph = m.m_pkthdr();
        ph.len.set(ph.len.get() - olen as i32);
    }
    ip.set_ip_hl((IP_HDR_LEN >> 2) as u8);
    ip.ip_len = u16::from_be(ip.ip_len).wrapping_sub(olen as u16).to_be();
    // The data was writable above: the header is written in place.
    ip_store(m, &ip)
}

/// `ip_forward(m, ifp, ro, flags)`: forwards a packet. If some error occurs return the sender
/// an icmp packet. Note we can't always generate a meaningful icmp message because icmp
/// doesn't have a large enough repertoire of codes and types. If not forwarding, just drop the
/// packet. This could be confusing if ip_forwarding was zero but some routing protocol was
/// advancing us as a gateway to somewhere. However, we must let the routing protocol deal with
/// that.
pub fn ip_forward(m: MbufBox, ifp: &Ifnet, ro: Option<&mut Route>, flags: i32, net: &NetHeld<'_>) {
    let mut m = m;
    let mut ip = ip_load(&m);
    let mut iproute = Route::new();
    let rtableid = m.m_pkthdr().ph_rtableid.get();
    let loopcnt = m.m_pkthdr().ph_loopcnt.get();
    let mut icmp_buf = [0u8; 68];
    let mut type_ = 0u8;
    let mut code = 0u8;
    let mut destmtu = 0i32;
    let mut dest = 0u32;

    if m.flags() & (M_BCAST | M_MCAST) != 0
        || !in_canforward(ip.ip_dst)
        || ip.ip_src.s_addr == INADDR_ANY
    {
        ipstat_inc(IpstatCounters::IpsCantforward);
        return;
    }
    if ip.ip_ttl <= IPTTLDEC {
        icmp_error(m, ICMP_TIMXCEED, ICMP_TIMXCEED_INTRANS, dest, 0, net);
        return;
    }

    let ro: &mut Route = match ro {
        Some(r) => r,
        None => &mut iproute,
    };
    let redirect = {
        let Some(rt) = ro.mpath4(
            ip.ip_dst.s_addr.to_ne_bytes(),
            Some(ip.ip_src.s_addr.to_ne_bytes()),
            rtableid,
            net,
        ) else {
            ipstat_inc(IpstatCounters::IpsNoroute);
            icmp_error(m, ICMP_UNREACH, ICMP_UNREACH_HOST, dest, 0, net);
            return;
        };

        // Save at most 68 bytes of the packet in case we need to generate an ICMP message to
        // the src. The data is saved on the stack. A new mbuf is only allocated when ICMP is
        // actually created.
        let icmp_len = icmp_buf.len().min(usize::from(u16::from_be(ip.ip_len)));
        let mflags = m.flags();
        let pfflags = m.m_pkthdr().pf.flags.get();
        m_copydata(&m, 0, &mut icmp_buf[..icmp_len]);

        ip.ip_ttl -= IPTTLDEC;

        // If forwarding packet using same interface that it came in on, perhaps should send a
        // redirect to sender to shortcut a hop. Only send redirect if source is sending
        // directly to us, and if packet was not source routed (or has any options). Also,
        // don't send redirect if forwarding using a default route or a route modified by a
        // redirect. Don't send redirect if we advertise destination's arp address as ours
        // (proxy arp).
        let key = pod::at::<SockaddrIn>(rt.dest(), 0).map_or(InAddr { s_addr: 0 }, |s| s.sin_addr);
        if rt.rt_ifidx.get() == ifp.if_index.get()
            && !rt.rt_flags.isset(RTF_DYNAMIC | RTF_MODIFIED)
            && key.s_addr != INADDR_ANY
            && flags & IP_REDIRECT == 0
            // NETHER > 0
            && !arpproxy(key, rtableid, net)
            && IP_SENDREDIRECTS.load(Ordering::Relaxed) != 0
        {
            let ia = rt.ifa().and_then(InIfaddr::of);
            if ia.is_some_and(|ia| ip.ip_src.s_addr & ia.ia_netmask.get() == ia.ia_net.get()) {
                if rt.rt_flags.isset(RTF_GATEWAY) {
                    dest = rt
                        .with_gateway(|g| g.and_then(|g| pod::at::<SockaddrIn>(g, 0)))
                        .map_or(0, |s| s.sin_addr.s_addr);
                } else {
                    dest = ip.ip_dst.s_addr;
                }
                // Router requirements says to only send host redirects
                type_ = ICMP_REDIRECT;
                code = ICMP_REDIRECT_HOST;
            }
        }
        (icmp_len, mflags, pfflags)
    };
    let (icmp_len, mflags, pfflags) = redirect;
    if !ip_store(&mut m, &ip) {
        // The header, writable since ipv4_check: not taken. Counted as the C counts a
        // forwarding its ip_output could not make.
        ipstat_inc(IpstatCounters::IpsCantforward);
        return;
    }

    let error = ip_output(
        m,
        None,
        Some(&mut *ro),
        flags | IP_FORWARDING,
        None,
        None,
        0,
        net,
    );
    if error.is_err() {
        ipstat_inc(IpstatCounters::IpsCantforward);
    } else {
        ipstat_inc(IpstatCounters::IpsForward);
        if type_ != 0 {
            ipstat_inc(IpstatCounters::IpsRedirectsent);
        } else {
            return;
        }
    }
    match error {
        Ok(()) => {
            // forwarded, but need redirect: type, code set above
        }

        Err(Errno::EMSGSIZE) => {
            type_ = ICMP_UNREACH;
            code = ICMP_UNREACH_NEEDFRAG;
            if let Some(rt) = ro.rt() {
                let rtmtu = rt.rt_mtu().load(Ordering::Relaxed);
                if rtmtu != 0 {
                    destmtu = rtmtu as i32;
                } else {
                    let destifp = if_get(rt.rt_ifidx.get());
                    if let Some(d) = destifp {
                        destmtu = d.if_mtu.get() as i32;
                    }
                    if_put(destifp);
                }
            }
            ipstat_inc(IpstatCounters::IpsCantfrag);
            if destmtu == 0 {
                return;
            }
        }

        // EACCES: pf(4) blocked the packet. There is no need to send an ICMP packet back since
        // pf(4) takes care of it.
        // ENOBUFS: a router should not generate ICMP_SOURCEQUENCH as required in RFC1812
        // Requirements for IP Version 4 Routers. source quench could be a big problem under
        // DoS attacks, or the underlying interface is rate-limited.
        Err(Errno::EACCES | Errno::ENOBUFS) => return,

        // ENETUNREACH (shouldn't happen, checked above), EHOSTUNREACH, ENETDOWN, EHOSTDOWN,
        // default:
        Err(_) => {
            type_ = ICMP_UNREACH;
            code = ICMP_UNREACH_HOST;
        }
    }

    let Some(mut mcopy) = m_gethdr(M_DONTWAIT, MT_DATA) else {
        return;
    };
    mcopy.m_len().set(icmp_len as u32);
    let ph = mcopy.m_pkthdr();
    ph.len.set(icmp_len as i32);
    ph.ph_rtableid.set(rtableid);
    ph.ph_ifidx.set(ifp.if_index.get());
    ph.ph_loopcnt.set(loopcnt);
    ph.pf
        .flags
        .set(ph.pf.flags.get() | (pfflags & PF_TAG_GENERATED));
    mcopy.m_flags().set(mcopy.flags() | (mflags & M_COPYFLAGS));
    // A fresh mbuf: written in place (without it, no error is sent, as without the mbuf).
    if !pkt_put(&mut mcopy, 0, &icmp_buf[..icmp_len]) {
        return;
    }
    icmp_error(mcopy, type_, code, dest, destmtu, net);
    // done: iproute's route given back with it (rtfree).
}

/// `ip_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the `net.inet.ip` sysctls.
pub fn ip_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // Almost all sysctl names at this level are terminal.
    let Some(&first) = name.first() else {
        return Err(Errno::ENOTDIR);
    };
    if name.len() != 1 && first != IPCTL_IFQUEUE && first != IPCTL_ARPQUEUE {
        return Err(Errno::ENOTDIR);
    }

    match first {
        IPCTL_SOURCEROUTE => securelevel_int(req, &IP_DOSOURCEROUTE),
        IPCTL_MTUDISC => {
            let oldval = ip_mtudisc.load(Ordering::Relaxed);
            let newval = AtomicI32::new(oldval);
            let error = int_bounded(req, &newval, 0, 1);
            let newval = newval.into_inner();
            if error.is_ok()
                && oldval != newval
                && ip_mtudisc
                    .compare_exchange(oldval, newval, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                && newval == 0
            {
                let w = NETLOCK.write();
                if let Some(net) = NetHeld::of(&w) {
                    rt_timer_queue_flush(&IP_MTUDISC_TIMEOUT_Q, &net);
                }
            }

            error
        }
        IPCTL_MTUDISCTIMEOUT => {
            let oldval = IP_MTUDISC_TIMEOUT.load(Ordering::Relaxed);
            let newval = AtomicI32::new(oldval);
            let error = int_bounded(req, &newval, 0, INT_MAX);
            let newval = newval.into_inner();
            if error.is_ok() && oldval != newval {
                let sysctl_lock = SYSCTL_LOCK.write();
                IP_MTUDISC_TIMEOUT.store(newval, Ordering::Relaxed);
                rt_timer_queue_change(&IP_MTUDISC_TIMEOUT_Q, newval);
                drop(sysctl_lock);
            }

            error
        }
        // IPSEC
        IPCTL_ENCDEBUG
        | IPCTL_IPSEC_STATS
        | IPCTL_IPSEC_EXPIRE_ACQUIRE
        | IPCTL_IPSEC_EMBRYONIC_SA_TIMEOUT
        | IPCTL_IPSEC_REQUIRE_PFS
        | IPCTL_IPSEC_SOFT_ALLOCATIONS
        | IPCTL_IPSEC_ALLOCATIONS
        | IPCTL_IPSEC_SOFT_BYTES
        | IPCTL_IPSEC_BYTES
        | IPCTL_IPSEC_TIMEOUT
        | IPCTL_IPSEC_SOFT_TIMEOUT
        | IPCTL_IPSEC_SOFT_FIRSTUSE
        | IPCTL_IPSEC_FIRSTUSE
        | IPCTL_IPSEC_ENC_ALGORITHM
        | IPCTL_IPSEC_AUTH_ALGORITHM
        | IPCTL_IPSEC_IPCOMP_ALGORITHM => match ipsec() {
            Some(i) => i.sysctl(name, req),
            // The C without IPSEC: no such case, ipctl_vars has none of them.
            None => bounded_arr(&IPCTL_VARS, name, req),
        },
        IPCTL_IFQUEUE => sysctl_niq(&name[1..], req, &IPINTRQ),
        IPCTL_ARPQUEUE => sysctl_niq(&name[1..], req, &ARPINQ),
        IPCTL_ARPQUEUED => rdint(req, la_hold_total.load(Ordering::Relaxed) as i32),
        IPCTL_STATS => ip_sysctl_ipstat(req),
        // !MROUTING:
        IPCTL_MRTPROTO | IPCTL_MRTSTATS | IPCTL_MRTMFC | IPCTL_MRTVIF => Err(Errno::EOPNOTSUPP),
        IPCTL_MULTIPATH => {
            let oldval = IPMULTIPATH.load(Ordering::Relaxed);
            let newval = AtomicI32::new(oldval);
            let error = int_bounded(req, &newval, 0, 1);
            let newval = newval.into_inner();
            if error.is_ok() && oldval != newval {
                IPMULTIPATH.store(newval, Ordering::Relaxed);
                core::sync::atomic::fence(Ordering::Release);
                RTGENERATION.fetch_add(1, Ordering::Relaxed);
            }

            error
        }
        _ => bounded_arr(&IPCTL_VARS, name, req),
    }
}

/// `ip_sysctl_ipstat(oldp, oldlenp, newp)`: `net.inet.ip.stats`, the counters as a `struct
/// ipstat`.
fn ip_sysctl_ipstat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    const _: () = assert!(size_of::<Ipstat>() == IPS_NCOUNTERS * size_of::<u64>());
    let mut bytes = [0u8; IPS_NCOUNTERS * size_of::<u64>()];
    for (i, c) in IPCOUNTERS.read().iter().enumerate() {
        bytes[i * 8..i * 8 + 8].copy_from_slice(&c.to_ne_bytes());
    }

    rdstruct(req, &bytes)
}

/// `ip_savecontrol(inp, mp, ip, m)`: the control messages the socket of `inp` asked for about
/// the received datagram `m` with header `ip`, as a chain (the C's `*mp`, which its callers
/// pass empty).
pub fn ip_savecontrol<G: ProcGlue>(inp: &Inpcb<G>, ip: &Ip, m: &Mbuf) -> Option<MbufBox> {
    let mut parts: [Option<MbufBox>; 5] = [None, None, None, None, None];

    if inp.socket().has_options(SO_TIMESTAMP) {
        let tv = m_microtime(m);
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&tv.tv_sec.to_ne_bytes());
        b[8..].copy_from_slice(&tv.tv_usec.to_ne_bytes());
        parts[0] = sbcreatecontrol(&b, SCM_TIMESTAMP, SOL_SOCKET);
    }

    if inp.has_flags(INP_RECVDSTADDR) {
        parts[1] = sbcreatecontrol(&ip.ip_dst.s_addr.to_ne_bytes(), IP_RECVDSTADDR, IPPROTO_IP);
    }
    // notyet: INP_RECVOPTS (the options were tossed already) and INP_RECVRETOPTS
    // (ip_srcroute doesn't do what we want here); compiled out in C too.
    if inp.has_flags(INP_RECVIF) {
        let ifp = if_get(m.m_pkthdr().ph_ifidx.get());
        let sadl = ifp.and_then(|i| {
            let s = i.sadl();
            s.as_ref().map(|sdl| {
                let b = hal::user::bytes_of(core::slice::from_ref(&**sdl));
                let mut out = [0u8; size_of::<hal::sys::net_if::SockaddrDl>()];
                let n = usize::from(sdl.sdl_len).min(out.len());
                out[..n].copy_from_slice(&b[..n]);
                (out, n)
            })
        });
        parts[2] = if let Some((out, n)) = sadl {
            sbcreatecontrol(&out[..n], IP_RECVIF, IPPROTO_IP)
        } else {
            let mut sdl = hal::sys::net_if::SockaddrDl {
                sdl_family: AF_LINK,
                sdl_index: ifp.map_or(0, |ifp| ifp.if_index.get() as u16),
                ..Default::default()
            };
            sdl.sdl_len = core::mem::offset_of!(hal::sys::net_if::SockaddrDl, sdl_data) as u8;
            let b = hal::user::bytes_of(core::slice::from_ref(&sdl));
            sbcreatecontrol(&b[..usize::from(sdl.sdl_len)], IP_RECVIF, IPPROTO_IP)
        };
        if_put(ifp);
    }
    if inp.has_flags(INP_RECVTTL) {
        parts[3] = sbcreatecontrol(&[ip.ip_ttl], IP_RECVTTL, IPPROTO_IP);
    }
    if inp.has_flags(INP_RECVRTABLE) {
        let mut rtableid: u32 = inp.inp_rtableid.get();

        // NPF > 0
        if m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED != 0 {
            let divert = pf().and_then(|p| p.find_divert(m));
            kassert!(divert.is_some());
            if let Some(d) = divert {
                rtableid = u32::from(d.rdomain);
            }
        }

        parts[4] = sbcreatecontrol(&rtableid.to_ne_bytes(), IP_RECVRTABLE, IPPROTO_IP);
    }

    // The chain, in the order made (`mp = &(*mp)->m_next` after each one made).
    let mut head: Option<MbufBox> = None;
    for c in parts.into_iter().rev().flatten() {
        let mut c = c;
        if let Some(h) = head.take() {
            drop(c.m_next.set(h));
        }
        head = Some(c);
    }
    head
}

/// `ip_send_do_dispatch(xmq, flags)`: sends the packets queued on `mq` with `ip_output`.
fn ip_send_do_dispatch(mq: &MbufQueue, flags: i32) {
    let mut ml = mq_delist(mq);
    if ml_empty(&ml) {
        return;
    }

    let shared = NETLOCK.read();
    let Some(net) = NetHeld::of(&shared) else {
        kpanic(format_args!("ip_send: the net lock"));
    };
    while let Some(mut m) = ml_dequeue(&mut ml) {
        let mut ipsecflowinfo = 0u32;

        if let Some(mtag) = m_tag_find(&m, PACKET_TAG_IPSEC_FLOWINFO, None) {
            let b = mtag.bytes();
            ipsecflowinfo = u32::from_ne_bytes([b[0], b[1], b[2], b[3]]);
            let t = core::ptr::from_ref(mtag);
            m_tag_delete(&mut m, t);
        }
        let _ = ip_output(m, None, None, flags, None, None, ipsecflowinfo, &net);
    }
}

/// `ip_sendraw_dispatch(xmq)`: `ipsendraw_task`.
fn ip_sendraw_dispatch(_xmq: *mut core::ffi::c_void) {
    ip_send_do_dispatch(&IPSENDRAW_MQ, IP_RAWOUTPUT);
}

/// `ip_send_dispatch(xmq)`: `ipsend_task`.
fn ip_send_dispatch(_xmq: *mut core::ffi::c_void) {
    ip_send_do_dispatch(&IPSEND_MQ, 0);
}

/// `ip_send(m)`: queues `m` for `ip_output` from the softnet task.
pub fn ip_send(m: MbufBox) {
    let _ = mq_enqueue(&IPSEND_MQ, m);
    if let Some(tq) = net_tq(0) {
        let _ = task_add(tq, core::pin::Pin::static_ref(&IPSEND_TASK));
    }
}

/// `ip_send_raw(m)`: queues `m` (a complete header) for `ip_output` from the softnet task.
pub fn ip_send_raw(m: MbufBox) {
    let _ = mq_enqueue(&IPSENDRAW_MQ, m);
    if let Some(tq) = net_tq(0) {
        let _ = task_add(tq, core::pin::Pin::static_ref(&IPSENDRAW_TASK));
    }
}

/// The host tests' `ipsend_task`, run by hand (no softnet thread runs it there).
#[cfg(any(test, feature = "doubles"))]
pub fn ip_send_test_run() {
    ip_send_do_dispatch(&IPSEND_MQ, 0);
}

/// The packets `ip_send` queued, taken (the host tests look at what ICMP answered).
#[cfg(any(test, feature = "doubles"))]
pub fn ip_send_test_take() -> crate::net::mbuf::MbufList {
    mq_delist(&IPSEND_MQ)
}

/// The reassembly queue and the send and input queues forgotten for the host tests (their
/// packets live in an earlier test's memory: dropping them would free into it), and the
/// counters' state left as it is.
#[cfg(any(test, feature = "doubles"))]
pub fn ipq_test_reset() {
    let mut g = IPQ.lock();
    core::mem::forget(core::mem::take(&mut g.q));
    g.frags = 0;
    drop(g);
    core::mem::forget(mq_delist(&IPSEND_MQ));
    core::mem::forget(mq_delist(&IPSENDRAW_MQ));
    core::mem::forget(crate::net::if_var::niq_delist(&IPINTRQ));
}

const _: () = {
    assert!(size_of::<IpSrcrt>() == 52);
    assert!(size_of::<IpSrcrt>() <= crate::net::mbuf::PACKET_TAG_MAXSIZE);
    assert!(size_of::<Ipoffnxt>() <= crate::net::mbuf::PACKET_TAG_MAXSIZE);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests for IPv4 input: `ipv4_check`'s rejections (one per bound of the C), the
    // reassembly (in order, reversed, overlapping, ECN mixing, the queue's bound, aging out),
    // `ip_dooptions`' bounds (one vector per check of the C, the parameter problem's pointer
    // or the unreachable code), the `PR_MPINPUT` split of `ip_deliver`, and `ip_sysctl`'s
    // names without a user buffer. The protocols are a test table (`SW`) over the tests' glue.

    use core::sync::atomic::AtomicU32;
    use std::sync::{MutexGuard, Once};
    use std::vec::Vec;

    use hal::netlock::NETLOCK;
    use hal::sys::netinet_in::{IPPROTO_ICMP, IPPROTO_RAW, IPPROTO_TCP, IPPROTO_UDP};
    use hal::sys::socket::{SOCK_DGRAM, SOCK_RAW, SOCK_STREAM};

    use super::*;
    use crate::fs::testfs::TestGlue;
    use crate::net::domain::Domain;
    use crate::net::mbuf::{m_devget, m_freem};
    use crate::net::proto::{PR_ADDR, PR_ATOMIC, Protocol, Usrreqs};

    /// The tests' glue.
    pub(crate) type G = TestGlue;

    /// A protocol of the test table: it counts the packets it is given and the control
    /// input, and frees the packets; the ICMP entry hands them to the kernel's ICMP.
    pub(crate) struct TestProto {
        proto: i16,
        ty: i16,
        flags: i16,
        icmp: bool,
        /// Packets given.
        pub(crate) inputs: AtomicU32,
        /// The last `pr_ctlinput` command (0: none).
        pub(crate) ctl: AtomicI32,
    }

    impl TestProto {
        const fn new(proto: i32, ty: i32, flags: i16, icmp: bool) -> Self {
            Self {
                proto: proto as i16,
                ty: ty as i16,
                flags,
                icmp,
                inputs: AtomicU32::new(0),
                ctl: AtomicI32::new(0),
            }
        }
    }

    /// The test domain: `AF_INET` with [`SW`].
    struct TestInet;

    /// IP's entry.
    pub(crate) static IP: TestProto = TestProto::new(0, 0, 0, false);
    /// A transport that runs under the shared net lock.
    pub(crate) static UDP: TestProto = TestProto::new(
        IPPROTO_UDP,
        SOCK_DGRAM,
        PR_ATOMIC | PR_ADDR | PR_MPINPUT,
        false,
    );
    /// One that needs it exclusively.
    pub(crate) static TCP: TestProto = TestProto::new(IPPROTO_TCP, SOCK_STREAM, 0, false);
    /// ICMP (the kernel's input).
    pub(crate) static ICMP: TestProto =
        TestProto::new(IPPROTO_ICMP, SOCK_RAW, PR_ATOMIC | PR_ADDR, true);
    /// The raw sockets.
    pub(crate) static RAW: TestProto = TestProto::new(
        IPPROTO_RAW,
        SOCK_RAW,
        PR_ATOMIC | PR_ADDR | PR_MPINPUT,
        false,
    );
    /// The test `inetsw[]`.
    pub(crate) static SW: [&dyn Protocol<G>; 5] = [&IP, &UDP, &TCP, &ICMP, &RAW];

    impl Domain<G> for TestInet {
        fn family(&self) -> i32 {
            i32::from(AF_INET)
        }
        fn name(&self) -> &'static str {
            "inet"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
            &SW
        }
    }

    impl Protocol<G> for TestProto {
        fn domain(&self) -> &'static dyn Domain<G> {
            &TestInet
        }
        fn ty(&self) -> i16 {
            self.ty
        }
        fn protocol(&self) -> i16 {
            self.proto
        }
        fn flags(&self) -> i16 {
            self.flags
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
            None
        }
        fn ctlinput(&self, cmd: i32, _sa: SockaddrView<'_>, _rdomain: u32, _ip: Option<&[u8]>) {
            self.ctl.store(cmd, Ordering::Relaxed);
        }
        fn input(
            &self,
            mp: &mut Option<MbufBox>,
            offp: &mut i32,
            proto: i32,
            af: i32,
            ns: Option<&mut Netstack>,
            net: &NetHeld<'_>,
        ) -> i32 {
            if self.icmp {
                return crate::netinet::ip_icmp::icmp_input::<G>(mp, offp, proto, af, ns, net);
            }
            self.inputs.fetch_add(1, Ordering::Relaxed);
            drop(mp.take());
            IPPROTO_DONE
        }
    }

    /// The routing tests' fixture with the test switch: `ip_init` and `icmp_init` over the
    /// tests' glue, the reassembly queue and the send queues forgotten.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        static IDS: Once = Once::new();
        let g = crate::net::route::tests::testroutes_setup();
        IDS.call_once(crate::netinet::ip_id::ip_randomid_init);
        ipq_test_reset();
        crate::netinet::ip_icmp::icmp_test_reset();
        crate::netinet::in_proto::inetsw_set::<G>(&SW);
        ip_init::<G>();
        crate::netinet::ip_icmp::icmp_init::<G>();
        g
    }

    /// The counter `c`.
    pub(crate) fn ips(c: IpstatCounters) -> u64 {
        IPCOUNTERS.read()[c as usize]
    }

    /// A header from 10.0.2.2 to 10.0.2.15 (`vhl`, total length `len`, `off` the C's
    /// `ip_off` word, protocol `p`, type of service `tos`), checksummed.
    pub(crate) fn header(vhl: u8, len: u16, id: u16, off: u16, p: u8, tos: u8) -> [u8; 20] {
        let mut h = [0u8; 20];
        h[0] = vhl;
        h[1] = tos;
        h[2..4].copy_from_slice(&len.to_be_bytes());
        h[4..6].copy_from_slice(&id.to_be_bytes());
        h[6..8].copy_from_slice(&off.to_be_bytes());
        h[8] = 64;
        h[9] = p;
        h[12..16].copy_from_slice(&[10, 0, 2, 2]);
        h[16..20].copy_from_slice(&[10, 0, 2, 15]);
        let sum = cksum(&h);
        h[10..12].copy_from_slice(&sum.to_be_bytes());
        h
    }

    /// The Internet checksum of `b` (an even number of bytes), as a host-order word.
    pub(crate) fn cksum(b: &[u8]) -> u16 {
        let mut s: u32 = b
            .chunks(2)
            .map(|c| u32::from(u16::from_be_bytes([c[0], c[1]])))
            .sum();
        while s >> 16 != 0 {
            s = (s & 0xffff) + (s >> 16);
        }
        !(s as u16)
    }

    /// The packet of `bytes`, its first `hlen` bytes in the first mbuf.
    pub(crate) fn packet(bytes: &[u8], hlen: usize) -> MbufBox {
        let m = m_devget(bytes, 0).expect("a packet");
        m_pullup(m, hlen.min(bytes.len()) as i32).expect("pullup")
    }

    /// The bytes of a packet.
    pub(crate) fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut v = std::vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(m, 0, &mut v);
        v
    }

    #[test]
    fn a_shared_cluster_gets_its_own_before_the_header_is_written() {
        // A packet in a cluster and a copy sharing it (m_copym's, as the broadcast copy
        // ether_resolve loops back): ipv4_check gives the copy a cluster of its own, so the
        // input path's in-place writes never reach the original.
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        let mut p = header(0x45, 1200, 1, 0, 17, 0).to_vec();
        p.resize(1200, 0xa5);
        let mut orig = m_devget(&p, 0).expect("a packet");
        let copy = crate::net::mbuf::m_copym(&mut orig, 0, crate::net::mbuf::M_COPYALL, M_DONTWAIT)
            .expect("a copy");
        assert!(copy.is_readonly(), "the copy shares the cluster");
        let (mut m, ip) = ipv4_check(ifp, copy).expect("accepted");
        assert!(!m.is_readonly(), "a cluster of its own");
        let mut ttl1 = ip;
        ttl1.ip_ttl = 1;
        assert!(ip_store(&mut m, &ttl1), "written in place");
        assert_eq!(
            (bytes(&m)[8], bytes(&orig)[8]),
            (1, 64),
            "the original untouched"
        );
        assert_eq!(bytes(&m)[20..], p[20..], "the rest of the packet kept");
    }

    #[test]
    fn ipv4_check_refuses_each_bad_header() {
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        let good = |len: u16| {
            let mut p = header(0x45, len, 1, 0, 17, 0).to_vec();
            p.resize(usize::from(len), 0xa5);
            p
        };
        // (what, the packet, the counter that counts its refusal)
        let mut badvers = good(28);
        badvers[0] = 0x65;
        let mut badhlen = good(28);
        badhlen[0] = 0x44;
        let mut longhdr = good(28);
        longhdr[0] = 0x4f;
        let mut loopback = good(28);
        loopback[12] = 127;
        let mut badsum = good(28);
        badsum[10] ^= 0xff;
        // hl 6 (an option word of NOPs), ip_len 20: below it; the checksum over the 24 bytes.
        let mut badlen = header(0x46, 20, 1, 0, 17, 0).to_vec();
        badlen.extend_from_slice(&[IPOPT_NOP; 4]);
        badlen[10..12].copy_from_slice(&[0, 0]);
        let sum = cksum(&badlen);
        badlen[10..12].copy_from_slice(&sum.to_be_bytes());
        badlen.resize(28, 1);
        let short = {
            let mut p = good(40);
            p.truncate(28);
            p
        };
        let cases: [(&str, Vec<u8>, IpstatCounters); 8] = [
            (
                "10 bytes",
                good(28)[..10].to_vec(),
                IpstatCounters::IpsToosmall,
            ),
            ("version 6", badvers, IpstatCounters::IpsBadvers),
            ("hl 4", badhlen, IpstatCounters::IpsBadhlen),
            ("hl 15 in 28 bytes", longhdr, IpstatCounters::IpsBadhlen),
            ("127/8 off loopback", loopback, IpstatCounters::IpsBadaddr),
            ("checksum", badsum, IpstatCounters::IpsBadsum),
            ("ip_len below hl", badlen, IpstatCounters::IpsBadlen),
            ("ip_len past the packet", short, IpstatCounters::IpsTooshort),
        ];
        for (what, p, c) in cases {
            let before = ips(c);
            let m = m_devget(&p, 0).expect("a packet");
            assert!(ipv4_check(ifp, m).is_none(), "{what}: refused");
            assert_eq!(ips(c), before + 1, "{what}: counted");
        }
        // A longer packet is trimmed to ip_len.
        let mut p = good(28);
        p.extend_from_slice(&[0; 12]);
        let (m, ip) = ipv4_check(ifp, m_devget(&p, 0).expect("a packet")).expect("accepted");
        assert_eq!(m.m_pkthdr().len.get(), 28);
        assert_eq!(
            (u16::from_be(ip.ip_len), ip.ip_hl()),
            (28, 5),
            "its header, read once"
        );
    }

    /// The fragment of datagram `id` at byte offset `off` with `data`, more following when
    /// `mf`.
    fn frag(id: u16, off: usize, mf: bool, data: &[u8], tos: u8) -> MbufBox {
        let word = (off / 8) as u16 | if mf { IP_MF } else { 0 };
        let mut p = header(0x45, (20 + data.len()) as u16, id, word, 17, tos).to_vec();
        p.extend_from_slice(data);
        packet(&p, 20)
    }

    /// `ip_fragcheck` of `m`: the datagram when it completes it.
    fn check(m: MbufBox) -> Option<MbufBox> {
        let ip = ip_load(&m);
        let mut mp = Some(m);
        let mut off = 0;
        let nxt = ip_fragcheck(&mut mp, &mut off, &ip);
        if nxt == IPPROTO_DONE {
            assert!(mp.is_none());
            return None;
        }
        assert_eq!((nxt, off), (17, 20));
        mp
    }

    fn queued() -> (i32, usize) {
        let g = IPQ.lock();
        (g.frags(), g.queues())
    }

    #[test]
    fn fragments_reassemble_in_any_order() {
        let _g = setup();
        let data: Vec<u8> = (0..72u8).collect();
        let orders: [[usize; 3]; 3] = [[0, 1, 2], [2, 1, 0], [1, 2, 0]];
        for (n, order) in orders.iter().enumerate() {
            let id = 100 + n as u16;
            let mut done = None;
            for (k, &i) in order.iter().enumerate() {
                let m = frag(id, i * 24, i < 2, &data[i * 24..i * 24 + 24], 0);
                let r = check(m);
                if k < 2 {
                    assert!(r.is_none(), "not complete before the last");
                    assert_eq!(queued(), (k as i32 + 1, 1));
                } else {
                    done = r;
                }
            }
            let m = done.expect("complete");
            let b = bytes(&m);
            assert_eq!(
                u16::from_be_bytes([b[2], b[3]]),
                92,
                "ip_len is the datagram's"
            );
            assert_eq!(&b[20..], &data[..], "the data in order");
            assert_eq!(b[6] & 0x3f, 0, "no fragment flag");
            assert_eq!(queued(), (0, 0), "the queue is gone");
        }
    }

    #[test]
    fn overlaps_are_trimmed_and_covered_fragments_dropped() {
        let _g = setup();
        let a = [1u8; 16];
        let b = [2u8; 16];
        let c = [3u8; 32];
        // [0,16) then [24,40) then [8,40): the new one is trimmed by [0,16) and covers
        // [24,40) whole, which goes.
        assert!(check(frag(7, 0, true, &a, 0)).is_none());
        assert!(check(frag(7, 24, true, &b, 0)).is_none());
        assert_eq!(queued(), (2, 1));
        assert!(check(frag(7, 8, true, &c, 0)).is_none());
        assert_eq!(queued(), (2, 1), "the covered fragment is gone");
        // [40,48) completes it.
        let m = check(frag(7, 40, false, &[4u8; 8], 0)).expect("complete");
        let got = bytes(&m);
        let mut want = std::vec![1u8; 16];
        want.extend_from_slice(&[3u8; 24]);
        want.extend_from_slice(&[4u8; 8]);
        assert_eq!(&got[20..], &want[..]);
        // A fragment the previous one covers whole is dropped.
        assert!(check(frag(8, 0, true, &[5u8; 16], 0)).is_none());
        let dropped = ips(IpstatCounters::IpsFragdropped);
        assert!(check(frag(8, 8, true, &[6u8; 8], 0)).is_none());
        assert_eq!(ips(IpstatCounters::IpsFragdropped), dropped + 1);
        assert_eq!(queued(), (1, 1));
    }

    #[test]
    fn ecn_ce_and_not_ect_are_not_mixed() {
        let _g = setup();
        let dropped = ips(IpstatCounters::IpsFragdropped);
        assert!(check(frag(9, 0, true, &[0u8; 8], IPTOS_ECN_NOTECT)).is_none());
        assert!(check(frag(9, 8, true, &[0u8; 8], IPTOS_ECN_CE)).is_none());
        assert_eq!(ips(IpstatCounters::IpsFragdropped), dropped + 1);
        assert!(check(frag(10, 0, true, &[0u8; 8], 0x01)).is_none());
        assert!(check(frag(10, 8, true, &[0u8; 8], IPTOS_ECN_NOTECT)).is_none());
        assert_eq!(ips(IpstatCounters::IpsFragdropped), dropped + 2);
        // CE after ECT is kept, and the first fragment carries it into the datagram.
        assert!(check(frag(11, 0, true, &[0u8; 8], 0x02)).is_none());
        let m = check(frag(11, 8, false, &[0u8; 8], IPTOS_ECN_CE)).expect("complete");
        assert_eq!(bytes(&m)[1] & IPTOS_ECN_MASK, IPTOS_ECN_CE);
    }

    #[test]
    fn a_fragment_without_a_multiple_of_8_is_bad() {
        let _g = setup();
        let bad = ips(IpstatCounters::IpsBadfrags);
        assert!(check(frag(12, 0, true, &[0u8; 12], 0)).is_none());
        assert_eq!(ips(IpstatCounters::IpsBadfrags), bad + 1);
        assert_eq!(queued(), (0, 0));
    }

    #[test]
    fn the_queue_is_bounded_and_ages_out() {
        let _g = setup();
        let max = IP_MAXQUEUE.swap(4, Ordering::Relaxed);
        for id in 0..4 {
            assert!(check(frag(200 + id, 0, true, &[0u8; 8], 0)).is_none());
        }
        assert_eq!(queued(), (4, 4));
        // The fifth is past ip_maxqueue: the oldest queues flushed down to 75 %, it dropped.
        let memdrop = ips(IpstatCounters::IpsRcvmemdrop);
        assert!(check(frag(300, 0, true, &[0u8; 8], 0)).is_none());
        assert_eq!(ips(IpstatCounters::IpsRcvmemdrop), memdrop + 1);
        assert_eq!(
            queued(),
            (3, 3),
            "flushed from the head (the newest) down to 3"
        );
        IP_MAXQUEUE.store(max, Ordering::Relaxed);
        // IPFRAGTTL slow timeouts later, every queue is gone.
        let timeout = ips(IpstatCounters::IpsFragtimeout);
        for _ in 0..IPFRAGTTL {
            ip_slowtimo();
        }
        assert_eq!(queued(), (0, 0));
        assert_eq!(ips(IpstatCounters::IpsFragtimeout), timeout + 3);
    }

    /// What `ip_dooptions`' walk left: its answer, the options, whether the packet is to be
    /// forwarded, its header and the packet.
    type Walked = (Result<(), (u8, i32)>, Vec<u8>, bool, Ip, MbufBox);

    /// `ip_dooptions`' walk over `opts` (padded to a word with EOL) of a packet from 10.0.2.2
    /// to 10.0.2.15 on `ifp`.
    fn walk_out(ifp: &'static Ifnet, opts: &[u8]) -> Walked {
        let mut o = opts.to_vec();
        while o.len() % 4 != 0 {
            o.push(IPOPT_EOL);
        }
        let hl = 20 + o.len();
        let mut p = header(0x40 | (hl / 4) as u8, (hl + 8) as u16, 1, 0, 17, 0).to_vec();
        p.extend_from_slice(&o);
        p.extend_from_slice(&[0; 8]);
        let mut m = packet(&p, hl);
        let mut ip = ip_load(&m);
        let dst = ip.ip_dst;
        let (mut forward, mut changed) = (false, false);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        hal::klock::kernel_lock();
        let r = dooptions_walk(
            &mut o,
            &mut ip,
            &mut m,
            dst,
            ifp,
            0,
            &mut forward,
            &mut changed,
            &net,
        );
        hal::klock::kernel_unlock();
        drop(w);
        (r.map_err(|b| (b.type_, b.code)), o, forward, ip, m)
    }

    /// `ip_dosourceroute` on until the guard drops, an assertion's failure included.
    struct SourceRouting;

    impl SourceRouting {
        fn on() -> Self {
            IP_DOSOURCEROUTE.store(1, Ordering::Relaxed);
            Self
        }
    }

    impl Drop for SourceRouting {
        fn drop(&mut self) {
            IP_DOSOURCEROUTE.store(0, Ordering::Relaxed);
        }
    }

    /// The walk's answer alone.
    fn walk(ifp: &'static Ifnet, opts: &[u8]) -> Result<(), (u8, i32)> {
        let (r, _, _, _, m) = walk_out(ifp, opts);
        m_freem(m);
        r
    }

    #[test]
    fn every_bound_of_the_options_is_checked() {
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        let pp = ICMP_PARAMPROB;
        // (what, the options, the C's answer: the parameter problem's pointer, or the
        // unreachable code, or none)
        let cases: [(&str, &[u8], Result<(), (u8, i32)>); 12] = [
            ("NOPs and EOL", &[IPOPT_NOP, IPOPT_NOP, IPOPT_EOL], Ok(())),
            // an option's type in the last byte: no room for its length
            (
                "no length byte",
                &[IPOPT_NOP, IPOPT_NOP, IPOPT_NOP, IPOPT_RR],
                Err((pp, 24)),
            ),
            ("length 1", &[IPOPT_RR, 1, 4, 0], Err((pp, 21))),
            (
                "length past the options",
                &[IPOPT_RR, 9, 4, 0],
                Err((pp, 21)),
            ),
            (
                "RR without its offset",
                &[IPOPT_RR, 2, IPOPT_NOP, IPOPT_NOP],
                Err((pp, 21)),
            ),
            (
                "RR offset below 4",
                &[IPOPT_RR, 7, 3, 0, 0, 0, 0],
                Err((pp, 22)),
            ),
            ("RR full: ignored", &[IPOPT_RR, 7, 8, 0, 0, 0, 0], Ok(())),
            (
                "TS shorter than its header",
                &[IPOPT_TS, 8, 5, 0, 0, 0, 0, 0],
                Err((pp, 20)),
            ),
            (
                "TS pointer below 5",
                &[IPOPT_TS, 12, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                Err((pp, 20)),
            ),
            (
                "TS full: overflow counted",
                &[IPOPT_TS, 12, 13, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                Ok(()),
            ),
            (
                "TS overflow wrapping",
                &[IPOPT_TS, 12, 13, 0xf0, 0, 0, 0, 0, 0, 0, 0, 0],
                Err((pp, 20)),
            ),
            (
                "TS unknown flag",
                &[IPOPT_TS, 12, 5, 0x02, 0, 0, 0, 0, 0, 0, 0, 0],
                Err((pp, 23)),
            ),
        ];
        for (what, opts, want) in cases {
            assert_eq!(walk(ifp, opts), want, "{what}");
        }
        // Source routes are refused while ip_dosourceroute is 0.
        assert_eq!(
            walk(ifp, &[IPOPT_LSRR, 7, 4, 10, 0, 2, 2]),
            Err((ICMP_UNREACH, i32::from(ICMP_UNREACH_SRCFAIL)))
        );
        {
            let _on = SourceRouting::on();
            assert_eq!(
                walk(ifp, &[IPOPT_SSRR, 2, IPOPT_NOP, IPOPT_NOP]),
                Err((pp, 21))
            );
            assert_eq!(walk(ifp, &[IPOPT_SSRR, 7, 3, 0, 0, 0, 0]), Err((pp, 22)));
        }
        // A record route with room gets the outgoing interface's address.
        assert_eq!(walk(ifp, &[IPOPT_RR, 7, 4, 0, 0, 0, 0]), Ok(()));
    }

    #[test]
    fn the_timestamp_option_stamps_where_it_has_room() {
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        let pp = ICMP_PARAMPROB;
        const E: u8 = 0xee;
        const TS: u8 = IPOPT_TS;
        // (what, the option, the answer, the bytes of the option the time lands in (none:
        // the option left as it was)). The C stamps through its local copy of the header: the
        // pointer, and TSANDADDR's address, are not written back.
        let cases: [(&str, [u8; 12], Result<(), (u8, i32)>, Option<usize>); 6] = [
            (
                "TSONLY with room",
                [TS, 12, 5, 0x00, E, E, E, E, E, E, E, E],
                Ok(()),
                Some(4),
            ),
            (
                "TSANDADDR with room",
                [TS, 12, 5, 0x01, E, E, E, E, E, E, E, E],
                Ok(()),
                Some(8),
            ),
            (
                "TSANDADDR without room for both",
                [TS, 12, 9, 0x01, E, E, E, E, E, E, E, E],
                Err((pp, 20)),
                None,
            ),
            (
                "PRESPEC of an address not ours: the next option",
                [TS, 12, 5, 0x03, 10, 0, 2, 99, E, E, E, E],
                Ok(()),
                None,
            ),
            (
                "PRESPEC of ours",
                [TS, 12, 5, 0x03, 10, 0, 2, 15, E, E, E, E],
                Ok(()),
                Some(8),
            ),
            (
                "PRESPEC without room for both",
                [TS, 12, 9, 0x03, 10, 0, 2, 15, E, E, E, E],
                Err((pp, 20)),
                None,
            ),
        ];
        for (what, opt, want, stamp) in cases {
            let len = usize::from(opt[1]).next_multiple_of(4);
            let (r, o, forward, _, m) = walk_out(ifp, &opt[..len]);
            m_freem(m);
            assert_eq!(r, want, "{what}");
            assert!(!forward, "{what}");
            if r.is_err() {
                continue;
            }
            let mut left = opt[..len].to_vec();
            if let Some(at) = stamp {
                assert_ne!(&o[at..at + 4], &[E; 4], "{what}: stamped");
                left[at..at + 4].copy_from_slice(&o[at..at + 4]);
            }
            assert_eq!(o, left, "{what}: nothing else changed");
        }
    }

    #[test]
    fn a_source_route_is_followed_and_saved_when_allowed() {
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        let _on = SourceRouting::on();
        // Not at its end: the next hop becomes the destination, the outgoing interface's
        // address takes its slot, the pointer moves on, and the packet is to be forwarded.
        let (r, o, forward, ip, m) = walk_out(ifp, &[IPOPT_LSRR, 11, 4, 10, 0, 2, 2, 0, 0, 0, 0]);
        m_freem(m);
        assert_eq!(r, Ok(()));
        assert!(forward);
        assert_eq!(ip.ip_dst.s_addr, u32::from_ne_bytes([10, 0, 2, 2]));
        assert_eq!(&o[..7], &[IPOPT_LSRR, 11, 8, 10, 0, 2, 15]);
        // At its end, for us: kept for the reply (save_rte), which ip_srcroute reverses: the
        // first hop, a NOP, the option's header with the pointer reset, the hops reversed and
        // the original source last.
        let (r, _, forward, _, mut m) = walk_out(ifp, &[IPOPT_LSRR, 7, 8, 10, 0, 2, 9]);
        assert_eq!(r, Ok(()));
        assert!(!forward);
        let rev = ip_srcroute(&mut m).expect("the route saved");
        m_freem(m);
        assert_eq!(
            rev.data(),
            &[
                10,
                0,
                2,
                9,
                IPOPT_NOP,
                IPOPT_LSRR,
                7,
                IPOPT_MINOFF,
                10,
                0,
                2,
                2
            ]
        );
    }

    #[test]
    fn a_protocol_without_mpinput_waits_for_ipintr() {
        let _g = setup();
        let ip = |p: u8| {
            let mut b = header(0x45, 28, 3, 0, p, 0).to_vec();
            b.extend_from_slice(&[0; 8]);
            packet(&b, 20)
        };
        let delivered = ips(IpstatCounters::IpsDelivered);
        let (udp, tcp) = (
            UDP.inputs.load(Ordering::Relaxed),
            TCP.inputs.load(Ordering::Relaxed),
        );
        let r = NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        let mut mp = Some(ip(17));
        let mut off = 20;
        assert_eq!(
            ip_deliver::<G>(&mut mp, &mut off, 17, i32::from(AF_INET), true, None, &net),
            IPPROTO_DONE
        );
        assert_eq!(
            UDP.inputs.load(Ordering::Relaxed),
            udp + 1,
            "delivered in place"
        );
        let mut mp = Some(ip(6));
        let mut off = 20;
        let _ = ip_deliver::<G>(&mut mp, &mut off, 6, i32::from(AF_INET), true, None, &net);
        drop(r);
        assert_eq!(
            TCP.inputs.load(Ordering::Relaxed),
            tcp,
            "queued, not delivered"
        );
        assert_eq!(
            ips(IpstatCounters::IpsDelivered),
            delivered + 1,
            "the queued one taken back"
        );
        let w = NETLOCK.write();
        ipintr::<G>(&NetHeld::of(&w).expect("the net lock"));
        drop(w);
        assert_eq!(
            TCP.inputs.load(Ordering::Relaxed),
            tcp + 1,
            "ipintr delivered it"
        );
    }

    #[test]
    fn the_sysctl_names_answer_as_the_c() {
        let _g = setup();
        let mut len = 0usize;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(ip_sysctl(&[], &mut req), Err(Errno::ENOTDIR));
        assert_eq!(ip_sysctl(&[IPCTL_DEFTTL, 1], &mut req), Err(Errno::ENOTDIR));
        assert_eq!(
            ip_sysctl(&[IPCTL_MRTSTATS], &mut req),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            ip_sysctl(&[IPCTL_IPSEC_STATS], &mut req),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(ip_sysctl(&[999], &mut req), Err(Errno::EOPNOTSUPP));
        assert_eq!(ip_sysctl(&[IPCTL_STATS], &mut req), Ok(()));
        assert_eq!(*req.oldlenp, size_of::<Ipstat>());
        assert_eq!(ip_sysctl(&[IPCTL_DEFTTL], &mut req), Ok(()));
        assert_eq!(*req.oldlenp, size_of::<i32>());
    }

    #[test]
    fn the_reassembly_records_stay_small() {
        // vmstat -m's "ipq" and "ipqe" items (D-3) against the C's LP64 structures.
        #[repr(C)]
        struct CIpqent {
            ipqe_q: [u64; 2],
            ipqe_ip: u64,
            ipqe_m: u64,
            ipqe_mff: u8,
        }
        #[repr(C)]
        struct CIpq {
            ipq_q: [u64; 2],
            ipq_rdomain: u32,
            ipq_ttl: u8,
            ipq_p: u8,
            ipq_id: u16,
            ipq_fragq: u64,
            ipq_src: u32,
            ipq_dst: u32,
        }
        std::eprintln!(
            "ipq: C {} kernel {}; ipqe: C {} kernel {}",
            size_of::<CIpq>(),
            size_of::<Ipq>(),
            size_of::<CIpqent>(),
            size_of::<Ipqent>()
        );
        assert_eq!((size_of::<CIpq>(), size_of::<CIpqent>()), (40, 40));
        assert!(size_of::<Ipq>() <= 56 && size_of::<Ipqent>() <= 40);
    }
}
/* </TESTS> */
