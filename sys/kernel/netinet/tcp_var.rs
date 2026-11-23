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
//! LZ: sys/netinet/tcp_var.rs@3c62ede68802
//!
//! The TCP control block ([`Tcpcb`], one per connection, owned by the internet control
//! block's slot as `inp_ppcb`), the reassembly queue entries, the SACK blocks and holes, the
//! SYN cache, the statistics with their per-counter enumeration, and the constants of the
//! header's kernel part. The header's userland part (`TCPCTL_*`, `struct tcpstat`, `struct
//! tcp_ident_mapping`) is `hal::sys::netinet_tcp_var`'s.
//!
//! Locks used to protect global data and struct members: \[I\] immutable after creation,
//! \[N\] net lock, \[S\] `syn_cache_mtx`, \[s\] `so_lock` of the socket (of the listen socket
//! for a SYN cache entry), \[T\] the timeout wheel's lock. The C's `struct tcpcb` carries no
//! legend: every member is the connection's socket lock's (`soassertlocked`).
//!
//! ## Deviations
//! - `TCP_TIME(sec)` is the `const fn` [`tcp_time`] and `TCP_REXMTVAL(tp)` the function
//!   [`tcp_rexmtval`] (the C macro row of `docs/C_TO_RUST.md`); `intotcpcb` returns an
//!   `Option` (the C's NULL before `tcp_newtcpcb` and after `tcp_close`).
//! - `tcpstat_inc`, `tcpstat_add` and `tcpstat_pkt` take no `splnet()` around the update
//!   (the C's inline functions do): `Counters::enter`'s generation update is a load then a
//!   store, so an `IPL_NET` interrupt that counts on the same CPU between the two loses no
//!   increment (each counter is its own atomic add) but can leave the generation even while
//!   the interrupted update is unfinished, and a `net.inet.tcp.stats` reader may then accept
//!   that CPU's copy with the interrupted update half done, a snapshot the C's `splnet` rules
//!   out. The counters stay exact; only one read's coherence across counters is weaker.
//! - `t_state` is an `i32` (a `short` in C) so that it compares with the `TCPS_*` constants
//!   without casts; `t_force` is a `bool` (a `char` used as a flag); `t_softerror` is an
//!   `Option<Errno>` (a `short` holding an errno or 0); `struct tcp_opt_info`'s `ts_present`
//!   is a `bool`. `struct syn_cache`'s `sc_request_r_scale:4` and `sc_requested_s_scale:4`
//!   bit-fields are two `u8`s (the structure is kernel internal).
//! - `struct tcpqent`'s `tcpqe_tcp`, the C's pointer to the segment's TCP header inside the
//!   mbuf, is a copy of the header, host order as `tcp_input` leaves it: only its sequence
//!   number, the length kept in `th_reseqlen` and the flags are read or changed, and the mbuf
//!   data no longer covers the header once `tcp_input` has trimmed it (LZ's choice).
//! - `union syn_cache_sa` is its bytes ([`SynCacheSa`], the 28 of `struct sockaddr_in6`, the
//!   largest member): the members are read by family through views of the bytes.
//! - The `INET6` declarations are compiled always, as `netinet6` is. `TCP_ECN` and
//!   `TCP_SIGNATURE` are configured (GENERIC), so the `TF_ECN_*` flags exist.
//!
//! ## Redesign
//! - N6.6a1 (D-16): `tcpcounters` (the C's `struct cpumem *` from `counters_alloc`) is
//!   [`TCPCOUNTERS`], a `hal::percpu::Counters` here beside the enum that indexes it, where LZ
//!   had a static array of shared atomics in `bsd`'s `tcp_subr.rs` that every softnet wrote
//!   (one cache line per counter group for all CPUs). [`tcpstat_inc`], [`tcpstat_add`] and
//!   [`tcpstat_pkt`] are the C's `counters_inc`/`counters_add`/`counters_pkt` on this CPU's
//!   copy (the generation number odd during the update, as the C's `counters_enter`);
//!   `net.inet.tcp.stats` sums them with `Counters::read`, the same words `struct tcpstat`
//!   always had. The cost is `MAXCPUS` copies (about 224 KiB of `.bss` for 107 counters,
//!   where LZ's array was 856 bytes), the shape of `ipcounters` and `rtcounters`
//!   (docs/ROADMAP.md's N12 row).
//! - N6.6a2 (D-12): LZ's `struct tcpcb` was a pool item of `Cell`s reached through
//!   `inp_ppcb`'s raw word by an `unsafe` cast (`intotcpcb`), with a `&'static Inpcb` back
//!   pointer, `t_template` and the queue entries' mbufs parked as `&'static Mbuf`s, the
//!   reassembly queue a `TailqHead` of pool items and the SACK holes `Cell<Option<&'static
//!   Sackhole>>` links. Here the control block is a "tcpcb" `BoxPool` item ([`Tcpcb`]) that
//!   the pcb's slot owns (`Inpcb::inp_tcpcb`, keyed to the socket's `so_lock` in part `PCB`):
//!   its timers and its SYN cache list stand apart (the wheel's lock and the SYN cache's
//!   mutex guard them), every other member is a plain field of [`TcpState`], one
//!   `hal::rwlock::ExclGuarded` cell keyed to `so_lock` in part `TCP` (an mbuf chain is `Send`
//!   but not `Sync`, so the cell opens only to an exclusive witness, D-15's cell (c)).
//!   [`intotcpcb`] opens both through the socket lock's parts and answers a [`Tp`]: the pcb,
//!   its socket, the pinned control block, its state mutably, the SYN part and the net lock's
//!   proof, the C's `struct tcpcb *` for every function of TCP. `t_inpcb` goes (the `Tp`
//!   carries the pcb); the back pointer was the C's way to the socket and the pcb, which the
//!   caller already holds.
//! - The reassembly queue and the SACK holes are bare owned chains
//!   (`hal::owned::SlistLink`, the link owning the next entry's box, D-13): `tcpqe_q` and
//!   `next` own the following entry; the entries are "tcpqe" and "sackhl" `BoxPool` items.
//!   The C walks both forward only, inserting and removing where it stands.
//! - The SYN cache entry ([`SynCache`]) is a counted "syncache" item (`PoolArc`, the C's
//!   `sc_refcnt`): its `[I]` members are plain fields set when it is made, its `[S]` members
//!   one `Guarded` cell keyed to the SYN cache's mutex ([`ScState`]: the set's and the
//!   bucket's index replace `sc_set` and `sc_buckethead`, the listener a counted handle), its
//!   `[s]` members one `ExclGuarded` cell keyed to the listener's `so_lock` in part `SYN`
//!   ([`ScListener`]: the route and the IP options' chain). The C's `[s]` `sc_timestamp` is
//!   an `[a]` member here, a relaxed `AtomicU64` beside the cells (N6.6a4's review fixes):
//!   a duplicate SYN to an entry of another listener stores it and `syn_cache_get` reads it
//!   for every child without that listener's lock, as the C does (a race in the C, an
//!   atomic word here, the IDIOMS row of a scalar read without the lock). The two sets
//!   ([`SynCaches`]) are the data of the glue's `Mutexed` (`syn_cache_mtx`, `IPL_SOFTNET`),
//!   their bucket arrays `MArray`s of pinned lists that leave the guard for a call and come
//!   back (the pcb table's shape).
//! - The glue's TCP globals (the pools, the SYN cache, `tcbtable` and `tcb6table`) are one
//!   [`TcpTables`] in its `NetTables`: a control block is generic over the glue, as the pcb.
//! - N6.6a3's diet (the user's of 2026-11-16, D-7 brought forward from N12): the SYN cache
//!   entry's set and bucket are a `u8` and a `u32` index (the set is 0 or 1, the bucket below
//!   the `int` `scs_size`), the `[S]` cell 64 -> 56 bytes, the "syncache" item 464 -> 456
//!   (the C's 336). The control block stays 1000 (the C's 736): its state is 344 bytes with 3
//!   of padding (rustc already orders the members), and what it carries beyond the C is
//!   `hal`'s and the timeout's: the five timeouts' 48 bytes each over the C's (the pinned
//!   entry's lock identity, the `[T]` cell's key, the shared argument beside the raw one), the
//!   `PinRing`'s 32 over a `LIST_HEAD` and the state cell's 16-byte key (docs/ROADMAP.md's N12
//!   row).

use core::pin::Pin;
use core::sync::atomic::{AtomicI32, AtomicU64};

use hal::intr::IPL_SOFTNET;
use hal::malloc::MArray;
use hal::mutex::Mutexed;
use hal::netlock::NetHeld;
use hal::owned::SlistLink;
use hal::percpu::Counters;
use hal::pinlist::{PinEntry, PinList, PinRing};
use hal::pool::{BoxPool, PoolHome, Pooled, TypedPool};
use hal::rwlock::{ExclGuarded, Guarded, LockId, WitnessPart};
use hal::sys::errno::Errno;
use hal::sys::netinet_tcp::{MAX_SACK_BLKS, TcpSeq, Tcphdr};

use crate::assert::kpanic;
use crate::net::ipsec_hooks::SOCKADDR_UNION_LEN;
use crate::net::mbuf::MbufBox;
use crate::net::route::Route;
use crate::net::socket::{InetLocked, Socket, so_part};
use crate::netinet::in_pcb::{Inpcb, InpcbRef, Inpcbtable};
use crate::netinet::tcp_subr::{SACKHL_POOL, TCP_STARTTIME, TCPQE_POOL};
use crate::netinet::tcp_timer::TCPT_NTIMERS;
use crate::proc::ProcGlue;
use crate::timeout::Timeout;

// t_flags.

/// Ack peer immediately.
pub const TF_ACKNOW: u32 = 0x0001;
/// Don't delay packets to coalesce.
pub const TF_NODELAY: u32 = 0x0004;
/// Don't use tcp options.
pub const TF_NOOPT: u32 = 0x0008;
/// Have sent FIN.
pub const TF_SENTFIN: u32 = 0x0010;
/// Have/will request window scaling.
pub const TF_REQ_SCALE: u32 = 0x0020;
/// Other side has requested scaling.
pub const TF_RCVD_SCALE: u32 = 0x0040;
/// Have/will request timestamps.
pub const TF_REQ_TSTMP: u32 = 0x0080;
/// A timestamp was received in SYN.
pub const TF_RCVD_TSTMP: u32 = 0x0100;
/// Other side said I could SACK.
pub const TF_SACK_PERMIT: u32 = 0x0200;
/// Require TCP MD5 signature.
pub const TF_SIGNATURE: u32 = 0x0400;
/// Other side said I could ECN (`TCP_ECN`).
pub const TF_ECN_PERMIT: u32 = 0x0000_8000;
/// Send ECE in subsequent segs (`TCP_ECN`).
pub const TF_RCVD_CE: u32 = 0x0001_0000;
/// Send CWR in next seg (`TCP_ECN`).
pub const TF_SEND_CWR: u32 = 0x0002_0000;
/// Disable ECN for this connection (`TCP_ECN`).
pub const TF_DISABLE_ECN: u32 = 0x0004_0000;
/// No outstanding ACK on last send.
pub const TF_LASTIDLE: u32 = 0x0010_0000;
/// Path MTU Discovery pending.
pub const TF_PMTUD_PEND: u32 = 0x0040_0000;
/// Call tcp_output after tcp_input.
pub const TF_NEEDOUTPUT: u32 = 0x0080_0000;
/// Don't push.
pub const TF_NOPUSH: u32 = 0x0200_0000;
/// Retransmit timer armed.
pub const TF_TMR_REXMT: u32 = 0x0400_0000;
/// Retransmit persistence timer armed.
pub const TF_TMR_PERSIST: u32 = 0x0800_0000;
/// Keep alive timer armed.
pub const TF_TMR_KEEP: u32 = 0x1000_0000;
/// 2*msl quiet time timer armed.
pub const TF_TMR_2MSL: u32 = 0x2000_0000;
/// Delayed ack timer armed.
pub const TF_TMR_DELACK: u32 = 0x4000_0000;
/// Used to shift with TCPT values.
pub const TF_TIMER: u32 = TF_TMR_REXMT;

// t_oobflags.

/// `TCPOOB_HAVEDATA`.
pub const TCPOOB_HAVEDATA: u8 = 0x01;
/// `TCPOOB_HADDATA`.
pub const TCPOOB_HADDATA: u8 = 0x02;

/// `TCP_SYN_HASH_SIZE`.
pub const TCP_SYN_HASH_SIZE: i32 = 293;
/// `TCP_SYN_BUCKET_SIZE`.
pub const TCP_SYN_BUCKET_SIZE: i32 = 35;

/// We've had an unreach error (`sc_dynflags`).
pub const SCF_UNREACH: u32 = 0x0001;
/// Peer will do timestamps (`sc_fixflags`).
pub const SCF_TIMESTAMP: u16 = 0x0010;
/// Permit sack.
pub const SCF_SACK_PERMIT: u16 = 0x0020;
/// Permit ecn.
pub const SCF_ECN_PERMIT: u16 = 0x0040;
/// Enforce tcp signatures.
pub const SCF_SIGNATURE: u16 = 0x0080;

// The smoothed round-trip time and estimated variance are stored as fixed point numbers
// scaled by the values below. For convenience, these scales are also used in smoothing the
// average (smoothed = (1/scale)sample + ((scale-1)/scale)smoothed). With these scales, srtt
// has 5 bits to the right of the binary point, and thus an "ALPHA" of 0.875. rttvar has 4
// bits to the right of the binary point, and is smoothed with an ALPHA of 0.75.

/// Shift for srtt; 5 bits frac.
pub const TCP_RTT_SHIFT: i32 = 3;
/// Shift for rttvar; 4 bits.
pub const TCP_RTTVAR_SHIFT: i32 = 2;
/// Remaining 2 bit shift.
pub const TCP_RTT_BASE_SHIFT: i32 = 2;
/// Maximum rtt.
pub const TCP_RTT_MAX: i32 = 1 << 18;

/// `struct sackblk`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sackblk {
    /// Start seq no. of sack block.
    pub start: TcpSeq,
    /// End seq no.
    pub end: TcpSeq,
}

/// `struct sackhole`: a "sackhl" item on the sender's sorted list of holes, which owns it.
#[derive(Default)]
pub struct Sackhole {
    /// Start seq no. of hole.
    pub start: TcpSeq,
    /// End seq no.
    pub end: TcpSeq,
    /// Number of dup(s)acks for this hole.
    pub dups: i32,
    /// Next seq. no in hole to be retransmitted.
    pub rxmit: TcpSeq,
    /// Next in list (owned).
    pub next: SlistLink<Sackhole>,
}

impl Sackhole {
    /// A hole of `[start, end)`, linked to nothing.
    pub const fn new(start: TcpSeq, end: TcpSeq, dups: i32, rxmit: TcpSeq) -> Self {
        Self {
            start,
            end,
            dups,
            rxmit,
            next: SlistLink::new(),
        }
    }
}

impl Pooled for Sackhole {
    const POOL: &'static BoxPool<Self> = &SACKHL_POOL;
}

/// `struct tcpqent`: a "tcpqe" item holding an out-of-order segment, on the reassembly queue
/// that owns it.
pub struct Tcpqent {
    /// `tcpqe_q`: the next entry (owned).
    pub tcpqe_q: SlistLink<Tcpqent>,
    /// `tcpqe_tcp`: the segment's TCP header, host order (see the deviations).
    pub tcpqe_tcp: Tcphdr,
    /// Mbuf contains packet (`Some` while the entry is queued; taken when the segment is
    /// presented).
    pub tcpqe_m: Option<MbufBox>,
}

impl Pooled for Tcpqent {
    const POOL: &'static BoxPool<Self> = &TCPQE_POOL;
}

/// The members of `struct tcpcb` under the connection's socket lock (all but the timers and
/// the SYN cache list): `so_lock`'s part `TCP` opens them ([`intotcpcb`]).
pub struct TcpState {
    /// Sequencing queue: the first entry (owned; each entry owns the next).
    pub t_segq: SlistLink<Tcpqent>,
    /// State of this connection (`TCPS_*`).
    pub t_state: i32,
    /// log(2) of rexmt exp. backoff.
    pub t_rxtshift: i16,
    /// Current retransmit value.
    pub t_rxtcur: i32,
    /// Consecutive dup acks recd.
    pub t_dupacks: i16,
    /// Maximum segment size.
    pub t_maxseg: u16,
    /// Forcing out a byte.
    pub t_force: bool,
    /// `TF_*`.
    pub t_flags: u32,
    /// Skeletal packet for transmit.
    pub t_template: Option<MbufBox>,

    // The following fields are used as in the protocol specification. See RFC793, Dec.
    // 1981, page 21.

    // send sequence variables
    /// Send unacknowledged.
    pub snd_una: TcpSeq,
    /// Send next.
    pub snd_nxt: TcpSeq,
    /// Send urgent pointer.
    pub snd_up: TcpSeq,
    /// Window update seg seq number.
    pub snd_wl1: TcpSeq,
    /// Window update seg ack number.
    pub snd_wl2: TcpSeq,
    /// Initial send sequence number.
    pub iss: TcpSeq,
    /// Send window.
    pub snd_wnd: u64,
    /// Enable SACK for this connection.
    pub sack_enable: i32,
    /// Number of holes seen by sender.
    pub snd_numholes: i32,
    /// Linked list of holes (sorted): the first hole (owned; each owns the next).
    pub snd_holes: SlistLink<Sackhole>,
    /// For use in fast recovery.
    pub snd_last: TcpSeq,
    // receive sequence variables
    /// Receive window.
    pub rcv_wnd: u64,
    /// Receive next.
    pub rcv_nxt: TcpSeq,
    /// Receive urgent pointer.
    pub rcv_up: TcpSeq,
    /// Initial receive sequence number.
    pub irs: TcpSeq,
    /// Last seq number(+1) sack'd by rcv'r.
    pub rcv_lastsack: TcpSeq,
    /// # distinct sack blks present.
    pub rcv_numsacks: i32,
    /// Seq nos. of sack blocks.
    pub sackblks: [Sackblk; MAX_SACK_BLKS],

    // Additional variables for this implementation.

    // receive variables
    /// Advertised window.
    pub rcv_adv: TcpSeq,
    // retransmit variables
    /// Highest sequence number sent; used to recognize retransmits.
    pub snd_max: TcpSeq,
    // congestion control (for slow start, source quench, retransmit after loss)
    /// Congestion-controlled window.
    pub snd_cwnd: u64,
    /// snd_cwnd size threshold for for slow start exponential to linear switch.
    pub snd_ssthresh: u64,

    // auto-sizing variables
    /// Recv buffer autoscaling time stamp.
    pub rfbuf_ts: u64,
    /// Recv buffer autoscaling byte count.
    pub rfbuf_cnt: u32,

    /// Mss plus options.
    pub t_maxopd: u16,
    /// Peer's maximum segment size.
    pub t_peermss: u16,

    // transmit timing stuff. See below for scale of srtt and rttvar. "Variance" is actually
    // smoothed difference.
    /// Time last segment received.
    pub t_rcvtime: u64,
    /// Time last ack received.
    pub t_rcvacktime: u64,
    /// Time last segment sent.
    pub t_sndtime: u64,
    /// Time last ack sent.
    pub t_sndacktime: u64,
    /// Time we started measuring rtt.
    pub t_rtttime: u64,
    /// Sequence number being timed.
    pub t_rtseq: TcpSeq,
    /// Smoothed round-trip time.
    pub t_srtt: i32,
    /// Variance in round-trip time.
    pub t_rttvar: i32,
    /// Minimum rtt allowed.
    pub t_rttmin: u32,
    /// Largest window peer has offered.
    pub max_sndwnd: u64,

    // out-of-band data
    /// Have some (`TCPOOB_*`).
    pub t_oobflags: u8,
    /// Input character.
    pub t_iobc: u8,
    /// Possible error not yet reported.
    pub t_softerror: Option<Errno>,

    // RFC 1323 variables
    /// Window scaling for send window.
    pub snd_scale: u8,
    /// Window scaling for recv window.
    pub rcv_scale: u8,
    /// Pending window scaling.
    pub request_r_scale: u8,
    /// `requested_s_scale`.
    pub requested_s_scale: u8,
    /// Timestamp echo data.
    pub ts_recent: u32,
    /// Modulation on timestamp.
    pub ts_modulate: u32,
    /// When last updated.
    pub ts_recent_age: u64,
    /// `last_ack_sent`.
    pub last_ack_sent: TcpSeq,

    // Path-MTU Discovery Information
    /// MSS acked, lower bound for MTU.
    pub t_pmtud_mss_acked: u32,
    /// MTU used, upper bound for MTU.
    pub t_pmtud_mtu_sent: u32,
    /// TCP SEQ from ICMP payload.
    pub t_pmtud_th_seq: TcpSeq,
    /// Advertised Next-Hop MTU from ICMP (network order, as `icmp_nextmtu`).
    pub t_pmtud_nextmtu: u32,
    /// IP length from ICMP payload (network order).
    pub t_pmtud_ip_len: u16,
    /// IP header length from ICMP payload.
    pub t_pmtud_ip_hl: u16,

    /// `pf`: the protocol family (`PF_INET`; 0 until attached).
    pub pf: i32,

    // maintain a few stats per connection:
    /// Out-of-order packets received.
    pub t_rcvoopack: u32,
    /// Retransmit packets sent.
    pub t_sndrexmitpack: u32,
    /// Zero-window updates sent.
    pub t_sndzerowin: u32,
}

impl TcpState {
    /// The zeroed members, as `pool_get(PR_ZERO)` leaves them.
    pub const fn new() -> Self {
        Self {
            t_segq: SlistLink::new(),
            t_state: 0,
            t_rxtshift: 0,
            t_rxtcur: 0,
            t_dupacks: 0,
            t_maxseg: 0,
            t_force: false,
            t_flags: 0,
            t_template: None,
            snd_una: 0,
            snd_nxt: 0,
            snd_up: 0,
            snd_wl1: 0,
            snd_wl2: 0,
            iss: 0,
            snd_wnd: 0,
            sack_enable: 0,
            snd_numholes: 0,
            snd_holes: SlistLink::new(),
            snd_last: 0,
            rcv_wnd: 0,
            rcv_nxt: 0,
            rcv_up: 0,
            irs: 0,
            rcv_lastsack: 0,
            rcv_numsacks: 0,
            sackblks: [Sackblk { start: 0, end: 0 }; MAX_SACK_BLKS],
            rcv_adv: 0,
            snd_max: 0,
            snd_cwnd: 0,
            snd_ssthresh: 0,
            rfbuf_ts: 0,
            rfbuf_cnt: 0,
            t_maxopd: 0,
            t_peermss: 0,
            t_rcvtime: 0,
            t_rcvacktime: 0,
            t_sndtime: 0,
            t_sndacktime: 0,
            t_rtttime: 0,
            t_rtseq: 0,
            t_srtt: 0,
            t_rttvar: 0,
            t_rttmin: 0,
            max_sndwnd: 0,
            t_oobflags: 0,
            t_iobc: 0,
            t_softerror: None,
            snd_scale: 0,
            rcv_scale: 0,
            request_r_scale: 0,
            requested_s_scale: 0,
            ts_recent: 0,
            ts_modulate: 0,
            ts_recent_age: 0,
            last_ack_sent: 0,
            t_pmtud_mss_acked: 0,
            t_pmtud_mtu_sent: 0,
            t_pmtud_th_seq: 0,
            t_pmtud_nextmtu: 0,
            t_pmtud_ip_len: 0,
            t_pmtud_ip_hl: 0,
            pf: 0,
            t_rcvoopack: 0,
            t_sndrexmitpack: 0,
            t_sndzerowin: 0,
        }
    }

    /// `ISSET(tp->t_flags, bits)`: any of `bits`.
    #[inline]
    pub fn has_flags(&self, bits: u32) -> bool {
        self.t_flags & bits != 0
    }

    /// `tp->t_flags |= bits`.
    #[inline]
    pub fn set_flags(&mut self, bits: u32) {
        self.t_flags |= bits;
    }

    /// `tp->t_flags &= ~bits`.
    #[inline]
    pub fn clear_flags(&mut self, bits: u32) {
        self.t_flags &= !bits;
    }
}

impl Default for TcpState {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct tcpcb`: TCP control block, one per tcp; a "tcpcb" item the pcb's slot owns,
/// pinned in place for its timers (see the module's Redesign).
pub struct Tcpcb<G: ProcGlue> {
    /// \[T\] `t_timer`: tcp timers, each holding a counted handle of the pcb from
    /// `tcp_timer_init` to the control block's free (D-11).
    pub t_timer: [Timeout; TCPT_NTIMERS],
    /// \[S\] `t_sc`: list of SYN cache entries by this tcb (a ring: an entry leaves it without
    /// naming the control block, as the C's `LIST_REMOVE`).
    pub t_sc: PinRing<ScTpq<G>>,
    /// \[s\] Every other member, keyed to the socket's `so_lock` in part `TCP`.
    pub st: ExclGuarded<TcpState>,
}

impl<G: ProcGlue> Tcpcb<G> {
    /// A zeroed control block of the socket whose `so_lock` is `so_lock`, as `pool_get(PR_ZERO)`
    /// leaves it: its timers not yet set (`tcp_timer_init`).
    pub fn new(so_lock: LockId) -> Self {
        Self {
            t_timer: [const { Timeout::unset() }; TCPT_NTIMERS],
            t_sc: PinRing::new(),
            st: ExclGuarded::keyed(so_lock, so_part::TCP, TcpState::new()),
        }
    }

    /// `&tp->t_timer[timer]`, pinned with the control block; `panic(9)` past `TCPT_NTIMERS`.
    pub fn timer(self: Pin<&Self>, timer: usize) -> Pin<&Timeout> {
        match timer {
            0 => hal::pin::project(self, |t| &t.t_timer[0]),
            1 => hal::pin::project(self, |t| &t.t_timer[1]),
            2 => hal::pin::project(self, |t| &t.t_timer[2]),
            3 => hal::pin::project(self, |t| &t.t_timer[3]),
            4 => hal::pin::project(self, |t| &t.t_timer[4]),
            _ => kpanic(format_args!("tcp timer {timer} of {TCPT_NTIMERS}")),
        }
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

/// `union syn_cache_sa`: a `struct sockaddr`, `sockaddr_in` or `sockaddr_in6`, as its bytes
/// (see the deviations).
pub type SynCacheSa = [u8; SOCKADDR_UNION_LEN];

/// The `[I]` members of a SYN cache entry, set once when it is made ([`SynCache::new`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct SynCacheInit {
    /// \[I\] Advertised window.
    pub sc_win: i64,
    /// \[I\] Our timestamp modulator.
    pub sc_modulate: u32,
    /// \[I\] `sc_src`.
    pub sc_src: SynCacheSa,
    /// \[I\] `sc_dst`.
    pub sc_dst: SynCacheSa,
    /// \[I\] `sc_irs`.
    pub sc_irs: TcpSeq,
    /// \[I\] `sc_iss`.
    pub sc_iss: TcpSeq,
    /// \[I\] `sc_rtableid`.
    pub sc_rtableid: u32,
    /// \[I\] Set during initialization (`SCF_TIMESTAMP`, ...).
    pub sc_fixflags: u16,
    /// \[I\] `sc_peermaxseg`.
    pub sc_peermaxseg: u16,
    /// \[I\] `sc_ourmaxseg`.
    pub sc_ourmaxseg: u16,
    /// \[I\] `sc_request_r_scale:4`.
    pub sc_request_r_scale: u8,
    /// \[I\] `sc_requested_s_scale:4`.
    pub sc_requested_s_scale: u8,
}

/// The `[S]` members of a SYN cache entry, `syn_cache_mtx`'s.
pub struct ScState<G: ProcGlue> {
    /// \[S\] Current rxt timeout.
    pub sc_rxtcur: u32,
    /// \[S\] Total time spend on queues.
    pub sc_rxttot: u32,
    /// \[S\] For computing backoff.
    pub sc_rxtshift: u32,
    /// \[S\] Flags accessed with mutex (`SCF_UNREACH`).
    pub sc_dynflags: u32,
    /// \[S\] `sc_hash`.
    pub sc_hash: u32,
    /// \[S\] `sc_set`: our syn cache set, by its index in the two sets (0 or 1).
    pub sc_set: u8,
    /// \[S\] `sc_buckethead`: our bucket, by its index in the set's array.
    pub sc_bucket: u32,
    /// \[S\] Inpcb for listening socket, counted (the C's `in_pcbref` kept by the entry).
    pub sc_inplisten: Option<InpcbRef<G>>,
}

/// The `[s]` members of a SYN cache entry, the listen socket's `so_lock`'s.
#[derive(Default)]
pub struct ScListener {
    /// \[s\] Cached route.
    pub sc_route: Route,
    /// \[s\] IP options.
    pub sc_ipopts: Option<MbufBox>,
}

/// `struct syn_cache`: an embryonic connection, a counted "syncache" item.
pub struct SynCache<G: ProcGlue> {
    /// The item's place in the "syncache" pool (`refcnt_take` of an entry a list lends).
    sc_home: PoolHome<Self>,
    /// \[S\] Link on bucket list.
    sc_bucketq: PinEntry<Self>,
    /// \[S\] List of entries by same tp.
    sc_tpq: PinEntry<Self>,
    /// Rexmt timer: it holds a counted handle of the entry while the entry is in the cache.
    pub sc_timer: Timeout,
    /// \[S\] The members `syn_cache_mtx` guards.
    pub sc_s: Guarded<ScState<G>>,
    /// \[s\] The members the listen socket's lock guards.
    pub sc_l: ExclGuarded<ScListener>,
    /// \[a\] Timestamp from SYN (the C's `[s]` `sc_timestamp`): stored by `syn_cache_add`
    /// before the entry is shared and by a duplicate SYN, read by the SYN-ACK and by
    /// `syn_cache_get`, all `Relaxed` (one word, no other memory published with it); a
    /// duplicate SYN to an entry of another listener writes it without that listener's lock,
    /// as the C does.
    pub sc_timestamp: AtomicU64,
    /// \[I\] The members set when the entry is made.
    pub sc_i: SynCacheInit,
}

hal::pool_home!(impl[G: ProcGlue] SynCache<G>, sc_home);

hal::pin_adapter!(
    /// `TAILQ_HEAD(, syn_cache) sch_bucket`, through `sc_bucketq`.
    impl[G: ProcGlue] pub ScBucketq<G>: SynCache<G>, sc_bucketq
);

hal::pin_adapter!(
    /// `LIST_HEAD(, syn_cache) t_sc` (a ring), through `sc_tpq`.
    impl[G: ProcGlue] pub ScTpq<G>: SynCache<G>, sc_tpq
);

impl<G: ProcGlue> SynCache<G> {
    /// A new entry with the `[I]` members `init` (`pool_get(PR_ZERO)` and `syn_cache_add`'s
    /// stores before the entry is shared): its `[S]` cell keyed to `syn_cache_mtx` (`mtx`),
    /// holding the listener's handle (`sc_inplisten = in_pcbref(tp->t_inpcb)`), its `[s]` cell
    /// to the listen socket's `so_lock` (`listen`) in part `SYN`, its timestamp 0
    /// (`syn_cache_add` stores the SYN's before the entry is shared), its timer not set
    /// (`syn_cache_timer_set` sets it).
    pub fn new(
        mtx: LockId,
        listen: LockId,
        init: SynCacheInit,
        l: ScListener,
        inplisten: Option<InpcbRef<G>>,
    ) -> Self {
        Self {
            sc_home: PoolHome::new(),
            sc_bucketq: PinEntry::new(),
            sc_tpq: PinEntry::new(),
            sc_timer: Timeout::unset(),
            sc_s: Guarded::keyed(
                mtx,
                0,
                ScState {
                    sc_rxtcur: 0,
                    sc_rxttot: 0,
                    sc_rxtshift: 0,
                    sc_dynflags: 0,
                    sc_hash: 0,
                    sc_set: 0,
                    sc_bucket: 0,
                    sc_inplisten: inplisten,
                },
            ),
            sc_l: ExclGuarded::keyed(listen, so_part::SYN, l),
            sc_timestamp: AtomicU64::new(0),
            sc_i: init,
        }
    }
}

/// `struct syn_cache_head`: one bucket of a set.
pub struct SynCacheHead<G: ProcGlue> {
    /// \[S\] Bucket entries.
    pub sch_bucket: PinList<ScBucketq<G>>,
    /// \[S\] # entries in bucket.
    pub sch_length: u16,
}

impl<G: ProcGlue> SynCacheHead<G> {
    /// An empty bucket.
    pub const fn new() -> Self {
        Self {
            sch_bucket: PinList::new(),
            sch_length: 0,
        }
    }
}

impl<G: ProcGlue> Default for SynCacheHead<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct syn_cache_set`: a hash table of embryonic connections; there are two, the active
/// one and the one being drained ([`SynCaches`]).
pub struct SynCacheSet<G: ProcGlue> {
    /// \[S\] `scs_buckethead`: the bucket array (`M_SYNCACHE`, `scs_size` buckets); `None`
    /// before `syn_cache_init` and while a call holds it out of the guard.
    pub scs_buckethead: Option<MArray<SynCacheHead<G>>>,
    /// \[S\] Uses left before the set is reseeded.
    pub scs_use: i64,
    /// \[S\] Current size of hash table.
    pub scs_size: i32,
    /// \[S\] `scs_count`.
    pub scs_count: i32,
    /// \[S\] Hash seed.
    pub scs_random: [u32; 5],
}

impl<G: ProcGlue> SynCacheSet<G> {
    /// An empty set (the static initialiser).
    pub const fn new() -> Self {
        Self {
            scs_buckethead: None,
            scs_use: 0,
            scs_size: 0,
            scs_count: 0,
            scs_random: [0; 5],
        }
    }
}

impl<G: ProcGlue> Default for SynCacheSet<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// What `syn_cache_mtx` keeps (\[S\]): `tcp_syn_cache[2]` and `tcp_syn_cache_active` (the
/// two `[S]` tunables of `tcp_input.c` are [`TcpTables`]' atomics, written under it).
pub struct SynCaches<G: ProcGlue> {
    /// `tcp_syn_cache[2]`.
    pub sets: [SynCacheSet<G>; 2],
    /// `tcp_syn_cache_active`: the index of the active set.
    pub active: usize,
}

impl<G: ProcGlue> SynCaches<G> {
    /// The globals before `syn_cache_init`.
    pub const fn new() -> Self {
        Self {
            sets: [SynCacheSet::new(), SynCacheSet::new()],
            active: 0,
        }
    }
}

impl<G: ProcGlue> Default for SynCaches<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The TCP globals of a glue, in its `NetTables`: `tcp_subr.c`'s `tcpcb_pool`, `tcp_input.c`'s
/// `syn_cache_pool` and `syn_cache_mtx` with what it guards, `tcp_usrreq.c`'s `tcbtable` and
/// `tcb6table`. The tcpqe and sackhl pools hold no glue type: they are statics
/// (`tcp_subr.rs`).
pub struct TcpTables<G: ProcGlue> {
    /// `tcpcb_pool`.
    pub(crate) tcpcb_pool: BoxPool<Tcpcb<G>>,
    /// `syn_cache_pool`.
    pub(crate) syn_cache_pool: TypedPool<SynCache<G>>,
    /// `syn_cache_mtx`: tcp syn cache global mutex, and the `[S]` globals.
    pub(crate) syn_cache: Mutexed<SynCaches<G>>,
    /// \[S\] `tcp_syn_hash_size`: size of hash table. Written under `syn_cache_mtx` (`[S]`)
    /// and read without it (`tcp_sysctl`'s `atomic_load_int`, `syn_cache_init`), as the C.
    pub tcp_syn_hash_size: AtomicI32,
    /// \[S\] `tcp_syn_use_limit`: reseed after uses; as `tcp_syn_hash_size`.
    pub tcp_syn_use_limit: AtomicI32,
    /// `tcbtable`.
    pub tcbtable: Inpcbtable<G>,
    /// `tcb6table`.
    pub tcb6table: Inpcbtable<G>,
}

impl<G: ProcGlue> TcpTables<G> {
    /// The tables before `tcp_init`.
    pub const fn new() -> Self {
        Self {
            tcpcb_pool: BoxPool::new(),
            syn_cache_pool: TypedPool::new(),
            syn_cache: Mutexed::new(IPL_SOFTNET, SynCaches::new()),
            tcp_syn_hash_size: AtomicI32::new(TCP_SYN_HASH_SIZE),
            tcp_syn_use_limit: AtomicI32::new(100_000),
            tcbtable: Inpcbtable::new(),
            tcb6table: Inpcbtable::new(),
        }
    }
}

impl<G: ProcGlue> Default for TcpTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// A connection's control block held for a TCP function: the C's `struct tcpcb *tp` with what
/// `tp->t_inpcb` reached, made by [`intotcpcb`] from the socket lock's parts. A function that
/// may free the control block takes it by value (`tcp_close` answers `Closed`), so a use after
/// the close does not compile.
pub struct Tp<'a, 'w, G: ProcGlue> {
    /// `tp->t_inpcb`.
    pub inp: &'a Inpcb<G>,
    /// `tp->t_inpcb->inp_socket`.
    pub so: &'a Socket<G>,
    /// The control block, pinned (its timers, its SYN cache list).
    pub cb: Pin<&'a Tcpcb<G>>,
    /// Its members, opened by the socket lock's part `TCP`.
    pub st: &'a mut TcpState,
    /// The socket lock's part `SYN`: the `[s]` members of the SYN cache entries of this
    /// listener.
    pub syn: &'a mut WitnessPart<'w>,
    /// The proof of the net lock the socket lock holds.
    pub net: &'a NetHeld<'w>,
}

/// `enum tcpstat_counters`: the index of each counter in `tcpcounters`, in the order of the
/// members of [`Tcpstat`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum TcpstatCounters {
    /// `tcps_connattempt`.
    TcpsConnattempt,
    /// `tcps_accepts`.
    TcpsAccepts,
    /// `tcps_connects`.
    TcpsConnects,
    /// `tcps_drops`.
    TcpsDrops,
    /// `tcps_conndrops`.
    TcpsConndrops,
    /// `tcps_closed`.
    TcpsClosed,
    /// `tcps_segstimed`.
    TcpsSegstimed,
    /// `tcps_rttupdated`.
    TcpsRttupdated,
    /// `tcps_delack`.
    TcpsDelack,
    /// `tcps_timeoutdrop`.
    TcpsTimeoutdrop,
    /// `tcps_rexmttimeo`.
    TcpsRexmttimeo,
    /// `tcps_persisttimeo`.
    TcpsPersisttimeo,
    /// `tcps_persistdrop`.
    TcpsPersistdrop,
    /// `tcps_keeptimeo`.
    TcpsKeeptimeo,
    /// `tcps_keepprobe`.
    TcpsKeepprobe,
    /// `tcps_keepdrops`.
    TcpsKeepdrops,
    /// `tcps_sndtotal`.
    TcpsSndtotal,
    /// `tcps_sndpack`.
    TcpsSndpack,
    /// `tcps_sndbyte`.
    TcpsSndbyte,
    /// `tcps_sndrexmitpack`.
    TcpsSndrexmitpack,
    /// `tcps_sndrexmitbyte`.
    TcpsSndrexmitbyte,
    /// `tcps_sndrexmitfast`.
    TcpsSndrexmitfast,
    /// `tcps_sndacks`.
    TcpsSndacks,
    /// `tcps_sndprobe`.
    TcpsSndprobe,
    /// `tcps_sndurg`.
    TcpsSndurg,
    /// `tcps_sndwinup`.
    TcpsSndwinup,
    /// `tcps_sndctrl`.
    TcpsSndctrl,
    /// `tcps_rcvtotal`.
    TcpsRcvtotal,
    /// `tcps_rcvpack`.
    TcpsRcvpack,
    /// `tcps_rcvbyte`.
    TcpsRcvbyte,
    /// `tcps_rcvbadsum`.
    TcpsRcvbadsum,
    /// `tcps_rcvbadoff`.
    TcpsRcvbadoff,
    /// `tcps_rcvmemdrop`.
    TcpsRcvmemdrop,
    /// `tcps_rcvnosec`.
    TcpsRcvnosec,
    /// `tcps_rcvshort`.
    TcpsRcvshort,
    /// `tcps_rcvduppack`.
    TcpsRcvduppack,
    /// `tcps_rcvdupbyte`.
    TcpsRcvdupbyte,
    /// `tcps_rcvpartduppack`.
    TcpsRcvpartduppack,
    /// `tcps_rcvpartdupbyte`.
    TcpsRcvpartdupbyte,
    /// `tcps_rcvoopack`.
    TcpsRcvoopack,
    /// `tcps_rcvoobyte`.
    TcpsRcvoobyte,
    /// `tcps_rcvpackafterwin`.
    TcpsRcvpackafterwin,
    /// `tcps_rcvbyteafterwin`.
    TcpsRcvbyteafterwin,
    /// `tcps_rcvafterclose`.
    TcpsRcvafterclose,
    /// `tcps_rcvwinprobe`.
    TcpsRcvwinprobe,
    /// `tcps_rcvdupack`.
    TcpsRcvdupack,
    /// `tcps_rcvacktoomuch`.
    TcpsRcvacktoomuch,
    /// `tcps_rcvacktooold`.
    TcpsRcvacktooold,
    /// `tcps_rcvackpack`.
    TcpsRcvackpack,
    /// `tcps_rcvackbyte`.
    TcpsRcvackbyte,
    /// `tcps_rcvwinupd`.
    TcpsRcvwinupd,
    /// `tcps_pawsdrop`.
    TcpsPawsdrop,
    /// `tcps_predack`.
    TcpsPredack,
    /// `tcps_preddat`.
    TcpsPreddat,
    /// `tcps_pcbhashmiss`.
    TcpsPcbhashmiss,
    /// `tcps_noport`.
    TcpsNoport,
    /// `tcps_closing`.
    TcpsClosing,
    /// `tcps_badsyn`.
    TcpsBadsyn,
    /// `tcps_dropsyn`.
    TcpsDropsyn,
    /// `tcps_rcvbadsig`.
    TcpsRcvbadsig,
    /// `tcps_rcvgoodsig`.
    TcpsRcvgoodsig,
    /// `tcps_inswcsum`.
    TcpsInswcsum,
    /// `tcps_outswcsum`.
    TcpsOutswcsum,
    /// `tcps_ecn_accepts`.
    TcpsEcnAccepts,
    /// `tcps_ecn_rcvece`.
    TcpsEcnRcvece,
    /// `tcps_ecn_rcvcwr`.
    TcpsEcnRcvcwr,
    /// `tcps_ecn_rcvce`.
    TcpsEcnRcvce,
    /// `tcps_ecn_sndect`.
    TcpsEcnSndect,
    /// `tcps_ecn_sndece`.
    TcpsEcnSndece,
    /// `tcps_ecn_sndcwr`.
    TcpsEcnSndcwr,
    /// `tcps_cwr_ecn`.
    TcpsCwrEcn,
    /// `tcps_cwr_frecovery`.
    TcpsCwrFrecovery,
    /// `tcps_cwr_timeout`.
    TcpsCwrTimeout,
    /// `tcps_sc_added`.
    TcpsScAdded,
    /// `tcps_sc_completed`.
    TcpsScCompleted,
    /// `tcps_sc_timed_out`.
    TcpsScTimedOut,
    /// `tcps_sc_overflowed`.
    TcpsScOverflowed,
    /// `tcps_sc_reset`.
    TcpsScReset,
    /// `tcps_sc_unreach`.
    TcpsScUnreach,
    /// `tcps_sc_bucketoverflow`.
    TcpsScBucketoverflow,
    /// `tcps_sc_aborted`.
    TcpsScAborted,
    /// `tcps_sc_dupesyn`.
    TcpsScDupesyn,
    /// `tcps_sc_dropped`.
    TcpsScDropped,
    /// `tcps_sc_collisions`.
    TcpsScCollisions,
    /// `tcps_sc_retransmitted`.
    TcpsScRetransmitted,
    /// `tcps_sc_seedrandom`.
    TcpsScSeedrandom,
    /// `tcps_sc_hash_size`.
    TcpsScHashSize,
    /// `tcps_sc_entry_count`.
    TcpsScEntryCount,
    /// `tcps_sc_entry_limit`.
    TcpsScEntryLimit,
    /// `tcps_sc_bucket_maxlen`.
    TcpsScBucketMaxlen,
    /// `tcps_sc_bucket_limit`.
    TcpsScBucketLimit,
    /// `tcps_sc_uses_left`.
    TcpsScUsesLeft,
    /// `tcps_conndrained`.
    TcpsConndrained,
    /// `tcps_sack_recovery_episode`.
    TcpsSackRecoveryEpisode,
    /// `tcps_sack_rexmits`.
    TcpsSackRexmits,
    /// `tcps_sack_rexmit_bytes`.
    TcpsSackRexmitBytes,
    /// `tcps_sack_rcv_opts`.
    TcpsSackRcvOpts,
    /// `tcps_sack_snd_opts`.
    TcpsSackSndOpts,
    /// `tcps_sack_drop_opts`.
    TcpsSackDropOpts,
    /// `tcps_outswtso`.
    TcpsOutswtso,
    /// `tcps_outhwtso`.
    TcpsOuthwtso,
    /// `tcps_outpkttso`.
    TcpsOutpkttso,
    /// `tcps_outbadtso`.
    TcpsOutbadtso,
    /// `tcps_inswlro`.
    TcpsInswlro,
    /// `tcps_inhwlro`.
    TcpsInhwlro,
    /// `tcps_inpktlro`.
    TcpsInpktlro,
    /// `tcps_inbadlro`.
    TcpsInbadlro,
    /// `tcps_ncounters`.
    TcpsNcounters,
}

/// `tcps_ncounters`.
pub const TCPS_NCOUNTERS: usize = TcpstatCounters::TcpsNcounters as usize;

/// \[a\] `tcpcounters`: TCP statistics, one copy per CPU (`counters_alloc(tcps_ncounters)`).
pub static TCPCOUNTERS: Counters<TCPS_NCOUNTERS> = Counters::new();

/// `tcpstat_inc(name)`: `counters_inc(tcpcounters, name)`.
#[inline]
pub fn tcpstat_inc(c: TcpstatCounters) {
    TCPCOUNTERS.inc(c as usize);
}

/// `tcpstat_add(name, val)`: `counters_add(tcpcounters, name, val)`.
#[inline]
pub fn tcpstat_add(c: TcpstatCounters, v: u64) {
    TCPCOUNTERS.add(c as usize, v);
}

/// `tcpstat_pkt(p, b, v)`: one more packet in `pcounter`, `v` more bytes in `bcounter`
/// (`counters_pkt`), in one update of this CPU's copy.
#[inline]
pub fn tcpstat_pkt(pcounter: TcpstatCounters, bcounter: TcpstatCounters, v: u64) {
    TCPCOUNTERS.pkt(pcounter as usize, bcounter as usize, v);
}

/// `tcp_now`: TCP time ticks in 63 bit milliseconds with 63 bit random offset.
#[inline]
pub fn tcp_now() -> u64 {
    TCP_STARTTIME
        .load(core::sync::atomic::Ordering::Relaxed)
        .wrapping_add(crate::tc::getnsecruntime() / 1_000_000)
}

/// `TCP_TIME(sec)`: `tcp_now()` is in milliseconds.
pub const fn tcp_time(sec: i32) -> i32 {
    sec * 1000
}

/// `TCP_REXMTVAL(tp)`: the initial retransmission should happen at rtt + 4 * rttvar.
/// Because of the way we do the smoothing, srtt and rttvar will each average +1/2 tick of
/// bias. When we compute the retransmit timer, we want 1/2 tick of rounding and 1 extra tick
/// because of +-1/2 tick uncertainty in the firing of the timer. The bias will give us
/// exactly the 1.5 tick we need. But, because the bias is statistical, we have to test that
/// we don't drop below the minimum feasible timer (which is 2 ticks). This assumes that the
/// value of `1 << TCP_RTTVAR_SHIFT` is the same as the multiplier for rttvar.
#[inline]
pub fn tcp_rexmtval(st: &TcpState) -> i32 {
    ((st.t_srtt >> TCP_RTT_SHIFT) + st.t_rttvar) >> TCP_RTT_BASE_SHIFT
}

/// `intotcpcb(inp)`: the TCP control block of `inp`, opened through its socket's lock in
/// parts (`il`, the lock of `inp`'s socket: the slot opens with part `PCB`, the members with
/// part `TCP`; another lock's parts are `panic(9)`); `None` before `tcp_newtcpcb` and after
/// `tcp_close` (the C's NULL). No count is taken: the slot keeps the control block while the
/// lock is held, as the C's `inp_ppcb` read.
pub fn intotcpcb<'a, 'w, G: ProcGlue>(
    inp: &'a Inpcb<G>,
    il: &'a mut InetLocked<'w>,
) -> Option<Tp<'a, 'w, G>> {
    let InetLocked {
        pcb, tcp, syn, net, ..
    } = il;
    let pcb: &'a WitnessPart<'w> = pcb;
    let net: &'a NetHeld<'w> = net;
    let cb = inp.tcpcb(pcb)?;
    let st = cb.get_ref().st.get_mut(tcp);
    Some(Tp {
        inp,
        so: inp.socket(),
        cb,
        st,
        syn,
        net,
    })
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_cover_the_structure() {
        // Every member of struct tcpstat has a counter; the last one is tcps_inbadlro.
        assert_eq!(TCPS_NCOUNTERS, 107);
        assert_eq!(TcpstatCounters::TcpsInbadlro as usize, 106);
        assert_eq!(TcpstatCounters::TcpsConnattempt as usize, 0);
        assert_eq!(tcp_time(30), 30_000);
    }

    #[test]
    fn the_counters_add_up() {
        // Other tests count too: compare differences, on counters no other test bumps.
        let before = TCPCOUNTERS.read();
        tcpstat_inc(TcpstatCounters::TcpsScSeedrandom);
        tcpstat_add(TcpstatCounters::TcpsScSeedrandom, 4);
        tcpstat_pkt(
            TcpstatCounters::TcpsConndrained,
            TcpstatCounters::TcpsSackDropOpts,
            1500,
        );
        let after = TCPCOUNTERS.read();
        let d = |c: TcpstatCounters| after[c as usize] - before[c as usize];
        assert_eq!(d(TcpstatCounters::TcpsScSeedrandom), 5);
        assert_eq!(d(TcpstatCounters::TcpsConndrained), 1);
        assert_eq!(d(TcpstatCounters::TcpsSackDropOpts), 1500);
    }

    #[test]
    fn rexmtval_and_flags_as_the_macros() {
        // srtt 8 << 3 (one second scaled), rttvar 4: ((64 >> 3) + 4) >> 2 = 3.
        let mut st = TcpState::new();
        st.t_srtt = 64;
        st.t_rttvar = 4;
        assert_eq!(tcp_rexmtval(&st), 3);
        st.set_flags(TF_ACKNOW | TF_NODELAY);
        assert!(st.has_flags(TF_NODELAY));
        st.clear_flags(TF_ACKNOW);
        assert!(!st.has_flags(TF_ACKNOW));
        assert!(st.has_flags(TF_ACKNOW | TF_NODELAY));
        assert_eq!(
            TF_TIMER << crate::netinet::tcp_timer::TCPT_DELACK,
            TF_TMR_DELACK
        );
    }

    /// The pool items' sizes against the C's structures on LP64 (D-7: `vmstat -m` shows
    /// them under the C's pool names; the commit and the hand-off record the columns).
    #[test]
    fn the_pool_items_against_the_c() {
        use crate::fs::testfs::TestGlue as G;
        use core::mem::size_of;

        #[allow(dead_code, reason = "a layout replica: only its size is read")]
        #[repr(C)]
        struct CTimeout {
            to_list: [usize; 2],
            to_abstime: [i64; 2],
            to_func: usize,
            to_arg: usize,
            to_process: usize,
            to_time: i32,
            to_flags: i32,
            to_kclock: i32,
        }
        #[allow(dead_code, reason = "a layout replica: only its size is read")]
        #[repr(C)]
        struct CTcpcb {
            t_segq: [usize; 2],
            t_timer: [CTimeout; TCPT_NTIMERS],
            t_state: i16,
            t_rxtshift: i16,
            t_rxtcur: i32,
            t_dupacks: i16,
            t_maxseg: u16,
            t_force: u8,
            t_flags: u32,
            t_template: usize,
            t_inpcb: usize,
            snd: [u32; 6],
            snd_wnd: u64,
            sack_enable: i32,
            snd_numholes: i32,
            snd_holes: usize,
            snd_last: u32,
            rcv_wnd: u64,
            rcv: [u32; 4],
            rcv_numsacks: i32,
            sackblks: [u32; 2 * MAX_SACK_BLKS],
            rcv_adv: u32,
            snd_max: u32,
            snd_cwnd: u64,
            snd_ssthresh: u64,
            rfbuf_ts: u64,
            rfbuf_cnt: u32,
            t_maxopd: u16,
            t_peermss: u16,
            times: [u64; 5],
            t_rtseq: u32,
            t_srtt: i32,
            t_rttvar: i32,
            t_rttmin: u32,
            max_sndwnd: u64,
            t_oobflags: u8,
            t_iobc: u8,
            t_softerror: i16,
            scales: [u8; 4],
            ts_recent: u32,
            ts_modulate: u32,
            ts_recent_age: u64,
            last_ack_sent: u32,
            t_sc: usize,
            pmtud: [u32; 4],
            pmtud_ip: [u16; 2],
            pf: i32,
            stats: [u32; 3],
        }
        #[allow(dead_code, reason = "a layout replica: only its size is read")]
        #[repr(C)]
        struct CSynCache {
            sc_bucketq: [usize; 2],
            sc_refcnt: [u32; 2],
            sc_timer: CTimeout,
            sc_route: [u64; 9],
            sc_win: i64,
            sc_buckethead: usize,
            sc_set: usize,
            sc_timestamp: u64,
            sc_hash: u32,
            sc_modulate: u32,
            sc_src: [u32; 7],
            sc_dst: [u32; 7],
            sc_irs: u32,
            sc_iss: u32,
            sc_rtableid: u32,
            sc_rxt: [u32; 3],
            sc_dynflags: u32,
            sc_fixflags: u16,
            sc_ipopts: usize,
            sc_maxseg: [u16; 2],
            sc_scales: u32,
            sc_inplisten: usize,
            sc_tpq: [usize; 2],
        }
        let rows = [
            ("tcpcb", size_of::<Tcpcb<G>>(), size_of::<CTcpcb>()),
            ("  TcpState", size_of::<TcpState>(), 0),
            ("  Timeout", size_of::<Timeout>(), size_of::<CTimeout>()),
            ("tcpqe", size_of::<Tcpqent>(), 4 * 8),
            ("sackhl", size_of::<Sackhole>(), 4 * 4 + 8),
            ("syncache", size_of::<SynCache<G>>(), size_of::<CSynCache>()),
            ("  ScState", size_of::<Guarded<ScState<G>>>(), 0),
            ("  ScListener", size_of::<ExclGuarded<ScListener>>(), 0),
        ];
        for (n, ours, c) in rows {
            std::println!("{n}: {ours} (C {c})");
        }
        assert_eq!(size_of::<CTimeout>(), 72);
        assert_eq!(size_of::<CTcpcb>(), 736);
        assert_eq!(size_of::<CSynCache>(), 336);
        // The items, against the C's 736, 32, 24 and 336: "tcpcb" 1000 (the five kernel
        // timeouts are 120 bytes each, the C's 72: the pinned entry's lock identity, the
        // `[T]` cell's key and the shared argument, +240; the state's cell key +16; the
        // state itself is 344, 3 bytes of padding, 24 below the C's members it holds: no
        // reorder or narrower member of the kernel's own brings it under 336),
        // "tcpqe" 48 (the mbuf's box is two words, the header a copy, LZ's choice), "sackhl"
        // 24 as the C, "syncache" 456 (440 and the counted item's 16-byte header: the
        // timeout +48, the two pinned entries' identities, the two cells' keys; N6.6a3's
        // diet: the set's and the bucket's indices a `u8` and a `u32`, the `[S]` cell 64 ->
        // 56). What is left is `hal`'s and the timeout's (docs/ROADMAP.md's N12 row).
        assert_eq!(size_of::<Tcpcb<G>>(), 1000);
        assert_eq!(size_of::<TcpState>(), 344);
        assert_eq!(size_of::<Tcpqent>(), 48);
        assert_eq!(size_of::<Sackhole>(), 24);
        assert_eq!(size_of::<SynCache<G>>(), 440);
        assert_eq!(size_of::<Guarded<ScState<G>>>(), 56);
    }

    #[test]
    fn the_pools_report_the_items_vmstat_shows() {
        // `vmstat -m`'s size column of TCP's four pools (`pr_size`): a box's item is its
        // value, a counted item ("syncache") carries the count and its pool beside it.
        use crate::netinet::tcp_subr::{SACKHL_POOL, TCPQE_POOL, testtcp};
        let (_g, _p) = testtcp::setup();
        let t = &crate::net::domain::net::<testtcp::G>().tcp;
        assert_eq!(t.tcpcb_pool.item_size(), Some(1000), "tcpcb");
        assert_eq!(t.syn_cache_pool.item_size(), Some(456), "syncache");
        assert_eq!(TCPQE_POOL.item_size(), Some(48), "tcpqe");
        assert_eq!(SACKHL_POOL.item_size(), Some(24), "sackhl");
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/tcp_var.h");
        crate::reftest::assert_defines!(defs;
            TCPOOB_HAVEDATA, TCPOOB_HADDATA, TCP_SYN_HASH_SIZE, TCP_SYN_BUCKET_SIZE,
            TCP_RTT_SHIFT, TCP_RTTVAR_SHIFT, TCP_RTT_BASE_SHIFT, TCP_RTT_MAX);
        // The flags carry the C's `U` suffix, which the evaluator does not read: compare
        // the hexadecimal digits.
        for (name, v) in [
            ("TF_ACKNOW", TF_ACKNOW),
            ("TF_NODELAY", TF_NODELAY),
            ("TF_NOOPT", TF_NOOPT),
            ("TF_SENTFIN", TF_SENTFIN),
            ("TF_REQ_SCALE", TF_REQ_SCALE),
            ("TF_RCVD_SCALE", TF_RCVD_SCALE),
            ("TF_REQ_TSTMP", TF_REQ_TSTMP),
            ("TF_RCVD_TSTMP", TF_RCVD_TSTMP),
            ("TF_SACK_PERMIT", TF_SACK_PERMIT),
            ("TF_SIGNATURE", TF_SIGNATURE),
            ("TF_ECN_PERMIT", TF_ECN_PERMIT),
            ("TF_RCVD_CE", TF_RCVD_CE),
            ("TF_SEND_CWR", TF_SEND_CWR),
            ("TF_DISABLE_ECN", TF_DISABLE_ECN),
            ("TF_LASTIDLE", TF_LASTIDLE),
            ("TF_PMTUD_PEND", TF_PMTUD_PEND),
            ("TF_NEEDOUTPUT", TF_NEEDOUTPUT),
            ("TF_NOPUSH", TF_NOPUSH),
            ("TF_TMR_REXMT", TF_TMR_REXMT),
            ("TF_TMR_PERSIST", TF_TMR_PERSIST),
            ("TF_TMR_KEEP", TF_TMR_KEEP),
            ("TF_TMR_2MSL", TF_TMR_2MSL),
            ("TF_TMR_DELACK", TF_TMR_DELACK),
            ("SCF_UNREACH", SCF_UNREACH),
            ("SCF_TIMESTAMP", u32::from(SCF_TIMESTAMP)),
            ("SCF_SACK_PERMIT", u32::from(SCF_SACK_PERMIT)),
            ("SCF_ECN_PERMIT", u32::from(SCF_ECN_PERMIT)),
            ("SCF_SIGNATURE", u32::from(SCF_SIGNATURE)),
        ] {
            let text = defs[name].trim_end_matches('U');
            let c = u32::from_str_radix(text.trim_start_matches("0x"), 16).expect(name);
            assert_eq!(c, v, "{name}");
        }
        assert_eq!(defs["TF_TIMER"], "TF_TMR_REXMT");
    }
}
/* </TESTS> */
