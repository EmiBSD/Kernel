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
//! TCP support routines: `netinet/tcp_subr.c`.
//!
//! Upstream: sys/netinet/tcp_subr.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_subr.rs@3c62ede68802
//!
//! `tcp_template` builds the skeletal TCP/IP header a connection sends with; `tcp_respond`
//! sends a bare segment (an ACK, a RST, a keepalive probe) either from a connection's
//! template or in reply to a received segment; `tcp_free` puts back a control block
//! `tcp_close` closed. `tcp_set_iss_tsm` derives initial sequence numbers and timestamp
//! modulators from a keyed hash of the connection (RFC 1948). With `TCP_SIGNATURE`,
//! `tcp_signature` computes the RFC 2385 MD5 digest.
//!
//! Locks used to protect struct members in this file: \[I\] immutable after creation,
//! \[T\] `tcp_timer_mtx` (global tcp timer data structures), \[a\] atomic.
//!
//! ## Deviations
//! - The sysctl-controlled `int`s are `AtomicI32`s (the C reads them with
//!   `atomic_load_int`).
//! - `tcp_reass_limit` starts at `NMBCLUSTERS / 8` in the C's static initializer; here
//!   `tcp_init` sets it from `nmbclust` (`kern.maxclusters`), whose boot value is the
//!   machine's `NMBCLUSTERS` (`mbinit` runs before `domaininit`), since the kernel crate
//!   names no machine constant: the same number.
//! - `tcp_respond`'s `caddr_t template` is a byte slice: the IP header, followed by the TCP
//!   header when `th0` is `None` (the template of a connection); the headers are written in
//!   place into the new mbuf with `hal::pod`. `flags` is the `u8` of `th_flags`. Without a
//!   control block it uses `ip6_defhlim`, what the C's `in6_selecthlim(NULL)` returns
//!   (`in6_selecthlim` takes a pcb here); the caller passes its proof of the net lock.
//! - `tcp_signature` returns `bool` (`true` for the C's 0); its `char *sig` is the 16-byte
//!   digest array; the pseudo header and the TCP header are hashed as the bytes the C's
//!   structures hold (`hal::pod`). `tcp_signature_apply` is the closure handed to `m_apply`.
//!   The five `tcp_signature_tdb_*` functions (the `XF_TCPSIGNATURE` transform, IPsec's
//!   `xformsw` row over `bsd`'s `struct tdb`) stay `bsd`'s until IPsec moves (D-10, N8).
//!
//! ## Redesign
//! - N6.6a2: the kernel's TCP is built beside `bsd`'s, unregistered until N6.6a5 (D-2):
//!   these globals and functions are the kernel's own, `bsd`'s `sys/netinet/tcp_subr.rs`
//!   keeps its until the switch. `tcpqe_pool` and `sackhl_pool` are `BoxPool`s whose items
//!   the queues own (`hal::pool::Pooled`), `tcpcb_pool` is the glue's (`tcp_var::TcpTables`);
//!   `tcp_iss` is the data of `tcp_timer_mtx` (a `Mutexed`, where LZ had an atomic beside a
//!   bare mutex); `tcp_secret` and `tcp_secret_ctx` are `OnceStatic`s set by `tcp_init` (LZ:
//!   `StaticCell`s written under `unsafe`).
//! - D-12: `tp = tcp_close(tp)` is a move: `tcp_close` consumes the control block's [`Tp`]
//!   and answers [`Closed`], and the frame that owns the socket lock's parts frees the block
//!   with [`tcp_free`] before it unlocks (the C's `inp_ppcb = NULL` and `pool_put`, a few
//!   statements later under the same lock). A use after the close does not compile.
//! - `tcp_template` takes the template out of the control block and gives it back filled
//!   (the C returns the pointer its callers store); `tcp_respond` writes its headers in
//!   place (`hal::pod`, where LZ wrote unaligned under `unsafe`), and lends the pcb's route
//!   cache and IPv6 options to the output call. `tcp_signature`'s key is lent by IPsec's
//!   `tdb_amxkey` leg to the MD5 update, never copied (D-9); the digest is compared with
//!   `timingsafe_bcmp` by its callers.

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::mutex::Mutexed;
use hal::netlock::NetHeld;
use hal::once::OnceStatic;
use hal::pod;
use hal::pool::BoxPool;
use hal::rwlock::WitnessPart;
use hal::sys::endian::{htonl, htons};
use hal::sys::mbuf::MT_HEADER;
use hal::sys::netinet_in::IPPROTO_TCP;
use hal::sys::netinet_ip::{Ip, Ippseudo};
use hal::sys::netinet_ip_var::Ipovly;
use hal::sys::netinet_ip6::{IPV6_FLOWLABEL_MASK, Ip6Hdr, Ip6HdrPseudo};
use hal::sys::netinet_tcp::{
    TCP_MAX_WINSHIFT, TCP_MAXWIN, TCP_MSS, TCPOLEN_TSTAMP_APPA, TCPOPT_TSTAMP_HDR, TH_ACK, TH_RST,
    TcpSeq, Tcphdr,
};
use hal::sys::socket::{AF_INET, AF_INET6};

use crate::assert::kpanic;
use crate::crypto::md5::{MD5_DIGEST_LENGTH, Md5Ctx};
use crate::crypto::sha2::{SHA512_DIGEST_LENGTH, Sha512Ctx};
use crate::net::ipsec_hooks::{TdbRef, ipsec};
use crate::net::mbuf::{
    M_DONTWAIT, M_TCP_CSUM_OUT, MAX_LINKHDR, Mbuf, MbufBox, m_apply, m_get, m_gethdr,
};
use crate::net::rtable::rtable_l2;
use crate::netinet::in_pcb::Inpcb;
use crate::netinet::ip_output::ip_output;
use crate::netinet::ip_var::{IP_DEFTTL, IP_MTUDISC, ip_mtudisc};
use crate::netinet::tcp_timer::TCPTV_SRTTDFLT;
use crate::netinet::tcp_var::{
    Sackhole, TF_NOOPT, TF_RCVD_TSTMP, TF_REQ_TSTMP, TcpState, Tcpqent, Tp,
};
use crate::netinet6::in6_src::{in6_clearscope, in6_selecthlim};
use crate::netinet6::ip6_output::ip6_output;
use crate::netinet6::ip6_var::{IP6_DEFHLIM, Ip6Pktopts};
use crate::proc::ProcGlue;

/// `AF_INET` as the `int` of `tp->pf` and of the functions' `af` arguments.
const AF_INET_I32: i32 = AF_INET as i32;
/// `AF_INET6` as the `int` of `tp->pf` and of the functions' `af` arguments.
const AF_INET6_I32: i32 = AF_INET6 as i32;

/// `TCP_ISS_CONN_INC`: the step of `tcp_iss` per connection.
const TCP_ISS_CONN_INC: u32 = 4096;

/// `tcp_close`'s answer: the control block is closed and must be freed by the frame that
/// owns the socket lock's parts ([`tcp_free`]) before it unlocks; the C's `tp = NULL`. A use
/// of the closed control block does not compile (`tcp_close` consumed its [`Tp`]).
#[must_use = "a closed control block is freed by tcp_free before the socket lock goes"]
pub struct Closed(());

/// The headers `tcp_respond` builds: the C's `ip` or `ip6` pointer into the mbuf.
enum RespondHdr {
    /// `struct ip`.
    V4(Ip),
    /// `struct ip6_hdr`.
    V6(Ip6Hdr),
}

// patchable/settable parameters for tcp

/// \[a\] `tcp_mssdflt`: default maximum segment size.
pub static TCP_MSSDFLT: AtomicI32 = AtomicI32::new(TCP_MSS);
/// `tcp_rttdflt`.
pub static TCP_RTTDFLT: AtomicI32 = AtomicI32::new(TCPTV_SRTTDFLT);

// values controllable via sysctl

/// \[a\] `tcp_do_rfc1323`.
pub static TCP_DO_RFC1323: AtomicI32 = AtomicI32::new(1);
/// \[a\] `tcp_do_sack`: RFC 2018 selective ACKs.
pub static TCP_DO_SACK: AtomicI32 = AtomicI32::new(1);
/// \[a\] `tcp_ack_on_push`: set to enable immediate ACK-on-PUSH.
pub static TCP_ACK_ON_PUSH: AtomicI32 = AtomicI32::new(0);
/// \[a\] `tcp_do_ecn`: RFC3168 ECN enabled/disabled? (`TCP_ECN`).
pub static TCP_DO_ECN: AtomicI32 = AtomicI32::new(0);
/// \[a\] `tcp_do_rfc3390`: increase TCP's Initial Window to 10*mss.
pub static TCP_DO_RFC3390: AtomicI32 = AtomicI32::new(2);
/// \[a\] `tcp_do_tso`: TCP segmentation offload for output.
pub static TCP_DO_TSO: AtomicI32 = AtomicI32::new(1);

/// `tcp_reass_limit`: hardlimit for `tcpqe_pool`; `NMBCLUSTERS / 8`, set by `tcp_init` (see
/// the deviations).
pub static TCP_REASS_LIMIT: AtomicI32 = AtomicI32::new(0);
/// `tcp_sackhole_limit`: hardlimit for `sackhl_pool`.
#[allow(non_upper_case_globals)] // TCP_SACKHOLE_LIMIT is a constant of tcp.h
pub static tcp_sackhole_limit: AtomicI32 = AtomicI32::new(32 * 1024);

/// `tcpqe_pool`: the reassembly queue entries.
pub static TCPQE_POOL: BoxPool<Tcpqent> = BoxPool::new();
/// `sackhl_pool`: the SACK holes.
pub static SACKHL_POOL: BoxPool<Sackhole> = BoxPool::new();

/// \[I\] `tcp_secret`.
pub(crate) static TCP_SECRET: OnceStatic<[u8; 16]> = OnceStatic::new();
/// \[I\] `tcp_secret_ctx`: the SHA-512 context over the secret.
pub(crate) static TCP_SECRET_CTX: OnceStatic<Sha512Ctx> = OnceStatic::new();
/// `tcp_timer_mtx` and what it keeps: \[T\] `tcp_iss`, updated by timer and connection.
pub static TCP_ISS: Mutexed<TcpSeq> = Mutexed::new(IPL_SOFTNET, 0);
/// \[I\] `tcp_starttime`: random offset for `tcp_now()`.
pub static TCP_STARTTIME: AtomicU64 = AtomicU64::new(0);

/// `tcp_template`: create template to be used to send tcp packets on a connection. Call
/// after host entry created, allocates an mbuf and fills in a skeletal tcp/ip header,
/// minimizing the amount of work necessary when the connection is used. The template the
/// control block holds is filled again and given back (the caller stores the answer in
/// `t_template`, as the C's callers do); `None` when no mbuf can be had.
///
/// To support IPv6 in addition to IPv4 and considering that the sizes of the IPv4 and IPv6
/// headers are not the same, we now use a separate pointer for the TCP header. Also, we made
/// the former tcpiphdr header pointer into just an IP overlay pointer, with casting as
/// appropriate for v6. rja
pub fn tcp_template<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) -> Option<MbufBox> {
    let inp = tp.inp;

    let mut m = match tp.st.t_template.take() {
        Some(m) => m,
        None => {
            let m = m_get(M_DONTWAIT, MT_HEADER)?;
            let nh = match tp.st.pf {
                // default to PF_INET
                0 | AF_INET_I32 => size_of::<Ip>(),
                AF_INET6_I32 => size_of::<Ip6Hdr>(),
                _ => 0,
            };
            m.m_len().set((nh + size_of::<Tcphdr>()) as u32);
            m
        }
    };

    let Some(d) = m.data_mut() else {
        kpanic(format_args!("tcp_template: the template is read-only"));
    };
    let thoff = match tp.st.pf {
        AF_INET_I32 => {
            let ipovly = Ipovly {
                ih_x1: [0; 9],
                ih_pr: IPPROTO_TCP as u8,
                ih_len: htons(size_of::<Tcphdr>() as u16),
                ih_src: inp.inp_laddr.get(),
                ih_dst: inp.inp_faddr.get(),
            };
            put_hdr(d, 0, &ipovly);
            size_of::<Ip>()
        }
        AF_INET6_I32 => {
            let ip6 = Ip6Hdr {
                ip6_src: inp.inp_laddr6.get(),
                ip6_dst: inp.inp_faddr6.get(),
                ip6_flow: htonl(0x6000_0000) | (inp.inp_flowinfo() & IPV6_FLOWLABEL_MASK),
                ip6_nxt: IPPROTO_TCP as u8,
                ip6_plen: htons(size_of::<Tcphdr>() as u16), // XXX
                ip6_hlim: in6_selecthlim(inp) as u8,         // XXX
            };
            put_hdr(d, 0, &ip6);
            size_of::<Ip6Hdr>()
        }
        pf => kpanic(format_args!("tcp_template: unhandled af {pf}")),
    };

    let mut th = Tcphdr {
        th_sport: inp.inp_lport.get(),
        th_dport: inp.inp_fport.get(),
        ..Tcphdr::default()
    };
    th.set_th_x2(0);
    th.set_th_off(5);
    put_hdr(d, thoff, &th);
    Some(m)
}

/// Writes the header `v` at `off` of `d`, the data of an mbuf this file just sized for it
/// (`MHLEN` holds an IPv6 and a TCP header with their options: the C's `CTASSERT`s).
fn put_hdr<T: hal::user::AbiPod>(d: &mut [u8], off: usize, v: &T) {
    if !pod::put(d, off, v) {
        kpanic(format_args!(
            "tcp: a header of {} bytes at {off} of {}",
            size_of::<T>(),
            d.len()
        ));
    }
}

/// `tcp_respond`: send a single message to the TCP at address specified by the given TCP/IP
/// header. If `th0` is `None`, then we make a copy of the tcpiphdr at `template` and send
/// directly to the addressed host. This is used to force keep alive messages out using the
/// TCP template for a connection `tp->t_template`. If `th0` is given then we send a message
/// back to the TCP which originated the segment, and discard the mbuf containing it and any
/// other attached mbufs.
///
/// In any case the ack and sequence number of the transmitted segment are as specified by
/// the parameters. `template` is the IP header (and, without `th0`, the TCP header after
/// it); `net` the caller's proof of the net lock (`tp.net` with a control block).
#[allow(clippy::too_many_arguments)]
pub fn tcp_respond<G: ProcGlue>(
    tp: Option<&Tp<'_, '_, G>>,
    template: &[u8],
    th0: Option<&Tcphdr>,
    ack: TcpSeq,
    seq: TcpSeq,
    flags: u8,
    rtableid: u32,
    now: u64,
    net: &NetHeld<'_>,
) {
    let mut flags = flags;
    let mut win: i64 = 0;
    // af on wire
    let af = match tp {
        Some(tp) => {
            win = tp.so.so_rcv.sbspace();
            // If this is called with an unconnected socket/tp/pcb (tp->pf is 0), we lose.
            tp.st.pf
        }
        None => {
            if template.first().is_some_and(|b| b >> 4 == 6) {
                AF_INET6_I32
            } else {
                AF_INET_I32
            }
        }
    };

    let Some(mut m) = m_gethdr(M_DONTWAIT, MT_HEADER) else {
        return;
    };
    m.data_add(MAX_LINKHDR.load(Ordering::Relaxed) as isize);

    let (nh, mut th, mut tlen, thoff) = match af {
        AF_INET6_I32 => {
            let tlen = size_of::<Ip6Hdr>() + size_of::<Tcphdr>();
            let mut ip6: Ip6Hdr = read_hdr(template, 0);
            let th = match th0 {
                Some(th0) => {
                    core::mem::swap(&mut ip6.ip6_dst, &mut ip6.ip6_src);
                    *th0
                }
                None => read_hdr(template, size_of::<Ip6Hdr>()),
            };
            (RespondHdr::V6(ip6), th, tlen, size_of::<Ip6Hdr>())
        }
        AF_INET_I32 => {
            let tlen = size_of::<Ip>() + size_of::<Tcphdr>();
            let mut ip: Ip = read_hdr(template, 0);
            let th = match th0 {
                Some(th0) => {
                    core::mem::swap(&mut ip.ip_dst.s_addr, &mut ip.ip_src.s_addr);
                    *th0
                }
                None => read_hdr(template, size_of::<Ip>()),
            };
            (RespondHdr::V4(ip), th, tlen, size_of::<Ip>())
        }
        // The C's switch has no other case: nothing is built, nothing sent.
        _ => return,
    };
    if th0.is_some() {
        core::mem::swap(&mut th.th_dport, &mut th.th_sport);
    } else {
        flags = TH_ACK;
    }

    th.th_seq = htonl(seq);
    th.th_ack = htonl(ack);
    th.set_th_x2(0);
    th.set_th_off((size_of::<Tcphdr>() >> 2) as u8);
    th.th_flags = flags;
    if let Some(tp) = tp {
        win >>= tp.st.rcv_scale;
    }
    if win > i64::from(TCP_MAXWIN) {
        win = i64::from(TCP_MAXWIN);
    }
    th.th_win = htons(win as u16);
    th.th_urp = 0;

    // The room the headers and the timestamp option take, written in place below.
    m.m_len()
        .set((tlen + usize::from(TCPOLEN_TSTAMP_APPA)) as u32);
    let Some(d) = m.data_mut() else {
        kpanic(format_args!("tcp_respond: a new mbuf is read-only"));
    };
    if let Some(tp) = tp
        && tp.st.t_flags & (TF_REQ_TSTMP | TF_NOOPT) == TF_REQ_TSTMP
        && flags & TH_RST == 0
        && tp.st.has_flags(TF_RCVD_TSTMP)
    {
        // Form timestamp option as shown in appendix A of RFC 1323.
        let lp = thoff + size_of::<Tcphdr>();
        let words = [
            htonl(TCPOPT_TSTAMP_HDR),
            htonl((now as u32).wrapping_add(tp.st.ts_modulate)),
            htonl(tp.st.ts_recent),
        ];
        for (i, w) in words.iter().enumerate() {
            d[lp + 4 * i..lp + 4 * i + 4].copy_from_slice(&w.to_ne_bytes());
        }
        tlen += usize::from(TCPOLEN_TSTAMP_APPA);
        th.set_th_off(((size_of::<Tcphdr>() + usize::from(TCPOLEN_TSTAMP_APPA)) >> 2) as u8);
    }
    put_hdr(d, thoff, &th);
    // The network header's last members, as the C fills them before its output call.
    match nh {
        RespondHdr::V6(mut ip6) => {
            ip6.ip6_flow = htonl(0x6000_0000);
            ip6.ip6_nxt = IPPROTO_TCP as u8;
            // XXX; `in6_selecthlim(NULL)` is `ip6_defhlim`
            ip6.ip6_hlim = match tp {
                Some(tp) => in6_selecthlim(tp.inp),
                None => IP6_DEFHLIM.load(Ordering::Relaxed),
            } as u8;
            ip6.ip6_plen = htons((tlen - size_of::<Ip6Hdr>()) as u16);
            put_hdr(d, 0, &ip6);
        }
        RespondHdr::V4(mut ip) => {
            ip.ip_len = htons(tlen as u16);
            ip.ip_ttl = IP_DEFTTL.load(Ordering::Relaxed) as u8;
            ip.ip_tos = 0;
            put_hdr(d, 0, &ip);
        }
    }

    m.m_len().set(tlen as u32);
    let ph = m.m_pkthdr();
    ph.len.set(tlen as i32);
    ph.ph_ifidx.set(0);
    ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_OUT);

    // force routing table
    ph.ph_rtableid.set(match tp {
        Some(tp) => tp.inp.inp_rtableid.get(),
        None => rtableid,
    });

    let seclevel = tp.map(|tp| tp.inp.inp_seclevel.get());
    match af {
        AF_INET6_I32 => {
            // The sticky options and the route cache, lent for the call (the caller holds
            // the socket lock with `tp`).
            let sticky = tp.map(|tp| tp.inp.outputopts6());
            let mut ro = tp.map(|tp| tp.inp.route());
            // The C ignores the result: a lost reply is a lost segment.
            let _ = ip6_output(
                m,
                sticky
                    .as_ref()
                    .and_then(|s| s.as_deref())
                    .map(Ip6Pktopts::as_opts),
                ro.as_deref_mut(),
                0,
                None,
                seclevel.as_ref(),
                net,
            );
        }
        _ => {
            let mut ro = tp.map(|tp| tp.inp.route());
            // The C ignores the result: a lost reply is a lost segment.
            let _ = ip_output(
                m,
                None,
                ro.as_deref_mut(),
                if ip_mtudisc.load(Ordering::Relaxed) != 0 {
                    IP_MTUDISC
                } else {
                    0
                },
                None,
                seclevel.as_ref(),
                0,
                net,
            );
        }
    }
}

/// A header read out of `template` at `off` (unaligned, as the C's `memcpy`); `panic(9)`
/// when the bytes are shorter (LZ's `read_from`; the C's callers pass whole headers).
fn read_hdr<T: hal::user::AbiPod>(template: &[u8], off: usize) -> T {
    match pod::at::<T>(template, off) {
        Some(h) => h,
        None => kpanic(format_args!(
            "tcp_respond: a template of {} bytes, a header of {} at {off}",
            template.len(),
            size_of::<T>()
        )),
    }
}

/// `tcp_free`: the control block `tcp_close` closed, put back (the C's `pool_put(&tcpcb_pool,
/// tp)` and `inp->inp_ppcb = NULL`, here a few statements later under the same socket lock):
/// the slot emptied with the socket lock's part `PCB` (`pcb`), its timers' handles of the pcb
/// dropped with it (D-11).
pub fn tcp_free<G: ProcGlue>(inp: &Inpcb<G>, pcb: &mut WitnessPart<'_>, closed: Closed) {
    let Closed(()) = closed;
    drop(inp.take_tcpcb(pcb));
}

/// `tcp_rscale`: compute proper scaling value for receiver window from buffer space.
pub fn tcp_rscale(st: &mut TcpState, hiwat: u64) {
    st.request_r_scale = 0;
    while st.request_r_scale < TCP_MAX_WINSHIFT
        && u64::from(TCP_MAXWIN) << st.request_r_scale < hiwat
    {
        st.request_r_scale += 1;
    }
}

/// `tcp_set_iss_tsm`: generate new ISNs with a method based on RFC1948.
pub fn tcp_set_iss_tsm<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) {
    let inp = tp.inp;
    let rdomain = rtable_l2(inp.inp_rtableid.get());

    let iss = {
        let mut g = TCP_ISS.lock();
        *g = g.wrapping_add(TCP_ISS_CONN_INC);
        *g
    };

    // The context is not `Copy`: the clone is the C's struct copy, wiped at its drop.
    let mut ctx = TCP_SECRET_CTX.get().cloned().unwrap_or_default();
    ctx.update(&rdomain.to_ne_bytes());
    ctx.update(&inp.inp_lport.get().to_ne_bytes());
    ctx.update(&inp.inp_fport.get().to_ne_bytes());
    if tp.st.pf == AF_INET6_I32 {
        ctx.update(&inp.inp_laddr6.bytes());
        ctx.update(&inp.inp_faddr6.bytes());
    } else {
        ctx.update(&inp.inp_laddr.get().s_addr.to_ne_bytes());
        ctx.update(&inp.inp_faddr.get().s_addr.to_ne_bytes());
    }
    let digest: [u8; SHA512_DIGEST_LENGTH] = ctx.finalize();
    let word =
        |i: usize| u32::from_ne_bytes([digest[i], digest[i + 1], digest[i + 2], digest[i + 3]]);
    tp.st.iss = word(0).wrapping_add(iss);
    tp.st.ts_modulate = word(4);
}

/// `tcp_signature`: the RFC 2385 MD5 digest of a segment (pseudo header, TCP header with a
/// zero checksum, data, key) into `sig`; `false` (the C's -1) when the data cannot be walked.
/// With `doswap` the header's sequence numbers, window and urgent pointer are in host order
/// and are swapped back first. The SA's key is lent by IPsec's leg to the MD5 update, never
/// copied out (D-9); `tdb` is the caller's counted reference to the SA.
pub fn tcp_signature(
    tdb: &TdbRef,
    af: i32,
    m: &Mbuf,
    th: &Tcphdr,
    iphlen: i32,
    doswap: bool,
    sig: &mut [u8; MD5_DIGEST_LENGTH],
) -> bool {
    signature_digest(af, m, th, iphlen, doswap, sig, |ctx| match ipsec() {
        Some(i) => i.tdb_amxkey(tdb, &mut |key| ctx.update(key)),
        // A TdbRef is made only by IPsec's legs.
        None => kpanic(format_args!("tcp_signature: an SA without IPsec")),
    })
}

/// [`tcp_signature`]'s digest, the key fed to the context last by `key` (IPsec's lend).
fn signature_digest(
    af: i32,
    m: &Mbuf,
    th: &Tcphdr,
    iphlen: i32,
    doswap: bool,
    sig: &mut [u8; MD5_DIGEST_LENGTH],
    key: impl FnOnce(&mut Md5Ctx),
) -> bool {
    let mut ctx = Md5Ctx::new();

    match af {
        0 | AF_INET_I32 => {
            let ip: Ip = read_hdr(m.data(), 0);
            let ippseudo = Ippseudo {
                ippseudo_src: ip.ip_src,
                ippseudo_dst: ip.ip_dst,
                ippseudo_pad: 0,
                ippseudo_p: IPPROTO_TCP as u8,
                ippseudo_len: htons((m.m_pkthdr().len.get() - iphlen) as u16),
            };
            let mut b = [0u8; size_of::<Ippseudo>()];
            put_hdr(&mut b, 0, &ippseudo);
            ctx.update(&b);
        }
        AF_INET6_I32 => {
            let ip6: Ip6Hdr = read_hdr(m.data(), 0);
            let mut ip6pseudo = Ip6HdrPseudo {
                ip6ph_src: ip6.ip6_src,
                ip6ph_dst: ip6.ip6_dst,
                ip6ph_len: htonl((m.m_pkthdr().len.get() - iphlen) as u32),
                ip6ph_zero: [0; 3],
                ip6ph_nxt: IPPROTO_TCP as u8,
            };
            in6_clearscope(&mut ip6pseudo.ip6ph_src);
            in6_clearscope(&mut ip6pseudo.ip6ph_dst);
            let mut b = [0u8; size_of::<Ip6HdrPseudo>()];
            put_hdr(&mut b, 0, &ip6pseudo);
            ctx.update(&b);
        }
        _ => {}
    }

    let mut th0 = *th;
    th0.th_sum = 0;

    if doswap {
        th0.th_seq = htonl(th0.th_seq);
        th0.th_ack = htonl(th0.th_ack);
        th0.th_win = htons(th0.th_win);
        th0.th_urp = htons(th0.th_urp);
    }
    let mut b = [0u8; size_of::<Tcphdr>()];
    put_hdr(&mut b, 0, &th0);
    ctx.update(&b);

    let thlen = i32::from(th.th_off()) * size_of::<u32>() as i32;
    let len = m.m_pkthdr().len.get() - iphlen - thlen;

    // tcp_signature_apply
    if len > 0
        && m_apply(m, iphlen + thlen, len, |data| {
            ctx.update(data);
            Ok(())
        })
        .is_err()
    {
        return false;
    }

    key(&mut ctx);
    *sig = ctx.finalize();

    true
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the kernel's `tcp_subr.rs`: window scaling, the RFC 2385 digests against
    // an independent MD5 of the same bytes, a reply to a segment as the wire sees it.

    use hal::netlock::NETLOCK;
    use hal::sys::netinet_in::InAddr;
    use hal::sys::netinet6_in6::In6Addr;

    use super::*;
    use crate::net::mbuf::{MT_DATA, m_gethdr};
    use crate::netinet::in_pcb::testinet;

    #[test]
    fn rscale_requests_the_smallest_sufficient_shift() {
        let mut st = TcpState::new();
        for (hiwat, shift) in [
            (16 * 1024, 0),
            (65535, 0),
            (65536, 1),
            (2 * 1024 * 1024, 6),
            (u64::MAX, TCP_MAX_WINSHIFT),
        ] {
            tcp_rscale(&mut st, hiwat);
            assert_eq!(st.request_r_scale, shift, "hiwat {hiwat}");
        }
    }

    /// A packet of `hdr` (an IP header), a TCP header with 20 bytes of options (the
    /// signature option, NOP, EOL) and "hello", in an mbuf; the TCP header in network order.
    fn segment(hdr: &[u8]) -> (MbufBox, Tcphdr) {
        let mut th = Tcphdr {
            th_sport: htons(1234),
            th_dport: htons(179),
            th_seq: htonl(0x0102_0304),
            th_ack: htonl(0x0a0b_0c0d),
            th_flags: 0x18,
            th_win: htons(512),
            th_sum: 0xbeef,
            ..Tcphdr::default()
        };
        th.set_th_off(10);
        let mut p = std::vec::Vec::from(hdr);
        let mut b = [0u8; 20];
        assert!(pod::put(&mut b, 0, &th));
        p.extend_from_slice(&b);
        p.extend_from_slice(&[19, 18]);
        p.extend_from_slice(&[0; 16]);
        p.extend_from_slice(&[1, 0]);
        p.extend_from_slice(b"hello");
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(p.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(&p);
        m.m_pkthdr().len.set(p.len() as i32);
        (m, th)
    }

    /// The test's key, fed as IPsec's leg feeds the SA's.
    fn key(ctx: &mut Md5Ctx) {
        ctx.update(b"emibsd-tcp-md5-key");
    }

    #[test]
    fn signatures_match_an_independent_md5_of_the_same_bytes() {
        // The digests: Python's hashlib.md5 over the pseudo header, the TCP header with a
        // zero checksum (the options left out), the data and the key (the scratch script
        // md5vec.py of N6.6a2).
        const V4: [u8; 16] = [
            60, 255, 188, 24, 215, 36, 5, 100, 234, 118, 221, 5, 228, 160, 216, 162,
        ];
        const V6: [u8; 16] = [
            56, 216, 142, 22, 36, 33, 102, 91, 125, 135, 29, 84, 250, 81, 254, 245,
        ];
        let (_g, _p) = testinet::setup();
        let ip = Ip {
            ip_vhl: 0x45,
            ip_len: htons(65),
            ip_ttl: 64,
            ip_p: IPPROTO_TCP as u8,
            ip_src: InAddr {
                s_addr: u32::from_ne_bytes([10, 0, 0, 1]),
            },
            ip_dst: InAddr {
                s_addr: u32::from_ne_bytes([10, 0, 0, 2]),
            },
            ..Ip::default()
        };
        let mut h = [0u8; 20];
        assert!(pod::put(&mut h, 0, &ip));
        let (m, th) = segment(&h);
        let mut sig = [0u8; 16];
        assert!(signature_digest(
            AF_INET_I32,
            &m,
            &th,
            20,
            false,
            &mut sig,
            key
        ));
        assert_eq!(sig, V4);
        // af 0 is the C's "default to PF_INET".
        assert!(signature_digest(0, &m, &th, 20, false, &mut sig, key));
        assert_eq!(sig, V4);
        // The header in host order, swapped back for the digest (doswap).
        let host = Tcphdr {
            th_seq: u32::from_be(th.th_seq),
            th_ack: u32::from_be(th.th_ack),
            th_win: u16::from_be(th.th_win),
            th_urp: u16::from_be(th.th_urp),
            ..th
        };
        assert!(signature_digest(
            AF_INET_I32,
            &m,
            &host,
            20,
            true,
            &mut sig,
            key
        ));
        assert_eq!(sig, V4);
        // The checksum does not count; any other header byte does.
        let mut other = th;
        other.th_sum = 0;
        assert!(signature_digest(
            AF_INET_I32,
            &m,
            &other,
            20,
            false,
            &mut sig,
            key
        ));
        assert_eq!(sig, V4);
        other.th_win ^= 1;
        assert!(signature_digest(
            AF_INET_I32,
            &m,
            &other,
            20,
            false,
            &mut sig,
            key
        ));
        assert_ne!(sig, V4);

        // IPv6, a link-local source whose embedded scope is cleared for the pseudo header.
        let a6 = |b: [u8; 16]| In6Addr { s6_addr: b };
        let mut src = [0u8; 16];
        src[..4].copy_from_slice(&[0xfe, 0x80, 0x00, 0x01]);
        src[15] = 1;
        let mut dst = [0u8; 16];
        dst[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
        dst[15] = 2;
        let ip6 = Ip6Hdr {
            ip6_flow: htonl(0x6000_0000),
            ip6_plen: htons(45),
            ip6_nxt: IPPROTO_TCP as u8,
            ip6_hlim: 64,
            ip6_src: a6(src),
            ip6_dst: a6(dst),
        };
        let mut h6 = [0u8; 40];
        assert!(pod::put(&mut h6, 0, &ip6));
        let (m6, th6) = segment(&h6);
        assert!(signature_digest(
            AF_INET6_I32,
            &m6,
            &th6,
            40,
            false,
            &mut sig,
            key
        ));
        assert_eq!(sig, V6);
    }

    #[test]
    fn a_reset_answers_a_segment_with_the_addresses_and_ports_swapped() {
        let (_g, _p) = testinet::setup();
        let (_ifp, d) = testinet::iface(b"trsp0", [10, 9, 0, 1]);
        // The segment: 10.9.0.2:1234 -> 10.9.0.1:80, its header in host order as
        // tcp_input leaves it (tcp_respond swaps the ports, takes seq and ack as given).
        let ip = Ip {
            ip_vhl: 0x45,
            ip_len: htons(40),
            ip_ttl: 7,
            ip_p: IPPROTO_TCP as u8,
            ip_src: InAddr {
                s_addr: u32::from_ne_bytes([10, 9, 0, 2]),
            },
            ip_dst: InAddr {
                s_addr: u32::from_ne_bytes([10, 9, 0, 1]),
            },
            ..Ip::default()
        };
        let mut template = [0u8; 20];
        assert!(pod::put(&mut template, 0, &ip));
        let th0 = Tcphdr {
            th_sport: htons(1234),
            th_dport: htons(80),
            th_seq: 1000,
            th_flags: hal::sys::netinet_tcp::TH_SYN,
            ..Tcphdr::default()
        };
        let before = d.sent.load(Ordering::Relaxed);
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            tcp_respond::<testinet::G>(
                None,
                &template,
                Some(&th0),
                1001,
                0,
                TH_RST | TH_ACK,
                0,
                5,
                &net,
            );
        }
        assert_eq!(d.sent.load(Ordering::Relaxed), before + 1);
        let p = d.last.lock().expect("the capture").clone();
        assert_eq!(p.len(), 40);
        let rip: Ip = pod::at(&p, 0).expect("the IP header");
        assert_eq!(u16::from_be(rip.ip_len), 40);
        assert_eq!(rip.ip_src.s_addr, u32::from_ne_bytes([10, 9, 0, 1]));
        assert_eq!(rip.ip_dst.s_addr, u32::from_ne_bytes([10, 9, 0, 2]));
        assert_eq!(i32::from(rip.ip_ttl), IP_DEFTTL.load(Ordering::Relaxed));
        assert_eq!(rip.ip_tos, 0);
        let rth: Tcphdr = pod::at(&p, 20).expect("the TCP header");
        assert_eq!(u16::from_be(rth.th_sport), 80);
        assert_eq!(u16::from_be(rth.th_dport), 1234);
        assert_eq!(u32::from_be(rth.th_seq), 0);
        assert_eq!(u32::from_be(rth.th_ack), 1001);
        assert_eq!(rth.th_off(), 5);
        assert_eq!(rth.th_x2(), 0);
        assert_eq!(rth.th_flags, TH_RST | TH_ACK);
        assert_eq!(rth.th_win, 0, "no control block: no window");
        assert_eq!(rth.th_urp, 0);
    }
}
/* </TESTS> */
