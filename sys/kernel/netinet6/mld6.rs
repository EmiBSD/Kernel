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

/*-
 * Copyright (c) 2009 Bruce Simpson.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. The name of the author may not be used to endorse or promote
 *    products derived from this software without specific prior written
 *    permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */

/*
 * Copyright (C) 1998 WIDE Project.
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
 * Copyright (c) 1988 Stephen Deering.
 * Copyright (c) 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Stephen Deering of Stanford University.
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
 *	@(#)igmp.c	8.1 (Berkeley) 7/19/93
 */
/* </LICENSES> */

/* <CODE> */
//! Multicast Listener Discovery, `netinet6/mld6.c`: the MLDv1 host side (joining and leaving
//! groups, answering queries, the fast timeout's reports). The message formats are
//! `hal::sys::netinet6_mld6` and `hal::sys::netinet_icmp6`'s `struct mld_hdr`.
//!
//! Upstream: sys/netinet6/mld6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/mld6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/mld6.rs@3c62ede68802
//!
//! ## Deviations
//! - `MROUTING` (`ip6_mrouter_active`, loopback of reports to the routing daemon) is not
//!   configured.
//! - `mld6_input` clears and sets `mld_addr`'s embedded scope in a copy of the header, not in
//!   the packet (which it frees at the end anyway; LZ's).
//! - The reports go out through `ip6_output`, `bsd`'s until N6.5c, by the `Inet` leg
//!   `ip6_output_opts` (whether the router alert goes, the multicast interface, hop limit
//!   and loop: what `ip6_opts` and the `struct ip6_moptions` on the C's stack carry; the
//!   leg points the options at its own `'static` copy of [`MLD6_HBH`], so no caller lends
//!   it bytes whose length it would have to trust; docs/SCAFFOLDING.md).
//!
//! ## Redesign
//! - `ip6_opts` (`static struct ip6_pktopts` pointing at `hbh_buf`, which `mld6_init`
//!   fills): LZ's `StaticCell<[u8; 8]>` written at init (one `unsafe`) and an `Ip6Pktopts`
//!   rebuilt per message -> [`MLD6_HBH`], the router-alert hop-by-hop header as a `const`
//!   image (`kernel::netinet::igmp`'s `ROUTER_ALERT` shape, N6.4a's R16: no allocation per
//!   report), which the output leg points at when MLD asks for the router alert.
//! - The lists of reports (`struct mld6_pktlist`, a `STAILQ` of `malloc(M_MRTABLE,
//!   M_NOWAIT)` entries; LZ: `Vec`s) -> an `MBox` chain of [`PktNode`]s built at its head and
//!   reversed before sending (`kernel::netinet::igmp`'s shape): an entry the allocator
//!   refuses skips its group, as the C's does.
//! - `mld6_input` takes the packet by value (`MbufBox`) and reads the header by copy
//!   (`m_copydata`, the C's `ip6_exthdr_get` plus LZ's copy), no `unsafe`; the packet is
//!   freed by its drop at the end.
//! - The IPv6 and MLD headers of a report are written into the two mbufs by `hal::pod::put`
//!   (LZ: `mtod_ip6_store` and an unaligned write, one `unsafe`).
//!
//! **Lock and spl transitions** against the C: `mld6_input` takes `if_maddrlock` write once
//! per query or report; `mld6_fasttimo` the net lock shared once and `if_maddrlock` write
//! once per interface; the reports are sent after both, as the C's. None above the C.

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::malloc::{M_NOWAIT, MBox};
use hal::netlock::{NETLOCK, NetHeld};
use hal::pod;
use hal::rwlock::RW_WRITE;
use hal::sys::endian::{htons, ntohs};
use hal::sys::net_if::IFF_LOOPBACK;
use hal::sys::netinet_icmp6::{
    Icmp6Hdr, Icmp6statCounters, MLD_LISTENER_DONE, MLD_LISTENER_QUERY, MLD_LISTENER_REPORT, MldHdr,
};
use hal::sys::netinet_in::IPPROTO_ICMPV6;
use hal::sys::netinet_ip6::{
    IP6OPT_PADN, IP6OPT_ROUTER_ALERT, IP6OPT_RTALERT_LEN, IP6OPT_RTALERT_MLD, IPV6_VERSION, Ip6Hdr,
};
use hal::sys::netinet6_in6::{
    __IPV6_ADDR_SCOPE_INTFACELOCAL, __IPV6_ADDR_SCOPE_LINKLOCAL, __ipv6_addr_mc_scope, IN6ADDR_ANY,
    IN6ADDR_LINKLOCAL_ALLNODES, IN6ADDR_LINKLOCAL_ALLROUTERS, In6Addr, in6_are_addr_equal,
    in6_is_addr_linklocal, in6_is_addr_mc_linklocal, in6_is_addr_multicast,
    in6_is_addr_unspecified,
};
use hal::sys::netinet6_in6_var::{IN6_IFF_ANYCAST, IN6_IFF_DUPLICATED, IN6_IFF_TENTATIVE};
use hal::sys::netinet6_mld6::{MLD_TIMER_SCALE, MLD_V1_MAX_RI};

use crate::assert::kassert;
use crate::net::if_::{IFNETLIST, IfRef, inet};
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{
    M_DONTWAIT, M_ICMP_CSUM_OUT, M_LOOP, MT_DATA, MT_HEADER, MbufBox, m_align, m_copydata, m_get,
    m_gethdr,
};
use crate::net::proto::PR_FASTHZ;
use crate::netinet::igmp::M_MRTABLE;
use crate::netinet6::in6::{in6_lookupmulti, in6ifa_ifpforlinklocal};
use crate::netinet6::in6_var::{In6Multi, ia6_in6};
use crate::netinet6::ip6_var::{IPV6_UNSPECSRC, icmp6stat_inc, icmp6stat_inc_hist};
use crate::netinet6::mld6_var::{
    MLD_IREPORTEDLAST, MLD_OTHERLISTENER, Mld6Pktinfo, mld_random_delay,
};

/// The size of the IPv6 header.
const IP6_HDR_LEN: usize = size_of::<Ip6Hdr>();
/// The size of the MLD header.
const MLD_HDR_LEN: usize = size_of::<MldHdr>();

/// `hbh_buf`: the hop-by-hop options header of every MLD message (`ip6_opts.ip6po_hbh`):
/// `ip6h_nxt` filled in by `ip6_output`, `ip6h_len` 0 ((8 >> 3) - 1), a 2-byte PadN, the
/// Router Alert option (`IP6OPT_RTALERT_LEN - 2` bytes of value) for MLD. "XXX: grotty hard
/// coding...", the C's `mld6_init`, made once at compile time.
pub const MLD6_HBH: [u8; 8] = {
    let rtalert_code = htons(IP6OPT_RTALERT_MLD).to_ne_bytes();
    [
        0,
        0,
        IP6OPT_PADN,
        0,
        IP6OPT_ROUTER_ALERT,
        (IP6OPT_RTALERT_LEN - 2) as u8,
        rtalert_code[0],
        rtalert_code[1],
    ]
};

/// A report `mld6_input` or `mld6_fasttimo` queued (`struct mld6_pktinfo` on its
/// `mpi_list`).
struct PktNode {
    /// The report.
    pkt: Mld6Pktinfo,
    /// `mpi_list`.
    next: Option<MBox<PktNode>>,
}

/// \[a\] `mld6_timers_are_running`: shortcut for fast timer.
pub static MLD6_TIMERS_ARE_RUNNING: AtomicI32 = AtomicI32::new(0);

/// `mld6_init`: initializes the MLD timers (the router alert option is [`MLD6_HBH`]).
pub fn mld6_init() {
    MLD6_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);
}

/// `mld6_start_listening(in6m, ifp, pkt)`: starts listening to group `in6m` on `ifp`; the
/// report to send (if any) is stored in `pkt` for `mld6_sendpkt` after the locks are
/// released. Called holding `if_maddrlock` for writing.
pub fn mld6_start_listening(in6m: &In6Multi, ifp: &Ifnet, pkt: &mut Mld6Pktinfo) {
    // XXX: These are necessary for KAME's link-local hack
    let mut all_nodes = IN6ADDR_LINKLOCAL_ALLNODES;
    let mut running = false;

    kassert!(ifp.rt().if_maddrlock.status() == RW_WRITE);

    // RFC2710 page 10:
    // The node never sends a Report or Done for the link-scope all-nodes address.
    // MLD messages are never sent for multicast addresses whose scope is 0 (reserved) or 1
    // (node-local).
    all_nodes.set_s6_addr16(1, htons(in6m.in6m_ifidx().get() as u16));
    let addr = in6m.in6m_addr();
    if in6_are_addr_equal(&addr, &all_nodes)
        || __ipv6_addr_mc_scope(&addr) < __IPV6_ADDR_SCOPE_LINKLOCAL
    {
        in6m.in6m_state.set(MLD_OTHERLISTENER);
        in6m.in6m_timer.set(0);
    } else {
        in6m.in6m_state.set(MLD_IREPORTEDLAST);
        in6m.in6m_timer
            .set(mld_random_delay(MLD_V1_MAX_RI * PR_FASTHZ as u32));
        pkt.mpi_addr = addr;
        pkt.mpi_rdomain = ifp.if_rdomain.get();
        pkt.mpi_ifidx = in6m.in6m_ifidx().get();
        pkt.mpi_type = i32::from(MLD_LISTENER_REPORT);
        running = true;
    }

    if running {
        MLD6_TIMERS_ARE_RUNNING.store(1, Ordering::Relaxed);
    }
}

/// `mld6_stop_listening(in6m, ifp, pkt)`: stops listening to group `in6m` on `ifp`; the
/// done message to send (if any) is stored in `pkt`. Called holding `if_maddrlock`.
pub fn mld6_stop_listening(in6m: &In6Multi, ifp: &Ifnet, pkt: &mut Mld6Pktinfo) {
    // XXX: These are necessary for KAME's link-local hack
    let mut all_nodes = IN6ADDR_LINKLOCAL_ALLNODES;
    let mut all_routers = IN6ADDR_LINKLOCAL_ALLROUTERS;

    kassert!(ifp.rt().if_maddrlock.status() != 0);

    all_nodes.set_s6_addr16(1, htons(in6m.in6m_ifidx().get() as u16));
    // XXX: necessary when mrouting
    all_routers.set_s6_addr16(1, htons(in6m.in6m_ifidx().get() as u16));

    let addr = in6m.in6m_addr();
    if in6m.in6m_state.get() == MLD_IREPORTEDLAST
        && !in6_are_addr_equal(&addr, &all_nodes)
        && __ipv6_addr_mc_scope(&addr) > __IPV6_ADDR_SCOPE_INTFACELOCAL
    {
        pkt.mpi_addr = all_routers;
        pkt.mpi_rdomain = ifp.if_rdomain.get();
        pkt.mpi_ifidx = in6m.in6m_ifidx().get();
        pkt.mpi_type = i32::from(MLD_LISTENER_DONE);
    }
}

/// The report of `in6m` on `ifp` queued at the head of `pktlist`; nothing when the allocator
/// refuses the entry (the C's `malloc(M_NOWAIT)` failure skips the group).
fn queue_report(pktlist: &mut Option<MBox<PktNode>>, ifp: &Ifnet, in6m: &In6Multi) {
    let pkt = Mld6Pktinfo {
        mpi_addr: in6m.in6m_addr(),
        mpi_rdomain: ifp.if_rdomain.get(),
        mpi_ifidx: in6m.in6m_ifidx().get(),
        mpi_type: i32::from(MLD_LISTENER_REPORT),
    };
    if let Some(mut node) = MBox::new(M_MRTABLE, M_NOWAIT, PktNode { pkt, next: None }) {
        node.next = pktlist.take();
        *pktlist = Some(node);
    }
}

/// The reports of `pktlist` sent in the order they were queued (`STAILQ_INSERT_TAIL`'s:
/// the chain was built at its head), each entry freed after its report.
fn send_pktlist(mut pktlist: Option<MBox<PktNode>>, net: &NetHeld<'_>) {
    let mut ordered: Option<MBox<PktNode>> = None;
    while let Some(mut n) = pktlist.take() {
        pktlist = n.next.take();
        n.next = ordered.take();
        ordered = Some(n);
    }
    while let Some(mut n) = ordered.take() {
        ordered = n.next.take();
        mld6_sendpkt(&n.pkt, net);
        // free(pkt, M_MRTABLE): the box's drop.
    }
}

/// `mld6_input(m, off)`: a received MLD message at offset `off` of `m` (consumed): a query
/// starts the timers of our groups (or answers at once), a report from another listener stops
/// the timer of its group. Under the net lock (`net`, for the immediate reports).
pub fn mld6_input(m: MbufBox, off: i32, net: &NetHeld<'_>) {
    let mut running = false;
    // XXX: These are necessary for KAME's link-local hack
    let mut all_nodes = IN6ADDR_LINKLOCAL_ALLNODES;

    // ip6_exthdr_get(&m, off, sizeof(*mldh)): the header copied out of the chain.
    let mut hdr = [0u8; MLD_HDR_LEN];
    let fits = usize::try_from(off)
        .is_ok_and(|o| usize::try_from(m.m_pkthdr().len.get()).is_ok_and(|l| o + MLD_HDR_LEN <= l));
    if !fits {
        icmp6stat_inc(Icmp6statCounters::Icp6sTooshort);
        return; // m freed, as m_pulldown frees it
    }
    m_copydata(&m, off, &mut hdr);
    let Some(mldh) = pod::at::<MldHdr>(&hdr, 0) else {
        return;
    };
    let mut mld_addr = mldh.mld_addr;

    // source address validation
    let Some(src) = pod::at::<In6Addr>(m.data(), 8) else {
        return; // the caller's packet holds its IPv6 header in its first mbuf
    };
    if !in6_is_addr_linklocal(&src) {
        // spec (RFC2710) does not explicitly specify to discard the packet from a non
        // link-local source address. But we believe it's expected to do so.
        return; // m_freem(m)
    }

    let Some(ifp) = IfRef::get(m.m_pkthdr().ph_ifidx.get()) else {
        return; // m_freem(m)
    };
    let lock = &ifp.rt().if_maddrlock;

    // In the MLD6 specification, there are 3 states and a flag.
    //
    // In Non-Listener state, we simply don't have a membership record.
    // In Delaying Listener state, our timer is running (in6m->in6m_timer)
    // In Idle Listener state, our timer is not running (in6m->in6m_timer==0)
    //
    // The flag is in6m->in6m_state, it is set to MLD_OTHERLISTENER if we have heard a report
    // from another member, or MLD_IREPORTEDLAST if we sent the last report.
    match mldh.mld_type() {
        MLD_LISTENER_QUERY => 'query: {
            if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
                break 'query;
            }

            if !in6_is_addr_unspecified(&mld_addr) && !in6_is_addr_multicast(&mld_addr) {
                break 'query; // print error or log stat?
            }
            if in6_is_addr_mc_linklocal(&mld_addr) {
                mld_addr.set_s6_addr16(1, htons(ifp.if_index.get() as u16)); // XXX
            }

            // - Start the timers in all of our membership records that the query applies to
            //   for the interface on which the query arrived excl. those that belong to the
            //   "all-nodes" group (ff02::1).
            // - Restart any timer that is already running but has A value longer than the
            //   requested timeout.
            // - Use the value specified in the query message as the maximum timeout.
            //
            // XXX: System timer resolution is too low to handle Max Response Delay, so set 1
            // to the internal timer even if the calculated value equals to zero when Max
            // Response Delay is positive.
            let mut timer =
                u32::from(ntohs(mldh.mld_maxdelay())) * PR_FASTHZ as u32 / MLD_TIMER_SCALE;
            if timer == 0 && mldh.mld_maxdelay() != 0 {
                timer = 1;
            }
            all_nodes.set_s6_addr16(1, htons(ifp.if_index.get() as u16));

            let mut pktlist: Option<MBox<PktNode>> = None;
            let _ = lock.enter(RW_WRITE);
            for in6m in ifp.if_maddrlist.iter().filter_map(In6Multi::of) {
                let addr = in6m.in6m_addr();
                if in6_are_addr_equal(&addr, &all_nodes)
                    || __ipv6_addr_mc_scope(&addr) < __IPV6_ADDR_SCOPE_LINKLOCAL
                {
                    continue;
                }

                if in6_is_addr_unspecified(&mld_addr) || in6_are_addr_equal(&mld_addr, &addr) {
                    if timer == 0 {
                        // send a report immediately
                        in6m.in6m_state.set(MLD_IREPORTEDLAST);
                        in6m.in6m_timer.set(0); // reset timer
                        queue_report(&mut pktlist, &ifp, in6m);
                    } else if in6m.in6m_timer.get() == 0 /* idle */
                        || in6m.in6m_timer.get() > timer
                    {
                        in6m.in6m_timer.set(mld_random_delay(timer));
                        running = true;
                    }
                }
            }
            lock.exit();

            send_pktlist(pktlist, net);
        }
        MLD_LISTENER_REPORT => 'report: {
            // For fast leave to work, we have to know that we are the last person to send a
            // report for this group. Reports can potentially get looped back if we are a
            // multicast router, so discard reports sourced by me. Note that it is impossible
            // to check IFF_LOOPBACK flag of ifp for this purpose, since ip6_mloopback pass
            // the physical interface to if_input_local().
            if m.flags() & M_LOOP != 0 {
                // XXX: grotty flag, but efficient
                break 'report;
            }

            if !in6_is_addr_multicast(&mld_addr) {
                break 'report;
            }

            if in6_is_addr_mc_linklocal(&mld_addr) {
                mld_addr.set_s6_addr16(1, htons(ifp.if_index.get() as u16)); // XXX
            }
            // If we belong to the group being reported, stop our timer for that group.
            let _ = lock.enter(RW_WRITE);
            if let Some(in6m) = in6_lookupmulti(&mld_addr, &ifp) {
                in6m.in6m_state.set(MLD_OTHERLISTENER); // clear flag
                in6m.in6m_timer.set(0); // transit to idle state
            }
            lock.exit();
        }
        // this is impossible: icmp6_input() only passes queries and reports.
        _ => {}
    }

    if running {
        MLD6_TIMERS_ARE_RUNNING.store(1, Ordering::Relaxed);
    }

    // if_put(ifp), m_freem(m): the drops.
}

/// `mld6_fasttimo()`: the MLD fast timeout: counts the membership timers down and sends the
/// reports of those that expire.
pub fn mld6_fasttimo() {
    let mut running = false;

    // Quick check to see if any work needs to be done, in order to minimize the overhead of
    // fasttimo processing. Variable mld6_timers_are_running is read atomically, but without
    // lock intentionally. In case it is not set due to MP races, we may miss to check the
    // timers. Then run the loop at next fast timeout.
    if MLD6_TIMERS_ARE_RUNNING.load(Ordering::Relaxed) == 0 {
        return;
    }
    MLD6_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);

    let w = NETLOCK.read();
    let Some(net) = NetHeld::of(&w) else {
        return;
    };

    let mut pktlist: Option<MBox<PktNode>> = None;
    for ifp in &IFNETLIST {
        if mld6_checktimer(ifp, &mut pktlist) {
            running = true;
        }
    }

    send_pktlist(pktlist, &net);

    // NET_UNLOCK_SHARED()
    drop(w);

    if running {
        MLD6_TIMERS_ARE_RUNNING.store(1, Ordering::Relaxed);
    }
}

/// `mld6_checktimer(ifp, pktlist)`: one tick of the timers of `ifp`'s groups; the reports of
/// those that expire queued on `pktlist`. `true` while some still run.
fn mld6_checktimer(ifp: &Ifnet, pktlist: &mut Option<MBox<PktNode>>) -> bool {
    let mut running = false;

    let lock = &ifp.rt().if_maddrlock;
    let _ = lock.enter(RW_WRITE);
    for in6m in ifp.if_maddrlist.iter().filter_map(In6Multi::of) {
        let timer = in6m.in6m_timer.get();
        if timer == 0 {
            // do nothing
        } else if timer == 1 {
            in6m.in6m_timer.set(0);
            in6m.in6m_state.set(MLD_IREPORTEDLAST);
            queue_report(pktlist, ifp, in6m);
        } else {
            in6m.in6m_timer.set(timer - 1);
            running = true;
        }
    }
    lock.exit();

    running
}

/// `mld6_sendpkt(pkt)`: sends the MLD message `pkt` describes: a hop limit 1 packet from a
/// link-local address of the interface (the unspecified one while it is tentative), with the
/// Router Alert option. Under the net lock (`net`).
pub fn mld6_sendpkt(pkt: &Mld6Pktinfo, net: &NetHeld<'_>) {
    let _ = net;
    let Some(ifp) = IfRef::get(pkt.mpi_ifidx) else {
        return;
    };

    // At first, find a link local address on the outgoing interface to use as the source
    // address of the MLD packet. We do not reject tentative addresses for MLD report to deal
    // with the case where we first join a link-local address.
    let ignflags = IN6_IFF_DUPLICATED | IN6_IFF_ANYCAST;
    let Some(ia6) = in6ifa_ifpforlinklocal(ifp.ifp(), ignflags) else {
        return; // if_put(ifp)
    };
    let ia6 = (ia6.ia6_flags.get() & IN6_IFF_TENTATIVE == 0).then_some(ia6);

    // Allocate mbufs to store ip6 header and MLD header. We allocate 2 mbufs and make chain
    // in advance because it is more convenient when inserting the hop-by-hop option later.
    let Some(mut mh) = m_gethdr(M_DONTWAIT, MT_HEADER) else {
        return;
    };
    let Some(mut md) = m_get(M_DONTWAIT, MT_DATA) else {
        return; // m_free(mh): the drop
    };

    mh.m_pkthdr().ph_rtableid.set(pkt.mpi_rdomain);
    mh.m_pkthdr().len.set((IP6_HDR_LEN + MLD_HDR_LEN) as i32);
    mh.m_len().set(IP6_HDR_LEN as u32);
    m_align(&mh, IP6_HDR_LEN as i32);

    // fill in the ip6 header
    let mut ip6 = Ip6Hdr {
        ip6_flow: 0,
        // ip6_plen will be set later
        ip6_nxt: IPPROTO_ICMPV6 as u8,
        // ip6_hlim will be set by im6o.im6o_hlim
        ip6_src: ia6.map_or(IN6ADDR_ANY, ia6_in6),
        ip6_dst: pkt.mpi_addr,
        ..Ip6Hdr::default()
    };
    ip6.set_ip6_vfc(IPV6_VERSION);
    // Fresh mbufs: written in place; without the room, nothing is sent.
    if !mh.data_mut().is_some_and(|d| pod::put(d, 0, &ip6)) {
        return;
    }

    // fill in the MLD header
    md.m_len().set(MLD_HDR_LEN as u32);
    let mut mldh = MldHdr {
        mld_icmp6_hdr: Icmp6Hdr::zeroed(),
        mld_addr: pkt.mpi_addr,
    };
    mldh.set_mld_type(pkt.mpi_type as u8);
    mldh.set_mld_code(0);
    mldh.set_mld_cksum(0);
    // XXX: we assume the function will not be called for query messages
    mldh.set_mld_maxdelay(0);
    mldh.set_mld_reserved(0);
    if in6_is_addr_mc_linklocal(&mldh.mld_addr) {
        mldh.mld_addr.set_s6_addr16(1, 0); // XXX
    }
    if !md.data_mut().is_some_and(|d| pod::put(d, 0, &mldh)) {
        return;
    }
    drop(mh.m_next.set(md));
    mh.m_pkthdr()
        .csum_flags
        .set(mh.m_pkthdr().csum_flags.get() | M_ICMP_CSUM_OUT);

    // construct multicast option: the interface, hop limit 1. Request loopback of the report
    // if we are acting as a multicast router, so that the process-level routing daemon can
    // hear it. MROUTING: ip6_mrouter_active; not configured: im6o_loop 0.
    let ifidx = pkt.mpi_ifidx as u16;
    drop(ifp); // if_put(ifp)

    icmp6stat_inc_hist(Icmp6statCounters::Icp6sOuthist, pkt.mpi_type as u8);
    let flags = if ia6.is_some() { 0 } else { IPV6_UNSPECSRC };
    if let Some(i) = inet() {
        // ip6_output(mh, &ip6_opts, NULL, flags, &im6o, NULL): the router alert is MLD6_HBH,
        // which the leg points the options at itself.
        let _ = i.ip6_output_opts(mh, true, flags, ifidx, 1, false);
    }
}
/* </CODE> */
