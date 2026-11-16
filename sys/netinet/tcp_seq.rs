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
 *	@(#)tcp_seq.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! TCP sequence number comparisons: `<netinet/tcp_seq.h>`.
//!
//! Upstream: sys/netinet/tcp_seq.h @ 3ce1f3f79392
//!
//! TCP sequence numbers are 32 bit integers operated on with modular arithmetic. These
//! functions compare such integers: `a` is before `b` when `b - a`, taken modulo 2^32, is
//! less than 2^31.
//!
//! Status: `ported`.
//!
//! ## Deviations
//! - `SEQ_LT`, `SEQ_LEQ`, `SEQ_GT`, `SEQ_GEQ` are `const fn`s over `TcpSeq`. The C's
//!   `(int)((a)-(b))` is `a.wrapping_sub(b) as i32`: the unsigned difference wraps and is read
//!   as a signed number, exactly as the C cast does.
//! - `tcp_rcvseqinit(tp)` and `tcp_sendseqinit(tp)` are functions over the control block of
//!   `netinet/tcp_var.rs`.
//! - N6.6a1: the comparisons and `TCP_ISSINCR`/`TCP_ISSINCR2` are the kernel's
//!   (`kernel::netinet::tcp_seq`), re-exported here; the two functions over the control block
//!   stay until the kernel's `tcp_var` (N6.6a2).
//! - `extern tcp_seq tcp_iss` is `TCP_ISS` in `netinet/tcp_subr.rs`, where the C defines it.

pub use kernel::netinet::tcp_seq::{TCP_ISSINCR, TCP_ISSINCR2, seq_geq, seq_gt, seq_leq, seq_lt};

use crate::netinet::tcp_var::Tcpcb;

/// `tcp_rcvseqinit(tp)`: initialize the receive sequence numbers from the initial receive
/// sequence number.
pub fn tcp_rcvseqinit(tp: &Tcpcb) {
    let v = tp.irs.get().wrapping_add(1);
    tp.rcv_nxt.set(v);
    tp.rcv_adv.set(v);
}

/// `tcp_sendseqinit(tp)`: initialize the send sequence numbers from the initial send
/// sequence number.
pub fn tcp_sendseqinit(tp: &Tcpcb) {
    let iss = tp.iss.get();
    tp.snd_up.set(iss);
    tp.snd_max.set(iss);
    tp.snd_nxt.set(iss);
    tp.snd_una.set(iss);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sequence_numbers_start_at_the_initial_ones() {
        let inp: &'static crate::netinet::in_pcb::Inpcb = std::boxed::Box::leak(
            std::boxed::Box::new(crate::netinet::in_pcb::Inpcb::new(None, None)),
        );
        let tp = Tcpcb::new(inp);
        tp.irs.set(u32::MAX);
        tcp_rcvseqinit(&tp);
        assert_eq!((tp.rcv_nxt.get(), tp.rcv_adv.get()), (0, 0));
        tp.iss.set(77);
        tcp_sendseqinit(&tp);
        assert_eq!(
            (
                tp.snd_up.get(),
                tp.snd_max.get(),
                tp.snd_nxt.get(),
                tp.snd_una.get()
            ),
            (77, 77, 77, 77)
        );
        assert!(seq_lt(tp.snd_una.get(), 78));
    }
}
/* </TESTS> */
