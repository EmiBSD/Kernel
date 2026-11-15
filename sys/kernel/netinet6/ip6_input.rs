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
//! IPv6 input, the part of `netinet6/ip6_input.c` the output path and the control blocks
//! need: the walkers of the extension header chain ([`ip6_nexthdr`], [`ip6_lasthdr`]) and the
//! hop-by-hop option parser ([`ip6_process_hopopts`], with [`ip6_unknown_opt`]), which
//! `ip6_output` runs over a packet's own hop-by-hop header and `in6_proto_cksum_out` uses to
//! find the upper-layer header, and [`INET6CTLERRMAP`], `in6_pcbnotify`'s errors; since
//! N6.5d's first step also [`ip6_get_prevhdr`], and `<netinet/ip6.h>`'s `ip6_exthdr_get`
//! ([`ip6_exthdr_get`], [`ip6_exthdr_bytes`]) and the one-byte header write
//! ([`ip6_set_byte`]) the extension header inputs and reassembly use. The rest of
//! `ip6_input.c` is `bsd`'s `netinet6/ip6_input.rs` until N6.5d's input path, which calls
//! these.
//!
//! Upstream: sys/netinet6/ip6_input.c @ 3ce1f3f79392
//! LZ: sys/netinet6/ip6_input.rs@3c62ede68802
//!
//! ## Deviations
//! - `ip6_process_hopopts`, `ip6_nexthdr` and `ip6_lasthdr` return `bool` and `Option`s
//!   where the C returns `-1`; `ip6_unknown_opt` returns `true` to skip the option (the C
//!   returns its length, which its callers read again themselves).
//! - The ICMPv6 errors of `ip6_process_hopopts` and `ip6_unknown_opt` go through the
//!   temporary `Inet::icmp6_error` leg (`bsd`'s `icmp6_error` until ICMPv6 moves at N6.5d):
//!   before the legs are registered the packet is freed, the C without `INET6`.
//!
//! ## Redesign
//! - LZ's `unsafe fn ip6_process_hopopts(mp, opthead: *const u8, hbhlen, ..)` (a slice made
//!   from the caller's pointer) and `unsafe fn ip6_unknown_opt(mp, optp: *const u8, off)`
//!   -> functions over the packet: the options are named by their offset in the packet
//!   (`off`, the C's `opthead`), and each step copies the option bytes it reads
//!   (`m_copydata` of at most six, the C's reads in place) out of the chain before it acts, so
//!   no borrow of the packet outlives a step that may free it or send an ICMPv6 error;
//!   `ip6_unknown_opt` takes the option's type byte, the only byte of `optp` the C reads. The
//!   options' own length checks are the C's, made against the `hbhlen` the caller found
//!   (every read is inside those bytes); a read past the packet itself, which no caller can
//!   ask for (the output path's header is in its own mbuf, the input path's pulled up), is
//!   counted as `ip6s_toosmall` and the packet dropped.
//! - The header copies of the walkers are `pod` images of their bytes (`m_copydata` into a
//!   small array, as LZ's), the C's `m_copydata` into a stack structure.
//! - `ip6_exthdr_get` (LZ: `bsd`'s `netinet/ip6.rs`, a raw pointer into the region) answers
//!   where the region is (the chain back, the index of its mbuf and the offset there;
//!   `m_pulldown` when the first mbuf does not hold it), and [`ip6_exthdr_bytes`] lends its
//!   bytes; `nd6_nbr`'s `nd6_msg` of N6.5b is this shape.
//!
//! **Lock and spl transitions** against the C: none (the C takes none; the caller holds the
//! net lock).

use core::mem::{offset_of, size_of};

use hal::intr::IPL_SOFTNET;
use hal::netlock::{NETLOCK, NetHeld};
use hal::sys::errno::Errno;
use hal::sys::netinet_icmp6::{ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, ICMP6_PARAMPROB_OPTION};
use hal::sys::netinet_in::{
    IPPROTO_AH, IPPROTO_DSTOPTS, IPPROTO_ESP, IPPROTO_FRAGMENT, IPPROTO_HOPOPTS, IPPROTO_IPCOMP,
    IPPROTO_IPV6, IPPROTO_NONE, IPPROTO_ROUTING,
};
use hal::sys::netinet_ip6::{
    IP6F_OFF_MASK, IP6OPT_JUMBO, IP6OPT_JUMBO_LEN, IP6OPT_MINLEN, IP6OPT_PAD1, IP6OPT_PADN,
    IP6OPT_ROUTER_ALERT, IP6OPT_RTALERT_LEN, IP6OPT_TYPE_DISCARD, IP6OPT_TYPE_FORCEICMP,
    IP6OPT_TYPE_ICMP, IP6OPT_TYPE_SKIP, IPV6_MAXPACKET, Ip6Ext, Ip6Frag, Ip6Hbh, Ip6Hdr,
    ip6opt_type,
};
use hal::sys::netinet6_in6::in6_is_addr_multicast;
use hal::sys::netinet6_ip6_var::Ip6statCounters;

use crate::net::mbuf::{
    M_BCAST, M_DONTWAIT, M_MCAST, M_PKTHDR, Mbuf, MbufBox, m_copyback, m_copydata,
};
use crate::net::mbuf::{MbufQueue, ml_dequeue, ml_empty, mq_delist, mq_enqueue};
use crate::net::mbuf2::m_pulldown;
use crate::net::proto::PRC_NCMDS;
use crate::netinet6::icmp6::icmp6_error;
use crate::netinet6::ip6_var::ip6stat_inc;
use crate::task::{Task, task_add};

/// `sizeof(struct ip6_hdr)`.
const IP6_HDR_LEN: i32 = size_of::<Ip6Hdr>() as i32;

/// `inet6ctlerrmap[]`: the errno of each `PRC_*` control command (`in6_pcbnotify`'s, N6.5c;
/// `bsd`'s `ip6_input.rs` re-exports it for its protocols' `ctlinput`s).
pub static INET6CTLERRMAP: [Option<Errno>; PRC_NCMDS] = [
    None,
    None,
    None,
    None,
    None,
    Some(Errno::EMSGSIZE),
    Some(Errno::EHOSTDOWN),
    Some(Errno::EHOSTUNREACH),
    Some(Errno::EHOSTUNREACH),
    Some(Errno::EHOSTUNREACH),
    Some(Errno::ECONNREFUSED),
    Some(Errno::ECONNREFUSED),
    Some(Errno::EMSGSIZE),
    Some(Errno::EHOSTUNREACH),
    None,
    None,
    None,
    None,
    None,
    None,
    Some(Errno::ENOPROTOOPT),
];

/// `erroff`: where the options of a hop-by-hop header right after the IPv6 header start, the
/// base of the parameter problem offsets (RFC 2460 p7: the hop-by-hop header follows the IPv6
/// header).
const HBH_ERROFF: i32 = IP6_HDR_LEN + size_of::<Ip6Hbh>() as i32;

/// `N` bytes of `m` from offset `off`, `None` when the packet does not hold them.
#[inline]
fn bytes_at<const N: usize>(m: &Mbuf, off: i32) -> Option<[u8; N]> {
    if off < 0 || i64::from(m.m_pkthdr().len.get()) < i64::from(off) + N as i64 {
        return None;
    }
    let mut b = [0u8; N];
    m_copydata(m, off, &mut b);
    Some(b)
}

/// `ip6_exthdr_get(&m, off, len)` (`<netinet/ip6.h>`): the `len` bytes at `off` of `m` made
/// contiguous, in place when the first mbuf holds them, else by `m_pulldown`: the chain back
/// with the index of the mbuf they are in and their offset there; `None` (the chain freed,
/// the C's NULL with `*mp` cleared) when the chain does not hold them or memory runs out.
pub fn ip6_exthdr_get(m: MbufBox, off: i32, len: i32) -> Option<(MbufBox, usize, usize)> {
    let (Ok(o), Ok(l)) = (usize::try_from(off), usize::try_from(len)) else {
        return None;
    };
    if m.data().len() >= o + l {
        return Some((m, 0, o));
    }
    let (m, at, o) = m_pulldown(m, off, len, true)?;
    Some((m, at, usize::try_from(o).ok()?))
}

/// The `len` bytes [`ip6_exthdr_get`] made contiguous in mbuf `at` of `m` at `o`.
#[inline]
pub fn ip6_exthdr_bytes(m: &Mbuf, at: usize, o: usize, len: usize) -> Option<&[u8]> {
    let mut n = m;
    for _ in 0..at {
        n = n.m_next.get()?;
    }
    n.data().get(o..o + len)
}

/// The byte at offset `off` of the packet set to `v` (a next header field: the C's
/// `*prvnxtp = nxt` through `ip6_exthdr_get`'s pointer, frag6's), in place in the mbuf of the
/// chain that holds it: found by the walk `off >= m_len`, never through `m_copyback` (whose
/// walk stops on the mbuf before when `off` is that mbuf's length and makes it writable for a
/// write of none of its bytes, a copy of a shared cluster). A read-only mbuf is given a
/// cluster of its own first (the C writes into the shared one); `false` when the packet does
/// not hold the byte or no cluster can be had.
#[must_use]
pub fn ip6_set_byte(m: &mut Mbuf, off: i32, v: u8) -> bool {
    let Ok(mut off) = usize::try_from(off) else {
        return false;
    };
    let mut p = m;
    while off >= p.m_len().get() as usize {
        off -= p.m_len().get() as usize;
        p = match p.m_next.get_mut() {
            Some(n) => n,
            None => return false,
        };
    }
    if let Some(d) = p.data_mut() {
        d[off] = v;
        return true;
    }
    // A shared cluster: the byte's own mbuf copied (`off` is inside it, never at its end).
    m_copyback(p, off as i32, &[v], M_DONTWAIT).is_ok()
}

/// The bytes at offset `off` of the packet set to `v` (headers the C writes in place through a
/// pointer into the packet), in the mbuf of the chain that holds `off`, found as
/// [`ip6_set_byte`] finds it; bytes that run past that mbuf, or a read-only one (a shared
/// cluster, which the C writes anyway), go through `m_copyback` from that mbuf on (`off` is
/// inside it: no copy of the mbuf before). `false` when the packet does not hold them or no
/// buffer can be had.
#[must_use]
pub fn ip6_set_bytes(m: &mut Mbuf, off: i32, v: &[u8]) -> bool {
    let Ok(mut off) = usize::try_from(off) else {
        return false;
    };
    let mut p = m;
    while off >= p.m_len().get() as usize {
        off -= p.m_len().get() as usize;
        p = match p.m_next.get_mut() {
            Some(n) => n,
            None => return false,
        };
    }
    if let Some(d) = p.data_mut()
        && let Some(dst) = d.get_mut(off..off + v.len())
    {
        dst.copy_from_slice(v);
        return true;
    }
    m_copyback(p, off as i32, v, M_DONTWAIT).is_ok()
}

/// `ip6_hopopts_input`'s second half, `ip6_process_hopopts(mp, opthead, hbhlen, rtalertp,
/// plenp)`: searches the `hbhlen` bytes of hop-by-hop options at offset `off` of the packet
/// `*mp` (the C's `opthead`) and processes each option (the router alert into `rtalertp`, the
/// jumbo payload length into `plenp`); `true` on success (the C's 0), `false` when the packet
/// was dropped or an ICMPv6 error sent for it (the C's -1, `*mp` cleared then). This function
/// is separate from `ip6_hopopts_input()` in order to handle a case where the sending node
/// itself process its hop-by-hop options header: it is called from `ip6_output()` then. The
/// hop-by-hop header is located right after the IPv6 header (RFC2460 p7), which the error
/// offsets assume.
pub fn ip6_process_hopopts(
    mp: &mut Option<MbufBox>,
    off: i32,
    hbhlen: i32,
    rtalertp: &mut u32,
    plenp: &mut u32,
    net: &NetHeld<'_>,
) -> bool {
    let mut hbhlen = hbhlen;
    // `opt - opthead`: the current option's offset among the options.
    let mut i: i32 = 0;

    // The `N` bytes of the current option, the packet dropped as too small when it does not
    // hold them (no caller lets that happen: the options were made or pulled up whole).
    macro_rules! opt {
        ($n:literal) => {{
            match mp.as_deref().and_then(|m| bytes_at::<$n>(m, off + i)) {
                Some(b) => b,
                None => {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
            }
        }};
    }

    while hbhlen > 0 {
        let [ty] = opt!(1);
        let optlen: i32 = match ty {
            IP6OPT_PAD1 => 1,
            IP6OPT_PADN => {
                if hbhlen < IP6OPT_MINLEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [_, len] = opt!(2);
                i32::from(len) + 2
            }
            IP6OPT_ROUTER_ALERT => {
                // XXX may need check for alignment
                if hbhlen < IP6OPT_RTALERT_LEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [_, len, a0, a1] = opt!(4);
                if usize::from(len) != IP6OPT_RTALERT_LEN - 2 {
                    // XXX stat
                    if let Some(m) = mp.take() {
                        icmp6_error(
                            m,
                            ICMP6_PARAM_PROB,
                            ICMP6_PARAMPROB_HEADER,
                            HBH_ERROFF + i + 1,
                            net,
                        );
                    }
                    return false;
                }
                *rtalertp = u32::from(u16::from_be_bytes([a0, a1]));
                IP6OPT_RTALERT_LEN as i32
            }
            IP6OPT_JUMBO => {
                // XXX may need check for alignment
                if hbhlen < IP6OPT_JUMBO_LEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [_, len, j0, j1, j2, j3] = opt!(6);
                if usize::from(len) != IP6OPT_JUMBO_LEN - 2 {
                    // XXX stat
                    if let Some(m) = mp.take() {
                        icmp6_error(
                            m,
                            ICMP6_PARAM_PROB,
                            ICMP6_PARAMPROB_HEADER,
                            HBH_ERROFF + i + 1,
                            net,
                        );
                    }
                    return false;
                }

                // IPv6 packets that have non 0 payload length must not contain a jumbo
                // payload option.
                let Some(m) = mp.take() else {
                    return false;
                };
                let plen = match bytes_at::<2>(&m, offset_of!(Ip6Hdr, ip6_plen) as i32) {
                    Some(p) => u16::from_ne_bytes(p),
                    None => kpanic_short(),
                };
                if plen != 0 {
                    ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
                    icmp6_error(
                        m,
                        ICMP6_PARAM_PROB,
                        ICMP6_PARAMPROB_HEADER,
                        HBH_ERROFF + i,
                        net,
                    );
                    return false;
                }

                // We may see jumbolen in unaligned location, so we'd need to perform memcpy().
                let jumboplen = u32::from_be_bytes([j0, j1, j2, j3]);

                // if there are multiple jumbo payload options, *plenp will be non-zero and the
                // packet will be rejected. the behavior may need some debate in ipngwg -
                // multiple options does not make sense, however, there's no explicit mention in
                // specification.
                if *plenp != 0 {
                    ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
                    icmp6_error(
                        m,
                        ICMP6_PARAM_PROB,
                        ICMP6_PARAMPROB_HEADER,
                        HBH_ERROFF + i + 2,
                        net,
                    );
                    return false;
                }

                // jumbo payload length must be larger than 65535.
                if jumboplen as usize <= IPV6_MAXPACKET {
                    ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
                    icmp6_error(
                        m,
                        ICMP6_PARAM_PROB,
                        ICMP6_PARAMPROB_HEADER,
                        HBH_ERROFF + i + 2,
                        net,
                    );
                    return false;
                }
                *plenp = jumboplen;
                *mp = Some(m);
                IP6OPT_JUMBO_LEN as i32
            }
            // unknown option
            _ => {
                if hbhlen < IP6OPT_MINLEN as i32 {
                    ip6stat_inc(Ip6statCounters::Ip6sToosmall);
                    drop(mp.take());
                    return false;
                }
                let [ty, len] = opt!(2);
                if !ip6_unknown_opt(mp, ty, HBH_ERROFF + i, net) {
                    return false;
                }
                i32::from(len) + 2
            }
        };
        hbhlen -= optlen;
        i += optlen;
    }

    true
}

/// The IPv6 header the jumbo option's check reads is not in the packet: no caller passes such
/// a packet (both put the hop-by-hop header after one), so this is `panic(9)`, as reading past
/// the first mbuf would be in the C.
#[cold]
fn kpanic_short() -> ! {
    crate::assert::kpanic(format_args!(
        "ip6_process_hopopts: a jumbo option without an IPv6 header"
    ))
}

/// `ip6_unknown_opt(mp, optp, off)`: unknown option processing, by the action bits of the
/// option's type `ty` (`*optp`). `off` is the offset from the IPv6 header to the option,
/// which allows returning an ICMPv6 error even if the IPv6 header and the option header are
/// not continuous. `true` to skip the option (the C returns its length), `false` when the
/// packet was dropped or an ICMPv6 error sent (the C's -1, `*mp` cleared).
pub fn ip6_unknown_opt(mp: &mut Option<MbufBox>, ty: u8, off: i32, net: &NetHeld<'_>) -> bool {
    match ip6opt_type(ty) {
        // ignore the option
        IP6OPT_TYPE_SKIP => true,
        // silently discard
        IP6OPT_TYPE_DISCARD => {
            drop(mp.take());
            false
        }
        // send ICMP even if multicasted
        IP6OPT_TYPE_FORCEICMP => {
            ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
            if let Some(m) = mp.take() {
                icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_OPTION, off, net);
            }
            false
        }
        // send ICMP if not multicasted
        IP6OPT_TYPE_ICMP => {
            ip6stat_inc(Ip6statCounters::Ip6sBadoptions);
            if let Some(m) = mp.take() {
                let dst = bytes_at::<16>(&m, offset_of!(Ip6Hdr, ip6_dst) as i32);
                let mcast = dst.is_some_and(|d| {
                    in6_is_addr_multicast(&hal::sys::netinet6_in6::In6Addr { s6_addr: d })
                });
                if mcast || m.flags() & (M_BCAST | M_MCAST) != 0 {
                    drop(m);
                } else {
                    icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_OPTION, off, net);
                }
            }
            false
        }
        _ => {
            drop(mp.take()); // XXX: NOTREACHED
            false
        }
    }
}

/// `ip6_get_prevhdr(m, off)`: the offset of the next header field that precedes offset
/// `off` (the field of the header before the one at `off`). `panic(9)` for an offset inside
/// the IPv6 header, as the C.
pub fn ip6_get_prevhdr(m: &Mbuf, off: i32) -> i32 {
    if off == IP6_HDR_LEN {
        return offset_of!(Ip6Hdr, ip6_nxt) as i32;
    } else if off < IP6_HDR_LEN {
        crate::assert::kpanic(format_args!(
            "ip6_get_prevhdr: off < sizeof(struct ip6_hdr)"
        ));
    }

    let mut nxt = i32::from(m.data()[offset_of!(Ip6Hdr, ip6_nxt)]);
    let mut len = IP6_HDR_LEN;
    let mut nlen = 0;
    while len < off {
        let mut ip6e = [0u8; size_of::<Ip6Ext>()];
        m_copydata(m, len, &mut ip6e);
        let elen = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_len)]);

        nlen = match nxt {
            IPPROTO_FRAGMENT => size_of::<Ip6Frag>() as i32,
            IPPROTO_AH => (elen + 2) << 2,
            _ => (elen + 1) << 3,
        };
        len += nlen;
        nxt = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_nxt)]);
    }

    len - nlen
}

/// `ip6_nexthdr(m, off, proto, nxtp)`: the offset of the header after the one of protocol
/// `proto` at `off`, its protocol stored in `nxtp`; `m` is retained. `None` where the C
/// returns -1.
pub fn ip6_nexthdr(m: &Mbuf, off: i32, proto: i32, nxtp: &mut i32) -> Option<i32> {
    let pktlen = m.m_pkthdr().len.get();
    if m.flags() & M_PKTHDR == 0 || pktlen < off {
        return None;
    }

    match proto {
        IPPROTO_IPV6 => {
            if pktlen < off + IP6_HDR_LEN {
                return None;
            }
            let ip6 = bytes_at::<{ size_of::<Ip6Hdr>() }>(m, off)?;
            *nxtp = i32::from(ip6[offset_of!(Ip6Hdr, ip6_nxt)]);
            Some(off + IP6_HDR_LEN)
        }
        IPPROTO_FRAGMENT => {
            // terminate parsing if it is not the first fragment, it does not make sense to
            // parse through it.
            if pktlen < off + size_of::<Ip6Frag>() as i32 {
                return None;
            }
            let fh = bytes_at::<{ size_of::<Ip6Frag>() }>(m, off)?;
            let o = offset_of!(Ip6Frag, ip6f_offlg);
            if u16::from_ne_bytes([fh[o], fh[o + 1]]) & IP6F_OFF_MASK != 0 {
                return None;
            }
            *nxtp = i32::from(fh[offset_of!(Ip6Frag, ip6f_nxt)]);
            Some(off + size_of::<Ip6Frag>() as i32)
        }
        IPPROTO_AH | IPPROTO_HOPOPTS | IPPROTO_ROUTING | IPPROTO_DSTOPTS => {
            if pktlen < off + size_of::<Ip6Ext>() as i32 {
                return None;
            }
            let ip6e = bytes_at::<{ size_of::<Ip6Ext>() }>(m, off)?;
            *nxtp = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_nxt)]);
            let len = i32::from(ip6e[offset_of!(Ip6Ext, ip6e_len)]);
            let off = if proto == IPPROTO_AH {
                off + ((len + 2) << 2)
            } else {
                off + ((len + 1) << 3)
            };
            if pktlen < off {
                return None;
            }
            Some(off)
        }
        // give up
        IPPROTO_NONE | IPPROTO_ESP | IPPROTO_IPCOMP => None,
        _ => None,
    }
}

/// `ip6_lasthdr(m, off, proto, nxtp)`: the offset of the last header in the chain (the
/// upper-layer protocol's), its protocol stored in `nxtp`; `m` is kept untainted. `None`
/// where the C returns -1 (an invalid chain).
pub fn ip6_lasthdr(m: &Mbuf, off: i32, proto: i32, nxtp: &mut i32) -> Option<i32> {
    let mut off = off;
    let mut proto = proto;

    loop {
        let Some(newoff) = ip6_nexthdr(m, off, proto, nxtp) else {
            return Some(off);
        };
        if newoff < off {
            return None; // invalid
        } else if newoff == off {
            return Some(newoff);
        }

        off = newoff;
        proto = *nxtp;
    }
}
/// `ip6send_mq`: the packets `ip6_send` queued for `ip6send_task`.
static IP6SEND_MQ: MbufQueue = MbufQueue::new(64, IPL_SOFTNET);
/// `ip6send_task` (its function names its queue: no argument).
static IP6SEND_TASK: Task = Task::new(ip6_send_dispatch, core::ptr::null_mut());

/// `ip6_send_dispatch(xmq)`: `ip6send_task`: sends the packets queued with `ip6_output`, under
/// the shared net lock.
fn ip6_send_dispatch(_xmq: *mut core::ffi::c_void) {
    let mut ml = mq_delist(&IP6SEND_MQ);
    if ml_empty(&ml) {
        return;
    }

    let shared = NETLOCK.read();
    let Some(net) = NetHeld::of(&shared) else {
        crate::assert::kpanic(format_args!("ip6_send: the net lock"));
    };
    while let Some(m) = ml_dequeue(&mut ml) {
        let _ = crate::netinet6::ip6_output::ip6_output(m, None, None, 0, None, None, &net);
    }
}

/// `ip6_send(m)`: queues `m` for `ip6_output` from the softnet task (ICMPv6's replies and
/// errors, made where `ip6_output` cannot be called).
pub fn ip6_send(m: MbufBox) {
    let _ = mq_enqueue(&IP6SEND_MQ, m);
    if let Some(tq) = crate::net::if_::net_tq(0) {
        let _ = task_add(tq, core::pin::Pin::static_ref(&IP6SEND_TASK));
    }
}

/// The packets `ip6_send` queued, taken (the host tests look at what ICMPv6 sent).
#[cfg(any(test, feature = "doubles"))]
pub fn ip6_send_test_take() -> crate::net::mbuf::MbufList {
    mq_delist(&IP6SEND_MQ)
}

/// The send queue forgotten for the host tests (its packets live in an earlier test's memory).
#[cfg(any(test, feature = "doubles"))]
pub fn ip6_test_reset() {
    core::mem::forget(mq_delist(&IP6SEND_MQ));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the walkers and the hop-by-hop parser over kernel packets: the chain walk
    // (LZ's `the_extension_header_chain_is_walked`, but for `ip6_get_prevhdr`, which stays
    // `bsd`'s), the option walk and the unknown option actions (LZ's
    // `hop_by_hop_options_are_walked`' parser half and `unknown_option_actions`), and one
    // crafted option string per check of `ip6_process_hopopts`, each passing every earlier
    // check, with the C's answer (the counter, the ICMPv6 error and its offset).

    use std::vec::Vec;

    use hal::sys::netinet_in::IPPROTO_UDP;
    use hal::sys::netinet_ip6::IPV6_VERSION;
    use hal::sys::netinet6_ip6_var::Ip6statCounters as C;

    use super::*;
    use crate::net::mbuf::m_devget;
    use crate::net::route::tests::{take_icmp6_errors, take_icmp6_params, testroutes_setup};
    use crate::netinet6::ip6_var::IP6COUNTERS;

    fn stat(c: C) -> u64 {
        IP6COUNTERS.read()[c as usize]
    }

    /// fd00::`last`.
    fn ula(last: u8) -> [u8; 16] {
        let mut a = [0u8; 16];
        a[0] = 0xfd;
        a[15] = last;
        a
    }

    /// An IPv6 header (`nxt`, payload length `plen`, to `dst`) followed by `rest`.
    fn packet(nxt: i32, plen: u16, dst: [u8; 16], rest: &[u8]) -> MbufBox {
        let mut b = Vec::new();
        b.extend_from_slice(&u32::from(IPV6_VERSION).wrapping_shl(24).to_be_bytes());
        b.extend_from_slice(&plen.to_be_bytes());
        b.push(nxt as u8);
        b.push(64);
        b.extend_from_slice(&ula(1));
        b.extend_from_slice(&dst);
        b.extend_from_slice(rest);
        m_devget(&b, 0).expect("packet")
    }

    /// The hop-by-hop options `opts` of a packet to `dst` with payload length `plen` (the
    /// options after the IPv6 header and the 2-byte hop-by-hop header, then 8 bytes of UDP, so
    /// that an option read past the options' length finds bytes, as in the C's packets, rather
    /// than the packet's end), run through the parser: its answer, the router alert and jumbo
    /// length it stored, whether the packet is still the caller's.
    fn run(opts: &[u8], plen: u16, dst: [u8; 16]) -> (bool, u32, u32, bool) {
        let mut rest = std::vec![IPPROTO_UDP as u8, 0];
        rest.extend_from_slice(opts);
        rest.extend_from_slice(&[0; 8]);
        let mut mp = Some(packet(IPPROTO_HOPOPTS, plen, dst, &rest));
        let (mut rtalert, mut jumbo) = (!0, 0);
        let r = hal::netlock::NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        let ok = ip6_process_hopopts(
            &mut mp,
            HBH_ERROFF,
            opts.len() as i32,
            &mut rtalert,
            &mut jumbo,
            &net,
        );
        (ok, rtalert, jumbo, mp.is_some())
    }

    #[test]
    fn hop_by_hop_options_are_walked() {
        let _g = testroutes_setup();
        let _ = take_icmp6_errors();

        // Pad1, a router alert for MLD (0x0000 as `MLD6_HBH`), PadN of 0.
        let opts = [IP6OPT_PAD1, IP6OPT_ROUTER_ALERT, 2, 0, 1, IP6OPT_PADN, 0, 0];
        assert_eq!(run(&opts, 8, ula(2)), (true, 1, 0, true));

        // An unknown option whose action is "skip" (type 0x1e) between PadN options.
        let opts = [0x1e, 2, 0xaa, 0xbb, IP6OPT_PADN, 0, 0, 0];
        assert_eq!(run(&opts[..6], 6, ula(2)), (true, !0, 0, true));

        // A valid jumbo option on a packet whose payload length is 0.
        let opts = [IP6OPT_PADN, 0, IP6OPT_JUMBO, 4, 0, 1, 0, 0];
        assert_eq!(run(&opts, 0, ula(2)), (true, !0, 0x1_0000, true));
        assert!(take_icmp6_errors().is_empty());
    }

    /// One option string per check of `ip6_process_hopopts` (each passes every earlier check)
    /// and the C's answer: the counter it bumps, the parameter problem it sends (its offset
    /// from the IPv6 header) or none (dropped silently, or counted only).
    #[test]
    fn each_bound_of_the_option_walk_answers_as_the_c() {
        let _g = testroutes_setup();
        struct Case {
            what: &'static str,
            opts: &'static [u8],
            plen: u16,
            counter: Option<C>,
            error: Option<(u8, u8, i32)>,
        }
        let prob = |code, param| Some((ICMP6_PARAM_PROB, code, param));
        let cases = [
            Case {
                what: "PadN with one byte left",
                opts: &[IP6OPT_PAD1, IP6OPT_PADN],
                plen: 8,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "router alert with three bytes left",
                opts: &[IP6OPT_ROUTER_ALERT, 2, 0],
                plen: 8,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "router alert of a wrong length",
                opts: &[IP6OPT_ROUTER_ALERT, 3, 0, 0, 0, 0],
                plen: 8,
                counter: None,
                error: prob(ICMP6_PARAMPROB_HEADER, 43),
            },
            Case {
                what: "jumbo with five bytes left",
                opts: &[IP6OPT_JUMBO, 4, 0, 1, 0],
                plen: 0,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "jumbo of a wrong length",
                opts: &[IP6OPT_JUMBO, 5, 0, 1, 0, 0, 0, 0],
                plen: 0,
                counter: None,
                error: prob(ICMP6_PARAMPROB_HEADER, 43),
            },
            Case {
                what: "jumbo with a payload length",
                opts: &[IP6OPT_JUMBO, 4, 0, 1, 0, 0],
                plen: 8,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_HEADER, 42),
            },
            Case {
                what: "two jumbo options",
                opts: &[IP6OPT_JUMBO, 4, 0, 1, 0, 0, IP6OPT_JUMBO, 4, 0, 1, 0, 0],
                plen: 0,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_HEADER, 50),
            },
            Case {
                what: "jumbo of at most 65535",
                opts: &[IP6OPT_JUMBO, 4, 0, 0, 0xff, 0xff],
                plen: 0,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_HEADER, 44),
            },
            Case {
                what: "unknown option with one byte left",
                opts: &[IP6OPT_PAD1, 0x1e],
                plen: 8,
                counter: Some(C::Ip6sToosmall),
                error: None,
            },
            Case {
                what: "unknown option, discard",
                opts: &[0x5e, 0],
                plen: 8,
                counter: None,
                error: None,
            },
            Case {
                what: "unknown option, ICMP even to multicast",
                opts: &[0x9e, 0],
                plen: 8,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_OPTION, 42),
            },
            Case {
                what: "unknown option, ICMP unless multicast",
                opts: &[0xde, 0],
                plen: 8,
                counter: Some(C::Ip6sBadoptions),
                error: prob(ICMP6_PARAMPROB_OPTION, 42),
            },
        ];
        for c in &cases {
            let _ = take_icmp6_errors();
            let _ = take_icmp6_params();
            let before = c.counter.map(stat);
            let (ok, _, _, kept) = run(c.opts, c.plen, ula(2));
            assert!(!ok && !kept, "{}: refused, the packet gone", c.what);
            if let (Some(k), Some(b)) = (c.counter, before) {
                assert_eq!(stat(k), b + 1, "{}: counted", c.what);
            }
            let errors = take_icmp6_errors();
            let params = take_icmp6_params();
            match c.error {
                Some((t, code, param)) => {
                    assert_eq!(errors, [(t, code)], "{}: the parameter problem", c.what);
                    assert_eq!(params, [param], "{}: its offset", c.what);
                }
                None => assert!(errors.is_empty(), "{}: no error", c.what),
            }
        }
        // "ICMP unless multicast" to a multicast destination: dropped without an error.
        let mut mcast = ula(2);
        mcast[0] = 0xff;
        let _ = take_icmp6_errors();
        assert!(!run(&[0xde, 0], 8, mcast).0);
        assert!(
            take_icmp6_errors().is_empty(),
            "no error to a multicast group"
        );
    }

    #[test]
    fn unknown_option_actions() {
        let _g = testroutes_setup();
        let mut mp = Some(packet(IPPROTO_UDP, 0, ula(2), &[0x1e, 0]));
        let r = hal::netlock::NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        assert!(ip6_unknown_opt(&mut mp, 0x1e, 42, &net));
        assert!(mp.is_some());

        assert!(!ip6_unknown_opt(&mut mp, 0x7f, 42, &net));
        assert!(mp.is_none());
    }

    /// An IPv6 header, a hop-by-hop header (8), destination options (16), a fragment header
    /// (`offlg`) and 8 bytes of UDP.
    fn chain(offlg: u16) -> MbufBox {
        let mut rest = Vec::new();
        rest.extend_from_slice(&[IPPROTO_DSTOPTS as u8, 0, 1, 4, 0, 0, 0, 0]);
        rest.extend_from_slice(&[IPPROTO_FRAGMENT as u8, 1, 1, 12]);
        rest.extend_from_slice(&[0; 12]);
        rest.extend_from_slice(&[IPPROTO_UDP as u8, 0]);
        rest.extend_from_slice(&offlg.to_be_bytes());
        rest.extend_from_slice(&[0, 0, 0, 7]);
        rest.extend_from_slice(&[0; 8]);
        packet(IPPROTO_HOPOPTS, rest.len() as u16, ula(2), &rest)
    }

    #[test]
    fn the_extension_header_chain_is_walked() {
        let _g = testroutes_setup();

        let m = chain(0x0001); // first fragment, more fragments
        let mut nxt = -1;
        assert_eq!(ip6_nexthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(40));
        assert_eq!(nxt, IPPROTO_HOPOPTS);
        assert_eq!(ip6_nexthdr(&m, 40, IPPROTO_HOPOPTS, &mut nxt), Some(48));
        assert_eq!(nxt, IPPROTO_DSTOPTS);
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(72));
        assert_eq!(nxt, IPPROTO_UDP);

        // A later fragment: the walk stops at the fragment header.
        let m = chain(0x0040); // byte offset 64
        let mut nxt = -1;
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(64));
        assert_eq!(nxt, IPPROTO_FRAGMENT);
        // An ESP header is never walked into.
        assert_eq!(ip6_nexthdr(&m, 64, IPPROTO_ESP, &mut nxt), None);
        // A header past the end of the packet.
        assert_eq!(ip6_nexthdr(&m, 1000, IPPROTO_HOPOPTS, &mut nxt), None);

        // AH counts its length in 4-byte words, plus 2.
        let ah = [IPPROTO_UDP as u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let m = packet(IPPROTO_AH, ah.len() as u16, ula(2), &ah);
        let mut nxt = -1;
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(52));
        assert_eq!(nxt, IPPROTO_UDP);

        // A header whose length runs past the packet ends the walk where it starts; its next
        // header is stored all the same (the C stores `*nxtp` before the length test).
        let m = packet(
            IPPROTO_DSTOPTS,
            8,
            ula(2),
            &[IPPROTO_UDP as u8, 4, 0, 0, 0, 0, 0, 0],
        );
        let mut nxt = -1;
        assert_eq!(ip6_nexthdr(&m, 40, IPPROTO_DSTOPTS, &mut nxt), None);
        assert_eq!(ip6_lasthdr(&m, 0, IPPROTO_IPV6, &mut nxt), Some(40));
        assert_eq!(nxt, IPPROTO_UDP);
    }
}
/* </TESTS> */
