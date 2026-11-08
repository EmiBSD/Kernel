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
//!   (`netinet/in_proto.rs`), `unixdomain` (the kernel's) and `routedomain`
//!   (`net/rtsock.rs`), with `pfkeydomain` (`net/pfkeyv2.rs`, `IPSEC`) first and
//!   `inet6domain` (`netinet6/in6_proto.rs`, `INET6`: the `inet6` feature) before
//!   `inetdomain`. `MPLS` and `NAF_FRAME` are not configured; their entries are comments,
//!   and so are the `PIPEX` branches of `net_sysctl`. `NPFLOW` is configured: `PF_PFLOW`
//!   goes to `pflow_sysctl`; so is `NBPFILTER`: `PF_BPF` goes to `bpf_sysctl`.
//! - `pffinddomain`, `pffindtype` and `pffindproto` over `bsd`'s tables return `Option`s for
//!   the C's NULL, for `bsd`'s input paths (`ip_input`'s raw protocol); `pfctlinput` is an
//!   `unsafe fn` over a raw socket address, as `pr_ctlinput` is (N6.4 types it); the
//!   kernel's `pr_ctlinput` of these tables is `panic(9)`: nothing in the kernel calls it
//!   before N6.4.
//! - `net_sysctl` follows `kern_sysctl.rs`'s calling convention (`docs/C_TO_RUST.md`);
//!   `sysctl_vslock` wraps the protocol handlers that are not `PR_MPSYSCTL`, as in C.

use core::ptr;

use kernel::net::domain::Domain as KDomain;
use kernel::net::mbuf::MbufBox;
use kernel::net::proto::{Protocol, SockaddrView, Usrreqs};
use kernel::net::socket::SoLocked;

use crate::kern::kern_sysctl::sysctl_vslock;
use crate::kern::proc_glue::BsdProcGlue as G;
use crate::kern::subr_prf::panic;
use crate::net::bpf::bpf_sysctl;
use crate::net::ifq::net_ifiq_sysctl;
use crate::net::pfkeyv2::{PFKEYDOMAIN, PFKEYSW};
use crate::net::rtsock::{ROUTEDOMAIN, ROUTESW};
use crate::netinet::in_proto::{INETDOMAIN, INETSW};
#[cfg(feature = "inet6")]
use crate::netinet6::in6_proto::{INET6DOMAIN, INET6SW};
use crate::sys::domain::Domain;
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, lend, park};
use crate::sys::proc::Proc;
use crate::sys::protosw::{PR_MPSYSCTL, PrUsrreqs, Protosw};
use crate::sys::socket::{
    NET_LINK_IFRXQ, PF_BPF, PF_LINK, PF_PFLOW, PF_UNIX, PF_UNSPEC, SOCK_RAW, Sockaddr,
};
use crate::sys::socketvar::{Socket, as_static};
use crate::sys::stat::Stat;
use crate::sys::systm::net_assert_locked;

/// `domains[]` of `bsd`'s tables: the configured communication domains whose protocols are
/// still `bsd`'s.
pub static DOMAINS: &[&Domain] = &[
    // MPLS: &mplsdomain, not configured.
    &PFKEYDOMAIN,
    #[cfg(feature = "inet6")]
    &INET6DOMAIN,
    &INETDOMAIN,
    &ROUTEDOMAIN,
    // NAF_FRAME: &framedomain, not configured.
];

/// `domains[]` as the kernel's socket layer sees it, in the C's order: `bsd`'s tables
/// through the traits, and the kernel's UNIX domain.
pub static NET_DOMAINS: &[&dyn KDomain<G>] = &[
    // MPLS: &mplsdomain, not configured.
    &PFKEYDOMAIN,
    #[cfg(feature = "inet6")]
    &INET6DOMAIN,
    &INETDOMAIN,
    &kernel::net::unix::Unix::<G>::NEW,
    &ROUTEDOMAIN,
    // NAF_FRAME: &framedomain, not configured.
];

/// The entries of a protocol table, as the kernel's protocols.
macro_rules! protocols {
    ($sw:ident; $($i:literal)*) => { [$(&$sw[$i] as &dyn Protocol<G>),*] };
}

/// `inetsw[]` as the kernel's protocols.
#[cfg(not(feature = "inet6"))]
static INET_PROTOS: [&dyn Protocol<G>; 13] = protocols!(INETSW; 0 1 2 3 4 5 6 7 8 9 10 11 12);
/// `inetsw[]` as the kernel's protocols.
#[cfg(feature = "inet6")]
static INET_PROTOS: [&dyn Protocol<G>; 14] = protocols!(INETSW; 0 1 2 3 4 5 6 7 8 9 10 11 12 13);
/// `inet6sw[]` as the kernel's protocols.
#[cfg(feature = "inet6")]
static INET6_PROTOS: [&dyn Protocol<G>; 15] =
    protocols!(INET6SW; 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14);
/// `routesw[]` as the kernel's protocols.
static ROUTE_PROTOS: [&dyn Protocol<G>; 1] = protocols!(ROUTESW; 0);
/// `pfkeysw[]` as the kernel's protocols.
static PFKEY_PROTOS: [&dyn Protocol<G>; 1] = protocols!(PFKEYSW; 0);

/// `domaininit`: the kernel's domains (`bsd`'s tables and the UNIX domain) and their
/// protocols initialised, the protocol timeouts started (`kernel::net::domain`).
pub fn domaininit() {
    kernel::net::domain::register_iflegacy::<G>(&crate::kern::sys_socket::IFLEGACY);
    #[cfg(feature = "nfsserver")]
    kernel::net::socket::register_upcall::<G>(crate::nfs::nfs_socket::nfsrv_rcv_upcall);
    kernel::net::domain::domaininit::<G>(NET_DOMAINS);
}

/// The kernel's domain table and `bsd`'s legs of the socket layer registered without the
/// protocols' initialisations, which `bsd`'s protocol tests run themselves over each test's
/// fresh memory (`domaininit` once per process would leave them in the first test's).
#[cfg(test)]
pub(crate) fn net_tables_for_tests() {
    kernel::net::domain::register_iflegacy::<G>(&crate::kern::sys_socket::IFLEGACY);
    #[cfg(feature = "nfsserver")]
    kernel::net::socket::register_upcall::<G>(crate::nfs::nfs_socket::nfsrv_rcv_upcall);
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
        if ptr::eq(self, &ROUTEDOMAIN) {
            return &ROUTE_PROTOS;
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
    fn ctlinput(&self, _cmd: i32, _sa: SockaddrView<'_>, _rdomain: u32) {
        // bsd's pfctlinput dispatches its own tables (N6.4 types the argument).
        panic(format_args!(
            "pr_ctlinput of a bsd protocol through the kernel"
        ));
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
    fn control(&self, so: &Socket, cmd: u64, data: &mut [u8]) -> Result<(), Errno> {
        match self.pru_control {
            Some(f) => f(as_static(so), cmd, data, None),
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
    let Some(dp) = pffinddomain(family) else {
        return Err(Errno::ENOPROTOOPT);
    };

    if name.len() < 3 {
        return Err(Errno::EISDIR); // overloaded
    }
    let protocol = name[1];
    for pr in dp.dom_protosw {
        if i32::from(pr.pr_protocol) == protocol
            && let Some(sysctl) = pr.pr_sysctl
        {
            let savelen = *oldlenp;
            // sysctl_vsunlock(oldp, savelen) is the guard's drop.
            let vs = if pr.pr_flags & PR_MPSYSCTL == 0 {
                Some(sysctl_vslock(oldp, savelen)?)
            } else {
                None
            };
            let error = sysctl(&name[2..], oldp, oldlenp, newp, newlen);
            drop(vs);

            return error;
        }
    }
    Err(Errno::ENOPROTOOPT)
}

/// `so->so_proto` as `bsd`'s table entry (`ifioctl`'s `pru_control`, which passes the
/// interface the kernel's `Usrreqs::control` does not carry until N6.3a): the entry of
/// `bsd`'s domains whose address the socket's protocol has, `None` for the kernel's UNIX
/// domain. A search of the tables (pffindproto's walk), on the interface ioctls alone.
pub fn so_protosw(so: &Socket) -> Option<&'static Protosw> {
    let want = ptr::from_ref(so.proto()).cast::<()>();
    DOMAINS
        .iter()
        .flat_map(|dp| dp.dom_protosw.iter())
        .find(|pr| ptr::eq(ptr::from_ref(*pr).cast::<()>(), want))
}

/// `pfctlinput`: hands control input `cmd` about `sa` to every protocol that takes it.
///
/// # Safety
///
/// `sa` points at a readable socket address of its `sa_len` bytes (`PrCtlinputFn`'s
/// contract).
pub unsafe fn pfctlinput(cmd: i32, sa: *const Sockaddr) {
    net_assert_locked("pfctlinput");

    for dp in DOMAINS {
        for pr in dp.dom_protosw {
            if let Some(ctlinput) = pr.pr_ctlinput {
                // SAFETY: the caller's contract; no argument, as the C passes NULL.
                unsafe { ctlinput(cmd, sa, 0, ptr::null_mut()) };
            }
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::netinet::in_::{IPPROTO_ICMP, IPPROTO_RAW, IPPROTO_UDP};
    use crate::sys::socket::{AF_INET, AF_ROUTE, AF_UNIX, SOCK_DGRAM};

    #[test]
    fn the_inet_protocols_are_found_by_number_and_type() {
        assert!(pffinddomain(i32::from(AF_INET)).is_some());
        assert!(pffinddomain(i32::from(AF_ROUTE)).is_some());
        // The UNIX domain is the kernel's: not among bsd's tables.
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
