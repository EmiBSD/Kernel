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
//! Divert sockets: `<netinet/ip_divert.h>` (its userland part is `hal::sys::
//! netinet_ip_divert`) and `netinet/ip_divert.c`. A pf rule with `divert-packet port N` hands
//! the packets it matches to the `IPPROTO_DIVERT` raw socket bound to port N
//! ([`divert_packet`]), with the address of the receiving interface (inbound) or the wildcard
//! (outbound) as the source; what the socket writes back is reinjected, into `ipv4_input`
//! for an address of ours, out through `ip_output` for the wildcard.
//!
//! Upstream: sys/netinet/ip_divert.h @ 3ce1f3f79392
//! Upstream: sys/netinet/ip_divert.c @ 3ce1f3f79392
//! LZ: sys/netinet/ip_divert.rs@3c62ede68802
//!
//! Locks used to protect data: \[a\] atomic. The requests hold the socket's lock and borrow
//! the pcb from its locked slot (raw IP's helpers, `kernel::netinet::raw_ip`);
//! `divert_packet` runs under pf's net lock and takes no socket lock (the append is under the
//! receive buffer's mutex), as the C.
//!
//! ## Deviations
//! - `divert_packet` skips the iterator markers `in_pcb_iterator` leaves in the table's queue
//!   (`kern.file`'s walk), which the C's `TAILQ_FOREACH` reads as control blocks (their
//!   `inp_lport`); LZ's deviation, kept (D-11).
//! - `divert_output` with no address (which `sosend` never passes for an unconnected
//!   `PR_ADDR` socket) fails with `EINVAL` where the C would dereference NULL; `divert_send`
//!   without a packet fails with `EINVAL` (LZ's).
//! - `DIVERTCTL_NAMES` (the `struct ctlname` table for sysctl(8)) is userland's.
//! - The IPv6 half (`divb6table`, `divert6_*`) is `kernel::netinet6::ip6_divert`.
//!
//! ## Redesign
//! - N6.6b1: LZ's divert(4) was `bsd`'s, over `&'static` sockets and parked mbufs, with a
//!   lend of the net lock for the inbound address walk and the reinjection's wrappers. Here
//!   it is the kernel's, generic over the glue: [`DivertUsrreqs`] (`divert_usrreqs`) borrow
//!   the pcb from the socket's locked slot; [`divert_packet`] takes pf's proof of the net lock
//!   (`net`): the walk to the first pcb of the port and routing domain under the table's
//!   mutex (one count, the C's `in_pcbref`), the interface's first IPv4 address read under
//!   that proof (`in_ifp2ia`), no lend. `divbtable` is the glue's ([`divbtable`],
//!   `NetTables`).
//! - `divcounters` (`struct cpumem *`) is the per-CPU [`DIVCOUNTERS`]; `divert_sendspace`
//!   and `divert_recvspace` are `AtomicI32`s, the type `sysctl_bounded_arr` takes (`u_int` in
//!   the C; the bounds keep them in `[0, SB_MAX]`).
//! - `divert_output` checks the header with raw IP's [`rip_chkhdr`] and reinjects inbound
//!   only to a valid `RTF_LOCAL` route (`EADDRNOTAVAIL` otherwise), both checksums recomputed,
//!   then `ipv4_input` with the caller's proof; outbound through `ip_output`
//!   (`IP_ALLOWBROADCAST | IP_RAWOUTPUT`), the route cache lent for the call.

use core::marker::PhantomData;
use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::netlock::NetHeld;
use hal::percpu::Counters;
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::net_route::RTF_LOCAL;
use hal::sys::netinet_in::{INADDR_ANY, IPPROTO_ICMP, IPPROTO_TCP, IPPROTO_UDP, SockaddrIn};
use hal::sys::netinet_ip::{IP_MAXPACKET, Ip};
use hal::sys::netinet_ip_divert::{
    DIVERTCTL_RECVSPACE, DIVERTCTL_SENDSPACE, DIVERTCTL_STATS, DIVS_NCOUNTERS, Divstat,
};
use hal::sys::netinet_ip_icmp::ICMP_MINLEN;
use hal::sys::netinet_tcp::Tcphdr;
use hal::sys::netinet_udp::Udphdr;
use hal::sys::socket::AF_INET;
use hal::sys::sysctl::SysctlBoundedArgs;

use crate::assert::kpanic;
use crate::net::domain::net;
use crate::net::if_::{if_get, if_put};
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{
    M_ICMP_CSUM_OUT, M_TCP_CSUM_OUT, M_UDP_CSUM_OUT, Mbuf, MbufBox, PF_TAG_DIVERTED_PACKET,
    m_pullup,
};
use crate::net::pf_hooks::{PF_IN, PF_OUT};
use crate::net::proto::{SockaddrView, Usrreqs};
use crate::net::route::{rtalloc, rtisvalid};
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::{SB_MAX, SS_PRIV};
use crate::net::socket::{SoLocked, Socket, socantsendmore, soreserve, sorwakeup};
use crate::netinet::in_::{in_control, in_ifp2ia, in_nam2sin};
use crate::netinet::in_pcb::{
    INP_HDRINCL, Inpcb, InpcbRef, Inpcbtable, in_pcb_is_iterator, in_pcballoc, in_pcbbind,
    in_pcbinit, in_peeraddr, in_sockaddr,
};
use crate::netinet::in_var::ifatoia;
use crate::netinet::ip_input::ipv4_input;
use crate::netinet::ip_output::{in_hdr_cksum_out, in_proto_cksum_out, ip_load, ip_output};
use crate::netinet::ip_var::{IP_ALLOWBROADCAST, IP_RAWOUTPUT};
use crate::netinet::raw_ip::{detach_slot, inet, pcb_of, rip_chkhdr, sa_bytes};
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::{SysctlReq, bounded_arr, rdstruct};

/// `DIVERT_SENDSPACE`.
pub const DIVERT_SENDSPACE: i32 = 65536 + 100;
/// `DIVERT_RECVSPACE`.
pub const DIVERT_RECVSPACE: i32 = 65536 + 100;
/// `DIVERT_HASHSIZE`.
pub const DIVERT_HASHSIZE: i32 = 128;

/// `enum divstat_counters`: the indices of [`DIVCOUNTERS`], one per member of [`Divstat`] in
/// its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum DivstatCounters {
    /// `divs_ipackets`.
    DivsIpackets,
    /// `divs_noport`.
    DivsNoport,
    /// `divs_fullsock`.
    DivsFullsock,
    /// `divs_opackets`.
    DivsOpackets,
    /// `divs_errors`.
    DivsErrors,
}

/// `divert_usrreqs`: the IPv4 divert sockets' user requests.
pub struct DivertUsrreqs<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> DivertUsrreqs<G> {
    /// `divert_usrreqs`.
    pub const NEW: Self = Self(PhantomData);
}

/// `divcounters`.
pub static DIVCOUNTERS: Counters<DIVS_NCOUNTERS> = Counters::new();

/// \[a\] `divert_sendspace`.
#[allow(non_upper_case_globals)] // `DIVERT_SENDSPACE` is the default, a constant of the header
pub static divert_sendspace: AtomicI32 = AtomicI32::new(DIVERT_SENDSPACE);
/// \[a\] `divert_recvspace`.
#[allow(non_upper_case_globals)] // `DIVERT_RECVSPACE` is the default, a constant of the header
pub static divert_recvspace: AtomicI32 = AtomicI32::new(DIVERT_RECVSPACE);

/// `divertctl_vars[]`.
static DIVERTCTL_VARS: [SysctlBoundedArgs; 2] = [
    SysctlBoundedArgs::new(DIVERTCTL_RECVSPACE, &divert_recvspace, 0, SB_MAX as i32),
    SysctlBoundedArgs::new(DIVERTCTL_SENDSPACE, &divert_sendspace, 0, SB_MAX as i32),
];

/// `divstat_inc(c)`.
#[inline]
pub fn divstat_inc(c: DivstatCounters) {
    DIVCOUNTERS.inc(c as usize);
}

/// `divbtable`: the glue's IPv4 divert control blocks (`NetTables`; `kern.file`'s walk reads
/// it).
pub fn divbtable<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().divbtable
}

/// `divert_init`: the control blocks' table, `DIVERT_HASHSIZE` buckets.
pub fn divert_init<G: ProcGlue>() {
    in_pcbinit(divbtable::<G>(), DIVERT_HASHSIZE);
}

/// The minimum transport header a reinjected packet of protocol `p` must hold, and the
/// checksum it asks `ip_output` or the reinjection to compute (`divert_output`'s and
/// `divert6_output`'s switch; `icmp` is the protocol whose header is `ICMP_MINLEN` or
/// `struct icmp6_hdr`, `icmp_len` that length).
pub(crate) fn divert_min_hdrlen(p: i32, icmp: i32, icmp_len: usize) -> (usize, u16) {
    match p {
        IPPROTO_TCP => (size_of::<Tcphdr>(), M_TCP_CSUM_OUT),
        IPPROTO_UDP => (size_of::<Udphdr>(), M_UDP_CSUM_OUT),
        _ if p == icmp => (icmp_len, M_ICMP_CSUM_OUT),
        _ => (0, 0),
    }
}

/// The route `dst` reaches in `rtableid`, when it is a valid local one (`RTF_LOCAL`): its
/// interface's index (`divert_output`'s inbound check; `EADDRNOTAVAIL` otherwise).
pub(crate) fn divert_local_ifidx(
    dst: SockaddrView<'_>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<u32, Errno> {
    let rt = rtalloc(dst, 0, rtableid, net);
    match rt.as_deref() {
        Some(r) if rtisvalid(Some(r)) && r.rt_flags.isset(RTF_LOCAL) => Ok(r.rt_ifidx.get()),
        _ => Err(Errno::EADDRNOTAVAIL),
    }
    // rtfree(rt): the handle's drop.
}

/// `divert_output(inp, m, nam, control)`: reinjects what a divert socket wrote: the header
/// checked as raw IP's `IP_HDRINCL` one, the transport's minimum length by protocol, the
/// packet marked diverted; to the address of ours `nam` names, inbound through `ipv4_input`
/// (only to a local route, both checksums recomputed), else out through `ip_output`. The
/// control is freed first; every failure counts an error.
fn divert_output<G: ProcGlue>(
    inp: &Inpcb<G>,
    m: MbufBox,
    nam: Option<&Mbuf>,
    control: Option<MbufBox>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    drop(control);

    let r = (|| -> Result<Result<(), Errno>, Errno> {
        let sin = in_nam2sin(nam.ok_or(Errno::EINVAL)?)?;

        if m.m_pkthdr().len.get() as usize > IP_MAXPACKET {
            return Err(Errno::EMSGSIZE);
        }

        let Some(mut m) = rip_chkhdr(m, false) else {
            return Err(Errno::EINVAL);
        };

        let ip: Ip = ip_load(&m);
        let off = usize::from(ip.ip_hl()) << 2;

        let dir = if sin.sin_addr.s_addr == INADDR_ANY {
            PF_OUT
        } else {
            PF_IN
        };

        let (min_hdrlen, csum) = divert_min_hdrlen(i32::from(ip.ip_p), IPPROTO_ICMP, ICMP_MINLEN);
        let ph = m.m_pkthdr();
        ph.csum_flags.set(ph.csum_flags.get() | csum);
        if min_hdrlen != 0 && (ph.len.get() as usize) < off + min_hdrlen {
            return Err(Errno::EINVAL);
        }

        ph.pf.flags.set(ph.pf.flags.get() | PF_TAG_DIVERTED_PACKET);

        if dir == PF_IN {
            let Some(view) = SockaddrView::new(sa_bytes(&sin)) else {
                kpanic(format_args!("divert_output: a sockaddr_in's view"));
            };
            let ifidx = divert_local_ifidx(view, inp.inp_rtableid.get(), net)?;
            m.m_pkthdr().ph_ifidx.set(ifidx);

            // Recalculate IP and protocol checksums for the inbound packet since the
            // userspace application may have modified the packet prior to reinjection.
            in_hdr_cksum_out(&mut m, None);
            in_proto_cksum_out(&mut m, None);

            let Some(ifp) = if_get(ifidx) else {
                return Err(Errno::ENETDOWN);
            };
            ipv4_input::<G>(ifp, m, None, net);
            if_put(Some(ifp));
            Ok(Ok(()))
        } else {
            m.m_pkthdr().ph_rtableid.set(inp.inp_rtableid.get());

            let mut ro = inp.route();
            Ok(ip_output(
                m,
                None,
                Some(&mut *ro),
                IP_ALLOWBROADCAST | IP_RAWOUTPUT,
                None,
                None,
                0,
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
            // fail: the packet already dropped with its box.
            divstat_inc(DivstatCounters::DivsErrors);
            Err(e)
        }
    }
}

/// The first pcb of `table` bound to `divert_port` (network order) in routing domain
/// `rdomain`, counted (the C's `in_pcbref`), under the table's mutex; the iterator markers of
/// `kern.file`'s walks are skipped (D-11). `divert_packet`'s and `divert6_packet`'s walk.
pub(crate) fn divert_lookup<G: ProcGlue>(
    table: &Inpcbtable<G>,
    divert_port: u16,
    rdomain: u32,
) -> Option<InpcbRef<G>> {
    let g = table.lock();
    let found = table
        .queue()
        .iter(&g)
        .filter(|inp| !in_pcb_is_iterator(inp))
        .find(|inp| {
            inp.inp_lport.get() == divert_port && rtable_l2(inp.inp_rtableid.get()) == rdomain
        })
        .and_then(PoolArc::from_ref);
    drop(g);
    found
}

/// `divert_packet(m, dir, divert_port)`: hands packet `m`, diverted by pf in direction `dir`
/// (`PF_IN` or `PF_OUT`), to the divert socket bound to `divert_port` in its routing domain,
/// from the receiving interface's first IPv4 address (inbound) or the wildcard (outbound, its
/// checksums computed first: pf diverts before the offload); under pf's net lock (`net`).
/// Without such a socket, or with no room, the packet is dropped and counted.
pub fn divert_packet<G: ProcGlue>(m: MbufBox, dir: i32, divert_port: u16, net: &NetHeld<'_>) {
    divstat_inc(DivstatCounters::DivsIpackets);

    let mut m = m;
    if (m.m_len().get() as usize) < size_of::<Ip>() {
        match m_pullup(m, size_of::<Ip>() as i32) {
            Some(n) => m = n,
            None => {
                divstat_inc(DivstatCounters::DivsErrors);
                return;
            }
        }
    }

    let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());
    let Some(inp) = divert_lookup(divbtable::<G>(), divert_port, rdomain) else {
        divstat_inc(DivstatCounters::DivsNoport);
        return;
    };

    let mut sin = SockaddrIn {
        sin_family: AF_INET,
        sin_len: size_of::<SockaddrIn>() as u8,
        ..SockaddrIn::default()
    };

    if dir == i32::from(PF_IN) {
        let Some(ifp) = if_get(m.m_pkthdr().ph_ifidx.get()) else {
            divstat_inc(DivstatCounters::DivsErrors);
            return;
        };
        if let Some(ia) = in_ifp2ia(&net.reader(), ifp) {
            sin.sin_addr = ifatoia(ia).ia_addr.addr();
        }
        if_put(Some(ifp));
    } else {
        // Calculate IP and protocol checksums for outbound packet diverted to userland. pf
        // rule diverts before cksum offload.
        in_hdr_cksum_out(&mut m, None);
        in_proto_cksum_out(&mut m, None);
    }

    divert_append(&inp, m, sa_bytes(&sin));
    drop(inp); // in_pcbunref(inp)
}

/// The append of `divert_packet` and `divert6_packet`: `m` from the address `sa` onto the
/// receive buffer of `inp`'s socket, the reader woken; with no room the packet is dropped
/// after the mutex is let go and counted (`divs_fullsock`).
pub(crate) fn divert_append<G: ProcGlue>(inp: &Inpcb<G>, m: MbufBox, sa: &[u8]) {
    let so = inp.socket();
    let mut rg = so.so_rcv.lock();
    let r = rg.sbappendaddr(sa, Some(m), None);
    drop(rg);
    match r {
        Ok(()) => sorwakeup(so),
        Err(refused) => {
            drop(refused);
            divstat_inc(DivstatCounters::DivsFullsock);
        }
    }
}

/// `divert_attach(so, proto, wait)`: a control block for a privileged divert socket
/// (`EINVAL` on an attached one, `EACCES` unprivileged), its buffers the tunables' sizes, the
/// user's headers taken whole (`INP_HDRINCL`), its handle in the socket's locked slot.
pub fn divert_attach<G: ProcGlue>(
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
    let inp = in_pcballoc(so, divbtable::<G>(), wait)?;

    inp.set_flags(INP_HDRINCL);
    let mut il = inet(w, "divert_attach");
    drop(so.set_pcb_locked(&mut il.pcb, Some(PoolArc::erase(inp))));
    Ok(())
}

/// `divert_detach(so)`: the control block detached; `EINVAL` without one. Shared with
/// divert6.
pub fn divert_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w, "divert_detach");

    if !detach_slot(so, &mut il) {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// `divert_bind(so, addr, p)`: `in_pcbbind` (the port pf's rule names). Shared with divert6.
pub fn divert_bind<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    addr: &Mbuf,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let il = inet(w, "divert_bind");
    let inp = pcb_of(so, &il, "divert_bind");
    in_pcbbind(inp, Some(addr), p, &il.net)
}

/// `divert_shutdown(so)`: no more sends. Shared with divert6.
pub fn divert_shutdown<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let _il = inet(w, "divert_shutdown");
    socantsendmore(so);
    Ok(())
}

/// `divert_send(so, m, addr, control)`: `divert_output`.
pub fn divert_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    addr: Option<&Mbuf>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    let il = inet(w, "divert_send");
    let inp = pcb_of(so, &il, "divert_send");
    // sosend always hands over a packet (a pkthdr mbuf, empty or not).
    let Some(m) = m else {
        return Err(Errno::EINVAL);
    };
    divert_output(inp, m, addr, control, &il.net)
}

/// `divert_sysctl_divstat`: the statistics, summed over the CPUs into the C's `u_long`
/// words, read only.
pub fn divert_sysctl_divstat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let c = DIVCOUNTERS.read();
    let divstat = Divstat {
        divs_ipackets: c[DivstatCounters::DivsIpackets as usize],
        divs_noport: c[DivstatCounters::DivsNoport as usize],
        divs_fullsock: c[DivstatCounters::DivsFullsock as usize],
        divs_opackets: c[DivstatCounters::DivsOpackets as usize],
        divs_errors: c[DivstatCounters::DivsErrors as usize],
    };
    rdstruct(req, sa_bytes(&divstat))
}

/// `divert_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the `net.inet.divert` node:
/// `stats` read only, `recvspace` and `sendspace` in `[0, SB_MAX]`; every name at this level
/// is terminal (`ENOTDIR` past it).
pub fn divert_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // All sysctl names at this level are terminal.
    let [n] = name else {
        return Err(Errno::ENOTDIR);
    };
    match *n {
        DIVERTCTL_STATS => divert_sysctl_divstat(req),
        _ => bounded_arr(&DIVERTCTL_VARS, name, req),
    }
}

impl<G: ProcGlue> Usrreqs<G> for DivertUsrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        divert_attach(so, w, proto, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        divert_detach(so, w)
    }
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        divert_bind(so, w, nam, p)
    }
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        divert_shutdown(so, w)
    }
    /// No `pru_rcvd` (the entry is not `PR_WANTRCVD`): never asked.
    fn rcvd(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_rcvd of divert"));
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        divert_send(so, w, top, addr, control)
    }
    /// No `pru_abort` (a divert socket is never on a listen queue): never asked.
    fn abort(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_abort of divert"));
    }
    /// `in_control`.
    fn control(
        &self,
        so: &Socket<G>,
        cmd: u64,
        data: &mut [u8],
        ifp: Option<&'static Ifnet>,
    ) -> Result<(), Errno> {
        in_control(so.has_state(SS_PRIV), cmd, data, ifp)
    }
    /// `in_sockaddr`.
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        in_sockaddr(so, w, addr)
    }
    /// `in_peeraddr`.
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        in_peeraddr(so, w, addr)
    }
}

// The counters are the statistics' words, the enum's end included.
const _: () = assert!(DivstatCounters::DivsErrors as usize + 1 == DIVS_NCOUNTERS);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::pin::pin;
    use core::sync::atomic::Ordering;

    use hal::sys::netinet_in::IPPROTO_UDP;

    use super::*;
    use crate::netinet::in_pcb::{in_pcb_iterator, in_pcb_iterator_abort, testinet};
    use crate::netinet::raw_ip::testraw::{
        self, DIVERT, G, close, ip4, locked, nam, pkt, recv, sin, socket, with_net,
    };

    /// The divert statistic `c`.
    fn divs(c: DivstatCounters) -> u64 {
        DIVCOUNTERS.read()[c as usize]
    }

    /// A divert socket bound to `port` (host order).
    fn bound(port: u16, p: &Thread<G>) -> crate::net::socket::SocketRef<G> {
        let so = socket(&DIVERT, 0, true).expect("attached");
        let n = nam(&sin([0, 0, 0, 0], port));
        assert_eq!(locked(&so, |w| divert_bind(&so, w, &n, p)), Ok(()));
        so
    }

    /// pf's `divert_packet` of `bytes` received on `ifidx`, in direction `dir`, to `port`.
    fn divert(bytes: &[u8], ifidx: u32, dir: u8, port: u16) {
        let m = pkt(bytes);
        m.m_pkthdr().ph_ifidx.set(ifidx);
        with_net(|net| divert_packet::<G>(m, i32::from(dir), port.to_be(), net));
    }

    /// `divert_send` of `bytes` to `to`.
    fn send(so: &Socket<G>, bytes: &[u8], to: [u8; 4]) -> Result<(), Errno> {
        let n = nam(&sin(to, 0));
        locked(so, |w| divert_send(so, w, Some(pkt(bytes)), Some(&n), None))
    }

    #[test]
    fn divert_attach_needs_privilege_and_takes_whole_headers() {
        let (_g, _p) = testraw::setup();
        assert_eq!(socket(&DIVERT, 0, false).err(), Some(Errno::EACCES));
        let so = socket(&DIVERT, 0, true).expect("attached");
        let inp = testraw::pcb(&so);
        assert!(inp.has_flags(INP_HDRINCL));
        assert!(core::ptr::eq(inp.table(), divbtable::<G>()));
        drop(inp);
        assert_eq!(so.so_rcv.hiwat(), DIVERT_RECVSPACE as u64);
        // so->so_pcb != NULL: EINVAL (the C's, not a panic).
        assert_eq!(
            locked(&so, |w| divert_attach(&so, w, 0, crate::net::mbuf::M_WAIT)),
            Err(Errno::EINVAL)
        );
        close(so);
    }

    #[test]
    fn divert_packet_reaches_the_socket_on_its_port() {
        let (_g, p) = testraw::setup();
        let (ifp, _d) = testinet::iface(b"div0", [10, 9, 80, 1]);
        let ifidx = ifp.if_index.get();
        let so = bound(7000, p);
        let b = ip4([10, 9, 80, 2], [10, 9, 80, 1], IPPROTO_UDP as u8, &[0; 8]);
        let (ipk, noport, errors) = (
            divs(DivstatCounters::DivsIpackets),
            divs(DivstatCounters::DivsNoport),
            divs(DivstatCounters::DivsErrors),
        );

        // Inbound: from the receiving interface's first IPv4 address.
        divert(&b, ifidx, PF_IN, 7000);
        let (name, data) = recv(&so).expect("diverted");
        assert_eq!(data, b);
        let from = hal::pod::at::<SockaddrIn>(&name, 0).expect("a sockaddr_in");
        assert_eq!(from.sin_addr.s_addr, u32::from_ne_bytes([10, 9, 80, 1]));
        // Outbound: from the wildcard.
        divert(&b, ifidx, PF_OUT, 7000);
        let (name, _) = recv(&so).expect("diverted");
        assert_eq!(
            hal::pod::at::<SockaddrIn>(&name, 0).map(|s| s.sin_addr.s_addr),
            Some(0)
        );
        // Another port; an inbound packet of an interface that is gone.
        divert(&b, ifidx, PF_IN, 7001);
        divert(&b, 999, PF_IN, 7000);
        assert!(recv(&so).is_none());
        assert_eq!(divs(DivstatCounters::DivsIpackets), ipk + 4);
        assert_eq!(divs(DivstatCounters::DivsNoport), noport + 1);
        assert_eq!(divs(DivstatCounters::DivsErrors), errors + 1);

        // A full receive buffer: counted, the packet dropped.
        let full = divs(DivstatCounters::DivsFullsock);
        let mut n = 0;
        while divs(DivstatCounters::DivsFullsock) == full && n < 4000 {
            divert(&b, ifidx, PF_IN, 7000);
            n += 1;
        }
        assert_eq!(divs(DivstatCounters::DivsFullsock), full + 1);
        close(so);
    }

    #[test]
    fn divert_packet_skips_an_iterator_marker() {
        let (_g, p) = testraw::setup();
        let (ifp, _d) = testinet::iface(b"div1", [10, 9, 81, 1]);
        // The queue, newest first: the bound socket, a walk's marker, the unbound one (port
        // 0, as a marker's): the C would read the marker's port and match it.
        let unbound = socket(&DIVERT, 0, true).expect("attached");
        let other = bound(7100, p);
        let table = divbtable::<G>();
        let iter = pin!(Inpcb::<G>::marker());
        let mut g = table.lock();
        let first = in_pcb_iterator(table, &mut g, None, iter.as_ref());
        assert!(first.is_some());
        drop(g);

        divert(
            &ip4([10, 9, 81, 2], [10, 9, 81, 1], 17, &[0; 8]),
            ifp.if_index.get(),
            PF_IN,
            0,
        );
        let got = recv(&unbound).is_some();
        let mut g = table.lock();
        in_pcb_iterator_abort(table, &mut g, first, iter.as_ref());
        drop(g);
        assert!(got, "the unbound socket, past the marker");
        close(unbound);
        close(other);
    }

    #[test]
    fn divert_output_reinjects_both_ways_and_refuses_what_the_c_refuses() {
        let (_g, _p) = testraw::setup();
        let (_ifp, d) = testinet::iface(b"div2", [10, 9, 82, 1]);
        let so = socket(&DIVERT, 0, true).expect("attached");
        let (op, errors) = (
            divs(DivstatCounters::DivsOpackets),
            divs(DivstatCounters::DivsErrors),
        );

        // Outbound (the wildcard): ip_output, the header as written.
        let out = ip4([10, 9, 82, 1], [10, 9, 82, 7], IPPROTO_UDP as u8, &[0; 8]);
        let sent = d.sent.load(Ordering::Relaxed);
        assert_eq!(send(&so, &out, [0, 0, 0, 0]), Ok(()));
        assert_eq!(d.sent.load(Ordering::Relaxed), sent + 1);
        let wire = d.last.lock().expect("the wire").clone();
        assert_eq!(wire[12..20], out[12..20]);
        assert_eq!(divs(DivstatCounters::DivsOpackets), op + 1);

        // Inbound: only to a local route.
        assert_eq!(
            send(&so, &out, [10, 9, 82, 7]),
            Err(Errno::EADDRNOTAVAIL),
            "10.9.82.7's route is not RTF_LOCAL"
        );
        // A header rip_chkhdr refuses; a TCP packet shorter than its header; too long.
        let mut v6 = out.clone();
        v6[0] = 0x65;
        assert_eq!(send(&so, &v6, [0, 0, 0, 0]), Err(Errno::EINVAL));
        let short_tcp = ip4([10, 9, 82, 1], [10, 9, 82, 7], 6, &[0; 10]);
        assert_eq!(send(&so, &short_tcp, [0, 0, 0, 0]), Err(Errno::EINVAL));
        let huge = std::vec![0x45; IP_MAXPACKET + 1];
        assert_eq!(send(&so, &huge, [0, 0, 0, 0]), Err(Errno::EMSGSIZE));
        assert_eq!(divs(DivstatCounters::DivsErrors), errors + 4);
        assert_eq!(divs(DivstatCounters::DivsOpackets), op + 1);
        close(so);
    }

    #[test]
    fn divert_output_reinjects_inbound_to_a_local_address() {
        let (_g, _p) = testraw::setup();
        crate::netinet::in_proto::inetsw_set::<G>(&crate::netinet::ip_input::tests::SW);
        crate::netinet::ip_input::ip_init::<G>();
        let (_ifp, _d) = testinet::iface(b"div3", [10, 9, 83, 1]);
        let so = socket(&DIVERT, 0, true).expect("attached");
        let udp = &crate::netinet::ip_input::tests::UDP;
        let before = udp.inputs.load(Ordering::Relaxed);
        let b = ip4([10, 9, 83, 2], [10, 9, 83, 1], IPPROTO_UDP as u8, &[0; 8]);
        assert_eq!(send(&so, &b, [10, 9, 83, 1]), Ok(()));
        assert!(
            udp.inputs.load(Ordering::Relaxed) > before,
            "ipv4_input delivered it to UDP"
        );
        close(so);
    }

    #[test]
    fn divert_sysctl_bounds_and_statistics() {
        let (_g, _p) = testraw::setup();
        let u = crate::testutil::USER_BASE + 0x3_c000;
        let call = |name: &[i32], oldp: usize, len: &mut usize, new: Option<i32>| {
            if let Some(v) = new {
                hal::user::UserPtr::<u8>::new(u + 0x100)
                    .copyout_slice(&v.to_ne_bytes())
                    .expect("user memory");
            }
            let mut r = SysctlReq {
                oldp,
                oldlenp: len,
                newp: if new.is_some() { u + 0x100 } else { 0 },
                newlen: if new.is_some() { 4 } else { 0 },
            };
            divert_sysctl(name, &mut r)
        };
        let mut len = 0;
        assert_eq!(call(&[DIVERTCTL_STATS], 0, &mut len, None), Ok(()));
        assert_eq!(len, 40);
        assert_eq!(call(&[DIVERTCTL_STATS], u, &mut len, None), Ok(()));
        assert!(
            crate::testutil::user_bytes(u, len)
                .and_then(|b| hal::user::pod_from_bytes::<Divstat>(&b))
                .is_some()
        );
        assert_eq!(
            call(&[DIVERTCTL_STATS, 0], 0, &mut len, None),
            Err(Errno::ENOTDIR)
        );

        let was = divert_recvspace.load(Ordering::Relaxed);
        let mut l = 4;
        assert_eq!(call(&[DIVERTCTL_RECVSPACE], 0, &mut l, Some(1000)), Ok(()));
        assert_eq!(divert_recvspace.load(Ordering::Relaxed), 1000);
        assert_eq!(
            call(&[DIVERTCTL_RECVSPACE], 0, &mut l, Some(SB_MAX as i32 + 1)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            call(&[DIVERTCTL_RECVSPACE], 0, &mut l, Some(-1)),
            Err(Errno::EINVAL)
        );
        assert_eq!(divert_recvspace.load(Ordering::Relaxed), 1000);
        divert_recvspace.store(was, Ordering::Relaxed);
    }
}
/* </TESTS> */
