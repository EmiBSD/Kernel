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
 * Copyright (c) 1989, 1991, 1993, 1995
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
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
 *	@(#)nfs_socket.c	8.5 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! `nfs/nfs_socket.c` in the kernel, its client half: the RPC engine (connect and reconnect,
//! send, receive, the match of replies to requests, the retransmit timer, the congestion
//! window and the RTT estimator) and the send lock both halves use. The server's half
//! (`nfs_rephead`, `nfs_getreq`, `nfs_realign`, the `nfsrv_*` socket side) stays `bsd`'s until
//! N6.7b (D-5); it reaches [`nfs_send`] and [`nfs_sndlock`] through their optional request,
//! as the C's server does.
//!
//! Upstream: sys/nfs/nfs_socket.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_socket.rs@e8249449d905
//!
//! ## Deviations
//! - `nfs_receive`, `nfs_reply` and `nfs_request` are `#ifdef NFSCLIENT` in the C; here they
//!   are built in every kernel, as the rest of `kernel::fs::nfs`: whether the client exists is
//!   whether the glue registers it (`vfsconflist`'s row, N6.7a5), which `bsd`'s `nfsclient`
//!   feature decides.
//! - Errors are `Result`; `nfs_sigintr` answers `Err(EINTR)` for the C's nonzero and
//!   `nfs_rcvlock`'s "reply already received" is `Err(EALREADY)`, as in LZ. A reply status
//!   that names no errno comes out of [`nfs_request`] as `NfsError::Status` (D-3, `nfs`'s
//!   deviation).
//! - The client's receive does not call `nfs_realign`, nor count in `nfs_realign_test` (the
//!   server's half keeps both): the reply is read through `XdrReader`, which copies, so an
//!   mbuf's alignment is of no matter to it. The counters are visible to no interface
//!   (neither sysctl nor `netstat`), only to `kvm`.
//! - `nfs_reply` skips a receive that hands back no chain without an error (a datagram with
//!   no data), as a reply too short to hold an xid: the C would read through the NULL chain.
//! - `nfs_request` frees the reply of a denied RPC (`MSG_DENIED`), which the C leaks, as LZ;
//!   a reply whose RPC header is cut short is freed too and gives `EBADRPC` with no reply,
//!   where the C hands the chain up with the error (its callers free it unread).
//! - `MGETHDR`, `M_PREPEND` and `pool_get` with `M_WAIT`/`PR_WAITOK` can fail here (`hal`'s
//!   pools cannot always sleep for an item): `nfs_request` answers `ENOBUFS` with no reply,
//!   the request freed, as LZ; the C cannot fail there.
//! - The first send's copy of the request is made under the kernel lock's witness without
//!   waiting (`M_DONTWAIT`); only when that fails is the chain taken out of its cell for an
//!   `M_WAIT` copy and put back after, a window in which `nfs_timer` finds no chain to resend
//!   (a tick it skips). The C's `M_WAIT` copy may sleep under the kernel lock too, and its
//!   timer then resends the same chain.
//!
//! ## Redesign
//! - LZ's engine walked `&'static` requests and mounts with `Cell` members (`unsafe` for the
//!   pool item, the queue's links, the timeout's argument and `r_procp`), parked every mbuf it
//!   handed to `bsd`'s socket adapters, and lent the net lock to `bsd`'s `solocked_legacy` for
//!   each retransmission. Here the request is a counted "nfsreqpl" item that `nfs_request`'s
//!   frame holds ([`NfsReq`]), the mount a counted `MArc` the caller and the timeout hold
//!   ([`NfsMount`]), the socket a counted handle cloned out of the mount for a send or a
//!   receive, and the sockets are the kernel's (`socreate`, `sobind` with `proc0`'s privilege,
//!   `soconnect`, `sosend`, `soreceive`, the protocol's `send` under `solock_net` in the
//!   timer): no mbuf is parked, no lock lent.
//! - The C's lock is the kernel lock and stays it: the queue is walked under `klock::reader`
//!   and changed under `klock::witness`, the request's chains opened by an exclusive witness
//!   of its part ([`PART_REQ`]), the scalars relaxed atomics written there. No witness or
//!   reader is held across a send, a receive, a waiting allocation or a sleep: a section
//!   takes what it needs (a copy of the request, a socket handle) and ends. Per RPC that is
//!   three witness sections (the enqueue with the first send's copy, the match, the unlink),
//!   where the C has no lock transition of its own (`klock::witness` raises the interrupt
//!   level; N12's row).
//! - The C's `rep` names the request, its mount and its thread; the functions given it take a
//!   [`Rep`], the three lent by `nfs_request`'s frame. The thread is the requester's own
//!   (`nmi_procp`); the timer, which runs in the softclock thread, finds it again by its id
//!   (`tfind`, only on an `NFSMNT_INT` mount, as the C reads `r_procp` only there).
//! - `nfs_timer` walks `nm_reqsq` one reader at a time and keeps its place in a counted handle
//!   of the request it stops at, for the two things it does without a witness: a
//!   retransmission (the copy under a witness section, the protocol's send under the
//!   socket's lock) and the "not responding" message (the requester's terminal); the walk
//!   goes on from that request (`PinList::next`, O(1)), so it is the C's one pass in the
//!   C's order. A tick with nothing due costs a reader and no count.
//! - A request built by its caller ([`Request`]: the procedure's arguments as an
//!   `XdrWriter`, the thread, the credential) gets its RPC header from `nfsm_rpchead`
//!   (`nfs_subs`) before it is queued; the reply comes back as a [`Reply`], the C's two
//!   outputs kept: the reply survives a status error (version 3's post-op attributes are read
//!   from it), not a transport one.

use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering::Relaxed};

use hal::mem::pool::{PR_WAITOK, PoolArc};
use hal::sync::klock;
use hal::sync::netlock::{NETLOCK, NetHeld};
use hal::sync::rwlock::Divisible;
use hal::sync::sleep::{INFSLP, PCATCH, tsleep_nsec, wakeup};

use super::nfs::{
    NFS_DEFAULT_TIMER, NFS_GETATTR_TIMER, NFS_LOOKUP_TIMER, NFS_MAX_TIMER, NFS_MAXREXMIT,
    NFS_READ_TIMER, NFS_TIMEOUTMUL, NFS_WRITE_TIMER, NFSINT_SIGMASK, NfsError, NfsReq,
    R_MUSTRESEND, R_SENT, R_SOFTTERM, R_TIMING, R_TPRINTFMSG, Reply, nfs_initrtt, nfs_maxrto,
    nfs_maxtimeo, nfs_minrto, nfs_mintimeo, nfsignore_soerror,
};
use super::nfs_subs::{NFS_TICKS, NFSSTATS, nfsm_rpchead, rpc_mismatch, rpc_msgdenied, rpc_reply};
use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_rndup};
use super::nfsmount::NfsMount;
use super::nfsproto::{NFS_MAXPACKET, NFS_MAXPKTHDR, NFS_NPROCS, NFSERR_TRYLATER, NFSX_UNSIGNED};
use crate::assert::{kassert, kpanic};
use crate::base::log::{LOG_ERR, LOG_INFO};
use crate::base::prf::{Str, log, printf, tprintf, tprintf_close, tprintf_open};
use crate::file::uio::{Uio, UioRw, UioSeg};
use crate::fs::cache::cache_purge;
use crate::fs::vnode::Vnode;
use crate::net::mbuf::{
    M_COPYALL, M_DONTWAIT, M_WAIT, MT_SONAME, MT_SOOPTS, Mbuf, MbufBox, m_copym, m_get, m_prepend,
};
use crate::net::proto::PR_CONNREQUIRED;
use crate::net::sockbuf::{PSOCK, SB_NOINTR, SS_ISCONNECTED, SS_ISCONNECTING};
use crate::net::socket::{
    Socket, SocketRef, sobind, soclose, soconnect, socreate, solock_net, solock_shared, soreceive,
    soreserve, sosend, sosetopt, soshutdown, sosleep_nsec,
};
use crate::proc::cred::Ucred;
use crate::proc::{ProcGlue, Thread, tfind};
use crate::sched::{NOWAKE, PZERO};
use crate::sig::{sigignore, sigpending};
use crate::sys::errno::Errno;
use crate::sys::mount::{
    NFSMNT_DUMBTIMR, NFSMNT_INT, NFSMNT_NFSV3, NFSMNT_NOCONN, NFSMNT_RCVLOCK, NFSMNT_SNDLOCK,
    NFSMNT_SOFT, NFSMNT_WANTRCV, NFSMNT_WANTSND,
};
use crate::sys::netinet_in::{
    INADDR_ANY, IP_PORTRANGE, IP_PORTRANGE_DEFAULT, IP_PORTRANGE_LOW, IPPROTO_IP, IPPROTO_TCP,
    InAddr, SockaddrIn,
};
use crate::sys::netinet_tcp::TCP_NODELAY;
use crate::sys::nfs_rpcv2::RPCAUTH_UNIX;
use crate::sys::socket::{
    AF_INET, MSG_EOR, MSG_WAITALL, SHUT_RDWR, SO_KEEPALIVE, SOCK_DGRAM, SOCK_STREAM, SOL_SOCKET,
};
use crate::sys::types::Pid;
use crate::time::clock::sec_to_nsec;

/// `NFS_CWNDSCALE`: the scale of the congestion window and of the sent count. There is a
/// congestion window for outstanding rpcs maintained per mount point. The cwnd size is
/// adjusted in roughly the way that: Van Jacobson, Congestion avoidance and Control, In
/// "Proceedings of SIGCOMM '88". ACM, August 1988. describes for TCP. The cwnd size is chopped
/// in half on a retransmit timeout and incremented by 1/cwnd when each rpc reply is received
/// and a full cwnd of rpcs is in progress. (The sent count and cwnd are scaled for integer
/// arith.) Variants of "slow start" were tried and were found to be too much of a performance
/// hit (ave. rtt 3 times larger), I suspect due to the large rtt that nfs rpcs have.
pub const NFS_CWNDSCALE: i32 = 256;

/// `NFS_MAXCWND`: the largest congestion window.
pub const NFS_MAXCWND: i32 = NFS_CWNDSCALE * 32;

/// `nfs_backoff`: the retransmit timeout multipliers, by number of timeouts in a row.
const NFS_BACKOFF: [i32; 8] = [2, 4, 8, 16, 32, 64, 128, 256];

/// `nfs_ptimers`: the RTT estimator of each procedure.
const NFS_PTIMERS: [usize; NFS_NPROCS] = [
    NFS_DEFAULT_TIMER, // NULL
    NFS_GETATTR_TIMER, // GETATTR
    NFS_DEFAULT_TIMER, // SETATTR
    NFS_LOOKUP_TIMER,  // LOOKUP
    NFS_GETATTR_TIMER, // ACCESS
    NFS_READ_TIMER,    // READLINK
    NFS_READ_TIMER,    // READ
    NFS_WRITE_TIMER,   // WRITE
    NFS_DEFAULT_TIMER, // CREATE
    NFS_DEFAULT_TIMER, // MKDIR
    NFS_DEFAULT_TIMER, // SYMLINK
    NFS_DEFAULT_TIMER, // MKNOD
    NFS_DEFAULT_TIMER, // REMOVE
    NFS_DEFAULT_TIMER, // RMDIR
    NFS_DEFAULT_TIMER, // RENAME
    NFS_DEFAULT_TIMER, // LINK
    NFS_READ_TIMER,    // READDIR
    NFS_READ_TIMER,    // READDIRPLUS
    NFS_DEFAULT_TIMER, // FSSTAT
    NFS_DEFAULT_TIMER, // FSINFO
    NFS_DEFAULT_TIMER, // PATHCONF
    NFS_DEFAULT_TIMER, // COMMIT
    NFS_DEFAULT_TIMER, // NOOP
];

/// The arguments of an RPC (`struct nfsm_info`'s `nmi_mreq`, `nmi_procp`, `nmi_cred`): the
/// procedure's arguments, built after the header `nfs_request` makes, the thread that does
/// the system call (`None`: an nfsiod's I/O, the C's NULL), and the credential the
/// `AUTH_UNIX` header carries.
pub struct Request<'a, G: ProcGlue> {
    /// `nmi_mreq`: the arguments.
    pub body: XdrWriter,
    /// `nmi_procp`.
    pub procp: Option<&'a Thread<G>>,
    /// `nmi_cred`.
    pub cred: &'a Ucred,
}

/// The C's `struct nfsreq *rep` as its functions use it: the request, its mount
/// (`rep->r_nmp`) and its requester's thread (`rep->r_procp`), lent by `nfs_request`'s frame.
pub struct Rep<'a, G: ProcGlue> {
    /// `rep->r_nmp`.
    pub nmp: &'a NfsMount<G>,
    /// The request.
    pub rep: &'a NfsReq<G>,
    /// `rep->r_procp`, the requester's own (no lookup).
    pub procp: Option<&'a Thread<G>>,
}

/// What a reply's RPC header says.
enum Header {
    /// Accepted, status 0.
    Ok,
    /// Accepted with a non-zero NFS status.
    Status(u32),
    /// An error with the reply freed (`MSG_DENIED`, a header cut short).
    Bad(Errno),
    /// An error with the reply kept (accepted with a non-zero `accept_stat`:
    /// `EPROTONOSUPPORT`; the C falls through to `nfsmout`).
    Kept(Errno),
}

/// The work one request's timer step stops the walk for, done without the walk's reader.
enum TimerWork<G: ProcGlue> {
    /// "not responding": the message to the requester's terminal, then the request's step
    /// goes on at the retry count's test.
    Msg,
    /// A datagram's retransmission on this socket; the walk goes on with the next request.
    Resend(SocketRef<G>),
}

/// Where `nfs_timer`'s walk starts again.
enum Place<G: ProcGlue> {
    /// At the queue's first request.
    First,
    /// After this request.
    After(PoolArc<NfsReq<G>>),
    /// At this request's retry count test (after its message).
    AtRetry(PoolArc<NfsReq<G>>),
}

/// `nfs_init_rtt(nmp)`: initialize the RTT estimator state for a new mount point.
pub fn nfs_init_rtt<G: ProcGlue>(nmp: &NfsMount<G>) {
    for srtt in &nmp.nm_srtt {
        srtt.store(nfs_initrtt(), Relaxed);
    }
    for sdrtt in &nmp.nm_sdrtt {
        sdrtt.store(0, Relaxed);
    }
}

/// `nfs_update_rtt(rep)`: update a mount point's RTT estimator state using data from the
/// passed-in request.
///
/// Use a gain of 0.125 on the mean and a gain of 0.25 on the deviation.
///
/// NB: Since the timer resolution of `NFS_HZ` is so coarse, it can often result in
/// `r_rtt == 0`. Since `r_rtt == N` means that the actual RTT is between N + dt and N + 2 - dt
/// ticks, add 1 before calculating the update values. (Called only for a timed request, whose
/// procedure has an estimator: `R_TIMING`.)
pub fn nfs_update_rtt<G: ProcGlue>(nmp: &NfsMount<G>, rep: &NfsReq<G>) {
    let Some(index) = NFS_PTIMERS[rep.r_procnum as usize].checked_sub(1) else {
        return;
    };
    let srtt = &nmp.nm_srtt[index];
    let sdrtt = &nmp.nm_sdrtt[index];
    let mut t1 = rep.r_rtt.load(Relaxed).wrapping_add(1);

    t1 = t1.wrapping_sub(srtt.load(Relaxed) >> 3);
    srtt.store(srtt.load(Relaxed).wrapping_add(t1), Relaxed);
    if t1 < 0 {
        t1 = t1.wrapping_neg();
    }
    t1 = t1.wrapping_sub(sdrtt.load(Relaxed) >> 2);
    sdrtt.store(sdrtt.load(Relaxed).wrapping_add(t1), Relaxed);
}

/// `nfs_estimate_rto(nmp, procnum)`: estimate RTO for an NFS RPC sent via an unreliable
/// datagram.
///
/// Use the mean and mean deviation of RTT for the appropriate type of RPC for the frequent
/// RPCs and a default for the others. The justification for doing "other" this way is that
/// these RPCs happen so infrequently that timer est. would probably be stale. Also, since many
/// of these RPCs are non-idempotent, a conservative timeout is desired.
///
/// getattr, lookup - A+2D; read, write - A+4D; other - `nm_timeo`.
pub fn nfs_estimate_rto<G: ProcGlue>(nmp: &NfsMount<G>, procnum: usize) -> i32 {
    let timer = NFS_PTIMERS[procnum];
    let srtt = |i: usize| nmp.nm_srtt[i].load(Relaxed);
    let sdrtt = |i: usize| nmp.nm_sdrtt[i].load(Relaxed);
    let rto = match timer {
        NFS_GETATTR_TIMER | NFS_LOOKUP_TIMER => {
            let index = timer - 1;
            (srtt(index).wrapping_add(3) >> 2).wrapping_add(sdrtt(index).wrapping_add(1) >> 1)
        }
        NFS_READ_TIMER | NFS_WRITE_TIMER => {
            let index = timer - 1;
            (srtt(index).wrapping_add(7) >> 3).wrapping_add(sdrtt(index).wrapping_add(1))
        }
        _ => return nmp.nm_timeo.load(Relaxed),
    };

    if rto < nfs_minrto() {
        nfs_minrto()
    } else if rto > nfs_maxrto() {
        nfs_maxrto()
    } else {
        rto
    }
}

/// `nmp->nm_so`, counted: a handle cloned out of the mount under a reader of the kernel lock
/// (a send or a receive then holds no witness).
fn nm_so<G: ProcGlue>(nmp: &NfsMount<G>) -> Option<SocketRef<G>> {
    let r = klock::reader();
    nmp.nm_so.get(&r).clone()
}

/// `nmp->nm_so = so`: the old socket's handle given back (dropped by the caller, outside the
/// witness).
fn set_nm_so<G: ProcGlue>(nmp: &NfsMount<G>, so: Option<SocketRef<G>>) -> Option<SocketRef<G>> {
    let mut w = klock::witness();
    core::mem::replace(nmp.nm_so.get_mut(&mut w), so)
}

/// `*mtod(m, int *) = v; m->m_len = sizeof(int)`: an option's value for `sosetopt`.
fn set_mtod_int(m: &mut Mbuf, v: i32) {
    m.m_len().set(size_of::<i32>() as u32);
    if let Some(d) = m.data_mut() {
        d.copy_from_slice(&v.to_ne_bytes());
    }
}

/// The C's `int` socket buffer size given to `soreserve`'s `u_long` (sign-extended, as the
/// C's conversion: a negative size is refused there as too large).
fn reserve_size(v: i32) -> u64 {
    i64::from(v) as u64
}

/// `nfs_connect(nmp, rep)`: initialize sockets and congestion for a new NFS connection. We do
/// not free the sockaddr if error. The socket is the running thread's (`socreate`'s
/// `curproc`), its reserved port bound with `proc0`'s privilege.
pub fn nfs_connect<G: ProcGlue>(nmp: &NfsMount<G>, rep: Option<&Rep<'_, G>>) -> Result<(), Errno> {
    let sotype = nmp.nm_sotype.load(Relaxed);
    if !(sotype == SOCK_DGRAM || sotype == SOCK_STREAM) {
        return Err(Errno::EINVAL);
    }

    drop(set_nm_so(nmp, None));
    let family = nmp.nm_nam.family();
    let so = match G::with_curproc(|cur| match cur {
        Some(p) => socreate(p, i32::from(family), sotype, nmp.nm_soproto.load(Relaxed)),
        None => kpanic(format_args!("nfs_connect: no curproc")),
    }) {
        Ok(so) => so,
        Err(error) => {
            nfs_disconnect(nmp);
            return Err(error);
        }
    };
    drop(set_nm_so(nmp, Some(so.clone())));

    match nfs_connect_so(nmp, &so, rep, family) {
        Ok(()) => {
            // Initialize other non-zero congestion variables.
            nfs_init_rtt(nmp);
            nmp.nm_cwnd.store(NFS_MAXCWND / 2, Relaxed); // Initial send window
            nmp.nm_sent.store(0, Relaxed);
            nmp.nm_timeouts.store(0, Relaxed);
            Ok(())
        }
        Err(error) => {
            drop(so);
            nfs_disconnect(nmp);
            Err(error)
        }
    }
}

/// The part of `nfs_connect` between the socket's creation and the `bad:` label: binds a
/// reserved port, connects, sets the timeouts and the buffer sizes.
fn nfs_connect_so<G: ProcGlue>(
    nmp: &NfsMount<G>,
    so: &Socket<G>,
    rep: Option<&Rep<'_, G>>,
    family: u8,
) -> Result<(), Errno> {
    // Allocate mbufs possibly waiting before grabbing the socket lock.
    let mut mopt = None;
    if nmp.nm_sotype.load(Relaxed) == SOCK_STREAM || family == AF_INET {
        mopt = Some(m_get(M_WAIT, MT_SOOPTS).ok_or(Errno::ENOBUFS)?);
    }
    let mut nam = None;
    if family == AF_INET {
        nam = Some(m_get(M_WAIT, MT_SONAME).ok_or(Errno::ENOBUFS)?);
    }

    nmp.nm_soflags.store(i32::from(so.proto().flags()), Relaxed);

    // Some servers require that the client port be a reserved port number. We always
    // allocate a reserved port, as this prevents filehandle disclosure through UDP port
    // capture.
    if let (Some(mopt), Some(nam)) = (mopt.as_deref_mut(), nam.as_deref_mut()) {
        G::with_curproc(|cur| -> Result<(), Errno> {
            let Some(p) = cur else {
                kpanic(format_args!("nfs_connect: no curproc"));
            };
            set_mtod_int(mopt, IP_PORTRANGE_LOW);
            sosetopt(so, p, IPPROTO_IP, IP_PORTRANGE, Some(&mut *mopt))?;

            let sin = SockaddrIn {
                sin_len: size_of::<SockaddrIn>() as u8,
                sin_family: AF_INET,
                sin_port: 0u16.to_be(),
                sin_addr: InAddr { s_addr: INADDR_ANY },
                ..SockaddrIn::default()
            };
            let b = super::xdr_subs::xdr_bytes(&sin);
            nam.m_len().set(b.len() as u32);
            if let Some(d) = nam.data_mut() {
                d.copy_from_slice(b);
            }
            let proc0 = G::tables().proc0();
            {
                let mut w = solock_shared(so);
                sobind(so, &mut w, nam, &proc0)?;
            }

            set_mtod_int(mopt, IP_PORTRANGE_DEFAULT);
            sosetopt(so, p, IPPROTO_IP, IP_PORTRANGE, Some(&mut *mopt))
        })?;
    }

    // Protocols that do not require connections may be optionally left unconnected for
    // servers that reply from a port other than NFS_PORT.
    if nmp.flag() & NFSMNT_NOCONN != 0 {
        if nmp.nm_soflags.load(Relaxed) & i32::from(PR_CONNREQUIRED) != 0 {
            return Err(Errno::ENOTCONN);
        }
    } else {
        let mut w = solock_shared(so);
        nmp.with_nam(M_WAIT, |srv| match srv {
            Some(srv) => soconnect(so, &mut w, srv),
            None => Err(Errno::ENOBUFS),
        })?;

        // Wait for the connection to complete. Cribbed from the connect system call but with
        // the wait timing out so that interruptible mounts don't hang here for a long time.
        while so.has_state(SS_ISCONNECTING) && so.error().is_none() {
            let _ = sosleep_nsec(&mut w, so.timeo_chan(), PSOCK, "nfscon", sec_to_nsec(2));
            if so.has_state(SS_ISCONNECTING)
                && so.error().is_none()
                && let Some(r) = rep
                && let Err(error) = nfs_sigintr(nmp, Some(r.rep), r.procp)
            {
                so.st_clear(SS_ISCONNECTING);
                return Err(error);
            }
        }
        if let Some(error) = so.error() {
            so.set_error(None);
            return Err(error);
        }
    }

    // Always set receive timeout to detect server crash and reconnect. Otherwise, we can get
    // stuck in soreceive forever.
    so.so_rcv.lock().set_timeo_nsecs(sec_to_nsec(5));
    so.so_snd
        .lock()
        .set_timeo_nsecs(if nmp.flag() & (NFSMNT_SOFT | NFSMNT_INT) != 0 {
            sec_to_nsec(5)
        } else {
            INFSLP
        });
    let (sndreserve, rcvreserve) = match nmp.nm_sotype.load(Relaxed) {
        SOCK_DGRAM => {
            let snd = nmp.nm_wsize.load(Relaxed) + NFS_MAXPKTHDR as i32;
            let rcv = (nmp
                .nm_rsize
                .load(Relaxed)
                .max(nmp.nm_readdirsize.load(Relaxed))
                + NFS_MAXPKTHDR as i32)
                * 2;
            (snd, rcv)
        }
        SOCK_STREAM => {
            G::with_curproc(|cur| {
                let (Some(p), Some(mopt)) = (cur, mopt.as_deref_mut()) else {
                    return;
                };
                if so.pr_flags(PR_CONNREQUIRED) {
                    set_mtod_int(mopt, 1);
                    let _ = sosetopt(so, p, SOL_SOCKET, SO_KEEPALIVE, Some(&mut *mopt));
                }
                if i32::from(so.proto().protocol()) == IPPROTO_TCP {
                    set_mtod_int(mopt, 1);
                    let _ = sosetopt(so, p, IPPROTO_TCP, TCP_NODELAY, Some(&mut *mopt));
                }
            });
            let u = NFSX_UNSIGNED as i32;
            let snd = (nmp.nm_wsize.load(Relaxed) + NFS_MAXPKTHDR as i32 + u) * 2;
            let rcv = (nmp.nm_rsize.load(Relaxed) + NFS_MAXPKTHDR as i32 + u) * 2;
            (snd, rcv)
        }
        t => kpanic(format_args!("nfs_connect: nm_sotype {t}")),
    };
    let w = solock_shared(so);
    soreserve(so, reserve_size(sndreserve), reserve_size(rcvreserve))?;
    so.so_rcv.lock().set_flags(SB_NOINTR);
    so.so_snd.lock().set_flags(SB_NOINTR);
    drop(w);
    Ok(())
}

/// `nfs_reconnect(rep)`: called when a connection is broken on a reliable protocol: clean up
/// the old socket, `nfs_connect()` again, set `R_MUSTRESEND` for all outstanding requests on
/// mount point. If this fails the mount point is DEAD! nb: Must be called with the
/// `nfs_sndlock()` set on the mount point.
pub fn nfs_reconnect<G: ProcGlue>(rep: &Rep<'_, G>) -> Result<(), Errno> {
    let nmp = rep.nmp;

    nfs_disconnect(nmp);
    loop {
        match nfs_connect(nmp, Some(rep)) {
            Ok(()) => break,
            Err(Errno::EINTR | Errno::ERESTART) => return Err(Errno::EINTR),
            Err(_) => {
                let _ = tsleep_nsec(&raw const NOWAKE, PSOCK, "nfsrecon", sec_to_nsec(1));
            }
        }
    }

    // Loop through outstanding request list and fix up all requests on old socket.
    let r = klock::reader();
    for rp in nmp.nm_reqsq.iter(&r) {
        rp.set_flags(R_MUSTRESEND);
        rp.r_rexmit.store(0, Relaxed);
    }
    Ok(())
}

/// `nfs_disconnect(nmp)`: NFS disconnect. Clean up and unlink.
pub fn nfs_disconnect<G: ProcGlue>(nmp: &NfsMount<G>) {
    if let Some(so) = set_nm_so(nmp, None) {
        let _ = soshutdown(&so, SHUT_RDWR);
        let _ = soclose(&so, 0);
    }
}

/// `nfs_send(so, nam, top, rep)`: this is the nfs send routine. For connection based socket
/// types, it must be called with an `nfs_sndlock()` on the socket. `rep == None` indicates
/// that it has been called from a server, with its socket `so` and its client's address
/// `nam`; the client gives `None` for both, the mount's socket and address being read here
/// (the C reads `rep->r_nmp->nm_so` over its argument).
///
/// For the client side: return `EINTR` if the RPC is terminated, 0 otherwise; set
/// `R_MUSTRESEND` if the send fails for any reason; do any cleanup required by recoverable
/// socket errors (???).
///
/// For the server side: return `EINTR` or `ERESTART` if interrupted by a signal; return
/// `EPIPE` if a connection is lost for connection based sockets (TCP...); do any cleanup
/// required by recoverable socket errors (???).
pub fn nfs_send<G: ProcGlue>(
    so: Option<&Socket<G>>,
    nam: Option<&Mbuf>,
    top: MbufBox,
    rep: Option<&Rep<'_, G>>,
) -> Result<(), Errno> {
    let held;
    let (so, soflags) = match rep {
        Some(r) => {
            if r.rep.flags() & R_SOFTTERM != 0 {
                drop(top);
                return Err(Errno::EINTR);
            }
            let Some(s) = nm_so(r.nmp) else {
                r.rep.set_flags(R_MUSTRESEND);
                drop(top);
                return Ok(());
            };
            held = s;
            r.rep.clear_flags(R_MUSTRESEND);
            (&*held, r.nmp.nm_soflags.load(Relaxed))
        }
        None => match so {
            Some(so) => (so, i32::from(so.proto().flags())),
            None => kpanic(format_args!("nfs_send: no socket")),
        },
    };
    let connected = soflags & i32::from(PR_CONNREQUIRED) != 0 || so.has_state(SS_ISCONNECTED);
    let flags = 0;

    let sent = match rep {
        // The client's unconnected socket: the mount's address, lent.
        Some(r) if !connected => r
            .nmp
            .with_nam(M_WAIT, |n| sosend(so, n, None, Some(top), None, flags)),
        _ => sosend(
            so,
            if connected { None } else { nam },
            None,
            Some(top),
            None,
            flags,
        ),
    };
    let Err(mut error) = sent else {
        return Ok(());
    };
    if let Some(r) = rep {
        // Deal with errors for the client side.
        if r.rep.flags() & R_SOFTTERM != 0 {
            error = Errno::EINTR;
        } else {
            r.rep.set_flags(R_MUSTRESEND);
        }
    }

    // Handle any recoverable (soft) socket errors here. (???)
    if error != Errno::EINTR
        && error != Errno::ERESTART
        && error != Errno::EWOULDBLOCK
        && error != Errno::EPIPE
    {
        return Ok(());
    }
    Err(error)
}

/// A copy of the request's chain for a send (`m_copym(rep->r_mreq, 0, M_COPYALL, how)`):
/// made under a witness section without waiting; for `M_WAIT`, when that fails, the chain is
/// taken out of its cell for a copy that may sleep and put back after (the module's
/// deviation). `None` when no copy is to be had (`M_DONTWAIT`) or the chain is out.
fn nfs_copy_mreq<G: ProcGlue>(rep: &NfsReq<G>, how: i32) -> Option<MbufBox> {
    let copy = {
        let mut w = klock::witness();
        rep.r_m
            .get_mut(&mut w)
            .r_mreq
            .as_mut()
            .and_then(|m| m_copym(m, 0, M_COPYALL, M_DONTWAIT))
    };
    if copy.is_some() || how != M_WAIT {
        return copy;
    }
    let mut m = {
        let mut w = klock::witness();
        rep.r_m.get_mut(&mut w).r_mreq.take()?
    };
    let copy = m_copym(&mut m, 0, M_COPYALL, M_WAIT);
    let mut w = klock::witness();
    rep.r_m.get_mut(&mut w).r_mreq = Some(m);
    copy
}

/// `nfs_receive(rep, aname, mp)`: receive a Sun RPC Request/Reply. For `SOCK_DGRAM`, the
/// work is all done by `soreceive()`, but for `SOCK_STREAM` we must deal with the Record Mark
/// and consolidate the data into a new mbuf list. nb: Sometimes TCP passes the data up to
/// `soreceive()` in long lists of small mbufs. For `SOCK_STREAM` we must be very careful to
/// read an entire record once we have read any of it, even if the system call has been
/// interrupted. The reply's chain, `None` for a datagram without data; the sender's address
/// (`*aname`), which the C's caller frees at once, is dropped here.
pub fn nfs_receive<G: ProcGlue>(rep: &Rep<'_, G>) -> Result<Option<MbufBox>, Errno> {
    G::with_curproc(|p| {
        if rep.nmp.nm_sotype.load(Relaxed) == SOCK_DGRAM {
            nfs_receive_dgram(rep, p)
        } else {
            nfs_receive_stream(rep, p) // XXX: the C's `p = curproc`
        }
    })
}

/// A uio of `resid` bytes and no iovec: `soreceive`'s count when the data comes back as a
/// chain (`mp0`).
pub(crate) fn count_uio<G: ProcGlue>(resid: usize, p: Option<&Thread<G>>) -> Uio<'_, G> {
    Uio {
        uio_iov: &mut [],
        uio_offset: 0,
        uio_resid: resid,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: UioRw::UIO_READ,
        uio_procp: p,
    }
}

/// `nfs_receive`'s `SOCK_DGRAM` branch.
fn nfs_receive_dgram<G: ProcGlue>(
    rep: &Rep<'_, G>,
    p: Option<&Thread<G>>,
) -> Result<Option<MbufBox>, Errno> {
    let Some(so) = nm_so(rep.nmp) else {
        return Err(Errno::EACCES);
    };
    let getnam = !so.has_state(SS_ISCONNECTED);
    let mut auio = count_uio(1_000_000, p);
    let mut mp = None;
    let mut aname = None;
    let error = loop {
        let mut rcvflg = 0;
        let paddr = if getnam { Some(&mut aname) } else { None };
        let error = soreceive(
            &so,
            paddr,
            &mut auio,
            Some(&mut mp),
            None,
            Some(&mut rcvflg),
            0,
        );
        if error == Err(Errno::EWOULDBLOCK) && rep.rep.flags() & R_SOFTTERM != 0 {
            return Err(Errno::EINTR);
        }
        if error != Err(Errno::EWOULDBLOCK) {
            break error;
        }
    };
    drop(aname);
    error.map(|()| mp)
}

/// `nfs_receive`'s branch for the reliable protocols: the send lock around a reconnect and
/// the resends, then the record (`SOCK_STREAM`: its mark first) or the message.
fn nfs_receive_stream<G: ProcGlue>(
    rep: &Rep<'_, G>,
    p: Option<&Thread<G>>,
) -> Result<Option<MbufBox>, Errno> {
    let nmp = rep.nmp;
    let sotype = nmp.nm_sotype.load(Relaxed);
    // For reliable protocols, lock against other senders/receivers in case a reconnect is
    // necessary. For SOCK_STREAM, first get the Record Mark to find out how much more there
    // is to get. We must lock the socket against other receivers until we have an entire rpc
    // request/reply.
    nfs_sndlock(&nmp.nm_flag, Some(rep))?;
    'tryagain: loop {
        // Check for fatal errors and resending request.
        //
        // Ugh: If a reconnect attempt just happened, nm_so would have changed. NULL indicates
        // a failed attempt that has essentially shut down this mount point.
        if rep.rep.has_reply() || rep.rep.flags() & R_SOFTTERM != 0 {
            nfs_sndunlock(&nmp.nm_flag);
            return Err(Errno::EINTR);
        }
        let Some(so) = nm_so(nmp) else {
            if let Err(error) = nfs_reconnect(rep) {
                nfs_sndunlock(&nmp.nm_flag);
                return Err(error);
            }
            continue 'tryagain;
        };
        while rep.rep.flags() & R_MUSTRESEND != 0 {
            let Some(m) = nfs_copy_mreq(rep.rep, M_WAIT) else {
                nfs_sndunlock(&nmp.nm_flag);
                return Err(Errno::ENOBUFS);
            };
            NFSSTATS.rpcretries.fetch_add(1, Relaxed);
            rep.rep.r_rtt.store(0, Relaxed);
            rep.rep.clear_flags(R_TIMING);
            if let Err(error) = nfs_send(None, None, m, Some(rep)) {
                if error == Errno::EINTR || error == Errno::ERESTART {
                    nfs_sndunlock(&nmp.nm_flag);
                    return Err(error);
                }
                if let Err(error) = nfs_reconnect(rep) {
                    nfs_sndunlock(&nmp.nm_flag);
                    return Err(error);
                }
                continue 'tryagain;
            }
        }
        nfs_sndunlock(&nmp.nm_flag);
        let received = if sotype == SOCK_STREAM {
            nfs_receive_record(rep, &so, p)
        } else {
            nfs_receive_message(rep, &so, p)
        };
        // errout:
        match received {
            Ok(m) => return Ok(m),
            Err(error) if error == Errno::EINTR || error == Errno::ERESTART => {
                return Err(error);
            }
            Err(error) => {
                if error != Errno::EPIPE {
                    log(
                        LOG_INFO,
                        format_args!(
                            "receive error {} from nfs server {}\n",
                            error as i32,
                            Str(&nmp.mntfromname())
                        ),
                    );
                }
                nfs_sndlock(&nmp.nm_flag, Some(rep))?;
                if let Err(error) = nfs_reconnect(rep) {
                    nfs_sndunlock(&nmp.nm_flag);
                    return Err(error);
                }
                // goto tryagain: the loop goes round.
            }
        }
    }
}

/// `nfs_receive`'s `SOCK_STREAM` record: the 4-byte Record Mark, its length bounded by
/// `NFS_MAXPACKET` (a disconnect and a reconnect past it), then the record as a chain.
fn nfs_receive_record<G: ProcGlue>(
    rep: &Rep<'_, G>,
    so: &Socket<G>,
    p: Option<&Thread<G>>,
) -> Result<Option<MbufBox>, Errno> {
    let nmp = rep.nmp;
    let mut mark = [0u8; NFSX_UNSIGNED];
    let mut aio = [crate::file::uio::Iovec::new()];
    let short = {
        let mut auio = Uio::kernel_read(&mut aio, &mut mark, p);
        let mut error;
        loop {
            let mut rcvflg = MSG_WAITALL;
            error = soreceive(so, None, &mut auio, None, None, Some(&mut rcvflg), 0);
            if error == Err(Errno::EWOULDBLOCK) {
                if rep.rep.flags() & R_SOFTTERM != 0 {
                    return Err(Errno::EINTR);
                }
                // looks like the server died after it received the request, make sure that
                // we will retransmit and we don't get stuck here forever.
                if rep.rep.r_rexmit.load(Relaxed) >= nmp.nm_retry.load(Relaxed) {
                    NFSSTATS.rpctimeouts.fetch_add(1, Relaxed);
                    error = Err(Errno::EPIPE);
                }
            }
            if error != Err(Errno::EWOULDBLOCK) {
                break;
            }
        }
        error?;
        auio.uio_resid
    };
    if short > 0 {
        log(
            LOG_INFO,
            format_args!(
                "short receive ({}/{}) from nfs server {}\n",
                NFSX_UNSIGNED - short,
                NFSX_UNSIGNED,
                Str(&nmp.mntfromname())
            ),
        );
        return Err(Errno::EPIPE);
    }

    let len = u32::from_be_bytes(mark) & !0x8000_0000;
    // This is SERIOUS! We are out of sync with the sender and forcing a disconnect/reconnect
    // is all I can do.
    if len as usize > NFS_MAXPACKET {
        log(
            LOG_ERR,
            format_args!(
                "{} ({}) from nfs server {}\n",
                "impossible packet length",
                len,
                Str(&nmp.mntfromname())
            ),
        );
        return Err(Errno::EFBIG);
    }
    let len = len as usize;
    let mut auio = count_uio(len, p);
    let mut mp = None;
    let error = loop {
        let mut rcvflg = MSG_WAITALL;
        let error = soreceive(
            so,
            None,
            &mut auio,
            Some(&mut mp),
            None,
            Some(&mut rcvflg),
            0,
        );
        if !matches!(
            error,
            Err(Errno::EWOULDBLOCK | Errno::EINTR | Errno::ERESTART)
        ) {
            break error;
        }
    };
    error?;
    if auio.uio_resid > 0 {
        log(
            LOG_INFO,
            format_args!(
                "short receive ({}/{}) from nfs server {}\n",
                len - auio.uio_resid,
                len,
                Str(&nmp.mntfromname())
            ),
        );
        return Err(Errno::EPIPE);
    }
    Ok(mp)
}

/// `nfs_receive`'s branch for a reliable protocol that is no stream: one message, its control
/// messages thrown away.
fn nfs_receive_message<G: ProcGlue>(
    rep: &Rep<'_, G>,
    so: &Socket<G>,
    p: Option<&Thread<G>>,
) -> Result<Option<MbufBox>, Errno> {
    // NB: Since uio_resid is big, MSG_WAITALL is ignored and soreceive() will return when it
    // has either a control msg or a data msg. We have no use for control msg., but must grab
    // them and then throw them away so we know what is going on.
    let mut auio = count_uio(100_000_000, p); // Anything Big
    let mut mp = None;
    let mut rcvflg;
    let error = loop {
        rcvflg = 0;
        let mut control = None;
        let error = soreceive(
            so,
            None,
            &mut auio,
            Some(&mut mp),
            Some(&mut control),
            Some(&mut rcvflg),
            0,
        );
        let had_control = control.is_some();
        drop(control);
        if error == Err(Errno::EWOULDBLOCK) && rep.rep.flags() & R_SOFTTERM != 0 {
            return Err(Errno::EINTR);
        }
        if !(error == Err(Errno::EWOULDBLOCK) || (error.is_ok() && mp.is_none() && had_control)) {
            break error;
        }
    };
    if rcvflg & MSG_EOR == 0 {
        printf(format_args!("Egad!!\n"));
    }
    error?;
    if mp.is_none() {
        return Err(Errno::EPIPE);
    }
    Ok(mp)
}

/// `nfs_reply(myrep)`: implement receipt of reply on a socket. We must search through the
/// list of received datagrams matching them with outstanding requests using the xid, until
/// ours is found.
pub fn nfs_reply<G: ProcGlue>(myrep: &Rep<'_, G>) -> Result<(), Errno> {
    let nmp = myrep.nmp;

    // Loop around until we get our own reply.
    loop {
        // Lock against other receivers so that I don't get stuck in sbwait() after someone
        // else has received my reply for me. Also necessary for connection based protocols
        // to avoid race conditions during a reconnect.
        match nfs_rcvlock(myrep) {
            Ok(()) => {}
            Err(Errno::EALREADY) => return Ok(()),
            Err(error) => return Err(error),
        }

        // Get the next Rpc reply off the socket.
        let received = nfs_receive(myrep);
        nfs_rcvunlock(&nmp.nm_flag);
        let mrep = match received {
            Ok(m) => m,
            Err(error) => {
                // Ignore routing errors on connectionless protocols??
                if nfsignore_soerror(nmp.nm_soflags.load(Relaxed), error) {
                    if let Some(so) = nm_so(nmp) {
                        so.set_error(None);
                    }
                    continue;
                }
                return Err(error);
            }
        };
        let Some(mrep) = mrep else {
            continue; // no data: nfsmout
        };

        // Get the xid and check that it is an rpc reply.
        let (rxid, kind, off) = {
            let mut x = XdrReader::new(&mrep);
            match (x.word(), x.word()) {
                (Ok(rxid), Ok(kind)) => (rxid, kind, x.offset()),
                _ => continue, // nfsmout: the reply freed
            }
        };
        if kind != rpc_reply {
            NFSSTATS.rpcinvalid.fetch_add(1, Relaxed);
            continue; // the reply freed
        }

        if nfs_reply_match(nmp, myrep.rep, rxid, mrep, off) {
            if !myrep.rep.has_reply() {
                kpanic(format_args!("nfsreply nil"));
            }
            return Ok(());
        }
    }
}

/// `nfs_reply`'s match of a reply (its xid `rxid`, the chain `mrep`, its first `off` bytes
/// read) to the mount's requests: loop through the request list to match up the reply; the
/// first request with no reply and that xid takes it (the congestion window, the sent count,
/// the RTT estimator and the timeouts' count updated). Iff no match, just drop the datagram
/// (`rpcunexpected`). Whether it was `myrep`'s.
fn nfs_reply_match<G: ProcGlue>(
    nmp: &NfsMount<G>,
    myrep: &NfsReq<G>,
    rxid: u32,
    mrep: MbufBox,
    off: usize,
) -> bool {
    let (found, unmatched) = {
        let mut w = klock::witness();
        let [list, mut part] = w.split::<2>();
        let mut mrep = Some(mrep);
        let mut found = None;
        for rep in nmp.nm_reqsq.iter(&list) {
            if !rep.has_reply() && rxid == rep.r_xid {
                // Found it..
                drop(rep.set_reply(&mut part, mrep.take().map(|m| Reply::new(m, off))));

                // Update congestion window. Do the additive increase of one rpc/rtt.
                let cwnd = nmp.nm_cwnd.load(Relaxed);
                if cwnd <= nmp.nm_sent.load(Relaxed) {
                    let cwnd = cwnd + (NFS_CWNDSCALE * NFS_CWNDSCALE + (cwnd >> 1)) / cwnd;
                    nmp.nm_cwnd.store(cwnd.min(NFS_MAXCWND), Relaxed);
                }
                rep.clear_flags(R_SENT);
                nmp.nm_sent.fetch_sub(NFS_CWNDSCALE, Relaxed);

                if rep.flags() & R_TIMING != 0 {
                    nfs_update_rtt(nmp, rep);
                }

                nmp.nm_timeouts.store(0, Relaxed);
                found = Some(ptr::eq(rep, myrep));
                break;
            }
        }
        (found, mrep)
    };
    // If not matched to a request, drop it. If it's mine, get out.
    if found.is_none() {
        NFSSTATS.rpcunexpected.fetch_add(1, Relaxed);
    }
    drop(unmatched);
    found == Some(true)
}

/// `nfs_request(vp, procnum, infop)`: goes something like this: fill in request struct;
/// links it into list; calls `nfs_send()` for first transmit; calls `nfs_receive()` to get
/// reply; break down rpc header and return with nfs reply pointed to by `mrep` or error.
/// nb: always frees up mreq mbuf list.
///
/// The C's two outputs: the reply, its RPC header consumed, on success and on a status
/// error (version 3's post-op attributes are read from it), `None` with any other error; and
/// the error.
///
/// `nmp` is `VFSTONFS(vp->v_mount)`, which the vnode operation holds already: lent here (the
/// C casts it again; the kernel's handle would be one more count pair per RPC).
pub fn nfs_request<G: ProcGlue>(
    nmp: &NfsMount<G>,
    vp: &'static Vnode<G>,
    procnum: usize,
    req: Request<'_, G>,
) -> (Option<Reply>, Result<(), NfsError>) {
    let Request { body, procp, cred } = req;
    kassert!(vp.mount().is_some_and(|mp| ptr::eq(mp, nmp.nm_mountp)));

    // The RPC header with authorization, an empty packet header mbuf (the AUTH_UNIX header)
    // before the arguments.
    let Some((mut m, xid)) = nfsm_rpchead(nmp, procnum, cred, RPCAUTH_UNIX, body.into_chain())
    else {
        return (None, Err(Errno::ENOBUFS.into()));
    };

    // For stream protocols, insert a Sun RPC Record Mark.
    if nmp.nm_sotype.load(Relaxed) == SOCK_STREAM {
        let Some(mut mm) = m_prepend(m, NFSX_UNSIGNED as i32, M_WAIT) else {
            // m_prepend freed the whole request.
            return (None, Err(Errno::ENOBUFS.into()));
        };
        let mark =
            0x8000_0000 | (mm.m_pkthdr().len.get() as u32).wrapping_sub(NFSX_UNSIGNED as u32);
        if let Some(d) = mm.data_mut() {
            d[..NFSX_UNSIGNED].copy_from_slice(&mark.to_be_bytes());
        }
        m = mm;
    }

    let procp_tid = procp.map(|p| p.p_tid.get());
    let Some(rep) = crate::fs::subr::vfs::<G>()
        .nfs
        .nfsreqpl
        .get_homed(PR_WAITOK, || {
            NfsReq::new(procnum as u32, xid, m, procp_tid, ptr::from_ref(vp).addr())
        })
    else {
        return (None, Err(Errno::ENOBUFS.into()));
    };
    let r = Rep {
        nmp,
        rep: &rep,
        procp,
    };

    let mut trylater_delay = nfs_mintimeo();
    let v3 = nmp.flag() & NFSMNT_NFSV3 != 0;

    loop {
        // tryagain:
        rep.r_rtt.store(0, Relaxed);
        rep.r_rexmit.store(0, Relaxed);
        rep.r_flags.store(
            if NFS_PTIMERS[procnum] == NFS_DEFAULT_TIMER {
                0
            } else {
                R_TIMING
            },
            Relaxed,
        );

        // Do the client side RPC.
        NFSSTATS.rpcrequests.fetch_add(1, Relaxed);
        // Chain request into list of outstanding requests. Be sure to put it LAST so timer
        // finds oldest requests first. If backing off another request or avoiding congestion,
        // don't send this one now but let timer do it. If not timing a request, do it now:
        // its copy made in the same section.
        let (send, copy) = {
            let mut w = klock::witness();
            drop(rep.set_reply(&mut w, None));
            if nmp.nm_reqsq.is_empty(&w) {
                nmp.nm_rtimeout.as_pin().add(NFS_TICKS.load(Relaxed));
            }
            nmp.nm_reqsq.insert_tail(&mut w, rep.as_pin());
            let send = nmp.nm_so.get(&w).is_some()
                && (nmp.nm_sotype.load(Relaxed) != SOCK_DGRAM
                    || nmp.flag() & NFSMNT_DUMBTIMR != 0
                    || nmp.nm_sent.load(Relaxed) < nmp.nm_cwnd.load(Relaxed));
            let copy = if send {
                rep.r_m
                    .get_mut(&mut w)
                    .r_mreq
                    .as_mut()
                    .and_then(|m| m_copym(m, 0, M_COPYALL, M_DONTWAIT))
            } else {
                None
            };
            (send, copy)
        };

        let mut error = Ok(());
        if send {
            let connreq = nmp.nm_soflags.load(Relaxed) & i32::from(PR_CONNREQUIRED) != 0;
            if connreq {
                error = nfs_sndlock(&nmp.nm_flag, Some(&r));
            }
            if error.is_ok() {
                match copy.or_else(|| nfs_copy_mreq(&rep, M_WAIT)) {
                    Some(c) => error = nfs_send(None, None, c, Some(&r)),
                    None => rep.set_flags(R_MUSTRESEND),
                }
                if connreq {
                    nfs_sndunlock(&nmp.nm_flag);
                }
            }
            if error.is_ok() && rep.flags() & R_MUSTRESEND == 0 {
                nmp.nm_sent.fetch_add(NFS_CWNDSCALE, Relaxed);
                rep.set_flags(R_SENT);
            }
        } else {
            drop(copy);
            rep.r_rtt.store(-1, Relaxed);
        }

        // Wait for the reply from our send or the timer's.
        if error.is_ok() || error == Err(Errno::EPIPE) {
            error = nfs_reply(&r);
        }

        // RPC done, unlink the request.
        let reply = {
            let mut w = klock::witness();
            nmp.nm_reqsq.remove(&mut w, &rep);
            if nmp.nm_reqsq.is_empty(&w) {
                nmp.nm_rtimeout.del();
            }
            rep.set_reply(&mut w, None)
        };

        // Decrement the outstanding request count.
        if rep.flags() & R_SENT != 0 {
            rep.clear_flags(R_SENT); // paranoia
            nmp.nm_sent.fetch_sub(NFS_CWNDSCALE, Relaxed);
        }

        // If there was a successful reply and a tprintf msg. tprintf a response.
        if error.is_ok() && rep.flags() & R_TPRINTFMSG != 0 {
            nfs_msg(nmp, procp, "is alive again");
        }
        if let Err(error) = error {
            drop(reply);
            return (None, Err(error.into()));
        }
        let Some(mut reply) = reply else {
            kpanic(format_args!("nfsreply nil"));
        };

        // break down the rpc header and check if ok
        match nfs_reply_header(&mut reply) {
            Header::Ok => return (Some(reply), Ok(())),
            Header::Bad(error) => return (None, Err(error.into())),
            Header::Kept(error) => return (Some(reply), Err(error.into())),
            Header::Status(status) => {
                let error = NfsError::from_status(status);
                if v3 && error.raw() == NFSERR_TRYLATER {
                    drop(reply);
                    let _ = tsleep_nsec(
                        &raw const NOWAKE,
                        PSOCK,
                        "nfsretry",
                        sec_to_nsec(u64::try_from(trylater_delay).unwrap_or(0)),
                    );
                    trylater_delay = nfs_trylater_next(trylater_delay);
                    continue; // tryagain
                }

                // If the File Handle was stale, invalidate the lookup cache, just in case.
                if error == NfsError::Errno(Errno::ESTALE) {
                    cache_purge(vp);
                }
                return (Some(reply), Err(error));
            }
        }
    }
}

/// `trylater_delay *= NFS_TIMEOUTMUL`, capped at `NFS_MAXTIMEO`: the next wait of a request a
/// version 3 server asked to try later (`NFSERR_TRYLATER`), in seconds (the C sleeps
/// `SEC_TO_NSEC` of a count of `NFS_HZ` ticks: kept).
pub fn nfs_trylater_next(delay: i32) -> i32 {
    delay.saturating_mul(NFS_TIMEOUTMUL).min(nfs_maxtimeo())
}

/// `nfs_request`'s dissection of a reply's RPC header after the xid and the message type:
/// the reply status, the verifier (skipped by its length, rounded up, when positive), the
/// accept status and the NFS status; `reply`'s offset left past them.
fn nfs_reply_header(reply: &mut Reply) -> Header {
    let (h, off) = {
        let Ok(mut x) = reply.reader() else {
            return Header::Bad(Errno::EBADRPC);
        };
        let h = (|| -> Result<Header, Errno> {
            let stat = x.word()?;
            let w1 = x.word()?;
            let vlen = x.word()?;
            if stat == rpc_msgdenied {
                return Ok(Header::Bad(if w1 == rpc_mismatch {
                    Errno::EOPNOTSUPP
                } else {
                    Errno::EACCES // Should be EAUTH.
                }));
            }

            // Since we only support RPCAUTH_UNIX atm we step over the reply verifier type
            // (w1), and in the (error) case that there really is any data in it, we advance
            // over it.
            let i = u32::from_be(vlen) as i32;
            if i > 0 {
                // Should not happen; the skip is bounded by the chain.
                x.adv(nfsm_rndup(i as usize))?;
            }

            // 0 == ok
            if x.word()? == 0 {
                let st = x.word()?;
                if st != 0 {
                    return Ok(Header::Status(u32::from_be(st)));
                }
                return Ok(Header::Ok);
            }
            Ok(Header::Kept(Errno::EPROTONOSUPPORT))
        })();
        let off = x.offset();
        (h, off)
    };
    match h {
        Ok(Header::Bad(e)) | Err(e) => Header::Bad(e),
        Ok(h) => {
            reply.set_offset(off);
            h
        }
    }
}

/// `nfs_timer(arg)`: nfs timer routine. Scan the nfsreq list and retransmit any requests that
/// have timed out. `arg` is the mount whose `nm_rtimeout` this is (`nfs_vfsops`'s
/// `timeout_set_proc`, a counted handle). The C's one walk under the net lock, kept: a
/// reader of the kernel lock at a time, the place kept in a counted handle across the work
/// done without it (the module's Redesign).
pub fn nfs_timer<G: ProcGlue>(arg: &hal::mem::pool::AnyArc) {
    let Some(nmp) = arg.downcast_ref::<NfsMount<G>>() else {
        kpanic(format_args!("nfs_timer: not an nfs mount"));
    };

    let nw = NETLOCK.write();
    let Some(net) = NetHeld::of(&nw) else {
        kpanic(format_args!("nfs_timer: the net lock"));
    };
    let mut place = Place::First;
    while let Some((h, work)) = nfs_timer_walk(nmp, &place) {
        place = match work {
            TimerWork::Msg => {
                // Check for server not responding.
                nfs_msg_tid(nmp, h.r_procp, "not responding");
                h.set_flags(R_TPRINTFMSG);
                Place::AtRetry(h)
            }
            TimerWork::Resend(so) => {
                nfs_timer_resend(nmp, &h, &so, &net);
                Place::After(h)
            }
        };
    }
    drop(place);
    drop(nw);
    nmp.nm_rtimeout.as_pin().add(NFS_TICKS.load(Relaxed));
}

/// One reader's part of `nfs_timer`'s walk, from `place`: each request's step until one asks
/// for work, which is answered with the request's handle and the work; `None` at the queue's
/// end. A place
/// whose request left the queue meanwhile (only possible where the kernel lock excludes
/// nothing: the host's double) ends the walk.
fn nfs_timer_walk<G: ProcGlue>(
    nmp: &NfsMount<G>,
    place: &Place<G>,
) -> Option<(PoolArc<NfsReq<G>>, TimerWork<G>)> {
    let r = klock::reader();
    let (mut e, mut at_retry) = match place {
        Place::First => (nmp.nm_reqsq.first(&r), false),
        Place::After(h) => {
            if !h.is_queued() {
                return None;
            }
            (nmp.nm_reqsq.next(&r, h), false)
        }
        Place::AtRetry(h) => {
            if !h.is_queued() {
                return None;
            }
            (Some(&**h), true)
        }
    };
    while let Some(rep) = e {
        if let Some(work) = nfs_timer_step(nmp, rep, &r, at_retry) {
            return Some((nfs_req_handle(rep), work));
        }
        at_retry = false;
        e = nmp.nm_reqsq.next(&r, rep);
    }
    None
}

/// One more handle to a queued request (its owner holds one while it is queued).
fn nfs_req_handle<G: ProcGlue>(rep: &NfsReq<G>) -> PoolArc<NfsReq<G>> {
    match PoolArc::from_ref(rep) {
        Some(h) => h,
        None => kpanic(format_args!("nfs_timer: a request not from nfsreqpl")),
    }
}

/// The C's loop body for one request (`at_retry`: from the retry count's test on, after the
/// message): the work it stops for, `None` for the next request.
fn nfs_timer_step<G: ProcGlue>(
    nmp: &NfsMount<G>,
    rep: &NfsReq<G>,
    r: &klock::KernelReader,
    at_retry: bool,
) -> Option<TimerWork<G>> {
    if !at_retry {
        if rep.has_reply() || rep.flags() & R_SOFTTERM != 0 {
            return None;
        }
        // The requester's thread, found by its id only on an interruptible mount (the C reads
        // r_procp there alone): it sleeps in nfs_reply while its request is queued.
        let p = if nmp.flag() & NFSMNT_INT != 0 {
            rep.r_procp.and_then(|tid| tfind::<G, _>(r, tid))
        } else {
            None
        };
        if nfs_sigintr(nmp, Some(rep), p).is_err() {
            rep.set_flags(R_SOFTTERM);
            return None;
        }
        let rtt = rep.r_rtt.load(Relaxed);
        if rtt >= 0 {
            let rtt = rtt.wrapping_add(1);
            rep.r_rtt.store(rtt, Relaxed);
            let mut timeo = if nmp.flag() & NFSMNT_DUMBTIMR != 0 {
                nmp.nm_timeo.load(Relaxed)
            } else {
                nfs_estimate_rto(nmp, rep.r_procnum as usize)
            };
            let timeouts = nmp.nm_timeouts.load(Relaxed);
            if timeouts > 0 {
                timeo = timeo.wrapping_mul(NFS_BACKOFF[(timeouts - 1) as usize]);
            }
            if rtt <= timeo {
                return None;
            }
            if (timeouts as usize) < NFS_BACKOFF.len() {
                nmp.nm_timeouts.store(timeouts + 1, Relaxed);
            }
        }

        // Check for server not responding.
        if rep.flags() & R_TPRINTFMSG == 0 && rep.r_rexmit.load(Relaxed) > 4 {
            return Some(TimerWork::Msg);
        }
    }
    if rep.r_rexmit.load(Relaxed) >= nmp.nm_retry.load(Relaxed) {
        // too many
        NFSSTATS.rpctimeouts.fetch_add(1, Relaxed);
        rep.set_flags(R_SOFTTERM);
        return None;
    }
    if nmp.nm_sotype.load(Relaxed) != SOCK_DGRAM {
        let x = rep.r_rexmit.load(Relaxed).wrapping_add(1);
        rep.r_rexmit.store(x.min(NFS_MAXREXMIT), Relaxed);
        return None;
    }

    let so = nmp.nm_so.get(r).clone()?;

    // If there is enough space and the window allows.. Resend it. Set r_rtt to -1 in case we
    // fail to send it now.
    rep.r_rtt.store(-1, Relaxed);
    if so.so_snd.sbspace() >= i64::from(rep.r_len)
        && (nmp.flag() & NFSMNT_DUMBTIMR != 0
            || rep.flags() & R_SENT != 0
            || nmp.nm_sent.load(Relaxed) < nmp.nm_cwnd.load(Relaxed))
    {
        return Some(TimerWork::Resend(so));
    }
    None
}

/// The timer's retransmission of `rep` on `so` (the C's `m_copym(.., M_DONTWAIT)` and
/// `pru_send` under the walk's net lock): the copy under a witness section, the send under
/// the socket's lock with the net lock's proof, the window and the counts after.
fn nfs_timer_resend<G: ProcGlue>(
    nmp: &NfsMount<G>,
    rep: &NfsReq<G>,
    so: &Socket<G>,
    net: &NetHeld<'_>,
) {
    let Some(m) = nfs_copy_mreq(rep, M_DONTWAIT) else {
        return;
    };
    let send = |addr: Option<&Mbuf>| {
        let mut w = solock_net(so, net);
        so.usrreqs().send(so, &mut w, Some(m), addr, None)
    };
    let error = if nmp.flag() & NFSMNT_NOCONN == 0 {
        send(None)
    } else {
        // The mount's address, lent (a copy made without waiting while another holds it; none
        // to be had: as no copy of the request).
        match nmp.with_nam(M_DONTWAIT, |a| a.map(|a| send(Some(a)))) {
            Some(error) => error,
            None => return,
        }
    };
    if let Err(error) = error {
        if nfsignore_soerror(nmp.nm_soflags.load(Relaxed), error) {
            so.set_error(None);
        }
    } else {
        // Iff first send, start timing else turn timing off, backoff timer and divide
        // congestion window by 2.
        if rep.flags() & R_SENT != 0 {
            rep.clear_flags(R_TIMING);
            let x = rep.r_rexmit.load(Relaxed).wrapping_add(1);
            rep.r_rexmit.store(x.min(NFS_MAXREXMIT), Relaxed);
            let cwnd = nmp.nm_cwnd.load(Relaxed) >> 1;
            nmp.nm_cwnd.store(cwnd.max(NFS_CWNDSCALE), Relaxed);
            NFSSTATS.rpcretries.fetch_add(1, Relaxed);
        } else {
            rep.set_flags(R_SENT);
            nmp.nm_sent.fetch_add(NFS_CWNDSCALE, Relaxed);
        }
        rep.r_rtt.store(0, Relaxed);
    }
}

/// `nfs_sigintr(nmp, rep, p)`: test for a termination condition pending on the process. This
/// is used for `NFSMNT_INT` mounts. `Err(EINTR)` when the RPC must stop.
pub fn nfs_sigintr<G: ProcGlue>(
    nmp: &NfsMount<G>,
    rep: Option<&NfsReq<G>>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    if rep.is_some_and(|r| r.flags() & R_SOFTTERM != 0) {
        return Err(Errno::EINTR);
    }
    if nmp.flag() & NFSMNT_INT == 0 {
        return Ok(());
    }
    // The ignored mask is read only when a signal of the set is pending: the C reads it every
    // time, unlocked; here it is under the process's ps_mtx, a lock pair per queued request
    // per tick otherwise. The answer is the same.
    if let Some(p) = p {
        let pending = sigpending(p) & NFSINT_SIGMASK;
        if pending != 0 && pending & !sigignore(p.process()) != 0 {
            return Err(Errno::EINTR);
        }
    }
    Ok(())
}

/// `nfs_sndlock(flagp, rep)`: lock a socket against others. Necessary for STREAM sockets to
/// ensure you get an entire rpc request/reply and also to avoid race conditions between the
/// processes with nfs requests in progress when a reconnect is necessary. `flagp` is the
/// client's `&nmp->nm_flag` or the server's `&slp->ns_solock` (with no request); its address
/// is the wait channel.
pub fn nfs_sndlock<G: ProcGlue>(flagp: &AtomicI32, rep: Option<&Rep<'_, G>>) -> Result<(), Errno> {
    let mut slptimeo = INFSLP;
    let mut slpflag = 0;

    if let Some(r) = rep
        && r.nmp.flag() & NFSMNT_INT != 0
    {
        slpflag = PCATCH;
    }
    while flagp.load(Relaxed) & NFSMNT_SNDLOCK != 0 {
        if let Some(r) = rep
            && nfs_sigintr(r.nmp, Some(r.rep), r.procp).is_err()
        {
            return Err(Errno::EINTR);
        }
        flagp.fetch_or(NFSMNT_WANTSND, Relaxed);
        let _ = tsleep_nsec(
            ptr::from_ref(flagp),
            slpflag | (PZERO - 1),
            "nfsndlck",
            slptimeo,
        );
        if slpflag == PCATCH {
            slpflag = 0;
            slptimeo = sec_to_nsec(2);
        }
    }
    flagp.fetch_or(NFSMNT_SNDLOCK, Relaxed);
    Ok(())
}

/// `nfs_sndunlock(flagp)`: unlock the stream socket for others.
pub fn nfs_sndunlock(flagp: &AtomicI32) {
    if flagp.load(Relaxed) & NFSMNT_SNDLOCK == 0 {
        kpanic(format_args!("nfs sndunlock"));
    }
    flagp.fetch_and(!NFSMNT_SNDLOCK, Relaxed);
    if flagp.load(Relaxed) & NFSMNT_WANTSND != 0 {
        flagp.fetch_and(!NFSMNT_WANTSND, Relaxed);
        wakeup(ptr::from_ref(flagp));
    }
}

/// `nfs_rcvlock(rep)`: lock the mount's socket against other receivers. `Err(EALREADY)` when
/// our reply has been received while we were sleeping.
pub fn nfs_rcvlock<G: ProcGlue>(rep: &Rep<'_, G>) -> Result<(), Errno> {
    let mut slptimeo = INFSLP;
    let flagp = &rep.nmp.nm_flag;
    let mut slpflag = if flagp.load(Relaxed) & NFSMNT_INT != 0 {
        PCATCH
    } else {
        0
    };

    while flagp.load(Relaxed) & NFSMNT_RCVLOCK != 0 {
        if nfs_sigintr(rep.nmp, Some(rep.rep), rep.procp).is_err() {
            return Err(Errno::EINTR);
        }
        flagp.fetch_or(NFSMNT_WANTRCV, Relaxed);
        let _ = tsleep_nsec(
            ptr::from_ref(flagp),
            slpflag | (PZERO - 1),
            "nfsrcvlk",
            slptimeo,
        );
        if rep.rep.has_reply() {
            // Don't take the lock if our reply has been received while we where sleeping.
            return Err(Errno::EALREADY);
        }
        if slpflag == PCATCH {
            slpflag = 0;
            slptimeo = sec_to_nsec(2);
        }
    }
    flagp.fetch_or(NFSMNT_RCVLOCK, Relaxed);
    Ok(())
}

/// `nfs_rcvunlock(flagp)`: unlock the stream socket for others.
pub fn nfs_rcvunlock(flagp: &AtomicI32) {
    if flagp.load(Relaxed) & NFSMNT_RCVLOCK == 0 {
        kpanic(format_args!("nfs rcvunlock"));
    }
    flagp.fetch_and(!NFSMNT_RCVLOCK, Relaxed);
    if flagp.load(Relaxed) & NFSMNT_WANTRCV != 0 {
        flagp.fetch_and(!NFSMNT_WANTRCV, Relaxed);
        wakeup(ptr::from_ref(flagp));
    }
}

/// `nfs_msg(rep, msg)`: "nfs server <mntfromname>: <msg>" to the requester's terminal (when
/// it has one) and to the log, by `tprintf`.
pub fn nfs_msg<G: ProcGlue>(nmp: &NfsMount<G>, p: Option<&Thread<G>>, msg: &str) {
    let tpr = p.and_then(tprintf_open);
    tprintf(
        &tpr,
        format_args!("nfs server {}: {}\n", Str(&nmp.mntfromname()), msg),
    );
    tprintf_close(tpr);
}

/// [`nfs_msg`] for a requester known by its thread id (the timer's): the thread found under
/// a reader, its session held, the message printed without the reader.
fn nfs_msg_tid<G: ProcGlue>(nmp: &NfsMount<G>, tid: Option<Pid>, msg: &str) {
    let tpr = {
        let r = klock::reader();
        tid.and_then(|t| tfind::<G, _>(&r, t))
            .and_then(tprintf_open)
    };
    tprintf(
        &tpr,
        format_args!("nfs server {}: {}\n", Str(&nmp.mntfromname()), msg),
    );
    tprintf_close(tpr);
}

const _: () = {
    assert!(NFS_MAX_TIMER == 4);
    assert!(NFS_PTIMERS.len() == NFS_NPROCS);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the engine's parts that run without a peer: the RTT estimator, the
    // timer's walk over queued requests (backoff, the message, the soft termination, the
    // stream's count), the flag locks, nfs_sigintr, the RPC header (AUTH_UNIX and its group
    // clamp, the version 2 procedure map), the xids, the TRYLATER backoff. The scripted
    // peer's tests (an RPC over the kernel's UDP and TCP on lo0) are `testnfs`'s.

    use core::ffi::c_void;
    use std::sync::MutexGuard;

    use hal::mem::malloc::{M_WAITOK, MArc, MBox};

    use super::*;
    use crate::fs::nfs::nfs_subs::{NFSV2_PROCID, nfs_get_xid, nfs_init};
    use crate::fs::nfs::nfsmount::{M_NFSMNT, NfsMountRef, NfsNam};
    use crate::fs::nfs::nfsproto::{NFSPROC_GETATTR, NFSPROC_LOOKUP, NFSPROC_NULL, NFSPROC_READ};
    use crate::fs::testfs::TestGlue as G;
    use crate::net::mbuf::{MT_DATA, MbufBox, m_gethdr};
    use crate::sys::signal::{SIGINT, SIGUSR1, sigmask};
    use crate::time::timeout::Timeout;

    fn nop(_: *mut c_void) {}

    /// The UNIX socket tests' fixture (memory, mbufs, a root thread on the test file system,
    /// the kernel lock), "nfsreqpl", and a mount's structure on the test root, of socket type
    /// `sotype`, flags `flag`, its timeout set to a function that does nothing.
    fn mount(
        sotype: i32,
        flag: i32,
    ) -> (MutexGuard<'static, ()>, &'static Thread<G>, NfsMountRef<G>) {
        let (g, p) = crate::net::unix::tests::setup();
        // nfs_init makes the server's tables again: an earlier test's are forgotten first.
        #[cfg(feature = "nfsserver")]
        crate::fs::nfs::nfssvc::nfsrv_test_reset::<G>();
        nfs_init::<G>();
        let mp = crate::fs::init::rootvnode::<G>()
            .and_then(|v| v.mount())
            .expect("the test root");
        let sin = [16u8, 2, 0x08, 0x01, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        let to = MBox::new(M_NFSMNT, M_WAITOK, Timeout::new(nop, core::ptr::null_mut()))
            .expect("a timeout");
        let nmp = MArc::new(
            M_NFSMNT,
            M_WAITOK,
            NfsMount::new(mp, NfsNam::new(&sin).expect("an address"), to),
        )
        .expect("a mount");
        nmp.nm_sotype.store(sotype, Relaxed);
        nmp.nm_flag.store(flag, Relaxed);
        nmp.nm_retry.store(10, Relaxed);
        nmp.nm_timeo.store(1, Relaxed);
        nmp.nm_numgrps.store(16, Relaxed);
        nfs_init_rtt(&nmp);
        nmp.nm_cwnd.store(NFS_MAXCWND / 2, Relaxed);
        (g, p, nmp)
    }

    /// A request of `procnum` queued on `nmp` as `nfs_request` queues it (its chain a 32-byte
    /// packet).
    fn queue(nmp: &NfsMount<G>, procnum: usize, tid: Option<Pid>) -> PoolArc<NfsReq<G>> {
        queue_xid(nmp, procnum, tid, 7)
    }

    /// [`queue`] with the transaction id `xid`.
    fn queue_xid(
        nmp: &NfsMount<G>,
        procnum: usize,
        tid: Option<Pid>,
        xid: u32,
    ) -> PoolArc<NfsReq<G>> {
        let m = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(32);
        m.m_pkthdr().len.set(32);
        let rep = crate::fs::subr::vfs::<G>()
            .nfs
            .nfsreqpl
            .get_homed(PR_WAITOK, || NfsReq::new(procnum as u32, xid, m, tid, 0))
            .expect("a request");
        let mut w = klock::witness();
        nmp.nm_reqsq.insert_tail(&mut w, rep.as_pin());
        rep
    }

    /// Unlinks `rep` and stops the mount's timeout (`nfs_request`'s end).
    fn unqueue(nmp: &NfsMount<G>, rep: &NfsReq<G>) {
        let mut w = klock::witness();
        nmp.nm_reqsq.remove(&mut w, rep);
        drop(w);
        nmp.nm_rtimeout.del();
    }

    /// One `nfs_timer` call, as the softclock thread makes it.
    fn tick(nmp: &NfsMountRef<G>) {
        nfs_timer::<G>(&MArc::erase(nmp.clone()));
    }

    #[test]
    fn the_rtt_estimator_follows_the_c_s_gains_and_clamps() {
        let (_g, _p, nmp) = mount(SOCK_DGRAM, 0);
        let hz = crate::fs::nfs::nfs::nfs_hz();
        // NFS_INITRTT is NFS_HZ << 3 on every timer.
        assert_eq!(nmp.nm_srtt[0].load(Relaxed), hz << 3);
        // getattr: A + 2D, clamped to [NFS_MINRTO, NFS_MAXRTO].
        let srtt = hz << 3;
        let want = (srtt + 3) >> 2; // + ((sdrtt + 1) >> 1), sdrtt 0
        assert_eq!(
            nfs_estimate_rto(&nmp, NFSPROC_GETATTR),
            want.clamp(nfs_minrto(), nfs_maxrto())
        );
        // Others: nm_timeo, unclamped.
        assert_eq!(nfs_estimate_rto(&nmp, NFSPROC_NULL), 1);
        // A read of RTT 4: t1 = 5 - (srtt >> 3), srtt += t1; |t1| - (sdrtt >> 2) to sdrtt.
        let rep = queue(&nmp, NFSPROC_READ, None);
        rep.r_rtt.store(4, Relaxed);
        nfs_update_rtt(&nmp, &rep);
        let t1 = 5 - (srtt >> 3);
        let i = NFS_READ_TIMER - 1;
        assert_eq!(nmp.nm_srtt[i].load(Relaxed), srtt + t1);
        assert_eq!(nmp.nm_sdrtt[i].load(Relaxed), t1.abs());
        unqueue(&nmp, &rep);
    }

    /// A stream mount's timer: each due tick backs off (`nm_timeouts`, at most 8), counts a
    /// retransmission (the reconnecting receive resends), prints "not responding" once past
    /// four and ends the request softly at `nm_retry` (`rpctimeouts`).
    #[test]
    fn the_timer_backs_off_warns_and_terminates_at_the_retry_count() {
        let (_g, _p, nmp) = mount(SOCK_STREAM, NFSMNT_DUMBTIMR);
        nmp.nm_timeo.store(0, Relaxed);
        nmp.nm_retry.store(7, Relaxed);
        let rep = queue(&nmp, NFSPROC_GETATTR, None);
        let timeouts0 = NFSSTATS.rpctimeouts.load(Relaxed);
        for n in 1..=5 {
            tick(&nmp);
            assert_eq!(rep.r_rexmit.load(Relaxed), n);
            assert_eq!(rep.flags() & R_TPRINTFMSG, 0);
        }
        tick(&nmp);
        assert_ne!(rep.flags() & R_TPRINTFMSG, 0, "not responding past four");
        assert_eq!(
            rep.r_rexmit.load(Relaxed),
            6,
            "the step goes on after the message"
        );
        tick(&nmp);
        assert_eq!(rep.r_rexmit.load(Relaxed), 7);
        assert_eq!(rep.flags() & R_SOFTTERM, 0);
        tick(&nmp);
        assert_ne!(rep.flags() & R_SOFTTERM, 0, "rexmit >= nm_retry");
        assert_eq!(NFSSTATS.rpctimeouts.load(Relaxed), timeouts0 + 1);
        assert_eq!(nmp.nm_timeouts.load(Relaxed), 8, "the backoff's cap");
        tick(&nmp);
        assert_eq!(
            rep.r_rexmit.load(Relaxed),
            7,
            "a terminated request is skipped"
        );
        unqueue(&nmp, &rep);
    }

    /// A request not yet due waits for its timeout; one with a reply, and one sent on no
    /// socket, are left as they are.
    #[test]
    fn the_timer_waits_for_the_timeout_and_skips_replied_requests() {
        let (_g, _p, nmp) = mount(SOCK_DGRAM, NFSMNT_DUMBTIMR);
        nmp.nm_timeo.store(2, Relaxed);
        let a = queue(&nmp, NFSPROC_NULL, None);
        let b = queue(&nmp, NFSPROC_NULL, None);
        {
            let mut w = klock::witness();
            let m = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
            m.m_len().set(0);
            drop(b.set_reply(&mut w, Some(Reply::new(m, 0))));
        }
        tick(&nmp);
        tick(&nmp);
        assert_eq!(a.r_rtt.load(Relaxed), 2, "rtt <= timeo: not due");
        assert_eq!(nmp.nm_timeouts.load(Relaxed), 0);
        tick(&nmp);
        assert_eq!(a.r_rtt.load(Relaxed), 3, "due, but no socket: no resend");
        assert_eq!(nmp.nm_timeouts.load(Relaxed), 1);
        assert_eq!(b.r_rtt.load(Relaxed), 0, "replied: skipped");
        {
            let mut w = klock::witness();
            drop(b.set_reply(&mut w, None));
        }
        unqueue(&nmp, &a);
        unqueue(&nmp, &b);
    }

    /// `NFSMNT_INT`: a pending signal of the mask interrupts (`EINTR`), another does not, and
    /// without the flag none does; a soft-terminated request always does.
    #[test]
    fn nfs_sigintr_answers_eintr_on_interruptible_mounts_only() {
        let (_g, p, nmp) = mount(SOCK_DGRAM, 0);
        crate::sig::test_sigacts(p.process());
        let rep = queue(&nmp, NFSPROC_NULL, None);
        p.p_siglist.store(sigmask(SIGINT), Relaxed);
        assert_eq!(
            nfs_sigintr(&nmp, Some(&rep), Some(p)),
            Ok(()),
            "no NFSMNT_INT"
        );
        nmp.nm_flag.store(NFSMNT_INT, Relaxed);
        assert_eq!(nfs_sigintr(&nmp, Some(&rep), Some(p)), Err(Errno::EINTR));
        assert_eq!(nfs_sigintr(&nmp, Some(&rep), None), Ok(()), "no thread");
        p.p_siglist.store(sigmask(SIGUSR1), Relaxed);
        assert_eq!(
            nfs_sigintr(&nmp, Some(&rep), Some(p)),
            Ok(()),
            "not in the mask"
        );
        p.p_siglist.store(0, Relaxed);
        rep.set_flags(R_SOFTTERM);
        nmp.nm_flag.store(0, Relaxed);
        assert_eq!(nfs_sigintr(&nmp, Some(&rep), Some(p)), Err(Errno::EINTR));
        unqueue(&nmp, &rep);
    }

    /// An empty reply chain for the match.
    fn chain() -> MbufBox {
        let m = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(0);
        m
    }

    /// Takes a request's reply back out (the requester's unlink).
    fn take_reply(rep: &NfsReq<G>) -> Option<Reply> {
        let mut w = klock::witness();
        rep.set_reply(&mut w, None)
    }

    /// Two outstanding requests: each reply goes to the request of its xid whatever the order,
    /// the sent count falls by one window unit for each; a second reply to a request that has
    /// one matches nothing (`rpcunexpected`).
    #[test]
    fn the_match_gives_each_reply_to_its_own_request() {
        let (_g, _p, nmp) = mount(SOCK_DGRAM, 0);
        let a = queue_xid(&nmp, NFSPROC_NULL, None, 0x0a);
        let b = queue_xid(&nmp, NFSPROC_NULL, None, 0x0b);
        for r in [&a, &b] {
            r.set_flags(R_SENT);
        }
        nmp.nm_sent.store(2 * NFS_CWNDSCALE, Relaxed);
        let unexpected = NFSSTATS.rpcunexpected.load(Relaxed);
        assert!(
            !nfs_reply_match(&nmp, &a, 0x0b, chain(), 8),
            "B's reply is not A's"
        );
        assert!(b.has_reply() && !a.has_reply());
        assert_eq!(b.flags() & R_SENT, 0);
        assert_eq!(nmp.nm_sent.load(Relaxed), NFS_CWNDSCALE);
        assert!(nfs_reply_match(&nmp, &a, 0x0a, chain(), 8), "A's own");
        assert!(a.has_reply());
        assert_eq!(nmp.nm_sent.load(Relaxed), 0);
        assert_eq!(take_reply(&a).map(|r| r.offset()), Some(8));
        // A has its reply cell empty again but is still the one with that xid: a duplicate
        // while A holds a reply is dropped.
        {
            let mut w = klock::witness();
            drop(a.set_reply(&mut w, Some(Reply::new(chain(), 8))));
        }
        assert!(!nfs_reply_match(&nmp, &a, 0x0a, chain(), 8));
        assert_eq!(NFSSTATS.rpcunexpected.load(Relaxed), unexpected + 1);
        assert_eq!(
            nmp.nm_sent.load(Relaxed),
            0,
            "no window unit for a dropped reply"
        );
        drop(take_reply(&a));
        drop(take_reply(&b));
        unqueue(&nmp, &a);
        unqueue(&nmp, &b);
    }

    /// `nfs_rcvlock` waiting for another receiver: a reply that arrives for the request
    /// meanwhile ends the wait with `EALREADY`, the lock not taken.
    #[test]
    fn the_receive_lock_answers_ealready_when_the_reply_came_meanwhile() {
        let (_g, p, nmp) = mount(SOCK_DGRAM, 0);
        let rep = queue(&nmp, NFSPROC_NULL, None);
        nmp.nm_flag.fetch_or(NFSMNT_RCVLOCK, Relaxed); // another receiver holds it
        let done = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
        let (d, h, m) = (done.clone(), rep.clone(), nmp.clone());
        let waker = std::thread::spawn(move || {
            hal::sync::klock::kernel_lock();
            {
                let mut w = klock::witness();
                drop(h.set_reply(&mut w, Some(Reply::new(chain(), 0))));
            }
            while !d.load(Relaxed) {
                wakeup(ptr::from_ref(&m.nm_flag));
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        let r = Rep {
            nmp: &nmp,
            rep: &rep,
            procp: Some(p),
        };
        let got = nfs_rcvlock(&r);
        done.store(true, Relaxed);
        waker.join().expect("the waker");
        assert_eq!(got, Err(Errno::EALREADY));
        assert_ne!(nmp.flag() & NFSMNT_WANTRCV, 0, "it waited");
        nmp.nm_flag
            .fetch_and(!(NFSMNT_RCVLOCK | NFSMNT_WANTRCV), Relaxed);
        drop(take_reply(&rep));
        unqueue(&nmp, &rep);
    }

    /// The timer on an `NFSMNT_INT` mount finds the requester by its thread id: a signal of
    /// the mask pending ends the request (`R_SOFTTERM`); none pending leaves it.
    #[test]
    fn the_timer_ends_an_interruptible_request_whose_thread_has_a_signal() {
        let (_g, p, nmp) = mount(SOCK_DGRAM, NFSMNT_INT | NFSMNT_DUMBTIMR);
        crate::sig::test_sigacts(p.process());
        nmp.nm_timeo.store(100, Relaxed);
        const TID: Pid = 6100;
        p.p_tid.set(TID);
        {
            let mut w = klock::witness();
            G::tables()
                .tidhash(TID)
                .insert_head(&mut w, core::pin::Pin::static_ref(p));
        }
        let rep = queue(&nmp, NFSPROC_NULL, Some(TID));
        tick(&nmp);
        assert_eq!(rep.flags() & R_SOFTTERM, 0, "no signal");
        p.p_siglist.store(sigmask(SIGINT), Relaxed);
        tick(&nmp);
        assert_ne!(rep.flags() & R_SOFTTERM, 0, "SIGINT on the requester");
        p.p_siglist.store(0, Relaxed);
        {
            let mut w = klock::witness();
            G::tables().tidhash(TID).remove(&mut w, p);
        }
        unqueue(&nmp, &rep);
    }

    /// The flag locks of `nm_flag`: taken, given back, the waiter's bit cleared.
    #[test]
    fn the_send_and_receive_locks_are_bits_of_nm_flag() {
        let (_g, p, nmp) = mount(SOCK_STREAM, 0);
        let rep = queue(&nmp, NFSPROC_NULL, None);
        let r = Rep {
            nmp: &nmp,
            rep: &rep,
            procp: Some(p),
        };
        assert_eq!(nfs_sndlock(&nmp.nm_flag, Some(&r)), Ok(()));
        assert_ne!(nmp.flag() & NFSMNT_SNDLOCK, 0);
        nmp.nm_flag.fetch_or(NFSMNT_WANTSND, Relaxed);
        nfs_sndunlock(&nmp.nm_flag);
        assert_eq!(nmp.flag() & (NFSMNT_SNDLOCK | NFSMNT_WANTSND), 0);
        assert_eq!(nfs_rcvlock(&r), Ok(()));
        assert_ne!(nmp.flag() & NFSMNT_RCVLOCK, 0);
        nfs_rcvunlock(&nmp.nm_flag);
        assert_eq!(nmp.flag() & NFSMNT_RCVLOCK, 0);
        // The server's form: a flag word of its own, no request.
        let solock = AtomicI32::new(0);
        assert_eq!(nfs_sndlock::<G>(&solock, None), Ok(()));
        nfs_sndunlock(&solock);
        assert_eq!(solock.load(Relaxed), 0);
        unqueue(&nmp, &rep);
    }

    /// The AUTH_UNIX header: the static part, the credential's ids, its groups clamped to the
    /// mount's `nm_numgrps`, the null verifier; the arguments linked behind; version 2's
    /// procedure number from `nfsv2_procid`.
    #[test]
    fn the_rpc_header_carries_the_credential_with_the_group_clamp() {
        let (_g, _p, nmp) = mount(SOCK_DGRAM, NFSMNT_NFSV3);
        let mut cr = Ucred::new();
        cr.cr_uid.set(1001);
        cr.cr_gid.set(20);
        cr.cr_ngroups.set(5);
        for (i, g) in cr.cr_groups.iter_mut().enumerate().take(5) {
            g.set(100 + i as u32);
        }
        nmp.nm_numgrps.store(3, Relaxed);
        let body = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
        body.m_len().set(8);
        let (m, xid) =
            nfsm_rpchead(&nmp, NFSPROC_LOOKUP, &cr, RPCAUTH_UNIX, body).expect("a header");
        let bytes = crate::fs::nfs::nfsm_subs::test_bytes(&m);
        let w = |i: usize| u32::from_be_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(
            m.m_pkthdr().len.get(),
            40 + 32 + 8,
            "header, AUTH_UNIX of 3 groups, body"
        );
        assert_eq!(bytes[0..4], xid.to_ne_bytes());
        assert_eq!([w(1), w(2), w(3), w(4), w(5)], [0, 2, 100_003, 3, 3]);
        assert_eq!(
            [w(6), w(7), w(8), w(9)],
            [1, 32, 0, 0],
            "AUTH_UNIX, authsiz, stamp, host"
        );
        assert_eq!(
            [w(10), w(11), w(12)],
            [1001, 20, 3],
            "uid, gid, the clamped count"
        );
        assert_eq!([w(13), w(14), w(15)], [100, 101, 102]);
        assert_eq!([w(16), w(17)], [0, 0], "AUTH_NULL verifier");
        // Fewer groups than nm_numgrps: the credential's count.
        nmp.nm_numgrps.store(16, Relaxed);
        nmp.nm_flag.store(0, Relaxed);
        cr.cr_ngroups.set(2);
        let body = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
        body.m_len().set(0);
        let (m, _) = nfsm_rpchead(&nmp, NFSPROC_LOOKUP, &cr, RPCAUTH_UNIX, body).expect("a header");
        let bytes = crate::fs::nfs::nfsm_subs::test_bytes(&m);
        let w = |i: usize| u32::from_be_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(bytes.len(), 40 + 28);
        assert_eq!(
            [w(4), w(5)],
            [2, NFSV2_PROCID[NFSPROC_LOOKUP] as u32],
            "version 2"
        );
        assert_eq!(w(12), 2);
    }

    #[test]
    fn xids_are_unpredictable_words_that_do_not_repeat() {
        crate::testutil::install();
        let a = nfs_get_xid();
        let b = nfs_get_xid();
        assert_ne!(a, b);
        assert!(a != 0 && b != 0);
    }

    /// `NFSERR_TRYLATER`'s wait doubles up to `NFS_MAXTIMEO`.
    #[test]
    fn the_trylater_backoff_doubles_to_its_cap() {
        let max = nfs_maxtimeo();
        let mut d = nfs_mintimeo().max(1);
        let mut seen = std::vec::Vec::new();
        while d < max {
            seen.push(d);
            d = nfs_trylater_next(d);
        }
        assert_eq!(d, max);
        assert_eq!(nfs_trylater_next(max), max);
        assert!(seen.windows(2).all(|p| p[1] == p[0] * NFS_TIMEOUTMUL));
    }

    // The scripted peer's tests: an RPC over the kernel's UDP and TCP on lo0, against a
    // kernel socket on 127.0.0.1:2049 that answers as each test's script says.

    use std::boxed::Box;

    use crate::fs::nfs::testnfs::{self as tn, Out};
    use crate::sys::netinet_in::IPPORT_RESERVED;

    /// An RPC of procedure `procnum` with no arguments, made by the fixture's requester with
    /// its credential.
    fn rpc(
        fx: &tn::Fixture,
        nmp: &NfsMount<tn::G>,
        procnum: usize,
    ) -> (Option<Reply>, Result<(), NfsError>) {
        nfs_request(
            nmp,
            fx.vp,
            procnum,
            Request {
                body: XdrWriter::reqhead(0),
                procp: Some(fx.user),
                cred: fx.user.ucred(),
            },
        )
    }

    /// The XDR words of a call.
    fn words(c: &[u8]) -> std::vec::Vec<u32> {
        c.chunks_exact(4)
            .map(|w| u32::from_be_bytes(w.try_into().unwrap()))
            .collect()
    }

    /// A NULL call over UDP: the header (RPC version 2, the program, version 3, the
    /// procedure, AUTH_UNIX with the requester's uid), the reply's header consumed, and the
    /// client's port reserved though the requester is no superuser (`proc0`'s bind).
    #[test]
    fn a_call_over_udp_is_answered_from_a_reserved_port() {
        let fx = tn::setup(1000);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3);
        assert_eq!(tn::connect(&nmp), Ok(()));
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &nmp,
            Box::new(|_, c| std::vec![Out::Data(tn::reply(c, 0, 0, 0, &[]))]),
        );
        let (reply, r) = rpc(&fx, &nmp, NFSPROC_NULL);
        let calls = peer.calls();
        peer.stop();
        assert_eq!(r, Ok(()));
        assert_eq!(
            reply.expect("the reply").offset(),
            28,
            "the RPC header's 7 words"
        );
        assert_eq!(calls.len(), 1);
        let (port, c) = &calls[0];
        assert!(*port < IPPORT_RESERVED as u16 && *port > 0, "port {port}");
        let w = words(c);
        assert_eq!(&w[1..6], &[0, 2, 100_003, 3, NFSPROC_NULL as u32]);
        assert_eq!([w[6], w[10]], [1, 1000], "AUTH_UNIX, the requester's uid");
        tn::unmount(&fx, nmp);
    }

    /// A datagram the server never answers is sent again by the timer (`R_SENT`: a
    /// retransmission, the window halved, `rpcretries`), and the reply to the copy ends the
    /// RPC; the timeouts' count is cleared by the reply.
    #[test]
    fn a_lost_datagram_is_resent_by_the_timer() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3 | NFSMNT_DUMBTIMR);
        nmp.nm_timeo.store(1, Relaxed);
        assert_eq!(tn::connect(&nmp), Ok(()));
        let retries = NFSSTATS.rpcretries.load(Relaxed);
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &nmp,
            Box::new(|n, c| {
                if n == 1 {
                    std::vec![Out::Tick(2)]
                } else {
                    std::vec![Out::Data(tn::reply(c, 0, 0, 0, &[]))]
                }
            }),
        );
        let (_, r) = rpc(&fx, &nmp, NFSPROC_NULL);
        let calls = peer.calls();
        peer.stop();
        assert_eq!(r, Ok(()));
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1, calls[1].1, "the same request, its xid kept");
        assert_eq!(NFSSTATS.rpcretries.load(Relaxed), retries + 1);
        assert_eq!(nmp.nm_cwnd.load(Relaxed), NFS_MAXCWND / 4, "halved once");
        assert_eq!(nmp.nm_sent.load(Relaxed), 0);
        assert_eq!(nmp.nm_timeouts.load(Relaxed), 0);
        tn::unmount(&fx, nmp);
    }

    /// `MSG_DENIED`: `RPC_MISMATCH` is `EOPNOTSUPP`, an authentication error `EACCES`; no
    /// reply is handed up.
    #[test]
    fn a_denied_call_answers_eopnotsupp_or_eacces() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3);
        assert_eq!(tn::connect(&nmp), Ok(()));
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &nmp,
            Box::new(|n, c| std::vec![Out::Data(tn::denied(c, n == 1))]),
        );
        let (a, ra) = rpc(&fx, &nmp, NFSPROC_NULL);
        let (b, rb) = rpc(&fx, &nmp, NFSPROC_NULL);
        peer.stop();
        assert!(a.is_none() && b.is_none());
        assert_eq!(ra, Err(NfsError::Errno(Errno::EOPNOTSUPP)));
        assert_eq!(rb, Err(NfsError::Errno(Errno::EACCES)));
        tn::unmount(&fx, nmp);
    }

    /// A reply to no request (`rpcunexpected`) and a message that is no reply
    /// (`rpcinvalid`) are dropped; the reply that matches ends the RPC.
    #[test]
    fn a_stray_xid_and_a_non_reply_are_counted_and_dropped() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3);
        assert_eq!(tn::connect(&nmp), Ok(()));
        let unexpected = NFSSTATS.rpcunexpected.load(Relaxed);
        let invalid = NFSSTATS.rpcinvalid.load(Relaxed);
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &nmp,
            Box::new(|_, c| {
                let mut stray = tn::reply(c, 0, 0, 0, &[]);
                stray[0] ^= 0x55;
                let mut call = tn::call_xid(c).to_vec();
                call.extend_from_slice(&0u32.to_be_bytes()); // RPC_CALL
                std::vec![
                    Out::Data(stray),
                    Out::Data(call),
                    Out::Data(tn::reply(c, 0, 0, 0, &[])),
                ]
            }),
        );
        let (_, r) = rpc(&fx, &nmp, NFSPROC_NULL);
        peer.stop();
        assert_eq!(r, Ok(()));
        assert_eq!(NFSSTATS.rpcunexpected.load(Relaxed), unexpected + 1);
        assert_eq!(NFSSTATS.rpcinvalid.load(Relaxed), invalid + 1);
        tn::unmount(&fx, nmp);
    }

    /// The C's two outputs: a non-zero NFS status keeps the reply (D-3: one that names no
    /// errno is a `Status`), a non-zero `accept_stat` keeps it with `EPROTONOSUPPORT`, a
    /// verifier with data is skipped by its rounded length.
    #[test]
    fn a_status_error_keeps_the_reply() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3);
        assert_eq!(tn::connect(&nmp), Ok(()));
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &nmp,
            Box::new(|n, c| {
                std::vec![Out::Data(match n {
                    1 => tn::reply(c, 0, 0, 10004, &[]),
                    2 => tn::reply(c, 0, 0, 70, &[]),
                    3 => tn::reply(c, 0, 1, 0, &[]),
                    _ => tn::reply(c, 5, 0, 0, &[]),
                })]
            }),
        );
        let (a, ra) = rpc(&fx, &nmp, NFSPROC_GETATTR);
        let (b, rb) = rpc(&fx, &nmp, NFSPROC_GETATTR);
        let (c, rc) = rpc(&fx, &nmp, NFSPROC_GETATTR);
        let (d, rd) = rpc(&fx, &nmp, NFSPROC_GETATTR);
        peer.stop();
        assert_eq!(ra, Err(NfsError::Status(10004)));
        assert_eq!(a.expect("kept").offset(), 28);
        assert_eq!(rb, Err(NfsError::Errno(Errno::ESTALE)));
        assert!(b.is_some());
        assert_eq!(rc, Err(NfsError::Errno(Errno::EPROTONOSUPPORT)));
        assert_eq!(
            c.expect("kept").offset(),
            24,
            "no status word after accept_stat"
        );
        assert_eq!(rd, Ok(()));
        assert_eq!(
            d.expect("the reply").offset(),
            28 + 8,
            "five verifier bytes, rounded"
        );
        tn::unmount(&fx, nmp);
    }

    /// Version 3's `NFSERR_TRYLATER`: the request is sent again after `NFS_MINTIMEO`
    /// seconds (one here: `nfs_ticks` made `hz`), with its xid.
    #[test]
    fn trylater_sends_the_request_again() {
        let fx = tn::setup(0);
        NFS_TICKS.store(crate::time::timeout::hz(), Relaxed);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3);
        assert_eq!(tn::connect(&nmp), Ok(()));
        let requests = NFSSTATS.rpcrequests.load(Relaxed);
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &nmp,
            Box::new(|n, c| {
                let st = if n == 1 { NFSERR_TRYLATER as u32 } else { 0 };
                std::vec![Out::Data(tn::reply(c, 0, 0, st, &[]))]
            }),
        );
        let (_, r) = rpc(&fx, &nmp, NFSPROC_GETATTR);
        let calls = peer.calls();
        peer.stop();
        assert_eq!(r, Ok(()));
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[..4], calls[1].1[..4], "the same xid");
        assert_eq!(NFSSTATS.rpcrequests.load(Relaxed), requests + 2);
        tn::unmount(&fx, nmp);
    }

    /// A NULL call over TCP: the connect through lo0, the record mark, the reply's record.
    #[test]
    fn a_call_over_tcp_is_answered() {
        let fx = tn::setup(1000);
        let nmp = tn::nfsmount(&fx, SOCK_STREAM, NFSMNT_NFSV3);
        let peer = tn::Peer::start(
            &fx,
            SOCK_STREAM,
            &nmp,
            Box::new(|_, c| std::vec![Out::Data(tn::reply(c, 0, 0, 0, &[]))]),
        );
        assert_eq!(tn::connect(&nmp), Ok(()));
        let (reply, r) = rpc(&fx, &nmp, NFSPROC_NULL);
        let calls = peer.calls();
        assert_eq!(r, Ok(()));
        assert_eq!(reply.expect("the reply").offset(), 28);
        assert_eq!(calls.len(), 1);
        assert_eq!(words(&calls[0].1)[10], 1000);
        tn::unmount(&fx, nmp);
        peer.stop();
    }

    /// A record mark past `NFS_MAXPACKET`: `EFBIG`, the connection made again and the request
    /// sent again on it (`R_MUSTRESEND`), then answered.
    #[test]
    fn a_record_past_the_maximum_reconnects_and_resends() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_STREAM, NFSMNT_NFSV3);
        let peer = tn::Peer::start(
            &fx,
            SOCK_STREAM,
            &nmp,
            Box::new(|n, c| {
                if n == 1 {
                    std::vec![Out::Mark(NFS_MAXPACKET as u32 + 1)]
                } else {
                    std::vec![Out::Data(tn::reply(c, 0, 0, 0, &[]))]
                }
            }),
        );
        assert_eq!(tn::connect(&nmp), Ok(()));
        let (_, r) = rpc(&fx, &nmp, NFSPROC_NULL);
        let calls = peer.calls();
        assert_eq!(r, Ok(()));
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1, calls[1].1, "resent whole");
        tn::unmount(&fx, nmp);
        peer.stop();
    }

    /// `NFSMNT_NOCONN` over UDP: no connect, each send names the server (the mount's name
    /// mbuf lent), the reply taken from an unconnected socket.
    #[test]
    fn a_call_on_an_unconnected_udp_socket_names_the_server() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3 | NFSMNT_NOCONN);
        assert_eq!(tn::connect(&nmp), Ok(()));
        assert!(!nm_so(&nmp).expect("a socket").has_state(SS_ISCONNECTED));
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &nmp,
            Box::new(|_, c| std::vec![Out::Data(tn::reply(c, 0, 0, 0, &[]))]),
        );
        let (_, a) = rpc(&fx, &nmp, NFSPROC_NULL);
        let (_, b) = rpc(&fx, &nmp, NFSPROC_NULL);
        let calls = peer.calls();
        peer.stop();
        assert_eq!((a, b), (Ok(()), Ok(())));
        assert_eq!(calls.len(), 2);
        tn::unmount(&fx, nmp);
    }

    /// An out-of-memory `pool_get` of the request: `ENOBUFS` with no reply (the module's
    /// deviation, LZ's).
    #[test]
    fn no_request_item_answers_enobufs() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, NFSMNT_NFSV3);
        crate::testutil::fail_pool("nfsreqpl", 1);
        let (reply, r) = rpc(&fx, &nmp, NFSPROC_NULL);
        crate::testutil::fail_pool("nfsreqpl", 0);
        assert!(reply.is_none());
        assert_eq!(r, Err(NfsError::Errno(Errno::ENOBUFS)));
        tn::unmount(&fx, nmp);
    }

    /// `NFSMNT_NOCONN` on a protocol that needs a connection: `ENOTCONN`, the socket gone.
    #[test]
    fn noconn_is_refused_on_tcp() {
        let fx = tn::setup(0);
        let nmp = tn::nfsmount(&fx, SOCK_STREAM, NFSMNT_NOCONN);
        assert_eq!(tn::connect(&nmp), Err(Errno::ENOTCONN));
        assert!(nm_so(&nmp).is_none(), "nfs_disconnect's");
        tn::unmount(&fx, nmp);
    }
}
/* </TESTS> */
