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
//! TCP segmentation offload: `tcp_output.c`'s `tcp_softtso_chop` and `tcp_if_output_tso`
//! (N6.6a1).
//!
//! Upstream: sys/netinet/tcp_output.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_output.rs@3c62ede68802
//!
//! A large send leaves TCP as one TSO packet (`M_TCP_TSO`, its segment size in `ph_mss`).
//! [`tcp_if_output_tso`] hands it to an interface that cuts it in hardware (`IFCAP_TSOv4`,
//! `IFCAP_TSOv6`), or cuts it here with [`tcp_softtso_chop`] and sends the segments; the
//! kernel's `if_output_tso` asks it first for every packet it sends. `bsd`'s IPsec output
//! halves (`sys/netinet/ip_output.rs`, `sys/netinet6/ip6_output.rs`, N8) call
//! [`tcp_softtso_chop`] through `bsd`'s `tcp_output.rs`, which lends them its list.
//!
//! ## Deviations
//! - `tcp_softtso_chop` reads the IP header once as a copy (`ip_load`/`ip6_load`, which panic
//!   on a first mbuf shorter than the header, where the C reads past it) and writes it back
//!   into the first mbuf as it is after `m_pullup` (the C keeps a pointer taken before the
//!   pullup); each new segment's TCP header and options are copied with `m_copydata` (the C
//!   copies from the pointer, assuming the options are contiguous). A header write into a
//!   read-only first mbuf goes through `m_copyback` (the kernel's `ip_store`): its failure
//!   (no cluster for the copy) is `ENOBUFS` and the packet is dropped as the C's other
//!   `ENOBUFS` are, where the C writes in place.
//! - `tcp_if_output_tso` calls the interface's `if_output` through its driver (`NetDriver`;
//!   a missing driver panics, where the C would jump through NULL); an empty `*mp` returns
//!   `Ok(())`, which the C does not check.
//! - `INET6` is configured: the IPv6 cases of both (`in6_proto_cksum_out`, `IFCAP_TSOv6`).
//!
//! ## Redesign
//! - LZ's `tcp_softtso_chop(ml: &MbufList, m0: &'static Mbuf, ..)` worked on parked mbufs
//!   and wrote the new segments' TCP headers through `from_raw_parts_mut(mtod(m))` (one
//!   `unsafe`); here the packet is an owned `MbufBox` and the list the kernel's `MbufList`:
//!   the segments are built and appended, the trimmed original goes to the list's head
//!   last (`push_head`, O(1)), so the list's order is the C's (the original first, then the
//!   segments in order) and an error drops everything at once (the C's `ml_purge`). The
//!   headers are copies written back with `hal::pod` (`th_load`/`th_store`), no `unsafe`.
//! - LZ's `pub unsafe fn tcp_if_output_tso(.., dst: *const Sockaddr, ..)` (the raw output
//!   hook's contract, two `unsafe` calls) takes the kernel's `SockaddrView` and calls the
//!   driver's safe `output` and the kernel's `if_output_ml`. It was the `Inet` bridge's leg
//!   `tcp_if_output_tso` (N6.4a); the kernel's `if_output_tso` calls it directly now.

use core::cmp::min;
use core::mem::size_of;

use hal::pod;
use hal::sys::errno::Errno;
use hal::sys::net_if::{IFCAP_TSOv4, IFCAP_TSOv6};
use hal::sys::netinet_in::IPPROTO_TCP;
use hal::sys::netinet_ip::{IP_MF, IP_OFFMASK, Ip};
use hal::sys::netinet_ip6::Ip6Hdr;
use hal::sys::netinet_tcp::{TH_FIN, TH_PUSH, Tcphdr};

use crate::assert::kpanic;
use crate::net::if_::if_output_ml;
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{
    M_DONTWAIT, M_TCP_CSUM_OUT, M_TCP_TSO, MT_HEADER, Mbuf, MbufBox, MbufList, m_adj, m_align,
    m_copyback, m_copydata, m_copym, m_dup_pkthdr, m_gethdr, m_pullup, ml_enqueue, ml_init, ml_len,
    ml_purge,
};
use crate::net::proto::SockaddrView;
use crate::net::route::Rtentry;
use crate::netinet::ip_id::ip_randomid;
use crate::netinet::ip_output::{
    in_hdr_cksum_out, in_ifcap_cksum, in_proto_cksum_out, ip_load, ip_store,
};
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_add, tcpstat_inc};
use crate::netinet6::ip6_output::{in6_proto_cksum_out, ip6_load, ip6_store};

/// The network header of the packet being cut: the C's `ip` or `ip6`, one of them set.
#[derive(Clone, Copy)]
enum NetHdr {
    /// An IPv4 header without options.
    V4(Ip),
    /// An IPv6 header followed by the TCP header.
    V6(Ip6Hdr),
}

/// The TCP header at byte `off` of `m`'s data, copied out; `None` when the data ends before
/// it does.
fn th_load(m: &Mbuf, off: usize) -> Option<Tcphdr> {
    pod::at::<Tcphdr>(m.data(), off)
}

/// `th` written back at byte `off` of `m`'s data: in place when the data is writable, through
/// `m_copyback` (a copy of a shared cluster first) when it is not; `false`, nothing written,
/// when no cluster can be had for that copy.
#[must_use]
fn th_store(m: &mut Mbuf, off: usize, th: &Tcphdr) -> bool {
    if let Some(d) = m.data_mut()
        && pod::put(d, off, th)
    {
        return true;
    }
    let mut b = [0u8; size_of::<Tcphdr>()];
    let _ = pod::put(&mut b, 0, th);
    m_copyback(m, off as i32, &b, M_DONTWAIT).is_ok()
}

/// `tcp_softtso_chop(ml, m0, ifp, mss)`: cuts the TSO packet `m0` (an IPv4 TCP segment
/// without IP options or fragmentation, or an IPv6 one without extension headers) into
/// segments of at most `mss` bytes of payload on `ml` (emptied first), each with its own IP
/// and TCP headers and the checksums `ifp` cannot compute; `m0`, trimmed, is the first.
/// Consumes `m0`; on error `ml` is purged and `tcps_outbadtso` counted.
pub fn tcp_softtso_chop(
    ml: &mut MbufList,
    m0: MbufBox,
    ifp: &Ifnet,
    mss: u32,
) -> Result<(), Errno> {
    ml_init(ml);

    match softtso_cut(ml, m0, ifp, mss) {
        Ok(m0) => {
            // The original packet ahead of its segments, as the C enqueued it first.
            ml.push_head(m0);
            tcpstat_add(TcpstatCounters::TcpsOutpkttso, u64::from(ml_len(ml)));
            Ok(())
        }
        Err(error) => {
            // bad:
            tcpstat_inc(TcpstatCounters::TcpsOutbadtso);
            let _ = ml_purge(ml);
            Err(error)
        }
    }
}

/// The body of [`tcp_softtso_chop`]: the segments after the first appended to `ml`, the
/// first (`m0` trimmed, its headers adjusted) returned; on error `m0` is dropped.
fn softtso_cut(ml: &mut MbufList, m0: MbufBox, ifp: &Ifnet, mss: u32) -> Result<MbufBox, Errno> {
    let mut m0 = m0;

    if mss == 0 {
        return Err(Errno::EINVAL);
    }

    let version = m0.data().first().map_or(0, |b| b >> 4);
    let (net, iphlen) = match version {
        4 => {
            let ip = ip_load(&m0);
            let iphlen = usize::from(ip.ip_hl()) << 2;
            if u16::from_be(ip.ip_off) & (IP_OFFMASK | IP_MF) != 0
                || iphlen != size_of::<Ip>()
                || i32::from(ip.ip_p) != IPPROTO_TCP
            {
                // only TCP without fragment or IP option supported
                return Err(Errno::EPROTOTYPE);
            }
            (NetHdr::V4(ip), iphlen)
        }
        6 => {
            let ip6 = ip6_load(&m0);
            if i32::from(ip6.ip6_nxt) != IPPROTO_TCP {
                // only TCP without IPv6 header chain supported
                return Err(Errno::EPROTOTYPE);
            }
            (NetHdr::V6(ip6), size_of::<Ip6Hdr>())
        }
        v => kpanic(format_args!("tcp_softtso_chop: unknown ip version {v}")),
    };

    let tlen = m0.m_pkthdr().len.get();
    let thmin = iphlen + size_of::<Tcphdr>();
    if (tlen as usize) < thmin {
        return Err(Errno::ENOPROTOOPT);
    }
    // IP and TCP header should be contiguous, this check is paranoia
    if (m0.m_len().get() as usize) < thmin {
        m0 = m_pullup(m0, thmin as i32).ok_or(Errno::ENOBUFS)?;
    }
    let Some(mut th) = th_load(&m0, iphlen) else {
        kpanic(format_args!(
            "tcp_softtso_chop: no TCP header after the pullup"
        ));
    };
    let hlen = (iphlen + (usize::from(th.th_off()) << 2)) as i32;
    if tlen < hlen {
        return Err(Errno::ENOPROTOOPT);
    }
    let firstlen = min((tlen - hlen) as u32, mss) as i32;

    let ph = m0.m_pkthdr();
    ph.csum_flags.set(ph.csum_flags.get() & !M_TCP_TSO);

    // Loop through length of payload after first segment, make new header and copy data of
    // each part and link onto chain.
    let mut off = hlen + firstlen;
    while off < tlen {
        let len = min((tlen - off) as u32, mss) as i32;

        let mut m = m_gethdr(M_DONTWAIT, MT_HEADER).ok_or(Errno::ENOBUFS)?;
        m_dup_pkthdr(&mut m, &m0, M_DONTWAIT)?;

        // IP and TCP header to the end, space for link layer header
        m.m_len().set(hlen as u32);
        m_align(&m, hlen);

        // copy and adjust TCP header
        match m.data_mut() {
            Some(d) => m_copydata(&m0, iphlen as i32, &mut d[iphlen..hlen as usize]),
            None => kpanic(format_args!(
                "tcp_softtso_chop: a new header mbuf is read-only"
            )),
        }
        let Some(mut mhth) = th_load(&m, iphlen) else {
            kpanic(format_args!("tcp_softtso_chop: no room for the TCP header"));
        };
        mhth.th_seq = u32::from_be(th.th_seq)
            .wrapping_add((off - hlen) as u32)
            .to_be();
        if off + len < tlen {
            mhth.th_flags &= !(TH_PUSH | TH_FIN);
        }
        if !th_store(&mut m, iphlen, &mhth) {
            return Err(Errno::ENOBUFS);
        }

        // add mbuf chain with payload
        m.m_pkthdr().len.set(hlen + len);
        let payload = m_copym(&mut m0, off, len, M_DONTWAIT).ok_or(Errno::ENOBUFS)?;
        drop(m.m_next.set(payload));

        // copy and adjust IP header, calculate checksum
        let ph = m.m_pkthdr();
        ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_OUT);
        match net {
            NetHdr::V4(ip) => {
                let mut mhip = ip;
                mhip.ip_len = ((hlen + len) as u16).to_be();
                mhip.ip_id = ip_randomid().to_be();
                if !ip_store(&mut m, &mhip) {
                    return Err(Errno::ENOBUFS);
                }
                in_hdr_cksum_out(&mut m, Some(ifp));
                in_proto_cksum_out(&mut m, Some(ifp));
            }
            NetHdr::V6(ip6) => {
                let mut mhip6 = ip6;
                mhip6.ip6_plen = ((hlen - iphlen as i32 + len) as u16).to_be();
                if !ip6_store(&mut m, &mhip6) {
                    return Err(Errno::ENOBUFS);
                }
                in6_proto_cksum_out(&mut m, Some(ifp));
            }
        }
        ml_enqueue(ml, m);

        off += mss as i32;
    }

    // Update first segment by trimming what's been copied out and updating header, then send
    // each segment (in order).
    if hlen + firstlen < tlen {
        m_adj(&m0, hlen + firstlen - tlen);
        th.th_flags &= !(TH_PUSH | TH_FIN);
        if !th_store(&mut m0, iphlen, &th) {
            return Err(Errno::ENOBUFS);
        }
    }
    // adjust IP header, calculate checksum
    let ph = m0.m_pkthdr();
    ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_OUT);
    let total = ph.len.get();
    match net {
        NetHdr::V4(mut ip) => {
            ip.ip_len = (total as u16).to_be();
            if !ip_store(&mut m0, &ip) {
                return Err(Errno::ENOBUFS);
            }
            in_hdr_cksum_out(&mut m0, Some(ifp));
            in_proto_cksum_out(&mut m0, Some(ifp));
        }
        NetHdr::V6(mut ip6) => {
            ip6.ip6_plen = ((total as usize - iphlen) as u16).to_be();
            if !ip6_store(&mut m0, &ip6) {
                return Err(Errno::ENOBUFS);
            }
            in6_proto_cksum_out(&mut m0, Some(ifp));
        }
    }

    Ok(m0)
}

/// `tcp_if_output_tso(ifp, mp, dst, rt, ifcap, mtu)`: sends the TSO packet `*mp` on `ifp`
/// toward `dst`, by the hardware when the interface can do TSO (`ifcap`), else cut by
/// [`tcp_softtso_chop`]; `*mp` is consumed then. A packet that did not ask for TSO, or whose
/// segments exceed `mtu` (its TSO flag cleared), is left in `*mp` for the caller to send,
/// fragment or drop.
pub fn tcp_if_output_tso(
    ifp: &'static Ifnet,
    mp: &mut Option<MbufBox>,
    dst: SockaddrView<'_>,
    rt: Option<&Rtentry>,
    ifcap: u32,
    mtu: u32,
) -> Result<(), Errno> {
    let Some(m) = mp.as_ref() else {
        return Ok(());
    };
    let ph = m.m_pkthdr();

    // caller must fail later or fragment
    if ph.csum_flags.get() & M_TCP_TSO == 0 {
        return Ok(());
    }
    if u32::from(ph.ph_mss.get()) > mtu {
        ph.csum_flags.set(ph.csum_flags.get() & !M_TCP_TSO);
        return Ok(());
    }

    // done: the packet is ours from here on, whatever happens to it (`*mp = NULL`).
    let Some(mut m) = mp.take() else {
        return Ok(());
    };

    // network interface hardware will do TSO
    if in_ifcap_cksum(&m, Some(ifp), ifcap) {
        if ifcap & IFCAP_TSOv4 != 0 {
            in_hdr_cksum_out(&mut m, Some(ifp));
            in_proto_cksum_out(&mut m, Some(ifp));
        }
        if ifcap & IFCAP_TSOv6 != 0 {
            in6_proto_cksum_out(&mut m, Some(ifp));
        }
        let Some(d) = ifp.if_driver.get() else {
            drop(m);
            kpanic(format_args!(
                "tcp_if_output_tso: an interface without if_output"
            ));
        };
        let error = d.output(ifp, m, dst, rt);
        if error.is_ok() {
            tcpstat_inc(TcpstatCounters::TcpsOuthwtso);
        }
        return error;
    }

    // as fallback do TSO in software
    let mss = u32::from(m.m_pkthdr().ph_mss.get());
    let mut ml = MbufList::new();
    tcp_softtso_chop(&mut ml, m, ifp, mss)?;
    if_output_ml(ifp, &mut ml, dst, rt)?;
    tcpstat_inc(TcpstatCounters::TcpsOutswtso);
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Software TSO (`tcp_softtso_chop`: segment sizes, sequence numbers, flags, both
    // checksums, IPv4 and IPv6; malformed packets; the counters) and `tcp_if_output_tso`'s
    // four ways (no TSO, segments over the MTU, software, hardware), re-expressed from LZ's
    // `tcp_output.rs` tests over the kernel's mbufs and a recording driver.

    use std::sync::Mutex as StdMutex;
    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use hal::sys::mbuf::MT_DATA;
    use hal::sys::netinet_in::IPPROTO_IPV6;
    use hal::sys::netinet_tcp::TH_ACK;

    use super::*;
    use crate::net::if_::test_ifnet_with;
    use crate::net::if_var::{NetDriver, Netstack};
    use crate::net::mbuf::{MCLBYTES, m_clget_hdr, ml_dequeue};

    const SRC: [u8; 4] = [10, 0, 0, 1];
    const DST: [u8; 4] = [10, 0, 0, 2];
    const SRC6: [u8; 16] = [0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    const DST6: [u8; 16] = [0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
    const SEQ: u32 = 0xffff_ff00;

    /// The interface layer's fixture, and the IP id permutation software TSO draws from.
    fn setup() -> MutexGuard<'static, ()> {
        let g = crate::net::if_::tests::setup();
        crate::netinet::ip_id::ip_randomid_init();
        g
    }

    /// An IPv4 (`v == 4`) or IPv6 TSO packet carrying `payload` bytes of TCP data (byte `i`
    /// is `i as u8`) with PUSH, ACK and FIN, segment size `mss`, `ip_off` as given, one
    /// 20-byte IPv4 header (`ihl` words: more than 5 adds options).
    fn tso_packet(v: u8, payload: usize, mss: u16, ihl: u8, ip_off: u16) -> MbufBox {
        let mut p = Vec::new();
        if v == 4 {
            let iphlen = usize::from(ihl) * 4;
            p.extend_from_slice(&[0x40 | ihl, 0]);
            p.extend_from_slice(&((iphlen + 20 + payload) as u16).to_be_bytes());
            p.extend_from_slice(&[0, 0]);
            p.extend_from_slice(&ip_off.to_be_bytes());
            p.extend_from_slice(&[64, IPPROTO_TCP as u8, 0, 0]);
            p.extend_from_slice(&SRC);
            p.extend_from_slice(&DST);
            p.resize(iphlen, 1); // options, if any
        } else {
            p.extend_from_slice(&[0x60, 0, 0, 0]);
            p.extend_from_slice(&((20 + payload) as u16).to_be_bytes());
            p.extend_from_slice(&[
                if ihl == 0 {
                    IPPROTO_IPV6 as u8
                } else {
                    IPPROTO_TCP as u8
                },
                64,
            ]);
            p.extend_from_slice(&SRC6);
            p.extend_from_slice(&DST6);
        }
        p.extend_from_slice(&1234u16.to_be_bytes());
        p.extend_from_slice(&80u16.to_be_bytes());
        p.extend_from_slice(&SEQ.to_be_bytes());
        p.extend_from_slice(&7u32.to_be_bytes());
        p.extend_from_slice(&[5 << 4, TH_PUSH | TH_ACK | TH_FIN]);
        p.extend_from_slice(&[0xff, 0xff, 0, 0, 0, 0]);
        p.extend((0..payload).map(|i| i as u8));
        assert!(p.len() <= MCLBYTES);

        let mut m = m_clget_hdr(M_DONTWAIT, MCLBYTES as u32).expect("cluster");
        m.m_len().set(p.len() as u32);
        m.data_mut().expect("writable")[..p.len()].copy_from_slice(&p);
        m.m_pkthdr().len.set(p.len() as i32);
        m.m_pkthdr().csum_flags.set(M_TCP_TSO | M_TCP_CSUM_OUT);
        m.m_pkthdr().ph_mss.set(mss);
        m
    }

    /// The bytes of packet `m`.
    fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut v = vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(m, 0, &mut v);
        v
    }

    /// The ones' complement sum of `words` (16-bit, big-endian; an odd byte padded), folded.
    fn sum16(words: &[u8]) -> u16 {
        let mut sum: u32 = words
            .chunks(2)
            .map(|c| u32::from(u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)])))
            .sum();
        while sum > 0xffff {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        sum as u16
    }

    /// Checks that `segs` are `payload` bytes cut at `mss`, with consecutive sequence
    /// numbers, PUSH and FIN only on the last, the IP lengths and both checksums right.
    fn check_segments(v: u8, segs: &[Vec<u8>], payload: usize, mss: usize) {
        let iphlen = if v == 4 { 20 } else { 40 };
        assert_eq!(segs.len(), payload.div_ceil(mss));
        for (k, b) in segs.iter().enumerate() {
            let dlen = min(mss, payload - k * mss);
            assert_eq!(b.len(), iphlen + 20 + dlen, "segment {k}");
            let t = &b[iphlen..];
            let seq = u32::from_be_bytes([t[4], t[5], t[6], t[7]]);
            assert_eq!(seq, SEQ.wrapping_add((k * mss) as u32), "segment {k}");
            let last = k + 1 == segs.len();
            let want = if last {
                TH_PUSH | TH_ACK | TH_FIN
            } else {
                TH_ACK
            };
            assert_eq!(t[13], want, "flags of segment {k}");
            for (i, &x) in t[20..].iter().enumerate() {
                assert_eq!(x, (k * mss + i) as u8);
            }
            let mut pseudo = Vec::new();
            if v == 4 {
                assert_eq!(usize::from(u16::from_be_bytes([b[2], b[3]])), 40 + dlen);
                assert_eq!(&b[12..20], &[SRC, DST].concat()[..]);
                assert_eq!(sum16(&b[..20]), 0xffff, "IP checksum of segment {k}");
                pseudo.extend_from_slice(&[SRC, DST].concat());
                pseudo.extend_from_slice(&[0, IPPROTO_TCP as u8]);
                pseudo.extend_from_slice(&((20 + dlen) as u16).to_be_bytes());
            } else {
                assert_eq!(usize::from(u16::from_be_bytes([b[4], b[5]])), 20 + dlen);
                pseudo.extend_from_slice(&[SRC6, DST6].concat());
                pseudo.extend_from_slice(&((20 + dlen) as u32).to_be_bytes());
                pseudo.extend_from_slice(&[0, 0, 0, IPPROTO_TCP as u8]);
            }
            pseudo.extend_from_slice(t);
            assert_eq!(sum16(&pseudo), 0xffff, "TCP checksum of segment {k}");
        }
    }

    /// The counter `c`.
    fn count(c: TcpstatCounters) -> u64 {
        crate::netinet::tcp_var::TCPCOUNTERS.read()[c as usize]
    }

    /// What [`RecDriver`] sent: (interface address, packet bytes).
    static SENT: StdMutex<Vec<(usize, Vec<u8>)>> = StdMutex::new(Vec::new());

    /// A driver whose `if_output` records the packet and frees it.
    struct RecDriver;

    impl NetDriver for RecDriver {
        fn ioctl(&self, _ifp: &'static Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            Ok(())
        }

        fn output(
            &self,
            ifp: &'static Ifnet,
            m: MbufBox,
            _dst: SockaddrView<'_>,
            _rt: Option<&Rtentry>,
        ) -> Result<(), Errno> {
            let b = bytes(&m);
            SENT.lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((core::ptr::from_ref(ifp) as usize, b));
            Ok(())
        }

        fn input(&self, _ifp: &'static Ifnet, m: MbufBox, _ns: Option<&mut Netstack>) {
            drop(m);
        }
    }

    static REC: RecDriver = RecDriver;

    /// The packets `ifp` sent, taken out of [`SENT`].
    fn sent_by(ifp: &Ifnet) -> Vec<Vec<u8>> {
        let tag = core::ptr::from_ref(ifp) as usize;
        let mut all = SENT.lock().unwrap_or_else(|e| e.into_inner());
        let mine = all
            .iter()
            .filter(|(i, _)| *i == tag)
            .map(|(_, b)| b.clone())
            .collect();
        all.retain(|(i, _)| *i != tag);
        mine
    }

    /// A socket address view of an IPv4 destination (the driver does not read it).
    fn with_dst<R>(f: impl FnOnce(SockaddrView<'_>) -> R) -> R {
        let mut sin = [0u8; 16];
        sin[0] = 16;
        sin[1] = hal::sys::socket::AF_INET;
        f(SockaddrView::new(&sin).expect("a socket address"))
    }

    #[test]
    fn softtso_chop_cuts_at_mss() {
        let _g = setup();
        let ifp = test_ifnet_with(b"ttso0", &REC);

        for v in [4, 6] {
            let pkts = count(TcpstatCounters::TcpsOutpkttso);
            let m0 = tso_packet(v, 1300, 500, 5, 0);
            let first = core::ptr::from_ref::<Mbuf>(&m0);
            let mut ml = MbufList::new();
            assert_eq!(tcp_softtso_chop(&mut ml, m0, ifp, 500), Ok(()));
            assert_eq!(ml_len(&ml), 3);
            assert_eq!(count(TcpstatCounters::TcpsOutpkttso) - pkts, 3);
            let mut segs = Vec::new();
            while let Some(m) = ml_dequeue(&mut ml) {
                segs.push(m);
            }
            assert!(
                core::ptr::eq(&*segs[0], first),
                "the first segment is the original packet"
            );
            for m in &segs {
                assert_eq!(m.m_pkthdr().csum_flags.get() & M_TCP_TSO, 0);
            }
            check_segments(
                v,
                &segs.iter().map(|m| bytes(m)).collect::<Vec<_>>(),
                1300,
                500,
            );

            // A packet that fits in one segment keeps its flags.
            let m0 = tso_packet(v, 300, 500, 5, 0);
            assert_eq!(tcp_softtso_chop(&mut ml, m0, ifp, 500), Ok(()));
            let m = ml_dequeue(&mut ml).expect("segment");
            assert!(ml_dequeue(&mut ml).is_none());
            check_segments(v, &[bytes(&m)], 300, 500);
        }
    }

    #[test]
    fn softtso_chop_cuts_at_every_size() {
        // Every payload length around the segment size, each cut checked whole.
        let _g = setup();
        let ifp = test_ifnet_with(b"ttso3", &REC);
        for v in [4, 6] {
            for payload in [1, 99, 100, 101, 199, 200, 201, 999, 1000, 1001] {
                let mut ml = MbufList::new();
                let m0 = tso_packet(v, payload, 100, 5, 0);
                assert_eq!(tcp_softtso_chop(&mut ml, m0, ifp, 100), Ok(()));
                let mut segs = Vec::new();
                while let Some(m) = ml_dequeue(&mut ml) {
                    segs.push(bytes(&m));
                }
                check_segments(v, &segs, payload, 100);
            }
        }
    }

    #[test]
    fn softtso_chop_rejects_what_it_cannot_cut() {
        let _g = setup();
        let ifp = test_ifnet_with(b"ttso1", &REC);
        let mut ml = MbufList::new();

        let cases = [
            (tso_packet(4, 1000, 500, 5, 0), 0, Errno::EINVAL),
            (tso_packet(4, 1000, 500, 5, IP_MF), 500, Errno::EPROTOTYPE),
            (tso_packet(4, 1000, 500, 5, 1), 500, Errno::EPROTOTYPE),
            (tso_packet(4, 1000, 500, 6, 0), 500, Errno::EPROTOTYPE),
            (tso_packet(6, 1000, 500, 0, 0), 500, Errno::EPROTOTYPE),
        ];
        for (m0, mss, error) in cases {
            let bad = count(TcpstatCounters::TcpsOutbadtso);
            ml_enqueue(&mut ml, m_gethdr(M_DONTWAIT, MT_DATA).expect("mbuf"));
            assert_eq!(tcp_softtso_chop(&mut ml, m0, ifp, mss), Err(error));
            assert_eq!(ml_len(&ml), 0, "the list is purged");
            assert_eq!(count(TcpstatCounters::TcpsOutbadtso) - bad, 1);
        }

        // Shorter than the headers it claims.
        let m0 = tso_packet(4, 0, 500, 5, 0);
        m0.m_pkthdr().len.set(30);
        assert_eq!(
            tcp_softtso_chop(&mut ml, m0, ifp, 500),
            Err(Errno::ENOPROTOOPT)
        );
        assert_eq!(ml_len(&ml), 0);
        // A data offset past the packet.
        let mut m0 = tso_packet(4, 0, 500, 5, 0);
        m0.data_mut().expect("writable")[32] = 15 << 4;
        assert_eq!(
            tcp_softtso_chop(&mut ml, m0, ifp, 500),
            Err(Errno::ENOPROTOOPT)
        );
        assert_eq!(ml_len(&ml), 0);
    }

    #[test]
    fn if_output_tso_chooses_the_way_out() {
        let _g = setup();
        let ifp = test_ifnet_with(b"ttso2", &REC);

        // Not a TSO packet: left to the caller.
        let m = tso_packet(4, 1000, 500, 5, 0);
        m.m_pkthdr().csum_flags.set(M_TCP_CSUM_OUT);
        let mut mp = Some(m);
        assert_eq!(
            with_dst(|d| tcp_if_output_tso(ifp, &mut mp, d, None, IFCAP_TSOv4, 1500)),
            Ok(())
        );
        assert!(mp.is_some());

        // Segments larger than the MTU: TSO is cleared, the caller fragments or drops.
        if let Some(m) = &mp {
            m.m_pkthdr().csum_flags.set(M_TCP_TSO | M_TCP_CSUM_OUT);
        }
        assert_eq!(
            with_dst(|d| tcp_if_output_tso(ifp, &mut mp, d, None, IFCAP_TSOv4, 400)),
            Ok(())
        );
        let m = mp.take().expect("left to the caller");
        assert_eq!(m.m_pkthdr().csum_flags.get() & M_TCP_TSO, 0);
        drop(m);
        assert!(sent_by(ifp).is_empty());

        // No hardware TSO: chopped in software.
        let sw = count(TcpstatCounters::TcpsOutswtso);
        let mut mp = Some(tso_packet(4, 1300, 500, 5, 0));
        assert_eq!(
            with_dst(|d| tcp_if_output_tso(ifp, &mut mp, d, None, IFCAP_TSOv4, 1500)),
            Ok(())
        );
        assert!(mp.is_none());
        check_segments(4, &sent_by(ifp), 1300, 500);
        assert_eq!(count(TcpstatCounters::TcpsOutswtso) - sw, 1);

        // Hardware TSO: the whole packet goes to the interface.
        let hw = count(TcpstatCounters::TcpsOuthwtso);
        ifp.if_capabilities.set(IFCAP_TSOv4);
        let mut mp = Some(tso_packet(4, 1300, 500, 5, 0));
        assert_eq!(
            with_dst(|d| tcp_if_output_tso(ifp, &mut mp, d, None, IFCAP_TSOv4, 1500)),
            Ok(())
        );
        assert!(mp.is_none());
        let out = sent_by(ifp);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 1340);
        assert_eq!(count(TcpstatCounters::TcpsOuthwtso) - hw, 1);
    }
}
/* </TESTS> */
