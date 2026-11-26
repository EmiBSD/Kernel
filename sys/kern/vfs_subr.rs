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
 *	@(#)vfs_subr.c	8.13 (Berkeley) 4/18/94
 */
/* </LICENSES> */

/* <CODE> */
//! External virtual filesystem routines that stay `bsd`'s: the shutdown's `vfs_unmountall`,
//! `vfs_shutdown` and `vfs_syncwait` (they call `sys_sync` and `dounmount`, the kernel's since
//! N5.4 through `vfs_syscalls.rs`, and `sr_quiesce`, `bsd`'s until N7). The vnode table, the
//! mounts, the special-device aliases and the other helpers of `vfs_subr.c` are the kernel
//! crate's (`kernel::fs::subr`, `kernel::fs::mount`), which `sys/vnode.rs` names for `bsd`;
//! the NFS server's export lists (`vfs_hang_addrlist` .. `vfs_export_lookup`) are
//! `kernel::fs::export`'s since N6.7b0. `vntblinit`'s `rn_init` has no counterpart since
//! N6.7b1, when `bsd`'s radix tree went (each kernel tree has its key length).
//!
//! Upstream: sys/kern/vfs_subr.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `vfs_syncwait` pushes the idle delayed writes (the cache's dirty queue,
//!   `kernel::fs::bio::take_dirty_buf`) and counts the busy writes before it
//!   (`bufhead_busy`), where the C counts and pushes in one walk of `bufhead` (a busy delayed
//!   write, which the C would `bremfree`, is left to its owner).

use kernel::fs::bio;

use crate::kern::subr_prf::panicstr;
use crate::kern::vfs_syscalls::{dounmount, sys_sync};
use crate::kprintf;
use crate::machine::cpu::{curproc, delay};
use crate::sys::mount::{VB_NOWAIT, VB_WRITE, mountlist_safe};
use crate::sys::proc::Proc;
use crate::sys::vnode::{vfs_busy, vfs_unbusy};

/// Unmount all file systems. We traverse the list in reverse order under the assumption that
/// doing so will avoid needing to worry about dependencies.
pub fn vfs_unmountall() {
    let mut again = true;

    loop {
        let mut allerror = false;
        for mp in mountlist_safe(true) {
            if vfs_busy(mp, VB_WRITE | VB_NOWAIT).is_err() {
                continue;
            }
            // XXX Here is a race, the next pointer is not locked.
            let Some(p) = curproc() else {
                vfs_unbusy(mp);
                continue;
            };
            if let Err(error) = dounmount(mp, crate::sys::mount::MNT_FORCE, p) {
                let (name, len) = mp.mntonname();
                kprintf!(
                    "unmount of {} failed with error {}\n",
                    crate::kern::subr_prf::Str(&name[..len]),
                    error as i32
                );
                allerror = true;
            }
        }

        if allerror {
            kprintf!("WARNING: some file systems would not unmount\n");
            if again {
                kprintf!("retrying\n");
                again = false;
                continue;
            }
        }
        break;
    }
}

/// Sync and unmount file systems before shutting down.
pub fn vfs_shutdown(p: &Proc) {
    kernel::proc::acct::acct_shutdown();

    kprintf!("syncing disks...");

    if !panicstr() {
        // Sync before unmount, in case we hang on something.
        let mut retval = [0; 2];
        let _ = sys_sync(p, &[0; 6], &mut retval);
        vfs_unmountall();
    }

    // NSOFTRAID > 0
    crate::dev::softraid::sr_quiesce();

    if vfs_syncwait(p, true) != 0 {
        kprintf!(" giving up\n");
    } else {
        kprintf!(" done\n");
    }
}

/// Perform sync() operation and wait for buffers to flush; returns the buffers still busy.
pub fn vfs_syncwait(p: &Proc, verbose: bool) -> i32 {
    let mut retval = [0; 2];
    let _ = sys_sync(p, &[0; 6], &mut retval);

    // Wait for sync to finish.
    let mut dcount = 10000;
    let mut nbusy = 0;
    for iter in 0..20u32 {
        nbusy = bio::bufhead_busy();
        // With soft updates, some buffers that are written will be remarked as dirty until
        // other buffers are written.
        //
        // XXX here be dragons. this should really go away but should be carefully made to go
        // away on it's own with testing.. XXX
        while let Some(t) = bio::take_dirty_buf() {
            nbusy += 1;
            bio::bawrite(t);
            dcount -= 1;
            if dcount <= 0 {
                if verbose {
                    kprintf!("softdep ");
                }
                return 1;
            }
        }
        if nbusy == 0 {
            break;
        }
        if verbose {
            kprintf!("{} ", nbusy);
        }
        #[cfg(feature = "multiprocessor")]
        let hold_count = if crate::kern::kern_lock::_kernel_lock_held() {
            crate::kern::kern_lock::__mp_release_all(&crate::kern::kern_lock::KERNEL_LOCK)
        } else {
            0
        };
        delay(40000 * iter);
        #[cfg(feature = "multiprocessor")]
        if hold_count != 0 {
            crate::kern::kern_lock::__mp_acquire_count(
                &crate::kern::kern_lock::KERNEL_LOCK,
                hold_count,
            );
        }
    }

    nbusy
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests of the kernel's vnode table with bsd's glue (`getnewvnode`, the use counts
    // and free lists of `vref`/`vrele`/`vput`/`vget`, `vgone`, `insmntque`, `vflush`, through
    // `sys/vnode.rs`'s names), `vattr_null`, `vaccess`, the mount helpers and the export list;
    // and `testfs`, a small in-memory file system (a fixed tree of directories, a file and
    // symbolic links, with the lock discipline of a real one) that the `namei`, name cache and
    // `getcwd` tests share.

    use core::ptr;
    use core::sync::atomic::Ordering;
    use std::sync::MutexGuard;
    use std::{assert, assert_eq};

    use super::*;
    use crate::kern::kern_descrip::{fdinit, filedesc_init};
    use crate::kern::kern_prot::crget;
    use crate::kern::proc_glue::BsdProcGlue;
    use crate::kern::proc_glue::{procinit, test_proc};
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::kern::vfs_init::{set_rootvnode, vfsinit};
    use crate::kern::vfs_vops::{VOP_ISLOCKED, VOP_UNLOCK};
    use crate::sys::errno::Errno;
    use crate::sys::mount::{MNT_UNMOUNT, Mount, VB_READ, mountlist_insert_tail};
    use crate::sys::types::{Mode, Uid};
    use crate::sys::ucred::Ucred;
    use crate::sys::vnode::{
        FORCECLOSE, MAXVNODES, NUMVNODES, VBAD, VBIOONFREELIST, VDIR, VEXEC, VNON, VREAD, VREG,
        VT_NON, VWRITE, Vattr, Vnode, VnodeKind, checkalias, getnewvnode, insmntque, vaccess,
        vattr_null, vdrop, vflush, vfs_getnewfsid, vfs_getvfs, vfs_isbusy, vfs_mount_alloc, vgone,
        vhold, vput, vref, vrele,
    };

    /// `testfs`: the file system the vfs tests mount as root.
    pub(crate) mod testfs {
        use core::ffi::c_void;
        use core::ptr;
        use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};
        use std::sync::MutexGuard;

        use super::setup;
        use crate::kern::kern_subr::uiomove;
        use crate::kern::subr_xxx::eopnotsupp;
        use crate::kern::vfs_init::set_rootvnode;
        use crate::kern::vfs_vops::{VOP_UNLOCK, vop_generic_abortop};
        use crate::sys::dirent::{DT_DIR, DT_LNK, DT_REG, Dirent, dirent_recsize};
        use crate::sys::errno::Errno;
        use crate::sys::lock::{LK_EXCLUSIVE, LK_NOWAIT};
        use crate::sys::mount::mountlist_insert_tail;
        use crate::sys::mount::{MNT_LOCAL, MNT_ROOTFS, Mount, VFS_ROOT, Vfsconf, Vfsops};
        use crate::sys::namei::{
            CREATE, ISDOTDOT, ISLASTCN, LOCKPARENT, MAKEENTRY, PDIRUNLOCK, RENAME,
        };
        use crate::sys::namei::{cache_enter, cache_lookup};
        use crate::sys::proc::Proc;
        use crate::sys::vnode::vn_lock;
        use crate::sys::vnode::{
            VDIR, VLNK, VREG, VROOT, VT_TMPFS, Vnode, VopAccessArgs, VopGetattrArgs,
            VopInactiveArgs, VopIslockedArgs, VopLockArgs, VopLookupArgs, VopReaddirArgs,
            VopReadlinkArgs, VopReclaimArgs, VopUnlockArgs, Vops,
        };
        use crate::sys::vnode::{getnewvnode, vfs_mount_alloc, vfs_unbusy, vget, vref};

        /// What a node is.
        pub(crate) enum Kind {
            /// A directory.
            Dir,
            /// A regular file.
            Reg,
            /// A symbolic link to the path.
            Lnk(&'static [u8]),
        }

        /// One node of the tree: its name, its parent's index and its kind.
        pub(crate) struct Node {
            pub(crate) name: &'static [u8],
            pub(crate) parent: usize,
            pub(crate) kind: Kind,
        }

        /// `/`.
        pub(crate) const ROOT: usize = 0;
        /// `/a`.
        pub(crate) const A: usize = 1;
        /// `/a/b`, a file.
        pub(crate) const B: usize = 2;
        /// `/a/c`.
        pub(crate) const C: usize = 3;
        /// `/l -> a/c`.
        pub(crate) const L: usize = 4;
        /// `/a/abs -> /a/b`.
        pub(crate) const ABS: usize = 5;
        /// `/loop -> loop`.
        pub(crate) const LOOP: usize = 6;
        /// `/a/xxx...x` (40 bytes, longer than `NAMECACHE_MAXLEN`).
        pub(crate) const LONG: usize = 7;
        /// `/usr/share/zoneinfo`.
        pub(crate) const ZONEINFO: usize = 10;
        /// `/usr/share/zoneinfo/UTC`, a file.
        pub(crate) const UTC: usize = 11;
        /// `/usr/share/zoneinfo/link -> UTC`.
        pub(crate) const ZLINK: usize = 12;
        /// `/usr/share/zoneinfo/Etc`, a directory.
        pub(crate) const ETCZ: usize = 13;
        /// `/etc/localtime -> /usr/share/zoneinfo/UTC`.
        pub(crate) const LOCALTIME: usize = 15;
        /// `/etc/lt-link -> /usr/share/zoneinfo/link`.
        pub(crate) const LT_LINK: usize = 16;
        /// `/etc/lt-dir -> /usr/share/zoneinfo/Etc`.
        pub(crate) const LT_DIR: usize = 17;
        /// `/etc/lt-up -> /usr/share/zoneinfo/../zoneinfo/UTC`.
        pub(crate) const LT_UP: usize = 18;
        /// The number of nodes.
        pub(crate) const N: usize = 19;

        /// The tree.
        pub(crate) static NODES: [Node; N] = [
            Node {
                name: b"/",
                parent: ROOT,
                kind: Kind::Dir,
            },
            Node {
                name: b"a",
                parent: ROOT,
                kind: Kind::Dir,
            },
            Node {
                name: b"b",
                parent: A,
                kind: Kind::Reg,
            },
            Node {
                name: b"c",
                parent: A,
                kind: Kind::Dir,
            },
            Node {
                name: b"l",
                parent: ROOT,
                kind: Kind::Lnk(b"a/c"),
            },
            Node {
                name: b"abs",
                parent: A,
                kind: Kind::Lnk(b"/a/b"),
            },
            Node {
                name: b"loop",
                parent: ROOT,
                kind: Kind::Lnk(b"loop"),
            },
            Node {
                name: b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
                parent: A,
                kind: Kind::Reg,
            },
            // 8: /usr, 9: /usr/share, then zoneinfo's tree and /etc's links.
            Node {
                name: b"usr",
                parent: ROOT,
                kind: Kind::Dir,
            },
            Node {
                name: b"share",
                parent: 8,
                kind: Kind::Dir,
            },
            Node {
                name: b"zoneinfo",
                parent: 9,
                kind: Kind::Dir,
            },
            Node {
                name: b"UTC",
                parent: ZONEINFO,
                kind: Kind::Reg,
            },
            Node {
                name: b"link",
                parent: ZONEINFO,
                kind: Kind::Lnk(b"UTC"),
            },
            Node {
                name: b"Etc",
                parent: ZONEINFO,
                kind: Kind::Dir,
            },
            // 14: /etc.
            Node {
                name: b"etc",
                parent: ROOT,
                kind: Kind::Dir,
            },
            Node {
                name: b"localtime",
                parent: 14,
                kind: Kind::Lnk(b"/usr/share/zoneinfo/UTC"),
            },
            Node {
                name: b"lt-link",
                parent: 14,
                kind: Kind::Lnk(b"/usr/share/zoneinfo/link"),
            },
            Node {
                name: b"lt-dir",
                parent: 14,
                kind: Kind::Lnk(b"/usr/share/zoneinfo/Etc"),
            },
            Node {
                name: b"lt-up",
                parent: 14,
                kind: Kind::Lnk(b"/usr/share/zoneinfo/../zoneinfo/UTC"),
            },
        ];

        /// Each node's vnode, while it has one (the "inode hash").
        static VTAB: [AtomicPtr<Vnode>; N] = [const { AtomicPtr::new(ptr::null_mut()) }; N];
        /// Each node's lock: 1 while held.
        static LOCKED: [AtomicU32; N] = [const { AtomicU32::new(0) }; N];
        /// How many times `VOP_INACTIVE` ran.
        pub(crate) static INACTIVE: AtomicUsize = AtomicUsize::new(0);
        /// How many times `VOP_RECLAIM` ran.
        pub(crate) static RECLAIM: AtomicUsize = AtomicUsize::new(0);
        /// The mount.
        static MP: AtomicPtr<Mount> = AtomicPtr::new(ptr::null_mut());

        /// Forgets every vnode and lock (the pools behind them are new after `setup`).
        pub(crate) fn reset() {
            for i in 0..N {
                VTAB[i].store(ptr::null_mut(), Ordering::SeqCst);
                LOCKED[i].store(0, Ordering::SeqCst);
            }
            INACTIVE.store(0, Ordering::SeqCst);
            RECLAIM.store(0, Ordering::SeqCst);
            MP.store(ptr::null_mut(), Ordering::SeqCst);
        }

        /// The node a testfs vnode stands for.
        pub(crate) fn node_of(vp: &Vnode) -> usize {
            let idx = vp.data() as usize;
            assert!(idx > 0, "not a testfs vnode");
            idx - 1
        }

        /// The node's vnode, if it has one.
        pub(crate) fn vnode_of(idx: usize) -> Option<&'static Vnode> {
            // SAFETY: only `vget_node` stores here, and only vnodes, which are never freed.
            unsafe { VTAB[idx].load(Ordering::SeqCst).as_ref() }
        }

        /// The use count of every node's vnode (0 for none).
        pub(crate) fn usecounts() -> [u32; N] {
            core::array::from_fn(|i| vnode_of(i).map_or(0, |vp| vp.usecount()))
        }

        /// Whether any node's lock is held.
        pub(crate) fn any_locked() -> bool {
            LOCKED.iter().any(|l| l.load(Ordering::SeqCst) != 0)
        }

        /// The vnode of node `idx`, referenced and locked (`VFS_VGET`).
        pub(crate) fn vget_node(mp: &'static Mount, idx: usize) -> Result<&'static Vnode, Errno> {
            if let Some(vp) = vnode_of(idx) {
                vget(vp, LK_EXCLUSIVE)?;
                return Ok(vp);
            }
            let vp = getnewvnode(VT_TMPFS, Some(mp), &TESTFS_VOPS)?;
            vp.set_vtype(match NODES[idx].kind {
                Kind::Dir => VDIR,
                Kind::Reg => VREG,
                Kind::Lnk(_) => VLNK,
            });
            vp.set_data(ptr::without_provenance_mut::<c_void>(idx + 1));
            if idx == ROOT {
                vp.set_flag(VROOT);
            }
            vn_lock(vp, LK_EXCLUSIVE)?;
            VTAB[idx].store(ptr::from_ref(vp).cast_mut(), Ordering::SeqCst);
            Ok(vp)
        }

        fn the_mount() -> &'static Mount {
            // SAFETY: `mount_root` stores the mount, which stays allocated while mounted.
            unsafe { &*MP.load(Ordering::SeqCst) }
        }

        fn testfs_mount(
            _mp: &'static Mount,
            _path: &[u8],
            _data: &mut [u8],
            _ndp: &mut crate::sys::namei::Nameidata<'_>,
            _p: &Proc,
        ) -> Result<(), Errno> {
            Ok(())
        }

        fn testfs_root(mp: &'static Mount) -> Result<&'static Vnode, Errno> {
            vget_node(mp, ROOT)
        }

        fn testfs_vget(mp: &'static Mount, ino: u64) -> Result<&'static Vnode, Errno> {
            vget_node(mp, ino as usize)
        }

        /// `vfsops` of testfs: only the root and the inode lookup do anything.
        pub(crate) static TESTFS_VFSOPS: Vfsops = Vfsops {
            vfs_mount: testfs_mount,
            vfs_start: |_, _, _| Ok(()),
            vfs_unmount: |_, _, _| Ok(()),
            vfs_root: testfs_root,
            vfs_quotactl: |_, _, _, _, _| eopnotsupp(),
            vfs_statfs: |_, _, _| Ok(()),
            vfs_sync: |_, _, _, _, _| Ok(()),
            vfs_vget: testfs_vget,
            vfs_fhtovp: |_, _| Err(Errno::EOPNOTSUPP),
            vfs_vptofh: |_, _| eopnotsupp(),
            vfs_init: None,
            vfs_sysctl: None,
            vfs_checkexp: |_, _| Err(Errno::EOPNOTSUPP),
        };

        /// The configuration entry of testfs.
        pub(crate) static TESTFS_CONF: Vfsconf =
            Vfsconf::new(&TESTFS_VFSOPS, b"testfs", 99, MNT_LOCAL, 0);

        fn testfs_lock(ap: &mut VopLockArgs) -> Result<(), Errno> {
            let l = &LOCKED[node_of(ap.a_vp)];
            if l.load(Ordering::SeqCst) != 0 {
                if ap.a_flags & LK_NOWAIT != 0 {
                    return Err(Errno::EBUSY);
                }
                std::panic!("testfs: node {} locked twice", node_of(ap.a_vp));
            }
            l.store(1, Ordering::SeqCst);
            Ok(())
        }

        fn testfs_unlock(ap: &mut VopUnlockArgs) -> Result<(), Errno> {
            let l = &LOCKED[node_of(ap.a_vp)];
            assert_eq!(
                l.load(Ordering::SeqCst),
                1,
                "testfs: unlock of an unlocked node"
            );
            l.store(0, Ordering::SeqCst);
            Ok(())
        }

        fn testfs_islocked(ap: &mut VopIslockedArgs) -> i32 {
            LOCKED[node_of(ap.a_vp)].load(Ordering::SeqCst) as i32
        }

        fn testfs_inactive(ap: &mut VopInactiveArgs<'_>) -> Result<(), Errno> {
            INACTIVE.fetch_add(1, Ordering::SeqCst);
            VOP_UNLOCK(ap.a_vp)
        }

        fn testfs_reclaim(ap: &mut VopReclaimArgs<'_>) -> Result<(), Errno> {
            RECLAIM.fetch_add(1, Ordering::SeqCst);
            let idx = node_of(ap.a_vp);
            VTAB[idx].store(ptr::null_mut(), Ordering::SeqCst);
            LOCKED[idx].store(0, Ordering::SeqCst);
            ap.a_vp.set_data(ptr::null_mut());
            Ok(())
        }

        /// The lookup of `ufs_lookup`, over the fixed tree: the name cache first, then the
        /// directory; the parent stays locked only for `LOCKPARENT` on the last component.
        fn testfs_lookup(ap: &mut VopLookupArgs<'_, '_>) -> Result<(), Errno> {
            let dvp = ap.a_dvp;
            let cnp = &mut *ap.a_cnp;
            *ap.a_vpp = None;
            if dvp.vtype() != VDIR {
                return Err(Errno::ENOTDIR);
            }
            let lockparent = cnp.cn_flags & LOCKPARENT != 0;
            let islast = cnp.cn_flags & ISLASTCN != 0;

            match cache_lookup(dvp, cnp) {
                Ok(Some(vp)) => {
                    *ap.a_vpp = Some(vp);
                    return Ok(());
                }
                Ok(None) => {}
                Err(e) => return Err(e),
            }

            let dir = node_of(dvp);
            let name = cnp.name();
            let found = if name == b"." {
                Some(dir)
            } else if cnp.cn_flags & ISDOTDOT != 0 {
                Some(NODES[dir].parent)
            } else {
                (1..N).find(|&i| NODES[i].parent == dir && NODES[i].name == name)
            };
            let Some(idx) = found else {
                if (cnp.cn_nameiop == CREATE || cnp.cn_nameiop == RENAME) && islast {
                    if !lockparent {
                        let _ = VOP_UNLOCK(dvp);
                        cnp.cn_flags |= PDIRUNLOCK;
                    }
                    return Err(Errno::EJUSTRETURN);
                }
                if cnp.cn_flags & MAKEENTRY != 0 && cnp.cn_nameiop != CREATE {
                    cache_enter(dvp, None, cnp.name());
                }
                return Err(Errno::ENOENT);
            };

            let mp = the_mount();
            let vp = if idx == dir {
                vref(dvp);
                dvp
            } else if cnp.cn_flags & ISDOTDOT != 0 {
                let _ = VOP_UNLOCK(dvp);
                cnp.cn_flags |= PDIRUNLOCK;
                let vp = vget_node(mp, idx)?;
                if lockparent && islast {
                    vn_lock(dvp, LK_EXCLUSIVE)?;
                    cnp.cn_flags &= !PDIRUNLOCK;
                }
                vp
            } else {
                let vp = vget_node(mp, idx)?;
                if !lockparent || !islast {
                    let _ = VOP_UNLOCK(dvp);
                    cnp.cn_flags |= PDIRUNLOCK;
                }
                vp
            };
            if cnp.cn_flags & MAKEENTRY != 0 {
                cache_enter(dvp, Some(vp), cnp.name());
            }
            *ap.a_vpp = Some(vp);
            Ok(())
        }

        fn testfs_getattr(ap: &mut VopGetattrArgs<'_>) -> Result<(), Errno> {
            let idx = node_of(ap.a_vp);
            let va = &mut *ap.a_vap;
            *va = crate::sys::vnode::Vattr::new();
            va.va_type = ap.a_vp.vtype();
            va.va_mode = 0o755;
            va.va_nlink = 1;
            va.va_fsid = 99;
            va.va_fileid = 100 + idx as u64;
            va.va_size = match NODES[idx].kind {
                Kind::Lnk(target) => target.len() as u64,
                _ => 512,
            };
            va.va_blocksize = 512;
            va.va_bytes = 512;
            Ok(())
        }

        fn testfs_readlink(ap: &mut VopReadlinkArgs<'_, '_>) -> Result<(), Errno> {
            let Kind::Lnk(target) = NODES[node_of(ap.a_vp)].kind else {
                return Err(Errno::EINVAL);
            };
            let mut buf = target.to_vec();
            uiomove(&mut buf, ap.a_uio)
        }

        /// One `struct dirent` record for `name`.
        fn dirent(fileno: u64, type_: u8, name: &[u8]) -> std::vec::Vec<u8> {
            let reclen = dirent_recsize(name.len());
            let mut rec = std::vec![0u8; reclen];
            rec[..8].copy_from_slice(&fileno.to_ne_bytes());
            rec[16..18].copy_from_slice(&(reclen as u16).to_ne_bytes());
            rec[18] = type_;
            rec[19] = name.len() as u8;
            rec[Dirent::NAME_OFFSET..Dirent::NAME_OFFSET + name.len()].copy_from_slice(name);
            rec
        }

        /// The whole directory in one call: `.`, `..` and the children.
        fn testfs_readdir(ap: &mut VopReaddirArgs<'_, '_>) -> Result<(), Errno> {
            let dir = node_of(ap.a_vp);
            if ap.a_uio.uio_offset != 0 {
                *ap.a_eofflag = 1;
                return Ok(());
            }
            let mut buf = dirent(100 + dir as u64, DT_DIR, b".");
            buf.extend(dirent(100 + NODES[dir].parent as u64, DT_DIR, b".."));
            for i in (1..N).filter(|&i| NODES[i].parent == dir) {
                let type_ = match NODES[i].kind {
                    Kind::Dir => DT_DIR,
                    Kind::Reg => DT_REG,
                    Kind::Lnk(_) => DT_LNK,
                };
                buf.extend(dirent(100 + i as u64, type_, NODES[i].name));
            }
            *ap.a_eofflag = 1;
            uiomove(&mut buf, ap.a_uio)
        }

        fn testfs_access(_ap: &mut VopAccessArgs<'_>) -> Result<(), Errno> {
            Ok(())
        }

        /// `vops` of testfs.
        pub(crate) static TESTFS_VOPS: Vops = Vops {
            vop_lock: Some(testfs_lock),
            vop_unlock: Some(testfs_unlock),
            vop_islocked: Some(testfs_islocked),
            vop_abortop: Some(vop_generic_abortop),
            vop_access: Some(testfs_access),
            vop_close: Some(|_| Ok(())),
            vop_getattr: Some(testfs_getattr),
            vop_inactive: Some(testfs_inactive),
            vop_lookup: Some(testfs_lookup),
            vop_open: Some(|_| Ok(())),
            vop_readdir: Some(testfs_readdir),
            vop_readlink: Some(testfs_readlink),
            vop_reclaim: Some(testfs_reclaim),
            vop_print: Some(|_| Ok(())),
            ..Vops::EMPTY
        };

        /// Mounts testfs as the root file system the way `main` does after `mountroot`: on the
        /// mount list with `MNT_ROOTFS`, its root as `rootvnode` and `p`'s current directory.
        pub(crate) fn mount_root(p: &Proc) -> &'static Mount {
            let mp = vfs_mount_alloc(None, &TESTFS_CONF);
            vfs_unbusy(mp);
            MP.store(ptr::from_ref(mp).cast_mut(), Ordering::SeqCst);
            mountlist_insert_tail(mp);
            mp.set_flag(mp.flag() | MNT_ROOTFS);
            let Ok(root) = VFS_ROOT(mp) else {
                std::panic!("testfs: no root");
            };
            set_rootvnode(Some(root));
            p.fd().set_cdir(Some(root));
            vref(root);
            let _ = VOP_UNLOCK(root);
            mp
        }

        /// `setup` plus testfs mounted as root: the guard, the thread and the mount.
        pub(crate) fn setup_root() -> (MutexGuard<'static, ()>, &'static Proc, &'static Mount) {
            let (guard, p) = setup();
            let mp = mount_root(p);
            (guard, p, mp)
        }
    }

    /// A path in the user memory of the host double (`hal::arch::host::copy`), where the
    /// kernel's `copyinstr` reads a system call's path (`namei`'s `NiDirp::User`).
    pub(crate) struct UserStr(usize);

    impl UserStr {
        /// The user address, as a system call's argument.
        pub(crate) fn as_ptr(&self) -> *const u8 {
            core::ptr::without_provenance(self.0)
        }

        /// The user address.
        pub(crate) fn addr(&self) -> usize {
            self.0
        }
    }

    /// `s`, NUL-terminated (a NUL is added when it has none), copied into the user memory of
    /// the host double: eight rotating slots of 2 KiB at `USER_BASE + 0xc000` (the other
    /// tests use lower offsets; a test holds a few paths at once). A longer string is cut
    /// at 2047 bytes, without its NUL: the kernel's `copyinstr` stops at `MAXPATHLEN`.
    pub(crate) fn user_str(s: &[u8]) -> UserStr {
        static NEXT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
        const SLOT: usize = 2048;
        let i = NEXT.fetch_add(1, Ordering::Relaxed) % 8;
        let addr = kernel::testutil::USER_BASE + 0xc000 + i * SLOT;
        let mut bytes = s.to_vec();
        if bytes.last() != Some(&0) {
            bytes.push(0);
        }
        bytes.truncate(SLOT - 1);
        hal::user::UserPtr::<u8>::new(addr)
            .copyout_slice(&bytes)
            .expect("the double's user memory");
        UserStr(addr)
    }

    /// A system call's user buffer in the user memory of the host double (a `struct stat`, a
    /// `struct statfs`, a file system's mount arguments), where the kernel's copies
    /// (`hal::user`, N5.4's calls) read and write it: one buffer at a time, at most 12 KiB at
    /// `USER_BASE + 0x8000` (below `user_str`'s slots, above `kern_sysctl`'s and under
    /// `kern_prot`'s ranges).
    pub(crate) struct UserBuf {
        addr: usize,
        len: usize,
    }

    impl UserBuf {
        /// The user address, as a system call's argument.
        pub(crate) fn addr(&self) -> usize {
            self.addr
        }

        /// The buffer's bytes as the call left them.
        pub(crate) fn bytes(&self) -> std::vec::Vec<u8> {
            let mut b = std::vec![0u8; self.len];
            hal::user::UserPtr::<u8>::new(self.addr)
                .copyin_slice(&mut b)
                .expect("the double's user memory");
            b
        }

        /// The `struct stat` the call wrote (`Stat::to_bytes`'s layout read back).
        pub(crate) fn stat(&self) -> crate::sys::stat::Stat {
            use crate::sys::stat::Stat;
            use core::mem::offset_of;
            let b = self.bytes();
            let word = |off: usize, n: usize| -> [u8; 8] {
                let mut w = [0u8; 8];
                w[..n].copy_from_slice(&b[off..off + n]);
                w
            };
            let w4 = |off: usize| -> [u8; 4] {
                let w = word(off, 4);
                [w[0], w[1], w[2], w[3]]
            };
            let u32_at = |off: usize| u32::from_ne_bytes(w4(off));
            let i32_at = |off: usize| i32::from_ne_bytes(w4(off));
            let u64_at = |off: usize| u64::from_ne_bytes(word(off, 8));
            let i64_at = |off: usize| i64::from_ne_bytes(word(off, 8));
            let ts_at = |off: usize| hal::sys::stat::Timespec {
                tv_sec: i64_at(off),
                tv_nsec: i64_at(off + 8),
            };
            Stat {
                st_mode: u32_at(offset_of!(Stat, st_mode)) as _,
                st_dev: i32_at(offset_of!(Stat, st_dev)) as _,
                st_ino: u64_at(offset_of!(Stat, st_ino)) as _,
                st_nlink: u32_at(offset_of!(Stat, st_nlink)) as _,
                st_uid: u32_at(offset_of!(Stat, st_uid)) as _,
                st_gid: u32_at(offset_of!(Stat, st_gid)) as _,
                st_rdev: i32_at(offset_of!(Stat, st_rdev)) as _,
                st_atim: ts_at(offset_of!(Stat, st_atim)),
                st_mtim: ts_at(offset_of!(Stat, st_mtim)),
                st_ctim: ts_at(offset_of!(Stat, st_ctim)),
                st_size: i64_at(offset_of!(Stat, st_size)) as _,
                st_blocks: i64_at(offset_of!(Stat, st_blocks)) as _,
                st_blksize: i32_at(offset_of!(Stat, st_blksize)) as _,
                st_flags: u32_at(offset_of!(Stat, st_flags)),
                st_gen: u32_at(offset_of!(Stat, st_gen)),
                __st_birthtim: ts_at(offset_of!(Stat, __st_birthtim)),
            }
        }
    }

    /// `bytes` copied into the double's user memory as a system call's buffer ([`UserBuf`]).
    pub(crate) fn user_buf(bytes: &[u8]) -> UserBuf {
        const BASE: usize = kernel::testutil::USER_BASE + 0x8000;
        assert!(bytes.len() <= 0x3000, "a user buffer of at most 12 KiB");
        hal::user::UserPtr::<u8>::new(BASE)
            .copyout_slice(bytes)
            .expect("the double's user memory");
        UserBuf {
            addr: BASE,
            len: bytes.len(),
        }
    }

    /// Real memory, the process, file and vfs pools, empty vnode lists; returns the guard and a
    /// thread with credentials and a descriptor table (no current directory).
    pub(crate) fn setup() -> (MutexGuard<'static, ()>, &'static Proc) {
        let guard = setup_real_memory();
        crate::machine::cons::consinit();
        // The VOPs run under the kernel lock, as the system calls that reach them take it:
        // VN_KNOTE (knote_locked on v_klist) needs it held. The test thread keeps it.
        crate::sys::systm::kernel_lock();
        procinit();
        filedesc_init();
        // The kernel's vnode lists, mounts, device vnodes and syncer wheel of an earlier test
        // lived in the memory just reset: forgotten, not freed (testing.md).
        kernel::fs::subr::vfs_test_reset::<BsdProcGlue>();
        let _ = kernel::fs::spec::register_devsw::<BsdProcGlue>(&crate::sys::specdev::BSD_DEVSW);
        // The file systems' pools and tables live in that memory too: let vfsinit's vfs_init
        // calls set them up again (ffs's flag is the kernel's UFS tables', which
        // vfs_test_reset cleared).
        vfsinit();
        testfs::reset();
        set_rootvnode(None);

        let p = test_proc(1, crget());
        let pr = p.process();
        pr.set_fd(fdinit());
        (guard, p)
    }

    /// Whether `vp` is on the kernel's free list.
    fn on_free(vp: &'static Vnode) -> bool {
        crate::kern::kern_descrip::VFS_TABLES
            .lists
            .lock()
            .free
            .iter()
            .any(|v| ptr::eq(v, vp))
    }

    /// Whether `vp` is on the kernel's hold list.
    fn on_hold(vp: &'static Vnode) -> bool {
        crate::kern::kern_descrip::VFS_TABLES
            .lists
            .lock()
            .hold
            .iter()
            .any(|v| ptr::eq(v, vp))
    }

    #[test]
    fn vattr_null_sets_every_field_to_vnoval() {
        let mut va = Vattr::new();
        va.va_type = VREG;
        va.va_vaflags = 7;
        vattr_null(&mut va);
        assert_eq!(va.va_type, VNON);
        assert_eq!(va.va_mode, Mode::MAX);
        assert_eq!(va.va_uid, Uid::MAX);
        assert_eq!(va.va_size, u64::MAX);
        assert_eq!(va.va_fsid, -1);
        assert_eq!(va.va_atime.tv_nsec, -1);
        assert_eq!(va.va_ctime.tv_sec, -1);
        assert_eq!(va.va_rdev, -1);
        assert_eq!(va.va_flags, u64::MAX);
        assert_eq!(va.va_vaflags, 0);
    }

    #[test]
    fn vaccess_checks_owner_then_group_then_others() {
        let _g = setup();
        let mut cr = Ucred::new();
        cr.cr_uid.set(1000);
        cr.cr_gid.set(10);
        cr.cr_ngroups.set(1);
        cr.cr_groups[0].set(10);
        // The owner's bits decide for the owner, even when others may.
        assert_eq!(
            vaccess(VREG, 0o077, 1000, 20, VREAD, &cr),
            Err(Errno::EACCES)
        );
        assert_eq!(vaccess(VREG, 0o600, 1000, 20, VREAD | VWRITE, &cr), Ok(()));
        // Then the group.
        assert_eq!(vaccess(VREG, 0o040, 0, 10, VREAD, &cr), Ok(()));
        assert_eq!(vaccess(VREG, 0o040, 0, 10, VWRITE, &cr), Err(Errno::EACCES));
        // Then everyone else.
        assert_eq!(vaccess(VDIR, 0o001, 0, 0, VEXEC, &cr), Ok(()));
        // Root reads anything, but executes only something executable (directories search).
        cr.cr_uid.set(0);
        assert_eq!(vaccess(VREG, 0o000, 1, 1, VREAD | VWRITE, &cr), Ok(()));
        assert_eq!(vaccess(VREG, 0o644, 1, 1, VEXEC, &cr), Err(Errno::EACCES));
        assert_eq!(vaccess(VDIR, 0o000, 1, 1, VEXEC, &cr), Ok(()));
    }

    #[test]
    fn getnewvnode_puts_the_vnode_on_its_mount() {
        let (_g, _p, mp) = testfs::setup_root();
        let before = NUMVNODES.load(Ordering::SeqCst);
        let vp = getnewvnode(VT_NON, Some(mp), &testfs::TESTFS_VOPS).unwrap();
        assert_eq!(vp.usecount(), 1);
        assert_eq!(vp.vtype(), VNON);
        assert!(vp.mount().is_some_and(|m| ptr::eq(m, mp)));
        assert!(mp.vnodes().any(|v| ptr::eq(v, vp)));
        assert_eq!(NUMVNODES.load(Ordering::SeqCst), before + 1);

        insmntque(vp, None);
        assert!(vp.mount().is_none());
        assert!(!mp.vnodes().any(|v| ptr::eq(v, vp)));
    }

    #[test]
    fn use_counts_and_the_free_list() {
        let (_g, _p, mp) = testfs::setup_root();
        let vp = testfs::vget_node(mp, testfs::B).unwrap();
        assert_eq!(vp.usecount(), 1);
        assert_eq!(VOP_ISLOCKED(vp), 1);

        vref(vp);
        assert_eq!(vp.usecount(), 2);
        // vput of a vnode still referenced only unlocks it.
        vput(vp);
        assert_eq!(vp.usecount(), 1);
        assert_eq!(VOP_ISLOCKED(vp), 0);
        assert_eq!(testfs::INACTIVE.load(Ordering::SeqCst), 0);

        // The last vrele locks, deactivates (which unlocks) and frees the vnode, which keeps its
        // identity on the free list.
        assert!(vrele(vp));
        assert_eq!(vp.usecount(), 0);
        assert_eq!(testfs::INACTIVE.load(Ordering::SeqCst), 1);
        assert!(!testfs::any_locked());
        assert!(vp.bufs.bioflag() & VBIOONFREELIST != 0);
        assert!(on_free(vp));
        assert_eq!(testfs::node_of(vp), testfs::B);

        // vget takes it back off the free list, referenced and locked.
        let again = testfs::vget_node(mp, testfs::B).unwrap();
        assert!(ptr::eq(again, vp));
        assert_eq!(vp.usecount(), 1);
        assert!(vp.bufs.bioflag() & VBIOONFREELIST == 0);
        assert!(!on_free(vp));
        vput(vp);
        assert_eq!(vp.usecount(), 0);
        assert_eq!(testfs::INACTIVE.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn vhold_moves_a_free_vnode_to_the_hold_list() {
        let (_g, _p, mp) = testfs::setup_root();
        let vp = testfs::vget_node(mp, testfs::B).unwrap();
        vput(vp);
        assert!(on_free(vp));
        vhold(vp);
        assert!(on_hold(vp));
        assert!(!on_free(vp));
        vdrop(vp);
        assert!(on_free(vp));
        assert!(!on_hold(vp));
    }

    #[test]
    fn vgone_reclaims_and_leaves_a_dead_vnode() {
        let (_g, _p, mp) = testfs::setup_root();
        let vp = testfs::vget_node(mp, testfs::C).unwrap();
        let _ = VOP_UNLOCK(vp);
        let id = vp.id();

        // An active vnode: closed, deactivated, reclaimed; the reference survives.
        vgone(vp);
        assert_eq!(testfs::RECLAIM.load(Ordering::SeqCst), 1);
        assert_eq!(testfs::INACTIVE.load(Ordering::SeqCst), 1);
        assert_eq!(vp.vtype(), VBAD);
        assert_eq!(vp.kind(), VnodeKind::Dead);
        assert!(vp.mount().is_none());
        assert!(vp.data().is_null());
        assert_ne!(vp.id(), id, "cache_purge gives a new capability");
        assert!(testfs::vnode_of(testfs::C).is_none());
        assert_eq!(vp.usecount(), 1);

        // The dead vnode's last release puts it at the head of the free list, to be recycled
        // first.
        assert!(vrele(vp));
        assert!(
            crate::kern::kern_descrip::VFS_TABLES
                .lists
                .lock()
                .free
                .first()
                .is_some_and(|v| ptr::eq(v, vp))
        );
    }

    #[test]
    fn getnewvnode_recycles_from_the_free_list_past_maxvnodes() {
        let (_g, _p, mp) = testfs::setup_root();
        let vp = testfs::vget_node(mp, testfs::B).unwrap();
        vput(vp);
        MAXVNODES.store(1, Ordering::SeqCst);
        let before = NUMVNODES.load(Ordering::SeqCst);
        let nvp = getnewvnode(VT_NON, None, &testfs::TESTFS_VOPS).unwrap();
        assert!(ptr::eq(nvp, vp), "the free vnode is recycled");
        assert_eq!(NUMVNODES.load(Ordering::SeqCst), before);
        assert_eq!(testfs::RECLAIM.load(Ordering::SeqCst), 1);
        assert!(testfs::vnode_of(testfs::B).is_none());
        assert_eq!(nvp.usecount(), 1);
        assert!(nvp.mount().is_none());
    }

    #[test]
    fn vflush_counts_busy_vnodes_unless_forced() {
        let (_g, _p, mp) = testfs::setup_root();
        let busy = testfs::vget_node(mp, testfs::B).unwrap();
        let _ = VOP_UNLOCK(busy);
        let idle = testfs::vget_node(mp, testfs::C).unwrap();
        vput(idle);

        // The root (two references) and B are busy; C is idle and goes.
        let root = crate::kern::vfs_init::rootvnode();
        assert_eq!(vflush(mp, root, 0), Err(Errno::EBUSY));
        assert!(testfs::vnode_of(testfs::C).is_none());
        assert!(testfs::vnode_of(testfs::B).is_some());

        assert_eq!(vflush(mp, root, FORCECLOSE), Ok(()));
        assert!(testfs::vnode_of(testfs::B).is_none());
        assert_eq!(busy.vtype(), VBAD);
        // Only the skipped root is left on the mount.
        assert_eq!(mp.vnodes().count(), 1);
    }

    #[test]
    fn vfs_busy_fails_on_an_unmounting_file_system() {
        let (_g, _p, mp) = testfs::setup_root();
        assert_eq!(vfs_busy(mp, VB_READ | VB_NOWAIT), Ok(()));
        assert!(vfs_isbusy(mp));
        vfs_unbusy(mp);
        assert!(!vfs_isbusy(mp));
        mp.set_flag(mp.flag() | MNT_UNMOUNT);
        assert_eq!(vfs_busy(mp, VB_READ | VB_NOWAIT), Err(Errno::EBUSY));
        assert!(!vfs_isbusy(mp));
        mp.set_flag(mp.flag() & !MNT_UNMOUNT);
    }

    #[test]
    fn vfs_getnewfsid_is_unique_among_mounts() {
        let (_g, _p, mp) = testfs::setup_root();
        vfs_getnewfsid(mp);
        let fsid = mp.fsid();
        assert!(vfs_getvfs(&fsid).is_some_and(|m| ptr::eq(m, mp)));
        let mp2 = vfs_mount_alloc(None, &testfs::TESTFS_CONF);
        vfs_unbusy(mp2);
        mountlist_insert_tail(mp2);
        vfs_getnewfsid(mp2);
        assert_ne!(mp2.fsid(), fsid);
        assert_eq!(&mp2.stat().f_fstypename[..7], b"testfs\0");
    }

    #[test]
    fn checkalias_is_not_reached_for_regular_vnodes() {
        let (_g, _p, mp) = testfs::setup_root();
        let vp = testfs::vget_node(mp, testfs::B).unwrap();
        assert!(checkalias(vp, 0, Some(mp)).is_none());
        assert!(!vp.has_specinfo());
        vput(vp);
    }

    /// The export list's helpers for `bsd`'s tests of a file system's export update and its
    /// `vfs_checkexp` (ffs's, the NFS server's): the list itself is `kernel::fs::export`'s
    /// since N6.7b0, with its own tests there.
    #[cfg(feature = "nfsserver")]
    pub(crate) mod exports {
        use std::boxed::Box;

        use super::*;
        use crate::sys::mount::ExportArgs;
        use crate::sys::syslimits::NGROUPS_MAX;
        use crate::sys::ucred::Xucred;

        /// Where `args` puts the address and the mask in the host double's user memory
        /// (`USER_BASE + 0x3_0000`, above every other test's buffers).
        const USER_SA: usize = kernel::testutil::USER_BASE + 0x3_0000;

        /// A `sockaddr_in` for `a` in a 32-byte buffer.
        pub(crate) fn sin(family: u8, a: [u8; 4]) -> &'static [u8; 32] {
            let mut k = [0u8; 32];
            k[0] = 16;
            k[1] = family;
            k[4..8].copy_from_slice(&a);
            Box::leak(Box::new(k))
        }

        /// `struct export_args` for the address and mask (`None`: an empty one), both copied
        /// into the user memory double where the kernel's `copyin` reads them: build it right
        /// before the call that uses it.
        pub(crate) fn args(
            flags: i32,
            anon: u32,
            addr: Option<&'static [u8; 32]>,
            mask: Option<&'static [u8; 32]>,
        ) -> ExportArgs {
            let place = |off: usize, b: &[u8; 32]| {
                hal::user::UserPtr::<u8>::new(USER_SA + off)
                    .copyout_slice(b)
                    .expect("the double's user memory");
                USER_SA + off
            };
            ExportArgs {
                ex_flags: flags,
                ex_root: 0,
                ex_anon: Xucred {
                    cr_uid: anon,
                    cr_gid: anon,
                    cr_ngroups: 0,
                    _pad0: 0,
                    cr_groups: [0; NGROUPS_MAX],
                },
                _pad0: 0,
                ex_addr: addr.map_or(0, |a| place(0, a)),
                ex_addrlen: if addr.is_some() { 16 } else { 0 },
                _pad1: 0,
                ex_mask: mask.map_or(0, |m| place(64, m)),
                ex_masklen: if mask.is_some() { 16 } else { 0 },
                _pad2: 0,
            }
        }

        /// What the mount's `vfs_checkexp` answers a client at `a` (an `AF_INET` address
        /// mbuf; the caller has run `mbinit_again`): the export flags and the anonymous
        /// credentials' uid, or the error.
        pub(crate) fn check_export(mp: &'static Mount, a: [u8; 4]) -> Result<(i32, u32), Errno> {
            let m = kernel::net::mbuf::m_devget(&sin(2, a)[..16], 0).ok_or(Errno::ENOBUFS)?;
            let g = mp.op().checkexp(mp, &m)?;
            Ok((g.exflags, g.uid))
        }
    }
}
/* </TESTS> */
