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
 * Copyright (c) 1989, 1993, 1995
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
 *	@(#)nfs.h	8.4 (Berkeley) 5/1/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfs.h>` in the kernel: what the header declares for the client and the server
//! beyond userland's image (`sysdefs::nfs_nfs`, re-exported here). N6.7a0 brings the
//! statistics' counters; N6.7a1 the client's outstanding request (`struct nfsreq`, its flags),
//! the RTT timers, the header's macros over `hz` and `nfs_ticks`, the reply a request ends
//! with ([`Reply`]) and the error it ends with ([`NfsError`]), and the client's pools
//! ([`NfsTables`]).
//!
//! Upstream: sys/nfs/nfs.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfs.rs@e8249449d905
//!
//! ## Deviations
//! - `struct nfsstats` is bumped racily from everywhere in the C (`nfsstats.rpccnt[p]++`):
//!   here each counter is an `AtomicU64` written `Relaxed`, the `uvmexp` idiom; a sysctl
//!   reads them into the C's structure one by one, so the copy is not one instant (the C's
//!   `copyout` of a structure other CPUs bump is not either).
//! - `NFS_HZ`, `NFS_TIMEO`, `NFS_MINTIMEO`, `NFS_MAXTIMEO`, `NFS_MINRTO`, `NFS_MAXRTO` and
//!   `NFS_INITRTT` read `hz` and `nfs_ticks`, so they are functions ([`nfs_hz`], ...), as in
//!   LZ; `NFSIGNORE_SOERROR` is [`nfsignore_soerror`].
//! - A reply status that is no errno(2) number (version 3's `NFSERR_BADHANDLE` ..
//!   `NFSERR_JUKEBOX`, 10001 and up) is carried inside the client as
//!   [`NfsError::Status`], so the C's tests of it match (D-3: `nfs_create`'s retry without
//!   exclusive create, the READDIRPLUS fallback, `EINVAL` for a bad cookie); one that leaves a
//!   vnode operation is `EIO` ([`NfsError::errno`]), where OpenBSD hands the raw number to
//!   userland: narrowed from LZ (which turned every such status into `EIO` inside
//!   `nfs_request`), not closed. `hal`'s `Errno` names errno(2)'s values only (decision D-3's
//!   option (c) is the user's).
//! - `pool_get(&nfsreqpl, PR_WAITOK)` does not zero the item in the C, so its `r_timer`
//!   (which nothing writes) prints garbage in ddb; here every member starts at 0.
//!
//! ## Redesign
//! - LZ's `Nfsstats` was both the counters and the ABI: `#[repr(C)]` atomics viewed as 78
//!   `u64` words through a raw cast (two `unsafe`). The ABI is `hal`'s plain image
//!   ([`Nfsstats`]); the counters are [`NfsCounters`], whose members have the C's names (so
//!   `NFSSTATS.rpccnt[procnum].fetch_add(1, Relaxed)` reads as the C), converted to and from
//!   the image member by member by [`NfsCounters::snapshot`] and [`NfsCounters::restore`]:
//!   the image is built by a structure literal and taken apart whole, so a member missing on
//!   either side does not compile.
//! - LZ's `struct nfsreq` was a `&'static` pool item with `Cell` members, linked through a
//!   `TailqEntry` (two `unsafe` to link and unlink it, one to cast the pool item, one `unsafe
//!   impl Sync`), with `r_procp` a raw `*const Proc`, `r_mreq`, `r_mrep` and `r_md` parked
//!   mbufs and `r_dpos` a raw position in one. [`NfsReq`] is a counted item of "nfsreqpl"
//!   ([`NfsTables::nfsreqpl`]) that `nfs_request`'s frame holds, linked on the mount's
//!   `nm_reqsq` (`hal::ds::pinlist`, bound to the kernel lock) through its own `r_chain`. The C
//!   keeps every member under the kernel lock: the scalars are relaxed atomics written there
//!   (`IDIOMS`' "a few words read unlocked"), the request's chain and its reply one cell of
//!   the kernel lock opened only by an exclusive witness ([`PART_REQ`]); whether a reply is in
//!   is an atomic beside its cell ([`NfsReq::has_reply`]), for the tests the C makes of
//!   `r_mrep` under a reader. The request is counted so that `nfs_timer`'s walk can keep its
//!   place across a send it makes without the lock's witness: one count pair per
//!   retransmission, none per tick. `r_procp` is the requester's thread id, found again with
//!   `tfind` by the timer (the thread sleeps in `nfs_reply` while its request is queued);
//!   `r_nmp` is gone: every function that reaches a request is given its mount; `r_vp` is
//!   the vnode's address for ddb's line, never dereferenced.
//! - The reply is [`Reply`]: the chain and the bytes consumed of it (`r_md`/`r_dpos`), read
//!   with a fresh `XdrReader::at`; nothing points into the chain while it waits.

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use hal::cpu::intr::IPL_NONE;
use hal::ds::pinlist::PinEntry;
use hal::mem::pool::{BoxPool, PoolHome, TypedPool};
use hal::sync::mutex::Mutexed;
use hal::sync::rwlock::{ExclGuarded, Guarded, Rwlocked};

use super::nfs_kq::{Kevq, NfsKevq};
use super::nfs_node::NfsNhash;
use super::nfs_syscalls::NFS_NASYNCDAEMON;
use super::nfs_vnops::{NfsFifoVnops, NfsSpecVnops, NfsVnops};
use super::nfsdiskless::NfsDiskless;
use super::nfsm_subs::XdrReader;
use super::nfsmount::NfsMount;
use super::nfsnode::NfsNode;
use crate::fs::buf::B_INVAL;
use crate::fs::bufq::Fifo;
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::proto::PR_CONNREQUIRED;
use crate::proc::ProcGlue;
use sysdefs::errno::Errno;
pub use sysdefs::nfs_nfs::*;
use sysdefs::nfs_nfsproto::NFS_NPROCS;
use sysdefs::signal::{SIGHUP, SIGINT, SIGKILL, SIGQUIT, SIGTERM, Sigset, sigmask};
use sysdefs::types::Pid;

/// `B_INVAFTERWRITE`: the buffer flag `nfs_bioread` sets on a block whose valid range does
/// not cover a read, so that its release after the write invalidates it (`B_INVAL`).
pub const B_INVAFTERWRITE: i64 = B_INVAL;

/// `NFSINT_SIGMASK`: the set of signals that interrupt an I/O in progress for `NFSMNT_INT`
/// mounts. What should be in this set is open to debate, but I believe that since I/O
/// system calls on ufs are never interrupted by signals the set should be minimal. My
/// reasoning is that many current programs that use signals such as SIGALRM will not expect
/// file I/O system calls to be interrupted by them and break.
pub const NFSINT_SIGMASK: Sigset =
    sigmask(SIGINT) | sigmask(SIGTERM) | sigmask(SIGKILL) | sigmask(SIGHUP) | sigmask(SIGQUIT);

/// `R_TIMING`: timing request (in mntp); flag values for `r_flags`.
pub const R_TIMING: i32 = 0x01;
/// `R_SENT`: request has been sent.
pub const R_SENT: i32 = 0x02;
/// `R_SOFTTERM`: soft mnt, too many retries.
pub const R_SOFTTERM: i32 = 0x04;
/// `R_INTR`: intr mnt, signal pending.
pub const R_INTR: i32 = 0x08;
/// `R_SOCKERR`: fatal error on socket.
pub const R_SOCKERR: i32 = 0x10;
/// `R_TPRINTFMSG`: did a tprintf msg.
pub const R_TPRINTFMSG: i32 = 0x20;
/// `R_MUSTRESEND`: must resend request.
pub const R_MUSTRESEND: i32 = 0x40;

/// `NFS_DEFAULT_TIMER` (`enum nfs_rto_timers`): the RTT estimator a procedure uses; the
/// values but the default index `nm_srtt` and `nm_sdrtt` (minus one).
pub const NFS_DEFAULT_TIMER: usize = 0;
/// `NFS_GETATTR_TIMER`.
pub const NFS_GETATTR_TIMER: usize = 1;
/// `NFS_LOOKUP_TIMER`.
pub const NFS_LOOKUP_TIMER: usize = 2;
/// `NFS_READ_TIMER`.
pub const NFS_READ_TIMER: usize = 3;
/// `NFS_WRITE_TIMER`.
pub const NFS_WRITE_TIMER: usize = 4;
/// `NFS_MAX_TIMER`.
pub const NFS_MAX_TIMER: usize = NFS_WRITE_TIMER;

/// The kernel lock's part that opens a request's chain and reply cells, apart from the part
/// that walks `nm_reqsq`, so `nfs_reply`'s match moves a reply into the request it finds
/// under the walk's one witness (`Divisible::split`).
pub const PART_REQ: u8 = 1;

/// The kernel's `nfsstats`: one atomic per member of `struct nfsstats`, in the C's order,
/// bumped without a lock as the C bumps its words.
pub struct NfsCounters {
    /// `attrcache_hits`.
    pub attrcache_hits: AtomicU64,
    /// `attrcache_misses`.
    pub attrcache_misses: AtomicU64,
    /// `lookupcache_hits`.
    pub lookupcache_hits: AtomicU64,
    /// `lookupcache_misses`.
    pub lookupcache_misses: AtomicU64,
    /// `direofcache_hits`.
    pub direofcache_hits: AtomicU64,
    /// `direofcache_misses`.
    pub direofcache_misses: AtomicU64,
    /// `biocache_reads`.
    pub biocache_reads: AtomicU64,
    /// `read_bios`.
    pub read_bios: AtomicU64,
    /// `read_physios`.
    pub read_physios: AtomicU64,
    /// `biocache_writes`.
    pub biocache_writes: AtomicU64,
    /// `write_bios`.
    pub write_bios: AtomicU64,
    /// `write_physios`.
    pub write_physios: AtomicU64,
    /// `biocache_readlinks`.
    pub biocache_readlinks: AtomicU64,
    /// `readlink_bios`.
    pub readlink_bios: AtomicU64,
    /// `biocache_readdirs`.
    pub biocache_readdirs: AtomicU64,
    /// `readdir_bios`.
    pub readdir_bios: AtomicU64,
    /// `rpccnt`: one per procedure.
    pub rpccnt: [AtomicU64; NFS_NPROCS],
    /// `rpcretries`.
    pub rpcretries: AtomicU64,
    /// `srvrpccnt`: one per procedure.
    pub srvrpccnt: [AtomicU64; NFS_NPROCS],
    /// `srvrpc_errs`.
    pub srvrpc_errs: AtomicU64,
    /// `srv_errs`.
    pub srv_errs: AtomicU64,
    /// `rpcrequests`.
    pub rpcrequests: AtomicU64,
    /// `rpctimeouts`.
    pub rpctimeouts: AtomicU64,
    /// `rpcunexpected`.
    pub rpcunexpected: AtomicU64,
    /// `rpcinvalid`.
    pub rpcinvalid: AtomicU64,
    /// `srvcache_inproghits`.
    pub srvcache_inproghits: AtomicU64,
    /// `srvcache_idemdonehits`.
    pub srvcache_idemdonehits: AtomicU64,
    /// `srvcache_nonidemdonehits`.
    pub srvcache_nonidemdonehits: AtomicU64,
    /// `srvcache_misses`.
    pub srvcache_misses: AtomicU64,
    /// `forcedsync`.
    pub forcedsync: AtomicU64,
    /// `srvnqnfs_leases`.
    pub srvnqnfs_leases: AtomicU64,
    /// `srvnqnfs_maxleases`.
    pub srvnqnfs_maxleases: AtomicU64,
    /// `srvnqnfs_getleases`.
    pub srvnqnfs_getleases: AtomicU64,
    /// `srvvop_writes`.
    pub srvvop_writes: AtomicU64,
}

impl NfsCounters {
    /// All counters zero.
    pub const fn new() -> Self {
        Self {
            attrcache_hits: AtomicU64::new(0),
            attrcache_misses: AtomicU64::new(0),
            lookupcache_hits: AtomicU64::new(0),
            lookupcache_misses: AtomicU64::new(0),
            direofcache_hits: AtomicU64::new(0),
            direofcache_misses: AtomicU64::new(0),
            biocache_reads: AtomicU64::new(0),
            read_bios: AtomicU64::new(0),
            read_physios: AtomicU64::new(0),
            biocache_writes: AtomicU64::new(0),
            write_bios: AtomicU64::new(0),
            write_physios: AtomicU64::new(0),
            biocache_readlinks: AtomicU64::new(0),
            readlink_bios: AtomicU64::new(0),
            biocache_readdirs: AtomicU64::new(0),
            readdir_bios: AtomicU64::new(0),
            rpccnt: [const { AtomicU64::new(0) }; NFS_NPROCS],
            rpcretries: AtomicU64::new(0),
            srvrpccnt: [const { AtomicU64::new(0) }; NFS_NPROCS],
            srvrpc_errs: AtomicU64::new(0),
            srv_errs: AtomicU64::new(0),
            rpcrequests: AtomicU64::new(0),
            rpctimeouts: AtomicU64::new(0),
            rpcunexpected: AtomicU64::new(0),
            rpcinvalid: AtomicU64::new(0),
            srvcache_inproghits: AtomicU64::new(0),
            srvcache_idemdonehits: AtomicU64::new(0),
            srvcache_nonidemdonehits: AtomicU64::new(0),
            srvcache_misses: AtomicU64::new(0),
            forcedsync: AtomicU64::new(0),
            srvnqnfs_leases: AtomicU64::new(0),
            srvnqnfs_maxleases: AtomicU64::new(0),
            srvnqnfs_getleases: AtomicU64::new(0),
            srvvop_writes: AtomicU64::new(0),
        }
    }

    /// The counters as `struct nfsstats`, what `vfs.nfs.nfsstats` copies out. The structure
    /// literal names every member of the image, so one missing here does not compile.
    pub fn snapshot(&self) -> Nfsstats {
        let ld = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let lda = |a: &[AtomicU64; NFS_NPROCS]| core::array::from_fn(|i| ld(&a[i]));
        Nfsstats {
            attrcache_hits: ld(&self.attrcache_hits),
            attrcache_misses: ld(&self.attrcache_misses),
            lookupcache_hits: ld(&self.lookupcache_hits),
            lookupcache_misses: ld(&self.lookupcache_misses),
            direofcache_hits: ld(&self.direofcache_hits),
            direofcache_misses: ld(&self.direofcache_misses),
            biocache_reads: ld(&self.biocache_reads),
            read_bios: ld(&self.read_bios),
            read_physios: ld(&self.read_physios),
            biocache_writes: ld(&self.biocache_writes),
            write_bios: ld(&self.write_bios),
            write_physios: ld(&self.write_physios),
            biocache_readlinks: ld(&self.biocache_readlinks),
            readlink_bios: ld(&self.readlink_bios),
            biocache_readdirs: ld(&self.biocache_readdirs),
            readdir_bios: ld(&self.readdir_bios),
            rpccnt: lda(&self.rpccnt),
            rpcretries: ld(&self.rpcretries),
            srvrpccnt: lda(&self.srvrpccnt),
            srvrpc_errs: ld(&self.srvrpc_errs),
            srv_errs: ld(&self.srv_errs),
            rpcrequests: ld(&self.rpcrequests),
            rpctimeouts: ld(&self.rpctimeouts),
            rpcunexpected: ld(&self.rpcunexpected),
            rpcinvalid: ld(&self.rpcinvalid),
            srvcache_inproghits: ld(&self.srvcache_inproghits),
            srvcache_idemdonehits: ld(&self.srvcache_idemdonehits),
            srvcache_nonidemdonehits: ld(&self.srvcache_nonidemdonehits),
            srvcache_misses: ld(&self.srvcache_misses),
            forcedsync: ld(&self.forcedsync),
            srvnqnfs_leases: ld(&self.srvnqnfs_leases),
            srvnqnfs_maxleases: ld(&self.srvnqnfs_maxleases),
            srvnqnfs_getleases: ld(&self.srvnqnfs_getleases),
            srvvop_writes: ld(&self.srvvop_writes),
        }
    }

    /// Overwrites the counters with `v`: what the `vfs.nfs.nfsstats` sysctl does when it is
    /// written (`nfs_sysctl`'s `copyin`). The image is destructured whole, so a member left
    /// out here does not compile.
    pub fn restore(&self, v: &Nfsstats) {
        let st = |c: &AtomicU64, w: u64| c.store(w, Ordering::Relaxed);
        let sta = |a: &[AtomicU64; NFS_NPROCS], w: [u64; NFS_NPROCS]| {
            for (c, w) in a.iter().zip(w) {
                st(c, w);
            }
        };
        let Nfsstats {
            attrcache_hits,
            attrcache_misses,
            lookupcache_hits,
            lookupcache_misses,
            direofcache_hits,
            direofcache_misses,
            biocache_reads,
            read_bios,
            read_physios,
            biocache_writes,
            write_bios,
            write_physios,
            biocache_readlinks,
            readlink_bios,
            biocache_readdirs,
            readdir_bios,
            rpccnt,
            rpcretries,
            srvrpccnt,
            srvrpc_errs,
            srv_errs,
            rpcrequests,
            rpctimeouts,
            rpcunexpected,
            rpcinvalid,
            srvcache_inproghits,
            srvcache_idemdonehits,
            srvcache_nonidemdonehits,
            srvcache_misses,
            forcedsync,
            srvnqnfs_leases,
            srvnqnfs_maxleases,
            srvnqnfs_getleases,
            srvvop_writes,
        } = *v;
        st(&self.attrcache_hits, attrcache_hits);
        st(&self.attrcache_misses, attrcache_misses);
        st(&self.lookupcache_hits, lookupcache_hits);
        st(&self.lookupcache_misses, lookupcache_misses);
        st(&self.direofcache_hits, direofcache_hits);
        st(&self.direofcache_misses, direofcache_misses);
        st(&self.biocache_reads, biocache_reads);
        st(&self.read_bios, read_bios);
        st(&self.read_physios, read_physios);
        st(&self.biocache_writes, biocache_writes);
        st(&self.write_bios, write_bios);
        st(&self.write_physios, write_physios);
        st(&self.biocache_readlinks, biocache_readlinks);
        st(&self.readlink_bios, readlink_bios);
        st(&self.biocache_readdirs, biocache_readdirs);
        st(&self.readdir_bios, readdir_bios);
        sta(&self.rpccnt, rpccnt);
        st(&self.rpcretries, rpcretries);
        sta(&self.srvrpccnt, srvrpccnt);
        st(&self.srvrpc_errs, srvrpc_errs);
        st(&self.srv_errs, srv_errs);
        st(&self.rpcrequests, rpcrequests);
        st(&self.rpctimeouts, rpctimeouts);
        st(&self.rpcunexpected, rpcunexpected);
        st(&self.rpcinvalid, rpcinvalid);
        st(&self.srvcache_inproghits, srvcache_inproghits);
        st(&self.srvcache_idemdonehits, srvcache_idemdonehits);
        st(&self.srvcache_nonidemdonehits, srvcache_nonidemdonehits);
        st(&self.srvcache_misses, srvcache_misses);
        st(&self.forcedsync, forcedsync);
        st(&self.srvnqnfs_leases, srvnqnfs_leases);
        st(&self.srvnqnfs_maxleases, srvnqnfs_maxleases);
        st(&self.srvnqnfs_getleases, srvnqnfs_getleases);
        st(&self.srvvop_writes, srvvop_writes);
    }
}

impl Default for NfsCounters {
    fn default() -> Self {
        Self::new()
    }
}

/// A reply an RPC ends with (`r_mrep` with `r_md`/`r_dpos`, then the caller's `nmi_mrep`
/// with `nmi_md`/`nmi_dpos`): the chain, which it owns, and the bytes of it consumed so far.
/// Read with [`Reply::reader`], a cursor at that offset; the chain is freed with the value.
pub struct Reply {
    /// The reply's chain, its first mbuf the datagram's or the record's.
    m: MbufBox,
    /// The bytes of the chain consumed (the RPC header's, after `nfs_request`).
    off: usize,
}

impl Reply {
    /// The reply `m`, `off` bytes of it consumed.
    pub fn new(m: MbufBox, off: usize) -> Self {
        Self { m, off }
    }

    /// A cursor at the consumed offset (`info.nmi_md = rep->r_md; info.nmi_dpos =
    /// rep->r_dpos`): the walk from the chain's first mbuf to it, `EBADRPC` past its end.
    pub fn reader(&self) -> Result<XdrReader<'_>, Errno> {
        XdrReader::at(&self.m, self.off)
    }

    /// The bytes of the chain consumed.
    pub fn offset(&self) -> usize {
        self.off
    }

    /// Records that `off` bytes are consumed (a cursor's [`XdrReader::offset`] after a
    /// dissection whose position the reply keeps).
    pub fn set_offset(&mut self, off: usize) {
        self.off = off;
    }

    /// The chain.
    pub fn chain(&self) -> &Mbuf {
        &self.m
    }

    /// The chain, given up.
    pub fn into_chain(self) -> MbufBox {
        self.m
    }
}

/// How an RPC failed: an errno(2) value (the transport's, the RPC layer's, or a reply
/// status that is one), or a reply status that is no errno number (D-3: version 3's
/// `NFSERR_BADHANDLE` .. `NFSERR_JUKEBOX`), which the C hands up as the same `int`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfsError {
    /// An errno(2) value.
    Errno(Errno),
    /// A reply status that names no errno(2) value.
    Status(u32),
}

impl NfsError {
    /// `error = fxdr_unsigned(int, *tl)` of a reply's non-zero status: the errno it names,
    /// or the status itself.
    pub fn from_status(status: u32) -> Self {
        match Errno::from_raw(status as i32) {
            Some(e) => Self::Errno(e),
            None => Self::Status(status),
        }
    }

    /// The C's `int`: the errno's number or the status (what `error == NFSERR_NOTSUPP`
    /// compares).
    pub fn raw(self) -> i32 {
        match self {
            Self::Errno(e) => e as i32,
            Self::Status(s) => s as i32,
        }
    }

    /// The error a vnode operation answers: the errno, or `EIO` for a status that names
    /// none (the module's deviation).
    pub fn errno(self) -> Errno {
        match self {
            Self::Errno(e) => e,
            Self::Status(_) => Errno::EIO,
        }
    }
}

impl From<Errno> for NfsError {
    fn from(e: Errno) -> Self {
        Self::Errno(e)
    }
}

/// A request's two chains, which the C keeps under the kernel lock: one cell, so the request
/// carries one lock key for both.
pub struct NfsReqChains {
    /// `r_mreq`: the request, its RPC header (and, on a stream, its record mark) first, copied
    /// for each send; `None` only while a copy that may sleep holds it.
    pub r_mreq: Option<MbufBox>,
    /// `r_mrep` with `r_md`/`r_dpos`: the reply `nfs_reply`'s match moved in.
    pub r_mrep: Option<Reply>,
}

/// `struct nfsreq`: an NFS outstanding request list element, a counted item of "nfsreqpl"
/// that `nfs_request` holds from its `pool_get` to its `pool_put` (the module's Redesign).
/// The mount's `nm_reqsq` links it while it waits for its reply.
pub struct NfsReq<G: ProcGlue> {
    /// `r_chain`: the link on the mount's `nm_reqsq` (the kernel lock).
    r_chain: PinEntry<NfsReq<G>>,
    /// The item's home in "nfsreqpl" (`PoolArc::from_ref`).
    r_home: PoolHome<NfsReq<G>>,
    /// `r_mreq` and `r_mrep`: the request's chain and its reply, one cell of part
    /// [`PART_REQ`] of the kernel lock.
    pub r_m: ExclGuarded<NfsReqChains>,
    /// `r_mrep != NULL`: set with the reply under the match's witness, cleared with it.
    r_hasrep: AtomicBool,
    /// `r_mreq->m_pkthdr.len`: the request's length, which does not change.
    pub r_len: i32,
    /// `r_vp`: the vnode's address, for ddb's line; never dereferenced.
    pub r_vp: usize,
    /// `r_xid`: the transaction id, a raw XDR word.
    pub r_xid: u32,
    /// `r_flags`: flags on request, see `R_*`.
    pub r_flags: AtomicI32,
    /// `r_rexmit`: current retrans count.
    pub r_rexmit: AtomicI32,
    /// `r_timer`: tick counter on reply (nothing writes it, as in the C).
    pub r_timer: i32,
    /// `r_procnum`: NFS procedure number (an index of `NFS_NPROCS`).
    pub r_procnum: u32,
    /// `r_rtt`: RTT for rpc.
    pub r_rtt: AtomicI32,
    /// `r_procp`: the thread that did the I/O system call, by its id (`None`: the C's NULL).
    pub r_procp: Option<Pid>,
}

hal::pin_adapter!(
    /// `TAILQ_HEAD(reqs, nfsreq)`: a mount's outstanding requests, through `r_chain`.
    impl[G: ProcGlue] pub RChain<G>: NfsReq<G>, r_chain
);

hal::pool_home!(impl[G: ProcGlue] NfsReq<G>, r_home);

impl<G: ProcGlue> NfsReq<G> {
    /// A request for procedure `procnum`, its transaction id `xid`, its chain `mreq` (the
    /// header made), its requester's thread id and its vnode's address; every count 0.
    pub fn new(procnum: u32, xid: u32, mreq: MbufBox, procp: Option<Pid>, vp: usize) -> Self {
        let len = mreq.pkthdr().map_or(0, |h| h.len.get());
        Self {
            r_chain: PinEntry::new(),
            r_home: PoolHome::new(),
            r_m: ExclGuarded::keyed(
                hal::sync::klock::kernel_lock_id(),
                PART_REQ,
                NfsReqChains {
                    r_mreq: Some(mreq),
                    r_mrep: None,
                },
            ),
            r_hasrep: AtomicBool::new(false),
            r_len: len,
            r_vp: vp,
            r_xid: xid,
            r_flags: AtomicI32::new(0),
            r_rexmit: AtomicI32::new(0),
            r_timer: 0,
            r_procnum: procnum,
            r_rtt: AtomicI32::new(0),
            r_procp: procp,
        }
    }

    /// `rep->r_mrep != NULL`, readable under any witness or none (the timer's, the receive
    /// lock's tests).
    #[inline]
    pub fn has_reply(&self) -> bool {
        self.r_hasrep.load(Ordering::Relaxed)
    }

    /// `rep->r_mrep = m` (and `r_md`, `r_dpos`), for the holder of an exclusive witness
    /// that opens [`PART_REQ`]; the reply there before, if any, is given back.
    pub fn set_reply<W: hal::sync::rwlock::Exclusive + ?Sized>(
        &self,
        w: &mut W,
        rep: Option<Reply>,
    ) -> Option<Reply> {
        self.r_hasrep.store(rep.is_some(), Ordering::Relaxed);
        core::mem::replace(&mut self.r_m.get_mut(w).r_mrep, rep)
    }

    /// Whether `r_chain` links the request on a list.
    pub fn is_queued(&self) -> bool {
        self.r_chain.is_linked()
    }

    /// `rep->r_flags & f`.
    #[inline]
    pub fn flags(&self) -> i32 {
        self.r_flags.load(Ordering::Relaxed)
    }

    /// `rep->r_flags |= f`.
    #[inline]
    pub fn set_flags(&self, f: i32) {
        self.r_flags.fetch_or(f, Ordering::Relaxed);
    }

    /// `rep->r_flags &= ~f`.
    #[inline]
    pub fn clear_flags(&self, f: i32) {
        self.r_flags.fetch_and(!f, Ordering::Relaxed);
    }
}

/// The client's globals of one glue (`nfs.h`'s externs and the C files' own): the pools
/// "nfsreqpl" (defined in the C's `nfs_subs.c`) and "nfsnodepl" (`nfs_node.c`), made by
/// `nfs_init` and `nfs_vfs_init`, the node hash that stands for every mount's `nm_ntree`
/// (`nfs_node`), the nfsiods' queue (`nfs_bio.c`'s `nfs_bufq`) and table
/// (`nfs_syscalls.c`'s `nfs_asyncdaemon[]`), and the kq poller's watch list (`nfs_kq.c`).
pub struct NfsTables<G: ProcGlue> {
    /// `nfsreqpl`: the outstanding requests.
    pub nfsreqpl: TypedPool<NfsReq<G>>,
    /// `nfs_node_pool` ("nfsnodepl"): the nodes.
    pub nfs_node_pool: TypedPool<NfsNode<G>>,
    /// `nm_ntree` of every mount: the node hash's fixed chains and its key.
    pub nhash: NfsNhash<G>,
    /// `nfs_bufq`: the buffers `nfs_asyncio` queued for the nfsiods, their tokens parked in
    /// them, under the kernel lock (the C's: `nfs_bio`'s Redesign); its address is the
    /// nfsiods' wait channel.
    pub nfs_bufq: Guarded<Fifo>,
    /// `nfs_asyncdaemon[NFS_MAXASYNCDAEMON]`: the running nfsiods' thread ids, 0 for a free
    /// slot.
    pub nfs_asyncdaemon: [AtomicI32; NFS_NASYNCDAEMON],
    /// `kevlist` and `pnfskq` under `nfskevq_lock` (`nfs_kq`).
    pub kevq: Rwlocked<NfsKevq<G>>,
    /// The watch list's entries (`nfs_kq`'s Deviations: the C's `malloc(M_KEVENT)`).
    pub kevqpl: BoxPool<Kevq<G>>,
    /// `nfs_vops`: the operations of the client's vnodes (`nfs_vnops`; one address per glue,
    /// for `vops_id`).
    pub vops: NfsVnops<G>,
    /// `nfs_specvops`: the operations of a device on an NFS mount.
    pub specvops: NfsSpecVnops<G>,
    /// `nfs_fifovops`: the operations of a fifo on an NFS mount (installed by nothing while
    /// fifofs is not ported, `nfs_vnops`' Deviations).
    pub fifovops: NfsFifoVnops<G>,
    /// `nfs_diskless` (`nfs_vfsops.c`): what `nfs_mountroot` booted from, kept once it is
    /// filled (`nfs_vfsops`' Deviations).
    pub nfs_diskless: Mutexed<Option<NfsDiskless<G>>>,
}

impl<G: ProcGlue> NfsTables<G> {
    /// No pool made yet, the queues and the table empty.
    pub const fn new() -> Self {
        Self {
            nfsreqpl: TypedPool::new(),
            nfs_node_pool: TypedPool::new(),
            nhash: NfsNhash::new(),
            nfs_bufq: Guarded::new_kernel(Fifo::new()),
            nfs_asyncdaemon: [const { AtomicI32::new(0) }; NFS_NASYNCDAEMON],
            kevq: Rwlocked::new("nfskqlk", NfsKevq::new()),
            kevqpl: BoxPool::new(),
            vops: NfsVnops::new(),
            specvops: NfsSpecVnops::new(),
            fifovops: NfsFifoVnops::new(),
            nfs_diskless: Mutexed::new(IPL_NONE, None),
        }
    }
}

impl<G: ProcGlue> Default for NfsTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `nfs_niothreads`: the number of NFS I/O threads, `-1` before they are started (defined
/// in the C's `nfs_syscalls.c`; lowercase, since `NFS_NIOTHREADS` is the sysctl identifier).
#[allow(non_upper_case_globals)] // the C name
pub static nfs_niothreads: AtomicI32 = AtomicI32::new(-1);

/// `NFS_HZ`: ticks per second of the NFS timer (`hz / nfs_ticks`).
pub fn nfs_hz() -> i32 {
    crate::time::timeout::hz() / super::nfs_subs::NFS_TICKS.load(Ordering::Relaxed).max(1)
}

/// `NFS_TIMEO`: default timeout = 1 second.
pub fn nfs_timeo() -> i32 {
    nfs_hz()
}

/// `NFS_MINTIMEO`: min timeout to use.
pub fn nfs_mintimeo() -> i32 {
    nfs_hz()
}

/// `NFS_MAXTIMEO`: max timeout to backoff to.
pub fn nfs_maxtimeo() -> i32 {
    60 * nfs_hz()
}

/// `NFS_MINRTO`: on fast networks, the estimator will try to reduce the timeout lower than
/// the latency of the server's disks, which results in too many timeouts, so cap the lower
/// bound.
pub fn nfs_minrto() -> i32 {
    nfs_hz() >> 2
}

/// `NFS_MAXRTO`: keep the RTO from increasing to unreasonably large values when a server is
/// not responding.
pub fn nfs_maxrto() -> i32 {
    20 * nfs_hz()
}

/// `NFS_INITRTT`.
pub fn nfs_initrtt() -> i32 {
    nfs_hz() << 3
}

/// `NFSIGNORE_SOERROR(s, e)`: socket errors ignored for connectionless sockets?? For now,
/// ignore them all. `s` is the protocol's `pr_flags`.
pub fn nfsignore_soerror(s: i32, e: Errno) -> bool {
    e != Errno::EINTR
        && e != Errno::ERESTART
        && e != Errno::EWOULDBLOCK
        && s & i32::from(PR_CONNREQUIRED) == 0
}

/// `NFS_ISV3(vp)`: whether the vnode's mount `nmp` (the caller's `VFSTONFS(vp->v_mount)`,
/// lent) speaks version 3.
#[inline]
pub fn nfs_isv3<G: ProcGlue>(nmp: &NfsMount<G>) -> bool {
    nmp.flag() & sysdefs::mount::NFSMNT_NFSV3 != 0
}

/// `NFS_CMPFH(n, f, s)`: whether the node's handle is the `fh` of a reply (the same size, the
/// same bytes).
#[inline]
pub fn nfs_cmpfh<G: ProcGlue>(np: &NfsNode<G>, fh: &[u8]) -> bool {
    np.fh().bytes() == fh
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the counters and their image: every member reaches its C word.

    use super::*;

    #[test]
    fn snapshot_is_in_c_order() {
        let st = NfsCounters::new();
        st.attrcache_hits.store(1, Ordering::Relaxed);
        st.rpccnt[0].store(2, Ordering::Relaxed);
        st.rpccnt[NFS_NPROCS - 1].store(3, Ordering::Relaxed);
        st.rpcretries.store(4, Ordering::Relaxed);
        st.srvvop_writes.store(5, Ordering::Relaxed);
        let w = st.snapshot();
        let words: &[u8] = hal::user::bytes_of(core::slice::from_ref(&w));
        let word = |i: usize| u64::from_ne_bytes(words[i * 8..i * 8 + 8].try_into().unwrap());
        assert_eq!(word(0), 1);
        assert_eq!(word(16), 2);
        assert_eq!(word(16 + NFS_NPROCS - 1), 3);
        assert_eq!(word(16 + NFS_NPROCS), 4);
        assert_eq!(word(Nfsstats::NWORDS - 1), 5);
    }

    #[test]
    fn restore_then_snapshot_is_the_identity_on_every_word() {
        // Each of the 78 words gets its own value, so a member mapped to the wrong one (or
        // left out of `restore`) shows.
        let mut img = Nfsstats::new();
        let bytes = hal::user::bytes_of_mut(core::slice::from_mut(&mut img));
        for (i, w) in bytes.chunks_exact_mut(8).enumerate() {
            w.copy_from_slice(&(i as u64 * 0x0101_0101 + 7).to_ne_bytes());
        }
        let st = NfsCounters::new();
        st.restore(&img);
        assert_eq!(st.snapshot(), img);
        assert_eq!(st.srvrpccnt[3].load(Ordering::Relaxed), img.srvrpccnt[3]);
    }

    /// The "nfsreqpl" item's size, member by member (D-6: `vmstat -m` shows it under the C's
    /// pool name; S11 stops above `bsd`'s item plus 64 bytes).
    #[test]
    fn the_nfsreqpl_item_size_member_by_member() {
        use crate::fs::testfs::TestGlue as G;
        use core::mem::size_of;
        let rows = [
            ("r_chain", size_of::<PinEntry<NfsReq<G>>>()),
            ("r_home", size_of::<PoolHome<NfsReq<G>>>()),
            ("r_m", size_of::<ExclGuarded<NfsReqChains>>()),
            ("r_vp", 8),
            ("r_procnum", 4),
            ("r_procp", size_of::<Option<Pid>>()),
            ("scalars", 1 + 4 * 6),
        ];
        let mut sum = 0;
        for (n, z) in rows {
            std::println!("{n}: {z}");
            sum += z;
        }
        std::println!("members {sum}, NfsReq<G> {}", size_of::<NfsReq<G>>());
        assert_eq!(size_of::<NfsReq<G>>(), NFSREQ_SIZE);
        // 144 bytes, 160 with the counted item's header (the count and the pool), against
        // bsd's 104 (LZ's shape, measured at the slice's base with a compile-time probe of
        // `size_of::<NfsReq>()`) and the C's 96 on LP64: the pinned entry (32: a lock
        // identity beside the two links), the cell of the two chains (48: its lock key), the
        // home (8) and the header are the growth, within the brief's bound (S11: 104 + 64).
        assert!(NFSREQ_SIZE + 16 <= 104 + 64, "S11: the \"nfsreqpl\" item");
    }

    /// D-3: a status that is an errno number is that errno; one that is not stays the status
    /// inside the client (the three tests of the C compare it) and is `EIO` outward.
    #[test]
    fn a_reply_status_is_an_errno_or_stays_a_status() {
        use sysdefs::nfs_nfsproto::{NFSERR_BAD_COOKIE, NFSERR_NOTSUPP, NFSERR_STALE};
        let stale = NfsError::from_status(NFSERR_STALE as u32);
        assert_eq!(stale, NfsError::Errno(Errno::ESTALE));
        assert_eq!(stale.raw(), NFSERR_STALE);
        let notsupp = NfsError::from_status(NFSERR_NOTSUPP as u32);
        assert_eq!(notsupp, NfsError::Status(10004));
        assert_eq!(notsupp.raw(), NFSERR_NOTSUPP);
        assert_eq!(notsupp.errno(), Errno::EIO);
        assert_eq!(NfsError::from_status(NFSERR_BAD_COOKIE as u32).raw(), 10003);
        // The C's int: an all-ones status is -1, ERESTART, as the C hands it up.
        assert_eq!(
            NfsError::from_status(u32::MAX),
            NfsError::Errno(Errno::ERESTART)
        );
        assert_eq!(NfsError::from(Errno::EINTR).errno(), Errno::EINTR);
    }

    #[test]
    fn socket_errors_are_ignored_only_on_connectionless_sockets() {
        for e in [Errno::EINTR, Errno::ERESTART, Errno::EWOULDBLOCK] {
            assert!(!nfsignore_soerror(0, e), "{e:?}");
        }
        assert!(nfsignore_soerror(0, Errno::EHOSTUNREACH));
        assert!(!nfsignore_soerror(
            i32::from(PR_CONNREQUIRED),
            Errno::EHOSTUNREACH
        ));
    }

    /// `size_of::<NfsReq<G>>()` measured by the test above.
    const NFSREQ_SIZE: usize = 144;

    /// The request flags, the last RTO timer and `B_INVAFTERWRITE` are the C header's.
    #[test]
    #[ignore = "reads reference/openbsd-src (just test-ref)"]
    fn request_flags_are_the_c_header_s() {
        let defs = crate::reftest::defines("sys/nfs/nfs.h");
        let _ = crate::reftest::assert_defines!(
            defs;
            R_TIMING,
            R_SENT,
            R_SOFTTERM,
            R_INTR,
            R_SOCKERR,
            R_TPRINTFMSG,
            R_MUSTRESEND
        );
        assert_eq!(
            defs.get("NFS_MAX_TIMER").map(std::string::String::as_str),
            Some("(NFS_WRITE_TIMER)")
        );
        assert_eq!(
            defs.get("B_INVAFTERWRITE").map(std::string::String::as_str),
            Some("B_INVAL")
        );
    }
}
/* </TESTS> */
