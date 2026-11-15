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
/* </LICENSES> */

/* <CODE> */
//! Neighbor solicitations and advertisements, and Duplicate Address Detection:
//! `netinet6/nd6_nbr.c`.
//!
//! Upstream: sys/netinet6/nd6_nbr.c @ 3ce1f3f79392
//! LZ: sys/netinet6/nd6_nbr.rs@3c62ede68802
//!
//! [`nd6_ns_input`] answers a solicitation for one of our addresses (or one we proxy) with an
//! advertisement and learns the solicitor's link-layer address; [`nd6_na_input`] completes the
//! neighbor cache entry `nd6_resolve` started (INCOMPLETE to REACHABLE or STALE) or updates an
//! existing one by the override/solicited rules of RFC 2461 7.2.5. [`nd6_ns_output`] and
//! [`nd6_na_output`] build those messages. Duplicate Address Detection (RFC 2462) keeps a
//! `dadq` per tentative address: `ip6_dad_count` solicitations from `::`, one per
//! `RETRANS_TIMER`; a solicitation or advertisement for the address from someone else marks it
//! duplicated, silence makes it usable.
//!
//! ## Deviations
//! - The input functions refuse (`icp6s_tooshort`) a message shorter than its fixed part,
//!   which `icmp6_input` already guarantees; the C would read past it (LZ's).
//! - The cache update half of `nd6_na_input` (from the lookup of an existing entry on) is
//!   [`nd6_na_cache`], so host tests can run it on an entry made by hand (LZ's).
//! - `nd6_ifptomac` returns the Ethernet address by value; the C returns `ifp + 1`, the
//!   `struct arpcom` address, for every type in its list: here an interface of those types
//!   that `ether_ifattach` did not attach has none. `nd6_na_output`'s `sdl0` is a
//!   `sockaddr_dl` (the C's `struct sockaddr *` checked for `AF_LINK`), and `nd6_ns_input`
//!   passes a copy of the proxy route's gateway (the C keeps the pointer after `rtfree`)
//!   (LZ's).
//! - `NCARP` (`carp_iamatch`) is not configured: each site is a comment. `DIAGNOSTIC`'s
//!   `MCLBYTES` check is under the `diagnostic` feature. `inet_ntop` is `nd6::In6Ntop` (LZ's).
//! - `nd6_dad_timer`'s argument is the record's address, looked up on `dadq`, where the C's is
//!   the `ifa` (`nd6_dad_find(ifa)`): a late timeout of a destroyed record finds nothing and
//!   does nothing, where the C's would act on a newer record of the same address (one started
//!   again before the old timeout ran). Safer than the C; no outcome userland sees changes but
//!   that a stale timeout no longer takes a step of the new DAD early (N6.5b, D-13).
//!
//! ## Redesign
//! - The ND messages are read out of the packet's bytes ([`nd6_msg`]: the C's
//!   `ip6_exthdr_get`, in place when the first mbuf holds the message, else `m_pulldown`'s
//!   contiguous copy) with `hal::pod::at` (the message's fixed part, 24 bytes, as LZ's copy),
//!   the options as a slice of the message (`nd6::nd6_options`, offsets, no pointers into the
//!   packet); LZ's `nd6_msg`/`nd6_lladdr_of` `unsafe fn`s over raw pointers go. The messages
//!   sent are written into their fresh mbuf with `hal::pod::put`.
//! - `nd6_ns_output` and `nd6_na_output` hand their packet to the kernel's `ip6_output` (no
//!   options; the multicast interface and hop limit as the C's stack `ip6_moptions`, an
//!   `Ip6Moptions::output`); N6.5b's temporary `Inet::ip6_output_opts` leg went at N6.5c.
//! - `struct dadq` (LZ: `Cell` counters, a `malloc`ed entry on a `TailqHead` freed by
//!   `nd6_dad_reaper`, its timeout's argument a raw `ifa` dereferenced in an `unsafe` block)
//!   -> [`Dadq`], a recycled `&'static` record (D-13: `malloc(M_IP6NDP)` once, then
//!   [`DAD_FREE`]) on [`DADQ`], a `hal::list::List` under the net lock as the C's `dadq`
//!   (`NET_ASSERT_LOCKED` at its changes, `NET_LOCK` in its timeout); `dad_ifa` the
//!   `ifaref`'d address's counted handle (N6.5e: `kernel::net::if_var::IfaddrRef`) in a
//!   `hal::slot::ReadMostly` cell (set under a holding that may be shared, so not a net cell):
//!   `nd6_dad_find` compares under a read, and the timer clones the handle out for its step
//!   (no reader held across its sends and the destroy that hands the record to the reaper,
//!   which empties the cell). Its timeout is embedded and pinned for ever (`Pin::static_ref`:
//!   the record is never dropped), set once to `nd6_dad_timer` with the record's own address
//!   as an integer argument, which the handler compares with the addresses of [`DADQ`]'s
//!   records under the net lock (the C's `nd6_dad_find` walk, O(n) both) and never
//!   dereferences: a timeout that fires after its record was destroyed finds nothing ("DAD
//!   structure not found"). The C's per-record reaper (its own timeout set again to
//!   `nd6_dad_reaper`, so that `free` runs after the handler) is one static [`DAD_REAPER`]
//!   that moves the records `nd6_dad_destroy` queued on [`DAD_REAP`] to [`DAD_FREE`] once
//!   `Timeout::del_barrier` of each record's timeout returned (neither pending nor running:
//!   both are `TIMEOUT_PROC` timeouts of one context, so the barrier answers at once; the
//!   record is reused only then), and gives the address's reference back (`ifafree`).
//! - `ifp->if_nd->reachable` is the embedded record's (`nd6::NdIfinfoCell`), which every
//!   interface has: LZ's fallback to `REACHABLE_TIME` for one without it goes.
//!
//! **Lock and spl transitions** against the C: `nd6_ns_input` and `nd6_na_input` run under
//! the exclusive net lock as the C's (its proof, `net`); the neighbor cache record is reached
//! under one `ND6_MTX` section per `nd6_na_input` (`nd6_na_cache`'s and the router's new
//! entry), where the C reads it under the net lock alone: one mutex pair per advertisement
//! above the C (`nd6`'s lock table, N6.4b's R5 class, the N12 row); its routing message and
//! `rt6_flush` after the section, in the C's order. DAD: the C's net lock (`DADQ` is checked
//! by no witness: the `if_addrlist` precedent), and `DAD_REAP`'s and `DAD_FREE`'s mutexes once
//! per record destroyed or reused, where the C `free`s and `malloc`s (two pairs per address
//! tested, control path).
//!
//! **Holders**: `dad_ifa`'s `ifaref` (`nd6_dad_start`) given back by the reaper's `ifafree`, as
//! the C's; `nd6_lookup`'s route reference dropped at the end of each input (`rtfree`).

use core::ffi::c_void;
use core::mem::size_of;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::list::{List, ListEntry};
use hal::malloc::{M_NOWAIT, MBox};
use hal::mutex::Mutexed;
use hal::netlock::{NETLOCK, NetHeld};
use hal::pod;
use hal::pool::PoolArc;
use hal::slot::ReadMostly;
use hal::sys::endian::htons;
use hal::sys::net_if::{
    IFF_RUNNING, IFF_UP, IFNAMSIZ, IFT_CARP, IFT_ETHER, IFT_IEEE1394, IFT_IEEE80211,
    IFT_PROPVIRTUAL, SockaddrDl,
};
use hal::sys::net_route::{
    RTF_ANNOUNCE, RTF_CLONED, RTF_CLONING, RTF_LLINFO, RTF_REJECT, RTM_CHGADDRATTR, RTM_RESOLVE,
};
use hal::sys::netinet_icmp6::{
    Icmp6Hdr, Icmp6statCounters, ND_NA_FLAG_OVERRIDE, ND_NA_FLAG_ROUTER, ND_NA_FLAG_SOLICITED,
    ND_NEIGHBOR_ADVERT, ND_NEIGHBOR_SOLICIT, ND_OPT_SOURCE_LINKADDR, ND_OPT_TARGET_LINKADDR,
    NdNeighborAdvert, NdNeighborSolicit, NdOptHdr,
};
use hal::sys::netinet_if_ether::ETHER_ADDR_LEN;
use hal::sys::netinet_in::IPPROTO_ICMPV6;
use hal::sys::netinet_ip6::{IPV6_VERSION, Ip6Hdr};
use hal::sys::netinet6_in6::{
    __IPV6_ADDR_INT16_MLL, __IPV6_ADDR_INT32_ONE, IN6ADDR_LINKLOCAL_ALLNODES,
    IN6ADDR_LINKLOCAL_ALLROUTERS, In6Addr, SockaddrIn6, in6_are_addr_equal, in6_is_addr_linklocal,
    in6_is_addr_multicast, in6_is_addr_unspecified, in6_is_scope_embed,
};
use hal::sys::netinet6_in6_var::{IN6_IFF_ANYCAST, IN6_IFF_DUPLICATED, IN6_IFF_TENTATIVE};
use hal::sys::netinet6_nd6::{ND6_LLINFO_INCOMPLETE, ND6_LLINFO_REACHABLE, ND6_LLINFO_STALE};
use hal::sys::socket::AF_LINK;

use crate::assert::{kassert, kpanic};
use crate::log::{LOG_ERR, LOG_INFO};
use crate::net::if_::{IfRef, if_isconnected};
use crate::net::if_var::{AtomicCell, Ifaddr, IfaddrRef, Ifnet};
use crate::net::mbuf::{
    M_DONTWAIT, M_EXT, M_ICMP_CSUM_OUT, M_MCAST, MAX_LINKHDR, MHLEN, MT_DATA, Mbuf, MbufBox,
    MbufList, m_align, m_gethdr, mclget, mq_delist,
};
use crate::net::mbuf2::m_pulldown;
use crate::net::proto::SockaddrView;
use crate::net::route::{RT_RESOLVE, Rtentry, ifafree, ifaref, rtalloc, rtisvalid};
use crate::net::rtsock::{rtm_addr, rtm_send};
use crate::netinet::if_ether::{gw_family_len, gw_sdl_copy};
use crate::netinet6::in6::{in6ifa_ifpforlinklocal, in6ifa_ifpwithaddr};
use crate::netinet6::in6_var::{ifa_in6, ifatoia6};
use crate::netinet6::ip6_output::ip6_output;
use crate::netinet6::ip6_var::{
    IP6_DAD_COUNT, IP6_DAD_PENDING, IP6_FORWARDING, IPV6_UNSPECSRC, Ip6Moptions, icmp6stat_inc,
    icmp6stat_inc_hist,
};
use crate::netinet6::nd6::{
    In6Ntop, LlinfoNd6, M_IP6NDP, ND6_GCTIMER, ND6_MTX, NdOpts, RETRANS_TIMER, nd6_cache_lladdr,
    nd6_ether_sprintf, nd6_lladdr_alen, nd6_llchanged, nd6_llinfo_settimer, nd6_llstore,
    nd6_lookup, nd6_opt_lladdr, nd6_options, nd6_output_held,
};
use crate::netinet6::nd6_rtr::rt6_flush;
use crate::prf::{Str, log};
use crate::timeout::{KCLOCK_NONE, TIMEOUT_PROC, Timeout};

/// The interface name the C prints for an address without an interface.
const UNKNOWN_IFNAME: [u8; IFNAMSIZ] = *b"???\0\0\0\0\0\0\0\0\0\0\0\0\0";

/// The length of an IPv6 header.
const IP6_HDR_LEN: usize = size_of::<Ip6Hdr>();

/// `struct dadq`: an address to run DAD on, a recycled record (the module's redesign). Locks:
/// \[N\] net lock.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct dadq's dad_* names so the C can be grepped"
)]
pub struct Dadq {
    /// \[N\] `dad_list`: the `dadq` list; once off it, the entry of [`DAD_REAP`] or
    /// [`DAD_FREE`] (under their mutex): a record is on one of the three at a time.
    dad_list: ListEntry<Dadq>,
    /// `dad_ifa`: the address, referenced (`ifaref`; `None` on the free lists).
    dad_ifa: ReadMostly<Option<IfaddrRef>>,
    /// \[N\] `dad_count`: max NS to send.
    dad_count: AtomicCell<i32>,
    /// \[N\] `dad_ns_tcount`: # of trials to send NS.
    dad_ns_tcount: AtomicCell<i32>,
    /// \[N\] `dad_ns_ocount`: NS sent so far.
    dad_ns_ocount: AtomicCell<i32>,
    /// \[N\] `dad_ns_icount`: NS received for the address.
    dad_ns_icount: AtomicCell<i32>,
    /// \[N\] `dad_na_icount`: NA received for the address.
    dad_na_icount: AtomicCell<i32>,
    /// `dad_timer_ch`: the retransmission timer (its argument the record's address).
    dad_timer_ch: Timeout,
}

impl Dadq {
    /// A record every member of which is zero (`M_ZERO`), its timeout `nd6_dad_timer`'s (its
    /// argument set once the record has its address).
    const fn new() -> Self {
        Self {
            dad_list: ListEntry::new(),
            dad_ifa: ReadMostly::new(None),
            dad_count: AtomicCell::new(),
            dad_ns_tcount: AtomicCell::new(),
            dad_ns_ocount: AtomicCell::new(),
            dad_ns_icount: AtomicCell::new(),
            dad_na_icount: AtomicCell::new(),
            dad_timer_ch: Timeout::new_flags(
                nd6_dad_timer,
                ptr::null_mut(),
                KCLOCK_NONE,
                TIMEOUT_PROC,
            ),
        }
    }

    /// The address under DAD (`dp->dad_ifa`), a reference cloned out of the cell (R5: no
    /// reference into a `ReadMostly` is held across a sleep or a write of its cell); a record
    /// on `DADQ` always has one. A clone the C lacks (it reads `dp->dad_ifa` in place), one
    /// per timer step and per received NS or NA: one read-modify-write pair of the count each,
    /// listed with the T1-T4 clones (`kernel::net::if_var`'s redesign, ROADMAP's N12 row).
    fn ifa(&self) -> IfaddrRef {
        match self.dad_ifa.read().clone() {
            Some(ifa) => ifa,
            None => kpanic(format_args!("nd6_dad: a record without its address")),
        }
    }
}

hal::list_adapter!(
    /// `TAILQ_HEAD(, dadq)` through `dad_list`: `dadq`, and the reaper's and the free lists
    /// of DAD records (the module's redesign).
    DadList: Dadq, dad_list
);

/// \[N\] `dadq`: list of addresses to run DAD on.
static DADQ: List<DadList> = List::new();

/// The records `nd6_dad_destroy` took off [`DADQ`], for [`DAD_REAPER`]. Its mutex is
/// `IPL_SOFTNET`: taken under the net lock and by the reaper's timeout, never from a hardware
/// interrupt.
static DAD_REAP: Mutexed<List<DadList>> = Mutexed::new(IPL_SOFTNET, List::new());

/// The records the reaper gave back, for [`nd6_dad_start`] (the C's `free`, then `malloc`).
static DAD_FREE: Mutexed<List<DadList>> = Mutexed::new(IPL_SOFTNET, List::new());

/// The reaper of destroyed DAD records (the module's redesign): a process-context timeout as
/// the records' own (`timeout_set_proc`).
static DAD_REAPER: Timeout =
    Timeout::new_flags(nd6_dad_reaper, ptr::null_mut(), KCLOCK_NONE, TIMEOUT_PROC);

/// `dad_maxtry`: max # of *tries* to transmit DAD packet.
static DAD_MAXTRY: AtomicI32 = AtomicI32::new(15);

/// The fields of the IPv6 header of `m` ND's inputs read: the hop limit, the source and the
/// destination (its first mbuf holds the header: `ip6_input`'s pullup).
fn ip6_fields(m: &Mbuf) -> Option<(u8, In6Addr, In6Addr)> {
    let d = m.data();
    Some((
        *d.get(7)?,
        pod::at::<In6Addr>(d, 8)?,
        pod::at::<In6Addr>(d, 24)?,
    ))
}

/// `ip6_exthdr_get(&m, off, len)` of an ND message (the C's `IP6_EXTHDR_GET`): the `len` bytes
/// at `off` of `m` made contiguous, in place when the first mbuf holds them, else by
/// `m_pulldown`: the chain back with the mbuf they are in (its index in the chain) and their
/// offset there; `None` (the chain freed) when the chain does not hold them or memory runs out.
pub(crate) fn nd6_msg(m: MbufBox, off: i32, len: i32) -> Option<(MbufBox, usize, usize)> {
    let (Ok(o), Ok(l)) = (usize::try_from(off), usize::try_from(len)) else {
        return None;
    };
    if m.data().len() >= o + l {
        return Some((m, 0, o));
    }
    let (m, at, o) = m_pulldown(m, off, len, true)?;
    Some((m, at, usize::try_from(o).ok()?))
}

/// The `len` bytes [`nd6_msg`] made contiguous in mbuf `at` of `m` at `o`.
pub(crate) fn nd6_msg_bytes(m: &Mbuf, at: usize, o: usize, len: usize) -> Option<&[u8]> {
    let mut n = m;
    for _ in 0..at {
        n = n.m_next.get()?;
    }
    n.data().get(o..o + len)
}

/// `nd6_ns_input(m, off, icmp6len)`: input a Neighbor Solicitation Message (`icmp6len` bytes
/// at `off` of `m`, consumed). Based on RFC 2461 and RFC 2462 (duplicated address detection).
/// Under the exclusive net lock (`net`).
pub fn nd6_ns_input(m: MbufBox, off: i32, icmp6len: i32, net: &NetHeld<'_>) {
    let Some((hlim, mut saddr6, daddr6)) = ip6_fields(&m) else {
        return; // m freed
    };
    let mut i_am_router = IP6_FORWARDING.load(Ordering::Relaxed) != 0;

    let Some(ifp) = IfRef::get(m.m_pkthdr().ph_ifidx.get()) else {
        return; // freeit
    };
    let ifp = ifp.ifp();

    let fixed = size_of::<NdNeighborSolicit>();
    let got = if (icmp6len as usize) < fixed {
        None
    } else {
        nd6_msg(m, off, icmp6len)
    };
    let Some((m, at, o)) = got else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return;
    };
    let Some(msg) = nd6_msg_bytes(&m, at, o, icmp6len as usize) else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return;
    };
    let Some(nd_ns) = pod::at::<NdNeighborSolicit>(msg, 0) else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return;
    };
    let opts = &msg[fixed..];
    let mut taddr6 = nd_ns.nd_ns_target;

    let bad = 'out: {
        if hlim != 255 {
            break 'out true;
        }

        if in6_is_addr_unspecified(&saddr6) {
            // dst has to be solicited node multicast address.
            // don't check ifindex portion
            if !(daddr6.s6_addr16(0) == __IPV6_ADDR_INT16_MLL
                && daddr6.s6_addr32(1) == 0
                && daddr6.s6_addr32(2) == __IPV6_ADDR_INT32_ONE
                && daddr6.s6_addr8(12) == 0xff)
            {
                break 'out true;
            }
        } else {
            // Make sure the source address is from a neighbor's address.
            if !nd6_isneighbor(ifp, &saddr6, net) {
                break 'out true;
            }
        }

        if in6_is_addr_multicast(&taddr6) {
            break 'out true;
        }

        if in6_is_scope_embed(&taddr6) {
            taddr6.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        }

        let mut ndopts = NdOpts::default();
        if !nd6_options(opts, &mut ndopts) {
            // nd6_options have incremented stats
            break 'out false;
        }

        let (lladdr, lladdrlen) = match ndopts.nd_opts_src_lladdr {
            Some(off) => {
                let (l, n) = nd6_opt_lladdr(opts, off);
                (Some(l), n)
            }
            None => (None, 0),
        };

        if in6_is_addr_unspecified(&saddr6) && lladdr.is_some() {
            break 'out true;
        }

        // Attaching target link-layer address to the NA? (RFC 2461 7.2.4)
        //
        // NS IP dst is unicast/anycast			MUST NOT add
        // NS IP dst is solicited-node multicast	MUST add
        //
        // In implementation, we add target link-layer address by default. We do not add one
        // in MUST NOT cases.
        // (#if 0 in the C, too much!: no tlladdr when the destination is our anycast address.)
        let tlladdr = in6_is_addr_multicast(&daddr6);

        // Target address (taddr6) must be either:
        // (1) Valid unicast/anycast address for my receiving interface,
        // (2) Unicast address for which I'm offering proxy service, or
        // (3) "tentative" address on which DAD is being performed.
        // (1) and (3) check.
        let rd = net.reader();
        let mut ifa = in6ifa_ifpwithaddr(&rd, ifp, &taddr6);
        // NCARP > 0: a carp interface that is not the master ignores it; not configured.

        // (2) check.
        let mut proxy = false;
        let mut proxydl: Option<SockaddrDl> = None;
        if ifa.is_none() {
            let tsin6 = sin6_bytes(&taddr6);
            let rt = SockaddrView::new(&tsin6)
                .and_then(|v| rtalloc(v, 0, m.m_pkthdr().ph_rtableid.get(), net));
            if let Some(r) = &rt
                && r.rt_flags.isset(RTF_ANNOUNCE)
                && gw_family_len(r).0 == AF_LINK
            {
                // proxy NDP for single entry
                ifa = in6ifa_ifpforlinklocal(
                    &rd,
                    ifp,
                    IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED | IN6_IFF_ANYCAST,
                );
                if ifa.is_some() {
                    proxy = true;
                    // An `AF_LINK` gateway is a `sockaddr_dl`.
                    proxydl = gw_sdl_copy(r);
                    i_am_router = false; // XXX
                }
            }
            drop(rt); // rtfree
        }
        let Some(ifa) = ifa else {
            // We've got an NS packet, and we don't have that address assigned for us. We MUST
            // silently ignore it. See RFC2461 7.2.3.
            break 'out false;
        };
        let myaddr6 = ifa_in6(ifa);
        let ia6 = ifatoia6(ifa);
        let anycast = ia6.ia6_flags.get() & IN6_IFF_ANYCAST != 0;
        let tentative = ia6.ia6_flags.get() & IN6_IFF_TENTATIVE != 0;
        if ia6.ia6_flags.get() & IN6_IFF_DUPLICATED != 0 {
            break 'out false;
        }

        if lladdr.is_some() && ((i32::from(ifp.if_addrlen.get()) + 2 + 7) & !7) != lladdrlen {
            break 'out true;
        }

        if in6_are_addr_equal(&myaddr6, &saddr6) {
            log(
                LOG_INFO,
                format_args!("nd6_ns_input: duplicate IP6 address {}\n", In6Ntop(saddr6)),
            );
            break 'out false;
        }

        // We have neighbor solicitation packet, with target address equals to one of my
        // tentative address.
        //
        // src addr	how to process?
        // ---		---
        // multicast	of course, invalid (rejected in ip6_input)
        // unicast	somebody is doing address resolution -> ignore
        // unspec	dup address detection
        //
        // The processing is defined in RFC 2462.
        if tentative {
            // If source address is unspecified address, it is for duplicated address
            // detection.
            //
            // If not, the packet is for address resolution; silently ignore it.
            if in6_is_addr_unspecified(&saddr6) {
                nd6_dad_ns_input(ifa, net);
            }

            break 'out false;
        }

        let flags = if anycast || proxy || !tlladdr {
            0
        } else {
            ND_NA_FLAG_OVERRIDE
        } | if i_am_router { ND_NA_FLAG_ROUTER } else { 0 };

        // If the source address is unspecified address, entries must not be created or
        // updated. It looks that sender is performing DAD. Output NA toward all-node multicast
        // address, to tell the sender that I'm using the address. S bit ("solicited") must be
        // zero.
        if in6_is_addr_unspecified(&saddr6) {
            saddr6 = IN6ADDR_LINKLOCAL_ALLNODES;
            saddr6.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
            nd6_na_output(ifp, &saddr6, &taddr6, flags, tlladdr, proxydl.as_ref(), net);
            break 'out false;
        }

        nd6_cache_lladdr(
            ifp,
            &saddr6,
            lladdr,
            i32::from(ND_NEIGHBOR_SOLICIT),
            0,
            i_am_router,
            net,
        );

        nd6_na_output(
            ifp,
            &saddr6,
            &taddr6,
            flags | ND_NA_FLAG_SOLICITED,
            tlladdr,
            proxydl.as_ref(),
            net,
        );
        false
    };

    if bad {
        icmp6stat_inc(Icmp6statCounters::Icp6sBadns);
    }
    // freeit: m freed, if_put(ifp)
}

/// The `sockaddr_in6` image of `addr`, for the routes' requests.
#[inline]
fn sin6_bytes(addr: &In6Addr) -> [u8; size_of::<SockaddrIn6>()] {
    let mut b = [0u8; size_of::<SockaddrIn6>()];
    let _ = pod::put(&mut b, 0, &SockaddrIn6::with_addr(*addr));
    b
}

/// The packet `nd6_ns_output`/`nd6_na_output` build: a header mbuf (with a cluster when
/// `max_linkhdr + maxlen` does not fit) for `ifp`'s routing domain; `None` when out of mbufs.
fn nd6_getpkt(ifp: &Ifnet, maxlen: usize, func: &str) -> Option<MbufBox> {
    let max_linkhdr = usize::try_from(MAX_LINKHDR.load(Ordering::Relaxed)).unwrap_or(0);
    #[cfg(feature = "diagnostic")]
    if max_linkhdr + maxlen >= crate::net::mbuf::MCLBYTES {
        crate::prf::printf(format_args!(
            "{}: max_linkhdr + maxlen >= MCLBYTES ({} + {} > {})\n",
            func,
            max_linkhdr,
            maxlen,
            crate::net::mbuf::MCLBYTES
        ));
        kpanic(format_args!("{func}: insufficient MCLBYTES"));
    }
    let _ = func;

    let mut m = m_gethdr(M_DONTWAIT, MT_DATA)?;
    if max_linkhdr + maxlen >= MHLEN {
        mclget(&mut m, M_DONTWAIT);
        if m.flags() & M_EXT == 0 {
            return None; // m_free(m): the drop
        }
    }
    m.m_pkthdr().ph_rtableid.set(ifp.if_rdomain.get());
    Some(m)
}

/// The link-layer address option of `type_` carrying `mac` (`if_addrlen` bytes, zeros past a
/// shorter one) written at the start of `buf`, which has room for it; the option's length.
fn nd6_put_lladdr_opt(buf: &mut [u8], type_: u8, mac: &[u8], ifp: &Ifnet) -> usize {
    let addrlen = usize::from(ifp.if_addrlen.get());
    // 8 byte alignments...
    let optlen = (size_of::<NdOptHdr>() + addrlen + 7) & !7;
    let n = addrlen.min(mac.len());
    let opt = &mut buf[..optlen];
    opt.fill(0);
    opt[0] = type_;
    opt[1] = (optlen >> 3) as u8;
    opt[size_of::<NdOptHdr>()..size_of::<NdOptHdr>() + n].copy_from_slice(&mac[..n]);
    optlen
}

/// The length of the link-layer address option `ifp`'s address takes (`(sizeof(struct
/// nd_opt_hdr) + ifp->if_addrlen + 7) & ~7`).
fn lladdr_optlen(ifp: &Ifnet) -> usize {
    (size_of::<NdOptHdr>() + usize::from(ifp.if_addrlen.get()) + 7) & !7
}

/// An IPv6 header for ND output: version 6, ICMPv6, hop limit 255, `ip6_plen` the message's.
fn nd6_ip6hdr(src: In6Addr, dst: In6Addr, icmp6len: usize) -> Ip6Hdr {
    let mut ip6 = Ip6Hdr {
        ip6_flow: 0,
        ip6_plen: htons(icmp6len as u16),
        ip6_nxt: IPPROTO_ICMPV6 as u8,
        ip6_hlim: 255,
        ip6_src: src,
        ip6_dst: dst,
    };
    ip6.set_ip6_vfc(IPV6_VERSION);
    ip6
}

/// `ip6_output(m, NULL, NULL, flags, &im6o, NULL)` of an ND message: no options; for a
/// multicast destination out of `ifp` with hop limit 255 and no loopback (the C fills its
/// stack `ip6_moptions` only then, and `ip6_output` reads it only then, so a unicast message
/// lends none). Under the net lock (`net`).
fn nd6_ip6_output(m: MbufBox, flags: i32, ifp: &Ifnet, mcast: bool, net: &NetHeld<'_>) {
    let im6o = mcast.then(|| Ip6Moptions::output(ifp.if_index.get() as u16, 255, 0));
    let _ = ip6_output(m, None, None, flags, im6o.as_ref(), None, net);
}

/// `nd6_ns_output(ifp, daddr6, taddr6, ln, dad)`: output a Neighbor Solicitation Message.
/// Caller specifies:
/// - ICMP6 header source IP6 address (`saddr6`, the prompting packet's: the C's `ln`, for
///   source address determination)
/// - ND6 header target IP6 address (`taddr6`)
/// - ND6 header source datalink address
///
/// `daddr6` `None` means the solicited-node multicast address of `taddr6`; `dad`: a duplicated
/// address detection probe. Based on RFC 2461 and RFC 2462. Under the net lock (`net`).
pub fn nd6_ns_output(
    ifp: &'static Ifnet,
    daddr6: Option<&In6Addr>,
    taddr6: &In6Addr,
    saddr6: Option<&In6Addr>,
    dad: bool,
    net: &NetHeld<'_>,
) {
    if in6_is_addr_multicast(taddr6) {
        return;
    }

    // estimate the size of message
    let maxlen = IP6_HDR_LEN + size_of::<NdNeighborSolicit>() + lladdr_optlen(ifp);

    let Some(mut m) = nd6_getpkt(ifp, maxlen, "nd6_ns_output") else {
        return;
    };
    m.m_pkthdr().ph_ifidx.set(0);

    let mcast = daddr6.is_none_or(in6_is_addr_multicast);
    if mcast {
        m.m_flags().set(m.flags() | M_MCAST);
    }

    // determine the source and destination addresses
    let dst = if let Some(d) = daddr6 {
        *d
    } else {
        let mut a = In6Addr::default();
        a.set_s6_addr16(0, __IPV6_ADDR_INT16_MLL);
        a.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        a.set_s6_addr32(1, 0);
        a.set_s6_addr32(2, __IPV6_ADDR_INT32_ONE);
        a.set_s6_addr32(3, taddr6.s6_addr32(3));
        a.s6_addr[12] = 0xff;
        a
    };
    let src = if dad {
        // Source address for DAD packet must always be IPv6 unspecified address. (0::0) We
        // actually don't have to 0-clear the address (we did it above), but we do so here
        // explicitly to make the intention clearer.
        In6Addr::default()
    } else {
        // RFC2461 7.2.2:
        // "If the source address of the packet prompting the solicitation is the same as one
        // of the addresses assigned to the outgoing interface, that address SHOULD be placed
        // in the IP Source Address of the outgoing solicitation. Otherwise, any one of the
        // addresses assigned to the interface should be used."
        //
        // We use the source address for the prompting packet (saddr6), if:
        // - saddr6 is given from the caller (by giving "ln"), and
        // - saddr6 belongs to the outgoing interface and
        // - if taddr is link local saddr6 must be link local as well
        // Otherwise, we perform the source address selection as usual.
        let s = saddr6.copied().unwrap_or_default();

        if !in6_is_addr_linklocal(taddr6)
            || in6_is_addr_unspecified(&s)
            || in6_is_addr_linklocal(&s)
            || in6ifa_ifpwithaddr(&net.reader(), ifp, &s).is_none()
        {
            let dst_sa = sin6_bytes(&dst);
            let rt = SockaddrView::new(&dst_sa)
                .and_then(|v| rtalloc(v, RT_RESOLVE, m.m_pkthdr().ph_rtableid.get(), net));
            // A valid route has its address (`rt_ifa`).
            let rd = net.reader();
            let Some(ia6) = rt
                .as_deref()
                .filter(|r| rtisvalid(Some(*r)))
                .and_then(|r| r.ifa(&rd))
                .map(ifatoia6)
            else {
                // bad: m freed, rtfree(rt)
                return;
            };
            ia6.ia_addr.addr()
        } else {
            s
        }
    };

    let mut nd_ns = NdNeighborSolicit {
        nd_ns_hdr: Icmp6Hdr::zeroed(),
        nd_ns_target: *taddr6,
    };
    nd_ns.set_nd_ns_type(ND_NEIGHBOR_SOLICIT);
    nd_ns.set_nd_ns_code(0);
    nd_ns.set_nd_ns_reserved(0);
    if in6_is_scope_embed(&nd_ns.nd_ns_target) {
        nd_ns.nd_ns_target.set_s6_addr16(1, 0);
    }
    nd_ns.set_nd_ns_cksum(0);

    // Add source link-layer address option.
    //
    //				spec		implementation
    //				---		---
    // DAD packet			MUST NOT	do not add the option
    // there's no link layer address:
    //				impossible	do not add the option
    // there's link layer address:
    //	Multicast NS		MUST add one	add the option
    //	Unicast NS		SHOULD add one	add the option
    let mac = if dad { None } else { nd6_ifptomac(ifp) };
    let mut icmp6len = size_of::<NdNeighborSolicit>();
    if mac.is_some() {
        icmp6len += lladdr_optlen(ifp);
    }

    m.m_len().set((IP6_HDR_LEN + icmp6len) as u32);
    m.m_pkthdr().len.set((IP6_HDR_LEN + icmp6len) as i32);
    m_align(&m, maxlen as i32);

    // fill neighbor solicitation packet: the fresh mbuf written in place; without the room,
    // nothing is sent.
    let ip6 = nd6_ip6hdr(src, dst, icmp6len);
    let Some(d) = m.data_mut() else {
        return;
    };
    if !pod::put(d, 0, &ip6) || !pod::put(d, IP6_HDR_LEN, &nd_ns) {
        return;
    }
    if let Some(mac) = mac {
        let at = IP6_HDR_LEN + size_of::<NdNeighborSolicit>();
        let _ = nd6_put_lladdr_opt(&mut d[at..], ND_OPT_SOURCE_LINKADDR, &mac, ifp);
    }
    m.m_pkthdr()
        .csum_flags
        .set(m.m_pkthdr().csum_flags.get() | M_ICMP_CSUM_OUT);

    nd6_ip6_output(m, if dad { IPV6_UNSPECSRC } else { 0 }, ifp, mcast, net);
    icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, ND_NEIGHBOR_SOLICIT);
}

/// `nd6_na_input(m, off, icmp6len)`: neighbor advertisement input handling (`icmp6len` bytes
/// at `off` of `m`, consumed). Based on RFC 2461 and RFC 2462 (duplicated address detection).
/// Under the exclusive net lock (`net`).
///
/// the following items are not implemented yet:
/// - proxy advertisement delay rule (RFC2461 7.2.8, last paragraph, SHOULD)
/// - anycast advertisement delay rule (RFC2461 7.2.7, SHOULD)
pub fn nd6_na_input(m: MbufBox, off: i32, icmp6len: i32, net: &NetHeld<'_>) {
    let i_am_router = IP6_FORWARDING.load(Ordering::Relaxed) != 0;

    kassert!(hal::netlock::status() == hal::rwlock::RW_WRITE);

    let Some(ifp) = IfRef::get(m.m_pkthdr().ph_ifidx.get()) else {
        return; // m freed
    };
    let ifp = ifp.ifp();
    let Some((hlim, src, daddr6)) = ip6_fields(&m) else {
        return;
    };

    if hlim != 255 {
        icmp6stat_inc(Icmp6statCounters::Icp6sBadna);
        return; // m freed, if_put(ifp)
    }

    let fixed = size_of::<NdNeighborAdvert>();
    let got = if (icmp6len as usize) < fixed {
        None
    } else {
        nd6_msg(m, off, icmp6len)
    };
    let Some((m, at, o)) = got else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return;
    };
    let Some(msg) = nd6_msg_bytes(&m, at, o, icmp6len as usize) else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return;
    };
    let Some(nd_na) = pod::at::<NdNeighborAdvert>(msg, 0) else {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return;
    };
    let opts = &msg[fixed..];
    let mut taddr6 = nd_na.nd_na_target;
    let flags = nd_na.nd_na_flags_reserved();
    let is_solicited = flags & ND_NA_FLAG_SOLICITED != 0;

    let bad = 'out: {
        if in6_is_scope_embed(&taddr6) {
            taddr6.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        }

        if in6_is_addr_multicast(&taddr6) {
            break 'out true;
        }
        if is_solicited && in6_is_addr_multicast(&daddr6) {
            break 'out true;
        }

        let mut ndopts = NdOpts::default();
        if !nd6_options(opts, &mut ndopts) {
            // nd6_options have incremented stats
            break 'out false;
        }

        if in6_is_addr_multicast(&daddr6) && ndopts.nd_opts_tgt_lladdr.is_none() {
            break 'out true;
        }

        let (lladdr, lladdrlen) = match ndopts.nd_opts_tgt_lladdr {
            Some(off) => {
                let (l, n) = nd6_opt_lladdr(opts, off);
                (Some(l), n)
            }
            None => (None, 0),
        };

        let rd = net.reader();
        let ifa = in6ifa_ifpwithaddr(&rd, ifp, &taddr6);

        // Target address matches one of my interface address.
        //
        // If my address is tentative, this means that there's somebody already using the same
        // address as mine. This indicates DAD failure. This is defined in RFC 2462.
        //
        // Otherwise, process as defined in RFC 2461.
        if let Some(ifa) = ifa
            && ifatoia6(ifa).ia6_flags.get() & IN6_IFF_TENTATIVE != 0
        {
            if let Some(dp) = nd6_dad_find(ifa) {
                dp.dad_na_icount.set(dp.dad_na_icount.get() + 1);

                // remove the address.
                nd6_dad_duplicated(dp, ifa, net);
            }
            break 'out false;
        }

        if ifa.is_some() {
            // NCARP > 0: ignore NAs silently for carp addresses if we're not the CARP master;
            // not configured.
            log(
                LOG_ERR,
                format_args!("nd6_na_input: duplicate IP6 address {}\n", In6Ntop(taddr6)),
            );
            break 'out false;
        }

        if lladdr.is_some() && ((i32::from(ifp.if_addrlen.get()) + 2 + 7) & !7) != lladdrlen {
            break 'out true;
        }

        // Check if we already have this neighbor in our cache.
        let rt = nd6_lookup(&taddr6, false, Some(ifp), ifp.if_rdomain.get(), net);

        // If we are a router, we may create new stale cache entries upon receiving Unsolicited
        // Neighbor Advertisements.
        if rt.is_none() && i_am_router {
            let Some(rt) = nd6_lookup(&taddr6, true, Some(ifp), ifp.if_rdomain.get(), net) else {
                break 'out false;
            };
            let Some(l) = lladdr else {
                break 'out false;
            };
            if !rt.with_gateway(|g| g.is_some()) {
                break 'out false;
            }
            let g = ND6_MTX.lock();
            if let Some(ln) = rt.rt_llinfo.get_b(&g) {
                // The gateway of a neighbor route `nd6_lookup` validated is a `sockaddr_dl`
                // with room for the interface's address.
                nd6_llstore(l, &rt, ifp);

                // RFC9131 6.1.1
                //
                // Routers SHOULD create a new entry for the target address with the
                // reachability state set to STALE.
                ln.ln_state.set(ND6_LLINFO_STALE);
                nd6_llinfo_settimer(ln, ND6_GCTIMER as u32);
            }
            break 'out false;
        }

        // Host: If no neighbor cache entry is found, NA SHOULD silently be discarded.
        if let Some(rt) = &rt {
            nd6_na_cache(ifp, rt, lladdr, flags, i_am_router, &taddr6, &src, net);
        }
        false
    };

    if bad {
        icmp6stat_inc(Icmp6statCounters::Icp6sBadna);
    }
    // freeit: the route's reference dropped (rtfree), m freed, if_put(ifp)
}

/// What `nd6_na_cache`'s section decided, for after the mutex.
struct NaTail {
    /// `rtm_send(rt, RTM_RESOLVE)`: a new entry became reachable.
    notify: bool,
    /// `rt6_flush` of the advertisement's source: a router stopped being one.
    flush: bool,
    /// The held packets, delisted (`if_output_mq`).
    held: MbufList,
    /// The link-layer address changed (the "ndp info overwritten" line).
    overwritten: bool,
}

/// `nd6_na_input` from the neighbor cache entry `rt` of the target `taddr6` on: updates it from
/// an advertisement (`flags`, its `ND_NA_FLAG_*` word: router, solicited, override; target
/// link-layer address `lladdr`) sent from `src`, and sends what it holds once usable. The record under one
/// `ND6_MTX` section, the routing message, `rt6_flush` and the output after it (the C's order:
/// what they read is not what the section changes after them in the C).
#[allow(clippy::too_many_arguments)] // the locals of the C function the code came from
pub fn nd6_na_cache(
    ifp: &'static Ifnet,
    rt: &PoolArc<Rtentry>,
    lladdr: Option<&[u8]>,
    flags: u32,
    i_am_router: bool,
    taddr6: &In6Addr,
    src: &In6Addr,
    net: &NetHeld<'_>,
) {
    let is_router = flags & ND_NA_FLAG_ROUTER != 0;
    let is_solicited = flags & ND_NA_FLAG_SOLICITED != 0;
    let is_override = flags & ND_NA_FLAG_OVERRIDE != 0;
    if !rt.with_gateway(|g| g.is_some()) {
        return;
    }

    let tail = {
        let g = ND6_MTX.lock();
        let Some(ln) = rt.rt_llinfo.get_b(&g) else {
            return;
        };
        match na_update(ifp, rt, ln, lladdr, is_router, is_solicited, is_override) {
            None => return, // goto freeit
            Some((notify, overwritten, flush)) => {
                ln.ln_asked.set(0);
                NaTail {
                    notify,
                    flush: flush && !i_am_router,
                    held: mq_delist(&ln.ln_mq),
                    overwritten,
                }
            }
        }
    };

    if tail.overwritten
        && let Some(l) = lladdr
    {
        let s = nd6_ether_sprintf(l);
        log(
            LOG_INFO,
            format_args!(
                "ndp info overwritten for {} by {} on {}\n",
                In6Ntop(*taddr6),
                Str(&s),
                Str(&ifp.if_xname.get())
            ),
        );
    }
    if tail.notify {
        // Notify userland that a new ND entry is reachable.
        rtm_send(rt, RTM_RESOLVE, 0, ifp.if_rdomain.get(), net);
    }
    if tail.flush {
        // The neighbor may be used as a next hop for some destinations (e.g. redirect case).
        // So we must call rt6_flush explicitly.
        let _ = rt6_flush(src, ifp, net);
    }
    rt.rt_flags.clear(RTF_REJECT);
    nd6_output_held(ifp, rt, tail.held);
}

/// `nd6_na_cache`'s state transitions of the record `ln` of `rt` (lent under `ND6_MTX`):
/// `None` for the C's `goto freeit` (the advertisement changes nothing more), else whether the
/// routing socket is told (`notify`), whether the address was overwritten, and whether the
/// entry stopped being a router's (`rt6_flush`'s condition but for `i_am_router`).
fn na_update(
    ifp: &Ifnet,
    rt: &Rtentry,
    ln: &LlinfoNd6,
    lladdr: Option<&[u8]>,
    is_router: bool,
    is_solicited: bool,
    is_override: bool,
) -> Option<(bool, bool, bool)> {
    if ln.ln_state() == ND6_LLINFO_INCOMPLETE {
        // If the link-layer has address, and no lladdr option came, discard the packet.
        if ifp.if_addrlen.get() != 0 && lladdr.is_none() {
            return None;
        }

        // Record link-layer address, and update the state. The gateway of a neighbor route is
        // a `sockaddr_dl` with room for the interface's address (`nd6_lookup` validated it).
        nd6_llstore(lladdr.unwrap_or(&[]), rt, ifp);
        let notify = is_solicited;
        if is_solicited {
            ln.ln_state.set(ND6_LLINFO_REACHABLE);
            if !ln.permanent() {
                nd6_llinfo_settimer(ln, ifp.if_nd.reachable());
            }
        } else {
            ln.ln_state.set(ND6_LLINFO_STALE);
            nd6_llinfo_settimer(ln, ND6_GCTIMER as u32);
        }
        ln.ln_router.set(i16::from(is_router));
        if is_router {
            // This means a router's state has changed from non-reachable to probably
            // reachable, and might affect the status of associated prefixes..
            if !rt.rt_flags.isset(RTF_LLINFO) {
                return None; // ln is gone
            }
        }
        return Some((notify, false, false));
    }

    // Check if the link-layer address has changed or not.
    let llchange = match lladdr {
        None => false,
        Some(l) => nd6_lladdr_alen(rt) == 0 || nd6_llchanged(l, rt, ifp),
    };

    // This is VERY complex.  Look at it with care.
    //
    // override solicit lladdr llchange	action
    //					(L: record lladdr)
    //
    //	0	0	n	--	(2c)
    //	0	0	y	n	(2b) L
    //	0	0	y	y	(1)    REACHABLE->STALE
    //	0	1	n	--	(2c)   *->REACHABLE
    //	0	1	y	n	(2b) L *->REACHABLE
    //	0	1	y	y	(1)    REACHABLE->STALE
    //	1	0	n	--	(2a)
    //	1	0	y	n	(2a) L
    //	1	0	y	y	(2a) L *->STALE
    //	1	1	n	--	(2a)   *->REACHABLE
    //	1	1	y	n	(2a) L *->REACHABLE
    //	1	1	y	y	(2a) L *->REACHABLE
    let mut overwritten = false;
    if !is_override && lladdr.is_some() && llchange {
        // (1)
        // If state is REACHABLE, make it STALE. no other updates should be done.
        if ln.ln_state() == ND6_LLINFO_REACHABLE {
            ln.ln_state.set(ND6_LLINFO_STALE);
            nd6_llinfo_settimer(ln, ND6_GCTIMER as u32);
        }
        return None;
    } else if is_override // (2a)
        || !llchange // (2b): with (1) above, an address given and unchanged
        || lladdr.is_none()
    // (2c)
    {
        // Update link-local address, if any.
        overwritten = llchange;
        if let Some(l) = lladdr {
            nd6_llstore(l, rt, ifp);
        }

        // If solicited, make the state REACHABLE. If not solicited and the link-layer address
        // was changed, make it STALE.
        if is_solicited {
            ln.ln_state.set(ND6_LLINFO_REACHABLE);
            if !ln.permanent() {
                nd6_llinfo_settimer(ln, ifp.if_nd.reachable());
            }
        } else if lladdr.is_some() && llchange {
            ln.ln_state.set(ND6_LLINFO_STALE);
            nd6_llinfo_settimer(ln, ND6_GCTIMER as u32);
        }
    }

    let flush = ln.ln_router() != 0 && !is_router;
    ln.ln_router.set(i16::from(is_router));
    Some((false, overwritten, flush))
}

/// `nd6_na_output(ifp, daddr6, taddr6, flags, tlladdr, sdl0)`: neighbor advertisement output
/// handling. Based on RFC 2461. `tlladdr`: include the target link-layer address; `sdl0`: the
/// `sockaddr_dl` of a proxy NA, or `None`. `flags` in network order. Under the net lock
/// (`net`).
///
/// the following items are not implemented yet:
/// - proxy advertisement delay rule (RFC2461 7.2.8, last paragraph, SHOULD)
/// - anycast advertisement delay rule (RFC2461 7.2.7, SHOULD)
pub fn nd6_na_output(
    ifp: &'static Ifnet,
    daddr6: &In6Addr,
    taddr6: &In6Addr,
    flags: u32,
    tlladdr: bool,
    sdl0: Option<&SockaddrDl>,
    net: &NetHeld<'_>,
) {
    let mut flags = flags;

    // NCARP > 0: do not send NAs for carp addresses if we're not the CARP master; not
    // configured.

    // estimate the size of message
    let maxlen = IP6_HDR_LEN + size_of::<NdNeighborAdvert>() + lladdr_optlen(ifp);

    let Some(mut m) = nd6_getpkt(ifp, maxlen, "nd6_na_output") else {
        return;
    };

    let mcast = in6_is_addr_multicast(daddr6);
    if mcast {
        m.m_flags().set(m.flags() | M_MCAST);
    }

    let mut dst = *daddr6;
    if in6_is_addr_unspecified(daddr6) {
        // reply to DAD
        dst.set_s6_addr16(0, __IPV6_ADDR_INT16_MLL);
        dst.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        dst.set_s6_addr32(1, 0);
        dst.set_s6_addr32(2, 0);
        dst.set_s6_addr32(3, __IPV6_ADDR_INT32_ONE);

        flags &= !ND_NA_FLAG_SOLICITED;
    }

    // Select a source whose scope is the same as that of the dest.
    let dst_sa = sin6_bytes(&dst);
    let rt =
        SockaddrView::new(&dst_sa).and_then(|v| rtalloc(v, RT_RESOLVE, ifp.if_rdomain.get(), net));
    // A valid route has its address (`rt_ifa`).
    let rd = net.reader();
    let Some(ia6) = rt
        .as_deref()
        .filter(|r| rtisvalid(Some(*r)))
        .and_then(|r| r.ifa(&rd))
        .map(ifatoia6)
    else {
        // bad: m freed, rtfree(rt)
        return;
    };
    let src = ia6.ia_addr.addr();
    drop(rt); // rtfree

    let mut nd_na = NdNeighborAdvert {
        nd_na_hdr: Icmp6Hdr::zeroed(),
        nd_na_target: *taddr6,
    };
    nd_na.set_nd_na_type(ND_NEIGHBOR_ADVERT);
    nd_na.set_nd_na_code(0);
    if in6_is_scope_embed(&nd_na.nd_na_target) {
        nd_na.nd_na_target.set_s6_addr16(1, 0);
    }

    // "tlladdr" indicates NS's condition for adding tlladdr or not. see nd6_ns_input() for
    // details. Basically, if NS packet is sent to unicast/anycast addr, target lladdr option
    // SHOULD NOT be included.
    let mut mac: Option<[u8; ETHER_ADDR_LEN]> = None;
    let mut proxy_mac: Option<&[u8]> = None;
    if tlladdr {
        // sdl0 != NULL indicates proxy NA. If we do proxy, use lladdr in sdl0. If we are not
        // proxying (sending NA for my address) use lladdr configured for the interface.
        match sdl0 {
            None => mac = nd6_ifptomac(ifp),
            Some(sdl) if sdl.sdl_family == AF_LINK && sdl.sdl_alen == ifp.if_addrlen.get() => {
                // A `sockaddr_dl` holds `sdl_alen` address bytes after its name.
                let n = usize::from(sdl.sdl_nlen);
                proxy_mac = sdl.sdl_data.get(n..n + usize::from(sdl.sdl_alen));
            }
            Some(_) => {}
        }
    }
    let lladdr: Option<&[u8]> = mac.as_ref().map(|m| &m[..]).or(proxy_mac);
    let lladdr = lladdr.filter(|_| tlladdr);

    let mut icmp6len = size_of::<NdNeighborAdvert>();
    match lladdr {
        Some(_) => icmp6len += lladdr_optlen(ifp),
        None => flags &= !ND_NA_FLAG_OVERRIDE,
    }
    nd_na.set_nd_na_flags_reserved(flags);
    nd_na.set_nd_na_cksum(0);

    m.m_len().set((IP6_HDR_LEN + icmp6len) as u32);
    m.m_pkthdr().len.set((IP6_HDR_LEN + icmp6len) as i32);
    m_align(&m, maxlen as i32);

    // fill neighbor advertisement packet: the fresh mbuf written in place; without the room,
    // nothing is sent.
    let ip6 = nd6_ip6hdr(src, dst, icmp6len);
    let Some(d) = m.data_mut() else {
        return;
    };
    if !pod::put(d, 0, &ip6) || !pod::put(d, IP6_HDR_LEN, &nd_na) {
        return;
    }
    if let Some(l) = lladdr {
        let at = IP6_HDR_LEN + size_of::<NdNeighborAdvert>();
        let _ = nd6_put_lladdr_opt(&mut d[at..], ND_OPT_TARGET_LINKADDR, l, ifp);
    }
    m.m_pkthdr()
        .csum_flags
        .set(m.m_pkthdr().csum_flags.get() | M_ICMP_CSUM_OUT);

    nd6_ip6_output(m, 0, ifp, mcast, net);
    icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, ND_NEIGHBOR_ADVERT);
}

/// `nd6_ifptomac(ifp)`: the link-layer address of `ifp` for ND options (the `struct arpcom`
/// interfaces of the C's list).
pub fn nd6_ifptomac(ifp: &Ifnet) -> Option<[u8; ETHER_ADDR_LEN]> {
    match ifp.if_type.get() {
        IFT_ETHER | IFT_IEEE1394 | IFT_PROPVIRTUAL | IFT_CARP | IFT_IEEE80211 => {
            ifp.if_arpcom.get().map(|ac| ac.ac_enaddr.get())
        }
        _ => None,
    }
}

/// `nd6_dad_find(ifa)`: the DAD state of `ifa`, if DAD runs on it. Under the net lock.
fn nd6_dad_find(ifa: &Ifaddr) -> Option<&'static Dadq> {
    DADQ.iter()
        .find(|dp| dp.dad_ifa.read().as_ref().is_some_and(|a| a.is(ifa)))
}

/// `nd6_dad_destroy(dp)`: takes `dp` off the list and hands it to the reaper (the C sets its
/// timeout again to `nd6_dad_reaper`: the module's redesign).
fn nd6_dad_destroy(dp: &'static Dadq) {
    DADQ.remove(dp);
    IP6_DAD_PENDING.fetch_sub(1, Ordering::Relaxed);
    DAD_REAP.lock().insert_tail(dp);
    let _ = Pin::static_ref(&DAD_REAPER).add(0);
}

/// `nd6_dad_reaper`: the records `nd6_dad_destroy` queued, once their timeout is neither
/// pending nor running (`del_barrier`), given their address's reference back and put on the
/// free list (the C's `ifafree` and `free`).
fn nd6_dad_reaper(_arg: *mut c_void) {
    loop {
        let dp = DAD_REAP.lock().pop_head();
        let Some(dp) = dp else {
            break;
        };
        let _ = dp.dad_timer_ch.del_barrier();
        if let Some(ifa) = dp.dad_ifa.replace(None) {
            ifafree(ifa);
        }
        DAD_FREE.lock().insert_tail(dp);
    }
}

/// `nd6_dad_starttimer(dp)`: the next DAD step in `RETRANS_TIMER` milliseconds.
fn nd6_dad_starttimer(dp: &'static Dadq) {
    let _ = Pin::static_ref(&dp.dad_timer_ch).add_msec(u64::from(RETRANS_TIMER));
}

/// `nd6_dad_stoptimer(dp)`.
fn nd6_dad_stoptimer(dp: &Dadq) {
    let _ = dp.dad_timer_ch.del();
}

/// A DAD record: one of the free list, or a new one (`malloc(sizeof(*dp), M_IP6NDP, M_NOWAIT
/// | M_ZERO)`, its timeout's argument its own address); `None` when the allocator has none.
fn dadq_alloc() -> Option<&'static Dadq> {
    let reused = DAD_FREE.lock().pop_head();
    if reused.is_some() {
        return reused;
    }
    let dp = MBox::leak(MBox::new(M_IP6NDP, M_NOWAIT, Dadq::new())?);
    // The argument is the record's address as an integer, compared and never followed.
    dp.dad_timer_ch.set(
        Some(nd6_dad_timer),
        ptr::without_provenance_mut(ptr::from_ref(dp).addr()),
        KCLOCK_NONE,
        TIMEOUT_PROC,
    );
    Some(dp)
}

/// `nd6_dad_start(ifa)`: start Duplicated Address Detection (DAD) for specified interface
/// address. Under the net lock (`net`).
pub fn nd6_dad_start(ifa: &IfaddrRef, net: &NetHeld<'_>) {
    let ia6 = ifatoia6(ifa);
    let ip6_dad_count_local = IP6_DAD_COUNT.load(Ordering::Relaxed);

    kassert!(matches!(
        hal::netlock::status(),
        hal::rwlock::RW_WRITE | hal::rwlock::RW_READ
    ));

    // If we don't need DAD, don't do it. There are several cases:
    // - DAD is disabled (ip6_dad_count == 0)
    // - the interface address is anycast
    kassert!(ia6.ia6_flags.get() & IN6_IFF_TENTATIVE != 0);
    if ia6.ia6_flags.get() & IN6_IFF_ANYCAST != 0 || ip6_dad_count_local == 0 {
        ia6.ia6_flags.set(ia6.ia6_flags.get() & !IN6_IFF_TENTATIVE);

        rtm_addr(RTM_CHGADDRATTR, ifa, net);

        return;
    }

    // DAD already in progress
    if nd6_dad_find(ifa).is_some() {
        return;
    }

    let Some(dp) = dadq_alloc() else {
        log(
            LOG_ERR,
            format_args!(
                "nd6_dad_start: memory allocation failed for {}({})\n",
                In6Ntop(ia6.ia_addr.addr()),
                Str(&ifa
                    .ifa_ifp
                    .get()
                    .map_or(UNKNOWN_IFNAME, |ifp| ifp.if_xname.get()))
            ),
        );
        return;
    };

    // The address first (the C stores it after the insert, with nothing between them under
    // the net lock): a record on DADQ always names its address.
    drop(dp.dad_ifa.replace(Some(ifaref(ifa))));
    DADQ.insert_tail(dp);
    IP6_DAD_PENDING.fetch_add(1, Ordering::Relaxed);

    // Send NS packet for DAD, ip6_dad_count times. Note that we must delay the first
    // transmission, if this is the first packet to be sent from the interface after interface
    // (re)initialization.
    dp.dad_count.set(ip6_dad_count_local);
    dp.dad_ns_icount.set(0);
    dp.dad_na_icount.set(0);
    dp.dad_ns_ocount.set(0);
    dp.dad_ns_tcount.set(0);
    nd6_dad_ns_output(dp, ifa, net);
    nd6_dad_starttimer(dp);
}

/// `nd6_dad_stop(ifa)`: terminate DAD unconditionally. used for address removals. Under the
/// net lock (its proof, `_net`).
pub fn nd6_dad_stop(ifa: &Ifaddr, _net: &NetHeld<'_>) {
    let Some(dp) = nd6_dad_find(ifa) else {
        // DAD wasn't started yet
        return;
    };

    nd6_dad_stoptimer(dp);
    nd6_dad_destroy(dp);
}

/// `nd6_dad_timer(arg)`: one DAD step: another solicitation, or the verdict. `arg` is the
/// record's address as an integer (the module's redesign), found on `DADQ` under the net lock.
fn nd6_dad_timer(arg: *mut c_void) {
    let w = NETLOCK.write();
    let Some(net) = NetHeld::of(&w) else {
        return;
    };

    'done: {
        let Some(dp) = DADQ
            .iter()
            .find(|dp| ptr::from_ref(*dp).addr() == arg.addr())
        else {
            log(
                LOG_ERR,
                format_args!("nd6_dad_timer: DAD structure not found\n"),
            );
            break 'done;
        };
        let ifa = dp.ifa();
        let ifa = &ifa;
        let ia6 = ifatoia6(ifa);
        let taddr6 = ia6.ia_addr.addr();
        let ifname = ifa
            .ifa_ifp
            .get()
            .map_or(UNKNOWN_IFNAME, |ifp| ifp.if_xname.get());
        if ia6.ia6_flags.get() & IN6_IFF_DUPLICATED != 0 {
            log(
                LOG_ERR,
                format_args!(
                    "nd6_dad_timer: called with duplicated address {}({})\n",
                    In6Ntop(taddr6),
                    Str(&ifname)
                ),
            );
            break 'done;
        }
        if ia6.ia6_flags.get() & IN6_IFF_TENTATIVE == 0 {
            log(
                LOG_ERR,
                format_args!(
                    "nd6_dad_timer: called with non-tentative address {}({})\n",
                    In6Ntop(taddr6),
                    Str(&ifname)
                ),
            );
            break 'done;
        }

        // timeouted with IFF_{RUNNING,UP} check
        if dp.dad_ns_tcount.get() > DAD_MAXTRY.load(Ordering::Relaxed) {
            nd6_dad_destroy(dp);
            break 'done;
        }

        // Need more checks?
        if dp.dad_ns_ocount.get() < dp.dad_count.get() {
            // We have more NS to go. Send NS packet for DAD.
            nd6_dad_ns_output(dp, ifa, &net);
            nd6_dad_starttimer(dp);
        } else {
            // We have transmitted sufficient number of DAD packets.
            if dp.dad_na_icount.get() != 0 || dp.dad_ns_icount.get() != 0 {
                // dp will be freed in nd6_dad_duplicated()
                nd6_dad_duplicated(dp, ifa, &net);
            } else {
                // We are done with DAD. No NA came, no NS came.
                ia6.ia6_flags.set(ia6.ia6_flags.get() & !IN6_IFF_TENTATIVE);

                rtm_addr(RTM_CHGADDRATTR, ifa, &net);

                if let Some(ifp) = ifa.ifa_ifp.get() {
                    let mut daddr6 = IN6ADDR_LINKLOCAL_ALLROUTERS;
                    daddr6.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
                    // RFC9131 - inform routers about our new address
                    nd6_na_output(ifp, &daddr6, &taddr6, 0, true, None, &net);
                }

                nd6_dad_destroy(dp);
            }
        }
    }
}

/// `nd6_dad_duplicated(dp)`: DAD found the address in use: mark it duplicated and stop.
fn nd6_dad_duplicated(dp: &'static Dadq, ifa: &IfaddrRef, net: &NetHeld<'_>) {
    let ia6 = ifatoia6(ifa);
    let ifname = ia6
        .ia_ifp()
        .get()
        .map_or(UNKNOWN_IFNAME, |ifp| ifp.if_xname.get());
    let addr = ia6.ia_addr.addr();

    log(
        LOG_ERR,
        format_args!(
            "{}: DAD detected duplicate IPv6 address {}: NS in/out={}/{}, NA in={}\n",
            Str(&ifname),
            In6Ntop(addr),
            dp.dad_ns_icount.get(),
            dp.dad_ns_ocount.get(),
            dp.dad_na_icount.get()
        ),
    );

    ia6.ia6_flags.set(ia6.ia6_flags.get() & !IN6_IFF_TENTATIVE);
    ia6.ia6_flags.set(ia6.ia6_flags.get() | IN6_IFF_DUPLICATED);

    // We are done with DAD, with duplicated address found. (failure)
    nd6_dad_stoptimer(dp);

    log(
        LOG_ERR,
        format_args!(
            "{}: DAD complete for {} - duplicate found\n",
            Str(&ifname),
            In6Ntop(addr)
        ),
    );
    log(
        LOG_ERR,
        format_args!("{}: manual intervention required\n", Str(&ifname)),
    );

    rtm_addr(RTM_CHGADDRATTR, ifa, net);

    nd6_dad_destroy(dp);
}

/// `nd6_dad_ns_output(dp, ifa)`: one DAD solicitation for `ifa`, if its interface is up and
/// running.
fn nd6_dad_ns_output(dp: &Dadq, ifa: &IfaddrRef, net: &NetHeld<'_>) {
    let ia6 = ifatoia6(ifa);
    let Some(ifp) = ifa.ifa_ifp.get() else {
        return;
    };

    dp.dad_ns_tcount.set(dp.dad_ns_tcount.get() + 1);
    if ifp.if_flags.get() & IFF_UP == 0 {
        return;
    }
    if ifp.if_flags.get() & IFF_RUNNING == 0 {
        return;
    }

    dp.dad_ns_ocount.set(dp.dad_ns_ocount.get() + 1);
    nd6_ns_output(ifp, None, &ia6.ia_addr.addr(), None, true, net);
}

/// `nd6_dad_ns_input(ifa)`: a DAD solicitation from someone else for our tentative `ifa`.
fn nd6_dad_ns_input(ifa: &IfaddrRef, net: &NetHeld<'_>) {
    let Some(dp) = nd6_dad_find(ifa) else {
        log(
            LOG_ERR,
            format_args!("nd6_dad_ns_input: DAD structure not found\n"),
        );
        return;
    };

    // if I'm yet to start DAD, someone else started using this address first. I have a
    // duplicate and you win.
    // XXX more checks for loopback situation - see nd6_dad_timer too
    if dp.dad_ns_ocount.get() == 0 {
        // dp will be freed in nd6_dad_duplicated()
        nd6_dad_duplicated(dp, ifa, net);
    } else {
        // not sure if I got a duplicate. increment ns count and see what happens.
        dp.dad_ns_icount.set(dp.dad_ns_icount.get() + 1);
    }
}

/// `nd6_isneighbor(ifp, addr)`: check whether `addr` is a neighbor address connected to
/// `ifp`.
fn nd6_isneighbor(ifp: &Ifnet, addr: &In6Addr, net: &NetHeld<'_>) -> bool {
    let tableid = ifp.if_rdomain.get();
    let sin6 = sin6_bytes(addr);

    let rt = SockaddrView::new(&sin6).and_then(|v| rtalloc(v, 0, tableid, net));

    rt.as_deref().is_some_and(|r| {
        rtisvalid(Some(r))
            && r.rt_flags.isset(RTF_CLONING | RTF_CLONED)
            && if_isconnected(ifp, r.rt_ifidx.get())
    })
    // rtfree(rt): the handle's drop
}

/// Forgets the DAD records for the host tests (`setup_real_memory`'s rule, testing.md): the
/// list, the reaper's and the free list emptied without touching their records (an earlier
/// test's memory; their own timeouts are `TIMEOUT_PROC` ones, which no host test runs), the
/// reaper's timeout taken off the wheel, nothing pending counted.
#[cfg(any(test, feature = "doubles"))]
pub fn dad_test_reset() {
    while DADQ.pop_head().is_some() {}
    *DAD_REAP.lock() = List::new();
    *DAD_FREE.lock() = List::new();
    // Off the wheel as a fresh one, without counting a deletion (the timeout tests read the
    // wheel's counters beside these).
    DAD_REAPER.set_fresh(
        Some(nd6_dad_reaper),
        ptr::null_mut(),
        KCLOCK_NONE,
        TIMEOUT_PROC,
    );
    IP6_DAD_PENDING.store(0, Ordering::Relaxed);
}

/// The DAD state of `ifa` for the tests: the solicitations sent and the solicitations and
/// advertisements received.
#[cfg(test)]
pub(crate) fn dad_counts(ifa: &Ifaddr) -> Option<(i32, i32, i32)> {
    nd6_dad_find(ifa).map(|dp| {
        (
            dp.dad_ns_ocount.get(),
            dp.dad_ns_icount.get(),
            dp.dad_na_icount.get(),
        )
    })
}

/// The DAD state of `ifa` for the tests: the solicitations to send (`dad_count`), the tries
/// and the solicitations sent.
#[cfg(test)]
pub(crate) fn dad_tries(ifa: &Ifaddr) -> Option<(i32, i32, i32)> {
    nd6_dad_find(ifa).map(|dp| {
        (
            dp.dad_count.get(),
            dp.dad_ns_tcount.get(),
            dp.dad_ns_ocount.get(),
        )
    })
}

/// The lengths of the reaper's and the free list, for the tests.
#[cfg(test)]
pub(crate) fn dad_free_counts() -> (usize, usize) {
    (
        DAD_REAP.lock().iter().count(),
        DAD_FREE.lock().iter().count(),
    )
}

/// Runs the DAD timeout of `ifa`'s record, and the reaper, for the tests.
#[cfg(test)]
pub(crate) fn dad_fire(ifa: &Ifaddr) {
    if let Some(dp) = nd6_dad_find(ifa) {
        let _ = dp.dad_timer_ch.del();
        nd6_dad_timer(ptr::without_provenance_mut(ptr::from_ref(dp).addr()));
    }
}

/// The address of `ifa`'s DAD record (its timeout's argument), for the tests.
#[cfg(test)]
pub(crate) fn dad_record_addr(ifa: &Ifaddr) -> Option<usize> {
    nd6_dad_find(ifa).map(|dp| ptr::from_ref(dp).addr())
}

/// Runs a DAD timeout whose argument is `addr`, for the tests (one that fires late).
#[cfg(test)]
pub(crate) fn dad_timer_at(addr: usize) {
    nd6_dad_timer(ptr::without_provenance_mut(addr));
}

/// Runs the reaper, for the tests.
#[cfg(test)]
pub(crate) fn dad_reap() {
    let _ = DAD_REAPER.del();
    nd6_dad_reaper(ptr::null_mut());
}
/* </CODE> */
