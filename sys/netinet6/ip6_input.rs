/*	$KAME: ip6_input.c,v 1.188 2001/03/29 05:34:31 itojun Exp $	*/
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
use crate::kern::uipc_domain::pffindproto;
use crate::net::if_::net_held;
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::route::Route;
use crate::netinet::in_::{IPPROTO_MAX, IPPROTO_RAW};
use crate::netinet::in_pcb::Inpcb;
use crate::netinet6::in6_proto::{INET6DOMAIN, INET6SW, IP6_PROTOX};
use crate::netinet6::ip6_output::ip6_randomid_init;
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, park, unpark};
use crate::sys::protosw::Protosw;
use crate::sys::socket::{PF_INET6, SOCK_RAW};
use core::ptr;
use core::sync::atomic::Ordering;

pub use kernel::netinet6::ip6_input::{INET6CTLERRMAP, ip6_get_prevhdr, ip6_lasthdr};
pub use kernel::netinet6::ip6_var::IP6COUNTERS;

/// The index of `pr` in `inet6sw[]` (`pr - inet6sw`).
fn inet6sw_index(pr: &Protosw) -> u8 {
    match INET6SW.iter().position(|p| ptr::eq(p, pr)) {
        Some(i) => i as u8,
        None => crate::kern::subr_prf::panic(format_args!("ip6_init: protocol not in inet6sw")),
    }
}

/// `ip6_init`: `inet6sw[]`'s IPV6 row until the table is the kernel's: `ip6_protox[]` filled
/// from `inet6sw[]` (all protocols not implemented in kernel go to the raw IP6 protocol
/// handler), the fragment identifiers' generator, and the kernel's `ip6_init`.
pub fn ip6_init() {
    let Some(pr) = pffindproto(i32::from(PF_INET6), IPPROTO_RAW, SOCK_RAW) else {
        crate::kern::subr_prf::panic(format_args!("ip6_init"));
    };
    let raw = inet6sw_index(pr);
    for p in &IP6_PROTOX {
        p.store(raw, Ordering::Relaxed);
    }
    for pr in INET6DOMAIN.dom_protosw {
        if pr.pr_domain.dom_family == i32::from(PF_INET6)
            && pr.pr_protocol != 0
            && i32::from(pr.pr_protocol) != IPPROTO_RAW
            && i32::from(pr.pr_protocol) < IPPROTO_MAX
        {
            IP6_PROTOX[pr.pr_protocol as usize].store(inet6sw_index(pr), Ordering::Relaxed);
        }
    }
    ip6_randomid_init();
    kernel::netinet6::ip6_input::ip6_init::<G>();
}

/// `ipv6_input(ifp, m, ns)`: the kernel's, of a parked packet (wg(4), divert).
pub fn ipv6_input(ifp: &'static Ifnet, m: &'static Mbuf, ns: Option<&mut Netstack>) {
    kernel::netinet6::ip6_input::ipv6_input::<G>(ifp, unpark(m), ns, &net_held("ipv6_input"));
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
        &net_held("ip6_input_if"),
    );
    *mp = k.map(park);
    nxt
}

/// `ip6_savecontrol(inp, m, mp)`: the kernel's; `*mp` the control chain it made, parked.
pub fn ip6_savecontrol(inp: &Inpcb, m: &Mbuf, mp: &mut Option<&'static Mbuf>) {
    *mp = kernel::netinet6::ip6_input::ip6_savecontrol::<G>(inp, m).map(park);
}

/// `ip6_forward(m, ro, flags)`: the kernel's, of a parked packet (pf's route-to and
/// reassembly).
pub fn ip6_forward(m: &'static Mbuf, ro: Option<&mut Route>, flags: i32) {
    kernel::netinet6::ip6_forward::ip6_forward(unpark(m), ro, flags, &net_held("ip6_forward"));
}

/// `ip6_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the kernel's (`inet6sw[]`'s IPV6
/// row until the table is the kernel's).
pub fn ip6_sysctl(
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
    kernel::netinet6::ip6_input::ip6_sysctl(name, &mut req)
}

/// `ip6_send(m)`: the kernel's, of a parked packet.
pub fn ip6_send(m: &'static Mbuf) {
    kernel::netinet6::ip6_input::ip6_send(unpark(m));
}
/* </CODE> */
