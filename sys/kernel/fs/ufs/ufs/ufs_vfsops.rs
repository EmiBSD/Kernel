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
use super::ufs_quota::DqTables;
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
    /// The dquot hash and free list, once `ufs_quota_init` made them.
    pub(super) dq: ReadMostly<Option<DqTables<G>>>,
}

impl<G: ProcGlue> UfsTables<G> {
    /// Nothing initialised.
    pub const fn new() -> Self {
        Self {
            init_done: AtomicBool::new(false),
            ihash: ReadMostly::new(None),
            dq: ReadMostly::new(None),
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

/// `ufs_init`: initialize UFS file systems, done only once: the inode hash, the quotas'
/// tables and the dirhash (`UFS_DIRHASH`), sized by `initialvnodes`.
pub fn ufs_init<G: ProcGlue>(initialvnodes: i32) -> Result<(), Errno> {
    if crate::fs::subr::vfs::<G>()
        .ufs
        .init_done
        .swap(true, core::sync::atomic::Ordering::Relaxed)
    {
        return Ok(());
    }
    super::ufs_ihash::ufs_ihashinit::<G>(initialvnodes);
    super::quota::ufs_quota_init::<G>(initialvnodes);
    #[cfg(feature = "ufs_dirhash")]
    super::ufs_dirhash::ufsdirhash_init();

    Ok(())
}

/// The UFS layer of `G` forgotten (a host test's file systems start afresh, as
/// `setup_real_memory` gives each test fresh kernel memory): `init_done` cleared and the
/// inode hash forgotten, never freed (testing.md).
#[cfg(any(test, feature = "doubles"))]
pub fn ufs_test_reset<G: ProcGlue>() {
    let t = &crate::fs::subr::vfs::<G>().ufs;
    core::mem::forget(t.ihash.replace(None));
    core::mem::forget(t.dq.replace(None));
    #[cfg(feature = "ufs_dirhash")]
    super::ufs_dirhash::dirhash_test_reset();
    t.init_done
        .store(false, core::sync::atomic::Ordering::SeqCst);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use hal::intr::IPL_NONE;
    use hal::pool::{BoxPool, PR_WAITOK, PoolArc, TypedPool};

    use super::*;
    use crate::fs::testfs::{B, TestGlue, any_locked, setup_root, vget_node};
    use crate::fs::ufs::ufs::dinode::Ufs1Dinode;
    use crate::fs::ufs::ufs::inode::{Dinode, Inode, InodeInit, Ufs1Din};
    use crate::fs::ufs::ufs::ufsmount::UM_UFS1;

    /// The tests' `ffs_ino_pool` and `ffs_dinode1_pool` (the heap doubles, made once).
    static INODES: TypedPool<Inode<TestGlue>> = TypedPool::new();
    static DINO1: BoxPool<Ufs1Din> = BoxPool::new();

    /// The handle `ffs_vptofh` writes for inode `B` of generation `generation`.
    fn fid(generation: u32) -> Ufid {
        Ufid {
            ufid_len: size_of::<Ufid>() as u16,
            ufid_pad: 0,
            ufid_ino: B as u32,
            ufid_gen: generation,
        }
    }

    #[test]
    fn a_handle_of_a_freed_or_reused_inode_is_stale() {
        let (_g, mp) = setup_root();
        let _ = INODES.init_recorded(IPL_NONE, PR_WAITOK, "ffsino");
        let _ = DINO1.init(IPL_NONE, PR_WAITOK, "dino1pl");
        // Inode B of generation 7, in core: VFS_VGET finds its vnode again.
        let vp = vget_node(mp, B).unwrap();
        let ip = INODES
            .get_homed(PR_WAITOK, || {
                Inode::new(InodeInit {
                    vnode: Some(vp),
                    mount: Some(mp),
                    number: B as u32,
                    fstype: UM_UFS1,
                    ..InodeInit::default()
                })
            })
            .unwrap();
        let d = Ufs1Dinode {
            di_mode: 0o100644,
            di_nlink: 1,
            di_gen: 7,
            ..Ufs1Dinode::default()
        };
        ip.set_din(Dinode::Ufs1(
            DINO1.get(PR_WAITOK, || Ufs1Din::new(&d)).unwrap(),
        ));
        vp.set_node(PoolArc::erase(ip.clone()));
        vput(vp);
        assert_eq!(vp.usecount(), 0);

        // The file's own handle: its vnode, referenced and locked.
        let got = ufs_fhtovp(mp, &fid(7)).unwrap();
        assert!(core::ptr::eq(got, vp));
        assert_eq!(vp.usecount(), 1);
        vput(got);

        // The number reused by another generation: ESTALE, the vnode given back.
        assert!(matches!(ufs_fhtovp(mp, &fid(8)), Err(Errno::ESTALE)));
        assert_eq!(vp.usecount(), 0);
        assert!(!any_locked());

        // The inode freed (mode 0): ESTALE whatever the generation.
        ip.dip_set_mode(0);
        assert!(matches!(ufs_fhtovp(mp, &fid(7)), Err(Errno::ESTALE)));
        assert_eq!(vp.usecount(), 0);
        assert!(!any_locked());
        drop(vp.take_node());
    }
}
/* </TESTS> */
