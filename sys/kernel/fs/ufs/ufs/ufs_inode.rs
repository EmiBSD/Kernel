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
 * Copyright (c) 1991, 1993
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
 *	@(#)ufs_inode.c	8.7 (Berkeley) 7/22/94
 */
/* </LICENSES> */

/* <CODE> */
//! The end of an inode's life: `ufs_inactive` (the last reference went: free a removed file's
//! blocks and inode, write the times back) and `ufs_reclaim` (the vnode is being reused:
//! unhash the inode and drop what hangs from it).
//!
//! Upstream: sys/ufs/ufs/ufs_inode.c @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufs_inode.rs@3c62ede68802
//!
//! ## Deviations
//! - `UFS_DIRHASH`'s `ufsdirhash_free` in `ufs_reclaim` is under feature `ufs_dirhash`.
//! - The quota calls are `quota.rs`'s: `ufs_quota.rs`'s with feature `quota` (`option QUOTA`),
//!   the no-quota answers of `ufs_quota_stub.c` without it.
//! - `ufs_reclaim`'s `if (ip->i_devvp)` is the inode's `i_devvp`, which every inode of a
//!   mounted UFS has (`panic(9)` otherwise, LZ's).
//!
//! ## Redesign
//! - LZ's `ufs_inactive(&mut VopInactiveArgs)` over `bsd`'s `vtoi` and the `UFS_*` vtable
//!   macros -> the kernel's `Vnops::inactive` shape (`vp`, `p`) over an [`InodeRef`] and
//!   [`InodeOps`]' methods: the file system's truncate, update and inode free are reached
//!   through the inode's own table, as the C's `i_vtbl` (`bridges.toml`'s `InodeOps` row
//!   while ffs and ext2fs are `bsd`'s). `ufs_reclaim` takes no `p` (the C's has none).
//!
//! [`InodeRef`]: super::inode::InodeRef
//! [`InodeOps`]: super::inode::InodeOps

use hal::sys::errno::Errno;
use hal::sys::mount::MNT_RDONLY;

use super::inode::{IN_ACCESS, IN_CHANGE, IN_LAZYMOD, IN_MODIFIED, IN_UPDATE, vtoi};
use super::quota::{getinoquota, ufs_quota_delete, ufs_quota_free_inode};
use super::ufs_ihash::ufs_ihashrem;
use crate::fs::cache::cache_purge;
use crate::fs::subr::{vrecycle, vrele};
use crate::fs::vnode::Vnode;
use crate::fs::vops::{Cred, vop_unlock};
use crate::proc::{ProcGlue, Thread};

/// `prtactive`'s `vprint` of an active vnode (`DIAGNOSTIC`).
#[cfg(feature = "diagnostic")]
fn pushing_active<G: ProcGlue>(vp: &'static Vnode<G>, what: &str) {
    if crate::fs::subr::vfs::<G>()
        .prtactive
        .load(core::sync::atomic::Ordering::Relaxed)
        != 0
        && vp.usecount() != 0
    {
        crate::fs::subr::vprint(Some(what), vp);
    }
}

/// `ufs_inactive` (`vop_inactive`): last reference to an inode. If necessary, write or
/// delete it.
pub fn ufs_inactive<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let mut error = Ok(());

    #[cfg(feature = "diagnostic")]
    pushing_active(vp, "ufs_inactive: pushing active");

    // Ignore inodes related to stale file handles.
    if !(ip.din_is_null() || ip.dip_mode() == 0) {
        let rdonly = vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0);
        if ip.dip_nlink() <= 0 && !rdonly {
            if getinoquota(&ip).is_ok() {
                let _ = ufs_quota_free_inode(&ip, Cred::NOCRED);
            }

            error = ip.vtbl().truncate(&ip, 0, 0, Cred::NOCRED);

            ip.dip_set_rdev(0);
            let mode = ip.dip_mode();
            ip.dip_set_mode(0);
            ip.set_flag(IN_CHANGE | IN_UPDATE);

            let _ = ip.vtbl().inode_free(&ip, ip.i_number, mode);
        }

        if ip.i_flag.get() & (IN_ACCESS | IN_CHANGE | IN_MODIFIED | IN_UPDATE) != 0 {
            let _ = ip.vtbl().update(&ip, 0);
        }
    }
    // out:
    let _ = vop_unlock(vp);

    // If we are done with the inode, reclaim it so that it can be reused immediately.
    if ip.din_is_null() || ip.dip_mode() == 0 {
        drop(ip);
        vrecycle(vp, p);
    }

    error
}

/// `ufs_reclaim`: reclaim an inode so that it can be used for other purposes (the UFS half
/// of a file system's `vop_reclaim`).
pub fn ufs_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    pushing_active(vp, "ufs_reclaim: pushing active");

    let ip = vtoi(vp);

    // Stop deferring timestamp writes.
    if ip.i_flag.get() & IN_LAZYMOD != 0 {
        ip.set_flag(IN_MODIFIED);
        let _ = ip.vtbl().update(&ip, 0);
    }

    // Remove the inode from its hash chain.
    ufs_ihashrem(&ip);
    // Purge old data structures associated with the inode.
    cache_purge(vp);

    vrele(ip.i_devvp());
    #[cfg(feature = "ufs_dirhash")]
    if ip.i_dirhash.read().is_some() {
        super::ufs_dirhash::ufsdirhash_free(&ip);
    }
    let _ = ufs_quota_delete(&ip);
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::vec::Vec;

    use hal::intr::IPL_NONE;
    use hal::pool::{BoxPool, PR_WAITOK, PoolArc, TypedPool};
    use hal::sys::types::{Mode, Off};

    use super::*;
    use crate::fs::subr::vref;
    use crate::fs::testfs::{TestGlue, setup};
    use crate::fs::ufs::ufs::dinode::{Ufs1Dinode, Ufsino};
    use crate::fs::ufs::ufs::inode::{
        Dinode, IN_HASHED, Inode, InodeInit, InodeOps, InodeRef, Ufs1Din,
    };
    use crate::fs::ufs::ufs::ufs_ihash::{ufs_ihashget, ufs_ihashinit, ufs_ihashins};
    use crate::fs::ufs::ufs::ufsmount::{UM_UFS1, Ufsmount};
    use crate::fs::vnode::VREG;
    use crate::fs::vops::IoBuf;
    use crate::testutil::test_vnode;

    /// The tests' `ffs_ino_pool` and `ffs_dinode1_pool`.
    static INODES: TypedPool<Inode<TestGlue>> = TypedPool::new();
    static DINO1: BoxPool<Ufs1Din> = BoxPool::new();
    /// The calls the test table saw, in order, with the mode `inode_free` was given.
    static CALLS: Mutex<Vec<(&'static str, Mode)>> = Mutex::new(Vec::new());

    /// An `InodeOps` that records its calls (the file system's half of the inode's life).
    struct Ops;

    fn note(what: &'static str, mode: Mode) {
        CALLS.lock().unwrap().push((what, mode));
    }

    impl InodeOps<TestGlue> for Ops {
        fn truncate(
            &self,
            _: &Inode<TestGlue>,
            off: Off,
            _: i32,
            cred: Cred<'_>,
        ) -> Result<(), Errno> {
            assert_eq!(off, 0);
            assert!(cred.is_nocred());
            note("truncate", 0);
            Ok(())
        }
        fn update(&self, _: &Inode<TestGlue>, waitfor: i32) -> Result<(), Errno> {
            assert_eq!(waitfor, 0);
            note("update", 0);
            Ok(())
        }
        fn inode_alloc(
            &self,
            _: &Inode<TestGlue>,
            _: Mode,
            _: Cred<'_>,
        ) -> Result<&'static Vnode<TestGlue>, Errno> {
            Err(Errno::ENOSPC)
        }
        fn inode_free(&self, _: &Inode<TestGlue>, _: Ufsino, mode: Mode) -> Result<(), Errno> {
            note("inode_free", mode);
            Ok(())
        }
        fn buf_alloc(
            &self,
            _: &Inode<TestGlue>,
            _: Off,
            _: i32,
            _: Cred<'_>,
            _: i32,
        ) -> Result<IoBuf, Errno> {
            Err(Errno::EIO)
        }
        fn bufatoff(&self, _: &Inode<TestGlue>, _: Off) -> Result<(IoBuf, usize), Errno> {
            Err(Errno::EIO)
        }
    }

    static OPS: Ops = Ops;

    /// The pools, the inode hash and an empty call log, over testfs's fresh tables.
    fn life_setup() -> std::sync::MutexGuard<'static, ()> {
        let g = setup();
        let _ = INODES.init_recorded(IPL_NONE, PR_WAITOK, "ffsino");
        let _ = DINO1.init(IPL_NONE, PR_WAITOK, "dino1pl");
        ufs_ihashinit::<TestGlue>(64);
        CALLS.lock().unwrap().clear();
        g
    }

    /// A referenced vnode holding a new FFS1 inode `ino` of `nlink` links and `mode`, on a
    /// UFS mount structure without quotas, with a held device vnode, as `ffs_vget` makes them.
    fn inode(ino: Ufsino, nlink: i16, mode: u16) -> (&'static Vnode<TestGlue>, InodeRef<TestGlue>) {
        let vp = test_vnode::<TestGlue>(VREG, 0);
        let devvp = test_vnode::<TestGlue>(crate::fs::vnode::VBLK, 0);
        let ump: &'static Ufsmount<TestGlue> =
            std::boxed::Box::leak(std::boxed::Box::new(Ufsmount::new()));
        let ip = INODES
            .get_homed(PR_WAITOK, || {
                Inode::new(InodeInit {
                    vnode: Some(vp),
                    ump: Some(ump),
                    devvp: Some(devvp),
                    dev: 9,
                    number: ino,
                    fstype: UM_UFS1,
                    vtbl: Some(&OPS),
                    ..InodeInit::default()
                })
            })
            .unwrap();
        let d = Ufs1Dinode {
            di_mode: mode,
            di_nlink: nlink,
            ..Ufs1Dinode::default()
        };
        ip.set_din(Dinode::Ufs1(
            DINO1.get(PR_WAITOK, || Ufs1Din::new(&d)).unwrap(),
        ));
        ip.dip_set_rdev(5);
        vp.set_node(PoolArc::erase(ip.clone()));
        (vp, ip)
    }

    fn calls() -> Vec<(&'static str, Mode)> {
        CALLS.lock().unwrap().clone()
    }

    #[test]
    fn an_unlinked_inode_is_truncated_freed_and_written_back() {
        let _g = life_setup();
        let (vp, ip) = inode(3, 0, 0o100644);
        ufs_inactive(vp, None).unwrap();
        // ufs_inode.c:79-95: truncate to 0, the mode and rdev cleared, the inode freed with
        // the mode it had, then the change written (IN_CHANGE | IN_UPDATE).
        assert_eq!(
            calls(),
            [("truncate", 0), ("inode_free", 0o100644), ("update", 0)]
        );
        assert_eq!((ip.dip_mode(), ip.dip_rdev()), (0, 0));
        assert_ne!(ip.i_flag.get() & (IN_CHANGE | IN_UPDATE), 0);
        // Still referenced: vrecycle leaves it (the C's v_usecount test).
        assert_eq!(vp.usecount(), 1);
    }

    #[test]
    fn a_linked_inode_is_written_back_only_when_a_time_changed() {
        let _g = life_setup();
        let (vp, ip) = inode(4, 1, 0o100644);
        ufs_inactive(vp, None).unwrap();
        assert!(calls().is_empty());
        ip.set_flag(IN_ACCESS);
        ufs_inactive(vp, None).unwrap();
        assert_eq!(calls(), [("update", 0)]);
        assert_eq!(ip.dip_mode(), 0o100644);
        // A stale file handle's inode (mode 0) is left alone.
        let (vp2, _ip2) = inode(5, 0, 0);
        CALLS.lock().unwrap().clear();
        ufs_inactive(vp2, None).unwrap();
        assert!(calls().is_empty());
    }

    #[test]
    fn reclaim_writes_lazy_times_unhashes_and_drops_the_device() {
        let _g = life_setup();
        let (vp, ip) = inode(6, 1, 0o100644);
        ufs_ihashins(ip.as_pin()).unwrap();
        let found = ufs_ihashget::<TestGlue>(9, 6).unwrap();
        assert!(core::ptr::eq(found, vp));
        crate::fs::subr::vput(found);
        let devvp = ip.i_devvp();
        vref(devvp);
        let held = devvp.usecount();
        ip.set_flag(IN_LAZYMOD);
        ufs_reclaim(vp).unwrap();
        // ufs_inode.c:129-151: the deferred times written, the inode off the hash, the
        // device's reference given back, no dquot left.
        assert_eq!(calls(), [("update", 0)]);
        assert_ne!(ip.i_flag.get() & IN_MODIFIED, 0);
        assert_eq!(ip.i_flag.get() & IN_HASHED, 0);
        assert!(ufs_ihashget::<TestGlue>(9, 6).is_none());
        assert_eq!(devvp.usecount(), held - 1);
        assert!(ip.i_dquot.iter().all(hal::link::Link::is_none));
        assert!(ip.i_dirhash.read().is_none());
    }
}
/* </TESTS> */
