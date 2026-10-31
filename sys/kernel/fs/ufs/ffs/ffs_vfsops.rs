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
 * Copyright (c) 1989, 1991, 1993, 1994
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
 *	@(#)ffs_vfsops.c	8.14 (Berkeley) 11/28/94
 */
/* </LICENSES> */

/* <CODE> */
//! The fast file system's half of the UFS layer: `ffs_vtbl`, the inode operations the UFS
//! code calls through `ip->i_vtbl` (N5.5b1); the mount, the `vfsops` and the file system's
//! own vnode operations follow at N5.5b2.
//!
//! Upstream: sys/ufs/ffs/ffs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - Until N5.5b2 moves the rest of `ffs_vfsops.c` here (`ffs_mount`, `ffs_mountfs`,
//!   `ffs_vget`, the `vfsops`, ...), `bsd`'s `sys/ufs/ffs/ffs_vfsops.rs` mounts every FFS
//!   file system with `bsd`'s own `ffs_vtbl`; nothing makes an inode with [`Ffs`]'s table
//!   but the kernel's tests (`testffs`).
//!
//! ## Redesign
//! - N5.5b1: LZ's `FfsVtbl` unit struct and its `FFS_VTBL` static (`impl InodeOps` for
//!   `bsd`'s inode) -> [`Ffs<G>`], generic over the glue, and its constant [`Ffs::VTBL`]
//!   (`&Ffs::<G>::VTBL` is the `&'static dyn InodeOps<G>` an inode holds, the
//!   `SpecVops::<G>::TABLE` shape): one type for the C's `ffs_vtbl`, which N5.5b2's `vfsops`
//!   and vnode tables join (D-N5.5b-2's brief, 2.3). Each method is the C's function of the
//!   same slot over the kernel's buffers: `iv_buf_alloc` and `iv_bufatoff` return the busy
//!   buffers' tokens (`IoBuf::Token`).

use core::marker::PhantomData;

use hal::sys::errno::Errno;
use hal::sys::types::{Mode, Off};

use super::ffs_alloc::{ffs_inode_alloc, ffs_inode_free};
use super::ffs_balloc::ffs_balloc;
use super::ffs_inode::{ffs_truncate, ffs_update};
use super::ffs_subr::ffs_bufatoff;
use crate::fs::ufs::ufs::dinode::Ufsino;
use crate::fs::ufs::ufs::inode::{Inode, InodeOps};
use crate::fs::vnode::Vnode;
use crate::fs::vops::{Cred, IoBuf};
use crate::proc::ProcGlue;

/// The fast file system of the glue `G`: `ffs_vtbl` (the inode operations, [`Ffs::VTBL`]).
pub struct Ffs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Ffs<G> {
    /// `ffs_vtbl`: the table an FFS inode's `i_vtbl` names.
    pub const VTBL: Ffs<G> = Ffs(PhantomData);
}

impl<G: ProcGlue> InodeOps<G> for Ffs<G> {
    /// `iv_truncate`: `ffs_truncate`.
    fn truncate(&self, ip: &Inode<G>, off: Off, flags: i32, cred: Cred<'_>) -> Result<(), Errno> {
        ffs_truncate(ip, off, flags, cred)
    }

    /// `iv_update`: `ffs_update`.
    fn update(&self, ip: &Inode<G>, waitfor: i32) -> Result<(), Errno> {
        ffs_update(ip, waitfor)
    }

    /// `iv_inode_alloc`: `ffs_inode_alloc`.
    fn inode_alloc(
        &self,
        pip: &Inode<G>,
        mode: Mode,
        cred: Cred<'_>,
    ) -> Result<&'static Vnode<G>, Errno> {
        ffs_inode_alloc(pip, mode, cred)
    }

    /// `iv_inode_free`: `ffs_inode_free`.
    fn inode_free(&self, pip: &Inode<G>, ino: Ufsino, mode: Mode) -> Result<(), Errno> {
        ffs_inode_free(pip, ino, mode)
    }

    /// `iv_buf_alloc`: `ffs_balloc`.
    fn buf_alloc(
        &self,
        ip: &Inode<G>,
        startoffset: Off,
        size: i32,
        cred: Cred<'_>,
        flags: i32,
    ) -> Result<IoBuf, Errno> {
        ffs_balloc(ip, startoffset, size, cred, flags).map(IoBuf::Token)
    }

    /// `iv_bufatoff`: `ffs_bufatoff`.
    fn bufatoff(&self, ip: &Inode<G>, offset: Off) -> Result<(IoBuf, usize), Errno> {
        ffs_bufatoff(ip, offset).map(|(t, off)| (IoBuf::Token(t), off))
    }
}
/* </CODE> */
