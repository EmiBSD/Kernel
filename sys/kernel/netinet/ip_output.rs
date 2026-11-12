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
//! (`ip_ctloutput` .. `ip_freemoptions`, ip_output.c:866-1750) is still `bsd`'s
//! `sys/netinet/ip_output.rs` until the pcb moves (N6.4c1).
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
//! - `in_ifp2ia(ifp)->ia_addr.sin_addr` and `in_hasmulti(dst, ifp)` read the interface's
//!   generic records (`if_addrlist`'s first `AF_INET` address through its `ifa_addr`, the
//!   `if_maddrlist` walk through `ifma_addr` under `if_maddrlock`) until the IPv4 records
//!   move (`in_var.h`'s `struct in_ifaddr` and `struct in_multi`, N6.4a's next commit).
//! - `icmp_mtudisc_clone` (ICMP's, N6.4c2) and the TCP and UDP counters of software
//!   checksums (`tcps_outswcsum`, `udps_outswcsum`, N6.6) are `Inet` legs.
//! - `ip_optcopy`'s option without its length byte (one byte left, not `IPOPT_EOL` or
//!   `IPOPT_NOP`) ends the copy, where the C reads the byte past the options (a `DIAGNOSTIC`
//!   kernel panics); a length byte of 0 ends it too (LZ's), where the C would loop.
//!
//! ## Redesign
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
//!
//! **Lock and spl transitions** against the C (`ip_output`, the unicast path): the caller's
//! net lock (any mode), no lock of its own; the route cache hit reads the route's validity
//! (`rtisvalid`, the gateway route's `ReadMostly` read, two atomics, N6.3b2) where the C reads
//! two words; `if_get`/`if_put` one reference count each, as the C; the IPsec send keeps the
//! C's `KERNEL_LOCK()` around `ipsp_process_packet` inside `bsd`. The multicast loopback
//! takes `if_maddrlock` for reading once, as `in_hasmulti`. No spl change: the C has none.

use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::malloc::M_NOWAIT;
use hal::netlock::NetHeld;
use hal::pod;
use hal::rwlock::RW_READ;
use hal::sys::endian::{htonl, htons, ntohl, ntohs};
use hal::sys::errno::Errno;
use hal::sys::net_if::{
    IFCAP_CSUM_IPv4, IFCAP_CSUM_TCPv4, IFCAP_CSUM_UDPv4, IFCAP_TSOv4, IFF_BROADCAST, IFF_LOOPBACK,
    IFF_MULTICAST, IFF_SIMPLEX,
};
use hal::sys::net_route::{RTF_BROADCAST, RTF_GATEWAY, RTF_HOST, RTF_LOCAL, RTV_MTU};
use hal::sys::netinet_in::{
    IN_CLASSA_NSHIFT, INADDR_ANY, INADDR_BROADCAST, IP_DEFAULT_MULTICAST_TTL, IPPROTO_ICMP,
    IPPROTO_TCP, IPPROTO_UDP, InAddr, in_multicast,
};
use hal::sys::netinet_ip::{
    IP_DF, IP_MAXPACKET, IP_MF, IPOPT_EOL, IPOPT_NOP, IPOPT_OLEN, IPVERSION, Ip, ipopt_copied,
};
use hal::sys::netinet_tcp::Tcphdr;
use hal::sys::netinet_udp::Udphdr;
use hal::sys::socket::AF_INET;

use crate::assert::kpanic;
use crate::net::if_::{IfRef, if_input_local, if_output_ml, if_output_tso, inet};
use crate::net::if_var::Ifnet;
use crate::net::ipsec_hooks::{IpsecLevel, SpdError, TdbHold, ipsec};
use crate::net::mbuf::{
    M_BCAST, M_DONTWAIT, M_ICMP_CSUM_OUT, M_IPV4_CSUM_OUT, M_MCAST, M_TCP_CSUM_OUT, M_TCP_TSO,
    M_UDP_CSUM_OUT, MAX_LINKHDR, MT_HEADER, Mbuf, MbufBox, MbufList, PACKET_TAG_IPSEC_IN_DONE,
    PF_TAG_GENERATED, PF_TAG_REROUTE, m_adj, m_copyback, m_copym, m_dup_pkt, m_dup_pkthdr,
    m_gethdr, m_move_hdr, ml_enlist, ml_enqueue, ml_init, ml_len,
};
use crate::net::pf_hooks::{PF_FWD, PF_OUT, PF_PASS, pf};
use crate::net::proto::SockaddrView;
use crate::net::route::{Route, rtalloc_mpath, rtisvalid};
use crate::net::rtable::rtable_loindex;
use crate::netinet::in_cksum::in_cksum;
use crate::netinet::in4_cksum::in4_cksum;
use crate::netinet::ip_id::ip_randomid;
use crate::netinet::ip_var::{
    IP_ALLOWBROADCAST, IP_FORWARDING, IP_FORWARDING_IPSEC, IP_MTUDISC, IP_RAWOUTPUT, IpMoptions,
    IpstatCounters, ip_mtudisc, ipstat_add, ipstat_inc,
};

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

/// `in_ifp2ia(ifp)->ia_addr.sin_addr`: the address of the first IPv4 address of `ifp`, in
/// `if_addrlist`'s order (the generic records, the module's deviations).
fn ifp_inaddr(ifp: &Ifnet, net: &NetHeld<'_>) -> Option<InAddr> {
    for ifa in &ifp.if_addrlist {
        if let Some(a) = ifa.ifa_addr.read(net)
            && a.family() == AF_INET
        {
            return Some(sin_addr_of(&a));
        }
    }
    None
}

/// `in_hasmulti(addr, ifp)`: whether `ifp` is a member of the IPv4 group `addr`, its
/// multicast records walked under `if_maddrlock` read (the generic records, the module's
/// deviations).
fn ifp_hasmulti(addr: InAddr, ifp: &Ifnet, net: &NetHeld<'_>) -> bool {
    let lock = &ifp.rt().if_maddrlock;
    // A read without `RW_NOSLEEP` or `RW_INTR` waits until it has the lock.
    let _ = lock.enter(RW_READ);
    let joined = ifp.if_maddrlist.iter().any(|ifma| {
        ifma.ifma_addr
            .read(net)
            .is_some_and(|a| a.family() == AF_INET && sin_addr_of(&a).s_addr == addr.s_addr)
    });
    lock.exit();
    joined
}

/// `ip_output(m, opt, ro, flags, imo, seclevel, ipsecflowinfo)`: IP output. The packet `m`
/// holds a skeletal IP header (with len, off, ttl, proto, tos, src, dst); it is consumed. The
/// options `opt` (a `struct ipoption`), if present, are not. The caller holds the net lock
/// (`net`).
#[allow(
    clippy::too_many_arguments,
    reason = "the C's signature and the net lock's proof"
)]
pub fn ip_output(
    m: MbufBox,
    opt: Option<&Mbuf>,
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
                && let Some(a) = ifp_inaddr(&i, net)
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

            // `ia = ifatoia(ro->ro_rt->rt_ifa)`: read below only for a source address.
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

            // Set the source IP address (`ia->ia_addr.sin_addr`, the C's cast: its bytes 4 to
            // 8 whatever the family).
            if ip.ip_src.s_addr == INADDR_ANY
                && let Some(a) = ia.and_then(|ia| ia.ifa_addr.read(net))
            {
                ip.ip_src = sin_addr_of(&a);
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
                && let Some(a) = ifp_inaddr(ifn, net)
            {
                ip.ip_src = a;
            }

            if imo.is_none_or(|imo| imo.imo_loop != 0) && ifp_hasmulti(ip.ip_dst, ifn, net) {
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

/// `ip_insertoptions(m, opt, phlen)`: the IP options `opt` (a `struct ipoption`: the first
/// hop, then the list) inserted into the preformed packet `m`. Adjusts the IP destination as
/// required for IP source routing, as indicated by a non-zero `in_addr` at the start of the
/// options; `phlen` gets the new header length. The packet is returned unchanged when the
/// options do not fit (`phlen` untouched: the C's "XXX should fail"), or with the header
/// alone (`phlen` 20) when they exceed a header.
pub fn ip_insertoptions(m: MbufBox, opt: &Mbuf, phlen: &mut i32) -> MbufBox {
    let mut m = m;
    let p = opt.data();
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

    use hal::link::SaWords;
    use hal::netlock::NETLOCK;
    use hal::sys::net_if::IFF_UP;
    use hal::sys::net_route::RTF_CONNECTED;
    use hal::sys::netinet_in::SockaddrIn;
    use hal::sys::netinet_ip::{IP_OFFMASK, IPOPT_LSRR, IPOPT_RR};

    use super::*;
    use crate::net::if_var::{Ifaddr, NetDriver};
    use crate::net::mbuf::{M_PKTHDR, m_copydata, m_devget, m_pullup, ml_dequeue};
    use crate::net::route::rt_ifa_add;
    use crate::net::route::tests::{sin, testroutes_setup};
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

    /// The `sockaddr_in` image of `a`, in words the address can be linked to.
    fn words(a: [u8; 4]) -> &'static SaWords {
        let w: &'static SaWords = Box::leak(Box::new(SaWords::new()));
        let mut sin = SockaddrIn::default();
        sin.sin_len = 16;
        sin.sin_family = AF_INET;
        sin.sin_addr.s_addr = u32::from_ne_bytes(a);
        w.set(&sin);
        w
    }

    /// An interface up with broadcast, MTU 1500, address 10.0.2.15/24 and its connected
    /// route, under the net lock the caller holds.
    fn iface(net: &NetHeld<'_>) -> (&'static Ifnet, &'static CapDriver) {
        let d: &'static CapDriver = Box::leak(Box::new(CapDriver {
            sent: Mutex::new(Vec::new()),
        }));
        let ifp = crate::net::if_::test_ifnet_with(b"ipo0", d);
        ifp.if_flags.set(IFF_UP | IFF_BROADCAST | IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        crate::net::if_::if_alloc_sadl(ifp);
        let ifa: &'static Ifaddr = Box::leak(Box::new(Ifaddr::new()));
        ifa.ifa_refcnt.init();
        ifa.ifa_addr
            .set_words::<SockaddrIn>(Some(words([10, 0, 2, 15])));
        ifa.ifa_netmask
            .set_words::<SockaddrIn>(Some(words([255, 255, 255, 0])));
        ifa.ifa_ifp.set(Some(ifp));
        crate::net::if_::ifa_add(ifp, ifa);
        let dst = sin([10, 0, 2, 0]);
        let view = SockaddrView::new(&dst).expect("a view");
        rt_ifa_add(ifa, RTF_CONNECTED, view, 0, net).expect("the connected route");
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

    /// An option mbuf holding `o`.
    fn optm(o: &[u8]) -> MbufBox {
        let mut m = crate::net::mbuf::m_get(M_DONTWAIT, hal::sys::mbuf::MT_SOOPTS).expect("mbuf");
        m.m_len().set(o.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(o);
        m
    }

    #[test]
    fn inserted_options_grow_the_header() {
        let _g = setup();
        // struct ipoption: no first hop, then a 4-byte record route; m_devget leaves no room
        // before the header: an mbuf of its own is made for it.
        let o = [0u8, 0, 0, 0, IPOPT_RR, 3, 4, IPOPT_EOL];
        let opt = optm(&o);
        let mut hlen = 0;
        let m = ip_insertoptions(packet(60, &[]), &opt, &mut hlen);
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
        let m = ip_insertoptions(n, &optm(&o), &mut hlen);
        assert_eq!(hlen, 24);
        assert!(m.m_next.get().is_none(), "no mbuf added");
        let b = bytes(&m);
        assert_eq!(b.len(), 64);
        assert_eq!(&b[16..20], &[10, 0, 0, 1], "the first hop");
        assert_eq!(&b[20..24], &o[4..8]);
        assert_eq!(&b[24..], &p[20..]);

        // Options longer than a header leave the packet alone, its header 20 bytes.
        let big = [0u8; 4 + 44];
        let m = ip_insertoptions(packet(60, &[]), &optm(&big), &mut hlen);
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
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let (_ifp, d) = iface(&net);

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

        // The same route again: a cache hit.
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
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let (ifp, d) = iface(&net);
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
        let imo = IpMoptions {
            imo_ifidx: ifp.if_index.get() as u16,
            imo_ttl: 1,
            imo_loop: 0,
        };
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
}
/* </TESTS> */
