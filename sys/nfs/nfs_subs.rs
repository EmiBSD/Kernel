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
//! `nfs/nfs_subs.c`, the server's half (D-5; the client's, the RPC header, the xids, the
//! attribute cache and the commit ranges, is the kernel's, `kernel::fs::nfs::nfs_subs`, since
//! N6.7a5): the functions that support the `nfsm_subs.h` inline functions and help fiddle
//! mbuf chains for the server's procedures: they copy data between mbuf chains and uio lists
//! and map errors.
//!
//! Upstream: sys/nfs/nfs_subs.c @ 3ce1f3f79392
//!
//! Mbufs are `&'static Mbuf` (`docs/C_TO_RUST.md`). A build cursor (the C's `struct mbuf
//! **mb`, the last mbuf of the chain being built) is a `&mut &'static Mbuf`; a dissection
//! cursor is an `Option<&'static Mbuf>` with a raw position in its data, checked by
//! `nfsm_avail` (`nfsm_subs.rs`) before every use. Data comes back as the bounds-checked
//! views of `nfsm_subs.rs` ([`XdrIn`], [`XdrOut`]).
//!
//! ## Deviations
//! - The XDR words `nfs_init` computes at startup (`rpc_call`, `nfs_true`, `nfs_xdrneg1`,
//!   ...) are `const`s computed at compile time, with their C names (lowercase:
//!   `RPC_CALL` is already the `rpcv2.h` constant).
//! - `MGET(M_WAIT)` cannot sleep here (`subr_pool.rs`): where the C cannot fail
//!   (`nfsm_build`, `nfsm_uiotombuf`) an empty pool panics, the
//!   `vfs_subr.rs` precedent; `nfsm_disct`, which returns an error anyway, fails with
//!   `ENOBUFS`.
//! - `nfsm_disct` and `nfs_adv` compute the bytes left in the current mbuf (the C's `left`
//!   argument, always that value) themselves; `nfs_adv` advances from the position when it
//!   is already far enough, a case the C's callers never reach. `nfsm_disct` returns the
//!   view instead of storing a pointer through `cp2`.
//! - [`nfsm_nextbytes`] is a Rust helper for the cursor walks of other files (`nfs_namei`
//!   copies a name byte by byte across mbufs), so that they need not dereference `dpos`.
//! - `nfsm_mbuftouio` ignores `copyout`'s error, and `nfsm_uiotombuf` `uiomove`'s, as the C
//!   does.
//! - `nfsm_buftombuf`/`nfsm_strtombuf` take a byte slice (its length is the C's `len`);
//!   `nfsm_srvfhtom` takes the `nfsfh_t` the server's handle lives in (the C passes its
//!   `fh_generic` member and reads `NFSX_V2FH` bytes from it, past the `fhandle_t`).
//! - `nfsstats` is the kernel's `NFSSTATS` of atomics.
//! - `nfs_init` calls the kernel's (`nfs_ticks`, "nfsreqpl"), then, under `NFSSERVER`,
//!   `nfsrv_init(0)` (`nfs_syscalls.c`) and `nfsrv_initcache()` (`nfs_srvcache.c`), as the C
//!   does.
//! - `nfsrv_errmap` maps an `err` below 1 to `NFSERR_IO`; the C indexes the version 2 table
//!   at `err - 1` and would read before it (its callers never pass one).

use core::ffi::c_void;
use core::ptr;
use core::slice;

use crate::kern::kern_subr::uiomove;
use crate::kern::subr_prf::panic;
use crate::kern::uipc_mbuf::{m_get, m_trailingspace};
use crate::machine::copy::copyout;
use crate::nfs::nfs::{ND_NFSV3, NfsrvDescript};
use crate::nfs::nfs_var::{mb_offset, nfsm_padlen};
use crate::nfs::nfsm_subs::{XdrIn, XdrOut, nfsm_avail};
use crate::nfs::nfsproto::NFSPROC_COMMIT;
use crate::nfs::nfsproto::{
    NFS_NPROCS, NFSERR_ACCES, NFSERR_BAD_COOKIE, NFSERR_BADHANDLE, NFSERR_BADTYPE, NFSERR_DQUOT,
    NFSERR_EXIST, NFSERR_FBIG, NFSERR_INVAL, NFSERR_IO, NFSERR_ISDIR, NFSERR_MLINK, NFSERR_NAMETOL,
    NFSERR_NODEV, NFSERR_NOENT, NFSERR_NOSPC, NFSERR_NOT_SYNC, NFSERR_NOTDIR, NFSERR_NOTEMPTY,
    NFSERR_NOTSUPP, NFSERR_NXIO, NFSERR_PERM, NFSERR_ROFS, NFSERR_SERVERFAULT, NFSERR_STALE,
    NFSERR_TOOSMALL, NFSERR_XDEV, NFSPROC_CREATE, NFSPROC_FSSTAT, NFSPROC_GETATTR, NFSPROC_LINK,
    NFSPROC_LOOKUP, NFSPROC_MKDIR, NFSPROC_NOOP, NFSPROC_NULL, NFSPROC_READ, NFSPROC_READDIR,
    NFSPROC_READLINK, NFSPROC_REMOVE, NFSPROC_RENAME, NFSPROC_RMDIR, NFSPROC_SETATTR,
    NFSPROC_SYMLINK, NFSPROC_WRITE, NFSV2PROC_CREATE, NFSV2PROC_GETATTR, NFSV2PROC_LINK,
    NFSV2PROC_LOOKUP, NFSV2PROC_MKDIR, NFSV2PROC_NOOP, NFSV2PROC_NULL, NFSV2PROC_READ,
    NFSV2PROC_READDIR, NFSV2PROC_READLINK, NFSV2PROC_REMOVE, NFSV2PROC_RENAME, NFSV2PROC_RMDIR,
    NFSV2PROC_SETATTR, NFSV2PROC_STATFS, NFSV2PROC_SYMLINK, NFSV2PROC_WRITE, NFSX_V2FH, NFSX_V3FH,
    Nfsfh,
};
use crate::nfs::xdr_subs::{fxdr_unsigned, txdr_unsigned};
use crate::sys::errno::Errno;
use crate::sys::mbuf::{M_WAIT, MHLEN, MLEN, MT_DATA, Mbuf, mclget, mtod};
use crate::sys::mbuf::{m_data_add, m_set_next, m_take_next};
use crate::sys::uio::{Iovec, Uio, UioRw, UioSeg};

pub use kernel::fs::nfs::nfs_subs::{
    NFSV2_TYPE, NFSV3_TYPE, NV2TOV_TYPE, NV3TOV_TYPE, nfs_false, nfs_prog, nfs_true, nfs_xdrneg1,
    rpc_auth_unix, rpc_autherr, rpc_call, rpc_mismatch, rpc_msgaccepted, rpc_msgdenied, rpc_reply,
    rpc_vers,
};

/// `nfsv3_procid`: mapping of old NFS version 2 RPC numbers to generic numbers.
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

/// `nfsv2_procid`: and the reverse mapping from generic to version 2 procedure numbers.
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

/// `nfsv3err_null`.
const NFSV3ERR_NULL: &[i16] = &[0, 0];

/// `nfsv3err_getattr`.
const NFSV3ERR_GETATTR: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_setattr`.
const NFSV3ERR_SETATTR: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_PERM as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_INVAL as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOT_SYNC as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_lookup`.
const NFSV3ERR_LOOKUP: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_NOENT as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_access`.
const NFSV3ERR_ACCESS: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_readlink`.
const NFSV3ERR_READLINK: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_INVAL as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_read`.
const NFSV3ERR_READ: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_NXIO as i16,
    NFSERR_ACCES as i16,
    NFSERR_INVAL as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_write`.
const NFSV3ERR_WRITE: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_INVAL as i16,
    NFSERR_FBIG as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_create`.
const NFSV3ERR_CREATE: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_EXIST as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_mkdir`.
const NFSV3ERR_MKDIR: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_EXIST as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_symlink`.
const NFSV3ERR_SYMLINK: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_EXIST as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_mknod`.
const NFSV3ERR_MKNOD: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_EXIST as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    NFSERR_BADTYPE as i16,
    0,
];

/// `nfsv3err_remove`.
const NFSV3ERR_REMOVE: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_NOENT as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_ROFS as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_rmdir`.
const NFSV3ERR_RMDIR: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_NOENT as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_EXIST as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_INVAL as i16,
    NFSERR_ROFS as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_NOTEMPTY as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_rename`.
const NFSV3ERR_RENAME: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_NOENT as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_EXIST as i16,
    NFSERR_XDEV as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_ISDIR as i16,
    NFSERR_INVAL as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_MLINK as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_NOTEMPTY as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_link`.
const NFSV3ERR_LINK: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_EXIST as i16,
    NFSERR_XDEV as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_INVAL as i16,
    NFSERR_NOSPC as i16,
    NFSERR_ROFS as i16,
    NFSERR_MLINK as i16,
    NFSERR_NAMETOL as i16,
    NFSERR_DQUOT as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_readdir`.
const NFSV3ERR_READDIR: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_BAD_COOKIE as i16,
    NFSERR_TOOSMALL as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_readdirplus`.
const NFSV3ERR_READDIRPLUS: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_ACCES as i16,
    NFSERR_NOTDIR as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_BAD_COOKIE as i16,
    NFSERR_NOTSUPP as i16,
    NFSERR_TOOSMALL as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_fsstat`.
const NFSV3ERR_FSSTAT: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_fsinfo`.
const NFSV3ERR_FSINFO: &[i16] = &[
    NFSERR_STALE as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_pathconf`.
const NFSV3ERR_PATHCONF: &[i16] = &[
    NFSERR_STALE as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsv3err_commit`.
const NFSV3ERR_COMMIT: &[i16] = &[
    NFSERR_IO as i16,
    NFSERR_IO as i16,
    NFSERR_STALE as i16,
    NFSERR_BADHANDLE as i16,
    NFSERR_SERVERFAULT as i16,
    0,
];

/// `nfsrv_v3errmap`: the version 3 error list of each procedure, `NFSPROC_NULL` to
/// `NFSPROC_COMMIT`.
pub const NFSRV_V3ERRMAP: [&[i16]; 22] = [
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

pub use kernel::fs::nfs::nfs_subs::NFSSTATS;

/// `MGET(m, M_WAIT, MT_DATA)` where the C cannot fail: panics when the pool cannot serve
/// (it cannot sleep here, `subr_pool.rs`).
fn nfsm_mget() -> &'static Mbuf {
    match m_get(M_WAIT, MT_DATA) {
        Some(m) => m,
        None => panic(format_args!("nfs: out of mbufs")),
    }
}

/// `nfsm_mbuftouio(mrep, uiop, siz, dpos)`: copies `siz` bytes of an mbuf chain, from the
/// cursor (`*mrep`, `*dpos`), to the uio scatter/gather list, then skips the XDR padding.
pub fn nfsm_mbuftouio(
    mrep: &mut Option<&'static Mbuf>,
    uiop: &mut Uio<'_>,
    mut siz: usize,
    dpos: &mut *mut u8,
) -> Result<(), Errno> {
    let Some(mut mp) = *mrep else {
        return Err(Errno::EBADRPC);
    };
    let mut mbufcp = *dpos;
    let mut len = nfsm_avail(mp, mbufcp);
    let rem = nfsm_padlen(siz);
    while siz > 0 {
        if uiop.uio_iov.is_empty() {
            return Err(Errno::EFBIG);
        }
        let mut left = uiop.uio_iov[0].iov_len;
        let mut uiocp = uiop.uio_iov[0].iov_base.cast::<u8>();
        if left > siz {
            left = siz;
        }
        let uiosiz = left;
        while left > 0 {
            while len == 0 {
                mp = mp.m_next().get().ok_or(Errno::EBADRPC)?;
                mbufcp = mtod::<u8>(mp);
                len = mp.m_len().get() as usize;
            }
            let xfer = left.min(len);
            if uiop.uio_segflg == UioSeg::UIO_SYSSPACE {
                // SAFETY: `xfer <= len` bytes at `mbufcp` are `mp`'s data; a UIO_SYSSPACE
                // iovec is a kernel buffer of at least `iov_len >= xfer` bytes past `uiocp`
                // that its builder vouches for (`struct uio`'s contract); the two are
                // distinct memory.
                unsafe { ptr::copy_nonoverlapping(mbufcp, uiocp, xfer) };
            } else {
                // SAFETY: `xfer <= len` bytes at `mbufcp` are `mp`'s data, initialised and
                // not written during the copy.
                let src = unsafe { slice::from_raw_parts(mbufcp, xfer) };
                // The C ignores copyout's error.
                let _ = copyout(src, uiocp.addr());
            }
            left -= xfer;
            len -= xfer;
            mbufcp = mbufcp.wrapping_add(xfer);
            uiocp = uiocp.wrapping_add(xfer);
            uiop.uio_offset += xfer as i64;
            uiop.uio_resid -= xfer;
        }
        if uiop.uio_iov[0].iov_len <= siz {
            let iov = core::mem::take(&mut uiop.uio_iov);
            uiop.uio_iov = &mut iov[1..];
        } else {
            let iov = &mut uiop.uio_iov[0];
            iov.iov_base = iov.iov_base.wrapping_byte_add(uiosiz);
            iov.iov_len -= uiosiz;
        }
        siz -= uiosiz;
    }
    *dpos = mbufcp;
    *mrep = Some(mp);
    if rem > 0 {
        if len < rem {
            nfs_adv(mrep, dpos, rem)?;
        } else {
            *dpos = dpos.wrapping_add(rem);
        }
    }
    Ok(())
}

/// `nfsm_uiotombuf(mp, uiop, len)`: copies `len` bytes of a uio scatter/gather list to the
/// end of the mbuf chain at the build cursor `*mp`, then the XDR padding.
pub fn nfsm_uiotombuf(mp: &mut &'static Mbuf, uiop: &mut Uio<'_>, mut len: usize) {
    let mut mb = *mp;

    let pad = nfsm_padlen(len);

    // XXX -- the following should be done by the caller
    uiop.uio_resid = len;
    uiop.uio_rw = UioRw::UIO_WRITE;

    while len > 0 {
        let xfer = len.min(m_trailingspace(mb) as usize);
        // SAFETY: the `xfer` bytes after `mb`'s data are its trailing space
        // (`m_trailingspace`), storage of the chain being built that nothing else uses.
        let dst = unsafe { slice::from_raw_parts_mut(mb_offset(mb), xfer) };
        // The C ignores uiomove's error.
        let _ = uiomove(dst, uiop);
        mb.m_len().set(mb.m_len().get() + xfer as u32);
        len -= xfer;
        if len > 0 {
            let mb2 = nfsm_mget();
            if len > MLEN {
                mclget(mb2, M_WAIT);
            }
            mb2.m_len().set(0);
            let _ = m_set_next(mb, Some(mb2));
            mb = mb2;
        }
    }

    if pad > 0 {
        if pad > m_trailingspace(mb) as usize {
            let mb2 = nfsm_mget();
            mb2.m_len().set(0);
            let _ = m_set_next(mb, Some(mb2));
            mb = mb2;
        }
        // SAFETY: the `pad` bytes after `mb`'s data are its trailing space (checked above, or
        // a fresh mbuf of `MLEN` bytes), storage of the chain being built.
        unsafe { ptr::write_bytes(mb_offset(mb), 0, pad) };
        mb.m_len().set(mb.m_len().get() + pad as u32);
    }

    *mp = mb;
}

/// `nfsm_buftombuf(mp, buf, len)`: copies a buffer to the end of an mbuf chain.
pub fn nfsm_buftombuf(mp: &mut &'static Mbuf, buf: &[u8]) {
    let mut iov = [Iovec {
        iov_base: buf.as_ptr().cast_mut().cast::<c_void>(),
        iov_len: buf.len(),
    }];

    // A UIO_WRITE uio is only read from: `buf` is not written.
    let mut io = Uio {
        uio_iov: &mut iov,
        uio_offset: 0,
        uio_resid: buf.len(),
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: UioRw::UIO_WRITE,
        uio_procp: None,
    };

    nfsm_uiotombuf(mp, &mut io, buf.len());
}

/// `nfsm_strtombuf(mp, str, len)`: copies a string to the end of an mbuf chain, as XDR: the
/// length word, the bytes, the padding.
pub fn nfsm_strtombuf(mp: &mut &'static Mbuf, str: &[u8]) {
    let strlen = txdr_unsigned(str.len() as u32);

    let mut iov = [
        Iovec {
            iov_base: ptr::from_ref(&strlen).cast_mut().cast::<c_void>(),
            iov_len: size_of::<u32>(),
        },
        Iovec {
            iov_base: str.as_ptr().cast_mut().cast::<c_void>(),
            iov_len: str.len(),
        },
    ];

    // A UIO_WRITE uio is only read from: neither buffer is written.
    let resid = size_of::<u32>() + str.len();
    let mut io = Uio {
        uio_iov: &mut iov,
        uio_offset: 0,
        uio_resid: resid,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: UioRw::UIO_WRITE,
        uio_procp: None,
    };

    nfsm_uiotombuf(mp, &mut io, resid);
}

/// `nfsm_disct(mdp, dposp, siz, left, cp2)`: helps break down an mbuf chain by making the
/// next `siz` bytes at the cursor contiguous, returned as a view. This is used by
/// `nfsm_dissect` for tough cases: when they straddle mbufs, a new mbuf is inserted that
/// holds them.
pub fn nfsm_disct<'a>(
    mdp: &'a mut Option<&'static Mbuf>,
    dposp: &'a mut *mut u8,
    siz: usize,
) -> Result<XdrIn<'a>, Errno> {
    let Some(mut mp) = *mdp else {
        return Err(Errno::EBADRPC);
    };
    let mut left = nfsm_avail(mp, *dposp);
    while left == 0 {
        *mdp = mp.m_next().get();
        let Some(m) = *mdp else {
            return Err(Errno::EBADRPC);
        };
        mp = m;
        left = mp.m_len().get() as usize;
        *dposp = mtod::<u8>(mp);
    }
    if left >= siz {
        let cp2 = *dposp;
        *dposp = cp2.wrapping_add(siz);
        // SAFETY: `siz <= left` bytes at `cp2` are `mp`'s data, which the chain's owner does
        // not write while the view borrows its cursor.
        return Ok(unsafe { XdrIn::new(cp2, siz) });
    }
    if mp.m_next().get().is_none() {
        return Err(Errno::EBADRPC);
    }
    if siz > MHLEN {
        panic(format_args!("nfs S too big"));
    }

    let Some(mp2) = m_get(M_WAIT, MT_DATA) else {
        return Err(Errno::ENOBUFS);
    };
    // mp2->m_next = mp->m_next; mp->m_next = mp2 (the rest taken off its link first).
    let rest = m_take_next(mp);
    let _ = m_set_next(mp2, rest);
    let _ = m_set_next(mp, Some(mp2));
    mp.m_len().set(mp.m_len().get() - left as u32);
    let mp = mp2;
    let cp2 = mtod::<u8>(mp);
    // Copy what was left.
    // SAFETY: the `left` bytes at the cursor are the old mbuf's data (`nfsm_avail`, before
    // its length was cut); `cp2` is the fresh mbuf's data area of `MLEN >= MHLEN >= siz`
    // bytes; the two do not overlap.
    unsafe { ptr::copy_nonoverlapping(*dposp, cp2, left) };
    let mut siz2 = siz - left;
    let mut p = left;
    let mut next = mp.m_next().get();
    // Loop around copying up the siz2 bytes.
    let last = loop {
        let Some(m2) = next else {
            return Err(Errno::EBADRPC);
        };
        let xfer = siz2.min(m2.m_len().get() as usize);
        if xfer > 0 {
            // SAFETY: `xfer` bytes at `m2`'s data are its data; `p + xfer <= siz` bytes of
            // the fresh mbuf's data area (checked above); distinct mbufs.
            unsafe { ptr::copy_nonoverlapping(mtod::<u8>(m2), cp2.add(p), xfer) };
            m_data_add(m2, xfer as isize);
            m2.m_len().set(m2.m_len().get() - xfer as u32);
            p += xfer;
            siz2 -= xfer;
        }
        if siz2 == 0 {
            break m2;
        }
        next = m2.m_next().get();
    };
    mp.m_len().set(siz as u32);
    *mdp = Some(last);
    *dposp = mtod::<u8>(last);
    // SAFETY: the `siz` bytes at `cp2` were just copied into the inserted mbuf, whose data
    // they now are; the chain's owner does not write them while the view borrows its
    // cursor.
    Ok(unsafe { XdrIn::new(cp2, siz) })
}

/// `nfs_adv(mdp, dposp, offs, left)`: advances the position in the mbuf chain by `offs`
/// bytes.
pub fn nfs_adv(
    mdp: &mut Option<&'static Mbuf>,
    dposp: &mut *mut u8,
    mut offs: usize,
) -> Result<(), Errno> {
    let Some(mut m) = *mdp else {
        return Err(Errno::EBADRPC);
    };
    let mut s = nfsm_avail(m, *dposp);
    if s >= offs {
        *dposp = dposp.wrapping_add(offs);
        return Ok(());
    }
    while s < offs {
        offs -= s;
        m = m.m_next().get().ok_or(Errno::EBADRPC)?;
        s = m.m_len().get() as usize;
    }
    *mdp = Some(m);
    *dposp = mtod::<u8>(m).wrapping_add(offs);
    Ok(())
}

/// The cursor walk of `nfs_namei` (`nfs_srvsubs.c`), which reads a name byte by byte across
/// mbufs without making it contiguous: the bytes at the cursor that are in its current mbuf,
/// at most `max` of them, after skipping exhausted mbufs (`EBADRPC` at the end of the
/// chain); the cursor moves past them. A Rust helper, so that no caller dereferences
/// `dpos`.
pub fn nfsm_nextbytes<'a>(
    mdp: &'a mut Option<&'static Mbuf>,
    dposp: &'a mut *mut u8,
    max: usize,
) -> Result<XdrIn<'a>, Errno> {
    let Some(mut md) = *mdp else {
        return Err(Errno::EBADRPC);
    };
    let mut rem = nfsm_avail(md, *dposp);
    while rem == 0 && max > 0 {
        md = md.m_next().get().ok_or(Errno::EBADRPC)?;
        *mdp = Some(md);
        *dposp = mtod::<u8>(md);
        rem = md.m_len().get() as usize;
    }
    let n = rem.min(max);
    let p = *dposp;
    *dposp = p.wrapping_add(n);
    // SAFETY: `n <= rem` bytes at `p` are `md`'s data, which the chain's owner does not write
    // while the view borrows its cursor.
    Ok(unsafe { XdrIn::new(p, n) })
}

/// `nfs_init`: called once to initialize data structures: the kernel's half (the timer's
/// period and "nfsreqpl", `kernel::fs::nfs::nfs_subs::nfs_init`, N6.7a5), then the server's
/// tables (`bsd`'s until N6.7b).
pub fn nfs_init() {
    kernel::fs::nfs::nfs_subs::nfs_init::<crate::kern::proc_glue::BsdProcGlue>();
    #[cfg(feature = "nfsserver")]
    {
        // Init server data structures.
        crate::nfs::nfs_syscalls::nfsrv_init(0);
        crate::nfs::nfs_srvcache::nfsrv_initcache(); // Init the server request cache
    }
}

/// `nfsrv_errmap(nd, err)`: maps an errno to an NFS error number. For version 3 also
/// filters out error numbers not specified for the associated procedure.
pub fn nfsrv_errmap(nd: &NfsrvDescript, err: i32) -> i32 {
    if nd.nd_flag & ND_NFSV3 != 0 {
        if nd.nd_procnum <= NFSPROC_COMMIT {
            let errs = NFSRV_V3ERRMAP[nd.nd_procnum];
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
        } else {
            return err & 0xffff;
        }
    }
    if err >= 1 && err as usize <= NFSRV_V2ERRMAP.len() {
        return i32::from(NFSRV_V2ERRMAP[err as usize - 1]);
    }
    NFSERR_IO
}

/// `nfsm_build(mp, len)`: ensures a contiguous buffer `len` bytes long at the end of the
/// chain at the build cursor `*mp` (a new mbuf when the last one has no room) and returns it
/// as a view; the cursor moves to the mbuf that holds it.
pub fn nfsm_build<'a>(mp: &'a mut &'static Mbuf, len: usize) -> XdrOut<'a> {
    let mut mb = *mp;
    let mut bpos = mb_offset(mb);

    if len > m_trailingspace(mb) as usize {
        let mb2 = nfsm_mget();
        if len > MLEN {
            panic(format_args!("build > MLEN"));
        }
        let _ = m_set_next(mb, Some(mb2));
        mb = mb2;
        mb.m_len().set(0);
        bpos = mtod::<u8>(mb);
    }
    mb.m_len().set(mb.m_len().get() + len as u32);

    *mp = mb;

    // SAFETY: the `len` bytes at `bpos` were `mb`'s trailing space (checked above, or a fresh
    // mbuf of `MLEN >= len` bytes) and are now the end of its data: storage of the chain
    // being built, which the view borrows through the cursor.
    unsafe { XdrOut::new(bpos, len) }
}

/// `nfsm_srvfhtom(mp, f, v3)`: appends the server's file handle (in its `nfsfh_t`) to the
/// reply.
pub fn nfsm_srvfhtom(mp: &mut &'static Mbuf, f: &Nfsfh, v3: bool) {
    if v3 {
        nfsm_strtombuf(mp, &f.fh_bytes[..NFSX_V3FH]);
    } else {
        nfsm_buftombuf(mp, &f.fh_bytes[..NFSX_V2FH]);
    }
}

pub use kernel::fs::nfs::nfs_subs::txdr_nfsv2time;

const _: () = {
    assert!(fxdr_unsigned(nfs_xdrneg1) == u32::MAX);
    assert!(NFSV3_TYPE.len() == 9 && NV3TOV_TYPE.len() == 8);
    assert!(MHLEN <= MLEN);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests for `nfs_subs.c`: dissecting and advancing across mbuf boundaries, building
    // and growing a request chain, the XDR strings and their padding, the uio copies both ways,
    // and the version 2 times.

    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::kern::uipc_mbuf::tests::setup;
    use crate::kern::uipc_mbuf::{m_copyback, m_copydata, m_freem};
    use crate::nfs::nfsproto::{NFSERR_NOENT, NFSERR_NOTDIR, NFSPROC_GETATTR, NFSPROC_LOOKUP};
    use crate::sys::time::Timespec;
    use crate::sys::vnode::VNOVAL;

    /// An empty request mbuf (the client's `nfsm_reqhead(0)`, the kernel's since N6.7a5), for
    /// the builders' tests.
    pub(crate) fn nfsm_reqhead(_hsiz: usize) -> &'static Mbuf {
        match m_get(M_WAIT, MT_DATA) {
            Some(m) => m,
            None => panic!("an mbuf"),
        }
    }

    /// A chain of mbufs holding `parts`, one mbuf each.
    pub(crate) fn chain(parts: &[&[u8]]) -> &'static Mbuf {
        let mut head: Option<&'static Mbuf> = None;
        let mut tail: Option<&'static Mbuf> = None;
        for p in parts {
            let m = m_get(M_WAIT, MT_DATA).expect("an mbuf");
            m.m_len().set(p.len() as u32);
            m_copyback(m, 0, p, M_WAIT).expect("copyback");
            match tail {
                Some(t) => {
                    let _ = m_set_next(t, Some(m));
                }
                None => head = Some(m),
            }
            tail = Some(m);
        }
        head.expect("at least one part")
    }

    /// The lengths of a chain's mbufs.
    pub(crate) fn lens(m: &Mbuf) -> Vec<usize> {
        let mut v = vec![m.m_len().get() as usize];
        let mut n = m.m_next().get();
        while let Some(m) = n {
            v.push(m.m_len().get() as usize);
            n = m.m_next().get();
        }
        v
    }

    /// The data of a whole chain.
    pub(crate) fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut v = vec![0; lens(m).iter().sum()];
        m_copydata(m, 0, &mut v);
        v
    }

    #[test]
    fn disct_makes_straddling_bytes_contiguous() {
        let _g = setup();
        let head = chain(&[&[1, 2, 3], &[4, 5, 6, 7, 8], &[9, 10]]);
        let mut md = Some(head);
        let mut dpos = mtod::<u8>(head).wrapping_add(1);

        let v = nfsm_disct(&mut md, &mut dpos, 6).expect("six bytes");
        assert_eq!(v.bytes(), &[2, 3, 4, 5, 6, 7]);
        // The first mbuf lost its two bytes to a new one, the second its first four.
        assert_eq!(lens(head), [1, 6, 1, 2]);
        assert_eq!(bytes(head), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(nfsm_avail(md.expect("cursor"), dpos), 1);

        let v = nfsm_disct(&mut md, &mut dpos, 3).expect("three bytes");
        assert_eq!(v.bytes(), &[8, 9, 10]);
        assert_eq!(
            nfsm_disct(&mut md, &mut dpos, 1).err(),
            Some(Errno::EBADRPC)
        );
        m_freem(head);
    }

    #[test]
    fn disct_in_one_mbuf_and_past_empty_ones() {
        let _g = setup();
        let head = chain(&[&[1, 2], &[], &[3, 4, 5, 6]]);
        let mut md = Some(head);
        let mut dpos = mtod::<u8>(head).wrapping_add(2);
        let v = nfsm_disct(&mut md, &mut dpos, 4).expect("four bytes");
        assert_eq!(v.get(0), u32::from_ne_bytes([3, 4, 5, 6]));
        assert_eq!(
            lens(head),
            [2, 0, 4],
            "no copy when the bytes are contiguous"
        );
        // A short chain is EBADRPC, not a read past it.
        let mut md = Some(head);
        let mut dpos = mtod::<u8>(head);
        assert_eq!(
            nfsm_disct(&mut md, &mut dpos, 7).err(),
            Some(Errno::EBADRPC)
        );
        m_freem(head);
    }

    #[test]
    fn adv_crosses_mbufs() {
        let _g = setup();
        let head = chain(&[&[1, 2], &[3, 4, 5], &[6]]);
        let mut md = Some(head);
        let mut dpos = mtod::<u8>(head);
        nfs_adv(&mut md, &mut dpos, 4).expect("advance");
        let second = head.m_next().get().expect("second mbuf");
        assert!(core::ptr::eq(md.expect("cursor"), second));
        assert_eq!(nfsm_avail(second, dpos), 1);
        let v = nfsm_disct(&mut md, &mut dpos, 2).expect("two bytes");
        assert_eq!(v.bytes(), &[5, 6]);
        assert_eq!(nfs_adv(&mut md, &mut dpos, 1).err(), Some(Errno::EBADRPC));
        m_freem(head);
    }

    #[test]
    fn nextbytes_walks_mbuf_by_mbuf() {
        let _g = setup();
        let head = chain(&[&[1, 2], &[], &[3, 4, 5]]);
        let mut md = Some(head);
        let mut dpos = mtod::<u8>(head).wrapping_add(1);
        let mut name = Vec::new();
        while name.len() < 4 {
            let v = nfsm_nextbytes(&mut md, &mut dpos, 4 - name.len()).expect("more bytes");
            name.extend_from_slice(v.bytes());
        }
        assert_eq!(name, [2, 3, 4, 5]);
        assert_eq!(
            nfsm_nextbytes(&mut md, &mut dpos, 1).err(),
            Some(Errno::EBADRPC)
        );
        m_freem(head);
    }

    #[test]
    fn build_grows_the_chain() {
        let _g = setup();
        let head = nfsm_reqhead(0);
        let mut mb = head;
        let mut tl = nfsm_build(&mut mb, 200);
        tl.put(txdr_unsigned(7));
        tl.put_hyper(0x0102_0304_0506_0708);
        assert!(core::ptr::eq(mb, head));
        let mut tl = nfsm_build(&mut mb, 40);
        tl.set(9, nfs_true);
        assert!(!core::ptr::eq(mb, head), "a new mbuf for what does not fit");
        assert_eq!(lens(head), [200, 40]);
        let b = bytes(head);
        assert_eq!(b[..12], [0, 0, 0, 7, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(b[236..240], [0, 0, 0, 1]);
        m_freem(head);
    }

    #[test]
    fn strings_are_counted_and_padded() {
        let _g = setup();
        let head = nfsm_reqhead(0);
        let mut mb = head;
        nfsm_strtombuf(&mut mb, b"hello");
        assert_eq!(
            bytes(head),
            [0, 0, 0, 5, b'h', b'e', b'l', b'l', b'o', 0, 0, 0]
        );
        nfsm_buftombuf(&mut mb, &[9; 4]);
        assert_eq!(lens(head), [16]);
        m_freem(head);
    }

    #[test]
    fn uio_round_trip_across_mbufs() {
        let _g = setup();
        let data: Vec<u8> = (0..601).map(|i| (i * 7 + 3) as u8).collect();
        let head = nfsm_reqhead(0);
        let mut mb = head;
        nfsm_buftombuf(&mut mb, &data);
        let mut tl = nfsm_build(&mut mb, 4);
        tl.put(nfs_false);
        // 601 bytes and 3 of padding, then the word.
        assert_eq!(lens(head).iter().sum::<usize>(), 608);
        assert!(lens(head).len() > 1);

        // Read it back in two iovecs.
        let mut a = vec![0u8; 100];
        let mut b = vec![0u8; 600];
        let mut iov = [
            Iovec {
                iov_base: a.as_mut_ptr().cast(),
                iov_len: a.len(),
            },
            Iovec {
                iov_base: b.as_mut_ptr().cast(),
                iov_len: b.len(),
            },
        ];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: 601,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: None,
        };
        let mut md = Some(head);
        let mut dpos = mtod::<u8>(head);
        nfsm_mbuftouio(&mut md, &mut uio, 601, &mut dpos).expect("copy out");
        assert_eq!(uio.uio_resid, 0);
        assert_eq!(uio.uio_offset, 601);
        assert_eq!(
            uio.uio_iovcnt(),
            1,
            "the first iovec consumed, the second partly"
        );
        assert_eq!(uio.uio_iov[0].iov_len, 99);
        assert_eq!(a[..], data[..100]);
        assert_eq!(b[..501], data[100..]);
        // The padding was skipped: the cursor is at the last word.
        let v = nfsm_disct(&mut md, &mut dpos, 4).expect("the last word");
        assert_eq!(v.get(0), nfs_false);
        m_freem(head);
    }

    #[test]
    fn mbuftouio_needs_iovecs() {
        let _g = setup();
        let head = chain(&[&[1, 2, 3, 4]]);
        let mut iov: [Iovec; 0] = [];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: 4,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: None,
        };
        let mut md = Some(head);
        let mut dpos = mtod::<u8>(head);
        assert_eq!(
            nfsm_mbuftouio(&mut md, &mut uio, 4, &mut dpos).err(),
            Some(Errno::EFBIG)
        );
        m_freem(head);
    }

    #[test]
    fn file_handles() {
        let _g = setup();
        let mut fh = Nfsfh::new();
        fh.fh_bytes[0] = 0xab;
        fh.fh_bytes[NFSX_V2FH - 1] = 0xcd;
        let head = nfsm_reqhead(0);
        let mut mb = head;
        nfsm_srvfhtom(&mut mb, &fh, false);
        nfsm_srvfhtom(&mut mb, &fh, true);
        let b = bytes(head);
        assert_eq!(b.len(), NFSX_V2FH + 4 + NFSX_V3FH);
        assert_eq!(b[0], 0xab);
        assert_eq!(b[NFSX_V2FH - 1], 0xcd);
        assert_eq!(
            b[NFSX_V2FH..NFSX_V2FH + 4],
            (NFSX_V3FH as u32).to_be_bytes()
        );
        m_freem(head);
    }

    #[test]
    fn version2_times() {
        let none = txdr_nfsv2time(&Timespec::new(5, i64::from(VNOVAL)));
        assert_eq!((none.nfsv2_sec, none.nfsv2_usec), (u32::MAX, u32::MAX));
        let minus = txdr_nfsv2time(&Timespec::new(-1, 0));
        assert_eq!(fxdr_unsigned(minus.nfsv2_sec) as i32, -2);
        assert_eq!(fxdr_unsigned(minus.nfsv2_usec), 999_999);
        let t = txdr_nfsv2time(&Timespec::new(100, 5_000_999));
        assert_eq!(
            crate::nfs::xdr_subs::fxdr_nfsv2time(&t),
            Timespec::new(100, 5_000_000)
        );
    }

    #[test]
    fn tables() {
        assert_eq!(NFSV3_PROCID[NFSV2PROC_STATFS], NFSPROC_FSSTAT);
        assert_eq!(NFSV2_PROCID[NFSPROC_MKNOD_FOR_TEST], NFSV2PROC_CREATE);
        for (v2, &generic) in NFSV3_PROCID.iter().enumerate() {
            if generic != NFSPROC_NOOP {
                assert_eq!(NFSV2_PROCID[generic], v2, "procedure {generic}");
            }
        }
        assert_eq!(
            NFSRV_V2ERRMAP[Errno::ESTALE as usize - 1],
            NFSERR_STALE as u8
        );
        assert_eq!(
            NFSRV_V2ERRMAP[Errno::EACCES as usize - 1],
            NFSERR_ACCES as u8
        );
        for (p, l) in NFSRV_V3ERRMAP.iter().enumerate() {
            assert_eq!(l.last(), Some(&0), "procedure {p} ends with 0");
        }
    }

    /// `NFSPROC_MKNOD`, which version 2 maps to `NFSV2PROC_CREATE`.
    const NFSPROC_MKNOD_FOR_TEST: usize = crate::nfs::nfsproto::NFSPROC_MKNOD;

    #[test]
    fn errors_are_mapped_per_version_and_procedure() {
        let mut nd = NfsrvDescript::new();
        assert_eq!(nfsrv_errmap(&nd, Errno::ESTALE as i32), NFSERR_STALE);
        assert_eq!(nfsrv_errmap(&nd, Errno::ENOTSUP as i32), NFSERR_IO);
        assert_eq!(nfsrv_errmap(&nd, 0), NFSERR_IO);
        nd.nd_flag = ND_NFSV3;
        nd.nd_procnum = NFSPROC_LOOKUP;
        assert_eq!(nfsrv_errmap(&nd, NFSERR_NOENT), NFSERR_NOENT);
        assert_eq!(nfsrv_errmap(&nd, NFSERR_NOTDIR), NFSERR_NOTDIR);
        assert_eq!(
            nfsrv_errmap(&nd, NFSERR_PERM),
            NFSERR_IO,
            "not a LOOKUP error"
        );
        nd.nd_procnum = NFSPROC_GETATTR;
        assert_eq!(nfsrv_errmap(&nd, NFSERR_NOENT), NFSERR_IO);
        nd.nd_procnum = NFSPROC_NOOP;
        assert_eq!(
            nfsrv_errmap(&nd, 0x1_0000 | NFSERR_BADHANDLE),
            NFSERR_BADHANDLE
        );
    }
}
/* </TESTS> */
