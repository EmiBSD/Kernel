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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)uipc_domain.c	8.2 (Berkeley) 10/18/93
 */
/* </LICENSES> */

/* <CODE> */
//! The communication domains: `domains[]` of `bsd`'s tables, the kernel's traits over them,
//! the protocol lookups `bsd`'s input paths make and the `net.*` sysctl tree.
//!
//! Upstream: sys/kern/uipc_domain.c @ 3ce1f3f79392
//!
//! The kernel's socket layer (`kernel::net`) reaches `bsd`'s protocols through its traits:
//! `struct domain`, `struct protosw` and `struct pr_usrreqs` of `bsd`'s tables implement
//! `kernel::net::domain::Domain`, `kernel::net::proto::Protocol` and
//! `kernel::net::proto::Usrreqs` here (the `Protocol` row of `bridges.toml`), each method
//! calling the table's entry with the C's arguments; a NULL entry answers what the C's
//! `pru_*` wrapper answers (`EOPNOTSUPP`, 0), and an entry the C calls unchecked is
//! `panic(9)` when NULL, as the C's call through NULL. [`NET_DOMAINS`] is the kernel's
//! `domains[]` (the UNIX domain is the kernel's own), which [`domaininit`] hands over.
//!
//! ## Deviations
//! - `domains[]` is a slice without the C's NULL terminator. It holds `inetdomain`
//!   (`netinet/in_proto.rs`), `unixdomain` and `routedomain` (the kernel's), with
//!   `pfkeydomain` (`net/pfkeyv2.rs`, `IPSEC`) first and
//!   `inet6domain` (`netinet6/in6_proto.rs`, `INET6`: the `inet6` feature) before
//!   `inetdomain`. `MPLS` and `NAF_FRAME` are not configured; their entries are comments,
//!   and so are the `PIPEX` branches of `net_sysctl`. `NPFLOW` is configured: `PF_PFLOW`
//!   goes to `pflow_sysctl`; so is `NBPFILTER`: `PF_BPF` goes to `bpf_sysctl`.
//! - `pffinddomain`, `pffindtype` and `pffindproto` over `bsd`'s tables return `Option`s for
//!   the C's NULL, for `bsd`'s input paths (`ip_input`'s raw protocol); `pfctlinput` is the
//!   kernel's (`kernel::net::domain`, over its `domains[]`): these tables' `pr_ctlinput` is
//!   reached through their `Protocol` impl, the one raw call of the table's function.
//! - `net_sysctl` follows `kern_sysctl.rs`'s calling convention (`docs/C_TO_RUST.md`);
//!   `sysctl_vslock` wraps the protocol handlers that are not `PR_MPSYSCTL`, as in C.

use core::ptr;

use kernel::net::domain::Domain as KDomain;
use kernel::net::mbuf::MbufBox;
use kernel::net::proto::{Protocol, SockaddrView, Usrreqs};
use kernel::net::socket::SoLocked;
use kernel::netinet::in_proto::{IcmpProtocol, IgmpProtocol, InetDomain, IpProtocol};

use crate::kern::kern_sysctl::sysctl_vslock;
use crate::kern::proc_glue::BsdProcGlue as G;
use crate::kern::subr_prf::panic;
use crate::net::bpf::bpf_sysctl;
use crate::net::ifq::net_ifiq_sysctl;
use crate::net::pfkeyv2::{PFKEYDOMAIN, PFKEYSW};
use crate::netinet::in_proto::{INETDOMAIN, INETSW};
#[cfg(feature = "inet6")]
use crate::netinet6::in6_proto::{INET6DOMAIN, INET6SW};
use crate::sys::domain::Domain;
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, lend, park, unpark};
use crate::sys::proc::Proc;
use crate::sys::protosw::{PR_MPSYSCTL, PrUsrreqs, Protosw};
use crate::sys::socket::{
    AF_INET, NET_LINK_IFRXQ, PF_BPF, PF_LINK, PF_PFLOW, PF_UNIX, PF_UNSPEC, SOCK_RAW,
};
use crate::sys::socketvar::{Socket, as_static};
use crate::sys::stat::Stat;
use hal::sys::netinet_in::IPPROTO_DONE;
use hal::sys::netinet_ip_icmp::ICMP_MINLEN;

/// `domains[]` of `bsd`'s tables: the configured communication domains whose protocols are
/// still `bsd`'s.
pub static DOMAINS: &[&Domain] = &[
    // MPLS: &mplsdomain, not configured.
    &PFKEYDOMAIN,
    #[cfg(feature = "inet6")]
    &INET6DOMAIN,
    &INETDOMAIN,
    // routedomain: the kernel's (NET_DOMAINS).
    // NAF_FRAME: &framedomain, not configured.
];

/// `domains[]` as the kernel's socket layer sees it, in the C's order: `bsd`'s tables
/// through the traits, and the kernel's UNIX domain.
pub static NET_DOMAINS: &[&dyn KDomain<G>] = &[
    // MPLS: &mplsdomain, not configured.
    &PFKEYDOMAIN,
    #[cfg(feature = "inet6")]
    &INET6DOMAIN,
    &InetDomain::<G>::NEW,
    &kernel::net::unix::Unix::<G>::NEW,
    &kernel::net::rtsock::RouteDomain::<G>::NEW,
    // NAF_FRAME: &framedomain, not configured.
];

/// The entries of a protocol table, as the kernel's protocols.
macro_rules! protocols {
    ($sw:ident; $($i:literal)*) => { [$(&$sw[$i] as &dyn Protocol<G>),*] };
}

/// `inetsw[]` as the kernel's protocols, in the C's order: the kernel's IP, ICMP and IGMP
/// entries (`kernel::netinet::in_proto`), `bsd`'s rows for the rest.
#[cfg(not(feature = "inet6"))]
pub(crate) static INET_PROTOS: [&dyn Protocol<G>; 13] = [
    &IpProtocol::<G>::NEW,
    &INETSW[1],
    &INETSW[2],
    &INETSW[3],
    &IcmpProtocol::<G>::NEW,
    &INETSW[5],
    &IgmpProtocol::<G>::NEW,
    &INETSW[7],
    &INETSW[8],
    &INETSW[9],
    &INETSW[10],
    &INETSW[11],
    &INETSW[12],
];
/// `inetsw[]` as the kernel's protocols, in the C's order: the kernel's IP, ICMP and IGMP
/// entries (`kernel::netinet::in_proto`), `bsd`'s rows for the rest.
#[cfg(feature = "inet6")]
pub(crate) static INET_PROTOS: [&dyn Protocol<G>; 14] = [
    &IpProtocol::<G>::NEW,
    &INETSW[1],
    &INETSW[2],
    &INETSW[3],
    &IcmpProtocol::<G>::NEW,
    &INETSW[5],
    &INETSW[6],
    &IgmpProtocol::<G>::NEW,
    &INETSW[8],
    &INETSW[9],
    &INETSW[10],
    &INETSW[11],
    &INETSW[12],
    &INETSW[13],
];

/// `inet6sw[]` as the kernel's protocols.
#[cfg(feature = "inet6")]
static INET6_PROTOS: [&dyn Protocol<G>; 15] =
    protocols!(INET6SW; 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14);
/// `pfkeysw[]` as the kernel's protocols.
static PFKEY_PROTOS: [&dyn Protocol<G>; 1] = protocols!(PFKEYSW; 0);

/// `domaininit`: the kernel's domains (`bsd`'s tables and the UNIX domain) and their
/// protocols initialised, the protocol timeouts started (`kernel::net::domain`).
pub fn domaininit() {
    // in_init's half that is bsd's: the key of bsd's pcb counts (the kernel's InetDomain
    // runs the kernel's in_init).
    let _ = crate::netinet::in_pcb::legacy_key();
    kernel::netinet::in_proto::inetsw_set::<G>(&INET_PROTOS);
    #[cfg(feature = "nfsserver")]
    kernel::net::socket::register_upcall::<G>(crate::nfs::nfs_socket::nfsrv_rcv_upcall);
    kernel::net::domain::domaininit::<G>(NET_DOMAINS);
}

/// The kernel's domain table and `bsd`'s legs of the socket layer registered without the
/// protocols' initialisations, which `bsd`'s protocol tests run themselves over each test's
/// fresh memory (`domaininit` once per process would leave them in the first test's).
#[cfg(test)]
pub(crate) fn net_tables_for_tests() {
    #[cfg(feature = "nfsserver")]
    kernel::net::socket::register_upcall::<G>(crate::nfs::nfs_socket::nfsrv_rcv_upcall);
    kernel::netinet::in_proto::inetsw_set::<G>(&INET_PROTOS);
    kernel::net::domain::domains_set::<G>(NET_DOMAINS);
}

/// An entry the C calls without a NULL test: `panic(9)` for a NULL one, as the C's call.
fn need<T>(f: Option<T>, what: &str) -> T {
    match f {
        Some(f) => f,
        None => panic(format_args!("{what}: a NULL request")),
    }
}

impl KDomain<G> for Domain {
    fn family(&self) -> i32 {
        self.dom_family
    }
    fn name(&self) -> &'static str {
        core::str::from_utf8(self.dom_name).unwrap_or("domain")
    }
    fn init(&self) {
        if let Some(init) = self.dom_init {
            init();
        }
    }
    fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
        if ptr::eq(self, &INETDOMAIN) {
            return &INET_PROTOS;
        }
        #[cfg(feature = "inet6")]
        if ptr::eq(self, &INET6DOMAIN) {
            return &INET6_PROTOS;
        }
        if ptr::eq(self, &PFKEYDOMAIN) {
            return &PFKEY_PROTOS;
        }
        &[]
    }
    fn legacy(&self) -> bool {
        true
    }
    fn sasize(&self) -> u32 {
        self.dom_sasize
    }
    fn rtoffset(&self) -> u32 {
        self.dom_rtoffset
    }
    fn maxplen(&self) -> u32 {
        self.dom_maxplen
    }
}

/// The largest socket address `bsd`'s `pr_ctlinput` functions read: a `sockaddr_in6`
/// (TCP's, UDP's and raw IPv6's, after their length check); TCP's IPv4 function reads a
/// `sockaddr_in`, 16 bytes, after its family check alone.
const CTL_SA_LEN: usize = size_of::<hal::sys::netinet6_in6::SockaddrIn6>();

/// The socket address handed to a `pr_ctlinput` function: its bytes in a zero-filled buffer
/// of [`CTL_SA_LEN`], a shorter address reading as zeros past its end (an AF_INET address
/// without its `sin_addr` is `INADDR_ANY`, which TCP's ignores), a longer one cut there
/// (its `sa_len` then matches no length the functions accept).
fn ctl_sa(sa: &[u8]) -> [u8; CTL_SA_LEN] {
    let mut b = [0u8; CTL_SA_LEN];
    let n = sa.len().min(CTL_SA_LEN);
    b[..n].copy_from_slice(&sa[..n]);
    b
}

/// Whether the ICMP message `b` holds what `bsd`'s IPv4 `pr_ctlinput` functions read of the
/// header it returns: the ICMP header (`icmp_nextmtu` in it), a returned header of at least
/// `struct ip` (`ip_hl` >= 5, the C's `icmp_input` check) whole, and 8 bytes after it (the
/// ports, TCP's sequence number, the SPI): `ICMP_ADVLEN`.
fn ctl_quote_holds(b: &[u8]) -> bool {
    let Some(&vhl) = b.get(ICMP_MINLEN) else {
        return false;
    };
    let hl = usize::from(vhl & 0x0f) << 2;
    hl >= size_of::<crate::netinet::ip::Ip>() && b.len() >= ICMP_MINLEN + hl + 8
}

impl Protocol<G> for Protosw {
    fn domain(&self) -> &'static dyn KDomain<G> {
        self.pr_domain
    }
    fn ty(&self) -> i16 {
        self.pr_type
    }
    fn protocol(&self) -> i16 {
        self.pr_protocol
    }
    fn flags(&self) -> i16 {
        self.pr_flags
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        self.pr_usrreqs.map(|u| u as &'static dyn Usrreqs<G>)
    }
    fn ctlinput(&self, cmd: i32, sa: SockaddrView<'_>, rdomain: u32, ip: Option<&[u8]>) {
        let Some(ctlinput) = self.pr_ctlinput else {
            return;
        };
        // The returned IPv4 header (the C's `&icp->icmp_ip`, ICMP_MINLEN into the message) is
        // what an AF_INET table's entry takes; an entry of another family reads a non-NULL
        // argument as its own (IPv6's as a `struct ip6ctlparam *`) and is never handed one,
        // and a quote shorter than what the functions read is not handed over (the C's
        // `icmp_input` drops such a message before `pr_ctlinput`): nothing is called.
        let arg = match ip {
            None => ptr::null_mut(),
            Some(b) if self.pr_domain.dom_family == i32::from(AF_INET) && ctl_quote_holds(b) => b
                .as_ptr()
                .wrapping_add(ICMP_MINLEN)
                .cast_mut()
                .cast::<core::ffi::c_void>(),
            Some(_) => return,
        };
        let sab = ctl_sa(sa.bytes());
        // SAFETY: `ctlinput` is one of `bsd`'s `pr_ctlinput` functions (`tcp_ctlinput`,
        // `udp_ctlinput` and `udpencap_ctlinput` through it, `ah4_ctlinput`, `esp4_ctlinput`,
        // `tcp6_ctlinput`, `udp6_ctlinput`, `rip6_ctlinput`); what they read is bounded by
        // this function's own checks above, whatever its caller passes:
        // - the address is `sab`, a local buffer of `CTL_SA_LEN` (28) initialised bytes alive
        //   until the call returns; `Sockaddr` has alignment 1. Each function reads its two
        //   header bytes, then at most a `sockaddr_in` (16 bytes: TCP's after the family
        //   alone, UDP's and the IPsec ones after `sa_len` too) or a `sockaddr_in6` (28, after
        //   its length check), unaligned reads inside the buffer; none writes through it.
        // - the argument is NULL, or, for an AF_INET table's entry only, ICMP_MINLEN into `b`,
        //   which `ip` borrows shared (unchanged) for the call and which `ctl_quote_holds`
        //   checked: the byte at ICMP_MINLEN gives a returned header of `hl` >= 20 bytes and
        //   `b` holds ICMP_MINLEN + `hl` + 8 bytes. The inet functions read the 20-byte
        //   `struct ip` there (`hl` from that same byte), 8 bytes at `hl` (the ports, TCP's
        //   sequence number, the SPI) and, before it, the ICMP header (`icmp_nextmtu`; TCP's
        //   `IcmpPkt` spans ICMP_MINLEN + `hl` + 8 bytes from the message's start): unaligned
        //   reads inside `b`, whose `as_ptr()` the pointer derives from, so its provenance
        //   covers the whole message; none writes through it. IPv6's functions get NULL.
        unsafe { ctlinput(cmd, sab.as_ptr().cast(), rdomain, arg) };
    }
    fn has_ctloutput(&self) -> bool {
        self.pr_ctloutput.is_some()
    }
    fn ctloutput(
        &self,
        op: i32,
        so: &Socket,
        _w: &mut SoLocked<'_>,
        level: i32,
        optname: i32,
        m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        match self.pr_ctloutput {
            Some(f) => f(op, as_static(so), level, optname, m.map(|m| lend(&*m))),
            None => Err(Errno::ENOPROTOOPT),
        }
    }
    fn input(
        &self,
        mp: &mut Option<MbufBox>,
        offp: &mut i32,
        proto: i32,
        af: i32,
        ns: Option<&mut kernel::net::if_var::Netstack>,
        _net: &hal::netlock::NetHeld<'_>,
    ) -> i32 {
        let input = need(self.pr_input, "pr_input");
        let mut m = mp.take().map(park);
        let nxt = input(&mut m, offp, proto, af, ns);
        // IPPROTO_DONE: the protocol consumed the packet (`bsd`'s inputs may leave the slot
        // naming the packet they kept or freed, as the C's leave `*mp`); the next protocol
        // otherwise, the packet back in the slot.
        if nxt != IPPROTO_DONE {
            *mp = m.map(unpark);
        }
        nxt
    }
    fn has_sysctl(&self) -> bool {
        self.pr_sysctl.is_some()
    }
    fn sysctl(&self, name: &[i32], req: &mut kernel::sysctl::SysctlReq<'_>) -> Result<(), Errno> {
        match self.pr_sysctl {
            Some(sysctl) => sysctl(name, req.oldp, req.oldlenp, req.newp, req.newlen),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn init(&self) {
        if let Some(init) = self.pr_init {
            init();
        }
    }
    fn fasttimo(&self) {
        if let Some(f) = self.pr_fasttimo {
            f();
        }
    }
    fn slowtimo(&self) {
        if let Some(f) = self.pr_slowtimo {
            f();
        }
    }
}

impl Usrreqs<G> for PrUsrreqs {
    fn attach(
        &self,
        so: &Socket,
        _w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        need(self.pru_attach, "pru_attach")(as_static(so), proto, wait)
    }
    fn detach(&self, so: &Socket, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        need(self.pru_detach, "pru_detach")(as_static(so))
    }
    fn bind(&self, so: &Socket, _w: &mut SoLocked<'_>, nam: &Mbuf, p: &Proc) -> Result<(), Errno> {
        match self.pru_bind {
            Some(f) => f(as_static(so), lend(nam), p),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn listen(&self, so: &Socket, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        match self.pru_listen {
            Some(f) => f(as_static(so)),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn connect(&self, so: &Socket, _w: &mut SoLocked<'_>, nam: &Mbuf) -> Result<(), Errno> {
        match self.pru_connect {
            Some(f) => f(as_static(so), lend(nam)),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn accept(&self, so: &Socket, _w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        match self.pru_accept {
            Some(f) => f(as_static(so), lend(&*nam)),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn disconnect(&self, so: &Socket, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        match self.pru_disconnect {
            Some(f) => f(as_static(so)),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn shutdown(&self, so: &Socket, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        need(self.pru_shutdown, "pru_shutdown")(as_static(so))
    }
    fn rcvd(&self, so: &Socket, _w: &mut SoLocked<'_>) {
        need(self.pru_rcvd, "pru_rcvd")(as_static(so));
    }
    fn send(
        &self,
        so: &Socket,
        _w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        need(self.pru_send, "pru_send")(
            as_static(so),
            top.map(park),
            addr.map(lend),
            control.map(park),
        )
    }
    fn abort(&self, so: &Socket, _w: &mut SoLocked<'_>) {
        need(self.pru_abort, "pru_abort")(as_static(so));
    }
    fn send_id(&self) -> usize {
        self.pru_send.map_or(0, |f| f as usize)
    }
    fn control(
        &self,
        so: &Socket,
        cmd: u64,
        data: &mut [u8],
        ifp: Option<&'static crate::net::if_var::Ifnet>,
    ) -> Result<(), Errno> {
        match self.pru_control {
            Some(f) => f(as_static(so), cmd, data, ifp),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn sense(&self, so: &Socket, _w: &mut SoLocked<'_>, ub: &mut Stat) -> Result<(), Errno> {
        match self.pru_sense {
            Some(f) => f(as_static(so), ub),
            None => Ok(()),
        }
    }
    fn rcvoob(
        &self,
        so: &Socket,
        _w: &mut SoLocked<'_>,
        m: &mut Mbuf,
        flags: i32,
    ) -> Result<(), Errno> {
        match self.pru_rcvoob {
            Some(f) => f(as_static(so), lend(&*m), flags),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
    fn sendoob(
        &self,
        so: &Socket,
        _w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        match self.pru_sendoob {
            Some(f) => f(
                as_static(so),
                top.map(park),
                addr.map(lend),
                control.map(park),
            ),
            None => {
                drop(top);
                drop(control);
                Err(Errno::EOPNOTSUPP)
            }
        }
    }
    fn sockaddr(&self, so: &Socket, _w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        need(self.pru_sockaddr, "pru_sockaddr")(as_static(so), lend(&*nam))
    }
    fn peeraddr(&self, so: &Socket, _w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        need(self.pru_peeraddr, "pru_peeraddr")(as_static(so), lend(&*nam))
    }
    fn flowid(&self, so: &Socket, _w: &mut SoLocked<'_>) -> i32 {
        need(self.pru_flowid, "pru_flowid")(as_static(so))
    }
    fn connect2(
        &self,
        so: &Socket,
        _w: &mut SoLocked<'_>,
        so2: &Socket,
        _w2: Option<&mut SoLocked<'_>>,
    ) -> Result<(), Errno> {
        match self.pru_connect2 {
            Some(f) => f(as_static(so), as_static(so2)),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
}

/// `pffinddomain`: the domain of address family `family` among `bsd`'s tables.
pub fn pffinddomain(family: i32) -> Option<&'static Domain> {
    DOMAINS.iter().copied().find(|dp| dp.dom_family == family)
}

/// `pffindtype`: the first protocol of `family` with socket type `type_`, among `bsd`'s
/// tables.
pub fn pffindtype(family: i32, type_: i32) -> Option<&'static Protosw> {
    let dp = pffinddomain(family)?;

    dp.dom_protosw
        .iter()
        .find(|pr| pr.pr_type != 0 && i32::from(pr.pr_type) == type_)
}

/// `pffindproto`: the protocol `protocol` of `family` with socket type `type_`, among
/// `bsd`'s tables; for raw sockets, the domain's raw wildcard when no protocol matches.
pub fn pffindproto(family: i32, protocol: i32, type_: i32) -> Option<&'static Protosw> {
    let mut maybe = None;

    if family == i32::from(PF_UNSPEC) {
        return None;
    }

    let dp = pffinddomain(family)?;

    for pr in dp.dom_protosw {
        if i32::from(pr.pr_protocol) == protocol && i32::from(pr.pr_type) == type_ {
            return Some(pr);
        }

        if type_ == SOCK_RAW
            && i32::from(pr.pr_type) == SOCK_RAW
            && pr.pr_protocol == 0
            && maybe.is_none()
        {
            maybe = Some(pr);
        }
    }
    maybe
}

/// `net_link_sysctl`: `net.link`.
fn net_link_sysctl(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    // All sysctl names at this level are nonterminal.
    if name.len() < 2 {
        return Err(Errno::EISDIR); // overloaded
    }
    let node = name[0];

    match node {
        NET_LINK_IFRXQ => net_ifiq_sysctl(&name[1..], oldp, Some(oldlenp), newp, newlen),
        _ => Err(Errno::ENOPROTOOPT),
    }
}

/// `net_sysctl`: the `net.*` tree, dispatched to the protocol family and the protocol.
pub fn net_sysctl(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    _p: &Proc,
) -> Result<(), Errno> {
    // All sysctl names at this level are nonterminal. Usually: next two components are
    // protocol family and protocol number, then at least one addition component.
    if name.len() < 2 {
        return Err(Errno::EISDIR); // overloaded
    }
    let family = name[0];

    if family == i32::from(PF_UNSPEC) {
        return Ok(());
    }
    if family == i32::from(PF_LINK) {
        return net_link_sysctl(&name[1..], oldp, oldlenp, newp, newlen);
    }
    if family == i32::from(PF_UNIX) {
        let mut req = kernel::sysctl::SysctlReq {
            oldp,
            oldlenp,
            newp,
            newlen,
        };
        return kernel::net::unix::uipc_sysctl::<G>(&name[1..], &mut req);
    }
    if family == i32::from(PF_BPF) {
        return bpf_sysctl(&name[1..], oldp, oldlenp, newp, newlen);
    }
    if family == i32::from(PF_PFLOW) {
        return crate::net::if_pflow::pflow_sysctl(&name[1..], oldp, oldlenp, newp, newlen);
    }
    // PIPEX (PF_PIPEX), MPLS (PF_MPLS): not configured.
    // The kernel's domains[]: the route and internet domains are the kernel's (routesw's
    // sysctl_rtable, the IP, ICMP and IGMP entries), bsd's rows answer through their
    // Protocol impl.
    let Some(dp) = kernel::net::domain::pffinddomain::<G>(family) else {
        return Err(Errno::ENOPROTOOPT);
    };

    if name.len() < 3 {
        return Err(Errno::EISDIR); // overloaded
    }
    let protocol = name[1];
    for pr in dp.protocols() {
        if i32::from(pr.protocol()) == protocol && pr.has_sysctl() {
            let savelen = *oldlenp;
            // sysctl_vsunlock(oldp, savelen) is the guard's drop.
            let vs = if pr.flags() & PR_MPSYSCTL == 0 {
                Some(sysctl_vslock(oldp, savelen)?)
            } else {
                None
            };
            let mut req = kernel::sysctl::SysctlReq {
                oldp,
                oldlenp,
                newp,
                newlen,
            };
            let error = pr.sysctl(&name[2..], &mut req);
            drop(vs);

            return error;
        }
    }
    Err(Errno::ENOPROTOOPT)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::netinet::in_::{IPPROTO_ICMP, IPPROTO_RAW, IPPROTO_TCP, IPPROTO_UDP};
    use crate::sys::protosw::PRC_MSGSIZE;
    use crate::sys::socket::{AF_ROUTE, AF_UNIX, SOCK_DGRAM, SOCK_STREAM};

    #[test]
    fn ctlinput_s_address_is_padded_and_its_quote_checked_here() {
        // The address: a 2-byte view padded with zeros, a 40-byte one cut at 28.
        let mut short = [0u8; CTL_SA_LEN];
        short[..2].copy_from_slice(&[2, AF_INET]);
        assert_eq!(ctl_sa(&[2, AF_INET]), short);
        let long: [u8; 40] = core::array::from_fn(|i| i as u8);
        assert_eq!(ctl_sa(&long)[..], long[..CTL_SA_LEN]);
        // The quote: ICMP_MINLEN + ip_hl * 4 + 8 bytes, ip_hl >= 5, or nothing is handed over.
        let msg = |vhl: u8, len: usize| {
            let mut b = std::vec![0u8; len];
            if let Some(x) = b.get_mut(ICMP_MINLEN) {
                *x = vhl;
            }
            b
        };
        for (vhl, len, holds) in [
            (0x45, 36, true),
            (0x45, 35, false),
            (0x46, 40, true),
            (0x46, 39, false),
            (0x4f, 76, true),
            (0x4f, 75, false),
            (0x4f, 28, false),
            (0x44, 64, false),
            (0x40, 64, false),
            (0x45, ICMP_MINLEN, false),
            (0x45, 0, false),
        ] {
            assert_eq!(
                ctl_quote_holds(&msg(vhl, len)),
                holds,
                "vhl {vhl:#x}, len {len}"
            );
        }
    }

    #[test]
    fn a_short_address_and_an_overlong_quote_reach_tcp_ctlinput_safely() {
        // INETSW's TCP and UDP rows called as the kernel calls them, with what safe code can
        // build: a 2-byte AF_INET address (TCP's reads 16 bytes after its family check) and a
        // 28-byte message whose returned header says 60 bytes (TCP's would read at 8 + 60).
        let sa = SockaddrView::new(&[2, AF_INET]).expect("a 2-byte address");
        let mut quote = [0u8; 28];
        quote[ICMP_MINLEN] = 0x4f;
        for (proto, ty) in [(IPPROTO_TCP, SOCK_STREAM), (IPPROTO_UDP, SOCK_DGRAM)] {
            let pr = pffindproto(i32::from(AF_INET), proto, ty).expect("the row");
            Protocol::<G>::ctlinput(pr, PRC_MSGSIZE, sa, 0, Some(&quote));
            Protocol::<G>::ctlinput(pr, PRC_MSGSIZE, sa, 0, None);
        }
    }

    #[test]
    fn the_inet_protocols_are_found_by_number_and_type() {
        assert!(pffinddomain(i32::from(AF_INET)).is_some());
        // The UNIX and route domains are the kernel's (N6.2a, N6.3b2): not among bsd's
        // tables.
        assert!(pffinddomain(i32::from(AF_ROUTE)).is_none());
        assert!(pffinddomain(i32::from(AF_UNIX)).is_none());
        assert!(pffinddomain(i32::from(PF_UNSPEC)).is_none());

        let icmp = pffindproto(i32::from(AF_INET), IPPROTO_ICMP, SOCK_RAW).expect("icmp");
        assert_eq!(i32::from(icmp.pr_protocol), IPPROTO_ICMP);
        let udp = pffindproto(i32::from(AF_INET), IPPROTO_UDP, SOCK_DGRAM).expect("udp");
        assert_eq!(i32::from(udp.pr_protocol), IPPROTO_UDP);
        // An unknown raw protocol gets the raw wildcard; IPPROTO_RAW has its own entry.
        let wild = pffindproto(i32::from(AF_INET), 200, SOCK_RAW).expect("wildcard");
        assert_eq!(wild.pr_protocol, 0);
        let raw = pffindproto(i32::from(AF_INET), IPPROTO_RAW, SOCK_RAW).expect("raw");
        assert_eq!(i32::from(raw.pr_protocol), IPPROTO_RAW);
        assert!(pffindtype(i32::from(AF_INET), SOCK_DGRAM).is_some());
    }
}
/* </TESTS> */
