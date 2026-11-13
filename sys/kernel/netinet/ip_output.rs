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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)ip_output.c	8.3 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! IPv4 output, the output half of `ip_output.c`: `ip_output`, fragmentation, option
//! insertion and copy, multicast loopback and the output checksums.
//!
//! Upstream: sys/netinet/ip_output.c @ 3ce1f3f79392
//! LZ: sys/netinet/ip_output.rs@3c62ede68802
//!
//! `ip_output` takes a packet with a skeletal IP header (length, offset, ttl, protocol, tos,
//! source, destination), completes it, finds the route and the interface (caching the route
//! in a `struct route`) and hands the packet to `if_output_tso`, fragmenting it with
//! [`ip_fragment`] when it is too large for the interface. The socket option half
//! ([`ip_ctloutput`] .. [`ip_freemoptions`], N6.4c1) keeps a socket's IP options, header
//! fields and multicast options in its control block (`kernel::netinet::in_pcb::Inpcb`).
//!
//! ## Deviations
//! - IPsec (`#ifdef IPSEC`, configured): the C's `ip_output_ipsec_lookup` and
//!   `ip_output_ipsec_send` (with `ip_output_ipsec_pmtu_update`) are `bsd`'s, reached through
//!   the `Ipsec` bridge (`kernel::net::ipsec_hooks`) at the C's points, with `ipsec_in_use`,
//!   `tdb_unref` and `ipsec_adjust_mtu`: they read the SA's members and call TCP's software
//!   TSO and ICMP's route cloning, all `bsd`'s until N8 (the module's redesign, N6.4a's R10).
//!   Before the bridge is registered `ip_output` runs as the C without `IPSEC`.
//! - `NPF` (pf(4)) is configured: `pf_test` through the `Pf` bridge's `test`, which passes
//!   every packet before registration (`NPF == 0`); a packet pf tagged `PF_TAG_REROUTE`
//!   reruns the route lookup (the C's `goto reroute` is a labelled loop).
//! - Not configured, a comment at its site: `MROUTING` (`ip_mforward`).
//! - `icmp_mtudisc_clone` (ICMP's, N6.4c2) and the TCP and UDP counters of software
//!   checksums (`tcps_outswcsum`, `udps_outswcsum`, N6.6) are `Inet` legs.
//! - `ip_ctloutput` with `IPSEC` configured: the four IPsec levels are set and read (no
//!   `EOPNOTSUPP`); `IP_IPSEC_LOCAL_ID` and `IP_IPSEC_REMOTE_ID` answer `EOPNOTSUPP` as the C.
//!   A `PRCO_GETOPT` without an answer mbuf is `panic(9)` (`sogetopt` always hands one over;
//!   the C would write through NULL; LZ answered `EINVAL`).
//! - `ip_setmoptions`'s `malloc(M_WAITOK)` and `mallocarray(M_WAITOK)` cannot fail in C:
//!   here their failure (no `malloc(9)` registered) is `panic(9)`, as LZ's.
//! - `ip_optcopy`'s option without its length byte (one byte left, not `IPOPT_EOL` or
//!   `IPOPT_NOP`) ends the copy, where the C reads the byte past the options (a `DIAGNOSTIC`
//!   kernel panics); a length byte of 0 ends it too (LZ's), where the C would loop.
//!
//! ## Redesign
//! - The options to insert (`struct mbuf *opt`, a `struct ipoption` in one mbuf) are its
//!   bytes, `opt: Option<&[u8]>` (`mtod(opt)`, `opt->m_len` long): `bsd`'s callers lend their
//!   option mbuf's data, IGMP its constant Router Alert option (N6.4a's R16).
//! - The source address (`ifatoia(rt_ifa)->ia_addr`, `in_ifp2ia(ifp)->ia_addr`) is the
//!   Internet record's own word (`kernel::netinet::in_var::SinWords::addr`), one load.
//! - LZ's `ip_output(m: &'static Mbuf, ..)` -> [`ip_output`] over the kernel's owned packet
//!   (`MbufBox`): the C's `goto bad` (free the packet) and `goto done` (it is consumed) are
//!   the same `return` here, the packet dropped by its owner if it still has one; the
//!   interface reference is an `IfRef` (`if_put` on drop), the SA's a `TdbHold` (`tdb_unref`
//!   on drop), `iproute`'s route its `Route` (`rtfree` on drop). `NET_ASSERT_LOCKED()` is the
//!   `net` proof parameter; `bsd`'s 21 importers keep LZ's signature through a wrapper in
//!   `sys/netinet/ip_output.rs` (`unpark`, the proof through the `SaToken`).
//! - The IP header: LZ's `mtod_ip`/`mtod_ip_store` (an unaligned copy through `unsafe`) ->
//!   [`ip_load`]/[`ip_store`], `hal::pod`'s view of the first mbuf's bytes when they are
//!   aligned (the transports' packets and the 2-byte aligned Ethernet frames), a copy
//!   otherwise; a read-only cluster is written through `m_copyback` (copy on write).
//!   `ip_output` loads the header once, works on the copy and stores it before each step
//!   that reads the packet (IPsec, the multicast loopback, pf, the interface), where LZ
//!   stored it after every change; the checksum writes touch their two bytes only
//!   ([`cksum_store`]).
//! - `ip_fragment(m0, ml, ..)`: the fragments built on a list of their own and appended
//!   after the first one at the end, so the first stays owned (the C enqueues it first and
//!   keeps writing it through its pointer); on failure both are freed, as `ml_purge`.
//! - `ip_optcopy(ip, jp)` over raw headers (an `unsafe fn` in LZ) -> [`ip_optcopy`] over the
//!   two option slices.
//! - `in_hdr_cksum_out`, `in_proto_cksum_out` (and `in_delayed_cksum`) take `&mut Mbuf`:
//!   the kernel's Ethernet layer (`ether_offload_ifcap`) and `if_output_tso` call them
//!   directly, so the `Inet` legs of the same names go; `bsd`'s callers reach them through
//!   the wrappers of `sys/netinet/ip_output.rs`.
//! - The packet filter's verdict: `pf_test(af, dir, ifp, &m)` -> `Pf::test`, the verdict and
//!   the packet as pf left it.
//! - N6.4c1, the socket options: LZ's `ip_ctloutput(op, so, level, optname, m)` read the
//!   option values with unaligned raw reads at `mtod(m)` and wrote the answers the same way
//!   (`mtod_int`, `set_mtod_int`, `mtod_bytes`, `mtod_mreqn`: four `unsafe`), kept the
//!   multicast options as a raw `malloc(M_IPMOPTS)` pointer in the pcb with its membership
//!   vector reached through slot pointers (`imo_slot`, `imo_member`, six `unsafe`), and freed
//!   them with an `unsafe fn ip_freemoptions`. Here [`ip_ctloutput`] takes the control block
//!   (`sotoinpcb(so)` is `bsd`'s caller's until N6.6 types `so_pcb`), reads a value from the
//!   option mbuf's bytes (`Mbuf::data`, exactly `m_len` long: a length check is a slice
//!   pattern) and writes an answer with `m_len` set first ([`opt_put`]); the multicast
//!   options are an `MBox<IpMoptions>` of `M_IPMOPTS` that the pcb owns
//!   (`Inpcb::moptions`, lent under the socket lock check) with an `MArray` of memberships
//!   (`kernel::netinet::ip_var::IpMoptions`): the C's two allocations, the vector grown by a
//!   new `MArray` copied from the old one (`M_NOWAIT`, the old freed after), the set freed
//!   when every option is back to its default, [`ip_freemoptions`] by value. `ip_multicast_if`
//!   answers the index (the C's out parameter). The socket's IPv4 options (`inp_options`) stay
//!   a parked mbuf in `bsd`'s word, read per packet by `bsd`'s transports: `bsd`'s adapter
//!   lends them to [`ip_ctloutput`] as an `&mut Option<MbufBox>` for the call (R11,
//!   docs/SCAFFOLDING.md), [`ip_pcbopts`] builds the new `struct ipoption` in a local
//!   buffer and copies it into a fresh `MT_SOOPTS` mbuf (the C's `m_get(M_NOWAIT)`).
//!   `ip_defttl` is `kernel::netinet::ip_var::IP_DEFTTL`.
//!
//! **Lock and spl transitions of the socket options** against the C: `sosetopt` and
//! `sogetopt` hold the socket lock (`solock`: the net lock exclusively, for an internet
//! socket) around `ip_ctloutput`, which takes none of its own, as the C; the multicast
//! options' loan is two claims of the pcb's cell (atomics, no lock, no spl) and a check of
//! the socket lock (two lock-word loads); `IP_MULTICAST_IF` by address walks the interfaces
//! (`ifa_ifwithaddr`) and `ip_multicast_if` looks a route up, as the C; `in_addmulti` and
//! `in_delmulti` take `if_maddrlock` as the C's. The transports' `ip_output` calls read the
//! three scalars `ip_output` uses out of the cell (`Inpcb::moptions_out`: a reader's count,
//! two atomics) where the C passes the pointer: named for N12.
//!
//! **Lock and spl transitions** against the C (`ip_output`, the unicast path): the caller's
//! net lock (any mode), no lock of its own; the route cache hit reads the route's validity
//! (`rtisvalid`, the gateway route's `ReadMostly` read, two atomics, N6.3b2) where the C reads
//! two words; `if_get`/`if_put` one reference count each, as the C; the IPsec send keeps the
//! C's `KERNEL_LOCK()` around `ipsp_process_packet` inside `bsd`. The multicast loopback
//! takes `if_maddrlock` for reading once, as `in_hasmulti`. No spl change: the C has none.

use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::netlock::NetHeld;
use hal::pod;
use hal::sys::endian::{htonl, htons, ntohl, ntohs};
use hal::sys::errno::Errno;
use hal::sys::net_if::{
    IFCAP_CSUM_IPv4, IFCAP_CSUM_TCPv4, IFCAP_CSUM_UDPv4, IFCAP_TSOv4, IFF_BROADCAST, IFF_LOOPBACK,
    IFF_MULTICAST, IFF_SIMPLEX,
};
use hal::sys::net_route::{RTF_BROADCAST, RTF_GATEWAY, RTF_HOST, RTF_LOCAL, RTV_MTU};
use hal::sys::netinet_in::{
    IN_CLASSA_NSHIFT, INADDR_ANY, INADDR_BROADCAST, IP_ADD_MEMBERSHIP, IP_AUTH_LEVEL,
    IP_DEFAULT_MULTICAST_LOOP, IP_DEFAULT_MULTICAST_TTL, IP_DROP_MEMBERSHIP, IP_ESP_NETWORK_LEVEL,
    IP_ESP_TRANS_LEVEL, IP_IPCOMP_LEVEL, IP_IPDEFTTL, IP_IPSEC_LOCAL_ID, IP_IPSEC_REMOTE_ID,
    IP_IPSECFLOWINFO, IP_MAX_MEMBERSHIPS, IP_MIN_MEMBERSHIPS, IP_MINTTL, IP_MULTICAST_IF,
    IP_MULTICAST_LOOP, IP_MULTICAST_TTL, IP_OPTIONS, IP_PIPEX, IP_PORTRANGE, IP_PORTRANGE_DEFAULT,
    IP_PORTRANGE_HIGH, IP_PORTRANGE_LOW, IP_RECVDSTADDR, IP_RECVDSTPORT, IP_RECVIF, IP_RECVOPTS,
    IP_RECVRETOPTS, IP_RECVRTABLE, IP_RECVTTL, IP_RETOPTS, IP_TOS, IP_TTL, IPPROTO_ICMP,
    IPPROTO_IP, IPPROTO_TCP, IPPROTO_UDP, IPSEC_AUTH_LEVEL_DEFAULT,
    IPSEC_ESP_NETWORK_LEVEL_DEFAULT, IPSEC_ESP_TRANS_LEVEL_DEFAULT, IPSEC_IPCOMP_LEVEL_DEFAULT,
    IPSEC_LEVEL_BYPASS, IPSEC_LEVEL_UNIQUE, InAddr, IpMreq, IpMreqn, in_multicast,
};
use hal::sys::netinet_ip::{
    IP_DF, IP_MAXPACKET, IP_MF, IPOPT_EOL, IPOPT_LSRR, IPOPT_MINOFF, IPOPT_NOP, IPOPT_OFFSET,
    IPOPT_OLEN, IPOPT_OPTVAL, IPOPT_SSRR, IPVERSION, Ip, MAXTTL, ipopt_copied,
};
use hal::sys::netinet_ip_var::{Ipoption, MAX_IPOPTLEN};
use hal::sys::netinet_tcp::Tcphdr;
use hal::sys::netinet_udp::Udphdr;
use hal::sys::socket::{AF_INET, SO_RTABLE};

use crate::assert::kpanic;
use crate::net::if_::{IfRef, if_input_local, if_output_ml, if_output_tso, ifa_ifwithaddr, inet};
use crate::net::if_var::Ifnet;
use crate::net::ipsec_hooks::{IpsecLevel, SpdError, TdbHold, ipsec};
use crate::net::mbuf::{
    M_BCAST, M_DONTWAIT, M_ICMP_CSUM_OUT, M_IPV4_CSUM_OUT, M_MCAST, M_TCP_CSUM_OUT, M_TCP_TSO,
    M_UDP_CSUM_OUT, MAX_LINKHDR, MT_HEADER, MT_SOOPTS, Mbuf, MbufBox, MbufList,
    PACKET_TAG_IPSEC_IN_DONE, PF_TAG_GENERATED, PF_TAG_REROUTE, m_adj, m_copyback, m_copym,
    m_dup_pkt, m_dup_pkthdr, m_get, m_gethdr, m_move_hdr, ml_enlist, ml_enqueue, ml_init, ml_len,
};
use crate::net::pf_hooks::{PF_FWD, PF_OUT, PF_PASS, pf};
use crate::net::proto::{PRCO_GETOPT, PRCO_SETOPT, SockaddrView};
use crate::net::route::{RT_RESOLVE, Route, rtalloc, rtalloc_mpath, rtisvalid};
use crate::net::rtable::{rtable_l2, rtable_loindex};
use crate::netinet::in_::{in_addmulti, in_delmulti, in_hasmulti, in_ifp2ia};
use crate::netinet::in_cksum::in_cksum;
use crate::netinet::in_pcb::{
    INP_HIGHPORT, INP_IPSECFLOWINFO, INP_LOWPORT, INP_RECVDSTADDR, INP_RECVDSTPORT, INP_RECVIF,
    INP_RECVOPTS, INP_RECVRETOPTS, INP_RECVRTABLE, INP_RECVTTL, Inpcb, in_pcbset_rtableid,
};
use crate::netinet::in_var::InIfaddr;
use crate::netinet::in4_cksum::in4_cksum;
use crate::netinet::ip_id::ip_randomid;
use crate::netinet::ip_var::{
    IP_ALLOWBROADCAST, IP_DEFTTL, IP_FORWARDING, IP_FORWARDING_IPSEC, IP_MTUDISC, IP_RAWOUTPUT,
    IpMoptions, IpstatCounters, ip_mtudisc, ipstat_add, ipstat_inc,
};
use crate::proc::{ProcGlue, Thread, suser};

/// `sizeof(struct ip)`: the header without options.
const IP_HDR_LEN: usize = size_of::<Ip>();
/// `offsetof(struct ip, ip_sum)`.
const IP_SUM_OFFSET: usize = core::mem::offset_of!(Ip, ip_sum);
/// `offsetof(struct tcphdr, th_sum)`.
const TH_SUM_OFFSET: usize = core::mem::offset_of!(Tcphdr, th_sum);
/// `offsetof(struct udphdr, uh_sum)`.
const UH_SUM_OFFSET: usize = core::mem::offset_of!(Udphdr, uh_sum);
/// `offsetof(struct icmp, icmp_cksum)`: after `icmp_type` and `icmp_code` (`ip_icmp.h`; the
/// image moves with ICMP, N6.4c2).
const ICMP_CKSUM_OFFSET: usize = 2;
/// The size of a `struct sockaddr_in`.
const SIN_LEN: usize = 16;
/// The offset of `sin_addr` in a `struct sockaddr_in`.
const SIN_ADDR: usize = 4;
/// `sizeof(struct in_addr)`.
const IN_ADDR_LEN: usize = size_of::<InAddr>();
/// `M_IPMOPTS`: internet multicast options (`malloc.h`).
const M_IPMOPTS: i32 = 53;

/// `mtod(m, struct ip *)`, read: the IP header at the start of `m`'s data, in place when it
/// is aligned for one, copied otherwise. `panic(9)` when the first mbuf holds less than a
/// header, where the C reads past it.
#[inline]
pub fn ip_load(m: &Mbuf) -> Ip {
    let d = m.data();
    if let Some(ip) = pod::view::<Ip>(d, 0) {
        return *ip;
    }
    match pod::at::<Ip>(d, 0) {
        Some(ip) => ip,
        None => kpanic(format_args!("ip_output: an IP header in {} bytes", d.len())),
    }
}

/// `*mtod(m, struct ip *) = *ip`: the header written back at the start of `m`'s data, in
/// place when the data is writable, through `m_copyback` (a copy of a shared cluster first)
/// when it is not.
#[inline]
pub fn ip_store(m: &mut Mbuf, ip: &Ip) {
    if let Some(d) = m.data_mut() {
        if let Some(v) = pod::view_mut::<Ip>(d, 0) {
            *v = *ip;
            return;
        }
        if pod::put(d, 0, ip) {
            return;
        }
    }
    let mut b = [0u8; IP_HDR_LEN];
    let _ = pod::put(&mut b, 0, ip);
    if m_copyback(m, 0, &b, M_DONTWAIT).is_err() {
        kpanic(format_args!("ip_output: the IP header cannot be written"));
    }
}

/// The address of the `sockaddr_in` image `sa` (its bytes 4 to 8), network order.
#[inline]
fn sin_addr_of(sa: &[u8]) -> InAddr {
    let mut a = [0u8; 4];
    if let Some(b) = sa.get(SIN_ADDR..SIN_ADDR + 4) {
        a.copy_from_slice(b);
    }
    InAddr {
        s_addr: u32::from_ne_bytes(a),
    }
}

/// `in_ifp2ia(ifp)->ia_addr.sin_addr`: the address of the first IPv4 address of `ifp`.
#[inline]
fn ifp_inaddr(ifp: &Ifnet) -> Option<InAddr> {
    in_ifp2ia(ifp).map(|ia| ia.ia_addr.addr())
}

/// `ip_output(m, opt, ro, flags, imo, seclevel, ipsecflowinfo)`: IP output. The packet `m`
/// holds a skeletal IP header (with len, off, ttl, proto, tos, src, dst); it is consumed. The
/// options `opt` (the bytes of a `struct ipoption`), if present, are not. The caller holds the
/// net lock (`net`).
#[allow(
    clippy::too_many_arguments,
    reason = "the C's signature and the net lock's proof"
)]
pub fn ip_output(
    m: MbufBox,
    opt: Option<&[u8]>,
    ro: Option<&mut Route>,
    flags: i32,
    imo: Option<&IpMoptions>,
    seclevel: Option<&IpsecLevel>,
    ipsecflowinfo: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut m = m;
    let mut hlen = IP_HDR_LEN as i32;

    // DIAGNOSTIC's "ip_output no HDR": `m_pkthdr()` below is `panic(9)` without one.
    if let Some(opt) = opt {
        m = ip_insertoptions(m, opt, &mut hlen);
    }

    let mut ip = ip_load(&m);

    // Fill in IP header.
    if flags & (IP_FORWARDING | IP_RAWOUTPUT) == 0 {
        ip.set_ip_v(IPVERSION);
        ip.ip_off &= htons(IP_DF);
        ip.ip_id = htons(ip_randomid());
        ip.set_ip_hl((hlen >> 2) as u8);
        ipstat_inc(IpstatCounters::IpsLocalout);
    } else {
        hlen = i32::from(ip.ip_hl()) << 2;
    }

    // We should not send traffic to 0/8 say both Stevens and RFCs 5735 section 3 and 1122
    // sections 3.2.1.3 and 3.3.6.
    if ntohl(ip.ip_dst.s_addr) >> IN_CLASSA_NSHIFT == 0 {
        return Err(Errno::ENETUNREACH);
    }

    let orig_rtableid = m.m_pkthdr().ph_rtableid.get();
    let mut caller_ro = ro;
    let mut iproute = Route::new();
    let mut ifp: Option<IfRef>;
    let mut tdb: Option<TdbHold> = None;
    let mut mtu: u32;

    // reroute: pf(4) asks for a new route lookup after it changed the packet.
    loop {
        // Do a route lookup now in case we need the source address to do an SPD lookup in
        // IPsec; for most packets, the source address is set at a higher level protocol.
        // ICMPs and other packets though (e.g., traceroute) have a source address of zeroes.
        // The caller's route, or ours when it gave none or pf rerouted.
        let ro: &mut Route = match caller_ro.as_deref_mut() {
            Some(r) => r,
            None => &mut iproute,
        };

        // If there is a cached route, check that it is to the same destination and is still
        // up. If not, free it and try again.
        let _ = ro.cache4(
            ip.ip_dst.s_addr.to_ne_bytes(),
            Some(ip.ip_src.s_addr.to_ne_bytes()),
            m.m_pkthdr().ph_rtableid.get(),
        );
        // `dst = &ro->ro_dstsin`, or the gateway below: the address the interface is given.
        let mut dst = [0u8; SIN_LEN];
        dst.copy_from_slice(&ro.ro_dst[..SIN_LEN]);

        let mcast_or_bcast = in_multicast(ip.ip_dst.s_addr) || ip.ip_dst.s_addr == INADDR_BROADCAST;
        let imo_ifp = match imo {
            Some(imo) if mcast_or_bcast => IfRef::get(u32::from(imo.imo_ifidx)),
            _ => None,
        };
        if let Some(i) = imo_ifp {
            mtu = i.if_mtu.get();
            if ip.ip_src.s_addr == INADDR_ANY
                && let Some(a) = ifp_inaddr(&i)
            {
                ip.ip_src = a;
            }
            ifp = Some(i);
        } else {
            if ro.rt().is_none() {
                ro.ro_rt = rtalloc_mpath(ro.dst(), Some(&[ip.ip_src.s_addr]), ro.ro_tableid, net);
            }

            let Some(rt) = ro.rt() else {
                ipstat_inc(IpstatCounters::IpsNoroute);
                return Err(Errno::EHOSTUNREACH);
            };

            // `ia = ifatoia(ro->ro_rt->rt_ifa)`: its record found below, for a source
            // address only.
            let ia = rt.ifa();
            let idx = if rt.rt_flags.isset(RTF_LOCAL) {
                rtable_loindex(m.m_pkthdr().ph_rtableid.get())
            } else {
                rt.rt_ifidx.get()
            };
            // We aren't using rtisvalid() here because the UP/DOWN state machine is broken
            // with some Ethernet drivers like em(4). As a result we might try to use an
            // invalid cached route entry while an interface is being detached.
            let Some(i) = IfRef::get(idx) else {
                ipstat_inc(IpstatCounters::IpsNoroute);
                return Err(Errno::EHOSTUNREACH);
            };
            mtu = rt.rt_mtu().load(Ordering::Relaxed);
            if mtu == 0 {
                mtu = i.if_mtu.get();
            }

            // `dst = satosin(ro->ro_rt->rt_gateway)`: the gateway read once, copied.
            if rt.rt_flags.isset(RTF_GATEWAY) {
                rt.with_gateway(|g| {
                    if let Some(g) = g.and_then(|g| g.get(..SIN_LEN)) {
                        dst.copy_from_slice(g);
                    }
                });
            }

            // Set the source IP address
            if ip.ip_src.s_addr == INADDR_ANY
                && let Some(ia) = ia.and_then(InIfaddr::of)
            {
                ip.ip_src = ia.ia_addr.addr();
            }
            ifp = Some(i);
        }

        if let Some(legs) = ipsec()
            && (legs.in_use() || seclevel.is_some())
        {
            // Do we have any pending SAs to apply ? The SPD reads the header in the packet.
            ip_store(&mut m, &ip);
            match legs.ip_output_lookup(&m, hlen, seclevel, ipsecflowinfo) {
                // Should silently drop packet
                Err(SpdError::Drop) => return Ok(()),
                Err(SpdError::Errno(e)) => return Err(e),
                Ok(t) => tdb = t.map(|t| TdbHold::new(legs, t)),
            }
            if tdb.is_some() {
                // If it needs TCP/UDP hardware-checksumming, do the computation now.
                in_proto_cksum_out(&mut m, None);
            }
        }

        // The interface is known here: both branches above set it or returned.
        let Some(ifn) = ifp.as_ref().map(IfRef::ifp) else {
            ipstat_inc(IpstatCounters::IpsNoroute);
            return Err(Errno::EHOSTUNREACH);
        };

        if mcast_or_bcast {
            let bit = if ip.ip_dst.s_addr == INADDR_BROADCAST {
                M_BCAST
            } else {
                M_MCAST
            };
            m.m_flags().set(m.flags() | bit);

            // IP destination address is multicast. Make sure "dst" still points to the
            // address in "ro". (It may have been changed to point to a gateway address,
            // above.)
            dst.copy_from_slice(&ro.ro_dst[..SIN_LEN]);

            // See if the caller provided any multicast options
            ip.ip_ttl = imo.map_or(IP_DEFAULT_MULTICAST_TTL, |imo| imo.imo_ttl);

            // Confirm that the outgoing interface supports multicast, but only if the packet
            // actually is going out on that interface (i.e., no IPsec is applied).
            let f = ifn.if_flags.get();
            if ((m.flags() & M_MCAST != 0 && f & IFF_MULTICAST == 0)
                || (m.flags() & M_BCAST != 0 && f & IFF_BROADCAST == 0))
                && tdb.is_none()
            {
                ipstat_inc(IpstatCounters::IpsNoroute);
                return Err(Errno::ENETUNREACH);
            }

            // If source address not specified yet, use address of outgoing interface.
            if ip.ip_src.s_addr == INADDR_ANY
                && let Some(a) = ifp_inaddr(ifn)
            {
                ip.ip_src = a;
            }

            if imo.is_none_or(|imo| imo.imo_loop != 0) && in_hasmulti(&ip.ip_dst, ifn) {
                // If we belong to the destination multicast group on the outgoing interface,
                // and the caller did not forbid loopback, loop back a copy. Can't defer
                // TCP/UDP checksumming, do the computation now.
                ip_store(&mut m, &ip);
                in_proto_cksum_out(&mut m, None);
                ip_mloopback(ifn, &m, &dst);
            }
            // MROUTING: ip_mforward when ipmforwarding and ip_mrouter_active; not configured.

            // Multicasts with a time-to-live of zero may be looped-back, above, but must not
            // be transmitted on a network. Also, multicasts addressed to the loopback
            // interface are not sent -- the above call to ip_mloopback() will loop back a
            // copy if this host actually belongs to the destination group on the loopback
            // interface.
            if ip.ip_ttl == 0 || f & IFF_LOOPBACK != 0 {
                return Ok(());
            }
        }

        // Look for broadcast address and verify user is allowed to send such a packet; if the
        // packet is going in an IPsec tunnel, skip this check.
        if tdb.is_none()
            && (sin_addr_of(&dst).s_addr == INADDR_BROADCAST
                || ro.rt().is_some_and(|rt| rt.rt_flags.isset(RTF_BROADCAST)))
        {
            if ifn.if_flags.get() & IFF_BROADCAST == 0 {
                return Err(Errno::EADDRNOTAVAIL);
            }
            if flags & IP_ALLOWBROADCAST == 0 {
                return Err(Errno::EACCES);
            }

            // Don't allow broadcast messages to be fragmented
            if u32::from(ntohs(ip.ip_len)) > ifn.if_mtu.get() {
                return Err(Errno::EMSGSIZE);
            }
            m.m_flags().set(m.flags() | M_BCAST);
        } else {
            m.m_flags().set(m.flags() & !M_BCAST);
        }

        // If we're doing Path MTU discovery, we need to set DF unless the route's MTU is
        // locked.
        if flags & IP_MTUDISC != 0 && ro.rt().is_some_and(|rt| rt.rt_locks().get() & RTV_MTU == 0) {
            ip.ip_off |= htons(IP_DF);
        }
        ip_store(&mut m, &ip);

        // Check if the packet needs encapsulation.
        if let Some(t) = tdb.as_ref() {
            // Callee frees mbuf
            return t.legs().ip_output_send(
                t.tdb(),
                m,
                ro,
                orig_rtableid,
                flags & IP_FORWARDING != 0,
            );
        }

        // Packet filter
        if let Some(p) = pf() {
            let dir = if flags & IP_FORWARDING != 0 {
                PF_FWD
            } else {
                PF_OUT
            };
            let (action, left) = p.test(AF_INET, dir, ifn, m);
            if action != PF_PASS {
                // `goto bad` frees the packet as pf left it.
                drop(left);
                return Err(Errno::EACCES);
            }
            let Some(mm) = left else {
                return Ok(());
            };
            m = mm;
            ip = ip_load(&m);
            // The C recomputes `hlen` here; nothing reads it below.
            let pfh = &m.m_pkthdr().pf;
            let tags = pfh.flags.get();
            if tags & (PF_TAG_REROUTE | PF_TAG_GENERATED) == (PF_TAG_REROUTE | PF_TAG_GENERATED) {
                // already rerun the route lookup, go on
                pfh.flags.set(tags & !(PF_TAG_GENERATED | PF_TAG_REROUTE));
            } else if tags & PF_TAG_REROUTE != 0 {
                // tag as generated to skip over pf_test on rerun
                pfh.flags.set(tags | PF_TAG_GENERATED);
                // `ro = NULL`, then `ro = &iproute; ro->ro_rt = NULL` at reroute: our
                // route given back (`rtfree`), the caller's left to the caller.
                caller_ro = None;
                iproute.ro_rt = None;
                // drop reference since target changed (`if_put(ifp); ifp = NULL`)
                drop(ifp.take());
                continue;
            }
        }

        if flags & IP_FORWARDING != 0
            && flags & IP_FORWARDING_IPSEC != 0
            && m.m_pkthdr().ph_tagsset.get() & PACKET_TAG_IPSEC_IN_DONE == 0
        {
            return Err(Errno::EHOSTUNREACH);
        }

        let Some(dsa) = SockaddrView::new(&dst) else {
            kpanic(format_args!("ip_output: a destination too short"));
        };

        // If TSO or small enough for interface, can just send directly.
        let mut mp = Some(m);
        if_output_tso(ifn, &mut mp, dsa, ro.rt(), mtu)?;
        let Some(m) = mp else {
            return Ok(());
        };

        // Too large for interface; fragment if possible. Must be able to put at least 8 bytes
        // per fragment. (`ip` is the header `m` holds: nothing changed it since.)
        if ip.ip_off & htons(IP_DF) != 0 {
            if ip_mtudisc.load(Ordering::Relaxed) != 0
                && let Some(legs) = ipsec()
            {
                legs.adjust_mtu(&m, ifn.if_mtu.get());
            }
            // pf changed routing table, use orig rtable for path MTU
            if ro.ro_tableid != orig_rtableid {
                ro.ro_rt = None;
                ro.ro_tableid = orig_rtableid;
                let mut a = [0u8; 4];
                a.copy_from_slice(&ro.ro_dst[SIN_ADDR..SIN_ADDR + 4]);
                ro.ro_rt = inet().and_then(|i| i.icmp_mtudisc_clone(a, ro.ro_tableid, false));
            }

            // This case can happen if the user changed the MTU of an interface after enabling
            // IP on it. Because most netifs don't keep track of routes pointing to them, there
            // is no way for one to update all its routes when the MTU is changed.
            if let Some(rt) = ro.rt()
                && rtisvalid(Some(rt))
                && rt.rt_flags.isset(RTF_HOST)
                && rt.rt_locks().get() & RTV_MTU == 0
            {
                let rtmtu = rt.rt_mtu().load(Ordering::Relaxed);
                if rtmtu > ifn.if_mtu.get() {
                    let _ = rt.rt_mtu().compare_exchange(
                        rtmtu,
                        ifn.if_mtu.get(),
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
                }
            }
            ipstat_inc(IpstatCounters::IpsCantfrag);
            return Err(Errno::EMSGSIZE);
        }

        let mut ml = MbufList::new();
        ip_fragment(m, &mut ml, ifn, mtu)?;
        if_output_ml(ifn, &mut ml, dsa, ro.rt())?;
        ipstat_inc(IpstatCounters::IpsFragmented);
        return Ok(());
    }
}

/// `ip_fragment(m0, ml, ifp, mtu)`: `m0` split into fragments for `mtu` on `ml`, the first
/// (trimmed) one first; on failure every fragment is freed.
pub fn ip_fragment(m0: MbufBox, ml: &mut MbufList, ifp: &Ifnet, mtu: u32) -> Result<(), Errno> {
    let mut m0 = m0;
    ml_init(ml);

    let mut ip = ip_load(&m0);
    let hlen = i32::from(ip.ip_hl()) << 2;
    let tlen = m0.m_pkthdr().len.get();
    // `(mtu - hlen) & ~7` in the C's unsigned arithmetic, then an `int`.
    let mut len = ((i64::from(mtu) - i64::from(hlen)) & !7) as i32;
    if len < 8 {
        ipstat_inc(IpstatCounters::IpsOdropped);
        return Err(Errno::EMSGSIZE);
    }
    let firstlen = len;

    // If we are doing fragmentation, we can't defer TCP/UDP checksumming; compute the
    // checksum and clear the flag.
    in_proto_cksum_out(&mut m0, None);

    // The options of the header, for the later fragments' copies.
    let mut opts = [0u8; 40];
    let nopts = (hlen as usize).saturating_sub(IP_HDR_LEN).min(opts.len());
    if nopts > 0 {
        match m0.data().get(IP_HDR_LEN..IP_HDR_LEN + nopts) {
            Some(o) => opts[..nopts].copy_from_slice(o),
            None => kpanic(format_args!("ip_fragment: a header longer than its mbuf")),
        }
    }

    // Loop through length of payload after first fragment, make new header and copy data of
    // each part and link onto chain.
    let mut frags = MbufList::new();
    let mut off = hlen + firstlen;
    let built: Result<(), Errno> = 'bad: {
        while off < tlen {
            let Some(mut m) = m_gethdr(M_DONTWAIT, MT_HEADER) else {
                break 'bad Err(Errno::ENOBUFS);
            };
            if let Err(e) = m_dup_pkthdr(&mut m, &m0, M_DONTWAIT) {
                break 'bad Err(e);
            }
            m.data_add(MAX_LINKHDR.load(Ordering::Relaxed) as isize);
            let mut mhip = ip;
            let mhlen = if hlen as usize > IP_HDR_LEN {
                m.m_len().set(hlen as u32);
                let n = match m.data_mut() {
                    Some(d) => ip_optcopy(&opts[..nopts], &mut d[IP_HDR_LEN..]),
                    None => kpanic(format_args!("ip_fragment: a fresh mbuf read-only")),
                };
                let mhlen = (n + IP_HDR_LEN) as i32;
                mhip.set_ip_hl((mhlen >> 2) as u8);
                mhlen
            } else {
                IP_HDR_LEN as i32
            };
            m.m_len().set(mhlen as u32);

            let mut moff = ((off - hlen) >> 3) as u16 + (ntohs(ip.ip_off) & !IP_MF);
            if ip.ip_off & htons(IP_MF) != 0 {
                moff |= IP_MF;
            }
            if off + len >= tlen {
                len = tlen - off;
            } else {
                moff |= IP_MF;
            }
            mhip.ip_off = htons(moff);

            m.m_pkthdr().len.set(mhlen + len);
            mhip.ip_len = htons((mhlen + len) as u16);
            ip_store(&mut m, &mhip);
            let Some(n) = m_copym(&mut m0, off, len, M_NOWAIT) else {
                break 'bad Err(Errno::ENOBUFS);
            };
            drop(m.m_next.set(n));

            in_hdr_cksum_out(&mut m, Some(ifp));
            ml_enqueue(&mut frags, m);
            off += len;
        }
        Ok(())
    };
    if let Err(e) = built {
        ipstat_inc(IpstatCounters::IpsOdropped);
        return Err(e);
    }

    // Update first fragment by trimming what's been copied out and updating header, then send
    // each fragment (in order).
    if hlen + firstlen < tlen {
        m_adj(&m0, hlen + firstlen - tlen);
        ip.ip_off |= htons(IP_MF);
    }
    ip.ip_len = htons(m0.m_pkthdr().len.get() as u16);
    ip_store(&mut m0, &ip);

    in_hdr_cksum_out(&mut m0, Some(ifp));

    ml_enqueue(ml, m0);
    ml_enlist(ml, &mut frags);
    ipstat_add(IpstatCounters::IpsOfragments, u64::from(ml_len(ml)));
    Ok(())
}

/// `ip_insertoptions(m, opt, phlen)`: the IP options `opt` (the bytes of a `struct ipoption`:
/// the first hop, then the list) inserted into the preformed packet `m`. Adjusts the IP destination as
/// required for IP source routing, as indicated by a non-zero `in_addr` at the start of the
/// options; `phlen` gets the new header length. The packet is returned unchanged when the
/// options do not fit (`phlen` untouched: the C's "XXX should fail"), or with the header
/// alone (`phlen` 20) when they exceed a header.
pub fn ip_insertoptions(m: MbufBox, opt: &[u8], phlen: &mut i32) -> MbufBox {
    let mut m = m;
    let p = opt;
    // `opt->m_len - sizeof(p->ipopt_dst)` in the C's unsigned arithmetic: a shorter mbuf
    // wraps past `IP_MAXPACKET`.
    let Some(optlen) = p.len().checked_sub(size_of::<InAddr>()) else {
        return m;
    };
    let mut ip = ip_load(&m);
    if optlen + usize::from(ntohs(ip.ip_len)) > IP_MAXPACKET {
        return m; // XXX should fail
    }

    // check if options will fit to IP header
    if optlen + IP_HDR_LEN > (0x0f << 2) {
        *phlen = IP_HDR_LEN as i32;
        return m;
    }

    let first = u32::from_ne_bytes([p[0], p[1], p[2], p[3]]);
    if first != 0 {
        ip.ip_dst = InAddr { s_addr: first };
        // The C writes it in place before it may give up on a header mbuf below.
        ip_store(&mut m, &ip);
    }
    if m.ext().is_some() || m.data_off() < optlen {
        let Some(mut n) = m_gethdr(M_DONTWAIT, MT_HEADER) else {
            return m;
        };
        m_move_hdr(&mut n, &mut m);
        let ph = n.m_pkthdr();
        ph.len.set(ph.len.get() + optlen as i32);
        m.m_len().set(m.m_len().get() - IP_HDR_LEN as u32);
        m.data_add(IP_HDR_LEN as isize);
        n.m_len().set((optlen + IP_HDR_LEN) as u32);
        n.data_add(MAX_LINKHDR.load(Ordering::Relaxed) as isize);
        drop(n.m_next.set(m));
        m = n;
    } else {
        m.data_add(-(optlen as isize));
        m.m_len().set(m.m_len().get() + optlen as u32);
        let ph = m.m_pkthdr();
        ph.len.set(ph.len.get() + optlen as i32);
    }
    // The header at the new start (the C's memcpy/memmove of the old header), the options
    // after it.
    *phlen = (IP_HDR_LEN + optlen) as i32;
    ip.ip_len = htons(ntohs(ip.ip_len) + optlen as u16);
    ip_store(&mut m, &ip);
    let list = &p[size_of::<InAddr>()..];
    match m.data_mut() {
        Some(d) => d[IP_HDR_LEN..IP_HDR_LEN + optlen].copy_from_slice(list),
        None => {
            let _ = m_copyback(&mut m, IP_HDR_LEN as i32, list, M_DONTWAIT);
        }
    }
    m
}

/// `ip_optcopy(ip, jp)`: the options `cp` of a header copied to `dp` (the options of a later
/// fragment), omitting those not copied during fragmentation, and padded with `IPOPT_EOL` to
/// a multiple of 4. Returns the length of the copy. `dp` must hold at least as many bytes as
/// `cp` rounded up to 4.
pub fn ip_optcopy(cp: &[u8], dp: &mut [u8]) -> usize {
    let mut d = 0;
    let mut c = 0;
    while c < cp.len() {
        let cnt = cp.len() - c;
        let opt = cp[c];
        if opt == IPOPT_EOL {
            break;
        }
        if opt == IPOPT_NOP {
            // Preserve for IP mcast tunnel's LSRR alignment.
            dp[d] = IPOPT_NOP;
            d += 1;
            c += 1;
            continue;
        }
        // An option without its length byte (the module's deviations).
        let Some(&ol) = cp.get(c + IPOPT_OLEN) else {
            break;
        };
        // bogus lengths should have been caught by ip_dooptions
        let optlen = usize::from(ol).min(cnt);
        if optlen == 0 {
            break;
        }
        if ipopt_copied(opt) != 0 {
            dp[d..d + optlen].copy_from_slice(&cp[c..c + optlen]);
            d += optlen;
        }
        c += optlen;
    }
    while d & 0x3 != 0 {
        dp[d] = IPOPT_EOL;
        d += 1;
    }
    d
}

/// The `int` value of a socket option (`*mtod(m, int *)` once `m->m_len == sizeof(int)`):
/// `None` without a value or of another length.
#[inline]
fn opt_int(m: Option<&Mbuf>) -> Option<i32> {
    match m?.data() {
        &[a, b, c, d] => Some(i32::from_ne_bytes([a, b, c, d])),
        _ => None,
    }
}

/// The answer `b` written into the option mbuf `m` of a `getsockopt` (`m->m_len = len`, then
/// the value at `mtod(m)`): `sogetopt` hands over a writable mbuf of `MLEN` bytes.
fn opt_put(m: &mut Mbuf, b: &[u8]) {
    m.m_len().set(b.len() as u32);
    match m.data_mut() {
        Some(d) => d.copy_from_slice(b),
        None => kpanic(format_args!("ip_ctloutput: a read-only option mbuf")),
    }
}

/// `ip_ctloutput(op, so, level, optname, m)`: IP socket option processing, on the control
/// block `inp` of the socket (`sotoinpcb(so)`) as `curproc` asks it, the net lock held
/// (`net`). `options` is the socket's `inp_options`, which `bsd`'s caller lends for the call
/// (the module's Redesign). `m` holds the value of a `PRCO_SETOPT` (`None` for none) and
/// receives the answer of a `PRCO_GETOPT`.
pub fn ip_ctloutput<G: ProcGlue>(
    op: i32,
    inp: &Inpcb<G>,
    options: &mut Option<MbufBox>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    if level != IPPROTO_IP {
        return Err(Errno::EINVAL);
    }

    G::with_curproc(|p| {
        let Some(p) = p else {
            kpanic(format_args!("ip_ctloutput: no curproc")); // XXX
        };
        let rtableid = p.process().ps_rtableid.load(Ordering::Relaxed);
        match op {
            PRCO_SETOPT => ip_setopt(inp, options, p, rtableid, optname, m.as_deref(), net),
            PRCO_GETOPT => match m {
                Some(m) => ip_getopt(inp, options, optname, m),
                None => kpanic(format_args!("ip_ctloutput: PRCO_GETOPT without an mbuf")),
            },
            _ => Ok(()),
        }
    })
}

/// `ip_ctloutput`'s `PRCO_SETOPT` half.
fn ip_setopt<G: ProcGlue>(
    inp: &Inpcb<G>,
    options: &mut Option<MbufBox>,
    p: &Thread<G>,
    rtableid: u32,
    optname: i32,
    m: Option<&Mbuf>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    match optname {
        IP_OPTIONS => ip_pcbopts(options, m),

        IP_TOS | IP_TTL | IP_MINTTL | IP_RECVOPTS | IP_RECVRETOPTS | IP_RECVDSTADDR | IP_RECVIF
        | IP_RECVTTL | IP_RECVDSTPORT | IP_RECVRTABLE | IP_IPSECFLOWINFO => {
            let Some(optval) = opt_int(m) else {
                return Err(Errno::EINVAL);
            };
            let optset = |bit: i32| {
                if optval != 0 {
                    inp.set_flags(bit);
                } else {
                    inp.clear_flags(bit);
                }
            };
            match optname {
                IP_TOS => {
                    let mut ip = inp.inp_ip.get();
                    ip.ip_tos = optval as u8;
                    inp.inp_ip.set(ip);
                }
                IP_TTL => {
                    let ttl = if optval > 0 && optval <= i32::from(MAXTTL) {
                        optval as u8
                    } else if optval == -1 {
                        IP_DEFTTL.load(Ordering::Relaxed) as u8
                    } else {
                        return Err(Errno::EINVAL);
                    };
                    let mut ip = inp.inp_ip.get();
                    ip.ip_ttl = ttl;
                    inp.inp_ip.set(ip);
                }
                IP_MINTTL => {
                    if !(0..=i32::from(MAXTTL)).contains(&optval) {
                        return Err(Errno::EINVAL);
                    }
                    inp.inp_ip_minttl.set(optval as u8);
                }
                IP_RECVOPTS => optset(INP_RECVOPTS),
                IP_RECVRETOPTS => optset(INP_RECVRETOPTS),
                IP_RECVDSTADDR => optset(INP_RECVDSTADDR),
                IP_RECVIF => optset(INP_RECVIF),
                IP_RECVTTL => optset(INP_RECVTTL),
                IP_RECVDSTPORT => optset(INP_RECVDSTPORT),
                IP_RECVRTABLE => optset(INP_RECVRTABLE),
                _ => optset(INP_IPSECFLOWINFO), // IP_IPSECFLOWINFO
            }
            Ok(())
        }

        IP_MULTICAST_IF | IP_MULTICAST_TTL | IP_MULTICAST_LOOP | IP_ADD_MEMBERSHIP
        | IP_DROP_MEMBERSHIP => {
            ip_setmoptions(optname, &mut inp.moptions(), m, inp.inp_rtableid.get(), net)
        }

        IP_PORTRANGE => {
            let Some(optval) = opt_int(m) else {
                return Err(Errno::EINVAL);
            };
            match optval {
                IP_PORTRANGE_DEFAULT => inp.clear_flags(INP_LOWPORT | INP_HIGHPORT),
                IP_PORTRANGE_HIGH => {
                    inp.clear_flags(INP_LOWPORT);
                    inp.set_flags(INP_HIGHPORT);
                }
                IP_PORTRANGE_LOW => {
                    inp.clear_flags(INP_HIGHPORT);
                    inp.set_flags(INP_LOWPORT);
                }
                _ => return Err(Errno::EINVAL),
            }
            Ok(())
        }

        // IPSEC is configured.
        IP_AUTH_LEVEL | IP_ESP_TRANS_LEVEL | IP_ESP_NETWORK_LEVEL | IP_IPCOMP_LEVEL => {
            let Some(optval) = opt_int(m) else {
                return Err(Errno::EINVAL);
            };
            if !(IPSEC_LEVEL_BYPASS..=IPSEC_LEVEL_UNIQUE).contains(&optval) {
                return Err(Errno::EINVAL);
            }
            let default = match optname {
                IP_AUTH_LEVEL => IPSEC_AUTH_LEVEL_DEFAULT,
                IP_ESP_TRANS_LEVEL => IPSEC_ESP_TRANS_LEVEL_DEFAULT,
                IP_ESP_NETWORK_LEVEL => IPSEC_ESP_NETWORK_LEVEL_DEFAULT,
                _ => IPSEC_IPCOMP_LEVEL_DEFAULT, // IP_IPCOMP_LEVEL
            };
            // Below the system's default only for the superuser.
            if optval < default && suser(p).is_err() {
                return Err(Errno::EACCES);
            }
            let mut sl = inp.inp_seclevel.get();
            let level = optval as u8;
            match optname {
                IP_AUTH_LEVEL => sl.sl_auth = level,
                IP_ESP_TRANS_LEVEL => sl.sl_esp_trans = level,
                IP_ESP_NETWORK_LEVEL => sl.sl_esp_network = level,
                _ => sl.sl_ipcomp = level, // IP_IPCOMP_LEVEL
            }
            inp.inp_seclevel.set(sl);
            Ok(())
        }

        IP_IPSEC_LOCAL_ID | IP_IPSEC_REMOTE_ID => Err(Errno::EOPNOTSUPP),

        SO_RTABLE => {
            let Some(&[a, b, c, d, ..]) = m.map(Mbuf::data) else {
                return Err(Errno::EINVAL);
            };
            let rtid = u32::from_ne_bytes([a, b, c, d]);
            if inp.inp_rtableid.get() == rtid {
                return Ok(());
            }
            // needs privileges to switch when already set
            if rtableid != rtid && rtableid != 0 {
                suser(p)?;
            }
            in_pcbset_rtableid(inp, rtid)
        }

        IP_PIPEX => match opt_int(m) {
            Some(v) => {
                inp.inp_pipex.set(v);
                Ok(())
            }
            None => Err(Errno::EINVAL),
        },

        _ => Err(Errno::ENOPROTOOPT),
    }
}

/// `ip_ctloutput`'s `PRCO_GETOPT` half: the answer in `m`.
fn ip_getopt<G: ProcGlue>(
    inp: &Inpcb<G>,
    options: &Option<MbufBox>,
    optname: i32,
    m: &mut Mbuf,
) -> Result<(), Errno> {
    let optbit = |bit: i32| i32::from(inp.has_flags(bit));
    let optval = match optname {
        IP_OPTIONS | IP_RETOPTS => {
            match options {
                Some(opts) => opt_put(m, opts.data()),
                None => m.m_len().set(0),
            }
            return Ok(());
        }

        IP_TOS => i32::from(inp.inp_ip.get().ip_tos),
        IP_TTL => i32::from(inp.inp_ip.get().ip_ttl),
        IP_MINTTL => i32::from(inp.inp_ip_minttl.get()),
        IP_IPDEFTTL => IP_DEFTTL.load(Ordering::Relaxed),
        IP_RECVOPTS => optbit(INP_RECVOPTS),
        IP_RECVRETOPTS => optbit(INP_RECVRETOPTS),
        IP_RECVDSTADDR => optbit(INP_RECVDSTADDR),
        IP_RECVIF => optbit(INP_RECVIF),
        IP_RECVTTL => optbit(INP_RECVTTL),
        IP_RECVDSTPORT => optbit(INP_RECVDSTPORT),
        IP_RECVRTABLE => optbit(INP_RECVRTABLE),
        IP_IPSECFLOWINFO => optbit(INP_IPSECFLOWINFO),

        IP_MULTICAST_IF | IP_MULTICAST_TTL | IP_MULTICAST_LOOP | IP_ADD_MEMBERSHIP
        | IP_DROP_MEMBERSHIP => return ip_getmoptions(optname, inp.moptions_out().as_ref(), m),

        IP_PORTRANGE => {
            if inp.has_flags(INP_HIGHPORT) {
                IP_PORTRANGE_HIGH
            } else if inp.has_flags(INP_LOWPORT) {
                IP_PORTRANGE_LOW
            } else {
                0
            }
        }

        // IPSEC is configured.
        IP_AUTH_LEVEL | IP_ESP_TRANS_LEVEL | IP_ESP_NETWORK_LEVEL | IP_IPCOMP_LEVEL => {
            let sl = inp.inp_seclevel.get();
            i32::from(match optname {
                IP_AUTH_LEVEL => sl.sl_auth,
                IP_ESP_TRANS_LEVEL => sl.sl_esp_trans,
                IP_ESP_NETWORK_LEVEL => sl.sl_esp_network,
                _ => sl.sl_ipcomp, // IP_IPCOMP_LEVEL
            })
        }

        IP_IPSEC_LOCAL_ID | IP_IPSEC_REMOTE_ID => return Err(Errno::EOPNOTSUPP),
        SO_RTABLE => {
            opt_put(m, &inp.inp_rtableid.get().to_ne_bytes());
            return Ok(());
        }
        IP_PIPEX => inp.inp_pipex.get(),
        _ => return Err(Errno::ENOPROTOOPT),
    };
    opt_put(m, &optval.to_ne_bytes());
    Ok(())
}

/// `ip_pcbopts(pcbopt, m)`: sets up the IP options of a pcb for insertion in output packets,
/// from the options `m` holds: stored in an mbuf in `pcbopt` (a `struct ipoption`), the first
/// hop of a source route moved before them. The old options are freed first; no `m`, or an
/// empty one, only turns them off.
pub fn ip_pcbopts(pcbopt: &mut Option<MbufBox>, m: Option<&Mbuf>) -> Result<(), Errno> {
    // turn off any old options
    drop(pcbopt.take());
    let Some(src) = m.map(Mbuf::data).filter(|d| !d.is_empty()) else {
        // Only turning off any previous options.
        return Ok(());
    };

    let len = src.len();
    if !len.is_multiple_of(size_of::<i32>()) || len > MAX_IPOPTLEN + IN_ADDR_LEN {
        return Err(Errno::EINVAL);
    }

    // Don't sleep because NET_LOCK() is hold.
    let Some(mut n) = m_get(M_DONTWAIT, i32::from(MT_SOOPTS)) else {
        return Err(Errno::ENOBUFS);
    };
    // The struct ipoption, zeroed (0 = IPOPT_EOL, needed for padding): the first hop, then
    // the option list, which the walk may fill past MAX_IPOPTLEN before the final check
    // (by at most the four bytes of a source route's first hop the input carries).
    let mut p = [0u8; IN_ADDR_LEN + MAX_IPOPTLEN + IN_ADDR_LEN];
    let (dst, list) = p.split_at_mut(IN_ADDR_LEN);

    let mut off = 0usize;
    let mut cp = 0usize;
    let mut cnt = len;
    while cnt > 0 {
        let opt = src[cp + IPOPT_OPTVAL];

        let mut optlen = if opt == IPOPT_NOP || opt == IPOPT_EOL {
            1
        } else {
            if cnt < IPOPT_OLEN + 1 {
                return Err(Errno::EINVAL);
            }
            let optlen = usize::from(src[cp + IPOPT_OLEN]);
            if optlen < IPOPT_OLEN + 1 || optlen > cnt {
                return Err(Errno::EINVAL);
            }
            optlen
        };
        if opt == IPOPT_LSRR || opt == IPOPT_SSRR {
            // user process specifies route as:
            //	->A->B->C->D
            // D must be our final destination (but we can't check that since we may not
            // have connected yet). A is first hop destination, which doesn't appear in
            // actual IP option, but is stored before the options.
            if optlen < usize::from(IPOPT_MINOFF) - 1 + IN_ADDR_LEN {
                return Err(Errno::EINVAL);
            }

            // Optlen is smaller because first address is popped. Cnt and cp will be adjusted
            // a bit later to reflect this.
            optlen -= IN_ADDR_LEN;
            list[off + IPOPT_OPTVAL] = opt;
            list[off + IPOPT_OLEN] = optlen as u8;

            // Move first hop before start of options.
            dst.copy_from_slice(&src[cp + IPOPT_OFFSET..cp + IPOPT_OFFSET + IN_ADDR_LEN]);
            cp += IN_ADDR_LEN;
            cnt -= IN_ADDR_LEN;
            // Then copy rest of options
            let rest = optlen - IPOPT_OFFSET;
            list[off + IPOPT_OFFSET..off + IPOPT_OFFSET + rest]
                .copy_from_slice(&src[cp + IPOPT_OFFSET..cp + IPOPT_OFFSET + rest]);
        } else {
            list[off..off + optlen].copy_from_slice(&src[cp..cp + optlen]);
        }
        off += optlen;
        cp += optlen;
        cnt -= optlen;

        if opt == IPOPT_EOL {
            break;
        }
    }
    // pad options to next word, since p was zeroed just adjust off
    let off = off.next_multiple_of(size_of::<i32>());
    let nlen = IN_ADDR_LEN + off;
    if nlen > size_of::<Ipoption>() {
        return Err(Errno::EINVAL);
    }
    opt_put(&mut n, &p[..nlen]);

    *pcbopt = Some(n);
    Ok(())
}

/// The `struct ip_mreqn` an `ip_mreq` or `ip_mreqn` option holds, zero-extended (the C's
/// `memset` and `memcpy` of `m_len` bytes), from its words.
fn mreqn_of(d: &[u8]) -> IpMreqn {
    let word = |at: usize| match d.get(at..at + 4) {
        Some(&[a, b, c, e]) => u32::from_ne_bytes([a, b, c, e]),
        _ => 0,
    };
    IpMreqn {
        imr_multiaddr: InAddr { s_addr: word(0) },
        imr_address: InAddr { s_addr: word(4) },
        imr_ifindex: word(8) as i32,
    }
}

/// The `sockaddr_in` image of `addr`, port 0 (`sin_len`, `AF_INET`, the address).
#[inline]
fn sin_image(addr: InAddr) -> [u8; SIN_LEN] {
    let mut sin = [0u8; SIN_LEN];
    sin[0] = SIN_LEN as u8;
    sin[1] = AF_INET;
    sin[SIN_ADDR..SIN_ADDR + 4].copy_from_slice(&addr.s_addr.to_ne_bytes());
    sin
}

/// `ip_multicast_if(mreq, rtableid, &ifidx)`: the interface of the request `mreq`: the index
/// userland gave, else the interface of the route to the group (no interface address) or of
/// the local address given; `EADDRNOTAVAIL` without a valid route (or a local one).
pub fn ip_multicast_if(mreq: &IpMreqn, rtableid: u32, net: &NetHeld<'_>) -> Result<u32, Errno> {
    // In case userland provides the imr_ifindex use this as interface. If no interface
    // address was provided, use the interface of the route to the given multicast address.
    if mreq.imr_ifindex != 0 {
        return Ok(mreq.imr_ifindex as u32);
    }
    let (addr, flags) = if mreq.imr_address.s_addr == INADDR_ANY {
        (mreq.imr_multiaddr, RT_RESOLVE)
    } else {
        (mreq.imr_address, 0)
    };
    let sin = sin_image(addr);
    let Some(dst) = SockaddrView::new(&sin) else {
        return Err(Errno::EADDRNOTAVAIL);
    };
    let rt = rtalloc(dst, flags, rtableid, net);
    match rt.as_deref() {
        Some(rt)
            if rtisvalid(Some(rt)) && (flags == RT_RESOLVE || rt.rt_flags.isset(RTF_LOCAL)) =>
        {
            Ok(rt.rt_ifidx.get())
        }
        _ => Err(Errno::EADDRNOTAVAIL),
    }
}

/// `ip_setmoptions(optname, imop, m, rtableid)`: sets the IP multicast options of a socket
/// (`*imop`, made here with the C's defaults when it has none) in response to user
/// setsockopt(), and frees them once every option is back to its default.
pub fn ip_setmoptions(
    optname: i32,
    imop: &mut Option<MBox<IpMoptions>>,
    m: Option<&Mbuf>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let imo = match imop {
        Some(imo) => imo,
        None => {
            // No multicast option buffer attached to the pcb; allocate one and initialize to
            // default values.
            let Some(imo) = MBox::new(
                M_IPMOPTS,
                M_WAITOK,
                IpMoptions::output(0, IP_DEFAULT_MULTICAST_TTL, IP_DEFAULT_MULTICAST_LOOP),
            ) else {
                kpanic(format_args!("ip_setmoptions: malloc(M_WAITOK)"));
            };
            let imo = imop.insert(imo);
            let Some(immp) =
                MArray::new(M_IPMOPTS, M_WAITOK, usize::from(IP_MIN_MEMBERSHIPS), |_| {
                    None
                })
            else {
                kpanic(format_args!("ip_setmoptions: mallocarray(M_WAITOK)"));
            };
            imo.imo_membership = Some(immp);
            imo
        }
    };

    let error = match optname {
        IP_MULTICAST_IF => set_multicast_if(imo, m, rtableid, net),

        // Set the IP time-to-live for outgoing multicast packets.
        IP_MULTICAST_TTL => match m.map(Mbuf::data) {
            Some(&[ttl]) => {
                imo.imo_ttl = ttl;
                Ok(())
            }
            _ => Err(Errno::EINVAL),
        },

        // Set the loopback flag for outgoing multicast packets. Must be zero or one.
        IP_MULTICAST_LOOP => match m.map(Mbuf::data) {
            Some(&[lp]) if lp <= 1 => {
                imo.imo_loop = lp;
                Ok(())
            }
            _ => Err(Errno::EINVAL),
        },

        IP_ADD_MEMBERSHIP => add_membership(imo, m, rtableid, net),
        IP_DROP_MEMBERSHIP => drop_membership(imo, m, rtableid, net),

        _ => Err(Errno::EOPNOTSUPP),
    };

    // If all options have default values, no need to keep the data.
    if imo.imo_ifidx == 0
        && imo.imo_ttl == IP_DEFAULT_MULTICAST_TTL
        && imo.imo_loop == IP_DEFAULT_MULTICAST_LOOP
        && imo.imo_num_memberships == 0
    {
        // free(imo->imo_membership), then free(*imop).
        drop(imop.take());
    }

    error
}

/// `ip_setmoptions`'s `IP_MULTICAST_IF`: selects the interface for outgoing multicast
/// packets, by its index (an `ip_mreqn`) or by one of its local addresses (an `in_addr` or
/// the `ip_mreq`'s interface address); `INADDR_ANY` removes a previous selection.
fn set_multicast_if(
    imo: &mut IpMoptions,
    m: Option<&Mbuf>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let Some(d) = m.map(Mbuf::data) else {
        return Err(Errno::EINVAL);
    };
    let addr = if d.len() == IN_ADDR_LEN {
        mreqn_of(d).imr_multiaddr
    } else if d.len() == size_of::<IpMreq>() || d.len() == size_of::<IpMreqn>() {
        let mreqn = mreqn_of(d);
        // If an interface index is given use this index to set the imo_ifidx but check first
        // that the interface actually exists. In the other case just set the addr to the
        // imr_address and fall through to the regular code.
        if mreqn.imr_ifindex != 0 {
            return match IfRef::get(mreqn.imr_ifindex as u32) {
                Some(ifp) if ifp.if_rdomain.get() == rtable_l2(rtableid) => {
                    imo.imo_ifidx = ifp.if_index.get() as u16;
                    Ok(())
                }
                _ => Err(Errno::EADDRNOTAVAIL),
            };
        }
        mreqn.imr_address
    } else {
        return Err(Errno::EINVAL);
    };
    // INADDR_ANY is used to remove a previous selection. When no interface is selected, a
    // default one is chosen every time a multicast packet is sent.
    if addr.s_addr == INADDR_ANY {
        imo.imo_ifidx = 0;
        return Ok(());
    }
    // The selected interface is identified by its local IP address. Find the interface and
    // confirm that it supports multicasting.
    let ifp = ifa_ifwithaddr(net, &sin_image(addr), rtableid)
        .and_then(InIfaddr::of)
        .and_then(|ia| ia.ia_ifp().get());
    match ifp {
        Some(ifp) if ifp.if_flags.get() & IFF_MULTICAST != 0 => {
            imo.imo_ifidx = ifp.if_index.get() as u16;
            Ok(())
        }
        _ => Err(Errno::EADDRNOTAVAIL),
    }
}

/// The request of an `IP_ADD_MEMBERSHIP` or `IP_DROP_MEMBERSHIP` (an `ip_mreq` or an
/// `ip_mreqn`), whose group must be a valid IP multicast address: `EINVAL` otherwise.
fn membership_request(m: Option<&Mbuf>) -> Result<IpMreqn, Errno> {
    let Some(d) = m
        .map(Mbuf::data)
        .filter(|d| d.len() == size_of::<IpMreq>() || d.len() == size_of::<IpMreqn>())
    else {
        return Err(Errno::EINVAL);
    };
    let mreqn = mreqn_of(d);
    if !in_multicast(mreqn.imr_multiaddr.s_addr) {
        return Err(Errno::EINVAL);
    }
    Ok(mreqn)
}

/// `ip_setmoptions`'s `IP_ADD_MEMBERSHIP`: adds a multicast group membership, growing the
/// vector to the next power of two minus one when it is full, up to `IP_MAX_MEMBERSHIPS`.
fn add_membership(
    imo: &mut IpMoptions,
    m: Option<&Mbuf>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mreqn = membership_request(m)?;
    let ifidx = ip_multicast_if(&mreqn, rtableid, net)?;

    // See if we found an interface, and confirm that it supports multicast.
    let Some(ifp) = IfRef::get(ifidx).filter(|ifp| {
        ifp.if_rdomain.get() == rtable_l2(rtableid) && ifp.if_flags.get() & IFF_MULTICAST != 0
    }) else {
        return Err(Errno::EADDRNOTAVAIL);
    };

    // See if the membership already exists or if all the membership slots are full.
    let group = mreqn.imr_multiaddr;
    let num = usize::from(imo.imo_num_memberships);
    let exists = (0..num).any(|i| {
        imo.membership(i)
            .is_some_and(|inm| inm.inm_ifidx().get() == ifidx && inm.inm_addr() == group)
    });
    if exists {
        return Err(Errno::EADDRINUSE);
    }
    if imo.imo_num_memberships == imo.imo_max_memberships() {
        // Resize the vector to next power-of-two minus 1. If the size would exceed the
        // maximum then we know we've really run out of entries. Otherwise, we reallocate the
        // vector.
        let newmax = (usize::from(imo.imo_max_memberships()) + 1) * 2 - 1;
        let old = imo.imo_membership.as_deref().unwrap_or(&[]);
        let nmships = (newmax <= usize::from(IP_MAX_MEMBERSHIPS))
            .then(|| {
                MArray::new(M_IPMOPTS, M_NOWAIT, newmax, |i| {
                    old.get(i).copied().flatten()
                })
            })
            .flatten();
        let Some(nmships) = nmships else {
            return Err(Errno::ENOBUFS);
        };
        // The old vector freed once copied.
        imo.imo_membership = Some(nmships);
    }
    // Everything looks good; add a new record to the multicast address list for the given
    // interface.
    let Some(inm) = in_addmulti(&group, ifp.ifp(), net) else {
        return Err(Errno::ENOBUFS);
    };
    let Some(slot) = imo
        .imo_membership
        .as_deref_mut()
        .and_then(|v| v.get_mut(num))
    else {
        kpanic(format_args!(
            "ip_setmoptions: membership {num} out of the vector"
        ));
    };
    *slot = Some(inm);
    imo.imo_num_memberships += 1;
    Ok(())
}

/// `ip_setmoptions`'s `IP_DROP_MEMBERSHIP`: drops a multicast group membership, on the
/// interface the request names (any, when it names none).
fn drop_membership(
    imo: &mut IpMoptions,
    m: Option<&Mbuf>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mreqn = membership_request(m)?;

    // If an interface address was specified, get a pointer to its ifnet structure.
    let ifidx = ip_multicast_if(&mreqn, rtableid, net)?;

    // Find the membership in the membership array.
    let group = mreqn.imr_multiaddr;
    let num = usize::from(imo.imo_num_memberships);
    let Some((i, inm)) = (0..num).find_map(|i| {
        imo.membership(i)
            .filter(|inm| (ifidx == 0 || inm.inm_ifidx().get() == ifidx) && inm.inm_addr() == group)
            .map(|inm| (i, inm))
    }) else {
        return Err(Errno::EADDRNOTAVAIL);
    };
    // Give up the multicast address record to which the membership points.
    in_delmulti(inm, net);
    // Remove the gap in the membership array.
    if let Some(v) = imo.imo_membership.as_deref_mut() {
        v.copy_within(i + 1..num, i);
        v[num - 1] = None;
    }
    imo.imo_num_memberships -= 1;
    Ok(())
}

/// `ip_getmoptions(optname, imo, m)`: returns the IP multicast options of a socket (`imo`,
/// `None` when it has none: the defaults) in response to user getsockopt().
pub fn ip_getmoptions(optname: i32, imo: Option<&IpMoptions>, m: &mut Mbuf) -> Result<(), Errno> {
    match optname {
        IP_MULTICAST_IF => {
            let addr = imo
                .and_then(|imo| IfRef::get(u32::from(imo.imo_ifidx)))
                .and_then(|ifp| ifp_inaddr(&ifp))
                .map_or(INADDR_ANY, |a| a.s_addr);
            opt_put(m, &addr.to_ne_bytes());
            Ok(())
        }
        IP_MULTICAST_TTL => {
            opt_put(
                m,
                &[imo.map_or(IP_DEFAULT_MULTICAST_TTL, |imo| imo.imo_ttl)],
            );
            Ok(())
        }
        IP_MULTICAST_LOOP => {
            opt_put(
                m,
                &[imo.map_or(IP_DEFAULT_MULTICAST_LOOP, |imo| imo.imo_loop)],
            );
            Ok(())
        }
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// `ip_freemoptions(imo)`: discards the IP multicast options of a socket, leaving its groups.
pub fn ip_freemoptions(imo: Option<MBox<IpMoptions>>, net: &NetHeld<'_>) {
    let Some(imo) = imo else {
        return;
    };
    for i in 0..usize::from(imo.imo_num_memberships) {
        if let Some(inm) = imo.membership(i) {
            in_delmulti(inm, net);
        }
    }
    // free(imo->imo_membership), then free(imo).
    drop(imo);
}

/// `ip_mloopback(ifp, m, dst)`: a copy of the IP multicast packet `m` looped back to the input
/// queue of `ifp` (called from `ip_output`).
pub fn ip_mloopback(ifp: &'static Ifnet, m: &Mbuf, dst: &[u8; SIN_LEN]) {
    if let Some(mut copym) = m_dup_pkt(m, MAX_LINKHDR.load(Ordering::Relaxed) as u32, M_DONTWAIT) {
        // We don't bother to fragment if the IP length is greater than the interface's MTU.
        // Can this possibly matter?
        in_hdr_cksum_out(&mut copym, None);
        let _ = if_input_local(ifp, copym, dst[1], None);
    }
}

/// `in_hdr_cksum_out(m, ifp)`: the IP header checksum, by the interface if it can or in
/// software.
#[inline]
pub fn in_hdr_cksum_out(m: &mut Mbuf, ifp: Option<&Ifnet>) {
    cksum_store(m, IP_SUM_OFFSET, 0);
    let csum = &m.m_pkthdr().csum_flags;
    if in_ifcap_cksum(m, ifp, IFCAP_CSUM_IPv4) {
        csum.set(csum.get() | M_IPV4_CSUM_OUT);
    } else {
        csum.set(csum.get() & !M_IPV4_CSUM_OUT);
        ipstat_inc(IpstatCounters::IpsOutswcsum);
        // `ip->ip_hl << 2`: the low nibble of the header's first byte.
        let hl = m.data().first().map_or(0, |b| i32::from(b & 0x0f) << 2);
        let sum = in_cksum(m, hl);
        cksum_store(m, IP_SUM_OFFSET, sum);
    }
}

/// `in_cksum_phdr(src, dst, lenproto)`: computes the significant parts of the IPv4 checksum
/// pseudo-header for use in a delayed TCP/UDP checksum calculation.
#[inline]
fn in_cksum_phdr(src: u32, dst: u32, lenproto: u32) -> u16 {
    let mut sum: u32 = lenproto
        .wrapping_add(u32::from((src >> 16) as u16))
        .wrapping_add(u32::from(src as u16))
        .wrapping_add(u32::from((dst >> 16) as u16))
        .wrapping_add(u32::from(dst as u16));

    sum = u32::from((sum >> 16) as u16) + u32::from(sum as u16);

    if sum > 0xffff {
        sum -= 0xffff;
    }

    sum as u16
}

/// A 16-bit checksum written at byte `offset` of the packet: in the first mbuf when it holds
/// the two bytes and is writable, else through `m_copyback` (the C's, whose failure it
/// ignores).
#[inline]
fn cksum_store(m: &mut Mbuf, offset: usize, csum: u16) {
    if offset + size_of::<u16>() <= m.m_len().get() as usize
        && let Some(d) = m.data_mut()
    {
        d[offset..offset + 2].copy_from_slice(&csum.to_ne_bytes());
        return;
    }
    let _ = m_copyback(m, offset as i32, &csum.to_ne_bytes(), M_NOWAIT);
}

/// `in_delayed_cksum(m)`: processes a delayed payload checksum calculation.
pub fn in_delayed_cksum(m: &mut Mbuf) {
    let ip = ip_load(m);
    let mut offset = usize::from(ip.ip_hl()) << 2;
    let mut csum = in4_cksum(m, 0, offset as i32, m.m_pkthdr().len.get() - offset as i32);
    if csum == 0 && i32::from(ip.ip_p) == IPPROTO_UDP {
        csum = 0xffff;
    }

    match i32::from(ip.ip_p) {
        IPPROTO_TCP => offset += TH_SUM_OFFSET,
        IPPROTO_UDP => offset += UH_SUM_OFFSET,
        IPPROTO_ICMP => offset += ICMP_CKSUM_OFFSET,
        _ => return,
    }

    cksum_store(m, offset, csum);
}

/// `in_proto_cksum_out(m, ifp)`: the transport checksum of an outgoing packet: the pseudo
/// header for the hardware, or the whole checksum in software when the interface cannot.
pub fn in_proto_cksum_out(m: &mut Mbuf, ifp: Option<&Ifnet>) {
    let flags = m.m_pkthdr().csum_flags.get();
    if flags & (M_TCP_CSUM_OUT | M_UDP_CSUM_OUT | M_ICMP_CSUM_OUT) == 0 {
        return;
    }
    let ip = ip_load(m);

    // some hw and in_delayed_cksum need the pseudo header cksum
    let mut csum: u16 = 0;
    let mut offset = usize::from(ip.ip_hl()) << 2;
    if flags & M_TCP_TSO != 0 && in_ifcap_cksum(m, ifp, IFCAP_TSOv4) {
        csum = in_cksum_phdr(
            ip.ip_src.s_addr,
            ip.ip_dst.s_addr,
            htonl(u32::from(ip.ip_p)),
        );
    } else if flags & (M_TCP_CSUM_OUT | M_UDP_CSUM_OUT) != 0 {
        csum = in_cksum_phdr(
            ip.ip_src.s_addr,
            ip.ip_dst.s_addr,
            htonl(
                u32::from(ntohs(ip.ip_len))
                    .wrapping_sub(offset as u32)
                    .wrapping_add(u32::from(ip.ip_p)),
            ),
        );
    }
    match i32::from(ip.ip_p) {
        IPPROTO_TCP => offset += TH_SUM_OFFSET,
        IPPROTO_UDP => offset += UH_SUM_OFFSET,
        IPPROTO_ICMP => offset += ICMP_CKSUM_OFFSET,
        _ => {}
    }
    cksum_store(m, offset, csum);

    if flags & M_TCP_CSUM_OUT != 0 {
        if !in_ifcap_cksum(m, ifp, IFCAP_CSUM_TCPv4) || ip.ip_hl() != 5 {
            if let Some(i) = inet() {
                i.proto_outswcsum(true);
            }
            in_delayed_cksum(m);
            m.m_pkthdr().csum_flags.set(flags & !M_TCP_CSUM_OUT); // Clear
        }
    } else if flags & M_UDP_CSUM_OUT != 0 {
        if !in_ifcap_cksum(m, ifp, IFCAP_CSUM_UDPv4) || ip.ip_hl() != 5 {
            if let Some(i) = inet() {
                i.proto_outswcsum(false);
            }
            in_delayed_cksum(m);
            m.m_pkthdr().csum_flags.set(flags & !M_UDP_CSUM_OUT); // Clear
        }
    } else {
        // M_ICMP_CSUM_OUT, the third flag the test above found.
        in_delayed_cksum(m);
        m.m_pkthdr().csum_flags.set(flags & !M_ICMP_CSUM_OUT); // Clear
    }
}

/// `in_ifcap_cksum(m, ifp, ifcap)`: whether `ifp` computes checksum `ifcap` for `m` in
/// hardware.
#[inline]
pub fn in_ifcap_cksum(m: &Mbuf, ifp: Option<&Ifnet>, ifcap: u32) -> bool {
    let Some(ifp) = ifp else {
        return false;
    };
    if ifp.if_capabilities.get() & ifcap == 0 || ifp.if_bridgeidx.get() != 0 {
        return false;
    }
    // Simplex interface sends packet back without hardware cksum. Keep this check in sync
    // with the condition where ether_resolve() calls if_input_local().
    if m.flags() & M_BCAST != 0
        && ifp.if_flags.get() & IFF_SIMPLEX != 0
        && m.m_pkthdr().pf.routed.get() == 0
    {
        return false;
    }
    true
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for IPv4 output: fragmentation (offsets, flags, lengths, the id and the
    // checksum of each fragment), the option copy of the later fragments, option insertion,
    // the output checksums, and `ip_output` itself over a test interface (the header
    // completed, the source chosen, the refusals with their counters, fragments on the wire).

    use std::boxed::Box;
    use std::sync::Mutex;
    use std::vec;
    use std::vec::Vec;

    use hal::netlock::NETLOCK;
    use hal::sys::net_if::IFF_UP;
    use hal::sys::netinet_in::SockaddrIn;
    use hal::sys::netinet_ip::{IP_OFFMASK, IPOPT_LSRR, IPOPT_RR};
    use hal::sys::sockio::SIOCAIFADDR;

    use super::*;
    use crate::net::if_var::NetDriver;
    use crate::net::mbuf::{M_PKTHDR, m_copydata, m_devget, m_pullup, ml_dequeue};
    use crate::net::route::tests::testroutes_setup;
    use crate::netinet::ip_var::IPCOUNTERS;

    /// A packet of `len` bytes: an IP header of `hlen` (options from `opts`, padded with NOPs)
    /// from 10.0.2.15 to `dst`, UDP, and a patterned payload, in a chain of clusters with the
    /// header in the first mbuf.
    fn packet_bytes(len: usize, opts: &[u8], dst: [u8; 4]) -> Vec<u8> {
        let hlen = 20 + opts.len().div_ceil(4) * 4;
        let mut p = vec![0u8; len];
        p[0] = 0x40 | (hlen / 4) as u8;
        p[2..4].copy_from_slice(&(len as u16).to_be_bytes());
        p[8] = 64;
        p[9] = 17;
        p[12..16].copy_from_slice(&[10, 0, 2, 15]);
        p[16..20].copy_from_slice(&dst);
        p[20..20 + opts.len()].copy_from_slice(opts);
        for b in &mut p[20 + opts.len()..hlen] {
            *b = IPOPT_NOP;
        }
        for (i, b) in p[hlen..].iter_mut().enumerate() {
            *b = (i * 13 + 5) as u8;
        }
        p
    }

    /// The packet of `p`, its header (as its first byte says) in the first mbuf.
    fn mbuf_of(p: &[u8]) -> MbufBox {
        let hlen = (usize::from(p[0] & 0x0f) * 4).max(20);
        let m = m_devget(p, 0).expect("packet");
        m_pullup(m, hlen as i32).expect("pullup")
    }

    fn packet_to(len: usize, opts: &[u8], dst: [u8; 4]) -> MbufBox {
        mbuf_of(&packet_bytes(len, opts, dst))
    }

    fn packet(len: usize, opts: &[u8]) -> MbufBox {
        packet_to(len, opts, [10, 0, 2, 2])
    }

    fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut v = vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(m, 0, &mut v);
        v
    }

    /// The routing tests' fixture, and the IP ids' permutation made once (`ip_init`'s
    /// `ip_randomid_init`: an empty one has no id but 0, which `ip_randomid` never returns).
    fn setup() -> std::sync::MutexGuard<'static, ()> {
        static IDS: std::sync::Once = std::sync::Once::new();
        let g = testroutes_setup();
        IDS.call_once(crate::netinet::ip_id::ip_randomid_init);
        g
    }

    fn ips(c: IpstatCounters) -> u64 {
        IPCOUNTERS.read()[c as usize]
    }

    /// A driver that keeps the packets it is given and the flags they came with.
    struct CapDriver {
        sent: Mutex<Vec<(Vec<u8>, u16)>>,
    }

    impl CapDriver {
        fn count(&self) -> usize {
            self.sent.lock().expect("sent").len()
        }

        fn last(&self) -> (Vec<u8>, u16) {
            self.sent.lock().expect("sent").pop().expect("a packet")
        }
    }

    impl NetDriver for CapDriver {
        fn ioctl(&self, _ifp: &'static Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            Ok(())
        }
        fn output(
            &self,
            _ifp: &'static Ifnet,
            m: MbufBox,
            dst: SockaddrView<'_>,
            _rt: Option<&crate::net::route::Rtentry>,
        ) -> Result<(), Errno> {
            assert_eq!(dst.family(), AF_INET);
            self.sent
                .lock()
                .expect("sent")
                .push((bytes(&m), m.flags() & !M_PKTHDR));
            Ok(())
        }
    }

    /// An interface up with broadcast, MTU 1500, on the interface list, and address
    /// 10.0.2.15/24 (`ifconfig ipo0 inet 10.0.2.15/24`: its record and its routes). Called
    /// without the net lock, which `in_ioctl` takes.
    fn iface() -> (&'static Ifnet, &'static CapDriver) {
        let d: &'static CapDriver = Box::leak(Box::new(CapDriver {
            sent: Mutex::new(Vec::new()),
        }));
        let ifp = crate::net::if_::test_ifnet_with(b"ipo0", d);
        ifp.if_flags.set(IFF_UP | IFF_BROADCAST | IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        crate::net::if_::if_alloc_sadl(ifp);
        {
            let _w = NETLOCK.write();
            crate::net::if_::IFNETLIST.insert_tail(ifp);
        }
        let mut r = [0u8; 64];
        let sa = |b: [u8; 4]| SockaddrIn {
            sin_len: 16,
            sin_family: AF_INET,
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes(b),
            },
            ..SockaddrIn::default()
        };
        let _ = pod::put(&mut r, 16, &sa([10, 0, 2, 15]));
        let _ = pod::put(&mut r, 48, &sa([255, 255, 255, 0]));
        crate::netinet::in_::in_ioctl(SIOCAIFADDR, &mut r, Some(ifp), true).expect("an address");
        d.sent.lock().expect("sent").clear();
        (ifp, d)
    }

    #[test]
    fn fragments_cover_the_payload_with_valid_headers() {
        let _g = setup();
        let (ifp, _d) = crate::net::if_::test_ifnet(b"tst0");
        let len = 3000;
        let m = packet(len, &[]);
        let whole = bytes(&m);
        let mut ml = MbufList::new();
        let ofrag = ips(IpstatCounters::IpsOfragments);

        ip_fragment(m, &mut ml, ifp, 1500).expect("fragment");
        assert_eq!(ml_len(&ml), 3);
        assert_eq!(ips(IpstatCounters::IpsOfragments), ofrag + 3);

        let mut payload = Vec::new();
        let mut expect_off = 0usize;
        let mut n = 0;
        while let Some(f) = ml_dequeue(&mut ml) {
            let b = bytes(&f);
            let ip = ip_load(&f);
            assert_eq!(ip.ip_hl(), 5);
            assert_eq!(usize::from(ntohs(ip.ip_len)), b.len());
            assert!(b.len() <= 1500);
            assert_eq!(&b[4..6], &whole[4..6], "every fragment keeps the id");
            let off = ntohs(ip.ip_off);
            assert_eq!(usize::from(off & IP_OFFMASK) * 8, expect_off);
            let last = expect_off + b.len() - 20 >= len - 20;
            assert_eq!(off & IP_MF != 0, !last, "MF on every fragment but the last");
            assert_eq!(in_cksum(&f, 20), 0, "fragment {n} header checksum");
            payload.extend_from_slice(&b[20..]);
            expect_off += b.len() - 20;
            n += 1;
        }
        assert_eq!(
            payload,
            &whole[20..],
            "the fragments carry the payload in order"
        );
    }

    #[test]
    fn too_small_an_mtu_is_emsgsize() {
        let _g = setup();
        let (ifp, _d) = crate::net::if_::test_ifnet(b"tst0");
        let m = packet(100, &[]);
        let mut ml = MbufList::new();
        let dropped = ips(IpstatCounters::IpsOdropped);
        assert_eq!(ip_fragment(m, &mut ml, ifp, 27), Err(Errno::EMSGSIZE));
        assert_eq!(ml_len(&ml), 0, "the packet was freed");
        assert_eq!(ips(IpstatCounters::IpsOdropped), dropped + 1);
    }

    #[test]
    fn later_fragments_keep_only_the_copied_options() {
        // Record route (not copied) then loose source route (copied), a NOP and the end.
        let opts = [
            IPOPT_RR, 7, 4, 0, 0, 0, 0, IPOPT_LSRR, 7, 4, 10, 0, 2, 9, IPOPT_NOP, IPOPT_EOL,
        ];
        let mut out = [0xeeu8; 16];
        let n = ip_optcopy(&opts, &mut out);
        assert_eq!(
            n, 8,
            "the source route and the NOP after it (kept for alignment)"
        );
        assert_eq!(&out[..7], &opts[7..14]);
        assert_eq!(out[7], IPOPT_NOP);
        // Padding to a multiple of 4; a length past the list is cut to it; an option without
        // its length byte or of length 0 ends the copy.
        let mut out = [0xeeu8; 8];
        assert_eq!(ip_optcopy(&[IPOPT_LSRR, 3, 4, IPOPT_EOL], &mut out), 4);
        assert_eq!(&out[..4], &[IPOPT_LSRR, 3, 4, IPOPT_EOL]);
        assert_eq!(ip_optcopy(&[IPOPT_LSRR, 9, 4, 0], &mut out), 4);
        assert_eq!(&out[..4], &[IPOPT_LSRR, 9, 4, 0]);
        let nops = [IPOPT_NOP, IPOPT_NOP, IPOPT_NOP, IPOPT_LSRR];
        assert_eq!(ip_optcopy(&nops, &mut out), 4);
        assert_eq!(&out[..4], &[IPOPT_NOP, IPOPT_NOP, IPOPT_NOP, IPOPT_EOL]);
        assert_eq!(ip_optcopy(&[IPOPT_LSRR, 0, 4, 0], &mut out), 0);
    }

    #[test]
    fn fragments_after_the_first_carry_the_copied_options() {
        let _g = setup();
        let (ifp, _d) = crate::net::if_::test_ifnet(b"tst0");
        let opts = [IPOPT_RR, 7, 4, 0, 0, 0, 0, IPOPT_LSRR, 7, 4, 10, 0, 2, 9];
        let m = packet(1200, &opts);
        let mut ml = MbufList::new();
        ip_fragment(m, &mut ml, ifp, 600).expect("fragment");
        let first = ml_dequeue(&mut ml).expect("the first");
        assert_eq!(ip_load(&first).ip_hl(), 9, "the first keeps every option");
        let mut covered = bytes(&first).len() - 36;
        let mut later = 0;
        while let Some(f) = ml_dequeue(&mut ml) {
            let b = bytes(&f);
            let ip = ip_load(&f);
            // The source route, the two NOPs of the padding (kept: the C's), then EOLs.
            assert_eq!(ip.ip_hl(), 8, "the source route and the NOPs, padded");
            assert_eq!(&b[20..27], &opts[7..14]);
            assert_eq!(
                &b[27..32],
                &[IPOPT_NOP, IPOPT_NOP, IPOPT_EOL, IPOPT_EOL, IPOPT_EOL]
            );
            assert_eq!(in_cksum(&f, 32), 0);
            assert_eq!(usize::from(ntohs(ip.ip_off) & IP_OFFMASK) * 8, covered);
            covered += b.len() - 32;
            later += 1;
        }
        assert!(later >= 1);
        assert_eq!(covered, 1200 - 36);
    }

    #[test]
    fn inserted_options_grow_the_header() {
        let _g = setup();
        // struct ipoption: no first hop, then a 4-byte record route; m_devget leaves no room
        // before the header: an mbuf of its own is made for it.
        let o = [0u8, 0, 0, 0, IPOPT_RR, 3, 4, IPOPT_EOL];
        let mut hlen = 0;
        let m = ip_insertoptions(packet(60, &[]), &o, &mut hlen);
        assert_eq!(hlen, 24);
        assert_eq!(m.m_pkthdr().len.get(), 64);
        let b = bytes(&m);
        assert_eq!(&b[2..4], &64u16.to_be_bytes(), "ip_len grew");
        assert_eq!(&b[20..24], &o[4..8]);
        assert_eq!(
            &b[16..20],
            &[10, 0, 2, 2],
            "the destination stays without a first hop"
        );
        assert_eq!(&b[24..], &packet_bytes(60, &[], [10, 0, 2, 2])[20..]);

        // In place, with leading space; a first hop becomes the destination.
        let p = packet_bytes(60, &[], [10, 0, 2, 2]);
        let mut n = crate::net::mbuf::m_gethdr(M_DONTWAIT, MT_HEADER).expect("mbuf");
        n.data_add(64);
        n.m_len().set(60);
        n.data_mut().expect("writable").copy_from_slice(&p);
        n.m_pkthdr().len.set(60);
        let o = [10u8, 0, 0, 1, IPOPT_RR, 3, 4, IPOPT_EOL];
        let m = ip_insertoptions(n, &o, &mut hlen);
        assert_eq!(hlen, 24);
        assert!(m.m_next.get().is_none(), "no mbuf added");
        let b = bytes(&m);
        assert_eq!(b.len(), 64);
        assert_eq!(&b[16..20], &[10, 0, 0, 1], "the first hop");
        assert_eq!(&b[20..24], &o[4..8]);
        assert_eq!(&b[24..], &p[20..]);

        // Options longer than a header leave the packet alone, its header 20 bytes.
        let big = [0u8; 4 + 44];
        let m = ip_insertoptions(packet(60, &[]), &big, &mut hlen);
        assert_eq!(hlen, 20);
        assert_eq!(m.m_pkthdr().len.get(), 60);
    }

    #[test]
    fn the_output_checksums_verify_and_clear_their_flags() {
        let _g = setup();
        let mut m = packet(301, &[]);
        m.m_pkthdr()
            .csum_flags
            .set(M_UDP_CSUM_OUT | M_IPV4_CSUM_OUT);
        in_proto_cksum_out(&mut m, None);
        assert_eq!(m.m_pkthdr().csum_flags.get() & M_UDP_CSUM_OUT, 0);
        assert_eq!(
            in4_cksum(&m, IPPROTO_UDP as u8, 20, 281),
            0,
            "the UDP sum verifies"
        );
        let sw = ips(IpstatCounters::IpsOutswcsum);
        in_hdr_cksum_out(&mut m, None);
        assert_eq!(m.m_pkthdr().csum_flags.get() & M_IPV4_CSUM_OUT, 0);
        assert_eq!(in_cksum(&m, 20), 0, "the header sum verifies");
        assert_eq!(ips(IpstatCounters::IpsOutswcsum), sw + 1);

        // An interface that sums the header: the flag set, the sum left 0.
        let (ifp, _d) = crate::net::if_::test_ifnet(b"tst0");
        ifp.if_capabilities.set(IFCAP_CSUM_IPv4);
        in_hdr_cksum_out(&mut m, Some(ifp));
        assert_ne!(m.m_pkthdr().csum_flags.get() & M_IPV4_CSUM_OUT, 0);
        assert_eq!(&bytes(&m)[10..12], &[0, 0]);

        // TCP over an interface that sums it: only the pseudo header's sum is written.
        let mut b = packet_bytes(200, &[], [10, 0, 2, 2]);
        b[9] = 6;
        let mut t = mbuf_of(&b);
        t.m_pkthdr().csum_flags.set(M_TCP_CSUM_OUT);
        ifp.if_capabilities.set(IFCAP_CSUM_TCPv4);
        in_proto_cksum_out(&mut t, Some(ifp));
        assert_ne!(
            t.m_pkthdr().csum_flags.get() & M_TCP_CSUM_OUT,
            0,
            "left to it"
        );
        let want = in_cksum_phdr(
            u32::from_ne_bytes([10, 0, 2, 15]),
            u32::from_ne_bytes([10, 0, 2, 2]),
            htonl(180 + 6),
        );
        let at = 20 + TH_SUM_OFFSET;
        assert_eq!(&bytes(&t)[at..at + 2], &want.to_ne_bytes());
    }

    #[test]
    fn ip_output_completes_the_header_and_sends_it() {
        let _g = setup();
        let (_ifp, d) = iface();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");

        // No source, no version: the route's address is chosen, the header completed and
        // summed in software (the interface sums nothing).
        let mut b = packet_bytes(100, &[], [10, 0, 2, 2]);
        b[0] = 0;
        b[12..16].copy_from_slice(&[0; 4]);
        let m = m_devget(&b, 0).expect("packet");
        let out = ips(IpstatCounters::IpsLocalout);
        let mut ro = Route::new();
        ip_output(m, None, Some(&mut ro), 0, None, None, 0, &net).expect("sent");
        assert_eq!(ips(IpstatCounters::IpsLocalout), out + 1);
        let (s, _) = d.last();
        assert_eq!(s[0], 0x45, "version 4, 20 bytes");
        assert_eq!(&s[12..16], &[10, 0, 2, 15], "the interface's address");
        assert_eq!(&s[16..20], &[10, 0, 2, 2]);
        let check = m_devget(&s, 0).expect("copy");
        assert_eq!(in_cksum(&check, 20), 0, "summed in software");
        assert!(ro.rt().is_some(), "the caller's route cached");

        // The same route again: the first lookup cloned the host route, which moved the
        // routes' generation past the one the cache noted (a miss, as the C's); then a hit.
        ip_output(
            packet(100, &[]),
            None,
            Some(&mut ro),
            0,
            None,
            None,
            0,
            &net,
        )
        .expect("sent");
        let _ = d.last();
        let hits = ips(IpstatCounters::IpsRtcachehit);
        ip_output(
            packet(100, &[]),
            None,
            Some(&mut ro),
            0,
            None,
            None,
            0,
            &net,
        )
        .expect("sent");
        assert_eq!(ips(IpstatCounters::IpsRtcachehit), hits + 1);
        assert_eq!(d.count(), 1);

        // Too large: three fragments on the wire.
        let frag = ips(IpstatCounters::IpsFragmented);
        ip_output(packet(3000, &[]), None, None, 0, None, None, 0, &net).expect("sent");
        assert_eq!(ips(IpstatCounters::IpsFragmented), frag + 1);
        assert_eq!(d.count(), 1 + 3);
        drop(ro);
        drop(net);
        drop(w);
    }

    #[test]
    fn ip_output_refuses_with_the_c_errors() {
        let _g = setup();
        let (ifp, d) = iface();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let send = |m: MbufBox, flags: i32, imo: Option<&IpMoptions>| {
            ip_output(m, None, None, flags, imo, None, 0, &net)
        };

        // 0/8.
        let r = send(packet_to(60, &[], [0, 1, 2, 3]), 0, None);
        assert_eq!(r, Err(Errno::ENETUNREACH));

        // No route.
        let noroute = ips(IpstatCounters::IpsNoroute);
        let r = send(packet_to(60, &[], [10, 9, 9, 9]), 0, None);
        assert_eq!(r, Err(Errno::EHOSTUNREACH));
        assert_eq!(ips(IpstatCounters::IpsNoroute), noroute + 1);

        // Don't fragment over the MTU.
        let mut b = packet_bytes(3000, &[], [10, 0, 2, 2]);
        b[6..8].copy_from_slice(&IP_DF.to_be_bytes());
        let cant = ips(IpstatCounters::IpsCantfrag);
        assert_eq!(send(mbuf_of(&b), 0, None), Err(Errno::EMSGSIZE));
        assert_eq!(ips(IpstatCounters::IpsCantfrag), cant + 1);

        // The limited broadcast through the multicast options' interface: refused without
        // IP_ALLOWBROADCAST, sent as a broadcast with it, refused when too large.
        let imo = IpMoptions::output(ifp.if_index.get() as u16, 1, 0);
        let bcast = |len| packet_to(len, &[], [255, 255, 255, 255]);
        assert_eq!(send(bcast(60), 0, Some(&imo)), Err(Errno::EACCES));
        assert_eq!(d.count(), 0);
        send(bcast(60), IP_ALLOWBROADCAST, Some(&imo)).expect("sent");
        let (s, f) = d.last();
        assert_ne!(f & M_BCAST, 0);
        assert_eq!(s[8], 1, "the multicast options' TTL");
        assert_eq!(
            send(bcast(1600), IP_ALLOWBROADCAST, Some(&imo)),
            Err(Errno::EMSGSIZE)
        );

        // A broadcast on an interface without IFF_BROADCAST.
        ifp.if_flags.set(IFF_UP | IFF_MULTICAST);
        assert_eq!(
            send(bcast(60), IP_ALLOWBROADCAST, Some(&imo)),
            Err(Errno::ENETUNREACH),
            "the multicast check first: M_BCAST on an interface that cannot"
        );
        drop(net);
        drop(w);
    }

    // The IP socket options (`ip_ctloutput` .. `ip_freemoptions`) over a socket of the
    // kernel's Internet fixture (`kernel::netinet::in_pcb::testinet`): every option name set
    // and read back, each errno the C answers, the multicast options made, grown and freed.

    use hal::sys::netinet_in::IPPROTO_TCP;

    use crate::cred::{CredRef, Ucred};
    use crate::net::proto::{PRCO_GETOPT, PRCO_SETOPT};
    use crate::netinet::in_::in_hasmulti;
    use crate::netinet::in_pcb::{INP_RECVIF, testinet};

    type Tg = testinet::G;

    /// An option mbuf holding `v`.
    fn optm(v: &[u8]) -> MbufBox {
        let mut m = m_get(M_DONTWAIT, i32::from(MT_SOOPTS)).expect("an mbuf");
        m.m_len().set(v.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(v);
        m
    }

    /// `ip_ctloutput(op, inp, level, name, m)` under the net lock: `v` the value of a
    /// `PRCO_SETOPT` (`None` for none); the answer's bytes.
    fn ctl(
        inp: &Inpcb<Tg>,
        options: &mut Option<MbufBox>,
        op: i32,
        level: i32,
        name: i32,
        v: Option<&[u8]>,
    ) -> Result<Vec<u8>, Errno> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut m = optm(v.unwrap_or(&[]));
        let arg = (op == PRCO_GETOPT || v.is_some()).then_some(&mut *m);
        ip_ctloutput(op, inp, options, level, name, arg, &net)?;
        Ok(m.data().to_vec())
    }

    /// `setsockopt(IPPROTO_IP, name, v)`.
    fn set(inp: &Inpcb<Tg>, o: &mut Option<MbufBox>, name: i32, v: &[u8]) -> Result<(), Errno> {
        ctl(inp, o, PRCO_SETOPT, IPPROTO_IP, name, Some(v)).map(drop)
    }

    /// `getsockopt(IPPROTO_IP, name)`.
    fn get(inp: &Inpcb<Tg>, o: &mut Option<MbufBox>, name: i32) -> Result<Vec<u8>, Errno> {
        ctl(inp, o, PRCO_GETOPT, IPPROTO_IP, name, None)
    }

    /// An `int` option's bytes.
    fn int(v: i32) -> Vec<u8> {
        v.to_ne_bytes().to_vec()
    }

    /// `f` as a user that is not the superuser (uid 1000), then `p` root again.
    fn as_user<R>(p: &crate::proc::Thread<Tg>, f: impl FnOnce() -> R) -> R {
        let mut ids = Ucred::new();
        ids.cr_uid.set(1000);
        ids.cr_ruid.set(1000);
        let root = p.set_cred(CredRef::new(ids));
        let r = f();
        if let Some(root) = root {
            drop(p.set_cred(root));
        }
        r
    }

    #[test]
    fn ip_ctloutput_sets_and_gets_every_option_as_the_c() {
        let (_g, p) = testinet::setup();
        let (so, inp) = testinet::pcb(testinet::table(16));
        let mut opts = None;
        let o = &mut opts;
        use Errno::{EACCES, EINVAL, ENOPROTOOPT, EOPNOTSUPP};

        // Another level, either way, before anything else.
        let tos = Some(&[1u8, 0, 0, 0][..]);
        assert_eq!(
            ctl(&inp, o, PRCO_SETOPT, IPPROTO_TCP, IP_TOS, tos),
            Err(EINVAL)
        );
        assert_eq!(
            ctl(&inp, o, PRCO_GETOPT, IPPROTO_TCP, IP_TOS, None),
            Err(EINVAL)
        );

        // The `int` options: no value or another length is EINVAL, a value is read back.
        let ints = [
            (IP_TOS, 0x10),
            (IP_TTL, 33),
            (IP_MINTTL, 255),
            (IP_RECVOPTS, 1),
            (IP_RECVRETOPTS, 1),
            (IP_RECVDSTADDR, 1),
            (IP_RECVIF, 1),
            (IP_RECVTTL, 1),
            (IP_RECVDSTPORT, 1),
            (IP_RECVRTABLE, 1),
            (IP_IPSECFLOWINFO, 1),
            (IP_PORTRANGE, IP_PORTRANGE_HIGH),
            (IP_PIPEX, 7),
            (IP_AUTH_LEVEL, IPSEC_LEVEL_UNIQUE),
            (IP_ESP_TRANS_LEVEL, IPSEC_LEVEL_UNIQUE),
            (IP_ESP_NETWORK_LEVEL, IPSEC_LEVEL_UNIQUE),
            (IP_IPCOMP_LEVEL, IPSEC_LEVEL_UNIQUE),
        ];
        for (name, v) in ints {
            assert_eq!(
                ctl(&inp, o, PRCO_SETOPT, IPPROTO_IP, name, None),
                Err(EINVAL),
                "{name} without a value"
            );
            assert_eq!(
                set(&inp, o, name, &[1, 0]),
                Err(EINVAL),
                "{name} of 2 bytes"
            );
            assert_eq!(
                set(&inp, o, name, &[0; 8]),
                Err(EINVAL),
                "{name} of 8 bytes"
            );
            set(&inp, o, name, &int(v)).expect("set");
            assert_eq!(get(&inp, o, name), Ok(int(v)), "{name}");
        }
        assert_eq!(inp.inp_ip.get().ip_tos, 0x10);
        assert_eq!(inp.inp_ip.get().ip_ttl, 33);
        assert_eq!(inp.inp_ip_minttl.get(), 255);
        assert_eq!(inp.inp_pipex.get(), 7);

        // OPTSET: the flag follows the value, OPTBIT reads it.
        let flags = [
            (IP_RECVOPTS, INP_RECVOPTS),
            (IP_RECVRETOPTS, INP_RECVRETOPTS),
            (IP_RECVDSTADDR, INP_RECVDSTADDR),
            (IP_RECVIF, INP_RECVIF),
            (IP_RECVTTL, INP_RECVTTL),
            (IP_RECVDSTPORT, INP_RECVDSTPORT),
            (IP_RECVRTABLE, INP_RECVRTABLE),
            (IP_IPSECFLOWINFO, INP_IPSECFLOWINFO),
        ];
        for (name, bit) in flags {
            assert!(inp.has_flags(bit), "{name}");
            set(&inp, o, name, &int(0)).expect("cleared");
            assert!(!inp.has_flags(bit), "{name}");
            assert_eq!(get(&inp, o, name), Ok(int(0)), "{name}");
            set(&inp, o, name, &int(-5)).expect("any non-zero value sets");
            assert!(inp.has_flags(bit), "{name}");
        }

        // The bounds of IP_TTL (1..=MAXTTL, -1 the default) and IP_MINTTL (0..=MAXTTL).
        for bad in [0, 256, -2] {
            assert_eq!(set(&inp, o, IP_TTL, &int(bad)), Err(EINVAL), "ttl {bad}");
        }
        assert_eq!(
            get(&inp, o, IP_TTL),
            Ok(int(33)),
            "a refused TTL changes nothing"
        );
        set(&inp, o, IP_TTL, &int(-1)).expect("the default");
        assert_eq!(
            get(&inp, o, IP_TTL),
            Ok(int(IP_DEFTTL.load(Ordering::Relaxed)))
        );
        for bad in [-1, 256] {
            assert_eq!(
                set(&inp, o, IP_MINTTL, &int(bad)),
                Err(EINVAL),
                "minttl {bad}"
            );
        }
        set(&inp, o, IP_MINTTL, &int(0)).expect("0");

        // IP_PORTRANGE: the two flags exclude each other; another range is EINVAL.
        set(&inp, o, IP_PORTRANGE, &int(IP_PORTRANGE_LOW)).expect("low");
        assert!(inp.has_flags(INP_LOWPORT) && !inp.has_flags(INP_HIGHPORT));
        assert_eq!(get(&inp, o, IP_PORTRANGE), Ok(int(IP_PORTRANGE_LOW)));
        set(&inp, o, IP_PORTRANGE, &int(IP_PORTRANGE_DEFAULT)).expect("default");
        assert!(!inp.has_flags(INP_LOWPORT | INP_HIGHPORT));
        assert_eq!(get(&inp, o, IP_PORTRANGE), Ok(int(0)));
        assert_eq!(set(&inp, o, IP_PORTRANGE, &int(3)), Err(EINVAL));

        // The IPsec levels: outside BYPASS..=UNIQUE is EINVAL; below the default, EACCES but
        // for the superuser.
        for name in [
            IP_AUTH_LEVEL,
            IP_ESP_TRANS_LEVEL,
            IP_ESP_NETWORK_LEVEL,
            IP_IPCOMP_LEVEL,
        ] {
            for bad in [IPSEC_LEVEL_BYPASS - 1, IPSEC_LEVEL_UNIQUE + 1] {
                assert_eq!(set(&inp, o, name, &int(bad)), Err(EINVAL), "{name} {bad}");
            }
            assert_eq!(
                as_user(p, || set(&inp, o, name, &int(IPSEC_LEVEL_BYPASS))),
                Err(EACCES),
                "{name}"
            );
            assert_eq!(
                get(&inp, o, name),
                Ok(int(IPSEC_LEVEL_UNIQUE)),
                "{name} kept"
            );
            set(&inp, o, name, &int(IPSEC_LEVEL_BYPASS)).expect("the superuser");
            assert_eq!(get(&inp, o, name), Ok(int(IPSEC_LEVEL_BYPASS)), "{name}");
        }
        assert_eq!(inp.inp_seclevel.get().sl_ipcomp, IPSEC_LEVEL_BYPASS as u8);

        // Read-only, unsupported and unknown names.
        assert_eq!(
            get(&inp, o, IP_IPDEFTTL),
            Ok(int(IP_DEFTTL.load(Ordering::Relaxed)))
        );
        assert_eq!(set(&inp, o, IP_IPDEFTTL, &int(9)), Err(ENOPROTOOPT));
        assert_eq!(set(&inp, o, IP_RETOPTS, &[0; 4]), Err(ENOPROTOOPT));
        for name in [IP_IPSEC_LOCAL_ID, IP_IPSEC_REMOTE_ID] {
            assert_eq!(set(&inp, o, name, &int(1)), Err(EOPNOTSUPP), "{name}");
            assert_eq!(get(&inp, o, name), Err(EOPNOTSUPP), "{name}");
        }
        assert_eq!(set(&inp, o, 99, &int(1)), Err(ENOPROTOOPT));
        assert_eq!(get(&inp, o, 99), Err(ENOPROTOOPT));

        // SO_RTABLE: at least a `u_int`; the same table is no change; a table that does not
        // exist is EINVAL (in_pcbset_rtableid).
        assert_eq!(get(&inp, o, SO_RTABLE), Ok(int(0)));
        assert_eq!(set(&inp, o, SO_RTABLE, &[0; 2]), Err(EINVAL));
        set(&inp, o, SO_RTABLE, &[0; 8]).expect("table 0 again, 8 bytes");
        assert_eq!(set(&inp, o, SO_RTABLE, &int(5)), Err(EINVAL));
        assert_eq!(inp.inp_rtableid.get(), 0);

        // IP_OPTIONS: the list stored behind the first hop (zero, no source route), read back
        // by IP_OPTIONS and IP_RETOPTS; no value turns them off.
        let list = [IPOPT_NOP, IPOPT_NOP, IPOPT_NOP, IPOPT_EOL];
        set(&inp, o, IP_OPTIONS, &list).expect("options");
        let want = [0, 0, 0, 0, IPOPT_NOP, IPOPT_NOP, IPOPT_NOP, IPOPT_EOL];
        assert_eq!(get(&inp, o, IP_OPTIONS), Ok(want.to_vec()));
        assert_eq!(get(&inp, o, IP_RETOPTS), Ok(want.to_vec()));
        ctl(&inp, o, PRCO_SETOPT, IPPROTO_IP, IP_OPTIONS, None).expect("off");
        assert!(o.is_none());
        assert_eq!(get(&inp, o, IP_OPTIONS), Ok(vec![]));

        // The multicast options' get side without options: the C's defaults.
        assert_eq!(get(&inp, o, IP_MULTICAST_IF), Ok(vec![0; 4]));
        assert_eq!(
            get(&inp, o, IP_MULTICAST_TTL),
            Ok(vec![IP_DEFAULT_MULTICAST_TTL])
        );
        assert_eq!(
            get(&inp, o, IP_MULTICAST_LOOP),
            Ok(vec![IP_DEFAULT_MULTICAST_LOOP])
        );
        for name in [IP_ADD_MEMBERSHIP, IP_DROP_MEMBERSHIP] {
            assert_eq!(get(&inp, o, name), Err(EOPNOTSUPP), "{name}");
        }

        testinet::release(so, inp);
    }

    /// An `ip_mreq` of `group` on the interface of local address `ifaddr`.
    fn mreq(group: [u8; 4], ifaddr: [u8; 4]) -> Vec<u8> {
        [group, ifaddr].concat()
    }

    #[test]
    fn the_multicast_options_are_made_grown_and_freed_as_the_c() {
        let (_g, _p) = testinet::setup();
        let (ifp, _d) = testinet::iface(b"tst0", [10, 0, 2, 15]);
        let idx = ifp.if_index.get();
        let (so, inp) = testinet::pcb(testinet::table(16));
        let mut opts = None;
        let o = &mut opts;
        use Errno::{EADDRINUSE, EADDRNOTAVAIL, EINVAL};

        // TTL and loopback: one byte; the loopback switch 0 or 1.
        assert_eq!(set(&inp, o, IP_MULTICAST_TTL, &[5, 0]), Err(EINVAL));
        set(&inp, o, IP_MULTICAST_TTL, &[5]).expect("ttl");
        assert_eq!(get(&inp, o, IP_MULTICAST_TTL), Ok(vec![5]));
        assert_eq!(set(&inp, o, IP_MULTICAST_LOOP, &[2]), Err(EINVAL));
        set(&inp, o, IP_MULTICAST_LOOP, &[0]).expect("loop");
        assert_eq!(get(&inp, o, IP_MULTICAST_LOOP), Ok(vec![0]));
        assert_eq!(
            inp.moptions_out()
                .map(|m| (m.imo_ifidx, m.imo_ttl, m.imo_loop)),
            Some((0, 5, 0))
        );
        // Back to the defaults: the options are freed (`*imop = NULL`).
        set(&inp, o, IP_MULTICAST_TTL, &[1]).expect("ttl");
        set(&inp, o, IP_MULTICAST_LOOP, &[1]).expect("loop");
        assert!(inp.moptions_out().is_none());

        // The interface: by a local address, by an index (`ip_mreqn`), cleared by ANY.
        set(&inp, o, IP_MULTICAST_IF, &[10, 0, 2, 15]).expect("by address");
        assert_eq!(get(&inp, o, IP_MULTICAST_IF), Ok(vec![10, 0, 2, 15]));
        assert_eq!(
            inp.moptions_out().map(|m| u32::from(m.imo_ifidx)),
            Some(idx)
        );
        assert_eq!(
            set(&inp, o, IP_MULTICAST_IF, &[10, 0, 2, 99]),
            Err(EADDRNOTAVAIL),
            "not a local address"
        );
        set(&inp, o, IP_MULTICAST_IF, &[0; 4]).expect("INADDR_ANY");
        assert!(inp.moptions_out().is_none(), "the defaults again");
        let mut mreqn = [0u8; 12];
        mreqn[8..].copy_from_slice(&(idx as i32).to_ne_bytes());
        set(&inp, o, IP_MULTICAST_IF, &mreqn).expect("by index");
        assert_eq!(
            inp.moptions_out().map(|m| u32::from(m.imo_ifidx)),
            Some(idx)
        );
        mreqn[8..].copy_from_slice(&99i32.to_ne_bytes());
        assert_eq!(set(&inp, o, IP_MULTICAST_IF, &mreqn), Err(EADDRNOTAVAIL));
        assert_eq!(set(&inp, o, IP_MULTICAST_IF, &[0; 5]), Err(EINVAL));
        assert_eq!(
            ctl(&inp, o, PRCO_SETOPT, IPPROTO_IP, IP_MULTICAST_IF, None),
            Err(EINVAL)
        );
        set(&inp, o, IP_MULTICAST_IF, &[0; 4]).expect("INADDR_ANY");

        // A membership: a multicast group only, of an ip_mreq or ip_mreqn's size.
        let group = [224, 1, 2, 3];
        let ga = InAddr {
            s_addr: u32::from_ne_bytes(group),
        };
        let here = [10, 0, 2, 15];
        assert_eq!(
            set(&inp, o, IP_ADD_MEMBERSHIP, &mreq([10, 0, 0, 1], here)),
            Err(EINVAL)
        );
        assert_eq!(set(&inp, o, IP_ADD_MEMBERSHIP, &group), Err(EINVAL));
        assert_eq!(
            set(&inp, o, IP_ADD_MEMBERSHIP, &mreq(group, [10, 0, 2, 99])),
            Err(EADDRNOTAVAIL),
            "no local route to the interface address"
        );
        set(&inp, o, IP_ADD_MEMBERSHIP, &mreq(group, here)).expect("joined");
        assert!(in_hasmulti(&ga, ifp));
        assert_eq!(
            set(&inp, o, IP_ADD_MEMBERSHIP, &mreq(group, here)),
            Err(EADDRINUSE)
        );
        set(&inp, o, IP_DROP_MEMBERSHIP, &mreq(group, here)).expect("left");
        assert!(!in_hasmulti(&ga, ifp));
        assert_eq!(
            set(&inp, o, IP_DROP_MEMBERSHIP, &mreq(group, here)),
            Err(EADDRNOTAVAIL)
        );
        assert!(
            inp.moptions_out().is_none(),
            "no membership, the defaults: freed"
        );

        // Sixteen groups grow the vector from IP_MIN_MEMBERSHIPS to 2 * 16 - 1.
        let groups: Vec<[u8; 4]> = (0..16u8).map(|i| [224, 1, 3, i]).collect();
        for g in &groups {
            set(&inp, o, IP_ADD_MEMBERSHIP, &mreq(*g, here)).expect("joined");
        }
        {
            let _w = NETLOCK.write();
            let imo = inp.moptions();
            let imo = imo.as_deref().expect("options");
            assert_eq!(imo.imo_num_memberships, 16);
            assert_eq!(imo.imo_max_memberships(), 2 * (IP_MIN_MEMBERSHIPS + 1) - 1);
            assert!(imo.membership(15).is_some() && imo.membership(16).is_none());
        }
        // A group in the middle dropped: the gap closed, the order kept.
        set(&inp, o, IP_DROP_MEMBERSHIP, &mreq(groups[3], here)).expect("left");
        {
            let _w = NETLOCK.write();
            let imo = inp.moptions();
            let imo = imo.as_deref().expect("options");
            let left: Vec<u8> = (0..15)
                .filter_map(|i| imo.membership(i))
                .map(|inm| inm.inm_addr().s_addr.to_ne_bytes()[3])
                .collect();
            let want: Vec<u8> = (0..16u8).filter(|&i| i != 3).collect();
            assert_eq!(left, want);
        }

        // The detach leaves every group (`ip_freemoptions`).
        testinet::release(so, inp);
        for g in &groups {
            let a = InAddr {
                s_addr: u32::from_ne_bytes(*g),
            };
            assert!(!in_hasmulti(&a, ifp), "{g:?}");
        }
    }

    #[test]
    fn ip_pcbopts_pops_the_first_hop_of_a_source_route() {
        let _g = setup();
        let mut pcbopt = None;

        // NOP, then LSRR through 10.0.0.1 (first hop) and 10.0.0.2 as the user writes it (the
        // addresses where the pointer byte goes), then EOL: the first hop moves before the
        // options and the route shrinks by one address.
        let m = optm(&[
            IPOPT_NOP, IPOPT_LSRR, 10, 10, 0, 0, 1, 10, 0, 0, 2, IPOPT_EOL,
        ]);
        ip_pcbopts(&mut pcbopt, Some(&m)).expect("options");
        assert_eq!(
            pcbopt.as_deref().map(Mbuf::data),
            Some(
                &[
                    10, 0, 0, 1, IPOPT_NOP, IPOPT_LSRR, 6, 10, 0, 0, 2, IPOPT_EOL
                ][..]
            )
        );

        // A length that is not a multiple of 4, or an option longer than the list, is refused
        // (and the old options are gone either way).
        assert_eq!(
            ip_pcbopts(&mut pcbopt, Some(&optm(&[IPOPT_NOP; 3]))),
            Err(Errno::EINVAL)
        );
        assert!(pcbopt.is_none());
        assert_eq!(
            ip_pcbopts(&mut pcbopt, Some(&optm(&[IPOPT_RR, 9, 4, 0]))),
            Err(Errno::EINVAL)
        );
        // Longer than a `struct ipoption` holds once the first hop is popped, or than the
        // input may be: EINVAL.
        assert_eq!(
            ip_pcbopts(&mut pcbopt, Some(&optm(&[IPOPT_NOP; 48]))),
            Err(Errno::EINVAL)
        );
        let mut rr = [IPOPT_NOP; 44];
        rr[0] = IPOPT_RR;
        rr[1] = 44;
        rr[2] = 4;
        assert_eq!(
            ip_pcbopts(&mut pcbopt, Some(&optm(&rr))),
            Err(Errno::EINVAL),
            "44 bytes of options after an empty first hop"
        );
        // A source route too short for its first hop.
        assert_eq!(
            ip_pcbopts(&mut pcbopt, Some(&optm(&[IPOPT_LSRR, 6, 4, 0, 0, 0, 0, 0]))),
            Err(Errno::EINVAL)
        );
        // No mbuf, or an empty one, only turns the options off.
        ip_pcbopts(&mut pcbopt, Some(&m)).expect("options");
        ip_pcbopts(&mut pcbopt, None).expect("off");
        assert!(pcbopt.is_none());
        ip_pcbopts(&mut pcbopt, Some(&m)).expect("options");
        ip_pcbopts(&mut pcbopt, Some(&optm(&[]))).expect("off");
        assert!(pcbopt.is_none());
    }

    #[test]
    fn ip_getmoptions_answers_the_defaults_without_options() {
        let _g = setup();
        let mut m = optm(&[0; 4]);
        ip_getmoptions(IP_MULTICAST_TTL, None, &mut m).expect("ttl");
        assert_eq!(m.data(), &[IP_DEFAULT_MULTICAST_TTL]);
        ip_getmoptions(IP_MULTICAST_LOOP, None, &mut m).expect("loop");
        assert_eq!(m.data(), &[IP_DEFAULT_MULTICAST_LOOP]);
        ip_getmoptions(IP_MULTICAST_IF, None, &mut m).expect("if");
        assert_eq!(m.data(), &[0, 0, 0, 0]);
        let set = IpMoptions::output(0, 9, 0);
        ip_getmoptions(IP_MULTICAST_TTL, Some(&set), &mut m).expect("ttl");
        assert_eq!(m.data(), &[9]);
        assert_eq!(ip_getmoptions(IP_TTL, None, &mut m), Err(Errno::EOPNOTSUPP));
    }
}
/* </TESTS> */
