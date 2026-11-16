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
//! TCP input: `netinet/tcp_input.c`'s `tcp_input` (N6.6a4: `tcp_input`, `tcp_input_mlist`,
//! `tcp_input_solocked`, `tcp_dooptions`, `tcp_pulloutofband`, `tcp_mss_adv`) and its path
//! parameters and round-trip timing (N6.6a3: `tcp_xmit_timer`, `tcp_mss`, `tcp_hdrsz`,
//! `tcp_mss_update`, `tcp_newreno_partialack`). The reassembly queue and the SACK scoreboard
//! are `tcp_sack.rs`'s, the SYN cache `tcp_syncache.rs`'s, the soft LRO `tcp_lro.rs`'s.
//!
//! Upstream: sys/netinet/tcp_input.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_input.rs@3c62ede68802
//!
//! [`tcp_input`] follows pages 65-76 of the protocol specification dated September, 1981
//! very closely: it checks a segment's checksum and data offset, finds the connection's
//! control block (or the listening one, whose SYN cache takes the handshake), runs header
//! prediction for the two common cases of a uni-directional transfer, and otherwise trims the
//! segment to the receive window, processes RST (at the window's edge only), SYN (a challenge
//! ACK, rate-limited) and ACK (duplicate ACKs and fast retransmit, the congestion window, the
//! states the FIN moves through), updates the send window, the urgent pointer and the data
//! (in sequence, or through the reassembly queue), and the FIN. Segments a softnet queued
//! are processed by [`tcp_input_mlist`], which keeps one socket locked across consecutive
//! segments of a connection. [`tcp_dooptions`] parses the options and checks the TCP MD5
//! signature; [`tcp_pulloutofband`] takes the urgent byte out of the data.
//!
//! [`tcp_xmit_timer`] folds a round-trip sample into the smoothed estimate and its variance
//! (fixed point, RFC 6298's shape) and recomputes the retransmit timeout; [`tcp_mss`] picks
//! the maximum segment size from the route's MTU, the interface and the peer's offer, and
//! seeds the congestion window; [`tcp_mss_update`] rounds the socket buffers to it;
//! [`tcp_hdrsz`] is the header length every segment carries; [`tcp_newreno_partialack`]
//! retransmits the next unacknowledged segment on a partial ACK; [`tcp_mss_adv`] is the
//! segment size a SYN-ACK offers.
//!
//! ## Deviations
//! - The TCP header is copied out of the mbuf after `ip6_exthdr_get` and the C's in-place
//!   `NTOHL`/`NTOHS` of `th_seq`, `th_ack`, `th_win` and `th_urp` are done on that copy,
//!   which plays the C's `struct tcphdr *th`; the mbuf keeps the header in network order
//!   (nothing reads it there again: `tcp_respond`, `tcp_signature` and the reassembly queue
//!   take the copy, the data is trimmed past it). The options are copied likewise, and the
//!   network header (`ip` or `ip6`, the C's two pointers, as `Option`s). LZ's choice.
//! - `tcp_input_solocked`'s labels are run after the body (`TcpInputExit`, `NoTpExit`, the
//!   end the body reaches with the control block open, closed or absent): `findpcb` is a
//!   loop, `step6` and `dodata` labelled blocks, the ACK switch's fall-through from
//!   `SYN_RECEIVED` an `if` before the shared arm. `tcp_close`'s pool put is `tcp_free`'s,
//!   after the label's work under the same socket lock (`drop`'s trace and free, the reset).
//! - When `syn_cache_get` aborts the connection, the C's `drop:` traces (under `SO_DEBUG`)
//!   with the listener's control block, read after `syn_cache_get` let the listener's lock
//!   go; here that record has no control block (`tcp_trace`'s NULL): the debug ring, which
//!   trpt(8) does not read (D-6).
//! - `tcp_pulloutofband` takes the control block's state (the C's `sototcpcb(so)`, the
//!   segment's own); when the urgent byte lies in a cluster another mbuf shares, the cluster
//!   is copied first (`m_copyback`), where the C moves the bytes inside the shared one; when
//!   no cluster can be had for the copy the byte stays in the data, unrecorded (the C cannot
//!   fail there).
//! - `tcp_dooptions` returns `false` for the C's -1 and takes the options as a slice (NULL
//!   is the empty one); the signature's SA is held by a `TdbHold`, given back by its drop on
//!   both of the C's paths. The SA's addresses are unions read whole (`sig_union`).
//! - `tcp_rst_ppslim_count`/`_last` and `tcp_ackdrop_ppslim_count`/`_last` are cells of
//!   `ppsratecheck_mtx`, read and written in one section of it (`pps_ok`, the `icmperrppslim`
//!   idiom of `ip_icmp.rs`).
//! - `INET6`, `TCP_ECN`, `TCP_SIGNATURE`, `IPSEC` and `NPF` are configured (GENERIC);
//!   `DIAGNOSTIC`'s `TCPS_LISTEN` panic is the feature `diagnostic`'s.
//! - Where the C dereferences a pointer that cannot be NULL there, the harmless way out is
//!   taken (LZ's): a pcb whose socket holds no witness (a `bsd` protocol's) has no control
//!   block, as `intotcpcb`'s NULL.
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
//! - N6.6a4: `tcp_input` is the kernel's (unregistered until the switch, N6.6a5; `bsd`'s
//!   stays the softnet's through the `Inet` leg). LZ's body worked on `&'static` pcbs,
//!   sockets and control blocks of `Cell`s, with `in_pcbsolock`/`in_pcbsounlock` a detached
//!   pair. Here the pcb is a counted handle (the lookup's, or pf's erased one: `InpHold`,
//!   counted once as the C's `in_pcbref`), its socket's lock a `SoLocked` witness opened in
//!   parts, and the control block a `Tp` the body takes by value: `tp = tcp_close(tp)` is a
//!   move that answers `Closed`, so a use after the close does not compile, and the frame
//!   that holds the parts frees it (`tcp_free`) before the socket lock goes.
//! - The body is three frames, each the holder of one socket's lock: `tcp_input_pcb` (the
//!   pcb found: the minimum TTL, the window, `SO_DEBUG`'s snapshot and a listener's SYN cache
//!   paths), `tcp_input_child` (`syn_cache_get`'s new connection, locked again after the
//!   listener's lock went to `syn_cache_get`), and the common `tcp_input_tcb` (the
//!   connection's processing to the output), whose end (`TcpInputEnd`) the frame runs
//!   through the labels (`TcpInputState::finish`). `findpcb` and the new connection are a
//!   loop in `tcp_input_solocked` over the frames' answers (`TcpInputNext`).
//! - `tcp_input_mlist` keeps the socket of a run of segments locked between them as a
//!   `SoKept` (the pcb's handle, `so_lock` held detached): the next segment's pcb is compared
//!   with the kept socket's `so_pcb` under the lock it carries (the C's `sotoinpcb(*solocked)
//!   == inp`), and on a match the holding is attached back to a witness (one compare and
//!   swap), else let go and the new socket locked. A frame that ends at one of the C's five
//!   hand-back exits detaches its witness for the next segment (one store); the C keeps the
//!   lock without either. The pcb's count is the lookup's, moved into the `SoKept` where the
//!   C drops its reference and takes the next lookup's: the same two counts a segment.
//! - `tcp_dooptions` works on the control block's state (`TcpState`), so `syn_cache_add`'s
//!   `struct tcpcb tb` is a `TcpState` on the stack (344 bytes, the C's 736).
//! - Costs against the C (docs/ROADMAP.md's N12 row): the attach and detach of a kept
//!   socket per segment of a run; pf's link and lookup through the `Pf` legs (one dynamic
//!   call each); the SPD check through the `Ipsec` leg; `syn_cache_get`'s new socket detached
//!   and attached again once to be handed to the frame (per connection).

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::netlock::NetHeld;
use hal::pool::AnyArc;
use hal::rwlock::Guarded;
use hal::sys::errno::Errno;
use hal::sys::net_if::IFF_LOOPBACK;
use hal::sys::netinet_in::{IPPROTO_DONE, IPPROTO_TCP, SockaddrIn};
use hal::sys::netinet_ip::{IPTOS_ECN_CE, IPTOS_ECN_MASK, Ip};
use hal::sys::netinet_ip6::{IPV6_MMTU, Ip6Frag, Ip6Hdr};
use hal::sys::netinet_tcp::{
    MAX_TCPOPTLEN, TCP_MAX_WINSHIFT, TCP_MAXWIN, TCPOLEN_MAXSEG, TCPOLEN_SACK_PERMITTED,
    TCPOLEN_SIGLEN, TCPOLEN_SIGNATURE, TCPOLEN_TIMESTAMP, TCPOLEN_TSTAMP_APPA, TCPOLEN_WINDOW,
    TCPOPT_EOL, TCPOPT_MAXSEG, TCPOPT_NOP, TCPOPT_SACK, TCPOPT_SACK_PERMITTED, TCPOPT_SIGNATURE,
    TCPOPT_TIMESTAMP, TCPOPT_TSTAMP_HDR, TCPOPT_WINDOW, TH_ACK, TH_CWR, TH_ECE, TH_FIN, TH_PUSH,
    TH_RST, TH_SYN, TH_URG, TcpSeq, Tcphdr,
};
use hal::sys::netinet6_in6::{
    SockaddrIn6, in6_are_addr_equal, in6_is_addr_multicast, in6_is_addr_unspecified,
};
use hal::sys::socket::{AF_INET, AF_INET6, SO_ACCEPTCONN, SO_DEBUG, SO_OOBINLINE};
use libkern::timingsafe_bcmp;

use crate::assert::{kassert, kpanic};
use crate::clock::Timeval;
use crate::crypto::md5::MD5_DIGEST_LENGTH;
use crate::net::if_::{if_get, unhandled_af};
use crate::net::if_var::Netstack;
use crate::net::ipsec_hooks::{SOCKADDR_UNION_LEN, TdbHold, ipsec};
use crate::net::mbuf::{
    M_BCAST, M_DONTWAIT, M_MCAST, M_PKTHDR, M_TCP_CSUM_IN_BAD, M_TCP_CSUM_IN_OK, Mbuf, MbufBox,
    MbufList, m_adj, m_copyback, ml_dequeue, ml_enqueue,
};
use crate::net::pf_hooks::pf;
use crate::net::route::Rtentry;
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::{SB_MAX_VAR, SS_CANTRCVMORE, SS_NOFDREF, SS_RCVATMARK};
use crate::net::socket::{
    SoLocked, socantrcvmore, sohasoutofband, soisconnected, soisdisconnected, solock_attach,
    sorwakeup, sounlock_detached, sowwakeup,
};
use crate::netinet::in_pcb::{
    Inpcb, InpcbRef, in_pcblookup, in_pcblookup_listen, in_pcbrtentry, in_pcbsolock,
};
use crate::netinet::in4_cksum::in4_cksum;
use crate::netinet::ip_output::ip_load;
use crate::netinet::ip_var::ip_mtudisc;
use crate::netinet::tcp_debug::{TA_DROP, TA_INPUT, tcp_trace};
use crate::netinet::tcp_fsm::{
    TCPS_CLOSE_WAIT, TCPS_CLOSED, TCPS_CLOSING, TCPS_ESTABLISHED, TCPS_FIN_WAIT_1, TCPS_FIN_WAIT_2,
    TCPS_LAST_ACK, TCPS_LISTEN, TCPS_SYN_RECEIVED, TCPS_SYN_SENT, TCPS_TIME_WAIT,
    tcps_haveestablished, tcps_havercvdfin, tcps_havercvdsyn,
};
use crate::netinet::tcp_lro::tstmp_lt;
use crate::netinet::tcp_output::tcp_output;
use crate::netinet::tcp_sack::{
    TCPREXMTTHRESH, tcp_clean_sackreport, tcp_del_sackholes, tcp_flush_queue, tcp_reass,
    tcp_sack_option, tcp_sack_partialack, tcp_seq_subtract, tcp_update_sack_list,
};
use crate::netinet::tcp_seq::{seq_geq, seq_gt, seq_leq, seq_lt, tcp_rcvseqinit};
use crate::netinet::tcp_subr::{
    Closed, TCP_ACK_ON_PUSH, TCP_DO_ECN, TCP_DO_RFC3390, TCP_MSSDFLT, tcp_close, tcp_drop,
    tcp_free, tcp_respond, tcp_signature,
};
use crate::netinet::tcp_syncache::{SynCacheGet, syn_cache_add, syn_cache_get, syn_cache_reset};
use crate::netinet::tcp_timer::{
    TCP_KEEPIDLE, TCPT_2MSL, TCPT_DELACK, TCPT_KEEP, TCPT_PERSIST, TCPT_REXMT, TCPTV_KEEPCNT,
    TCPTV_MSL, TCPTV_REXMTMAX, tcp_canceltimers, tcp_delack_msecs, tcp_timer_arm, tcp_timer_disarm,
    tcp_timer_isarmed, tcpt_rangeset,
};
use crate::netinet::tcp_usrreq::{tcp_update_rcvspace, tcp_update_sndspace};
use crate::netinet::tcp_var::{
    SynCacheSa, TCP_RTT_BASE_SHIFT, TCP_RTT_MAX, TCP_RTT_SHIFT, TCP_RTTVAR_SHIFT, TCPOOB_HADDATA,
    TCPOOB_HAVEDATA, TF_ACKNOW, TF_DISABLE_ECN, TF_ECN_PERMIT, TF_NEEDOUTPUT, TF_NOOPT,
    TF_PMTUD_PEND, TF_RCVD_CE, TF_RCVD_SCALE, TF_RCVD_TSTMP, TF_REQ_SCALE, TF_REQ_TSTMP,
    TF_SACK_PERMIT, TF_SEND_CWR, TF_SIGNATURE, TcpOptInfo, TcpState, Tcpcb, TcpstatCounters, Tp,
    intotcpcb, tcp_now, tcp_rexmtval, tcp_time, tcpstat_add, tcpstat_inc, tcpstat_pkt,
};
use crate::netinet6::in6_cksum::in6_cksum;
use crate::netinet6::in6_pcb::{in6_pcblookup, in6_pcblookup_listen};
use crate::netinet6::ip6_input::{ip6_exthdr_bytes, ip6_exthdr_get};
use crate::netinet6::ip6_output::ip6_load;
use crate::proc::ProcGlue;
use crate::time::{PPSRATECHECK_MTX, ppsratecheck_held, ppsratecheck_locked};
use crate::timeout::hz;

/// `sizeof(struct tcphdr)`.
const TCP_HDR_LEN: usize = size_of::<Tcphdr>();

/// `AF_INET` as the `int` of `af` and `tp->pf`.
const AF_INET_I32: i32 = AF_INET as i32;
/// `AF_INET6` as the `int` of `af` and `tp->pf`.
const AF_INET6_I32: i32 = AF_INET6 as i32;

/// `TCP_PAWS_IDLE`.
const TCP_PAWS_IDLE: i32 = tcp_time(24 * 24 * 60 * 60);

/// `sizeof(saveti)`: the union of `struct tcpiphdr` and `struct tcpipv6hdr` that `tcp_trace`
/// reads (the IPv6 header and the TCP header).
const SAVETI_LEN: usize = size_of::<Ip6Hdr>() + size_of::<Tcphdr>();

/// A rate limiter's state: `tcp_rst_ppslim_last` and `tcp_rst_ppslim_count`, or the
/// `tcp_ackdrop_ppslim` pair.
struct Ppslim {
    /// `*_last`.
    last: Timeval,
    /// `*_count`.
    count: i32,
}

/// Where `tcp_input_solocked` goes once the body has looked at the segment with the control
/// block open: the C's labels and its common return.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TcpInputExit {
    /// The socket goes back to the caller (or is unlocked): the end of every path that
    /// processed the segment (the output, if any, is the path's own).
    Done,
    /// `badsyn`.
    BadSyn,
    /// `dropafterack_ratelim`.
    DropAfterAckRatelim,
    /// `dropafterack`.
    DropAfterAck,
    /// `dropwithreset` (its rate-limited entry is reached only without a control block).
    DropWithReset,
    /// `drop`.
    Drop,
}

/// The labels reached with no control block (the C's `tp == NULL`: before `intotcpcb`, after
/// `tcp_close`, after `badsyn`): the ones that do not use one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoTpExit {
    /// `badsyn`.
    BadSyn,
    /// `dropwithreset_ratelim`.
    DropWithResetRatelim,
    /// `dropwithreset`.
    DropWithReset,
    /// `drop`.
    Drop,
}

/// What the body of `tcp_input_solocked` leaves for its labels: the exit and the control
/// block still open, or closed (its [`Closed`] for the frame that holds the socket lock's
/// parts to free), or none.
enum TcpInputEnd<'a, 'w, G: ProcGlue> {
    /// An exit with the control block open.
    Tp(TcpInputExit, Tp<'a, 'w, G>),
    /// An exit before there is one (`tp == NULL`).
    NoTp(NoTpExit),
    /// An exit after `tp = tcp_close(tp)` (the C's NULL).
    Closed(NoTpExit, Closed),
    /// `TIME_WAIT` reused (`goto findpcb`): the old connection closed, the initial send
    /// sequence number of the new one.
    Reuse(TcpSeq, Closed),
}

/// What the exits leave to the frame that holds the socket lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum After {
    /// `in_pcbsounlock(inp, so)`.
    Unlock,
    /// `*solocked = so`: the socket kept locked for the caller's next segment.
    HandBack,
    /// `goto findpcb` with the socket unlocked and `reuse` set.
    Findpcb(TcpSeq),
}

/// What a pcb's frame of `tcp_input_solocked` leaves to the lookup loop.
enum TcpInputNext<G: ProcGlue> {
    /// Done; nothing held (the socket unlocked, or never locked).
    Done,
    /// Done; the socket's lock kept, detached (`*solocked = so`).
    HandBack,
    /// `goto findpcb` (`TIME_WAIT` reused): the socket unlocked.
    Findpcb(TcpSeq),
    /// `syn_cache_get`'s new connection, its pcb counted and its socket locked detached: the
    /// segment goes on there (the listener's reference dropped by the caller).
    Conn(InpcbRef<G>),
}

/// The pcb a segment is for, counted (the C's `in_pcbref`): the lookup's handle, or the one
/// pf's `pf_inp_lookup` returned, kept erased so it is counted once, as the C's.
enum InpHold<G: ProcGlue> {
    /// `in_pcblookup`'s, `in_pcblookup_listen`'s, `syn_cache_get`'s.
    Pcb(InpcbRef<G>),
    /// `pf_inp_lookup`'s: a handle of an `Inpcb<G>`, checked when it is made.
    Pf(AnyArc),
}

impl<G: ProcGlue> InpHold<G> {
    /// pf's answer, when it is a pcb of this glue (anything else is no pcb).
    fn pf(a: AnyArc) -> Option<Self> {
        a.downcast_ref::<Inpcb<G>>()?;
        Some(Self::Pf(a))
    }
}

impl<G: ProcGlue> core::ops::Deref for InpHold<G> {
    type Target = Inpcb<G>;

    #[inline]
    fn deref(&self) -> &Inpcb<G> {
        match self {
            Self::Pcb(h) => h,
            Self::Pf(a) => match a.downcast_ref::<Inpcb<G>>() {
                Some(inp) => inp,
                // InpHold::pf checked it.
                None => kpanic(format_args!("tcp_input: pf's handle is not a pcb")),
            },
        }
    }
}

/// The socket `tcp_input_mlist` keeps locked between segments (its `so`, `*solocked`): the
/// pcb counted, the socket's `so_lock` held detached by the running process. Its drop is
/// `in_pcbsounlock(NULL, so)`.
#[must_use = "dropping it unlocks the socket"]
struct SoKept<G: ProcGlue> {
    /// The pcb (`None` once given to a frame).
    inp: Option<InpHold<G>>,
}

impl<G: ProcGlue> SoKept<G> {
    /// `sotoinpcb(*solocked) == inp`: the kept socket's pcb, read under the lock the holding
    /// keeps (its `so_pcb` word, whose writer holds that lock).
    fn is_of(&self, inp: &Inpcb<G>) -> bool {
        self.inp
            .as_ref()
            .is_some_and(|k| k.socket().legacy_pcb() == core::ptr::from_ref::<Inpcb<G>>(inp).addr())
    }

    /// The holding given over for `inp`'s socket, its own (`is_of`) and now the new lookup's:
    /// the kept handle dropped, `so_lock` attached back to a witness under `net`.
    fn attach<'a>(mut self, inp: &'a Inpcb<G>, net: &'a NetHeld<'a>) -> SoLocked<'a> {
        kassert!(self.is_of(inp));
        drop(self.inp.take());
        solock_attach(inp.socket(), net)
    }
}

impl<G: ProcGlue> Drop for SoKept<G> {
    fn drop(&mut self) {
        if let Some(inp) = self.inp.take() {
            sounlock_detached(inp.socket());
        }
    }
}

/// The locals of `tcp_input_solocked` that its labels and its parts read.
struct TcpInputState {
    /// `m`: the segment; `None` once consumed.
    m: Option<MbufBox>,
    /// `iphlen`.
    iphlen: i32,
    /// `af`.
    af: i32,
    /// `off`: the TCP header's length with its options.
    off: i32,
    /// `*th`: the TCP header, host order once converted.
    th: Tcphdr,
    /// `tiflags`.
    tiflags: u8,
    /// `tlen`.
    tlen: i32,
    /// `hdroptlen`.
    hdroptlen: i32,
    /// `ip`: the IPv4 header, copied (`None` for IPv6).
    ip: Option<Ip>,
    /// `ip6`: the IPv6 header, copied (`None` for IPv4).
    ip6: Option<Ip6Hdr>,
    /// `iptos` (TCP_ECN).
    iptos: u8,
    /// The options, copied: `optp`'s bytes.
    opt: [u8; MAX_TCPOPTLEN],
    /// `optlen`.
    optlen: usize,
    /// `optp != NULL`.
    have_optp: bool,
    /// `opti`.
    opti: TcpOptInfo,
    /// `now`.
    now: u64,
    /// `do_ecn`.
    do_ecn: bool,
    /// `tiwin`.
    tiwin: u64,
    /// `m->m_pkthdr.ph_rtableid`.
    rtableid: u32,
    /// `otp`: the address of the control block traced under `SO_DEBUG`, 0 for none.
    otp: usize,
    /// `ostate`.
    ostate: i32,
    /// `saveti`: the IP and TCP headers for `tcp_trace`.
    saveti: [u8; SAVETI_LEN],
    /// `reuse`: `TIME_WAIT`'s new initial send sequence number.
    reuse: Option<TcpSeq>,
    /// `solocked != NULL`: a socket is handed back to the caller.
    keep: bool,
}

/// \[a\] `tcp_rst_ppslim`: maximum outgoing RST packets per second.
pub static TCP_RST_PPSLIM: AtomicI32 = AtomicI32::new(100);
/// `tcp_rst_ppslim_count`, `tcp_rst_ppslim_last`, under `ppsratecheck_mtx`.
static TCP_RST_PPS: Guarded<Ppslim> = Guarded::new_static_mutexed(
    &PPSRATECHECK_MTX,
    Ppslim {
        last: Timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        count: 0,
    },
);

/// `tcp_ackdrop_ppslim`: 100pps.
pub static TCP_ACKDROP_PPSLIM: AtomicI32 = AtomicI32::new(100);
/// `tcp_ackdrop_ppslim_count`, `tcp_ackdrop_ppslim_last`, under `ppsratecheck_mtx`.
static TCP_ACKDROP_PPS: Guarded<Ppslim> = Guarded::new_static_mutexed(
    &PPSRATECHECK_MTX,
    Ppslim {
        last: Timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        count: 0,
    },
);

/// The `in_pcbsolock` calls of `tcp_input_solocked` (a socket kept by `tcp_input_mlist` is
/// not locked again): the host tests' count.
#[cfg(test)]
static TEST_SOLOCKS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// `TSTMP_GEQ(a, b)`: modulo comparison of timestamps.
#[inline]
const fn tstmp_geq(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) >= 0
}

/// `ppsratecheck(&lasttime, &count, lim)` of one of `tcp_input.c`'s two pairs: one section of
/// `ppsratecheck_mtx`, the pair read and written in it (the `icmperrppslim` idiom).
fn pps_ok(pps: &Guarded<Ppslim>, lim: i32) -> bool {
    ppsratecheck_locked(|l| {
        let (mut last, mut count) = {
            let p = pps.get(l.witness());
            (p.last, p.count)
        };
        let ok = ppsratecheck_held(l, &mut last, &mut count, lim);
        let p = pps.get_mut(l.witness_mut());
        p.last = last;
        p.count = count;
        ok
    })
}

/// `TCP_SETUP_ACK(tp, tiflags, m)`: compute ACK transmission behavior. Delay the ACK unless
/// we have already delayed an ACK (must send an ACK every two segments). We also ACK
/// immediately if we received a PUSH and the ACK-on-PUSH option is enabled or when the packet
/// is coming from a loopback interface.
fn tcp_setup_ack<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, tiflags: u8, m: Option<&Mbuf>) {
    // if_get lends the interface without a count: no if_put.
    let ifp = m
        .filter(|m| m.m_flags().get() & M_PKTHDR != 0)
        .and_then(|m| if_get(m.m_pkthdr().ph_ifidx.get()));
    if tcp_timer_isarmed(tp.st, TCPT_DELACK)
        || (TCP_ACK_ON_PUSH.load(Ordering::Relaxed) != 0 && tiflags & TH_PUSH != 0)
        || ifp.is_some_and(|i| i.if_flags.get() & IFF_LOOPBACK != 0)
    {
        tp.st.set_flags(TF_ACKNOW);
    } else {
        tcp_timer_arm(tp, TCPT_DELACK, tcp_delack_msecs as u64);
    }
}

/// `tcp_input` (`pr_input`): a TCP segment from IP, its header at `*offp`. Without a softnet
/// thread's stack (`ns`) it is processed now; otherwise it is queued on the stack's list for
/// [`tcp_input_mlist`], with the header offset in `ph_cookie`. The net lock is the caller's
/// (`net`, the softnet's shared holding).
#[allow(
    clippy::too_many_arguments,
    reason = "the C's arguments and the net lock's proof"
)]
pub fn tcp_input<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    _proto: i32,
    af: i32,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let Some(m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let Some(ns) = ns else {
        tcp_input_solocked::<G>(m, *offp, af, None, net);
        return IPPROTO_DONE;
    };
    // `(void *)(long)(*offp)`.
    m.m_pkthdr().ph_cookie.set(*offp as usize);
    match af {
        AF_INET_I32 => ml_enqueue(&mut ns.ns_tcp_ml, m),
        AF_INET6_I32 => ml_enqueue(&mut ns.ns_tcp6_ml, m),
        // m_freemp(mp)
        _ => drop(m),
    }
    IPPROTO_DONE
}

/// `tcp_input_mlist`: processes the segments `tcp_input` queued on a softnet's list `ml`
/// (family `af`), keeping a socket locked while consecutive segments belong to one
/// connection; the last one is unlocked after the loop. Under the softnet's shared net lock
/// (`net`).
pub fn tcp_input_mlist<G: ProcGlue>(ml: &mut MbufList, af: i32, net: &NetHeld<'_>) {
    let mut so: Option<SoKept<G>> = None;

    while let Some(m) = ml_dequeue(ml) {
        let ph = m.m_pkthdr();
        let off = ph.ph_cookie.get() as i32;
        ph.ph_cookie.set(0);
        // KASSERT(nxt == IPPROTO_DONE): tcp_input_solocked consumes the segment always.
        tcp_input_solocked(m, off, af, Some(&mut so), net);
    }

    // in_pcbsounlock(NULL, so)
    drop(so);
}

/// `tcp_input_solocked`: TCP input routine, follows pages 65-76 of the protocol specification
/// dated September, 1981 very closely. With `solocked`, the socket stays locked for the
/// caller's next segment and one it already holds is reused or unlocked.
fn tcp_input_solocked<G: ProcGlue>(
    m: MbufBox,
    iphlen: i32,
    af: i32,
    mut solocked: Option<&mut Option<SoKept<G>>>,
    net: &NetHeld<'_>,
) {
    tcpstat_inc(TcpstatCounters::TcpsRcvtotal);

    let now = tcp_now();
    // TCP_ECN
    let do_ecn = TCP_DO_ECN.load(Ordering::Relaxed) != 0;
    let Some(mut s) = TcpInputState::parse(m, iphlen, af, now, do_ecn, solocked.is_some()) else {
        return;
    };

    if s.th.th_dport == 0 {
        tcpstat_inc(TcpstatCounters::TcpsNoport);
        let _ = s.finish_notp::<G>(NoTpExit::DropWithResetRatelim, None, net);
        return;
    }

    // Locate pcb for segment.
    // NPF > 0
    let mut pfinp: Option<InpHold<G>> = match (pf(), s.m.as_deref()) {
        (Some(pf), Some(m)) => pf.inp_lookup(m).and_then(InpHold::pf),
        _ => None,
    };
    'findpcb: loop {
        let mut inp = pfinp.take().or_else(|| s.lookup::<G>());
        if inp.is_none() {
            tcpstat_inc(TcpstatCounters::TcpsPcbhashmiss);
            inp = s.lookup_listen::<G>();
            // If the state is CLOSED (i.e., TCB does not exist) then all data in the incoming
            // segment is discarded. If the TCB exists but is in CLOSED state, it is
            // embryonic, but should either do a listen or a connect soon.
        }
        // IPSEC: while IPsec is in use, the inbound policy (the leg's sequence: the innermost
        // IPsec tag's SA, ipsp_spd_lookup, tdb_unref); a refusal drops the segment.
        if let (Some(legs), Some(m)) = (ipsec(), s.m.as_deref()) {
            let seclevel = inp.as_deref().map(|i| i.inp_seclevel.get());
            if legs
                .spd_check_in(m, s.af, s.iphlen, seclevel.as_ref())
                .is_err()
            {
                tcpstat_inc(TcpstatCounters::TcpsRcvnosec);
                let _ = s.finish_notp::<G>(NoTpExit::Drop, None, net);
                return;
            }
        }

        let Some(mut inp) = inp else {
            tcpstat_inc(TcpstatCounters::TcpsNoport);
            let _ = s.finish_notp::<G>(NoTpExit::DropWithResetRatelim, None, net);
            return;
        };
        let mut next = {
            // Avoid needless lock and unlock operation when handling multiple TCP packets
            // from the same stream consecutively.
            let kept = solocked.as_deref_mut().and_then(Option::take);
            let w = match kept {
                Some(k) if k.is_of(&inp) => Some(k.attach(&inp, net)),
                other => {
                    // in_pcbsounlock(NULL, *solocked)
                    drop(other);
                    #[cfg(test)]
                    TEST_SOLOCKS.fetch_add(1, Ordering::Relaxed);
                    in_pcbsolock(&inp, net)
                }
            };
            let Some(w) = w else {
                tcpstat_inc(TcpstatCounters::TcpsClosing);
                let _ = s.finish_notp::<G>(NoTpExit::DropWithResetRatelim, None, net);
                return;
            };
            // KASSERT(sotoinpcb(inp->inp_socket) == inp): in_pcbsolock and is_of checked it;
            // soassertlocked(inp->inp_socket): `w`.
            tcp_input_pcb(&mut s, &inp, w, net)
        };
        loop {
            match next {
                TcpInputNext::Done => return,
                TcpInputNext::HandBack => {
                    if let Some(k) = solocked {
                        *k = Some(SoKept { inp: Some(inp) });
                    }
                    return;
                }
                TcpInputNext::Findpcb(iss) => {
                    s.reuse = Some(iss);
                    // in_pcbunref(inp); inp = NULL
                    drop(inp);
                    continue 'findpcb;
                }
                TcpInputNext::Conn(ninp) => {
                    // in_pcbunref(inp): the listener's; syn_cache_get counted the new pcb.
                    drop(inp);
                    inp = InpHold::Pcb(ninp);
                    let w = solock_attach(inp.socket(), net);
                    next = tcp_input_child(&mut s, &inp, w, net);
                }
            }
        }
    }
}

/// The frame of a pcb found for the segment, its socket locked (`w`): the minimum TTL, the
/// control block, the window, `SO_DEBUG`'s snapshot and a listener's SYN cache paths, then
/// the connection's processing ([`tcp_input_tcb`]) and the labels.
fn tcp_input_pcb<G: ProcGlue>(
    s: &mut TcpInputState,
    inp: &Inpcb<G>,
    mut w: SoLocked<'_>,
    net: &NetHeld<'_>,
) -> TcpInputNext<G> {
    // Check the minimum TTL for socket.
    let low = match (&s.ip, &s.ip6) {
        (Some(ip), _) => {
            let minttl = inp.inp_ip_minttl.get();
            minttl != 0 && minttl > ip.ip_ttl
        }
        (None, Some(ip6)) => {
            let minhlim = inp.inp_ip6_minhlim().get();
            minhlim != 0 && minhlim > ip6.ip6_hlim
        }
        (None, None) => false,
    };
    if low {
        let a = s.finish_notp::<G>(NoTpExit::Drop, None, net);
        return settle(w, a);
    }

    // A kernel control block belongs to a kernel protocol's socket, held by a witness (D-1);
    // without one there is no control block to find (the C's NULL).
    let Some(mut il) = w.inet() else {
        let a = s.finish_notp::<G>(NoTpExit::DropWithResetRatelim, None, net);
        return settle(w, a);
    };
    let end = 'end: {
        let Some(mut tp) = intotcpcb(inp, &mut il) else {
            break 'end TcpInputEnd::NoTp(NoTpExit::DropWithResetRatelim);
        };
        if tp.st.t_state == TCPS_CLOSED {
            break 'end TcpInputEnd::Tp(TcpInputExit::Drop, tp);
        }

        // Unscale the window into a 32-bit value.
        s.tiwin = if s.tiflags & TH_SYN == 0 {
            u64::from(s.th.th_win) << tp.st.snd_scale
        } else {
            u64::from(s.th.th_win)
        };

        let so = tp.so;
        if so.has_options(SO_DEBUG | SO_ACCEPTCONN) {
            let (src, dst) = s.sa_pair();

            if so.has_options(SO_DEBUG) {
                s.otp = core::ptr::from_ref::<Tcpcb<G>>(&tp.cb).addr();
                s.ostate = tp.st.t_state;
                s.save_ti();
            }
            if so.has_options(SO_ACCEPTCONN) {
                match s.tiflags & (TH_RST | TH_SYN | TH_ACK) {
                    // TH_SYN|TH_ACK|TH_RST, TH_SYN|TH_RST, TH_ACK|TH_RST, TH_RST
                    f if f & TH_RST != 0 => {
                        syn_cache_reset::<G>(&src, &dst, &s.th, inp.inp_rtableid.get(), net);
                        break 'end TcpInputEnd::Tp(TcpInputExit::Drop, tp);
                    }

                    f if f == TH_SYN | TH_ACK => {
                        // Received a SYN,ACK. This should never happen while we are in LISTEN.
                        // Send an RST.
                        break 'end TcpInputEnd::Tp(TcpInputExit::BadSyn, tp);
                    }

                    TH_ACK => {
                        // The listener's lock goes to syn_cache_get, which lets it go.
                        drop(tp);
                        drop(il);
                        let Some(m) = s.m.take() else {
                            kpanic(format_args!("tcp_input: the segment is gone"));
                        };
                        let th = s.th;
                        return match syn_cache_get::<G>(
                            &src, &dst, &th, inp, w, m, s.now, s.do_ecn, net,
                        ) {
                            SynCacheGet::NotFound(m) => {
                                // We don't have a SYN for this ACK; send an RST. The listener
                                // is unlocked (so = NULL).
                                s.m = Some(m);
                                let _ = s.finish_notp::<G>(NoTpExit::BadSyn, None, net);
                                TcpInputNext::Done
                            }
                            SynCacheGet::Aborted => {
                                // We were unable to create the connection. If the 3-way
                                // handshake was completed, and RST has been sent to the peer.
                                // Since the mbuf might be in use for the reply, do not free
                                // it. The listener is unlocked: the trace has no control
                                // block (see the deviations).
                                let _ = s.finish_notp::<G>(NoTpExit::Drop, None, net);
                                TcpInputNext::Done
                            }
                            // We have created a full-blown connection.
                            SynCacheGet::Socket(ninp, m) => {
                                s.m = Some(m);
                                TcpInputNext::Conn(ninp)
                            }
                        };
                    }

                    TH_SYN => {
                        // Received a SYN.

                        // LISTEN socket received a SYN from itself? This can't possibly be
                        // valid; drop the packet.
                        if s.th.th_dport == s.th.th_sport && s.self_addressed() {
                            tcpstat_inc(TcpstatCounters::TcpsBadsyn);
                            break 'end TcpInputEnd::Tp(TcpInputExit::Drop, tp);
                        }

                        // SYN looks ok; create compressed TCP state for it.
                        if so.qlen() > so.qlimit() {
                            tcpstat_inc(TcpstatCounters::TcpsDropsyn);
                            break 'end TcpInputEnd::Tp(TcpInputExit::Drop, tp);
                        }
                        let Some(m) = s.m.take() else {
                            kpanic(format_args!("tcp_input: the segment is gone"));
                        };
                        let optp = s.have_optp.then_some(&s.opt[..s.optlen]);
                        if let Err(m) = syn_cache_add(
                            &src,
                            &dst,
                            &s.th,
                            s.iphlen,
                            &mut tp,
                            m,
                            optp,
                            &mut s.opti,
                            s.reuse,
                            s.now,
                            s.do_ecn,
                        ) {
                            s.m = Some(m);
                            tcpstat_inc(TcpstatCounters::TcpsDropsyn);
                            break 'end TcpInputEnd::Tp(TcpInputExit::Drop, tp);
                        }
                        // The listener goes back to the caller, or is unlocked.
                        break 'end TcpInputEnd::Tp(TcpInputExit::Done, tp);
                    }

                    _ => {
                        // None of RST, SYN or ACK was set. This is an invalid packet for a TCB
                        // in LISTEN state. Send a RST.
                        break 'end TcpInputEnd::Tp(TcpInputExit::BadSyn, tp);
                    }
                }
            }
        }

        tcp_input_tcb(s, inp, tp)
    };
    let (closed, a) = s.finish(end, net);
    if let Some(c) = closed {
        tcp_free(inp, &mut il.pcb, c);
    }
    drop(il);
    settle(w, a)
}

/// The frame of `syn_cache_get`'s new connection (`inp`, its socket locked again, `w`): its
/// control block (`goto badsyn` without one, the C's XXX), then [`tcp_input_tcb`] and the
/// labels.
fn tcp_input_child<G: ProcGlue>(
    s: &mut TcpInputState,
    inp: &Inpcb<G>,
    mut w: SoLocked<'_>,
    net: &NetHeld<'_>,
) -> TcpInputNext<G> {
    let Some(mut il) = w.inet() else {
        let a = s.finish_notp::<G>(NoTpExit::BadSyn, None, net);
        return settle(w, a);
    };
    let end = match intotcpcb(inp, &mut il) {
        Some(tp) => tcp_input_tcb(s, inp, tp),
        None => TcpInputEnd::NoTp(NoTpExit::BadSyn),
    };
    let (closed, a) = s.finish(end, net);
    if let Some(c) = closed {
        tcp_free(inp, &mut il.pcb, c);
    }
    drop(il);
    settle(w, a)
}

/// The socket lock as the labels left it: kept for the caller (detached), or let go.
fn settle<G: ProcGlue>(w: SoLocked<'_>, a: After) -> TcpInputNext<G> {
    match a {
        After::HandBack => {
            w.detach();
            TcpInputNext::HandBack
        }
        After::Unlock => {
            drop(w);
            TcpInputNext::Done
        }
        After::Findpcb(iss) => {
            drop(w);
            TcpInputNext::Findpcb(iss)
        }
    }
}

impl TcpInputState {
    /// The C's header checks of `m` (`iphlen` bytes of IP header before the TCP one, family
    /// `af`), to the conversion of the header to host order: the broadcast and multicast
    /// segments, the header and its options made contiguous (`ip6_exthdr_get`, the first 20
    /// bytes whatever `th_off` says), the IPv6 source and destination, the checksum, the
    /// data offset (below 5 words or past the segment: `tcps_rcvbadoff`), the timestamps'
    /// fast path. `None` when the segment is dropped (or freed by a failed pullup).
    fn parse(m: MbufBox, iphlen: i32, af: i32, now: u64, do_ecn: bool, keep: bool) -> Option<Self> {
        let mut opti = TcpOptInfo {
            ts_present: false,
            maxseg: 0,
            ..TcpOptInfo::default()
        };

        // RFC1122 4.2.3.10, p. 104: discard bcast/mcast SYN
        if m.m_flags().get() & (M_BCAST | M_MCAST) != 0 {
            return None;
        }

        // Get IP and TCP header together in first mbuf. Note: IP leaves IP header in first
        // mbuf.
        let Some((m, at, o)) = ip6_exthdr_get(m, iphlen, TCP_HDR_LEN as i32) else {
            tcpstat_inc(TcpstatCounters::TcpsRcvshort);
            return None;
        };
        let mut th: Tcphdr = ip6_exthdr_bytes(&m, at, o, TCP_HDR_LEN)
            .and_then(|b| hal::pod::at(b, 0))
            .unwrap_or_default();

        let mut tlen = m.m_pkthdr().len.get() - iphlen;
        let (ip, ip6, iptos) = match af {
            AF_INET_I32 => {
                let ip = ip_load(&m);
                // save ip_tos before clearing it for checksum
                (Some(ip), None, ip.ip_tos)
            }
            AF_INET6_I32 => {
                let ip6 = ip6_load(&m);
                let iptos = ((u32::from_be(ip6.ip6_flow) >> 20) & 0xff) as u8;

                // Be proactive about unspecified IPv6 address in source. As we use all-zero
                // to indicate unbounded/unconnected pcb, unspecified IPv6 address can be used
                // to confuse us.
                //
                // Note that packets with unspecified IPv6 destination is already dropped in
                // ip6_input.
                if in6_is_addr_unspecified(&ip6.ip6_src) {
                    // XXX stat
                    return None;
                }

                // Discard packets to multicast
                if in6_is_addr_multicast(&ip6.ip6_dst) {
                    // XXX stat
                    return None;
                }
                (None, Some(ip6), iptos)
            }
            _ => unhandled_af(af),
        };

        // Checksum extended TCP header and data.
        let csum = m.m_pkthdr().csum_flags.get();
        if csum & M_TCP_CSUM_IN_OK == 0 {
            if csum & M_TCP_CSUM_IN_BAD != 0 {
                tcpstat_inc(TcpstatCounters::TcpsRcvbadsum);
                return None;
            }
            tcpstat_inc(TcpstatCounters::TcpsInswcsum);
            let sum = match af {
                AF_INET6_I32 => in6_cksum(&m, IPPROTO_TCP as u8, iphlen as u32, tlen as u32),
                _ => in4_cksum(&m, IPPROTO_TCP as u8, iphlen, tlen),
            };
            if sum != 0 {
                tcpstat_inc(TcpstatCounters::TcpsRcvbadsum);
                return None;
            }
        }

        // Check that TCP offset makes sense, pull out TCP options and adjust length. XXX
        let off = i32::from(th.th_off()) << 2;
        if off < TCP_HDR_LEN as i32 || off > tlen {
            tcpstat_inc(TcpstatCounters::TcpsRcvbadoff);
            return None;
        }
        tlen -= off;
        let mut opt = [0u8; MAX_TCPOPTLEN];
        let mut optlen = 0usize;
        let mut have_optp = false;
        let mut m = m;
        if off > TCP_HDR_LEN as i32 {
            let Some((m2, at, o)) = ip6_exthdr_get(m, iphlen, off) else {
                tcpstat_inc(TcpstatCounters::TcpsRcvshort);
                return None;
            };
            m = m2;
            // `th_off` is four bits: at most MAX_TCPOPTLEN bytes of options.
            optlen = off as usize - TCP_HDR_LEN;
            kassert!(optlen <= MAX_TCPOPTLEN);
            let Some(b) = ip6_exthdr_bytes(&m, at, o, off as usize) else {
                kpanic(format_args!("tcp_input: the pulled up header is not there"));
            };
            th = hal::pod::at(b, 0).unwrap_or(th);
            opt[..optlen].copy_from_slice(&b[TCP_HDR_LEN..]);
            have_optp = true;
            // Do quick retrieval of timestamp options ("options prediction?"). If timestamp
            // is the only option and it's formatted as recommended in RFC 1323 appendix A, we
            // quickly get the values now and not bother calling tcp_dooptions(), etc.
            let appa = usize::from(TCPOLEN_TSTAMP_APPA);
            let be32 = |i: usize| u32::from_be_bytes([opt[i], opt[i + 1], opt[i + 2], opt[i + 3]]);
            if (optlen == appa || (optlen > appa && opt[appa] == TCPOPT_EOL))
                && be32(0) == TCPOPT_TSTAMP_HDR
                && th.th_flags & TH_SYN == 0
            {
                opti.ts_present = true;
                opti.ts_val = be32(4);
                opti.ts_ecr = be32(8);
                // we've parsed the options
                have_optp = false;
            }
        }
        let tiflags = th.th_flags;

        // Convert TCP protocol specific fields to host format.
        th.th_seq = u32::from_be(th.th_seq);
        th.th_ack = u32::from_be(th.th_ack);
        th.th_win = u16::from_be(th.th_win);
        th.th_urp = u16::from_be(th.th_urp);

        let rtableid = m.m_pkthdr().ph_rtableid.get();
        Some(Self {
            m: Some(m),
            iphlen,
            af,
            off,
            th,
            tiflags,
            tlen,
            hdroptlen: 0,
            ip,
            ip6,
            iptos,
            opt,
            optlen,
            have_optp,
            opti,
            now,
            do_ecn,
            tiwin: 0,
            rtableid,
            otp: 0,
            ostate: 0,
            saveti: [0; SAVETI_LEN],
            reuse: None,
            keep,
        })
    }

    /// The segment, while it is held (every path that reads it after consuming it returns).
    fn seg(&self) -> &Mbuf {
        match &self.m {
            Some(m) => m,
            None => kpanic(format_args!("tcp_input: the segment is gone")),
        }
    }

    /// `in_pcblookup` / `in6_pcblookup` of the connection, counted.
    fn lookup<G: ProcGlue>(&self) -> Option<InpHold<G>> {
        let t = &crate::net::domain::net::<G>().tcp;
        let inp = match (&self.ip, &self.ip6) {
            (None, Some(ip6)) => in6_pcblookup(
                &t.tcb6table,
                &ip6.ip6_src,
                self.th.th_sport,
                &ip6.ip6_dst,
                self.th.th_dport,
                self.rtableid,
            ),
            (Some(ip), _) => in_pcblookup(
                &t.tcbtable,
                ip.ip_src,
                self.th.th_sport,
                ip.ip_dst,
                self.th.th_dport,
                self.rtableid,
            ),
            (None, None) => None,
        };
        inp.map(InpHold::Pcb)
    }

    /// `in_pcblookup_listen` / `in6_pcblookup_listen` of the segment's destination, counted.
    fn lookup_listen<G: ProcGlue>(&self) -> Option<InpHold<G>> {
        let t = &crate::net::domain::net::<G>().tcp;
        let m = self.m.as_deref();
        let inp = match (&self.ip, &self.ip6) {
            (None, Some(ip6)) => in6_pcblookup_listen(
                &t.tcb6table,
                &ip6.ip6_dst,
                self.th.th_dport,
                m,
                self.rtableid,
            ),
            (Some(ip), _) => {
                in_pcblookup_listen(&t.tcbtable, ip.ip_dst, self.th.th_dport, m, self.rtableid)
            }
            (None, None) => None,
        };
        inp.map(InpHold::Pcb)
    }

    /// The SYN cache's `src` and `dst` of the segment: each `union syn_cache_sa` zeroed, then
    /// its `sin` or `sin6` member's length, family, address and port.
    fn sa_pair(&self) -> (SynCacheSa, SynCacheSa) {
        let mut src: SynCacheSa = [0; SOCKADDR_UNION_LEN];
        let mut dst: SynCacheSa = [0; SOCKADDR_UNION_LEN];
        match (&self.ip, &self.ip6) {
            (None, Some(ip6)) => {
                for (sa, addr, port) in [
                    (&mut src, &ip6.ip6_src, self.th.th_sport),
                    (&mut dst, &ip6.ip6_dst, self.th.th_dport),
                ] {
                    sa[0] = size_of::<SockaddrIn6>() as u8;
                    sa[1] = AF_INET6;
                    sa[2..4].copy_from_slice(&port.to_ne_bytes());
                    sa[8..24].copy_from_slice(&addr.s6_addr);
                }
            }
            (Some(ip), _) => {
                for (sa, addr, port) in [
                    (&mut src, ip.ip_src, self.th.th_sport),
                    (&mut dst, ip.ip_dst, self.th.th_dport),
                ] {
                    sa[0] = size_of::<SockaddrIn>() as u8;
                    sa[1] = AF_INET;
                    sa[2..4].copy_from_slice(&port.to_ne_bytes());
                    sa[4..8].copy_from_slice(&addr.s_addr.to_ne_bytes());
                }
            }
            (None, None) => {}
        }
        (src, dst)
    }

    /// `saveti`: the network header and the TCP header (host order) for `tcp_trace`
    /// (`struct tcpiphdr` or `struct tcpipv6hdr`).
    fn save_ti(&mut self) {
        let hlen = match (&self.ip, &self.ip6) {
            (None, Some(ip6)) => {
                let _ = hal::pod::put(&mut self.saveti, 0, ip6);
                size_of::<Ip6Hdr>()
            }
            (Some(ip), _) => {
                let _ = hal::pod::put(&mut self.saveti, 0, ip);
                size_of::<Ip>()
            }
            (None, None) => 0,
        };
        let _ = hal::pod::put(&mut self.saveti, hlen, &self.th);
    }

    /// A listener's SYN from its own address (`th_dport == th_sport` is the caller's test).
    fn self_addressed(&self) -> bool {
        match (&self.ip, &self.ip6) {
            (None, Some(ip6)) => in6_are_addr_equal(&ip6.ip6_src, &ip6.ip6_dst),
            (Some(ip), _) => ip.ip_dst.s_addr == ip.ip_src.s_addr,
            (None, None) => false,
        }
    }

    /// Where the body ends, through the labels: what the frame frees (`tcp_close`'s
    /// [`Closed`]) and what it does with the socket lock.
    fn finish<G: ProcGlue>(
        &mut self,
        end: TcpInputEnd<'_, '_, G>,
        net: &NetHeld<'_>,
    ) -> (Option<Closed>, After) {
        match end {
            TcpInputEnd::Tp(exit, tp) => (None, self.finish_tp(exit, tp, net)),
            TcpInputEnd::NoTp(exit) => (None, self.finish_notp::<G>(exit, None, net)),
            TcpInputEnd::Closed(exit, c) => (Some(c), self.finish_notp::<G>(exit, None, net)),
            TcpInputEnd::Reuse(iss, c) => (Some(c), After::Findpcb(iss)),
        }
    }

    /// `*solocked = so` or `in_pcbsounlock(inp, so)`: the common return.
    fn done(&self) -> After {
        if self.keep {
            After::HandBack
        } else {
            After::Unlock
        }
    }

    /// The labels with the control block `tp` open.
    fn finish_tp<G: ProcGlue>(
        &mut self,
        exit: TcpInputExit,
        mut tp: Tp<'_, '_, G>,
        net: &NetHeld<'_>,
    ) -> After {
        let mut exit = exit;
        loop {
            let notp = match exit {
                TcpInputExit::Done => return self.done(),
                TcpInputExit::DropAfterAckRatelim => {
                    if !pps_ok(&TCP_ACKDROP_PPS, TCP_ACKDROP_PPSLIM.load(Ordering::Relaxed)) {
                        // XXX stat
                        NoTpExit::Drop
                    } else {
                        // ...fall into dropafterack...
                        exit = TcpInputExit::DropAfterAck;
                        continue;
                    }
                }
                TcpInputExit::DropAfterAck => {
                    // Generate an ACK dropping incoming segment if it occupies sequence space,
                    // where the ACK reflects our state.
                    if self.tiflags & TH_RST != 0 {
                        NoTpExit::Drop
                    } else {
                        drop(self.m.take());
                        tp.st.set_flags(TF_ACKNOW);
                        let _ = tcp_output(&mut tp);
                        return self.done();
                    }
                }
                TcpInputExit::BadSyn => NoTpExit::BadSyn,
                TcpInputExit::DropWithReset => NoTpExit::DropWithReset,
                TcpInputExit::Drop => NoTpExit::Drop,
            };
            return self.finish_notp(notp, Some(&tp), net);
        }
    }

    /// The labels that read the control block only for the reset's window and the trace
    /// (`tp`, or the C's NULL).
    fn finish_notp<G: ProcGlue>(
        &mut self,
        exit: NoTpExit,
        tp: Option<&Tp<'_, '_, G>>,
        net: &NetHeld<'_>,
    ) -> After {
        let mut exit = exit;
        let mut tp = tp;
        loop {
            match exit {
                NoTpExit::BadSyn => {
                    // Received a bad SYN. Increment counters and dropwithreset.
                    tcpstat_inc(TcpstatCounters::TcpsBadsyn);
                    tp = None;
                    exit = NoTpExit::DropWithReset;
                }
                NoTpExit::DropWithResetRatelim => {
                    // We may want to rate-limit RSTs in certain situations, particularly if
                    // we are sending an RST in response to an attempt to connect to or
                    // otherwise communicate with a port for which we have no socket.
                    exit = if pps_ok(&TCP_RST_PPS, TCP_RST_PPSLIM.load(Ordering::Relaxed)) {
                        // ...fall into dropwithreset...
                        NoTpExit::DropWithReset
                    } else {
                        // XXX stat
                        NoTpExit::Drop
                    };
                }
                NoTpExit::DropWithReset => {
                    // Generate a RST, dropping incoming segment. Make ACK acceptable to
                    // originator of segment. Don't bother to respond to RST.
                    if self.tiflags & TH_RST != 0 {
                        exit = NoTpExit::Drop;
                        continue;
                    }
                    if let Some(m) = self.m.take() {
                        let rtableid = m.m_pkthdr().ph_rtableid.get();
                        if self.tiflags & TH_ACK != 0 {
                            tcp_respond(
                                tp,
                                m.data(),
                                Some(&self.th),
                                0,
                                self.th.th_ack,
                                TH_RST,
                                rtableid,
                                self.now,
                                net,
                            );
                        } else {
                            if self.tiflags & TH_SYN != 0 {
                                self.tlen += 1;
                            }
                            tcp_respond(
                                tp,
                                m.data(),
                                Some(&self.th),
                                self.th.th_seq.wrapping_add(self.tlen as u32),
                                0,
                                TH_RST | TH_ACK,
                                rtableid,
                                self.now,
                                net,
                            );
                        }
                        // m_freem(m)
                        drop(m);
                    }
                    return After::Unlock;
                }
                NoTpExit::Drop => {
                    // Drop space held by incoming segment and return.
                    if self.otp != 0 {
                        tcp_trace(
                            TA_DROP,
                            self.ostate,
                            tp.map(|t| &*t.st),
                            self.otp,
                            Some(&self.saveti),
                            0,
                            self.tlen,
                        );
                    }
                    drop(self.m.take());
                    return After::Unlock;
                }
            }
        }
    }
}

/// The body of `tcp_input_solocked` for a connection's control block `tp` (of the pcb `inp`),
/// from `pf_inp_link` to the output, ending at the label it reaches.
fn tcp_input_tcb<'a, 'w, G: ProcGlue>(
    s: &mut TcpInputState,
    inp: &Inpcb<G>,
    mut tp: Tp<'a, 'w, G>,
) -> TcpInputEnd<'a, 'w, G> {
    use TcpInputEnd::{Closed as Cl, Reuse, Tp as T};
    use TcpInputExit::{Done, Drop, DropAfterAck, DropAfterAckRatelim, DropWithReset};

    let so = tp.so;
    let now = s.now;

    // Should not happen now that all embryonic connections are handled with compressed
    // state.
    if cfg!(feature = "diagnostic") && tp.st.t_state == TCPS_LISTEN {
        kpanic(format_args!("tcp_input: TCPS_LISTEN"));
    }

    // NPF > 0
    if let Some(pf) = pf() {
        let a: &(dyn core::any::Any + Sync) = inp;
        pf.inp_link(s.seg(), a);
    }

    // Segment received on connection. Reset idle time and keep-alive timer.
    tp.st.t_rcvtime = now;
    if tcps_haveestablished(tp.st.t_state) {
        tcp_timer_arm(
            &mut tp,
            TCPT_KEEP,
            TCP_KEEPIDLE.load(Ordering::Relaxed) as u64,
        );
    }

    if tp.st.sack_enable != 0 {
        // Delete stale SACK holes
        tcp_del_sackholes(tp.st, &s.th);
    }

    // Process options.
    if s.have_optp || tp.st.has_flags(TF_SIGNATURE) {
        let optp: &[u8] = if s.have_optp { &s.opt[..s.optlen] } else { &[] };
        let Some(m) = s.m.as_deref() else {
            kpanic(format_args!("tcp_input: the segment is gone"));
        };
        if !tcp_dooptions(
            tp.st,
            optp,
            &s.th,
            m,
            s.iphlen,
            &mut s.opti,
            s.rtableid,
            now,
        ) {
            return T(Drop, tp);
        }
    }

    if s.opti.ts_present && s.opti.ts_ecr != 0 {
        // subtract out the tcp timestamp modulator
        s.opti.ts_ecr = s.opti.ts_ecr.wrapping_sub(tp.st.ts_modulate);

        // make sure ts_ecr is sensible: `int32_t rtt_test = now - opti.ts_ecr`.
        let rtt_test = now.wrapping_sub(u64::from(s.opti.ts_ecr)) as i32;
        if !(0..=TCP_RTT_MAX).contains(&rtt_test) {
            s.opti.ts_ecr = 0;
        }
    }

    // TCP_ECN: if congestion experienced, set ECE bit in subsequent packets.
    if s.iptos & IPTOS_ECN_MASK == IPTOS_ECN_CE {
        tp.st.set_flags(TF_RCVD_CE);
        tcpstat_inc(TcpstatCounters::TcpsEcnRcvce);
    }

    // Header prediction: check for the two common cases of a uni-directional data xfer. If
    // the packet has no control flags, is in-sequence, the window didn't change and we're
    // not retransmitting, it's a candidate. If the length is zero and the ack moved forward,
    // we're the sender side of the xfer. Just free the data acked & wake any higher level
    // process that was blocked waiting for space. If the length is non-zero and the ack
    // didn't move, we're the receiver side. If we're getting packets in-order (the
    // reassembly queue is empty), add the data to the socket buffer and note that we need a
    // delayed ack.
    if tp.st.t_state == TCPS_ESTABLISHED
        && s.tiflags & (TH_SYN | TH_FIN | TH_RST | TH_URG | TH_ECE | TH_CWR | TH_ACK) == TH_ACK
        && (!s.opti.ts_present || tstmp_geq(s.opti.ts_val, tp.st.ts_recent))
        && s.th.th_seq == tp.st.rcv_nxt
        && s.tiwin != 0
        && s.tiwin == tp.st.snd_wnd
        && tp.st.snd_nxt == tp.st.snd_max
    {
        // If last ACK falls within this segment's sequence numbers, record the timestamp.
        // Fix from Braden, see Stevens p. 870
        if s.opti.ts_present && seq_leq(s.th.th_seq, tp.st.last_ack_sent) {
            tp.st.ts_recent_age = now;
            tp.st.ts_recent = s.opti.ts_val;
        }

        if s.tlen == 0 {
            let st = &*tp.st;
            if seq_gt(s.th.th_ack, st.snd_una)
                && seq_leq(s.th.th_ack, st.snd_max)
                && st.snd_cwnd >= st.snd_wnd
                && st.t_dupacks == 0
            {
                // this is a pure ack for outstanding data.
                tcpstat_inc(TcpstatCounters::TcpsPredack);
                let st = &mut *tp.st;
                if s.opti.ts_present && s.opti.ts_ecr != 0 {
                    tcp_xmit_timer(st, now.wrapping_sub(u64::from(s.opti.ts_ecr)) as i32);
                } else if st.t_rtttime != 0 && seq_gt(s.th.th_ack, st.t_rtseq) {
                    tcp_xmit_timer(st, now.wrapping_sub(st.t_rtttime) as i32);
                }
                let acked = s.th.th_ack.wrapping_sub(st.snd_una) as i32;
                tcpstat_pkt(
                    TcpstatCounters::TcpsRcvackpack,
                    TcpstatCounters::TcpsRcvackbyte,
                    acked as u64,
                );
                st.t_rcvacktime = now;

                so.so_snd.lock().sbdrop(acked);

                // If we had a pending ICMP message that refers to data that have just been
                // acknowledged, disregard the recorded ICMP message.
                if st.has_flags(TF_PMTUD_PEND) && seq_gt(s.th.th_ack, st.t_pmtud_th_seq) {
                    st.clear_flags(TF_PMTUD_PEND);
                }

                // Keep track of the largest chunk of data acknowledged since last PMTU update
                if st.t_pmtud_mss_acked < acked as u32 {
                    st.t_pmtud_mss_acked = acked as u32;
                }

                st.snd_una = s.th.th_ack;
                // Pull snd_wl2 up to prevent seq wrap.
                st.snd_wl2 = s.th.th_ack;
                // We want snd_last to track snd_una so as to avoid sequence wraparound
                // problems for very large transfers.
                // TCP_ECN
                if seq_gt(st.snd_una, st.snd_last) {
                    st.snd_last = st.snd_una;
                }
                // m_freem(m)
                drop(s.m.take());

                // If all outstanding data are acked, stop retransmit timer, otherwise restart
                // timer using current (possibly backed-off) value. If process is waiting for
                // space, wakeup/selwakeup/signal. If data are ready to send, let tcp_output
                // decide between more output or persist.
                if tp.st.snd_una == tp.st.snd_max {
                    tcp_timer_disarm(&mut tp, TCPT_REXMT);
                } else if !tcp_timer_isarmed(tp.st, TCPT_PERSIST) {
                    let rxtcur = tp.st.t_rxtcur;
                    tcp_timer_arm(&mut tp, TCPT_REXMT, rxtcur as u64);
                }

                tcp_update_sndspace(&tp);
                if so.so_snd.sb_notify() {
                    sowwakeup(so);
                }
                if so.so_snd.cc() != 0 || tp.st.has_flags(TF_NEEDOUTPUT) {
                    let _ = tcp_output(&mut tp);
                }
                return T(Done, tp);
            }
        } else if s.th.th_ack == tp.st.snd_una
            && !tp.st.t_segq.is_some()
            && i64::from(s.tlen) <= so.so_rcv.sbspace()
        {
            // This is a pure, in-sequence data packet with nothing on the reassembly queue
            // and we have enough buffer space to take it.
            let st = &mut *tp.st;
            // Clean receiver SACK report if present
            if st.sack_enable != 0 && st.rcv_numsacks != 0 {
                tcp_clean_sackreport(st);
            }
            tcpstat_inc(TcpstatCounters::TcpsPreddat);
            st.rcv_nxt = st.rcv_nxt.wrapping_add(s.tlen as u32);
            // Pull snd_wl1 and rcv_up up to prevent seq wrap.
            st.snd_wl1 = s.th.th_seq;
            // Packet has most recent segment, no urgent exists.
            st.rcv_up = st.rcv_nxt;
            tcpstat_pkt(
                TcpstatCounters::TcpsRcvpack,
                TcpstatCounters::TcpsRcvbyte,
                s.tlen as u64,
            );

            tcp_setup_ack(&mut tp, s.tiflags, s.m.as_deref());
            // Drop TCP, IP headers and TCP options then add data to socket buffer.
            let m = s.m.take();
            if so.so_rcv.has_state(SS_CANTRCVMORE) {
                // m_freem(m)
                drop(m);
            } else {
                let st = &mut *tp.st;
                let srtt = st.t_srtt;
                if srtt != 0
                    && st.rfbuf_ts != 0
                    && now.wrapping_sub(st.rfbuf_ts)
                        > (srtt >> (TCP_RTT_SHIFT + TCP_RTT_BASE_SHIFT)) as u64
                {
                    tcp_update_rcvspace(&tp);
                    // Start over with next RTT.
                    tp.st.rfbuf_cnt = 0;
                    tp.st.rfbuf_ts = 0;
                } else {
                    st.rfbuf_cnt = st.rfbuf_cnt.wrapping_add(s.tlen as u32);
                }
                if let Some(m) = m {
                    m_adj(&m, s.iphlen + s.off);
                    so.so_rcv.lock().sbappendstream(m);
                }
            }
            sorwakeup(so);
            if tp.st.has_flags(TF_ACKNOW | TF_NEEDOUTPUT) {
                let _ = tcp_output(&mut tp);
            }
            return T(Done, tp);
        }
    }

    // Compute mbuf offset to TCP data segment.
    s.hdroptlen = s.iphlen + s.off;

    // Calculate amount of space in receive window, and then do TCP input processing. Receive
    // window is amount of space in rcv queue, but not less than advertised window.
    {
        // `int win = sbspace(..)`: the `long` truncated, then `imax` of `int`s.
        let win = (so.so_rcv.sbspace() as i32).max(0);
        let st = &mut *tp.st;
        st.rcv_wnd = win.max(st.rcv_adv.wrapping_sub(st.rcv_nxt) as i32) as u64;
    }

    let mut step6 = false;
    match tp.st.t_state {
        // If the state is SYN_RECEIVED: if seg contains SYN/ACK, send an RST. if seg contains
        // an ACK, but not for our SYN/ACK, send an RST
        TCPS_SYN_RECEIVED => {
            if s.tiflags & TH_ACK != 0 {
                if s.tiflags & TH_SYN != 0 {
                    tcpstat_inc(TcpstatCounters::TcpsBadsyn);
                    return T(DropWithReset, tp);
                }
                if seq_leq(s.th.th_ack, tp.st.snd_una) || seq_gt(s.th.th_ack, tp.st.snd_max) {
                    return T(DropWithReset, tp);
                }
            }
        }

        // If the state is SYN_SENT: if seg contains an ACK, but not for our SYN, drop the
        // input. if seg contains a RST, then drop the connection. if seg does not contain SYN,
        // then drop it. Otherwise this is an acceptable SYN segment: initialize tp->rcv_nxt
        // and tp->irs; if seg contains ack then advance tp->snd_una; if SYN has been acked
        // change to ESTABLISHED else SYN_RCVD state; arrange for segment to be acked
        // (eventually); continue processing rest of data/controls, beginning with URG
        TCPS_SYN_SENT => {
            if s.tiflags & TH_ACK != 0
                && (seq_leq(s.th.th_ack, tp.st.iss) || seq_gt(s.th.th_ack, tp.st.snd_max))
            {
                return T(DropWithReset, tp);
            }
            if s.tiflags & TH_RST != 0 {
                // TCP_ECN: if ECN is enabled, fall back to non-ecn at rexmit
                if s.do_ecn && !tp.st.has_flags(TF_DISABLE_ECN) {
                    return T(Drop, tp);
                }
                if s.tiflags & TH_ACK != 0 {
                    let c = tcp_drop(tp, Some(Errno::ECONNREFUSED));
                    return Cl(NoTpExit::Drop, c);
                }
                return T(Drop, tp);
            }
            if s.tiflags & TH_SYN == 0 {
                return T(Drop, tp);
            }
            if s.tiflags & TH_ACK != 0 {
                tp.st.snd_una = s.th.th_ack;
                if seq_lt(tp.st.snd_nxt, tp.st.snd_una) {
                    tp.st.snd_nxt = tp.st.snd_una;
                }
            }
            tcp_timer_disarm(&mut tp, TCPT_REXMT);
            tp.st.irs = s.th.th_seq;
            let _ = tcp_mss(&mut tp, i32::from(s.opti.maxseg));
            let st = &mut *tp.st;
            // Reset initial window to 1 segment for retransmit
            if st.t_rxtshift > 0 {
                st.snd_cwnd = u64::from(st.t_maxseg);
            }
            tcp_rcvseqinit(st);
            st.set_flags(TF_ACKNOW);
            // If we've sent a SACK_PERMITTED option, and the peer also replied with one, then
            // TF_SACK_PERMIT should have been set in tcp_dooptions(). If it was not, disable
            // SACKs.
            if st.sack_enable != 0 {
                st.sack_enable = (st.t_flags & TF_SACK_PERMIT) as i32;
            }
            // TCP_ECN: if ECE is set but CWR is not set for SYN-ACK, or both ECE and CWR are
            // set for simultaneous open, peer is ECN capable.
            if s.do_ecn {
                let f = s.tiflags & (TH_ACK | TH_ECE | TH_CWR);
                if f == TH_ACK | TH_ECE || f == TH_ECE | TH_CWR {
                    st.set_flags(TF_ECN_PERMIT);
                    s.tiflags &= !(TH_ECE | TH_CWR);
                    tcpstat_inc(TcpstatCounters::TcpsEcnAccepts);
                }
            }

            if s.tiflags & TH_ACK != 0 && seq_gt(st.snd_una, st.iss) {
                tcpstat_inc(TcpstatCounters::TcpsConnects);
                soisconnected(so);
                tp.st.t_state = TCPS_ESTABLISHED;
                tcp_timer_arm(
                    &mut tp,
                    TCPT_KEEP,
                    TCP_KEEPIDLE.load(Ordering::Relaxed) as u64,
                );
                let st = &mut *tp.st;
                // Do window scaling on this connection?
                if st.t_flags & (TF_RCVD_SCALE | TF_REQ_SCALE) == (TF_RCVD_SCALE | TF_REQ_SCALE) {
                    st.snd_scale = st.requested_s_scale;
                    st.rcv_scale = st.request_r_scale;
                }
                let _ = tcp_flush_queue(&mut tp);

                // if we didn't have to retransmit the SYN, use its rtt as our initial srtt &
                // rtt var.
                let st = &mut *tp.st;
                if st.t_rtttime != 0 {
                    tcp_xmit_timer(st, now.wrapping_sub(st.t_rtttime) as i32);
                }
                // Since new data was acked (the SYN), open the congestion window by one MSS.
                // We do this here, because we won't go through the normal ACK processing
                // below. And since this is the start of the connection, we know we are in the
                // exponential phase of slow-start.
                st.snd_cwnd += u64::from(st.t_maxseg);
            } else {
                st.t_state = TCPS_SYN_RECEIVED;
            }

            // trimthenstep6 (an `#if 0` label in the C): Advance th->th_seq to correspond to
            // first data byte. If data, trim to stay within window, dropping FIN if
            // necessary.
            let st = &mut *tp.st;
            s.th.th_seq = s.th.th_seq.wrapping_add(1);
            // `tlen > tp->rcv_wnd`: the `int` compared as the `u_long`.
            if s.tlen as u64 > st.rcv_wnd {
                let todrop = s.tlen.wrapping_sub(st.rcv_wnd as i32);
                m_adj(s.seg(), -todrop);
                s.tlen = st.rcv_wnd as i32;
                s.tiflags &= !TH_FIN;
                tcpstat_pkt(
                    TcpstatCounters::TcpsRcvpackafterwin,
                    TcpstatCounters::TcpsRcvbyteafterwin,
                    todrop as u64,
                );
            }
            st.snd_wl1 = s.th.th_seq.wrapping_sub(1);
            st.rcv_up = s.th.th_seq;
            step6 = true;
        }

        // If a new connection request is received while in TIME_WAIT, drop the old connection
        // and start over if the if the timestamp or the sequence numbers are above the
        // previous ones.
        TCPS_TIME_WAIT
            if s.tiflags & (TH_SYN | TH_ACK) == TH_SYN
                && ((s.opti.ts_present && tstmp_lt(tp.st.ts_recent, s.opti.ts_val))
                    || seq_gt(s.th.th_seq, tp.st.rcv_nxt)) =>
        {
            // NPF > 0: The socket will be recreated but the new state has already been linked
            // to the socket. Remove the link between old socket and new state.
            if let Some(pf) = pf() {
                pf.inp_unlink(&inp.inp_pf_sk);
            }
            // Advance the iss by at least 32768, but clear the msb in order to make sure that
            // SEG_LT(snd_nxt, iss).
            let iss = tp
                .st
                .snd_nxt
                .wrapping_add((hal::random::arc4random() & 0x7fff_ffff) | 0x8000);
            let c = tcp_close(tp);
            // in_pcbsounlock(inp, so); in_pcbunref(inp); goto findpcb: the caller's.
            return Reuse(iss, c);
        }

        _ => {}
    }

    if !step6 {
        // States other than LISTEN or SYN_SENT. First check timestamp, if present. Then check
        // that at least some bytes of segment are within receive window. If segment begins
        // before rcv_nxt, drop leading data (and SYN); if nothing left, just ack.
        //
        // RFC 1323 PAWS: If we have a timestamp reply on this segment and it's less than
        // opti.ts_recent, drop it.
        let st = &mut *tp.st;
        if s.opti.ts_present
            && s.tiflags & TH_RST == 0
            && st.ts_recent != 0
            && tstmp_lt(s.opti.ts_val, st.ts_recent)
        {
            // Check to see if ts_recent is over 24 days old.
            if now.wrapping_sub(st.ts_recent_age) > TCP_PAWS_IDLE as u64 {
                // Invalidate ts_recent. If this segment updates ts_recent, the age will be
                // reset later and ts_recent will get a valid value. If it does not, setting
                // ts_recent to zero will at least satisfy the requirement that zero be placed
                // in the timestamp echo reply when ts_recent isn't valid. The age isn't reset
                // until we get a valid ts_recent because we don't want out-of-order segments
                // to be dropped when ts_recent is old.
                st.ts_recent = 0;
            } else {
                tcpstat_pkt(
                    TcpstatCounters::TcpsRcvduppack,
                    TcpstatCounters::TcpsRcvdupbyte,
                    s.tlen as u64,
                );
                tcpstat_inc(TcpstatCounters::TcpsPawsdrop);
                if s.tlen != 0 {
                    return T(DropAfterAck, tp);
                }
                return T(Drop, tp);
            }
        }

        // `int todrop = tp->rcv_nxt - th->th_seq`.
        let mut todrop = st.rcv_nxt.wrapping_sub(s.th.th_seq) as i32;
        if todrop > 0 {
            if s.tiflags & TH_SYN != 0 {
                s.tiflags &= !TH_SYN;
                s.th.th_seq = s.th.th_seq.wrapping_add(1);
                if s.th.th_urp > 1 {
                    s.th.th_urp -= 1;
                } else {
                    s.tiflags &= !TH_URG;
                }
                todrop -= 1;
            }
            if todrop > s.tlen || (todrop == s.tlen && s.tiflags & TH_FIN == 0) {
                // Any valid FIN must be to the left of the window. At this point, FIN must be
                // a duplicate or out-of-sequence, so drop it.
                s.tiflags &= !TH_FIN;
                // Send ACK to resynchronize, and drop any data, but keep on processing for RST
                // or ACK.
                st.set_flags(TF_ACKNOW);
                todrop = s.tlen;
                tcpstat_pkt(
                    TcpstatCounters::TcpsRcvduppack,
                    TcpstatCounters::TcpsRcvdupbyte,
                    todrop as u64,
                );
            } else {
                tcpstat_pkt(
                    TcpstatCounters::TcpsRcvpartduppack,
                    TcpstatCounters::TcpsRcvpartdupbyte,
                    todrop as u64,
                );
            }
            // drop from head afterwards
            s.hdroptlen += todrop;
            s.th.th_seq = s.th.th_seq.wrapping_add(todrop as u32);
            s.tlen -= todrop;
            if i32::from(s.th.th_urp) > todrop {
                s.th.th_urp -= todrop as u16;
            } else {
                s.tiflags &= !TH_URG;
                s.th.th_urp = 0;
            }
        }

        // If new data are received on a connection after the user processes are gone, then
        // RST the other end.
        if so.has_state(SS_NOFDREF) && tp.st.t_state > TCPS_CLOSE_WAIT && s.tlen != 0 {
            let c = tcp_close(tp);
            tcpstat_inc(TcpstatCounters::TcpsRcvafterclose);
            return Cl(NoTpExit::DropWithReset, c);
        }

        // If segment ends after window, drop trailing data (and PUSH and FIN); if nothing
        // left, just ACK. `(th_seq + tlen) - (rcv_nxt + rcv_wnd)`, the `u_long` difference
        // truncated to the `int`.
        let st = &mut *tp.st;
        let todrop =
            s.th.th_seq
                .wrapping_add(s.tlen as u32)
                .wrapping_sub(st.rcv_nxt.wrapping_add(st.rcv_wnd as u32)) as i32;
        if todrop > 0 {
            tcpstat_inc(TcpstatCounters::TcpsRcvpackafterwin);
            if todrop >= s.tlen {
                tcpstat_add(TcpstatCounters::TcpsRcvbyteafterwin, s.tlen as u64);
                // If window is closed can only take segments at window edge, and have to drop
                // data and PUSH from incoming segments. Continue processing, but remember to
                // ack. Otherwise, drop segment and ack.
                if st.rcv_wnd == 0 && s.th.th_seq == st.rcv_nxt {
                    st.set_flags(TF_ACKNOW);
                    tcpstat_inc(TcpstatCounters::TcpsRcvwinprobe);
                } else {
                    return T(DropAfterAck, tp);
                }
            } else {
                tcpstat_add(TcpstatCounters::TcpsRcvbyteafterwin, todrop as u64);
            }
            m_adj(s.seg(), -todrop);
            s.tlen -= todrop;
            s.tiflags &= !(TH_PUSH | TH_FIN);
        }

        // If last ACK falls within this segment's sequence numbers, record its timestamp if
        // it's more recent. NOTE that the test is modified according to the latest proposal
        // of the tcplw@cray.com list (Braden 1993/04/26).
        if s.opti.ts_present
            && tstmp_geq(s.opti.ts_val, st.ts_recent)
            && seq_leq(s.th.th_seq, st.last_ack_sent)
        {
            st.ts_recent_age = now;
            st.ts_recent = s.opti.ts_val;
        }

        // If the RST bit is set examine the state:
        //    SYN_RECEIVED STATE:
        //     If passive open, return to LISTEN state.
        //     If active open, inform user that connection was refused.
        //    ESTABLISHED, FIN_WAIT_1, FIN_WAIT2, CLOSE_WAIT STATES:
        //     Inform user that connection was reset, and close tcb.
        //    CLOSING, LAST_ACK, TIME_WAIT STATES
        //     Close the tcb.
        if s.tiflags & TH_RST != 0 {
            // The RST must be at the last ACK sent, at rcv_nxt or one past it.
            if s.th.th_seq != st.last_ack_sent
                && s.th.th_seq != st.rcv_nxt
                && s.th.th_seq != st.rcv_nxt.wrapping_add(1)
            {
                return T(Drop, tp);
            }

            let error = match st.t_state {
                TCPS_SYN_RECEIVED => {
                    // TCP_ECN: if ECN is enabled, fall back to non-ecn at rexmit
                    if s.do_ecn && !st.has_flags(TF_DISABLE_ECN) {
                        return T(Drop, tp);
                    }
                    Some(Errno::ECONNREFUSED)
                }

                TCPS_ESTABLISHED | TCPS_FIN_WAIT_1 | TCPS_FIN_WAIT_2 | TCPS_CLOSE_WAIT => {
                    Some(Errno::ECONNRESET)
                }
                TCPS_CLOSING | TCPS_LAST_ACK | TCPS_TIME_WAIT => {
                    let c = tcp_close(tp);
                    return Cl(NoTpExit::Drop, c);
                }
                _ => None,
            };
            if let Some(e) = error {
                so.set_error(Some(e));
                // close:
                tp.st.t_state = TCPS_CLOSED;
                tcpstat_inc(TcpstatCounters::TcpsDrops);
                let c = tcp_close(tp);
                return Cl(NoTpExit::Drop, c);
            }
        }

        // If a SYN is in the window, then this is an error and we ACK and drop the packet.
        if s.tiflags & TH_SYN != 0 {
            return T(DropAfterAckRatelim, tp);
        }

        // If the ACK bit is off we drop the segment and return.
        if s.tiflags & TH_ACK == 0 {
            if tp.st.has_flags(TF_ACKNOW) {
                return T(DropAfterAck, tp);
            }
            return T(Drop, tp);
        }

        // Ack processing.
        'ack: {
            let state = tp.st.t_state;
            if !matches!(
                state,
                TCPS_SYN_RECEIVED
                    | TCPS_ESTABLISHED
                    | TCPS_FIN_WAIT_1
                    | TCPS_FIN_WAIT_2
                    | TCPS_CLOSE_WAIT
                    | TCPS_CLOSING
                    | TCPS_LAST_ACK
                    | TCPS_TIME_WAIT
            ) {
                break 'ack;
            }

            // In SYN_RECEIVED state, the ack ACKs our SYN, so enter ESTABLISHED state and
            // continue processing. The ACK was checked above.
            if state == TCPS_SYN_RECEIVED {
                tcpstat_inc(TcpstatCounters::TcpsConnects);
                soisconnected(so);
                tp.st.t_state = TCPS_ESTABLISHED;
                tcp_timer_arm(
                    &mut tp,
                    TCPT_KEEP,
                    TCP_KEEPIDLE.load(Ordering::Relaxed) as u64,
                );
                let st = &mut *tp.st;
                // Do window scaling?
                if st.t_flags & (TF_RCVD_SCALE | TF_REQ_SCALE) == (TF_RCVD_SCALE | TF_REQ_SCALE) {
                    st.snd_scale = st.requested_s_scale;
                    st.rcv_scale = st.request_r_scale;
                    s.tiwin = u64::from(s.th.th_win) << st.snd_scale;
                }
                let _ = tcp_flush_queue(&mut tp);
                tp.st.snd_wl1 = s.th.th_seq.wrapping_sub(1);
                // fall into ...
            }

            // In ESTABLISHED state: drop duplicate ACKs; ACK out of range ACKs. If the ack is
            // in the range tp->snd_una < th->th_ack <= tp->snd_max then advance tp->snd_una to
            // th->th_ack and drop data from the retransmission queue. If this ACK reflects
            // more up to date window information we update our window information.

            // TCP_ECN: if we receive ECE and are not already in recovery phase, reduce cwnd by
            // half but don't slow-start. advance snd_last to snd_max not to reduce cwnd again
            // until all outstanding packets are acked.
            let st = &mut *tp.st;
            if s.do_ecn && s.tiflags & TH_ECE != 0 {
                if st.has_flags(TF_ECN_PERMIT) && seq_geq(st.snd_una, st.snd_last) {
                    // libkern's `u_int` min of the two `u_long`s, then the `u_int` division.
                    let win = (st.snd_wnd as u32).min(st.snd_cwnd as u32) / u32::from(st.t_maxseg);
                    if win > 1 {
                        st.snd_ssthresh = u64::from((win / 2).wrapping_mul(u32::from(st.t_maxseg)));
                        st.snd_cwnd = st.snd_ssthresh;
                        st.snd_last = st.snd_max;
                        st.set_flags(TF_SEND_CWR);
                        tcpstat_inc(TcpstatCounters::TcpsCwrEcn);
                    }
                }
                tcpstat_inc(TcpstatCounters::TcpsEcnRcvece);
            }
            // if we receive CWR, we know that the peer has reduced its congestion window.
            // stop sending ecn-echo.
            if s.tiflags & TH_CWR != 0 {
                st.clear_flags(TF_RCVD_CE);
                tcpstat_inc(TcpstatCounters::TcpsEcnRcvcwr);
            }

            if seq_leq(s.th.th_ack, st.snd_una) {
                // Duplicate/old ACK processing.
                // Increments t_dupacks:
                //     Pure duplicate (same seq/ack/window, no data)
                // Doesn't affect t_dupacks:
                //     Data packets.
                //     Normal window updates (window opens)
                // Resets t_dupacks:
                //     New data ACKed.
                //     Window shrinks
                //     Old ACK
                if s.tlen != 0 {
                    // Drop very old ACKs unless th_seq matches
                    if s.th.th_seq != st.rcv_nxt
                        && seq_lt(s.th.th_ack, st.snd_una.wrapping_sub(st.max_sndwnd as u32))
                    {
                        tcpstat_inc(TcpstatCounters::TcpsRcvacktooold);
                        return T(Drop, tp);
                    }
                    break 'ack;
                }
                // If we get an old ACK, there is probably packet reordering going on. Be
                // conservative and reset t_dupacks so that we are less aggressive in doing a
                // fast retransmit.
                if s.th.th_ack != st.snd_una {
                    st.t_dupacks = 0;
                    break 'ack;
                }
                if s.tiwin == st.snd_wnd {
                    tcpstat_inc(TcpstatCounters::TcpsRcvdupack);
                    // If we have outstanding data (other than a window probe), this is a
                    // completely duplicate ack (ie, window info didn't change), the ack is the
                    // biggest we've seen and we've seen exactly our rexmt threshold of them,
                    // assume a packet has been dropped and retransmit it. Kludge snd_nxt & the
                    // congestion window so we send only this one packet.
                    //
                    // We know we're losing at the current window size so do congestion
                    // avoidance (set ssthresh to half the current window and pull our
                    // congestion window back to the new ssthresh).
                    //
                    // Dup acks mean that packets have left the network (they're now cached at
                    // the receiver) so bump cwnd by the amount in the receiver to keep a
                    // constant cwnd packets in the network.
                    if !tcp_timer_isarmed(st, TCPT_REXMT) {
                        st.t_dupacks = 0;
                    } else {
                        st.t_dupacks = st.t_dupacks.wrapping_add(1);
                        if i32::from(st.t_dupacks) == TCPREXMTTHRESH {
                            let onxt = st.snd_nxt;
                            let maxseg = u64::from(st.t_maxseg);
                            let mut win = st.snd_wnd.min(st.snd_cwnd) / 2 / maxseg;

                            if seq_lt(s.th.th_ack, st.snd_last) {
                                // False fast retx after timeout. Do not cut window.
                                st.t_dupacks = 0;
                                return T(Drop, tp);
                            }
                            if win < 2 {
                                win = 2;
                            }
                            st.snd_ssthresh = win.wrapping_mul(maxseg);
                            st.snd_last = st.snd_max;
                            if st.sack_enable != 0 {
                                tcp_timer_disarm(&mut tp, TCPT_REXMT);
                                let st = &mut *tp.st;
                                st.t_rtttime = 0;
                                // TCP_ECN
                                st.set_flags(TF_SEND_CWR);
                                tcpstat_inc(TcpstatCounters::TcpsCwrFrecovery);
                                tcpstat_inc(TcpstatCounters::TcpsSackRecoveryEpisode);
                                // tcp_output() will send oldest SACK-eligible rtx.
                                let _ = tcp_output(&mut tp);
                                let st = &mut *tp.st;
                                // `t_maxseg * t_dupacks`, an `int`.
                                st.snd_cwnd = st.snd_ssthresh.wrapping_add(
                                    (i32::from(st.t_maxseg) * i32::from(st.t_dupacks)) as u64,
                                );
                                return T(Drop, tp);
                            }
                            tcp_timer_disarm(&mut tp, TCPT_REXMT);
                            let st = &mut *tp.st;
                            st.t_rtttime = 0;
                            st.snd_nxt = s.th.th_ack;
                            st.snd_cwnd = maxseg;
                            // TCP_ECN
                            st.set_flags(TF_SEND_CWR);
                            tcpstat_inc(TcpstatCounters::TcpsCwrFrecovery);
                            tcpstat_inc(TcpstatCounters::TcpsSndrexmitfast);
                            let _ = tcp_output(&mut tp);

                            let st = &mut *tp.st;
                            st.snd_cwnd = st.snd_ssthresh.wrapping_add(
                                (i32::from(st.t_maxseg) * i32::from(st.t_dupacks)) as u64,
                            );
                            if seq_gt(onxt, st.snd_nxt) {
                                st.snd_nxt = onxt;
                            }
                            return T(Drop, tp);
                        } else if i32::from(st.t_dupacks) > TCPREXMTTHRESH {
                            st.snd_cwnd += u64::from(st.t_maxseg);
                            let _ = tcp_output(&mut tp);
                            return T(Drop, tp);
                        }
                    }
                } else if s.tiwin < st.snd_wnd {
                    // The window was retracted! Previous dup ACKs may have been due to
                    // packets arriving after the shrunken window, not a missing packet, so
                    // play it safe and reset t_dupacks
                    st.t_dupacks = 0;
                }
                break 'ack;
            }
            // If the congestion window was inflated to account for the other side's cached
            // packets, retract it.
            if i32::from(st.t_dupacks) >= TCPREXMTTHRESH {
                // Check for a partial ACK
                if seq_lt(s.th.th_ack, st.snd_last) {
                    if st.sack_enable != 0 {
                        tcp_sack_partialack(&mut tp, &s.th);
                    } else {
                        tcp_newreno_partialack(&mut tp, &s.th);
                    }
                } else {
                    // Out of fast recovery
                    st.snd_cwnd = st.snd_ssthresh;
                    let outstanding =
                        tcp_seq_subtract(u64::from(st.snd_max), u64::from(s.th.th_ack));
                    if outstanding < st.snd_ssthresh {
                        st.snd_cwnd = outstanding;
                    }
                    st.t_dupacks = 0;
                }
            } else {
                // Reset the duplicate ACK counter if we were not in fast recovery.
                st.t_dupacks = 0;
            }
            let st = &mut *tp.st;
            if seq_gt(s.th.th_ack, st.snd_max) {
                tcpstat_inc(TcpstatCounters::TcpsRcvacktoomuch);
                return T(DropAfterAckRatelim, tp);
            }
            let acked = s.th.th_ack.wrapping_sub(st.snd_una) as i32;
            tcpstat_pkt(
                TcpstatCounters::TcpsRcvackpack,
                TcpstatCounters::TcpsRcvackbyte,
                acked as u64,
            );
            st.t_rcvacktime = now;

            // If we have a timestamp reply, update smoothed round trip time. If no timestamp
            // is present but transmit timer is running and timed sequence number was acked,
            // update smoothed round trip time. Since we now have an rtt measurement, cancel
            // the timer backoff (cf., Phil Karn's retransmit alg.). Recompute the initial
            // retransmit timer.
            if s.opti.ts_present && s.opti.ts_ecr != 0 {
                tcp_xmit_timer(st, now.wrapping_sub(u64::from(s.opti.ts_ecr)) as i32);
            } else if st.t_rtttime != 0 && seq_gt(s.th.th_ack, st.t_rtseq) {
                tcp_xmit_timer(st, now.wrapping_sub(st.t_rtttime) as i32);
            }

            // If all outstanding data is acked, stop retransmit timer and remember to restart
            // (more output or persist). If there is more data to be acked, restart retransmit
            // timer, using current (possibly backed-off) value.
            if s.th.th_ack == tp.st.snd_max {
                tcp_timer_disarm(&mut tp, TCPT_REXMT);
                tp.st.set_flags(TF_NEEDOUTPUT);
            } else if !tcp_timer_isarmed(tp.st, TCPT_PERSIST) {
                let rxtcur = tp.st.t_rxtcur;
                tcp_timer_arm(&mut tp, TCPT_REXMT, rxtcur as u64);
            }
            // When new data is acked, open the congestion window. If the window gives us less
            // than ssthresh packets in flight, open exponentially (maxseg per packet).
            // Otherwise open linearly: maxseg per window (maxseg^2 / cwnd per packet).
            let st = &mut *tp.st;
            {
                // `u_int cw = tp->snd_cwnd`, `u_int incr = tp->t_maxseg`.
                let cw = st.snd_cwnd as u32;
                let mut incr = u32::from(st.t_maxseg);

                if u64::from(cw) > st.snd_ssthresh {
                    incr = (incr.wrapping_mul(incr) / cw).max(1);
                }
                if i32::from(st.t_dupacks) < TCPREXMTTHRESH {
                    st.snd_cwnd =
                        u64::from(cw.wrapping_add(incr)).min(u64::from(TCP_MAXWIN << st.snd_scale));
                }
            }
            // `acked > so->so_snd.sb_cc`: the `int` compared as the `u_long`.
            let sb_cc = so.so_snd.cc();
            let ourfinisacked = if acked as u64 > sb_cc {
                if st.snd_wnd > sb_cc {
                    st.snd_wnd -= sb_cc;
                } else {
                    st.snd_wnd = 0;
                }
                let mut g = so.so_snd.lock();
                let cc = so.so_snd.cc();
                g.sbdrop(cc as i32);
                drop(g);
                true
            } else {
                so.so_snd.lock().sbdrop(acked);
                if st.snd_wnd > acked as u64 {
                    st.snd_wnd -= acked as u64;
                } else {
                    st.snd_wnd = 0;
                }
                false
            };

            tcp_update_sndspace(&tp);
            if so.so_snd.sb_notify() {
                sowwakeup(so);
            }

            let st = &mut *tp.st;
            // If we had a pending ICMP message that referred to data that have just been
            // acknowledged, disregard the recorded ICMP message.
            if st.has_flags(TF_PMTUD_PEND) && seq_gt(s.th.th_ack, st.t_pmtud_th_seq) {
                st.clear_flags(TF_PMTUD_PEND);
            }

            // Keep track of the largest chunk of data acknowledged since last PMTU update
            if st.t_pmtud_mss_acked < acked as u32 {
                st.t_pmtud_mss_acked = acked as u32;
            }

            st.snd_una = s.th.th_ack;
            // TCP_ECN: sync snd_last with snd_una
            if seq_gt(st.snd_una, st.snd_last) {
                st.snd_last = st.snd_una;
            }
            if seq_lt(st.snd_nxt, st.snd_una) {
                st.snd_nxt = st.snd_una;
            }

            match st.t_state {
                // In FIN_WAIT_1 STATE in addition to the processing for the ESTABLISHED state
                // if our FIN is now acknowledged then enter FIN_WAIT_2.
                TCPS_FIN_WAIT_1 => {
                    if ourfinisacked {
                        // If we can't receive any more data, then closing user can proceed.
                        // Starting the timer is contrary to the specification, but if we don't
                        // get a FIN we'll hang forever.
                        if so.so_rcv.has_state(SS_CANTRCVMORE) {
                            soisdisconnected(so);
                            let maxidle =
                                TCPTV_KEEPCNT.wrapping_mul(TCP_KEEPIDLE.load(Ordering::Relaxed));
                            tcp_timer_arm(&mut tp, TCPT_2MSL, maxidle as u64);
                        }
                        tp.st.t_state = TCPS_FIN_WAIT_2;
                    }
                }

                // In CLOSING STATE in addition to the processing for the ESTABLISHED state if
                // the ACK acknowledges our FIN then enter the TIME-WAIT state, otherwise
                // ignore the segment.
                TCPS_CLOSING => {
                    if ourfinisacked {
                        tp.st.t_state = TCPS_TIME_WAIT;
                        tcp_canceltimers(&mut tp);
                        tcp_timer_arm(&mut tp, TCPT_2MSL, (2 * TCPTV_MSL) as u64);
                        soisdisconnected(so);
                    }
                }

                // In LAST_ACK, we may still be waiting for data to drain and/or to be acked,
                // as well as for the ack of our FIN. If our FIN is now acknowledged, delete
                // the TCB, enter the closed state and return.
                TCPS_LAST_ACK => {
                    if ourfinisacked {
                        let c = tcp_close(tp);
                        return Cl(NoTpExit::Drop, c);
                    }
                }

                // In TIME_WAIT state the only thing that should arrive is a retransmission of
                // the remote FIN. Acknowledge it and restart the finack timer.
                TCPS_TIME_WAIT => {
                    tcp_timer_arm(&mut tp, TCPT_2MSL, (2 * TCPTV_MSL) as u64);
                    return T(DropAfterAck, tp);
                }
                _ => {}
            }
        }
    }

    // step6:
    // Update window information. Don't look at window if no ACK: TAC's send garbage on first
    // SYN.
    let st = &mut *tp.st;
    if s.tiflags & TH_ACK != 0
        && (seq_lt(st.snd_wl1, s.th.th_seq)
            || (st.snd_wl1 == s.th.th_seq
                && (seq_lt(st.snd_wl2, s.th.th_ack)
                    || (st.snd_wl2 == s.th.th_ack && s.tiwin > st.snd_wnd))))
    {
        // keep track of pure window updates
        if s.tlen == 0 && st.snd_wl2 == s.th.th_ack && s.tiwin > st.snd_wnd {
            tcpstat_inc(TcpstatCounters::TcpsRcvwinupd);
        }
        st.snd_wnd = s.tiwin;
        st.snd_wl1 = s.th.th_seq;
        st.snd_wl2 = s.th.th_ack;
        if st.snd_wnd > st.max_sndwnd {
            st.max_sndwnd = st.snd_wnd;
        }
        st.set_flags(TF_NEEDOUTPUT);
    }

    // Process segments with URG.
    'dodata: {
        if s.tiflags & TH_URG != 0 && s.th.th_urp != 0 && !tcps_havercvdfin(st.t_state) {
            // This is a kludge, but if we receive and accept random urgent pointers, we'll
            // crash in soreceive. It's hard to imagine someone actually wanting to send this
            // much urgent data.
            let g = so.so_rcv.lock();
            let urgent = u64::from(s.th.th_urp).wrapping_add(so.so_rcv.cc());
            drop(g);

            if urgent > SB_MAX_VAR.load(Ordering::Relaxed) {
                s.th.th_urp = 0; // XXX
                s.tiflags &= !TH_URG; // XXX
                break 'dodata; // XXX
            }
            // If this segment advances the known urgent pointer, then mark the data stream.
            // This should not happen in CLOSE_WAIT, CLOSING, LAST_ACK or TIME_WAIT STATES
            // since a FIN has been received from the remote side. In these states we ignore
            // the URG.
            //
            // According to RFC961 (Assigned Protocols), the urgent pointer points to the last
            // octet of urgent data. We continue, however, to consider it to indicate the first
            // octet of data past the urgent section as the original spec states (in one of
            // two places).
            let urp_seq = s.th.th_seq.wrapping_add(u32::from(s.th.th_urp));
            if seq_gt(urp_seq, st.rcv_up) {
                st.rcv_up = urp_seq;
                let mut g = so.so_rcv.lock();
                // `sb_cc + (rcv_up - rcv_nxt) - 1`, the `u_long` sum.
                let mark = so
                    .so_rcv
                    .cc()
                    .wrapping_add(u64::from(st.rcv_up.wrapping_sub(st.rcv_nxt)))
                    .wrapping_sub(1);
                so.set_oobmark(mark);
                if mark == 0 {
                    g.set_state(SS_RCVATMARK);
                }
                drop(g);
                sohasoutofband(so);
                st.t_oobflags &= !(TCPOOB_HAVEDATA | TCPOOB_HADDATA);
            }
            // Remove out of band data so doesn't get presented to user. This can happen
            // independent of advancing the URG pointer, but if two URG's are pending at once,
            // some out-of-band data may creep in... ick.
            if s.th.th_urp <= s.tlen as u16
                && !so.has_options(SO_OOBINLINE)
                && let Some(m) = s.m.as_deref_mut()
            {
                tcp_pulloutofband(st, u32::from(s.th.th_urp), m, s.hdroptlen);
            }
        } else if seq_gt(st.rcv_nxt, st.rcv_up) {
            // If no out of band data is expected, pull receive urgent pointer along with the
            // receive window.
            st.rcv_up = st.rcv_nxt;
        }
    }
    // dodata: XXX

    // Process the segment text, merging it into the TCP sequencing queue, and arranging for
    // acknowledgment of receipt if necessary. This process logically involves adjusting
    // tp->rcv_wnd as data is presented to the user (this happens in tcp_usrreq.c, case
    // PRU_RCVD). If a FIN has already been received on this connection then we just ignore
    // the text.
    if (s.tlen != 0 || s.tiflags & TH_FIN != 0) && !tcps_havercvdfin(tp.st.t_state) {
        let laststart = s.th.th_seq;
        let lastend = s.th.th_seq.wrapping_add(s.tlen as u32);
        let mut reass_failed = false;

        if s.th.th_seq == tp.st.rcv_nxt
            && !tp.st.t_segq.is_some()
            && tp.st.t_state == TCPS_ESTABLISHED
        {
            tcp_setup_ack(&mut tp, s.tiflags, s.m.as_deref());
            let st = &mut *tp.st;
            st.rcv_nxt = st.rcv_nxt.wrapping_add(s.tlen as u32);
            // The header's own FIN (`th->th_flags`, not the trimmed `tiflags`), as the C's.
            s.tiflags = s.th.th_flags & TH_FIN;
            tcpstat_pkt(
                TcpstatCounters::TcpsRcvpack,
                TcpstatCounters::TcpsRcvbyte,
                s.tlen as u64,
            );
            let m = s.m.take();
            if so.so_rcv.has_state(SS_CANTRCVMORE) {
                // m_freem(m)
                drop(m);
            } else if let Some(m) = m {
                m_adj(&m, s.hdroptlen);
                so.so_rcv.lock().sbappendstream(m);
            }
            sorwakeup(so);
        } else {
            if let Some(m) = s.m.take() {
                m_adj(&m, s.hdroptlen);
                match tcp_reass(&mut tp, &mut s.th, m, &mut s.tlen) {
                    Ok(flags) => s.tiflags = flags,
                    Err(_) => {
                        s.tiflags = 0;
                        reass_failed = true;
                    }
                }
            }
            tp.st.set_flags(TF_ACKNOW);
        }
        if tp.st.sack_enable != 0 && !reass_failed {
            tcp_update_sack_list(tp.st, laststart, lastend);
        }

        // variable len never referenced again in modern BSD, so why bother computing it ??
        // (the C keeps `len = so->so_rcv.sb_hiwat - (tp->rcv_adv - tp->rcv_nxt)`, the amount
        // of data the peer has sent into our window, under `#if 0`.)
    } else {
        // m_freem(m)
        drop(s.m.take());
        s.tiflags &= !TH_FIN;
    }

    // If FIN is received ACK the FIN and let the user know that the connection is closing.
    // Ignore a FIN received before the connection is fully established.
    if s.tiflags & TH_FIN != 0 && tcps_haveestablished(tp.st.t_state) {
        if !tcps_havercvdfin(tp.st.t_state) {
            socantrcvmore(so);
            tp.st.set_flags(TF_ACKNOW);
            tp.st.rcv_nxt = tp.st.rcv_nxt.wrapping_add(1);
        }
        match tp.st.t_state {
            // In ESTABLISHED STATE enter the CLOSE_WAIT state.
            TCPS_ESTABLISHED => tp.st.t_state = TCPS_CLOSE_WAIT,

            // If still in FIN_WAIT_1 STATE FIN has not been acked so enter the CLOSING state.
            TCPS_FIN_WAIT_1 => tp.st.t_state = TCPS_CLOSING,

            // In FIN_WAIT_2 state enter the TIME_WAIT state, starting the time-wait timer,
            // turning off the other standard timers.
            TCPS_FIN_WAIT_2 => {
                tp.st.t_state = TCPS_TIME_WAIT;
                tcp_canceltimers(&mut tp);
                tcp_timer_arm(&mut tp, TCPT_2MSL, (2 * TCPTV_MSL) as u64);
                soisdisconnected(so);
            }

            // In TIME_WAIT state restart the 2 MSL time_wait timer.
            TCPS_TIME_WAIT => tcp_timer_arm(&mut tp, TCPT_2MSL, (2 * TCPTV_MSL) as u64),
            _ => {}
        }
    }
    if s.otp != 0 {
        tcp_trace(
            TA_INPUT,
            s.ostate,
            Some(tp.st),
            s.otp,
            Some(&s.saveti),
            0,
            s.tlen,
        );
    }

    // Return any desired output.
    if tp.st.has_flags(TF_ACKNOW | TF_NEEDOUTPUT) {
        let _ = tcp_output(&mut tp);
    }
    T(Done, tp)
}

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

    // tcp_input (N6.6a4): segments from the peers of testtcp's connections through the
    // kernel's tcp_input, as a wire carries them; what changes is read off the control block,
    // the socket buffers and the test interface's wire.

    use hal::netlock::NETLOCK;
    use hal::sys::netinet6_in6::In6Addr;

    use crate::net::mbuf::MbufList;
    use crate::net::socket::Socket;
    use crate::netinet::in_pcb::sotoinpcb;
    use crate::netinet::tcp_fsm::{TCPS_CLOSE_WAIT, TCPS_TIME_WAIT};
    use crate::netinet::tcp_subr::testtcp::{G, estab6, last6, listener, sig_legs, tcpcbs};

    /// The header of a segment from the peer of `estab`'s connection (port 80 to 7000),
    /// network order.
    fn peer_th(flags: u8, seq: u32, ack: u32, win: u16) -> Tcphdr {
        Tcphdr {
            th_sport: 80u16.to_be(),
            th_dport: 7000u16.to_be(),
            th_seq: seq.to_be(),
            th_ack: ack.to_be(),
            th_flags: flags,
            th_win: win.to_be(),
            ..Tcphdr::default()
        }
    }

    /// A segment from the peer of `estab(_, n)`'s connection (10.9.n.2:80 to 10.9.n.1:7000),
    /// its checksum taken as verified by the interface (`M_TCP_CSUM_IN_OK`).
    fn from_peer(n: u8, th: Tcphdr, opts: &[u8], data: &[u8]) -> MbufBox {
        let m = seg4([10, 9, n, 2], [10, 9, n, 1], th, opts, data);
        csum_ok(m)
    }

    /// `m` with its TCP checksum verified by the interface.
    fn csum_ok(m: MbufBox) -> MbufBox {
        let ph = m.m_pkthdr();
        ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_IN_OK);
        m
    }

    /// `ip_input`'s delivery of `m` (a 20-byte IPv4 header, or a 40-byte IPv6 one for
    /// `AF_INET6`) to `tcp_input`, without a softnet's stack, under the shared net lock.
    fn deliver_af(m: MbufBox, af: i32) {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut off = if af == AF_INET6_I32 { 40 } else { 20 };
        let mut mp = Some(m);
        let r = tcp_input::<G>(&mut mp, &mut off, IPPROTO_TCP, af, None, &net);
        drop(w);
        assert_eq!(r, IPPROTO_DONE);
        assert!(mp.is_none(), "the segment consumed");
    }

    /// [`deliver_af`] of an IPv4 segment.
    fn deliver(m: MbufBox) {
        deliver_af(m, AF_INET_I32);
    }

    /// A softnet's batch: `ms` queued as `tcp_input` queues them, then `tcp_input_mlist`.
    fn deliver_list(ms: std::vec::Vec<MbufBox>) {
        let mut ml = MbufList::new();
        for m in ms {
            m.m_pkthdr().ph_cookie.set(20);
            ml_enqueue(&mut ml, m);
        }
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        tcp_input_mlist::<G>(&mut ml, AF_INET_I32, &net);
        drop(w);
        assert!(ml_dequeue(&mut ml).is_none(), "the list consumed");
    }

    /// The bytes in the receive buffer of `so`.
    fn received(so: &Socket<G>) -> std::vec::Vec<u8> {
        let n = so.so_rcv.cc() as usize;
        let g = so.so_rcv.lock();
        let mut v = std::vec![0u8; n];
        g.copydata_first(0, &mut v);
        v
    }

    /// What `f` reads off the control block of `inp`.
    fn peek<R>(inp: &Inpcb<G>, f: impl FnOnce(&TcpState) -> R) -> R {
        testtcp::with_tp(inp, |tp| (f(tp.st), None))
    }

    /// Whether `inp` still has its control block (none once its socket is detached).
    fn has_tcpcb(inp: &Inpcb<G>) -> bool {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let Some(mut so) = in_pcbsolock(inp, &net) else {
            return false;
        };
        let mut il = so.inet().expect("a kernel protocol's socket lock");
        intotcpcb(inp, &mut il).is_some()
    }

    #[test]
    fn input_takes_in_sequence_data_by_prediction_and_delays_the_ack() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tia0", 100);
        let (pred, before) = (stat(C::TcpsPreddat), sent(d));
        deliver(from_peer(
            100,
            peer_th(TH_ACK, 5000, 1000, 65535),
            &[],
            b"hello",
        ));
        assert_eq!(received(&so), b"hello");
        assert_eq!(stat(C::TcpsPreddat), pred + 1, "header prediction");
        let (nxt, delack) = peek(&inp, |st| (st.rcv_nxt, tcp_timer_isarmed(st, TCPT_DELACK)));
        assert_eq!(nxt, 5005);
        assert!(delack, "the ACK delayed");
        assert_eq!(sent(d), before, "nothing sent yet");
        // The second segment with an ACK delayed: ACKNOW, one ACK for both at once.
        deliver(from_peer(
            100,
            peer_th(TH_ACK, 5005, 1000, 65535),
            &[],
            b" world",
        ));
        assert_eq!(received(&so), b"hello world");
        assert_eq!(sent(d), before + 1);
        let (_ip, th, _o, data) = last(d);
        assert_eq!((th.th_flags, u32::from_be(th.th_ack)), (TH_ACK, 5011));
        assert_eq!(u32::from_be(th.th_seq), 1000);
        assert!(data.is_empty());
        // A segment whose window changed is not predicted: the slow path takes its data and
        // the new window (step6's window update).
        let pred = stat(C::TcpsPreddat);
        deliver(from_peer(
            100,
            peer_th(TH_ACK, 5011, 1000, 30000),
            &[],
            b"!",
        ));
        assert_eq!(received(&so), b"hello world!");
        assert_eq!(stat(C::TcpsPreddat), pred, "not predicted");
        assert_eq!(peek(&inp, |st| (st.snd_wnd, st.rcv_nxt)), (30000, 5012));
        testtcp::close(so, inp);
    }

    #[test]
    fn input_frees_acknowledged_data_by_prediction() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tia1", 101);
        queue(&so, &pattern(100));
        testtcp::with_tp(&inp, |mut tp| {
            // The window the peer will repeat (header prediction needs tiwin == snd_wnd and
            // snd_cwnd >= snd_wnd).
            tp.st.snd_wnd = 8192;
            let _ = tcp_output(&mut tp);
            ((), None)
        });
        assert_eq!(sent(d), 1);
        let pred = stat(C::TcpsPredack);
        deliver(from_peer(101, peer_th(TH_ACK, 5000, 1100, 8192), &[], &[]));
        assert_eq!(stat(C::TcpsPredack), pred + 1);
        assert_eq!(so.so_snd.cc(), 0, "the acknowledged data dropped");
        let (una, rexmt) = peek(&inp, |st| (st.snd_una, tcp_timer_isarmed(st, TCPT_REXMT)));
        assert_eq!(una, 1100);
        assert!(!rexmt, "nothing outstanding: no retransmit timer");
        testtcp::close(so, inp);
    }

    #[test]
    fn input_refuses_short_and_malformed_segments() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tia2", 102);
        let good = || from_peer(102, peer_th(TH_ACK, 5000, 1000, 65535), &[], b"data");
        // (the segment, the counter it bumps)
        let off4 = {
            let mut m = good();
            m.data_mut().expect("writable")[32] = 4 << 4;
            m
        };
        let off15 = {
            let mut m = good();
            m.data_mut().expect("writable")[32] = 15 << 4;
            m
        };
        let short = {
            let m = good();
            m.m_len().set(30);
            m.m_pkthdr().len.set(30);
            m
        };
        let badsum = seg4(
            [10, 9, 102, 2],
            [10, 9, 102, 1],
            peer_th(TH_ACK, 5000, 1000, 65535),
            &[],
            b"data",
        );
        let markedbad = {
            let m = good();
            let ph = m.m_pkthdr();
            ph.csum_flags.set(M_TCP_CSUM_IN_BAD);
            m
        };
        let bcast = {
            let m = good();
            m.m_flags().set(m.m_flags().get() | M_BCAST);
            m
        };
        for (m, c, what) in [
            (off4, Some(C::TcpsRcvbadoff), "th_off 4"),
            (off15, Some(C::TcpsRcvbadoff), "th_off past the segment"),
            (short, Some(C::TcpsRcvshort), "a header cut short"),
            (badsum, Some(C::TcpsRcvbadsum), "a bad checksum"),
            (markedbad, Some(C::TcpsRcvbadsum), "M_TCP_CSUM_IN_BAD"),
            (bcast, None, "a broadcast"),
        ] {
            let before = c.map(stat);
            deliver(m);
            if let (Some(c), Some(b)) = (c, before) {
                assert_eq!(stat(c), b + 1, "{what}");
            }
            assert!(received(&so).is_empty(), "{what}: no data taken");
            assert_eq!(sent(d), 0, "{what}: nothing answered");
        }
        // The checksum computed here: verified in software and the data taken.
        let mut m = seg4(
            [10, 9, 102, 2],
            [10, 9, 102, 1],
            peer_th(TH_ACK, 5000, 1000, 65535),
            &[],
            b"data",
        );
        let sum = in4_cksum(&m, IPPROTO_TCP as u8, 20, 24);
        m.data_mut().expect("writable")[36..38].copy_from_slice(&sum.to_ne_bytes());
        let sw = stat(C::TcpsInswcsum);
        deliver(m);
        assert_eq!(stat(C::TcpsInswcsum), sw + 1);
        assert_eq!(received(&so), b"data");
        testtcp::close(so, inp);
    }

    #[test]
    fn input_resets_a_segment_for_no_connection() {
        let (_g, _p) = testtcp::setup();
        let (_ifp, d) = crate::netinet::in_pcb::testinet::iface(b"tia3", [10, 9, 103, 1]);
        let noport = stat(C::TcpsNoport);
        let mut th = peer_th(TH_SYN, 777, 0, 4096);
        th.th_dport = 4444u16.to_be();
        deliver(from_peer(103, th, &[], &[]));
        assert_eq!(stat(C::TcpsNoport), noport + 1);
        assert_eq!(sent(d), 1, "a reset");
        let (ip, rth, _o, _p) = last(d);
        assert_eq!(ip.ip_dst.s_addr, u32::from_ne_bytes([10, 9, 103, 2]));
        assert_eq!(rth.th_flags, TH_RST | TH_ACK);
        assert_eq!(u32::from_be(rth.th_ack), 778, "the SYN counted");
        assert_eq!(
            (u16::from_be(rth.th_sport), u16::from_be(rth.th_dport)),
            (4444, 80)
        );
        // To port 0: no connection either, and reset.
        th.th_dport = 0;
        deliver(from_peer(103, th, &[], &[]));
        assert_eq!(stat(C::TcpsNoport), noport + 2);
        assert_eq!(sent(d), 2);
        // A reset is never answered.
        let mut rst = peer_th(TH_RST, 1, 0, 0);
        rst.th_dport = 4444u16.to_be();
        deliver(from_peer(103, rst, &[], &[]));
        assert_eq!(sent(d), 2);
    }

    #[test]
    fn input_rst_must_fall_at_the_window_edge() {
        let (_g, _p) = testtcp::setup();
        // (the RST's sequence number, whether it resets): rcv_nxt, rcv_nxt + 1 and the last
        // ACK sent are taken (RFC 5961 as the C does it); anything else is dropped.
        for (i, (seq, resets)) in [
            (5000u32, true),
            (5001, true),
            (4321, true),
            (5002, false),
            (4999, false),
            (9000, false),
        ]
        .into_iter()
        .enumerate()
        {
            let n = 104 + i as u8;
            let name = [b't', b'r', b's', b'0' + i as u8];
            let (so, inp, d) = estab(&name, n);
            testtcp::with_tp(&inp, |tp| {
                tp.st.last_ack_sent = 4321;
                ((), None)
            });
            let (drops, cbs) = (stat(C::TcpsDrops), tcpcbs());
            deliver(from_peer(n, peer_th(TH_RST, seq, 0, 0), &[], &[]));
            assert_eq!(sent(d), 0, "seq {seq}: never answered");
            if resets {
                assert_eq!(so.error(), Some(Errno::ECONNRESET), "seq {seq}");
                assert_eq!(stat(C::TcpsDrops), drops + 1);
                assert!(!has_tcpcb(&inp), "seq {seq}: the control block freed");
                assert_eq!(tcpcbs(), cbs - 1);
                // in_pcbdetach emptied so_pcb (both words): no pcb, the socket free to go.
                assert!(sotoinpcb(&so).is_none(), "seq {seq}: so_pcb's slot emptied");
                assert!(!so.has_pcb());
                drop((so, inp));
            } else {
                assert_eq!(so.error(), None, "seq {seq}");
                assert_eq!(peek(&inp, |st| st.t_state), TCPS_ESTABLISHED);
                testtcp::close(so, inp);
            }
        }
    }

    /// Restores `TCP_ACKDROP_PPSLIM` when dropped (a failing test leaves it as it was).
    struct AckdropLim(i32);
    impl Drop for AckdropLim {
        fn drop(&mut self) {
            TCP_ACKDROP_PPSLIM.store(self.0, Ordering::Relaxed);
        }
    }

    #[test]
    fn input_challenges_a_syn_in_the_window_within_the_rate_limit() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tia4", 110);
        // A SYN in the window: an ACK reflecting our state, the segment dropped.
        deliver(from_peer(110, peer_th(TH_SYN, 5000, 0, 65535), &[], &[]));
        assert_eq!(sent(d), 1);
        let (_ip, th, _o, _p) = last(d);
        assert_eq!(th.th_flags, TH_ACK);
        assert_eq!(
            (u32::from_be(th.th_seq), u32::from_be(th.th_ack)),
            (1000, 5000)
        );
        assert_eq!(peek(&inp, |st| st.t_state), TCPS_ESTABLISHED);
        // At the limit (0: none at all), no ACK.
        let _restore = AckdropLim(TCP_ACKDROP_PPSLIM.swap(0, Ordering::Relaxed));
        deliver(from_peer(110, peer_th(TH_SYN, 5000, 0, 65535), &[], &[]));
        assert_eq!(sent(d), 1, "rate-limited");
        testtcp::close(so, inp);
    }

    #[test]
    fn input_takes_the_urgent_byte_out_of_band() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tia5", 111);
        let mut th = peer_th(TH_ACK | TH_URG, 5000, 1000, 65535);
        th.th_urp = 3u16.to_be();
        deliver(from_peer(111, th, &[], b"abcdef"));
        // The urgent pointer points past the urgent byte: "c", out of the stream.
        assert_eq!(received(&so), b"abdef");
        assert_eq!(so.oobmark(), 2);
        let (iobc, oob, up, nxt) =
            peek(&inp, |st| (st.t_iobc, st.t_oobflags, st.rcv_up, st.rcv_nxt));
        assert_eq!((iobc, oob), (b'c', TCPOOB_HAVEDATA));
        assert_eq!((up, nxt), (5003, 5006), "the sequence space keeps the byte");
        testtcp::close(so, inp);
    }

    #[test]
    fn input_fin_moves_to_close_wait_and_is_acknowledged() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tia6", 112);
        deliver(from_peer(
            112,
            peer_th(TH_ACK | TH_FIN, 5000, 1000, 65535),
            &[],
            b"bye",
        ));
        assert_eq!(received(&so), b"bye");
        assert!(so.so_rcv.has_state(SS_CANTRCVMORE));
        let (state, nxt) = peek(&inp, |st| (st.t_state, st.rcv_nxt));
        assert_eq!((state, nxt), (TCPS_CLOSE_WAIT, 5004));
        assert_eq!(sent(d), 1);
        let (_ip, th, _o, _p) = last(d);
        assert_eq!((th.th_flags, u32::from_be(th.th_ack)), (TH_ACK, 5004));
        testtcp::close(so, inp);
    }

    #[test]
    fn input_third_duplicate_ack_retransmits_the_first_segment() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tia7", 113);
        let data = pattern(4000);
        queue(&so, &data);
        testtcp::with_tp(&inp, |mut tp| {
            // NewReno's recovery (the SACK one starts from the holes).
            tp.st.sack_enable = 0;
            let _ = tcp_output(&mut tp);
            ((), None)
        });
        let sent0 = sent(d);
        assert!(sent0 >= 3, "the window's segments out");
        let fast = stat(C::TcpsSndrexmitfast);
        for i in 0..3 {
            deliver(from_peer(113, peer_th(TH_ACK, 5000, 1000, 65535), &[], &[]));
            assert_eq!(sent(d), sent0 + u32::from(i == 2), "duplicate {}", i + 1);
        }
        assert_eq!(stat(C::TcpsSndrexmitfast), fast + 1);
        let (_ip, th, _o, payload) = last(d);
        assert_eq!(
            u32::from_be(th.th_seq),
            1000,
            "the first unacknowledged byte"
        );
        assert_eq!(payload, &data[..1460]);
        let (dup, nxt, cwnd, ssthresh) = peek(&inp, |st| {
            (st.t_dupacks, st.snd_nxt, st.snd_cwnd, st.snd_ssthresh)
        });
        assert_eq!(dup, 3);
        assert_eq!(nxt, 1000 + 4000, "snd_nxt back where it was");
        assert_eq!(cwnd, ssthresh + 3 * 1460);
        testtcp::close(so, inp);
    }

    #[test]
    fn input_completes_a_handshake_through_the_syn_cache() {
        let (_g, _p) = testtcp::setup();
        let (lso, linp, d) = listener(b"tih0", 114, 80);
        let (cbs, added, completed, accepts, connects) = (
            tcpcbs(),
            stat(C::TcpsScAdded),
            stat(C::TcpsScCompleted),
            stat(C::TcpsAccepts),
            stat(C::TcpsConnects),
        );
        let syn = |flags, seq, ack, opts: &[u8], data: &[u8]| {
            let mut th = peer_th(flags, seq, ack, 16384);
            th.th_sport = 5555u16.to_be();
            th.th_dport = 80u16.to_be();
            csum_ok(seg4([10, 9, 114, 2], [10, 9, 114, 1], th, opts, data))
        };
        // The SYN (an MSS of 1400, SACK permitted): an entry in the cache and its SYN-ACK on
        // the wire, SACK permitted (the listener's sack_enable is on).
        let sackp = [
            TCPOPT_MAXSEG,
            4,
            0x05,
            0x78,
            TCPOPT_NOP,
            TCPOPT_NOP,
            TCPOPT_SACK_PERMITTED,
            2,
        ];
        deliver(syn(TH_SYN, 777, 0, &sackp, &[]));
        assert_eq!(stat(C::TcpsScAdded), added + 1);
        assert_eq!(sent(d), 1);
        let (_ip, sa, opts, _p) = last(d);
        assert_eq!(sa.th_flags, TH_SYN | TH_ACK);
        assert_eq!(u32::from_be(sa.th_ack), 778);
        assert_eq!(&opts[..2], &[TCPOPT_MAXSEG, 4], "our MSS offered");
        assert_eq!(
            &opts[4..8],
            &[TCPOPT_NOP, TCPOPT_NOP, TCPOPT_SACK_PERMITTED, 2],
            "SACK permitted"
        );
        let iss = u32::from_be(sa.th_seq);
        assert!(lso.queued_first().is_none(), "no connection yet");
        // An ACK that is not of our SYN-ACK: the SYN-ACK sent again, no connection made.
        deliver(syn(TH_ACK, 778, iss.wrapping_add(2), &[], &[]));
        assert_eq!(sent(d), 2, "the SYN-ACK again");
        let (_ip, sa2, _o, _p) = last(d);
        assert_eq!(
            (sa2.th_flags, u32::from_be(sa2.th_seq)),
            (TH_SYN | TH_ACK, iss)
        );
        assert!(lso.queued_first().is_none());
        assert_eq!(stat(C::TcpsScCompleted), completed);
        // The ACK of the SYN-ACK: a new connection on the listener's queue, established.
        deliver(syn(TH_ACK, 778, iss.wrapping_add(1), &[], &[]));
        assert_eq!(stat(C::TcpsScCompleted), completed + 1);
        assert_eq!(stat(C::TcpsAccepts), accepts + 1);
        assert_eq!(stat(C::TcpsConnects), connects + 1);
        assert_eq!(tcpcbs(), cbs + 1, "the child's control block");
        let child = lso.queued_first().expect("the new connection");
        let cinp = {
            let _w = crate::net::socket::solock(&child);
            sotoinpcb(&child).expect("its pcb")
        };
        let (state, una, nxt, maxseg) = peek(&cinp, |st| {
            (st.t_state, st.snd_una, st.rcv_nxt, st.t_maxseg)
        });
        assert_eq!(state, TCPS_ESTABLISHED);
        assert_eq!((una, nxt), (iss.wrapping_add(1), 778));
        assert_eq!(maxseg, 1400, "the peer's MSS");
        assert!(child.has_state(crate::net::sockbuf::SS_ISCONNECTED));
        // The child's SACK from the entry's SCF_SACK_PERMIT (ISSET's bit, as the C's).
        let (sack, permit) = peek(&cinp, |st| (st.sack_enable, st.has_flags(TF_SACK_PERMIT)));
        assert_eq!(sack, i32::from(crate::netinet::tcp_var::SCF_SACK_PERMIT));
        assert!(permit);
        // Data on the new connection reaches its buffer.
        deliver(syn(TH_ACK, 778, iss.wrapping_add(1), &[], b"hello"));
        assert_eq!(received(&child), b"hello");
        // An ACK from a port with no entry: badsyn, a reset.
        let badsyn = stat(C::TcpsBadsyn);
        let mut th = peer_th(TH_ACK, 42, 4242, 16384);
        th.th_sport = 6666u16.to_be();
        th.th_dport = 80u16.to_be();
        let before = sent(d);
        deliver(csum_ok(seg4(
            [10, 9, 114, 2],
            [10, 9, 114, 1],
            th,
            &[],
            &[],
        )));
        assert_eq!(stat(C::TcpsBadsyn), badsyn + 1);
        assert_eq!(sent(d), before + 1);
        let (_ip, rst, _o, _p) = last(d);
        assert_eq!((rst.th_flags, u32::from_be(rst.th_seq)), (TH_RST, 4242));
        testtcp::close(child, cinp);
        testtcp::close_listener(lso, linp);
        assert_eq!(tcpcbs(), cbs - 1, "the listener's and the child's freed");
    }

    #[test]
    fn input_listener_without_sack_permits_none() {
        let (_g, _p) = testtcp::setup();
        let (lso, linp, d) = listener(b"tih1", 120, 80);
        testtcp::with_tp(&linp, |tp| {
            tp.st.sack_enable = 0;
            ((), None)
        });
        let mut th = peer_th(TH_SYN, 300, 0, 16384);
        th.th_sport = 5000u16.to_be();
        th.th_dport = 80u16.to_be();
        let opts = [TCPOPT_NOP, TCPOPT_NOP, TCPOPT_SACK_PERMITTED, 2];
        deliver(csum_ok(seg4(
            [10, 9, 120, 2],
            [10, 9, 120, 1],
            th,
            &opts,
            &[],
        )));
        assert_eq!(sent(d), 1);
        let (_ip, sa, o, _p) = last(d);
        assert_eq!(sa.th_flags, TH_SYN | TH_ACK);
        // The MSS alone: no SACK permitted where the listener has SACK off.
        assert_eq!(o.len(), 4);
        assert_eq!(&o[..2], &[TCPOPT_MAXSEG, 4]);
        testtcp::close_listener(lso, linp);
    }

    /// NOP, NOP and the timestamp option `val`, `ecr` (RFC 1323 appendix A).
    fn ts_opt(val: u32, ecr: u32) -> [u8; 12] {
        let mut o = [0u8; 12];
        o[..4].copy_from_slice(&[TCPOPT_NOP, TCPOPT_NOP, TCPOPT_TIMESTAMP, TCPOLEN_TIMESTAMP]);
        o[4..8].copy_from_slice(&val.to_be_bytes());
        o[8..].copy_from_slice(&ecr.to_be_bytes());
        o
    }

    /// The timestamp option of the option block `opts`: its value and its echo.
    fn ts_of(opts: &[u8]) -> Option<(u32, u32)> {
        let be = |j: usize| u32::from_be_bytes([opts[j], opts[j + 1], opts[j + 2], opts[j + 3]]);
        walk(opts)
            .into_iter()
            .find(|&(k, _, l)| k == TCPOPT_TIMESTAMP && l == usize::from(TCPOLEN_TIMESTAMP))
            .map(|(_, i, _)| (be(i + 2), be(i + 6)))
    }

    /// A segment from 10.9.`n`.2:`sport` to port 80 of 10.9.`n`.1 (a listener's), its
    /// checksum verified.
    #[allow(clippy::too_many_arguments, reason = "a segment's fields")]
    fn to80(n: u8, sport: u16, flags: u8, seq: u32, ack: u32, opts: &[u8], data: &[u8]) -> MbufBox {
        let mut th = peer_th(flags, seq, ack, 16384);
        th.th_sport = sport.to_be();
        th.th_dport = 80u16.to_be();
        csum_ok(seg4([10, 9, n, 2], [10, 9, n, 1], th, opts, data))
    }

    /// The first connection on the completed queue of `lso`, and its pcb.
    fn child_of(lso: &Socket<G>) -> Option<(crate::net::socket::SocketRef<G>, InpcbRef<G>)> {
        let child = lso.queued_first()?;
        let cinp = {
            let _w = crate::net::socket::solock(&child);
            sotoinpcb(&child).expect("its pcb")
        };
        Some((child, cinp))
    }

    /// The SYN cache entry of another listener completed on this one, over the interface
    /// `name` of 10.9.`n`.1: the SYN's, the duplicate SYN's and the ACK's timestamps `ts`, and
    /// the new connection's `ts_recent` after the ACK, `want_ts_recent`.
    fn another_listener_s_entry(name: &[u8], n: u8, ts: [u32; 3], want_ts_recent: u32) {
        let (_g, _p) = testtcp::setup();
        let (_ifp, d) = crate::netinet::in_pcb::testinet::iface(name, [10, 9, n, 1]);
        // Listener A, on the wildcard, takes the SYN: an MSS of 1400, SACK permitted, a
        // timestamp.
        let (aso, ainp) = testtcp::listen_at([0; 4], 80);
        let syn = |ts: u32| {
            let mut o = std::vec![
                TCPOPT_MAXSEG,
                4,
                0x05,
                0x78,
                TCPOPT_NOP,
                TCPOPT_NOP,
                TCPOPT_SACK_PERMITTED,
                2,
            ];
            o.extend_from_slice(&ts_opt(ts, 0));
            to80(n, 5600, TH_SYN, 3000, 0, &o, &[])
        };
        deliver(syn(ts[0]));
        assert_eq!(sent(d), 1);
        let (_ip, sa, o, _p) = last(d);
        assert_eq!(sa.th_flags, TH_SYN | TH_ACK);
        let iss = u32::from_be(sa.th_seq);
        assert_eq!(
            ts_of(&o).map(|t| t.1),
            Some(ts[0]),
            "the SYN's timestamp echoed"
        );
        // Listener B, bound to the address, is found first from now on: the entry is A's.
        let (bso, binp) = testtcp::listen_at([10, 9, n, 1], 80);
        // A duplicate SYN reaches B: counted and answered with the entry's SYN-ACK (the same
        // iss), echoing this SYN's timestamp, as the C's after its overwrite.
        let (dupe, acks) = (stat(C::TcpsScDupesyn), stat(C::TcpsSndacks));
        deliver(syn(ts[1]));
        assert_eq!(stat(C::TcpsScDupesyn), dupe + 1);
        assert_eq!(stat(C::TcpsSndacks), acks + 1);
        assert_eq!(sent(d), 2, "the SYN-ACK again");
        let (_ip, sa2, o2, _p) = last(d);
        assert_eq!(
            (
                sa2.th_flags,
                u32::from_be(sa2.th_seq),
                u32::from_be(sa2.th_ack)
            ),
            (TH_SYN | TH_ACK, iss, 3001)
        );
        assert_eq!(ts_of(&o2).map(|t| t.1), Some(ts[1]));
        // An ACK whose numbers do not match: dropped, no SYN-ACK, nothing made, nothing
        // counted, the entry left.
        let (aborted, completed) = (stat(C::TcpsScAborted), stat(C::TcpsScCompleted));
        deliver(to80(
            n,
            5600,
            TH_ACK,
            3001,
            iss.wrapping_add(2),
            &ts_opt(ts[2], 0),
            &[],
        ));
        assert_eq!(sent(d), 2, "nothing sent");
        assert!(aso.queued_first().is_none() && bso.queued_first().is_none());
        assert_eq!(stat(C::TcpsScAborted), aborted);
        // The ACK of the SYN-ACK: the connection made on B from the entry.
        deliver(to80(
            n,
            5600,
            TH_ACK,
            3001,
            iss.wrapping_add(1),
            &ts_opt(ts[2], 0),
            &[],
        ));
        assert_eq!(stat(C::TcpsScCompleted), completed + 1);
        assert_eq!(stat(C::TcpsScAborted), aborted);
        assert!(aso.queued_first().is_none(), "nothing on A's queue");
        let (child, cinp) = child_of(&bso).expect("the connection on B's queue");
        assert_eq!(
            (cinp.inp_laddr.get().s_addr, cinp.inp_faddr.get().s_addr),
            (
                u32::from_ne_bytes([10, 9, n, 1]),
                u32::from_ne_bytes([10, 9, n, 2])
            )
        );
        assert_eq!(
            (cinp.inp_lport.get(), cinp.inp_fport.get()),
            (80u16.to_be(), 5600u16.to_be())
        );
        let (state, una, nxt, maxseg) = peek(&cinp, |st| {
            (st.t_state, st.snd_una, st.rcv_nxt, st.t_maxseg)
        });
        assert_eq!(state, TCPS_ESTABLISHED);
        assert_eq!((una, nxt), (iss.wrapping_add(1), 3001));
        assert_eq!(
            maxseg,
            1400 - u16::from(TCPOLEN_TSTAMP_APPA),
            "the peer's MSS less the timestamps' room"
        );
        let (sack, flags, ts_recent) = peek(&cinp, |st| (st.sack_enable, st.t_flags, st.ts_recent));
        assert_eq!(sack, i32::from(crate::netinet::tcp_var::SCF_SACK_PERMIT));
        for f in [TF_SACK_PERMIT, TF_REQ_TSTMP, TF_RCVD_TSTMP] {
            assert_ne!(flags & f, 0, "flag {f:#x}");
        }
        assert_eq!(ts_recent, want_ts_recent);
        // The entry is gone: once the connection is closed, the same ACK again finds none
        // and is reset by the listener.
        testtcp::close(child, cinp);
        let (badsyn, before) = (stat(C::TcpsBadsyn), sent(d));
        deliver(to80(n, 5600, TH_ACK, 3001, iss.wrapping_add(1), &[], &[]));
        assert_eq!(stat(C::TcpsBadsyn), badsyn + 1);
        assert_eq!(sent(d), before + 1, "the reset");
        assert_eq!(
            bso.qlen(),
            1,
            "no second connection (the closed one stays queued)"
        );
        testtcp::close_listener(bso, binp);
        testtcp::close_listener(aso, ainp);
    }

    #[test]
    fn input_completes_another_listener_s_entry_on_this_one() {
        // The child's ts_recent starts at 0 (the C's: the SYN's), and this ACK's timestamp
        // step records the ACK's, as the C's does.
        another_listener_s_entry(b"tib0", 121, [0x100, 0x180, 0x200], 0x200);
    }

    #[test]
    fn input_completes_another_listener_s_entry_ts_recent_0_past_2_31() {
        // tcp_syncache.rs's Deviations: an ACK timestamp 2^31 or more past 0, not older
        // than the SYN's, is not recorded over a ts_recent of 0 (TSTMP_GEQ(ts_val, 0) is
        // false), where the C records it over the SYN's (0x8000_0200 here).
        another_listener_s_entry(b"tib1", 122, [0x8000_0100, 0x8000_0180, 0x8000_0200], 0);
    }

    #[test]
    fn input_mlist_keeps_one_socket_locked_across_its_segments() {
        let (_g, _p) = testtcp::setup();
        let (aso, ainp, _da) = estab(b"tia8", 115);
        let (bso, binp, _db) = estab(b"tia9", 116);
        let seg = |n: u8, seq: u32, data: &[u8]| {
            from_peer(n, peer_th(TH_ACK | TH_PUSH, seq, 1000, 65535), &[], data)
        };
        let locks = || TEST_SOLOCKS.load(Ordering::Relaxed);
        // Two segments of one connection: its socket locked once.
        let l0 = locks();
        deliver_list(std::vec![seg(115, 5000, b"ab"), seg(115, 5002, b"cd")]);
        assert_eq!(locks() - l0, 1);
        assert_eq!(received(&aso), b"abcd");
        // A, B, A: three lockings, the kept socket let go for the other.
        let l0 = locks();
        deliver_list(std::vec![
            seg(115, 5004, b"e"),
            seg(116, 5000, b"x"),
            seg(115, 5005, b"f"),
        ]);
        assert_eq!(locks() - l0, 3);
        assert_eq!(received(&aso), b"abcdef");
        assert_eq!(received(&bso), b"x");
        testtcp::close(aso, ainp);
        testtcp::close(bso, binp);

        // A listener's handshake then data on the new connection in one batch: the listener
        // locked once, the child kept from syn_cache_get to its data.
        let (lso, linp, d) = listener(b"tiha", 117, 80);
        let syn = |flags, seq, ack, data: &[u8]| {
            let mut th = peer_th(flags, seq, ack, 16384);
            th.th_sport = 5556u16.to_be();
            th.th_dport = 80u16.to_be();
            csum_ok(seg4([10, 9, 117, 2], [10, 9, 117, 1], th, &[], data))
        };
        deliver(syn(TH_SYN, 900, 0, &[]));
        let (_ip, sa, _o, _p) = last(d);
        let iss = u32::from_be(sa.th_seq);
        let l0 = locks();
        deliver_list(std::vec![
            syn(TH_ACK, 901, iss.wrapping_add(1), &[]),
            syn(TH_ACK | TH_PUSH, 901, iss.wrapping_add(1), b"hi"),
        ]);
        assert_eq!(locks() - l0, 1, "the child is not locked again");
        let child = lso.queued_first().expect("the new connection");
        assert_eq!(received(&child), b"hi");
        let cinp = {
            let _w = crate::net::socket::solock(&child);
            sotoinpcb(&child).expect("its pcb")
        };
        testtcp::close(child, cinp);
        testtcp::close_listener(lso, linp);
    }

    #[test]
    fn input_reuses_time_wait_for_a_new_syn() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab(b"tiaa", 118);
        let (lso, linp) = testtcp::listen_on(118, 7000);
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_TIME_WAIT;
            tp.st.snd_nxt = 1000;
            ((), None)
        });
        let cbs = tcpcbs();
        // A SYN above rcv_nxt: the old connection closed, the SYN goes to the listener, whose
        // SYN-ACK starts at least 32768 past the old snd_nxt (msb clear).
        deliver(from_peer(118, peer_th(TH_SYN, 6000, 0, 4096), &[], &[]));
        assert!(!has_tcpcb(&inp), "the old control block freed");
        assert_eq!(tcpcbs(), cbs - 1);
        let (_ip, sa, _o, _p) = last(d);
        assert_eq!(sa.th_flags, TH_SYN | TH_ACK);
        assert_eq!(u32::from_be(sa.th_ack), 6001);
        let step = u32::from_be(sa.th_seq).wrapping_sub(1000);
        assert!(
            (0x8000..0x8000_0000).contains(&step),
            "iss {step:#x} past the old"
        );
        drop((so, inp));
        testtcp::close_listener(lso, linp);
    }

    #[test]
    fn input_checks_a_signed_connection_s_segments() {
        let (_g, _p) = testtcp::setup();
        let _legs = sig_legs();
        let (so, inp, _d) = estab(b"tiab", SIG_NET);
        testtcp::with_tp(&inp, |tp| {
            tp.st.sack_enable = 0;
            tp.st.set_flags(TF_SIGNATURE);
            ((), None)
        });
        let th = peer_th(TH_ACK, 5000, 1000, 65535);
        let (src, dst) = ([10, 9, SIG_NET, 2], [10, 9, SIG_NET, 1]);
        let good = digest4(src, dst, &th, 40, b"signed");
        let (goodc, badc) = (stat(C::TcpsRcvgoodsig), stat(C::TcpsRcvbadsig));
        deliver(csum_ok(seg4(src, dst, th, &sig_opt(&good), b"signed")));
        assert_eq!(received(&so), b"signed");
        assert_eq!(stat(C::TcpsRcvgoodsig), goodc + 1);
        // The next segment with a digest one bit off, or none: dropped and counted.
        let th = peer_th(TH_ACK, 5006, 1000, 65535);
        let mut bad = digest4(src, dst, &th, 40, b"forged");
        bad[9] ^= 0x04;
        deliver(csum_ok(seg4(src, dst, th, &sig_opt(&bad), b"forged")));
        deliver(csum_ok(seg4(src, dst, th, &[TCPOPT_NOP; 4], b"forged")));
        assert_eq!(received(&so), b"signed");
        assert_eq!(stat(C::TcpsRcvbadsig), badc + 2);
        assert_eq!(peek(&inp, |st| st.rcv_nxt), 5006);
        testtcp::close(so, inp);
    }

    #[test]
    fn input_drops_what_the_inbound_policy_refuses() {
        let (_g, _p) = testtcp::setup();
        let _legs = sig_legs();
        let (_ifp, d) = crate::netinet::in_pcb::testinet::iface(b"tiac", [10, 9, 72, 1]);
        let nosec = stat(C::TcpsRcvnosec);
        let noport = stat(C::TcpsNoport);
        let m = csum_ok(seg4(
            testtcp::SPD_DENY,
            [10, 9, 72, 1],
            peer_th(TH_SYN, 1, 0, 4096),
            &[],
            &[],
        ));
        deliver(m);
        assert_eq!(stat(C::TcpsRcvnosec), nosec + 1);
        assert_eq!(
            stat(C::TcpsNoport),
            noport,
            "dropped before the port is looked at"
        );
        assert_eq!(sent(d), 0, "no reset");
        // Another source on the same net is not refused: no connection, a reset.
        let m = csum_ok(seg4(
            [10, 9, 72, 67],
            [10, 9, 72, 1],
            peer_th(TH_SYN, 1, 0, 4096),
            &[],
            &[],
        ));
        deliver(m);
        assert_eq!(stat(C::TcpsNoport), noport + 1);
        assert_eq!(sent(d), 1);
    }

    #[test]
    fn input_takes_data_over_ipv6() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = estab6(b"tiad", 119);
        let th = peer_th(TH_ACK | TH_PUSH, 5000, 1000, 65535);
        let mut thv = th;
        thv.set_th_off(5);
        let data = b"six";
        let mut p = std::vec![0u8; 40 + 20 + data.len()];
        let ip6 = Ip6Hdr {
            ip6_flow: 0x6000_0000u32.to_be(),
            ip6_plen: ((20 + data.len()) as u16).to_be(),
            ip6_nxt: IPPROTO_TCP as u8,
            ip6_hlim: 64,
            ip6_src: testtcp::a6(119, 2),
            ip6_dst: testtcp::a6(119, 1),
        };
        assert!(hal::pod::put(&mut p, 0, &ip6));
        assert!(hal::pod::put(&mut p, 40, &thv));
        p[60..].copy_from_slice(data);
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(p.len() as u32);
        m.m_pkthdr().len.set(p.len() as i32);
        m.data_mut().expect("writable").copy_from_slice(&p);
        deliver_af(csum_ok(m), AF_INET6_I32);
        assert_eq!(received(&so), b"six");
        // An IPv6 segment from the unspecified address is dropped before any lookup.
        let mut p2 = p.clone();
        p2[8..24].copy_from_slice(&In6Addr::default().s6_addr);
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(p2.len() as u32);
        m.m_pkthdr().len.set(p2.len() as i32);
        m.data_mut().expect("writable").copy_from_slice(&p2);
        deliver_af(csum_ok(m), AF_INET6_I32);
        assert_eq!(received(&so), b"six");
        // The FIN over IPv6: acknowledged on the wire.
        let fin = peer_th(TH_ACK | TH_FIN, 5003, 1000, 65535);
        let mut thv = fin;
        thv.set_th_off(5);
        let mut p3 = std::vec![0u8; 60];
        let ip6 = Ip6Hdr {
            ip6_plen: 20u16.to_be(),
            ..ip6
        };
        assert!(hal::pod::put(&mut p3, 0, &ip6));
        assert!(hal::pod::put(&mut p3, 40, &thv));
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(60);
        m.m_pkthdr().len.set(60);
        m.data_mut().expect("writable").copy_from_slice(&p3);
        let before = sent(d);
        deliver_af(csum_ok(m), AF_INET6_I32);
        assert_eq!(sent(d), before + 1);
        let (_ip6, ath, _o, _p) = last6(d);
        assert_eq!(u32::from_be(ath.th_ack), 5004);
        testtcp::close(so, inp);
    }

    /// Fails the test protocol's attaches (`testtcp::FAIL_ATTACH`), for a test, until the
    /// drop.
    struct FailAttach;

    impl FailAttach {
        fn new() -> Self {
            testtcp::FAIL_ATTACH.store(true, Ordering::Relaxed);
            Self
        }
    }

    impl Drop for FailAttach {
        fn drop(&mut self) {
            testtcp::FAIL_ATTACH.store(false, Ordering::Relaxed);
        }
    }

    #[test]
    fn input_resets_a_handshake_whose_socket_cannot_be_made() {
        let (_g, _p) = testtcp::setup();
        let n = 125;
        let (lso, linp, d) = listener(b"tic2", n, 80);
        deliver(to80(n, 5700, TH_SYN, 400, 0, &[], &[]));
        let (_ip, sa, _o, _p) = last(d);
        let iss = u32::from_be(sa.th_seq);
        let table = &crate::net::domain::net::<G>().tcp.tcbtable;
        let (cbs, pcbs, aborted, completed) = (
            tcpcbs(),
            table.count(),
            stat(C::TcpsScAborted),
            stat(C::TcpsScCompleted),
        );
        // The ACK, while the new socket's attach fails after its pcb is made (sonewconn
        // fails): resetandabort, the peer reset at its acknowledgment, the entry gone.
        let fail = FailAttach::new();
        deliver(to80(n, 5700, TH_ACK, 401, iss.wrapping_add(1), &[], &[]));
        drop(fail);
        assert_eq!(stat(C::TcpsScAborted), aborted + 1);
        assert_eq!(stat(C::TcpsScCompleted), completed);
        let (_ip, rst, _o, _p) = last(d);
        assert_eq!(
            (rst.th_flags, u32::from_be(rst.th_seq)),
            (TH_RST, iss.wrapping_add(1))
        );
        assert!(lso.queued_first().is_none(), "nothing queued");
        assert_eq!(
            (tcpcbs(), table.count()),
            (cbs, pcbs),
            "the new pcb detached again, nothing left"
        );
        // The entry is gone: the same ACK again is a bad SYN, reset.
        let badsyn = stat(C::TcpsBadsyn);
        deliver(to80(n, 5700, TH_ACK, 401, iss.wrapping_add(1), &[], &[]));
        assert_eq!(stat(C::TcpsBadsyn), badsyn + 1);
        testtcp::close_listener(lso, linp);
    }
}
/* </TESTS> */
