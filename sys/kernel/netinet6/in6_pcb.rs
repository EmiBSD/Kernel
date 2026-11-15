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

/*
 * Copyright (c) 1982, 1986, 1990, 1993, 1995
 *	Regents of the University of California.  All rights reserved.
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
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 protocol control blocks: binding, connecting, lookups and notifications of
//! `INP_IPV6` pcbs, `netinet6/in6_pcb.c` (prototypes in `<netinet/in_pcb.h>`). `in_pcb.c` is
//! used for inet and inet6; this file only holds the special IPv6 cases: the hash of an IPv6
//! quadruple, the checks of a local IPv6 address, connecting, the address queries of
//! `getsockname(2)`/`getpeername(2)`, ICMPv6 notifications, the route, the exact and
//! listening lookups and the addresses of an accepted connection.
//!
//! Upstream: sys/netinet6/in6_pcb.c @ 3ce1f3f79392
//! LZ: sys/netinet6/in6_pcb.rs@3c62ede68802
//!
//! The licence block carries the NRL notice with its advertising clause, accepted as every
//! licence of the pinned OpenBSD tree is.
//!
//! ## Deviations
//! - `NPF`: the divert keys of [`in6_pcblookup_listen`] come through the `Pf` bridge
//!   (`find_divert`); before pf registers, no packet carries `PF_TAG_DIVERTED`.
//! - The `DIAGNOSTIC` `in_pcbnotifymiss` printfs are not made.
//! - `NSTOEPLITZ`: the flow id of a connected or accepted pcb is `bsd`'s adapter's, after
//!   [`in6_pcbconnect`] and [`in6_pcbset_addr`] return (`stoeplitz` is `bsd`'s until N8; the
//!   C computes it at their end, after the table mutex as there).
//! - [`in6_pcbnotify`] reads an unconnected pcb's route cache (the redirect clause) once it
//!   holds that pcb's socket lock, where the C reads it under the table mutex alone: the
//!   cache is `[s]`, and a read of another socket's cache is a race with its owner (N6.3b2's
//!   first review, kept from `bsd`'s). A redirect or a dead host thus costs one socket lock
//!   pair (and the table mutex let go and taken again) per unconnected pcb that neither the
//!   cache nor the addresses select, where the C takes the socket lock only for the pcbs it
//!   notifies. Its `cmdarg`, which the C does not use, is not taken.
//!
//! ## Redesign
//! - D-17 (R3): the lookups and the hash, which read only the table and the pcbs' \[t\]
//!   members, are the kernel's since N6.4c1, over `crate::netinet::in_pcb`'s table (the
//!   pinned hash chains, the counted answers); the functions that need `in6_src.c`, `in6.c`'s
//!   address flags or the scope zones (`in6_pcbaddrisavail`, `in6_pcbconnect`,
//!   `in6_setsockaddr`, `in6_setpeeraddr`, `in6_pcbnotify`, `in6_pcbrtentry`,
//!   `in6_pcbset_addr`) since N6.5c, with `in_pcb.c`'s `#ifdef INET6` branches
//!   (`in_pcbbind_locked`, `in_pcbconnect`, `in_pcbrtentry`), so `bsd`'s `netinet6/in6_pcb.rs`
//!   is an adapter of LZ's signatures.
//! - The options `in6_pcb.c` reads (`inp_outputopts6`'s packet info, `inp_moptions6`'s
//!   interface) are lent by the pcb's typed cells for the statement that reads them
//!   (`Inpcb::outputopts6`, `Inpcb::moptions6_out`, D-10), never across a call that lends
//!   them again; [`in6_pcbconnect`] lends the sticky options to `in6_pcbselsrc` mutably, the
//!   C's "this overrides pi" (D-22).
//! - The `struct sockaddr_in6 *` the C modifies in place (`in6_pcbaddrisavail`'s, the copy
//!   `in6_pcbconnect` makes) is a `&mut` or a local; [`in6_setsockaddr`] and
//!   [`in6_setpeeraddr`] return the address the caller writes into its name mbuf.
//! - [`in6_pcbnotify`]'s `notify` is an `Option<&mut dyn FnMut>`, the C's nullable function
//!   pointer, replaced by `in_pcbrtchange` for a redirect as there.
//! - The addresses are their sixteen bytes (`crate::netinet::in_pcb::Addr6`), so `bsd`'s
//!   `In6Addr` and `hal`'s both reach them.
//!
//! **Lock and spl transitions** against the C: `in6_pcbaddrisavail` takes the table mutex
//! once for both lookups where the C's `IN_PCBLOCK_GRAB` takes it per lookup (one pair
//! fewer); `in6_pcbconnect`'s are the C's (the table mutex once; the options' loans are two
//! atomics each, no lock); `in6_pcbnotify`'s are the module's deviations above.

use core::mem::size_of;

use hal::netlock::NetHeld;
use hal::pod;
use hal::pool::PoolArc;
use hal::sys::endian::htonl;
use hal::sys::errno::Errno;
use hal::sys::net_route::RTF_HOST;
use hal::sys::netinet_ip6::{IPV6_FLOWLABEL_MASK, Ip6Hdr};
use hal::sys::netinet6_in6::{
    IN6ADDR_ANY, In6Addr, SA6_ANY, SockaddrIn6, in6_are_addr_equal, in6_is_addr_multicast,
    in6_is_addr_unspecified, in6_is_addr_v4mapped,
};
use hal::sys::netinet6_in6_var::{
    IN6_IFF_ANYCAST, IN6_IFF_DETACHED, IN6_IFF_DUPLICATED, IN6_IFF_TENTATIVE,
};
use hal::sys::socket::{SO_BINDANY, SO_REUSEADDR, SO_REUSEPORT};

use crate::assert::kassert;
use crate::assert::kpanic;
use crate::net::if_::ifa_ifwithaddr;
use crate::net::mbuf::{Mbuf, PF_TAG_DIVERTED, PF_TAG_TRANSLATE_LOCALHOST};
use crate::net::pf_hooks::{PF_DIVERT_REPLY, PF_DIVERT_TO, pf};
use crate::net::proto::{PRC_HOSTDEAD, PRC_NCMDS, prc_is_redirect};
use crate::net::route::Rtentry;
use crate::net::rtable::rtable_l2;
use crate::netinet::in_pcb::{
    Addr6, INP_IPV6, INPLOOKUP_IPV6, INPLOOKUP_WILDCARD, InpGuard, Inpaddru, Inpcb, InpcbRef,
    Inpcbtable, hash_chain_find, in_pcb_iterator, in_pcbbind_locked, in_pcblookup_local,
    in_pcbrehash, in_pcbrtchange, in_pcbsolock, in_pcbsounlock,
};
use crate::netinet6::in6::in6_nam2sin6;
use crate::netinet6::in6_src::{in6_embedscope, in6_pcbselsrc, in6_recoverscope, in6_selecthlim};
use crate::netinet6::in6_var::In6Ifaddr;
use crate::netinet6::ip6_id::ip6_randomflowlabel;
use crate::netinet6::ip6_input::INET6CTLERRMAP;
use crate::netinet6::ip6_output::ro_sin6;
use crate::netinet6::ip6_var::Ip6Pktopts;
use crate::proc::{ProcGlue, Thread};

/// `in6_pcbnotify`'s notify function: called on each pcb that matches, with the error the
/// command maps to (`inet6ctlerrmap`).
pub type In6PcbNotify<'a, G> = dyn FnMut(&Inpcb<G>, Option<Errno>) + 'a;

/// `zeroin6_addr`: the unspecified address.
pub static ZEROIN6_ADDR: In6Addr = In6Addr { s6_addr: [0; 16] };

/// `in6_pcbhash`: the hash of the complete IPv6 address quadruple in routing domain
/// `rdomain`.
#[inline]
pub fn in6_pcbhash<G: ProcGlue>(
    table: &Inpcbtable<G>,
    rdomain: u32,
    faddr: &[u8; 16],
    fport: u16,
    laddr: &[u8; 16],
    lport: u16,
) -> u64 {
    let mut ctx = table.inpt_key.siphash24();
    ctx.update(&rdomain.to_be_bytes());
    ctx.update(faddr);
    ctx.update(&fport.to_ne_bytes());
    ctx.update(laddr);
    ctx.update(&lport.to_ne_bytes());
    ctx.end()
}

/// `in6_pcbaddrisavail_lock(inp, sin6, wild, p, IN_PCBLOCK_HOLD)`: whether `inp` may bind to
/// `sin6` (an address of ours, not anycast nor unusable, and a port nobody else has, as `wild`
/// and the reuse options allow), the table mutex held (`g`). The scope zone is embedded into
/// `sin6`'s address and its scope id cleared, as the C does in place.
pub fn in6_pcbaddrisavail_held<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    inp: &Inpcb<G>,
    sin6: &mut SockaddrIn6,
    wild: i32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let so = inp.socket();
    let lport = sin6.sin6_port;
    let mut reuseport = so.options() & SO_REUSEPORT;
    let wild = wild | INPLOOKUP_IPV6;

    // KAME hack: embed scopeid (the pcb's packet info and multicast interface: the C reads
    // `inp_outputopts6` and `inp_moptions6`).
    let pi_ifindex = inp
        .outputopts6()
        .as_deref()
        .and_then(Ip6Pktopts::pktinfo)
        .map_or(0, |p| p.ipi6_ifindex);
    let im6o_ifidx = inp.moptions6_out().map_or(0, |o| o.im6o_ifidx);
    let mut addr = sin6.sin6_addr;
    if in6_embedscope(&mut addr, sin6, pi_ifindex, im6o_ifidx).is_err() {
        return Err(Errno::EINVAL);
    }
    sin6.sin6_addr = addr;
    // this must be cleared for ifa_ifwithaddr()
    sin6.sin6_scope_id = 0;
    // reject IPv4 mapped address, we have no support for it
    if in6_is_addr_v4mapped(&sin6.sin6_addr) {
        return Err(Errno::EADDRNOTAVAIL);
    }

    if in6_is_addr_multicast(&sin6.sin6_addr) {
        // Treat SO_REUSEADDR as SO_REUSEPORT for multicast; allow complete duplication of
        // binding if SO_REUSEPORT is set, or if SO_REUSEADDR is set and a multicast address is
        // bound on both new and duplicated sockets.
        if so.has_options(SO_REUSEADDR | SO_REUSEPORT) {
            reuseport = SO_REUSEADDR | SO_REUSEPORT;
        }
    } else if !in6_is_addr_unspecified(&sin6.sin6_addr) {
        // Cleared because ifa_ifwithaddr() compares the whole socket address, ports (and flow)
        // included.
        sin6.sin6_port = 0;
        sin6.sin6_flowinfo = 0;
        let mut ifa = None;
        if !so.has_options(SO_BINDANY) {
            let mut b = [0u8; size_of::<SockaddrIn6>()];
            let _ = pod::put(&mut b, 0, &*sin6);
            ifa = ifa_ifwithaddr(net, &b, inp.inp_rtableid.get());
            if ifa.is_none() {
                return Err(Errno::EADDRNOTAVAIL);
            }
        }
        sin6.sin6_port = lport;

        // bind to an anycast address might accidentally cause sending a packet with an
        // anycast source address, so we forbid it.
        //
        // We should allow to bind to a deprecated address, since the application dare to use
        // it. But, can we assume that they are careful enough to check if the address is
        // deprecated or not? Maybe, as a safeguard, we should have a setsockopt flag to
        // control the bind(2) behavior against deprecated addresses (default: forbid
        // bind(2)).
        if let Some(ia6) = ifa.and_then(In6Ifaddr::of)
            && ia6.ia6_flags.get()
                & (IN6_IFF_ANYCAST | IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED | IN6_IFF_DETACHED)
                != 0
        {
            return Err(Errno::EADDRNOTAVAIL);
        }
    }
    if lport != 0 {
        let table = inp.table();
        let laddr = Inpaddru::from_addr6(sin6.sin6_addr);
        if so.creds().0 != 0 && !in6_is_addr_multicast(&sin6.sin6_addr) {
            let t = in_pcblookup_local(
                g,
                table,
                &laddr,
                lport,
                INPLOOKUP_WILDCARD | INPLOOKUP_IPV6,
                inp.inp_rtableid.get(),
            );
            if t.is_some_and(|t| so.creds().0 != t.socket().creds().0) {
                return Err(Errno::EADDRINUSE);
            }
        }
        let t = in_pcblookup_local(g, table, &laddr, lport, wild, inp.inp_rtableid.get());
        if t.is_some_and(|t| reuseport & t.socket().options() == 0) {
            return Err(Errno::EADDRINUSE);
        }
    }
    Ok(())
}

/// `in6_pcbaddrisavail(inp, sin6, wild, p)`: [`in6_pcbaddrisavail_held`] taking the table
/// mutex (`IN_PCBLOCK_GRAB`).
pub fn in6_pcbaddrisavail<G: ProcGlue>(
    inp: &Inpcb<G>,
    sin6: &mut SockaddrIn6,
    wild: i32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut g = inp.table().lock();
    in6_pcbaddrisavail_held(&mut g, inp, sin6, wild, net)
}

/// `in6_pcbconnect(inp, nam)`: connects `inp` to the `sockaddr_in6` in `nam`; both address and
/// port must be specified. If the socket has no local address yet, one is picked (and a local
/// port, if it has none either). Eventually, flow labels will have to be dealt with here, as
/// well. `p` is `curproc` (the port pick's privilege), `net` the net lock's proof. The flow id
/// (`NSTOEPLITZ`) is `bsd`'s adapter's, after (`kernel::netinet::in_pcb`'s deviations).
pub fn in6_pcbconnect<G: ProcGlue>(
    inp: &Inpcb<G>,
    nam: &Mbuf,
    p: &Thread<G>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let table = inp.table();

    kassert!(inp.has_flags(INP_IPV6));

    // protect *sin6 from overwrites: a copy.
    let mut sin6 = in6_nam2sin6(nam)?;
    if sin6.sin6_port == 0 {
        return Err(Errno::EADDRNOTAVAIL);
    }
    // reject IPv4 mapped address, we have no support for it
    if in6_is_addr_v4mapped(&sin6.sin6_addr) {
        return Err(Errno::EADDRNOTAVAIL);
    }

    // The pcb's sticky options, lent for the scope and the source selection (which may
    // override their packet info's address, as the C does).
    let mut opts6 = inp.outputopts6();
    let im6o_ifidx = inp.moptions6_out().map(|o| o.im6o_ifidx);

    // KAME hack: embed scopeid
    let pi_ifindex = opts6
        .as_deref()
        .and_then(Ip6Pktopts::pktinfo)
        .map_or(0, |p| p.ipi6_ifindex);
    let mut addr = sin6.sin6_addr;
    if in6_embedscope(&mut addr, &sin6, pi_ifindex, im6o_ifidx.unwrap_or(0)).is_err() {
        return Err(Errno::EINVAL);
    }
    sin6.sin6_addr = addr;
    // this must be cleared for ifa_ifwithaddr()
    sin6.sin6_scope_id = 0;

    // Source address selection.
    // XXX: in6_selectsrc might replace the bound local address with the address specified by
    // setsockopt(IPV6_PKTINFO). Is it the intended behavior?
    let pi = opts6
        .as_deref_mut()
        .and_then(|o| o.ip6po_pktinfo.as_deref_mut());
    let in6a = in6_pcbselsrc(&sin6, inp, pi, im6o_ifidx, net)?;
    drop(opts6);

    let mut ip6 = inp.inp_ipv6.get::<Ip6Hdr>();
    ip6.ip6_hlim = in6_selecthlim(inp) as u8;
    inp.inp_ipv6.set(ip6);

    // keep lookup, modification, and rehash in sync
    let mut g = table.lock();

    let laddr6 = inp.inp_laddr6.get::<In6Addr>();
    let found = |g: &mut InpGuard<'_, G>, laddr: &In6Addr| {
        in6_pcblookup_held(
            g,
            table,
            &sin6.sin6_addr,
            sin6.sin6_port,
            laddr,
            inp.inp_lport.get(),
            inp.inp_rtableid.get(),
        )
        .is_some()
    };
    let unbound = in6_is_addr_unspecified(&laddr6);
    if found(&mut g, if unbound { &in6a } else { &laddr6 }) {
        return Err(Errno::EADDRINUSE);
    }

    kassert!(unbound || inp.inp_lport.get() != 0);

    if unbound {
        if inp.inp_lport.get() == 0 {
            in_pcbbind_locked(&mut g, inp, None, &Inpaddru::from_addr6(in6a), p, net)?;
            if found(&mut g, &in6a) {
                inp.set_lport(&g, 0);
                return Err(Errno::EADDRINUSE);
            }
        }
        inp.set_laddr6(&g, in6a);
    }
    inp.set_faddr6(&g, sin6.sin6_addr);
    inp.set_fport(&g, sin6.sin6_port);
    in_pcbrehash(&mut g, inp);
    drop(g);

    in6_pcb_newflowlabel(inp);
    Ok(())
}

/// A fresh random flow label for `inp` (`inp_flowinfo`'s low 20 bits), after the table mutex
/// (the tail `in6_pcbconnect` and `in6_pcbset_addr` share; the flow id is `bsd`'s adapter's).
fn in6_pcb_newflowlabel<G: ProcGlue>(inp: &Inpcb<G>) {
    let flowinfo = (inp.inp_flowinfo() & !IPV6_FLOWLABEL_MASK)
        | (htonl(ip6_randomflowlabel()) & IPV6_FLOWLABEL_MASK);
    inp.set_inp_flowinfo(flowinfo);
}

/// `in6_setsockaddr(inp, nam)`: the local address and port of `inp`, as the `struct
/// sockaddr_in6` the caller writes into the name mbuf (the scope id recovered). This services
/// the getsockname(2) call.
pub fn in6_setsockaddr<G: ProcGlue>(inp: &Inpcb<G>) -> SockaddrIn6 {
    let laddr6 = inp.inp_laddr6.get::<In6Addr>();
    let mut sin6 = SockaddrIn6 {
        sin6_port: inp.inp_lport.get(),
        ..SockaddrIn6::with_addr(laddr6)
    };
    // KAME hack: recover scopeid
    in6_recoverscope(&mut sin6, &laddr6);
    sin6
}

/// `in6_setpeeraddr(inp, nam)`: the foreign address and port of `inp`, as
/// [`in6_setsockaddr`]. This services the getpeername(2) call.
pub fn in6_setpeeraddr<G: ProcGlue>(inp: &Inpcb<G>) -> SockaddrIn6 {
    let faddr6 = inp.inp_faddr6.get::<In6Addr>();
    let mut sin6 = SockaddrIn6 {
        sin6_port: inp.inp_fport.get(),
        ..SockaddrIn6::with_addr(faddr6)
    };
    // KAME hack: recover scopeid
    in6_recoverscope(&mut sin6, &faddr6);
    sin6
}

/// `in6_pcbnotify(table, dst, fport_arg, src, lport_arg, rtable, cmd, cmdarg, notify)`:
/// passes control command `cmd` to every connection of `table` associated with address `dst`.
/// The local address and/or port numbers may be specified (`src`, `lport_arg`, `fport_arg`)
/// to limit the search; `src` is `None` for a notification of local fragmentation. The
/// "usual action" will be taken, depending on the ctlinput cmd: redirects and dead-host
/// indications go to every reference to the destination (`in_pcbrtchange` invalidates the
/// route cache of redirected ones). The caller must filter any cmds that are uninteresting
/// (e.g., no error in the map). `notify` reports the errno of `inet6ctlerrmap` to each
/// matching socket, with the socket locked; the C's `cmdarg`, which it does not use, is not
/// taken.
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn in6_pcbnotify<G: ProcGlue>(
    table: &Inpcbtable<G>,
    dst: &SockaddrIn6,
    fport_arg: u32,
    src: Option<&SockaddrIn6>,
    lport_arg: u32,
    rtable: u32,
    cmd: i32,
    notify: Option<&mut In6PcbNotify<'_, G>>,
) {
    let mut fport = fport_arg as u16;
    let mut lport = lport_arg as u16;

    if cmd as u32 >= PRC_NCMDS as u32 {
        return;
    }

    if in6_is_addr_unspecified(&dst.sin6_addr) {
        return;
    }
    if in6_is_addr_v4mapped(&dst.sin6_addr) {
        #[cfg(feature = "diagnostic")]
        crate::prf::printf(format_args!(
            "in6_pcbnotify: Huh?  Thought we never got called with mapped!\n"
        ));
        return;
    }

    // note that src can be NULL when we get notify by local fragmentation.
    let mut sa6_src = src.copied().unwrap_or(SA6_ANY);
    let flowinfo = sa6_src.sin6_flowinfo;

    // Redirects go to all references to the destination, and use in_pcbrtchange to invalidate
    // the route cache. Dead host indications: also use in_pcbrtchange to invalidate the cache,
    // and deliver the error to all the sockets. Otherwise, if we have knowledge of the local
    // port and address, deliver only to that socket.
    let rtchange = prc_is_redirect(cmd) && cmd != PRC_HOSTDEAD;
    if prc_is_redirect(cmd) || cmd == PRC_HOSTDEAD {
        fport = 0;
        lport = 0;
        sa6_src.sin6_addr = IN6ADDR_ANY;
    }
    let errno = INET6CTLERRMAP[cmd as usize];
    let mut rtchange_fn = in_pcbrtchange::<G>;
    let notify: &mut In6PcbNotify<'_, G> = match notify {
        _ if rtchange => &mut rtchange_fn,
        Some(n) => n,
        None => return,
    };

    let iter = core::pin::pin!(Inpcb::<G>::marker());
    let mut inp: Option<InpcbRef<G>> = None;
    let rdomain = rtable_l2(rtable);
    let mut g = table.lock();
    while let Some(i) = in_pcb_iterator(table, &mut g, inp.take(), iter.as_ref()) {
        kassert!(i.has_flags(INP_IPV6));
        let (laddr6, faddr6) = (i.inp_laddr6.get::<In6Addr>(), i.inp_faddr6.get::<In6Addr>());

        // A redirect or a dead host also reaches an unconnected pcb whose cached route is a
        // network route (not RTF_HOST) towards `dst`, whatever its addresses: the message then
        // comes from the next hop that pcb used last, and without this rule it would keep the
        // redirected route for good. (Cloning a host route per destination on output, as other
        // systems do, would let the redirect update that clone, at the price of a local denial
        // of service; icmp6_mtudisc_update() caps such clones.) The cache is `[s]`: it is read
        // once the pcb's socket is locked, below; the C reads it here, under the table mutex
        // alone, a race with the socket's owner.
        let by_route =
            (prc_is_redirect(cmd) || cmd == PRC_HOSTDEAD) && in6_is_addr_unspecified(&laddr6);
        // Whether the addresses select the pcb. With no ports given, a non-zero flow label and
        // an equal local address match (an error about an ESP-encrypted packet); else the
        // addresses and the ports given are compared.
        let by_addr = (lport == 0
            && fport == 0
            && flowinfo != 0
            && flowinfo == (i.inp_flowinfo() & IPV6_FLOWLABEL_MASK)
            && in6_are_addr_equal(&laddr6, &sa6_src.sin6_addr))
            || !(!in6_are_addr_equal(&faddr6, &dst.sin6_addr)
                || rtable_l2(i.inp_rtableid.get()) != rdomain
                || (lport != 0 && i.inp_lport.get() != lport)
                || (!in6_is_addr_unspecified(&sa6_src.sin6_addr)
                    && !in6_are_addr_equal(&laddr6, &sa6_src.sin6_addr))
                || (fport != 0 && i.inp_fport.get() != fport));
        if !by_route && !by_addr {
            inp = Some(i);
            continue;
        }

        drop(g);
        let so = in_pcbsolock(&i);
        if so.is_some() && (by_addr || cached_route_to(&i, &dst.sin6_addr)) {
            notify(&i, errno);
        }
        in_pcbsounlock(Some(&i), so);
        g = table.lock();
        inp = Some(i);
    }
    drop(g);
}

/// Whether `inp` caches a network (not host) route whose destination is `dst`: the redirect
/// clause of `in6_pcbnotify`, read under the pcb's socket lock.
fn cached_route_to<G: ProcGlue>(inp: &Inpcb<G>, dst: &In6Addr) -> bool {
    let ro = inp.route();
    ro.rt().is_some_and(|rt| rt.rt_flags.get() & RTF_HOST == 0)
        && in6_are_addr_equal(&ro_sin6(&ro).sin6_addr, dst)
}

/// `in6_pcbrtentry(inp)`: the route to the foreign address of `inp`, from its cache (refreshed
/// when stale), counted as `kernel::netinet::in_pcb::in_pcbrtentry`'s (the cache keeps its
/// own).
pub fn in6_pcbrtentry<G: ProcGlue>(inp: &Inpcb<G>, net: &NetHeld<'_>) -> Option<PoolArc<Rtentry>> {
    let faddr = inp.inp_faddr6.get::<In6Addr>();
    if in6_is_addr_unspecified(&faddr) {
        return None;
    }
    let laddr = inp.inp_laddr6.get::<In6Addr>();
    let mut ro = inp.route();
    ro.mpath6(
        &faddr.s6_addr,
        Some(&laddr.s6_addr),
        inp.inp_rtableid.get(),
        net,
    )?;
    ro.ro_rt.clone()
}

/// `in6_pcbhash_lookup`: the IPv6 control block with exactly these addresses, moved to the
/// head of its chain, counted; the table mutex held.
pub fn in6_pcbhash_lookup<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    hash: u64,
    rdomain: u32,
    faddr: &[u8; 16],
    fport: u16,
    laddr: &[u8; 16],
    lport: u16,
) -> Option<InpcbRef<G>> {
    hash_chain_find(g, hash, |inp| {
        kassert!(inp.has_flags(INP_IPV6));

        inp.inp_fport.get() == fport
            && inp.inp_lport.get() == lport
            && inp.inp_faddr6.bytes() == *faddr
            && inp.inp_laddr6.bytes() == *laddr
            && rtable_l2(inp.inp_rtableid.get()) == rdomain
    })
}

/// `in6_pcblookup_lock(.., IN_PCBLOCK_HOLD)`: the connected IPv6 control block for
/// `faddr.fport <-> laddr.lport`, counted; no wildcard matching is done. The table mutex
/// held.
pub fn in6_pcblookup_held<G: ProcGlue, A: Addr6>(
    g: &mut InpGuard<'_, G>,
    table: &Inpcbtable<G>,
    faddr: &A,
    fport: u16,
    laddr: &A,
    lport: u16,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let (faddr, laddr) = (faddr.to_bytes(), laddr.to_bytes());
    let rdomain = rtable_l2(rtable);
    let hash = in6_pcbhash(table, rdomain, &faddr, fport, &laddr, lport);
    in6_pcbhash_lookup(g, hash, rdomain, &faddr, fport, &laddr, lport)
}

/// `in6_pcblookup`: [`in6_pcblookup_held`] taking the table mutex (`IN_PCBLOCK_GRAB`).
pub fn in6_pcblookup<G: ProcGlue, A: Addr6>(
    table: &Inpcbtable<G>,
    faddr: &A,
    fport: u16,
    laddr: &A,
    lport: u16,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let (faddr, laddr) = (faddr.to_bytes(), laddr.to_bytes());
    let rdomain = rtable_l2(rtable);
    let hash = in6_pcbhash(table, rdomain, &faddr, fport, &laddr, lport);
    let mut g = table.lock();
    in6_pcbhash_lookup(&mut g, hash, rdomain, &faddr, fport, &laddr, lport)
}

/// `in6_pcblookup_listen`: the listening IPv6 control block for `laddr.lport_arg`:
/// unspecified foreign address and port, bound to `laddr` or to the wildcard address (pf's
/// divert-to key, or the wildcard first for a connection redirected to localhost, from `m`),
/// counted.
pub fn in6_pcblookup_listen<G: ProcGlue, A: Addr6>(
    table: &Inpcbtable<G>,
    laddr: &A,
    lport_arg: u16,
    m: Option<&Mbuf>,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let l = in6_pcblookup_listen_keys(table, laddr, lport_arg, m, rtable)?;
    let mut g = table.lock();
    in6_pcblookup_listen_held(&mut g, table, &l)
}

/// What `in6_pcblookup_listen` computes before it takes the table mutex (in6_pcb.c's order,
/// as `kernel::netinet::in_pcb::ListenLookup`): the two local addresses, the port, the
/// routing domain and the first address's hash.
pub struct Listen6Lookup {
    key1: [u8; 16],
    key2: [u8; 16],
    lport: u16,
    rdomain: u32,
    hash1: u64,
}

/// [`Listen6Lookup`] of `laddr.lport_arg` in `table`; `None` for a divert-reply packet.
pub fn in6_pcblookup_listen_keys<G: ProcGlue, A: Addr6>(
    table: &Inpcbtable<G>,
    laddr: &A,
    lport_arg: u16,
    m: Option<&Mbuf>,
    rtable: u32,
) -> Option<Listen6Lookup> {
    let laddr = laddr.to_bytes();
    let zero = ZEROIN6_ADDR.s6_addr;
    let mut key1 = laddr;
    let mut key2 = zero;
    let mut lport = lport_arg;

    if let Some(m) = m
        && m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED != 0
    {
        let divert = pf().and_then(|pf| pf.find_divert(m));
        kassert!(divert.is_some());
        let divert = divert?;
        match divert.type_ {
            PF_DIVERT_TO => {
                key1 = divert.addr;
                key2 = key1;
                lport = divert.port;
            }
            PF_DIVERT_REPLY => return None,
            t => kpanic(format_args!(
                "in6_pcblookup_listen: unknown divert type {t}, mbuf {m:p}"
            )),
        }
    } else if let Some(m) = m
        && m.m_pkthdr().pf.flags.get() & PF_TAG_TRANSLATE_LOCALHOST != 0
    {
        // Redirected connections should not be treated the same as connections directed to
        // ::1 since localhost can only be accessed from the host itself.
        key1 = zero;
        key2 = laddr;
    }
    let rdomain = rtable_l2(rtable);
    let hash1 = in6_pcbhash(table, rdomain, &zero, 0, &key1, lport);
    Some(Listen6Lookup {
        key1,
        key2,
        lport,
        rdomain,
        hash1,
    })
}

/// `in6_pcblookup_listen`'s lookups of `l`, the table mutex held: the pcb bound to the first
/// key (hashed before the mutex), else to the second (hashed here, as the C), counted.
pub fn in6_pcblookup_listen_held<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    table: &Inpcbtable<G>,
    l: &Listen6Lookup,
) -> Option<InpcbRef<G>> {
    let zero = ZEROIN6_ADDR.s6_addr;
    let &Listen6Lookup {
        key1,
        key2,
        lport,
        rdomain,
        hash1,
    } = l;
    let mut inp = in6_pcbhash_lookup(g, hash1, rdomain, &zero, 0, &key1, lport);
    if inp.is_none() && key1 != key2 {
        let hash = in6_pcbhash(table, rdomain, &zero, 0, &key2, lport);
        inp = in6_pcbhash_lookup(g, hash, rdomain, &zero, 0, &key2, lport);
    }
    inp
}

/// `in6_pcbset_addr(inp, fsin6, lsin6, rtableid)`: gives `inp` both addresses and ports at
/// once (a connection accepted from a listener) in routing table `rtableid`, unless another
/// control block has them; the IPv6 half of `in_pcbset_addr` (the flow id `bsd`'s adapter's).
pub fn in6_pcbset_addr<G: ProcGlue>(
    inp: &Inpcb<G>,
    fsin6: &SockaddrIn6,
    lsin6: &SockaddrIn6,
    rtableid: u32,
) -> Result<(), Errno> {
    let table = inp.table();

    let mut g = table.lock();
    if in6_pcblookup_held(
        &mut g,
        table,
        &fsin6.sin6_addr,
        fsin6.sin6_port,
        &lsin6.sin6_addr,
        lsin6.sin6_port,
        rtableid,
    )
    .is_some()
    {
        return Err(Errno::EADDRINUSE);
    }

    inp.set_rtableid(&g, rtableid);
    inp.set_laddr6(&g, lsin6.sin6_addr);
    inp.set_lport(&g, lsin6.sin6_port);
    inp.set_faddr6(&g, fsin6.sin6_addr);
    inp.set_fport(&g, fsin6.sin6_port);
    in_pcbrehash(&mut g, inp);
    drop(g);

    in6_pcb_newflowlabel(inp);
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the IPv6 halves N6.5c moved here: `in_pcbbind_locked`'s `INP_IPV6`
    // branch (`in6_pcbaddrisavail`'s checks of the address and the port, the address and port
    // getsockname(2) then reads), `in6_pcbset_addr`'s conflict check and the commands
    // `in6_pcbnotify` passes on. `bsd`'s adapter keeps LZ's tests (the lookups, the scope
    // zones, the matching, the route cache) over these functions.

    use std::vec::Vec;

    use hal::netlock::NETLOCK;
    use hal::sys::endian::htons;

    use super::*;
    use crate::net::mbuf::{M_DONTWAIT, MT_SONAME, MbufBox, m_get};
    use crate::net::proto::{PRC_MSGSIZE, PRC_REDIRECT_HOST};
    use crate::net::socket::SocketRef;
    use crate::netinet::in_pcb::{in_pcbbind, testinet};
    use crate::netinet6::in6::tests::{a6, iface, test_ia6};

    type Tg = testinet::G;

    /// `addr.port` (host order).
    fn sin6(addr: In6Addr, port: u16) -> SockaddrIn6 {
        SockaddrIn6 {
            sin6_port: htons(port),
            ..SockaddrIn6::with_addr(addr)
        }
    }

    /// A name mbuf holding `sin6`, as `sys_bind` copies one in.
    fn nam(sin6: SockaddrIn6) -> MbufBox {
        let mut m = m_get(M_DONTWAIT, i32::from(MT_SONAME)).expect("an mbuf");
        m.m_len().set(size_of::<SockaddrIn6>() as u32);
        assert!(pod::put(m.data_mut().expect("writable"), 0, &sin6));
        m
    }

    /// An `INP_IPV6` control block of the test protocol in `table`.
    fn pcb6(table: &'static Inpcbtable<Tg>) -> (SocketRef<Tg>, InpcbRef<Tg>) {
        let (so, inp) = testinet::pcb(table);
        inp.set_flags(INP_IPV6);
        (so, inp)
    }

    /// bind(2) of `inp` to `addr.port` under the net lock.
    fn bind(inp: &Inpcb<Tg>, p: &Thread<Tg>, addr: In6Addr, port: u16) -> Result<(), Errno> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        in_pcbbind(inp, Some(&nam(sin6(addr, port))), p, &net)
    }

    #[test]
    fn an_ipv6_bind_checks_its_address_and_port_as_the_c() {
        let (_g, p) = testinet::setup();
        let (ifp, _d) = iface(b"i6p0", [0x52, 0x54, 0, 0xcc, 0, 1]);
        let ours = a6("2001:db8::1");
        let tentative = a6("2001:db8::2");
        test_ia6(ifp, ours, 0);
        test_ia6(ifp, tentative, IN6_IFF_TENTATIVE);
        let table = testinet::table(16);
        use Errno::{EADDRINUSE, EADDRNOTAVAIL, EINVAL};

        let (sa, a) = pcb6(table);
        assert_eq!(
            bind(&a, p, a6("2001:db8::99"), 8080),
            Err(EADDRNOTAVAIL),
            "not ours"
        );
        assert_eq!(
            bind(&a, p, a6("::ffff:a00:202"), 8080),
            Err(EADDRNOTAVAIL),
            "mapped"
        );
        assert_eq!(
            bind(&a, p, tentative, 8080),
            Err(EADDRNOTAVAIL),
            "tentative"
        );
        {
            // A name of another length.
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            let m = nam(sin6(ours, 8080));
            m.m_len().set(16);
            assert_eq!(
                in_pcbbind(&a, Some(&m), p, &net),
                Err(EINVAL),
                "a short name"
            );
        }
        assert_eq!(
            in6_setsockaddr(&a).sin6_port,
            0,
            "nothing bound by the refusals"
        );

        bind(&a, p, ours, 8080).expect("our address");
        let got = in6_setsockaddr(&a);
        assert_eq!((got.sin6_addr, got.sin6_port), (ours, htons(8080)));
        assert_eq!(bind(&a, p, ours, 8081), Err(EINVAL), "bound already");

        let (sb, b) = pcb6(table);
        assert_eq!(
            bind(&b, p, ours, 8080),
            Err(EADDRINUSE),
            "the port is taken"
        );
        bind(&b, p, IN6ADDR_ANY, 0).expect("a port picked");
        let got = in6_setsockaddr(&b);
        assert!(in6_is_addr_unspecified(&got.sin6_addr), "the wildcard");
        assert_ne!(got.sin6_port, 0, "a port picked");

        testinet::release(sa, a);
        testinet::release(sb, b);
    }

    #[test]
    fn in6_pcbset_addr_and_the_commands_in6_pcbnotify_passes_on() {
        let (_g, _p) = testinet::setup();
        let table = testinet::table(16);
        let (f, l) = (a6("2001:db8::2"), a6("2001:db8::1"));

        let (sa, a) = pcb6(table);
        let (sb, b) = pcb6(table);
        in6_pcbset_addr(&a, &sin6(f, 2000), &sin6(l, 1000), 0).expect("a free quadruple");
        assert_eq!(
            in6_pcbset_addr(&b, &sin6(f, 2000), &sin6(l, 1000), 0),
            Err(Errno::EADDRINUSE)
        );
        let peer = in6_setpeeraddr(&a);
        assert_eq!((peer.sin6_addr, peer.sin6_port), (f, htons(2000)));
        let sock = in6_setsockaddr(&a);
        assert_eq!((sock.sin6_addr, sock.sin6_port), (l, htons(1000)));

        // The errors of a command about `dst`, as a softnet's ctlinput (the shared net lock).
        let notify = |cmd: i32, dst: In6Addr| -> Vec<Option<Errno>> {
            let mut got = Vec::new();
            let mut record = |inp: &Inpcb<Tg>, e: Option<Errno>| {
                assert!(core::ptr::eq(inp, &*a), "the connection to {dst:?}");
                got.push(e);
            };
            let _r = NETLOCK.read();
            in6_pcbnotify(
                table,
                &SockaddrIn6::with_addr(dst),
                u32::from(htons(2000)),
                Some(&sin6(l, 1000)),
                u32::from(htons(1000)),
                0,
                cmd,
                Some(&mut record),
            );
            got
        };
        assert_eq!(notify(PRC_MSGSIZE, f), [Some(Errno::EMSGSIZE)]);
        assert!(notify(PRC_NCMDS as i32, f).is_empty(), "no such command");
        assert!(
            notify(PRC_MSGSIZE, IN6ADDR_ANY).is_empty(),
            "no destination"
        );
        assert!(
            notify(PRC_MSGSIZE, a6("::ffff:a00:202")).is_empty(),
            "a mapped address"
        );
        assert!(
            notify(PRC_MSGSIZE, a6("2001:db8::3")).is_empty(),
            "another destination"
        );
        assert!(
            notify(PRC_REDIRECT_HOST, f).is_empty(),
            "a redirect is in_pcbrtchange's"
        );

        testinet::release(sa, a);
        testinet::release(sb, b);
    }
}
/* </TESTS> */
