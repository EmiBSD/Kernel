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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! The Internet protocol switch as the input path reaches it: `inetsw[]` (the table of the
//! internet domain's protocols), `ip_protox[]` (an IP protocol number to its entry, every
//! number without one going to the raw entry) and the raw entry itself.
//!
//! Upstream: sys/netinet/in_proto.c @ 3ce1f3f79392
//! LZ: sys/netinet/in_proto.rs@3c62ede68802
//!
//! ## Deviations
//! - The table's entries are the kernel's IP, ICMP and IGMP ([`IpProtocol`], [`IcmpProtocol`],
//!   [`IgmpProtocol`]) and `bsd`'s transports and IPsec (`bsd`'s `inetsw[]` rows through the
//!   `Protocol` bridge impl) in the C's order; `bsd` assembles the slice and records it
//!   ([`inetsw_set`], `sys/kern/uipc_domain.rs`), until the transports move (N6.6, N8).
//!   `IPSEC` is configured (AH, ESP and IPComp after IGMP, `bsd`'s), `NPFSYNC` and `NPF`
//!   (pfsync's and divert's entries, `bsd`'s); `NGIF` is 0 (`ipip_input` serves
//!   `IPPROTO_IPV4` and, with `INET6`, `IPPROTO_IPV6`); `MPLS`, `NGRE`, `NCARP` and
//!   `NETHERIP` are not configured, their entries absent as LZ's. `SMALL_KERNEL` is not set:
//!   the sysctl handlers are in the table.
//! - `INET6` is configured: `ip_deliver`'s IPv6 steps reach `inet6sw[ip6_protox[nxt]]`
//!   through `kernel::netinet6::in6_proto`'s table (N6.5d; through the `Inet` bridge's
//!   `ip6_protox` leg and a `pffinddomain` of the IPv6 domain from N6.4c2 to N6.5c).
//!
//! ## Redesign
//! - LZ's `ip_protox[]` (a static array of atomics in `bsd`'s `in_proto.rs`) and the table's
//!   address are [`InetTables`], one per glue in the network's tables (`NetTables::inet`,
//!   D-9: the protocols are generic over the glue, `inetsw[]` cannot be a `static` of the
//!   kernel): the table, the index of its raw entry and `ip_protox[]`, filled once by
//!   [`inetsw_init`] as the C's `ip_init` fills `ip_protox[]`. A lookup is two loads and an
//!   index ([`ip_protocol`], `#[inline]`: per packet), the C's `inetsw[ip_protox[nxt]]`.
//! - LZ's `inetsw[]` of `Protosw` structs of function pointers and `inetdomain` -> the
//!   `Protocol`/`Domain` traits (decision 46): [`InetDomain`] over the glue's table, it and
//!   the kernel's entries zero-sized types with an associated `NEW` (the C's NULL entries are
//!   the traits' defaults); ICMP's and IGMP's share the raw entry's user requests and socket
//!   options (`rip_usrreqs`, `rip_ctloutput` in the C), reached through the raw entry
//!   ([`raw_protocol`]); IGMP's `rip_input` tail call is the raw entry's `Protocol::input`.
//!   Nothing keys a domain or a protocol by address (`pffinddomain`, `pffindproto` compare
//!   the family, the number and the type).

use core::marker::PhantomData;
use core::mem::size_of;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use hal::netlock::NetHeld;
use hal::once::OnceStatic;
use hal::sys::errno::Errno;
use hal::sys::netinet_in::{
    IPPROTO_DONE, IPPROTO_ICMP, IPPROTO_IGMP, IPPROTO_MAX, IPPROTO_RAW, SockaddrIn,
};
use hal::sys::socket::{AF_INET, PF_INET, SOCK_RAW};

use crate::assert::kpanic;
use crate::net::domain::{Domain, net};
use crate::net::if_var::Netstack;
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::proto::{PR_ADDR, PR_ATOMIC, PR_MPSYSCTL, Protocol, Usrreqs};
use crate::net::socket::{SoLocked, Socket};
use crate::proc::ProcGlue;
use crate::sysctl::SysctlReq;

/// The internet domain's switch of one glue: `inetsw[]`, its raw entry and `ip_protox[]`.
pub struct InetTables<G: ProcGlue> {
    /// `inetsw[]`, set once by [`inetsw_init`].
    sw: OnceStatic<&'static [&'static dyn Protocol<G>]>,
    /// The raw entry's index (`pffindproto(PF_INET, IPPROTO_RAW, SOCK_RAW) - inetsw`).
    raw: AtomicUsize,
    /// `ip_protox[]`: the entry of each IP protocol number.
    protox: [AtomicU8; IPPROTO_MAX as usize],
}

impl<G: ProcGlue> InetTables<G> {
    /// The tables before `ip_init`.
    pub const fn new() -> Self {
        Self {
            sw: OnceStatic::new(),
            raw: AtomicUsize::new(0),
            protox: [const { AtomicU8::new(0) }; IPPROTO_MAX as usize],
        }
    }
}

impl<G: ProcGlue> Default for InetTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The glue's internet switch.
#[inline]
fn tables<G: ProcGlue>() -> &'static InetTables<G> {
    &net::<G>().inet
}

/// `inetsw[]`: the internet protocols (none before `ip_init`).
pub fn inetsw<G: ProcGlue>() -> &'static [&'static dyn Protocol<G>] {
    tables::<G>().sw.get().copied().unwrap_or(&[])
}

/// `inetsw[]` recorded, once (the glue's boot, before `domaininit`: `bsd` assembles it of its
/// rows and the kernel's entries; a second call, a test's, keeps the first table).
pub fn inetsw_set<G: ProcGlue>(sw: &'static [&'static dyn Protocol<G>]) {
    let _ = tables::<G>().sw.set(sw);
}

/// `ip_init`'s switch: `ip_protox[]` filled from `inetsw[]`: every number to the raw entry,
/// then each protocol of the table (but the raw one) to its own. `panic(9)` without a raw
/// entry (the C's `panic("ip_init")`).
pub fn inetsw_init<G: ProcGlue>() {
    let t = tables::<G>();
    let sw = inetsw::<G>();

    // pffindproto(PF_INET, IPPROTO_RAW, SOCK_RAW): the raw protocol, or the raw wildcard.
    let exact = sw
        .iter()
        .position(|pr| i32::from(pr.protocol()) == IPPROTO_RAW && pr.ty() == SOCK_RAW as i16);
    let wildcard = sw
        .iter()
        .position(|pr| pr.ty() == SOCK_RAW as i16 && pr.protocol() == 0);
    let Some(raw) = exact.or(wildcard) else {
        kpanic(format_args!("ip_init"));
    };
    t.raw.store(raw, Ordering::Relaxed);
    for p in &t.protox {
        p.store(raw as u8, Ordering::Relaxed);
    }
    for (i, pr) in sw.iter().enumerate() {
        let proto = i32::from(pr.protocol());
        if pr.domain().family() == i32::from(PF_INET)
            && proto != 0
            && proto != IPPROTO_RAW
            && proto < IPPROTO_MAX
        {
            t.protox[proto as usize].store(i as u8, Ordering::Relaxed);
        }
    }
}

/// `inetsw[ip_protox[nxt]]`: the protocol of IP protocol number `nxt` (`None` before
/// `ip_init`, or for a number past `IPPROTO_MAX`, which no header holds).
#[inline]
pub fn ip_protocol<G: ProcGlue>(nxt: i32) -> Option<&'static dyn Protocol<G>> {
    let t = tables::<G>();
    let i = t
        .protox
        .get(usize::try_from(nxt).ok()?)?
        .load(Ordering::Relaxed);
    t.sw.get()?.get(usize::from(i)).copied()
}

/// The raw entry (`rip_input`'s, which ICMP and IGMP hand their messages to).
#[inline]
pub fn raw_protocol<G: ProcGlue>() -> Option<&'static dyn Protocol<G>> {
    let t = tables::<G>();
    t.sw.get()?.get(t.raw.load(Ordering::Relaxed)).copied()
}

/// `inetdomain`: the internet domain, its protocols the glue's `inetsw[]` ([`inetsw_set`], in
/// the C's order: the kernel's IP, ICMP and IGMP entries beside `bsd`'s transports and IPsec
/// until they move).
pub struct InetDomain<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> InetDomain<G> {
    /// The domain.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Domain<G> for InetDomain<G> {
    fn family(&self) -> i32 {
        i32::from(AF_INET)
    }
    fn name(&self) -> &'static str {
        "inet"
    }
    /// `dom_init`: `in_init`.
    fn init(&self) {
        crate::netinet::in_pcb::in_init::<G>();
    }
    fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
        inetsw::<G>()
    }
    /// The internet sockets' protocols are `bsd`'s (TCP, UDP, raw IP, divert, until N6.6):
    /// the kernel holds their sockets by a detached holding of `so_lock`.
    fn legacy(&self) -> bool {
        true
    }
    fn sasize(&self) -> u32 {
        size_of::<SockaddrIn>() as u32
    }
    fn rtoffset(&self) -> u32 {
        core::mem::offset_of!(SockaddrIn, sin_addr) as u32
    }
    fn maxplen(&self) -> u32 {
        32
    }
}

/// `&inetdomain`: `pr_domain` of the kernel's entries.
fn inetdomain<G: ProcGlue>() -> &'static dyn Domain<G> {
    &InetDomain::<G>::NEW
}

/// `inetsw[]`'s IP entry: `ip_init`, `ip_slowtimo`, `ip_sysctl`.
pub struct IpProtocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> IpProtocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for IpProtocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inetdomain::<G>()
    }
    fn ty(&self) -> i16 {
        0
    }
    fn protocol(&self) -> i16 {
        0
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
        crate::netinet::ip_input::ip_sysctl(name, req)
    }
    fn init(&self) {
        crate::netinet::ip_input::ip_init::<G>();
    }
    fn slowtimo(&self) {
        crate::netinet::ip_input::ip_slowtimo();
    }
}

/// The raw entry's user requests and socket options, which ICMP's and IGMP's entries share
/// (`rip_usrreqs`, `rip_ctloutput`: `bsd`'s until N6.6b).
fn raw<G: ProcGlue>() -> Option<&'static dyn Protocol<G>> {
    raw_protocol::<G>()
}

/// `inetsw[]`'s ICMP entry: `icmp_input`, `icmp_init`, `icmp_sysctl`, the raw sockets'
/// requests and options.
pub struct IcmpProtocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> IcmpProtocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for IcmpProtocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inetdomain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_ICMP as i16
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_MPSYSCTL
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        raw::<G>()?.usrreqs()
    }
    fn has_ctloutput(&self) -> bool {
        raw::<G>().is_some_and(Protocol::has_ctloutput)
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
        match raw::<G>() {
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
        crate::netinet::ip_icmp::icmp_input::<G>(mp, offp, proto, af, ns, net)
    }
    fn has_sysctl(&self) -> bool {
        true
    }
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        G::with_curproc(|p| match p {
            Some(p) => crate::netinet::ip_icmp::icmp_sysctl::<G>(p, name, req),
            None => Err(Errno::EINVAL),
        })
    }
    fn init(&self) {
        crate::netinet::ip_icmp::icmp_init::<G>();
    }
}

/// `inetsw[]`'s IGMP entry: `igmp_input` (then the raw sockets', `rip_input`), `igmp_init`,
/// the timeouts, `igmp_sysctl`, the raw sockets' requests and options.
pub struct IgmpProtocol<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> IgmpProtocol<G> {
    /// The entry.
    pub const NEW: Self = Self(PhantomData);
}

impl<G: ProcGlue> Protocol<G> for IgmpProtocol<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        inetdomain::<G>()
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        IPPROTO_IGMP as i16
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_MPSYSCTL
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        raw::<G>()?.usrreqs()
    }
    fn has_ctloutput(&self) -> bool {
        raw::<G>().is_some_and(Protocol::has_ctloutput)
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
        match raw::<G>() {
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
        let Some(m) = mp.take() else {
            return IPPROTO_DONE;
        };
        // igmp_input_if's tail: return rip_input(mp, offp, proto, af, ns).
        match crate::netinet::igmp::igmp_input(m, *offp) {
            Some(m) => match raw::<G>() {
                Some(r) => {
                    *mp = Some(m);
                    r.input(mp, offp, proto, af, ns, net)
                }
                None => IPPROTO_DONE,
            },
            None => IPPROTO_DONE,
        }
    }
    fn has_sysctl(&self) -> bool {
        true
    }
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        crate::netinet::igmp::igmp_sysctl(name, req)
    }
    fn init(&self) {
        crate::netinet::igmp::igmp_init();
    }
    fn fasttimo(&self) {
        crate::netinet::igmp::igmp_fasttimo();
    }
    fn slowtimo(&self) {
        crate::netinet::igmp::igmp_slowtimo();
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The kernel's entries answer as in_proto.c's rows (type, protocol, flags, the sysctl and
    // the raw entry's requests shared), the domain as `inetdomain`, and `ip_init`'s
    // `ip_protox[]` maps every number to its entry or the raw one.

    use hal::sys::netinet_in::{IPPROTO_TCP, IPPROTO_UDP};

    use super::*;
    use crate::netinet::ip_input::tests::{G, ICMP, RAW, SW, TCP, UDP, setup};

    #[test]
    fn the_entries_answer_as_the_c_s_rows() {
        let _g = setup();
        let ip = &IpProtocol::<G>::NEW;
        assert_eq!((ip.ty(), ip.protocol(), ip.flags()), (0, 0, PR_MPSYSCTL));
        assert!(ip.has_sysctl() && ip.usrreqs().is_none());
        let icmp = &IcmpProtocol::<G>::NEW;
        assert_eq!(
            (icmp.ty(), icmp.protocol(), icmp.flags()),
            (
                SOCK_RAW as i16,
                IPPROTO_ICMP as i16,
                PR_ATOMIC | PR_ADDR | PR_MPSYSCTL
            )
        );
        let igmp = &IgmpProtocol::<G>::NEW;
        assert_eq!(
            (igmp.ty(), igmp.protocol(), igmp.flags()),
            (
                SOCK_RAW as i16,
                IPPROTO_IGMP as i16,
                PR_ATOMIC | PR_ADDR | PR_MPSYSCTL
            )
        );
        assert!(icmp.has_sysctl() && igmp.has_sysctl());

        let dom = &InetDomain::<G>::NEW;
        assert_eq!((dom.family(), dom.name()), (i32::from(AF_INET), "inet"));
        assert_eq!((dom.sasize(), dom.rtoffset(), dom.maxplen()), (16, 4, 32));
        assert!(dom.legacy());
        assert_eq!(dom.protocols().len(), SW.len());
        assert_eq!(ip.domain().family(), i32::from(AF_INET));
    }

    #[test]
    fn ip_protox_maps_each_number_to_its_entry_or_the_raw_one() {
        let _g = setup();
        let same = |a: Option<&'static dyn Protocol<G>>, b: &'static dyn Protocol<G>| {
            a.is_some_and(|a| a.protocol() == b.protocol() && a.ty() == b.ty())
        };
        assert!(same(ip_protocol::<G>(IPPROTO_UDP), &UDP));
        assert!(same(ip_protocol::<G>(IPPROTO_TCP), &TCP));
        assert!(same(ip_protocol::<G>(IPPROTO_ICMP), &ICMP));
        assert!(same(ip_protocol::<G>(99), &RAW), "no entry: the raw one");
        assert!(same(raw_protocol::<G>(), &RAW));
        assert!(ip_protocol::<G>(IPPROTO_MAX).is_none());
        assert!(ip_protocol::<G>(-1).is_none());
    }
}
/* </TESTS> */
