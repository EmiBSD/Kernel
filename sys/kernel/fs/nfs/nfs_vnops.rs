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
 *	@(#)nfs_vnops.c	8.16 (Berkeley) 5/27/95
 */
/* </LICENSES> */

/* <CODE> */
//! vnode op calls for Sun NFS version 2 and 3 in the kernel: the client's vnode operations
//! (`nfs_vops`, [`NfsVnops`]), the operations of special devices ([`NfsSpecVnops`]) and fifos
//! ([`NfsFifoVnops`]) on an NFS mount, and the RPCs behind them.
//!
//! Upstream: sys/nfs/nfs_vnops.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_vnops.rs@e8249449d905
//!
//! Each operation builds its request with an `XdrWriter`, sends it with `nfs_request` and
//! dissects the reply with an `XdrReader`; the attribute cache (`nfs_loadattrcache`) and the
//! access cache in the nfsnode avoid RPCs where the C does. Reads and writes go through the
//! buffer cache (`nfs_bio`), which calls back into [`nfs_readrpc`], [`nfs_writerpc`],
//! [`nfs_readlinkrpc`] and [`nfs_writebp`]. Built beside `bsd`'s client and registered by
//! N6.7a5 (`nfs_vfsops`).
//!
//! ## Deviations
//! - D-3: an RPC that fails with a version 3 status that is no errno(2) number
//!   (`NFSERR_BADHANDLE` .. `NFSERR_JUKEBOX`) carries it inside the client
//!   (`NfsError::Status`), so the C's tests of `NFSERR_NOTSUPP` (`nfs_create`'s retry without
//!   `O_EXCL`, `nfs_readdir`'s fallback from READDIRPLUS) and `NFSERR_BAD_COOKIE`
//!   (`nfs_readdir`'s `EINVAL`) match again; a vnode operation still answers such a status as
//!   `EIO` where OpenBSD hands the raw number up (narrowed from LZ's, not closed).
//!   `NFSERR_STALEWRITEVERF` (30001, [`nfs_commit`]'s fake) is one of those statuses.
//! - `option FIFO` is in GENERIC but `miscfs/fifofs` is not ported (the ffs, tmpfs and cd9660
//!   precedent): [`NfsFifoVnops`] is defined, but `nfs_loadattrcache` takes the C's `!FIFO`
//!   path (`EOPNOTSUPP`) and never installs it; the `fifo_*` operations it names
//!   (`fifo_open`, `fifo_close`, `fifo_read`, `fifo_write`, `fifo_ioctl`, `fifo_kqfilter`,
//!   `fifo_pathconf`, `fifo_advlock`, `fifo_reclaim`, `fifo_printinfo`) are visible gaps
//!   (`unported!`, `ENOSYS`), as LZ's.
//! - `VOP_GETATTR`/`VOP_SETATTR` take a thread: where the C passes a pointer that may be NULL
//!   (`uio_procp`, a close's `a_p`) this file passes it, else `curproc`, else `proc0` (LZ's).
//! - A credential the C dereferences (the request's `AUTH_UNIX` ids, `crdup`) must be a real
//!   one: `NOCRED`/`FSCRED` are `panic(9)` where the C would follow the sentinel, and a read
//!   or a write of a node with no `n_rcred`/`n_wcred` yet is `panic(9)` where the C would
//!   follow NULL (LZ's).
//! - `nfs_lookitup`'s error path skips the `vput` of a node it never got (a version 3 LOOKUP
//!   that fails with `*npp == NULL`: the C would `vput(NULL)`), as LZ's.
//! - `nfs_lookup`'s return when it cannot relock the directory after a name cache hit
//!   leaves the hit's vnode referenced, as the C's; it is unreachable (`vn_lock` with
//!   `LK_RETRY` loops until it has the lock, kernel::fs::vnode's `vn_lock`, C
//!   kern/vfs_vnops.c:599), so it is kept, not fixed. The C's `#if 0` `vprint` in
//!   `nfs_flush` stays a comment.
//! - `nfs_print` prints under feature `debug` or `diagnostic`; the C's third condition,
//!   `VFSLCKDEBUG`, has no feature here. The `DIAGNOSTIC` checks and `printf`s are behind
//!   feature `diagnostic`.
//! - `nfs_writebp`'s return when a buffer's `B_NEEDCOMMIT` was cleared while it waited for
//!   the commit lock is `panic(9)`, where the C returns with the buffer still busy and
//!   `v_numoutput` raised (a later `vwaitforio` would sleep for ever); no path reaches it
//!   (the busy buffer is the caller's, so no one else clears the flag).
//! - Kept as the C: a version 3 WRITE reply whose count exceeds the request's (`rlen > len`)
//!   is not refused; `nfs_fsinfo`'s readdir clamp compares with the read maximum (`max`), as
//!   the C's reuse of the variable does; `nfs_readdirplusrpc` answers the error of the last
//!   entry's `nfs_nget` when it failed (a later one's success clears it, the C's
//!   assignment); `nfs_create` leaves the new vnode locked and referenced when the
//!   exclusive create's SETATTR fails (C:nfs/nfs_vnops.c:1647: the error is returned with
//!   `newvp` neither put nor handed up), a slip of the C reported, not fixed.
//!
//! ## Redesign
//! - LZ's `NFS_VOPS`, `NFS_SPECVOPS` and `NFS_FIFOVOPS` (`bsd` `Vops` of function pointers
//!   over argument structures) -> [`NfsVnops`], [`NfsSpecVnops`], [`NfsFifoVnops`], one
//!   `impl Vnops<G>` each of the C's slots (decision 46), members of the glue's `NfsTables`
//!   (one address per glue, for `vops_id`; a byte each, so that none is zero-sized), tmpfs's
//!   and fuse's shape. Each slot calls the C-named function, generic over the glue.
//! - The node and the mount are counted handles taken once at a vnode operation's entry
//!   (`vtonfs`, `vfstonfs`: the slot's reader and a count pair each where the C casts) and
//!   lent down to the RPCs, the attribute cache and the buffer cache (the hot path's rule);
//!   an RPC on another vnode (the directory of a remove, a rename's two) takes that vnode's
//!   handles itself, as the C's `VTONFS` there. `nfs_nget` answers a node's handle (the C's
//!   `*npp`), whose vnode is `nfstov`'s: a device's node that `nfs_loadattrcache` moves to an
//!   alias carries its vnode with it, so no `*vpp` is threaded through the dissections.
//! - The C's `struct nfsm_info` and `goto nfsmout` -> a reply's `Option<Reply>` and its
//!   reader ([`Mrep`]: `None` when no reply came, the C's NULL `nmi_mrep`); a dissection's
//!   error is `?`'d out, which overwrites the request's error as the C's `*nmi_errorp`; each
//!   function's `nfsmout:` tail runs after an inner body that answers its error. The helpers
//!   `nfsm_loadattr`, `nfsm_wcc_data`, `nfsm_getfh` and `nfsm_mtofh` keep the C's flags and
//!   out-arguments; `nfsm_getfh` answers a copy of the handle (`NfsFh`), never a pointer
//!   into the reply.
//! - The readdir RPCs pack their `struct nfs_dirent` records into `nfs_readdir`'s own buffer
//!   (`malloc(M_TEMP)`, an `MArray`) through [`DirPack`], a cursor over the slice at the C's
//!   offsets, not a uio (the C's `UIO_SYSSPACE` uio over that buffer); the position survives
//!   from a READDIRPLUS that failed to the READDIR that follows, as the C's uio does; the
//!   offset the READDIR starts from is the uio's, which the names the READDIRPLUS copied
//!   moved on (`nfsm_mtouio`, C:nfs/nfs_subs.c:674), as the C's.
//!   `nfs_readdir` fixes each record up in place and copies its `d_reclen` bytes out of the
//!   buffer with the glue's `uiomove_out`: the C's bytes, no second image.
//! - `n_rcred`/`n_wcred` are counted credentials in the node's kernel-lock cell, left alone
//!   when the cell holds the same credentials already (the same item, or ids the
//!   `AUTH_UNIX` header carries the same), so a read or a write of a file does not change
//!   them per call; a credential no pool placed (a thread's own copy) is held as a copy
//!   (`crdup`). The RPCs that use one clone it out of the cell for the request (one reader
//!   section and a count pair per RPC, docs/ROADMAP.md N12's row).
//! - The silly rename record is an `MBox` of `M_NFSREQ` in the node's kernel-lock cell, its
//!   credential a `crdup`'s `CredRef` and its directory a referenced vnode, given up by
//!   `nfs_inactive`.
//! - `nfs_flush`'s commit pass takes its buffers in one pass and one section of the cache's
//!   lock (`vnode_dirty_gather`, the C's `LIST_FOREACH_SAFE` at `splbio` filling `bvec`), its
//!   write pass restarts from the head after each write (`vnode_dirty_scan`), as the C's
//!   `goto loop`; a committed buffer is made clean, counted and done in one section
//!   (`buf_undirty_done`), `nfs_writebp`'s first part likewise (`buf_undirty_output`): the
//!   C's `splbio` pairs, one each.
//! - `nfs_writebp` and `nfs_strategy` hand the buffer's token on: a synchronous write keeps
//!   a counted link to the buffer and takes the token back from `biowait` (the kernel's
//!   `bwrite` form).

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::mem::malloc::{M_WAITOK, MArray, MBox};
use hal::random::arc4random_buf;
use hal::sync::klock;
use hal::sync::sleep::{INFSLP, PCATCH};
use sysdefs::dirent::{DT_UNKNOWN, Dirent, dirent_recsize, iftodt};
use sysdefs::errno::Errno;
use sysdefs::fcntl::{FREAD, FWRITE, Flock, O_EXCL};
use sysdefs::mount::{MNT_RDONLY, MNT_WAIT};
use sysdefs::param::PAGE_SIZE;
use sysdefs::syslimits::{LINK_MAX, NAME_MAX};
use sysdefs::types::{Daddr, Register, major, minor};
use sysdefs::unistd::{
    _PC_2_SYMLINKS, _PC_ALLOC_SIZE_MIN, _PC_CHOWN_RESTRICTED, _PC_FILESIZEBITS, _PC_LINK_MAX,
    _PC_NAME_MAX, _PC_NO_TRUNC, _PC_REC_INCR_XFER_SIZE, _PC_REC_MAX_XFER_SIZE,
    _PC_REC_MIN_XFER_SIZE, _PC_REC_XFER_ALIGN, _PC_SYMLINK_MAX, _PC_TIMESTAMP_RESOLUTION,
};

use super::nfs::{NfsError, Reply, nfs_cmpfh, nfs_isv3};
use super::nfs_bio::{DEV_BSIZE, nfs_asyncio, nfs_bioread, nfs_doio, nfs_vinvalbuf, nfs_write};
use super::nfs_kq::nfs_kqfilter;
use super::nfs_node::{nfs_inactive, nfs_nget, nfs_nrehash, nfs_reclaim};
use super::nfs_socket::{Request, nfs_request, nfs_sigintr};
use super::nfs_subs::{
    NFSSTATS, nfs_add_committed_range, nfs_attrtimeo, nfs_clearcommit, nfs_false, nfs_getattrcache,
    nfs_in_committed_range, nfs_in_tobecommitted_range, nfs_loadattrcache, nfs_merge_commit_ranges,
    nfs_true, nfs_xdrneg1, txdr_nfsv2time,
};
use super::nfs_vfsops::nfs_fsinfo;
use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_padlen, nfsm_postop_attr, nfsm_rndup};
use super::nfsmount::{NfsMount, NfsMountRef, vfstonfs};
use super::nfsnode::{
    NACC, NCHG, NMODIFIED, NUPD, NWRITEERR, NfsFh, NfsNode, NfsNodeRef, Sillyrename,
    nfs_invalidate_attrcache, nfstov, vtonfs,
};
use super::nfsproto::{
    NFS_FABLKSIZE, NFS_MAXNAMLEN, NFS_MAXPATHLEN, NFSERR_BAD_COOKIE, NFSERR_IO, NFSERR_NOTSUPP,
    NFSERR_STALEWRITEVERF, NFSPROC_ACCESS, NFSPROC_COMMIT, NFSPROC_CREATE, NFSPROC_GETATTR,
    NFSPROC_LINK, NFSPROC_LOOKUP, NFSPROC_MKDIR, NFSPROC_MKNOD, NFSPROC_READ, NFSPROC_READDIR,
    NFSPROC_READDIRPLUS, NFSPROC_READLINK, NFSPROC_REMOVE, NFSPROC_RENAME, NFSPROC_RMDIR,
    NFSPROC_SETATTR, NFSPROC_SYMLINK, NFSPROC_WRITE, NFSV3ACCESS_DELETE, NFSV3ACCESS_EXECUTE,
    NFSV3ACCESS_EXTEND, NFSV3ACCESS_LOOKUP, NFSV3ACCESS_MODIFY, NFSV3ACCESS_READ,
    NFSV3CREATE_EXCLUSIVE, NFSV3CREATE_UNCHECKED, NFSV3WRITE_DATASYNC, NFSV3WRITE_FILESYNC,
    NFSV3WRITE_UNSTABLE, NFSX_UNSIGNED, NFSX_V2FH, NFSX_V3FATTR, NFSX_V3FHMAX, NFSX_V3WRITEVERF,
    Nfsuint64, Nfsv2Sattr, nfsx_fh, nfsx_readdir, nfsx_sattr, vtonfsv2_mode, vtonfsv3_type,
};
use super::xdr_subs::{fxdr_hyper, fxdr_unsigned, txdr_hyper, txdr_unsigned};
use crate::assert::{kassert, kpanic};
use crate::base::prf::unported;
use crate::file::event::{NOTE_ATTRIB, NOTE_DELETE, NOTE_LINK, NOTE_TRUNCATE, NOTE_WRITE};
use crate::file::uio::{Uio, UioRw};
use crate::file::{FileGlue, LockOwner};
use crate::fs::bio::{
    BCSTATS, DirtyPick, DirtyScan, biodone, biowait, brelse, buf_undirty_done, buf_undirty_output,
    vnode_dirty_gather, vnode_dirty_scan,
};
use crate::fs::buf::{
    B_ASYNC, B_BUSY, B_DELWRI, B_DONE, B_ERROR, B_NEEDCOMMIT, B_PHYS, B_RAW, B_READ, B_WRITEINPROG,
    BufRef,
};
use crate::fs::cache::{cache_enter, cache_lookup, cache_purge};
use crate::fs::default;
use crate::fs::getcwd::M_TEMP;
use crate::fs::lockf::lf_advlock;
use crate::fs::mount::Mount;
use crate::fs::namei::MAXPATHLEN;
use crate::fs::namei::{
    CREATE, DELETE, ISDOTDOT, ISLASTCN, LOCKPARENT, LOOKUP, MAKEENTRY, PDIRUNLOCK, RENAME,
    SAVENAME, WANTPARENT,
};
use crate::fs::subr::{vaccess, vattr_null, vput, vref, vrele, vwaitforio};
use crate::fs::vnode::Vtype::{self, VBAD, VBLK, VCHR, VDIR, VFIFO, VLNK, VREG, VSOCK};
use crate::fs::vnode::{
    LK_EXCLUSIVE, LK_RETRY, LK_RWFLAGS, V_SAVE, VA_EXCLUSIVE, VEXEC, VN_KNOTE, VNOVAL, VREAD,
    VWRITE, Vattr, Vnode, vn_lock, vttoif,
};
use crate::fs::vops::{
    Cn, Cred, IoBuf, Vnops, vop_abortop, vop_bwrite, vop_fsync, vop_getattr, vop_islocked,
    vop_setattr, vop_unlock,
};
use crate::proc::cred::{CredRef, Ucred, crdup};
use crate::proc::{ProcGlue, Thread};
use crate::time::tc::{getnanotime, gettime};
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};
use sysdefs::mount::{
    NFSMNT_GOTFSINFO, NFSMNT_HASWRITEVERF, NFSMNT_INT, NFSMNT_NFSV3, NFSMNT_RDIRPLUS,
};

/// `NFSV3_WCCRATTR`: `*flagp` of `nfsm_wcc_data`: answer the post-op attribute flag.
const NFSV3_WCCRATTR: i32 = 0;
/// `NFSV3_WCCCHK`: `*flagp` of `nfsm_wcc_data`: answer whether the pre-op mtime changed.
const NFSV3_WCCCHK: i32 = 1;

/// `NFS_COMMITBVECSIZ`: the most buffers one `nfs_flush` commit pass gathers.
const NFS_COMMITBVECSIZ: usize = 20;

/// `M_NFSREQ` (`<sys/malloc.h>`): NFS request header, the silly rename record's type.
pub const M_NFSREQ: i32 = 22;

/// `NFS_DIRENT_OVERHEAD` (`offsetof(struct nfs_dirent, dirent)`): the NFS cookie's two raw
/// XDR words before each packed `struct dirent`.
const NFS_DIRENT_OVERHEAD: usize = 2 * NFSX_UNSIGNED;
/// `NFS_DIRHDSIZ` (`sizeof (struct nfs_dirent) - (MAXNAMLEN + 1)`): the bytes of a packed
/// record before its name.
const NFS_DIRHDSIZ: usize = NFS_DIRENT_OVERHEAD + Dirent::NAME_OFFSET;

/// Offsets of the members of a packed `struct nfs_dirent` (`cookie[2]`, then `struct
/// dirent`'s `d_fileno`, `d_off`, `d_reclen`, `d_type`, `d_namlen`, its padding and
/// `d_name`).
const ND_COOKIE: usize = 0;
/// `dirent.d_fileno`.
const ND_FILENO: usize = NFS_DIRENT_OVERHEAD + core::mem::offset_of!(Dirent, d_fileno);
/// `dirent.d_off`.
const ND_OFF: usize = NFS_DIRENT_OVERHEAD + core::mem::offset_of!(Dirent, d_off);
/// `dirent.d_reclen`.
const ND_RECLEN: usize = NFS_DIRENT_OVERHEAD + core::mem::offset_of!(Dirent, d_reclen);
/// `dirent.d_type`.
const ND_TYPE: usize = NFS_DIRENT_OVERHEAD + core::mem::offset_of!(Dirent, d_type);
/// `dirent.d_namlen`.
const ND_NAMLEN: usize = NFS_DIRENT_OVERHEAD + core::mem::offset_of!(Dirent, d_namlen);
/// `NFS_READDIRBLKSIZ` as a size.
const READDIRBLKSIZ: usize = super::nfs::NFS_READDIRBLKSIZ as usize;
/// `NFS_DIRBLKSIZ` as a size.
const DIRBLKSIZ: usize = super::nfs::NFS_DIRBLKSIZ as usize;

/// `nfs_vops`: global vfs data structures for nfs (the glue's `NfsTables::vops`).
pub struct NfsVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member (`vops_id` keys tables by address).
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> NfsVnops<G> {
    /// The table (the glue's `NfsTables::vops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `nfs_specvops`: special device vnode ops (the glue's `NfsTables::specvops`).
pub struct NfsSpecVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> NfsSpecVnops<G> {
    /// The table (the glue's `NfsTables::specvops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `nfs_fifovops`: the operations of a fifo on an NFS mount (the glue's
/// `NfsTables::fifovops`; installed by nothing while fifofs is not ported).
pub struct NfsFifoVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> NfsFifoVnops<G> {
    /// The table (the glue's `NfsTables::fifovops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// The reply of an RPC as the C's `info.nmi_mrep` with `nmi_md`/`nmi_dpos` after
/// `nfs_request`: its reader, `None` when no reply came (a transport error, a denial).
pub type Mrep<'a> = Option<XdrReader<'a>>;

/// `nfs_lookitup`'s `npp`: `NULL` (just do the lookup), `*npp == NULL` (allocate a new
/// nfsnode and make sure attributes are handled too: the node comes back in the option),
/// `*npp != NULL` (update the file handle in the vnode of the node given).
pub enum Npp<'a, G: ProcGlue> {
    /// `npp == NULL`.
    Null,
    /// `*npp == NULL`.
    Alloc(&'a mut Option<NfsNodeRef<G>>),
    /// `*npp != NULL`.
    Update(&'a NfsNodeRef<G>),
}

/// The packing of `struct nfs_dirent` records into a readdir RPC's buffer (the C's `uiop`
/// over `nfs_readdir`'s buffer, its `blksiz` and its `dp`): the record being filled and the
/// block it fills, at the C's offsets.
struct DirPack<'b> {
    /// The buffer (`readdir_iovec`'s base at the start of `nfs_readdir`'s RPC).
    buf: &'b mut [u8],
    /// The bytes packed (`iov_base`'s advance; `uio_resid` is the rest of `buf`).
    pos: usize,
    /// `blksiz`: the bytes used in the current `NFS_READDIRBLKSIZ` block.
    blksiz: usize,
    /// `dp`: the offset of the last record begun, if any.
    dp: Option<usize>,
}

impl<'b> DirPack<'b> {
    /// A packing into `buf` from `pos` (what an earlier RPC packed stays).
    fn new(buf: &'b mut [u8], pos: usize) -> Self {
        Self {
            buf,
            pos,
            blksiz: 0,
            dp: None,
        }
    }

    /// `uiop->uio_resid`: the room left.
    fn resid(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// `dp->d_reclen += n` of the last record (`dp` is always set when the C does this: a
    /// block is left only after a record filled part of it).
    fn grow_last(&mut self, n: usize) {
        if let Some(dp) = self.dp {
            let r = u16::from_ne_bytes([self.buf[dp + ND_RECLEN], self.buf[dp + ND_RECLEN + 1]]);
            // `d_reclen` is a `u_int16_t`: the C's `+=` wraps there (it never gets near: at
            // most `NFS_READDIRBLKSIZ`).
            let r = r.wrapping_add(n as u16);
            self.buf[dp + ND_RECLEN..dp + ND_RECLEN + 2].copy_from_slice(&r.to_ne_bytes());
        }
    }

    /// `iov_base += n; iov_len -= n; uio_resid -= n`.
    fn skip(&mut self, n: usize) {
        self.pos += n.min(self.resid());
    }

    /// Begin the record of the entry `fileno` with a name of `len` bytes (at most
    /// `NFS_MAXNAMLEN`, the caller's bound): when the record does not fit in the current
    /// `NFS_READDIRBLKSIZ` block the last record grows to fill it, and `false` (the C's
    /// `bigenough = 0`) when the buffer has no room for it. Otherwise the header is written
    /// and the record's name slot answered by [`DirPack::name_slot`].
    fn begin(&mut self, fileno: u64, len: usize) -> bool {
        let tlen = dirent_recsize(len) + NFS_DIRENT_OVERHEAD;
        let left = READDIRBLKSIZ - self.blksiz;
        if tlen > left {
            self.grow_last(left);
            self.skip(left);
            self.blksiz = 0;
        }
        if tlen > self.resid() {
            return false;
        }
        let dp = self.pos;
        self.buf[dp + ND_FILENO..dp + ND_FILENO + 8].copy_from_slice(&fileno.to_ne_bytes());
        // `len` is at most `NFS_MAXNAMLEN` (255), a `u_int8_t`.
        self.buf[dp + ND_NAMLEN] = len as u8;
        // `tlen` is at most `NFS_READDIRBLKSIZ` (the record fits its block).
        self.buf[dp + ND_RECLEN..dp + ND_RECLEN + 2].copy_from_slice(&(tlen as u16).to_ne_bytes());
        self.buf[dp + ND_TYPE] = DT_UNKNOWN;
        self.dp = Some(dp);
        self.blksiz += tlen;
        if self.blksiz == READDIRBLKSIZ {
            self.blksiz = 0;
        }
        self.pos += tlen;
        true
    }

    /// The name slot of the record just begun (`len` bytes after its header).
    fn name_slot(&mut self, len: usize) -> &mut [u8] {
        let Some(dp) = self.dp else {
            kpanic(format_args!("nfs readdir: a name with no record"));
        };
        &mut self.buf[dp + NFS_DIRHDSIZ..dp + NFS_DIRHDSIZ + len]
    }

    /// The name of `len` bytes of the record just begun is in place: NUL-terminate it (the
    /// record's size leaves room for the NUL).
    fn end_name(&mut self, len: usize) {
        if let Some(dp) = self.dp {
            self.buf[dp + NFS_DIRHDSIZ + len] = 0;
        }
    }

    /// The name of the record just begun.
    fn name(&self) -> &[u8] {
        match self.dp {
            Some(dp) => {
                let len = usize::from(self.buf[dp + ND_NAMLEN]);
                &self.buf[dp + NFS_DIRHDSIZ..dp + NFS_DIRHDSIZ + len]
            }
            None => &[],
        }
    }

    /// `ndp->cookie[0] = c[0]; ndp->cookie[1] = c[1]` of the record just begun (raw XDR
    /// words).
    fn set_cookie(&mut self, c: [u32; 2]) {
        if let Some(dp) = self.dp {
            self.buf[dp + ND_COOKIE..dp + ND_COOKIE + 4].copy_from_slice(&c[0].to_ne_bytes());
            self.buf[dp + ND_COOKIE + 4..dp + ND_COOKIE + 8].copy_from_slice(&c[1].to_ne_bytes());
        }
    }

    /// `dp->d_type = t` of the record just begun.
    fn set_type(&mut self, t: u8) {
        if let Some(dp) = self.dp {
            self.buf[dp + ND_TYPE] = t;
        }
    }

    /// Fill the last record, iff any, out to a multiple of `NFS_READDIRBLKSIZ` by increasing
    /// its `d_reclen`.
    fn finish(&mut self) {
        if self.blksiz > 0 {
            let left = READDIRBLKSIZ - self.blksiz;
            self.grow_last(left);
            self.skip(left);
        }
    }
}

/// `*cred` of a credential the C dereferences: a real one (`NOCRED`/`FSCRED` panic, the
/// module's Deviations).
pub(crate) fn ucred(cred: Cred<'_>) -> &Ucred {
    match cred.ucred() {
        Some(c) => c,
        None => kpanic(format_args!(
            "nfs: {} credential",
            if cred.is_nocred() {
                "missing"
            } else if cred.is_fscred() {
                "kernel"
            } else {
                "NULL"
            }
        )),
    }
}

/// `vp->v_mount`: the mount of one of the client's vnodes (`panic(9)` for none, which no
/// operation is given).
pub(crate) fn vmount<G: ProcGlue>(vp: &Vnode<G>) -> &'static Mount<G> {
    match vp.mount() {
        Some(mp) => mp,
        None => kpanic(format_args!("nfs: vnode {vp:p} has no mount")),
    }
}

/// `VFSTONFS(vp->v_mount)`: the vnode's mount's client structure, a counted handle.
fn vfstonfs_vp<G: ProcGlue>(vp: &Vnode<G>) -> NfsMountRef<G> {
    vfstonfs(vmount(vp))
}

/// `vp->v_mount->mnt_flag & MNT_RDONLY`.
fn rdonly<G: ProcGlue>(vp: &Vnode<G>) -> bool {
    vmount(vp).flag() & MNT_RDONLY != 0
}

/// The reader of a reply (`info.nmi_md`, `info.nmi_dpos` once `nfs_request` read the RPC
/// header), `None` for no reply.
fn mrep(reply: Option<&Reply>) -> Result<Mrep<'_>, Errno> {
    match reply {
        Some(r) => Ok(Some(r.reader()?)),
        None => Ok(None),
    }
}

/// The reader of a reply `nfs_request` answered with success (which always carries one: the
/// C's `nmi_mrep`, never NULL there).
fn rd<'r, 'a>(x: &'r mut Mrep<'a>) -> &'r mut XdrReader<'a> {
    match x {
        Some(x) => x,
        None => kpanic(format_args!("nfs: a successful RPC without its reply")),
    }
}

/// `nfsm_fhtom(&mb, vp, v3)`: the node's file handle into the request.
fn fhtom<G: ProcGlue>(w: &mut XdrWriter, np: &NfsNode<G>, v3: bool) {
    let fh = np.fh();
    w.fhtom(&fh.fh, usize::try_from(fh.size).unwrap_or(0), v3);
}

/// The thread a `VOP_GETATTR`/`VOP_SETATTR` runs for where the C passes a pointer that may
/// be NULL: `p`, else `curproc`, else `proc0` (the module's Deviations).
pub(crate) fn with_thread<G: ProcGlue, R>(
    p: Option<&Thread<G>>,
    f: impl FnOnce(&Thread<G>) -> R,
) -> R {
    match p {
        Some(p) => f(p),
        None => G::with_curproc(|cur| match cur {
            Some(cur) => f(cur),
            None => f(&G::tables().proc0()),
        }),
    }
}

/// Whether two credentials send the same `AUTH_UNIX` ids (`nfsm_rpchead`'s: the user, the
/// group, the groups).
fn same_ids(a: &Ucred, b: &Ucred) -> bool {
    ptr::eq(a, b)
        || (a.cr_uid.get() == b.cr_uid.get()
            && a.cr_gid.get() == b.cr_gid.get()
            && a.groups().len() == b.groups().len()
            && a.groups()
                .iter()
                .zip(b.groups())
                .all(|(x, y)| x.get() == y.get()))
}

/// `if (np->n_rcred) crfree(np->n_rcred); np->n_rcred = cred; crhold(cred);` (and the same of
/// `n_wcred`, `write`): the node's read or write credentials become `cred`, left alone when
/// the cell holds credentials that send the same ids already (the module's Redesign).
pub fn nfs_setcred<G: ProcGlue>(np: &NfsNode<G>, cred: &Ucred, write: bool) {
    {
        let r = klock::reader();
        let k = np.n_k.get(&r);
        let slot = if write { &k.n_wcred } else { &k.n_rcred };
        if slot.as_deref().is_some_and(|c| same_ids(c, cred)) {
            return;
        }
    }
    // `crhold`: a placed credential is counted again, a caller's own copy duplicated (a
    // sleeping allocation, outside the cell's section).
    let c = CredRef::from_ref(cred).unwrap_or_else(|| crdup(cred));
    let old = {
        let mut w = klock::witness();
        let k = np.n_k.get_mut(&mut w);
        let slot = if write {
            &mut k.n_wcred
        } else {
            &mut k.n_rcred
        };
        slot.replace(c)
    };
    // `crfree` of the old one, outside the section.
    drop(old);
}

/// `VTONFS(vp)->n_rcred` (`n_wcred` for `write`) for a request: one more reference, held
/// across the RPC (`panic(9)` for none, the module's Deviations).
fn nfs_getcred<G: ProcGlue>(np: &NfsNode<G>, write: bool) -> CredRef {
    let c = {
        let r = klock::reader();
        let k = np.n_k.get(&r);
        if write {
            k.n_wcred.clone()
        } else {
            k.n_rcred.clone()
        }
    };
    match c {
        Some(c) => c,
        None => kpanic(format_args!(
            "nfs: node {np:p} has no {} credential",
            if write { "write" } else { "read" }
        )),
    }
}

/// `nfs_cache_enter(dvp, vp, cnp)`: enter the name `name` in the cache, remembering in the
/// node the change time (a positive entry, `vp`'s node `np`) or in the directory's node `dnp`
/// its modification time (a negative entry, the first one since it changed).
pub fn nfs_cache_enter<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    dnp: &NfsNode<G>,
    np: Option<&NfsNode<G>>,
    name: &[u8],
) {
    match np {
        Some(np) => {
            let ctime = np.n_vattr.read().va_ctime.tv_sec;
            np.n_ctime.store(ctime, Ordering::Relaxed);
        }
        None => {
            if dnp.n_ctime.load(Ordering::Relaxed) == 0 {
                let mtime = dnp.n_vattr.read().va_mtime.tv_sec;
                dnp.n_ctime.store(mtime, Ordering::Relaxed);
            }
        }
    }

    cache_enter(dvp, np.map(nfstov), name);
}

/// The `NFSV3ACCESS_*` bits an access check of `a_mode` (`VREAD`, `VWRITE`, `VEXEC`) asks
/// of a vnode of type `vtype`.
fn nfs_access_mode(vtype: Vtype, a_mode: i32) -> u32 {
    let mut mode = if a_mode & VREAD != 0 {
        NFSV3ACCESS_READ
    } else {
        0
    };
    if vtype == VDIR {
        if a_mode & VWRITE != 0 {
            mode |= NFSV3ACCESS_MODIFY | NFSV3ACCESS_EXTEND | NFSV3ACCESS_DELETE;
        }
        if a_mode & VEXEC != 0 {
            mode |= NFSV3ACCESS_LOOKUP;
        }
    } else {
        if a_mode & VWRITE != 0 {
            mode |= NFSV3ACCESS_MODIFY | NFSV3ACCESS_EXTEND;
        }
        if a_mode & VEXEC != 0 {
            mode |= NFSV3ACCESS_EXECUTE;
        }
    }
    mode
}

/// `nfs_access` (`vop_access`): for nfs version 2, just return ok (`nfsspec_access`'s
/// imitation of the local check). File accesses may fail later. For nfs version 3, use the
/// access rpc to check accessibility. If file modes are changed on the server, accesses
/// might still fail later. `np` and `nmp` are the vnode's node and mount, lent.
pub fn nfs_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    a_mode: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let v3 = nfs_isv3(nmp);

    // Disallow write attempts on filesystems mounted read-only; unless the file is a socket,
    // fifo, or a block or character device resident on the filesystem.
    if a_mode & VWRITE != 0 && rdonly(vp) {
        match vp.vtype() {
            VREG | VDIR | VLNK => return Err(Errno::EROFS),
            _ => {}
        }
    }

    // Check access cache first. If a request has been made for this uid shortly before, use
    // the cached result.
    let cr = ucred(cred);
    let accstamp = np.n_accstamp.load(Ordering::Relaxed);
    let cachevalid = accstamp != -1
        && gettime().wrapping_sub(accstamp) < i64::from(nfs_attrtimeo(np, nmp))
        && np.n_accuid.load(Ordering::Relaxed) == cr.cr_uid.get();

    if cachevalid {
        let accmode = np.n_accmode.load(Ordering::Relaxed);
        let accerror = np.n_accerror.load(Ordering::Relaxed);
        if accerror == 0 {
            if accmode & a_mode == a_mode {
                return Ok(());
            }
        } else if accmode & a_mode == accmode {
            return Err(Errno::from_raw(accerror).unwrap_or(Errno::EACCES));
        }
    }

    // For nfs v3, do an access rpc, otherwise you are stuck emulating ufs_access() locally
    // using the vattr. This may not be correct, since the server may apply other access
    // criteria such as client uid-->server uid mapping that we do not know about, but this
    // is better than just returning anything that is lying about in the cache.
    if !v3 {
        return nfsspec_access(vp, a_mode, cred, p);
    }

    NFSSTATS.rpccnt[NFSPROC_ACCESS].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3) + NFSX_UNSIGNED);
    fhtom(&mut w, np, v3);
    let mode = nfs_access_mode(vp.vtype(), a_mode);
    w.u32(mode);

    let (reply, res) = nfs_request(
        nmp,
        vp,
        NFSPROC_ACCESS,
        Request {
            body: w,
            procp: Some(p),
            cred: cr,
        },
    );
    let mut x = mrep(reply.as_ref())?;
    let mut attrflag = 0;
    nfsm_postop_attr(&mut x, np, v3, &mut attrflag)?;
    let error = if let Err(e) = res {
        return Err(e.errno());
    } else {
        let rmode = rd(&mut x).u32()?;
        // The NFS V3 spec does not clarify whether or not the returned access bits can be
        // a superset of the ones requested, so...
        if rmode & mode == mode {
            None
        } else {
            Some(Errno::EACCES)
        }
    };
    drop(reply);

    // If we got the same result as for a previous, different request, OR it in. Don't
    // update the timestamp in that case.
    let errno = error.map_or(0, |e| e as i32);
    if cachevalid
        && np.n_accstamp.load(Ordering::Relaxed) != -1
        && errno == np.n_accerror.load(Ordering::Relaxed)
    {
        if errno == 0 {
            np.n_accmode.fetch_or(a_mode, Ordering::Relaxed);
        } else if np.n_accmode.load(Ordering::Relaxed) & a_mode == a_mode {
            np.n_accmode.store(a_mode, Ordering::Relaxed);
        }
    } else {
        np.n_accstamp.store(gettime(), Ordering::Relaxed);
        np.n_accuid.store(cr.cr_uid.get(), Ordering::Relaxed);
        np.n_accmode.store(a_mode, Ordering::Relaxed);
        np.n_accerror.store(errno, Ordering::Relaxed);
    }
    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// `nfs_open` (`vop_open`): check to see if the type is ok and that deletion is not in
/// progress. For paged in text files, you will need to flush the page cache if consistency
/// is lost.
pub fn nfs_open<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let vtype = vp.vtype();
    if vtype != VREG && vtype != VDIR && vtype != VLNK {
        #[cfg(feature = "diagnostic")]
        hal::prf::printf(format_args!("open eacces vtyp={}\n", vtype as i32));
        return Err(Errno::EACCES);
    }
    let np = vtonfs(vp);
    let nmp = vfstonfs_vp(vp);

    // Initialize read and write creds here, for swapfiles and other paths that don't set the
    // creds themselves.
    if mode & FREAD != 0 {
        nfs_setcred(&np, ucred(cred), false);
    }
    if mode & FWRITE != 0 {
        nfs_setcred(&np, ucred(cred), true);
    }

    let mut vattr = Vattr::new();
    if np.isset(NMODIFIED) {
        let error = nfs_vinvalbuf(&np, &nmp, V_SAVE, cred, Some(p));
        if error == Err(Errno::EINTR) {
            return error;
        }
        uvm_vnp_uncache(vp);
        nfs_invalidate_attrcache(&np);
        if vtype == VDIR {
            np.n_direofoffset.store(0, Ordering::Relaxed);
        }
        nfs_getattr_np(vp, &np, &nmp, &mut vattr, cred, p)?;
        let _ = np.n_mtime.replace(vattr.va_mtime);
    } else {
        nfs_getattr_np(vp, &np, &nmp, &mut vattr, cred, p)?;
        if *np.n_mtime.read() != vattr.va_mtime {
            if vtype == VDIR {
                np.n_direofoffset.store(0, Ordering::Relaxed);
            }
            let error = nfs_vinvalbuf(&np, &nmp, V_SAVE, cred, Some(p));
            if error == Err(Errno::EINTR) {
                return error;
            }
            uvm_vnp_uncache(vp);
            let _ = np.n_mtime.replace(vattr.va_mtime);
        }
    }
    // For open/close consistency.
    nfs_invalidate_attrcache(&np);
    Ok(())
}

/// `nfs_close` (`vop_close`): what an NFS client should do upon close after writing is a
/// debatable issue. Most NFS clients push delayed writes to the server upon close, basically
/// for two reasons: so that any write errors may be reported back to the client process
/// doing the close system call (by far the two most likely errors are `NFSERR_NOSPC` and
/// `NFSERR_DQUOT`), and to put a worst case upper bound on cache inconsistency between
/// multiple clients for the file. For NFS Version 2: play it safe and flush/invalidate all
/// dirty buffers; for NFS Version 3: flush dirty buffers to the server but don't invalidate
/// or commit them (a server crash after this close but before the commit RPC is felt to be
/// "good enough").
pub fn nfs_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    if vp.vtype() != VREG {
        return Ok(());
    }
    let np = vtonfs(vp);
    let mut error = Ok(());
    if np.isset(NMODIFIED) {
        let nmp = vfstonfs_vp(vp);
        if nfs_isv3(&nmp) {
            error = nfs_flush(vp, &np, &nmp, cred, MNT_WAIT, p, false);
            np.clr(NMODIFIED);
        } else {
            error = nfs_vinvalbuf(&np, &nmp, V_SAVE, cred, p);
        }
        nfs_invalidate_attrcache(&np);
    }
    if np.isset(NWRITEERR) {
        np.clr(NWRITEERR);
        error = errno_of(np.n_error.load(Ordering::Relaxed));
    }
    error
}

/// The answer of a saved error number (`n_error`, 0 for none).
pub(crate) fn errno_of(e: i32) -> Result<(), Errno> {
    if e == 0 {
        Ok(())
    } else {
        Err(Errno::from_raw(e).unwrap_or(Errno::EIO))
    }
}

/// `nfsm_loadattr(infop, &vp, vap)`: the `nfs_fattr` at the reader into the node's cache (and
/// `vap`), the caller's error made the dissection's (`nfs_loadattrcache`'s).
fn nfsm_loadattr<G: ProcGlue>(
    x: &mut XdrReader<'_>,
    np: &NfsNode<G>,
    v3: bool,
    vap: Option<&mut Vattr>,
) -> Result<(), Errno> {
    nfs_loadattrcache(np, v3, x, vap).map(|_| ())
}

/// `nfs_getattr` (`vop_getattr`): nfs getattr call from vfs.
pub fn nfs_getattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let np = vtonfs(vp);
    let nmp = vfstonfs_vp(vp);
    nfs_getattr_np(vp, &np, &nmp, vap, cred, p)
}

/// [`nfs_getattr`] of a vnode whose node and mount the caller holds (an operation's own
/// `VOP_GETATTR` of its vnode, the C's call through the table to the same function).
pub(crate) fn nfs_getattr_np<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let v3 = nfs_isv3(nmp);

    // Update local times for special files.
    if np.isset(NACC | NUPD) {
        np.set(NCHG);
    }
    // First look in the cache.
    if nfs_getattrcache(np, nmp, vap).is_ok() {
        return Ok(());
    }

    NFSSTATS.rpccnt[NFSPROC_GETATTR].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3));
    fhtom(&mut w, np, v3);
    let (reply, res) = nfs_request(
        nmp,
        vp,
        NFSPROC_GETATTR,
        Request {
            body: w,
            procp: Some(p),
            cred: ucred(cred),
        },
    );
    res.map_err(NfsError::errno)?;
    let mut x = mrep(reply.as_ref())?;
    nfsm_loadattr(rd(&mut x), np, v3, Some(vap))
}

/// `nfs_setattr` (`vop_setattr`): nfs setattr call.
pub fn nfs_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut hint = NOTE_ATTRIB;
    let mut tsize: u64 = 0;

    // Setting of flags is not supported.
    if vap.va_flags != VNOVAL as u64 {
        return Err(Errno::EOPNOTSUPP);
    }

    // Disallow write attempts if the filesystem is mounted read-only.
    let some_attr = vap.va_uid != VNOVAL as u32
        || vap.va_gid != VNOVAL as u32
        || vap.va_atime.tv_nsec != i64::from(VNOVAL)
        || vap.va_mtime.tv_nsec != i64::from(VNOVAL)
        || vap.va_mode != VNOVAL as u32;
    if some_attr && rdonly(vp) {
        return Err(Errno::EROFS);
    }
    let np = vtonfs(vp);
    let nmp = vfstonfs_vp(vp);
    if vap.va_size != VNOVAL as u64 {
        match vp.vtype() {
            VDIR => return Err(Errno::EISDIR),
            VCHR | VBLK | VSOCK | VFIFO => {
                if vap.va_mtime.tv_nsec == i64::from(VNOVAL)
                    && vap.va_atime.tv_nsec == i64::from(VNOVAL)
                    && vap.va_mode == VNOVAL as u32
                    && vap.va_uid == VNOVAL as u32
                    && vap.va_gid == VNOVAL as u32
                {
                    return Ok(());
                }
                vap.va_size = VNOVAL as u64;
            }
            _ => {
                // Disallow write attempts if the filesystem is mounted read-only.
                if rdonly(vp) {
                    return Err(Errno::EROFS);
                }
                let flags = if vap.va_size == 0 { 0 } else { V_SAVE };
                nfs_vinvalbuf(&np, &nmp, flags, cred, Some(p))?;
                tsize = np.n_size.load(Ordering::Relaxed);
                np.n_size.store(vap.va_size, Ordering::Relaxed);
                let size = vap.va_size;
                np.n_vattr.update(|v| v.va_size = size);
                // The C passes its `u_quad_t` size as the `voff_t` argument.
                uvm_vnp_setsize(vp, size as i64);
            }
        }
    } else if (vap.va_mtime.tv_nsec != i64::from(VNOVAL)
        || vap.va_atime.tv_nsec != i64::from(VNOVAL))
        && vp.vtype() == VREG
    {
        let error = nfs_vinvalbuf(&np, &nmp, V_SAVE, cred, Some(p));
        if error == Err(Errno::EINTR) {
            return error;
        }
    }
    let error = nfs_setattrrpc(vp, &np, &nmp, vap, ucred(cred), Some(p));
    if error.is_err() && vap.va_size != VNOVAL as u64 {
        np.n_size.store(tsize, Ordering::Relaxed);
        np.n_vattr.update(|v| v.va_size = tsize);
        uvm_vnp_setsize(vp, tsize as i64);
    }

    if vap.va_size != VNOVAL as u64 && vap.va_size < tsize {
        hint |= NOTE_TRUNCATE;
    }

    VN_KNOTE(vp, hint); // XXX setattrrpc?

    error.map_err(NfsError::errno)
}

/// `nfsm_wcc_data(infop, &vp, flagp)`: a version 3 `wcc_data` (the pre-op attributes, then
/// the post-op ones into `np`'s cache). With `*flagp == NFSV3_WCCRATTR` the post-op attribute
/// flag comes back in it; otherwise whether the pre-op mtime differed from the node's
/// `n_mtime`. Nothing (the flag left) without a reply.
fn nfsm_wcc_data<G: ProcGlue>(
    x: &mut Mrep<'_>,
    np: &NfsNode<G>,
    flagp: &mut i32,
) -> Result<(), Errno> {
    let Some(r) = x.as_mut() else {
        return Ok(());
    };
    let mut ttretf = 0;
    if r.word()? == nfs_true {
        let tl = r.get::<{ 6 * NFSX_UNSIGNED }>()?;
        let mtime = super::xdr_subs::fxdr_nfsv3time(&super::nfsproto::Nfsv3Time {
            nfsv3_sec: u32::from_ne_bytes([tl[8], tl[9], tl[10], tl[11]]),
            nfsv3_nsec: u32::from_ne_bytes([tl[12], tl[13], tl[14], tl[15]]),
        });
        if *flagp != NFSV3_WCCRATTR {
            ttretf = i32::from(*np.n_mtime.read() != mtime);
        }
    }
    let mut ttattrf = 0;
    nfsm_postop_attr(x, np, true, &mut ttattrf)?;
    *flagp = if *flagp == NFSV3_WCCRATTR {
        ttattrf
    } else {
        ttretf
    };
    Ok(())
}

/// `nfs_setattrrpc(vp, vap, cred, procp)`: do an nfs setattr rpc. `np` and `nmp` are the
/// vnode's node and mount, lent.
pub fn nfs_setattrrpc<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    vap: &Vattr,
    cred: &Ucred,
    procp: Option<&Thread<G>>,
) -> Result<(), NfsError> {
    let v3 = nfs_isv3(nmp);
    let mut wccflag = NFSV3_WCCRATTR;

    NFSSTATS.rpccnt[NFSPROC_SETATTR].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3) + nfsx_sattr(v3));
    fhtom(&mut w, np, v3);

    if v3 {
        w.v3attrbuild(vap, true);
        w.word(nfs_false);
    } else {
        let sp = Nfsv2Sattr {
            sa_mode: if vap.va_mode == VNOVAL as u32 {
                nfs_xdrneg1
            } else {
                vtonfsv2_mode(vp.vtype(), vap.va_mode)
            },
            sa_uid: if vap.va_uid == VNOVAL as u32 {
                nfs_xdrneg1
            } else {
                txdr_unsigned(vap.va_uid)
            },
            sa_gid: if vap.va_gid == VNOVAL as u32 {
                nfs_xdrneg1
            } else {
                txdr_unsigned(vap.va_gid)
            },
            // `txdr_unsigned(vap->va_size)`: the low 32 bits.
            sa_size: txdr_unsigned(vap.va_size as u32),
            sa_atime: txdr_nfsv2time(&vap.va_atime),
            sa_mtime: txdr_nfsv2time(&vap.va_mtime),
        };
        w.pod(&sp);
    }

    let (reply, res) = nfs_request(
        nmp,
        vp,
        NFSPROC_SETATTR,
        Request {
            body: w,
            procp,
            cred,
        },
    );
    let mut x = mrep(reply.as_ref())?;
    if v3 {
        nfsm_wcc_data(&mut x, np, &mut wccflag)?;
    } else if res.is_ok() {
        nfsm_loadattr(rd(&mut x), np, v3, None)?;
    }
    res
}

/// `nfsm_getfh(infop, &size, v3)`: a file handle of the reply (a counted string of at most
/// `NFSX_V3FHMAX` bytes in version 3, `EBADRPC` for a size `<= 0` or past it; the fixed
/// `NFSX_V2FH` bytes in version 2), then its padding: a copy.
fn nfsm_getfh(x: &mut XdrReader<'_>, v3: bool) -> Result<NfsFh, Errno> {
    let size = if v3 {
        let size = x.i32()?;
        if size <= 0 || size > NFSX_V3FHMAX as i32 {
            return Err(Errno::EBADRPC);
        }
        // `0 < size <= NFSX_V3FHMAX`: a size.
        size as usize
    } else {
        NFSX_V2FH
    };
    let mut b = [0u8; NFSX_V3FHMAX];
    x.bytes_into(&mut b[..size])?;
    x.adv(nfsm_padlen(size))?;
    match NfsFh::new(&b[..size]) {
        Some(fh) => Ok(fh),
        None => Err(Errno::EBADRPC),
    }
}

/// `vput(vp)` of a looked-up vnode, `vrele` when it is the directory itself.
fn vput_or_vrele<G: ProcGlue>(vp: &'static Vnode<G>, dvp: &'static Vnode<G>) {
    if ptr::eq(vp, dvp) {
        let _ = vrele(vp);
    } else {
        vput(vp);
    }
}

/// `nfs_lookup` (`vop_lookup`): nfs lookup call, one step at a time... First look in cache. If
/// not found, unlock the directory nfsnode and do the rpc.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function: the cache's three answers, the RPC, its four cases and the error tail"
)]
pub fn nfs_lookup<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    cnp.cn_flags &= !PDIRUNLOCK;
    let flags = cnp.cn_flags;

    *vpp = None;
    if flags & ISLASTCN != 0
        && rdonly(dvp)
        && (cnp.cn_nameiop == DELETE || cnp.cn_nameiop == RENAME)
    {
        return Err(Errno::EROFS);
    }
    if dvp.vtype() != VDIR {
        return Err(Errno::ENOTDIR);
    }
    let lockparent = flags & LOCKPARENT != 0;
    let wantparent = flags & (LOCKPARENT | WANTPARENT) != 0;
    let nmp = vfstonfs_vp(dvp);
    let np = vtonfs(dvp);
    let v3 = nfs_isv3(&nmp);
    let p = cnp.cn_proc;

    // Before tediously performing a linear scan of the directory, check the name cache to see
    // if the directory/name pair we are looking for is known already. If the directory/name
    // pair is found in the name cache, we have to ensure the directory has not changed from
    // the time the cache entry has been created. If it has, the cache entry has to be ignored.
    let cached = match cache_lookup(dvp, cnp) {
        Ok(None) => None,
        Ok(Some(vp)) => Some(Some(vp)),
        Err(Errno::ENOENT) => Some(None),
        Err(e) => return Err(e),
    };
    if let Some(hit) = cached {
        if cnp.cn_flags & PDIRUNLOCK != 0 {
            vn_lock(dvp, LK_EXCLUSIVE | LK_RETRY)?;
            cnp.cn_flags &= !PDIRUNLOCK;
        }

        if let Err(err2) = nfs_access(dvp, &np, &nmp, VEXEC, cnp.cn_cred, p) {
            if let Some(vp) = hit {
                vput_or_vrele(vp, dvp);
            }
            return Err(err2);
        }

        match hit {
            None => {
                let mut vattr = Vattr::new();
                if nfs_getattr_np(dvp, &np, &nmp, &mut vattr, cnp.cn_cred, p).is_ok()
                    && vattr.va_mtime.tv_sec == np.n_ctime.load(Ordering::Relaxed)
                {
                    return Err(Errno::ENOENT);
                }
                cache_purge(dvp);
                np.n_ctime.store(0, Ordering::Relaxed);
            }
            Some(newvp) => {
                let mut vattr = Vattr::new();
                // `VOP_GETATTR(newvp)`: `nfs_getattr` in each of the client's tables, the
                // node taken once and its `n_ctime` read from it.
                let nnp = vtonfs(newvp);
                if nfs_getattr_np(newvp, &nnp, &nmp, &mut vattr, cnp.cn_cred, p).is_ok()
                    && vattr.va_ctime.tv_sec == nnp.n_ctime.load(Ordering::Relaxed)
                {
                    NFSSTATS.lookupcache_hits.fetch_add(1, Ordering::Relaxed);
                    if cnp.cn_nameiop != LOOKUP && flags & ISLASTCN != 0 {
                        cnp.cn_flags |= SAVENAME;
                    }
                    if (!lockparent || flags & ISLASTCN == 0) && !ptr::eq(newvp, dvp) {
                        let _ = vop_unlock(dvp);
                        cnp.cn_flags |= PDIRUNLOCK;
                    }
                    *vpp = Some(newvp);
                    return Ok(());
                }
                cache_purge(newvp);
                vput_or_vrele(newvp, dvp);
            }
        }
    }

    // dorpc:
    let mut newnp: Option<NfsNodeRef<G>> = None;
    NFSSTATS.lookupcache_misses.fetch_add(1, Ordering::Relaxed);
    NFSSTATS.rpccnt[NFSPROC_LOOKUP].fetch_add(1, Ordering::Relaxed);
    let len = cnp.name().len();
    let mut w = XdrWriter::reqhead(nfsx_fh(v3) + NFSX_UNSIGNED + nfsm_rndup(len));
    fhtom(&mut w, &np, v3);
    let error: NfsError = 'nfsmout: {
        if let Err(e) = w.strtom(cnp.name(), NFS_MAXNAMLEN) {
            break 'nfsmout e.into();
        }
        let (reply, res) = nfs_request(
            &nmp,
            dvp,
            NFSPROC_LOOKUP,
            Request {
                body: w,
                procp: Some(p),
                cred: ucred(cnp.cn_cred),
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout e.into(),
        };
        let mut attrflag = 0;
        if let Err(e) = res {
            if v3 && let Err(e) = nfsm_postop_attr(&mut x, &np, v3, &mut attrflag) {
                break 'nfsmout e.into();
            }
            break 'nfsmout e;
        }

        let fh = match nfsm_getfh(rd(&mut x), v3) {
            Ok(fh) => fh,
            Err(e) => break 'nfsmout e.into(),
        };

        // The postop attr handling is duplicated for each case, because it should be done
        // while dvp is locked (unlocking dvp is different for each case).
        let postops = |x: &mut Mrep<'_>, nnp: &NfsNode<G>| -> Result<(), Errno> {
            let mut attrflag = 0;
            if v3 {
                nfsm_postop_attr(x, nnp, v3, &mut attrflag)?;
                nfsm_postop_attr(x, &np, v3, &mut attrflag)
            } else {
                nfsm_loadattr(rd(x), nnp, v3, None)
            }
        };

        // Handle RENAME case...
        if cnp.cn_nameiop == RENAME && wantparent && flags & ISLASTCN != 0 {
            if nfs_cmpfh(&np, fh.bytes()) {
                return Err(Errno::EISDIR);
            }
            let nnp = nfs_nget(vmount(dvp), fh.bytes())?;
            let r = postops(&mut x, &nnp);
            newnp = Some(nnp);
            if let Err(e) = r {
                break 'nfsmout e.into();
            }
            *vpp = newnp.as_ref().map(|n| nfstov(n));
            cnp.cn_flags |= SAVENAME;
            if !lockparent {
                let _ = vop_unlock(dvp);
                cnp.cn_flags |= PDIRUNLOCK;
            }
            return Ok(());
        }

        if nfs_cmpfh(&np, fh.bytes()) {
            vref(dvp);
            newnp = Some(np.clone());
            if let Err(e) = postops(&mut x, &np) {
                break 'nfsmout e.into();
            }
        } else if flags & ISDOTDOT != 0 {
            let _ = vop_unlock(dvp);
            cnp.cn_flags |= PDIRUNLOCK;

            let nnp = match nfs_nget(vmount(dvp), fh.bytes()) {
                Ok(n) => n,
                Err(e) => {
                    if vn_lock(dvp, LK_EXCLUSIVE | LK_RETRY).is_ok() {
                        cnp.cn_flags &= !PDIRUNLOCK;
                    }
                    return Err(e);
                }
            };
            let r = postops(&mut x, &nnp);
            newnp = Some(nnp);
            if let Err(e) = r {
                break 'nfsmout e.into();
            }

            if lockparent && flags & ISLASTCN != 0 {
                if let Err(e) = vn_lock(dvp, LK_EXCLUSIVE) {
                    if let Some(n) = newnp.take() {
                        vput(nfstov(&n));
                    }
                    return Err(e);
                }
                cnp.cn_flags &= !PDIRUNLOCK;
            }
        } else {
            let nnp = nfs_nget(vmount(dvp), fh.bytes())?;
            let r = postops(&mut x, &nnp);
            newnp = Some(nnp);
            if let Err(e) = r {
                break 'nfsmout e.into();
            }
            if !lockparent || flags & ISLASTCN == 0 {
                let _ = vop_unlock(dvp);
                cnp.cn_flags |= PDIRUNLOCK;
            }
        }
        drop(reply);

        if cnp.cn_nameiop != LOOKUP && flags & ISLASTCN != 0 {
            cnp.cn_flags |= SAVENAME;
        }
        if cnp.cn_flags & MAKEENTRY != 0 && (cnp.cn_nameiop != DELETE || flags & ISLASTCN == 0) {
            nfs_cache_enter(dvp, &np, newnp.as_deref(), cnp.name());
        }

        *vpp = newnp.as_ref().map(|n| nfstov(n));
        return Ok(());
    };

    // nfsmout: we get here only because of errors returned by the RPC. Otherwise we'd
    // already have returned.
    let mut error = error;
    if error == NfsError::Errno(Errno::ENOENT)
        && cnp.cn_flags & MAKEENTRY != 0
        && cnp.cn_nameiop != CREATE
    {
        nfs_cache_enter(dvp, &np, None, cnp.name());
    }
    if let Some(n) = newnp {
        vput_or_vrele(nfstov(&n), dvp);
    }
    if (cnp.cn_nameiop == CREATE || cnp.cn_nameiop == RENAME)
        && flags & ISLASTCN != 0
        && error == NfsError::Errno(Errno::ENOENT)
    {
        error = NfsError::Errno(if rdonly(dvp) {
            Errno::EROFS
        } else {
            Errno::EJUSTRETURN
        });
    }
    if cnp.cn_nameiop != LOOKUP && flags & ISLASTCN != 0 {
        cnp.cn_flags |= SAVENAME;
    }
    *vpp = None;
    Err(error.errno())
}

/// `nfs_read` (`vop_read`): nfs read call. Just call `nfs_bioread()` to do the work.
pub fn nfs_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    if vp.vtype() != VREG {
        return Err(Errno::EPERM);
    }
    nfs_bioread(vp, uio, ioflag, cred)
}

/// `nfs_readlink` (`vop_readlink`): nfs readlink call.
pub fn nfs_readlink<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    if vp.vtype() != VLNK {
        return Err(Errno::EPERM);
    }
    nfs_bioread(vp, uio, 0, cred)
}

/// `nfs_lock` (`vop_lock`): lock an inode.
pub fn nfs_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    vtonfs(vp).n_lock.enter(flags & LK_RWFLAGS)
}

/// `nfs_unlock` (`vop_unlock`): unlock an inode.
pub fn nfs_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    vtonfs(vp).n_lock.exit();
    Ok(())
}

/// `nfs_islocked` (`vop_islocked`): check for a locked inode.
pub fn nfs_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    vtonfs(vp).n_lock.status()
}

/// `nfs_readlinkrpc(vp, uiop, cred)`: do a readlink rpc. Called by `nfs_doio()` from below
/// the buffer cache (and by `nfs_bioread` for a root symbolic link, with the caller's uio).
/// `np` and `nmp` are the vnode's node and mount, lent.
pub fn nfs_readlinkrpc<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    uiop: &mut Uio<'_, G>,
    cred: &Ucred,
) -> Result<(), NfsError> {
    let v3 = nfs_isv3(nmp);

    NFSSTATS.rpccnt[NFSPROC_READLINK].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3));
    fhtom(&mut w, np, v3);

    let (reply, res) = G::with_curproc(|p| {
        nfs_request(
            nmp,
            vp,
            NFSPROC_READLINK,
            Request {
                body: w,
                procp: p,
                cred,
            },
        )
    });
    let mut x = mrep(reply.as_ref())?;
    if v3 {
        let mut attrflag = 0;
        nfsm_postop_attr(&mut x, np, v3, &mut attrflag)?;
    }
    res?;
    let x = rd(&mut x);
    let len = x.strsiz(NFS_MAXPATHLEN as i32)?;
    x.to_uio(uiop, len)?;
    Ok(())
}

/// `nfs_readrpc(vp, uiop)`: nfs read rpc call, `nm_rsize` bytes at most per RPC, until the
/// uio is full or the file ends (`EFBIG` for a version 2 read past 4 GB). `np` and `nmp` are
/// the vnode's node and mount, lent; the request's credential is the node's `n_rcred`.
pub fn nfs_readrpc<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    uiop: &mut Uio<'_, G>,
) -> Result<(), NfsError> {
    let v3 = nfs_isv3(nmp);
    // `int tsiz = uiop->uio_resid`: a buffer's size or a caller's bounded count.
    let mut tsiz = i32::try_from(uiop.uio_resid).unwrap_or(i32::MAX);
    if uiop.uio_offset.wrapping_add(i64::from(tsiz)) > 0xffff_ffff && !v3 {
        return Err(Errno::EFBIG.into());
    }
    G::with_curproc(|p| {
        while tsiz > 0 {
            NFSSTATS.rpccnt[NFSPROC_READ].fetch_add(1, Ordering::Relaxed);
            let rsize = nmp.nm_rsize.load(Ordering::Relaxed);
            let len = if tsiz > rsize { rsize } else { tsiz };
            let mut w = XdrWriter::reqhead(nfsx_fh(v3) + NFSX_UNSIGNED * 3);
            fhtom(&mut w, np, v3);
            if v3 {
                w.hyper(uiop.uio_offset as u64);
                w.u32(len as u32);
            } else {
                w.u32(uiop.uio_offset as u32);
                w.u32(len as u32);
                w.word(0);
            }

            let cred = nfs_getcred(np, false);
            let (reply, res) = nfs_request(
                nmp,
                vp,
                NFSPROC_READ,
                Request {
                    body: w,
                    procp: p,
                    cred: &cred,
                },
            );
            let mut x = mrep(reply.as_ref())?;
            if v3 {
                let mut attrflag = 0;
                nfsm_postop_attr(&mut x, np, v3, &mut attrflag)?;
            }
            res?;

            let r = rd(&mut x);
            let eof = if v3 {
                let tl = r.get::<{ 2 * NFSX_UNSIGNED }>()?;
                fxdr_unsigned(u32::from_ne_bytes([tl[4], tl[5], tl[6], tl[7]])) as i32
            } else {
                nfsm_loadattr(r, np, v3, None)?;
                0
            };

            let retlen = r.strsiz(rsize)?;
            r.to_uio(uiop, retlen)?;
            tsiz -= retlen;
            if v3 {
                if eof != 0 || retlen == 0 {
                    tsiz = 0;
                }
            } else if retlen < len {
                tsiz = 0;
            }
        }
        Ok(())
    })
}

/// `nfs_writerpc(vp, uiop, iomode, must_commit)`: nfs write call, `nm_wsize` bytes at most per
/// RPC. `iomode` is asked for and comes back as the lowest commitment level any RPC got;
/// `must_commit` is set when the server's write verifier changed. On an error `uio_resid` is
/// what was not written. `np` and `nmp` are the vnode's node and mount, lent; the request's
/// credential is the node's `n_wcred`.
pub fn nfs_writerpc<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    uiop: &mut Uio<'_, G>,
    iomode: &mut u32,
    must_commit: &mut bool,
) -> Result<(), NfsError> {
    let v3 = nfs_isv3(nmp);
    let mut committed = NFSV3WRITE_FILESYNC;

    #[cfg(feature = "diagnostic")]
    if uiop.uio_iovcnt() != 1 {
        kpanic(format_args!("nfs: writerpc iovcnt > 1"));
    }
    *must_commit = false;
    let mut tsiz = i32::try_from(uiop.uio_resid).unwrap_or(i32::MAX);
    if uiop.uio_offset.wrapping_add(i64::from(tsiz)) > 0xffff_ffff && !v3 {
        return Err(Errno::EFBIG.into());
    }
    let error = G::with_curproc(|p| -> Result<(), NfsError> {
        while tsiz > 0 {
            NFSSTATS.rpccnt[NFSPROC_WRITE].fetch_add(1, Ordering::Relaxed);
            let wsize = nmp.nm_wsize.load(Ordering::Relaxed);
            let mut len = if tsiz > wsize { wsize } else { tsiz };
            // `0 < len <= tsiz`, a count of the uio's bytes.
            let ulen = len as usize;
            let mut w = XdrWriter::reqhead(nfsx_fh(v3) + 5 * NFSX_UNSIGNED + nfsm_rndup(ulen));
            fhtom(&mut w, np, v3);
            if v3 {
                w.hyper(uiop.uio_offset as u64);
                w.u32(len as u32);
                w.u32(*iomode);
                w.u32(len as u32);
            } else {
                // Set both "begin" and "current" to non-garbage.
                let x = uiop.uio_offset as u32;
                w.u32(x); // "begin offset"
                w.u32(x); // "current offset"
                w.u32(len as u32); // total to this offset
                w.u32(len as u32); // size of this write
            }
            w.from_uio(uiop, ulen);

            let cred = nfs_getcred(np, true);
            let (reply, res) = nfs_request(
                nmp,
                vp,
                NFSPROC_WRITE,
                Request {
                    body: w,
                    procp: p,
                    cred: &cred,
                },
            );
            let mut x = mrep(reply.as_ref())?;
            let mut wccflag = NFSV3_WCCRATTR;
            if v3 {
                wccflag = NFSV3_WCCCHK;
                nfsm_wcc_data(&mut x, np, &mut wccflag)?;
            }
            res?;

            if v3 {
                wccflag = NFSV3_WCCCHK;
                let r = rd(&mut x);
                let rlen = r.i32()?;
                let commit = r.u32()?;
                let verf = r.get::<NFSX_V3WRITEVERF>()?;
                if rlen <= 0 {
                    // info.nmi_mrep free'd after the loop
                    return Err(Errno::from_raw(NFSERR_IO).unwrap_or(Errno::EIO).into());
                } else if rlen < len {
                    let backup = (len - rlen) as usize;
                    if let Some(iov) = uiop.uio_iov.first_mut() {
                        iov.iov_base = iov.iov_base.wrapping_byte_sub(backup);
                        iov.iov_len += backup;
                    }
                    uiop.uio_offset -= backup as i64;
                    uiop.uio_resid += backup;
                    len = rlen;
                }

                // Return the lowest commitment level obtained by any of the RPCs.
                if committed == NFSV3WRITE_FILESYNC
                    || (committed == NFSV3WRITE_DATASYNC && commit == NFSV3WRITE_UNSTABLE)
                {
                    committed = commit;
                }
                if nmp.flag() & NFSMNT_HASWRITEVERF == 0 {
                    nmp.set_verf(verf);
                    nmp.nm_flag.fetch_or(NFSMNT_HASWRITEVERF, Ordering::Relaxed);
                } else if verf != nmp.verf() {
                    *must_commit = true;
                    nmp.set_verf(verf);
                }
            } else {
                nfsm_loadattr(rd(&mut x), np, v3, None)?;
            }
            if wccflag != 0 {
                let mtime = np.n_vattr.read().va_mtime;
                let _ = np.n_mtime.replace(mtime);
            }
            tsiz -= len;
        }
        Ok(())
    });
    // nfsmout:
    *iomode = committed;
    if error.is_err() {
        uiop.uio_resid = usize::try_from(tsiz).unwrap_or(0);
    }
    error
}

/// `nfsm_mtofh(infop, dvp, &vp, &flag)`: the file handle a create reply may carry (version
/// 3's `post_op_fh3`, then its `post_op_attr`; version 2's always): the node it names
/// (`nfs_nget`, into `npp` even when what follows fails, for the caller's `vput`) and its
/// attributes loaded; `flagp` is whether a node came.
fn nfsm_mtofh<G: ProcGlue>(
    x: &mut XdrReader<'_>,
    dvp: &'static Vnode<G>,
    v3: bool,
    npp: &mut Option<NfsNodeRef<G>>,
    flagp: &mut i32,
) -> Result<(), Errno> {
    let mut flag = if v3 { x.i32()? } else { 1 };
    if flag != 0 {
        let fh = nfsm_getfh(x, v3)?;
        *npp = Some(nfs_nget(vmount(dvp), fh.bytes())?);
    }
    if v3 {
        let more = x.i32()?;
        if flag != 0 {
            flag = more;
        } else if more != 0 {
            x.adv(NFSX_V3FATTR)?;
        }
    }
    if flag != 0
        && let Some(np) = npp.as_deref()
    {
        nfsm_loadattr(x, np, v3, None)?;
    }
    *flagp = flag;
    Ok(())
}

/// The version 2 `sattr` of a node to create (`sa_mode` of `vtype` and `va_mode`, the owner
/// left to the server, `sa_size` the given raw word, the times of `vap`).
fn v2sattr_create(vtype: Vtype, vap: &Vattr, sa_size: u32) -> Nfsv2Sattr {
    Nfsv2Sattr {
        sa_mode: vtonfsv2_mode(vtype, vap.va_mode),
        sa_uid: nfs_xdrneg1,
        sa_gid: nfs_xdrneg1,
        sa_size,
        sa_atime: txdr_nfsv2time(&vap.va_atime),
        sa_mtime: txdr_nfsv2time(&vap.va_mtime),
    }
}

/// `nfs_mknodrpc(dvp, vpp, cnp, vap)`: nfs mknod rpc. For NFS v2 this is a kludge. Use a
/// create rpc but with the IFMT bits of the mode set to specify the file type and the size
/// field for rdev. The new vnode comes back locked in `vpp`; the pathname buffer is freed.
pub fn nfs_mknodrpc<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let rdev = match vap.va_type {
        VCHR | VBLK => txdr_unsigned(vap.va_rdev as u32),
        VFIFO | VSOCK => nfs_xdrneg1,
        _ => {
            let _ = vop_abortop(dvp, cnp);
            return Err(Errno::EOPNOTSUPP);
        }
    };
    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(dvp);
    let v3 = nfs_isv3(&nmp);
    let mut wccflag = NFSV3_WCCRATTR;
    let mut newnp: Option<NfsNodeRef<G>> = None;

    NFSSTATS.rpccnt[NFSPROC_MKNOD].fetch_add(1, Ordering::Relaxed);
    let len = cnp.name().len();
    let mut w =
        XdrWriter::reqhead(nfsx_fh(v3) + 4 * NFSX_UNSIGNED + nfsm_rndup(len) + nfsx_sattr(v3));
    fhtom(&mut w, &dnp, v3);
    let error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(cnp.name(), NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }

        if v3 {
            w.word(vtonfsv3_type(vap.va_type));
            w.v3attrbuild(vap, false);
            if vap.va_type == VCHR || vap.va_type == VBLK {
                w.u32(major(vap.va_rdev));
                w.u32(minor(vap.va_rdev));
            }
        } else {
            w.pod(&v2sattr_create(vap.va_type, vap, rdev));
        }

        assert_cn_curproc(cnp);
        let (reply, res) = nfs_request(
            &nmp,
            dvp,
            NFSPROC_MKNOD,
            Request {
                body: w,
                procp: Some(cnp.cn_proc),
                cred: ucred(cnp.cn_cred),
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };
        let mut error = res;
        if error.is_ok() {
            let mut gotvp = 0;
            if let Err(e) = nfsm_mtofh(rd(&mut x), dvp, v3, &mut newnp, &mut gotvp) {
                break 'nfsmout Err(e.into());
            }
            if gotvp == 0 {
                let mut np = None;
                error = nfs_lookitup(
                    dvp,
                    cnp.name(),
                    ucred(cnp.cn_cred),
                    Some(cnp.cn_proc),
                    Npp::Alloc(&mut np),
                );
                if error.is_ok() {
                    newnp = np;
                }
            }
        }
        if v3 && let Err(e) = nfsm_wcc_data(&mut x, &dnp, &mut wccflag) {
            break 'nfsmout Err(e.into());
        }
        error
    };

    // nfsmout:
    if error.is_err() {
        if let Some(n) = newnp.take() {
            vput(nfstov(&n));
        }
    } else {
        if cnp.cn_flags & MAKEENTRY != 0 {
            nfs_cache_enter(dvp, &dnp, newnp.as_deref(), cnp.name());
        }
        *vpp = newnp.as_ref().map(|n| nfstov(n));
    }
    drop(cnp.take_pnbuf());
    dnp.set(NMODIFIED);
    if wccflag == 0 {
        nfs_invalidate_attrcache(&dnp);
    }
    error.map_err(NfsError::errno)
}

/// `KASSERT(cnp->cn_proc == curproc)`.
fn assert_cn_curproc<G: ProcGlue>(cnp: &Cn<'_, G>) {
    G::with_curproc(|p| {
        kassert!(p.is_some_and(|p| ptr::eq(p, cnp.cn_proc)));
    });
}

/// `nfs_mknod` (`vop_mknod`): nfs mknod vop, just call `nfs_mknodrpc()` to do the work.
pub fn nfs_mknod<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let mut newvp = None;
    let error = nfs_mknodrpc(dvp, &mut newvp, cnp, vap);
    if error.is_ok()
        && let Some(vp) = newvp
    {
        vput(vp);
    }

    VN_KNOTE(dvp, NOTE_WRITE);

    error
}

/// `nfs_create` (`vop_create`): an exclusive create (`VA_EXCLUSIVE`) is a version 3
/// `EXCLUSIVE` create with a random verifier, retried unchecked when the server does not
/// support it (`NFSERR_NOTSUPP`), and its times set by a `SETATTR` after; a socket is a
/// `MKNOD`.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function: the request, the retry and the error tail"
)]
pub fn nfs_create<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    // Oops, not for me..
    if vap.va_type == VSOCK {
        return nfs_mknodrpc(dvp, vpp, cnp, vap);
    }

    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(dvp);
    let v3 = nfs_isv3(&nmp);
    let mut wccflag = NFSV3_WCCRATTR;
    let mut fmode = 0;
    let mut newnp: Option<NfsNodeRef<G>>;

    if vap.va_vaflags & VA_EXCLUSIVE != 0 {
        fmode |= O_EXCL;
    }

    let mut error: Result<(), NfsError> = loop {
        // again:
        newnp = None;
        NFSSTATS.rpccnt[NFSPROC_CREATE].fetch_add(1, Ordering::Relaxed);
        let len = cnp.name().len();
        let mut w =
            XdrWriter::reqhead(nfsx_fh(v3) + 2 * NFSX_UNSIGNED + nfsm_rndup(len) + nfsx_sattr(v3));
        fhtom(&mut w, &dnp, v3);
        let error: Result<(), NfsError> = 'nfsmout: {
            if let Err(e) = w.strtom(cnp.name(), NFS_MAXNAMLEN) {
                break 'nfsmout Err(e.into());
            }
            if v3 {
                if fmode & O_EXCL != 0 {
                    w.u32(NFSV3CREATE_EXCLUSIVE);
                    arc4random_buf(w.build::<8>());
                } else {
                    w.u32(NFSV3CREATE_UNCHECKED);
                    w.v3attrbuild(vap, false);
                }
            } else {
                w.pod(&v2sattr_create(vap.va_type, vap, 0));
            }

            assert_cn_curproc(cnp);
            let (reply, res) = nfs_request(
                &nmp,
                dvp,
                NFSPROC_CREATE,
                Request {
                    body: w,
                    procp: Some(cnp.cn_proc),
                    cred: ucred(cnp.cn_cred),
                },
            );
            let mut x = match mrep(reply.as_ref()) {
                Ok(x) => x,
                Err(e) => break 'nfsmout Err(e.into()),
            };
            let mut error = res;
            if error.is_ok() {
                let mut gotvp = 0;
                if let Err(e) = nfsm_mtofh(rd(&mut x), dvp, v3, &mut newnp, &mut gotvp) {
                    break 'nfsmout Err(e.into());
                }
                if gotvp == 0 {
                    let mut np = None;
                    error = nfs_lookitup(
                        dvp,
                        cnp.name(),
                        ucred(cnp.cn_cred),
                        Some(cnp.cn_proc),
                        Npp::Alloc(&mut np),
                    );
                    if error.is_ok() {
                        newnp = np;
                    }
                }
            }
            if v3 && let Err(e) = nfsm_wcc_data(&mut x, &dnp, &mut wccflag) {
                break 'nfsmout Err(e.into());
            }
            error
        };

        // nfsmout:
        match error {
            Err(e) => {
                if let Some(n) = newnp.take() {
                    vput(nfstov(&n));
                }
                if v3 && fmode & O_EXCL != 0 && e.raw() == NFSERR_NOTSUPP {
                    fmode &= !O_EXCL;
                    continue;
                }
                break Err(e);
            }
            Ok(()) => break Ok(()),
        }
    };
    if error.is_ok() && v3 && fmode & O_EXCL != 0 {
        let ts = getnanotime();
        if vap.va_atime.tv_nsec == i64::from(VNOVAL) {
            vap.va_atime = ts;
        }
        if vap.va_mtime.tv_nsec == i64::from(VNOVAL) {
            vap.va_mtime = ts;
        }
        error = match newnp.as_deref() {
            Some(np) => nfs_setattrrpc(
                nfstov(np),
                np,
                &nmp,
                vap,
                ucred(cnp.cn_cred),
                Some(cnp.cn_proc),
            ),
            None => kpanic(format_args!("nfs_create: created without a node")),
        };
    }
    if error.is_ok() {
        if cnp.cn_flags & MAKEENTRY != 0 {
            nfs_cache_enter(dvp, &dnp, newnp.as_deref(), cnp.name());
        }
        *vpp = newnp.as_ref().map(|n| nfstov(n));
    }
    drop(cnp.take_pnbuf());
    dnp.set(NMODIFIED);
    if wccflag == 0 {
        nfs_invalidate_attrcache(&dnp);
    }
    VN_KNOTE(dvp, NOTE_WRITE);
    error.map_err(NfsError::errno)
}

/// `nfs_remove` (`vop_remove`): nfs file remove call. To try and make nfs semantics closer to
/// ufs semantics, a file that has other processes using the vnode is renamed instead of
/// removed and then removed later on the last close: if `v_usecount > 1` and a rename is not
/// already in the works, `nfs_sillyrename()` sets it up, else the remove rpc is done.
pub fn nfs_remove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    {
        if cnp.cn_flags & crate::fs::namei::HASBUF == 0 {
            kpanic(format_args!("nfs_remove: no name"));
        }
        if vp.usecount() < 1 {
            kpanic(format_args!("nfs_remove: bad v_usecount"));
        }
    }
    let np = vtonfs(vp);
    let silly = || {
        let r = klock::reader();
        np.n_k.get(&r).n_sillyrename.is_some()
    };
    let error = if vp.vtype() == VDIR {
        Err(Errno::EPERM)
    } else if vp.usecount() == 1
        || (silly() && {
            let mut vattr = Vattr::new();
            vop_getattr(vp, &mut vattr, cnp.cn_cred, cnp.cn_proc).is_ok() && vattr.va_nlink > 1
        })
    {
        // Purge the name cache so that the chance of a lookup for the name succeeding while
        // the remove is in progress is minimized. Without node locking it can still happen,
        // such that an I/O op returns ESTALE, but since you get this if another host removes
        // the file..
        cache_purge(vp);
        // throw away biocache buffers, mainly to avoid unnecessary delayed writes later.
        let nmp = vfstonfs_vp(vp);
        let mut error =
            nfs_vinvalbuf(&np, &nmp, 0, cnp.cn_cred, Some(cnp.cn_proc)).map_err(NfsError::from);
        // Do the rpc
        if error != Err(NfsError::Errno(Errno::EINTR)) {
            error = nfs_removerpc(dvp, cnp.name(), ucred(cnp.cn_cred), Some(cnp.cn_proc));
        }
        // Kludge City: If the first reply to the remove rpc is lost.. the reply to the
        // retransmitted request will be ENOENT since the file was in fact removed. Therefore,
        // we cheat and return success.
        match error {
            Err(NfsError::Errno(Errno::ENOENT)) => Ok(()),
            e => e.map_err(NfsError::errno),
        }
    } else if !silly() {
        nfs_sillyrename(dvp, vp, &np, cnp)
    } else {
        Ok(())
    };
    drop(cnp.take_pnbuf());
    nfs_invalidate_attrcache(&np);
    VN_KNOTE(vp, NOTE_DELETE);
    VN_KNOTE(dvp, NOTE_WRITE);
    error
}

/// `nfs_removeit(sp)`: nfs file remove rpc called from `nfs_inactive`, of the silly name in
/// its directory (locked by the caller).
pub fn nfs_removeit<G: ProcGlue>(sp: &Sillyrename<G>) -> Result<(), NfsError> {
    kassert!(vop_islocked(sp.s_dvp) != 0);
    // Make sure that the directory vnode is still valid.
    //
    // NFS can potentially try to nuke a silly *after* the directory has already been pushed
    // out on a forced unmount. Since the silly is going to go away anyway, this is fine.
    if sp.s_dvp.vtype() == VBAD {
        return Ok(());
    }
    let n = usize::try_from(sp.s_namlen)
        .unwrap_or(0)
        .min(sp.s_name.len());
    nfs_removerpc(sp.s_dvp, &sp.s_name[..n], &sp.s_cred, None)
}

/// `nfs_removerpc(dvp, name, namelen, cred, proc)`: nfs remove rpc, called from
/// `nfs_remove()` and `nfs_removeit()`.
pub fn nfs_removerpc<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    name: &[u8],
    cred: &Ucred,
    proc: Option<&Thread<G>>,
) -> Result<(), NfsError> {
    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(dvp);
    let v3 = nfs_isv3(&nmp);
    let mut wccflag = NFSV3_WCCRATTR;

    NFSSTATS.rpccnt[NFSPROC_REMOVE].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3) + NFSX_UNSIGNED + nfsm_rndup(name.len()));
    fhtom(&mut w, &dnp, v3);
    let error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(name, NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }
        let (reply, res) = nfs_request(
            &nmp,
            dvp,
            NFSPROC_REMOVE,
            Request {
                body: w,
                procp: proc,
                cred,
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };
        if v3 && let Err(e) = nfsm_wcc_data(&mut x, &dnp, &mut wccflag) {
            break 'nfsmout Err(e.into());
        }
        res
    };

    // nfsmout:
    dnp.set(NMODIFIED);
    if wccflag == 0 {
        nfs_invalidate_attrcache(&dnp);
    }
    error
}

/// `nfs_rename` (`vop_rename`): nfs file rename call. A target that exists and is in use is
/// silly-renamed before the rename of the new file over it. Releases the four vnodes, as the
/// C's.
pub fn nfs_rename<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if tcnp.cn_flags & crate::fs::namei::HASBUF == 0
        || fcnp.cn_flags & crate::fs::namei::HASBUF == 0
    {
        kpanic(format_args!("nfs_rename: no name"));
    }
    let same = |a: &Vnode<G>, b: &Vnode<G>| match (a.mount(), b.mount()) {
        (Some(x), Some(y)) => ptr::eq(x, y),
        (None, None) => true,
        _ => false,
    };
    let mut tvp = tvp;
    // Check for cross-device rename
    let error = if !same(fvp, tdvp) || tvp.is_some_and(|t| !same(fvp, t)) {
        Err(Errno::EXDEV)
    } else {
        // If the tvp exists and is in use, sillyrename it before doing the rename of the new
        // file over it.
        if let Some(t) = tvp
            && t.usecount() > 1
            && t.vtype() != VDIR
        {
            let tnp = vtonfs(t);
            let silly = {
                let r = klock::reader();
                tnp.n_k.get(&r).n_sillyrename.is_some()
            };
            if !silly && nfs_sillyrename(tdvp, t, &tnp, tcnp).is_ok() {
                VN_KNOTE(t, NOTE_DELETE);
                vput(t);
                tvp = None;
            }
        }

        let error = nfs_renamerpc(
            fdvp,
            fcnp.name(),
            tdvp,
            tcnp.name(),
            ucred(tcnp.cn_cred),
            Some(tcnp.cn_proc),
        );

        VN_KNOTE(fdvp, NOTE_WRITE);
        VN_KNOTE(tdvp, NOTE_WRITE);

        if fvp.vtype() == VDIR {
            if tvp.is_some_and(|t| t.vtype() == VDIR) {
                cache_purge(tdvp);
            }
            cache_purge(fdvp);
        }
        error.map_err(NfsError::errno)
    };

    // out:
    if tvp.is_some_and(|t| ptr::eq(t, tdvp)) {
        let _ = vrele(tdvp);
    } else {
        vput(tdvp);
    }
    if let Some(t) = tvp {
        vput(t);
    }
    let _ = vrele(fdvp);
    let _ = vrele(fvp);
    // Kludge: Map ENOENT => 0 assuming that it is a reply to a retry.
    match error {
        Err(Errno::ENOENT) => Ok(()),
        e => e,
    }
}

/// `nfs_renameit(sdvp, scnp, sp)`: nfs file rename rpc called from `nfs_remove()` above (the
/// silly rename of `scnp`'s name in `sdvp` to `sp`'s).
pub fn nfs_renameit<G: ProcGlue>(
    sdvp: &'static Vnode<G>,
    scnp: &Cn<'_, G>,
    sp: &Sillyrename<G>,
) -> Result<(), NfsError> {
    let n = usize::try_from(sp.s_namlen)
        .unwrap_or(0)
        .min(sp.s_name.len());
    G::with_curproc(|p| {
        nfs_renamerpc(
            sdvp,
            scnp.name(),
            sdvp,
            &sp.s_name[..n],
            ucred(scnp.cn_cred),
            p,
        )
    })
}

/// `nfs_renamerpc(fdvp, fnameptr, fnamelen, tdvp, tnameptr, tnamelen, cred, proc)`: do an nfs
/// rename rpc. Called from `nfs_rename()` and `nfs_renameit()`.
pub fn nfs_renamerpc<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fname: &[u8],
    tdvp: &'static Vnode<G>,
    tname: &[u8],
    cred: &Ucred,
    proc: Option<&Thread<G>>,
) -> Result<(), NfsError> {
    let fdnp = vtonfs(fdvp);
    let tdnp = vtonfs(tdvp);
    let nmp = vfstonfs_vp(fdvp);
    let v3 = nfs_isv3(&nmp);
    let mut fwccflag = NFSV3_WCCRATTR;
    let mut twccflag = NFSV3_WCCRATTR;

    NFSSTATS.rpccnt[NFSPROC_RENAME].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(
        (nfsx_fh(v3) + NFSX_UNSIGNED) * 2 + nfsm_rndup(fname.len()) + nfsm_rndup(tname.len()),
    );
    fhtom(&mut w, &fdnp, v3);
    let error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(fname, NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }
        fhtom(&mut w, &tdnp, v3);
        if let Err(e) = w.strtom(tname, NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }

        let (reply, res) = nfs_request(
            &nmp,
            fdvp,
            NFSPROC_RENAME,
            Request {
                body: w,
                procp: proc,
                cred,
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };
        if v3 {
            if let Err(e) = nfsm_wcc_data(&mut x, &fdnp, &mut fwccflag) {
                break 'nfsmout Err(e.into());
            }
            if let Err(e) = nfsm_wcc_data(&mut x, &tdnp, &mut twccflag) {
                break 'nfsmout Err(e.into());
            }
        }
        res
    };

    // nfsmout:
    fdnp.set(NMODIFIED);
    tdnp.set(NMODIFIED);
    if fwccflag == 0 {
        nfs_invalidate_attrcache(&fdnp);
    }
    if twccflag == 0 {
        nfs_invalidate_attrcache(&tdnp);
    }
    error
}

/// `nfs_link` (`vop_link`): nfs hard link create call. Unlocks `vp` and `vput`s `dvp`, as the
/// C's.
pub fn nfs_link<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    if let Err(e) = vn_lock(vp, LK_EXCLUSIVE) {
        let _ = vop_abortop(dvp, cnp);
        vput(dvp);
        return Err(e);
    }
    let np = vtonfs(vp);
    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(vp);
    let v3 = nfs_isv3(&nmp);
    let mut wccflag = NFSV3_WCCRATTR;
    let mut attrflag = 0;

    // Push all writes to the server, so that the attribute cache doesn't get "out of sync"
    // with the server. XXX There should be a better way!
    let _ = vop_fsync(vp, cnp.cn_cred, MNT_WAIT, cnp.cn_proc);

    NFSSTATS.rpccnt[NFSPROC_LINK].fetch_add(1, Ordering::Relaxed);
    let len = cnp.name().len();
    let mut w = XdrWriter::reqhead(2 * nfsx_fh(v3) + NFSX_UNSIGNED + nfsm_rndup(len));
    fhtom(&mut w, &np, v3);
    fhtom(&mut w, &dnp, v3);
    let error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(cnp.name(), NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }

        let (reply, res) = nfs_request(
            &nmp,
            vp,
            NFSPROC_LINK,
            Request {
                body: w,
                procp: Some(cnp.cn_proc),
                cred: ucred(cnp.cn_cred),
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };
        if v3 {
            if let Err(e) = nfsm_postop_attr(&mut x, &np, v3, &mut attrflag) {
                break 'nfsmout Err(e.into());
            }
            if let Err(e) = nfsm_wcc_data(&mut x, &dnp, &mut wccflag) {
                break 'nfsmout Err(e.into());
            }
        }
        res
    };

    // nfsmout:
    drop(cnp.take_pnbuf());
    dnp.set(NMODIFIED);
    if attrflag == 0 {
        nfs_invalidate_attrcache(&np);
    }
    if wccflag == 0 {
        nfs_invalidate_attrcache(&dnp);
    }

    VN_KNOTE(vp, NOTE_LINK);
    VN_KNOTE(dvp, NOTE_WRITE);
    let _ = vop_unlock(vp);
    vput(dvp);
    error.map_err(NfsError::errno)
}

/// `nfs_symlink` (`vop_symlink`): nfs symbolic link create call. `vput`s `dvp`, as the C's.
pub fn nfs_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
    target: &[u8],
) -> Result<(), Errno> {
    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(dvp);
    let v3 = nfs_isv3(&nmp);
    let mut wccflag = NFSV3_WCCRATTR;
    let mut newnp: Option<NfsNodeRef<G>> = None;

    NFSSTATS.rpccnt[NFSPROC_SYMLINK].fetch_add(1, Ordering::Relaxed);
    // `slen = strlen(ap->a_target)`.
    let slen = target.iter().position(|&c| c == 0).unwrap_or(target.len());
    let target = &target[..slen];
    let len = cnp.name().len();
    let mut w = XdrWriter::reqhead(
        nfsx_fh(v3) + 2 * NFSX_UNSIGNED + nfsm_rndup(len) + nfsm_rndup(slen) + nfsx_sattr(v3),
    );
    fhtom(&mut w, &dnp, v3);
    let error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(cnp.name(), NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }
        if v3 {
            w.v3attrbuild(vap, false);
        }
        if let Err(e) = w.strtom(target, NFS_MAXPATHLEN) {
            break 'nfsmout Err(e.into());
        }
        if !v3 {
            w.pod(&v2sattr_create(VLNK, vap, nfs_xdrneg1));
        }

        let (reply, res) = nfs_request(
            &nmp,
            dvp,
            NFSPROC_SYMLINK,
            Request {
                body: w,
                procp: Some(cnp.cn_proc),
                cred: ucred(cnp.cn_cred),
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };
        if v3 {
            if res.is_ok() {
                let mut gotvp = 0;
                if let Err(e) = nfsm_mtofh(rd(&mut x), dvp, v3, &mut newnp, &mut gotvp) {
                    break 'nfsmout Err(e.into());
                }
            }
            if let Err(e) = nfsm_wcc_data(&mut x, &dnp, &mut wccflag) {
                break 'nfsmout Err(e.into());
            }
        }
        res
    };

    // nfsmout:
    if let Some(n) = newnp {
        vput(nfstov(&n));
    }
    drop(cnp.take_pnbuf());
    dnp.set(NMODIFIED);
    if wccflag == 0 {
        nfs_invalidate_attrcache(&dnp);
    }
    VN_KNOTE(dvp, NOTE_WRITE);
    vput(dvp);
    error.map_err(NfsError::errno)
}

/// `nfs_mkdir` (`vop_mkdir`): nfs make dir call. `vput`s `dvp`, as the C's.
pub fn nfs_mkdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(dvp);
    let v3 = nfs_isv3(&nmp);
    let mut wccflag = NFSV3_WCCRATTR;
    let mut newnp: Option<NfsNodeRef<G>> = None;

    let len = cnp.name().len();
    NFSSTATS.rpccnt[NFSPROC_MKDIR].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3) + NFSX_UNSIGNED + nfsm_rndup(len) + nfsx_sattr(v3));
    fhtom(&mut w, &dnp, v3);
    let mut error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(cnp.name(), NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }

        if v3 {
            w.v3attrbuild(vap, false);
        } else {
            w.pod(&v2sattr_create(VDIR, vap, nfs_xdrneg1));
        }

        let (reply, res) = nfs_request(
            &nmp,
            dvp,
            NFSPROC_MKDIR,
            Request {
                body: w,
                procp: Some(cnp.cn_proc),
                cred: ucred(cnp.cn_cred),
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };
        if res.is_ok() {
            let mut gotvp = 0;
            if let Err(e) = nfsm_mtofh(rd(&mut x), dvp, v3, &mut newnp, &mut gotvp) {
                break 'nfsmout Err(e.into());
            }
        }
        if v3 && let Err(e) = nfsm_wcc_data(&mut x, &dnp, &mut wccflag) {
            break 'nfsmout Err(e.into());
        }
        res
    };

    // nfsmout:
    dnp.set(NMODIFIED);
    if wccflag == 0 {
        nfs_invalidate_attrcache(&dnp);
    }

    if error.is_ok() && newnp.is_none() {
        let mut np = None;
        error = nfs_lookitup(
            dvp,
            cnp.name(),
            ucred(cnp.cn_cred),
            Some(cnp.cn_proc),
            Npp::Alloc(&mut np),
        );
        if error.is_ok() {
            newnp = np;
            if newnp.as_deref().is_some_and(|n| nfstov(n).vtype() != VDIR) {
                error = Err(Errno::EEXIST.into());
            }
        }
    }
    if error.is_err() {
        if let Some(n) = newnp {
            vput(nfstov(&n));
        }
    } else {
        VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);
        if cnp.cn_flags & MAKEENTRY != 0 {
            nfs_cache_enter(dvp, &dnp, newnp.as_deref(), cnp.name());
        }
        *vpp = newnp.as_ref().map(|n| nfstov(n));
    }
    drop(cnp.take_pnbuf());
    vput(dvp);
    error.map_err(NfsError::errno)
}

/// `nfs_rmdir` (`vop_rmdir`): nfs remove directory call. `vput`s both vnodes, as the C's.
pub fn nfs_rmdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(dvp);
    let v3 = nfs_isv3(&nmp);
    let mut wccflag = NFSV3_WCCRATTR;

    NFSSTATS.rpccnt[NFSPROC_RMDIR].fetch_add(1, Ordering::Relaxed);
    let len = cnp.name().len();
    let mut w = XdrWriter::reqhead(nfsx_fh(v3) + NFSX_UNSIGNED + nfsm_rndup(len));
    fhtom(&mut w, &dnp, v3);
    let error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(cnp.name(), NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }

        let (reply, res) = nfs_request(
            &nmp,
            dvp,
            NFSPROC_RMDIR,
            Request {
                body: w,
                procp: Some(cnp.cn_proc),
                cred: ucred(cnp.cn_cred),
            },
        );
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };
        if v3 && let Err(e) = nfsm_wcc_data(&mut x, &dnp, &mut wccflag) {
            break 'nfsmout Err(e.into());
        }
        res
    };

    // nfsmout:
    drop(cnp.take_pnbuf());
    dnp.set(NMODIFIED);
    if wccflag == 0 {
        nfs_invalidate_attrcache(&dnp);
    }

    VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);
    VN_KNOTE(vp, NOTE_DELETE);

    cache_purge(vp);
    vput(vp);
    vput(dvp);
    // Kludge: Map ENOENT => 0 assuming that you have a reply to a retry.
    match error {
        Err(NfsError::Errno(Errno::ENOENT)) => Ok(()),
        e => e.map_err(NfsError::errno),
    }
}

/// `nfs_readdir` (`vop_readdir`): nfs readdir call. The RPCs fill a buffer of
/// `NFS_DIRBLKSIZ` bytes with `struct nfs_dirent` records (READDIRPLUS on a mount that asks
/// for it, READDIR when it is not supported), each record is made a `struct dirent` (its
/// `d_off` the entry's cookie) and copied out while the uio has room; the uio's offset is the
/// cookie of the last entry copied. The EOF offset cache answers a read at the end of an
/// unchanged directory without an RPC.
///
/// The readdir logic has a big design bug. It stores the NFS cookie in the returned
/// `uio->uio_offset` but does not store the verifier (it cannot). Instead, the code stores the
/// verifier in the nfsnode and applies that verifies to all cookies, no matter what verifier
/// was originally with the cookie. From a practical standpoint, this is not a problem since
/// almost all NFS servers do not change the validity of cookies across deletes and inserts.
pub fn nfs_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    cred: Cred<'_>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    if vp.vtype() != VDIR {
        return Err(Errno::EPERM);
    }
    let np = vtonfs(vp);
    let nmp = vfstonfs_vp(vp);

    // First, check for hit on the EOF offset cache
    let direof = np.n_direofoffset.load(Ordering::Relaxed);
    if direof != 0 && uio.uio_offset == direof {
        let mut vattr = Vattr::new();
        if with_thread(uio.uio_procp, |p| {
            nfs_getattr_np(vp, &np, &nmp, &mut vattr, cred, p)
        })
        .is_ok()
            && *np.n_mtime.read() == vattr.va_mtime
        {
            NFSSTATS.direofcache_hits.fetch_add(1, Ordering::Relaxed);
            *eofflag = 1;
            return Ok(());
        }
    }

    if (uio.uio_resid as u64) < NFS_FABLKSIZE {
        return Err(Errno::EINVAL);
    }

    let tresid = uio.uio_resid;

    if uio.uio_rw != UioRw::UIO_READ {
        return Err(Errno::EINVAL);
    }

    let cr = ucred(cred);
    if nmp.flag() & (NFSMNT_NFSV3 | NFSMNT_GOTFSINFO) == NFSMNT_NFSV3 {
        let _ = nfs_fsinfo(&nmp, vp, cr, uio.uio_procp);
    }

    let mut cnt = 5;
    let mut newoff = uio.uio_offset as u64;
    let mut done = false;
    let mut eof = false;

    // M_ZERO to avoid leaking kernel data in dirent padding
    let Some(mut data) = MArray::new(M_TEMP, M_WAITOK, DIRBLKSIZ, |_| 0u8) else {
        kpanic(format_args!("nfs_readdir: no memory"));
    };
    let error = loop {
        // A fresh `readdir_uio` over the buffer at the offset `newoff`.
        let mut pos = 0;
        let mut uoff = newoff;
        let mut error: Result<(), NfsError> = Ok(());
        if nmp.flag() & NFSMNT_RDIRPLUS != 0 {
            error = nfs_readdirplusrpc(vp, &np, &nmp, &mut data, &mut pos, &mut uoff, cr, &mut eof);
            if error.is_err_and(|e| e.raw() == NFSERR_NOTSUPP) {
                nmp.nm_flag.fetch_and(!NFSMNT_RDIRPLUS, Ordering::Relaxed);
            }
        }
        if nmp.flag() & NFSMNT_RDIRPLUS == 0 {
            error = nfs_readdirrpc(vp, &np, &nmp, &mut data, &mut pos, &mut uoff, cr, &mut eof);
        }

        if error.is_err_and(|e| e.raw() == NFSERR_BAD_COOKIE) {
            error = Err(Errno::EINVAL.into());
        }

        let mut ndp = 0;
        while error.is_ok() && ndp < pos {
            match nfs_dirent_out(&mut data, ndp, uio) {
                Ok(DirentOut::Moved { reclen, cookie }) => {
                    newoff = cookie;
                    ndp += reclen;
                }
                Ok(DirentOut::Full) => {
                    eof = false;
                    done = true;
                    break;
                }
                Err(e) => error = Err(e.into()),
            }
        }
        if error.is_err() || done || eof || cnt == 0 {
            break error;
        }
        cnt -= 1;
    };
    drop(data);

    uio.uio_offset = newoff as i64;

    if error.is_ok() && (eof || uio.uio_resid == tresid) {
        NFSSTATS.direofcache_misses.fetch_add(1, Ordering::Relaxed);
        *eofflag = 1;
        return Ok(());
    }

    *eofflag = 0;
    error.map_err(NfsError::errno)
}

/// What [`nfs_dirent_out`] did with a record.
enum DirentOut {
    /// The record went out: its size in the buffer and its cookie.
    Moved {
        /// The record's `d_reclen` as packed (its cookie's 8 bytes included).
        reclen: usize,
        /// `fxdr_hyper(&ndp->cookie[0])`.
        cookie: u64,
    },
    /// The uio has no room for it.
    Full,
}

/// `nfs_readdir`'s step over the record at `ndp` of the buffer: made a `struct dirent`
/// (`d_reclen` less the cookie's bytes, `d_off` the cookie) in place, refused with `EBADRPC`
/// when its name holds a `/`, and copied out by its `d_reclen` bytes when the uio has room.
fn nfs_dirent_out<G: ProcGlue>(
    buf: &mut [u8],
    ndp: usize,
    uio: &mut Uio<'_, G>,
) -> Result<DirentOut, Errno> {
    // The records are the packer's own, whole within what it packed; a cut one is a
    // malformed record all the same.
    let Some(hdr) = buf.get(ndp..ndp + NFS_DIRHDSIZ) else {
        return Err(Errno::EBADRPC);
    };
    let reclen = usize::from(u16::from_ne_bytes([hdr[ND_RECLEN], hdr[ND_RECLEN + 1]]));
    let namlen = usize::from(hdr[ND_NAMLEN]);
    let cookie = fxdr_hyper([
        u32::from_ne_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]),
        u32::from_ne_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]),
    ]);
    let d_reclen = reclen.wrapping_sub(NFS_DIRENT_OVERHEAD) as u16;
    buf[ndp + ND_RECLEN..ndp + ND_RECLEN + 2].copy_from_slice(&d_reclen.to_ne_bytes());
    buf[ndp + ND_OFF..ndp + ND_OFF + 8].copy_from_slice(&cookie.to_ne_bytes());

    let Some(name) = buf.get(ndp + NFS_DIRHDSIZ..ndp + NFS_DIRHDSIZ + namlen) else {
        return Err(Errno::EBADRPC);
    };
    if name.contains(&b'/') {
        return Err(Errno::EBADRPC);
    }

    let d_reclen = usize::from(d_reclen);
    if uio.uio_resid < d_reclen {
        return Ok(DirentOut::Full);
    }

    let at = ndp + NFS_DIRENT_OVERHEAD;
    let Some(dp) = buf.get(at..at + d_reclen) else {
        return Err(Errno::EBADRPC);
    };
    <G::FileGlue as FileGlue>::uiomove_out(dp, uio)?;
    Ok(DirentOut::Moved { reclen, cookie })
}

/// `nfs_readdirrpc(vp, uiop, cred, end_of_directory)`: readdir rpc call. Loop around doing
/// readdir rpc's of size `nm_readdirsize` truncated to a multiple of `NFS_READDIRBLKSIZ`,
/// packing the entries into `buf` from `*posp` (the C's uio over `nfs_readdir`'s buffer, from
/// the cookie `*offset`, the uio's offset, which each name copied moves on as `nfsm_mtouio`
/// does; `*posp` is advanced over what was packed, on an error too). The
/// stopping criteria is EOF or buffer full. `np` and `nmp` are the directory's node and
/// mount, lent.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's four, the lent node and mount, the buffer's two halves"
)]
pub fn nfs_readdirrpc<G: ProcGlue>(
    vp: &'static Vnode<G>,
    dnp: &NfsNode<G>,
    nmp: &NfsMount<G>,
    buf: &mut [u8],
    posp: &mut usize,
    offset: &mut u64,
    cred: &Ucred,
    end_of_directory: &mut bool,
) -> Result<(), NfsError> {
    #[cfg(feature = "diagnostic")]
    if (buf.len() - *posp) & (DIRBLKSIZ - 1) != 0 {
        kpanic(format_args!("nfs readdirrpc bad uio"));
    }
    let mut pack = DirPack::new(buf, *posp);
    let error = readdir_pack(vp, dnp, nmp, &mut pack, offset, cred, end_of_directory);
    *posp = pack.pos;
    error
}

/// [`nfs_readdirrpc`]'s RPCs over the packing.
fn readdir_pack<G: ProcGlue>(
    vp: &'static Vnode<G>,
    dnp: &NfsNode<G>,
    nmp: &NfsMount<G>,
    pack: &mut DirPack<'_>,
    offset: &mut u64,
    cred: &Ucred,
    end_of_directory: &mut bool,
) -> Result<(), NfsError> {
    let v3 = nfs_isv3(nmp);
    let mut cookie = txdr_hyper(*offset);
    let mut more_dirs = true;
    let mut bigenough = true;

    G::with_curproc(|p| -> Result<(), NfsError> {
        while more_dirs && bigenough {
            NFSSTATS.rpccnt[NFSPROC_READDIR].fetch_add(1, Ordering::Relaxed);
            let mut w = XdrWriter::reqhead(nfsx_fh(v3) + nfsx_readdir(v3));
            fhtom(&mut w, dnp, v3);
            if v3 {
                w.word(cookie[0]);
                w.word(cookie[1]);
                if cookie == [0, 0] {
                    w.word(0);
                    w.word(0);
                } else {
                    let verf = dnp.n_cookieverf.read().nfsuquad;
                    w.word(verf[0]);
                    w.word(verf[1]);
                }
            } else {
                w.word(cookie[1]);
            }
            w.u32(nmp.nm_readdirsize.load(Ordering::Relaxed) as u32);

            let (reply, res) = nfs_request(
                nmp,
                vp,
                NFSPROC_READDIR,
                Request {
                    body: w,
                    procp: p,
                    cred,
                },
            );
            let mut x = mrep(reply.as_ref())?;
            if v3 {
                let mut attrflag = 0;
                nfsm_postop_attr(&mut x, dnp, v3, &mut attrflag)?;
            }
            res?;

            let r = rd(&mut x);
            if v3 {
                let tl = r.get::<{ 2 * NFSX_UNSIGNED }>()?;
                let _ = dnp.n_cookieverf.replace(Nfsuint64 {
                    nfsuquad: [
                        u32::from_ne_bytes([tl[0], tl[1], tl[2], tl[3]]),
                        u32::from_ne_bytes([tl[4], tl[5], tl[6], tl[7]]),
                    ],
                });
            }

            more_dirs = r.i32()? != 0;

            // loop thru the dir entries, doctoring them to dirent form
            while more_dirs && bigenough {
                let (fileno, len) = if v3 {
                    let tl = r.get::<{ 3 * NFSX_UNSIGNED }>()?;
                    (
                        u64::from_be_bytes([
                            tl[0], tl[1], tl[2], tl[3], tl[4], tl[5], tl[6], tl[7],
                        ]),
                        i32::from_be_bytes([tl[8], tl[9], tl[10], tl[11]]),
                    )
                } else {
                    let tl = r.get::<{ 2 * NFSX_UNSIGNED }>()?;
                    (
                        u64::from(u32::from_be_bytes([tl[0], tl[1], tl[2], tl[3]])),
                        i32::from_be_bytes([tl[4], tl[5], tl[6], tl[7]]),
                    )
                };
                if len <= 0 || len > NFS_MAXNAMLEN as i32 {
                    return Err(Errno::EBADRPC.into());
                }
                // `0 < len <= NFS_MAXNAMLEN`: a size.
                let len = len as usize;
                if pack.begin(fileno, len) {
                    r.bytes_into(pack.name_slot(len))?;
                    r.adv(nfsm_padlen(len))?;
                    pack.end_name(len);
                    // `nfsm_mtouio` moves the uio's offset past the name.
                    *offset = offset.wrapping_add(len as u64);
                } else {
                    bigenough = false;
                    r.adv(nfsm_rndup(len))?;
                }
                let more = if v3 {
                    let tl = r.get::<{ 3 * NFSX_UNSIGNED }>()?;
                    let c = [
                        u32::from_ne_bytes([tl[0], tl[1], tl[2], tl[3]]),
                        u32::from_ne_bytes([tl[4], tl[5], tl[6], tl[7]]),
                    ];
                    if bigenough {
                        cookie = c;
                        pack.set_cookie(c);
                    }
                    i32::from_be_bytes([tl[8], tl[9], tl[10], tl[11]])
                } else {
                    let tl = r.get::<{ 2 * NFSX_UNSIGNED }>()?;
                    if bigenough {
                        cookie[1] = u32::from_ne_bytes([tl[0], tl[1], tl[2], tl[3]]);
                        pack.set_cookie([0, cookie[1]]);
                    }
                    i32::from_be_bytes([tl[4], tl[5], tl[6], tl[7]])
                };
                more_dirs = more != 0;
            }
            // If at end of rpc data, get the eof boolean
            if !more_dirs {
                more_dirs = r.i32()? == 0;
            }
        }
        Ok(())
    })?;
    // Fill last record, iff any, out to a multiple of NFS_READDIRBLKSIZ by increasing
    // d_reclen for the last record.
    pack.finish();

    // We are now either at the end of the directory or have filled the block.
    if bigenough {
        dnp.n_direofoffset
            .store(fxdr_hyper(cookie) as i64, Ordering::Relaxed);
        *end_of_directory = true;
    } else if pack.resid() > 0 {
        hal::prf::printf(format_args!("EEK! readdirrpc resid > 0\n"));
    }
    Ok(())
}

/// `nfs_readdirplusrpc(vp, uiop, cred, end_of_directory, p)`: NFS V3 readdir plus RPC. Used in
/// place of `nfs_readdirrpc()`: each entry may carry its attributes and its handle, which
/// make or refresh its node (`d_type` then known) and its name cache entry. `buf`, `posp` and
/// `offset` as [`nfs_readdirrpc`]'s.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's five, the lent node and mount, the buffer's two halves"
)]
pub fn nfs_readdirplusrpc<G: ProcGlue>(
    vp: &'static Vnode<G>,
    dnp: &NfsNodeRef<G>,
    nmp: &NfsMount<G>,
    buf: &mut [u8],
    posp: &mut usize,
    offset: &mut u64,
    cred: &Ucred,
    end_of_directory: &mut bool,
) -> Result<(), NfsError> {
    #[cfg(feature = "diagnostic")]
    if (buf.len() - *posp) & (DIRBLKSIZ - 1) != 0 {
        kpanic(format_args!("nfs readdirplusrpc bad uio"));
    }
    let mut pack = DirPack::new(buf, *posp);
    let error = readdirplus_pack(vp, dnp, nmp, &mut pack, offset, cred, end_of_directory);
    *posp = pack.pos;
    error
}

/// `vput(newvp)` of an entry's node, `vrele` when it is the directory itself.
fn entry_release<G: ProcGlue>(np: Option<NfsNodeRef<G>>, vp: &'static Vnode<G>) {
    if let Some(n) = np {
        vput_or_vrele(nfstov(&n), vp);
    }
}

/// [`nfs_readdirplusrpc`]'s RPCs over the packing.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one loop: the entry, its attributes, its handle and its cache entry"
)]
fn readdirplus_pack<G: ProcGlue>(
    vp: &'static Vnode<G>,
    dnp: &NfsNodeRef<G>,
    nmp: &NfsMount<G>,
    pack: &mut DirPack<'_>,
    offset: &mut u64,
    cred: &Ucred,
    end_of_directory: &mut bool,
) -> Result<(), NfsError> {
    let mut cookie = txdr_hyper(*offset);
    let mut more_dirs = true;
    let mut bigenough = true;
    let mut error: Result<(), NfsError> = Ok(());

    G::with_curproc(|p| -> Result<(), NfsError> {
        while more_dirs && bigenough {
            NFSSTATS.rpccnt[NFSPROC_READDIRPLUS].fetch_add(1, Ordering::Relaxed);
            let mut w = XdrWriter::reqhead(nfsx_fh(true) + 6 * NFSX_UNSIGNED);
            fhtom(&mut w, dnp, true);
            w.word(cookie[0]);
            w.word(cookie[1]);
            if cookie == [0, 0] {
                w.word(0);
                w.word(0);
            } else {
                let verf = dnp.n_cookieverf.read().nfsuquad;
                w.word(verf[0]);
                w.word(verf[1]);
            }
            w.u32(nmp.nm_readdirsize.load(Ordering::Relaxed) as u32);
            w.u32(nmp.nm_rsize.load(Ordering::Relaxed) as u32);

            let (reply, res) = nfs_request(
                nmp,
                vp,
                NFSPROC_READDIRPLUS,
                Request {
                    body: w,
                    procp: p,
                    cred,
                },
            );
            let mut x = mrep(reply.as_ref())?;
            let mut attrflag = 0;
            nfsm_postop_attr(&mut x, dnp, true, &mut attrflag)?;
            res?;
            error = Ok(());

            let r = rd(&mut x);
            let tl = r.get::<{ 3 * NFSX_UNSIGNED }>()?;
            let _ = dnp.n_cookieverf.replace(Nfsuint64 {
                nfsuquad: [
                    u32::from_ne_bytes([tl[0], tl[1], tl[2], tl[3]]),
                    u32::from_ne_bytes([tl[4], tl[5], tl[6], tl[7]]),
                ],
            });
            more_dirs = i32::from_be_bytes([tl[8], tl[9], tl[10], tl[11]]) != 0;

            // loop thru the dir entries, doctoring them to 4bsd form
            while more_dirs && bigenough {
                let tl = r.get::<{ 3 * NFSX_UNSIGNED }>()?;
                let fileno =
                    u64::from_be_bytes([tl[0], tl[1], tl[2], tl[3], tl[4], tl[5], tl[6], tl[7]]);
                let len = i32::from_be_bytes([tl[8], tl[9], tl[10], tl[11]]);
                if len <= 0 || len > NFS_MAXNAMLEN as i32 {
                    return Err(Errno::EBADRPC.into());
                }
                // `0 < len <= NFS_MAXNAMLEN`: a size.
                let len = len as usize;
                if pack.begin(fileno, len) {
                    r.bytes_into(pack.name_slot(len))?;
                    r.adv(nfsm_padlen(len))?;
                    pack.end_name(len);
                    // `nfsm_mtouio` moves the uio's offset past the name.
                    *offset = offset.wrapping_add(len as u64);
                } else {
                    bigenough = false;
                    r.adv(nfsm_rndup(len))?;
                }
                let tl = r.get::<{ 3 * NFSX_UNSIGNED }>()?;
                if bigenough {
                    let c = [
                        u32::from_ne_bytes([tl[0], tl[1], tl[2], tl[3]]),
                        u32::from_ne_bytes([tl[4], tl[5], tl[6], tl[7]]),
                    ];
                    cookie = c;
                    pack.set_cookie(c);
                }

                // Since the attributes are before the file handle (sigh), we must skip over
                // the attributes and then come back and get them.
                let attrflag = i32::from_be_bytes([tl[8], tl[9], tl[10], tl[11]]);
                let mut newnp: Option<NfsNodeRef<G>> = None;
                let entry: Result<(), Errno> = 'entry: {
                    if attrflag != 0 {
                        let sav1 = r.clone();
                        if let Err(e) = r.adv(NFSX_V3FATTR) {
                            break 'entry Err(e);
                        }
                        let mut doit = match r.i32() {
                            Ok(d) => d,
                            Err(e) => break 'entry Err(e),
                        };
                        if doit != 0 {
                            let fh = match nfsm_getfh(r, true) {
                                Ok(fh) => fh,
                                Err(e) => break 'entry Err(e),
                            };
                            if nfs_cmpfh(dnp, fh.bytes()) {
                                vref(vp);
                                newnp = Some(dnp.clone());
                            } else {
                                match nfs_nget(vmount(vp), fh.bytes()) {
                                    Ok(n) => {
                                        newnp = Some(n);
                                        error = Ok(());
                                    }
                                    Err(e) => {
                                        error = Err(e.into());
                                        doit = 0;
                                    }
                                }
                            }
                        }
                        if doit != 0
                            && bigenough
                            && let Some(np) = newnp.as_deref()
                        {
                            let mut a = sav1;
                            if let Err(e) = nfsm_loadattr(&mut a, np, true, None) {
                                break 'entry Err(e);
                            }
                            let vtype = np.n_vattr.read().va_type;
                            pack.set_type(iftodt(vttoif(vtype)));
                            if pack.name().len() <= crate::fs::namei::NAMECACHE_MAXLEN {
                                cache_purge(vp);
                                nfs_cache_enter(vp, dnp, Some(np), pack.name());
                            }
                        }
                    } else {
                        // Just skip over the file handle
                        let i = match r.i32() {
                            Ok(i) => i,
                            Err(e) => break 'entry Err(e),
                        };
                        if i > 0 {
                            // `i > 0`: a size, rounded up as the C's `int` (at most 2^31 - 1).
                            if let Err(e) = r.adv(nfsm_rndup(i as usize)) {
                                break 'entry Err(e);
                            }
                        }
                    }
                    Ok(())
                };
                entry_release(newnp, vp);
                entry?;
                more_dirs = r.i32()? != 0;
            }
            // If at end of rpc data, get the eof boolean
            if !more_dirs {
                more_dirs = r.i32()? == 0;
            }
        }
        Ok(())
    })?;
    // Fill last record, iff any, out to a multiple of NFS_READDIRBLKSIZ by increasing
    // d_reclen for the last record.
    pack.finish();

    // We are now either at the end of the directory or have filled the block.
    if bigenough {
        dnp.n_direofoffset
            .store(fxdr_hyper(cookie) as i64, Ordering::Relaxed);
        *end_of_directory = true;
    } else if pack.resid() > 0 {
        hal::prf::printf(format_args!("EEK! readdirplusrpc resid > 0\n"));
    }
    error
}

/// `.nfs%08X%08X` of two random words: the silly name and its length (20 bytes, the record's
/// 24 have room).
fn nfs_sillyname(rnd: [u32; 2]) -> ([u8; 24], i64) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut name = [0u8; 24];
    name[..4].copy_from_slice(b".nfs");
    for (k, word) in rnd.iter().enumerate() {
        for i in 0..8 {
            let nibble = (word >> (28 - 4 * i)) & 0xf;
            name[4 + 8 * k + i] = HEX[nibble as usize];
        }
    }
    (name, 20)
}

/// `nfs_sillyrename(dvp, vp, cnp)`: silly rename. To make the NFS filesystem that is stateless
/// look a little more like the "ufs" a remove of an active vnode is translated to a rename to
/// a funny looking filename that is removed by `nfs_inactive` on the nfsnode. There is the
/// potential for another process on a different client to create the same funny name between
/// the `nfs_lookitup()` fails and the `nfs_rename()` completes, but... `np` is `vp`'s node.
pub fn nfs_sillyrename<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    np: &NfsNodeRef<G>,
    cnp: &Cn<'_, G>,
) -> Result<(), Errno> {
    cache_purge(dvp);
    let Some(mut sp) = MBox::new(
        M_NFSREQ,
        M_WAITOK,
        Sillyrename {
            s_cred: crdup(ucred(cnp.cn_cred)),
            s_dvp: dvp,
            s_namlen: 0,
            s_name: [0; 24],
        },
    ) else {
        kpanic(format_args!("nfs_sillyrename: no memory"));
    };
    vref(dvp);

    let error: Errno = 'bad: {
        if vp.vtype() == VDIR {
            #[cfg(feature = "diagnostic")]
            hal::prf::printf(format_args!("nfs: sillyrename dir\n"));
            break 'bad Errno::EINVAL;
        }

        // Try lookitups until we get one that isn't there
        loop {
            // Fudge together a funny name
            let mut b = [0u8; 8];
            arc4random_buf(&mut b);
            let rnd = [
                u32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
                u32::from_ne_bytes([b[4], b[5], b[6], b[7]]),
            ];
            let (name, namlen) = nfs_sillyname(rnd);
            sp.s_name = name;
            sp.s_namlen = namlen;

            if nfs_lookitup(
                dvp,
                &sp.s_name[..20],
                &sp.s_cred,
                Some(cnp.cn_proc),
                Npp::Null,
            )
            .is_err()
            {
                break;
            }
        }

        if let Err(e) = nfs_renameit(dvp, cnp, &sp) {
            break 'bad e.errno();
        }
        let _ = nfs_lookitup(
            dvp,
            &sp.s_name[..20],
            &sp.s_cred,
            Some(cnp.cn_proc),
            Npp::Update(np),
        );
        let old = {
            let mut w = klock::witness();
            np.n_k.get_mut(&mut w).n_sillyrename.replace(sp)
        };
        drop(old);
        return Ok(());
    };

    // bad: the directory's reference, then the credential and the record (`crfree`, `free`).
    let _ = vrele(sp.s_dvp);
    drop(sp);
    Err(error)
}

/// `nfs_lookitup(dvp, name, len, cred, procp, npp)`: look up a file name and optionally
/// either update the file handle or allocate an nfsnode, depending on the value of `npp`
/// ([`Npp`]). A new node comes back referenced and locked (or `dvp` itself, referenced).
pub fn nfs_lookitup<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    name: &[u8],
    cred: &Ucred,
    procp: Option<&Thread<G>>,
    npp: Npp<'_, G>,
) -> Result<(), NfsError> {
    let dnp = vtonfs(dvp);
    let nmp = vfstonfs_vp(dvp);
    let v3 = nfs_isv3(&nmp);
    let mut newnp: Option<NfsNodeRef<G>> = None;

    NFSSTATS.rpccnt[NFSPROC_LOOKUP].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3) + NFSX_UNSIGNED + nfsm_rndup(name.len()));
    fhtom(&mut w, &dnp, v3);
    let alloc = matches!(npp, Npp::Alloc(_));
    let error: Result<(), NfsError> = 'nfsmout: {
        if let Err(e) = w.strtom(name, NFS_MAXNAMLEN) {
            break 'nfsmout Err(e.into());
        }

        let (reply, res) = nfs_request(
            &nmp,
            dvp,
            NFSPROC_LOOKUP,
            Request {
                body: w,
                procp,
                cred,
            },
        );
        if res.is_err() && !v3 {
            break 'nfsmout res;
        }
        let mut x = match mrep(reply.as_ref()) {
            Ok(x) => x,
            Err(e) => break 'nfsmout Err(e.into()),
        };

        if !matches!(npp, Npp::Null) && res.is_ok() {
            let fh = match nfsm_getfh(rd(&mut x), v3) {
                Ok(fh) => fh,
                Err(e) => break 'nfsmout Err(e.into()),
            };
            match &npp {
                Npp::Update(np) => nfs_nrehash(np.as_pin(), fh),
                _ if nfs_cmpfh(&dnp, fh.bytes()) => {
                    vref(dvp);
                    newnp = Some(dnp.clone());
                }
                _ => {
                    let n = nfs_nget(vmount(dvp), fh.bytes())?;
                    newnp = Some(n);
                }
            }
            // The node the attributes are for: the one updated, or the one looked up.
            let target: &NfsNode<G> = match (&npp, newnp.as_deref()) {
                (Npp::Update(np), _) => np,
                (_, Some(n)) => n,
                (_, None) => &dnp,
            };
            if v3 {
                let mut attrflag = 0;
                if let Err(e) = nfsm_postop_attr(&mut x, target, v3, &mut attrflag) {
                    break 'nfsmout Err(e.into());
                }
                if attrflag == 0 && alloc {
                    entry_release(newnp.take(), dvp);
                    return Err(Errno::ENOENT.into());
                }
            } else if let Err(e) = nfsm_loadattr(rd(&mut x), target, v3, None) {
                break 'nfsmout Err(e.into());
            }
        }
        res
    };

    // nfsmout:
    if let Npp::Alloc(out) = npp {
        if error.is_err() {
            entry_release(newnp, dvp);
        } else {
            *out = newnp;
        }
    }
    error
}

/// `nfs_commit(vp, offset, cnt, procp)`: Nfs Version 3 commit rpc of `cnt` bytes at
/// `offset`: nothing to do before the server gave a write verifier; a changed verifier is
/// stored and answered as `NFSERR_STALEWRITEVERF` (the caller writes the blocks again).
/// `np` and `nmp` are the vnode's node and mount, lent; the credential is `n_wcred`.
pub fn nfs_commit<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    offset: u64,
    cnt: i32,
    procp: Option<&Thread<G>>,
) -> Result<(), NfsError> {
    let mut wccflag = NFSV3_WCCRATTR;

    if nmp.flag() & NFSMNT_HASWRITEVERF == 0 {
        return Ok(());
    }
    NFSSTATS.rpccnt[NFSPROC_COMMIT].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(true));
    fhtom(&mut w, np, true);
    w.hyper(offset);
    w.u32(cnt as u32);

    let cred = nfs_getcred(np, true);
    let (reply, res) = nfs_request(
        nmp,
        vp,
        NFSPROC_COMMIT,
        Request {
            body: w,
            procp,
            cred: &cred,
        },
    );
    let mut x = mrep(reply.as_ref())?;
    nfsm_wcc_data(&mut x, np, &mut wccflag)?;
    res?;
    let verf = rd(&mut x).get::<NFSX_V3WRITEVERF>()?;
    if nmp.verf() != verf {
        nmp.set_verf(verf);
        return Err(NfsError::from_status(NFSERR_STALEWRITEVERF as u32));
    }
    Ok(())
}

/// `nfs_bmap` (`vop_bmap`): Kludge City.. make `nfs_bmap()` essentially a no-op that does no
/// translation (the block in `DEV_BSIZE` units of the mount's I/O size); `nfs_strategy()`
/// does the I/O with `nfs_readrpc`/`nfs_writerpc`.
pub fn nfs_bmap<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    vpp: Option<&mut Option<&'static Vnode<G>>>,
    bnp: Option<&mut Daddr>,
) -> Result<(), Errno> {
    if let Some(vpp) = vpp {
        *vpp = Some(vp);
    }
    if let Some(bnp) = bnp {
        // `btodb(vp->v_mount->mnt_stat.f_iosize)`.
        *bnp = bn.wrapping_mul(Daddr::from(vmount(vp).iosize() / DEV_BSIZE as u32));
    }
    Ok(())
}

/// The token of the buffer `VOP_STRATEGY`/`VOP_BWRITE` was given (the kernel's buffer cache
/// passes its own; one lent by `bsd` carries its token parked).
fn strategy_token(bp: IoBuf) -> BufRef {
    match bp.into_token() {
        Some(t) => t,
        None => kpanic(format_args!("nfs_strategy: a lent buffer with no token")),
    }
}

/// `nfs_strategy` (`vop_strategy`): strategy routine. For async requests when nfsiod(s) are
/// running, queue the request by calling `nfs_asyncio()`, otherwise just all `nfs_doio()` to
/// do the request.
pub fn nfs_strategy<G: ProcGlue>(bp: IoBuf) -> Result<(), Errno> {
    let t = strategy_token(bp);
    let flags = t.buf().flags();
    if flags & (B_PHYS | B_ASYNC) == (B_PHYS | B_ASYNC) {
        kpanic(format_args!("nfs physio/async"));
    }
    // If the op is asynchronous and an i/o daemon is waiting queue the request, wake it up
    // and wait for completion otherwise just do it ourselves.
    if flags & B_ASYNC == 0 {
        // p = curproc (XXX)
        return G::with_curproc(|p| nfs_doio(t, p));
    }
    match nfs_asyncio::<G>(t, false) {
        Ok(()) => Ok(()),
        Err((t, _)) => nfs_doio::<G>(t, None),
    }
}

/// `nfs_ioctl` (`vop_ioctl`).
pub fn nfs_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `nfs_fsync` (`vop_fsync`): just call `nfs_flush()` with commit == 1.
pub fn nfs_fsync<G: ProcGlue>(
    vp: &'static Vnode<G>,
    cred: Cred<'_>,
    waitfor: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let np = vtonfs(vp);
    let nmp = vfstonfs_vp(vp);
    nfs_flush(vp, &np, &nmp, cred, waitfor, Some(p), true)
}

/// `nfs_flush(vp, cred, waitfor, p, commit)`: flush all the blocks associated with a vnode.
/// Walk through the buffer pool and push any dirty pages associated with the vnode. A
/// `b_flags == (B_DELWRI | B_NEEDCOMMIT)` block has been written to the server, but has not
/// been committed to stable storage on the server yet. On the first pass, the byte range is
/// worked out and the commit rpc is done. On the second pass, `nfs_writebp()` is called to do
/// the job. `np` and `nmp` are the vnode's node and mount, lent.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function: the commit pass, the write pass and the wait"
)]
pub fn nfs_flush<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    nmp: &NfsMount<G>,
    _cred: Cred<'_>,
    waitfor: i32,
    p: Option<&Thread<G>>,
    commit: bool,
) -> Result<(), Errno> {
    let mut slptimeo = INFSLP;
    let mut slpflag = if nmp.flag() & NFSMNT_INT != 0 {
        PCATCH
    } else {
        0
    };
    let mut error: Result<(), Errno> = Ok(());
    let mut passone = commit;
    let v3 = nfs_isv3(nmp);
    // The byte range to commit, over every pass (the C's two locals, set once).
    let mut off = u64::MAX;
    let mut endoff = 0u64;

    // again:
    loop {
        if v3 && commit {
            // A list of these buffers is kept so that the second loop knows which buffers have
            // actually been committed. This is necessary, since there may be a race between
            // the commit rpc and new uncommitted writes on the file.
            let bvec: [Option<BufRef>; NFS_COMMITBVECSIZ] = vnode_dirty_gather(&vp.bufs, |bp| {
                bp.flags() & (B_BUSY | B_DELWRI | B_NEEDCOMMIT) == (B_DELWRI | B_NEEDCOMMIT)
            });
            let mut bvecpos = 0;
            for t in bvec.iter().flatten() {
                let bp = t.buf();
                bp.set(B_WRITEINPROG);
                let mut toff = (bp.blkno() as u64)
                    .wrapping_mul(DEV_BSIZE as u64)
                    .wrapping_add(bp.dirtyoff() as u64);
                if toff < off {
                    off = toff;
                }
                toff = toff.wrapping_add((bp.dirtyend() - bp.dirtyoff()) as u64);
                if toff > endoff {
                    endoff = toff;
                }
                bvecpos += 1;
            }
            if bvecpos > 0 {
                // Commit data on the server, as required.
                BCSTATS.pendingwrites.fetch_add(1, Ordering::Relaxed);
                BCSTATS.numwrites.fetch_add(1, Ordering::Relaxed);
                let retv = nfs_commit(vp, np, nmp, off, endoff.wrapping_sub(off) as i32, p);
                if retv.is_err_and(|e| e.raw() == NFSERR_STALEWRITEVERF) {
                    nfs_clearcommit(vmount(vp));
                }
                // Now, either mark the blocks I/O done or mark the blocks dirty, depending on
                // whether the commit succeeded.
                for (i, t) in bvec.into_iter().flatten().enumerate() {
                    let bp = t.buf();
                    bp.clr(B_NEEDCOMMIT | B_WRITEINPROG);
                    if retv.is_err() {
                        if i == 0 {
                            BCSTATS.pendingwrites.fetch_sub(1, Ordering::Relaxed);
                        }
                        brelse(t);
                    } else {
                        if i > 0 {
                            BCSTATS.pendingwrites.fetch_add(1, Ordering::Relaxed);
                        }
                        bp.set(B_ASYNC);
                        bp.clr(B_READ | B_DONE | B_ERROR);
                        bp.set_dirtyoff(0);
                        bp.set_dirtyend(0);
                        buf_undirty_done(t);
                    }
                }
            }
        }

        // Start/do any write(s) that are required.
        'scan: loop {
            // loop:
            loop {
                let pick = |bp: &crate::fs::buf::Buf| {
                    if bp.isset(B_BUSY) {
                        if waitfor != MNT_WAIT || passone {
                            return DirtyPick::Skip;
                        }
                        return DirtyPick::Wait;
                    }
                    if !bp.isset(B_DELWRI) {
                        kpanic(format_args!("nfs_fsync: not dirty"));
                    }
                    if (passone || !commit) && bp.isset(B_NEEDCOMMIT) {
                        return DirtyPick::Skip;
                    }
                    if passone || !commit {
                        bp.set(B_ASYNC);
                    } else {
                        bp.set(B_ASYNC | B_WRITEINPROG | B_NEEDCOMMIT);
                    }
                    DirtyPick::Take
                };
                match vnode_dirty_scan(&vp.bufs, "nfsfsync", slpflag, slptimeo, pick) {
                    DirtyScan::Taken(t) => {
                        let _ = vop_bwrite::<G>(IoBuf::Token(t));
                    }
                    DirtyScan::Waited(r) => {
                        error = r;
                        if error.is_err() {
                            if nfs_sigintr(nmp, None, p).is_err() {
                                return Err(Errno::EINTR);
                            }
                            if slpflag == PCATCH {
                                slpflag = 0;
                                slptimeo = crate::time::clock::sec_to_nsec(2);
                            }
                        }
                    }
                    DirtyScan::Done => break,
                }
            }
            if passone {
                break 'scan;
            }
            if waitfor == MNT_WAIT {
                // loop2:
                loop {
                    error = vwaitforio(vp, slpflag, "nfs_fsync", slptimeo);
                    if error.is_err() {
                        if nfs_sigintr(nmp, None, p).is_err() {
                            return Err(Errno::EINTR);
                        }
                        if slpflag == PCATCH {
                            slpflag = 0;
                            slptimeo = crate::time::clock::sec_to_nsec(2);
                        }
                        continue;
                    }
                    break;
                }
                let dirty = !vp.bufs.dirty_is_empty() && commit;
                if dirty {
                    // #if 0 vprint("nfs_fsync: dirty", vp); #endif
                    continue 'scan;
                }
            }
            break 'scan;
        }
        if passone {
            passone = false;
            continue;
        }
        break;
    }
    if np.isset(NWRITEERR) {
        error = errno_of(np.n_error.load(Ordering::Relaxed));
        np.clr(NWRITEERR);
    }
    error
}

/// `nfs_pathconf` (`vop_pathconf`): return POSIX pathconf information applicable to nfs. Fake
/// it. For v3 we could ask the server, but such code hasn't been written yet.
pub fn nfs_pathconf<G: ProcGlue>(
    vp: &'static Vnode<G>,
    name: i32,
    retval: &mut Register,
) -> Result<(), Errno> {
    let nmp = vfstonfs_vp(vp);
    let xfer = || {
        nmp.nm_rsize
            .load(Ordering::Relaxed)
            .min(nmp.nm_wsize.load(Ordering::Relaxed)) as Register
    };

    *retval = match name {
        _PC_LINK_MAX => LINK_MAX as Register,
        _PC_NAME_MAX => NAME_MAX as Register,
        _PC_CHOWN_RESTRICTED | _PC_NO_TRUNC | _PC_2_SYMLINKS => 1,
        _PC_ALLOC_SIZE_MIN => NFS_FABLKSIZE as Register,
        _PC_FILESIZEBITS => 64,
        _PC_REC_INCR_XFER_SIZE | _PC_REC_MIN_XFER_SIZE => xfer(),
        _PC_REC_MAX_XFER_SIZE => -1, // means ``unlimited''
        _PC_REC_XFER_ALIGN => PAGE_SIZE as Register,
        _PC_SYMLINK_MAX => MAXPATHLEN as Register,
        _PC_TIMESTAMP_RESOLUTION => {
            if nfs_isv3(&nmp) {
                1
            } else {
                1000
            }
        }
        _ => return Err(Errno::EINVAL),
    };
    Ok(())
}

/// `nfs_advlock` (`vop_advlock`): NFS advisory byte-level locks.
pub fn nfs_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    let np = vtonfs(vp);
    lf_advlock::<G>(
        &np.n_lockf,
        np.n_size.load(Ordering::Relaxed) as i64,
        id,
        op,
        fl,
        flags,
    )
}

/// `nfs_print` (`vop_print`): print out the contents of an nfsnode.
pub fn nfs_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    {
        let np = vtonfs(vp);
        let va = *np.n_vattr.read();
        hal::prf::printf(format_args!(
            "tag VT_NFS, fileid {} fsid 0x{:x}",
            va.va_fileid as i64, va.va_fsid
        ));
        // FIFO: fifo_printinfo(vp) of a fifo (miscfs/fifofs, not ported; no fifo is made).
        hal::prf::printf(format_args!("\n"));
    }
    let _ = vp;
    Ok(())
}

/// `nfs_bwrite` (`vop_bwrite`): just call `nfs_writebp()` with the force argument set to 1.
pub fn nfs_bwrite<G: ProcGlue>(bp: IoBuf) -> Result<(), Errno> {
    nfs_writebp::<G>(strategy_token(bp), true)
}

/// The vnode a buffer of the client belongs to (`bp->b_vp`).
pub(crate) fn buf_vnode<G: ProcGlue>(bp: &crate::fs::buf::Buf) -> &'static Vnode<G> {
    let vp = bp
        .vnode_bufs()
        .and_then(|vb| vb.owner().as_any().downcast_ref::<Vnode<G>>());
    match vp {
        Some(vp) => vp,
        None => kpanic(format_args!("nfs: buffer {bp:p} without a vnode")),
    }
}

/// `nfs_writebp(bp, force)`: a clone of `vop_generic_bwrite()`, except that `B_WRITEINPROG`
/// isn't set unless the force flag is one and it also handles the `B_NEEDCOMMIT` flag: a
/// buffer whose data the server has but not stably is committed (`nfs_commit`, with as much
/// of the node's to-be-committed range as covers it) rather than written again. A
/// synchronous write waits for the I/O and releases the buffer, answering the I/O's error.
pub fn nfs_writebp<G: ProcGlue>(t: BufRef, force: bool) -> Result<(), Errno> {
    let oldflags = t.buf().flags();
    let mut retv: Option<Result<(), NfsError>> = None;

    if oldflags & B_BUSY == 0 {
        kpanic(format_args!("bwrite: buffer is not busy???"));
    }

    let vp: &'static Vnode<G> = buf_vnode(t.buf());
    let np = vtonfs(vp);

    t.buf().clr(B_READ | B_DONE | B_ERROR);

    // A synchronous write waits for its I/O through a counted link (the token goes on).
    let r = if oldflags & B_ASYNC == 0 {
        match t.counted() {
            Some(r) => Some(r),
            None => kpanic(format_args!(
                "nfs_writebp: a driver's own buffer {:p} written synchronously",
                t.buf()
            )),
        }
    } else {
        None
    };

    buf_undirty_output(&t);
    if oldflags & B_ASYNC != 0 && oldflags & B_DELWRI == 0 {
        vp.bufs.owner().charge_io(true);
    }

    // If B_NEEDCOMMIT is set, a commit rpc may do the trick. If not an actual write will have
    // to be scheduled via. VOP_STRATEGY(). If B_WRITEINPROG is already set, then push it with
    // a write anyhow.
    let mut t = Some(t);
    if oldflags & (B_NEEDCOMMIT | B_WRITEINPROG) == B_NEEDCOMMIT
        && let Some(tk) = t.take()
    {
        let bp = tk.buf();
        let mut off = (bp.blkno() as u64)
            .wrapping_mul(DEV_BSIZE as u64)
            .wrapping_add(bp.dirtyoff() as u64);
        let mut cnt = i64::from(bp.dirtyend() - bp.dirtyoff());

        let lock = np.n_commitlock.write();
        if !bp.isset(B_NEEDCOMMIT) {
            // The C returns 0 with the buffer still busy and `v_numoutput` raised, which no
            // path reaches: the busy buffer is ours, so no one else clears its B_NEEDCOMMIT.
            kpanic(format_args!(
                "nfs_writebp: B_NEEDCOMMIT of busy buffer {bp:p} cleared"
            ));
        }

        // If it's already been committed by somebody else, bail.
        let commit = if nfs_in_committed_range(&np, bp) {
            // It has already been committed.
            Ok(())
        } else {
            let mut pushedrange = false;
            // Since we're going to do this, push as much as we can.
            if nfs_in_tobecommitted_range(&np, bp) {
                pushedrange = true;
                off = np.n_pushlo.load(Ordering::Relaxed) as u64;
                cnt = np.n_pushhi.load(Ordering::Relaxed) - np.n_pushlo.load(Ordering::Relaxed);
            }

            bp.set(B_WRITEINPROG);
            BCSTATS.pendingwrites.fetch_add(1, Ordering::Relaxed);
            BCSTATS.numwrites.fetch_add(1, Ordering::Relaxed);
            let nmp = vfstonfs_vp(vp);
            let rv = G::with_curproc(|p| nfs_commit(vp, &np, &nmp, off, cnt as i32, p));
            bp.clr(B_WRITEINPROG);

            if rv.is_ok() {
                if pushedrange {
                    nfs_merge_commit_ranges(&np);
                } else {
                    nfs_add_committed_range(&np, bp);
                }
            } else {
                BCSTATS.pendingwrites.fetch_sub(1, Ordering::Relaxed);
            }
            rv
        };

        drop(lock);
        match commit {
            Ok(()) => {
                bp.set_dirtyoff(0);
                bp.set_dirtyend(0);
                bp.clr(B_NEEDCOMMIT);
                biodone(tk);
            }
            Err(e) => {
                if e.raw() == NFSERR_STALEWRITEVERF {
                    nfs_clearcommit(vmount(vp));
                }
                t = Some(tk);
            }
        }
        retv = Some(commit);
    }
    if retv.is_none_or(|r| r.is_err())
        && let Some(tk) = t.take()
    {
        if force {
            tk.buf().set(B_WRITEINPROG);
        }
        let _ = nfs_strategy::<G>(IoBuf::Token(tk));
    }

    if let Some(r) = r {
        if let Some(bp) = r.get() {
            bp.set(B_RAW);
        }
        let (tk, rtval) = biowait(r);
        if oldflags & B_DELWRI == 0 {
            vp.bufs.owner().charge_io(true);
        }
        brelse(tk);
        return rtval;
    }

    Ok(())
}

/// `nfsspec_access` (`vop_access` of the special and fifo tables): nfs special file access
/// vnode op. Essentially just get vattr and then imitate iaccess() since the device is local
/// to the client.
pub fn nfsspec_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    // Disallow write attempts on filesystems mounted read-only; unless the file is a socket,
    // fifo, or a block or character device resident on the filesystem.
    if mode & VWRITE != 0 && rdonly(vp) {
        match vp.vtype() {
            VREG | VDIR | VLNK => return Err(Errno::EROFS),
            _ => {}
        }
    }

    let mut va = Vattr::new();
    nfs_getattr(vp, &mut va, cred, p)?;

    vaccess(
        vp.vtype(),
        va.va_mode,
        va.va_uid,
        va.va_gid,
        mode,
        ucred(cred),
    )
}

/// `nfsspec_read` (`vop_read`): read wrapper for special devices: set access flag.
pub fn nfsspec_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    let np = vtonfs(vp);
    np.set(NACC);
    let _ = np.n_atim.replace(getnanotime());
    drop(np);
    crate::fs::spec::spec_read(vp, uio, ioflag)
}

/// `nfsspec_write` (`vop_write`): write wrapper for special devices: set update flag.
pub fn nfsspec_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    let np = vtonfs(vp);
    np.set(NUPD);
    let _ = np.n_mtim.replace(getnanotime());
    drop(np);
    crate::fs::spec::spec_write(vp, uio, ioflag)
}

/// The times of a special file pushed on its last close (`nfsspec_close`'s and
/// `nfsfifo_close`'s): `NCHG` set, and when the vnode's last user closes it on a writable
/// mount, a `VOP_SETATTR` of the access and update times the flags say.
fn nfs_settimes_on_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    np: &NfsNode<G>,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) {
    np.set(NCHG);
    if vp.usecount() == 1 && !rdonly(vp) {
        let mut vattr = Vattr::new();
        vattr_null(&mut vattr);
        if np.isset(NACC) {
            vattr.va_atime = *np.n_atim.read();
        }
        if np.isset(NUPD) {
            vattr.va_mtime = *np.n_mtim.read();
        }
        let _ = with_thread(p, |p| vop_setattr(vp, &mut vattr, cred, p));
    }
}

/// `nfsspec_close` (`vop_close`): close wrapper for special devices. Update the times on the
/// nfsnode then do device close.
pub fn nfsspec_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fflag: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let np = vtonfs(vp);
    if np.isset(NACC | NUPD) {
        nfs_settimes_on_close(vp, &np, cred, p);
    }
    drop(np);
    crate::fs::spec::spec_close(vp, fflag, cred, p)
}

/// `nfsfifo_read` (`vop_read`): read wrapper for fifos: set access flag.
pub fn nfsfifo_read<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let np = vtonfs(vp);
    np.set(NACC);
    let _ = np.n_atim.replace(getnanotime());
    // fifo_read(ap): miscfs/fifofs, not ported.
    Err(unported!("fifo_read (miscfs/fifofs)"))
}

/// `nfsfifo_write` (`vop_write`): write wrapper for fifos: set update flag.
pub fn nfsfifo_write<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let np = vtonfs(vp);
    np.set(NUPD);
    let _ = np.n_mtim.replace(getnanotime());
    // fifo_write(ap): miscfs/fifofs, not ported.
    Err(unported!("fifo_write (miscfs/fifofs)"))
}

/// `nfsfifo_close` (`vop_close`): close wrapper for fifos. Update the times on the nfsnode
/// then do fifo close.
pub fn nfsfifo_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let np = vtonfs(vp);
    if np.isset(NACC | NUPD) {
        if np.isset(NACC) {
            let _ = np.n_atim.replace(getnanotime());
        }
        if np.isset(NUPD) {
            let _ = np.n_mtim.replace(getnanotime());
        }
        nfs_settimes_on_close(vp, &np, cred, p);
    }
    // fifo_close(ap): miscfs/fifofs, not ported.
    Err(unported!("fifo_close (miscfs/fifofs)"))
}

/// `nfsfifo_reclaim` (`vop_reclaim`): the fifo's reclaim, then the node's.
pub fn nfsfifo_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    // fifo_reclaim(v): miscfs/fifofs, not ported (its answer is ignored, as the C's).
    let _ = unported!("fifo_reclaim (miscfs/fifofs)");
    nfs_reclaim(vp)
}

/// The slots the three tables share (each the C's function of the same name): `getattr`,
/// `setattr`, `fsync`, `inactive`, `lock`, `unlock`, `print` and `islocked` (tmpfs's
/// `tmpfs_common_slots!` form).
macro_rules! nfs_common_slots {
    () => {
        fn getattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            cred: Cred<'_>,
            p: &Thread<G>,
        ) -> Result<(), Errno> {
            nfs_getattr(vp, vap, cred, p)
        }
        fn setattr(
            &self,
            vp: &'static Vnode<G>,
            vap: &mut Vattr,
            cred: Cred<'_>,
            p: &Thread<G>,
        ) -> Result<(), Errno> {
            nfs_setattr(vp, vap, cred, p)
        }
        fn fsync(
            &self,
            vp: &'static Vnode<G>,
            cred: Cred<'_>,
            waitfor: i32,
            p: &Thread<G>,
        ) -> Result<(), Errno> {
            nfs_fsync(vp, cred, waitfor, p)
        }
        fn inactive(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
            nfs_inactive(vp)
        }
        fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
            nfs_lock(vp, flags)
        }
        fn unlock(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
            nfs_unlock(vp)
        }
        fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
            nfs_print(vp)
        }
        fn islocked(&self, vp: &'static Vnode<G>) -> i32 {
            nfs_islocked(vp)
        }
    };
}

/// `nfs_vops`: each slot the C's (34); `vop_revoke` and `vop_abortop` the generic helpers the
/// C installs.
impl<G: ProcGlue> Vnops<G> for NfsVnops<G> {
    nfs_common_slots!();

    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        nfs_lookup(dvp, vpp, cnp)
    }
    fn create(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        nfs_create(dvp, vpp, cnp, vap)
    }
    fn mknod(
        &self,
        dvp: &'static Vnode<G>,
        _vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        nfs_mknod(dvp, cnp, vap)
    }
    fn open(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfs_open(vp, mode, cred, p)
    }
    fn close(
        &self,
        vp: &'static Vnode<G>,
        _fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        nfs_close(vp, cred, p)
    }
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        let np = vtonfs(vp);
        let nmp = vfstonfs_vp(vp);
        nfs_access(vp, &np, &nmp, mode, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        nfs_read(vp, uio, ioflag, cred)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        nfs_write(vp, uio, ioflag, cred)
    }
    fn ioctl(
        &self,
        _vp: &'static Vnode<G>,
        _command: u64,
        _data: &mut [u8],
        _fflag: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfs_ioctl()
    }
    fn kqfilter(
        &self,
        vp: &'static Vnode<G>,
        _fflag: i32,
        kn: &crate::file::event::Knote<G>,
    ) -> Result<(), Errno> {
        nfs_kqfilter(vp, kn)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn remove(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        nfs_remove(dvp, vp, cnp)
    }
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        nfs_link(dvp, vp, cnp)
    }
    fn rename(
        &self,
        fdvp: &'static Vnode<G>,
        fvp: &'static Vnode<G>,
        fcnp: &mut Cn<'_, G>,
        tdvp: &'static Vnode<G>,
        tvp: Option<&'static Vnode<G>>,
        tcnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        nfs_rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
    }
    fn mkdir(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        nfs_mkdir(dvp, vpp, cnp, vap)
    }
    fn rmdir(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        nfs_rmdir(dvp, vp, cnp)
    }
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        _vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        nfs_symlink(dvp, cnp, vap, target)
    }
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        nfs_readdir(vp, uio, cred, eofflag)
    }
    fn readlink(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        nfs_readlink(vp, uio, cred)
    }
    fn abortop(&self, _dvp: &'static Vnode<G>, cnp: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_abortop(cnp)
    }
    fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        nfs_reclaim(vp)
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        _runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        nfs_bmap(vp, bn, vpp, bnp)
    }
    fn strategy(&self, _vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        nfs_strategy::<G>(bp)
    }
    fn pathconf(
        &self,
        vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        nfs_pathconf(vp, name, retval)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        nfs_advlock(vp, id, op, fl, flags)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        nfs_bwrite::<G>(bp)
    }
}

/// `nfs_specvops`: the client's slots of a device, and `spec_vops`' others (the C's "XXX:
/// Keep in sync with spec_vops").
impl<G: ProcGlue> Vnops<G> for NfsSpecVnops<G> {
    nfs_common_slots!();

    fn close(
        &self,
        vp: &'static Vnode<G>,
        fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        nfsspec_close(vp, fflag, cred, p)
    }
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfsspec_access(vp, mode, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        nfsspec_read(vp, uio, ioflag)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        nfsspec_write(vp, uio, ioflag)
    }
    fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        nfs_reclaim(vp)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }
    // XXX: Keep in sync with spec_vops (one macro: the same slots as SpecVops's).
    crate::fs::spec::spec_shared_slots!();
}

/// `nfs_fifovops`: the client's slots of a fifo, and `fifo_vops`' others (the C's "XXX: Keep
/// in sync with fifo_vops"; fifofs's are visible gaps, the module's Deviations).
impl<G: ProcGlue> Vnops<G> for NfsFifoVnops<G> {
    nfs_common_slots!();

    fn close(
        &self,
        vp: &'static Vnode<G>,
        _fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        nfsfifo_close(vp, cred, p)
    }
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfsspec_access(vp, mode, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        _uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        nfsfifo_read(vp)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        _uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        nfsfifo_write(vp)
    }
    fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        nfsfifo_reclaim(vp)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }

    // XXX: Keep in sync with fifo_vops.
    fn lookup(
        &self,
        _dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        _cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_lookup(vpp)
    }
    fn create(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mknod(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn open(
        &self,
        _vp: &'static Vnode<G>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_open (miscfs/fifofs)"))
    }
    fn ioctl(
        &self,
        _vp: &'static Vnode<G>,
        _command: u64,
        _data: &mut [u8],
        _fflag: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_ioctl (miscfs/fifofs)"))
    }
    fn kqfilter(
        &self,
        _vp: &'static Vnode<G>,
        _fflag: i32,
        _kn: &crate::file::event::Knote<G>,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_kqfilter (miscfs/fifofs)"))
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn remove(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn link(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rename(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
        _: &'static Vnode<G>,
        _: Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mkdir(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rmdir(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn symlink(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
        _: &[u8],
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn readdir(
        &self,
        _: &'static Vnode<G>,
        _: &mut Uio<'_, G>,
        _: Cred<'_>,
        _: &mut i32,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn readlink(&self, _: &'static Vnode<G>, _: &mut Uio<'_, G>, _: Cred<'_>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn abortop(&self, _: &'static Vnode<G>, _: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        default::vop_generic_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, _: &'static Vnode<G>, _: IoBuf) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn pathconf(
        &self,
        _vp: &'static Vnode<G>,
        _name: i32,
        _retval: &mut Register,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_pathconf (miscfs/fifofs)"))
    }
    fn advlock(
        &self,
        _vp: &'static Vnode<G>,
        _id: LockOwner,
        _op: i32,
        _fl: &mut Flock,
        _flags: i32,
    ) -> Result<(), Errno> {
        Err(unported!("fifo_advlock (miscfs/fifofs)"))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the vnode operations: the packer and the small helpers alone, then the
    // operations against testnfs's scripted server on 127.0.0.1 (a mount of the fixture's
    // glue whose nodes are the client's, `nfs_nget`'s, over the fixture's buffer cache).

    use std::boxed::Box;
    use std::sync::{Arc, Mutex};
    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::file::uio::Iovec;
    use crate::fs::namei::Componentname;
    use crate::fs::nfs::nfs_subs::nfs_vfs_init;
    use crate::fs::nfs::nfsproto::{NFDIR, NFLNK, NFREG};
    use crate::fs::nfs::testnfs::{self as tn, Out};
    use crate::net::mbuf::MBUFPL;
    use sysdefs::socket::SOCK_DGRAM;

    /// The silly name is `.nfs` and the two words in upper-case hex, 20 bytes.
    #[test]
    fn the_silly_name_is_the_c_s_format() {
        let (n, len) = nfs_sillyname([0x0123_abcd, 0xfedc_ba98]);
        assert_eq!(len, 20);
        assert_eq!(&n[..20], b".nfs0123ABCDFEDCBA98");
        assert!(n[20..].iter().all(|&b| b == 0));
    }

    /// The access bits a check asks: a directory's write is modify, extend and delete, its
    /// exec lookup; a file's exec is execute.
    #[test]
    fn the_access_bits_follow_the_vnode_type() {
        let rwx = VREAD | VWRITE | VEXEC;
        assert_eq!(
            nfs_access_mode(VDIR, rwx),
            NFSV3ACCESS_READ
                | NFSV3ACCESS_MODIFY
                | NFSV3ACCESS_EXTEND
                | NFSV3ACCESS_DELETE
                | NFSV3ACCESS_LOOKUP
        );
        assert_eq!(
            nfs_access_mode(VREG, rwx),
            NFSV3ACCESS_READ | NFSV3ACCESS_MODIFY | NFSV3ACCESS_EXTEND | NFSV3ACCESS_EXECUTE
        );
        assert_eq!(nfs_access_mode(VREG, 0), 0);
    }

    /// A xorshift generator for the packer's property test.
    fn next(s: &mut u64) -> u64 {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    }

    /// Whatever names come, the packer never writes past its buffer, every record lies in
    /// one `NFS_READDIRBLKSIZ` block, and the records (the last grown to its block's end)
    /// tile what was packed.
    #[test]
    fn the_packer_tiles_its_blocks_and_stays_in_the_buffer() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..2000 {
            let mut buf = vec![0u8; DIRBLKSIZ];
            let start = if next(&mut seed) % 4 == 0 {
                READDIRBLKSIZ
            } else {
                0
            };
            let mut pack = DirPack::new(&mut buf, start);
            let mut begun = Vec::new();
            loop {
                let len = 1 + (next(&mut seed) % NFS_MAXNAMLEN as u64) as usize;
                if !pack.begin(7, len) {
                    break;
                }
                let at = pack.dp.expect("the record begun");
                pack.name_slot(len).fill(b'a');
                pack.end_name(len);
                begun.push(at);
            }
            pack.finish();
            let end = pack.pos;
            assert!(end <= DIRBLKSIZ);
            let mut off = start;
            for &at in &begun {
                // A record that did not fit its block's rest began the next block, the
                // previous one grown to fill the gap: the records stay contiguous.
                assert_eq!(at, off, "a gap before the record at {at}");
                let reclen = usize::from(u16::from_ne_bytes([
                    buf[at + ND_RECLEN],
                    buf[at + ND_RECLEN + 1],
                ]));
                assert_eq!(at / READDIRBLKSIZ, (at + reclen - 1) / READDIRBLKSIZ);
                let namlen = usize::from(buf[at + ND_NAMLEN]);
                assert_eq!(buf[at + NFS_DIRHDSIZ + namlen], 0, "NUL-terminated");
                off += reclen;
            }
            assert_eq!(off, end, "the records tile what was packed");
        }
    }

    /// `fsinfo_apply`'s clamps: preferred sizes round up, maxima down (to themselves when they
    /// round to 0), the readdir size is clamped by the read maximum (the C's reuse), and a
    /// preferred size near 2^32 wraps as the C's unsigned add instead of overflowing.
    #[test]
    fn fsinfo_shrinks_the_sizes_as_the_c() {
        let tm = crate::fs::nfs::nfs_node::test_mount(NFSMNT_NFSV3);
        let nmp = &tm.nmp;
        let set = |w, r, d| {
            nmp.nm_wsize.store(w, Ordering::Relaxed);
            nmp.nm_rsize.store(r, Ordering::Relaxed);
            nmp.nm_readdirsize.store(d, Ordering::Relaxed);
        };
        let fs =
            |wp: u32, wm: u32, rp: u32, rm: u32, dp: u32| super::super::nfsproto::Nfsv3Fsinfo {
                fs_wtpref: wp.to_be(),
                fs_wtmax: wm.to_be(),
                fs_rtpref: rp.to_be(),
                fs_rtmax: rm.to_be(),
                fs_dtpref: dp.to_be(),
                ..Default::default()
            };
        let get = || {
            (
                nmp.nm_wsize.load(Ordering::Relaxed),
                nmp.nm_rsize.load(Ordering::Relaxed),
                nmp.nm_readdirsize.load(Ordering::Relaxed),
            )
        };
        set(8192, 8192, 8192);
        super::super::nfs_vfsops::fsinfo_apply(nmp, &fs(4000, 65536, 3000, 65536, 1500));
        assert_eq!(get(), (4096, 3072, 2048));
        assert_ne!(nmp.flag() & NFSMNT_GOTFSINFO, 0);
        set(8192, 8192, 8192);
        super::super::nfs_vfsops::fsinfo_apply(nmp, &fs(65536, 300, 65536, 1000, 65536));
        assert_eq!(
            get(),
            (300, 512, 1000),
            "maxima: 300 rounds to 0, 1000 to 512"
        );
        set(8192, 8192, 8192);
        super::super::nfs_vfsops::fsinfo_apply(
            nmp,
            &fs(u32::MAX - 100, 65536, 65536, 65536, 65536),
        );
        assert_eq!(
            get(),
            (8192, 8192, 8192),
            "sizes above the mount's change nothing"
        );
        set(8192, 8192, 8192);
        super::super::nfs_vfsops::fsinfo_apply(nmp, &fs(0, 0, 0, 0, 0));
        assert_eq!(get(), (8192, 8192, 8192), "a size of 0 keeps the mount's");
        set(8192, 8192, 8192);
        super::super::nfs_vfsops::fsinfo_apply(nmp, &fs(65536, 0, 65536, 0, 65536));
        assert_eq!(get(), (8192, 8192, 8192), "a maximum of 0 too");
    }

    // The scripted server's tests.

    /// The root's handle.
    const ROOTFH: [u8; 8] = [1; 8];
    /// A file's handle.
    const FILEFH: [u8; 8] = [2; 8];

    /// A test's mount: the fixture, the client's structure, the root directory's node (its
    /// vnode locked, `nfs_nget`'s), and the mbufs out before the test.
    struct Nt {
        fx: tn::Fixture,
        nmp: NfsMountRef<tn::G>,
        dnp: NfsNodeRef<tn::G>,
        mbufs: u64,
    }

    impl Nt {
        /// The root's vnode.
        fn dvp(&self) -> &'static Vnode<tn::G> {
            nfstov(&self.dnp)
        }

        /// The requester's credential.
        fn cred(&self) -> Cred<'static> {
            Cred::real(self.fx.user.ucred())
        }

        /// The mount taken down: the network as the test found it.
        fn done(self, peer: tn::Peer) {
            peer.stop();
            let Nt {
                fx,
                nmp,
                dnp,
                mbufs,
            } = self;
            drop(dnp);
            tn::unmount(&fx, nmp);
            tn::pump(fx.lo);
            assert_eq!(MBUFPL.nout(), mbufs, "every mbuf back");
            drop(fx);
        }
    }

    /// A version 3 mount (`flag` more) over UDP with attribute cache bounds of 3..60 s, its
    /// root a directory.
    fn setup(flag: i32) -> Nt {
        let fx = tn::setup(1000);
        // A wall clock past the servers' times (the attribute cache's stamps are seconds; 0 is
        // "no attributes").
        if crate::time::tc::gettime() == 0 {
            crate::time::tc::TIME_SECOND.store(1_000_000, Ordering::Relaxed);
        }
        let mbufs = MBUFPL.nout();
        nfs_vfs_init::<tn::G>(fx.mp.vfc()).expect("nfs_vfs_init");
        let nmp = tn::nfsmount(&fx, SOCK_DGRAM, flag);
        for (a, v) in [
            (&nmp.nm_acregmin, 3),
            (&nmp.nm_acregmax, 60),
            (&nmp.nm_acdirmin, 30),
            (&nmp.nm_acdirmax, 60),
        ] {
            a.store(v, Ordering::Relaxed);
        }
        assert_eq!(tn::connect(&nmp), Ok(()));
        let dnp = nfs_nget(fx.mp, &ROOTFH).expect("the root");
        nfstov(&dnp).set_vtype(VDIR);
        // The modification time the servers' directory attributes carry.
        let _ = dnp.n_mtime.replace(crate::time::clock::Timespec {
            tv_sec: 1000,
            tv_nsec: 0,
        });
        Nt {
            fx,
            nmp,
            dnp,
            mbufs,
        }
    }

    /// One XDR word.
    fn w(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_be_bytes());
    }

    /// A version 3 `fattr3` of the type `t`, mode, size, file id, modification and change
    /// second and owner.
    fn fattr3(t: u32, mode: u32, size: u64, fileid: u64, mtime: u32, uid: u32) -> Vec<u8> {
        let mut v = Vec::new();
        for x in [t, mode, 1, uid, 0] {
            w(&mut v, x);
        }
        for q in [size, size, 0, 7, fileid] {
            v.extend_from_slice(&q.to_be_bytes());
        }
        for x in [mtime, 0, mtime, 0, mtime, 0] {
            w(&mut v, x);
        }
        assert_eq!(v.len(), NFSX_V3FATTR);
        v
    }

    /// A `post_op_attr` (`None`: no attributes).
    fn post(a: Option<Vec<u8>>) -> Vec<u8> {
        let mut v = Vec::new();
        match a {
            Some(a) => {
                w(&mut v, 1);
                v.extend(a);
            }
            None => w(&mut v, 0),
        }
        v
    }

    /// A `wcc_data` with no attributes either side.
    fn wcc() -> Vec<u8> {
        vec![0; 8]
    }

    /// An `nfs_fh3`.
    fn fh3(b: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        w(&mut v, b.len() as u32);
        v.extend_from_slice(b);
        v.resize(v.len() + nfsm_padlen(b.len()), 0);
        v
    }

    /// The procedure of a call.
    fn proc_of(c: &[u8]) -> u32 {
        u32::from_be_bytes(c[20..24].try_into().unwrap())
    }

    /// The arguments of a call, after its credential and verifier.
    fn args_of(c: &[u8]) -> &[u8] {
        let be = |o: usize| u32::from_be_bytes(c[o..o + 4].try_into().unwrap()) as usize;
        let o = 32 + nfsm_rndup(be(28));
        &c[o + 8 + nfsm_rndup(be(o + 4))..]
    }

    /// The XDR string at `off` of `a` and the offset after it.
    fn xstr(a: &[u8], off: usize) -> (Vec<u8>, usize) {
        let n = u32::from_be_bytes(a[off..off + 4].try_into().unwrap()) as usize;
        (a[off + 4..off + 4 + n].to_vec(), off + 4 + nfsm_rndup(n))
    }

    /// A server answering by `f(procedure, arguments)`: a status and the results.
    fn peer(nt: &Nt, mut f: impl FnMut(u32, &[u8]) -> (u32, Vec<u8>) + Send + 'static) -> tn::Peer {
        tn::Peer::start(
            &nt.fx,
            SOCK_DGRAM,
            &nt.nmp,
            Box::new(move |_, c| {
                let (st, body) = f(proc_of(c), args_of(c));
                vec![Out::Data(tn::reply(c, 0, 0, st, &body))]
            }),
        )
    }

    /// The procedures of the calls a server saw.
    fn procs(peer: &tn::Peer) -> Vec<u32> {
        peer.calls().iter().map(|(_, c)| proc_of(c)).collect()
    }

    /// A GETATTR loads the cache; a second one within the timeout is answered from it.
    #[test]
    fn getattr_loads_the_cache_and_answers_from_it() {
        let nt = setup(NFSMNT_NFSV3);
        let p = peer(&nt, |pr, _| {
            assert_eq!(pr, NFSPROC_GETATTR as u32);
            (0, fattr3(NFDIR as u32, 0o755, 512, 2, 1000, 1000))
        });
        let mut va = Vattr::new();
        assert_eq!(
            nfs_getattr(nt.dvp(), &mut va, nt.cred(), nt.fx.user),
            Ok(())
        );
        let mut va2 = Vattr::new();
        assert_eq!(
            nfs_getattr(nt.dvp(), &mut va2, nt.cred(), nt.fx.user),
            Ok(())
        );
        assert_eq!(procs(&p), [NFSPROC_GETATTR as u32]);
        assert_eq!(
            (va.va_type, va.va_mode, va.va_size, va.va_fileid, va.va_uid),
            (VDIR, 0o755, 512, 2, 1000)
        );
        assert_eq!(va2.va_mtime, va.va_mtime);
        nt.done(p);
    }

    /// A failed GETATTR (a status, or attributes cut short) leaves the caller's attributes as
    /// they were: the kq poller compares them after an error, as the C's.
    #[test]
    fn a_failed_getattr_leaves_the_attributes() {
        for (status, want) in [(NFSERR_IO as u32, Errno::EIO), (0, Errno::EBADRPC)] {
            let nt = setup(NFSMNT_NFSV3);
            let p = peer(&nt, move |_, _| (status, vec![0; 8]));
            let mut va = Vattr::new();
            va.va_size = 77;
            va.va_mode = 0o123;
            assert_eq!(
                nfs_getattr(nt.dvp(), &mut va, nt.cred(), nt.fx.user),
                Err(want)
            );
            assert_eq!((va.va_size, va.va_mode), (77, 0o123));
            nt.done(p);
        }
    }

    /// A version 3 ACCESS is asked once for a uid and a mode; the same check is answered from
    /// the access cache, a refused bit is `EACCES`, and new attributes with another owner
    /// invalidate the cache (`nfs_loadattrcache`).
    #[test]
    fn access_is_asked_once_then_cached() {
        let nt = setup(NFSMNT_NFSV3);
        let grant = Arc::new(Mutex::new(NFSV3ACCESS_READ | NFSV3ACCESS_LOOKUP));
        let g2 = grant.clone();
        let p = peer(&nt, move |pr, _| match pr as usize {
            NFSPROC_ACCESS => {
                let mut v = post(Some(fattr3(NFDIR as u32, 0o755, 512, 2, 1000, 1000)));
                w(&mut v, *g2.lock().unwrap());
                (0, v)
            }
            _ => (0, fattr3(NFDIR as u32, 0o755, 512, 2, 1000, 7)),
        });
        let (vp, np, nmp) = (nt.dvp(), &nt.dnp, &nt.nmp);
        let user = nt.fx.user;
        assert_eq!(nfs_access(vp, np, nmp, VEXEC, nt.cred(), user), Ok(()));
        assert_eq!(nfs_access(vp, np, nmp, VEXEC, nt.cred(), user), Ok(()));
        assert_eq!(procs(&p).len(), 1, "the second check from the cache");
        assert_eq!(
            nfs_access(vp, np, nmp, VWRITE, nt.cred(), user),
            Err(Errno::EACCES)
        );
        assert_eq!(procs(&p).len(), 2);
        assert_eq!(
            nfs_access(vp, np, nmp, VWRITE, nt.cred(), user),
            Err(Errno::EACCES),
            "the refusal cached"
        );
        assert_eq!(procs(&p).len(), 2);
        // New attributes with another owner throw the cache away.
        nfs_invalidate_attrcache(np);
        let mut va = Vattr::new();
        assert_eq!(nfs_getattr(vp, &mut va, nt.cred(), user), Ok(()));
        assert_eq!(np.n_accstamp.load(Ordering::Relaxed), -1);
        *grant.lock().unwrap() = NFSV3ACCESS_READ | NFSV3ACCESS_LOOKUP | NFSV3ACCESS_MODIFY;
        assert_eq!(nfs_access(vp, np, nmp, VEXEC, nt.cred(), user), Ok(()));
        assert_eq!(
            procs(&p),
            [
                NFSPROC_ACCESS as u32,
                NFSPROC_ACCESS as u32,
                NFSPROC_GETATTR as u32,
                NFSPROC_ACCESS as u32
            ]
        );
        nt.done(p);
    }

    /// A componentname of the requester for `op` with `flags`, the name `name`.
    fn cn(nt: &Nt, op: u64, flags: u64, name: &'static [u8]) -> Componentname<'static, tn::G> {
        let mut c = Componentname::new(op, flags, nt.fx.user, nt.cred());
        c.set_name_lit(name);
        c
    }

    /// A LOOKUP sends the directory's handle and the name; the node made from the reply's
    /// handle has the attributes' type, and the name goes in the cache, so a second lookup
    /// (once the vnode is released) is answered there (an ACCESS of the directory, no
    /// LOOKUP).
    #[test]
    fn lookup_makes_the_node_and_caches_the_name() {
        let nt = setup(NFSMNT_NFSV3);
        let p = peer(&nt, |pr, a| match pr as usize {
            NFSPROC_LOOKUP => {
                let (fh, o) = xstr(a, 0);
                assert_eq!(fh, ROOTFH);
                assert_eq!(xstr(a, o).0, b"file");
                let mut v = fh3(&FILEFH);
                v.extend(post(Some(fattr3(NFREG as u32, 0o644, 100, 3, 1000, 1000))));
                v.extend(post(None));
                (0, v)
            }
            NFSPROC_ACCESS => {
                let mut v = post(None);
                w(&mut v, 0x3f);
                (0, v)
            }
            _ => (NFSERR_IO as u32, post(None)),
        });
        let mut vpp = None;
        let mut c = cn(&nt, LOOKUP, LOCKPARENT | ISLASTCN | MAKEENTRY, b"file");
        assert_eq!(nfs_lookup(nt.dvp(), &mut vpp, &mut c), Ok(()));
        let vp = vpp.expect("the file");
        assert_eq!(vp.vtype(), VREG);
        assert_eq!(vtonfs(vp).fh().bytes(), FILEFH);
        assert_eq!(vtonfs(vp).n_size.load(Ordering::Relaxed), 100);
        vput(vp);

        let mut vpp = None;
        let mut c = cn(&nt, LOOKUP, LOCKPARENT | ISLASTCN | MAKEENTRY, b"file");
        assert_eq!(nfs_lookup(nt.dvp(), &mut vpp, &mut c), Ok(()));
        assert!(
            vpp.is_some_and(|v| ptr::eq(v, vp)),
            "the same vnode, from the cache"
        );
        assert_eq!(procs(&p), [NFSPROC_LOOKUP as u32, NFSPROC_ACCESS as u32]);
        vput(vp);
        nt.done(p);
    }

    /// A name the server does not have is `ENOENT` for a LOOKUP, entered in the cache as a
    /// negative entry (`MAKEENTRY`), and `EJUSTRETURN` for the last component of a CREATE.
    #[test]
    fn a_missing_name_is_enoent_or_ejustreturn_for_a_create() {
        let nt = setup(NFSMNT_NFSV3);
        let p = peer(&nt, |_, _| (Errno::ENOENT as u32, post(None)));
        let mut vpp = None;
        let mut c = cn(&nt, LOOKUP, LOCKPARENT | ISLASTCN | MAKEENTRY, b"none");
        assert_eq!(nfs_lookup(nt.dvp(), &mut vpp, &mut c), Err(Errno::ENOENT));
        assert!(vpp.is_none());
        let mut c = cn(&nt, CREATE, LOCKPARENT | ISLASTCN, b"new");
        assert_eq!(
            nfs_lookup(nt.dvp(), &mut vpp, &mut c),
            Err(Errno::EJUSTRETURN)
        );
        assert_ne!(c.cn_flags & SAVENAME, 0);
        assert_eq!(procs(&p).len(), 2);
        nt.done(p);
    }

    /// An exclusive create the server does not support (`NFSERR_NOTSUPP`, D-3) is retried
    /// unchecked: two CREATEs, the first `EXCLUSIVE` with its verifier, the second
    /// `UNCHECKED` with the attributes; the node comes from the second reply.
    #[test]
    fn an_exclusive_create_is_retried_unchecked() {
        let nt = setup(NFSMNT_NFSV3);
        let p = peer(&nt, |pr, a| {
            assert_eq!(pr, NFSPROC_CREATE as u32);
            let (_, o) = xstr(a, 0);
            let (name, o) = xstr(a, o);
            assert_eq!(name, b"x");
            let how = u32::from_be_bytes(a[o..o + 4].try_into().unwrap());
            if how == NFSV3CREATE_EXCLUSIVE {
                (NFSERR_NOTSUPP as u32, wcc())
            } else {
                let mut v = Vec::new();
                w(&mut v, 1);
                v.extend(fh3(&FILEFH));
                v.extend(post(Some(fattr3(NFREG as u32, 0o600, 0, 3, 1000, 1000))));
                v.extend(wcc());
                (0, v)
            }
        });
        let mut vpp = None;
        let mut c = cn(&nt, CREATE, LOCKPARENT | ISLASTCN, b"x");
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_type = VREG;
        va.va_mode = 0o600;
        va.va_vaflags = VA_EXCLUSIVE;
        assert_eq!(nfs_create(nt.dvp(), &mut vpp, &mut c, &mut va), Ok(()));
        let vp = vpp.expect("the file");
        assert_eq!(vp.vtype(), VREG);
        let hows: Vec<u32> = p
            .calls()
            .iter()
            .map(|(_, c)| {
                let a = args_of(c);
                let (_, o) = xstr(a, 0);
                let (_, o) = xstr(a, o);
                u32::from_be_bytes(a[o..o + 4].try_into().unwrap())
            })
            .collect();
        assert_eq!(hows, [NFSV3CREATE_EXCLUSIVE, NFSV3CREATE_UNCHECKED]);
        assert!(nt.dnp.isset(NMODIFIED));
        vput(vp);
        nt.done(p);
    }

    /// A regular file of the root: its node by its handle, its type and size set, read and
    /// write credentials the requester's.
    fn file(nt: &Nt, size: u64) -> NfsNodeRef<tn::G> {
        let np = nfs_nget(nt.fx.mp, &FILEFH).expect("the file");
        let vp = nfstov(&np);
        vp.set_vtype(VREG);
        np.n_size.store(size, Ordering::Relaxed);
        np.n_vattr.update(|v| {
            v.va_type = VREG;
            v.va_size = size;
        });
        nfs_setcred(&np, nt.fx.user.ucred(), false);
        nfs_setcred(&np, nt.fx.user.ucred(), true);
        np
    }

    /// A kernel read of `len` bytes at `off` into `buf`.
    fn kread(
        vp: &'static Vnode<tn::G>,
        nt: &Nt,
        off: i64,
        buf: &mut [u8],
    ) -> (Result<(), Errno>, usize) {
        crate::testutil::set_copy_kernel(true);
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, buf, Some(nt.fx.user));
        uio.uio_offset = off;
        let r = nfs_read(vp, &mut uio, 0, nt.cred());
        (r, uio.uio_resid)
    }

    /// A version 3 READ reply of `data` (`eof` its flag).
    fn read3(data: &[u8], eof: bool) -> Vec<u8> {
        let mut v = post(None);
        w(&mut v, data.len() as u32);
        w(&mut v, u32::from(eof));
        w(&mut v, data.len() as u32);
        v.extend_from_slice(data);
        v.resize(v.len() + nfsm_padlen(data.len()), 0);
        v
    }

    /// A read goes through the buffer cache: one READ of the block, the bytes as the server
    /// sent them (a short block zero-filled to the file's size); a second read of the same
    /// block is answered from the cache.
    #[test]
    fn a_read_fills_a_cache_block_once() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 300);
        let vp = nfstov(&np);
        let p = peer(&nt, |pr, a| match pr as usize {
            NFSPROC_READ => {
                let (_, o) = xstr(a, 0);
                assert_eq!(&a[o..o + 8], &0u64.to_be_bytes(), "offset 0");
                (0, read3(&[b'z'; 200], true))
            }
            _ => (0, fattr3(NFREG as u32, 0o644, 300, 3, 1000, 1000)),
        });
        let mut buf = [0u8; 300];
        let (r, resid) = kread(vp, &nt, 0, &mut buf);
        assert_eq!(r, Ok(()));
        assert_eq!(resid, 0);
        assert!(buf[..200].iter().all(|&b| b == b'z'));
        assert!(buf[200..].iter().all(|&b| b == 0), "the hole zero-filled");
        let mut buf2 = [0u8; 50];
        assert_eq!(kread(vp, &nt, 10, &mut buf2), (Ok(()), 0));
        assert!(buf2.iter().all(|&b| b == b'z'));
        assert_eq!(
            procs(&p),
            [NFSPROC_GETATTR as u32, NFSPROC_READ as u32],
            "the attributes once, the block once"
        );
        vput(vp);
        nt.done(p);
    }

    /// A kernel write of `data` at `off`.
    fn kwrite(
        vp: &'static Vnode<tn::G>,
        nt: &Nt,
        off: i64,
        data: &[u8],
        ioflag: i32,
    ) -> Result<(), Errno> {
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_write(&mut iov, data, Some(nt.fx.user));
        uio.uio_offset = off;
        nfs_write(vp, &mut uio, ioflag, nt.cred())
    }

    /// The `stable` argument and the data of a version 3 WRITE call.
    fn write_args(a: &[u8]) -> (u64, u32, Vec<u8>) {
        let (_, o) = xstr(a, 0);
        let off = u64::from_be_bytes(a[o..o + 8].try_into().unwrap());
        let stable = u32::from_be_bytes(a[o + 12..o + 16].try_into().unwrap());
        (off, stable, xstr(a, o + 16).0)
    }

    /// A version 3 WRITE reply: the count written, its commitment, the verifier.
    fn write3(count: u32, committed: u32, verf: [u8; 8]) -> Vec<u8> {
        let mut v = wcc();
        w(&mut v, count);
        w(&mut v, committed);
        v.extend_from_slice(&verf);
        v
    }

    /// A partial block is a delayed write; `fsync` writes it `UNSTABLE` (the first pass),
    /// then commits it (the second): WRITE, COMMIT, the block clean after. When the COMMIT's
    /// verifier differs from the WRITE's (a server reboot), the block is written again
    /// `FILESYNC` and the mount takes the new verifier.
    #[test]
    fn fsync_writes_unstable_then_commits_and_rewrites_on_a_new_verifier() {
        for reboot in [false, true] {
            let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
            let np = file(&nt, 0);
            let vp = nfstov(&np);
            let seen = Arc::new(Mutex::new(Vec::new()));
            let s2 = seen.clone();
            // The server's verifier: a reboot between the WRITE and the COMMIT changes it.
            let mut verf = [1u8; 8];
            let p = peer(&nt, move |pr, a| match pr as usize {
                NFSPROC_WRITE => {
                    let (off, stable, data) = write_args(a);
                    s2.lock().unwrap().push((off, stable, data.len()));
                    let committed = if stable == NFSV3WRITE_UNSTABLE {
                        NFSV3WRITE_UNSTABLE
                    } else {
                        NFSV3WRITE_FILESYNC
                    };
                    (0, write3(data.len() as u32, committed, verf))
                }
                NFSPROC_COMMIT => {
                    if reboot {
                        verf = [2; 8];
                    }
                    let mut v = wcc();
                    v.extend_from_slice(&verf);
                    (0, v)
                }
                _ => (0, fattr3(NFREG as u32, 0o644, 0, 3, 1000, 1000)),
            });
            assert_eq!(kwrite(vp, &nt, 0, &[b'w'; 100], 0), Ok(()));
            assert!(p.calls().is_empty(), "a delayed write");
            assert_eq!(np.n_size.load(Ordering::Relaxed), 100);
            let bios = NFSSTATS.write_bios.load(Ordering::Relaxed);
            assert_eq!(nfs_fsync(vp, nt.cred(), MNT_WAIT, nt.fx.user), Ok(()));
            let want: Vec<u32> = if reboot {
                vec![
                    NFSPROC_WRITE as u32,
                    NFSPROC_COMMIT as u32,
                    NFSPROC_WRITE as u32,
                ]
            } else {
                vec![NFSPROC_WRITE as u32, NFSPROC_COMMIT as u32]
            };
            assert_eq!(procs(&p), want, "reboot {reboot}");
            let writes = seen.lock().unwrap().clone();
            assert_eq!(writes[0], (0, NFSV3WRITE_UNSTABLE, 100));
            if reboot {
                assert_eq!(writes[1], (0, NFSV3WRITE_FILESYNC, 100));
                assert_eq!(nt.nmp.verf(), [2; 8], "the new verifier");
            } else {
                assert_eq!(nt.nmp.verf(), [1; 8]);
            }
            assert!(vp.bufs.dirty_is_empty(), "the block clean");
            // One block write per WRITE: a committed block is done, never written again.
            assert_eq!(
                NFSSTATS.write_bios.load(Ordering::Relaxed) - bios,
                writes.len() as u64
            );
            vput(vp);
            nt.done(p);
        }
    }

    /// A WRITE reply that wrote nothing (`rlen <= 0`) is `NFSERR_IO`, the buffer's error: a
    /// synchronous write answers `EIO`.
    #[test]
    fn a_write_reply_of_no_bytes_is_eio() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        let p = peer(&nt, |pr, _| match pr as usize {
            NFSPROC_WRITE => (0, write3(0, NFSV3WRITE_FILESYNC, [1; 8])),
            _ => (0, fattr3(NFREG as u32, 0o644, 0, 3, 1000, 1000)),
        });
        assert_eq!(
            kwrite(vp, &nt, 0, &[b'w'; 10], crate::fs::vnode::IO_SYNC),
            Err(Errno::EIO)
        );
        assert!(np.isset(NWRITEERR));
        vput(vp);
        nt.done(p);
    }

    /// A version 3 WRITE's `wcc_data`: whatever the pre-op time says, `n_mtime` becomes the
    /// post-op attributes' modification time (the C sets `wccflag` again before the results),
    /// so the next open finds the file unchanged.
    #[test]
    fn a_v3_write_takes_the_post_op_mtime() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        let _ = np.n_mtime.replace(crate::time::clock::Timespec {
            tv_sec: 1000,
            tv_nsec: 0,
        });
        let p = peer(&nt, |pr, a| match pr as usize {
            NFSPROC_WRITE => {
                let (_, _, data) = write_args(a);
                // wcc_data: pre-op (size, mtime 1000, ctime), then post-op of mtime 1001.
                let mut v = Vec::new();
                w(&mut v, 1);
                v.extend_from_slice(&0u64.to_be_bytes());
                for x in [1000, 0, 1000, 0] {
                    w(&mut v, x);
                }
                v.extend(post(Some(fattr3(NFREG as u32, 0o644, 10, 3, 1001, 1000))));
                w(&mut v, data.len() as u32);
                w(&mut v, NFSV3WRITE_FILESYNC);
                v.extend_from_slice(&[1; 8]);
                (0, v)
            }
            _ => (0, fattr3(NFREG as u32, 0o644, 10, 3, 1001, 1000)),
        });
        assert_eq!(
            kwrite(vp, &nt, 0, &[b'w'; 10], crate::fs::vnode::IO_SYNC),
            Ok(())
        );
        assert_eq!(np.n_mtime.read().tv_sec, 1001);
        vput(vp);
        nt.done(p);
    }

    /// A version 3 WRITE the server takes only part of (`rlen < len`): the uio backs up over
    /// the rest and the next WRITE sends it from where the server stopped.
    #[test]
    fn a_short_write_sends_the_rest() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = seen.clone();
        let p = peer(&nt, move |pr, a| match pr as usize {
            NFSPROC_WRITE => {
                let (off, _, data) = write_args(a);
                let mut seen = s2.lock().unwrap();
                seen.push((off, data.clone()));
                let count = if seen.len() == 1 {
                    60
                } else {
                    data.len() as u32
                };
                (0, write3(count, NFSV3WRITE_FILESYNC, [1; 8]))
            }
            _ => (0, fattr3(NFREG as u32, 0o644, 100, 3, 1000, 1000)),
        });
        let data: Vec<u8> = (0..100u8).collect();
        assert_eq!(kwrite(vp, &nt, 0, &data, crate::fs::vnode::IO_SYNC), Ok(()));
        let seen = seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2);
        assert_eq!((seen[0].0, seen[0].1.len()), (0, 100));
        assert_eq!((seen[1].0, &seen[1].1[..]), (60, &data[60..]));
        vput(vp);
        nt.done(p);
    }

    /// The second commit pass (after the first pass's writes) commits from the lowest offset
    /// of both passes: the C's range is set once for the whole flush.
    #[test]
    fn the_commit_range_spans_both_passes() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        let commits = Arc::new(Mutex::new(Vec::new()));
        let c2 = commits.clone();
        let p = peer(&nt, move |pr, a| match pr as usize {
            NFSPROC_WRITE => {
                let (_, stable, data) = write_args(a);
                assert_eq!(stable, NFSV3WRITE_UNSTABLE);
                (0, write3(data.len() as u32, NFSV3WRITE_UNSTABLE, [1; 8]))
            }
            NFSPROC_COMMIT => {
                let (_, o) = xstr(a, 0);
                let off = u64::from_be_bytes(a[o..o + 8].try_into().unwrap());
                let cnt = u32::from_be_bytes(a[o + 8..o + 12].try_into().unwrap());
                c2.lock().unwrap().push((off, cnt));
                let mut v = wcc();
                v.extend_from_slice(&[1; 8]);
                (0, v)
            }
            _ => (0, fattr3(NFREG as u32, 0o644, 0, 3, 1000, 1000)),
        });
        // Block 0 written unstable (a flush without commit), block 1 a delayed write.
        assert_eq!(kwrite(vp, &nt, 0, &[b'a'; 100], 0), Ok(()));
        assert_eq!(
            nfs_flush(
                vp,
                &np,
                &nt.nmp,
                nt.cred(),
                MNT_WAIT,
                Some(nt.fx.user),
                false
            ),
            Ok(())
        );
        assert_eq!(kwrite(vp, &nt, 8192, &[b'b'; 50], 0), Ok(()));
        assert_eq!(nfs_fsync(vp, nt.cred(), MNT_WAIT, nt.fx.user), Ok(()));
        assert_eq!(*commits.lock().unwrap(), [(0, 100), (0, 8192 + 50)]);
        vput(vp);
        nt.done(p);
    }

    /// A version 3 READDIRPLUS entry (file id, name, cookie, attributes and handle).
    fn plus_entry(v: &mut Vec<u8>, fileid: u64, name: &[u8], cookie: u64, fh: &[u8]) {
        w(v, 1);
        v.extend_from_slice(&fileid.to_be_bytes());
        w(v, name.len() as u32);
        v.extend_from_slice(name);
        v.resize(v.len() + nfsm_padlen(name.len()), 0);
        v.extend_from_slice(&cookie.to_be_bytes());
        v.extend(post(Some(fattr3(
            NFREG as u32,
            0o644,
            1,
            fileid,
            1000,
            1000,
        ))));
        w(v, 1);
        v.extend(fh3(fh));
    }

    /// READDIRPLUS: an entry whose node cannot be had (no vnode) is packed without its type;
    /// a later entry's success clears that error (the C's assignment), so the read succeeds
    /// and the second entry's type is known.
    #[test]
    fn readdirplus_clears_an_entrys_nget_error_on_a_later_success() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO | NFSMNT_RDIRPLUS);
        let p = peer(&nt, |pr, _| match pr as usize {
            NFSPROC_READDIRPLUS => {
                let mut v = post(None);
                v.extend_from_slice(&[9; 8]);
                plus_entry(&mut v, 10, b"a", 100, &[5; 8]);
                plus_entry(&mut v, 11, b"b", 101, &[6; 8]);
                w(&mut v, 0);
                w(&mut v, 1);
                (0, v)
            }
            _ => (0, fattr3(NFDIR as u32, 0o755, 512, 2, 1000, 1000)),
        });
        crate::testutil::fail_pool("vnodes", 1);
        let mut buf = [0u8; 1024];
        let (r, eof, resid, _) = kreaddir(&nt, 0, &mut buf);
        crate::testutil::fail_pool("vnodes", 0);
        assert_eq!((r, eof), (Ok(()), 1));
        let d = dirents(&buf[..1024 - resid]);
        assert_eq!(
            d.iter().map(|e| e.2.clone()).collect::<Vec<_>>(),
            [b"a".to_vec(), b"b".to_vec()]
        );
        assert_eq!(buf[18], DT_UNKNOWN, "the first entry's type unknown");
        let second = usize::from(u16::from_ne_bytes([buf[16], buf[17]]));
        assert_eq!(buf[second + 18], sysdefs::dirent::DT_REG);
        nt.done(p);
    }

    /// MKDIR whose reply carries no handle: the LOOKUP that follows finds a file, which is
    /// `EEXIST`; the directory is put either way.
    #[test]
    fn mkdir_without_a_handle_looks_up_and_refuses_a_file() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let p = peer(&nt, |pr, _| match pr as usize {
            NFSPROC_MKDIR => {
                let mut v = Vec::new();
                w(&mut v, 0); // no handle
                v.extend(post(None));
                v.extend(wcc());
                (0, v)
            }
            NFSPROC_LOOKUP => {
                let mut v = fh3(&[7; 8]);
                v.extend(post(Some(fattr3(NFREG as u32, 0o644, 0, 7, 1000, 1000))));
                v.extend(post(None));
                (0, v)
            }
            _ => (NFSERR_IO as u32, Vec::new()),
        });
        let dvp = nt.dvp();
        vref(dvp);
        let uses = dvp.usecount();
        let mut vpp = None;
        let mut c = cn(
            &nt,
            CREATE,
            LOCKPARENT | ISLASTCN | crate::fs::namei::HASBUF,
            b"d",
        );
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_mode = 0o755;
        assert_eq!(nfs_mkdir(dvp, &mut vpp, &mut c, &va), Err(Errno::EEXIST));
        assert!(vpp.is_none());
        assert_eq!(dvp.usecount(), uses - 1, "the directory put");
        assert_eq!(procs(&p), [NFSPROC_MKDIR as u32, NFSPROC_LOOKUP as u32]);
        let _ = vn_lock(dvp, LK_EXCLUSIVE);
        nt.done(p);
    }

    /// LINK sends the file's and the directory's handles and the name; RMDIR's ENOENT (a
    /// retried request's) is success; both release their vnodes as the C's.
    #[test]
    fn link_and_an_rmdir_of_a_gone_directory() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        let _ = vop_unlock(vp);
        let p = peer(&nt, |pr, a| match pr as usize {
            NFSPROC_LINK => {
                let (fh, o) = xstr(a, 0);
                let (dfh, o) = xstr(a, o);
                assert_eq!((fh, dfh), (FILEFH.to_vec(), ROOTFH.to_vec()));
                assert_eq!(xstr(a, o).0, b"l");
                (0, [post(None), wcc()].concat())
            }
            NFSPROC_RMDIR => (Errno::ENOENT as u32, wcc()),
            _ => (0, fattr3(NFREG as u32, 0o644, 0, 3, 1000, 1000)),
        });
        let dvp = nt.dvp();
        vref(dvp);
        let mut c = cn(&nt, CREATE, crate::fs::namei::HASBUF, b"l");
        assert_eq!(nfs_link(dvp, vp, &mut c), Ok(()));
        assert!(nt.dnp.isset(NMODIFIED));
        // RMDIR of a directory node: both vnodes put by the operation.
        let _ = vn_lock(dvp, LK_EXCLUSIVE);
        vref(dvp);
        let sub = nfs_nget(nt.fx.mp, &[8; 8]).expect("a node");
        let subvp = nfstov(&sub);
        subvp.set_vtype(VDIR);
        let mut c = cn(&nt, DELETE, crate::fs::namei::HASBUF, b"s");
        assert_eq!(nfs_rmdir(dvp, subvp, &mut c), Ok(()));
        assert_eq!(procs(&p), [NFSPROC_LINK as u32, NFSPROC_RMDIR as u32]);
        drop(sub);
        drop(np);
        let _ = vn_lock(dvp, LK_EXCLUSIVE);
        nt.done(p);
    }

    /// `nfs_close` of a file a write failed for answers the saved error once.
    #[test]
    fn close_answers_a_saved_write_error_once() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        np.n_error.store(Errno::ENOSPC as i32, Ordering::Relaxed);
        np.set(NWRITEERR);
        assert_eq!(
            nfs_close(vp, nt.cred(), Some(nt.fx.user)),
            Err(Errno::ENOSPC)
        );
        assert_eq!(nfs_close(vp, nt.cred(), Some(nt.fx.user)), Ok(()));
        vput(vp);
        let p = peer(&nt, |_, _| (0, Vec::new()));
        nt.done(p);
    }

    /// `..`: the directory is unlocked for the parent's node and locked again for a
    /// `LOCKPARENT` last component; a RENAME's last component that names the directory
    /// itself is `EISDIR`, another name its node with `SAVENAME`.
    #[test]
    fn lookup_of_dotdot_and_of_a_rename_target() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let answer = Arc::new(Mutex::new([9u8; 8]));
        let a2 = answer.clone();
        let p = peer(&nt, move |_, _| {
            let mut v = fh3(&*a2.lock().unwrap());
            v.extend(post(Some(fattr3(NFDIR as u32, 0o755, 512, 9, 1000, 1000))));
            v.extend(post(None));
            (0, v)
        });
        let dvp = nt.dvp();
        let mut vpp = None;
        let mut c = cn(&nt, LOOKUP, LOCKPARENT | ISLASTCN | ISDOTDOT, b"..");
        assert_eq!(nfs_lookup(dvp, &mut vpp, &mut c), Ok(()));
        let parent = vpp.expect("the parent");
        assert_eq!(parent.vtype(), VDIR);
        assert_eq!(c.cn_flags & PDIRUNLOCK, 0, "the directory locked again");
        assert_ne!(vop_islocked(dvp), 0);
        vput(parent);

        *answer.lock().unwrap() = ROOTFH;
        let mut vpp = None;
        let mut c = cn(&nt, RENAME, WANTPARENT | LOCKPARENT | ISLASTCN, b"x");
        assert_eq!(nfs_lookup(dvp, &mut vpp, &mut c), Err(Errno::EISDIR));
        *answer.lock().unwrap() = [10; 8];
        let mut c = cn(&nt, RENAME, WANTPARENT | LOCKPARENT | ISLASTCN, b"y");
        assert_eq!(nfs_lookup(dvp, &mut vpp, &mut c), Ok(()));
        assert_ne!(c.cn_flags & SAVENAME, 0);
        vput(vpp.expect("the target"));
        nt.done(p);
    }

    /// `nfs_getset_niothreads`: a get reads how many slots run (unless the count is still
    /// -1); a set clamps the count to 0..=NFS_MAXASYNCDAEMON and, with more running than
    /// asked, counts down the surplus (whose threads, gone here, get no signal).
    #[test]
    fn niothreads_reads_back_and_clamps() {
        use super::super::nfs::nfs_niothreads;
        use super::super::nfs_syscalls::nfs_getset_niothreads;
        let nt = setup(NFSMNT_NFSV3);
        let slots = &crate::fs::subr::vfs::<tn::G>().nfs.nfs_asyncdaemon;
        let old = nfs_niothreads.load(Ordering::Relaxed);
        slots[0].store(90_001, Ordering::Relaxed);
        slots[3].store(90_002, Ordering::Relaxed);
        nfs_niothreads.store(-1, Ordering::Relaxed);
        nfs_getset_niothreads::<tn::G>(false);
        assert_eq!(
            nfs_niothreads.load(Ordering::Relaxed),
            -1,
            "none started yet"
        );
        nfs_niothreads.store(0, Ordering::Relaxed);
        nfs_getset_niothreads::<tn::G>(false);
        assert_eq!(nfs_niothreads.load(Ordering::Relaxed), 2);
        nfs_niothreads.store(-5, Ordering::Relaxed);
        nfs_getset_niothreads::<tn::G>(true);
        assert_eq!(nfs_niothreads.load(Ordering::Relaxed), 0, "clamped to 0");
        slots[0].store(0, Ordering::Relaxed);
        slots[3].store(0, Ordering::Relaxed);
        nfs_niothreads.store(old, Ordering::Relaxed);
        let p = peer(&nt, |_, _| (0, Vec::new()));
        nt.done(p);
    }

    /// The results of a version 3 READDIR of `names` (file ids from 10, cookies from 100),
    /// `eof` at the end.
    fn readdir3(names: &[&[u8]], eof: bool) -> Vec<u8> {
        let mut v = post(None);
        v.extend_from_slice(&[9; 8]); // the cookie verifier
        for (i, n) in names.iter().enumerate() {
            w(&mut v, 1);
            v.extend_from_slice(&(10 + i as u64).to_be_bytes());
            w(&mut v, n.len() as u32);
            v.extend_from_slice(n);
            v.resize(v.len() + nfsm_padlen(n.len()), 0);
            v.extend_from_slice(&(100 + i as u64).to_be_bytes());
        }
        w(&mut v, 0);
        w(&mut v, u32::from(eof));
        v
    }

    /// A readdir of the root into a user-sized kernel buffer: the entries' records.
    fn kreaddir(nt: &Nt, off: i64, buf: &mut [u8]) -> (Result<(), Errno>, i32, usize, i64) {
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, buf, Some(nt.fx.user));
        uio.uio_offset = off;
        let mut eof = 0;
        let r = nfs_readdir(nt.dvp(), &mut uio, nt.cred(), &mut eof);
        (r, eof, uio.uio_resid, uio.uio_offset)
    }

    /// The `(d_fileno, d_off, name)` of the records in `buf`.
    fn dirents(buf: &[u8]) -> Vec<(u64, i64, Vec<u8>)> {
        let mut v = Vec::new();
        let mut o = 0;
        while o + Dirent::NAME_OFFSET <= buf.len() {
            let r = &buf[o..];
            let fileno = u64::from_ne_bytes(r[0..8].try_into().unwrap());
            let off = i64::from_ne_bytes(r[8..16].try_into().unwrap());
            let reclen = usize::from(u16::from_ne_bytes(r[16..18].try_into().unwrap()));
            let namlen = usize::from(r[19]);
            if reclen == 0 {
                break;
            }
            assert!(Dirent::NAME_OFFSET + namlen < reclen && o + reclen <= buf.len());
            let name = &r[Dirent::NAME_OFFSET..Dirent::NAME_OFFSET + namlen];
            v.push((fileno, off, name.to_vec()));
            o += reclen;
        }
        v
    }

    /// READDIR: the entries become records (`d_off` the cookie, the uio's offset the last
    /// one), the end of the directory is `eofflag` and the EOF offset cache: a read there
    /// with unchanged attributes needs no RPC.
    #[test]
    fn readdir_packs_the_entries_and_caches_the_end() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let p = peer(&nt, |pr, _| match pr as usize {
            NFSPROC_READDIR => (0, readdir3(&[b"a", b"bcd"], true)),
            _ => (0, fattr3(NFDIR as u32, 0o755, 512, 2, 1000, 1000)),
        });
        let mut buf = [0u8; 1024];
        let (r, eof, resid, off) = kreaddir(&nt, 0, &mut buf);
        assert_eq!((r, eof), (Ok(()), 1));
        let used = 1024 - resid;
        assert_eq!(
            dirents(&buf[..used]),
            [(10, 100, b"a".to_vec()), (11, 101, b"bcd".to_vec())]
        );
        assert_eq!(off, 101);
        assert_eq!(nt.dnp.n_direofoffset.load(Ordering::Relaxed), 101);
        // The same directory read at its end: GETATTR (the attributes) once, no READDIR.
        let (r, eof, resid, _) = kreaddir(&nt, 101, &mut buf);
        assert_eq!((r, eof, resid), (Ok(()), 1, 1024));
        assert_eq!(procs(&p), [NFSPROC_READDIR as u32, NFSPROC_GETATTR as u32]);
        nt.done(p);
    }

    /// READDIRPLUS refused (`NFSERR_NOTSUPP`, D-3) clears `NFSMNT_RDIRPLUS` and the READDIR
    /// that follows answers; a bad cookie (`NFSERR_BAD_COOKIE`) is `EINVAL`.
    #[test]
    fn readdirplus_falls_back_and_a_bad_cookie_is_einval() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO | NFSMNT_RDIRPLUS);
        let bad = Arc::new(Mutex::new(false));
        let b2 = bad.clone();
        let p = peer(&nt, move |pr, _| match pr as usize {
            NFSPROC_READDIRPLUS => (NFSERR_NOTSUPP as u32, post(None)),
            NFSPROC_READDIR if *b2.lock().unwrap() => (NFSERR_BAD_COOKIE as u32, post(None)),
            NFSPROC_READDIR => (0, readdir3(&[b"x"], true)),
            _ => (0, fattr3(NFDIR as u32, 0o755, 512, 2, 1000, 1000)),
        });
        let mut buf = [0u8; 1024];
        let (r, eof, _, _) = kreaddir(&nt, 0, &mut buf);
        assert_eq!((r, eof), (Ok(()), 1));
        assert_eq!(nt.nmp.flag() & NFSMNT_RDIRPLUS, 0, "plus given up");
        assert_eq!(dirents(&buf)[0].2, b"x");
        *bad.lock().unwrap() = true;
        nt.dnp.n_direofoffset.store(0, Ordering::Relaxed);
        let (r, _, _, _) = kreaddir(&nt, 5, &mut buf);
        assert_eq!(r, Err(Errno::EINVAL));
        assert_eq!(
            procs(&p),
            [
                NFSPROC_READDIRPLUS as u32,
                NFSPROC_READDIR as u32,
                NFSPROC_READDIR as u32
            ]
        );
        nt.done(p);
    }

    /// A name of 0 or of more than `NFS_MAXNAMLEN` bytes, or holding a `/`, is `EBADRPC`:
    /// nothing reaches the caller's buffer.
    #[test]
    fn a_bad_entry_name_is_ebadrpc() {
        for (case, names) in [
            (0, std::vec![&b""[..]]),
            (1, std::vec![&[b'n'; 256][..]]),
            (2, std::vec![&b"a/b"[..]]),
        ] {
            let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
            let names: Vec<Vec<u8>> = names.iter().map(|n| n.to_vec()).collect();
            let p = peer(&nt, move |_, _| {
                let refs: Vec<&[u8]> = names.iter().map(|n| &n[..]).collect();
                (0, readdir3(&refs, true))
            });
            let mut buf = [0u8; 1024];
            let (r, _, resid, _) = kreaddir(&nt, 0, &mut buf);
            assert_eq!(r, Err(Errno::EBADRPC), "case {case}");
            assert_eq!(resid, 1024, "case {case}");
            nt.done(p);
        }
    }

    /// Random READDIR replies (names of 1..255 bytes, any count, `more` and `eof` either
    /// way): the records stay inside the caller's buffer and each holds the name sent; the
    /// directory buffer is never overrun (the packer's bounds) and the reply never read past.
    #[test]
    fn random_readdir_replies_stay_in_bounds() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let seed = Arc::new(Mutex::new(0x2545_f491_4f6c_dd1du64));
        let s2 = seed.clone();
        let p = peer(&nt, move |pr, _| {
            if pr as usize != NFSPROC_READDIR {
                return (0, fattr3(NFDIR as u32, 0o755, 512, 2, 1000, 1000));
            }
            let mut s = s2.lock().unwrap();
            // At most 6 entries: the fixture's peer sends a datagram from one cluster.
            let n = (next(&mut s) % 7) as usize;
            let names: Vec<Vec<u8>> = (0..n)
                .map(|_| vec![b'a' + (next(&mut s) % 26) as u8; 1 + (next(&mut s) % 255) as usize])
                .collect();
            let refs: Vec<&[u8]> = names.iter().map(|n| &n[..]).collect();
            let eof = next(&mut s) % 2 == 0;
            (0, readdir3(&refs, eof))
        });
        for i in 0..40 {
            nt.dnp.n_direofoffset.store(0, Ordering::Relaxed);
            let mut buf = vec![0u8; 512 + 512 * (i % 4)];
            let len = buf.len();
            let (r, _, resid, _) = kreaddir(&nt, 0, &mut buf);
            assert_eq!(r, Ok(()), "case {i}");
            for (_, _, name) in dirents(&buf[..len - resid]) {
                assert!(!name.is_empty() && name.iter().all(|&c| c == name[0]));
            }
        }
        nt.done(p);
    }

    /// Removing a file another user holds open renames it to a silly name instead (a LOOKUP
    /// that finds the name free, the RENAME, a LOOKUP that gives the node its new handle);
    /// its last release (`nfs_inactive`) removes the silly name.
    #[test]
    fn removing_an_open_file_renames_it_and_its_last_close_removes_it() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        vref(vp); // another user's
        let lookups = Arc::new(Mutex::new(0));
        let l2 = lookups.clone();
        let p = peer(&nt, move |pr, _| match pr as usize {
            NFSPROC_LOOKUP => {
                let mut n = l2.lock().unwrap();
                *n += 1;
                if *n == 1 {
                    (Errno::ENOENT as u32, post(None))
                } else {
                    let mut v = fh3(&[3; 8]);
                    v.extend(post(Some(fattr3(NFREG as u32, 0o644, 0, 3, 1000, 1000))));
                    v.extend(post(None));
                    (0, v)
                }
            }
            NFSPROC_RENAME => (0, [wcc(), wcc()].concat()),
            NFSPROC_REMOVE => (0, wcc()),
            _ => (0, fattr3(NFREG as u32, 0o644, 0, 3, 1000, 1000)),
        });
        let mut c = cn(
            &nt,
            DELETE,
            LOCKPARENT | ISLASTCN | crate::fs::namei::HASBUF,
            b"f",
        );
        assert_eq!(nfs_remove(nt.dvp(), vp, &mut c), Ok(()));
        let silly = {
            let r = klock::reader();
            let k = np.n_k.get(&r);
            let sp = k.n_sillyrename.as_ref().expect("the record");
            sp.s_name[..20].to_vec()
        };
        assert!(silly.starts_with(b".nfs"));
        assert_eq!(np.fh().bytes(), [3; 8], "the handle the second LOOKUP gave");
        let calls = p.calls();
        let names: Vec<Vec<u8>> = calls
            .iter()
            .map(|(_, c)| {
                let a = args_of(c);
                xstr(a, xstr(a, 0).1).0
            })
            .collect();
        assert_eq!(names[0], silly, "the probe of the silly name");
        assert_eq!(names[1], b"f", "the rename from the name");
        let a = args_of(&calls[1].1);
        let o = xstr(a, xstr(a, 0).1).1;
        assert_eq!(xstr(a, xstr(a, o).1).0, silly, "to the silly name");

        // The other user closes, then the remover: the last release removes the silly name.
        let _ = vrele(vp);
        drop(np);
        vput(vp);
        let calls = p.calls();
        let last = &calls[calls.len() - 1].1;
        assert_eq!(proc_of(last), NFSPROC_REMOVE as u32);
        let a = args_of(last);
        assert_eq!(xstr(a, xstr(a, 0).1).0, silly);
        assert_eq!(
            procs(&p),
            [
                NFSPROC_LOOKUP as u32,
                NFSPROC_RENAME as u32,
                NFSPROC_LOOKUP as u32,
                NFSPROC_REMOVE as u32
            ]
        );
        nt.done(p);
    }

    /// RENAME sends both directories' handles and names; a retried rename's `ENOENT` is
    /// success (the kludge); the vnodes are released by the operation.
    #[test]
    fn rename_sends_both_names_and_an_enoent_is_success() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let fvp = nfstov(&np);
        let _ = vop_unlock(fvp);
        let dvp = nt.dvp();
        vref(dvp);
        vref(dvp);
        let p = peer(&nt, |pr, a| {
            assert_eq!(pr, NFSPROC_RENAME as u32);
            let (_, o) = xstr(a, 0);
            let (from, o) = xstr(a, o);
            let (_, o) = xstr(a, o);
            assert_eq!((from, xstr(a, o).0), (b"a".to_vec(), b"b".to_vec()));
            (Errno::ENOENT as u32, [wcc(), wcc()].concat())
        });
        let mut fc = cn(&nt, DELETE, crate::fs::namei::HASBUF, b"a");
        let mut tc = cn(&nt, RENAME, crate::fs::namei::HASBUF, b"b");
        let uses = dvp.usecount();
        assert_eq!(nfs_rename(dvp, fvp, &mut fc, dvp, None, &mut tc), Ok(()));
        assert_eq!(dvp.usecount(), uses - 2, "fdvp released, tdvp put");
        assert!(nt.dnp.isset(NMODIFIED));
        drop(np);
        // tdvp's vput unlocked the root: the fixture's lock again for its end.
        let _ = vn_lock(dvp, LK_EXCLUSIVE);
        nt.done(p);
    }

    /// A version 3 SETATTR of a size: the sattr3 carries only the size, the node's size is
    /// the new one, and an error puts the old size back.
    #[test]
    fn a_truncation_sends_the_size_and_an_error_restores_it() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 100);
        let vp = nfstov(&np);
        let fail = Arc::new(Mutex::new(false));
        let f2 = fail.clone();
        let p = peer(&nt, move |pr, a| {
            assert_eq!(pr, NFSPROC_SETATTR as u32);
            let (_, o) = xstr(a, 0);
            let words: Vec<u32> = a[o..o + 24]
                .chunks_exact(4)
                .map(|x| u32::from_be_bytes(x.try_into().unwrap()))
                .collect();
            // mode, uid, gid not set; size set to 10.
            assert_eq!(&words[..6], &[0, 0, 0, 1, 0, 10]);
            let st = if *f2.lock().unwrap() {
                NFSERR_IO as u32
            } else {
                0
            };
            (st, wcc())
        });
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_size = 10;
        assert_eq!(nfs_setattr(vp, &mut va, nt.cred(), nt.fx.user), Ok(()));
        assert_eq!(np.n_size.load(Ordering::Relaxed), 10);
        np.n_size.store(100, Ordering::Relaxed);
        *fail.lock().unwrap() = true;
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_size = 10;
        assert_eq!(
            nfs_setattr(vp, &mut va, nt.cred(), nt.fx.user),
            Err(Errno::EIO)
        );
        assert_eq!(np.n_size.load(Ordering::Relaxed), 100, "the old size back");
        vput(vp);
        nt.done(p);
    }

    /// SYMLINK carries the name, the attributes and the target; the node the reply names is
    /// released, and the directory with it (`vput`, the C's contract).
    #[test]
    fn symlink_sends_the_target() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let p = peer(&nt, |pr, a| {
            assert_eq!(pr, NFSPROC_SYMLINK as u32);
            let (_, o) = xstr(a, 0);
            let (name, o) = xstr(a, o);
            assert_eq!(name, b"l");
            // sattr3: the mode set (0o755), the rest not.
            let o = o + 8 + 4 * 5;
            assert_eq!(xstr(a, o).0, b"/some/where");
            let mut v = Vec::new();
            w(&mut v, 1);
            v.extend(fh3(&[4; 8]));
            v.extend(post(Some(fattr3(NFLNK as u32, 0o755, 11, 4, 1000, 1000))));
            v.extend(wcc());
            (0, v)
        });
        let dvp = nt.dvp();
        vref(dvp);
        let mut c = cn(
            &nt,
            CREATE,
            LOCKPARENT | ISLASTCN | crate::fs::namei::HASBUF,
            b"l",
        );
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_mode = 0o755;
        assert_eq!(nfs_symlink(dvp, &mut c, &va, b"/some/where"), Ok(()));
        let _ = vn_lock(dvp, LK_EXCLUSIVE);
        nt.done(p);
    }

    /// A version 2 mount: GETATTR's `fattr` is the version 2 one (the mode's type bits, the
    /// blocks of 512 bytes, the change time's microseconds as `va_gen`), and the v2 kludge
    /// makes a character device with `rdev` -1 a fifo, which a new node cannot be
    /// (`EOPNOTSUPP`: fifofs is not ported).
    #[test]
    fn a_version_2_getattr_decodes_fattr2() {
        let nt = setup(0);
        let fifo = Arc::new(Mutex::new(false));
        let f2 = fifo.clone();
        let p = peer(&nt, move |pr, _| {
            assert_eq!(pr, NFSPROC_GETATTR as u32);
            let mut v = Vec::new();
            let (t, mode, rdev) = if *f2.lock().unwrap() {
                (4u32, 0o20644u32, u32::MAX)
            } else {
                (1, 0o100644, 0)
            };
            for x in [
                t, mode, 2, 1000, 20, 300, 8192, rdev, 3, 7, 42, 5, 0, 1000, 0, 2000, 77,
            ] {
                w(&mut v, x);
            }
            (0, v)
        });
        let np = nfs_nget(nt.fx.mp, &FILEFH).expect("a node");
        let vp = nfstov(&np);
        let mut va = Vattr::new();
        assert_eq!(nfs_getattr(vp, &mut va, nt.cred(), nt.fx.user), Ok(()));
        assert_eq!(
            (
                va.va_type,
                va.va_mode,
                va.va_nlink,
                va.va_size,
                va.va_bytes,
                va.va_fileid
            ),
            (VREG, 0o644, 2, 300, 3 * 512, 42)
        );
        assert_eq!(
            (va.va_ctime.tv_sec, va.va_ctime.tv_nsec, va.va_gen),
            (2000, 0, 77)
        );
        assert_eq!(vp.vtype(), VREG, "the new node's type");
        vput(vp);
        drop(np);
        *fifo.lock().unwrap() = true;
        let np = nfs_nget(nt.fx.mp, &[5; 8]).expect("a node");
        let vp = nfstov(&np);
        assert_eq!(
            nfs_getattr(vp, &mut va, nt.cred(), nt.fx.user),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(vp.vtype(), VFIFO);
        drop(np);
        vput(vp);
        nt.done(p);
    }

    /// A version 3 COMMIT: nothing before the mount has a verifier; with one, a reply of
    /// another verifier is `NFSERR_STALEWRITEVERF` and the mount takes it.
    #[test]
    fn a_commit_with_a_changed_verifier_is_stalewriteverf() {
        let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        let np = file(&nt, 0);
        let vp = nfstov(&np);
        let p = peer(&nt, |pr, a| {
            assert_eq!(pr, NFSPROC_COMMIT as u32);
            let (_, o) = xstr(a, 0);
            assert_eq!(&a[o..o + 12], &[0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 7]);
            let mut v = wcc();
            v.extend_from_slice(&[6; 8]);
            (0, v)
        });
        assert_eq!(nfs_commit(vp, &np, &nt.nmp, 9, 7, Some(nt.fx.user)), Ok(()));
        assert!(p.calls().is_empty(), "no verifier yet: nothing to commit");
        nt.nmp.set_verf([5; 8]);
        nt.nmp
            .nm_flag
            .fetch_or(NFSMNT_HASWRITEVERF, Ordering::Relaxed);
        assert_eq!(
            nfs_commit(vp, &np, &nt.nmp, 9, 7, Some(nt.fx.user)).map_err(NfsError::raw),
            Err(NFSERR_STALEWRITEVERF)
        );
        assert_eq!(nt.nmp.verf(), [6; 8]);
        assert_eq!(
            nfs_commit(vp, &np, &nt.nmp, 9, 7, Some(nt.fx.user)),
            Ok(()),
            "the same verifier again"
        );
        vput(vp);
        nt.done(p);
    }

    /// A version 3 handle of 0 or of more than `NFSX_V3FHMAX` bytes in a LOOKUP reply is
    /// `EBADRPC`; one of 64 is a node.
    #[test]
    fn a_handle_size_out_of_bounds_is_ebadrpc() {
        for (size, want) in [
            (0usize, Err(Errno::EBADRPC)),
            (65, Err(Errno::EBADRPC)),
            (64, Ok(())),
        ] {
            let nt = setup(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
            let p = peer(&nt, move |_, _| {
                let mut v = Vec::new();
                w(&mut v, size as u32);
                v.extend(vec![8u8; nfsm_rndup(size)]);
                v.extend(post(None));
                v.extend(post(None));
                (0, v)
            });
            let mut vpp = None;
            let mut c = cn(&nt, LOOKUP, LOCKPARENT | ISLASTCN, b"h");
            assert_eq!(nfs_lookup(nt.dvp(), &mut vpp, &mut c), want, "size {size}");
            if let Some(vp) = vpp {
                vput(vp);
            }
            nt.done(p);
        }
    }
    /// A version 3 `fattr3` (type word `t`, the mode word, uid, size, file id, the
    /// modification second) as the reply carries it.
    fn fattr3b(
        t: u32,
        mode: u32,
        uid: u32,
        size: u64,
        fileid: u64,
        mtime: u32,
    ) -> std::vec::Vec<u8> {
        let mut v = std::vec::Vec::new();
        for x in [t, mode, 3, uid, 20] {
            v.extend_from_slice(&x.to_be_bytes());
        }
        for q in [size, 2 * size, 0, 7, fileid] {
            v.extend_from_slice(&q.to_be_bytes());
        }
        for x in [mtime - 1, 5, mtime, 6, mtime + 1, 7] {
            v.extend_from_slice(&x.to_be_bytes());
        }
        v
    }

    /// `nfs_loadattrcache` of a new node (`VNON`): the type (the wire's word masked to three
    /// bits), the mode's permission bits, the sizes, the times, the mount's fsid and I/O size
    /// as the block size; the node's size, stamp and `n_mtime` set; the caller's copy; a reply
    /// cut short is `EBADRPC`.
    #[test]
    fn loadattrcache_fills_a_new_node() {
        let nt = setup(NFSMNT_NFSV3);
        let np = nfs_nget(nt.fx.mp, &[7; 8]).expect("a node");
        let vp = nfstov(&np);
        // Type word 9: the C masks it to 1, a regular file.
        let b = fattr3b(9, 0o104755, 1000, 300, 42, 5000);
        let m = crate::fs::nfs::nfsm_subs::test_chain(&[&b[..40], &b[40..]]);
        let mut x = super::super::nfsm_subs::XdrReader::new(&m);
        let mut va = Vattr::new();
        assert!(core::ptr::eq(
            super::super::nfs_subs::nfs_loadattrcache(&np, true, &mut x, Some(&mut va)).unwrap(),
            vp
        ));
        assert_eq!(x.offset(), 84);
        assert_eq!(vp.vtype(), VREG);
        assert_eq!(
            (va.va_type, va.va_mode, va.va_uid, va.va_gid, va.va_nlink),
            (VREG, 0o4755, 1000, 20, 3)
        );
        assert_eq!((va.va_size, va.va_bytes, va.va_fileid), (300, 600, 42));
        assert_eq!(
            (va.va_atime.tv_sec, va.va_mtime.tv_sec, va.va_ctime.tv_sec),
            (4999, 5000, 5001)
        );
        assert_eq!((va.va_mtime.tv_nsec, va.va_flags, va.va_filerev), (6, 0, 0));
        assert_eq!(va.va_fsid, i64::from(nt.fx.mp.fsid().val[0]));
        assert_eq!(va.va_blocksize, i64::from(nt.fx.mp.iosize()));
        assert_eq!(np.n_size.load(Ordering::Relaxed), 300);
        assert_eq!(np.n_mtime.read().tv_sec, 5000);
        assert_ne!(np.n_attrstamp.load(Ordering::Relaxed), 0);
        let short = crate::fs::nfs::nfsm_subs::test_chain(&[&b[..80]]);
        let mut x = super::super::nfsm_subs::XdrReader::new(&short);
        assert_eq!(
            super::super::nfs_subs::nfs_loadattrcache(&np, true, &mut x, None).err(),
            Some(Errno::EBADRPC)
        );
        drop(x);
        drop(short);
        drop(m);
        vput(vp);
        drop(np);
        let p = peer(&nt, |_, _| (0, Vec::new()));
        nt.done(p);
    }

    /// The access cache is thrown away when the owner, the group or the mode changes, kept
    /// otherwise; a modified file's larger local size is kept over the server's.
    #[test]
    fn loadattrcache_invalidates_access_and_keeps_a_modified_size() {
        let nt = setup(NFSMNT_NFSV3);
        let np = nfs_nget(nt.fx.mp, &[7; 8]).expect("a node");
        let vp = nfstov(&np);
        vp.set_vtype(VREG);
        let load = |uid: u32, size: u64| {
            let b = fattr3b(1, 0o644, uid, size, 42, 5000);
            let m = crate::fs::nfs::nfsm_subs::test_chain(&[&b]);
            let mut x = super::super::nfsm_subs::XdrReader::new(&m);
            super::super::nfs_subs::nfs_loadattrcache(&np, true, &mut x, None).map(|_| ())
        };
        assert_eq!(load(1000, 300), Ok(()));
        np.n_accstamp.store(5, Ordering::Relaxed);
        assert_eq!(load(1000, 300), Ok(()));
        assert_eq!(
            np.n_accstamp.load(Ordering::Relaxed),
            5,
            "the same owner and mode"
        );
        assert_eq!(load(2000, 300), Ok(()));
        assert_eq!(np.n_accstamp.load(Ordering::Relaxed), -1, "another owner");
        np.set(NMODIFIED);
        np.n_size.store(500, Ordering::Relaxed);
        assert_eq!(load(2000, 300), Ok(()));
        assert_eq!(np.n_size.load(Ordering::Relaxed), 500);
        assert_eq!(np.n_vattr.read().va_size, 500, "the local size kept");
        assert_eq!(load(2000, 900), Ok(()));
        assert_eq!(
            np.n_size.load(Ordering::Relaxed),
            900,
            "a larger server size taken"
        );
        np.clr(NMODIFIED);
        vput(vp);
        drop(np);
        let p = peer(&nt, |_, _| (0, Vec::new()));
        nt.done(p);
    }

    /// Version 2: a character device of `rdev` -1 is a fifo (the kludge), which a new node
    /// cannot be without fifofs (`EOPNOTSUPP`); the type is the mode's when the word says
    /// `NFNON` or `NFREG`.
    #[test]
    fn loadattrcache_of_version_2() {
        let nt = setup(0);
        let fattr2 = |t: u32, mode: u32, rdev: u32| {
            let mut v = std::vec::Vec::new();
            for x in [t, mode, 1, 0, 0, 10, 4096, rdev, 1, 7, 3, 1, 0, 2, 0, 3, 0] {
                v.extend_from_slice(&x.to_be_bytes());
            }
            v
        };
        let np = nfs_nget(nt.fx.mp, &[7; 8]).expect("a node");
        let vp = nfstov(&np);
        let b = fattr2(4, 0o20600, u32::MAX);
        let m = crate::fs::nfs::nfsm_subs::test_chain(&[&b]);
        let mut x = super::super::nfsm_subs::XdrReader::new(&m);
        assert_eq!(
            super::super::nfs_subs::nfs_loadattrcache(&np, false, &mut x, None).err(),
            Some(Errno::EOPNOTSUPP)
        );
        assert_eq!(vp.vtype(), VFIFO);
        drop(x);
        drop(m);
        vput(vp);
        drop(np);
        let np = nfs_nget(nt.fx.mp, &[8; 8]).expect("a node");
        let vp = nfstov(&np);
        let b = fattr2(1, 0o40755, 0);
        let m = crate::fs::nfs::nfsm_subs::test_chain(&[&b]);
        let mut x = super::super::nfsm_subs::XdrReader::new(&m);
        assert_eq!(
            super::super::nfs_subs::nfs_loadattrcache(&np, false, &mut x, None).map(|_| ()),
            Ok(())
        );
        assert_eq!(vp.vtype(), VDIR, "IFTOVT of the mode for NFREG");
        assert_eq!(np.n_vattr.read().va_blocksize, 4096);
        drop(x);
        drop(m);
        vput(vp);
        drop(np);
        let p = peer(&nt, |_, _| (0, Vec::new()));
        nt.done(p);
    }
}
/* </TESTS> */
