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
 * Copyright (c) 2010 Thordur I. Bjornsson <thib@openbsd.org>
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
 *
 * Copyright (c) 1992, 1993
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
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS AS IS'' AND
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
 */
/* </LICENSES> */

/* <CODE> */
//! The `VOP_*` wrappers as `bsd` names them, and `bsd`'s operations tables as the kernel's
//! traits. Since N5.2 the wrappers' checks (`vfs_vops.c`: `VBIOERROR` after an fsync, the
//! releases of `VOP_REMOVE`, `ENOTDIR` of `VOP_READDIR`, `VOP_PATHCONF`'s constant names, the
//! `curproc` asserts, `VOP_LOCK`'s `vnode_mtx` check) are the kernel's (`kernel::fs::vops`);
//! each `VOP_*` here is a thin adapter over the kernel's function (the credential the C's
//! pointer, a buffer the lent `&'static Buf` passed on as it came). `bsd`'s `struct vops` of
//! `Option<fn>` slots implements the kernel's `Vnops` (each method packs the slot's `struct
//! vop_*_args`; a NULL slot answers `EOPNOTSUPP`, the wrapper's answer in C), and its
//! `struct vfsops` the kernel's `Vfs`, so every `bsd` file system's `static` tables reach the
//! kernel unchanged. The generic operations of `vfs_default.c` the file systems' tables hold
//! (`vop_generic_*`) are adapters over `kernel::fs::default`'s.
//!
//! Upstream: sys/kern/vfs_vops.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - The wrappers keep the C's uppercase names (`#[allow(non_snake_case)]`); out-parameters
//!   are `&mut Option<&'static Vnode>` as in the argument structures.
//! - `VOP_PRINT` exists under feature `diagnostic` or `debug` (the C's `DEBUG ||
//!   DIAGNOSTIC`).
//! - `VOP_BWRITE` panics for a buffer without a vnode, where the C would dereference NULL.

use kernel::file::LockOwner;

use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::buf::{Buf, bwrite, lend};
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::fcntl::Flock;
use crate::sys::mount::{Fid, Mount, Statfs, Vfsconf, Vfsops};
use crate::sys::namei::{Componentname, Nameidata};
use crate::sys::proc::Proc;
use crate::sys::types::{Daddr, Ino, Register, Uid};
use crate::sys::ucred::Ucred;
use crate::sys::uio::Uio;
use crate::sys::vnode::{
    Vattr, Vnode, VopAbortopArgs, VopAccessArgs, VopAdvlockArgs, VopBmapArgs, VopBwriteArgs,
    VopCloseArgs, VopCreateArgs, VopFsyncArgs, VopGetattrArgs, VopInactiveArgs, VopIoctlArgs,
    VopIslockedArgs, VopKqfilterArgs, VopLinkArgs, VopLockArgs, VopLookupArgs, VopMkdirArgs,
    VopMknodArgs, VopOpenArgs, VopPathconfArgs, VopPrintArgs, VopReadArgs, VopReaddirArgs,
    VopReadlinkArgs, VopReclaimArgs, VopRemoveArgs, VopRenameArgs, VopRevokeArgs, VopRmdirArgs,
    VopSetattrArgs, VopStrategyArgs, VopSymlinkArgs, VopUnlockArgs, VopWriteArgs, Vops,
};
use kernel::fs::default;
use kernel::fs::vops::{self, Cred, IoBuf, Vfs, Vnops};

/// The buffer a `bsd` operation receives (`struct buf *`): a lent one as it came, a token lent
/// (parked in its buffer) for the operation to give back as the C's code does.
fn legacy_buf(bp: IoBuf) -> &'static Buf {
    match bp {
        IoBuf::Lent(bp) => bp,
        IoBuf::Token(t) => lend(t),
    }
}

impl Vnops<BsdProcGlue> for Vops {
    fn lock(&self, vp: &'static Vnode, flags: i32) -> Result<(), Errno> {
        match self.vop_lock {
            Some(f) => f(&mut VopLockArgs {
                a_vp: vp,
                a_flags: flags,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn unlock(&self, vp: &'static Vnode) -> Result<(), Errno> {
        match self.vop_unlock {
            Some(f) => f(&mut VopUnlockArgs { a_vp: vp }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn islocked(&self, vp: &'static Vnode) -> i32 {
        match self.vop_islocked {
            Some(f) => f(&mut VopIslockedArgs { a_vp: vp }),
            None => Errno::EOPNOTSUPP as i32,
        }
    }

    fn lookup(
        &self,
        dvp: &'static Vnode,
        vpp: &mut Option<&'static Vnode>,
        cnp: &mut Componentname,
    ) -> Result<(), Errno> {
        match self.vop_lookup {
            Some(f) => f(&mut VopLookupArgs {
                a_dvp: dvp,
                a_vpp: vpp,
                a_cnp: cnp,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn create(
        &self,
        dvp: &'static Vnode,
        vpp: &mut Option<&'static Vnode>,
        cnp: &mut Componentname,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        match self.vop_create {
            Some(f) => f(&mut VopCreateArgs {
                a_dvp: dvp,
                a_vpp: vpp,
                a_cnp: cnp,
                a_vap: vap,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn mknod(
        &self,
        dvp: &'static Vnode,
        vpp: &mut Option<&'static Vnode>,
        cnp: &mut Componentname,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        match self.vop_mknod {
            Some(f) => f(&mut VopMknodArgs {
                a_dvp: dvp,
                a_vpp: vpp,
                a_cnp: cnp,
                a_vap: vap,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn open(&self, vp: &'static Vnode, mode: i32, cred: Cred<'_>, p: &Proc) -> Result<(), Errno> {
        match self.vop_open {
            Some(f) => f(&mut VopOpenArgs {
                a_vp: vp,
                a_mode: mode,
                a_cred: cred.as_raw(),
                a_p: p,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn close(
        &self,
        vp: &'static Vnode,
        fflag: i32,
        cred: Cred<'_>,
        p: Option<&Proc>,
    ) -> Result<(), Errno> {
        match self.vop_close {
            Some(f) => f(&mut VopCloseArgs {
                a_vp: vp,
                a_fflag: fflag,
                a_cred: cred.as_raw(),
                a_p: p,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn access(&self, vp: &'static Vnode, mode: i32, cred: Cred<'_>, p: &Proc) -> Result<(), Errno> {
        match self.vop_access {
            Some(f) => f(&mut VopAccessArgs {
                a_vp: vp,
                a_mode: mode,
                a_cred: cred.as_raw(),
                a_p: p,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn getattr(
        &self,
        vp: &'static Vnode,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Proc,
    ) -> Result<(), Errno> {
        match self.vop_getattr {
            Some(f) => f(&mut VopGetattrArgs {
                a_vp: vp,
                a_vap: vap,
                a_cred: cred.as_raw(),
                a_p: p,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn setattr(
        &self,
        vp: &'static Vnode,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Proc,
    ) -> Result<(), Errno> {
        match self.vop_setattr {
            Some(f) => f(&mut VopSetattrArgs {
                a_vp: vp,
                a_vap: vap,
                a_cred: cred.as_raw(),
                a_p: p,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn read(
        &self,
        vp: &'static Vnode,
        uio: &mut Uio<'_>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        match self.vop_read {
            Some(f) => f(&mut VopReadArgs {
                a_vp: vp,
                a_uio: uio,
                a_ioflag: ioflag,
                a_cred: cred.as_raw(),
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn write(
        &self,
        vp: &'static Vnode,
        uio: &mut Uio<'_>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        match self.vop_write {
            Some(f) => f(&mut VopWriteArgs {
                a_vp: vp,
                a_uio: uio,
                a_ioflag: ioflag,
                a_cred: cred.as_raw(),
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn ioctl(
        &self,
        vp: &'static Vnode,
        command: u64,
        data: &mut [u8],
        fflag: i32,
        cred: Cred<'_>,
        p: &Proc,
    ) -> Result<(), Errno> {
        match self.vop_ioctl {
            Some(f) => f(&mut VopIoctlArgs {
                a_vp: vp,
                a_command: command,
                a_data: data,
                a_fflag: fflag,
                a_cred: cred.as_raw(),
                a_p: p,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn kqfilter(&self, vp: &'static Vnode, fflag: i32, kn: &Knote) -> Result<(), Errno> {
        match self.vop_kqfilter {
            Some(f) => f(&mut VopKqfilterArgs {
                a_vp: vp,
                a_fflag: fflag,
                a_kn: kn,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn revoke(&self, vp: &'static Vnode, flags: i32) -> Result<(), Errno> {
        match self.vop_revoke {
            Some(f) => f(&mut VopRevokeArgs {
                a_vp: vp,
                a_flags: flags,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn fsync(
        &self,
        vp: &'static Vnode,
        cred: Cred<'_>,
        waitfor: i32,
        p: &Proc,
    ) -> Result<(), Errno> {
        match self.vop_fsync {
            Some(f) => f(&mut VopFsyncArgs {
                a_vp: vp,
                a_cred: cred.as_raw(),
                a_waitfor: waitfor,
                a_p: p,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn remove(
        &self,
        dvp: &'static Vnode,
        vp: &'static Vnode,
        cnp: &mut Componentname,
    ) -> Result<(), Errno> {
        match self.vop_remove {
            Some(f) => f(&mut VopRemoveArgs {
                a_dvp: dvp,
                a_vp: vp,
                a_cnp: cnp,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn link(
        &self,
        dvp: &'static Vnode,
        vp: &'static Vnode,
        cnp: &mut Componentname,
    ) -> Result<(), Errno> {
        match self.vop_link {
            Some(f) => f(&mut VopLinkArgs {
                a_dvp: dvp,
                a_vp: vp,
                a_cnp: cnp,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn rename(
        &self,
        fdvp: &'static Vnode,
        fvp: &'static Vnode,
        fcnp: &mut Componentname,
        tdvp: &'static Vnode,
        tvp: Option<&'static Vnode>,
        tcnp: &mut Componentname,
    ) -> Result<(), Errno> {
        match self.vop_rename {
            Some(f) => f(&mut VopRenameArgs {
                a_fdvp: fdvp,
                a_fvp: fvp,
                a_fcnp: fcnp,
                a_tdvp: tdvp,
                a_tvp: tvp,
                a_tcnp: tcnp,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn mkdir(
        &self,
        dvp: &'static Vnode,
        vpp: &mut Option<&'static Vnode>,
        cnp: &mut Componentname,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        match self.vop_mkdir {
            Some(f) => f(&mut VopMkdirArgs {
                a_dvp: dvp,
                a_vpp: vpp,
                a_cnp: cnp,
                a_vap: vap,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn rmdir(
        &self,
        dvp: &'static Vnode,
        vp: &'static Vnode,
        cnp: &mut Componentname,
    ) -> Result<(), Errno> {
        match self.vop_rmdir {
            Some(f) => f(&mut VopRmdirArgs {
                a_dvp: dvp,
                a_vp: vp,
                a_cnp: cnp,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn symlink(
        &self,
        dvp: &'static Vnode,
        vpp: &mut Option<&'static Vnode>,
        cnp: &mut Componentname,
        vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        match self.vop_symlink {
            Some(f) => f(&mut VopSymlinkArgs {
                a_dvp: dvp,
                a_vpp: vpp,
                a_cnp: cnp,
                a_vap: vap,
                a_target: target,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn readdir(
        &self,
        vp: &'static Vnode,
        uio: &mut Uio<'_>,
        cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        match self.vop_readdir {
            Some(f) => f(&mut VopReaddirArgs {
                a_vp: vp,
                a_uio: uio,
                a_cred: cred.as_raw(),
                a_eofflag: eofflag,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn readlink(&self, vp: &'static Vnode, uio: &mut Uio<'_>, cred: Cred<'_>) -> Result<(), Errno> {
        match self.vop_readlink {
            Some(f) => f(&mut VopReadlinkArgs {
                a_vp: vp,
                a_uio: uio,
                a_cred: cred.as_raw(),
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn abortop(&self, dvp: &'static Vnode, cnp: &mut Componentname) -> Result<(), Errno> {
        match self.vop_abortop {
            Some(f) => f(&mut VopAbortopArgs {
                a_dvp: dvp,
                a_cnp: cnp,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn inactive(&self, vp: &'static Vnode, p: Option<&Proc>) -> Result<(), Errno> {
        match self.vop_inactive {
            Some(f) => f(&mut VopInactiveArgs { a_vp: vp, a_p: p }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn reclaim(&self, vp: &'static Vnode, p: Option<&Proc>) -> Result<(), Errno> {
        match self.vop_reclaim {
            Some(f) => f(&mut VopReclaimArgs { a_vp: vp, a_p: p }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn bmap(
        &self,
        vp: &'static Vnode,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        match self.vop_bmap {
            Some(f) => f(&mut VopBmapArgs {
                a_vp: vp,
                a_bn: bn,
                a_vpp: vpp,
                a_bnp: bnp,
                a_runp: runp,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn print(&self, vp: &'static Vnode) -> Result<(), Errno> {
        match self.vop_print {
            Some(f) => f(&mut VopPrintArgs { a_vp: vp }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn pathconf(&self, vp: &'static Vnode, name: i32, retval: &mut Register) -> Result<(), Errno> {
        match self.vop_pathconf {
            Some(f) => f(&mut VopPathconfArgs {
                a_vp: vp,
                a_name: name,
                a_retval: retval,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn advlock(
        &self,
        vp: &'static Vnode,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        match self.vop_advlock {
            Some(f) => f(&mut VopAdvlockArgs {
                a_vp: vp,
                a_id: id,
                a_op: op,
                a_fl: fl,
                a_flags: flags,
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn strategy(&self, vp: &'static Vnode, bp: IoBuf) -> Result<(), Errno> {
        match self.vop_strategy {
            Some(f) => f(&mut VopStrategyArgs {
                a_vp: vp,
                a_bp: legacy_buf(bp),
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        match self.vop_bwrite {
            Some(f) => f(&mut VopBwriteArgs {
                a_bp: legacy_buf(bp),
            }),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn as_any(&self) -> Option<&dyn core::any::Any> {
        Some(self)
    }
}

impl Vfs<BsdProcGlue> for Vfsops {
    fn mount(
        &self,
        mp: &'static Mount,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nameidata<'_>,
        p: &Proc,
    ) -> Result<(), Errno> {
        (self.vfs_mount)(mp, path, data, ndp, p)
    }

    fn start(&self, mp: &'static Mount, flags: i32, p: &Proc) -> Result<(), Errno> {
        (self.vfs_start)(mp, flags, p)
    }

    fn unmount(&self, mp: &'static Mount, mntflags: i32, p: &Proc) -> Result<(), Errno> {
        (self.vfs_unmount)(mp, mntflags, p)
    }

    fn root(&self, mp: &'static Mount) -> Result<&'static Vnode, Errno> {
        (self.vfs_root)(mp)
    }

    fn quotactl(
        &self,
        mp: &'static Mount,
        cmds: i32,
        uid: Uid,
        arg: usize,
        p: &Proc,
    ) -> Result<(), Errno> {
        (self.vfs_quotactl)(mp, cmds, uid, arg, p)
    }

    fn statfs(&self, mp: &'static Mount, sbp: &mut Statfs, p: &Proc) -> Result<(), Errno> {
        (self.vfs_statfs)(mp, sbp, p)
    }

    fn sync(
        &self,
        mp: &'static Mount,
        waitfor: i32,
        stall: i32,
        cred: Cred<'_>,
        p: &Proc,
    ) -> Result<(), Errno> {
        (self.vfs_sync)(mp, waitfor, stall, cred.as_raw(), p)
    }

    fn vget(&self, mp: &'static Mount, ino: Ino) -> Result<&'static Vnode, Errno> {
        (self.vfs_vget)(mp, ino)
    }

    fn fhtovp(&self, mp: &'static Mount, fid: &Fid) -> Result<&'static Vnode, Errno> {
        (self.vfs_fhtovp)(mp, fid)
    }

    fn vptofh(&self, vp: &'static Vnode, fid: &mut Fid) -> Result<(), Errno> {
        (self.vfs_vptofh)(vp, fid)
    }

    fn init(&self, vfc: &'static Vfsconf) -> Result<(), Errno> {
        match self.vfs_init {
            Some(f) => f(vfc),
            None => Ok(()),
        }
    }

    fn sysctl(
        &self,
        name: &[i32],
        oldp: usize,
        oldlenp: &mut usize,
        newp: usize,
        newlen: usize,
        p: &Proc,
    ) -> Result<(), Errno> {
        match self.vfs_sysctl {
            Some(f) => f(name, oldp, oldlenp, newp, newlen, p),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn as_any(&self) -> Option<&dyn core::any::Any> {
        Some(self)
    }
}

/// `VOP_ISLOCKED(vp)`: the lock status, or `EOPNOTSUPP` as an `int`, as in C.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_ISLOCKED(vp: &'static Vnode) -> i32 {
    vops::vop_islocked(vp)
}

/// `VOP_LOOKUP(dvp, vpp, cnp)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_LOOKUP(
    dvp: &'static Vnode,
    vpp: &mut Option<&'static Vnode>,
    cnp: &mut Componentname,
) -> Result<(), Errno> {
    vops::vop_lookup(dvp, vpp, cnp)
}

/// `VOP_CREATE(dvp, vpp, cnp, vap)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_CREATE(
    dvp: &'static Vnode,
    vpp: &mut Option<&'static Vnode>,
    cnp: &mut Componentname,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    vops::vop_create(dvp, vpp, cnp, vap)
}

/// `VOP_MKNOD(dvp, vpp, cnp, vap)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_MKNOD(
    dvp: &'static Vnode,
    vpp: &mut Option<&'static Vnode>,
    cnp: &mut Componentname,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    vops::vop_mknod(dvp, vpp, cnp, vap)
}

/// `VOP_OPEN(vp, mode, cred, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_OPEN(vp: &'static Vnode, mode: i32, cred: *const Ucred, p: &Proc) -> Result<(), Errno> {
    vops::vop_open(vp, mode, Cred::from_raw(cred), p)
}

/// `VOP_CLOSE(vp, fflag, cred, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_CLOSE(
    vp: &'static Vnode,
    fflag: i32,
    cred: *const Ucred,
    p: Option<&Proc>,
) -> Result<(), Errno> {
    vops::vop_close(vp, fflag, Cred::from_raw(cred), p)
}

/// `VOP_ACCESS(vp, mode, cred, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_ACCESS(
    vp: &'static Vnode,
    mode: i32,
    cred: *const Ucred,
    p: &Proc,
) -> Result<(), Errno> {
    vops::vop_access(vp, mode, Cred::from_raw(cred), p)
}

/// `VOP_GETATTR(vp, vap, cred, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_GETATTR(
    vp: &'static Vnode,
    vap: &mut Vattr,
    cred: *const Ucred,
    p: &Proc,
) -> Result<(), Errno> {
    vops::vop_getattr(vp, vap, Cred::from_raw(cred), p)
}

/// `VOP_SETATTR(vp, vap, cred, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_SETATTR(
    vp: &'static Vnode,
    vap: &mut Vattr,
    cred: *const Ucred,
    p: &Proc,
) -> Result<(), Errno> {
    vops::vop_setattr(vp, vap, Cred::from_raw(cred), p)
}

/// `VOP_READ(vp, uio, ioflag, cred)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_READ(
    vp: &'static Vnode,
    uio: &mut Uio<'_>,
    ioflag: i32,
    cred: *const Ucred,
) -> Result<(), Errno> {
    vops::vop_read(vp, uio, ioflag, Cred::from_raw(cred))
}

/// `VOP_WRITE(vp, uio, ioflag, cred)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_WRITE(
    vp: &'static Vnode,
    uio: &mut Uio<'_>,
    ioflag: i32,
    cred: *const Ucred,
) -> Result<(), Errno> {
    vops::vop_write(vp, uio, ioflag, Cred::from_raw(cred))
}

/// `VOP_IOCTL(vp, command, data, fflag, cred, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_IOCTL(
    vp: &'static Vnode,
    command: u64,
    data: &mut [u8],
    fflag: i32,
    cred: *const Ucred,
    p: &Proc,
) -> Result<(), Errno> {
    vops::vop_ioctl(vp, command, data, fflag, Cred::from_raw(cred), p)
}

/// `VOP_KQFILTER(vp, fflag, kn)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_KQFILTER(vp: &'static Vnode, fflag: i32, kn: &Knote) -> Result<(), Errno> {
    vops::vop_kqfilter(vp, fflag, kn)
}

/// `VOP_REVOKE(vp, flags)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_REVOKE(vp: &'static Vnode, flags: i32) -> Result<(), Errno> {
    vops::vop_revoke(vp, flags)
}

/// `VOP_FSYNC(vp, cred, waitfor, p)`: a write that failed since makes a successful sync
/// `EIO`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_FSYNC(
    vp: &'static Vnode,
    cred: *const Ucred,
    waitfor: i32,
    p: &Proc,
) -> Result<(), Errno> {
    vops::vop_fsync(vp, Cred::from_raw(cred), waitfor, p)
}

/// `VOP_REMOVE(dvp, vp, cnp)`: removes the entry and releases both vnodes.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_REMOVE(
    dvp: &'static Vnode,
    vp: &'static Vnode,
    cnp: &mut Componentname,
) -> Result<(), Errno> {
    vops::vop_remove(dvp, vp, cnp)
}

/// `VOP_LINK(dvp, vp, cnp)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_LINK(
    dvp: &'static Vnode,
    vp: &'static Vnode,
    cnp: &mut Componentname,
) -> Result<(), Errno> {
    vops::vop_link(dvp, vp, cnp)
}

/// `VOP_RENAME(fdvp, fvp, fcnp, tdvp, tvp, tcnp)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_RENAME(
    fdvp: &'static Vnode,
    fvp: &'static Vnode,
    fcnp: &mut Componentname,
    tdvp: &'static Vnode,
    tvp: Option<&'static Vnode>,
    tcnp: &mut Componentname,
) -> Result<(), Errno> {
    vops::vop_rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
}

/// `VOP_MKDIR(dvp, vpp, cnp, vap)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_MKDIR(
    dvp: &'static Vnode,
    vpp: &mut Option<&'static Vnode>,
    cnp: &mut Componentname,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    vops::vop_mkdir(dvp, vpp, cnp, vap)
}

/// `VOP_RMDIR(dvp, vp, cnp)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_RMDIR(
    dvp: &'static Vnode,
    vp: &'static Vnode,
    cnp: &mut Componentname,
) -> Result<(), Errno> {
    vops::vop_rmdir(dvp, vp, cnp)
}

/// `VOP_SYMLINK(dvp, vpp, cnp, vap, target)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_SYMLINK(
    dvp: &'static Vnode,
    vpp: &mut Option<&'static Vnode>,
    cnp: &mut Componentname,
    vap: &mut Vattr,
    target: &[u8],
) -> Result<(), Errno> {
    vops::vop_symlink(dvp, vpp, cnp, vap, target)
}

/// `VOP_READDIR(vp, uio, cred, eofflag)`: `ENOTDIR` but for a directory.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_READDIR(
    vp: &'static Vnode,
    uio: &mut Uio<'_>,
    cred: *const Ucred,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    vops::vop_readdir(vp, uio, Cred::from_raw(cred), eofflag)
}

/// `VOP_READLINK(vp, uio, cred)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_READLINK(
    vp: &'static Vnode,
    uio: &mut Uio<'_>,
    cred: *const Ucred,
) -> Result<(), Errno> {
    vops::vop_readlink(vp, uio, Cred::from_raw(cred))
}

/// `VOP_ABORTOP(dvp, cnp)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_ABORTOP(dvp: &'static Vnode, cnp: &mut Componentname) -> Result<(), Errno> {
    vops::vop_abortop(dvp, cnp)
}

/// `VOP_INACTIVE(vp, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_INACTIVE(vp: &'static Vnode, p: Option<&Proc>) -> Result<(), Errno> {
    vops::vop_inactive(vp, p)
}

/// `VOP_RECLAIM(vp, p)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_RECLAIM(vp: &'static Vnode, p: Option<&Proc>) -> Result<(), Errno> {
    vops::vop_reclaim(vp, p)
}

/// `VOP_LOCK(vp, flags)`: never with `vnode_mtx` held.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_LOCK(vp: &'static Vnode, flags: i32) -> Result<(), Errno> {
    vops::vop_lock(vp, flags)
}

/// `VOP_UNLOCK(vp)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_UNLOCK(vp: &'static Vnode) -> Result<(), Errno> {
    vops::vop_unlock(vp)
}

/// `VOP_BMAP(vp, bn, vpp, bnp, runp)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_BMAP(
    vp: &'static Vnode,
    bn: Daddr,
    vpp: Option<&mut Option<&'static Vnode>>,
    bnp: Option<&mut Daddr>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    vops::vop_bmap(vp, bn, vpp, bnp, runp)
}

/// `VOP_PRINT(vp)`.
#[cfg(any(feature = "debug", feature = "diagnostic"))]
#[allow(non_snake_case)] // the C name
pub fn VOP_PRINT(vp: &'static Vnode) -> Result<(), Errno> {
    vops::vop_print(vp)
}

/// `VOP_PATHCONF(vp, name, retval)`: the names constant across file systems answered here.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_PATHCONF(vp: &'static Vnode, name: i32, retval: &mut Register) -> Result<(), Errno> {
    vops::vop_pathconf(vp, name, retval)
}

/// `VOP_ADVLOCK(vp, id, op, fl, flags)`.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_ADVLOCK(
    vp: &'static Vnode,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    vops::vop_advlock(vp, id, op, fl, flags)
}

/// `VOP_STRATEGY(vp, bp)`: the lent buffer passed on as it came (no token taken or parked on
/// the way).
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_STRATEGY(vp: &'static Vnode, bp: &'static Buf) -> Result<(), Errno> {
    vops::vop_strategy(vp, IoBuf::Lent(bp))
}

/// `VOP_BWRITE(bp)`: dispatches on the buffer's vnode.
#[allow(non_snake_case)] // the C name
#[inline]
pub fn VOP_BWRITE(bp: &'static Buf) -> Result<(), Errno> {
    vops::vop_bwrite::<BsdProcGlue>(IoBuf::Lent(bp))
}

/// `vop_generic_revoke`: eliminates all activity of a vnode (and of its aliases, `REVOKEALL`).
pub fn vop_generic_revoke(ap: &mut VopRevokeArgs) -> Result<(), Errno> {
    default::vop_generic_revoke(ap.a_vp, ap.a_flags)
}

/// `vop_generic_badop`: an operation that must never be reached.
pub fn vop_generic_badop() -> ! {
    default::vop_generic_badop()
}

/// `vop_generic_bmap`: the identity block map: the vnode itself, the same block, no run.
pub fn vop_generic_bmap(ap: &mut VopBmapArgs<'_>) -> Result<(), Errno> {
    default::vop_generic_bmap(
        ap.a_vp,
        ap.a_bn,
        ap.a_vpp.as_deref_mut(),
        ap.a_bnp.as_deref_mut(),
        ap.a_runp.as_deref_mut(),
    )
}

/// `vop_generic_bwrite`: `bwrite(ap->a_bp)` (the token parked in it, or a driver's buffer's
/// own).
pub fn vop_generic_bwrite(ap: &mut VopBwriteArgs) -> Result<(), Errno> {
    bwrite(ap.a_bp)
}

/// `vop_generic_abortop`: frees the pathname buffer a lookup left behind, unless the caller
/// asked to keep it (`SAVESTART`).
pub fn vop_generic_abortop(ap: &mut VopAbortopArgs<'_, '_>) -> Result<(), Errno> {
    default::vop_generic_abortop::<BsdProcGlue>(ap.a_cnp)
}

/// Trivial lookup routine that always fails.
pub fn vop_generic_lookup(ap: &mut VopLookupArgs<'_, '_>) -> Result<(), Errno> {
    default::vop_generic_lookup(ap.a_vpp)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Every adapter of this module called once (N5.1's lesson: an untested boundary helper
    // deadlocked every softraid I/O): each `VOP_*` reaches its slot of a `bsd` table through
    // the kernel's dispatch with its arguments as they came (the credential the same
    // pointer), a NULL slot answers `EOPNOTSUPP`, and the `vfsops` table and the generic
    // operations reach theirs.

    use core::sync::atomic::{AtomicU64, Ordering};
    use std::{assert, assert_eq};

    use super::*;
    use crate::kern::kern_event::tests::{owned_knote, owned_knote_drop};
    use crate::kern::vfs_subr::tests::setup;
    use crate::sys::buf::{BufLegacy, with_private_buf};
    use crate::sys::mount::{
        MNT_LOCAL, VFS_FHTOVP, VFS_QUOTACTL, VFS_ROOT, VFS_START, VFS_STATFS, VFS_SYNC,
        VFS_UNMOUNT, VFS_VGET, VFS_VPTOFH,
    };
    use crate::sys::namei::Componentname;
    use crate::sys::ucred::NOCRED;
    use crate::sys::uio::{Iovec, UioRw, UioSeg};
    use crate::sys::unistd::{_PC_LINK_MAX, _PC_PATH_MAX};
    use crate::sys::vnode::{VDIR, VT_NON, getnewvnode, insmntque, vfs_mount_alloc, vref, vrele};
    use core::ptr;

    /// The slots each test table's operations hit, one bit each.
    static HITS: AtomicU64 = AtomicU64::new(0);

    fn hit(bit: u32) {
        HITS.fetch_or(1 << bit, Ordering::SeqCst);
    }

    /// A slot that records its bit; with a credential, it must be `NOCRED` (passed through).
    macro_rules! slot {
        ($name:ident, $args:ty, $bit:expr) => {
            fn $name(_ap: &mut $args) -> Result<(), Errno> {
                hit($bit);
                Ok(())
            }
        };
        ($name:ident, $args:ty, $bit:expr, cred) => {
            fn $name(ap: &mut $args) -> Result<(), Errno> {
                assert!(core::ptr::eq(ap.a_cred, NOCRED), "a_cred passes through");
                hit($bit);
                Ok(())
            }
        };
    }

    slot!(c_lock, VopLockArgs, 0);
    slot!(c_unlock, VopUnlockArgs, 1);
    fn c_islocked(_ap: &mut VopIslockedArgs) -> i32 {
        hit(2);
        0
    }
    slot!(c_abortop, VopAbortopArgs<'_, '_>, 3);
    slot!(c_access, VopAccessArgs<'_>, 4, cred);
    slot!(c_advlock, VopAdvlockArgs<'_>, 5);
    slot!(c_bmap, VopBmapArgs<'_>, 6);
    slot!(c_bwrite, VopBwriteArgs, 7);
    slot!(c_close, VopCloseArgs<'_>, 8, cred);
    slot!(c_create, VopCreateArgs<'_, '_>, 9);
    slot!(c_fsync, VopFsyncArgs<'_>, 10, cred);
    slot!(c_getattr, VopGetattrArgs<'_>, 11, cred);
    slot!(c_inactive, VopInactiveArgs<'_>, 12);
    slot!(c_ioctl, VopIoctlArgs<'_>, 13, cred);
    slot!(c_link, VopLinkArgs<'_, '_>, 14);
    slot!(c_lookup, VopLookupArgs<'_, '_>, 15);
    slot!(c_mknod, VopMknodArgs<'_, '_>, 16);
    slot!(c_open, VopOpenArgs<'_>, 17, cred);
    slot!(c_pathconf, VopPathconfArgs<'_>, 18);
    slot!(c_print, VopPrintArgs, 19);
    slot!(c_read, VopReadArgs<'_, '_>, 20, cred);
    slot!(c_readdir, VopReaddirArgs<'_, '_>, 21, cred);
    slot!(c_readlink, VopReadlinkArgs<'_, '_>, 22, cred);
    slot!(c_reclaim, VopReclaimArgs<'_>, 23);
    slot!(c_remove, VopRemoveArgs<'_, '_>, 24);
    slot!(c_rename, VopRenameArgs<'_, '_, '_>, 25);
    slot!(c_revoke, VopRevokeArgs, 26);
    slot!(c_mkdir, VopMkdirArgs<'_, '_>, 27);
    slot!(c_rmdir, VopRmdirArgs<'_, '_>, 28);
    slot!(c_setattr, VopSetattrArgs<'_>, 29, cred);
    slot!(c_strategy, VopStrategyArgs, 30);
    slot!(c_symlink, VopSymlinkArgs<'_, '_>, 31);
    slot!(c_write, VopWriteArgs<'_, '_>, 32, cred);
    slot!(c_kqfilter, VopKqfilterArgs<'_>, 33);

    /// Every slot filled with a recorder.
    static COUNTING: Vops = Vops {
        vop_lock: Some(c_lock),
        vop_unlock: Some(c_unlock),
        vop_islocked: Some(c_islocked),
        vop_abortop: Some(c_abortop),
        vop_access: Some(c_access),
        vop_advlock: Some(c_advlock),
        vop_bmap: Some(c_bmap),
        vop_bwrite: Some(c_bwrite),
        vop_close: Some(c_close),
        vop_create: Some(c_create),
        vop_fsync: Some(c_fsync),
        vop_getattr: Some(c_getattr),
        vop_inactive: Some(c_inactive),
        vop_ioctl: Some(c_ioctl),
        vop_link: Some(c_link),
        vop_lookup: Some(c_lookup),
        vop_mknod: Some(c_mknod),
        vop_open: Some(c_open),
        vop_pathconf: Some(c_pathconf),
        vop_print: Some(c_print),
        vop_read: Some(c_read),
        vop_readdir: Some(c_readdir),
        vop_readlink: Some(c_readlink),
        vop_reclaim: Some(c_reclaim),
        vop_remove: Some(c_remove),
        vop_rename: Some(c_rename),
        vop_revoke: Some(c_revoke),
        vop_mkdir: Some(c_mkdir),
        vop_rmdir: Some(c_rmdir),
        vop_setattr: Some(c_setattr),
        vop_strategy: Some(c_strategy),
        vop_symlink: Some(c_symlink),
        vop_write: Some(c_write),
        vop_kqfilter: Some(c_kqfilter),
    };

    /// Every slot NULL.
    static EMPTY: Vops = Vops::EMPTY;

    /// A `UIO_SYSSPACE` uio of nothing over `iov`.
    fn uio<'a>(iov: &'a mut [Iovec; 1], rw: UioRw, p: &'a Proc) -> Uio<'a> {
        Uio {
            uio_iov: iov,
            uio_offset: 0,
            uio_resid: 0,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: rw,
            uio_procp: Some(p),
        }
    }

    #[test]
    fn every_vop_adapter_reaches_its_slot() {
        let (_g, p) = setup();
        HITS.store(0, Ordering::SeqCst);
        let vp = getnewvnode(VT_NON, None, &COUNTING).unwrap();
        vp.set_vtype(VDIR);
        let mut va = Vattr::new();
        let mut cn = Componentname::new(0, 0, p, Cred::NULL);
        let mut cn2 = Componentname::new(0, 0, p, Cred::NULL);
        let mut vpp = None;
        let mut iov = [Iovec::new()];

        assert_eq!(VOP_LOCK(vp, 0), Ok(()));
        assert_eq!(VOP_UNLOCK(vp), Ok(()));
        assert_eq!(VOP_ISLOCKED(vp), 0);
        assert_eq!(VOP_ABORTOP(vp, &mut cn), Ok(()));
        assert_eq!(VOP_ACCESS(vp, 0, NOCRED, p), Ok(()));
        let mut fl = Flock::default();
        assert_eq!(VOP_ADVLOCK(vp, LockOwner::Table(0), 0, &mut fl, 0), Ok(()));
        assert_eq!(VOP_BMAP(vp, 0, None, None, None), Ok(()));
        assert_eq!(VOP_CLOSE(vp, 0, NOCRED, Some(p)), Ok(()));
        assert_eq!(VOP_CREATE(vp, &mut vpp, &mut cn, &mut va), Ok(()));
        assert_eq!(VOP_FSYNC(vp, NOCRED, 0, p), Ok(()));
        assert_eq!(VOP_GETATTR(vp, &mut va, NOCRED, p), Ok(()));
        assert_eq!(VOP_INACTIVE(vp, Some(p)), Ok(()));
        assert_eq!(VOP_IOCTL(vp, 0, &mut [0u8; 8], 0, NOCRED, p), Ok(()));
        assert_eq!(VOP_LINK(vp, vp, &mut cn), Ok(()));
        assert_eq!(VOP_LOOKUP(vp, &mut vpp, &mut cn), Ok(()));
        assert_eq!(VOP_MKNOD(vp, &mut vpp, &mut cn, &mut va), Ok(()));
        assert_eq!(VOP_OPEN(vp, 0, NOCRED, p), Ok(()));
        let mut rv: Register = 0;
        assert_eq!(VOP_PATHCONF(vp, _PC_LINK_MAX, &mut rv), Ok(()));
        // A name constant across file systems is answered without the slot.
        assert_eq!(VOP_PATHCONF(vp, _PC_PATH_MAX, &mut rv), Ok(()));
        assert!(rv > 0);
        #[cfg(any(feature = "debug", feature = "diagnostic"))]
        assert_eq!(VOP_PRINT(vp), Ok(()));
        #[cfg(not(any(feature = "debug", feature = "diagnostic")))]
        hit(19);
        assert_eq!(
            VOP_READ(vp, &mut uio(&mut iov, UioRw::UIO_READ, p), 0, NOCRED),
            Ok(())
        );
        let mut eof = 0;
        assert_eq!(
            VOP_READDIR(vp, &mut uio(&mut iov, UioRw::UIO_READ, p), NOCRED, &mut eof),
            Ok(())
        );
        assert_eq!(
            VOP_READLINK(vp, &mut uio(&mut iov, UioRw::UIO_READ, p), NOCRED),
            Ok(())
        );
        assert_eq!(VOP_RECLAIM(vp, Some(p)), Ok(()));
        assert_eq!(VOP_RENAME(vp, vp, &mut cn, vp, None, &mut cn2), Ok(()));
        assert_eq!(VOP_REVOKE(vp, 0), Ok(()));
        assert_eq!(VOP_MKDIR(vp, &mut vpp, &mut cn, &mut va), Ok(()));
        assert_eq!(
            VOP_RMDIR(vp, getnewvnode(VT_NON, None, &EMPTY).unwrap(), &mut cn),
            Ok(())
        );
        assert_eq!(VOP_SETATTR(vp, &mut va, NOCRED, p), Ok(()));
        assert_eq!(VOP_SYMLINK(vp, &mut vpp, &mut cn, &mut va, b"t"), Ok(()));
        assert_eq!(
            VOP_WRITE(vp, &mut uio(&mut iov, UioRw::UIO_WRITE, p), 0, NOCRED),
            Ok(())
        );
        let (kq, h) = owned_knote(p);
        assert_eq!(VOP_KQFILTER(vp, 0, h.kn()), Ok(()));
        owned_knote_drop(p, kq, h);
        // The buffer passes as it came: lent, its token left parked for its owner.
        kernel::fs::bio::bio_test_reset();
        kernel::fs::bufq::bufq_test_reset();
        crate::sys::buf::bufinit();
        with_private_buf(|bp| {
            assert_eq!(VOP_STRATEGY(vp, bp), Ok(()));
            bp.set_vp(Some(vp));
            assert_eq!(VOP_BWRITE(bp), Ok(()));
        });
        // VOP_REMOVE releases both vnodes (here the same one: a vrele, then the vput).
        vref(vp);
        vref(vp);
        assert_eq!(VOP_REMOVE(vp, vp, &mut cn), Ok(()));
        assert_eq!(vp.usecount(), 1);

        let all = (1u64 << 34) - 1;
        assert_eq!(
            HITS.load(Ordering::SeqCst),
            all,
            "{:#x}",
            HITS.load(Ordering::SeqCst)
        );
        let _ = vrele(vp);
    }

    #[test]
    fn a_null_slot_answers_eopnotsupp() {
        let (_g, p) = setup();
        let vp = getnewvnode(VT_NON, None, &EMPTY).unwrap();
        vp.set_vtype(VDIR);
        let mut va = Vattr::new();
        let mut cn = Componentname::new(0, 0, p, Cred::NULL);
        let mut vpp = None;
        let mut iov = [Iovec::new()];
        let e = Err(Errno::EOPNOTSUPP);

        assert_eq!(VOP_ISLOCKED(vp), Errno::EOPNOTSUPP as i32);
        assert_eq!(VOP_LOCK(vp, 0), e);
        assert_eq!(VOP_OPEN(vp, 0, NOCRED, p), e);
        assert_eq!(VOP_GETATTR(vp, &mut va, NOCRED, p), e);
        assert_eq!(VOP_LOOKUP(vp, &mut vpp, &mut cn), e);
        assert_eq!(
            VOP_READ(vp, &mut uio(&mut iov, UioRw::UIO_READ, p), 0, NOCRED),
            e
        );
        assert_eq!(VOP_FSYNC(vp, NOCRED, 0, p), e);
        assert_eq!(VOP_BMAP(vp, 0, None, None, None), e);
        // The constant names need no slot; the others do.
        let mut rv: Register = 0;
        assert_eq!(VOP_PATHCONF(vp, _PC_PATH_MAX, &mut rv), Ok(()));
        assert_eq!(VOP_PATHCONF(vp, _PC_LINK_MAX, &mut rv), e);
        // Not a directory: VOP_READDIR answers ENOTDIR before the slot.
        vp.set_vtype(crate::sys::vnode::VREG);
        let mut eof = 0;
        assert_eq!(
            VOP_READDIR(vp, &mut uio(&mut iov, UioRw::UIO_READ, p), NOCRED, &mut eof),
            Err(Errno::ENOTDIR)
        );
        // VOP_REMOVE without a slot still releases the vnodes.
        vref(vp);
        vref(vp);
        assert_eq!(VOP_REMOVE(vp, vp, &mut cn), e);
        assert_eq!(vp.usecount(), 1);
        let _ = vrele(vp);
    }

    #[test]
    fn the_generic_operations_answer_as_vfs_default() {
        let (_g, p) = setup();
        let vp = getnewvnode(VT_NON, None, &EMPTY).unwrap();
        let mut vpp = None;
        let mut bn: Daddr = 0;
        let mut run = 7;
        assert_eq!(
            vop_generic_bmap(&mut VopBmapArgs {
                a_vp: vp,
                a_bn: 42,
                a_vpp: Some(&mut vpp),
                a_bnp: Some(&mut bn),
                a_runp: Some(&mut run),
            }),
            Ok(())
        );
        assert!(vpp.is_some_and(|v| ptr::eq(v, vp)));
        assert_eq!((bn, run), (42, 0));

        let mut cn = Componentname::new(0, 0, p, Cred::NULL);
        let mut vpp = Some(vp);
        assert_eq!(
            vop_generic_lookup(&mut VopLookupArgs {
                a_dvp: vp,
                a_vpp: &mut vpp,
                a_cnp: &mut cn,
            }),
            Err(Errno::ENOTDIR)
        );
        assert!(vpp.is_none());
        // Nothing to free: no HASBUF.
        assert_eq!(
            vop_generic_abortop(&mut VopAbortopArgs {
                a_dvp: vp,
                a_cnp: &mut cn,
            }),
            Ok(())
        );
        let _ = vrele(vp);
    }

    static VFS_HITS: AtomicU64 = AtomicU64::new(0);

    fn vhit(bit: u32) {
        VFS_HITS.fetch_or(1 << bit, Ordering::SeqCst);
    }

    /// A `vfsops` table that records each member it reaches.
    static COUNTING_VFSOPS: Vfsops = Vfsops {
        vfs_mount: |_, _, _, _, _| {
            vhit(0);
            Ok(())
        },
        vfs_start: |_, _, _| {
            vhit(1);
            Ok(())
        },
        vfs_unmount: |_, _, _| {
            vhit(2);
            Ok(())
        },
        vfs_root: |_| {
            vhit(3);
            Err(Errno::ENOENT)
        },
        vfs_quotactl: |_, _, _, _, _| {
            vhit(4);
            Ok(())
        },
        vfs_statfs: |_, _, _| {
            vhit(5);
            Ok(())
        },
        vfs_sync: |_, _, _, cred, _| {
            assert!(core::ptr::eq(cred, NOCRED));
            vhit(6);
            Ok(())
        },
        vfs_vget: |_, _| {
            vhit(7);
            Err(Errno::ENOENT)
        },
        vfs_fhtovp: |_, _| {
            vhit(8);
            Err(Errno::ESTALE)
        },
        vfs_vptofh: |_, _| {
            vhit(9);
            Ok(())
        },
        vfs_init: Some(|_| {
            vhit(10);
            Ok(())
        }),
        vfs_sysctl: Some(|_, _, _, _, _, _| {
            vhit(11);
            Ok(())
        }),
        vfs_checkexp: |_, _, _, _| Ok(()),
    };

    static COUNTING_CONF: Vfsconf = Vfsconf::new(&COUNTING_VFSOPS, b"countfs", 97, MNT_LOCAL, 0);

    #[test]
    fn every_vfs_wrapper_reaches_its_member() {
        let (_g, p) = setup();
        VFS_HITS.store(0, Ordering::SeqCst);
        let mp = vfs_mount_alloc(None, &COUNTING_CONF);
        crate::sys::vnode::vfs_unbusy(mp);
        let ops = mp.op();

        let mut nd = crate::sys::namei::ndinit(0, 0, crate::sys::namei::NiDirp::Sys(b"/"), p);
        assert_eq!(
            crate::sys::mount::VFS_MOUNT(mp, b"/", &mut [], &mut nd, p),
            Ok(())
        );
        assert_eq!(VFS_START(mp, 0, p), Ok(()));
        assert_eq!(VFS_UNMOUNT(mp, 0, p), Ok(()));
        assert_eq!(VFS_ROOT(mp).err(), Some(Errno::ENOENT));
        assert_eq!(VFS_QUOTACTL(mp, 0, 0, 0, p), Ok(()));
        let mut st = mp.stat();
        assert_eq!(VFS_STATFS(mp, &mut st, p), Ok(()));
        assert_eq!(VFS_SYNC(mp, 0, 0, NOCRED, p), Ok(()));
        assert_eq!(VFS_VGET(mp, 2).err(), Some(Errno::ENOENT));
        let mut fid = Fid {
            fid_len: 0,
            fid_reserved: 0,
            fid_data: [0; crate::sys::mount::MAXFIDSZ],
        };
        assert_eq!(VFS_FHTOVP(mp, &fid).err(), Some(Errno::ESTALE));
        let vp = getnewvnode(VT_NON, None, &EMPTY).unwrap();
        insmntque(vp, Some(mp));
        assert_eq!(VFS_VPTOFH(vp, &mut fid), Ok(()));
        insmntque(vp, None);
        assert_eq!(ops.init(&COUNTING_CONF), Ok(()));
        let mut oldlen = 0;
        assert_eq!(ops.sysctl(&[1], 0, &mut oldlen, 0, 0, p), Ok(()));
        // The table as bsd knows it, for the NFS server's vfs_checkexp.
        assert!(
            ops.as_any()
                .and_then(|a| a.downcast_ref::<Vfsops>())
                .is_some_and(|o| ptr::eq(o, &COUNTING_VFSOPS))
        );
        assert_eq!(VFS_HITS.load(Ordering::SeqCst), (1 << 12) - 1);
        let _ = vrele(vp);
    }
}
/* </TESTS> */
