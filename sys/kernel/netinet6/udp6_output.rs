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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 */
/* </LICENSES> */

/* <CODE> */
//! UDP output for IPv6 sockets: `netinet6/udp6_output.c`. UDP protocol implementation, per
//! RFC 768, August, 1980.
//!
//! Upstream: sys/netinet6/udp6_output.c @ 3ce1f3f79392
//! LZ: sys/netinet6/udp6_output.rs@3c62ede68802
//!
//! [`udp6_output`] is `udp_output`'s IPv6 half (`kernel::netinet::udp_usrreq` calls it for an
//! `INP_IPV6` control block): it checks the destination (an explicit `sockaddr_in6`, or the
//! connected peer), picks the source address, binds a local port if there is none yet,
//! prepends the IPv6 and UDP headers and hands the datagram to `ip6_output` with the
//! checksum left to `in6_proto_cksum_out` (`M_UDP_CSUM_OUT`).
//!
//! ## Deviations
//! - `NPF` and `NSTOEPLITZ` are configured: `pf_mbuf_link_inpcb` (the `Pf` bridge) and the
//!   flow id of a connected socket.
//!
//! ## Redesign
//! - N6.6b0: LZ's `bsd` module over parked mbufs, `&'static` pcbs and `bsd`'s adapters
//!   (each send lending the net lock to `ip6_output`, `in6_pcbselsrc` and
//!   `in6_pcbaddrisavail`) is the kernel's, the caller's proof of the net lock passed down.
//!   The destination `sockaddr_in6` is `in6_nam2sin6`'s copy (the C copies it too, "protect
//!   *sin6 from overwrites"); the headers are written with `hal::mem::pod::put` where `m_prepend`
//!   made room.
//! - The pcb's sticky options are lent once per send (`Inpcb::outputopts6`, under the socket
//!   lock), from `ip6_setpktopts` to `ip6_output`'s return, where LZ lent them three or four
//!   times; the loan is let go around `in6_pcbaddrisavail` alone (it reads the pcb's options
//!   itself), on the path of a bound socket sending from another address. The multicast
//!   options' scalars are copied once (`Inpcb::moptions6_out`); the C reads both pointers.
//!
//! Costs against the C: the options loan (two atomics) and the multicast copy (two) per send,
//! where the C loads two pointers; the rest is the C's.

use core::mem::size_of;

use hal::mem::pod;
use hal::sync::netlock::NetHeld;
use sysdefs::base::errno::Errno;
use sysdefs::netinet::in_::IPPROTO_UDP;
use sysdefs::netinet::ip6::{IPV6_FLOWINFO_MASK, IPV6_VERSION, IPV6_VERSION_MASK, Ip6Hdr};
use sysdefs::netinet::udp::Udphdr;
use sysdefs::netinet6::in6::{
    In6Addr, SockaddrIn6, in6_are_addr_equal, in6_is_addr_unspecified, in6_is_addr_v4mapped,
};

use crate::assert::kpanic;
use crate::net::mbuf::{M_DONTWAIT, M_FLOWID, M_UDP_CSUM_OUT, Mbuf, MbufBox, m_prepend};
use crate::net::pf_hooks::pf;
use crate::net::sockbuf::{SS_ISCONNECTED, SS_PRIV};
use crate::netinet::in_pcb::{IN6P_MINMTU, Inpcb, in_pcbbind, with_curproc};
use crate::netinet::udp_var::{UdpstatCounters, udpstat_inc};
use crate::netinet6::in6::in6_nam2sin6;
use crate::netinet6::in6_pcb::in6_pcbaddrisavail;
use crate::netinet6::in6_src::{in6_embedscope, in6_pcbselsrc, in6_selecthlim};
use crate::netinet6::ip6_output::{ip6_clearpktopts, ip6_output, ip6_setpktopts};
use crate::netinet6::ip6_var::{IPV6_MINMTU, Ip6Pktopts};
use crate::proc::ProcGlue;

/// `udp6_output`: sends datagram `m` of `inp` to the `sockaddr_in6` in `addr6` (`None`: the
/// connected peer) with control messages `control`, the socket lock held (`net` its proof of
/// the net lock). Consumes `m` and `control`.
pub fn udp6_output<G: ProcGlue>(
    inp: &Inpcb<G>,
    m: MbufBox,
    addr6: Option<&Mbuf>,
    control: Option<MbufBox>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    // `opt`, the control messages' options (the C's stack copy, which `ip6_setpktopts`
    // initialises).
    let mut opt = Ip6Pktopts::new();

    let error = udp6_send(inp, m, addr6, control.as_deref(), &mut opt, net);

    // releaseopt:
    if control.is_some() {
        ip6_clearpktopts(&mut opt, -1);
        // m_freem(control)
        drop(control);
    }
    error
}

/// `udp6_output` up to `releaseopt`: the datagram `m` consumed whatever the answer (the C's
/// `release`, or `ip6_output`); `opt` the control messages' options when there is `control`.
fn udp6_send<G: ProcGlue>(
    inp: &Inpcb<G>,
    m: MbufBox,
    addr6: Option<&Mbuf>,
    control: Option<&Mbuf>,
    opt: &mut Ip6Pktopts,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let ulen = m.m_pkthdr().len.get() as u32;
    let plen = size_of::<Udphdr>() as u32 + ulen;

    let priv_ = inp.socket().has_state(SS_PRIV);
    // `inp->inp_moptions6`'s members the scope zone, the source selection and `ip6_output`
    // read, copied once.
    let mopts = inp.moptions6_out();
    // `inp->inp_outputopts6`, lent for the send (the socket lock is held).
    let mut sticky = inp.outputopts6();

    if let Some(control) = control {
        ip6_setpktopts(control, opt, sticky.as_deref(), priv_, IPPROTO_UDP)?;
    }

    let (laddr, faddr, fport): (In6Addr, In6Addr, u16) = if let Some(addr6) = addr6 {
        // protect *sin6 from overwrites: a copy.
        let mut sin6 = in6_nam2sin6(addr6)?;
        if sin6.sin6_port == 0 {
            return Err(Errno::EADDRNOTAVAIL);
        }
        if in6_is_addr_v4mapped(&sin6.sin6_addr) {
            return Err(Errno::EADDRNOTAVAIL);
        }
        if !in6_is_addr_unspecified(&inp.inp_faddr6.get::<In6Addr>()) {
            return Err(Errno::EISCONN);
        }

        let fport = sin6.sin6_port; // allow 0 port

        // KAME hack: embed scopeid (the pcb's sticky packet info and multicast interface, as
        // the C passes `inp_outputopts6` and `inp_moptions6`).
        let pi_ifindex = sticky
            .as_deref()
            .and_then(Ip6Pktopts::pktinfo)
            .map_or(0, |p| p.ipi6_ifindex);
        let mut a = sin6.sin6_addr;
        if in6_embedscope(
            &mut a,
            &sin6,
            pi_ifindex,
            mopts.as_ref().map_or(0, |o| o.im6o_ifidx),
        )
        .is_err()
        {
            return Err(Errno::EINVAL);
        }
        sin6.sin6_addr = a;
        let faddr = sin6.sin6_addr;

        // in6_pcbselsrc(&laddr, sin6, inp, optp): `optp` the control's options, else the
        // sticky ones.
        let pi = if control.is_some() {
            opt.ip6po_pktinfo.as_deref_mut()
        } else {
            sticky
                .as_deref_mut()
                .and_then(|o| o.ip6po_pktinfo.as_deref_mut())
        };
        let laddr = in6_pcbselsrc(&sin6, inp, pi, mopts.as_ref().map(|o| o.im6o_ifidx), net)?;

        if inp.inp_lport.get() == 0 {
            with_curproc::<G, _>("udp6_output", |p| in_pcbbind(inp, None, p, net))?;
        }

        let laddr6 = inp.inp_laddr6.get::<In6Addr>();
        if !in6_is_addr_unspecified(&laddr6) && !in6_are_addr_equal(&laddr6, &laddr) {
            let mut valid = SockaddrIn6 {
                sin6_port: inp.inp_lport.get(),
                sin6_scope_id: 0,
                ..SockaddrIn6::with_addr(laddr)
            };
            // in6_pcbaddrisavail reads the pcb's sticky options itself: the loan let go
            // around it.
            drop(sticky);
            in6_pcbaddrisavail(inp, &mut valid, 0, net)?;
            sticky = inp.outputopts6();
        }
        (laddr, faddr, fport)
    } else {
        let faddr = inp.inp_faddr6.get::<In6Addr>();
        if in6_is_addr_unspecified(&faddr) {
            return Err(Errno::ENOTCONN);
        }
        (inp.inp_laddr6.get::<In6Addr>(), faddr, inp.inp_fport.get())
    };

    let hlen = size_of::<Ip6Hdr>();

    // Calculate data length and get a mbuf for UDP and IP6 headers.
    let Some(mut m) = m_prepend(m, (hlen + size_of::<Udphdr>()) as i32, M_DONTWAIT) else {
        return Err(Errno::ENOBUFS);
    };

    // Stuff checksum and output datagram.
    udp6_output_hdr(&mut m, inp, &laddr, &faddr, fport, plen);

    let ph = m.m_pkthdr();
    ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_OUT);

    let mut flags = 0;
    if inp.has_flags(IN6P_MINMTU) {
        flags |= IPV6_MINMTU;
    }

    udpstat_inc(UdpstatCounters::UdpsOpackets);

    // force routing table
    ph.ph_rtableid.set(inp.inp_rtableid.get());

    if inp.socket().has_state(SS_ISCONNECTED) {
        // NPF > 0: the packet names its pcb (pf's own reference, pf's leg).
        if let Some(pf) = pf() {
            let a: &(dyn core::any::Any + Sync) = inp;
            pf.mbuf_link_inpcb_of(&m, a);
        }
        // NSTOEPLITZ
        let ph = m.m_pkthdr();
        ph.ph_flowid.set(inp.inp_flowid.get());
        ph.csum_flags.set(ph.csum_flags.get() | M_FLOWID);
    }

    // optp: the control's options, else the sticky ones (the loan's); the route cache lent
    // for the call.
    let optp = if control.is_some() {
        Some(opt.as_opts())
    } else {
        sticky.as_deref().map(Ip6Pktopts::as_opts)
    };
    let mut ro = inp.route();
    let error = ip6_output(
        m,
        optp,
        Some(&mut *ro),
        flags,
        mopts.as_ref(),
        Some(&inp.inp_seclevel.get()),
        net,
    );
    drop(ro);
    drop(sticky);
    error
}

/// The IPv6 and UDP headers `udp6_output` writes at the front of `m` (prepended for them):
/// from `laddr` and `inp`'s local port to `faddr`.`fport`, `plen` bytes of UDP (header
/// included; a length that does not fit in 16 bits is written as 0), the checksum zero for
/// `in6_proto_cksum_out` to fill. `ip6_plen` is left to `ip6_output` (and the IPv6 header's
/// other bytes as the prepended space held them, as the C leaves them).
fn udp6_output_hdr<G: ProcGlue>(
    m: &mut Mbuf,
    inp: &Inpcb<G>,
    laddr: &In6Addr,
    faddr: &In6Addr,
    fport: u16,
    plen: u32,
) {
    let hlen = size_of::<Ip6Hdr>();
    let Some(d) = m.data_mut() else {
        kpanic(format_args!("udp6_output: a new header mbuf is read-only"));
    };

    let udp6 = Udphdr {
        uh_sport: inp.inp_lport.get(), // lport is always set in the PCB
        uh_dport: fport,
        uh_ulen: if plen <= 0xffff {
            (plen as u16).to_be()
        } else {
            0
        },
        uh_sum: 0,
    };
    if !pod::put(d, hlen, &udp6) {
        kpanic(format_args!("udp6_output: no room for the headers"));
    }
    let Some(mut ip6) = pod::at::<Ip6Hdr>(d, 0) else {
        kpanic(format_args!("udp6_output: no room for the headers"));
    };
    ip6.ip6_flow = inp.inp_flowinfo() & IPV6_FLOWINFO_MASK;
    ip6.set_ip6_vfc((ip6.ip6_vfc() & !IPV6_VERSION_MASK) | IPV6_VERSION);
    // ip6_plen will be filled in ip6_output.
    ip6.ip6_nxt = IPPROTO_UDP as u8;
    ip6.ip6_hlim = in6_selecthlim(inp) as u8;
    ip6.ip6_src = *laddr;
    ip6.ip6_dst = *faddr;
    let _ = pod::put(d, 0, &ip6);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // bsd's two tests re-expressed over the kernel's IPv6 test interface: the headers and the
    // checksum of a datagram, and the destinations that cannot be sent to.

    use super::*;
    use crate::netinet::in_pcb::sotoinpcb;
    use crate::netinet::tcp_subr::testtcp::{a6, iface6};
    use crate::netinet::udp_usrreq::testudp::{
        self, Udp6Proto, cksum6_ok, close, locked, nam, pkt, sin6,
    };
    use crate::netinet::udp_usrreq::{udp_connect, udp_send};

    #[test]
    fn the_headers_and_the_checksum_of_a_datagram() {
        let (_g, _p) = testudp::setup();
        let n = 210;
        let d = iface6(b"udp6o0", n);
        let so = testudp::socket(&Udp6Proto);

        let r = locked(&so, |w| {
            udp_send(
                &so,
                w,
                Some(pkt(b"v6 data")),
                Some(&nam(&sin6(a6(n, 2), 53))),
                None,
            )
        });
        r.expect("sendto");
        let p = d.last.lock().expect("the capture").clone();
        let ip6: Ip6Hdr = pod::at(&p, 0).expect("an IPv6 header");
        let uh: Udphdr = pod::at(&p, 40).expect("a UDP header");
        assert_eq!(ip6.ip6_vfc() & IPV6_VERSION_MASK, IPV6_VERSION);
        assert_eq!(ip6.ip6_nxt, IPPROTO_UDP as u8);
        assert_eq!(ip6.ip6_src, a6(n, 1));
        assert_eq!(ip6.ip6_dst, a6(n, 2));
        assert_eq!(
            u16::from_be(ip6.ip6_plen),
            15,
            "ip6_output's payload length"
        );
        assert_eq!(u16::from_be(uh.uh_dport), 53);
        assert_eq!(u16::from_be(uh.uh_ulen), 15);
        let inp = sotoinpcb(&so).expect("its pcb");
        assert_eq!(uh.uh_sport, inp.inp_lport.get(), "the port picked");
        assert_ne!(uh.uh_sport, 0);
        assert_eq!(i32::from(ip6.ip6_hlim), in6_selecthlim(&inp));
        assert!(cksum6_ok(&p), "in6_proto_cksum_out's checksum");
        assert_eq!(&p[48..], b"v6 data");
        drop(inp);

        close(so);
    }

    #[test]
    fn destinations_that_cannot_be_sent_to() {
        let (_g, _p) = testudp::setup();
        let n = 211;
        let _d = iface6(b"udp6o1", n);
        let so = testudp::socket(&Udp6Proto);
        let send = |to: Option<SockaddrIn6>| {
            let to = to.map(|s| nam(&s));
            locked(&so, |w| {
                udp_send(&so, w, Some(pkt(b"x")), to.as_deref(), None)
            })
        };

        // Port 0, a v4-mapped address, no destination unconnected.
        assert_eq!(send(Some(sin6(a6(n, 2), 0))), Err(Errno::EADDRNOTAVAIL));
        let mut mapped = In6Addr::default();
        mapped.s6_addr[10] = 0xff;
        mapped.s6_addr[11] = 0xff;
        mapped.s6_addr[12..].copy_from_slice(&[10, 9, n, 2]);
        assert_eq!(send(Some(sin6(mapped, 53))), Err(Errno::EADDRNOTAVAIL));
        assert_eq!(send(None), Err(Errno::ENOTCONN));

        // Connected: an address refused.
        locked(&so, |w| udp_connect(&so, w, &nam(&sin6(a6(n, 2), 53)))).expect("connect");
        send(None).expect("send");
        assert_eq!(send(Some(sin6(a6(n, 2), 53))), Err(Errno::EISCONN));

        close(so);
    }
}
/* </TESTS> */
