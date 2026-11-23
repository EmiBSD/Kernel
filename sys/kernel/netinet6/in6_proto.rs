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
 *	@(#)in_proto.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The IPv6 protocol switch as the input path reaches it: `inet6sw[]` (the IPv6 domain's
//! protocols), `ip6_protox[]` (an IP protocol number to its entry, every number without one
//! going to the raw IPv6 entry) and the raw entry itself; `inet6domain`; and the entries of
//! `inet6sw[]` that are the kernel's: `netinet6/in6_proto.c`.
//!
//! Upstream: sys/netinet6/in6_proto.c @ 3ce1f3f79392
//! LZ: sys/netinet6/in6_proto.rs@3c62ede68802
//!
//! The configuration variables `in6_proto.c` defines (`ip6_forwarding` ..
//! `ip6_maxdynroutes`, `icmp6_redirtimeout`, `icmp6errppslim`, `ip6_mtudisc_timeout`) are
//! `kernel::netinet6::ip6_var`'s, beside the header's `extern`s (N6.5a); the raw sockets'
//! `rip6_sendspace` and `rip6_recvspace` stay with raw IPv6 (`bsd`'s `netinet6/in6_proto.rs`
//! until N6.6b).
//!
//! Locks: \[a\] the table is recorded once ([`inet6sw_set`], before `domaininit`) and
//! `ip6_protox[]` written by `ip6_init` ([`inet6sw_init`]) before the first packet: both read
//! lock-free, the table through its `OnceStatic`, `ip6_protox[]` relaxed, as the C's plain
//! array.
//!
//! ## Deviations
//! - The table's entries are the kernel's IPv6, TCP (N6.6a5), ICMPv6, destination options,
//!   routing header and fragment header ([`Ip6Protocol`], [`Tcp6Protocol`],
//!   [`Icmp6Protocol`], [`Dest6Protocol`], [`Route6Protocol`], [`Frag6Protocol`]) and `bsd`'s
//!   UDP, raw IPv6, AH, ESP, IPComp, IPv4 and IPv6 in IPv6, divert and the raw wildcard
//!   (`bsd`'s `inet6sw[]` rows through the `Protocol` bridge impl) in the C's order; `bsd`
//!   assembles the slice and records it ([`inet6sw_set`], `sys/kern/uipc_domain.rs`), until
//!   the transports move (N6.6b, N8).
//!   `IPSEC` is configured (AH, ESP and IPComp after the fragment header, `bsd`'s); so is
//!   `NPF` (divert's entry, `bsd`'s). `NGIF` is 0 (`ipip_input` serves `IPPROTO_IPV4` and
//!   `IPPROTO_IPV6`); `MPLS`, `NCARP`, `NETHERIP` and `NGRE` are not configured, their entries
//!   absent as LZ's. `SMALL_KERNEL` is not set: the sysctl handlers are in the table.
//!
//! ## Redesign
//! - LZ's `ip6_protox[]` (a static array of atomics in `bsd`'s `in6_proto.rs`), filled by
//!   `bsd`'s `ip6_init` (each entry's index found again by its address, `inet6sw_index`) and
//!   reached by the kernel through the `Inet` bridge's `ip6_protox` leg and a `pffinddomain`
//!   of the IPv6 domain at the first packet -> [`Inet6Tables`] in the glue's network tables
//!   (`NetTables::inet6`, D-9; the IPv4 twin is `kernel::netinet::in_proto::InetTables`): the
//!   table and the index of its raw entry, recorded together once by [`inet6sw_set`] (so the
//!   raw sockets' requests ICMPv6's entry lends are there before `ip6_init`, as the C's static
//!   `pr_usrreqs`: the R9 class of N6.4c2, a table read before it is set), and `ip6_protox[]`,
//!   filled by [`inet6sw_init`] from `ip6_init` as the C's `ip6_init` fills it (an entry's
//!   index is its place in the walk, no search). A lookup is two loads and an index
//!   ([`inet6_protocol`], `#[inline]`: per packet), the C's `inet6sw[ip6_protox[nxt]]`;
//!   [`raw6_protocol`] is one load and an index, for ICMPv6's tail call to the raw sockets. The
//!   `Inet` leg is gone.
//! - LZ's `inet6sw[]` of `Protosw` structs of function pointers and `inet6domain` -> the
//!   `Protocol`/`Domain` traits (decision 46): [`Inet6Domain`] over the glue's table, it and
//!   the kernel's entries zero-sized types with an associated `NEW` (the C's NULL entries are
//!   the traits' defaults). ICMPv6's control input, socket options and user requests are the
//!   raw entry's (`rip6_ctlinput`, `rip6_ctloutput`, `rip6_usrreqs` in the C, `bsd`'s until
//!   N6.6b), reached through the raw entry ([`raw6_protocol`]: the C names the same functions
//!   in both rows). `bsd`'s rows of the kernel's inputs (`kernel_input!`'s wrappers of
//!   `dest6_input`, `route6_input` and `frag6_input`, the parked packet unparked for each),
//!   its `frag6_slowtimo` wrapper and its wrappers of `ip6_init`, `ip6_sysctl`, `icmp6_input`,
//!   `icmp6_init`, `icmp6_fasttimo` and `icmp6_sysctl` go: the entries call the kernel's
//!   functions with the packet owned. Nothing keys a domain or a protocol by address: an
//!   entry's `pr_domain` is the zero-sized domain itself, no lookup (N6.4c2's R9: a
//!   `pffinddomain` there panicked in every `bsd` test that never registered the domains).
//! - Cost against the C: `ip_deliver`'s lookup per protocol header is the C's (`ip6_protox[]`
//!   read, the table's slot read, the entry's `&dyn` call for the C's call through
//!   `pr_input`); no lock, no spl transition, as the C. The C's `ip6_init` finds the raw
//!   entry with `pffindproto` (a walk of `domains[]` and of the table) and then walks the
//!   table once; here the raw entry is found when the table is recorded (at most two walks:
//!   the raw protocol, then the wildcard) and [`inet6sw_init`] walks it once to fill: O(n) as
//!   the C, once at boot.

use core::marker::PhantomData;
use core::mem::{offset_of, size_of};
use core::sync::atomic::{AtomicU8, Ordering};

use hal::netlock::NetHeld;
use hal::once::OnceStatic;
use hal::sys::errno::Errno;
use hal::sys::netinet_in::{
    IPPROTO_DIVERT, IPPROTO_DSTOPTS, IPPROTO_FRAGMENT, IPPROTO_ICMPV6, IPPROTO_IPV6, IPPROTO_MAX,
    IPPROTO_RAW, IPPROTO_ROUTING, IPPROTO_TCP,
};
use hal::sys::netinet6_in6::SockaddrIn6;
use hal::sys::socket::{AF_INET6, PF_INET6, SOCK_RAW, SOCK_STREAM};

use crate::assert::kpanic;
use crate::net::domain::{Domain, net};
use crate::net::if_var::Netstack;
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::proto::{
    CtlArg, PR_ABRTACPTDIS, PR_ADDR, PR_ATOMIC, PR_CONNREQUIRED, PR_MPINPUT, PR_MPSYSCTL,
    PR_SPLICE, PR_WANTRCVD, Protocol, SockaddrView, Usrreqs,
};
use crate::net::socket::{SoLocked, Socket};
use crate::netinet::tcp_input::tcp_input;
use crate::netinet::tcp_subr::tcp6_ctlinput;
use crate::netinet::tcp_usrreq::{TcpUsrreqs, tcp_ctloutput, tcp_sysctl};
use crate::netinet6::ip6_divert::{Divert6Usrreqs, divert6_init};
use crate::netinet6::raw_ip6::{
    Rip6Usrreqs, rip6_ctlinput, rip6_ctloutput, rip6_init, rip6_input, rip6_sysctl,
};
use crate::proc::ProcGlue;
use crate::sysctl::SysctlReq;

/// `inet6sw[]` as the glue recorded it, with the index of its raw entry: one record, so no
/// reader sees the table without its raw entry.
struct Inet6sw<G: ProcGlue> {
    /// `inet6sw[]`.
    sw: &'static [&'static dyn Protocol<G>],
    /// The raw entry's index (`pffindproto(PF_INET6, IPPROTO_RAW, SOCK_RAW) - inet6sw`), past
    /// the table when it has none.
    raw: usize,
}

/// The IPv6 domain's switch of one glue: `inet6sw[]` with its raw entry, and `ip6_protox[]`.
pub struct Inet6Tables<G: ProcGlue> {
    /// `inet6sw[]` and its raw entry, set once by [`inet6sw_set`].
    sw: OnceStatic<Inet6sw<G>>,
    /// `ip6_protox[]`: the entry of each IP protocol number (`ip6_init`'s, [`inet6sw_init`]).
    protox: [AtomicU8; IPPROTO_MAX as usize],
}

impl<G: ProcGlue> Inet6Tables<G> {
    /// The tables before the glue records `inet6sw[]`.
    pub const fn new() -> Self {
        Self {
            sw: OnceStatic::new(),
            protox: [const { AtomicU8::new(0) }; IPPROTO_MAX as usize],
        }
    }
}

impl<G: ProcGlue> Default for Inet6Tables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The glue's IPv6 switch.
#[inline]
fn tables<G: ProcGlue>() -> &'static Inet6Tables<G> {
    &net::<G>().inet6
}

/// `inet6sw[]`: the IPv6 protocols (none before the glue records them).
pub fn inet6sw<G: ProcGlue>() -> &'static [&'static dyn Protocol<G>] {
    tables::<G>().sw.get().map_or(&[], |s| s.sw)
}

/// `pffindproto(PF_INET6, IPPROTO_RAW, SOCK_RAW)` over `sw`: the raw protocol's index, or the
/// raw wildcard's.
fn raw_entry<G: ProcGlue>(sw: &[&'static dyn Protocol<G>]) -> Option<usize> {
    let exact = sw
        .iter()
        .position(|pr| i32::from(pr.protocol()) == IPPROTO_RAW && pr.ty() == SOCK_RAW as i16);
    exact.or_else(|| {
        sw.iter()
            .position(|pr| pr.ty() == SOCK_RAW as i16 && pr.protocol() == 0)
    })
}

/// `inet6sw[]` recorded with its raw entry, once (the glue's boot, before `domaininit`: `bsd`
/// assembles it of its rows and the kernel's entries; a second call, a test's, keeps the first
/// table). The raw entry is the table's from here on, before `ip6_init`: ICMPv6's entry lends
/// the raw sockets' requests to `socreate` as the C's static `pr_usrreqs` does.
pub fn inet6sw_set<G: ProcGlue>(sw: &'static [&'static dyn Protocol<G>]) {
    let raw = raw_entry(sw).unwrap_or(usize::MAX);
    let _ = tables::<G>().sw.set(Inet6sw { sw, raw });
}

/// `ip6_init`'s switch: `ip6_protox[]` filled from `inet6sw[]`: every number to the raw entry
/// (all protocols not implemented in the kernel go to the raw IPv6 protocol handler), then each
/// protocol of the table (but the raw one) to its own, a later entry of the same number over
/// an earlier one (IPv6 in IPv6 over the IPv6 entry, both of number 41). `panic(9)` without a
/// raw entry (the C's `panic("ip6_init")`), or before the glue recorded the table.
pub fn inet6sw_init<G: ProcGlue>() {
    let t = tables::<G>();
    let Some(&Inet6sw { sw, raw }) = t.sw.get().filter(|s| s.raw < s.sw.len()) else {
        kpanic(format_args!("ip6_init"));
    };
    for p in &t.protox {
        p.store(raw as u8, Ordering::Relaxed);
    }
    for (i, pr) in sw.iter().enumerate() {
        let proto = i32::from(pr.protocol());
        if pr.domain().family() == i32::from(PF_INET6)
            && proto != 0
            && proto != IPPROTO_RAW
            && proto < IPPROTO_MAX
        {
            t.protox[proto as usize].store(i as u8, Ordering::Relaxed);
        }
    }
}

/// `inet6sw[ip6_protox[nxt]]`: the IPv6 protocol of number `nxt` (`None` before the glue
/// records the table, or for a number past `IPPROTO_MAX`, which no header holds).
#[inline]
pub fn inet6_protocol<G: ProcGlue>(nxt: i32) -> Option<&'static dyn Protocol<G>> {
    let t = tables::<G>();
    let i = t
        .protox
        .get(usize::try_from(nxt).ok()?)?
        .load(Ordering::Relaxed);
    t.sw.get()?.sw.get(usize::from(i)).copied()
}

/// The raw IPv6 entry (`rip6_input`'s, which ICMPv6 hands its messages to, and whose control
/// input, socket options and user requests ICMPv6's entry shares); `None` before the glue
/// records the table, or for a table without one.
#[inline]
pub fn raw6_protocol<G: ProcGlue>() -> Option<&'static dyn Protocol<G>> {
    let s = tables::<G>().sw.get()?;
    s.sw.get(s.raw).copied()
}

/// `inet6domain`: the IPv6 domain, its protocols the glue's `inet6sw[]` ([`inet6sw_set`], in
/// the C's order: the kernel's IPv6, ICMPv6 and extension header entries beside `bsd`'s
/// transports, raw IPv6, IPsec, IP-in-IP and divert until they move). No `dom_init`, as the C.
pub struct Inet6Domain<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Inet6Domain<G> {
    /// The domain.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Domain<G> for Inet6Domain<G> {
    fn family(&self) -> i32 {
        i32::from(AF_INET6)
    }
    fn name(&self) -> &'static str {
        "inet6"
    }
    fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
        inet6sw::<G>()
    }
    fn sasize(&self) -> u32 {
        size_of::<SockaddrIn6>() as u32
    }
    fn rtoffset(&self) -> u32 {
        offset_of!(SockaddrIn6, sin6_addr) as u32
    }
    fn maxplen(&self) -> u32 {
        128
    }
}

/// `&inet6domain`: `pr_domain` of the kernel's entries.
fn inet6domain<G: ProcGlue>() -> &'static dyn Domain<G> {
    &Inet6Domain::<G>::NEW
}

/// `inet6sw[]`'s IPv6 entry: `ip6_init`, `frag6_slowtimo`, `ip6_sysctl` (no input: the
/// number's input is IPv6 in IPv6's, the later entry of number 41).
pub struct Ip6Protocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Ip6Protocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for Ip6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        0
    }
    fn protocol(&self) -> i16 {
        IPPROTO_IPV6 as i16
    }
    fn flags(&self) -> i16 {
        PR_MPSYSCTL
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        None
    }
    fn has_sysctl(&self) -> bool {
        true
    }
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        crate::netinet6::ip6_input::ip6_sysctl(name, req)
    }
    fn init(&self) {
        crate::netinet6::ip6_input::ip6_init::<G>();
    }
    fn slowtimo(&self) {
        crate::netinet6::frag6::frag6_slowtimo();
    }
}

/// `inet6sw[]`'s TCP entry: `tcp_input`, `tcp6_ctlinput`, `tcp_ctloutput`, `tcp6_usrreqs`,
/// `tcp_sysctl` (N6.6a5; `bsd`'s row until then).
pub struct Tcp6Protocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Tcp6Protocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for Tcp6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_STREAM as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_TCP as i16
    }
    fn flags(&self) -> i16 {
        PR_CONNREQUIRED | PR_WANTRCVD | PR_ABRTACPTDIS | PR_SPLICE | PR_MPINPUT | PR_MPSYSCTL
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        Some(&TcpUsrreqs::<G>::INET6)
    }
    fn ctlinput(
        &self,
        cmd: i32,
        sa: SockaddrView<'_>,
        rdomain: u32,
        arg: CtlArg<'_>,
        net: &NetHeld<'_>,
    ) {
        tcp6_ctlinput::<G>(cmd, sa, rdomain, arg, net);
    }
    fn input(
        &self,
        mp: &mut Option<MbufBox>,
        offp: &mut i32,
        proto: i32,
        af: i32,
        ns: Option<&mut Netstack>,
        net: &NetHeld<'_>,
    ) -> i32 {
        tcp_input::<G>(mp, offp, proto, af, ns, net)
    }
    fn has_ctloutput(&self) -> bool {
        true
    }
    fn ctloutput(
        &self,
        op: i32,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        level: i32,
        optname: i32,
        m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        tcp_ctloutput(op, so, w, level, optname, m)
    }
    fn has_sysctl(&self) -> bool {
        true
    }
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        tcp_sysctl::<G>(name, req)
    }
}

/// `inet6sw[]`'s raw entries (N6.6b1): the `IPPROTO_RAW` row (`rip6_input`,
/// `rip6_ctlinput`, `rip6_ctloutput`, `rip6_usrreqs`, `rip6_sysctl`) and the raw wildcard
/// (`wild`, protocol 0, the last row: `rip6_input`, `rip6_ctloutput`, `rip6_usrreqs`, its
/// `pr_init` `rip6_init`).
pub struct Rip6Protocol<G: ProcGlue> {
    /// The raw wildcard row.
    wild: bool,
    /// The glue.
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> Rip6Protocol<G> {
    /// The `IPPROTO_RAW` row.
    pub const RAW: Self = Self {
        wild: false,
        _g: PhantomData,
    };
    /// The raw wildcard row.
    pub const WILD: Self = Self {
        wild: true,
        _g: PhantomData,
    };
}

impl<G: ProcGlue> Protocol<G> for Rip6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        if self.wild { 0 } else { IPPROTO_RAW as i16 }
    }
    fn flags(&self) -> i16 {
        if self.wild {
            PR_ATOMIC | PR_ADDR | PR_MPINPUT
        } else {
            PR_ATOMIC | PR_ADDR | PR_MPINPUT | PR_MPSYSCTL
        }
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        Some(&Rip6Usrreqs::<G>::NEW)
    }
    /// `rip6_ctlinput` (the `IPPROTO_RAW` row's; the wildcard has none).
    fn ctlinput(
        &self,
        cmd: i32,
        sa: SockaddrView<'_>,
        rdomain: u32,
        arg: CtlArg<'_>,
        net: &NetHeld<'_>,
    ) {
        if !self.wild {
            rip6_ctlinput::<G>(cmd, sa, rdomain, arg, net);
        }
    }
    fn input(
        &self,
        mp: &mut Option<MbufBox>,
        offp: &mut i32,
        proto: i32,
        af: i32,
        ns: Option<&mut Netstack>,
        net: &NetHeld<'_>,
    ) -> i32 {
        rip6_input::<G>(mp, offp, proto, af, ns, net)
    }
    fn has_ctloutput(&self) -> bool {
        true
    }
    fn ctloutput(
        &self,
        op: i32,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        level: i32,
        optname: i32,
        m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        rip6_ctloutput(op, so, w, level, optname, m)
    }
    /// `rip6_sysctl` (the `IPPROTO_RAW` row's).
    fn has_sysctl(&self) -> bool {
        !self.wild
    }
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        rip6_sysctl(name, req)
    }
    /// `rip6_init` (the wildcard row's).
    fn init(&self) {
        if self.wild {
            rip6_init::<G>();
        }
    }
}

/// `inet6sw[]`'s divert6 entry (N6.6b1; `NPF`): no input (pf hands it the packets,
/// `divert6_packet`), `rip6_ctloutput`, `divert6_usrreqs`, `divert6_init`; no sysctl.
pub struct Divert6Protocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Divert6Protocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for Divert6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_DIVERT as i16
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_MPSYSCTL
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        Some(&Divert6Usrreqs::<G>::NEW)
    }
    fn has_ctloutput(&self) -> bool {
        true
    }
    fn ctloutput(
        &self,
        op: i32,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        level: i32,
        optname: i32,
        m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        rip6_ctloutput(op, so, w, level, optname, m)
    }
    fn init(&self) {
        divert6_init::<G>();
    }
}

/// `inet6sw[]`'s ICMPv6 entry: `icmp6_input`, `icmp6_init`, `icmp6_fasttimo`,
/// `icmp6_sysctl`, the raw IPv6 sockets' control input, requests and options (the raw entry's,
/// [`raw6_protocol`]: `rip6_ctlinput`, `rip6_usrreqs`, `rip6_ctloutput`, `bsd`'s until N6.6b).
pub struct Icmp6Protocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Icmp6Protocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for Icmp6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_ICMPV6 as i16
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_MPSYSCTL
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        raw6_protocol::<G>()?.usrreqs()
    }
    /// The raw IPv6 entry's: an ICMPv6 socket is a raw socket served by its requests.
    fn legacy(&self) -> bool {
        raw6_protocol::<G>().is_some_and(Protocol::legacy)
    }
    fn ctlinput(
        &self,
        cmd: i32,
        sa: SockaddrView<'_>,
        rdomain: u32,
        arg: CtlArg<'_>,
        net: &NetHeld<'_>,
    ) {
        if let Some(r) = raw6_protocol::<G>() {
            r.ctlinput(cmd, sa, rdomain, arg, net);
        }
    }
    fn has_ctloutput(&self) -> bool {
        raw6_protocol::<G>().is_some_and(Protocol::has_ctloutput)
    }
    fn ctloutput(
        &self,
        op: i32,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        level: i32,
        optname: i32,
        m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        match raw6_protocol::<G>() {
            Some(r) => r.ctloutput(op, so, w, level, optname, m),
            None => Err(Errno::ENOPROTOOPT),
        }
    }
    fn input(
        &self,
        mp: &mut Option<MbufBox>,
        offp: &mut i32,
        proto: i32,
        af: i32,
        ns: Option<&mut Netstack>,
        net: &NetHeld<'_>,
    ) -> i32 {
        crate::netinet6::icmp6::icmp6_input::<G>(mp, offp, proto, af, ns, net)
    }
    fn has_sysctl(&self) -> bool {
        true
    }
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        crate::netinet6::icmp6::icmp6_sysctl(name, req)
    }
    fn init(&self) {
        crate::netinet6::icmp6::icmp6_init();
    }
    fn fasttimo(&self) {
        crate::netinet6::icmp6::icmp6_fasttimo();
    }
}

/// `inet6sw[]`'s destination options entry: `dest6_input`, under the shared net lock.
pub struct Dest6Protocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Dest6Protocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for Dest6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_DSTOPTS as i16
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_MPINPUT
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        None
    }
    fn input(
        &self,
        mp: &mut Option<MbufBox>,
        offp: &mut i32,
        proto: i32,
        af: i32,
        ns: Option<&mut Netstack>,
        net: &NetHeld<'_>,
    ) -> i32 {
        crate::netinet6::dest6::dest6_input(mp, offp, proto, af, ns, net)
    }
}

/// `inet6sw[]`'s routing header entry: `route6_input`, under the shared net lock.
pub struct Route6Protocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Route6Protocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for Route6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_ROUTING as i16
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_MPINPUT
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        None
    }
    fn input(
        &self,
        mp: &mut Option<MbufBox>,
        offp: &mut i32,
        proto: i32,
        af: i32,
        ns: Option<&mut Netstack>,
        net: &NetHeld<'_>,
    ) -> i32 {
        crate::netinet6::route6::route6_input(mp, offp, proto, af, ns, net)
    }
}

/// `inet6sw[]`'s fragment header entry: `frag6_input` (reassembly), under the shared net
/// lock.
pub struct Frag6Protocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Frag6Protocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for Frag6Protocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inet6domain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_FRAGMENT as i16
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_MPINPUT
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        None
    }
    fn input(
        &self,
        mp: &mut Option<MbufBox>,
        offp: &mut i32,
        proto: i32,
        af: i32,
        ns: Option<&mut Netstack>,
        net: &NetHeld<'_>,
    ) -> i32 {
        crate::netinet6::frag6::frag6_input(mp, offp, proto, af, ns, net)
    }
}

/// The kernel's host tests' `inet6sw[]` (the glue's table is set once per test process, so
/// every test that needs one records this one): the kernel's five entries at their C indices
/// and doubles of `bsd`'s rows (UDP with `PR_MPINPUT`, TCP without it, the raw protocol, IPv6
/// in IPv6, divert, whose number is past `IPPROTO_MAX`, and the raw wildcard), each counting
/// the packets it is given (then freed) and the control input it is told of.
#[cfg(test)]
pub(crate) mod testsw6 {
    use core::sync::atomic::{AtomicI32, AtomicU32};

    use hal::sys::netinet_in::{IPPROTO_DONE, IPPROTO_TCP, IPPROTO_UDP};
    use hal::sys::socket::{SOCK_DGRAM, SOCK_STREAM};

    use super::*;

    /// The tests' glue.
    pub(crate) type G = crate::fs::testfs::TestGlue;

    /// A double of one of `bsd`'s rows.
    pub(crate) struct Row6 {
        proto: i16,
        ty: i16,
        flags: i16,
        /// Of another family's domain (`ip6_init`'s check of each entry's family).
        foreign: bool,
        /// Packets given.
        pub(crate) inputs: AtomicU32,
        /// The last `pr_ctlinput` command (0: none).
        pub(crate) ctl: AtomicI32,
        /// The quoted packet's next header of the last `CtlArg::Ip6` (-1: none).
        pub(crate) ctl6_nxt: AtomicI32,
        /// The last `CtlArg::Ip6`'s command argument (0: none).
        pub(crate) ctl6_cmdarg: AtomicU32,
        /// The last `CtlArg::Ip6`'s offset of the upper layer's header (`ip6c_off`; -1: none).
        pub(crate) ctl6_off: AtomicI32,
        /// The last `CtlArg::Ip6`'s final destination (`*ip6c_finaldst`).
        pub(crate) ctl6_finaldst: std::sync::Mutex<[u8; 16]>,
        /// A final destination to give the next `CtlArg::Ip6` back (`udp6_ctlinput`'s
        /// embedded zone, written through `ip6c_finaldst`), taken when used.
        pub(crate) ctl6_rewrite: std::sync::Mutex<Option<[u8; 16]>>,
        /// The bytes of the last packet given.
        pub(crate) last: std::sync::Mutex<std::vec::Vec<u8>>,
    }

    impl Row6 {
        const fn new(proto: i32, ty: i32, flags: i16) -> Self {
            Self {
                proto: proto as i16,
                ty: ty as i16,
                flags,
                foreign: false,
                inputs: AtomicU32::new(0),
                ctl: AtomicI32::new(0),
                ctl6_nxt: AtomicI32::new(-1),
                ctl6_cmdarg: AtomicU32::new(0),
                ctl6_off: AtomicI32::new(-1),
                ctl6_finaldst: std::sync::Mutex::new([0; 16]),
                ctl6_rewrite: std::sync::Mutex::new(None),
                last: std::sync::Mutex::new(std::vec::Vec::new()),
            }
        }

        /// The double made one of another family's domain.
        const fn foreign(mut self) -> Self {
            self.foreign = true;
            self
        }
    }

    /// UDP: runs under the shared net lock.
    pub(crate) static UDP6: Row6 =
        Row6::new(IPPROTO_UDP, SOCK_DGRAM, PR_ATOMIC | PR_ADDR | PR_MPINPUT);
    /// TCP: needs the exclusive net lock.
    pub(crate) static TCP6: Row6 = Row6::new(IPPROTO_TCP, SOCK_STREAM, 0);
    /// The raw IPv6 protocol (`rip6_input`).
    pub(crate) static RAW6: Row6 =
        Row6::new(IPPROTO_RAW, SOCK_RAW, PR_ATOMIC | PR_ADDR | PR_MPINPUT);
    /// IPv6 in IPv6 (`ipip_input`), the second entry of number 41.
    pub(crate) static IPIP6: Row6 = Row6::new(IPPROTO_IPV6, SOCK_RAW, PR_ATOMIC | PR_ADDR);
    /// Divert (`IPPROTO_DIVERT`, 258: past `ip6_protox[]`, as the C's table has it).
    pub(crate) static DIVERT6: Row6 = Row6::new(
        hal::sys::netinet_in::IPPROTO_DIVERT,
        SOCK_RAW,
        PR_ATOMIC | PR_ADDR,
    );
    /// The raw wildcard.
    pub(crate) static WILD6: Row6 = Row6::new(0, SOCK_RAW, PR_ATOMIC | PR_ADDR | PR_MPINPUT);
    /// An entry of another family's domain, of number 50 (no table of the C has one: it shows
    /// `ip6_init`'s check of the family).
    pub(crate) static FOREIGN6: Row6 = Row6::new(
        hal::sys::netinet_in::IPPROTO_ESP,
        SOCK_RAW,
        PR_ATOMIC | PR_ADDR,
    )
    .foreign();

    /// The domain of [`FOREIGN6`]: `AF_INET`, no protocols of its own.
    struct OtherDomain;

    impl Domain<G> for OtherDomain {
        fn family(&self) -> i32 {
            i32::from(hal::sys::socket::AF_INET)
        }
        fn name(&self) -> &'static str {
            "inet"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
            &[]
        }
    }

    /// The tests' `inet6sw[]`, in the C's order of the entries it has.
    pub(crate) static SW6: [&dyn Protocol<G>; 12] = [
        &Ip6Protocol::<G>::NEW,
        &UDP6,
        &TCP6,
        &RAW6,
        &Icmp6Protocol::<G>::NEW,
        &Dest6Protocol::<G>::NEW,
        &Route6Protocol::<G>::NEW,
        &Frag6Protocol::<G>::NEW,
        &IPIP6,
        &FOREIGN6,
        &DIVERT6,
        &WILD6,
    ];

    /// The tests' table recorded and `ip6_init` run over it (`ip6_protox[]` filled; ND's and
    /// reassembly's pools and timeout, as their own fixtures make them; the receive entry
    /// point the routing fixture publishes already).
    pub(crate) fn install() {
        inet6sw_set::<G>(&SW6);
        crate::netinet6::ip6_input::ip6_init::<G>();
    }

    impl Protocol<G> for Row6 {
        fn domain(&self) -> &'static dyn Domain<G> {
            if self.foreign {
                return &OtherDomain;
            }
            inet6domain::<G>()
        }
        fn ty(&self) -> i16 {
            self.ty
        }
        fn protocol(&self) -> i16 {
            self.proto
        }
        fn flags(&self) -> i16 {
            self.flags
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
            None
        }
        fn ctlinput(
            &self,
            cmd: i32,
            _sa: SockaddrView<'_>,
            _rdomain: u32,
            arg: CtlArg<'_>,
            _net: &NetHeld<'_>,
        ) {
            self.ctl.store(cmd, Ordering::Relaxed);
            if let CtlArg::Ip6(p) = arg {
                self.ctl6_nxt.store(i32::from(p.nxt), Ordering::Relaxed);
                self.ctl6_cmdarg
                    .store(p.cmdarg.unwrap_or(0), Ordering::Relaxed);
                self.ctl6_off.store(p.off, Ordering::Relaxed);
                *self.ctl6_finaldst.lock().unwrap_or_else(|e| e.into_inner()) =
                    p.finaldst.get().s6_addr;
                let rewrite = self
                    .ctl6_rewrite
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
                if let Some(a) = rewrite {
                    p.finaldst
                        .set(hal::sys::netinet6_in6::In6Addr { s6_addr: a });
                }
            }
        }
        fn has_ctloutput(&self) -> bool {
            self.proto == IPPROTO_RAW as i16
        }
        fn input(
            &self,
            mp: &mut Option<MbufBox>,
            _offp: &mut i32,
            _proto: i32,
            _af: i32,
            _ns: Option<&mut Netstack>,
            _net: &NetHeld<'_>,
        ) -> i32 {
            self.inputs.fetch_add(1, Ordering::Relaxed);
            if let Some(m) = mp.take() {
                let mut b = std::vec![0u8; m.m_pkthdr().len.get() as usize];
                crate::net::mbuf::m_copydata(&m, 0, &mut b);
                *self.last.lock().unwrap_or_else(|e| e.into_inner()) = b;
            }
            IPPROTO_DONE
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The kernel's entries answer as in6_proto.c's rows (type, protocol, flags, the sysctl,
    // ICMPv6 sharing the raw entry's control input and options), the domain as `inet6domain`,
    // `ip6_init`'s `ip6_protox[]` maps every number to its entry or the raw one (a later entry
    // of a number over an earlier one, an entry of another family's domain skipped, divert's
    // number past the array), `ip_deliver` walks the table, queueing on `ip6intrq` the
    // protocols without `PR_MPINPUT`, and the kernel's entries take their headers (the
    // extension headers' inputs chained to UDP's entry, ICMPv6's counted by `icmp6_input`).
    // The table is the tests' `testsw6::SW6`, `bsd`'s rows doubled, and `ip6_init` fills
    // `ip6_protox[]` from it.

    use core::sync::atomic::Ordering::Relaxed;
    use std::sync::MutexGuard;

    use hal::netlock::NETLOCK;
    use hal::sys::netinet_in::{IPPROTO_DONE, IPPROTO_ESP, IPPROTO_TCP, IPPROTO_UDP};
    use hal::sys::netinet6_ip6_var::Ip6statCounters;

    use super::testsw6::{FOREIGN6, G, IPIP6, RAW6, Row6, SW6, TCP6, UDP6, install};
    use super::*;
    use crate::net::mbuf::m_devget;
    use crate::net::proto::PRC_MSGSIZE;
    use crate::netinet::ip_input::ip_deliver;
    use crate::netinet6::ip6_input::{ip6_test_reset, ip6intr};
    use crate::netinet6::ip6_var::IP6COUNTERS;

    /// The routing tests' fixture with the tests' `inet6sw[]` recorded and `ip6_protox[]`
    /// filled, `ip6intrq` empty.
    fn setup() -> MutexGuard<'static, ()> {
        let g = crate::net::route::tests::testroutes_setup();
        ip6_test_reset();
        install();
        g
    }

    /// Whether `a` is the double `b` (the same static).
    fn is_row(a: Option<&'static dyn Protocol<G>>, b: &'static Row6) -> bool {
        a.is_some_and(|a| core::ptr::addr_eq(a, b))
    }

    /// Whether `a` answers as the entry of type `ty` and number `proto` (the kernel's entries
    /// are zero-sized: their addresses tell nothing).
    fn is_entry(a: Option<&'static dyn Protocol<G>>, ty: i32, proto: i32) -> bool {
        a.is_some_and(|a| i32::from(a.ty()) == ty && i32::from(a.protocol()) == proto)
    }

    #[test]
    fn the_entries_answer_as_the_c_s_rows() {
        let _g = setup();
        let row = |pr: &dyn Protocol<G>| (i32::from(pr.ty()), i32::from(pr.protocol()), pr.flags());
        let ip6 = &Ip6Protocol::<G>::NEW;
        assert_eq!(row(ip6), (0, IPPROTO_IPV6, PR_MPSYSCTL));
        assert!(ip6.has_sysctl() && ip6.usrreqs().is_none() && !ip6.has_ctloutput());
        let icmp6 = &Icmp6Protocol::<G>::NEW;
        assert_eq!(
            row(icmp6),
            (SOCK_RAW, IPPROTO_ICMPV6, PR_ATOMIC | PR_ADDR | PR_MPSYSCTL)
        );
        assert!(icmp6.has_sysctl());
        let ext = PR_ATOMIC | PR_ADDR | PR_MPINPUT;
        let dest6: &dyn Protocol<G> = &Dest6Protocol::<G>::NEW;
        let route6: &dyn Protocol<G> = &Route6Protocol::<G>::NEW;
        let frag6: &dyn Protocol<G> = &Frag6Protocol::<G>::NEW;
        for (pr, proto) in [
            (dest6, IPPROTO_DSTOPTS),
            (route6, IPPROTO_ROUTING),
            (frag6, IPPROTO_FRAGMENT),
        ] {
            assert_eq!(row(pr), (SOCK_RAW, proto, ext), "{proto}");
            assert!(!pr.has_sysctl() && pr.usrreqs().is_none() && !pr.has_ctloutput());
        }

        // ICMPv6's control input and socket options are the raw entry's (rip6_ctlinput,
        // rip6_ctloutput in both rows of the C).
        assert!(icmp6.has_ctloutput(), "the raw entry's");
        RAW6.ctl.store(0, Relaxed);
        let mut sin6 = [0u8; 28];
        sin6[..2].copy_from_slice(&[28, AF_INET6]);
        let sa = SockaddrView::new(&sin6).expect("an address");
        {
            let w = hal::netlock::NETLOCK.read();
            let net = NetHeld::of(&w).expect("the net lock");
            icmp6.ctlinput(PRC_MSGSIZE, sa, 0, CtlArg::None, &net);
        }
        assert_eq!(RAW6.ctl.load(Relaxed), PRC_MSGSIZE);

        // The domain, and every entry's domain without a lookup.
        let dom = &Inet6Domain::<G>::NEW;
        assert_eq!((dom.family(), dom.name()), (i32::from(AF_INET6), "inet6"));
        assert_eq!((dom.sasize(), dom.rtoffset(), dom.maxplen()), (28, 8, 128));
        assert!(dom.rights().is_none());
        assert_eq!(icmp6.legacy(), RAW6.legacy(), "the raw entry's");
        assert_eq!(dom.protocols().len(), SW6.len());
        for pr in [ip6 as &dyn Protocol<G>, icmp6, dest6, route6, frag6] {
            assert_eq!(pr.domain().family(), i32::from(AF_INET6));
            assert_eq!(pr.domain().protocols().len(), SW6.len());
        }
    }

    #[test]
    fn ip6_protox_maps_each_number_to_its_entry_or_the_raw_one() {
        let _g = setup();
        assert!(is_row(inet6_protocol::<G>(IPPROTO_UDP), &UDP6));
        assert!(is_row(inet6_protocol::<G>(IPPROTO_TCP), &TCP6));
        assert!(is_entry(
            inet6_protocol::<G>(IPPROTO_ICMPV6),
            SOCK_RAW,
            IPPROTO_ICMPV6
        ));
        assert!(is_entry(
            inet6_protocol::<G>(IPPROTO_DSTOPTS),
            SOCK_RAW,
            IPPROTO_DSTOPTS
        ));
        assert!(is_entry(
            inet6_protocol::<G>(IPPROTO_ROUTING),
            SOCK_RAW,
            IPPROTO_ROUTING
        ));
        assert!(is_entry(
            inet6_protocol::<G>(IPPROTO_FRAGMENT),
            SOCK_RAW,
            IPPROTO_FRAGMENT
        ));
        // Number 41: IPv6 in IPv6, the later of its two entries (the IPv6 entry has no input).
        assert!(is_row(inet6_protocol::<G>(IPPROTO_IPV6), &IPIP6));
        // Every number without an entry, the raw protocol's own and 0: the raw protocol,
        // before the raw wildcard.
        for nxt in [99, IPPROTO_RAW, 0, IPPROTO_MAX - 1] {
            assert!(is_row(inet6_protocol::<G>(nxt), &RAW6), "{nxt}");
        }
        assert!(is_row(raw6_protocol::<G>(), &RAW6));
        // An entry of another family's domain is no IPv6 protocol.
        assert!(FOREIGN6.protocol() == IPPROTO_ESP as i16);
        assert!(is_row(inet6_protocol::<G>(IPPROTO_ESP), &RAW6));
        // Past the table's numbers: none.
        assert!(inet6_protocol::<G>(IPPROTO_MAX).is_none());
        assert!(inet6_protocol::<G>(hal::sys::netinet_in::IPPROTO_DIVERT).is_none());
        assert!(inet6_protocol::<G>(-1).is_none());
    }

    /// A 48-byte IPv6 packet whose header names `nxt`.
    fn packet(nxt: i32) -> MbufBox {
        let mut b = [0u8; 48];
        b[0] = 0x60;
        b[6] = nxt as u8;
        m_devget(&b, 0).expect("a packet")
    }

    /// `ip_deliver` of an IPv6 packet whose payload is protocol `nxt`, under the shared net
    /// lock, the input path's call.
    fn deliver(nxt: i32) -> i32 {
        let r = NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        let mut mp = Some(packet(nxt));
        let mut off = 40;
        ip_deliver::<G>(
            &mut mp,
            &mut off,
            nxt,
            i32::from(AF_INET6),
            true,
            None,
            &net,
        )
    }

    #[test]
    fn a_protocol_without_mpinput_waits_for_ip6intr() {
        let _g = setup();
        let delivered = || IP6COUNTERS.read()[Ip6statCounters::Ip6sDelivered as usize];
        let d = delivered();
        let (udp, tcp, raw) = (
            UDP6.inputs.load(Relaxed),
            TCP6.inputs.load(Relaxed),
            RAW6.inputs.load(Relaxed),
        );
        assert_eq!(deliver(IPPROTO_UDP), IPPROTO_DONE);
        assert_eq!(UDP6.inputs.load(Relaxed), udp + 1, "delivered in place");
        assert_eq!(deliver(99), IPPROTO_DONE);
        assert_eq!(RAW6.inputs.load(Relaxed), raw + 1, "to the raw sockets");
        assert_eq!(deliver(IPPROTO_TCP), IPPROTO_DONE);
        assert_eq!(TCP6.inputs.load(Relaxed), tcp, "queued, not delivered");
        assert_eq!(delivered(), d + 2, "the queued one taken back");
        let w = NETLOCK.write();
        ip6intr::<G>(&NetHeld::of(&w).expect("the net lock"));
        drop(w);
        assert_eq!(TCP6.inputs.load(Relaxed), tcp + 1, "ip6intr delivered it");
        assert_eq!(delivered(), d + 3);
    }

    #[test]
    fn the_kernel_s_entries_take_their_headers() {
        let _g = setup();
        // Destination options, a routing header at its final destination, an atomic fragment
        // and 8 bytes of UDP: dest6_input, route6_input and frag6_input each skip their header
        // and name the next, and ip_deliver reaches UDP's entry.
        let mut b = std::vec![0u8; 40];
        b[0] = 0x60;
        b[4..6].copy_from_slice(&32u16.to_be_bytes());
        b[6] = IPPROTO_DSTOPTS as u8;
        b.extend_from_slice(&[IPPROTO_ROUTING as u8, 0, 1, 4, 0, 0, 0, 0]);
        b.extend_from_slice(&[IPPROTO_FRAGMENT as u8, 0, 4, 0, 0, 0, 0, 0]);
        b.extend_from_slice(&[IPPROTO_UDP as u8, 0, 0, 0, 0, 0, 0, 7]);
        b.extend_from_slice(&[0; 8]);
        let deliver = |b: &[u8]| {
            let r = NETLOCK.read();
            let net = NetHeld::of(&r).expect("the net lock");
            let mut mp = Some(m_devget(b, 0).expect("a packet"));
            let mut off = 40;
            ip_deliver::<G>(
                &mut mp,
                &mut off,
                IPPROTO_DSTOPTS,
                i32::from(AF_INET6),
                true,
                None,
                &net,
            )
        };
        let udp = UDP6.inputs.load(Relaxed);
        let stat = |c: Ip6statCounters| IP6COUNTERS.read()[c as usize];
        let (a, bad) = (
            stat(Ip6statCounters::Ip6sReassembled),
            stat(Ip6statCounters::Ip6sBadoptions),
        );
        assert_eq!(deliver(&b), IPPROTO_DONE);
        assert_eq!(UDP6.inputs.load(Relaxed), udp + 1, "through three entries");
        assert_eq!(
            stat(Ip6statCounters::Ip6sReassembled),
            a + 1,
            "frag6_input's atomic fragment"
        );
        // The routing header with a segment left: route6_input's parameter problem.
        b[51] = 1;
        assert_eq!(deliver(&b), IPPROTO_DONE);
        assert_eq!(UDP6.inputs.load(Relaxed), udp + 1, "not delivered");
        assert_eq!(stat(Ip6statCounters::Ip6sBadoptions), bad + 1);

        // ICMPv6's entry is icmp6_input: a message shorter than its header is counted there.
        let short = || {
            crate::netinet6::ip6_var::ICMP6COUNTERS.read()
                [hal::sys::netinet_icmp6::Icmp6statCounters::Icp6sTooshort as usize]
        };
        let s = short();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut b = [0u8; 42];
        b[0] = 0x60;
        b[5] = 2;
        b[6] = IPPROTO_ICMPV6 as u8;
        let mut mp = Some(m_devget(&b, 0).expect("a packet"));
        let mut off = 40;
        let nxt = ip_deliver::<G>(
            &mut mp,
            &mut off,
            IPPROTO_ICMPV6,
            i32::from(AF_INET6),
            false,
            None,
            &net,
        );
        drop(w);
        assert_eq!(nxt, IPPROTO_DONE);
        assert_eq!(short(), s + 1);
    }
}
/* </TESTS> */
