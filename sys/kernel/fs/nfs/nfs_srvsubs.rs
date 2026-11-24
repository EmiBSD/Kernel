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
//! The NFS server's helpers for the procedures of `nfs_serv.c` (N6.7b2): `nfs_srvsubs.c`'s
//! name lookup from a request (`nfs_namei`), the reply chain's trim (`nfsm_adj`), the reply
//! builders of attributes and weak cache consistency data, the file handle to vnode
//! conversion with its export and reserved port checks (`nfsrv_fhtovp`), the host address
//! comparison (`netaddr_match`) and the `sattr3` decoder; and the server's part of
//! `nfs_subs.c`: the error maps (`nfsrv_errmap`) and the reply's file handle
//! (`nfsm_srvfhtom`). Beside `bsd`'s server until N6.7b4.
//!
//! Upstream: sys/nfs/nfs_srvsubs.c @ 3ce1f3f79392
//! Upstream: sys/nfs/nfs_subs.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_srvsubs.rs@e8249449d905
//! LZ: sys/nfs/nfs_subs.rs@e8249449d905
//!
//! `option NFSSERVER` only: the module is behind the kernel's `nfsserver` feature.
//!
//! ## Deviations
//! - `nfs_namei` and `nfsrv_fhtovp` return `Result<_, i32>`: the C's `int error` is an errno
//!   or an NFS status (`NFSERR_AUTHERR | AUTH_TOOWEAK` names no errno) and goes into
//!   `nd_repstat` as it is. The vnode the C returns through `vpp`, and `*rdonlyp`, are the
//!   `Ok` value; `nfs_namei`'s `retdirp` stays an out parameter, since the C sets it before
//!   the lookup and its callers `vrele` it on every path after, errors included.
//! - `nfs_namei` refuses a name of `MAXPATHLEN` bytes or more (`ENAMETOOLONG`), which the
//!   buffer could not hold with its NUL; the C relies on its callers' `NFS_MAXNAMLEN` bound.
//! - `nfsrv_fhtovp` reads the client's `struct sockaddr_in` out of the address mbuf as a copy;
//!   an address mbuf shorter than a `sockaddr_in`, or none, fails the reserved port check
//!   (`NFSERR_AUTHERR | AUTH_TOOWEAK`) where the C would read past it (LZ's). With no address
//!   at all (the C's NULL `nd_nam`, which `nfssvc_checknam` keeps away from every procedure
//!   but the NULL one) the check comes before `VFS_CHECKEXP`, which the C would hand the NULL.
//! - The export's rights come by value (N6.7b0, D-6: `Vfs::checkexp` answers an
//!   `ExportGrant` copied under the kernel lock's reader), where the C reads the anonymous
//!   credentials through a pointer into the export entry after `VFS_FHTOVP`'s sleeps, in which
//!   a `MNT_DELEXPORT` may free it.
//! - `nfsm_srvwcc` and `nfsm_srvpostop_attr` take the attributes as `Option<&Vattr>`: `None`
//!   is the C's non-zero `before_ret`/`after_ret` (the `VOP_GETATTR` failed).
//! - `nfsm_srvfattr` returns the `nfs_fattr` instead of filling one the caller built.
//! - `nfsm_adj` clamps the null fill to the mbuf's data (the C would write before it for a
//!   `nul` above `m_len`; its callers never ask for that).
//! - `nfsrv_errmap` maps an `err` below 1 to `NFSERR_IO`, where the C indexes the version 2
//!   table at `err - 1` (its callers never pass one).
//!
//! ## Redesign
//! - LZ's helpers took the C's cursor, an mbuf and a raw position in it (`nd_md`, `nd_dpos`),
//!   and built replies through `nfsm_build` views into the mbufs, with three `unsafe` (the
//!   null fill, the address copy, `credanon`'s deref, the last gone at N6.7b0). Here a request
//!   is read through the kernel's `XdrReader` (every length compared before it is a size,
//!   never a read past the chain) and a reply built through its `XdrWriter`; the name's bytes
//!   are checked as they are copied, mbuf by mbuf, as the C does, so a `/` met before the
//!   chain ends is `EACCES` and the end met first `EBADRPC`. The address is copied out of its
//!   mbuf; the null fill writes the mbuf's own data, which the caller owns.

use core::mem::size_of;

use hal::sys::errno::Errno;
use hal::sys::types::{SaFamily, major, minor};

use super::nfs_subs::{nfs_false, nfs_true, txdr_nfsv2time};
use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_padlen};
use super::nfsproto::{
    NFS_FABLKSIZE, NFSERR_ACCES, NFSERR_AUTHERR, NFSERR_BAD_COOKIE, NFSERR_BADHANDLE,
    NFSERR_BADTYPE, NFSERR_DQUOT, NFSERR_EXIST, NFSERR_FBIG, NFSERR_INVAL, NFSERR_IO, NFSERR_ISDIR,
    NFSERR_MLINK, NFSERR_NAMETOL, NFSERR_NODEV, NFSERR_NOENT, NFSERR_NOSPC, NFSERR_NOT_SYNC,
    NFSERR_NOTDIR, NFSERR_NOTEMPTY, NFSERR_NOTSUPP, NFSERR_NXIO, NFSERR_PERM, NFSERR_ROFS,
    NFSERR_SERVERFAULT, NFSERR_STALE, NFSERR_TOOSMALL, NFSERR_XDEV, NFSPROC_COMMIT,
    NFSV3SATTRTIME_TOCLIENT, NFSV3SATTRTIME_TOSERVER, NFSX_UNSIGNED, NFSX_V2FH, NFSX_V3FATTR,
    NFSX_V3FH, NfsFattr, Nfsfh, Nfsuint64, Nfsv3Spec, Nfsv3Time, nfstov_mode, vtonfsv2_mode,
    vtonfsv2_type, vtonfsv3_mode, vtonfsv3_type,
};
use super::nfssvc::{ND_NFSV3, NfsrvDescript, NfssvcSock};
use super::rpcv2::AUTH_TOOWEAK;
use super::xdr_subs::{
    fxdr_nfsv3time, txdr_hyper, txdr_nfsv3time, txdr_unsigned, xdr_bytes, xdr_get,
};
use crate::assert::kpanic;
use crate::cred::UcredCell;
use crate::fs::lookup::vfs_lookup;
use crate::fs::namei::{
    HASBUF, ISSYMLINK, LOCKPARENT, MAXPATHLEN, NOCROSSMOUNT, Nameidata, RDONLY, SAVENAME,
    SAVESTART, pathbuf_get,
};
use crate::fs::subr::{vfs_getvfs, vput, vref, vrele};
use crate::fs::vnode::{VA_UTIMES_CHANGE, VA_UTIMES_NULL, Vattr, Vnode, Vtype};
use crate::fs::vops::vop_unlock;
use crate::net::mbuf::Mbuf;
use crate::proc::{ProcGlue, Thread};
use crate::sys::mount::{Fhandle, MNT_EXPORTANON, MNT_EXRDONLY};
use crate::sys::netinet_in::{IPPORT_RESERVED, SockaddrIn};
use crate::sys::socket::{AF_INET, SOCK_STREAM};
use crate::sys::syslimits::NGROUPS_MAX;
use crate::tc::getnanotime;

/// `nfsrv_v2errmap`: maps errno values to NFS error numbers. Use `NFSERR_IO` as the catch
/// all for ones not specifically defined in RFC 1094. Everything after the table maps to
/// `NFSERR_IO`, so far.
pub const NFSRV_V2ERRMAP: [u8; 70] = {
    const P: u8 = NFSERR_PERM as u8;
    const NOENT: u8 = NFSERR_NOENT as u8;
    const IO: u8 = NFSERR_IO as u8;
    const NXIO: u8 = NFSERR_NXIO as u8;
    const ACCES: u8 = NFSERR_ACCES as u8;
    const EXIST: u8 = NFSERR_EXIST as u8;
    const NODEV: u8 = NFSERR_NODEV as u8;
    const NOTDIR: u8 = NFSERR_NOTDIR as u8;
    const ISDIR: u8 = NFSERR_ISDIR as u8;
    const FBIG: u8 = NFSERR_FBIG as u8;
    const NOSPC: u8 = NFSERR_NOSPC as u8;
    const ROFS: u8 = NFSERR_ROFS as u8;
    const NAMETOL: u8 = NFSERR_NAMETOL as u8;
    const NOTEMPTY: u8 = NFSERR_NOTEMPTY as u8;
    const DQUOT: u8 = NFSERR_DQUOT as u8;
    const STALE: u8 = NFSERR_STALE as u8;
    [
        P, NOENT, IO, IO, IO, NXIO, IO, IO, IO, IO, //
        IO, IO, ACCES, IO, IO, IO, EXIST, IO, NODEV, NOTDIR, //
        ISDIR, IO, IO, IO, IO, IO, FBIG, NOSPC, IO, ROFS, //
        IO, IO, IO, IO, IO, IO, IO, IO, IO, IO, //
        IO, IO, IO, IO, IO, IO, IO, IO, IO, IO, //
        IO, IO, IO, IO, IO, IO, IO, IO, IO, IO, //
        IO, IO, NAMETOL, IO, IO, NOTEMPTY, IO, IO, DQUOT, STALE,
    ]
};

// Maps errno values to NFS error numbers. Although it is not obvious whether or not NFS
// clients really care if a returned error value is in the specified list for the procedure,
// the safest thing to do is filter them appropriately. For version 2, the X/Open XNFS
// document is the only specification that defines error values for each RPC (the RFC simply
// lists all possible error values for all RPCs), so I have decided to not do this for
// version 2. The first entry is the default error return and the rest are the valid errors
// for that RPC in increasing numeric order.

/// The version 3 error numbers, as the tables' `short`s.
const E_PERM: i16 = NFSERR_PERM as i16;
/// `NFSERR_NOENT`.
const E_NOENT: i16 = NFSERR_NOENT as i16;
/// `NFSERR_IO`.
const E_IO: i16 = NFSERR_IO as i16;
/// `NFSERR_NXIO`.
const E_NXIO: i16 = NFSERR_NXIO as i16;
/// `NFSERR_ACCES`.
const E_ACCES: i16 = NFSERR_ACCES as i16;
/// `NFSERR_EXIST`.
const E_EXIST: i16 = NFSERR_EXIST as i16;
/// `NFSERR_XDEV`.
const E_XDEV: i16 = NFSERR_XDEV as i16;
/// `NFSERR_NOTDIR`.
const E_NOTDIR: i16 = NFSERR_NOTDIR as i16;
/// `NFSERR_ISDIR`.
const E_ISDIR: i16 = NFSERR_ISDIR as i16;
/// `NFSERR_INVAL`.
const E_INVAL: i16 = NFSERR_INVAL as i16;
/// `NFSERR_FBIG`.
const E_FBIG: i16 = NFSERR_FBIG as i16;
/// `NFSERR_NOSPC`.
const E_NOSPC: i16 = NFSERR_NOSPC as i16;
/// `NFSERR_ROFS`.
const E_ROFS: i16 = NFSERR_ROFS as i16;
/// `NFSERR_MLINK`.
const E_MLINK: i16 = NFSERR_MLINK as i16;
/// `NFSERR_NAMETOL`.
const E_NAMETOL: i16 = NFSERR_NAMETOL as i16;
/// `NFSERR_NOTEMPTY`.
const E_NOTEMPTY: i16 = NFSERR_NOTEMPTY as i16;
/// `NFSERR_DQUOT`.
const E_DQUOT: i16 = NFSERR_DQUOT as i16;
/// `NFSERR_STALE`.
const E_STALE: i16 = NFSERR_STALE as i16;
/// `NFSERR_BADHANDLE`.
const E_BADHANDLE: i16 = NFSERR_BADHANDLE as i16;
/// `NFSERR_NOT_SYNC`.
const E_NOT_SYNC: i16 = NFSERR_NOT_SYNC as i16;
/// `NFSERR_BAD_COOKIE`.
const E_BAD_COOKIE: i16 = NFSERR_BAD_COOKIE as i16;
/// `NFSERR_NOTSUPP`.
const E_NOTSUPP: i16 = NFSERR_NOTSUPP as i16;
/// `NFSERR_TOOSMALL`.
const E_TOOSMALL: i16 = NFSERR_TOOSMALL as i16;
/// `NFSERR_SERVERFAULT`.
const E_SERVERFAULT: i16 = NFSERR_SERVERFAULT as i16;
/// `NFSERR_BADTYPE`.
const E_BADTYPE: i16 = NFSERR_BADTYPE as i16;

/// `nfsv3err_null`.
const NFSV3ERR_NULL: &[i16] = &[0, 0];
/// `nfsv3err_getattr`.
const NFSV3ERR_GETATTR: &[i16] = &[E_IO, E_IO, E_STALE, E_BADHANDLE, E_SERVERFAULT, 0];
/// `nfsv3err_setattr`.
const NFSV3ERR_SETATTR: &[i16] = &[
    E_IO,
    E_PERM,
    E_IO,
    E_ACCES,
    E_INVAL,
    E_NOSPC,
    E_ROFS,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_NOT_SYNC,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_lookup`.
const NFSV3ERR_LOOKUP: &[i16] = &[
    E_IO,
    E_NOENT,
    E_IO,
    E_ACCES,
    E_NOTDIR,
    E_NAMETOL,
    E_STALE,
    E_BADHANDLE,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_access`.
const NFSV3ERR_ACCESS: &[i16] = &[E_IO, E_IO, E_STALE, E_BADHANDLE, E_SERVERFAULT, 0];
/// `nfsv3err_readlink`.
const NFSV3ERR_READLINK: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_INVAL,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_read`.
const NFSV3ERR_READ: &[i16] = &[
    E_IO,
    E_IO,
    E_NXIO,
    E_ACCES,
    E_INVAL,
    E_STALE,
    E_BADHANDLE,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_write`.
const NFSV3ERR_WRITE: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_INVAL,
    E_FBIG,
    E_NOSPC,
    E_ROFS,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_create`.
const NFSV3ERR_CREATE: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_EXIST,
    E_NOTDIR,
    E_NOSPC,
    E_ROFS,
    E_NAMETOL,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_mkdir`.
const NFSV3ERR_MKDIR: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_EXIST,
    E_NOTDIR,
    E_NOSPC,
    E_ROFS,
    E_NAMETOL,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_symlink`.
const NFSV3ERR_SYMLINK: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_EXIST,
    E_NOTDIR,
    E_NOSPC,
    E_ROFS,
    E_NAMETOL,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_mknod`.
const NFSV3ERR_MKNOD: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_EXIST,
    E_NOTDIR,
    E_NOSPC,
    E_ROFS,
    E_NAMETOL,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    E_BADTYPE,
    0,
];
/// `nfsv3err_remove`.
const NFSV3ERR_REMOVE: &[i16] = &[
    E_IO,
    E_NOENT,
    E_IO,
    E_ACCES,
    E_NOTDIR,
    E_ROFS,
    E_NAMETOL,
    E_STALE,
    E_BADHANDLE,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_rmdir`.
const NFSV3ERR_RMDIR: &[i16] = &[
    E_IO,
    E_NOENT,
    E_IO,
    E_ACCES,
    E_EXIST,
    E_NOTDIR,
    E_INVAL,
    E_ROFS,
    E_NAMETOL,
    E_NOTEMPTY,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_rename`.
const NFSV3ERR_RENAME: &[i16] = &[
    E_IO,
    E_NOENT,
    E_IO,
    E_ACCES,
    E_EXIST,
    E_XDEV,
    E_NOTDIR,
    E_ISDIR,
    E_INVAL,
    E_NOSPC,
    E_ROFS,
    E_MLINK,
    E_NAMETOL,
    E_NOTEMPTY,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_link`.
const NFSV3ERR_LINK: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_EXIST,
    E_XDEV,
    E_NOTDIR,
    E_INVAL,
    E_NOSPC,
    E_ROFS,
    E_MLINK,
    E_NAMETOL,
    E_DQUOT,
    E_STALE,
    E_BADHANDLE,
    E_NOTSUPP,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_readdir`.
const NFSV3ERR_READDIR: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_NOTDIR,
    E_STALE,
    E_BADHANDLE,
    E_BAD_COOKIE,
    E_TOOSMALL,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_readdirplus`.
const NFSV3ERR_READDIRPLUS: &[i16] = &[
    E_IO,
    E_IO,
    E_ACCES,
    E_NOTDIR,
    E_STALE,
    E_BADHANDLE,
    E_BAD_COOKIE,
    E_NOTSUPP,
    E_TOOSMALL,
    E_SERVERFAULT,
    0,
];
/// `nfsv3err_fsstat`.
const NFSV3ERR_FSSTAT: &[i16] = &[E_IO, E_IO, E_STALE, E_BADHANDLE, E_SERVERFAULT, 0];
/// `nfsv3err_fsinfo`.
const NFSV3ERR_FSINFO: &[i16] = &[E_STALE, E_STALE, E_BADHANDLE, E_SERVERFAULT, 0];
/// `nfsv3err_pathconf`.
const NFSV3ERR_PATHCONF: &[i16] = &[E_STALE, E_STALE, E_BADHANDLE, E_SERVERFAULT, 0];
/// `nfsv3err_commit`.
const NFSV3ERR_COMMIT: &[i16] = &[E_IO, E_IO, E_STALE, E_BADHANDLE, E_SERVERFAULT, 0];

/// `nfsrv_v3errmap`: the version 3 error list of each procedure, `NFSPROC_NULL` to
/// `NFSPROC_COMMIT`.
pub const NFSRV_V3ERRMAP: [&[i16]; NFSPROC_COMMIT + 1] = [
    NFSV3ERR_NULL,
    NFSV3ERR_GETATTR,
    NFSV3ERR_SETATTR,
    NFSV3ERR_LOOKUP,
    NFSV3ERR_ACCESS,
    NFSV3ERR_READLINK,
    NFSV3ERR_READ,
    NFSV3ERR_WRITE,
    NFSV3ERR_CREATE,
    NFSV3ERR_MKDIR,
    NFSV3ERR_SYMLINK,
    NFSV3ERR_MKNOD,
    NFSV3ERR_REMOVE,
    NFSV3ERR_RMDIR,
    NFSV3ERR_RENAME,
    NFSV3ERR_LINK,
    NFSV3ERR_READDIR,
    NFSV3ERR_READDIRPLUS,
    NFSV3ERR_FSSTAT,
    NFSV3ERR_FSINFO,
    NFSV3ERR_PATHCONF,
    NFSV3ERR_COMMIT,
];

/// `nfs_namei(ndp, fhp, len, slp, nam, mdp, dposp, retdirp, p)`: sets up the `nameidata` for
/// a `vfs_lookup` of the `len`-byte name at the request's cursor `r`, relative to the
/// directory `fhp` names, and does it.
///
/// The name is copied into a pathname buffer (a NUL or a `/` in it is `EACCES`), the cursor
/// moves past it and its XDR padding, and the directory is found with [`nfsrv_fhtovp`]
/// (`ENOTDIR` unless it is one). The directory goes to `*retdirp` (the caller `vrele`s it,
/// even when this fails). The lookup does not cross mount points, and is read-only on a
/// read-only export. A symbolic link at the end is `EINVAL`. The buffer is given back, except
/// when the caller asked for `SAVENAME` or `SAVESTART`: then it stays in `cn_pnbuf` with
/// `HASBUF` set.
///
/// `cred` is the request's credentials (`nd_cr`, which `ndp.ni_cnd.cn_cred` names), mapped
/// for the export by `nfsrv_fhtovp`. The error is the C's `int`: an errno or an NFS status.
#[allow(clippy::too_many_arguments, reason = "the C's argument list")]
pub fn nfs_namei<'a, G: ProcGlue>(
    ndp: &mut Nameidata<'a, G>,
    fhp: &Fhandle,
    len: usize,
    slp: &NfssvcSock<G>,
    nam: Option<&Mbuf>,
    r: &mut XdrReader<'_>,
    retdirp: &mut Option<&'static Vnode<G>>,
    p: &'a Thread<G>,
    cred: &UcredCell,
) -> Result<(), i32> {
    *retdirp = None;
    let Some(buf) = pathbuf_get::<G>() else {
        return Err(Errno::ENOMEM.as_i32());
    };
    ndp.ni_cnd.set_pnbuf(buf);

    let res = namei_lookup(ndp, fhp, len, slp, nam, r, retdirp, p, cred);

    // Check for saved name request
    if res.is_ok() && ndp.ni_cnd.cn_flags & (SAVENAME | SAVESTART) != 0 {
        ndp.ni_cnd.cn_flags |= HASBUF;
        return Ok(());
    }
    // out:
    drop(ndp.ni_cnd.take_pnbuf());
    res
}

/// The part of `nfs_namei` before its `out:` label: everything that can fail, the buffer
/// still in `cn_pnbuf`.
#[allow(clippy::too_many_arguments, reason = "nfs_namei's, split")]
fn namei_lookup<'a, G: ProcGlue>(
    ndp: &mut Nameidata<'a, G>,
    fhp: &Fhandle,
    len: usize,
    slp: &NfssvcSock<G>,
    nam: Option<&Mbuf>,
    r: &mut XdrReader<'_>,
    retdirp: &mut Option<&'static Vnode<G>>,
    p: &'a Thread<G>,
    cred: &UcredCell,
) -> Result<(), i32> {
    if len >= MAXPATHLEN {
        return Err(Errno::ENAMETOOLONG.as_i32());
    }
    // Copy the name from the mbuf list to ndp->ni_pnbuf and set the various ndp fields
    // appropriately: each byte checked as it is copied, mbuf by mbuf.
    let Some(pn) = ndp.ni_cnd.pnbuf_mut() else {
        return Err(Errno::ENOMEM.as_i32());
    };
    let mut n = 0;
    while n < len {
        let piece = r.next_piece(len - n).map_err(Errno::as_i32)?;
        for &c in piece {
            if c == 0 || c == b'/' {
                return Err(Errno::EACCES.as_i32());
            }
            pn[n] = c;
            n += 1;
        }
    }
    pn[len] = 0;
    let pad = nfsm_padlen(len);
    if pad > 0 {
        r.adv(pad).map_err(Errno::as_i32)?;
    }
    ndp.ni_pathlen = len;
    ndp.ni_cnd.set_name_at(0, 0);

    // Extract and set starting directory.
    let (dp, rdonly) = nfsrv_fhtovp(fhp, false, cred, slp, nam)?;
    if dp.vtype() != Vtype::VDIR {
        vrele(dp);
        return Err(Errno::ENOTDIR.as_i32());
    }
    vref(dp);
    *retdirp = Some(dp);
    ndp.ni_startdir = Some(dp);
    if rdonly {
        ndp.ni_cnd.cn_flags |= NOCROSSMOUNT | RDONLY;
    } else {
        ndp.ni_cnd.cn_flags |= NOCROSSMOUNT;
    }

    // And call lookup() to do the real work
    ndp.ni_cnd.cn_proc = p;
    vfs_lookup(ndp).map_err(Errno::as_i32)?;

    // Check for encountering a symbolic link
    if ndp.ni_cnd.cn_flags & ISSYMLINK != 0 {
        if let Some(dvp) = ndp.ni_dvp {
            if ndp.ni_cnd.cn_flags & LOCKPARENT != 0 && ndp.ni_pathlen == 1 {
                vput(dvp);
            } else {
                vrele(dvp);
            }
        }
        if let Some(vp) = ndp.ni_vp.take() {
            vput(vp);
        }
        return Err(Errno::EINVAL.as_i32());
    }
    Ok(())
}

/// `nfsm_adj(mp, len, nul)`: a fiddled version of `m_adj()` that ensures null fill to a long
/// boundary and only trims off the back end: `len` bytes off the end of the chain `mp`, then
/// the last `nul` bytes left zeroed.
pub fn nfsm_adj(mp: &mut Mbuf, len: i32, nul: i32) {
    // Trim from tail. Scan the mbuf chain, calculating its length and finding the last mbuf.
    // If the adjustment only affects this mbuf, then just adjust and return. Otherwise,
    // rescan and truncate after the remaining size.
    let mut count: i32 = 0;
    let mut nbufs = 0usize;
    let mut m: &Mbuf = mp;
    let last_len = loop {
        let l = m.m_len().get() as i32;
        count = count.wrapping_add(l);
        match m.m_next().get() {
            Some(next) => {
                m = next;
                nbufs += 1;
            }
            None => break l,
        }
    };
    if last_len > len {
        let mut last = &mut *mp;
        for _ in 0..nbufs {
            last = match last.next_mut() {
                Some(next) => next,
                None => kpanic(format_args!("nfsm_adj: the chain changed")),
            };
        }
        last.m_len().set((last_len - len) as u32);
        nul_fill(last, nul);
        return;
    }
    count = count.wrapping_sub(len);
    if count < 0 {
        count = 0;
    }
    // Correct length for chain is "count". Find the mbuf with last data, adjust its length,
    // and toss data from remaining mbufs on chain.
    let mut cur = &mut *mp;
    loop {
        let l = cur.m_len().get() as i32;
        if l >= count {
            cur.m_len().set(count as u32);
            nul_fill(cur, nul);
            break;
        }
        count -= l;
        match cur.next_mut() {
            Some(next) => cur = next,
            None => return,
        }
    }
    let mut rest = cur.m_next().get();
    while let Some(m) = rest {
        m.m_len().set(0);
        rest = m.m_next().get();
    }
}

/// The null fill of `nfsm_adj`: the last `nul` bytes of `m`'s data, clamped to the data.
fn nul_fill(m: &mut Mbuf, nul: i32) {
    let Ok(nul) = usize::try_from(nul) else {
        return;
    };
    if let Some(d) = m.data_mut() {
        let n = nul.min(d.len());
        let at = d.len() - n;
        d[at..].fill(0);
    }
}

/// `nfsm_srvwcc(nfsd, before_ret, before_vap, after_ret, after_vap, mb)`: appends the NFSv3
/// weak cache consistency data (`wcc_data`) to the reply `mb`: the pre-operation attributes
/// (size, mtime, ctime; `None` when they could not be had) and the post-operation ones.
pub fn nfsm_srvwcc(
    nfsd: &NfsrvDescript,
    before: Option<&Vattr>,
    after: Option<&Vattr>,
    mb: &mut XdrWriter,
) {
    match before {
        None => mb.word(nfs_false),
        Some(b) => {
            let tl = mb.build::<{ 7 * NFSX_UNSIGNED }>();
            tl[..4].copy_from_slice(&nfs_true.to_ne_bytes());
            tl[4..12].copy_from_slice(xdr_bytes(&txdr_hyper(b.va_size)));
            tl[12..20].copy_from_slice(xdr_bytes(&txdr_nfsv3time(&b.va_mtime)));
            tl[20..28].copy_from_slice(xdr_bytes(&txdr_nfsv3time(&b.va_ctime)));
        }
    }
    nfsm_srvpostop_attr(nfsd, after, mb);
}

/// `nfsm_srvpostop_attr(nfsd, after_ret, after_vap, mb)`: appends an NFSv3 `post_op_attr`:
/// the attributes when `after` has them (`None` is the C's non-zero `after_ret`).
pub fn nfsm_srvpostop_attr(nfsd: &NfsrvDescript, after: Option<&Vattr>, mb: &mut XdrWriter) {
    match after {
        None => mb.word(nfs_false),
        Some(a) => {
            let tl = mb.build::<{ NFSX_UNSIGNED + NFSX_V3FATTR }>();
            tl[..4].copy_from_slice(&nfs_true.to_ne_bytes());
            tl[4..].copy_from_slice(&xdr_bytes(&nfsm_srvfattr(nfsd, a))[..NFSX_V3FATTR]);
        }
    }
}

/// `nfsm_srvfattr(nfsd, vap, fp)`: the `nfs_fattr` of `vap` (version 3 when the request is),
/// in wire order; version 2 fills the words of its shorter structure only.
pub fn nfsm_srvfattr(nfsd: &NfsrvDescript, vap: &Vattr) -> NfsFattr {
    let mut fp = NfsFattr {
        fa_nlink: txdr_unsigned(vap.va_nlink),
        fa_uid: txdr_unsigned(vap.va_uid),
        fa_gid: txdr_unsigned(vap.va_gid),
        ..NfsFattr::default()
    };
    if nfsd.nd_flag & ND_NFSV3 != 0 {
        fp.fa_type = vtonfsv3_type(vap.va_type);
        fp.fa_mode = vtonfsv3_mode(vap.va_mode);
        fp.set_fa3_size(Nfsuint64::from_words(txdr_hyper(vap.va_size)));
        fp.set_fa3_used(Nfsuint64::from_words(txdr_hyper(vap.va_bytes)));
        fp.set_fa3_rdev(Nfsv3Spec {
            specdata1: txdr_unsigned(major(vap.va_rdev)),
            specdata2: txdr_unsigned(minor(vap.va_rdev)),
        });
        // `txdr_unsigned(vap->va_fsid)`: the C's `long` truncated to the word.
        fp.set_fa3_fsid(Nfsuint64::from_words([
            0,
            txdr_unsigned(vap.va_fsid as u32),
        ]));
        fp.set_fa3_fileid(Nfsuint64::from_words(txdr_hyper(vap.va_fileid)));
        fp.set_fa3_atime(txdr_nfsv3time(&vap.va_atime));
        fp.set_fa3_mtime(txdr_nfsv3time(&vap.va_mtime));
        fp.set_fa3_ctime(txdr_nfsv3time(&vap.va_ctime));
    } else {
        fp.fa_type = vtonfsv2_type(vap.va_type);
        fp.fa_mode = vtonfsv2_mode(vap.va_type, vap.va_mode);
        // Version 2's words are the C's truncations of the 64-bit members.
        fp.set_fa2_size(txdr_unsigned(vap.va_size as u32));
        fp.set_fa2_blocksize(txdr_unsigned(vap.va_blocksize as u32));
        fp.set_fa2_rdev(if vap.va_type == Vtype::VFIFO {
            0xffff_ffff
        } else {
            txdr_unsigned(vap.va_rdev as u32)
        });
        fp.set_fa2_blocks(txdr_unsigned((vap.va_bytes / NFS_FABLKSIZE) as u32));
        fp.set_fa2_fsid(txdr_unsigned(vap.va_fsid as u32));
        fp.set_fa2_fileid(txdr_unsigned(vap.va_fileid as u32));
        fp.set_fa2_atime(txdr_nfsv2time(&vap.va_atime));
        fp.set_fa2_mtime(txdr_nfsv2time(&vap.va_mtime));
        fp.set_fa2_ctime(txdr_nfsv2time(&vap.va_ctime));
    }
    fp
}

/// `nfsrv_fhtovp(fhp, lockflag, vpp, cred, slp, nam, rdonlyp)`: the vnode a file handle
/// names (locked when `lockflag`), and whether its export is read-only:
/// - the file system id looked up in the mount list (`ESTALE` when it is not there);
/// - the export rights from the file system's `vfs_checkexp` (by value), the vnode from
///   `VFS_FHTOVP()`;
/// - a client not on a reserved port refused (`NFSERR_AUTHERR | AUTH_TOOWEAK`), and on a
///   stream socket the ftp-data port 20 too;
/// - if `cred`'s uid is 0 or the export maps everyone (`MNT_EXPORTANON`), `cred`'s uid, gid
///   and groups set to the export's anonymous credentials;
/// - the vnode unlocked unless `lockflag`.
///
/// The error is the C's `int`: an errno or an NFS status.
pub fn nfsrv_fhtovp<G: ProcGlue>(
    fhp: &Fhandle,
    lockflag: bool,
    cred: &UcredCell,
    slp: &NfssvcSock<G>,
    nam: Option<&Mbuf>,
) -> Result<(&'static Vnode<G>, bool), i32> {
    let tooweak = NFSERR_AUTHERR | AUTH_TOOWEAK as i32;
    let Some(mp) = vfs_getvfs::<G>(&fhp.fh_fsid) else {
        return Err(Errno::ESTALE.as_i32());
    };
    let Some(nam) = nam else {
        return Err(tooweak);
    };
    // The client's rights, by value (D-6): nothing below reads the export entry, which a
    // `MNT_DELEXPORT` may free while `VFS_FHTOVP` sleeps.
    let grant = mp.op().checkexp(mp, nam).map_err(Errno::as_i32)?;
    let vp = mp.op().fhtovp(mp, &fhp.fh_fid).map_err(Errno::as_i32)?;

    match sockaddr_in_of(nam) {
        Some(saddr) => {
            let port = u16::from_be(saddr.sin_port);
            if saddr.sin_family == AF_INET
                && (i32::from(port) >= IPPORT_RESERVED
                    || (slp.sotype() == SOCK_STREAM && port == 20))
            {
                vput(vp);
                return Err(tooweak);
            }
        }
        None => {
            // Shorter than a sockaddr_in: not an address of a client we can vouch for.
            vput(vp);
            return Err(tooweak);
        }
    }

    // Check/setup credentials.
    if cred.cr_uid.get() == 0 || grant.exflags & MNT_EXPORTANON != 0 {
        cred.set_uid(grant.uid);
        cred.set_gid(grant.gid);
        let n = usize::try_from(grant.ngroups).unwrap_or(0).min(NGROUPS_MAX);
        for (i, g) in grant.groups[..n].iter().enumerate() {
            cred.set_group(i, *g);
        }
        // `i` after the loop: at most NGROUPS_MAX (16), an `i16`.
        cred.set_ngroups(n as i16);
    }
    let rdonly = grant.exflags & MNT_EXRDONLY != 0;
    if !lockflag {
        let _ = vop_unlock(vp);
    }

    Ok((vp, rdonly))
}

/// The `struct sockaddr_in` at the start of the address mbuf `nam` (`mtod(nam, struct
/// sockaddr_in *)`), copied out; `None` when the mbuf is shorter.
pub fn sockaddr_in_of(nam: &Mbuf) -> Option<SockaddrIn> {
    let d = nam.data();
    if d.len() < size_of::<SockaddrIn>() {
        return None;
    }
    Some(xdr_get::<SockaddrIn>(d))
}

/// `netaddr_match(family, haddr, nam)`: whether two net addresses, by family, are the same
/// host; false if there is any doubt. The `AF_INET` family is handled as a special case so
/// that address mbufs need not be saved to store a `struct in_addr`, which is only 4 bytes:
/// `inetaddr` is `haddr->had_inetaddr` (the cache's `rc_inetaddr`), the only member read.
pub fn netaddr_match(family: SaFamily, inetaddr: u32, nam: &Mbuf) -> bool {
    match family {
        AF_INET => sockaddr_in_of(nam)
            .is_some_and(|sin| sin.sin_family == AF_INET && sin.sin_addr.s_addr == inetaddr),
        _ => false,
    }
}

/// `nfsm_srvsattr(nfsd, va)`: decodes an NFSv3 `sattr3` at the request's cursor into `va`:
/// the mode, uid, gid and size when the request sets them, then the access and modification
/// times (the client's value, the server's, or left alone). `EBADRPC` when the request ends
/// first.
pub fn nfsm_srvsattr(r: &mut XdrReader<'_>, va: &mut Vattr) -> Result<(), Errno> {
    if r.word()? == nfs_true {
        va.va_mode = nfstov_mode(r.word()?);
    }
    if r.word()? == nfs_true {
        va.va_uid = r.u32()?;
    }
    if r.word()? == nfs_true {
        va.va_gid = r.u32()?;
    }
    if r.word()? == nfs_true {
        va.va_size = r.hyper()?;
    }
    srvsattr_time(r, va, true)?;
    srvsattr_time(r, va, false)
}

/// One of `sattr3`'s two time unions: the access time (`atime`) or the modification time.
fn srvsattr_time(r: &mut XdrReader<'_>, va: &mut Vattr, atime: bool) -> Result<(), Errno> {
    let how = r.i32()?;
    if how == NFSV3SATTRTIME_TOCLIENT as i32 {
        va.va_vaflags |= VA_UTIMES_CHANGE;
        va.va_vaflags &= !VA_UTIMES_NULL;
        let t: Nfsv3Time = r.pod()?;
        let ts = fxdr_nfsv3time(&t);
        if atime {
            va.va_atime = ts;
        } else {
            va.va_mtime = ts;
        }
    } else if how == NFSV3SATTRTIME_TOSERVER as i32 {
        va.va_vaflags |= VA_UTIMES_CHANGE;
        let ts = getnanotime();
        if atime {
            va.va_atime = ts;
        } else {
            va.va_mtime = ts;
        }
    }
    Ok(())
}

/// `nfsrv_errmap(nd, err)`: maps an errno to an NFS error number. For version 3 also
/// filters out error numbers not specified for the associated procedure.
pub fn nfsrv_errmap(nd: &NfsrvDescript, err: i32) -> i32 {
    if nd.nd_flag & ND_NFSV3 != 0 {
        let Some(errs) = NFSRV_V3ERRMAP.get(nd.nd_procnum) else {
            return err & 0xffff;
        };
        for &e in &errs[1..] {
            if e == 0 {
                break;
            }
            let e = i32::from(e);
            if e == err {
                return err;
            } else if e > err {
                break;
            }
        }
        return i32::from(errs[0]);
    }
    match usize::try_from(err) {
        Ok(e) if e >= 1 && e <= NFSRV_V2ERRMAP.len() => i32::from(NFSRV_V2ERRMAP[e - 1]),
        _ => NFSERR_IO,
    }
}

/// `nfsm_srvfhtom(mp, f, v3)`: the server's file handle (the `nfsfh_t` it lives in) appended
/// to the reply: version 3's `NFSX_V3FH` bytes as an XDR string, version 2's fixed
/// `NFSX_V2FH` bytes.
pub fn nfsm_srvfhtom(mb: &mut XdrWriter, f: &Nfsfh, v3: bool) {
    if v3 {
        mb.strtombuf(&f.fh_bytes[..NFSX_V3FH]);
    } else {
        mb.buftombuf(&f.fh_bytes[..NFSX_V2FH]);
    }
}

const _: () = {
    // `nfsrv_v3errmap[]` covers every procedure `nfsrv_errmap` indexes it with.
    assert!(NFSRV_V3ERRMAP.len() == NFSPROC_COMMIT + 1);
    assert!(NFSX_V3FATTR <= size_of::<NfsFattr>());
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the server's helpers: the error maps (the C's tables), `nfsm_adj`, the
    // reply builders' wire layouts (version 2 and 3), `nfsm_srvsattr` over a request split
    // across mbufs and cut short, `netaddr_match`, the file handle's two forms. The export
    // checks of `nfsrv_fhtovp` and `nfs_namei` are tested over a real exported file system in
    // `nfssvc`'s tests.

    use std::vec::Vec;

    use super::*;
    use crate::clock::Timespec;
    use crate::fs::nfs::nfsm_subs::{test_bytes, test_chain, test_lens};
    use crate::net::mbuf::{M_WAIT, MT_DATA, m_get};

    fn writer() -> XdrWriter {
        let m = m_get(M_WAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(0);
        XdrWriter::with_first(m)
    }

    fn v3() -> NfsrvDescript {
        NfsrvDescript {
            nd_flag: ND_NFSV3,
            ..NfsrvDescript::default()
        }
    }

    fn be(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_be_bytes()).collect()
    }

    fn vattr() -> Vattr {
        let mut va = Vattr::default();
        va.va_type = Vtype::VREG;
        va.va_mode = 0o644;
        va.va_nlink = 2;
        va.va_uid = 1000;
        va.va_gid = 20;
        va.va_size = 0x1_0000_0002;
        va.va_blocksize = 8192;
        va.va_rdev = 0x0102_0304;
        va.va_bytes = 4096;
        va.va_fsid = 7;
        va.va_fileid = 0x2_0000_0009;
        va.va_atime = Timespec {
            tv_sec: 10,
            tv_nsec: 1000,
        };
        va.va_mtime = Timespec {
            tv_sec: 11,
            tv_nsec: 2000,
        };
        va.va_ctime = Timespec {
            tv_sec: 12,
            tv_nsec: 3000,
        };
        va
    }

    #[test]
    fn the_version_2_map_is_the_c_table() {
        let mut nd = NfsrvDescript::default();
        let cases: &[(i32, i32)] = &[
            (Errno::EPERM.as_i32(), NFSERR_PERM),
            (Errno::ENOENT.as_i32(), NFSERR_NOENT),
            (Errno::EACCES.as_i32(), NFSERR_ACCES),
            (Errno::ENOTDIR.as_i32(), NFSERR_NOTDIR),
            (Errno::EROFS.as_i32(), NFSERR_ROFS),
            (Errno::ENAMETOOLONG.as_i32(), NFSERR_NAMETOL),
            (Errno::ENOTEMPTY.as_i32(), NFSERR_NOTEMPTY),
            (Errno::EDQUOT.as_i32(), NFSERR_DQUOT),
            (Errno::ESTALE.as_i32(), NFSERR_STALE),
            (Errno::EINTR.as_i32(), NFSERR_IO),
            (71, NFSERR_IO),
            (0, NFSERR_IO),
            (-1, NFSERR_IO),
        ];
        for &(e, want) in cases {
            nd.nd_procnum = 1;
            assert_eq!(nfsrv_errmap(&nd, e), want, "errno {e}");
        }
    }

    #[test]
    fn the_version_3_map_keeps_a_procedures_own_errors_only() {
        let mut nd = v3();
        nd.nd_procnum = 3; // LOOKUP
        assert_eq!(nfsrv_errmap(&nd, NFSERR_NOENT), NFSERR_NOENT);
        assert_eq!(nfsrv_errmap(&nd, NFSERR_NOTDIR), NFSERR_NOTDIR);
        // ROFS is not LOOKUP's: the default.
        assert_eq!(nfsrv_errmap(&nd, NFSERR_ROFS), NFSERR_IO);
        nd.nd_procnum = 19; // FSINFO's default is STALE
        assert_eq!(nfsrv_errmap(&nd, NFSERR_IO), NFSERR_STALE);
        nd.nd_procnum = 7; // WRITE
        assert_eq!(nfsrv_errmap(&nd, NFSERR_ROFS), NFSERR_ROFS);
        assert_eq!(nfsrv_errmap(&nd, NFSERR_NOENT), NFSERR_IO);
        // Past NFSPROC_COMMIT (NOOP): the low half of the number.
        nd.nd_procnum = NFSPROC_COMMIT + 1;
        assert_eq!(
            nfsrv_errmap(&nd, 0x1_0000 | NFSERR_BADHANDLE),
            NFSERR_BADHANDLE
        );
        // Every list ends with 0 and its errors rise (the loop's early break).
        for (p, l) in NFSRV_V3ERRMAP.iter().enumerate() {
            assert_eq!(l.last(), Some(&0), "proc {p}");
            let body = &l[1..l.len() - 1];
            assert!(body.windows(2).all(|w| w[0] < w[1]), "proc {p}");
        }
    }

    #[test]
    fn nfsm_adj_trims_the_tail_and_zero_fills() {
        let _g = crate::net::mbuf::tests::setup();
        // Within the last mbuf.
        let mut m = test_chain(&[&[1, 2, 3, 4], &[5, 6, 7, 8, 9, 10, 11, 12]]);
        nfsm_adj(&mut m, 3, 2);
        assert_eq!(test_bytes(&m), [1, 2, 3, 4, 5, 6, 7, 0, 0]);
        // Across mbufs: the rest emptied.
        let mut m = test_chain(&[&[1, 2, 3, 4], &[5, 6], &[7, 8]]);
        nfsm_adj(&mut m, 5, 1);
        assert_eq!(test_bytes(&m), [1, 2, 0]);
        assert_eq!(test_lens(&m), [3, 0, 0]);
        // More than the chain holds: everything trimmed; a fill wider than the data clamped.
        let mut m = test_chain(&[&[1, 2], &[3]]);
        nfsm_adj(&mut m, 9, 4);
        assert_eq!(test_lens(&m), [0, 0]);
    }

    #[test]
    fn the_wcc_data_and_post_op_attributes_are_the_wire_layout() {
        let _g = crate::net::mbuf::tests::setup();
        let nd = v3();
        let va = vattr();
        let mut w = writer();
        nfsm_srvwcc(&nd, Some(&va), None, &mut w);
        let b = test_bytes(&w.into_chain());
        let mut want = be(&[1, 1, 2, 11, 2000, 12, 3000, 0]);
        assert_eq!(b, want);

        let mut w = writer();
        nfsm_srvwcc(&nd, None, Some(&va), &mut w);
        let b = test_bytes(&w.into_chain());
        assert_eq!(b.len(), 4 + 4 + NFSX_V3FATTR);
        assert_eq!(&b[..8], &be(&[0, 1])[..]);
        // rdev 0x01020304: major 3, minor 0x010204 (`<sys/types.h>`'s split).
        want = be(&[
            1,
            0o644,
            2,
            1000,
            20,
            1,
            2,
            0,
            4096,
            3,
            0x0001_0204,
            0,
            7,
            2,
            9,
        ]);
        assert_eq!(&b[8..8 + want.len()], &want[..]);
        // The times: 10.000001, 11.000002, 12.000003.
        assert_eq!(&b[68..92], &be(&[10, 1000, 11, 2000, 12, 3000])[..]);
    }

    #[test]
    fn version_2_attributes_are_the_shorter_structure() {
        let nd = NfsrvDescript::default();
        let mut va = vattr();
        let fp = nfsm_srvfattr(&nd, &va);
        assert_eq!(u32::from_be(fp.fa_type), 1); // NFREG
        assert_eq!(u32::from_be(fp.fa2_size()), 2);
        assert_eq!(u32::from_be(fp.fa2_blocksize()), 8192);
        assert_eq!(u32::from_be(fp.fa2_blocks()), 8);
        assert_eq!(u32::from_be(fp.fa2_fileid()), 9);
        assert_eq!(u32::from_be(fp.fa2_rdev()), 0x0102_0304);
        assert_eq!(fp.fa_un[12..], [0, 0, 0, 0]);
        va.va_type = Vtype::VFIFO;
        assert_eq!(nfsm_srvfattr(&nd, &va).fa2_rdev(), 0xffff_ffff);
    }

    #[test]
    fn sattr3_is_read_across_mbufs_and_refused_when_cut_short() {
        let _g = crate::net::mbuf::tests::setup();
        // mode 0o600, no uid, gid 5, size 3, atime to the client's 7.8, mtime the server's.
        let w = be(&[1, 0o600, 0, 1, 5, 1, 0, 3, 2, 7, 8, 1]);
        let m = test_chain(&[&w[..6], &w[6..30], &w[30..]]);
        let mut r = XdrReader::new(&m);
        let mut va = Vattr::default();
        va.va_uid = 42;
        assert_eq!(nfsm_srvsattr(&mut r, &mut va), Ok(()));
        assert_eq!(va.va_mode, 0o600);
        assert_eq!(va.va_uid, 42);
        assert_eq!(va.va_gid, 5);
        assert_eq!(va.va_size, 3);
        assert_eq!(
            va.va_atime,
            Timespec {
                tv_sec: 7,
                tv_nsec: 8
            }
        );
        assert!(va.va_vaflags & VA_UTIMES_CHANGE != 0);
        assert_eq!(r.offset(), w.len());
        // Every cut before the end is EBADRPC, never a panic.
        for cut in 0..w.len() {
            let m = test_chain(&[&w[..cut]]);
            let mut r = XdrReader::new(&m);
            let mut va = Vattr::default();
            assert_eq!(
                nfsm_srvsattr(&mut r, &mut va),
                Err(Errno::EBADRPC),
                "cut {cut}"
            );
        }
    }

    #[test]
    fn netaddr_match_compares_inet_hosts_only() {
        let _g = crate::net::mbuf::tests::setup();
        let sin = [16u8, AF_INET, 0, 7, 10, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        let m = test_chain(&[&sin]);
        let a = u32::from_ne_bytes([10, 0, 0, 1]);
        assert!(netaddr_match(AF_INET, a, &m));
        assert!(!netaddr_match(AF_INET, a + 1, &m));
        assert!(!netaddr_match(0, a, &m));
        // A short address never matches.
        let m = test_chain(&[&sin[..8]]);
        assert!(!netaddr_match(AF_INET, a, &m));
        assert!(sockaddr_in_of(&m).is_none());
    }

    #[test]
    fn the_file_handle_is_fixed_in_version_2_and_a_string_in_version_3() {
        let _g = crate::net::mbuf::tests::setup();
        let mut f = Nfsfh::new();
        for (i, b) in f.fh_bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut w = writer();
        nfsm_srvfhtom(&mut w, &f, false);
        let b = test_bytes(&w.into_chain());
        assert_eq!(b, f.fh_bytes[..NFSX_V2FH]);
        let mut w = writer();
        nfsm_srvfhtom(&mut w, &f, true);
        let b = test_bytes(&w.into_chain());
        assert_eq!(&b[..4], &(NFSX_V3FH as u32).to_be_bytes());
        assert_eq!(&b[4..4 + NFSX_V3FH], &f.fh_bytes[..NFSX_V3FH]);
        assert_eq!(b.len(), 4 + super::super::nfsm_subs::nfsm_rndup(NFSX_V3FH));
    }

    // The export check over a test mount: a file system that is the test file system with an
    // export list of the test's choosing (`checkexp` answers the grant a thread-local holds),
    // on the mount list with a fsid of its own.

    mod export {
        use core::cell::Cell;

        use hal::sys::types::Ino;

        use super::*;
        use crate::fs::export::ExportGrant;
        use crate::fs::lookup::ndinit;
        use crate::fs::mount::{
            Mount, Vfsconf, mountlist_insert_tail, vfs_mount_alloc, vfs_unbusy,
        };
        use crate::fs::namei::{LOCKLEAF, LOOKUP, NiDirp};
        use crate::fs::nfs::nfsm_subs::nfsm_rndup;
        use crate::fs::subr::vfs_getnewfsid;
        use crate::fs::testfs::{Kind, ROOT, TEST_DATASIZE, TEST_VFS, TestGlue, tree_add};
        use crate::fs::vops::{Cred, Nd, Vfs};
        use crate::net::mbuf::MbufBox;
        use crate::proc::Thread;
        use crate::sys::mount::{Fid, MNT_EXPORTED, MNT_LOCAL, Statfs};
        use crate::sys::socket::SOCK_DGRAM;

        type G = TestGlue;

        std::thread_local! {
            /// What the test file system's `checkexp` answers: a grant, or the errno.
            static GRANT: Cell<Result<ExportGrant, Errno>> = const { Cell::new(Err(Errno::EACCES)) };
        }

        /// The test file system with an export list.
        struct ExpVfs;

        impl Vfs<G> for ExpVfs {
            fn mount(
                &self,
                mp: &'static Mount<G>,
                path: &[u8],
                data: &mut [u8],
                ndp: &mut Nd<'_, G>,
                p: &Thread<G>,
            ) -> Result<(), Errno> {
                TEST_VFS.mount(mp, path, data, ndp, p)
            }
            fn start(&self, mp: &'static Mount<G>, f: i32, p: &Thread<G>) -> Result<(), Errno> {
                TEST_VFS.start(mp, f, p)
            }
            fn unmount(&self, mp: &'static Mount<G>, f: i32, p: &Thread<G>) -> Result<(), Errno> {
                TEST_VFS.unmount(mp, f, p)
            }
            fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
                TEST_VFS.root(mp)
            }
            fn quotactl(
                &self,
                mp: &'static Mount<G>,
                cmds: i32,
                uid: u32,
                arg: usize,
                p: &Thread<G>,
            ) -> Result<(), Errno> {
                TEST_VFS.quotactl(mp, cmds, uid, arg, p)
            }
            fn statfs(
                &self,
                mp: &'static Mount<G>,
                sbp: &mut Statfs,
                p: &Thread<G>,
            ) -> Result<(), Errno> {
                TEST_VFS.statfs(mp, sbp, p)
            }
            fn sync(
                &self,
                mp: &'static Mount<G>,
                waitfor: i32,
                stall: i32,
                cred: Cred<'_>,
                p: &Thread<G>,
            ) -> Result<(), Errno> {
                TEST_VFS.sync(mp, waitfor, stall, cred, p)
            }
            fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
                TEST_VFS.vget(mp, ino)
            }
            fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
                TEST_VFS.fhtovp(mp, fid)
            }
            fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
                TEST_VFS.vptofh(vp, fid)
            }
            fn checkexp(&self, _mp: &'static Mount<G>, _nam: &Mbuf) -> Result<ExportGrant, Errno> {
                GRANT.with(Cell::get)
            }
        }

        static EXP_VFS: ExpVfs = ExpVfs;
        static EXP_CONF: [Vfsconf<G>; 1] = [Vfsconf::new(
            &EXP_VFS,
            b"expfs",
            98,
            MNT_LOCAL,
            TEST_DATASIZE,
        )];

        /// A grant of `exflags`, the anonymous ids `anon`, groups 5 and 6.
        fn grant(exflags: i32, anon: u32) -> ExportGrant {
            let mut groups = [0; crate::sys::syslimits::NGROUPS_MAX];
            groups[0] = 5;
            groups[1] = 6;
            ExportGrant {
                exflags,
                uid: anon,
                gid: anon,
                ngroups: 2,
                groups,
            }
        }

        /// The exported mount, a file and a directory in its tree, and their handles.
        fn exported() -> (
            std::sync::MutexGuard<'static, ()>,
            &'static Thread<G>,
            Fhandle,
            Fhandle,
        ) {
            let (g, p) = crate::fs::testfs::test_thread_cached(64 << 20, 64);
            crate::testutil::kernel_maps_install();
            crate::net::mbuf::mbinit(&crate::net::mbuf::tests::TEST_NMBCLUST);
            let _ = crate::net::mbuf::nmbclust_update(256 * 1024);
            let mp = vfs_mount_alloc(None, &EXP_CONF[0]);
            vfs_unbusy(mp);
            vfs_getnewfsid(mp);
            mountlist_insert_tail(mp);
            let file = tree_add(b"file", ROOT, Kind::Reg, None);
            let handle = |idx: usize| {
                let vp = TEST_VFS
                    .fhtovp(mp, &{
                        let mut f = Fid::default();
                        f.fid_len = 4;
                        f.fid_data[..4].copy_from_slice(&(idx as u32).to_ne_bytes());
                        f
                    })
                    .expect("a node");
                let mut fid = Fid::default();
                TEST_VFS.vptofh(vp, &mut fid).expect("its handle");
                vput(vp);
                Fhandle {
                    fh_fsid: mp.fsid(),
                    fh_fid: fid,
                }
            };
            let (dir, f) = (handle(ROOT), handle(file));
            (g, p, dir, f)
        }

        /// A client's name at 127.0.0.1:`port`.
        fn client(port: u16) -> MbufBox {
            let mut sin = [0u8; 16];
            sin[0] = 16;
            sin[1] = AF_INET;
            sin[2..4].copy_from_slice(&port.to_be_bytes());
            sin[4..8].copy_from_slice(&[127, 0, 0, 1]);
            test_chain(&[&sin])
        }

        fn sock(sotype: i32) -> NfssvcSock<G> {
            let s = NfssvcSock::new(None);
            s.ns_sotype
                .store(sotype, core::sync::atomic::Ordering::Relaxed);
            s
        }

        fn root_cred() -> UcredCell {
            UcredCell::new()
        }

        #[test]
        fn fhtovp_checks_the_mount_the_export_and_the_port() {
            let (_g, _p, dir, _) = exported();
            hal::klock::kernel_lock();
            let udp = sock(SOCK_DGRAM);
            let tcp = sock(SOCK_STREAM);
            let tooweak = NFSERR_AUTHERR | AUTH_TOOWEAK as i32;
            GRANT.with(|g| g.set(Ok(grant(MNT_EXPORTED, 32767))));
            let ok = |r: Result<(&'static Vnode<G>, bool), i32>| match r {
                Ok((vp, _)) => {
                    vrele(vp);
                    Ok(())
                }
                Err(e) => Err(e),
            };
            // An unknown file system: ESTALE, before the export is asked.
            let mut stale = dir;
            stale.fh_fsid.val[0] ^= 0x5a5a;
            assert_eq!(
                ok(nfsrv_fhtovp(
                    &stale,
                    false,
                    &root_cred(),
                    &udp,
                    Some(&client(700))
                )),
                Err(Errno::ESTALE.as_i32())
            );
            // The export refuses the client: its errno.
            GRANT.with(|g| g.set(Err(Errno::EACCES)));
            assert_eq!(
                ok(nfsrv_fhtovp(
                    &dir,
                    false,
                    &root_cred(),
                    &udp,
                    Some(&client(700))
                )),
                Err(Errno::EACCES.as_i32())
            );
            GRANT.with(|g| g.set(Ok(grant(MNT_EXPORTED, 32767))));
            // Reserved ports pass, 1024 does not; port 20 only on a datagram socket.
            assert_eq!(
                ok(nfsrv_fhtovp(
                    &dir,
                    false,
                    &root_cred(),
                    &udp,
                    Some(&client(1023))
                )),
                Ok(())
            );
            assert_eq!(
                ok(nfsrv_fhtovp(
                    &dir,
                    false,
                    &root_cred(),
                    &udp,
                    Some(&client(1024))
                )),
                Err(tooweak)
            );
            assert_eq!(
                ok(nfsrv_fhtovp(
                    &dir,
                    false,
                    &root_cred(),
                    &udp,
                    Some(&client(20))
                )),
                Ok(())
            );
            assert_eq!(
                ok(nfsrv_fhtovp(
                    &dir,
                    false,
                    &root_cred(),
                    &tcp,
                    Some(&client(20))
                )),
                Err(tooweak)
            );
            // A short address, or none: not a client to vouch for.
            let short = test_chain(&[&client(700).data()[..8]]);
            assert_eq!(
                ok(nfsrv_fhtovp(&dir, false, &root_cred(), &udp, Some(&short))),
                Err(tooweak)
            );
            assert_eq!(
                ok(nfsrv_fhtovp(&dir, false, &root_cred(), &udp, None)),
                Err(tooweak)
            );
            hal::klock::kernel_unlock();
        }

        #[test]
        fn fhtovp_maps_root_and_everyone_on_mapall_and_says_read_only() {
            let (_g, _p, dir, _) = exported();
            hal::klock::kernel_lock();
            let udp = sock(SOCK_DGRAM);
            let at = Some(client(700));
            // -maproot: uid 0 takes the anonymous ids and groups.
            GRANT.with(|g| g.set(Ok(grant(MNT_EXPORTED, 32767))));
            let cr = root_cred();
            let (vp, rdonly) =
                nfsrv_fhtovp(&dir, false, &cr, &udp, at.as_deref()).expect("granted");
            vrele(vp);
            assert!(!rdonly);
            assert_eq!(
                (cr.cr_uid.get(), cr.cr_gid.get(), cr.cr_ngroups.get()),
                (32767, 32767, 2)
            );
            assert_eq!(cr.cr_groups[1].get(), 6);
            // Another user keeps its ids...
            let cr = UcredCell::new();
            cr.set_uid(1000);
            cr.set_gid(100);
            let (vp, _) = nfsrv_fhtovp(&dir, false, &cr, &udp, at.as_deref()).expect("granted");
            vrele(vp);
            assert_eq!((cr.cr_uid.get(), cr.cr_gid.get()), (1000, 100));
            // ...unless -mapall (MNT_EXPORTANON); -ro (MNT_EXRDONLY) is the answer's second.
            GRANT.with(|g| g.set(Ok(grant(MNT_EXPORTED | MNT_EXPORTANON | MNT_EXRDONLY, 4))));
            let (vp, rdonly) =
                nfsrv_fhtovp(&dir, false, &cr, &udp, at.as_deref()).expect("granted");
            vrele(vp);
            assert!(rdonly);
            assert_eq!(
                (cr.cr_uid.get(), cr.cr_gid.get(), cr.cr_ngroups.get()),
                (4, 4, 2)
            );
            // A group count past NGROUPS_MAX takes NGROUPS_MAX groups; a negative one none.
            let mut big = grant(MNT_EXPORTED, 9);
            big.ngroups = 40;
            GRANT.with(|g| g.set(Ok(big)));
            let cr = root_cred();
            let (vp, _) = nfsrv_fhtovp(&dir, false, &cr, &udp, at.as_deref()).expect("granted");
            vrele(vp);
            assert_eq!(
                cr.cr_ngroups.get() as usize,
                crate::sys::syslimits::NGROUPS_MAX
            );
            big.ngroups = -1;
            GRANT.with(|g| g.set(Ok(big)));
            let cr = root_cred();
            let (vp, _) = nfsrv_fhtovp(&dir, false, &cr, &udp, at.as_deref()).expect("granted");
            vrele(vp);
            assert_eq!(cr.cr_ngroups.get(), 0);
            hal::klock::kernel_unlock();
        }

        /// `nfs_namei` of the request bytes `req` (a name and its padding) under `dir`.
        fn namei(
            p: &'static Thread<G>,
            dir: &Fhandle,
            name: &[u8],
            len: usize,
            cr: &UcredCell,
        ) -> (Result<(), i32>, Option<&'static Vnode<G>>, u64) {
            let mut b = name.to_vec();
            b.resize(nfsm_rndup(name.len()), 0);
            let m = test_chain(&[&b[..b.len().min(3)], &b[b.len().min(3)..]]);
            let mut r = XdrReader::new(&m);
            let udp = sock(SOCK_DGRAM);
            let at = client(700);
            let mut nd = ndinit(LOOKUP, LOCKLEAF, NiDirp::Sys(b""), p);
            nd.ni_cnd.cn_cred = Cred::Real(cr);
            let mut dirp = None;
            let res = nfs_namei(&mut nd, dir, len, &udp, Some(&at), &mut r, &mut dirp, p, cr);
            if let Some(d) = dirp {
                vrele(d);
            }
            let flags = nd.ni_cnd.cn_flags;
            if res.is_ok() {
                assert_eq!(
                    r.offset(),
                    b.len(),
                    "the cursor past the name and its padding"
                );
                if let Some(dvp) = nd.ni_dvp.take() {
                    vrele(dvp);
                }
            }
            (res, nd.ni_vp.take(), flags)
        }

        #[test]
        fn namei_looks_names_up_under_the_handle_and_refuses_slashes_and_nuls() {
            let (_g, p, dir, file) = exported();
            hal::klock::kernel_lock();
            GRANT.with(|g| g.set(Ok(grant(MNT_EXPORTED | MNT_EXRDONLY, 32767))));
            let cr = root_cred();
            let (res, vp, flags) = namei(p, &dir, b"file", 4, &cr);
            assert_eq!(res, Ok(()));
            let vp = vp.expect("the file");
            assert_eq!(vp.vtype(), Vtype::VREG);
            vput(vp);
            // Never across a mount point; read-only on a read-only export.
            assert_eq!(flags & (NOCROSSMOUNT | RDONLY), NOCROSSMOUNT | RDONLY);
            for bad in [&b"fi/le"[..], &b"fi\0le"[..], &b"/"[..]] {
                let (res, vp, _) = namei(p, &dir, bad, bad.len(), &cr);
                assert_eq!(res, Err(Errno::EACCES.as_i32()), "{bad:?}");
                assert!(vp.is_none());
            }
            // A handle of a file is no directory to look under.
            let (res, _, _) = namei(p, &file, b"x", 1, &cr);
            assert_eq!(res, Err(Errno::ENOTDIR.as_i32()));
            // A name the request does not hold all of: EBADRPC (four bytes, no padding to
            // meet first: a NUL of the padding is EACCES, as the C checks each byte).
            let (res, _, _) = namei(p, &dir, b"filx", 9, &cr);
            assert_eq!(res, Err(Errno::EBADRPC.as_i32()));
            let (res, _, _) = namei(p, &dir, b"fil", 9, &cr);
            assert_eq!(res, Err(Errno::EACCES.as_i32()));
            // A name too long for the pathname buffer.
            let (res, _, _) = namei(p, &dir, b"f", MAXPATHLEN, &cr);
            assert_eq!(res, Err(Errno::ENAMETOOLONG.as_i32()));
            hal::klock::kernel_unlock();
        }
    }
}
/* </TESTS> */
