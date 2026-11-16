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
 * Copyright (c) 1982, 1986, 1988, 1993
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
//! TCP user requests: `netinet/tcp_usrreq.c`'s socket buffer scaling (N6.6a3:
//! `tcp_update_sndspace`, `tcp_update_rcvspace` and the buffer sizes they read; the requests,
//! `tcp_ctloutput`, `tcp_sysctl` and `tcp_ident` follow at N6.6a5).
//!
//! Upstream: sys/netinet/tcp_usrreq.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_usrreq.rs@3c62ede68802
//!
//! The send buffer grows with the data in flight, so that what the peer has not acknowledged
//! is not counted against the limit the user asked for; the receive buffer grows by a step
//! when most of it was filled within about one round trip. Both shrink back to their
//! defaults when mbufs run low, and keep a socket writable or readable that `poll(2)` found
//! so.
//!
//! ## Deviations
//! - `tcp_sendspace`, `tcp_recvspace` and `tcp_autorcvbuf_inc` are \[I\] `u32` statics, as
//!   LZ's; the first two keep their lowercase names (`TCP_SENDSPACE` and `TCP_RECVSPACE` are
//!   this file's constants).
//!
//! ## Redesign
//! - N6.6a3: the two functions are the kernel's, over the kernel's control block (opened as
//!   [`Tp`]) and socket buffers; `bsd`'s copies in `sys/netinet/tcp_usrreq.rs` stay for
//!   `bsd`'s TCP until the switch (N6.6a5). The buffer's mutex is held across the
//!   computation and the reserve, as the C's `sb_mtx`; the counts and marks are read under it
//!   from the buffer's words.
//! - The sums keep the C's `u_long` order and wrap (`sb_wat + snd_max - snd_una`, which LZ
//!   computed as `sb_wat + (snd_max - snd_una)`: across the sequence space's wrap the C's
//!   difference is past `sb_max` and the buffer goes to `sb_max` for that call), and the
//!   marks' comparisons are the C's unsigned ones (`sb_cc >= sb_lowat` and `nmax < sb_lowat`
//!   with the `long` mark converted, where LZ compared them signed).

use core::sync::atomic::Ordering;

use crate::net::sockbuf::{SB_MAX_VAR, sbchecklowmem};
use crate::netinet::tcp_var::Tp;
use crate::proc::ProcGlue;

/// `TCP_SENDSPACE`.
const TCP_SENDSPACE: u32 = 1024 * 16;
/// `TCP_RECVSPACE`.
const TCP_RECVSPACE: u32 = 1024 * 16;

/// \[I\] `tcp_sendspace`.
#[allow(non_upper_case_globals)] // TCP_SENDSPACE is a constant of this file
pub static tcp_sendspace: u32 = TCP_SENDSPACE;
/// \[I\] `tcp_recvspace`.
#[allow(non_upper_case_globals)] // TCP_RECVSPACE is a constant of this file
pub static tcp_recvspace: u32 = TCP_RECVSPACE;
/// \[I\] `tcp_autorcvbuf_inc`.
pub static TCP_AUTORCVBUF_INC: u32 = 16 * 1024;

/// `roundup(x, y)`: `x` rounded up to a multiple of `y`, in the C's `u_long` arithmetic.
#[inline]
fn roundup(x: u64, y: u64) -> u64 {
    x.wrapping_add(y - 1) / y * y
}

/// `tcp_update_sndspace`: scale the send buffer so that inflight data is not accounted
/// against the limit. The buffer will scale with the congestion window, if the the receiver
/// stops acking data the window will shrink and therefore the buffer size will shrink as
/// well. In low memory situation try to shrink the buffer to the initial size disabling the
/// send buffer scaling as long as the situation persists.
pub fn tcp_update_sndspace<G: ProcGlue>(tp: &Tp<'_, '_, G>) {
    let sb = &tp.so.so_snd;

    let mut g = sb.lock();

    let mut nmax = sb.hiwat();

    if sbchecklowmem() {
        // low on memory try to get rid of some
        if u64::from(tcp_sendspace) < nmax {
            nmax = u64::from(tcp_sendspace);
        }
    } else if sb.wat() != u64::from(tcp_sendspace) {
        // user requested buffer size, auto-scaling disabled
        nmax = sb.wat();
    } else {
        // automatic buffer scaling: `sb_wat + snd_max - snd_una` in the C's order.
        nmax = SB_MAX_VAR.load(Ordering::Relaxed).min(
            sb.wat()
                .wrapping_add(u64::from(tp.st.snd_max))
                .wrapping_sub(u64::from(tp.st.snd_una)),
        );
    }

    // a writable socket must be preserved because of poll(2) semantics
    let lowat = sb.lowat();
    if g.space() >= lowat {
        // `sb_cc + sb_lowat`: the `long` mark converted to the `u_long` count's type.
        let lowat = lowat as u64;
        if nmax < sb.cc().wrapping_add(lowat) {
            nmax = sb.cc().wrapping_add(lowat);
        }
        // keep in sync with sbreserve() calculation
        if nmax.wrapping_mul(8) < sb.mbcnt().wrapping_add(lowat) {
            nmax = sb.mbcnt().wrapping_add(lowat).wrapping_add(7) / 8;
        }
    }

    // round to MSS boundary
    nmax = roundup(nmax, u64::from(tp.st.t_maxseg));

    if nmax != sb.hiwat() {
        // The C ignores the result: a refused size leaves the buffer as it was.
        let _ = g.sbreserve(nmax);
    }

    drop(g);
}

/// `tcp_update_rcvspace`: scale the recv buffer by looking at how much data was transferred
/// in one approximated RTT. If more than a big part of the recv buffer was transferred
/// during that time we increase the buffer by a constant. In low memory situation try to
/// shrink the buffer to the initial size.
pub fn tcp_update_rcvspace<G: ProcGlue>(tp: &Tp<'_, '_, G>) {
    let so = tp.so;
    let sb = &so.so_rcv;

    let mut g = sb.lock();

    let mut nmax = sb.hiwat();

    if sbchecklowmem() {
        // low on memory try to get rid of some
        if u64::from(tcp_recvspace) < nmax {
            nmax = u64::from(tcp_recvspace);
        }
    } else if sb.wat() != u64::from(tcp_recvspace) {
        // user requested buffer size, auto-scaling disabled
        nmax = sb.wat();
    } else {
        // automatic buffer scaling
        if u64::from(tp.st.rfbuf_cnt) > sb.hiwat() / 8 * 7 {
            nmax = SB_MAX_VAR
                .load(Ordering::Relaxed)
                .min(sb.hiwat().wrapping_add(u64::from(TCP_AUTORCVBUF_INC)));
        }
    }

    // a readable socket must be preserved because of poll(2) semantics: the send buffer's
    // mutex for its mark, inside the receive buffer's (the C's order).
    let gs = so.so_snd.lock();
    let slowat = so.so_snd.lowat() as u64;
    if sb.cc() >= sb.lowat() as u64 && nmax < slowat {
        nmax = slowat;
    }
    drop(gs);

    if nmax != sb.hiwat() {
        // round to MSS boundary
        nmax = roundup(nmax, u64::from(tp.st.t_maxseg));
        // The C ignores the result, as above.
        let _ = g.sbreserve(nmax);
    }

    drop(g);
}
/* </CODE> */
