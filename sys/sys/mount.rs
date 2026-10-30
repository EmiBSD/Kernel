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
 * Copyright (c) 1989, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)mount.h	8.15 (Berkeley) 7/14/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/mount.h>`: the mounted file system (`struct mount`), the operations a file system
//! type provides (`struct vfsops`, called through the `VFS_*` macros), its configuration
//! entry (`struct vfsconf`), `struct statfs` and the `MNT_*` flags, file handles, the
//! `CTL_VFS` names and the buffer cache statistics.
//!
//! Upstream: sys/sys/mount.h @ 3ce1f3f79392
//!
//! Since N5.2 `struct mount` and `struct vfsconf` are the kernel crate's (`kernel::fs::mount`,
//! never freed: a mount whose last reference goes is recycled, D-N5.2-2) and `struct vfsops`
//! is the kernel's trait `Vfs`; this module is their face for the `bsd` file systems: the
//! aliases [`Mount`] and [`Vfsconf`], `struct vfsops` of `fn` members (which
//! `kern/vfs_vops.rs` implements as the kernel's `Vfs`), the `VFS_*` names over the kernel's
//! and the per-file system `*_args`.
//!
//! ## How a file system plugs in
//! - It fills a `static` [`Vfsops`] and an entry of `vfsconflist[]` (`kern/vfs_init.rs`).
//!   `vfs_root`, `vfs_vget` and `vfs_fhtovp` return the vnode where the C fills `*vpp`;
//!   `vfs_mount` receives the kernel copy of the user's arguments as a byte slice of
//!   `vfc_datasize` bytes, which the file system reads as its own `*_args` structure.
//! - Its per-mount data hangs from `mnt_data` (`*mut c_void`, as in C).
//!
//! ## Deviations
//! - `mnt_stat` is read with the kernel's `Mount::stat` and changed with `Mount::with_stat`.
//! - `struct statfs` names the holes the C compiler leaves (`_pad0` after `f_iosize`, `_pad1`
//!   before `mount_info`), so the structure is plain data that `copyout` may read whole;
//!   `union mount_info` is its 160 bytes, 8-aligned: the per-filesystem views came with
//!   their file systems (`ufs_args` and the `export_args` it embeds with ffs, `struct
//!   mfs_args` (`MfsArgs`) with MFS, `iso_args`, `msdosfs_args`, `udf_args` and `tmpfs_args`
//!   with M10c, `nfs_args` with NFS (M10e), `ntfs_args` with NTFS (M10d)); each
//!   `*Args::from_bytes` reads them out of the kernel copy of the mount arguments. Since N5.4
//!   they are `hal`'s (`hal::sys::mount`, D-N5.4-3), re-exported here, their holes named.
//! - `VFS_*` are functions with the macros' names (`#[allow(non_snake_case)]`).
//! - `struct netcred` and `struct netexport` are always defined, as the C header does, over
//!   the ported radix tree (`net/radix.rs`): their members are `Cell`s, the `ne_rtable_inet`
//!   pointer an `Option<&'static RadixNodeHead>`; a `Netcred` is `malloc`ed with the
//!   address and mask it is keyed by behind it, as in C (`kern/vfs_subr.rs`). `struct
//!   nfs_args` (`NfsArgs`) reads like the other `*Args`; its `addr`, `fh` and `hostname`
//!   are user addresses.
//! - The user-level prototypes (`mount`, `statfs`, ...) are not kernel material; the kernel
//!   ones are their functions in `vfs_subr.rs`, `vfs_init.rs` and `vfs_syscalls.rs`.

use core::cell::Cell;

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::net::radix::{RadixNode, RadixNodeHead};
use crate::sys::errno::Errno;
use crate::sys::mbuf::Mbuf;
use crate::sys::namei::Nameidata;
use crate::sys::proc::Proc;
use crate::sys::sysctl::Sysctlfn;
use crate::sys::types::{Ino, Uid};
use crate::sys::ucred::{Ucred, UcredCell};
use crate::sys::vnode::Vnode;
/// `struct bcachestats`: buffer cache statistics, `hal`'s (N5.1: one copy for the kernel's
/// cache and `vfs.generic.bcachestat`).
pub use hal::sys::mount::{
    Bcachestats, Fhandle, Fid, Fsid, MAXFIDSZ, MFSNAMELEN, MNAMELEN, MNT_ASYNC, MNT_BITS,
    MNT_DEFEXPORTED, MNT_DELEXPORT, MNT_DOOMED, MNT_EXPORTANON, MNT_EXPORTED, MNT_EXRDONLY,
    MNT_FORCE, MNT_LAZY, MNT_LOCAL, MNT_NOATIME, MNT_NODEV, MNT_NOEXEC, MNT_NOPERM, MNT_NOSUID,
    MNT_NOWAIT, MNT_OP_FLAGS, MNT_QUOTA, MNT_RDONLY, MNT_RELOAD, MNT_ROOTFS, MNT_SOFTDEP,
    MNT_STALLED, MNT_SWAPPABLE, MNT_SYNCHRONOUS, MNT_UNMOUNT, MNT_UPDATE, MNT_VISFLAGMASK,
    MNT_WAIT, MNT_WANTRDWR, MNT_WXALLOWED, MOUNT_AFS, MOUNT_CD9660, MOUNT_EXT2FS, MOUNT_FFS,
    MOUNT_FUSEFS, MOUNT_MFS, MOUNT_MSDOS, MOUNT_NCPFS, MOUNT_NFS, MOUNT_NTFS, MOUNT_TMPFS,
    MOUNT_UDF, MOUNT_UFS, MountInfo, Statfs, VB_DUPOK, VB_NOWAIT, VB_READ, VB_WAIT, VB_WRITE,
    VFS_BCACHESTAT, VFS_CONF, VFS_GENERIC, VFS_MAXTYPENUM, VfsconfAbi,
};
use kernel::fs::vops::Cred;

/// The file systems' mount arguments, `hal`'s since N5.4 (D-N5.4-3: one copy for the
/// kernel's `mount(2)` and the file systems that read them).
pub use hal::sys::mount::{
    ExportArgs, FusefsArgs, ISOFSMNT_EXTATT, ISOFSMNT_GENS, ISOFSMNT_NOJOLIET, ISOFSMNT_NORRIP,
    ISOFSMNT_SESS, IsoArgs, MSDOSFSMNT_LONGNAME, MSDOSFSMNT_NOWIN95, MSDOSFSMNT_SHORTNAME, MfsArgs,
    MsdosfsArgs, NFS_ARGSVERSION, NFSMNT_ACDIRMAX, NFSMNT_ACDIRMIN, NFSMNT_ACREGMAX,
    NFSMNT_ACREGMIN, NFSMNT_AUTHERR, NFSMNT_DEADTHRESH, NFSMNT_DISMINPROG, NFSMNT_DISMNT,
    NFSMNT_DUMBTIMR, NFSMNT_GOTFSINFO, NFSMNT_GOTPATHCONF, NFSMNT_HASAUTH, NFSMNT_HASWRITEVERF,
    NFSMNT_INT, NFSMNT_INTERNAL, NFSMNT_KERB, NFSMNT_LEASETERM, NFSMNT_MAXGRPS, NFSMNT_MNTD,
    NFSMNT_NFSV3, NFSMNT_NOAC, NFSMNT_NOCONN, NFSMNT_NQNFS, NFSMNT_RCVLOCK, NFSMNT_RDIRPLUS,
    NFSMNT_READAHEAD, NFSMNT_READDIRSIZE, NFSMNT_RESVPORT, NFSMNT_RETRANS, NFSMNT_RSIZE,
    NFSMNT_SNDLOCK, NFSMNT_SOFT, NFSMNT_TIMEO, NFSMNT_WAITAUTH, NFSMNT_WANTAUTH, NFSMNT_WANTRCV,
    NFSMNT_WANTSND, NFSMNT_WSIZE, NTFS_MFLAG_ALLNAMES, NTFS_MFLAG_CASEINS, NfsArgs, NtfsArgs,
    TMPFS_ARGS_VERSION, TmpfsArgs, UdfArgs, UfsArgs,
};

/// `struct mount`: the kernel's.
pub type Mount = kernel::fs::mount::Mount<BsdProcGlue>;

/// `struct vfsconf`: the kernel's (`vfsconflist[]` is `kern/vfs_init.rs`'s).
pub type Vfsconf = kernel::fs::mount::Vfsconf<BsdProcGlue>;

pub use kernel::fs::mount::{
    MountIter, MountIterSafe, mountlist, mountlist_first, mountlist_insert_head,
    mountlist_insert_tail, mountlist_is_empty, mountlist_next, mountlist_remove, mountlist_safe,
};

/// `BUFPAGES_DEFICIT`: how far the buffer cache is below its low water mark, in pages.
pub fn bufpages_deficit() -> i64 {
    use crate::sys::buf::{BCSTATS, BUFLOWPAGES};
    let d = BUFLOWPAGES.load(core::sync::atomic::Ordering::Relaxed)
        - BCSTATS
            .numbufpages
            .load(core::sync::atomic::Ordering::Relaxed);
    d.max(0)
}

/// `BUFPAGES_INACT`: the buffer cache's clean pages above its low water mark.
pub fn bufpages_inact() -> i64 {
    use crate::sys::buf::{BCSTATS, BUFLOWPAGES};
    let d = BCSTATS
        .numcleanpages
        .load(core::sync::atomic::Ordering::Relaxed)
        - BUFLOWPAGES.load(core::sync::atomic::Ordering::Relaxed);
    d.max(0)
}

/// The type of `vfs_mount(mp, path, data, ndp, p)`.
pub type VfsMountFn =
    fn(&'static Mount, &[u8], &mut [u8], &mut Nameidata<'_>, &Proc) -> Result<(), Errno>;

/// The type of `vfs_checkexp(mp, nam, extflagsp, credanonp)`.
pub type VfsCheckexpFn =
    fn(&'static Mount, &Mbuf, &mut i32, &mut *const Ucred) -> Result<(), Errno>;

/// The type of `vfs_init(vfsconf)`.
pub type VfsInitFn = fn(&'static Vfsconf) -> Result<(), Errno>;

/// `struct vfsops`: operations supported on mounted file system.
pub struct Vfsops {
    /// `vfs_mount(mp, path, data, ndp, p)`: `data` is the kernel copy of the user's
    /// arguments.
    pub vfs_mount: VfsMountFn,
    /// `vfs_start(mp, flags, p)`.
    pub vfs_start: fn(&'static Mount, i32, &Proc) -> Result<(), Errno>,
    /// `vfs_unmount(mp, mntflags, p)`.
    pub vfs_unmount: fn(&'static Mount, i32, &Proc) -> Result<(), Errno>,
    /// `vfs_root(mp, vpp)`: the root vnode, locked.
    pub vfs_root: fn(&'static Mount) -> Result<&'static Vnode, Errno>,
    /// `vfs_quotactl(mp, cmds, uid, arg, p)`: `arg` is a user address.
    pub vfs_quotactl: fn(&'static Mount, i32, Uid, usize, &Proc) -> Result<(), Errno>,
    /// `vfs_statfs(mp, sbp, p)`.
    pub vfs_statfs: fn(&'static Mount, &mut Statfs, &Proc) -> Result<(), Errno>,
    /// `vfs_sync(mp, waitfor, stall, cred, p)`.
    pub vfs_sync: fn(&'static Mount, i32, i32, *const Ucred, &Proc) -> Result<(), Errno>,
    /// `vfs_vget(mp, ino, vpp)`.
    pub vfs_vget: fn(&'static Mount, Ino) -> Result<&'static Vnode, Errno>,
    /// `vfs_fhtovp(mp, fhp, vpp)`.
    pub vfs_fhtovp: fn(&'static Mount, &Fid) -> Result<&'static Vnode, Errno>,
    /// `vfs_vptofh(vp, fhp)`.
    pub vfs_vptofh: fn(&'static Vnode, &mut Fid) -> Result<(), Errno>,
    /// `vfs_init(vfsconf)`, NULL when the type needs no initialisation.
    pub vfs_init: Option<VfsInitFn>,
    /// `vfs_sysctl(name, namelen, oldp, oldlenp, newp, newlen, p)`, NULL when the type has no
    /// `vfs.<type>` node.
    pub vfs_sysctl: Option<Sysctlfn>,
    /// `vfs_checkexp(mp, nam, extflagsp, credanonp)`.
    pub vfs_checkexp: VfsCheckexpFn,
}

/// `VFS_MOUNT(MP, PATH, DATA, NDP, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_MOUNT(
    mp: &'static Mount,
    path: &[u8],
    data: &mut [u8],
    ndp: &mut Nameidata<'_>,
    p: &Proc,
) -> Result<(), Errno> {
    kernel::fs::vops::VFS_MOUNT(mp, path, data, ndp, p)
}

/// `VFS_START(MP, FLAGS, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_START(mp: &'static Mount, flags: i32, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_START(mp, flags, p)
}

/// `VFS_UNMOUNT(MP, FORCE, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_UNMOUNT(mp: &'static Mount, force: i32, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_UNMOUNT(mp, force, p)
}

/// `VFS_ROOT(MP, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_ROOT(mp: &'static Mount) -> Result<&'static Vnode, Errno> {
    kernel::fs::vops::VFS_ROOT(mp)
}

/// `VFS_QUOTACTL(MP, C, U, A, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_QUOTACTL(mp: &'static Mount, c: i32, u: Uid, a: usize, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_QUOTACTL(mp, c, u, a, p)
}

/// `VFS_STATFS(MP, SBP, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_STATFS(mp: &'static Mount, sbp: &mut Statfs, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_STATFS(mp, sbp, p)
}

/// `VFS_SYNC(MP, W, S, C, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_SYNC(
    mp: &'static Mount,
    w: i32,
    s: i32,
    c: *const Ucred,
    p: &Proc,
) -> Result<(), Errno> {
    kernel::fs::vops::VFS_SYNC(mp, w, s, Cred::from_raw(c), p)
}

/// `VFS_VGET(MP, INO, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_VGET(mp: &'static Mount, ino: Ino) -> Result<&'static Vnode, Errno> {
    kernel::fs::vops::VFS_VGET(mp, ino)
}

/// `VFS_FHTOVP(MP, FIDP, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_FHTOVP(mp: &'static Mount, fidp: &Fid) -> Result<&'static Vnode, Errno> {
    kernel::fs::vops::VFS_FHTOVP(mp, fidp)
}

/// `VFS_VPTOFH(VP, FIDP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_VPTOFH(vp: &'static Vnode, fidp: &mut Fid) -> Result<(), Errno> {
    kernel::fs::vops::VFS_VPTOFH(vp, fidp)
}

/// `VFS_CHECKEXP(MP, NAM, EXFLG, CRED)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_CHECKEXP(
    mp: &'static Mount,
    nam: &Mbuf,
    exflg: &mut i32,
    cred: &mut *const Ucred,
) -> Result<(), Errno> {
    // `vfs_checkexp` is the NFS server's, `bsd`'s: the table as `bsd` knows it.
    match mp.op().as_any().and_then(|a| a.downcast_ref::<Vfsops>()) {
        Some(ops) => (ops.vfs_checkexp)(mp, nam, exflg, cred),
        None => panic(format_args!(
            "VFS_CHECKEXP: mount {:p} of a kernel file system",
            mp
        )),
    }
}

/// `struct netcred`: network address lookup element. The radix tree of a [`Netexport`] holds
/// these `malloc(M_NETADDR)`ed with the address and mask they are keyed by right behind
/// them (`netc_len` is the size of the whole allocation); the first member must stay
/// `netc_rnodes`, as `vfs_export_lookup` turns the leaf `rn_match` finds back into its
/// `Netcred`.
#[repr(C)]
pub struct Netcred {
    /// `netc_rnodes`: the radix tree's leaf and internal node for this entry.
    pub netc_rnodes: [RadixNode; 2],
    /// `netc_exflags`: the `export_args`'s `ex_flags` (`MNT_EXRDONLY`, ...).
    pub netc_exflags: Cell<i32>,
    /// `netc_len`: size of the allocation.
    pub netc_len: Cell<i32>,
    /// `netc_anon`: the credentials anonymous clients map to (the entry's own, filled in
    /// place by `vfs_hang_addrlist`).
    pub netc_anon: UcredCell,
}

// SAFETY: the members are changed under the kernel lock while the file system's export
// list is updated (`vfs_export`), as in C; the kernel serialises them like the rest of the
// mount's state.
unsafe impl Sync for Netcred {}

impl Netcred {
    /// A zeroed entry, as `malloc(M_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            netc_rnodes: [RadixNode::new(), RadixNode::new()],
            netc_exflags: Cell::new(0),
            netc_len: Cell::new(0),
            netc_anon: UcredCell::new(),
        }
    }
}

impl Default for Netcred {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct netexport`: network export information of one file system.
pub struct Netexport {
    /// `ne_defexported`: default export.
    pub ne_defexported: Netcred,
    /// `ne_rtable_inet`: individual exports.
    pub ne_rtable_inet: Cell<Option<&'static RadixNodeHead>>,
}

// SAFETY: as for `Netcred`.
unsafe impl Sync for Netexport {}

impl Netexport {
    /// An empty export list, as `malloc(M_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            ne_defexported: Netcred::new(),
            ne_rtable_inet: Cell::new(None),
        }
    }
}

impl Default for Netexport {
    fn default() -> Self {
        Self::new()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/mount.h");
        for (name, value) in [
            ("MNT_RDONLY", MNT_RDONLY),
            ("MNT_NOPERM", MNT_NOPERM),
            ("MNT_WXALLOWED", MNT_WXALLOWED),
            ("MNT_ROOTFS", MNT_ROOTFS),
            ("MNT_VISFLAGMASK", MNT_VISFLAGMASK),
            ("MNT_UPDATE", MNT_UPDATE),
            ("MNT_STALLED", MNT_STALLED),
            ("MNT_UNMOUNT", MNT_UNMOUNT),
            ("MNT_DOOMED", MNT_DOOMED),
            ("MNT_LAZY", MNT_LAZY),
            ("MFSNAMELEN", MFSNAMELEN as i32),
            ("MNAMELEN", MNAMELEN as i32),
            ("MAXFIDSZ", MAXFIDSZ as i32),
            ("VFS_BCACHESTAT", VFS_BCACHESTAT),
            ("VB_DUPOK", VB_DUPOK),
            ("ISOFSMNT_NORRIP", ISOFSMNT_NORRIP),
            ("ISOFSMNT_GENS", ISOFSMNT_GENS),
            ("ISOFSMNT_EXTATT", ISOFSMNT_EXTATT),
            ("ISOFSMNT_NOJOLIET", ISOFSMNT_NOJOLIET),
            ("ISOFSMNT_SESS", ISOFSMNT_SESS),
            ("MSDOSFSMNT_SHORTNAME", MSDOSFSMNT_SHORTNAME),
            ("MSDOSFSMNT_LONGNAME", MSDOSFSMNT_LONGNAME),
            ("MSDOSFSMNT_NOWIN95", MSDOSFSMNT_NOWIN95),
            ("TMPFS_ARGS_VERSION", TMPFS_ARGS_VERSION),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value)),
                "{name}"
            );
        }
        // The NFSMNT_* bits fill all 32 bits; the C's `int` constants are the unsigned values.
        for (name, value) in [
            ("NFS_ARGSVERSION", NFS_ARGSVERSION),
            ("NFSMNT_RESVPORT", NFSMNT_RESVPORT),
            ("NFSMNT_SOFT", NFSMNT_SOFT),
            ("NFSMNT_WSIZE", NFSMNT_WSIZE),
            ("NFSMNT_RSIZE", NFSMNT_RSIZE),
            ("NFSMNT_TIMEO", NFSMNT_TIMEO),
            ("NFSMNT_RETRANS", NFSMNT_RETRANS),
            ("NFSMNT_MAXGRPS", NFSMNT_MAXGRPS),
            ("NFSMNT_INT", NFSMNT_INT),
            ("NFSMNT_NOCONN", NFSMNT_NOCONN),
            ("NFSMNT_NQNFS", NFSMNT_NQNFS),
            ("NFSMNT_NFSV3", NFSMNT_NFSV3),
            ("NFSMNT_KERB", NFSMNT_KERB),
            ("NFSMNT_DUMBTIMR", NFSMNT_DUMBTIMR),
            ("NFSMNT_LEASETERM", NFSMNT_LEASETERM),
            ("NFSMNT_READAHEAD", NFSMNT_READAHEAD),
            ("NFSMNT_DEADTHRESH", NFSMNT_DEADTHRESH),
            ("NFSMNT_NOAC", NFSMNT_NOAC),
            ("NFSMNT_RDIRPLUS", NFSMNT_RDIRPLUS),
            ("NFSMNT_READDIRSIZE", NFSMNT_READDIRSIZE),
            ("NFSMNT_ACREGMIN", NFSMNT_ACREGMIN),
            ("NFSMNT_ACREGMAX", NFSMNT_ACREGMAX),
            ("NFSMNT_ACDIRMIN", NFSMNT_ACDIRMIN),
            ("NFSMNT_ACDIRMAX", NFSMNT_ACDIRMAX),
            ("NFSMNT_INTERNAL", NFSMNT_INTERNAL),
            ("NFSMNT_HASWRITEVERF", NFSMNT_HASWRITEVERF),
            ("NFSMNT_GOTPATHCONF", NFSMNT_GOTPATHCONF),
            ("NFSMNT_GOTFSINFO", NFSMNT_GOTFSINFO),
            ("NFSMNT_MNTD", NFSMNT_MNTD),
            ("NFSMNT_DISMINPROG", NFSMNT_DISMINPROG),
            ("NFSMNT_DISMNT", NFSMNT_DISMNT),
            ("NFSMNT_SNDLOCK", NFSMNT_SNDLOCK),
            ("NFSMNT_WANTSND", NFSMNT_WANTSND),
            ("NFSMNT_RCVLOCK", NFSMNT_RCVLOCK),
            ("NFSMNT_WANTRCV", NFSMNT_WANTRCV),
            ("NFSMNT_WAITAUTH", NFSMNT_WAITAUTH),
            ("NFSMNT_HASAUTH", NFSMNT_HASAUTH),
            ("NFSMNT_WANTAUTH", NFSMNT_WANTAUTH),
            ("NFSMNT_AUTHERR", NFSMNT_AUTHERR),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value as u32)),
                "{name}"
            );
        }
    }

    #[test]
    fn nfs_args_read_out_of_the_kernel_copy() {
        let mut bytes = [0u8; NfsArgs::SIZE];
        bytes[..4].copy_from_slice(&NFS_ARGSVERSION.to_ne_bytes());
        bytes[44..48].copy_from_slice(&(NFSMNT_NFSV3 | NFSMNT_RSIZE).to_ne_bytes());
        bytes[96..100].copy_from_slice(&7i32.to_ne_bytes());
        let args = NfsArgs::from_bytes(&bytes).expect("arguments");
        assert_eq!(args.version, NFS_ARGSVERSION);
        assert_eq!(args.flags, NFSMNT_NFSV3 | NFSMNT_RSIZE);
        assert_eq!(args.acregmin, 7);
        assert!(NfsArgs::from_bytes(&bytes[..NfsArgs::SIZE - 1]).is_none());
    }
}
/* </TESTS> */
