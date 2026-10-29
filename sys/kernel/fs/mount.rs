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
//! `<sys/mount.h>`: the mounted file system (`struct mount`) and its configuration entry
//! (`struct vfsconf`); the mount's lifetime and busy protocol (`vfs_subr.c`'s
//! `vfs_mount_alloc`, `vfs_mount_free`, `vfs_busy`, `vfs_unbusy`, `vfs_isbusy`) and the
//! mount list.
//!
//! Upstream: sys/sys/mount.h @ 3ce1f3f79392
//! Upstream: sys/kern/vfs_subr.c @ 3ce1f3f79392
//! LZ: sys/sys/mount.rs@3c62ede68802
//! LZ: sys/kern/vfs_subr.rs@3c62ede68802
//!
//! The structures userland sees (`struct statfs`, `fsid_t`, `struct fid`, `fhandle_t`) and
//! the `MNT_*`, `VB_*`, `VFS_*` constants are `hal::sys::mount`'s, shared with `bsd`; the
//! operations a file system type provides (`struct vfsops`) are [`super::vops::Vfs`].
//!
//! ## Deviations
//! - `struct vfsconf`'s `vfc_refcount` is atomic (`atomic_inc_int` in C; LZ's deviation).
//!
//! ## Redesign
//! - A mount is never freed (D-N5.2-2, the user's): `vfs_mount_alloc` takes a mount from the
//!   free list or allocates one (`malloc(M_MOUNT, M_WAITOK | M_ZERO)`, leaked: `MBox::leak`),
//!   and the release of its last reference (`mnt_refs`, a `hal::refcnt::Refcnt`: the creator's,
//!   and one per `vfs_busy` while it waits) puts it on the free list instead of freeing it.
//!   So `&'static Mount<G>` is true for every holder (the vnodes' `v_mount`, the pager's
//!   mount identities, the `bsd` file systems) and a `vfs_busy` sleeper never wakes on a reused
//!   mount (the C's reason for the count). `vmstat -m`'s `M_MOUNT` row counts the mounts ever
//!   alive at once (a high-water mark) where the C's falls after an unmount.
//! - `mnt_op` and `mnt_vfc` are one link to the configuration entry ([`Mount::vfc`]; the
//!   operations are its `vfc_vfsops`, as the C copies them from it); `mnt_flag` is a Relaxed
//!   atomic (read unlocked by the file systems on every write, as the C reads it);
//!   `mnt_stat` a mutex at `IPL_NONE` ([`Mount::stat`], [`Mount::with_stat`]) beside the four
//!   I/O counters, atomics the buffer cache bumps without it (the C's unlocked `++`), merged
//!   into every copy of the statistics; `mnt_lock` a [`SplitRwlock`] (`vfs_busy` and
//!   `vfs_unbusy` are separate calls, as the C's); the WITNESS-only flags
//!   (`RWL_IS_VNODE`, `VB_DUPOK`'s `RW_DUPOK`) are not configured.
//! - `mountlist` and the free mounts are `MountLists`, under a leaf mutex at `IPL_NONE` the C
//!   does not have (it covers the list with the kernel lock, whose witness the kernel cannot
//!   take per access; D-N5.2-13): mount, unmount and the walks take it around each step, never
//!   across a sleep; a walk reads the next element as the C's `TAILQ_FOREACH` does, the
//!   current mount kept by `vfs_busy`. A mount's vnode list (`mnt_vnodelist`) is a leaf mutex
//!   of the mount's too, taken by `insmntque` and the walks of
//!   [`super::subr::vfs_mount_foreach_vnode`].

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, AtomicU64, Ordering};

use hal::intr::IPL_NONE;
use hal::link::Link;
use hal::list::{List, ListEntry};
use hal::malloc::MBox;
use hal::mutex::Mutexed;
use hal::refcnt::Refcnt;
use hal::rwlock::{RW_NOSLEEP, RW_READ, RW_WRITE, SplitRwlock};
use hal::sys::errno::Errno;
use hal::sys::mount::{
    MFSNAMELEN, MNAMELEN, MNT_UNMOUNT, Statfs, VB_NOWAIT, VB_READ, VB_WAIT, VB_WRITE,
};

use super::vnode::Vnode;
use super::vops::Vfs;
use crate::assert::kpanic;
use crate::proc::ProcGlue;

/// `M_MOUNT`: the malloc type of `struct mount` (`<sys/malloc.h>`).
pub const M_MOUNT: i32 = 5;
/// `M_WAITOK` (`<sys/malloc.h>`).
const M_WAITOK: i32 = 0x0001;
/// `M_ZERO` (`<sys/malloc.h>`).
const M_ZERO: i32 = 0x0008;

/// `struct vfsconf`: filesystem configuration information. One of these exists for each
/// type of filesystem supported by the kernel (`vfsconflist[]`, `bsd`'s `vfs_init.rs`).
pub struct Vfsconf<G: ProcGlue> {
    /// `vfc_vfsops`: filesystem operations vector.
    pub vfc_vfsops: &'static dyn Vfs<G>,
    /// `vfc_name`: filesystem type name.
    pub vfc_name: [u8; MFSNAMELEN],
    /// `vfc_typenum`: historic filesystem type number.
    pub vfc_typenum: i32,
    /// `vfc_refcount`: number mounted of this type.
    pub vfc_refcount: AtomicU32,
    /// `vfc_flags`: permanent flags.
    pub vfc_flags: i32,
    /// `vfc_datasize`: size of data args.
    pub vfc_datasize: usize,
}

impl<G: ProcGlue> Vfsconf<G> {
    /// A configuration entry: `{ &ops, name, typenum, 0, flags, datasize }`.
    pub const fn new(
        vfsops: &'static dyn Vfs<G>,
        name: &[u8],
        typenum: i32,
        flags: i32,
        datasize: usize,
    ) -> Self {
        let mut vfc_name = [0u8; MFSNAMELEN];
        let mut i = 0;
        while i < name.len() && i < MFSNAMELEN - 1 {
            vfc_name[i] = name[i];
            i += 1;
        }
        Self {
            vfc_vfsops: vfsops,
            vfc_name,
            vfc_typenum: typenum,
            vfc_refcount: AtomicU32::new(0),
            vfc_flags: flags,
            vfc_datasize: datasize,
        }
    }

    /// `vfc_name` without the NUL padding.
    pub fn name(&self) -> &[u8] {
        let len = self
            .vfc_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(MFSNAMELEN);
        &self.vfc_name[..len]
    }
}

/// `mnt_vnodelist`'s adapter: a mount's vnodes, through `v_mntvnodes`.
pub struct ByMntvnodes<G: ProcGlue>(core::marker::PhantomData<G>);

impl<G: ProcGlue> hal::list::ListAdapter for ByMntvnodes<G> {
    type Elem = Vnode<G>;
    fn entry(e: &Vnode<G>) -> &ListEntry<Vnode<G>> {
        &e.mntvnodes
    }
}

/// `mnt_list`'s adapter: `mountlist`, or the free mounts.
pub struct ByMntList<G: ProcGlue>(core::marker::PhantomData<G>);

impl<G: ProcGlue> hal::list::ListAdapter for ByMntList<G> {
    type Elem = Mount<G>;
    fn entry(e: &Mount<G>) -> &ListEntry<Mount<G>> {
        &e.list
    }
}

/// `mnt_dounmount`'s adapter: `dounmount`'s stack of the mounts to unmount after one.
pub struct ByDounmount<G: ProcGlue>(core::marker::PhantomData<G>);

impl<G: ProcGlue> hal::list::ListAdapter for ByDounmount<G> {
    type Elem = Mount<G>;
    fn entry(e: &Mount<G>) -> &ListEntry<Mount<G>> {
        &e.dounmount
    }
}

/// The index of each I/O counter in [`Mount`]'s `io`.
const IO_SYNCWRITES: usize = 0;
const IO_SYNCREADS: usize = 1;
const IO_ASYNCWRITES: usize = 2;
const IO_ASYNCREADS: usize = 3;

/// `struct mount`: structure per mounted file system. Each mounted file system has an
/// array of operations and an instance record.
pub struct Mount<G: ProcGlue> {
    /// `mnt_list`: mount list (or the free mounts).
    pub(super) list: ListEntry<Mount<G>>,
    /// Whether the mount is on `mountlist` (`list` is on the free list otherwise, or none).
    pub(super) listed: AtomicBool,
    /// `mnt_dounmount`: unmount work queue.
    pub dounmount: ListEntry<Mount<G>>,
    /// `mnt_vfc` (and `mnt_op`, its `vfc_vfsops`): configuration info.
    vfc: Link<Vfsconf<G>>,
    /// `mnt_vnodecovered`: vnode we mounted on.
    vnodecovered: Link<Vnode<G>>,
    /// `mnt_syncer`: syncer vnode.
    syncer: Link<Vnode<G>>,
    /// `mnt_vnodelist`: list of vnodes this mount.
    pub(super) vnodelist: Mutexed<List<ByMntvnodes<G>>>,
    /// `mnt_lock`: mount structure lock (`vfs_busy`).
    lock: SplitRwlock,
    /// `mnt_refs`: the references that keep the mount off the free list.
    refs: Refcnt,
    /// `mnt_flag`: flags.
    flag: AtomicI32,
    /// `mnt_stat`: cache of filesystem stats (but the four I/O counters).
    stat: Mutexed<Statfs>,
    /// `mnt_stat.f_syncwrites`, `f_syncreads`, `f_asyncwrites`, `f_asyncreads`.
    io: [AtomicU64; 4],
    /// `mnt_data`: private data.
    data: AtomicPtr<c_void>,
}

impl<G: ProcGlue> Mount<G> {
    /// A zeroed mount, as `malloc(M_ZERO)` returns it.
    fn new() -> Self {
        Self {
            list: ListEntry::new(),
            listed: AtomicBool::new(false),
            dounmount: ListEntry::new(),
            vfc: Link::new(),
            vnodecovered: Link::new(),
            syncer: Link::new(),
            vnodelist: Mutexed::new(IPL_NONE, List::new()),
            lock: SplitRwlock::new("vfslock"),
            refs: Refcnt::new(),
            flag: AtomicI32::new(0),
            stat: Mutexed::new(IPL_NONE, Statfs::new()),
            io: [const { AtomicU64::new(0) }; 4],
            data: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// A mount outside `vfs_mount_alloc`, for a test double (`bsd`'s file systems' unit tests
    /// build one by hand), of the type `vfc` when given; on no list, not busy.
    #[cfg(any(test, feature = "doubles"))]
    pub fn new_unpooled(vfc: Option<&'static Vfsconf<G>>) -> Self {
        let mp = Self::new();
        mp.vfc.set(vfc);
        mp
    }

    /// `mp->mnt_vfc`, which every mount out of `vfs_mount_alloc` has.
    #[inline]
    pub fn vfc(&self) -> &'static Vfsconf<G> {
        match self.vfc.get() {
            Some(vfc) => vfc,
            None => kpanic(format_args!("mount {self:p}: no mnt_vfc")),
        }
    }

    /// `mp->mnt_op`: the type's operations (`mnt_vfc->vfc_vfsops`).
    #[inline]
    pub fn op(&self) -> &'static dyn Vfs<G> {
        self.vfc().vfc_vfsops
    }

    /// `mp->mnt_flag`.
    #[inline]
    pub fn flag(&self) -> i32 {
        self.flag.load(Ordering::Relaxed)
    }

    /// `mp->mnt_flag = f`.
    #[inline]
    pub fn set_flag(&self, f: i32) {
        self.flag.store(f, Ordering::Relaxed);
    }

    /// `mp->mnt_flag |= f`.
    #[inline]
    pub fn flag_set(&self, f: i32) {
        self.flag.fetch_or(f, Ordering::Relaxed);
    }

    /// `mp->mnt_flag &= ~f`.
    #[inline]
    pub fn flag_clr(&self, f: i32) {
        self.flag.fetch_and(!f, Ordering::Relaxed);
    }

    /// `mp->mnt_vnodecovered`.
    pub fn vnodecovered(&self) -> Option<&'static Vnode<G>> {
        self.vnodecovered.get()
    }

    /// `mp->mnt_vnodecovered = vp`.
    pub fn set_vnodecovered(&self, vp: Option<&'static Vnode<G>>) {
        self.vnodecovered.set(vp);
    }

    /// `mp->mnt_syncer`.
    pub fn syncer(&self) -> Option<&'static Vnode<G>> {
        self.syncer.get()
    }

    /// `mp->mnt_syncer = vp`.
    pub fn set_syncer(&self, vp: Option<&'static Vnode<G>>) {
        self.syncer.set(vp);
    }

    /// `mp->mnt_data`.
    #[inline]
    pub fn data(&self) -> *mut c_void {
        self.data.load(Ordering::Relaxed)
    }

    /// `mp->mnt_data = p`.
    #[inline]
    pub fn set_data(&self, p: *mut c_void) {
        self.data.store(p, Ordering::Relaxed);
    }

    /// A copy of `mp->mnt_stat`.
    pub fn stat(&self) -> Statfs {
        let mut sp = *self.stat.lock();
        self.merge_io(&mut sp);
        sp
    }

    /// `f(&mp->mnt_stat)` under the statistics' lock (the I/O counters as they are now, read
    /// only: the buffer cache keeps them). `f` must not sleep.
    pub fn with_stat<R>(&self, f: impl FnOnce(&mut Statfs) -> R) -> R {
        let mut g = self.stat.lock();
        self.merge_io(&mut g);
        f(&mut g)
    }

    /// The four I/O counters into `sp`.
    fn merge_io(&self, sp: &mut Statfs) {
        sp.f_syncwrites = self.io[IO_SYNCWRITES].load(Ordering::Relaxed);
        sp.f_syncreads = self.io[IO_SYNCREADS].load(Ordering::Relaxed);
        sp.f_asyncwrites = self.io[IO_ASYNCWRITES].load(Ordering::Relaxed);
        sp.f_asyncreads = self.io[IO_ASYNCREADS].load(Ordering::Relaxed);
    }

    /// `mp->mnt_stat.f_{sync,async}{reads,writes}++` (the buffer cache's statistics).
    #[inline]
    pub fn count_io(&self, write: bool, async_: bool) {
        let i = match (write, async_) {
            (true, false) => IO_SYNCWRITES,
            (false, false) => IO_SYNCREADS,
            (true, true) => IO_ASYNCWRITES,
            (false, true) => IO_ASYNCREADS,
        };
        self.io[i].fetch_add(1, Ordering::Relaxed);
    }

    /// `mp->mnt_stat.f_mntonname` and its length.
    pub fn mntonname(&self) -> ([u8; MNAMELEN], usize) {
        let name = self.stat.lock().f_mntonname;
        let len = name.iter().position(|&c| c == 0).unwrap_or(MNAMELEN);
        (name, len)
    }

    /// `rw_status(&mp->mnt_lock)` (`vfs_isbusy`'s and `vfs_stall`'s questions).
    pub fn lock_status(&self) -> i32 {
        self.lock.status()
    }

    /// The mount's address as a number (messages, identities).
    pub fn addr(&self) -> usize {
        ptr::from_ref(self).addr()
    }

    /// Whether the mount is on `mountlist`.
    pub fn is_listed(&self) -> bool {
        self.listed.load(Ordering::Relaxed)
    }

    /// `TAILQ_FOREACH(vp, &mp->mnt_vnodelist, v_mntvnodes)`: the mount's vnodes, the first
    /// read under the list's lock, each next one read before a vnode is handed out (unlocked,
    /// as the C reads it), so the walker may move the vnode it has. A walker that sleeps
    /// restarts when the vnode it is at left the mount (`vp.mount() != mp`), as the C's
    /// `goto loop`s do.
    pub fn vnodes(&self) -> MntVnodes<G> {
        MntVnodes {
            next: self.vnodelist.lock().first(),
        }
    }

    /// `TAILQ_EMPTY(&mp->mnt_vnodelist)`.
    pub fn vnodelist_is_empty(&self) -> bool {
        self.vnodelist.lock().is_empty()
    }
}

/// The walk of [`Mount::vnodes`].
pub struct MntVnodes<G: ProcGlue> {
    next: Option<&'static Vnode<G>>,
}

impl<G: ProcGlue> Iterator for MntVnodes<G> {
    type Item = &'static Vnode<G>;

    fn next(&mut self) -> Option<&'static Vnode<G>> {
        let vp = self.next?;
        self.next = List::<ByMntvnodes<G>>::next(vp);
        Some(vp)
    }
}

/// `mountlist` and the free mounts (D-N5.2-2), under the subsystem's leaf mutex
/// ([`super::subr::VfsTables`]'s `mounts`).
pub struct MountLists<G: ProcGlue> {
    /// `mountlist`: mounted filesystem list.
    pub all: List<ByMntList<G>>,
    /// The mounts whose last reference went, for `vfs_mount_alloc` to reuse.
    pub(super) free: List<ByMntList<G>>,
}

impl<G: ProcGlue> MountLists<G> {
    /// Both lists empty.
    pub const fn new() -> Self {
        Self {
            all: List::new(),
            free: List::new(),
        }
    }
}

impl<G: ProcGlue> Default for MountLists<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The mount lists of `G`'s subsystem.
fn mounts<G: ProcGlue>() -> &'static Mutexed<MountLists<G>> {
    &super::subr::vfs::<G>().mounts
}

/// Allocate a mount point. The returned mount point is marked as busy.
pub fn vfs_mount_alloc<G: ProcGlue>(
    vp: Option<&'static Vnode<G>>,
    vfsp: &'static Vfsconf<G>,
) -> &'static Mount<G> {
    let reused = mounts::<G>().lock().free.pop_head();
    let mp: &'static Mount<G> = match reused {
        Some(mp) => {
            mp.reset();
            mp
        }
        None => match MBox::new(M_MOUNT, M_WAITOK | M_ZERO, Mount::new()) {
            Some(b) => MBox::leak(b),
            None => kpanic(format_args!("vfs_mount_alloc: out of memory")),
        },
    };
    let _ = vfs_busy(mp, VB_READ | VB_NOWAIT);

    mp.vnodecovered.set(vp);

    vfsp.vfc_refcount.fetch_add(1, Ordering::SeqCst);
    mp.vfc.set(Some(vfsp));
    mp.flag.store(vfsp.vfc_flags, Ordering::Relaxed);
    let name = vfsp.name();
    mp.stat.lock().f_fstypename[..name.len()].copy_from_slice(name);

    mp
}

impl<G: ProcGlue> Mount<G> {
    /// A recycled mount made as `malloc(M_ZERO)` leaves a new one (it is on no list, its
    /// last reference gone: no one else reaches it).
    fn reset(&self) {
        self.listed.store(false, Ordering::Relaxed);
        self.vfc.set(None);
        self.vnodecovered.set(None);
        self.syncer.set(None);
        if !self.vnodelist.lock().is_empty() {
            kpanic(format_args!(
                "vfs_mount_alloc: reused mount {self:p} has vnodes"
            ));
        }
        if self.lock.status() != 0 {
            kpanic(format_args!(
                "vfs_mount_alloc: reused mount {self:p} is busy"
            ));
        }
        self.refs.init();
        self.flag.store(0, Ordering::Relaxed);
        *self.stat.lock() = Statfs::new();
        for c in &self.io {
            c.store(0, Ordering::Relaxed);
        }
        self.data.store(ptr::null_mut(), Ordering::Relaxed);
    }
}

/// `vfs_mount_take(mp)`: another reference to `mp`.
pub fn vfs_mount_take<G: ProcGlue>(mp: &'static Mount<G>) -> &'static Mount<G> {
    mp.refs.take();
    mp
}

/// `vfs_mount_rele(mp)`: drops a reference; the last one puts the mount on the free list
/// (the C frees it).
pub(crate) fn vfs_mount_rele<G: ProcGlue>(mp: &'static Mount<G>) {
    if mp.refs.rele() {
        mounts::<G>().lock().free.insert_tail(mp);
    }
}

/// Release a mount point.
pub fn vfs_mount_free<G: ProcGlue>(mp: &'static Mount<G>) {
    mp.flag_set(MNT_UNMOUNT);
    mp.vfc().vfc_refcount.fetch_sub(1, Ordering::SeqCst);
    vfs_mount_rele(mp);
}

/// Mark a mount point as busy. Used to synchronize access and to delay unmounting.
///
/// Default behaviour is to attempt getting a READ lock and in case of an ongoing unmount, to
/// wait for it to finish and then return failure.
pub fn vfs_busy<G: ProcGlue>(mp: &'static Mount<G>, flags: i32) -> Result<(), Errno> {
    let mut rwflags = if flags & VB_WRITE != 0 {
        RW_WRITE
    } else {
        RW_READ
    };
    if flags & VB_WAIT == 0 {
        rwflags |= RW_NOSLEEP;
    }
    // WITNESS: VB_DUPOK -> RW_DUPOK, not configured.

    vfs_mount_take(mp);
    let error = if mp.lock.enter(rwflags).is_err() {
        Err(Errno::EBUSY)
    } else if mp.flag() & MNT_UNMOUNT != 0 {
        mp.lock.exit();
        Err(Errno::EBUSY)
    } else {
        Ok(())
    };
    vfs_mount_rele(mp);

    error
}

/// Free a busy file system.
pub fn vfs_unbusy<G: ProcGlue>(mp: &Mount<G>) {
    mp.lock.exit();
}

/// `vfs_isbusy(mp)`: whether the mount is busied.
pub fn vfs_isbusy<G: ProcGlue>(mp: &Mount<G>) -> bool {
    mp.lock.status() != 0
}

/// `TAILQ_INSERT_TAIL(&mountlist, mp, mnt_list)`: a new mount joins the list.
pub fn mountlist_insert_tail<G: ProcGlue>(mp: &'static Mount<G>) {
    let g = mounts::<G>().lock();
    g.all.insert_tail(mp);
    mp.listed.store(true, Ordering::Relaxed);
}

/// `TAILQ_INSERT_HEAD(&mountlist, mp, mnt_list)`: the root mount, first.
pub fn mountlist_insert_head<G: ProcGlue>(mp: &'static Mount<G>) {
    let g = mounts::<G>().lock();
    g.all.insert_head(mp);
    mp.listed.store(true, Ordering::Relaxed);
}

/// `TAILQ_REMOVE(&mountlist, mp, mnt_list)`: an unmounted (or failed) mount leaves the list.
pub fn mountlist_remove<G: ProcGlue>(mp: &'static Mount<G>) {
    let g = mounts::<G>().lock();
    g.all.remove(mp);
    mp.listed.store(false, Ordering::Relaxed);
}

/// `TAILQ_EMPTY(&mountlist)`.
pub fn mountlist_is_empty<G: ProcGlue>() -> bool {
    mounts::<G>().lock().all.is_empty()
}

/// The first mount of `mountlist` (`TAILQ_FIRST`), the last with `rev` (`TAILQ_LAST`).
pub fn mountlist_first<G: ProcGlue>(rev: bool) -> Option<&'static Mount<G>> {
    let g = mounts::<G>().lock();
    if rev { g.all.last() } else { g.all.first() }
}

/// The mount after `mp` on `mountlist` (`TAILQ_NEXT`, `TAILQ_PREV` with `rev`): read as the
/// C reads it, unlocked, while the walker keeps `mp` on the list (`vfs_busy`); `None` when
/// `mp` left it meanwhile.
pub fn mountlist_next<G: ProcGlue>(mp: &'static Mount<G>, rev: bool) -> Option<&'static Mount<G>> {
    if !mp.is_listed() {
        return None;
    }
    if rev {
        List::<ByMntList<G>>::prev(mp)
    } else {
        List::<ByMntList<G>>::next(mp)
    }
}

/// A walk of `mountlist` (`TAILQ_FOREACH_SAFE`, `_REVERSE_SAFE` with `rev`): the successor is
/// read before a mount is handed out, so the walker may unmount it; a successor that left the
/// list ends the walk.
pub struct MountIter<G: ProcGlue> {
    next: Option<&'static Mount<G>>,
    rev: bool,
}

impl<G: ProcGlue> Iterator for MountIter<G> {
    type Item = &'static Mount<G>;

    fn next(&mut self) -> Option<&'static Mount<G>> {
        let mp = self.next?;
        if !mp.is_listed() {
            self.next = None;
            return None;
        }
        self.next = mountlist_next(mp, self.rev);
        Some(mp)
    }
}

/// `TAILQ_FOREACH(mp, &mountlist, mnt_list)` (`TAILQ_FOREACH_REVERSE` with `rev`).
pub fn mountlist<G: ProcGlue>(rev: bool) -> MountIter<G> {
    MountIter {
        next: mountlist_first(rev),
        rev,
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_io_counters_land_in_every_copy_of_the_statistics() {
        let mp: Mount<crate::fs::testfs::TestGlue> = Mount::new();
        mp.count_io(false, false);
        mp.count_io(false, false);
        mp.count_io(true, false);
        mp.count_io(false, true);
        mp.count_io(true, true);
        mp.count_io(true, true);
        mp.count_io(true, true);
        let sp = mp.stat();
        assert_eq!(sp.f_syncreads, 2);
        assert_eq!(sp.f_syncwrites, 1);
        assert_eq!(sp.f_asyncreads, 1);
        assert_eq!(sp.f_asyncwrites, 3);
        let w = mp.with_stat(|sp| {
            sp.f_bsize = 512;
            sp.f_asyncwrites
        });
        assert_eq!(w, 3);
        assert_eq!(mp.stat().f_bsize, 512);
    }

    #[test]
    fn a_vfsconf_name_drops_its_padding() {
        let c: Vfsconf<crate::fs::testfs::TestGlue> =
            Vfsconf::new(&crate::fs::testfs::TEST_VFS, b"testfs", 99, 0, 0);
        assert_eq!(c.name(), b"testfs");
        assert_eq!(c.vfc_typenum, 99);
    }
}
/* </TESTS> */
