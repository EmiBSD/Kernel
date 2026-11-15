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
 *	@(#)in_pcb.c	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 source address and route selection, the hop limit of a pcb, and the embedding of
//! scope zone ids in addresses: `netinet6/in6_src.c`.
//!
//! Upstream: sys/netinet6/in6_src.c @ 3ce1f3f79392
//! LZ: sys/netinet6/in6_src.rs@3c62ede68802
//!
//! ## Deviations
//! - `in6_pcbselsrc` and `in6_selectsrc` answer the address by value (`Ok(In6Addr)`), where
//!   the C returns a pointer to it (into the pcb, the packet info or the interface address).
//! - `in6_selectif` is file-local in C and here; its `struct ifnet **retifp` is the returned
//!   `Option<IfRef>` (a reference from `if_get`, whose drop is `if_put`).
//! - `in6_selectsrc`'s route-based fallback (`route6_mpath` and the route's interface address)
//!   is in `in6_pcbselsrc`, as in the C; a route whose interface address is not an IPv6 one is
//!   treated like a route without one (the C reinterprets the structure).
//!
//! ## Redesign
//! - The packet options and the multicast options (`struct ip6_pktopts`, `struct
//!   ip6_moptions`), `bsd`'s `malloc`'d structures behind the pcb's raw words until N6.5c,
//!   are read by `in6_src.c` for one member each: the packet info (`ip6po_pktinfo`, its
//!   interface and its address, which `in6_pcbselsrc` overrides in place) and the multicast
//!   interface (`im6o_ifidx`). The functions take those, `Option<&mut In6Pktinfo>`,
//!   `pi_ifindex` and `im6o_ifidx` (D-22 of N6.5, with the override's `&mut`), so they never
//!   deref the words (LZ's 9 `unsafe` over `NonNull`s); `bsd`'s adapter copies the members
//!   out of the words for its callers until the options are the kernel's.
//! - The routes are the kernel's: `in6_selectroute` lends the cached route of the caller's
//!   `Route` (`Route::mpath6`), `in6_pcbselsrc` borrows the pcb's (`Inpcb::route`'s loan) and
//!   the net lock's proof (`net`), where LZ returned `&'static` routes through `bsd`'s
//!   `as_static`.
//! - `ifatoia6(ifa_ifwithaddr(..))` and the route's address are `In6Ifaddr::of` (the
//!   records' back-link), no read of the family through the generic part's pointer.
//!
//! **Lock and spl transitions** against the C: none; the pcb's route loan is two word moves
//! (N6.4c1's `InpLoan`), not a lock.

use hal::netlock::NetHeld;
use hal::pod;
use hal::sys::endian::{htons, ntohs};
use hal::sys::errno::Errno;
use hal::sys::net_if::IFF_UP;
use hal::sys::net_route::{RTF_BLACKHOLE, RTF_GATEWAY, RTF_HOST, RTF_LOCAL, RTF_REJECT};
use hal::sys::netinet6_in6::{
    In6Addr, In6Pktinfo, SockaddrIn6, in6_is_addr_linklocal, in6_is_addr_mc_intfacelocal,
    in6_is_addr_mc_linklocal, in6_is_addr_multicast, in6_is_addr_unspecified, in6_is_scope_embed,
};
use hal::sys::netinet6_in6_var::{IN6_IFF_ANYCAST, IN6_IFF_DUPLICATED, IN6_IFF_TENTATIVE};
use hal::sys::socket::AF_INET6;

use crate::net::if_::{IfRef, ifa_ifwithaddr};
use crate::net::route::{Route, Rtentry};
use crate::net::rtable::rtable_getsource;
use crate::netinet::in_pcb::Inpcb;
use crate::netinet6::in6::in6_ifawithscope;
use crate::netinet6::in6_var::{In6Ifaddr, ia6_in6, ifatoia6};
use crate::netinet6::ip6_var::IP6_DEFHLIM;
use crate::proc::ProcGlue;

/// `in6_pcbselsrc(&in6src, dstsock, inp, opts)`: an IPv6 address, the most appropriate for
/// destination `dstsock` and pcb `inp`. `pi` is the packet info of the options in force (a
/// control message's or the pcb's sticky one), whose address the C overrides in place with
/// the scoped form it validated; `im6o_ifidx` the pcb's multicast interface (0: none). Under
/// the net lock (`net`).
pub fn in6_pcbselsrc<G: ProcGlue>(
    dstsock: &SockaddrIn6,
    inp: &Inpcb<G>,
    pi: Option<&mut In6Pktinfo>,
    im6o_ifidx: Option<u16>,
    net: &NetHeld<'_>,
) -> Result<In6Addr, Errno> {
    let dst = &dstsock.sin6_addr;
    let laddr = inp.inp_laddr6.get::<In6Addr>();
    let rtableid = inp.inp_rtableid.get();
    let pi_ifindex = pi.as_deref().map_or(0, |p| p.ipi6_ifindex);

    // If the source address is explicitly specified by the caller, check if the requested
    // source address is indeed a unicast address assigned to the node, and can be used as the
    // packet's source address. If everything is okay, use the address as source.
    if let Some(pi) = pi
        && !in6_is_addr_unspecified(&pi.ipi6_addr)
    {
        // get the outgoing interface
        let ifp = {
            let mut ro = inp.route();
            in6_selectif(dst, pi_ifindex, im6o_ifidx, &mut ro, rtableid, net)?
        };

        let mut sa6 = SockaddrIn6::with_addr(pi.ipi6_addr);
        if let Some(ifp) = &ifp
            && in6_is_scope_embed(&sa6.sin6_addr)
        {
            sa6.sin6_addr
                .set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        }
        drop(ifp); // put reference from in6_selectif

        let mut b = [0u8; core::mem::size_of::<SockaddrIn6>()];
        let _ = pod::put(&mut b, 0, &sa6);
        let rd = net.reader();
        let ia6 = ifa_ifwithaddr(net, &rd, &b, rtableid).and_then(In6Ifaddr::of);
        let Some(ia6) = ia6 else {
            return Err(Errno::EADDRNOTAVAIL);
        };
        if ia6.ia6_flags.get() & (IN6_IFF_ANYCAST | IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED) != 0 {
            return Err(Errno::EADDRNOTAVAIL);
        }

        pi.ipi6_addr = sa6.sin6_addr; // XXX: this overrides pi

        return Ok(sa6.sin6_addr);
    }

    // If the source address is not specified but the socket(if any) is already bound, use the
    // bound address.
    if !in6_is_addr_unspecified(&laddr) {
        return Ok(laddr);
    }

    // If the caller doesn't specify the source address but the outgoing interface, use an
    // address associated with the interface.
    if pi_ifindex != 0 {
        let Some(ifp) = IfRef::get(pi_ifindex) else {
            return Err(Errno::ENXIO); // XXX: better error?
        };

        let ia6 = in6_ifawithscope(&ifp, dst, rtableid, None, &net.reader())
            .map(|ia| ia6_in6(ifatoia6(ia)));
        drop(ifp); // if_put(ifp)

        return ia6.ok_or(Errno::EADDRNOTAVAIL);
    }

    match in6_selectsrc(dstsock, im6o_ifidx, rtableid, net) {
        Err(Errno::EADDRNOTAVAIL) => {}
        error => return error,
    }

    // If route is known or can be allocated now, our src addr is taken from the i/f, else
    // punt.
    let mut ro = inp.route();
    let rt = ro.mpath6(&dst.s6_addr, None, rtableid, net);

    // in_pcbconnect() checks out IFF_LOOPBACK to skip using the address. But we don't know
    // why it does so. It is necessary to ensure the scope even for lo0 so doesn't check out
    // IFF_LOOPBACK.

    // One reader for the source selection: the scoped walk, the route's address and the
    // preferred one.
    let rd = net.reader();
    let mut ia6 = None;
    if let Some(rt) = rt {
        if let Some(ifp) = IfRef::get(rt.rt_ifidx.get()) {
            ia6 = in6_ifawithscope(&ifp, dst, rtableid, Some(rt), &rd).map(ifatoia6);
            // if_put(ifp): the reference's drop.
        }
        if ia6.is_none() {
            // xxx scope error ?
            ia6 = rt.ifa(&rd).and_then(In6Ifaddr::of);
        }
    }

    // Use preferred source address if :
    // - destination is not onlink
    // - preferred source address is set
    // - output interface is UP
    if let Some(rt) = rt
        && rt.rt_flags.get() & RTF_GATEWAY != 0
        && let Some(ip6_source) = rtable_getsource(rtableid, AF_INET6, net)
    {
        let ifa = ifa_ifwithaddr(net, &rd, ip6_source.bytes(), rtableid);
        if let Some(ifa) = ifa
            && ifa
                .ifa_ifp
                .get()
                .is_some_and(|ifp| ifp.if_flags.get() & IFF_UP != 0)
            && let Some(sin6) = pod::at::<SockaddrIn6>(ip6_source.bytes(), 0)
        {
            return Ok(sin6.sin6_addr);
        }
    }

    ia6.map(ia6_in6).ok_or(Errno::EHOSTUNREACH) // no route
}

/// `in6_selectsrc(&in6src, dstsock, mopts, rtableid)`: an IPv6 address, the most appropriate
/// for destination `dstsock` with multicast interface `im6o_ifidx` (`None`: no multicast
/// options), when the destination's scope or its multicast interface names one; else
/// `EADDRNOTAVAIL` (the caller looks the route up). Under the net lock `net` proves (the scoped
/// walks' reader).
pub fn in6_selectsrc(
    dstsock: &SockaddrIn6,
    im6o_ifidx: Option<u16>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<In6Addr, Errno> {
    let dst = &dstsock.sin6_addr;

    // If the destination address is a link-local unicast address or a link/interface-local
    // multicast address, and if the outgoing interface is specified by the sin6_scope_id
    // filed, use an address associated with the interface.
    // XXX: We're now trying to define more specific semantics of sin6_scope_id field, so this
    //      part will be rewritten in the near future.
    if (in6_is_addr_linklocal(dst)
        || in6_is_addr_mc_linklocal(dst)
        || in6_is_addr_mc_intfacelocal(dst))
        && dstsock.sin6_scope_id != 0
    {
        let Some(ifp) = IfRef::get(dstsock.sin6_scope_id) else {
            return Err(Errno::ENXIO); // XXX: better error?
        };

        let ia6 = in6_ifawithscope(&ifp, dst, rtableid, None, &net.reader())
            .map(|ia| ia6_in6(ifatoia6(ia)));
        drop(ifp); // if_put(ifp)

        return ia6.ok_or(Errno::EADDRNOTAVAIL);
    }

    // If the destination address is a multicast address and the outgoing interface for the
    // address is specified by the caller, use an address associated with the interface. Even
    // if the outgoing interface is not specified, we also choose a loopback interface as the
    // outgoing interface.
    if in6_is_addr_multicast(dst) {
        let mut ifp = im6o_ifidx.and_then(|i| IfRef::get(u32::from(i)));

        if ifp.is_none() && dstsock.sin6_scope_id != 0 {
            ifp = IfRef::get(u32::from(htons(dstsock.sin6_scope_id as u16)));
        }

        if let Some(ifp) = ifp {
            let ia6 = in6_ifawithscope(&ifp, dst, rtableid, None, &net.reader())
                .map(|ia| ia6_in6(ifatoia6(ia)));
            drop(ifp); // if_put(ifp)

            return ia6.ok_or(Errno::EADDRNOTAVAIL);
        }
    }

    Err(Errno::EADDRNOTAVAIL)
}

/// `in6_selectroute(dst, opts, ro, rtableid)`: the route to `dst`, from the cache `ro` if
/// valid, else a newly allocated one cached there; `None` when there is none or it conflicts
/// with the packet info's interface `pi_ifindex` (0: none). Under the net lock (`net`).
pub fn in6_selectroute<'a>(
    dst: &In6Addr,
    pi_ifindex: u32,
    ro: &'a mut Route,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Option<&'a Rtentry> {
    // Use a cached route if it exists and is valid, else try to allocate a new one.
    let rt = ro.mpath6(&dst.s6_addr, None, rtableid, net);

    // Check if the outgoing interface conflicts with the interface specified by ipi6_ifindex
    // (if specified). Note that loopback interface is always okay. (this may happen when we
    // are sending a packet to one of our own addresses.)
    if pi_ifindex != 0
        && let Some(rt) = rt
        && rt.rt_flags.get() & RTF_LOCAL == 0
        && rt.rt_ifidx.get() != pi_ifindex
    {
        return None;
    }

    rt
}

/// `in6_selectif(dst, opts, mopts, ro, &retifp, rtableid)`: the outgoing interface for
/// `dst` (with a reference; the C's `*retifp`, which stays NULL, `Ok(None)`, when the route's
/// interface is gone).
fn in6_selectif(
    dst: &In6Addr,
    pi_ifindex: u32,
    im6o_ifidx: Option<u16>,
    ro: &mut Route,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<Option<IfRef>, Errno> {
    // If the caller specify the outgoing interface explicitly, use it.
    if pi_ifindex != 0
        && let Some(ifp) = IfRef::get(pi_ifindex)
    {
        return Ok(Some(ifp));
    }

    // If the destination address is a multicast address and the outgoing interface for the
    // address is specified by the caller, use it.
    if in6_is_addr_multicast(dst)
        && let Some(ifp) = im6o_ifidx.and_then(|i| IfRef::get(u32::from(i)))
    {
        return Ok(Some(ifp));
    }

    let Some(rt) = in6_selectroute(dst, pi_ifindex, ro, rtableid, net) else {
        return Err(Errno::EHOSTUNREACH);
    };

    // do not use a rejected or black hole route.
    // XXX: this check should be done in the L2 output routine. However, if we skipped this
    // check here, we'd see the following scenario:
    // - install a rejected route for a scoped address prefix (like fe80::/10)
    // - send a packet to a destination that matches the scoped prefix, with ambiguity about
    //   the scope zone.
    // - pick the outgoing interface from the route, and disambiguate the scope zone with the
    //   interface.
    // - ip6_output() would try to get another route with the "new" destination, which may be
    //   valid.
    // - we'd see no error on output.
    // Although this may not be very harmful, it should still be confusing. We thus reject the
    // case here.
    if rt.rt_flags.get() & (RTF_REJECT | RTF_BLACKHOLE) != 0 {
        return Err(if rt.rt_flags.get() & RTF_HOST != 0 {
            Errno::EHOSTUNREACH
        } else {
            Errno::ENETUNREACH
        });
    }

    Ok(IfRef::get(rt.rt_ifidx.get()))
}

/// `in6_selecthlim(inp)`: the hop limit for packets of `inp`: its `inp_hops` if set, else
/// `ip6_defhlim`.
#[inline]
pub fn in6_selecthlim<G: ProcGlue>(inp: &Inpcb<G>) -> i32 {
    if inp.inp_hops.get() >= 0 {
        return inp.inp_hops.get();
    }

    IP6_DEFHLIM.load(core::sync::atomic::Ordering::Relaxed)
}

/// `in6_embedscope(in6, sin6, outputopts6, moptions6)`: the kernel-internal form (scopeid
/// embedded into `s6_addr16[1]`) of the address of `sin6`, into `in6`. If the address scope
/// is link-local, embed the interface index in the address. The routine determines our
/// precedence between advanced API scope/interface specification and basic API
/// specification: the packet info's interface `pi_ifindex` (0: none), the multicast
/// interface `im6o_ifidx` (0: none), `sin6_scope_id`.
///
/// This function should be nuked in the future, when we get rid of embedded scopeid thing.
///
/// XXX actually, it is over-specification to return ifp against sin6_scope_id. there can be
/// multiple interfaces that belong to a particular scope zone (in specification, we have 1:N
/// mapping between a scope zone and interfaces). we may want to change the function to return
/// something other than ifp.
pub fn in6_embedscope(
    in6: &mut In6Addr,
    sin6: &SockaddrIn6,
    pi_ifindex: u32,
    im6o_ifidx: u16,
) -> Result<(), Errno> {
    *in6 = sin6.sin6_addr;

    // don't try to read sin6->sin6_addr beyond here, since the caller may ask us to overwrite
    // existing sockaddr_in6

    if in6_is_scope_embed(in6) {
        // KAME assumption: link id == interface id
        let scopeid = if pi_ifindex != 0 {
            pi_ifindex
        } else if in6_is_addr_multicast(in6) && im6o_ifidx != 0 {
            u32::from(im6o_ifidx)
        } else {
            sin6.sin6_scope_id
        };

        if scopeid != 0 {
            let Some(ifp) = IfRef::get(scopeid) else {
                return Err(Errno::ENXIO); // XXX EINVAL?
            };
            // XXX assignment to 16bit from 32bit variable
            in6.set_s6_addr16(1, htons((scopeid & 0xffff) as u16));
            drop(ifp); // if_put(ifp)
        }
    }

    Ok(())
}

/// `in6_recoverscope(sin6, in6)`: the standard `sockaddr_in6` of the embedded form `in6`;
/// touches `sin6_addr` and `sin6_scope_id` only.
///
/// This function should be nuked in the future, when we get rid of embedded scopeid thing.
pub fn in6_recoverscope(sin6: &mut SockaddrIn6, in6: &In6Addr) {
    sin6.sin6_addr = *in6;

    // don't try to read *in6 beyond here, since the caller may ask us to overwrite existing
    // sockaddr_in6

    sin6.sin6_scope_id = 0;
    if in6_is_scope_embed(in6) {
        // KAME assumption: link id == interface id
        let scopeid = u32::from(ntohs(sin6.sin6_addr.s6_addr16(1)));
        if scopeid != 0 {
            sin6.sin6_addr.set_s6_addr16(1, 0);
            sin6.sin6_scope_id = scopeid;
        }
    }
}

/// `in6_clearscope(addr)`: just clear the embedded scope identifier.
#[inline]
pub fn in6_clearscope(addr: &mut In6Addr) {
    if in6_is_scope_embed(addr) {
        addr.set_s6_addr16(1, 0);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the scope helpers and the destination-driven source choices over the
    // kernel's interfaces (LZ's tests of the pcb paths stay in bsd's adapter, over its
    // fixture).

    use hal::netlock::NETLOCK;

    use super::*;
    use crate::netinet6::in6::tests::{a6, iface, setup, test_ia6, zoned};

    /// A `sockaddr_in6` of `a` with scope id `scope_id`.
    fn sin6(a: &str, scope_id: u32) -> SockaddrIn6 {
        let mut s = SockaddrIn6::with_addr(a6(a));
        s.sin6_scope_id = scope_id;
        s
    }

    #[test]
    fn the_scope_is_embedded_by_precedence_recovered_and_cleared() {
        let _g = setup();
        let (ifp, _) = iface(b"tes0", [0x52, 0x54, 0, 0, 6, 1]);
        let (other, _) = iface(b"tes1", [0x52, 0x54, 0, 0, 6, 2]);
        let idx = ifp.if_index.get();
        let mut out = In6Addr::default();
        // The scope id, for a link-local address only.
        in6_embedscope(&mut out, &sin6("fe80::1", idx), 0, 0).expect("embed");
        assert_eq!(out, zoned(a6("fe80::1"), idx));
        in6_embedscope(&mut out, &sin6("2001:db8::1", idx), 0, 0).expect("embed");
        assert_eq!(out, a6("2001:db8::1"));
        // An absent interface; no scope id at all.
        assert_eq!(
            in6_embedscope(&mut out, &sin6("fe80::1", 4000), 0, 0),
            Err(Errno::ENXIO)
        );
        in6_embedscope(&mut out, &sin6("fe80::1", 0), 0, 0).expect("embed");
        assert_eq!(out, a6("fe80::1"));
        // The packet info's interface first, then the multicast interface (multicast only).
        let oidx = other.if_index.get();
        in6_embedscope(&mut out, &sin6("fe80::1", idx), oidx, 0).expect("embed");
        assert_eq!(out, zoned(a6("fe80::1"), oidx));
        in6_embedscope(&mut out, &sin6("ff02::1", idx), 0, oidx as u16).expect("embed");
        assert_eq!(out, zoned(a6("ff02::1"), oidx));
        in6_embedscope(&mut out, &sin6("fe80::1", idx), 0, oidx as u16).expect("embed");
        assert_eq!(out, zoned(a6("fe80::1"), idx));

        let mut sa = SockaddrIn6::default();
        in6_recoverscope(&mut sa, &zoned(a6("fe80::9"), idx));
        assert_eq!((sa.sin6_addr, sa.sin6_scope_id), (a6("fe80::9"), idx));
        in6_recoverscope(&mut sa, &a6("2001:db8::1"));
        assert_eq!((sa.sin6_addr, sa.sin6_scope_id), (a6("2001:db8::1"), 0));
        let mut a = zoned(a6("ff02::1"), idx);
        in6_clearscope(&mut a);
        assert_eq!(a, a6("ff02::1"));
    }

    #[test]
    fn the_source_of_a_scoped_or_multicast_destination() {
        let _g = setup();
        let (if1, _) = iface(b"tss0", [0x52, 0x54, 0, 0, 7, 1]);
        let (if2, _) = iface(b"tss1", [0x52, 0x54, 0, 0, 7, 2]);
        let (i1, i2) = (if1.if_index.get(), if2.if_index.get());
        let ll1 = test_ia6(if1, zoned(a6("fe80::1"), i1), 0);
        let ll2 = test_ia6(if2, zoned(a6("fe80::2"), i2), 0);
        let w = NETLOCK.write();
        let net = &NetHeld::of(&w).expect("the net lock");
        // A link-local destination with its scope id: that interface's address.
        let src = in6_selectsrc(&sin6("fe80::99", i2), None, 0, net).expect("src");
        assert_eq!(src, ia6_in6(ifatoia6(&ll2)));
        let src = in6_selectsrc(&sin6("ff02::1", i1), None, 0, net).expect("src");
        assert_eq!(src, ia6_in6(ifatoia6(&ll1)));
        assert_eq!(
            in6_selectsrc(&sin6("fe80::99", 4000), None, 0, net),
            Err(Errno::ENXIO)
        );
        // A multicast destination: the multicast interface's address.
        let src = in6_selectsrc(&sin6("ff05::1", 0), Some(i2 as u16), 0, net).expect("src");
        assert_eq!(src, ia6_in6(ifatoia6(&ll2)));
        // Nothing names an interface: the caller looks the route up.
        assert_eq!(
            in6_selectsrc(&sin6("ff05::1", 0), None, 0, net),
            Err(Errno::EADDRNOTAVAIL)
        );
        assert_eq!(
            in6_selectsrc(&sin6("2001:db8::99", 0), Some(i2 as u16), 0, net),
            Err(Errno::EADDRNOTAVAIL)
        );
        // A tentative address is no source.
        ifatoia6(&ll2).ia6_flags.set(IN6_IFF_TENTATIVE);
        assert_eq!(
            in6_selectsrc(&sin6("fe80::99", i2), None, 0, net),
            Err(Errno::EADDRNOTAVAIL)
        );
    }
}
/* </TESTS> */
