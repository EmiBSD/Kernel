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
 *	@(#)vnode.h	8.11 (Berkeley) 11/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/vnode.h>` as `bsd` names it. Since N5.2 the vnode, its types, tags and flags,
//! `struct vattr`, the operations and the vnode table are the kernel crate's
//! (`kernel::fs::vnode`, `kernel::fs::vops`, `kernel::fs::subr`); this module is their face for
//! the `bsd` file systems until each moves (N5.5 to N5.9, nfs N6, ntfs N10): the alias
//! [`Vnode`], the vnode's `bsd` members ([`VnodeG`]), the `struct vop_*_args` the file
//! systems' operations receive and `struct vops` of `Option<fn>` slots (which
//! `kern/vfs_vops.rs` implements as the kernel's `Vnops`), and `vfs_subr.c`'s names over the
//! kernel's functions.
//!
//! Upstream: sys/sys/vnode.h @ 3ce1f3f79392
//!
//! ## How a file system plugs in
//! - It fills a `static` [`Vops`] whose slots are `Option<fn(&mut VopXxxArgs) -> Result<(),
//!   Errno>>` (`vop_islocked` returns the lock status as an `i32`); `None` is the C's NULL,
//!   which the `VOP_*` wrappers answer with `EOPNOTSUPP`. Shared helpers that the C installs in
//!   many slots (`nullop`, `vop_generic_badop`, `eopnotsupp`) are written as closures,
//!   `Some(|_| nullop())`, so one helper serves every argument type.
//! - `getnewvnode(tag, mp, &FOO_VOPS)` returns the vnode (the kernel registers the table by
//!   its address on its first use); `set_vops(vp, &FOO_SPECVOPS)` is the C's `vp->v_op =
//!   &foo_specvops`, and `vp.set_ops_id(VopsId::SPEC)` its `vp->v_op = &spec_vops`. The file
//!   system hangs its own node from `v_data` (a `*mut c_void`, as in C) and reads it back with
//!   a cast at the one place it knows the type. The operations receive `&'static Vnode`s.
//! - The out-parameters of the C argument structures stay out-parameters: `a_vpp` is an
//!   `&mut Option<&'static Vnode>` the operation fills in, so a file system's body ports
//!   unchanged.
//!
//! ## Deviations
//! - `a_cred` is the C's `struct ucred *`, a raw pointer, since `NOCRED` and `FSCRED` are
//!   sentinel values; [`cred_ref`] turns a real one into a reference (the kernel passes it on
//!   as an opaque `kernel::fs::vops::Cred`).
//! - `struct buf *` (`a_bp`) is a `&'static Buf` (`sys/buf.rs`): a busy buffer lent with its
//!   token parked in it.
//! - `v_socket` is the member [`VnodeG::v_socket`] (the socket is `bsd`'s), not a variant of
//!   the kernel's `v_un`; `kinfo_file`'s `v_un` reads it first.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64};
use kernel::file::LockOwner;

use crate::kern::kern_descrip::BsdFileGlue;
use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::buf::Buf;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::fcntl::Flock;
use crate::sys::file::Fileops;
use crate::sys::mount::Mount;
use crate::sys::namei::Componentname;
use crate::sys::proc::{Proc, ProcExt};
use crate::sys::socketvar::Socket;
use crate::sys::types::{Daddr, Dev, Register};
use crate::sys::ucred::{FSCRED, NOCRED, Ucred};
use crate::sys::uio::Uio;
use hal::link::Link;
use kernel::fs::subr;
use kernel::fs::vops::Cred;

pub use kernel::fs::vnode::{
    DOCLOSE, FORCECLOSE, GETCWD_CHECK_ACCESS, IFTOVT_TAB, IGNORECLEAN, IO_APPEND, IO_NDELAY,
    IO_NOCACHE, IO_NODELOCKED, IO_NOLIMIT, IO_SYNC, IO_UNIT, Locked, REVOKEALL, SKIPSYSTEM, V_SAVE,
    V_SAVEMETA, VA_EXCLUSIVE, VA_UTIMES_CHANGE, VA_UTIMES_NULL, VALIASED, VBAD, VBIOERROR,
    VBIOONFREELIST, VBIOONSYNCLIST, VBIOWAIT, VBLK, VCHR, VCLONE, VCLONED, VDIR, VEXEC, VFIFO,
    VISTTY, VLARVAL, VLNK, VLOCKSWORK, VN_KNOTE, VNODE_MTX, VNON, VNOVAL, VREAD, VREG, VROOT,
    VSGID, VSOCK, VSUID, VSVTX, VSYSTEM, VT_ADOSFS, VT_AFS, VT_EXT2FS, VT_FUSEFS, VT_ISOFS, VT_MFS,
    VT_MSDOSFS, VT_NFS, VT_NON, VT_NTFS, VT_PORTAL, VT_PROCFS, VT_TMPFS, VT_UDF, VT_UFS, VT_VFS,
    VTAG_NAMES, VTEXT, VTTOIF_TAB, VTYPE_NAMES, VWRITE, VXLOCK, VXWANT, Vattr, VnodeKind, VopsId,
    Vtagtype, Vtype, WRITECLOSE, iftovt, makeimode, vttoif,
};

/// `struct vnode`: the kernel's, with `bsd`'s members ([`VnodeG`]).
pub type Vnode = kernel::fs::vnode::Vnode<BsdProcGlue>;

/// The members of `struct vnode` of `bsd`'s types (the kernel's `FileGlue::VnodeG`): the socket
/// bound to a `VSOCK` vnode's name (`uipc_usrreq.rs`, N6). The name cache's heads and a
/// device's record locks are the kernel vnode's own since N5.3 (`Vnode::nc`,
/// `Vnode::speclockf`).
#[derive(Default)]
pub struct VnodeG {
    /// `v_socket` (`v_un.vu_socket`): unix ipc, the socket bound to the name. An atomic
    /// word: `unp_detach` clears it under the vnode's lock alone and `sysctl_file` reads it
    /// with no lock, as the C does (`uipc_usrreq.c:787`).
    pub v_socket: Link<Socket>,
}

// SAFETY: LZ's argument for `struct vnode` (sys/sys/vnode.rs:280 at the pin), moved with the
// members it covered (D-N5.2-1, decision 2: the same site, not a new one), narrowed at N5.3 to
// the one member left: `v_socket` is an atomic word (`Link`), whose loads and stores never
// race; it is not `Sync` by itself only because `Socket` is not (its fields are `Cell`s under
// the socket's lock), and the reference it hands another CPU is opened as every socket
// reference of `bsd` is: `unp_connect` reads `so_type` (`[I]`, set once by `socreate`) and
// takes `solock` before the rest, `sysctl_file` takes only the address. Paid at N6 (the
// sockets).
unsafe impl Sync for VnodeG {}

/// `prtactive`: 1 => print out reclaim of active vnodes (the kernel's word).
pub static PRTACTIVE: &AtomicI32 = &crate::kern::kern_descrip::VFS_TABLES.prtactive;
/// `numvnodes`: the vnodes allocated so far (the kernel's word, `kern.numvnodes`).
pub static NUMVNODES: &AtomicI64 = &crate::kern::kern_descrip::VFS_TABLES.numvnodes;
/// `maxvnodes` (the kernel's word, `kern.maxvnodes`).
pub static MAXVNODES: &AtomicI32 = &crate::kern::kern_descrip::VFS_TABLES.maxvnodes;

/// What `kern/vfs_subr.rs` keeps (the NFS export list, the shutdown trio), named here as LZ's
/// whole module was.
pub use crate::kern::vfs_subr::*;
/// `vnode.h`'s `vn_*` functions (`vfs_vnops.c`) and `vfs_getcwd_common` (`vfs_getcwd.c`),
/// where the C declares them: the kernel's (N5.3).
pub use kernel::fs::getcwd::{vfs_getcwd_common, vfs_getcwd_getcache, vfs_getcwd_scandir};
pub use kernel::fs::mount::{
    vfs_busy, vfs_isbusy, vfs_mount_alloc, vfs_mount_free, vfs_mount_take, vfs_unbusy,
};
#[cfg(any(feature = "debug", feature = "diagnostic"))]
pub use kernel::fs::subr::vprint;
pub use kernel::fs::subr::{
    checkalias, copy_statfs_info, insmntque, vaccess, vattr_null, vcount, vdrop, vflush, vflushbuf,
    vfs_getnewfsid, vfs_mount_foreach_vnode, vfs_mountedon, vget, vgone, vgonel, vhold, vnoperm,
    vput, vputonfreelist, vrecycle, vref, vrele, vwaitforio,
};
pub use kernel::fs::vnode::vn_lock;
pub use kernel::fs::vnops::{
    RdwrBuf, vn_close, vn_isunder, vn_marktext, vn_open, vn_rdwr, vn_stat, vn_writechk,
};

/// `vn_fsizechk(vp, uio, ioflag, &overrun)`: the kernel's, its thread the uio's
/// (`uio_procp`), pinned for `SIGXFSZ` as `bsd` pins its threads (`ProcExt::as_static`).
pub fn vn_fsizechk(vp: &'static Vnode, uio: &mut Uio<'_>, ioflag: i32) -> Result<usize, Errno> {
    let p = uio.uio_procp.map(|p| hal::pin::pin_static(p.as_static()));
    kernel::fs::vnops::vn_fsizechk(vp, uio, ioflag, p)
}

/// `vnops`: the vnode files' operations, the kernel's (`kernel::fs::vnops::VnodeFileOps`,
/// N5.3) as `bsd`'s table holds them.
pub static VNOPS: Fileops = Fileops {
    fo_read: kernel::fs::vnops::vn_read::<BsdFileGlue>,
    fo_write: kernel::fs::vnops::vn_write::<BsdFileGlue>,
    fo_ioctl: kernel::fs::vnops::vn_ioctl::<BsdFileGlue>,
    fo_kqfilter: kernel::fs::vnops::vn_kqfilter::<BsdFileGlue>,
    fo_stat: kernel::fs::vnops::vn_statfile::<BsdFileGlue>,
    fo_close: kernel::fs::vnops::vn_closefile::<BsdFileGlue>,
    fo_seek: Some(kernel::fs::vnops::vn_seek::<BsdFileGlue>),
};

/// `vp->v_op = ops`: a file system's table (registered with the kernel on its first use).
pub fn set_vops(vp: &Vnode, ops: &'static Vops) {
    vp.set_ops_id(subr::vops_id::<BsdProcGlue>(ops));
}

/// Return the next vnode from the free list (`kernel::fs::subr::getnewvnode`).
pub fn getnewvnode(
    tag: Vtagtype,
    mp: Option<&'static Mount>,
    vops: &'static Vops,
) -> Result<&'static Vnode, Errno> {
    subr::getnewvnode::<BsdProcGlue>(tag, mp, vops)
}

/// Create a vnode for a block device (`kernel::fs::subr::bdevvp`).
pub fn bdevvp(dev: Dev) -> Result<Option<&'static Vnode>, Errno> {
    subr::bdevvp::<BsdProcGlue>(dev)
}

/// Create a vnode for a character device (`kernel::fs::subr::cdevvp`).
pub fn cdevvp(dev: Dev) -> Result<Option<&'static Vnode>, Errno> {
    subr::cdevvp::<BsdProcGlue>(dev)
}

/// The device vnode of `dev` and `type_` (`kernel::fs::subr::vfinddev`).
pub fn vfinddev(dev: Dev, type_: Vtype) -> Option<&'static Vnode> {
    subr::vfinddev::<BsdProcGlue>(dev, type_)
}

/// Revoke the vnodes of a minor range of a major (`kernel::fs::subr::vdevgone`).
pub fn vdevgone(maj: u32, minl: u32, minh: u32, type_: Vtype) {
    subr::vdevgone::<BsdProcGlue>(maj, minl, minh, type_);
}

/// A mount for the root file system of type `fstypename` (`kernel::fs::subr`).
pub fn vfs_rootmountalloc(fstypename: &[u8], devname: &[u8]) -> Result<&'static Mount, Errno> {
    subr::vfs_rootmountalloc::<BsdProcGlue>(fstypename, devname)
}

/// The mount of the file system `fsid` (`kernel::fs::subr::vfs_getvfs`).
pub fn vfs_getvfs(fsid: &crate::sys::mount::Fsid) -> Option<&'static Mount> {
    subr::vfs_getvfs::<BsdProcGlue>(fsid)
}

/// Flush out and invalidate all buffers associated with a vnode (`kernel::fs::subr`), the
/// credential the C's pointer.
pub fn vinvalbuf(
    vp: &'static Vnode,
    flags: i32,
    cred: *const Ucred,
    p: Option<&Proc>,
    slpflag: i32,
    slptimeo: u64,
) -> Result<(), Errno> {
    subr::vinvalbuf(vp, flags, Cred::from_raw(cred), p, slpflag, slptimeo)
}

/// The credentials of a `VOP_*` argument (`a_cred`) when they are real ones: `None` for NULL,
/// `NOCRED` and `FSCRED`.
///
/// # Safety
///
/// `cred` is NULL, `NOCRED`, `FSCRED` or a credential its holder keeps alive for `'a`: the
/// `a_cred` a vnode operation receives qualifies for the operation's duration (the caller
/// holds the thread's `p_ucred` or the file's `f_cred`).
pub unsafe fn cred_ref<'a>(cred: *const Ucred) -> Option<&'a Ucred> {
    if cred.is_null() || ptr::eq(cred, NOCRED) || ptr::eq(cred, FSCRED) {
        None
    } else {
        // SAFETY: the caller's contract.
        Some(unsafe { &*cred })
    }
}

/// `struct vop_generic_args`.
pub struct VopGenericArgs {
    /// `a_garbage`: other data probably follows.
    pub a_garbage: *mut c_void,
}

/// `struct vop_islocked_args`.
pub struct VopIslockedArgs {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
}

/// `struct vop_lookup_args`.
pub struct VopLookupArgs<'a, 'b> {
    /// `a_dvp`: the directory searched.
    pub a_dvp: &'static Vnode,
    /// `a_vpp`: the vnode found, NULL when there is none.
    pub a_vpp: &'a mut Option<&'static Vnode>,
    /// `a_cnp`: the component looked up.
    pub a_cnp: &'a mut Componentname<'b>,
}

/// `struct vop_create_args`.
pub struct VopCreateArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_vpp`.
    pub a_vpp: &'a mut Option<&'static Vnode>,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
    /// `a_vap`.
    pub a_vap: &'a mut Vattr,
}

/// `struct vop_mknod_args`.
pub struct VopMknodArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_vpp`.
    pub a_vpp: &'a mut Option<&'static Vnode>,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
    /// `a_vap`.
    pub a_vap: &'a mut Vattr,
}

/// `struct vop_open_args`.
pub struct VopOpenArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_mode`.
    pub a_mode: i32,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_p`.
    pub a_p: &'a Proc,
}

/// `struct vop_close_args`.
pub struct VopCloseArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_fflag`.
    pub a_fflag: i32,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_p`: NULL when no thread closes (a file in a message).
    pub a_p: Option<&'a Proc>,
}

/// `struct vop_access_args`.
pub struct VopAccessArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_mode`.
    pub a_mode: i32,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_p`.
    pub a_p: &'a Proc,
}

/// `struct vop_getattr_args`.
pub struct VopGetattrArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_vap`.
    pub a_vap: &'a mut Vattr,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_p`.
    pub a_p: &'a Proc,
}

/// `struct vop_setattr_args`.
pub struct VopSetattrArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_vap`.
    pub a_vap: &'a mut Vattr,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_p`.
    pub a_p: &'a Proc,
}

/// `struct vop_read_args`.
pub struct VopReadArgs<'a, 'b> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_uio`.
    pub a_uio: &'a mut Uio<'b>,
    /// `a_ioflag`.
    pub a_ioflag: i32,
    /// `a_cred`.
    pub a_cred: *const Ucred,
}

/// `struct vop_write_args`.
pub struct VopWriteArgs<'a, 'b> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_uio`.
    pub a_uio: &'a mut Uio<'b>,
    /// `a_ioflag`.
    pub a_ioflag: i32,
    /// `a_cred`.
    pub a_cred: *const Ucred,
}

/// `struct vop_ioctl_args`.
pub struct VopIoctlArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_command`.
    pub a_command: u64,
    /// `a_data`: the kernel copy of the argument.
    pub a_data: &'a mut [u8],
    /// `a_fflag`.
    pub a_fflag: i32,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_p`.
    pub a_p: &'a Proc,
}

/// `struct vop_kqfilter_args`.
pub struct VopKqfilterArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_fflag`.
    pub a_fflag: i32,
    /// `a_kn`.
    pub a_kn: &'a Knote,
}

/// `struct vop_revoke_args`.
pub struct VopRevokeArgs {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_flags`.
    pub a_flags: i32,
}

/// `struct vop_fsync_args`.
pub struct VopFsyncArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_waitfor`.
    pub a_waitfor: i32,
    /// `a_p`.
    pub a_p: &'a Proc,
}

/// `struct vop_remove_args`.
pub struct VopRemoveArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
}

/// `struct vop_link_args`.
pub struct VopLinkArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
}

/// `struct vop_rename_args`.
pub struct VopRenameArgs<'a, 'b, 'c> {
    /// `a_fdvp`.
    pub a_fdvp: &'static Vnode,
    /// `a_fvp`.
    pub a_fvp: &'static Vnode,
    /// `a_fcnp`.
    pub a_fcnp: &'a mut Componentname<'b>,
    /// `a_tdvp`.
    pub a_tdvp: &'static Vnode,
    /// `a_tvp`: NULL when the target does not exist.
    pub a_tvp: Option<&'static Vnode>,
    /// `a_tcnp`.
    pub a_tcnp: &'a mut Componentname<'c>,
}

/// `struct vop_mkdir_args`.
pub struct VopMkdirArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_vpp`.
    pub a_vpp: &'a mut Option<&'static Vnode>,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
    /// `a_vap`.
    pub a_vap: &'a mut Vattr,
}

/// `struct vop_rmdir_args`.
pub struct VopRmdirArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
}

/// `struct vop_symlink_args`.
pub struct VopSymlinkArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_vpp`.
    pub a_vpp: &'a mut Option<&'static Vnode>,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
    /// `a_vap`.
    pub a_vap: &'a mut Vattr,
    /// `a_target`: the link's contents, without the NUL.
    pub a_target: &'a [u8],
}

/// `struct vop_readdir_args`.
pub struct VopReaddirArgs<'a, 'b> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_uio`.
    pub a_uio: &'a mut Uio<'b>,
    /// `a_cred`.
    pub a_cred: *const Ucred,
    /// `a_eofflag`.
    pub a_eofflag: &'a mut i32,
}

/// `struct vop_readlink_args`.
pub struct VopReadlinkArgs<'a, 'b> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_uio`.
    pub a_uio: &'a mut Uio<'b>,
    /// `a_cred`.
    pub a_cred: *const Ucred,
}

/// `struct vop_abortop_args`.
pub struct VopAbortopArgs<'a, 'b> {
    /// `a_dvp`.
    pub a_dvp: &'static Vnode,
    /// `a_cnp`.
    pub a_cnp: &'a mut Componentname<'b>,
}

/// `struct vop_inactive_args`.
pub struct VopInactiveArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_p`: `curproc`, NULL before process 0 exists.
    pub a_p: Option<&'a Proc>,
}

/// `struct vop_reclaim_args`.
pub struct VopReclaimArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_p`: `curproc`, NULL before process 0 exists.
    pub a_p: Option<&'a Proc>,
}

/// `struct vop_lock_args`.
pub struct VopLockArgs {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_flags`: `LK_*`.
    pub a_flags: i32,
}

/// `struct vop_unlock_args`.
pub struct VopUnlockArgs {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
}

/// `struct vop_bmap_args`.
pub struct VopBmapArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_bn`.
    pub a_bn: Daddr,
    /// `a_vpp`: NULL when the caller does not want it.
    pub a_vpp: Option<&'a mut Option<&'static Vnode>>,
    /// `a_bnp`: NULL when the caller does not want it.
    pub a_bnp: Option<&'a mut Daddr>,
    /// `a_runp`: NULL when the caller does not want it.
    pub a_runp: Option<&'a mut i32>,
}

/// `struct vop_print_args`.
pub struct VopPrintArgs {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
}

/// `struct vop_pathconf_args`.
pub struct VopPathconfArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_name`.
    pub a_name: i32,
    /// `a_retval`.
    pub a_retval: &'a mut Register,
}

/// `struct vop_advlock_args`.
pub struct VopAdvlockArgs<'a> {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_id`: the lock's owner (a descriptor table or a file, D-N5.3-11).
    pub a_id: LockOwner,
    /// `a_op`.
    pub a_op: i32,
    /// `a_fl`.
    pub a_fl: &'a mut Flock,
    /// `a_flags`.
    pub a_flags: i32,
}

/// `struct vop_strategy_args`.
pub struct VopStrategyArgs {
    /// `a_vp`.
    pub a_vp: &'static Vnode,
    /// `a_bp`.
    pub a_bp: &'static Buf,
}

/// `struct vop_bwrite_args`: a special case.
pub struct VopBwriteArgs {
    /// `a_bp`.
    pub a_bp: &'static Buf,
}

/// The type of `vop_lock`.
pub type VopLockFn = fn(&mut VopLockArgs) -> Result<(), Errno>;
/// The type of `vop_unlock`.
pub type VopUnlockFn = fn(&mut VopUnlockArgs) -> Result<(), Errno>;
/// The type of `vop_islocked`: the lock status, as the C returns it.
pub type VopIslockedFn = fn(&mut VopIslockedArgs) -> i32;
/// The type of `vop_abortop`.
pub type VopAbortopFn = fn(&mut VopAbortopArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_access`.
pub type VopAccessFn = fn(&mut VopAccessArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_advlock`.
pub type VopAdvlockFn = fn(&mut VopAdvlockArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_bmap`.
pub type VopBmapFn = fn(&mut VopBmapArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_bwrite`.
pub type VopBwriteFn = fn(&mut VopBwriteArgs) -> Result<(), Errno>;
/// The type of `vop_close`.
pub type VopCloseFn = fn(&mut VopCloseArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_create`.
pub type VopCreateFn = fn(&mut VopCreateArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_fsync`.
pub type VopFsyncFn = fn(&mut VopFsyncArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_getattr`.
pub type VopGetattrFn = fn(&mut VopGetattrArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_inactive`.
pub type VopInactiveFn = fn(&mut VopInactiveArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_ioctl`.
pub type VopIoctlFn = fn(&mut VopIoctlArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_link`.
pub type VopLinkFn = fn(&mut VopLinkArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_lookup`.
pub type VopLookupFn = fn(&mut VopLookupArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_mknod`.
pub type VopMknodFn = fn(&mut VopMknodArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_open`.
pub type VopOpenFn = fn(&mut VopOpenArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_pathconf`.
pub type VopPathconfFn = fn(&mut VopPathconfArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_print`.
pub type VopPrintFn = fn(&mut VopPrintArgs) -> Result<(), Errno>;
/// The type of `vop_read`.
pub type VopReadFn = fn(&mut VopReadArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_readdir`.
pub type VopReaddirFn = fn(&mut VopReaddirArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_readlink`.
pub type VopReadlinkFn = fn(&mut VopReadlinkArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_reclaim`.
pub type VopReclaimFn = fn(&mut VopReclaimArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_remove`.
pub type VopRemoveFn = fn(&mut VopRemoveArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_rename`.
pub type VopRenameFn = fn(&mut VopRenameArgs<'_, '_, '_>) -> Result<(), Errno>;
/// The type of `vop_revoke`.
pub type VopRevokeFn = fn(&mut VopRevokeArgs) -> Result<(), Errno>;
/// The type of `vop_mkdir`.
pub type VopMkdirFn = fn(&mut VopMkdirArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_rmdir`.
pub type VopRmdirFn = fn(&mut VopRmdirArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_setattr`.
pub type VopSetattrFn = fn(&mut VopSetattrArgs<'_>) -> Result<(), Errno>;
/// The type of `vop_strategy`.
pub type VopStrategyFn = fn(&mut VopStrategyArgs) -> Result<(), Errno>;
/// The type of `vop_symlink`.
pub type VopSymlinkFn = fn(&mut VopSymlinkArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_write`.
pub type VopWriteFn = fn(&mut VopWriteArgs<'_, '_>) -> Result<(), Errno>;
/// The type of `vop_kqfilter`.
pub type VopKqfilterFn = fn(&mut VopKqfilterArgs<'_>) -> Result<(), Errno>;

/// `struct vops`: vnode operations. `None` is the C's NULL: the `VOP_*` wrapper answers
/// `EOPNOTSUPP` without calling anything.
pub struct Vops {
    /// `vop_lock`.
    pub vop_lock: Option<VopLockFn>,
    /// `vop_unlock`.
    pub vop_unlock: Option<VopUnlockFn>,
    /// `vop_islocked`.
    pub vop_islocked: Option<VopIslockedFn>,
    /// `vop_abortop`.
    pub vop_abortop: Option<VopAbortopFn>,
    /// `vop_access`.
    pub vop_access: Option<VopAccessFn>,
    /// `vop_advlock`.
    pub vop_advlock: Option<VopAdvlockFn>,
    /// `vop_bmap`.
    pub vop_bmap: Option<VopBmapFn>,
    /// `vop_bwrite`.
    pub vop_bwrite: Option<VopBwriteFn>,
    /// `vop_close`.
    pub vop_close: Option<VopCloseFn>,
    /// `vop_create`.
    pub vop_create: Option<VopCreateFn>,
    /// `vop_fsync`.
    pub vop_fsync: Option<VopFsyncFn>,
    /// `vop_getattr`.
    pub vop_getattr: Option<VopGetattrFn>,
    /// `vop_inactive`.
    pub vop_inactive: Option<VopInactiveFn>,
    /// `vop_ioctl`.
    pub vop_ioctl: Option<VopIoctlFn>,
    /// `vop_link`.
    pub vop_link: Option<VopLinkFn>,
    /// `vop_lookup`.
    pub vop_lookup: Option<VopLookupFn>,
    /// `vop_mknod`.
    pub vop_mknod: Option<VopMknodFn>,
    /// `vop_open`.
    pub vop_open: Option<VopOpenFn>,
    /// `vop_pathconf`.
    pub vop_pathconf: Option<VopPathconfFn>,
    /// `vop_print`.
    pub vop_print: Option<VopPrintFn>,
    /// `vop_read`.
    pub vop_read: Option<VopReadFn>,
    /// `vop_readdir`.
    pub vop_readdir: Option<VopReaddirFn>,
    /// `vop_readlink`.
    pub vop_readlink: Option<VopReadlinkFn>,
    /// `vop_reclaim`.
    pub vop_reclaim: Option<VopReclaimFn>,
    /// `vop_remove`.
    pub vop_remove: Option<VopRemoveFn>,
    /// `vop_rename`.
    pub vop_rename: Option<VopRenameFn>,
    /// `vop_revoke`.
    pub vop_revoke: Option<VopRevokeFn>,
    /// `vop_mkdir`.
    pub vop_mkdir: Option<VopMkdirFn>,
    /// `vop_rmdir`.
    pub vop_rmdir: Option<VopRmdirFn>,
    /// `vop_setattr`.
    pub vop_setattr: Option<VopSetattrFn>,
    /// `vop_strategy`.
    pub vop_strategy: Option<VopStrategyFn>,
    /// `vop_symlink`.
    pub vop_symlink: Option<VopSymlinkFn>,
    /// `vop_write`.
    pub vop_write: Option<VopWriteFn>,
    /// `vop_kqfilter`.
    pub vop_kqfilter: Option<VopKqfilterFn>,
}

impl Vops {
    /// A vector with every slot NULL, for `..Vops::EMPTY` in a file system's table.
    pub const EMPTY: Vops = Vops {
        vop_lock: None,
        vop_unlock: None,
        vop_islocked: None,
        vop_abortop: None,
        vop_access: None,
        vop_advlock: None,
        vop_bmap: None,
        vop_bwrite: None,
        vop_close: None,
        vop_create: None,
        vop_fsync: None,
        vop_getattr: None,
        vop_inactive: None,
        vop_ioctl: None,
        vop_link: None,
        vop_lookup: None,
        vop_mknod: None,
        vop_open: None,
        vop_pathconf: None,
        vop_print: None,
        vop_read: None,
        vop_readdir: None,
        vop_readlink: None,
        vop_reclaim: None,
        vop_remove: None,
        vop_rename: None,
        vop_revoke: None,
        vop_mkdir: None,
        vop_rmdir: None,
        vop_setattr: None,
        vop_strategy: None,
        vop_symlink: None,
        vop_write: None,
        vop_kqfilter: None,
    };
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/vnode.h");
        for (name, value) in [
            ("VROOT", i64::from(VROOT)),
            ("VTEXT", i64::from(VTEXT)),
            ("VISTTY", i64::from(VISTTY)),
            ("VXLOCK", i64::from(VXLOCK)),
            ("VXWANT", i64::from(VXWANT)),
            ("VALIASED", i64::from(VALIASED)),
            ("VCLONE", i64::from(VCLONE)),
            ("VBIOONFREELIST", i64::from(VBIOONFREELIST)),
            ("VA_UTIMES_CHANGE", i64::from(VA_UTIMES_CHANGE)),
            ("IO_NOCACHE", i64::from(IO_NOCACHE)),
            ("VEXEC", i64::from(VEXEC)),
            ("VSUID", i64::from(VSUID)),
            ("IGNORECLEAN", i64::from(IGNORECLEAN)),
            ("REVOKEALL", i64::from(REVOKEALL)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
    }

    /// Every slot of `VNOPS` (bsd's table over the kernel's vnode file operations) and the
    /// adapters of this header called once on a tmpfs file, as a file opened by `open(2)`
    /// holds it (N5.1's lesson: an adapter no host test calls is one a boot finds broken):
    /// `fo_read` and `fo_write` of an empty file, `fo_ioctl`'s `FIONREAD`, `fo_stat`,
    /// `fo_seek`, `fo_kqfilter` (an unknown filter), `vn_fsizechk`, a record lock through
    /// `VOP_ADVLOCK` (`sys::lockf::lf_advlock`), and `fo_close` from the last reference.
    #[test]
    fn every_vnops_slot_reaches_the_kernel_s_vnode_file_operations() {
        use crate::kern::kern_descrip::{BsdFileData, closef, fnew};
        use crate::kern::vfs_init::set_rootvnode;
        use crate::kern::vfs_vops::{VOP_ADVLOCK, VOP_UNLOCK};
        use crate::sys::fcntl::{F_FLOCK, F_SETLK, F_UNLCK, F_WRLCK, FREAD, FWRITE, O_CREAT};
        use crate::sys::file::DTYPE_VNODE;
        use crate::sys::filio::FIONREAD;
        use crate::sys::namei::{LOOKUP, NiDirp, ndinit};
        use crate::sys::stat::Stat;
        use crate::sys::uio::Iovec;
        use crate::tmpfs::tmpfs_vfsops::tests::{args, setup, tmpfs_conf};
        use crate::tmpfs::tmpfs_vfsops::{tmpfs_mount, tmpfs_root};

        // A tmpfs as "/", the thread's current directory (tmpfs_vnops' tests' set-up).
        let (_g, p) = setup();
        // Unlimited resources (vn_fsizechk reads RLIMIT_FSIZE).
        let mut limits = kernel::resource::Plimit::new();
        for l in &mut limits.pl_rlimit {
            l.rlim_cur = kernel::resource::RLIM_INFINITY;
            l.rlim_max = kernel::resource::RLIM_INFINITY;
        }
        drop(
            p.process()
                .set_limit(kernel::resource::PlimitRef::new(limits)),
        );
        // The record locks' pools (init_main's lf_init).
        crate::sys::lockf::lf_init();
        let mp = vfs_mount_alloc(None, tmpfs_conf());
        let mut data = args(1 << 20, 0, 0, 0o755);
        let mut nd = ndinit(LOOKUP, 0, NiDirp::Sys(b"/"), p);
        tmpfs_mount(mp, b"/", &mut data, &mut nd, p).expect("mount");
        vfs_unbusy(mp);
        let root = tmpfs_root(mp).expect("root");
        set_rootvnode(Some(root));
        p.fd().set_cdir(Some(root));
        vref(root);
        let _ = VOP_UNLOCK(root);

        // open("/f", O_RDWR | O_CREAT, 0644): the vnode on a file of VNOPS.
        let mut nd = ndinit(0, 0, NiDirp::Sys(b"/f"), p);
        vn_open(&mut nd, FREAD | FWRITE | O_CREAT, 0o644).expect("open");
        let vp = nd.ni_vp.expect("a vnode");
        let _ = VOP_UNLOCK(vp);
        let fp = fnew(p).expect("a file");
        fp.f_flag.store(
            (FREAD | FWRITE) as u32,
            core::sync::atomic::Ordering::SeqCst,
        );
        fp.install(DTYPE_VNODE, &VNOPS, BsdFileData::vnode(vp));
        assert_eq!(vp.writecount(), 1);

        let mut byte = [0u8; 1];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut byte, Some(p));
        assert_eq!((VNOPS.fo_read)(&fp, &mut uio, 0), Ok(()));
        assert_eq!(uio.uio_resid, 1, "an empty file reads nothing");
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_write(&mut iov, &byte[..0], Some(p));
        assert_eq!((VNOPS.fo_write)(&fp, &mut uio, 0), Ok(()));
        let mut n = [0xffu8; 4];
        assert_eq!((VNOPS.fo_ioctl)(&fp, FIONREAD, &mut n, p), Ok(()));
        assert_eq!(i32::from_ne_bytes(n), 0);
        let mut st = Stat::default();
        assert_eq!((VNOPS.fo_stat)(&fp, &mut st, p), Ok(()));
        assert_eq!(st.st_mode & 0o7777, 0o644);
        let mut off = 5;
        let seek = VNOPS.fo_seek.expect("a vnode seeks");
        assert_eq!(seek(&fp, &mut off, 0, p), Ok(()));
        assert_eq!(off, 5);
        let kn: &'static Knote = std::boxed::Box::leak(std::boxed::Box::new(Knote::detached()));
        kn.kn_filter().set(-100);
        assert_eq!((VNOPS.fo_kqfilter)(&fp, kn), Err(Errno::EINVAL));
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_write(&mut iov, &byte, Some(p));
        assert_eq!(vn_fsizechk(vp, &mut uio, 0), Ok(0));
        let mut fl = Flock {
            l_type: F_WRLCK,
            ..Flock::default()
        };
        let id = LockOwner::File(fp.addr());
        assert_eq!(VOP_ADVLOCK(vp, id, F_SETLK, &mut fl, F_FLOCK), Ok(()));
        fl.l_type = F_UNLCK;
        assert_eq!(
            VOP_ADVLOCK(vp, id, i32::from(F_UNLCK), &mut fl, F_FLOCK),
            Ok(())
        );
        // The last reference: fdrop's fo_close (vn_closefile, vn_close's FWRITE count).
        assert_eq!(closef(fp, p), Ok(()));
        assert_eq!(vp.writecount(), 0);
    }
}
/* </TESTS> */
