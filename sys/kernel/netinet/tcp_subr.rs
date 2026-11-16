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
//! `tcp_init` sets up the pools, `tcbtable`, the ISN secret and the SYN cache. `tcp_template`
//! builds the skeletal TCP/IP header a connection sends with; `tcp_respond` sends a bare
//! segment (an ACK, a RST, a keepalive probe) either from a connection's template or in reply
//! to a received segment. `tcp_newtcpcb`, `tcp_close`, `tcp_free` and `tcp_freeq` create and
//! tear down control blocks. `tcp_set_iss_tsm` derives initial sequence numbers and timestamp
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
//!   names no machine constant: the same number. Before `mbinit` (a host test without the
//!   mbuf layer) no limit is set.
//! - `tcp_init`'s `icmp6_mtudisc_callback_register(tcp6_mtudisc_callback)` arrives with
//!   `tcp6_mtudisc_callback` (N6.6a3, with `tcp_ctlinput`); until then the kernel's TCP is
//!   unregistered and nothing calls `tcp_init` but the host tests.
//! - `tcp_newtcpcb` returns the new control block opened ([`Tp`]) or `None`, the C's pointer
//!   or NULL; `tcp_close` answers [`Closed`] for the C's NULL; `tcp_freeq` returns `bool`
//!   (the C's 0 or 1).
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
use hal::pool::{BoxPool, PR_NOWAIT, PR_WAITOK, PoolArc};
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
    M_DONTWAIT, M_TCP_CSUM_OUT, M_WAIT, MAX_LINKHDR, Mbuf, MbufBox, m_apply, m_get, m_gethdr,
};
use crate::net::rtable::rtable_l2;
use crate::net::socket::{InetLocked, soisdisconnected};
use crate::netinet::in_pcb::{INP_IPV6, Inpcb, in_pcbdetach, in_pcbinit};
use crate::netinet::ip_output::ip_output;
use crate::netinet::ip_var::{IP_DEFTTL, IP_MTUDISC, ip_mtudisc};
use crate::netinet::tcp_syncache::{syn_cache_cleanup, syn_cache_init};
use crate::netinet::tcp_timer::TCPTV_SRTTDFLT;
use crate::netinet::tcp_timer::{
    TCPT_NTIMERS, TCPTV_MIN, TCPTV_REXMTMAX, TCPTV_SRTTBASE, tcp_canceltimers, tcp_timer_freesack,
    tcp_timer_init, tcpt_rangeset,
};
use crate::netinet::tcp_var::{
    Sackhole, TCP_RTT_BASE_SHIFT, TCP_RTTVAR_SHIFT, TF_NOOPT, TF_RCVD_TSTMP, TF_REQ_SCALE,
    TF_REQ_TSTMP, TcpState, Tcpcb, Tcpqent, TcpstatCounters, Tp, intotcpcb, tcp_rexmtval,
    tcpstat_inc,
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

/// `TCB_INITIAL_HASH_SIZE`.
const TCB_INITIAL_HASH_SIZE: i32 = 128;

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

/// `tcp_init`: TCP initialization (from `domaininit`; a second call, a test glue's, leaves
/// what exists as it is).
pub fn tcp_init<G: ProcGlue>() {
    *TCP_ISS.lock() = 1; // wrong
    // 0 is treated special so add 1, 63 bits to count is enough
    let mut start = [0u8; 8];
    hal::random::arc4random_buf(&mut start);
    TCP_STARTTIME.store(1 + u64::from_ne_bytes(start) / 2, Ordering::Relaxed);
    let t = &crate::net::domain::net::<G>().tcp;
    let _ = t.tcpcb_pool.init(IPL_SOFTNET, 0, "tcpcb");
    let _ = TCPQE_POOL.init(IPL_SOFTNET, 0, "tcpqe");
    if TCP_REASS_LIMIT.load(Ordering::Relaxed) == 0 {
        let n = crate::net::mbuf::nmbclust() / 8;
        TCP_REASS_LIMIT.store(i32::try_from(n).unwrap_or(i32::MAX), Ordering::Relaxed);
    }
    let reass = TCP_REASS_LIMIT.load(Ordering::Relaxed);
    // The C ignores the result, as below: the limit only caps the pool.
    if reass > 0 {
        let _ = TCPQE_POOL.set_hardlimit(reass as u32);
    }
    let _ = SACKHL_POOL.init(IPL_SOFTNET, 0, "sackhl");
    let _ = SACKHL_POOL.set_hardlimit(tcp_sackhole_limit.load(Ordering::Relaxed) as u32);
    in_pcbinit(&t.tcbtable, TCB_INITIAL_HASH_SIZE);
    in_pcbinit(&t.tcb6table, TCB_INITIAL_HASH_SIZE);
    // tcpcounters = counters_alloc(tcps_ncounters): a static (tcp_var.rs).

    let mut secret = [0u8; 16];
    hal::random::arc4random_buf(&mut secret);
    let mut ctx = Sha512Ctx::new();
    ctx.update(&secret);
    // Set once: a second call keeps the first secret, as the first boot's. The stack's copy
    // is wiped (the C draws the bytes into its global).
    let _ = TCP_SECRET.set(secret);
    libkern::explicit_bzero(&mut secret);
    let _ = TCP_SECRET_CTX.set(ctx);

    // Since sizeof(struct ip6_hdr) > sizeof(struct ip), we do max length checks/computations
    // only on the former.
    let hdrs = (size_of::<Ip6Hdr>() + size_of::<Tcphdr>()) as i32;
    if crate::net::mbuf::MAX_PROTOHDR.load(Ordering::Relaxed) < hdrs {
        crate::net::mbuf::MAX_PROTOHDR.store(hdrs, Ordering::Relaxed);
    }
    if (MAX_LINKHDR.load(Ordering::Relaxed) + hdrs) as usize > crate::net::mbuf::MHLEN {
        kpanic(format_args!("tcp_init"));
    }

    // icmp6_mtudisc_callback_register(tcp6_mtudisc_callback): N6.6a3's (the deviations).

    // Initialize the compressed state engine.
    syn_cache_init::<G>();
}

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
pub(crate) fn put_hdr<T: hal::user::AbiPod>(d: &mut [u8], off: usize, v: &T) {
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

/// `tcp_newtcpcb`: create a new TCP control block, making an empty reassembly queue and
/// hooking it to the argument protocol control block (`inp`, whose socket lock `il` holds:
/// the slot opens with its part `PCB`). The new block, opened, or `None` when no item can be
/// had (`wait` `M_WAIT` sleeps for one, as the C's `PR_WAITOK`).
pub fn tcp_newtcpcb<'a, 'w, G: ProcGlue>(
    inp: &'a Inpcb<G>,
    il: &'a mut InetLocked<'w>,
    wait: i32,
) -> Option<Tp<'a, 'w, G>> {
    let pool = &crate::net::domain::net::<G>().tcp.tcpcb_pool;
    let flags = if wait == M_WAIT { PR_WAITOK } else { PR_NOWAIT };
    let so_lock = inp.socket().so_lock.lock().id();
    let cb = pool.get(flags, || {
        let mut cb = Tcpcb::new(so_lock);
        let st = cb.st.get_mut_owned();
        // TAILQ_INIT(&tp->t_segq): the queue is empty.
        st.t_maxseg = TCP_MSSDFLT.load(Ordering::Relaxed) as u16;
        st.t_maxopd = 0;

        st.sack_enable = TCP_DO_SACK.load(Ordering::Relaxed);
        st.t_flags = if TCP_DO_RFC1323.load(Ordering::Relaxed) != 0 {
            TF_REQ_SCALE | TF_REQ_TSTMP
        } else {
            0
        };
        // Init srtt to TCPTV_SRTTBASE (0), so we can tell that we have no rtt estimate. Set
        // rttvar so that srtt + 2 * rttvar gives reasonable initial retransmit time.
        st.t_srtt = TCPTV_SRTTBASE;
        st.t_rttvar =
            TCP_RTTDFLT.load(Ordering::Relaxed) << (TCP_RTTVAR_SHIFT + TCP_RTT_BASE_SHIFT - 1);
        st.t_rttmin = TCPTV_MIN as u32;
        st.t_rxtcur = tcpt_rangeset(tcp_rexmtval(st), TCPTV_MIN, TCPTV_REXMTMAX);
        st.snd_cwnd = u64::from(TCP_MAXWIN) << TCP_MAX_WINSHIFT;
        st.snd_ssthresh = u64::from(TCP_MAXWIN) << TCP_MAX_WINSHIFT;

        st.t_pmtud_mtu_sent = 0;
        st.t_pmtud_mss_acked = 0;

        st.pf = if inp.has_flags(INP_IPV6) {
            AF_INET6_I32
        } else {
            AF_INET_I32
        };
        cb
    })?;

    // tp->t_inpcb = inp: the timers' argument (D-11).
    for i in 0..TCPT_NTIMERS {
        tcp_timer_init(&cb, inp, i);
    }

    if inp.has_flags(INP_IPV6) {
        let mut ip6: Ip6Hdr = inp.inp_ipv6.get();
        ip6.ip6_hlim = IP6_DEFHLIM.load(Ordering::Relaxed) as u8;
        inp.inp_ipv6.set(ip6);
    } else {
        let mut ip = inp.inp_ip.get();
        ip.ip_ttl = IP_DEFTTL.load(Ordering::Relaxed) as u8;
        inp.inp_ip.set(ip);
    }

    // inp->inp_ppcb = (caddr_t)tp
    if inp.set_tcpcb(&mut il.pcb, cb).is_some() {
        kpanic(format_args!(
            "tcp_newtcpcb: inpcb {inp:p} has a control block"
        ));
    }
    intotcpcb(inp, il)
}

/// `tcp_close`: close a TCP control block: discard all space held by the tcp, discard
/// internet protocol block, wake up any sleepers. The block is consumed: [`Closed`] tells the
/// frame that owns the socket lock's parts to free it ([`tcp_free`]) before it unlocks, the
/// C's `pool_put` after the same steps (the queue, the timers, the SYN cache, the holes, the
/// template, `soisdisconnected`, `in_pcbdetach`, `tcps_closed`).
pub fn tcp_close<G: ProcGlue>(mut tp: Tp<'_, '_, G>) -> Closed {
    let inp = tp.inp;
    let so = tp.so;

    // free the reassembly queue, if any
    tcp_freeq(tp.st);

    tcp_canceltimers(&mut tp);
    syn_cache_cleanup(&tp);

    // Free SACK holes.
    tcp_timer_freesack(tp.st);

    // m_free(tp->t_template)
    drop(tp.st.t_template.take());
    // inp->inp_ppcb = NULL; pool_put(&tcpcb_pool, tp): tcp_free's, after these steps.
    soisdisconnected(so);
    let Some(h) = PoolArc::from_ref(inp) else {
        kpanic(format_args!("tcp_close: inpcb {inp:p}: not a pool item"));
    };
    in_pcbdetach(h, tp.net);
    tcpstat_inc(TcpstatCounters::TcpsClosed);
    Closed(())
}

/// `tcp_freeq`: frees the reassembly queue, one entry at a time; `true` when it held a
/// segment.
pub fn tcp_freeq(st: &mut TcpState) -> bool {
    let mut rv = false;

    while let Some(mut qe) = st.t_segq.take() {
        if let Some(next) = qe.tcpqe_q.take() {
            let _ = st.t_segq.set(next);
        }
        // m_freem(qe->tcpqe_m); pool_put(&tcpqe_pool, qe)
        drop(qe);
        rv = true;
    }
    rv
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

/// The host tests' TCP connections: control blocks of the kernel's TCP over the Internet
/// socket fixture (`in_pcb::testinet`), its socket lock taken as a callout takes it.
#[cfg(test)]
pub(crate) mod testtcp {
    use std::sync::MutexGuard;

    use hal::netlock::{NETLOCK, NetHeld};

    use super::*;
    use crate::net::sockbuf::SS_NOFDREF;
    use crate::net::socket::SocketRef;
    use crate::netinet::in_pcb::{InpcbRef, in_pcbsolock, testinet};
    use crate::proc::Thread;

    /// The fixture's glue.
    pub(crate) type G = testinet::G;

    /// `testinet::setup` and `tcp_init` (once per glue: a second call keeps the pools).
    pub(crate) fn setup() -> (MutexGuard<'static, ()>, &'static Thread<G>) {
        let r = testinet::setup();
        tcp_init::<G>();
        r
    }

    /// A socket with a pcb in `tcbtable` and a control block (`tcp_attach`'s two steps).
    pub(crate) fn conn() -> (SocketRef<G>, InpcbRef<G>) {
        let (so, inp) = testinet::pcb(&crate::net::domain::net::<G>().tcp.tcbtable);
        with_lock(&inp, |inp, il| {
            assert!(tcp_newtcpcb(inp, il, M_WAIT).is_some(), "a control block");
        });
        (so, inp)
    }

    /// Runs `f` with the socket of `inp` locked in parts under the shared net lock (a
    /// callout's holding); `panic` when the socket is detached.
    pub(crate) fn with_lock<R>(
        inp: &Inpcb<G>,
        f: impl FnOnce(&Inpcb<G>, &mut InetLocked<'_>) -> R,
    ) -> R {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut so = in_pcbsolock(inp, &net).expect("an attached socket");
        let mut il = so.inet().expect("a kernel protocol's socket lock");
        f(inp, &mut il)
    }

    /// Runs `f` with the control block of `inp` (`intotcpcb`); what it closes is freed.
    pub(crate) fn with_tp<R>(
        inp: &Inpcb<G>,
        f: impl FnOnce(Tp<'_, '_, G>) -> (R, Option<Closed>),
    ) -> R {
        with_lock(inp, |inp, il| {
            let tp = intotcpcb(inp, il).expect("a control block");
            let (r, closed) = f(tp);
            if let Some(c) = closed {
                tcp_free(inp, &mut il.pcb, c);
            }
            r
        })
    }

    /// `tcp_close` and `tcp_free` of `inp`'s connection, the socket let go (no file holds it).
    pub(crate) fn close(so: SocketRef<G>, inp: InpcbRef<G>) {
        so.st_set(SS_NOFDREF);
        with_tp(&inp, |tp| ((), Some(tcp_close(tp))));
        drop(inp);
        drop(so);
    }

    /// The items out of the "tcpcb" pool.
    pub(crate) fn tcpcbs() -> u64 {
        crate::net::domain::net::<G>().tcp.tcpcb_pool.nout()
    }
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

    #[test]
    fn a_control_block_starts_as_the_cs_and_closes_back_to_the_pools() {
        use crate::netinet::tcp_timer::{TCPT_REXMT, tcp_timer_arm, tcp_timer_isarmed};
        use crate::netinet::tcp_var::TF_REQ_SCALE;
        let (_g, _p) = testtcp::setup();
        let tcpcbs = testtcp::tcpcbs();
        let (so, inp) = testtcp::conn();
        assert_eq!(testtcp::tcpcbs(), tcpcbs + 1);
        // The pcb's handles: ours, the socket's anchor, and one per timer (D-11).
        let handles = PoolArc::count(&inp);
        testtcp::with_tp(&inp, |mut tp| {
            let st = &*tp.st;
            assert_eq!(st.t_maxseg, 512);
            assert_eq!(st.sack_enable, 1);
            assert_eq!(st.t_flags, TF_REQ_SCALE | TF_REQ_TSTMP);
            assert_eq!(st.t_srtt, 0);
            assert_eq!(st.t_rttvar, 3000 << 3);
            assert_eq!(st.t_rttmin, 1000);
            // TCPT_RANGESET(TCP_REXMTVAL): ((0 >> 3) + 24000) >> 2.
            assert_eq!(st.t_rxtcur, 6000);
            assert_eq!(st.snd_cwnd, 65535 << 14);
            assert_eq!(st.snd_ssthresh, 65535 << 14);
            assert_eq!(st.pf, AF_INET_I32);
            assert!(st.t_segq.get().is_none() && st.snd_holes.get().is_none());
            assert_eq!(
                i32::from(tp.inp.inp_ip.get().ip_ttl),
                IP_DEFTTL.load(Ordering::Relaxed)
            );
            tcp_timer_arm(&mut tp, TCPT_REXMT, 60_000);
            assert!(tcp_timer_isarmed(tp.st, TCPT_REXMT));
            assert!(tp.cb.t_timer[TCPT_REXMT].pending());
            ((), None)
        });
        assert!(handles >= TCPT_NTIMERS + 2, "{handles} handles");

        // tcp_close cancels the timers before the block goes (the slot still holds it).
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        let pending = testtcp::with_tp(&inp, |tp| {
            let cb = tp.cb;
            let closed = tcp_close(tp);
            (cb.t_timer[TCPT_REXMT].pending(), Some(closed))
        });
        assert!(!pending, "the retransmit timer cancelled by tcp_close");
        drop(so);
        // tcp_free: the item back, its five handles of the pcb gone with its timers.
        assert_eq!(testtcp::tcpcbs(), tcpcbs);
        assert_eq!(
            PoolArc::count(&inp),
            handles - TCPT_NTIMERS - 1,
            "the timers' and the anchor's"
        );
        // The pcb is detached: its socket's lock finds no pcb any more.
        {
            let w = hal::netlock::NETLOCK.read();
            let net = NetHeld::of(&w).expect("the net lock");
            assert!(crate::netinet::in_pcb::in_pcbsolock(&inp, &net).is_none());
        }
        // soisdisconnected.
        assert!(
            inp.socket()
                .so_rcv
                .has_state(crate::net::sockbuf::SS_CANTRCVMORE)
        );
    }

    #[test]
    fn a_template_and_a_keepalive_carry_the_connection() {
        use crate::netinet::tcp_var::TF_RCVD_TSTMP;
        let (_g, _p) = testtcp::setup();
        let (_ifp, d) = testinet::iface(b"ttpl0", [10, 9, 1, 1]);
        let (so, inp) = testtcp::conn();
        {
            let t = &crate::net::domain::net::<testinet::G>().tcp.tcbtable;
            let g = t.lock();
            inp.set_laddr(
                &g,
                InAddr {
                    s_addr: u32::from_ne_bytes([10, 9, 1, 1]),
                },
            );
            inp.set_faddr(
                &g,
                InAddr {
                    s_addr: u32::from_ne_bytes([10, 9, 1, 2]),
                },
            );
            inp.set_lport(&g, htons(7000));
            inp.set_fport(&g, htons(80));
        }
        let before = d.sent.load(Ordering::Relaxed);
        testtcp::with_tp(&inp, |mut tp| {
            let m = tcp_template(&mut tp).expect("a template");
            let bytes = std::vec::Vec::from(m.data());
            tp.st.t_template = Some(m);
            // The overlay: protocol, length, addresses; the TCP header's ports, th_off 5.
            let ov: Ipovly = pod::at(&bytes, 0).expect("the overlay");
            assert_eq!(ov.ih_pr, IPPROTO_TCP as u8);
            assert_eq!(u16::from_be(ov.ih_len), 20);
            assert_eq!(ov.ih_src.s_addr, u32::from_ne_bytes([10, 9, 1, 1]));
            assert_eq!(ov.ih_dst.s_addr, u32::from_ne_bytes([10, 9, 1, 2]));
            let th: Tcphdr = pod::at(&bytes, 20).expect("the header");
            assert_eq!(
                (u16::from_be(th.th_sport), u16::from_be(th.th_dport)),
                (7000, 80)
            );
            assert_eq!((th.th_off(), th.th_flags, th.th_seq), (5, 0, 0));
            // A second call fills the same mbuf again.
            let again = tcp_template(&mut tp).expect("the template");
            assert_eq!(again.data(), &bytes[..]);
            tp.st.t_template = Some(again);

            // A keepalive from the template: an ACK with the timestamp option.
            tp.st.set_flags(TF_RCVD_TSTMP);
            tp.st.ts_recent = 0x1122_3344;
            tp.st.ts_modulate = 5;
            let net = tp.net;
            tcp_respond(Some(&tp), &bytes, None, 77, 66, 0, 0, 100, net);
            ((), None)
        });
        assert_eq!(d.sent.load(Ordering::Relaxed), before + 1);
        let p = d.last.lock().expect("the capture").clone();
        assert_eq!(p.len(), 20 + 20 + 12);
        let th: Tcphdr = pod::at(&p, 20).expect("the header");
        assert_eq!(th.th_flags, TH_ACK, "a template's reply is an ACK");
        assert_eq!(th.th_off(), 8);
        assert_eq!((u32::from_be(th.th_seq), u32::from_be(th.th_ack)), (66, 77));
        assert_eq!(&p[40..44], &htonl(TCPOPT_TSTAMP_HDR).to_ne_bytes());
        assert_eq!(&p[44..48], &105u32.to_be_bytes());
        assert_eq!(&p[48..52], &0x1122_3344u32.to_be_bytes());
        testtcp::close(so, inp);
    }

    #[test]
    fn the_iss_comes_from_the_secret_and_the_counter() {
        let (_g, _p) = testtcp::setup();
        let (so, inp) = testtcp::conn();
        let (a, b) = testtcp::with_tp(&inp, |mut tp| {
            tcp_set_iss_tsm(&mut tp);
            let a = (tp.st.iss, tp.st.ts_modulate);
            tcp_set_iss_tsm(&mut tp);
            let b = (tp.st.iss, tp.st.ts_modulate);
            ((a, b), None)
        });
        // The same connection: the same digest, the counter 4096 further.
        assert_eq!(b.0.wrapping_sub(a.0), TCP_ISS_CONN_INC);
        assert_eq!(a.1, b.1);
        testtcp::close(so, inp);
    }
}
/* </TESTS> */
