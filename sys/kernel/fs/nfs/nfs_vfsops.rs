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
//! The NFS client's file system type in the kernel: the `vfsops` ([`NfsVfs`]), mounting
//! (`nfs_mount`, `mountnfs`, `nfs_decode_args`, `nfs_unmount` and the reaper), the root
//! (`nfs_root`), `nfs_statfs` and `nfs_fsinfo`, `nfs_sync`, the `vfs.nfs` sysctls
//! (`nfs_sysctl`), and mounting the root file system and the swap file over the network
//! (`nfs_mountroot`, `nfs_mount_diskless`, `option NFSCLIENT`).
//!
//! Upstream: sys/nfs/nfs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_vfsops.rs@e8249449d905
//!
//! ## Deviations
//! - `nfs_mount` reads its `struct nfs_args` out of the kernel copy of the mount arguments
//!   (`NfsArgs::from_bytes`), `EINVAL` when they are short (the C dereferences NULL); `args->fh`
//!   and `args->hostname` are user addresses copied in as the C does (LZ's). `mountnfs` takes
//!   the file handle as a slice and the server's address as its bytes (the C reads the handle
//!   through `argp->fh`, a kernel pointer for `nfs_mount_diskless` and the local copy for
//!   `nfs_mount`), and returns the root vnode (the C's `*vpp`); its `MNT_UPDATE` branch, which
//!   no caller reaches, returns `Ok(None)` (LZ's).
//! - `mountnfs`' failure path also empties the mount's slot, which the C leaves pointing at
//!   the freed `nfsmount` of a mount about to be freed (LZ's).
//! - `nfs_mount_diskless` returns the mount and its root vnode, takes the server's address
//!   from `ndm_saddr` (where the C's `ndm_args.addr` points) and the handle from `ndm_fh`, and
//!   panics as the C when the mount fails. `nfs_mountroot` takes the two things only `bsd`
//!   names, `swdevt[0]` and the machine's `swapdev` (ffs's precedent); it ignores the swap
//!   file's size (printed only under `DEBUG`) and the `#ifdef notyet` swap credentials; a
//!   failed `nfs_boot_init` panics (the C's cannot fail). `nfs_diskless` is filled on
//!   `nfs_mountroot`'s stack (it sleeps in RPCs) and kept in the glue's tables after
//!   (`NfsTables::nfs_diskless`); nothing reads `sw_vp` (LZ never wired swap on NFS: kept).
//! - `nfs_sysctl` reads `name[0]` only when there is one (`EOPNOTSUPP` otherwise);
//!   `vfs.nfs.nfsstats` is copied out of and into the kernel's counters as `hal`'s image of
//!   `struct nfsstats` (`NfsCounters::snapshot`, `restore`) (LZ's).
//! - `nfs_statfs`' credential goes on every path (the C frees it through `nfsmout`).
//! - Kept as the C: `nfs_unmount`'s `vp->v_usecount > 2` test, and `nfs_root` reached on a
//!   mount whose root was never set panics (LZ's check of the C's NULL dereference).
//! - A preferred or maximum transfer size of 0 in the server's answer leaves the mount's size
//!   as it was (`NFSMNT_GOTFSINFO` is set all the same), where the C stores 0: the next read
//!   divides by `nm_rsize` (`nfs_bioread`'s block arithmetic: a divide trap on amd64; on
//!   arm64 `UDIV` answers 0, `biosize - on` is negative and the `uiomove` reads past
//!   `b_data`), and a version 3 WRITE of 0 bytes loops for ever when the server answers a
//!   count above 0 (C:nfs/nfs_vnops.c:1291, :1380). A size of 0 is no transfer size.
//! - Kept as the C: the readdir size's clamp compares with the server's read maximum (the C
//!   reuses `max`, LZ's note).
//!
//! ## Redesign
//! - LZ's slip fixed: `f_mntonname`, `f_mntfromname` and `f_mntfromspec` are `strlcpy`'d with
//!   the C's `MNAMELEN` (C:nfs/nfs_vfsops.c:653-658, an 89-character name kept whole), where
//!   LZ's `MNAMELEN - 1` cut it to 88 (fuse's D-N5.9-10).
//! - LZ's `NFS_VFSOPS` (a `bsd` `Vfsops` of function pointers) -> `impl Vfs<G> for NfsVfs<G>`
//!   and its constant [`NfsVfs::VFS`] (fuse's shape), each slot the C's function.
//! - The mount's `struct nfsmount` is a counted `malloc(M_NFSMNT)` block (`nfsmount`'s
//!   Redesign) in the mount's typed slot, set where the C sets `mnt_data`. Its timeout holds
//!   a second handle: `timeout_set_proc(&nm_rtimeout, nfs_timer, nmp)` is
//!   `Timeout::set_shared`. `nfs_unmount` sets the timeout again with [`nfs_reaper`] and the
//!   slot gives its handle up; the reaper clears its own timeout, so the last handle drops in
//!   the softclock thread after any `nfs_timer` in flight (the C's `free` in the reaper,
//!   without the `'static` LZ's text claimed): the block cannot be freed under a timer.
//! - `mnt_stat.mount_info.nfs_args` is read and written as a whole `NfsArgs`, its bytes
//!   `hal`'s image (its holes named), where LZ built them field by field (`nfsargs_bytes`).
//! - The server's address is `NfsNam`'s bytes (`nfsmount`'s Redesign): `sockargs`' mbuf is
//!   copied into the structure and goes, so `nfs_unmount`'s `m_freem(nm_nam)` has no
//!   counterpart.
//! - `nfs_ninit` (an empty `nm_ntree`) and `TAILQ_INIT(&nm_reqsq)` are the structure's
//!   constructor (`nfs_node`'s one hash stands for every mount's tree).
//! - `nfs_sync` walks `mnt_vnodelist` with the mount's iterator and the C's `goto loop`
//!   restarts (O(1) a step); `nfs_statfs` lends the mount it holds to `nfs_fsinfo` and
//!   `nfs_request` (one count pair, the C's cast).

use core::marker::PhantomData;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::cpu::intr::{IPL_BIO, splraise};
use hal::mem::malloc::{M_WAITOK, MArc, MBox};
use hal::mem::pool::AnyArc;
use hal::sync::sleep::tsleep_nsec;
use hal::sys::dirent::MAXNAMLEN;
use hal::sys::errno::Errno;
use hal::sys::mount::{
    Fid, MNAMELEN, MNT_FORCE, MNT_LAZY, MNT_UPDATE, NFSMNT_ACDIRMAX, NFSMNT_ACDIRMIN,
    NFSMNT_ACREGMAX, NFSMNT_ACREGMIN, NFSMNT_GOTFSINFO, NFSMNT_INTERNAL, NFSMNT_MAXGRPS,
    NFSMNT_NFSV3, NFSMNT_NOCONN, NFSMNT_RDIRPLUS, NFSMNT_READAHEAD, NFSMNT_READDIRSIZE,
    NFSMNT_RETRANS, NFSMNT_RSIZE, NFSMNT_SOFT, NFSMNT_TIMEO, NFSMNT_WSIZE, NfsArgs, Statfs,
};
use hal::sys::types::{Ino, Uid};
use hal::user::UserPtr;

use super::nfs::{
    NFS_DEFRAHEAD, NFS_DIRBLKSIZ, NFS_MAXATTRTIMO, NFS_MAXGRPS, NFS_MAXRAHEAD, NFS_MAXREXMIT,
    NFS_MINATTRTIMO, NFS_NFSSTATS, NFS_NIOTHREADS, NFS_READDIRSIZE, NFS_RETRANS, NFS_RSIZE,
    NFS_WSIZE, NfsError, Nfsstats, nfs_hz, nfs_maxtimeo, nfs_mintimeo, nfs_niothreads, nfs_timeo,
};
use super::nfs_node::nfs_nget;
use super::nfs_socket::{Request, nfs_connect, nfs_disconnect, nfs_request, nfs_timer};
use super::nfs_subs::{NFSSTATS, nfs_vfs_init};
use super::nfs_syscalls::nfs_getset_niothreads;
use super::nfsm_subs::{XdrWriter, nfsm_postop_attr};
use super::nfsmount::{M_NFSMNT, NfsMount, NfsMountRef, NfsNam, vfstonfs};
use super::nfsnode::{nfstov, vtonfs};
use super::nfsproto::{
    NFS_FABLKSIZE, NFS_MAXDATA, NFS_MAXDGRAMDATA, NFS_V2MAXDATA, NFSPROC_FSINFO, NFSPROC_FSSTAT,
    NFSX_V3FHMAX, NfsStatfs, Nfsv3Fsinfo, nfsx_fh, nfsx_statfs,
};
use super::xdr_subs::{fxdr_hyper, fxdr_unsigned};
use crate::assert::kpanic;
use crate::base::sysctl::{SysctlReq, int};
use crate::fs::export::ExportGrant;
use crate::fs::mount::{Mount, Vfsconf};
use crate::fs::subr::{copy_statfs_info, vflush, vfs_getnewfsid, vget, vgone, vput, vref, vrele};
use crate::fs::vnode::{
    FORCECLOSE, LK_EXCLUSIVE, LK_RETRY, VDIR, VNON, VROOT, Vattr, Vnode, vn_lock,
};
use crate::fs::vops::{Cred, Nd, Vfs, vop_fsync, vop_getattr, vop_islocked, vop_unlock};
use crate::net::mbuf::{MT_SONAME, Mbuf};
use crate::net::sockbuf::PSOCK;
use crate::net::syscalls::sockargs;
use crate::proc::cred::{Ucred, crget};
use crate::proc::{ProcGlue, Thread};
use crate::sched::NOWAKE;
use crate::sys::socket::SOCK_DGRAM;
use crate::time::clock::sec_to_nsec;
use crate::time::timeout::{KCLOCK_NONE, TIMEOUT_PROC, Timeout};
use crate::vm::io::MAXBSIZE;

/// The NFS client of the glue `G`: `nfs_vfsops` ([`NfsVfs::VFS`]).
pub struct NfsVfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> NfsVfs<G> {
    /// `nfs_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"nfs"`).
    pub const VFS: NfsVfs<G> = NfsVfs(PhantomData);
}

/// `mp->mnt_stat.mount_info.nfs_args = *args`.
pub fn store_mount_args<G: ProcGlue>(mp: &Mount<G>, args: &NfsArgs) {
    let b = hal::user::bytes_of(core::slice::from_ref(args));
    mp.with_stat(|sp| sp.mount_info.__align[..NfsArgs::SIZE].copy_from_slice(b));
}

/// `mp->mnt_stat.mount_info.nfs_args`.
pub fn mount_args<G: ProcGlue>(mp: &Mount<G>) -> NfsArgs {
    match NfsArgs::from_bytes(&mp.stat().mount_info.__align) {
        Some(a) => a,
        None => kpanic(format_args!("nfs: mount_info is shorter than nfs_args")),
    }
}

/// `nfs_statfs(mp, sbp, p)`: nfs statfs call.
pub fn nfs_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let nmp = vfstonfs(mp);
    let v3 = nmp.flag() & NFSMNT_NFSV3 != 0;

    let vp = nfs_root(mp)?;
    // Root's credentials, no groups (`crget`'s).
    let cred = crget();
    if v3 && nmp.flag() & NFSMNT_GOTFSINFO == 0 {
        let _ = nfs_fsinfo(&nmp, vp, &cred, Some(p));
    }
    NFSSTATS.rpccnt[NFSPROC_FSSTAT].fetch_add(1, Ordering::Relaxed);
    let np = vtonfs(vp);
    let mut w = XdrWriter::reqhead(nfsx_fh(v3));
    let fh = np.fh();
    w.fhtom(&fh.fh, usize::try_from(fh.size).unwrap_or(0), v3);

    let (reply, res) = nfs_request(
        &nmp,
        vp,
        NFSPROC_FSSTAT,
        Request {
            body: w,
            procp: Some(p),
            cred: &cred,
        },
    );
    let r = (|| -> Result<(), Errno> {
        let mut x = match &reply {
            Some(r) => Some(r.reader()?),
            None => None,
        };
        if v3 {
            let mut retattr = 0;
            nfsm_postop_attr(&mut x, &np, true, &mut retattr)?;
        }
        res.map_err(NfsError::errno)?;

        let Some(x) = x.as_mut() else {
            return Err(Errno::EBADRPC);
        };
        let sfp = x.pod_len::<NfsStatfs>(nfsx_statfs(v3))?;
        sbp.f_iosize = nmp
            .nm_rsize
            .load(Ordering::Relaxed)
            .min(nmp.nm_wsize.load(Ordering::Relaxed)) as u32;
        if v3 {
            let blk = NFS_FABLKSIZE;
            sbp.f_bsize = NFS_FABLKSIZE as u32;
            sbp.f_blocks = fxdr_hyper(sfp.sf_tbytes().words()) / blk;
            sbp.f_bfree = fxdr_hyper(sfp.sf_fbytes().words()) / blk;
            sbp.f_bavail = (fxdr_hyper(sfp.sf_abytes().words()) as i64) / (blk as i64);

            sbp.f_files = fxdr_hyper(sfp.sf_tfiles().words());
            let tquad = fxdr_hyper(sfp.sf_ffiles().words());
            sbp.f_ffree = tquad;
            sbp.f_favail = tquad as i64;
        } else {
            // `fxdr_unsigned(int32_t, ..)` into the unsigned and signed 64-bit counts.
            let int = |v: u32| i64::from(fxdr_unsigned(v) as i32);
            sbp.f_bsize = fxdr_unsigned(sfp.sf_bsize());
            sbp.f_blocks = int(sfp.sf_blocks()) as u64;
            sbp.f_bfree = int(sfp.sf_bfree()) as u64;
            sbp.f_bavail = int(sfp.sf_bavail());
            sbp.f_files = 0;
            sbp.f_ffree = 0;
            sbp.f_favail = 0;
        }
        copy_statfs_info(sbp, mp);
        Ok(())
    })();
    // nfsmout:
    drop(reply);
    drop(np);
    vput(vp);
    drop(cred);
    r
}

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
    // A size the answer would make 0 keeps the mount's (the module's Deviations).
    let set = |s: &AtomicI32, v: u32| {
        if v != 0 {
            s.store(v as i32, Ordering::Relaxed);
        }
    };
    // `max & ~(blk - 1)`, or `max` itself when that is 0.
    let down = |max: u32, blk: u32| match max & !(blk - 1) {
        0 => max,
        v => v,
    };

    let pref = fxdr_unsigned(fsp.fs_wtpref);
    if pref < get(&nmp.nm_wsize) {
        set(&nmp.nm_wsize, pref.wrapping_add(fab - 1) & !(fab - 1));
    }
    let mut max = fxdr_unsigned(fsp.fs_wtmax);
    if max < get(&nmp.nm_wsize) {
        set(&nmp.nm_wsize, down(max, fab));
    }
    let pref = fxdr_unsigned(fsp.fs_rtpref);
    if pref < get(&nmp.nm_rsize) {
        set(&nmp.nm_rsize, pref.wrapping_add(fab - 1) & !(fab - 1));
    }
    max = fxdr_unsigned(fsp.fs_rtmax);
    if max < get(&nmp.nm_rsize) {
        set(&nmp.nm_rsize, down(max, fab));
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
        set(&nmp.nm_readdirsize, down(max, dirblk));
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

/// `NETDEV` (`<sys/swap.h>`): `swdevt[0]` of a swap file on NFS.
pub const NETDEV: i32 = -2;

/// `NODEV` (`<sys/param.h>`), read by the diskless boot only.
#[cfg(feature = "nfsclient")]
const NODEV: i32 = -1;

/// `nfs_mountroot()`: mount a remote root fs via NFS. It goes like this:
/// - Call `nfs_boot_init()` to fill in the `nfs_diskless` struct (using RARP, bootparam RPC,
///   mountd RPC)
/// - hand craft the swap nfs vnode hanging off a fake mount point if `swdevt[0] == NODEV`
/// - build the rootfs mount point and call `mountnfs()` to do the rest.
///
/// `swdev0` is `swdevt[0]`, `swapdev` the machine's swap device (`bsd`'s, the module's
/// Deviations).
#[cfg(feature = "nfsclient")]
pub fn nfs_mountroot<G: ProcGlue>(swdev0: &AtomicI32, swapdev: i32) -> Result<(), Errno> {
    use super::nfs_boot::{nfs_boot_getfh, nfs_boot_init};
    use super::nfsdiskless::NfsDiskless;
    use crate::base::prf::{Str, printf};
    use crate::file::FileGlue;
    use crate::fs::mount::{mountlist_insert_tail, vfs_unbusy};
    use crate::fs::subr::bdevvp;
    use crate::fs::vnode::VREG;

    G::with_curproc(|procp| {
        let Some(procp) = procp else {
            kpanic(format_args!("nfs_mountroot: no curproc"));
        };
        let mut nd = NfsDiskless::<G>::new();

        // Call nfs_boot_init() to fill in the nfs_diskless struct. Side effect: finds and
        // configures a network interface.
        if let Err(e) = nfs_boot_init(&mut nd, procp) {
            kpanic(format_args!(
                "nfs_mountroot: nfs_boot_init: error {}",
                e.as_i32()
            ));
        }

        // Create the root mount point.
        let boot = nd.nd_boot;
        if nfs_boot_getfh(procp, &boot, b"root", &mut nd.nd_root, -1).is_err() {
            kpanic(format_args!("nfs_mountroot: root"));
        }
        let (mp, vp) = nfs_mount_diskless(&nd.nd_root, b"/", 0, procp);
        printf(format_args!("root on {}\n", Str(&nd.nd_root.ndm_host)));

        // Link it into the mount list.
        mountlist_insert_tail(mp);
        <G::FileGlue as FileGlue>::set_rootvp(Some(vp));
        vfs_unbusy(mp);

        // Get root attributes (for the time).
        let mut attr = Vattr::new();
        if vop_getattr(vp, &mut attr, Cred::real(procp.ucred()), procp).is_err() {
            kpanic(format_args!("nfs_mountroot: getattr for root"));
        }
        crate::time::inittodr(attr.va_atime.tv_sec);

        // "Mount" the swap device.
        //
        // On a "dataless" configuration (swap on disk) we will have: (swdevt[0] != NODEV)
        // identifying the swap device.
        let sw0 = swdev0.load(Ordering::Relaxed);
        if sw0 != NODEV {
            match bdevvp::<G>(swapdev) {
                Ok(Some(svp)) => <G::FileGlue as FileGlue>::set_swapdev_vp(Some(svp)),
                _ => kpanic(format_args!("nfs_mountroot: can't setup swap vp")),
            }
            printf(format_args!("swap on device {sw0:#x}\n"));
            *vfs_nfs_diskless::<G>().lock() = Some(nd);
            return Ok(());
        }

        // If swapping to an nfs node: (swdevt[0] == NODEV)
        // Create a fake mount point just for the swap vnode so that the swap file can be on
        // a different server from the rootfs.
        //
        // Wait 5 retries, finally no swap is cool. -mickey
        if nfs_boot_getfh(procp, &boot, b"swap", &mut nd.nd_swap, 5).is_ok() {
            let (mp, vp) = nfs_mount_diskless(&nd.nd_swap, b"/swap", 0, procp);
            vfs_unbusy(mp);

            // Since the swap file is not the root dir of a file system, hack it to a regular
            // file.
            vp.set_vtype(VREG);
            vp.set_flag(0);

            // Next line is a hack to make swapmount() work on NFS swap files.
            swdev0.store(NETDEV, Ordering::Relaxed);
            // end hack
            nd.sw_vp = Some(vp);

            // Find out how large the swap file is (its size is printed only under DEBUG).
            if vop_getattr(vp, &mut attr, Cred::real(procp.ucred()), procp).is_err() {
                printf(format_args!("nfs_mountroot: getattr for swap\n"));
            }

            printf(format_args!("swap on {}\n", Str(&nd.nd_swap.ndm_host)));
            *vfs_nfs_diskless::<G>().lock() = Some(nd);
            return Ok(());
        }

        printf(format_args!("WARNING: no swap\n"));
        swdev0.store(NODEV, Ordering::Relaxed);
        *vfs_nfs_diskless::<G>().lock() = Some(nd);
        Ok(())
    })
}

/// `nfs_diskless` of the glue's tables.
#[cfg(feature = "nfsclient")]
fn vfs_nfs_diskless<G: ProcGlue>()
-> &'static hal::sync::mutex::Mutexed<Option<super::nfsdiskless::NfsDiskless<G>>> {
    &crate::fs::subr::vfs::<G>().nfs.nfs_diskless
}

/// `nfs_mount_diskless(ndmntp, mntname, mntflag, vpp, p)`: internal version of mount system
/// call for diskless setup. Returns the mount and its root vnode.
#[cfg(feature = "nfsclient")]
pub fn nfs_mount_diskless<G: ProcGlue>(
    ndmntp: &super::nfsdiskless::NfsDlmount,
    mntname: &[u8],
    mntflag: i32,
    p: &Thread<G>,
) -> (&'static Mount<G>, &'static Vnode<G>) {
    use crate::base::prf::Str;
    use crate::fs::subr::vfs_rootmountalloc;

    let Ok(mp) = vfs_rootmountalloc::<G>(b"nfs", mntname) else {
        kpanic(format_args!(
            "nfs_mount_diskless: vfs_rootmountalloc failed"
        ));
    };
    mp.set_flag(mp.flag() | mntflag);

    // The server's sockaddr (`ndm_saddr`, where `ndm_args.addr` points), `sa_len` bytes.
    let sin = hal::user::bytes_of(core::slice::from_ref(&ndmntp.ndm_saddr));
    let len = usize::from(ndmntp.ndm_saddr.sin_len).min(sin.len());
    let Some(nam) = NfsNam::new(&sin[..len]) else {
        kpanic(format_args!("nfs_mount_diskless: no server address"));
    };

    let fhsize = usize::try_from(ndmntp.ndm_args.fhsize)
        .unwrap_or(0)
        .min(NFSX_V3FHMAX);
    match mountnfs(
        &ndmntp.ndm_args,
        mp,
        nam,
        mntname,
        &ndmntp.ndm_host,
        &ndmntp.ndm_fh[..fhsize],
        p,
    ) {
        Ok(Some(vp)) => (mp, vp),
        Ok(None) => kpanic(format_args!(
            "nfs_mountroot: mount {} gave no root vnode",
            Str(mntname)
        )),
        Err(e) => kpanic(format_args!(
            "nfs_mountroot: mount {} failed: {}",
            Str(mntname),
            e.as_i32()
        )),
    }
}

/// The initial timeout of a mount from `timeo` (tenths of a second): `(timeo * NFS_HZ + 5) /
/// 10` ticks, clamped to `NFS_MINTIMEO`..`NFS_MAXTIMEO` (`nfs_decode_args`).
pub fn decode_timeo(timeo: i32, hz: i32, mintimeo: i32, maxtimeo: i32) -> i32 {
    let t = timeo.wrapping_mul(hz).wrapping_add(5) / 10;
    if t < mintimeo {
        mintimeo
    } else if t > maxtimeo {
        maxtimeo
    } else {
        t
    }
}

/// `MIN(v, 0xffff)` of an attribute cache bound (a `u_short`).
fn clip(v: i32) -> u16 {
    if v > 0xffff { 0xffff } else { v as u16 }
}

/// `nfs_decode_args(nmp, argp, nargp)`: applies the mount arguments `argp` to the mount
/// `nmp` and returns what the mount ended up with in `nargp`.
pub fn nfs_decode_args<G: ProcGlue>(nmp: &NfsMount<G>, argp: &NfsArgs, nargp: &mut NfsArgs) {
    let ld = |a: &AtomicI32| a.load(Ordering::Relaxed);
    let st = |a: &AtomicI32, v: i32| a.store(v, Ordering::Relaxed);

    // Also re-bind if we're switching to/from a connected UDP socket.
    let mut adjsock = (nmp.flag() & NFSMNT_NOCONN) != (argp.flags & NFSMNT_NOCONN);

    st(
        &nmp.nm_flag,
        (argp.flags & !NFSMNT_INTERNAL) | (nmp.flag() & NFSMNT_INTERNAL),
    );

    if argp.flags & NFSMNT_TIMEO != 0 && argp.timeo > 0 {
        st(
            &nmp.nm_timeo,
            decode_timeo(argp.timeo, nfs_hz(), nfs_mintimeo(), nfs_maxtimeo()),
        );
    }

    if argp.flags & NFSMNT_RETRANS != 0 && argp.retrans > 1 {
        st(&nmp.nm_retry, argp.retrans.min(NFS_MAXREXMIT));
    }
    if nmp.flag() & NFSMNT_SOFT == 0 {
        st(&nmp.nm_retry, NFS_MAXREXMIT + 1); // past clip limit
    }

    let maxio = if argp.flags & NFSMNT_NFSV3 != 0 {
        if argp.sotype == SOCK_DGRAM {
            NFS_MAXDGRAMDATA as i32
        } else {
            NFS_MAXDATA as i32
        }
    } else {
        NFS_V2MAXDATA as i32
    };
    let fab = NFS_FABLKSIZE as i32;
    let maxb = MAXBSIZE as i32;

    // The write and read sizes: rounded down to a multiple of the block size, at least one
    // block, capped by the transport and the buffer size.
    for (bit, asked, size) in [
        (NFSMNT_WSIZE, argp.wsize, &nmp.nm_wsize),
        (NFSMNT_RSIZE, argp.rsize, &nmp.nm_rsize),
    ] {
        if argp.flags & bit != 0 && asked > 0 {
            let osize = ld(size);
            // Round down to multiple of blocksize.
            let mut n = asked & !(fab - 1);
            if n <= 0 {
                n = fab;
            }
            st(size, n);
            adjsock |= n != osize;
        }
        if ld(size) > maxio {
            st(size, maxio);
        }
        if ld(size) > maxb {
            st(size, maxb);
        }
    }

    if argp.flags & NFSMNT_READDIRSIZE != 0 && argp.readdirsize > 0 {
        // Round down to multiple of blocksize.
        let mut n = argp.readdirsize & !(NFS_DIRBLKSIZ - 1);
        if n < NFS_DIRBLKSIZ {
            n = NFS_DIRBLKSIZ;
        }
        st(&nmp.nm_readdirsize, n);
    } else if argp.flags & NFSMNT_RSIZE != 0 {
        st(&nmp.nm_readdirsize, ld(&nmp.nm_rsize));
    }

    if ld(&nmp.nm_readdirsize) > maxio {
        st(&nmp.nm_readdirsize, maxio);
    }

    if argp.flags & NFSMNT_MAXGRPS != 0
        && argp.maxgrouplist >= 0
        && argp.maxgrouplist <= NFS_MAXGRPS
    {
        st(&nmp.nm_numgrps, argp.maxgrouplist);
    }
    if argp.flags & NFSMNT_READAHEAD != 0 && argp.readahead >= 0 && argp.readahead <= NFS_MAXRAHEAD
    {
        st(&nmp.nm_readahead, argp.readahead);
    }
    let ac = |a: &core::sync::atomic::AtomicU16| a.load(Ordering::Relaxed);
    if argp.flags & NFSMNT_ACREGMIN != 0 && argp.acregmin >= 0 {
        nmp.nm_acregmin
            .store(clip(argp.acregmin), Ordering::Relaxed);
    }
    if argp.flags & NFSMNT_ACREGMAX != 0 && argp.acregmax >= 0 {
        nmp.nm_acregmax
            .store(clip(argp.acregmax), Ordering::Relaxed);
    }
    if ac(&nmp.nm_acregmin) > ac(&nmp.nm_acregmax) {
        nmp.nm_acregmin
            .store(ac(&nmp.nm_acregmax), Ordering::Relaxed);
    }

    if argp.flags & NFSMNT_ACDIRMIN != 0 && argp.acdirmin >= 0 {
        nmp.nm_acdirmin
            .store(clip(argp.acdirmin), Ordering::Relaxed);
    }
    if argp.flags & NFSMNT_ACDIRMAX != 0 && argp.acdirmax >= 0 {
        nmp.nm_acdirmax
            .store(clip(argp.acdirmax), Ordering::Relaxed);
    }
    if ac(&nmp.nm_acdirmin) > ac(&nmp.nm_acdirmax) {
        nmp.nm_acdirmin
            .store(ac(&nmp.nm_acdirmax), Ordering::Relaxed);
    }

    let connected = {
        let r = hal::sync::klock::reader();
        nmp.nm_so.get(&r).is_some()
    };
    if adjsock && connected {
        nfs_disconnect(nmp);
        if ld(&nmp.nm_sotype) == SOCK_DGRAM {
            while nfs_connect(nmp, None).is_err() {
                crate::base::prf::printf(format_args!("nfs_args: retrying connect\n"));
                let _ = tsleep_nsec(&raw const NOWAKE, PSOCK, "nfscon", sec_to_nsec(1));
            }
        }
    }

    // Update nargp based on nmp.
    nargp.wsize = ld(&nmp.nm_wsize);
    nargp.rsize = ld(&nmp.nm_rsize);
    nargp.readdirsize = ld(&nmp.nm_readdirsize);
    nargp.timeo = ld(&nmp.nm_timeo);
    nargp.retrans = ld(&nmp.nm_retry);
    nargp.maxgrouplist = ld(&nmp.nm_numgrps);
    nargp.readahead = ld(&nmp.nm_readahead);
    nargp.acregmin = i32::from(ac(&nmp.nm_acregmin));
    nargp.acregmax = i32::from(ac(&nmp.nm_acregmax));
    nargp.acdirmin = i32::from(ac(&nmp.nm_acdirmin));
    nargp.acdirmax = i32::from(ac(&nmp.nm_acdirmax));
}

/// `nfs_mount(mp, path, data, ndp, p)`: VFS operations: mount system call. It seems a bit
/// dumb to copyinstr() the host here and then bcopy() it in mountnfs(), but I wanted to
/// detect errors before doing the sockargs() call because sockargs() allocates an mbuf and
/// an error after that means that I have to release the mbuf.
pub fn nfs_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    _ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let args = NfsArgs::from_bytes(data);

    if let Some(a) = &args
        && a.flags & (NFSMNT_NFSV3 | NFSMNT_RDIRPLUS) == NFSMNT_RDIRPLUS
    {
        return Err(Errno::EINVAL);
    }

    if nfs_niothreads.load(Ordering::Relaxed) < 0 {
        nfs_niothreads.store(4, Ordering::Relaxed);
        nfs_getset_niothreads::<G>(true);
    }

    if mp.flag() & MNT_UPDATE != 0 {
        let Some(nmp) = mp.node_arc::<NfsMount<G>>() else {
            return Err(Errno::EIO);
        };
        // When doing an update, we can't change from or to v3.
        if let Some(mut a) = args {
            a.flags = (a.flags & !NFSMNT_NFSV3) | (nmp.flag() & NFSMNT_NFSV3);
            let mut nargs = mount_args(mp);
            nfs_decode_args(&nmp, &a, &mut nargs);
            store_mount_args(mp, &nargs);
        }
        return Ok(());
    }
    let Some(args) = args else {
        return Err(Errno::EINVAL);
    };
    let fhsize = match usize::try_from(args.fhsize) {
        Ok(n) if n <= NFSX_V3FHMAX => n,
        _ => return Err(Errno::EINVAL),
    };
    let mut nfh = [0u8; NFSX_V3FHMAX];
    UserPtr::<u8>::new(args.fh).copyin_slice(&mut nfh[..fhsize])?;
    // The rest of the name zero (the C's memset after the copy).
    let mut hst = [0u8; MNAMELEN];
    UserPtr::<u8>::new(args.hostname).copyinstr(&mut hst[..MNAMELEN - 1])?;
    // sockargs() call must be after above copyin() calls. (`addrlen` an `int` made a
    // `size_t`, as the C's prototype converts it: a negative one is refused by the bound.)
    let nam = sockargs(UserPtr::new(args.addr), args.addrlen as usize, MT_SONAME)?;
    // The name's bytes; `sockargs` bounds a name to `UCHAR_MAX`, `NfsNam`'s room.
    let Some(nam) = NfsNam::new(nam.data()) else {
        return Err(Errno::EINVAL);
    };
    mountnfs(&args, mp, nam, path, &hst, &nfh[..fhsize], p)?;
    Ok(())
}

/// `mountnfs(argp, mp, nam, pth, hst, vpp, p)`: common code for mount and mountroot. `fh` is
/// the root file handle (`argp->fh`, `argp->fhsize` bytes), `nam` the server's address;
/// returns the root vnode, unlocked and referenced for the mount.
pub fn mountnfs<G: ProcGlue>(
    argp: &NfsArgs,
    mp: &'static Mount<G>,
    nam: NfsNam,
    pth: &[u8],
    hst: &[u8],
    fh: &[u8],
    p: &Thread<G>,
) -> Result<Option<&'static Vnode<G>>, Errno> {
    if mp.flag() & MNT_UPDATE != 0 {
        // update paths, file handles, etc, here	XXX
        return Ok(None);
    }
    let Some(to) = MBox::new(M_NFSMNT, M_WAITOK, Timeout::unset()) else {
        kpanic(format_args!("mountnfs: malloc(M_WAITOK) failed"));
    };
    let Some(nmp) = MArc::new(M_NFSMNT, M_WAITOK, NfsMount::new(mp, nam, to)) else {
        kpanic(format_args!("mountnfs: malloc(M_WAITOK) failed"));
    };
    mp.set_node(MArc::erase(nmp.clone()));

    vfs_getnewfsid(mp);
    let st = |a: &AtomicI32, v: i32| a.store(v, Ordering::Relaxed);
    st(&nmp.nm_timeo, nfs_timeo());
    st(&nmp.nm_retry, NFS_RETRANS);
    st(&nmp.nm_wsize, NFS_WSIZE);
    st(&nmp.nm_rsize, NFS_RSIZE);
    st(&nmp.nm_readdirsize, NFS_READDIRSIZE);
    st(&nmp.nm_numgrps, NFS_MAXGRPS);
    st(&nmp.nm_readahead, NFS_DEFRAHEAD);
    for (a, v) in [
        (&nmp.nm_acregmin, NFS_MINATTRTIMO),
        (&nmp.nm_acregmax, NFS_MAXATTRTIMO),
        (&nmp.nm_acdirmin, NFS_MINATTRTIMO),
        (&nmp.nm_acdirmax, NFS_MAXATTRTIMO),
    ] {
        a.store(v as u16, Ordering::Relaxed);
    }
    mp.with_stat(|sp| {
        sp.f_namemax = MAXNAMLEN as u32;
        sp.f_mntonname.fill(0);
        libkern::strlcpy(&mut sp.f_mntonname, pth);
        sp.f_mntfromname.fill(0);
        libkern::strlcpy(&mut sp.f_mntfromname, hst);
        sp.f_mntfromspec.fill(0);
        libkern::strlcpy(&mut sp.f_mntfromspec, hst);
    });
    let mut nargs = *argp;
    nargs.addr = 0;
    nargs.fh = 0;
    nargs.hostname = 0;
    nfs_decode_args(&nmp, argp, &mut nargs);
    store_mount_args(mp, &nargs);

    // nfs_ninit, TAILQ_INIT(&nm_reqsq): the structure's constructor.
    nmp.nm_rtimeout.set_shared(
        nfs_timer::<G>,
        MArc::erase(nmp.clone()),
        KCLOCK_NONE,
        TIMEOUT_PROC,
    );

    // Set up the sockets and per-host congestion.
    st(&nmp.nm_sotype, argp.sotype);
    st(&nmp.nm_soproto, argp.proto);

    let bad = |nmp: NfsMountRef<G>, error: Errno| -> Result<Option<&'static Vnode<G>>, Errno> {
        nfs_disconnect(&nmp);
        // free(nmp): the timeout's handle and the slot's go, ours last.
        nmp.nm_rtimeout.del();
        nmp.nm_rtimeout.clear();
        drop(mp.take_node());
        drop(nmp);
        Err(error)
    };

    // For Connection based sockets (TCP,...) defer the connect until the first request, in
    // case the server is not responding.
    if argp.sotype == SOCK_DGRAM
        && let Err(e) = nfs_connect(&nmp, None)
    {
        return bad(nmp, e);
    }

    // This is silly, but it has to be set so that vinifod() works. We do not want to do an
    // nfs_statfs() here since we can get stuck on a dead server and we are holding a lock on
    // the mount point.
    mp.with_stat(|sp| sp.f_iosize = NFS_MAXDGRAMDATA as u32);
    let np = match nfs_nget(mp, fh) {
        Ok(np) => np,
        Err(e) => return bad(nmp, e),
    };
    let vp = nfstov(&np);
    drop(np);
    let mut attr = Vattr::new();
    if let Err(e) = vop_getattr(vp, &mut attr, Cred::real(p.ucred()), p) {
        vput(vp);
        return bad(nmp, e);
    }

    // A reference count is needed on the nfsnode representing the remote root. If this
    // object is not persistent, then backward traversals of the mount point (i.e. "..") will
    // not work if the nfsnode gets flushed out of the cache. Ufs does not have this problem,
    // because one can identify root inodes by their number == ROOTINO (2). So, just unlock,
    // but no rele.
    let _ = nmp.nm_vnode.set(vp);
    if vp.vtype() == VNON {
        vp.set_vtype(VDIR);
    }
    vp.set_flag(VROOT);
    let _ = vop_unlock(vp);

    Ok(Some(vp))
}

/// `nfs_unmount(mp, mntflags, p)`: unmount system call.
pub fn nfs_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let nmp = vfstonfs(mp);
    let mut flags = 0;

    let vp = nfs_root(mp)?;

    if mntflags & MNT_FORCE == 0 && vp.usecount() > 2 {
        vput(vp);
        return Err(Errno::EBUSY);
    }

    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }

    if let Err(e) = vflush(mp, Some(vp), flags) {
        vput(vp);
        return Err(e);
    }

    // There are two references count to get rid of here: one from mountnfs() and one from
    // nfs_root() above.
    vrele(vp);
    vput(vp);
    vgone(vp);
    nfs_disconnect(&nmp);
    // m_freem(nmp->nm_nam): the address's bytes go with the structure.
    nmp.nm_rtimeout.del();
    nmp.nm_rtimeout.set_shared(
        nfs_reaper::<G>,
        MArc::erase(nmp.clone()),
        KCLOCK_NONE,
        TIMEOUT_PROC,
    );
    nmp.nm_rtimeout.as_pin().add(0);
    // mp->mnt_data = NULL: the slot's handle goes; the timeout's is the last but ours.
    drop(mp.take_node());
    Ok(())
}

/// `nfs_reaper(arg)`: delay nfs mount point free until pending or sleeping timeouts have
/// finished: the timeout gives its handle back (the softclock thread's copy of it, dropped
/// after this returns, is then the last).
pub fn nfs_reaper<G: ProcGlue>(arg: &AnyArc) {
    let Some(nmp) = arg.downcast_ref::<NfsMount<G>>() else {
        kpanic(format_args!("nfs_reaper: not an nfs mount"));
    };
    nmp.nm_rtimeout.clear();
}

/// `nfs_root(mp, vpp)`: return root of a filesystem, referenced and locked.
pub fn nfs_root<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    let nmp = vfstonfs(mp);
    let Some(&vp) = nmp.nm_vnode.get() else {
        kpanic(format_args!("nfs_root: mount without a root vnode"));
    };
    drop(nmp);
    vref(vp);
    if let Err(e) = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY) {
        vrele(vp);
        return Err(e);
    }
    Ok(vp)
}

/// `nfs_sync(mp, waitfor, stall, cred, p)`: flush out the buffer cache.
pub fn nfs_sync<G: ProcGlue>(
    mp: &'static Mount<G>,
    waitfor: i32,
    _stall: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut allerror = Ok(());

    // Don't traverse the vnode list if we want to skip all of them.
    if waitfor == MNT_LAZY {
        return allerror;
    }

    // Force stale buffer cache information to be flushed.
    'restart: loop {
        for vp in mp.vnodes() {
            // If the vnode that we are about to sync is no longer associated with this mount
            // point, start over.
            if !vp.mount().is_some_and(|m| core::ptr::eq(m, mp)) {
                continue 'restart;
            }
            if vop_islocked(vp) != 0 {
                continue;
            }
            let s = splraise(IPL_BIO);
            let empty = vp.bufs.dirty_is_empty();
            drop(s);
            if empty {
                continue;
            }
            if vget(vp, LK_EXCLUSIVE).is_err() {
                continue 'restart;
            }
            if let Err(e) = vop_fsync(vp, cred, waitfor, p) {
                allerror = Err(e);
            }
            vput(vp);
        }
        break;
    }

    allerror
}

/// `nfs_vget(mp, ino, vpp)`: NFS flat namespace lookup. Currently unsupported.
pub fn nfs_vget<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _ino: Ino,
) -> Result<&'static Vnode<G>, Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `nfs_sysctl(name, namelen, oldp, oldlenp, newp, newlen, p)`: do that sysctl thang...
/// (`vfs.nfs`: the statistics and the number of nfsiods).
pub fn nfs_sysctl<G: ProcGlue>(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    const STATS_SIZE: usize = size_of::<Nfsstats>();
    // All names at this level are terminal.
    if name.len() > 1 {
        return Err(Errno::ENOTDIR); // overloaded
    }
    let Some(&first) = name.first() else {
        return Err(Errno::EOPNOTSUPP);
    };

    match first {
        NFS_NFSSTATS => {
            if oldp == 0 {
                *oldlenp = STATS_SIZE;
                return Ok(());
            }

            if *oldlenp < STATS_SIZE {
                *oldlenp = STATS_SIZE;
                return Err(Errno::ENOMEM);
            }

            UserPtr::<Nfsstats>::new(oldp).copyout(&NFSSTATS.snapshot())?;

            if newp != 0 && newlen != STATS_SIZE {
                return Err(Errno::EINVAL);
            }

            if newp != 0 {
                let st = UserPtr::<Nfsstats>::new(newp).copyin()?;
                NFSSTATS.restore(&st);
            }
            Ok(())
        }
        NFS_NIOTHREADS => {
            nfs_getset_niothreads::<G>(false);

            let mut req = SysctlReq {
                oldp,
                oldlenp,
                newp,
                newlen,
            };
            let rv = int(&mut req, &nfs_niothreads);
            if newp != 0 {
                nfs_getset_niothreads::<G>(true);
            }

            rv
        }
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// `nfs_fhtovp(mp, fhp, vpp)`: at this point, this should never happen.
pub fn nfs_fhtovp<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _fhp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    Err(Errno::EINVAL)
}

/// `nfs_vptofh(vp, fhp)`: vnode pointer to file handle, should never happen either.
pub fn nfs_vptofh<G: ProcGlue>(_vp: &'static Vnode<G>, _fhp: &mut Fid) -> Result<(), Errno> {
    Err(Errno::EINVAL)
}

/// `nfs_start(mp, flags, p)`: vfs start routine, a no-op.
pub fn nfs_start() -> Result<(), Errno> {
    Ok(())
}

/// `nfs_quotactl(mp, cmd, uid, arg, p)`: do operations associated with quotas, not supported.
pub fn nfs_quotactl() -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `nfs_checkexp(mp, nam, exflagsp, credanonp)`: check export permission, not supported.
pub fn nfs_checkexp() -> Result<ExportGrant, Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `nfs_vfsops`: each slot the C's function of the same name.
impl<G: ProcGlue> Vfs<G> for NfsVfs<G> {
    /// `vfs_mount`: [`nfs_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfs_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: [`nfs_start`].
    fn start(&self, _mp: &'static Mount<G>, _flags: i32, _p: &Thread<G>) -> Result<(), Errno> {
        nfs_start()
    }

    /// `vfs_unmount`: [`nfs_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        nfs_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: [`nfs_root`].
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        nfs_root(mp)
    }

    /// `vfs_quotactl`: [`nfs_quotactl`].
    fn quotactl(
        &self,
        _mp: &'static Mount<G>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfs_quotactl()
    }

    /// `vfs_statfs`: [`nfs_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        nfs_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`nfs_sync`].
    fn sync(
        &self,
        mp: &'static Mount<G>,
        waitfor: i32,
        stall: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfs_sync(mp, waitfor, stall, cred, p)
    }

    /// `vfs_vget`: [`nfs_vget`].
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        nfs_vget(mp, ino)
    }

    /// `vfs_fhtovp`: [`nfs_fhtovp`].
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        nfs_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: [`nfs_vptofh`].
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        nfs_vptofh(vp, fid)
    }

    /// `vfs_init`: `nfs_vfs_init` (`nfs_subs`).
    fn init(&self, vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        nfs_vfs_init(vfc)
    }

    /// `vfs_sysctl`: [`nfs_sysctl`] (`#ifndef SMALL_KERNEL`).
    fn sysctl(
        &self,
        name: &[i32],
        oldp: usize,
        oldlenp: &mut usize,
        newp: usize,
        newlen: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        nfs_sysctl::<G>(name, oldp, oldlenp, newp, newlen, p)
    }

    /// `vfs_checkexp`: [`nfs_checkexp`].
    fn checkexp(&self, _mp: &'static Mount<G>, _nam: &Mbuf) -> Result<ExportGrant, Errno> {
        nfs_checkexp()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of `nfs_vfsops.c`: the timeout arithmetic and `nfs_decode_args`, the
    // `nfs_args` image in `mount_info`, the `vfs.nfs` sysctl's errors, the operations that
    // only answer an errno, and a whole mount against testnfs's scripted server: `mountnfs`,
    // `nfs_statfs` (with its FSINFO), `nfs_unmount` with the timer armed, the reaper.

    use std::boxed::Box;
    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::fs::nfs::nfs_node::test_mount;
    use crate::fs::nfs::testnfs::{self as tn, Out};
    use crate::fs::testfs::TestGlue as G;
    use crate::net::mbuf::MBUFPL;
    use crate::sys::mount::{NFS_ARGSVERSION, NFSMNT_AUTHERR};
    use crate::sys::socket::SOCK_STREAM;

    /// Mount arguments with every field zero but the version, the flags and the socket type.
    fn args(flags: i32, sotype: i32) -> NfsArgs {
        let mut a = hal::user::zeroed::<NfsArgs>();
        a.version = NFS_ARGSVERSION;
        a.flags = flags;
        a.sotype = sotype;
        a
    }

    /// A mount structure with `mountnfs`' defaults.
    fn defaults(nmp: &NfsMount<G>) {
        for (a, v) in [
            (&nmp.nm_timeo, 1),
            (&nmp.nm_retry, NFS_RETRANS),
            (&nmp.nm_wsize, NFS_WSIZE),
            (&nmp.nm_rsize, NFS_RSIZE),
            (&nmp.nm_readdirsize, NFS_READDIRSIZE),
            (&nmp.nm_numgrps, NFS_MAXGRPS),
            (&nmp.nm_readahead, NFS_DEFRAHEAD),
        ] {
            a.store(v, Ordering::Relaxed);
        }
        nmp.nm_flag.store(0, Ordering::Relaxed);
    }

    /// `nfs_decode_args` with the kernel lock held (its socket test reads under it).
    fn decode(nmp: &NfsMount<G>, a: &NfsArgs, out: &mut NfsArgs) {
        hal::sync::klock::kernel_lock();
        nfs_decode_args(nmp, a, out);
        hal::sync::klock::kernel_unlock();
    }

    fn ld(a: &AtomicI32) -> i32 {
        a.load(Ordering::Relaxed)
    }

    #[test]
    fn timeout_is_tenths_of_a_second_in_ticks_and_clamped() {
        // 1 second at hz 100.
        assert_eq!(decode_timeo(10, 100, 100, 6000), 100);
        // (3 * 100 + 5) / 10 = 30, below the minimum.
        assert_eq!(decode_timeo(3, 100, 100, 6000), 100);
        // 20 seconds stays; 100 seconds is capped at the maximum.
        assert_eq!(decode_timeo(200, 100, 100, 6000), 2000);
        assert_eq!(decode_timeo(1000, 100, 100, 6000), 6000);
        // The +5 rounds half a tick up.
        assert_eq!(decode_timeo(1, 5, 0, 600), 1);
        assert_eq!(decode_timeo(15, 10, 1, 600), 15);
    }

    #[test]
    fn decode_args_rounds_and_caps_the_sizes() {
        let tm = test_mount(0);
        let nmp = &tm.nmp;
        defaults(nmp);
        let mut out = args(0, SOCK_DGRAM);
        let fab = NFS_FABLKSIZE as i32;
        // Version 3 over UDP: the largest datagram; sizes round down to the block size.
        let mut a = args(
            NFSMNT_NFSV3 | NFSMNT_WSIZE | NFSMNT_RSIZE | NFSMNT_READDIRSIZE,
            SOCK_DGRAM,
        );
        a.wsize = 50000;
        a.rsize = 20000;
        a.readdirsize = 3000;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_wsize), NFS_MAXDGRAMDATA as i32);
        assert_eq!(ld(&nmp.nm_rsize), 20000 & !(fab - 1));
        assert_eq!(ld(&nmp.nm_readdirsize), 3000 & !(NFS_DIRBLKSIZ - 1));
        assert_eq!(
            (out.wsize, out.rsize, out.readdirsize),
            (
                ld(&nmp.nm_wsize),
                ld(&nmp.nm_rsize),
                ld(&nmp.nm_readdirsize)
            )
        );
        // Too small to hold a block: one block (readdir: one directory block).
        a.wsize = 100;
        a.rsize = 1;
        a.readdirsize = 5;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_wsize), fab);
        assert_eq!(ld(&nmp.nm_rsize), fab);
        assert_eq!(ld(&nmp.nm_readdirsize), NFS_DIRBLKSIZ);
        // Version 2: 8192 at most, whatever the socket; v3 over TCP may use NFS_MAXDATA.
        let mut a = args(NFSMNT_WSIZE | NFSMNT_RSIZE, SOCK_STREAM);
        a.wsize = 30000;
        a.rsize = 30000;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_wsize), NFS_V2MAXDATA as i32);
        assert_eq!(ld(&nmp.nm_rsize), NFS_V2MAXDATA as i32);
        // The readdir size follows the read size when only that was given.
        assert_eq!(ld(&nmp.nm_readdirsize), NFS_V2MAXDATA as i32);
        let mut a = args(NFSMNT_NFSV3 | NFSMNT_WSIZE, SOCK_STREAM);
        a.wsize = 60000;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_wsize), 60000 & !(fab - 1));
    }

    #[test]
    fn decode_args_retries_flags_and_attribute_cache() {
        let tm = test_mount(0);
        let nmp = &tm.nmp;
        defaults(nmp);
        let mut out = args(0, SOCK_DGRAM);
        // A hard mount retries "forever" (past the clip limit), whatever it was told.
        let mut a = args(NFSMNT_RETRANS, SOCK_DGRAM);
        a.retrans = 5;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_retry), NFS_MAXREXMIT + 1);
        // A soft one takes the count, capped; 1 is ignored.
        let mut a = args(NFSMNT_SOFT | NFSMNT_RETRANS, SOCK_DGRAM);
        a.retrans = 5;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_retry), 5);
        a.retrans = 100_000;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_retry), NFS_MAXREXMIT);
        a.retrans = 1;
        decode(nmp, &a, &mut out);
        assert_eq!(out.retrans, NFS_MAXREXMIT);
        // The kernel's own flag bits survive, the caller's are dropped.
        nmp.nm_flag
            .store(NFSMNT_GOTFSINFO | NFSMNT_SOFT, Ordering::Relaxed);
        let a = args(NFSMNT_NFSV3 | NFSMNT_GOTFSINFO | NFSMNT_AUTHERR, SOCK_DGRAM);
        decode(nmp, &a, &mut out);
        assert_eq!(nmp.flag(), NFSMNT_NFSV3 | NFSMNT_GOTFSINFO);
        // The timeout is tenths of a second in the timer's ticks.
        let mut a = args(NFSMNT_TIMEO, SOCK_DGRAM);
        a.timeo = 10;
        decode(nmp, &a, &mut out);
        assert_eq!(
            out.timeo,
            decode_timeo(10, nfs_hz(), nfs_mintimeo(), nfs_maxtimeo())
        );
        // Group list and read-ahead are range checked.
        let mut a = args(NFSMNT_MAXGRPS | NFSMNT_READAHEAD, SOCK_DGRAM);
        a.maxgrouplist = NFS_MAXGRPS + 1;
        a.readahead = NFS_MAXRAHEAD + 1;
        decode(nmp, &a, &mut out);
        assert_eq!(ld(&nmp.nm_numgrps), NFS_MAXGRPS);
        assert_eq!(ld(&nmp.nm_readahead), NFS_DEFRAHEAD);
        a.maxgrouplist = 3;
        a.readahead = 2;
        decode(nmp, &a, &mut out);
        assert_eq!((out.maxgrouplist, out.readahead), (3, 2));
        // Attribute cache times clip to 16 bits and the minimum never exceeds the maximum.
        let mut a = args(
            NFSMNT_ACREGMIN | NFSMNT_ACREGMAX | NFSMNT_ACDIRMIN | NFSMNT_ACDIRMAX,
            SOCK_DGRAM,
        );
        a.acregmin = 100_000;
        a.acregmax = 30;
        a.acdirmin = 7;
        a.acdirmax = 1_000_000;
        decode(nmp, &a, &mut out);
        assert_eq!((out.acregmin, out.acregmax), (30, 30));
        assert_eq!((out.acdirmin, out.acdirmax), (7, 0xffff));
        // Negative values are ignored.
        a.acregmin = -1;
        a.acdirmax = -5;
        decode(nmp, &a, &mut out);
        assert_eq!((out.acregmin, out.acdirmax), (30, 0xffff));
    }

    /// `mount_info` holds the arguments' image, its holes zero, and reads back whole.
    #[test]
    fn mount_info_holds_the_args_image() {
        let tm = test_mount(0);
        let mut a = args(NFSMNT_NFSV3 | NFSMNT_SOFT, SOCK_STREAM);
        a.addrlen = 16;
        a.proto = 6;
        a.fhsize = 28;
        a.wsize = 8192;
        a.timeo = 600;
        a.acdirmax = 60;
        store_mount_args(tm.mp, &a);
        let b = tm.mp.stat().mount_info.__align;
        assert_eq!(b[..4], NFS_ARGSVERSION.to_ne_bytes());
        assert_eq!(b[4..8], [0; 4], "the hole after version");
        let back = mount_args(tm.mp);
        assert_eq!(
            hal::user::bytes_of(core::slice::from_ref(&back)),
            hal::user::bytes_of(core::slice::from_ref(&a))
        );
    }

    /// The sysctl's errors: a deeper name, an unknown one, a short buffer for the statistics
    /// (`ENOMEM` with the size), the size alone for no buffer.
    #[test]
    fn sysctl_errors_and_the_statistics_size() {
        let tm = test_mount(0);
        let p = tm.p;
        let mut len = 0;
        assert_eq!(
            nfs_sysctl::<G>(&[NFS_NFSSTATS, 1], 0, &mut len, 0, 0, p),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            nfs_sysctl::<G>(&[99], 0, &mut len, 0, 0, p),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            nfs_sysctl::<G>(&[], 0, &mut len, 0, 0, p),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            nfs_sysctl::<G>(&[NFS_NFSSTATS], 0, &mut len, 0, 0, p),
            Ok(())
        );
        assert_eq!(len, size_of::<Nfsstats>());
        assert_eq!(len, 624);
        let mut short = 8;
        assert_eq!(
            nfs_sysctl::<G>(&[NFS_NFSSTATS], 0x1000, &mut short, 0, 0, p),
            Err(Errno::ENOMEM)
        );
        assert_eq!(short, 624);
    }

    /// `args` as the kernel copy `mount(2)` makes of the user's `struct nfs_args`.
    fn data_of(a: &NfsArgs) -> Vec<u8> {
        hal::user::bytes_of(core::slice::from_ref(a)).to_vec()
    }

    /// `nfs_mount` on `mp` with the kernel copy `data`, the nfsiods asked for already (no
    /// thread is started: `nfs_niothreads` made non-negative).
    fn mount(mp: &'static Mount<G>, p: &Thread<G>, data: &mut [u8]) -> Result<(), Errno> {
        let _ = nfs_niothreads.fetch_max(0, Ordering::Relaxed);
        let mut nd = crate::fs::lookup::ndinit(
            crate::fs::namei::LOOKUP,
            crate::fs::namei::FOLLOW,
            crate::fs::namei::NiDirp::Sys(b"/mnt"),
            p,
        );
        nfs_mount(mp, b"/mnt", data, &mut nd, p)
    }

    /// `mount(2)`'s argument checks, each before any copy in, in the C's order: `RDIRPLUS`
    /// without version 3, arguments shorter than `struct nfs_args` (the C's NULL), a handle
    /// size below 0 or above `NFSX_V3FHMAX`, an update of a mount with no NFS structure; then,
    /// after the handle and the host name are copied in, a server address longer than
    /// `UCHAR_MAX` or negative (`sockargs`).
    #[test]
    fn mount_refuses_bad_arguments() {
        let tm = test_mount(0);
        let (mp, p) = (tm.mp, tm.p);
        let mut a = args(NFSMNT_RDIRPLUS, SOCK_DGRAM);
        assert_eq!(mount(mp, p, &mut data_of(&a)), Err(Errno::EINVAL));
        a.flags = NFSMNT_NFSV3 | NFSMNT_RDIRPLUS;
        a.fhsize = -1;
        assert_eq!(mount(mp, p, &mut data_of(&a)), Err(Errno::EINVAL));
        a.fhsize = NFSX_V3FHMAX as i32 + 1;
        assert_eq!(mount(mp, p, &mut data_of(&a)), Err(Errno::EINVAL));
        assert_eq!(mount(mp, p, &mut [0u8; 8]), Err(Errno::EINVAL));

        // An update of a mount whose slot holds no NFS structure.
        let fresh = crate::fs::mount::vfs_mount_alloc(None, mp.vfc());
        crate::fs::mount::vfs_unbusy(fresh);
        fresh.set_flag(fresh.flag() | MNT_UPDATE);
        a.fhsize = 0;
        assert_eq!(mount(fresh, p, &mut data_of(&a)), Err(Errno::EIO));

        // The address's length, after the handle (`fhsize` 64, the bound kept) and the host
        // name are copied in from user memory (testfs's slots).
        a.fhsize = NFSX_V3FHMAX as i32;
        a.fh = crate::fs::testfs::up(2, &[7u8; 64]).addr();
        a.hostname = crate::fs::testfs::up(3, b"127.0.0.1:/x").addr();
        for len in [256, -1] {
            a.addrlen = len;
            assert_eq!(mount(mp, p, &mut data_of(&a)), Err(Errno::EINVAL), "{len}");
        }
    }

    /// The operations that only answer an errno, as the C's.
    #[test]
    fn operations_that_only_fail_or_do_nothing() {
        let tm = test_mount(0);
        let v = NfsVfs::<G>::VFS;
        assert_eq!(v.start(tm.mp, 0, tm.p), Ok(()));
        assert_eq!(v.quotactl(tm.mp, 0, 0, 0, tm.p), Err(Errno::EOPNOTSUPP));
        assert_eq!(v.vget(tm.mp, 2).err(), Some(Errno::EOPNOTSUPP));
        let fid = hal::user::zeroed::<Fid>();
        assert_eq!(v.fhtovp(tm.mp, &fid).err(), Some(Errno::EINVAL));
        let m = crate::net::mbuf::m_get(crate::net::mbuf::M_WAIT, MT_SONAME).expect("an mbuf");
        assert_eq!(v.checkexp(tm.mp, &m).err(), Some(Errno::EOPNOTSUPP));
    }

    /// A version 3 `fattr3` of a directory.
    fn fattr3_dir() -> Vec<u8> {
        let mut v = Vec::new();
        for x in [2u32, 0o755, 2, 0, 0] {
            v.extend_from_slice(&x.to_be_bytes());
        }
        for q in [512u64, 512, 0, 7, 2] {
            v.extend_from_slice(&q.to_be_bytes());
        }
        for x in [1000u32, 0, 1000, 0, 1000, 0] {
            v.extend_from_slice(&x.to_be_bytes());
        }
        v
    }

    /// A whole mount: `mountnfs` connects, makes the root's node and asks its attributes;
    /// `nfs_statfs` asks FSINFO once then FSSTAT; `nfs_unmount` with the timer's timeout
    /// set gives the slot's handle up and leaves the timeout to the reaper, whose run gives
    /// the timeout's back: the caller's handle is then the last.
    #[test]
    fn mount_statfs_unmount_and_the_reaper() {
        let fx = tn::setup(0);
        if crate::time::tc::gettime() == 0 {
            crate::time::tc::TIME_SECOND.store(1_000_000, Ordering::Relaxed);
        }
        let mbufs = MBUFPL.nout();
        nfs_vfs_init::<tn::G>(fx.mp.vfc()).expect("nfs_vfs_init");
        let mp = crate::fs::mount::vfs_mount_alloc(None, fx.mp.vfc());
        crate::fs::mount::vfs_unbusy(mp);
        // The peer's timer mount (its `Out::Tick`s, unused here), out of the slot again.
        let dummy = tn::nfsmount(&fx, SOCK_DGRAM, 0);
        drop(fx.mp.take_node());
        let peer = tn::Peer::start(
            &fx,
            SOCK_DGRAM,
            &dummy,
            Box::new(|_, c| {
                let pr = u32::from_be_bytes(c[20..24].try_into().unwrap()) as usize;
                let mut body = Vec::new();
                match pr {
                    p if p == super::super::nfsproto::NFSPROC_GETATTR => body = fattr3_dir(),
                    NFSPROC_FSINFO => {
                        body.extend_from_slice(&0u32.to_be_bytes());
                        for x in [65536u32, 65536, 512, 65536, 65536, 512, 65536] {
                            body.extend_from_slice(&x.to_be_bytes());
                        }
                        body.extend_from_slice(&[0; 20]);
                    }
                    NFSPROC_FSSTAT => {
                        body.extend_from_slice(&0u32.to_be_bytes());
                        for q in [1u64 << 30, 1 << 29, 1 << 28, 1000, 600, 600] {
                            body.extend_from_slice(&q.to_be_bytes());
                        }
                        body.extend_from_slice(&0u32.to_be_bytes());
                    }
                    _ => panic!("procedure {pr}"),
                }
                vec![Out::Data(tn::reply(c, 0, 0, 0, &body))]
            }),
        );

        let a = args(NFSMNT_NFSV3, SOCK_DGRAM);
        let sa = tn::lo_sin(tn::PEER_PORT);
        let nam = NfsNam::new(super::super::xdr_subs::xdr_bytes(&sa)).expect("an address");
        let vp = mountnfs(
            &a,
            mp,
            nam,
            b"/mnt",
            b"127.0.0.1:/x",
            &[1, 2, 3, 4],
            fx.user,
        )
        .expect("mounted")
        .expect("a root");
        assert_eq!(vp.vtype(), VDIR);
        assert_ne!(vp.flag() & VROOT, 0);
        assert_eq!(vp.usecount(), 1, "the mount's reference");
        let st = mp.stat();
        assert_eq!(&st.f_mntonname[..5], b"/mnt\0");
        assert_eq!(&st.f_mntfromname[..13], b"127.0.0.1:/x\0");
        assert_eq!(st.f_namemax, MAXNAMLEN as u32);
        let back = mount_args(mp);
        assert_eq!((back.addr, back.fh, back.hostname), (0, 0, 0));
        assert_eq!(back.wsize, NFS_WSIZE);

        let mut sb = mp.stat();
        assert_eq!(nfs_statfs(mp, &mut sb, fx.user), Ok(()));
        assert_eq!(sb.f_bsize, NFS_FABLKSIZE as u32);
        assert_eq!(sb.f_blocks, (1u64 << 30) / NFS_FABLKSIZE as u64);
        assert_eq!(sb.f_bavail, (1i64 << 28) / NFS_FABLKSIZE as i64);
        assert_eq!((sb.f_files, sb.f_ffree, sb.f_favail), (1000, 600, 600));
        assert_eq!(sb.f_iosize, NFS_RSIZE as u32);
        let nmp = vfstonfs(mp);
        assert_ne!(nmp.flag() & NFSMNT_GOTFSINFO, 0);

        assert_eq!(nfs_unmount(mp, 0, fx.user), Ok(()));
        assert!(
            mp.node_arc::<NfsMount<tn::G>>().is_none(),
            "the slot emptied"
        );
        // Ours and the timeout's (the reaper's argument).
        assert_eq!(MArc::count(&nmp), 2);
        nmp.nm_rtimeout.del();
        nfs_reaper::<tn::G>(&MArc::erase(nmp.clone()));
        assert_eq!(
            MArc::count(&nmp),
            1,
            "the reaper gave the timeout's handle back"
        );
        drop(nmp);

        peer.stop();
        drop(dummy);
        tn::pump(fx.lo);
        assert_eq!(MBUFPL.nout(), mbufs, "every mbuf back");
    }
}
/* </TESTS> */
