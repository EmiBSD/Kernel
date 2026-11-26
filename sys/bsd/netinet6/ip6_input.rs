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
 *	@(#)ip_input.c	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 input: `netinet6/ip6_input.c`, the kernel's since N6.5d
//! (`kernel::netinet6::ip6_input`, with forwarding, `kernel::netinet6::ip6_forward`). This is
//! `bsd`'s adapter of it: the kernel's items under LZ's path for `bsd`'s importers (the
//! transports' control inputs and their control messages, raw IPv6, IP-in-IP, divert, wg(4),
//! pf), and wrappers of the functions they call over parked packets (the net lock's proof made
//! by check, `net_held`).
//!
//! Upstream: sys/netinet6/ip6_input.c @ 3ce1f3f79392

use crate::kern::proc_glue::BsdProcGlue as G;
use crate::net::if_::net_held;
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::route::Route;
use crate::sys::mbuf::{Mbuf, park, unpark};

pub use kernel::netinet6::ip6_input::{INET6CTLERRMAP, ip6_get_prevhdr, ip6_lasthdr};
pub use kernel::netinet6::ip6_var::IP6COUNTERS;

/// `ipv6_input(ifp, m, ns)`: the kernel's, of a parked packet (wg(4), divert).
pub fn ipv6_input(ifp: &'static Ifnet, m: &'static Mbuf, ns: Option<&mut Netstack>) {
    kernel::netinet6::ip6_input::ipv6_input::<G>(ifp, unpark(m), ns, net_held("ipv6_input").net());
}

/// `ip6_input_if(mp, offp, nxt, af, ifp, ns)`: the kernel's, over the parked packet
/// (IP-in-IP's inner IPv6 packet).
pub fn ip6_input_if(
    mp: &mut Option<&'static Mbuf>,
    offp: &mut i32,
    nxt: i32,
    af: i32,
    ifp: &'static Ifnet,
    ns: Option<&mut Netstack>,
) -> i32 {
    let mut k = mp.take().map(unpark);
    let nxt = kernel::netinet6::ip6_input::ip6_input_if::<G>(
        &mut k,
        offp,
        nxt,
        af,
        ifp,
        ns,
        net_held("ip6_input_if").net(),
    );
    *mp = k.map(park);
    nxt
}

/// `ip6_forward(m, ro, flags)`: the kernel's, of a parked packet (pf's route-to and
/// reassembly).
pub fn ip6_forward(m: &'static Mbuf, ro: Option<&mut Route>, flags: i32) {
    kernel::netinet6::ip6_forward::ip6_forward(unpark(m), ro, flags, net_held("ip6_forward").net());
}

/// `ip6_send(m)`: the kernel's, of a parked packet.
pub fn ip6_send(m: &'static Mbuf) {
    kernel::netinet6::ip6_input::ip6_send(unpark(m));
}
/* </CODE> */
