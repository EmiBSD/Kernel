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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
//! TCP output: `netinet/tcp_output.c` (N6.6a3; its TSO cut and hand-off are
//! [`crate::netinet::tcp_tso`]'s since N6.6a1).
//!
//! Upstream: sys/netinet/tcp_output.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_output.rs@3c62ede68802
//!
//! [`tcp_output`] figures out what a connection should send (new data within the send and
//! congestion windows, a SACK retransmission, a window update, an ACK, a SYN, FIN or RST)
//! and sends it: the IP and TCP headers come from the connection's template
//! (`t_template`, an `ipovly` or an IPv6 header, and a `tcphdr`), the options (MSS, SACK
//! permitted, window scale, timestamps, the TCP MD5 signature, SACK blocks) are built in a
//! local buffer as in C, the data is copied out of the send buffer, and `ip_output` or
//! `ip6_output` takes the packet; the TCP checksum is left to the interface or
//! `in_proto_cksum_out`. A large send may be handed down as one TSO packet. [`tcp_setpersist`]
//! (re)starts the persist timer, [`tcp_sack_output`] and [`tcp_sack_adjust`] walk the
//! sender's SACK holes.
//!
//! ## Deviations
//! - `tcp_output` returns `Result<(), Errno>`; the C's error tests (`error == ENOBUFS`,
//!   `EMSGSIZE`, `EHOSTUNREACH`/`ENETDOWN`) are a `match` after the IP layer's answer, the
//!   `out:` label. The `goto send` decision is a labelled block, `goto again` the function's
//!   loop (one pass of it is `tcp_output_pass`), `goto timer` an `if` around the code it
//!   skips.
//! - `sack_rxmit` and `p` (the hole being retransmitted) are one `Option<&mut Sackhole>`: the
//!   C sets the flag exactly when it sets the pointer, and reads the pointer only under it.
//! - The TCP header is a local [`Tcphdr`] read from the copied template and written back into
//!   the packet once its fields are final, before the signature, the trace and the IP layer
//!   read the packet; the options are a `[u8; MAX_TCPOPTLEN]` written big-endian (the C's
//!   `htonl` through a `u_int32_t *`), each write checked against the buffer
//!   (`panic(9)` "tcp_output: options too long", the C's `DIAGNOSTIC` message, where the C
//!   would write past its stack buffer: a signed connection never permits SACK, so the 40
//!   bytes hold every option it sends, the C's own reliance, `tcp_ctloutput`'s two refusals).
//! - `tcp_signature` writes the digest into a local array, copied into the option (the C
//!   hands it a pointer into the packet); the digest does not cover the options, so the bytes
//!   it reads are the same.
//! - `inp_options` and `inp_outputopts6` share one union slot in the C, whose TSO test reads
//!   both names of the one word; here they are two cells, and the test reads the one of the
//!   pcb's family (an IPv4 pcb's IPv6 options and an IPv6 pcb's IPv4 options are never set:
//!   `ip_ctloutput` and `ip6_ctloutput` are reached by family), after the test's cheaper terms.
//! - `ip6_plen` is stored in host order, as the C stores it (`ip6_output` writes it again).
//! - The refusal of a signed connection that permits SACK (`EINVAL`), which the C compiles
//!   only under `TCP_SIGNATURE && DIAGNOSTIC`, is made in every build, following GENERIC's
//!   `DIAGNOSTIC` build (conf/GENERIC:12): `EINVAL` is what OpenBSD's userland sees; without
//!   the check such a SYN's options would pass the 40 bytes (the C writes past its buffer,
//!   here the bounded writes' `panic(9)`). `tcp_ctloutput` keeps the pair from being made.
//! - `TCP_SACK_DEBUG` (not in GENERIC) is the cargo feature `tcp_sack_debug`:
//!   `tcp_print_holes` and its call in [`tcp_sack_output`]. `TCPDEBUG` is not configured
//!   (the `SO_DEBUG` trace calls `tcp_trace`, as the C does without it). Configured:
//!   `TCP_ECN`, `TCP_SIGNATURE` (the SA through IPsec's leg), `NPF` (`pf_mbuf_link_inpcb`,
//!   pf's leg), `NSTOEPLITZ` (the flow id). `DIAGNOSTIC` is the feature `diagnostic`.
//!
//! ## Redesign
//! - N6.6a3: `tcp_output` is the kernel's, over the kernel's control block held as [`Tp`]
//!   (the socket lock's parts and the net lock's proof its caller holds): no lend of the net
//!   lock per segment (`bsd`'s `ip_output` adapter lends it through the `SaToken`, a
//!   contended pair per packet; here `tp.net` is the entry's own proof), the header written
//!   with `hal::pod` into the new mbuf's data (LZ: unaligned stores under `unsafe`), the
//!   template read from the control block's own mbuf (LZ: a parked `&'static Mbuf`). `bsd`'s
//!   `sys/netinet/tcp_output.rs` stays for `bsd`'s TCP until the switch (N6.6a5).
//! - The SACK holes are an owned chain in the control block's state: for one pass the chain
//!   leaves the state and comes back (two word moves), so the hole being retransmitted is a
//!   `&mut` beside the control block for the whole pass, the C's `p`, found once by
//!   [`tcp_sack_output`] (no second walk), and the functions the pass calls (the timers,
//!   `tcp_mss`, `tcp_setpersist`) take the control block as they stand. The chain is back
//!   before the IP layer's answer is handled, so `tcp_mtudisc`'s own `tcp_output` sees it.
//! - Lock and atomic counts per segment against the C's: the send buffer's mutex once (the
//!   copy; the C copies under the socket lock alone, and reads `soissending` under the mutex,
//!   an atomic read here: the same count of mutex sections), the receive buffer's mutex once
//!   (the C's), `tcp_update_sndspace`'s once (the C's); the route cache's loan (two atomics,
//!   the C reads the pointer) and, for IPv4, the options' loan (two more), the pcb's `[s]`
//!   cells until N6.6b re-keys them to the socket lock (docs/ROADMAP.md's N12 row); a TSO
//!   candidate's options test one more loan. pf's link is one dynamic call (its count is
//!   `bsd`'s). The timers' arm and disarm take the wheel's lock as the C's `timeout_mutex`,
//!   with no count of the pcb (D-11).

use core::cmp::min;
use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::owned::SlistLink;
use hal::pod;
use hal::sys::endian::{htonl, htons};
use hal::sys::errno::Errno;
use hal::sys::mbuf::MT_HEADER;
use hal::sys::netinet_in::IPPROTO_TCP;
use hal::sys::netinet_ip::{IP_MAXPACKET, IPTOS_ECN_ECT0, Ip};
use hal::sys::netinet_ip6::Ip6Hdr;
use hal::sys::netinet_tcp::{
    MAX_TCPOPTLEN, TCP_MAX_SACK, TCP_MAXWIN, TCPOLEN_SACK, TCPOLEN_SIGLEN, TCPOLEN_SIGNATURE,
    TCPOLEN_TSTAMP_APPA, TCPOLEN_WINDOW, TCPOPT_MAXSEG, TCPOPT_NOP, TCPOPT_SACK_HDR,
    TCPOPT_SACK_PERMIT_HDR, TCPOPT_SIGNATURE, TCPOPT_TSTAMP_HDR, TCPOPT_WINDOW, TH_ACK, TH_CWR,
    TH_ECE, TH_FIN, TH_PUSH, TH_RST, TH_SYN, TH_URG, Tcphdr,
};
use hal::sys::socket::{AF_INET, AF_INET6, SO_DEBUG};

use crate::assert::{kassert, kpanic};
use crate::crypto::md5::MD5_DIGEST_LENGTH;
use crate::net::ipsec_hooks::{SOCKADDR_UNION_LEN, TdbHold, ipsec};
use crate::net::mbuf::{
    M_DONTWAIT, M_FLOWID, M_PKTHDR, M_TCP_CSUM_OUT, M_TCP_TSO, MAX_LINKHDR, MAXMCLBYTES, MCLBYTES,
    MHLEN, Mbuf, MbufBox, m_clget, m_gethdr, m_trailingspace,
};
use crate::net::pf_hooks::pf;
use crate::net::rtable::rtable_l2;
use crate::net::socket::soissending;
use crate::netinet::in_pcb::{INP_IPV6, Inpcb};
use crate::netinet::ip_output::ip_output;
use crate::netinet::ip_var::{IP_MTUDISC, ip_mtudisc};
use crate::netinet::tcp_debug::{TA_OUTPUT, tcp_trace};
use crate::netinet::tcp_fsm::{TCP_OUTFLAGS, TCPS_ESTABLISHED, tcps_havercvdsyn};
use crate::netinet::tcp_input::{tcp_mss, tcp_mss_update};
use crate::netinet::tcp_sack::TCPREXMTTHRESH;
use crate::netinet::tcp_seq::{seq_geq, seq_gt, seq_lt};
use crate::netinet::tcp_subr::{TCP_DO_ECN, TCP_DO_TSO, put_hdr, tcp_mtudisc, tcp_signature};
use crate::netinet::tcp_timer::{
    TCP_BACKOFF, TCP_MAXRXTSHIFT, TCPT_DELACK, TCPT_PERSIST, TCPT_REXMT, TCPTV_PERSMAX,
    TCPTV_PERSMIN, tcp_delack_msecs, tcp_timer_arm, tcp_timer_disarm, tcp_timer_isarmed,
    tcpt_rangeset,
};
use crate::netinet::tcp_usrreq::tcp_update_sndspace;
use crate::netinet::tcp_var::{
    Sackhole, TCP_RTT_BASE_SHIFT, TF_ACKNOW, TF_DISABLE_ECN, TF_ECN_PERMIT, TF_LASTIDLE,
    TF_NODELAY, TF_NOOPT, TF_NOPUSH, TF_RCVD_CE, TF_RCVD_SCALE, TF_RCVD_TSTMP, TF_REQ_SCALE,
    TF_REQ_TSTMP, TF_SACK_PERMIT, TF_SEND_CWR, TF_SENTFIN, TF_SIGNATURE, TcpState, Tcpcb,
    TcpstatCounters, Tp, tcp_now, tcpstat_inc, tcpstat_pkt,
};
use crate::netinet6::in6_src::in6_selecthlim;
use crate::netinet6::ip6_output::ip6_output;
use crate::netinet6::ip6_var::Ip6Pktopts;
use crate::proc::ProcGlue;

/// `AF_INET` as the `int` of `tp->pf`.
const AF_INET_I32: i32 = AF_INET as i32;
/// `AF_INET6` as the `int` of `tp->pf`.
const AF_INET6_I32: i32 = AF_INET6 as i32;

/// What one pass of `tcp_output` (from its `again:` label to the IP layer) leaves for the
/// loop.
enum Pass {
    /// No segment: the C's `return` with this value before one was made.
    Done(Result<(), Errno>),
    /// A segment was handed to the IP layer, or could not be made (`goto out`): the result,
    /// the window it advertised (`win`), its length (`packetlen`) and whether to go again
    /// (`sendalot`).
    Sent {
        /// The IP layer's answer, or `ENOBUFS` without an mbuf.
        error: Result<(), Errno>,
        /// `win`: the receive window advertised.
        win: i64,
        /// `packetlen`: the IP packet's length.
        packetlen: u32,
        /// `sendalot`.
        sendalot: bool,
    },
}

/// `tcp_print_holes` (`TCP_SACK_DEBUG`): prints the sender's SACK holes.
#[cfg(feature = "tcp_sack_debug")]
pub fn tcp_print_holes(holes: &SlistLink<Sackhole>) {
    let mut p = holes.get();
    if p.is_none() {
        return;
    }
    crate::prf::printf(format_args!("Hole report: start--end dups rxmit\n"));
    while let Some(h) = p {
        crate::prf::printf(format_args!(
            "{:x}--{:x} d {} r {:x}\n",
            h.start, h.end, h.dups, h.rxmit
        ));
        p = h.next.get();
    }
    crate::prf::printf(format_args!("\n"));
}

/// `tcp_sack_output`: the first SACK hole with pending retransmissions (enough duplicate
/// ACKs, data left to resend at or past `snd_una`), or `None`. `holes` is the control block's
/// list (`st`'s `snd_holes`, out of `st` for `tcp_output`'s pass).
pub fn tcp_sack_output<'h>(
    st: &TcpState,
    holes: &'h mut SlistLink<Sackhole>,
) -> Option<&'h mut Sackhole> {
    if st.sack_enable == 0 {
        return None;
    }
    // TCP_SACK_DEBUG: the holes printed when one is returned (a shared walk first: the
    // answer is a `&mut` into the list).
    #[cfg(feature = "tcp_sack_debug")]
    {
        let mut q = holes.get();
        while let Some(h) = q {
            if sack_pending(st, h) {
                tcp_print_holes(holes);
                break;
            }
            q = h.next.get();
        }
    }
    let mut link = holes;
    loop {
        let p = link.get_mut()?;
        if sack_pending(st, p) {
            return Some(p);
        }
        link = &mut p.next;
    }
}

/// Whether hole `p` has data to retransmit: enough duplicate ACKs, `rxmit` short of its end,
/// and not an old SACK hole (`rxmit` below `snd_una`, which the C's walk skips).
#[inline]
fn sack_pending(st: &TcpState, p: &Sackhole) -> bool {
    p.dups >= TCPREXMTTHRESH && seq_lt(p.rxmit, p.end) && !seq_lt(p.rxmit, st.snd_una)
}

/// `tcp_sack_adjust`: after a timeout, the SACK list may be rebuilt. This SACK information
/// should be used to avoid retransmitting SACKed data. This function traverses the SACK list
/// (`holes`, the control block's) to see if `snd_nxt` should be moved forward.
pub fn tcp_sack_adjust(st: &mut TcpState, holes: &SlistLink<Sackhole>) {
    let Some(mut cur) = holes.get() else {
        return; // No holes
    };
    if seq_geq(st.snd_nxt, st.rcv_lastsack) {
        return; // We're already beyond any SACKed blocks
    }
    // Two cases for which we want to advance snd_nxt:
    // i) snd_nxt lies between end of one hole and beginning of another
    // ii) snd_nxt lies between end of last hole and rcv_lastsack
    while let Some(next) = cur.next.get() {
        if seq_lt(st.snd_nxt, cur.end) {
            return;
        }
        if seq_geq(st.snd_nxt, next.start) {
            cur = next;
        } else {
            st.snd_nxt = next.start;
            return;
        }
    }
    if seq_lt(st.snd_nxt, cur.end) {
        return;
    }
    st.snd_nxt = st.rcv_lastsack;
}

/// Stores `v` in network order at byte `off` of the option buffer (the C's
/// `*(u_int32_t *)(opt + off) = htonl(v)`).
fn opt_put32(opt: &mut [u8; MAX_TCPOPTLEN], off: usize, v: u32) {
    opt_put(opt, off, &v.to_be_bytes());
}

/// Writes `b` at byte `off` of the option buffer; `panic(9)` past its 40 bytes (the C's
/// `DIAGNOSTIC` check, made after it wrote past the buffer).
fn opt_put(opt: &mut [u8; MAX_TCPOPTLEN], off: usize, b: &[u8]) {
    match opt.get_mut(off..off + b.len()) {
        Some(d) => d.copy_from_slice(b),
        None => kpanic(format_args!("tcp_output: options too long")),
    }
}

/// `MGETHDR(m, M_DONTWAIT, MT_HEADER)`, with a cluster when the link, IP and TCP headers do
/// not fit in `MHLEN`, its data moved past `max_linkhdr` and `m_len` set to `hdrlen`; `None`
/// when out of mbufs (the C's two copies of this sequence in `tcp_output`).
fn tcp_output_gethdr(max_linkhdr: u32, hdrlen: u32) -> Option<MbufBox> {
    let mut m = m_gethdr(M_DONTWAIT, MT_HEADER)?;
    if (max_linkhdr + hdrlen) as usize > MHLEN && !m_clget(&mut m, M_DONTWAIT, MCLBYTES as u32) {
        return None;
    }
    m.data_add(max_linkhdr as isize);
    m.m_len().set(hdrlen);
    Some(m)
}

/// Whether `inp` has no IP options its family's output would insert (the C's TSO test of
/// `inp_options` and `inp_outputopts6`, one union slot): the IPv6 options of an IPv6 pcb, the
/// IPv4 options of another.
fn no_ip_options<G: ProcGlue>(inp: &Inpcb<G>) -> bool {
    if inp.has_flags(INP_IPV6) {
        !inp.has_outputopts6()
    } else {
        inp.options().is_none()
    }
}

/// `tcp_output`: TCP output routine: figure out what should be sent and send it.
pub fn tcp_output<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) -> Result<(), Errno> {
    // TCP_ECN
    let do_ecn = TCP_DO_ECN.load(Ordering::Relaxed) != 0;

    // TCP_SIGNATURE && DIAGNOSTIC: a signed connection that permits SACK is refused, in every
    // build (GENERIC's DIAGNOSTIC, the deviations).
    if tp.st.sack_enable != 0 && tp.st.has_flags(TF_SIGNATURE) {
        return Err(Errno::EINVAL);
    }

    let now = tcp_now();

    // The C reads SS_ISSENDING under sb_mtx; the kernel's buffer state is an atomic its
    // guard writes, read here without the mutex, which the pass takes once, for the
    // segment's copy out of the send buffer (the C copies under the socket's lock alone; the
    // kernel's records are the mutex's).
    let doing_sosend = soissending(tp.so);

    // Determine length of data that should be transmitted, and flags that will be used. If
    // there is some data or critical controls (SYN, RST) to send, then transmit; otherwise,
    // investigate further.
    let st = &mut *tp.st;
    let mut idle = st.has_flags(TF_LASTIDLE) || st.snd_max == st.snd_una;
    // The `uint64_t` idle time against the `int` timeout, converted.
    if idle && now.wrapping_sub(st.t_rcvtime) >= st.t_rxtcur as u64 {
        // We have been idle for "a while" and no acks are expected to clock out any data we
        // send -- slow start to get ack "clock" running again.
        st.snd_cwnd = 2 * u64::from(st.t_maxseg);
    }

    // remember 'idle' for next invocation of tcp_output
    if idle && doing_sosend {
        st.set_flags(TF_LASTIDLE);
        idle = false;
    } else {
        st.clear_flags(TF_LASTIDLE);
    }

    let once = Once {
        idle,
        doing_sosend,
        now,
        do_ecn,
    };
    let mut sendalot = false;
    loop {
        // again: the hole list out of the state for the pass (the module's Redesign), back
        // before the answer is handled.
        let mut holes = core::mem::replace(&mut tp.st.snd_holes, SlistLink::new());
        let pass = tcp_output_pass(tp, &mut holes, sendalot, once);
        // Nothing the pass calls makes a hole: the state's list is still the empty one.
        kassert!(!tp.st.snd_holes.is_some());
        tp.st.snd_holes = holes;
        let (error, win, packetlen) = match pass {
            Pass::Done(r) => return r,
            Pass::Sent {
                error,
                win,
                packetlen,
                sendalot: again,
            } => {
                sendalot = again;
                (error, win, packetlen)
            }
        };

        if let Err(error) = error {
            // out:
            match error {
                Errno::ENOBUFS => {
                    // If the interface queue is full, or IP cannot get an mbuf, trigger TCP
                    // slow start.
                    tp.st.snd_cwnd = u64::from(tp.st.t_maxseg);
                    return Ok(());
                }
                Errno::EMSGSIZE => {
                    // ip_output() will have already fixed the route for us. tcp_mtudisc()
                    // will, as its last action, initiate retransmission, so it is important
                    // to not do so here. (The C's errno -1: no error to report.)
                    tcp_mtudisc(tp, None);
                    return Ok(());
                }
                Errno::EHOSTUNREACH | Errno::ENETDOWN if tcps_havercvdsyn(tp.st.t_state) => {
                    tp.st.t_softerror = Some(error);
                    return Ok(());
                }
                _ => {}
            }

            // Restart the delayed ACK timer, if necessary.
            if tcp_timer_isarmed(tp.st, TCPT_DELACK) {
                tcp_timer_arm(tp, TCPT_DELACK, tcp_delack_msecs as u64);
            }

            return Err(error);
        }

        let st = &mut *tp.st;
        if packetlen > st.t_pmtud_mtu_sent {
            st.t_pmtud_mtu_sent = packetlen;
        }

        tcpstat_inc(TcpstatCounters::TcpsSndtotal);
        if tcp_timer_isarmed(st, TCPT_DELACK) {
            tcpstat_inc(TcpstatCounters::TcpsDelack);
        }

        // Data sent (as far as we can tell). If this advertises a larger window than any
        // other segment, then remember the size of the advertised window. Any pending ACK
        // has now been sent.
        if win > 0 && seq_gt(st.rcv_nxt.wrapping_add(win as u32), st.rcv_adv) {
            st.rcv_adv = st.rcv_nxt.wrapping_add(win as u32);
        }
        st.last_ack_sent = st.rcv_nxt;
        st.t_sndacktime = now;
        st.clear_flags(TF_ACKNOW);
        tcp_timer_disarm(tp, TCPT_DELACK);
        if !sendalot {
            return Ok(());
        }
    }
}

/// What `tcp_output` decides once, before its first pass.
#[derive(Clone, Copy)]
struct Once {
    /// `idle`.
    idle: bool,
    /// `doing_sosend`.
    doing_sosend: bool,
    /// `now`.
    now: u64,
    /// `do_ecn` (`TCP_ECN`).
    do_ecn: bool,
}

/// One pass of [`tcp_output`], from `again:` to the IP layer's answer: `holes` the control
/// block's SACK holes (out of its state for the pass), `sendalot` the previous pass's.
fn tcp_output_pass<G: ProcGlue>(
    tp: &mut Tp<'_, '_, G>,
    holes: &mut SlistLink<Sackhole>,
    sendalot: bool,
    once: Once,
) -> Pass {
    let Once {
        idle,
        doing_sosend,
        now,
        do_ecn,
    } = once;
    let so = tp.so;
    let inp = tp.inp;
    let mut opt = [0u8; MAX_TCPOPTLEN];
    let mut len: i64 = 0;

    // If we've recently taken a timeout, snd_max will be greater than snd_nxt. There may be
    // SACK information that allows us to avoid resending already delivered data. Adjust
    // snd_nxt accordingly.
    if tp.st.sack_enable != 0 && seq_lt(tp.st.snd_nxt, tp.st.snd_max) {
        tcp_sack_adjust(tp.st, holes);
    }
    let mut off = tp.st.snd_nxt.wrapping_sub(tp.st.snd_una) as i32;
    let mut win = min(tp.st.snd_wnd, tp.st.snd_cwnd) as i64;

    let mut flags = TCP_OUTFLAGS[tp.st.t_state as usize];

    // Send any SACK-generated retransmissions. If we're explicitly trying to send out new
    // data (when sendalot is 1), bypass this function. If we retransmit in fast recovery
    // mode, decrement snd_cwnd, since we're replacing a (future) new transmission with a
    // retransmission now, and we previously incremented snd_cwnd in tcp_input().
    let mut p: Option<&mut Sackhole> = None;
    if tp.st.sack_enable != 0
        && !sendalot
        && i32::from(tp.st.t_dupacks) >= TCPREXMTTHRESH
        && let Some(h) = tcp_sack_output(tp.st, holes)
    {
        let st = &mut *tp.st;
        off = h.rxmit.wrapping_sub(st.snd_una) as i32;
        // Coalesce holes into a single retransmission (libkern's u_int min).
        len = i64::from(min(u32::from(st.t_maxseg), h.end.wrapping_sub(h.rxmit)));
        if seq_lt(st.snd_una, st.snd_last) {
            st.snd_cwnd = st.snd_cwnd.wrapping_sub(u64::from(st.t_maxseg));
        }
        p = Some(h);
    }

    let mut sendalot = false;
    let mut tso = false;
    // If in persist timeout with window of 0, send 1 byte. Otherwise, if window is small but
    // nonzero and timer expired, we will send what we can and go to transmit state.
    if tp.st.t_force {
        if win == 0 {
            // If we still have some data to send, then clear the FIN bit. Usually this would
            // happen below when it realizes that we aren't sending all the data. However, if
            // we have exactly 1 byte of unset data, then it won't clear the FIN bit below,
            // and if we are in persist state, we wind up sending the packet without recording
            // that we sent the FIN bit.
            //
            // We can't just blindly clear the FIN bit, because if we don't have any more data
            // to send then the probe will be the FIN itself.
            if (off as u64) < so.so_snd.cc() {
                flags &= !TH_FIN;
            }
            win = 1;
        } else {
            tcp_timer_disarm(tp, TCPT_PERSIST);
            tp.st.t_rxtshift = 0;
        }
    }

    if p.is_none() {
        // ulmin(sb_cc, win) - off, in the C's `u_long`, assigned to the `long` len.
        len = min(so.so_snd.cc(), win as u64).wrapping_sub(off as u64) as i64;
    }

    if len < 0 {
        // If FIN has been sent but not acked, but we haven't been called to retransmit, len
        // will be -1. Otherwise, window shrank after we sent into it. If window shrank to 0,
        // cancel pending retransmit, pull snd_nxt back to (closed) window, and set the
        // persist timer if it isn't already going. If the window didn't close completely,
        // just wait for an ACK.
        len = 0;
        if win == 0 {
            tcp_timer_disarm(tp, TCPT_REXMT);
            tp.st.t_rxtshift = 0;
            tp.st.snd_nxt = tp.st.snd_una;
            if !tcp_timer_isarmed(tp.st, TCPT_PERSIST) {
                tcp_setpersist(tp);
            }
        }
    }

    // Never send more than half a buffer full. This insures that we can always keep 2
    // packets on the wire, no matter what SO_SNDBUF is, and therefore acks will never be
    // delayed unless we run out of data to transmit.
    let txmaxseg = min(so.so_snd.hiwat() / 2, u64::from(tp.st.t_maxseg)) as i64;

    if len > txmaxseg {
        let maxseg = i64::from(tp.st.t_maxseg);
        // The C's terms; the options' test, a loan of a pcb cell, last (the deviations).
        if TCP_DO_TSO.load(Ordering::Relaxed) != 0
            && !tp.st.has_flags(TF_SIGNATURE)
            && len >= 2 * maxseg
            && tp.st.rcv_numsacks == 0
            && p.is_none()
            && flags & (TH_SYN | TH_RST | TH_FIN) == 0
            && no_ip_options(inp)
        {
            tso = true;
            // avoid small chopped packets
            if len > (len / maxseg) * maxseg {
                len = (len / maxseg) * maxseg;
                sendalot = true;
            }
        } else {
            len = txmaxseg;
            sendalot = true;
        }
    }
    if ((i64::from(off) + len) as u64) < so.so_snd.cc() {
        flags &= !TH_FIN;
    }

    let (mut win, rcv_hiwat) = {
        let g = so.so_rcv.lock();
        (g.space(), so.so_rcv.hiwat() as i64)
    };

    'send: {
        // Sender silly window avoidance. If connection is idle and can send all data, a
        // maximum segment, at least a maximum default-size segment do it, or are forced, do
        // it; otherwise don't bother. If peer's buffer is tiny, then send when window is at
        // least half open. If retransmitting (possibly after persist timer forced us to send
        // into a small window), then must resend.
        let st = &*tp.st;
        if len != 0 {
            if len >= txmaxseg {
                break 'send;
            }
            if (idle || st.has_flags(TF_NODELAY))
                && ((len + i64::from(off)) as u64) >= so.so_snd.cc()
                && !doing_sosend
                && !st.has_flags(TF_NOPUSH)
            {
                break 'send;
            }
            if st.t_force {
                break 'send;
            }
            if (len as u64) >= st.max_sndwnd / 2 && st.max_sndwnd > 0 {
                break 'send;
            }
            if seq_lt(st.snd_nxt, st.snd_max) {
                break 'send;
            }
            if p.is_some() {
                break 'send;
            }
        }

        // Compare available window to amount of window known to peer (as advertised window
        // less next expected input). If the difference is at least two max size segments, or
        // at least 50% of the maximum possible window, then want to send a window update to
        // peer.
        if win > 0 {
            // "adv" is the amount we can increase the window, taking into account that we are
            // limited by TCP_MAXWIN << tp->rcv_scale.
            let adv = min(win, i64::from(TCP_MAXWIN) << st.rcv_scale)
                - i64::from(st.rcv_adv.wrapping_sub(st.rcv_nxt));

            if adv >= 2 * i64::from(st.t_maxseg) {
                break 'send;
            }
            if 2 * adv >= rcv_hiwat {
                break 'send;
            }
        }

        // Send if we owe peer an ACK.
        if st.has_flags(TF_ACKNOW) {
            break 'send;
        }
        if flags & (TH_SYN | TH_RST) != 0 {
            break 'send;
        }
        if seq_gt(st.snd_up, st.snd_una) {
            break 'send;
        }
        // If our state indicates that FIN should be sent and we have not yet done so, or
        // we're retransmitting the FIN, then we need to send.
        if flags & TH_FIN != 0 && (!st.has_flags(TF_SENTFIN) || st.snd_nxt == st.snd_una) {
            break 'send;
        }
        // In SACK, it is possible for tcp_output to fail to send a segment after the
        // retransmission timer has been turned off. Make sure that the retransmission timer
        // is set.
        if seq_gt(st.snd_max, st.snd_una)
            && !tcp_timer_isarmed(st, TCPT_REXMT)
            && !tcp_timer_isarmed(st, TCPT_PERSIST)
        {
            let rxtcur = st.t_rxtcur;
            tcp_timer_arm(tp, TCPT_REXMT, rxtcur as u64);
            return Pass::Done(Ok(()));
        }

        // TCP window updates are not reliable, rather a polling protocol using ``persist''
        // packets is used to insure receipt of window updates. The three ``states'' for the
        // output side are:
        //	idle			not doing retransmits or persists
        //	persisting		to move a small or zero window
        //	(re)transmitting	and thereby not persisting
        //
        // tp->t_timer[TCPT_PERSIST]
        //	is set when we are in persist state.
        // tp->t_force
        //	is set when we are called to send a persist packet.
        // tp->t_timer[TCPT_REXMT]
        //	is set when we are retransmitting
        // The output side is idle when both timers are zero.
        //
        // If send window is too small, there is data to transmit, and no retransmit or
        // persist is pending, then go to persist state. If nothing happens soon, send when
        // timer expires: if window is nonzero, transmit what we can, otherwise force out a
        // byte.
        if so.so_snd.cc() != 0
            && !tcp_timer_isarmed(st, TCPT_REXMT)
            && !tcp_timer_isarmed(st, TCPT_PERSIST)
        {
            tp.st.t_rxtshift = 0;
            tcp_setpersist(tp);
        }

        // No reason to send a segment, just return.
        return Pass::Done(Ok(()));
    }

    // send:
    // Before ESTABLISHED, force sending of initial options unless TCP set not to do any
    // options. NOTE: we assume that the IP/TCP header plus TCP options always fit in a
    // single mbuf, leaving room for a maximum link header, i.e.
    //	max_linkhdr + sizeof(network header) + sizeof(struct tcphdr +
    //		optlen <= MHLEN
    let mut optlen: usize = 0;

    let af = tp.st.pf;
    let mut hdrlen = match af {
        // default to PF_INET
        0 | AF_INET_I32 => (size_of::<Ip>() + size_of::<Tcphdr>()) as u32,
        AF_INET6_I32 => (size_of::<Ip6Hdr>() + size_of::<Tcphdr>()) as u32,
        _ => return Pass::Done(Err(Errno::EPFNOSUPPORT)),
    };

    if flags & TH_SYN != 0 {
        tp.st.snd_nxt = tp.st.iss;
        if !tp.st.has_flags(TF_NOOPT) {
            opt[0] = TCPOPT_MAXSEG;
            opt[1] = 4;
            let mss = tcp_mss(tp, 0) as u16;
            opt[2..4].copy_from_slice(&mss.to_be_bytes());
            optlen = 4;

            if flags & TH_ACK != 0 {
                tcp_mss_update(tp);
            }
            let st = &*tp.st;
            // If this is the first SYN of connection (not a SYN ACK), include SACK_PERMIT_HDR
            // option. If this is a SYN ACK, include SACK_PERMIT_HDR option if peer has
            // already done so.
            if st.sack_enable != 0 && (flags & TH_ACK == 0 || st.has_flags(TF_SACK_PERMIT)) {
                opt_put32(&mut opt, optlen, TCPOPT_SACK_PERMIT_HDR);
                optlen += 4;
            }
            if st.has_flags(TF_REQ_SCALE) && (flags & TH_ACK == 0 || st.has_flags(TF_RCVD_SCALE)) {
                opt_put32(
                    &mut opt,
                    optlen,
                    u32::from(TCPOPT_NOP) << 24
                        | u32::from(TCPOPT_WINDOW) << 16
                        | u32::from(TCPOLEN_WINDOW) << 8
                        | u32::from(st.request_r_scale),
                );
                optlen += 4;
            }
        }
    }

    let st = &mut *tp.st;
    // Send a timestamp and echo-reply if this is a SYN and our side wants to use timestamps
    // (TF_REQ_TSTMP is set) or both our side and our peer have sent timestamps in our SYN's.
    if st.t_flags & (TF_REQ_TSTMP | TF_NOOPT) == TF_REQ_TSTMP
        && flags & TH_RST == 0
        && (flags & (TH_SYN | TH_ACK) == TH_SYN || st.has_flags(TF_RCVD_TSTMP))
    {
        // Form timestamp option as shown in appendix A of RFC 1323.
        opt_put32(&mut opt, optlen, TCPOPT_TSTAMP_HDR);
        opt_put32(
            &mut opt,
            optlen + 4,
            (now as u32).wrapping_add(st.ts_modulate),
        );
        opt_put32(&mut opt, optlen + 8, st.ts_recent);
        optlen += usize::from(TCPOLEN_TSTAMP_APPA);
    }
    // Set receive buffer autosizing timestamp.
    if st.rfbuf_ts == 0 {
        st.rfbuf_ts = now;
        st.rfbuf_cnt = 0;
    }

    // TCP_SIGNATURE
    let mut sigoff: usize = 0;
    if st.has_flags(TF_SIGNATURE) {
        // Send signature option: its kind and length, sixteen zeroes for the digest, then
        // pad options list to the next 32 bit boundary and terminate it.
        let mut sigopt = [0u8; TCPOLEN_SIGLEN as usize];
        sigopt[0] = TCPOPT_SIGNATURE;
        sigopt[1] = TCPOLEN_SIGNATURE;
        sigopt[18] = TCPOPT_NOP;
        sigopt[19] = TCPOPT_NOP;
        sigoff = optlen + 2;
        opt_put(&mut opt, optlen, &sigopt);
        optlen += usize::from(TCPOLEN_SIGLEN);
    }

    // Send SACKs if necessary. This should be the last option processed. Only as many SACKs
    // are sent as are permitted by the maximum options size. No more than three SACKs are
    // sent.
    if st.sack_enable != 0
        && st.t_state == TCPS_ESTABLISHED
        && st.t_flags & (TF_SACK_PERMIT | TF_NOOPT) == TF_SACK_PERMIT
        && st.rcv_numsacks != 0
    {
        let olp = optlen;
        let mut lp = optlen + 4;
        let mut count: u32 = 0; // actual number of SACKs inserted
        // The C's unsigned difference, divided, then libkern's u_int min.
        let maxsack =
            (MAX_TCPOPTLEN as u32).wrapping_sub(optlen as u32 + 4) / u32::from(TCPOLEN_SACK);

        tcpstat_inc(TcpstatCounters::TcpsSackSndOpts);
        let maxsack = min(maxsack, TCP_MAX_SACK as u32);
        for sack in st.sackblks.iter().take(st.rcv_numsacks.max(0) as usize) {
            if count >= maxsack {
                break;
            }
            if sack.start == 0 && sack.end == 0 {
                continue;
            }
            opt_put32(&mut opt, lp, sack.start);
            opt_put32(&mut opt, lp + 4, sack.end);
            lp += 8;
            count += 1;
        }
        opt_put32(
            &mut opt,
            olp,
            TCPOPT_SACK_HDR | (u32::from(TCPOLEN_SACK) * count + 2),
        );
        optlen += usize::from(TCPOLEN_SACK) * count as usize + 4; // including leading NOPs
    }

    // DIAGNOSTIC: "tcp_output: options too long" (each write above checked the buffer).
    kassert!(optlen <= MAX_TCPOPTLEN);

    hdrlen += optlen as u32;

    let max_linkhdr = MAX_LINKHDR.load(Ordering::Relaxed) as u32;

    // Adjust data length if insertion of options will bump the packet length beyond the
    // t_maxopd length. Clear the FIN bit because we cut off the tail of the segment. (The
    // C's `u_short - u_int`, an unsigned difference, widened to the `long` len.)
    let maxopd = u32::from(st.t_maxopd).wrapping_sub(optlen as u32);
    if len > i64::from(maxopd) {
        if tso {
            if len + i64::from(hdrlen) + i64::from(max_linkhdr) > MAXMCLBYTES as i64 {
                len = i64::from(
                    (MAXMCLBYTES as u32)
                        .wrapping_sub(hdrlen)
                        .wrapping_sub(max_linkhdr),
                );
                sendalot = true;
            }
        } else {
            len = i64::from(maxopd);
            sendalot = true;
        }
        flags &= !TH_FIN;
    }

    if cfg!(feature = "diagnostic") && (max_linkhdr + hdrlen) as usize > MCLBYTES {
        kpanic(format_args!("tcphdr too big"));
    }

    // Grab a header mbuf, attaching a copy of data to be transmitted, and initialize the
    // header from the template for sends on this connection.
    let mut m = if len != 0 {
        if st.t_force && len == 1 {
            tcpstat_inc(TcpstatCounters::TcpsSndprobe);
        } else if seq_lt(st.snd_nxt, st.snd_max) {
            tcpstat_pkt(
                TcpstatCounters::TcpsSndrexmitpack,
                TcpstatCounters::TcpsSndrexmitbyte,
                len as u64,
            );
            st.t_sndrexmitpack = st.t_sndrexmitpack.wrapping_add(1);
        } else {
            tcpstat_pkt(
                TcpstatCounters::TcpsSndpack,
                TcpstatCounters::TcpsSndbyte,
                len as u64,
            );
        }
        let Some(mut m) = tcp_output_gethdr(max_linkhdr, hdrlen) else {
            return out(Errno::ENOBUFS);
        };
        // so->so_snd.sb_mb, under the buffer's mutex for the copy.
        let mut g = so.so_snd.lock();
        let loopcnt = match g.data().first() {
            Some(sb_mb) if sb_mb.m_flags().get() & M_PKTHDR != 0 => {
                Some(sb_mb.m_pkthdr().ph_loopcnt.get())
            }
            Some(_) => None,
            // The C dereferences sb_mb: data to send is in the buffer.
            None => kpanic(format_args!("tcp_output: no data in the send buffer")),
        };
        if len <= i64::from(m_trailingspace(&m)) {
            m.m_len().set(hdrlen + len as u32);
            let Some(d) = m.data_mut() else {
                kpanic(format_args!("tcp_output: a new header mbuf is read-only"));
            };
            g.copydata_first(off, &mut d[hdrlen as usize..]);
            drop(g);
        } else {
            let next = g.copym_first(off, len as i32, M_DONTWAIT);
            drop(g);
            let Some(next) = next else {
                drop(m);
                return out(Errno::ENOBUFS);
            };
            let _ = m.m_next.set(next);
        }
        if let Some(loopcnt) = loopcnt {
            m.m_pkthdr().ph_loopcnt.set(loopcnt);
        }
        // If we're sending everything we've got, set PUSH. (This will keep happy those
        // implementations which only give data to the user when a buffer fills or a PUSH
        // comes in.)
        if (i64::from(off) + len) as u64 == so.so_snd.cc() && !doing_sosend {
            flags |= TH_PUSH;
        }
        st.t_sndtime = now;
        m
    } else {
        if st.has_flags(TF_ACKNOW) {
            tcpstat_inc(TcpstatCounters::TcpsSndacks);
        } else if flags & (TH_SYN | TH_FIN | TH_RST) != 0 {
            tcpstat_inc(TcpstatCounters::TcpsSndctrl);
        } else if seq_gt(st.snd_up, st.snd_una) {
            tcpstat_inc(TcpstatCounters::TcpsSndurg);
        } else {
            tcpstat_inc(TcpstatCounters::TcpsSndwinup);
        }

        let Some(m) = tcp_output_gethdr(max_linkhdr, hdrlen) else {
            return out(Errno::ENOBUFS);
        };
        m
    };
    let ph = m.m_pkthdr();
    ph.ph_ifidx.set(0);
    ph.len.set((i64::from(hdrlen) + len) as i32);

    // Enable TSO and specify the size of the resulting segments.
    if tso {
        ph.csum_flags.set(ph.csum_flags.get() | M_TCP_TSO);
        ph.ph_mss.set(st.t_maxseg);
    }

    let Some(template) = st.t_template.as_ref() else {
        kpanic(format_args!("tcp_output"));
    };
    let tbytes = template.data();
    let tlen = tbytes.len();
    if cfg!(feature = "diagnostic") && tlen != hdrlen as usize - optlen {
        kpanic(format_args!("tcp_output: template len != hdrlen - optlen"));
    }
    let Some(d) = m.data_mut() else {
        kpanic(format_args!("tcp_output: a new header mbuf is read-only"));
    };
    d[..tlen].copy_from_slice(tbytes);
    let thoff = tlen - size_of::<Tcphdr>();
    let Some(mut th) = pod::at::<Tcphdr>(d, thoff) else {
        kpanic(format_args!("tcp_output: a template of {tlen} bytes"));
    };

    // Fill in fields, remembering maximum advertised window for use in delaying messages
    // about window sizes. If resending a FIN, be sure not to use a new sequence number.
    if flags & TH_FIN != 0 && st.has_flags(TF_SENTFIN) && st.snd_nxt == st.snd_max {
        st.snd_nxt = st.snd_nxt.wrapping_sub(1);
    }
    // If we are doing retransmissions, then snd_nxt will not reflect the first unsent octet.
    // For ACK only packets, we do not want the sequence number of the retransmitted packet,
    // we want the sequence number of the next unsent octet. So, if there is no data (and no
    // SYN or FIN), use snd_max instead of snd_nxt when filling in ti_seq. But if we are in
    // persist state, snd_max might reflect one byte beyond the right edge of the window, so
    // use snd_nxt in that case, since we know we aren't doing a retransmission. (retransmit
    // and persist are mutually exclusive...)
    if len != 0 || flags & (TH_SYN | TH_FIN) != 0 || tcp_timer_isarmed(st, TCPT_PERSIST) {
        th.th_seq = htonl(st.snd_nxt);
    } else {
        th.th_seq = htonl(st.snd_max);
    }

    if let Some(h) = p.as_deref_mut() {
        // If sendalot was turned on (due to option stuffing), turn it off. Properly set
        // th_seq field. Advance the ret'x pointer by len.
        sendalot = false;
        th.th_seq = htonl(h.rxmit);
        h.rxmit = h.rxmit.wrapping_add(len as u32);
        tcpstat_pkt(
            TcpstatCounters::TcpsSackRexmits,
            TcpstatCounters::TcpsSackRexmitBytes,
            len as u64,
        );
    }

    th.th_ack = htonl(st.rcv_nxt);
    let thend = thoff + size_of::<Tcphdr>();
    if optlen != 0 {
        d[thend..thend + optlen].copy_from_slice(&opt[..optlen]);
        th.set_th_off(((size_of::<Tcphdr>() + optlen) >> 2) as u8);
    }
    // TCP_ECN
    if do_ecn {
        // if we have received congestion experienced segs, set ECE bit.
        if st.has_flags(TF_RCVD_CE) {
            flags |= TH_ECE;
            tcpstat_inc(TcpstatCounters::TcpsEcnSndece);
        }
        if !st.has_flags(TF_DISABLE_ECN) {
            // if this is a SYN seg, set ECE and CWR. set only ECE for SYN-ACK if peer
            // supports ECN.
            if flags & (TH_SYN | TH_ACK) == TH_SYN {
                flags |= TH_ECE | TH_CWR;
            } else if st.has_flags(TF_ECN_PERMIT) && flags & (TH_SYN | TH_ACK) == (TH_SYN | TH_ACK)
            {
                flags |= TH_ECE;
            }
        }
        // if we have reduced the congestion window, notify the peer by setting CWR bit.
        if st.has_flags(TF_ECN_PERMIT) && st.has_flags(TF_SEND_CWR) {
            flags |= TH_CWR;
            st.clear_flags(TF_SEND_CWR);
            tcpstat_inc(TcpstatCounters::TcpsEcnSndcwr);
        }
    }
    th.th_flags = flags;

    // Calculate receive window. Don't shrink window, but avoid silly window syndrome.
    if win < rcv_hiwat / 4 && win < i64::from(st.t_maxseg) {
        win = 0;
    }
    if win > i64::from(TCP_MAXWIN) << st.rcv_scale {
        win = i64::from(TCP_MAXWIN) << st.rcv_scale;
    }
    // (long)(int32_t)(rcv_adv - rcv_nxt): the window already advertised, signed.
    let advertised = i64::from(st.rcv_adv.wrapping_sub(st.rcv_nxt) as i32);
    if win < advertised {
        win = advertised;
    }
    if flags & TH_RST != 0 {
        win = 0;
    }
    th.th_win = htons((win >> st.rcv_scale) as u16);
    if th.th_win == 0 {
        st.t_sndzerowin = st.t_sndzerowin.wrapping_add(1);
    }
    if seq_gt(st.snd_up, st.snd_nxt) {
        let urp = min(st.snd_up.wrapping_sub(st.snd_nxt), IP_MAXPACKET as u32);
        th.th_urp = htons(urp as u16);
        th.th_flags |= TH_URG;
    } else {
        // If no urgent pointer to send, then we pull the urgent pointer to the left edge of
        // the send window so that it doesn't drift into the send window on sequence number
        // wraparound.
        st.snd_up = st.snd_una; // drag it along
    }
    put_hdr(d, thoff, &th);

    // TCP_SIGNATURE
    if st.has_flags(TF_SIGNATURE) {
        // The SA between the packet's addresses: two unions zeroed but for their length,
        // family and address (`af` is 0, AF_INET or AF_INET6: the switch above refused any
        // other).
        let mut src = [0u8; SOCKADDR_UNION_LEN];
        let mut dst = [0u8; SOCKADDR_UNION_LEN];
        let iphlen = if af == AF_INET6_I32 {
            let Some(ip6) = pod::at::<Ip6Hdr>(d, 0) else {
                kpanic(format_args!("tcp_output: no IPv6 header"));
            };
            for (u, a) in [
                (&mut src, ip6.ip6_src.s6_addr),
                (&mut dst, ip6.ip6_dst.s6_addr),
            ] {
                u[0] = 28; // sizeof(struct sockaddr_in6)
                u[1] = AF_INET6;
                u[8..24].copy_from_slice(&a);
            }
            size_of::<Ip6Hdr>() as i32
        } else {
            let Some(ip) = pod::at::<Ip>(d, 0) else {
                kpanic(format_args!("tcp_output: no IP header"));
            };
            for (u, a) in [(&mut src, ip.ip_src), (&mut dst, ip.ip_dst)] {
                u[0] = 16; // sizeof(struct sockaddr_in)
                u[1] = AF_INET;
                u[4..8].copy_from_slice(&a.s_addr.to_ne_bytes());
            }
            size_of::<Ip>() as i32
        };

        // The lookup's reference to the SA, given back by the hold's drop on every way out
        // of this block (the C's tdb_unref after the digest, or after m_freem on EINVAL).
        let Some(hold) = ipsec().and_then(|s| {
            s.tcp_signature_tdb(rtable_l2(inp.inp_rtableid.get()), &src, &dst)
                .map(|t| TdbHold::new(s, t))
        }) else {
            drop(m);
            return Pass::Done(Err(Errno::EPERM));
        };

        let mut sig = [0u8; MD5_DIGEST_LENGTH];
        if !tcp_signature(hold.tdb(), af, &m, &th, iphlen, false, &mut sig) {
            drop(m);
            drop(hold);
            return Pass::Done(Err(Errno::EINVAL));
        }
        let at = hdrlen as usize - optlen + sigoff;
        let Some(d) = m.data_mut() else {
            kpanic(format_args!("tcp_output: a new header mbuf is read-only"));
        };
        d[at..at + MD5_DIGEST_LENGTH].copy_from_slice(&sig);
        drop(hold);
    }

    // Defer checksumming until later (ip_output() or hardware)
    let ph = m.m_pkthdr();
    ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_OUT);

    // In transmit state, time the transmission and arrange for the retransmit. In persist
    // state, just set snd_max.
    if !tp.st.t_force || !tcp_timer_isarmed(tp.st, TCPT_PERSIST) {
        let st = &mut *tp.st;
        let startseq = st.snd_nxt;

        // Advance snd_nxt over sequence space of this segment.
        if flags & (TH_SYN | TH_FIN) != 0 {
            if flags & TH_SYN != 0 {
                st.snd_nxt = st.snd_nxt.wrapping_add(1);
            }
            if flags & TH_FIN != 0 {
                st.snd_nxt = st.snd_nxt.wrapping_add(1);
                st.set_flags(TF_SENTFIN);
            }
        }
        // goto timer: a SACK retransmission that does not end at snd_nxt.
        let skip = st.sack_enable != 0 && p.as_ref().is_some_and(|h| h.rxmit != st.snd_nxt);
        if !skip {
            st.snd_nxt = st.snd_nxt.wrapping_add(len as u32);
            if seq_gt(st.snd_nxt, st.snd_max) {
                st.snd_max = st.snd_nxt;
                // Time this transmission if not a retransmission and not currently timing
                // anything.
                if st.t_rtttime == 0 {
                    st.t_rtttime = now;
                    st.t_rtseq = startseq;
                    tcpstat_inc(TcpstatCounters::TcpsSegstimed);
                }
            }
        }

        // Set retransmit timer if not currently set, and not doing an ack or a keep-alive
        // probe. Initial value for retransmit timer is smoothed round-trip time + 2 *
        // round-trip time variance. Initialize shift counter which is used for backoff of
        // retransmit time.
        // timer:
        if tp.st.sack_enable != 0
            && p.is_some()
            && !tcp_timer_isarmed(tp.st, TCPT_REXMT)
            && tp.st.snd_nxt != tp.st.snd_max
        {
            arm_rexmt(tp);
        }

        if !tcp_timer_isarmed(tp.st, TCPT_REXMT) && tp.st.snd_nxt != tp.st.snd_una {
            arm_rexmt(tp);
        }

        if len == 0
            && so.so_snd.cc() != 0
            && !tcp_timer_isarmed(tp.st, TCPT_REXMT)
            && !tcp_timer_isarmed(tp.st, TCPT_PERSIST)
        {
            // Avoid a situation where we do not set persist timer after a zero window
            // condition. For example:
            // 1) A -> B: packet with enough data to fill the window
            // 2) B -> A: ACK for #1 + new data (0 window advertisement)
            // 3) A -> B: ACK for #2, 0 len packet
            //
            // In this case, A will not activate the persist timer, because it chose to send
            // a packet. Unless tcp_output is called for some other reason (delayed ack
            // timer, another input packet from B, socket syscall), A will not send zero
            // window probes.
            //
            // So, if you send a 0-length packet, but there is data in the socket buffer, and
            // neither the rexmt or persist timer is already set, then activate the persist
            // timer.
            tp.st.t_rxtshift = 0;
            tcp_setpersist(tp);
        }
    } else if seq_gt(tp.st.snd_nxt.wrapping_add(len as u32), tp.st.snd_max) {
        tp.st.snd_max = tp.st.snd_nxt.wrapping_add(len as u32);
    }

    tcp_update_sndspace(tp);

    let st = &mut *tp.st;
    // Trace.
    if so.options() & SO_DEBUG != 0 {
        tcp_trace(
            TA_OUTPUT,
            st.t_state,
            Some(st),
            core::ptr::from_ref::<Tcpcb<G>>(&tp.cb).addr(),
            Some(&m.data()[..hdrlen as usize]),
            0,
            len as i32,
        );
    }

    // Fill in IP length and desired time to live and send to IP level. There should be a
    // better way to handle ttl and tos; we could keep them in the template, but need a way
    // to checksum without them.

    // TCP_ECN: if peer is ECN capable, set the ECT bit in the IP header. but don't set ECT
    // for a pure ack, a retransmit or a window probe.
    let mut needect = false;
    if do_ecn && st.has_flags(TF_ECN_PERMIT) {
        if len == 0 || seq_lt(st.snd_nxt, st.snd_max) || (st.t_force && len == 1) {
            // don't set ECT
        } else {
            needect = true;
            tcpstat_inc(TcpstatCounters::TcpsEcnSndect);
        }
    }

    let ph = m.m_pkthdr();
    // force routing table
    ph.ph_rtableid.set(inp.inp_rtableid.get());

    // NPF: the packet names its pcb (pf's own reference, pf's leg).
    if let Some(pf) = pf() {
        let a: &(dyn core::any::Any + Sync) = inp;
        pf.mbuf_link_inpcb_of(&m, a);
    }
    // NSTOEPLITZ
    let ph = m.m_pkthdr();
    ph.ph_flowid.set(inp.inp_flowid.get());
    ph.csum_flags.set(ph.csum_flags.get() | M_FLOWID);

    let packetlen = ph.len.get() as u32;
    let seclevel = inp.inp_seclevel.get();
    let Some(d) = m.data_mut() else {
        kpanic(format_args!("tcp_output: a new header mbuf is read-only"));
    };
    let error = if af == AF_INET6_I32 {
        let Some(mut ip6) = pod::at::<Ip6Hdr>(d, 0) else {
            kpanic(format_args!("tcp_output: no IPv6 header"));
        };
        // Host order, as the C stores it (ip6_output writes it again).
        ip6.ip6_plen = (packetlen as usize - size_of::<Ip6Hdr>()) as u16;
        ip6.ip6_nxt = IPPROTO_TCP as u8;
        ip6.ip6_hlim = in6_selecthlim(inp) as u8;
        if needect {
            ip6.ip6_flow |= htonl(u32::from(IPTOS_ECN_ECT0) << 20);
        }
        put_hdr(d, 0, &ip6);
        // The sticky options and the route cache, lent for the call (the socket lock is
        // held).
        let sticky = inp.outputopts6();
        let mut ro = inp.route();
        ip6_output(
            m,
            sticky.as_deref().map(Ip6Pktopts::as_opts),
            Some(&mut *ro),
            0,
            None,
            Some(&seclevel),
            tp.net,
        )
    } else {
        // case 0: default to PF_INET; AF_INET
        let Some(mut ip) = pod::at::<Ip>(d, 0) else {
            kpanic(format_args!("tcp_output: no IP header"));
        };
        ip.ip_len = htons(packetlen as u16);
        let proto = inp.inp_ip.get();
        ip.ip_ttl = proto.ip_ttl;
        ip.ip_tos = proto.ip_tos;
        if needect {
            ip.ip_tos |= IPTOS_ECN_ECT0;
        }
        put_hdr(d, 0, &ip);
        // The options and the route cache, lent for the call.
        let opts = inp.options();
        let mut ro = inp.route();
        ip_output(
            m,
            opts.as_deref().map(Mbuf::data),
            Some(&mut *ro),
            if ip_mtudisc.load(Ordering::Relaxed) != 0 {
                IP_MTUDISC
            } else {
                0
            },
            None,
            Some(&seclevel),
            0,
            tp.net,
        )
    };

    Pass::Sent {
        error,
        win,
        packetlen,
        sendalot,
    }
}

/// The pass's `goto out` before a segment reached the IP layer.
#[inline]
fn out(error: Errno) -> Pass {
    Pass::Sent {
        error: Err(error),
        win: 0,
        packetlen: 0,
        sendalot: false,
    }
}

/// `TCP_TIMER_ARM(tp, TCPT_REXMT, tp->t_rxtcur)` and the persist timer stopped, the two
/// arms of `tcp_output`'s `timer:` block.
fn arm_rexmt<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) {
    let rxtcur = tp.st.t_rxtcur;
    tcp_timer_arm(tp, TCPT_REXMT, rxtcur as u64);
    if tcp_timer_isarmed(tp.st, TCPT_PERSIST) {
        tcp_timer_disarm(tp, TCPT_PERSIST);
        tp.st.t_rxtshift = 0;
    }
}

/// `tcp_setpersist`: start or restart the persistence timer, backed off by `t_rxtshift`
/// within `TCPTV_PERSMIN`..`TCPTV_PERSMAX`; `panic(9)` while the retransmit timer runs.
pub fn tcp_setpersist<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) {
    let st = &*tp.st;
    let mut t = (st.t_srtt >> 2).wrapping_add(st.t_rttvar) >> (1 + TCP_RTT_BASE_SHIFT);

    if tcp_timer_isarmed(st, TCPT_REXMT) {
        kpanic(format_args!("tcp_output REXMT"));
    }
    // Start/restart persistence timer: the `int` against the `u_int` minimum, unsigned.
    if (t as u32) < st.t_rttmin {
        t = st.t_rttmin as i32;
    }
    let msec = tcpt_rangeset(
        t.wrapping_mul(TCP_BACKOFF[st.t_rxtshift as usize]),
        TCPTV_PERSMIN,
        TCPTV_PERSMAX,
    );
    tcp_timer_arm(tp, TCPT_PERSIST, msec as u64);
    if i32::from(tp.st.t_rxtshift) < TCP_MAXRXTSHIFT {
        tp.st.t_rxtshift += 1;
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the kernel's `tcp_output`: the segments as the wire sees them (headers,
    // options, payload, the state after them) over a test interface, the SACK hole's
    // retransmission and its list back in the control block, TSO's cut, the persist state,
    // and the signature's refusal without an SA.

    use super::*;
    use crate::netinet::tcp_fsm::TCPS_SYN_SENT;
    use crate::netinet::tcp_subr::testtcp::{self, estab, last, pattern, queue, sent, stat};
    use crate::netinet::tcp_var::{TF_NODELAY, TcpstatCounters as C, Tp};

    type G = testtcp::G;

    /// Runs `tcp_output` on `inp`'s connection after `prep`, the answer and a reader's view
    /// of the state after it (`look`).
    fn output<R>(
        inp: &Inpcb<G>,
        prep: impl FnOnce(&mut Tp<'_, '_, G>),
        look: impl FnOnce(&Tp<'_, '_, G>) -> R,
    ) -> (Result<(), Errno>, R) {
        testtcp::with_tp(inp, |mut tp| {
            prep(&mut tp);
            let r = tcp_output(&mut tp);
            ((r, look(&tp)), None)
        })
    }

    #[test]
    fn a_data_segment_goes_out_as_the_c_builds_it() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout0", 20);
        let data = pattern(300);
        queue(&so, &data);
        let (pack, byte, total, timed) = (
            stat(C::TcpsSndpack),
            stat(C::TcpsSndbyte),
            stat(C::TcpsSndtotal),
            stat(C::TcpsSegstimed),
        );
        let before = sent(d);
        let (r, after) = output(
            &inp,
            |_| {},
            |tp| {
                let st = &*tp.st;
                (
                    (st.snd_nxt, st.snd_max, st.t_rtseq, st.t_rtttime != 0),
                    (st.rcv_adv, st.last_ack_sent, st.has_flags(TF_ACKNOW)),
                    (
                        tcp_timer_isarmed(st, TCPT_REXMT),
                        tp.cb.t_timer[TCPT_REXMT].pending(),
                        st.t_pmtud_mtu_sent,
                    ),
                )
            },
        );
        assert_eq!(r, Ok(()));
        assert_eq!(sent(d), before + 1, "one segment");
        let (ip, th, opts, payload) = last(d);
        // The IP header: the length, the pcb's TTL and TOS, the template's addresses.
        assert_eq!(u16::from_be(ip.ip_len), 340);
        assert_eq!(ip.ip_ttl, inp.inp_ip.get().ip_ttl);
        assert_eq!(ip.ip_tos, 0);
        assert_eq!(ip.ip_p, IPPROTO_TCP as u8);
        assert_eq!(ip.ip_src.s_addr, u32::from_ne_bytes([10, 9, 20, 1]));
        assert_eq!(ip.ip_dst.s_addr, u32::from_ne_bytes([10, 9, 20, 2]));
        // The TCP header: the ports, our next byte, the peer's next, ACK and PUSH (all the
        // buffer goes), the window of the receive buffer, no options.
        assert_eq!(
            (u16::from_be(th.th_sport), u16::from_be(th.th_dport)),
            (7000, 80)
        );
        assert_eq!(
            (u32::from_be(th.th_seq), u32::from_be(th.th_ack)),
            (1000, 5000)
        );
        assert_eq!((th.th_off(), th.th_x2()), (5, 0));
        assert_eq!(th.th_flags, TH_ACK | TH_PUSH);
        assert_eq!(u16::from_be(th.th_win), 16384);
        assert_eq!(th.th_urp, 0);
        assert!(opts.is_empty());
        assert_eq!(payload, data, "the send buffer's bytes");
        // The state: the sequence space advanced, the segment timed, the retransmit timer
        // running, the window advertised, the ACK owed sent.
        let ((nxt, max, rtseq, timing), (adv, lastack, acknow), (rexmt, pending, mtu)) = after;
        assert_eq!((nxt, max, rtseq, timing), (1300, 1300, 1000, true));
        assert_eq!((adv, lastack, acknow), (5000 + 16384, 5000, false));
        assert_eq!((rexmt, pending, mtu), (true, true, 340));
        assert_eq!(stat(C::TcpsSndpack), pack + 1);
        assert_eq!(stat(C::TcpsSndbyte), byte + 300);
        assert_eq!(stat(C::TcpsSndtotal), total + 1);
        assert_eq!(stat(C::TcpsSegstimed), timed + 1);
        testtcp::close(so, inp);
    }

    #[test]
    fn a_syn_carries_its_options_in_the_cs_order() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout1", 21);
        let t0 = tcp_now();
        let (r, nxt) = output(
            &inp,
            |tp| {
                let st = &mut *tp.st;
                st.t_state = TCPS_SYN_SENT;
                st.snd_una = 1000;
                st.snd_nxt = 1000;
                st.snd_max = 1000;
                st.set_flags(TF_REQ_SCALE | TF_REQ_TSTMP);
                st.request_r_scale = 3;
                st.ts_modulate = 0x10;
                st.ts_recent = 0;
            },
            |tp| (tp.st.snd_nxt, tp.st.snd_max),
        );
        let t1 = tcp_now();
        assert_eq!(r, Ok(()));
        let (_ip, th, opts, payload) = last(d);
        assert_eq!(th.th_flags, TH_SYN, "SYN_SENT's flags");
        assert_eq!(u32::from_be(th.th_seq), 1000, "the iss");
        assert_eq!(th.th_off(), (20 + 24) / 4);
        assert!(payload.is_empty());
        // MSS (the interface's 1500 less the headers), SACK permitted, window scale 3, the
        // timestamps (our clock and its modulation, the peer's 0): the C's order.
        assert_eq!(
            &opts[..4],
            &[
                TCPOPT_MAXSEG,
                4,
                1460u16.to_be_bytes()[0],
                1460u16.to_be_bytes()[1]
            ]
        );
        assert_eq!(&opts[4..8], &TCPOPT_SACK_PERMIT_HDR.to_be_bytes());
        assert_eq!(
            &opts[8..12],
            &[TCPOPT_NOP, TCPOPT_WINDOW, TCPOLEN_WINDOW, 3]
        );
        assert_eq!(&opts[12..16], &TCPOPT_TSTAMP_HDR.to_be_bytes());
        let tsval = u32::from_be_bytes([opts[16], opts[17], opts[18], opts[19]]);
        assert!(
            (t0 as u32).wrapping_add(0x10) <= tsval && tsval <= (t1 as u32).wrapping_add(0x10),
            "TSval {tsval} from the clock"
        );
        assert_eq!(&opts[20..24], &[0, 0, 0, 0]);
        assert_eq!(nxt, (1001, 1001), "the SYN takes one sequence number");
        testtcp::close(so, inp);
    }

    #[test]
    fn a_pure_ack_carries_snd_max_and_clears_what_it_owed() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout2", 22);
        let acks = stat(C::TcpsSndacks);
        let (r, (acknow, delack)) = output(
            &inp,
            |tp| {
                // A retransmission started (snd_nxt pulled back), an ACK owed.
                tp.st.snd_max = 1500;
                tp.st.set_flags(TF_ACKNOW);
                tcp_timer_arm(tp, TCPT_DELACK, 200);
            },
            |tp| {
                (
                    tp.st.has_flags(TF_ACKNOW),
                    tcp_timer_isarmed(tp.st, TCPT_DELACK),
                )
            },
        );
        assert_eq!(r, Ok(()));
        let (_ip, th, opts, payload) = last(d);
        assert_eq!(th.th_flags, TH_ACK);
        assert_eq!(u32::from_be(th.th_seq), 1500, "no data: snd_max");
        assert!(opts.is_empty() && payload.is_empty());
        assert!(!acknow && !delack, "the ACK sent, the delayed ACK disarmed");
        assert_eq!(stat(C::TcpsSndacks), acks + 1);
        testtcp::close(so, inp);
    }

    #[test]
    fn nothing_to_send_is_nothing_sent_and_a_closed_window_persists() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout3", 23);
        let before = sent(d);
        let (r, persist) = output(&inp, |_| {}, |tp| tcp_timer_isarmed(tp.st, TCPT_PERSIST));
        assert_eq!((r, persist), (Ok(()), false), "no reason to send");
        assert_eq!(sent(d), before);

        // Data, but the peer's window is shut: the persist timer starts.
        queue(&so, &pattern(100));
        let (r, (persist, shift)) = output(
            &inp,
            |tp| tp.st.snd_wnd = 0,
            |tp| (tcp_timer_isarmed(tp.st, TCPT_PERSIST), tp.st.t_rxtshift),
        );
        assert_eq!(r, Ok(()));
        assert_eq!(sent(d), before, "nothing into a closed window");
        assert_eq!(
            (persist, shift),
            (true, 1),
            "tcp_setpersist backed off once"
        );
        testtcp::close(so, inp);
    }

    #[test]
    fn the_sack_option_reports_the_receivers_blocks() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout4", 24);
        let snd = stat(C::TcpsSackSndOpts);
        let (r, ()) = output(
            &inp,
            |tp| {
                let st = &mut *tp.st;
                st.sack_enable = 1;
                st.set_flags(TF_SACK_PERMIT | TF_ACKNOW);
                st.rcv_numsacks = 2;
                st.sackblks[0] = crate::netinet::tcp_var::Sackblk {
                    start: 6000,
                    end: 6100,
                };
                st.sackblks[1] = crate::netinet::tcp_var::Sackblk {
                    start: 6200,
                    end: 6300,
                };
            },
            |_| (),
        );
        assert_eq!(r, Ok(()));
        let (_ip, th, opts, _payload) = last(d);
        assert_eq!(th.th_off(), (20 + 20) / 4);
        let mut want = std::vec::Vec::new();
        want.extend_from_slice(&(TCPOPT_SACK_HDR | (2 * 8 + 2)).to_be_bytes());
        for v in [6000u32, 6100, 6200, 6300] {
            want.extend_from_slice(&v.to_be_bytes());
        }
        assert_eq!(opts, want, "two NOPs, SACK, 18, the two blocks");
        assert_eq!(stat(C::TcpsSackSndOpts), snd + 1);
        testtcp::close(so, inp);
    }

    #[test]
    fn a_sack_hole_is_retransmitted_from_its_rxmit_and_the_holes_stay() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout5", 25);
        // 2000 bytes in flight from snd_una 1000; the peer holds 2000..3000 (a hole at
        // 1500..2000 with three duplicate ACKs).
        let data = pattern(2000);
        queue(&so, &data);
        let rex = stat(C::TcpsSackRexmits);
        let (r, (rxmit, holes, nxt)) = output(
            &inp,
            |tp| {
                let st = &mut *tp.st;
                st.sack_enable = 1;
                st.snd_nxt = 3000;
                st.snd_max = 3000;
                st.t_dupacks = 3;
                let h = crate::netinet::tcp_subr::SACKHL_POOL
                    .get(hal::pool::PR_WAITOK, || Sackhole::new(1500, 2000, 3, 1500))
                    .expect("a hole");
                let _ = st.snd_holes.set(h);
            },
            |tp| {
                let h = tp.st.snd_holes.get().expect("the hole is back");
                (h.rxmit, h.next.get().is_none(), tp.st.snd_nxt)
            },
        );
        assert_eq!(r, Ok(()));
        let (_ip, th, _opts, payload) = last(d);
        assert_eq!(u32::from_be(th.th_seq), 1500, "from the hole's rxmit");
        assert_eq!(payload, &data[500..1000], "the hole's bytes");
        assert_eq!(rxmit, 2000, "rxmit advanced by the segment");
        assert!(holes, "one hole, back in the control block");
        assert_eq!(nxt, 3000, "a hole's retransmission leaves snd_nxt");
        assert_eq!(stat(C::TcpsSackRexmits), rex + 1);
        testtcp::with_tp(&inp, |tp| {
            crate::netinet::tcp_timer::tcp_timer_freesack(tp.st);
            ((), None)
        });
        testtcp::close(so, inp);
    }

    #[test]
    fn a_large_send_goes_as_tso_unless_the_pcb_has_ip_options() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout6", 26);
        let data = pattern(4000);
        queue(&so, &data);
        let pack = stat(C::TcpsSndpack);
        let before = sent(d);
        let (r, nxt) = output(
            &inp,
            |tp| {
                tp.st.set_flags(TF_NODELAY);
                tp.st.snd_cwnd = 65535;
            },
            |tp| tp.st.snd_nxt,
        );
        assert_eq!(r, Ok(()));
        // Two passes: a TSO packet of two segments (the interface cuts it in software),
        // then the 1080 bytes left; three packets on the wire, two counted sends.
        assert_eq!(stat(C::TcpsSndpack), pack + 2);
        assert_eq!(sent(d), before + 3);
        assert_eq!(nxt, 5000);
        let (_ip, th, _opts, payload) = last(d);
        assert_eq!(u32::from_be(th.th_seq), 1000 + 2920);
        assert_eq!(payload, &data[2920..]);
        testtcp::close(so, inp);

        // The same send from a pcb with IP options: no TSO, three passes.
        let (so, inp, d) = estab(b"tout7", 27);
        queue(&so, &data);
        testtcp::with_lock(&inp, |inp, _il| {
            *inp.options() = crate::net::mbuf::m_get(M_DONTWAIT, hal::sys::mbuf::MT_SOOPTS);
        });
        let pack = stat(C::TcpsSndpack);
        let before = sent(d);
        let (r, ()) = output(
            &inp,
            |tp| {
                tp.st.set_flags(TF_NODELAY);
                tp.st.snd_cwnd = 65535;
            },
            |_| (),
        );
        assert_eq!(r, Ok(()));
        assert_eq!(stat(C::TcpsSndpack), pack + 3, "segment by segment");
        assert_eq!(sent(d), before + 3);
        testtcp::close(so, inp);
    }

    #[test]
    fn a_signed_connection_without_its_sa_sends_nothing() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tout8", 28);
        queue(&so, &pattern(10));
        let before = sent(d);
        let (r, nxt) = output(
            &inp,
            |tp| {
                // TCP_MD5SIG clears sack_enable (tcp_ctloutput): the state the C reaches.
                tp.st.sack_enable = 0;
                tp.st.set_flags(TF_SIGNATURE);
            },
            |tp| tp.st.snd_nxt,
        );
        // No SA between these addresses: gettdbbysrcdst finds none, EPERM before the segment
        // leaves, the state as it was.
        assert_eq!(r, Err(Errno::EPERM));
        assert_eq!(sent(d), before);
        assert_eq!(nxt, 1000);
        testtcp::close(so, inp);
    }

    #[test]
    fn a_signed_connection_that_permits_sack_is_refused_in_every_build() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"touta", 70);
        queue(&so, &pattern(10));
        let before = sent(d);
        let (r, nxt) = output(
            &inp,
            |tp| {
                tp.st.sack_enable = 1;
                tp.st.set_flags(TF_SIGNATURE);
            },
            |tp| tp.st.snd_nxt,
        );
        // GENERIC's DIAGNOSTIC check: EINVAL, nothing sent, nothing changed.
        assert_eq!(r, Err(Errno::EINVAL));
        assert_eq!(sent(d), before);
        assert_eq!(nxt, 1000);
        testtcp::close(so, inp);
    }

    #[test]
    fn the_persist_timer_backs_off_up_to_the_last_shift() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tout9", 29);
        let shifts = testtcp::with_tp(&inp, |mut tp| {
            let mut v = std::vec::Vec::new();
            for _ in 0..14 {
                tcp_timer_disarm(&mut tp, TCPT_PERSIST);
                tcp_setpersist(&mut tp);
                v.push(tp.st.t_rxtshift);
            }
            (v, None)
        });
        assert_eq!(
            shifts,
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 12, 12],
            "t_rxtshift capped at TCP_MAXRXTSHIFT"
        );
        testtcp::close(so, inp);
    }

    #[test]
    fn sack_output_skips_old_and_short_holes_and_adjust_moves_snd_nxt() {
        let (_g, _p) = testtcp::setup();
        let mut st = TcpState::new();
        st.sack_enable = 1;
        st.snd_una = 1000;
        let hole = |s, e, d, r| {
            crate::netinet::tcp_subr::SACKHL_POOL
                .get(hal::pool::PR_WAITOK, || Sackhole::new(s, e, d, r))
                .expect("a hole")
        };
        // Old (rxmit below snd_una), too few dups, done (rxmit at end), then the one.
        let mut holes = SlistLink::new();
        let mut h4 = hole(4000, 4500, 3, 4100);
        let mut h3 = hole(3000, 3500, 3, 3500);
        let mut h2 = hole(2000, 2500, 1, 2000);
        let mut h1 = hole(500, 900, 3, 600);
        let _ = h4.next.set(hole(5000, 5100, 3, 5000));
        let _ = h3.next.set(h4);
        let _ = h2.next.set(h3);
        let _ = h1.next.set(h2);
        let _ = holes.set(h1);
        let p = tcp_sack_output(&st, &mut holes).expect("a hole to resend");
        assert_eq!((p.start, p.rxmit), (4000, 4100));
        st.sack_enable = 0;
        assert!(tcp_sack_output(&st, &mut holes).is_none(), "SACK off");

        // snd_nxt between two holes moves to the next one's start; past the last hole's end
        // to rcv_lastsack; inside a hole it stays.
        st.rcv_lastsack = 6000;
        for (nxt, want) in [(2600, 3000), (5100, 6000), (2100, 2100), (6000, 6000)] {
            st.snd_nxt = nxt;
            tcp_sack_adjust(&mut st, &holes);
            assert_eq!(st.snd_nxt, want, "snd_nxt {nxt}");
        }
        drop(holes);
    }
}
/* </TESTS> */
