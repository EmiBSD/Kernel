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
//!   reference) and gives the saved name back.
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
//! - `nfsrv3_procs[]` (`nfs_syscalls.c`) is `nfsrv3_procs`, the table of one glue's
//!   procedures, built where they are.

use core::sync::atomic::Ordering::Relaxed;

use super::nfs_srvsocket::nfs_rephead;
use super::nfs_srvsubs::{
    nfs_namei, nfsm_adj, nfsm_srvfattr, nfsm_srvfhtom, nfsm_srvpostop_attr, nfsm_srvsattr,
    nfsm_srvwcc, nfsrv_fhtovp,
};
use super::nfs_subs::{NFSSTATS, nfs_false, nfs_true, nfs_xdrneg1};
use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_rndup};
use super::nfsproto::{
    NFS_MAXDATA, NFS_MAXNAMLEN, NFS_MAXPATHLEN, NFSERR_NAMETOL, NFSERR_NOT_SYNC, NFSERR_RETVOID,
    NFSV3ACCESS_DELETE, NFSV3ACCESS_EXECUTE, NFSV3ACCESS_EXTEND, NFSV3ACCESS_LOOKUP,
    NFSV3ACCESS_MODIFY, NFSV3ACCESS_READ, NFSV3WRITE_FILESYNC, NFSV3WRITE_UNSTABLE, NFSX_UNSIGNED,
    NFSX_V2FATTR, NFSX_V2FH, NFSX_V3FATTR, NFSX_V3FH, Nfsfh, Nfsv2Sattr, Nfsv3Time, nfstov_mode,
    nfsx_fattr, nfsx_postopattr, nfsx_postoporfattr, nfsx_preopattr, nfsx_srvfh, nfsx_wccdata,
    nfsx_wccorfattr, nfsx_writeverf,
};
use super::nfssvc::{ND_NFSV3, NfsrvDescript, NfssvcSock, nd_reader, nfs_srvmaxdata};
use super::xdr_subs::{fxdr_nfsv2time, fxdr_nfsv3time, fxdr_unsigned, txdr_unsigned, xdr_bytes};
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::cred::Ucred;
use crate::fs::lookup::ndinit;
use crate::fs::mount::Mount;
use crate::fs::namei::{Componentname, LOCKLEAF, LOOKUP, Nameidata, NiDirp, SAVESTART};
use crate::fs::subr::{vattr_null, vput, vrele};
use crate::fs::vnode::{
    IO_NODELOCKED, IO_SYNC, VA_UTIMES_NULL, VEXEC, VREAD, VTEXT, VWRITE, Vattr, Vnode, Vtype,
};
use crate::fs::vops::{
    Cred, VFS_VPTOFH, vop_access, vop_getattr, vop_read, vop_readlink, vop_setattr, vop_write,
};
use crate::net::mbuf::{
    M_WAIT, MChainBuilder, MINCLSIZE, MT_DATA, Mbuf, MbufBox, m_cat, m_get, m_trailingspace, mclget,
};
use crate::proc::{ProcGlue, Thread};
use crate::sys::errno::Errno;
use crate::sys::mount::{Fhandle, Fid, MNT_RDONLY};
use crate::sys::types::Off;
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
/* </CODE> */
