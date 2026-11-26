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
/* </LICENSES> */

/* <CODE> */
//! Routing header processing: `netinet6/route6.c`. [`route6_input`] is the routing header's
//! `pr_input`: a header with no segments left is skipped (the packet is at its final
//! destination); one with segments left is refused with an ICMPv6 parameter problem at its
//! routing type, whatever the type: type 0 is handled as an unknown type (RFC 5095), and no
//! other is implemented.
//!
//! Upstream: sys/netinet6/route6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/route6.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's read of the header through the raw pointer `ip6_exthdr_get` returned (one `unsafe`)
//!   -> a `pod` copy of its four bytes from the mbuf that holds them
//!   (`ip6_input::ip6_exthdr_bytes`).
//! - The parameter problem's pointer is the routing type's offset from the IPv6 header,
//!   `*offp` plus its place in the header (the C's `&rh->ip6r_type - ip6`, which is that when
//!   the header shares the first mbuf, as it does after `ip6_exthdr_get` of a short header in
//!   a pulled-up packet).
//!
//! **Lock and spl transitions** against the C: none (the C takes none; the caller holds the
//! net lock).

use core::mem::{offset_of, size_of};

use hal::mem::pod;
use hal::sync::netlock::NetHeld;
use sysdefs::netinet_icmp6::{ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER};
use sysdefs::netinet_in::IPPROTO_DONE;
use sysdefs::netinet_ip6::Ip6Rthdr;
use sysdefs::netinet6_ip6_var::Ip6statCounters;

use crate::net::if_var::Netstack;
use crate::net::mbuf::MbufBox;
use crate::netinet6::icmp6::icmp6_error;
use crate::netinet6::ip6_input::{ip6_exthdr_bytes, ip6_exthdr_get};
use crate::netinet6::ip6_var::ip6stat_inc;

/// `route6_input(mp, offp, proto, af, ns)`: the routing header's input; the next header with
/// `*offp` past the header, or `IPPROTO_DONE` (`*mp` cleared) when it was refused. `proto` is
/// unused.
pub fn route6_input(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    _proto: i32,
    _af: i32,
    _ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    const LEN: usize = size_of::<Ip6Rthdr>();
    let Some(m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let off = *offp;

    let Some((m, at, o)) = ip6_exthdr_get(m, off, LEN as i32) else {
        ip6stat_inc(Ip6statCounters::Ip6sTooshort);
        return IPPROTO_DONE;
    };
    let Some(rh) = ip6_exthdr_bytes(&m, at, o, LEN).and_then(|b| pod::at::<Ip6Rthdr>(b, 0)) else {
        return IPPROTO_DONE;
    };

    // IPV6_RTHDR_TYPE_0: RFC 5095 specifies to handle routing header type 0 the same way as
    // an unrecognised routing type. default: unknown routing type.
    if rh.ip6r_segleft != 0 {
        ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
        icmp6_error(
            m,
            ICMP6_PARAM_PROB,
            ICMP6_PARAMPROB_HEADER,
            off + offset_of!(Ip6Rthdr, ip6r_type) as i32,
            net,
        );
        return IPPROTO_DONE;
    }
    // Final dst. Just ignore the header.
    let rhlen = (i32::from(rh.ip6r_len) + 1) << 3;

    *offp += rhlen;
    *mp = Some(m);
    i32::from(rh.ip6r_nxt)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the routing header input over kernel packets: LZ's (a final destination's
    // header skipped, a header with segments left refused at its type) and the checks one by
    // one: every routing type with segments left (type 0 as unknown, RFC 5095), a short
    // header.

    use hal::sync::netlock::NETLOCK;
    use sysdefs::netinet_in::IPPROTO_UDP;
    use sysdefs::netinet_ip6::{IPV6_VERSION, Ip6Hdr};
    use sysdefs::netinet6_in6::In6Addr;

    use super::*;
    use crate::net::mbuf::m_devget;
    use crate::net::route::tests::{take_icmp6_errors, take_icmp6_params, testroutes_setup};
    use crate::netinet6::ip6_var::IP6COUNTERS;

    fn stat(c: Ip6statCounters) -> u64 {
        IP6COUNTERS.read()[c as usize]
    }

    fn ula(last: u8) -> In6Addr {
        let mut a = [0u8; 16];
        a[0] = 0xfd;
        a[15] = last;
        In6Addr { s6_addr: a }
    }

    /// `route6_input` of a packet whose header is followed by the routing header `rh` at
    /// offset `at` (`at - 40` bytes of destination options before it), under the net lock.
    fn input_at(at: usize, rh: &[u8]) -> (i32, i32, bool) {
        let mut ext = std::vec![0u8; at - 40];
        if at > 40 {
            ext[0] = 43;
            ext[1] = ((at - 40) / 8 - 1) as u8;
        }
        ext.extend_from_slice(rh);
        let mut ip6 = Ip6Hdr {
            ip6_plen: (ext.len() as u16).to_be(),
            ip6_nxt: if at > 40 { 60 } else { 43 },
            ip6_hlim: 64,
            ip6_src: ula(2),
            ip6_dst: ula(1),
            ..Ip6Hdr::default()
        };
        ip6.set_ip6_vfc(IPV6_VERSION);
        let mut b = std::vec![0u8; 40];
        assert!(hal::mem::pod::put(&mut b, 0, &ip6));
        b.extend_from_slice(&ext);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut mp = Some(m_devget(&b, 0).expect("a packet"));
        let mut off = at as i32;
        let nxt = route6_input(&mut mp, &mut off, 43, 24, None, &net);
        (nxt, off, mp.is_some())
    }

    #[test]
    fn a_final_destination_skips_the_header() {
        let _g = testroutes_setup();
        let _ = (take_icmp6_errors(), take_icmp6_params());
        // segleft 0 of any type, type 0 among them: skipped by its length.
        for ty in [0u8, 2, 4, 253] {
            let rh = [
                IPPROTO_UDP as u8,
                1,
                ty,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ];
            assert_eq!(input_at(40, &rh), (IPPROTO_UDP, 56, true), "type {ty}");
        }
        assert!(take_icmp6_errors().is_empty());
    }

    #[test]
    fn segments_left_are_refused_at_the_routing_type() {
        let _g = testroutes_setup();
        let _ = (take_icmp6_errors(), take_icmp6_params());
        for (at, ty) in [(40, 0u8), (40, 2), (48, 0), (48, 4)] {
            let bad = stat(Ip6statCounters::Ip6sBadoptions);
            let rh = [IPPROTO_UDP as u8, 0, ty, 1, 0, 0, 0, 0];
            assert_eq!(input_at(at, &rh), (IPPROTO_DONE, at as i32, false));
            assert_eq!(stat(Ip6statCounters::Ip6sBadoptions), bad + 1);
            assert_eq!(
                take_icmp6_errors(),
                [(
                    sysdefs::netinet_icmp6::ICMP6_PARAM_PROB,
                    sysdefs::netinet_icmp6::ICMP6_PARAMPROB_HEADER
                )]
            );
            assert_eq!(
                take_icmp6_params(),
                [at as i32 + 2],
                "the routing type's offset"
            );
        }
    }

    #[test]
    fn a_short_header_is_too_short() {
        let _g = testroutes_setup();
        let short = stat(Ip6statCounters::Ip6sTooshort);
        assert_eq!(
            input_at(40, &[IPPROTO_UDP as u8, 0, 0]),
            (IPPROTO_DONE, 40, false)
        );
        assert_eq!(stat(Ip6statCounters::Ip6sTooshort), short + 1);
    }
}
/* </TESTS> */
