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
//! `tcp_input` itself, its options and the SYN cache's add and get follow at N6.6a4).
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
use hal::sys::netinet_ip::Ip;
use hal::sys::netinet_ip6::{IPV6_MMTU, Ip6Frag, Ip6Hdr};
use hal::sys::netinet_tcp::{TCPOLEN_SIGLEN, TCPOLEN_TSTAMP_APPA, Tcphdr};
use hal::sys::socket::{AF_INET, AF_INET6};

use crate::net::if_::{if_get, unhandled_af};
use crate::net::sockbuf::SB_MAX_VAR;
use crate::netinet::in_pcb::in_pcbrtentry;
use crate::netinet::ip_var::ip_mtudisc;
use crate::netinet::tcp_output::tcp_output;
use crate::netinet::tcp_seq::seq_gt;
use crate::netinet::tcp_subr::{TCP_DO_RFC3390, TCP_MSSDFLT};
use crate::netinet::tcp_timer::{TCPT_REXMT, TCPTV_REXMTMAX, tcp_timer_disarm, tcpt_rangeset};
use crate::netinet::tcp_var::{
    TCP_RTT_BASE_SHIFT, TCP_RTT_MAX, TCP_RTT_SHIFT, TCP_RTTVAR_SHIFT, TF_NOOPT, TF_PMTUD_PEND,
    TF_RCVD_TSTMP, TF_REQ_TSTMP, TF_SIGNATURE, TcpState, TcpstatCounters, Tp, tcp_rexmtval,
    tcp_time, tcpstat_inc,
};
use crate::proc::ProcGlue;
use crate::timeout::hz;

/// `AF_INET` as the `int` of `tp->pf`.
const AF_INET_I32: i32 = AF_INET as i32;
/// `AF_INET6` as the `int` of `tp->pf`.
const AF_INET6_I32: i32 = AF_INET6 as i32;

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
}
/* </TESTS> */
