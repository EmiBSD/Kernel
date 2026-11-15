/*	$KAME: icmp6.c,v 1.217 2001/06/20 15:03:29 jinmei Exp $	*/
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
//! transports' control inputs, raw IPv6, pf, the input path until it moves), wrappers of the
//! functions they call over parked packets and routes (the net lock's proof made by check,
//! `net_held`), and `icmp6_ctloutput`, which stays here with the raw sockets' filter word it
//! reads and writes (`inp_icmp6filt`, `bsd`'s until N6.6b).
//!
//! Upstream: sys/netinet6/icmp6.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `ICMP6_FILTER` get/set (`icmp6_ctloutput`) reaches the socket's control block through
//!   `sotoinpcb`, which must accept `AF_INET6` sockets (phase 2 of the INET6 port).

use core::mem::size_of;
use core::ptr;

use crate::net::if_::net_held;
use crate::net::if_var::Netstack;
use crate::net::route::Rtentry;
use crate::netinet::icmp6::Icmp6Filter;
use crate::netinet::in_::IPPROTO_ICMPV6;
use crate::netinet::in_pcb::InpcbLegacy as _;
use crate::netinet::in_pcb::sotoinpcb;
use crate::netinet6::in6::{ICMP6_FILTER, SockaddrIn6};
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, mtod, park, unpark};
use crate::sys::protosw::{PRCO_GETOPT, PRCO_SETOPT};
use crate::sys::socketvar::Socket;

pub use kernel::netinet6::icmp6::{
    ICMP6_MTUDISC_HIWAT, ICMP6_MTUDISC_LOWAT, ICMP6_MTUDISC_TIMEOUT_Q, ICMP6_REDIRECT_TIMEOUT_Q,
    Ip6CtlParam, icmp6_errcount, icmp6_mtudisc_callback_register, icmp6_ratelimit,
};
pub use kernel::netinet6::ip6_var::ICMP6COUNTERS;

/// `void (*)(struct sockaddr_in6 *, u_int)`: a path MTU change callback (the kernel's
/// `Mtudisc6Fn`).
pub type Icmp6MtudiscCallbackFn = kernel::netinet6::icmp6::Mtudisc6Fn;

/// `icmp6_error(m, type, code, param)` of a parked packet (pf, UDP, raw IPv6, the input path).
pub fn icmp6_error(m: &'static Mbuf, type_: u8, code: u8, param: i32) {
    kernel::netinet6::icmp6::icmp6_error(unpark(m), type_, code, param, &net_held("icmp6_error"));
}

/// `icmp6_mtudisc_update(ip6cp, validated)`: the kernel's, for `bsd`'s IPv6 control inputs.
pub fn icmp6_mtudisc_update(ip6cp: &Ip6CtlParam<'_>, validated: bool) {
    kernel::netinet6::icmp6::icmp6_mtudisc_update(
        ip6cp,
        validated,
        &net_held("icmp6_mtudisc_update"),
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
        &net_held("icmp6_mtudisc_clone"),
    )
    .map(crate::net::route::park)
}

/// `icmp6_redirect_output(m0, rt)`: the kernel's, for `bsd`'s forwarding.
pub fn icmp6_redirect_output(m0: &'static Mbuf, rt: &'static Rtentry) {
    kernel::netinet6::icmp6::icmp6_redirect_output(
        unpark(m0),
        rt,
        &net_held("icmp6_redirect_output"),
    );
}

/// `icmp6_init`: the kernel's (`inet6sw[]`'s row until the table is the kernel's).
pub fn icmp6_init() {
    kernel::netinet6::icmp6::icmp6_init();
}

/// `icmp6_fasttimo`: the kernel's.
pub fn icmp6_fasttimo() {
    kernel::netinet6::icmp6::icmp6_fasttimo();
}

/// `icmp6_input(mp, offp, proto, af, ns)`: the kernel's over the parked packet.
pub fn icmp6_input(
    mp: &mut Option<&'static Mbuf>,
    offp: &mut i32,
    proto: i32,
    af: i32,
    ns: Option<&mut Netstack>,
) -> i32 {
    let mut k = mp.take().map(unpark);
    let nxt = kernel::netinet6::icmp6::icmp6_input::<crate::kern::proc_glue::BsdProcGlue>(
        &mut k,
        offp,
        proto,
        af,
        ns,
        &net_held("icmp6_input"),
    );
    *mp = k.map(park);
    nxt
}

/// `icmp6_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the kernel's.
pub fn icmp6_sysctl(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let mut req = kernel::sysctl::SysctlReq {
        oldp,
        oldlenp,
        newp,
        newlen,
    };
    kernel::netinet6::icmp6::icmp6_sysctl(name, &mut req)
}

/// `icmp6_ctloutput`: ICMPv6 socket option processing (`ICMP6_FILTER`).
pub fn icmp6_ctloutput(
    op: i32,
    so: &'static Socket,
    level: i32,
    optname: i32,
    m: Option<&'static Mbuf>,
) -> Result<(), Errno> {
    let inp = sotoinpcb(so);

    if level != IPPROTO_ICMPV6 {
        return Err(Errno::EINVAL);
    }

    match op {
        PRCO_SETOPT => match optname {
            ICMP6_FILTER => {
                let Some(m) = m else {
                    return Err(Errno::EMSGSIZE);
                };
                if m.m_len().get() as usize != size_of::<Icmp6Filter>() {
                    return Err(Errno::EMSGSIZE);
                }
                let Some(filt) = inp.and_then(|inp| inp.icmp6filt_ptr()) else {
                    return Err(Errno::EINVAL);
                };
                // SAFETY: the mbuf holds a whole filter at its data; the pcb's filter is the
                // `Icmp6Filter` allocated at attach time, owned by the socket.
                unsafe {
                    ptr::copy_nonoverlapping(
                        mtod::<u8>(m).cast_const(),
                        filt.as_ptr().cast::<u8>(),
                        size_of::<Icmp6Filter>(),
                    )
                };
                Ok(())
            }
            _ => Err(Errno::ENOPROTOOPT),
        },
        PRCO_GETOPT => match optname {
            ICMP6_FILTER => {
                let Some(filt) = inp.and_then(|inp| inp.icmp6filt_ptr()) else {
                    return Err(Errno::EINVAL);
                };
                let Some(m) = m else {
                    return Err(Errno::EINVAL);
                };
                m.m_len().set(size_of::<Icmp6Filter>() as u32);
                // SAFETY: the pcb's filter is a whole `Icmp6Filter`; the reply mbuf has room
                // for it (`sogetopt` provides an mbuf with MLEN bytes).
                unsafe {
                    ptr::copy_nonoverlapping(
                        filt.as_ptr().cast::<u8>().cast_const(),
                        mtod::<u8>(m),
                        size_of::<Icmp6Filter>(),
                    )
                };
                Ok(())
            }
            _ => Err(Errno::ENOPROTOOPT),
        },
        _ => Ok(()),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The ICMPv6 filter socket option, which stays here with its word (LZ's
    // `the_icmp6_filter_is_a_socket_option`); ICMPv6's other tests are the kernel's.

    use std::ptr::NonNull;
    use std::vec::Vec;

    use super::*;
    use crate::kern::uipc_mbuf::{m_copydata, m_freem, m_get};
    use crate::netinet::icmp6::ICMP6_ECHO_REPLY;
    use crate::netinet::icmp6::ICMP6_ECHO_REQUEST;
    use crate::sys::mbuf::{M_DONTWAIT, MT_DATA};

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
        // A raw socket of the inet domain stands in for a raw ICMPv6 socket (no `AF_INET6`
        // socket is possible before `rip6_attach`): the option code only needs the pcb.
        let so = socreate(i32::from(crate::sys::socket::AF_INET), SOCK_RAW, 1).expect("socket");
        let inp = sotoinpcb(so).expect("pcb");
        let filt: &'static mut Icmp6Filter =
            std::boxed::Box::leak(std::boxed::Box::new(Icmp6Filter { icmp6_filt: [0; 8] }));
        inp.set_icmp6filt_ptr(NonNull::new(filt));

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
        m.m_len().set(size_of::<Icmp6Filter>() as u32);
        // SAFETY: an mbuf holds MLEN bytes, more than a filter.
        unsafe {
            ptr::copy_nonoverlapping(
                ptr::from_ref(&want).cast::<u8>(),
                mtod::<u8>(m),
                size_of::<Icmp6Filter>(),
            )
        };
        icmp6_ctloutput(PRCO_SETOPT, so, IPPROTO_ICMPV6, ICMP6_FILTER, Some(m)).expect("set");
        assert!(filt.willpass(ICMP6_ECHO_REPLY) && filt.willblock(ICMP6_ECHO_REQUEST));

        let out = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        icmp6_ctloutput(PRCO_GETOPT, so, IPPROTO_ICMPV6, ICMP6_FILTER, Some(out)).expect("get");
        assert_eq!(out.m_len().get() as usize, size_of::<Icmp6Filter>());
        assert_eq!(bytes_of(out), bytes_of_filter(&want));

        // No filter, no option.
        inp.set_icmp6filt_ptr(None);
        assert_eq!(
            icmp6_ctloutput(PRCO_GETOPT, so, IPPROTO_ICMPV6, ICMP6_FILTER, Some(out)),
            Err(Errno::EINVAL)
        );
        m_freem(m);
        m_freem(out);
        soclose(so, 0).expect("close");
        crate::netinet::in_pcb::tests::teardown();
    }
}
/* </TESTS> */
