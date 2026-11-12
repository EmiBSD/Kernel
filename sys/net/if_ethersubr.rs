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
 * Copyright (c) 1982, 1989, 1993
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
 *	@(#)if_ethersubr.c	8.1 (Berkeley) 6/10/93
 */

/*
%%% portions-copyright-nrl-95
Portions of this software are Copyright 1995-1998 by Randall Atkinson,
Ronald Lee, Daniel McDonald, Bao Phan, and Chris Winters. All Rights
Reserved. All rights under this copyright have been assigned to the US
Naval Research Laboratory (NRL). The NRL Copyright Notice and License
Agreement Version 1.1 (January 17, 1995) applies to these portions of the
software.
You should have received a copy of the license with this software. If you
didn't get a copy, you may request one from <license@ipv6.nrl.navy.mil>.
*/
/* </LICENSES> */

/* <CODE> */
//! Ethernet for `bsd`: the link layer is the kernel's since N6.3c2
//! (`kernel::net::if_ethersubr`), re-exported here under LZ's path for the drivers, ARP, ND
//! and the tests, with three adapters of `bsd`'s shapes: the drivers' raw `ether_ioctl`, the
//! parked-mbuf `ether_input` the tests call, and `ether_extract_headers` into `bsd`'s
//! pointer view of `struct ether_extracted` (`netinet/if_ether.rs`), which tcp_input's
//! software LRO, vio and vmx dereference.
//!
//! Upstream: sys/net/if_ethersubr.c @ 3ce1f3f79392

use core::ptr;

use crate::net::if_var::Netstack;
use crate::netinet::if_ether::EtherExtracted;
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, unpark};

pub use kernel::net::if_ethersubr::{
    Arpcom, ETHER, ETHER_IOCTL_MULTI_LEN, ETHER_IP6MULTICAST_MAX, ETHER_IP6MULTICAST_MIN,
    ETHER_IPMULTICAST_MAX, ETHER_IPMULTICAST_MIN, ETHERANYADDR, ETHERBROADCASTADDR, Ether,
    EtherAddrCell, EtherMulti, EtherMultiIter, EtherMultis, EtherPort, EtherPortSlot, arpcom_of,
    ether_addmulti, ether_addr_to_e64, ether_brport_clr, ether_brport_get, ether_brport_get_locked,
    ether_brport_isset, ether_brport_set, ether_crc32_be, ether_crc32_be_update, ether_crc32_le,
    ether_crc32_le_update, ether_delmulti, ether_e64_to_addr, ether_encap, ether_fakeaddr,
    ether_ifattach, ether_ifdetach, ether_ioctl_len, ether_map_ip_multicast,
    ether_map_ipv6_multicast, ether_multiaddr, ether_offload_ifcap, ether_output, ether_resolve,
    ether_rtrequest, ether_sprintf,
};

/// `ether_ioctl(ifp, arp, cmd, data)` of a driver's `if_ioctl`: the kernel's over the bytes
/// of the request it reads ([`ether_ioctl_len`]).
///
/// # Safety
///
/// As for `if_ioctl` (`IfIoctlFn`): `data` points at the request's structure for `cmd`,
/// readable for `IOCPARM_LEN(cmd)` bytes and through the largest union member a handler
/// reads there (the multicast requests' `sockaddr_in6`, [`ETHER_IOCTL_MULTI_LEN`] bytes from
/// the start), and nothing writes those bytes during the call.
pub unsafe fn ether_ioctl(
    ifp: &crate::net::if_var::Ifnet,
    arp: &Arpcom,
    cmd: u64,
    data: *mut u8,
) -> Result<(), Errno> {
    // SAFETY: `data` is non-null and readable for the request's structure and the largest
    // union member a handler reads (the caller's contract), which covers the
    // `ether_ioctl_len(cmd)` bytes (`size_of::<Ifreq>()` = `IOCPARM_LEN` for `SIOCSIFMTU`,
    // `ETHER_IOCTL_MULTI_LEN` for the multicast requests, none otherwise); they are only read,
    // and nothing writes them while the slice lives (the call's duration), so a shared slice
    // of them is sound; `u8` has alignment 1.
    let req = unsafe { core::slice::from_raw_parts(data.cast_const(), ether_ioctl_len(cmd)) };
    kernel::net::if_ethersubr::ether_ioctl(ifp, arp, cmd, req)
}

/// `ether_input(ifp, m, ns)` of a parked mbuf (the tests' frames): the kernel's.
pub fn ether_input(
    ifp: &'static crate::net::if_var::Ifnet,
    m: &'static Mbuf,
    ns: Option<&mut Netstack>,
) {
    kernel::net::if_ethersubr::ether_input(ifp, unpark(m), ns);
}

/// `ether_extract_headers(m0, ext)` into `bsd`'s view: the kernel's offsets, each turned into
/// the address of its header inside the mbuf that holds it (`mtod(m, caddr_t) + hoff`, the
/// C's pointer; NULL for a header not found). The addresses are made, never followed here.
pub fn ether_extract_headers(m0: &Mbuf, ext: &mut EtherExtracted) {
    let mut k = kernel::net::if_ethersubr::EtherExtracted::new();
    kernel::net::if_ethersubr::ether_extract_headers(m0, &mut k);
    *ext = EtherExtracted {
        eh: header_at(m0, k.eh).cast(),
        evh: header_at(m0, k.evh).cast(),
        ip4: header_at(m0, k.ip4).cast(),
        ip6: header_at(m0, k.ip6).cast(),
        tcp: header_at(m0, k.tcp),
        udp: header_at(m0, k.udp),
        iplen: k.iplen,
        iphlen: k.iphlen,
        tcphlen: k.tcphlen,
        paylen: k.paylen,
    };
}

/// The address of the packet's byte `off` (`m_getptr`'s mbuf and offset in it), NULL for
/// none.
fn header_at(m0: &Mbuf, off: Option<u32>) -> *mut u8 {
    off.and_then(|o| kernel::net::mbuf::m_getptr(m0, o as i32))
        .map_or(ptr::null_mut(), |(m, hoff)| {
            m.data_raw().wrapping_add(hoff as usize)
        })
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the adapters over `bsd`'s protocols: the parked `ether_input` into bsd's
    // inputs (every frame freed), `ether_extract_headers`' pointers, the raw `ether_ioctl`
    // over an `in6_ifreq`, and `ether_resolve` through bsd's neighbor cache. The Ethernet
    // layer's own tests are the kernel's (`kernel::net::if_ethersubr`).

    use std::vec::Vec;

    use super::*;
    use crate::kern::uipc_mbuf::{MBUFPL, m_freem};
    use crate::net::ethertypes::{ETHERTYPE_ARP, ETHERTYPE_IP, ETHERTYPE_IPV6, ETHERTYPE_VLAN};
    use crate::net::if_::tests::{setup_net, test_ioctl, test_packet, zeroed_static};
    use crate::net::if_::{IFF_BROADCAST, IFF_MULTICAST, IFF_NOARP, IFF_RUNNING, IFF_SIMPLEX};
    use crate::net::if_var::IfnetLegacy;
    use crate::sys::mbuf::mtod;

    const OURS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    const PEER: [u8; 6] = [0x52, 0x55, 0x0a, 0x00, 0x02, 0x02];

    /// An Ethernet interface `teth0` with our address, attached to the Ethernet layer.
    fn test_arpcom() -> &'static Arpcom {
        // SAFETY: the all-zero `Arpcom` is valid (kernel::net::if_ethersubr's redesign).
        let ac: &'static Arpcom = unsafe { zeroed_static() };
        let mut xname = [0u8; crate::net::if_::IFNAMSIZ];
        xname[..5].copy_from_slice(b"teth0");
        ac.ac_if.if_xname.set(xname);
        ac.ac_if.if_ioctl().set(Some(test_ioctl));
        ac.ac_if
            .if_flags
            .set(IFF_BROADCAST | IFF_SIMPLEX | IFF_MULTICAST | IFF_RUNNING);
        ac.ac_enaddr.set(OURS);
        // The C holds the kernel lock across an attach (bpf_ifattach asserts it).
        hal::klock::kernel_lock();
        ether_ifattach(ac);
        hal::klock::kernel_unlock();
        ac
    }

    /// A frame from `src` to `dst` of type `etype` with an IPv4 header as payload.
    fn frame(dst: [u8; 6], src: [u8; 6], etype: u16) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&dst);
        f.extend_from_slice(&src);
        f.extend_from_slice(&etype.to_be_bytes());
        f.extend_from_slice(&[
            0x45, 0, 0, 20, 0, 0, 0, 0, 64, 1, 0, 0, 10, 0, 2, 2, 10, 0, 2, 15,
        ]);
        f
    }

    #[test]
    fn ether_input_filters_and_frees_every_frame() {
        let _g = setup_net();
        let ac = test_arpcom();
        let ifp = &ac.ac_if;
        let before = MBUFPL.nout();
        let input = |bytes: &[u8]| ether_input(ifp, test_packet(bytes), None);

        // Too short for an Ethernet header.
        input(&[0u8; 10]);
        // To us: bsd's ipv4_input.
        input(&frame(OURS, PEER, ETHERTYPE_IP));
        assert_eq!(ifp.if_imcasts().get(), 0);
        // Unicast for someone else.
        input(&frame([0x52, 0x54, 0, 0, 0, 1], PEER, ETHERTYPE_IP));
        assert_eq!(ifp.if_imcasts().get(), 0);
        // Broadcast ARP from a peer counts as a multicast reception.
        input(&frame(ETHERBROADCASTADDR, PEER, ETHERTYPE_ARP));
        assert_eq!(ifp.if_imcasts().get(), 1);
        // A VLAN-tagged frame without vlan(4) is service delimited: dropped.
        input(&frame(OURS, PEER, ETHERTYPE_VLAN));
        // An IPv6 multicast is counted, then dropped by ipv6_input (an IPv4 header follows).
        input(&frame([0x33, 0x33, 0, 0, 0, 1], PEER, ETHERTYPE_IPV6));
        assert_eq!(ifp.if_imcasts().get(), 2);
        // Our own multicast on a non-simplex interface: dropped before it is counted.
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_SIMPLEX);
        input(&frame(ETHERBROADCASTADDR, OURS, ETHERTYPE_ARP));
        assert_eq!(ifp.if_imcasts().get(), 2);
        // ARP on an IFF_NOARP interface.
        ifp.if_flags.set(ifp.if_flags.get() | IFF_NOARP);
        input(&frame(ETHERBROADCASTADDR, PEER, ETHERTYPE_ARP));
        assert_eq!(ifp.if_imcasts().get(), 3);

        assert_eq!(MBUFPL.nout(), before, "every frame was freed");
    }

    #[test]
    fn extract_headers_points_at_an_ipv4_udp_frame_s_headers() {
        let _g = setup_net();
        let mut f = Vec::new();
        f.extend_from_slice(&OURS);
        f.extend_from_slice(&PEER);
        f.extend_from_slice(&ETHERTYPE_IP.to_be_bytes());
        // IPv4, 20-byte header, total length 28, UDP.
        f.extend_from_slice(&[
            0x45, 0, 0, 28, 0, 0, 0, 0, 64, 17, 0, 0, 10, 0, 2, 2, 10, 0, 2, 15,
        ]);
        f.extend_from_slice(&[0x12, 0x34, 0x00, 0x35, 0x00, 0x08, 0x00, 0x00]);
        let m = test_packet(&f);

        let mut ext = EtherExtracted::new();
        ether_extract_headers(m, &mut ext);
        let base = mtod::<u8>(m);
        assert_eq!(ext.eh.cast::<u8>(), base);
        assert_eq!(ext.ip4.cast::<u8>(), base.wrapping_add(14));
        assert_eq!(ext.udp, base.wrapping_add(34));
        assert!(ext.tcp.is_null() && ext.ip6.is_null() && ext.evh.is_null());
        assert_eq!((ext.iplen, ext.iphlen, ext.paylen), (28, 20, 8));

        // A fragment: the transport header is not looked at.
        // SAFETY: the byte at ip_off in the mbuf's data.
        unsafe { mtod::<u8>(m).add(14 + 6).write(0x20) };
        ether_extract_headers(m, &mut ext);
        assert!(!ext.ip4.is_null() && ext.udp.is_null());
        m_freem(m);
    }

    /// `ifp`'s `SIOCADDMULTI`/`SIOCDELMULTI` request for `addr`, as `in6_addmulti` builds it: a
    /// `struct in6_ifreq`, which `ether_ioctl` reads through its 44 first bytes.
    #[cfg(feature = "inet6")]
    fn in6_ifreq_for(addr: crate::netinet6::in6::In6Addr) -> *mut u8 {
        use crate::netinet6::in6::SockaddrIn6;
        use crate::netinet6::in6_var::In6Ifreq;
        let ifr6: &'static mut In6Ifreq =
            std::boxed::Box::leak(std::boxed::Box::new(In6Ifreq::zeroed()));
        ifr6.set_ifr_addr(SockaddrIn6::with_addr(addr));
        ptr::from_mut(ifr6).cast()
    }

    #[test]
    #[cfg(feature = "inet6")]
    fn ipv6_multicast_requests_reach_the_list_through_the_raw_ioctl() {
        use crate::netinet6::in6::{IN6ADDR_ANY, In6Addr};
        use crate::sys::sockio::{SIOCADDMULTI, SIOCDELMULTI};
        let _g = setup_net();
        let ac = test_arpcom();
        let ifp = &ac.ac_if;
        // ff02::1:ff00:2, the solicited-node group of ...::2.
        let group = In6Addr::new([0xff, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0xff, 0, 0, 2]);
        let ifr = in6_ifreq_for(group);
        // SAFETY: a whole `in6_ifreq`, leaked: readable past ETHER_IOCTL_MULTI_LEN.
        assert_eq!(
            unsafe { ether_ioctl(ifp, ac, SIOCADDMULTI, ifr) },
            Err(Errno::ENETRESET)
        );
        {
            let g = ac.multiaddrs();
            let enm = g.iter().next().expect("added");
            assert_eq!(enm.enm_addrlo, [0x33, 0x33, 0xff, 0x00, 0x00, 0x02]);
            assert_eq!(enm.enm_addrhi, enm.enm_addrlo);
        }
        assert_eq!(ac.ac_multirangecnt.get(), 0);

        // The unspecified address claims the whole IPv6 multicast range.
        let any = in6_ifreq_for(IN6ADDR_ANY);
        // SAFETY: as above.
        assert_eq!(
            unsafe { ether_ioctl(ifp, ac, SIOCADDMULTI, any) },
            Err(Errno::ENETRESET)
        );
        assert_eq!(ac.ac_multirangecnt.get(), 1);
        assert!(
            ac.multiaddrs().iter().any(|e| (e.enm_addrlo, e.enm_addrhi)
                == (ETHER_IP6MULTICAST_MIN, ETHER_IP6MULTICAST_MAX))
        );

        // SAFETY: as above.
        unsafe {
            assert_eq!(
                ether_ioctl(ifp, ac, SIOCDELMULTI, ifr),
                Err(Errno::ENETRESET)
            );
            assert_eq!(
                ether_ioctl(ifp, ac, SIOCDELMULTI, any),
                Err(Errno::ENETRESET)
            );
        }
        assert_eq!(ac.ac_multicnt.get(), 0);
    }

    #[test]
    #[cfg(feature = "inet6")]
    fn resolve_an_ipv6_destination_through_the_neighbor_cache() {
        use crate::net::route::RTF_LLINFO;
        use crate::netinet::ip_input::tests::PEER as PEER_MAC;
        use crate::netinet6::in6::{In6Addr, SockaddrIn6};
        use crate::netinet6::nd6::tests::{PEER6, fake_neighbor, nd6_setup};
        use crate::netinet6::nd6_nbr::nd6_na_cache;
        use crate::sys::mbuf::M_MCAST;
        use crate::sys::systm::{net_lock, net_unlock};
        use kernel::net::proto::SockaddrView;

        let (_g, ifp) = nd6_setup();
        let mut eh = hal::sys::netinet_if_ether::EtherHeader::default();
        let view =
            |s: &SockaddrIn6| -> Vec<u8> { hal::user::bytes_of(core::slice::from_ref(s)).to_vec() };

        // A multicast destination maps straight to 33:33:<low 32 bits>, no route needed.
        let all_nodes = view(&SockaddrIn6::with_addr(In6Addr::new([
            0xff, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ])));
        let m = test_packet(&[0; 40]);
        m.m_flags().set(m.m_flags().get() | M_MCAST);
        let dst = SockaddrView::new(&all_nodes).expect("an address");
        let m = ether_resolve(ifp, unpark(m), dst, None, &mut eh).expect("resolved");
        assert_eq!(eh.ether_dhost, [0x33, 0x33, 0, 0, 0, 1]);
        assert_eq!(u16::from_be(eh.ether_type), ETHERTYPE_IPV6);
        drop(m);

        // A unicast neighbor: a host route given its cache entry by nd6_rtrequest (through
        // ether_rtrequest), as nd6's tests build one.
        let rt = fake_neighbor(ifp, PEER6);
        assert_ne!(rt.rt_flags.get() & RTF_LLINFO, 0, "nd6_rtrequest ran");

        // Unresolved: the packet is held (EAGAIN) while the neighbor is solicited.
        let peer = view(&SockaddrIn6::with_addr(PEER6));
        let dst = SockaddrView::new(&peer).expect("an address");
        net_lock();
        let r = ether_resolve(ifp, unpark(test_packet(&[0; 40])), dst, Some(rt), &mut eh);
        net_unlock();
        assert!(matches!(r, Err(Errno::EAGAIN)));

        // Its advertisement resolves it: the next packet gets the neighbor's address.
        net_lock();
        nd6_na_cache(
            ifp,
            rt,
            Some(&PEER_MAC),
            false,
            true,
            true,
            false,
            &PEER6,
            &PEER6,
        );
        net_unlock();
        assert_eq!(crate::netinet::if_ether::gw_alen_family(rt).0, 6);
        net_lock();
        let r = ether_resolve(ifp, unpark(test_packet(&[0; 40])), dst, Some(rt), &mut eh);
        net_unlock();
        assert!(r.is_ok());
        assert_eq!(eh.ether_dhost, PEER_MAC);
        assert_eq!(u16::from_be(eh.ether_type), ETHERTYPE_IPV6);
    }

    #[test]
    #[cfg(feature = "inet6")]
    fn extract_headers_points_at_an_ipv6_tcp_frame_s_headers() {
        let _g = setup_net();
        let mut f = Vec::new();
        f.extend_from_slice(&OURS);
        f.extend_from_slice(&PEER);
        f.extend_from_slice(&ETHERTYPE_IPV6.to_be_bytes());
        // IPv6, payload length 24, TCP, hop limit 64, fe80::1 -> fe80::2.
        f.extend_from_slice(&[0x60, 0, 0, 0, 0, 24, 6, 64]);
        f.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        f.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        // TCP, data offset 5 (20 bytes), then 4 bytes of payload.
        f.extend_from_slice(&[
            0, 22, 0, 80, 0, 0, 0, 1, 0, 0, 0, 0, 0x50, 0x18, 0x40, 0, 0, 0, 0, 0,
        ]);
        f.extend_from_slice(&[1, 2, 3, 4]);
        let m = test_packet(&f);

        let mut ext = EtherExtracted::new();
        ether_extract_headers(m, &mut ext);
        let base = mtod::<u8>(m);
        assert_eq!(ext.ip6.cast::<u8>(), base.wrapping_add(14));
        assert_eq!(ext.tcp, base.wrapping_add(54));
        assert!(ext.ip4.is_null() && ext.udp.is_null());
        assert_eq!(
            (ext.iplen, ext.iphlen, ext.tcphlen, ext.paylen),
            (64, 40, 20, 4)
        );
        m_freem(m);
    }
}
/* </TESTS> */
