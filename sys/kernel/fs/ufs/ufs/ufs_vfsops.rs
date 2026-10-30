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
 * Copyright (c) 1991, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)ufs_vfsops.c	8.4 (Berkeley) 4/16/94
 */
/* </LICENSES> */

/* <CODE> */
//! The file-system-type operations the UFS file systems share: `ufs_start`, `ufs_root`, the
//! generic half of `fhtovp`, and `ufs_init`'s state.
//!
//! Upstream: sys/ufs/ufs/ufs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufs_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - `ufs_init`'s `static int done` and the inode hash are the glue's [`UfsTables`] (in its
//!   VFS tables, `fs::subr::VfsTables::ufs`), so a host test's file systems start afresh
//!   ([`ufs_test_reset`]). `ufs_init` itself stays `bsd`'s until N5.5a2 (D-N5.5a-13): it
//!   calls the quota and dirhash initialisations, `bsd`'s until then, and this module's
//!   [`ufs_ihashinit`](super::ufs_ihash::ufs_ihashinit) once [`UfsTables::init_done`] says so.
//! - `ufs_check_export` reads `um_export` (the NFS server's export list, `bsd`'s until N6):
//!   it stays `bsd`'s, over the ufsmount's glue member.
//!
//! ## Redesign
//! - N5.5a: LZ's `UFS_INIT_DONE` static -> [`UfsTables::init_done`], beside the inode hash
//!   it guards; `ufs_root` and `ufs_fhtovp` are the kernel's over its `VFS_VGET` and `vtoi`.

use core::sync::atomic::AtomicBool;

use hal::slot::ReadMostly;
use hal::sys::errno::Errno;

use super::dinode::ROOTINO;
use super::inode::{Ufid, vtoi};
use super::ufs_ihash::Ihash;
use crate::fs::mount::Mount;
use crate::fs::subr::vput;
use crate::fs::vnode::Vnode;
use crate::fs::vops::VFS_VGET;
use crate::proc::{ProcGlue, Thread};

/// The UFS layer's globals of a glue: `ufs_init`'s `done` and the inode hash
/// (`ufs_ihash.c`'s `ihashtbl`, `ihash`, `ihashkey`).
pub struct UfsTables<G: ProcGlue> {
    /// `ufs_init`'s `done`: the UFS layer is initialised.
    pub init_done: AtomicBool,
    /// The inode hash, once `ufs_ihashinit` made it.
    pub(super) ihash: ReadMostly<Option<Ihash<G>>>,
}

impl<G: ProcGlue> UfsTables<G> {
    /// Nothing initialised.
    pub const fn new() -> Self {
        Self {
            init_done: AtomicBool::new(false),
            ihash: ReadMostly::new(None),
        }
    }
}

impl<G: ProcGlue> Default for UfsTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `ufs_start` (`vfs_start`): make a filesystem operational. Nothing to do at the moment.
pub fn ufs_start<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _flags: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    Ok(())
}

/// `ufs_root` (`vfs_root`): return the root of a filesystem, referenced and locked.
pub fn ufs_root<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    VFS_VGET(mp, u64::from(ROOTINO))
}

/// `ufs_fhtovp`: the generic part of `fhtovp` called after the underlying filesystem has
/// validated the file handle: the vnode of the handle's inode, referenced and locked,
/// `ESTALE` when the inode was freed or reused since.
pub fn ufs_fhtovp<G: ProcGlue>(
    mp: &'static Mount<G>,
    ufhp: &Ufid,
) -> Result<&'static Vnode<G>, Errno> {
    let nvp = VFS_VGET(mp, u64::from(ufhp.ufid_ino))?;
    let ip = vtoi(nvp);
    if ip.dip_mode() == 0 || ip.dip_gen() != ufhp.ufid_gen {
        drop(ip);
        vput(nvp);
        return Err(Errno::ESTALE);
    }
    Ok(nvp)
}

/// The UFS layer of `G` forgotten (a host test's file systems start afresh, as
/// `setup_real_memory` gives each test fresh kernel memory): `init_done` cleared and the
/// inode hash forgotten, never freed (testing.md).
#[cfg(any(test, feature = "doubles"))]
pub fn ufs_test_reset<G: ProcGlue>() {
    let t = &crate::fs::subr::vfs::<G>().ufs;
    core::mem::forget(t.ihash.replace(None));
    t.init_done
        .store(false, core::sync::atomic::Ordering::SeqCst);
}
/* </CODE> */
