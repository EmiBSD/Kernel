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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993, 1994
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
//! Software large receive offload: `tcp_input.c`'s `tcp_softlro_glue` and its helpers
//! (N6.6a1).
//!
//! Upstream: sys/netinet/tcp_input.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_input.rs@3c62ede68802
//!
//! A driver's receive path hands each packet of an interface with `IFXF_LRO` to
//! [`tcp_softlro_glue`] before it queues the packet on its list: a TCP segment that continues
//! a segment already on the list (same VLAN, addresses and ports, the next sequence number,
//! only ACK and PUSH, no options or the RFC 1323 appendix A timestamp) is
//! appended to that packet, which becomes one TSO packet (`M_TCP_TSO`, its largest segment in
//! `ph_mss`) whose headers say the merged length; anything else is queued as it came.
//!
//! The timestamp test is the C's as written: `TSTMP_LT(head, tail)` refuses a tail whose
//! timestamps are newer than the head's, though its comment reads "Tail timestamps must be
//! more recent"; a run of segments with equal timestamps (one burst) merges. Kept, as
//! behaviour is the C's (the slip goes to the report, not into the code).
//!
//! ## Deviations
//! - `tcp_softlro_compare` refuses a head without a TCP header (the C dereferences it; only a
//!   segment that passed `tcp_softlro_check` has a non-zero `ph_mss`, so it is found), as LZ.
//! - A head whose IP or TCP header lies in a read-only (shared) cluster is not merged into:
//!   the tail is queued as it came, the head untouched (the C writes into it in place). A
//!   driver's receive cluster is never shared, so no packet of the C's callers takes this
//!   way.
//! - `TSTMP_LT` (`tcp_input.c`'s macro) is [`tstmp_lt`].
//! - `SMALL_KERNEL` is not set, so the soft LRO is compiled. [`tcp_softlro_glue`] has no
//!   caller yet: its drivers (`ixl`, `ice`, `bnxt`, `cnmac`) are not ported, as in LZ.
//!
//! ## Redesign
//! - LZ read and wrote the headers through the raw pointers of its `struct ether_extracted`
//!   (three `unsafe fn`s and fifteen blocks); the kernel's `EtherExtracted` holds the headers'
//!   offsets in the packet, each header inside one mbuf (`ether_extract_headers`), so the
//!   headers are slices of that mbuf's data (`m_getptr`, `data`/`data_mut`) read and written
//!   by field offset, the C's in-place stores without a copy of the header.
//! - LZ walked the list by `m_nextpkt` over parked mbufs and changed the head through a
//!   shared reference. Here the list is the kernel's `MbufList` (owned packets): the walk is
//!   a cursor kept on the predecessor of the packet it looks at, O(1) a step as the C's; the
//!   packet that matches is taken off where it stands (`remove_next`, or `pop_head` for the
//!   first), changed through ownership and put back in its place (`split_after`, `push_tail`,
//!   `concat`, or `push_head`), O(1) each, as `ip_input`'s reassembly queue does
//!   (docs/COLLECTIONS.md).

use core::cmp::max;
use core::mem::size_of;
use core::sync::atomic::Ordering;

use hal::sys::net_if::IFXF_LRO;
use hal::sys::netinet_if_ether::{ETHER_HDR_LEN, EtherVlanHeader, evl_prioftag, evl_vlanoftag};
use hal::sys::netinet_ip::{IP_MAXPACKET, Ip};
use hal::sys::netinet_ip6::IPV6_MAXPACKET;
use hal::sys::netinet_tcp::{
    TCPOLEN_TSTAMP_APPA, TCPOPT_EOL, TCPOPT_TSTAMP_HDR, TH_ACK, TH_PUSH, Tcphdr,
};

use crate::assert::kpanic;
use crate::net::if_ethersubr::{EtherExtracted, ether_extract_headers};
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{
    M_FLOWID, M_IPV4_CSUM_IN_OK, M_IPV4_CSUM_OUT, M_TCP_CSUM_IN_OK, M_TCP_CSUM_OUT, M_TCP_TSO,
    M_VLANTAG, MAX_LINKHDR, Mbuf, MbufBox, MbufList, m_adj, m_getptr, m_getptr_mut, m_removehdr,
    ml_enqueue,
};
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_inc};

/// The TCP header's length without options.
const TCP_HDR_LEN: usize = size_of::<Tcphdr>();
/// An IPv4 header's length without options.
const IP_HDR_LEN: usize = size_of::<Ip>();

// Offsets of the fields the soft LRO reads and writes, in their headers.
/// `ip_len` in `struct ip`.
const IP_LEN_OFF: usize = 2;
/// `ip_sum` in `struct ip`.
const IP_SUM_OFF: usize = 10;
/// `ip_src` and `ip_dst` in `struct ip`.
const IP_ADDRS: core::ops::Range<usize> = 12..20;
/// `ip6_plen` in `struct ip6_hdr`.
const IP6_PLEN_OFF: usize = 4;
/// `ip6_src` and `ip6_dst` in `struct ip6_hdr`.
const IP6_ADDRS: core::ops::Range<usize> = 8..40;
/// `th_sport` and `th_dport` in `struct tcphdr`.
const TH_PORTS: core::ops::Range<usize> = 0..4;
/// `th_seq`.
const TH_SEQ_OFF: usize = 4;
/// `th_ack`.
const TH_ACK_OFF: usize = 8;
/// `th_flags`.
const TH_FLAGS_OFF: usize = 13;
/// `th_win`.
const TH_WIN_OFF: usize = 14;
/// `th_sum`.
const TH_SUM_OFF: usize = 16;
/// The two timestamps of the appendix A option (`(uint32_t *)(th + 1)` words 1 and 2).
const TH_TSTAMPS: core::ops::Range<usize> = TCP_HDR_LEN + 4..TCP_HDR_LEN + 12;
/// `evl_tag` in `struct ether_vlan_header`.
const EVL_TAG_OFF: usize = 14;

/// `TSTMP_LT(a, b)`: timestamp `a` is older than `b`, modulo 2^32.
#[inline]
pub(crate) const fn tstmp_lt(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) < 0
}

/// The `len` bytes of a header at offset `off` of packet `m`: inside one mbuf, as
/// `ether_extract_headers` found it; `None` when they are not.
fn hdr(m: &Mbuf, off: u32, len: usize) -> Option<&[u8]> {
    let (n, o) = m_getptr(m, off as i32)?;
    n.data().get(o as usize..o as usize + len)
}

/// [`hdr`], to write it: `None` also when the mbuf that holds it is read-only.
fn hdr_mut(m: &mut Mbuf, off: u32, len: usize) -> Option<&mut [u8]> {
    let (n, o) = m_getptr_mut(m, off as i32)?;
    n.data_mut()?.get_mut(o as usize..o as usize + len)
}

/// The big-endian word at `at` of `b` (the C's `ntohl` of a member).
fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// `max_linkhdr`, as the C's `int` subtrahend of an unsigned length.
fn max_linkhdr() -> usize {
    MAX_LINKHDR.load(Ordering::Relaxed) as usize
}

/// `tcp_softlro_check(m, ext)`: whether the segment in `m`, whose headers `ext` describes,
/// may be merged at all.
fn tcp_softlro_check(m: &Mbuf, ext: &EtherExtracted) -> bool {
    let csum_flags = m.m_pkthdr().csum_flags.get();

    // Don't merge packets with invalid TCP checksum.
    if csum_flags & M_TCP_CSUM_IN_OK == 0 {
        return false;
    }

    if ext.ip4.is_some() {
        // Don't merge packets with invalid IP header checksum.
        if csum_flags & M_IPV4_CSUM_IN_OK == 0 {
            return false;
        }

        // Don't merge IPv4 packets with IP options.
        if ext.iphlen as usize != IP_HDR_LEN {
            return false;
        }
    }

    // Check TCP protocol and header.
    let Some(toff) = ext.tcp else {
        return false;
    };
    let Some(th) = hdr(m, toff, ext.tcphlen as usize) else {
        return false;
    };

    // Don't merge empty TCP segments.
    if ext.paylen == 0 {
        return false;
    }

    // Just ACK and PUSH TCP flags are allowed.
    let flags = th[TH_FLAGS_OFF];
    if flags & (TH_ACK | TH_PUSH) != flags {
        return false;
    }

    // TCP ACK flag has to be set.
    if flags & TH_ACK == 0 {
        return false;
    }

    // Either no TCP options or timestamp as in RFC 1323 appendix A.
    if ext.tcphlen as usize > TCP_HDR_LEN {
        let opt = &th[TCP_HDR_LEN..];
        let appa = usize::from(TCPOLEN_TSTAMP_APPA);

        // Same logic as in TCP input quick retrieval.
        if (opt.len() != appa && (opt.len() <= appa || opt[appa] != TCPOPT_EOL))
            || opt[..4] != TCPOPT_TSTAMP_HDR.to_be_bytes()
        {
            return false;
        }
    }

    true
}

/// `tcp_softlro_compare(head, tail)`: whether the segment `tail` (in `mt`) continues `head`
/// (in `mh`): same VLAN, ports, addresses and options, contiguous sequence numbers, not too
/// long together, timestamps not newer than the head's (the C's test, the module's doc).
fn tcp_softlro_compare(mh: &Mbuf, head: &EtherExtracted, mt: &Mbuf, tail: &EtherExtracted) -> bool {
    // Don't merge packets inside and outside of VLANs
    match (head.evh, tail.evh) {
        (Some(h), Some(t)) => {
            let vlen = size_of::<EtherVlanHeader>();
            let (Some(hv), Some(tv)) = (hdr(mh, h, vlen), hdr(mt, t, vlen)) else {
                return false;
            };
            // The tags as the C's member reads them, in memory order.
            let htag = u16::from_ne_bytes([hv[EVL_TAG_OFF], hv[EVL_TAG_OFF + 1]]);
            let ttag = u16::from_ne_bytes([tv[EVL_TAG_OFF], tv[EVL_TAG_OFF + 1]]);

            // Don't merge packets of different VLANs
            if evl_vlanoftag(htag) != evl_vlanoftag(ttag) {
                return false;
            }

            // Don't merge packets of different priorities
            if evl_prioftag(htag) != evl_prioftag(ttag) {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    let (Some(ht), Some(tt)) = (head.tcp, tail.tcp) else {
        return false;
    };
    let (Some(hth), Some(tth)) = (
        hdr(mh, ht, head.tcphlen as usize),
        hdr(mt, tt, tail.tcphlen as usize),
    ) else {
        return false;
    };

    // Check TCP ports.
    if hth[TH_PORTS] != tth[TH_PORTS] {
        return false;
    }

    // Check IP header.
    match (head.ip4, tail.ip4, head.ip6, tail.ip6) {
        (Some(h), Some(t), _, _) => {
            let (Some(hip), Some(tip)) = (hdr(mh, h, IP_HDR_LEN), hdr(mt, t, IP_HDR_LEN)) else {
                return false;
            };
            // Check IPv4 addresses.
            if hip[IP_ADDRS] != tip[IP_ADDRS] {
                return false;
            }

            // Check max. IPv4 length.
            if (head.iplen + tail.iplen) as usize > IP_MAXPACKET.wrapping_sub(max_linkhdr()) {
                return false;
            }
        }
        (_, _, Some(h), Some(t)) => {
            let ip6len = IP6_ADDRS.end;
            let (Some(hip6), Some(tip6)) = (hdr(mh, h, ip6len), hdr(mt, t, ip6len)) else {
                return false;
            };
            // Check IPv6 addresses.
            if hip6[IP6_ADDRS] != tip6[IP6_ADDRS] {
                return false;
            }

            // Check max. IPv6 length.
            if ((head.iplen - head.iphlen) + (tail.iplen - tail.iphlen)) as usize
                > IPV6_MAXPACKET.wrapping_sub(max_linkhdr())
            {
                return false;
            }
        }
        // Address family does not match.
        _ => return false,
    }

    // Check for contiguous segments.
    if be32(hth, TH_SEQ_OFF).wrapping_add(head.paylen) != be32(tth, TH_SEQ_OFF) {
        return false;
    }

    // Ignore segments with different TCP options.
    if head.tcphlen != tail.tcphlen {
        return false;
    }

    // TCP timestamp options must match, type and length already checked.
    if head.tcphlen as usize > TCP_HDR_LEN {
        let (h1, t1) = (be32(hth, TH_TSTAMPS.start), be32(tth, TH_TSTAMPS.start));
        let (h2, t2) = (
            be32(hth, TH_TSTAMPS.start + 4),
            be32(tth, TH_TSTAMPS.start + 4),
        );

        // Tail timestamps must be more recent.
        if tstmp_lt(h1, t1) || tstmp_lt(h2, t2) {
            return false;
        }
    }

    true
}

/// Whether the headers of `head` that [`tcp_softlro_concat`] rewrites can be written in
/// place (the module's deviations).
fn tcp_softlro_writable(mh: &Mbuf, head: &EtherExtracted) -> bool {
    let writable = |off: Option<u32>| {
        off.is_none_or(|o| m_getptr(mh, o as i32).is_some_and(|(n, _)| !n.is_readonly()))
    };
    writable(head.ip4) && writable(head.ip6) && writable(head.tcp)
}

/// `tcp_softlro_concat(mhead, head, mtail, tail)`: appends the payload of `mtail` to `mhead`
/// and fixes up the head's headers and offload flags. `Err` gives `mtail` back, the head
/// untouched, when a header of the head cannot be written in place
/// ([`tcp_softlro_writable`], asked before the first write).
fn tcp_softlro_concat(
    mhead: &mut Mbuf,
    head: &EtherExtracted,
    mtail: MbufBox,
    tail: &EtherExtracted,
) -> Result<(), MbufBox> {
    let mut mtail = mtail;
    let (Some(ht), Some(tt)) = (head.tcp, tail.tcp) else {
        return Err(mtail);
    };
    if !tcp_softlro_writable(mhead, head) {
        return Err(mtail);
    }
    let thlen = head.tcphlen as usize;

    // What the head takes of the tail's TCP header: flags, window, acknowledgement and the
    // timestamps.
    let Some(tth) = hdr(&mtail, tt, thlen) else {
        return Err(mtail);
    };
    let tflags = tth[TH_FLAGS_OFF];
    let mut twin = [0u8; 2];
    twin.copy_from_slice(&tth[TH_WIN_OFF..TH_WIN_OFF + 2]);
    let mut tack = [0u8; 4];
    tack.copy_from_slice(&tth[TH_ACK_OFF..TH_ACK_OFF + 4]);
    let mut tstamps = [0u8; 8];
    if thlen > TCP_HDR_LEN {
        tstamps.copy_from_slice(&tth[TH_TSTAMPS]);
    }

    let tso = mhead.m_pkthdr().csum_flags.get() & M_TCP_TSO != 0;

    // Adjust IP header length.
    if let Some(o) = head.ip4 {
        let Some(ip) = hdr_mut(mhead, o, IP_HDR_LEN) else {
            return Err(mtail);
        };
        let len = (head.iplen + tail.paylen) as u16;
        ip[IP_LEN_OFF..IP_LEN_OFF + 2].copy_from_slice(&len.to_be_bytes());
        if !tso {
            ip[IP_SUM_OFF..IP_SUM_OFF + 2].fill(0);
        }
    } else if let Some(o) = head.ip6 {
        let Some(ip6) = hdr_mut(mhead, o, IP6_ADDRS.end) else {
            return Err(mtail);
        };
        let plen = (head.iplen - head.iphlen + tail.paylen) as u16;
        ip6[IP6_PLEN_OFF..IP6_PLEN_OFF + 2].copy_from_slice(&plen.to_be_bytes());
    }

    let Some(hth) = hdr_mut(mhead, ht, thlen) else {
        // Not reached: tcp_softlro_writable found the header's mbuf writable above.
        return Err(mtail);
    };
    // Combine TCP flags from head and tail.
    if tflags & TH_PUSH != 0 {
        hth[TH_FLAGS_OFF] |= TH_PUSH;
    }

    // Adjust TCP header.
    hth[TH_WIN_OFF..TH_WIN_OFF + 2].copy_from_slice(&twin);
    hth[TH_ACK_OFF..TH_ACK_OFF + 4].copy_from_slice(&tack);

    // Use more recent timestamps from tail.
    if thlen > TCP_HDR_LEN {
        hth[TH_TSTAMPS].copy_from_slice(&tstamps);
    }
    if !tso {
        hth[TH_SUM_OFF..TH_SUM_OFF + 2].fill(0);
    }

    // Calculate header length of tail packet.
    let mut hdrlen = ETHER_HDR_LEN as u32;
    if tail.evh.is_some() {
        hdrlen = size_of::<EtherVlanHeader>() as u32;
    }
    hdrlen += tail.iphlen;
    hdrlen += tail.tcphlen;

    // Skip protocol headers in tail.
    m_adj(&mtail, hdrlen as i32);
    m_removehdr(&mut mtail);

    // Concatenate
    let mut m = &mut *mhead;
    while m.m_next.is_some() {
        m = match m.next_mut() {
            Some(next) => next,
            None => kpanic(format_args!(
                "tcp_softlro_concat: a link emptied under the walk"
            )),
        };
    }
    drop(m.m_next.set(mtail));
    let ph = mhead.m_pkthdr();
    ph.len.set(ph.len.get() + tail.paylen as i32);

    // Flag mbuf as TSO packet with MSS.
    if !tso {
        // Set CSUM_OUT flags in case of forwarding (the checksums were cleared above).
        ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_OUT);
        if head.ip4.is_some() {
            ph.csum_flags.set(ph.csum_flags.get() | M_IPV4_CSUM_OUT);
        }

        ph.csum_flags.set(ph.csum_flags.get() | M_TCP_TSO);
        ph.ph_mss.set(head.paylen as u16);
        tcpstat_inc(TcpstatCounters::TcpsInswlro);
        tcpstat_inc(TcpstatCounters::TcpsInpktlro); // count head
    }
    ph.ph_mss.set(max(ph.ph_mss.get(), tail.paylen as u16));
    tcpstat_inc(TcpstatCounters::TcpsInpktlro); // count tail
    Ok(())
}

/// The headers of `mhead` when the segment `mtail` (headers `tail`) may be appended to it:
/// the glue loop's tests before `tcp_softlro_compare`, and the compare.
fn tcp_softlro_head(mhead: &Mbuf, mtail: &Mbuf, tail: &EtherExtracted) -> Option<EtherExtracted> {
    let hph = mhead.m_pkthdr();
    let tph = mtail.m_pkthdr();

    // This packet has been checked and was not mergable before.
    if hph.ph_mss.get() == 0 {
        return None;
    }

    // Use RSS hash to skip packets of different connections.
    if hph.csum_flags.get() & M_FLOWID != 0
        && tph.csum_flags.get() & M_FLOWID != 0
        && hph.ph_flowid.get() != tph.ph_flowid.get()
    {
        return None;
    }

    // Don't merge packets inside and outside of VLANs
    let hvlan = mhead.flags() & M_VLANTAG != 0;
    if hvlan != (mtail.flags() & M_VLANTAG != 0) {
        return None;
    }

    if hvlan {
        // Don't merge packets of different VLANs
        if evl_vlanoftag(hph.ether_vtag.get()) != evl_vlanoftag(tph.ether_vtag.get()) {
            return None;
        }

        // Don't merge packets of different priorities
        if evl_prioftag(hph.ether_vtag.get()) != evl_prioftag(tph.ether_vtag.get()) {
            return None;
        }
    }

    let mut head = EtherExtracted::new();
    ether_extract_headers(mhead, &mut head);
    if !tcp_softlro_compare(mhead, &head, mtail, tail) {
        return None;
    }
    Some(head)
}

/// `tcp_softlro_glue(ml, mtail, ifp)`: a driver's receive path hands each packet `mtail` of
/// interface `ifp` here instead of queueing it on `ml`; with `IFXF_LRO` a TCP segment that
/// continues one already on the list is appended to it, else it is queued.
pub fn tcp_softlro_glue(ml: &mut MbufList, mtail: MbufBox, ifp: &Ifnet) {
    'dontmerge: {
        if ifp.if_xflags.get() & IFXF_LRO == 0 {
            break 'dontmerge;
        }

        mtail.m_pkthdr().ph_mss.set(0);

        let mut tail = EtherExtracted::new();
        ether_extract_headers(&mtail, &mut tail);

        if tail.tcp.is_some() {
            // Remove possible ethernet padding at the end.
            let tcpdatalen = tail
                .iplen
                .wrapping_sub(tail.iphlen)
                .wrapping_sub(tail.tcphlen);
            if tcpdatalen < tail.paylen {
                m_adj(&mtail, tcpdatalen.wrapping_sub(tail.paylen) as i32);
                tail.paylen = tcpdatalen;
            }
        }

        if !tcp_softlro_check(&mtail, &tail) {
            break 'dontmerge;
        }

        mtail.m_pkthdr().ph_mss.set(tail.paylen as u16);

        // The list's first packet, taken off and put back at the head.
        if let Some(head) = ml.first().and_then(|h| tcp_softlro_head(h, &mtail, &tail)) {
            let Some(mut h) = ml.pop_head() else {
                break 'dontmerge;
            };
            let refused = tcp_softlro_concat(&mut h, &head, mtail, &tail).err();
            ml.push_head(h);
            if let Some(m) = refused {
                ml_enqueue(ml, m);
            }
            return;
        }

        // The others: the cursor stays on the predecessor of the packet looked at.
        let mut c = ml.cursor_mut();
        let found = loop {
            let Some(next) = c.current().and_then(MbufList::next) else {
                break None;
            };
            if let Some(head) = tcp_softlro_head(next, &mtail, &tail) {
                break Some(head);
            }
            c.move_next();
        };
        let Some(head) = found else {
            break 'dontmerge;
        };
        let Some(mut h) = c.remove_next() else {
            break 'dontmerge;
        };
        let refused = tcp_softlro_concat(&mut h, &head, mtail, &tail).err();
        // Back after its predecessor, where it stood.
        let mut rest = c.split_after();
        ml.push_tail(h);
        ml.concat(&mut rest);
        if let Some(m) = refused {
            ml_enqueue(ml, m);
        }
        return;
    }
    // dontmerge:
    ml_enqueue(ml, mtail);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The soft LRO over Ethernet frames of the kernel's mbufs: a run of contiguous segments
    // becomes one TSO packet with the merged headers and the counters; each of the C's
    // refusals keeps a packet apart; a merge in the middle of the list keeps the list's
    // order; software TSO's segments glued back give the original stream.

    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use hal::sys::net_ethertypes::{ETHERTYPE_IP, ETHERTYPE_IPV6};
    use hal::sys::netinet_in::IPPROTO_TCP;
    use hal::sys::netinet_tcp::{TCPOLEN_TIMESTAMP, TCPOPT_NOP, TCPOPT_TIMESTAMP, TH_SYN};

    use super::*;
    use crate::net::if_::test_ifnet;
    use hal::sys::mbuf::MT_DATA;

    use crate::net::mbuf::{
        M_COPYALL, M_DONTWAIT, MCLBYTES, m_clget, m_clget_hdr, m_copydata, m_copym, m_get,
        m_gethdr, ml_dequeue, ml_len,
    };
    use crate::netinet::tcp_var::TCPCOUNTERS;

    const SRC: [u8; 4] = [10, 0, 0, 1];
    const DST: [u8; 4] = [10, 0, 0, 2];

    /// The interface layer's fixture, and the IP id permutation software TSO draws from.
    fn setup() -> MutexGuard<'static, ()> {
        let g = crate::net::if_::tests::setup();
        crate::netinet::ip_id::ip_randomid_init();
        g
    }

    /// A segment's description: version, sequence number, payload length, flags,
    /// timestamps (none: no options), source port.
    #[derive(Clone, Copy)]
    struct Seg {
        v: u8,
        seq: u32,
        len: usize,
        flags: u8,
        ts: Option<(u32, u32)>,
        sport: u16,
    }

    impl Seg {
        const fn new(seq: u32, len: usize) -> Self {
            Seg {
                v: 4,
                seq,
                len,
                flags: TH_ACK,
                ts: Some((100, 50)),
                sport: 1234,
            }
        }
    }

    /// The frame of `s`: an Ethernet header, the IP header, the TCP header (with the
    /// appendix A timestamp when `s.ts`), and `s.len` bytes whose value is their stream
    /// position (`seq` on); the receive checksum flags set.
    fn frame(s: Seg) -> MbufBox {
        let thlen = if s.ts.is_some() { 32 } else { 20 };
        let mut p = vec![0u8; 12];
        if s.v == 4 {
            p.extend_from_slice(&ETHERTYPE_IP.to_be_bytes());
            p.extend_from_slice(&[0x45, 0]);
            p.extend_from_slice(&((20 + thlen + s.len) as u16).to_be_bytes());
            p.extend_from_slice(&[0, 0, 0x40, 0, 64, IPPROTO_TCP as u8, 0xab, 0xcd]);
            p.extend_from_slice(&SRC);
            p.extend_from_slice(&DST);
        } else {
            p.extend_from_slice(&ETHERTYPE_IPV6.to_be_bytes());
            p.extend_from_slice(&[0x60, 0, 0, 0]);
            p.extend_from_slice(&((thlen + s.len) as u16).to_be_bytes());
            p.extend_from_slice(&[IPPROTO_TCP as u8, 64]);
            p.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
            p.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        }
        p.extend_from_slice(&s.sport.to_be_bytes());
        p.extend_from_slice(&80u16.to_be_bytes());
        p.extend_from_slice(&s.seq.to_be_bytes());
        p.extend_from_slice(&(s.seq / 2).to_be_bytes()); // the ack moves with the segments
        p.extend_from_slice(&[((thlen / 4) as u8) << 4, s.flags]);
        p.extend_from_slice(&((s.seq & 0xffff) as u16).to_be_bytes()); // so does the window
        p.extend_from_slice(&[0x12, 0x34, 0, 0]);
        if let Some((a, b)) = s.ts {
            p.extend_from_slice(&[TCPOPT_NOP, TCPOPT_NOP, TCPOPT_TIMESTAMP, TCPOLEN_TIMESTAMP]);
            p.extend_from_slice(&a.to_be_bytes());
            p.extend_from_slice(&b.to_be_bytes());
        }
        p.extend((0..s.len).map(|i| s.seq.wrapping_add(i as u32) as u8));
        mbuf_of(&p)
    }

    /// A packet of `p`'s bytes with the receive checksum flags set.
    fn mbuf_of(p: &[u8]) -> MbufBox {
        let mut m = m_clget_hdr(M_DONTWAIT, MCLBYTES as u32).expect("cluster");
        m.m_len().set(p.len() as u32);
        m.data_mut().expect("writable")[..p.len()].copy_from_slice(p);
        m.m_pkthdr().len.set(p.len() as i32);
        m.m_pkthdr()
            .csum_flags
            .set(M_TCP_CSUM_IN_OK | M_IPV4_CSUM_IN_OK);
        m
    }

    /// The bytes of packet `m`.
    fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut v = vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(m, 0, &mut v);
        v
    }

    /// The list's packets, taken off it.
    fn drain(ml: &mut MbufList) -> Vec<MbufBox> {
        let mut v = Vec::new();
        while let Some(m) = ml_dequeue(ml) {
            v.push(m);
        }
        v
    }

    /// The counter `c`.
    fn count(c: TcpstatCounters) -> u64 {
        TCPCOUNTERS.read()[c as usize]
    }

    /// An interface with `IFXF_LRO`.
    fn lro_ifnet(name: &[u8]) -> &'static Ifnet {
        let (ifp, _) = test_ifnet(name);
        ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_LRO);
        ifp
    }

    #[test]
    fn contiguous_segments_become_one_tso_packet() {
        let _g = setup();
        let ifp = lro_ifnet(b"tlro0");

        for v in [4, 6] {
            let (sw, pkts) = (
                count(TcpstatCounters::TcpsInswlro),
                count(TcpstatCounters::TcpsInpktlro),
            );
            let mut ml = MbufList::new();
            let segs = [
                Seg {
                    v,
                    ..Seg::new(1000, 100)
                },
                Seg {
                    v,
                    ..Seg::new(1100, 300)
                },
                // Not newer than the head's: the C merges a tail whose timestamps are not
                // past the head's (TSTMP_LT(head, tail) refuses), and the head takes them.
                Seg {
                    v,
                    ts: Some((99, 49)),
                    flags: TH_ACK | TH_PUSH,
                    ..Seg::new(1400, 200)
                },
            ];
            for s in segs {
                tcp_softlro_glue(&mut ml, frame(s), ifp);
            }
            assert_eq!(ml_len(&ml), 1, "IPv{v}");
            let m = drain(&mut ml).pop().expect("the merged packet");
            let ph = m.m_pkthdr();
            assert_eq!(
                ph.csum_flags.get() & (M_TCP_TSO | M_TCP_CSUM_OUT),
                M_TCP_TSO | M_TCP_CSUM_OUT
            );
            assert_eq!(ph.ph_mss.get(), 300, "the largest segment");
            assert_eq!(count(TcpstatCounters::TcpsInswlro) - sw, 1);
            assert_eq!(count(TcpstatCounters::TcpsInpktlro) - pkts, 3);

            let b = bytes(&m);
            let (ip, iphlen) = (&b[14..], if v == 4 { 20 } else { 40 });
            let th = &ip[iphlen..];
            if v == 4 {
                assert_eq!(u16::from_be_bytes([ip[2], ip[3]]), 20 + 32 + 600);
                assert_eq!(&ip[10..12], &[0, 0], "ip_sum cleared");
                assert_ne!(ph.csum_flags.get() & M_IPV4_CSUM_OUT, 0);
            } else {
                assert_eq!(u16::from_be_bytes([ip[4], ip[5]]), 32 + 600);
                assert_eq!(ph.csum_flags.get() & M_IPV4_CSUM_OUT, 0);
            }
            assert_eq!(be32(th, TH_SEQ_OFF), 1000);
            assert_eq!(be32(th, TH_ACK_OFF), 1400 / 2, "the last segment's ack");
            assert_eq!(u16::from_be_bytes([th[14], th[15]]), 1400, "its window");
            assert_eq!(
                th[TH_FLAGS_OFF],
                TH_ACK | TH_PUSH,
                "PUSH taken from the tail"
            );
            assert_eq!(&th[16..18], &[0, 0], "th_sum cleared");
            assert_eq!((be32(th, 24), be32(th, 28)), (99, 49), "its timestamps");
            let data = &th[32..];
            assert_eq!(data.len(), 600);
            for (i, &x) in data.iter().enumerate() {
                assert_eq!(x, (1000 + i) as u8);
            }
        }
    }

    #[test]
    fn each_refusal_keeps_the_segment_apart() {
        let _g = setup();
        let ifp = lro_ifnet(b"tlro1");

        let base = Seg::new(5000, 100);
        let next = Seg::new(5100, 100);
        // (what the tail looks like, its frame changed by f)
        let cases: [(Seg, fn(&mut MbufBox)); 9] = [
            (Seg { seq: 5101, ..next }, |_| {}),  // not contiguous
            (Seg { sport: 999, ..next }, |_| {}), // another flow
            (
                Seg {
                    flags: TH_ACK | TH_SYN,
                    ..next
                },
                |_| {},
            ), // a flag other than ACK, PUSH
            (
                Seg {
                    flags: TH_PUSH,
                    ..next
                },
                |_| {},
            ), // no ACK
            (Seg { ts: None, ..next }, |_| {}),   // other options
            (
                Seg {
                    ts: Some((101, 50)),
                    ..next
                },
                |_| {},
            ), // a newer timestamp
            (Seg { v: 6, ..next }, |_| {}),       // another family
            (next, |m| m.m_pkthdr().csum_flags.set(M_IPV4_CSUM_IN_OK)), // bad TCP checksum
            (next, |m| m.m_pkthdr().csum_flags.set(M_TCP_CSUM_IN_OK)), // bad IP checksum
        ];
        for (k, (s, f)) in cases.into_iter().enumerate() {
            let mut ml = MbufList::new();
            tcp_softlro_glue(&mut ml, frame(base), ifp);
            let mut t = frame(s);
            f(&mut t);
            tcp_softlro_glue(&mut ml, t, ifp);
            let ms = drain(&mut ml);
            assert_eq!(ms.len(), 2, "case {k}");
            assert_eq!(ms[0].m_pkthdr().csum_flags.get() & M_TCP_TSO, 0, "case {k}");
            assert_eq!(bytes(&ms[0]).len(), 14 + 20 + 32 + 100, "case {k}");
        }

        // An empty segment is not merged, nor is anything merged into it.
        let mut ml = MbufList::new();
        tcp_softlro_glue(&mut ml, frame(Seg::new(5000, 0)), ifp);
        tcp_softlro_glue(&mut ml, frame(Seg::new(5000, 100)), ifp);
        assert_eq!(drain(&mut ml).len(), 2);

        // A head whose headers lie in a shared (read-only) cluster is not written into.
        let mut ml = MbufList::new();
        let mut head = frame(base);
        let share =
            m_copym(&mut head, 0, M_COPYALL, M_DONTWAIT).expect("a copy sharing the cluster");
        assert!(head.is_readonly());
        tcp_softlro_glue(&mut ml, head, ifp);
        tcp_softlro_glue(&mut ml, frame(next), ifp);
        assert_eq!(drain(&mut ml).len(), 2);
        drop(share);

        // The IP header writable, the TCP header in a shared cluster of the next mbuf: the
        // head is left whole (no length written before the refusal).
        let f = bytes(&frame(base));
        let mut head = m_gethdr(M_DONTWAIT, MT_DATA).expect("mbuf");
        head.m_len().set(34);
        head.data_mut().expect("writable")[..34].copy_from_slice(&f[..34]);
        let mut n = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        assert!(m_clget(&mut n, M_DONTWAIT, MCLBYTES as u32));
        n.m_len().set((f.len() - 34) as u32);
        n.data_mut().expect("writable")[..f.len() - 34].copy_from_slice(&f[34..]);
        let share = m_copym(&mut n, 0, M_COPYALL, M_DONTWAIT).expect("a copy sharing the cluster");
        drop(head.m_next.set(n));
        head.m_pkthdr().len.set(f.len() as i32);
        head.m_pkthdr()
            .csum_flags
            .set(M_TCP_CSUM_IN_OK | M_IPV4_CSUM_IN_OK);
        tcp_softlro_glue(&mut ml, head, ifp);
        tcp_softlro_glue(&mut ml, frame(next), ifp);
        let ms = drain(&mut ml);
        assert_eq!(ms.len(), 2);
        assert_eq!(bytes(&ms[0]), f, "the head untouched");
        drop(share);

        // An empty segment is not merged into a head that it continues.
        tcp_softlro_glue(&mut ml, frame(base), ifp);
        tcp_softlro_glue(&mut ml, frame(Seg::new(5100, 0)), ifp);
        assert_eq!(drain(&mut ml).len(), 2);

        // An interface without IFXF_LRO queues every packet as it came.
        let (plain, _) = test_ifnet(b"tlro2");
        let mut ml = MbufList::new();
        tcp_softlro_glue(&mut ml, frame(base), plain);
        tcp_softlro_glue(&mut ml, frame(next), plain);
        assert_eq!(drain(&mut ml).len(), 2);
    }

    #[test]
    fn a_merge_in_the_middle_keeps_the_order() {
        let _g = setup();
        let ifp = lro_ifnet(b"tlro3");
        let mut ml = MbufList::new();

        // Flow A, flow B, flow C; then B's next segment: B grows where it stands.
        tcp_softlro_glue(
            &mut ml,
            frame(Seg {
                sport: 1,
                ..Seg::new(0, 10)
            }),
            ifp,
        );
        tcp_softlro_glue(
            &mut ml,
            frame(Seg {
                sport: 2,
                ..Seg::new(100, 10)
            }),
            ifp,
        );
        tcp_softlro_glue(
            &mut ml,
            frame(Seg {
                sport: 3,
                ..Seg::new(200, 10)
            }),
            ifp,
        );
        tcp_softlro_glue(
            &mut ml,
            frame(Seg {
                sport: 2,
                ..Seg::new(110, 10)
            }),
            ifp,
        );
        // A's next one merges into the head.
        tcp_softlro_glue(
            &mut ml,
            frame(Seg {
                sport: 1,
                ..Seg::new(10, 10)
            }),
            ifp,
        );
        // C's next one merges into the last.
        tcp_softlro_glue(
            &mut ml,
            frame(Seg {
                sport: 3,
                ..Seg::new(210, 10)
            }),
            ifp,
        );
        let ms = drain(&mut ml);
        assert_eq!(ms.len(), 3);
        for (m, sport) in ms.iter().zip([1u16, 2, 3]) {
            let b = bytes(m);
            assert_eq!(u16::from_be_bytes([b[34], b[35]]), sport);
            assert_eq!(b.len(), 14 + 20 + 32 + 20, "flow {sport} merged");
        }

        // Ethernet padding past the IP length is trimmed before the merge.
        let t = frame(Seg {
            sport: 9,
            ..Seg::new(300, 10)
        });
        let n = t.m_len().get() + 6;
        t.m_len().set(n);
        t.m_pkthdr().len.set(n as i32);
        tcp_softlro_glue(
            &mut ml,
            frame(Seg {
                sport: 9,
                ..Seg::new(290, 10)
            }),
            ifp,
        );
        tcp_softlro_glue(&mut ml, t, ifp);
        let ms = drain(&mut ml);
        assert_eq!(ms.len(), 1);
        assert_eq!(bytes(&ms[0]).len(), 14 + 20 + 32 + 20);
    }

    #[test]
    fn softtso_chop_then_glue_is_the_identity() {
        // Software TSO's segments, framed again and glued back, carry the original stream
        // under one header with the original lengths.
        use crate::netinet::tcp_tso::tcp_softtso_chop;
        let _g = setup();
        let ifp = lro_ifnet(b"tlro4");
        for (payload, mss) in [(1, 100), (100, 100), (101, 100), (1000, 100), (1900, 600)] {
            let s = Seg::new(0x8000_0000 - 50, payload);
            let f = bytes(&frame(Seg {
                flags: TH_ACK | TH_PUSH,
                ..s
            }));
            let m0 = mbuf_of(&f[14..]);
            m0.m_pkthdr().csum_flags.set(M_TCP_TSO | M_TCP_CSUM_OUT);
            m0.m_pkthdr().ph_mss.set(mss as u16);
            let mut segs = MbufList::new();
            assert_eq!(tcp_softtso_chop(&mut segs, m0, ifp, mss), Ok(()));
            assert_eq!(ml_len(&segs) as usize, payload.div_ceil(mss as usize));

            let mut ml = MbufList::new();
            for seg in drain(&mut segs) {
                let mut g = f[..14].to_vec();
                g.extend_from_slice(&bytes(&seg));
                tcp_softlro_glue(&mut ml, mbuf_of(&g), ifp);
            }
            let ms = drain(&mut ml);
            assert_eq!(ms.len(), 1, "payload {payload}");
            let b = bytes(&ms[0]);
            assert_eq!(b.len(), f.len(), "payload {payload}");
            // The stream and every header field LRO keeps or restores are the original's.
            assert_eq!(&b[14 + 52..], &f[14 + 52..], "payload {payload}");
            assert_eq!(&b[14 + 2..14 + 4], &f[14 + 2..14 + 4], "ip_len");
            assert_eq!(
                &b[14 + 20..14 + 36],
                &f[14 + 20..14 + 36],
                "ports, seq, ack, flags, win"
            );
        }
    }
}
/* </TESTS> */
