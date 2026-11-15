/*	$KAME: mld6.c,v 1.26 2001/02/16 14:50:35 itojun Exp $	*/
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
//! Multicast Listener Discovery: `<netinet6/mld6.h>` (`hal::sys::netinet6_mld6`) and
//! `netinet6/mld6.c`, the kernel's since N6.5a (`kernel::netinet6::mld6`), re-exported whole;
//! LZ's tests drive it through `bsd`'s Ethernet fixture and output path.
//!
//! Upstream: sys/netinet6/mld6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/mld6.c @ 3ce1f3f79392

pub use hal::sys::netinet6_mld6::*;
pub use kernel::netinet6::mld6::*;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use crate::net::if_var::IfnetLegacy;
    // Host tests for MLD: the message field macros, and the listener's timer arithmetic and state
    // transitions on a crafted query, a report and the fast timeout.

    use core::mem::size_of;
    use core::ptr;
    use core::sync::atomic::Ordering;
    use std::sync::MutexGuard;

    use super::*;
    use crate::net::if_::net_held;
    use crate::net::if_::tests::test_packet;
    use crate::net::if_var::Ifnet;
    use crate::netinet::icmp6::{
        Icmp6Hdr, Icmp6statCounters, MLD_LISTENER_DONE, MLD_LISTENER_QUERY, MLD_LISTENER_REPORT,
        MldHdr,
    };
    use crate::netinet::in_::IPPROTO_ICMPV6;
    use crate::netinet::ip_input::tests::setup as setup_ip;
    use crate::netinet::ip6::{IP6OPT_PADN, IP6OPT_ROUTER_ALERT, IPV6_VERSION, Ip6Hdr};
    use crate::netinet6::icmp6::ICMP6COUNTERS;
    use crate::netinet6::in6::tests::{a6, test_ia6, test_if};
    use crate::netinet6::in6::{
        IN6ADDR_ANY, IN6ADDR_LINKLOCAL_ALLNODES, IN6ADDR_LINKLOCAL_ALLROUTERS, In6Addr,
        in6_addmulti, in6_is_addr_mc_linklocal,
    };
    use crate::netinet6::in6_var::In6Multi;
    use crate::netinet6::mld6_var::{MLD_IREPORTEDLAST, MLD_OTHERLISTENER, Mld6Pktinfo};
    use crate::netinet6::nd6::{nd6_ifattach, nd6_init};
    use crate::sys::endian::htons;
    use crate::sys::errno::Errno;
    use crate::sys::mbuf::{M_LOOP, Mbuf, unpark};
    use crate::sys::protosw::PR_FASTHZ;
    use crate::sys::systm::{net_lock, net_unlock};
    use kernel::netinet6::icmp6::icmp6_init;

    /// `mld6_input(m, off)` under the net lock, as `icmp6_input` calls it.
    fn input(m: &'static Mbuf, off: i32) {
        net_lock();
        mld6_input(unpark(m), off, &net_held("mld6_input"));
        net_unlock();
    }

    #[test]
    fn query_fields() {
        // Maximum Response Code 0x7123: exponent 7, mantissa 0x123.
        assert_eq!(mld_mrc_exp(htons(0x7123)), 7);
        assert_eq!(mld_mrc_mant(htons(0x7123)), 0x123);
        // misc: resv 0xa, S 1, QRV 2.
        let misc = 0xa0 | 0x08 | 0x02;
        assert_eq!(
            (mld_qresv(misc), mld_sflag(misc), mld_qrv(misc)),
            (0xa, 1, 2)
        );
        assert_eq!((mld_qqic_exp(0x9c), mld_qqic_mant(0x9c)), (1, 0xc));
        assert_eq!(MLD_MINLEN, 8);
        assert_eq!(MLD_V2_QUERY_MINLEN, 28);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet6/mld6.h");
        crate::reftest::assert_defines!(defs;
        MLD_V2_REPORT_MAXRECS, MLD_DO_NOTHING, MLD_MODE_IS_INCLUDE, MLD_MODE_IS_EXCLUDE,
        MLD_CHANGE_TO_INCLUDE_MODE, MLD_CHANGE_TO_EXCLUDE_MODE, MLD_ALLOW_NEW_SOURCES,
        MLD_BLOCK_OLD_SOURCES, MLD_V2_GENERAL_QUERY, MLD_V2_GROUP_QUERY,
        MLD_V2_GROUP_SOURCE_QUERY, MLD_V1_MAX_RI, MLD_TIMER_SCALE);
    }

    /// An interface with a link-local address (which MLD reports come from) and the MLD state.
    /// Its MTU is 0: `ip6_output` drops (EMSGSIZE) what it is given, because `if_output_tso` has no
    /// AF_INET6 case yet (phase 2 of the INET6 port) and would panic.
    fn mld_if() -> (
        (MutexGuard<'static, ()>, MutexGuard<'static, ()>),
        &'static Ifnet,
    ) {
        let g = setup_ip();
        nd6_init();
        icmp6_init();
        let ifp = test_if(b"tml0");
        ifp.if_ioctl().set(Some(accepting_ioctl));
        nd6_ifattach(ifp);
        test_ia6(ifp, ll(ifp), 0);
        ifp.if_mtu.set(0);
        MLD6_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);
        (g, ifp)
    }

    /// `fe80::5054:ff:fe00:1` with the zone of `ifp` embedded.
    fn ll(ifp: &Ifnet) -> In6Addr {
        let mut a = a6("fe80::5054:ff:fe00:1");
        a.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        a
    }

    /// A group address with `ifp`'s zone embedded when it has a link-local scope.
    fn group(ifp: &Ifnet, s: &str) -> In6Addr {
        let mut g = a6(s);
        if in6_is_addr_mc_linklocal(&g) {
            g.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
        }
        g
    }

    /// `ifp` joins `g`.
    fn join(ifp: &'static Ifnet, g: In6Addr) -> &'static In6Multi {
        net_lock();
        let in6m = in6_addmulti(&g, ifp).expect("join");
        net_unlock();
        in6m
    }

    /// The count of MLD messages of `type_` sent.
    fn sent(type_: u8) -> u64 {
        ICMP6COUNTERS.read()[Icmp6statCounters::Icp6sOuthist as usize + usize::from(type_)]
    }

    fn tooshort() -> u64 {
        ICMP6COUNTERS.read()[Icmp6statCounters::Icp6sTooshort as usize]
    }

    /// A received MLD message `type_` from `src` for group `addr`, maximum response delay
    /// `maxdelay` (in milliseconds), on `ifp`.
    fn mld_message(
        ifp: &Ifnet,
        src: In6Addr,
        type_: u8,
        maxdelay: u16,
        addr: In6Addr,
    ) -> &'static Mbuf {
        let mut ip6 = Ip6Hdr::zeroed();
        ip6.set_ip6_vfc(IPV6_VERSION);
        ip6.ip6_plen = htons(size_of::<MldHdr>() as u16);
        ip6.ip6_nxt = IPPROTO_ICMPV6 as u8;
        ip6.ip6_hlim = 1;
        ip6.ip6_src = src;
        ip6.ip6_dst = IN6ADDR_LINKLOCAL_ALLNODES;
        let mut mldh = MldHdr {
            mld_icmp6_hdr: Icmp6Hdr::zeroed(),
            mld_addr: addr,
        };
        mldh.set_mld_type(type_);
        mldh.set_mld_maxdelay(htons(maxdelay));
        let mut b = std::vec![0u8; size_of::<Ip6Hdr>() + size_of::<MldHdr>()];
        // SAFETY: plain-data headers, no padding, copied into a buffer of their size.
        unsafe {
            ptr::copy_nonoverlapping(
                ptr::from_ref(&ip6).cast::<u8>(),
                b.as_mut_ptr(),
                size_of::<Ip6Hdr>(),
            );
            ptr::copy_nonoverlapping(
                ptr::from_ref(&mldh).cast::<u8>(),
                b.as_mut_ptr().add(size_of::<Ip6Hdr>()),
                size_of::<MldHdr>(),
            );
        }
        let m = test_packet(&b);
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        m
    }

    #[test]
    fn joining_a_group_starts_a_delayed_report() {
        let (_g, ifp) = mld_if();
        let reports = sent(MLD_LISTENER_REPORT);

        let g = group(ifp, "ff02::1234");
        let in6m = join(ifp, g);
        // The state says we reported last, the timer is a random delay of up to 10 seconds of
        // fast timeouts, and the first report went out with the join.
        assert_eq!(in6m.in6m_state.get(), MLD_IREPORTEDLAST);
        assert!((1..=MLD_V1_MAX_RI * PR_FASTHZ as u32).contains(&in6m.in6m_timer.get()));
        assert_eq!(MLD6_TIMERS_ARE_RUNNING.load(Ordering::Relaxed), 1);
        assert_eq!(sent(MLD_LISTENER_REPORT), reports + 1);

        // The all-nodes group, and the groups of interface-local and smaller scope, are never
        // reported.
        MLD6_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);
        for s in ["ff02::1", "ff01::5", "ff01::1"] {
            let in6m = join(ifp, group(ifp, s));
            assert_eq!(in6m.in6m_state.get(), MLD_OTHERLISTENER, "{s}");
            assert_eq!(in6m.in6m_timer.get(), 0, "{s}");
        }
        assert_eq!(sent(MLD_LISTENER_REPORT), reports + 1);
        assert_eq!(MLD6_TIMERS_ARE_RUNNING.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn leaving_sends_done_only_if_we_reported_last() {
        let (_g, ifp) = mld_if();
        let g = group(ifp, "ff02::1234");
        let in6m = join(ifp, g);
        let all = join(ifp, group(ifp, "ff02::1"));
        let mut all_routers = IN6ADDR_LINKLOCAL_ALLROUTERS;
        all_routers.set_s6_addr16(1, htons(ifp.if_index.get() as u16));

        crate::net::if_var::if_maddrlock_enter_write(ifp);
        // We reported last: a Done message to the all-routers group.
        let mut pkt = Mld6Pktinfo::default();
        mld6_stop_listening(in6m, ifp, &mut pkt);
        assert_eq!(
            pkt,
            Mld6Pktinfo {
                mpi_addr: all_routers,
                mpi_rdomain: ifp.if_rdomain.get(),
                mpi_ifidx: ifp.if_index.get(),
                mpi_type: i32::from(MLD_LISTENER_DONE),
            }
        );

        // Somebody else reported after us: nothing.
        in6m.in6m_state.set(MLD_OTHERLISTENER);
        let mut pkt = Mld6Pktinfo::default();
        mld6_stop_listening(in6m, ifp, &mut pkt);
        assert_eq!(pkt, Mld6Pktinfo::default());

        // The all-nodes group is never left loudly.
        all.in6m_state.set(MLD_IREPORTEDLAST);
        let mut pkt = Mld6Pktinfo::default();
        mld6_stop_listening(all, ifp, &mut pkt);
        assert_eq!(pkt, Mld6Pktinfo::default());
        crate::net::if_var::if_maddrlock_exit_write(ifp);
    }

    #[test]
    fn the_fast_timeout_counts_down_and_reports() {
        let (_g, ifp) = mld_if();
        let in6m = join(ifp, group(ifp, "ff02::1234"));
        let reports = sent(MLD_LISTENER_REPORT);

        // No timer running: nothing to do.
        in6m.in6m_timer.set(2);
        in6m.in6m_state.set(MLD_OTHERLISTENER);
        MLD6_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);
        mld6_fasttimo();
        assert_eq!(
            in6m.in6m_timer.get(),
            2,
            "the shortcut says no timer is running"
        );

        // Two ticks: the first only counts, the second reports.
        MLD6_TIMERS_ARE_RUNNING.store(1, Ordering::Relaxed);
        mld6_fasttimo();
        assert_eq!(in6m.in6m_timer.get(), 1);
        assert_eq!(
            MLD6_TIMERS_ARE_RUNNING.load(Ordering::Relaxed),
            1,
            "still running"
        );
        assert_eq!(sent(MLD_LISTENER_REPORT), reports);
        mld6_fasttimo();
        assert_eq!(in6m.in6m_timer.get(), 0);
        assert_eq!(in6m.in6m_state.get(), MLD_IREPORTEDLAST);
        assert_eq!(sent(MLD_LISTENER_REPORT), reports + 1);
        assert_eq!(
            MLD6_TIMERS_ARE_RUNNING.load(Ordering::Relaxed),
            0,
            "idle again"
        );
    }

    #[test]
    fn a_query_sets_the_timers_of_the_groups_it_asks_about() {
        let (_g, ifp) = mld_if();
        let a = join(ifp, group(ifp, "ff02::1234"));
        let b = join(ifp, group(ifp, "ff02::5678"));
        let all = join(ifp, group(ifp, "ff02::1"));
        let querier = ll(ifp);
        let reset = || {
            for x in [a, b] {
                x.in6m_timer.set(0);
                x.in6m_state.set(MLD_OTHERLISTENER);
            }
            MLD6_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);
        };
        let query = |maxdelay: u16, g: In6Addr| {
            input(
                mld_message(ifp, querier, MLD_LISTENER_QUERY, maxdelay, g),
                40,
            );
        };

        // A general query (group ::) with a 10 s maximum response delay: 50 fast ticks at most.
        reset();
        query(10_000, IN6ADDR_ANY);
        for x in [a, b] {
            assert!((1..=50).contains(&x.in6m_timer.get()));
        }
        assert_eq!(all.in6m_timer.get(), 0, "never the all-nodes group");
        assert_eq!(MLD6_TIMERS_ARE_RUNNING.load(Ordering::Relaxed), 1);

        // A running timer longer than the response delay is restarted with a shorter one; a
        // shorter one is kept.
        a.in6m_timer.set(40);
        b.in6m_timer.set(1);
        query(1_000, IN6ADDR_ANY);
        assert!((1..=5).contains(&a.in6m_timer.get()));
        assert_eq!(b.in6m_timer.get(), 1);

        // A group-specific query only touches that group. 100 ms is less than a tick: 1.
        reset();
        query(100, group(ifp, "ff02::5678"));
        assert_eq!((a.in6m_timer.get(), b.in6m_timer.get()), (0, 1));

        // A maximum response delay of 0 asks for an answer at once.
        reset();
        let reports = sent(MLD_LISTENER_REPORT);
        query(0, IN6ADDR_ANY);
        assert_eq!((a.in6m_timer.get(), b.in6m_timer.get()), (0, 0));
        assert_eq!(a.in6m_state.get(), MLD_IREPORTEDLAST);
        assert_eq!(sent(MLD_LISTENER_REPORT), reports + 2, "one per group");
        assert_eq!(all.in6m_state.get(), MLD_OTHERLISTENER);
    }

    #[test]
    fn queries_from_the_wrong_place_or_about_the_wrong_thing_are_ignored() {
        let (_g, ifp) = mld_if();
        let a = join(ifp, group(ifp, "ff02::1234"));
        a.in6m_timer.set(0);
        a.in6m_state.set(MLD_OTHERLISTENER);
        MLD6_TIMERS_ARE_RUNNING.store(0, Ordering::Relaxed);
        let untouched = |what: &str| {
            assert_eq!(a.in6m_timer.get(), 0, "{what}");
            assert_eq!(MLD6_TIMERS_ARE_RUNNING.load(Ordering::Relaxed), 0, "{what}");
        };

        // Not from a link-local address.
        input(
            mld_message(
                ifp,
                a6("fd00:77::2"),
                MLD_LISTENER_QUERY,
                10_000,
                IN6ADDR_ANY,
            ),
            40,
        );
        untouched("a global source");

        // About a unicast address.
        input(
            mld_message(ifp, ll(ifp), MLD_LISTENER_QUERY, 10_000, a6("fd00:77::2")),
            40,
        );
        untouched("a unicast group");

        // Shorter than an MLD header: counted, nothing else.
        let before = tooshort();
        let m = mld_message(ifp, ll(ifp), MLD_LISTENER_QUERY, 10_000, IN6ADDR_ANY);
        m.m_len().set(60);
        m.m_pkthdr().len.set(60);
        input(m, 40);
        assert_eq!(tooshort(), before + 1);
        untouched("a short message");
    }

    #[test]
    fn a_report_from_another_listener_stops_our_timer() {
        let (_g, ifp) = mld_if();
        let a = join(ifp, group(ifp, "ff02::1234"));
        let other = group(ifp, "ff02::9999");
        let src = a6("fe80::5054:ff:fe00:7");
        let report = |g: In6Addr| mld_message(ifp, src, MLD_LISTENER_REPORT, 0, g);

        // A report for our group: we stop reporting it (somebody else did).
        a.in6m_timer.set(30);
        a.in6m_state.set(MLD_IREPORTEDLAST);
        input(report(group(ifp, "ff02::1234")), 40);
        assert_eq!(
            (a.in6m_timer.get(), a.in6m_state.get()),
            (0, MLD_OTHERLISTENER)
        );

        // For a group we are not in: nothing; for a non-multicast address: nothing.
        a.in6m_timer.set(30);
        input(report(other), 40);
        input(report(a6("fd00:77::2")), 40);
        assert_eq!(a.in6m_timer.get(), 30);

        // A report that we looped back ourselves does not count.
        let m = report(group(ifp, "ff02::1234"));
        m.m_flags().set(m.m_flags().get() | M_LOOP);
        input(m, 40);
        assert_eq!(a.in6m_timer.get(), 30);
    }

    #[test]
    fn the_router_alert_option_is_ready_for_every_message() {
        // next header (filled in by ip6_output), length 0 (8 bytes), PadN of 2, Router Alert
        // option (type 5, length 2) with value 0: MLD.
        assert_eq!(
            MLD6_HBH[1..],
            [0, IP6OPT_PADN, 0, IP6OPT_ROUTER_ALERT, 2, 0, 0]
        );
    }

    /// A driver `ioctl` that accepts the multicast requests.
    ///
    /// # Safety
    ///
    /// `IfIoctlFn`'s contract.
    unsafe fn accepting_ioctl(_ifp: &'static Ifnet, cmd: u64, _data: *mut u8) -> Result<(), Errno> {
        use crate::sys::sockio::{SIOCADDMULTI, SIOCDELMULTI};
        match cmd {
            SIOCADDMULTI | SIOCDELMULTI => Ok(()),
            _ => Err(Errno::ENOTTY),
        }
    }
}
/* </TESTS> */
