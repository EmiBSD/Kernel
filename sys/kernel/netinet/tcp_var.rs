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
//! Kernel variables for TCP: `<netinet/tcp_var.h>`'s constants and statistics (N6.6a1).
//!
//! Upstream: sys/netinet/tcp_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_var.rs@3c62ede68802
//!
//! The `t_flags` and `t_oobflags` bits, the SYN cache's sizes and flags, the round-trip
//! scales, `TCP_TIME`, and `struct tcpstat`'s counters: `enum tcpstat_counters` and the
//! per-CPU `tcpcounters` they index. The header's userland part (`TCPCTL_*`, `struct
//! tcpstat`, `struct tcp_ident_mapping`) is `hal::sys::netinet_tcp_var`'s; the control block,
//! the reassembly entries, the SACK holes and the SYN cache's structures follow at N6.6a2.
//!
//! ## Deviations
//! - `TCP_TIME(sec)` is the `const fn` [`tcp_time`] (the C macro row of `docs/C_TO_RUST.md`).
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
//! - The constants of LZ's `sys/netinet/tcp_var.rs` moved here unchanged; `bsd`'s module
//!   re-exports them and keeps the structures until N6.6a2.

use hal::percpu::Counters;

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

/// `TCP_TIME(sec)`: `tcp_now()` is in milliseconds.
pub const fn tcp_time(sec: i32) -> i32 {
    sec * 1000
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
