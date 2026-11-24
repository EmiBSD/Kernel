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
 *	@(#)nfs_vfsops.c	8.12 (Berkeley) 5/20/95
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS client's mount operations in the kernel, their first part (N6.7a3): the version 3
//! FSINFO RPC (`nfs_fsinfo`), which the read, write and readdir paths call before the first
//! transfer of a mount that has not asked yet. The rest of `nfs_vfsops.c` (`nfs_mount`,
//! `mountnfs`, `nfs_unmount`, `nfs_root`, `nfs_statfs`, `nfs_sync`, the sysctls,
//! `nfs_mountroot`) comes with the switch (N6.7a5).
//!
//! Upstream: sys/nfs/nfs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_vfsops.rs@e8249449d905
//!
//! ## Deviations
//! - Kept as the C: the readdir size's clamp compares with the server's read maximum (the C
//!   reuses `max`, LZ's note).
//!
//! ## Redesign
//! - LZ's `nfs_fsinfo` took a raw `*const Ucred` and an `NfsmInfo` over the request; here it
//!   takes the credential borrowed, the reply's reader dissects the `nfsv3_fsinfo` by copy,
//!   and [`fsinfo_apply`] (LZ's) does the C's unsigned arithmetic with wrapping adds: LZ's
//!   `pref + fab - 1` panicked in a debug build for a preferred size within 511 of 2^32 that
//!   a server sends.

use core::sync::atomic::{AtomicI32, Ordering};

use hal::sys::errno::Errno;

use super::nfs::{NFS_DIRBLKSIZ, NfsError};
use super::nfs_socket::{Request, nfs_request};
use super::nfs_subs::NFSSTATS;
use super::nfsm_subs::{XdrWriter, nfsm_postop_attr};
use super::nfsmount::NfsMount;
use super::nfsnode::vtonfs;
use super::nfsproto::{NFS_FABLKSIZE, NFSPROC_FSINFO, Nfsv3Fsinfo, nfsx_fh};
use super::xdr_subs::fxdr_unsigned;
use crate::cred::Ucred;
use crate::fs::vnode::Vnode;
use crate::proc::{ProcGlue, Thread};
use crate::sys::mount::NFSMNT_GOTFSINFO;

/// What `nfs_fsinfo` does with the server's answer: shrinks the mount's `nm_wsize`,
/// `nm_rsize` and `nm_readdirsize` to the preferred and maximum transfer sizes the server gave
/// (rounded up to whole `NFS_FABLKSIZE` or `NFS_DIRBLKSIZ` blocks for a preferred size, down
/// for a maximum, the maximum itself when that rounds to 0) and marks the mount
/// `NFSMNT_GOTFSINFO`. The compares are the C's `u_int32_t` against the `int` sizes made
/// unsigned.
pub fn fsinfo_apply<G: ProcGlue>(nmp: &NfsMount<G>, fsp: &Nfsv3Fsinfo) {
    let fab = NFS_FABLKSIZE as u32;
    let dirblk = NFS_DIRBLKSIZ as u32;
    let get = |s: &AtomicI32| s.load(Ordering::Relaxed) as u32;
    let set = |s: &AtomicI32, v: u32| s.store(v as i32, Ordering::Relaxed);

    let pref = fxdr_unsigned(fsp.fs_wtpref);
    if pref < get(&nmp.nm_wsize) {
        set(&nmp.nm_wsize, pref.wrapping_add(fab - 1) & !(fab - 1));
    }
    let mut max = fxdr_unsigned(fsp.fs_wtmax);
    if max < get(&nmp.nm_wsize) {
        set(&nmp.nm_wsize, max & !(fab - 1));
        if get(&nmp.nm_wsize) == 0 {
            set(&nmp.nm_wsize, max);
        }
    }
    let pref = fxdr_unsigned(fsp.fs_rtpref);
    if pref < get(&nmp.nm_rsize) {
        set(&nmp.nm_rsize, pref.wrapping_add(fab - 1) & !(fab - 1));
    }
    max = fxdr_unsigned(fsp.fs_rtmax);
    if max < get(&nmp.nm_rsize) {
        set(&nmp.nm_rsize, max & !(fab - 1));
        if get(&nmp.nm_rsize) == 0 {
            set(&nmp.nm_rsize, max);
        }
    }
    let pref = fxdr_unsigned(fsp.fs_dtpref);
    if pref < get(&nmp.nm_readdirsize) {
        set(
            &nmp.nm_readdirsize,
            pref.wrapping_add(dirblk - 1) & !(dirblk - 1),
        );
    }
    // (`max` is still the read maximum here, as in the C.)
    if max < get(&nmp.nm_readdirsize) {
        set(&nmp.nm_readdirsize, max & !(dirblk - 1));
        if get(&nmp.nm_readdirsize) == 0 {
            set(&nmp.nm_readdirsize, max);
        }
    }
    nmp.nm_flag.fetch_or(NFSMNT_GOTFSINFO, Ordering::Relaxed);
}

/// `nfs_fsinfo(nmp, vp, cred, p)`: nfs version 3 fsinfo rpc call, on the mount `nmp` through
/// its vnode `vp` (the root's, or the operation's).
pub fn nfs_fsinfo<G: ProcGlue>(
    nmp: &NfsMount<G>,
    vp: &'static Vnode<G>,
    cred: &Ucred,
    p: Option<&Thread<G>>,
) -> Result<(), NfsError> {
    let np = vtonfs(vp);
    NFSSTATS.rpccnt[NFSPROC_FSINFO].fetch_add(1, Ordering::Relaxed);
    let mut w = XdrWriter::reqhead(nfsx_fh(true));
    let fh = np.fh();
    w.fhtom(&fh.fh, usize::try_from(fh.size).unwrap_or(0), true);

    let (reply, res) = nfs_request(
        nmp,
        vp,
        NFSPROC_FSINFO,
        Request {
            body: w,
            procp: p,
            cred,
        },
    );
    let mut x = match &reply {
        Some(r) => Some(r.reader()?),
        None => None,
    };
    let mut retattr = 0;
    nfsm_postop_attr(&mut x, &np, true, &mut retattr)?;
    res?;

    let Some(r) = x.as_mut() else {
        return Err(Errno::EBADRPC.into());
    };
    let fsp = r.pod::<Nfsv3Fsinfo>()?;
    fsinfo_apply(nmp, &fsp);
    Ok(())
}
/* </CODE> */
