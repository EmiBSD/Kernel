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
 *	@(#)ip_icmp.c	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! ICMPv6: `netinet6/icmp6.c`, the kernel's since N6.5d (`kernel::netinet6::icmp6`). This is
//! `bsd`'s adapter of it: the kernel's items under LZ's path for `bsd`'s importers (the
//! transports' control inputs, raw IPv6, pf), wrappers of the functions they call over parked
//! packets and routes (the net lock's proof made by check, `net_held`), and
//! `icmp6_ctloutput` over a socket, which forwards to the kernel's (N6.6b1, D-7: the raw
//! sockets' filter is the kernel pcb's words) until N6.6b1's switch deletes it with `bsd`'s
//! raw IPv6. `inet6sw[]`'s ICMPv6 entry is the kernel's
//! (`kernel::netinet6::in6_proto::Icmp6Protocol`).
//!
//! Upstream: sys/netinet6/icmp6.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `ICMP6_FILTER` get/set (`icmp6_ctloutput`) reaches the socket's control block through
//!   `sotoinpcb`, which must accept `AF_INET6` sockets (phase 2 of the INET6 port); a socket
//!   without one answers `EINVAL` where the C dereferences NULL.

use crate::net::if_::net_held;
use crate::net::route::Rtentry;
use crate::netinet::in_pcb::sotoinpcb;
use crate::netinet6::in6::SockaddrIn6;
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, mut_of, unpark};
use crate::sys::socketvar::Socket;

pub use kernel::netinet6::icmp6::{Ip6CtlParam, icmp6_mtudisc_callback_register};
pub use kernel::netinet6::ip6_var::ICMP6COUNTERS;

/// `icmp6_error(m, type, code, param)` of a parked packet (pf, UDP, raw IPv6).
pub fn icmp6_error(m: &'static Mbuf, type_: u8, code: u8, param: i32) {
    kernel::netinet6::icmp6::icmp6_error(
        unpark(m),
        type_,
        code,
        param,
        net_held("icmp6_error").net(),
    );
}

/// `icmp6_mtudisc_update(ip6cp, validated)`: the kernel's, for `bsd`'s IPv6 control inputs.
pub fn icmp6_mtudisc_update(ip6cp: &Ip6CtlParam<'_>, validated: bool) {
    kernel::netinet6::icmp6::icmp6_mtudisc_update(
        ip6cp,
        validated,
        net_held("icmp6_mtudisc_update").net(),
    );
}

/// `icmp6_mtudisc_clone(dst, rtableid, ipsec)`: the kernel's, the route parked.
pub fn icmp6_mtudisc_clone(
    dst: &SockaddrIn6,
    rtableid: u32,
    ipsec: bool,
) -> Option<&'static Rtentry> {
    kernel::netinet6::icmp6::icmp6_mtudisc_clone(
        dst,
        rtableid,
        ipsec,
        net_held("icmp6_mtudisc_clone").net(),
    )
    .map(crate::net::route::park)
}

/// `icmp6_ctloutput`: ICMPv6 socket option processing (`ICMP6_FILTER`): the kernel's, on the
/// socket's control block (`bsd`'s `rip6_ctloutput` sends the `IPPROTO_ICMPV6` level here).
pub fn icmp6_ctloutput(
    op: i32,
    so: &'static Socket,
    level: i32,
    optname: i32,
    m: Option<&'static Mbuf>,
) -> Result<(), Errno> {
    match sotoinpcb(so) {
        Some(inp) => {
            kernel::netinet6::icmp6::icmp6_ctloutput(op, inp, level, optname, m.map(mut_of))
        }
        None => Err(Errno::EINVAL),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The ICMPv6 filter socket option through `bsd`'s forward (LZ's
    // `the_icmp6_filter_is_a_socket_option`); the kernel's own tests are
    // `kernel::netinet6::raw_ip6`'s.

    use std::vec::Vec;

    use super::*;
    use crate::kern::uipc_mbuf::{m_copydata, m_freem, m_get};
    use crate::netinet::icmp6::{ICMP6_ECHO_REPLY, ICMP6_ECHO_REQUEST, Icmp6Filter};
    use crate::netinet::in_::IPPROTO_ICMPV6;
    use crate::netinet6::in6::ICMP6_FILTER;
    use crate::sys::mbuf::{M_DONTWAIT, MT_DATA};
    use crate::sys::protosw::{PRCO_GETOPT, PRCO_SETOPT};

    fn bytes_of(m: &Mbuf) -> Vec<u8> {
        let mut v = std::vec![0u8; m.m_len().get() as usize];
        m_copydata(m, 0, &mut v);
        v
    }

    fn bytes_of_filter(f: &Icmp6Filter) -> Vec<u8> {
        f.icmp6_filt.iter().flat_map(|w| w.to_ne_bytes()).collect()
    }

    #[test]
    fn the_icmp6_filter_is_a_socket_option() {
        use crate::kern::uipc_socket::{soclose, socreate};
        use crate::sys::socket::SOCK_RAW;
        let (_g, _t, _p) = crate::netinet::in_pcb::tests::setup();
        crate::netinet::raw_ip::rip_init();
        // Raw sockets of the inet domain stand in for raw ICMPv6 sockets (no `AF_INET6`
        // socket is possible before `rip6_attach`): the option code only needs the pcb. The
        // first gets the filter `rip6_attach` makes, the second has none.
        let so = socreate(i32::from(crate::sys::socket::AF_INET), SOCK_RAW, 1).expect("socket");
        let inp = sotoinpcb(so).expect("pcb");
        inp.icmp6filt_init();
        let bare = socreate(i32::from(crate::sys::socket::AF_INET), SOCK_RAW, 1).expect("socket");

        // Wrong level, wrong option.
        assert_eq!(
            icmp6_ctloutput(PRCO_SETOPT, so, 0, ICMP6_FILTER, None),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            icmp6_ctloutput(PRCO_SETOPT, so, IPPROTO_ICMPV6, 99, None),
            Err(Errno::ENOPROTOOPT)
        );
        // The wrong size of filter.
        let m = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        m.m_len().set(8);
        assert_eq!(
            icmp6_ctloutput(PRCO_SETOPT, so, IPPROTO_ICMPV6, ICMP6_FILTER, Some(m)),
            Err(Errno::EMSGSIZE)
        );

        // Set a filter that blocks everything but echo replies, and read it back.
        let mut want = Icmp6Filter { icmp6_filt: [0; 8] };
        want.setblockall();
        want.setpass(ICMP6_ECHO_REPLY);
        let wb = bytes_of_filter(&want);
        m.m_len().set(wb.len() as u32);
        mut_of(m)
            .data_mut()
            .expect("a writable mbuf")
            .copy_from_slice(&wb);
        icmp6_ctloutput(PRCO_SETOPT, so, IPPROTO_ICMPV6, ICMP6_FILTER, Some(m)).expect("set");
        let words = inp.icmp6filt().expect("a filter");
        let got = Icmp6Filter {
            icmp6_filt: core::array::from_fn(|i| {
                words[i].load(core::sync::atomic::Ordering::Relaxed)
            }),
        };
        assert!(got.willpass(ICMP6_ECHO_REPLY) && got.willblock(ICMP6_ECHO_REQUEST));

        let out = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        icmp6_ctloutput(PRCO_GETOPT, so, IPPROTO_ICMPV6, ICMP6_FILTER, Some(out)).expect("get");
        assert_eq!(bytes_of(out), wb);

        // No filter, no option.
        assert_eq!(
            icmp6_ctloutput(PRCO_GETOPT, bare, IPPROTO_ICMPV6, ICMP6_FILTER, Some(out)),
            Err(Errno::EINVAL)
        );
        m_freem(m);
        m_freem(out);
        soclose(so, 0).expect("close");
        soclose(bare, 0).expect("close");
        crate::netinet::in_pcb::tests::teardown();
    }
}
/* </TESTS> */
