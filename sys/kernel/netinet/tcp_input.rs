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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993, 1994
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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! TCP input: `netinet/tcp_input.c`'s path parameters and round-trip timing (N6.6a3:
//! `tcp_xmit_timer`, `tcp_mss`, `tcp_hdrsz`, `tcp_mss_update`, `tcp_newreno_partialack`;
//! N6.6a4: `tcp_dooptions`, `tcp_pulloutofband`, `tcp_mss_adv`; `tcp_input` itself
//! follows).
//!
//! Upstream: sys/netinet/tcp_input.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_input.rs@3c62ede68802
//!
//! [`tcp_xmit_timer`] folds a round-trip sample into the smoothed estimate and its variance
//! (fixed point, RFC 6298's shape) and recomputes the retransmit timeout; [`tcp_mss`] picks
//! the maximum segment size from the route's MTU, the interface and the peer's offer, and
//! seeds the congestion window; [`tcp_mss_update`] rounds the socket buffers to it;
//! [`tcp_hdrsz`] is the header length every segment carries; [`tcp_newreno_partialack`]
//! retransmits the next unacknowledged segment on a partial ACK.
//!
//! ## Deviations
//! - `tcp_xmit_timer` and `tcp_hdrsz` take the control block's state ([`TcpState`]), all they
//!   read; `tcp_mss`, `tcp_mss_update` and `tcp_newreno_partialack` the control block held
//!   ([`Tp`]), whose proof of the net lock `in_pcbrtentry` uses. `if_put` has no counterpart:
//!   the kernel's `if_get` lends the interface without a count.
//! - The C's `int` arithmetic that can overflow (the estimator's sums, the MTU differences)
//!   wraps here, two's complement, where the C's overflow is undefined; the conversions
//!   between `int`, `u_int` and `u_long` are the C's, in the C's order (`max` and `min` are
//!   libkern's `u_int` functions, `imin` and `imax` the `int` ones).
//!
//! ## Redesign
//! - N6.6a3: these functions are the kernel's, over the kernel's control block
//!   (`tcp_var::Tcpcb`, opened as [`Tp`]); `bsd`'s copies in `sys/netinet/tcp_input.rs` stay
//!   for `bsd`'s TCP until the switch (N6.6a5). LZ's took `&Tcpcb`, a struct of `Cell`s
//!   reached through the pcb's raw word.
//! - `in_pcbrtentry` answers a counted handle of the route (the cache keeps its own), where
//!   the C borrows the cache's pointer under the socket lock: one count pair per
//!   `tcp_mss` and `tcp_mss_update`, per connection set-up and path MTU change, not per
//!   segment.

use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::sys::net_if::IFF_LOOPBACK;
use hal::sys::netinet_in::SockaddrIn;
use hal::sys::netinet_ip::Ip;
use hal::sys::netinet_ip6::{IPV6_MMTU, Ip6Frag, Ip6Hdr};
use hal::sys::netinet_tcp::{
    TCP_MAX_WINSHIFT, TCPOLEN_MAXSEG, TCPOLEN_SACK_PERMITTED, TCPOLEN_SIGLEN, TCPOLEN_SIGNATURE,
    TCPOLEN_TIMESTAMP, TCPOLEN_TSTAMP_APPA, TCPOLEN_WINDOW, TCPOPT_EOL, TCPOPT_MAXSEG, TCPOPT_NOP,
    TCPOPT_SACK, TCPOPT_SACK_PERMITTED, TCPOPT_SIGNATURE, TCPOPT_TIMESTAMP, TCPOPT_WINDOW, TH_SYN,
    Tcphdr,
};
use hal::sys::netinet6_in6::SockaddrIn6;
use hal::sys::socket::{AF_INET, AF_INET6};
use libkern::timingsafe_bcmp;

use crate::assert::kpanic;
use crate::crypto::md5::MD5_DIGEST_LENGTH;
use crate::net::if_::{if_get, unhandled_af};
use crate::net::ipsec_hooks::{SOCKADDR_UNION_LEN, TdbHold, ipsec};
use crate::net::mbuf::{M_DONTWAIT, Mbuf, m_copyback};
use crate::net::route::Rtentry;
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::SB_MAX_VAR;
use crate::netinet::in_pcb::in_pcbrtentry;
use crate::netinet::ip_output::ip_load;
use crate::netinet::ip_var::ip_mtudisc;
use crate::netinet::tcp_fsm::{TCPS_LISTEN, tcps_havercvdsyn};
use crate::netinet::tcp_output::tcp_output;
use crate::netinet::tcp_sack::tcp_sack_option;
use crate::netinet::tcp_seq::seq_gt;
use crate::netinet::tcp_subr::{TCP_DO_RFC3390, TCP_MSSDFLT, tcp_signature};
use crate::netinet::tcp_timer::{TCPT_REXMT, TCPTV_REXMTMAX, tcp_timer_disarm, tcpt_rangeset};
use crate::netinet::tcp_var::{
    TCP_RTT_BASE_SHIFT, TCP_RTT_MAX, TCP_RTT_SHIFT, TCP_RTTVAR_SHIFT, TCPOOB_HAVEDATA, TF_NOOPT,
    TF_PMTUD_PEND, TF_RCVD_SCALE, TF_RCVD_TSTMP, TF_REQ_TSTMP, TF_SACK_PERMIT, TF_SIGNATURE,
    TcpOptInfo, TcpState, TcpstatCounters, Tp, tcp_rexmtval, tcp_time, tcpstat_inc,
};
use crate::netinet6::ip6_output::ip6_load;
use crate::proc::ProcGlue;
use crate::timeout::hz;

/// `AF_INET` as the `int` of `af` and `tp->pf`.
const AF_INET_I32: i32 = AF_INET as i32;
/// `AF_INET6` as the `int` of `af` and `tp->pf`.
const AF_INET6_I32: i32 = AF_INET6 as i32;

/// The `union sockaddr_union` of an address `tcp_dooptions` asks IPsec's SA for: the C's
/// `memset` of the union and the stores of `sa_len`, `sa_family` and the address (`sin_addr` at
/// 4, `sin6_addr` at 8).
fn sig_union(len: usize, family: u8, at: usize, addr: &[u8]) -> [u8; SOCKADDR_UNION_LEN] {
    let mut u = [0u8; SOCKADDR_UNION_LEN];
    u[0] = len as u8;
    u[1] = family;
    u[at..at + addr.len()].copy_from_slice(addr);
    u
}

/// `tcp_dooptions`: parses the options `cp` (the C's pointer and count; NULL is the empty
/// slice) of segment `th` (host order) for the control block whose state is `st` into `oi`
/// (and the window scale, timestamp and SACK permitted flags of `st`), feeds the SACK blocks
/// to `tcp_sack_option`, and checks the TCP MD5 signature against the SA of the segment's
/// addresses (IPv4 or IPv6 by `st.pf`, read from the IP header at the front of `m`).
/// `false` (the C's -1) drops the segment.
///
/// Every option is read inside its own length, which the walk bounds by what is left of
/// `cp`: a length below 2 or past the block ends the walk, a known option of the wrong length
/// is skipped, as the C's `continue`.
#[allow(clippy::too_many_arguments, reason = "the C's prototype")]
#[must_use = "false drops the segment"]
pub fn tcp_dooptions(
    st: &mut TcpState,
    cp: &[u8],
    th: &Tcphdr,
    m: &Mbuf,
    iphlen: i32,
    oi: &mut TcpOptInfo,
    rtableid: u32,
    now: u64,
) -> bool {
    // TCP_SIGNATURE: the signature option's 16 bytes, where they lie in `cp`.
    let mut sigp: Option<&[u8]> = None;
    let syn = th.th_flags & TH_SYN != 0;

    let mut cp = cp;
    while let Some(&opt) = cp.first() {
        if opt == TCPOPT_EOL {
            break;
        }
        let optlen = if opt == TCPOPT_NOP {
            1
        } else {
            let Some(&l) = cp.get(1) else {
                break;
            };
            let l = usize::from(l);
            if l < 2 || l > cp.len() {
                break;
            }
            l
        };
        // `cnt -= optlen, cp += optlen`, the C's step, taken before the option's `continue`s.
        let (o, rest) = cp.split_at(optlen);
        cp = rest;
        match opt {
            TCPOPT_MAXSEG => {
                if optlen != usize::from(TCPOLEN_MAXSEG) || !syn || tcps_havercvdsyn(st.t_state) {
                    continue;
                }
                oi.maxseg = u16::from_be_bytes([o[2], o[3]]);
            }

            TCPOPT_WINDOW => {
                if optlen != usize::from(TCPOLEN_WINDOW) || !syn || tcps_havercvdsyn(st.t_state) {
                    continue;
                }
                st.set_flags(TF_RCVD_SCALE);
                st.requested_s_scale = o[2].min(TCP_MAX_WINSHIFT);
            }

            TCPOPT_TIMESTAMP => {
                if optlen != usize::from(TCPOLEN_TIMESTAMP) {
                    continue;
                }
                oi.ts_present = true;
                oi.ts_val = u32::from_be_bytes([o[2], o[3], o[4], o[5]]);
                oi.ts_ecr = u32::from_be_bytes([o[6], o[7], o[8], o[9]]);

                if !syn || tcps_havercvdsyn(st.t_state) {
                    continue;
                }
                // A timestamp received in a SYN makes it ok to send timestamp requests and
                // replies.
                st.set_flags(TF_RCVD_TSTMP);
                st.ts_recent = oi.ts_val;
                st.ts_recent_age = now;
            }

            TCPOPT_SACK_PERMITTED => {
                if st.sack_enable == 0
                    || optlen != usize::from(TCPOLEN_SACK_PERMITTED)
                    || !syn
                    || tcps_havercvdsyn(st.t_state)
                {
                    continue;
                }
                // MUST only be set on SYN
                st.set_flags(TF_SACK_PERMIT);
            }
            TCPOPT_SACK => tcp_sack_option(st, th, o),
            // TCP_SIGNATURE
            TCPOPT_SIGNATURE => {
                if optlen != usize::from(TCPOLEN_SIGNATURE) {
                    continue;
                }

                // A second signature option must carry the same digest (compared in constant
                // time, as the C's timingsafe_bcmp); `goto bad` holds no SA yet.
                if let Some(s) = sigp
                    && timingsafe_bcmp(s, &o[2..18])
                {
                    return false;
                }

                sigp = Some(&o[2..18]);
            }
            _ => {}
        }
    }

    // TCP_SIGNATURE: the SA's reference, given back by the hold's drop (`tdb_unref`, on the
    // `bad:` path and after the check alike).
    let mut tdb: Option<TdbHold> = None;
    if st.has_flags(TF_SIGNATURE) {
        let (src, dst) = match st.pf {
            0 | AF_INET_I32 => {
                let ip = ip_load(m);
                let sin = size_of::<SockaddrIn>();
                (
                    sig_union(sin, AF_INET, 4, &ip.ip_src.s_addr.to_ne_bytes()),
                    sig_union(sin, AF_INET, 4, &ip.ip_dst.s_addr.to_ne_bytes()),
                )
            }
            AF_INET6_I32 => {
                let ip6 = ip6_load(m);
                let sin6 = size_of::<SockaddrIn6>();
                (
                    sig_union(sin6, AF_INET6, 8, &ip6.ip6_src.s6_addr),
                    sig_union(sin6, AF_INET6, 8, &ip6.ip6_dst.s6_addr),
                )
            }
            _ => ([0; SOCKADDR_UNION_LEN], [0; SOCKADDR_UNION_LEN]),
        };

        tdb = ipsec().and_then(|legs| {
            legs.tcp_signature_tdb(rtable_l2(rtableid), &src, &dst)
                .map(|t| TdbHold::new(legs, t))
        });

        // We don't have an SA for this peer, so we turn off TF_SIGNATURE on the listen
        // socket
        if tdb.is_none() && st.t_state == TCPS_LISTEN {
            st.clear_flags(TF_SIGNATURE);
        }
    }

    // A signature where none is expected, or none where one is: refused.
    if sigp.is_some() != st.has_flags(TF_SIGNATURE) {
        tcpstat_inc(TcpstatCounters::TcpsRcvbadsig);
        return false;
    }

    if let Some(sigp) = sigp {
        let Some(t) = &tdb else {
            tcpstat_inc(TcpstatCounters::TcpsRcvbadsig);
            return false;
        };

        let mut sig = [0u8; MD5_DIGEST_LENGTH];
        if !tcp_signature(t.tdb(), st.pf, m, th, iphlen, true, &mut sig) {
            return false;
        }

        // The digest compared in constant time (timingsafe_bcmp), never with `==`.
        if timingsafe_bcmp(&sig, sigp) {
            tcpstat_inc(TcpstatCounters::TcpsRcvbadsig);
            return false;
        }

        tcpstat_inc(TcpstatCounters::TcpsRcvgoodsig);
    }

    // tdb_unref(tdb): the hold's drop.
    true
}

/// `tcp_pulloutofband`: pull out of band byte out of a segment so it doesn't appear in the
/// user's data queue. It is still reflected in the segment length for sequencing purposes.
/// The byte `urgent - 1` past `off` of the chain `m` is kept in the control block whose state
/// is `st` (`sototcpcb(so)`, the segment's: the caller's) and taken out of its mbuf's data.
pub fn tcp_pulloutofband(st: &mut TcpState, urgent: u32, m: &mut Mbuf, off: i32) {
    // `off + urgent - 1` in the C's `int`.
    let mut cnt = off.wrapping_add(urgent as i32).wrapping_sub(1);
    let mut m = m;

    while cnt >= 0 {
        let len = m.m_len().get() as i32;
        if len > cnt {
            let at = cnt as usize;
            // A read-only cluster (one another mbuf shares) gets a copy of its own before the
            // bytes move: m_copyback of the byte over itself (the C moves them in the shared
            // one).
            if m.data_mut().is_none() {
                let b = m.data()[at];
                if m_copyback(m, cnt, &[b], M_DONTWAIT).is_err() {
                    // No cluster for the copy: the byte stays in the data (see the
                    // deviations).
                    return;
                }
            }
            let Some(d) = m.data_mut() else {
                kpanic(format_args!("tcp_pulloutofband: the copy is read-only"));
            };
            st.t_iobc = d[at];
            st.t_oobflags |= TCPOOB_HAVEDATA;
            d.copy_within(at + 1.., at);
            m.m_len().set(m.m_len().get() - 1);
            return;
        }
        cnt -= len;
        m = match m.next_mut() {
            Some(n) => n,
            None => break,
        };
    }
    kpanic(format_args!("tcp_pulloutofband"));
}

/// `tcp_xmit_timer`: collect new round-trip time estimate and update averages and current
/// timeout.
pub fn tcp_xmit_timer(st: &mut TcpState, rtt: i32) {
    let rtt = rtt.clamp(0, TCP_RTT_MAX);

    tcpstat_inc(TcpstatCounters::TcpsRttupdated);
    if st.t_srtt != 0 {
        // delta is fixed point with 2 (TCP_RTT_BASE_SHIFT) bits after the binary point
        // (scaled by 4), whereas srtt is stored as fixed point with 5 bits after the binary
        // point (i.e., scaled by 32). The following magic is equivalent to the smoothing
        // algorithm in rfc793 with an alpha of .875 (srtt = rtt/8 + srtt*7/8 in fixed
        // point).
        let mut delta = (rtt << TCP_RTT_BASE_SHIFT).wrapping_sub(st.t_srtt >> TCP_RTT_SHIFT);
        st.t_srtt = st.t_srtt.wrapping_add(delta);
        if st.t_srtt <= 0 {
            st.t_srtt = 1 << TCP_RTT_BASE_SHIFT;
        }
        // We accumulate a smoothed rtt variance (actually, a smoothed mean difference), then
        // set the retransmit timer to smoothed rtt + 4 times the smoothed variance. rttvar is
        // stored as fixed point with 4 bits after the binary point (scaled by 16). The
        // following is equivalent to rfc793 smoothing with an alpha of .75 (rttvar =
        // rttvar*3/4 + |delta| / 4). This replaces rfc793's wired-in beta.
        if delta < 0 {
            delta = delta.wrapping_neg();
        }
        delta = delta.wrapping_sub(st.t_rttvar >> TCP_RTTVAR_SHIFT);
        st.t_rttvar = st.t_rttvar.wrapping_add(delta);
        if st.t_rttvar <= 0 {
            st.t_rttvar = 1 << TCP_RTT_BASE_SHIFT;
        }
    } else {
        // No rtt measurement yet - use the unsmoothed rtt. Set the variance to half the rtt
        // (so our first retransmit happens at 3*rtt).
        st.t_srtt = (rtt + 1) << (TCP_RTT_SHIFT + TCP_RTT_BASE_SHIFT);
        st.t_rttvar = (rtt + 1) << (TCP_RTTVAR_SHIFT + TCP_RTT_BASE_SHIFT - 1);
    }
    st.t_rtttime = 0;
    st.t_rxtshift = 0;

    // the retransmit should happen at rtt + 4 * rttvar. Because of the way we do the
    // smoothing, srtt and rttvar will each average +1/2 tick of bias. When we compute the
    // retransmit timer, we want 1/2 tick of rounding and 1 extra tick because of +-1/2 tick
    // uncertainty in the firing of the timer. The bias will give us exactly the 1.5 tick we
    // need. But, because the bias is statistical, we have to test that we don't drop below
    // the minimum feasible timer (which is 2 ticks).
    // libkern's `max` and `min` are `u_int` functions; the result is assigned to an `int`.
    let floor = rtt.wrapping_add(2 * (tcp_time(1) / hz())) as u32;
    let rttmin = st.t_rttmin.max(floor).min(TCPTV_REXMTMAX as u32) as i32;
    st.t_rxtcur = tcpt_rangeset(tcp_rexmtval(st), rttmin, TCPTV_REXMTMAX);

    // We received an ack for a packet that wasn't retransmitted; it is probably safe to
    // discard any error indications we've received recently. This isn't quite right, but
    // close enough for now (a route might have failed after we sent a segment, and the
    // return path might not be symmetrical).
    st.t_softerror = None;
}

/// `tcp_mss`: determine a reasonable value for maxseg size. If the route is known, check
/// route for mtu. If none, use an mss that can be handled on the outgoing interface without
/// forcing IP to fragment; if bigger than an mbuf cluster (MCLBYTES), round down to nearest
/// multiple of MCLBYTES to utilize large mbufs. If no route is found, route has no mtu, or
/// the destination isn't local, use a default, hopefully conservative size (usually 512 or
/// the default IP max size, but no more than the mtu of the interface), as we can't discover
/// anything about intervening gateways or networks. We also initialize the congestion/slow
/// start window to be a single segment if the destination isn't local. While looking at the
/// routing entry, we also initialize other path-dependent parameters from pre-set or cached
/// values in the routing entry.
///
/// Also take into account the space needed for options that we send regularly. Make maxseg
/// shorter by that amount to assure that we can send maxseg amount of data even when the
/// options are present. Store the upper limit of the length of options plus data in maxopd.
///
/// NOTE: `offer == -1` indicates that the maxseg size changed due to Path MTU discovery.
pub fn tcp_mss<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, offer: i32) -> i32 {
    let mssdflt = TCP_MSSDFLT.load(Ordering::Relaxed);
    let mut mss = mssdflt;
    let mut mssopt = mssdflt;
    let thlen = size_of::<Tcphdr>() as i32;

    'out: {
        let Some(rt) = in_pcbrtentry(tp.inp, tp.net) else {
            break 'out;
        };

        let Some(ifp) = if_get(rt.rt_ifidx.get()) else {
            break 'out;
        };

        let pf = tp.st.pf;
        let iphlen = match pf {
            AF_INET_I32 => size_of::<Ip>() as i32,
            AF_INET6_I32 => size_of::<Ip6Hdr>() as i32,
            _ => unhandled_af(pf),
        };

        // if there's an mtu associated with the route and we support path MTU discovery for
        // the underlying protocol family, use it.
        let rtmtu = rt.rt_mtu().load(Ordering::Relaxed);
        // The C's `u_int` less the headers, truncated to the `int` it is assigned to.
        let if_mss = (ifp.if_mtu.get() as i32)
            .wrapping_sub(iphlen)
            .wrapping_sub(thlen);
        if rtmtu != 0 {
            // One may wish to lower MSS to take into account options, especially
            // security-related options.
            if pf == AF_INET6_I32 && rtmtu < IPV6_MMTU {
                // RFC2460 section 5, last paragraph: if path MTU is smaller than 1280, use
                // 1280 as packet size and attach fragment header.
                mss = IPV6_MMTU as i32 - iphlen - size_of::<Ip6Frag>() as i32 - thlen;
            } else {
                mss = (rtmtu as i32).wrapping_sub(iphlen).wrapping_sub(thlen);
            }
        } else if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
            mss = if_mss;
        } else if pf == AF_INET_I32 {
            if ip_mtudisc.load(Ordering::Relaxed) != 0 {
                mss = if_mss;
            }
        } else if pf == AF_INET6_I32 {
            // for IPv6, path MTU discovery is always turned on, or the node must use packet
            // size <= 1280.
            mss = if_mss;
        }

        // Calculate the value that we offer in TCPOPT_MAXSEG
        if offer != -1 {
            mssopt = if_mss.max(mssdflt);
        }
        // if_put(ifp): the kernel's if_get lends the interface without a count.
    }
    // out:
    // The current mss, t_maxseg, is initialized to the default value. If we compute a
    // smaller value, reduce the current mss. If we compute a larger value, return it for use
    // in sending a max seg size option, but don't store it for use unless we received an
    // offer at least that large from peer.
    //
    // However, do not accept offers lower than the minimum of the interface MTU and 216.
    let st = &mut *tp.st;
    if offer > 0 {
        st.t_peermss = offer as u16;
    }
    if st.t_peermss != 0 {
        // imin(mss, max(t_peermss, 216)): libkern's u_int max, then the int min.
        mss = mss.min(u32::from(st.t_peermss).max(216) as i32);
    }

    // sanity - at least max opt. space
    mss = mss.max(64);

    // maxopd stores the maximum length of data AND options in a segment; maxseg is the
    // amount of data in a normal segment. We need to store this value (maxopd) apart from
    // maxseg, because now every segment carries options and thus we normally have somewhat
    // less data in segments.
    st.t_maxopd = mss as u16;

    if st.t_flags & (TF_REQ_TSTMP | TF_NOOPT) == TF_REQ_TSTMP
        && st.t_flags & TF_RCVD_TSTMP == TF_RCVD_TSTMP
    {
        mss -= i32::from(TCPOLEN_TSTAMP_APPA);
    }
    // TCP_SIGNATURE
    if st.has_flags(TF_SIGNATURE) {
        mss -= i32::from(TCPOLEN_SIGLEN);
    }

    let do_rfc3390 = TCP_DO_RFC3390.load(Ordering::Relaxed);
    // `mss` as the `u_long` of the window computations (at least 64 - 32 here).
    let mssl = mss as u64;
    if offer == -1 {
        // mss changed due to Path MTU discovery
        st.clear_flags(TF_PMTUD_PEND);
        st.t_pmtud_mtu_sent = 0;
        st.t_pmtud_mss_acked = 0;
        if mss < i32::from(st.t_maxseg) {
            // Follow suggestion in RFC 2414 to reduce the congestion window by the ratio of
            // the old segment size to the new segment size.
            st.snd_cwnd = ((st.snd_cwnd / u64::from(st.t_maxseg)) * mssl).max(mssl);
        }
    } else if do_rfc3390 == 2 {
        // increase initial window
        st.snd_cwnd = (10 * mssl).min((2 * mssl).max(14600));
    } else if do_rfc3390 != 0 {
        // increase initial window
        st.snd_cwnd = (4 * mssl).min((2 * mssl).max(4380));
    } else {
        st.snd_cwnd = mssl;
    }

    st.t_maxseg = mss as u16;

    if offer != -1 { mssopt } else { mss }
}

/// `tcp_hdrsz`: the length of the IP and TCP headers with the options every segment of the
/// connection carries (the timestamps, the signature).
pub fn tcp_hdrsz(st: &TcpState) -> u32 {
    let mut hlen = match st.pf {
        AF_INET6_I32 => size_of::<Ip6Hdr>() as u32,
        AF_INET_I32 => size_of::<Ip>() as u32,
        _ => 0,
    };
    hlen += size_of::<Tcphdr>() as u32;

    if st.t_flags & (TF_REQ_TSTMP | TF_NOOPT) == TF_REQ_TSTMP
        && st.t_flags & TF_RCVD_TSTMP == TF_RCVD_TSTMP
    {
        hlen += u32::from(TCPOLEN_TSTAMP_APPA);
    }
    // TCP_SIGNATURE
    if st.has_flags(TF_SIGNATURE) {
        hlen += u32::from(TCPOLEN_SIGLEN);
    }
    hlen
}

/// `tcp_mss_update`: set connection variables based on the effective MSS. We are passed the
/// TCPCB for the actual connection. If we are the server, we are called by the compressed
/// state engine when the 3-way handshake is complete. If we are the client, we are called
/// when we receive the SYN,ACK from the server.
///
/// NOTE: The `t_maxseg` value must be initialized in the TCPCB before this routine is
/// called!
pub fn tcp_mss_update<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) {
    let so = tp.so;
    // `int mss`, as the `u_long` the buffer sizes are compared with.
    let mut mss = u64::from(tp.st.t_maxseg);
    let sb_max = SB_MAX_VAR.load(Ordering::Relaxed);

    if in_pcbrtentry(tp.inp, tp.net).is_none() {
        return;
    }

    let mut g = so.so_snd.lock();
    let bufsize = so.so_snd.hiwat();
    if bufsize < mss {
        drop(g);
        mss = bufsize;
        // Update t_maxseg and t_maxopd
        tcp_mss(tp, mss as i32);
    } else {
        let bufsize = bufsize.div_ceil(mss).wrapping_mul(mss).min(sb_max);
        // The C ignores the result: a refused size leaves the buffer as it was.
        let _ = g.sbreserve(bufsize);
        drop(g);
    }

    let mut g = so.so_rcv.lock();
    let bufsize = so.so_rcv.hiwat();
    if bufsize > mss {
        let bufsize = bufsize.div_ceil(mss).wrapping_mul(mss).min(sb_max);
        // The C ignores the result, as above.
        let _ = g.sbreserve(bufsize);
    }
    drop(g);
}

/// `tcp_newreno_partialack`: when a partial ack arrives, force the retransmission of the next
/// unacknowledged segment. Do not clear `tp->t_dupacks`. By setting `snd_nxt` to `ti_ack`
/// (`th`'s acknowledgement, host order), this forces retransmission timer to be started
/// again.
pub fn tcp_newreno_partialack<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, th: &Tcphdr) {
    // snd_una has not been updated and the socket send buffer not yet drained of the acked
    // data, so we have to leave snd_una as it was to get the correct data offset in
    // tcp_output().
    let onxt = tp.st.snd_nxt;
    let ocwnd = tp.st.snd_cwnd;
    let acked = th.th_ack.wrapping_sub(tp.st.snd_una);

    tcp_timer_disarm(tp, TCPT_REXMT);
    tp.st.t_rtttime = 0;
    tp.st.snd_nxt = th.th_ack;
    // Set snd_cwnd to one segment beyond acknowledged offset (tp->snd_una not yet updated
    // when this function is called): the C's `u_int` sum, widened to the `u_long` window.
    tp.st.snd_cwnd = u64::from(u32::from(tp.st.t_maxseg).wrapping_add(acked));
    let _ = tcp_output(tp);
    let st = &mut *tp.st;
    st.snd_cwnd = ocwnd;
    if seq_gt(onxt, st.snd_nxt) {
        st.snd_nxt = onxt;
    }
    // Partial window deflation. Relies on fact that tp->snd_una not updated yet.
    if st.snd_cwnd > u64::from(acked) {
        st.snd_cwnd -= u64::from(acked);
    } else {
        st.snd_cwnd = 0;
    }
    st.snd_cwnd += u64::from(st.t_maxseg);
}

/// `tcp_mss_adv`: the segment size a SYN-ACK offers over the route `rt` to the peer of family
/// `af` (`syn_cache_add`'s `sc_ourmaxseg`): the interface's MTU less the headers, at least
/// `tcp_mssdflt`; `tcp_mssdflt` without a route or its interface.
pub fn tcp_mss_adv(rt: Option<&Rtentry>, af: i32) -> i32 {
    let mssdflt = TCP_MSSDFLT.load(Ordering::Relaxed);

    let Some(rt) = rt else {
        return mssdflt;
    };

    let Some(ifp) = if_get(rt.rt_ifidx.get()) else {
        return mssdflt;
    };

    let iphlen = match af {
        AF_INET_I32 => size_of::<Ip>() as i32,
        AF_INET6_I32 => size_of::<Ip6Hdr>() as i32,
        _ => unhandled_af(af),
    };
    // The C's `u_int` MTU less the headers, truncated to the `int` it is assigned to.
    let mss = (ifp.if_mtu.get() as i32)
        .wrapping_sub(iphlen)
        .wrapping_sub(size_of::<Tcphdr>() as i32);
    // if_put(ifp): the kernel's if_get lends the interface without a count.

    mss.max(mssdflt)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the kernel's tcp_input.c helpers: the round-trip estimator's fixed point
    // against values worked by hand from the C's formulas, the header size, the segment size
    // from the route and the peer's offer, the buffers rounded to it, and NewReno's partial
    // ACK.

    use super::*;
    use crate::netinet::tcp_subr::testtcp::{self, estab, last, pattern, queue, sent};
    use crate::netinet::tcp_var::TF_RCVD_TSTMP;

    #[test]
    fn xmit_timer_smooths_as_the_cs_fixed_point() {
        // (srtt, rttvar, rttmin, rtt) -> (srtt, rttvar, rxtcur), worked from tcp_xmit_timer:
        // the first sample unsmoothed (srtt = (rtt + 1) << 5, rttvar = (rtt + 1) << 3), then
        // delta = (rtt << 2) - (srtt >> 3), rttvar += |delta| - (rttvar >> 2), and the
        // timeout TCP_REXMTVAL clamped to [max(rttmin, rtt + 2 * (1000 / hz)), 64000].
        #[rustfmt::skip]
        let table: [((i32, i32, u32, i32), (i32, i32, i32)); 7] = [
            ((0, 0, 1000, 10), (352, 88, 1000)),
            ((352, 88, 1000, 20), (388, 102, 1000)),
            ((352, 88, 10, 20), (388, 102, 40)),
            // A negative sample counts as 0, one past TCP_RTT_MAX as the maximum.
            ((0, 0, 10, -5), (32, 8, 20)),
            ((0, 0, 10, TCP_RTT_MAX + 7), ((TCP_RTT_MAX + 1) << 5, (TCP_RTT_MAX + 1) << 3, 64000)),
            // A smoothed estimate driven to or below zero restarts at 1 << 2.
            ((-100, 40, 10, 0), (4, 43, 20)),
            // The floor of the timeout: the sample plus two ticks (hz 100: 20 ms), above
            // TCP_REXMTVAL's 115.
            ((3200, 80, 10, 100), (3200, 60, 120)),
        ];
        for ((srtt, rttvar, rttmin, rtt), want) in table {
            let mut st = TcpState::new();
            st.t_srtt = srtt;
            st.t_rttvar = rttvar;
            st.t_rttmin = rttmin;
            st.t_rtttime = 5;
            st.t_rxtshift = 3;
            st.t_softerror = Some(hal::sys::errno::Errno::EHOSTDOWN);
            tcp_xmit_timer(&mut st, rtt);
            assert_eq!(
                (st.t_srtt, st.t_rttvar, st.t_rxtcur),
                want,
                "srtt {srtt} rttvar {rttvar} rttmin {rttmin} rtt {rtt}"
            );
            assert_eq!((st.t_rtttime, st.t_rxtshift), (0, 0));
            assert_eq!(st.t_softerror, None, "the soft error forgotten");
        }
    }

    #[test]
    fn hdrsz_counts_the_options_every_segment_carries() {
        let mut st = TcpState::new();
        for (pf, flags, want) in [
            (AF_INET_I32, 0, 40),
            (AF_INET6_I32, 0, 60),
            (0, 0, 20),
            (AF_INET_I32, TF_REQ_TSTMP, 40),
            (AF_INET_I32, TF_REQ_TSTMP | TF_RCVD_TSTMP, 52),
            (AF_INET_I32, TF_REQ_TSTMP | TF_RCVD_TSTMP | TF_NOOPT, 40),
            (
                AF_INET6_I32,
                TF_REQ_TSTMP | TF_RCVD_TSTMP | TF_SIGNATURE,
                92,
            ),
        ] {
            st.pf = pf;
            st.t_flags = flags;
            assert_eq!(tcp_hdrsz(&st), want, "pf {pf} flags {flags:#x}");
        }
    }

    #[test]
    fn mss_takes_the_route_and_clamps_the_peers_offer() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tin0", 40);
        let got = testtcp::with_tp(&inp, |mut tp| {
            let mut v = std::vec::Vec::new();
            // No offer yet: the interface's 1500 less 40, offered and used; rfc3390=2's
            // initial window min(10 * mss, max(2 * mss, 14600)).
            tp.st.t_peermss = 0;
            v.push((
                tcp_mss(&mut tp, 0),
                tp.st.t_maxseg,
                tp.st.t_maxopd,
                tp.st.snd_cwnd,
            ));
            // The peer offers 536: used, our offer stays 1460.
            v.push((
                tcp_mss(&mut tp, 536),
                tp.st.t_maxseg,
                tp.st.t_maxopd,
                tp.st.snd_cwnd,
            ));
            // An offer below 216 counts as 216.
            v.push((
                tcp_mss(&mut tp, 100),
                tp.st.t_maxseg,
                tp.st.t_maxopd,
                tp.st.snd_cwnd,
            ));
            // Timestamps on both sides take 12 bytes of every segment.
            tp.st.t_peermss = 0;
            tp.st.set_flags(TF_REQ_TSTMP | TF_RCVD_TSTMP);
            v.push((
                tcp_mss(&mut tp, 0),
                tp.st.t_maxseg,
                tp.st.t_maxopd,
                tp.st.snd_cwnd,
            ));
            (v, None)
        });
        assert_eq!(
            got,
            [
                (1460, 1460, 1460, 14600),
                (1460, 536, 536, 5360),
                (1460, 216, 216, 2160),
                (1460, 1448, 1460, 14480),
            ]
        );
        // A path MTU change (offer -1): the window cut by the ratio, the notice forgotten.
        let got = testtcp::with_tp(&inp, |mut tp| {
            let st = &mut *tp.st;
            st.t_flags = 0;
            st.t_maxseg = 1460;
            st.snd_cwnd = 10 * 1460;
            st.t_peermss = 1000;
            st.set_flags(TF_PMTUD_PEND);
            st.t_pmtud_mtu_sent = 1500;
            let r = tcp_mss(&mut tp, -1);
            let st = &*tp.st;
            (
                (
                    r,
                    st.t_maxseg,
                    st.snd_cwnd,
                    st.has_flags(TF_PMTUD_PEND),
                    st.t_pmtud_mtu_sent,
                ),
                None,
            )
        });
        assert_eq!(got, (1000, 1000, 10 * 1000, false, 0));
        testtcp::close(so, inp);
    }

    #[test]
    fn mss_update_rounds_the_buffers_to_the_segment() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tin1", 41);
        testtcp::with_tp(&inp, |mut tp| {
            tcp_mss_update(&mut tp);
            ((), None)
        });
        // 16384 rounded up to a multiple of 1460.
        assert_eq!(so.so_snd.hiwat(), 12 * 1460);
        assert_eq!(so.so_rcv.hiwat(), 12 * 1460);
        testtcp::close(so, inp);
    }

    #[test]
    fn a_partial_ack_resends_from_it_and_deflates_the_window() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tin2", 42);
        let data = pattern(3000);
        queue(&so, &data);
        let before = sent(d);
        let (nxt, cwnd) = testtcp::with_tp(&inp, |mut tp| {
            let st = &mut *tp.st;
            st.snd_nxt = 4000;
            st.snd_max = 4000;
            st.snd_cwnd = 8000;
            let th = Tcphdr {
                th_ack: 2000,
                ..Tcphdr::default()
            };
            tcp_newreno_partialack(&mut tp, &th);
            ((tp.st.snd_nxt, tp.st.snd_cwnd), None)
        });
        // One segment from the partial ACK (cwnd: one segment past it), snd_nxt back to
        // where it was, the window less the 1000 acknowledged plus one segment.
        assert_eq!(sent(d), before + 1);
        let (_ip, th, _o, payload) = last(d);
        assert_eq!(u32::from_be(th.th_seq), 2000);
        assert_eq!(payload, &data[1000..2460]);
        assert_eq!(nxt, 4000);
        assert_eq!(cwnd, 8000 - 1000 + 1460);
        testtcp::close(so, inp);
    }

    // tcp_dooptions and tcp_pulloutofband (N6.6a4).

    use hal::sys::netinet_in::{IPPROTO_TCP, InAddr};
    use hal::sys::netinet_tcp::{TCPOLEN_SACK, TH_ACK};

    use crate::crypto::md5::Md5Ctx;
    use crate::net::mbuf::{M_COPYALL, MT_DATA, MbufBox, m_clget, m_copym, m_gethdr};
    use crate::netinet::tcp_fsm::{
        TCPS_CLOSED, TCPS_ESTABLISHED, TCPS_SYN_RECEIVED, TCPS_SYN_SENT,
    };
    use crate::netinet::tcp_subr::testtcp::{SIG_KEY, SIG_NET, stat};
    use crate::netinet::tcp_var::{TCPOOB_HADDATA, TcpstatCounters as C};

    /// An IPv4 packet from `src` to `dst` carrying the TCP header `th` (network order, its
    /// `th_off` set here from `opts`), the options `opts` (a multiple of 4 bytes) and `data`,
    /// in one mbuf (a cluster when `MHLEN` is too small).
    fn seg4(src: [u8; 4], dst: [u8; 4], th: Tcphdr, opts: &[u8], data: &[u8]) -> MbufBox {
        let thlen = size_of::<Tcphdr>() + opts.len();
        assert_eq!(thlen % 4, 0, "whole option words");
        let len = 20 + thlen + data.len();
        let mut ip = Ip {
            ip_tos: 0,
            ip_len: (len as u16).to_be(),
            ip_ttl: 64,
            ip_p: IPPROTO_TCP as u8,
            ip_src: InAddr {
                s_addr: u32::from_ne_bytes(src),
            },
            ip_dst: InAddr {
                s_addr: u32::from_ne_bytes(dst),
            },
            ..Ip::default()
        };
        ip.set_ip_v(4);
        ip.set_ip_hl(5);
        let mut th = th;
        th.set_th_off((thlen / 4) as u8);
        let mut p = std::vec![0u8; len];
        assert!(hal::pod::put(&mut p, 0, &ip));
        assert!(hal::pod::put(&mut p, 20, &th));
        p[40..40 + opts.len()].copy_from_slice(opts);
        p[20 + thlen..].copy_from_slice(data);
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        if len > crate::net::mbuf::MHLEN {
            assert!(m_clget(&mut m, M_DONTWAIT, len as u32), "a cluster");
        }
        m.m_len().set(len as u32);
        m.m_pkthdr().len.set(len as i32);
        m.data_mut()
            .expect("a new mbuf is writable")
            .copy_from_slice(&p);
        m
    }

    /// The RFC 2385 digest of a segment made by [`seg4`] under [`SIG_KEY`], computed here
    /// over the bytes (an MD5 of its own, not `tcp_signature`'s): the pseudo header, the TCP
    /// header with a zero checksum, the data, the key.
    fn digest4(src: [u8; 4], dst: [u8; 4], th: &Tcphdr, thlen: usize, data: &[u8]) -> [u8; 16] {
        let mut ctx = Md5Ctx::new();
        let mut pseudo = [0u8; 12];
        pseudo[..4].copy_from_slice(&src);
        pseudo[4..8].copy_from_slice(&dst);
        pseudo[9] = IPPROTO_TCP as u8;
        pseudo[10..].copy_from_slice(&((thlen + data.len()) as u16).to_be_bytes());
        ctx.update(&pseudo);
        let mut h = [0u8; 20];
        let mut th0 = *th;
        th0.th_sum = 0;
        th0.set_th_off((thlen / 4) as u8);
        assert!(hal::pod::put(&mut h, 0, &th0));
        ctx.update(&h);
        ctx.update(data);
        ctx.update(SIG_KEY);
        ctx.finalize()
    }

    /// The header `th` (network order) as `tcp_input` leaves it: host order.
    fn host(th: &Tcphdr, thlen: usize) -> Tcphdr {
        let mut h = *th;
        h.th_seq = u32::from_be(h.th_seq);
        h.th_ack = u32::from_be(h.th_ack);
        h.th_win = u16::from_be(h.th_win);
        h.th_urp = u16::from_be(h.th_urp);
        h.set_th_off((thlen / 4) as u8);
        h
    }

    /// A header with `flags`, from port 7000 to 80.
    fn hdr(flags: u8) -> Tcphdr {
        Tcphdr {
            th_sport: 7000u16.to_be(),
            th_dport: 80u16.to_be(),
            th_seq: 5000u32.to_be(),
            th_ack: 1001u32.to_be(),
            th_flags: flags,
            th_win: 4096u16.to_be(),
            ..Tcphdr::default()
        }
    }

    /// `tcp_dooptions` of `opts` for a segment with `flags` on a block in state `state`
    /// (SACK enabled), without a signature: the state's flags and values it sets, and its
    /// answer.
    fn dooptions(state: i32, flags: u8, opts: &[u8]) -> (bool, TcpState, TcpOptInfo) {
        let mut st = TcpState::new();
        st.t_state = state;
        st.sack_enable = 1;
        st.pf = AF_INET_I32;
        let th = hdr(flags);
        let m = seg4([10, 9, 100, 2], [10, 9, 100, 1], th, &[], &[]);
        let mut oi = TcpOptInfo::default();
        let ok = tcp_dooptions(&mut st, opts, &host(&th, 20), &m, 20, &mut oi, 0, 777);
        (ok, st, oi)
    }

    const MSS1460: [u8; 4] = [TCPOPT_MAXSEG, TCPOLEN_MAXSEG, 0x05, 0xb4];
    const WS: [u8; 3] = [TCPOPT_WINDOW, TCPOLEN_WINDOW, 7];
    const SACKP: [u8; 2] = [TCPOPT_SACK_PERMITTED, TCPOLEN_SACK_PERMITTED];
    const TS: [u8; 10] = [TCPOPT_TIMESTAMP, TCPOLEN_TIMESTAMP, 0, 0, 1, 2, 0, 0, 3, 4];

    #[test]
    fn dooptions_takes_the_syn_options_before_the_syn_is_received() {
        let (_g, _p) = testtcp::setup();
        let mut opts = std::vec::Vec::new();
        for o in [&MSS1460[..], &WS, &SACKP, &TS] {
            opts.extend_from_slice(o);
        }
        for state in [TCPS_CLOSED, TCPS_LISTEN, TCPS_SYN_SENT] {
            let (ok, st, oi) = dooptions(state, TH_SYN, &opts);
            assert!(ok, "state {state}");
            assert_eq!(oi.maxseg, 1460);
            assert!(st.has_flags(TF_RCVD_SCALE) && st.requested_s_scale == 7);
            assert!(st.has_flags(TF_SACK_PERMIT));
            assert!(oi.ts_present && (oi.ts_val, oi.ts_ecr) == (0x102, 0x304));
            assert!(st.has_flags(TF_RCVD_TSTMP));
            assert_eq!((st.ts_recent, st.ts_recent_age), (0x102, 777));
        }
        // A window shift past TCP_MAX_WINSHIFT is cut to it.
        let (_, st, _) = dooptions(TCPS_LISTEN, TH_SYN, &[TCPOPT_WINDOW, 3, 200]);
        assert_eq!(st.requested_s_scale, TCP_MAX_WINSHIFT);
        // SACK permitted only when the block enables SACK.
        let mut st = TcpState::new();
        st.t_state = TCPS_LISTEN;
        let th = hdr(TH_SYN);
        let m = seg4([10, 9, 100, 2], [10, 9, 100, 1], th, &[], &[]);
        let mut oi = TcpOptInfo::default();
        assert!(tcp_dooptions(
            &mut st,
            &SACKP,
            &host(&th, 20),
            &m,
            20,
            &mut oi,
            0,
            1
        ));
        assert!(!st.has_flags(TF_SACK_PERMIT));
    }

    #[test]
    fn dooptions_ignores_the_syn_options_elsewhere() {
        let (_g, _p) = testtcp::setup();
        let mut opts = std::vec::Vec::new();
        for o in [&MSS1460[..], &WS, &SACKP, &TS] {
            opts.extend_from_slice(o);
        }
        // Once the SYN is received, or on a segment without SYN: the timestamps are taken
        // for the segment, nothing else.
        for (state, flags) in [
            (TCPS_ESTABLISHED, TH_SYN),
            (TCPS_SYN_RECEIVED, TH_SYN | TH_ACK),
            (TCPS_LISTEN, TH_ACK),
        ] {
            let (ok, st, oi) = dooptions(state, flags, &opts);
            assert!(ok);
            assert_eq!(oi.maxseg, 0, "state {state} flags {flags:#x}");
            assert_eq!(st.t_flags, 0);
            assert_eq!(st.requested_s_scale, 0);
            assert!(oi.ts_present && oi.ts_val == 0x102);
            assert_eq!(st.ts_recent, 0);
        }
    }

    #[test]
    fn dooptions_stops_at_a_short_or_malformed_option() {
        let (_g, _p) = testtcp::setup();
        // (options, the MSS taken): the walk's bounds, each from the C's loop.
        #[rustfmt::skip]
        let table: [(&[u8], u16); 12] = [
            (&[], 0),
            // NOPs are one byte each; EOL ends the block.
            (&[TCPOPT_NOP, TCPOPT_NOP, 2, 4, 0x02, 0x00], 512),
            (&[TCPOPT_EOL, 2, 4, 0x02, 0x00], 0),
            // A kind without its length byte, a length of 0 or 1, a length past the block:
            // the walk stops there.
            (&[2], 0),
            (&[TCPOPT_NOP, 2], 0),
            (&[99, 0, 2, 4, 0x02, 0x00], 0),
            (&[99, 1, 2, 4, 0x02, 0x00], 0),
            (&[2, 4, 0x02], 0),
            (&[99, 9, 2, 4, 0x02, 0x00], 0),
            // An unknown option of a valid length is skipped over.
            (&[99, 3, 0xff, 2, 4, 0x02, 0x00], 512),
            // An MSS of the wrong length is skipped (continue), the next one taken.
            (&[2, 3, 0x05, 2, 4, 0x02, 0x00], 512),
            // The last MSS wins.
            (&[2, 4, 0x02, 0x00, 2, 4, 0x05, 0xb4], 1460),
        ];
        for (opts, mss) in table {
            let (ok, _st, oi) = dooptions(TCPS_LISTEN, TH_SYN, opts);
            assert!(ok);
            assert_eq!(oi.maxseg, mss, "options {opts:?}");
        }
        // A timestamp of the wrong length is not one.
        let (_, st, oi) = dooptions(
            TCPS_LISTEN,
            TH_SYN,
            &[TCPOPT_TIMESTAMP, 9, 1, 2, 3, 4, 5, 6, 7],
        );
        assert!(!oi.ts_present && !st.has_flags(TF_RCVD_TSTMP));
    }

    /// The C's walk of an option block written again here, by indices over the whole block:
    /// each option's kind, offset and length, up to where the C stops.
    fn walk(cp: &[u8]) -> std::vec::Vec<(u8, usize, usize)> {
        let mut v = std::vec::Vec::new();
        let (mut i, mut cnt) = (0usize, cp.len() as i32);
        while cnt > 0 {
            let opt = cp[i];
            if opt == TCPOPT_EOL {
                break;
            }
            let optlen = if opt == TCPOPT_NOP {
                1
            } else {
                if cnt < 2 {
                    break;
                }
                let l = i32::from(cp[i + 1]);
                if l < 2 || l > cnt {
                    break;
                }
                l
            };
            v.push((opt, i, optlen as usize));
            cnt -= optlen;
            i += optlen as usize;
        }
        v
    }

    #[test]
    fn dooptions_never_reads_past_the_option_block() {
        let (_g, _p) = testtcp::setup();
        // A property over 20,000 random blocks (xorshift, fixed seed): the block cut at `n`
        // from a buffer whose tail beyond holds a valid MSS of 0xbeef; the walk's answer is
        // the reference walk's over the cut (the last MSS of length 4 there), so nothing past
        // `n` is ever read, and no block panics.
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..20_000 {
            let n = (next() % 41) as usize;
            let mut buf = [0u8; 44];
            for b in buf.iter_mut().take(n) {
                // Kinds and lengths near the interesting values.
                *b = match next() % 4 {
                    0 => (next() % 6) as u8,
                    1 => [2, 3, 4, 5, 8, 19][(next() % 6) as usize],
                    _ => next() as u8,
                };
            }
            buf[n..n + 4].copy_from_slice(&[2, 4, 0xbe, 0xef]);
            let cut = &buf[..n];
            let want = walk(cut)
                .iter()
                .filter(|&&(o, _, l)| o == TCPOPT_MAXSEG && l == 4)
                .last()
                .map_or(0, |&(_, i, _)| u16::from_be_bytes([cut[i + 2], cut[i + 3]]));
            let (_ok, _st, oi) = dooptions(TCPS_LISTEN, TH_SYN, cut);
            assert_eq!(oi.maxseg, want, "block {cut:?}");
        }
    }

    /// A segment of the signature tests: from 10.9.SIG_NET.2:7000 to .1:80 with `opts` and
    /// "signed", and the host-order header `tcp_input` hands `tcp_dooptions`.
    fn sig_seg(opts: &[u8]) -> (MbufBox, Tcphdr) {
        let th = hdr(TH_ACK);
        let m = seg4(
            [10, 9, SIG_NET, 2],
            [10, 9, SIG_NET, 1],
            th,
            opts,
            b"signed",
        );
        (m, host(&th, 20 + opts.len()))
    }

    /// The signature option of the digest `d`, then two NOPs (a 20-byte block).
    fn sig_opt(d: &[u8; 16]) -> [u8; 20] {
        let mut o = [TCPOPT_NOP; 20];
        o[0] = TCPOPT_SIGNATURE;
        o[1] = TCPOLEN_SIGNATURE;
        o[2..18].copy_from_slice(d);
        o
    }

    #[test]
    fn dooptions_checks_the_signature_in_constant_time() {
        let (_g, _p) = testtcp::setup();
        let legs = testtcp::sig_legs();
        let src = [10, 9, SIG_NET, 2];
        let dst = [10, 9, SIG_NET, 1];
        let good = digest4(src, dst, &hdr(TH_ACK), 40, b"signed");
        let check = |opts: &[u8], signed: bool, state: i32| {
            let (m, th) = sig_seg(opts);
            let mut st = TcpState::new();
            st.t_state = state;
            st.pf = AF_INET_I32;
            if signed {
                st.set_flags(TF_SIGNATURE);
            }
            let mut oi = TcpOptInfo::default();
            let unrefs = legs.unrefs.load(Ordering::Relaxed);
            let (bad, goodc) = (stat(C::TcpsRcvbadsig), stat(C::TcpsRcvgoodsig));
            let ok = tcp_dooptions(&mut st, opts, &th, &m, 20, &mut oi, 0, 1);
            (
                ok,
                stat(C::TcpsRcvbadsig) - bad,
                stat(C::TcpsRcvgoodsig) - goodc,
                legs.unrefs.load(Ordering::Relaxed) - unrefs,
            )
        };
        // The right digest: accepted, counted good, the SA given back.
        let (ok, bad, goodc, unrefs) = check(&sig_opt(&good), true, TCPS_ESTABLISHED);
        assert!(ok && bad == 0 && goodc >= 1 && unrefs >= 1);
        // One bit off in the first byte, or in the last: refused, counted bad, given back.
        for at in [0, 15] {
            let mut d = good;
            d[at] ^= 0x01;
            let (ok, bad, _, unrefs) = check(&sig_opt(&d), true, TCPS_ESTABLISHED);
            assert!(!ok && bad >= 1 && unrefs >= 1, "byte {at}");
        }
        // A signed connection's segment without the option, or an option where none is
        // expected: refused and counted.
        let (ok, bad, ..) = check(&[TCPOPT_NOP; 4], true, TCPS_ESTABLISHED);
        assert!(!ok && bad >= 1);
        let (ok, bad, ..) = check(&sig_opt(&good), false, TCPS_ESTABLISHED);
        assert!(!ok && bad >= 1);
        // Two signature options that differ: refused before any SA is looked up (the C's
        // `goto bad`, uncounted); the same digest twice is one signature.
        let mut two = [TCPOPT_NOP; 36];
        two[..18].copy_from_slice(&sig_opt(&good)[..18]);
        two[18..36].copy_from_slice(&sig_opt(&good)[..18]);
        let goodsig = digest4(src, dst, &hdr(TH_ACK), 56, b"signed");
        two[2..18].copy_from_slice(&goodsig);
        two[20..36].copy_from_slice(&goodsig);
        let (ok, ..) = check(&two, true, TCPS_ESTABLISHED);
        assert!(ok, "the same digest twice");
        let mut d = goodsig;
        d[7] ^= 0x80;
        two[20..36].copy_from_slice(&d);
        let (ok, _, _, unrefs) = check(&two, true, TCPS_ESTABLISHED);
        assert!(!ok);
        assert_eq!(unrefs, 0, "no SA taken");
    }

    #[test]
    fn dooptions_turns_the_signature_off_on_a_listener_without_an_sa() {
        let (_g, _p) = testtcp::setup();
        let _legs = testtcp::sig_legs();
        // Between addresses the legs have no SA for: a listener's TF_SIGNATURE goes, the
        // unsigned SYN is accepted; an established connection keeps it and refuses.
        let th = hdr(TH_SYN);
        let m = seg4([10, 9, 100, 2], [10, 9, 100, 1], th, &[], &[]);
        for (state, ok_want, flag_want) in
            [(TCPS_LISTEN, true, false), (TCPS_ESTABLISHED, false, true)]
        {
            let mut st = TcpState::new();
            st.t_state = state;
            st.pf = AF_INET_I32;
            st.set_flags(TF_SIGNATURE);
            let mut oi = TcpOptInfo::default();
            let ok = tcp_dooptions(&mut st, &[], &host(&th, 20), &m, 20, &mut oi, 0, 1);
            assert_eq!((ok, st.has_flags(TF_SIGNATURE)), (ok_want, flag_want));
        }
    }

    #[test]
    fn dooptions_feeds_the_sack_blocks_to_the_scoreboard() {
        let (_g, _p) = testtcp::setup();
        let mut st = TcpState::new();
        st.t_state = TCPS_ESTABLISHED;
        st.pf = AF_INET_I32;
        st.sack_enable = 1;
        st.t_maxseg = 1000;
        st.snd_una = 1000;
        st.snd_max = 5000;
        st.snd_nxt = 5000;
        let mut opts = std::vec![TCPOPT_NOP, TCPOPT_NOP, TCPOPT_SACK, TCPOLEN_SACK + 2];
        opts.extend_from_slice(&2000u32.to_be_bytes());
        opts.extend_from_slice(&3000u32.to_be_bytes());
        let mut th = hdr(TH_ACK);
        th.th_ack = 1000u32.to_be();
        let m = seg4([10, 9, 100, 2], [10, 9, 100, 1], th, &opts, &[]);
        let mut oi = TcpOptInfo::default();
        assert!(tcp_dooptions(
            &mut st,
            &opts,
            &host(&th, 32),
            &m,
            20,
            &mut oi,
            0,
            1
        ));
        let h = st.snd_holes.get().expect("a hole");
        assert_eq!((h.start, h.end), (1000, 2000));
        assert_eq!(st.snd_numholes, 1);
    }

    /// A chain of two mbufs: "0123456789" then "abcdefghij".
    fn chain() -> MbufBox {
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(10);
        m.m_pkthdr().len.set(20);
        m.data_mut()
            .expect("writable")
            .copy_from_slice(b"0123456789");
        let mut n = crate::net::mbuf::m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        n.m_len().set(10);
        n.data_mut()
            .expect("writable")
            .copy_from_slice(b"abcdefghij");
        assert!(m.m_next.set(n).is_none());
        m
    }

    /// The bytes of the chain `m`, mbuf by mbuf.
    fn bytes(m: &Mbuf) -> std::vec::Vec<std::vec::Vec<u8>> {
        let mut v = std::vec![m.data().to_vec()];
        let mut n = m.m_next().get();
        while let Some(x) = n {
            v.push(x.data().to_vec());
            n = x.m_next().get();
        }
        v
    }

    #[test]
    fn pulloutofband_takes_the_byte_out_of_its_mbuf() {
        let (_g, _p) = testtcp::setup();
        // (off, urgent): the byte at off + urgent - 1, in the first mbuf or the second.
        for (off, urgent, iobc, want) in [
            (2, 3, b'4', [&b"012356789"[..], b"abcdefghij"]),
            (0, 1, b'0', [&b"123456789"[..], b"abcdefghij"]),
            (5, 5, b'9', [&b"012345678"[..], b"abcdefghij"]),
            (8, 6, b'd', [&b"0123456789"[..], b"abcefghij"]),
            (10, 10, b'j', [&b"0123456789"[..], b"abcdefghi"]),
        ] {
            let mut m = chain();
            let mut st = TcpState::new();
            st.t_oobflags = TCPOOB_HADDATA;
            tcp_pulloutofband(&mut st, urgent, &mut m, off);
            assert_eq!(st.t_iobc, iobc, "off {off} urgent {urgent}");
            assert_eq!(st.t_oobflags, TCPOOB_HADDATA | TCPOOB_HAVEDATA);
            assert_eq!(bytes(&m), want);
            // The packet's length is not the C's to change: m_len alone.
            assert_eq!(m.m_pkthdr().len.get(), 20);
        }
    }

    #[test]
    fn pulloutofband_copies_a_shared_cluster_first() {
        let (_g, _p) = testtcp::setup();
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        assert!(m_clget(&mut m, M_DONTWAIT, 300));
        let data: std::vec::Vec<u8> = (0..300).map(|i| i as u8).collect();
        m.m_len().set(300);
        m.m_pkthdr().len.set(300);
        m.data_mut().expect("writable").copy_from_slice(&data);
        let mut copy = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT).expect("a copy");
        assert!(copy.data_mut().is_none(), "the cluster is shared");
        let mut st = TcpState::new();
        tcp_pulloutofband(&mut st, 101, &mut copy, 0);
        assert_eq!(st.t_iobc, 100);
        let mut want = data.clone();
        want.remove(100);
        assert_eq!(copy.data(), &want[..]);
        assert_eq!(m.data(), &data[..], "the other mbuf's data untouched");
    }
}
/* </TESTS> */
