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
//! The operations a file system provides: `struct vops` ([`Vnops`], `<sys/vnode.h>`) and
//! `struct vfsops` ([`Vfs`], `<sys/mount.h>`); and the `VOP_*` wrappers (`vfs_vops.c`) and
//! the `VFS_*` macros, as functions that check what the C checks and call the vnode's (or the
//! mount's) table.
//!
//! Upstream: sys/sys/vnode.h @ 3ce1f3f79392
//! Upstream: sys/sys/mount.h @ 3ce1f3f79392
//! Upstream: sys/kern/vfs_vops.c @ 3ce1f3f79392
//! LZ: sys/sys/vnode.rs@3c62ede68802
//! LZ: sys/sys/mount.rs@3c62ede68802
//! LZ: sys/kern/vfs_vops.rs@3c62ede68802
//!
//! ## Deviations
//! - `ASSERT_VP_ISLOCKED` is compiled only with `VFSLCKDEBUG`, which is not configured (LZ's
//!   deviation, kept); the `KASSERT(p == curproc)`s are `kassert!`s.
//! - `VOP_PRINT` calls the table under feature `diagnostic` or `debug` (the C's `DEBUG ||
//!   DIAGNOSTIC`); without them it answers `Ok`.
//!
//! ## Redesign
//! - LZ's `struct Vops` of `Option<fn(&mut VopXxxArgs)>` slots, one argument structure per
//!   operation (sys/sys/vnode.rs:640-1170) -> the trait [`Vnops<G>`]: one method per slot
//!   with the C's arguments (out-parameters stay `&mut`), each defaulting to the wrapper's
//!   answer to a NULL slot (`EOPNOTSUPP`), so a table states only what it fills, as a C table
//!   does. `bsd`'s file systems keep their `static` tables of the old shape: `bsd` implements
//!   the trait on its `Vops` (packing the argument structures), so every
//!   `getnewvnode(tag, mp, &FOO_VOPS)` keeps its spelling. `struct vfsops` the same, as
//!   [`Vfs<G>`] (the C's 13 members; `vfs_checkexp` is the NFS server's and stays `bsd`'s,
//!   reached through [`Vfs::as_any`]).
//! - `struct ucred *a_cred` with its `NOCRED`/`FSCRED` sentinels is [`Cred`], an opaque copy
//!   of the pointer: the kernel compares the sentinels and passes it on, it never reads a
//!   credential's fields through it (a typed credential arrives with `namei`, N5.3), so the
//!   adapters of `bsd`, whose callers hold the C's pointer, need no deref.
//! - `struct buf *` of `VOP_STRATEGY`/`VOP_BWRITE` is [`IoBuf`]: the owner's token (the
//!   kernel's buffer cache, N5.1), or a buffer `bsd` lent with its token parked in it, passed
//!   on as it came so that no lock section parks or takes a token on the way (each is an spl
//!   pair, N5.1's lesson).

use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr;

use hal::sys::errno::Errno;
use hal::sys::fcntl::Flock;
use hal::sys::mount::{Fid, Statfs};
use hal::sys::syslimits::{PATH_MAX, PIPE_BUF};
use hal::sys::types::{Daddr, Ino, Register, Uid};
use hal::sys::unistd::{_PC_ASYNC_IO, _PC_PATH_MAX, _PC_PIPE_BUF, _PC_PRIO_IO, _PC_SYNC_IO};

use super::buf::{Buf, BufRef};
use super::mount::{Mount, Vfsconf};
use super::vnode::{VBIOERROR, VDIR, VNODE_MTX, Vattr, Vnode};
use crate::assert::{kassert, kpanic};
use crate::cred::Ucred;
use crate::event::Knote;
use crate::file::FileGlue;
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `struct componentname` of the glue `G` (`bsd`'s until N5.3).
pub type Cn<G> = <<G as ProcGlue>::FileGlue as FileGlue>::Componentname;
/// `struct nameidata` of the glue `G` (`bsd`'s until N5.3).
pub type Nd<'a, G> = <<G as ProcGlue>::FileGlue as FileGlue>::Nameidata<'a>;

/// `struct ucred *` as a vnode operation receives it: a real credential, NULL or one of the
/// sentinels `NOCRED` and `FSCRED`. An opaque copy of the C's pointer (see the module's
/// notes): compared and passed on, never dereferenced here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cred<'a> {
    ptr: *const Ucred,
    _life: PhantomData<&'a Ucred>,
}

impl<'a> Cred<'a> {
    /// `NOCRED`: no credential available.
    pub const NOCRED: Cred<'static> = Cred {
        ptr: ptr::without_provenance(usize::MAX),
        _life: PhantomData,
    };
    /// `FSCRED`: filesystem credential.
    pub const FSCRED: Cred<'static> = Cred {
        ptr: ptr::without_provenance(usize::MAX - 1),
        _life: PhantomData,
    };
    /// NULL.
    pub const NULL: Cred<'static> = Cred {
        ptr: ptr::null(),
        _life: PhantomData,
    };

    /// A real credential.
    pub fn real(c: &'a Ucred) -> Self {
        Self {
            ptr: ptr::from_ref(c),
            _life: PhantomData,
        }
    }

    /// The C's pointer as `bsd`'s callers pass it (a real credential, NULL or a sentinel).
    pub const fn from_raw(ptr: *const Ucred) -> Self {
        Self {
            ptr,
            _life: PhantomData,
        }
    }

    /// The C's pointer, for `bsd`'s file systems (`a_cred`).
    pub fn as_raw(self) -> *const Ucred {
        self.ptr
    }

    /// `cred == FSCRED`.
    pub fn is_fscred(self) -> bool {
        ptr::eq(self.ptr, Cred::FSCRED.ptr)
    }

    /// `cred == NOCRED`.
    pub fn is_nocred(self) -> bool {
        ptr::eq(self.ptr, Cred::NOCRED.ptr)
    }
}

/// The buffer of `VOP_STRATEGY` and `VOP_BWRITE` (`struct buf *`): its owner's token, or a
/// buffer `bsd` lent (its token parked in it, N5.1's `lend`), passed on as it came.
pub enum IoBuf {
    /// The busy buffer's token.
    Token(BufRef),
    /// A busy buffer lent to `bsd`, its token parked in it.
    Lent(&'static Buf),
}

impl IoBuf {
    /// The buffer.
    pub fn buf(&self) -> &Buf {
        match self {
            IoBuf::Token(t) => t.buf(),
            IoBuf::Lent(bp) => bp,
        }
    }

    /// The token: the one the caller passed, or the one parked in a lent buffer (one section
    /// of the cache's lock), `None` when none is parked there.
    pub fn into_token(self) -> Option<BufRef> {
        match self {
            IoBuf::Token(t) => Some(t),
            IoBuf::Lent(bp) => bp.take_parked(),
        }
    }
}

/// `struct vops`: the operations of a vnode's file system. Each method is a slot of the C's
/// table with its `struct vop_*_args` as arguments; its default is the answer of the
/// `VOP_*` wrapper to a NULL slot, `EOPNOTSUPP`.
#[allow(
    unused_variables,
    reason = "the defaults are the NULL slot's, which reads nothing"
)]
pub trait Vnops<G: ProcGlue>: Sync + 'static {
    /// `vop_lock`: lock the vnode (`flags` are `LK_*`).
    fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_unlock`.
    fn unlock(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_islocked`: the lock status (`LK_EXCLUSIVE`, `LK_SHARED`, `LK_EXCLOTHER` or 0), or
    /// `EOPNOTSUPP` as an `int` for a NULL slot, as the C answers.
    fn islocked(&self, vp: &'static Vnode<G>) -> i32 {
        Errno::EOPNOTSUPP as i32
    }
    /// `vop_lookup`: the vnode `cnp` names in `dvp`, into `vpp`.
    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_create`.
    fn create(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_mknod`.
    fn mknod(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_open`.
    fn open(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_close`: `p` is `None` when no thread closes.
    fn close(
        &self,
        vp: &'static Vnode<G>,
        fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_access`.
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_getattr`.
    fn getattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_setattr`.
    fn setattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_read`.
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_write`.
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_ioctl`.
    #[allow(
        clippy::too_many_arguments,
        reason = "the C's six arguments and the table"
    )]
    fn ioctl(
        &self,
        vp: &'static Vnode<G>,
        command: u64,
        data: &mut [u8],
        fflag: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_kqfilter`.
    fn kqfilter(&self, vp: &'static Vnode<G>, fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_revoke`.
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_fsync`.
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        cred: Cred<'_>,
        waitfor: i32,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_remove`.
    fn remove(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_link`.
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_rename`.
    #[allow(
        clippy::too_many_arguments,
        reason = "the C's six arguments and the table"
    )]
    fn rename(
        &self,
        fdvp: &'static Vnode<G>,
        fvp: &'static Vnode<G>,
        fcnp: &mut Cn<G>,
        tdvp: &'static Vnode<G>,
        tvp: Option<&'static Vnode<G>>,
        tcnp: &mut Cn<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_mkdir`.
    fn mkdir(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_rmdir`.
    fn rmdir(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_symlink`.
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<G>,
        vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_readdir`.
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_readlink`.
    fn readlink(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        cred: Cred<'_>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_abortop`.
    fn abortop(&self, dvp: &'static Vnode<G>, cnp: &mut Cn<G>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_inactive`: unlocks the vnode, as the C's.
    fn inactive(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_reclaim`.
    fn reclaim(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_bmap`: the out-parameters the caller wants are the `Some`s.
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_print`.
    fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_pathconf`.
    fn pathconf(
        &self,
        vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_advlock`: `id` is the lock's owner, an address compared and never followed.
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: *const c_void,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_strategy`: starts the buffer's I/O (its token passes on to the driver, whose
    /// `biodone` gives it back). A NULL slot leaves the buffer to its caller, as the C's
    /// `EOPNOTSUPP` does.
    fn strategy(&self, vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `vop_bwrite`.
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// The table as `bsd` knows it (its `Vops`), for `bsd`'s adapters; the kernel's own
    /// tables answer `None`.
    fn as_any(&self) -> Option<&dyn core::any::Any> {
        None
    }
}

/// `struct vfsops`: the operations a file system type supports on its mounts (the type's
/// name, number, flags and argument size are its [`Vfsconf`]'s).
#[allow(unused_variables, reason = "the defaults read nothing")]
pub trait Vfs<G: ProcGlue>: Sync + 'static {
    /// `vfs_mount(mp, path, data, ndp, p)`: `data` is the kernel copy of the user's
    /// arguments (`vfc_datasize` bytes).
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno>;
    /// `vfs_start(mp, flags, p)`.
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno>;
    /// `vfs_unmount(mp, mntflags, p)`.
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno>;
    /// `vfs_root(mp, vpp)`: the root vnode, locked.
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno>;
    /// `vfs_quotactl(mp, cmds, uid, arg, p)`: `arg` is a user address.
    fn quotactl(
        &self,
        mp: &'static Mount<G>,
        cmds: i32,
        uid: Uid,
        arg: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno>;
    /// `vfs_statfs(mp, sbp, p)`.
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno>;
    /// `vfs_sync(mp, waitfor, stall, cred, p)`.
    fn sync(
        &self,
        mp: &'static Mount<G>,
        waitfor: i32,
        stall: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno>;
    /// `vfs_vget(mp, ino, vpp)`: the vnode of the inode, locked.
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno>;
    /// `vfs_fhtovp(mp, fhp, vpp)`.
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno>;
    /// `vfs_vptofh(vp, fhp)`.
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno>;
    /// `vfs_init(vfsconf)`: the type's initialisation (`vfsinit`); NULL in the C is `Ok`.
    fn init(&self, vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        Ok(())
    }
    /// `vfs_sysctl(name, namelen, oldp, oldlenp, newp, newlen, p)`: the type's `vfs.<type>`
    /// node; NULL in the C is `EOPNOTSUPP`.
    fn sysctl(
        &self,
        name: &[i32],
        oldp: usize,
        oldlenp: &mut usize,
        newp: usize,
        newlen: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// The table as `bsd` knows it (its `Vfsops`: `vfs_checkexp`, the NFS server's), `None`
    /// for the kernel's.
    fn as_any(&self) -> Option<&dyn core::any::Any> {
        None
    }
}

/// `KASSERT(p == curproc)`.
fn assert_curproc<G: ProcGlue>(p: &Thread<G>) {
    #[cfg(feature = "diagnostic")]
    G::with_curproc(|cur| kassert!(cur.is_some_and(|c| ptr::eq(c, p))));
    let _ = p;
}

/// `KASSERT(p == curproc)` of an operation whose thread may be none (`curproc` NULL then).
fn assert_curproc_opt<G: ProcGlue>(p: Option<&Thread<G>>) {
    #[cfg(feature = "diagnostic")]
    G::with_curproc(|cur| {
        kassert!(match (p, cur) {
            (Some(p), Some(c)) => ptr::eq(p, c),
            (None, None) => true,
            _ => false,
        })
    });
    let _ = p;
}

/// `VOP_ISLOCKED(vp)`: the lock status, or `EOPNOTSUPP` as an `int`, as in C.
pub fn vop_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    vp.ops().islocked(vp)
}

/// `VOP_LOOKUP(dvp, vpp, cnp)`.
pub fn vop_lookup<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<G>,
) -> Result<(), Errno> {
    dvp.ops().lookup(dvp, vpp, cnp)
}

/// `VOP_CREATE(dvp, vpp, cnp, vap)`.
pub fn vop_create<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(dvp): VFSLCKDEBUG.
    dvp.ops().create(dvp, vpp, cnp, vap)
}

/// `VOP_MKNOD(dvp, vpp, cnp, vap)`.
pub fn vop_mknod<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(dvp): VFSLCKDEBUG.
    dvp.ops().mknod(dvp, vpp, cnp, vap)
}

/// `VOP_OPEN(vp, mode, cred, p)`.
pub fn vop_open<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    assert_curproc(p);
    vp.ops().open(vp, mode, cred, p)
}

/// `VOP_CLOSE(vp, fflag, cred, p)`: `p` is `None` when no thread closes.
pub fn vop_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fflag: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    if let Some(p) = p {
        assert_curproc(p);
    }
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().close(vp, fflag, cred, p)
}

/// `VOP_ACCESS(vp, mode, cred, p)`.
pub fn vop_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    assert_curproc(p);
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().access(vp, mode, cred, p)
}

/// `VOP_GETATTR(vp, vap, cred, p)`.
pub fn vop_getattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    assert_curproc(p);
    vp.ops().getattr(vp, vap, cred, p)
}

/// `VOP_SETATTR(vp, vap, cred, p)`.
pub fn vop_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    assert_curproc(p);
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().setattr(vp, vap, cred, p)
}

/// `VOP_READ(vp, uio, ioflag, cred)`.
pub fn vop_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().read(vp, uio, ioflag, cred)
}

/// `VOP_WRITE(vp, uio, ioflag, cred)`.
pub fn vop_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().write(vp, uio, ioflag, cred)
}

/// `VOP_IOCTL(vp, command, data, fflag, cred, p)`.
pub fn vop_ioctl<G: ProcGlue>(
    vp: &'static Vnode<G>,
    command: u64,
    data: &mut [u8],
    fflag: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    assert_curproc(p);
    vp.ops().ioctl(vp, command, data, fflag, cred, p)
}

/// `VOP_KQFILTER(vp, fflag, kn)`.
pub fn vop_kqfilter<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fflag: i32,
    kn: &Knote<G>,
) -> Result<(), Errno> {
    vp.ops().kqfilter(vp, fflag, kn)
}

/// `VOP_REVOKE(vp, flags)`.
pub fn vop_revoke<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    vp.ops().revoke(vp, flags)
}

/// `VOP_FSYNC(vp, cred, waitfor, p)`: a write that failed since (`VBIOERROR`) makes a
/// successful sync `EIO`.
pub fn vop_fsync<G: ProcGlue>(
    vp: &'static Vnode<G>,
    cred: Cred<'_>,
    waitfor: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    assert_curproc(p);
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    let r = vp.ops().fsync(vp, cred, waitfor, p);
    // The C reads v_bioflag at splbio; it is an atomic interrupts update.
    if r.is_ok() && vp.bufs.bioflag() & VBIOERROR != 0 {
        return Err(Errno::EIO);
    }
    r
}

/// `VOP_REMOVE(dvp, vp, cnp)`: the vnodes are released whatever the answer (`vrele` of `vp`
/// when it is `dvp`, `vput` otherwise, then `vput(dvp)`).
pub fn vop_remove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<G>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(dvp), ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    let error = dvp.ops().remove(dvp, vp, cnp);

    if ptr::eq(dvp, vp) {
        let _ = super::subr::vrele(vp);
    } else {
        super::subr::vput(vp);
    }
    super::subr::vput(dvp);

    error
}

/// `VOP_LINK(dvp, vp, cnp)`.
pub fn vop_link<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<G>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(dvp): VFSLCKDEBUG.
    dvp.ops().link(dvp, vp, cnp)
}

/// `VOP_RENAME(fdvp, fvp, fcnp, tdvp, tvp, tcnp)`.
pub fn vop_rename<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<G>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(tdvp): VFSLCKDEBUG.
    fdvp.ops().rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
}

/// `VOP_MKDIR(dvp, vpp, cnp, vap)`.
pub fn vop_mkdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<G>,
    vap: &mut Vattr,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(dvp): VFSLCKDEBUG.
    dvp.ops().mkdir(dvp, vpp, cnp, vap)
}

/// `VOP_RMDIR(dvp, vp, cnp)`.
pub fn vop_rmdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<G>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(dvp), ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    kassert!(!ptr::eq(dvp, vp));
    dvp.ops().rmdir(dvp, vp, cnp)
}

/// `VOP_SYMLINK(dvp, vpp, cnp, vap, target)`.
pub fn vop_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<G>,
    vap: &mut Vattr,
    target: &[u8],
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(dvp): VFSLCKDEBUG.
    dvp.ops().symlink(dvp, vpp, cnp, vap, target)
}

/// `VOP_READDIR(vp, uio, cred, eofflag)`: `ENOTDIR` but for a directory.
pub fn vop_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    cred: Cred<'_>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    if vp.vtype() != VDIR {
        return Err(Errno::ENOTDIR);
    }
    vp.ops().readdir(vp, uio, cred, eofflag)
}

/// `VOP_READLINK(vp, uio, cred)`.
pub fn vop_readlink<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().readlink(vp, uio, cred)
}

/// `VOP_ABORTOP(dvp, cnp)`.
pub fn vop_abortop<G: ProcGlue>(dvp: &'static Vnode<G>, cnp: &mut Cn<G>) -> Result<(), Errno> {
    dvp.ops().abortop(dvp, cnp)
}

/// `VOP_INACTIVE(vp, p)`.
pub fn vop_inactive<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    assert_curproc_opt(p);
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().inactive(vp, p)
}

/// `VOP_RECLAIM(vp, p)`.
pub fn vop_reclaim<G: ProcGlue>(vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
    assert_curproc_opt(p);
    vp.ops().reclaim(vp, p)
}

/// `VOP_LOCK(vp, flags)`: never with `vnode_mtx` held (`MUTEX_ASSERT_UNLOCKED`).
pub fn vop_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    if VNODE_MTX.owned() {
        kpanic(format_args!("mutex {:p} held in VOP_LOCK", &VNODE_MTX));
    }
    vp.ops().lock(vp, flags)
}

/// `VOP_UNLOCK(vp)`.
pub fn vop_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    vp.ops().unlock(vp)
}

/// `VOP_BMAP(vp, bn, vpp, bnp, runp)`.
pub fn vop_bmap<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    vpp: Option<&mut Option<&'static Vnode<G>>>,
    bnp: Option<&mut Daddr>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().bmap(vp, bn, vpp, bnp, runp)
}

/// `VOP_PRINT(vp)` (`DEBUG || DIAGNOSTIC`).
pub fn vop_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if cfg!(any(feature = "debug", feature = "diagnostic")) {
        vp.ops().print(vp)
    } else {
        Ok(())
    }
}

/// `VOP_PATHCONF(vp, name, retval)`: the names constant across file systems answered here.
pub fn vop_pathconf<G: ProcGlue>(
    vp: &'static Vnode<G>,
    name: i32,
    retval: &mut Register,
) -> Result<(), Errno> {
    // Handle names that are constant across filesystem
    match name {
        _PC_PATH_MAX => {
            *retval = PATH_MAX as Register;
            return Ok(());
        }
        _PC_PIPE_BUF => {
            *retval = PIPE_BUF as Register;
            return Ok(());
        }
        _PC_ASYNC_IO | _PC_PRIO_IO | _PC_SYNC_IO => {
            *retval = 0;
            return Ok(());
        }
        _ => {}
    }
    // ASSERT_VP_ISLOCKED(vp): VFSLCKDEBUG.
    vp.ops().pathconf(vp, name, retval)
}

/// `VOP_ADVLOCK(vp, id, op, fl, flags)`.
pub fn vop_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: *const c_void,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    vp.ops().advlock(vp, id, op, fl, flags)
}

/// `VOP_STRATEGY(vp, bp)`.
pub fn vop_strategy<G: ProcGlue>(vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
    vp.ops().strategy(vp, bp)
}

/// `VOP_BWRITE(bp)`: the operation of the buffer's vnode (`bp->b_vp`); a buffer without
/// one is `panic(9)`, where the C would follow NULL.
pub fn vop_bwrite<G: ProcGlue>(bp: IoBuf) -> Result<(), Errno> {
    let vp = bp
        .buf()
        .vnode_bufs()
        .and_then(|vb| vb.owner().as_any().downcast_ref::<Vnode<G>>());
    let Some(vp) = vp else {
        kpanic(format_args!(
            "VOP_BWRITE: buffer {:p} without a vnode",
            bp.buf()
        ));
    };
    vp.ops().bwrite(bp)
}

/// `VFS_MOUNT(MP, PATH, DATA, NDP, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_MOUNT<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    mp.op().mount(mp, path, data, ndp, p)
}

/// `VFS_START(MP, FLAGS, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_START<G: ProcGlue>(
    mp: &'static Mount<G>,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    mp.op().start(mp, flags, p)
}

/// `VFS_UNMOUNT(MP, FORCE, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_UNMOUNT<G: ProcGlue>(
    mp: &'static Mount<G>,
    force: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    mp.op().unmount(mp, force, p)
}

/// `VFS_ROOT(MP, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_ROOT<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    mp.op().root(mp)
}

/// `VFS_QUOTACTL(MP, C, U, A, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_QUOTACTL<G: ProcGlue>(
    mp: &'static Mount<G>,
    c: i32,
    u: Uid,
    a: usize,
    p: &Thread<G>,
) -> Result<(), Errno> {
    mp.op().quotactl(mp, c, u, a, p)
}

/// `VFS_STATFS(MP, SBP, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_STATFS<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    p: &Thread<G>,
) -> Result<(), Errno> {
    mp.op().statfs(mp, sbp, p)
}

/// `VFS_SYNC(MP, W, S, C, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_SYNC<G: ProcGlue>(
    mp: &'static Mount<G>,
    w: i32,
    s: i32,
    c: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    mp.op().sync(mp, w, s, c, p)
}

/// `VFS_VGET(MP, INO, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_VGET<G: ProcGlue>(mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
    mp.op().vget(mp, ino)
}

/// `VFS_FHTOVP(MP, FIDP, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_FHTOVP<G: ProcGlue>(
    mp: &'static Mount<G>,
    fidp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    mp.op().fhtovp(mp, fidp)
}

/// `VFS_VPTOFH(VP, FIDP)`: the operation of the vnode's mount; a vnode without one is
/// `panic(9)`, where the C would follow NULL.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_VPTOFH<G: ProcGlue>(vp: &'static Vnode<G>, fidp: &mut Fid) -> Result<(), Errno> {
    let Some(mp) = vp.mount() else {
        kpanic(format_args!("VFS_VPTOFH: vnode {:p} has no mount", vp));
    };
    mp.op().vptofh(vp, fidp)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_credential_sentinels_are_the_c_s_words() {
        assert!(Cred::FSCRED.is_fscred());
        assert!(!Cred::FSCRED.is_nocred());
        assert!(Cred::NOCRED.is_nocred());
        assert_eq!(Cred::NOCRED.as_raw().addr(), usize::MAX);
        assert_eq!(Cred::FSCRED.as_raw().addr(), usize::MAX - 1);
        assert!(Cred::NULL.as_raw().is_null());
        assert_eq!(Cred::from_raw(Cred::FSCRED.as_raw()), Cred::FSCRED);
    }
}
/* </TESTS> */
