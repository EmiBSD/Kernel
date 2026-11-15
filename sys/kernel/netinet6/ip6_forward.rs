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
/* </LICENSES> */

/* <CODE> */
//! IPv6 forwarding: `netinet6/ip6_forward.c`. [`ip6_forward`] decrements the hop limit, finds
//! the route (`Route::mpath6`), applies the outbound IPsec policy, decides on a redirect, runs
//! pf(4) in the forward direction and sends the packet with `if_output_tso`. A copy of the
//! start of the packet (on the stack when it is small) feeds the ICMPv6 error or redirect sent
//! back to the source.
//!
//! Upstream: sys/netinet6/ip6_forward.c @ 3ce1f3f79392
//! LZ: sys/netinet6/ip6_forward.rs@3c62ede68802
//!
//! ## Deviations
//! - `NPF` (pf(4)) and `IPSEC` are configured: `pf_test(AF_INET6, PF_FWD, ..)` is the `Pf`
//!   bridge's leg, `ip6_output_ipsec_lookup`/`_send` the `Ipsec` bridge's
//!   `ip6_output_lookup`/`ip6_output_send` (D-18, `bsd`'s until N8).
//! - The `IPV6_FORWARDING_IPSEC` check frees the packet before `goto senderr`; the C leaves it
//!   allocated there (a leak). An SA looked up again after a pf reroute gives the first one's
//!   reference back (its hold's drop); the C overwrites `tdb` (a reference leaked).
//! - `ip6_output_ipsec_lookup`'s errors (`SpdError::Drop` is the C's `-EINVAL`, silently
//!   drop) all lead to `freecopy`, as the C's `error` is not read after it.
//!
//! ## Redesign
//! - LZ's packet as a parked `&'static Mbuf` with its header read and written as copies
//!   through `bsd`'s `mtod_ip6`, the stack copy put into a fresh mbuf by `copy_nonoverlapping`
//!   (one `unsafe`) and the destination handed to `bsd`'s `if_output_tso` as a raw socket
//!   address (one `unsafe`) -> the packet owned (`MbufBox`), the header a copy written back
//!   with `ip6_store` (in place: `ipv6_check` made the first mbuf writable), the stack copy
//!   written with `ip6_set_bytes` into the fresh mbuf, the destination a `SockaddrView` of a
//!   local `sockaddr_in6` for the kernel's `if_output_tso`.
//! - The C's `goto reroute` is a labelled loop whose exits name the C's labels (`senderr`,
//!   `icmperror`, `freecopy`); `done` is the function's end. The SA is a `TdbHold`, given back
//!   by its drop (the C's `tdb_unref` at `done`).
//! - The route: the caller's cache (the softnet's `ns_route` through `ip6_input_if`) or the
//!   call's own `iproute`; its route is lent by `Route::mpath6` and read again from the cache
//!   where the C keeps `rt` (no handle taken: the cache keeps the C's one reference).
//!
//! **Lock and spl transitions** against the C: none of its own (the caller holds the net
//! lock); the kernel lock inside the `Ipsec` leg, as the C's `ip6_output_ipsec_send`.

use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::netlock::NetHeld;
use hal::pod;
use hal::sys::errno::Errno;
use hal::sys::net_if::IFF_POINTOPOINT;
use hal::sys::net_route::{RTF_DYNAMIC, RTF_GATEWAY, RTF_MODIFIED};
use hal::sys::netinet_icmp6::{
    ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_ADDR, ICMP6_DST_UNREACH_BEYONDSCOPE,
    ICMP6_DST_UNREACH_NOROUTE, ICMP6_PACKET_TOO_BIG, ICMP6_TIME_EXCEED_TRANSIT,
    ICMP6_TIME_EXCEEDED, ICMPV6_PLD_MAXLEN, ND_REDIRECT,
};
use hal::sys::netinet_in::{IPPROTO_ESP, IPPROTO_TCP, IPPROTO_UDP};
use hal::sys::netinet_ip6::{IPV6_HLIMDEC, Ip6Hdr};
use hal::sys::netinet_tcp::Tcphdr;
use hal::sys::netinet_udp::Udphdr;
use hal::sys::netinet6_in6::{
    SockaddrIn6, in6_is_addr_multicast, in6_is_addr_unspecified, in6_is_scope_embed,
};
use hal::sys::netinet6_ip6_var::Ip6statCounters;
use hal::sys::socket::AF_INET6;

use crate::net::if_::{if_get, if_output_tso, if_put};
use crate::net::ipsec_hooks::{SpdError, TdbHold, ipsec};
use crate::net::mbuf::{
    M_BCAST, M_COPYFLAGS, M_DONTWAIT, M_MCAST, MHLEN, MT_DATA, MbufBox, PACKET_TAG_IPSEC_IN_DONE,
    PF_TAG_GENERATED, PF_TAG_REROUTE, m_copydata, m_copym, m_gethdr,
};
use crate::net::pf_hooks::{PF_FWD, PF_PASS, pf};
use crate::net::proto::SockaddrView;
use crate::net::route::Route;
use crate::netinet6::icmp6::{icmp6_error, icmp6_redirect_output};
use crate::netinet6::in6::in6_addr2scopeid;
use crate::netinet6::ip6_input::ip6_set_bytes;
use crate::netinet6::ip6_output::{ip6_load, ip6_store, ro_sin6};
use crate::netinet6::ip6_var::{
    IP6_SENDREDIRECTS, IPV6_FORWARDING, IPV6_FORWARDING_IPSEC, IPV6_REDIRECT, ip6stat_inc,
};
use crate::netinet6::nd6::nd6_is_addr_neighbor;

/// `MAX_TCPOPTLEN` (`<netinet/tcp.h>`): the longest TCP options.
const MAX_TCPOPTLEN: usize = 40;

/// Where `ip6_forward` goes once the packet left its hands: the C's labels.
enum Exit {
    /// `senderr`: the send failed or the packet was consumed; maybe an ICMPv6 message.
    Senderr,
    /// `icmperror`: send the ICMPv6 error of `type`/`code`.
    Icmperror,
    /// `freecopy`: drop the copy.
    Freecopy,
}

/// The bytes of a `sockaddr_in6`.
#[inline]
fn sin6_bytes(sin6: &SockaddrIn6) -> [u8; size_of::<SockaddrIn6>()] {
    let mut b = [0u8; size_of::<SockaddrIn6>()];
    let _ = pod::put(&mut b, 0, sin6);
    b
}

/// `ip6_forward(m, ro, flags)`: forward a packet. If some error occurs return the sender an
/// icmp packet. Note we can't always generate a meaningful icmp message because icmp doesn't
/// have a large enough repertoire of codes and types. If not forwarding, just drop the packet.
/// This could be confusing if ip6_forwarding was zero but some routing protocol was advancing
/// us as a gateway to somewhere. However, we must let the routing protocol deal with that.
/// The packet is consumed; the caller holds the net lock (`net`).
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function with its labels; split, its state would cross four calls"
)]
pub fn ip6_forward(m: MbufBox, ro: Option<&mut Route>, flags: i32, net: &NetHeld<'_>) {
    let mut m = m;
    let mut ip6 = ip6_load(&m);
    let mut iproute = Route::new();
    let mut ro = ro;
    let rtableid = m.m_pkthdr().ph_rtableid.get();
    let ifidx = m.m_pkthdr().ph_ifidx.get();
    let loopcnt = m.m_pkthdr().ph_loopcnt.get();
    let mut flags = flags;
    let mut error: Result<(), Errno> = Ok(());
    let mut type_: u8 = 0;
    let mut code: u8 = 0;
    let mut destmtu: i32 = 0;
    let mut ifp = None;
    let mut tdb: Option<TdbHold> = None;

    // Do not forward packets to multicast destination (should be handled by ip6_mforward().
    // Do not forward packets with unspecified source. It was discussed in July 2000, on
    // ipngwg mailing list.
    if m.flags() & (M_BCAST | M_MCAST) != 0
        || in6_is_addr_multicast(&ip6.ip6_dst)
        || in6_is_addr_unspecified(&ip6.ip6_src)
    {
        ip6stat_inc(Ip6statCounters::Ip6sCantforward);
        return;
    }

    if ip6.ip6_hlim <= IPV6_HLIMDEC {
        icmp6_error(m, ICMP6_TIME_EXCEEDED, ICMP6_TIME_EXCEED_TRANSIT, 0, net);
        return;
    }
    ip6.ip6_hlim -= IPV6_HLIMDEC;
    if !ip6_store(&mut m, &ip6) {
        // The header, writable since ipv6_check: not taken. Counted as the C counts a
        // forwarding it could not make.
        ip6stat_inc(Ip6statCounters::Ip6sCantforward);
        return;
    }

    // Save at most ICMPV6_PLD_MAXLEN (= the min IPv6 MTU - size of IPv6 + ICMPv6 headers)
    // bytes of the packet in case we need to generate an ICMP6 message to the src. Thanks to
    // M_EXT, in most cases copy will not occur. For small packets copy original onto stack
    // instead of mbuf.
    //
    // For final protocol header like TCP or UDP, full header chain in ICMP6 packet is not
    // necessary. In this case only copy small part of original packet and save it on stack
    // instead of mbuf. Although this violates RFC 4443 2.4. (c), it avoids additional mbuf
    // allocations. Also pf nat and rdr do not affect the shared mbuf cluster.
    //
    // It is important to save it before IPsec processing as IPsec processing may modify the
    // mbuf.
    let mut icmp_len: usize = match i32::from(ip6.ip6_nxt) {
        IPPROTO_TCP => size_of::<Ip6Hdr>() + size_of::<Tcphdr>() + MAX_TCPOPTLEN,
        IPPROTO_UDP => size_of::<Ip6Hdr>() + size_of::<Udphdr>(),
        IPPROTO_ESP => size_of::<Ip6Hdr>() + 2 * size_of::<u32>(),
        _ => ICMPV6_PLD_MAXLEN,
    };
    let pktlen = m.m_pkthdr().len.get().max(0) as usize;
    if icmp_len > pktlen {
        icmp_len = pktlen;
    }
    let mut icmp_buf = [0u8; MHLEN];
    let mut mflags = 0;
    let mut pfflags = 0;
    let mut mcopy: Option<MbufBox> = None;
    if icmp_len <= icmp_buf.len() {
        mflags = m.flags();
        pfflags = m.m_pkthdr().pf.flags.get();
        m_copydata(&m, 0, &mut icmp_buf[..icmp_len]);
    } else {
        mcopy = m_copym(&mut m, 0, icmp_len as i32, M_DONTWAIT);
        icmp_len = 0;
    }
    // The copy on the stack as a packet, for the ICMPv6 message.
    let stack_copy = |icmp_buf: &[u8; MHLEN]| -> Option<MbufBox> {
        let mut mc = m_gethdr(M_DONTWAIT, MT_DATA)?;
        mc.m_len().set(icmp_len as u32);
        let ph = mc.m_pkthdr();
        ph.len.set(icmp_len as i32);
        ph.ph_rtableid.set(rtableid);
        ph.ph_ifidx.set(ifidx);
        ph.ph_loopcnt.set(loopcnt);
        ph.pf
            .flags
            .set(ph.pf.flags.get() | (pfflags & PF_TAG_GENERATED));
        mc.m_flags().set(mc.flags() | (mflags & M_COPYFLAGS));
        // A fresh packet header mbuf has MHLEN bytes, `icmp_len` of them at most here.
        if !ip6_set_bytes(&mut mc, 0, &icmp_buf[..icmp_len]) {
            return None;
        }
        Some(mc)
    };

    let orig_rtableid = m.m_pkthdr().ph_rtableid.get();

    let exit = 'reroute: loop {
        // IPSEC
        if let Some(legs) = ipsec()
            && legs.in_use()
        {
            match legs.ip6_output_lookup(&m, None) {
                Ok(t) => tdb = t.map(|t| TdbHold::new(legs, t)),
                // -EINVAL is used to indicate that the packet should be silently dropped,
                // typically because we've asked key management for an SA.
                Err(SpdError::Drop | SpdError::Errno(_)) => {
                    break 'reroute Exit::Freecopy;
                }
            }
        }

        // The caller's route, or ours (iproute) when it gave none or pf rerouted.
        let r: &mut Route = match ro.as_deref_mut() {
            Some(r) => r,
            None => {
                drop(iproute.take_rt());
                &mut iproute
            }
        };
        let Some(rt) = r.mpath6(
            &ip6.ip6_dst.s6_addr,
            Some(&ip6.ip6_src.s6_addr),
            m.m_pkthdr().ph_rtableid.get(),
            net,
        ) else {
            ip6stat_inc(Ip6statCounters::Ip6sNoroute);
            type_ = ICMP6_DST_UNREACH;
            code = ICMP6_DST_UNREACH_NOROUTE;
            break 'reroute Exit::Icmperror;
        };
        let (rt_ifidx, rt_flags) = (rt.rt_ifidx.get(), rt.rt_flags.get());
        let gw = if rt_flags & RTF_GATEWAY != 0 {
            rt.with_gateway(|g| g.and_then(|g| pod::at::<SockaddrIn6>(g, 0)))
        } else {
            None
        };
        let mut dst: SockaddrIn6 = ro_sin6(r);

        // Scope check: if a packet can't be delivered to its destination for the reason that
        // the destination is beyond the scope of the source address, discard the packet and
        // return an icmp6 destination unreachable error with Code 2 (beyond scope of source
        // address). [draft-ietf-ipngwg-icmp-v3-00.txt, Section 3.1]
        if in6_addr2scopeid(ifidx, &ip6.ip6_src) != in6_addr2scopeid(rt_ifidx, &ip6.ip6_src) {
            ip6stat_inc(Ip6statCounters::Ip6sCantforward);
            ip6stat_inc(Ip6statCounters::Ip6sBadscope);
            type_ = ICMP6_DST_UNREACH;
            code = ICMP6_DST_UNREACH_BEYONDSCOPE;
            break 'reroute Exit::Icmperror;
        }

        // IPSEC: Check if the packet needs encapsulation. ipsp_process_packet will never come
        // back to here.
        if let Some(t) = tdb.as_ref() {
            // Callee frees mbuf
            error = t.legs().ip6_output_send(t.tdb(), m, r, orig_rtableid, true);
            if error.is_err() {
                break 'reroute Exit::Senderr;
            }
            break 'reroute Exit::Freecopy;
        }

        if let Some(g) = gw {
            dst = g;
        }

        // If we are to forward the packet using the same interface as one we got the packet
        // from, perhaps we should send a redirect to sender to shortcut a hop. Only send
        // redirect if source is sending directly to us, and if packet was not source routed
        // (or has any options). Also, don't send redirect if forwarding using a route
        // modified by a redirect.
        if_put(ifp.take());
        ifp = if_get(rt_ifidx);
        let Some(i) = ifp else {
            break 'reroute Exit::Freecopy;
        };
        if rt_ifidx == ifidx
            && rt_flags & (RTF_DYNAMIC | RTF_MODIFIED) == 0
            && flags & IPV6_REDIRECT == 0
            && IP6_SENDREDIRECTS.load(Ordering::Relaxed) != 0
        {
            if i.if_flags.get() & IFF_POINTOPOINT != 0 && nd6_is_addr_neighbor(&ro_sin6(r), i, net)
            {
                // If the incoming interface is equal to the outgoing one, the link attached to
                // the interface is point-to-point, and the IPv6 destination is regarded as
                // on-link on the link, then it will be highly probable that the destination
                // address does not exist on the link and that the packet is going to loop.
                // Thus, we immediately drop the packet and send an ICMPv6 error message. For
                // other routing loops, we dare to let the packet go to the loop, so that a
                // remote diagnosing host can detect the loop by traceroute. type/code is based
                // on suggestion by Rich Draves. not sure if it is the best pick.
                type_ = ICMP6_DST_UNREACH;
                code = ICMP6_DST_UNREACH_ADDR;
                break 'reroute Exit::Icmperror;
            }
            type_ = ND_REDIRECT;
        }

        // Fake scoped addresses. Note that even link-local source or destination can appear,
        // if the originating node just sends the packet to us (without address resolution for
        // the destination). Since both icmp6_error and icmp6_redirect_output fill the embedded
        // link identifiers, we can do this stuff after making a copy for returning an error.
        if in6_is_scope_embed(&ip6.ip6_src) || in6_is_scope_embed(&ip6.ip6_dst) {
            if in6_is_scope_embed(&ip6.ip6_src) {
                ip6.ip6_src.set_s6_addr16(1, 0);
            }
            if in6_is_scope_embed(&ip6.ip6_dst) {
                ip6.ip6_dst.set_s6_addr16(1, 0);
            }
            if !ip6_store(&mut m, &ip6) {
                ip6stat_inc(Ip6statCounters::Ip6sCantforward);
                break 'reroute Exit::Freecopy;
            }
        }

        // NPF > 0: Packet filter
        if let Some(p) = pf() {
            let (action, mm) = p.test(AF_INET6, PF_FWD, i, m);
            if action != PF_PASS {
                drop(mm);
                break 'reroute Exit::Senderr;
            }
            let Some(mm) = mm else {
                break 'reroute Exit::Senderr;
            };
            m = mm;
            ip6 = ip6_load(&m);
            let pff = &m.m_pkthdr().pf.flags;
            if pff.get() & (PF_TAG_REROUTE | PF_TAG_GENERATED)
                == (PF_TAG_REROUTE | PF_TAG_GENERATED)
            {
                // already rerun the route lookup, go on
                pff.set(pff.get() & !(PF_TAG_GENERATED | PF_TAG_REROUTE));
            } else if pff.get() & PF_TAG_REROUTE != 0 {
                // tag as generated to skip over pf_test on rerun
                pff.set(pff.get() | PF_TAG_GENERATED);
                flags |= IPV6_REDIRECT;
                // iproute's route given back (rtfree), the caller's left to the caller.
                drop(iproute.take_rt());
                ro = None;
                if_put(ifp.take());
                continue 'reroute;
            }
        }

        // IPSEC
        if flags & IPV6_FORWARDING != 0
            && flags & IPV6_FORWARDING_IPSEC != 0
            && m.m_pkthdr().ph_tagsset.get() & PACKET_TAG_IPSEC_IN_DONE == 0
        {
            error = Err(Errno::EHOSTUNREACH);
            // The C keeps the packet here (see the module's deviations).
            break 'reroute Exit::Senderr;
        }

        let db = sin6_bytes(&dst);
        let Some(dv) = SockaddrView::new(&db) else {
            break 'reroute Exit::Freecopy;
        };
        let mut mp = Some(m);
        // The route again from its cache (the C's `rt`).
        let r: &Route = match ro.as_deref() {
            Some(r) => r,
            None => &iproute,
        };
        error = if_output_tso(i, &mut mp, dv, r.rt(), i.if_mtu.get());
        if error.is_err() {
            ip6stat_inc(Ip6statCounters::Ip6sCantforward);
        } else if mp.is_none() {
            ip6stat_inc(Ip6statCounters::Ip6sForward);
        }
        if error.is_err() || mp.is_none() {
            break 'reroute Exit::Senderr;
        }

        type_ = ICMP6_PACKET_TOO_BIG;
        destmtu = i.if_mtu.get() as i32;
        break 'reroute Exit::Icmperror;
    };

    // The route the C's `rt` names at `senderr` (the last lookup's, from its cache).
    let r: &Route = match ro.as_deref() {
        Some(r) => r,
        None => &iproute,
    };
    let exit = match exit {
        Exit::Senderr => 'senderr: {
            if mcopy.is_none() && icmp_len == 0 {
                break 'senderr Exit::Freecopy;
            }
            match error {
                Ok(()) => {
                    if type_ == ND_REDIRECT {
                        if icmp_len != 0 {
                            mcopy = stack_copy(&icmp_buf);
                        }
                        if let Some(mc) = mcopy.take() {
                            // A sent packet had a route.
                            if let Some(rt) = r.rt() {
                                icmp6_redirect_output(mc, rt, net);
                                ip6stat_inc(Ip6statCounters::Ip6sRedirectsent);
                            }
                        }
                        break 'senderr Exit::Freecopy;
                    }
                    Exit::Freecopy
                }
                Err(Errno::EMSGSIZE) => {
                    type_ = ICMP6_PACKET_TOO_BIG;
                    if let Some(rt) = r.rt() {
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
                    ip6stat_inc(Ip6statCounters::Ip6sCantfrag);
                    if destmtu == 0 {
                        Exit::Freecopy
                    } else {
                        Exit::Icmperror
                    }
                }
                // EACCES: pf(4) blocked the packet. There is no need to send an ICMP packet
                // back since pf(4) takes care of it. ENOBUFS: Tell source to slow down like
                // source quench in IP?
                Err(Errno::EACCES | Errno::ENOBUFS) => Exit::Freecopy,
                // ENETUNREACH (shouldn't happen, checked above), EHOSTUNREACH, ENETDOWN,
                // EHOSTDOWN, default:
                Err(_) => {
                    type_ = ICMP6_DST_UNREACH;
                    code = ICMP6_DST_UNREACH_ADDR;
                    Exit::Icmperror
                }
            }
        }
        other => other,
    };

    if let Exit::Icmperror = exit {
        if icmp_len != 0 {
            mcopy = stack_copy(&icmp_buf);
        }
        if let Some(mc) = mcopy.take() {
            icmp6_error(mc, type_, code, destmtu, net);
        }
    }
    // freecopy: the copy, if any is left, freed. done: iproute's route given back with it
    // (rtfree), the interface's reference, the SA's (tdb_unref) with its hold.
    if_put(ifp);
    drop(tdb);
}

// The stack copy holds the headers of a TCP segment (the C's CTASSERT on `icmp_buf`).
const _: () = assert!(size_of::<Ip6Hdr>() + size_of::<Tcphdr>() + MAX_TCPOPTLEN <= MHLEN);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of forwarding over ND's test interface (fd00:77::1/64, its neighbor
    // fd00:77::2) and a second interface without addresses: what is never forwarded, the hop
    // limit, no route, the scope check, a packet sent with its hop limit lowered, the redirect
    // decision, and the Packet Too Big answer from the stack copy and from the mbuf copy.

    use std::vec::Vec;

    use hal::sys::netinet_icmp6::ND_NA_FLAG_OVERRIDE;
    use hal::sys::netinet_icmp6::ND_NA_FLAG_SOLICITED;
    use hal::sys::netinet_in::IPPROTO_NONE;
    use hal::sys::netinet6_in6::In6Addr;

    use super::*;
    use crate::net::if_var::Ifnet;
    use crate::net::mbuf::m_devget;
    use crate::net::route::tests::{take_icmp6_errors, take_icmp6_params};
    use crate::netinet6::icmp6::tests::{gw_route, net6, packet};
    use crate::netinet6::ip6_var::IP6COUNTERS;
    use crate::netinet6::nd6::tests::{PEER, advertise, ether_bare, ours6, peer6, sent};

    fn stat(c: Ip6statCounters) -> u64 {
        IP6COUNTERS.read()[c as usize]
    }

    /// 2001:db8::`last`.
    fn far(last: u8) -> In6Addr {
        let mut a = In6Addr::default();
        a.s6_addr[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
        a.s6_addr[15] = last;
        a
    }

    /// A packet from `src` to `dst` (next header `nxt`, hop limit `hlim`, `len` bytes of
    /// payload) received on `ifp`.
    fn rx(ifp: &Ifnet, src: In6Addr, dst: In6Addr, nxt: i32, hlim: u8, len: usize) -> MbufBox {
        let b = packet(src, dst, nxt as u8, hlim, &std::vec![0x5a; len]);
        let m = m_devget(&b, 0).expect("a packet");
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        m
    }

    /// `ip6_forward` of `m` with no route cache, as pf calls it, under the net lock (taken
    /// detached, as ND's tests take it: the driver's output reaches `nd6_resolve`'s
    /// solicitation without the proof, which the `Inet` double lends from that holding).
    fn forward(m: MbufBox, flags: i32) {
        hal::netlock::lock();
        hal::netlock::lend_detached(|net| ip6_forward(m, None, flags, net));
        hal::netlock::unlock();
    }

    /// The IPv6 packets `ifp` sent since the last call (its Ethernet frames' payloads).
    fn sent6(ifp: &'static Ifnet) -> Vec<Vec<u8>> {
        sent(ifp)
            .into_iter()
            .filter(|f| f.len() > 14 && f[12..14] == [0x86, 0xdd])
            .map(|f| f[14..].to_vec())
            .collect()
    }

    #[test]
    fn what_is_never_forwarded_is_counted_and_dropped() {
        let (_g, ifp) = net6();
        let _ = take_icmp6_errors();
        let mut mc = far(1);
        mc.s6_addr[0] = 0xff;
        let cases: [(&str, In6Addr, In6Addr, u16); 4] = [
            ("a multicast destination", ours6(), mc, 0),
            ("an unspecified source", In6Addr::default(), far(1), 0),
            ("a link-layer multicast", ours6(), far(1), M_MCAST),
            ("a link-layer broadcast", ours6(), far(1), M_BCAST),
        ];
        for (what, src, dst, f) in cases {
            let m = rx(ifp, src, dst, IPPROTO_UDP, 64, 8);
            m.m_flags().set(m.flags() | f);
            let cant = stat(Ip6statCounters::Ip6sCantforward);
            forward(m, 0);
            assert_eq!(stat(Ip6statCounters::Ip6sCantforward), cant + 1, "{what}");
        }
        assert!(take_icmp6_errors().is_empty());
    }

    #[test]
    fn a_spent_hop_limit_or_no_route_is_answered() {
        let (_g, ifp) = net6();
        let _ = take_icmp6_errors();
        for hlim in [0, IPV6_HLIMDEC] {
            forward(rx(ifp, peer6(), far(1), IPPROTO_UDP, hlim, 8), 0);
            assert_eq!(
                take_icmp6_errors(),
                [(ICMP6_TIME_EXCEEDED, ICMP6_TIME_EXCEED_TRANSIT)]
            );
            assert_eq!(take_icmp6_params(), [0]);
        }
        let noroute = stat(Ip6statCounters::Ip6sNoroute);
        forward(
            rx(ifp, peer6(), far(1), IPPROTO_UDP, IPV6_HLIMDEC + 1, 8),
            0,
        );
        assert_eq!(stat(Ip6statCounters::Ip6sNoroute), noroute + 1);
        assert_eq!(
            take_icmp6_errors(),
            [(ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOROUTE)]
        );
    }

    #[test]
    fn a_link_local_source_is_not_forwarded_beyond_its_link() {
        let (_g, ifp) = net6();
        let other = ether_bare(b"tfw1");
        let _ = take_icmp6_errors();
        gw_route(ifp, far(1), peer6());
        let mut ll = peer6();
        ll.s6_addr[..8].copy_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0]);
        ll.set_s6_addr16(1, (other.if_index.get() as u16).to_be());
        let (cant, scope) = (
            stat(Ip6statCounters::Ip6sCantforward),
            stat(Ip6statCounters::Ip6sBadscope),
        );
        forward(rx(other, ll, far(1), IPPROTO_UDP, 64, 8), 0);
        assert_eq!(
            (
                stat(Ip6statCounters::Ip6sCantforward),
                stat(Ip6statCounters::Ip6sBadscope)
            ),
            (cant + 1, scope + 1)
        );
        assert_eq!(
            take_icmp6_errors(),
            [(ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_BEYONDSCOPE)]
        );
    }

    #[test]
    fn a_forwarded_packet_goes_out_with_its_hop_limit_lowered() {
        let (_g, ifp) = net6();
        let other = ether_bare(b"tfw2");
        let _ = take_icmp6_errors();
        gw_route(ifp, far(1), peer6());
        let (fwd, redir) = (
            stat(Ip6statCounters::Ip6sForward),
            stat(Ip6statCounters::Ip6sRedirectsent),
        );
        forward(rx(other, far(9), far(1), IPPROTO_UDP, 64, 8), 0);
        assert_eq!(stat(Ip6statCounters::Ip6sForward), fwd + 1);
        // Held for the gateway's address until it answers, then sent.
        advertise(
            ifp,
            peer6(),
            ND_NA_FLAG_SOLICITED | ND_NA_FLAG_OVERRIDE,
            Some(PEER),
        );
        let out: Vec<Vec<u8>> = sent6(ifp)
            .into_iter()
            .filter(|p| p[24..40] == far(1).s6_addr)
            .collect();
        assert_eq!(out.len(), 1, "the packet");
        assert_eq!(out[0][7], 63, "one hop less");
        assert_eq!(
            stat(Ip6statCounters::Ip6sRedirectsent),
            redir,
            "another interface"
        );
        assert!(take_icmp6_errors().is_empty());
    }

    #[test]
    fn a_packet_back_out_its_interface_earns_its_sender_a_redirect() {
        let (_g, ifp) = net6();
        let _ = take_icmp6_errors();
        gw_route(ifp, far(1), peer6());
        advertise(
            ifp,
            peer6(),
            ND_NA_FLAG_SOLICITED | ND_NA_FLAG_OVERRIDE,
            Some(PEER),
        );
        let redir = stat(Ip6statCounters::Ip6sRedirectsent);
        let mut src = peer6();
        src.s6_addr[15] = 3;
        forward(rx(ifp, src, far(1), IPPROTO_UDP, 64, 8), 0);
        assert_eq!(stat(Ip6statCounters::Ip6sRedirectsent), redir + 1);
        // Not when a route was rerouted (IPV6_REDIRECT), nor with net.inet6.ip6.redirect off.
        forward(rx(ifp, src, far(1), IPPROTO_UDP, 64, 8), IPV6_REDIRECT);
        IP6_SENDREDIRECTS.store(0, Ordering::Relaxed);
        forward(rx(ifp, src, far(1), IPPROTO_UDP, 64, 8), 0);
        IP6_SENDREDIRECTS.store(1, Ordering::Relaxed);
        assert_eq!(stat(Ip6statCounters::Ip6sRedirectsent), redir + 1);
        let _ = sent6(ifp);
    }

    #[test]
    fn a_packet_too_big_for_the_link_is_answered_with_its_mtu() {
        let (_g, ifp) = net6();
        let other = ether_bare(b"tfw3");
        let _ = take_icmp6_errors();
        gw_route(ifp, far(1), peer6());
        advertise(
            ifp,
            peer6(),
            ND_NA_FLAG_SOLICITED | ND_NA_FLAG_OVERRIDE,
            Some(PEER),
        );
        ifp.if_mtu.set(1280);
        // UDP's headers on the stack; a payload with no transport header copied in an mbuf.
        for nxt in [IPPROTO_UDP, IPPROTO_NONE] {
            forward(rx(other, far(9), far(1), nxt, 64, 1400), 0);
            assert_eq!(take_icmp6_errors(), [(ICMP6_PACKET_TOO_BIG, 0)], "{nxt}");
            assert_eq!(take_icmp6_params(), [1280]);
        }
        // The stack copy keeps the packet's flags of M_COPYFLAGS and pf's "generated" tag
        // alone (the C's mflags and pfflags), not the rest of its header.
        let _ = crate::netinet6::ip6_input::ip6_send_test_take();
        let m = rx(other, peer6(), far(1), IPPROTO_UDP, 64, 1400);
        m.m_pkthdr()
            .pf
            .flags
            .set(PF_TAG_GENERATED | crate::net::mbuf::PF_TAG_TRANSLATE_LOCALHOST);
        forward(m, 0);
        let _ = take_icmp6_errors();
        let mut q = crate::netinet6::ip6_input::ip6_send_test_take();
        let e = crate::net::mbuf::ml_dequeue(&mut q).expect("the error sent");
        assert_eq!(e.m_pkthdr().pf.flags.get(), PF_TAG_GENERATED);
        ifp.if_mtu.set(1500);
        let _ = sent6(ifp);
    }
}
/* </TESTS> */
