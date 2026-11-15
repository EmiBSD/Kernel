/*	$KAME: nd6.h,v 1.95 2002/06/08 11:31:06 itojun Exp $	*/
/*	$KAME: nd6.c,v 1.280 2002/06/08 19:52:07 itojun Exp $	*/
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
//! IPv6 Neighbor Discovery: `<netinet6/nd6.h>` (`hal::sys::netinet6_nd6`) and `netinet6/nd6.c`,
//! the kernel's since N6.5b (`kernel::netinet6::nd6`), re-exported whole for `bsd`'s importers;
//! the neighbor fixtures `bsd`'s tests share (`if_ethersubr`'s, ICMPv6's, the extension
//! headers') stay here over `bsd`'s Ethernet fixture.
//!
//! Upstream: sys/netinet6/nd6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/nd6.c @ 3ce1f3f79392

pub use hal::sys::netinet6_nd6::*;
pub use kernel::netinet6::nd6::*;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // The fixtures `bsd`'s IPv6 tests share (an Ethernet interface that is up, with its ND
    // information; a neighbor's host route given its cache entry by the kernel's
    // `nd6_rtrequest`; the packets the interface was given), and the constants' check against
    // the C header. ND's own tests are the kernel's (`kernel::netinet6::nd6`).

    use std::sync::MutexGuard;
    use std::vec::Vec;

    use super::*;

    use crate::kern::uipc_mbuf::m_freem;
    use crate::net::if_::{IFF_RUNNING, IFF_UP};
    use crate::net::if_dl::SockaddrDl;
    use crate::net::if_var::{Ifnet, IfnetLegacy};
    use crate::net::route::{RTF_LLINFO, Rtentry};
    use crate::netinet::ip_input::tests::{bytes, setup, test_ether};
    use crate::netinet6::in6::{In6Addr, SockaddrIn6};
    use crate::netinet6::in6_var::In6Ifaddr;
    use crate::sys::errno::Errno;
    use crate::sys::mbuf::Mbuf;
    use crate::sys::socket::{AF_LINK, Sockaddr};
    use crate::sys::systm::{net_lock, net_unlock};

    /// `fd00:77::1`, our address in the tests.
    pub(crate) const OURS6: In6Addr =
        In6Addr::new([0xfd, 0, 0, 0x77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    /// `fd00:77::2`, the neighbor's.
    pub(crate) const PEER6: In6Addr =
        In6Addr::new([0xfd, 0, 0, 0x77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);

    /// The packets [`capture_output`] took: the destination's family and the bytes.
    pub(crate) static SENT: std::sync::Mutex<Vec<(u8, Vec<u8>)>> =
        std::sync::Mutex::new(Vec::new());

    /// An `if_output` that keeps what it is given in [`SENT`].
    ///
    /// # Safety
    ///
    /// `IfOutputFn`'s contract.
    unsafe fn capture_output(
        _ifp: &'static Ifnet,
        m: &'static Mbuf,
        dst: *const Sockaddr,
        _rt: Option<&'static Rtentry>,
    ) -> Result<(), Errno> {
        // SAFETY: the caller's contract.
        let af = unsafe { (*dst).sa_family };
        SENT.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((af, bytes(m)));
        m_freem(m);
        Ok(())
    }

    /// Takes the packets sent so far.
    pub(crate) fn take_sent() -> Vec<(u8, Vec<u8>)> {
        core::mem::take(&mut *SENT.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// The network setup of the IPv4 tests, the ND state, and an Ethernet interface that is
    /// up, with its ND information.
    pub(crate) fn nd6_setup() -> (
        (MutexGuard<'static, ()>, MutexGuard<'static, ()>),
        &'static Ifnet,
    ) {
        let g = setup();
        nd6_init();
        let ifp = test_ether();
        ifp.if_flags.set(ifp.if_flags.get() | IFF_UP | IFF_RUNNING);
        nd6_ifattach(ifp);
        ifp.if_output().set(Some(capture_output));
        // `ip6_output` drops (EMSGSIZE) a packet for an MTU below IPV6_MMTU: the solicitations
        // the tests trigger stop there.
        ifp.if_mtu.set(0);
        let _ = take_sent();
        (g, ifp)
    }

    /// An inet6 address `addr` of `ifp` (not on its list), referenced once: the kernel's
    /// record (`in6_ifaddr_alloc`).
    pub(crate) fn fake_ifa6(ifp: &'static Ifnet, addr: In6Addr) -> &'static In6Ifaddr {
        let ia = crate::netinet6::in6_var::in6_ifaddr_alloc(ifp, 0);
        ia.ia_addr.set(SockaddrIn6::with_addr(addr));
        ia
    }

    /// A host route to `dst` on `ifp` with an empty link-layer gateway, added to table 0 (the
    /// kernel's `nd6_rtrequest` gives it its neighbor cache entry) and left as `rtrequest`'s
    /// clone of an on-link prefix leaves it (`RTM_RESOLVE`: NOSTATE, its timer due at once;
    /// `RTM_ADD`, an ndp(8) entry, would be REACHABLE). Its reference is never dropped.
    pub(crate) fn fake_neighbor(ifp: &'static Ifnet, dst: In6Addr) -> &'static Rtentry {
        use crate::net::route::{
            RTAX_DST, RTAX_GATEWAY, RTF_HOST, RTM_ADD, RtAddrinfo, rtrequest, sa_of, sa_view,
        };

        let key = SockaddrIn6::with_addr(dst);
        let gate = SockaddrDl {
            sdl_len: core::mem::size_of::<SockaddrDl>() as u8,
            sdl_family: AF_LINK,
            ..SockaddrDl::default()
        };
        let mut info = RtAddrinfo::new();
        info.rti_ifa = Some(&fake_ifa6(ifp, OURS6).ia_ifa);
        info.rti_flags = RTF_HOST;
        info.rti_info[RTAX_DST] = Some(sa_view(sa_of(&key)));
        info.rti_info[RTAX_GATEWAY] = Some(sa_view(sa_of(&gate)));
        let mut rt = None;

        net_lock();
        rtrequest(RTM_ADD, &mut info, 0, Some(&mut rt), ifp.if_rdomain.get())
            .expect("the neighbor's route");
        let rt = rt.expect("route");
        nd6_entry_set_for_tests(rt, ND6_LLINFO_NOSTATE, Some(0));
        net_unlock();
        assert_ne!(rt.rt_flags.get() & RTF_LLINFO, 0);
        assert!(nd6_entry(rt).is_some(), "nd6_rtrequest's entry");
        rt
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet6/nd6.h");
        let ll = crate::reftest::assert_defines!(defs;
        ND6_LLINFO_PURGE, ND6_LLINFO_NOSTATE, ND6_LLINFO_INCOMPLETE, ND6_LLINFO_REACHABLE,
        ND6_LLINFO_STALE, ND6_LLINFO_DELAY, ND6_LLINFO_PROBE, ND6_INFINITE_LIFETIME);
        crate::reftest::assert_complete(&defs, "ND6_", &ll);
        crate::reftest::assert_defines!(defs;
        MAX_RTR_SOLICITATION_DELAY, RTR_SOLICITATION_INTERVAL, MAX_RTR_SOLICITATIONS,
        LN_HOLD_QUEUE, LN_HOLD_TOTAL, REACHABLE_TIME, RETRANS_TIMER, MIN_RANDOM_FACTOR,
        MAX_RANDOM_FACTOR);
    }

    #[test]
    fn the_kernel_s_malloc_type_is_the_header_s() {
        assert_eq!(M_IP6NDP, crate::sys::malloc::M_IP6NDP);
    }

    #[test]
    fn a_fake_neighbor_has_its_entry_and_the_interface_its_nd_information() {
        let (_g, ifp) = nd6_setup();
        let rt = fake_neighbor(ifp, PEER6);
        let (state, asked, router, held, _) = nd6_entry(rt).expect("the entry");
        assert_eq!((state, asked, router, held), (ND6_LLINFO_NOSTATE, 0, 0, 0));
        let reachable = ifp.if_nd.get().reachable;
        assert!((14..=44).contains(&reachable), "ND_COMPUTE_RTIME(30000)");
    }
}
/* </TESTS> */
