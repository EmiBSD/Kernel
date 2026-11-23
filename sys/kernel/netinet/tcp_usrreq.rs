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
//! TCP user requests and socket options: `netinet/tcp_usrreq.c`.
//!
//! Upstream: sys/netinet/tcp_usrreq.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_usrreq.rs@3c62ede68802
//!
//! The `pr_usrreqs` of TCP sockets ([`TcpUsrreqs`]): attaching makes the internet and TCP
//! control blocks and the socket buffers; listen, connect (a SYN sent), accept, send (the
//! data appended to the send buffer, then `tcp_output`), shutdown and disconnect (the FIN
//! states walked by [`tcp_usrclosed`]), abort (`tcp_drop`), out-of-band data and the
//! addresses. [`tcp_ctloutput`] serves the `IPPROTO_TCP` socket options (`TCP_NODELAY`,
//! `TCP_NOPUSH`, `TCP_MAXSEG`, `TCP_SACK_ENABLE`, `TCP_INFO`, `TCP_MD5SIG`) and hands the
//! others to IP; [`tcp_sysctl`] is `net.inet.tcp`, [`tcp_ident`] its `ident` and `drop`
//! requests. The send buffer grows with the data in flight, so that what the peer has not
//! acknowledged is not counted against the limit the user asked for; the receive buffer
//! grows by a step when most of it was filled within about one round trip. Both shrink back
//! to their defaults when mbufs run low, and keep a socket writable or readable that
//! `poll(2)` found so.
//!
//! Locks used to protect global variables in this file: \[I\] immutable after creation.
//!
//! ## Deviations
//! - `tcp_sendspace`, `tcp_recvspace` and `tcp_autorcvbuf_inc` are \[I\] `u32` statics, as
//!   LZ's; the first two keep their lowercase names (`TCP_SENDSPACE` and `TCP_RECVSPACE` are
//!   this file's constants). `tcpctl_vars[]` points at the `AtomicI32`s of the files that
//!   define them.
//! - Not a behavioural deviation, a defensive answer: `tcp_ctloutput` answers `ECONNRESET`
//!   for a pcb without a control block, where the C dereferences `intotcpcb(inp)` unchecked
//!   (LZ's). The case cannot arise in the C or here: the close that frees the block
//!   (`tcp_close`, C `tcp_subr.c:526-529`) detaches the pcb under the same socket lock, and
//!   `in_pcbdetach` clears `so_pcb` (C `in_pcb.c:591`), so the request finds no pcb and
//!   answers the C's own `ECONNRESET` first.
//! - `INET6` is configured: the `PF_INET6` attach, `ip6_ctloutput`, the IPv6 checks of
//!   `tcp_connect` and `tcp_ident`'s `AF_INET6` case. `TCP_ECN` and `TCP_SIGNATURE` are
//!   configured. `SMALL_KERNEL` is not set: `tcpctl_vars`, `tcp_sysctl_tcpstat` and
//!   `tcp_sysctl` are compiled.
//!
//! ## Redesign
//! - N6.6a3: `tcp_update_sndspace` and `tcp_update_rcvspace` are the kernel's, over the
//!   kernel's control block (opened as [`Tp`]) and socket buffers. The buffer's mutex is
//!   held across the computation and the reserve, as the C's `sb_mtx`; the counts and marks
//!   are read under it from the buffer's words. The sums keep the C's `u_long` order and wrap
//!   (`sb_wat + snd_max - snd_una`, which LZ computed as `sb_wat + (snd_max - snd_una)`:
//!   across the sequence space's wrap the C's difference is past `sb_max` and the buffer goes
//!   to `sb_max` for that call), and the marks' comparisons are the C's unsigned ones
//!   (`sb_cc >= sb_lowat` and `nmax < sb_lowat` with the `long` mark converted, where LZ
//!   compared them signed).
//! - N6.6a5: LZ's `tcp_usrreqs`/`tcp6_usrreqs` tables of functions over `&'static Socket`
//!   and parked mbufs are [`TcpUsrreqs`] (decision 46: `impl Usrreqs`), one type whose two
//!   values differ only in `pru_control` (`in_control`, `in6_control`), as the C's two tables
//!   share every other function. Each request takes the socket's lock as a witness
//!   ([`SoLocked`], the C's `soassertlocked`), opens it in parts ([`SoLocked::inet`]) and
//!   reaches the pcb and the control block through them: `tcp_sogetpcb`'s two pointers are
//!   one [`Tp`], the pcb borrowed from the socket's locked slot (`sotoinpcb_locked`, part
//!   `PCB`) and the block from the pcb's (`intotcpcb`), for as long as the parts are
//!   borrowed: the C's two loads, no count taken (the review's S3, the user's decision). The
//!   socket's counted slot (`sotoinpcb`, a reader pair and a count pair) stays for the
//!   readers without the socket lock (`kern.file`, `fstat`, N6.6a4). `tcp_attach` fills the
//!   locked slot with `in_pcballoc`'s handle once the block is made; `tcp_free` empties it.
//! - `tp = tcp_close(tp)`, `tp = tcp_drop(tp, e)` and the `tcp_dodisconnect`/
//!   `tcp_usrclosed` answers (a block or NULL) are [`TpOrClosed`]: the request that owns the
//!   socket lock's parts frees a [`Closed`] block before it unlocks, so a use of the block
//!   after its close does not compile; nor of the pcb, which the request borrowed from the
//!   locked slot: freeing the block takes the pcb out of the slot (`tcp_free_request`, the
//!   part `PCB` borrowed mutably), which compiles only once the borrow has ended, and the
//!   slot's count keeps the pcb from `tcp_close`'s `in_pcbdetach` to then. `SO_DEBUG`'s
//!   `otp` is the block's address before the request, never followed.
//! - `tcp_ident` reads the two `sockaddr_storage`s whole and takes `sockaddr_in` or
//!   `sockaddr_in6` from their first bytes by family (`hal::pod::at`), never cut at
//!   `sa_len`: the C's casts. `TCPCTL_DROP` drops the connection under its socket's lock
//!   taken by `in_pcbsolock` (the C's), the pcb counted by the lookup until after the unlock.
//! - `tcp_sysctl`'s port bitmaps go through the C's `malloc(M_SYSCTL)` buffer (an `MArray`,
//!   where LZ put 8 KiB on the stack; N6.6b0: `in_pcb::in_sysctl_ports`, which
//!   `udp_sysctl`'s identical block shares); `tcp_sysctl_tcpstat` builds `struct tcpstat` from the
//!   per-CPU counters in the C's `ASSIGN` order and copies its bytes out (`hal::sys`'s image,
//!   holes named). `tcp_syn_hash_size` and `tcp_syn_use_limit` are atomics beside the SYN
//!   cache's mutex (`TcpTables`), written under it and read without it, as the C's
//!   `atomic_load_int`.
//! - `sosplice`'s `pru_send` comparison is the address of one static (`TCP_SEND_ID`), the
//!   same for both tables, as the C's two tables share `tcp_send`.
//! - The switch (N6.6a5): `inetsw[]`'s and `inet6sw[]`'s TCP rows are the kernel's
//!   (`in_proto::TcpProtocol`, `in6_proto::Tcp6Protocol`) with these requests; LZ's
//!   `tcbtable`/`tcb6table` statics are the glue's `TcpTables` members, reached by
//!   [`tcbtable`] and [`tcb6table`] (pf's socket lookup and `kern.file`'s walk, `bsd`'s).

use core::marker::PhantomData;
use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::netlock::{NETLOCK, NetHeld};
use hal::pool::PoolArc;
use hal::rwlock::{RW_INTR, WitnessPart};
use hal::sys::errno::Errno;
use hal::sys::netinet_in::{INADDR_ANY, INADDR_BROADCAST, IPPROTO_TCP, SockaddrIn, in_multicast};
use hal::sys::netinet_tcp::{
    TCP_INFO, TCP_MAXSEG, TCP_MD5SIG, TCP_MSS, TCP_NODELAY, TCP_NOPUSH, TCP_SACK_ENABLE,
    TCPI_OPT_ECN, TCPI_OPT_SACK, TCPI_OPT_TIMESTAMPS, TCPI_OPT_WSCALE, TcpInfo,
};
use hal::sys::netinet_tcp_var::{
    TCPCTL_ACK_ON_PUSH, TCPCTL_ALWAYS_KEEPALIVE, TCPCTL_BADDYNAMIC, TCPCTL_DROP, TCPCTL_ECN,
    TCPCTL_IDENT, TCPCTL_KEEPIDLE, TCPCTL_KEEPINITTIME, TCPCTL_KEEPINTVL, TCPCTL_MSSDFLT,
    TCPCTL_REASS_LIMIT, TCPCTL_RFC1323, TCPCTL_RFC3390, TCPCTL_ROOTONLY, TCPCTL_RSTPPSLIMIT,
    TCPCTL_SACK, TCPCTL_SACKHOLE_LIMIT, TCPCTL_STATS, TCPCTL_SYN_BUCKET_LIMIT,
    TCPCTL_SYN_CACHE_LIMIT, TCPCTL_SYN_HASH_SIZE, TCPCTL_SYN_USE_LIMIT, TCPCTL_TSO,
    TcpIdentMapping, Tcpstat,
};
use hal::sys::netinet6_in6::{
    In6Addr, SockaddrIn6, in6_is_addr_multicast, in6_is_addr_unspecified,
};
use hal::sys::socket::{
    AF_INET, AF_INET6, MSG_PEEK, PF_INET, PF_INET6, SO_ACCEPTCONN, SO_DEBUG, SO_LINGER,
    SO_OOBINLINE,
};
use hal::sys::stat::Stat;
use hal::sys::sysctl::SysctlBoundedArgs;
use hal::user::{AbiPod, UserPtr, bytes_of};

use crate::assert::kpanic;
use crate::net::domain::net;
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{M_WAIT, MLEN, Mbuf, MbufBox, mclgetl};
use crate::net::proto::{
    PRCO_GETOPT, PRCO_SETOPT, PRU_ABORT, PRU_ACCEPT, PRU_ATTACH, PRU_BIND, PRU_CONNECT, PRU_DETACH,
    PRU_DISCONNECT, PRU_LISTEN, PRU_PEERADDR, PRU_RCVD, PRU_RCVOOB, PRU_SEND, PRU_SENDOOB,
    PRU_SENSE, PRU_SHUTDOWN, PRU_SOCKADDR, Usrreqs,
};
use crate::net::sockbuf::{
    SB_MAX_VAR, SS_CANTSENDMORE, SS_CONNECTOUT, SS_ISCONNECTED, SS_ISCONNECTING, SS_NOFDREF,
    SS_PRIV, SS_RCVATMARK, sbchecklowmem, sbcheckreserve,
};
use crate::net::socket::{
    InetLocked, SoLocked, Socket, socantsendmore, soisconnecting, soisdisconnected,
    soisdisconnecting, soreserve,
};
use crate::netinet::in_::{in_broadcast, in_control, in_nam2sin};
use crate::netinet::in_pcb::{
    INP_IPV6, Inpcb, Inpcbtable, in_pcballoc, in_pcbbind, in_pcbconnect, in_pcbdetach,
    in_pcbdisconnect, in_pcblookup, in_pcblookup_listen, in_pcbsolock, in_pcbunset_faddr,
    in_setpeeraddr, in_setsockaddr, in_sysctl_ports, sotoinpcb_locked,
};
use crate::netinet::ip_output::ip_ctloutput;
use crate::netinet::tcp_debug::{TA_USER, tcp_trace};
use crate::netinet::tcp_fsm::{
    TCPS_CLOSE_WAIT, TCPS_CLOSED, TCPS_ESTABLISHED, TCPS_FIN_WAIT_1, TCPS_FIN_WAIT_2,
    TCPS_LAST_ACK, TCPS_LISTEN, TCPS_SYN_RECEIVED, TCPS_SYN_SENT, tcps_haveestablished,
};
use crate::netinet::tcp_input::TCP_RST_PPSLIM;
use crate::netinet::tcp_output::tcp_output;
use crate::netinet::tcp_seq::tcp_sendseqinit;
use crate::netinet::tcp_subr::{
    Closed, SACKHL_POOL, TCP_ACK_ON_PUSH, TCP_DO_ECN, TCP_DO_RFC1323, TCP_DO_RFC3390, TCP_DO_SACK,
    TCP_DO_TSO, TCP_MSSDFLT, TCP_REASS_LIMIT, TCPQE_POOL, tcp_close, tcp_drop, tcp_free,
    tcp_newtcpcb, tcp_rscale, tcp_sackhole_limit, tcp_set_iss_tsm, tcp_template,
};
use crate::netinet::tcp_syncache::{TCP_SYN_BUCKET_LIMIT, TCP_SYN_CACHE_LIMIT, syn_cache_lock};
use crate::netinet::tcp_timer::{
    TCP_ALWAYS_KEEPALIVE, TCP_KEEPIDLE, TCP_KEEPIDLE_SEC, TCP_KEEPINIT, TCP_KEEPINIT_SEC,
    TCP_KEEPINTVL, TCP_KEEPINTVL_SEC, TCP_LINGERTIME, TCPT_2MSL, TCPT_KEEP, TCPTV_KEEPCNT,
    TCPTV_KEEPIDLE, TCPTV_KEEPINIT, TCPTV_KEEPINTVL, tcp_timer_arm,
};
use crate::netinet::tcp_var::{
    TCP_RTT_BASE_SHIFT, TCP_RTT_SHIFT, TCP_RTTVAR_SHIFT, TCPCOUNTERS, TCPOOB_HADDATA,
    TCPOOB_HAVEDATA, TCPS_NCOUNTERS, TF_ECN_PERMIT, TF_NODELAY, TF_NOPUSH, TF_RCVD_SCALE,
    TF_RCVD_TSTMP, TF_REQ_SCALE, TF_REQ_TSTMP, TF_SACK_PERMIT, TF_SIGNATURE, TcpState,
    TcpstatCounters, Tp, intotcpcb, intotcpcb_parts, tcp_now, tcp_time, tcpstat_inc,
};
use crate::netinet6::in6::{in6_control, in6_nam2sin6};
use crate::netinet6::in6_pcb::{
    in6_pcblookup, in6_pcblookup_listen, in6_setpeeraddr, in6_setsockaddr,
};
use crate::netinet6::in6_src::in6_embedscope;
use crate::netinet6::ip6_output::ip6_ctloutput;
use crate::proc::{ProcGlue, Thread, suser};
use crate::sysctl::{SYSCTL_LOCK, SysctlReq, bounded_arr, int, int_bounded, rdstruct};

/// `TCP_SENDSPACE`.
const TCP_SENDSPACE: u32 = 1024 * 16;
/// `TCP_RECVSPACE`.
const TCP_RECVSPACE: u32 = 1024 * 16;

/// What a function that may close the control block answers (the C's `tp` or NULL): the
/// block, still open, or the token of its close, which the request that owns the socket
/// lock's parts frees ([`tcp_free`]).
pub enum TpOrClosed<'a, 'w, G: ProcGlue> {
    /// The control block, still attached.
    Open(Tp<'a, 'w, G>),
    /// The block closed (`tcp_close`'s NULL).
    Closed(Closed),
}

impl<G: ProcGlue> TpOrClosed<'_, '_, G> {
    /// The open block's members, for the trace after the request (`None` once closed).
    fn st(&self) -> Option<&TcpState> {
        match self {
            TpOrClosed::Open(tp) => Some(&*tp.st),
            TpOrClosed::Closed(_) => None,
        }
    }
}

/// `tcp_usrreqs` and `tcp6_usrreqs`: TCP's user requests, the two tables differing in
/// `pru_control` only.
pub struct TcpUsrreqs<G: ProcGlue> {
    /// The IPv6 table (`in6_control`).
    inet6: bool,
    /// The glue.
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> TcpUsrreqs<G> {
    /// `tcp_usrreqs`.
    pub const INET: Self = Self {
        inet6: false,
        _g: PhantomData,
    };
    /// `tcp6_usrreqs`.
    pub const INET6: Self = Self {
        inet6: true,
        _g: PhantomData,
    };
}

/// \[I\] `tcp_sendspace`.
#[allow(non_upper_case_globals)] // TCP_SENDSPACE is a constant of this file
pub static tcp_sendspace: u32 = TCP_SENDSPACE;
/// \[I\] `tcp_recvspace`.
#[allow(non_upper_case_globals)] // TCP_RECVSPACE is a constant of this file
pub static tcp_recvspace: u32 = TCP_RECVSPACE;
/// \[I\] `tcp_autorcvbuf_inc`.
pub static TCP_AUTORCVBUF_INC: u32 = 16 * 1024;

/// The identity of `tcp_send` for `sosplice` (`pru_send != pru_send`): both tables answer
/// this static's address.
static TCP_SEND_ID: u8 = 0;

/// `tcbtable`: the glue's IPv4 TCP control blocks (`TcpTables`; pf's socket lookup and
/// `kern.file`'s walk read it).
pub fn tcbtable<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().tcp.tcbtable
}

/// `tcb6table`: the glue's IPv6 TCP control blocks, as [`tcbtable`].
pub fn tcb6table<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().tcp.tcb6table
}

/// `tcpctl_vars[]`.
static TCPCTL_VARS: [SysctlBoundedArgs; 14] = [
    SysctlBoundedArgs::new(
        TCPCTL_KEEPINITTIME,
        &TCP_KEEPINIT_SEC,
        1,
        3 * TCPTV_KEEPINIT / tcp_time(1),
    ),
    SysctlBoundedArgs::new(
        TCPCTL_KEEPIDLE,
        &TCP_KEEPIDLE_SEC,
        1,
        5 * TCPTV_KEEPIDLE / tcp_time(1),
    ),
    SysctlBoundedArgs::new(
        TCPCTL_KEEPINTVL,
        &TCP_KEEPINTVL_SEC,
        1,
        3 * TCPTV_KEEPINTVL / tcp_time(1),
    ),
    SysctlBoundedArgs::new(TCPCTL_RFC1323, &TCP_DO_RFC1323, 0, 1),
    SysctlBoundedArgs::new(TCPCTL_SACK, &TCP_DO_SACK, 0, 1),
    SysctlBoundedArgs::new(TCPCTL_MSSDFLT, &TCP_MSSDFLT, TCP_MSS, 65535),
    SysctlBoundedArgs::new(TCPCTL_RSTPPSLIMIT, &TCP_RST_PPSLIM, 1, 1000 * 1000),
    SysctlBoundedArgs::new(TCPCTL_ACK_ON_PUSH, &TCP_ACK_ON_PUSH, 0, 1),
    // TCP_ECN
    SysctlBoundedArgs::new(TCPCTL_ECN, &TCP_DO_ECN, 0, 1),
    SysctlBoundedArgs::new(TCPCTL_SYN_CACHE_LIMIT, &TCP_SYN_CACHE_LIMIT, 1, 1000 * 1000),
    SysctlBoundedArgs::new(TCPCTL_SYN_BUCKET_LIMIT, &TCP_SYN_BUCKET_LIMIT, 1, i32::MAX),
    SysctlBoundedArgs::new(TCPCTL_RFC3390, &TCP_DO_RFC3390, 0, 2),
    SysctlBoundedArgs::new(TCPCTL_ALWAYS_KEEPALIVE, &TCP_ALWAYS_KEEPALIVE, 0, 1),
    SysctlBoundedArgs::new(TCPCTL_TSO, &TCP_DO_TSO, 0, 1),
];

/// The socket lock of a request opened in its parts (the C's `soassertlocked`): a kernel TCP
/// socket is always held by a witness under a proof of the net lock.
fn inet<'w>(w: &'w mut SoLocked<'_>) -> InetLocked<'w> {
    match w.inet() {
        Some(il) => il,
        None => kpanic(format_args!(
            "tcp: the socket lock is not held as a kernel protocol's"
        )),
    }
}

/// `tcp_sogetpcb`: when a TCP is attached to a socket, then there will be a (struct inpcb)
/// pointed at by the socket, and this structure will point at a subsidiary (struct tcpcb):
/// both borrowed through the socket lock's parts (`il`) for as long as the answer lives, the
/// pcb from the socket's locked slot (`sotoinpcb_locked`, part `PCB`: the C's load of
/// `so_pcb`, no count taken) and the control block from the pcb's (`intotcpcb`). Without
/// either, the socket's pending error or `EINVAL`.
fn tcp_sogetpcb<'a, 'w, G: ProcGlue>(
    so: &'a Socket<G>,
    il: &'a mut InetLocked<'w>,
) -> Result<Tp<'a, 'w, G>, Errno> {
    let InetLocked {
        pcb, tcp, syn, net, ..
    } = il;
    let pcb: &'a WitnessPart<'w> = pcb;
    sotoinpcb_locked(so, pcb)
        .and_then(|inp| intotcpcb_parts(inp, pcb, tcp, syn, net))
        .ok_or_else(|| so.error().unwrap_or(Errno::EINVAL))
}

/// `tcp_free` of the control block a request closed ([`Closed`]: `tcp_close` ran, and its
/// `in_pcbdetach`): the pcb taken out of the socket's locked slot, whose count becomes this
/// frame's (the request borrowed the pcb from the slot), the block freed through it, then
/// the count dropped (the C's `in_pcbunref`, `in_pcbdetach`'s last step).
fn tcp_free_request<G: ProcGlue>(so: &Socket<G>, pcb: &mut WitnessPart<'_>, closed: Closed) {
    let slot = so.set_pcb_locked(pcb, None);
    match slot.as_ref().and_then(|s| s.downcast_ref::<Inpcb<G>>()) {
        Some(inp) => tcp_free(inp, pcb, closed),
        None => kpanic(format_args!(
            "tcp_free: socket {so:p}: no pcb in its locked slot"
        )),
    }
}

/// `otp` and `ostate` of a request on a socket with `SO_DEBUG`: the control block's address
/// and its state before the request.
fn trace_start<G: ProcGlue>(tp: &Tp<'_, '_, G>) -> Option<(usize, i32)> {
    tp.so.has_options(SO_DEBUG).then(|| {
        (
            core::ptr::from_ref::<crate::netinet::tcp_var::Tcpcb<G>>(&tp.cb).addr(),
            tp.st.t_state,
        )
    })
}

/// The `tcp_trace(TA_USER, ostate, tp, otp, NULL, req, 0)` at the end of a request, `st` the
/// block's members after it (`None` once closed).
fn trace_end(start: Option<(usize, i32)>, st: Option<&TcpState>, req: i32) {
    if let Some((otp, ostate)) = start {
        tcp_trace(TA_USER, ostate, st, otp, None, req, 0);
    }
}

/// The `tcp_trace(TA_USER, tp->t_state, tp, tp, NULL, req, 0)` of the requests that change no
/// state (`accept`, `sense`, `sockaddr`, `peeraddr`, `rcvoob`), on a socket with `SO_DEBUG`.
fn trace_now<G: ProcGlue>(tp: &Tp<'_, '_, G>, req: i32) {
    trace_end(trace_start(tp), Some(tp.st), req);
}

/// `curproc`, which the socket requests run as (`panic(9)` without one: a request reaches a
/// protocol from a system call).
fn with_curproc<G: ProcGlue, R>(func: &str, f: impl FnOnce(&Thread<G>) -> R) -> R {
    G::with_curproc(|p| match p {
        Some(p) => f(p),
        None => kpanic(format_args!("{func}: no curproc")),
    })
}

/// `addr` written into the name mbuf `nam` (`m_len` its size).
fn set_nam<T: AbiPod>(nam: &mut Mbuf, addr: &T) {
    let bytes = bytes_of(core::slice::from_ref(addr));
    nam.m_len().set(bytes.len() as u32);
    match nam.data_mut() {
        Some(d) => d.copy_from_slice(bytes),
        None => kpanic(format_args!("tcp: a read-only name mbuf")),
    }
}

/// `in_setsockaddr(inp, nam)`: the local address of `inp`, of its family, into `nam`.
fn tcp_setsockaddr<G: ProcGlue>(inp: &Inpcb<G>, nam: &mut Mbuf) {
    if inp.has_flags(INP_IPV6) {
        set_nam(nam, &in6_setsockaddr(inp));
    } else {
        set_nam(nam, &in_setsockaddr(inp));
    }
}

/// `in_setpeeraddr(inp, nam)`: the foreign address of `inp`, of its family, into `nam`.
fn tcp_setpeeraddr<G: ProcGlue>(inp: &Inpcb<G>, nam: &mut Mbuf) {
    if inp.has_flags(INP_IPV6) {
        set_nam(nam, &in6_setpeeraddr(inp));
    } else {
        set_nam(nam, &in_setpeeraddr(inp));
    }
}

/// `tcp_fill_info`: export internal TCP state information via a struct tcp_info without
/// leaking any sensitive information. Sequence numbers are reported relative to the initial
/// sequence number. `m` receives the structure (a cluster first when it does not fit an
/// mbuf, `M_WAITOK`; `ENOMEM` without one); the members from `tcpi_snd_wnd` on only for a
/// privileged caller.
pub fn tcp_fill_info<G: ProcGlue>(tp: &Tp<'_, '_, G>, m: &mut Mbuf) -> Result<(), Errno> {
    let so = tp.so;
    let st = &*tp.st;
    // msec => usec
    let t: u32 = 1000;

    if size_of::<TcpInfo>() > MLEN && !mclgetl(m, M_WAIT, size_of::<TcpInfo>() as u32) {
        return Err(Errno::ENOMEM);
    }
    let mut ti = TcpInfo::default();
    let now = tcp_now();
    // (now - since) * t in uint64_t, stored in a u_int32_t member.
    let ago = |since: u64| now.wrapping_sub(since).wrapping_mul(u64::from(t)) as u32;

    ti.tcpi_state = st.t_state as u8;
    if st.t_flags & TF_REQ_TSTMP != 0 && st.t_flags & TF_RCVD_TSTMP != 0 {
        ti.tcpi_options |= TCPI_OPT_TIMESTAMPS;
    }
    if st.has_flags(TF_SACK_PERMIT) {
        ti.tcpi_options |= TCPI_OPT_SACK;
    }
    if st.t_flags & TF_REQ_SCALE != 0 && st.t_flags & TF_RCVD_SCALE != 0 {
        ti.tcpi_options |= TCPI_OPT_WSCALE;
        ti.tcpi_snd_wscale = st.snd_scale;
        ti.tcpi_rcv_wscale = st.rcv_scale;
    }
    // TCP_ECN
    if st.has_flags(TF_ECN_PERMIT) {
        ti.tcpi_options |= TCPI_OPT_ECN;
    }

    ti.tcpi_rto = (st.t_rxtcur as u32).wrapping_mul(t);
    ti.tcpi_snd_mss = u32::from(st.t_maxseg);
    ti.tcpi_rcv_mss = u32::from(st.t_peermss);

    ti.tcpi_last_data_sent = ago(st.t_sndtime);
    ti.tcpi_last_ack_sent = ago(st.t_sndacktime);
    ti.tcpi_last_data_recv = ago(st.t_rcvtime);
    ti.tcpi_last_ack_recv = ago(st.t_rcvacktime);

    // ((uint64_t)t_srtt * t) >> shift: the int widened with its sign, as the C's cast.
    ti.tcpi_rtt = ((st.t_srtt as u64).wrapping_mul(u64::from(t))
        >> (TCP_RTT_SHIFT + TCP_RTT_BASE_SHIFT)) as u32;
    ti.tcpi_rttvar = ((st.t_rttvar as u64).wrapping_mul(u64::from(t))
        >> (TCP_RTTVAR_SHIFT + TCP_RTT_BASE_SHIFT)) as u32;
    ti.tcpi_snd_ssthresh = st.snd_ssthresh as u32;
    ti.tcpi_snd_cwnd = st.snd_cwnd as u32;

    ti.tcpi_rcv_space = st.rcv_wnd as u32;

    // Provide only minimal information for unprivileged processes.
    if with_curproc::<G, _>("tcp_fill_info", |p| suser(p).is_ok()) {
        // FreeBSD-specific extension fields for tcp_info.
        ti.tcpi_snd_wnd = st.snd_wnd as u32;
        ti.tcpi_snd_nxt = st.snd_nxt.wrapping_sub(st.iss);
        ti.tcpi_rcv_nxt = st.rcv_nxt.wrapping_sub(st.irs);
        // missing tcpi_toe_tid
        ti.tcpi_snd_rexmitpack = st.t_sndrexmitpack;
        ti.tcpi_rcv_ooopack = st.t_rcvoopack;
        ti.tcpi_snd_zerowin = st.t_sndzerowin;

        // OpenBSD extensions
        ti.tcpi_rttmin = st.t_rttmin.wrapping_mul(t);
        ti.tcpi_max_sndwnd = st.max_sndwnd as u32;
        ti.tcpi_rcv_adv = st.rcv_adv.wrapping_sub(st.irs);
        ti.tcpi_rcv_up = st.rcv_up.wrapping_sub(st.irs);
        ti.tcpi_snd_una = st.snd_una.wrapping_sub(st.iss);
        ti.tcpi_snd_up = st.snd_up.wrapping_sub(st.iss);
        ti.tcpi_snd_wl1 = st.snd_wl1.wrapping_sub(st.iss);
        ti.tcpi_snd_wl2 = st.snd_wl2.wrapping_sub(st.iss);
        ti.tcpi_snd_max = st.snd_max.wrapping_sub(st.iss);

        ti.tcpi_ts_recent = st.ts_recent; // XXX value from the wire
        ti.tcpi_ts_recent_age = ago(st.ts_recent_age);
        ti.tcpi_rfbuf_cnt = st.rfbuf_cnt;
        ti.tcpi_rfbuf_ts = ago(st.rfbuf_ts);

        let g = so.so_rcv.lock();
        ti.tcpi_so_rcv_sb_cc = so.so_rcv.cc() as u32;
        ti.tcpi_so_rcv_sb_hiwat = so.so_rcv.hiwat() as u32;
        ti.tcpi_so_rcv_sb_lowat = so.so_rcv.lowat() as u32;
        ti.tcpi_so_rcv_sb_wat = so.so_rcv.wat() as u32;
        drop(g);
        let g = so.so_snd.lock();
        ti.tcpi_so_snd_sb_cc = so.so_snd.cc() as u32;
        ti.tcpi_so_snd_sb_hiwat = so.so_snd.hiwat() as u32;
        ti.tcpi_so_snd_sb_lowat = so.so_snd.lowat() as u32;
        ti.tcpi_so_snd_sb_wat = so.so_snd.wat() as u32;
        drop(g);
    }

    set_nam(m, &ti);
    Ok(())
}

/// The `int` an option mbuf carries (`*mtod(m, int *)`); `None` for a missing or short one
/// (the C's `m == NULL || m->m_len < sizeof(int)`).
fn opt_int(m: Option<&Mbuf>) -> Option<i32> {
    let d = m?.data();
    let b: [u8; size_of::<i32>()] = d.get(..size_of::<i32>())?.try_into().ok()?;
    Some(i32::from_ne_bytes(b))
}

/// `m->m_len = sizeof(int); *mtod(m, int *) = v` of a `PRCO_GETOPT` answer.
fn opt_set_int(m: Option<&mut Mbuf>, v: i32) {
    match m {
        Some(m) => set_nam(m, &v),
        None => kpanic(format_args!("tcp_ctloutput: no mbuf for PRCO_GETOPT")),
    }
}

/// `tcp_ctloutput`: the `IPPROTO_TCP` socket options of `so`, locked by `w`; the other levels
/// go to IP. `m` holds a `PRCO_SETOPT`'s value and receives a `PRCO_GETOPT`'s answer.
pub fn tcp_ctloutput<G: ProcGlue>(
    op: i32,
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    let mut il = inet(w);
    if level != IPPROTO_TCP {
        let Some(inp) = sotoinpcb_locked(so, &il.pcb) else {
            return Err(Errno::ECONNRESET);
        };
        if inp.has_flags(INP_IPV6) {
            return ip6_ctloutput(op, inp, level, optname, m, &il.net);
        }
        let mut options = inp.options();
        return ip_ctloutput(op, inp, &mut options, level, optname, m, &il.net);
    }
    // No pcb: the C's ECONNRESET. A pcb without a control block: the C dereferences it
    // unchecked (the Deviations).
    let Ok(mut tp) = tcp_sogetpcb(so, &mut il) else {
        return Err(Errno::ECONNRESET);
    };

    match op {
        PRCO_SETOPT => {
            let v = opt_int(m.as_deref());
            let st = &mut *tp.st;
            match optname {
                TCP_NODELAY => match v {
                    None => Err(Errno::EINVAL),
                    Some(0) => {
                        st.clear_flags(TF_NODELAY);
                        Ok(())
                    }
                    Some(_) => {
                        st.set_flags(TF_NODELAY);
                        Ok(())
                    }
                },

                TCP_NOPUSH => match v {
                    None => Err(Errno::EINVAL),
                    Some(0) => {
                        if st.has_flags(TF_NOPUSH) {
                            st.clear_flags(TF_NOPUSH);
                            if tcps_haveestablished(st.t_state) {
                                return tcp_output(&mut tp);
                            }
                        }
                        Ok(())
                    }
                    Some(_) => {
                        st.set_flags(TF_NOPUSH);
                        Ok(())
                    }
                },

                TCP_MAXSEG => match v {
                    Some(i) if i > 0 && i <= i32::from(st.t_maxseg) => {
                        st.t_maxseg = i as u16;
                        Ok(())
                    }
                    _ => Err(Errno::EINVAL),
                },

                TCP_SACK_ENABLE => {
                    let Some(i) = v else {
                        return Err(Errno::EINVAL);
                    };
                    if tcps_haveestablished(st.t_state) {
                        return Err(Errno::EPERM);
                    }
                    if st.has_flags(TF_SIGNATURE) {
                        return Err(Errno::EPERM);
                    }
                    st.sack_enable = i32::from(i != 0);
                    Ok(())
                }
                // TCP_SIGNATURE
                TCP_MD5SIG => {
                    let Some(i) = v else {
                        return Err(Errno::EINVAL);
                    };
                    if tcps_haveestablished(st.t_state) {
                        return Err(Errno::EPERM);
                    }
                    if i != 0 {
                        st.set_flags(TF_SIGNATURE);
                        st.sack_enable = 0;
                    } else {
                        st.clear_flags(TF_SIGNATURE);
                    }
                    Ok(())
                }
                _ => Err(Errno::ENOPROTOOPT),
            }
        }

        PRCO_GETOPT => {
            let st = &*tp.st;
            match optname {
                TCP_NODELAY => {
                    opt_set_int(m, (st.t_flags & TF_NODELAY) as i32);
                    Ok(())
                }
                TCP_NOPUSH => {
                    opt_set_int(m, (st.t_flags & TF_NOPUSH) as i32);
                    Ok(())
                }
                TCP_MAXSEG => {
                    opt_set_int(m, i32::from(st.t_maxseg));
                    Ok(())
                }
                TCP_SACK_ENABLE => {
                    opt_set_int(m, st.sack_enable);
                    Ok(())
                }
                TCP_INFO => match m {
                    Some(m) => tcp_fill_info(&tp, m),
                    None => kpanic(format_args!("tcp_ctloutput: no mbuf for TCP_INFO")),
                },
                // TCP_SIGNATURE
                TCP_MD5SIG => {
                    opt_set_int(m, (st.t_flags & TF_SIGNATURE) as i32);
                    Ok(())
                }
                _ => Err(Errno::ENOPROTOOPT),
            }
        }

        _ => Ok(()),
    }
}

/// `tcp_attach`: attach TCP protocol to socket, allocating internet protocol control block,
/// tcp control block, buffer space, and entering LISTEN state to accept connections.
pub fn tcp_attach<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    _proto: i32,
    wait: i32,
) -> Result<(), Errno> {
    if so.has_pcb() {
        return Err(Errno::EISCONN);
    }
    if so.so_snd.hiwat() == 0
        || so.so_rcv.hiwat() == 0
        || sbcheckreserve(so.so_snd.wat(), u64::from(tcp_sendspace)).is_err()
        || sbcheckreserve(so.so_rcv.wat(), u64::from(tcp_recvspace)).is_err()
    {
        soreserve(so, u64::from(tcp_sendspace), u64::from(tcp_recvspace))?;
    }

    let t = &net::<G>().tcp;
    let table = if so.dom_family() == i32::from(PF_INET6) {
        &t.tcb6table
    } else {
        &t.tcbtable
    };
    let inp = in_pcballoc(so, table, wait)?;
    let mut il = inet(w);
    let Some(tp) = tcp_newtcpcb(&inp, &mut il, wait) else {
        // The socket's typed slot and the pcb count each other until in_pcbdetach: detach
        // it, keeping the socket (in_pcbdetach's sofree is refused while it has a file).
        let nofd = so.has_state(SS_NOFDREF); // XXX

        so.st_clear(SS_NOFDREF); // don't free the socket yet
        in_pcbdetach(inp, &il.net);
        if nofd {
            so.st_set(SS_NOFDREF);
        }
        return Err(Errno::ENOBUFS);
    };
    tp.st.t_state = TCPS_CLOSED;
    tp.st.pf = if inp.has_flags(INP_IPV6) {
        i32::from(PF_INET6)
    } else {
        i32::from(PF_INET)
    };
    if so.has_options(SO_LINGER) && so.linger() == 0 {
        so.set_linger(TCP_LINGERTIME);
    }

    if so.has_options(SO_DEBUG) {
        let otp = core::ptr::from_ref::<crate::netinet::tcp_var::Tcpcb<G>>(&tp.cb).addr();
        tcp_trace(TA_USER, TCPS_CLOSED, Some(tp.st), otp, None, PRU_ATTACH, 0);
    }
    // so->so_pcb for the requests (`tcp_sogetpcb`): in_pcballoc's handle, kept by the
    // socket's locked slot until tcp_free.
    drop(so.set_pcb_locked(&mut il.pcb, Some(PoolArc::erase(inp))));
    Ok(())
}

/// `tcp_detach`: detach the TCP protocol from the socket. If the protocol state is
/// non-embryonic, then can't do this directly: have to initiate a PRU_DISCONNECT, which may
/// finish later; embryonic TCB's can just be discarded here.
pub fn tcp_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    let end = tcp_dodisconnect(tp);

    trace_end(start, end.st(), PRU_DETACH);
    if let TpOrClosed::Closed(c) = end {
        tcp_free_request(so, &mut il.pcb, c);
    }
    Ok(())
}

/// `tcp_bind`: give the socket an address.
pub fn tcp_bind<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    let error = in_pcbbind(tp.inp, Some(nam), p, tp.net);

    trace_end(start, Some(tp.st), PRU_BIND);
    error
}

/// `tcp_listen`: prepare to accept connections.
pub fn tcp_listen<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    let error = if tp.inp.inp_lport.get() == 0 {
        with_curproc::<G, _>("tcp_listen", |p| in_pcbbind(tp.inp, None, p, tp.net))
    } else {
        Ok(())
    };
    // If the in_pcbbind() above is called, the tp->pf should still be whatever it was
    // before.
    if error.is_ok() {
        tp.st.t_state = TCPS_LISTEN;
    }

    trace_end(start, Some(tp.st), PRU_LISTEN);
    error
}

/// `tcp_connect`: initiate connection to peer. Create a template for use in transmissions
/// on this connection. Enter SYN_SENT state, and mark socket as connecting. Start keep-alive
/// timer, and seed output sequence space. Send initial segment on connection.
pub fn tcp_connect<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
) -> Result<(), Errno> {
    let mut il = inet(w);
    let mut tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    let error = tcp_connect_tp(&mut tp, nam);

    trace_end(start, Some(tp.st), PRU_CONNECT);
    if error.is_err() && !so.has_pcb() {
        // tcp_connect_tp's in_pcbdetach (a socket without a file): the locked slot let go
        // too, now that nothing borrows the pcb from it.
        drop(so.set_pcb_locked(&mut il.pcb, None));
    }
    error
}

/// `tcp_connect`'s work between its traces.
fn tcp_connect_tp<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, nam: &Mbuf) -> Result<(), Errno> {
    let (inp, so) = (tp.inp, tp.so);
    if inp.has_flags(INP_IPV6) {
        let sin6 = in6_nam2sin6(nam)?;
        if in6_is_addr_unspecified(&sin6.sin6_addr) || in6_is_addr_multicast(&sin6.sin6_addr) {
            return Err(Errno::EINVAL);
        }
    } else {
        let sin = in_nam2sin(nam)?;
        let a = sin.sin_addr.s_addr;
        if a == INADDR_ANY
            || a == INADDR_BROADCAST
            || in_multicast(a)
            || in_broadcast(sin.sin_addr, inp.inp_rtableid.get(), tp.net)
        {
            return Err(Errno::EINVAL);
        }
    }
    with_curproc::<G, _>("tcp_connect", |p| in_pcbconnect(inp, nam, p, tp.net))?;

    let template = tcp_template(tp);
    tp.st.t_template = template;
    if tp.st.t_template.is_none() {
        in_pcbunset_faddr(inp);
        if in_pcbdisconnect(inp) {
            // in_pcbdisconnect's in_pcbdetach of a socket without a file.
            match PoolArc::from_ref(inp) {
                Some(h) => in_pcbdetach(h, tp.net),
                None => kpanic(format_args!("tcp_connect: inpcb {inp:p}: not a pool item")),
            }
        }
        return Err(Errno::ENOBUFS);
    }

    so.st_set(SS_CONNECTOUT);

    // Compute window scaling to request.
    tcp_rscale(tp.st, SB_MAX_VAR.load(Ordering::Relaxed));

    soisconnecting(so);
    tcpstat_inc(TcpstatCounters::TcpsConnattempt);
    tp.st.t_state = TCPS_SYN_SENT;
    tcp_timer_arm(tp, TCPT_KEEP, TCP_KEEPINIT.load(Ordering::Relaxed) as u64);
    tcp_set_iss_tsm(tp);
    tcp_sendseqinit(tp.st);
    tp.st.snd_last = tp.st.snd_una;
    tcp_output(tp)
}

/// `tcp_accept`: accept a connection. Essentially all the work is done at higher levels;
/// just return the address of the peer, storing through addr.
pub fn tcp_accept<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &mut Mbuf,
) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;

    tcp_setpeeraddr(tp.inp, nam);

    trace_now(&tp, PRU_ACCEPT);
    Ok(())
}

/// `tcp_disconnect`: initiate disconnect from peer. If connection never passed embryonic
/// stage, just drop; else if don't need to let data drain, then can just drop anyways, else
/// have to begin TCP shutdown process: mark socket disconnecting, drain unread data, state
/// switch to reflect user close, and send segment (e.g. FIN) to peer. Socket will be really
/// disconnected when peer sends FIN and acks ours.
///
/// SHOULD IMPLEMENT LATER PRU_CONNECT VIA REALLOC TCPCB.
pub fn tcp_disconnect<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    let end = tcp_dodisconnect(tp);

    trace_end(start, end.st(), PRU_DISCONNECT);
    if let TpOrClosed::Closed(c) = end {
        tcp_free_request(so, &mut il.pcb, c);
    }
    Ok(())
}

/// `tcp_shutdown`: mark the connection as being incapable of further output.
pub fn tcp_shutdown<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    let (end, error) = if so.so_snd.has_state(SS_CANTSENDMORE) {
        (TpOrClosed::Open(tp), Ok(()))
    } else {
        socantsendmore(so);
        match tcp_usrclosed(tp) {
            TpOrClosed::Open(mut tp) => {
                let error = tcp_output(&mut tp);
                (TpOrClosed::Open(tp), error)
            }
            closed => (closed, Ok(())),
        }
    };

    trace_end(start, end.st(), PRU_SHUTDOWN);
    if let TpOrClosed::Closed(c) = end {
        tcp_free_request(so, &mut il.pcb, c);
    }
    error
}

/// `tcp_rcvd`: after a receive, possibly send window update to peer.
pub fn tcp_rcvd<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) {
    let mut il = inet(w);
    let Ok(mut tp) = tcp_sogetpcb(so, &mut il) else {
        return;
    };
    let start = trace_start(&tp);

    // soreceive() calls this function when a user receives ancillary data on a listening
    // socket. We don't call tcp_output in such a case, since there is no header template
    // for a listening socket and hence the kernel will panic.
    if so.state() & (SS_ISCONNECTED | SS_ISCONNECTING) != 0 {
        let _ = tcp_output(&mut tp);
    }

    trace_end(start, Some(tp.st), PRU_RCVD);
}

/// `tcp_send`: do a send by putting data in output queue and updating urgent marker if URG
/// set. Possibly send more data. The data and the control are consumed.
pub fn tcp_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    if control.as_ref().is_some_and(|c| c.m_len().get() != 0) {
        // m_freem(control), m_freem(m): both dropped.
        return Err(Errno::EINVAL);
    }
    drop(control);

    let mut il = inet(w);
    let mut tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    if let Some(m) = m {
        so.so_snd.lock().sbappendstream(m);
    }

    let error = tcp_output(&mut tp);

    trace_end(start, Some(tp.st), PRU_SEND);
    error
}

/// `tcp_abort`: abort the TCP.
pub fn tcp_abort<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) {
    let mut il = inet(w);
    let Ok(tp) = tcp_sogetpcb(so, &mut il) else {
        return;
    };
    let start = trace_start(&tp);

    let c = tcp_drop(tp, Some(Errno::ECONNABORTED));

    trace_end(start, None, PRU_ABORT);
    tcp_free_request(so, &mut il.pcb, c);
}

/// `tcp_sense`: the send buffer's size as the optimal block size.
pub fn tcp_sense<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    ub: &mut Stat,
) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;

    let g = so.so_snd.lock();
    ub.st_blksize = so.so_snd.hiwat() as _;
    drop(g);

    trace_now(&tp, PRU_SENSE);
    Ok(())
}

/// `tcp_rcvoob`: the out-of-band byte into `m` (`MSG_PEEK` leaves it unread).
pub fn tcp_rcvoob<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: &mut Mbuf,
    flags: i32,
) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;

    let st = &mut *tp.st;
    let error = if (so.oobmark() == 0 && !so.so_rcv.has_state(SS_RCVATMARK))
        || so.has_options(SO_OOBINLINE)
        || st.t_oobflags & TCPOOB_HADDATA != 0
    {
        Err(Errno::EINVAL)
    } else if st.t_oobflags & TCPOOB_HAVEDATA == 0 {
        Err(Errno::EWOULDBLOCK)
    } else {
        set_nam(m, &st.t_iobc);
        if flags & MSG_PEEK == 0 {
            st.t_oobflags ^= TCPOOB_HAVEDATA | TCPOOB_HADDATA;
        }
        Ok(())
    };
    trace_now(&tp, PRU_RCVOOB);
    error
}

/// `tcp_sendoob`: urgent data: appended to the send buffer, the urgent pointer past it, and
/// sent at once. The data and the control are consumed.
pub fn tcp_sendoob<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    if control.as_ref().is_some_and(|c| c.m_len().get() != 0) {
        return Err(Errno::EINVAL);
    }
    drop(control);

    let mut il = inet(w);
    let mut tp = tcp_sogetpcb(so, &mut il)?;
    let start = trace_start(&tp);

    let error = if so.so_snd.sbspace() < -512 {
        drop(m);
        Err(Errno::ENOBUFS)
    } else {
        // According to RFC961 (Assigned Protocols), the urgent pointer points to the last
        // octet of urgent data. We continue, however, to consider it to indicate the first
        // octet of data past the urgent section. Otherwise, snd_up should be one lower.
        if let Some(m) = m {
            so.so_snd.lock().sbappendstream(m);
        }
        tp.st.snd_up = tp.st.snd_una.wrapping_add(so.so_snd.cc() as u32);
        tp.st.t_force = true;
        let error = tcp_output(&mut tp);
        tp.st.t_force = false;
        error
    };

    trace_end(start, Some(tp.st), PRU_SENDOOB);
    error
}

/// `tcp_sockaddr`: the local address into `nam`.
pub fn tcp_sockaddr<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &mut Mbuf,
) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;

    tcp_setsockaddr(tp.inp, nam);

    trace_now(&tp, PRU_SOCKADDR);
    Ok(())
}

/// `tcp_peeraddr`: the foreign address into `nam`.
pub fn tcp_peeraddr<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &mut Mbuf,
) -> Result<(), Errno> {
    let mut il = inet(w);
    let tp = tcp_sogetpcb(so, &mut il)?;

    tcp_setpeeraddr(tp.inp, nam);

    trace_now(&tp, PRU_PEERADDR);
    Ok(())
}

/// `tcp_dodisconnect`: initiate (or continue) disconnect. If embryonic state, just send
/// reset (once). If in ``let data drain'' option and linger null, just drop. Otherwise
/// (hard), mark socket disconnecting and drop current input data; switch states based on
/// user close, and send segment to peer (with FIN).
pub fn tcp_dodisconnect<'a, 'w, G: ProcGlue>(tp: Tp<'a, 'w, G>) -> TpOrClosed<'a, 'w, G> {
    let so = tp.so;

    if !tcps_haveestablished(tp.st.t_state) {
        TpOrClosed::Closed(tcp_close(tp))
    } else if so.has_options(SO_LINGER) && so.linger() == 0 {
        TpOrClosed::Closed(tcp_drop(tp, None))
    } else {
        soisdisconnecting(so);
        so.so_rcv.lock().sbflush();
        match tcp_usrclosed(tp) {
            TpOrClosed::Open(mut tp) => {
                let _ = tcp_output(&mut tp);
                TpOrClosed::Open(tp)
            }
            closed => closed,
        }
    }
}

/// `tcp_usrclosed`: user issued close, and wish to trail through shutdown states: if never
/// received SYN, just forget it. If got a SYN from peer, but haven't sent FIN, then go to
/// FIN_WAIT_1 state to send peer a FIN. If already got a FIN from peer, then almost done; go
/// to LAST_ACK state. In all other cases, have already sent FIN to peer (e.g. after
/// PRU_SHUTDOWN), and just have to play tedious game waiting for peer to send FIN or not
/// respond to keep-alives, etc. We can let the user exit from the close as soon as the FIN
/// is acked.
pub fn tcp_usrclosed<'a, 'w, G: ProcGlue>(mut tp: Tp<'a, 'w, G>) -> TpOrClosed<'a, 'w, G> {
    match tp.st.t_state {
        TCPS_CLOSED | TCPS_LISTEN | TCPS_SYN_SENT => {
            tp.st.t_state = TCPS_CLOSED;
            return TpOrClosed::Closed(tcp_close(tp));
        }
        TCPS_SYN_RECEIVED | TCPS_ESTABLISHED => tp.st.t_state = TCPS_FIN_WAIT_1,
        TCPS_CLOSE_WAIT => tp.st.t_state = TCPS_LAST_ACK,
        _ => {}
    }
    if tp.st.t_state >= TCPS_FIN_WAIT_2 {
        soisdisconnected(tp.so);
        // If we are in FIN_WAIT_2, we arrived here because the application did a shutdown
        // of the send side. Like the case of a transition from FIN_WAIT_1 to FIN_WAIT_2
        // after a full close, we start a timer to make sure sockets are not left in
        // FIN_WAIT_2 forever.
        if tp.st.t_state == TCPS_FIN_WAIT_2 {
            let maxidle = TCPTV_KEEPCNT.wrapping_mul(TCP_KEEPIDLE.load(Ordering::Relaxed));
            tcp_timer_arm(&mut tp, TCPT_2MSL, maxidle as u64);
        }
    }
    TpOrClosed::Open(tp)
}

/// The looked-up addresses of [`tcp_ident`]: `fin`/`lin`, or the IPv6 ports with `f6`/`l6`
/// (their scopes embedded).
enum IdentAddrs {
    /// `AF_INET`.
    V4(SockaddrIn, SockaddrIn),
    /// `AF_INET6`.
    V6 {
        /// `fin6->sin6_port`.
        fport: u16,
        /// `f6`.
        f6: In6Addr,
        /// `lin6->sin6_port`.
        lport: u16,
        /// `l6`.
        l6: In6Addr,
    },
}

/// The `T` at the start of a `sockaddr_storage`, the storage read whole (the C's cast).
fn storage_as<T: AbiPod>(ss: &hal::sys::socket::SockaddrStorage) -> Option<T> {
    hal::pod::at(bytes_of(core::slice::from_ref(ss)), 0)
}

/// `tcp_ident`: look up a socket for ident or tcpdrop, ... (`TCPCTL_IDENT`: the user ids of
/// the connection's socket when it connected out, -1 otherwise; `TCPCTL_DROP`, `dodrop`: the
/// connection dropped, `ESRCH` for none or a listener).
pub fn tcp_ident<G: ProcGlue>(req: &mut SysctlReq<'_>, dodrop: bool) -> Result<(), Errno> {
    let size = size_of::<TcpIdentMapping>();
    let mut tir: TcpIdentMapping = if dodrop {
        if req.oldp != 0 || *req.oldlenp != 0 {
            return Err(Errno::EINVAL);
        }
        if req.newp == 0 {
            return Err(Errno::EPERM);
        }
        if req.newlen < size {
            return Err(Errno::ENOMEM);
        }
        UserPtr::<TcpIdentMapping>::new(req.newp).copyin()?
    } else {
        if req.oldp == 0 {
            return Err(Errno::EINVAL);
        }
        if *req.oldlenp < size {
            return Err(Errno::ENOMEM);
        }
        if req.newp != 0 || req.newlen != 0 {
            return Err(Errno::EINVAL);
        }
        UserPtr::<TcpIdentMapping>::new(req.oldp).copyin()?
    };

    let shared = NETLOCK.read();
    let Some(net) = NetHeld::of(&shared) else {
        kpanic(format_args!("tcp_ident: the net lock"));
    };

    let addrs = match tir.faddr.ss_family {
        AF_INET6 => {
            if tir.laddr.ss_family != AF_INET6 {
                return Err(Errno::EAFNOSUPPORT);
            }
            let (Some(fin6), Some(lin6)) = (
                storage_as::<SockaddrIn6>(&tir.faddr),
                storage_as::<SockaddrIn6>(&tir.laddr),
            ) else {
                kpanic(format_args!(
                    "tcp_ident: a sockaddr_storage shorter than sockaddr_in6"
                ));
            };
            let mut f6 = In6Addr::default();
            if in6_embedscope(&mut f6, &fin6, 0, 0).is_err() {
                return Err(Errno::EINVAL); // ?
            }
            let mut l6 = In6Addr::default();
            if in6_embedscope(&mut l6, &lin6, 0, 0).is_err() {
                return Err(Errno::EINVAL); // ?
            }
            IdentAddrs::V6 {
                fport: fin6.sin6_port,
                f6,
                lport: lin6.sin6_port,
                l6,
            }
        }
        AF_INET => {
            if tir.laddr.ss_family != AF_INET {
                return Err(Errno::EAFNOSUPPORT);
            }
            let (Some(fin), Some(lin)) = (
                storage_as::<SockaddrIn>(&tir.faddr),
                storage_as::<SockaddrIn>(&tir.laddr),
            ) else {
                kpanic(format_args!(
                    "tcp_ident: a sockaddr_storage shorter than sockaddr_in"
                ));
            };
            IdentAddrs::V4(fin, lin)
        }
        _ => return Err(Errno::EAFNOSUPPORT),
    };

    let t = &crate::net::domain::net::<G>().tcp;
    let mut inp = match addrs {
        IdentAddrs::V6 {
            fport,
            f6,
            lport,
            l6,
        } => in6_pcblookup(&t.tcb6table, &f6, fport, &l6, lport, tir.rdomain),
        IdentAddrs::V4(fin, lin) => in_pcblookup(
            &t.tcbtable,
            fin.sin_addr,
            fin.sin_port,
            lin.sin_addr,
            lin.sin_port,
            tir.rdomain,
        ),
    };

    if dodrop {
        let error = match inp.as_deref() {
            Some(i) => tcp_ident_drop(i, &net),
            None => Err(Errno::ESRCH),
        };
        // in_pcbsounlock (tcp_ident_drop's), NET_UNLOCK_SHARED, in_pcbunref.
        drop(shared);
        drop(inp);
        return error;
    }

    if inp.is_none() {
        tcpstat_inc(TcpstatCounters::TcpsPcbhashmiss);
        inp = match addrs {
            IdentAddrs::V6 { lport, l6, .. } => {
                in6_pcblookup_listen(&t.tcb6table, &l6, lport, None, tir.rdomain)
            }
            IdentAddrs::V4(_, lin) => {
                in_pcblookup_listen(&t.tcbtable, lin.sin_addr, lin.sin_port, None, tir.rdomain)
            }
        };
    }

    let (ruid, euid) = match inp
        .as_deref()
        .and_then(|i| in_pcbsolock(i, &net).map(|w| (i, w)))
    {
        Some((i, w)) => {
            let so = i.socket();
            let ids = if so.has_state(SS_CONNECTOUT) {
                let (euid, ruid, ..) = so.creds();
                (ruid as i32, euid as i32)
            } else {
                (-1, -1)
            };
            // in_pcbsounlock
            drop(w);
            ids
        }
        None => (-1, -1),
    };
    tir.ruid = ruid;
    tir.euid = euid;

    drop(shared);
    drop(inp);

    *req.oldlenp = size;
    UserPtr::<TcpIdentMapping>::new(req.oldp).copyout(&tir)
}

/// `TCPCTL_DROP`'s work on the found pcb `inp`, under the shared net lock (`net`): its socket
/// locked (`in_pcbsolock`), the connection dropped unless the socket is a listener; `ESRCH`
/// for a socket detached, without a control block or listening.
fn tcp_ident_drop<G: ProcGlue>(inp: &Inpcb<G>, net: &NetHeld<'_>) -> Result<(), Errno> {
    let Some(mut so) = in_pcbsolock(inp, net) else {
        return Err(Errno::ESRCH);
    };
    let listening = inp.socket().has_options(SO_ACCEPTCONN);
    let mut il = inet(&mut so);
    match intotcpcb(inp, &mut il) {
        Some(tp) if !listening => {
            let c = tcp_drop(tp, Some(Errno::ECONNABORTED));
            tcp_free(inp, &mut il.pcb, c);
            Ok(())
        }
        _ => Err(Errno::ESRCH),
    }
}

/// `ASSIGN(field)` of `tcp_sysctl_tcpstat` for every member, in the counters' order.
macro_rules! tcpstat_assign {
    ($counters:expr; $($f:ident),* $(,)?) => {{
        let counters: &[u64; TCPS_NCOUNTERS] = $counters;
        let mut tcpstat = Tcpstat::default();
        let mut i = 0;
        $(
            tcpstat.$f = counters[i] as _;
            i += 1;
        )*
        const _: () = assert!([$(stringify!($f)),*].len() == TCPS_NCOUNTERS);
        let _ = i;
        tcpstat
    }};
}

/// `tcp_sysctl_tcpstat`: `net.inet.tcp.stats`, the counters summed over the CPUs and the SYN
/// cache's state (its active set, under its mutex).
fn tcp_sysctl_tcpstat<G: ProcGlue>(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let counters = TCPCOUNTERS.read();
    let mut tcpstat = tcpstat_assign!(&counters;
        tcps_connattempt,
        tcps_accepts,
        tcps_connects,
        tcps_drops,
        tcps_conndrops,
        tcps_closed,
        tcps_segstimed,
        tcps_rttupdated,
        tcps_delack,
        tcps_timeoutdrop,
        tcps_rexmttimeo,
        tcps_persisttimeo,
        tcps_persistdrop,
        tcps_keeptimeo,
        tcps_keepprobe,
        tcps_keepdrops,
        tcps_sndtotal,
        tcps_sndpack,
        tcps_sndbyte,
        tcps_sndrexmitpack,
        tcps_sndrexmitbyte,
        tcps_sndrexmitfast,
        tcps_sndacks,
        tcps_sndprobe,
        tcps_sndurg,
        tcps_sndwinup,
        tcps_sndctrl,
        tcps_rcvtotal,
        tcps_rcvpack,
        tcps_rcvbyte,
        tcps_rcvbadsum,
        tcps_rcvbadoff,
        tcps_rcvmemdrop,
        tcps_rcvnosec,
        tcps_rcvshort,
        tcps_rcvduppack,
        tcps_rcvdupbyte,
        tcps_rcvpartduppack,
        tcps_rcvpartdupbyte,
        tcps_rcvoopack,
        tcps_rcvoobyte,
        tcps_rcvpackafterwin,
        tcps_rcvbyteafterwin,
        tcps_rcvafterclose,
        tcps_rcvwinprobe,
        tcps_rcvdupack,
        tcps_rcvacktoomuch,
        tcps_rcvacktooold,
        tcps_rcvackpack,
        tcps_rcvackbyte,
        tcps_rcvwinupd,
        tcps_pawsdrop,
        tcps_predack,
        tcps_preddat,
        tcps_pcbhashmiss,
        tcps_noport,
        tcps_closing,
        tcps_badsyn,
        tcps_dropsyn,
        tcps_rcvbadsig,
        tcps_rcvgoodsig,
        tcps_inswcsum,
        tcps_outswcsum,
        tcps_ecn_accepts,
        tcps_ecn_rcvece,
        tcps_ecn_rcvcwr,
        tcps_ecn_rcvce,
        tcps_ecn_sndect,
        tcps_ecn_sndece,
        tcps_ecn_sndcwr,
        tcps_cwr_ecn,
        tcps_cwr_frecovery,
        tcps_cwr_timeout,
        tcps_sc_added,
        tcps_sc_completed,
        tcps_sc_timed_out,
        tcps_sc_overflowed,
        tcps_sc_reset,
        tcps_sc_unreach,
        tcps_sc_bucketoverflow,
        tcps_sc_aborted,
        tcps_sc_dupesyn,
        tcps_sc_dropped,
        tcps_sc_collisions,
        tcps_sc_retransmitted,
        tcps_sc_seedrandom,
        tcps_sc_hash_size,
        tcps_sc_entry_count,
        tcps_sc_entry_limit,
        tcps_sc_bucket_maxlen,
        tcps_sc_bucket_limit,
        tcps_sc_uses_left,
        tcps_conndrained,
        tcps_sack_recovery_episode,
        tcps_sack_rexmits,
        tcps_sack_rexmit_bytes,
        tcps_sack_rcv_opts,
        tcps_sack_snd_opts,
        tcps_sack_drop_opts,
        tcps_outswtso,
        tcps_outhwtso,
        tcps_outpkttso,
        tcps_outbadtso,
        tcps_inswlro,
        tcps_inhwlro,
        tcps_inpktlro,
        tcps_inbadlro,
    );

    let g = syn_cache_lock::<G>();
    let set = &g.sets[g.active];
    tcpstat.tcps_sc_hash_size = set.scs_size as u64;
    tcpstat.tcps_sc_entry_count = set.scs_count as u64;
    tcpstat.tcps_sc_entry_limit = TCP_SYN_CACHE_LIMIT.load(Ordering::Relaxed) as u64;
    tcpstat.tcps_sc_bucket_maxlen = 0;
    if let Some(heads) = set.scs_buckethead.as_ref() {
        for head in heads
            .iter()
            .take(usize::try_from(set.scs_size).unwrap_or(0))
        {
            if tcpstat.tcps_sc_bucket_maxlen < u64::from(head.sch_length) {
                tcpstat.tcps_sc_bucket_maxlen = u64::from(head.sch_length);
            }
        }
    }
    tcpstat.tcps_sc_bucket_limit = TCP_SYN_BUCKET_LIMIT.load(Ordering::Relaxed) as u64;
    tcpstat.tcps_sc_uses_left = set.scs_use;
    drop(g);

    rdstruct(req, bytes_of(core::slice::from_ref(&tcpstat)))
}

/// `tcp_sysctl`: sysctl for tcp variables.
pub fn tcp_sysctl<G: ProcGlue>(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // All sysctl names at this level are terminal.
    let [n] = *name else {
        return Err(Errno::ENOTDIR);
    };

    match n {
        TCPCTL_ROOTONLY | TCPCTL_BADDYNAMIC => {
            in_sysctl_ports(req, IPPROTO_TCP as u16, n == TCPCTL_ROOTONLY)
        }
        TCPCTL_IDENT => tcp_ident::<G>(req, false),

        TCPCTL_DROP => tcp_ident::<G>(req, true),

        TCPCTL_REASS_LIMIT | TCPCTL_SACKHOLE_LIMIT => {
            let reass = n == TCPCTL_REASS_LIMIT;
            let var = if reass {
                &TCP_REASS_LIMIT
            } else {
                &tcp_sackhole_limit
            };

            let oval = var.load(Ordering::Relaxed);
            let nval = AtomicI32::new(oval);
            int(req, &nval)?;
            let nval = nval.into_inner();

            if oval != nval {
                let sysctl_lock = SYSCTL_LOCK.write_flags(RW_INTR)?;
                let mut error = Ok(());
                if nval != var.load(Ordering::Relaxed) {
                    // pool_sethardlimit(pool, nval): the int as the C's u_int.
                    error = if reass {
                        TCPQE_POOL.set_hardlimit(nval as u32)
                    } else {
                        SACKHL_POOL.set_hardlimit(nval as u32)
                    };
                    if error.is_ok() {
                        var.store(nval, Ordering::Relaxed);
                    }
                }
                drop(sysctl_lock);
                return error;
            }

            Ok(())
        }
        TCPCTL_STATS => tcp_sysctl_tcpstat::<G>(req),

        TCPCTL_SYN_USE_LIMIT => {
            let t = &net::<G>().tcp;
            let oval = t.tcp_syn_use_limit.load(Ordering::Relaxed);
            let nval = AtomicI32::new(oval);
            int_bounded(req, &nval, 0, i32::MAX)?;
            let nval = nval.into_inner();
            if oval != nval {
                // Global tcp_syn_use_limit is used when reseeding a new cache. Also update
                // the value in active cache.
                let mut g = syn_cache_lock::<G>();
                for set in &mut g.sets {
                    if set.scs_use > i64::from(nval) {
                        set.scs_use = i64::from(nval);
                    }
                }
                t.tcp_syn_use_limit.store(nval, Ordering::Relaxed);
                drop(g);
            }
            Ok(())
        }

        TCPCTL_SYN_HASH_SIZE => {
            let t = &net::<G>().tcp;
            let oval = t.tcp_syn_hash_size.load(Ordering::Relaxed);
            let nval = AtomicI32::new(oval);
            int_bounded(req, &nval, 1, 100_000)?;
            let nval = nval.into_inner();
            if oval != nval {
                // If global hash size has been changed, switch sets as soon as possible.
                // Then the actual hash array will be reallocated.
                let mut g = syn_cache_lock::<G>();
                for set in &mut g.sets {
                    if set.scs_size != nval {
                        set.scs_use = 0;
                    }
                }
                t.tcp_syn_hash_size.store(nval, Ordering::Relaxed);
                drop(g);
            }
            Ok(())
        }

        _ => {
            let error = bounded_arr(&TCPCTL_VARS, name, req);
            match n {
                TCPCTL_KEEPINITTIME => TCP_KEEPINIT.store(
                    TCP_KEEPINIT_SEC.load(Ordering::Relaxed) * tcp_time(1),
                    Ordering::Relaxed,
                ),
                TCPCTL_KEEPIDLE => TCP_KEEPIDLE.store(
                    TCP_KEEPIDLE_SEC.load(Ordering::Relaxed) * tcp_time(1),
                    Ordering::Relaxed,
                ),
                TCPCTL_KEEPINTVL => TCP_KEEPINTVL.store(
                    TCP_KEEPINTVL_SEC.load(Ordering::Relaxed) * tcp_time(1),
                    Ordering::Relaxed,
                ),
                _ => {}
            }
            error
        }
    }
}

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

impl<G: ProcGlue> Usrreqs<G> for TcpUsrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        tcp_attach(so, w, proto, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        tcp_detach(so, w)
    }
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        tcp_bind(so, w, nam, p)
    }
    fn listen(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        tcp_listen(so, w)
    }
    fn connect(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &Mbuf) -> Result<(), Errno> {
        tcp_connect(so, w, nam)
    }
    fn accept(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &mut Mbuf) -> Result<(), Errno> {
        tcp_accept(so, w, nam)
    }
    fn disconnect(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        tcp_disconnect(so, w)
    }
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        tcp_shutdown(so, w)
    }
    fn rcvd(&self, so: &Socket<G>, w: &mut SoLocked<'_>) {
        tcp_rcvd(so, w);
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        _addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        tcp_send(so, w, top, control)
    }
    fn abort(&self, so: &Socket<G>, w: &mut SoLocked<'_>) {
        tcp_abort(so, w);
    }
    fn send_id(&self) -> usize {
        core::ptr::from_ref(&TCP_SEND_ID).addr()
    }
    /// `in_control` (`tcp_usrreqs`) or `in6_control` (`tcp6_usrreqs`).
    fn control(
        &self,
        so: &Socket<G>,
        cmd: u64,
        data: &mut [u8],
        ifp: Option<&'static Ifnet>,
    ) -> Result<(), Errno> {
        if self.inet6 {
            in6_control(so.has_state(SS_PRIV), cmd, data, ifp)
        } else {
            in_control(so.has_state(SS_PRIV), cmd, data, ifp)
        }
    }
    fn sense(&self, so: &Socket<G>, w: &mut SoLocked<'_>, ub: &mut Stat) -> Result<(), Errno> {
        tcp_sense(so, w, ub)
    }
    fn rcvoob(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        m: &mut Mbuf,
        flags: i32,
    ) -> Result<(), Errno> {
        tcp_rcvoob(so, w, m, flags)
    }
    fn sendoob(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        _addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        tcp_sendoob(so, w, top, control)
    }
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        tcp_sockaddr(so, w, addr)
    }
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        tcp_peeraddr(so, w, addr)
    }
    /// `in_flowid`.
    fn flowid(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> i32 {
        let il = inet(w);
        sotoinpcb_locked(so, &il.pcb).map_or(0, |inp| i32::from(inp.inp_flowid.get()))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of TCP's user requests over the test interfaces (`testtcp`'s fixtures, whose
    // protocols' requests are these): an active open walked to TIME_WAIT with data both ways
    // (bsd's `active_open_data_and_close`, re-expressed); the socket buffers' scaling; the
    // statistics and variables of `net.inet.tcp` (bsd's `sysctl_reads_...`). The passive
    // open through the SYN cache, the reset of a segment to a closed port and IPv6's open and
    // reset are `tcp_input`'s tests, whose listeners attach through `tcp_attach`.

    use std::vec::Vec;

    use hal::netlock::{NETLOCK, NetHeld};
    use hal::sys::netinet_in::InAddr;
    use hal::sys::netinet_ip::Ip;
    use hal::sys::netinet_tcp::{TH_ACK, TH_FIN, TH_SYN, Tcphdr};

    use super::*;
    use crate::net::mbuf::{M_DONTWAIT, M_TCP_CSUM_IN_OK, MT_DATA, MT_SONAME, m_get, m_gethdr};
    use crate::net::sockbuf::SS_CANTRCVMORE;
    use crate::netinet::in_pcb::{BADDYNAMICPORTS, DP_MAPSIZE, sotoinpcb, testinet};
    use crate::netinet::tcp_fsm::TCPS_TIME_WAIT;
    use crate::netinet::tcp_input::tcp_input;
    use crate::netinet::tcp_subr::testtcp::{self, TcpProto, estab, last, stat};
    use crate::netinet::tcp_var::TcpstatCounters as C;
    use crate::sysctl::SECURELEVEL;

    /// A `sockaddr_in` of `addr`:`port` in a name mbuf (`MT_SONAME`).
    fn nam4(addr: [u8; 4], port: u16) -> MbufBox {
        let sin = SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_port: port.to_be(),
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes(addr),
            },
            ..SockaddrIn::default()
        };
        let mut m = m_get(M_WAIT, MT_SONAME).expect("an mbuf");
        set_nam(&mut m, &sin);
        m
    }

    /// A segment from 10.9.`n`.2:80 to 10.9.`n`.1:`lport` with the header fields given (host
    /// order), the options `opts` and `data`, its checksum taken as verified by the interface.
    #[allow(clippy::too_many_arguments, reason = "the header's fields")]
    fn from_peer(
        n: u8,
        lport: u16,
        flags: u8,
        seq: u32,
        ack: u32,
        opts: &[u8],
        data: &[u8],
    ) -> MbufBox {
        let thlen = size_of::<Tcphdr>() + opts.len();
        let len = 20 + thlen + data.len();
        let mut ip = Ip {
            ip_len: (len as u16).to_be(),
            ip_ttl: 64,
            ip_p: IPPROTO_TCP as u8,
            ip_src: InAddr {
                s_addr: u32::from_ne_bytes([10, 9, n, 2]),
            },
            ip_dst: InAddr {
                s_addr: u32::from_ne_bytes([10, 9, n, 1]),
            },
            ..Ip::default()
        };
        ip.set_ip_v(4);
        ip.set_ip_hl(5);
        let mut th = Tcphdr {
            th_sport: 80u16.to_be(),
            th_dport: lport.to_be(),
            th_seq: seq.to_be(),
            th_ack: ack.to_be(),
            th_flags: flags,
            th_win: 16384u16.to_be(),
            ..Tcphdr::default()
        };
        th.set_th_off((thlen / 4) as u8);
        let mut p = std::vec![0u8; len];
        assert!(hal::pod::put(&mut p, 0, &ip));
        assert!(hal::pod::put(&mut p, 20, &th));
        p[40..40 + opts.len()].copy_from_slice(opts);
        p[20 + thlen..].copy_from_slice(data);
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(len as u32);
        m.m_pkthdr().len.set(len as i32);
        m.data_mut().expect("writable").copy_from_slice(&p);
        let ph = m.m_pkthdr();
        ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_IN_OK);
        m
    }

    /// `ip_input`'s delivery of the IPv4 segment `m` to `tcp_input`, under the shared net
    /// lock.
    fn deliver(m: MbufBox) {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut off = 20;
        let mut mp = Some(m);
        let _ = tcp_input::<testtcp::G>(
            &mut mp,
            &mut off,
            IPPROTO_TCP,
            i32::from(AF_INET),
            None,
            &net,
        );
        drop(w);
        assert!(mp.is_none(), "the segment consumed");
    }

    /// What `f` reads off the control block of `inp`.
    fn peek<R>(inp: &Inpcb<testtcp::G>, f: impl FnOnce(&TcpState) -> R) -> R {
        testtcp::with_tp(inp, |tp| (f(tp.st), None))
    }

    /// The bytes in the receive buffer of `so`.
    fn received(so: &Socket<testtcp::G>) -> Vec<u8> {
        let n = so.so_rcv.cc() as usize;
        let g = so.so_rcv.lock();
        let mut v = std::vec![0u8; n];
        g.copydata_first(0, &mut v);
        v
    }

    /// `data` in an mbuf of its own.
    fn data(bytes: &[u8]) -> MbufBox {
        let mut m = m_get(M_WAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(bytes.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(bytes);
        m
    }

    #[test]
    fn an_active_open_carries_data_both_ways_and_closes() {
        let (_g, _p) = testtcp::setup();
        let n = 150;
        let (_ifp, d) = testinet::iface(b"tusr2", [10, 9, n, 1]);
        let so = testtcp::socket(&TcpProto);
        let inp = sotoinpcb(&so).expect("its pcb");
        let attempts = stat(C::TcpsConnattempt);

        // connect(2): SYN_SENT, the SYN out with our MSS first.
        locked(&so, |w| tcp_connect(&so, w, &nam4([10, 9, n, 2], 80))).expect("connect");
        assert_eq!(stat(C::TcpsConnattempt), attempts + 1);
        assert_eq!(peek(&inp, |st| st.t_state), TCPS_SYN_SENT);
        assert!(so.has_state(SS_ISCONNECTING));
        assert!(so.has_state(SS_CONNECTOUT));
        let (_ip, syn, opts, _p) = last(d);
        assert_eq!(syn.th_flags, TH_SYN);
        assert_eq!(u16::from_be(syn.th_dport), 80);
        let lport = u16::from_be(syn.th_sport);
        let iss = u32::from_be(syn.th_seq);
        assert_eq!(&opts[..2], &[2, 4], "the MSS option first");

        // The SYN-ACK with an MSS of 1000: ESTABLISHED, its ACK out.
        let piss = 0x1000_0000u32;
        let mss = [2, 4, 0x03, 0xe8];
        deliver(from_peer(
            n,
            lport,
            TH_SYN | TH_ACK,
            piss,
            iss.wrapping_add(1),
            &mss,
            &[],
        ));
        assert_eq!(
            peek(&inp, |st| (st.t_state, st.t_maxseg)),
            (TCPS_ESTABLISHED, 1000)
        );
        assert!(so.has_state(SS_ISCONNECTED));
        let (_ip, ack, _o, _p) = last(d);
        assert_eq!(ack.th_flags, TH_ACK);
        assert_eq!(u32::from_be(ack.th_ack), piss.wrapping_add(1));

        // Data out (tcp_send), then the peer's data acknowledging it.
        locked(&so, |w| tcp_send(&so, w, Some(data(b"hello")), None)).expect("send");
        let (_ip, out, _o, payload) = last(d);
        assert_eq!(out.th_flags & TH_ACK, TH_ACK);
        assert_eq!(u32::from_be(out.th_seq), iss.wrapping_add(1));
        assert_eq!(payload, b"hello");
        deliver(from_peer(
            n,
            lport,
            TH_ACK,
            piss.wrapping_add(1),
            iss.wrapping_add(6),
            &[],
            b"world",
        ));
        assert_eq!(received(&so), b"world");
        assert_eq!(
            peek(&inp, |st| (st.snd_una, st.rcv_nxt)),
            (iss.wrapping_add(6), piss.wrapping_add(6))
        );

        // shutdown(2): FIN_WAIT_1 and our FIN; its ACK, then the peer's FIN: TIME_WAIT.
        locked(&so, |w| tcp_shutdown(&so, w)).expect("shutdown");
        assert_eq!(peek(&inp, |st| st.t_state), TCPS_FIN_WAIT_1);
        let (_ip, fin, _o, _p) = last(d);
        assert_eq!(fin.th_flags & TH_FIN, TH_FIN);
        assert_eq!(u32::from_be(fin.th_seq), iss.wrapping_add(6));
        deliver(from_peer(
            n,
            lport,
            TH_ACK,
            piss.wrapping_add(6),
            iss.wrapping_add(7),
            &[],
            &[],
        ));
        assert_eq!(peek(&inp, |st| st.t_state), TCPS_FIN_WAIT_2);
        deliver(from_peer(
            n,
            lport,
            TH_FIN | TH_ACK,
            piss.wrapping_add(6),
            iss.wrapping_add(7),
            &[],
            &[],
        ));
        assert_eq!(peek(&inp, |st| st.t_state), TCPS_TIME_WAIT);
        assert!(so.so_rcv.has_state(SS_CANTRCVMORE));
        let (_ip, last_ack, _o, _p) = last(d);
        assert_eq!(u32::from_be(last_ack.th_ack), piss.wrapping_add(7));

        testtcp::close(so, inp);
    }

    #[test]
    fn sysctl_reads_the_statistics_and_the_variables() {
        let (_g, _p) = testtcp::setup();
        let u = crate::testutil::USER_BASE + 0x3_d000;
        let req = |oldp: usize, len: &mut usize, newp: usize, newlen: usize| -> Result<(), Errno> {
            let mut r = SysctlReq {
                oldp,
                oldlenp: len,
                newp,
                newlen,
            };
            tcp_sysctl::<testtcp::G>(&[TCPCTL_STATS], &mut r)
        };
        // The size alone, then the structure: the counters as they stand.
        let mut len = 0;
        assert_eq!(req(0, &mut len, 0, 0), Ok(()));
        assert_eq!(len, size_of::<Tcpstat>());
        let before = stat(C::TcpsConnattempt);
        assert_eq!(req(u, &mut len, 0, 0), Ok(()));
        let after = stat(C::TcpsConnattempt);
        let got: Tcpstat = crate::testutil::user_bytes(u, len)
            .and_then(|b| hal::user::pod_from_bytes(&b))
            .expect("the structure");
        assert!((before..=after).contains(&u64::from(got.tcps_connattempt)));
        let limit = TCP_SYN_CACHE_LIMIT.load(Ordering::Relaxed);
        assert_eq!(got.tcps_sc_entry_limit, limit as u64);
        // Short room: ENOMEM; a new value: EPERM.
        let mut short = len - 1;
        assert_eq!(req(u, &mut short, 0, 0), Err(Errno::ENOMEM));
        let size = len;
        assert_eq!(req(0, &mut len, u, size), Err(Errno::EPERM));

        // A variable of tcpctl_vars: read, then a write out of its bounds refused.
        let int_req = |name: i32, oldp: usize, newp: usize| -> Result<(), Errno> {
            let mut len = size_of::<i32>();
            let mut r = SysctlReq {
                oldp,
                oldlenp: &mut len,
                newp,
                newlen: if newp != 0 { size_of::<i32>() } else { 0 },
            };
            tcp_sysctl::<testtcp::G>(&[name], &mut r)
        };
        assert_eq!(int_req(TCPCTL_MSSDFLT, u, 0), Ok(()));
        let v: i32 = UserPtr::<i32>::new(u).copyin().expect("the value");
        assert_eq!(v, TCP_MSSDFLT.load(Ordering::Relaxed));
        UserPtr::<i32>::new(u + 8)
            .copyout(&(TCP_MSS - 1))
            .expect("a value");
        assert_eq!(int_req(TCPCTL_MSSDFLT, 0, u + 8), Err(Errno::EINVAL));
        assert_eq!(v, TCP_MSSDFLT.load(Ordering::Relaxed), "unchanged");
        // TCPCTL_KEEPINITTIME in seconds sets tcp_keepinit in milliseconds.
        let old = TCP_KEEPINIT_SEC.load(Ordering::Relaxed);
        UserPtr::<i32>::new(u + 8).copyout(&7).expect("a value");
        assert_eq!(int_req(TCPCTL_KEEPINITTIME, 0, u + 8), Ok(()));
        assert_eq!(TCP_KEEPINIT.load(Ordering::Relaxed), tcp_time(7));
        UserPtr::<i32>::new(u + 8).copyout(&old).expect("a value");
        assert_eq!(int_req(TCPCTL_KEEPINITTIME, 0, u + 8), Ok(()));
        assert_eq!(TCP_KEEPINIT.load(Ordering::Relaxed), tcp_time(old));
        // Names: one component only, and known.
        let mut len = 0;
        let mut r = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            tcp_sysctl::<testtcp::G>(&[TCPCTL_STATS, 1], &mut r),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            tcp_sysctl::<testtcp::G>(&[99], &mut r),
            Err(Errno::EOPNOTSUPP)
        );
    }

    #[test]
    fn the_send_buffer_scales_with_the_data_in_flight() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tusr0", 50);
        // The user's size is tcp_sendspace (soreserve's 16384): automatic scaling, by the
        // 10000 bytes in flight, rounded up to the segment (1460).
        testtcp::with_tp(&inp, |tp| {
            tp.st.snd_max = tp.st.snd_una.wrapping_add(10000);
            tcp_update_sndspace(&tp);
            ((), None)
        });
        assert_eq!(so.so_snd.hiwat(), (16384u64 + 10000).div_ceil(1460) * 1460);

        // Across the wrap: snd_una near 2^32, snd_max past 0. The C sums
        // sb_wat + snd_max - snd_una in u_long, a value past sb_max: the buffer asks for
        // sb_max rounded up, which sbreserve refuses, and stays as it was (LZ's
        // sb_wat + (snd_max - snd_una) would have grown it by the 512 bytes in flight).
        let before = so.so_snd.hiwat();
        testtcp::with_tp(&inp, |tp| {
            tp.st.snd_una = 0xffff_ff00;
            tp.st.snd_max = 0x100;
            tcp_update_sndspace(&tp);
            ((), None)
        });
        assert_eq!(so.so_snd.hiwat(), before, "the C's sum: sbreserve refused");
        testtcp::with_tp(&inp, |tp| {
            tp.st.snd_una = 1000;
            tp.st.snd_max = 1000;
            ((), None)
        });
        testtcp::close(so, inp);
    }

    #[test]
    fn the_receive_buffer_grows_by_a_step_when_nearly_filled() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tusr1", 51);
        // Less than 7/8 of the buffer in a round trip: unchanged.
        testtcp::with_tp(&inp, |tp| {
            tp.st.rfbuf_cnt = 16384 / 8 * 7;
            tcp_update_rcvspace(&tp);
            ((), None)
        });
        assert_eq!(so.so_rcv.hiwat(), 16384);
        // More: tcp_autorcvbuf_inc more, rounded up to the segment.
        testtcp::with_tp(&inp, |tp| {
            tp.st.rfbuf_cnt = 16384 / 8 * 7 + 1;
            tcp_update_rcvspace(&tp);
            ((), None)
        });
        assert_eq!(
            so.so_rcv.hiwat(),
            (16384u64 + u64::from(TCP_AUTORCVBUF_INC)).div_ceil(1460) * 1460
        );
        testtcp::close(so, inp);
    }

    /// An option mbuf (`MT_SOOPTS`) holding `bytes`.
    fn optm(bytes: &[u8]) -> MbufBox {
        let mut m = m_get(M_WAIT, crate::net::mbuf::MT_SOOPTS).expect("an mbuf");
        m.m_len().set(bytes.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(bytes);
        m
    }

    /// `f` under `so`'s lock, as a request runs; its answer is checked by the caller after
    /// the lock is let go (an assertion that fails under a socket lock would leave it held
    /// for the tests after). `R` cannot borrow the lock.
    fn locked<R>(so: &Socket<testtcp::G>, f: impl FnOnce(&mut SoLocked<'_>) -> R) -> R {
        let mut w = crate::net::socket::solock(so);
        let r = f(&mut w);
        drop(w);
        r
    }

    /// `setsockopt(so, IPPROTO_TCP, opt, &v, sizeof(int))` through `tcp_ctloutput`.
    fn set(so: &Socket<testtcp::G>, opt: i32, v: i32) -> Result<(), Errno> {
        let mut w = crate::net::socket::solock(so);
        let mut m = optm(&v.to_ne_bytes());
        tcp_ctloutput(PRCO_SETOPT, so, &mut w, IPPROTO_TCP, opt, Some(&mut m))
    }

    /// `getsockopt(so, IPPROTO_TCP, opt, ..)` through `tcp_ctloutput`: the answer's bytes.
    fn get(so: &Socket<testtcp::G>, opt: i32) -> Result<Vec<u8>, Errno> {
        let mut w = crate::net::socket::solock(so);
        let mut m = optm(&[]);
        tcp_ctloutput(PRCO_GETOPT, so, &mut w, IPPROTO_TCP, opt, Some(&mut m))?;
        Ok(m.data().to_vec())
    }

    /// [`get`] of an `int` option.
    fn get_int(so: &Socket<testtcp::G>, opt: i32) -> i32 {
        let b = get(so, opt).expect("the option");
        i32::from_ne_bytes(b[..4].try_into().expect("an int"))
    }

    #[test]
    fn tcp_ctloutput_keeps_the_c_s_options_and_errnos() {
        let (_g, _p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tusr3", 152);
        // TCP_NODELAY and TCP_NOPUSH answer their flag's bit, as the C's `t_flags & TF_x`.
        assert_eq!(set(&so, TCP_NODELAY, 1), Ok(()));
        assert_eq!(get_int(&so, TCP_NODELAY), TF_NODELAY as i32);
        assert_eq!(set(&so, TCP_NODELAY, 0), Ok(()));
        assert_eq!(get_int(&so, TCP_NODELAY), 0);
        assert_eq!(set(&so, TCP_NOPUSH, 7), Ok(()));
        assert_eq!(get_int(&so, TCP_NOPUSH), TF_NOPUSH as i32);
        assert_eq!(set(&so, TCP_NOPUSH, 0), Ok(()), "established: tcp_output");
        assert_eq!(get_int(&so, TCP_NOPUSH), 0);
        // A value shorter than an int, or none: EINVAL.
        let (short, none) = locked(&so, |w| {
            let mut m = optm(&[1, 0]);
            let short = tcp_ctloutput(PRCO_SETOPT, &so, w, IPPROTO_TCP, TCP_NODELAY, Some(&mut m));
            let none = tcp_ctloutput(PRCO_SETOPT, &so, w, IPPROTO_TCP, TCP_MAXSEG, None);
            (short, none)
        });
        assert_eq!(short, Err(Errno::EINVAL));
        assert_eq!(none, Err(Errno::EINVAL));
        // TCP_MAXSEG: within (0, t_maxseg] only.
        let maxseg = get_int(&so, TCP_MAXSEG);
        assert_eq!(maxseg, 1460, "estab's segment, from the route");
        assert_eq!(set(&so, TCP_MAXSEG, 0), Err(Errno::EINVAL));
        assert_eq!(set(&so, TCP_MAXSEG, maxseg + 1), Err(Errno::EINVAL));
        assert_eq!(set(&so, TCP_MAXSEG, maxseg), Ok(()));
        assert_eq!(set(&so, TCP_MAXSEG, 536), Ok(()));
        assert_eq!(get_int(&so, TCP_MAXSEG), 536);
        // Established: SACK and the signature are refused (EPERM).
        assert_eq!(set(&so, TCP_SACK_ENABLE, 0), Err(Errno::EPERM));
        assert_eq!(set(&so, TCP_MD5SIG, 1), Err(Errno::EPERM));
        assert_eq!(get_int(&so, TCP_MD5SIG), 0);
        // Unknown options.
        assert_eq!(set(&so, 0x7f, 1), Err(Errno::ENOPROTOOPT));
        assert_eq!(get(&so, 0x7f), Err(Errno::ENOPROTOOPT));
        testtcp::close(so, inp);

        // Before the connection: TCP_MD5SIG clears sack_enable, and TCP_SACK_ENABLE is
        // refused while the signature is on (the user's 6a3 decision: in every build).
        let so = testtcp::socket(&TcpProto);
        let inp = sotoinpcb(&so).expect("its pcb");
        assert_eq!(get_int(&so, TCP_SACK_ENABLE), 1, "tcp_do_sack");
        assert_eq!(set(&so, TCP_MD5SIG, 1), Ok(()));
        assert_eq!(get_int(&so, TCP_MD5SIG), TF_SIGNATURE as i32);
        assert_eq!(get_int(&so, TCP_SACK_ENABLE), 0, "cleared by TCP_MD5SIG");
        assert_eq!(set(&so, TCP_SACK_ENABLE, 1), Err(Errno::EPERM));
        assert_eq!(set(&so, TCP_MD5SIG, 0), Ok(()));
        assert_eq!(set(&so, TCP_SACK_ENABLE, 1), Ok(()));
        assert_eq!(get_int(&so, TCP_SACK_ENABLE), 1);
        so.st_set(SS_NOFDREF);
        locked(&so, |w| tcp_detach(&so, w)).expect("the control block");
        drop(inp);
    }

    #[test]
    fn tcp_info_gives_the_privileged_members_to_root_alone() {
        let (_g, p) = testtcp::setup();
        let (so, inp, _d) = estab(b"tusr5", 154);
        let info = || -> TcpInfo {
            let b = get(&so, TCP_INFO).expect("TCP_INFO");
            assert_eq!(b.len(), size_of::<TcpInfo>());
            hal::pod::at(&b, 0).expect("the structure")
        };
        let root = info();
        assert_eq!(i32::from(root.tcpi_state), TCPS_ESTABLISHED);
        assert_eq!(root.tcpi_snd_mss, 1460);
        assert_eq!(root.tcpi_snd_wnd, 65535, "root sees the window");
        assert_eq!(root.tcpi_rcv_nxt, 1, "relative to irs");
        assert_eq!(root.tcpi_so_rcv_sb_hiwat, 16384);
        // Another user: the members from tcpi_snd_wnd on stay zero.
        let mut cr = crate::cred::Ucred::new();
        cr.cr_uid.set(1000);
        cr.cr_ruid.set(1000);
        let old = p.set_cred(crate::cred::CredRef::new(cr));
        let user = info();
        drop(p.set_cred(old.expect("root's credentials")));
        assert_eq!(i32::from(user.tcpi_state), TCPS_ESTABLISHED);
        assert_eq!(user.tcpi_snd_mss, 1460);
        assert_eq!(
            (
                user.tcpi_snd_wnd,
                user.tcpi_rcv_nxt,
                user.tcpi_so_rcv_sb_hiwat
            ),
            (0, 0, 0),
            "nothing privileged for a user"
        );
        testtcp::close(so, inp);
    }

    /// `struct tcp_ident_mapping` of a connection from `l` to `f` (`sockaddr_in`s), with the
    /// storages' `ss_len` left 0: the C reads them whole, by family.
    fn mapping(f: ([u8; 4], u16), l: ([u8; 4], u16)) -> TcpIdentMapping {
        let mut t = TcpIdentMapping::zeroed();
        let sin = |(a, p): ([u8; 4], u16)| SockaddrIn {
            sin_len: 0,
            sin_family: AF_INET,
            sin_port: p.to_be(),
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes(a),
            },
            ..SockaddrIn::default()
        };
        let mut b = [0u8; size_of::<hal::sys::socket::SockaddrStorage>()];
        assert!(hal::pod::put(&mut b, 0, &sin(f)));
        t.faddr = hal::pod::at(&b, 0).expect("a storage");
        assert!(hal::pod::put(&mut b, 0, &sin(l)));
        t.laddr = hal::pod::at(&b, 0).expect("a storage");
        t
    }

    /// `sysctl(net.inet.tcp.ident or .drop)` with the mapping at the user address `u`.
    fn ident(u: usize, drop: bool, oldlen: usize, newlen: usize) -> Result<usize, Errno> {
        let mut len = oldlen;
        let mut r = SysctlReq {
            oldp: if drop { 0 } else { u },
            oldlenp: &mut len,
            newp: if drop { u } else { 0 },
            newlen,
        };
        let name = if drop { TCPCTL_DROP } else { TCPCTL_IDENT };
        tcp_sysctl::<testtcp::G>(&[name], &mut r).map(|()| len)
    }

    #[test]
    fn tcp_ident_answers_the_ids_and_drop_ends_the_connection() {
        let (_g, _p) = testtcp::setup();
        let n = 153;
        let (so, inp, _d) = estab(b"tusr4", n);
        let u = crate::testutil::USER_BASE + 0x3_e000;
        let size = size_of::<TcpIdentMapping>();
        let put = |t: &TcpIdentMapping| {
            UserPtr::<TcpIdentMapping>::new(u)
                .copyout(t)
                .expect("copyout")
        };
        let back = || UserPtr::<TcpIdentMapping>::new(u).copyin().expect("copyin");
        let conn = mapping(([10, 9, n, 2], 80), ([10, 9, n, 1], 7000));

        // ident: the connection did not connect out (no SS_CONNECTOUT): -1, -1.
        put(&conn);
        assert_eq!(ident(u, false, size, 0), Ok(size));
        assert_eq!((back().ruid, back().euid), (-1, -1));
        so.st_set(SS_CONNECTOUT);
        put(&conn);
        assert_eq!(ident(u, false, size, 0), Ok(size));
        // The fixture's ids differ (euid 0, ruid 1000), so a swap is caught.
        let (euid, ruid, ..) = so.creds();
        assert_eq!((euid, ruid), (0, testinet::TEST_RUID));
        assert_eq!((back().ruid, back().euid), (ruid as i32, euid as i32));
        // No connection: the listen lookup's miss counted, -1, -1.
        let miss = stat(C::TcpsPcbhashmiss);
        put(&mapping(([10, 9, n, 2], 81), ([10, 9, n, 1], 7000)));
        assert_eq!(ident(u, false, size, 0), Ok(size));
        assert_eq!(stat(C::TcpsPcbhashmiss), miss + 1);
        assert_eq!((back().ruid, back().euid), (-1, -1));
        // ident's checks, in the C's order.
        assert_eq!(ident(0, false, size, 0), Err(Errno::EINVAL));
        assert_eq!(ident(u, false, size - 1, 0), Err(Errno::ENOMEM));
        let mut len = size;
        let mut r = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: u,
            newlen: 0,
        };
        assert_eq!(
            tcp_sysctl::<testtcp::G>(&[TCPCTL_IDENT], &mut r),
            Err(Errno::EINVAL)
        );
        // Families: both AF_INET, or both AF_INET6; anything else EAFNOSUPPORT.
        let mut mixed = conn;
        mixed.laddr.ss_family = AF_INET6;
        put(&mixed);
        assert_eq!(ident(u, false, size, 0), Err(Errno::EAFNOSUPPORT));
        mixed.faddr.ss_family = 99;
        put(&mixed);
        assert_eq!(ident(u, false, size, 0), Err(Errno::EAFNOSUPPORT));

        // drop's checks: no old value, a new one, the whole mapping.
        put(&conn);
        assert_eq!(ident(u, true, 4, size), Err(Errno::EINVAL));
        assert_eq!(ident(0, true, 0, size), Err(Errno::EPERM));
        assert_eq!(ident(u, true, 0, size - 1), Err(Errno::ENOMEM));
        // A listener is not dropped (ESRCH).
        let (lso, linp) = testtcp::listen_on(n, 81);
        put(&mapping(([0, 0, 0, 0], 0), ([10, 9, n, 1], 81)));
        assert_eq!(ident(u, true, 0, size), Err(Errno::ESRCH));
        testtcp::close_listener(lso, linp);
        // The connection dropped: its control block freed, the error reported; then none.
        let cbs = testtcp::tcpcbs();
        put(&conn);
        assert_eq!(ident(u, true, 0, size), Ok(0));
        assert_eq!(testtcp::tcpcbs(), cbs - 1);
        assert_eq!(so.error(), Some(Errno::ECONNABORTED));
        assert_eq!(ident(u, true, 0, size), Err(Errno::ESRCH));
        so.st_set(SS_NOFDREF);
        drop(inp);
        drop(so);
    }

    /// `sysctl(net.inet.tcp.<name>)` of an `int`: the old value at `u`, the new one from
    /// `u + 8` when `new` is given.
    fn int_sysctl(name: i32, u: usize, new: Option<i32>) -> Result<i32, Errno> {
        let mut len = size_of::<i32>();
        if let Some(v) = new {
            UserPtr::<i32>::new(u + 8).copyout(&v).expect("copyout");
        }
        let mut r = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: if new.is_some() { u + 8 } else { 0 },
            newlen: if new.is_some() { size_of::<i32>() } else { 0 },
        };
        tcp_sysctl::<testtcp::G>(&[name], &mut r)?;
        Ok(UserPtr::<i32>::new(u).copyin().expect("copyin"))
    }

    #[test]
    fn sysctl_keeps_the_bounds_the_locks_and_the_pools_limits() {
        let (_g, _p) = testtcp::setup();
        // 8 KiB for the ports' bitmaps, below the user window's end.
        let u = crate::testutil::USER_BASE + 0x3_a000;
        // The SYN cache's tunables: bounded, the sets told under the mutex.
        let t = &net::<testtcp::G>().tcp;
        let hash = t.tcp_syn_hash_size.load(Ordering::Relaxed);
        assert_eq!(int_sysctl(TCPCTL_SYN_HASH_SIZE, u, None), Ok(hash));
        assert_eq!(
            int_sysctl(TCPCTL_SYN_HASH_SIZE, u, Some(0)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            int_sysctl(TCPCTL_SYN_HASH_SIZE, u, Some(100_001)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            int_sysctl(TCPCTL_SYN_HASH_SIZE, u, Some(hash + 1)),
            Ok(hash)
        );
        assert_eq!(t.tcp_syn_hash_size.load(Ordering::Relaxed), hash + 1);
        assert!(
            syn_cache_lock::<testtcp::G>()
                .sets
                .iter()
                .all(|s| s.scs_use == 0),
            "a size change switches the sets as soon as possible"
        );
        assert_eq!(
            int_sysctl(TCPCTL_SYN_HASH_SIZE, u, Some(hash)),
            Ok(hash + 1)
        );
        let limit = t.tcp_syn_use_limit.load(Ordering::Relaxed);
        assert_eq!(
            int_sysctl(TCPCTL_SYN_USE_LIMIT, u, Some(-1)),
            Err(Errno::EINVAL)
        );
        assert_eq!(int_sysctl(TCPCTL_SYN_USE_LIMIT, u, Some(5)), Ok(limit));
        assert!(
            syn_cache_lock::<testtcp::G>()
                .sets
                .iter()
                .all(|s| s.scs_use <= 5)
        );
        assert_eq!(int_sysctl(TCPCTL_SYN_USE_LIMIT, u, Some(limit)), Ok(5));

        // The reassembly limit is the "tcpqe" pool's hard limit: refused below the items out.
        let reass = TCP_REASS_LIMIT.load(Ordering::Relaxed);
        let held = TCPQE_POOL
            .get(hal::pool::PR_WAITOK, || crate::netinet::tcp_var::Tcpqent {
                tcpqe_q: hal::owned::SlistLink::new(),
                tcpqe_tcp: hal::sys::netinet_tcp::Tcphdr::default(),
                tcpqe_m: None,
            })
            .expect("an entry");
        assert_eq!(
            int_sysctl(TCPCTL_REASS_LIMIT, u, Some(0)),
            Err(Errno::EINVAL)
        );
        assert_eq!(TCP_REASS_LIMIT.load(Ordering::Relaxed), reass, "unchanged");
        drop(held);
        assert_eq!(
            int_sysctl(TCPCTL_REASS_LIMIT, u, Some(reass + 1)),
            Ok(reass)
        );
        assert_eq!(TCP_REASS_LIMIT.load(Ordering::Relaxed), reass + 1);
        assert_eq!(
            int_sysctl(TCPCTL_REASS_LIMIT, u, Some(reass)),
            Ok(reass + 1)
        );

        // The ports' bitmaps: read whole; rootonly refuses a new value at securelevel > 0.
        let buflen = DP_MAPSIZE * size_of::<u32>();
        let map = |name: i32, newp: usize| -> Result<(), Errno> {
            let mut len = buflen;
            let mut r = SysctlReq {
                oldp: u,
                oldlenp: &mut len,
                newp,
                newlen: if newp != 0 { buflen } else { 0 },
            };
            tcp_sysctl::<testtcp::G>(&[name], &mut r)
        };
        assert_eq!(map(TCPCTL_BADDYNAMIC, 0), Ok(()));
        let word0 = BADDYNAMICPORTS.tcp[0].load(Ordering::Relaxed);
        assert_eq!(UserPtr::<u32>::new(u).copyin(), Ok(word0));
        let level = SECURELEVEL.swap(1, Ordering::Relaxed);
        let refused = map(TCPCTL_ROOTONLY, u);
        let read = map(TCPCTL_ROOTONLY, 0);
        SECURELEVEL.store(level, Ordering::Relaxed);
        assert_eq!((refused, read), (Err(Errno::EPERM), Ok(())));
        // A write (the same bytes back) under the exclusive net lock.
        assert_eq!(map(TCPCTL_BADDYNAMIC, u), Ok(()));
        assert_eq!(BADDYNAMICPORTS.tcp[0].load(Ordering::Relaxed), word0);
    }

    #[test]
    fn the_requests_walk_the_close_states_and_the_oob_byte() {
        let (_g, _p) = testtcp::setup();
        // A user close in CLOSE_WAIT: LAST_ACK and our FIN.
        let (so, inp, d) = estab(b"tusr6", 155);
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_CLOSE_WAIT;
            ((), None)
        });
        locked(&so, |w| tcp_disconnect(&so, w)).expect("disconnect");
        assert_eq!(peek(&inp, |st| st.t_state), TCPS_LAST_ACK);
        let (_ip, fin, _o, _p) = last(d);
        assert_eq!(fin.th_flags & TH_FIN, TH_FIN);
        testtcp::close(so, inp);

        // A shutdown in FIN_WAIT_2: disconnected, the 2MSL timer armed (not left for ever).
        let (so, inp, _d) = estab(b"tusr7", 156);
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_state = TCPS_FIN_WAIT_2;
            ((), None)
        });
        locked(&so, |w| tcp_shutdown(&so, w)).expect("shutdown");
        assert!(so.has_state(crate::net::sockbuf::SS_ISDISCONNECTED));
        assert!(peek(&inp, |st| st
            .has_flags(crate::netinet::tcp_var::TF_TMR_2MSL)));
        testtcp::close(so, inp);

        // SO_LINGER with a zero linger: the disconnect drops (a RST), freeing the block.
        let (so, inp, d) = estab(b"tusr8", 157);
        let cbs = testtcp::tcpcbs();
        locked(&so, |w| {
            so.set_options(w, SO_LINGER, true);
            tcp_disconnect(&so, w)
        })
        .expect("disconnect");
        let (_ip, rst, _o, _p) = last(d);
        assert_eq!(
            rst.th_flags & hal::sys::netinet_tcp::TH_RST,
            hal::sys::netinet_tcp::TH_RST
        );
        assert_eq!(testtcp::tcpcbs(), cbs - 1);
        so.st_set(SS_NOFDREF);
        drop(inp);
        drop(so);

        // The out-of-band byte: read once (MSG_PEEK keeps it), then EINVAL.
        let (so, inp, _d) = estab(b"tusr9", 158);
        testtcp::with_tp(&inp, |tp| {
            tp.st.t_oobflags = TCPOOB_HAVEDATA;
            tp.st.t_iobc = b'x';
            ((), None)
        });
        so.set_oobmark(1);
        let oob = |flags: i32| -> Result<u8, Errno> {
            let mut m = optm(&[]);
            locked(&so, |w| tcp_rcvoob(&so, w, &mut m, flags))?;
            Ok(m.data()[0])
        };
        assert_eq!(oob(MSG_PEEK), Ok(b'x'));
        assert_eq!(oob(0), Ok(b'x'));
        assert_eq!(oob(0), Err(Errno::EINVAL), "TCPOOB_HADDATA");
        testtcp::close(so, inp);
    }

    #[test]
    fn connect_refuses_what_the_c_refuses_and_attach_undoes_a_failure() {
        let (_g, _p) = testtcp::setup();
        let n = 159;
        let (_ifp, _d) = testinet::iface(b"tusra", [10, 9, n, 1]);
        let so = testtcp::socket(&TcpProto);
        for a in [
            [0, 0, 0, 0],
            [255, 255, 255, 255],
            [224, 0, 0, 1],
            [10, 9, n, 255],
        ] {
            let r = locked(&so, |w| tcp_connect(&so, w, &nam4(a, 80)));
            assert_eq!(r, Err(Errno::EINVAL), "{a:?}");
        }
        so.st_set(SS_NOFDREF);
        locked(&so, |w| tcp_detach(&so, w)).expect("the control block");
        drop(so);

        // tcp_newtcpcb without an item: ENOBUFS, the pcb detached again (the socket's slot
        // and the pcb count each other until in_pcbdetach), SS_NOFDREF as it was.
        let pcbs = || tcbtable::<testtcp::G>().count();
        for nofd in [false, true] {
            let so = crate::net::socket::soalloc(
                &TcpProto,
                M_WAIT,
                crate::net::socket::SoInit {
                    so_type: hal::sys::socket::SOCK_STREAM,
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
            if nofd {
                so.st_set(SS_NOFDREF);
            }
            let before = pcbs();
            crate::testutil::fail_pool("tcpcb", 1);
            let r = locked(&so, |w| tcp_attach(&so, w, 0, M_WAIT));
            crate::testutil::fail_pool("tcpcb", 0);
            assert_eq!(r, Err(Errno::ENOBUFS));
            assert_eq!(pcbs(), before, "the pcb freed");
            assert!(!so.has_pcb());
            assert_eq!(so.has_state(SS_NOFDREF), nofd);
            so.st_set(SS_NOFDREF);
            drop(so);
        }
    }

    #[test]
    fn the_protocol_rows_are_the_c_s_and_the_softnet_reaches_tcp_input_mlist() {
        use crate::net::proto::{
            PR_ABRTACPTDIS, PR_CONNREQUIRED, PR_MPINPUT, PR_MPSYSCTL, PR_SPLICE, PR_WANTRCVD,
            Protocol,
        };
        use crate::netinet::in_proto::TcpProtocol;
        use crate::netinet6::in6_proto::Tcp6Protocol;

        let (_g, _p) = testtcp::setup();
        let flags =
            PR_CONNREQUIRED | PR_WANTRCVD | PR_ABRTACPTDIS | PR_SPLICE | PR_MPINPUT | PR_MPSYSCTL;
        let v4: &dyn Protocol<testtcp::G> = &TcpProtocol::<testtcp::G>::NEW;
        let v6: &dyn Protocol<testtcp::G> = &Tcp6Protocol::<testtcp::G>::NEW;
        for (pr, family) in [(v4, i32::from(AF_INET)), (v6, i32::from(AF_INET6))] {
            assert_eq!(i32::from(pr.ty()), hal::sys::socket::SOCK_STREAM);
            assert_eq!(i32::from(pr.protocol()), IPPROTO_TCP);
            assert_eq!(pr.flags(), flags);
            assert_eq!(pr.domain().family(), family);
            assert!(!pr.legacy(), "a kernel protocol: so_lock by a witness");
            assert!(pr.has_ctloutput() && pr.has_sysctl() && pr.usrreqs().is_some());
            let mut len = 0;
            let mut r = SysctlReq {
                oldp: 0,
                oldlenp: &mut len,
                newp: 0,
                newlen: 0,
            };
            assert_eq!(pr.sysctl(&[TCPCTL_STATS, 0], &mut r), Err(Errno::ENOTDIR));
        }
        // tcp_send is the two tables' one send: sosplice splices them into each other.
        let send = |pr: &dyn Protocol<testtcp::G>| pr.usrreqs().map(|u| u.send_id());
        assert_eq!(send(v4), send(v6));
        assert_ne!(send(v4), Some(0));

        // if_input_process's call: a softnet's batch through tcp_init's registration.
        let n = 160;
        let (so, inp, _d) = estab(b"tusrb", n);
        let mut ml = crate::net::mbuf::MbufList::new();
        let m = from_peer(n, 7000, TH_ACK, 5000, 1000, &[], b"abc");
        m.m_pkthdr().ph_cookie.set(20);
        crate::net::mbuf::ml_enqueue(&mut ml, m);
        {
            let w = NETLOCK.read();
            let net = NetHeld::of(&w).expect("the net lock");
            crate::netinet::tcp_input::tcp_input_mlist_softnet(&mut ml, i32::from(AF_INET), &net);
        }
        assert_eq!(received(&so), b"abc");
        testtcp::close(so, inp);
    }

    /// The pcb a request reaches, from the socket's locked slot (`sotoinpcb_locked`, part
    /// `PCB`), as `tcp_sogetpcb` borrows it: the counted slot's pcb, or none.
    fn slot_pcb(so: &Socket<testtcp::G>) -> Option<usize> {
        locked(so, |w| {
            let il = inet(w);
            sotoinpcb_locked(so, &il.pcb).map(|i| core::ptr::from_ref(i).addr())
        })
    }

    /// The review's S3: the requests borrow the pcb from the socket's locked slot, with no
    /// count taken; `tcp_attach` fills the slot, every close empties it (a request's through
    /// `tcp_free_request`, another frame's through `tcp_free`, `tcp_connect`'s detach of a
    /// socket without a file by itself), and a request after the close answers the socket's
    /// error as the C's `tcp_sogetpcb` (`tcp_ctloutput`'s other levels: `ECONNRESET`).
    #[test]
    fn the_requests_borrow_the_pcb_from_the_locked_slot_until_the_close() {
        let (_g, _p) = testtcp::setup();
        let n = 170;
        let (_ifp, _d) = testinet::iface(b"tusrc", [10, 9, n, 1]);

        // tcp_attach fills the slot with the counted slot's pcb.
        let so = testtcp::socket(&TcpProto);
        let inp = sotoinpcb(&so).expect("its pcb");
        let addr = core::ptr::from_ref::<Inpcb<testtcp::G>>(&inp).addr();
        assert_eq!(slot_pcb(&so), Some(addr));
        // A request borrows it: no count taken while it holds the pcb and the block.
        let before = PoolArc::count(&inp);
        let (same, during) = locked(&so, |w| {
            let mut il = inet(w);
            let tp = tcp_sogetpcb(&so, &mut il).expect("the control block");
            let same = core::ptr::eq(tp.inp, &*inp) && core::ptr::eq(tp.so, &*so);
            (same, PoolArc::count(&inp))
        });
        assert!(same, "the socket's pcb and the socket");
        assert_eq!(during, before, "no count taken by the request");
        // tcp_abort (a request's close): the slots empty, the pcb's handles gone but ours.
        locked(&so, |w| tcp_abort(&so, w));
        assert_eq!(slot_pcb(&so), None);
        assert!(sotoinpcb(&so).is_none());
        assert_eq!(PoolArc::count(&inp), 1, "ours alone");
        // A request after the close: the socket's error (ECONNABORTED), as tcp_sogetpcb's.
        let mut m = optm(&[]);
        let r = locked(&so, |w| tcp_sockaddr(&so, w, &mut m));
        assert_eq!(r, Err(Errno::ECONNABORTED));
        let r = locked(&so, |w| {
            use hal::sys::netinet_in::{IP_TOS, IPPROTO_IP};
            tcp_ctloutput(PRCO_GETOPT, &so, w, IPPROTO_IP, IP_TOS, Some(&mut m))
        });
        assert_eq!(r, Err(Errno::ECONNRESET), "no pcb: the C's ECONNRESET");
        so.st_set(SS_NOFDREF);
        drop(inp);
        drop(so);

        // Another frame's close (a callout's: tcp_close, then tcp_free under the lock it
        // took): the slot emptied by tcp_free.
        let (so, inp, _d) = estab(b"tusrd", n + 1);
        assert_eq!(
            slot_pcb(&so),
            Some(core::ptr::from_ref::<Inpcb<testtcp::G>>(&inp).addr())
        );
        let emptied = testtcp::with_lock(&inp, |i, il| {
            let tp = intotcpcb(i, il)?;
            let c = tcp_close(tp);
            tcp_free(i, &mut il.pcb, c);
            Some(sotoinpcb_locked(i.socket(), &il.pcb).is_none())
        });
        assert_eq!(emptied, Some(true), "tcp_free emptied the locked slot");
        assert_eq!(slot_pcb(&so), None);
        so.st_set(SS_NOFDREF);
        drop(inp);
        drop(so);

        // tcp_connect's template failure on a socket without a file: in_pcbdisconnect's
        // in_pcbdetach, and the locked slot let go once the request is done with the pcb.
        // A first connection clones the route to the peer (whose routing message would take
        // the failing mbuf), and is dropped.
        let warm = testtcp::socket(&TcpProto);
        let r = locked(&warm, |w| tcp_connect(&warm, w, &nam4([10, 9, n, 2], 80)));
        assert_eq!(r, Ok(()));
        locked(&warm, |w| tcp_abort(&warm, w));
        warm.st_set(SS_NOFDREF);
        drop(warm);
        let so = testtcp::socket(&TcpProto);
        let inp = sotoinpcb(&so).expect("its pcb");
        so.st_set(SS_NOFDREF);
        let nam = nam4([10, 9, n, 2], 80);
        crate::testutil::fail_pool("mbufpl", 1);
        let r = locked(&so, |w| tcp_connect(&so, w, &nam));
        crate::testutil::fail_pool("mbufpl", 0);
        assert_eq!(r, Err(Errno::ENOBUFS));
        assert!(!so.has_pcb(), "detached");
        assert_eq!(slot_pcb(&so), None);
        // The pcb keeps its control block (the C leaves `inp_ppcb` too: no `tcp_close` on
        // this path), whose timers count the pcb: neither slot counts it any more.
        let timers = crate::netinet::tcp_timer::TCPT_NTIMERS;
        assert_eq!(PoolArc::count(&inp), 1 + timers, "ours and the timers'");
        drop(inp);
        drop(so);
    }
}
/* </TESTS> */
