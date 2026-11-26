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
 *	@(#)tcp_fsm.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! TCP FSM state definitions, per RFC 793, September 1981: `<netinet/tcp_fsm.h>`'s states,
//! which userland reads (`struct tcp_info`'s `tcpi_state`, `kinfo_file`'s `t_state`, pf's
//! state table).
//!
//! Upstream: sys/netinet/tcp_fsm.h @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_fsm.rs@3c62ede68802
//!
//! ## Redesign
//! - N6.6a0: `TCP_NSTATES` and the eleven `TCPS_*` of LZ's `sys/netinet/tcp_fsm.rs` move into
//!   `hal::sys`, where the kernel's TCP (N6.6a) names them; `bsd`'s `tcp_fsm.rs` re-exports
//!   them and keeps the predicates and the two tables (`tcp_outflags[]`, `tcpstates[]`),
//!   which move with the kernel's `tcp_fsm` (N6.6a1).

/// `TCP_NSTATES`: the number of states.
pub const TCP_NSTATES: usize = 11;

/// Closed.
pub const TCPS_CLOSED: i32 = 0;
/// Listening for connection.
pub const TCPS_LISTEN: i32 = 1;
/// Active, have sent syn.
pub const TCPS_SYN_SENT: i32 = 2;
/// Have sent and received syn.
pub const TCPS_SYN_RECEIVED: i32 = 3;
// States < TCPS_ESTABLISHED are those where connections not established.
/// Established.
pub const TCPS_ESTABLISHED: i32 = 4;
/// Received fin, waiting for close.
pub const TCPS_CLOSE_WAIT: i32 = 5;
// States > TCPS_CLOSE_WAIT are those where user has closed.
/// Have closed, sent fin.
pub const TCPS_FIN_WAIT_1: i32 = 6;
/// Closed, exchanged FIN; await ACK.
pub const TCPS_CLOSING: i32 = 7;
/// Had fin and close; await FIN ACK.
pub const TCPS_LAST_ACK: i32 = 8;
// States > TCPS_CLOSE_WAIT && < TCPS_FIN_WAIT_2 await ACK of FIN.
/// Have closed, fin is acked.
pub const TCPS_FIN_WAIT_2: i32 = 9;
/// In 2*msl quiet wait after close.
pub const TCPS_TIME_WAIT: i32 = 10;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::reftest::{assert_complete, assert_defines};

    #[test]
    fn the_states_are_ordered_as_the_predicates_read_them() {
        let all = [
            TCPS_CLOSED,
            TCPS_LISTEN,
            TCPS_SYN_SENT,
            TCPS_SYN_RECEIVED,
            TCPS_ESTABLISHED,
            TCPS_CLOSE_WAIT,
            TCPS_FIN_WAIT_1,
            TCPS_CLOSING,
            TCPS_LAST_ACK,
            TCPS_FIN_WAIT_2,
            TCPS_TIME_WAIT,
        ];
        assert_eq!(all.len(), TCP_NSTATES);
        for (n, s) in all.into_iter().enumerate() {
            assert_eq!(usize::try_from(s), Ok(n));
        }
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::base::reftest::defines("sys/netinet/tcp_fsm.h");
        let s = assert_defines!(defs;
        TCPS_CLOSED, TCPS_LISTEN, TCPS_SYN_SENT, TCPS_SYN_RECEIVED, TCPS_ESTABLISHED,
        TCPS_CLOSE_WAIT, TCPS_FIN_WAIT_1, TCPS_CLOSING, TCPS_LAST_ACK, TCPS_FIN_WAIT_2,
        TCPS_TIME_WAIT);
        // The three predicates are function-like macros, which the reader skips
        // (`tcps_havercvdsyn` & co. in `bsd`'s `tcp_fsm.rs`).
        assert_complete(&defs, "TCPS_", &s);
        assert_defines!(defs; TCP_NSTATES);
    }
}
/* </TESTS> */
