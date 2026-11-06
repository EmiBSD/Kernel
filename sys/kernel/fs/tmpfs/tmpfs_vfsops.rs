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
 * Copyright (c) 2005, 2006, 2007 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Julio M. Merino Vidal, developed as part of Google's Summer of Code
 * 2005 program.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! Efficient memory file system: the file-system-type operations (mount, unmount, root,
//! file handles, statfs, sync, init).
//!
//! tmpfs is a file system that uses NetBSD's virtual memory sub-system (the well-known UVM)
//! to store file data and metadata in an efficient way. This means that it does not follow
//! the structure of an on-disk file system because it simply does not need to. Instead, it
//! uses memory-specific data structures and algorithms to automatically allocate and
//! release resources.
//!
//! Upstream: sys/tmpfs/tmpfs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/tmpfs/tmpfs_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - `tmpfs_mount` reads its `struct tmpfs_args` out of the kernel copy of the mount
//!   arguments (`TmpfsArgs::from_bytes`), `EINVAL` when they are short; the C dereferences
//!   the pointer it is given (LZ's).
//! - `tmpfs_mount_update` panics when the root node has no vnode, where the C would follow
//!   the NULL `tn_vnode` (LZ's).
//! - `tmpfs_root`, `tmpfs_vget` and `tmpfs_fhtovp` return the vnode (`Vfs`'s shape).
//! - `vfs_quotactl`'s `(void *)eopnotsupp` is [`tmpfs_quotactl`]; `vfs_sysctl`'s is the
//!   trait's default (`EOPNOTSUPP`); `vfs_checkexp`'s is `VFS_CHECKEXP`'s arm (`bsd`'s, at the
//!   switch).
//! - `ta_version` is not checked (the C neither).
//!
//! ## Redesign
//! - LZ's `TMPFS_VFSOPS` (a `bsd` `Vfsops` of function pointers) -> `impl Vfs<G> for
//!   TmpfsVfs<G>` and its constant [`TmpfsVfs::VFS`] (udf's and cd9660's shape), the
//!   functions the C's, generic over the glue, in its order. LZ's two `Pool` statics, the
//!   `tmpfs_bytes_limit`/`tmpfs_bytes_used` atomics of `tmpfs_mem.rs` and the tables
//!   `tmpfs_vnode_get` installs are the glue's [`TmpfsTables`] (`VfsTables::tmpfs`), the pools
//!   recorded (`tmpfs.rs`'s notes) with the C's names and alignment.
//! - `tmpfs_mount` makes the [`TmpfsMount`] whole where the C mallocs it (a counted
//!   `malloc(M_MISCFSMNT)` block, the limits, the counts and the locks of the C's fill),
//!   allocates the root against it, and hangs it from the mount's typed slot at the C's
//!   `mp->mnt_data = tmp`; nothing can refuse in between (`KASSERT(error == 0)`), so the
//!   `M_MISCFSMNT` requests are the C's on every path. `tmpfs_unmount` takes it back from the
//!   slot (`free(tmp, M_MISCFSMNT)` with the last handle). LZ wrote it into a raw `malloc`
//!   block and freed it by hand (1 `unsafe`).
//! - The unmount's rounds walk `tm_nodes` under `tm_lock` (the list's witness): the C takes
//!   no lock in its first round (tmpfs_vfsops.c:230-244, `LIST_FOREACH` unlocked) and
//!   `tm_lock` once per node in its second (inside each `tmpfs_free_node`,
//!   tmpfs_subr.c:234-236); the kernel holds it across the first round (one pair the C lacks)
//!   and once across the whole second (`free_node_unlinked`), and in both nests `tm_acc_lock`
//!   under it (the first round's `tmpfs_free_dirent` -> `tmpfs_mem_decr`, the second's freed
//!   nodes' accounting), an order the C never makes (`tmpfs_subr.rs`'s lock order). Two
//!   pairs against the C's one per node; only the unmount runs there.
//! - `tmpfs_statfs` divides the free memory by `sizeof(tmpfs_node_t)`, the C's 272
//!   ([`TMPFS_NODE_CHARGE`], D-N5.9-6): `f_files`/`f_ffree` are OpenBSD's.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::intr::IPL_NONE;
use hal::malloc::{M_WAITOK, MArc};
use hal::pool::{PR_WAITOK, PoolArc, TypedPool};
use hal::sys::errno::Errno;
use hal::sys::limits::INT_MAX;
use hal::sys::mount::{
    Fid, MNAMELEN, MNT_FORCE, MNT_LOCAL, MNT_RDONLY, MNT_UPDATE, MNT_WANTRDWR, Statfs, TmpfsArgs,
};
use hal::sys::stat::ALLPERMS;
use hal::sys::types::{Gid, Ino, Mode, Uid};
use hal::vm::uvmexp::UVMEXP;

use crate::assert::{kassert, kpanic};
use crate::fs::mount::{Mount, Vfsconf};
use crate::fs::subr::{copy_statfs_info, vflush, vfs, vfs_getnewfsid};
use crate::fs::tmpfs::tmpfs::{
    TMPFS_MAXNAMLEN, TMPFS_NODE_CHARGE, TmpfsDirent, TmpfsFid, TmpfsMount, TmpfsNode, TmpfsNodeRef,
    tmpfs_node_gen, vfstotmpfs, vptonode,
};
use crate::fs::tmpfs::tmpfs_fifoops::TmpfsFifoVnops;
use crate::fs::tmpfs::tmpfs_mem::{tmpfs_mntmem_destroy, tmpfs_pages_avail, tmpfs_pages_total};
use crate::fs::tmpfs::tmpfs_specops::TmpfsSpecVnops;
use crate::fs::tmpfs::tmpfs_subr::{
    free_node_unlinked, tmpfs_alloc_node, tmpfs_dir_detach, tmpfs_free_dirent, tmpfs_vnode_get,
};
use crate::fs::tmpfs::tmpfs_vnops::TmpfsVnops;
use crate::fs::vnode::{
    FORCECLOSE, LK_EXCLUSIVE, LK_RETRY, VDIR, VNOVAL, Vnode, WRITECLOSE, vn_lock,
};
use crate::fs::vops::{Cred, Nd, Vfs, vop_unlock};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};

/// `M_MISCFSMNT`: the malloc type of `struct tmpfs_mount` (`vmstat -m`'s "miscfs mount").
const M_MISCFSMNT: i32 = 64;

/// tmpfs's globals of the glue `G`: `VfsTables::tmpfs` (feature `tmpfs`).
pub struct TmpfsTables<G: ProcGlue> {
    /// `tmpfs_node_pool` (`"tmpfs_node"`), recorded (the vnode's slot counts a handle back
    /// from the node it lends, the mount's list parks one).
    pub node_pool: TypedPool<TmpfsNode<G>>,
    /// `tmpfs_dirent_pool` (`"tmpfs_dirent"`), recorded (a directory's list parks one).
    pub dirent_pool: TypedPool<TmpfsDirent<G>>,
    /// `tmpfs_bytes_limit`: the memory all tmpfs mounts without a limit of their own may use
    /// (`tmpfs_init`: half of physical memory).
    pub bytes_limit: AtomicU64,
    /// `tmpfs_bytes_used`: the memory reserved by the mounts with a limit plus the memory
    /// used by those without one.
    pub bytes_used: AtomicU64,
    /// `tmpfs_vops`, the operations `tmpfs_vnode_get` gives a vnode (one address per glue).
    pub vops: TmpfsVnops<G>,
    /// `tmpfs_specvops`, those of a device node's vnode.
    pub specvops: TmpfsSpecVnops<G>,
    /// `tmpfs_fifovops`, those of a fifo's (installed by nothing until fifofs is ported:
    /// `tmpfs_fifoops.rs`'s deviations).
    pub fifovops: TmpfsFifoVnops<G>,
}

impl<G: ProcGlue> TmpfsTables<G> {
    /// No pool made yet, nothing used, no table.
    pub const fn new() -> Self {
        Self {
            node_pool: TypedPool::new(),
            dirent_pool: TypedPool::new(),
            bytes_limit: AtomicU64::new(0),
            bytes_used: AtomicU64::new(0),
            vops: TmpfsVnops::new(),
            specvops: TmpfsSpecVnops::new(),
            fifovops: TmpfsFifoVnops::new(),
        }
    }
}

impl<G: ProcGlue> Default for TmpfsTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The tmpfs file system of the glue `G`: `tmpfs_vfsops` ([`TmpfsVfs::VFS`]).
pub struct TmpfsVfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> TmpfsVfs<G> {
    /// `tmpfs_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"tmpfs"`).
    pub const VFS: TmpfsVfs<G> = TmpfsVfs(PhantomData);
}

/// `tmpfs_init` (`vfs_init`): the global memory limit (half of the managed pages) and the
/// node and directory entry pools; a second call (a host test's) keeps the pools made first
/// (their `init` answers `EBUSY`).
pub fn tmpfs_init<G: ProcGlue>() -> Result<(), Errno> {
    let t = &vfs::<G>().tmpfs;
    let npages = UVMEXP.npages.load(Ordering::Relaxed);
    t.bytes_limit
        .store(((npages / 2) as u64) << PAGE_SHIFT, Ordering::Relaxed);

    for r in [
        t.dirent_pool
            .init_recorded(IPL_NONE, PR_WAITOK, "tmpfs_dirent"),
        t.node_pool.init_recorded(IPL_NONE, PR_WAITOK, "tmpfs_node"),
    ] {
        match r {
            Ok(()) | Err(Errno::EBUSY) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

/// `tmpfs_mount_update`: `mount -u`. Only a read-write to read-only change is supported:
/// it flushes the files opened for writing.
pub fn tmpfs_mount_update<G: ProcGlue>(mp: &'static Mount<G>) -> Result<(), Errno> {
    if mp.flag() & MNT_RDONLY == 0 {
        return Err(Errno::EOPNOTSUPP);
    }

    // ro->rw transition: nothing to do?
    if mp.flag() & MNT_WANTRDWR != 0 {
        return Ok(());
    }

    let tmp = vfstotmpfs(mp);
    let Some(rootvp) = tmp.root().vnode() else {
        kpanic(format_args!("tmpfs_mount_update: the root has no vnode"));
    };

    // Lock root to prevent lookups.
    vn_lock(rootvp, LK_EXCLUSIVE | LK_RETRY)?;

    // Lock mount point to prevent nodes from being added/removed.
    let error = {
        let _w = tmp.tm_lock.write();

        // Flush files opened for writing; skip rootvp.
        vflush(mp, Some(rootvp), WRITECLOSE)
    };

    let _ = vop_unlock(rootvp);

    error
}

/// `roundup(x, PAGE_SIZE)` over the C's `off_t` (a negative size wraps as in C).
fn roundup_page(x: i64) -> i64 {
    let y = PAGE_SIZE as i64;
    (x.wrapping_add(y - 1) / y).wrapping_mul(y)
}

/// `tmpfs_mount` (`vfs_mount`): mount system call. `data` is the kernel copy of the user's
/// `struct tmpfs_args`.
pub fn tmpfs_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    _ndp: &mut Nd<'_, G>,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    if mp.flag() & MNT_UPDATE != 0 {
        return tmpfs_mount_update(mp);
    }

    let Some(args) = TmpfsArgs::from_bytes(data) else {
        return Err(Errno::EINVAL);
    };

    if args.ta_root_uid == VNOVAL as Uid
        || args.ta_root_gid == VNOVAL as Gid
        || args.ta_root_mode == VNOVAL as Mode
    {
        return Err(Errno::EINVAL);
    }

    // Get the memory usage limit for this file-system.
    let t = &vfs::<G>().tmpfs;
    let mut memlimit: u64 = 0;
    if args.ta_size_max != 0 {
        memlimit = roundup_page(args.ta_size_max) as u64;

        let avail = t
            .bytes_limit
            .load(Ordering::Relaxed)
            .wrapping_sub(t.bytes_used.load(Ordering::Relaxed));
        if avail < memlimit {
            return Err(Errno::EINVAL); // historic error
        }
        t.bytes_used.fetch_add(memlimit, Ordering::Relaxed);
    }

    let mut nodes: u64 = if args.ta_nodes_max <= 3 {
        3 + (if memlimit != 0 { memlimit } else { u64::MAX }) / 1024
    } else {
        args.ta_nodes_max
    };
    nodes = nodes.min(INT_MAX as u64);
    kassert!(nodes >= 3);

    // Allocate the tmpfs mount structure and fill it (tm_nodes_max, tm_nodes_cnt 0,
    // tm_highest_inode 1, the empty list, tm_lock, tmpfs_mntmem_init's accounting).
    let Some(tmp) = MArc::new(
        M_MISCFSMNT,
        M_WAITOK,
        TmpfsMount::<G>::new(nodes as u32, memlimit),
    ) else {
        kpanic(format_args!("tmpfs_mount: malloc failed"));
    };

    // Allocate the root node.
    let root = tmpfs_alloc_node(
        &tmp,
        VDIR,
        args.ta_root_uid,
        args.ta_root_gid,
        args.ta_root_mode & ALLPERMS,
        None,
        VNOVAL,
    );
    kassert!(root.is_ok());
    let root = match root {
        Ok(root) => root,
        Err(e) => kpanic(format_args!(
            "tmpfs_mount: no root node (error {})",
            e as i32
        )),
    };

    // Parent of the root inode is itself. Also, root inode has no directory entry (i.e. is
    // never attached), thus hold an extra reference (link) for it.
    root.tn_links.set(root.tn_links.get() + 1);
    drop(root.dir().tn_parent.set(Some(PoolArc::clone(&root))));
    let _ = tmp.tm_root.set(root);

    mp.set_node(MArc::erase(tmp));
    mp.set_flag(mp.flag() | MNT_LOCAL);
    mp.with_stat(|sp| sp.f_namemax = TMPFS_MAXNAMLEN as u32);
    vfs_getnewfsid(mp);

    mp.with_stat(|sp| {
        sp.mount_info.__align[..TmpfsArgs::SIZE].copy_from_slice(&data[..TmpfsArgs::SIZE]);

        sp.f_mntonname = [0; MNAMELEN];
        sp.f_mntfromname = [0; MNAMELEN];
        sp.f_mntfromspec = [0; MNAMELEN];

        libkern::strlcpy(&mut sp.f_mntonname[..MNAMELEN - 1], path);
        libkern::strlcpy(&mut sp.f_mntfromname[..MNAMELEN - 1], b"tmpfs");
        libkern::strlcpy(&mut sp.f_mntfromspec[..MNAMELEN - 1], b"tmpfs");
    });

    Ok(())
}

/// `tmpfs_start` (`vfs_start`).
pub fn tmpfs_start<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _flags: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    Ok(())
}

/// `tmpfs_unmount` (`vfs_unmount`): flush the vnodes, then destroy every directory entry
/// and node and the mount structure.
pub fn tmpfs_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let tmp = vfstotmpfs(mp);
    let mut flags = 0;

    // Handle forced unmounts.
    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }

    // Finalize all pending I/O.
    vflush(mp, None, flags)?;

    // First round, detach and destroy all directory entries. Also, clear the pointers to
    // the vnodes - they are gone.
    {
        let w = tmp.tm_lock.write();
        for node in tmp.tm_nodes.iter(&w) {
            node.tn_vnode.set(None);
            if node.tn_type != VDIR {
                continue;
            }
            let Some(node) = PoolArc::from_ref(node) else {
                kpanic(format_args!("tmpfs_unmount: a node of no pool"));
            };
            loop {
                let de = {
                    let r = hal::klock::reader();
                    node.dir().tn_dir.first(&r).and_then(PoolArc::from_ref)
                };
                let Some(de) = de else {
                    break;
                };
                if let Some(cnode) = de.td_node() {
                    cnode.tn_vnode.set(None);
                }
                tmpfs_dir_detach(&node, &de);
                tmpfs_free_dirent(&tmp, de);
            }
        }
    }

    // Second round, destroy all inodes.
    {
        let mut w = tmp.tm_lock.write();
        loop {
            let Some(node) = tmp.tm_nodes.first(&w).and_then(PoolArc::from_ref) else {
                break;
            };
            tmp.tm_nodes.remove(&mut w, &node);
            free_node_unlinked(&tmp, node);
        }
    }

    let limit = tmp.tm_acc.read().tm_mem_limit;
    if limit != 0 {
        vfs::<G>()
            .tmpfs
            .bytes_used
            .fetch_sub(limit, Ordering::Relaxed);
    }

    // Throw away the tmpfs_mount structure.
    tmpfs_mntmem_destroy(&tmp);
    // mutex_destroy(&tmp->tm_lock); kmem_free(tmp, sizeof(*tmp)): with the last handle.
    drop(mp.take_node());
    drop(tmp);

    Ok(())
}

/// `tmpfs_root` (`vfs_root`): the root directory's vnode, locked.
pub fn tmpfs_root<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    let tmp = vfstotmpfs(mp);
    let node = tmp.root();

    tmpfs_vnode_get(mp, node, node.tn_nlock.write())
}

/// `tmpfs_quotactl` (`vfs_quotactl`'s `(void *)eopnotsupp`).
pub fn tmpfs_quotactl() -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `tmpfs_vget` (`vfs_vget`).
pub fn tmpfs_vget<G: ProcGlue>(
    _mp: &'static Mount<G>,
    _ino: Ino,
) -> Result<&'static Vnode<G>, Errno> {
    printf(format_args!("tmpfs_vget called; need for it unknown yet\n"));
    Err(Errno::EOPNOTSUPP)
}

/// `tmpfs_fhtovp` (`vfs_fhtovp`): the vnode of the node a file handle names, locked; the C's
/// linear scan of `tm_nodes` under `tm_lock`.
pub fn tmpfs_fhtovp<G: ProcGlue>(
    mp: &'static Mount<G>,
    fhp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    let tmp = vfstotmpfs(mp);

    if usize::from(fhp.fid_len) != TmpfsFid::SIZE {
        return Err(Errno::EINVAL);
    }
    let tfh = TmpfsFid::from_fid(fhp);

    let node: Option<TmpfsNodeRef<G>> = {
        let w = tmp.tm_lock.write();
        tmp.tm_nodes
            .iter(&w)
            .find(|node| {
                node.tn_id.get() == tfh.tf_id && tmpfs_node_gen(node) == u64::from(tfh.tf_gen)
            })
            .and_then(PoolArc::from_ref)
    };

    // Will release the tn_nlock (taken under tm_lock in the C: the node cannot go while the
    // handle is held here).
    match &node {
        Some(node) => tmpfs_vnode_get(mp, node, node.tn_nlock.write()),
        None => Err(Errno::ESTALE),
    }
}

/// `tmpfs_vptofh` (`vfs_vptofh`): the file handle of a vnode's node.
pub fn tmpfs_vptofh<G: ProcGlue>(vp: &'static Vnode<G>, fhp: &mut Fid) -> Result<(), Errno> {
    let node = vptonode(vp);

    let tfh = TmpfsFid {
        tf_len: TmpfsFid::SIZE as u16,
        tf_pad: 0,
        tf_gen: tmpfs_node_gen(&node) as u32,
        tf_id: node.tn_id.get(),
    };
    tfh.to_fid(fhp);

    Ok(())
}

/// `tmpfs_statfs` (`vfs_statfs`).
pub fn tmpfs_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let tmp = vfstotmpfs(mp);

    sbp.f_bsize = PAGE_SIZE as u32;
    sbp.f_iosize = PAGE_SIZE as u32;

    {
        let acc = tmp.tm_acc.read();
        let avail = tmpfs_pages_avail::<G>(&acc);
        sbp.f_blocks = tmpfs_pages_total::<G>(&acc);
        sbp.f_bfree = avail;
        sbp.f_bavail = (avail & i64::MAX as u64) as i64; // f_bavail is int64_t

        let freenodes = u64::from(tmp.tm_nodes_max.wrapping_sub(tmp.tm_nodes_cnt.get()))
            .min(avail.wrapping_mul(PAGE_SIZE as u64) / TMPFS_NODE_CHARGE as u64);

        sbp.f_files = u64::from(tmp.tm_nodes_cnt.get()) + freenodes;
        sbp.f_ffree = freenodes;
        sbp.f_favail = (freenodes & i64::MAX as u64) as i64; // f_favail is int64_t
    }

    copy_statfs_info(sbp, mp);

    Ok(())
}

/// `tmpfs_sync` (`vfs_sync`): nothing to write back.
pub fn tmpfs_sync() -> Result<(), Errno> {
    Ok(())
}

/// `tmpfs_vfsops`: each slot the C's function of the same name; `vfs_sysctl` the trait's
/// default (`EOPNOTSUPP`, the C's `eopnotsupp`); `vfs_checkexp` is `VFS_CHECKEXP`'s arm (the
/// module's deviations).
impl<G: ProcGlue> Vfs<G> for TmpfsVfs<G> {
    /// `vfs_mount`: [`tmpfs_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        tmpfs_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: [`tmpfs_start`].
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno> {
        tmpfs_start(mp, flags, p)
    }

    /// `vfs_unmount`: [`tmpfs_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        tmpfs_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: [`tmpfs_root`].
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        tmpfs_root(mp)
    }

    /// `vfs_quotactl`: [`tmpfs_quotactl`].
    fn quotactl(
        &self,
        _mp: &'static Mount<G>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        tmpfs_quotactl()
    }

    /// `vfs_statfs`: [`tmpfs_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        tmpfs_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`tmpfs_sync`].
    fn sync(
        &self,
        _mp: &'static Mount<G>,
        _waitfor: i32,
        _stall: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        tmpfs_sync()
    }

    /// `vfs_vget`: [`tmpfs_vget`].
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        tmpfs_vget(mp, ino)
    }

    /// `vfs_fhtovp`: [`tmpfs_fhtovp`].
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        tmpfs_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: [`tmpfs_vptofh`].
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        tmpfs_vptofh(vp, fid)
    }

    /// `vfs_init`: [`tmpfs_init`].
    fn init(&self, _vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        tmpfs_init::<G>()
    }
}

/// The glue's tmpfs globals forgotten for the next host test (`vfs_test_reset`'s): nothing
/// used, the global limit 0 (the pools and the tables stay, as the other file systems').
#[cfg(any(test, feature = "doubles"))]
pub fn tmpfs_test_reset<G: ProcGlue>() {
    let t = &vfs::<G>().tmpfs;
    t.bytes_limit.store(0, Ordering::Relaxed);
    t.bytes_used.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for tmpfs over a mount of the fixture (testtmpfs, LZ's moved): mount's
    // refusals, the root vnode, files, directories and symbolic links made with
    // `tmpfs_alloc_file`, lookups, `getdents`, resizing a file's object, file handles,
    // `statfs`'s numbers at the C's charges (D-N5.9-6), the malloc and pool requests against
    // the C's (vmstat -m), and `tmpfs_unmount`, which gives every node, entry and byte back.

    use std::vec::Vec;

    use super::*;
    use crate::fs::namei::Componentname;
    use crate::fs::subr::{vput, vrele};
    use crate::fs::testfs::TestGlue;
    use crate::fs::tmpfs::testtmpfs::{self, args, mount};
    use crate::fs::tmpfs::tmpfs::{
        TMPFS_DIRENT_CHARGE, TMPFS_DIRSEQ_EOF, TMPFS_DIRSEQ_START, vptodir,
    };
    use crate::fs::tmpfs::tmpfs_mem::M_TEMP;
    use crate::fs::tmpfs::tmpfs_subr::{
        tmpfs_alloc_file, tmpfs_dir_cached, tmpfs_dir_getdents, tmpfs_dir_lookup, tmpfs_reg_resize,
        tmpfs_truncate,
    };
    use crate::fs::vnode::{VLNK, VREG, VROOT, Vattr, Vtype};
    use crate::testutil::{malloc_out, malloc_requests};
    use crate::uio::{Iovec, Uio};
    use hal::sys::dirent::Dirent;
    use hal::sys::types::{Dev, Off};

    type P = &'static Thread<TestGlue>;

    /// A component name for `name`, with the thread's credentials and no pathname buffer.
    fn cn(p: P, name: &'static [u8]) -> Componentname<'static, TestGlue> {
        let mut cnp = Componentname::new(0, 0, p, Cred::real(p.ucred()));
        cnp.set_name_lit(name);
        cnp
    }

    /// The attributes `VOP_CREATE`/`VOP_MKDIR`/`VOP_SYMLINK` pass for a new node.
    fn vattr(type_: Vtype, mode: Mode) -> Vattr {
        let mut va = Vattr::new();
        va.va_type = type_;
        va.va_mode = mode;
        va.va_rdev = VNOVAL as Dev;
        va
    }

    /// `getdents` of a directory from `offset` into a buffer of `len` bytes: the names read
    /// and the offset the read ended at.
    fn getdents(node: &TmpfsNode<TestGlue>, offset: Off, len: usize) -> (Vec<Vec<u8>>, Off) {
        let mut buf = std::vec![0u8; len];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::<TestGlue>::kernel_read(&mut iov, &mut buf, None);
        uio.uio_offset = offset;
        tmpfs_dir_getdents(node, &mut uio).expect("getdents");
        let used = len - uio.uio_resid;
        let end = uio.uio_offset;

        let mut names = Vec::new();
        let mut off = 0;
        while off < used {
            let d = Dirent::from_bytes(&buf[off..]).expect("a dirent");
            let name = &buf[off + Dirent::NAME_OFFSET..][..usize::from(d.d_namlen)];
            names.push(name.to_vec());
            off += usize::from(d.d_reclen);
        }
        (names, end)
    }

    fn used() -> u64 {
        vfs::<TestGlue>().tmpfs.bytes_used.load(Ordering::Relaxed)
    }

    #[test]
    fn mount_rejects_unset_owners_and_short_arguments() {
        let (_g, p) = testtmpfs::setup();
        let mut data = args(0, 0, VNOVAL as Uid, 0o755);
        assert_eq!(mount(p, &mut data).err(), Some(Errno::EINVAL));
        let mut short = [0u8; 8];
        assert_eq!(mount(p, &mut short).err(), Some(Errno::EINVAL));
        // A limit past what is free: the historic EINVAL, nothing reserved.
        let mut huge = args(i64::MAX - 4096, 0, 0, 0o755);
        assert_eq!(mount(p, &mut huge).err(), Some(Errno::EINVAL));
        assert_eq!(used(), 0);
    }

    #[test]
    fn files_directories_and_links_live_and_die_with_the_mount() {
        let (_g, p) = testtmpfs::setup();
        let (mnt_reqs, _) = malloc_requests(M_MISCFSMNT);
        let (mnt_out, temp_out) = (malloc_out(M_MISCFSMNT), malloc_out(M_TEMP));
        let mut data = args(64 * 1024, 0, 0, 0o1777);
        let mp = mount(p, &mut data).expect("mount");
        let tmp = vfstotmpfs(mp);
        assert_eq!(
            malloc_requests(M_MISCFSMNT).0,
            mnt_reqs + 1,
            "one M_MISCFSMNT block"
        );
        assert_eq!(used(), 64 * 1024);
        assert_eq!(tmp.tm_nodes_max, 3 + 64);
        assert_eq!(&mp.stat().f_mntonname[..5], b"/tmp\0");
        assert_eq!(&mp.stat().f_mntfromname[..6], b"tmpfs\0");
        assert_eq!(mp.stat().f_namemax, 255);
        assert!(mp.flag() & MNT_LOCAL != 0);

        // The root: its own parent, mode as given, a vnode flagged VROOT.
        let root = PoolArc::clone(tmp.root());
        assert!(root.dir().tn_parent.points_to(&root));
        assert_eq!(root.tn_links.get(), 2);
        assert_eq!(root.tn_mode.get(), 0o1777);
        let dvp = tmpfs_root(mp).expect("root vnode");
        assert!(dvp.flag() & VROOT != 0);
        assert!(PoolArc::ptr_eq(&vptodir(dvp), &root));

        // A file, a directory and a symbolic link: an M_TEMP name each, the link's target a
        // fourth.
        let (temp_reqs, _) = malloc_requests(M_TEMP);
        let fvp =
            tmpfs_alloc_file(dvp, &vattr(VREG, 0o644), &mut cn(p, b"file"), None).expect("create");
        let ddvp =
            tmpfs_alloc_file(dvp, &vattr(VDIR, 0o755), &mut cn(p, b"dir"), None).expect("mkdir");
        let lvp = tmpfs_alloc_file(
            dvp,
            &vattr(VLNK, 0o777),
            &mut cn(p, b"link"),
            Some(b"file\0"),
        )
        .expect("symlink");
        assert_eq!(malloc_requests(M_TEMP).0, temp_reqs + 4);
        let (file, sub, link) = (vptonode(fvp), vptonode(ddvp), vptonode(lvp));
        assert_eq!(root.tn_size.get(), 3 * TMPFS_DIRENT_CHARGE as Off);
        assert_eq!(root.tn_links.get(), 3, "'.', the root's own and dir's '..'");
        assert_eq!(file.tn_links.get(), 1);
        assert_eq!(sub.tn_links.get(), 2);
        assert!(sub.dir().tn_parent.points_to(&root));
        assert_eq!(link.link(), b"file");
        assert_eq!(link.tn_size.get(), 4);
        assert_eq!(tmp.tm_nodes_cnt.get(), 4);

        // Lookups.
        let de = tmpfs_dir_lookup(&root, &cn(p, b"dir")).expect("dir's entry");
        assert!(de.td_node_is(&sub));
        assert_eq!(de.td_seq.get(), TMPFS_DIRSEQ_START + 1);
        drop(de);
        assert!(tmpfs_dir_lookup(&root, &cn(p, b"none")).is_none());
        assert!(tmpfs_dir_cached(&file).is_some_and(|de| de.name_is(b"file")));

        // getdents: everything at once, then two entries at a time.
        let (names, end) = getdents(&root, 0, 1024);
        assert_eq!(names, [&b"."[..], b"..", b"file", b"dir", b"link"]);
        assert_eq!(end as u64, TMPFS_DIRSEQ_EOF);
        let (names, end) = getdents(&root, 0, 64);
        assert_eq!(names, [&b"."[..], b".."]);
        assert_eq!(end as u64, TMPFS_DIRSEQ_START);
        let (names, end) = getdents(&root, end, 64);
        assert_eq!(names, [&b"file"[..], b"dir"]);
        let (names, end) = getdents(&root, end, 64);
        assert_eq!(names, [&b"link"[..]]);
        assert_eq!(end as u64, TMPFS_DIRSEQ_EOF);
        let (names, _) = getdents(&sub, 0, 1024);
        assert_eq!(names, [&b"."[..], b".."]);

        // The file's object grows and shrinks a page at a time, accounted to the mount.
        let before = tmp.tm_acc.read().tm_bytes_used;
        tmpfs_reg_resize(fvp, 3 * PAGE_SIZE as Off).expect("grow");
        assert_eq!(file.reg().tn_aobj_pages.get(), 3);
        assert_eq!(
            tmp.tm_acc.read().tm_bytes_used,
            before + 3 * PAGE_SIZE as u64
        );
        tmpfs_truncate(fvp, PAGE_SIZE as Off).expect("shrink");
        assert_eq!(file.reg().tn_aobj_pages.get(), 1);
        assert_eq!(file.tn_size.get(), PAGE_SIZE as Off);
        assert_eq!(
            tmpfs_reg_resize(fvp, 1 << 20).err(),
            Some(Errno::ENOSPC),
            "past the 64 KB limit"
        );
        assert_eq!(file.tn_size.get(), PAGE_SIZE as Off);

        // File handles.
        let mut fid = Fid::default();
        tmpfs_vptofh(fvp, &mut fid).expect("vptofh");
        assert_eq!(usize::from(fid.fid_len), TmpfsFid::SIZE);
        let _ = vop_unlock(fvp);
        let again = tmpfs_fhtovp(mp, &fid).expect("fhtovp");
        assert!(core::ptr::eq(again, fvp));
        vput(again);
        let mut stale = fid;
        stale.fid_data[4] ^= 0xff;
        assert_eq!(tmpfs_fhtovp(mp, &stale).err(), Some(Errno::ESTALE));
        let mut short = fid;
        short.fid_len = 12;
        assert_eq!(tmpfs_fhtovp(mp, &short).err(), Some(Errno::EINVAL));

        // statfs: 16 pages; the nodes left are the fewer of the count left and the free
        // memory over the C's sizeof(tmpfs_node_t).
        let mut sb = Statfs::default();
        tmpfs_statfs(mp, &mut sb, p).expect("statfs");
        assert_eq!(sb.f_blocks, 16);
        let avail = sb.f_bfree;
        let want = u64::from(tmp.tm_nodes_max - 4).min(avail * PAGE_SIZE as u64 / 272);
        assert_eq!((sb.f_ffree, sb.f_files), (want, 4 + want));
        assert_eq!(sb.f_bsize, PAGE_SIZE as u32);

        // Drop the vnodes (fvp is unlocked already), then unmount: every node, entry, name
        // and byte comes back.
        drop((file, sub, link, root));
        vrele(fvp);
        vput(ddvp);
        vput(lvp);
        vput(dvp);
        assert_eq!(tmp.tm_nodes_cnt.get(), 4);
        drop(tmp);
        tmpfs_unmount(mp, 0, p).expect("unmount");
        assert!(!mp.has_node());
        assert_eq!(used(), 0);
        assert_eq!(
            malloc_out(M_MISCFSMNT),
            mnt_out,
            "the mount structure freed"
        );
        assert_eq!(malloc_out(M_TEMP), temp_out, "every name and target freed");
        let t = &vfs::<TestGlue>().tmpfs;
        assert_eq!((t.node_pool.nout(), t.dirent_pool.nout()), (0, 0));
    }

    #[test]
    fn the_node_limit_is_enospc() {
        let (_g, p) = testtmpfs::setup();
        let mut data = args(0, 4, 0, 0o755);
        let mp = mount(p, &mut data).expect("mount");
        let tmp = vfstotmpfs(mp);
        assert_eq!(tmp.tm_nodes_max, 4);
        let dvp = tmpfs_root(mp).expect("root vnode");

        let mut vps = Vec::new();
        for name in [&b"a"[..], b"b", b"c"] {
            let name: &'static [u8] = std::boxed::Box::leak(name.to_vec().into_boxed_slice());
            vps.push(
                tmpfs_alloc_file(dvp, &vattr(VREG, 0o644), &mut cn(p, name), None).expect("create"),
            );
        }
        assert_eq!(
            tmpfs_alloc_file(dvp, &vattr(VREG, 0o644), &mut cn(p, b"d"), None).err(),
            Some(Errno::ENOSPC)
        );
        assert_eq!(tmp.tm_nodes_cnt.get(), 4);

        for vp in vps {
            vput(vp);
        }
        vput(dvp);
        drop(tmp);
        tmpfs_unmount(mp, 0, p).expect("unmount");
    }

    #[test]
    fn the_pools_are_the_cs_with_their_items() {
        // vmstat -m's rows: the C's names; an item is the kernel structure and the handle's
        // count and pool (`pr_size`, the accepted class of ZERO_UNSAFE 0.10).
        let (_g, _p) = testtmpfs::setup();
        let t = &vfs::<TestGlue>().tmpfs;
        assert_eq!(
            t.node_pool.item_size(),
            Some(size_of::<TmpfsNode<TestGlue>>() + 16)
        );
        assert_eq!(
            t.dirent_pool.item_size(),
            Some(size_of::<TmpfsDirent<TestGlue>>() + 16)
        );
        std::println!(
            "tmpfs pool items: tmpfs_node {} (TmpfsNode {}), tmpfs_dirent {} (TmpfsDirent {})",
            t.node_pool.item_size().unwrap_or(0),
            size_of::<TmpfsNode<TestGlue>>(),
            t.dirent_pool.item_size().unwrap_or(0),
            size_of::<TmpfsDirent<TestGlue>>()
        );
    }
}
/* </TESTS> */
