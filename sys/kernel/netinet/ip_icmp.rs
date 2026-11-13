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
 *	@(#)ip_icmp.h	8.1 (Berkeley) 6/10/93
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
//! ICMP routines: error generation, receive packet processing, and routines to turnaround
//! packets back to the originator, and host table maintenance routines.
//!
//! Upstream: sys/netinet/ip_icmp.h @ 3ce1f3f79392
//! Upstream: sys/netinet/ip_icmp.c @ 3ce1f3f79392
//! LZ: sys/netinet/ip_icmp.rs@3c62ede68802
//!
//! [`icmp_input`] counts every message it accepts by type (`icps_inhist`), answers echo,
//! timestamp and mask requests ([`icmp_reflect`], [`icmp_send`]), hands errors to the
//! protocol of the offending packet (`pr_ctlinput`) and redirects to the routing table, and
//! passes every message on to the raw sockets (the raw entry's `pr_input`, `rip_input`).
//! [`icmp_error`] answers a bad packet with an error message. The header (`struct icmp`, the
//! types and codes) is `hal::sys::netinet_ip_icmp`'s.
//!
//! Locks: \[a\] atomic operations.
//!
//! ## Deviations
//! - `ICMPPRINTFS` (debug printfs) is not configured; `NCARP` (`carp_lsdrop`) is not
//!   configured: each is a comment at its site. `INET6` is: the length check of a v6-in-v4
//!   message. `NPF` (pf(4)) is configured: the `PF_TAG_DIVERTED` handling of diverted
//!   connections.
//! - `icmp_error` and `icmp_do_error` take the C's `int type, int code` as `u8`s (the values
//!   of `icmp_type`/`icmp_code`; a parameter problem's pointer is an offset in a header of at
//!   most 60 bytes).
//! - `icmp_do_exthdr` has no caller at the pin (MPLS is not configured); it is kept.
//!
//! ## Redesign
//! - LZ's `IcmpPkt` (a raw pointer and a length into the mbuf, read and written with
//!   unaligned `unsafe` copies at the members' offsets) -> the message read out of the first
//!   mbuf's bytes as a `hal::sys::netinet_ip_icmp::Icmp` image (`pod::at`, its 28 bytes or
//!   fewer, zero-filled past the message: every read is behind the C's length check of that
//!   member) and the members it changes written back at their offsets ([`icmp_put`]: in
//!   place, or through `m_copyback` for a read-only cluster). `bsd` keeps `IcmpPkt` for its
//!   own readers (TCP's path MTU, pf) until they move.
//! - The packets are owned (`MbufBox`): `icmp_reflect` answers the packet back or `None`
//!   when it freed it (the C's `EHOSTUNREACH`/`ELOOP`), its options through `op`;
//!   `icmp_send` takes the options' bytes.
//! - `pr_ctlinput` of the offending packet's protocol is `Protocol::ctlinput` with the
//!   message's bytes (the C's `&icp->icmp_ip`, whose `struct icmp` TCP reads back), found
//!   with `kernel::netinet::in_proto::ip_protocol`; the raw sockets' `rip_input` is the raw
//!   entry's `Protocol::input` (`raw_protocol`).
//! - `icmperrppslim_last`/`icmperrpps_count`, a `StaticCell` in LZ touched through
//!   `ppsratecheck_shared`'s raw pointers, are a `Guarded` cell keyed to `ppsratecheck_mtx`
//!   ([`ICMPERRPPS`]), read and written in `ppsratecheck_locked`'s one section: the C's
//!   unlocked static pair, one mutex pair per error (the C's own `ppsratecheck` takes the
//!   same mutex).
//! - `icmpcounters` is `kernel::netinet::icmp_var::ICMPCOUNTERS`.
//! - The route timer queues' function (`icmp_mtudisc_timeout`) takes the net lock's proof
//!   the queue's timer holds (`RtqFunc`).
//!
//! **Lock and spl transitions** against the C: `icmp_input` runs under the exclusive net
//! lock (`ipintr`, ICMP has no `PR_MPINPUT`) and takes none of its own; `icmp_error` takes
//! `ppsratecheck_mtx` once (the C's `ppsratecheck` does); `icmp_send`'s `ip_send` queues the
//! packet for the softnet task, as the C.

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::netlock::{NETLOCK, NetHeld};
use hal::pod;
use hal::rwlock::Guarded;
use hal::sys::errno::Errno;
use hal::sys::limits::INT_MAX;
use hal::sys::net_if::{IFF_BROADCAST, IFF_POINTOPOINT, IFF_UP};
use hal::sys::netinet_icmp_var::{
    ICMPCTL_BMCASTECHO, ICMPCTL_ERRPPSLIMIT, ICMPCTL_MASKREPL, ICMPCTL_REDIRACCEPT,
    ICMPCTL_REDIRTIMEOUT, ICMPCTL_STATS, ICMPCTL_TSTAMPREPL,
};
use hal::sys::netinet_in::{
    IN_CLASSA_NET, IN_CLASSA_NSHIFT, IN_LOOPBACKNET, INADDR_ANY, INADDR_BROADCAST, IPPROTO_DONE,
    IPPROTO_ICMP, IPPROTO_IPV6, IPPROTO_TCP, InAddr, SockaddrIn, in_multicast,
};
use hal::sys::netinet_ip::{
    IP_DF, IP_OFFMASK, IPOPT_EOL, IPOPT_NOP, IPOPT_OLEN, IPOPT_OPTVAL, IPOPT_RR, IPOPT_SECURITY,
    IPOPT_TS, IPVERSION, Ip, MAXTTL,
};
use hal::sys::netinet_ip_icmp::{
    ICMP_ADVLENMAX, ICMP_ADVLENMIN, ICMP_CKSUM_OFFSET, ICMP_DATACONVERR, ICMP_ECHO, ICMP_ECHOREPLY,
    ICMP_EXT_HDR_VERSION, ICMP_EXT_OFFSET, ICMP_HUN_OFFSET, ICMP_IP_OFFSET, ICMP_IPV6_IAMHERE,
    ICMP_IPV6_WHEREAREYOU, ICMP_IREQREPLY, ICMP_MASKLEN, ICMP_MASKREPLY, ICMP_MASKREQ,
    ICMP_MAXTYPE, ICMP_MINLEN, ICMP_MOBILE_REDIRECT, ICMP_MOBILE_REGREPLY, ICMP_MOBILE_REGREQUEST,
    ICMP_PARAMPROB, ICMP_PHOTURIS, ICMP_REDIRECT, ICMP_ROUTERADVERT, ICMP_ROUTERSOLICIT,
    ICMP_SOURCEQUENCH, ICMP_TIMXCEED, ICMP_TRACEROUTE, ICMP_TSLEN, ICMP_TSTAMP, ICMP_TSTAMPREPLY,
    ICMP_UNREACH, ICMP_UNREACH_FILTER_PROHIB, ICMP_UNREACH_HOST, ICMP_UNREACH_HOST_PRECEDENCE,
    ICMP_UNREACH_HOST_PROHIB, ICMP_UNREACH_HOST_UNKNOWN, ICMP_UNREACH_ISOLATED,
    ICMP_UNREACH_NEEDFRAG, ICMP_UNREACH_NET, ICMP_UNREACH_NET_PROHIB, ICMP_UNREACH_NET_UNKNOWN,
    ICMP_UNREACH_PORT, ICMP_UNREACH_PRECEDENCE_CUTOFF, ICMP_UNREACH_PROTOCOL, ICMP_UNREACH_SRCFAIL,
    ICMP_UNREACH_TOSHOST, ICMP_UNREACH_TOSNET, ICMP_V6ADVLENMIN, Icmp, IcmpExtHdr, IcmpExtObjHdr,
    icmp_advlen, icmp_infotype, icmp_v6advlen,
};
use hal::sys::socket::AF_INET;
use hal::sys::sysctl::SysctlBoundedArgs;

use crate::assert::kpanic;
use crate::net::domain::pfctlinput;
use crate::net::if_::{if_get, if_put, ifa_ifwithaddr, ifaof_ifpforaddr};
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::mbuf::{
    M_BCAST, M_DONTWAIT, M_EXT, M_ICMP_CSUM_OUT, M_MAXLOOP, M_MCAST, MCLBYTES, MHLEN, MT_HEADER,
    Mbuf, MbufBox, PF_TAG_DIVERTED, PF_TAG_GENERATED, m_align, m_copyback, m_copydata, m_gethdr,
    m_getptr, m_prepend, m_pullup, m_resethdr, mclget,
};
use crate::net::proto::{
    PRC_MSGSIZE, PRC_MTUINC, PRC_PARAMPROB, PRC_QUENCH, PRC_REDIRECT_HOST, PRC_TIMXCEED_INTRANS,
    PRC_UNREACH_HOST, PRC_UNREACH_NET, SockaddrView,
};
use hal::sys::net_route::{
    RTAX_DST, RTAX_GATEWAY, RTAX_LABEL, RTF_BLACKHOLE, RTF_BROADCAST, RTF_DYNAMIC, RTF_GATEWAY,
    RTF_HOST, RTF_LLINFO, RTF_LOCAL, RTF_REJECT, RTM_ADD, RTV_MTU, SockaddrRtlabel,
};

use crate::clock::Timeval;
use crate::net::route::{
    RT_RESOLVE, RtAddrinfo, Rtentry, RttimerQueue, rt_timer_add, rt_timer_queue_change,
    rt_timer_queue_init, rtalloc, rtdeletemsg, rtisvalid, rtlabel_id2sa, rtredirect, rtrequest,
};
use crate::net::rtable::rtable_getsource;
use crate::net::rtsock::rtm_send;
use crate::netinet::icmp_var::{
    ICMPCOUNTERS, ICPS_NCOUNTERS, IcmpstatCounters, icmpstat_inc, icmpstat_inc_hist,
};
use crate::netinet::in_::in_canforward;
use crate::netinet::in_proto::{ip_protocol, raw_protocol};
use crate::netinet::in_var::InIfaddr;
use crate::netinet::in4_cksum::in4_cksum;
use crate::netinet::ip_id::ip_randomid;
use crate::netinet::ip_input::{
    IP_MTUDISC_TIMEOUT, ip_forwarding, ip_send, ip_send_raw, ip_srcroute, ip_stripoptions, pkt_put,
    sin_bytes,
};
use crate::netinet::ip_output::{ip_insertoptions, ip_load, ip_store};
use crate::netinet::ip_var::{IpstatCounters, ipstat_inc};
use crate::proc::ProcGlue;
use crate::sysctl::{SysctlReq, bounded_arr, int_bounded, rdstruct, sysctl_vslock};
use crate::time::{PPSRATECHECK_MTX, ppsratecheck_held, ppsratecheck_locked};

/// `sizeof(struct ip)`.
const IP_HDR_LEN: usize = size_of::<Ip>();

/// `mtu_table[]`: table of common MTUs.
const MTU_TABLE: [u16; 14] = [
    65535, 65280, 32000, 17914, 9180, 8166, 4352, 2002, 1492, 1006, 508, 296, 68, 0,
];

/// `icmperrppslim_last` and `icmperrpps_count`.
struct IcmpErrPps {
    /// `icmperrppslim_last`.
    last: Timeval,
    /// `icmperrpps_count`.
    count: i32,
}

/// \[a\] `icmpmaskrepl`.
pub static ICMPMASKREPL: AtomicI32 = AtomicI32::new(0);
/// \[a\] `icmpbmcastecho`.
pub static ICMPBMCASTECHO: AtomicI32 = AtomicI32::new(0);
/// \[a\] `icmptstamprepl`.
pub static ICMPTSTAMPREPL: AtomicI32 = AtomicI32::new(1);
/// \[a\] `icmperrppslim`.
pub static ICMPERRPPSLIM: AtomicI32 = AtomicI32::new(100);
/// \[a\] `icmp_rediraccept`.
pub static ICMP_REDIRACCEPT: AtomicI32 = AtomicI32::new(0);
/// `icmp_redirtimeout`: changed under the net lock.
pub static ICMP_REDIRTIMEOUT: AtomicI32 = AtomicI32::new(10 * 60);

/// `icmperrpps_count`, `icmperrppslim_last`, under `ppsratecheck_mtx`.
static ICMPERRPPS: Guarded<IcmpErrPps> = Guarded::new_static_mutexed(
    &PPSRATECHECK_MTX,
    IcmpErrPps {
        last: Timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        count: 0,
    },
);

/// `ip_mtudisc_timeout_q`.
pub static IP_MTUDISC_TIMEOUT_Q: RttimerQueue = RttimerQueue::new();
/// `icmp_redirect_timeout_q`.
pub static ICMP_REDIRECT_TIMEOUT_Q: RttimerQueue = RttimerQueue::new();

/// `icmpctl_vars[]`.
static ICMPCTL_VARS: [SysctlBoundedArgs; 5] = [
    SysctlBoundedArgs::new(ICMPCTL_MASKREPL, &ICMPMASKREPL, 0, 1),
    SysctlBoundedArgs::new(ICMPCTL_BMCASTECHO, &ICMPBMCASTECHO, 0, 1),
    SysctlBoundedArgs::new(ICMPCTL_ERRPPSLIMIT, &ICMPERRPPSLIM, -1, INT_MAX),
    SysctlBoundedArgs::new(ICMPCTL_REDIRACCEPT, &ICMP_REDIRACCEPT, 0, 1),
    SysctlBoundedArgs::new(ICMPCTL_TSTAMPREPL, &ICMPTSTAMPREPL, 0, 1),
];

/// `icmp_init()`: the route timer queues of path MTU discovery and of redirects (the
/// counters are a static).
pub fn icmp_init<G: ProcGlue>() {
    rt_timer_queue_init(
        &IP_MTUDISC_TIMEOUT_Q,
        IP_MTUDISC_TIMEOUT.load(Ordering::Relaxed),
        Some(icmp_mtudisc_timeout::<G>),
    );
    rt_timer_queue_init(
        &ICMP_REDIRECT_TIMEOUT_Q,
        ICMP_REDIRTIMEOUT.load(Ordering::Relaxed),
        None,
    );
    // icmpcounters = counters_alloc(icps_ncounters): ICMPCOUNTERS.
}

/// The ICMP message at `off` of `m`'s first mbuf, as much of `struct icmp` as the mbuf holds
/// there (the rest zero: every member read past it is behind a length check of the C's).
#[inline]
fn icmp_at(m: &Mbuf, off: usize) -> Icmp {
    let mut b = [0u8; size_of::<Icmp>()];
    if let Some(d) = m.data().get(off..) {
        let n = d.len().min(b.len());
        b[..n].copy_from_slice(&d[..n]);
    }
    pod::at::<Icmp>(&b, 0).unwrap_or_default()
}

/// The first `n` bytes of `icp`'s image.
#[inline]
fn icmp_bytes(icp: &Icmp) -> [u8; size_of::<Icmp>()] {
    let mut b = [0u8; size_of::<Icmp>()];
    let _ = pod::put(&mut b, 0, icp);
    b
}

/// A view of the `sockaddr_in` bytes `b`.
#[inline]
fn view(b: &[u8]) -> SockaddrView<'_> {
    match SockaddrView::new(b) {
        Some(v) => v,
        None => kpanic(format_args!("icmp: a socket address too short")),
    }
}

/// `icmp_do_error(n, type, code, dest, destmtu)`: the ICMP error message of `type_` and
/// `code` about packet `n` (freed), with its IP header in front; `None` when no error may or
/// can be sent.
pub fn icmp_do_error(n: MbufBox, type_: u8, code: u8, dest: u32, destmtu: i32) -> Option<MbufBox> {
    let oip = ip_load(&n);
    let oiplen = usize::from(oip.ip_hl()) << 2;
    let mut code = code;

    // ICMPPRINTFS: not configured.
    if type_ != ICMP_REDIRECT {
        icmpstat_inc(IcmpstatCounters::IcpsError);
    }
    // Don't send error if not the first fragment of message. Don't error if the old packet
    // protocol was ICMP error message, only known informational types.
    if oip.ip_off & IP_OFFMASK.to_be() != 0 {
        return None;
    }
    if i32::from(oip.ip_p) == IPPROTO_ICMP
        && type_ != ICMP_REDIRECT
        && n.m_len().get() as usize >= oiplen + ICMP_MINLEN
        && !icmp_infotype(icmp_at(&n, oiplen).icmp_type)
    {
        icmpstat_inc(IcmpstatCounters::IcpsOldicmp);
        return None;
    }
    // Don't send error in response to a multicast or broadcast packet
    if n.flags() & (M_BCAST | M_MCAST) != 0 {
        return None;
    }

    // First, do a rate limitation check.
    if icmp_ratelimit(&oip.ip_src, type_, code) {
        icmpstat_inc(IcmpstatCounters::IcpsToofreq);
        return None;
    }

    // Now, formulate icmp message
    let mut icmplen = oiplen + 8.min(usize::from(u16::from_be(oip.ip_len)));
    // Defend against mbuf chains shorter than oip->ip_len:
    let mut mblen = 0;
    let mut mm = Some(&*n);
    while let Some(x) = mm {
        if mblen >= icmplen {
            break;
        }
        mblen += x.m_len().get() as usize;
        mm = x.m_next().get();
    }
    icmplen = mblen.min(icmplen);

    // As we are not required to return everything we have, we return whatever we can return
    // at ease. Note that ICMP datagrams longer than 576 octets are out of spec according to
    // RFC1812;
    const _: () = assert!(ICMP_MINLEN + size_of::<Ip>() <= MCLBYTES);

    if IP_HDR_LEN + icmplen + ICMP_MINLEN > MCLBYTES {
        icmplen = MCLBYTES - ICMP_MINLEN - IP_HDR_LEN;
    }

    let mut m = m_gethdr(M_DONTWAIT, MT_HEADER);
    if let Some(mm) = m.as_mut()
        && ((IP_HDR_LEN + icmplen + ICMP_MINLEN + size_of::<u64>() - 1) & !(size_of::<u64>() - 1))
            > MHLEN
    {
        mclget(mm, M_DONTWAIT);
        if mm.flags() & M_EXT == 0 {
            m = None;
        }
    }
    let mut m = m?;
    // keep in same rtable and preserve other pkthdr bits
    let ph = m.m_pkthdr();
    ph.ph_rtableid.set(n.m_pkthdr().ph_rtableid.get());
    ph.ph_ifidx.set(n.m_pkthdr().ph_ifidx.get());
    // move PF_GENERATED to new packet, if existent XXX preserve more?
    if n.m_pkthdr().pf.flags.get() & PF_TAG_GENERATED != 0 {
        ph.pf.flags.set(ph.pf.flags.get() | PF_TAG_GENERATED);
    }
    let len = icmplen + ICMP_MINLEN;
    ph.len.set(len as i32);
    m.m_len().set(len as u32);
    m_align(&m, len as i32);
    if type_ > ICMP_MAXTYPE {
        kpanic(format_args!("icmp_error"));
    }
    icmpstat_inc_hist(IcmpstatCounters::IcpsOuthist, type_);
    let mut icp = Icmp {
        icmp_type: type_,
        ..Icmp::default()
    };
    if type_ == ICMP_REDIRECT {
        icp.set_icmp_gwaddr(InAddr { s_addr: dest });
    } else {
        icp.set_icmp_void(0);
        // The following assignments assume an overlay with the zeroed icmp_void field.
        if type_ == ICMP_PARAMPROB {
            icp.set_icmp_pptr(code);
            code = 0;
        } else if type_ == ICMP_UNREACH && code == ICMP_UNREACH_NEEDFRAG && destmtu != 0 {
            icp.set_icmp_nextmtu((destmtu as u16).to_be());
        }
    }

    icp.icmp_code = code;
    let b = icmp_bytes(&icp);
    // A fresh mbuf: written in place (without it, no error is sent, as without the mbuf).
    if !pkt_put(&mut m, 0, &b[..ICMP_IP_OFFSET]) {
        return None;
    }
    let Some(d) = m.data_mut() else {
        return None;
    };
    m_copydata(&n, 0, &mut d[ICMP_IP_OFFSET..ICMP_IP_OFFSET + icmplen]);

    // Now, copy old ip header (without options) in front of icmp message.
    let mut m = m_prepend(m, IP_HDR_LEN as i32, M_DONTWAIT)?;
    let mut nip = Ip::default();
    // ip_v set in ip_output
    nip.set_ip_hl((IP_HDR_LEN >> 2) as u8);
    nip.ip_tos = 0;
    nip.ip_len = (m.m_len().get() as u16).to_be();
    // ip_id set in ip_output
    nip.ip_off = 0;
    // ip_ttl set in icmp_reflect
    nip.ip_p = IPPROTO_ICMP as u8;
    nip.ip_src = oip.ip_src;
    nip.ip_dst = oip.ip_dst;
    if !ip_store(&mut m, &nip) {
        return None;
    }

    drop(n);
    Some(m)
}

/// `icmp_error(n, type, code, dest, destmtu)`: generates an error packet of `type_` in
/// response to bad packet `n` (whose header has `ip_off` and `ip_len` in network order).
pub fn icmp_error(n: MbufBox, type_: u8, code: u8, dest: u32, destmtu: i32, net: &NetHeld<'_>) {
    if let Some(m) = icmp_do_error(n, type_, code, dest, destmtu)
        && let Some(m) = icmp_reflect(m, None, None, net)
    {
        icmp_send(m, None);
    }
}

/// `icmp_input(mp, offp, proto, af, ns)`: processes a received ICMP message (the ICMP entry's
/// `pr_input`).
pub fn icmp_input<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    proto: i32,
    af: i32,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let Some(m) = mp.as_ref() else {
        return IPPROTO_DONE;
    };
    let Some(ifp) = if_get(m.m_pkthdr().ph_ifidx.get()) else {
        drop(mp.take());
        return IPPROTO_DONE;
    };

    let proto = icmp_input_if::<G>(ifp, mp, offp, proto, af, ns, net);
    if_put(Some(ifp));
    proto
}

/// What `icmp_input_if` does with the message once it has looked at it.
enum Then {
    /// To the raw sockets (`goto raw`).
    Raw,
    /// Freed (`goto freeit`).
    Free,
    /// Answered (`goto reflect`): the message was turned into its reply.
    Reflect,
}

/// A reply that could not be written (no buffer for a shared cluster's copy; the message is
/// writable since `ipv4_check`, so not taken): freed, counted as the C counts `icmp_input`'s
/// `m_pullup` failures (`icps_tooshort`).
fn unwritable() -> Then {
    icmpstat_inc(IcmpstatCounters::IcpsTooshort);
    Then::Free
}

/// `icmp_input_if(ifp, mp, offp, proto, af, ns)`: `icmp_input` for the interface it was
/// received on.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's arguments and the net lock's proof"
)]
#[allow(
    clippy::too_many_lines,
    reason = "the C's switch over the message types"
)]
pub fn icmp_input_if<G: ProcGlue>(
    ifp: &'static Ifnet,
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
    let hlen = usize::try_from(*offp).unwrap_or(0);
    let ip = ip_load(&m);

    // Locate icmp structure in mbuf, and check that not corrupted and of at least minimum
    // length.
    let icmplen = i32::from(u16::from_be(ip.ip_len)) - hlen as i32;
    // ICMPPRINTFS: not configured.
    if icmplen < ICMP_MINLEN as i32 {
        icmpstat_inc(IcmpstatCounters::IcpsTooshort);
        return IPPROTO_DONE;
    }
    let icmplen_u = icmplen as usize;
    let i = hlen + icmplen_u.min(ICMP_ADVLENMAX);
    let Some(mut m) = m_pullup(m, i as i32) else {
        icmpstat_inc(IcmpstatCounters::IcpsTooshort);
        return IPPROTO_DONE;
    };
    let mut ip = ip_load(&m);
    if in4_cksum(&m, 0, hlen as i32, icmplen) != 0 {
        icmpstat_inc(IcmpstatCounters::IcpsChecksum);
        return IPPROTO_DONE;
    }

    let mut icp = icmp_at(&m, hlen);
    // ICMPPRINTFS: not configured.
    let then = 'sw: {
        if icp.icmp_type > ICMP_MAXTYPE {
            break 'sw Then::Raw;
        }
        // NPF > 0
        if m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED != 0 {
            match icp.icmp_type {
                // As pf_icmp_mapping() considers redirects belonging to a diverted connection,
                // we must include it here.
                // These ICMP types map to other connections. They must be delivered to
                // pr_ctlinput() also for diverted connections.
                ICMP_REDIRECT | ICMP_UNREACH | ICMP_TIMXCEED | ICMP_PARAMPROB
                | ICMP_SOURCEQUENCH => {
                    // Do not use the divert-to property of the TCP or UDP rule when doing the
                    // PCB lookup for the raw socket.
                    let f = &m.m_pkthdr().pf.flags;
                    f.set(f.get() & !PF_TAG_DIVERTED);
                }
                _ => break 'sw Then::Raw,
            }
        }
        icmpstat_inc_hist(IcmpstatCounters::IcpsInhist, icp.icmp_type);
        let mut code = i32::from(icp.icmp_code);
        // The deliver: label: the error advised to the protocol of the offending packet.
        let deliver = |code: i32, icp: &Icmp, m: &Mbuf| -> Then {
            // Problem with datagram; advise higher level routines.
            if icmplen_u < ICMP_ADVLENMIN
                || icmplen_u < icmp_advlen(icp)
                || usize::from(icp.icmp_ip().ip_hl()) < (IP_HDR_LEN >> 2)
            {
                icmpstat_inc(IcmpstatCounters::IcpsBadlen);
                return Then::Free;
            }
            if in_multicast(icp.icmp_ip().ip_dst.s_addr) {
                icmpstat_inc(IcmpstatCounters::IcpsBadcode);
                return Then::Raw;
            }
            // INET6: get more contiguous data for a v6 in v4 ICMP message.
            if i32::from(icp.icmp_ip().ip_p) == IPPROTO_IPV6
                && (icmplen_u < ICMP_V6ADVLENMIN || icmplen_u < icmp_v6advlen(icp))
            {
                icmpstat_inc(IcmpstatCounters::IcpsBadlen);
                return Then::Free;
            }
            // ICMPPRINTFS: not configured.
            let sin = sin_bytes(icp.icmp_ip().ip_dst);
            // NCARP > 0: carp_lsdrop; not configured.
            // XXX if the packet contains [IPv4 AH TCP], we can't make a notification to TCP
            // layer.
            if let Some(pr) = ip_protocol::<G>(i32::from(icp.icmp_ip().ip_p)) {
                // The message, from its type byte (`&icp->icmp_ip` is ICMP_MINLEN into it).
                let msg = m.data().get(hlen..).unwrap_or(&[]);
                pr.ctlinput(code, view(&sin), m.m_pkthdr().ph_rtableid.get(), Some(msg));
            }
            Then::Raw
        };
        match icp.icmp_type {
            ICMP_UNREACH => {
                code = match icp.icmp_code {
                    ICMP_UNREACH_NET
                    | ICMP_UNREACH_HOST
                    | ICMP_UNREACH_PROTOCOL
                    | ICMP_UNREACH_PORT
                    | ICMP_UNREACH_SRCFAIL => code + PRC_UNREACH_NET,
                    ICMP_UNREACH_NEEDFRAG => PRC_MSGSIZE,
                    ICMP_UNREACH_NET_UNKNOWN | ICMP_UNREACH_NET_PROHIB | ICMP_UNREACH_TOSNET => {
                        PRC_UNREACH_NET
                    }
                    ICMP_UNREACH_HOST_UNKNOWN
                    | ICMP_UNREACH_ISOLATED
                    | ICMP_UNREACH_HOST_PROHIB
                    | ICMP_UNREACH_TOSHOST
                    | ICMP_UNREACH_FILTER_PROHIB
                    | ICMP_UNREACH_HOST_PRECEDENCE
                    | ICMP_UNREACH_PRECEDENCE_CUTOFF => PRC_UNREACH_HOST,
                    _ => {
                        icmpstat_inc(IcmpstatCounters::IcpsBadcode);
                        break 'sw Then::Raw;
                    }
                };
                deliver(code, &icp, &m)
            }

            ICMP_TIMXCEED => {
                if code > 1 {
                    icmpstat_inc(IcmpstatCounters::IcpsBadcode);
                    break 'sw Then::Raw;
                }
                deliver(code + PRC_TIMXCEED_INTRANS, &icp, &m)
            }

            ICMP_PARAMPROB => {
                if code > 1 {
                    icmpstat_inc(IcmpstatCounters::IcpsBadcode);
                    break 'sw Then::Raw;
                }
                deliver(PRC_PARAMPROB, &icp, &m)
            }

            ICMP_SOURCEQUENCH => {
                if code != 0 {
                    icmpstat_inc(IcmpstatCounters::IcpsBadcode);
                    break 'sw Then::Raw;
                }
                deliver(PRC_QUENCH, &icp, &m)
            }

            ICMP_ECHO => {
                if ICMPBMCASTECHO.load(Ordering::Relaxed) == 0
                    && m.flags() & (M_MCAST | M_BCAST) != 0
                {
                    icmpstat_inc(IcmpstatCounters::IcpsBmcastecho);
                    break 'sw Then::Raw;
                }
                icp.icmp_type = ICMP_ECHOREPLY;
                if !pkt_put(&mut m, hlen, &[icp.icmp_type]) {
                    break 'sw unwritable();
                }
                Then::Reflect
            }

            ICMP_TSTAMP => {
                if ICMPTSTAMPREPL.load(Ordering::Relaxed) == 0 {
                    break 'sw Then::Raw;
                }

                if ICMPBMCASTECHO.load(Ordering::Relaxed) == 0
                    && m.flags() & (M_MCAST | M_BCAST) != 0
                {
                    icmpstat_inc(IcmpstatCounters::IcpsBmcastecho);
                    break 'sw Then::Raw;
                }
                if icmplen_u < ICMP_TSLEN {
                    icmpstat_inc(IcmpstatCounters::IcpsBadlen);
                    break 'sw Then::Raw;
                }
                icp.icmp_type = ICMP_TSTAMPREPLY;
                icp.set_icmp_rtime(iptime());
                icp.set_icmp_ttime(icp.icmp_rtime()); // bogus, do later!
                let b = icmp_bytes(&icp);
                if !(pkt_put(&mut m, hlen, &b[..1])
                    && pkt_put(
                        &mut m,
                        hlen + ICMP_IP_OFFSET + 4,
                        &b[ICMP_IP_OFFSET + 4..ICMP_TSLEN],
                    ))
                {
                    break 'sw unwritable();
                }
                Then::Reflect
            }

            ICMP_MASKREQ => {
                if ICMPMASKREPL.load(Ordering::Relaxed) == 0 {
                    break 'sw Then::Raw;
                }
                if icmplen_u < ICMP_MASKLEN {
                    icmpstat_inc(IcmpstatCounters::IcpsBadlen);
                    break 'sw Then::Raw;
                }
                // We are not able to respond with all ones broadcast unless we receive it over
                // a point-to-point interface.
                let a = if ip.ip_dst.s_addr == INADDR_BROADCAST || ip.ip_dst.s_addr == INADDR_ANY {
                    ip.ip_src
                } else {
                    ip.ip_dst
                };
                let Some(ia) = ifaof_ifpforaddr(net, &sin_bytes(a), ifp).and_then(InIfaddr::of)
                else {
                    break 'sw Then::Raw;
                };
                icp.icmp_type = ICMP_MASKREPLY;
                icp.set_icmp_mask(ia.ia_sockmask.addr().s_addr);
                if ip.ip_src.s_addr == 0 {
                    if ifp.if_flags.get() & IFF_BROADCAST != 0 {
                        let b = ia.ia_broadaddr().addr();
                        if b.s_addr != 0 {
                            ip.ip_src = b;
                        } else {
                            ip.ip_src.s_addr = INADDR_BROADCAST;
                        }
                    } else if ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
                        ip.ip_src = ia.ia_dstaddr.addr();
                    }
                    if !ip_store(&mut m, &ip) {
                        break 'sw unwritable();
                    }
                }
                let b = icmp_bytes(&icp);
                if !(pkt_put(&mut m, hlen, &b[..1])
                    && pkt_put(
                        &mut m,
                        hlen + ICMP_IP_OFFSET,
                        &b[ICMP_IP_OFFSET..ICMP_MASKLEN],
                    ))
                {
                    break 'sw unwritable();
                }
                Then::Reflect
            }

            ICMP_REDIRECT => {
                let i_am_router = ip_forwarding.load(Ordering::Relaxed) != 0;

                if ICMP_REDIRACCEPT.load(Ordering::Relaxed) == 0 || i_am_router {
                    break 'sw Then::Free;
                }
                if code > 3 {
                    icmpstat_inc(IcmpstatCounters::IcpsBadcode);
                    break 'sw Then::Raw;
                }
                if icmplen_u < ICMP_ADVLENMIN
                    || icmplen_u < icmp_advlen(&icp)
                    || usize::from(icp.icmp_ip().ip_hl()) < (IP_HDR_LEN >> 2)
                {
                    icmpstat_inc(IcmpstatCounters::IcpsBadlen);
                    break 'sw Then::Raw;
                }
                // Short circuit routing redirects to force immediate change in the kernel's
                // routing tables. The message is also handed to anyone listening on a raw
                // socket (e.g. the routing daemon for use in updating its tables).
                let sdst = sin_bytes(icp.icmp_ip().ip_dst);
                let sgw = sin_bytes(icp.icmp_gwaddr());
                let ssrc = sin_bytes(ip.ip_src);

                // ICMPPRINTFS: not configured.
                // NCARP > 0: carp_lsdrop; not configured.
                let rtableid = m.m_pkthdr().ph_rtableid.get();
                let mut newrt = None;
                rtredirect(
                    view(&sdst),
                    view(&sgw),
                    view(&ssrc),
                    Some(&mut newrt),
                    rtableid,
                    net,
                );
                if let Some(rt) = newrt.as_ref()
                    && ICMP_REDIRTIMEOUT.load(Ordering::Relaxed) > 0
                {
                    let _ = rt_timer_add(rt, &ICMP_REDIRECT_TIMEOUT_Q, rtableid);
                }
                drop(newrt);
                pfctlinput::<G>(PRC_REDIRECT_HOST, view(&sdst));
                Then::Raw
            }

            // No kernel processing for the following; just fall through to send to raw
            // listener.
            ICMP_ECHOREPLY
            | ICMP_ROUTERADVERT
            | ICMP_ROUTERSOLICIT
            | ICMP_TSTAMPREPLY
            | ICMP_IREQREPLY
            | ICMP_MASKREPLY
            | ICMP_TRACEROUTE
            | ICMP_DATACONVERR
            | ICMP_MOBILE_REDIRECT
            | ICMP_IPV6_WHEREAREYOU
            | ICMP_IPV6_IAMHERE
            | ICMP_MOBILE_REGREQUEST
            | ICMP_MOBILE_REGREPLY
            | ICMP_PHOTURIS => Then::Raw,
            _ => Then::Raw,
        }
    };

    match then {
        Then::Raw => {
            // raw:
            *mp = Some(m);
            match raw_protocol::<G>() {
                Some(raw) => raw.input(mp, offp, proto, af, ns, net),
                None => {
                    drop(mp.take());
                    IPPROTO_DONE
                }
            }
        }
        Then::Free => IPPROTO_DONE,
        Then::Reflect => {
            // reflect: NCARP > 0: carp_lsdrop; not configured.
            icmpstat_inc(IcmpstatCounters::IcpsReflect);
            icmpstat_inc_hist(IcmpstatCounters::IcpsOuthist, icp.icmp_type);
            let mut opts = None;
            if let Some(m) = icmp_reflect(m, Some(&mut opts), None, net) {
                icmp_send(m, opts.as_deref().map(Mbuf::data));
            }
            drop(opts);
            IPPROTO_DONE
        }
    }
}

/// `icmp_reflect(m, op, ia)`: reflects the IP packet `m` back to the source; the packet back,
/// with the options to send it with in `*op` (when asked), or `None` when it was freed (a bad
/// return address, `EHOSTUNREACH`; a loop, `ELOOP`).
pub fn icmp_reflect(
    m: MbufBox,
    op: Option<&mut Option<MbufBox>>,
    ia: Option<&'static InIfaddr>,
    net: &NetHeld<'_>,
) -> Option<MbufBox> {
    let mut m = m;
    let mut ip = ip_load(&m);
    let mut opts: Option<MbufBox> = None;
    let mut ip_src = InAddr { s_addr: INADDR_ANY };
    let optlen = (usize::from(ip.ip_hl()) << 2) - IP_HDR_LEN;

    if !in_canforward(ip.ip_src)
        && (ip.ip_src.s_addr & IN_CLASSA_NET) != (IN_LOOPBACKNET << IN_CLASSA_NSHIFT).to_be()
    {
        // Bad return address: EHOSTUNREACH.
        return None;
    }

    let ph = m.m_pkthdr();
    let loopcnt = ph.ph_loopcnt.get();
    ph.ph_loopcnt.set(loopcnt.wrapping_add(1));
    if loopcnt >= M_MAXLOOP {
        // ELOOP.
        return None;
    }
    let rtableid = ph.ph_rtableid.get();

    // If the incoming packet was addressed directly to us, use dst as the src for the reply.
    // For broadcast, use the address which corresponds to the incoming interface.
    match ia {
        None => {
            let sin = sin_bytes(ip.ip_dst);
            let rt = rtalloc(view(&sin), 0, rtableid, net);
            if let Some(rt) = rt.as_deref()
                && rtisvalid(Some(rt))
            {
                if rt.rt_flags.isset(RTF_LOCAL) {
                    ip_src = ip.ip_dst;
                } else if rt.rt_flags.isset(RTF_BROADCAST)
                    && let Some(ia) = rt.ifa().and_then(InIfaddr::of)
                {
                    ip_src = ia.ia_addr.addr();
                }
            }
        }
        Some(ia) => ip_src = ia.ia_addr.addr(),
    }

    // The following happens if the packet was not addressed to us. If we're directly
    // connected use the closest address, otherwise try to use the sourceaddr from the routing
    // table.
    if ip_src.s_addr == INADDR_ANY {
        let sin = sin_bytes(ip.ip_src);
        // keep packet in the original virtual instance
        let rt = rtalloc(view(&sin), RT_RESOLVE, rtableid, net);
        match rt.as_deref() {
            Some(rt) if rtisvalid(Some(rt)) && !rt.rt_flags.isset(RTF_GATEWAY) => {
                if let Some(ia) = rt.ifa().and_then(InIfaddr::of) {
                    ip_src = ia.ia_addr.addr();
                }
            }
            _ => {
                if let Some(sourceaddr) = rtable_getsource(rtableid, AF_INET, net) {
                    let sa = sourceaddr.view();
                    if let Some(ifa) = ifa_ifwithaddr(net, sa.bytes(), rtableid)
                        && ifa
                            .ifa_ifp
                            .get()
                            .is_some_and(|ifp| ifp.if_flags.get() & IFF_UP != 0)
                        && let Some(s) = pod::at::<SockaddrIn>(sa.bytes(), 0)
                    {
                        ip_src = s.sin_addr;
                    }
                }
            }
        }
    }

    // If the above didn't find an ip_src, ip_output() will try and fill it in for us.

    let pfflags = m.m_pkthdr().pf.flags.get();

    m_resethdr(&mut m);
    let ph = m.m_pkthdr();
    ph.ph_rtableid.set(rtableid);
    ph.pf.flags.set(pfflags & PF_TAG_GENERATED);
    ip.ip_dst = ip.ip_src;
    ip.ip_src = ip_src;
    ip.ip_ttl = MAXTTL;
    // The header, writable since ipv4_check: not taken; dropped as the C's icmp_reflect
    // drops what it cannot build.
    if !ip_store(&mut m, &ip) {
        return None;
    }

    if optlen > 0 {
        // Retrieve any source routing from the incoming packet; add on any record-route or
        // timestamp options.
        if op.is_some() {
            opts = ip_srcroute(&mut m);
            if opts.is_none()
                && let Some(mut o) = m_gethdr(M_DONTWAIT, MT_HEADER)
            {
                o.m_len().set(size_of::<InAddr>() as u32);
                // A fresh header mbuf: written in place.
                if pkt_put(&mut o, 0, &[0; 4]) {
                    opts = Some(o);
                }
            }
        }
        // The options' mbuf is fresh: its writes are in place; without one the reply goes
        // without options, as the C's when it cannot get the mbuf.
        let mut written = true;
        if let Some(o) = opts.as_mut() {
            // ICMPPRINTFS: not configured.
            let mut cp = [0u8; 40];
            m_copydata(&m, IP_HDR_LEN as i32, &mut cp[..optlen]);
            let cp = &cp[..optlen];
            let mut at = 0usize;
            let mut cnt = optlen as i32;
            while cnt > 0 {
                let opt = cp[at + IPOPT_OPTVAL];
                if opt == IPOPT_EOL {
                    break;
                }
                let len: i32 = if opt == IPOPT_NOP {
                    1
                } else {
                    if cnt < (IPOPT_OLEN + 1) as i32 {
                        break;
                    }
                    let len = i32::from(cp[at + IPOPT_OLEN]);
                    if len < (IPOPT_OLEN + 1) as i32 || len > cnt {
                        break;
                    }
                    len
                };
                // Should check for overflow, but it "can't happen"
                if opt == IPOPT_RR || opt == IPOPT_TS || opt == IPOPT_SECURITY {
                    let ol = o.m_len().get() as usize;
                    o.m_len().set((ol + len as usize) as u32);
                    written &= pkt_put(o, ol, &cp[at..at + len as usize]);
                }
                cnt -= len;
                at += len as usize;
            }
            // Terminate & pad, if necessary
            let mut cnt = o.m_len().get() % 4;
            if cnt != 0 {
                while cnt < 4 {
                    let ol = o.m_len().get() as usize;
                    o.m_len().set((ol + 1) as u32);
                    written &= pkt_put(o, ol, &[IPOPT_EOL]);
                    cnt += 1;
                }
            }
            // ICMPPRINTFS: not configured.
        }
        if !written {
            opts = None;
        }
        if !ip_stripoptions(&mut m) {
            return None;
        }
    }
    m.m_flags().set(m.flags() & !(M_BCAST | M_MCAST));
    if let Some(op) = op {
        *op = opts;
    }

    Some(m)
}

/// `icmp_send(m, opts)`: sends an icmp packet back to the ip level, with the options `opts`
/// (the bytes of a `struct ipoption`: the first hop, then the options) if any.
pub fn icmp_send(m: MbufBox, opts: Option<&[u8]>) {
    let mut m = m;
    let ip = ip_load(&m);
    let mut hlen = i32::from(ip.ip_hl()) << 2;

    // The reflected message, writable since ipv4_check: not taken; dropped unsent.
    if !pkt_put(&mut m, hlen as usize + ICMP_CKSUM_OFFSET, &[0, 0]) {
        return;
    }
    m.m_pkthdr().csum_flags.set(M_ICMP_CSUM_OUT);
    // ICMPPRINTFS: not configured.
    // ip_send() cannot handle IP options properly. So in case we have options fill out the IP
    // header here and use ip_send_raw() instead.
    match opts {
        Some(opts) => {
            let mut m = ip_insertoptions(m, opts, &mut hlen);
            let mut ip = ip_load(&m);
            ip.set_ip_hl((hlen >> 2) as u8);
            ip.set_ip_v(IPVERSION);
            ip.ip_off &= IP_DF.to_be();
            ip.ip_id = ip_randomid().to_be();
            if !ip_store(&mut m, &ip) {
                return;
            }
            ipstat_inc(IpstatCounters::IpsLocalout);
            ip_send_raw(m);
        }
        None => ip_send(m),
    }
}

/// `iptime()`: the time since midnight UTC in milliseconds, network order (a timestamp of
/// the timestamp messages and option).
pub fn iptime() -> u32 {
    let atv = crate::tc::microtime();
    let t = (atv.tv_sec % (24 * 60 * 60)) * 1000 + atv.tv_usec / 1000;
    (t as u32).to_be()
}

/// `icmp_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the `net.inet.icmp` sysctls;
/// `p` is `curproc` (`ICMPCTL_REDIRTIMEOUT` wires the old value's pages).
pub fn icmp_sysctl<G: ProcGlue>(
    p: &crate::proc::Thread<G>,
    name: &[i32],
    req: &mut SysctlReq<'_>,
) -> Result<(), Errno> {
    // All sysctl names at this level are terminal.
    let [first] = *name else {
        return Err(Errno::ENOTDIR);
    };

    match first {
        ICMPCTL_REDIRTIMEOUT => {
            let savelen = *req.oldlenp;

            let vs = sysctl_vslock(p, req.oldp, savelen)?;
            let w = NETLOCK.write();
            let error = int_bounded(req, &ICMP_REDIRTIMEOUT, 0, INT_MAX);
            rt_timer_queue_change(
                &ICMP_REDIRECT_TIMEOUT_Q,
                ICMP_REDIRTIMEOUT.load(Ordering::Relaxed),
            );
            drop(w);
            drop(vs);
            error
        }
        ICMPCTL_STATS => icmp_sysctl_icmpstat(req),
        _ => bounded_arr(&ICMPCTL_VARS, name, req),
    }
}

/// `icmp_sysctl_icmpstat(oldp, oldlenp, newp)`: `net.inet.icmp.stats`, the counters as a
/// `struct icmpstat`.
fn icmp_sysctl_icmpstat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let mut bytes = [0u8; ICPS_NCOUNTERS * size_of::<u64>()];
    for (i, c) in ICMPCOUNTERS.read().iter().enumerate() {
        bytes[i * 8..i * 8 + 8].copy_from_slice(&c.to_ne_bytes());
    }
    rdstruct(req, &bytes)
}

/// `icmp_mtudisc_clone(dst, rtableid, ipsec)`: a host route to `dst` in `rtableid` that can
/// carry a path MTU (with a timer of `ip_mtudisc_timeout_q`), cloned when the route found is
/// not one; `None` when no usable route can hold one.
pub fn icmp_mtudisc_clone(
    dst: InAddr,
    rtableid: u32,
    ipsec: bool,
    net: &NetHeld<'_>,
) -> Option<hal::pool::PoolArc<Rtentry>> {
    let sin = sin_bytes(dst);

    let mut rt = rtalloc(view(&sin), RT_RESOLVE, rtableid, net)?;

    // Check if the route is actually usable
    if !rtisvalid(Some(&rt)) {
        return None;
    }
    // IPsec needs the route only for PMTU, it can use reject for that
    if !ipsec && rt.rt_flags.isset(RTF_REJECT | RTF_BLACKHOLE) {
        return None;
    }

    // No PMTU for local routes and permanent neighbors, ARP and NDP use the same expire timer
    // as the route.
    if rt.rt_flags.isset(RTF_LOCAL) || (rt.rt_flags.isset(RTF_LLINFO) && rt.rt_expire().get() == 0)
    {
        return None;
    }

    // If we didn't get a host route, allocate one
    if !rt.rt_flags.isset(RTF_HOST) {
        let mut info = RtAddrinfo::new();
        let mut sa_rl = SockaddrRtlabel::default();
        let gw = rt.gateway_copy();

        info.rti_ifa = rt.rt_ifa.get();
        info.rti_flags = RTF_GATEWAY | RTF_HOST | RTF_DYNAMIC;
        info.rti_info[RTAX_DST] = Some(view(&sin));
        info.rti_info[RTAX_GATEWAY] = gw.as_ref().map(|g| g.view());
        info.rti_info[RTAX_LABEL] = rtlabel_id2sa(rt.rt_labelid.get(), &mut sa_rl);

        let mut nrt = None;
        rtrequest(
            RTM_ADD,
            &mut info,
            rt.rt_priority.get(),
            Some(&mut nrt),
            rtableid,
            net,
        )
        .ok()?;
        let n = nrt?;
        n.rt_rmx.assign(&rt.rt_rmx);
        rt = n;
        rtm_send(&rt, RTM_ADD, 0, rtableid, net);
    }
    rt_timer_add(&rt, &IP_MTUDISC_TIMEOUT_Q, rtableid).ok()?;

    Some(rt)
}

/// `icmp_mtudisc(icp, rtableid)`: a path MTU learned from a "fragmentation needed" error:
/// `nextmtu` (network order) the message's next-hop MTU, `oip` the header it returned.
pub fn icmp_mtudisc(nextmtu: u16, oip: &Ip, rtableid: u32, net: &NetHeld<'_>) {
    let mut mtu = u64::from(u16::from_be(nextmtu)); // Why a long?  IPv6

    let Some(rt) = icmp_mtudisc_clone(oip.ip_dst, rtableid, false, net) else {
        return;
    };

    let Some(ifp) = if_get(rt.rt_ifidx.get()) else {
        return;
    };

    let rtmtu = rt.rt_mtu().load(Ordering::Relaxed);
    if mtu == 0 {
        mtu = u64::from(u16::from_be(oip.ip_len));
        // Some 4.2BSD-based routers incorrectly adjust the ip_len
        if mtu > u64::from(rtmtu) && rtmtu != 0 {
            // The C's u_long arithmetic: it wraps.
            mtu = mtu.wrapping_sub(u64::from(oip.ip_hl()) << 2);
        }

        // If we still can't guess a value, try the route
        if mtu == 0 {
            mtu = u64::from(rtmtu);

            // If no route mtu, default to the interface mtu
            if mtu == 0 {
                mtu = u64::from(ifp.if_mtu.get());
            }
        }

        if let Some(&t) = MTU_TABLE.iter().find(|&&t| mtu > u64::from(t)) {
            mtu = u64::from(t);
        }
    }

    // XXX: RTV_MTU is overloaded, since the admin can set it to turn off PMTU for a route, and
    // the kernel can set it to indicate a serious problem with PMTU on a route. We should be
    // using a separate flag for the kernel to indicate this.
    if rt.rt_locks().get() & RTV_MTU == 0 {
        if mtu < 296 || mtu > u64::from(ifp.if_mtu.get()) {
            rt.rt_locks().set(rt.rt_locks().get() | RTV_MTU);
        } else if u64::from(rtmtu) > mtu || rtmtu == 0 {
            let _ = rt.rt_mtu().compare_exchange(
                rtmtu,
                mtu as u32,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
        }
    }

    if_put(Some(ifp));
}

/// `icmp_mtudisc_timeout(rt, rtableid)`: a learned path MTU expired (`ip_mtudisc_timeout_q`'s
/// function): a cloned host route deleted and TCP told the MTU may grow, else the route's
/// MTU forgotten. Under the net lock (`net`, the timer's).
pub fn icmp_mtudisc_timeout<G: ProcGlue>(
    rt: &hal::pool::PoolArc<Rtentry>,
    rtableid: u32,
    net: &NetHeld<'_>,
) {
    let Some(ifp) = if_get(rt.rt_ifidx.get()) else {
        return;
    };

    if rt.rt_flags.get() & (RTF_DYNAMIC | RTF_HOST) == (RTF_DYNAMIC | RTF_HOST) {
        // A route's key in the inet table is a `sockaddr_in`.
        let mut sin = [0u8; size_of::<SockaddrIn>()];
        let key = rt.dest();
        let n = key.len().min(sin.len());
        sin[..n].copy_from_slice(&key[..n]);

        let _ = rtdeletemsg(rt, ifp, rtableid, net);

        // Notify TCP layer of increased Path MTU estimate
        if let Some(pr) = ip_protocol::<G>(IPPROTO_TCP) {
            pr.ctlinput(PRC_MTUINC, view(&sin), rtableid, None);
        }
    } else if rt.rt_locks().get() & RTV_MTU == 0 {
        rt.rt_mtu().store(0, Ordering::Relaxed);
    }

    if_put(Some(ifp));
}

/// `icmp_ratelimit(dst, type, code)`: perform rate limit check. `false` if it is okay to send
/// the icmp packet, `true` if the router SHOULD NOT send this icmp packet due to rate
/// limitation. XXX per-destination/type check necessary?
pub fn icmp_ratelimit(_dst: &InAddr, _type: u8, _code: u8) -> bool {
    let icmperrppslim_local = ICMPERRPPSLIM.load(Ordering::Relaxed);
    // PPS limit
    let ok = ppsratecheck_locked(|l| {
        let (mut last, mut count) = {
            let st = ICMPERRPPS.get(l.witness());
            (st.last, st.count)
        };
        let ok = ppsratecheck_held(l, &mut last, &mut count, icmperrppslim_local);
        let st = ICMPERRPPS.get_mut(l.witness_mut());
        st.last = last;
        st.count = count;
        ok
    });
    // The packet is subject to rate limit; okay to send.
    !ok
}

/// `icmp_do_exthdr(m, class, ctype, buf, len)`: an RFC 4884 extension object of class
/// `class`, type `ctype` and data `buf` appended to the ICMP error `m` (whose header is at
/// its start). `ENOBUFS` (the packet freed) when it cannot grow; the packet as it was for a
/// message that takes no extension or already has one.
pub fn icmp_do_exthdr(m: MbufBox, class: u16, ctype: u8, buf: &[u8]) -> Result<MbufBox, Errno> {
    let mut m = m;
    let mut ip = ip_load(&m);
    let mut hlen = usize::from(ip.ip_hl()) << 2;
    let mut icp = icmp_at(&m, hlen);
    if icp.icmp_type != ICMP_TIMXCEED
        && icp.icmp_type != ICMP_UNREACH
        && icp.icmp_type != ICMP_PARAMPROB
    {
        // exthdr not supported
        return Ok(m);
    }

    if icp.icmp_length() != 0 {
        // exthdr already present, giving up
        return Ok(m);
    }

    // the actual offset starts after the common ICMP header
    hlen += ICMP_MINLEN;
    // exthdr must start on a word boundary
    let off = (usize::from(u16::from_be(ip.ip_len)).wrapping_sub(hlen)).next_multiple_of(4);
    // ... and at an offset of ICMP_EXT_OFFSET or bigger
    let off = off.max(ICMP_EXT_OFFSET);
    icp.set_icmp_length((off / size_of::<u32>()) as u8);
    let ib = icmp_bytes(&icp);
    if !pkt_put(
        &mut m,
        hlen - ICMP_MINLEN + ICMP_HUN_OFFSET,
        &ib[ICMP_HUN_OFFSET..ICMP_HUN_OFFSET + 4],
    ) {
        return Err(Errno::ENOBUFS);
    }

    let ieh = IcmpExtHdr {
        ieh_version: ICMP_EXT_HDR_VERSION,
        ..IcmpExtHdr::default()
    };
    let ieo = IcmpExtObjHdr {
        ieo_length: ((size_of::<IcmpExtObjHdr>() + buf.len()) as u16).to_be(),
        ieo_cnum: class as u8,
        ieo_ctype: ctype,
    };
    let mut hdr = [0u8; size_of::<IcmpExtHdr>() + size_of::<IcmpExtObjHdr>()];
    let _ = pod::put(&mut hdr, 0, &ieh);
    let _ = pod::put(&mut hdr, size_of::<IcmpExtHdr>(), &ieo);

    let at = (hlen + off) as i32;
    if m_copyback(&mut m, at, &hdr, M_DONTWAIT).is_err()
        || m_copyback(&mut m, at + hdr.len() as i32, buf, M_DONTWAIT).is_err()
    {
        return Err(Errno::ENOBUFS);
    }

    // calculate checksum
    let Some((n, noff)) = m_getptr(&m, at) else {
        kpanic(format_args!("icmp_do_exthdr: m_getptr failure"));
    };
    let sum = in4_cksum(n, 0, noff, (hdr.len() + buf.len()) as i32);
    let pos = 2usize; // offsetof(struct icmp_ext_hdr, ieh_cksum)
    m_copyback(&mut m, at + pos as i32, &sum.to_ne_bytes(), M_DONTWAIT)
        .map_err(|_| Errno::ENOBUFS)?;

    ip.ip_len = (m.m_pkthdr().len.get() as u16).to_be();
    if !ip_store(&mut m, &ip) {
        return Err(Errno::ENOBUFS);
    }

    Ok(m)
}

/// The rate limiter's state forgotten for the host tests (each test starts with the limit's
/// window open).
#[cfg(any(test, feature = "doubles"))]
pub fn icmp_test_reset() {
    ppsratecheck_locked(|l| {
        let st = ICMPERRPPS.get_mut(l.witness_mut());
        st.last = Timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        st.count = 0;
    });
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for ICMP over the IPv4 tests' switch and interface (10.0.2.15/24 on ipo0):
    // the input's length and checksum refusals, each type's minimum length, an echo request
    // answered, the advice to the offending packet's protocol, and `icmp_error`'s refusals
    // (a later fragment, a broadcast, an ICMP error) and its rate limit.

    use std::vec::Vec;

    use hal::sys::netinet_in::IPPROTO_UDP;

    use super::*;
    use crate::net::mbuf::ml_dequeue;
    use crate::netinet::ip_input::ip_send_test_take;
    use crate::netinet::ip_input::tests::{G, UDP, bytes, cksum, header, packet, setup};

    fn icps(c: IcmpstatCounters) -> u64 {
        ICMPCOUNTERS.read()[c as usize]
    }

    /// An ICMP message from 10.0.2.2 to us: `msg` after a 20-byte header, its checksum set
    /// unless `bad_sum`, received on `ifp`.
    fn icmp_packet(ifp: &Ifnet, msg: &[u8], bad_sum: bool) -> MbufBox {
        let mut m = msg.to_vec();
        if m.len() % 2 == 1 {
            m.push(0);
        }
        let len = msg.len();
        if len >= 4 {
            m[2] = 0;
            m[3] = 0;
            let s = cksum(&m[..len + len % 2]);
            m[2..4].copy_from_slice(&s.to_be_bytes());
            if bad_sum {
                m[2] ^= 0xff;
            }
        }
        let mut p = header(0x45, (20 + len) as u16, 9, 0, IPPROTO_ICMP as u8, 0).to_vec();
        p.extend_from_slice(&m[..len]);
        let pk = packet(&p, 20);
        pk.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        pk
    }

    /// `icmp_input` of `m`, under the exclusive net lock as `ipintr` runs it.
    fn input(m: MbufBox) -> i32 {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut mp = Some(m);
        let mut off = 20;
        icmp_input::<G>(
            &mut mp,
            &mut off,
            IPPROTO_ICMP,
            i32::from(AF_INET),
            None,
            &net,
        )
    }

    /// What `ip_send` queued since the last call: each packet's bytes.
    fn sent() -> Vec<Vec<u8>> {
        let mut ml = ip_send_test_take();
        let mut v = Vec::new();
        while let Some(m) = ml_dequeue(&mut ml) {
            v.push(bytes(&m));
        }
        v
    }

    #[test]
    fn short_and_bad_messages_are_refused() {
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        let _ = sent();
        let short = icps(IcmpstatCounters::IcpsTooshort);
        let _ = input(icmp_packet(ifp, &[ICMP_ECHO, 0, 0, 0], false));
        assert_eq!(icps(IcmpstatCounters::IcpsTooshort), short + 1);
        let sum = icps(IcmpstatCounters::IcpsChecksum);
        let _ = input(icmp_packet(ifp, &[ICMP_ECHO, 0, 0, 0, 0, 1, 0, 1], true));
        assert_eq!(icps(IcmpstatCounters::IcpsChecksum), sum + 1);
        // A timestamp request shorter than ICMP_TSLEN, a mask request shorter than
        // ICMP_MASKLEN (with the reply enabled): bad lengths, not answered.
        let badlen = icps(IcmpstatCounters::IcpsBadlen);
        let _ = input(icmp_packet(
            ifp,
            &[ICMP_TSTAMP, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0],
            false,
        ));
        ICMPMASKREPL.store(1, Ordering::Relaxed);
        let _ = input(icmp_packet(
            ifp,
            &[ICMP_MASKREQ, 0, 0, 0, 0, 1, 0, 1, 0, 0],
            false,
        ));
        ICMPMASKREPL.store(0, Ordering::Relaxed);
        assert_eq!(icps(IcmpstatCounters::IcpsBadlen), badlen + 2);
        // An unreachable message too short to hold a header and 8 bytes of its data.
        let _ = input(icmp_packet(
            ifp,
            &[ICMP_UNREACH, 3, 0, 0, 0, 0, 0, 0, 0x45, 0],
            false,
        ));
        assert_eq!(icps(IcmpstatCounters::IcpsBadlen), badlen + 3);
        assert!(sent().is_empty(), "nothing answered");
    }

    #[test]
    fn an_echo_request_is_answered() {
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        let _ = sent();
        let reflect = icps(IcmpstatCounters::IcpsReflect);
        let _ = input(icmp_packet(
            ifp,
            &[ICMP_ECHO, 0, 0, 0, 0, 9, 0, 1, 7, 7],
            false,
        ));
        assert_eq!(icps(IcmpstatCounters::IcpsReflect), reflect + 1);
        let s = sent();
        assert_eq!(s.len(), 1);
        let b = &s[0];
        assert_eq!(
            &b[12..16],
            &[10, 0, 2, 15],
            "from the address the request was for"
        );
        assert_eq!(&b[16..20], &[10, 0, 2, 2], "back to the sender");
        assert_eq!(b[8], MAXTTL);
        assert_eq!(b[20], ICMP_ECHOREPLY);
        assert_eq!(
            &b[24..30],
            &[0, 9, 0, 1, 7, 7],
            "the id, the sequence and the data stay"
        );
    }

    #[test]
    fn errors_are_advised_to_the_offending_protocol() {
        let _g = setup();
        let (ifp, _) = crate::netinet::ip_output::tests::iface();
        // Port unreachable about a UDP datagram we sent to 10.0.2.2.
        let mut msg = std::vec![ICMP_UNREACH, ICMP_UNREACH_PORT, 0, 0, 0, 0, 0, 0];
        let mut inner = header(0x45, 28, 5, 0, IPPROTO_UDP as u8, 0);
        inner[12..16].copy_from_slice(&[10, 0, 2, 15]);
        inner[16..20].copy_from_slice(&[10, 0, 2, 2]);
        msg.extend_from_slice(&inner);
        msg.extend_from_slice(&[0x30, 0x39, 0x00, 0x35, 0, 8, 0, 0]);
        UDP.ctl.store(0, Ordering::Relaxed);
        let _ = input(icmp_packet(ifp, &msg, false));
        assert_eq!(
            UDP.ctl.load(Ordering::Relaxed),
            PRC_UNREACH_NET + i32::from(ICMP_UNREACH_PORT)
        );
        // An unknown unreachable code is a bad code, not advised.
        msg[1] = 99;
        UDP.ctl.store(0, Ordering::Relaxed);
        let badcode = icps(IcmpstatCounters::IcpsBadcode);
        let _ = input(icmp_packet(ifp, &msg, false));
        assert_eq!(icps(IcmpstatCounters::IcpsBadcode), badcode + 1);
        assert_eq!(UDP.ctl.load(Ordering::Relaxed), 0);
    }

    /// A UDP datagram from 10.0.2.2 to us, `off` its `ip_off` word, `flags` its mbuf flags.
    fn datagram(off: u16, flags: u16) -> MbufBox {
        let mut p = header(0x45, 36, 6, off, IPPROTO_UDP as u8, 0).to_vec();
        p.extend_from_slice(&[0x30, 0x39, 0x00, 0x35, 0, 16, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
        let m = packet(&p, 20);
        m.m_flags().set(m.flags() | flags);
        m
    }

    fn error(m: MbufBox) {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        icmp_error(m, ICMP_UNREACH, ICMP_UNREACH_PORT, 0, 0, &net);
    }

    #[test]
    fn icmp_error_refuses_what_the_c_refuses_and_is_rate_limited() {
        let _g = setup();
        let (_ifp, _) = crate::netinet::ip_output::tests::iface();
        let _ = sent();
        let lim = ICMPERRPPSLIM.swap(-1, Ordering::Relaxed);
        // A later fragment, a broadcast, a multicast: no error.
        error(datagram(2, 0));
        error(datagram(0, M_BCAST));
        error(datagram(0, M_MCAST));
        assert!(sent().is_empty());
        // An ICMP error about an ICMP error: none either.
        let oldicmp = icps(IcmpstatCounters::IcpsOldicmp);
        let mut p = header(0x45, 28, 6, 0, IPPROTO_ICMP as u8, 0).to_vec();
        p.extend_from_slice(&[ICMP_UNREACH, 1, 0, 0, 0, 0, 0, 0]);
        error(packet(&p, 28));
        assert_eq!(icps(IcmpstatCounters::IcpsOldicmp), oldicmp + 1);
        assert!(sent().is_empty());
        // A datagram: the error quotes its header and 8 bytes.
        error(datagram(0, 0));
        let s = sent();
        assert_eq!(s.len(), 1);
        let b = &s[0];
        assert_eq!(&b[16..20], &[10, 0, 2, 2], "to the sender");
        assert_eq!((b[20], b[21]), (ICMP_UNREACH, ICMP_UNREACH_PORT));
        assert_eq!(b.len(), 20 + 8 + 20 + 8, "the header and 8 bytes quoted");
        assert_eq!(&b[48..56], &[0x30, 0x39, 0x00, 0x35, 0, 16, 0, 0]);
        // One error a second: the second within the second is too frequent.
        ICMPERRPPSLIM.store(1, Ordering::Relaxed);
        icmp_test_reset();
        let toofreq = icps(IcmpstatCounters::IcpsToofreq);
        error(datagram(0, 0));
        error(datagram(0, 0));
        assert_eq!(icps(IcmpstatCounters::IcpsToofreq), toofreq + 1);
        assert_eq!(sent().len(), 1);
        ICMPERRPPSLIM.store(lim, Ordering::Relaxed);
    }
}
/* </TESTS> */
