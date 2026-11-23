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
 * Copyright (c) 2009 Michele Marchetto <michele@openbsd.org>
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
/* </LICENSES> */

/* <CODE> */
//! Divert sockets for IPv6: `netinet6/ip6_divert.c` (prototypes in `<netinet/ip_divert.h>`,
//! `kernel::netinet::ip_divert`). A pf rule with `divert-packet port N` hands the IPv6
//! packets it matches to the `IPPROTO_DIVERT` raw socket of the inet6 domain bound to port N
//! ([`divert6_packet`]), with an IPv6 address of the receiving interface (inbound) or the
//! wildcard (outbound) as the source; what the socket writes back is reinjected, into
//! `ipv6_input` for an address of ours, out through `ip6_output` for the wildcard.
//!
//! Upstream: sys/netinet6/ip6_divert.c @ 3ce1f3f79392
//! LZ: sys/netinet6/ip6_divert.rs@3c62ede68802
//!
//! Locks used to protect data: \[a\] atomic; the rest as IPv4's divert.
//!
//! ## Deviations
//! - `div6counters` is the per-CPU [`DIV6COUNTERS`], indexed by IPv4's `DivstatCounters`;
//!   `counters_alloc` in `divert6_init` has nothing left to do. As in the C, the IPv6 paths
//!   count in `divcounters` (`divstat_inc`), so `div6counters` stays at zero.
//! - `divert6_packet` skips the iterator markers of `kern.file`'s walks, as `divert_packet`
//!   does (D-11).
//! - `divert6_output` with no address (which `sosend` never passes for an unconnected
//!   `PR_ADDR` socket) fails with `EINVAL` where the C would dereference NULL, and
//!   `divert6_send` without a packet fails with `EINVAL`, as `divert_send` does (LZ's).
//!
//! ## Redesign
//! - N6.6b1: LZ's divert6 was `bsd`'s, over `&'static` sockets and parked mbufs; here it is
//!   the kernel's, in IPv4 divert's shape and over its helpers (the lookup, the append, the
//!   minimum header, the local route's check): [`Divert6Usrreqs`] (`divert6_usrreqs`, whose
//!   detach, bind and shutdown are divert's) borrow the pcb from the socket's locked slot;
//!   [`divert6_packet`] takes pf's proof of the net lock, the interface's first IPv6 address
//!   read under it. `divb6table` is the glue's ([`divb6table`], `NetTables`).

use core::marker::PhantomData;
use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::netlock::NetHeld;
use hal::percpu::Counters;
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::netinet_icmp6::Icmp6Hdr;
use hal::sys::netinet_in::{IPPROTO_ICMPV6, IPPROTO_IPV6};
use hal::sys::netinet_ip_divert::DIVS_NCOUNTERS;
use hal::sys::netinet_ip6::{IPV6_VERSION, IPV6_VERSION_MASK, Ip6Hdr};
use hal::sys::netinet6_in6::{SockaddrIn6, in6_is_addr_unspecified};
use hal::sys::socket::AF_INET6;

use crate::assert::kpanic;
use crate::net::domain::net;
use crate::net::if_::{if_get, if_put};
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{Mbuf, MbufBox, PF_TAG_DIVERTED_PACKET, m_pullup};
use crate::net::pf_hooks::{PF_IN, PF_OUT};
use crate::net::proto::{SockaddrView, Usrreqs};
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::SS_PRIV;
use crate::net::socket::{SoLocked, Socket, soreserve};
use crate::netinet::in_pcb::{Inpcb, Inpcbtable, in_pcballoc, in_pcbinit};
use crate::netinet::ip_divert::{
    DIVERT_HASHSIZE, DivstatCounters, divert_append, divert_bind, divert_detach,
    divert_local_ifidx, divert_lookup, divert_min_hdrlen, divert_recvspace, divert_sendspace,
    divert_shutdown, divstat_inc,
};
use crate::netinet::ip_var::{IP_ALLOWBROADCAST, IP_RAWOUTPUT};
use crate::netinet::raw_ip::{inet, pcb_of, sa_bytes};
use crate::netinet6::in6::{in6_control, in6_nam2sin6};
use crate::netinet6::in6_pcb::{in6_peeraddr, in6_sockaddr};
use crate::netinet6::in6_var::{In6Ifaddr, ia6_in6};
use crate::netinet6::ip6_input::{ip6_lasthdr, ipv6_input};
use crate::netinet6::ip6_output::{in6_proto_cksum_out, ip6_load, ip6_output};
use crate::proc::{ProcGlue, Thread};

/// `divert6_usrreqs`: the IPv6 divert sockets' user requests (detach, bind and shutdown are
/// IPv4's).
pub struct Divert6Usrreqs<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Divert6Usrreqs<G> {
    /// `divert6_usrreqs`.
    pub const NEW: Self = Self(PhantomData);
}

/// `div6counters`: allocated as the C's, never counted in (the IPv6 paths count in
/// `divcounters`).
pub static DIV6COUNTERS: Counters<DIVS_NCOUNTERS> = Counters::new();

/// `divb6table`: the glue's IPv6 divert control blocks (`NetTables`; `kern.file`'s walk
/// reads it).
pub fn divb6table<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().divb6table
}

/// `divert6_init`: the control blocks' table, `DIVERT_HASHSIZE` buckets.
pub fn divert6_init<G: ProcGlue>() {
    in_pcbinit(divb6table::<G>(), DIVERT_HASHSIZE);
}

/// `divert6_output(inp, m, nam, control)`: reinjects what an IPv6 divert socket wrote: a
/// whole version-6 header whose payload length the packet holds, the last header's protocol
/// found (`ip6_lasthdr`) and its minimum length checked, the packet marked diverted; to the
/// address of ours `nam` names, inbound through `ipv6_input` (only to a local route, the
/// protocol checksum recomputed), else out through `ip6_output`. The control is freed first;
/// every failure counts an error and answers `EINVAL` unless it has an errno of its own; a
/// failed `m_pullup` answers `ENOBUFS`.
fn divert6_output<G: ProcGlue>(
    inp: &Inpcb<G>,
    m: MbufBox,
    nam: Option<&Mbuf>,
    control: Option<MbufBox>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    drop(control);

    let r = (|| -> Result<Result<(), Errno>, Option<Errno>> {
        let sin6 = in6_nam2sin6(nam.ok_or(Some(Errno::EINVAL))?).map_err(Some)?;

        // Do basic sanity checks.
        if (m.m_pkthdr().len.get() as usize) < size_of::<Ip6Hdr>() {
            return Err(None);
        }
        let Some(mut m) = m_pullup(m, size_of::<Ip6Hdr>() as i32) else {
            // m_pullup() has freed the mbuf, so just return (an error counted, ENOBUFS).
            return Err(Some(Errno::ENOBUFS));
        };
        let ip6 = ip6_load(&m);
        if ip6.ip6_vfc() & IPV6_VERSION_MASK != IPV6_VERSION {
            return Err(None);
        }
        if (m.m_pkthdr().len.get() as usize)
            < size_of::<Ip6Hdr>() + usize::from(u16::from_be(ip6.ip6_plen))
        {
            return Err(None);
        }

        // Recalculate the protocol checksum since the userspace application may have modified
        // the packet prior to reinjection.
        let mut nxt = 0;
        let off = ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt).unwrap_or(-1);
        if off < size_of::<Ip6Hdr>() as i32 {
            return Err(None);
        }

        let dir = if in6_is_addr_unspecified(&sin6.sin6_addr) {
            PF_OUT
        } else {
            PF_IN
        };

        let (min_hdrlen, csum) = divert_min_hdrlen(nxt, IPPROTO_ICMPV6, size_of::<Icmp6Hdr>());
        let ph = m.m_pkthdr();
        ph.csum_flags.set(ph.csum_flags.get() | csum);
        if min_hdrlen != 0 && i64::from(ph.len.get()) < i64::from(off) + min_hdrlen as i64 {
            return Err(None);
        }

        ph.pf.flags.set(ph.pf.flags.get() | PF_TAG_DIVERTED_PACKET);

        if dir == PF_IN {
            let Some(view) = SockaddrView::new(sa_bytes(&sin6)) else {
                kpanic(format_args!("divert6_output: a sockaddr_in6's view"));
            };
            let ifidx = divert_local_ifidx(view, inp.inp_rtableid.get(), net).map_err(Some)?;
            m.m_pkthdr().ph_ifidx.set(ifidx);

            // Recalculate the protocol checksum for the inbound packet since the userspace
            // application may have modified the packet prior to reinjection.
            in6_proto_cksum_out(&mut m, None);

            let Some(ifp) = if_get(ifidx) else {
                return Err(Some(Errno::ENETDOWN));
            };
            ipv6_input::<G>(ifp, m, None, net);
            if_put(Some(ifp));
            Ok(Ok(()))
        } else {
            m.m_pkthdr().ph_rtableid.set(inp.inp_rtableid.get());

            let mut ro = inp.route();
            Ok(ip6_output(
                m,
                None,
                Some(&mut *ro),
                IP_ALLOWBROADCAST | IP_RAWOUTPUT,
                None,
                None,
                net,
            ))
        }
    })();

    match r {
        Ok(error) => {
            divstat_inc(DivstatCounters::DivsOpackets);
            error
        }
        Err(e) => {
            // fail: the packet already dropped with its box; `error ? error : EINVAL`.
            divstat_inc(DivstatCounters::DivsErrors);
            Err(e.unwrap_or(Errno::EINVAL))
        }
    }
}

/// `divert6_packet(m, dir, divert_port)`: hands IPv6 packet `m`, diverted by pf in direction
/// `dir`, to the divert socket bound to `divert_port` in its routing domain, from the
/// receiving interface's first IPv6 address (inbound) or the wildcard (outbound, its protocol
/// checksum computed first); under pf's net lock (`net`). Without such a socket, or with no
/// room, the packet is dropped and counted (in `divcounters`, as the C).
pub fn divert6_packet<G: ProcGlue>(m: MbufBox, dir: i32, divert_port: u16, net: &NetHeld<'_>) {
    divstat_inc(DivstatCounters::DivsIpackets);

    let mut m = m;
    if (m.m_len().get() as usize) < size_of::<Ip6Hdr>() {
        match m_pullup(m, size_of::<Ip6Hdr>() as i32) {
            Some(n) => m = n,
            None => {
                divstat_inc(DivstatCounters::DivsErrors);
                return;
            }
        }
    }

    let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());
    let Some(inp) = divert_lookup(divb6table::<G>(), divert_port, rdomain) else {
        divstat_inc(DivstatCounters::DivsNoport);
        return;
    };

    let mut sin6 = SockaddrIn6 {
        sin6_family: AF_INET6,
        sin6_len: size_of::<SockaddrIn6>() as u8,
        ..SockaddrIn6::default()
    };

    if dir == i32::from(PF_IN) {
        let Some(ifp) = if_get(m.m_pkthdr().ph_ifidx.get()) else {
            divstat_inc(DivstatCounters::DivsErrors);
            return;
        };
        let rd = net.reader();
        if let Some(ia6) = ifp.if_addrlist().iter(&rd).find_map(In6Ifaddr::of) {
            sin6.sin6_addr = ia6_in6(ia6);
        }
        drop(rd);
        if_put(Some(ifp));
    } else {
        // Calculate protocol checksum for outbound packet diverted to userland. pf out rule
        // diverts before cksum offload.
        in6_proto_cksum_out(&mut m, None);
    }

    divert_append(&inp, m, sa_bytes(&sin6));
    drop(inp); // in_pcbunref(inp)
}

/// `divert6_attach(so, proto, wait)`: a control block for a privileged IPv6 divert socket
/// (`EINVAL` on an attached one, `EACCES` unprivileged), its buffers the tunables' sizes, its
/// handle in the socket's locked slot. (The C sets no `INP_HDRINCL` here.)
pub fn divert6_attach<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    _proto: i32,
    wait: i32,
) -> Result<(), Errno> {
    if so.has_pcb() {
        return Err(Errno::EINVAL);
    }
    if !so.has_state(SS_PRIV) {
        return Err(Errno::EACCES);
    }

    soreserve(
        so,
        divert_sendspace.load(Ordering::Relaxed) as u32 as u64,
        divert_recvspace.load(Ordering::Relaxed) as u32 as u64,
    )?;
    let inp = in_pcballoc(so, divb6table::<G>(), wait)?;

    let mut il = inet(w, "divert6_attach");
    drop(so.set_pcb_locked(&mut il.pcb, Some(PoolArc::erase(inp))));
    Ok(())
}

/// `divert6_send(so, m, addr, control)`: `divert6_output`.
pub fn divert6_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    addr: Option<&Mbuf>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    let il = inet(w, "divert6_send");
    let inp = pcb_of(so, &il, "divert6_send");
    // sosend always hands over a packet (a pkthdr mbuf, empty or not).
    let Some(m) = m else {
        return Err(Errno::EINVAL);
    };
    divert6_output(inp, m, addr, control, &il.net)
}

impl<G: ProcGlue> Usrreqs<G> for Divert6Usrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        divert6_attach(so, w, proto, wait)
    }
    /// `divert_detach`.
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        divert_detach(so, w)
    }
    /// `divert_bind`.
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        divert_bind(so, w, nam, p)
    }
    /// `divert_shutdown`.
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        divert_shutdown(so, w)
    }
    /// No `pru_rcvd` (the entry is not `PR_WANTRCVD`): never asked.
    fn rcvd(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_rcvd of divert6"));
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        divert6_send(so, w, top, addr, control)
    }
    /// No `pru_abort` (a divert socket is never on a listen queue): never asked.
    fn abort(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_abort of divert6"));
    }
    /// `in6_control`.
    fn control(
        &self,
        so: &Socket<G>,
        cmd: u64,
        data: &mut [u8],
        ifp: Option<&'static Ifnet>,
    ) -> Result<(), Errno> {
        in6_control(so.has_state(SS_PRIV), cmd, data, ifp)
    }
    /// `in6_sockaddr`.
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        in6_sockaddr(so, w, addr)
    }
    /// `in6_peeraddr`.
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        in6_peeraddr(so, w, addr)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use hal::pod;
    use hal::sys::netinet6_in6::In6Addr;

    use super::*;
    use crate::netinet::ip_divert::DIVCOUNTERS;
    use crate::netinet::raw_ip::testraw::{
        self, DIVERT6, G, close, locked, nam, nam6, pkt, recv, socket, with_net,
    };
    use crate::netinet::tcp_subr::testtcp::{a6, iface6};

    /// The divert statistic `c` (`divcounters`, which IPv6 counts in).
    fn divs(c: DivstatCounters) -> u64 {
        DIVCOUNTERS.read()[c as usize]
    }

    /// An IPv6 UDP packet from `src` to `dst` with an 8-byte payload, its payload length
    /// `plen`.
    fn udp6(src: In6Addr, dst: In6Addr, plen: u16) -> std::vec::Vec<u8> {
        let mut h = Ip6Hdr {
            ip6_plen: plen.to_be(),
            ip6_nxt: 17,
            ip6_hlim: 64,
            ip6_src: src,
            ip6_dst: dst,
            ..Ip6Hdr::zeroed()
        };
        h.set_ip6_vfc(IPV6_VERSION);
        let mut b = std::vec![0u8; 48];
        assert!(pod::put(&mut b, 0, &h));
        b
    }

    /// `divert6_send` of `bytes` to `to`.
    fn send(so: &Socket<G>, bytes: &[u8], to: In6Addr) -> Result<(), Errno> {
        let n = nam6(to);
        locked(so, |w| {
            divert6_send(so, w, Some(pkt(bytes)), Some(&n), None)
        })
    }

    #[test]
    fn divert6_packet_reaches_the_socket_on_its_port() {
        let (_g, p) = testraw::setup();
        let _d = iface6(b"div6a", 84);
        let ifidx = crate::net::if_::if_unit(b"div6a").map(|i| i.if_index.get());
        let so = socket(&DIVERT6, 0, true).expect("attached");
        assert!(!testraw::pcb(&so).has_flags(crate::netinet::in_pcb::INP_HDRINCL));
        let n = nam(&testraw::sin([0, 0, 0, 0], 7200));
        assert_eq!(
            locked(&so, |w| divert_bind(&so, w, &n, p)).err(),
            Some(Errno::EAFNOSUPPORT),
            "an IPv4 name for an IPv6 socket"
        );
        let n6 = {
            let mut s = SockaddrIn6 {
                sin6_len: 28,
                sin6_family: AF_INET6,
                sin6_port: 7200u16.to_be(),
                ..SockaddrIn6::default()
            };
            s.sin6_addr = In6Addr::default();
            nam(&s)
        };
        assert_eq!(locked(&so, |w| divert_bind(&so, w, &n6, p)), Ok(()));
        let six = DIV6COUNTERS.read();
        let ipk = divs(DivstatCounters::DivsIpackets);

        let b = udp6(a6(84, 2), a6(84, 1), 8);
        let m = pkt(&b);
        m.m_pkthdr().ph_ifidx.set(ifidx.unwrap_or(0));
        with_net(|net| divert6_packet::<G>(m, i32::from(PF_IN), 7200u16.to_be(), net));
        let (name, data) = recv(&so).expect("diverted");
        assert_eq!(data, b);
        let from = pod::at::<SockaddrIn6>(&name, 0).expect("a sockaddr_in6");
        // The first IPv6 address of the interface's list (the C's walk): the link-local one
        // in6_ifattach made before 2001:db8:84::1.
        assert!(
            hal::sys::netinet6_in6::in6_is_addr_linklocal(&from.sin6_addr),
            "the interface's first IPv6 address"
        );
        with_net(|net| divert6_packet::<G>(pkt(&b), i32::from(PF_OUT), 7200u16.to_be(), net));
        let (name, _) = recv(&so).expect("diverted");
        let from = pod::at::<SockaddrIn6>(&name, 0).expect("a sockaddr_in6");
        assert!(in6_is_addr_unspecified(&from.sin6_addr));
        assert_eq!(
            divs(DivstatCounters::DivsIpackets),
            ipk + 2,
            "divcounters, as the C"
        );
        assert_eq!(DIV6COUNTERS.read(), six, "div6counters never counted in");
        close(so);
    }

    #[test]
    fn divert6_output_checks_the_header_and_reinjects() {
        let (_g, _p) = testraw::setup();
        let d = iface6(b"div6b", 85);
        let so = socket(&DIVERT6, 0, true).expect("attached");
        let (op, errors) = (
            divs(DivstatCounters::DivsOpackets),
            divs(DivstatCounters::DivsErrors),
        );
        let any = In6Addr::default();
        let good = udp6(a6(85, 1), a6(85, 2), 8);

        // Short, version 4, a payload length past the packet: EINVAL.
        assert_eq!(send(&so, &good[..39], any), Err(Errno::EINVAL));
        let mut v4 = good.clone();
        v4[0] = 0x45;
        assert_eq!(send(&so, &v4, any), Err(Errno::EINVAL));
        assert_eq!(
            send(&so, &udp6(a6(85, 1), a6(85, 2), 9), any),
            Err(Errno::EINVAL)
        );
        // Inbound to an address that is not ours: EADDRNOTAVAIL.
        assert_eq!(send(&so, &good, a6(85, 2)), Err(Errno::EADDRNOTAVAIL));
        assert_eq!(divs(DivstatCounters::DivsErrors), errors + 4);

        // Outbound: ip6_output, the packet on the wire.
        let sent = d.sent.load(core::sync::atomic::Ordering::Relaxed);
        assert_eq!(send(&so, &good, any), Ok(()));
        assert_eq!(d.sent.load(core::sync::atomic::Ordering::Relaxed), sent + 1);
        assert_eq!(divs(DivstatCounters::DivsOpackets), op + 1);
        close(so);
    }
}
/* </TESTS> */
