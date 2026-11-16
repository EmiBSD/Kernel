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
//! rescheduled meanwhile, then runs the timer's work.
//!
//! ## Deviations
//! - The work of the retransmit, persist, keep alive and delayed ACK callouts is N6.6a3's (it
//!   calls `tcp_output` and `tcp_drop`, the cycle that sub-slice moves): until then the
//!   kernel's TCP is unregistered (N6.6a5), no kernel control block exists outside the host
//!   tests, and a live timer of those four that fires is `panic(9)` naming the sub-slice
//!   (`docs/SCAFFOLDING.md`). Their entry through [`tcp_timer_enter`] (the checks that make a
//!   fire after `tcp_close`, after a disarm or after a re-arm do nothing) is whole.
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

use core::sync::atomic::AtomicI32;

use hal::netlock::{NETLOCK, NetHeld};
use hal::pool::{AnyArc, PoolArc};

use crate::assert::{kassert, kpanic};
use crate::netinet::in_pcb::{Inpcb, in_pcbsolock};
use crate::netinet::tcp_subr::{Closed, tcp_free};
use crate::netinet::tcp_var::{TF_TIMER, TcpState, Tcpcb, Tp, intotcpcb, tcp_time};
use crate::proc::ProcGlue;
use crate::timeout::{KCLOCK_NONE, SharedFn, TIMEOUT_MPSAFE, TIMEOUT_PROC};

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
/// What `work` closed is freed before the socket lock goes (`tcp_free`); then the locks are
/// let go in the C's order (the socket's, then the net lock), and the wheel drops its clone
/// of the pcb's handle after the callout (`in_pcbunref`).
pub fn tcp_timer_enter<G: ProcGlue>(
    inp: &Inpcb<G>,
    timer: usize,
    work: impl FnOnce(Tp<'_, '_, G>) -> Option<Closed>,
) {
    kassert!(timer < TCPT_NTIMERS);

    let w = NETLOCK.read();
    let Some(net) = NetHeld::of(&w) else {
        kpanic(format_args!(
            "tcp_timer_enter: the net lock's witness proves no holding"
        ));
    };
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

    if let Some(closed) = work(tp) {
        tcp_free(inp, &mut il.pcb, closed);
    }
}

/// The pcb a TCP timer was set with ([`tcp_timer_init`]): the wheel's clone of its handle,
/// lent for the call.
fn timer_inpcb<G: ProcGlue>(arg: &AnyArc) -> &Inpcb<G> {
    match arg.downcast_ref::<Inpcb<G>>() {
        Some(inp) => inp,
        None => kpanic(format_args!("tcp timer: the argument is not an inpcb")),
    }
}

/// The work of a callout that is N6.6a3's (the module's deviations).
fn n6_6a3(callout: &str) -> ! {
    kpanic(format_args!(
        "{callout}: the callout's work is N6.6a3's (the kernel's TCP is unregistered until N6.6a5)"
    ))
}

/// `tcp_timer_delack`: callout to process delayed ACKs for a TCPCB (its work N6.6a3's).
pub fn tcp_timer_delack<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_DELACK, |_| {
        n6_6a3("tcp_timer_delack")
    });
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

/// `tcp_timer_rexmt`: the retransmit timer (its work N6.6a3's).
pub fn tcp_timer_rexmt<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_REXMT, |_| {
        n6_6a3("tcp_timer_rexmt")
    });
}

/// `tcp_timer_persist`: the persist timer (its work N6.6a3's).
pub fn tcp_timer_persist<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_PERSIST, |_| {
        n6_6a3("tcp_timer_persist")
    });
}

/// `tcp_timer_keep`: the keep alive timer (its work N6.6a3's).
pub fn tcp_timer_keep<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_KEEP, |_| {
        n6_6a3("tcp_timer_keep")
    });
}

/// `tcp_timer_2msl`: the 2*MSL timer (its work N6.6a3's).
pub fn tcp_timer_2msl<G: ProcGlue>(arg: &AnyArc) {
    tcp_timer_enter(timer_inpcb::<G>(arg), TCPT_2MSL, |_| {
        n6_6a3("tcp_timer_2msl")
    });
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
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
        // Each check of tcp_timer_enter stops the callout before its work (which is N6.6a3's
        // and panics): a disarmed timer (its flag clear) ...
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
                .get(hal::pool::PR_WAITOK, || {
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
}
/* </TESTS> */
