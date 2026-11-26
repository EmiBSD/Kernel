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
//! Router advertisements and solicitations seen by a host, and route flushing:
//! `netinet6/nd6_rtr.c`.
//!
//! Upstream: sys/netinet6/nd6_rtr.c @ 3ce1f3f79392
//! LZ: sys/netinet6/nd6_rtr.rs@3c62ede68802
//!
//! The kernel no longer processes router advertisements (slaacd(8) does, from a raw socket);
//! what is left is caching the link-layer address a router solicitation or advertisement
//! carries ([`nd6_rtr_cache`]), and [`rt6_flush`], which removes the host routes through a
//! router that stopped being one.
//!
//! ## Deviations
//! - A message shorter than its fixed part (which `icmp6_input` already refuses) counts as
//!   `icp6s_tooshort` instead of being read past (LZ's).
//! - `rt6_deleteroute` is the closure `rtable_walk` takes (a `void *` argument in C); its
//!   `EEXIST` is the walk's error (LZ's).
//!
//! ## Redesign
//! - The message is read from the packet's bytes (`nd6_nbr::nd6_msg`, the C's
//!   `ip6_exthdr_get`), its options a slice of it (`nd6::nd6_options`, offsets): LZ's
//!   `from_raw_parts` over the pointer `ip6_exthdr_get` returned and `nd6_opt_lladdr`'s
//!   `unsafe fn` go.
//! - `nd6_rtr_cache` takes the packet (`MbufBox`) and the net lock's proof; `rt6_flush` the
//!   proof, the route `rtable_walk` stopped at a counted handle (`rtfree` its drop).
//!
//! **Lock and spl transitions** against the C: the net lock held by the caller (the exclusive
//! one, which `nd6_cache_lladdr` asserts); `KERNEL_LOCK` around `rtrequest_delete`, C-style
//! (`hal::sync::klock`); `rtable_walk`'s own table lock per walk, as the C's.

use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::mem::pod;
use hal::mem::pool::PoolArc;
use hal::sync::netlock::NetHeld;
use sysdefs::base::errno::Errno;
use sysdefs::net::route::{RTAX_DST, RTAX_GATEWAY, RTAX_NETMASK, RTF_HOST, RTF_STATIC, RTP_ANY};
use sysdefs::net::socket::AF_INET6;
use sysdefs::netinet::icmp6::{
    Icmp6statCounters, ND_ROUTER_ADVERT, ND_ROUTER_SOLICIT, NdRouterAdvert, NdRouterSolicit,
};
use sysdefs::netinet6::in6::{
    In6Addr, in6_are_addr_equal, in6_is_addr_linklocal, in6_is_addr_unspecified,
};

use crate::assert::kassert;
use crate::net::if_::IfRef;
use crate::net::if_var::Ifnet;
use crate::net::mbuf::MbufBox;
use crate::net::route::{RtAddrinfo, Rtentry, rt_plen2mask, rtrequest_delete};
use crate::net::rtable::{SaCopy, rtable_walk};
use crate::netinet6::ip6_var::{IP6_FORWARDING, icmp6stat_inc};
use crate::netinet6::nd6::{NdOpts, nd6_cache_lladdr, nd6_opt_lladdr, nd6_options};
use crate::netinet6::nd6_nbr::{nd6_msg, nd6_msg_bytes};

/// `nd6_rtr_cache(m, off, icmp6len, icmp6_type)`: process Source Link-layer Address Options
/// from Router Solicitation / Advertisement Messages of `icmp6_type` (`icmp6len` bytes at
/// `off` of `m`, consumed). Under the exclusive net lock (`net`).
pub fn nd6_rtr_cache(m: MbufBox, off: i32, icmp6len: i32, icmp6_type: u8, net: &NetHeld<'_>) {
    // The IPv6 header's hop limit and source (its first mbuf holds it: ip6_input's pullup).
    let d = m.data();
    let (Some(&hlim), Some(saddr6)) = (d.get(7), pod::at::<In6Addr>(d, 8)) else {
        return; // m freed
    };
    let i_am_router = IP6_FORWARDING.load(Ordering::Relaxed) != 0;

    kassert!(icmp6_type == ND_ROUTER_SOLICIT || icmp6_type == ND_ROUTER_ADVERT);

    let bad = 'out: {
        // Sanity checks
        if hlim != 255 {
            break 'out true;
        }

        let fixed = if icmp6_type == ND_ROUTER_SOLICIT {
            // Don't update the neighbor cache, if src = ::. This indicates that the src has no
            // IP address assigned yet.
            if in6_is_addr_unspecified(&saddr6) {
                break 'out false;
            }
            size_of::<NdRouterSolicit>()
        } else {
            if !in6_is_addr_linklocal(&saddr6) {
                break 'out true;
            }
            size_of::<NdRouterAdvert>()
        };

        let got = if (icmp6len as usize) < fixed {
            None
        } else {
            nd6_msg(m, off, icmp6len)
        };
        let Some((m, at, o)) = got else {
            icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
            return;
        };
        let Some(msg) = nd6_msg_bytes(&m, at, o, icmp6len as usize) else {
            icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
            return;
        };

        let opts = &msg[fixed..];
        let mut ndopts = NdOpts::default();
        if !nd6_options(opts, &mut ndopts) {
            // nd6_options have incremented stats
            break 'out false;
        }

        let (lladdr, lladdrlen) = match ndopts.nd_opts_src_lladdr {
            Some(off) => {
                let (l, n) = nd6_opt_lladdr(opts, off);
                (Some(l), n)
            }
            None => (None, 0),
        };

        let Some(ifp) = IfRef::get(m.m_pkthdr().ph_ifidx.get()) else {
            break 'out false;
        };

        if lladdr.is_some() && ((i32::from(ifp.if_addrlen.get()) + 2 + 7) & !7) != lladdrlen {
            break 'out true; // if_put(ifp): its drop
        }

        nd6_cache_lladdr(
            ifp.ifp(),
            &saddr6,
            lladdr,
            i32::from(icmp6_type),
            0,
            i_am_router,
            net,
        );
        false // if_put(ifp), freeit
    };

    if bad {
        icmp6stat_inc(if icmp6_type == ND_ROUTER_SOLICIT {
            Icmp6statCounters::Icp6sBadrs
        } else {
            Icmp6statCounters::Icp6sBadra
        });
    }
    // freeit: m freed
}

/// `rt6_flush(gateway, ifp)`: delete all the routing table entries that use the specified
/// gateway. XXX: this function causes search through all entries of routing table, so it
/// shouldn't be called when acting as a router. The gateway must already contain KAME's hack
/// for link-local scope. Under the net lock (`net`).
pub fn rt6_flush(gateway: &In6Addr, ifp: &'static Ifnet, net: &NetHeld<'_>) -> Result<(), Errno> {
    kassert!(matches!(
        hal::sync::netlock::status(),
        hal::sync::rwlock::RW_WRITE | hal::sync::rwlock::RW_READ
    ));

    // We'll care only link-local addresses
    if !in6_is_addr_linklocal(gateway) {
        return Ok(());
    }

    kassert!(gateway.s6_addr16(1) != 0);

    loop {
        let mut rt: Option<PoolArc<Rtentry>> = None;
        let mut error = rtable_walk(ifp.if_rdomain.get(), AF_INET6, Some(&mut rt), |rt, id| {
            rt6_deleteroute(rt, gateway, id)
        });
        if let Some(r) = &rt
            && error == Err(Errno::EEXIST)
        {
            let mut info = RtAddrinfo::new();
            let mut sa_mask = SaCopy::zeroed();
            let gw = r.gateway_copy();
            info.rti_flags = r.rt_flags.get();
            info.rti_info[RTAX_DST] = Some(r.dest_view());
            info.rti_info[RTAX_GATEWAY] = gw.as_ref().map(SaCopy::view);
            info.rti_info[RTAX_NETMASK] = rt_plen2mask(r, &mut sa_mask);
            hal::sync::klock::kernel_lock();
            error = rtrequest_delete(&mut info, RTP_ANY, ifp, None, ifp.if_rdomain.get(), net);
            hal::sync::klock::kernel_unlock();
            if error.is_ok() {
                error = Err(Errno::EAGAIN);
            }
        }
        drop(rt); // rtfree(rt)
        if error != Err(Errno::EAGAIN) {
            return error;
        }
    }
}

/// `rt6_deleteroute(rt, arg, id)`: whether `rt` is a host route through `gate` that `rt6_flush`
/// should delete (`Err(EEXIST)` stops the walk at it).
fn rt6_deleteroute(rt: &Rtentry, gate: &In6Addr, _id: u32) -> Result<(), Errno> {
    let Some(g) = rt.gateway_copy() else {
        return Ok(());
    };
    if g.view().family() != AF_INET6 {
        return Ok(());
    }

    // An `AF_INET6` gateway is a `sockaddr_in6`: its address at offset 8.
    let ga = pod::at::<In6Addr>(g.bytes(), 8).unwrap_or_default();
    if !in6_are_addr_equal(gate, &ga) {
        return Ok(());
    }

    // Do not delete a static route.
    // XXX: this seems to be a bit ad-hoc. Should we consider the 'cloned' bit instead?
    if rt.rt_flags.isset(RTF_STATIC) {
        return Ok(());
    }

    // We delete only host route. This means, in particular, we don't delete default route.
    if !rt.rt_flags.isset(RTF_HOST) {
        return Ok(());
    }

    Err(Errno::EEXIST)
}
/* </CODE> */
