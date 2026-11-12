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
 *	@(#)igmp.h	8.1 (Berkeley) 6/10/93
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
 *	@(#)igmp.c	8.2 (Berkeley) 5/3/95
 */
/* </LICENSES> */

/* <CODE> */
//! The Internet Group Management Protocol (IGMP), `netinet/igmp.c`; `<netinet/igmp.h>`, the
//! packet format and constants, is `hal::sys::netinet_igmp`. Written by Steve Deering,
//! Stanford, May 1988. Modified by Rosen Sharma, Stanford, Aug 1994. Modified by Bill
//! Fenner, Xerox PARC, Feb 1995. MULTICAST Revision: 1.3.
//!
//! Upstream: sys/netinet/igmp.h @ 3ce1f3f79392
//! Upstream: sys/netinet/igmp.c @ 3ce1f3f79392
//! LZ: sys/netinet/igmp.rs@3c62ede68802
//!
//! A host joining a group (`in_addmulti`) reports its membership at once and again after a
//! random delay; a router's query restarts the delays of the groups it asks about, and
//! another host's report for one of our groups stops our timer. Local groups (224.0.0.X,
//! the all-hosts group `in_ifinit` joins among them) are never reported. The per-interface
//! router version (`struct router_info`) decides between v1 and v2 reports and leave
//! messages; a v1 query makes it v1 for `IGMP_AGE_THRESHOLD` slow timeouts.
//!
//! Locks used to protect global data and struct members: \[I\] immutable after creation,
//! \[a\] atomic, \[G\] the global igmp mutex `igmp_mtx`.
//!
//! ## Deviations
//! - `MROUTING` is not configured (the own-report check of v2 reports, `imo_loop` from
//!   `ip_mrouter_active`).
//! - `router_alert`, the Router Alert option of the reports `igmp_init` builds once in an
//!   mbuf, is the constant image of that `struct ipoption` ([`ROUTER_ALERT`]), which
//!   `ip_output` takes as bytes: one mbuf fewer in use from boot on (N6.4a's R16).
//! - `igmp_input` gives the valid packet back for the raw IGMP sockets instead of calling
//!   `rip_input` itself: `bsd`'s `inetsw[]` entry, which calls the raw input, wraps it until
//!   the protocol table moves (N6.4c2's `Protocol::input`; N6.4a's R8).
//! - `igmp_sendpkt` zeroes the IP header fields the C leaves to `ip_output` (version, header
//!   length, identification, TTL, checksum), which `ip_output` then fills (LZ's).
//!
//! ## Redesign
//! - `rti_head` (a `LIST` of `malloc(M_MRTABLE)`'d records) -> the chain of `MBox`es that
//!   `igmp_mtx` owns ([`IGMP_MTX`], the route labels' shape); `LIST_EMPTY` without the lock
//!   (`igmp_slowtimo`'s shortcut) is [`RTI_ANY`], kept beside it under the mutex.
//! - `igmpcounters` (`counters_alloc`) -> [`IGMPCOUNTERS`], a `hal::percpu::Counters`.
//! - `igmp_fasttimo`'s `struct igmp_pktlist` of `malloc(M_MRTABLE, M_NOWAIT)`'d records -> a
//!   chain of `MBox`es of the same type and flags (a report that cannot be allocated is
//!   skipped, as the C's), built at the head and sent from the oldest.
//! - The IGMP and IP headers are read and written through `hal::pod` at their offsets;
//!   `igmp_input`'s `KERNEL_LOCK()` is the kernel lock taken C-style around
//!   `igmp_input_if`, as the C.
//!
//! **Lock and spl transitions** against the C: `rti_*` take `igmp_mtx` once each, as the C;
//! `igmp_fasttimo` the net lock shared once and each interface's `if_maddrlock` once;
//! `igmp_input` the kernel lock once and `if_maddrlock` once per query or report; no spl
//! change: the C has none.

use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::malloc::{M_NOWAIT, M_WAITOK, MBox};
use hal::mutex::Mutexed;
use hal::netlock::{NETLOCK, NetHeld};
use hal::percpu::Counters;
use hal::pod;
use hal::rwlock::{RW_READ, RW_WRITE};
use hal::sys::endian::{htons, ntohs};
use hal::sys::errno::Errno;
use hal::sys::net_if::IFF_LOOPBACK;
use hal::sys::netinet_igmp::{
    IGMP_AGE_THRESHOLD, IGMP_AWAKENING_MEMBER, IGMP_DELAYING_MEMBER, IGMP_HOST_LEAVE_MESSAGE,
    IGMP_HOST_MEMBERSHIP_QUERY, IGMP_IDLE_MEMBER, IGMP_LAZY_MEMBER, IGMP_MAX_HOST_REPORT_DELAY,
    IGMP_MINLEN, IGMP_SLEEPING_MEMBER, IGMP_TIMER_SCALE, IGMP_v1_HOST_MEMBERSHIP_REPORT,
    IGMP_v1_ROUTER, IGMP_v2_HOST_MEMBERSHIP_REPORT, IGMP_v2_ROUTER, Igmp,
};
use hal::sys::netinet_igmp_var::IGMPCTL_STATS;
use hal::sys::netinet_in::{
    IN_CLASSA_NET, INADDR_ALLHOSTS_GROUP, INADDR_ALLROUTERS_GROUP, INADDR_ANY, IPPROTO_IGMP,
    InAddr, in_local_group, in_multicast,
};
use hal::sys::netinet_ip::{IPOPT_RA, Ip};

use crate::assert::kassert;
use crate::net::if_::{IFNETLIST, IfRef};
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{M_DONTWAIT, M_EXT, MAX_LINKHDR, MT_HEADER, MbufBox, m_gethdr, m_pullup};
use crate::net::proto::PR_FASTHZ;
use crate::netinet::igmp_var::{
    IGPS_NCOUNTERS, IgmpPktinfo, IgmpstatCounters, igmp_random_delay, igmpstat_inc,
};
use crate::netinet::in_::{in_ifp2ia, in_lookupmulti};
use crate::netinet::in_cksum::in_cksum;
use crate::netinet::in_var::InMulti;
use crate::netinet::ip_output::{ip_load, ip_output, ip_store};
use crate::netinet::ip_var::IpMoptions;
use crate::sysctl::{SysctlReq, rdstruct};

/// `ip_output`'s flags for a report: none (`IP_MULTICASTOPTS`).
const IP_MULTICASTOPTS: i32 = 0;

/// `M_MRTABLE`: the `malloc(9)` type of the router records and the pending reports
/// (`<sys/malloc.h>`'s `mrt`).
pub const M_MRTABLE: i32 = 56;

/// `sizeof(struct ip)`.
const IP_HDR_LEN: usize = size_of::<Ip>();

/// `router_alert`: the Router Alert option (RAO) to use in report messages as required by
/// RFC2236, a `struct ipoption`: no first hop (`ipopt_dst` `INADDR_ANY`), then
///
/// ```text
/// | 10010100 | 00000100 |  2 octet value  |
/// ```
///
/// where a value of "0" indicates that routers shall examine the packet; its length is
/// `sizeof(ipopt_dst) + ipopt_list[1]`.
pub const ROUTER_ALERT: [u8; 8] = [0, 0, 0, 0, IPOPT_RA, 0x04, 0x00, 0x00];

/// `struct router_info`: per-interface router version information, a link of the chain
/// `igmp_mtx` owns.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct router_info's rti_* names so the C can be grepped"
)]
pub struct RouterInfo {
    /// \[G\] `rti_list`: the next record.
    rti_next: Option<MBox<RouterInfo>>,
    /// \[I\] `rti_ifidx`.
    rti_ifidx: u32,
    /// \[G\] `rti_type`: type of router on interface.
    rti_type: i32,
    /// \[G\] `rti_age`: time since last v1 query.
    rti_age: i32,
}

/// What `igmp_mtx` guards: \[G\] `rti_head`.
pub struct IgmpState {
    /// `rti_head`: the router records, newest first.
    rti: Option<MBox<RouterInfo>>,
}

/// A report `igmp_fasttimo` queued (`struct igmp_pktinfo` on its `ipi_list`).
struct PktNode {
    /// The report.
    pkt: IgmpPktinfo,
    /// `ipi_list`.
    next: Option<MBox<PktNode>>,
}

/// \[a\] `igmp_timers_are_running`: shortcut for fast timer.
pub static IGMP_TIMERS_ARE_RUNNING: AtomicI32 = AtomicI32::new(0);

/// `igmp_mtx` and what it guards.
pub static IGMP_MTX: Mutexed<IgmpState> = Mutexed::new(IPL_SOFTNET, IgmpState { rti: None });

/// `!LIST_EMPTY(&rti_head)`, written under `igmp_mtx`, read without it by `igmp_slowtimo`'s
/// shortcut (the C reads the head unlocked).
static RTI_ANY: AtomicBool = AtomicBool::new(false);

/// `igmpcounters`: the IGMP statistics, per CPU.
pub static IGMPCOUNTERS: Counters<IGPS_NCOUNTERS> = Counters::new();

/// `igmp_init()`: no timer running, no router known (`LIST_INIT`, which forgets whatever the
/// head held: nothing at boot). The counters and the Router Alert option are static (the
/// module's redesign and deviations).
pub fn igmp_init() {
    IGMP_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);
    let mut g = IGMP_MTX.lock();
    if let Some(old) = g.rti.take() {
        let _ = MBox::leak(old);
    }
    RTI_ANY.store(false, Ordering::Relaxed);
}

/// The record of `ifidx` in the chain `head`. Called holding `igmp_mtx` (`head` is its data).
fn rti_find(head: Option<&RouterInfo>, ifidx: u32) -> Option<&RouterInfo> {
    let mut p = head;
    while let Some(rti) = p {
        if rti.rti_ifidx == ifidx {
            return Some(rti);
        }
        p = rti.rti_next.as_deref();
    }
    None
}

/// The record of `ifidx` in the chain `head`, to change it.
fn rti_find_mut(head: &mut Option<MBox<RouterInfo>>, ifidx: u32) -> Option<&mut RouterInfo> {
    let mut p = head.as_deref_mut();
    while let Some(rti) = p {
        if rti.rti_ifidx == ifidx {
            return Some(rti);
        }
        p = rti.rti_next.as_deref_mut();
    }
    None
}

/// The report type for a router of `type_`.
#[inline]
fn report_of(type_: i32) -> i32 {
    if type_ == IGMP_v1_ROUTER {
        i32::from(IGMP_v1_HOST_MEMBERSHIP_REPORT)
    } else {
        i32::from(IGMP_v2_HOST_MEMBERSHIP_REPORT)
    }
}

/// `rti_fill(ifidx)`: the report type for interface `ifidx`, making its router information (a
/// v2 router) if it has none.
pub fn rti_fill(ifidx: u32) -> i32 {
    {
        let g = IGMP_MTX.lock();
        if let Some(rti) = rti_find(g.rti.as_deref(), ifidx) {
            return report_of(rti.rti_type);
        }
    }

    let new_rti = MBox::new(
        M_MRTABLE,
        M_WAITOK,
        RouterInfo {
            rti_next: None,
            rti_ifidx: ifidx,
            rti_type: IGMP_v2_ROUTER,
            rti_age: 0,
        },
    );

    let mut g = IGMP_MTX.lock();
    // check again after unlock and lock
    if let Some(rti) = rti_find(g.rti.as_deref(), ifidx) {
        let type_ = rti.rti_type;
        drop(g);
        // free(new_rti): the box's drop, after the mutex.
        drop(new_rti);
        return report_of(type_);
    }
    if let Some(mut rti) = new_rti {
        rti.rti_next = g.rti.take();
        g.rti = Some(rti);
        RTI_ANY.store(true, Ordering::Relaxed);
    }
    drop(g);

    i32::from(IGMP_v2_HOST_MEMBERSHIP_REPORT)
}

/// `rti_type(ifidx)`: router version of the interface, `IGMP_v2_ROUTER` if unknown.
pub fn rti_type(ifidx: u32) -> i32 {
    let g = IGMP_MTX.lock();
    rti_find(g.rti.as_deref(), ifidx).map_or(IGMP_v2_ROUTER, |rti| rti.rti_type)
}

/// `rti_reset(ifp)`: a v1 query arrived on `ifp`: its router is v1, aged 0; `ENOBUFS` when
/// its record cannot be made.
pub fn rti_reset(ifp: &Ifnet) -> Result<(), Errno> {
    let ifidx = ifp.if_index.get();
    let mut g = IGMP_MTX.lock();
    if rti_find(g.rti.as_deref(), ifidx).is_none() {
        let Some(mut rti) = MBox::new(
            M_MRTABLE,
            M_NOWAIT,
            RouterInfo {
                rti_next: None,
                rti_ifidx: ifidx,
                rti_type: IGMP_v1_ROUTER,
                rti_age: 0,
            },
        ) else {
            return Err(Errno::ENOBUFS);
        };
        rti.rti_next = g.rti.take();
        g.rti = Some(rti);
        RTI_ANY.store(true, Ordering::Relaxed);
    }
    // found:
    if let Some(rti) = rti_find_mut(&mut g.rti, ifidx) {
        rti.rti_type = IGMP_v1_ROUTER;
        rti.rti_age = 0;
    }

    Ok(())
}

/// `rti_delete(ifp)`: forgets the router information of an interface going away.
pub fn rti_delete(ifp: &Ifnet) {
    let ifidx = ifp.if_index.get();
    let rti = {
        let mut g = IGMP_MTX.lock();
        let mut p = &mut g.rti;
        loop {
            if p.as_ref().is_none_or(|r| r.rti_ifidx == ifidx) {
                break;
            }
            match p {
                Some(r) => p = &mut r.rti_next,
                None => break,
            }
        }
        let rti = p.take().map(|mut r| {
            *p = r.rti_next.take();
            r
        });
        RTI_ANY.store(g.rti.is_some(), Ordering::Relaxed);
        rti
    };
    // free(rti): after the mutex, as the C.
    drop(rti);
}

/// `igmp_input(mp, offp, proto, af, ns)`: an IGMP message, its IP header `off` bytes long,
/// checked and acted on under the kernel lock (`igmp_input_if`); the valid packet is given
/// back for the raw IGMP sockets (the module's deviations), `None` when it was dropped.
pub fn igmp_input(m: MbufBox, off: i32) -> Option<MbufBox> {
    igmpstat_inc(IgmpstatCounters::IgpsRcvTotal);

    let ifp = IfRef::get(m.m_pkthdr().ph_ifidx.get())?;

    hal::klock::kernel_lock();
    let m = igmp_input_if(ifp.ifp(), m, off);
    hal::klock::kernel_unlock();
    // if_put(ifp): the reference's drop.
    m
}

/// The states of the memberships of `ifp` changed by a query to `dst` whose maximum response
/// time is `timer` ticks (0: a v1 query, the standard delay), except those that are already
/// running (or, for a v2 query, need no reset) and those that belong to a "local" group
/// (224.0.0.X); whether a timer was started. Takes `if_maddrlock` for writing.
fn igmp_query(ifp: &Ifnet, dst: u32, timer: u32) -> bool {
    let mut running = false;
    let lock = &ifp.rt().if_maddrlock;
    let _ = lock.enter(RW_WRITE);
    for inm in ifp.if_maddrlist.iter().filter_map(InMulti::of) {
        let group = inm.inm_addr().s_addr;
        if timer == 0 {
            if inm.inm_timer.get() == 0 && !in_local_group(group) {
                inm.inm_state.set(IGMP_DELAYING_MEMBER);
                inm.inm_timer.set(igmp_random_delay(
                    IGMP_MAX_HOST_REPORT_DELAY * PR_FASTHZ as u32,
                ));
                running = true;
            }
        } else if !in_local_group(group) && (dst == INADDR_ALLHOSTS_GROUP || dst == group) {
            match inm.inm_state.get() {
                IGMP_DELAYING_MEMBER if inm.inm_timer.get() <= timer => {}
                IGMP_DELAYING_MEMBER
                | IGMP_IDLE_MEMBER
                | IGMP_LAZY_MEMBER
                | IGMP_AWAKENING_MEMBER => {
                    inm.inm_state.set(IGMP_DELAYING_MEMBER);
                    inm.inm_timer.set(igmp_random_delay(timer));
                    running = true;
                }
                IGMP_SLEEPING_MEMBER => inm.inm_state.set(IGMP_AWAKENING_MEMBER),
                _ => {}
            }
        }
    }
    lock.exit();
    running
}

/// A report of `group` heard on `ifp`: our timer for it stopped, our state the C's for a v1
/// (`v1`) or a v2 report. Takes `if_maddrlock` for writing.
fn igmp_heard(ifp: &Ifnet, group: InAddr, v1: bool) {
    let lock = &ifp.rt().if_maddrlock;
    let _ = lock.enter(RW_WRITE);
    if let Some(inm) = in_lookupmulti(&group, ifp) {
        inm.inm_timer.set(0);
        igmpstat_inc(IgmpstatCounters::IgpsRcvOurreports);

        let state = inm.inm_state.get();
        if v1 {
            match state {
                IGMP_IDLE_MEMBER
                | IGMP_LAZY_MEMBER
                | IGMP_AWAKENING_MEMBER
                | IGMP_SLEEPING_MEMBER => inm.inm_state.set(IGMP_SLEEPING_MEMBER),
                IGMP_DELAYING_MEMBER => {
                    if rti_type(inm.inm_ifidx().get()) == IGMP_v1_ROUTER {
                        inm.inm_state.set(IGMP_LAZY_MEMBER);
                    } else {
                        inm.inm_state.set(IGMP_SLEEPING_MEMBER);
                    }
                }
                _ => {}
            }
        } else {
            match state {
                IGMP_DELAYING_MEMBER | IGMP_IDLE_MEMBER | IGMP_AWAKENING_MEMBER => {
                    inm.inm_state.set(IGMP_LAZY_MEMBER);
                }
                // IGMP_LAZY_MEMBER, IGMP_SLEEPING_MEMBER
                _ => {}
            }
        }
    }
    lock.exit();
}

/// `igmp_input_if(ifp, mp, offp, proto, af, ns)`: validates an IGMP message from `ifp` (its
/// length and checksum) and acts on a query or a report; the valid packet back for the raw
/// sockets, `None` when it was dropped (and counted).
fn igmp_input_if(ifp: &'static Ifnet, m: MbufBox, iphlen: i32) -> Option<MbufBox> {
    let mut m = m;
    let mut ip = ip_load(&m);
    let mut running = false;

    let igmplen = i32::from(ntohs(ip.ip_len)) - iphlen;

    // Validate lengths
    if igmplen < IGMP_MINLEN as i32 {
        igmpstat_inc(IgmpstatCounters::IgpsRcvTooshort);
        return None;
    }
    let minlen = iphlen + IGMP_MINLEN as i32;
    if m.flags() & M_EXT != 0 || (m.m_len().get() as i32) < minlen {
        let Some(n) = m_pullup(m, minlen) else {
            igmpstat_inc(IgmpstatCounters::IgpsRcvTooshort);
            return None;
        };
        m = n;
    }

    // Validate checksum
    m.data_add(iphlen as isize);
    m.m_len().set(m.m_len().get() - iphlen as u32);
    let Some(igmp) = pod::at::<Igmp>(m.data(), 0) else {
        igmpstat_inc(IgmpstatCounters::IgpsRcvTooshort);
        return None;
    };
    if in_cksum(&m, igmplen) != 0 {
        igmpstat_inc(IgmpstatCounters::IgpsRcvBadsum);
        return None;
    }
    m.data_add(-(iphlen as isize));
    m.m_len().set(m.m_len().get() + iphlen as u32);

    match igmp.igmp_type {
        IGMP_HOST_MEMBERSHIP_QUERY => 'query: {
            igmpstat_inc(IgmpstatCounters::IgpsRcvQueries);

            if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
                break 'query;
            }

            if igmp.igmp_code == 0 {
                if rti_reset(ifp).is_err() {
                    return None;
                }

                if ip.ip_dst.s_addr != INADDR_ALLHOSTS_GROUP {
                    igmpstat_inc(IgmpstatCounters::IgpsRcvBadqueries);
                    return None;
                }

                // Start the timers in all of our membership records for the interface on
                // which the query arrived, except those that are already running and those
                // that belong to a "local" group (224.0.0.X).
                running = igmp_query(ifp, ip.ip_dst.s_addr, 0);
            } else {
                if !in_multicast(ip.ip_dst.s_addr) {
                    igmpstat_inc(IgmpstatCounters::IgpsRcvBadqueries);
                    return None;
                }

                let mut timer = u32::from(igmp.igmp_code) * PR_FASTHZ as u32 / IGMP_TIMER_SCALE;
                if timer == 0 {
                    timer = 1;
                }

                // Start the timers in all of our membership records for the interface on
                // which the query arrived, except those that are already running and those
                // that belong to a "local" group (224.0.0.X). For timers already running,
                // check if they need to be reset.
                running = igmp_query(ifp, ip.ip_dst.s_addr, timer);
            }
        }

        t if t == IGMP_v1_HOST_MEMBERSHIP_REPORT || t == IGMP_v2_HOST_MEMBERSHIP_REPORT => 'report: {
            let v1 = t == IGMP_v1_HOST_MEMBERSHIP_REPORT;
            // MROUTING (v2): a report of our own address is not heard; not configured.
            igmpstat_inc(IgmpstatCounters::IgpsRcvReports);

            if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
                break 'report;
            }

            if !in_multicast(igmp.igmp_group.s_addr) || igmp.igmp_group.s_addr != ip.ip_dst.s_addr {
                igmpstat_inc(IgmpstatCounters::IgpsRcvBadreports);
                return None;
            }

            // KLUDGE: if the IP source address of the report has an unspecified (i.e., zero)
            // subnet number, as is allowed for a booting host, replace it with the correct
            // subnet number so that a process-level multicast routing daemon can determine
            // which subnet it arrived from. This is necessary to compensate for the lack of
            // any way for a process to determine the arrival interface of an incoming packet.
            if ip.ip_src.s_addr & IN_CLASSA_NET == 0
                && let Some(ia) = in_ifp2ia(ifp)
            {
                ip.ip_src.s_addr = ia.ia_net.get();
                ip_store(&mut m, &ip);
            }

            // If we belong to the group being reported, stop our timer for that group.
            igmp_heard(ifp, igmp.igmp_group, v1);
        }

        _ => {}
    }

    if running {
        IGMP_TIMERS_ARE_RUNNING.store(1, Ordering::Relaxed);
    }

    // Pass all valid IGMP packets up to any process(es) listening on a raw IGMP socket.
    Some(m)
}

/// `igmp_joingroup(inm, ifp, pkt)`: we joined `inm` on `ifp`: start reporting it (`pkt` the
/// first report) unless it is a local group or the interface is a loopback. Called holding
/// `if_maddrlock` for writing.
pub fn igmp_joingroup(inm: &InMulti, ifp: &Ifnet, pkt: &mut IgmpPktinfo) {
    let mut running = false;

    kassert!(ifp.rt().if_maddrlock.status() == RW_WRITE);

    inm.inm_state.set(IGMP_IDLE_MEMBER);

    if !in_local_group(inm.inm_addr().s_addr) && ifp.if_flags.get() & IFF_LOOPBACK == 0 {
        inm.inm_state.set(IGMP_DELAYING_MEMBER);
        inm.inm_timer.set(igmp_random_delay(
            IGMP_MAX_HOST_REPORT_DELAY * PR_FASTHZ as u32,
        ));
        pkt.ipi_addr = inm.inm_addr();
        pkt.ipi_rdomain = ifp.if_rdomain.get();
        pkt.ipi_ifidx = inm.inm_ifidx().get();
        pkt.ipi_type = rti_fill(inm.inm_ifidx().get());
        running = true;
    } else {
        inm.inm_timer.set(0);
    }

    if running {
        IGMP_TIMERS_ARE_RUNNING.store(1, Ordering::Relaxed);
    }
}

/// `igmp_leavegroup(inm, ifp, pkt)`: we leave `inm` on `ifp`: a leave message to the
/// all-routers group (in `pkt`) when we may have been the last reporter and the router
/// speaks v2. Called holding `if_maddrlock`.
pub fn igmp_leavegroup(inm: &InMulti, ifp: &Ifnet, pkt: &mut IgmpPktinfo) {
    kassert!(matches!(ifp.rt().if_maddrlock.status(), RW_WRITE | RW_READ));

    match inm.inm_state.get() {
        IGMP_DELAYING_MEMBER | IGMP_IDLE_MEMBER
            if !in_local_group(inm.inm_addr().s_addr)
                && ifp.if_flags.get() & IFF_LOOPBACK == 0
                && rti_type(inm.inm_ifidx().get()) != IGMP_v1_ROUTER =>
        {
            pkt.ipi_addr.s_addr = INADDR_ALLROUTERS_GROUP;
            pkt.ipi_rdomain = ifp.if_rdomain.get();
            pkt.ipi_ifidx = inm.inm_ifidx().get();
            pkt.ipi_type = i32::from(IGMP_HOST_LEAVE_MESSAGE);
        }
        // A local group, a loopback or a v1 router; IGMP_LAZY_MEMBER, IGMP_AWAKENING_MEMBER,
        // IGMP_SLEEPING_MEMBER.
        _ => {}
    }
}

/// `igmp_fasttimo()`: counts the membership timers down (`PR_FASTHZ` times a second) and
/// sends the reports of those that expire.
pub fn igmp_fasttimo() {
    let mut running = false;

    // Quick check to see if any work needs to be done, in order to minimize the overhead of
    // fasttimo processing. Variable igmp_timers_are_running is read atomically, but without
    // lock intentionally. In case it is not set due to MP races, we may miss to check the
    // timers. Then run the loop at next fast timeout.
    if IGMP_TIMERS_ARE_RUNNING.load(Ordering::Relaxed) == 0 {
        return;
    }
    IGMP_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);

    let w = NETLOCK.read();
    let Some(net) = NetHeld::of(&w) else {
        return;
    };

    let mut pktlist: Option<MBox<PktNode>> = None;
    for ifp in &IFNETLIST {
        if igmp_checktimer(ifp, &mut pktlist) {
            running = true;
        }
    }

    // The reports in the order they were queued: the chain was built at its head.
    let mut ordered: Option<MBox<PktNode>> = None;
    while let Some(mut n) = pktlist.take() {
        pktlist = n.next.take();
        n.next = ordered.take();
        ordered = Some(n);
    }
    while let Some(mut n) = ordered.take() {
        ordered = n.next.take();
        igmp_sendpkt(&n.pkt, &net);
        // free(pkt, M_MRTABLE): the box's drop.
    }

    // NET_UNLOCK_SHARED()
    drop(w);

    if running {
        IGMP_TIMERS_ARE_RUNNING.store(1, Ordering::Relaxed);
    }
}

/// `igmp_checktimer(ifp, pktlist)`: one tick of the timers of `ifp`'s groups; the reports of
/// those that expire queued on `pktlist` (newest first). `true` while some still run.
fn igmp_checktimer(ifp: &Ifnet, pktlist: &mut Option<MBox<PktNode>>) -> bool {
    let mut running = false;
    let mut type_ = 0;

    let lock = &ifp.rt().if_maddrlock;
    let _ = lock.enter(RW_WRITE);
    for inm in ifp.if_maddrlist.iter().filter_map(InMulti::of) {
        let timer = inm.inm_timer.get();
        if timer == 0 {
            // do nothing
        } else if timer == 1 {
            inm.inm_timer.set(0);
            if inm.inm_state.get() == IGMP_DELAYING_MEMBER {
                inm.inm_state.set(IGMP_IDLE_MEMBER);
                if type_ == 0 {
                    type_ = rti_type(ifp.if_index.get());
                }
                let pkt = IgmpPktinfo {
                    ipi_addr: inm.inm_addr(),
                    ipi_rdomain: ifp.if_rdomain.get(),
                    ipi_ifidx: inm.inm_ifidx().get(),
                    ipi_type: report_of(type_),
                };
                let Some(mut node) = MBox::new(M_MRTABLE, M_NOWAIT, PktNode { pkt, next: None })
                else {
                    continue;
                };
                node.next = pktlist.take();
                *pktlist = Some(node);
            }
        } else {
            inm.inm_timer.set(timer - 1);
            running = true;
        }
    }
    lock.exit();

    running
}

/// `igmp_slowtimo()`: ages the v1 routers; one silent for `IGMP_AGE_THRESHOLD` slow timeouts
/// is v2 again.
pub fn igmp_slowtimo() {
    if !RTI_ANY.load(Ordering::Relaxed) {
        return;
    }

    let mut g = IGMP_MTX.lock();
    let mut p = g.rti.as_deref_mut();
    while let Some(rti) = p {
        if rti.rti_type == IGMP_v1_ROUTER {
            rti.rti_age += 1;
            if rti.rti_age >= IGMP_AGE_THRESHOLD {
                rti.rti_type = IGMP_v2_ROUTER;
            }
        }
        p = rti.rti_next.as_deref_mut();
    }
}

/// `igmp_sendpkt(pkt)`: the report or leave message `pkt` describes, sent with the Router
/// Alert option, TTL 1, out of `pkt.ipi_ifidx`, under the net lock the caller holds (`net`).
pub fn igmp_sendpkt(pkt: &IgmpPktinfo, net: &NetHeld<'_>) {
    let Some(mut m) = m_gethdr(M_DONTWAIT, MT_HEADER) else {
        return;
    };

    // Assume max_linkhdr + sizeof(struct ip) + IGMP_MINLEN is smaller than mbuf size returned
    // by MGETHDR.
    m.data_add(MAX_LINKHDR.load(Ordering::Relaxed) as isize);
    m.m_len().set((IP_HDR_LEN + IGMP_MINLEN) as u32);
    m.m_pkthdr().len.set((IP_HDR_LEN + IGMP_MINLEN) as i32);

    let ip = Ip {
        ip_tos: 0,
        ip_len: htons((IP_HDR_LEN + IGMP_MINLEN) as u16),
        ip_off: 0,
        ip_p: IPPROTO_IGMP as u8,
        ip_src: InAddr { s_addr: INADDR_ANY },
        ip_dst: pkt.ipi_addr,
        ..Ip::default()
    };
    ip_store(&mut m, &ip);

    let mut igmp = Igmp {
        igmp_type: pkt.ipi_type as u8,
        igmp_code: 0,
        igmp_cksum: 0,
        igmp_group: pkt.ipi_addr,
    };
    m.data_add(IP_HDR_LEN as isize);
    m.m_len().set(m.m_len().get() - IP_HDR_LEN as u32);
    if let Some(d) = m.data_mut() {
        let _ = pod::put(d, 0, &igmp);
    }
    igmp.igmp_cksum = in_cksum(&m, IGMP_MINLEN as i32);
    if let Some(d) = m.data_mut() {
        let _ = pod::put(d, 0, &igmp);
    }
    m.data_add(-(IP_HDR_LEN as isize));
    m.m_len().set(m.m_len().get() + IP_HDR_LEN as u32);

    m.m_pkthdr().ph_rtableid.set(pkt.ipi_rdomain);
    let imo = IpMoptions {
        imo_ifidx: pkt.ipi_ifidx as u16,
        imo_ttl: 1,
        // Request loopback of the report if we are acting as a multicast router, so that the
        // process-level routing daemon can hear it. MROUTING: ip_mrouter_active; not
        // configured.
        imo_loop: 0,
    };

    let _ = ip_output(
        m,
        Some(&ROUTER_ALERT),
        None,
        IP_MULTICASTOPTS,
        Some(&imo),
        None,
        0,
        net,
    );

    igmpstat_inc(IgmpstatCounters::IgpsSndReports);
}

/// `igmp_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: sysctl for igmp variables
/// (`net.inet.igmp.*`).
pub fn igmp_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // All sysctl names at this level are terminal.
    let [n] = name else {
        return Err(Errno::ENOTDIR);
    };

    match *n {
        IGMPCTL_STATS => igmp_sysctl_igmpstat(req),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// `igmp_sysctl_igmpstat(oldp, oldlenp, newp)`: `net.inet.igmp.stats`, the counters as a
/// `struct igmpstat` of `u_long`s.
fn igmp_sysctl_igmpstat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let counters = IGMPCOUNTERS.read();
    let mut bytes = [0u8; IGPS_NCOUNTERS * size_of::<u64>()];
    for (w, c) in bytes.as_chunks_mut::<8>().0.iter_mut().zip(counters.iter()) {
        *w = c.to_ne_bytes();
    }

    rdstruct(req, &bytes)
}

/// Forgets the router records, which an earlier test's memory holds (testing.md's rule), and
/// stops the timers.
#[cfg(any(test, feature = "doubles"))]
pub fn igmp_test_reset() {
    igmp_init();
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for IGMP's own state: the router records (v1 after a v1 query, aged back to
    // v2, forgotten with their interface) and `net.inet.igmp.stats` over a user buffer. The
    // protocol on the wire is `bsd`'s `sys/netinet/igmp.rs`'s tests (over its test Ethernet
    // interface) and `kernel::netinet::in_`'s (the report and the leave message).

    use hal::user::UserPtr;

    use super::*;
    use crate::testutil::USER_BASE;

    /// A user buffer of the tests (free between `select`'s and `futex`'s ranges).
    const USTAT: usize = USER_BASE + 0xe000;

    #[test]
    fn routers_are_v1_after_a_v1_query_and_age_back() {
        let _g = crate::net::if_::tests::setup();
        let (ifp, _d) = crate::net::if_::test_ifnet(b"igm0");
        let idx = ifp.if_index.get();
        assert_eq!(rti_type(idx), IGMP_v2_ROUTER, "unknown is v2");
        assert_eq!(
            rti_fill(idx),
            i32::from(IGMP_v2_HOST_MEMBERSHIP_REPORT),
            "a new record is a v2 router"
        );
        assert_eq!(rti_reset(ifp), Ok(()));
        assert_eq!(rti_type(idx), IGMP_v1_ROUTER);
        assert_eq!(rti_fill(idx), i32::from(IGMP_v1_HOST_MEMBERSHIP_REPORT));
        for _ in 0..IGMP_AGE_THRESHOLD - 1 {
            igmp_slowtimo();
        }
        assert_eq!(rti_type(idx), IGMP_v1_ROUTER);
        igmp_slowtimo();
        assert_eq!(rti_type(idx), IGMP_v2_ROUTER);
        rti_delete(ifp);
        assert_eq!(rti_type(idx), IGMP_v2_ROUTER);
        assert!(!RTI_ANY.load(Ordering::Relaxed), "the list is empty again");
    }

    #[test]
    fn the_stats_are_the_counters() {
        let _g = crate::net::if_::tests::setup();
        igmpstat_inc(IgmpstatCounters::IgpsRcvBadqueries);
        let size = size_of::<hal::sys::netinet_igmp_var::Igmpstat>();
        let mut oldlen = size;
        let mut req = SysctlReq {
            oldp: USTAT,
            oldlenp: &mut oldlen,
            newp: 0,
            newlen: 0,
        };
        igmp_sysctl(&[IGMPCTL_STATS], &mut req).expect("stats");
        assert_eq!(oldlen, size);
        let i = IgmpstatCounters::IgpsRcvBadqueries as usize;
        let word = UserPtr::<u64>::new(USTAT + 8 * i)
            .copyin()
            .expect("the word");
        assert!(word >= 1);
        let mut oldlen = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut oldlen,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(igmp_sysctl(&[1, 2], &mut req), Err(Errno::ENOTDIR));
        assert_eq!(igmp_sysctl(&[9], &mut req), Err(Errno::EOPNOTSUPP));
    }
}
/* </TESTS> */
