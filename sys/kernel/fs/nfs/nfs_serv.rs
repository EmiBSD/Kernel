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
//! - The C's second `pool_put` of the name buffer after a failed `VOP_MKNOD` (a double free;
//!   LZ's note) is closed: the buffer is owned, and given back once.
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
//! - LZ slip fixed: `nfsrv_symlink` hands `VOP_SYMLINK` the target up to its first NUL, the
//!   C's `strlen` of its NUL-terminated copy (LZ passed all `len2` bytes).
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
//! - The name lookups take the request's credentials for both their parts: `nfs_namei` is
//!   given `&nd.nd_cr`, which its `nfsrv_fhtovp` maps and which it makes the lookup's
//!   `cn_cred` (the C's caller's `ndp->ni_cnd.cn_cred = cred`), so the procedures set no
//!   `cn_cred` of their own, nor before the second lookups of create and symlink, which keep
//!   it; `nfsrv_mknod`'s second lookup reads the nfsd's (`procp->p_ucred`), as the C's.
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
        let mut ni = ndinit(LOOKUP, LOCKLEAF | SAVESTART, NiDirp::Sys(&[]), p);
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

    let mut ni = ndinit(
        CREATE,
        LOCKPARENT | LOCKLEAF | SAVESTART,
        NiDirp::Sys(&[]),
        p,
    );
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
            // cn_cred: the request's still, as nfs_namei set it.
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

    let mut ni = ndinit(
        CREATE,
        LOCKPARENT | LOCKLEAF | SAVESTART,
        NiDirp::Sys(&[]),
        p,
    );
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

    let mut ni = ndinit(DELETE, LOCKPARENT | LOCKLEAF, NiDirp::Sys(&[]), p);
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

    let mut fromnd = ndinit(DELETE, WANTPARENT | SAVESTART, NiDirp::Sys(&[]), p);
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

    let mut tond = ndinit(
        RENAME,
        LOCKPARENT | LOCKLEAF | NOCACHE | SAVESTART,
        NiDirp::Sys(&[]),
        p,
    );
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
    let mut ni = ndinit(CREATE, LOCKPARENT, NiDirp::Sys(&[]), p);
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

    let mut ni = ndinit(CREATE, LOCKPARENT | SAVESTART, NiDirp::Sys(&[]), p);
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
                // cn_cred: the request's still, as nfs_namei set it.
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

    let mut ni = ndinit(CREATE, LOCKPARENT, NiDirp::Sys(&[]), p);
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

    let mut ni = ndinit(DELETE, LOCKPARENT | LOCKLEAF, NiDirp::Sys(&[]), p);
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

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the server's procedures over an exported ffs (the kernel's testffs image,
    // mounted at the root, its default export), driven by hand-built requests whose replies
    // are read back word by word: LZ's five re-expressed (the readdir packing and sizes,
    // flrep, the null/no-op/bad requests, every procedure in both versions with a clean
    // unmount at the end, which a leaked vnode would refuse), then the C's checks one at a
    // time (a read-only and a mapall export, the owner override, the name, count and write
    // bounds, the stable modes, rename's rules, an unreserved port), and random requests to
    // every procedure (a reply or a drop, never a panic, the unmount still clean).

    use core::mem::offset_of;
    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use hal::sys::mount::{
        ExportArgs, MNT_DELEXPORT, MNT_EXPORTANON, MNT_EXPORTED, MNT_EXRDONLY, Xucred,
    };
    use hal::sys::syslimits::NGROUPS_MAX as XNGROUPS;

    use super::*;
    use crate::fs::export::vfs_export;
    use crate::fs::nfs::nfsm_subs::{test_bytes, test_chain};
    use crate::fs::nfs::nfsproto::{
        NFSERR_ACCES, NFSERR_EXIST, NFSERR_INVAL, NFSERR_IO, NFSERR_ISDIR, NFSERR_NOENT,
        NFSERR_NOTDIR, NFSERR_NOTEMPTY, NFSERR_NXIO, NFSERR_ROFS, NFSERR_STALE, NFSPROC_ACCESS,
        NFSPROC_COMMIT, NFSPROC_CREATE, NFSPROC_FSINFO, NFSPROC_FSSTAT, NFSPROC_GETATTR,
        NFSPROC_LINK, NFSPROC_LOOKUP, NFSPROC_MKDIR, NFSPROC_MKNOD, NFSPROC_NOOP, NFSPROC_NULL,
        NFSPROC_PATHCONF, NFSPROC_READ, NFSPROC_READDIR, NFSPROC_READDIRPLUS, NFSPROC_READLINK,
        NFSPROC_REMOVE, NFSPROC_RENAME, NFSPROC_RMDIR, NFSPROC_SETATTR, NFSPROC_SYMLINK,
        NFSPROC_WRITE, NFSV3CREATE_EXCLUSIVE, NFSV3SATTRTIME_DONTCHANGE, NFSV3WRITE_DATASYNC,
        Nfstype,
    };
    use crate::fs::nfs::rpcv2::{AUTH_TOOWEAK, RPC_GARBAGE, RPC_PROCUNAVAIL};
    use crate::fs::nfs::xdr_subs::xdr_get;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs::{self, newfs};
    use crate::fs::vops::VFS_ROOT;
    use crate::net::mbuf::{m_devget, mbinit, nmbclust_update};
    use crate::sys::socket::{AF_INET, SOCK_STREAM};

    type G = TestGlue;

    /// The raw XDR words of `b` as host values.
    fn words(b: &[u8]) -> Vec<u32> {
        b.chunks(4)
            .map(|c| {
                let mut w = [0u8; 4];
                w[..c.len()].copy_from_slice(c);
                u32::from_be_bytes(w)
            })
            .collect()
    }

    /// An XDR request body.
    #[derive(Default, Clone)]
    struct Req(Vec<u8>);

    impl Req {
        fn u(mut self, v: u32) -> Self {
            self.0.extend_from_slice(&v.to_be_bytes());
            self
        }

        fn hyper(self, v: u64) -> Self {
            self.u((v >> 32) as u32).u(v as u32)
        }

        fn raw(mut self, b: &[u8]) -> Self {
            self.0.extend_from_slice(b);
            self
        }

        /// A file handle: version 3 `nfs_fh3` (length and bytes), version 2 32 bytes.
        fn fh(self, fh: &Fhandle, v3: bool) -> Self {
            let b = xdr_bytes(fh).to_vec();
            if v3 {
                self.u(NFSX_V3FH as u32).raw(&b)
            } else {
                self.raw(&b).raw(&[0; NFSX_V2FH - NFSX_V3FH])
            }
        }

        /// A string or opaque: length, bytes and padding.
        fn name(self, s: &[u8]) -> Self {
            let pad = nfsm_padlen(s.len());
            self.u(s.len() as u32).raw(s).raw(&[0; 3][..pad])
        }

        /// A version 3 `sattr3` that sets the mode and the size it is given.
        fn sattr3(self, mode: Option<u32>, size: Option<u64>) -> Self {
            let s = match mode {
                Some(m) => self.u(1).u(m),
                None => self.u(0),
            };
            let s = s.u(0).u(0);
            let s = match size {
                Some(z) => s.u(1).hyper(z),
                None => s.u(0),
            };
            s.u(NFSV3SATTRTIME_DONTCHANGE).u(NFSV3SATTRTIME_DONTCHANGE)
        }

        /// A version 2 `sattr` that sets only the mode.
        fn sattr2(self, mode: u32) -> Self {
            let mut s = self.u(mode);
            for _ in 0..7 {
                s = s.u(u32::MAX);
            }
            s
        }
    }

    /// A reply being read back.
    struct Rd {
        w: Vec<u32>,
        i: usize,
    }

    impl Rd {
        /// An accepted RPC reply whose `accept_stat` is `stat`.
        fn accepted(w: Vec<u32>, stat: u32) -> Rd {
            assert!(w.len() >= 6, "short reply {w:x?}");
            assert_eq!(&w[..6], &[0x1234, 1, 0, 0, 0, stat], "RPC reply header");
            Rd { w, i: 6 }
        }

        /// An accepted reply whose NFS status is `status`.
        fn status(w: Vec<u32>, status: i32) -> Rd {
            let mut r = Rd::accepted(w, 0);
            assert_eq!(r.u(), status as u32, "NFS status");
            r
        }

        fn u(&mut self) -> u32 {
            let v = self.w[self.i];
            self.i += 1;
            v
        }

        fn hyper(&mut self) -> u64 {
            (u64::from(self.u()) << 32) | u64::from(self.u())
        }

        fn n(&mut self, n: usize) -> Vec<u32> {
            let v = self.w[self.i..self.i + n].to_vec();
            self.i += n;
            v
        }

        /// An opaque or string: its bytes.
        fn opaque(&mut self) -> Vec<u8> {
            let len = self.u() as usize;
            let ws = self.n(len.div_ceil(4));
            let mut b: Vec<u8> = ws.iter().flat_map(|w| w.to_be_bytes()).collect();
            b.truncate(len);
            b
        }

        /// A `post_op_attr`: the 21 words of `fattr3`, when present.
        fn postop(&mut self) -> Option<Vec<u32>> {
            (self.u() == 1).then(|| self.n(21))
        }

        /// A `wcc_data`: the pre-operation attributes (6 words) and the post-operation ones.
        fn wcc(&mut self) -> (Option<Vec<u32>>, Option<Vec<u32>>) {
            let pre = (self.u() == 1).then(|| self.n(6));
            (pre, self.postop())
        }

        /// A `post_op_fh3`: the handle, when present.
        fn postop_fh(&mut self) -> Option<Fhandle> {
            (self.u() == 1).then(|| {
                let b = self.opaque();
                assert_eq!(b.len(), NFSX_V3FH);
                xdr_get::<Fhandle>(&b)
            })
        }

        fn done(&self) {
            assert_eq!(self.i, self.w.len(), "the whole reply was read");
        }
    }

    /// The exported file system and the server's view of it.
    struct Srv {
        p: &'static Thread<G>,
        mp: &'static Mount<G>,
        root: Fhandle,
        slp: NfssvcSock<G>,
        /// The client's port (0: no client address).
        port: u16,
    }

    /// `struct export_args` of the default export: `flags`, the anonymous ids `anon`.
    fn export(mp: &'static Mount<G>, flags: i32, anon: u32) {
        let a = ExportArgs {
            ex_flags: flags,
            ex_root: 0,
            ex_anon: Xucred {
                cr_uid: anon,
                cr_gid: anon,
                cr_ngroups: 0,
                _pad0: 0,
                cr_groups: [0; XNGROUPS],
            },
            _pad0: 0,
            ex_addr: 0,
            ex_addrlen: 0,
            _pad1: 0,
            ex_mask: 0,
            ex_masklen: 0,
            _pad2: 0,
        };
        vfs_export(mp, mp.export(), &a).expect("exported");
    }

    /// An ffs image with `/motd` (13 bytes, mode 0644), mounted at the root, its default export
    /// `flags` (`MNT_EXPORTED` and more) mapping to `anon`, a datagram socket and a client on a
    /// reserved port; the kernel lock held, as the nfsds hold it.
    fn served(flags: i32, anon: u32) -> (MutexGuard<'static, ()>, Srv) {
        let mut img = newfs::Image::new(newfs::FFS1_4M);
        img.add_file(b"motd", b"hello, world\n");
        let (g, p) = testffs::attach(img.finish());
        crate::testutil::kernel_maps_install();
        mbinit(&crate::net::mbuf::tests::TEST_NMBCLUST);
        let _ = nmbclust_update(256 * 1024);
        hal::klock::kernel_lock();
        let mp = testffs::mount_root(p, 0);
        export(mp, flags, anon);
        let vp = VFS_ROOT(mp).expect("the root");
        let root = nfsrv_vptofh(vp).expect("its handle").fh_generic();
        vput(vp);
        let slp = NfssvcSock::new(None);
        slp.ns_sotype.store(
            crate::sys::socket::SOCK_DGRAM,
            core::sync::atomic::Ordering::Relaxed,
        );
        let s = Srv {
            p,
            mp,
            root,
            slp,
            port: 800,
        };
        (g, s)
    }

    /// A client's name at 127.0.0.1:`port`.
    fn client(port: u16) -> MbufBox {
        let mut sin = [0u8; 16];
        sin[0] = 16;
        sin[1] = AF_INET;
        sin[2..4].copy_from_slice(&port.to_be_bytes());
        sin[4..8].copy_from_slice(&[127, 0, 0, 1]);
        m_devget(&sin, 0).expect("an address mbuf")
    }

    /// A request descriptor for `body`, split over mbufs of 50 bytes (so that words straddle
    /// them), from 127.0.0.1:`port` (none for 0).
    fn descript(body: &[u8], v3: bool, port: u16) -> NfsrvDescript {
        let parts: Vec<&[u8]> = if body.is_empty() {
            vec![&[][..]]
        } else {
            body.chunks(50).collect()
        };
        let mut nd = NfsrvDescript {
            nd_mrep: Some(test_chain(&parts)),
            nd_retxid: 0x1234,
            ..NfsrvDescript::default()
        };
        if v3 {
            nd.nd_flag |= ND_NFSV3;
        }
        if port != 0 {
            nd.nd_nam2 = Some(client(port));
        }
        nd
    }

    /// The procedure `procnum` of the table on the request `body`: the C's `int` and the
    /// reply's words; the request was freed either way.
    fn run(s: &Srv, procnum: usize, body: &Req, v3: bool) -> (i32, Option<Vec<u32>>) {
        let mut nd = descript(&body.0, v3, s.port);
        nd.nd_procnum = procnum;
        let (m, e) = nfsrv3_procs::<G>()[procnum](&mut nd, &s.slp, s.p);
        assert!(nd.nd_mrep.is_none(), "the request was freed");
        assert!(m.is_some() == (e == 0), "a reply iff 0: {e}");
        (e, m.map(|m| words(&test_bytes(&m))))
    }

    /// [`run`] of a request that gets a reply.
    fn reply(s: &Srv, procnum: usize, body: &Req, v3: bool) -> Vec<u32> {
        let (e, w) = run(s, procnum, body, v3);
        assert_eq!(e, 0);
        w.expect("a reply")
    }

    /// LOOKUP of `name` under `dir` (version 3): the handle, or the status.
    fn lookup(s: &Srv, dir: &Fhandle, name: &[u8]) -> Result<Fhandle, i32> {
        let w = reply(
            s,
            NFSPROC_LOOKUP,
            &Req::default().fh(dir, true).name(name),
            true,
        );
        let mut r = Rd::accepted(w, 0);
        match r.u() {
            0 => {
                let b = r.opaque();
                assert!(r.postop().is_some());
                assert!(r.postop().is_some());
                r.done();
                Ok(xdr_get::<Fhandle>(&b))
            }
            st => Err(st as i32),
        }
    }

    /// The version 3 status of `procnum` on `body`.
    fn status3(s: &Srv, procnum: usize, body: &Req) -> i32 {
        let w = reply(s, procnum, body, true);
        Rd::accepted(w, 0).u() as i32
    }

    /// A directory entry as `VOP_READDIR` writes it.
    fn dirent(fileno: u64, off: i64, name: &[u8]) -> Vec<u8> {
        let reclen = (Dirent::NAME_OFFSET + name.len() + 1 + 7) & !7;
        let mut b = vec![0u8; reclen];
        b[offset_of!(Dirent, d_fileno)..][..8].copy_from_slice(&fileno.to_ne_bytes());
        b[offset_of!(Dirent, d_off)..][..8].copy_from_slice(&off.to_ne_bytes());
        b[offset_of!(Dirent, d_reclen)..][..2].copy_from_slice(&(reclen as u16).to_ne_bytes());
        b[offset_of!(Dirent, d_namlen)] = name.len() as u8;
        b[Dirent::NAME_OFFSET..][..name.len()].copy_from_slice(name);
        b
    }

    /// The words a writer built.
    fn written(mb: XdrWriter) -> Vec<u32> {
        words(&test_bytes(&mb.into_chain()))
    }

    fn writer() -> XdrWriter {
        let m = m_get(M_WAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(0);
        XdrWriter::with_first(m)
    }

    #[test]
    fn readdir_packs_entries_with_a_file_number_while_they_fit() {
        let _g = crate::net::mbuf::tests::setup();
        let mut buf = dirent(5, 100, b"a");
        buf.extend(dirent(0, 200, b"gone"));
        buf.extend(dirent(7, 300, b"bcdef"));

        // Version 3: true, fileid (hyper), name, cookie (hyper).
        let mut mb = writer();
        assert!(nfsrv_readdir_pack(&mut mb, &buf, true, 1000, 12));
        let a = u32::from_be_bytes(*b"a\0\0\0");
        let bcde = u32::from_be_bytes(*b"bcde");
        let f = u32::from_be_bytes(*b"f\0\0\0");
        assert_eq!(
            written(mb),
            [1, 0, 5, 1, a, 0, 100, 1, 0, 7, 5, bcde, f, 0, 300]
        );

        // Version 2: true, fileid, name, cookie; the second entry does not fit the count.
        let mut mb = writer();
        // 12 + (16 + 4) = 32 for "a", then 32 + (16 + 8) = 56 > 40.
        assert!(!nfsrv_readdir_pack(&mut mb, &buf, false, 40, 12));
        assert_eq!(written(mb), [1, 5, 1, a, 100]);

        // A truncated entry or one with no length ends the walk.
        let mut mb = writer();
        let mut bad = dirent(9, 1, b"x");
        bad[offset_of!(Dirent, d_reclen)..][..2].copy_from_slice(&0u16.to_ne_bytes());
        assert!(nfsrv_readdir_pack(&mut mb, &bad, true, 1000, 12));
        assert!(written(mb).is_empty());
        assert_eq!(nfsrv_dirent_skip(&bad), bad.len());
        let long = dirent(9, 1, b"x");
        assert!(nfsrv_dirent(&long[..long.len() - 1]).is_none());

        // The degenerate-case walk stops at the first entry with a file number.
        let mut skip = dirent(0, 1, b"x");
        let first = skip.len();
        skip.extend(dirent(3, 2, b"y"));
        assert_eq!(nfsrv_dirent_skip(&skip), first);
        let empty = dirent(0, 1, b"x");
        assert_eq!(nfsrv_dirent_skip(&empty), empty.len());
    }

    #[test]
    fn readdir_sizes_follow_the_c() {
        assert_eq!(nfsrv_readdir_cnt(100, 8192), 100);
        assert_eq!(nfsrv_readdir_cnt(-1, 8192), 8192);
        assert_eq!(nfsrv_readdir_cnt(9000, 8192), 8192);
        assert_eq!(nfsrv_readdir_siz(100, 8192), 512);
        assert_eq!(nfsrv_readdir_siz(512, 8192), 512);
        assert_eq!(nfsrv_readdir_siz(513, 8192), 1024);
        assert_eq!(nfsrv_readdir_siz(9000, 8192), 8192);
        assert_eq!(nfsrv_readdir_siz(-5, 8192), 8192, "not positive: xfer");
        assert_eq!(
            nfsrv_readdir_siz(i32::MAX, 8192),
            8192,
            "wraps negative: xfer"
        );
    }

    #[test]
    fn flrep_is_cookie_attributes_and_handle() {
        let fattr = NfsFattr {
            fa_type: txdr_unsigned(1),
            ..NfsFattr::default()
        };
        let mut nfh = Nfsfh::new();
        nfh.fh_bytes[0] = 0xaa;
        nfh.fh_bytes[NFSX_V3FH - 1] = 0xbb;
        nfh.fh_bytes[NFSX_V3FH] = 0xcc; // past the handle: not copied
        let fl = nfsrv_flrep(0x1_0000_0002, &fattr, &nfh);
        assert_eq!(FLREP_SIZE, 33 * 4);
        let w = words(&fl);
        assert_eq!(&w[..4], &[1, 2, 1, 1], "cookie, post_op_attr true, fa_type");
        assert_eq!(
            &w[24..26],
            &[1, NFSX_V3FH as u32],
            "post_op_fh3 true, length"
        );
        assert_eq!(fl[26 * 4], 0xaa);
        assert_eq!(fl[FLREP_SIZE - 1], 0xbb);
    }

    #[test]
    fn null_noop_and_bad_requests() {
        let (_g, mut s) = served(MNT_EXPORTED, 0);

        // NULL: an accepted reply with no status.
        let w = reply(&s, NFSPROC_NULL, &Req::default(), true);
        Rd::accepted(w, 0).done();

        // An obsolete procedure: PROC_UNAVAIL ...
        let w = reply(&s, NFSPROC_NOOP, &Req::default(), false);
        Rd::accepted(w, RPC_PROCUNAVAIL).done();
        // ... or the status nfs_getreq left.
        let mut nd = descript(&[], true, 0);
        nd.nd_repstat = Errno::EBADRPC.as_i32();
        let (m, e) = nfsrv_noop(&mut nd, &s.slp, s.p);
        assert_eq!(e, 0);
        let w = words(&test_bytes(&m.expect("a reply")));
        Rd::accepted(w, RPC_GARBAGE).done();

        // A version 3 file handle of the wrong length: GARBAGE_ARGS, no NFS status, for the
        // handle of every procedure that takes one, and for link's second.
        let bad = Req::default().u(12).raw(&[0; 12]);
        for procnum in 1..NFS_NPROCS - 1 {
            let w = reply(&s, procnum, &bad, true);
            Rd::accepted(w, RPC_GARBAGE).done();
        }
        let link = Req::default().fh(&s.root, true).u(NFSX_V3FH as u32 + 4);
        let w = reply(&s, NFSPROC_LINK, &link, true);
        Rd::accepted(w, RPC_GARBAGE).done();

        // A request that ends inside the file handle is dropped without a reply.
        let short = Req::default().u(NFSX_V3FH as u32).raw(&[0; 8]);
        for procnum in 1..NFS_NPROCS - 1 {
            assert_eq!(
                run(&s, procnum, &short, true),
                (Errno::EBADRPC.as_i32(), None)
            );
        }

        // An unknown file system: STALE.
        let mut nofs = Fhandle::default();
        nofs.fh_fsid.val = [0x7eed_beef, 0x1234];
        let unknown = Req::default().fh(&nofs, true);
        assert_eq!(status3(&s, NFSPROC_GETATTR, &unknown), NFSERR_STALE);

        // A client on an unreserved port, and a request with no client address, do not pass
        // the port check: MSG_DENIED, AUTH_ERROR, AUTH_TOOWEAK.
        let getattr = Req::default().fh(&s.root, true);
        for port in [1024, 0] {
            s.port = port;
            let w = reply(&s, NFSPROC_GETATTR, &getattr, true);
            assert_eq!(w, [0x1234, 1, 1, 1, AUTH_TOOWEAK]);
        }
        s.port = 20;
        s.slp
            .ns_sotype
            .store(SOCK_STREAM, core::sync::atomic::Ordering::Relaxed);
        let w = reply(&s, NFSPROC_GETATTR, &getattr, true);
        assert_eq!(w, [0x1234, 1, 1, 1, AUTH_TOOWEAK], "port 20 on a stream");

        // The table is in procedure number order.
        let t = nfsrv3_procs::<G>();
        assert!(core::ptr::fn_addr_eq(
            t[NFSPROC_GETATTR],
            nfsrv_getattr::<G> as NfsrvProc<G>
        ));
        assert!(core::ptr::fn_addr_eq(
            t[NFSPROC_COMMIT],
            nfsrv_commit::<G> as NfsrvProc<G>
        ));
        assert!(core::ptr::fn_addr_eq(
            t[NFS_NPROCS - 1],
            nfsrv_noop::<G> as NfsrvProc<G>
        ));
    }

    #[test]
    fn procedures_over_an_exported_ffs_version_3() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        let root = s.root;
        let motd = lookup(&s, &root, b"motd").expect("motd");
        assert_eq!(lookup(&s, &root, b"nothere"), Err(NFSERR_NOENT));

        // GETATTR: a regular file of 13 bytes, mode 0644.
        let w = reply(&s, NFSPROC_GETATTR, &Req::default().fh(&motd, true), true);
        let mut r = Rd::status(w, 0);
        let fa = r.n(21);
        r.done();
        assert_eq!((fa[0], fa[1] & 0o7777, fa[5], fa[6]), (1, 0o644, 0, 13));

        // ACCESS: root may do all but execute a file with no execute bit.
        let w = reply(
            &s,
            NFSPROC_ACCESS,
            &Req::default().fh(&motd, true).u(0x3f),
            true,
        );
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        assert_eq!(r.u(), 0x3f & !0x20);
        r.done();

        // READ: the bytes, their count, eof.
        let read = |fh: &Fhandle, off: u64, cnt: u32| {
            let w = reply(
                &s,
                NFSPROC_READ,
                &Req::default().fh(fh, true).hyper(off).u(cnt),
                true,
            );
            let mut r = Rd::status(w, 0);
            assert!(r.postop().is_some());
            let count = r.u();
            let eof = r.u();
            let data = r.opaque();
            r.done();
            assert_eq!(count as usize, data.len());
            (data, eof)
        };
        assert_eq!(read(&motd, 0, 100), (b"hello, world\n".to_vec(), 1));
        assert_eq!(read(&motd, 7, 3), (b"wor".to_vec(), 0));
        assert_eq!(read(&motd, 13, 10), (Vec::new(), 1));

        // CREATE a file (mode 0640), WRITE it in the three stable modes, READ it back.
        let w = reply(
            &s,
            NFSPROC_CREATE,
            &Req::default()
                .fh(&root, true)
                .name(b"new")
                .u(0)
                .sattr3(Some(0o640), None),
            true,
        );
        let mut r = Rd::status(w, 0);
        let new = r.postop_fh().expect("a handle");
        let fa = r.postop().expect("attributes");
        assert_eq!(fa[1] & 0o7777, 0o640);
        let (pre, post) = r.wcc();
        assert!(pre.is_some() && post.is_some());
        r.done();
        for (off, stable, data, committed) in [
            (
                0u64,
                NFSV3WRITE_FILESYNC,
                &b"abcde"[..],
                NFSV3WRITE_FILESYNC,
            ),
            (5, NFSV3WRITE_UNSTABLE, b"fgh", NFSV3WRITE_UNSTABLE),
            (8, NFSV3WRITE_DATASYNC, b"ij", NFSV3WRITE_FILESYNC),
        ] {
            let body = Req::default()
                .fh(&new, true)
                .hyper(off)
                .u(data.len() as u32)
                .u(stable)
                .name(data);
            let w = reply(&s, NFSPROC_WRITE, &body, true);
            let mut r = Rd::status(w, 0);
            let (pre, post) = r.wcc();
            assert!(pre.is_some() && post.is_some());
            assert_eq!(r.u() as usize, data.len());
            assert_eq!(r.u(), committed);
            let verf = r.n(2);
            r.done();
            let bt = microboottime();
            assert_eq!(verf, [bt.tv_sec as u32, bt.tv_usec as u32]);
        }
        assert_eq!(read(&new, 0, 4096), (b"abcdefghij".to_vec(), 1));

        // SETATTR: the size cut to 2.
        let w = reply(
            &s,
            NFSPROC_SETATTR,
            &Req::default().fh(&new, true).sattr3(None, Some(2)).u(0),
            true,
        );
        let mut r = Rd::status(w, 0);
        let (_, post) = r.wcc();
        assert_eq!(post.expect("attributes")[6], 2);
        r.done();
        assert_eq!(read(&new, 0, 100).0, b"ab");

        // MKDIR, SYMLINK, READLINK.
        let w = reply(
            &s,
            NFSPROC_MKDIR,
            &Req::default()
                .fh(&root, true)
                .name(b"d")
                .sattr3(Some(0o755), None),
            true,
        );
        let mut r = Rd::status(w, 0);
        let d = r.postop_fh().expect("a handle");
        assert_eq!(r.postop().expect("attributes")[0], 2);
        r.wcc();
        r.done();
        let w = reply(
            &s,
            NFSPROC_SYMLINK,
            &Req::default()
                .fh(&root, true)
                .name(b"l")
                .sattr3(Some(0o755), None)
                .name(b"new"),
            true,
        );
        let mut r = Rd::status(w, 0);
        let l = r.postop_fh().expect("a handle");
        assert_eq!(r.postop().expect("attributes")[0], 5);
        r.wcc();
        r.done();
        let w = reply(&s, NFSPROC_READLINK, &Req::default().fh(&l, true), true);
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        assert_eq!(r.opaque(), b"new");
        r.done();
        // READLINK of a file that is not a link: INVAL.
        let w = reply(&s, NFSPROC_READLINK, &Req::default().fh(&new, true), true);
        let mut r = Rd::status(w, NFSERR_INVAL);
        assert!(r.postop().is_some());
        r.done();

        // MKNOD a character device (VOP_MKNOD, then the name looked up again) and a socket
        // (VOP_CREATE). Not a fifo: the kernel's inherited gap (no fifofs: ffs_vinit refuses
        // one with EOPNOTSUPP after the node is made).
        for (name, t, spec) in [
            (&b"tty"[..], Nfstype::NFCHR, true),
            (b"sock", Nfstype::NFSOCK, false),
        ] {
            let rq = Req::default()
                .fh(&root, true)
                .name(name)
                .u(t as u32)
                .sattr3(Some(0o600), None);
            let rq = if spec { rq.u(2).u(0) } else { rq };
            let mut r = Rd::status(reply(&s, NFSPROC_MKNOD, &rq, true), 0);
            assert!(r.postop_fh().is_some());
            assert_eq!(r.postop().expect("attributes")[0], t as u32);
            r.wcc();
            r.done();
        }

        // READDIR and READDIRPLUS list them all.
        let w = reply(
            &s,
            NFSPROC_READDIR,
            &Req::default().fh(&root, true).hyper(0).hyper(0).u(4096),
            true,
        );
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        r.hyper();
        let mut names = Vec::new();
        while r.u() == 1 {
            r.hyper();
            names.push(r.opaque());
            r.hyper();
        }
        assert_eq!(r.u(), 1, "eof");
        r.done();
        names.sort();
        let want: [&[u8]; 8] = [b".", b"..", b"d", b"l", b"motd", b"new", b"sock", b"tty"];
        assert_eq!(names, want);
        let w = reply(
            &s,
            NFSPROC_READDIRPLUS,
            &Req::default()
                .fh(&root, true)
                .hyper(0)
                .hyper(0)
                .u(4096)
                .u(8192),
            true,
        );
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        r.hyper();
        let mut n = 0;
        while r.u() == 1 {
            r.hyper();
            let name = r.opaque();
            r.hyper();
            assert!(r.postop().is_some());
            let fh = r.postop_fh().expect("a handle");
            if name == b"motd" {
                assert_eq!(fh, motd);
            }
            n += 1;
        }
        assert_eq!((n, r.u()), (8, 1));
        r.done();

        // RENAME new to d/n2, LINK it back as hard, REMOVE both names, RMDIR d.
        let w = reply(
            &s,
            NFSPROC_RENAME,
            &Req::default()
                .fh(&root, true)
                .name(b"new")
                .fh(&d, true)
                .name(b"n2"),
            true,
        );
        let mut r = Rd::status(w, 0);
        r.wcc();
        r.wcc();
        r.done();
        assert_eq!(lookup(&s, &root, b"new"), Err(NFSERR_NOENT));
        let n2 = lookup(&s, &d, b"n2").expect("renamed");
        assert_eq!(n2, new, "the same file");
        let w = reply(
            &s,
            NFSPROC_LINK,
            &Req::default().fh(&n2, true).fh(&root, true).name(b"hard"),
            true,
        );
        let mut r = Rd::status(w, 0);
        assert_eq!(r.postop().expect("attributes")[2], 2, "two links");
        r.wcc();
        r.done();
        let w = reply(
            &s,
            NFSPROC_RMDIR,
            &Req::default().fh(&root, true).name(b"d"),
            true,
        );
        let mut r = Rd::status(w, NFSERR_NOTEMPTY);
        r.wcc();
        r.done();
        for (dir, name) in [
            (&root, &b"hard"[..]),
            (&d, b"n2"),
            (&root, b"tty"),
            (&root, b"sock"),
            (&root, b"l"),
        ] {
            let w = reply(
                &s,
                NFSPROC_REMOVE,
                &Req::default().fh(dir, true).name(name),
                true,
            );
            let mut r = Rd::status(w, 0);
            r.wcc();
            r.done();
        }
        assert_eq!(
            status3(
                &s,
                NFSPROC_RMDIR,
                &Req::default().fh(&root, true).name(b"d")
            ),
            0
        );

        // FSSTAT, FSINFO, PATHCONF, COMMIT.
        let w = reply(&s, NFSPROC_FSSTAT, &Req::default().fh(&root, true), true);
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        let sf = r.n(13);
        r.done();
        assert!(sf[1] > 0, "total bytes");
        let w = reply(&s, NFSPROC_FSINFO, &Req::default().fh(&root, true), true);
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        let fi = r.n(12);
        r.done();
        assert_eq!(
            &fi[..3],
            &[NFS_MAXDATA as u32, NFS_MAXDGRAMDATA as u32, 512]
        );
        assert_eq!(&fi[7..9], &[u32::MAX, u32::MAX]);
        assert_eq!(fi[11], 0x1b);
        let w = reply(&s, NFSPROC_PATHCONF, &Req::default().fh(&root, true), true);
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        let pc = r.n(6);
        r.done();
        assert_eq!((pc[1], pc[4], pc[5]), (255, 0, 1));
        let w = reply(
            &s,
            NFSPROC_COMMIT,
            &Req::default().fh(&motd, true).hyper(0).u(0),
            true,
        );
        let mut r = Rd::status(w, 0);
        r.wcc();
        r.n(2);
        r.done();

        // Every vnode the procedures took was given back: the unmount is clean.
        testffs::unmount_root(s.p, s.mp);
    }

    #[test]
    fn procedures_over_an_exported_ffs_version_2() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        let root = s.root;
        let lookup2 = |name: &[u8]| {
            let w = reply(
                &s,
                NFSPROC_LOOKUP,
                &Req::default().fh(&root, false).name(name),
                false,
            );
            let mut r = Rd::status(w, 0);
            let fh = r.n(8);
            let fa = r.n(17);
            r.done();
            let b: Vec<u8> = fh.iter().flat_map(|w| w.to_be_bytes()).collect();
            (xdr_get::<Fhandle>(&b), fa)
        };
        let (motd, fa) = lookup2(b"motd");
        assert_eq!((fa[0], fa[5]), (1, 13));
        // A version 2 error is the status alone.
        let w = reply(
            &s,
            NFSPROC_LOOKUP,
            &Req::default().fh(&root, false).name(b"no"),
            false,
        );
        Rd::status(w, NFSERR_NOENT).done();

        // READ: attributes, then the data.
        let w = reply(
            &s,
            NFSPROC_READ,
            &Req::default().fh(&motd, false).u(0).u(100).u(0),
            false,
        );
        let mut r = Rd::status(w, 0);
        assert_eq!(r.n(17)[5], 13);
        assert_eq!(r.opaque(), b"hello, world\n");
        r.done();

        // CREATE (sattr), WRITE (beginoffset, offset, totalcount, data), READ it back.
        let w = reply(
            &s,
            NFSPROC_CREATE,
            &Req::default()
                .fh(&root, false)
                .name(b"f2")
                .sattr2(0o100_600),
            false,
        );
        let mut r = Rd::status(w, 0);
        let fh: Vec<u8> = r.n(8).iter().flat_map(|w| w.to_be_bytes()).collect();
        let f2 = xdr_get::<Fhandle>(&fh);
        assert_eq!(r.n(17)[1] & 0o7777, 0o600);
        r.done();
        let w = reply(
            &s,
            NFSPROC_WRITE,
            &Req::default().fh(&f2, false).u(0).u(0).u(0).name(b"xyz"),
            false,
        );
        let mut r = Rd::status(w, 0);
        assert_eq!(r.n(17)[5], 3);
        r.done();

        // Reading a directory: ISDIR; MKDIR (a mode word); READDIR; STATFS.
        let w = reply(
            &s,
            NFSPROC_READ,
            &Req::default().fh(&root, false).u(0).u(10).u(0),
            false,
        );
        Rd::status(w, NFSERR_ISDIR).done();
        let w = reply(
            &s,
            NFSPROC_MKDIR,
            &Req::default().fh(&root, false).name(b"d2").sattr2(0o755),
            false,
        );
        let mut r = Rd::status(w, 0);
        r.n(8);
        assert_eq!(r.n(17)[0], 2);
        r.done();
        let w = reply(
            &s,
            NFSPROC_READDIR,
            &Req::default().fh(&root, false).u(0).u(4096),
            false,
        );
        let mut r = Rd::status(w, 0);
        let mut n = 0;
        while r.u() == 1 {
            r.u();
            r.opaque();
            r.u();
            n += 1;
        }
        assert_eq!((n, r.u()), (5, 1));
        r.done();
        let w = reply(&s, NFSPROC_FSSTAT, &Req::default().fh(&root, false), false);
        let mut r = Rd::status(w, 0);
        assert_eq!(r.u(), NFS_MAXDGRAMDATA as u32);
        r.n(4);
        r.done();

        // GETATTR: the file's attributes after the status.
        let w = reply(&s, NFSPROC_GETATTR, &Req::default().fh(&f2, false), false);
        let mut r = Rd::status(w, 0);
        let fa = r.n(17);
        r.done();
        assert_eq!((fa[0], fa[1] & 0o7777, fa[5]), (1, 0o600, 3));
        // SETATTR with the Sun client's 0xffff mode ("don't set") and a size: the mode is
        // kept, the size is set; the reply carries the attributes.
        let sattr = |mode: u32, size: u32| {
            Req::default()
                .fh(&f2, false)
                .u(mode)
                .u(u32::MAX)
                .u(u32::MAX)
                .u(size)
                .raw(&[0xff; 16])
        };
        let w = reply(&s, NFSPROC_SETATTR, &sattr(0xffff, 2), false);
        let mut r = Rd::status(w, 0);
        let fa = r.n(17);
        r.done();
        assert_eq!((fa[1] & 0o7777, fa[5]), (0o600, 2), "mode kept, size set");
        // A real mode is set.
        let w = reply(&s, NFSPROC_SETATTR, &sattr(0o100_640, u32::MAX), false);
        let mut r = Rd::status(w, 0);
        let fa = r.n(17);
        r.done();
        assert_eq!((fa[1] & 0o7777, fa[5]), (0o640, 2));

        // SYMLINK: file handle, name, the target, then the sattr (status alone back);
        // READLINK of it gives the target, of a regular file ENXIO (EINVAL in version 3).
        let w = reply(
            &s,
            NFSPROC_SYMLINK,
            &Req::default()
                .fh(&root, false)
                .name(b"sl")
                .name(b"motd")
                .sattr2(0o120_777),
            false,
        );
        Rd::status(w, 0).done();
        let (sl, fa) = lookup2(b"sl");
        assert_eq!(fa[0], 5, "a symbolic link");
        let w = reply(&s, NFSPROC_READLINK, &Req::default().fh(&sl, false), false);
        let mut r = Rd::status(w, 0);
        assert_eq!(r.opaque(), b"motd");
        r.done();
        let w = reply(
            &s,
            NFSPROC_READLINK,
            &Req::default().fh(&motd, false),
            false,
        );
        Rd::status(w, NFSERR_NXIO).done();

        // LINK: the file's handle, the directory's, the name; status alone.
        let w = reply(
            &s,
            NFSPROC_LINK,
            &Req::default().fh(&motd, false).fh(&root, false).name(b"lk"),
            false,
        );
        Rd::status(w, 0).done();
        let (lk, fa) = lookup2(b"lk");
        assert_eq!((lk, fa[2]), (motd, 2), "the same file, two links");
        for name in [&b"lk"[..], b"sl"] {
            let w = reply(
                &s,
                NFSPROC_REMOVE,
                &Req::default().fh(&root, false).name(name),
                false,
            );
            Rd::status(w, 0).done();
        }

        // A rename of a directory over a file: ISDIR in version 2 (EEXIST in version 3).
        let ren = Req::default()
            .fh(&root, false)
            .name(b"d2")
            .fh(&root, false)
            .name(b"f2");
        Rd::status(reply(&s, NFSPROC_RENAME, &ren, false), NFSERR_ISDIR).done();
        for name in [&b"f2"[..], b"d2"] {
            let pn = if name == b"f2" {
                NFSPROC_REMOVE
            } else {
                NFSPROC_RMDIR
            };
            let w = reply(&s, pn, &Req::default().fh(&root, false).name(name), false);
            Rd::status(w, 0).done();
        }
        testffs::unmount_root(s.p, s.mp);
    }

    #[test]
    fn a_read_only_export_refuses_every_change() {
        let (_g, s) = served(MNT_EXPORTED | MNT_EXRDONLY, 0);
        let root = s.root;
        let motd = lookup(&s, &root, b"motd").expect("motd");
        let write = Req::default().fh(&motd, true).hyper(0).u(1).u(0).name(b"x");
        let mut r = Rd::status(reply(&s, NFSPROC_WRITE, &write, true), NFSERR_ROFS);
        r.wcc();
        r.done();
        let size0 = Req::default().fh(&motd, true).sattr3(None, Some(0)).u(0);
        assert_eq!(status3(&s, NFSPROC_SETATTR, &size0), NFSERR_ROFS);
        let mode = Req::default()
            .fh(&motd, true)
            .sattr3(Some(0o600), None)
            .u(0);
        assert_eq!(status3(&s, NFSPROC_SETATTR, &mode), NFSERR_ROFS);
        let create = Req::default()
            .fh(&root, true)
            .name(b"c")
            .u(0)
            .sattr3(None, None);
        assert_eq!(status3(&s, NFSPROC_CREATE, &create), NFSERR_ROFS);
        let mkdir = Req::default().fh(&root, true).name(b"c").sattr3(None, None);
        assert_eq!(status3(&s, NFSPROC_MKDIR, &mkdir), NFSERR_ROFS);
        let remove = Req::default().fh(&root, true).name(b"motd");
        assert_eq!(status3(&s, NFSPROC_REMOVE, &remove), NFSERR_ROFS);
        let ren = Req::default()
            .fh(&root, true)
            .name(b"motd")
            .fh(&root, true)
            .name(b"m2");
        Rd::status(reply(&s, NFSPROC_RENAME, &ren, true), NFSERR_ROFS);
        // ACCESS takes the write bits away; READ still works.
        let w = reply(
            &s,
            NFSPROC_ACCESS,
            &Req::default().fh(&motd, true).u(0x0d),
            true,
        );
        let mut r = Rd::status(w, 0);
        r.postop();
        assert_eq!(r.u(), 0x01);
        r.done();
        let v2 = Req::default().fh(&motd, false).u(0).u(0).u(0).name(b"x");
        Rd::status(reply(&s, NFSPROC_WRITE, &v2, false), NFSERR_ROFS).done();
        assert_eq!(lookup(&s, &root, b"motd"), Ok(motd));
        testffs::unmount_root(s.p, s.mp);
    }

    #[test]
    fn mapall_maps_root_and_the_owner_keeps_its_open_file() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        let root = s.root;
        // As root (maproot=root): a directory everyone may write into.
        let w = reply(
            &s,
            NFSPROC_MKDIR,
            &Req::default()
                .fh(&root, true)
                .name(b"pub")
                .sattr3(Some(0o777), None),
            true,
        );
        let mut r = Rd::status(w, 0);
        let public = r.postop_fh().expect("a handle");
        // Every client is nobody now.
        export(s.mp, MNT_DELEXPORT, 0);
        export(s.mp, MNT_EXPORTED | MNT_EXPORTANON, 32767);
        let w = reply(
            &s,
            NFSPROC_CREATE,
            &Req::default()
                .fh(&public, true)
                .name(b"mine")
                .u(0)
                .sattr3(Some(0o644), None),
            true,
        );
        let mut r = Rd::status(w, 0);
        let mine = r.postop_fh().expect("a handle");
        assert_eq!(
            r.postop().expect("attributes")[3],
            32767,
            "owned by the anonymous uid"
        );
        // Root's file is out of reach ...
        let motd = lookup(&s, &root, b"motd").expect("motd");
        let w = Req::default().fh(&motd, true).hyper(0).u(1).u(2).name(b"x");
        Rd::status(reply(&s, NFSPROC_WRITE, &w, true), NFSERR_ACCES);
        // ... but the owner writes its own file after taking its write bit away (the owner
        // override of a file already open), and reads it after taking every bit away.
        let mode0 = Req::default()
            .fh(&mine, true)
            .sattr3(Some(0o444), None)
            .u(0);
        assert_eq!(status3(&s, NFSPROC_SETATTR, &mode0), 0);
        let w = Req::default()
            .fh(&mine, true)
            .hyper(0)
            .u(2)
            .u(2)
            .name(b"ok");
        assert_eq!(status3(&s, NFSPROC_WRITE, &w), 0);
        let mode0 = Req::default().fh(&mine, true).sattr3(Some(0), None).u(0);
        assert_eq!(status3(&s, NFSPROC_SETATTR, &mode0), 0);
        let rd = Req::default().fh(&mine, true).hyper(0).u(10);
        assert_eq!(status3(&s, NFSPROC_READ, &rd), 0);
        // ACCESS has no override: nothing is granted on a mode 0 file.
        let w = reply(
            &s,
            NFSPROC_ACCESS,
            &Req::default().fh(&mine, true).u(0x2d),
            true,
        );
        let mut r = Rd::status(w, 0);
        r.postop();
        assert_eq!(r.u(), 0);
        r.done();
        // A device node needs root, whom mapall maps away; a socket's does not.
        let chr = Req::default()
            .fh(&public, true)
            .name(b"tty")
            .u(Nfstype::NFCHR as u32)
            .sattr3(None, None)
            .u(1)
            .u(2);
        // EPERM is not one of MKNOD's version 3 errors: the C's map answers IO.
        assert_eq!(status3(&s, NFSPROC_MKNOD, &chr), NFSERR_IO);
        let sock = Req::default()
            .fh(&public, true)
            .name(b"q")
            .u(Nfstype::NFSOCK as u32)
            .sattr3(None, None);
        assert_eq!(status3(&s, NFSPROC_MKNOD, &sock), 0);
        // So does removing a directory with REMOVE.
        let sub = Req::default()
            .fh(&public, true)
            .name(b"sub")
            .sattr3(Some(0o755), None);
        assert_eq!(status3(&s, NFSPROC_MKDIR, &sub), 0);
        let rm = Req::default().fh(&public, true).name(b"sub");
        // EPERM is not one of REMOVE's version 3 errors either: IO; the directory stays.
        assert_eq!(status3(&s, NFSPROC_REMOVE, &rm), NFSERR_IO);
        assert!(lookup(&s, &public, b"sub").is_ok());
        testffs::unmount_root(s.p, s.mp);
    }

    /// Version 3 CREATE of `name` under the root and WRITE of `data` at `off` (one request).
    fn put(s: &Srv, name: &[u8], off: u64, data: &[u8], create: bool) -> Fhandle {
        let root = s.root;
        let fh = if create {
            let rq = Req::default()
                .fh(&root, true)
                .name(name)
                .u(0)
                .sattr3(Some(0o644), None);
            Rd::status(reply(s, NFSPROC_CREATE, &rq, true), 0)
                .postop_fh()
                .expect("a handle")
        } else {
            lookup(s, &root, name).expect("the file")
        };
        let rq = Req::default()
            .fh(&fh, true)
            .hyper(off)
            .u(data.len() as u32)
            .u(NFSV3WRITE_FILESYNC)
            .name(data);
        let mut r = Rd::status(reply(s, NFSPROC_WRITE, &rq, true), 0);
        r.wcc();
        assert_eq!(r.u() as usize, data.len(), "written");
        fh
    }

    /// A known pattern with no zero byte.
    fn pattern(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i % 251) as u8 + 1).collect()
    }

    #[test]
    fn a_read_of_several_clusters_ends_at_the_end_of_file_with_zero_padding() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        let data = pattern(9003);
        let big = put(&s, b"big", 0, &data, true);
        // 8192 bytes asked from 1000: 8003 are left, so eof; 8003 is padded by one byte.
        let rq = Req::default().fh(&big, true).hyper(1000).u(8192);
        let w = reply(&s, NFSPROC_READ, &rq, true);
        let mut r = Rd::status(w, 0);
        assert!(r.postop().is_some());
        assert_eq!(r.u(), 8003, "count");
        assert_eq!(r.u(), 1, "eof");
        assert_eq!(r.u(), 8003, "the opaque's length");
        let ws = r.n(8003usize.div_ceil(4));
        r.done();
        let bytes: Vec<u8> = ws.iter().flat_map(|w| w.to_be_bytes()).collect();
        assert_eq!(&bytes[..8003], &data[1000..], "the data");
        assert_eq!(&bytes[8003..], &[0], "the XDR padding is zero");
        // The same asked short of the end: no eof, no padding.
        let rq = Req::default().fh(&big, true).hyper(0).u(8192);
        let mut r = Rd::status(reply(&s, NFSPROC_READ, &rq, true), 0);
        r.postop();
        assert_eq!((r.u(), r.u()), (8192, 0));
        assert_eq!(r.opaque(), &data[..8192]);
        r.done();
        // Version 2: attributes, then the data.
        let rq = Req::default().fh(&big, false).u(1000).u(8192).u(0);
        let mut r = Rd::status(reply(&s, NFSPROC_READ, &rq, false), 0);
        assert_eq!(r.n(17)[5], 9003);
        assert_eq!(r.opaque(), &data[1000..]);
        r.done();
        testffs::unmount_root(s.p, s.mp);
    }

    #[test]
    fn a_write_cut_over_many_small_mbufs_lands_whole() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        // The request is cut in 50-byte mbufs (`descript`): 3001 bytes are ~60 pieces, the
        // first not aligned to the arguments.
        let data = pattern(3001);
        let f = put(&s, b"w", 13, &data, true);
        let w = reply(&s, NFSPROC_GETATTR, &Req::default().fh(&f, true), true);
        let mut r = Rd::status(w, 0);
        assert_eq!(r.n(21)[6], 3014, "size");
        r.done();
        let rq = Req::default().fh(&f, true).hyper(0).u(4096);
        let mut r = Rd::status(reply(&s, NFSPROC_READ, &rq, true), 0);
        r.postop();
        assert_eq!((r.u(), r.u()), (3014, 1));
        let got = r.opaque();
        r.done();
        assert_eq!(&got[..13], &[0; 13], "the hole before the write");
        assert_eq!(&got[13..], &data[..], "the data");
        // A second write over the middle, again through the pieces.
        let patch = [0xeeu8; 700];
        put(&s, b"w", 1000, &patch, false);
        let rq = Req::default().fh(&f, true).hyper(0).u(4096);
        let mut r = Rd::status(reply(&s, NFSPROC_READ, &rq, true), 0);
        r.postop();
        r.u();
        r.u();
        let got = r.opaque();
        assert_eq!(&got[1000..1700], &patch[..]);
        assert_eq!(&got[1700..], &data[1687..]);
        assert_eq!(&got[13..1000], &data[..987]);
        testffs::unmount_root(s.p, s.mp);
    }

    #[test]
    fn the_wire_bounds_are_checked_before_use() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        let root = s.root;
        let motd = lookup(&s, &root, b"motd").expect("motd");

        // Names: 1..=NFS_MAXNAMLEN, no `/`, no NUL.
        let long = [b'a'; NFS_MAXNAMLEN + 1];
        let w = reply(
            &s,
            NFSPROC_LOOKUP,
            &Req::default().fh(&root, true).name(&long),
            true,
        );
        Rd::status(w, NFSERR_NAMETOL).done();
        let max = [b'a'; NFS_MAXNAMLEN];
        assert_eq!(lookup(&s, &root, &max), Err(NFSERR_NOENT));
        let w = reply(
            &s,
            NFSPROC_LOOKUP,
            &Req::default().fh(&root, true).name(b""),
            true,
        );
        Rd::accepted(w, RPC_GARBAGE).done();
        let w = reply(
            &s,
            NFSPROC_LOOKUP,
            &Req::default().fh(&root, true).u(u32::MAX),
            true,
        );
        Rd::accepted(w, RPC_GARBAGE).done();
        assert_eq!(lookup(&s, &root, b"motd/x"), Err(NFSERR_ACCES));
        assert_eq!(lookup(&s, &root, b"mo\0d"), Err(NFSERR_ACCES));
        // A name longer than the request: nfs_namei's EBADRPC, a GARBAGE reply.
        let cut = Req::default().fh(&root, true).u(10).raw(b"abc");
        let w = reply(&s, NFSPROC_LOOKUP, &cut, true);
        Rd::accepted(w, RPC_GARBAGE).done();
        // A symlink target past NFS_MAXPATHLEN, rename's second name past NFS_MAXNAMLEN.
        let sym = Req::default()
            .fh(&root, true)
            .name(b"s")
            .sattr3(None, None)
            .u(NFS_MAXPATHLEN as u32 + 1);
        assert_eq!(
            run(&s, NFSPROC_SYMLINK, &sym, true),
            (Errno::EBADRPC.as_i32(), None)
        );
        let ren = Req::default()
            .fh(&root, true)
            .name(b"motd")
            .fh(&root, true)
            .u(NFS_MAXNAMLEN as u32 + 1);
        assert_eq!(
            run(&s, NFSPROC_RENAME, &ren, true),
            (Errno::EBADRPC.as_i32(), None)
        );

        // Read counts: 1..=NFS_SRVMAXDATA (a datagram's NFS_MAXDGRAMDATA in version 3).
        for cnt in [0, u32::MAX, NFS_MAXDGRAMDATA as u32 + 1] {
            let rd = Req::default().fh(&motd, true).hyper(0).u(cnt);
            Rd::accepted(reply(&s, NFSPROC_READ, &rd, true), RPC_GARBAGE).done();
        }
        let rd = Req::default()
            .fh(&motd, true)
            .hyper(0)
            .u(NFS_MAXDGRAMDATA as u32);
        assert_eq!(status3(&s, NFSPROC_READ, &rd), 0);

        // Readdir counts: 0 is TOOSMALL (GARBAGE in version 2); the rest are clamped.
        let rdd = Req::default().fh(&root, true).hyper(0).hyper(0).u(0);
        let mut r = Rd::status(reply(&s, NFSPROC_READDIR, &rdd, true), NFSERR_TOOSMALL);
        assert!(r.postop().is_none());
        r.done();
        let rdd2 = Req::default().fh(&root, false).u(0).u(0);
        Rd::accepted(reply(&s, NFSPROC_READDIR, &rdd2, false), RPC_GARBAGE).done();
        for (dircount, maxcount) in [(0, 4096), (4096, 0)] {
            let rdp = Req::default()
                .fh(&root, true)
                .hyper(0)
                .hyper(0)
                .u(dircount)
                .u(maxcount);
            let r = Rd::status(reply(&s, NFSPROC_READDIRPLUS, &rdp, true), NFSERR_TOOSMALL);
            drop(r);
        }
        let huge = Req::default().fh(&root, true).hyper(0).hyper(0).u(u32::MAX);
        assert_eq!(status3(&s, NFSPROC_READDIR, &huge), 0);
        // A readdir of a file: NOTDIR.
        let notdir = Req::default().fh(&motd, true).hyper(0).hyper(0).u(512);
        assert_eq!(status3(&s, NFSPROC_READDIR, &notdir), NFSERR_NOTDIR);
        // A count too small for one entry: the entries that fit, eof false.
        let small = Req::default().fh(&root, true).hyper(0).hyper(0).u(30);
        let mut r = Rd::status(reply(&s, NFSPROC_READDIR, &small, true), 0);
        r.postop();
        r.hyper();
        assert_eq!((r.u(), r.u()), (0, 0), "no entry, not eof");
        r.done();

        // Writes: more than NFS_MAXDATA, a negative length, fewer bytes than the length.
        let w = |len: u32, data: &[u8]| {
            Req::default()
                .fh(&motd, true)
                .hyper(0)
                .u(len)
                .u(2)
                .u(len)
                .raw(data)
        };
        for (len, data) in [
            (NFS_MAXDATA as u32 + 1, &b"abcd"[..]),
            (u32::MAX, b"abcd"),
            (10, b"abcd"),
        ] {
            let mut r = Rd::status(reply(&s, NFSPROC_WRITE, &w(len, data), true), NFSERR_IO);
            assert_eq!(r.wcc(), (None, None));
            r.done();
        }
        // A write of exactly what is there, and of nothing.
        assert_eq!(status3(&s, NFSPROC_WRITE, &w(4, b"abcd")), 0);
        assert_eq!(status3(&s, NFSPROC_WRITE, &w(0, b"")), 0);
        assert_eq!(
            status3(
                &s,
                NFSPROC_READ,
                &Req::default().fh(&motd, true).hyper(0).u(4)
            ),
            0
        );
        // A write to a directory: INVAL (version 3).
        let wd = Req::default().fh(&root, true).hyper(0).u(1).u(2).name(b"x");
        Rd::status(reply(&s, NFSPROC_WRITE, &wd, true), NFSERR_INVAL);
        testffs::unmount_root(s.p, s.mp);
    }

    #[test]
    fn rename_and_create_follow_the_c_s_rules() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        let root = s.root;
        let mkdir = |name: &[u8]| {
            let rq = Req::default()
                .fh(&root, true)
                .name(name)
                .sattr3(Some(0o755), None);
            let mut r = Rd::status(reply(&s, NFSPROC_MKDIR, &rq, true), 0);
            r.postop_fh().expect("a handle")
        };
        let d = mkdir(b"d");
        mkdir(b"e");
        let ren = |from: &[u8], tdir: &Fhandle, to: &[u8], v3: bool| {
            let rq = Req::default()
                .fh(&root, v3)
                .name(from)
                .fh(tdir, v3)
                .name(to);
            let w = reply(&s, NFSPROC_RENAME, &rq, v3);
            Rd::accepted(w, 0).u() as i32
        };
        // A directory over a file, a file over a directory.
        assert_eq!(ren(b"d", &root, b"motd", true), NFSERR_EXIST);
        assert_eq!(ren(b"motd", &root, b"d", true), NFSERR_EXIST);
        assert_eq!(ren(b"motd", &root, b"d", false), NFSERR_NOTDIR);
        // A directory into itself: INVAL (NOTEMPTY in version 2).
        assert_eq!(ren(b"d", &d, b"x", true), NFSERR_INVAL);
        assert_eq!(ren(b"d", &d, b"x", false), NFSERR_NOTEMPTY);
        // A name onto itself: nothing to do, and no error.
        assert_eq!(ren(b"motd", &root, b"motd", true), 0);
        assert!(lookup(&s, &root, b"motd").is_ok());
        // A directory over an empty directory.
        assert_eq!(ren(b"d", &root, b"e", true), 0);
        assert_eq!(lookup(&s, &root, b"d"), Err(NFSERR_NOENT));

        // CREATE: GUARDED refuses an existing name, UNCHECKED truncates it, EXCLUSIVE with
        // the same verifier again (a retransmission) is refused with EEXIST. On 64-bit the
        // C's 8-byte verifier `bcopy` fills only `tv_sec`, so `tv_nsec` stays VNOVAL and
        // `ufs_setattr` sets no time (nfs_serv.c:1119, ufs_vnops.c:408-410): the verifier is
        // never stored, the retransmission fails the `bcmp` and answers EEXIST, in the C and
        // here.
        let create = |name: &[u8], how: u32, tail: Req| {
            let rq = Req::default()
                .fh(&root, true)
                .name(name)
                .u(how)
                .raw(&tail.0);
            reply(&s, NFSPROC_CREATE, &rq, true)
        };
        let w = create(
            b"motd",
            NFSV3CREATE_GUARDED,
            Req::default().sattr3(None, None),
        );
        let mut r = Rd::status(w, NFSERR_EXIST);
        r.wcc();
        r.done();
        let w = create(
            b"motd",
            NFSV3CREATE_UNCHECKED,
            Req::default().sattr3(None, Some(0)),
        );
        let mut r = Rd::status(w, 0);
        r.postop_fh();
        assert_eq!(r.postop().expect("attributes")[6], 0, "truncated");
        let verf = Req::default().raw(b"verifier");
        let w = create(b"x", NFSV3CREATE_EXCLUSIVE, verf.clone());
        Rd::status(w, 0);
        let w = create(b"x", NFSV3CREATE_EXCLUSIVE, verf.clone());
        let mut r = Rd::status(w, NFSERR_EXIST);
        r.wcc();
        r.done();
        let w = create(b"x", NFSV3CREATE_EXCLUSIVE, Req::default().raw(b"another!"));
        let mut r = Rd::status(w, NFSERR_EXIST);
        r.wcc();
        r.done();
        testffs::unmount_root(s.p, s.mp);
    }

    /// The next of a xorshift sequence.
    fn next(x: &mut u64) -> u64 {
        *x ^= *x << 13;
        *x ^= *x >> 7;
        *x ^= *x << 17;
        *x
    }

    #[test]
    fn random_requests_are_answered_or_dropped_never_panic() {
        let (_g, s) = served(MNT_EXPORTED, 0);
        let root = s.root;
        let motd = lookup(&s, &root, b"motd").expect("motd");
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..3000 {
            let v3 = next(&mut x) & 1 == 0;
            let procnum = (next(&mut x) % NFS_NPROCS as u64) as usize;
            let mut body = match next(&mut x) % 3 {
                0 => Req::default(),
                1 => Req::default().fh(&root, v3),
                _ => Req::default().fh(&motd, v3),
            };
            let n = (next(&mut x) % 48) as usize;
            for _ in 0..n {
                // Small words mostly (lengths, modes, counts), now and then any word.
                let w = next(&mut x);
                let w = if w & 3 == 0 {
                    w as u32
                } else {
                    (w % 40) as u32
                };
                body = body.u(w);
            }
            let (e, w) = run(&s, procnum, &body, v3);
            if let Some(w) = w {
                assert!(w.len() >= 3 && w[0] == 0x1234, "{w:x?}");
            } else {
                assert!(e != 0);
            }
        }
        // Whatever the requests made, they gave every vnode back.
        let list = |dir: &Fhandle| {
            let rdd = Req::default().fh(dir, true).hyper(0).hyper(0).u(8192);
            let mut r = Rd::status(reply(&s, NFSPROC_READDIR, &rdd, true), 0);
            r.postop();
            r.hyper();
            let mut names = Vec::new();
            while r.u() == 1 {
                r.hyper();
                names.push(r.opaque());
                r.hyper();
            }
            names
        };
        let names = list(&root);
        assert!(names.iter().any(|n| n == b"motd"));
        testffs::unmount_root(s.p, s.mp);
    }
}
/* </TESTS> */
