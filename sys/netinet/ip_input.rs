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
//! IPv4 input (`netinet/ip_input.c`), the kernel's since N6.4c2 (`kernel::netinet::ip_input`):
//! the kernel's items under LZ's path for `bsd`'s importers, and LZ's signatures over parked
//! mbufs (`&'static Mbuf`) for `bsd`'s callers (the transports, pf, wg, divert, IP-in-IP,
//! IPv6's `ip6_input` and the protocol table's entries), each unparking the packet, taking the
//! net lock's proof through the `SaToken` (`bsd` holds the lock C-style) and calling the
//! kernel's.
//!
//! Upstream: sys/netinet/ip_input.c @ 3ce1f3f79392

use hal::netlock::NetLend;

use crate::kern::proc_glue::BsdProcGlue as G;
use crate::net::if_::net_held;
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::route::Route;
use crate::netinet::in_pcb::Inpcb;
use crate::netinet::ip::Ip;
use crate::sys::mbuf::{Mbuf, park, unpark};

pub use kernel::netinet::ip_input::{
    INETCTLERRMAP, IP_DIRECTEDBCAST, IP_DOSOURCEROUTE, IP_MAXQUEUE, IP_MTUDISC_TIMEOUT,
    IP_SENDREDIRECTS, IPINTRQ, IPMFORWARDING, IpSrcrt, ip_forwarding, ip_slowtimo,
};
pub use kernel::netinet::ip_var::{IP_DEFTTL, IPMULTIPATH, ip_mtudisc};

/// The proof that `bsd`'s caller holds the net lock, lent for one call into the kernel.
fn held(what: &str) -> NetLend {
    net_held(what)
}

/// `ip_init` over `bsd`'s table, for `bsd`'s protocol tests (the boot's runs from
/// `domaininit`, through the kernel's IP entry).
#[cfg(test)]
pub(crate) fn ip_init() {
    kernel::netinet::in_proto::inetsw_set::<G>(&crate::kern::uipc_domain::INET_PROTOS);
    kernel::netinet::ip_input::ip_init::<G>();
}

/// `ipintr()`: the kernel's, under the exclusive net lock a test holds (`net_lock()`;
/// `if_netisr` calls the kernel's).
#[cfg(test)]
pub(crate) fn ipintr() {
    kernel::netinet::ip_input::ipintr::<G>(held("ipintr").net());
}

/// `ipv4_input(ifp, m, ns)` of a parked packet (wg(4), divert(4)).
pub fn ipv4_input(ifp: &'static Ifnet, m: &'static Mbuf, ns: Option<&mut Netstack>) {
    kernel::netinet::ip_input::ipv4_input::<G>(ifp, unpark(m), ns, held("ipv4_input").net());
}

/// `ip_input_if(mp, offp, nxt, af, ifp, ns)` of a parked packet (IP-in-IP).
pub fn ip_input_if(
    mp: &mut Option<&'static Mbuf>,
    offp: &mut i32,
    nxt: i32,
    af: i32,
    ifp: &'static Ifnet,
    ns: Option<&mut Netstack>,
) -> i32 {
    let mut m = mp.take().map(unpark);
    let nxt = kernel::netinet::ip_input::ip_input_if::<G>(
        &mut m,
        offp,
        nxt,
        af,
        ifp,
        ns,
        held("ip_input_if").net(),
    );
    *mp = m.map(park);
    nxt
}

/// `ip_deliver(mp, offp, nxt, af, shared, ns)` of a parked packet (IPv6's `ip6_input`).
pub fn ip_deliver(
    mp: &mut Option<&'static Mbuf>,
    offp: &mut i32,
    nxt: i32,
    af: i32,
    shared: bool,
    ns: Option<&mut Netstack>,
) -> i32 {
    let mut m = mp.take().map(unpark);
    let nxt = kernel::netinet::ip_input::ip_deliver::<G>(
        &mut m,
        offp,
        nxt,
        af,
        shared,
        ns,
        held("ip_deliver").net(),
    );
    *mp = m.map(park);
    nxt
}

/// `ip_srcroute(m0)`: the kernel's, the options mbuf parked.
pub fn ip_srcroute(m0: &'static Mbuf) -> Option<&'static Mbuf> {
    let mut m = unpark(m0);
    let r = kernel::netinet::ip_input::ip_srcroute(&mut m);
    let _ = park(m);
    r.map(park)
}

/// `ip_forward(m, ifp, ro, flags)` of a parked packet (pf's route-to).
pub fn ip_forward(m: &'static Mbuf, ifp: &Ifnet, ro: Option<&mut Route>, flags: i32) {
    kernel::netinet::ip_input::ip_forward(unpark(m), ifp, ro, flags, held("ip_forward").net());
}

/// `ip_send(m)` of a parked packet.
pub fn ip_send(m: &'static Mbuf) {
    kernel::netinet::ip_input::ip_send(unpark(m));
}

/// `ip_savecontrol(inp, mp, ip, m)`: the kernel's control messages appended at `*mp` (an
/// empty slot in every caller), parked.
pub fn ip_savecontrol(inp: &Inpcb, mp: &mut Option<&'static Mbuf>, ip: &Ip, m: &Mbuf) {
    let chain = kernel::netinet::ip_input::ip_savecontrol::<G>(inp, ip, m);
    *mp = chain.map(park);
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use crate::net::if_var::IfnetLegacy;
    // Host tests for IPv4: header validation in `ipv4_input` on crafted packets, and the whole
    // path of a ping over a test Ethernet interface: an address through `in_ioctl`, the
    // routes it makes, a default route, ARP resolution with the held packet sent once the reply
    // arrives, and an echo reply counted by `icmp_input`.

    use core::mem::size_of;
    use core::ptr;
    use core::sync::atomic::Ordering;
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use super::*;
    use crate::kern::uipc_mbuf::m_copydata;
    use crate::kern::uipc_mbuf::{m_freem, m_gethdr};
    use crate::net::ethertypes::{ETHERTYPE_ARP, ETHERTYPE_IP};
    use crate::net::if_::tests::{setup_net, test_packet, zeroed_static};
    use crate::net::if_::{
        IFF_BROADCAST, IFF_MULTICAST, IFF_RUNNING, IFF_SIMPLEX, IFF_UP, IFNAMSIZ, if_attach,
    };
    use crate::net::if_arp::{ARPOP_REPLY, ARPOP_REQUEST};
    use crate::net::if_ethersubr::{ether_ifattach, ether_input, ether_ioctl};
    use crate::net::if_var::Ifnet;
    use crate::net::ifq::ifq_dequeue;
    use crate::net::route::{
        RTAX_DST, RTAX_GATEWAY, RTAX_NETMASK, RTF_BROADCAST, RTF_CLONED, RTF_CLONING,
        RTF_CONNECTED, RTF_GATEWAY, RTF_LLINFO, RTF_LOCAL, RTF_STATIC, RTM_ADD, RtAddrinfo,
        route_init, rtalloc, rtfree, rtrequest, sa_of, sa_view,
    };
    use crate::net::rtable::rtable_init;
    use crate::netinet::icmp_var::IcmpstatCounters;
    use crate::netinet::if_ether::{Arpcom, EtherArp, EtherHeader, arpcom_of, arpintr};
    use crate::netinet::in_::{IPPROTO_ICMP, in_ioctl};
    use crate::netinet::in_::{InAddr, SockaddrIn, sintosa};
    use crate::netinet::in_cksum::in_cksum;
    use crate::netinet::in_var::InAliasreq;
    use crate::netinet::in4_cksum::in4_cksum;
    use crate::netinet::ip_icmp::{ICMP_ECHO, ICMP_ECHOREPLY, ICMPCOUNTERS, IcmpPkt, icmp_init};
    use crate::netinet::ip_output::ip_output;
    use crate::netinet::ip_var::{IPCOUNTERS, IpstatCounters, mtod_ip, mtod_ip_store};
    use crate::sys::endian::htons;
    use crate::sys::errno::Errno;
    use crate::sys::mbuf::{M_DONTWAIT, m_data_add};
    use crate::sys::mbuf::{M_ICMP_CSUM_OUT, MHLEN};
    use crate::sys::socket::AF_INET;
    use crate::sys::sockio::{SIOCAIFADDR, SIOCDIFADDR, SIOCSIFADDR};
    use crate::sys::systm::{net_lock, net_unlock};

    pub(crate) const OURS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    pub(crate) const PEER: [u8; 6] = [0x52, 0x55, 0x0a, 0x00, 0x02, 0x02];
    pub(crate) const ADDR: [u8; 4] = [10, 0, 2, 15];
    pub(crate) const GATEWAY: [u8; 4] = [10, 0, 2, 2];

    /// `f` under the net lock, as the C runs the stack (`if_input_process`, `if_netisr`, the protocols' output): the route lookups want the lock's proof.
    fn locked<R>(f: impl FnOnce() -> R) -> R {
        crate::sys::systm::net_lock();
        let r = f();
        crate::sys::systm::net_unlock();
        r
    }

    /// The network test setup plus what `rtable_init` and `domaininit` do for the inet and route
    /// domains; with the timeout wheel reset (`arpinit` arms a timeout), under its lock too.
    pub(crate) fn setup() -> (MutexGuard<'static, ()>, MutexGuard<'static, ()>) {
        let g = setup_net();
        let t = crate::kern::kern_timeout::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::kern::kern_timeout::timeout_startup();
        // ARP reads an expiry of 0 as "permanent": run as a system past its first second.
        crate::kern::kern_tc::TIME_UPTIME.store(100, Ordering::Relaxed);
        // The interface queues name their softnet task queue (no thread runs it here).
        crate::net::if_::softnet_init();
        // The kernel's tables and labels of an earlier test live in the memory just reset.
        kernel::net::rtable::rtable_test_reset();
        kernel::net::route::rtlabel_test_reset();
        rtable_init();
        route_init();
        kernel::netinet::ip_input::ipq_test_reset();
        kernel::netinet::ip_icmp::icmp_test_reset();
        ip_init();
        icmp_init();
        (g, t)
    }

    /// A `sockaddr_in` for `a`.
    pub(crate) fn sin(a: [u8; 4]) -> SockaddrIn {
        SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes(a),
            },
            ..SockaddrIn::default()
        }
    }

    /// A driver `ioctl` that takes addresses (as drivers do with `ether_ioctl`).
    ///
    /// # Safety
    ///
    /// `IfIoctlFn`'s contract.
    unsafe fn test_ether_ioctl(ifp: &'static Ifnet, cmd: u64, data: *mut u8) -> Result<(), Errno> {
        if cmd == SIOCSIFADDR {
            ifp.if_flags.set(ifp.if_flags.get() | IFF_UP | IFF_RUNNING);
            return Ok(());
        }
        // SAFETY: the caller's contract.
        unsafe { ether_ioctl(ifp, arpcom_of(ifp), cmd, data) }
    }

    /// An attached Ethernet interface `tvio0` with our hardware address.
    pub(crate) fn test_ether() -> &'static Ifnet {
        // SAFETY: the all-zero `Arpcom` is valid (`netinet/if_ether.rs`).
        let ac: &'static Arpcom = unsafe { zeroed_static() };
        let mut xname = [0u8; IFNAMSIZ];
        xname[..5].copy_from_slice(b"tvio0");
        ac.ac_if.if_xname.set(xname);
        ac.ac_if.if_ioctl().set(Some(test_ether_ioctl));
        ac.ac_if
            .if_flags
            .set(IFF_BROADCAST | IFF_SIMPLEX | IFF_MULTICAST);
        ac.ac_enaddr.set(OURS);
        if_attach(&ac.ac_if);
        // The C holds the kernel lock across an attach (bpf_ifattach asserts it).
        hal::klock::kernel_lock();
        ether_ifattach(ac);
        hal::klock::kernel_unlock();
        &ac.ac_if
    }

    /// `ifconfig <ifp> inet <addr> netmask <mask>`, from the kernel.
    pub(crate) fn configure(ifp: &'static Ifnet, addr: [u8; 4], mask: [u8; 4]) {
        let mut ifra = InAliasreq::default();
        ifra.ifra_name = ifp.if_xname.get();
        ifra.ifra_addr = sin(addr);
        ifra.ifra_mask = sin(mask);
        // SAFETY: an `in_aliasreq`, from the kernel itself.
        unsafe {
            in_ioctl(
                SIOCAIFADDR,
                ptr::from_mut(&mut ifra).cast(),
                Some(ifp),
                true,
            )
        }
        .expect("SIOCAIFADDR");
    }

    /// `ifconfig <ifp> inet <addr> delete`.
    pub(crate) fn unconfigure(ifp: &'static Ifnet, addr: [u8; 4]) {
        let mut ifra = InAliasreq::default();
        ifra.ifra_name = ifp.if_xname.get();
        ifra.ifra_addr = sin(addr);
        // SAFETY: an `in_aliasreq`, from the kernel itself.
        unsafe {
            in_ioctl(
                SIOCDIFADDR,
                ptr::from_mut(&mut ifra).cast(),
                Some(ifp),
                true,
            )
        }
        .expect("SIOCDIFADDR");
    }

    /// The bytes of a packet.
    pub(crate) fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut v = std::vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(m, 0, &mut v);
        v
    }

    /// An IPv4 packet with header `hdr` (checksummed here unless `bad_sum`) and `payload`.
    fn ip_packet(hdr: [u8; 20], payload: &[u8], bad_sum: bool) -> &'static Mbuf {
        let mut p = hdr.to_vec();
        p.extend_from_slice(payload);
        let m = test_packet(&p);
        if !bad_sum {
            let mut ip = mtod_ip(m);
            ip.ip_sum = 0;
            mtod_ip_store(m, &ip);
            ip.ip_sum = in_cksum(m, 20);
            mtod_ip_store(m, &ip);
        }
        m
    }

    /// The counter of `c`.
    fn ipstat(c: IpstatCounters) -> u64 {
        IPCOUNTERS.read()[c as usize]
    }

    #[test]
    fn ipv4_input_rejects_bad_headers() {
        let _g = setup();
        let ifp = test_ether();
        let hdr = |vhl: u8, len: u16| -> [u8; 20] {
            let l = len.to_be_bytes();
            [
                vhl, 0, l[0], l[1], 0, 0, 0, 0, 64, 1, 0, 0, 10, 0, 2, 2, 10, 0, 2, 15,
            ]
        };

        let badvers = ipstat(IpstatCounters::IpsBadvers);
        locked(|| ipv4_input(ifp, ip_packet(hdr(0x65, 20), &[], false), None));
        assert_eq!(ipstat(IpstatCounters::IpsBadvers), badvers + 1);

        let badhlen = ipstat(IpstatCounters::IpsBadhlen);
        locked(|| ipv4_input(ifp, ip_packet(hdr(0x44, 20), &[], false), None));
        assert_eq!(ipstat(IpstatCounters::IpsBadhlen), badhlen + 1);

        let badsum = ipstat(IpstatCounters::IpsBadsum);
        let m = ip_packet(hdr(0x45, 20), &[], true);
        let mut ip = mtod_ip(m);
        ip.ip_sum = 0x1234;
        mtod_ip_store(m, &ip);
        locked(|| ipv4_input(ifp, m, None));
        assert_eq!(ipstat(IpstatCounters::IpsBadsum), badsum + 1);

        let badlen = ipstat(IpstatCounters::IpsBadlen);
        locked(|| ipv4_input(ifp, ip_packet(hdr(0x45, 10), &[], false), None));
        assert_eq!(ipstat(IpstatCounters::IpsBadlen), badlen + 1);

        let tooshort = ipstat(IpstatCounters::IpsTooshort);
        locked(|| ipv4_input(ifp, ip_packet(hdr(0x45, 40), &[0; 8], false), None));
        assert_eq!(ipstat(IpstatCounters::IpsTooshort), tooshort + 1);

        // 127/8 must not appear on the wire.
        let badaddr = ipstat(IpstatCounters::IpsBadaddr);
        let mut h = hdr(0x45, 20);
        h[12] = 127;
        locked(|| ipv4_input(ifp, ip_packet(h, &[], false), None));
        assert_eq!(ipstat(IpstatCounters::IpsBadaddr), badaddr + 1);

        // A good header for an address we do not have: not forwarded (ip_forwarding is 0).
        let cantforward = ipstat(IpstatCounters::IpsCantforward);
        locked(|| ipv4_input(ifp, ip_packet(hdr(0x45, 28), &[0; 8], false), None));
        assert_eq!(ipstat(IpstatCounters::IpsCantforward), cantforward + 1);
    }

    /// A frame from the peer to us of `etype` around `payload`, as the driver would hand it up.
    pub(crate) fn frame(ifp: &Ifnet, dst: [u8; 6], etype: u16, payload: &[u8]) -> &'static Mbuf {
        let mut f = Vec::new();
        f.extend_from_slice(&dst);
        f.extend_from_slice(&PEER);
        f.extend_from_slice(&etype.to_be_bytes());
        f.extend_from_slice(payload);
        let m = test_packet(&f);
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        m
    }

    #[test]
    fn a_ping_to_the_gateway_resolves_it_and_counts_the_reply() {
        let _g = setup();
        let ifp = test_ether();

        // ifconfig tvio0 10.0.2.15/24
        configure(ifp, ADDR, [255, 255, 255, 0]);
        assert!(ifp.if_flags.get() & IFF_UP != 0, "the driver brought it up");

        // The local, cloning and broadcast routes.
        let route_flags = |a: [u8; 4]| {
            let s = sin(a);
            // rtalloc under the net lock, as its callers (route.c's NET_ASSERT_LOCKED proof).
            crate::sys::systm::net_lock();
            let rt = rtalloc(sa_of(&s), 0, 0).expect("route");
            crate::sys::systm::net_unlock();
            let f = rt.rt_flags.get();
            rtfree(Some(rt));
            f
        };
        assert_ne!(route_flags(ADDR) & RTF_LOCAL, 0);
        assert_ne!(
            route_flags([10, 0, 2, 77]) & (RTF_CLONING | RTF_CONNECTED),
            0
        );
        assert_ne!(route_flags([10, 0, 2, 255]) & RTF_BROADCAST, 0);
        // The address was announced (a gratuitous ARP request for itself).
        let m = ifq_dequeue(ifp.if_snd()).expect("announcement");
        let b = bytes(m);
        assert_eq!(&b[12..14], &ETHERTYPE_ARP.to_be_bytes());
        assert_eq!(&b[28..32], &ADDR);
        assert_eq!(&b[38..42], &ADDR);
        m_freem(m);
        while let Some(m) = ifq_dequeue(ifp.if_snd()) {
            m_freem(m);
        }

        // route add default 10.0.2.2
        let dst = sin([0; 4]);
        let mask = sin([0; 4]);
        let mut gw = sin(GATEWAY);
        let gw_copy = gw;
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_DST] = Some(sa_view(sa_of(&dst)));
        info.rti_info[RTAX_NETMASK] = Some(sa_view(sa_of(&mask)));
        info.rti_info[RTAX_GATEWAY] = Some(sa_view(sa_of(&gw_copy)));
        info.rti_flags = RTF_GATEWAY | RTF_STATIC;
        // route(8)'s rtm_getifa and rtrequest, both under the net lock (ifaof_ifpforaddr reads
        // the interface's addresses).
        net_lock();
        // SAFETY: a local `sockaddr_in`.
        let ifa = unsafe { crate::net::if_::ifaof_ifpforaddr(sintosa(&mut gw), ifp) };
        info.rti_ifa = ifa.as_ref();
        let mut rt = None;
        rtrequest(RTM_ADD, &mut info, 0, Some(&mut rt), 0).expect("default route");
        net_unlock();
        let def = rt.expect("route");
        let nh = def.gwroute().expect("next hop cached");
        assert_ne!(nh.rt_flags.get() & RTF_LLINFO, 0);
        assert_ne!(nh.rt_flags.get() & RTF_CLONED, 0);
        drop(nh);
        rtfree(rt);
        assert_ne!(route_flags([8, 8, 8, 8]) & RTF_GATEWAY, 0);

        // ping: the request is held while ARP asks for the gateway.
        let m = m_gethdr(M_DONTWAIT, crate::sys::mbuf::MT_DATA).expect("mbuf");
        let len = 20 + 8 + 16;
        m_data_add(m, ((MHLEN - len) & !7) as isize);
        m.m_len().set(len as u32);
        m.m_pkthdr().len.set(len as i32);
        let ip = Ip {
            ip_len: htons(len as u16),
            ip_ttl: 64,
            ip_p: IPPROTO_ICMP as u8,
            ip_dst: sin(GATEWAY).sin_addr,
            ..Ip::default()
        };
        mtod_ip_store(m, &ip);
        let icp = IcmpPkt::of(m, 20);
        icp.set_icmp_type(ICMP_ECHO);
        icp.set_icmp_code(0);
        icp.set_icmp_cksum(0);
        icp.set_icmp_id(htons(7));
        icp.set_icmp_seq(htons(1));
        m.m_pkthdr().csum_flags.set(M_ICMP_CSUM_OUT);
        locked(|| ip_output(m, None, None, 0, None, None, 0)).expect("held by ARP");

        let req = ifq_dequeue(ifp.if_snd()).expect("ARP request");
        let b = bytes(req);
        m_freem(req);
        assert_eq!(&b[0..6], &[0xff; 6]);
        assert_eq!(&b[12..14], &ETHERTYPE_ARP.to_be_bytes());
        assert_eq!(&b[20..22], &ARPOP_REQUEST.to_be_bytes());
        assert_eq!(&b[28..32], &ADDR, "from our address");
        assert_eq!(&b[38..42], &GATEWAY, "for the gateway");
        assert!(
            ifq_dequeue(ifp.if_snd()).is_none(),
            "the echo request is held"
        );

        // The gateway answers: the held packet goes out to its hardware address.
        let reply = EtherArp {
            ea_hdr: crate::net::if_arp::Arphdr {
                ar_hrd: htons(crate::net::if_arp::ARPHRD_ETHER),
                ar_pro: htons(ETHERTYPE_IP),
                ar_hln: 6,
                ar_pln: 4,
                ar_op: htons(ARPOP_REPLY),
            },
            arp_sha: PEER,
            arp_spa: GATEWAY,
            arp_tha: OURS,
            arp_tpa: ADDR,
        };
        // SAFETY: an `ether_arp` is plain bytes.
        let arp = unsafe {
            core::slice::from_raw_parts(ptr::from_ref(&reply).cast::<u8>(), size_of::<EtherArp>())
        };
        locked(|| ether_input(ifp, frame(ifp, OURS, ETHERTYPE_ARP, arp), None));
        locked(|| arpintr());

        let echo = ifq_dequeue(ifp.if_snd()).expect("the echo request");
        let b = bytes(echo);
        assert_eq!(&b[0..6], &PEER, "to the gateway's hardware address");
        assert_eq!(&b[6..12], &OURS);
        assert_eq!(&b[12..14], &ETHERTYPE_IP.to_be_bytes());
        let ipb = &b[size_of::<EtherHeader>()..];
        assert_eq!(ipb[0], 0x45);
        assert_eq!(&ipb[12..16], &ADDR, "ip_output chose our address");
        assert_eq!(&ipb[16..20], &GATEWAY);
        assert_eq!(ipb[20], ICMP_ECHO);
        // Both checksums are right: the header's and the delayed ICMP one.
        let m2 = test_packet(ipb);
        assert_eq!(in_cksum(m2, 20), 0, "ip header checksum");
        assert_eq!(in4_cksum(m2, 0, 20, len as i32 - 20), 0, "icmp checksum");
        m_freem(m2);
        m_freem(echo);

        // The echo reply comes back and icmp_input counts it.
        let mut r = ipb.to_vec();
        r[12..16].copy_from_slice(&GATEWAY);
        r[16..20].copy_from_slice(&ADDR);
        r[10] = 0;
        r[11] = 0;
        r[20] = ICMP_ECHOREPLY;
        r[22] = 0;
        r[23] = 0;
        let m3 = test_packet(&r);
        let mut ip = mtod_ip(m3);
        ip.ip_sum = in_cksum(m3, 20);
        mtod_ip_store(m3, &ip);
        let sum = in4_cksum(m3, 0, 20, len as i32 - 20);
        IcmpPkt::of(m3, 20).set_icmp_cksum(sum);
        let r = bytes(m3);
        m_freem(m3);

        let idx = IcmpstatCounters::IcpsInhist as usize + usize::from(ICMP_ECHOREPLY);
        let counter = || ICMPCOUNTERS.read()[idx];
        let before = counter();
        locked(|| ether_input(ifp, frame(ifp, OURS, ETHERTYPE_IP, &r), None));
        // icmp_input needs the exclusive net lock: the packet waits on ipintrq.
        assert_eq!(counter(), before);
        locked(|| ipintr());
        assert_eq!(counter(), before + 1, "echo reply received");

        // Deleting the address takes its routes, and the gateway's ARP entry, with it.
        unconfigure(ifp, ADDR);
        let s = sin(GATEWAY);
        crate::sys::systm::net_lock();
        let rt = rtalloc(sa_of(&s), 0, 0);
        crate::sys::systm::net_unlock();
        assert!(rt.is_none(), "no route to the gateway is left");
        while let Some(m) = ifq_dequeue(ifp.if_snd()) {
            m_freem(m);
        }
    }

    /// An ICMP message of `type_` from the gateway to us, with `opts` in its IP header.
    fn icmp_to_us(type_: u8, opts: &[u8]) -> &'static Mbuf {
        let hlen = 20 + opts.len();
        let len = hlen + 8 + 8;
        let mut p = std::vec![0u8; len];
        p[0] = 0x40 | (hlen / 4) as u8;
        p[2..4].copy_from_slice(&(len as u16).to_be_bytes());
        p[8] = 64;
        p[9] = IPPROTO_ICMP as u8;
        p[12..16].copy_from_slice(&GATEWAY);
        p[16..20].copy_from_slice(&ADDR);
        p[20..hlen].copy_from_slice(opts);
        p[hlen] = type_;
        p[hlen + 4..hlen + 8].copy_from_slice(&[0, 9, 0, 1]);
        let m = test_packet(&p);
        let mut ip = mtod_ip(m);
        ip.ip_sum = in_cksum(m, hlen as i32);
        mtod_ip_store(m, &ip);
        let sum = in4_cksum(m, 0, hlen as i32, (len - hlen) as i32);
        IcmpPkt::of(m, hlen).set_icmp_cksum(sum);
        m
    }

    /// Runs `ipsend_task` as the softnet thread would: `ip_output` of what `ip_send` queued.
    pub(crate) fn run_ip_send() {
        kernel::netinet::ip_input::ip_send_test_run();
    }

    /// The packets `ip_send` queued, freed after `check` looked at each.
    pub(crate) fn sent(mut check: impl FnMut(&[u8], u16)) -> usize {
        let mut ml = kernel::netinet::ip_input::ip_send_test_take();
        let mut n = 0;
        while let Some(m) = kernel::net::mbuf::ml_dequeue(&mut ml) {
            check(&bytes(&m), m.m_pkthdr().csum_flags.get());
            n += 1;
        }
        n
    }

    #[test]
    fn an_echo_request_is_answered() {
        let _g = setup();
        let ifp = test_ether();
        configure(ifp, ADDR, [255, 255, 255, 0]);
        let _ = sent(|_, _| {});

        let reflect = ICMPCOUNTERS.read()[IcmpstatCounters::IcpsReflect as usize];
        let m = icmp_to_us(ICMP_ECHO, &[]);
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        locked(|| ipv4_input(ifp, m, None));
        locked(|| ipintr());
        assert_eq!(
            ICMPCOUNTERS.read()[IcmpstatCounters::IcpsReflect as usize],
            reflect + 1
        );

        let n = sent(|b, csum| {
            assert_eq!(&b[12..16], &ADDR, "from the address the request was for");
            assert_eq!(&b[16..20], &GATEWAY, "back to the sender");
            assert_eq!(b[8], crate::netinet::ip::MAXTTL);
            assert_eq!(b[20], ICMP_ECHOREPLY);
            assert_eq!(&b[24..28], &[0, 9, 0, 1], "the id and sequence stay");
            assert_ne!(csum & M_ICMP_CSUM_OUT, 0, "the checksum is left to output");
        });
        assert_eq!(n, 1);
        unconfigure(ifp, ADDR);
    }

    #[test]
    fn a_bad_option_gets_a_parameter_problem() {
        let _g = setup();
        let ifp = test_ether();
        configure(ifp, ADDR, [255, 255, 255, 0]);
        let _ = sent(|_, _| {});

        // An option whose length runs past the header.
        let badoptions = ipstat(IpstatCounters::IpsBadoptions);
        let m = icmp_to_us(ICMP_ECHO, &[crate::netinet::ip::IPOPT_RR, 9, 4, 0]);
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        locked(|| ipv4_input(ifp, m, None));
        assert_eq!(ipstat(IpstatCounters::IpsBadoptions), badoptions + 1);

        let n = sent(|b, _| {
            assert_eq!(&b[16..20], &GATEWAY);
            assert_eq!(b[20], crate::netinet::ip_icmp::ICMP_PARAMPROB);
            assert_eq!(b[24], 21, "the pointer is the option's length byte");
            assert_eq!(
                &b[28..32],
                &[0x46, 0, 0, 40],
                "the offending header follows"
            );
        });
        assert_eq!(n, 1);
        unconfigure(ifp, ADDR);
    }
}
/* </TESTS> */
