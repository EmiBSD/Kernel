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
 * Copyright (c) 1989, 1993
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
 *	@(#)nfs_subs.c	8.8 (Berkeley) 5/22/95
 */
/* </LICENSES> */

/* <CODE> */
//! `nfs/nfs_subs.c` in the kernel: the XDR words `nfs_init` computes, the type tables of the
//! conversion macros, the statistics, `txdr_nfsv2time`, and the functions that support the
//! `nfsm_subs.h` cursors: building a request (`nfsm_reqhead`, `nfsm_build`,
//! `nfsm_uiotombuf`, `nfsm_buftombuf`, `nfsm_strtombuf`, `nfsm_fhtom`, `nfsm_v3attrbuild`)
//! and taking a reply apart (`nfsm_disct`, `nfs_adv`, `nfsm_mbuftouio`), as methods of the
//! cursors of `nfsm_subs` ([`XdrWriter`], [`XdrReader`]).
//!
//! Upstream: sys/nfs/nfs_subs.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_subs.rs@e8249449d905
//!
//! ## Deviations
//! - The XDR words `nfs_init` computes at startup (`rpc_call`, `nfs_true`, `nfs_xdrneg1`,
//!   ...) are `const`s computed at compile time, with their C names (lowercase:
//!   `RPC_CALL` is already the `rpcv2.h` constant).
//! - `nfsstats` is [`NFSSTATS`], counters of atomics (`nfs`).
//! - `nfsm_uiotombuf` ignores `uiomove`'s error as the C does; the bytes a failed move did not
//!   fill are unspecified in the C (whatever the fresh mbuf held: `MGET` does not zero), and
//!   zeros are chosen here, one of the C's outcomes, so no stale kernel bytes go on the wire.
//! - `nfs_get_xid`'s context is seeded at its first call under a mutex of its own
//!   ([`NFS_XID_CTX`]): the C's `static int called` and `static struct idgen32_ctx`, which
//!   the kernel lock serialises there, are one `Mutexed` here (one lock pair per RPC).
//! - `nfsm_mbuftouio` checks the uio's residual count where the C checks for iovecs left
//!   (`EFBIG`), and needs a read uio (`UIO_READ`) whose `uio_resid` is the sum of its iovecs:
//!   the glue's `uiomove_out` panics otherwise, where the C needs neither. Every C caller passes
//!   such a uio (C:nfs/nfs_vnops.c:1176, 1248, 2460, 2656; nfs_serv.c:1941) and the wire cannot
//!   change either. Its ignored `copyout` error is kept: the uio moves on past the bytes.
//!
//! ## Redesign
//! - LZ's `bsd` module is the starting point of the client's half (D-5, N6.7a0): the words,
//!   the tables, the statistics and `txdr_nfsv2time` are the kernel's, `bsd`'s `nfs_subs`
//!   re-exports them for the server; N6.7a1 brings the RPC header (`nfsm_rpchead`), the xids
//!   and the procedure maps; N6.7a2 the attribute cache, the commit ranges and
//!   `nfs_vfs_init` (`nfs_loadattrcache` comes with the vnode operations, N6.7a3).
//! - LZ's attribute cache and commit ranges took the vnode and found its node and mount again
//!   (`VTONFS`, `VFSTONFS`: an `unsafe` cast each). Here the vnode operation's node and mount
//!   are lent ([`nfs_getattrcache`] and [`nfs_attrtimeo`] take `&NfsNode` and `&NfsMount`,
//!   the range functions `&NfsNode`): `nfs_getattrcache` is per I/O, and the hot path's rule
//!   allows no lookup the C does not make. The cache is read once into the caller's copy (the
//!   C's `bcopy`), a reader pair of its cell where the C reads in place, and written back
//!   only when the C writes it (a modified file's larger size). The six range functions are
//!   one pair of words and its valid bit each (`Range`), the C's two copies of each body
//!   written once. `nfs_clearcommit` walks the mount's vnodes (`Mount::vnodes`) and clears
//!   each dirty list's `B_NEEDCOMMIT` in one section of the cache's lock
//!   (`vnode_dirty_for_each`), where the C holds `splbio` once over the whole walk: one
//!   section per vnode, on a verifier change only.
//! - LZ's `nfsm_rpchead` wrote the header into the request's first mbuf through
//!   `nfsm_build` over a raw cursor of the request (`r_mreq`, `r_nmp`, `r_xid` read and set
//!   through it); [`nfsm_rpchead`] takes the mount, the procedure and the arguments' chain
//!   and answers the request's chain and its xid, from which `nfs_request` makes the request:
//!   the header is an [`XdrWriter`] over the packet header mbuf `m_align` sized (the C's
//!   layout: it fits, so the writer never grows), the arguments linked behind it.
//! - LZ's build functions over a `&mut &'static Mbuf` cursor (`nfsm_build`, `nfsm_uiotombuf`
//!   with its two raw writes into the trailing space, `nfsm_buftombuf` and `nfsm_strtombuf`
//!   over `UIO_SYSSPACE` uios of raw iovecs) are methods of the writer that owns the chain:
//!   one body, `append`, takes the bytes from the source in pieces, so a buffer or a string is
//!   copied straight from its slice (no uio built over it) with the C's mbuf layout. LZ's
//!   `nfsm_mbuftouio` (a raw copy into a `UIO_SYSSPACE` iovec, `copyout` otherwise) is the
//!   reader's, each piece moved by the glue's `uiomove_out`; `nfsm_disct`, `nfs_adv` and
//!   `nfsm_nextbytes` are the reader's copies and walks (`nfsm_subs`): no mbuf is inserted
//!   into a reply, no raw position kept.
//! - N6.7a3: [`nfs_loadattrcache`] takes the node and its mount's version from the operation
//!   (the hot path's rule: no `VTONFS` or `VFSTONFS` the C's cast does not cost), reads the
//!   `nfs_fattr` by the reader's `pod_len` (a copy of 68 or 84 bytes into the frame), and
//!   writes the cache in one section of its cell (`ReadMostly::update`), its copy for the
//!   caller made from the same section; the alias of a device moves the node's handle to the
//!   alias vnode's slot and its `n_vnode` link, and answers the vnode (the C's `*vpp`).

#![allow(non_upper_case_globals)] // the C's names of the XDR words

use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64, Ordering};

use hal::intr::IPL_NONE;
use hal::pool::PR_WAITOK;

use hal::mutex::Mutexed;

use super::nfs::{NFS_TICKINTVL, NfsCounters};
use super::nfs_bio::DEV_BSIZE;
use super::nfs_node::nfs_nhashinit;
use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_padlen};
use super::nfsmount::NfsMount;
use super::nfsnode::{
    NACC, NCHG, NFS_COMMIT_PUSH_VALID, NFS_COMMIT_PUSHED_VALID, NMODIFIED, NUPD, NfsNode,
};
use super::nfsproto::{
    NFBLK, NFCHR, NFDIR, NFFIFO, NFLNK, NFNON, NFREG, NFS_MAXFHSIZE, NFS_PROG, NFSOCK,
    NFSV3SATTRTIME_DONTCHANGE, NFSV3SATTRTIME_TOCLIENT, NFSV3SATTRTIME_TOSERVER, NFSX_V2FH, Nfsfh,
    Nfstype, Nfsv2Time,
};
use super::nfsproto::{NFS_FABLKSIZE, NfsFattr, nfsv2tov_type, nfsv3tov_type, nfsx_fattr};
use super::nfsproto::{
    NFS_NPROCS, NFS_VER2, NFS_VER3, NFSPROC_CREATE, NFSPROC_FSSTAT, NFSPROC_GETATTR, NFSPROC_LINK,
    NFSPROC_LOOKUP, NFSPROC_MKDIR, NFSPROC_NOOP, NFSPROC_NULL, NFSPROC_READ, NFSPROC_READDIR,
    NFSPROC_READLINK, NFSPROC_REMOVE, NFSPROC_RENAME, NFSPROC_RMDIR, NFSPROC_SETATTR,
    NFSPROC_SYMLINK, NFSPROC_WRITE, NFSV2PROC_CREATE, NFSV2PROC_GETATTR, NFSV2PROC_LINK,
    NFSV2PROC_LOOKUP, NFSV2PROC_MKDIR, NFSV2PROC_NOOP, NFSV2PROC_NULL, NFSV2PROC_READ,
    NFSV2PROC_READDIR, NFSV2PROC_READLINK, NFSV2PROC_REMOVE, NFSV2PROC_RENAME, NFSV2PROC_RMDIR,
    NFSV2PROC_SETATTR, NFSV2PROC_STATFS, NFSV2PROC_SYMLINK, NFSV2PROC_WRITE, NFSX_UNSIGNED,
};
use super::xdr_subs::{
    fxdr_hyper, fxdr_nfsv2time, fxdr_nfsv3time, fxdr_unsigned, txdr_nfsv3time, txdr_unsigned,
};
use crate::assert::kassert;
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::cred::Ucred;
use crate::crypto::idgen::Idgen32Ctx;
use crate::file::FileGlue;
use crate::fs::bio::vnode_dirty_for_each;
use crate::fs::buf::{B_BUSY, B_DELWRI, B_NEEDCOMMIT, Buf};
use crate::fs::cache::cache_purge;
use crate::fs::mount::{Mount, Vfsconf};
use crate::fs::spec::BLKDEV_IOSIZE;
use crate::fs::subr::{checkalias, vfs, vgone, vops_id, vrele};
use crate::fs::vnode::Vtype::{self, VBLK, VCHR, VDIR, VFIFO, VLNK, VNON, VREG, VSOCK};
use crate::fs::vnode::{VNOVAL, Vattr, Vnode, VopsId, iftovt};
use crate::net::mbuf::{
    M_WAIT, MLEN, MT_DATA, MbufBox, m_align, m_calchdrlen, m_get, m_gethdr, mclget,
};
use crate::proc::ProcGlue;
use crate::sys::errno::Errno;
use crate::sys::mount::NFSMNT_NFSV3;
use crate::sys::nfs_rpcv2::{
    RPC_AUTHERR, RPC_CALL, RPC_MISMATCH, RPC_MSGACCEPTED, RPC_MSGDENIED, RPC_REPLY, RPC_VER2,
    RPCAUTH_NULL, RPCAUTH_UNIX,
};
use crate::sys::types::{Dev, Gid, Mode, Off, Uid, makedev};
use crate::tc::gettime;
use crate::uio::{Uio, UioRw};
use crate::vm::io::MAXBSIZE;
use crate::vm::vnode::uvm_vnp_setsize;

/// `nfs_xdrneg1`: the XDR word of -1. Data items converted to xdr at startup, since they are
/// constant. This is kinda hokey, but may save a little time doing byte swaps.
pub const nfs_xdrneg1: u32 = txdr_unsigned(-1i32 as u32);
/// `rpc_call`.
pub const rpc_call: u32 = txdr_unsigned(RPC_CALL);
/// `rpc_vers`.
pub const rpc_vers: u32 = txdr_unsigned(RPC_VER2);
/// `rpc_reply`.
pub const rpc_reply: u32 = txdr_unsigned(RPC_REPLY);
/// `rpc_msgdenied`.
pub const rpc_msgdenied: u32 = txdr_unsigned(RPC_MSGDENIED);
/// `rpc_autherr`.
pub const rpc_autherr: u32 = txdr_unsigned(RPC_AUTHERR);
/// `rpc_mismatch`.
pub const rpc_mismatch: u32 = txdr_unsigned(RPC_MISMATCH);
/// `rpc_auth_unix`.
pub const rpc_auth_unix: u32 = txdr_unsigned(RPCAUTH_UNIX);
/// `rpc_msgaccepted`.
pub const rpc_msgaccepted: u32 = txdr_unsigned(RPC_MSGACCEPTED);
/// `nfs_prog`.
pub const nfs_prog: u32 = txdr_unsigned(NFS_PROG);
/// `nfs_true`.
pub const nfs_true: u32 = txdr_unsigned(1);
/// `nfs_false`.
pub const nfs_false: u32 = txdr_unsigned(0);

/// `nfsv2_type`: the version 2 file type of each vnode type.
pub const NFSV2_TYPE: [Nfstype; 9] = [
    NFNON, NFREG, NFDIR, NFBLK, NFCHR, NFLNK, NFNON, NFCHR, NFNON,
];
/// `nfsv3_type`: the version 3 file type of each vnode type.
pub const NFSV3_TYPE: [Nfstype; 9] = [
    NFNON, NFREG, NFDIR, NFBLK, NFCHR, NFLNK, NFSOCK, NFFIFO, NFNON,
];
/// `nv2tov_type`: the vnode type of each version 2 file type.
pub const NV2TOV_TYPE: [Vtype; 8] = [VNON, VREG, VDIR, VBLK, VCHR, VLNK, VNON, VNON];
/// `nv3tov_type`: the vnode type of each version 3 file type.
pub const NV3TOV_TYPE: [Vtype; 8] = [VNON, VREG, VDIR, VBLK, VCHR, VLNK, VSOCK, VFIFO];

/// `nfsv3_procid`: mapping of old NFS Version 2 RPC numbers to generic numbers.
pub const NFSV3_PROCID: [usize; NFS_NPROCS] = [
    NFSPROC_NULL,
    NFSPROC_GETATTR,
    NFSPROC_SETATTR,
    NFSPROC_NOOP,
    NFSPROC_LOOKUP,
    NFSPROC_READLINK,
    NFSPROC_READ,
    NFSPROC_NOOP,
    NFSPROC_WRITE,
    NFSPROC_CREATE,
    NFSPROC_REMOVE,
    NFSPROC_RENAME,
    NFSPROC_LINK,
    NFSPROC_SYMLINK,
    NFSPROC_MKDIR,
    NFSPROC_RMDIR,
    NFSPROC_READDIR,
    NFSPROC_FSSTAT,
    NFSPROC_NOOP,
    NFSPROC_NOOP,
    NFSPROC_NOOP,
    NFSPROC_NOOP,
    NFSPROC_NOOP,
];

/// `nfsv2_procid`: and the reverse mapping from generic to Version 2 procedure numbers.
pub const NFSV2_PROCID: [usize; NFS_NPROCS] = [
    NFSV2PROC_NULL,
    NFSV2PROC_GETATTR,
    NFSV2PROC_SETATTR,
    NFSV2PROC_LOOKUP,
    NFSV2PROC_NOOP,
    NFSV2PROC_READLINK,
    NFSV2PROC_READ,
    NFSV2PROC_WRITE,
    NFSV2PROC_CREATE,
    NFSV2PROC_MKDIR,
    NFSV2PROC_SYMLINK,
    NFSV2PROC_CREATE,
    NFSV2PROC_REMOVE,
    NFSV2PROC_RMDIR,
    NFSV2PROC_RENAME,
    NFSV2PROC_LINK,
    NFSV2PROC_READDIR,
    NFSV2PROC_NOOP,
    NFSV2PROC_STATFS,
    NFSV2PROC_NOOP,
    NFSV2PROC_NOOP,
    NFSV2PROC_NOOP,
    NFSV2PROC_NOOP,
];

/// `nfsstats`: the client's and the server's counters (`vfs.nfs.nfsstats`).
pub static NFSSTATS: NfsCounters = NfsCounters::new();

/// `nfs_ticks`: the NFS timer's period in clock ticks (`nfs_init`'s, at least 1).
pub static NFS_TICKS: AtomicI32 = AtomicI32::new(0);

/// `nfs_get_xid`'s `nfs_xid_ctx` and `called`: the context, `None` before the first call.
static NFS_XID_CTX: Mutexed<Option<Idgen32Ctx>> = Mutexed::new(IPL_NONE, None);

/// `txdr_nfsv2time(from, to)`: the version 2 time of `from`; `VNOVAL` nanoseconds are sent
/// as -1, a time of -1 seconds as -1.000001.
pub fn txdr_nfsv2time(from: &Timespec) -> Nfsv2Time {
    if from.tv_nsec == i64::from(VNOVAL) {
        Nfsv2Time {
            nfsv2_sec: nfs_xdrneg1,
            nfsv2_usec: nfs_xdrneg1,
        }
    } else if from.tv_sec == -1 {
        // can't request a time of -1; send -1.000001 == {-2,999999} instead
        Nfsv2Time {
            nfsv2_sec: txdr_unsigned(-2i32 as u32),
            nfsv2_usec: txdr_unsigned(999_999),
        }
    } else {
        Nfsv2Time {
            nfsv2_sec: txdr_unsigned(from.tv_sec as u32),
            nfsv2_usec: txdr_unsigned((from.tv_nsec / 1000) as u32),
        }
    }
}

/// `nfs_get_xid`: return an unpredictable XID in XDR form (the context seeded at the first
/// call).
pub fn nfs_get_xid() -> u32 {
    let mut g = NFS_XID_CTX.lock();
    let ctx = g.get_or_insert_with(|| {
        let mut c = Idgen32Ctx::zeroed();
        c.init();
        c
    });
    txdr_unsigned(ctx.generate())
}

/// `nfsm_rpchead(req, cr, auth_type)`: build the RPC header and fill in the authorization
/// info. Right now we are pretty centric around `RPCAUTH_UNIX`, in the future, this function
/// will need some love to be able to handle other authorization methods, such as Kerberos.
///
/// The header goes in a fresh packet header mbuf (`rep->r_mreq`), `body` (the procedure's
/// arguments, `nmi_mreq`) linked behind it; the answer is that chain, its packet length the
/// whole request's, and the request's new xid (`r_xid`). The credential's groups are clamped
/// to the mount's `nm_numgrps`. `None` when no mbuf is to be had for the header (`hal`'s
/// pools can fail a waiting `MGETHDR`; `nfs_request` answers `ENOBUFS`, as LZ), `body` freed.
pub fn nfsm_rpchead<G: ProcGlue>(
    nmp: &NfsMount<G>,
    procnum: usize,
    cr: &Ucred,
    auth_type: u32,
    body: MbufBox,
) -> Option<(MbufBox, u32)> {
    kassert!(auth_type == RPCAUTH_UNIX);

    // RPCAUTH_UNIX fits in an hdr mbuf, in the future other authorization methods need to
    // figure out their own sizes and allocate and chain mbufs accordingly.
    let mb = m_gethdr(M_WAIT, MT_DATA)?;

    // We need to start out by finding how big the authorization cred and verifier are for
    // the auth_type, to be able to correctly align the mbuf header/chain. In the
    // RPCAUTH_UNIX case, the size is the static part as shown in RFC1831 + the number of
    // groups, RPCAUTH_UNIX has a zero verifier.
    let crgroups = cr.groups();
    let numgrps = nmp.nm_numgrps.load(Ordering::Relaxed);
    let ngroups = if i32::from(cr.cr_ngroups.get()) > numgrps {
        numgrps
    } else {
        i32::from(cr.cr_ngroups.get())
    };
    let ngroups = usize::try_from(ngroups).unwrap_or(0).min(crgroups.len());
    let auth_len = (ngroups << 2) + 5 * NFSX_UNSIGNED;
    let authsiz = super::nfsm_subs::nfsm_rndup(auth_len);
    // The authorization size + the size of the static part.
    m_align(&mb, (authsiz + 10 * NFSX_UNSIGNED) as i32);
    mb.m_len().set(0);
    let mut w = XdrWriter::with_first(mb);

    // First the RPC header. Get a new (non-zero) xid.
    let xid = nfs_get_xid();
    w.word(xid);
    w.word(rpc_call);
    w.word(rpc_vers);
    w.word(nfs_prog);
    if nmp.nm_flag.load(Ordering::Relaxed) & NFSMNT_NFSV3 != 0 {
        w.u32(NFS_VER3);
        w.u32(procnum as u32);
    } else {
        w.u32(NFS_VER2);
        w.u32(NFSV2_PROCID[procnum] as u32);
    }

    // The Authorization cred and its verifier.
    w.u32(RPCAUTH_UNIX);
    w.u32(authsiz as u32);
    // The authorization cred.
    w.word(0); // stamp
    w.word(0); // NULL hostname
    w.u32(cr.cr_uid.get());
    w.u32(cr.cr_gid.get());
    w.u32(ngroups as u32);
    for g in &crgroups[..ngroups] {
        w.u32(g.get());
    }
    // The authorization verifier.
    w.u32(RPCAUTH_NULL);
    w.word(0);

    let mut m = w.into_chain();
    drop(m.m_next.set(body));
    m_calchdrlen(&m);
    m.m_pkthdr().ph_ifidx.set(0);
    Some((m, xid))
}

impl XdrWriter {
    /// `nfsm_reqhead(hsiz)`: a request's first mbuf, empty, with a cluster when the rest of
    /// the header (`hsiz` bytes) would not fit a plain one.
    pub fn reqhead(hsiz: usize) -> Self {
        let Some(mut mb) = m_get(M_WAIT, MT_DATA) else {
            kpanic(format_args!("nfs: out of mbufs"));
        };
        if hsiz > MLEN {
            mclget(&mut mb, M_WAIT);
        }
        mb.m_len().set(0);
        Self::with_first(mb)
    }

    /// The body `nfsm_uiotombuf` runs for every source: `len` bytes appended across mbufs
    /// (a fresh one, with a cluster while more than `MLEN` bytes are left, each time the tail
    /// is full), `fill` writing each piece in order, then the XDR padding, zeroed (in a plain
    /// fresh mbuf when the tail has no room for it).
    fn append(&mut self, mut len: usize, mut fill: impl FnMut(&mut [u8])) {
        let pad = nfsm_padlen(len);
        while len > 0 {
            let xfer = len.min(self.trailing());
            fill(self.take(xfer));
            len -= xfer;
            if len > 0 {
                self.grow(len > MLEN);
            }
        }
        if pad > 0 {
            if pad > self.trailing() {
                self.grow(false);
            }
            self.take(pad).fill(0);
        }
    }

    /// `nfsm_uiotombuf(mp, uiop, len)`: `len` bytes of `uio` appended, then their padding. As
    /// the C, `uio_resid` is set to `len` and the uio made a write first, and `uiomove`'s
    /// error is not returned; the bytes a failed move did not fill are zero.
    pub fn from_uio<G: ProcGlue>(&mut self, uio: &mut Uio<'_, G>, len: usize) {
        // XXX -- the following should be done by the caller
        uio.uio_resid = len;
        uio.uio_rw = UioRw::UIO_WRITE;
        self.append(len, |dst| {
            let before = uio.uio_resid;
            if <G::FileGlue as FileGlue>::uiomove(dst, uio).is_err() {
                let moved = before - uio.uio_resid;
                dst[moved..].fill(0);
            }
        });
    }

    /// `nfsm_buftombuf(mp, buf, len)`: the bytes of `buf` appended, then their padding.
    pub fn buftombuf(&mut self, buf: &[u8]) {
        let mut at = 0;
        self.append(buf.len(), |dst| {
            dst.copy_from_slice(&buf[at..at + dst.len()]);
            at += dst.len();
        });
    }

    /// `nfsm_strtombuf(mp, str, len)`: the XDR string `str`: its length word, its bytes, the
    /// padding, appended as one stream (the length word may straddle two mbufs, as the C's
    /// two-iovec uio lets it).
    pub fn strtombuf(&mut self, str: &[u8]) {
        let Ok(n) = u32::try_from(str.len()) else {
            kpanic(format_args!("nfsm_strtombuf: {} bytes", str.len()));
        };
        let head = n.to_be_bytes();
        let mut at = 0;
        self.append(head.len() + str.len(), |dst| {
            for b in dst.iter_mut() {
                *b = if at < head.len() {
                    head[at]
                } else {
                    str[at - head.len()]
                };
                at += 1;
            }
        });
    }

    /// `nfsm_fhtom(mb, v, v3)`: a node's file handle (`n_fhp`, `n_fhsize` bytes of it): a
    /// counted string in version 3, the fixed `NFSX_V2FH` bytes in version 2.
    pub fn fhtom(&mut self, fh: &Nfsfh, fhsize: usize, v3: bool) {
        if v3 {
            self.strtombuf(&fh.fh_bytes[..fhsize.min(NFS_MAXFHSIZE)]);
        } else {
            self.buftombuf(&fh.fh_bytes[..NFSX_V2FH]);
        }
    }

    /// `nfsm_v3attrbuild(mp, a, full)`: a version 3 `sattr3` of `a`: the mode and the times
    /// always, the owner, group and size only when `full`; a member left `VNOVAL` is "do not
    /// set", a time equal to the server's second is "set to the server's time". The bytes on the
    /// wire are the C's; the mbuf split may differ, since a flag word and its value are two
    /// 4-byte builds here where the C builds one 8- or 12-byte block.
    pub fn v3attrbuild(&mut self, a: &Vattr, full: bool) {
        let set = |w: &mut Self, on: bool, v: u32| {
            if on {
                w.word(nfs_true);
                w.u32(v);
            } else {
                w.word(nfs_false);
            }
        };
        set(self, a.va_mode != VNOVAL as Mode, a.va_mode);
        set(self, full && a.va_uid != VNOVAL as Uid, a.va_uid);
        set(self, full && a.va_gid != VNOVAL as Gid, a.va_gid);
        if full && a.va_size != VNOVAL as u64 {
            self.word(nfs_true);
            self.hyper(a.va_size);
        } else {
            self.word(nfs_false);
        }
        for t in [&a.va_atime, &a.va_mtime] {
            if t.tv_nsec == i64::from(VNOVAL) {
                self.u32(NFSV3SATTRTIME_DONTCHANGE);
            } else {
                if t.tv_sec == gettime() {
                    self.u32(NFSV3SATTRTIME_TOSERVER);
                } else {
                    self.u32(NFSV3SATTRTIME_TOCLIENT);
                    self.pod(&txdr_nfsv3time(t));
                }
            }
        }
    }
}

impl XdrReader<'_> {
    /// `nfsm_mbuftouio(mrep, uiop, siz, dpos)`: `siz` bytes of the reply into `uio`, mbuf by
    /// mbuf, then their XDR padding skipped. `EFBIG` when the uio is full first, `EBADRPC`
    /// when the chain ends first. A failed copy is the C's ignored `copyout` error: the uio
    /// moves past the bytes all the same.
    pub(super) fn mbuftouio<G: ProcGlue>(
        &mut self,
        uio: &mut Uio<'_, G>,
        siz: usize,
    ) -> Result<(), Errno> {
        let mut left = siz;
        while left > 0 {
            if uio.uio_resid == 0 || uio.uio_iov.is_empty() {
                return Err(Errno::EFBIG);
            }
            let piece = self.next_piece(left.min(uio.uio_resid))?;
            let before = uio.uio_resid;
            if <G::FileGlue as FileGlue>::uiomove_out(piece, uio).is_err() {
                uio_skip(uio, piece.len() - (before - uio.uio_resid));
            }
            left -= piece.len();
        }
        self.adv(nfsm_padlen(siz))
    }
}

/// The C's `uiop->uio_iov++`/`iov_base += n` past `n` bytes a copy did not move (its
/// ignored `copyout` error): the iovecs, `uio_resid` and `uio_offset` advanced, nothing copied.
fn uio_skip<G: ProcGlue>(uio: &mut Uio<'_, G>, mut n: usize) {
    while n > 0 {
        let Some(iov) = uio.uio_iov.first_mut() else {
            return;
        };
        if iov.iov_len == 0 {
            let iovs = core::mem::take(&mut uio.uio_iov);
            uio.uio_iov = &mut iovs[1..];
            continue;
        }
        let c = iov.iov_len.min(n);
        iov.iov_base = iov.iov_base.wrapping_byte_add(c);
        iov.iov_len -= c;
        uio.uio_resid -= c;
        uio.uio_offset = uio
            .uio_offset
            .saturating_add(Off::try_from(c).unwrap_or(Off::MAX));
        n -= c;
    }
}

/// `nfs_init`: called once to initialize data structures: the timer's period
/// (`(hz * NFS_TICKINTVL + 500) / 1000`, at least one tick), with `option NFSSERVER` the
/// server's tables (`nfsrv_init(0)`, which makes "ndscpl", then `nfsrv_initcache`, which
/// makes "nfsrvcpl") and the server sockets' upcall function registered with the glue's
/// socket layer (`nfsrv_rcv`, which `nfssvc_addsock` sets on each socket), then "nfsreqpl",
/// last, as the C's order (`vmstat -m`'s rows and the `kern.pool` serials). The XDR words
/// are constants here. A second call (a test glue's, which forgets the server's tables
/// first, `nfsrv_test_reset`) makes the server's tables again and keeps the pools.
pub fn nfs_init<G: ProcGlue>() {
    let ticks = (crate::timeout::hz() * NFS_TICKINTVL + 500) / 1000;
    NFS_TICKS.store(ticks.max(1), Ordering::Relaxed);
    #[cfg(feature = "nfsserver")]
    {
        super::nfssvc::nfsrv_init::<G>(false); // Init server data structures
        super::nfs_srvcache::nfsrv_initcache::<G>(); // Init the server request cache
        crate::net::socket::register_upcall::<G>(super::nfs_srvsocket::nfsrv_rcv_upcall::<G>);
    }
    let _ = vfs::<G>()
        .nfs
        .nfsreqpl
        .init_recorded(IPL_NONE, PR_WAITOK, "nfsreqpl");
}

/// `nfs_vfs_init` (`vfs_init`): the nfsiod buffer queue (`TAILQ_INIT(&nfs_bufq)`: an empty
/// `NfsTables::nfs_bufq`, made with the tables), "nfsnodepl" (recorded: `vtonfs` counts a
/// node again from the vnode's slot), the key of the node hash that stands for every
/// mount's `nm_ntree` (`nfs_node`; its chains are the tables'), and the kq watch list's pool
/// (`nfs_kq`). A second call (a test glue's) keeps the pools and the key.
pub fn nfs_vfs_init<G: ProcGlue>(_vfsp: &'static Vfsconf<G>) -> Result<(), Errno> {
    let t = &vfs::<G>().nfs;
    let _ = t
        .nfs_node_pool
        .init_recorded(IPL_NONE, PR_WAITOK, "nfsnodepl");
    let _ = t.kevqpl.init(IPL_NONE, PR_WAITOK, "nfskevqpl");
    nfs_nhashinit::<G>();
    Ok(())
}

/// `nfs_loadattrcache(vpp, mdp, dposp, vaper)`: load the attribute cache (that lives in the
/// nfsnode entry) with the values of the `nfs_fattr` at the reader `x` (`NFSX_FATTR(v3)`
/// bytes, `EBADRPC` past the reply's end) and, iff `vaper` is given, copy the attributes to
/// it. `np` is the node, `v3` its mount's version (the C's `NFS_ISV3(vp)`, lent by the
/// operation). A new node (its vnode's type `VNON`) gets its type, and a device its
/// operations (`nfs_specvops`) and, when `checkalias` finds one, the alias's vnode, which the
/// node moves to: the vnode the node is on is returned (the C's `*vpp`). A fifo is
/// `EOPNOTSUPP` (`miscfs/fifofs` is not ported: the C's `!FIFO` path).
pub fn nfs_loadattrcache<G: ProcGlue>(
    np: &NfsNode<G>,
    v3: bool,
    x: &mut XdrReader<'_>,
    vaper: Option<&mut Vattr>,
) -> Result<&'static Vnode<G>, Errno> {
    let mut vp = super::nfsnode::nfstov(np);
    let fp = x.pod_len::<NfsFattr>(nfsx_fattr(v3))?;
    let vmode: Mode = fxdr_unsigned(fp.fa_mode);
    let (mut vtyp, rdev, mtime) = if v3 {
        let spec = fp.fa3_rdev();
        (
            nfsv3tov_type(fp.fa_type),
            makedev(fxdr_unsigned(spec.specdata1), fxdr_unsigned(spec.specdata2)),
            fxdr_nfsv3time(&fp.fa3_mtime()),
        )
    } else {
        let mut vtyp = nfsv2tov_type(fp.fa_type);
        if vtyp == VNON || vtyp == VREG {
            vtyp = iftovt(vmode);
        }
        // `int32_t rdev = fxdr_unsigned(int32_t, fp->fa2_rdev)`.
        (
            vtyp,
            fxdr_unsigned(fp.fa2_rdev()) as Dev,
            fxdr_nfsv2time(&fp.fa2_mtime()),
        )
    };
    // Really ugly NFSv2 kludge (`rdev == 0xffffffff`, the `int` made unsigned).
    if !v3 && vtyp == VCHR && rdev as u32 == 0xffff_ffff {
        vtyp = VFIFO;
    }

    // If v_type == VNON it is a new node, so fill in the v_type, n_mtime fields. Check to see
    // if it represents a special device, and if so, check for a possible alias. Once the
    // correct vnode has been obtained, fill in the rest of the information.
    if vp.vtype() == VNON {
        cache_purge(vp);
        vp.set_vtype(vtyp);
        if vtyp == VFIFO {
            // FIFO: vp->v_op = &nfs_fifovops (miscfs/fifofs, not ported); without FIFO:
            return Err(Errno::EOPNOTSUPP);
        }
        if vtyp == VCHR || vtyp == VBLK {
            vp.set_ops_id(vops_id::<G>(&vfs::<G>().nfs.specvops));
            if let Some(nvp) = checkalias(vp, rdev, vp.mount()) {
                // Discard unneeded vnode, but save its nfsnode. Since the nfsnode does not
                // have a lock, its vnode lock has to be carried over.
                if let Some(node) = vp.take_node() {
                    nvp.set_node(node);
                }
                vp.set_ops_id(VopsId::SPEC);
                vrele(vp);
                vgone(vp);
                // Reinitialize aliased node.
                np.n_vnode.set(Some(nvp));
                vp = nvp;
            }
        }
        let _ = np.n_mtime.replace(mtime);
    }

    let uid: Uid = fxdr_unsigned(fp.fa_uid);
    let gid: Gid = fxdr_unsigned(fp.fa_gid);
    let (fsid, iosize) = match vp.mount() {
        Some(mp) => (mp.fsid().val[0], mp.iosize()),
        None => kpanic(format_args!("nfs_loadattrcache: vnode {vp:p} has no mount")),
    };
    let n_size = np.n_size.load(Ordering::Relaxed);
    let modified = np.isset(NMODIFIED);
    let (va, changed, size) = np.n_vattr.update(|vap| {
        vap.va_type = vtyp;
        vap.va_rdev = rdev;
        vap.va_mtime = mtime;
        vap.va_fsid = i64::from(fsid);

        // Invalidate access cache if uid, gid or mode changed.
        if np.n_accstamp.load(Ordering::Relaxed) != -1
            && (gid != vap.va_gid || uid != vap.va_uid || (vmode & 0o7777) != vap.va_mode)
        {
            np.n_accstamp.store(-1, Ordering::Relaxed);
        }

        vap.va_mode = vmode & 0o7777;

        vap.va_blocksize = match vtyp {
            VBLK => i64::from(BLKDEV_IOSIZE),
            VCHR => MAXBSIZE as i64,
            _ if v3 => i64::from(iosize),
            // `fxdr_unsigned(int32_t, fp->fa2_blocksize)`.
            _ => i64::from(fxdr_unsigned(fp.fa2_blocksize()) as i32),
        };
        vap.va_nlink = fxdr_unsigned(fp.fa_nlink);
        vap.va_uid = uid;
        vap.va_gid = gid;
        if v3 {
            vap.va_size = fxdr_hyper(fp.fa3_size().words());
            vap.va_bytes = fxdr_hyper(fp.fa3_used().words());
            vap.va_fileid = fxdr_hyper(fp.fa3_fileid().words());
            vap.va_atime = fxdr_nfsv3time(&fp.fa3_atime());
            vap.va_ctime = fxdr_nfsv3time(&fp.fa3_ctime());
        } else {
            vap.va_size = u64::from(fxdr_unsigned(fp.fa2_size()));
            // `(u_quad_t)fxdr_unsigned(int32_t, fp->fa2_blocks) * NFS_FABLKSIZE`: the
            // signed word widened, then multiplied in 64 bits.
            vap.va_bytes = (i64::from(fxdr_unsigned(fp.fa2_blocks()) as i32) as u64)
                .wrapping_mul(NFS_FABLKSIZE);
            // `fxdr_unsigned(int32_t, fp->fa2_fileid)`, widened as a signed value.
            vap.va_fileid = i64::from(fxdr_unsigned(fp.fa2_fileid()) as i32) as u64;
            vap.va_atime = fxdr_nfsv2time(&fp.fa2_atime());
            let ctime = fp.fa2_ctime();
            vap.va_ctime = Timespec {
                tv_sec: i64::from(fxdr_unsigned(ctime.nfsv2_sec)),
                tv_nsec: 0,
            };
            vap.va_gen = u64::from(fxdr_unsigned(ctime.nfsv2_usec));
        }
        vap.va_flags = 0;
        vap.va_filerev = 0;

        // The size: the node's own while it has local modifications past the server's.
        let changed = vap.va_size != n_size;
        let mut size = None;
        if changed {
            if vap.va_type == VREG && modified && vap.va_size < n_size {
                vap.va_size = n_size;
            } else {
                size = Some(vap.va_size);
            }
        }
        (*vap, changed, size)
    });
    if changed {
        if let Some(size) = size {
            np.n_size.store(size, Ordering::Relaxed);
        }
        if va.va_type == VREG {
            // The C passes its `u_quad_t` size as the `voff_t` argument.
            uvm_vnp_setsize(vp, np.n_size.load(Ordering::Relaxed) as Off);
        }
    }
    np.n_attrstamp.store(gettime(), Ordering::Relaxed);
    if let Some(vaper) = vaper {
        *vaper = va;
        if np.isset(NCHG) {
            if np.isset(NACC) {
                vaper.va_atime = *np.n_atim.read();
            }
            if np.isset(NUPD) {
                vaper.va_mtime = *np.n_mtim.read();
            }
        }
    }
    Ok(vp)
}

/// `nfs_attrtimeo(np)`: the attribute cache timeout of a node, in seconds: a tenth of the
/// age of its last modification, clamped to the mount's `acregmin..acregmax`
/// (`acdirmin..acdirmax` for a directory); the minimum while it has local modifications.
/// `nmp` is the node's mount, lent by the caller (the C's `VFSTONFS(vp->v_mount)`).
pub fn nfs_attrtimeo<G: ProcGlue>(np: &NfsNode<G>, nmp: &NfsMount<G>) -> i32 {
    let vp = super::nfsnode::nfstov(np);
    // `int tenthage = (gettime() - np->n_mtime.tv_sec) / 10`: a `time_t` quotient kept in an
    // `int`, as the C's conversion (the server's times are 32-bit seconds).
    let tenthage = (gettime().wrapping_sub(np.n_mtime.read().tv_sec) / 10) as i32;

    let (minto, maxto) = if vp.vtype() == VDIR {
        (
            i32::from(nmp.nm_acdirmin.load(Ordering::Relaxed)),
            i32::from(nmp.nm_acdirmax.load(Ordering::Relaxed)),
        )
    } else {
        (
            i32::from(nmp.nm_acregmin.load(Ordering::Relaxed)),
            i32::from(nmp.nm_acregmax.load(Ordering::Relaxed)),
        )
    };

    if np.isset(NMODIFIED) || tenthage < minto {
        minto
    } else if tenthage < maxto {
        tenthage
    } else {
        maxto
    }
}

/// `nfs_getattrcache(vp, vaper)`: checks the time stamp; if the cache is valid, copies its
/// contents to `vaper`, otherwise fails with `ENOENT`. `np` is the vnode's node and `nmp` its
/// mount, both lent by the vnode operation (the C's `VTONFS(vp)` and, in `nfs_attrtimeo`,
/// `VFSTONFS(vp->v_mount)`).
pub fn nfs_getattrcache<G: ProcGlue>(
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    vaper: &mut Vattr,
) -> Result<(), Errno> {
    let stamp = np.n_attrstamp.load(Ordering::Relaxed);
    if stamp == 0 || gettime().wrapping_sub(stamp) >= i64::from(nfs_attrtimeo(np, nmp)) {
        NFSSTATS.attrcache_misses.fetch_add(1, Ordering::Relaxed);
        return Err(Errno::ENOENT);
    }
    NFSSTATS.attrcache_hits.fetch_add(1, Ordering::Relaxed);
    // `vap = &np->n_vattr`: the cache read once, the copy the C's `bcopy` makes.
    *vaper = *np.n_vattr.read();
    let n_size = np.n_size.load(Ordering::Relaxed);
    if vaper.va_size != n_size {
        if vaper.va_type == VREG {
            if np.isset(NMODIFIED) && vaper.va_size < n_size {
                // `vap->va_size = np->n_size`: the cache, then the copy.
                np.n_vattr.update(|v| v.va_size = n_size);
                vaper.va_size = n_size;
            } else {
                np.n_size.store(vaper.va_size, Ordering::Relaxed);
            }
            // The C passes its `u_quad_t` size as the `voff_t` argument.
            uvm_vnp_setsize(
                super::nfsnode::nfstov(np),
                np.n_size.load(Ordering::Relaxed) as Off,
            );
        } else {
            np.n_size.store(vaper.va_size, Ordering::Relaxed);
        }
    }
    if np.isset(NCHG) {
        if np.isset(NACC) {
            vaper.va_atime = *np.n_atim.read();
        }
        if np.isset(NUPD) {
            vaper.va_mtime = *np.n_mtim.read();
        }
    }
    Ok(())
}

/// `nfs_clearcommit(mp)`: the write verifier has changed (probably due to a server reboot),
/// so all `B_NEEDCOMMIT` blocks will have to be written again. Since they are on the dirty
/// block list as `B_DELWRI`, all this takes is clearing the `B_NEEDCOMMIT` flag. Once done
/// the new write verifier can be set for the mount point. The C holds `splbio` over the
/// whole walk; here each vnode's dirty list is one section of the cache's lock (the C's
/// `splbio` is that lock's level).
pub fn nfs_clearcommit<G: ProcGlue>(mp: &'static Mount<G>) {
    'restart: loop {
        for vp in mp.vnodes() {
            if !vp.mount().is_some_and(|m| ptr::eq(m, mp)) {
                // Paranoia.
                continue 'restart;
            }
            vnode_dirty_for_each(&vp.bufs, |bp| {
                if bp.flags() & (B_BUSY | B_DELWRI | B_NEEDCOMMIT) == B_DELWRI | B_NEEDCOMMIT {
                    bp.clr(B_NEEDCOMMIT);
                }
            });
        }
        break;
    }
}

/// `nfs_merge_commit_ranges(vp)`: folds the to-be-committed range into the committed one.
pub fn nfs_merge_commit_ranges<G: ProcGlue>(np: &NfsNode<G>) {
    let flags = np.n_commitflags.load(Ordering::Relaxed);
    let (pushlo, pushhi) = (
        np.n_pushlo.load(Ordering::Relaxed),
        np.n_pushhi.load(Ordering::Relaxed),
    );
    if flags & NFS_COMMIT_PUSHED_VALID == 0 {
        np.n_pushedlo.store(pushlo, Ordering::Relaxed);
        np.n_pushedhi.store(pushhi, Ordering::Relaxed);
        np.n_commitflags
            .fetch_or(NFS_COMMIT_PUSHED_VALID, Ordering::Relaxed);
    } else {
        if pushlo < np.n_pushedlo.load(Ordering::Relaxed) {
            np.n_pushedlo.store(pushlo, Ordering::Relaxed);
        }
        if pushhi > np.n_pushedhi.load(Ordering::Relaxed) {
            np.n_pushedhi.store(pushhi, Ordering::Relaxed);
        }
    }

    np.n_pushlo.store(0, Ordering::Relaxed);
    np.n_pushhi.store(0, Ordering::Relaxed);
    np.n_commitflags
        .fetch_and(!NFS_COMMIT_PUSH_VALID, Ordering::Relaxed);
}

/// The byte range of a buffer's dirty data: `(off_t)bp->b_blkno * DEV_BSIZE` to that plus
/// `b_dirtyend`.
fn commit_range(bp: &Buf) -> (Off, Off) {
    let lo = bp.blkno().wrapping_mul(Off::from(DEV_BSIZE));
    (lo, lo.wrapping_add(Off::from(bp.dirtyend())))
}

/// One of a node's two ranges, its pair of words and its valid bit: the committed one
/// (`n_pushedlo`, `n_pushedhi`, `NFS_COMMIT_PUSHED_VALID`) or the one to be committed
/// (`n_pushlo`, `n_pushhi`, `NFS_COMMIT_PUSH_VALID`). The C writes each function twice, once
/// per range.
struct Range<'a> {
    lo: &'a AtomicI64,
    hi: &'a AtomicI64,
    valid: i32,
    flags: &'a AtomicI32,
}

impl<'a> Range<'a> {
    /// The committed range of `np`.
    fn pushed<G: ProcGlue>(np: &'a NfsNode<G>) -> Self {
        Self {
            lo: &np.n_pushedlo,
            hi: &np.n_pushedhi,
            valid: NFS_COMMIT_PUSHED_VALID,
            flags: &np.n_commitflags,
        }
    }

    /// The range of `np` to be committed.
    fn push<G: ProcGlue>(np: &'a NfsNode<G>) -> Self {
        Self {
            lo: &np.n_pushlo,
            hi: &np.n_pushhi,
            valid: NFS_COMMIT_PUSH_VALID,
            flags: &np.n_commitflags,
        }
    }

    /// Whether the range is valid.
    fn is_valid(&self) -> bool {
        self.flags.load(Ordering::Relaxed) & self.valid != 0
    }

    /// Whether the buffer's dirty bytes lie inside the valid range.
    fn holds(&self, bp: &Buf) -> bool {
        if !self.is_valid() {
            return false;
        }
        let (lo, hi) = commit_range(bp);
        lo >= self.lo.load(Ordering::Relaxed) && hi <= self.hi.load(Ordering::Relaxed)
    }

    /// The range widened to the buffer's dirty bytes (made the buffer's when not valid).
    fn add(&self, bp: &Buf) {
        let (lo, hi) = commit_range(bp);
        if self.is_valid() {
            if hi > self.hi.load(Ordering::Relaxed) {
                self.hi.store(hi, Ordering::Relaxed);
            }
            if lo < self.lo.load(Ordering::Relaxed) {
                self.lo.store(lo, Ordering::Relaxed);
            }
        } else {
            self.lo.store(lo, Ordering::Relaxed);
            self.hi.store(hi, Ordering::Relaxed);
            self.flags.fetch_or(self.valid, Ordering::Relaxed);
        }
    }

    /// The buffer's dirty bytes taken out of the valid range; with the bytes in its middle,
    /// one of the two pieces is kept (XXX There's only one range): by the C's comparison, the
    /// smaller one, although its comment says the largest.
    fn del(&self, bp: &Buf) {
        if !self.is_valid() {
            return;
        }
        let (lo, hi) = commit_range(bp);
        let (rlo, rhi) = (
            self.lo.load(Ordering::Relaxed),
            self.hi.load(Ordering::Relaxed),
        );
        if lo > rhi || hi < rlo {
            return;
        }
        if lo <= rlo {
            self.lo.store(hi, Ordering::Relaxed);
        } else if hi >= rhi {
            self.hi.store(lo, Ordering::Relaxed);
        } else {
            // XXX There's only one range. If the deleted range is in the middle, pick the
            // largest of the contiguous ranges that it leaves. The C compares `n_pushedlo - lo`
            // (minus the left piece) with `hi - n_pushedhi` (minus the right one), which keeps
            // the smaller piece: kept as written (the tests pin it).
            if rlo.wrapping_sub(lo) > hi.wrapping_sub(rhi) {
                self.hi.store(lo, Ordering::Relaxed);
            } else {
                self.lo.store(hi, Ordering::Relaxed);
            }
        }
    }
}

/// `nfs_in_committed_range(vp, bp)`: whether the buffer's dirty data was committed.
pub fn nfs_in_committed_range<G: ProcGlue>(np: &NfsNode<G>, bp: &Buf) -> bool {
    Range::pushed(np).holds(bp)
}

/// `nfs_in_tobecommitted_range(vp, bp)`: whether the buffer's dirty data is in the range
/// to be committed.
pub fn nfs_in_tobecommitted_range<G: ProcGlue>(np: &NfsNode<G>, bp: &Buf) -> bool {
    Range::push(np).holds(bp)
}

/// `nfs_add_committed_range(vp, bp)`: widens the committed range to the buffer.
pub fn nfs_add_committed_range<G: ProcGlue>(np: &NfsNode<G>, bp: &Buf) {
    Range::pushed(np).add(bp);
}

/// `nfs_del_committed_range(vp, bp)`: takes the buffer out of the committed range.
pub fn nfs_del_committed_range<G: ProcGlue>(np: &NfsNode<G>, bp: &Buf) {
    Range::pushed(np).del(bp);
}

/// `nfs_add_tobecommitted_range(vp, bp)`: widens the range to be committed to the buffer.
pub fn nfs_add_tobecommitted_range<G: ProcGlue>(np: &NfsNode<G>, bp: &Buf) {
    Range::push(np).add(bp);
}

/// `nfs_del_tobecommitted_range(vp, bp)`: takes the buffer out of the range to be
/// committed.
pub fn nfs_del_tobecommitted_range<G: ProcGlue>(np: &NfsNode<G>, bp: &Buf) {
    Range::push(np).del(bp);
}

/// The host tests' reset of the client's globals of a test glue (testing.md): the nfsiod
/// queue and the kq watch list forgotten with what they hold, the queue's counts and the
/// nfsiods' table cleared. Nothing is freed: the memory behind them is gone with the last
/// test's. The node hash's chains are left as they are: a node a test left linked is in that
/// test's leaked memory, on a mount no later test has, so no lookup matches it.
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn nfs_test_reset<G: ProcGlue>() {
    let t = &vfs::<G>().nfs;
    {
        // Every caller of `vfs_test_reset` holds the kernel lock (the test set-ups).
        let mut w = hal::klock::witness();
        core::mem::forget(core::mem::replace(
            t.nfs_bufq.get_mut(&mut w),
            crate::fs::bufq::Fifo::new(),
        ));
    }
    super::nfs_kq::nfs_kq_test_reset::<G>();
    for d in &t.nfs_asyncdaemon {
        d.store(0, Ordering::Relaxed);
    }
    super::nfsnode::NFS_BUFQLEN.store(0, Ordering::Relaxed);
    super::nfsnode::NFS_BUFQMAX.store(0, Ordering::Relaxed);
    super::nfsnode::NFS_NUMASYNC.store(0, Ordering::Relaxed);
}

const _: () = {
    assert!(super::xdr_subs::fxdr_unsigned(nfs_xdrneg1) == u32::MAX);
    assert!(NFSV3_TYPE.len() == 9 && NV3TOV_TYPE.len() == 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of nfs_subs.c's words, tables and times, and of the cursors' methods it
    // supplies: the uio copies both ways, the strings, the handles, the sattr3.

    use super::*;
    use crate::fs::nfs::xdr_subs::fxdr_unsigned;

    use std::vec;
    use std::vec::Vec;

    use crate::fs::nfs::nfsm_subs::{TestRng, nfsm_rndup, test_bytes, test_chain, test_lens};
    use crate::fs::vnode::Vattr;
    use crate::net::mbuf::tests::setup;
    use crate::uio::{Iovec, Uio, UioSeg};

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// `len` bytes of the reply at `r` read into a buffer of `room` bytes through a
    /// `UIO_SYSSPACE` uio (`nfsm_mtouio`): the result, the buffer and the residual count.
    fn read_into(
        r: &mut XdrReader<'_>,
        len: i32,
        room: usize,
    ) -> (Result<(), Errno>, Vec<u8>, usize) {
        let mut buf = vec![0xeeu8; room];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::<Procs>::kernel_read(&mut iov, &mut buf, None);
        let res = r.to_uio(&mut uio, len);
        let resid = uio.uio_resid;
        drop(uio);
        (res, buf, resid)
    }

    #[test]
    fn mtouio_copies_skips_the_padding_and_checks_both_ends() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        // A 5-byte opaque, its 3 padding bytes, then a word.
        let m = test_chain(&[&[1, 2], &[3, 4, 5, 0], &[0, 0, 0xaa, 0xbb, 0xcc, 0xdd]]);
        let mut r = XdrReader::new(&m);
        let (res, buf, resid) = read_into(&mut r, 5, 8);
        assert_eq!(res, Ok(()));
        assert_eq!(buf[..5], [1, 2, 3, 4, 5]);
        assert_eq!(resid, 3);
        assert_eq!(r.u32(), Ok(0xaabb_ccdd));
        // len <= 0 moves nothing.
        let mut r = XdrReader::new(&m);
        assert_eq!(read_into(&mut r, 0, 8).0, Ok(()));
        assert_eq!(read_into(&mut r, -7, 8).0, Ok(()));
        assert_eq!(r.offset(), 0);
        // A uio smaller than the data: EFBIG; a chain shorter than it: EBADRPC.
        let mut r = XdrReader::new(&m);
        assert_eq!(read_into(&mut r, 5, 4).0, Err(Errno::EFBIG));
        let mut r = XdrReader::new(&m);
        assert_eq!(read_into(&mut r, 13, 64).0, Err(Errno::EBADRPC));
        crate::testutil::set_copy_kernel(false);
    }

    #[test]
    fn an_ignored_copy_error_still_moves_the_uio_past_the_bytes() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        // A 13-byte opaque over three mbufs (6, 6, then 1 and its 3 padding bytes), a word
        // after it, read into two 8-byte iovecs. The legs: the first piece into a[0..6] (leg
        // 0); the second into a[6..8] (leg 1) and b[0..4] (leg 2, which faults); the third
        // into b[4] (leg 3).
        let data: Vec<u8> = (1..=13).collect();
        let m = test_chain(&[
            &data[..6],
            &data[6..12],
            &[13, 0, 0, 0, 0xaa, 0xbb, 0xcc, 0xdd],
        ]);
        let mut a = [0xeeu8; 8];
        let mut b = [0xeeu8; 8];
        let b_base = b.as_mut_ptr().cast::<core::ffi::c_void>();
        let mut iov = [
            Iovec {
                iov_base: a.as_mut_ptr().cast(),
                iov_len: 8,
            },
            Iovec {
                iov_base: b_base,
                iov_len: 8,
            },
        ];
        let mut uio = Uio::<Procs> {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: 16,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: None,
        };
        crate::testutil::set_uiomove_fault(Some(2));
        let mut r = XdrReader::new(&m);
        // The C drops copyout's error (C:nfs/nfs_subs.c:668-688): the read succeeds.
        assert_eq!(r.to_uio(&mut uio, 13), Ok(()));
        let (resid, off, left) = (uio.uio_resid, uio.uio_offset, uio.uio_iov.len());
        let last = uio.uio_iov.first().map(|i| (i.iov_base, i.iov_len));
        drop(uio);
        crate::testutil::set_uiomove_fault(None);
        crate::testutil::set_copy_kernel(false);
        // The uio stands where a full copy leaves it.
        assert_eq!((resid, off, left), (3, 13, 1));
        assert_eq!(last, Some((b_base.wrapping_byte_add(5), 3)));
        assert_eq!(a, data[..8]);
        assert_eq!(b[..4], [0xee; 4], "the faulted leg wrote nothing");
        assert_eq!(b[4], 13);
        assert_eq!(b[5..], [0xee; 3]);
        // The padding skipped: the next word is the reply's.
        assert_eq!(r.offset(), 16);
        assert_eq!(r.u32(), Ok(0xaabb_ccdd));
    }

    #[test]
    fn a_failed_uiomove_leaves_zeros_not_the_mbufs_old_bytes() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        // The tail's storage full of an old pattern, as a recycled mbuf's may be.
        let mut m = m_get(M_WAIT, MT_DATA).expect("an mbuf");
        if let Some(st) = m.storage_mut() {
            st.fill(0xa5);
        }
        m.m_len().set(0);
        let mut w = XdrWriter::with_first(m);
        let src0 = [1u8; 4];
        let src1 = [2u8; 4];
        let mut iov = [
            Iovec {
                iov_base: src0.as_ptr().cast_mut().cast(),
                iov_len: 4,
            },
            Iovec {
                iov_base: src1.as_ptr().cast_mut().cast(),
                iov_len: 4,
            },
        ];
        let mut uio = Uio::<Procs> {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: 8,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_WRITE,
            uio_procp: None,
        };
        // The second iovec's leg faults.
        crate::testutil::set_uiomove_fault(Some(1));
        w.from_uio(&mut uio, 8);
        drop(uio);
        crate::testutil::set_uiomove_fault(None);
        crate::testutil::set_copy_kernel(false);
        assert_eq!(test_bytes(&w.into_chain()), [1, 1, 1, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn property_mtouio_around_the_uio_and_the_chain() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        let mut rng = TestRng(0x6e66_735f_7569_6f31);
        for _ in 0..600 {
            let n = rng.below(120);
            let data = rng.bytes(n);
            let m = test_chain(&rng.split(&data));
            let len = rng.below(n + 8) as i32 - 4;
            let room = rng.below(n + 8);
            let mut r = XdrReader::new(&m);
            let (res, buf, resid) = read_into(&mut r, len, room);
            let siz = usize::try_from(len).unwrap_or(0);
            if siz == 0 {
                assert_eq!((res, resid, r.offset()), (Ok(()), room, 0));
            } else if siz > room {
                // The uio fills first unless the chain ends before it.
                let want = if room > n {
                    Errno::EBADRPC
                } else {
                    Errno::EFBIG
                };
                assert_eq!(res, Err(want), "len {len} room {room} n {n}");
            } else if siz > n {
                assert_eq!(res, Err(Errno::EBADRPC));
            } else {
                assert_eq!(buf[..siz], data[..siz]);
                assert_eq!(resid, room - siz);
                // The padding skipped, or EBADRPC when the chain has not all of it.
                let padded = nfsm_rndup(siz);
                if padded <= n {
                    assert_eq!((res, r.offset()), (Ok(()), padded));
                } else {
                    assert_eq!(res, Err(Errno::EBADRPC));
                }
            }
            assert!(
                buf[siz.min(room)..].iter().all(|&b| b == 0xee),
                "never past the data"
            );
        }
        crate::testutil::set_copy_kernel(false);
    }

    #[test]
    fn uiotombuf_appends_the_uio_and_its_padding_across_mbufs() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        let data: Vec<u8> = (0..(MLEN * 3 + 5)).map(|i| (i * 7 + 3) as u8).collect();
        let mut w = XdrWriter::reqhead(0);
        w.u32(7);
        let mut iov = [Iovec::new()];
        let mut uio = Uio::<Procs>::kernel_write(&mut iov, &data, None);
        uio.uio_resid = 0; // nfsm_uiotombuf sets it, as the C
        w.from_uio(&mut uio, data.len());
        assert_eq!(uio.uio_resid, 0);
        drop(uio);
        let m = w.into_chain();
        let mut want = 7u32.to_be_bytes().to_vec();
        want.extend_from_slice(&data);
        want.extend_from_slice(&[0, 0, 0]);
        assert_eq!(test_bytes(&m), want);
        assert!(test_lens(&m).len() >= 2);
        crate::testutil::set_copy_kernel(false);
    }

    #[test]
    fn strings_and_buffers_keep_the_c_layout() {
        let _g = setup();
        // The tail with two bytes of room: the length word straddles, as through the C's uio.
        let mut w = XdrWriter::with_first(test_chain(&[&[]]));
        let room = w.trailing();
        let pre = vec![0x55u8; room - 2];
        w.buftombuf(&pre);
        assert_eq!(w.trailing(), room - nfsm_rndup(room - 2));
        let name = b"hello";
        let mut w2 = XdrWriter::with_first(test_chain(&[&[]]));
        let fill = vec![1u8; w2.trailing() - 2];
        w2.build_slice(fill.len()).copy_from_slice(&fill);
        w2.strtombuf(name);
        let m = w2.into_chain();
        assert_eq!(
            test_lens(&m)[0],
            fill.len() + 2,
            "the word's first half in the first mbuf"
        );
        let mut want = fill.clone();
        want.extend_from_slice(&[0, 0, 0, 5]);
        want.extend_from_slice(name);
        want.extend_from_slice(&[0, 0, 0]);
        assert_eq!(test_bytes(&m), want);
        drop(w);
    }

    #[test]
    fn file_handles_by_version() {
        let _g = setup();
        let mut fh = Nfsfh::new();
        for (i, b) in fh.fh_bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut w = XdrWriter::reqhead(0);
        w.fhtom(&fh, 28, true);
        w.fhtom(&fh, 28, false);
        let m = w.into_chain();
        let b = test_bytes(&m);
        assert_eq!(b[..4], 28u32.to_be_bytes());
        assert_eq!(b[4..32], fh.fh_bytes[..28]);
        assert_eq!(b[32..64], fh.fh_bytes[..NFSX_V2FH]);
        assert_eq!(b.len(), 64);
    }

    #[test]
    fn v3_sattr() {
        let _g = setup();
        let mut va = Vattr::new();
        crate::fs::subr::vattr_null(&mut va);
        va.va_mode = 0o644;
        va.va_size = 0x1_0000_0002;
        va.va_mtime = Timespec::new(12_345, 0);
        let words = |full: bool| {
            let mut w = XdrWriter::reqhead(0);
            w.v3attrbuild(&va, full);
            let b = test_bytes(&w.into_chain());
            b.chunks(4)
                .map(|c| u32::from_be_bytes(c.try_into().unwrap()))
                .collect::<Vec<_>>()
        };
        // mode set; uid, gid unset; size only when full; atime unchanged; mtime to the client's.
        assert_eq!(
            words(true),
            [
                1,
                0o644,
                0,
                0,
                1,
                1,
                2,
                NFSV3SATTRTIME_DONTCHANGE,
                NFSV3SATTRTIME_TOCLIENT,
                12_345,
                0
            ]
        );
        assert_eq!(
            words(false),
            [
                1,
                0o644,
                0,
                0,
                0,
                NFSV3SATTRTIME_DONTCHANGE,
                NFSV3SATTRTIME_TOCLIENT,
                12_345,
                0
            ]
        );
    }

    #[test]
    fn version2_times() {
        let t = txdr_nfsv2time(&Timespec::new(5, 7_000));
        assert_eq!(
            (fxdr_unsigned(t.nfsv2_sec), fxdr_unsigned(t.nfsv2_usec)),
            (5, 7)
        );
        let none = txdr_nfsv2time(&Timespec::new(5, i64::from(VNOVAL)));
        assert_eq!(
            (none.nfsv2_sec, none.nfsv2_usec),
            (nfs_xdrneg1, nfs_xdrneg1)
        );
        let m1 = txdr_nfsv2time(&Timespec::new(-1, 0));
        assert_eq!(fxdr_unsigned(m1.nfsv2_sec) as i32, -2);
        assert_eq!(fxdr_unsigned(m1.nfsv2_usec), 999_999);
    }

    #[test]
    fn tables() {
        assert_eq!(NFSV2_TYPE[VFIFO as usize], NFCHR);
        assert_eq!(NFSV3_TYPE[VSOCK as usize], NFSOCK);
        assert_eq!(NFSV3_TYPE[Vtype::VBAD as usize], NFNON);
        assert_eq!(NV3TOV_TYPE[NFFIFO as usize], VFIFO);
        assert_eq!(NV2TOV_TYPE[NFSOCK as usize], VNON);
        assert_eq!(fxdr_unsigned(rpc_call), 0);
        assert_eq!(fxdr_unsigned(nfs_prog), 100_003);
        assert_eq!(fxdr_unsigned(nfs_true), 1);
    }

    // N6.7a2: the attribute cache (the timeout's arithmetic, the hits and misses, the size
    // rules, the special file times), the commit ranges (the C's table and the merge, with
    // property tests of their bounds) and `nfs_clearcommit`.

    use crate::fs::bio::{bdwrite, brelse, getblk_wait};
    use crate::fs::nfs::nfs_node::{test_mount, test_node};
    use crate::fs::testfs::TestGlue as G;
    use crate::fs::vnode::{VDIR, VREG};

    /// `n_mtime` set `age` seconds before now.
    fn modified_ago(np: &NfsNode<G>, age: i64) {
        let _ = np.n_mtime.replace(Timespec {
            tv_sec: gettime().wrapping_sub(age),
            tv_nsec: 0,
        });
    }

    /// `nfs_attrtimeo`: a tenth of the age, clamped by the mount's bounds of the node's type
    /// (files 3..60, directories 30..60), the minimum while the node is modified.
    #[test]
    fn the_timeout_is_a_tenth_of_the_age_clamped_by_type() {
        let t = test_mount(0);
        let (_f, file) = test_node(t.mp, VREG, &[1; 8]);
        let (_d, dir) = test_node(t.mp, VDIR, &[2; 8]);
        // (node, age in seconds, modified, timeout)
        let cases: [(&NfsNode<G>, i64, bool, i32); 13] = [
            (&file, 0, false, 3),
            (&file, 29, false, 3),
            (&file, 30, false, 3),
            (&file, 39, false, 3),
            (&file, 40, false, 4),
            (&file, 100, false, 10),
            (&file, 599, false, 59),
            (&file, 600, false, 60),
            (&file, 100_000, false, 60),
            (&file, 100, true, 3),
            (&dir, 100, false, 30),
            (&dir, 500, false, 50),
            (&dir, 9_000, false, 60),
        ];
        for (np, age, modified, want) in cases {
            modified_ago(np, age);
            if modified {
                np.set(NMODIFIED);
            } else {
                np.clr(NMODIFIED);
            }
            assert_eq!(
                nfs_attrtimeo(np, &t.nmp),
                want,
                "age {age} modified {modified}"
            );
        }
        dir.set(NMODIFIED);
        modified_ago(&dir, 9_000);
        assert_eq!(nfs_attrtimeo(&dir, &t.nmp), 30, "a modified directory");
    }

    /// The timeout over every age and both bounds: always within `min..=max`, the minimum
    /// for a modified node, a tenth of the age between the bounds.
    #[test]
    fn the_timeout_stays_within_its_bounds_for_any_age() {
        let t = test_mount(0);
        let (_f, np) = test_node(t.mp, VREG, &[3; 8]);
        let mut rng = TestRng(0x6174_7472_7469_6d65);
        for _ in 0..2_000 {
            let min = rng.below(100) as u16;
            let max = min + rng.below(400) as u16;
            t.nmp.nm_acregmin.store(min, Ordering::Relaxed);
            t.nmp.nm_acregmax.store(max, Ordering::Relaxed);
            let age = rng.below(10_000) as i64;
            modified_ago(&np, age);
            let modified = rng.below(4) == 0;
            if modified {
                np.set(NMODIFIED);
            } else {
                np.clr(NMODIFIED);
            }
            let to = nfs_attrtimeo(&np, &t.nmp);
            let (min, max) = (i32::from(min), i32::from(max));
            assert!(to >= min && to <= max.max(min), "{to} {min} {max}");
            let tenth = (age / 10) as i32;
            if modified {
                assert_eq!(to, min);
            } else if tenth >= min && tenth < max {
                assert_eq!(to, tenth);
            }
        }
    }

    /// A node never stamped, or stamped a timeout ago or more, misses (`ENOENT`, counted);
    /// one stamped less than a timeout ago hits (counted) and copies its attributes.
    #[test]
    fn the_cache_misses_unstamped_and_expired_and_hits_inside_the_timeout() {
        let t = test_mount(0);
        let (_vp, np) = test_node(t.mp, VREG, &[4; 8]);
        modified_ago(&np, 0); // a timeout of 3 s
        let mut va = Vattr::new();
        va.va_type = VREG;
        va.va_size = 77;
        va.va_nlink = 5;
        let _ = np.n_vattr.replace(va);
        np.n_size.store(77, Ordering::Relaxed);
        let (hits, misses) = (&NFSSTATS.attrcache_hits, &NFSSTATS.attrcache_misses);
        let (h0, m0) = (hits.load(Ordering::Relaxed), misses.load(Ordering::Relaxed));
        let mut out = Vattr::new();
        assert_eq!(nfs_getattrcache(&np, &t.nmp, &mut out), Err(Errno::ENOENT));
        np.n_attrstamp
            .store(gettime().wrapping_sub(3), Ordering::Relaxed);
        assert_eq!(nfs_getattrcache(&np, &t.nmp, &mut out), Err(Errno::ENOENT));
        assert_eq!(misses.load(Ordering::Relaxed), m0 + 2);
        assert_eq!(hits.load(Ordering::Relaxed), h0);
        np.n_attrstamp
            .store(gettime().wrapping_sub(2), Ordering::Relaxed);
        assert_eq!(nfs_getattrcache(&np, &t.nmp, &mut out), Ok(()));
        assert_eq!(hits.load(Ordering::Relaxed), h0 + 1);
        assert_eq!((out.va_size, out.va_nlink), (77, 5));
        // A directory's timeout is its own (30 s at least).
        let (_d, dir) = test_node(t.mp, VDIR, &[5; 8]);
        modified_ago(&dir, 0);
        let mut dva = Vattr::new();
        dva.va_type = VDIR;
        let _ = dir.n_vattr.replace(dva);
        dir.n_attrstamp
            .store(gettime().wrapping_sub(29), Ordering::Relaxed);
        assert_eq!(nfs_getattrcache(&dir, &t.nmp, &mut out), Ok(()));
        dir.n_attrstamp
            .store(gettime().wrapping_sub(30), Ordering::Relaxed);
        assert_eq!(nfs_getattrcache(&dir, &t.nmp, &mut out), Err(Errno::ENOENT));
    }

    /// The size the cache answers: an unmodified file takes the server's size, a modified
    /// one keeps its own larger size (written into the cache) or takes a larger server's, a
    /// non-file takes the server's; the special file times replace the copy's while `NCHG`.
    #[test]
    fn the_size_and_the_times_a_hit_answers() {
        let t = test_mount(0);
        let (_vp, np) = test_node(t.mp, VREG, &[6; 8]);
        let (_d, dir) = test_node(t.mp, VDIR, &[7; 8]);
        // (node, type, cached size, node size, modified, answer, node size after, cache after)
        let cases: [(&NfsNode<G>, Vtype, u64, u64, bool, u64, u64, u64); 6] = [
            (&np, VREG, 100, 50, false, 100, 100, 100),
            (&np, VREG, 100, 200, false, 100, 100, 100),
            (&np, VREG, 100, 200, true, 200, 200, 200),
            (&np, VREG, 300, 200, true, 300, 300, 300),
            (&np, VREG, 64, 64, true, 64, 64, 64),
            (&dir, VDIR, 512, 1024, true, 512, 512, 512),
        ];
        for (node, ty, cached, size, modified, want, size_after, cache_after) in cases {
            modified_ago(node, 0);
            node.n_attrstamp
                .store(gettime().wrapping_sub(1), Ordering::Relaxed);
            let mut va = Vattr::new();
            va.va_type = ty;
            va.va_size = cached;
            let _ = node.n_vattr.replace(va);
            node.n_size.store(size, Ordering::Relaxed);
            if modified {
                node.set(NMODIFIED);
            } else {
                node.clr(NMODIFIED);
            }
            let mut out = Vattr::new();
            assert_eq!(nfs_getattrcache(node, &t.nmp, &mut out), Ok(()));
            assert_eq!(out.va_size, want, "{cached} {size} {modified}");
            assert_eq!(node.n_size.load(Ordering::Relaxed), size_after);
            assert_eq!(node.n_vattr.read().va_size, cache_after);
        }
        // The special file times: NACC and NUPD each replace their time, only under NCHG.
        let at = Timespec {
            tv_sec: 11,
            tv_nsec: 1,
        };
        let mt = Timespec {
            tv_sec: 22,
            tv_nsec: 2,
        };
        let _ = np.n_atim.replace(at);
        let _ = np.n_mtim.replace(mt);
        let mut va = Vattr::new();
        va.va_type = VREG;
        let _ = np.n_vattr.replace(va);
        np.n_size.store(0, Ordering::Relaxed);
        np.n_attrstamp
            .store(gettime().wrapping_sub(1), Ordering::Relaxed);
        for (flags, want_a, want_m) in [
            (NACC | NUPD, Timespec::ZERO, Timespec::ZERO),
            (NCHG, Timespec::ZERO, Timespec::ZERO),
            (NCHG | NACC, at, Timespec::ZERO),
            (NCHG | NUPD, Timespec::ZERO, mt),
            (NCHG | NACC | NUPD, at, mt),
        ] {
            np.n_flag.store(flags, Ordering::Relaxed);
            let mut out = Vattr::new();
            assert_eq!(nfs_getattrcache(&np, &t.nmp, &mut out), Ok(()));
            assert_eq!((out.va_atime, out.va_mtime), (want_a, want_m), "{flags:#x}");
        }
    }

    /// A buffer of dirty bytes `[blkno * 512, blkno * 512 + dirtyend)`.
    fn dirty(blkno: i64, dirtyend: i32) -> Buf {
        let bp = Buf::new();
        bp.set_blkno(blkno);
        bp.set_dirtyend(dirtyend);
        bp
    }

    /// The pushed range's words and valid bit.
    fn pushed(np: &NfsNode<G>) -> Option<(i64, i64)> {
        (np.n_commitflags.load(Ordering::Relaxed) & NFS_COMMIT_PUSHED_VALID != 0).then(|| {
            (
                np.n_pushedlo.load(Ordering::Relaxed),
                np.n_pushedhi.load(Ordering::Relaxed),
            )
        })
    }

    /// The to-be-committed range's words and valid bit.
    fn push(np: &NfsNode<G>) -> Option<(i64, i64)> {
        (np.n_commitflags.load(Ordering::Relaxed) & NFS_COMMIT_PUSH_VALID != 0).then(|| {
            (
                np.n_pushlo.load(Ordering::Relaxed),
                np.n_pushhi.load(Ordering::Relaxed),
            )
        })
    }

    /// The C's arithmetic of both ranges: the first add makes the range the buffer's, later
    /// ones widen it; the membership tests; a delete of a disjoint buffer does nothing, one
    /// over the low or the high end cuts it there, one in the middle keeps the smaller piece
    /// (the C's comparison); nothing of an invalid range.
    #[test]
    fn the_commit_ranges_follow_the_c() {
        let t = test_mount(0);
        let (_vp, np) = test_node(t.mp, VREG, &[8; 8]);
        type Add = fn(&NfsNode<G>, &Buf);
        type In = fn(&NfsNode<G>, &Buf) -> bool;
        type Get = fn(&NfsNode<G>) -> Option<(i64, i64)>;
        let sets: [(Add, Add, In, Get); 2] = [
            (
                nfs_add_committed_range::<G>,
                nfs_del_committed_range::<G>,
                nfs_in_committed_range::<G>,
                pushed,
            ),
            (
                nfs_add_tobecommitted_range::<G>,
                nfs_del_tobecommitted_range::<G>,
                nfs_in_tobecommitted_range::<G>,
                push,
            ),
        ];
        for (add, del, isin, get) in sets {
            np.n_commitflags.store(0, Ordering::Relaxed);
            assert!(!isin(&np, &dirty(0, 0)), "an invalid range holds nothing");
            del(&np, &dirty(0, 512));
            assert_eq!(get(&np), None);
            add(&np, &dirty(4, 1024)); // [2048, 3072)
            assert_eq!(get(&np), Some((2048, 3072)));
            add(&np, &dirty(2, 100)); // [1024, 1124): the low end widens
            assert_eq!(get(&np), Some((1024, 3072)));
            add(&np, &dirty(8, 512)); // [4096, 4608): the high end widens
            assert_eq!(get(&np), Some((1024, 4608)));
            assert!(isin(&np, &dirty(2, 3584)));
            assert!(!isin(&np, &dirty(2, 3585)), "one byte past the high end");
            assert!(!isin(&np, &dirty(1, 512)), "below the low end");
            // Disjoint: nothing changes.
            del(&np, &dirty(10, 100)); // [5120, 5220)
            del(&np, &dirty(0, 512)); // [0, 512)
            assert_eq!(get(&np), Some((1024, 4608)));
            // Over the low end: the range starts at the buffer's end.
            del(&np, &dirty(1, 1024)); // [512, 1536)
            assert_eq!(get(&np), Some((1536, 4608)));
            // Over the high end: the range ends at the buffer's start.
            del(&np, &dirty(8, 1024)); // [4096, 5120)
            assert_eq!(get(&np), Some((1536, 4096)));
            // The middle, the left piece the smaller: the C keeps it.
            del(&np, &dirty(4, 512)); // [2048, 2560): left 512, right 1536
            assert_eq!(get(&np), Some((1536, 2048)));
            add(&np, &dirty(3, 2560)); // [1536, 4096) again
            assert_eq!(get(&np), Some((1536, 4096)));
            // The middle, the right piece the smaller: the C keeps it.
            del(&np, &dirty(6, 512)); // [3072, 3584): left 1536, right 512
            assert_eq!(get(&np), Some((3584, 4096)));
        }
    }

    /// `nfs_merge_commit_ranges`: the to-be-committed range becomes the committed one, or
    /// widens it; it is cleared and invalid after.
    #[test]
    fn the_merge_folds_the_push_range_into_the_pushed_one() {
        let t = test_mount(0);
        let (_vp, np) = test_node(t.mp, VREG, &[9; 8]);
        nfs_add_tobecommitted_range(&np, &dirty(2, 512));
        nfs_merge_commit_ranges(&np);
        assert_eq!(pushed(&np), Some((1024, 1536)));
        assert_eq!(push(&np), None);
        assert_eq!(
            (
                np.n_pushlo.load(Ordering::Relaxed),
                np.n_pushhi.load(Ordering::Relaxed)
            ),
            (0, 0)
        );
        nfs_add_tobecommitted_range(&np, &dirty(1, 100));
        nfs_merge_commit_ranges(&np);
        assert_eq!(pushed(&np), Some((512, 1536)));
        nfs_add_tobecommitted_range(&np, &dirty(8, 100));
        nfs_merge_commit_ranges(&np);
        assert_eq!(pushed(&np), Some((512, 4196)));
        nfs_add_tobecommitted_range(&np, &dirty(2, 1));
        nfs_merge_commit_ranges(&np);
        assert_eq!(pushed(&np), Some((512, 4196)), "inside: no change");
    }

    /// Property: an added buffer is in its range, which holds what it held; a deleted
    /// buffer of dirty bytes that met the range is no longer in it; a merge's range holds
    /// both the ranges it folded.
    #[test]
    fn the_ranges_hold_what_was_added_and_not_what_was_deleted() {
        let t = test_mount(0);
        let (_vp, np) = test_node(t.mp, VREG, &[10; 8]);
        let mut rng = TestRng(0x636f_6d6d_6974_7267);
        let mut bufs = std::vec::Vec::new();
        for _ in 0..5_000 {
            np.n_commitflags.store(0, Ordering::Relaxed);
            bufs.clear();
            for _ in 0..1 + rng.below(4) {
                let bp = dirty(rng.below(64) as i64, 1 + rng.below(8192) as i32);
                nfs_add_committed_range(&np, &bp);
                bufs.push(bp);
            }
            for bp in &bufs {
                assert!(nfs_in_committed_range(&np, bp));
            }
            let d = dirty(rng.below(64) as i64, 1 + rng.below(8192) as i32);
            let before = pushed(&np).expect("valid");
            let (lo, hi) = commit_range(&d);
            nfs_del_committed_range(&np, &d);
            if lo > before.1 || hi < before.0 {
                assert_eq!(pushed(&np), Some(before), "disjoint");
            } else {
                assert!(!nfs_in_committed_range(&np, &d), "deleted {lo}..{hi}");
            }
            // The merge: a to-be-committed range folded into the committed one.
            let p = dirty(rng.below(64) as i64, 1 + rng.below(8192) as i32);
            nfs_add_tobecommitted_range(&np, &p);
            let old = pushed(&np).expect("valid");
            nfs_merge_commit_ranges(&np);
            let merged = pushed(&np).expect("valid");
            let (plo, phi) = commit_range(&p);
            assert_eq!(merged, (old.0.min(plo), old.1.max(phi)));
            assert_eq!(push(&np), None);
        }
    }

    /// `nfs_clearcommit`: every dirty buffer of the mount's vnodes that is delayed and to be
    /// committed, and not busy, loses `B_NEEDCOMMIT`; a busy one, and one not delayed, keep
    /// it.
    #[test]
    fn clearcommit_clears_the_idle_delayed_buffers_only() {
        let t = test_mount(0);
        let (a, _na) = test_node(t.mp, VREG, &[11; 8]);
        let (b, _nb) = test_node(t.mp, VREG, &[12; 8]);
        for vp in [a, b] {
            for blk in 0..3 {
                bdwrite(getblk_wait(&vp.bufs, blk, 512));
            }
        }
        for vp in [a, b] {
            vnode_dirty_for_each(&vp.bufs, |bp| bp.set(B_NEEDCOMMIT));
        }
        // b's block 1 is busy (taken off the free queues, still on the dirty list).
        let busy = getblk_wait(&b.bufs, 1, 512);
        // a's block 2 is to be committed but not delayed (as a buffer half way).
        vnode_dirty_for_each(&a.bufs, |bp| {
            if bp.lblkno() == 2 {
                bp.clr(B_DELWRI);
            }
        });
        nfs_clearcommit(t.mp);
        let mut seen = std::vec::Vec::new();
        for (name, vp) in [("a", a), ("b", b)] {
            vnode_dirty_for_each(&vp.bufs, |bp| {
                seen.push((name, bp.lblkno(), bp.isset(B_NEEDCOMMIT)));
            });
        }
        seen.sort();
        assert_eq!(
            seen,
            [
                ("a", 0, false),
                ("a", 1, false),
                ("a", 2, true),
                ("b", 0, false),
                ("b", 1, true),
                ("b", 2, false),
            ]
        );
        vnode_dirty_for_each(&a.bufs, |bp| {
            if bp.lblkno() == 2 {
                bp.set(B_DELWRI);
            }
        });
        brelse(busy);
    }
}
/* </TESTS> */
