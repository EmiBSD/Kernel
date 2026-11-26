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
//! The part of `<netinet/tcp_var.h>` userland reads: the `net.inet.tcp` sysctl names
//! (`TCPCTL_*`), `struct tcpstat` (`net.inet.tcp.stats`, netstat(1)'s `-s`) and `struct
//! tcp_ident_mapping` (`net.inet.tcp.ident` and `drop`, tcpdrop(8)).
//!
//! Upstream: sys/netinet/tcp_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_var.rs@3c62ede68802
//!
//! ## Redesign
//! - N6.6a0: the three ABI items of LZ's `sys/netinet/tcp_var.rs` move into `hal::sys`, where
//!   the kernel's TCP (N6.6a) names them; `bsd`'s `tcp_var.rs` re-exports them.
//! - [`Tcpstat`] and [`TcpIdentMapping`] are `abi_pod!` images (their tiling checked at
//!   compile time), so LZ's hand-written `unsafe impl AbiPod for TcpIdentMapping` goes; the
//!   C's trailing padding of `struct tcp_ident_mapping` is the named member `_pad` (LZ's).

use core::mem::size_of;

use crate::net::socket::SockaddrStorage;

// Names for TCP sysctl objects.

/// Enable RFC1323 timestamps/scaling.
pub const TCPCTL_RFC1323: i32 = 1;
/// TCPT_KEEP value.
pub const TCPCTL_KEEPINITTIME: i32 = 2;
/// Allow tcp_keepidle to be changed.
pub const TCPCTL_KEEPIDLE: i32 = 3;
/// Allow tcp_keepintvl to be changed.
pub const TCPCTL_KEEPINTVL: i32 = 4;
/// Return kernel idea of PR_SLOWHZ.
pub const TCPCTL_SLOWHZ: i32 = 5;
/// Return bad dynamic port bitmap.
pub const TCPCTL_BADDYNAMIC: i32 = 6;
/// Receive buffer space.
pub const TCPCTL_RECVSPACE: i32 = 7;
/// Send buffer space.
pub const TCPCTL_SENDSPACE: i32 = 8;
/// Get connection owner.
pub const TCPCTL_IDENT: i32 = 9;
/// Selective acknowledgement, rfc 2018.
pub const TCPCTL_SACK: i32 = 10;
/// Default maximum segment size.
pub const TCPCTL_MSSDFLT: i32 = 11;
/// RST pps limit.
pub const TCPCTL_RSTPPSLIMIT: i32 = 12;
/// ACK immediately on PUSH.
pub const TCPCTL_ACK_ON_PUSH: i32 = 13;
/// RFC3168 ECN.
pub const TCPCTL_ECN: i32 = 14;
/// Max size of comp. state engine.
pub const TCPCTL_SYN_CACHE_LIMIT: i32 = 15;
/// Max size of hash bucket.
pub const TCPCTL_SYN_BUCKET_LIMIT: i32 = 16;
/// Enable/disable RFC3390 increased cwnd.
pub const TCPCTL_RFC3390: i32 = 17;
/// Max entries for tcp reass queues.
pub const TCPCTL_REASS_LIMIT: i32 = 18;
/// Drop tcp connection.
pub const TCPCTL_DROP: i32 = 19;
/// Max entries for tcp sack queues.
pub const TCPCTL_SACKHOLE_LIMIT: i32 = 20;
/// TCP statistics.
pub const TCPCTL_STATS: i32 = 21;
/// Assume SO_KEEPALIVE is always set.
pub const TCPCTL_ALWAYS_KEEPALIVE: i32 = 22;
/// Number of uses before reseeding hash.
pub const TCPCTL_SYN_USE_LIMIT: i32 = 23;
/// Return root only port bitmap.
pub const TCPCTL_ROOTONLY: i32 = 24;
/// Number of buckets in the hash.
pub const TCPCTL_SYN_HASH_SIZE: i32 = 25;
/// Enable TCP segmentation offload.
pub const TCPCTL_TSO: i32 = 26;
/// `TCPCTL_MAXID`.
pub const TCPCTL_MAXID: i32 = 27;

/// `struct tcpstat`: TCP statistics, as `sysctl(2)` returns them. Many of these should be
/// kept per connection, but that's inconvenient at the moment.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tcpstat {
    /// `tcps_connattempt`: Connections initiated.
    pub tcps_connattempt: u32,
    /// `tcps_accepts`: Connections accepted.
    pub tcps_accepts: u32,
    /// `tcps_connects`: Connections established.
    pub tcps_connects: u32,
    /// `tcps_drops`: Connections dropped.
    pub tcps_drops: u32,
    /// `tcps_conndrops`: Embryonic connections dropped.
    pub tcps_conndrops: u32,
    /// `tcps_closed`: Conn. closed (includes drops).
    pub tcps_closed: u32,
    /// `tcps_segstimed`: Segs where we tried to get rtt.
    pub tcps_segstimed: u32,
    /// `tcps_rttupdated`: Times we succeeded.
    pub tcps_rttupdated: u32,
    /// `tcps_delack`: Delayed acks sent.
    pub tcps_delack: u32,
    /// `tcps_timeoutdrop`: Conn. dropped in rxmt timeout.
    pub tcps_timeoutdrop: u32,
    /// `tcps_rexmttimeo`: Retransmit timeouts.
    pub tcps_rexmttimeo: u32,
    /// `tcps_persisttimeo`: Persist timeouts.
    pub tcps_persisttimeo: u32,
    /// `tcps_persistdrop`: Connections dropped in persist.
    pub tcps_persistdrop: u32,
    /// `tcps_keeptimeo`: Keepalive timeouts.
    pub tcps_keeptimeo: u32,
    /// `tcps_keepprobe`: Keepalive probes sent.
    pub tcps_keepprobe: u32,
    /// `tcps_keepdrops`: Connections dropped in keepalive.
    pub tcps_keepdrops: u32,
    /// `tcps_sndtotal`: Total packets sent.
    pub tcps_sndtotal: u32,
    /// `tcps_sndpack`: Data packets sent.
    pub tcps_sndpack: u32,
    /// `tcps_sndbyte`: Data bytes sent.
    pub tcps_sndbyte: u64,
    /// `tcps_sndrexmitpack`: Data packets retransmitted.
    pub tcps_sndrexmitpack: u32,
    /// The C compiler's hole before `tcps_sndrexmitbyte` (its 8-byte alignment), zero as the C's
    /// `memset` leaves it.
    pub _pad0: u32,
    /// `tcps_sndrexmitbyte`: Data bytes retransmitted.
    pub tcps_sndrexmitbyte: u64,
    /// `tcps_sndrexmitfast`: Fast retransmits.
    pub tcps_sndrexmitfast: u64,
    /// `tcps_sndacks`: Ack-only packets sent.
    pub tcps_sndacks: u32,
    /// `tcps_sndprobe`: Window probes sent.
    pub tcps_sndprobe: u32,
    /// `tcps_sndurg`: Packets sent with URG only.
    pub tcps_sndurg: u32,
    /// `tcps_sndwinup`: Window update-only packets sent.
    pub tcps_sndwinup: u32,
    /// `tcps_sndctrl`: Control (SYN|FIN|RST) packets sent.
    pub tcps_sndctrl: u32,
    /// `tcps_rcvtotal`: Total packets received.
    pub tcps_rcvtotal: u32,
    /// `tcps_rcvpack`: Packets received in sequence.
    pub tcps_rcvpack: u32,
    /// The C compiler's hole before `tcps_rcvbyte` (its 8-byte alignment), zero as the C's
    /// `memset` leaves it.
    pub _pad1: u32,
    /// `tcps_rcvbyte`: Bytes received in sequence.
    pub tcps_rcvbyte: u64,
    /// `tcps_rcvbadsum`: Packets received with ccksum errs.
    pub tcps_rcvbadsum: u32,
    /// `tcps_rcvbadoff`: Packets received with bad offset.
    pub tcps_rcvbadoff: u32,
    /// `tcps_rcvmemdrop`: Packets dropped for lack of memory.
    pub tcps_rcvmemdrop: u32,
    /// `tcps_rcvnosec`: Packets dropped for lack of ipsec.
    pub tcps_rcvnosec: u32,
    /// `tcps_rcvshort`: Packets received too short.
    pub tcps_rcvshort: u32,
    /// `tcps_rcvduppack`: Duplicate-only packets received.
    pub tcps_rcvduppack: u32,
    /// `tcps_rcvdupbyte`: Duplicate-only bytes received.
    pub tcps_rcvdupbyte: u64,
    /// `tcps_rcvpartduppack`: Packets with some duplicate data.
    pub tcps_rcvpartduppack: u32,
    /// The C compiler's hole before `tcps_rcvpartdupbyte` (its 8-byte alignment), zero as the C's
    /// `memset` leaves it.
    pub _pad2: u32,
    /// `tcps_rcvpartdupbyte`: Dup. bytes in part-dup. packets.
    pub tcps_rcvpartdupbyte: u64,
    /// `tcps_rcvoopack`: Out-of-order packets received.
    pub tcps_rcvoopack: u32,
    /// The C compiler's hole before `tcps_rcvoobyte` (its 8-byte alignment), zero as the C's
    /// `memset` leaves it.
    pub _pad3: u32,
    /// `tcps_rcvoobyte`: Out-of-order bytes received.
    pub tcps_rcvoobyte: u64,
    /// `tcps_rcvpackafterwin`: Packets with data after window.
    pub tcps_rcvpackafterwin: u32,
    /// The C compiler's hole before `tcps_rcvbyteafterwin` (its 8-byte alignment), zero as the C's
    /// `memset` leaves it.
    pub _pad4: u32,
    /// `tcps_rcvbyteafterwin`: Bytes rcvd after window.
    pub tcps_rcvbyteafterwin: u64,
    /// `tcps_rcvafterclose`: Packets rcvd after "close".
    pub tcps_rcvafterclose: u32,
    /// `tcps_rcvwinprobe`: Rcvd window probe packets.
    pub tcps_rcvwinprobe: u32,
    /// `tcps_rcvdupack`: Rcvd duplicate acks.
    pub tcps_rcvdupack: u32,
    /// `tcps_rcvacktoomuch`: Rcvd acks for unsent data.
    pub tcps_rcvacktoomuch: u32,
    /// `tcps_rcvacktooold`: Rcvd acks for old data.
    pub tcps_rcvacktooold: u32,
    /// `tcps_rcvackpack`: Rcvd ack packets.
    pub tcps_rcvackpack: u32,
    /// `tcps_rcvackbyte`: Bytes acked by rcvd acks.
    pub tcps_rcvackbyte: u64,
    /// `tcps_rcvwinupd`: Rcvd window update packets.
    pub tcps_rcvwinupd: u32,
    /// `tcps_pawsdrop`: Segments dropped due to PAWS.
    pub tcps_pawsdrop: u32,
    /// `tcps_predack`: Times hdr predict ok for acks.
    pub tcps_predack: u32,
    /// `tcps_preddat`: Times hdr predict ok for data pkts.
    pub tcps_preddat: u32,
    /// `tcps_pcbhashmiss`: Input packets missing pcb hash.
    pub tcps_pcbhashmiss: u32,
    /// `tcps_noport`: No socket on port.
    pub tcps_noport: u32,
    /// `tcps_closing`: Inpcb exists, socket is closing.
    pub tcps_closing: u32,
    /// `tcps_badsyn`: SYN packet with src==dst rcv'ed.
    pub tcps_badsyn: u32,
    /// `tcps_dropsyn`: SYN packet dropped.
    pub tcps_dropsyn: u32,
    /// `tcps_rcvbadsig`: Rcvd bad/missing TCP signatures.
    pub tcps_rcvbadsig: u32,
    /// `tcps_rcvgoodsig`: Rcvd good TCP signatures.
    pub tcps_rcvgoodsig: u64,
    /// `tcps_inswcsum`: Input software-checksummed pkts.
    pub tcps_inswcsum: u32,
    /// `tcps_outswcsum`: Output software-checksummed pkts.
    pub tcps_outswcsum: u32,
    // ECN stats
    /// `tcps_ecn_accepts`: Ecn connections accepted.
    pub tcps_ecn_accepts: u32,
    /// `tcps_ecn_rcvece`: # of rcvd ece.
    pub tcps_ecn_rcvece: u32,
    /// `tcps_ecn_rcvcwr`: # of rcvd cwr.
    pub tcps_ecn_rcvcwr: u32,
    /// `tcps_ecn_rcvce`: # of rcvd ce in ip header.
    pub tcps_ecn_rcvce: u32,
    /// `tcps_ecn_sndect`: # of cwr sent.
    pub tcps_ecn_sndect: u32,
    /// `tcps_ecn_sndece`: # of ece sent.
    pub tcps_ecn_sndece: u32,
    /// `tcps_ecn_sndcwr`: # of cwr sent.
    pub tcps_ecn_sndcwr: u32,
    /// `tcps_cwr_ecn`: # of cwnd reduced by ecn.
    pub tcps_cwr_ecn: u32,
    /// `tcps_cwr_frecovery`: # of cwnd reduced by fastrecovery.
    pub tcps_cwr_frecovery: u32,
    /// `tcps_cwr_timeout`: # of cwnd reduced by timeout.
    pub tcps_cwr_timeout: u32,
    // These statistics deal with the SYN cache.
    /// `tcps_sc_added`: # of entries added.
    pub tcps_sc_added: u64,
    /// `tcps_sc_completed`: # of connections completed.
    pub tcps_sc_completed: u64,
    /// `tcps_sc_timed_out`: # of entries timed out.
    pub tcps_sc_timed_out: u64,
    /// `tcps_sc_overflowed`: # dropped due to overflow.
    pub tcps_sc_overflowed: u64,
    /// `tcps_sc_reset`: # dropped due to RST.
    pub tcps_sc_reset: u64,
    /// `tcps_sc_unreach`: # dropped due to ICMP unreach.
    pub tcps_sc_unreach: u64,
    /// `tcps_sc_bucketoverflow`: # dropped due to bucket overflow.
    pub tcps_sc_bucketoverflow: u64,
    /// `tcps_sc_aborted`: # of entries aborted (no mem).
    pub tcps_sc_aborted: u64,
    /// `tcps_sc_dupesyn`: # of duplicate SYNs received.
    pub tcps_sc_dupesyn: u64,
    /// `tcps_sc_dropped`: # of SYNs dropped (no route/mem).
    pub tcps_sc_dropped: u64,
    /// `tcps_sc_collisions`: # of hash collisions.
    pub tcps_sc_collisions: u64,
    /// `tcps_sc_retransmitted`: # of retransmissions.
    pub tcps_sc_retransmitted: u64,
    /// `tcps_sc_seedrandom`: # of syn cache seeds with random.
    pub tcps_sc_seedrandom: u64,
    /// `tcps_sc_hash_size`: Hash buckets in current syn cache.
    pub tcps_sc_hash_size: u64,
    /// `tcps_sc_entry_count`: # of entries in current syn cache.
    pub tcps_sc_entry_count: u64,
    /// `tcps_sc_entry_limit`: Limit of syn cache entries.
    pub tcps_sc_entry_limit: u64,
    /// `tcps_sc_bucket_maxlen`: Maximum # of entries in any bucket.
    pub tcps_sc_bucket_maxlen: u64,
    /// `tcps_sc_bucket_limit`: Limit of syn cache bucket list.
    pub tcps_sc_bucket_limit: u64,
    /// `tcps_sc_uses_left`: Use counter of current syn cache.
    pub tcps_sc_uses_left: i64,
    /// `tcps_conndrained`: # of connections drained.
    pub tcps_conndrained: u64,
    /// `tcps_sack_recovery_episode`: SACK recovery episodes.
    pub tcps_sack_recovery_episode: u64,
    /// `tcps_sack_rexmits`: SACK rexmit segments.
    pub tcps_sack_rexmits: u64,
    /// `tcps_sack_rexmit_bytes`: SACK rexmit bytes.
    pub tcps_sack_rexmit_bytes: u64,
    /// `tcps_sack_rcv_opts`: SACK options received.
    pub tcps_sack_rcv_opts: u64,
    /// `tcps_sack_snd_opts`: SACK options sent.
    pub tcps_sack_snd_opts: u64,
    /// `tcps_sack_drop_opts`: SACK options dropped.
    pub tcps_sack_drop_opts: u64,
    /// `tcps_outswtso`: Output tso chopped in software.
    pub tcps_outswtso: u32,
    /// `tcps_outhwtso`: Output tso processed by hardware.
    pub tcps_outhwtso: u32,
    /// `tcps_outpkttso`: Packets generated by tso.
    pub tcps_outpkttso: u32,
    /// `tcps_outbadtso`: Output tso failed, packet dropped.
    pub tcps_outbadtso: u32,
    /// `tcps_inswlro`: Input lro on pseudo device.
    pub tcps_inswlro: u32,
    /// `tcps_inhwlro`: Input lro from hardware.
    pub tcps_inhwlro: u32,
    /// `tcps_inpktlro`: Packets coalesced by hardware lro.
    pub tcps_inpktlro: u32,
    /// `tcps_inbadlro`: Input bad lro packets.
    pub tcps_inbadlro: u32,
}

crate::abi_pod!(Tcpstat {
    tcps_connattempt: u32,
    tcps_accepts: u32,
    tcps_connects: u32,
    tcps_drops: u32,
    tcps_conndrops: u32,
    tcps_closed: u32,
    tcps_segstimed: u32,
    tcps_rttupdated: u32,
    tcps_delack: u32,
    tcps_timeoutdrop: u32,
    tcps_rexmttimeo: u32,
    tcps_persisttimeo: u32,
    tcps_persistdrop: u32,
    tcps_keeptimeo: u32,
    tcps_keepprobe: u32,
    tcps_keepdrops: u32,
    tcps_sndtotal: u32,
    tcps_sndpack: u32,
    tcps_sndbyte: u64,
    tcps_sndrexmitpack: u32,
    _pad0: u32,
    tcps_sndrexmitbyte: u64,
    tcps_sndrexmitfast: u64,
    tcps_sndacks: u32,
    tcps_sndprobe: u32,
    tcps_sndurg: u32,
    tcps_sndwinup: u32,
    tcps_sndctrl: u32,
    tcps_rcvtotal: u32,
    tcps_rcvpack: u32,
    _pad1: u32,
    tcps_rcvbyte: u64,
    tcps_rcvbadsum: u32,
    tcps_rcvbadoff: u32,
    tcps_rcvmemdrop: u32,
    tcps_rcvnosec: u32,
    tcps_rcvshort: u32,
    tcps_rcvduppack: u32,
    tcps_rcvdupbyte: u64,
    tcps_rcvpartduppack: u32,
    _pad2: u32,
    tcps_rcvpartdupbyte: u64,
    tcps_rcvoopack: u32,
    _pad3: u32,
    tcps_rcvoobyte: u64,
    tcps_rcvpackafterwin: u32,
    _pad4: u32,
    tcps_rcvbyteafterwin: u64,
    tcps_rcvafterclose: u32,
    tcps_rcvwinprobe: u32,
    tcps_rcvdupack: u32,
    tcps_rcvacktoomuch: u32,
    tcps_rcvacktooold: u32,
    tcps_rcvackpack: u32,
    tcps_rcvackbyte: u64,
    tcps_rcvwinupd: u32,
    tcps_pawsdrop: u32,
    tcps_predack: u32,
    tcps_preddat: u32,
    tcps_pcbhashmiss: u32,
    tcps_noport: u32,
    tcps_closing: u32,
    tcps_badsyn: u32,
    tcps_dropsyn: u32,
    tcps_rcvbadsig: u32,
    tcps_rcvgoodsig: u64,
    tcps_inswcsum: u32,
    tcps_outswcsum: u32,
    tcps_ecn_accepts: u32,
    tcps_ecn_rcvece: u32,
    tcps_ecn_rcvcwr: u32,
    tcps_ecn_rcvce: u32,
    tcps_ecn_sndect: u32,
    tcps_ecn_sndece: u32,
    tcps_ecn_sndcwr: u32,
    tcps_cwr_ecn: u32,
    tcps_cwr_frecovery: u32,
    tcps_cwr_timeout: u32,
    tcps_sc_added: u64,
    tcps_sc_completed: u64,
    tcps_sc_timed_out: u64,
    tcps_sc_overflowed: u64,
    tcps_sc_reset: u64,
    tcps_sc_unreach: u64,
    tcps_sc_bucketoverflow: u64,
    tcps_sc_aborted: u64,
    tcps_sc_dupesyn: u64,
    tcps_sc_dropped: u64,
    tcps_sc_collisions: u64,
    tcps_sc_retransmitted: u64,
    tcps_sc_seedrandom: u64,
    tcps_sc_hash_size: u64,
    tcps_sc_entry_count: u64,
    tcps_sc_entry_limit: u64,
    tcps_sc_bucket_maxlen: u64,
    tcps_sc_bucket_limit: u64,
    tcps_sc_uses_left: i64,
    tcps_conndrained: u64,
    tcps_sack_recovery_episode: u64,
    tcps_sack_rexmits: u64,
    tcps_sack_rexmit_bytes: u64,
    tcps_sack_rcv_opts: u64,
    tcps_sack_snd_opts: u64,
    tcps_sack_drop_opts: u64,
    tcps_outswtso: u32,
    tcps_outhwtso: u32,
    tcps_outpkttso: u32,
    tcps_outbadtso: u32,
    tcps_inswlro: u32,
    tcps_inhwlro: u32,
    tcps_inpktlro: u32,
    tcps_inbadlro: u32,
});

/// `struct tcp_ident_mapping`: the `net.inet.tcp.ident` and `drop` request.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TcpIdentMapping {
    /// `faddr`.
    pub faddr: SockaddrStorage,
    /// `laddr`.
    pub laddr: SockaddrStorage,
    /// `euid`.
    pub euid: i32,
    /// `ruid`.
    pub ruid: i32,
    /// `rdomain`.
    pub rdomain: u32,
    /// The C compiler's trailing padding (the storage is 8-byte aligned).
    pub _pad: u32,
}

crate::abi_pod!(TcpIdentMapping {
    faddr: SockaddrStorage,
    laddr: SockaddrStorage,
    euid: i32,
    ruid: i32,
    rdomain: u32,
    _pad: u32,
});

impl TcpIdentMapping {
    /// An all-zero mapping.
    pub const fn zeroed() -> Self {
        Self {
            faddr: SockaddrStorage::zeroed(),
            laddr: SockaddrStorage::zeroed(),
            euid: 0,
            ruid: 0,
            rdomain: 0,
            _pad: 0,
        }
    }
}

// The sizes and offsets the C compiler gives these structures for `aarch64-unknown-openbsd`
// (clang's `_Static_assert`s over `reference/openbsd-src`, N6.6a0).
const _: () = {
    assert!(size_of::<TcpIdentMapping>() == 528);
    assert!(core::mem::offset_of!(TcpIdentMapping, laddr) == 256);
    assert!(core::mem::offset_of!(TcpIdentMapping, euid) == 512);
    assert!(core::mem::offset_of!(TcpIdentMapping, rdomain) == 520);
    assert!(align_of::<TcpIdentMapping>() == 8);
    assert!(core::mem::offset_of!(Tcpstat, tcps_sndbyte) == 72);
    assert!(core::mem::offset_of!(Tcpstat, tcps_sc_added) == 352);
    assert!(core::mem::offset_of!(Tcpstat, tcps_conndrained) == 504);
    assert!(core::mem::offset_of!(Tcpstat, tcps_outswtso) == 560);
    assert!(core::mem::offset_of!(Tcpstat, tcps_inbadlro) == 588);
    assert!(size_of::<Tcpstat>() == 592);
    assert!(align_of::<Tcpstat>() == 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::reftest::{assert_complete, assert_defines};

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::base::reftest::defines("sys/netinet/tcp_var.h");
        let mut ctl = assert_defines!(defs;
        TCPCTL_RFC1323, TCPCTL_KEEPINITTIME, TCPCTL_KEEPIDLE, TCPCTL_KEEPINTVL, TCPCTL_SLOWHZ,
        TCPCTL_BADDYNAMIC, TCPCTL_RECVSPACE, TCPCTL_SENDSPACE, TCPCTL_IDENT, TCPCTL_SACK,
        TCPCTL_MSSDFLT, TCPCTL_RSTPPSLIMIT, TCPCTL_ACK_ON_PUSH, TCPCTL_ECN,
        TCPCTL_SYN_CACHE_LIMIT, TCPCTL_SYN_BUCKET_LIMIT, TCPCTL_RFC3390, TCPCTL_REASS_LIMIT,
        TCPCTL_DROP, TCPCTL_SACKHOLE_LIMIT, TCPCTL_STATS, TCPCTL_ALWAYS_KEEPALIVE,
        TCPCTL_SYN_USE_LIMIT, TCPCTL_ROOTONLY, TCPCTL_SYN_HASH_SIZE, TCPCTL_TSO, TCPCTL_MAXID);
        // `TCPCTL_NAMES` is sysctl(8)'s table, not a value.
        ctl.push("TCPCTL_NAMES");
        assert_complete(&defs, "TCPCTL_", &ctl);
    }
}
/* </TESTS> */
