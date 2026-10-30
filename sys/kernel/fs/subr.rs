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
//! External virtual filesystem routines: the vnode table (`getnewvnode`, the free and hold
//! lists, `vget`/`vref`/`vput`/`vrele`, `vhold`/`vdrop`, `vclean`/`vgone`/`vflush`), the
//! special-device aliases (`bdevvp`, `cdevvp`, `checkalias`, `vfinddev`, `vcount`), the mount
//! helpers (`vfs_rootmountalloc`, `vfs_getvfs`, `vfs_getnewfsid`, `vfs_stall`), `vattr_null`,
//! `vaccess`, `vfs_sysctl` and the vnode/buffer helpers; the subsystem's tables
//! ([`VfsTables`]) and the vnode as the buffer cache and the pager reach it.
//!
//! Upstream: sys/kern/vfs_subr.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_subr.rs@3c62ede68802
//!
//! Vnodes are never freed: a vnode whose last user goes (`vrele`, `vput`) is deactivated
//! (`VOP_INACTIVE`) and put on the free list (or the hold list while buffers reference it),
//! still carrying its file system's identity, so a later lookup can `vget` it back.
//! `getnewvnode` allocates new vnodes until `maxvnodes` and then recycles from the free
//! lists, `vgonel`ing the old identity away.
//!
//! ## Deviations
//! - Out-parameters are return values: `getnewvnode` and `vfs_rootmountalloc` return the
//!   vnode or mount, `bdevvp`/`cdevvp` return `Option` (the C's NULL for `NODEV`),
//!   `vfinddev` returns `Option`. `vrele` and `vrecycle` return `bool` (LZ's, kept).
//! - `vinvalbuf` panics if it finds dirty buffers with no thread to `VOP_FSYNC` them (the C
//!   always has `curproc`; LZ's, kept).
//! - `vprint` is compiled under feature `diagnostic` or `debug`, `printlockedvnodes` under
//!   `debug`, as in C. The DDB printers wait for the ddb command loop (LZ's, kept).
//! - `vfs_unmountall`, `vfs_shutdown` and `vfs_syncwait` stay in `bsd`'s `vfs_subr.rs` until
//!   `dounmount` and `sys_sync` move (N5.4); so does the NFS export list
//!   (`vfs_hang_addrlist`, `vfs_export`, `vfs_export_lookup`, feature `nfsserver`, and
//!   `vntblinit`'s `rn_init`), until the NFS server moves (N6).
//! - `vfs_stall` reads each successor before the body ([`mountlist_safe_restart`]), and a
//!   successor unmounted while the body slept (`vfs_busy`'s wait, `VFS_SYNC`) restarts the
//!   walk from the head past the mounts already handled (`MNT_STALLED` set when stalling,
//!   clear when resuming): every mount still listed is synced and stalled. The C follows
//!   the unmounted mount's stale `TAILQ_NEXT` (vfs_subr.c:1720-1726), a read of freed memory;
//!   LZ's and N5.2's walk ended there, leaving the later mounts unsynced (D-N5.3-7). A mount
//!   whose sync failed is tried again after such a restart.
//!
//! ## Redesign
//! - LZ's global lists of `Cell` links under `unsafe impl Sync` (`VNODE_FREE_LIST`,
//!   `VNODE_HOLD_LIST`, `MOUNTLIST`, `SPECLISTH`, the syncer's wheel) and the unsafe item
//!   writes of `vnode_pool` and `malloc` -> [`VfsTables<G>`], one per glue (`bsd`'s static,
//!   a test thread's own): the vnode pool (`BoxPool`, items leaked: vnodes are never freed),
//!   the free and hold lists under one mutex at `IPL_BIO` (the C's `splbio`, each section
//!   where the C raises it, `vputonfreelist`'s nested `splbio` inside `vput`'s), the mount
//!   lists, the alias chains and the syncer's wheel under leaf mutexes, the operations
//!   registry, `rootvnode` and the device switch.
//! - Lock order, outermost first: the kernel lock (every caller, as the C), a mount's busy
//!   lock, a vnode's lock (`VOP_LOCK`), the buffer cache's lock, the free lists' lock;
//!   `vnode_mtx` is a leaf around the `[V]` words and their sleeps; the mount lists, a
//!   mount's vnode list and statistics, the alias chains and the syncer's wheel are leaves
//!   (the wheel inside the cache's lock: `reassignbuf`'s worklist). No guard is held across
//!   a vnode or file system operation, a bridge or a sleep, with one exception the C makes:
//!   `getnewvnode` asks `VOP_ISLOCKED` of the free list's vnodes under its lock (the C at
//!   `splbio`), an operation that neither sleeps nor locks.
//! - `struct vflush_args` and its callback through `void *` -> a closure.
//! - The C's cost, step by step: every walk (the free lists in `getnewvnode`, a mount's vnodes
//!   in `vfs_mount_foreach_vnode` and `vflush`, `mountlist`, the alias chains of `checkalias`,
//!   `vcount`, `vfinddev` and `vfs_mountedon`, the syncer's slots) steps by its list's next
//!   link, O(1) a step, and restarts from the head where the C's `goto loop` does; none finds
//!   its element again by key. The spl pairs are the C's on every vnode op (`vget`, `vput`,
//!   `vrele`, `vhold`, `vdrop`, `vn_lock`, measured against vfs_subr.c's table at N5.2's
//!   switch) but for the leaf mutexes the C covers with the kernel lock (D-N5.2-13): one pair
//!   per section of the mount lists, a mount's vnode list (`insmntque`: one or two per
//!   `getnewvnode` of a mounted vnode and per `vgonel`), the alias chains and the statistics,
//!   on the mount, unmount, device-vnode and vnode-allocation paths, never on a read, a write
//!   or a lookup's hit. The statistics' words the file systems read on those paths
//!   (`f_iosize`, `f_bsize`, `f_flags`, `f_fsid`, `f_owner`) are unlocked copies
//!   (`Mount::iosize`), and a device vnode's `v_rdev` an unlocked copy of its specinfo's
//!   `si_rdev` (`Vnode::rdev`), one load per element of an alias chain's walk.

use core::any::Any;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, Ordering};

use hal::intr::{IPL_BIO, IPL_NONE};
use hal::link::Link;
use hal::list::{List, ListEntry};
use hal::malloc::MBox;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::pool::{BoxPool, PR_WAITOK, PoolBox, TypedPool};
use hal::rwlock::{Guarded, RW_READ, RW_WRITE, SplitRwlock};
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::mount::{
    Fsid, MFSNAMELEN, MNAMELEN, MNT_ASYNC, MNT_NOPERM, MNT_RDONLY, MNT_STALLED, MNT_WAIT, Statfs,
    VB_DUPOK, VB_WAIT, VB_WRITE,
};
use hal::sys::types::{Daddr, Dev, Gid, Mode, Uid, major, makedev, minor};

use super::bio::{self, BufVnode, DirtyPick, DirtyScan, NODEV};
use super::buf::{B_BUSY, B_DELWRI, Buf, BufRef};
use super::init::vfs_byname;
use super::mount::{
    Mount, MountLists, mountlist_is_empty, mountlist_safe_restart, vfs_busy, vfs_mount_alloc,
    vfs_unbusy,
};
use super::spec::{
    BySpecnext, CLONE_SHIFT, CloneBits, D_CLONE, D_TTY, SPECHSZ, Specinfo, devsw, spechash,
};
use super::sync::{self, SYNCDELAY, Wheel, vn_syncer_add_to_worklist};
use super::vnode::{
    DOCLOSE, FORCECLOSE, IGNORECLEAN, LK_DRAIN, LK_EXCLUSIVE, LK_NOWAIT, LK_TYPE_MASK, PINOD,
    REVOKEALL, SKIPSYSTEM, V_SAVE, V_SAVEMETA, VALIASED, VBAD, VBIOERROR, VBIOONFREELIST, VBLK,
    VCHR, VDIR, VEXEC, VISTTY, VN_KNOTE, VNODE_MTX, VNON, VNOVAL, VREAD, VREG, VROOT, VSYSTEM,
    VT_NON, VWRITE, VXLOCK, VXWANT, Vattr, Vnode, VopsId, Vtagtype, Vtype, WRITECLOSE, stat,
    vn_lock,
};
use super::vops::{
    Cred, IoBuf, VFS_SYNC, Vnops, vop_bmap, vop_bwrite, vop_close, vop_fsync, vop_getattr,
    vop_inactive, vop_islocked, vop_lock, vop_read, vop_reclaim, vop_revoke, vop_strategy,
    vop_unlock, vop_write,
};
use crate::assert::{kassert, kpanic};
use crate::cred::{Ucred, groupmember};
use crate::file::{FileGlue, VFS_STALLING};
use crate::proc::{ProcGlue, Thread};
use crate::uio::{Iovec, Uio};
use crate::vm::types::Voff;
use crate::vm::vnode::{Uvn, VType, VnodeOps};

/// `M_VNODE`: the malloc type of `struct specinfo` (`<sys/malloc.h>`).
pub const M_VNODE: i32 = 25;
/// `M_WAITOK` (`<sys/malloc.h>`).
const M_WAITOK: i32 = 0x0001;

/// `v_freelist`'s adapter: the free list and the hold list.
pub struct ByFreelist<G: ProcGlue>(core::marker::PhantomData<G>);

impl<G: ProcGlue> hal::list::ListAdapter for ByFreelist<G> {
    type Elem = Vnode<G>;
    fn entry(e: &Vnode<G>) -> &ListEntry<Vnode<G>> {
        &e.freelist
    }
}

/// `vnode_free_list` and `vnode_hold_list`, under the free lists' lock (the C's `splbio`).
pub struct FreeLists<G: ProcGlue> {
    /// `vnode_free_list`: vnode free list.
    pub free: List<ByFreelist<G>>,
    /// `vnode_hold_list`: list of vnodes referencing buffers.
    pub hold: List<ByFreelist<G>>,
}

/// The free lists' guard: the witness of every vnode's `[B]` hold count.
pub type ListsGuard<'a, G> = MutexedGuard<'a, FreeLists<G>>;

/// The vnode and mount tables of one glue's processes: `bsd`'s `static`, or a test thread's.
pub struct VfsTables<G: ProcGlue> {
    /// `namei_pool` (`"namei"`): the pathname buffers.
    pub namei: BoxPool<[u8; super::namei::MAXPATHLEN]>,
    /// `vnode_pool`.
    pub vnodes: BoxPool<Vnode<G>>,
    /// `nch_pool` (`"nchpl"`): the name cache's entries.
    pub nch_pool: BoxPool<super::cache::Namecache<G>>,
    /// The name cache (`nclruhead`, `nclruneghead`, the counts), part 0 of
    /// `fs::cache::NCLOCK`.
    pub namecache: Guarded<super::cache::NameCache<G>>,
    /// `nchstats`: cache effectiveness statistics (racy, as the C's).
    pub nchstats: super::cache::Nchstats,
    /// The file systems' operations tables (ids `VopsId::FIRST_FS..`).
    vops: [OnceStatic<&'static dyn Vnops<G>>; (VopsId::MAX - VopsId::FIRST_FS) as usize],
    /// The free and hold lists, at `IPL_BIO`.
    pub lists: Mutexed<FreeLists<G>>,
    /// `mountlist` and the free mounts, a leaf at `IPL_NONE`.
    pub mounts: Mutexed<MountLists<G>>,
    /// `speclisth[SPECHSZ]`: the special device vnodes, hashed by device, a leaf at
    /// `IPL_NONE`.
    pub spechash: Mutexed<[List<BySpecnext<G>>; SPECHSZ]>,
    /// The clone bitmaps ("clonemap").
    pub(super) clonemaps: TypedPool<CloneBits>,
    /// The syncer's wheel, at `IPL_BIO`.
    pub(super) syncer: Mutexed<Wheel<G>>,
    /// `rootvnode`: the root (i.e. "/") vnode.
    pub(super) rootvnode: Link<Vnode<G>>,
    /// The device switch ([`super::spec::Devsw`]).
    pub(super) devsw: OnceStatic<&'static dyn super::spec::Devsw<G>>,
    /// `vfsconflist[]`: the configured file system types.
    pub(super) vfsconf: OnceStatic<&'static [super::mount::Vfsconf<G>]>,
    /// `maxvfsconf`: the highest type number (`vfs.generic.maxtypenum`).
    pub maxvfsconf: AtomicI32,
    /// `numvnodes`: the vnodes allocated so far (`kern.numvnodes`).
    pub numvnodes: AtomicI64,
    /// `maxvnodes`: the number of vnodes `getnewvnode` allocates before it recycles
    /// (`kern.maxvnodes`).
    pub maxvnodes: AtomicI32,
    /// `getnewvnode`'s `toggle`.
    toggle: AtomicI32,
    /// `vfs_stall_lock`.
    pub stall_lock: SplitRwlock,
    /// `prtactive`: 1 => print out reclaim of active vnodes.
    pub prtactive: AtomicI32,
    /// `vfs_getnewfsid`'s `xxxfs_mntid`.
    mntid: AtomicU32,
}

impl<G: ProcGlue> VfsTables<G> {
    /// Empty tables: no vnode, no mount, nothing registered.
    pub const fn new() -> Self {
        Self {
            namei: BoxPool::new(),
            vnodes: BoxPool::new(),
            nch_pool: BoxPool::new(),
            namecache: Guarded::new_static_mutexed(
                &super::cache::NCLOCK,
                super::cache::NameCache::new(),
            ),
            nchstats: super::cache::Nchstats::new(),
            vops: [const { OnceStatic::new() }; (VopsId::MAX - VopsId::FIRST_FS) as usize],
            lists: Mutexed::new(
                IPL_BIO,
                FreeLists {
                    free: List::new(),
                    hold: List::new(),
                },
            ),
            mounts: Mutexed::new(IPL_NONE, MountLists::new()),
            spechash: Mutexed::new(IPL_NONE, [const { List::new() }; SPECHSZ]),
            clonemaps: TypedPool::new(),
            syncer: Mutexed::new(IPL_BIO, Wheel::new()),
            rootvnode: Link::new(),
            devsw: OnceStatic::new(),
            vfsconf: OnceStatic::new(),
            maxvfsconf: AtomicI32::new(0),
            numvnodes: AtomicI64::new(0),
            maxvnodes: AtomicI32::new(0),
            toggle: AtomicI32::new(0),
            stall_lock: SplitRwlock::new("vfs_stall"),
            prtactive: AtomicI32::new(0),
            mntid: AtomicU32::new(0),
        }
    }
}

impl<G: ProcGlue> Default for VfsTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The tables of `G`'s processes.
#[inline]
pub fn vfs<G: ProcGlue>() -> &'static VfsTables<G> {
    <G::FileGlue as FileGlue>::vfs()
}

/// The operations table of `id` (`vp->v_op`).
#[inline]
pub(super) fn vops_table<G: ProcGlue>(id: VopsId) -> &'static dyn Vnops<G> {
    match id {
        VopsId::DEAD => &super::miscfs::deadfs::DeadVops::<G>::TABLE,
        VopsId::SPEC => &super::spec::SpecVops::<G>::TABLE,
        VopsId::SYNC => &super::sync::SyncVops::<G>::TABLE,
        VopsId(n) => match vfs::<G>()
            .vops
            .get(usize::from(n - VopsId::FIRST_FS))
            .and_then(OnceStatic::get)
        {
            Some(t) => *t,
            None => kpanic(format_args!("vnode operations {n}: no table")),
        },
    }
}

/// The id of a file system's operations table, registered on its first use (by the table's
/// address); a table past the registry's size (29 in a kernel; the C has 25) is `panic(9)`.
///
/// The registry is an open-addressed set keyed by the table's address: the search starts at
/// the address's home slot (`vops_home`) and steps to the next slot while it meets other
/// tables, so a table is found in one probe or a few whatever the order the tables came in
/// (the C stores `v_op` without a lookup). Slots are set once and never emptied, so a table
/// registered at a slot past its home finds every slot before it taken on each later search,
/// and two CPUs registering one table meet at the same slot.
pub fn vops_id<G: ProcGlue>(ops: &'static dyn Vnops<G>) -> VopsId {
    let t = vfs::<G>();
    let n = t.vops.len();
    let mut i = vops_home(ops, n);
    for _ in 0..n {
        let slot = &t.vops[i];
        // `i < n <= VopsId::MAX - VopsId::FIRST_FS`, both `u8`s.
        let id = VopsId(VopsId::FIRST_FS + i as u8);
        i = if i + 1 == n { 0 } else { i + 1 };
        match slot.get() {
            Some(o) if ptr::addr_eq(*o, ops) => return id,
            Some(_) => {}
            None => {
                if slot.set(ops).is_ok() {
                    return id;
                }
                // Another CPU took the slot meanwhile: is it this table?
                if slot.get().is_some_and(|o| ptr::addr_eq(*o, ops)) {
                    return id;
                }
            }
        }
    }
    kpanic(format_args!(
        "getnewvnode: more vnode operations tables than {}",
        VopsId::MAX - VopsId::FIRST_FS
    ))
}

/// The registry slot a table's search starts at, of `n`: its address hashed (Fibonacci
/// hashing: bits 32-63 of the address times 2^64 / phi, modulo 2^64), so tables laid out next
/// to each other spread over the slots.
fn vops_home<G: ProcGlue>(ops: &'static dyn Vnops<G>, n: usize) -> usize {
    let a = ptr::from_ref(ops).cast::<()>().addr() as u64;
    let h = a.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 32;
    // `h < 2^32`, so it fits a `usize` on every target the kernel builds for.
    h as usize % n
}

/// `iftovt_tab[]`/`vttoif_tab[]` are `super::vnode`'s.
///
/// Initialize the vnode management data structures (the pools may exist already: a second
/// call, a test, leaves them).
pub fn vntblinit<G: ProcGlue>(initialvnodes: i32) {
    let t = vfs::<G>();
    // buffer cache may need a vnode for each buffer
    t.maxvnodes.store(2 * initialvnodes, Ordering::Relaxed);
    let _ = t.vnodes.init(IPL_NONE, PR_WAITOK, "vnodes");
    let _ = t.clonemaps.init(IPL_NONE, PR_WAITOK, "clonemap");
    // The lists are empty at their making (TAILQ_INIT has nothing to do).

    // Initialize the filesystem syncer.
    sync::vn_initialize_syncerd::<G>();
}

/// `vfs_test_reset`: forgets the lists, the wheel, the counts and `rootvnode` of a test
/// thread's tables, without freeing anything (the memory behind them is gone with the last
/// test's, testing.md).
#[cfg(any(test, feature = "doubles"))]
pub fn vfs_test_reset<G: ProcGlue>() {
    let t = vfs::<G>();
    {
        let mut l = t.lists.lock();
        while l.free.pop_head().is_some() {}
        while l.hold.pop_head().is_some() {}
        let _ = &mut *l;
    }
    {
        let m = t.mounts.lock();
        while m.all.pop_head().is_some() {}
        while m.free.pop_head().is_some() {}
    }
    {
        let c = t.spechash.lock();
        for chain in c.iter() {
            while chain.pop_head().is_some() {}
        }
    }
    sync::syncer_test_reset::<G>();
    super::cache::namecache_test_reset::<G>();
    t.rootvnode.set(None);
    t.numvnodes.store(0, Ordering::Relaxed);
}

/// Lookup a filesystem type, and if found allocate and initialize a mount structure for it.
///
/// Devname is usually updated by mount(8) after booting.
pub fn vfs_rootmountalloc<G: ProcGlue>(
    fstypename: &[u8],
    devname: &[u8],
) -> Result<&'static Mount<G>, Errno> {
    let Some(vfsp) = vfs_byname::<G>(fstypename) else {
        return Err(Errno::ENODEV);
    };
    let mp = vfs_mount_alloc(None, vfsp);
    mp.flag_set(MNT_RDONLY);
    mp.with_stat(|sp| {
        sp.f_mntonname[0] = b'/';
        strlcpy(&mut sp.f_mntfromname, devname);
        strlcpy(&mut sp.f_mntfromspec, devname);
    });
    Ok(mp)
}

/// `strlcpy(dst, src, sizeof(dst))` of a byte string into a fixed name buffer.
fn strlcpy(dst: &mut [u8; MNAMELEN], src: &[u8]) {
    let src = src.split(|&c| c == 0).next().unwrap_or(&[]);
    let n = src.len().min(MNAMELEN - 1);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
}

/// Lookup a mount point by filesystem identifier.
pub fn vfs_getvfs<G: ProcGlue>(fsid: &Fsid) -> Option<&'static Mount<G>> {
    let g = vfs::<G>().mounts.lock();
    g.all.iter().find(|mp| mp.fsid().val == fsid.val)
}

/// Get a new unique fsid.
pub fn vfs_getnewfsid<G: ProcGlue>(mp: &'static Mount<G>) {
    let t = vfs::<G>();
    let mtype = mp.vfc().vfc_typenum;
    let base = devsw::<G>().nblkdev() + mtype as u32;
    mp.with_stat(|sp| {
        sp.f_fsid.val[0] = makedev(base, 0);
        sp.f_fsid.val[1] = mtype;
    });
    if t.mntid.load(Ordering::Relaxed) == 0 {
        t.mntid.fetch_add(1, Ordering::Relaxed);
    }
    let mut tfsid = Fsid {
        val: [makedev(base, t.mntid.load(Ordering::Relaxed)), mtype],
    };
    if !mountlist_is_empty::<G>() {
        while vfs_getvfs::<G>(&tfsid).is_some() {
            tfsid.val[0] = tfsid.val[0].wrapping_add(1);
            t.mntid.fetch_add(1, Ordering::Relaxed);
        }
    }
    mp.with_stat(|sp| sp.f_fsid.val[0] = tfsid.val[0]);
}

/// Set vnode attributes to `VNOVAL`.
pub fn vattr_null(vap: &mut Vattr) {
    vap.va_type = VNON;
    // Don't get fancy: u_quad_t = u_int = VNOVAL leaves the u_quad_t with 2^31-1 instead of
    // 2^64-1. Just write'm out and let the compiler do its job.
    vap.va_mode = VNOVAL as Mode;
    vap.va_nlink = VNOVAL as u32;
    vap.va_uid = VNOVAL as Uid;
    vap.va_gid = VNOVAL as Gid;
    vap.va_fsid = i64::from(VNOVAL);
    vap.va_fileid = VNOVAL as u64;
    vap.va_size = VNOVAL as u64;
    vap.va_blocksize = i64::from(VNOVAL);
    vap.va_atime.tv_sec = i64::from(VNOVAL);
    vap.va_atime.tv_nsec = i64::from(VNOVAL);
    vap.va_mtime.tv_sec = i64::from(VNOVAL);
    vap.va_mtime.tv_nsec = i64::from(VNOVAL);
    vap.va_ctime.tv_sec = i64::from(VNOVAL);
    vap.va_ctime.tv_nsec = i64::from(VNOVAL);
    vap.va_gen = VNOVAL as u64;
    vap.va_flags = VNOVAL as u64;
    vap.va_rdev = VNOVAL;
    vap.va_bytes = VNOVAL as u64;
    vap.va_filerev = VNOVAL as u64;
    vap.va_vaflags = 0;
}

/// Return the next vnode from the free list, with the file system's operations `ops`.
pub fn getnewvnode<G: ProcGlue>(
    tag: Vtagtype,
    mp: Option<&'static Mount<G>>,
    ops: &'static dyn Vnops<G>,
) -> Result<&'static Vnode<G>, Errno> {
    getnewvnode_id(tag, mp, vops_id(ops))
}

/// [`getnewvnode`] with the operations by id (the kernel's own tables: spec, sync).
pub fn getnewvnode_id<G: ProcGlue>(
    tag: Vtagtype,
    mp: Option<&'static Mount<G>>,
    id: VopsId,
) -> Result<&'static Vnode<G>, Errno> {
    let t = vfs::<G>();

    // allow maxvnodes to increase if the buffer cache itself is big enough to justify it.
    // (we don't shrink it ever)
    let maxvnodes = t
        .maxvnodes
        .load(Ordering::Relaxed)
        .max(bio::BCSTATS.numbufs.load(Ordering::Relaxed) as i32);
    t.maxvnodes.store(maxvnodes, Ordering::Relaxed);

    // We must choose whether to allocate a new vnode or recycle an existing one. The
    // criterion for allocating a new one is that the total number of vnodes is less than the
    // number desired or there are no vnodes on either free list. Generally we only want to
    // recycle vnodes that have no buffers associated with them, so we look first on the
    // vnode_free_list. If it is empty, we next consider vnodes with referencing buffers on
    // the vnode_hold_list. The toggle ensures that half the time we will use a buffer from
    // the vnode_hold_list, and half the time we will allocate a new one unless the list has
    // grown to twice the desired size. We are reticent to recycle vnodes from the
    // vnode_hold_list because we will lose the identity of all its referencing buffers.
    let mut toggle = t.toggle.load(Ordering::Relaxed) ^ 1;
    let numvnodes = t.numvnodes.load(Ordering::Relaxed);
    if numvnodes / 2 > i64::from(maxvnodes) {
        toggle = 0;
    }
    t.toggle.store(toggle, Ordering::Relaxed);

    let l = t.lists.lock();
    let vp: &'static Vnode<G> = if numvnodes < i64::from(maxvnodes)
        || (l.free.is_empty() && (l.hold.is_empty() || toggle != 0))
    {
        drop(l);
        let Some(b) = t.vnodes.get(PR_WAITOK, || Vnode::new()) else {
            <G::FileGlue as FileGlue>::tablefull("vnode");
            return Err(Errno::ENFILE);
        };
        let vp: &'static Vnode<G> = PoolBox::leak(b);
        // The vnode's buffer half is the cache's from here (`RBT_INIT` of `v_bufs_tree`: the
        // fresh vnode's tree is empty; the name cache's heads were made with the vnode).
        vp.bufs.set_owner(vp);
        t.numvnodes.fetch_add(1, Ordering::Relaxed);
        vp
    } else {
        let use_hold = l.free.is_empty();
        let listhd = if use_hold { &l.hold } else { &l.free };
        // VOP_ISLOCKED at splbio, as the C asks it: a lock status neither sleeps nor locks.
        let found = listhd.iter().find(|vp| vop_islocked(vp) == 0);
        // Unless this is a bad time of the month, at most the first NCPUS items on the free
        // list are locked, so this is close enough to being empty.
        let Some(vp) = found else {
            drop(l);
            <G::FileGlue as FileGlue>::tablefull("vnode");
            return Err(Errno::ENFILE);
        };

        #[cfg(feature = "diagnostic")]
        if vp.usecount() != 0 {
            drop(l);
            vprint(Some("free vnode"), vp);
            kpanic(format_args!("free vnode isn't"));
        }

        listhd.remove(vp);
        vp.bufs.clr_bioflag(VBIOONFREELIST);
        drop(l);

        if vp.vtype() != VBAD {
            G::with_curproc(|p| vgonel(vp, p));
        }
        #[cfg(feature = "diagnostic")]
        {
            if !vp.data().is_null() {
                vprint(Some("cleaned vnode"), vp);
                kpanic(format_args!("cleaned vnode isn't"));
            }
            if vp.bufs.numoutput() != 0 {
                kpanic(format_args!("Clean vnode has pending I/O's"));
            }
        }
        vp.set_flag(0);
        drop(vp.take_un());
        <G::FileGlue as FileGlue>::vnode_recycle(vp);
        vp
    };
    super::cache::cache_purge(vp);
    vp.set_vtype(VNON);
    vp.set_tag(tag);
    vp.set_ops_id(id);
    insmntque(vp, mp);
    vp.set_usecount(1);
    vp.set_data(ptr::null_mut());
    Ok(vp)
}

/// Move a vnode from one mount queue to another.
pub fn insmntque<G: ProcGlue>(vp: &'static Vnode<G>, mp: Option<&'static Mount<G>>) {
    // Delete from old mount point vnode list, if on one.
    if let Some(old) = vp.mount() {
        old.vnodelist.lock().remove(vp);
    }
    // Insert into list of vnodes for the new mount point, if available.
    vp.set_mount(mp);
    if let Some(mp) = mp {
        mp.vnodelist.lock().insert_tail(vp);
    }
}

/// Create a vnode for a block device. Used for root filesystem, argdev, and swap areas.
/// Also used for memory file system special devices.
pub fn bdevvp<G: ProcGlue>(dev: Dev) -> Result<Option<&'static Vnode<G>>, Errno> {
    getdevvp(dev, VBLK)
}

/// Create a vnode for a character device. Used for console handling.
pub fn cdevvp<G: ProcGlue>(dev: Dev) -> Result<Option<&'static Vnode<G>>, Errno> {
    getdevvp(dev, VCHR)
}

/// Create a vnode for a device. Used by bdevvp (block device) for root file system etc., and
/// by cdevvp (character device) for console.
pub fn getdevvp<G: ProcGlue>(dev: Dev, type_: Vtype) -> Result<Option<&'static Vnode<G>>, Errno> {
    if dev == NODEV {
        return Ok(None);
    }
    let nvp = getnewvnode_id::<G>(VT_NON, None, VopsId::SPEC)?;
    let mut vp = nvp;
    vp.set_vtype(type_);
    if let Some(alias) = checkalias(vp, dev, None) {
        vput(vp);
        vp = alias;
    }
    if vp.vtype() == VCHR && devsw::<G>().cdev_type(major(vp.rdev())) == D_TTY {
        vp.flag_set(VISTTY);
    }
    Ok(Some(vp))
}

/// The first vnode of the chain `bucket` that `f` picks, under the chains' lock.
fn chain_find<G: ProcGlue>(
    bucket: usize,
    f: impl FnMut(&&'static Vnode<G>) -> bool,
) -> Option<&'static Vnode<G>> {
    let chains = vfs::<G>().spechash.lock();
    chains[bucket].iter().find(f)
}

/// Check to see if the new vnode represents a special device for which we already have a
/// vnode (either because of bdevvp() or because of a different vnode representing the same
/// block device). If such an alias exists, deallocate the existing contents and return the
/// aliased vnode. The caller is responsible for filling it with its new contents.
pub fn checkalias<G: ProcGlue>(
    nvp: &'static Vnode<G>,
    nvp_rdev: Dev,
    mp: Option<&'static Mount<G>>,
) -> Option<&'static Vnode<G>> {
    G::with_curproc(|p| {
        let vt = nvp.vtype();
        if vt != VBLK && vt != VCHR {
            return None;
        }

        let bucket = spechash(nvp_rdev);
        let vp = loop {
            // loop: the first alias on the chain, each sleep below from the head again.
            let Some(vp) = chain_find::<G>(bucket, |vp| vp.rdev() == nvp_rdev && vp.vtype() == vt)
            else {
                break None;
            };
            // Alias, but not in use, so flush it out.
            if vp.usecount() == 0 {
                vgonel(vp, p);
                continue;
            }
            let vpid = vp.id();
            if vget(vp, LK_EXCLUSIVE).is_err() {
                continue;
            }
            if vpid != vp.id() {
                vput(vp);
                continue;
            }
            break Some(vp);
        };

        // Common case is actually in the if statement
        match vp {
            Some(vp) if vp.tag() == VT_NON && vp.vtype() == VBLK => {
                // This code is the uncommon case. It is called in case we found an alias that
                // was VT_NON && vtype of VBLK. This means we found a block device that was
                // created using bdevvp. An example of such a vnode is the root partition
                // device vnode created in ffs_mountroot.
                //
                // The vnodes created by bdevvp should not be aliased (why?).
                let _ = vop_unlock(vp);
                vclean(vp, 0, p);
                vp.set_ops_id(nvp.ops_id());
                vp.set_tag(nvp.tag());
                nvp.set_vtype(VNON);
                insmntque(vp, mp);
                Some(vp)
            }
            vp => {
                let t = vfs::<G>();
                let clone = vt == VCHR
                    && devsw::<G>().cdev_flags(major(nvp_rdev)) & D_CLONE != 0
                    && minor(nvp_rdev) >> CLONE_SHIFT == 0;
                let bitmap = if !clone {
                    None
                } else if let Some(vp) = vp {
                    vp.with_specinfo(Specinfo::bitmap).flatten()
                } else {
                    match t.clonemaps.get_shared(PR_WAITOK, || {
                        [const { core::sync::atomic::AtomicU8::new(0) }; super::spec::CLONE_MAPSZ]
                    }) {
                        Some(m) => Some(m),
                        None => kpanic(format_args!("checkalias: out of memory")),
                    }
                };
                let Some(si) =
                    MBox::new(M_VNODE, M_WAITOK, Specinfo::new(nvp_rdev, bucket, bitmap))
                else {
                    kpanic(format_args!("checkalias: out of memory"));
                };
                nvp.set_specinfo(si);
                t.spechash.lock()[bucket].insert_head(nvp);
                if let Some(vp) = vp {
                    nvp.flag_set(VALIASED);
                    vp.flag_set(VALIASED);
                    vput(vp);
                }
                None
            }
        }
    })
}

/// Grab a particular vnode from the free list, increment its reference count and lock it.
/// If the vnode lock bit is set, the vnode is being eliminated in vgone. In that case, we
/// cannot grab it, so the process is awakened when the transition is completed, and an error
/// code is returned to indicate that the vnode is no longer usable, possibly having been
/// changed to a new file system type.
pub fn vget<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    // If the vnode is in the process of being cleaned out for another use, we wait for the
    // cleaning to finish and then return failure. Cleaning is determined by checking that the
    // VXLOCK flag is set.
    {
        let mut g = VNODE_MTX.lock();
        if vp.vlock.get(&g).lflag & VXLOCK != 0 {
            if flags & LK_NOWAIT != 0 {
                return Err(Errno::EBUSY);
            }
            vp.vlock.get_mut(&mut g).lflag |= VXWANT;
            let _ = g.sleep(ptr::from_ref(vp), PINOD, "vget");
            return Err(Errno::ENOENT);
        }
    }

    let onfreelist = {
        let l = vfs::<G>().lists.lock();
        let onfreelist = vp.bufs.bioflag() & VBIOONFREELIST != 0;
        if vp.usecount() == 0 && onfreelist {
            if vp.holdcnt() > 0 {
                l.hold.remove(vp);
            } else {
                l.free.remove(vp);
            }
            vp.bufs.clr_bioflag(VBIOONFREELIST);
        }
        onfreelist
    };

    vp.usecount_inc();
    if flags & LK_TYPE_MASK != 0 {
        let error = vn_lock(vp, flags);
        if error.is_err() && vp.usecount_dec() == 0 && onfreelist {
            vputonfreelist(vp);
        }
        return error;
    }

    Ok(())
}

/// Vnode reference.
pub fn vref<G: ProcGlue>(vp: &'static Vnode<G>) {
    hal::klock::kernel_assert_locked();

    #[cfg(feature = "diagnostic")]
    {
        if vp.usecount() == 0 {
            kpanic(format_args!("vref used where vget required"));
        }
        if vp.vtype() == VNON {
            kpanic(format_args!("vref on a VNON vnode"));
        }
    }
    vp.usecount_inc();
}

/// `vputonfreelist(vp)`: puts an unused vnode on the hold list (if buffers reference it) or
/// the free list; a dead vnode (`VBAD`) goes to the head, to be recycled first.
pub fn vputonfreelist<G: ProcGlue>(vp: &'static Vnode<G>) {
    let l = vfs::<G>().lists.lock();
    vputonfreelist_locked(&l, vp);
}

/// [`vputonfreelist`] under the free lists' lock.
fn vputonfreelist_locked<G: ProcGlue>(l: &ListsGuard<'_, G>, vp: &'static Vnode<G>) {
    #[cfg(feature = "diagnostic")]
    {
        if vp.usecount() != 0 {
            kpanic(format_args!("Use count is not zero!"));
        }

        // If the hold count is still positive, one or many threads could still be waiting
        // on the vnode lock inside uvn_io().
        if vp.holdcnt() == 0 && vp.lockcount() != 0 {
            kpanic(format_args!("vputonfreelist: lock count is not zero"));
        }

        if vp.bufs.bioflag() & VBIOONFREELIST != 0 {
            kpanic(format_args!("vnode {:p} already on free list", vp));
        }
    }

    vp.bufs.set_bioflag(VBIOONFREELIST);
    vp.bufs.clr_bioflag(VBIOERROR);

    let lst = if vp.holdcnt() > 0 { &l.hold } else { &l.free };

    if vp.vtype() == VBAD {
        lst.insert_head(vp);
    } else {
        lst.insert_tail(vp);
    }
}

/// `vput()`, just unlock and `vrele()`.
pub fn vput<G: ProcGlue>(vp: &'static Vnode<G>) {
    #[cfg(feature = "diagnostic")]
    if vp.usecount() == 0 {
        vprint(Some("vput: bad ref count"), vp);
        kpanic(format_args!("vput: ref cnt"));
    }
    let n = vp.usecount_dec();
    kassert!(n > 0 || vp.uvcount() == 0);
    if n > 0 {
        let _ = vop_unlock(vp);
        return;
    }

    #[cfg(feature = "diagnostic")]
    if vp.writecount() != 0 {
        vprint(Some("vput: bad writecount"), vp);
        kpanic(format_args!("vput: v_writecount != 0"));
    }

    let _ = G::with_curproc(|p| vop_inactive(vp, p));

    let l = vfs::<G>().lists.lock();
    if vp.usecount() == 0 && vp.bufs.bioflag() & VBIOONFREELIST == 0 {
        vputonfreelist_locked(&l, vp);
    }
}

/// Vnode release - use for active VNODES. If count drops to zero, call inactive routine and
/// return to freelist. Returns `false` if it did not sleep.
pub fn vrele<G: ProcGlue>(vp: &'static Vnode<G>) -> bool {
    #[cfg(feature = "diagnostic")]
    if vp.usecount() == 0 {
        vprint(Some("vrele: bad ref count"), vp);
        kpanic(format_args!("vrele: ref cnt"));
    }
    if vp.usecount_dec() > 0 {
        return false;
    }

    #[cfg(feature = "diagnostic")]
    if vp.writecount() != 0 {
        vprint(Some("vrele: bad writecount"), vp);
        kpanic(format_args!("vrele: v_writecount != 0"));
    }

    if vn_lock(vp, LK_EXCLUSIVE).is_err() {
        #[cfg(feature = "diagnostic")]
        vprint(Some("vrele: cannot lock"), vp);
        return true;
    }

    let _ = G::with_curproc(|p| vop_inactive(vp, p));

    let l = vfs::<G>().lists.lock();
    if vp.usecount() == 0 && vp.bufs.bioflag() & VBIOONFREELIST == 0 {
        vputonfreelist_locked(&l, vp);
    }
    true
}

/// Page or buffer structure gets a reference.
pub fn vhold<G: ProcGlue>(vp: &'static Vnode<G>) {
    let l = vfs::<G>().lists.lock();

    // If it is on the freelist and the hold count is currently zero, move it to the hold
    // list.
    // The C's test, its unlocked words first (the same answer, no read of the cell for a
    // vnode in use).
    if vp.bufs.bioflag() & VBIOONFREELIST != 0 && vp.usecount() == 0 && vp.holdcnt() == 0 {
        l.free.remove(vp);
        l.hold.insert_tail(vp);
    }
    vp.holdcnt.store(vp.holdcnt() + 1, Ordering::Relaxed);
}

/// Lose interest in a vnode.
pub fn vdrop<G: ProcGlue>(vp: &'static Vnode<G>) {
    let l = vfs::<G>().lists.lock();

    let h = vp.holdcnt();
    if h == 0 {
        kpanic(format_args!("vdrop: zero holdcnt"));
    }
    vp.holdcnt.store(h - 1, Ordering::Relaxed);
    let unheld = h == 1;

    // If it is on the holdlist and the hold count drops to zero, move it to the free list.
    if vp.bufs.bioflag() & VBIOONFREELIST != 0 && unheld && vp.usecount() == 0 {
        l.hold.remove(vp);
        l.free.insert_tail(vp);
    }
}

/// `vfs_mount_foreach_vnode(mp, func, arg)`: calls `func` on every vnode of `mp`, stopping at
/// the first error; restarts when a vnode left the mount under it. The next vnode is read
/// before `func` runs (`TAILQ_FOREACH_SAFE`), unlocked as the C reads it.
pub fn vfs_mount_foreach_vnode<G: ProcGlue>(
    mp: &'static Mount<G>,
    func: &mut dyn FnMut(&'static Vnode<G>) -> Result<(), Errno>,
) -> Result<(), Errno> {
    'restart: loop {
        let mut cur = mp.vnodelist.lock().first();
        while let Some(vp) = cur {
            if !vp.mount().is_some_and(|m| ptr::eq(m, mp)) {
                continue 'restart;
            }
            cur = List::<super::mount::ByMntvnodes<G>>::next(vp);
            func(vp)?;
        }
        return Ok(());
    }
}

/// `struct vflush_args`.
struct VflushArgs<G: ProcGlue> {
    skipvp: Option<&'static Vnode<G>>,
    busy: i32,
    flags: i32,
}

/// `vflush_vnode(vp, arg)`: one vnode of `vflush`.
fn vflush_vnode<G: ProcGlue>(vp: &'static Vnode<G>, va: &mut VflushArgs<G>, p: Option<&Thread<G>>) {
    if va.skipvp.is_some_and(|skip| ptr::eq(skip, vp)) {
        return;
    }

    if va.flags & SKIPSYSTEM != 0 && vp.flag() & VSYSTEM != 0 {
        return;
    }

    // If WRITECLOSE is set, only flush out regular file vnodes open for writing.
    if va.flags & WRITECLOSE != 0 && (vp.writecount() == 0 || vp.vtype() != VREG) {
        return;
    }

    // With v_usecount == 0, all we need to do is clear out the vnode data structures and we
    // are done.
    if vp.usecount() == 0 {
        vgonel(vp, p);
        return;
    }

    // If FORCECLOSE is set, forcibly close the vnode. For block or character devices,
    // revert to an anonymous device. For all other files, just kill them.
    if va.flags & FORCECLOSE != 0 {
        if vp.vtype() != VBLK && vp.vtype() != VCHR {
            vgonel(vp, p);
        } else {
            vclean(vp, 0, p);
            vp.set_ops_id(VopsId::SPEC);
            insmntque(vp, None);
        }
        return;
    }

    // If set, this is allowed to ignore vnodes which don't have changes pending to disk.
    // XXX Might be nice to check per-fs "inode" flags, but generally the filesystem is sync'd
    // already, right?
    if va.flags & IGNORECLEAN != 0 && vp.bufs.dirty_is_empty() {
        return;
    }

    // DEBUG_SYSCTL busyprt: not configured.
    va.busy += 1;
}

/// Remove any vnodes in the vnode table belonging to mount point `mp`.
///
/// If `MNT_NOFORCE` is specified, there should not be any active ones, return error if any
/// are found (nb: this is a user error, not a system error). If `MNT_FORCE` is specified,
/// detach any active vnodes that are found.
pub fn vflush<G: ProcGlue>(
    mp: &'static Mount<G>,
    skipvp: Option<&'static Vnode<G>>,
    flags: i32,
) -> Result<(), Errno> {
    let mut va = VflushArgs {
        skipvp,
        busy: 0,
        flags,
    };

    G::with_curproc(|p| {
        let _ = vfs_mount_foreach_vnode(mp, &mut |vp| {
            vflush_vnode(vp, &mut va, p);
            Ok(())
        });
    });

    if va.busy != 0 {
        return Err(Errno::EBUSY);
    }
    Ok(())
}

/// Disassociate the underlying file system from a vnode.
pub fn vclean<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32, p: Option<&Thread<G>>) {
    let mut do_wakeup = false;

    // Check to see if the vnode is in use. If so we have to reference it before we clean it
    // out so that its count cannot fall to zero and generate a race against ourselves to
    // recycle it.
    let active = vp.usecount();
    if active != 0 {
        vp.set_usecount(active + 1);
    }

    // Prevent the vnode from being recycled or brought into use while we clean it out.
    {
        let mut g = VNODE_MTX.lock();
        if vp.vlock.get(&g).lflag & VXLOCK != 0 {
            kpanic(format_args!("vclean: deadlock"));
        }
        vp.vlock.get_mut(&mut g).lflag |= VXLOCK;

        if vp.vlock.get(&g).lockcount > 0 {
            // Ensure that any thread currently waiting on the same lock has observed that the
            // vnode is about to be exclusively locked before continuing.
            let _ = g.sleep(ptr::from_ref(&vp.vlock), PINOD, "vop_lock");
            kassert!(vp.vlock.get(&g).lockcount == 0);
        }
    }

    // Even if the count is zero, the VOP_INACTIVE routine may still have the object locked
    // while it cleans it out. The VOP_LOCK ensures that the VOP_INACTIVE routine is done with
    // its work. For active vnodes, it ensures that no other activity can occur while the
    // underlying object is being cleaned out.
    let _ = vop_lock(vp, LK_EXCLUSIVE | LK_DRAIN);

    // Clean out any VM data associated with the vnode.
    crate::vm::vnode::uvm_vnp_terminate(vp);
    // Clean out any buffers associated with the vnode.
    if flags & DOCLOSE != 0
        && let Err(error) = vinvalbuf(vp, V_SAVE, Cred::NOCRED, p, 0, INFSLP)
    {
        crate::prf::printf(format_args!(
            "vclean: failed to flush buffers, error {}; discarding dirty buffers",
            error as i32
        ));
        if let Some(mp) = vp.mount() {
            let (name, len) = mp.mntonname();
            crate::prf::printf(format_args!(
                "; mounted on: {}",
                core::str::from_utf8(&name[..len]).unwrap_or("?")
            ));
        }
        crate::prf::printf(format_args!("\n"));
        let _ = vinvalbuf(vp, 0, Cred::NOCRED, p, 0, INFSLP);
    }
    // If purging an active vnode, it must be closed and deactivated before being reclaimed.
    // Note that the VOP_INACTIVE will unlock the vnode
    if active != 0 {
        if flags & DOCLOSE != 0 {
            let _ = vop_close(vp, hal::sys::fcntl::FNONBLOCK, Cred::NOCRED, p);
        }
        let _ = vop_inactive(vp, p);
    } else {
        // Any other processes trying to obtain this lock must first wait for VXLOCK to
        // clear, then call the new lock operation.
        let _ = vop_unlock(vp);
    }

    // Reclaim the vnode.
    if vop_reclaim(vp, p).is_err() {
        kpanic(format_args!("vclean: cannot reclaim"));
    }
    if active != 0 && vp.usecount_dec() == 0 {
        let l = vfs::<G>().lists.lock();
        if vp.holdcnt() > 0 {
            kpanic(format_args!("vclean: not clean"));
        }
        vputonfreelist_locked(&l, vp);
    }
    super::cache::cache_purge(vp);

    // Done with purge, notify sleepers of the grim news.
    vp.set_ops_id(VopsId::DEAD);
    VN_KNOTE(vp, crate::event::NOTE_REVOKE);
    vp.set_tag(VT_NON);
    // VFSLCKDEBUG: not configured.
    {
        let mut g = VNODE_MTX.lock();
        let vl = vp.vlock.get_mut(&mut g);
        vl.lflag &= !VXLOCK;
        if vl.lflag & VXWANT != 0 {
            vl.lflag &= !VXWANT;
            do_wakeup = true;
        }
    }
    if do_wakeup {
        hal::sleep::wakeup(ptr::from_ref(vp));
    }
}

/// Recycle an unused vnode to the front of the free list.
pub fn vrecycle<G: ProcGlue>(vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> bool {
    if vp.usecount() == 0 {
        vgonel(vp, p);
        return true;
    }
    false
}

/// Eliminate all activity associated with a vnode in preparation for reuse.
pub fn vgone<G: ProcGlue>(vp: &'static Vnode<G>) {
    G::with_curproc(|p| vgonel(vp, p));
}

/// vgone, with struct proc.
pub fn vgonel<G: ProcGlue>(vp: &'static Vnode<G>, p: Option<&Thread<G>>) {
    kassert!(vp.uvcount() == 0);

    // If a vgone (or vclean) is already in progress, wait until it is done and return.
    {
        let mut g = VNODE_MTX.lock();
        if vp.vlock.get(&g).lflag & VXLOCK != 0 {
            vp.vlock.get_mut(&mut g).lflag |= VXWANT;
            let _ = g.sleep(ptr::from_ref(vp), PINOD, "vgone");
            return;
        }
    }

    // Clean out the filesystem specific data.
    vclean(vp, DOCLOSE, p);
    // Delete from old mount point vnode list, if on one.
    if vp.mount().is_some() {
        insmntque(vp, None);
    }
    // If special device, remove it from special device alias list if it is on one.
    if (vp.vtype() == VBLK || vp.vtype() == VCHR)
        && let Some((rdev, bucket)) = vp.with_specinfo(|si| (si.rdev(), si.hashchain()))
    {
        // The clone bitmap goes with its last holder (the C frees it with the alias that is
        // not VALIASED any more): this specinfo's handle drops below.
        if let Some(bucket) = bucket {
            let chains = vfs::<G>().spechash.lock();
            let chain = &chains[bucket];
            chain.remove(vp);
            let _ = vp.with_specinfo(Specinfo::clear_hashchain);
            if vp.flag() & VALIASED != 0 {
                let vt = vp.vtype();
                let mut vx: Option<&'static Vnode<G>> = None;
                let mut more = false;
                for vq in chain {
                    if vq.rdev() != rdev || vq.vtype() != vt {
                        continue;
                    }
                    if vx.is_some() {
                        more = true;
                        break;
                    }
                    vx = Some(vq);
                }
                let Some(vx) = vx else {
                    drop(chains);
                    kpanic(format_args!("missing alias"));
                };
                if !more {
                    vx.flag_clr(VALIASED);
                }
                vp.flag_clr(VALIASED);
            }
        }
        super::lockf::lf_purgelocks::<G>(&vp.speclockf);
        drop(vp.take_un());
    }
    // If it is on the freelist and not already at the head, move it to the head of the list.
    vp.set_vtype(VBAD);

    // Move onto the free list, unless we were called from getnewvnode and we're not on any
    // free list
    let l = vfs::<G>().lists.lock();
    if vp.usecount() == 0 && vp.bufs.bioflag() & VBIOONFREELIST != 0 {
        if vp.holdcnt() > 0 {
            kpanic(format_args!("vgonel: not clean"));
        }

        if !l.free.first().is_some_and(|f| ptr::eq(f, vp)) {
            l.free.remove(vp);
            l.free.insert_head(vp);
        }
    }
}

/// Lookup a vnode by device number.
pub fn vfinddev<G: ProcGlue>(dev: Dev, type_: Vtype) -> Option<&'static Vnode<G>> {
    chain_find::<G>(spechash(dev), |vp| dev == vp.rdev() && type_ == vp.vtype())
}

/// Revoke all the vnodes corresponding to the specified minor number range (endpoints
/// inclusive) of the specified major.
pub fn vdevgone<G: ProcGlue>(maj: u32, minl: u32, minh: u32, type_: Vtype) {
    for mn in minl..=minh {
        if let Some(vp) = vfinddev::<G>(makedev(maj, mn), type_) {
            let _ = vop_revoke(vp, REVOKEALL);
        }
    }
}

/// Calculate the total number of references to a special device.
pub fn vcount<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    loop {
        // loop:
        if vp.flag() & VALIASED == 0 {
            return vp.usecount() as i32;
        }
        let Some(bucket) = vp.with_specinfo(Specinfo::hashchain).flatten() else {
            return vp.usecount() as i32;
        };
        let (rdev, vt) = (vp.rdev(), vp.vtype());
        let (count, gone) = {
            let chains = vfs::<G>().spechash.lock();
            let mut count = 0;
            let mut gone = None;
            for vq in &chains[bucket] {
                if vq.rdev() != rdev || vq.vtype() != vt {
                    continue;
                }
                // Alias, but not in use, so flush it out.
                if vq.usecount() == 0 && !ptr::eq(vq, vp) {
                    gone = Some(vq);
                    break;
                }
                count += vq.usecount() as i32;
            }
            (count, gone)
        };
        match gone {
            Some(vq) => vgone(vq),
            None => return count,
        }
    }
}

/// Print out a description of a vnode.
#[cfg(any(feature = "debug", feature = "diagnostic"))]
pub fn vprint<G: ProcGlue>(label: Option<&str>, vp: &'static Vnode<G>) {
    use super::vnode::{VBIOONSYNCLIST, VBIOWAIT, VTEXT, VTYPE_NAMES};
    use crate::prf::printf;

    if let Some(label) = label {
        printf(format_args!("{}: ", label));
    }
    printf(format_args!(
        "{:p}, type {}, use {}, write {}, hold {},",
        vp,
        VTYPE_NAMES[vp.vtype() as usize],
        vp.usecount(),
        vp.writecount(),
        vp.holdcnt()
    ));
    let lflag = vp.lflag();
    let flags: [(bool, &str); 9] = [
        (vp.flag() & VROOT != 0, "VROOT"),
        (vp.flag() & VTEXT != 0, "VTEXT"),
        (vp.flag() & VSYSTEM != 0, "VSYSTEM"),
        (lflag & VXLOCK != 0, "VXLOCK"),
        (lflag & VXWANT != 0, "VXWANT"),
        (vp.bufs.bioflag() & VBIOWAIT != 0, "VBIOWAIT"),
        (vp.bufs.bioflag() & VBIOONFREELIST != 0, "VBIOONFREELIST"),
        (vp.bufs.bioflag() & VBIOONSYNCLIST != 0, "VBIOONSYNCLIST"),
        (vp.flag() & VALIASED != 0, "VALIASED"),
    ];
    let mut first = true;
    for (set, name) in flags {
        if set {
            printf(format_args!(
                "{}{}",
                if first { " flags (" } else { "|" },
                name
            ));
            first = false;
        }
    }
    if !first {
        printf(format_args!(")"));
    }
    if vp.data().is_null() {
        printf(format_args!("\n"));
    } else {
        printf(format_args!("\n\t"));
        let _ = super::vops::vop_print(vp);
    }
}

/// List all of the locked vnodes in the system. Called when debugging the kernel.
#[cfg(feature = "debug")]
pub fn printlockedvnodes<G: ProcGlue>() {
    use hal::sys::mount::{VB_NOWAIT, VB_READ};
    crate::prf::printf(format_args!("Locked vnodes\n"));

    for mp in super::mount::mountlist::<G>(false) {
        if vfs_busy(mp, VB_READ | VB_NOWAIT).is_err() {
            continue;
        }
        let _ = vfs_mount_foreach_vnode(mp, &mut |vp| {
            if vop_islocked(vp) != 0 {
                vprint(None, vp);
            }
            Ok(())
        });
        vfs_unbusy(mp);
    }
}

/// `vfs_sysctl`: the `CTL_VFS` tree (`vfs.generic.*`, and each type's `vfs.<type>` node
/// through its operations), registered with the kernel's sysctl dispatcher.
pub struct VfsSysctl<G: ProcGlue>(core::marker::PhantomData<G>);

impl<G: ProcGlue> VfsSysctl<G> {
    /// The node.
    pub const NODE: VfsSysctl<G> = VfsSysctl(core::marker::PhantomData);
}

impl<G: ProcGlue> crate::sysctl::SysctlNode for VfsSysctl<G> {
    fn sysctl(&self, name: &[i32], req: &mut crate::sysctl::SysctlReq<'_>) -> Result<(), Errno> {
        use crate::sysctl::{rdint, rdstruct};
        use hal::sys::mount::{VFS_BCACHESTAT, VFS_CONF, VFS_GENERIC, VFS_MAXTYPENUM, VfsconfAbi};

        // all sysctl names at this level are at least name and field
        if name.len() < 2 {
            return Err(Errno::ENOTDIR); // overloaded
        }

        if name[0] != VFS_GENERIC {
            let Some(vfsp) = super::init::vfs_bytypenum::<G>(name[0]) else {
                return Err(Errno::EOPNOTSUPP);
            };
            return G::with_curproc(|p| match p {
                Some(p) => vfsp.vfc_vfsops.sysctl(
                    &name[1..],
                    req.oldp,
                    req.oldlenp,
                    req.newp,
                    req.newlen,
                    p,
                ),
                None => Err(Errno::EOPNOTSUPP),
            });
        }

        match name[1] {
            VFS_MAXTYPENUM => rdint(req, vfs::<G>().maxvfsconf.load(Ordering::Relaxed)),
            VFS_CONF => {
                if name.len() < 3 {
                    return Err(Errno::ENOTDIR); // overloaded
                }

                let Some(vfsp) = super::init::vfs_bytypenum::<G>(name[2]) else {
                    return Err(Errno::EOPNOTSUPP);
                };

                // Make a copy, clear out kernel pointers
                let tmpvfsp = VfsconfAbi {
                    vfc_vfsops: 0,
                    vfc_name: vfsp.vfc_name,
                    vfc_typenum: vfsp.vfc_typenum,
                    vfc_refcount: vfsp.vfc_refcount.load(Ordering::Relaxed),
                    vfc_flags: vfsp.vfc_flags,
                    _pad: 0,
                    vfc_datasize: vfsp.vfc_datasize as u64,
                };

                rdstruct(req, hal::user::bytes_of(core::slice::from_ref(&tmpvfsp)))
            }
            // buffer cache statistics
            VFS_BCACHESTAT => {
                let bcstats = bio::BCSTATS.snapshot();
                rdstruct(req, hal::user::bytes_of(core::slice::from_ref(&bcstats)))
            }
            _ => Err(Errno::EOPNOTSUPP),
        }
    }
}

/// Check to see if a filesystem is mounted on a block device.
pub fn vfs_mountedon<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if vp.specmountpoint().is_some() {
        return Err(Errno::EBUSY);
    }
    if vp.flag() & VALIASED != 0
        && let Some(bucket) = vp.with_specinfo(Specinfo::hashchain).flatten()
    {
        let (rdev, vt) = (vp.rdev(), vp.vtype());
        let busy = chain_find::<G>(bucket, |vq| {
            vq.rdev() == rdev && vq.vtype() == vt && vq.specmountpoint().is_some()
        });
        if busy.is_some() {
            return Err(Errno::EBUSY);
        }
    }
    Ok(())
}

/// `vaccess(type, file_mode, uid, gid, acc_mode, cred)`: the UNIX permission check (the
/// superuser reads and writes anything and executes what has an execute bit, a directory
/// always; then the owner's bits, the group's, the others').
pub fn vaccess(
    type_: Vtype,
    file_mode: Mode,
    uid: Uid,
    gid: Gid,
    acc_mode: i32,
    cred: &Ucred,
) -> Result<(), Errno> {
    use stat::{S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR, S_IXGRP, S_IXOTH, S_IXUSR};

    // User id 0 always gets read/write access.
    if cred.cr_uid.get() == 0 {
        // For VEXEC, at least one of the execute bits must be set.
        if acc_mode & VEXEC != 0 && type_ != VDIR && file_mode & (S_IXUSR | S_IXGRP | S_IXOTH) == 0
        {
            return Err(Errno::EACCES);
        }
        return Ok(());
    }

    let check = |mask: Mode| {
        if file_mode & mask == mask {
            Ok(())
        } else {
            Err(Errno::EACCES)
        }
    };
    let bits = |x: Mode, r: Mode, w: Mode| {
        let mut mask: Mode = 0;
        if acc_mode & VEXEC != 0 {
            mask |= x;
        }
        if acc_mode & VREAD != 0 {
            mask |= r;
        }
        if acc_mode & VWRITE != 0 {
            mask |= w;
        }
        mask
    };

    // Otherwise, check the owner.
    if cred.cr_uid.get() == uid {
        return check(bits(S_IXUSR, S_IRUSR, S_IWUSR));
    }

    // Otherwise, check the groups.
    if groupmember(gid, cred) {
        return check(bits(S_IXGRP, S_IRGRP, S_IWGRP));
    }

    // Otherwise, check everyone else.
    check(bits(S_IXOTH, S_IROTH, S_IWOTH))
}

/// `vnoperm(vp)`: whether permission checks are off for the vnode's file system
/// (`MNT_NOPERM`); never for a file system root or a vnode without a mount.
pub fn vnoperm<G: ProcGlue>(vp: &'static Vnode<G>) -> bool {
    if vp.flag() & VROOT != 0 {
        return false;
    }
    match vp.mount() {
        None => false,
        Some(mp) => mp.flag() & MNT_NOPERM != 0,
    }
}

/// `vfs_stall(p, stall)`: syncs and busies every file system (suspend), or releases them.
pub fn vfs_stall<G: ProcGlue>(p: &Thread<G>, stall: bool) -> Result<(), Errno> {
    let t = vfs::<G>();
    let mut allerror = Ok(());

    if stall {
        VFS_STALLING.fetch_add(1, Ordering::SeqCst);
        let _ = t.stall_lock.enter(RW_WRITE);
    }

    // The loop variable mp is protected by vfs_busy() so that it cannot be unmounted while
    // VFS_SYNC() sleeps. Traverse forward to keep the lock order consistent with dounmount().
    // The successor is read before the body, and one unmounted while the body slept restarts
    // the walk past the mounts this pass already handled (marked MNT_STALLED when stalling,
    // cleared when resuming).
    let done = if stall { MNT_STALLED } else { 0 };
    for mp in mountlist_safe_restart::<G>(false, MNT_STALLED, done) {
        let (name, len) = mp.mntonname();
        let name = core::str::from_utf8(&name[..len]).unwrap_or("?");
        if stall {
            if let Err(error) = vfs_busy(mp, VB_WRITE | VB_WAIT | VB_DUPOK) {
                crate::prf::printf(format_args!("{name}: busy\n"));
                allerror = Err(error);
                continue;
            }
            crate::vm::vnode::uvm_vnp_sync(Some(mp.addr()));
            if let Err(error) = VFS_SYNC(mp, MNT_WAIT, 1, Cred::real(p.ucred()), p) {
                crate::prf::printf(format_args!("{name}: failed to sync\n"));
                vfs_unbusy(mp);
                allerror = Err(error);
                continue;
            }
            mp.flag_set(MNT_STALLED);
        } else if mp.flag() & MNT_STALLED != 0 {
            vfs_unbusy(mp);
            mp.flag_clr(MNT_STALLED);
        }
    }

    if !stall {
        t.stall_lock.exit();
        VFS_STALLING.fetch_sub(1, Ordering::SeqCst);
    }

    allerror
}

/// `vfs_stall_barrier()`: waits while the file systems are stalled.
pub fn vfs_stall_barrier<G: ProcGlue>() {
    if VFS_STALLING.load(Ordering::Relaxed) != 0 {
        let t = vfs::<G>();
        let _ = t.stall_lock.enter(RW_READ);
        t.stall_lock.exit();
    }
}

/// Wait for all outstanding I/Os to complete (`v_numoutput`; the cache's lock is its
/// interlock).
pub fn vwaitforio<G: ProcGlue>(
    vp: &'static Vnode<G>,
    slpflag: i32,
    wmesg: &'static str,
    timeo: u64,
) -> Result<(), Errno> {
    bio::vwaitforio(&vp.bufs, slpflag, wmesg, timeo)
}

/// Flush out and invalidate all buffers associated with a vnode. Called with the underlying
/// object locked.
pub fn vinvalbuf<G: ProcGlue>(
    vp: &'static Vnode<G>,
    flags: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
    slpflag: i32,
    slptimeo: u64,
) -> Result<(), Errno> {
    // VFSLCKDEBUG: not configured.

    if flags & V_SAVE != 0 {
        let _ = vwaitforio(vp, 0, "vinvalbuf", INFSLP);
        if !vp.bufs.dirty_is_empty() {
            let Some(p) = p else {
                kpanic(format_args!(
                    "vinvalbuf: dirty buffers and no thread to sync them"
                ));
            };
            vop_fsync(vp, cred, MNT_WAIT, p)?;
            if vp.bufs.numoutput() > 0 || !vp.bufs.dirty_is_empty() {
                kpanic(format_args!("vinvalbuf: dirty bufs, vp {vp:p}"));
            }
        }
    }
    // The buffer lists' loop is the cache's (it takes, writes and frees the buffers).
    bio::vinvalbuf_bufs(
        &vp.bufs,
        flags & V_SAVE != 0,
        flags & V_SAVEMETA != 0,
        slpflag,
        slptimeo,
    )
}

/// `vflushbuf(vp, sync)`: writes the vnode's dirty buffers out, and with `sync` waits for
/// them.
pub fn vflushbuf<G: ProcGlue>(vp: &'static Vnode<G>, sync: bool) {
    loop {
        // loop:
        let pick = |bp: &Buf| {
            if bp.isset(B_BUSY) {
                return DirtyPick::Skip;
            }
            if !bp.isset(B_DELWRI) {
                kpanic(format_args!("vflushbuf: not dirty"));
            }
            DirtyPick::Take
        };
        if let DirtyScan::Taken(t) = bio::vnode_dirty_scan(&vp.bufs, "vflushbuf", 0, INFSLP, pick) {
            // Wait for I/O associated with indirect blocks to complete, since there is no way
            // to quickly wait for them below.
            if t.buf()
                .vnode_bufs()
                .is_some_and(|vb| ptr::eq(vb, &raw const vp.bufs))
                || !sync
            {
                bio::bawrite(t);
            } else {
                let _ = bio::bwrite(t);
            }
            continue;
        }
        if !sync {
            return;
        }
        let _ = vwaitforio(vp, 0, "vflushbuf", INFSLP);
        if !vp.bufs.dirty_is_empty() {
            #[cfg(feature = "diagnostic")]
            vprint(Some("vflushbuf: dirty"), vp);
            continue;
        }
        return;
    }
}

/// `copy_statfs_info(sbp, mp)`: fills the mount-wide members of a file system's `statfs`
/// answer from the mount.
pub fn copy_statfs_info<G: ProcGlue>(sbp: &mut Statfs, mp: &Mount<G>) {
    let name = mp.vfc().name();
    sbp.f_fstypename = [0; MFSNAMELEN];
    sbp.f_fstypename[..name.len()].copy_from_slice(name);

    // sbp == &mp->mnt_stat never happens here: the caller's is a copy (LZ's deviation); the
    // copy has the same values, so the result is the same.
    let mbp = mp.stat();
    sbp.f_fsid = mbp.f_fsid;
    sbp.f_owner = mbp.f_owner;
    sbp.f_flags = mbp.f_flags;
    sbp.f_syncwrites = mbp.f_syncwrites;
    sbp.f_asyncwrites = mbp.f_asyncwrites;
    sbp.f_syncreads = mbp.f_syncreads;
    sbp.f_asyncreads = mbp.f_asyncreads;
    sbp.f_namemax = mbp.f_namemax;
    sbp.f_mntonname = mbp.f_mntonname;
    sbp.f_mntfromname = mbp.f_mntfromname;
    sbp.f_mntfromspec = mbp.f_mntfromspec;
    sbp.mount_info = mbp.mount_info;
}

/// The mount a vnode's I/O is charged to: a block device's `v_specmountpoint`, else
/// `v_mount` (`vfs_bio.c`'s reads and writes).
fn io_mount<G: ProcGlue>(vp: &Vnode<G>) -> Option<&'static Mount<G>> {
    if vp.vtype() == VBLK {
        vp.specmountpoint()
    } else {
        vp.mount()
    }
}

// The vnode as the buffer cache reaches it: one call each into the vnode layer. Vnodes are
// never freed, so the cache keeps them as `&'static`.
impl<G: ProcGlue> BufVnode for Vnode<G> {
    fn vhold(&'static self) {
        vhold(self);
    }

    fn vdrop(&'static self) {
        vdrop(self);
    }

    fn strategy(&'static self, bp: BufRef) -> Result<(), Errno> {
        vop_strategy(self, IoBuf::Token(bp))
    }

    fn bwrite(&'static self, bp: BufRef) -> Result<(), Errno> {
        vop_bwrite::<G>(IoBuf::Token(bp))
    }

    fn bmap(&'static self, bn: Daddr) -> Result<(Daddr, i32), Errno> {
        let mut bnp = 0;
        let mut runp = 0;
        vop_bmap(self, bn, None, Some(&mut bnp), Some(&mut runp))?;
        Ok((bnp, runp))
    }

    fn vtype(&self) -> VType {
        Vnode::vtype(self)
    }

    fn rdev(&self) -> Dev {
        Vnode::rdev(self)
    }

    fn mnt_async(&self) -> bool {
        io_mount(self).is_some_and(|mp| mp.flag() & MNT_ASYNC != 0)
    }

    fn count_io(&self, write: bool, async_: bool) {
        // Collect statistics on synchronous and asynchronous reads and writes. Block devices
        // are charged to their associated filesystem (if any).
        if let Some(mp) = io_mount(self) {
            mp.count_io(write, async_);
        }
    }

    fn charge_io(&self, output: bool) {
        // Pay for the I/O operation.
        G::with_curproc(|p| {
            if let Some(p) = p {
                let w = if output {
                    &p.p_ru.ru_oublock
                } else {
                    &p.p_ru.ru_inblock // XXX
                };
                w.set(w.get() + 1);
            }
        });
    }

    fn syncer_add_dirty(&'static self) {
        let syncdelay = SYNCDELAY.load(Ordering::Relaxed);
        let delay = match self.vtype() {
            VDIR => syncdelay / 2,
            VBLK if self.specmountpoint().is_some() => syncdelay / 3,
            _ => syncdelay,
        };
        vn_syncer_add_to_worklist(self, delay);
    }

    fn syncer_remove(&'static self) {
        sync::syncer_remove(self);
    }

    fn as_any(&'static self) -> &'static dyn Any {
        self
    }
}

/// `VOP_READ` or `VOP_WRITE` of `buf` at `off` over a `UIO_SYSSPACE` uio with `curproc`'s
/// credentials (the pager's I/O): the bytes moved.
fn vop_rw_kernel<G: ProcGlue>(
    vp: &'static Vnode<G>,
    write: Option<&[u8]>,
    read: Option<&mut [u8]>,
    off: Voff,
    ioflag: i32,
) -> Result<usize, Errno> {
    G::with_curproc(|p| {
        let cred = p.map_or(Cred::NULL, |p| Cred::real(p.ucred()));
        let mut iov = [Iovec::new()];
        match (write, read) {
            (Some(buf), _) => {
                let len = buf.len();
                let mut uio = Uio::kernel_write(&mut iov, buf, p);
                uio.uio_offset = off;
                vop_write(vp, &mut uio, ioflag, cred)?;
                Ok(len - uio.uio_resid)
            }
            (None, Some(buf)) => {
                let len = buf.len();
                let mut uio = Uio::kernel_read(&mut iov, buf, p);
                uio.uio_offset = off;
                vop_read(vp, &mut uio, ioflag, cred)?;
                Ok(len - uio.uio_resid)
            }
            (None, None) => Ok(0),
        }
    })
}

/// `VOP_GETATTR(vp, &vattr, p->p_ucred, p)` for `curproc`.
fn getattr_cur<G: ProcGlue>(vp: &'static Vnode<G>, who: &str) -> Result<Vattr, Errno> {
    G::with_curproc(|p| {
        let Some(p) = p else {
            kpanic(format_args!("{who}: no curproc"));
        };
        let mut vattr = Vattr::new();
        vop_getattr(vp, &mut vattr, Cred::real(p.ucred()), p)?;
        Ok(vattr)
    })
}

// The vnode as the vnode pager reaches it: one call each into the vnode layer. Vnodes are
// never freed, so the pager keeps them as `&'static`.
impl<G: ProcGlue> VnodeOps for Vnode<G> {
    fn uvn(&self) -> Option<&'static Uvn> {
        Vnode::uvn(self)
    }

    fn set_uvn(&self, uvn: &'static Uvn) {
        Vnode::set_uvn(self, uvn);
    }

    fn attr_size(&'static self) -> Result<u64, Errno> {
        Ok(getattr_cur(self, "uvn_attach")?.va_size)
    }

    fn is_blk(&self) -> bool {
        Vnode::vtype(self) == VBLK
    }

    fn vtext(&self) -> bool {
        self.flag() & super::vnode::VTEXT != 0
    }

    fn clear_vtext(&self) {
        self.flag_clr(super::vnode::VTEXT);
    }

    fn mount_id(&self) -> usize {
        self.mount().map_or(0, super::mount::Mount::addr)
    }

    fn vref(&'static self) {
        vref(self);
    }

    fn vrele(&'static self) {
        let _ = vrele(self);
    }

    fn vget_nowait(&'static self) -> Result<(), Errno> {
        vget(self, LK_NOWAIT)
    }

    fn lock(&'static self, flags: i32) -> Result<(), Errno> {
        vn_lock(self, flags)
    }

    fn unlock(&'static self) {
        let _ = vop_unlock(self);
    }

    fn read(&'static self, off: Voff, buf: &mut [u8], ioflag: i32) -> Result<usize, Errno> {
        vop_rw_kernel(self, None, Some(buf), off, ioflag)
    }

    fn write(&'static self, off: Voff, buf: &[u8], ioflag: i32) -> Result<usize, Errno> {
        vop_rw_kernel(self, Some(buf), None, off, ioflag)
    }

    fn vtype(&self) -> VType {
        Vnode::vtype(self)
    }

    fn is_zero_device(&self) -> bool {
        devsw::<G>().iszerodev(Vnode::rdev(self))
    }

    fn attr_flags(&'static self) -> Result<u64, Errno> {
        Ok(getattr_cur(self, "mmap")?.va_flags)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use super::*;
    use crate::fs::mount::{vfs_isbusy, vfs_mount_free, vfs_mount_take};
    use crate::fs::sync::{SyncVops, vfs_allocate_syncvnode};
    use crate::fs::testfs::{
        self, B, C, INACTIVE, RECLAIM, ROOT, TEST_CONF, TEST_VOPS, TestGlue, UNLOCKS, setup_root,
    };
    use crate::fs::vnode::{VBIOONSYNCLIST, VnodeKind};
    use hal::sys::mount::{MNT_UNMOUNT, VB_NOWAIT, VB_READ};

    type V = Vnode<TestGlue>;

    fn on_free(vp: &'static V) -> bool {
        vfs::<TestGlue>()
            .lists
            .lock()
            .free
            .iter()
            .any(|v| ptr::eq(v, vp))
    }

    fn on_hold(vp: &'static V) -> bool {
        vfs::<TestGlue>()
            .lists
            .lock()
            .hold
            .iter()
            .any(|v| ptr::eq(v, vp))
    }

    fn on_mount(mp: &'static Mount<TestGlue>, vp: &'static V) -> bool {
        mp.vnodelist.lock().iter().any(|v| ptr::eq(v, vp))
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
        let (_g, mp) = setup_root();
        let t = vfs::<TestGlue>();
        let before = t.numvnodes.load(Ordering::SeqCst);
        let vp = getnewvnode(VT_NON, Some(mp), &TEST_VOPS).unwrap();
        assert_eq!(vp.usecount(), 1);
        assert_eq!(vp.vtype(), VNON);
        assert!(vp.mount().is_some_and(|m| ptr::eq(m, mp)));
        assert!(on_mount(mp, vp));
        assert_eq!(t.numvnodes.load(Ordering::SeqCst), before + 1);
        // The table is a file system's, registered once by its address.
        let VnodeKind::Fs(id) = vp.kind() else {
            panic!("not a file system's vnode");
        };
        assert_eq!(vops_id::<TestGlue>(&TEST_VOPS), id);

        insmntque(vp, None);
        assert!(vp.mount().is_none());
        assert!(!on_mount(mp, vp));
    }

    #[test]
    fn the_registry_holds_more_tables_in_the_host_tests_than_a_kernel_has() {
        let _g = testfs::setup();
        // A table of every slot's default (`EOPNOTSUPP`), a field so each has its own address.
        struct Table {
            _id: u8,
        }
        impl Vnops<TestGlue> for Table {}
        let tables: std::vec::Vec<&'static dyn Vnops<TestGlue>> = (0..40u8)
            .map(|i| &*std::boxed::Box::leak(std::boxed::Box::new(Table { _id: i })) as _)
            .collect();
        let ids: std::vec::Vec<VopsId> = tables.iter().map(|t| vops_id(*t)).collect();
        for (i, (t, id)) in tables.iter().zip(&ids).enumerate() {
            // Registered once: the same id again, and that id names the table.
            assert_eq!(vops_id(*t), *id);
            assert!(ptr::addr_eq(vops_table::<TestGlue>(*id), *t));
            assert!(id.0 >= VopsId::FIRST_FS);
            assert!(ids[..i].iter().all(|other| other != id));
        }
    }

    #[test]
    fn use_counts_and_the_free_list() {
        let (_g, mp) = setup_root();
        let inactive = INACTIVE.load(Ordering::SeqCst);
        let vp = testfs::vget_node(mp, B).unwrap();
        assert_eq!(vp.usecount(), 1);
        assert_eq!(vop_islocked(vp), LK_EXCLUSIVE);

        vref(vp);
        assert_eq!(vp.usecount(), 2);
        // vput of a vnode still referenced only unlocks it.
        vput(vp);
        assert_eq!(vp.usecount(), 1);
        assert_eq!(vop_islocked(vp), 0);
        assert_eq!(INACTIVE.load(Ordering::SeqCst), inactive);

        // The last vrele locks, deactivates (which unlocks) and frees the vnode, which keeps
        // its identity on the free list.
        assert!(vrele(vp));
        assert_eq!(vp.usecount(), 0);
        assert_eq!(INACTIVE.load(Ordering::SeqCst), inactive + 1);
        assert_eq!(vop_islocked(vp), 0);
        assert!(vp.bufs.bioflag() & VBIOONFREELIST != 0);
        assert!(on_free(vp));
        assert!(testfs::node_of(vp).is_some_and(|n| n.idx == B));

        // vget takes it back off the free list, referenced and locked.
        let again = testfs::vget_node(mp, B).unwrap();
        assert!(ptr::eq(again, vp));
        assert_eq!(vp.usecount(), 1);
        assert!(vp.bufs.bioflag() & VBIOONFREELIST == 0);
        assert!(!on_free(vp));
        vput(vp);
        assert_eq!(vp.usecount(), 0);
        assert_eq!(INACTIVE.load(Ordering::SeqCst), inactive + 2);
    }

    #[test]
    fn vput_of_the_last_use_unlocks_once() {
        let (_g, mp) = setup_root();
        let vp = testfs::vget_node(mp, C).unwrap();
        let before = UNLOCKS.load(Ordering::SeqCst);
        // VOP_INACTIVE unlocks the vnode itself; vput does not unlock it again.
        vput(vp);
        assert_eq!(UNLOCKS.load(Ordering::SeqCst), before + 1);
        assert_eq!(vop_islocked(vp), 0);
    }

    #[test]
    fn vhold_moves_a_free_vnode_to_the_hold_list() {
        let (_g, mp) = setup_root();
        let vp = testfs::vget_node(mp, B).unwrap();
        vput(vp);
        assert!(on_free(vp));
        vhold(vp);
        assert_eq!(vp.holdcnt(), 1);
        assert!(on_hold(vp));
        assert!(!on_free(vp));
        vdrop(vp);
        assert_eq!(vp.holdcnt(), 0);
        assert!(on_free(vp));
        assert!(!on_hold(vp));
    }

    #[test]
    fn vgone_reclaims_and_leaves_a_dead_vnode() {
        let (_g, mp) = setup_root();
        let reclaim = RECLAIM.load(Ordering::SeqCst);
        let inactive = INACTIVE.load(Ordering::SeqCst);
        let vp = testfs::vget_node(mp, C).unwrap();
        let _ = vop_unlock(vp);
        let id = vp.id();

        // An active vnode: closed, deactivated, reclaimed; the reference survives.
        vgone(vp);
        assert_eq!(RECLAIM.load(Ordering::SeqCst), reclaim + 1);
        assert_eq!(INACTIVE.load(Ordering::SeqCst), inactive + 1);
        assert_eq!(vp.vtype(), VBAD);
        assert_eq!(vp.kind(), VnodeKind::Dead);
        assert!(vp.mount().is_none());
        assert!(vp.data().is_null());
        assert_ne!(vp.id(), id, "cache_purge gives a new capability");
        assert!(testfs::vnode_of(C).is_none());
        assert_eq!(vp.usecount(), 1);

        // The dead vnode's last release puts it at the head of the free list, to be recycled
        // first.
        assert!(vrele(vp));
        assert!(
            vfs::<TestGlue>()
                .lists
                .lock()
                .free
                .first()
                .is_some_and(|v| ptr::eq(v, vp))
        );
    }

    #[test]
    fn getnewvnode_recycles_from_the_free_list_past_maxvnodes() {
        let (_g, mp) = setup_root();
        let t = vfs::<TestGlue>();
        let vp = testfs::vget_node(mp, B).unwrap();
        vput(vp);
        t.maxvnodes.store(1, Ordering::SeqCst);
        let before = t.numvnodes.load(Ordering::SeqCst);
        let reclaim = RECLAIM.load(Ordering::SeqCst);
        let nvp = getnewvnode(VT_NON, None, &TEST_VOPS).unwrap();
        assert!(ptr::eq(nvp, vp), "the free vnode is recycled");
        assert_eq!(t.numvnodes.load(Ordering::SeqCst), before);
        assert_eq!(RECLAIM.load(Ordering::SeqCst), reclaim + 1);
        assert!(testfs::vnode_of(B).is_none());
        assert_eq!(nvp.usecount(), 1);
        assert!(nvp.mount().is_none());
    }

    #[test]
    fn getnewvnode_answers_enfile_when_every_free_vnode_is_locked() {
        let (_g, mp) = setup_root();
        let t = vfs::<TestGlue>();
        let vp = testfs::vget_node(mp, B).unwrap();
        // Off the free list again with its lock held: release the use, keep the lock.
        vp.set_usecount(1);
        vput(vp);
        let _ = vn_lock(vp, LK_EXCLUSIVE);
        assert!(on_free(vp));
        t.maxvnodes.store(1, Ordering::SeqCst);
        let full = t.numvnodes.load(Ordering::SeqCst) * 4;
        t.numvnodes.store(full, Ordering::SeqCst);
        // The toggle keeps half the calls off the hold list; the free list is not empty.
        assert_eq!(
            getnewvnode(VT_NON, None, &TEST_VOPS).err(),
            Some(Errno::ENFILE)
        );
        let _ = vop_unlock(vp);
    }

    #[test]
    fn vflush_counts_busy_vnodes_unless_forced() {
        let (_g, mp) = setup_root();
        let busy = testfs::vget_node(mp, B).unwrap();
        let _ = vop_unlock(busy);
        let idle = testfs::vget_node(mp, C).unwrap();
        vput(idle);

        // The root (two references) and B are busy; C is idle and goes.
        let root = super::super::init::rootvnode::<TestGlue>();
        assert_eq!(vflush(mp, root, 0), Err(Errno::EBUSY));
        assert!(testfs::vnode_of(C).is_none());
        assert!(testfs::vnode_of(B).is_some());

        assert_eq!(vflush(mp, root, FORCECLOSE), Ok(()));
        assert!(testfs::vnode_of(B).is_none());
        assert_eq!(busy.vtype(), VBAD);
        // Only the skipped root is left on the mount.
        assert_eq!(mp.vnodelist.lock().iter().count(), 1);
        assert!(testfs::vnode_of(ROOT).is_some());
    }

    #[test]
    fn vfs_busy_fails_on_an_unmounting_file_system() {
        let (_g, mp) = setup_root();
        assert_eq!(vfs_busy(mp, VB_READ | VB_NOWAIT), Ok(()));
        assert!(vfs_isbusy(mp));
        vfs_unbusy(mp);
        assert!(!vfs_isbusy(mp));
        mp.flag_set(MNT_UNMOUNT);
        assert_eq!(vfs_busy(mp, VB_READ | VB_NOWAIT), Err(Errno::EBUSY));
        assert!(!vfs_isbusy(mp));
        mp.flag_clr(MNT_UNMOUNT);
    }

    #[test]
    fn a_mount_is_reused_only_after_its_last_reference() {
        let (_g, _mp) = setup_root();
        let m2 = vfs_mount_alloc(None, &TEST_CONF[0]);
        vfs_unbusy(m2);
        // A holder's reference (a vfs_busy sleeper's) keeps it off the free list.
        let _ = vfs_mount_take(m2);
        vfs_mount_free(m2);
        let m3 = vfs_mount_alloc(None, &TEST_CONF[0]);
        assert!(!ptr::eq(m3, m2), "a referenced mount is not reused");
        vfs_unbusy(m3);
        // The holder's release puts it on the free list; the next mount is the same one,
        // made afresh.
        assert!(vfs_busy(m2, VB_READ | VB_NOWAIT).is_err(), "unmounting");
        crate::fs::mount::vfs_mount_rele(m2);
        let m4 = vfs_mount_alloc(None, &TEST_CONF[0]);
        assert!(ptr::eq(m4, m2));
        assert_eq!(m4.flag() & MNT_UNMOUNT, 0);
        assert!(vfs_isbusy(m4));
        vfs_unbusy(m4);
    }

    #[test]
    fn vfs_getnewfsid_is_unique_among_mounts() {
        let (_g, mp) = setup_root();
        vfs_getnewfsid(mp);
        let fsid = mp.stat().f_fsid;
        assert!(vfs_getvfs::<TestGlue>(&fsid).is_some_and(|m| ptr::eq(m, mp)));
        let mp2 = vfs_mount_alloc(None, &TEST_CONF[0]);
        vfs_unbusy(mp2);
        crate::fs::mount::mountlist_insert_tail(mp2);
        vfs_getnewfsid(mp2);
        assert_ne!(mp2.stat().f_fsid, fsid);
        assert_eq!(&mp2.stat().f_fstypename[..7], b"testfs\0");
    }

    #[test]
    fn checkalias_is_not_reached_for_regular_vnodes() {
        let (_g, mp) = setup_root();
        let vp = testfs::vget_node(mp, B).unwrap();
        assert!(checkalias(vp, 0, Some(mp)).is_none());
        assert!(!vp.has_specinfo());
        vput(vp);
    }

    #[test]
    fn device_aliases_are_counted_and_unmarked_when_one_goes() {
        let (_g, _mp) = setup_root();
        let dev = makedev(1, 7);
        let a = cdevvp::<TestGlue>(dev).unwrap().unwrap();
        assert_eq!(a.rdev(), dev);
        assert_ne!(a.flag() & VISTTY, 0, "major 1 is a tty");
        assert!(vfinddev::<TestGlue>(dev, VCHR).is_some_and(|v| ptr::eq(v, a)));
        // A second vnode of the same device: an alias (getdevvp returns the first).
        let b = getnewvnode_id::<TestGlue>(VT_NON, None, VopsId::SPEC).unwrap();
        b.set_vtype(VCHR);
        assert!(checkalias(b, dev, None).is_none());
        assert_ne!(a.flag() & VALIASED, 0);
        assert_ne!(b.flag() & VALIASED, 0);
        assert_eq!(vcount(a), 2);
        // The alias goes: the other one is the device's only vnode again.
        vgone(b);
        assert_eq!(a.flag() & VALIASED, 0);
        assert_eq!(vcount(a), 1);
        assert!(!b.has_specinfo());
    }

    #[test]
    fn a_revoke_of_an_aliased_device_takes_every_alias() {
        let (_g, _mp) = setup_root();
        let dev = makedev(4, 1);
        let a = cdevvp::<TestGlue>(dev).unwrap().unwrap();
        let b = getnewvnode_id::<TestGlue>(VT_NON, None, VopsId::SPEC).unwrap();
        b.set_vtype(VCHR);
        assert!(checkalias(b, dev, None).is_none());
        assert_eq!(vop_revoke(a, REVOKEALL), Ok(()));
        assert_eq!(a.kind(), VnodeKind::Dead);
        assert_eq!(b.kind(), VnodeKind::Dead);
        assert!(vfinddev::<TestGlue>(dev, VCHR).is_none());
    }

    #[test]
    fn syncer_vnodes_go_on_the_wheel_and_move_by_their_delay() {
        let (_g, _mp) = setup_root();
        let mp = vfs_mount_alloc(None, &TEST_CONF[0]);
        vfs_allocate_syncvnode(mp).expect("a syncer vnode");
        let vp = mp.syncer().expect("mnt_syncer");
        assert!(vp.bufs.bioflag() & VBIOONSYNCLIST != 0);
        assert_eq!(vp.kind(), VnodeKind::Sync);
        let slot = |vp: &V| usize::from(vp.syncslot.load(Ordering::Relaxed));
        let base = slot(vp);

        vn_syncer_add_to_worklist(vp, 1000);
        let far = slot(vp);
        assert_ne!(far, base);
        vn_syncer_add_to_worklist(vp, 3);
        assert_ne!(slot(vp), far);
        // The syncer vnode's last use takes it off the wheel.
        assert_eq!(SyncVops::<TestGlue>::TABLE.inactive(vp, None), Ok(()));
        assert_eq!(vp.bufs.bioflag() & VBIOONSYNCLIST, 0);
        assert!(mp.syncer().is_none());
    }

    #[test]
    fn the_pager_reads_through_a_kernel_uio() {
        let (_g, mp) = setup_root();
        let vp = testfs::vget_node(mp, B).unwrap();
        let mut buf = [0u8; 16];
        let ops: &'static dyn VnodeOps = vp;
        assert_eq!(ops.read(0, &mut buf, 0), Ok(16));
        assert_eq!(ops.vtype(), VREG);
        assert_eq!(ops.mount_id(), mp.addr());
        vput(vp);
    }
}
/* </TESTS> */
