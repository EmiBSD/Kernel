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
//! Attaching IPv6 to an interface: the interface identifier, the link-local and loopback
//! addresses: `<netinet6/in6_ifattach.h>` and `netinet6/in6_ifattach.c`.
//!
//! Upstream: sys/netinet6/in6_ifattach.c @ 3ce1f3f79392
//! Upstream: sys/netinet6/in6_ifattach.h @ 3ce1f3f79392
//! LZ: sys/netinet6/in6_ifattach.rs@3c62ede68802
//!
//! The header only declares `in6_ifattach`, `in6_ifdetach` and `in6_ifattach_linklocal`;
//! they are defined here. This version of `in6_ifattach.c` has no stable-privacy (RFC 7217)
//! identifiers and no node-information group (`in6_get_soii_ifid`, `in6_nigroup`): the
//! interface identifier is the EUI-64 of a hardware address (`in6_get_hw_ifid`), borrowed
//! from another interface, or random (`in6_get_rand_ifid`).
//!
//! ## Deviations
//! - `in6_get_hw_ifid` returns `bool` (true: an identifier was made; the C's 0 / -1) and
//!   `in6_get_ifid` / `in6_get_rand_ifid` are public so the tests reach them (LZ's).
//! - `ifra_name` is a copy of `if_xname` (both `IFNAMSIZ` bytes) instead of `strlcpy`.
//! - `ip6_mrouter_detach` (`MROUTING`) is not configured: a comment in `in6_ifdetach`.
//! - `in6_ifattach_linklocal` panics where the C would dereference a NULL address (the
//!   link-local address `in6_update_ifa` just configured is not there).
//! - ND's `nd6_dad_start`, `nd6_purge` and `nd6_need_cache` are the kernel's
//!   (`kernel::netinet6::{nd6, nd6_nbr}`) since N6.5b: called directly (N6.5a's `Inet` legs
//!   and its copy of `nd6_need_cache` went).
//!
//! ## Redesign
//! - The EUI-64 of a hardware address: LZ read `if_sadl`'s link-level address through a raw
//!   pointer (three `unsafe`) -> the kernel's `Ifnet::sadl()` record, borrowed for the copy
//!   (`sdl_data` after the name, `sdl_alen` bytes), no `unsafe`.
//! - `in6_ifdetach`'s walk over `if_addrlist` filters by the records' back-links
//!   (`In6Ifaddr::of`), no family read through the address's pointer (LZ's `unsafe`).
//! - Every function takes the caller's holding of the net lock (`kernel::netinet6::in6`'s
//!   `NetHolding`, or the proof for `in6_ifdetach`): `in6_update_ifa` calls the driver.
//!
//! **Lock and spl transitions** against the C: none of its own; the steps' are `in6`'s.

use core::mem::size_of;
use core::ptr;

use hal::random::arc4random;
use hal::sync::netlock::NetHeld;
use hal::sys::endian::{htonl, htons};
use hal::sys::errno::Errno;
use hal::sys::net_if::{
    IFF_LOOPBACK, IFF_MULTICAST, IFF_POINTOPOINT, IFT_BRIDGE, IFT_CARP, IFT_ENC, IFT_ETHER,
    IFT_GIF, IFT_IEEE1394, IFT_IEEE80211, IFT_PFLOG, IFT_PFSYNC, IFT_WIREGUARD, IFXF_AUTOCONF6,
    IFXF_AUTOCONF6TEMP,
};
use hal::sys::net_route::{RTF_CLONING, RTF_CONNECTED, RTF_MPATH};
use hal::sys::netinet_ip6::IPV6_MMTU;
use hal::sys::netinet6_in6::{
    IN6ADDR_INTFACELOCAL_ALLNODES, IN6ADDR_LINKLOCAL_ALLNODES, IN6ADDR_LOOPBACK, IN6MASK64,
    IN6MASK128, In6Addr, SockaddrIn6,
};
use hal::sys::netinet6_in6_var::{IN6_IFF_TENTATIVE, In6Aliasreq};
use hal::sys::netinet6_nd6::ND6_INFINITE_LIFETIME;
use hal::sys::socket::AF_INET6;

use crate::assert::{kassert, kpanic};
use crate::net::if_::{IFNETLIST, if_addrhooks_run};
use crate::net::if_var::{IfaddrRef, Ifnet};
use crate::net::proto::SockaddrView;
use crate::net::route::{rt_ifa_add, rtalloc, rtdeletemsg};
use crate::net::rtable::rtable_loindex;
use crate::netinet6::in6::{
    NetHolding, in6_purgeaddr, in6_update_ifa, in6if_do_dad, in6ifa_ifpforlinklocal,
    in6ifa_ifpwithaddr, net_assert_locked,
};
use crate::netinet6::in6_var::{In6Ifaddr, SIN6_LEN, ifatoia6};
use crate::netinet6::nd6::{nd6_need_cache, nd6_purge};
use crate::netinet6::nd6_nbr::nd6_dad_start;

/// `EUI64_GBIT`: the group bit of an EUI-64.
const EUI64_GBIT: u8 = 0x01;
/// `EUI64_UBIT`: the universal/local bit of an EUI-64.
const EUI64_UBIT: u8 = 0x02;

/// `EUI64_TO_IFID(in6)`: converts the EUI-64 in the low half of `in6` into an IPv6
/// interface identifier (flips the universal/local bit).
fn eui64_to_ifid(in6: &mut In6Addr) {
    in6.s6_addr[8] ^= EUI64_UBIT;
}

/// `EUI64_GROUP(in6)`: the group bit of the EUI-64 in `in6`.
fn eui64_group(in6: &In6Addr) -> bool {
    in6.s6_addr[8] & EUI64_GBIT != 0
}

/// `in6_get_rand_ifid`: generate a random interface identifier; the upper 64 bits of `in6`
/// are preserved.
pub fn in6_get_rand_ifid(ifp: &Ifnet, in6: &mut In6Addr) {
    let _ = ifp;
    // arc4random_buf(&in6->s6_addr8[8], 8): two words of the stream.
    in6.s6_addr[8..12].copy_from_slice(&arc4random().to_ne_bytes());
    in6.s6_addr[12..16].copy_from_slice(&arc4random().to_ne_bytes());

    // make sure to set "u" bit to local, and "g" bit to individual.
    in6.s6_addr[8] &= !EUI64_GBIT; // g bit to "individual"
    in6.s6_addr[8] |= EUI64_UBIT; // u bit to "local"

    // convert EUI64 into IPv6 interface identifier
    eui64_to_ifid(in6);
}

/// `in6_get_hw_ifid`: get the interface identifier of the interface from its hardware
/// address; the upper 64 bits of `in6` are preserved. `false` (the C's -1) when the
/// interface has no usable one.
pub fn in6_get_hw_ifid(ifp: &Ifnet, in6: &mut In6Addr) -> bool {
    const ALLZERO: [u8; 8] = [0; 8];
    const ALLONE: [u8; 8] = [0xff; 8];

    // The link-level address, copied out of `if_sadl` (`LLADDR(sdl)`, `sdl_alen` bytes).
    let mut lladdr = [0u8; 16];
    let mut addrlen = {
        let sdl = ifp.sadl();
        let Some(sdl) = sdl.as_ref() else {
            return false;
        };
        let start = usize::from(sdl.sdl_nlen);
        let alen = usize::from(sdl.sdl_alen);
        let Some(a) = sdl.sdl_data.get(start..start + alen) else {
            return false;
        };
        if alen == 0 || alen > lladdr.len() {
            return false;
        }
        lladdr[..alen].copy_from_slice(a);
        alen
    };

    if matches!(ifp.if_type.get(), IFT_IEEE1394 | IFT_IEEE80211) {
        // IEEE1394 uses 16byte length address starting with EUI64
        addrlen = addrlen.min(8);
    }
    let addr = &lladdr[..addrlen];

    // get EUI64
    match ifp.if_type.get() {
        // IEEE802/EUI64 cases - what others?
        IFT_ETHER | IFT_CARP | IFT_IEEE1394 | IFT_IEEE80211 => {
            // look at IEEE802/EUI64 only
            if addrlen != 8 && addrlen != 6 {
                return false;
            }

            // check for invalid MAC address - on bsdi, we see it a lot since wildboar
            // configures all-zero MAC on pccard before card insertion.
            if addr == &ALLZERO[..addrlen] {
                return false;
            }
            if addr == &ALLONE[..addrlen] {
                return false;
            }

            // make EUI64 address
            if addrlen == 8 {
                in6.s6_addr[8..16].copy_from_slice(addr);
            } else {
                in6.s6_addr[8] = addr[0];
                in6.s6_addr[9] = addr[1];
                in6.s6_addr[10] = addr[2];
                in6.s6_addr[11] = 0xff;
                in6.s6_addr[12] = 0xfe;
                in6.s6_addr[13] = addr[3];
                in6.s6_addr[14] = addr[4];
                in6.s6_addr[15] = addr[5];
            }
        }

        IFT_GIF => {
            // RFC2893 says: "SHOULD use IPv4 address as ifid source". however, IPv4 address
            // is not very suitable as unique identifier source (can be renumbered). we
            // don't do this.
            return false;
        }

        _ => return false,
    }

    // sanity check: g bit must not indicate "group"
    if eui64_group(in6) {
        return false;
    }

    // convert EUI64 into IPv6 interface identifier
    eui64_to_ifid(in6);

    // sanity check: ifid must not be all zero, avoid conflict with subnet router anycast
    if in6.s6_addr[8] & !(EUI64_GBIT | EUI64_UBIT) == 0x00 && in6.s6_addr[9..16] == ALLZERO[..7] {
        return false;
    }

    true
}

/// `in6_get_ifid`: get interface identifier for the specified interface. If it is not
/// available on `ifp0`, borrow interface identifier from other information sources. Under
/// the net lock (the interface list).
pub fn in6_get_ifid(ifp0: &Ifnet, in6: &mut In6Addr) {
    // first, try to get it from the interface itself
    if in6_get_hw_ifid(ifp0, in6) {
        return;
    }

    net_assert_locked();

    // next, try to get it from some other hardware interface
    for ifp in &IFNETLIST {
        if ptr::eq(ifp, ifp0) {
            continue;
        }
        if in6_get_hw_ifid(ifp, in6) {
            return;
        }
    }

    // last resort: get from random number source
    in6_get_rand_ifid(ifp0, in6);
}

/// `in6_ifattach_linklocal`: configures the link-local address of `ifp`. `ifid` is used as
/// the EUI-64 if given, overriding the other EUI-64 sources (`None`: derived from the
/// hardware address, borrowed or random).
pub fn in6_ifattach_linklocal(
    ifp: &'static Ifnet,
    ifid: Option<&In6Addr>,
    net: &NetHolding<'_>,
) -> Result<(), Errno> {
    let mut ifra = In6Aliasreq::default();

    net_assert_locked();

    // configure link-local address.
    ifra.ifra_name = ifp.if_xname.get();
    let ifidx = ifp.if_index.get() as u16;
    {
        let a = &mut ifra.ifra_addr;
        a.sin6_family = AF_INET6;
        a.sin6_len = SIN6_LEN;
        a.sin6_addr.set_s6_addr16(0, htons(0xfe80));
        a.sin6_addr.set_s6_addr16(1, htons(ifidx));
        a.sin6_addr.set_s6_addr32(1, 0);
        if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
            a.sin6_addr.set_s6_addr32(2, 0);
            a.sin6_addr.set_s6_addr32(3, htonl(1));
        } else if let Some(ifid) = ifid {
            a.sin6_addr = *ifid;
            a.sin6_addr.set_s6_addr16(0, htons(0xfe80));
            a.sin6_addr.set_s6_addr16(1, htons(ifidx));
            a.sin6_addr.set_s6_addr32(1, 0);

            // RFC5072: Use negotiated P2P ifid as-is.
            if ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
                a.sin6_addr.s6_addr[8] &= !EUI64_GBIT;
                a.sin6_addr.s6_addr[8] |= EUI64_UBIT;
            }
        } else {
            in6_get_ifid(ifp, &mut a.sin6_addr);
        }
    }

    ifra.ifra_prefixmask = SockaddrIn6::with_addr(IN6MASK64);
    // link-local addresses should NEVER expire.
    ifra.ifra_lifetime.ia6t_vltime = ND6_INFINITE_LIFETIME;
    ifra.ifra_lifetime.ia6t_pltime = ND6_INFINITE_LIFETIME;

    // XXX: Some P2P interfaces seem not to send packets just after becoming up, so we skip
    // p2p interfaces for safety.
    if in6if_do_dad(ifp) && ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
        ifra.ifra_flags |= IN6_IFF_TENTATIVE;
    }

    // T1: the link-local address, cloned before its reader ends (`in6_update_ifa` may add an
    // address, which takes the writer), dropped with the function.
    let ll = net.with(|n| in6ifa_ifpforlinklocal(&n.reader(), ifp, 0).cloned());
    in6_update_ifa(ifp, &ifra, ll.as_ref(), net)?;

    let ll = net.with(|n| in6ifa_ifpforlinklocal(&n.reader(), ifp, 0).cloned());
    let Some(ifa) = ll else {
        kpanic(format_args!(
            "in6_ifattach_linklocal: no link-local address after in6_update_ifa"
        ));
    };

    // Perform DAD, if needed.
    if ifatoia6(&ifa).ia6_flags.get() & IN6_IFF_TENTATIVE != 0 {
        net.with(|n| nd6_dad_start(&ifa, n));
    }

    if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
        if_addrhooks_run(ifp);
        return Ok(()); // No need to install a connected route.
    }

    let mut flags = RTF_CONNECTED | RTF_MPATH;
    if ifp.if_flags.get() & IFF_POINTOPOINT == 0 {
        flags |= RTF_CLONING;
    }

    net.with(|n| connected_route(&ifa, ifp, flags, n))?;
    if_addrhooks_run(ifp);

    Ok(())
}

/// The connected route of link-local address `ia6` (`rt_ifa_add(&ia6->ia_ifa, flags,
/// ia6->ia_ifa.ifa_addr, ...)`), the address purged when it cannot be made.
fn connected_route(
    ifa: &IfaddrRef,
    ifp: &'static Ifnet,
    flags: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    // The address's own socket address (ia_addr), copied.
    let mut a = [0u8; size_of::<SockaddrIn6>()];
    let _ = hal::mem::pod::put(&mut a, 0, &ifatoia6(ifa).ia_addr.get());
    let Some(dst) = SockaddrView::new(&a) else {
        kpanic(format_args!("in6_ifattach: a sockaddr_in6 too short"));
    };
    let error = rt_ifa_add(ifa, flags, dst, ifp.if_rdomain.get(), net);
    if error.is_err() {
        in6_purgeaddr(ifa, net);
    }
    error
}

/// `in6_ifattach_loopback`: assigns `::1` to a loopback interface.
fn in6_ifattach_loopback(ifp: &'static Ifnet, net: &NetHolding<'_>) -> Result<(), Errno> {
    let in6 = IN6ADDR_LOOPBACK;
    let mut ifra = In6Aliasreq::default();

    kassert!(ifp.if_flags.get() & IFF_LOOPBACK != 0);

    if net.with(|n| in6ifa_ifpwithaddr(&n.reader(), ifp, &in6).is_some()) {
        return Ok(());
    }

    ifra.ifra_name = ifp.if_xname.get();
    ifra.ifra_prefixmask = SockaddrIn6::with_addr(IN6MASK128);

    // Always initialize ia_dstaddr (= broadcast address) to loopback address. Follows IPv4
    // practice - see in_ifinit().
    ifra.ifra_dstaddr = SockaddrIn6::with_addr(IN6ADDR_LOOPBACK);
    ifra.ifra_addr = SockaddrIn6::with_addr(IN6ADDR_LOOPBACK);

    // the loopback  address should NEVER expire.
    ifra.ifra_lifetime.ia6t_vltime = ND6_INFINITE_LIFETIME;
    ifra.ifra_lifetime.ia6t_pltime = ND6_INFINITE_LIFETIME;

    // We are sure that this is a newly assigned address, so we can set NULL to the 3rd arg.
    in6_update_ifa(ifp, &ifra, None, net)
}

/// `in6_ifattach(ifp)`: attaches IPv6 to `ifp`: the loopback address on the default loopback
/// interface of its routing domain and the link-local address. Under the net lock, held as
/// `net` says.
///
/// XXX multiple loopback interface needs more care. for instance, nodelocal address needs to
/// be configured onto only one of them. XXX multiple link-local address case.
pub fn in6_ifattach(ifp: &'static Ifnet, net: &NetHolding<'_>) -> Result<(), Errno> {
    // some of the interfaces are inherently not IPv6 capable
    match ifp.if_type.get() {
        IFT_BRIDGE | IFT_ENC | IFT_PFLOG | IFT_PFSYNC => return Ok(()),
        _ => {}
    }

    // if link mtu is too small, don't try to configure IPv6. remember there could be some
    // link-layer that has special fragmentation logic.
    if ifp.if_mtu.get() < IPV6_MMTU {
        return Err(Errno::EINVAL);
    }

    if nd6_need_cache(ifp) && ifp.if_flags.get() & IFF_MULTICAST == 0 {
        return Err(Errno::EINVAL);
    }

    // Assign loopback address if this lo(4) interface is the default for its rdomain.
    if ifp.if_flags.get() & IFF_LOOPBACK != 0
        && ifp.if_index.get() == rtable_loindex(ifp.if_rdomain.get())
    {
        in6_ifattach_loopback(ifp, net)?;
    }

    if ifp.if_type.get() == IFT_WIREGUARD {
        return Ok(());
    }

    // Assign a link-local address, if there's none.
    if net.with(|n| in6ifa_ifpforlinklocal(&n.reader(), ifp, 0).is_none())
        && in6_ifattach_linklocal(ifp, None, net).is_err()
    {
        // failed to assign linklocal address. bark?
    }

    Ok(())
}

/// Removes the route to the all-nodes group `allnodes` (with the interface index embedded)
/// if it belongs to `ifp`: the two identical blocks of `in6_ifdetach`.
fn in6_ifdetach_route(ifp: &'static Ifnet, allnodes: &In6Addr, net: &NetHeld<'_>) {
    let mut sin6 = SockaddrIn6::with_addr(*allnodes);
    sin6.sin6_addr
        .set_s6_addr16(1, htons(ifp.if_index.get() as u16));
    let mut b = [0u8; size_of::<SockaddrIn6>()];
    let _ = hal::mem::pod::put(&mut b, 0, &sin6);
    let Some(dst) = SockaddrView::new(&b) else {
        return;
    };
    let rt = rtalloc(dst, 0, ifp.if_rdomain.get(), net);
    if let Some(rt) = &rt
        && rt.rt_ifidx.get() == ifp.if_index.get()
    {
        let _ = rtdeletemsg(rt, ifp, ifp.if_rdomain.get(), net);
    }
    // rtfree(rt): the handle's drop.
}

/// `in6_ifdetach(ifp)`: removes every IPv6 address, route and membership of `ifp`. Under
/// the net lock (`net`).
///
/// NOTE: `in6_ifdetach()` does not support loopback if at this moment.
pub fn in6_ifdetach(ifp: &'static Ifnet, net: &NetHeld<'_>) {
    // MROUTING: ip6_mrouter_detach(ifp) removes the ip6_mrouter stuff; not configured.

    // nuke any of IPv6 addresses we have (TAILQ_FOREACH_SAFE: `in6_purgeaddr` takes the
    // address it is given off the list; T1/T2, `IfaList::for_each_safe`)
    ifp.if_addrlist().for_each_safe(net, |ifa| {
        if In6Ifaddr::of(ifa).is_none() {
            return;
        }
        in6_purgeaddr(ifa, net);
        if_addrhooks_run(ifp);
    });

    // Remove neighbor management table. Must be called after purging addresses.
    nd6_purge(ifp, net);

    // remove route to interface local allnodes multicast (ff01::1)
    in6_ifdetach_route(ifp, &IN6ADDR_INTFACELOCAL_ALLNODES, net);

    // remove route to link-local allnodes multicast (ff02::1)
    in6_ifdetach_route(ifp, &IN6ADDR_LINKLOCAL_ALLNODES, net);

    if ifp.if_xflags.get() & (IFXF_AUTOCONF6 | IFXF_AUTOCONF6TEMP) != 0 {
        ifp.if_xflags
            .set(ifp.if_xflags.get() & !(IFXF_AUTOCONF6 | IFXF_AUTOCONF6TEMP));
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the interface identifier code of `in6_ifattach.c` (LZ's, over the
    // kernel's interfaces): the EUI-64 of a hardware address, the sanity checks, the borrowed
    // and random identifiers.

    use hal::mem::malloc::{M_WAITOK, MBox};
    use hal::sync::netlock::NETLOCK;
    use hal::sys::net_if::{IFT_PPP, SockaddrDl};

    use super::*;
    use crate::net::if_::{M_IFADDR, test_ifnet};
    use crate::netinet6::in6::tests::a6;

    /// A link-level address: `name`, then `lladdr`.
    fn sdl(name: &[u8], lladdr: &[u8]) -> SockaddrDl {
        let mut sdl = SockaddrDl {
            sdl_nlen: name.len() as u8,
            sdl_alen: lladdr.len() as u8,
            ..SockaddrDl::default()
        };
        sdl.sdl_data[..name.len()].copy_from_slice(name);
        sdl.sdl_data[name.len()..name.len() + lladdr.len()].copy_from_slice(lladdr);
        sdl
    }

    /// `ifp`'s link name set to `sdl` (`if_alloc_sadl`'s record).
    fn set_sadl(ifp: &Ifnet, s: SockaddrDl) {
        let b = MBox::new(M_IFADDR, M_WAITOK, s).expect("a link name");
        drop(ifp.if_sadl.replace(Some(b)));
    }

    /// An interface of type `ty` with the link-level address `lladdr` (after a 4 byte name).
    fn if_with_lladdr(name: &[u8], ty: u8, lladdr: &[u8]) -> &'static Ifnet {
        let (ifp, _) = test_ifnet(name);
        ifp.if_type.set(ty);
        set_sadl(ifp, sdl(b"tsdl", lladdr));
        ifp
    }

    fn ifid_of(ifp: &Ifnet) -> Option<In6Addr> {
        let mut a = a6("fe80::");
        in6_get_hw_ifid(ifp, &mut a).then_some(a)
    }

    #[test]
    fn ethernet_address_gives_an_eui64_identifier() {
        let _g = crate::net::if_::tests::setup();
        // 52:54:00:bb:00:01 -> fe80::5054:ff:febb:1 (the "u" bit flipped, ff:fe in the
        // middle): smoke-inet6's address of its first VM.
        let ifp = if_with_lladdr(b"te0", IFT_ETHER, &[0x52, 0x54, 0x00, 0xbb, 0x00, 0x01]);
        let ifid = ifid_of(ifp).expect("an identifier");
        assert_eq!(
            &ifid.s6_addr[8..],
            [0x50, 0x54, 0x00, 0xff, 0xfe, 0xbb, 0x00, 0x01]
        );
        // The upper 64 bits are preserved.
        assert_eq!(&ifid.s6_addr[..8], &a6("fe80::").s6_addr[..8]);
        assert_eq!(ifid, a6("fe80::5054:ff:febb:1"));

        // A locally administered ("u" bit set) address becomes universal-looking and back.
        let ifp = if_with_lladdr(b"te1", IFT_ETHER, &[0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
        let ifid = ifid_of(ifp).expect("an identifier");
        assert_eq!(ifid.s6_addr[8], 0x00);
        assert_eq!(&ifid.s6_addr[11..13], [0xff, 0xfe]);
    }

    #[test]
    fn eight_byte_addresses_are_used_as_they_are() {
        let _g = crate::net::if_::tests::setup();
        let eui = [0x02, 0x11, 0x22, 0xff, 0xfe, 0x33, 0x44, 0x55];
        for ty in [IFT_IEEE1394, IFT_IEEE80211] {
            let ifp = if_with_lladdr(b"tf0", ty, &eui);
            let ifid = ifid_of(ifp).expect("an identifier");
            assert_eq!(ifid.s6_addr[8], 0x00);
            assert_eq!(&ifid.s6_addr[9..], &eui[1..]);
        }
        // IEEE1394 and 802.11 longer than 8 bytes: the first 8.
        let long = [
            0x02, 0x11, 0x22, 0xff, 0xfe, 0x33, 0x44, 0x55, 9, 9, 9, 9, 9, 9, 9, 9,
        ];
        let ifp = if_with_lladdr(b"tf1", IFT_IEEE1394, &long);
        let ifid = ifid_of(ifp).expect("an identifier");
        assert_eq!(&ifid.s6_addr[9..], &eui[1..]);
        // But an Ethernet-type 16 byte address is no EUI.
        let ifp = if_with_lladdr(b"tf2", IFT_ETHER, &long);
        assert!(ifid_of(ifp).is_none());
    }

    #[test]
    fn unusable_hardware_addresses_give_no_identifier() {
        let _g = crate::net::if_::tests::setup();
        // No link-level address.
        let (ifp, _) = test_ifnet(b"tn0");
        ifp.if_type.set(IFT_ETHER);
        assert!(ifid_of(ifp).is_none());
        let ifp = if_with_lladdr(b"tn1", IFT_ETHER, &[]);
        assert!(ifid_of(ifp).is_none());
        // All zero and all ones.
        assert!(ifid_of(if_with_lladdr(b"tn2", IFT_ETHER, &[0; 6])).is_none());
        assert!(ifid_of(if_with_lladdr(b"tn3", IFT_ETHER, &[0xff; 6])).is_none());
        assert!(ifid_of(if_with_lladdr(b"tn4", IFT_IEEE80211, &[0; 8])).is_none());
        assert!(ifid_of(if_with_lladdr(b"tn5", IFT_IEEE1394, &[0xff; 8])).is_none());
        // The group bit set: a multicast address.
        assert!(ifid_of(if_with_lladdr(b"tn6", IFT_ETHER, &[0x01, 0, 0x5e, 0, 0, 1])).is_none());
        // An identifier that would be all zero (subnet router anycast): u bit only.
        let zero_ifid = [0x02, 0, 0, 0, 0, 0, 0, 0];
        assert!(ifid_of(if_with_lladdr(b"tn7", IFT_IEEE1394, &zero_ifid)).is_none());
        // The wrong length for an Ethernet.
        assert!(ifid_of(if_with_lladdr(b"tn8", IFT_ETHER, &[1, 2, 3, 4, 5])).is_none());
        // Interface types that have none: gif (RFC 2893 says to borrow, the C does not) and PPP.
        let mac = [0x52, 0x54, 0, 0xbb, 0, 2];
        assert!(ifid_of(if_with_lladdr(b"tn9", IFT_GIF, &mac)).is_none());
        assert!(ifid_of(if_with_lladdr(b"tna", IFT_PPP, &mac)).is_none());
        // CARP uses the Ethernet rules.
        let carp = [0x00, 0x00, 0x5e, 0x00, 0x01, 0x01];
        assert!(ifid_of(if_with_lladdr(b"tnb", IFT_CARP, &carp)).is_some());
    }

    #[test]
    fn random_identifier_is_local_and_individual() {
        let _g = crate::net::if_::tests::setup();
        let (ifp, _) = test_ifnet(b"tr0");
        for _ in 0..32 {
            let mut a = a6("fe80:1::");
            in6_get_rand_ifid(ifp, &mut a);
            assert_eq!(&a.s6_addr[..8], &a6("fe80:1::").s6_addr[..8]);
            // "u" local and "g" individual in the EUI-64, then flipped by EUI64_TO_IFID:
            // both bits clear in the interface identifier.
            assert_eq!(a.s6_addr[8] & (EUI64_GBIT | EUI64_UBIT), 0);
        }
    }

    #[test]
    fn identifier_is_borrowed_from_another_interface_or_random() {
        let _g = crate::net::if_::tests::setup();
        // No hardware address on the first interface: it borrows the second's.
        let (a, _) = test_ifnet(b"tb0");
        a.if_type.set(IFT_PPP);
        let (b, _) = test_ifnet(b"tb1");
        b.if_type.set(IFT_ETHER);
        set_sadl(b, sdl(b"", &[0x52, 0x54, 0x00, 0xbb, 0x00, 0x02]));

        let w = NETLOCK.write();
        IFNETLIST.insert_tail(a);
        IFNETLIST.insert_tail(b);
        let mut id = a6("fe80::");
        in6_get_ifid(a, &mut id);
        assert_eq!(
            &id.s6_addr[8..],
            [0x50, 0x54, 0x00, 0xff, 0xfe, 0xbb, 0x00, 0x02]
        );
        // Its own hardware address first.
        let mut id = a6("fe80::");
        in6_get_ifid(b, &mut id);
        assert_eq!(
            &id.s6_addr[8..],
            [0x50, 0x54, 0x00, 0xff, 0xfe, 0xbb, 0x00, 0x02]
        );
        // With nobody to borrow from: random.
        b.if_type.set(IFT_PPP);
        let mut id = a6("fe80::");
        in6_get_ifid(a, &mut id);
        assert_eq!(id.s6_addr[8] & (EUI64_GBIT | EUI64_UBIT), 0);
        drop(w);
    }

    #[test]
    fn nd6_need_cache_is_the_cs() {
        let _g = crate::net::if_::tests::setup();
        let (ifp, _) = test_ifnet(b"tnc0");
        for (ty, want) in [
            (IFT_ETHER, true),
            (IFT_IEEE80211, true),
            (IFT_CARP, true),
            (IFT_PPP, false),
            (IFT_GIF, false),
            (IFT_WIREGUARD, false),
        ] {
            ifp.if_type.set(ty);
            assert_eq!(nd6_need_cache(ifp), want, "type {ty:#x}");
        }
    }
}
/* </TESTS> */
