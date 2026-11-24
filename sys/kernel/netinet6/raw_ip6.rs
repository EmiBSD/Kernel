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
 * Copyright (C) 2001 WIDE Project.
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
 *	@(#)raw_ip.c	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! Raw IPv6 sockets: `netinet6/raw_ip6.c` (its statistics' image is `hal::sys::
//! netinet6_raw_ip6`). A `SOCK_RAW` socket of the inet6 domain (ping6's `IPPROTO_ICMPV6`
//! one, or any protocol number) has a control block in `rawin6pcbtable`. `rip6_input`, the
//! input of every protocol without a handler of its own and the end of `icmp6_input`, hands
//! a copy of each datagram, the headers before `*offp` stripped, to every raw socket whose
//! protocol, addresses, ICMPv6 filter and (with `IPV6_CHECKSUM`) checksum match.
//! `rip6_output` prepends an IPv6 header and, for ICMPv6 or with `IPV6_CHECKSUM`, computes
//! the checksum at its offset, then sends the datagram with `ip6_output`.
//!
//! Upstream: sys/netinet6/raw_ip6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/raw_ip6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/raw_ip6.rs@3c62ede68802
//!
//! Locks: as raw IP's (`kernel::netinet::raw_ip`): the requests hold the socket's lock and
//! borrow the pcb from its locked slot; input holds none, reading each pcb's `[I]`, `[t]` and
//! atomic members (the ICMPv6 filter's words among them) under the table's mutex.
//!
//! ## Deviations
//! - `rip6counters` (`struct cpumem *`) is the per-CPU [`RIP6COUNTERS`]; `counters_alloc`
//!   in `rip6_init` has nothing left to do.
//! - `ip6counters`' `counters_enter` pair (`ip6s_delivered` down) is one counter update.
//! - `rip6_send` without a packet (which `sosend` never does) fails with `EINVAL`, as
//!   `rip_send` does (LZ's).
//! - `MROUTING` (`ip6_mroute.c`) is not configured: the `MRT6_*` options go to
//!   `ip6_ctloutput` as any other, and `rip6_detach` has no `ip6_mrouter_done`. `NPF` is
//!   configured: the divert-to key and `pf_mbuf_link_inpcb`. `SMALL_KERNEL` is not set:
//!   `rip6_sysctl` is here.
//!
//! ## Redesign
//! - N6.6b1: LZ's raw IPv6 was `bsd`'s, over `&'static` sockets and pcbs, parked mbufs and
//!   adapters that lent the net lock's proof per call. Here it is the kernel's, generic over
//!   the glue, in raw IP's shape: [`Rip6Usrreqs`] (`rip6_usrreqs`) borrow the pcb from the
//!   socket's locked slot (no count per request), input walks `rawin6pcbtable` (the glue's,
//!   [`rawin6pcbtable`]) with `in_pcb_iterator`, the prototype's next header one word's load.
//! - D-7: the ICMPv6 filter is the pcb's words ([`Inpcb::icmp6filt`]), set to pass-all by
//!   [`rip6_attach`] before input can see the pcb; input reads the one word a type's bit is in
//!   (the C's unlocked read), and the filter goes with the pcb, not in `rip6_detach`. The C's
//!   `malloc(M_PCB)` per socket and `rip6_attach`'s `ENOMEM` path (which `socreate`'s waiting
//!   attach never reached) have no counterpart: `vmstat -m` shows one `pcb` allocation fewer
//!   per raw IPv6 socket, and the "inpcb" item 24 bytes more for every internet socket.
//! - `rip6_output` takes one loan of the pcb's sticky options for the whole send, from
//!   `ip6_setpktopts` to `ip6_output`'s return (LZ lent them three or four times), and one
//!   copy of the multicast options; the checksum is written at its offset through the mbuf
//!   that holds it (`ip6_set_bytes`, after `m_pulldown` made the two bytes contiguous, as the
//!   C's pointer write).
//! - `rip6_ctlinput` is a safe function of the control input's view and its `ip6ctlparam`
//!   ([`CtlArg::Ip6`]), the caller's net lock proved by `net`; LZ's was an `unsafe fn` over
//!   the raw `pr_ctlinput` contract.
//! - The checksum bounds keep the C's integer order: `m_pkthdr.len < *offp + 2` and
//!   `len - *offp - 2 < inp_cksum6` before `in6_cksum` on input, `plen < 2 || plen - 2 < off`
//!   (an unsigned subtraction) on output: a user's offset never indexes past the packet.
//! - Cost against the C (measured at the N6 close by the `rip6.input.filter` bench row,
//!   decision 50): a send pays one sticky options loan and one route loan (two atomics
//!   each, D-10 of N6.6b, N12) and the multicast options' copy, where the C reads the
//!   members under the socket lock; input pays the IPv6 header's copy (`ip6_load`) and a
//!   dynamic call to pf's `find_divert` (N8), and the filter two loads (the cell's flag,
//!   then the word) where the C loads a pointer and a word.

use core::marker::PhantomData;
use core::mem::{offset_of, size_of};
use core::pin::pin;
use core::sync::atomic::Ordering;

use hal::netlock::NetHeld;
use hal::percpu::Counters;
use hal::pod;
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::netinet_icmp6::{
    ICMP6_PARAM_PROB, ICMP6_PARAMPROB_NEXTHEADER, Icmp6Hdr, Icmp6statCounters,
};
use hal::sys::netinet_in::{IPPROTO_DONE, IPPROTO_ICMPV6, IPPROTO_IPV6, IPPROTO_MAX, IPPROTO_NONE};
use hal::sys::netinet_ip6::{IPV6_FLOWINFO_MASK, IPV6_VERSION, IPV6_VERSION_MASK, Ip6Hdr};
use hal::sys::netinet6_in6::{
    IPV6_CHECKSUM, In6Addr, SA6_ANY, SockaddrIn6, in6_are_addr_equal, in6_is_addr_unspecified,
    in6_is_addr_v4mapped,
};
use hal::sys::netinet6_ip6_var::Ip6statCounters;
use hal::sys::netinet6_raw_ip6::{RIP6S_NCOUNTERS, RIPV6CTL_STATS, Rip6stat};
use hal::sys::socket::AF_INET6;
use hal::user::bytes_of;

use crate::assert::{kassert, kpanic};
use crate::net::domain::net;
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::mbuf::{
    M_COPYALL, M_DONTWAIT, M_MCAST, Mbuf, MbufBox, PF_TAG_DIVERTED, m_adj, m_copym, m_prepend,
    m_pullup,
};
use crate::net::mbuf2::m_pulldown;
use crate::net::pf_hooks::{PF_DIVERT_REPLY, PF_DIVERT_TO, pf};
use crate::net::proto::{
    CtlArg, PRC_HOSTDEAD, PRC_MSGSIZE, PRC_NCMDS, SockaddrView, Usrreqs, prc_is_redirect,
};
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::{SS_CANTRCVMORE, SS_ISCONNECTED, SS_PRIV};
use crate::net::socket::{
    SoLocked, Socket, socantsendmore, soisconnected, soisdisconnected, soreserve, sorwakeup,
};
use crate::netinet::in_pcb::inet;
use crate::netinet::in_pcb::{
    IN6P_CONTROLOPTS, IN6P_MINMTU, INP_IPV6, Inpcb, InpcbRef, Inpcbtable, detach_slot,
    in_pcb_iterator, in_pcballoc, in_pcbinit, in_pcbrtchange, pcb_of,
};
use crate::netinet::raw_ip::sa_bytes;
use crate::netinet6::icmp6::{icmp6_ctloutput, icmp6_error, icmp6_mtudisc_update};
use crate::netinet6::in6::{in6_control, in6_nam2sin6};
use crate::netinet6::in6_cksum::in6_cksum;
use crate::netinet6::in6_pcb::{
    in6_pcbaddrisavail, in6_pcblookup, in6_pcbnotify, in6_peeraddr, in6_sockaddr,
};
use crate::netinet6::in6_src::{in6_embedscope, in6_pcbselsrc, in6_recoverscope, in6_selecthlim};
use crate::netinet6::ip6_input::{
    INET6CTLERRMAP, ip6_exthdr_bytes, ip6_exthdr_get, ip6_get_prevhdr, ip6_savecontrol,
    ip6_set_bytes,
};
use crate::netinet6::ip6_output::{
    ip6_clearpktopts, ip6_ctloutput, ip6_load, ip6_output, ip6_raw_ctloutput, ip6_setpktopts,
    ip6_store,
};
use crate::netinet6::ip6_var::{IPV6_MINMTU, Ip6Pktopts, icmp6stat_inc_hist, ip6stat_dec};
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::{SysctlReq, rdstruct};

/// `RIPV6SNDQ` (`in6_proto.c`).
const RIPV6SNDQ: u64 = 8192;
/// `RIPV6RCVQ`.
const RIPV6RCVQ: u64 = 8192;
/// `rip6_sendspace` (`in6_proto.c`'s `u_long`, no sysctl name).
pub const RIP6_SENDSPACE: u64 = RIPV6SNDQ;
/// `rip6_recvspace`.
pub const RIP6_RECVSPACE: u64 = RIPV6RCVQ;

/// `sizeof(struct ip6_hdr)`.
const IP6_HDR_LEN: usize = size_of::<Ip6Hdr>();

/// `enum rip6stat_counters`: the indices of [`RIP6COUNTERS`], one per member of
/// [`Rip6stat`] in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Rip6statCounters {
    /// `rip6s_ipackets`.
    Rip6sIpackets,
    /// `rip6s_isum`.
    Rip6sIsum,
    /// `rip6s_badsum`.
    Rip6sBadsum,
    /// `rip6s_nosock`.
    Rip6sNosock,
    /// `rip6s_nosockmcast`.
    Rip6sNosockmcast,
    /// `rip6s_fullsock`.
    Rip6sFullsock,
    /// `rip6s_opackets`.
    Rip6sOpackets,
}

/// `rip6_usrreqs`: raw interface to IP6 protocol (the raw entries', ICMPv6's and the IPv6
/// rows of IPsec's and IP-in-IP's user requests).
pub struct Rip6Usrreqs<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Rip6Usrreqs<G> {
    /// `rip6_usrreqs`.
    pub const NEW: Self = Self(PhantomData);
}

/// `rip6counters`.
pub static RIP6COUNTERS: Counters<RIP6S_NCOUNTERS> = Counters::new();

/// `rip6stat_inc(c)`.
#[inline]
pub fn rip6stat_inc(c: Rip6statCounters) {
    RIP6COUNTERS.inc(c as usize);
}

/// `rawin6pcbtable`: the glue's raw IPv6 control blocks (`NetTables`; `kern.file`'s walk
/// reads it).
pub fn rawin6pcbtable<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().rawin6pcbtable
}

/// `ICMP6_FILTER_WILLBLOCK(type, inp->inp_icmp6filt)` over the pcb's filter words: the one
/// word the type's bit is in, loaded once (the C reads it without a lock).
#[inline]
fn willblock(filt: &[core::sync::atomic::AtomicU32; 8], type_: u8) -> bool {
    filt[usize::from(type_ >> 5)].load(Ordering::Relaxed) & (1 << (type_ & 31)) == 0
}

/// `rip6_init`: initialize raw connection block queue.
pub fn rip6_init<G: ProcGlue>() {
    in_pcbinit(rawin6pcbtable::<G>(), 1);
}

/// `rip6_input(mp, offp, proto, af, ns)`: hands a copy of datagram `*mp`, its headers before
/// `*offp` stripped, to every raw IPv6 socket that matches it, under the net lock (`net`);
/// without one, an ICMPv6 message or a `IPPROTO_NONE` packet is dropped and any other is
/// answered with a parameter problem at its next header field.
#[allow(
    clippy::too_many_arguments,
    reason = "pr_input's arguments and the net lock's proof"
)]
pub fn rip6_input<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    proto: i32,
    af: i32,
    _ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let Some(m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let off = *offp;

    kassert!(af == i32::from(AF_INET6));

    let (mut m, type_) = if proto == IPPROTO_ICMPV6 {
        let Some((m, at, o)) = ip6_exthdr_get(m, off, size_of::<Icmp6Hdr>() as i32) else {
            return IPPROTO_DONE;
        };
        let Some(&t) = ip6_exthdr_bytes(&m, at, o, 1).and_then(|b| b.first()) else {
            kpanic(format_args!(
                "rip6_input: no ICMPv6 type after ip6_exthdr_get"
            ));
        };
        (m, t)
    } else {
        rip6stat_inc(Rip6statCounters::Rip6sIpackets);
        (m, 0)
    };
    let ip6 = ip6_load(&m);
    let table = rawin6pcbtable::<G>();
    let iter = pin!(Inpcb::<G>::marker());

    let mut rip6src = SockaddrIn6 {
        sin6_family: AF_INET6,
        sin6_len: size_of::<SockaddrIn6>() as u8,
        ..SockaddrIn6::default()
    };
    // KAME hack: recover scopeid
    in6_recoverscope(&mut rip6src, &ip6.ip6_src);

    let mut key = ip6.ip6_dst;
    if m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED != 0 {
        let divert = pf().and_then(|p| p.find_divert(&m));
        kassert!(divert.is_some());
        if let Some(divert) = divert {
            match divert.type_ {
                PF_DIVERT_TO => {
                    key = In6Addr {
                        s6_addr: divert.addr,
                    }
                }
                PF_DIVERT_REPLY => {}
                t => kpanic(format_args!(
                    "rip6_input: unknown divert type {t}, mbuf {:p}",
                    &*m
                )),
            }
        }
    }

    let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());
    let len = i64::from(m.m_pkthdr().len.get());
    let mut g = table.lock();
    let mut inp: Option<InpcbRef<G>> = None;
    let mut last: Option<InpcbRef<G>> = None;
    while let Some(i) = in_pcb_iterator(table, &mut g, inp.take(), iter.as_ref()) {
        kassert!(i.has_flags(INP_IPV6));

        // Packet must not be inserted after disconnected wakeup call. To avoid race, check
        // again when holding receive buffer mutex.
        let nxt = i32::from(i.inp_ipv6.ip6_nxt());
        let mut skip = i.socket().so_rcv.has_state(SS_CANTRCVMORE)
            || rtable_l2(i.inp_rtableid.get()) != rdomain
            || ((nxt != 0 || proto == IPPROTO_ICMPV6) && nxt != proto)
            || {
                let l = i.inp_laddr6.get::<In6Addr>();
                !in6_is_addr_unspecified(&l) && !in6_are_addr_equal(&l, &key)
            }
            || {
                let f = i.inp_faddr6.get::<In6Addr>();
                !in6_is_addr_unspecified(&f) && !in6_are_addr_equal(&f, &ip6.ip6_src)
            }
            || (proto == IPPROTO_ICMPV6 && i.icmp6filt().is_some_and(|f| willblock(f, type_)));
        let cksum6 = i.inp_cksum6.get();
        if !skip && proto != IPPROTO_ICMPV6 && cksum6 != -1 {
            rip6stat_inc(Rip6statCounters::Rip6sIsum);
            // Although in6_cksum() does not need the position of the checksum field for
            // verification, enforce that it is located within the packet. Userland has given
            // a checksum offset, a packet too short for that is invalid. Avoid overflow with
            // user supplied offset.
            if len < i64::from(off) + 2
                || len - i64::from(off) - 2 < i64::from(cksum6)
                || in6_cksum(&m, proto as u8, off as u32, (len - i64::from(off)) as u32) != 0
            {
                rip6stat_inc(Rip6statCounters::Rip6sBadsum);
                skip = true;
            }
        }
        if skip {
            inp = Some(i);
            continue;
        }

        if let Some(l) = last.take() {
            drop(g);

            if let Some(n) = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT) {
                rip6_sbappend(&l, n, off, &rip6src);
            }
            drop(l); // in_pcbunref(last)

            g = table.lock();
        }
        last = Some(i.clone()); // in_pcbref(inp)
        inp = Some(i);
    }
    drop(g);

    let Some(last) = last else {
        if proto != IPPROTO_ICMPV6 {
            rip6stat_inc(Rip6statCounters::Rip6sNosock);
            if m.m_flags().get() & M_MCAST != 0 {
                rip6stat_inc(Rip6statCounters::Rip6sNosockmcast);
            }
        }
        if proto == IPPROTO_NONE || proto == IPPROTO_ICMPV6 {
            drop(m);
        } else {
            let prvnxt = ip6_get_prevhdr(&m, off);
            icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_NEXTHEADER, prvnxt, net);
        }
        ip6stat_dec(Ip6statCounters::Ip6sDelivered);

        return IPPROTO_DONE;
    };

    rip6_sbappend(&last, m, off, &rip6src);
    drop(last); // in_pcbunref(last)

    IPPROTO_DONE
}

/// `rip6_sbappend`: queues datagram `m` from `rip6src`, its first `hlen` bytes (the headers
/// before the payload) stripped, with the control messages its socket asked for, on the
/// receive buffer of `inp`'s socket; the mutex is let go before a refused datagram is freed.
fn rip6_sbappend<G: ProcGlue>(inp: &Inpcb<G>, m: MbufBox, hlen: i32, rip6src: &SockaddrIn6) {
    let so = inp.socket();
    let opts = if inp.has_flags(IN6P_CONTROLOPTS) {
        ip6_savecontrol(inp, &m)
    } else {
        None
    };
    // strip intermediate headers
    m_adj(&m, hlen);

    let mut rg = so.so_rcv.lock();
    let r = if so.so_rcv.has_state(SS_CANTRCVMORE) {
        Err((Some(m), opts))
    } else {
        rg.sbappendaddr(sa_bytes(rip6src), Some(m), opts)
    };
    drop(rg);

    match r {
        Ok(()) => sorwakeup(so),
        Err((m, opts)) => {
            drop(m);
            drop(opts);
            rip6stat_inc(Rip6statCounters::Rip6sFullsock);
        }
    }
}

/// `rip6_ctlinput(cmd, sa, rdomain, d)`: a control input for the raw IPv6 sockets, under the
/// net lock (`net`): a redirect or a dead host notifies every pcb of the destination (their
/// route caches let go, `in_pcbrtchange`); a Packet Too Big also updates the path MTU, the
/// raw socket of the quoted packet's protocol, if any, validating it; other commands without
/// an error in `inet6ctlerrmap` are ignored.
pub fn rip6_ctlinput<G: ProcGlue>(
    cmd: i32,
    sa: SockaddrView<'_>,
    rdomain: u32,
    arg: CtlArg<'_>,
    net: &NetHeld<'_>,
) {
    if sa.family() != AF_INET6 || usize::from(sa.len()) != size_of::<SockaddrIn6>() {
        return;
    }
    let Some(sa6) = pod::at::<SockaddrIn6>(sa.bytes(), 0) else {
        return;
    };
    // The argument of this family only (an IPv4 quote is never handed to an IPv6 entry).
    let mut d = match arg {
        CtlArg::Ip6(p) => Some(p),
        CtlArg::None => None,
        CtlArg::Ip(_) => return,
    };

    if cmd as u32 as usize >= PRC_NCMDS {
        return;
    }
    if prc_is_redirect(cmd) || cmd == PRC_HOSTDEAD {
        d = None;
    } else if cmd == PRC_MSGSIZE {
        // special code is present, see below
    } else if INET6CTLERRMAP[cmd as usize].is_none() {
        return;
    }

    // if the parameter is from icmp6, decode it.
    let (sa6_src, nxt) = d.map_or((SA6_ANY, -1), |p| (p.src, i32::from(p.nxt)));

    if let Some(p) = d
        && cmd == PRC_MSGSIZE
    {
        // Check to see if we have a valid raw IPv6 socket corresponding to the address in
        // the ICMPv6 message payload, and the protocol (ip6_nxt) meets the socket. XXX chase
        // extension headers, or pass final nxt value from icmp6_notify_error()
        let inp = in6_pcblookup(
            rawin6pcbtable::<G>(),
            &sa6.sin6_addr,
            0,
            &sa6_src.sin6_addr,
            0,
            rdomain,
        );
        let valid = inp.as_ref().is_some_and(|inp| {
            let n = i32::from(inp.inp_ipv6.ip6_nxt());
            n != 0 && n == nxt
        });

        // Depending on the value of "valid" and routing table size (mtudisc_{hi,lo}wat), we
        // will:
        // - recalculate the new MTU and create the corresponding routing entry, or
        // - ignore the MTU change notification.
        icmp6_mtudisc_update(p, valid, net);
        drop(inp); // in_pcbunref(inp)

        // regardless of if we called icmp6_mtudisc_update(), we need to call
        // in6_pcbnotify(), to notify path MTU change to the userland (2292bis-02), because
        // some unconnected sockets may share the same destination and want to know the path
        // MTU.
    }

    let mut notify = |inp: &Inpcb<G>, _w: &mut SoLocked<'_>, e: Option<Errno>| {
        in_pcbrtchange(inp, e);
    };
    in6_pcbnotify(
        rawin6pcbtable::<G>(),
        &sa6,
        0,
        Some(&sa6_src),
        0,
        rdomain,
        cmd,
        net,
        Some(&mut notify),
    );
}

/// `rip6_output(m, so, dstaddr, control)`: generates an IPv6 header and passes the packet to
/// `ip6_output`, with the options the control messages give (`control`, disposed of here) or
/// the pcb's sticky ones; for ICMPv6, or with `IPV6_CHECKSUM`, the checksum at its offset
/// first. `inp` is the socket's pcb, borrowed under its lock; `net` the net lock's proof.
pub fn rip6_output<G: ProcGlue>(
    m: MbufBox,
    so: &Socket<G>,
    inp: &Inpcb<G>,
    dstaddr: &SockaddrIn6,
    control: Option<MbufBox>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let plen = m.m_pkthdr().len.get() as u32;
    let proto = i32::from(so.proto().protocol());
    let priv_ = so.has_state(SS_PRIV);
    // inp->inp_outputopts6, lent once from ip6_setpktopts to ip6_output's return; the
    // multicast options' members ip6_output and the scope read, copied once.
    let mut sticky = inp.outputopts6();
    let mopts = inp.moptions6_out();
    let mut opt = Ip6Pktopts::default();

    let result = 'out: {
        // optp: the control messages' options (`opt`), else the pcb's sticky ones, borrowed
        // from the one loan at each use below.
        if let Some(c) = &control
            && let Err(e) = ip6_setpktopts(c, &mut opt, sticky.as_deref(), priv_, proto)
        {
            break 'out Err(e);
        }
        let ctl = control.is_some();

        if dstaddr.sin6_family != AF_INET6 {
            break 'out Err(Errno::EAFNOSUPPORT);
        }
        let dst = dstaddr.sin6_addr;
        if in6_is_addr_v4mapped(&dst) {
            break 'out Err(Errno::EADDRNOTAVAIL);
        }

        // For an ICMPv6 packet, we should know its type and code to update statistics.
        let mut m = m;
        let mut type_ = 0u8;
        if proto == IPPROTO_ICMPV6 {
            if (m.m_len().get() as usize) < size_of::<Icmp6Hdr>() {
                match m_pullup(m, size_of::<Icmp6Hdr>() as i32) {
                    Some(n) => m = n,
                    None => break 'out Err(Errno::ENOBUFS),
                }
            }
            type_ = m.data()[0];
        }

        let Some(mut m) = m_prepend(m, IP6_HDR_LEN as i32, M_DONTWAIT) else {
            break 'out Err(Errno::ENOBUFS);
        };
        let mut ip6 = ip6_load(&m);

        // Next header might not be ICMP6 but use its pseudo header anyway.
        ip6.ip6_dst = dst;

        // KAME hack: embed scopeid
        let optp = if ctl { Some(&opt) } else { sticky.as_deref() };
        let pi_ifindex = optp
            .and_then(Ip6Pktopts::pktinfo)
            .map_or(0, |p| p.ipi6_ifindex);
        let im6o_ifidx = mopts.as_ref().map(|o| o.im6o_ifidx);
        if in6_embedscope(
            &mut ip6.ip6_dst,
            dstaddr,
            pi_ifindex,
            im6o_ifidx.unwrap_or(0),
        )
        .is_err()
        {
            break 'out Err(Errno::EINVAL);
        }

        // Source address selection: the options' packet info, which the selection may
        // rewrite in place (the C's `in6_pcbselsrc(&in6a, dstaddr, inp, optp)`).
        let pi = if ctl {
            opt.ip6po_pktinfo.as_deref_mut()
        } else {
            sticky
                .as_deref_mut()
                .and_then(|o| o.ip6po_pktinfo.as_deref_mut())
        };
        match in6_pcbselsrc(dstaddr, inp, pi, im6o_ifidx, net) {
            Ok(a) => ip6.ip6_src = a,
            Err(e) => break 'out Err(e),
        }

        ip6.ip6_flow = inp.inp_flowinfo() & IPV6_FLOWINFO_MASK;
        ip6.set_ip6_vfc((ip6.ip6_vfc() & !IPV6_VERSION_MASK) | IPV6_VERSION);
        // ip6_plen will be filled in ip6_output.
        ip6.ip6_nxt = inp.inp_ipv6.ip6_nxt();
        ip6.ip6_hlim = in6_selecthlim(inp) as u8;
        if !ip6_store(&mut m, &ip6) {
            break 'out Err(Errno::ENOBUFS);
        }

        let cksum6 = inp.inp_cksum6.get();
        if proto == IPPROTO_ICMPV6 || cksum6 != -1 {
            // compute checksum
            let off = if proto == IPPROTO_ICMPV6 {
                offset_of!(Icmp6Hdr, icmp6_cksum) as i32
            } else {
                cksum6
            };
            // The C's `u_int plen` against the `int` offset, converted to unsigned.
            if plen < 2 || plen - 2 < off as u32 {
                break 'out Err(Errno::EINVAL);
            }
            let off = off + IP6_HDR_LEN as i32;

            let Some((n, _, _)) = m_pulldown(m, off, size_of::<u16>() as i32, true) else {
                break 'out Err(Errno::ENOBUFS);
            };
            m = n;
            if !ip6_set_bytes(&mut m, off, &[0, 0]) {
                break 'out Err(Errno::ENOBUFS);
            }
            let sum = in6_cksum(&m, ip6.ip6_nxt, IP6_HDR_LEN as u32, plen);
            if !ip6_set_bytes(&mut m, off, &sum.to_ne_bytes()) {
                break 'out Err(Errno::ENOBUFS);
            }
        }

        let mut flags = 0;
        if inp.has_flags(IN6P_MINMTU) {
            flags |= IPV6_MINMTU;
        }

        // force routing table
        m.m_pkthdr().ph_rtableid.set(inp.inp_rtableid.get());

        if inp.socket().has_state(SS_ISCONNECTED)
            && proto != IPPROTO_ICMPV6
            && let Some(pf) = pf()
        {
            let a: &(dyn core::any::Any + Sync) = inp;
            pf.mbuf_link_inpcb_of(&m, a);
        }

        let seclevel = inp.inp_seclevel.get();
        let error = {
            let opts = if ctl {
                Some(opt.as_opts())
            } else {
                sticky.as_deref().map(Ip6Pktopts::as_opts)
            };
            let mut ro = inp.route();
            ip6_output(
                m,
                opts,
                Some(&mut *ro),
                flags,
                mopts.as_ref(),
                Some(&seclevel),
                net,
            )
        };
        if proto == IPPROTO_ICMPV6 {
            icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, type_);
        } else {
            rip6stat_inc(Rip6statCounters::Rip6sOpackets);
        }
        error
    };

    // freectl:
    if let Some(control) = control {
        ip6_clearpktopts(&mut opt, -1);
        drop(control);
    }
    drop(sticky);
    result
}

/// `rip6_ctloutput(op, so, level, optname, m)`: raw IPv6 socket option processing:
/// `IPV6_CHECKSUM` (`ip6_raw_ctloutput`), the other `IPPROTO_IPV6` options (`ip6_ctloutput`),
/// `IPPROTO_ICMPV6`'s (`icmp6_ctloutput`, `ICMP6_FILTER`); another level is `EINVAL`.
pub fn rip6_ctloutput<G: ProcGlue>(
    op: i32,
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip6_ctloutput");

    match level {
        IPPROTO_IPV6 => match optname {
            // MROUTING (MRT6_*): not configured, ip6_ctloutput's as any other option.
            IPV6_CHECKSUM => ip6_raw_ctloutput(op, inp, level, optname, m),
            _ => ip6_ctloutput(op, inp, level, optname, m, &il.net),
        },

        // XXX: is it better to call icmp6_ctloutput() directly from protosw?
        IPPROTO_ICMPV6 => icmp6_ctloutput(op, inp, level, optname, m),

        _ => Err(Errno::EINVAL),
    }
}

/// `rip6_attach(so, proto, wait)`: a control block for a privileged raw IPv6 socket of
/// protocol `proto` (`EACCES` unprivileged, `EPROTONOSUPPORT` outside `[0, IPPROTO_MAX)`), no
/// checksum offset, its ICMPv6 filter passing everything, its handle in the socket's locked
/// slot.
pub fn rip6_attach<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    proto: i32,
    wait: i32,
) -> Result<(), Errno> {
    if so.has_pcb() {
        kpanic(format_args!("rip6_attach"));
    }
    if !so.has_state(SS_PRIV) {
        return Err(Errno::EACCES);
    }
    if !(0..IPPROTO_MAX).contains(&proto) {
        return Err(Errno::EPROTONOSUPPORT);
    }

    soreserve(so, RIP6_SENDSPACE, RIP6_RECVSPACE)?;
    let inp = in_pcballoc(so, rawin6pcbtable::<G>(), wait)?;

    let mut h = inp.inp_ipv6.get::<Ip6Hdr>();
    h.ip6_nxt = proto as u8;
    inp.inp_ipv6.set(h);
    inp.inp_cksum6.set(-1);
    // ICMP6_FILTER_SETPASSALL(inp->inp_icmp6filt): the pcb's words (D-7). Input may walk the
    // pcb from in_pcballoc on: until the flag is published it reads no filter and lets every
    // type pass, as the C's NULL pointer does before its malloc returns.
    inp.icmp6filt_init();

    let mut il = inet(w);
    drop(so.set_pcb_locked(&mut il.pcb, Some(PoolArc::erase(inp))));
    Ok(())
}

/// `rip6_detach(so)`: the control block detached (its ICMPv6 filter with it); `panic(9)`
/// without one, as the C.
pub fn rip6_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);

    // MROUTING: ip6_mrouter_done(so); not configured.
    if !detach_slot(so, &mut il) {
        kpanic(format_args!("rip6_detach"));
    }
    Ok(())
}

/// `rip6_bind(so, nam, p)`: a local address of ours (or the wildcard) for `so`
/// (`in6_pcbaddrisavail`'s answer). Local ports are nonsensical for raw sockets: the port is
/// zeroed so no local lookup is made.
pub fn rip6_bind<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip6_bind");

    let mut addr = in6_nam2sin6(nam)?;

    // Make sure to not enter in_pcblookup_local(), local ports are non-sensical for raw
    // sockets.
    addr.sin6_port = 0;

    in6_pcbaddrisavail(inp, &mut addr, 0, &il.net)?;

    let g = inp.table().lock();
    inp.set_laddr6(&g, addr.sin6_addr);
    drop(g);

    Ok(())
}

/// `rip6_connect(so, nam)`: the foreign address of `so`, and the local one source selection
/// picks for it.
pub fn rip6_connect<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip6_connect");

    let addr = in6_nam2sin6(nam)?;

    // Source address selection. XXX: need pcblookup?
    let in6a = {
        let mut sticky = inp.outputopts6();
        let pi = sticky
            .as_deref_mut()
            .and_then(|o| o.ip6po_pktinfo.as_deref_mut());
        let ifidx = inp.moptions6_out().map(|o| o.im6o_ifidx);
        in6_pcbselsrc(&addr, inp, pi, ifidx, &il.net)?
    };

    let g = inp.table().lock();
    inp.set_laddr6(&g, in6a);
    inp.set_faddr6(&g, addr.sin6_addr);
    drop(g);
    soisconnected(so);

    Ok(())
}

/// `rip6_disconnect(so)`: `ENOTCONN` unless connected; the foreign address back to the
/// unspecified one.
pub fn rip6_disconnect<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip6_disconnect");

    if !so.has_state(SS_ISCONNECTED) {
        return Err(Errno::ENOTCONN);
    }

    soisdisconnected(so);
    let g = inp.table().lock();
    inp.set_faddr6(&g, In6Addr::default());
    drop(g);

    Ok(())
}

/// `rip6_shutdown(so)`: marks the connection as being incapable of further input.
pub fn rip6_shutdown<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let _il = inet(w);
    socantsendmore(so);
    Ok(())
}

/// `rip6_send(so, m, nam, control)`: ships a packet out to the connected address (`EISCONN`
/// with an address) or to `nam` (`ENOTCONN` without one), its scope zone kept; `rip6_output`
/// handles any massaging necessary. The data and the control are disposed of here.
pub fn rip6_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    nam: Option<&Mbuf>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip6_send");

    // always copy sockaddr to avoid overwrites
    let mut dst = SockaddrIn6 {
        sin6_family: AF_INET6,
        sin6_len: size_of::<SockaddrIn6>() as u8,
        ..SockaddrIn6::default()
    };
    if so.has_state(SS_ISCONNECTED) {
        if nam.is_some() {
            return Err(Errno::EISCONN);
        }
        dst.sin6_addr = inp.inp_faddr6.get();
    } else {
        let Some(nam) = nam else {
            return Err(Errno::ENOTCONN);
        };
        let addr6 = in6_nam2sin6(nam)?;
        dst.sin6_addr = addr6.sin6_addr;
        dst.sin6_scope_id = addr6.sin6_scope_id;
    }
    // sosend always hands over a packet (a pkthdr mbuf, empty or not).
    let Some(m) = m else {
        return Err(Errno::EINVAL);
    };
    rip6_output(m, so, inp, &dst, control, &il.net)
}

/// `rip6_sysctl_rip6stat`: the statistics, summed over the CPUs, read only.
fn rip6_sysctl_rip6stat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let c = RIP6COUNTERS.read();
    let rip6stat = Rip6stat {
        rip6s_ipackets: c[Rip6statCounters::Rip6sIpackets as usize],
        rip6s_isum: c[Rip6statCounters::Rip6sIsum as usize],
        rip6s_badsum: c[Rip6statCounters::Rip6sBadsum as usize],
        rip6s_nosock: c[Rip6statCounters::Rip6sNosock as usize],
        rip6s_nosockmcast: c[Rip6statCounters::Rip6sNosockmcast as usize],
        rip6s_fullsock: c[Rip6statCounters::Rip6sFullsock as usize],
        rip6s_opackets: c[Rip6statCounters::Rip6sOpackets as usize],
    };
    rdstruct(req, bytes_of(core::slice::from_ref(&rip6stat)))
}

/// `rip6_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: `{CTL_NET, PF_INET6,
/// IPPROTO_RAW, RIPV6CTL_STATS}`; every name at this level is terminal (`ENOTDIR` past it),
/// another one `EOPNOTSUPP`.
pub fn rip6_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // All sysctl names at this level are terminal.
    let [n] = name else {
        return Err(Errno::ENOTDIR);
    };
    match *n {
        RIPV6CTL_STATS => rip6_sysctl_rip6stat(req),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

impl<G: ProcGlue> Usrreqs<G> for Rip6Usrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        rip6_attach(so, w, proto, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        rip6_detach(so, w)
    }
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        rip6_bind(so, w, nam, p)
    }
    fn connect(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &Mbuf) -> Result<(), Errno> {
        rip6_connect(so, w, nam)
    }
    fn disconnect(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        rip6_disconnect(so, w)
    }
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        rip6_shutdown(so, w)
    }
    /// No `pru_rcvd` (the entries are not `PR_WANTRCVD`): never asked.
    fn rcvd(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_rcvd of raw IPv6"));
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        rip6_send(so, w, top, addr, control)
    }
    /// No `pru_abort` (a raw socket is never on a listen queue): never asked.
    fn abort(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_abort of raw IPv6"));
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

// CTASSERT(sizeof(rip6stat) == rip6s_ncounters * sizeof(uint64_t)), the enum's end included.
const _: () = assert!(Rip6statCounters::Rip6sOpackets as usize + 1 == RIP6S_NCOUNTERS);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicU32;
    use std::vec::Vec;

    use hal::sys::netinet_icmp6::Icmp6Filter;
    use hal::sys::netinet_in::IPPROTO_TCP;
    use hal::sys::netinet6_in6::ICMP6_FILTER;
    use hal::sys::socket::AF_INET;

    use super::*;
    use crate::net::mbuf::{M_WAIT, MT_SOOPTS, m_get};
    use crate::net::proto::{PRC_HOSTDEAD, PRCO_GETOPT, PRCO_SETOPT};
    use crate::netinet::raw_ip::testraw::{
        self, DIVERT6, G, ICMP6, RAW6, close, locked, nam6, pkt, recv, socket, with_net,
    };
    use crate::netinet::tcp_subr::testtcp::{a6, iface6};

    /// An option mbuf of `bytes`.
    fn optm(bytes: &[u8]) -> MbufBox {
        let mut m = m_get(M_WAIT, MT_SOOPTS).expect("an mbuf");
        m.m_len().set(bytes.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(bytes);
        m
    }

    /// `setsockopt` through `rip6_ctloutput`.
    fn set(so: &Socket<G>, level: i32, opt: i32, v: &[u8]) -> Result<(), Errno> {
        let mut m = optm(v);
        locked(so, |w| {
            rip6_ctloutput(PRCO_SETOPT, so, w, level, opt, Some(&mut m))
        })
    }

    /// `getsockopt` through `rip6_ctloutput`: the answer's bytes.
    fn get(so: &Socket<G>, level: i32, opt: i32) -> Result<Vec<u8>, Errno> {
        let mut m = optm(&[]);
        locked(so, |w| {
            rip6_ctloutput(PRCO_GETOPT, so, w, level, opt, Some(&mut m))
        })
        .map(|()| m.data().to_vec())
    }

    /// The bytes of a filter.
    fn fbytes(f: &Icmp6Filter) -> Vec<u8> {
        f.icmp6_filt.iter().flat_map(|w| w.to_ne_bytes()).collect()
    }

    /// An IPv6 packet from `src` to `dst` of next header `nxt`, its 40-byte header and
    /// `payload`.
    fn ip6(src: In6Addr, dst: In6Addr, nxt: u8, payload: &[u8]) -> Vec<u8> {
        let mut h = Ip6Hdr {
            ip6_plen: (payload.len() as u16).to_be(),
            ip6_nxt: nxt,
            ip6_hlim: 64,
            ip6_src: src,
            ip6_dst: dst,
            ..Ip6Hdr::zeroed()
        };
        h.set_ip6_vfc(IPV6_VERSION);
        let mut b = std::vec![0u8; 40 + payload.len()];
        assert!(pod::put(&mut b, 0, &h));
        b[40..].copy_from_slice(payload);
        b
    }

    /// `b` with the checksum of its payload stored at payload offset `off` (in6_cksum's).
    fn with_sum(mut b: Vec<u8>, off: usize) -> Vec<u8> {
        b[40 + off..42 + off].copy_from_slice(&[0, 0]);
        let m = pkt(&b);
        let sum = in6_cksum(&m, b[6], 40, (b.len() - 40) as u32);
        b[40 + off..42 + off].copy_from_slice(&sum.to_ne_bytes());
        b
    }

    /// `rip6_input` of `bytes` (whose next header is the protocol) under the shared net lock.
    fn deliver(bytes: &[u8]) {
        let p = i32::from(bytes[6]);
        with_net(|net| {
            let mut mp = Some(pkt(bytes));
            let mut off = 40;
            assert_eq!(
                rip6_input::<G>(&mut mp, &mut off, p, i32::from(AF_INET6), None, net),
                IPPROTO_DONE
            );
            assert!(mp.is_none(), "the packet consumed");
        });
    }

    /// The statistic `c`.
    fn r6s(c: Rip6statCounters) -> u64 {
        RIP6COUNTERS.read()[c as usize]
    }

    /// `rip6_send` of `bytes` to `to`.
    fn send(so: &Socket<G>, bytes: &[u8], to: Option<In6Addr>) -> Result<(), Errno> {
        let n = to.map(nam6);
        locked(so, |w| {
            rip6_send(so, w, Some(pkt(bytes)), n.as_deref(), None)
        })
    }

    #[test]
    fn rip6_attach_needs_privilege_and_sets_no_checksum_and_a_pass_all_filter() {
        let (_g, _p) = testraw::setup();
        assert_eq!(socket(&RAW6, 58, false).err(), Some(Errno::EACCES));
        assert_eq!(
            socket(&RAW6, IPPROTO_MAX, true).err(),
            Some(Errno::EPROTONOSUPPORT)
        );
        let so = socket(&RAW6, 253, true).expect("attached");
        let inp = testraw::pcb(&so);
        assert!(inp.has_flags(INP_IPV6));
        assert_eq!(inp.inp_ipv6.ip6_nxt(), 253);
        assert_eq!(inp.inp_cksum6.get(), -1);
        let filt = inp.icmp6filt().expect("rip6_attach's filter");
        assert!(
            filt.iter()
                .all(|w: &AtomicU32| w.load(Ordering::Relaxed) == u32::MAX)
        );
        assert!(core::ptr::eq(inp.table(), rawin6pcbtable::<G>()));
        drop(inp);
        close(so);
    }

    #[test]
    fn icmpv6_reaches_the_sockets_whose_filter_passes_it() {
        let (_g, _p) = testraw::setup();
        let all = socket(&ICMP6, 58, true).expect("attached");
        let no_echo = socket(&ICMP6, 58, true).expect("attached");
        let other = socket(&RAW6, 253, true).expect("attached");
        let mut f = Icmp6Filter::default();
        f.setpassall();
        f.setblock(128);
        assert_eq!(
            set(&no_echo, IPPROTO_ICMPV6, ICMP6_FILTER, &fbytes(&f)),
            Ok(())
        );
        let ipackets = r6s(Rip6statCounters::Rip6sIpackets);

        let (src, dst) = (a6(70, 2), a6(70, 1));
        let request = with_sum(ip6(src, dst, 58, &[128, 0, 0, 0, 0, 1, 0, 1]), 2);
        deliver(&request);
        let (name, data) = recv(&all).expect("a copy");
        assert_eq!(data, request[40..], "the headers before *offp stripped");
        let from = pod::at::<SockaddrIn6>(&name, 0).expect("a sockaddr_in6");
        assert_eq!(from.sin6_addr, src);
        assert!(recv(&no_echo).is_none(), "ICMP6_FILTER_WILLBLOCK(128)");
        assert!(recv(&other).is_none(), "another protocol");

        let reply = with_sum(ip6(src, dst, 58, &[129, 0, 0, 0, 0, 1, 0, 1]), 2);
        deliver(&reply);
        assert!(recv(&all).is_some() && recv(&no_echo).is_some());
        // ICMPv6 is counted by ICMPv6, not as raw input.
        assert_eq!(r6s(Rip6statCounters::Rip6sIpackets), ipackets);

        // The filter set again while packets are delivered by another thread: each word is
        // read whole, each packet is delivered or blocked, nothing else.
        let inp = testraw::pcb(&no_echo);
        std::thread::scope(|s| {
            let t = s.spawn(|| {
                for _ in 0..64 {
                    deliver(&request);
                }
            });
            for i in 0..64 {
                let mut g = optm(&fbytes(&if i % 2 == 0 {
                    Icmp6Filter::default()
                } else {
                    f
                }));
                let _ = icmp6_ctloutput(
                    PRCO_SETOPT,
                    &inp,
                    IPPROTO_ICMPV6,
                    ICMP6_FILTER,
                    Some(&mut g),
                );
            }
            assert!(t.join().is_ok());
        });
        let mut n = 0;
        while let Some((_, d)) = recv(&no_echo) {
            assert_eq!(d, request[40..]);
            n += 1;
        }
        assert!(n <= 64);
        drop(inp);
        for so in [all, no_echo, other] {
            close(so);
        }
    }

    #[test]
    fn ipv6_checksum_offsets_are_verified_on_input() {
        let (_g, _p) = testraw::setup();
        let at2 = socket(&RAW6, 253, true).expect("attached");
        let at8 = socket(&RAW6, 253, true).expect("attached");
        assert_eq!(
            set(&at2, IPPROTO_IPV6, IPV6_CHECKSUM, &2i32.to_ne_bytes()),
            Ok(())
        );
        assert_eq!(
            set(&at8, IPPROTO_IPV6, IPV6_CHECKSUM, &8i32.to_ne_bytes()),
            Ok(())
        );
        let (src, dst) = (a6(71, 2), a6(71, 1));
        let (isum, badsum) = (
            r6s(Rip6statCounters::Rip6sIsum),
            r6s(Rip6statCounters::Rip6sBadsum),
        );

        // A good sum at 2: the socket at 2 takes it; the one at 8 refuses it on the bound
        // (8 bytes: 8 - 2 < 8) though the sum itself is good.
        let good = with_sum(ip6(src, dst, 253, &[1, 2, 0, 0, 5, 6, 7, 8]), 2);
        deliver(&good);
        assert_eq!(recv(&at2).map(|r| r.1), Some(good[40..].to_vec()));
        assert!(recv(&at8).is_none());
        assert_eq!(r6s(Rip6statCounters::Rip6sIsum), isum + 2);
        assert_eq!(r6s(Rip6statCounters::Rip6sBadsum), badsum + 1);

        // A bad sum; a packet shorter than *offp + 2.
        let mut bad = good.clone();
        bad[47] ^= 0xff;
        deliver(&bad);
        deliver(&ip6(src, dst, 253, &[1]));
        assert!(recv(&at2).is_none() && recv(&at8).is_none());
        assert_eq!(r6s(Rip6statCounters::Rip6sBadsum), badsum + 5);
        for so in [at2, at8] {
            close(so);
        }
    }

    #[test]
    fn ipv6_checksum_output_and_its_bounds() {
        let (_g, _p) = testraw::setup();
        let d = iface6(b"rip6a", 72);
        let raw = socket(&RAW6, 253, true).expect("attached");
        assert_eq!(
            set(&raw, IPPROTO_IPV6, IPV6_CHECKSUM, &2i32.to_ne_bytes()),
            Ok(())
        );
        let to = a6(72, 2);
        let opackets = r6s(Rip6statCounters::Rip6sOpackets);

        assert_eq!(send(&raw, &[1, 2, 0, 0, 5, 6, 7, 8], Some(to)), Ok(()));
        let out = d.last.lock().expect("the wire").clone();
        assert_eq!(out.len(), 48);
        assert_eq!(out[6], 253);
        let m = pkt(&out);
        assert_eq!(in6_cksum(&m, 253, 40, 8), 0, "the sum written at offset 2");
        assert_eq!(r6s(Rip6statCounters::Rip6sOpackets), opackets + 1);

        // `plen < 2 || plen - 2 < off`.
        assert_eq!(send(&raw, &[1], Some(to)), Err(Errno::EINVAL));
        assert_eq!(
            set(&raw, IPPROTO_IPV6, IPV6_CHECKSUM, &8i32.to_ne_bytes()),
            Ok(())
        );
        assert_eq!(send(&raw, &[0; 8], Some(to)), Err(Errno::EINVAL));
        assert_eq!(
            set(&raw, IPPROTO_IPV6, IPV6_CHECKSUM, &6i32.to_ne_bytes()),
            Ok(())
        );
        assert_eq!(send(&raw, &[0; 8], Some(to)), Ok(()));

        // ping6: ICMPv6's sum at 2, the echo request counted in ICMPv6's histogram.
        let ping = socket(&ICMP6, 58, true).expect("attached");
        let hist = crate::netinet6::ip6_var::ICMP6COUNTERS.read()
            [Icmp6statCounters::Icp6sOuthist as usize + 128];
        assert_eq!(send(&ping, &[128, 0, 0, 0, 0, 7, 0, 1], Some(to)), Ok(()));
        let out = d.last.lock().expect("the wire").clone();
        assert_eq!((out[6], out[40]), (58, 128));
        assert_eq!(in6_cksum(&pkt(&out), 58, 40, 8), 0);
        assert_eq!(
            crate::netinet6::ip6_var::ICMP6COUNTERS.read()
                [Icmp6statCounters::Icp6sOuthist as usize + 128],
            hist + 1
        );
        // An unconnected socket needs an address.
        assert_eq!(send(&ping, &[128, 0, 0, 0], None), Err(Errno::ENOTCONN));
        for so in [raw, ping] {
            close(so);
        }
    }

    #[test]
    fn rip6_ctloutput_and_the_filter_option() {
        let (_g, _p) = testraw::setup();
        let so = socket(&ICMP6, 58, true).expect("attached");
        let mut pass = Icmp6Filter::default();
        pass.setpassall();
        assert_eq!(get(&so, IPPROTO_ICMPV6, ICMP6_FILTER), Ok(fbytes(&pass)));
        let mut block = Icmp6Filter::default();
        block.setblockall();
        assert_eq!(
            set(&so, IPPROTO_ICMPV6, ICMP6_FILTER, &fbytes(&block)),
            Ok(())
        );
        assert_eq!(get(&so, IPPROTO_ICMPV6, ICMP6_FILTER), Ok(fbytes(&block)));
        assert_eq!(
            set(&so, IPPROTO_ICMPV6, ICMP6_FILTER, &[0; 31]),
            Err(Errno::EMSGSIZE)
        );
        assert_eq!(
            set(&so, IPPROTO_ICMPV6, 99, &[0; 32]),
            Err(Errno::ENOPROTOOPT)
        );
        assert_eq!(get(&so, IPPROTO_ICMPV6, 99), Err(Errno::ENOPROTOOPT));
        assert_eq!(set(&so, IPPROTO_TCP, 1, &[0; 4]), Err(Errno::EINVAL));
        // ICMPv6's checksum offset is fixed (ip6_raw_ctloutput's icmp6off).
        assert_eq!(
            get(&so, IPPROTO_IPV6, IPV6_CHECKSUM),
            Ok(2i32.to_ne_bytes().to_vec())
        );
        close(so);

        // A divert6 socket has no filter: the C's NULL, EINVAL.
        let div = socket(&DIVERT6, 0, true).expect("attached");
        assert_eq!(get(&div, IPPROTO_ICMPV6, ICMP6_FILTER), Err(Errno::EINVAL));
        assert_eq!(
            set(&div, IPPROTO_ICMPV6, ICMP6_FILTER, &fbytes(&pass)),
            Err(Errno::EINVAL)
        );
        close(div);
    }

    #[test]
    fn rip6_ctlinput_notifies_the_sockets_of_the_destination() {
        let (_g, _p) = testraw::setup();
        let _d = iface6(b"rip6b", 73);
        let so = socket(&RAW6, 253, true).expect("attached");
        let to = a6(73, 2);
        assert_eq!(locked(&so, |w| rip6_connect(&so, w, &nam6(to))), Ok(()));
        assert_eq!(send(&so, &[0; 4], None), Ok(()));
        let inp = testraw::pcb(&so);
        let cached = || {
            let w = crate::net::socket::solock(&so);
            let r = inp.route().ro_rt.is_some();
            drop(w);
            r
        };
        assert!(cached(), "the send cached its route");
        let sa6 = |a: In6Addr, len: u8, fam: u8| {
            let mut b = [0u8; 28];
            let s = SockaddrIn6 {
                sin6_len: len,
                sin6_family: fam,
                sin6_addr: a,
                ..SockaddrIn6::default()
            };
            assert!(pod::put(&mut b, 0, &s));
            b
        };
        let ctl = |b: &[u8], cmd: i32| {
            with_net(|net| {
                rip6_ctlinput::<G>(
                    cmd,
                    SockaddrView::new(b).expect("a view"),
                    0,
                    CtlArg::None,
                    net,
                )
            })
        };
        // Another family, another length, a command past the table: nothing.
        ctl(&sa6(to, 28, AF_INET), PRC_HOSTDEAD);
        ctl(&sa6(to, 24, AF_INET6), PRC_HOSTDEAD);
        ctl(&sa6(to, 28, AF_INET6), PRC_NCMDS as i32);
        assert!(cached());
        // A dead host: in_pcbrtchange of the sockets to it.
        ctl(&sa6(to, 28, AF_INET6), PRC_HOSTDEAD);
        assert!(!cached(), "the route cache let go");
        drop(inp);
        close(so);
    }

    #[test]
    fn the_statistics_sysctl() {
        let (_g, _p) = testraw::setup();
        let u = crate::testutil::USER_BASE + 0x3_c000;
        let req = |name: &[i32], oldp: usize, len: &mut usize| -> Result<(), Errno> {
            let mut r = SysctlReq {
                oldp,
                oldlenp: len,
                newp: 0,
                newlen: 0,
            };
            rip6_sysctl(name, &mut r)
        };
        let mut len = 0;
        assert_eq!(req(&[RIPV6CTL_STATS], 0, &mut len), Ok(()));
        assert_eq!(len, 56);
        let opackets = r6s(Rip6statCounters::Rip6sOpackets);
        assert_eq!(req(&[RIPV6CTL_STATS], u, &mut len), Ok(()));
        let got: Rip6stat = crate::testutil::user_bytes(u, len)
            .and_then(|b| hal::user::pod_from_bytes(&b))
            .expect("the structure");
        assert!(got.rip6s_opackets >= opackets);
        assert_eq!(req(&[2], 0, &mut len), Err(Errno::EOPNOTSUPP));
        assert_eq!(req(&[RIPV6CTL_STATS, 0], 0, &mut len), Err(Errno::ENOTDIR));
    }
}
/* </TESTS> */
