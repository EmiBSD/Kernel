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
//! `panic(9)` when NULL, as the C's call through NULL. A raw row (`pr_rip`: the IP-in-IP,
//! IPsec and pfsync rows, whose C entries name `rip_usrreqs` and `rip_ctloutput` or the IPv6
//! pair) answers `usrreqs`, `legacy` and `ctloutput` with the kernel's raw IP of its family
//! (N6.6b1, D-2: its sockets are the kernel's, held by a witness). [`NET_DOMAINS`] is the kernel's
//! `domains[]` (the UNIX domain is the kernel's own), which [`domaininit`] hands over.
//!
//! ## Deviations
//! - `domains[]` is a slice without the C's NULL terminator. It holds `inetdomain` and
//!   `inet6domain` (the kernel's `InetDomain` and `Inet6Domain`, their tables `inetsw[]` and
//!   `inet6sw[]` assembled here of the kernel's entries and `bsd`'s rows of
//!   `netinet/in_proto.rs` and `netinet6/in6_proto.rs`), `unixdomain` and `routedomain` (the
//!   kernel's), with `pfkeydomain` (`net/pfkeyv2.rs`, `IPSEC`) first and `inet6domain`
//!   (`INET6`: the `inet6` feature) before `inetdomain`. `MPLS` and `NAF_FRAME` are not configured; their entries are comments,
//!   and so are the `PIPEX` branches of `net_sysctl`. `NPFLOW` is configured: `PF_PFLOW`
//!   goes to `pflow_sysctl`; so is `NBPFILTER`: `PF_BPF` goes to `bpf_sysctl`.
//! - `pffinddomain`, `pffindtype`, `pffindproto` and `pfctlinput` are the kernel's
//!   (`kernel::net::domain`, over its `domains[]`; `bsd`'s finders over its own tables went
//!   with their last caller, `bsd`'s `ip6_init`, at N6.5d): these tables' `pr_ctlinput` is
//!   reached through their `Protocol` impl, the one raw call of the table's function.
//! - `net_sysctl` follows `kern_sysctl.rs`'s calling convention (`docs/C_TO_RUST.md`);
//!   `sysctl_vslock` wraps the protocol handlers that are not `PR_MPSYSCTL`, as in C.

use core::ptr;

use kernel::net::domain::Domain as KDomain;
use kernel::net::mbuf::MbufBox;
use kernel::net::proto::{CtlArg, Protocol, SockaddrView, Usrreqs};
use kernel::net::socket::SoLocked;
use kernel::netinet::in_proto::{
    DivertProtocol, IcmpProtocol, IgmpProtocol, InetDomain, IpProtocol, RipProtocol, TcpProtocol,
    UdpProtocol,
};
#[cfg(feature = "inet6")]
use kernel::netinet6::in6_proto::{
    Dest6Protocol, Divert6Protocol, Frag6Protocol, Icmp6Protocol, Inet6Domain, Ip6Protocol,
    Rip6Protocol, Route6Protocol, Tcp6Protocol, Udp6Protocol,
};

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
    AF_INET, AF_INET6, NET_LINK_IFRXQ, PF_BPF, PF_LINK, PF_PFLOW, PF_UNIX, PF_UNSPEC,
};
use crate::sys::socketvar::{Socket, as_static};
use crate::sys::stat::Stat;
use sysdefs::netinet::in_::IPPROTO_DONE;
use sysdefs::netinet::ip_icmp::ICMP_MINLEN;

/// `domains[]` as the kernel's socket layer sees it, in the C's order: `bsd`'s pfkey table
/// through the traits, and the kernel's IPv6, internet, UNIX and route domains.
pub static NET_DOMAINS: &[&dyn KDomain<G>] = &[
    // MPLS: &mplsdomain, not configured.
    &PFKEYDOMAIN,
    #[cfg(feature = "inet6")]
    &Inet6Domain::<G>::NEW,
    &InetDomain::<G>::NEW,
    &kernel::net::unix::Unix::<G>::NEW,
    &kernel::net::rtsock::RouteDomain::<G>::NEW,
    // NAF_FRAME: &framedomain, not configured.
];

/// The entries of a protocol table, as the kernel's protocols.
macro_rules! protocols {
    ($sw:ident; $($i:literal)*) => { [$(&$sw[$i] as &dyn Protocol<G>),*] };
}

/// `inetsw[]` as the kernel's protocols, in the C's order: the kernel's IP, UDP, TCP, raw IP,
/// ICMP, IGMP, divert and raw wildcard entries (`kernel::netinet::in_proto`), `bsd`'s rows for
/// the rest (IP-in-IP, IPsec, pfsync: `pr_rip`).
#[cfg(not(feature = "inet6"))]
pub(crate) static INET_PROTOS: [&dyn Protocol<G>; 13] = [
    &IpProtocol::<G>::NEW,
    &UdpProtocol::<G>::NEW,
    &TcpProtocol::<G>::NEW,
    &RipProtocol::<G>::RAW,
    &IcmpProtocol::<G>::NEW,
    &INETSW[0],
    &IgmpProtocol::<G>::NEW,
    &INETSW[1],
    &INETSW[2],
    &INETSW[3],
    &INETSW[4],
    &DivertProtocol::<G>::NEW,
    &RipProtocol::<G>::WILD,
];
/// `inetsw[]` as the kernel's protocols, in the C's order: the kernel's IP, UDP, TCP, raw IP,
/// ICMP, IGMP, divert and raw wildcard entries (`kernel::netinet::in_proto`), `bsd`'s rows for
/// the rest (IP-in-IP, IPsec, pfsync: `pr_rip`).
#[cfg(feature = "inet6")]
pub(crate) static INET_PROTOS: [&dyn Protocol<G>; 14] = [
    &IpProtocol::<G>::NEW,
    &UdpProtocol::<G>::NEW,
    &TcpProtocol::<G>::NEW,
    &RipProtocol::<G>::RAW,
    &IcmpProtocol::<G>::NEW,
    &INETSW[0],
    &INETSW[1],
    &IgmpProtocol::<G>::NEW,
    &INETSW[2],
    &INETSW[3],
    &INETSW[4],
    &INETSW[5],
    &DivertProtocol::<G>::NEW,
    &RipProtocol::<G>::WILD,
];

/// `inet6sw[]` as the kernel's protocols, in the C's order: the kernel's IPv6, UDP, TCP, raw
/// IPv6, ICMPv6, destination options, routing header, fragment header, divert and raw wildcard
/// entries (`kernel::netinet6::in6_proto`), `bsd`'s rows for the rest (IPsec, IP-in-IP:
/// `pr_rip`).
#[cfg(feature = "inet6")]
pub(crate) static INET6_PROTOS: [&dyn Protocol<G>; 15] = [
    &Ip6Protocol::<G>::NEW,
    &Udp6Protocol::<G>::NEW,
    &Tcp6Protocol::<G>::NEW,
    &Rip6Protocol::<G>::RAW,
    &Icmp6Protocol::<G>::NEW,
    &Dest6Protocol::<G>::NEW,
    &Route6Protocol::<G>::NEW,
    &Frag6Protocol::<G>::NEW,
    &INET6SW[0],
    &INET6SW[1],
    &INET6SW[2],
    &INET6SW[3],
    &INET6SW[4],
    &Divert6Protocol::<G>::NEW,
    &Rip6Protocol::<G>::WILD,
];
/// `pfkeysw[]` as the kernel's protocols.
static PFKEY_PROTOS: [&dyn Protocol<G>; 1] = protocols!(PFKEYSW; 0);

/// `domaininit`: the kernel's domains (`bsd`'s tables and the UNIX domain) and their
/// protocols initialised, the protocol timeouts started (`kernel::net::domain`).
pub fn domaininit() {
    // in_init's half that is bsd's: the key of bsd's pcb counts (the kernel's InetDomain
    // runs the kernel's in_init).
    let _ = crate::netinet::in_pcb::legacy_key();
    kernel::netinet::in_proto::inetsw_set::<G>(&INET_PROTOS);
    // ip6_init's half that is bsd's: the fragment identifiers' generator (idgen32, bsd's
    // crypto; the kernel's Ip6Protocol runs the kernel's ip6_init, its switch first).
    #[cfg(feature = "inet6")]
    {
        crate::netinet6::ip6_output::ip6_randomid_init();
        kernel::netinet6::in6_proto::inet6sw_set::<G>(&INET6_PROTOS);
    }
    kernel::net::domain::domaininit::<G>(NET_DOMAINS);
}

/// The kernel's domain table and `bsd`'s legs of the socket layer registered without the
/// protocols' initialisations, which `bsd`'s protocol tests run themselves over each test's
/// fresh memory (`domaininit` once per process would leave them in the first test's).
#[cfg(test)]
pub(crate) fn net_tables_for_tests() {
    kernel::netinet::in_proto::inetsw_set::<G>(&INET_PROTOS);
    #[cfg(feature = "inet6")]
    kernel::netinet6::in6_proto::inet6sw_set::<G>(&INET6_PROTOS);
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
const CTL_SA_LEN: usize = size_of::<sysdefs::netinet6::in6::SockaddrIn6>();

/// The socket address handed to a `pr_ctlinput` function: its bytes (`sa_len` of them, at
/// least the header's two: `SockaddrView`'s) in a zero-filled buffer of [`CTL_SA_LEN`], a
/// shorter address reading as zeros past its end (an AF_INET address without its `sin_addr`
/// is `INADDR_ANY`, which TCP's ignores); `None` for a longer one, which no C caller passes
/// (the C's are a `sockaddr_in` or a `sockaddr_in6`).
fn ctl_sa(sa: &[u8]) -> Option<[u8; CTL_SA_LEN]> {
    if sa.len() > CTL_SA_LEN {
        return None;
    }
    let mut b = [0u8; CTL_SA_LEN];
    b[..sa.len()].copy_from_slice(sa);
    Some(b)
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

/// The raw IP user requests of family `family` (`rip_usrreqs`, `rip6_usrreqs`): the kernel's,
/// served to `bsd`'s raw rows (`pr_rip`); `None` for another family, which no such row has.
fn rip_usrreqs(family: i32) -> Option<&'static dyn Usrreqs<G>> {
    match family {
        f if f == i32::from(AF_INET) => Some(&kernel::netinet::raw_ip::RipUsrreqs::<G>::NEW),
        #[cfg(feature = "inet6")]
        f if f == i32::from(AF_INET6) => Some(&kernel::netinet6::raw_ip6::Rip6Usrreqs::<G>::NEW),
        _ => None,
    }
}

/// The raw IP socket options of family `family` (`rip_ctloutput`, `rip6_ctloutput`): the
/// kernel's, for `bsd`'s raw rows (`pr_rip`); `ENOPROTOOPT` for another family, which no such
/// row has.
fn rip_ctloutput(
    family: i32,
    op: i32,
    so: &Socket,
    w: &mut SoLocked<'_>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    match family {
        f if f == i32::from(AF_INET) => {
            kernel::netinet::raw_ip::rip_ctloutput(op, so, w, level, optname, m)
        }
        #[cfg(feature = "inet6")]
        f if f == i32::from(AF_INET6) => {
            kernel::netinet6::raw_ip6::rip6_ctloutput(op, so, w, level, optname, m)
        }
        _ => Err(Errno::ENOPROTOOPT),
    }
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
    /// The table's `pr_usrreqs`; a raw row's (`pr_rip`) are the kernel's raw IP requests of
    /// the row's family (`rip_usrreqs`, `rip6_usrreqs`).
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        if self.pr_rip {
            return rip_usrreqs(self.pr_domain.dom_family);
        }
        self.pr_usrreqs.map(|u| u as &'static dyn Usrreqs<G>)
    }
    /// `bsd`'s protocols release and take the socket lock again C-style: their sockets are
    /// held by a detached holding of `so_lock`. A raw row's (`pr_rip`) are the kernel's raw
    /// sockets, held by a witness.
    fn legacy(&self) -> bool {
        !self.pr_rip
    }
    fn ctlinput(
        &self,
        cmd: i32,
        sa: SockaddrView<'_>,
        rdomain: u32,
        arg: CtlArg<'_>,
        _net: &hal::sync::netlock::NetHeld<'_>,
    ) {
        let Some(ctlinput) = self.pr_ctlinput else {
            return;
        };
        // Each family's entry is handed its own argument only, as the C's callers pass it: the
        // returned IPv4 header (the C's `&icp->icmp_ip`, ICMP_MINLEN into the message) to an
        // AF_INET table's entry, a quote shorter than what the functions read not handed over
        // (the C's `icmp_input` drops such a message before `pr_ctlinput`); the ICMPv6 error's
        // parameter to an AF_INET6 entry. An argument of the other family (each reads a
        // non-NULL argument as its own) calls nothing.
        let family = self.pr_domain.dom_family;
        let arg = match arg {
            CtlArg::None => ptr::null_mut(),
            CtlArg::Ip(b) if family == i32::from(AF_INET) && ctl_quote_holds(b) => b
                .as_ptr()
                .wrapping_add(ICMP_MINLEN)
                .cast_mut()
                .cast::<core::ffi::c_void>(),
            CtlArg::Ip6(p) if family == i32::from(AF_INET6) => {
                ptr::from_ref(p).cast_mut().cast::<core::ffi::c_void>()
            }
            CtlArg::Ip(_) | CtlArg::Ip6(_) => return,
        };
        // An address longer than a sockaddr_in6: no C caller passes one, nothing is called.
        let Some(sab) = ctl_sa(sa.bytes()) else {
            return;
        };
        // SAFETY: `PrCtlinputFn`'s contract (`sys/sys/protosw.rs`), met by this function's own
        // checks above, whatever its caller passes:
        // - `sa` is `sab`, a local buffer of `CTL_SA_LEN` (a `sockaddr_in6`'s 28) initialised
        //   bytes, alive until the call returns (`Sockaddr` has alignment 1). It holds the
        //   view's bytes, `sa_len` of them (`SockaddrView` holds its `sa_len` bytes, and one
        //   longer than 28 returned above), so its `sa_len` is at most the bytes lent, and a
        //   `sockaddr_in6`'s bytes are readable there.
        // - `arg` is NULL, or, for an AF_INET table's entry only, ICMP_MINLEN into the message
        //   `b`, which `ctl_quote_holds` checked holds the ICMP header, a returned header of
        //   `hl` >= 20 bytes (the byte at ICMP_MINLEN) and 8 bytes after it: ICMP_ADVLEN. `b`
        //   is borrowed shared and unchanged for the call; the contract's functions only read
        //   it, so the `cast_mut` lends no write; the pointer derives from `b.as_ptr()`, so
        //   its provenance covers the message's header before it too. For an AF_INET6 table's
        //   entry only, `arg` is the `Ip6CtlParam` the kernel lent (`CtlArg::Ip6`'s reference,
        //   alive for this call): the contract's AF_INET6 clause, its functions reading it
        //   through `&` only (the `cast_mut` lends no write; the final destination they change
        //   is a `Cell`). An entry is never handed the other family's argument (above).
        unsafe { ctlinput(cmd, sab.as_ptr().cast(), rdomain, arg) };
    }
    fn has_ctloutput(&self) -> bool {
        self.pr_rip || self.pr_ctloutput.is_some()
    }
    /// The table's `pr_ctloutput`; a raw row's (`pr_rip`) is the kernel's `rip_ctloutput` or
    /// `rip6_ctloutput`, by the row's family, with the socket's witness.
    fn ctloutput(
        &self,
        op: i32,
        so: &Socket,
        w: &mut SoLocked<'_>,
        level: i32,
        optname: i32,
        m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        if self.pr_rip {
            return rip_ctloutput(self.pr_domain.dom_family, op, so, w, level, optname, m);
        }
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
        _net: &hal::sync::netlock::NetHeld<'_>,
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
    fn sysctl(
        &self,
        name: &[i32],
        req: &mut kernel::base::sysctl::SysctlReq<'_>,
    ) -> Result<(), Errno> {
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
        let mut req = kernel::base::sysctl::SysctlReq {
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
            let mut req = kernel::base::sysctl::SysctlReq {
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
    use crate::netinet::in_::{IPPROTO_ICMP, IPPROTO_RAW, IPPROTO_UDP};
    use crate::sys::protosw::PRC_MSGSIZE;
    use crate::sys::socket::{AF_ROUTE, AF_UNIX, SOCK_DGRAM, SOCK_RAW};

    #[test]
    fn ctlinput_s_address_is_padded_and_its_quote_checked_here() {
        // The address: a 2-byte view padded with zeros, a sockaddr_in6 whole, a longer one
        // refused.
        let mut short = [0u8; CTL_SA_LEN];
        short[..2].copy_from_slice(&[2, AF_INET]);
        assert_eq!(ctl_sa(&[2, AF_INET]), Some(short));
        let six: [u8; CTL_SA_LEN] = core::array::from_fn(|i| i as u8);
        assert_eq!(ctl_sa(&six), Some(six));
        let long: [u8; 40] = core::array::from_fn(|i| i as u8);
        assert_eq!(ctl_sa(&long), None);
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

    /// The calls [`probe`] took, and how many had an argument.
    static PROBED: [core::sync::atomic::AtomicU32; 2] = [
        core::sync::atomic::AtomicU32::new(0),
        core::sync::atomic::AtomicU32::new(0),
    ];

    /// The address of the last non-NULL argument the probe was handed.
    static PROBED_ARG: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

    /// A `pr_ctlinput` that only counts its calls (it reads nothing through its pointers).
    fn probe(
        _cmd: i32,
        _sa: *const crate::sys::socket::Sockaddr,
        _rd: u32,
        arg: *mut core::ffi::c_void,
    ) {
        use core::sync::atomic::Ordering::Relaxed;
        PROBED[0].fetch_add(1, Relaxed);
        if !arg.is_null() {
            PROBED[1].fetch_add(1, Relaxed);
            PROBED_ARG.store(arg.addr(), Relaxed);
        }
    }

    #[test]
    fn ctlinput_calls_its_function_only_with_what_it_checked() {
        use core::sync::atomic::Ordering::Relaxed;
        let calls = || (PROBED[0].load(Relaxed), PROBED[1].load(Relaxed));
        let inet = Protosw {
            pr_type: SOCK_RAW as i16,
            pr_protocol: 99,
            pr_ctlinput: Some(probe as crate::sys::protosw::PrCtlinputFn),
            ..Protosw::new(&INETDOMAIN)
        };
        let ctl = |pr: &Protosw, sa: &[u8], ip: Option<&[u8]>| {
            let sa = SockaddrView::new(sa).expect("an address");
            let arg = ip.map_or(CtlArg::None, CtlArg::Ip);
            let w = hal::sync::netlock::NETLOCK.read();
            let net = hal::sync::netlock::NetHeld::of(&w).expect("the net lock");
            Protocol::<G>::ctlinput(pr, PRC_MSGSIZE, sa, 0, arg, &net);
        };
        let mut sin = [0u8; 16];
        sin[..2].copy_from_slice(&[16, AF_INET]);
        let mut quote = [0u8; 36];
        quote[ICMP_MINLEN] = 0x45;
        // A sockaddr_in and a quote of ICMP_ADVLEN bytes: called, with the argument.
        ctl(&inet, &sin, Some(&quote));
        assert_eq!(calls(), (1, 1));
        // A quote whose ip_hl says 60 bytes in 28 (the C's icmp_input drops it): not called.
        let mut overlong = [0u8; 28];
        overlong[ICMP_MINLEN] = 0x4f;
        ctl(&inet, &sin, Some(&overlong));
        assert_eq!(calls(), (1, 1));
        // An address longer than a sockaddr_in6: not called.
        let mut long = [0u8; 40];
        long[..2].copy_from_slice(&[40, AF_INET]);
        ctl(&inet, &long, None);
        assert_eq!(calls(), (1, 1));
        // A 2-byte address (padded with zeros), no quote: called, without an argument.
        ctl(&inet, &[2, AF_INET], None);
        assert_eq!(calls(), (2, 1));
        // An IPv6 table's entry is never handed an IPv4 quote.
        #[cfg(feature = "inet6")]
        {
            let inet6 = Protosw {
                pr_domain: &INET6DOMAIN,
                ..inet
            };
            ctl(&inet6, &sin, Some(&quote));
            assert_eq!(calls(), (2, 1));
            ctl(&inet6, &sin, None);
            assert_eq!(calls(), (3, 1));

            // The ICMPv6 error's parameter: handed to an IPv6 entry, the kernel's own value
            // (its address), never to an IPv4 one.
            let _g = crate::net::if_::tests::setup_net();
            let m = crate::net::if_::tests::test_packet(&[0u8; 48]);
            let p = kernel::netinet6::icmp6::Ip6CtlParam {
                m,
                icmp6: sysdefs::netinet::icmp6::Icmp6Hdr::zeroed(),
                ip6: sysdefs::netinet::ip6::Ip6Hdr::default(),
                off: 48,
                src: sysdefs::netinet6::in6::SockaddrIn6::default(),
                finaldst: core::cell::Cell::new(sysdefs::netinet6::in6::In6Addr::default()),
                cmdarg: Some(1280),
                nxt: 17,
            };
            let mut sin6 = [0u8; 28];
            sin6[..2].copy_from_slice(&[28, crate::sys::socket::AF_INET6]);
            let ctl6 = |pr: &Protosw| {
                let sa = SockaddrView::new(&sin6).expect("an address");
                let w = hal::sync::netlock::NETLOCK.read();
                let net = hal::sync::netlock::NetHeld::of(&w).expect("the net lock");
                Protocol::<G>::ctlinput(pr, PRC_MSGSIZE, sa, 0, CtlArg::Ip6(&p), &net);
            };
            ctl6(&inet6);
            assert_eq!(calls(), (4, 2));
            assert_eq!(
                PROBED_ARG.load(Relaxed),
                core::ptr::from_ref(&p).addr(),
                "the kernel's parameter itself"
            );
            ctl6(&inet);
            assert_eq!(calls(), (4, 2), "an IPv4 entry is never handed it");
            crate::kern::uipc_mbuf::m_freem(m);
        }
    }

    #[test]
    fn the_inet_protocols_are_found_by_number_and_type() {
        use kernel::net::domain::{pffinddomain, pffindproto, pffindtype};
        net_tables_for_tests();
        let f = i32::from(AF_INET);
        assert!(pffinddomain::<G>(f).is_some_and(|d| d.name() == "inet"));
        // The UNIX and route domains are the kernel's (N6.2a, N6.3b2), and so is the internet
        // one (N6.4c2): all are in the kernel's domains[].
        assert!(pffinddomain::<G>(i32::from(AF_ROUTE)).is_some());
        assert!(pffinddomain::<G>(i32::from(AF_UNIX)).is_some());
        assert!(pffinddomain::<G>(i32::from(PF_UNSPEC)).is_none());

        // ICMP's entry is the kernel's; bsd's rows are its INETSW itself.
        let icmp = pffindproto::<G>(f, IPPROTO_ICMP, SOCK_RAW).expect("icmp");
        assert_eq!(i32::from(icmp.protocol()), IPPROTO_ICMP);
        assert!(
            INETSW.iter().all(|r| !ptr::addr_eq(icmp, r)),
            "the kernel's"
        );
        // UDP's entry is the kernel's (N6.6b0): its sockets held by a witness of so_lock.
        let udp = pffindproto::<G>(f, IPPROTO_UDP, SOCK_DGRAM).expect("UDP");
        assert!(INETSW.iter().all(|r| !ptr::addr_eq(udp, r)), "the kernel's");
        assert!(!udp.legacy() && udp.usrreqs().is_some() && udp.has_ctloutput());
        assert!(udp.has_sysctl());
        // An unknown raw protocol gets the raw wildcard; IPPROTO_RAW has its own entry: both
        // the kernel's (N6.6b1), held by a witness.
        let wild = pffindproto::<G>(f, 200, SOCK_RAW).expect("the raw wildcard");
        assert_eq!((wild.ty(), wild.protocol()), (SOCK_RAW as i16, 0));
        let raw = pffindproto::<G>(f, IPPROTO_RAW, SOCK_RAW).expect("raw IP");
        assert_eq!(i32::from(raw.protocol()), IPPROTO_RAW);
        for pr in [wild, raw, icmp] {
            assert!(INETSW.iter().all(|r| !ptr::addr_eq(pr, r)), "the kernel's");
            assert!(!pr.legacy() && pr.usrreqs().is_some() && pr.has_ctloutput());
        }
        // So is divert's (N6.6b1); bsd's rows left (IPsec, IP-in-IP, pfsync) are raw sockets
        // served by the kernel's raw requests and options (pr_rip): no longer legacy.
        let div =
            pffindproto::<G>(f, crate::netinet::in_::IPPROTO_DIVERT, SOCK_RAW).expect("divert");
        assert!(INETSW.iter().all(|r| !ptr::addr_eq(div, r)), "the kernel's");
        assert!(!div.legacy() && div.has_sysctl());
        let esp = pffindproto::<G>(f, crate::netinet::in_::IPPROTO_ESP, SOCK_RAW).expect("ESP");
        assert!(INETSW.iter().any(|r| ptr::addr_eq(esp, r)), "bsd's row");
        assert!(!esp.legacy() && esp.usrreqs().is_some() && esp.has_ctloutput());
        assert!(esp.has_sysctl());
        assert!(INETSW.iter().all(|r| r.pr_rip && r.pr_usrreqs.is_none()));
        let dgram = pffindtype::<G>(f, SOCK_DGRAM).expect("a datagram protocol");
        assert!(ptr::addr_eq(dgram, udp), "UDP, the first SOCK_DGRAM entry");
        // TCP's entry is the kernel's (N6.6a5): its sockets held by a witness of so_lock.
        let tcp = pffindtype::<G>(f, crate::sys::socket::SOCK_STREAM).expect("TCP");
        assert_eq!(i32::from(tcp.protocol()), crate::netinet::in_::IPPROTO_TCP);
        assert!(INETSW.iter().all(|r| !ptr::addr_eq(tcp, r)), "the kernel's");
        assert!(!tcp.legacy() && tcp.usrreqs().is_some() && tcp.has_ctloutput());
    }

    /// The IPv6 domain the kernel's socket layer walks: the kernel's `Inet6Domain`, its table
    /// the kernel's ten entries among `bsd`'s five rows in the C's order, found by the
    /// kernel's `pffindproto`/`pffindtype` (`bsd`'s rows are its `INET6SW` itself, raw sockets
    /// of the kernel's raw IPv6 requests), and `ip6_init`'s `ip6_protox[]`
    /// over it.
    #[cfg(feature = "inet6")]
    #[test]
    fn the_inet6_table_is_the_kernel_s_entries_among_bsd_s_rows() {
        use crate::netinet::in_::{
            IPPROTO_DIVERT, IPPROTO_DSTOPTS, IPPROTO_FRAGMENT, IPPROTO_ICMPV6, IPPROTO_IPV6,
            IPPROTO_ROUTING,
        };
        use crate::sys::socket::SOCK_STREAM;
        use kernel::net::domain::{
            pffinddomain as kfinddomain, pffindproto as kfindproto, pffindtype as kfindtype,
        };
        use kernel::net::proto::{PR_ADDR, PR_ATOMIC};
        use kernel::netinet6::in6_proto::{inet6_protocol, inet6sw_init};

        let (_g, _t, _p) = crate::netinet::in_pcb::tests::setup();
        let f = i32::from(AF_INET6);
        let dom = kfinddomain::<G>(f).expect("the IPv6 domain");
        assert_eq!(
            (dom.name(), dom.sasize(), dom.rtoffset(), dom.maxplen()),
            ("inet6", 28, 8, 128)
        );
        // Every entry sockets can be made of is served by the kernel's requests (N6.6b1: TCP's,
        // UDP's, raw IPv6's and divert's own, ICMPv6's and bsd's raw rows raw IPv6's): held by
        // a witness of `so_lock`, none legacy.
        assert!(
            dom.protocols()
                .iter()
                .filter(|p| p.usrreqs().is_some())
                .all(|p| !p.legacy())
        );
        assert_eq!(
            dom.protocols()
                .iter()
                .filter(|p| p.usrreqs().is_some())
                .count(),
            11,
            "all but IPv6's and the three extension headers'"
        );
        // (type, protocol) of each entry, in6_proto.c's order.
        let rows: std::vec::Vec<(i32, i32)> = dom
            .protocols()
            .iter()
            .map(|p| (i32::from(p.ty()), i32::from(p.protocol())))
            .collect();
        assert_eq!(
            rows,
            [
                (0, IPPROTO_IPV6),
                (SOCK_DGRAM, IPPROTO_UDP),
                (SOCK_STREAM, crate::netinet::in_::IPPROTO_TCP),
                (SOCK_RAW, IPPROTO_RAW),
                (SOCK_RAW, IPPROTO_ICMPV6),
                (SOCK_RAW, IPPROTO_DSTOPTS),
                (SOCK_RAW, IPPROTO_ROUTING),
                (SOCK_RAW, IPPROTO_FRAGMENT),
                (SOCK_RAW, crate::netinet::in_::IPPROTO_AH),
                (SOCK_RAW, crate::netinet::in_::IPPROTO_ESP),
                (SOCK_RAW, crate::netinet::in_::IPPROTO_IPCOMP),
                (SOCK_RAW, crate::netinet::in_::IPPROTO_IPV4),
                (SOCK_RAW, IPPROTO_IPV6),
                (SOCK_RAW, IPPROTO_DIVERT),
                (SOCK_RAW, 0),
            ]
        );

        // bsd's rows are its table's own.
        let is = |a: Option<&'static dyn Protocol<G>>, b: &'static Protosw| {
            a.is_some_and(|a| ptr::addr_eq(a, b))
        };
        let udp6 = kfindproto::<G>(f, IPPROTO_UDP, SOCK_DGRAM).expect("UDP");
        assert!(
            INET6SW.iter().all(|r| !ptr::addr_eq(udp6, r)),
            "the kernel's"
        );
        assert!(!udp6.legacy() && udp6.domain().family() == f && udp6.has_sysctl());
        let tcp6 = kfindtype::<G>(f, SOCK_STREAM).expect("TCP");
        assert!(
            INET6SW.iter().all(|r| !ptr::addr_eq(tcp6, r)),
            "the kernel's"
        );
        assert!(!tcp6.legacy() && tcp6.domain().family() == f);
        // Raw IPv6, divert and the raw wildcard are the kernel's (N6.6b1).
        let raw6 = kfindproto::<G>(f, IPPROTO_RAW, SOCK_RAW).expect("raw IPv6");
        let div6 = kfindproto::<G>(f, IPPROTO_DIVERT, SOCK_RAW).expect("divert6");
        let wild6 = kfindproto::<G>(f, 200, SOCK_RAW).expect("the raw wildcard");
        assert_eq!((wild6.ty(), wild6.protocol()), (SOCK_RAW as i16, 0));
        for pr in [raw6, div6, wild6] {
            assert!(INET6SW.iter().all(|r| !ptr::addr_eq(pr, r)), "the kernel's");
            assert!(!pr.legacy() && pr.has_ctloutput());
        }
        assert!(raw6.has_sysctl() && !div6.has_sysctl() && !wild6.has_sysctl());
        // bsd's rows: raw sockets of the kernel's raw IPv6 requests (pr_rip).
        let esp6 = kfindproto::<G>(f, crate::netinet::in_::IPPROTO_ESP, SOCK_RAW).expect("ESP");
        assert!(is(Some(esp6), &INET6SW[1]));
        assert!(!esp6.legacy() && esp6.usrreqs().is_some() && esp6.has_ctloutput());
        // ICMPv6's entry is the kernel's, the raw sockets' requests and options its own.
        let icmp6 = kfindproto::<G>(f, IPPROTO_ICMPV6, SOCK_RAW).expect("ICMPv6");
        assert!(INET6SW.iter().all(|r| !ptr::addr_eq(icmp6, r)));
        assert_eq!(icmp6.flags(), PR_ATOMIC | PR_ADDR | PR_MPSYSCTL);
        assert!(icmp6.usrreqs().is_some() && icmp6.has_ctloutput());
        assert_eq!(icmp6.domain().family(), f);

        // ip6_init's switch over the table: each number to its entry, IPv6 in IPv6 for 41,
        // the raw protocol for the rest.
        inet6sw_init::<G>();
        assert!(inet6_protocol::<G>(IPPROTO_UDP).is_some_and(|p| ptr::addr_eq(p, udp6)));
        assert!(is(inet6_protocol::<G>(IPPROTO_IPV6), &INET6SW[4]));
        let other = inet6_protocol::<G>(99).expect("the raw entry");
        assert_eq!(
            (other.ty(), i32::from(other.protocol())),
            (SOCK_RAW as i16, IPPROTO_RAW)
        );
        for n in [
            IPPROTO_ICMPV6,
            IPPROTO_DSTOPTS,
            IPPROTO_ROUTING,
            IPPROTO_FRAGMENT,
        ] {
            let pr = inet6_protocol::<G>(n).expect("an entry");
            assert_eq!(i32::from(pr.protocol()), n);
            assert!(INET6SW.iter().all(|r| !ptr::addr_eq(pr, r)), "the kernel's");
        }

        crate::netinet::in_pcb::tests::teardown();
    }

    /// This module's window of the kernel's user-copy double (free in `bsd`'s tests at N6.5d).
    #[cfg(feature = "inet6")]
    const UNET: usize = kernel::testutil::USER_BASE + 0x2_0000;

    /// `net_sysctl(name)` as sysctl(2) asks it: `room` bytes for the old value at [`UNET`]
    /// (0: no buffer, the size alone), `new` given at `UNET + 0x100`: the answer, the length
    /// and the old value's first word (-7 when nothing was written there).
    #[cfg(feature = "inet6")]
    fn net(
        p: &Proc,
        name: &[i32],
        room: usize,
        new: Option<i32>,
    ) -> (Result<(), Errno>, usize, i32) {
        use hal::user::UserPtr;
        if let Some(v) = new {
            assert!(UserPtr::<i32>::new(UNET + 0x100).copyout(&v).is_ok());
        }
        assert!(UserPtr::<i32>::new(UNET).copyout(&-7).is_ok());
        let mut len = room;
        let r = net_sysctl(
            name,
            if room == 0 { 0 } else { UNET },
            &mut len,
            if new.is_some() { UNET + 0x100 } else { 0 },
            if new.is_some() { 4 } else { 0 },
            p,
        );
        let v = UserPtr::<i32>::new(UNET).copyin().unwrap_or(-9);
        (r, len, v)
    }

    /// `net.inet6.*` through the IPv6 domain's table, as the C's `net_sysctl` walks
    /// `inet6sw[]`: the levels that are not terminal, the protocols without a handler, the IPv6
    /// entry's (the first of number 41: IPv6-in-IPv6's has none) and ICMPv6's names, bounds and
    /// statistics, the kernel's UDP (N6.6b0) and raw IPv6 (N6.6b1).
    #[cfg(feature = "inet6")]
    #[test]
    fn the_net_inet6_tree_answers_through_the_inet6_table() {
        use crate::netinet::in_::{IPPROTO_DIVERT, IPPROTO_ICMPV6, IPPROTO_IPV6};
        use crate::netinet::udp_var::{UDPCTL_STATS, Udpstat};
        use core::sync::atomic::Ordering::Relaxed;
        use kernel::netinet6::ip6_var::{
            ICMP6_REDIRTIMEOUT, ICMP6ERRPPSLIM, IP6_DEFHLIM, IP6_HDRNESTLIMIT,
        };
        use sysdefs::netinet::icmp6::{
            ICMPV6CTL_ERRPPSLIMIT, ICMPV6CTL_ND6_QUEUED, ICMPV6CTL_REDIRTIMEOUT, ICMPV6CTL_STATS,
            Icmp6stat,
        };
        use sysdefs::netinet6::in6::{
            IPV6CTL_DEFHLIM, IPV6CTL_HDRNESTLIMIT, IPV6CTL_MRTPROTO, IPV6CTL_STATS,
        };
        use sysdefs::netinet6::ip6_var::Ip6stat;
        use sysdefs::netinet6::raw_ip6::{RIPV6CTL_STATS, Rip6stat};

        let (_g, _t, p) = crate::netinet::in_pcb::tests::setup();
        let f = i32::from(AF_INET6);
        let ip6 = |n: i32| [f, IPPROTO_IPV6, n];
        let icmp6 = |n: i32| [f, IPPROTO_ICMPV6, n];

        // Not terminal below three names; a number without an entry, and divert6's entry
        // (no pr_sysctl in inet6sw[]): ENOPROTOOPT.
        assert_eq!(net(p, &[f], 4, None).0, Err(Errno::EISDIR));
        assert_eq!(net(p, &[f, IPPROTO_IPV6], 4, None).0, Err(Errno::EISDIR));
        assert_eq!(net(p, &[f, 99, 1], 4, None).0, Err(Errno::ENOPROTOOPT));
        assert_eq!(
            net(p, &[f, IPPROTO_DIVERT, 1], 4, None).0,
            Err(Errno::ENOPROTOOPT)
        );

        // net.inet6.ip6: a bounded name read, written (the old value answered), refused past
        // its bound; a terminal name with more below it, the multicast routing names.
        let hlim = IP6_DEFHLIM.load(Relaxed);
        assert_eq!(net(p, &ip6(IPV6CTL_DEFHLIM), 4, None), (Ok(()), 4, hlim));
        assert_eq!(
            net(p, &ip6(IPV6CTL_DEFHLIM), 4, Some(255)),
            (Ok(()), 4, hlim)
        );
        assert_eq!(IP6_DEFHLIM.load(Relaxed), 255);
        assert_eq!(
            net(p, &ip6(IPV6CTL_DEFHLIM), 4, Some(256)).0,
            Err(Errno::EINVAL)
        );
        assert_eq!(IP6_DEFHLIM.load(Relaxed), 255);
        IP6_DEFHLIM.store(hlim, Relaxed);
        let nest = IP6_HDRNESTLIMIT.load(Relaxed);
        assert_eq!(
            net(p, &ip6(IPV6CTL_HDRNESTLIMIT), 4, None),
            (Ok(()), 4, nest)
        );
        assert_eq!(
            net(p, &[f, IPPROTO_IPV6, IPV6CTL_DEFHLIM, 1], 4, None).0,
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            net(p, &ip6(IPV6CTL_MRTPROTO), 4, None).0,
            Err(Errno::EOPNOTSUPP)
        );
        let (r, len, _) = net(p, &ip6(IPV6CTL_STATS), 0, None);
        assert_eq!((r, len), (Ok(()), size_of::<Ip6stat>()));

        // net.inet6.icmp6: the bounded table, the redirect timeout, the queued count, the
        // statistics.
        let lim = ICMP6ERRPPSLIM.load(Relaxed);
        assert_eq!(
            net(p, &icmp6(ICMPV6CTL_ERRPPSLIMIT), 4, Some(-1)),
            (Ok(()), 4, lim)
        );
        assert_eq!(ICMP6ERRPPSLIM.load(Relaxed), -1);
        assert_eq!(
            net(p, &icmp6(ICMPV6CTL_ERRPPSLIMIT), 4, Some(-2)).0,
            Err(Errno::EINVAL)
        );
        ICMP6ERRPPSLIM.store(lim, Relaxed);
        assert_eq!(
            net(p, &icmp6(ICMPV6CTL_REDIRTIMEOUT), 4, None),
            (Ok(()), 4, ICMP6_REDIRTIMEOUT.load(Relaxed))
        );
        assert_eq!(net(p, &icmp6(ICMPV6CTL_ND6_QUEUED), 4, None).0, Ok(()));
        let (r, len, _) = net(p, &icmp6(ICMPV6CTL_STATS), 0, None);
        assert_eq!((r, len), (Ok(()), size_of::<Icmp6stat>()));
        assert_eq!(
            net(p, &[f, IPPROTO_ICMPV6, ICMPV6CTL_STATS, 1], 4, None).0,
            Err(Errno::ENOTDIR)
        );

        // The kernel's UDP's and raw IPv6's statistics.
        let (r, len, _) = net(p, &[f, IPPROTO_UDP, UDPCTL_STATS], 0, None);
        assert_eq!((r, len), (Ok(()), size_of::<Udpstat>()));
        let (r, len, _) = net(p, &[f, IPPROTO_RAW, RIPV6CTL_STATS], 0, None);
        assert_eq!((r, len), (Ok(()), size_of::<Rip6stat>()));

        crate::netinet::in_pcb::tests::teardown();
    }
}
/* </TESTS> */
