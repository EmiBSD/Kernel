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
//! `tcp_signature` computes the RFC 2385 MD5 digest. `tcp_mtudisc` and
//! `tcp_mtudisc_increase` take a path MTU change: the route renewed, the segment size taken
//! again, the unacknowledged data sent again.
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
//! - `tcp_ctlinput` and `tcp6_ctlinput` take the address as the `SockaddrView` the protocol
//!   switch lends (an IPv4 one read as a `sockaddr_in` padded with zeroes past its bytes, as
//!   `bsd`'s adapter pads it), the argument as `CtlArg` (an argument of the other family, or
//!   an ICMP message too short for the header it returns and the eight TCP bytes after it,
//!   calls nothing: `icmp_input` hands no such message on), and the caller's proof of the
//!   net lock (the C's callers hold it). `tcp6_mtudisc_callback` takes it too
//!   (`icmp6_mtudisc_update`'s). The functions are reached by no protocol switch until the
//!   switch (N6.6a5), when `Protocol::ctlinput` passes its callers' proof.
//! - The notify functions (`tcp_notify`, `tcp_mtudisc` and `tcp_mtudisc_increase` in their
//!   notify forms) do nothing for a pcb without a control block (the C's `tcp_notify`
//!   dereferences it: a TCP pcb in the table has one under its socket lock).
//! - `tcp_newtcpcb` returns the new control block opened ([`Tp`]) or `None`, the C's pointer
//!   or NULL; `tcp_close` answers [`Closed`] for the C's NULL; `tcp_freeq` returns `bool`
//!   (the C's 0 or 1).
//! - `tcp_respond`'s `caddr_t template` is a byte slice: the IP header, followed by the TCP
//!   header when `th0` is `None` (the template of a connection); the headers are written in
//!   place into the new mbuf with `hal::mem::pod`. `flags` is the `u8` of `th_flags`. Without a
//!   control block it uses `ip6_defhlim`, what the C's `in6_selecthlim(NULL)` returns
//!   (`in6_selecthlim` takes a pcb here); the caller passes its proof of the net lock.
//! - `tcp_signature` returns `bool` (`true` for the C's 0); its `char *sig` is the 16-byte
//!   digest array; the pseudo header and the TCP header are hashed as the bytes the C's
//!   structures hold (`hal::mem::pod`). `tcp_signature_apply` is the closure handed to `m_apply`.
//!   The five `tcp_signature_tdb_*` functions (the `XF_TCPSIGNATURE` transform, IPsec's
//!   `xformsw` row over `bsd`'s `struct tdb`) stay `bsd`'s until IPsec moves (D-10, N8).
//!
//! ## Redesign
//! - N6.6a2: the kernel's TCP is built beside `bsd`'s, unregistered until N6.6a5 (D-2):
//!   these globals and functions are the kernel's own, `bsd`'s `sys/netinet/tcp_subr.rs`
//!   keeps its until the switch. `tcpqe_pool` and `sackhl_pool` are `BoxPool`s whose items
//!   the queues own (`hal::mem::pool::Pooled`), `tcpcb_pool` is the glue's (`tcp_var::TcpTables`);
//!   `tcp_iss` is an `AtomicU32` beside `tcp_timer_mtx`, a `Mutexed<()>` whose holders alone
//!   write it (since N6.6a5, when pf reads it without the mutex, the C's `READ_ONCE`);
//!   `tcp_secret` is a `OnceStatic` set by `tcp_init` (LZ: a `StaticCell` written under
//!   `unsafe`), and `tcp_secret_ctx`, the context made over it once, is made per ISN
//!   from it (the same cost as the C's copy of the context; no second copy of the secret's
//!   bytes, which `tcp_init` wipes from its stack).
//! - D-12: `tp = tcp_close(tp)` is a move: `tcp_close` consumes the control block's [`Tp`]
//!   and answers [`Closed`], and the frame that owns the socket lock's parts frees the block
//!   with [`tcp_free`] before it unlocks (the C's `inp_ppcb = NULL` and `pool_put`, a few
//!   statements later under the same lock). A use after the close does not compile.
//! - `tcp_template` takes the template out of the control block and gives it back filled
//!   (the C returns the pointer its callers store); `tcp_respond` writes its headers in
//!   place (`hal::mem::pod`, where LZ wrote unaligned under `unsafe`), and lends the pcb's route
//!   cache and IPv6 options to the output call. `tcp_signature`'s key is lent by IPsec's
//!   `tdb_amxkey` leg to the MD5 update, never copied (D-9); the digest is compared with
//!   `timingsafe_bcmp` by its callers.
//! - N6.6a3: `tcp_mtudisc` and `tcp_mtudisc_increase` take the control block held ([`Tp`]),
//!   called so by `tcp_output` (the C passes `tp->t_inpcb` and the callee finds `tp` again
//!   through `intotcpcb`); their notify forms for the pcb walks ([`tcp_mtudisc_notify`],
//!   [`tcp_mtudisc_increase_notify`]) open the block under the socket lock the walk lends,
//!   and do nothing for a pcb without one (the C's `tp == NULL` test). `in_pcbrtentry`
//!   answers a counted route, held where the C holds the cache's pointer.
//! - N6.6a3: `tcp_drop` consumes the control block and answers `tcp_close`'s [`Closed`].
//!   The control inputs (LZ's `unsafe fn`s over raw `sockaddr` and `void *` contracts, 17
//!   `unsafe` in all) read the ICMP quote through `hal::mem::pod` (the IPv4 header, the eight TCP
//!   bytes, `icmp_nextmtu`) and the ICMPv6 parameter by reference; the connection is locked
//!   by `in_pcbsolock` under the caller's proof and opened by `intotcpcb`, and the C's
//!   `notify` function pointer is a closure (the walks') or [`tcp_notify`]'s work on the block
//!   the lookup opened (the per-connection path, whose notify is always `tcp_notify`). The
//!   path MTU branch's socket lock is let go before the walk, as the C's. `tcp_init`
//!   registers `tcp6_mtudisc_callback` with ICMPv6.

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};

use hal::cpu::intr::IPL_SOFTNET;
use hal::mem::pod;
use hal::mem::pool::{BoxPool, PR_NOWAIT, PR_WAITOK, PoolArc};
use hal::sync::mutex::Mutexed;
use hal::sync::netlock::NetHeld;
use hal::sync::once::OnceStatic;
use hal::sync::rwlock::WitnessPart;
use sysdefs::endian::{htonl, htons, ntohl, ntohs};
use sysdefs::errno::Errno;
use sysdefs::mbuf::MT_HEADER;
use sysdefs::net_route::{RTF_HOST, RTV_MTU};
use sysdefs::netinet_in::{INADDR_ANY, IPPROTO_TCP, SockaddrIn};
use sysdefs::netinet_ip::{Ip, Ippseudo};
use sysdefs::netinet_ip_icmp::ICMP_MINLEN;
use sysdefs::netinet_ip_var::Ipovly;
use sysdefs::netinet_ip6::{IPV6_FLOWLABEL_MASK, Ip6Hdr, Ip6HdrPseudo};
use sysdefs::netinet_tcp::{
    TCP_MAX_WINSHIFT, TCP_MAXWIN, TCP_MSS, TCPOLEN_TSTAMP_APPA, TCPOPT_TSTAMP_HDR, TH_ACK, TH_RST,
    TcpSeq, Tcphdr,
};
use sysdefs::netinet6_in6::{SA6_ANY, SockaddrIn6, in6_is_addr_unspecified, in6_is_addr_v4mapped};
use sysdefs::socket::{AF_INET, AF_INET6};

use crate::assert::{kassert, kpanic};
use crate::crypto::md5::{MD5_DIGEST_LENGTH, Md5Ctx};
use crate::crypto::sha2::{SHA512_DIGEST_LENGTH, Sha512Ctx};
use crate::net::ipsec_hooks::{SOCKADDR_UNION_LEN, TdbRef, ipsec};
use crate::net::mbuf::{
    M_DONTWAIT, M_TCP_CSUM_OUT, M_WAIT, MAX_LINKHDR, Mbuf, MbufBox, m_apply, m_copydata, m_get,
    m_gethdr,
};
use crate::net::proto::{
    CtlArg, PRC_HOSTDEAD, PRC_MSGSIZE, PRC_MTUINC, PRC_NCMDS, PRC_QUENCH, SockaddrView,
    prc_is_redirect,
};
use crate::net::rtable::rtable_l2;
use crate::net::socket::{
    InetLocked, SoLocked, so_timeo_wakeup, soisdisconnected, sorwakeup, sowwakeup,
};
use crate::netinet::in_pcb::{
    INP_IPV6, Inpcb, in_pcbdetach, in_pcbinit, in_pcblookup, in_pcbnotifyall, in_pcbrtchange,
    in_pcbrtentry, in_pcbsolock,
};
use crate::netinet::ip_icmp::icmp_mtudisc;
use crate::netinet::ip_input::INETCTLERRMAP;
use crate::netinet::ip_output::ip_output;
use crate::netinet::ip_var::{IP_DEFTTL, IP_MTUDISC, ip_mtudisc};
use crate::netinet::tcp_fsm::{
    TCPS_CLOSED, TCPS_ESTABLISHED, tcps_haveestablished, tcps_havercvdsyn,
};
use crate::netinet::tcp_input::{tcp_hdrsz, tcp_mss};
use crate::netinet::tcp_output::tcp_output;
use crate::netinet::tcp_seq::{seq_geq, seq_lt};
use crate::netinet::tcp_syncache::{syn_cache_cleanup, syn_cache_init, syn_cache_unreach};
use crate::netinet::tcp_timer::TCPTV_SRTTDFLT;
use crate::netinet::tcp_timer::{
    TCPT_NTIMERS, TCPTV_MIN, TCPTV_REXMTMAX, TCPTV_SRTTBASE, tcp_canceltimers, tcp_timer_freesack,
    tcp_timer_init, tcpt_rangeset,
};
use crate::netinet::tcp_var::{
    Sackhole, SynCacheSa, TCP_RTT_BASE_SHIFT, TCP_RTTVAR_SHIFT, TF_NOOPT, TF_PMTUD_PEND,
    TF_RCVD_TSTMP, TF_REQ_SCALE, TF_REQ_TSTMP, TcpState, Tcpcb, Tcpqent, TcpstatCounters, Tp,
    intotcpcb, tcp_rexmtval, tcpstat_inc,
};
use crate::netinet6::icmp6::{icmp6_mtudisc_callback_register, icmp6_mtudisc_update};
use crate::netinet6::in6_pcb::{in6_pcblookup, in6_pcbnotify};
use crate::netinet6::in6_src::{in6_clearscope, in6_selecthlim};
use crate::netinet6::ip6_input::INET6CTLERRMAP;
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
/// `tcp_timer_mtx`: global tcp timer data structures (`tcp_iss`'s writers).
pub static TCP_TIMER_MTX: Mutexed<()> = Mutexed::new(IPL_SOFTNET, ());
/// \[T\] `tcp_iss`: updated by timer and connection under [`TCP_TIMER_MTX`], read without
/// it by pf (`pf_tcp_iss`'s `READ_ONCE`): a word, loaded and stored relaxed by the
/// mutex's holders, so the reader costs a load as the C's.
pub static TCP_ISS: AtomicU32 = AtomicU32::new(0);
/// \[I\] `tcp_starttime`: random offset for `tcp_now()`.
pub static TCP_STARTTIME: AtomicU64 = AtomicU64::new(0);

/// `tcp_init`: TCP initialization (from `domaininit`; a second call, a test glue's, leaves
/// what exists as it is).
pub fn tcp_init<G: ProcGlue>() {
    TCP_ISS.store(1, Ordering::Relaxed); // wrong
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
    // Set once: a second call keeps the first secret, as the first boot's. The stack's copy
    // is wiped (the C draws the bytes into its global); `tcp_set_iss_tsm` hashes the one copy
    // the static holds (the C's `tcp_secret_ctx` is a context over it, made here).
    let _ = TCP_SECRET.set(secret);
    libkern::explicit_bzero(&mut secret);

    // Since sizeof(struct ip6_hdr) > sizeof(struct ip), we do max length checks/computations
    // only on the former.
    let hdrs = (size_of::<Ip6Hdr>() + size_of::<Tcphdr>()) as i32;
    if crate::net::mbuf::MAX_PROTOHDR.load(Ordering::Relaxed) < hdrs {
        crate::net::mbuf::MAX_PROTOHDR.store(hdrs, Ordering::Relaxed);
    }
    if (MAX_LINKHDR.load(Ordering::Relaxed) + hdrs) as usize > crate::net::mbuf::MHLEN {
        kpanic(format_args!("tcp_init"));
    }

    icmp6_mtudisc_callback_register(tcp6_mtudisc_callback::<G>);
    // The softnet's tcp_input_mlist (if.c calls it directly; the glue generic's hop).
    crate::netinet::tcp_input::tcp_input_mlist_register::<G>();

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
#[must_use = "the template goes back into the control block's t_template"]
pub fn tcp_template<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) -> Option<MbufBox> {
    let inp = tp.inp;

    let mut m = if let Some(m) = tp.st.t_template.take() {
        m
    } else {
        let m = m_get(M_DONTWAIT, MT_HEADER)?;
        let nh = match tp.st.pf {
            // default to PF_INET
            0 | AF_INET_I32 => size_of::<Ip>(),
            AF_INET6_I32 => size_of::<Ip6Hdr>(),
            _ => 0,
        };
        m.m_len().set((nh + size_of::<Tcphdr>()) as u32);
        m
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
pub(crate) fn put_hdr<T: sysdefs::pod::AbiPod>(d: &mut [u8], off: usize, v: &T) {
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
    if af == AF_INET6_I32 {
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
    } else {
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

/// A header read out of `template` at `off` (unaligned, as the C's `memcpy`); `panic(9)`
/// when the bytes are shorter (LZ's `read_from`; the C's callers pass whole headers).
fn read_hdr<T: sysdefs::pod::AbiPod>(template: &[u8], off: usize) -> T {
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

/// `tcp_drop`: drop a TCP connection, reporting the specified error (`errno`, `None` for the
/// C's 0). If connection is synchronized, then send a RST to peer. The control block is
/// consumed: closed by `tcp_close`, whose [`Closed`] the caller frees (the C's NULL).
pub fn tcp_drop<G: ProcGlue>(mut tp: Tp<'_, '_, G>, errno: Option<Errno>) -> Closed {
    let so = tp.so;

    if tcps_havercvdsyn(tp.st.t_state) {
        tp.st.t_state = TCPS_CLOSED;
        // The RST is best effort, as the C's (void).
        let _ = tcp_output(&mut tp);
        tcpstat_inc(TcpstatCounters::TcpsDrops);
    } else {
        tcpstat_inc(TcpstatCounters::TcpsConndrops);
    }
    let mut errno = errno;
    if errno == Some(Errno::ETIMEDOUT) && tp.st.t_softerror.is_some() {
        errno = tp.st.t_softerror;
    }
    so.set_error(errno);
    tcp_close(tp)
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
/// dropped with it (D-11). The socket's locked slot, which the requests read
/// (`sotoinpcb_locked`), is emptied with it (N6.6a5): `in_pcbdetach` emptied the counted one,
/// and the pcb and its socket count each other until both are. A request has taken it out
/// already (`tcp_usrreq`'s `tcp_free_request`).
pub fn tcp_free<G: ProcGlue>(inp: &Inpcb<G>, pcb: &mut WitnessPart<'_>, Closed(()): Closed) {
    drop(inp.take_tcpcb(pcb));
    if let Some(slot) = inp.socket().set_pcb_locked(pcb, None) {
        kassert!(
            slot.downcast_ref::<Inpcb<G>>()
                .is_some_and(|s| core::ptr::eq(s, inp))
        );
        drop(slot);
    }
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

/// `tcp_notify`: notify a tcp user of an asynchronous error; store error as soft error, but
/// wake up user (for now, won't do anything until can select for soft error). The notify
/// form of the pcb walks: the control block of `inp`, whose socket `w` locks (the C's
/// `soassertlocked`), if it has one.
pub fn tcp_notify<G: ProcGlue>(inp: &Inpcb<G>, w: &mut SoLocked<'_>, error: Option<Errno>) {
    with_tcpcb(inp, w, |tp| tcp_notify_tp(tp, error));
}

/// [`tcp_notify`]'s work on the control block held (the control inputs' own lookups hold it
/// already: their notify is always `tcp_notify`).
fn tcp_notify_tp<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, error: Option<Errno>) {
    let so = tp.so;
    let st = &mut *tp.st;

    // Ignore some errors if we are hooked up. If connection hasn't completed, has
    // retransmitted several times, and receives a second error, give up now. This is better
    // than waiting a long time to establish a connection that can never complete.
    if st.t_state == TCPS_ESTABLISHED
        && matches!(
            error,
            Some(Errno::EHOSTUNREACH | Errno::ENETUNREACH | Errno::EHOSTDOWN)
        )
    {
        return;
    } else if !tcps_haveestablished(st.t_state) && st.t_rxtshift > 3 && st.t_softerror.is_some() {
        so.set_error(error);
    } else {
        st.t_softerror = error;
    }
    // wakeup((caddr_t) &so->so_timeo)
    so_timeo_wakeup(so);
    sorwakeup(so);
    sowwakeup(so);
}

/// The returned TCP header's first eight bytes (ports and sequence number, network order)
/// as a header whose other members are zero (the C reads only those through its pointer).
fn th_of8(b: [u8; 8]) -> Tcphdr {
    Tcphdr {
        th_sport: u16::from_ne_bytes([b[0], b[1]]),
        th_dport: u16::from_ne_bytes([b[2], b[3]]),
        th_seq: u32::from_ne_bytes([b[4], b[5], b[6], b[7]]),
        ..Tcphdr::default()
    }
}

/// A socket address's bytes as `union syn_cache_sa`'s (zero past what the view holds).
fn sa_union(sa: &[u8]) -> SynCacheSa {
    let mut u = [0u8; SOCKADDR_UNION_LEN];
    let n = sa.len().min(u.len());
    u[..n].copy_from_slice(&sa[..n]);
    u
}

/// Whether a control input's error is one an embryonic connection is told of
/// (`syn_cache_unreach`).
fn unreach_errno(e: Option<Errno>) -> bool {
    matches!(
        e,
        Some(Errno::EHOSTUNREACH | Errno::ENETUNREACH | Errno::EHOSTDOWN)
    )
}

/// `tcp6_ctlinput`: an ICMPv6 error about a segment we sent (`INET6`; compiled always, as
/// `netinet6` is): `arg` the error's parameter (`struct ip6ctlparam`), or none; the caller's
/// proof of the net lock `net` (the C's callers hold it).
pub fn tcp6_ctlinput<G: ProcGlue>(
    cmd: i32,
    sa: SockaddrView<'_>,
    rdomain: u32,
    arg: CtlArg<'_>,
    net: &NetHeld<'_>,
) {
    let mut rtchange = false;

    if sa.family() != AF_INET6 || usize::from(sa.len()) != size_of::<SockaddrIn6>() {
        return;
    }
    let Some(sa6) = pod::at::<SockaddrIn6>(sa.bytes(), 0) else {
        return;
    };
    if in6_is_addr_unspecified(&sa6.sin6_addr) || in6_is_addr_v4mapped(&sa6.sin6_addr) {
        return;
    }
    // The argument of this family only (an IPv4 quote is never handed to an IPv6 entry).
    let mut d = match arg {
        CtlArg::Ip6(p) => Some(p),
        CtlArg::None => None,
        CtlArg::Ip(_) => return,
    };
    if cmd as u32 as usize >= PRC_NCMDS {
        return;
    } else if cmd == PRC_QUENCH {
        // Don't honor ICMP Source Quench messages meant for TCP connections.
        // XXX there's no PRC_QUENCH in IPv6
        return;
    } else if prc_is_redirect(cmd) {
        rtchange = true;
        d = None;
    } else if cmd == PRC_MSGSIZE {
        // special code is present, see below
    } else if cmd == PRC_HOSTDEAD {
        d = None;
    } else if INET6CTLERRMAP[cmd as usize].is_none() {
        return;
    }
    let errno = INET6CTLERRMAP[cmd as usize];

    // if the parameter is from icmp6, decode it.
    let sa6_src = d.map_or(SA6_ANY, |p| p.src);

    let Some(p) = d else {
        // in_rtchange for a redirect, tcp_notify otherwise.
        let mut notify: NotifyFn<G> = if rtchange {
            rtchange_notify::<G>
        } else {
            tcp_notify::<G>
        };
        in6_pcbnotify(
            &crate::net::domain::net::<G>().tcp.tcb6table,
            &sa6,
            0,
            Some(&sa6_src),
            0,
            rdomain,
            cmd,
            net,
            Some(&mut notify),
        );
        return;
    };

    // XXX: We assume that when ip6 is non NULL, M and OFF are valid (the parameter carries
    // the quoted header with them).
    let (m, off) = (p.m, p.off);

    // check if we can safely examine src and dst ports (the C's `int` against the `size_t`
    // sum, unsigned)
    if (m.m_pkthdr().len.get() as u64) < (off as u64).wrapping_add(8) {
        return;
    }

    let mut b = [0u8; 8];
    m_copydata(m, off, &mut b);
    let th = th_of8(b);

    // Check to see if we have a valid TCP connection corresponding to the address in the
    // ICMPv6 message payload.
    let inp = in6_pcblookup(
        &crate::net::domain::net::<G>().tcp.tcb6table,
        &sa6.sin6_addr,
        th.th_dport,
        &sa6_src.sin6_addr,
        th.th_sport,
        rdomain,
    );
    if cmd == PRC_MSGSIZE {
        // Depending on the value of "valid" and routing table size (mtudisc_{hi,lo}wat), we
        // will:
        // - recalculate the new MTU and create the corresponding routing entry, or
        // - ignore the MTU change notification.
        icmp6_mtudisc_update(p, inp.is_some(), net);
        drop(inp); // in_pcbunref
        return;
    }
    let mut had_tp = false;
    if let Some(i) = inp.as_deref()
        && let Some(mut w) = in_pcbsolock(i, net)
        && let Some(mut il) = w.inet()
        && let Some(mut tp) = intotcpcb(i, &mut il)
    {
        had_tp = true;
        let seq = ntohl(th.th_seq);
        if seq_geq(seq, tp.st.snd_una) && seq_lt(seq, tp.st.snd_max) {
            tcp_notify_tp(&mut tp, errno);
        }
    }
    // in_pcbsounlock (the holding's drop above), in_pcbunref
    drop(inp);

    if !had_tp && unreach_errno(errno) {
        let mut src = [0u8; SOCKADDR_UNION_LEN];
        put_hdr(&mut src, 0, &sa6_src);
        syn_cache_unreach::<G>(&src, &sa_union(sa.bytes()), &th, rdomain, net);
    }
}

/// What `tcp_ctlinput` reads of an ICMP message that returned an IPv4 header: the header,
/// the TCP header's first eight bytes after it, and the message's `icmp_nextmtu` (network
/// order).
struct Quote {
    /// The returned IP header (`&icp->icmp_ip`).
    ip: Ip,
    /// The returned TCP header's ports and sequence number.
    th: Tcphdr,
    /// `icp->icmp_nextmtu`.
    nextmtu: u16,
}

/// The [`Quote`] of `msg`, the ICMP message from its type byte (`CtlArg::Ip`); `None` when
/// the message is shorter than the header it returns and the TCP bytes the C reads after it,
/// or returns a header shorter than 20 bytes (`icmp_input` hands no such message on: the C
/// would read past the message; here nothing is done).
fn tcp_quote(msg: &[u8]) -> Option<Quote> {
    let ip: Ip = pod::at(msg, ICMP_MINLEN)?;
    let hl = usize::from(ip.ip_hl()) << 2;
    if hl < size_of::<Ip>() {
        return None;
    }
    let at = ICMP_MINLEN + hl;
    let t: [u8; 8] = msg.get(at..at + 8)?.try_into().ok()?;
    // icmp_nextmtu: the second half of icmp_hun's ih_pmtu, bytes 6 and 7.
    let nextmtu = u16::from_ne_bytes([msg[6], msg[7]]);
    Some(Quote {
        ip,
        th: th_of8(t),
        nextmtu,
    })
}

/// The notify function the control inputs hand their walks (the C's `void (*notify)(struct
/// inpcb *, int)`): the connection, its socket held in parts, the error.
type NotifyFn<G> = fn(&Inpcb<G>, &mut SoLocked<'_>, Option<Errno>);

/// `in_rtchange` as a [`NotifyFn`]: the cached route let go (the socket's holding unused).
fn rtchange_notify<G: ProcGlue>(inp: &Inpcb<G>, _w: &mut SoLocked<'_>, errno: Option<Errno>) {
    in_pcbrtchange(inp, errno);
}

/// `tcp_ctlinput`: an ICMP error about a segment we sent: `arg` the ICMP message that
/// returned the segment's IP header (from its type byte), or none; the caller's proof of the
/// net lock `net` (the C's callers hold it).
pub fn tcp_ctlinput<G: ProcGlue>(
    cmd: i32,
    sa: SockaddrView<'_>,
    rdomain: u32,
    arg: CtlArg<'_>,
    net: &NetHeld<'_>,
) {
    let mut notify: NotifyFn<G> = tcp_notify::<G>;

    if sa.family() != AF_INET {
        return;
    }
    // satosin(sa): the address's bytes as a sockaddr_in (zero past what the view holds).
    let mut sinb = [0u8; size_of::<SockaddrIn>()];
    let n = sa.bytes().len().min(sinb.len());
    sinb[..n].copy_from_slice(&sa.bytes()[..n]);
    let Some(sin) = pod::at::<SockaddrIn>(&sinb, 0) else {
        return;
    };
    let faddr = sin.sin_addr;
    if faddr.s_addr == INADDR_ANY {
        return;
    }

    // The argument of this family only (an IPv6 parameter is never handed to an IPv4
    // entry); its quote is read where the C reads it.
    let mut ip = match arg {
        CtlArg::Ip(msg) => Some(msg),
        CtlArg::None => None,
        CtlArg::Ip6(_) => return,
    };

    if cmd as u32 as usize >= PRC_NCMDS {
        return;
    }
    let errno = INETCTLERRMAP[cmd as usize];
    if cmd == PRC_QUENCH {
        // Don't honor ICMP Source Quench messages meant for TCP connections.
        return;
    } else if prc_is_redirect(cmd) {
        notify = rtchange_notify::<G>;
        ip = None;
    } else if cmd == PRC_MSGSIZE
        && ip_mtudisc.load(Ordering::Relaxed) != 0
        && let Some(msg) = ip
    {
        // The quote `icmp_input` checked holds the header and the TCP bytes the C reads.
        let Some(q) = tcp_quote(msg) else {
            return;
        };
        // Verify that the packet in the icmp payload refers to an existing TCP connection.
        let seq = ntohl(q.th.th_seq);
        let inp = in_pcblookup(
            &crate::net::domain::net::<G>().tcp.tcbtable,
            q.ip.ip_dst,
            q.th.th_dport,
            q.ip.ip_src,
            q.th.th_sport,
            rdomain,
        );
        // `Some(true)`: go on to the walk with tcp_mtudisc; else return (no matching
        // connection, a Next-Hop MTU too large, or a notice recorded for later).
        let go_on = inp.as_deref().and_then(|i| {
            let mut w = in_pcbsolock(i, net)?;
            let mut il = w.inet()?;
            let tp = intotcpcb(i, &mut il)?;
            let st = tp.st;
            if !(seq_geq(seq, st.snd_una) && seq_lt(seq, st.snd_max)) {
                // ignore if we don't have a matching connection
                return None;
            }
            // If the ICMP message advertises a Next-Hop MTU equal or larger than the maximum
            // packet size we have ever sent, drop the message.
            let mtu = u32::from(ntohs(q.nextmtu));
            if mtu >= st.t_pmtud_mtu_sent {
                return Some(false);
            }
            if mtu >= tcp_hdrsz(st).wrapping_add(st.t_pmtud_mss_acked) {
                // Calculate new MTU, and create corresponding route (traditional PMTUD).
                st.clear_flags(TF_PMTUD_PEND);
                icmp_mtudisc(q.nextmtu, &q.ip, i.inp_rtableid.get(), net);
                Some(true)
            } else {
                // Record the information got in the ICMP message; act on it later. If we had
                // already recorded an ICMP message, replace the old one only if the new
                // message refers to an older TCP segment
                if st.has_flags(TF_PMTUD_PEND) {
                    if seq_lt(st.t_pmtud_th_seq, seq) {
                        return Some(false);
                    }
                } else {
                    st.set_flags(TF_PMTUD_PEND);
                }
                st.t_pmtud_th_seq = seq;
                st.t_pmtud_nextmtu = u32::from(q.nextmtu);
                st.t_pmtud_ip_len = q.ip.ip_len;
                st.t_pmtud_ip_hl = u16::from(q.ip.ip_hl());
                Some(false)
            }
        });
        // in_pcbsounlock (the holding's drop in the closure), in_pcbunref
        drop(inp);
        if go_on != Some(true) {
            return;
        }
        notify = tcp_mtudisc_notify::<G>;
        ip = None;
    } else if cmd == PRC_MTUINC {
        notify = tcp_mtudisc_increase_notify::<G>;
        ip = None;
    } else if cmd == PRC_HOSTDEAD {
        ip = None;
    } else if errno.is_none() {
        return;
    }

    let Some(msg) = ip else {
        in_pcbnotifyall(
            &crate::net::domain::net::<G>().tcp.tcbtable,
            &sin,
            rdomain,
            errno,
            net,
            notify,
        );
        return;
    };

    // The returned segment's own connection: its notify is tcp_notify (every other one
    // cleared `ip` above).
    let Some(q) = tcp_quote(msg) else {
        return;
    };
    let inp = in_pcblookup(
        &crate::net::domain::net::<G>().tcp.tcbtable,
        q.ip.ip_dst,
        q.th.th_dport,
        q.ip.ip_src,
        q.th.th_sport,
        rdomain,
    );
    let mut had_tp = false;
    if let Some(i) = inp.as_deref()
        && let Some(mut w) = in_pcbsolock(i, net)
        && let Some(mut il) = w.inet()
        && let Some(mut tp) = intotcpcb(i, &mut il)
    {
        had_tp = true;
        let seq = ntohl(q.th.th_seq);
        if seq_geq(seq, tp.st.snd_una) && seq_lt(seq, tp.st.snd_max) {
            tcp_notify_tp(&mut tp, errno);
        }
    }
    // in_pcbsounlock (the holding's drop above), in_pcbunref
    drop(inp);

    if !had_tp && unreach_errno(INETCTLERRMAP[cmd as usize]) {
        let src = SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_port: q.th.th_sport,
            sin_addr: q.ip.ip_src,
            ..SockaddrIn::default()
        };
        let mut srcu = [0u8; SOCKADDR_UNION_LEN];
        put_hdr(&mut srcu, 0, &src);
        syn_cache_unreach::<G>(&srcu, &sa_union(sa.bytes()), &q.th, rdomain, net);
    }
}

/// `tcp6_mtudisc_callback`: path MTU discovery handler of `icmp6_mtudisc_update` (`INET6`),
/// registered by `tcp_init`: every connection to `sin6` takes the new MTU (`tcp_mtudisc`).
pub fn tcp6_mtudisc_callback<G: ProcGlue>(sin6: &SockaddrIn6, rdomain: u32, net: &NetHeld<'_>) {
    let mut f = |inp: &Inpcb<G>, w: &mut SoLocked<'_>, e: Option<Errno>| {
        tcp_mtudisc_notify(inp, w, e);
    };
    in6_pcbnotify(
        &crate::net::domain::net::<G>().tcp.tcb6table,
        sin6,
        0,
        Some(&SA6_ANY),
        0,
        rdomain,
        PRC_MSGSIZE,
        net,
        Some(&mut f),
    );
}

/// `tcp_mtudisc`: on receipt of path MTU corrections, flush old route and replace it with the
/// new one. Retransmit all unacknowledged packets, to ensure that all packets will be
/// received. `errno` is the notification's (`None` for `tcp_output`'s -1: no error to
/// report); the C's `inpcb` argument is the control block's, held (the notify form is
/// [`tcp_mtudisc_notify`]).
pub fn tcp_mtudisc<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, errno: Option<Errno>) {
    let inp = tp.inp;
    let mut change = false;

    let orig_maxseg = tp.st.t_maxseg;

    if let Some(mut rt) = in_pcbrtentry(inp, tp.net) {
        let orig_mtulock = rt.rt_locks().get() & RTV_MTU;

        // If this was not a host route, remove and realloc.
        if !rt.rt_flags.isset(RTF_HOST) {
            in_pcbrtchange(inp, errno);
            match in_pcbrtentry(inp, tp.net) {
                Some(r) => rt = r,
                None => return,
            }
        }
        if orig_mtulock < rt.rt_locks().get() & RTV_MTU {
            change = true;
        }
    }
    tcp_mss(tp, -1);
    if orig_maxseg > tp.st.t_maxseg {
        change = true;
    }

    // Resend unacknowledged packets
    tp.st.snd_nxt = tp.st.snd_una;
    if change || errno.is_some_and(|e| e.as_i32() > 0) {
        // The C ignores the result: the segments go again at the next chance.
        let _ = tcp_output(tp);
    }
}

/// `tcp_mtudisc_increase`: a path MTU estimate grew (`PRC_MTUINC`): a host route is dropped
/// for a new one, and the segment size taken again (`tcp_mss(tp, -1)`, which also sets the
/// congestion window). The notify form is [`tcp_mtudisc_increase_notify`].
pub fn tcp_mtudisc_increase<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, errno: Option<Errno>) {
    let Some(rt) = in_pcbrtentry(tp.inp, tp.net) else {
        return;
    };
    // If this was a host route, remove and realloc.
    if rt.rt_flags.isset(RTF_HOST) {
        in_pcbrtchange(tp.inp, errno);
    }
    drop(rt);

    // also takes care of congestion window
    tcp_mss(tp, -1);
}

/// `tcp_mtudisc` as a notify function (`in_pcbnotifyall`'s, `in6_pcbnotify`'s): the control
/// block of `inp`, whose socket `w` locks, if it has one (`intotcpcb`).
pub fn tcp_mtudisc_notify<G: ProcGlue>(inp: &Inpcb<G>, w: &mut SoLocked<'_>, errno: Option<Errno>) {
    with_tcpcb(inp, w, |tp| tcp_mtudisc(tp, errno));
}

/// `tcp_mtudisc_increase` as a notify function, as [`tcp_mtudisc_notify`].
pub fn tcp_mtudisc_increase_notify<G: ProcGlue>(
    inp: &Inpcb<G>,
    w: &mut SoLocked<'_>,
    errno: Option<Errno>,
) {
    with_tcpcb(inp, w, |tp| tcp_mtudisc_increase(tp, errno));
}

/// The control block of `inp` opened under its socket lock `w` for a notify function (the
/// C's `intotcpcb(inp)` at their start); nothing without one (a socket a close detached, or
/// one held C-style, which no kernel protocol's is).
fn with_tcpcb<G: ProcGlue>(
    inp: &Inpcb<G>,
    w: &mut SoLocked<'_>,
    f: impl FnOnce(&mut Tp<'_, '_, G>),
) {
    let Some(mut il) = w.inet() else {
        return;
    };
    if let Some(mut tp) = intotcpcb(inp, &mut il) {
        f(&mut tp);
    }
}

/// `tcp_set_iss_tsm`: generate new ISNs with a method based on RFC1948.
pub fn tcp_set_iss_tsm<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) {
    let inp = tp.inp;
    let rdomain = rtable_l2(inp.inp_rtableid.get());

    let iss = {
        let g = TCP_TIMER_MTX.lock();
        let iss = TCP_ISS
            .load(Ordering::Relaxed)
            .wrapping_add(TCP_ISS_CONN_INC);
        TCP_ISS.store(iss, Ordering::Relaxed);
        drop(g);
        iss
    };

    // The context is not `Copy`: the clone is the C's struct copy, wiped at its drop.
    // tcp_secret_ctx's copy: a context over the secret's one copy, wiped when it is finalized
    // (the C copies a context made at init; the cost is the same, the secret's bytes stay
    // in one place).
    let Some(secret) = TCP_SECRET.get() else {
        kpanic(format_args!(
            "tcp_set_iss_tsm: no ISN secret before tcp_init"
        ));
    };
    let mut ctx = Sha512Ctx::new();
    ctx.update(secret);
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

/// The host tests' TCP globals reset (testing.md's rule for a global that holds kernel memory
/// or pool items): the SYN cache's sets, which hold an earlier test's bucket arrays and
/// possibly its entries, forgotten without being freed (`syn_cache_init` makes them again in
/// this test's memory), the debug ring zeroed (under `RING_TEST`) and `tcp_input`'s two rate
/// limiters started again (`tcp_input_test_reset`), and the SYN cache's two tunables
/// (`tcp_syn_hash_size`, `tcp_syn_use_limit`, which a test may change through `tcp_sysctl`)
/// given back their initial values. The pcb tables are reset by `in_pcbinit` itself; the
/// counters (compared by difference, and bumped by other modules' tests
/// meanwhile) and the ISN secret (set once) hold no memory and stay. `bsd`'s host tests run
/// the kernel's TCP over their glue since N6.6a5 and call it from their network setup.
#[cfg(any(test, feature = "doubles"))]
pub fn tcp_test_reset<G: ProcGlue>() {
    let tcp = &crate::net::domain::net::<G>().tcp;
    let mut g = tcp.syn_cache.lock();
    core::mem::forget(core::mem::replace(
        &mut *g,
        crate::netinet::tcp_var::SynCaches::new(),
    ));
    // `TcpTables::new`'s values, the C's initializers (`tcp_input.c`).
    tcp.tcp_syn_hash_size.store(
        crate::netinet::tcp_var::TCP_SYN_HASH_SIZE,
        Ordering::Relaxed,
    );
    tcp.tcp_syn_use_limit.store(100_000, Ordering::Relaxed);
    drop(g);
    let _ring = crate::netinet::tcp_debug::RING_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut r = crate::netinet::tcp_debug::TCP_DEBUG.lock();
    r.tcp_debug = [const { crate::netinet::tcp_debug::TcpDebug::zeroed() };
        crate::netinet::tcp_debug::TCP_NDEBUG];
    r.tcp_debx = 0;
    drop(r);
    crate::netinet::tcp_input::tcp_input_test_reset();
}

/// The host tests' TCP connections: control blocks of the kernel's TCP over the Internet
/// socket fixture (`in_pcb::testinet`), its socket lock taken as a callout takes it.
#[cfg(test)]
pub(crate) mod testtcp {
    use std::sync::MutexGuard;

    use hal::sync::netlock::{NETLOCK, NetHeld};

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
        tcp_test_reset::<G>();
        tcp_init::<G>();
        r
    }

    /// A socket with a pcb in `tcbtable` and a control block (`tcp_attach`'s two steps).
    pub(crate) fn conn() -> (SocketRef<G>, InpcbRef<G>) {
        let (so, inp) = testinet::pcb(&crate::net::domain::net::<G>().tcp.tcbtable);
        with_lock(&inp, |inp, il| {
            assert!(tcp_newtcpcb(inp, il, M_WAIT).is_some(), "a control block");
        });
        attach_slot(&inp);
        (so, inp)
    }

    /// What `tcp_attach` does once the control block is made: the pcb in its socket's
    /// locked slot, which the user requests read (`sotoinpcb_locked`).
    pub(crate) fn attach_slot(inp: &InpcbRef<G>) {
        let h = PoolArc::erase(inp.clone());
        with_lock(inp, |i, il| {
            assert!(i.socket().set_pcb_locked(&mut il.pcb, Some(h)).is_none());
        });
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

    /// The driver of a test interface: its packets' count and the last one's bytes.
    pub(crate) type Wire = testinet::InetDriver;

    /// The host tests' TCP protocols over the test domains (`testinet`'s `InetDom` and
    /// `Inet6Dom`), whose requests are the kernel's (`tcp_usrreq`'s [`TcpUsrreqs`]): a
    /// listener of these gives `sonewconn` the real `tcp_attach` (the pcb in the family's
    /// table, the control block, the `ENOBUFS` path that detaches the pcb again when
    /// "tcpcb" has no item: `testutil::fail_pool`).
    ///
    /// [`TcpUsrreqs`]: crate::netinet::tcp_usrreq::TcpUsrreqs
    pub(crate) struct TcpProto;

    /// [`TcpProto`] over IPv6.
    pub(crate) struct Tcp6Proto;

    impl crate::net::proto::Protocol<G> for TcpProto {
        fn domain(&self) -> &'static dyn crate::net::domain::Domain<G> {
            &testinet::InetDom
        }
        fn ty(&self) -> i16 {
            sysdefs::socket::SOCK_STREAM as i16
        }
        fn protocol(&self) -> i16 {
            IPPROTO_TCP as i16
        }
        fn flags(&self) -> i16 {
            crate::net::proto::PR_CONNREQUIRED | crate::net::proto::PR_WANTRCVD
        }
        fn usrreqs(&self) -> Option<&'static dyn crate::net::proto::Usrreqs<G>> {
            Some(&crate::netinet::tcp_usrreq::TcpUsrreqs::<G>::INET)
        }
    }

    impl crate::net::proto::Protocol<G> for Tcp6Proto {
        fn domain(&self) -> &'static dyn crate::net::domain::Domain<G> {
            &testinet::Inet6Dom
        }
        fn ty(&self) -> i16 {
            sysdefs::socket::SOCK_STREAM as i16
        }
        fn protocol(&self) -> i16 {
            IPPROTO_TCP as i16
        }
        fn flags(&self) -> i16 {
            crate::net::proto::PR_CONNREQUIRED | crate::net::proto::PR_WANTRCVD
        }
        fn usrreqs(&self) -> Option<&'static dyn crate::net::proto::Usrreqs<G>> {
            Some(&crate::netinet::tcp_usrreq::TcpUsrreqs::<G>::INET6)
        }
    }

    /// A socket of `proto` with `tcp_attach` done (its buffers 16 KB each, its pcb and
    /// control block made), unbound.
    pub(crate) fn socket(proto: &'static dyn crate::net::proto::Protocol<G>) -> SocketRef<G> {
        let so = crate::net::socket::soalloc(
            proto,
            M_WAIT,
            crate::net::socket::SoInit {
                so_type: sysdefs::socket::SOCK_STREAM,
                state: 0,
                options: 0,
                linger: 0,
                timeo: 0,
                euid: 0,
                ruid: 0,
                egid: 0,
                rgid: 0,
                cpid: 1,
            },
        )
        .expect("a socket");
        crate::net::socket::soreserve(&so, 16384, 16384).expect("the buffers");
        {
            let mut w = crate::net::socket::solock(&so);
            proto
                .usrreqs()
                .expect("TCP's requests")
                .attach(&so, &mut w, 0, M_WAIT)
                .expect("the control block");
        }
        so
    }
    /// A listener of [`TcpProto`] on 10.9.`n`.1:`port` over a new test interface `name`
    /// (`socket`, `bind`, `listen(5)`): its socket and pcb, the interface's driver. Its
    /// buffers 16 KB each, as `estab`'s.
    pub(crate) fn listener(
        name: &[u8],
        n: u8,
        port: u16,
    ) -> (SocketRef<G>, InpcbRef<G>, &'static Wire) {
        let (_ifp, d) = testinet::iface(name, [10, 9, n, 1]);
        let (so, inp) = listen_on(n, port);
        (so, inp, d)
    }

    /// [`listener`] on an interface made already (10.9.`n`.1's).
    pub(crate) fn listen_on(n: u8, port: u16) -> (SocketRef<G>, InpcbRef<G>) {
        listen_at([10, 9, n, 1], port)
    }

    /// A listener of [`TcpProto`] bound to `addr`:`port` (`[0; 4]`: the wildcard).
    pub(crate) fn listen_at(addr: [u8; 4], port: u16) -> (SocketRef<G>, InpcbRef<G>) {
        use sysdefs::netinet_in::InAddr;

        listen_proto(
            &TcpProto,
            &crate::net::domain::net::<G>().tcp.tcbtable,
            |g, inp| {
                inp.set_laddr(
                    g,
                    InAddr {
                        s_addr: u32::from_ne_bytes(addr),
                    },
                );
            },
            port,
        )
    }

    /// A listener of [`Tcp6Proto`] bound to [`addr`]:`port`.
    pub(crate) fn listen6_at(
        addr: sysdefs::netinet6_in6::In6Addr,
        port: u16,
    ) -> (SocketRef<G>, InpcbRef<G>) {
        listen_proto(
            &Tcp6Proto,
            &crate::net::domain::net::<G>().tcp.tcb6table,
            |g, inp| {
                inp.set_laddr6(g, addr);
            },
            port,
        )
    }

    /// A listener of `proto` in `table`, its local address set by `bind`, its port `port`.
    fn listen_proto(
        proto: &'static dyn crate::net::proto::Protocol<G>,
        table: &'static crate::netinet::in_pcb::Inpcbtable<G>,
        bind: impl FnOnce(&crate::netinet::in_pcb::InpGuard<'_, G>, &Inpcb<G>),
        port: u16,
    ) -> (SocketRef<G>, InpcbRef<G>) {
        let so = socket(proto);
        let inp = crate::netinet::in_pcb::sotoinpcb(&so).expect("a pcb");
        {
            let mut g = table.lock();
            bind(&g, &inp);
            inp.set_lport(&g, htons(port));
            crate::netinet::in_pcb::in_pcbrehash(&mut g, &inp);
        }
        {
            let mut w = crate::net::socket::solock(&so);
            crate::net::socket::solisten(&so, &mut w, 5).expect("listen");
        }
        (so, inp)
    }

    /// The listener `so` closed (`tcp_detach`): its control block (and its SYN cache
    /// entries) closed and freed, the socket let go (no file holds it; nothing is left on its
    /// queues).
    pub(crate) fn close_listener(so: SocketRef<G>, inp: InpcbRef<G>) {
        so.st_set(SS_NOFDREF);
        {
            let mut w = crate::net::socket::solock(&so);
            crate::netinet::tcp_usrreq::tcp_detach(&so, &mut w).expect("a control block");
        }
        drop(inp);
        drop(so);
    }

    /// An established connection from 10.9.`n`.1:7000 to 10.9.`n`.2:80 over a new test
    /// interface `name` (MTU 1500): the socket's buffers 16 KB each, no options requested,
    /// the segment size taken from the route (`tcp_mss`: 1460, the window 14600), the template
    /// made, our sequence space at 1000 and the peer's at 5000, its window 65535 and ours
    /// (16384) advertised.
    pub(crate) fn estab(name: &[u8], n: u8) -> (SocketRef<G>, InpcbRef<G>, &'static Wire) {
        estab_to(name, n, [10, 9, n, 2])
    }

    /// [`estab`] to the peer `peer`; one off the interface's net goes through a gateway,
    /// 10.9.`n`.254 (`route add 10.20.n/24 10.9.n.254`, made here for `peer` in 10.20/16).
    pub(crate) fn estab_to(
        name: &[u8],
        n: u8,
        peer: [u8; 4],
    ) -> (SocketRef<G>, InpcbRef<G>, &'static Wire) {
        use sysdefs::netinet_in::InAddr;

        let (_ifp, d) = testinet::iface(name, [10, 9, n, 1]);
        if peer[..2] == [10, 20] {
            gateway_route(n, peer[2]);
        }
        let (so, inp) = conn();
        {
            let t = &crate::net::domain::net::<G>().tcp.tcbtable;
            let mut g = t.lock();
            inp.set_laddr(
                &g,
                InAddr {
                    s_addr: u32::from_ne_bytes([10, 9, n, 1]),
                },
            );
            inp.set_faddr(
                &g,
                InAddr {
                    s_addr: u32::from_ne_bytes(peer),
                },
            );
            inp.set_lport(&g, htons(7000));
            inp.set_fport(&g, htons(80));
            crate::netinet::in_pcb::in_pcbrehash(&mut g, &inp);
        }
        crate::net::socket::soreserve(&so, 16384, 16384).expect("the buffers");
        with_tp(&inp, |mut tp| {
            tp.st
                .clear_flags(crate::netinet::tcp_var::TF_REQ_SCALE | TF_REQ_TSTMP);
            let m = tcp_template(&mut tp).expect("a template");
            tp.st.t_template = Some(m);
            let _ = crate::netinet::tcp_input::tcp_mss(&mut tp, 0);
            let st = &mut *tp.st;
            st.t_state = crate::netinet::tcp_fsm::TCPS_ESTABLISHED;
            st.iss = 1000;
            st.snd_una = 1000;
            st.snd_nxt = 1000;
            st.snd_max = 1000;
            st.snd_up = 1000;
            st.irs = 4999;
            st.rcv_nxt = 5000;
            st.rcv_up = 5000;
            st.rcv_adv = 5000 + 16384;
            st.snd_wnd = 65535;
            st.max_sndwnd = 65535;
            st.t_rcvtime = crate::netinet::tcp_var::tcp_now();
            ((), None)
        });
        (so, inp, d)
    }

    /// `route add 10.20.p.0/24 10.9.n.254` (through the interface of 10.9.`n`.1).
    fn gateway_route(n: u8, p: u8) {
        use sysdefs::net_route::{
            RTAX_DST, RTAX_GATEWAY, RTAX_NETMASK, RTF_GATEWAY, RTF_STATIC, RTF_UP, RTM_ADD,
            RTP_STATIC,
        };

        use crate::net::proto::SockaddrView;
        use crate::net::route::{RtAddrinfo, rt_plentosa, rtrequest};

        let sin = |a: [u8; 4]| {
            let mut b = [0u8; 16];
            b[0] = 16;
            b[1] = AF_INET;
            b[4..8].copy_from_slice(&a);
            b
        };
        let (dst, gw, us) = (
            sin([10, 20, p, 0]),
            sin([10, 9, n, 254]),
            sin([10, 9, n, 1]),
        );
        let mut mask = crate::net::rtable::SaCopy::zeroed();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_DST] = SockaddrView::new(&dst);
        info.rti_info[RTAX_NETMASK] = rt_plentosa(AF_INET, Some(24), &mut mask);
        info.rti_info[RTAX_GATEWAY] = SockaddrView::new(&gw);
        let r = net.reader();
        info.rti_ifa = crate::net::if_::ifa_ifwithaddr(&net, &r, &us, 0);
        info.rti_flags = RTF_GATEWAY | RTF_STATIC | RTF_UP;
        rtrequest(RTM_ADD, &mut info, RTP_STATIC, None, 0, &net).expect("the route");
    }

    /// 2001:db8:`n`::`h`.
    pub(crate) fn a6(n: u8, h: u8) -> sysdefs::netinet6_in6::In6Addr {
        let mut a = sysdefs::netinet6_in6::In6Addr::default();
        a.s6_addr[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
        a.s6_addr[5] = n;
        a.s6_addr[15] = h;
        a
    }

    /// A new test interface `name` (MTU 1500, not Ethernet, so no neighbor to resolve) with
    /// 2001:db8:`n`::1/64 and its routes: its wire.
    pub(crate) fn iface6(name: &[u8], n: u8) -> &'static Wire {
        use sysdefs::net_if::{IFF_MULTICAST, IFF_RUNNING, IFF_UP};

        let d: &'static Wire = std::boxed::Box::leak(std::boxed::Box::default());
        let ifp = crate::net::if_::test_ifnet_with(name, d);
        // Not running when the address comes: no DAD (in6if_do_dad), so no neighbor
        // solicitation of a timer reaches this wire during a test, and the address is not
        // tentative.
        ifp.if_flags.set(IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        crate::net::if_::if_alloc_sadl(ifp);
        {
            let _w = NETLOCK.write();
            crate::net::if_::IFNETLIST.insert_tail(ifp);
        }
        crate::netinet6::in6::tests::add(ifp, a6(n, 1), 64).expect("SIOCAIFADDR_IN6");
        ifp.if_flags.set(IFF_UP | IFF_RUNNING | IFF_MULTICAST);
        d
    }

    /// [`estab`] over IPv6: an `INP_IPV6` control block in `tcb6table` from
    /// [2001:db8:`n`::1]:7000 to [2001:db8:`n`::2]:80 over a new interface `name` (MTU 1500,
    /// not Ethernet, so no neighbor to resolve) with 2001:db8:`n`::1/64 and its routes; the
    /// segment size taken from the route (1440), the template an IPv6 header.
    pub(crate) fn estab6(name: &[u8], n: u8) -> (SocketRef<G>, InpcbRef<G>, &'static Wire) {
        let d = iface6(name, n);
        let (so, inp) = testinet::pcb(&crate::net::domain::net::<G>().tcp.tcb6table);
        inp.set_flags(INP_IPV6);
        {
            let t = &crate::net::domain::net::<G>().tcp.tcb6table;
            let mut g = t.lock();
            inp.set_laddr6(&g, a6(n, 1));
            inp.set_faddr6(&g, a6(n, 2));
            inp.set_lport(&g, htons(7000));
            inp.set_fport(&g, htons(80));
            crate::netinet::in_pcb::in_pcbrehash(&mut g, &inp);
        }
        with_lock(&inp, |inp, il| {
            assert!(tcp_newtcpcb(inp, il, M_WAIT).is_some(), "a control block");
        });
        attach_slot(&inp);
        crate::net::socket::soreserve(&so, 16384, 16384).expect("the buffers");
        with_tp(&inp, |mut tp| {
            tp.st
                .clear_flags(crate::netinet::tcp_var::TF_REQ_SCALE | TF_REQ_TSTMP);
            let m = tcp_template(&mut tp).expect("a template");
            tp.st.t_template = Some(m);
            let _ = crate::netinet::tcp_input::tcp_mss(&mut tp, 0);
            let st = &mut *tp.st;
            st.t_state = crate::netinet::tcp_fsm::TCPS_ESTABLISHED;
            st.iss = 1000;
            st.snd_una = 1000;
            st.snd_nxt = 1000;
            st.snd_max = 1000;
            st.snd_up = 1000;
            st.irs = 4999;
            st.rcv_nxt = 5000;
            st.rcv_up = 5000;
            st.rcv_adv = 5000 + 16384;
            st.snd_wnd = 65535;
            st.max_sndwnd = 65535;
            st.t_rcvtime = crate::netinet::tcp_var::tcp_now();
            ((), None)
        });
        (so, inp, d)
    }

    /// The last packet `d` sent, an IPv6 one: its header, the TCP header, the TCP options and
    /// the payload.
    pub(crate) fn last6(d: &Wire) -> (Ip6Hdr, Tcphdr, std::vec::Vec<u8>, std::vec::Vec<u8>) {
        let p = d.last.lock().expect("the capture").clone();
        let ip6: Ip6Hdr = pod::at(&p, 0).expect("an IPv6 header");
        let th: Tcphdr = pod::at(&p, 40).expect("a TCP header");
        let thlen = usize::from(th.th_off()) * 4;
        let opts = p[60..40 + thlen].to_vec();
        let data = p[40 + thlen..].to_vec();
        (ip6, th, opts, data)
    }

    /// Appends `bytes` to the send buffer of `so`, as `sosend` leaves them for `tcp_output`.
    pub(crate) fn queue(so: &crate::net::socket::Socket<G>, bytes: &[u8]) {
        let mut m = m_gethdr(M_WAIT, sysdefs::mbuf::MT_DATA).expect("an mbuf");
        if bytes.len() > crate::net::mbuf::MHLEN {
            assert!(
                crate::net::mbuf::m_clget(&mut m, M_WAIT, bytes.len() as u32),
                "a cluster"
            );
        }
        m.m_len().set(bytes.len() as u32);
        m.m_pkthdr().len.set(bytes.len() as i32);
        m.data_mut()
            .expect("a new mbuf is writable")
            .copy_from_slice(bytes);
        so.so_snd.lock().sbappendstream(m);
    }

    /// The bytes `n` of a send buffer's test data: `n` mod 251, so offsets show.
    pub(crate) fn pattern(len: usize) -> std::vec::Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    /// The last packet `d` sent: its IP header, its TCP header, the TCP options and the
    /// payload (an IPv4 packet with a 20-byte IP header).
    pub(crate) fn last(d: &Wire) -> (Ip, Tcphdr, std::vec::Vec<u8>, std::vec::Vec<u8>) {
        let p = d.last.lock().expect("the capture").clone();
        let ip: Ip = pod::at(&p, 0).expect("an IP header");
        let th: Tcphdr = pod::at(&p, 20).expect("a TCP header");
        let thlen = usize::from(th.th_off()) * 4;
        let opts = p[40..20 + thlen].to_vec();
        let data = p[20 + thlen..].to_vec();
        (ip, th, opts, data)
    }

    /// The packets `d` has sent so far.
    pub(crate) fn sent(d: &Wire) -> u32 {
        d.sent.load(Ordering::Relaxed)
    }

    /// The counter `c` of `net.inet.tcp.stats` now (summed over the CPUs).
    pub(crate) fn stat(c: TcpstatCounters) -> u64 {
        crate::netinet::tcp_var::TCPCOUNTERS.read()[c as usize]
    }

    /// The source [`SigLegs`]'s inbound policy refuses (`tcp_input`'s SPD check): every
    /// other segment passes it, as the C's with IPsec and no policy.
    pub(crate) const SPD_DENY: [u8; 4] = [10, 9, 72, 66];
    /// The source whose datagrams [`SigLegs`]'s inbound policy passes with an SA's flow id
    /// ([`SPD_FLOW_ID`], UDP's `IP_IPSECFLOWINFO`, N6.6b0); every other source has none.
    pub(crate) const SPD_FLOW: [u8; 4] = [10, 9, 72, 67];
    /// The flow id of [`SPD_FLOW`]'s SA.
    pub(crate) const SPD_FLOW_ID: u32 = 0x0bad_f00d;

    /// The subnet whose addresses [`SigLegs`] has an SA between: 10.9.`SIG_NET`.0/24, and
    /// over IPv6 2001:db8:`SIG_NET`::/64 ([`a6`]`(SIG_NET, h)`).
    pub(crate) const SIG_NET: u8 = 71;
    /// The SA's MD5 key.
    pub(crate) const SIG_KEY: &[u8] = b"N6.6a3 tcp md5 key";
    /// The SA, a `u32` standing for `bsd`'s `struct tdb`.
    static SIG_SA: u32 = 0x5a5a;

    /// IPsec's legs for the host tests of `TCP_SIGNATURE`: no policy (the C with IPSEC and no
    /// SA: not in use, every SPD lookup empty, every check passing, the sysctl names
    /// `EOPNOTSUPP` as without IPSEC), and one SA, between the addresses of
    /// 10.9.[`SIG_NET`].0/24 (or of 2001:db8:[`SIG_NET`]::/64), keyed [`SIG_KEY`]; the
    /// references given back are counted. The
    /// bridge is set once for the run ([`sig_legs`], `register_ipsec`): the other tests'
    /// addresses find no SA, so their `tcp_output` and `syn_cache_respond` answer `EPERM` as
    /// without the legs, and every other leg behaves as the C without an SA.
    pub(crate) struct SigLegs {
        /// `tdb_unref` calls.
        pub(crate) unrefs: core::sync::atomic::AtomicU32,
        /// `udpencap_ctlinput` calls (UDP's path MTU branch, N6.6b0).
        pub(crate) udpencap_ctl: core::sync::atomic::AtomicU32,
        /// `udpencap_input` calls: `skip`, `protoff`, `af` and the packet's first bytes (at
        /// most 26), the packet freed (N6.6b0).
        #[allow(clippy::type_complexity, reason = "a test's record")]
        pub(crate) udpencap_in: std::sync::Mutex<std::vec::Vec<(i32, i32, i32, std::vec::Vec<u8>)>>,
        /// The capability `register_ipsec` gave.
        key: std::sync::OnceLock<&'static crate::net::ipsec_hooks::TdbKey>,
    }

    /// The legs, registered by [`sig_legs`].
    static SIG_LEGS: SigLegs = SigLegs {
        unrefs: core::sync::atomic::AtomicU32::new(0),
        udpencap_ctl: core::sync::atomic::AtomicU32::new(0),
        udpencap_in: std::sync::Mutex::new(std::vec::Vec::new()),
        key: std::sync::OnceLock::new(),
    };

    /// [`SigLegs`], registered as IPsec's legs on the first call.
    pub(crate) fn sig_legs() -> &'static SigLegs {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let key = crate::net::ipsec_hooks::register_ipsec(&SIG_LEGS)
                .expect("no other IPsec legs in the kernel's host tests");
            assert!(SIG_LEGS.key.set(key).is_ok());
        });
        &SIG_LEGS
    }

    impl SigLegs {
        /// Whether `tdb` is the test's SA.
        fn ours(&self, tdb: &crate::net::ipsec_hooks::TdbRef) -> bool {
            let key = self.key.get().expect("registered");
            let sa = tdb.get(key);
            core::ptr::addr_eq(core::ptr::from_ref(sa), core::ptr::from_ref(&SIG_SA))
        }
    }

    impl crate::net::ipsec_hooks::Ipsec for SigLegs {
        fn enc_getifa(&self, _rtableid: u32, _unit: u32) -> Option<crate::net::if_var::IfaddrRef> {
            None
        }
        fn in_use(&self) -> bool {
            false
        }
        fn ip_output_lookup(
            &self,
            _m: &Mbuf,
            _hlen: i32,
            _seclevel: Option<&crate::net::ipsec_hooks::IpsecLevel>,
            _ipsecflowinfo: u32,
        ) -> Result<Option<crate::net::ipsec_hooks::TdbRef>, crate::net::ipsec_hooks::SpdError>
        {
            Ok(None)
        }
        fn ip_output_send(
            &self,
            _tdb: &crate::net::ipsec_hooks::TdbRef,
            _m: MbufBox,
            _ro: &mut crate::net::route::Route,
            _rtableid: u32,
            _fwd: bool,
        ) -> Result<(), Errno> {
            panic!("no SPD lookup answers an SA")
        }
        fn ip6_output_lookup(
            &self,
            _m: &Mbuf,
            _seclevel: Option<&crate::net::ipsec_hooks::IpsecLevel>,
        ) -> Result<Option<crate::net::ipsec_hooks::TdbRef>, crate::net::ipsec_hooks::SpdError>
        {
            Ok(None)
        }
        fn ip6_output_send(
            &self,
            _tdb: &crate::net::ipsec_hooks::TdbRef,
            _m: MbufBox,
            _ro: &mut crate::net::route::Route,
            _rtableid: u32,
            _fwd: bool,
        ) -> Result<(), Errno> {
            panic!("no SPD lookup answers an SA")
        }
        fn tdb_unref(&self, tdb: crate::net::ipsec_hooks::TdbRef) {
            assert!(self.ours(&tdb), "the test's SA");
            self.unrefs.fetch_add(1, Ordering::Relaxed);
        }
        fn adjust_mtu(&self, _m: &Mbuf, _mtu: u32) {}
        fn spd_check_in(
            &self,
            m: &Mbuf,
            _af: i32,
            _hlen: i32,
            _seclevel: Option<&crate::net::ipsec_hooks::IpsecLevel>,
        ) -> Result<(), crate::net::ipsec_hooks::SpdError> {
            // The run's one inbound policy: what SPD_DENY sends is refused.
            match pod::at::<Ip>(m.data(), 0) {
                Some(ip) if ip.ip_v() == 4 && ip.ip_src.s_addr.to_ne_bytes() == SPD_DENY => {
                    Err(crate::net::ipsec_hooks::SpdError::Drop)
                }
                _ => Ok(()),
            }
        }
        fn spd_check_in_flow(
            &self,
            m: &Mbuf,
            _af: i32,
            _hlen: i32,
            _seclevel: Option<&crate::net::ipsec_hooks::IpsecLevel>,
        ) -> Result<u32, crate::net::ipsec_hooks::SpdError> {
            // SPD_DENY refused, SPD_FLOW's SA giving its flow id, no SA for the others.
            match pod::at::<Ip>(m.data(), 0) {
                Some(ip) if ip.ip_v() == 4 && ip.ip_src.s_addr.to_ne_bytes() == SPD_DENY => {
                    Err(crate::net::ipsec_hooks::SpdError::Drop)
                }
                Some(ip) if ip.ip_v() == 4 && ip.ip_src.s_addr.to_ne_bytes() == SPD_FLOW => {
                    Ok(SPD_FLOW_ID)
                }
                _ => Ok(0),
            }
        }
        fn udpencap_input(
            &self,
            mp: &mut Option<MbufBox>,
            skip: i32,
            protoff: i32,
            af: i32,
            _ns: Option<&mut crate::net::if_var::Netstack>,
        ) -> i32 {
            if let Some(m) = mp.take() {
                let n = (m.m_pkthdr().len.get() as usize).min(26);
                let mut head = std::vec![0u8; n];
                crate::net::mbuf::m_copydata(&m, 0, &mut head);
                if let Ok(mut r) = self.udpencap_in.lock() {
                    r.push((skip, protoff, af, head));
                }
            }
            sysdefs::netinet_in::IPPROTO_DONE
        }
        fn udpencap_ctlinput(&self, _rdomain: u32, _icmp: &[u8]) {
            self.udpencap_ctl.fetch_add(1, Ordering::Relaxed);
        }
        fn tcp_signature_tdb(
            &self,
            _rdomain: u32,
            src: &[u8; SOCKADDR_UNION_LEN],
            dst: &[u8; SOCKADDR_UNION_LEN],
        ) -> Option<crate::net::ipsec_hooks::TdbRef> {
            let net = [10, 9, SIG_NET];
            let inet = |u: &[u8; SOCKADDR_UNION_LEN]| {
                u[0] == 16 && u[1] == AF_INET && u[4..7] == net && u[8..] == [0; 20]
            };
            // And over IPv6, 2001:db8:SIG_NET::/64: a sockaddr_in6, the address at 8.
            let net6 = a6(SIG_NET, 0).s6_addr;
            let inet6 = |u: &[u8; SOCKADDR_UNION_LEN]| {
                u[0] == 28
                    && u[1] == sysdefs::socket::AF_INET6
                    && u[2..8] == [0; 6]
                    && u[8..16] == net6[..8]
                    && u[24..] == [0; 4]
            };
            ((inet(src) && inet(dst)) || (inet6(src) && inet6(dst))).then(|| {
                crate::net::ipsec_hooks::TdbRef::new(self.key.get().expect("registered"), &SIG_SA)
            })
        }
        fn tdb_amxkey(&self, tdb: &crate::net::ipsec_hooks::TdbRef, f: &mut dyn FnMut(&[u8])) {
            assert!(self.ours(tdb), "the test's SA");
            f(SIG_KEY);
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the kernel's `tcp_subr.rs`: window scaling, the RFC 2385 digests against
    // an independent MD5 of the same bytes, a reply to a segment as the wire sees it.

    use hal::sync::netlock::NETLOCK;
    use sysdefs::netinet_in::InAddr;
    use sysdefs::netinet6_in6::In6Addr;

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
            th_flags: sysdefs::netinet_tcp::TH_SYN,
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
        // The pcb's handles: ours, the socket's anchor, its two so_pcb slots' (the counted
        // one and the requests' locked one), and one per timer (D-11).
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
        // The anchor's, the socket's typed so_pcb slot's (N6.6a4, emptied by in_pcbdetach),
        // its locked slot's (N6.6a5, emptied by tcp_free) and the timers'.
        assert_eq!(
            PoolArc::count(&inp),
            handles - TCPT_NTIMERS - 3,
            "the timers', the anchor's and the two so_pcb slots'"
        );
        // The pcb is detached: its socket's lock finds no pcb any more.
        {
            let w = hal::sync::netlock::NETLOCK.read();
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

        // The digest's inputs in the C's order: the secret, the routing domain, the local and
        // foreign ports, the local and foreign addresses (each as the pcb holds it).
        {
            let t = &crate::net::domain::net::<testinet::G>().tcp.tcbtable;
            let g = t.lock();
            inp.set_laddr(
                &g,
                InAddr {
                    s_addr: u32::from_ne_bytes([10, 1, 2, 3]),
                },
            );
            inp.set_faddr(
                &g,
                InAddr {
                    s_addr: u32::from_ne_bytes([10, 4, 5, 6]),
                },
            );
            inp.set_lport(&g, htons(1111));
            inp.set_fport(&g, htons(2222));
        }
        let before = TCP_ISS.load(Ordering::Relaxed);
        let (iss, tsm) = testtcp::with_tp(&inp, |mut tp| {
            tcp_set_iss_tsm(&mut tp);
            ((tp.st.iss, tp.st.ts_modulate), None)
        });
        let mut ctx = Sha512Ctx::new();
        ctx.update(TCP_SECRET.get().expect("the secret"));
        ctx.update(&rtable_l2(0).to_ne_bytes());
        ctx.update(&htons(1111).to_ne_bytes());
        ctx.update(&htons(2222).to_ne_bytes());
        ctx.update(&[10, 1, 2, 3]);
        ctx.update(&[10, 4, 5, 6]);
        let d = ctx.finalize();
        let word = |i: usize| u32::from_ne_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
        assert_eq!(
            iss,
            word(0).wrapping_add(before.wrapping_add(TCP_ISS_CONN_INC))
        );
        assert_eq!(tsm, word(4));
        testtcp::close(so, inp);
    }

    /// The two pools' limits put back, whatever the test's assertions do.
    struct Limits(i32, i32);

    impl Drop for Limits {
        fn drop(&mut self) {
            TCP_REASS_LIMIT.store(self.0, Ordering::Relaxed);
            tcp_sackhole_limit.store(self.1, Ordering::Relaxed);
            let _ = TCPQE_POOL.set_hardlimit(self.0 as u32);
            let _ = SACKHL_POOL.set_hardlimit(self.1 as u32);
        }
    }

    #[test]
    fn tcp_init_sets_the_pools_hard_limits() {
        let (_g, _p) = testtcp::setup();
        let _limits = Limits(
            TCP_REASS_LIMIT.load(Ordering::Relaxed),
            tcp_sackhole_limit.load(Ordering::Relaxed),
        );
        let (qe, sh) = (TCPQE_POOL.nout() as i32, SACKHL_POOL.nout() as i32);
        TCP_REASS_LIMIT.store(qe + 2, Ordering::Relaxed);
        tcp_sackhole_limit.store(sh + 2, Ordering::Relaxed);
        tcp_init::<testinet::G>();
        let q: std::vec::Vec<_> = (0..3)
            .map(|_| {
                TCPQE_POOL.get(PR_NOWAIT, || Tcpqent {
                    tcpqe_q: hal::ds::owned::SlistLink::new(),
                    tcpqe_tcp: Tcphdr::default(),
                    tcpqe_m: None,
                })
            })
            .collect();
        let h: std::vec::Vec<_> = (0..3)
            .map(|_| SACKHL_POOL.get(PR_NOWAIT, || Sackhole::new(0, 1, 1, 0)))
            .collect();
        assert_eq!(q.iter().flatten().count(), 2, "tcpqe: tcp_reass_limit");
        assert_eq!(h.iter().flatten().count(), 2, "sackhl: tcp_sackhole_limit");
    }

    /// The socket of `inp` locked under the shared net lock (a control input's holding),
    /// for `f`.
    fn with_so<R>(
        inp: &Inpcb<testinet::G>,
        f: impl FnOnce(&mut SoLocked<'_>, &NetHeld<'_>) -> R,
    ) -> R {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut so = crate::netinet::in_pcb::in_pcbsolock(inp, &net).expect("attached");
        f(&mut so, &net)
    }

    /// The connection's soft error and its socket's error.
    fn errors(inp: &Inpcb<testinet::G>) -> (Option<Errno>, Option<Errno>) {
        let soft = testtcp::with_tp(inp, |tp| (tp.st.t_softerror, None));
        (soft, inp.socket().error())
    }

    #[test]
    fn tcp_drop_resets_a_synchronized_connection_and_reports_the_error() {
        use crate::netinet::tcp_fsm::TCPS_SYN_SENT;
        let (_g, _p) = testtcp::setup();
        let tcpcbs = testtcp::tcpcbs();
        // Synchronized: an RST (CLOSED's flags) before the close, tcps_drops.
        let (so, inp, d) = testtcp::estab(b"tsub0", 60);
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        let (drops, before) = (testtcp::stat(TcpstatCounters::TcpsDrops), testtcp::sent(d));
        testtcp::with_tp(&inp, |tp| {
            let closed = tcp_drop(tp, Some(Errno::ECONNABORTED));
            ((), Some(closed))
        });
        assert_eq!(testtcp::sent(d), before + 1);
        let (_ip, th, _o, _p) = testtcp::last(d);
        assert_eq!(th.th_flags, TH_RST | TH_ACK);
        assert_eq!(testtcp::stat(TcpstatCounters::TcpsDrops), drops + 1);
        assert_eq!(so.error(), Some(Errno::ECONNABORTED));
        drop((so, inp));
        assert_eq!(testtcp::tcpcbs(), tcpcbs, "the control block freed");

        // Not synchronized: no RST, tcps_conndrops; a timeout reports the soft error.
        let (so, inp, d) = testtcp::estab(b"tsub1", 61);
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        let (cdrops, before) = (
            testtcp::stat(TcpstatCounters::TcpsConndrops),
            testtcp::sent(d),
        );
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_SYN_SENT;
            tp.st.t_softerror = Some(Errno::EHOSTDOWN);
            let closed = tcp_drop(tp, Some(Errno::ETIMEDOUT));
            ((), Some(closed))
        });
        assert_eq!(testtcp::sent(d), before);
        assert_eq!(testtcp::stat(TcpstatCounters::TcpsConndrops), cdrops + 1);
        assert_eq!(
            so.error(),
            Some(Errno::EHOSTDOWN),
            "the soft error for a timeout"
        );
        drop((so, inp));
        assert_eq!(testtcp::tcpcbs(), tcpcbs);
    }

    #[test]
    fn tcp_notify_keeps_soft_errors_and_gives_up_on_a_failing_open() {
        use crate::netinet::tcp_fsm::TCPS_SYN_SENT;
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = testtcp::estab(b"tsub2", 62);
        // Established: an unreachable is ignored.
        with_so(&inp, |w, _| tcp_notify(&inp, w, Some(Errno::EHOSTUNREACH)));
        assert_eq!(errors(&inp), (None, None));
        // Another error: kept soft.
        with_so(&inp, |w, _| tcp_notify(&inp, w, Some(Errno::ECONNREFUSED)));
        assert_eq!(errors(&inp), (Some(Errno::ECONNREFUSED), None));
        // Opening, retransmitted four times, a soft error already: reported to the socket.
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_SYN_SENT;
            tp.st.t_rxtshift = 4;
            ((), None)
        });
        with_so(&inp, |w, _| tcp_notify(&inp, w, Some(Errno::EHOSTUNREACH)));
        assert_eq!(
            errors(&inp),
            (Some(Errno::ECONNREFUSED), Some(Errno::EHOSTUNREACH))
        );
        testtcp::close(so, inp);
    }

    /// An ICMP message of type `ty` and code `code`, its next-hop MTU `mtu`, returning our
    /// segment from 10.9.`n`.1:7000 to `peer`:80 with sequence number `seq`: its IPv4 header
    /// and the TCP header's first eight bytes.
    fn quote(ty: u8, code: u8, mtu: u16, n: u8, peer: [u8; 4], seq: u32) -> std::vec::Vec<u8> {
        let mut msg = std::vec![ty, code, 0, 0, 0, 0];
        msg.extend_from_slice(&mtu.to_be_bytes());
        let ip = Ip {
            ip_vhl: 0x45,
            ip_len: htons(1040),
            ip_ttl: 64,
            ip_p: IPPROTO_TCP as u8,
            ip_src: InAddr {
                s_addr: u32::from_ne_bytes([10, 9, n, 1]),
            },
            ip_dst: InAddr {
                s_addr: u32::from_ne_bytes(peer),
            },
            ..Ip::default()
        };
        let mut b = [0u8; 20];
        assert!(pod::put(&mut b, 0, &ip));
        msg.extend_from_slice(&b);
        msg.extend_from_slice(&7000u16.to_be_bytes());
        msg.extend_from_slice(&80u16.to_be_bytes());
        msg.extend_from_slice(&seq.to_be_bytes());
        msg
    }

    /// `tcp_ctlinput(cmd, sin(peer), 0, msg)` under the shared net lock (`icmp_input`'s).
    fn ctl(cmd: i32, peer: [u8; 4], msg: Option<&[u8]>) {
        let mut sin = [0u8; 16];
        sin[0] = 16;
        sin[1] = AF_INET;
        sin[4..8].copy_from_slice(&peer);
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let arg = msg.map_or(CtlArg::None, CtlArg::Ip);
        tcp_ctlinput::<testinet::G>(
            cmd,
            SockaddrView::new(&sin).expect("a sockaddr_in"),
            0,
            arg,
            &net,
        );
    }

    #[test]
    fn the_control_input_checks_the_quoted_sequence_number() {
        use crate::net::proto::PRC_UNREACH_HOST;
        use crate::netinet::tcp_fsm::TCPS_SYN_RECEIVED;
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = testtcp::estab(b"tsub3", 63);
        let peer = [10, 9, 63, 2];
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_SYN_RECEIVED;
            tp.st.snd_max = 1300;
            ((), None)
        });
        // Below snd_una and at snd_max: not a segment in flight, nothing told.
        for seq in [999, 1300] {
            ctl(PRC_UNREACH_HOST, peer, Some(&quote(3, 1, 0, 63, peer, seq)));
            assert_eq!(errors(&inp), (None, None), "seq {seq}");
        }
        // In flight: tcp_notify keeps the error.
        ctl(
            PRC_UNREACH_HOST,
            peer,
            Some(&quote(3, 1, 0, 63, peer, 1000)),
        );
        assert_eq!(errors(&inp), (Some(Errno::EHOSTUNREACH), None));
        // Source quench, a code past PRC_NCMDS, another family: ignored.
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_softerror = None;
            ((), None)
        });
        ctl(PRC_QUENCH, peer, Some(&quote(4, 0, 0, 63, peer, 1000)));
        ctl(
            PRC_NCMDS as i32,
            peer,
            Some(&quote(3, 1, 0, 63, peer, 1000)),
        );
        assert_eq!(errors(&inp), (None, None));
        // A dead host: every connection to it is told (no quote).
        ctl(PRC_HOSTDEAD, peer, None);
        assert_eq!(errors(&inp), (Some(Errno::EHOSTDOWN), None));
        testtcp::close(so, inp);
    }

    /// `tcp6_ctlinput(cmd, sin6(peer), 0, ip6cp)` under the shared net lock, the parameter
    /// `icmp6_notify_error` lends: the quoted packet's IPv6 header (from 2001:db8:`n`::1 to
    /// 2001:db8:`n`::2) and the TCP header's first eight bytes (7000 to 80, `seq`) at 40.
    fn ctl6(cmd: i32, n: u8, seq: u32) {
        use core::cell::Cell;
        use sysdefs::netinet_icmp6::Icmp6Hdr;

        use crate::netinet::tcp_subr::testtcp::a6;
        use crate::netinet6::icmp6::Ip6CtlParam;

        let (ours, peer) = (a6(n, 1), a6(n, 2));
        let ip6 = Ip6Hdr {
            ip6_plen: 8u16.to_be(),
            ip6_nxt: IPPROTO_TCP as u8,
            ip6_hlim: 64,
            ip6_src: ours,
            ip6_dst: peer,
            ..Ip6Hdr::default()
        };
        let mut b = [0u8; 48];
        assert!(pod::put(&mut b, 0, &ip6));
        b[40..42].copy_from_slice(&7000u16.to_be_bytes());
        b[42..44].copy_from_slice(&80u16.to_be_bytes());
        b[44..48].copy_from_slice(&seq.to_be_bytes());
        let m = crate::net::mbuf::m_devget(&b, 0).expect("the quote");
        let p = Ip6CtlParam {
            m: &m,
            icmp6: Icmp6Hdr::default(),
            ip6,
            off: 40,
            src: SockaddrIn6::with_addr(ours),
            finaldst: Cell::new(peer),
            cmdarg: None,
            nxt: IPPROTO_TCP as u8,
        };
        let mut sa = [0u8; 28];
        assert!(pod::put(&mut sa, 0, &SockaddrIn6::with_addr(peer)));
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        tcp6_ctlinput::<testinet::G>(
            cmd,
            SockaddrView::new(&sa).expect("a sockaddr_in6"),
            0,
            CtlArg::Ip6(&p),
            &net,
        );
    }

    #[test]
    fn the_ipv6_control_input_checks_the_quoted_sequence_number() {
        use crate::net::proto::PRC_UNREACH_HOST;
        use crate::netinet::tcp_fsm::TCPS_SYN_RECEIVED;
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = testtcp::estab6(b"tsub6", 81);
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_SYN_RECEIVED;
            tp.st.snd_max = 1300;
            ((), None)
        });
        // Below snd_una and at snd_max: not a segment in flight, nothing told.
        for seq in [999, 1300] {
            ctl6(PRC_UNREACH_HOST, 81, seq);
            assert_eq!(errors(&inp), (None, None), "seq {seq}");
        }
        // In flight: tcp_notify keeps the error.
        ctl6(PRC_UNREACH_HOST, 81, 1000);
        assert_eq!(errors(&inp), (Some(Errno::EHOSTUNREACH), None));
        testtcp::close(so, inp);
    }

    #[test]
    fn an_ipv6_dead_host_is_told_and_an_ipv6_redirect_flushes_the_route() {
        use crate::net::proto::PRC_REDIRECT_HOST;
        use crate::netinet::tcp_fsm::TCPS_SYN_RECEIVED;
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = testtcp::estab6(b"tsub7", 83);
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_SYN_RECEIVED;
            ((), None)
        });
        // A dead host: tcp_notify for every connection to it (the quote set aside).
        ctl6(PRC_HOSTDEAD, 83, 999);
        assert_eq!(errors(&inp), (Some(Errno::EHOSTDOWN), None));
        // A redirect: in_rtchange, the cached route let go.
        let cached = |inp: &Inpcb<testinet::G>| {
            testtcp::with_lock(inp, |inp, _il| inp.route().rt().is_some())
        };
        assert!(cached(&inp), "tcp_mss cached the route");
        ctl6(PRC_REDIRECT_HOST, 83, 1000);
        assert!(!cached(&inp), "in_rtchange");
        testtcp::close(so, inp);
    }

    #[test]
    fn a_redirect_flushes_the_cached_route_of_every_connection_to_the_peer() {
        use crate::net::proto::PRC_REDIRECT_HOST;
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = testtcp::estab(b"tsub4", 64);
        let cached = |inp: &Inpcb<testinet::G>| {
            testtcp::with_lock(inp, |inp, _il| inp.route().rt().is_some())
        };
        assert!(cached(&inp), "tcp_mss cached the route");
        ctl(
            PRC_REDIRECT_HOST,
            [10, 9, 64, 2],
            Some(&quote(5, 1, 0, 64, [10, 9, 64, 2], 1000)),
        );
        assert!(!cached(&inp), "in_pcbrtchange");
        testtcp::close(so, inp);
    }

    #[test]
    fn a_path_mtu_message_lowers_the_segment_or_waits_for_the_retransmit_timer() {
        let (_g, _p) = testtcp::setup();
        let peer = [10, 20, 65, 2];
        let (so, inp, _d) = testtcp::estab_to(b"tsub5", 65, peer);
        let set = |mtu_sent: u32, acked: u32| {
            testtcp::with_tp(&inp, |tp| {
                let st = tp.st;
                st.snd_max = 1300;
                st.t_pmtud_mtu_sent = mtu_sent;
                st.t_pmtud_mss_acked = acked;
                ((), None)
            });
        };
        let look = || {
            testtcp::with_tp(&inp, |tp| {
                let st = &*tp.st;
                (
                    (
                        st.t_maxseg,
                        st.has_flags(TF_PMTUD_PEND),
                        st.t_pmtud_th_seq,
                        st.t_pmtud_nextmtu,
                    ),
                    None,
                )
            })
        };
        // A segment not in flight: ignored whatever the MTU.
        set(1500, 0);
        ctl(PRC_MSGSIZE, peer, Some(&quote(3, 4, 1000, 65, peer, 1300)));
        assert_eq!(look(), (1460, false, 0, 0));
        // A next-hop MTU at or past the largest packet sent: dropped (1400, below the
        // interface's 1500, would lower the segment if it were taken).
        set(1400, 0);
        ctl(PRC_MSGSIZE, peer, Some(&quote(3, 4, 1400, 65, peer, 1000)));
        assert_eq!(look(), (1460, false, 0, 0));
        set(1500, 0);
        // Larger than the headers and what was acknowledged: the route's MTU lowered
        // (icmp_mtudisc), and every connection to the peer takes it (tcp_mtudisc).
        ctl(PRC_MSGSIZE, peer, Some(&quote(3, 4, 1000, 65, peer, 1000)));
        assert_eq!(look().0, 1000 - 40);
        // Smaller: kept for the retransmit timer, an older segment's notice over a newer one.
        set(1500, 1400);
        ctl(PRC_MSGSIZE, peer, Some(&quote(3, 4, 900, 65, peer, 1100)));
        assert_eq!(look(), (960, true, 1100, u32::from(900u16.to_be())));
        ctl(PRC_MSGSIZE, peer, Some(&quote(3, 4, 800, 65, peer, 1200)));
        assert_eq!(
            look(),
            (960, true, 1100, u32::from(900u16.to_be())),
            "newer: kept"
        );
        ctl(PRC_MSGSIZE, peer, Some(&quote(3, 4, 700, 65, peer, 1050)));
        assert_eq!(
            look(),
            (960, true, 1050, u32::from(700u16.to_be())),
            "older: taken"
        );
        let (ip_len, ip_hl) = testtcp::with_tp(&inp, |tp| {
            ((tp.st.t_pmtud_ip_len, tp.st.t_pmtud_ip_hl), None)
        });
        assert_eq!((ip_len, ip_hl), (htons(1040), 5), "the returned header's");
        testtcp::close(so, inp);
    }

    #[test]
    fn tcp_init_registers_the_ipv6_path_mtu_callback() {
        let (_g, _p) = testtcp::setup();
        let cbs = crate::netinet6::icmp6::ICMP6_MTUDISC_CALLBACKS.read();
        let mine: crate::netinet6::icmp6::Mtudisc6Fn = tcp6_mtudisc_callback::<testinet::G>;
        assert!(
            cbs.as_deref()
                .unwrap_or(&[])
                .iter()
                .any(|f| core::ptr::fn_addr_eq(*f, mine)),
            "tcp6_mtudisc_callback registered"
        );
    }
}
/* </TESTS> */
