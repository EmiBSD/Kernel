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
 * Copyright (c) 1982, 1986, 1993, 1994
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
 *	@(#)tcp_var.h	8.3 (Berkeley) 4/10/94
 */
/* </LICENSES> */

/* <CODE> */
//! Kernel variables for TCP: `<netinet/tcp_var.h>`.
//!
//! Upstream: sys/netinet/tcp_var.h @ 3ce1f3f79392
//!
//! The TCP control block ([`Tcpcb`], one per connection, hung off the internet control block
//! as `inp_ppcb`), the reassembly queue entries, the SACK blocks and holes, the SYN cache, the
//! statistics with their per-counter enumeration, and the `net.inet.tcp` sysctl names.
//!
//! Locks used to protect global data and struct members: \[I\] immutable after creation,
//! \[N\] net lock, \[S\] `syn_cache_mtx`, \[s\] `so_lock` of the listen socket.
//!
//! ## Deviations
//! - `struct tcpcb` is a `tcpcb_pool` item of `Cell`s (the `struct mbuf` idiom of
//!   `docs/C_TO_RUST.md`): every layer changes it through a shared pointer under the socket
//!   lock. `t_inpcb` is set once by `tcp_newtcpcb` and is a `&'static Inpcb`. `t_state` is an
//!   `i32` (a `short` in C) so that it compares with the `TCPS_*` constants without casts;
//!   `t_force` is a `bool` (a `char` used as a flag); `t_softerror` is an `Option<Errno>` (a
//!   `short` holding an errno or 0). `sackblks[]` is an array of `Cell<Sackblk>`.
//! - `struct tcpqent`'s `tcpqe_tcp`, the C's pointer to the segment's TCP header inside the
//!   mbuf, is a copy of the header (`Cell<Tcphdr>`), host order as `tcp_input` leaves it:
//!   only its sequence number, the length kept in `th_reseqlen` and the flags are read or
//!   changed, and the mbuf data no longer covers the header once `tcp_input` has trimmed it.
//! - `struct tcp_opt_info`'s `ts_present` is a `bool`.
//! - `union syn_cache_sa` has the members and the size of `union sockaddr_union`; it is that
//!   type ([`SockaddrUnion`], a byte image with accessors, `netinet/ip_ipsp.rs`).
//! - `struct syn_cache`'s `sc_request_r_scale:4` and `sc_requested_s_scale:4` bit-fields are
//!   two `u8`s (the structure is kernel internal). `sc_buckethead` is a raw pointer into the
//!   set's bucket array, `sc_set` a reference to one of the two static sets.
//!   `struct syn_cache_set`'s `scs_buckethead` is the `malloc`ed bucket array as a slice, as
//!   `inpt_hashtbl` is in `in_pcb.rs`.
//! - N6.6a1: the constants (`TF_*`, `TCPOOB_*`, the SYN cache's sizes and `SCF_*`, the
//!   round-trip scales), `TCP_TIME`, `enum tcpstat_counters` and `tcpcounters` with
//!   `tcpstat_inc`/`tcpstat_add`/`tcpstat_pkt` are the kernel's (`kernel::netinet::tcp_var`,
//!   `tcpcounters` per CPU), re-exported here; the structures follow at N6.6a2.
//! - `TCP_REXMTVAL(tp)` is the function `tcp_rexmtval` (the C macro row of
//!   `docs/C_TO_RUST.md`); `intotcpcb`/`sototcpcb` return `Option`.
//! - `TCPCTL_NAMES` is for `sysctl(8)` and is not compiled; `struct tcp_ident_mapping` has
//!   its trailing padding as a named member (`AbiPod`).
//! - The `INET6` declarations (`tcp6_usrreqs`, `tcb6table`, `tcp6_ctlinput`,
//!   `tcp6_mtudisc_callback`) are the items of `tcp_usrreq.rs` and `tcp_subr.rs`, compiled
//!   always, as `netinet6` is. The `sin6` member of `union syn_cache_sa` is read and written
//!   through `tcp_input.rs`'s `sa_sin6`/`sa_from_sin6`. `TCP_ECN` and `TCP_SIGNATURE` are (GENERIC), so the `TF_ECN_*`
//!   flags and `tcp_signature` exist. `SMALL_KERNEL` is not set: `tcp_trace` is the function of
//!   `tcp_debug.rs`.

use core::cell::Cell;
use core::ptr;
use core::sync::atomic::Ordering;

use crate::kern::kern_tc::getnsecruntime;
use crate::net::route::RouteCell;
use crate::netinet::in_pcb::{Inpcb, sotoinpcb};
use crate::netinet::ip_ipsp::SockaddrUnion;
use crate::netinet::tcp::{MAX_SACK_BLKS, TcpSeq, Tcphdr};
use crate::netinet::tcp_subr::TCP_STARTTIME;
use crate::netinet::tcp_timer::TCPT_NTIMERS;
use crate::queue_adapter;
use crate::sys::errno::Errno;
use crate::sys::mbuf::Mbuf;
use crate::sys::queue::{ListEntry, ListHead, TailqEntry, TailqHead};
use crate::sys::refcnt::Refcnt;
use crate::sys::socketvar::Socket;
use crate::sys::timeout::Timeout;

pub use hal::sys::netinet_tcp_var::{
    TCPCTL_ACK_ON_PUSH, TCPCTL_ALWAYS_KEEPALIVE, TCPCTL_BADDYNAMIC, TCPCTL_DROP, TCPCTL_ECN,
    TCPCTL_IDENT, TCPCTL_KEEPIDLE, TCPCTL_KEEPINITTIME, TCPCTL_KEEPINTVL, TCPCTL_MAXID,
    TCPCTL_MSSDFLT, TCPCTL_REASS_LIMIT, TCPCTL_RECVSPACE, TCPCTL_RFC1323, TCPCTL_RFC3390,
    TCPCTL_ROOTONLY, TCPCTL_RSTPPSLIMIT, TCPCTL_SACK, TCPCTL_SACKHOLE_LIMIT, TCPCTL_SENDSPACE,
    TCPCTL_SLOWHZ, TCPCTL_STATS, TCPCTL_SYN_BUCKET_LIMIT, TCPCTL_SYN_CACHE_LIMIT,
    TCPCTL_SYN_HASH_SIZE, TCPCTL_SYN_USE_LIMIT, TCPCTL_TSO,
};
pub use kernel::netinet::tcp_var::{
    SCF_ECN_PERMIT, SCF_SACK_PERMIT, SCF_SIGNATURE, SCF_TIMESTAMP, SCF_UNREACH, TCP_RTT_BASE_SHIFT,
    TCP_RTT_MAX, TCP_RTT_SHIFT, TCP_RTTVAR_SHIFT, TCP_SYN_BUCKET_SIZE, TCP_SYN_HASH_SIZE,
    TCPCOUNTERS, TCPOOB_HADDATA, TCPOOB_HAVEDATA, TCPS_NCOUNTERS, TF_ACKNOW, TF_DISABLE_ECN,
    TF_ECN_PERMIT, TF_LASTIDLE, TF_NEEDOUTPUT, TF_NODELAY, TF_NOOPT, TF_NOPUSH, TF_PMTUD_PEND,
    TF_RCVD_CE, TF_RCVD_SCALE, TF_RCVD_TSTMP, TF_REQ_SCALE, TF_REQ_TSTMP, TF_SACK_PERMIT,
    TF_SEND_CWR, TF_SENTFIN, TF_SIGNATURE, TF_TIMER, TF_TMR_2MSL, TF_TMR_DELACK, TF_TMR_KEEP,
    TF_TMR_PERSIST, TF_TMR_REXMT, TcpstatCounters, tcp_time, tcpstat_add, tcpstat_inc, tcpstat_pkt,
};

/// `struct sackblk`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sackblk {
    /// Start seq no. of sack block.
    pub start: TcpSeq,
    /// End seq no.
    pub end: TcpSeq,
}

/// `struct sackhole`: a `sackhl_pool` item on the sender's sorted list of holes.
pub struct Sackhole {
    /// Start seq no. of hole.
    pub start: Cell<TcpSeq>,
    /// End seq no.
    pub end: Cell<TcpSeq>,
    /// Number of dup(s)acks for this hole.
    pub dups: Cell<i32>,
    /// Next seq. no in hole to be retransmitted.
    pub rxmit: Cell<TcpSeq>,
    /// Next in list.
    pub next: Cell<Option<&'static Sackhole>>,
}

impl Sackhole {
    /// A zeroed hole, as `pool_get(PR_ZERO)` would hand it out.
    pub const fn new() -> Self {
        Self {
            start: Cell::new(0),
            end: Cell::new(0),
            dups: Cell::new(0),
            rxmit: Cell::new(0),
            next: Cell::new(None),
        }
    }
}

impl Default for Sackhole {
    fn default() -> Self {
        Self::new()
    }
}

queue_adapter!(
    /// `TAILQ_ENTRY(tcpqent) tcpqe_q`.
    pub TcpqeQ: Tcpqent, tcpqe_q => TailqEntry<Tcpqent>
);

/// `TAILQ_HEAD(tcpqehead, tcpqent)`: the TCP sequence queue.
pub type Tcpqehead = TailqHead<TcpqeQ>;

/// `struct tcpqent`: a `tcpqe_pool` item holding an out-of-order segment.
pub struct Tcpqent {
    /// `tcpqe_q`.
    pub tcpqe_q: TailqEntry<Tcpqent>,
    /// `tcpqe_tcp`: the segment's TCP header, host order (see the deviations).
    pub tcpqe_tcp: Cell<Tcphdr>,
    /// Mbuf contains packet.
    pub tcpqe_m: Cell<Option<&'static Mbuf>>,
}

impl Tcpqent {
    /// An entry for `m` with header `th`.
    pub const fn new(th: Tcphdr, m: Option<&'static Mbuf>) -> Self {
        Self {
            tcpqe_q: TailqEntry::new(),
            tcpqe_tcp: Cell::new(th),
            tcpqe_m: Cell::new(m),
        }
    }
}

queue_adapter!(
    /// `LIST_ENTRY(syn_cache) sc_tpq`: list of entries by same tp.
    pub ScTpq: SynCache, sc_tpq => ListEntry<SynCache>
);

queue_adapter!(
    /// `TAILQ_ENTRY(syn_cache) sc_bucketq`: link on bucket list.
    pub ScBucketq: SynCache, sc_bucketq => TailqEntry<SynCache>
);

/// `struct tcpcb`: TCP control block, one per tcp. Every member is changed under the socket
/// lock of the connection.
pub struct Tcpcb {
    /// Sequencing queue.
    pub t_segq: Tcpqehead,
    /// Tcp timers.
    pub t_timer: [Timeout; TCPT_NTIMERS],
    /// State of this connection (`TCPS_*`).
    pub t_state: Cell<i32>,
    /// log(2) of rexmt exp. backoff.
    pub t_rxtshift: Cell<i16>,
    /// Current retransmit value.
    pub t_rxtcur: Cell<i32>,
    /// Consecutive dup acks recd.
    pub t_dupacks: Cell<i16>,
    /// Maximum segment size.
    pub t_maxseg: Cell<u16>,
    /// Forcing out a byte.
    pub t_force: Cell<bool>,
    /// `TF_*`.
    pub t_flags: Cell<u32>,
    /// Skeletal packet for transmit.
    pub t_template: Cell<Option<&'static Mbuf>>,
    /// Back pointer to internet pcb.
    pub t_inpcb: &'static Inpcb,

    // The following fields are used as in the protocol specification. See RFC793, Dec.
    // 1981, page 21.

    // send sequence variables
    /// Send unacknowledged.
    pub snd_una: Cell<TcpSeq>,
    /// Send next.
    pub snd_nxt: Cell<TcpSeq>,
    /// Send urgent pointer.
    pub snd_up: Cell<TcpSeq>,
    /// Window update seg seq number.
    pub snd_wl1: Cell<TcpSeq>,
    /// Window update seg ack number.
    pub snd_wl2: Cell<TcpSeq>,
    /// Initial send sequence number.
    pub iss: Cell<TcpSeq>,
    /// Send window.
    pub snd_wnd: Cell<u64>,
    /// Enable SACK for this connection.
    pub sack_enable: Cell<i32>,
    /// Number of holes seen by sender.
    pub snd_numholes: Cell<i32>,
    /// Linked list of holes (sorted).
    pub snd_holes: Cell<Option<&'static Sackhole>>,
    /// For use in fast recovery.
    pub snd_last: Cell<TcpSeq>,
    // receive sequence variables
    /// Receive window.
    pub rcv_wnd: Cell<u64>,
    /// Receive next.
    pub rcv_nxt: Cell<TcpSeq>,
    /// Receive urgent pointer.
    pub rcv_up: Cell<TcpSeq>,
    /// Initial receive sequence number.
    pub irs: Cell<TcpSeq>,
    /// Last seq number(+1) sack'd by rcv'r.
    pub rcv_lastsack: Cell<TcpSeq>,
    /// # distinct sack blks present.
    pub rcv_numsacks: Cell<i32>,
    /// Seq nos. of sack blocks.
    pub sackblks: [Cell<Sackblk>; MAX_SACK_BLKS],

    // Additional variables for this implementation.

    // receive variables
    /// Advertised window.
    pub rcv_adv: Cell<TcpSeq>,
    // retransmit variables
    /// Highest sequence number sent; used to recognize retransmits.
    pub snd_max: Cell<TcpSeq>,
    // congestion control (for slow start, source quench, retransmit after loss)
    /// Congestion-controlled window.
    pub snd_cwnd: Cell<u64>,
    /// snd_cwnd size threshold for for slow start exponential to linear switch.
    pub snd_ssthresh: Cell<u64>,

    // auto-sizing variables
    /// Recv buffer autoscaling time stamp.
    pub rfbuf_ts: Cell<u64>,
    /// Recv buffer autoscaling byte count.
    pub rfbuf_cnt: Cell<u32>,

    /// Mss plus options.
    pub t_maxopd: Cell<u16>,
    /// Peer's maximum segment size.
    pub t_peermss: Cell<u16>,

    // transmit timing stuff. See below for scale of srtt and rttvar. "Variance" is actually
    // smoothed difference.
    /// Time last segment received.
    pub t_rcvtime: Cell<u64>,
    /// Time last ack received.
    pub t_rcvacktime: Cell<u64>,
    /// Time last segment sent.
    pub t_sndtime: Cell<u64>,
    /// Time last ack sent.
    pub t_sndacktime: Cell<u64>,
    /// Time we started measuring rtt.
    pub t_rtttime: Cell<u64>,
    /// Sequence number being timed.
    pub t_rtseq: Cell<TcpSeq>,
    /// Smoothed round-trip time.
    pub t_srtt: Cell<i32>,
    /// Variance in round-trip time.
    pub t_rttvar: Cell<i32>,
    /// Minimum rtt allowed.
    pub t_rttmin: Cell<u32>,
    /// Largest window peer has offered.
    pub max_sndwnd: Cell<u64>,

    // out-of-band data
    /// Have some (`TCPOOB_*`).
    pub t_oobflags: Cell<u8>,
    /// Input character.
    pub t_iobc: Cell<u8>,
    /// Possible error not yet reported.
    pub t_softerror: Cell<Option<Errno>>,

    // RFC 1323 variables
    /// Window scaling for send window.
    pub snd_scale: Cell<u8>,
    /// Window scaling for recv window.
    pub rcv_scale: Cell<u8>,
    /// Pending window scaling.
    pub request_r_scale: Cell<u8>,
    /// `requested_s_scale`.
    pub requested_s_scale: Cell<u8>,
    /// Timestamp echo data.
    pub ts_recent: Cell<u32>,
    /// Modulation on timestamp.
    pub ts_modulate: Cell<u32>,
    /// When last updated.
    pub ts_recent_age: Cell<u64>,
    /// `last_ack_sent`.
    pub last_ack_sent: Cell<TcpSeq>,

    /// Pointer for syn cache entries: list of entries by this tcb.
    pub t_sc: ListHead<ScTpq>,

    // Path-MTU Discovery Information
    /// MSS acked, lower bound for MTU.
    pub t_pmtud_mss_acked: Cell<u32>,
    /// MTU used, upper bound for MTU.
    pub t_pmtud_mtu_sent: Cell<u32>,
    /// TCP SEQ from ICMP payload.
    pub t_pmtud_th_seq: Cell<TcpSeq>,
    /// Advertised Next-Hop MTU from ICMP (network order, as `icmp_nextmtu`).
    pub t_pmtud_nextmtu: Cell<u32>,
    /// IP length from ICMP payload (network order).
    pub t_pmtud_ip_len: Cell<u16>,
    /// IP header length from ICMP payload.
    pub t_pmtud_ip_hl: Cell<u16>,

    /// `pf`: the protocol family (`PF_INET`; 0 until attached).
    pub pf: Cell<i32>,

    // maintain a few stats per connection:
    /// Out-of-order packets received.
    pub t_rcvoopack: Cell<u32>,
    /// Retransmit packets sent.
    pub t_sndrexmitpack: Cell<u32>,
    /// Zero-window updates sent.
    pub t_sndzerowin: Cell<u32>,
}

impl Tcpcb {
    /// A zeroed control block of `inp`, as `pool_get(PR_ZERO)` and the `t_inpcb` assignment
    /// of `tcp_newtcpcb` leave it.
    pub const fn new(inp: &'static Inpcb) -> Self {
        Self {
            t_segq: TailqHead::new(),
            t_timer: [const { Timeout::zeroed() }; TCPT_NTIMERS],
            t_state: Cell::new(0),
            t_rxtshift: Cell::new(0),
            t_rxtcur: Cell::new(0),
            t_dupacks: Cell::new(0),
            t_maxseg: Cell::new(0),
            t_force: Cell::new(false),
            t_flags: Cell::new(0),
            t_template: Cell::new(None),
            t_inpcb: inp,
            snd_una: Cell::new(0),
            snd_nxt: Cell::new(0),
            snd_up: Cell::new(0),
            snd_wl1: Cell::new(0),
            snd_wl2: Cell::new(0),
            iss: Cell::new(0),
            snd_wnd: Cell::new(0),
            sack_enable: Cell::new(0),
            snd_numholes: Cell::new(0),
            snd_holes: Cell::new(None),
            snd_last: Cell::new(0),
            rcv_wnd: Cell::new(0),
            rcv_nxt: Cell::new(0),
            rcv_up: Cell::new(0),
            irs: Cell::new(0),
            rcv_lastsack: Cell::new(0),
            rcv_numsacks: Cell::new(0),
            sackblks: [const { Cell::new(Sackblk { start: 0, end: 0 }) }; MAX_SACK_BLKS],
            rcv_adv: Cell::new(0),
            snd_max: Cell::new(0),
            snd_cwnd: Cell::new(0),
            snd_ssthresh: Cell::new(0),
            rfbuf_ts: Cell::new(0),
            rfbuf_cnt: Cell::new(0),
            t_maxopd: Cell::new(0),
            t_peermss: Cell::new(0),
            t_rcvtime: Cell::new(0),
            t_rcvacktime: Cell::new(0),
            t_sndtime: Cell::new(0),
            t_sndacktime: Cell::new(0),
            t_rtttime: Cell::new(0),
            t_rtseq: Cell::new(0),
            t_srtt: Cell::new(0),
            t_rttvar: Cell::new(0),
            t_rttmin: Cell::new(0),
            max_sndwnd: Cell::new(0),
            t_oobflags: Cell::new(0),
            t_iobc: Cell::new(0),
            t_softerror: Cell::new(None),
            snd_scale: Cell::new(0),
            rcv_scale: Cell::new(0),
            request_r_scale: Cell::new(0),
            requested_s_scale: Cell::new(0),
            ts_recent: Cell::new(0),
            ts_modulate: Cell::new(0),
            ts_recent_age: Cell::new(0),
            last_ack_sent: Cell::new(0),
            t_sc: ListHead::new(),
            t_pmtud_mss_acked: Cell::new(0),
            t_pmtud_mtu_sent: Cell::new(0),
            t_pmtud_th_seq: Cell::new(0),
            t_pmtud_nextmtu: Cell::new(0),
            t_pmtud_ip_len: Cell::new(0),
            t_pmtud_ip_hl: Cell::new(0),
            pf: Cell::new(0),
            t_rcvoopack: Cell::new(0),
            t_sndrexmitpack: Cell::new(0),
            t_sndzerowin: Cell::new(0),
        }
    }

    /// `ISSET(tp->t_flags, bits)`: any of `bits`.
    pub fn has_flags(&self, bits: u32) -> bool {
        self.t_flags.get() & bits != 0
    }

    /// `tp->t_flags |= bits`.
    pub fn set_flags(&self, bits: u32) {
        self.t_flags.set(self.t_flags.get() | bits);
    }

    /// `tp->t_flags &= ~bits`.
    pub fn clear_flags(&self, bits: u32) {
        self.t_flags.set(self.t_flags.get() & !bits);
    }

    /// `tp->t_inpcb->inp_socket`.
    pub fn socket(&self) -> &'static Socket {
        self.t_inpcb.socket()
    }
}

/// `struct tcp_opt_info`: handy way of passing around TCP option info.
#[derive(Clone, Copy, Debug, Default)]
pub struct TcpOptInfo {
    /// `ts_present`.
    pub ts_present: bool,
    /// `ts_val`.
    pub ts_val: u32,
    /// `ts_ecr`.
    pub ts_ecr: u32,
    /// `maxseg`.
    pub maxseg: u16,
}

/// `union syn_cache_sa`: a `struct sockaddr`, `sockaddr_in` or `sockaddr_in6` (see the
/// deviations).
pub type SynCacheSa = SockaddrUnion;

/// `struct syn_cache`: an embryonic connection, a `syn_cache_pool` item.
pub struct SynCache {
    /// \[S\] Link on bucket list.
    pub sc_bucketq: TailqEntry<SynCache>,
    /// Ref count list and timer.
    pub sc_refcnt: Refcnt,
    /// Rexmt timer.
    pub sc_timer: Timeout,
    /// \[s\] Cached route.
    pub sc_route: RouteCell,
    /// \[I\] Advertised window.
    pub sc_win: Cell<i64>,
    /// \[S\] Our bucket index.
    pub sc_buckethead: Cell<*const SynCacheHead>,
    /// \[S\] Our syn cache set.
    pub sc_set: Cell<Option<&'static SynCacheSet>>,
    /// \[s\] Timestamp from SYN.
    pub sc_timestamp: Cell<u64>,
    /// \[S\] `sc_hash`.
    pub sc_hash: Cell<u32>,
    /// \[I\] Our timestamp modulator.
    pub sc_modulate: Cell<u32>,
    /// \[I\] `sc_src`.
    pub sc_src: Cell<SynCacheSa>,
    /// \[I\] `sc_dst`.
    pub sc_dst: Cell<SynCacheSa>,
    /// \[I\] `sc_irs`.
    pub sc_irs: Cell<TcpSeq>,
    /// \[I\] `sc_iss`.
    pub sc_iss: Cell<TcpSeq>,
    /// \[I\] `sc_rtableid`.
    pub sc_rtableid: Cell<u32>,
    /// \[S\] Current rxt timeout.
    pub sc_rxtcur: Cell<u32>,
    /// \[S\] Total time spend on queues.
    pub sc_rxttot: Cell<u32>,
    /// \[S\] For computing backoff.
    pub sc_rxtshift: Cell<u32>,
    /// \[S\] Flags accessed with mutex (`SCF_UNREACH`).
    pub sc_dynflags: Cell<u32>,
    /// \[I\] Set during initialization (`SCF_TIMESTAMP`, ...).
    pub sc_fixflags: Cell<u16>,
    /// \[s\] IP options.
    pub sc_ipopts: Cell<Option<&'static Mbuf>>,
    /// \[I\] `sc_peermaxseg`.
    pub sc_peermaxseg: Cell<u16>,
    /// \[I\] `sc_ourmaxseg`.
    pub sc_ourmaxseg: Cell<u16>,
    /// \[I\] `sc_request_r_scale:4`.
    pub sc_request_r_scale: Cell<u8>,
    /// \[I\] `sc_requested_s_scale:4`.
    pub sc_requested_s_scale: Cell<u8>,
    /// \[S\] Inpcb for listening socket.
    pub sc_inplisten: Cell<Option<&'static Inpcb>>,
    /// \[S\] List of entries by same tp.
    pub sc_tpq: ListEntry<SynCache>,
}

impl SynCache {
    /// A zeroed entry, as `pool_get(PR_ZERO)` hands it out.
    pub const fn new() -> Self {
        Self {
            sc_bucketq: TailqEntry::new(),
            sc_refcnt: Refcnt::new(),
            sc_timer: Timeout::zeroed(),
            sc_route: RouteCell::new(),
            sc_win: Cell::new(0),
            sc_buckethead: Cell::new(ptr::null()),
            sc_set: Cell::new(None),
            sc_timestamp: Cell::new(0),
            sc_hash: Cell::new(0),
            sc_modulate: Cell::new(0),
            sc_src: Cell::new(SockaddrUnion::new()),
            sc_dst: Cell::new(SockaddrUnion::new()),
            sc_irs: Cell::new(0),
            sc_iss: Cell::new(0),
            sc_rtableid: Cell::new(0),
            sc_rxtcur: Cell::new(0),
            sc_rxttot: Cell::new(0),
            sc_rxtshift: Cell::new(0),
            sc_dynflags: Cell::new(0),
            sc_fixflags: Cell::new(0),
            sc_ipopts: Cell::new(None),
            sc_peermaxseg: Cell::new(0),
            sc_ourmaxseg: Cell::new(0),
            sc_request_r_scale: Cell::new(0),
            sc_requested_s_scale: Cell::new(0),
            sc_inplisten: Cell::new(None),
            sc_tpq: ListEntry::new(),
        }
    }
}

impl Default for SynCache {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct syn_cache_head`: one bucket of a set.
pub struct SynCacheHead {
    /// \[S\] Bucket entries.
    pub sch_bucket: TailqHead<ScBucketq>,
    /// \[S\] # entries in bucket.
    pub sch_length: Cell<u16>,
}

impl SynCacheHead {
    /// An empty bucket.
    pub const fn new() -> Self {
        Self {
            sch_bucket: TailqHead::new(),
            sch_length: Cell::new(0),
        }
    }
}

impl Default for SynCacheHead {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct syn_cache_set`: a hash table of embryonic connections; there are two, the
/// active one and the one being drained (`tcp_syn_cache[2]`).
pub struct SynCacheSet {
    /// \[S\] The bucket array (`malloc`ed, `scs_size` entries).
    pub scs_buckethead: Cell<&'static [SynCacheHead]>,
    /// \[S\] Uses left before the set is reseeded.
    pub scs_use: Cell<i64>,
    /// \[S\] Current size of hash table.
    pub scs_size: Cell<i32>,
    /// \[S\] `scs_count`.
    pub scs_count: Cell<i32>,
    /// \[S\] Hash seed.
    pub scs_random: Cell<[u32; 5]>,
}

impl SynCacheSet {
    /// An empty set (the static initialiser).
    pub const fn new() -> Self {
        Self {
            scs_buckethead: Cell::new(&[]),
            scs_use: Cell::new(0),
            scs_size: Cell::new(0),
            scs_count: Cell::new(0),
            scs_random: Cell::new([0; 5]),
        }
    }
}

impl Default for SynCacheSet {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: every member is read and written only with `syn_cache_mtx` held (\[S\]), which
// serialises all access to the two static sets.
unsafe impl Sync for SynCacheSet {}

pub use hal::sys::netinet_tcp_var::{TcpIdentMapping, Tcpstat};

/// `tcp_now`: TCP time ticks in 63 bit milliseconds with 63 bit random offset.
pub fn tcp_now() -> u64 {
    TCP_STARTTIME
        .load(Ordering::Relaxed)
        .wrapping_add(getnsecruntime() / 1_000_000)
}

/// `TCP_REXMTVAL(tp)`: the initial retransmission should happen at rtt + 4 * rttvar.
/// Because of the way we do the smoothing, srtt and rttvar will each average +1/2 tick of
/// bias. When we compute the retransmit timer, we want 1/2 tick of rounding and 1 extra tick
/// because of +-1/2 tick uncertainty in the firing of the timer. The bias will give us
/// exactly the 1.5 tick we need. But, because the bias is statistical, we have to test that
/// we don't drop below the minimum feasible timer (which is 2 ticks). This assumes that the
/// value of `1 << TCP_RTTVAR_SHIFT` is the same as the multiplier for rttvar.
pub fn tcp_rexmtval(tp: &Tcpcb) -> i32 {
    ((tp.t_srtt.get() >> TCP_RTT_SHIFT) + tp.t_rttvar.get()) >> TCP_RTT_BASE_SHIFT
}

/// `intotcpcb(inp)`: the TCP control block of an internet control block, `None` before
/// `tcp_newtcpcb` and after `tcp_close`.
pub fn intotcpcb(inp: &Inpcb) -> Option<&'static Tcpcb> {
    // SAFETY: a `tcbtable` control block's `inp_ppcb` is NULL or the `tcpcb_pool` item
    // `tcp_newtcpcb` stored there; `tcp_close` clears it before the item goes back to the
    // pool.
    unsafe { inp.inp_ppcb.get().cast::<Tcpcb>().as_ref() }
}

/// `sototcpcb(so)`.
pub fn sototcpcb(so: &Socket) -> Option<&'static Tcpcb> {
    intotcpcb(sotoinpcb(so)?)
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;

    use super::*;

    #[test]
    fn rexmtval_matches_the_macro() {
        // srtt 8 << 3 (one second scaled), rttvar 4: ((64 >> 3) + 4) >> 2 = 3.
        let inp: &'static Inpcb = Box::leak(Box::new(Inpcb::new(None, None)));
        let tp = Tcpcb::new(inp);
        tp.t_srtt.set(64);
        tp.t_rttvar.set(4);
        assert_eq!(tcp_rexmtval(&tp), 3);
        assert_eq!(tcp_time(30), 30_000);
    }
}
/* </TESTS> */
