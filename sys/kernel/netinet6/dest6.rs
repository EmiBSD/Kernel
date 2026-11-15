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
//! Destination options header processing: `netinet6/dest6.c`. [`dest6_input`] is the
//! destination options header's `pr_input`: it walks the options with their lengths checked
//! against the header, skips the padding and hands every other option to `ip6_unknown_opt`,
//! whose action bits decide.
//!
//! Upstream: sys/netinet6/dest6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/dest6.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's walk over a raw pointer into the header `ip6_exthdr_get` made contiguous (5
//!   `unsafe` reads of option bytes) -> the header's bytes lent by the mbuf that holds them
//!   (`ip6_input::ip6_exthdr_bytes`) and read by index; each step reads the two bytes it needs
//!   before `ip6_unknown_opt` may free the packet or answer it with an ICMPv6 error, so no
//!   borrow of the packet outlives that call.
//!
//! **Lock and spl transitions** against the C: none (the C takes none; the caller holds the
//! net lock).

use core::mem::size_of;

use hal::netlock::NetHeld;
use hal::sys::netinet_in::IPPROTO_DONE;
use hal::sys::netinet_ip6::{IP6OPT_MINLEN, IP6OPT_PAD1, IP6OPT_PADN, Ip6Dest};
use hal::sys::netinet6_ip6_var::Ip6statCounters;

use crate::net::if_var::Netstack;
use crate::net::mbuf::MbufBox;
use crate::netinet6::ip6_input::{ip6_exthdr_bytes, ip6_exthdr_get, ip6_unknown_opt};
use crate::netinet6::ip6_var::ip6stat_inc;

/// `dest6_input(mp, offp, proto, af, ns)`: destination options header processing; the next
/// header with `*offp` past the options, or `IPPROTO_DONE` (`*mp` cleared) when the packet
/// was dropped or answered.
pub fn dest6_input(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    _proto: i32,
    _af: i32,
    _ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let Some(m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let mut off = *offp;
    const HDR: usize = size_of::<Ip6Dest>();

    // validation of the length of the header
    let Some((m, at, o)) = ip6_exthdr_get(m, off, HDR as i32) else {
        return IPPROTO_DONE;
    };
    let Some(len) = ip6_exthdr_bytes(&m, at, o, HDR).map(|b| b[1]) else {
        return IPPROTO_DONE;
    };
    let dstoptlen = (usize::from(len) + 1) << 3;

    let Some((m, at, o)) = ip6_exthdr_get(m, off, dstoptlen as i32) else {
        return IPPROTO_DONE;
    };
    off += dstoptlen as i32;
    let mut mp_ = Some(m);

    // search header for all options: `i` the option's offset in the header (the C's `opt`),
    // from the end of its two fixed bytes.
    let mut i = HDR;
    while i < dstoptlen {
        let left = dstoptlen - i;
        // The option's type and length, read before the option is acted on.
        let Some((ty, olen)) = mp_.as_deref().and_then(|m| {
            let h = ip6_exthdr_bytes(m, at, o, dstoptlen)?;
            Some((h[i], h.get(i + 1).copied()))
        }) else {
            return IPPROTO_DONE;
        };
        if ty != IP6OPT_PAD1
            && (left < IP6OPT_MINLEN as usize || olen.is_none_or(|l| usize::from(l) + 2 > left))
        {
            ip6stat_inc(Ip6statCounters::Ip6sToosmall);
            // bad: m_freemp(mp)
            return IPPROTO_DONE;
        }

        let optlen = match ty {
            IP6OPT_PAD1 => 1,
            IP6OPT_PADN => usize::from(olen.unwrap_or(0)) + 2,
            // unknown option
            _ => {
                if !ip6_unknown_opt(&mut mp_, ty, *offp + i as i32, net) {
                    return IPPROTO_DONE;
                }
                usize::from(olen.unwrap_or(0)) + 2
            }
        };
        i += optlen;
    }
    // dstopts->ip6d_nxt
    let Some(nxt) = mp_
        .as_deref()
        .and_then(|m| ip6_exthdr_bytes(m, at, o, HDR))
        .map(|h| h[0])
    else {
        return IPPROTO_DONE;
    };

    *offp = off;
    *mp = mp_;
    i32::from(nxt)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the destination options input over kernel packets: LZ's (the padding
    // skipped, an option running past the header, a header longer than the packet, the
    // unknown options' action bits) and one crafted header per bound of the option walk,
    // each passing the earlier ones, with the C's answer.

    use std::vec::Vec;

    use hal::netlock::NETLOCK;
    use hal::sys::netinet_icmp6::{ICMP6_PARAM_PROB, ICMP6_PARAMPROB_OPTION};
    use hal::sys::netinet_in::IPPROTO_UDP;
    use hal::sys::netinet_ip6::{IPV6_VERSION, Ip6Hdr};
    use hal::sys::netinet6_in6::In6Addr;

    use super::*;
    use crate::net::mbuf::m_devget;
    use crate::net::route::tests::{take_icmp6_errors, take_icmp6_params, testroutes_setup};
    use crate::netinet6::ip6_var::IP6COUNTERS;

    const UDP: u8 = IPPROTO_UDP as u8;

    fn stat(c: Ip6statCounters) -> u64 {
        IP6COUNTERS.read()[c as usize]
    }

    /// fd00::`last`.
    fn ula(last: u8) -> In6Addr {
        let mut a = [0u8; 16];
        a[0] = 0xfd;
        a[15] = last;
        In6Addr { s6_addr: a }
    }

    /// ff02::1.
    fn allnodes() -> In6Addr {
        let mut a = [0u8; 16];
        a[0] = 0xff;
        a[1] = 0x02;
        a[15] = 1;
        In6Addr { s6_addr: a }
    }

    /// A packet to `dst` whose header is followed by the destination options `ext`.
    fn packet_to(dst: In6Addr, ext: &[u8]) -> MbufBox {
        let mut ip6 = Ip6Hdr {
            ip6_plen: (ext.len() as u16).to_be(),
            ip6_nxt: 60,
            ip6_hlim: 64,
            ip6_src: ula(2),
            ip6_dst: dst,
            ..Ip6Hdr::default()
        };
        ip6.set_ip6_vfc(IPV6_VERSION);
        let mut b = std::vec![0u8; 40];
        assert!(hal::pod::put(&mut b, 0, &ip6));
        b.extend_from_slice(ext);
        m_devget(&b, 0).expect("a packet")
    }

    /// `dest6_input` of the options `ext` sent to `dst`, under the net lock: the answer, the
    /// offset after it and whether the packet is still the caller's.
    fn input_to(dst: In6Addr, ext: &[u8]) -> (i32, i32, bool) {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut mp = Some(packet_to(dst, ext));
        let mut off = 40;
        let nxt = dest6_input(&mut mp, &mut off, 60, 10, None, &net);
        (nxt, off, mp.is_some())
    }

    fn input(ext: &[u8]) -> (i32, i32, bool) {
        input_to(ula(1), ext)
    }

    #[test]
    fn pad_options_are_skipped_and_the_next_header_returned() {
        let _g = testroutes_setup();
        #[rustfmt::skip]
        let ext = [
            UDP, 1,
            0, 0,
            1, 4, 0, 0, 0, 0,
            1, 2, 0, 0,
            0, 0,
        ];
        assert_eq!(input(&ext), (i32::from(UDP), 40 + 16, true));
    }

    #[test]
    fn an_option_running_past_the_header_is_dropped() {
        let _g = testroutes_setup();
        let before = stat(Ip6statCounters::Ip6sToosmall);
        assert_eq!(
            input(&[UDP, 0, 1, 6, 0, 0, 0, 0]),
            (IPPROTO_DONE, 40, false)
        );
        assert_eq!(stat(Ip6statCounters::Ip6sToosmall), before + 1);

        let before = stat(Ip6statCounters::Ip6sToosmall);
        assert_eq!(
            input(&[UDP, 0, 0, 0, 0, 0, 0, 1]),
            (IPPROTO_DONE, 40, false)
        );
        assert_eq!(stat(Ip6statCounters::Ip6sToosmall), before + 1);
    }

    #[test]
    fn a_header_longer_than_the_packet_is_dropped() {
        let _g = testroutes_setup();
        let before = stat(Ip6statCounters::Ip6sToosmall);
        assert_eq!(
            input(&[UDP, 2, 0, 0, 0, 0, 0, 0]),
            (IPPROTO_DONE, 40, false)
        );
        assert_eq!(stat(Ip6statCounters::Ip6sToosmall), before, "uncounted");
        // Shorter than its fixed part.
        assert_eq!(input(&[UDP]), (IPPROTO_DONE, 40, false));
    }

    #[test]
    fn unknown_options_follow_their_action_bits() {
        let _g = testroutes_setup();
        let _ = (take_icmp6_errors(), take_icmp6_params());
        let opt = |o: u8| [UDP, 0, o, 4, 0, 0, 0, 0];
        let bad = || stat(Ip6statCounters::Ip6sBadoptions);

        // skip
        assert_eq!(input(&opt(0x1e)), (i32::from(UDP), 48, true));
        // discard, silently
        let b = bad();
        assert_eq!(input(&opt(0x5e)), (IPPROTO_DONE, 40, false));
        assert_eq!(bad(), b);
        assert!(take_icmp6_errors().is_empty());
        // ICMP even if multicast: the option's offset from the IPv6 header.
        for dst in [ula(1), allnodes()] {
            let b = bad();
            assert_eq!(input_to(dst, &opt(0x9e)), (IPPROTO_DONE, 40, false));
            assert_eq!(bad(), b + 1);
            assert_eq!(
                take_icmp6_errors(),
                [(ICMP6_PARAM_PROB, ICMP6_PARAMPROB_OPTION)]
            );
            assert_eq!(take_icmp6_params(), [42]);
        }
        // ICMP unless multicast.
        let b = bad();
        assert_eq!(input(&opt(0xde)), (IPPROTO_DONE, 40, false));
        assert_eq!(bad(), b + 1);
        assert_eq!(take_icmp6_params(), [42]);
        let _ = take_icmp6_errors();
        let b = bad();
        assert_eq!(input_to(allnodes(), &opt(0xde)), (IPPROTO_DONE, 40, false));
        assert_eq!(bad(), b + 1);
        assert!(
            take_icmp6_errors().is_empty(),
            "no error about a multicast packet"
        );
        // The second option of the header: its own offset.
        let ext = [UDP, 0, 1, 0, 0x9e, 2, 0, 0];
        assert_eq!(input(&ext), (IPPROTO_DONE, 40, false));
        assert_eq!(take_icmp6_params(), [44]);
        let _ = take_icmp6_errors();
    }

    /// The bounds of the walk: `*opt != PAD1 && (dstoptlen < IP6OPT_MINLEN || opt[1] + 2 >
    /// dstoptlen)` -> `ip6s_toosmall`, at the walk's last bytes.
    #[test]
    fn each_bound_of_the_option_walk_answers_as_the_c() {
        let _g = testroutes_setup();
        let ok = (i32::from(UDP), 48, true);
        let bad = (IPPROTO_DONE, 40, false);
        #[rustfmt::skip]
        let cases: [([u8; 8], (i32, i32, bool), u64); 6] = [
            // a Pad1 as the last byte: fine
            ([UDP, 0, 1, 3, 0, 0, 0, 0], ok, 0),
            // a PadN as the last byte (one byte left, < IP6OPT_MINLEN)
            ([UDP, 0, 1, 3, 0, 0, 0, 1], bad, 1),
            // a PadN filling the header exactly
            ([UDP, 0, 1, 4, 0, 0, 0, 0], ok, 0),
            // a PadN one byte too long
            ([UDP, 0, 1, 5, 0, 0, 0, 0], bad, 1),
            // a skipped unknown option filling the header exactly
            ([UDP, 0, 0x1e, 4, 0, 0, 0, 0], ok, 0),
            // a skipped unknown option one byte too long
            ([UDP, 0, 0x1e, 5, 0, 0, 0, 0], bad, 1),
        ];
        for (ext, want, small) in cases {
            let before = stat(Ip6statCounters::Ip6sToosmall);
            assert_eq!(input(&ext), want, "{ext:?}");
            assert_eq!(
                stat(Ip6statCounters::Ip6sToosmall),
                before + small,
                "{ext:?}"
            );
        }
        // Options past the first eight bytes: a 16-byte header, the last option at 14.
        let mut ext: Vec<u8> = std::vec![UDP, 1, 1, 10];
        ext.extend_from_slice(&[0; 10]);
        ext.extend_from_slice(&[1, 0]);
        assert_eq!(input(&ext), (i32::from(UDP), 56, true));
        ext[15] = 1;
        assert_eq!(input(&ext).0, IPPROTO_DONE);
    }
}
/* </TESTS> */
