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
//! transports' control inputs, pf), and wrappers of the functions they call over parked
//! packets and routes (the net lock's proof made by check, `net_held`). `inet6sw[]`'s ICMPv6
//! entry is the kernel's (`kernel::netinet6::in6_proto::Icmp6Protocol`), and so is
//! `icmp6_ctloutput` since N6.6b1 (`kernel::netinet6::icmp6`, called by the kernel's
//! `rip6_ctloutput`; the raw sockets' filter is the kernel pcb's words, D-7).
//!
//! Upstream: sys/netinet6/icmp6.c @ 3ce1f3f79392

use crate::net::if_::net_held;
use crate::net::route::Rtentry;
use crate::netinet6::in6::SockaddrIn6;
use crate::sys::mbuf::{Mbuf, unpark};

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
/* </CODE> */
