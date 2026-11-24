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
 *	@(#)nfs_serv.c	8.7 (Berkeley) 5/14/95
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS server's procedures (N6.7b3): `nfs_serv.c`, the version 2 and 3 server calls to
//! vnode operations, one function per procedure (`nfsrv_getattr`, `nfsrv_lookup`,
//! `nfsrv_read`, ...), each of nfssvc's `NfsrvProc` type, and `nfs_syscalls.c`'s table of
//! them, `nfsrv3_procs`, which the nfsds dispatch through by `nd_procnum`. Beside `bsd`'s
//! server until N6.7b4 switches the server on.
//!
//! Upstream: sys/nfs/nfs_serv.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_serv.rs@e8249449d905
//!
//! These routines generally have 3 phases:
//! 1. break down and validate the rpc request in the mbuf list;
//! 2. do the vnode ops for the request (surprisingly many are very similar to the syscalls
//!    in vfs_syscalls.c);
//! 3. build the rpc reply in an mbuf list.
//!
//! Do not mix the phases, since a bad request makes the dissection fail without doing any
//! `vrele()` or `vput()`. `nfsm_reply()` generates an nfs rpc reply with the nfs error number
//! iff error != 0, whereas returning an error from the server function implies a fatal error
//! such as a badly constructed rpc request that should be dropped without a reply. For
//! version 3, we do not return after `nfsm_reply()` for the error case, since most version 3
//! rpcs return more than the status for error cases.
//!
//! A procedure serves a client the file system cannot tell from a local user: every check
//! the C makes is kept, in its order (the export and the reserved port in `nfsrv_fhtovp`, a
//! read-only export or file system, [`nfsrv_access`] and its owner override, `suser_ucred`
//! for device nodes and directory removal, every length from the wire against its bound).
//!
//! `option NFSSERVER` only: the module is behind the kernel's `nfsserver` feature.
//!
//! ## Deviations
//! - A procedure returns the reply and the C's `int` (`NfsrvProc`): `(Some(reply), 0)` is
//!   the C's 0 with the reply in `*mrq`, `(None, error)` a request dropped without one. The
//!   status put in a reply stays the C's `int` (an errno or an NFS status, `NFSERR_*`), as
//!   `nfs_namei` and `nfsrv_fhtovp` return it.
//! - Leaks of the C fixed on error paths (LZ's, kept): the directory of a failed name lookup
//!   is released before the reply, so also when `nfsm_reply` returns non-zero (a version 3
//!   `EBADRPC`); `nfsrv_create` releases the existing file a `GUARDED` create refuses with
//!   `EEXIST` (the C leaves it locked); `nfsrv_rename`'s `nfsmout:` releases the source's
//!   parent also when it is the file handle's directory (the C's `XXX` workaround keeps one
//!   reference) and gives the saved name back. One more of the kind (N6.7b3): an exclusive
//!   create whose `VOP_SETATTR` of the verifier fails leaves the new file locked and
//!   referenced in the C (and in LZ); here it is `vput`.
//! - `nfsrv_symlink` hands `VOP_SYMLINK` the target up to its first NUL, the C's `strlen`
//!   of its NUL-terminated copy (LZ passed all `len2` bytes).
//! - The device path of `nfsrv_create` is version 2's only (version 3 creates a `VREG`); where
//!   it ends a call with its status, the directory the lookup kept (none in version 2) is
//!   released first, as `nfsmout:` would.
//! - `MGET`/`MCLGET` with `M_WAIT` cannot fail in the C; here an mbuf or a cluster that cannot
//!   be had for the data of a read or a readlink is an `ENOBUFS` reply (LZ's).
//! - Before the second `vfs_lookup` of `nfsrv_create`'s device path, `nfsrv_mknod` and
//!   `nfsrv_symlink`, `ni_pathlen` is set to the length of the name looked up again: the
//!   lookup subtracts it a second time, which the C's `size_t` survives by wrapping (LZ's).
//! - Readdir: a directory entry with a zero `d_reclen`, or one that does not fit the buffer,
//!   ends the walk (the C would loop or read past the buffer; LZ's).
//!
//! ## Redesign
//! - LZ's procedures took the descriptor's raw cursor (`nd_md`, `nd_dpos`), parked
//!   `&'static Mbuf` replies built through `nfsm_build` views and filled in later through an
//!   `unsafe` view (`nfsm_rebuild`, the read's attributes), and `Vec<Iovec>`s of `mtod`
//!   pointers for `VOP_READ`/`VOP_WRITE`. Here a procedure reads its arguments through one
//!   `XdrReader` over the descriptor's owned request (every length compared before it is a
//!   size, never a read past the chain) and builds its reply through the `XdrWriter`
//!   `nfs_rephead` begins, which owns the chain: [`nfsm_reply`] answers the writer to go on
//!   with or the finished reply. No `unsafe`.
//! - Read and write hand the vnode one uio over the mbufs' bytes, as the C:
//!   `Uio::kernel_readv` over the data mbufs (`Mbuf::data_and_next_mut` lends them all at
//!   once) and `Uio::kernel_writev` over the request's pieces after the arguments, the
//!   iovecs one `M_TEMP` array. A read fills its data mbufs before the reply is begun and
//!   builds the attributes and counts after, in one `nfsm_build`, then joins the data with
//!   `m_cat` (the C reads into mbufs already linked after a header it fills in afterwards);
//!   the bytes on the wire are the C's, the reply's first mbuf takes no data cluster.
//! - The name lookups take the request's credentials for both their parts: `nfs_namei`'s
//!   `nfsrv_fhtovp` maps `&nd.nd_cr` and the lookup reads `cn_cred = Cred::real(&nd.nd_cr)`,
//!   the same credential (the C's `ndp->ni_cnd.cn_cred = cred`); `nfsrv_mknod`'s second
//!   lookup reads the nfsd's (`procp->p_ucred`), as the C's.
//! - The symlink target is copied out of the request straight into its `M_TEMP` buffer
//!   (`XdrReader::bytes_into`, then the padding skipped): the C's `nfsd_mtouio` through a
//!   kernel uio, the same bytes and the same `EBADRPC` at the request's end, without a
//!   `uiomove` per mbuf.
//! - Readdir's and readdirplus' passes are one function (`nfsrv_readdir_pass`: lock,
//!   `VOP_READDIR` into the `M_TEMP` buffer, the attributes, unlock), the C's `again:` loop
//!   around it.
//! - `nfsrv3_procs[]` (`nfs_syscalls.c`) is `nfsrv3_procs`, the table of one glue's
//!   procedures, built where they are.

use core::ptr;
use core::sync::atomic::Ordering::Relaxed;

use super::nfs_srvsocket::nfs_rephead;
use super::nfs_srvsubs::{
    nfs_namei, nfsm_adj, nfsm_srvfattr, nfsm_srvfhtom, nfsm_srvpostop_attr, nfsm_srvsattr,
    nfsm_srvwcc, nfsrv_fhtovp,
};
use super::nfs_subs::{NFSSTATS, nfs_false, nfs_true, nfs_xdrneg1};
use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_padlen, nfsm_rndup};
use super::nfsproto::{
    NFS_FABLKSIZE, NFS_MAXDATA, NFS_MAXDGRAMDATA, NFS_MAXNAMLEN, NFS_MAXPATHLEN, NFS_NPROCS,
    NFSERR_BADTYPE, NFSERR_NAMETOL, NFSERR_NOT_SYNC, NFSERR_RETVOID, NFSERR_TOOSMALL,
    NFSV3ACCESS_DELETE, NFSV3ACCESS_EXECUTE, NFSV3ACCESS_EXTEND, NFSV3ACCESS_LOOKUP,
    NFSV3ACCESS_MODIFY, NFSV3ACCESS_READ, NFSV3CREATE_EXCLUSIVE, NFSV3CREATE_GUARDED,
    NFSV3CREATE_UNCHECKED, NFSV3FSINFO_CANSETTIME, NFSV3FSINFO_HOMOGENEOUS, NFSV3FSINFO_LINK,
    NFSV3FSINFO_SYMLINK, NFSV3WRITE_FILESYNC, NFSV3WRITE_UNSTABLE, NFSX_UNSIGNED, NFSX_V2FATTR,
    NFSX_V2FH, NFSX_V3COOKIEVERF, NFSX_V3CREATEVERF, NFSX_V3FATTR, NFSX_V3FH, NFSX_V3FSINFO,
    NFSX_V3PATHCONF, NFSX_V3POSTOPATTR, NFSX_V3WCCDATA, NFSX_V3WRITEVERF, NfsFattr, NfsStatfs,
    Nfsfh, Nfsuint64, Nfsv2Sattr, Nfsv3Fsinfo, Nfsv3Pathconf, Nfsv3Time, nfstov_mode,
    nfsv3tov_type, nfsx_cookieverf, nfsx_fattr, nfsx_postopattr, nfsx_postoporfattr,
    nfsx_preopattr, nfsx_srvfh, nfsx_statfs, nfsx_wccdata, nfsx_wccorfattr, nfsx_writeverf,
};
use super::nfssvc::{ND_NFSV3, NfsrvDescript, NfsrvProc, NfssvcSock, nd_reader, nfs_srvmaxdata};
use super::xdr_subs::{
    fxdr_nfsv2time, fxdr_nfsv3time, fxdr_unsigned, txdr_hyper, txdr_unsigned, xdr_bytes,
};
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::cred::{Ucred, suser_ucred};
use crate::fs::lookup::{ndinit, vfs_lookup};
use crate::fs::mount::Mount;
use crate::fs::namei::{
    CREATE, Componentname, DELETE, FOLLOW, HASBUF, ISSYMLINK, LOCKLEAF, LOCKPARENT, LOOKUP,
    NOCACHE, NOFOLLOW, Nameidata, NiDirp, RDONLY, RENAME, SAVESTART, WANTPARENT,
};
use crate::fs::subr::{vattr_null, vput, vrele};
use crate::fs::ufs::ufs::dir::DIRBLKSIZ;
use crate::fs::vnode::{
    IO_NODELOCKED, IO_SYNC, LK_EXCLUSIVE, LK_RETRY, VA_UTIMES_NULL, VEXEC, VREAD, VROOT, VTEXT,
    VWRITE, Vattr, Vnode, Vtype, iftovt, vn_lock,
};
use crate::fs::vops::{
    Cred, VFS_STATFS, VFS_VGET, VFS_VPTOFH, vop_abortop, vop_access, vop_create, vop_fsync,
    vop_getattr, vop_link, vop_mkdir, vop_mknod, vop_pathconf, vop_read, vop_readdir, vop_readlink,
    vop_remove, vop_rename, vop_rmdir, vop_setattr, vop_symlink, vop_unlock, vop_write,
};
use crate::net::mbuf::{
    M_WAIT, MChainBuilder, MINCLSIZE, MT_DATA, Mbuf, MbufBox, m_cat, m_get, m_trailingspace, mclget,
};
use crate::proc::{ProcGlue, Thread};
use crate::sys::dirent::Dirent;
use crate::sys::errno::Errno;
use crate::sys::mount::{Fhandle, Fid, MNT_RDONLY, MNT_WAIT, Statfs};
use crate::sys::socket::SOCK_DGRAM;
use crate::sys::types::{Dev, Mode, Off, Register, makedev};
use crate::sys::unistd::{_PC_CHOWN_RESTRICTED, _PC_LINK_MAX, _PC_NAME_MAX, _PC_NO_TRUNC};
use crate::tc::microboottime;
use crate::uio::{Iovec, Uio};
use crate::vm::vnode::uvm_vnp_uncache;
use hal::malloc::{M_WAITOK, MArray};

/// `malloc(9)`'s `M_TEMP`: the read and write iovec arrays, the readdir buffer, the symlink
/// target.
const M_TEMP: i32 = 127;

/// `va_size` of a `vattr_null`ed `struct vattr` (`(u_quad_t)VNOVAL`): no size given.
const VNOVAL_SIZE: u64 = u64::MAX;

/// What a procedure answers (`NfsrvProc`'s result): the reply (the C's `*mrq`, `None` for
/// none) and the C's `int` (0, or the error of a request dropped without a reply).
type Answer = (Option<MbufBox>, i32);

/// `nfsd_dissect()`'s `goto nfsmout` with the value or the error: the procedure returns the
/// error with the request freed, as `nfsd_dissect` frees it (no reply).
macro_rules! dissect {
    ($nd:expr, $e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return bad_request($nd, e),
        }
    };
}

/// The status of a vnode operation as the C's `int error` (0 or an errno).
fn status_of<T>(r: &Result<T, Errno>) -> i32 {
    match r {
        Ok(_) => 0,
        Err(e) => e.as_i32(),
    }
}

/// `&nfsd->nd_cr` as the vnode operations take it: the request's credentials.
fn cred(nd: &NfsrvDescript) -> Cred<'_> {
    Cred::real(&nd.nd_cr)
}

/// `vp->v_mount`, which a vnode an export or a lookup returned always has.
fn v_mount<G: ProcGlue>(vp: &'static Vnode<G>) -> &'static Mount<G> {
    match vp.mount() {
        Some(mp) => mp,
        None => kpanic(format_args!("nfs_serv: vnode {vp:p} without a mount")),
    }
}

/// `vp->v_mount->mnt_flag & MNT_RDONLY`.
fn mnt_rdonly<G: ProcGlue>(vp: &'static Vnode<G>) -> bool {
    v_mount(vp).flag() & MNT_RDONLY != 0
}

/// `nd.ni_vp` after a lookup that returned it.
fn ni_vp<G: ProcGlue>(ni: &Nameidata<'_, G>) -> &'static Vnode<G> {
    match ni.ni_vp {
        Some(vp) => vp,
        None => kpanic(format_args!("nfs_serv: lookup without a vnode")),
    }
}

/// `nd.ni_dvp` after a lookup that returned the parent.
fn ni_dvp<G: ProcGlue>(ni: &Nameidata<'_, G>) -> &'static Vnode<G> {
    match ni.ni_dvp {
        Some(dvp) => dvp,
        None => kpanic(format_args!("nfs_serv: lookup without a parent")),
    }
}

/// Whether `a` and `b` are the same vnode (the C's pointer comparison; NULL is no vnode).
fn same_vnode<G: ProcGlue>(a: Option<&'static Vnode<G>>, b: Option<&'static Vnode<G>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// Whether `vp` and `wp` are on the same mount (`vp->v_mount == wp->v_mount`).
fn same_mount<G: ProcGlue>(vp: &'static Vnode<G>, wp: &'static Vnode<G>) -> bool {
    ptr::eq(v_mount(vp), v_mount(wp))
}

/// `VOP_ABORTOP(nd.ni_dvp, &nd.ni_cnd)`.
fn abortop<G: ProcGlue>(ni: &mut Nameidata<'_, G>) {
    if let Some(dvp) = ni.ni_dvp {
        let _ = vop_abortop(dvp, &mut ni.ni_cnd);
    }
}

/// `if (nd.ni_dvp == nd.ni_vp) vrele(nd.ni_dvp); else vput(nd.ni_dvp);`: releases the locked
/// parent of a lookup, which is only referenced when it is also the leaf.
fn dvp_release<G: ProcGlue>(ni: &Nameidata<'_, G>) {
    if let Some(dvp) = ni.ni_dvp {
        if same_vnode(Some(dvp), ni.ni_vp) {
            vrele(dvp);
        } else {
            vput(dvp);
        }
    }
}

/// `if (cn_flags & HASBUF) { pool_put(&namei_pool, cn_pnbuf); cn_flags &= ~HASBUF; }`.
fn pnbuf_put_hasbuf<G: ProcGlue>(cnp: &mut Componentname<'_, G>) {
    if cnp.cn_flags & HASBUF != 0 {
        pnbuf_put(cnp);
        cnp.cn_flags &= !HASBUF;
    }
}

/// `ni_pathlen` for the second `vfs_lookup` of the name the first one left in
/// `cn_nameptr` (after `VOP_MKNOD`/`VOP_SYMLINK`): the length of that name, which the lookup
/// consumes again (the C lets the unsigned count wrap below zero there).
fn nfsrv_relookup_pathlen<G: ProcGlue>(ni: &mut Nameidata<'_, G>) {
    ni.ni_pathlen = usize::try_from(ni.ni_cnd.namelen()).unwrap_or(0);
}

/// `malloc(n, M_TEMP, M_WAITOK)` of bytes, which cannot fail in the C (zeroed here).
fn temp_bytes(n: usize) -> MArray<u8> {
    match MArray::new(M_TEMP, M_WAITOK, n, |_| 0u8) {
        Some(a) => a,
        None => kpanic(format_args!("nfs_serv: no buffer")),
    }
}

/// `vrele(nd.ni_startdir)`: the starting directory a `SAVESTART` lookup kept referenced.
fn startdir_rele<G: ProcGlue>(ni: &mut Nameidata<'_, G>) {
    if let Some(sd) = ni.ni_startdir.take() {
        vrele(sd);
    }
}

/// `pool_put(&namei_pool, cnp->cn_pnbuf)`: gives a saved name buffer back.
fn pnbuf_put<G: ProcGlue>(cnp: &mut Componentname<'_, G>) {
    drop(cnp.take_pnbuf());
}

/// `NDINIT(&nd, op, flags, UIO_SYSSPACE, NULL, procp); nd.ni_cnd.cn_cred = cred`: a
/// lookup of a name the request carries, with the request's credentials.
fn ndinit_cred<'a, G: ProcGlue>(
    op: u64,
    flags: u64,
    p: &'a Thread<G>,
    nd: &'a NfsrvDescript,
) -> Nameidata<'a, G> {
    let mut ni = ndinit(op, flags, NiDirp::Sys(&[]), p);
    ni.ni_cnd.cn_cred = cred(nd);
    ni
}

/// `nfsrv_fhtovp(fhp, lockflag, &vp, &nfsd->nd_cr, slp, nfsd->nd_nam, &rdonly)`.
fn fhtovp<G: ProcGlue>(
    nd: &NfsrvDescript,
    fh: &Fhandle,
    lockflag: bool,
    slp: &NfssvcSock<G>,
) -> Result<(&'static Vnode<G>, bool), i32> {
    nfsrv_fhtovp(fh, lockflag, &nd.nd_cr, slp, nd.nam())
}

/// `nfs_namei(ndp, fhp, len, slp, nfsd->nd_nam, &nfsd->nd_md, &nfsd->nd_dpos, retdirp, p)`:
/// the name of `len` bytes at the cursor `r` looked up under the directory `fh` names.
#[allow(clippy::too_many_arguments, reason = "the C's argument list")]
fn namei<'a, G: ProcGlue>(
    nd: &'a NfsrvDescript,
    ni: &mut Nameidata<'a, G>,
    fh: &Fhandle,
    len: usize,
    slp: &NfssvcSock<G>,
    r: &mut XdrReader<'_>,
    retdirp: &mut Option<&'static Vnode<G>>,
    p: &'a Thread<G>,
) -> Result<(), i32> {
    nfs_namei(ni, fh, len, slp, nd.nam(), r, retdirp, p, &nd.nd_cr)
}

/// The file handle of `vp`, as the server writes it into a reply: `memset(fhp, 0,
/// sizeof(nfh)); fhp->fh_fsid = vp->v_mount->mnt_stat.f_fsid; VFS_VPTOFH(vp, &fhp->fh_fid)`.
fn nfsrv_vptofh<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<Nfsfh, Errno> {
    let mut fh = Fhandle {
        fh_fsid: v_mount(vp).fsid(),
        fh_fid: Fid::default(),
    };
    VFS_VPTOFH(vp, &mut fh.fh_fid)?;
    let mut nfh = Nfsfh::new();
    nfh.set_fh_generic(&fh);
    Ok(nfh)
}

/// The finished reply `mb` (`*mrq`), the procedure's 0.
fn finish(mb: XdrWriter) -> Answer {
    (Some(mb.into_chain()), 0)
}

/// The finished reply `mb` with the data chain `data` after it (`mb->m_next = mp`, joined by
/// `m_cat`: a small first data mbuf may be copied into the header's room).
fn finish_with(mb: XdrWriter, data: Option<MbufBox>) -> Answer {
    let mut m = mb.into_chain();
    if data.is_some() {
        m_cat(&mut m, data);
    }
    (Some(m), 0)
}

/// A request that could not be dissected (`nfsmout:` with `nfsd_dissect`'s error): freed as
/// `nfsd_dissect` frees it, no reply.
fn bad_request(nd: &mut NfsrvDescript, e: Errno) -> Answer {
    nd.nd_mrep = None;
    (None, e.as_i32())
}

/// `nfsm_reply(nfsd, slp, mrq, &mb, error, statuslen)`: builds the reply header for the
/// status `error` (with room for `statuslen` bytes after it; none for a version 2 error) and
/// frees the request. `Ok` is the writer the procedure goes on with; `Err` the finished
/// reply where the C returns non-zero (a version 2 error, `EBADRPC`): the procedure returns
/// it.
fn nfsm_reply<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    error: i32,
    statuslen: usize,
) -> Result<XdrWriter, Answer> {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    nd.nd_repstat = error;
    let statuslen = if error != 0 && !v3 { 0 } else { statuslen };
    let mb = nfs_rephead(statuslen, nd, slp, error);
    nd.nd_mrep = None;
    if error != 0 && (!v3 || error == Errno::EBADRPC.as_i32()) {
        return Err(finish(mb));
    }
    Ok(mb)
}

/// `(void)nfsm_reply(...); return 0;`: the reply is the status alone.
fn nfsm_reply_only<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    error: i32,
    statuslen: usize,
) -> Answer {
    match nfsm_reply(nd, slp, error, statuslen) {
        Ok(mb) => finish(mb),
        Err(done) => done,
    }
}

/// `nfsm_reply(error, statuslen)` then `nfsm_srvpostop_attr(after)`: an error reply with the
/// post-operation attributes (version 3; a version 2 error ends at its status).
fn reply_postop<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    error: i32,
    statuslen: usize,
    after: Option<&Vattr>,
) -> Answer {
    match nfsm_reply(nd, slp, error, statuslen) {
        Ok(mut mb) => {
            nfsm_srvpostop_attr(nd, after, &mut mb);
            finish(mb)
        }
        Err(done) => done,
    }
}

/// `nfsm_reply(error, statuslen)` then `nfsm_srvwcc(before, after)`: an error reply with the
/// weak cache consistency data (version 3; a version 2 error ends at its status).
fn reply_wcc<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    error: i32,
    statuslen: usize,
    before: Option<&Vattr>,
    after: Option<&Vattr>,
) -> Answer {
    match nfsm_reply(nd, slp, error, statuslen) {
        Ok(mut mb) => {
            nfsm_srvwcc(nd, before, after, &mut mb);
            finish(mb)
        }
        Err(done) => done,
    }
}

/// `fp = nfsm_build(&mb, NFSX_FATTR(v3)); nfsm_srvfattr(nfsd, vap, fp)`: the attributes of
/// `va` in the request's version.
fn nfsm_srvfattr_build(nd: &NfsrvDescript, va: &Vattr, mb: &mut XdrWriter) {
    let n = nfsx_fattr(nd.nd_flag & ND_NFSV3 != 0);
    let fp = nfsm_srvfattr(nd, va);
    mb.build_slice(n).copy_from_slice(&xdr_bytes(&fp)[..n]);
}

/// `*tl++ = w` into a block `nfsm_build` gave: the raw XDR word `w` at word `i` of `tl`.
fn set_word(tl: &mut [u8], i: usize, w: u32) {
    tl[i * NFSX_UNSIGNED..(i + 1) * NFSX_UNSIGNED].copy_from_slice(&w.to_ne_bytes());
}

/// `nfsm_srvmtofh1(nfsd, slp, mrq, &mb, &error)`: the length word of a version 3 file
/// handle; `true` when it is not `NFSX_V3FH` (the C's `EBADRPC` reply).
fn nfsm_srvmtofh1(r: &mut XdrReader<'_>, v3: bool) -> Result<bool, Errno> {
    Ok(v3 && r.i32()? != NFSX_V3FH as i32)
}

/// `nfsm_srvmtofh2(nfsd, fhp, &error)`: the file handle (`NFSX_V3FH` bytes; a version 2
/// handle is padded to `NFSX_V2FH`).
fn nfsm_srvmtofh2(r: &mut XdrReader<'_>, v3: bool) -> Result<Fhandle, Errno> {
    let fh: Fhandle = r.pod()?;
    if !v3 {
        r.adv(NFSX_V2FH - NFSX_V3FH)?;
    }
    Ok(fh)
}

/// `nfsm_srvmtofh1` then `nfsm_srvmtofh2`, as every procedure begins: the request's file
/// handle, `None` for a version 3 handle of the wrong length.
fn nfsm_srvmtofh(r: &mut XdrReader<'_>, v3: bool) -> Result<Option<Fhandle>, Errno> {
    if nfsm_srvmtofh1(r, v3)? {
        return Ok(None);
    }
    nfsm_srvmtofh2(r, v3).map(Some)
}

/// The cursor at a request's arguments (`nd_md`/`nd_dpos`, over `nd_mrep` alone so the
/// descriptor's other members stay usable) and the file handle they begin with
/// ([`nfsm_srvmtofh`]).
fn srv_args(
    mrep: &Option<MbufBox>,
    off: usize,
    v3: bool,
) -> Result<(XdrReader<'_>, Option<Fhandle>), Errno> {
    let mut r = nd_reader(mrep, off)?;
    let fh = nfsm_srvmtofh(&mut r, v3)?;
    Ok((r, fh))
}

/// `nfsm_srvnamesiz(nfsd, &len, &error)`: the length of the name that follows: `(len, 0)`, or
/// `(0, NFSERR_NAMETOL)` past `NFS_MAXNAMLEN`, `(0, EBADRPC)` for an empty or negative one.
fn nfsm_srvnamesiz(r: &mut XdrReader<'_>) -> Result<(usize, i32), Errno> {
    let len = r.i32()?;
    Ok(if len > NFS_MAXNAMLEN as i32 {
        (0, NFSERR_NAMETOL)
    } else if len <= 0 {
        (0, Errno::EBADRPC.as_i32())
    } else {
        (len as usize, 0)
    })
}

/// `mallocarray(n, sizeof(struct iovec), M_TEMP, M_WAITOK)`, which cannot fail in the C.
fn temp_iovecs(n: usize) -> MArray<Iovec> {
    match MArray::new(M_TEMP, M_WAITOK, n, |_| Iovec::new()) {
        Some(a) => a,
        None => kpanic(format_args!("nfs_serv: no iovec array")),
    }
}

/// nfs v3 access service
pub fn nfsrv3_access<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let mut nfsmode = dissect!(nd, r.u32());
    let (vp, rdonly) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return reply_postop(nd, slp, error, NFSX_UNSIGNED, None),
    };
    let isdir = vp.vtype() == Vtype::VDIR;
    if nfsmode & NFSV3ACCESS_READ != 0
        && nfsrv_access(vp, VREAD, &nd.nd_cr, rdonly, p, false).is_err()
    {
        nfsmode &= !NFSV3ACCESS_READ;
    }
    let testmode = if isdir {
        NFSV3ACCESS_MODIFY | NFSV3ACCESS_EXTEND | NFSV3ACCESS_DELETE
    } else {
        NFSV3ACCESS_MODIFY | NFSV3ACCESS_EXTEND
    };
    if nfsmode & testmode != 0 && nfsrv_access(vp, VWRITE, &nd.nd_cr, rdonly, p, false).is_err() {
        nfsmode &= !testmode;
    }
    let testmode = if isdir {
        NFSV3ACCESS_LOOKUP
    } else {
        NFSV3ACCESS_EXECUTE
    };
    if nfsmode & testmode != 0 && nfsrv_access(vp, VEXEC, &nd.nd_cr, rdonly, p, false).is_err() {
        nfsmode &= !testmode;
    }
    let mut va = Vattr::new();
    let getret = vop_getattr(vp, &mut va, cred(nd), p).is_ok();
    vput(vp);
    let mut mb = match nfsm_reply(nd, slp, 0, nfsx_postopattr(true) + NFSX_UNSIGNED) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    nfsm_srvpostop_attr(nd, getret.then_some(&va), &mut mb);
    mb.u32(nfsmode);
    finish(mb)
}

/// nfs getattr service
pub fn nfsrv_getattr<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let fh = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((_, Some(fh))) => fh,
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (vp, _) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return nfsm_reply_only(nd, slp, error, 0),
    };
    let mut va = Vattr::new();
    let error = status_of(&vop_getattr(vp, &mut va, cred(nd), p));
    vput(vp);
    let mut mb = match nfsm_reply(nd, slp, error, nfsx_fattr(v3)) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if error == 0 {
        nfsm_srvfattr_build(nd, &va, &mut mb);
    }
    finish(mb)
}

/// The arguments of a SETATTR after the file handle: the attributes to set into `va`, and
/// the version 3 guard (`gcheck`, the ctime the client expects).
fn nfsrv_setattr_args(
    r: &mut XdrReader<'_>,
    v3: bool,
    va: &mut Vattr,
) -> Result<Option<Timespec>, Errno> {
    if v3 {
        va.va_vaflags |= VA_UTIMES_NULL;
        nfsm_srvsattr(r, va)?;
        let gcheck = r.i32()? != 0;
        if gcheck {
            let t: Nfsv3Time = r.pod()?;
            return Ok(Some(fxdr_nfsv3time(&t)));
        }
        return Ok(None);
    }
    let sp: Nfsv2Sattr = r.pod()?;
    // Nah nah nah nah na nah
    // There is a bug in the Sun client that puts 0xffff in the mode field of sattr when it
    // should put in 0xffffffff. The u_short doesn't sign extend.
    // --> check the low order 2 bytes for 0xffff
    if fxdr_unsigned(sp.sa_mode) & 0xffff != 0xffff {
        va.va_mode = nfstov_mode(sp.sa_mode);
    }
    if sp.sa_uid != nfs_xdrneg1 {
        va.va_uid = fxdr_unsigned(sp.sa_uid);
    }
    if sp.sa_gid != nfs_xdrneg1 {
        va.va_gid = fxdr_unsigned(sp.sa_gid);
    }
    if sp.sa_size != nfs_xdrneg1 {
        va.va_size = u64::from(fxdr_unsigned(sp.sa_size));
    }
    if sp.sa_atime.nfsv2_sec != nfs_xdrneg1 {
        // `#ifdef notyet`: fxdr_nfsv2time(&sp->sa_atime, &va.va_atime);
        va.va_atime.tv_sec = i64::from(fxdr_unsigned(sp.sa_atime.nfsv2_sec));
        va.va_atime.tv_nsec = 0;
    }
    if sp.sa_mtime.nfsv2_sec != nfs_xdrneg1 {
        va.va_mtime = fxdr_nfsv2time(&sp.sa_mtime);
    }
    Ok(None)
}

/// nfs setattr service
pub fn nfsrv_setattr<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let mut va = Vattr::new();
    vattr_null(&mut va);
    let guard = dissect!(nd, nfsrv_setattr_args(&mut r, v3, &mut va));

    // Now that we have all the fields, lets do it.
    let (vp, rdonly) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return reply_wcc(nd, slp, error, 2 * NFSX_UNSIGNED, None, None),
    };
    let mut preat = Vattr::new();
    let mut preat_ok = false;
    if v3 {
        let g = vop_getattr(vp, &mut preat, cred(nd), p);
        preat_ok = g.is_ok();
        let mut error = status_of(&g);
        if error == 0
            && let Some(guard) = guard
            && (preat.va_ctime.tv_sec != guard.tv_sec || preat.va_ctime.tv_nsec != guard.tv_nsec)
        {
            error = NFSERR_NOT_SYNC;
        }
        if error != 0 {
            vput(vp);
            let before = preat_ok.then_some(&preat);
            return reply_wcc(nd, slp, error, nfsx_wccdata(v3), before, None);
        }
    }

    // If the size is being changed write access is required, otherwise just check for a read
    // only file system.
    let mut postat_ok = false;
    let error = 'out: {
        if va.va_size == VNOVAL_SIZE {
            if rdonly || mnt_rdonly(vp) {
                break 'out Errno::EROFS.as_i32();
            }
        } else if vp.vtype() == Vtype::VDIR {
            break 'out Errno::EISDIR.as_i32();
        } else if let Err(e) = nfsrv_access(vp, VWRITE, &nd.nd_cr, rdonly, p, true) {
            break 'out e.as_i32();
        }
        let error = status_of(&vop_setattr(vp, &mut va, cred(nd), p));
        let postat = vop_getattr(vp, &mut va, cred(nd), p);
        postat_ok = postat.is_ok();
        if error == 0 {
            status_of(&postat)
        } else {
            error
        }
    };
    // out:
    vput(vp);
    let mut mb = match nfsm_reply(nd, slp, error, nfsx_wccorfattr(v3)) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        let before = preat_ok.then_some(&preat);
        nfsm_srvwcc(nd, before, postat_ok.then_some(&va), &mut mb);
    } else {
        nfsm_srvfattr_build(nd, &va, &mut mb);
    }
    finish(mb)
}

/// nfs lookup rpc
pub fn nfsrv_lookup<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let mut dirattr = Vattr::new();
    let mut dirattr_ok = false;
    let mut va = Vattr::new();
    let res = {
        let mut ni = ndinit_cred(LOOKUP, LOCKLEAF | SAVESTART, p, nd);
        let mut dirp = None;
        let res = namei(nd, &mut ni, &fh, len, slp, &mut r, &mut dirp, p);
        if let Some(dirp) = dirp {
            if v3 {
                dirattr_ok = vop_getattr(dirp, &mut dirattr, cred(nd), p).is_ok();
            }
            vrele(dirp);
        }
        res.map(|()| {
            startdir_rele(&mut ni);
            pnbuf_put(&mut ni.ni_cnd);
            let vp = ni_vp(&ni);
            let r = nfsrv_vptofh(vp)
                .and_then(|nfh| vop_getattr(vp, &mut va, cred(nd), p).map(|()| nfh));
            vput(vp);
            r
        })
    };
    let dirattr = dirattr_ok.then_some(&dirattr);
    let r = match res {
        Ok(r) => r,
        Err(error) => return reply_postop(nd, slp, error, nfsx_postopattr(v3), dirattr),
    };
    let siz = nfsx_srvfh(v3) + nfsx_postoporfattr(v3) + nfsx_postopattr(v3);
    let mut mb = match nfsm_reply(nd, slp, status_of(&r), siz) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    let Ok(nfh) = r else {
        nfsm_srvpostop_attr(nd, dirattr, &mut mb);
        return finish(mb);
    };
    nfsm_srvfhtom(&mut mb, &nfh, v3);
    if v3 {
        nfsm_srvpostop_attr(nd, Some(&va), &mut mb);
        nfsm_srvpostop_attr(nd, dirattr, &mut mb);
    } else {
        nfsm_srvfattr_build(nd, &va, &mut mb);
    }
    finish(mb)
}

/// The cluster `nfsrv_readlink` reads the link into (`MGET(mp, M_WAIT, MT_DATA);
/// MCLGET(mp, M_WAIT); mp->m_len = NFS_MAXPATHLEN`): `None` when no cluster can be had.
fn nfsrv_readlink_mbuf() -> Option<MbufBox> {
    // MLEN < NFS_MAXPATHLEN < MCLBYTES
    let mut m = m_get(M_WAIT, MT_DATA)?;
    mclget(&mut m, M_WAIT);
    if m.storage_len() < NFS_MAXPATHLEN {
        return None;
    }
    m.m_len().set(NFS_MAXPATHLEN as u32);
    Some(m)
}

/// nfs readlink service
pub fn nfsrv_readlink<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let fh = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((_, Some(fh))) => fh,
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (vp, _) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return reply_postop(nd, slp, error, 2 * NFSX_UNSIGNED, None),
    };
    let mut mp: Option<MbufBox> = None;
    let mut len = 0usize;
    let mut resid = 0usize;
    let r: Result<(), Errno> = if vp.vtype() != Vtype::VLNK {
        Err(if v3 { Errno::EINVAL } else { Errno::ENXIO })
    } else {
        match nfsrv_readlink_mbuf() {
            None => Err(Errno::ENOBUFS),
            Some(mut m) => {
                len = NFS_MAXPATHLEN;
                let r = match m.data_and_next_mut().0 {
                    Some(d) => {
                        let mut iov = [Iovec::new()];
                        let mut uio = Uio::kernel_read(&mut iov, d, None);
                        let r = vop_readlink(vp, &mut uio, cred(nd));
                        resid = uio.uio_resid;
                        r
                    }
                    None => Err(Errno::ENOBUFS),
                };
                mp = Some(m);
                r
            }
        }
    };
    // out:
    let mut attr = Vattr::new();
    let getret = vop_getattr(vp, &mut attr, cred(nd), p).is_ok();
    vput(vp);
    if r.is_err() {
        mp = None;
    }
    let error = status_of(&r);
    let mut mb = match nfsm_reply(nd, slp, error, nfsx_postopattr(v3) + NFSX_UNSIGNED) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        nfsm_srvpostop_attr(nd, getret.then_some(&attr), &mut mb);
        if error != 0 {
            return finish(mb);
        }
    }
    if resid > 0 {
        len -= resid;
        let tlen = nfsm_rndup(len);
        if let Some(m) = mp.as_deref_mut() {
            nfsm_adj(m, (NFS_MAXPATHLEN - tlen) as i32, (tlen - len) as i32);
        }
    }
    mb.u32(len as u32);
    finish_with(mb, mp)
}

/// `bad:`/`vbad:` of `nfsrv_read`: the error reply with the attributes fetched so far.
fn nfsrv_read_bad<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    error: i32,
    va: Option<&Vattr>,
) -> Answer {
    reply_postop(nd, slp, error, 0, va)
}

/// The data mbufs of a read of `len` bytes (the C's loop that generates the mbuf list for
/// the uio): an mbuf each, with a cluster while `MINCLSIZE` bytes or more are left, its
/// length the bytes it takes; the chain and the count of its mbufs, `None` when an mbuf
/// cannot be had.
fn nfsrv_read_chain(len: usize) -> Option<(MbufBox, usize)> {
    let mut chain = MChainBuilder::new();
    let mut left = len;
    let mut n = 0;
    while left > 0 {
        let mut m = m_get(M_WAIT, MT_DATA)?;
        if left >= MINCLSIZE {
            mclget(&mut m, M_WAIT);
        }
        let siz = usize::try_from(m_trailingspace(&m)).unwrap_or(0).min(left);
        if siz == 0 {
            return None;
        }
        m.m_len().set(siz as u32);
        left -= siz;
        n += 1;
        chain.push(m);
    }
    chain.into_head().map(|h| (h, n))
}

/// The data of every mbuf of the chain at `m`, in order, lent at once (the read's iovecs).
fn chain_data(m: &mut Mbuf) -> impl Iterator<Item = &mut [u8]> {
    let mut cur = Some(m);
    core::iter::from_fn(move || {
        let m = cur.take()?;
        let (d, next) = m.data_and_next_mut();
        cur = next;
        Some(d.unwrap_or_else(|| kpanic(format_args!("nfsrv_read: a read-only data mbuf"))))
    })
}

/// nfs read service
pub fn nfsrv_read<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let off: Off = if v3 {
        dissect!(nd, r.hyper()) as Off
    } else {
        Off::from(dissect!(nd, r.u32()))
    };
    let reqlen = dissect!(nd, r.i32());
    if reqlen > nfs_srvmaxdata(nd) as i32 || reqlen <= 0 {
        // nfsm_reply returns EBADRPC: the C returns 0.
        return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0);
    }

    let (vp, rdonly) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return nfsrv_read_bad(nd, slp, error, None),
    };
    let mut error = 0;
    if vp.vtype() != Vtype::VREG {
        error = if v3 {
            Errno::EINVAL
        } else if vp.vtype() == Vtype::VDIR {
            Errno::EISDIR
        } else {
            Errno::EACCES
        }
        .as_i32();
    }
    if error == 0 && nfsrv_access(vp, VREAD, &nd.nd_cr, rdonly, p, true).is_err() {
        error = status_of(&nfsrv_access(vp, VEXEC, &nd.nd_cr, rdonly, p, true));
    }
    let mut va = Vattr::new();
    let g = vop_getattr(vp, &mut va, cred(nd), p);
    let mut getret = g.is_ok();
    if error == 0 {
        error = status_of(&g);
    }
    if error != 0 {
        vput(vp);
        return nfsrv_read_bad(nd, slp, error, getret.then_some(&va));
    }

    let offu = off as u64;
    let mut cnt: usize = if offu >= va.va_size {
        0
    } else if offu.wrapping_add(reqlen as u64) > va.va_size {
        (va.va_size - offu) as usize
    } else {
        reqlen as usize
    };
    let len = nfsm_rndup(cnt);
    let mut data: Option<MbufBox> = None;
    let mut resid = 0usize;
    if cnt > 0 {
        // Generate the mbuf list with the uio_iov ref. to it.
        let Some((chain, n)) = nfsrv_read_chain(len) else {
            vput(vp);
            return nfsrv_read_bad(nd, slp, Errno::ENOBUFS.as_i32(), getret.then_some(&va));
        };
        let chain = data.insert(chain);
        let mut iv = temp_iovecs(n);
        let rr = {
            let mut uio = Uio::kernel_readv(&mut iv, chain_data(chain), None);
            uio.uio_offset = off;
            let rr = vop_read(vp, &mut uio, IO_NODELOCKED, cred(nd));
            resid = uio.uio_resid;
            rr
        };
        drop(iv);
        let error = match rr {
            Err(e) => e.as_i32(),
            Ok(()) => {
                let g = vop_getattr(vp, &mut va, cred(nd), p);
                getret = g.is_ok();
                status_of(&g)
            }
        };
        if error != 0 {
            vput(vp);
            return nfsrv_read_bad(nd, slp, error, getret.then_some(&va));
        }
    }
    vput(vp);
    let fp = nfsm_srvfattr(nd, &va);
    let tlen = len - resid;
    cnt = cnt.min(tlen);
    let tlen = nfsm_rndup(cnt);
    if (len != tlen || tlen != cnt)
        && let Some(m) = data.as_deref_mut()
    {
        nfsm_adj(m, (len - tlen) as i32, (tlen - cnt) as i32);
    }
    // The data joins the reply after the header: room for the header alone.
    let mut mb = match nfsm_reply(nd, slp, 0, nfsx_postoporfattr(v3) + 3 * NFSX_UNSIGNED) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    let fa = xdr_bytes(&fp);
    if v3 {
        let tl = mb.build::<{ NFSX_V3FATTR + 4 * NFSX_UNSIGNED }>();
        set_word(tl, 0, nfs_true);
        tl[NFSX_UNSIGNED..NFSX_UNSIGNED + NFSX_V3FATTR].copy_from_slice(&fa[..NFSX_V3FATTR]);
        let w = (NFSX_UNSIGNED + NFSX_V3FATTR) / NFSX_UNSIGNED;
        set_word(tl, w, txdr_unsigned(cnt as u32));
        set_word(
            tl,
            w + 1,
            if len < reqlen as usize {
                nfs_true
            } else {
                nfs_false
            },
        );
        set_word(tl, w + 2, txdr_unsigned(cnt as u32));
    } else {
        let tl = mb.build::<{ NFSX_V2FATTR + NFSX_UNSIGNED }>();
        tl[..NFSX_V2FATTR].copy_from_slice(&fa[..NFSX_V2FATTR]);
        set_word(tl, NFSX_V2FATTR / NFSX_UNSIGNED, txdr_unsigned(cnt as u32));
    }
    finish_with(mb, data)
}

/// `bad:`/`vbad:` of `nfsrv_write`: the error reply with the weak cache consistency data.
fn nfsrv_write_bad<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    error: i32,
    forat: Option<&Vattr>,
) -> Answer {
    reply_wcc(nd, slp, error, 0, forat, None)
}

/// The data of a write: the request's bytes at the cursor `r`, piece by piece as they lie in
/// its mbufs (the C's mbufs cut to them), at most `len` bytes, ending where the request does.
fn write_pieces<'a>(r: &XdrReader<'a>, len: usize) -> impl Iterator<Item = &'a [u8]> {
    let mut r = r.clone();
    let mut left = len;
    core::iter::from_fn(move || {
        if left == 0 {
            return None;
        }
        let piece = r.next_piece(left).ok()?;
        left -= piece.len();
        Some(piece)
    })
}

/// nfs write service
pub fn nfsrv_write<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    if nd.nd_mrep.is_none() {
        return (None, 0);
    }
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (off, stable, len): (Off, u32, i32) = if v3 {
        let off = dissect!(nd, r.hyper()) as Off;
        dissect!(nd, r.adv(NFSX_UNSIGNED));
        let stable = dissect!(nd, r.u32());
        (off, stable, dissect!(nd, r.i32()))
    } else {
        dissect!(nd, r.adv(NFSX_UNSIGNED));
        let off = Off::from(dissect!(nd, r.u32()));
        dissect!(nd, r.adv(NFSX_UNSIGNED));
        (off, NFSV3WRITE_FILESYNC, dissect!(nd, r.i32()))
    };
    let retlen = len;

    // For NFS Version 2, it is not obvious what a write of zero length should do, but I
    // might as well be consistent with Version 3, which is to return ok so long as there
    // are no permission problems.
    let (i, cnt) = match usize::try_from(len) {
        Ok(l) if l > 0 => {
            write_pieces(&r, l).fold((0usize, 0usize), |(i, c), piece| (i + piece.len(), c + 1))
        }
        _ => (0, 0),
    };
    if len > NFS_MAXDATA as i32 || len < 0 || i < len as usize {
        return nfsrv_write_bad(nd, slp, Errno::EIO.as_i32(), None);
    }
    let (vp, rdonly) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return nfsrv_write_bad(nd, slp, error, None),
    };
    let mut forat = Vattr::new();
    let forat_ok = v3 && vop_getattr(vp, &mut forat, cred(nd), p).is_ok();
    if vp.vtype() != Vtype::VREG {
        let error = if v3 {
            Errno::EINVAL
        } else if vp.vtype() == Vtype::VDIR {
            Errno::EISDIR
        } else {
            Errno::EACCES
        };
        vput(vp);
        return nfsrv_write_bad(nd, slp, error.as_i32(), forat_ok.then_some(&forat));
    }
    if let Err(e) = nfsrv_access(vp, VWRITE, &nd.nd_cr, rdonly, p, true) {
        vput(vp);
        return nfsrv_write_bad(nd, slp, e.as_i32(), forat_ok.then_some(&forat));
    }

    let mut w = Ok(());
    if len > 0 {
        let mut iv = temp_iovecs(cnt);
        let mut uio = Uio::kernel_writev(&mut iv, write_pieces(&r, len as usize), None);
        uio.uio_offset = off;
        // NFSV3WRITE_DATASYNC and NFSV3WRITE_FILESYNC are both written synchronously.
        let ioflags = if stable == NFSV3WRITE_UNSTABLE {
            IO_NODELOCKED
        } else {
            IO_SYNC | IO_NODELOCKED
        };
        w = vop_write(vp, &mut uio, ioflags, cred(nd));
        NFSSTATS.srvvop_writes.fetch_add(1, Relaxed);
    }
    let mut va = Vattr::new();
    let aftat = vop_getattr(vp, &mut va, cred(nd), p);
    vput(vp);
    let error = if w.is_ok() {
        status_of(&aftat)
    } else {
        status_of(&w)
    };
    let siz = nfsx_preopattr(v3) + nfsx_postoporfattr(v3) + 2 * NFSX_UNSIGNED + nfsx_writeverf(v3);
    let mut mb = match nfsm_reply(nd, slp, error, siz) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        let after = aftat.is_ok().then_some(&va);
        nfsm_srvwcc(nd, forat_ok.then_some(&forat), after, &mut mb);
        if error != 0 {
            return finish(mb);
        }
        let tl = mb.build::<{ 4 * NFSX_UNSIGNED }>();
        set_word(tl, 0, txdr_unsigned(retlen as u32));
        let how = if stable == NFSV3WRITE_UNSTABLE {
            stable
        } else {
            NFSV3WRITE_FILESYNC
        };
        set_word(tl, 1, txdr_unsigned(how));
        // Actually, there is no need to txdr these fields, but it may make the values more
        // human readable, for debugging purposes.
        let boottime = microboottime();
        set_word(tl, 2, txdr_unsigned(boottime.tv_sec as u32));
        set_word(tl, 3, txdr_unsigned(boottime.tv_usec as u32));
    } else {
        nfsm_srvfattr_build(nd, &va, &mut mb);
    }
    finish(mb)
}

/// `nfsm_srvpostop_fh(mb, fhp)`: a version 3 `post_op_fh3` holding the file handle.
fn nfsm_srvpostop_fh(mb: &mut XdrWriter, nfh: &Nfsfh) {
    let tl = mb.build::<{ 2 * NFSX_UNSIGNED + NFSX_V3FH }>();
    set_word(tl, 0, nfs_true);
    set_word(tl, 1, txdr_unsigned(NFSX_V3FH as u32));
    tl[2 * NFSX_UNSIGNED..].copy_from_slice(&nfh.fh_bytes[..NFSX_V3FH]);
}

/// The arguments of a CREATE after its name: the version 3 `createhow3` (`sattr3` or the
/// `createverf3`), or the version 2 `sattr`. `exists` is whether the name is there
/// (`nd.ni_vp`). The C's `error` (`EEXIST` for a `GUARDED` create of an existing file), or the
/// `goto nfsmout` of a request that ends first.
fn nfsrv_create_args(
    r: &mut XdrReader<'_>,
    v3: bool,
    exists: bool,
    va: &mut Vattr,
    cverf: &mut [u8; NFSX_V3CREATEVERF],
    exclusive_flag: &mut bool,
    rdev: &mut u32,
) -> Result<i32, Errno> {
    let mut error = 0;
    if v3 {
        match r.u32()? {
            NFSV3CREATE_GUARDED if exists => error = Errno::EEXIST.as_i32(),
            NFSV3CREATE_GUARDED | NFSV3CREATE_UNCHECKED => nfsm_srvsattr(r, va)?,
            NFSV3CREATE_EXCLUSIVE => {
                r.bytes_into(cverf)?;
                *exclusive_flag = true;
                if !exists {
                    va.va_mode = 0;
                }
            }
            _ => {}
        }
        va.va_type = Vtype::VREG;
    } else {
        let sp: Nfsv2Sattr = r.pod()?;
        va.va_type = iftovt(fxdr_unsigned(sp.sa_mode) as Mode);
        if va.va_type == Vtype::VNON {
            va.va_type = Vtype::VREG;
        }
        va.va_mode = nfstov_mode(sp.sa_mode);
        match va.va_type {
            Vtype::VREG => {
                let tsize = fxdr_unsigned(sp.sa_size) as i32;
                if tsize != -1 {
                    // `(u_quad_t)tsize`: sign-extended, as the C's.
                    va.va_size = tsize as u64;
                }
            }
            Vtype::VCHR | Vtype::VBLK | Vtype::VFIFO => *rdev = fxdr_unsigned(sp.sa_size),
            _ => {}
        }
    }
    Ok(error)
}

/// `nfsmout:` of `nfsrv_create` and `nfsrv_mknod`: releases what the name lookup returned
/// (the request's error is the caller's).
fn nfsrv_create_nfsmout<G: ProcGlue>(ni: &mut Nameidata<'_, G>, dirp: Option<&'static Vnode<G>>) {
    if let Some(dirp) = dirp {
        vrele(dirp);
    }
    if ni.ni_cnd.cn_nameiop != LOOKUP {
        startdir_rele(ni);
        pnbuf_put_hasbuf(&mut ni.ni_cnd);
    }
    abortop(ni);
    dvp_release(ni);
    if let Some(vp) = ni.ni_vp {
        vput(vp);
    }
}

/// nfs create service: now does a truncate to 0 length via setattr if it already exists.
pub fn nfsrv_create<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let mut ni = ndinit_cred(CREATE, LOCKPARENT | LOCKLEAF | SAVESTART, p, nd);
    let mut dirp = None;
    let res = namei(nd, &mut ni, &fh, len, slp, &mut r, &mut dirp, p);
    let mut dirfor = Vattr::new();
    let mut dirfor_ok = false;
    if let Some(d) = dirp {
        if v3 {
            dirfor_ok = vop_getattr(d, &mut dirfor, cred(nd), p).is_ok();
        } else {
            vrele(d);
            dirp = None;
        }
    }
    if let Err(error) = res {
        if let Some(d) = dirp {
            vrele(d);
        }
        drop(ni);
        let before = dirfor_ok.then_some(&dirfor);
        return reply_wcc(nd, slp, error, nfsx_wccdata(v3), before, None);
    }

    let mut va = Vattr::new();
    vattr_null(&mut va);
    let mut cverf = [0u8; NFSX_V3CREATEVERF];
    let mut exclusive_flag = false;
    let mut rdev: u32 = 0;
    let exists = ni.ni_vp.is_some();
    let args = nfsrv_create_args(
        &mut r,
        v3,
        exists,
        &mut va,
        &mut cverf,
        &mut exclusive_flag,
        &mut rdev,
    );
    let mut error = match args {
        Ok(error) => error,
        Err(e) => {
            nfsrv_create_nfsmout(&mut ni, dirp);
            drop(ni);
            return bad_request(nd, e);
        }
    };

    // Iff doesn't exist, create it
    // otherwise just truncate to 0 length
    //   should I set the mode too ??
    let vp: Option<&'static Vnode<G>>;
    if ni.ni_vp.is_none() {
        let dvp = ni_dvp(&ni);
        if va.va_type == Vtype::VREG || va.va_type == Vtype::VSOCK {
            startdir_rele(&mut ni);
            error = status_of(&vop_create(dvp, &mut ni.ni_vp, &mut ni.ni_cnd, &mut va));
            vput(dvp);
            if error == 0 {
                pnbuf_put(&mut ni.ni_cnd);
                if exclusive_flag {
                    exclusive_flag = false;
                    vattr_null(&mut va);
                    // bcopy(cverf, (caddr_t)&va.va_atime, NFSX_V3CREATEVERF)
                    va.va_atime.tv_sec = i64::from_ne_bytes(cverf);
                    let nvp = ni_vp(&ni);
                    error = status_of(&vop_setattr(nvp, &mut va, cred(nd), p));
                    if error != 0 {
                        // The C leaves the new file locked here.
                        vput(nvp);
                        ni.ni_vp = None;
                    }
                }
            }
        } else if matches!(va.va_type, Vtype::VCHR | Vtype::VBLK | Vtype::VFIFO) {
            if va.va_type == Vtype::VCHR && rdev == 0xffff_ffff {
                va.va_type = Vtype::VFIFO;
            }
            if va.va_type != Vtype::VFIFO
                && let Err(e) = suser_ucred(&nd.nd_cr)
            {
                startdir_rele(&mut ni);
                pnbuf_put_hasbuf(&mut ni.ni_cnd);
                let _ = vop_abortop(dvp, &mut ni.ni_cnd);
                vput(dvp);
                if let Some(d) = dirp {
                    vrele(d);
                }
                drop(ni);
                // Version 2 only: nfsm_reply ends the call.
                return nfsm_reply_only(nd, slp, e.as_i32(), 0);
            }
            va.va_rdev = rdev as Dev;
            let rr = vop_mknod(dvp, &mut ni.ni_vp, &mut ni.ni_cnd, &mut va);
            vput(dvp);
            if let Err(e) = rr {
                startdir_rele(&mut ni);
                // The file system gave the name buffer back (the C frees it again).
                if let Some(d) = dirp {
                    vrele(d);
                }
                drop(ni);
                return nfsm_reply_only(nd, slp, e.as_i32(), 0);
            }
            nfsrv_relookup_pathlen(&mut ni);
            ni.ni_cnd.cn_nameiop = LOOKUP;
            ni.ni_cnd.cn_flags &= !(LOCKPARENT | SAVESTART);
            ni.ni_cnd.cn_proc = p;
            ni.ni_cnd.cn_cred = cred(nd);
            if let Err(e) = vfs_lookup(&mut ni) {
                pnbuf_put_hasbuf(&mut ni.ni_cnd);
                if let Some(d) = dirp {
                    vrele(d);
                }
                drop(ni);
                return nfsm_reply_only(nd, slp, e.as_i32(), 0);
            }
            pnbuf_put(&mut ni.ni_cnd);
            if ni.ni_cnd.cn_flags & ISSYMLINK != 0 {
                if let Some(d) = ni.ni_dvp {
                    vrele(d);
                }
                vput(ni_vp(&ni));
                abortop(&mut ni);
                if let Some(d) = dirp {
                    vrele(d);
                }
                drop(ni);
                return nfsm_reply_only(nd, slp, Errno::EINVAL.as_i32(), 0);
            }
        } else {
            startdir_rele(&mut ni);
            pnbuf_put(&mut ni.ni_cnd);
            ni.ni_cnd.cn_flags &= !HASBUF;
            let _ = vop_abortop(dvp, &mut ni.ni_cnd);
            vput(dvp);
            error = Errno::ENXIO.as_i32();
        }
        vp = ni.ni_vp;
    } else {
        startdir_rele(&mut ni);
        pnbuf_put(&mut ni.ni_cnd);
        ni.ni_cnd.cn_flags &= !HASBUF;
        let evp = ni_vp(&ni);
        vp = Some(evp);
        dvp_release(&ni);
        abortop(&mut ni);
        if va.va_size != VNOVAL_SIZE {
            let rdonly = ni.ni_cnd.cn_flags & RDONLY != 0;
            let rr = nfsrv_access(evp, VWRITE, &nd.nd_cr, rdonly, p, false).and_then(|()| {
                let tempsize = va.va_size;
                vattr_null(&mut va);
                va.va_size = tempsize;
                vop_setattr(evp, &mut va, cred(nd), p)
            });
            error = status_of(&rr);
            if error != 0 {
                vput(evp);
            }
        } else if error != 0 {
            // A GUARDED create of an existing file: the C leaves it locked.
            vput(evp);
        }
    }
    let mut nfh = Nfsfh::new();
    if error == 0 {
        let vp = match vp {
            Some(vp) => vp,
            None => kpanic(format_args!("nfsrv_create: created without a vnode")),
        };
        error = match nfsrv_vptofh(vp) {
            Ok(f) => {
                nfh = f;
                status_of(&vop_getattr(vp, &mut va, cred(nd), p))
            }
            Err(e) => e.as_i32(),
        };
        vput(vp);
    }
    let mut diraft = Vattr::new();
    let mut diraft_ok = false;
    if v3 {
        if exclusive_flag && error == 0 && va.va_atime.tv_sec.to_ne_bytes() != cverf {
            error = Errno::EEXIST.as_i32();
        }
        if let Some(d) = dirp {
            diraft_ok = vop_getattr(d, &mut diraft, cred(nd), p).is_ok();
            vrele(d);
        }
    }
    drop(ni);
    let siz = nfsx_srvfh(v3) + nfsx_fattr(v3) + nfsx_wccdata(v3);
    let mut mb = match nfsm_reply(nd, slp, error, siz) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        if error == 0 {
            nfsm_srvpostop_fh(&mut mb, &nfh);
            nfsm_srvpostop_attr(nd, Some(&va), &mut mb);
        }
        let (before, after) = (dirfor_ok.then_some(&dirfor), diraft_ok.then_some(&diraft));
        nfsm_srvwcc(nd, before, after, &mut mb);
    } else {
        nfsm_srvfhtom(&mut mb, &nfh, v3);
        nfsm_srvfattr_build(nd, &va, &mut mb);
    }
    finish(mb)
}

/// nfs v3 mknod service
pub fn nfsrv_mknod<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let mut ni = ndinit_cred(CREATE, LOCKPARENT | LOCKLEAF | SAVESTART, p, nd);
    let mut dirp = None;
    let res = namei(nd, &mut ni, &fh, len, slp, &mut r, &mut dirp, p);
    let mut dirfor = Vattr::new();
    let mut dirfor_ok = false;
    if let Some(d) = dirp {
        dirfor_ok = vop_getattr(d, &mut dirfor, cred(nd), p).is_ok();
    }
    if let Err(error) = res {
        if let Some(d) = dirp {
            vrele(d);
        }
        drop(ni);
        let before = dirfor_ok.then_some(&dirfor);
        return reply_wcc(nd, slp, error, nfsx_wccdata(true), before, None);
    }

    let vtyp = match r.word() {
        Ok(w) => nfsv3tov_type(w),
        Err(e) => {
            nfsrv_create_nfsmout(&mut ni, dirp);
            drop(ni);
            return bad_request(nd, e);
        }
    };
    let mut va = Vattr::new();
    let error = 'out: {
        if !matches!(
            vtyp,
            Vtype::VCHR | Vtype::VBLK | Vtype::VSOCK | Vtype::VFIFO
        ) {
            startdir_rele(&mut ni);
            pnbuf_put(&mut ni.ni_cnd);
            abortop(&mut ni);
            dvp_release(&ni);
            if let Some(vp) = ni.ni_vp {
                vput(vp);
            }
            break 'out NFSERR_BADTYPE;
        }
        vattr_null(&mut va);
        let mut rdev = None;
        let args = nfsm_srvsattr(&mut r, &mut va).and_then(|()| {
            if matches!(vtyp, Vtype::VCHR | Vtype::VBLK) {
                let major = r.u32()?;
                rdev = Some(makedev(major, r.u32()?));
            }
            Ok(())
        });
        if let Err(e) = args {
            nfsrv_create_nfsmout(&mut ni, dirp);
            drop(ni);
            return bad_request(nd, e);
        }
        if let Some(rdev) = rdev {
            va.va_rdev = rdev;
        }

        // Iff doesn't exist, create it.
        if let Some(vp) = ni.ni_vp {
            startdir_rele(&mut ni);
            pnbuf_put(&mut ni.ni_cnd);
            abortop(&mut ni);
            dvp_release(&ni);
            vput(vp);
            break 'out Errno::EEXIST.as_i32();
        }
        let dvp = ni_dvp(&ni);
        va.va_type = vtyp;
        if vtyp == Vtype::VSOCK {
            startdir_rele(&mut ni);
            let rr = vop_create(dvp, &mut ni.ni_vp, &mut ni.ni_cnd, &mut va);
            vput(dvp);
            if rr.is_ok() {
                pnbuf_put(&mut ni.ni_cnd);
            }
            break 'out status_of(&rr);
        }
        if va.va_type != Vtype::VFIFO
            && let Err(e) = suser_ucred(&nd.nd_cr)
        {
            startdir_rele(&mut ni);
            pnbuf_put(&mut ni.ni_cnd);
            let _ = vop_abortop(dvp, &mut ni.ni_cnd);
            vput(dvp);
            break 'out e.as_i32();
        }
        let rr = vop_mknod(dvp, &mut ni.ni_vp, &mut ni.ni_cnd, &mut va);
        vput(dvp);
        if let Err(e) = rr {
            startdir_rele(&mut ni);
            break 'out e.as_i32();
        }
        nfsrv_relookup_pathlen(&mut ni);
        ni.ni_cnd.cn_nameiop = LOOKUP;
        ni.ni_cnd.cn_flags &= !(LOCKPARENT | SAVESTART);
        ni.ni_cnd.cn_proc = p;
        // The nfsd's own credentials (`procp->p_ucred`), as the C's.
        ni.ni_cnd.cn_cred = Cred::real(p.ucred());
        let rr = vfs_lookup(&mut ni);
        pnbuf_put(&mut ni.ni_cnd);
        if let Err(e) = rr {
            break 'out e.as_i32();
        }
        if ni.ni_cnd.cn_flags & ISSYMLINK != 0 {
            if let Some(d) = ni.ni_dvp {
                vrele(d);
            }
            vput(ni_vp(&ni));
            abortop(&mut ni);
            break 'out Errno::EINVAL.as_i32();
        }
        0
    };
    // out:
    let mut error = error;
    let mut nfh = Nfsfh::new();
    if error == 0 {
        let vp = ni_vp(&ni);
        error = match nfsrv_vptofh(vp) {
            Ok(f) => {
                nfh = f;
                status_of(&vop_getattr(vp, &mut va, cred(nd), p))
            }
            Err(e) => e.as_i32(),
        };
        vput(vp);
    }
    let mut diraft = Vattr::new();
    let mut diraft_ok = false;
    if let Some(d) = dirp {
        diraft_ok = vop_getattr(d, &mut diraft, cred(nd), p).is_ok();
        vrele(d);
    }
    drop(ni);
    let siz = nfsx_srvfh(true) + nfsx_postopattr(true) + nfsx_wccdata(true);
    let mut mb = match nfsm_reply(nd, slp, error, siz) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if error == 0 {
        nfsm_srvpostop_fh(&mut mb, &nfh);
        nfsm_srvpostop_attr(nd, Some(&va), &mut mb);
    }
    let (before, after) = (dirfor_ok.then_some(&dirfor), diraft_ok.then_some(&diraft));
    nfsm_srvwcc(nd, before, after, &mut mb);
    finish(mb)
}

/// nfs remove service
pub fn nfsrv_remove<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let mut ni = ndinit_cred(DELETE, LOCKPARENT | LOCKLEAF, p, nd);
    let mut dirp = None;
    let res = namei(nd, &mut ni, &fh, len, slp, &mut r, &mut dirp, p);
    let mut dirfor = Vattr::new();
    let mut dirfor_ok = false;
    if let Some(d) = dirp {
        if v3 {
            dirfor_ok = vop_getattr(d, &mut dirfor, cred(nd), p).is_ok();
        } else {
            vrele(d);
            dirp = None;
        }
    }

    let error = match res {
        Err(error) => error,
        Ok(()) => {
            let vp = ni_vp(&ni);
            let error = 'out: {
                if vp.vtype() == Vtype::VDIR
                    && let Err(e) = suser_ucred(&nd.nd_cr)
                {
                    break 'out e.as_i32();
                }
                // The root of a mounted filesystem cannot be deleted.
                if vp.flag() & VROOT != 0 {
                    break 'out Errno::EBUSY.as_i32();
                }
                if vp.flag() & VTEXT != 0 {
                    let _ = uvm_vnp_uncache(vp);
                }
                0
            };
            // out:
            if error == 0 {
                status_of(&vop_remove(ni_dvp(&ni), vp, &mut ni.ni_cnd))
            } else {
                abortop(&mut ni);
                dvp_release(&ni);
                vput(vp);
                error
            }
        }
    };
    let mut diraft = Vattr::new();
    let mut diraft_ok = false;
    if v3 && let Some(d) = dirp {
        diraft_ok = vop_getattr(d, &mut diraft, cred(nd), p).is_ok();
        vrele(d);
    }
    drop(ni);
    let mut mb = match nfsm_reply(nd, slp, error, nfsx_wccdata(v3)) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        let (before, after) = (dirfor_ok.then_some(&dirfor), diraft_ok.then_some(&diraft));
        nfsm_srvwcc(nd, before, after, &mut mb);
    }
    finish(mb)
}

/// `nfsmout:` of `nfsrv_rename` once the source was looked up: releases what that lookup
/// returned (the reply or the error is the caller's).
fn nfsrv_rename_nfsmout<G: ProcGlue>(
    fromnd: &mut Nameidata<'_, G>,
    fdirp: Option<&'static Vnode<G>>,
    fvp: Option<&'static Vnode<G>>,
) {
    if let Some(d) = fdirp {
        vrele(d);
    }
    if fromnd.ni_cnd.cn_nameiop != LOOKUP {
        startdir_rele(fromnd);
        abortop(fromnd);
        // The C skips this when ni_dvp is fdirp ("XXX: Workaround"), which leaks the
        // lookup's own reference: fdirp holds another one, released above.
        if let Some(dvp) = fromnd.ni_dvp {
            vrele(dvp);
        }
        if let Some(fvp) = fvp {
            vrele(fvp);
        }
        pnbuf_put(&mut fromnd.ni_cnd);
    }
}

/// nfs rename service
pub fn nfsrv_rename<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, ffh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    // Remember our original uid so that we can reset cr_uid before the second nfs_namei()
    // call, in case it is remapped.
    let saved_uid = nd.nd_cr.cr_uid.get();

    let mut fromnd = ndinit_cred(DELETE, WANTPARENT | SAVESTART, p, nd);
    let mut fdirp = None;
    let res = namei(nd, &mut fromnd, &ffh, len, slp, &mut r, &mut fdirp, p);
    let mut fdirfor = Vattr::new();
    let mut fdirfor_ok = false;
    if let Some(d) = fdirp {
        if v3 {
            fdirfor_ok = vop_getattr(d, &mut fdirfor, cred(nd), p).is_ok();
        } else {
            vrele(d);
            fdirp = None;
        }
    }
    if let Err(error) = res {
        if let Some(d) = fdirp {
            vrele(d);
        }
        drop(fromnd);
        return match nfsm_reply(nd, slp, error, 2 * nfsx_wccdata(v3)) {
            Ok(mut mb) => {
                nfsm_srvwcc(nd, fdirfor_ok.then_some(&fdirfor), None, &mut mb);
                nfsm_srvwcc(nd, None, None, &mut mb);
                finish(mb)
            }
            Err(done) => done,
        };
    }

    let fvp = ni_vp(&fromnd);
    let tfh = match nfsm_srvmtofh(&mut r, v3) {
        Ok(Some(f)) => f,
        Ok(None) => {
            nfsrv_rename_nfsmout(&mut fromnd, fdirp, Some(fvp));
            drop(fromnd);
            return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0);
        }
        Err(e) => {
            nfsrv_rename_nfsmout(&mut fromnd, fdirp, Some(fvp));
            drop(fromnd);
            return bad_request(nd, e);
        }
    };
    let len2 = match r.strsiz(NFS_MAXNAMLEN as i32) {
        Ok(l) => l as usize,
        Err(e) => {
            nfsrv_rename_nfsmout(&mut fromnd, fdirp, Some(fvp));
            drop(fromnd);
            return bad_request(nd, e);
        }
    };
    nd.nd_cr.set_uid(saved_uid);

    let mut tond = ndinit_cred(RENAME, LOCKPARENT | LOCKLEAF | NOCACHE | SAVESTART, p, nd);
    let mut tdirp = None;
    let res = namei(nd, &mut tond, &tfh, len2, slp, &mut r, &mut tdirp, p);
    let mut tdirfor = Vattr::new();
    let mut tdirfor_ok = false;
    if let Some(d) = tdirp {
        if v3 {
            tdirfor_ok = vop_getattr(d, &mut tdirfor, cred(nd), p).is_ok();
        } else {
            vrele(d);
            tdirp = None;
        }
    }
    let fdvp = ni_dvp(&fromnd);
    let error = match res {
        Err(error) => {
            let _ = vop_abortop(fdvp, &mut fromnd.ni_cnd);
            vrele(fdvp);
            vrele(fvp);
            error
        }
        Ok(()) => {
            let tdvp = ni_dvp(&tond);
            let tvp = tond.ni_vp;
            let pick = |e3: Errno, e2: Errno| if v3 { e3 } else { e2 }.as_i32();
            let mut error = 'out: {
                if let Some(tvp) = tvp {
                    let fdir = fvp.vtype() == Vtype::VDIR;
                    let tdir = tvp.vtype() == Vtype::VDIR;
                    if fdir && !tdir {
                        break 'out pick(Errno::EEXIST, Errno::EISDIR);
                    } else if !fdir && tdir {
                        break 'out pick(Errno::EEXIST, Errno::ENOTDIR);
                    }
                    if tdir && tvp.mountedhere().is_some() {
                        break 'out pick(Errno::EXDEV, Errno::ENOTEMPTY);
                    }
                }
                if fvp.vtype() == Vtype::VDIR && fvp.mountedhere().is_some() {
                    break 'out pick(Errno::EXDEV, Errno::ENOTEMPTY);
                }
                if !same_mount(fvp, tdvp) {
                    break 'out pick(Errno::EXDEV, Errno::ENOTEMPTY);
                }
                let mut error = 0;
                if ptr::eq(fvp, tdvp) {
                    error = pick(Errno::EINVAL, Errno::ENOTEMPTY);
                }
                // If source is the same as the destination (that is the same vnode with the
                // same name in the same directory), then there is nothing to do.
                if same_vnode(Some(fvp), tvp)
                    && ptr::eq(fdvp, tdvp)
                    && fromnd.ni_cnd.name() == tond.ni_cnd.name()
                {
                    error = -1;
                }
                error
            };
            // out:
            if error == 0 {
                if let Some(tvp) = tvp {
                    let _ = uvm_vnp_uncache(tvp);
                }
                error = status_of(&vop_rename(
                    fdvp,
                    fvp,
                    &mut fromnd.ni_cnd,
                    tdvp,
                    tvp,
                    &mut tond.ni_cnd,
                ));
            } else {
                let _ = vop_abortop(tdvp, &mut tond.ni_cnd);
                if same_vnode(Some(tdvp), tvp) {
                    vrele(tdvp);
                } else {
                    vput(tdvp);
                }
                if let Some(tvp) = tvp {
                    vput(tvp);
                }
                let _ = vop_abortop(fdvp, &mut fromnd.ni_cnd);
                vrele(fdvp);
                vrele(fvp);
                if error == -1 {
                    error = 0;
                }
            }
            startdir_rele(&mut tond);
            pnbuf_put(&mut tond.ni_cnd);
            error
        }
    };
    // out1:
    let mut fdiraft = Vattr::new();
    let mut fdiraft_ok = false;
    if let Some(d) = fdirp {
        fdiraft_ok = vop_getattr(d, &mut fdiraft, cred(nd), p).is_ok();
        vrele(d);
    }
    let mut tdiraft = Vattr::new();
    let mut tdiraft_ok = false;
    if let Some(d) = tdirp {
        tdiraft_ok = vop_getattr(d, &mut tdiraft, cred(nd), p).is_ok();
        vrele(d);
    }
    startdir_rele(&mut fromnd);
    pnbuf_put(&mut fromnd.ni_cnd);
    drop(tond);
    drop(fromnd);
    let mut mb = match nfsm_reply(nd, slp, error, 2 * nfsx_wccdata(v3)) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        let (before, after) = (
            fdirfor_ok.then_some(&fdirfor),
            fdiraft_ok.then_some(&fdiraft),
        );
        nfsm_srvwcc(nd, before, after, &mut mb);
        let (before, after) = (
            tdirfor_ok.then_some(&tdirfor),
            tdiraft_ok.then_some(&tdiraft),
        );
        nfsm_srvwcc(nd, before, after, &mut mb);
    }
    finish(mb)
}

/// nfs link service
pub fn nfsrv_link<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let dfh = match nfsm_srvmtofh(&mut r, v3) {
        Ok(Some(f)) => f,
        Ok(None) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let (vp, _) = match fhtovp(nd, &fh, false, slp) {
        Ok(x) => x,
        Err(error) => {
            return match nfsm_reply(nd, slp, error, nfsx_postopattr(v3) + nfsx_wccdata(v3)) {
                Ok(mut mb) => {
                    nfsm_srvpostop_attr(nd, None, &mut mb);
                    nfsm_srvwcc(nd, None, None, &mut mb);
                    finish(mb)
                }
                Err(done) => done,
            };
        }
    };
    let mut dirp = None;
    let mut dirfor = Vattr::new();
    let mut dirfor_ok = false;
    let mut error = 0;
    let mut ni = ndinit_cred(CREATE, LOCKPARENT, p, nd);
    // A directory is not linked, and not refused either: the C's `goto out1` with no error.
    if vp.vtype() != Vtype::VDIR {
        let res = namei(nd, &mut ni, &dfh, len, slp, &mut r, &mut dirp, p);
        if let Some(d) = dirp {
            if v3 {
                dirfor_ok = vop_getattr(d, &mut dirfor, cred(nd), p).is_ok();
            } else {
                vrele(d);
                dirp = None;
            }
        }
        error = match res {
            Err(error) => error,
            Ok(()) => {
                let dvp = ni_dvp(&ni);
                let error = if ni.ni_vp.is_some() {
                    Errno::EEXIST.as_i32()
                } else if !same_mount(vp, dvp) {
                    Errno::EXDEV.as_i32()
                } else {
                    0
                };
                // out:
                if error == 0 {
                    status_of(&vop_link(dvp, vp, &mut ni.ni_cnd))
                } else {
                    abortop(&mut ni);
                    dvp_release(&ni);
                    if let Some(xp) = ni.ni_vp {
                        vrele(xp);
                    }
                    error
                }
            }
        };
    }
    // out1:
    let mut at = Vattr::new();
    let getret = v3 && vop_getattr(vp, &mut at, cred(nd), p).is_ok();
    let mut diraft = Vattr::new();
    let mut diraft_ok = false;
    if let Some(d) = dirp {
        diraft_ok = vop_getattr(d, &mut diraft, cred(nd), p).is_ok();
        vrele(d);
    }
    vrele(vp);
    drop(ni);
    let mut mb = match nfsm_reply(nd, slp, error, nfsx_postopattr(v3) + nfsx_wccdata(v3)) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);
        let (before, after) = (dirfor_ok.then_some(&dirfor), diraft_ok.then_some(&diraft));
        nfsm_srvwcc(nd, before, after, &mut mb);
    }
    finish(mb)
}

/// `nfsmout:` of `nfsrv_symlink`: releases what the lookup returned (the request's error is
/// the caller's; the target buffer goes with its owner).
fn nfsrv_symlink_nfsmout<G: ProcGlue>(ni: &mut Nameidata<'_, G>, dirp: Option<&'static Vnode<G>>) {
    if ni.ni_cnd.cn_nameiop != LOOKUP {
        startdir_rele(ni);
        pnbuf_put(&mut ni.ni_cnd);
    }
    if let Some(d) = dirp {
        vrele(d);
    }
    abortop(ni);
    dvp_release(ni);
    if let Some(vp) = ni.ni_vp {
        vrele(vp);
    }
}

/// nfs symbolic link service
pub fn nfsrv_symlink<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let mut ni = ndinit_cred(CREATE, LOCKPARENT | SAVESTART, p, nd);
    let mut dirp = None;
    let res = namei(nd, &mut ni, &fh, len, slp, &mut r, &mut dirp, p);
    let mut dirfor = Vattr::new();
    let mut dirfor_ok = false;
    if let Some(d) = dirp {
        if v3 {
            dirfor_ok = vop_getattr(d, &mut dirfor, cred(nd), p).is_ok();
        } else {
            vrele(d);
            dirp = None;
        }
    }
    let mut va = Vattr::new();
    let mut nfh = Nfsfh::new();
    let error = 'out: {
        if let Err(error) = res {
            break 'out error;
        }
        vattr_null(&mut va);
        // The target, its length bounded by NFS_MAXPATHLEN, copied out of the request (the
        // C's nfsd_mtouio into a malloc'd buffer), then the version 2 attributes.
        let args = (|| -> Result<(MArray<u8>, usize), Errno> {
            if v3 {
                nfsm_srvsattr(&mut r, &mut va)?;
            }
            let len2 = r.strsiz(NFS_MAXPATHLEN as i32)? as usize;
            let mut pathcp = temp_bytes(len2 + 1);
            r.bytes_into(&mut pathcp[..len2])?;
            r.adv(nfsm_padlen(len2))?;
            if !v3 {
                let sp: Nfsv2Sattr = r.pod()?;
                va.va_mode = nfstov_mode(sp.sa_mode);
            }
            Ok((pathcp, len2))
        })();
        let (pathcp, len2) = match args {
            Ok(x) => x,
            Err(e) => {
                nfsrv_symlink_nfsmout(&mut ni, dirp);
                drop(ni);
                return bad_request(nd, e);
            }
        };
        if let Some(vp) = ni.ni_vp {
            startdir_rele(&mut ni);
            pnbuf_put(&mut ni.ni_cnd);
            abortop(&mut ni);
            dvp_release(&ni);
            vrele(vp);
            break 'out Errno::EEXIST.as_i32();
        }
        // The target is the C's string: it ends at its first NUL (`strlen`).
        let target = &pathcp[..len2];
        let target = &target[..target.iter().position(|&c| c == 0).unwrap_or(len2)];
        let dvp = ni_dvp(&ni);
        let mut error = status_of(&vop_symlink(
            dvp,
            &mut ni.ni_vp,
            &mut ni.ni_cnd,
            &mut va,
            target,
        ));
        if error != 0 {
            startdir_rele(&mut ni);
        } else {
            if v3 {
                nfsrv_relookup_pathlen(&mut ni);
                ni.ni_cnd.cn_nameiop = LOOKUP;
                ni.ni_cnd.cn_flags &= !(LOCKPARENT | SAVESTART | FOLLOW);
                ni.ni_cnd.cn_flags |= NOFOLLOW | LOCKLEAF;
                ni.ni_cnd.cn_proc = p;
                ni.ni_cnd.cn_cred = cred(nd);
                error = status_of(&vfs_lookup(&mut ni));
                if error == 0 {
                    let vp = ni_vp(&ni);
                    error = match nfsrv_vptofh(vp) {
                        Ok(f) => {
                            nfh = f;
                            status_of(&vop_getattr(vp, &mut va, cred(nd), p))
                        }
                        Err(e) => e.as_i32(),
                    };
                    vput(vp);
                }
            } else {
                startdir_rele(&mut ni);
            }
            pnbuf_put(&mut ni.ni_cnd);
        }
        error
    };
    // out:
    let mut diraft = Vattr::new();
    let mut diraft_ok = false;
    if let Some(d) = dirp {
        diraft_ok = vop_getattr(d, &mut diraft, cred(nd), p).is_ok();
        vrele(d);
    }
    drop(ni);
    let siz = nfsx_srvfh(v3) + nfsx_postopattr(v3) + nfsx_wccdata(v3);
    let mut mb = match nfsm_reply(nd, slp, error, siz) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        if error == 0 {
            nfsm_srvpostop_fh(&mut mb, &nfh);
            nfsm_srvpostop_attr(nd, Some(&va), &mut mb);
        }
        let (before, after) = (dirfor_ok.then_some(&dirfor), diraft_ok.then_some(&diraft));
        nfsm_srvwcc(nd, before, after, &mut mb);
    }
    finish(mb)
}

/// `nfsmout:` of `nfsrv_mkdir`: releases what the lookup returned (the request's error is
/// the caller's).
fn nfsrv_mkdir_nfsmout<G: ProcGlue>(ni: &mut Nameidata<'_, G>, dirp: Option<&'static Vnode<G>>) {
    if let Some(d) = dirp {
        vrele(d);
    }
    abortop(ni);
    dvp_release(ni);
    if let Some(vp) = ni.ni_vp {
        vrele(vp);
    }
}

/// nfs mkdir service
pub fn nfsrv_mkdir<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let mut ni = ndinit_cred(CREATE, LOCKPARENT, p, nd);
    let mut dirp = None;
    let res = namei(nd, &mut ni, &fh, len, slp, &mut r, &mut dirp, p);
    let mut dirfor = Vattr::new();
    let mut dirfor_ok = false;
    if let Some(d) = dirp {
        if v3 {
            dirfor_ok = vop_getattr(d, &mut dirfor, cred(nd), p).is_ok();
        } else {
            vrele(d);
            dirp = None;
        }
    }
    if let Err(error) = res {
        if let Some(d) = dirp {
            vrele(d);
        }
        drop(ni);
        let before = dirfor_ok.then_some(&dirfor);
        return reply_wcc(nd, slp, error, nfsx_wccdata(v3), before, None);
    }

    let mut va = Vattr::new();
    vattr_null(&mut va);
    let args = if v3 {
        nfsm_srvsattr(&mut r, &mut va)
    } else {
        r.word().map(|w| va.va_mode = nfstov_mode(w))
    };
    if let Err(e) = args {
        nfsrv_mkdir_nfsmout(&mut ni, dirp);
        drop(ni);
        return bad_request(nd, e);
    }
    va.va_type = Vtype::VDIR;
    let mut nfh = Nfsfh::new();
    let error = if let Some(vp) = ni.ni_vp {
        abortop(&mut ni);
        dvp_release(&ni);
        vrele(vp);
        Errno::EEXIST.as_i32()
    } else {
        let dvp = ni_dvp(&ni);
        match vop_mkdir(dvp, &mut ni.ni_vp, &mut ni.ni_cnd, &mut va) {
            Ok(()) => {
                let vp = ni_vp(&ni);
                let error = match nfsrv_vptofh(vp) {
                    Ok(f) => {
                        nfh = f;
                        status_of(&vop_getattr(vp, &mut va, cred(nd), p))
                    }
                    Err(e) => e.as_i32(),
                };
                vput(vp);
                error
            }
            Err(e) => e.as_i32(),
        }
    };
    // out:
    let mut diraft = Vattr::new();
    let mut diraft_ok = false;
    if let Some(d) = dirp {
        diraft_ok = vop_getattr(d, &mut diraft, cred(nd), p).is_ok();
        vrele(d);
    }
    drop(ni);
    let siz = nfsx_srvfh(v3) + nfsx_postopattr(v3) + nfsx_wccdata(v3);
    let mut mb = match nfsm_reply(nd, slp, error, siz) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        if error == 0 {
            nfsm_srvpostop_fh(&mut mb, &nfh);
            nfsm_srvpostop_attr(nd, Some(&va), &mut mb);
        }
        let (before, after) = (dirfor_ok.then_some(&dirfor), diraft_ok.then_some(&diraft));
        nfsm_srvwcc(nd, before, after, &mut mb);
    } else {
        nfsm_srvfhtom(&mut mb, &nfh, v3);
        nfsm_srvfattr_build(nd, &va, &mut mb);
    }
    finish(mb)
}

/// nfs rmdir service
pub fn nfsrv_rmdir<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (len, error) = dissect!(nd, nfsm_srvnamesiz(&mut r));
    if error != 0 {
        // nfsm_reply would return zero if v3 and an error different from EBADRPC. But it
        // does not make sense to continue anyway if the error set in nfsm_srvnamesiz is
        // NFSERR_NAMETOL.
        return nfsm_reply_only(nd, slp, error, 0);
    }

    let mut ni = ndinit_cred(DELETE, LOCKPARENT | LOCKLEAF, p, nd);
    let mut dirp = None;
    let res = namei(nd, &mut ni, &fh, len, slp, &mut r, &mut dirp, p);
    let mut dirfor = Vattr::new();
    let mut dirfor_ok = false;
    if let Some(d) = dirp {
        if v3 {
            dirfor_ok = vop_getattr(d, &mut dirfor, cred(nd), p).is_ok();
        } else {
            vrele(d);
            dirp = None;
        }
    }
    if let Err(error) = res {
        if let Some(d) = dirp {
            vrele(d);
        }
        drop(ni);
        let before = dirfor_ok.then_some(&dirfor);
        return reply_wcc(nd, slp, error, nfsx_wccdata(v3), before, None);
    }
    let vp = ni_vp(&ni);
    let dvp = ni_dvp(&ni);
    let error = 'out: {
        if vp.vtype() != Vtype::VDIR {
            break 'out Errno::ENOTDIR.as_i32();
        }
        // No rmdir "." please.
        if ptr::eq(dvp, vp) {
            break 'out Errno::EINVAL.as_i32();
        }
        // A mounted on directory cannot be deleted.
        if vp.mountedhere().is_some() {
            break 'out Errno::EBUSY.as_i32();
        }
        // The root of a mounted filesystem cannot be deleted.
        if vp.flag() & VROOT != 0 {
            break 'out Errno::EBUSY.as_i32();
        }
        0
    };
    // out:
    let error = if error == 0 {
        status_of(&vop_rmdir(dvp, vp, &mut ni.ni_cnd))
    } else {
        abortop(&mut ni);
        dvp_release(&ni);
        vput(vp);
        error
    };
    let mut diraft = Vattr::new();
    let mut diraft_ok = false;
    if let Some(d) = dirp {
        diraft_ok = vop_getattr(d, &mut diraft, cred(nd), p).is_ok();
        vrele(d);
    }
    drop(ni);
    let mut mb = match nfsm_reply(nd, slp, error, nfsx_wccdata(v3)) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        let (before, after) = (dirfor_ok.then_some(&dirfor), diraft_ok.then_some(&diraft));
        nfsm_srvwcc(nd, before, after, &mut mb);
    }
    finish(mb)
}

/// The directory entry at the start of `buf` and its name, or `None` when the buffer ends
/// before the entry does (or its `d_reclen` is zero, which would never move on).
fn nfsrv_dirent(buf: &[u8]) -> Option<(Dirent, &[u8])> {
    let dp = Dirent::from_bytes(buf)?;
    let reclen = usize::from(dp.d_reclen);
    let name_end = Dirent::NAME_OFFSET + usize::from(dp.d_namlen);
    if reclen == 0 || reclen > buf.len() || name_end > buf.len() {
        return None;
    }
    Some((dp, &buf[Dirent::NAME_OFFSET..name_end]))
}

/// The offset in `buf` (what `VOP_READDIR` read) of its first entry with a file number: the
/// C's walk past the entries with `d_fileno == 0` (`buf.len()` when there is none).
fn nfsrv_dirent_skip(buf: &[u8]) -> usize {
    let mut cpos = 0;
    while cpos < buf.len() {
        match nfsrv_dirent(&buf[cpos..]) {
            Some((dp, _)) if dp.d_fileno == 0 => cpos += usize::from(dp.d_reclen),
            Some(_) => return cpos,
            None => return buf.len(),
        }
    }
    cpos
}

/// The loop of `nfsrv_readdir` that builds the reply: packs the entries of `buf` (the
/// `struct dirent`s from `cpos` to `cend`) as READDIR `entry`s at `mb`, skipping the ones
/// without a file number, while the reply counted in `len` (`3 * NFSX_UNSIGNED` of paranoia
/// to start) stays within the client's `cnt` bytes. `false` when an entry did not fit (the
/// C's `eofflag = 0; break`).
fn nfsrv_readdir_pack(mb: &mut XdrWriter, buf: &[u8], v3: bool, cnt: i32, mut len: i32) -> bool {
    let mut cpos = 0;
    while cpos < buf.len() {
        let Some((dp, name)) = nfsrv_dirent(&buf[cpos..]) else {
            break;
        };
        if dp.d_fileno != 0 {
            let nlen = name.len();
            let pad = nfsm_padlen(nlen);
            len += (4 * NFSX_UNSIGNED + nlen + pad) as i32;
            if v3 {
                len += (2 * NFSX_UNSIGNED) as i32;
            }
            if len > cnt {
                return false;
            }
            // Build the directory record xdr from the dirent entry.
            if v3 {
                let tl = mb.build::<{ 3 * NFSX_UNSIGNED }>();
                set_word(tl, 0, nfs_true);
                tl[NFSX_UNSIGNED..].copy_from_slice(&dp.d_fileno.to_be_bytes());
            } else {
                let tl = mb.build::<{ 2 * NFSX_UNSIGNED }>();
                set_word(tl, 0, nfs_true);
                set_word(tl, 1, txdr_unsigned(dp.d_fileno as u32));
            }

            // And copy the name
            mb.strtombuf(name);

            // Finish off the record
            if v3 {
                mb.hyper(dp.d_off as u64);
            } else {
                mb.u32(dp.d_off as u32);
            }
        }
        cpos += usize::from(dp.d_reclen);
    }
    true
}

/// `if (cnt > xfer || cnt < 0) cnt = xfer;`: the client's byte count limited to
/// `NFS_SRVMAXDATA` (`xfer`).
fn nfsrv_readdir_cnt(cnt: i32, xfer: i32) -> i32 {
    if cnt > xfer || cnt < 0 { xfer } else { cnt }
}

/// The size of the buffer for `VOP_READDIR`: `siz` rounded up to a multiple of `DIRBLKSIZ`,
/// `xfer` when that is larger than `xfer` or not positive.
fn nfsrv_readdir_siz(siz: i32, xfer: i32) -> usize {
    let blk = DIRBLKSIZ as i32;
    let siz = siz.wrapping_add(blk - 1) & !(blk - 1);
    (if siz > xfer || siz <= 0 { xfer } else { siz }) as usize
}

/// One pass of a readdir over `vp` (unlocked) from the cookie `off`: `vn_lock`,
/// `VOP_READDIR` into `rbuf`, the attributes when `at` is asked for, `VOP_UNLOCK`. The read's
/// result, the bytes read, the offset it ended at, the eof flag and the attributes' result.
fn nfsrv_readdir_pass<G: ProcGlue>(
    vp: &'static Vnode<G>,
    rbuf: &mut [u8],
    off: u64,
    cred: Cred<'_>,
    p: &Thread<G>,
    at: Option<&mut Vattr>,
) -> (
    Result<(), Errno>,
    usize,
    u64,
    i32,
    Option<Result<(), Errno>>,
) {
    let fullsiz = rbuf.len();
    let mut iv = [Iovec::new()];
    let mut io = Uio::<G>::kernel_read(&mut iv, rbuf, None);
    io.uio_offset = off as Off;
    let mut eofflag = 0;

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let rr = vop_readdir(vp, &mut io, cred, &mut eofflag);

    let off = io.uio_offset as u64;
    let siz = fullsiz - io.uio_resid;
    let g = at.map(|at| vop_getattr(vp, at, cred, p));

    let _ = vop_unlock(vp);
    (rr, siz, off, eofflag, g)
}

/// nfs readdir service
/// - mallocs what it thinks is enough to read count rounded up to a multiple of
///   `NFS_DIRBLKSIZ <= NFS_MAXREADDIR`
/// - calls `VOP_READDIR()`
/// - loops around building the reply; if the output generated exceeds count break out of
///   loop
/// - it only knows that it has encountered eof when the `VOP_READDIR()` reads nothing
/// - as such one readdir rpc will return eof false although you are there and then the
///   next will return eof
/// - it trims out records with `d_fileno == 0`; this doesn't matter for Unix clients, but
///   they might confuse clients for other os'.
///
/// NB: It is tempting to set eof to true if the `VOP_READDIR()` reads less than requested,
/// but this may not apply to all filesystems. For example, client NFS does not { although
/// it is never remote mounted anyhow }. The alternate call `nfsrv_readdirplus()` does
/// lookups as well.
///
/// PS: The NFS protocol spec. does not clarify what the "count" byte argument is a count
/// of.. just name strings and file id's or the entire reply rpc or ... I tried just file
/// name and id sizes and it confused the Sun client, so I am using the full rpc size now.
/// The "paranoia.." comment refers to including the status longwords that are not a part
/// of the dir. "entry" structures, but are in the rpc.
pub fn nfsrv_readdir<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    // The cookie verifier (`verf`) is not checked.
    let (toff, cnt) = if v3 {
        let toff = dissect!(nd, r.hyper());
        dissect!(nd, r.adv(2 * NFSX_UNSIGNED));
        (toff, dissect!(nd, r.i32()))
    } else {
        let toff = u64::from(dissect!(nd, r.u32()));
        (toff, dissect!(nd, r.i32()))
    };
    let mut off = toff;
    let r0 = if cnt == 0 {
        Err(if v3 {
            NFSERR_TOOSMALL
        } else {
            Errno::EBADRPC.as_i32()
        })
    } else {
        let xfer = nfs_srvmaxdata(nd) as i32;
        let cnt = nfsrv_readdir_cnt(cnt, xfer);
        let siz = nfsrv_readdir_siz(cnt, xfer);
        fhtovp(nd, &fh, true, slp).and_then(|(vp, rdonly)| {
            if vp.vtype() != Vtype::VDIR {
                vput(vp);
                Err(Errno::ENOTDIR.as_i32())
            } else {
                Ok((vp, rdonly, cnt, siz))
            }
        })
    };
    let (vp, rdonly, cnt, fullsiz) = match r0 {
        Ok(x) => x,
        Err(error) => return reply_postop(nd, slp, error, NFSX_UNSIGNED, None),
    };
    let mut at = Vattr::new();
    let mut getret = false;
    let mut error = 0;
    if v3 {
        let g = vop_getattr(vp, &mut at, cred(nd), p);
        getret = g.is_ok();
        error = status_of(&g);
    }
    if error == 0 {
        error = status_of(&nfsrv_access(vp, VEXEC, &nd.nd_cr, rdonly, p, false));
    }
    if error != 0 {
        vput(vp);
        let after = getret.then_some(&at);
        return reply_postop(nd, slp, error, nfsx_postopattr(v3), after);
    }
    let _ = vop_unlock(vp);
    let mut rbuf = temp_bytes(fullsiz);
    // again:
    let (cpos, siz, eofflag) = loop {
        let (rr, siz, noff, eofflag, g) =
            nfsrv_readdir_pass(vp, &mut rbuf, off, cred(nd), p, v3.then_some(&mut at));
        off = noff;
        let mut error = status_of(&rr);
        if let Some(g) = g {
            getret = g.is_ok();
            if error == 0 {
                error = status_of(&g);
            }
        }
        if error != 0 {
            vrele(vp);
            drop(rbuf);
            let after = getret.then_some(&at);
            return reply_postop(nd, slp, error, nfsx_postopattr(v3), after);
        }
        // If nothing read, return eof rpc reply
        if siz == 0 {
            vrele(vp);
            drop(rbuf);
            let siz = nfsx_postopattr(v3) + nfsx_cookieverf(v3) + 2 * NFSX_UNSIGNED;
            let mut mb = match nfsm_reply(nd, slp, 0, siz) {
                Ok(mb) => mb,
                Err(done) => return done,
            };
            if v3 {
                nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);
                let tl = mb.build::<{ 4 * NFSX_UNSIGNED }>();
                tl[..2 * NFSX_UNSIGNED].copy_from_slice(&at.va_filerev.to_be_bytes());
                set_word(tl, 2, nfs_false);
                set_word(tl, 3, nfs_true);
            } else {
                let tl = mb.build::<{ 2 * NFSX_UNSIGNED }>();
                set_word(tl, 0, nfs_false);
                set_word(tl, 1, nfs_true);
            }
            return finish(mb);
        }

        // Check for degenerate cases of nothing useful read. If so go try again
        let cpos = nfsrv_dirent_skip(&rbuf[..siz]);
        if cpos >= siz {
            continue;
        }
        break (cpos, siz, eofflag);
    };

    let len = (3 * NFSX_UNSIGNED) as i32; // paranoia, probably can be 0
    let siz_hint = nfsx_postopattr(v3) + nfsx_cookieverf(v3) + siz;
    let mut mb = match nfsm_reply(nd, slp, 0, siz_hint) {
        Ok(mb) => mb,
        Err(done) => {
            vrele(vp);
            return done;
        }
    };
    if v3 {
        nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);
        mb.hyper(at.va_filerev);
    }

    // Loop through the records and build reply
    let all = nfsrv_readdir_pack(&mut mb, &rbuf[cpos..siz], v3, cnt, len);
    let eofflag = if all { eofflag } else { 0 };
    vrele(vp);
    let tl = mb.build::<{ 2 * NFSX_UNSIGNED }>();
    set_word(tl, 0, nfs_false);
    set_word(tl, 1, if eofflag != 0 { nfs_true } else { nfs_false });
    drop(rbuf);
    finish(mb)
}

/// `struct flrep`'s size: `fl_off` (2 words), `fl_postopok`, `fl_fattr`, `fl_fhok`,
/// `fl_fhsize`, `fl_nfh`.
const FLREP_SIZE: usize =
    2 * NFSX_UNSIGNED + NFSX_UNSIGNED + NFSX_V3FATTR + 2 * NFSX_UNSIGNED + NFSX_V3FH;

/// `struct flrep`: the part of a READDIRPLUS `entryplus3` after the name (the cookie, the
/// `post_op_attr` and the `post_op_fh3`), as the bytes `nfsm_buftombuf` copies out.
fn nfsrv_flrep(off: u64, fattr: &NfsFattr, nfh: &Nfsfh) -> [u8; FLREP_SIZE] {
    let mut fl = [0u8; FLREP_SIZE];
    let off = Nfsuint64::from_words(txdr_hyper(off));
    fl[..2 * NFSX_UNSIGNED].copy_from_slice(xdr_bytes(&off));
    set_word(&mut fl, 2, nfs_true);
    let fa = 3 * NFSX_UNSIGNED;
    fl[fa..fa + NFSX_V3FATTR].copy_from_slice(&xdr_bytes(fattr)[..NFSX_V3FATTR]);
    let fhok = (fa + NFSX_V3FATTR) / NFSX_UNSIGNED;
    set_word(&mut fl, fhok, nfs_true);
    set_word(&mut fl, fhok + 1, txdr_unsigned(NFSX_V3FH as u32));
    fl[(fhok + 2) * NFSX_UNSIGNED..].copy_from_slice(&nfh.fh_bytes[..NFSX_V3FH]);
    fl
}

/// nfs readdirplus service: `nfsrv_readdir` with the attributes and file handle of each
/// entry (version 3 only).
pub fn nfsrv_readdirplus<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    // The cookie verifier (`verf`) is not checked.
    let toff = dissect!(nd, r.hyper());
    dissect!(nd, r.adv(2 * NFSX_UNSIGNED));
    let siz = dissect!(nd, r.i32());
    let cnt = dissect!(nd, r.i32());
    let mut off = toff;
    let r0 = if siz == 0 || cnt == 0 {
        Err(NFSERR_TOOSMALL)
    } else {
        let xfer = nfs_srvmaxdata(nd) as i32;
        let cnt = nfsrv_readdir_cnt(cnt, xfer);
        let siz = nfsrv_readdir_siz(siz, xfer);
        fhtovp(nd, &fh, true, slp).and_then(|(vp, rdonly)| {
            if vp.vtype() != Vtype::VDIR {
                vput(vp);
                Err(Errno::ENOTDIR.as_i32())
            } else {
                Ok((vp, rdonly, cnt, siz))
            }
        })
    };
    let (vp, rdonly, cnt, fullsiz) = match r0 {
        Ok(x) => x,
        Err(error) => return reply_postop(nd, slp, error, NFSX_UNSIGNED, None),
    };
    let mut at = Vattr::new();
    let g = vop_getattr(vp, &mut at, cred(nd), p);
    let mut getret = g.is_ok();
    let mut error = status_of(&g);
    if error == 0 {
        error = status_of(&nfsrv_access(vp, VEXEC, &nd.nd_cr, rdonly, p, false));
    }
    if error != 0 {
        vput(vp);
        return reply_postop(nd, slp, error, NFSX_V3POSTOPATTR, getret.then_some(&at));
    }
    let _ = vop_unlock(vp);

    let mut rbuf = temp_bytes(fullsiz);
    // again:
    let (cpos, siz, eofflag) = loop {
        let (rr, siz, noff, eofflag, g) =
            nfsrv_readdir_pass(vp, &mut rbuf, off, cred(nd), p, Some(&mut at));
        off = noff;
        let g = g.unwrap_or(Ok(()));
        getret = g.is_ok();
        let error = if rr.is_ok() {
            status_of(&g)
        } else {
            status_of(&rr)
        };
        if error != 0 {
            vrele(vp);
            drop(rbuf);
            return reply_postop(nd, slp, error, NFSX_V3POSTOPATTR, getret.then_some(&at));
        }
        // If nothing read, return eof rpc reply
        if siz == 0 {
            vrele(vp);
            drop(rbuf);
            let siz = NFSX_V3POSTOPATTR + NFSX_V3COOKIEVERF + 2 * NFSX_UNSIGNED;
            let mut mb = match nfsm_reply(nd, slp, 0, siz) {
                Ok(mb) => mb,
                Err(done) => return done,
            };
            nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);
            let tl = mb.build::<{ 4 * NFSX_UNSIGNED }>();
            tl[..2 * NFSX_UNSIGNED].copy_from_slice(&at.va_filerev.to_be_bytes());
            set_word(tl, 2, nfs_false);
            set_word(tl, 3, nfs_true);
            return finish(mb);
        }

        // Check for degenerate cases of nothing useful read. If so go try again
        let cpos = nfsrv_dirent_skip(&rbuf[..siz]);
        if cpos >= siz {
            continue;
        }
        break (cpos, siz, eofflag);
    };

    // struct READDIRPLUS3resok {
    //     postop_attr dir_attributes;
    //     cookieverf3 cookieverf;
    //     dirlistplus3 reply;
    // }
    //
    // struct dirlistplus3 {
    //     entryplus3  *entries;
    //     bool eof;
    //  }
    let mut len = NFSX_V3POSTOPATTR + NFSX_V3COOKIEVERF + 2 * NFSX_UNSIGNED;
    let mut dirlen = len;
    let mut mb = match nfsm_reply(nd, slp, 0, cnt as usize) {
        Ok(mb) => mb,
        Err(done) => {
            vrele(vp);
            return done;
        }
    };
    nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);
    mb.hyper(at.va_filerev);

    // Loop through the records and build reply
    let mut eofflag = eofflag;
    let buf = &rbuf[cpos..siz];
    let mut cpos = 0;
    let mut va = Vattr::new();
    let mp = v_mount(vp);
    'entries: while cpos < buf.len() {
        let Some((dp, name)) = nfsrv_dirent(&buf[cpos..]) else {
            break;
        };
        'invalid: {
            if dp.d_fileno == 0 {
                break 'invalid;
            }
            let nlen = name.len();
            let pad = nfsm_padlen(nlen);

            // For readdir_and_lookup get the vnode using the file number.
            let Ok(nvp) = VFS_VGET(mp, dp.d_fileno) else {
                break 'invalid;
            };
            let Ok(nfhp) = nfsrv_vptofh(nvp) else {
                vput(nvp);
                break 'invalid;
            };
            if vop_getattr(nvp, &mut va, cred(nd), p).is_err() {
                vput(nvp);
                break 'invalid;
            }
            vput(nvp);

            // If either the dircount or maxcount will be exceeded, get out now. Both of
            // these lengths are calculated conservatively, including all XDR overheads.
            //
            // Each entry:
            // 2 * NFSX_UNSIGNED for fileid3
            // 1 * NFSX_UNSIGNED for length of name
            // nlen + pad == space the name takes up
            // 2 * NFSX_UNSIGNED for the cookie
            // 1 * NFSX_UNSIGNED to indicate if file handle present
            // 1 * NFSX_UNSIGNED for the file handle length
            // NFSX_V3FH == space our file handle takes up
            // NFSX_V3POSTOPATTR == space the attributes take up
            // 1 * NFSX_UNSIGNED for next pointer
            len += 8 * NFSX_UNSIGNED + nlen + pad + NFSX_V3FH + NFSX_V3POSTOPATTR;
            dirlen += 6 * NFSX_UNSIGNED + nlen + pad;
            if len > cnt as usize || dirlen > fullsiz {
                eofflag = 0;
                break 'entries;
            }

            let tl = mb.build::<{ 3 * NFSX_UNSIGNED }>();
            set_word(tl, 0, nfs_true);
            tl[NFSX_UNSIGNED..].copy_from_slice(&dp.d_fileno.to_be_bytes());

            // And copy the name
            mb.strtombuf(name);

            // Build the directory record xdr from the dirent entry.
            let fp = nfsm_srvfattr(nd, &va);
            let fl = nfsrv_flrep(dp.d_off as u64, &fp, &nfhp);

            // Now copy the flrep structure out.
            mb.buftombuf(&fl);
        }
        // invalid:
        cpos += usize::from(dp.d_reclen);
    }
    vrele(vp);
    let tl = mb.build::<{ 2 * NFSX_UNSIGNED }>();
    set_word(tl, 0, nfs_false);
    set_word(tl, 1, if eofflag != 0 { nfs_true } else { nfs_false });
    drop(rbuf);
    finish(mb)
}

/// nfs commit service
pub fn nfsrv_commit<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let (mut r, fh) = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((r, Some(fh))) => (r, fh),
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    // XXX At this time VOP_FSYNC() does not accept offset and byte count parameters, so
    // these arguments are useless (someday maybe).
    let _off = dissect!(nd, r.hyper());
    let _cnt = dissect!(nd, r.i32()).max(0);
    let (vp, _) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return reply_wcc(nd, slp, error, 2 * NFSX_UNSIGNED, None, None),
    };
    let mut bfor = Vattr::new();
    let for_ret = vop_getattr(vp, &mut bfor, cred(nd), p).is_ok();
    let error = status_of(&vop_fsync(vp, cred(nd), MNT_WAIT, p));
    let mut aft = Vattr::new();
    let aft_ret = vop_getattr(vp, &mut aft, cred(nd), p).is_ok();
    vput(vp);
    let mut mb = match nfsm_reply(nd, slp, error, NFSX_V3WCCDATA + NFSX_V3WRITEVERF) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    let (before, after) = (for_ret.then_some(&bfor), aft_ret.then_some(&aft));
    nfsm_srvwcc(nd, before, after, &mut mb);
    if error == 0 {
        let tl = mb.build::<NFSX_V3WRITEVERF>();
        let boottime = microboottime();
        set_word(tl, 0, txdr_unsigned(boottime.tv_sec as u32));
        set_word(tl, 1, txdr_unsigned(boottime.tv_usec as u32));
    }
    finish(mb)
}

/// nfs statfs service
pub fn nfsrv_statfs<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let fh = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((_, Some(fh))) => fh,
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (vp, _) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return reply_postop(nd, slp, error, NFSX_UNSIGNED, None),
    };
    let mut sf = Statfs::default();
    let error = status_of(&VFS_STATFS(v_mount(vp), &mut sf, p));
    let mut at = Vattr::new();
    let getret = vop_getattr(vp, &mut at, cred(nd), p).is_ok();
    vput(vp);
    let mut mb = match nfsm_reply(nd, slp, error, nfsx_postopattr(v3) + nfsx_statfs(v3)) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    if v3 {
        nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);
    }
    if error != 0 {
        return finish(mb);
    }
    let mut sfp = NfsStatfs::default();
    if v3 {
        let bsize = u64::from(sf.f_bsize);
        let hyper = |v: u64| Nfsuint64::from_words(txdr_hyper(v));
        sfp.set_sf_tbytes(hyper(sf.f_blocks.wrapping_mul(bsize)));
        sfp.set_sf_fbytes(hyper(sf.f_bfree.wrapping_mul(bsize)));
        sfp.set_sf_abytes(hyper((sf.f_bavail as u64).wrapping_mul(bsize)));
        sfp.set_sf_tfiles(hyper(sf.f_files));
        let tval = sf.f_ffree;
        sfp.set_sf_ffiles(hyper(tval));
        sfp.set_sf_afiles(hyper(tval));
        sfp.set_sf_invarsec(0);
    } else {
        sfp.set_sf_tsize(txdr_unsigned(NFS_MAXDGRAMDATA as u32));
        sfp.set_sf_bsize(txdr_unsigned(sf.f_bsize));
        sfp.set_sf_blocks(txdr_unsigned(sf.f_blocks as u32));
        sfp.set_sf_bfree(txdr_unsigned(sf.f_bfree as u32));
        sfp.set_sf_bavail(txdr_unsigned(sf.f_bavail as u32));
    }
    let n = nfsx_statfs(v3);
    mb.build_slice(n).copy_from_slice(&xdr_bytes(&sfp)[..n]);
    finish(mb)
}

/// nfs fsinfo service
pub fn nfsrv_fsinfo<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let fh = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((_, Some(fh))) => fh,
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (vp, _) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return reply_postop(nd, slp, error, NFSX_UNSIGNED, None),
    };
    let mut at = Vattr::new();
    let getret = vop_getattr(vp, &mut at, cred(nd), p).is_ok();
    vput(vp);
    let mut mb = match nfsm_reply(nd, slp, 0, NFSX_V3POSTOPATTR + NFSX_V3FSINFO) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);

    // XXX
    // There should be file system VFS OP(s) to get this information. For now, assume ufs.
    let pref = if slp.sotype() == SOCK_DGRAM {
        NFS_MAXDGRAMDATA
    } else {
        NFS_MAXDATA
    } as u32;
    let sip = Nfsv3Fsinfo {
        fs_rtmax: txdr_unsigned(NFS_MAXDATA as u32),
        fs_rtpref: txdr_unsigned(pref),
        fs_rtmult: txdr_unsigned(NFS_FABLKSIZE as u32),
        fs_wtmax: txdr_unsigned(NFS_MAXDATA as u32),
        fs_wtpref: txdr_unsigned(pref),
        fs_wtmult: txdr_unsigned(NFS_FABLKSIZE as u32),
        fs_dtpref: txdr_unsigned(pref),
        fs_maxfilesize: Nfsuint64::from_words([0xffff_ffff, 0xffff_ffff]),
        fs_timedelta: Nfsv3Time {
            nfsv3_sec: 0,
            nfsv3_nsec: txdr_unsigned(1),
        },
        fs_properties: txdr_unsigned(
            NFSV3FSINFO_LINK
                | NFSV3FSINFO_SYMLINK
                | NFSV3FSINFO_HOMOGENEOUS
                | NFSV3FSINFO_CANSETTIME,
        ),
    };
    mb.build_slice(NFSX_V3FSINFO)
        .copy_from_slice(&xdr_bytes(&sip)[..NFSX_V3FSINFO]);
    finish(mb)
}

/// nfs pathconf service
pub fn nfsrv_pathconf<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    p: &Thread<G>,
) -> Answer {
    let v3 = nd.nd_flag & ND_NFSV3 != 0;
    let fh = match srv_args(&nd.nd_mrep, nd.nd_off, v3) {
        Ok((_, Some(fh))) => fh,
        Ok((_, None)) => return nfsm_reply_only(nd, slp, Errno::EBADRPC.as_i32(), 0),
        Err(e) => return bad_request(nd, e),
    };
    let (vp, _) = match fhtovp(nd, &fh, true, slp) {
        Ok(x) => x,
        Err(error) => return reply_postop(nd, slp, error, NFSX_UNSIGNED, None),
    };
    let mut linkmax: Register = 0;
    let mut namemax: Register = 0;
    let mut chownres: Register = 0;
    let mut notrunc: Register = 0;
    let rr = vop_pathconf(vp, _PC_LINK_MAX, &mut linkmax)
        .and_then(|()| vop_pathconf(vp, _PC_NAME_MAX, &mut namemax))
        .and_then(|()| vop_pathconf(vp, _PC_CHOWN_RESTRICTED, &mut chownres))
        .and_then(|()| vop_pathconf(vp, _PC_NO_TRUNC, &mut notrunc));
    let error = status_of(&rr);
    let mut at = Vattr::new();
    let getret = vop_getattr(vp, &mut at, cred(nd), p).is_ok();
    vput(vp);
    let mut mb = match nfsm_reply(nd, slp, error, NFSX_V3POSTOPATTR + NFSX_V3PATHCONF) {
        Ok(mb) => mb,
        Err(done) => return done,
    };
    nfsm_srvpostop_attr(nd, getret.then_some(&at), &mut mb);
    if error != 0 {
        return finish(mb);
    }
    let pc = Nfsv3Pathconf {
        pc_linkmax: txdr_unsigned(linkmax as u32),
        pc_namemax: txdr_unsigned(namemax as u32),
        pc_notrunc: txdr_unsigned(notrunc as u32),
        pc_chownrestricted: txdr_unsigned(chownres as u32),
        // These should probably be supported by VOP_PATHCONF(), but until msdosfs is
        // exportable (why would you want to?), the Unix defaults should be ok.
        pc_caseinsensitive: nfs_false,
        pc_casepreserving: nfs_true,
    };
    mb.build_slice(NFSX_V3PATHCONF)
        .copy_from_slice(&xdr_bytes(&pc)[..NFSX_V3PATHCONF]);
    finish(mb)
}

/// Null operation, used by clients to ping server
pub fn nfsrv_null<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    _p: &Thread<G>,
) -> Answer {
    nfsm_reply_only(nd, slp, NFSERR_RETVOID, 0)
}

/// No operation, used for obsolete procedures
pub fn nfsrv_noop<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
    _p: &Thread<G>,
) -> Answer {
    let error = if nd.nd_repstat != 0 {
        nd.nd_repstat
    } else {
        Errno::EPROCUNAVAIL.as_i32()
    };
    nfsm_reply_only(nd, slp, error, 0)
}

/// Perform access checking for vnodes obtained from file handles that would refer to files
/// already opened by a Unix client. You cannot just use `vn_writechk()` and `VOP_ACCESS()`
/// for two reasons:
/// 1. You must check for exported rdonly as well as `MNT_RDONLY` for the write case.
/// 2. The owner is to be given access irrespective of mode bits for some operations
///    (`override_`), so that processes that chmod after opening a file don't break. I don't
///    like this because it opens a security hole, but since the nfs server opens a security
///    hole the size of a barn door anyhow, what the heck. A notable exception to this rule
///    is when `VOP_ACCESS()` returns `EPERM` (e.g. when a file is immutable) which is always
///    an error.
pub fn nfsrv_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    flags: i32,
    cred: &Ucred,
    rdonly: bool,
    p: &Thread<G>,
    override_: bool,
) -> Result<(), Errno> {
    if flags & VWRITE != 0 {
        // Just vn_writechk() changed to check rdonly
        //
        // Disallow write attempts on read-only file systems; unless the file is a socket or
        // a block or character device resident on the file system.
        if (rdonly || mnt_rdonly(vp))
            && matches!(vp.vtype(), Vtype::VREG | Vtype::VDIR | Vtype::VLNK)
        {
            return Err(Errno::EROFS);
        }
        // If there's shared text associated with the inode, try to free it up once. If we
        // fail, we can't allow writing.
        if vp.flag() & VTEXT != 0 && !uvm_vnp_uncache(vp) {
            return Err(Errno::ETXTBSY);
        }
    }
    let r = vop_access(vp, flags, Cred::real(cred), p);
    // Allow certain operations for the owner (reads and writes on files that are already
    // open).
    if override_ && r == Err(Errno::EACCES) {
        let mut vattr = Vattr::new();
        if vop_getattr(vp, &mut vattr, Cred::real(cred), p).is_ok()
            && cred.cr_uid.get() == vattr.va_uid
        {
            return Ok(());
        }
    }
    r
}
/// `nfsrv3_procs[]` (`nfs_syscalls.c`): the server procedures of the glue `G`, indexed by
/// `nd_procnum` (the version 3 numbers; `nfs_getreq` maps a version 2 request's onto them).
pub fn nfsrv3_procs<G: ProcGlue>() -> [NfsrvProc<G>; NFS_NPROCS] {
    [
        nfsrv_null,
        nfsrv_getattr,
        nfsrv_setattr,
        nfsrv_lookup,
        nfsrv3_access,
        nfsrv_readlink,
        nfsrv_read,
        nfsrv_write,
        nfsrv_create,
        nfsrv_mkdir,
        nfsrv_symlink,
        nfsrv_mknod,
        nfsrv_remove,
        nfsrv_rmdir,
        nfsrv_rename,
        nfsrv_link,
        nfsrv_readdir,
        nfsrv_readdirplus,
        nfsrv_statfs,
        nfsrv_fsinfo,
        nfsrv_pathconf,
        nfsrv_commit,
        nfsrv_noop,
    ]
}
/* </CODE> */
