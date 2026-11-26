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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)tcp_timer.h	8.1 (Berkeley) 6/10/93
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
 *	@(#)tcp_timer.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! TCP timers: `<netinet/tcp_timer.h>` and `netinet/tcp_timer.c`.
//!
//! Upstream: sys/netinet/tcp_timer.h @ 3ce1f3f79392
//! Upstream: sys/netinet/tcp_timer.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_timer.rs@3c62ede68802
//!
//! The five timers of a TCP connection (retransmit, persist, keep alive, 2*MSL, delayed
//! ACK) are numbered by `TCPT_*`, which index the control block's `t_timer[]` and shift
//! `TF_TIMER`; their intervals are milliseconds (`TCP_TIME`). `TCP_TIMER_INIT`, `_ARM`,
//! `_DISARM` and `_ISARMED` are [`tcp_timer_init`], [`tcp_timer_arm`], [`tcp_timer_disarm`]
//! and [`tcp_timer_isarmed`]; every callout enters through [`tcp_timer_enter`], which takes
//! the shared net lock and the socket lock and ignores a timer that was canceled or
//! rescheduled meanwhile, then runs the timer's work. [`tcp_slowtimo`], the protocol's
//! 500 ms routine, only advances `tcp_iss`.
//!
//! ## Deviations
//! - A keep alive probe of a connection without a template sends nothing (the C dereferences
//!   `tp->t_template`; every connection past its SYN has one: `tcp_connect` and
//!   `syn_cache_get` fail without it), as LZ's.
//! - The retransmit timer's path MTU notice (`TF_PMTUD_PEND`) gives the pcb's count back
//!   when the callout returns (the wheel's clone, D-11), after `icmp_mtudisc` and the walk,
//!   where the C's `in_pcbunref` runs before them; no interface reports the count.
//! - The 2*MSL callout's trace after a close records the closed block's address with no
//!   snapshot (the C passes `tp`, NULL after `tcp_close`, beside the freed `otp`).
//! - `TCPT_RANGESET` is the function [`tcpt_rangeset`] (the C macro row of
//!   `docs/C_TO_RUST.md`); it returns the value instead of assigning an lvalue. Timer numbers
//!   are `usize` (they index `t_timer[]`), times in milliseconds `i32` (the C's `int`
//!   constants).
//! - The `tcptimers[]` names (behind `TCPTIMERS`) are only compiled by `tcp_debug.c` with
//!   `TCPDEBUG`, which is not configured.
//!
//! ## Redesign
//! - N6.6a1: the header's constants and `TCPT_RANGESET` moved here unchanged; `bsd`'s
//!   `sys/netinet/tcp_timer.rs` re-exports them and keeps its own callouts and helpers for
//!   its TCP until N6.6a5.
//! - N6.6a2 (D-11): the C's `in_pcbref` per armed timer (`TCP_TIMER_ARM` counts the pcb when
//!   `timeout_add` returns true, `TCP_TIMER_DISARM` or the callout gives it back; LZ the same
//!   over a raw `void *`) is one counted handle of the pcb per timer for the control block's
//!   life: [`tcp_timer_init`] gives it to the timeout (`Timeout::set_shared`), whose wheel
//!   clones it for each firing; arm and disarm count nothing. The cycle pcb, slot, tcpcb,
//!   timer, pcb is broken where the C frees the tcpcb: `tcp_free` drops it, and with it the
//!   five handles (a pending timer is deleted by its drop). A callout already running when
//!   `tcp_close` runs holds the wheel's clone, waits for the socket lock, finds the socket
//!   detached and returns: the C's.
//! - `tcp_timer_enter` and `tcp_timer_leave` are one function taking the timer's work as a
//!   closure: the holdings (the shared net lock's witness, the socket lock opened in parts,
//!   the control block) live on its stack and are released in the C's order when the work
//!   returns; work that closes the connection answers `Closed`, and the control block is freed
//!   there, before the socket lock goes (`tcp_free`). The callouts' `void *` is the pcb's
//!   erased handle, named back by its type (LZ: an `unsafe` cast, `timer_inpcb`).
//! - `tcp_timer_funcs[]` is [`tcp_timer_funcs`], a function of the glue (a callout is generic
//!   over it, as the pcb): five `SharedFn`s by `TCPT_*` index, as `softintr_hand`.
//! - `tcp_timer_freesack` empties the owned hole list one hole at a time.
//! - N6.6a3: the four other callouts' work (LZ's over `&'static Tcpcb`): the work answers a
//!   [`TimerWork`]: the block kept, closed (`tcp_drop`, `tcp_close`: freed by the entry
//!   before the socket lock goes), or the retransmit timer's [`PmtudNotice`], which the entry
//!   acts on once the socket lock is let go and before the net lock is (`icmp_mtudisc`, then
//!   `in_pcbnotifyall` with `tcp_mtudisc`, which locks each connection's socket, this one's
//!   included: the C unlocks first for the same reason). The trace's `otp` is the block's
//!   address, `ostate` its state before the work (`trace_start`). The C's mixed `int`,
//!   `u_int` and `uint64_t` comparisons and `TCPT_RANGESET`'s order are kept where the types
//!   mix (the retransmit timeout, the persist and keep alive idle bounds).

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::mem::pool::{AnyArc, PoolArc};
use hal::sync::netlock::{NETLOCK, NetHeld};
use hal::sys::errno::Errno;
use hal::sys::net_route::{RTF_HOST, RTV_MTU};
use hal::sys::netinet_in::SockaddrIn;
use hal::sys::netinet_ip::Ip;
use hal::sys::socket::{AF_INET, SO_DEBUG, SO_KEEPALIVE};

use crate::assert::{kassert, kpanic};
use crate::net::proto::PR_SLOWHZ;
use crate::netinet::in_pcb::{
    INP_IPV6, Inpcb, in_losing, in_pcbnotifyall, in_pcbrtchange, in_pcbrtentry, in_pcbsolock,
};
use crate::netinet::ip_icmp::{icmp_mtudisc, icmp_mtudisc_clone};
use crate::netinet::ip_var::ip_mtudisc;
use crate::netinet::tcp_debug::{TA_TIMER, tcp_trace};
use crate::netinet::tcp_fsm::{
    TCPS_CLOSING, TCPS_SYN_RECEIVED, TCPS_SYN_SENT, TCPS_TIME_WAIT, tcps_haveestablished,
};
use crate::netinet::tcp_output::{tcp_output, tcp_setpersist};
use crate::netinet::tcp_seq::{TCP_ISSINCR2, seq_geq, seq_lt};
use crate::netinet::tcp_subr::{
    Closed, TCP_DO_ECN, TCP_ISS, TCP_TIMER_MTX, tcp_close, tcp_drop, tcp_free, tcp_mtudisc_notify,
    tcp_respond,
};
use crate::netinet::tcp_var::{
    TCP_RTT_SHIFT, TF_ACKNOW, TF_DISABLE_ECN, TF_PMTUD_PEND, TF_SEND_CWR, TF_TIMER, TcpState,
    Tcpcb, TcpstatCounters, Tp, intotcpcb, tcp_now, tcp_rexmtval, tcp_time, tcpstat_inc,
};
use crate::proc::ProcGlue;
use crate::timeout::{KCLOCK_NONE, SharedFn, TIMEOUT_MPSAFE, TIMEOUT_PROC};

/// `AF_INET` as the `int` of `tp->pf`.
const AF_INET_I32: i32 = AF_INET as i32;

// Definitions of the TCP timers.

/// Retransmit.
pub const TCPT_REXMT: usize = 0;
/// Retransmit persistence.
pub const TCPT_PERSIST: usize = 1;
/// Keep alive.
pub const TCPT_KEEP: usize = 2;
/// 2*msl quiet time timer.
pub const TCPT_2MSL: usize = 3;
/// Delayed ack timeout.
pub const TCPT_DELACK: usize = 4;

/// `TCPT_NTIMERS`.
pub const TCPT_NTIMERS: usize = 5;

// Time constants.

/// Max seg lifetime (hah!).
pub const TCPTV_MSL: i32 = tcp_time(30);
/// Base roundtrip time; if 0, no idea yet.
pub const TCPTV_SRTTBASE: i32 = 0;
/// Assumed RTT if no info.
pub const TCPTV_SRTTDFLT: i32 = tcp_time(3);

/// Retransmit persistence.
pub const TCPTV_PERSMIN: i32 = tcp_time(5);
/// Maximum persist interval.
pub const TCPTV_PERSMAX: i32 = tcp_time(60);

/// Initial connect keep alive.
pub const TCPTV_KEEPINIT: i32 = tcp_time(75);
/// Dflt time before probing.
pub const TCPTV_KEEPIDLE: i32 = tcp_time(120 * 60);
/// Default probe interval.
pub const TCPTV_KEEPINTVL: i32 = tcp_time(75);
/// Max probes before drop.
pub const TCPTV_KEEPCNT: i32 = 8;

/// Minimum allowable value.
pub const TCPTV_MIN: i32 = tcp_time(1);
/// Max allowable REXMT value.
pub const TCPTV_REXMTMAX: i32 = tcp_time(64);

/// Linger at most 2 minutes.
pub const TCP_LINGERTIME: i16 = 120;

/// Maximum retransmits.
pub const TCP_MAXRXTSHIFT: i32 = 12;

/// Time to delay ACK.
pub const TCP_DELACK_MSECS: i32 = 200;

/// `tcp_totbackoff`: sum of `tcp_backoff[]`.
pub const TCP_TOTBACKOFF: i32 = 511;

/// What a callout's work leaves for [`tcp_timer_enter`] to finish.
pub enum TimerWork {
    /// The control block stays.
    Kept,
    /// The work closed it (`tcp_close`, `tcp_drop`): it is freed before the socket lock
    /// goes.
    Closed(Closed),
    /// The retransmit timer's path MTU notice (`TF_PMTUD_PEND`), acted on once the socket
    /// lock is let go, under the net lock.
    Mtudisc(PmtudNotice),
}

/// The retransmit timer's path MTU notice: the fake ICMP message `icmp_mtudisc` reads (its
/// next-hop MTU, network order, and the returned header's length, header length and
/// destination), the peer every connection to which is told (`in_pcbnotifyall`), and the
/// routing table.
pub struct PmtudNotice {
    /// `icmp.icmp_nextmtu`.
    pub nextmtu: u16,
    /// `icmp.icmp_ip`.
    pub oip: Ip,
    /// The peer, as `in_pcbnotifyall`'s destination.
    pub sin: SockaddrIn,
    /// `inp->inp_rtableid`.
    pub rtableid: u32,
}

/// \[a\] `tcp_always_keepalive`: assume `SO_KEEPALIVE` always set.
pub static TCP_ALWAYS_KEEPALIVE: AtomicI32 = AtomicI32::new(0);
/// \[a\] `tcp_keepinit`: time to keep alive initial SYN packet.
pub static TCP_KEEPINIT: AtomicI32 = AtomicI32::new(TCPTV_KEEPINIT);
/// \[a\] `tcp_keepidle`: time before keepalive probes begin.
pub static TCP_KEEPIDLE: AtomicI32 = AtomicI32::new(TCPTV_KEEPIDLE);
/// \[a\] `tcp_keepintvl`: time between keepalive probes.
pub static TCP_KEEPINTVL: AtomicI32 = AtomicI32::new(TCPTV_KEEPINTVL);
/// \[a\] `tcp_keepinit_sec`: copy of `tcp_keepinit` in seconds for sysctl.
pub static TCP_KEEPINIT_SEC: AtomicI32 = AtomicI32::new(TCPTV_KEEPINIT / tcp_time(1));
/// \[a\] `tcp_keepidle_sec`: copy of `tcp_keepidle` in seconds for sysctl.
pub static TCP_KEEPIDLE_SEC: AtomicI32 = AtomicI32::new(TCPTV_KEEPIDLE / tcp_time(1));
/// \[a\] `tcp_keepintvl_sec`: copy of `tcp_keepintvl` in seconds for sysctl.
pub static TCP_KEEPINTVL_SEC: AtomicI32 = AtomicI32::new(TCPTV_KEEPINTVL / tcp_time(1));
/// `tcp_maxpersistidle`: max idle time in persist.
pub static TCP_MAXPERSISTIDLE: AtomicI32 = AtomicI32::new(TCPTV_KEEPIDLE);
/// \[I\] `tcp_delack_msecs`: time to delay the ACK.
#[allow(non_upper_case_globals)] // TCP_DELACK_MSECS is a constant of tcp_timer.h
pub static tcp_delack_msecs: i32 = TCP_DELACK_MSECS;

/// `tcp_backoff[]`.
pub static TCP_BACKOFF: [i32; TCP_MAXRXTSHIFT as usize + 1] =
    [1, 2, 4, 8, 16, 32, 64, 64, 64, 64, 64, 64, 64];

/// `TCPT_RANGESET(tv, value, tvmin, tvmax)`: force a time value to be in a certain range;
/// returns what the C assigns to `tv`.
pub fn tcpt_rangeset<T: PartialOrd>(value: T, tvmin: T, tvmax: T) -> T {
    if value < tvmin {
        tvmin
    } else if value > tvmax {
        tvmax
    } else {
        value
    }
}

/// `tcp_timer_funcs[]`: the callout of each timer, indexed by `TCPT_*`.
pub fn tcp_timer_funcs<G: ProcGlue>() -> [SharedFn; TCPT_NTIMERS] {
    [
        tcp_timer_rexmt::<G>,
        tcp_timer_persist::<G>,
        tcp_timer_keep::<G>,
        tcp_timer_2msl::<G>,
        tcp_timer_delack::<G>,
    ]
}

/// `TCP_TIMER_INIT(tp, timer)`: the timer's callout, with the pcb `inp` as its argument: a
/// counted handle the timeout holds until the control block is freed (D-11), process context
/// without the kernel lock (`TIMEOUT_PROC | TIMEOUT_MPSAFE`). `panic(9)` for a pcb no pool
/// placed (an iterator's marker).
pub fn tcp_timer_init<G: ProcGlue>(cb: &Tcpcb<G>, inp: &Inpcb<G>, timer: usize) {
    let Some(h) = PoolArc::from_ref(inp) else {
        kpanic(format_args!(
            "tcp_timer_init: inpcb {inp:p}: not a pool item"
        ));
    };
    cb.t_timer[timer].set_shared(
        tcp_timer_funcs::<G>()[timer],
        PoolArc::erase(h),
        KCLOCK_NONE,
        TIMEOUT_PROC | TIMEOUT_MPSAFE,
    );
}

/// `TCP_TIMER_ARM(tp, timer, msecs)`: the flag set, the timeout added (the handle it holds is
/// the pcb's count: none is taken here, D-11).
#[inline]
pub fn tcp_timer_arm<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, timer: usize, msecs: u64) {
    tp.st.set_flags(TF_TIMER << timer);
    let _ = tp.cb.timer(timer).add_msec(msecs);
}

/// `TCP_TIMER_DISARM(tp, timer)`: the flag cleared, the timeout deleted (no count to give
/// back, D-11).
#[inline]
pub fn tcp_timer_disarm<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, timer: usize) {
    tp.st.clear_flags(TF_TIMER << timer);
    let _ = tp.cb.t_timer[timer].del();
}

/// `TCP_TIMER_ISARMED(tp, timer)`.
#[inline]
pub fn tcp_timer_isarmed(st: &TcpState, timer: usize) -> bool {
    st.has_flags(TF_TIMER << timer)
}

/// `tcp_timer_enter` .. `tcp_timer_leave`: the shared net lock, then the socket lock of
/// `inp`; nothing more for a socket gone (detached by a close that ran first), a control
/// block gone, or a timeout that was canceled (its flag clear) or rescheduled (pending
/// again) meanwhile; otherwise the flag is cleared and `work` runs with the control block.
/// What `work` closed is freed before the socket lock goes (`tcp_free`); a path MTU notice
/// it leaves runs once the socket lock is let go, under the net lock (the retransmit timer's
/// `TF_PMTUD_PEND` path: `icmp_mtudisc` and the walk that tells every connection to the
/// peer); then the net lock is let go, and the wheel drops its clone of the pcb's handle
/// after the callout (`in_pcbunref`).
pub fn tcp_timer_enter<G: ProcGlue>(
    inp: &Inpcb<G>,
    timer: usize,
    work: impl FnOnce(Tp<'_, '_, G>) -> TimerWork,
) {
    kassert!(timer < TCPT_NTIMERS);

    let w = NETLOCK.read();
    let Some(net) = NetHeld::of(&w) else {
        kpanic(format_args!(
            "tcp_timer_enter: the net lock's witness proves no holding"
        ));
    };
    let notice = {
        let Some(mut so) = in_pcbsolock(inp, &net) else {
            return;
        };
        // A kernel control block belongs to a kernel protocol's socket, always held by a
        // witness (D-1): there is no other case to enter.
        let Some(mut il) = so.inet() else {
            return;
        };
        let Some(tp) = intotcpcb(inp, &mut il) else {
            return;
        };
        // Ignore canceled timeouts or timeouts that have been rescheduled.
        if !tp.st.has_flags(TF_TIMER << timer) || tp.cb.t_timer[timer].pending() {
            return;
        }
        tp.st.clear_flags(TF_TIMER << timer);

        match work(tp) {
            TimerWork::Kept => None,
            TimerWork::Closed(closed) => {
                tcp_free(inp, &mut il.pcb, closed);
                None
            }
            TimerWork::Mtudisc(n) => Some(n),
        }
        // in_pcbsounlock: the socket lock's holding goes here.
    };
    if let Some(n) = notice {
        icmp_mtudisc(n.nextmtu, &n.oip, n.rtableid, &net);
        in_pcbnotifyall(
            &crate::net::domain::net::<G>().tcp.tcbtable,
            &n.sin,
            n.rtableid,
            Some(Errno::EMSGSIZE),
            &net,
            tcp_mtudisc_notify::<G>,
        );
    }
    // NET_UNLOCK_SHARED: the witness's drop.
}

/// The pcb a TCP timer was set with ([`tcp_timer_init`]): the wheel's clone of its handle,
/// lent for the call.
fn timer_inpcb<G: ProcGlue>(arg: &AnyArc) -> &Inpcb<G> {
    match arg.downcast_ref::<Inpcb<G>>() {
        Some(inp) => inp,
        None => kpanic(format_args!("tcp timer: the argument is not an inpcb")),
    }
}

/// `otp` and `ostate` when the socket has `SO_DEBUG`: the control block's address and its
/// state as the callout found them, for the trace after its work.
fn trace_start<G: ProcGlue>(tp: &Tp<'_, '_, G>) -> Option<(usize, i32)> {
    (tp.so.options() & SO_DEBUG != 0).then(|| {
        (
            core::ptr::from_ref::<Tcpcb<G>>(&tp.cb).addr(),
            tp.st.t_state,
        )
    })
}

/// `tcp_timer_delack`: callout to process delayed ACKs for a TCPCB.
pub fn tcp_timer_delack<G: ProcGlue>(arg: &AnyArc) {
    // If tcp_output() wasn't able to transmit the ACK for whatever reason, it will restart
    // the delayed ACK callout.
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_DELACK, |mut tp| {
        let otp = trace_start(&tp);
        tp.st.set_flags(TF_ACKNOW);
        let _ = tcp_output(&mut tp);
        if let Some((otp, ostate)) = otp {
            tcp_trace(
                TA_TIMER,
                ostate,
                Some(tp.st),
                otp,
                None,
                TCPT_DELACK as i32,
                0,
            );
        }
        TimerWork::Kept
    });
}

/// `tcp_slowtimo`: tcp protocol timeout routine called every 500 ms (`pr_slowtimo`): it
/// advances `tcp_iss` (the connections' own timers are callouts).
pub fn tcp_slowtimo() {
    let g = TCP_TIMER_MTX.lock();
    // increment iss
    TCP_ISS.store(
        TCP_ISS
            .load(Ordering::Relaxed)
            .wrapping_add(TCP_ISSINCR2 / PR_SLOWHZ as u32),
        Ordering::Relaxed,
    );
    drop(g);
}

/// `tcp_canceltimers`: cancel all timers for TCP `tp`.
pub fn tcp_canceltimers<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) {
    for i in 0..TCPT_NTIMERS {
        tcp_timer_disarm(tp, i);
    }
}

/// `tcp_timer_freesack`: free SACK holes for 2MSL and REXMT timers, one hole at a time.
pub fn tcp_timer_freesack(st: &mut TcpState) {
    let mut q = st.snd_holes.take();
    while let Some(mut p) = q {
        q = p.next.take();
    }
}

/// `tcp_timer_rexmt`: the retransmit timer: the retransmission backed off (or the
/// connection dropped past `TCP_MAXRXTSHIFT`), path MTU discovery turned off for a peer that
/// seems to swallow large segments, the estimate and the congestion window reset, the
/// unacknowledged data sent again; a path MTU notice `tcp_ctlinput` kept for this segment
/// (`TF_PMTUD_PEND`) is acted on instead.
pub fn tcp_timer_rexmt<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_REXMT, |mut tp| {
        let inp = tp.inp;
        let st = &mut *tp.st;
        if st.has_flags(TF_PMTUD_PEND)
            && seq_geq(st.t_pmtud_th_seq, st.snd_una)
            && seq_lt(
                st.t_pmtud_th_seq,
                st.snd_una.wrapping_add(u32::from(st.t_maxseg)),
            )
        {
            // TF_PMTUD_PEND is set in tcp_ctlinput() which is IPv4 only
            kassert!(!inp.has_flags(INP_IPV6));
            st.clear_flags(TF_PMTUD_PEND);

            let rtableid = inp.inp_rtableid.get();

            // XXX create fake icmp message with relevant entries: its next-hop MTU and the
            // returned header's length, header length and destination, all icmp_mtudisc
            // reads of it.
            let mut oip = Ip {
                ip_len: st.t_pmtud_ip_len,
                ip_dst: inp.inp_faddr.get(),
                ..Ip::default()
            };
            oip.set_ip_hl(st.t_pmtud_ip_hl as u8);

            // Notify all connections to the same peer about new mss and trigger retransmit:
            // after the socket lock is let go (in_pcbsounlock, in_pcbunref).
            let sin = SockaddrIn {
                sin_len: size_of::<SockaddrIn>() as u8,
                sin_family: AF_INET,
                sin_addr: inp.inp_faddr.get(),
                ..SockaddrIn::default()
            };
            return TimerWork::Mtudisc(PmtudNotice {
                nextmtu: st.t_pmtud_nextmtu as u16,
                oip,
                sin,
                rtableid,
            });
        }

        tcp_timer_freesack(st);
        st.t_rxtshift += 1;
        if i32::from(st.t_rxtshift) > TCP_MAXRXTSHIFT {
            st.t_rxtshift = TCP_MAXRXTSHIFT as i16;
            tcpstat_inc(TcpstatCounters::TcpsTimeoutdrop);
            let errno = st.t_softerror.unwrap_or(Errno::ETIMEDOUT);
            return TimerWork::Closed(tcp_drop(tp, Some(errno)));
        }
        let otp = trace_start(&tp);
        let st = &mut *tp.st;
        tcpstat_inc(TcpstatCounters::TcpsRexmttimeo);
        // `uint32_t rto`, the `u_int` minimum.
        let mut rto = tcp_rexmtval(st) as u32;
        if rto < st.t_rttmin {
            rto = st.t_rttmin;
        }
        // TCPT_RANGESET(tp->t_rxtcur, rto * tcp_backoff[], tp->t_rttmin, TCPTV_REXMTMAX):
        // the `u_int` product assigned to the `int` first, then compared with the `u_int`
        // minimum (unsigned) and the `int` maximum, in the macro's order.
        let mut rxtcur = rto.wrapping_mul(TCP_BACKOFF[st.t_rxtshift as usize] as u32) as i32;
        if (rxtcur as u32) < st.t_rttmin {
            rxtcur = st.t_rttmin as i32;
        } else if rxtcur > TCPTV_REXMTMAX {
            rxtcur = TCPTV_REXMTMAX;
        }
        st.t_rxtcur = rxtcur;
        tcp_timer_arm(&mut tp, TCPT_REXMT, rxtcur as u64);

        // If we are losing and we are trying path MTU discovery, try turning it off. This
        // will avoid black holes in the network which suppress or fail to send "packet too
        // big" ICMP messages. We should ideally do lots more sophisticated searching to find
        // the right value here...
        if ip_mtudisc.load(Ordering::Relaxed) != 0
            && tcps_haveestablished(tp.st.t_state)
            && i32::from(tp.st.t_rxtshift) > TCP_MAXRXTSHIFT / 6
        {
            'leave: {
                // No data to send means path mtu is not a problem
                if tp.so.so_snd.cc() == 0 {
                    break 'leave;
                }

                // Check if path MTU discovery is disabled already
                if in_pcbrtentry(inp, tp.net).is_some_and(|rt| {
                    rt.rt_flags.isset(RTF_HOST) && rt.rt_locks().get() & RTV_MTU != 0
                }) {
                    break 'leave;
                }

                let rt = match tp.st.pf {
                    AF_INET_I32 => icmp_mtudisc_clone(
                        inp.inp_faddr.get(),
                        inp.inp_rtableid.get(),
                        false,
                        tp.net,
                    ),
                    // AF_INET6: we can not turn off path MTU for IPv6. Do nothing for now,
                    // maybe lower to minimum MTU. No other family has a route here.
                    _ => None,
                };
                if let Some(rt) = rt {
                    // Disable path MTU discovery
                    if rt.rt_locks().get() & RTV_MTU == 0 {
                        rt.rt_locks().set(rt.rt_locks().get() | RTV_MTU);
                        in_pcbrtchange(inp, None);
                    }
                    // rtfree(rt): the handle's drop.
                    drop(rt);
                }
            }
        }

        // If losing, let the lower level know and try for a better route. Also, if we
        // backed off this far, our srtt estimate is probably bogus. Clobber it so we'll take
        // the next rtt measurement as our srtt; move the current srtt into rttvar to keep the
        // current retransmit times until then.
        if i32::from(tp.st.t_rxtshift) > TCP_MAXRXTSHIFT / 4 {
            in_losing(inp, tp.net);
            let st = &mut *tp.st;
            st.t_rttvar = st.t_rttvar.wrapping_add(st.t_srtt >> TCP_RTT_SHIFT);
            st.t_srtt = 0;
        }
        let st = &mut *tp.st;
        st.snd_nxt = st.snd_una;
        // Note: We overload snd_last to function also as the snd_last variable described in
        // RFC 2582
        st.snd_last = st.snd_max;
        // If timing a segment in this window, stop the timer.
        st.t_rtttime = 0;
        // TCP_ECN: if ECN is enabled, there might be a broken firewall which blocks ecn
        // packets. fall back to non-ecn.
        if (st.t_state == TCPS_SYN_SENT || st.t_state == TCPS_SYN_RECEIVED)
            && TCP_DO_ECN.load(Ordering::Relaxed) != 0
            && !st.has_flags(TF_DISABLE_ECN)
        {
            st.set_flags(TF_DISABLE_ECN);
        }
        // Close the congestion window down to one segment (we'll open it by one segment for
        // each ack we get). Since we probably have a window's worth of unacked data
        // accumulated, this "slow start" keeps us from dumping all that data as back-to-back
        // packets (which might overwhelm an intermediate gateway).
        //
        // There are two phases to the opening: Initially we open by one mss on each ack.
        // This makes the window size increase exponentially with time. If the window is
        // larger than the path can handle, this exponential growth results in dropped
        // packet(s) almost immediately. To get more time between drops but still "push" the
        // network to take advantage of improving conditions, we switch from exponential to
        // linear window opening at some threshold size. For a threshold, we use half the
        // current window size, truncated to a multiple of the mss.
        //
        // (the minimum cwnd that will give us exponential growth is 2 mss. We don't allow
        // the threshold to go below this.)
        {
            let maxseg = u64::from(st.t_maxseg);
            let win = (st.snd_wnd.min(st.snd_cwnd) / 2 / maxseg).max(2);
            st.snd_cwnd = maxseg;
            st.snd_ssthresh = win * maxseg;
            st.t_dupacks = 0;
            // TCP_ECN
            st.snd_last = st.snd_max;
            st.set_flags(TF_SEND_CWR);
            // #if 1 /* TCP_ECN */
            tcpstat_inc(TcpstatCounters::TcpsCwrTimeout);
        }
        let _ = tcp_output(&mut tp);
        if let Some((otp, ostate)) = otp {
            tcp_trace(
                TA_TIMER,
                ostate,
                Some(tp.st),
                otp,
                None,
                TCPT_REXMT as i32,
                0,
            );
        }
        TimerWork::Kept
    });
}

/// `tcp_timer_persist`: the persist timer: a window probe forced out (`t_force`), or the
/// connection dropped once the probes have gone unanswered as long as a full retransmit
/// backoff.
pub fn tcp_timer_persist<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_PERSIST, |mut tp| {
        if tcp_timer_isarmed(tp.st, TCPT_REXMT) {
            return TimerWork::Kept;
        }

        let otp = trace_start(&tp);
        tcpstat_inc(TcpstatCounters::TcpsPersisttimeo);
        // Hack: if the peer is dead/unreachable, we do not time out if the window is closed.
        // After a full backoff, drop the connection if the idle time (no responses to
        // probes) reaches the maximum backoff that we would use if retransmitting.
        let st = &*tp.st;
        let mut rto = tcp_rexmtval(st) as u32;
        if rto < st.t_rttmin {
            rto = st.t_rttmin;
        }
        let now = tcp_now();
        let idle = now.wrapping_sub(st.t_rcvtime);
        // The `uint64_t` idle time against the `int` maximum (converted) and the `u_int`
        // product `rto * tcp_totbackoff` (widened).
        if i32::from(st.t_rxtshift) == TCP_MAXRXTSHIFT
            && (idle >= TCP_MAXPERSISTIDLE.load(Ordering::Relaxed) as u64
                || idle >= u64::from(rto.wrapping_mul(TCP_TOTBACKOFF as u32)))
        {
            tcpstat_inc(TcpstatCounters::TcpsPersistdrop);
            return TimerWork::Closed(tcp_drop(tp, Some(Errno::ETIMEDOUT)));
        }
        tcp_setpersist(&mut tp);
        tp.st.t_force = true;
        let _ = tcp_output(&mut tp);
        tp.st.t_force = false;
        if let Some((otp, ostate)) = otp {
            tcp_trace(
                TA_TIMER,
                ostate,
                Some(tp.st),
                otp,
                None,
                TCPT_PERSIST as i32,
                0,
            );
        }
        TimerWork::Kept
    });
}

/// `tcp_timer_keep`: the keep alive (and connection establishment) timer: a connection not
/// yet established is dropped; an idle one with keep alive on is probed, and dropped when
/// the probes go unanswered.
pub fn tcp_timer_keep<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_KEEP, |mut tp| {
        let otp = trace_start(&tp);
        tcpstat_inc(TcpstatCounters::TcpsKeeptimeo);
        if !tcps_haveestablished(tp.st.t_state) {
            tcpstat_inc(TcpstatCounters::TcpsKeepdrops);
            return TimerWork::Closed(tcp_drop(tp, Some(Errno::ETIMEDOUT)));
        }
        if (TCP_ALWAYS_KEEPALIVE.load(Ordering::Relaxed) != 0 || tp.so.has_options(SO_KEEPALIVE))
            && tp.st.t_state <= TCPS_CLOSING
        {
            let keepidle = TCP_KEEPIDLE.load(Ordering::Relaxed);
            let keepintvl = TCP_KEEPINTVL.load(Ordering::Relaxed);
            let maxidle = TCPTV_KEEPCNT.wrapping_mul(keepintvl);
            let now = tcp_now();
            // The `uint64_t` idle time against the `int` sum, converted.
            if maxidle > 0
                && now.wrapping_sub(tp.st.t_rcvtime) >= keepidle.wrapping_add(maxidle) as u64
            {
                tcpstat_inc(TcpstatCounters::TcpsKeepdrops);
                return TimerWork::Closed(tcp_drop(tp, Some(Errno::ETIMEDOUT)));
            }
            // Send a packet designed to force a response if the peer is up and reachable:
            // either an ACK if the connection is still alive, or an RST if the peer has
            // closed the connection due to timeout or reboot. Using sequence number
            // tp->snd_una-1 causes the transmitted zero-length segment to lie outside the
            // receive window; by the protocol spec, this requires the correspondent TCP to
            // respond.
            tcpstat_inc(TcpstatCounters::TcpsKeepprobe);
            let (ack, seq) = (tp.st.rcv_nxt, tp.st.snd_una.wrapping_sub(1));
            // A connection past SYN has its template (the deviations).
            if let Some(t) = tp.st.t_template.as_ref() {
                tcp_respond(Some(&tp), t.data(), None, ack, seq, 0, 0, now, tp.net);
            }
            tcp_timer_arm(&mut tp, TCPT_KEEP, keepintvl as u64);
        } else {
            let keepidle = TCP_KEEPIDLE.load(Ordering::Relaxed);
            tcp_timer_arm(&mut tp, TCPT_KEEP, keepidle as u64);
        }
        if let Some((otp, ostate)) = otp {
            tcp_trace(
                TA_TIMER,
                ostate,
                Some(tp.st),
                otp,
                None,
                TCPT_KEEP as i32,
                0,
            );
        }
        TimerWork::Kept
    });
}

/// `tcp_timer_2msl`: the 2*MSL timer: past TIME_WAIT, or idle past the keep alive probes,
/// the connection is closed; otherwise the timer runs again in `tcp_keepintvl`.
pub fn tcp_timer_2msl<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_2MSL, |mut tp| {
        let otp = trace_start(&tp);
        tcp_timer_freesack(tp.st);

        let keepintvl = TCP_KEEPINTVL.load(Ordering::Relaxed);
        let maxidle = TCPTV_KEEPCNT * keepintvl;
        let now = tcp_now();
        // The C compares the `uint64_t` difference with the `int` bound, converted.
        if tp.st.t_state != TCPS_TIME_WAIT
            && (maxidle == 0 || now.wrapping_sub(tp.st.t_rcvtime) <= maxidle as u64)
        {
            tcp_timer_arm(&mut tp, TCPT_2MSL, u64::try_from(keepintvl).unwrap_or(0));
            if let Some((otp, ostate)) = otp {
                tcp_trace(
                    TA_TIMER,
                    ostate,
                    Some(tp.st),
                    otp,
                    None,
                    TCPT_2MSL as i32,
                    0,
                );
            }
            TimerWork::Kept
        } else {
            let closed = tcp_close(tp);
            if let Some((otp, ostate)) = otp {
                tcp_trace(TA_TIMER, ostate, None, otp, None, TCPT_2MSL as i32, 0);
            }
            TimerWork::Closed(closed)
        }
    });
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use hal::sys::netinet_tcp::{TH_ACK, TH_RST};

    use super::*;
    use crate::netinet::tcp_subr::testtcp;

    #[test]
    fn rangeset_clamps_as_the_macro() {
        assert_eq!(tcpt_rangeset(5, 10, 20), 10);
        assert_eq!(tcpt_rangeset(25, 10, 20), 20);
        assert_eq!(tcpt_rangeset(15, 10, 20), 15);
        // The macro tests the minimum first: an empty range yields the minimum.
        assert_eq!(tcpt_rangeset(15, 30, 20), 30);
        assert_eq!(TCPTV_KEEPIDLE, 7_200_000);
    }

    /// The C's `tcp_totbackoff` is the sum of `tcp_backoff[]` (`tcp_timer.c`, where the two
    /// are written apart), and `tcp_keepintvl_sec` starts as `TCPTV_KEEPINTVL` in seconds
    /// (75). bsd's `backoff_sums_to_the_total`, re-expressed where the items live now.
    #[test]
    fn backoff_sums_to_the_total() {
        assert_eq!(TCP_BACKOFF.iter().sum::<i32>(), TCP_TOTBACKOFF);
        assert_eq!(TCP_KEEPINTVL_SEC.load(Ordering::Relaxed), 75);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/tcp_timer.h");
        crate::reftest::assert_defines!(defs;
            TCPT_REXMT, TCPT_PERSIST, TCPT_KEEP, TCPT_2MSL, TCPT_DELACK, TCPT_NTIMERS,
            TCPTV_SRTTBASE, TCPTV_KEEPCNT, TCP_LINGERTIME, TCP_MAXRXTSHIFT, TCP_DELACK_MSECS);
        // TCP_TIME(n) is a function-like macro: compare its argument.
        for (name, v) in [
            ("TCPTV_MSL", TCPTV_MSL),
            ("TCPTV_SRTTDFLT", TCPTV_SRTTDFLT),
            ("TCPTV_PERSMIN", TCPTV_PERSMIN),
            ("TCPTV_PERSMAX", TCPTV_PERSMAX),
            ("TCPTV_KEEPINIT", TCPTV_KEEPINIT),
            ("TCPTV_KEEPIDLE", TCPTV_KEEPIDLE),
            ("TCPTV_KEEPINTVL", TCPTV_KEEPINTVL),
            ("TCPTV_MIN", TCPTV_MIN),
            ("TCPTV_REXMTMAX", TCPTV_REXMTMAX),
        ] {
            let arg = defs[name]
                .strip_prefix("TCP_TIME(")
                .and_then(|s| s.strip_suffix(')'))
                .expect(name);
            let secs: i32 = arg
                .split('*')
                .map(|f| f.trim().parse::<i32>().expect(name))
                .product();
            assert_eq!(tcp_time(secs), v, "{name}");
        }
    }

    #[test]
    fn a_callout_does_nothing_once_disarmed_rescheduled_or_closed() {
        let (_g, _p) = testtcp::setup();
        let (so, inp) = testtcp::conn();
        let arg = PoolArc::erase(inp.clone());
        // Each check of tcp_timer_enter stops the callout before its work (which would send
        // a retransmission here): a disarmed timer (its flag clear) ...
        tcp_timer_rexmt::<testtcp::G>(&arg);
        // ... a rescheduled one (its flag set, the timeout pending again) ...
        testtcp::with_tp(&inp, |mut tp| {
            tcp_timer_arm(&mut tp, TCPT_REXMT, 60_000);
            ((), None)
        });
        tcp_timer_rexmt::<testtcp::G>(&arg);
        let armed = testtcp::with_tp(&inp, |tp| (tcp_timer_isarmed(tp.st, TCPT_REXMT), None));
        assert!(armed, "the rescheduled timer's flag is kept");
        // ... and one of a closed connection (the socket detached).
        testtcp::close(so, inp);
        tcp_timer_rexmt::<testtcp::G>(&arg);
        tcp_timer_keep::<testtcp::G>(&arg);
        drop(arg);
    }

    #[test]
    fn freesack_empties_the_holes() {
        let (_g, _p) = testtcp::setup();
        let mut st = TcpState::new();
        let out = crate::netinet::tcp_subr::SACKHL_POOL.nout();
        for i in 0..3 {
            let h = crate::netinet::tcp_subr::SACKHL_POOL
                .get(hal::mem::pool::PR_WAITOK, || {
                    crate::netinet::tcp_var::Sackhole::new(i, i + 1, 1, i)
                })
                .expect("a hole");
            let mut h = h;
            if let Some(rest) = st.snd_holes.take() {
                let _ = h.next.set(rest);
            }
            let _ = st.snd_holes.set(h);
        }
        assert_eq!(crate::netinet::tcp_subr::SACKHL_POOL.nout(), out + 3);
        tcp_timer_freesack(&mut st);
        assert!(!st.snd_holes.is_some());
        assert_eq!(crate::netinet::tcp_subr::SACKHL_POOL.nout(), out);
    }

    #[test]
    fn the_2msl_timer_rearms_a_live_connection_and_closes_an_idle_or_time_wait_one() {
        use crate::net::sockbuf::SS_NOFDREF;
        use crate::netinet::tcp_debug::{RING_TEST, TCP_DEBUG, TCP_NDEBUG, TcpDebugCb};
        use crate::netinet::tcp_fsm::TCPS_ESTABLISHED;
        use crate::netinet::tcp_var::{Sackhole, TF_TMR_2MSL};

        let (_g, _p) = testtcp::setup();
        let _ring = RING_TEST.lock().unwrap_or_else(|e| e.into_inner());
        let last = || {
            let r = TCP_DEBUG.lock();
            r.tcp_debug[(r.tcp_debx + TCP_NDEBUG - 1) % TCP_NDEBUG]
        };
        let tcpcbs = testtcp::tcpcbs();
        for (state, idle) in [(TCPS_ESTABLISHED, true), (TCPS_TIME_WAIT, false)] {
            let (so, inp) = testtcp::conn();
            so.set_options_legacy(SO_DEBUG, true);
            let arg = PoolArc::erase(inp.clone());
            // A timer that came due: its flag set, the timeout no longer pending; a hole.
            let addr = testtcp::with_tp(&inp, |tp| {
                tp.st.t_state = TCPS_ESTABLISHED;
                tp.st.t_rcvtime = tcp_now();
                tp.st.set_flags(TF_TMR_2MSL);
                let h = crate::netinet::tcp_subr::SACKHL_POOL
                    .get(hal::mem::pool::PR_WAITOK, || Sackhole::new(1, 2, 1, 1))
                    .expect("a hole");
                let _ = tp.st.snd_holes.set(h);
                (
                    core::ptr::from_ref::<Tcpcb<testtcp::G>>(&tp.cb).addr(),
                    None,
                )
            });
            // Heard from recently, not in TIME_WAIT: run again, the holes freed.
            tcp_timer_2msl::<testtcp::G>(&arg);
            let (armed, pending, holes) = testtcp::with_tp(&inp, |tp| {
                (
                    (
                        tcp_timer_isarmed(tp.st, TCPT_2MSL),
                        tp.cb.t_timer[TCPT_2MSL].pending(),
                        tp.st.snd_holes.is_some(),
                    ),
                    None,
                )
            });
            assert!(armed && pending && !holes, "re-armed, the holes freed");
            let td = last();
            assert_eq!((td.td_act, td.td_req), (TA_TIMER, TCPT_2MSL as i16));
            assert_eq!((td.td_tcb, td.td_ostate), (addr, TCPS_ESTABLISHED as i16));
            assert_eq!(td.td_cb.t_flags & TF_TMR_2MSL, TF_TMR_2MSL, "the snapshot");

            // Idle past the keep alive probes, or in TIME_WAIT: closed and freed.
            so.st_set(SS_NOFDREF);
            testtcp::with_tp(&inp, |mut tp| {
                tcp_timer_disarm(&mut tp, TCPT_2MSL);
                tp.st.set_flags(TF_TMR_2MSL);
                tp.st.t_state = state;
                if idle {
                    tp.st.t_rcvtime = 0;
                }
                ((), None)
            });
            tcp_timer_2msl::<testtcp::G>(&arg);
            assert_eq!(testtcp::tcpcbs(), tcpcbs, "the control block freed");
            let td = last();
            assert_eq!((td.td_tcb, td.td_ostate), (addr, state as i16));
            assert_eq!(
                td.td_cb,
                TcpDebugCb::zeroed(),
                "no snapshot after the close"
            );
            {
                let w = NETLOCK.read();
                let net = NetHeld::of(&w).expect("the net lock");
                assert!(in_pcbsolock(&inp, &net).is_none(), "detached");
            }
            drop(arg);
            drop(inp);
            drop(so);
        }
    }

    /// Makes timer `t` of `inp`'s connection come due: its timeout taken off the wheel with
    /// its flag kept (what the wheel leaves when it runs the callout).
    fn due(inp: &Inpcb<testtcp::G>, t: usize) {
        testtcp::with_tp(inp, |tp| {
            let _ = tp.cb.t_timer[t].del();
            tp.st.set_flags(TF_TIMER << t);
            ((), None)
        });
    }

    #[test]
    fn the_retransmit_timer_backs_off_and_sends_again_from_snd_una() {
        use crate::netinet::tcp_var::{TF_SEND_CWR, TcpstatCounters as C};
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = testtcp::estab(b"ttmr0", 30);
        let data = testtcp::pattern(300);
        testtcp::queue(&so, &data);
        let arg = PoolArc::erase(inp.clone());
        // The segment out, its retransmit timer running.
        let r = testtcp::with_tp(&inp, |mut tp| {
            (crate::netinet::tcp_output::tcp_output(&mut tp), None)
        });
        assert_eq!(r, Ok(()));
        let first = testtcp::sent(d);
        let timeo = testtcp::stat(C::TcpsRexmttimeo);
        due(&inp, TCPT_REXMT);
        tcp_timer_rexmt::<testtcp::G>(&arg);
        let (shift, rxtcur, expect, cwnd, maxseg, nxt, cwr, pending) =
            testtcp::with_tp(&inp, |tp| {
                let st = &*tp.st;
                // TCPT_RANGESET(rto * tcp_backoff[1], t_rttmin, TCPTV_REXMTMAX).
                let rto = (tcp_rexmtval(st) as u32).max(st.t_rttmin);
                (
                    (
                        st.t_rxtshift,
                        st.t_rxtcur,
                        tcpt_rangeset(rto * 2, st.t_rttmin, TCPTV_REXMTMAX as u32) as i32,
                        st.snd_cwnd,
                        u64::from(st.t_maxseg),
                        st.snd_nxt,
                        st.has_flags(TF_SEND_CWR),
                        tp.cb.t_timer[TCPT_REXMT].pending(),
                    ),
                    None,
                )
            });
        assert_eq!(shift, 1);
        assert_eq!(rxtcur, expect, "the timeout backed off");
        assert_eq!(cwnd, maxseg, "slow start from one segment");
        assert!(cwr && pending, "CWR to send, the timer running again");
        assert_eq!(
            nxt, 1300,
            "snd_nxt pulled back to snd_una, then past the resend"
        );
        assert_eq!(testtcp::sent(d), first + 1, "the data again");
        let (_ip, th, _o, payload) = testtcp::last(d);
        assert_eq!(u32::from_be(th.th_seq), 1000);
        assert_eq!(payload, data);
        assert_eq!(testtcp::stat(C::TcpsRexmttimeo), timeo + 1);
        drop(arg);
        testtcp::close(so, inp);
    }

    #[test]
    fn the_retransmit_timer_drops_past_its_last_shift_with_the_soft_error() {
        use crate::netinet::tcp_var::TcpstatCounters as C;
        let (_g, _p) = testtcp::setup();
        let tcpcbs = testtcp::tcpcbs();
        for (name, n, soft, want) in [
            (b"ttmr1", 31, None, Errno::ETIMEDOUT),
            (b"ttmr2", 36, Some(Errno::EHOSTUNREACH), Errno::EHOSTUNREACH),
        ] {
            let (so, inp, d) = testtcp::estab(name, n);
            let arg = PoolArc::erase(inp.clone());
            testtcp::with_tp(&inp, |tp| {
                tp.st.t_rxtshift = TCP_MAXRXTSHIFT as i16;
                tp.st.t_softerror = soft;
                ((), None)
            });
            so.st_set(crate::net::sockbuf::SS_NOFDREF);
            let (drops, before) = (testtcp::stat(C::TcpsTimeoutdrop), testtcp::sent(d));
            due(&inp, TCPT_REXMT);
            tcp_timer_rexmt::<testtcp::G>(&arg);
            assert_eq!(testtcp::stat(C::TcpsTimeoutdrop), drops + 1);
            assert_eq!(so.error(), Some(want));
            // Synchronized: tcp_drop's RST (CLOSED's flags) before the close.
            assert_eq!(testtcp::sent(d), before + 1);
            let (_ip, th, _o, _p) = testtcp::last(d);
            assert_eq!(th.th_flags, TH_RST | TH_ACK);
            assert_eq!(testtcp::tcpcbs(), tcpcbs, "the control block freed");
            drop(arg);
            drop(inp);
            drop(so);
        }
    }

    #[test]
    fn the_retransmit_timeout_keeps_the_macros_int_order() {
        // TCPT_RANGESET assigns the u_int product to the int t_rxtcur first: past INT_MAX it
        // is negative, under neither bound (the unsigned minimum and the int maximum), and
        // stays (the C's); clamping the product unsigned would give TCPTV_REXMTMAX.
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = testtcp::estab(b"ttmr7", 37);
        let arg = PoolArc::erase(inp.clone());
        testtcp::with_tp(&inp, |tp| {
            // TCP_REXMTVAL = ((2^30 >> 3) + 0) >> 2 = 2^25; tcp_backoff[6] = 64: 2^31.
            tp.st.t_srtt = 1 << 30;
            tp.st.t_rttvar = 0;
            tp.st.t_rxtshift = 5;
            ((), None)
        });
        due(&inp, TCPT_REXMT);
        tcp_timer_rexmt::<testtcp::G>(&arg);
        let rxtcur = testtcp::with_tp(&inp, |tp| (tp.st.t_rxtcur, None));
        assert_eq!(rxtcur, i32::MIN);
        drop(arg);
        testtcp::close(so, inp);
    }

    #[test]
    fn a_path_mtu_notice_kept_by_tcp_ctlinput_is_acted_on_by_the_retransmit_timer() {
        let (_g, _p) = testtcp::setup();
        // A peer behind a gateway, whose route icmp_mtudisc clones into a host route.
        let (so, inp, _d) = testtcp::estab_to(b"ttmr3", 32, [10, 20, 32, 2]);
        let arg = PoolArc::erase(inp.clone());
        testtcp::with_tp(&inp, |tp| {
            let st = tp.st;
            st.snd_max = 1300;
            st.set_flags(TF_PMTUD_PEND);
            st.t_pmtud_th_seq = 1000;
            st.t_pmtud_nextmtu = u32::from(1000u16.to_be());
            st.t_pmtud_ip_len = 1040u16.to_be();
            st.t_pmtud_ip_hl = 5;
            ((), None)
        });
        due(&inp, TCPT_REXMT);
        tcp_timer_rexmt::<testtcp::G>(&arg);
        // No back off: the notice made the route's MTU 1000 (icmp_mtudisc) and tcp_mtudisc
        // took the segment size from it, for every connection to the peer.
        let (pend, shift, maxseg) = testtcp::with_tp(&inp, |tp| {
            (
                (
                    tp.st.has_flags(TF_PMTUD_PEND),
                    tp.st.t_rxtshift,
                    tp.st.t_maxseg,
                ),
                None,
            )
        });
        assert!(!pend, "the notice consumed");
        assert_eq!(shift, 0, "no retransmission back off");
        assert_eq!(maxseg, 1000 - 40, "the new path MTU less the headers");
        drop(arg);
        testtcp::close(so, inp);
    }

    #[test]
    fn the_persist_timer_forces_a_byte_into_a_closed_window_then_drops() {
        use crate::netinet::tcp_var::TcpstatCounters as C;
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = testtcp::estab(b"ttmr4", 33);
        let data = testtcp::pattern(100);
        testtcp::queue(&so, &data);
        let arg = PoolArc::erase(inp.clone());
        let (r, persisting) = testtcp::with_tp(&inp, |mut tp| {
            tp.st.snd_wnd = 0;
            let r = crate::netinet::tcp_output::tcp_output(&mut tp);
            ((r, tcp_timer_isarmed(tp.st, TCPT_PERSIST)), None)
        });
        assert_eq!((r, persisting), (Ok(()), true), "persisting");
        let (before, probes) = (testtcp::sent(d), testtcp::stat(C::TcpsSndprobe));
        due(&inp, TCPT_PERSIST);
        tcp_timer_persist::<testtcp::G>(&arg);
        assert_eq!(testtcp::sent(d), before + 1, "a window probe");
        assert_eq!(testtcp::stat(C::TcpsSndprobe), probes + 1);
        let (_ip, th, _o, payload) = testtcp::last(d);
        assert_eq!(u32::from_be(th.th_seq), 1000);
        assert_eq!(payload, &data[..1], "one byte");
        let (force, nxt, max, shift) = testtcp::with_tp(&inp, |tp| {
            let st = &*tp.st;
            ((st.t_force, st.snd_nxt, st.snd_max, st.t_rxtshift), None)
        });
        assert!(!force);
        assert_eq!((nxt, max), (1000, 1001), "persist state: snd_max only");
        assert_eq!(shift, 2, "backed off again");

        // At the last shift, silent past tcp_maxpersistidle: dropped.
        testtcp::with_tp(&inp, |tp| {
            let st = tp.st;
            st.t_rxtshift = TCP_MAXRXTSHIFT as i16;
            st.t_rcvtime = 0;
            ((), None)
        });
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        let pdrops = testtcp::stat(C::TcpsPersistdrop);
        due(&inp, TCPT_PERSIST);
        tcp_timer_persist::<testtcp::G>(&arg);
        assert_eq!(testtcp::stat(C::TcpsPersistdrop), pdrops + 1);
        assert_eq!(so.error(), Some(Errno::ETIMEDOUT));
        drop(arg);
        drop(inp);
        drop(so);
    }

    #[test]
    fn the_keep_alive_timer_probes_an_idle_connection_and_drops_an_embryonic_one() {
        use crate::netinet::tcp_fsm::TCPS_SYN_SENT;
        use crate::netinet::tcp_var::TcpstatCounters as C;
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = testtcp::estab(b"ttmr5", 34);
        let arg = PoolArc::erase(inp.clone());
        // Keep alive off: re-armed with tcp_keepidle, nothing sent.
        let before = testtcp::sent(d);
        due(&inp, TCPT_KEEP);
        tcp_timer_keep::<testtcp::G>(&arg);
        assert_eq!(testtcp::sent(d), before);
        let armed = testtcp::with_tp(&inp, |tp| (tp.cb.t_timer[TCPT_KEEP].pending(), None));
        assert!(armed);

        // Keep alive on: a probe at snd_una - 1 acknowledging rcv_nxt.
        so.set_options_legacy(SO_KEEPALIVE, true);
        let probes = testtcp::stat(C::TcpsKeepprobe);
        due(&inp, TCPT_KEEP);
        tcp_timer_keep::<testtcp::G>(&arg);
        assert_eq!(testtcp::sent(d), before + 1);
        assert_eq!(testtcp::stat(C::TcpsKeepprobe), probes + 1);
        let (_ip, th, _o, payload) = testtcp::last(d);
        assert_eq!(th.th_flags, TH_ACK);
        assert_eq!(
            (u32::from_be(th.th_seq), u32::from_be(th.th_ack)),
            (999, 5000)
        );
        assert!(payload.is_empty());

        // Not established yet: dropped, no RST (nothing received).
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_SYN_SENT;
            ((), None)
        });
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        let (kd, cd) = (
            testtcp::stat(C::TcpsKeepdrops),
            testtcp::stat(C::TcpsConndrops),
        );
        due(&inp, TCPT_KEEP);
        tcp_timer_keep::<testtcp::G>(&arg);
        assert_eq!(testtcp::sent(d), before + 1, "no RST before a SYN came");
        assert_eq!(
            (
                testtcp::stat(C::TcpsKeepdrops),
                testtcp::stat(C::TcpsConndrops)
            ),
            (kd + 1, cd + 1)
        );
        assert_eq!(so.error(), Some(Errno::ETIMEDOUT));
        drop(arg);
        drop(inp);
        drop(so);
    }

    #[test]
    fn the_delayed_ack_timer_sends_the_ack() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = testtcp::estab(b"ttmr6", 35);
        let arg = PoolArc::erase(inp.clone());
        let before = testtcp::sent(d);
        due(&inp, TCPT_DELACK);
        tcp_timer_delack::<testtcp::G>(&arg);
        assert_eq!(testtcp::sent(d), before + 1);
        let (_ip, th, _o, _p) = testtcp::last(d);
        assert_eq!(th.th_flags, TH_ACK);
        assert_eq!(u32::from_be(th.th_ack), 5000);
        let (acknow, armed) = testtcp::with_tp(&inp, |tp| {
            (
                (
                    tp.st.has_flags(TF_ACKNOW),
                    tcp_timer_isarmed(tp.st, TCPT_DELACK),
                ),
                None,
            )
        });
        assert!(!acknow && !armed);
        drop(arg);
        testtcp::close(so, inp);
    }

    #[test]
    fn the_wheel_runs_the_callouts_and_one_frees_the_control_block() {
        use std::time::{Duration, Instant};

        use crate::netinet::tcp_fsm::TCPS_SYN_SENT;

        // The wheel is the process's: the tests that tick it serialise on timeout's lock and
        // kevent's, both taken before the net tests' memory, as theirs are.
        let _t = crate::timeout::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let _e = crate::event::tests::WHEEL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::timeout::startup(100, 10_000, 10_000_000);
        crate::timeout::start_mp_thread_for_tests();
        let (_g, _p) = testtcp::setup();
        let (so, inp, d) = testtcp::estab(b"ttmr9", 39);
        // Hardclock by hand until `done` (the softclockmp thread runs TCP's callouts, which
        // are TIMEOUT_PROC | TIMEOUT_MPSAFE), ten seconds at most.
        let tick_until = |done: &dyn Fn() -> bool| {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !done() && Instant::now() < deadline {
                crate::timeout::tick_once();
                std::thread::sleep(Duration::from_millis(1));
            }
            done()
        };

        // A delayed ACK armed for 10 ms: tcp_timer_delack from the wheel sends the ACK.
        let before = testtcp::sent(d);
        testtcp::with_tp(&inp, |mut tp| {
            tcp_timer_arm(&mut tp, TCPT_DELACK, 10);
            ((), None)
        });
        assert!(
            tick_until(&|| testtcp::sent(d) > before),
            "the delayed ACK went out"
        );
        let (_ip, th, _o, _p) = testtcp::last(d);
        assert_eq!((th.th_flags, u32::from_be(th.th_ack)), (TH_ACK, 5000));
        let armed = testtcp::with_tp(&inp, |tp| (tcp_timer_isarmed(tp.st, TCPT_DELACK), None));
        assert!(!armed, "the callout's flag cleared");

        // An embryonic connection's keep alive armed for 10 ms: tcp_drop and tcp_close from
        // the wheel's thread, the control block freed there.
        let tcpcbs = testtcp::tcpcbs();
        testtcp::with_tp(&inp, |mut tp| {
            tp.st.t_state = TCPS_SYN_SENT;
            tcp_timer_arm(&mut tp, TCPT_KEEP, 10);
            ((), None)
        });
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        assert!(
            tick_until(&|| testtcp::tcpcbs() < tcpcbs),
            "the control block freed by the callout"
        );
        assert_eq!(so.error(), Some(Errno::ETIMEDOUT));
        assert_eq!(testtcp::sent(d), before + 1, "no RST before a SYN came");
        drop(inp);
        drop(so);
    }

    #[test]
    fn slowtimo_advances_the_iss() {
        let (_g, _p) = testtcp::setup();
        let before = TCP_ISS.load(Ordering::Relaxed);
        tcp_slowtimo();
        assert_eq!(
            TCP_ISS.load(Ordering::Relaxed),
            before.wrapping_add(TCP_ISSINCR2 / 2),
            "TCP_ISSINCR2 / PR_SLOWHZ"
        );
    }
}
/* </TESTS> */
