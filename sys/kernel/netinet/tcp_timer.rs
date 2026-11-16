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
//! TCP timers: `<netinet/tcp_timer.h>`'s timer numbers and time constants, and
//! `TCPT_RANGESET` (N6.6a1).
//!
//! Upstream: sys/netinet/tcp_timer.h @ 3ce1f3f79392
//! Upstream: sys/netinet/tcp_timer.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_timer.rs@3c62ede68802
//!
//! The five timers of a TCP connection (retransmit, persist, keep alive, 2*MSL, delayed
//! ACK) are numbered by `TCPT_*`, which index the control block's `t_timer[]` and shift
//! `TF_TIMER`; their intervals are milliseconds (`TCP_TIME`). The arm and disarm helpers and
//! the callouts of `tcp_timer.c` follow at N6.6a2 and N6.6a3.
//!
//! ## Deviations
//! - `TCPT_RANGESET` is the function [`tcpt_rangeset`] (the C macro row of
//!   `docs/C_TO_RUST.md`); it returns the value instead of assigning an lvalue. Timer numbers
//!   are `usize` (they index `t_timer[]`), times in milliseconds `i32` (the C's `int`
//!   constants).
//! - The `tcptimers[]` names (behind `TCPTIMERS`) are only compiled by `tcp_debug.c` with
//!   `TCPDEBUG`, which is not configured.
//!
//! ## Redesign
//! - N6.6a1: the header's constants and `TCPT_RANGESET` moved here unchanged; `bsd`'s
//!   `sys/netinet/tcp_timer.rs` re-exports them and keeps the callouts, the arm and disarm
//!   helpers, `tcp_backoff[]` and the keepalive variables until N6.6a2-6a3.

use crate::netinet::tcp_var::tcp_time;

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
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

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
}
/* </TESTS> */
