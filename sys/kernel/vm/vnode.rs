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
 *
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed by Charles D. Cranor and
 *      Washington University.
 * 4. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_vnode.h,v 1.1.2.4 1997/10/03 21:18:24 chuck Exp
 */
/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993
 *      The Regents of the University of California.
 * Copyright (c) 1990 University of Utah.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
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
 *      @(#)vnode_pager.c       8.8 (Berkeley) 2/13/94
 * from: Id: uvm_vnode.c,v 1.1.2.26 1998/02/02 20:38:07 chuck Exp
 */
/* </LICENSES> */

/* <CODE> */
//! The vnode pager, the `uvm_object` behind a mapped file: `struct uvm_vnode` of
//! `<uvm/uvm_vnode.h>` and `uvm/uvm_vnode.c`. Its pages are read and written through the
//! vnode layer (`VOP_READ`/`VOP_WRITE`, so through the file system and the buffer cache), one
//! page at a time on the way in and a cluster at a time on the way out; an object whose last
//! mapping goes away persists with its pages until the vnode is recycled
//! ([`uvm_vnp_terminate`]).
//!
//! Upstream: sys/uvm/uvm_vnode.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_vnode.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_vnode.rs@3c62ede68802
//!
//! NOTE: currently we have to use VOP_READ/VOP_WRITE because they go through the buffer
//! cache and allow I/O in any size. These VOPs use synchronous i/o. [vs. VOP_STRATEGY which
//! can be async, but doesn't go through the buffer cache or allow I/O sizes larger than a
//! block]. we will eventually want to change this. (The C's comment goes on to sketch an
//! async aiodesc interface that was never written.)
//!
//! ## Deviations
//! - `uvn_attach` of a block device needs the device switch (`bdevsw[].d_type`, `DIOCGPART`
//!   through `d_ioctl`, `conf.c`): reported, and the attach fails (NULL) as for a device that
//!   is not a disk.
//! - `uvm_vnp_sync(NULL)` is `uvm_vnp_sync(None)`, a mount named by its identity
//!   ([`VnodeOps::mount_id`]); `uvm_vnp_uncache` returns `bool`.
//! - `UVM_PAGE_OWN` is not configured; the `DEBUG` printfs are compiled out as in C.
//!
//! ## Redesign
//! - The vnode (`struct vnode *u_vnode`, `vp->v_uvm`, and the `VOP_*`, `vget`, `vref`,
//!   `vrele`, `vn_lock` calls on it) -> `&'static dyn VnodeOps`: a trait `bsd`'s `Vnode`
//!   implements (bridges.toml `VnodeOps`, until the vnode layer moves, N5), each method one
//!   call; `VOP_READ`/`VOP_WRITE` take the pager window's bytes as a slice and the offset
//!   (`bsd` builds the `UIO_SYSSPACE` uio over them with `curproc`'s credentials, as pipe's
//!   `UioMove` does). The sequence of `uvn_io` (the net lock let go, the kernel lock, `vn_lock`,
//!   the VOP, the unlocks) is composed here from those one-call methods and the `KernelLock`
//!   and `NetLock` bridges (`hal::klock`, `hal::netlock`); `ratecheck`'s clock and `rebooting`
//!   are the [`SysInfo`] bridge's.
//! - `ratecheck(9)` (`kern_time.c`) is `bsd`'s until N4: the kernel has its own copy for
//!   [`uvn_flush`]'s rate-limited error message, over [`SysInfo`]'s uptime. The C's
//!   `ratecheck` serializes every caller's `lasttime` under one `ratecheck_mtx` at
//!   `IPL_HIGH` (`kern_time.c` 824-850); here the one call site's `lasttime` (the C's
//!   function-local `static`) sits in a mutex of its own at `IPL_HIGH`, so the same update is
//!   made under a lock, per call site rather than shared.
//! - A `struct uvm_vnode` is a `uvm_vnode_pool` item that is never freed (the vnode keeps it
//!   in `v_uvm` across recycling, as in C): a `hal::pool::PoolArc` of `"uvmvnodes"` leaked
//!   once made (`PoolArc::leak`), so a true `&'static Uvn`, which the map's handle
//!   (`kernel::vm::object::ObjRef::Vnode`) and the vnode hold; no `#[repr(C)]` first-member
//!   cast: the pager functions take the [`Uvn`].
//! - `u_flags`, `u_nio`, `u_size` (\[v\]) are relaxed atomics whose setters assert the
//!   object's lock; `u_wlist` (\[K\]) and `u_syncq` (\[S\]) are `hal::list` entries, the write
//!   list a static list under the kernel lock (asserted where the C asserts it), the sync queue
//!   inside the `hal::rwlock::Rwlocked` that is `uvn_sync_lock` (its `RWL_IS_VNODE` flag is a
//!   witness(4) class, nothing here): the C's `SIMPLEQ_INIT` of the queue becomes taking each
//!   uvn off it as it is synced.
//! - `uvn_get` is two functions: [`uvn_get_locked`] (`PGO_LOCKED`: a witness of either mode,
//!   kept) and [`uvn_get_sync`] (the write witness, consumed: the C returns unlocked).
//!   [`uvn_io`], [`uvn_put`] and [`uvn_flush`] borrow the write witness mutably: the lock let
//!   go and taken again inside (`WriteWitness::unlocked` around the I/O, `sleep` for the
//!   waits).
//! - The pager window of `uvn_io` is a `kernel::vm::pager::PagerWindow`: the tail past what the
//!   read returned is zeroed through its slice, and its drop is `uvm_pagermapout`.
//! - `uvm_vnp_sync` counts the uvns it takes with an atomic increment of `uo_refs`, where the
//!   C changes the count under the kernel lock alone (the other changes of a vnode object's
//!   count are under its lock).

use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64, Ordering};

use hal::arch::current::param::{PAGE_SHIFT, PAGE_SIZE};
use hal::bridges::AlreadyRegistered;
use hal::intr::{IPL_HIGH, IPL_NONE};
use hal::klock::{kernel_assert_locked, kernel_lock, kernel_unlock};
use hal::list::{List, ListEntry};
use hal::mutex::Mutexed;
use hal::netlock::{net_lock, net_lock_write_held, net_unlock};
use hal::once::OnceStatic;
use hal::pool::{PR_WAITOK, PoolArc, TypedPool};
use hal::rwlock::{RW_NOSLEEP, RW_WRITE, Rwlocked, Witness, WriteWitness};
use hal::sleep::{INFSLP, PVM, tsleep_nsec, wakeup};
use hal::sys::errno::Errno;
use hal::sys::mman::{PROT_NONE, PROT_READ, PROT_WRITE};
use hal::vm::param::{round_page, trunc_page};

use super::object::{ObjKind, ObjRef, Object, pagewait_relock, slot_page};
use super::page::{
    Owner, PG_BUSY, PG_CLEAN, PG_CLEANCHK, PG_FAKE, PG_WANTED, PQ_ACTIVE, PQ_INACTIVE, VmPage,
    uvm_lock_pageq, uvm_pagealloc, uvm_pagedeactivate, uvm_pagedequeue, uvm_pagefree,
    uvm_pagelookup, uvm_pagewait,
};
use super::pager::{
    MAXBSIZE, PGO_ALLPAGES, PGO_CLEANIT, PGO_DEACTIVATE, PGO_DOACTCLUST, PGO_FREE, PGO_LOCKED,
    PGO_NOWAIT, PGO_PDFREECLUST, PGO_SYNCIO, PageSlot, UVMPAGER_MAPIN_READ, UVMPAGER_MAPIN_WAITOK,
    UVMPAGER_MAPIN_WRITE, VM_PAGER_AGAIN, VM_PAGER_BAD, VM_PAGER_ERROR, VM_PAGER_OK, VM_PAGER_PEND,
    VM_PAGER_UNLOCK, uvm_pager_put, uvm_pagermapin, uvm_pagermapout,
};
use super::pmemrange::uvm_wait;
use super::types::{VmProt, Voff};
use crate::assert::{kassert, kpanic};
use crate::prf::{printf, unported};

// u_flags values

/// `UVM_VNODE_VALID`: we are attached to the vnode.
pub const UVM_VNODE_VALID: i32 = 0x001;
/// `UVM_VNODE_CANPERSIST`: we can persist after ref == 0.
pub const UVM_VNODE_CANPERSIST: i32 = 0x002;
/// `UVM_VNODE_DYING`: final detach/terminate in progress.
pub const UVM_VNODE_DYING: i32 = 0x008;
/// `UVM_VNODE_RELKILL`: uvn should be killed by releasepg when final i/o is done.
pub const UVM_VNODE_RELKILL: i32 = 0x010;
/// `UVM_VNODE_WANTED`: someone is waiting for alock, dying, or relkill to clear.
pub const UVM_VNODE_WANTED: i32 = 0x020;
/// `UVM_VNODE_VNISLOCKED`: underlying vnode struct is locked (valid when DYING is true).
pub const UVM_VNODE_VNISLOCKED: i32 = 0x040;
/// `UVM_VNODE_IOSYNC`: I/O sync in progress ... setter sleeps on `&uvn->u_nio`.
pub const UVM_VNODE_IOSYNC: i32 = 0x080;
/// `UVM_VNODE_IOSYNCWANTED`: a process is waiting for the i/o sync to clear so it can do i/o.
pub const UVM_VNODE_IOSYNCWANTED: i32 = 0x100;
/// `UVM_VNODE_WRITEABLE`: uvn has pages that are writeable.
pub const UVM_VNODE_WRITEABLE: i32 = 0x200;

/// `UVM_VNODE_BLOCKED`: any condition that should new processes from touching the vnode
/// \[set WANTED and sleep to wait for it to clear\].
pub const UVM_VNODE_BLOCKED: i32 = UVM_VNODE_DYING | UVM_VNODE_RELKILL;

/// `LK_EXCLUSIVE` (`<sys/lock.h>`): exclusive lock.
pub const LK_EXCLUSIVE: i32 = RW_WRITE;
/// `LK_NOWAIT`: do not sleep to await lock.
pub const LK_NOWAIT: i32 = RW_NOSLEEP;
/// `LK_RECURSEFAIL` (`RW_RECURSEFAIL`): fail if recursive.
pub const LK_RECURSEFAIL: i32 = 0x0080;
/// `LK_RETRY`: vn_lock: retry until locked.
pub const LK_RETRY: i32 = 0x2000;

/// `IO_NOCACHE` (`<sys/vnode.h>`): don't cache the data in the buffer cache.
pub const IO_NOCACHE: i32 = 0x40;

/// `struct uvm_vnode`: the vnode's handle into the VM system.
///
/// Locks: \[I\] immutable after creation, \[K\] kernel lock, \[S\] `uvn_sync_lock`, \[v\]
/// the object's lock.
pub struct Uvn {
    /// `u_obj`: the actual VM object.
    obj: Object,
    /// \[I\] `u_vnode`: pointer back to vnode.
    vnode: &'static dyn VnodeOps,
    /// \[v\] `u_flags`: flags.
    flags: AtomicI32,
    /// \[v\] `u_nio`: number of running I/O requests.
    nio: AtomicI32,
    /// \[v\] `u_size`: size of object.
    size: AtomicI64,
    /// \[K\] `u_wlist`: list of writeable vnode objs.
    wlist: ListEntry<Uvn>,
    /// \[S\] `u_syncq`: vnode objs due for a "sync".
    syncq: ListEntry<Uvn>,
}

hal::list_adapter!(
    /// `LIST_HEAD(, uvm_vnode) uvn_wlist`, through `u_wlist`.
    UvnWlist: Uvn, wlist
);

hal::list_adapter!(
    /// `SIMPLEQ_HEAD(, uvm_vnode) uvn_sync_q`, through `u_syncq`.
    UvnSyncq: Uvn, syncq
);

impl Uvn {
    /// A new `uvm_vnode` of `vp` (`pool_get(PR_ZERO)`, then `uvm_obj_init(&uvn->u_obj,
    /// &uvm_vnodeops, 0)` and `uvn->u_vnode = vp`): no references, a lock of its own.
    fn new(vp: &'static dyn VnodeOps) -> Self {
        Self {
            obj: Object::new(ObjKind::Vnode, 0),
            vnode: vp,
            flags: AtomicI32::new(0),
            nio: AtomicI32::new(0),
            size: AtomicI64::new(0),
            wlist: ListEntry::new(),
            syncq: ListEntry::new(),
        }
    }

    /// `u_obj`.
    pub fn object(&self) -> &Object {
        &self.obj
    }

    /// `u_vnode`.
    pub fn vnode(&self) -> &'static dyn VnodeOps {
        self.vnode
    }

    /// `u_flags`.
    pub fn flags(&self) -> i32 {
        self.flags.load(Ordering::Relaxed)
    }

    /// `u_flags = f`, under the object's lock held for writing.
    fn set_flags(&self, f: i32) {
        kassert!(self.obj.write_held());
        self.flags.store(f, Ordering::Relaxed);
    }

    /// `u_flags |= f`, under the lock.
    fn or_flags(&self, f: i32) {
        self.set_flags(self.flags() | f);
    }

    /// `u_flags &= ~f`, under the lock.
    fn clear_flags(&self, f: i32) {
        self.set_flags(self.flags() & !f);
    }

    /// `u_nio`.
    pub fn nio(&self) -> i32 {
        self.nio.load(Ordering::Relaxed)
    }

    /// `u_nio += delta`, under the lock.
    fn add_nio(&self, delta: i32) {
        kassert!(self.obj.write_held());
        self.nio.fetch_add(delta, Ordering::Relaxed);
    }

    /// `u_size`.
    pub fn size(&self) -> Voff {
        self.size.load(Ordering::Relaxed)
    }

    /// `u_size = size`, under the lock.
    fn set_size(&self, size: Voff) {
        kassert!(self.obj.write_held());
        self.size.store(size, Ordering::Relaxed);
    }

    /// The wait channel of `UVM_VNODE_WANTED` (the uvn's address).
    fn chan(&self) -> *const () {
        ptr::from_ref(self).cast()
    }

    /// The wait channel of `UVM_VNODE_IOSYNC` (`&uvn->u_nio`).
    fn nio_chan(&self) -> *const () {
        ptr::from_ref(&self.nio).cast()
    }

    /// The wait channel of `UVM_VNODE_IOSYNCWANTED` (`&uvn->u_flags`).
    fn flags_chan(&self) -> *const () {
        ptr::from_ref(&self.flags).cast()
    }
}

/// Which way `uvn_io` moves the bytes (`UIO_READ`, `UIO_WRITE`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum UioRw {
    /// `UIO_READ`: from the vnode into the pages.
    Read,
    /// `UIO_WRITE`: from the pages to the vnode.
    Write,
}

/// `enum vtype` (`<sys/vnode.h>`), as `mmap(2)` asks it of a vnode ([`VnodeOps::vtype`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)] // OpenBSD's names
pub enum VType {
    /// No type.
    VNON,
    /// Regular file.
    VREG,
    /// Directory.
    VDIR,
    /// Block device.
    VBLK,
    /// Character device.
    VCHR,
    /// Symbolic link.
    VLNK,
    /// Socket.
    VSOCK,
    /// Fifo.
    VFIFO,
    /// A vnode that has been cleaned out (`vgone`).
    VBAD,
}

/// `uvm_vnode_pool`.
static UVN_POOL: TypedPool<Uvn> = TypedPool::new();
/// `uvn_wlist`: \[K\] writeable uvns.
static UVN_WLIST: List<UvnWlist> = List::new();
/// `uvn_sync_q` inside `uvn_sync_lock`: \[S\] sync'ing uvns.
static UVN_SYNC: Rwlocked<List<UvnSyncq>> = Rwlocked::new("uvnsync", List::new());
/// `lasttime` of `uvn_flush`'s pageout error message (see the module's deviations).
static FLUSH_LASTTIME: Mutexed<(i64, i64)> = Mutexed::new(IPL_HIGH, (0, 0));

/// The [`SysInfo`] bridge, registered once by `bsd`'s `uvm_init` (the host tests' doubles).
static SYS_INFO: OnceStatic<&'static dyn SysInfo> = OnceStatic::new();

/// A vnode as the pager needs it (`struct vnode`, `bsd`'s until the vnode layer moves, N5):
/// implemented by `bsd`'s `Vnode`, passed as `&'static dyn VnodeOps` (vnodes are never freed)
/// and kept by the [`Uvn`]; each method is one call into the vnode layer (bridges.toml
/// `VnodeOps`). The methods that reach the vnode layer take `&'static self`: its functions
/// take `&'static Vnode`, and the pager only ever holds a vnode so.
pub trait VnodeOps: Sync {
    /// `vp->v_uvm`: the vnode's object, once attached.
    fn uvn(&self) -> Option<&'static Uvn>;
    /// `vp->v_uvm = uvn`.
    fn set_uvn(&self, uvn: &'static Uvn);
    /// `VOP_GETATTR(vp, &vattr, p->p_ucred, p)` for `curproc`: `va_size`.
    fn attr_size(&'static self) -> Result<u64, Errno>;
    /// `vp->v_type == VBLK`.
    fn is_blk(&self) -> bool;
    /// `vp->v_flag & VTEXT`: a running text.
    fn vtext(&self) -> bool;
    /// `vp->v_flag &= ~VTEXT`.
    fn clear_vtext(&self);
    /// `vp->v_mount`'s identity (its address as a number, compared and never followed; 0 for
    /// none).
    fn mount_id(&self) -> usize;
    /// `vref(vp)`.
    fn vref(&'static self);
    /// `vrele(vp)`.
    fn vrele(&'static self);
    /// `vget(vp, LK_NOWAIT)`: a reference, unless the vnode is being recycled.
    fn vget_nowait(&'static self) -> Result<(), Errno>;
    /// `vn_lock(vp, flags)`.
    fn lock(&'static self, flags: i32) -> Result<(), Errno>;
    /// `VOP_UNLOCK(vp)`.
    fn unlock(&'static self);
    /// `VOP_READ(vp, &uio, ioflag, curproc's cred)` of `buf.len()` bytes at `off` into `buf`
    /// (`UIO_SYSSPACE`): the bytes read (`len - uio_resid`). The vnode is locked.
    fn read(&'static self, off: Voff, buf: &mut [u8], ioflag: i32) -> Result<usize, Errno>;
    /// `VOP_WRITE(vp, &uio, ioflag, curproc's cred)` of `buf` at `off`: the bytes written. The
    /// vnode is locked.
    fn write(&'static self, off: Voff, buf: &[u8], ioflag: i32) -> Result<usize, Errno>;
    /// `vp->v_type` (`mmap(2)`'s checks, `uvm_mmapfile`).
    fn vtype(&self) -> VType;
    /// `iszerodev(vp->v_rdev)`: the device is `/dev/zero` (meaningful for a `VCHR` vnode).
    fn is_zero_device(&self) -> bool;
    /// `VOP_GETATTR(vp, &vattr, p->p_ucred, p)` for `curproc`: `va_flags` (`mmap(2)`'s
    /// `IMMUTABLE | APPEND` check).
    fn attr_flags(&'static self) -> Result<u64, Errno>;
}

/// The system's clock and its shutdown state, as the vnode pager needs them, `bsd`'s until
/// `kern_time` and `kern_xxx` move (N4): bridges.toml `SysInfo`, registered once
/// ([`install_sysinfo`]); each method is one read.
pub trait SysInfo: Sync {
    /// `getmicrouptime()`: the uptime as `(tv_sec, tv_usec)`.
    fn uptime(&self) -> (i64, i64);
    /// `rebooting`: the system is going down.
    fn rebooting(&self) -> bool;
    /// `&rebooting`, the wait channel of the processes that wait for the end.
    fn rebooting_chan(&self) -> *const ();
}

/// Registers the [`SysInfo`] bridge, once; a second call keeps the first and names it.
pub fn install_sysinfo(imp: &'static dyn SysInfo) -> Result<(), AlreadyRegistered> {
    SYS_INFO.set(imp).map_err(|_| AlreadyRegistered("SysInfo"))
}

/// `getmicrouptime()`: (0, 0) before the bridge is registered.
fn uptime() -> (i64, i64) {
    SYS_INFO.get().map_or((0, 0), |s| s.uptime())
}

/// `rebooting`: false before the bridge is registered.
fn rebooting() -> bool {
    SYS_INFO.get().is_some_and(|s| s.rebooting())
}

/// `ratecheck(lasttime, mininterval)` (`kern_time.c`): true, and `lasttime` set to now, when
/// at least `mininterval` has passed since `lasttime` or `lasttime` is (0, 0) (so the message
/// is seen at least once, even if interval is huge). The times are `(tv_sec, tv_usec)`.
fn ratecheck(lasttime: &Mutexed<(i64, i64)>, mininterval: (i64, i64)) -> bool {
    let tv = uptime();

    let mut last = lasttime.lock();
    // timersub(&tv, lasttime, &delta)
    let mut delta = (tv.0 - last.0, tv.1 - last.1);
    if delta.1 < 0 {
        delta = (delta.0 - 1, delta.1 + 1_000_000);
    }

    // check for 0,0 is so that the message will be seen at least once, even if interval is
    // huge.
    if delta >= mininterval || *last == (0, 0) {
        *last = tv;
        return true;
    }
    false
}

/// `uvn_init`: init pager private data structures: the pool `"uvmvnodes"` (`IPL_NONE`,
/// `PR_WAITOK`). `ENOMEM` when the pool cannot be made, `EBUSY` when it was. The write list
/// and the sync queue need nothing (`uvn_sync_q` is init'd in `uvm_vnp_sync()`).
pub fn uvn_init() -> Result<(), Errno> {
    UVN_POOL.init(IPL_NONE, PR_WAITOK, "uvmvnodes")
}

/// uvn_attach
///
/// attach a vnode structure to a VM object. if the vnode is already attached, then just bump
/// the reference count by one and return the VM object. if not already attached, attach and
/// return the new VM obj. the "accessprot" tells the max access the attaching thread wants to
/// our pages.
///
/// => in fact, nothing should be locked so that we can sleep here.
pub fn uvn_attach(vp: &'static dyn VnodeOps, accessprot: VmProt) -> Option<ObjRef> {
    // if we're mapping a BLK device, make sure it is a disk.
    if vp.is_blk() {
        // bdevsw[major(vp->v_rdev)].d_type != D_DISK, then the size from DIOCGPART through
        // d_ioctl (the disk label's d_secsize * DL_GETPSIZE): the device switch (conf.c).
        let _ = unported!("uvn_attach: bdevsw[].d_type, DIOCGPART (conf.c)");
        return None;
    }

    let used_vnode_size = vp.attr_size().ok()?;

    // make sure that the newsize fits within a vaddr_t
    // XXX: need to revise addressing data types

    if vp.uvn().is_none() {
        let Some(uvn) = UVN_POOL.get_shared(PR_WAITOK, || Uvn::new(vp)) else {
            // PR_WAITOK sleeps for an item; none means the pool was never made.
            return None;
        };
        kernel_assert_locked();
        if vp.uvn().is_none() {
            // The vnode keeps it for good.
            vp.set_uvn(PoolArc::leak(uvn));
        } // else the item goes back to the pool with the handle.
    }

    let uvn = vp.uvn()?;
    let lock = uvn.obj.lock();
    let mut w = lock.write();
    while uvn.flags() & UVM_VNODE_BLOCKED != 0 {
        uvn.or_flags(UVM_VNODE_WANTED);
        let _ = w.sleep(uvn.chan(), PVM, "uvn_attach");
    }

    // now uvn must not be in a blocked state. first check to see if it is already active, in
    // which case we can bump the reference count.
    if uvn.flags() & UVM_VNODE_VALID != 0 {
        // already active?
        // regain vref if we were persisting
        if uvn.obj.refs() == 0 {
            vp.vref();
        }
        uvn.obj.set_refs(uvn.obj.refs() + 1); // bump uvn ref!
    } else {
        // now set up the uvn.
        kassert!(uvn.obj.refs() == 0);
        uvn.obj.set_refs(uvn.obj.refs() + 1);
        uvn.set_flags(UVM_VNODE_VALID | UVM_VNODE_CANPERSIST);
        uvn.nio.store(0, Ordering::Relaxed);
        uvn.set_size(used_vnode_size as Voff);

        // this reference will stay as long as there is a valid mapping of the vnode. dropped
        // when the reference count goes to zero [and we either free or persist].
        vp.vref();
    }

    // if write access, we need to add it to the wlist
    if accessprot & PROT_WRITE != 0 && uvn.flags() & UVM_VNODE_WRITEABLE == 0 {
        uvn.or_flags(UVM_VNODE_WRITEABLE); // we are on wlist!
        kernel_assert_locked();
        UVN_WLIST.insert_head(uvn);
    }

    if uvn.flags() & UVM_VNODE_WANTED != 0 {
        wakeup(uvn.chan());
    }
    drop(w);

    Some(ObjRef::Vnode(uvn))
}

/// uvn_reference
///
/// duplicate a reference to a VM object. Note that the reference count must already be at
/// least one (the passed in reference) so there is no chance of the uvn being killed out
/// here.
///
/// => caller must be using the same accessprot as was used at attach time
pub fn uvn_reference(uvn: &Uvn) {
    let _w = uvn.obj.lock().write();
    // DEBUG: the UVM_VNODE_VALID check.
    uvn.obj.set_refs(uvn.obj.refs() + 1);
}

/// uvn_detach
///
/// remove a reference to a VM object.
///
/// => caller must call with map locked.
/// => this starts the detach process, but doesn't have to finish it (async i/o could still be
///    pending).
pub fn uvn_detach(uvn: &'static Uvn) {
    let uobj = &uvn.obj;
    let lock = uobj.lock();
    let mut w = lock.write();
    uobj.set_refs(uobj.refs() - 1); // drop ref!
    if uobj.refs() != 0 {
        // still more refs
        return;
    }

    kernel_lock();
    // get other pointers ...
    let vp = uvn.vnode;

    // clear VTEXT flag now that there are no mappings left (VTEXT is used to keep an active
    // text file from being overwritten).
    vp.clear_vtext();

    'out: {
        // we just dropped the last reference to the uvn. see if we can let it "stick
        // around".
        if uvn.flags() & UVM_VNODE_CANPERSIST != 0 {
            // won't block
            let _ = uvn_flush(uvn, &mut w, 0, 0, PGO_DEACTIVATE | PGO_ALLPAGES);
            break 'out;
        }

        // its a goner!
        uvn.or_flags(UVM_VNODE_DYING);

        // even though we may unlock in flush, no one can gain a reference to us until we
        // clear the "dying" flag [because it blocks attaches]. we will not do that until
        // after we've disposed of all the pages with uvn_flush(). note that before the flush
        // the only pages that could be marked PG_BUSY are ones that are in async pageout by
        // the daemon. (there can't be any pending "get"'s because there are no references to
        // the object).
        let _ = uvn_flush(uvn, &mut w, 0, 0, PGO_CLEANIT | PGO_FREE | PGO_ALLPAGES);

        // given the structure of this pager, the above flush request will create the
        // following state: all the pages that were in the object have either been free'd or
        // they are marked PG_BUSY and in the middle of an async io. If we still have pages
        // we set the "relkill" state, so that in the case the vnode gets terminated we know
        // to leave it alone. Otherwise we'll kill the vnode when it's empty.
        uvn.or_flags(UVM_VNODE_RELKILL);
        // wait on any outstanding io
        while uobj.npages() != 0 && uvn.flags() & UVM_VNODE_RELKILL != 0 {
            uvn.or_flags(UVM_VNODE_IOSYNC);
            let _ = w.sleep(uvn.nio_chan(), PVM, "uvn_term");
        }

        if uvn.flags() & UVM_VNODE_RELKILL == 0 {
            drop(w);
            kernel_unlock();
            return;
        }

        // kill object now. note that we can't be on the sync q because all references are
        // gone.
        if uvn.flags() & UVM_VNODE_WRITEABLE != 0 {
            UVN_WLIST.remove(uvn);
        }
        kassert!(uobj.pages().is_empty());
        let oldflags = uvn.flags();
        uvn.set_flags(0);

        // wake up any sleepers
        if oldflags & UVM_VNODE_WANTED != 0 {
            wakeup(uvn.chan());
        }
    }
    // out:
    drop(w);

    // drop our reference to the vnode.
    vp.vrele();
    kernel_unlock();
}

/// uvm_vnp_terminate: external hook to clear out a vnode's VM
///
/// called in two cases:
///  \[1\] when a persisting vnode vm object (i.e. one with a zero reference count) needs to
///      be freed so that a vnode can be reused. this happens under "getnewvnode" in
///      vfs_subr.c. if the vnode from the free list is still attached (i.e. not VBAD) then
///      vgone is called. as part of the vgone trace this should get called to free the vm
///      object. this is the common case.
///  \[2\] when a filesystem is being unmounted by force (MNT_FORCE, "umount -f") the vgone()
///      function is called on active vnodes on the mounted file systems to kill their data
///      (the vnodes become "dead" ones [see src/sys/miscfs/deadfs/...]). that results in a
///      call here (even if the uvn is still in use -- i.e. has a non-zero reference count).
///      this case happens at "umount -f" and during a "reboot/halt" operation.
///
/// => the caller must XLOCK and VOP_LOCK the vnode before calling us \[protects us from
///    getting a vnode that is already in the DYING state...\]
/// => in case \[2\] the uvn is still alive after this call, but all I/O ops will fail (due to
///    the backing vnode now being "dead"). this will prob. kill any process using the uvn due
///    to pgo_get failing.
pub fn uvm_vnp_terminate(vp: &'static dyn VnodeOps) {
    let Some(uvn) = vp.uvn() else {
        return;
    };
    let uobj = &uvn.obj;
    let lock = uobj.lock();
    let mut w = lock.write();
    if uvn.flags() & UVM_VNODE_VALID == 0 {
        return;
    }

    // must be a valid uvn that is not already dying (because XLOCK protects us from that).
    // DEBUG: the "terminating active vnode" printf.

    // it is possible that the uvn was detached and is in the relkill state [i.e. waiting for
    // async i/o to finish]. we take over the vnode now and cancel the relkill. we want to know
    // when the i/o is done so we can recycle right away. note that a uvn can only be in the
    // RELKILL state if it has a zero reference count.
    if uvn.flags() & UVM_VNODE_RELKILL != 0 {
        uvn.clear_flags(UVM_VNODE_RELKILL); // cancel RELKILL
    }

    // block the uvn by setting the dying flag, and then flush the pages.
    //
    // also, note that we tell I/O that we are already VOP_LOCK'd so that uvn_io doesn't
    // attempt to VOP_LOCK again.
    //
    // XXXCDC: setting VNISLOCKED on an active uvn which is being terminated due to a forceful
    // unmount might not be a good idea. maybe we need a way to pass in this info to uvn_flush
    // through a pager-defined PGO_ constant [currently there are none].
    uvn.or_flags(UVM_VNODE_DYING | UVM_VNODE_VNISLOCKED);

    let _ = uvn_flush(uvn, &mut w, 0, 0, PGO_CLEANIT | PGO_FREE | PGO_ALLPAGES);

    // as we just did a flush we expect all the pages to be gone or in the process of going.
    // sleep to wait for the rest to go [via iosync].
    while uobj.npages() != 0 {
        // DEBUG: the busy-page and u_nio checks.
        uvn.or_flags(UVM_VNODE_IOSYNC);
        let _ = w.sleep(uvn.nio_chan(), PVM, "uvn_term");
    }

    // done. now we free the uvn if its reference count is zero (true if we are zapping a
    // persisting uvn). however, if we are terminating a uvn with active mappings we let it
    // live ... future calls down to the vnode layer will fail.
    let oldflags = uvn.flags();
    if uobj.refs() != 0 {
        // uvn must live on it is dead-vnode state until all references are gone. restore
        // flags. clear CANPERSIST state.
        uvn.clear_flags(
            UVM_VNODE_DYING | UVM_VNODE_VNISLOCKED | UVM_VNODE_WANTED | UVM_VNODE_CANPERSIST,
        );
    } else {
        // free the uvn now. note that the vref reference is already gone [it is dropped when
        // we enter the persist state].
        if uvn.flags() & UVM_VNODE_IOSYNCWANTED != 0 {
            kpanic(format_args!("uvm_vnp_terminate: io sync wanted bit set"));
        }

        if uvn.flags() & UVM_VNODE_WRITEABLE != 0 {
            UVN_WLIST.remove(uvn);
        }
        uvn.set_flags(0); // uvn is history, clear all bits
    }

    if oldflags & UVM_VNODE_WANTED != 0 {
        wakeup(uvn.chan());
    }
}

/// The number of slots in `uvn_flush`'s cluster array (`MAXBSIZE >> PAGE_SHIFT`).
const FLUSH_PPS: usize = MAXBSIZE >> PAGE_SHIFT;

/// uvn_flush: flush pages out of a uvm object.
///
/// => object locked for writing (`w`), held across (let go and taken again for I/O and
///    waits)
/// => if PGO_CLEANIT is set, we may block (due to I/O). thus, a caller might want to unlock
///    higher level resources (e.g. vm_map) before calling flush.
/// => if PGO_CLEANIT is not set, then we will not block
/// => if PGO_ALLPAGE is set, then all pages in the object are valid targets for flushing.
/// => NOTE: we are allowed to lock the page queues, so the caller must not be holding the
///    lock on them [e.g. pagedaemon had better not call us with the queues locked]
/// => we return TRUE unless we encountered some sort of I/O error
///
/// comment on "cleaning" object and PG_BUSY pages: this routine is holding the lock on the
/// object. the only time that it can run into a PG_BUSY page that it does not own is if some
/// other process has started I/O on the page (e.g. either a pagein, or a pageout). if the
/// PG_BUSY page is being paged in, then it can not be dirty (!PG_CLEAN) because no one has had
/// a chance to modify it yet. if the PG_BUSY page is being paged out then it means that
/// someone else has already started cleaning the page for us (how nice!). in this case, if we
/// have syncio specified, then after we make our pass through the object we need to wait for
/// the other PG_BUSY pages to clear off (i.e. we need to do an iosync). also note that once a
/// page is PG_BUSY it must stay in its object until it is un-busyed.
pub fn uvn_flush(
    uvn: &'static Uvn,
    w: &mut WriteWitness<'_>,
    start: Voff,
    stop: Voff,
    flags: i32,
) -> bool {
    let uobj = &uvn.obj;
    let handle = ObjRef::Vnode(uvn);
    let mut flags = flags;
    let mut pps = [PageSlot::Empty; FLUSH_PPS];
    let ps = PAGE_SIZE as Voff;

    kassert!(uobj.write_held());

    // get init vals and determine how we are going to traverse object
    let mut need_iosync = false;
    let mut retval = true; // return value
    let end = round_page(uvn.size() as usize) as Voff;
    let (start, stop) = if flags & PGO_ALLPAGES != 0 {
        (0, end)
    } else {
        (
            trunc_page(start as usize) as Voff,
            (round_page(stop as usize) as Voff).min(end),
        )
    };

    // PG_CLEANCHK: this bit is used by the pgo_mk_pcluster function as a _hint_ as to how up
    // to date the PG_CLEAN bit is. if the hint is wrong it will only prevent us from
    // clustering... it won't break anything. we clear all PG_CLEANCHK bits here, and
    // pgo_mk_pcluster will set them as it syncs PG_CLEAN. This is only an issue if we are
    // looking at non-inactive pages (because inactive page's PG_CLEAN bit is always up to date
    // since there are no mappings). [borrowed PG_CLEANCHK idea from FreeBSD VM]
    if flags & PGO_CLEANIT != 0 {
        let mut curoff = start;
        while curoff < stop {
            if let Some(pp) = uvm_pagelookup(uobj, curoff) {
                pp.clear_bits(PG_CLEANCHK);
            }
            curoff += ps;
        }
    }

    // locked: both page queues (the guard; dropping it is uvm_unlock_pageq)
    let mut q = uvm_lock_pageq();
    let owner = Owner::Object(uobj);
    let mut curoff = start;
    while curoff < stop {
        let Some(pp) = uvm_pagelookup(uobj, curoff) else {
            curoff += ps;
            continue;
        };
        // handle case where we do not need to clean page (either because we are not clean or
        // because page is not dirty or is busy):
        //
        // NOTE: we are allowed to deactivate a non-wired active PG_BUSY page, but once a
        // PG_BUSY page is on the inactive queue it must stay put until it is !PG_BUSY (so as
        // not to confuse pagedaemon).
        let needs_clean = if flags & PGO_CLEANIT == 0 || pp.flags() & PG_BUSY != 0 {
            if pp.flags() & PG_BUSY != 0
                && flags & (PGO_CLEANIT | PGO_SYNCIO) == (PGO_CLEANIT | PGO_SYNCIO)
            {
                need_iosync = true;
            }
            false
        } else {
            // freeing: nuke all mappings so we can sync PG_CLEAN bit with no race
            if pp.flags() & PG_CLEAN != 0 && flags & PGO_FREE != 0 && pp.flags() & PQ_ACTIVE != 0 {
                hal::pmap::page_protect(&pp.mdpage, PROT_NONE);
            }
            if pp.flags() & PG_CLEAN != 0 && hal::pmap::is_modified(&pp.mdpage) {
                pp.clear_bits(PG_CLEAN);
            }
            pp.set_bits(PG_CLEANCHK);

            pp.flags() & PG_CLEAN == 0
        };

        // if we don't need a clean, deactivate/free pages then cont.
        if !needs_clean {
            if flags & PGO_DEACTIVATE != 0 {
                drop(q);
                uvm_pagedeactivate(pp, owner);
                q = uvm_lock_pageq();
            } else if flags & PGO_FREE != 0 {
                if pp.flags() & PG_BUSY != 0 {
                    drop(q);
                    pagewait_relock(pp, uobj, w, "uvn_flsh");
                    q = uvm_lock_pageq();
                    continue; // the same offset again
                } else {
                    hal::pmap::page_protect(&pp.mdpage, PROT_NONE);
                    // dequeue to prevent lock recursion
                    if pp.flags() & (PQ_ACTIVE | PQ_INACTIVE) != 0 {
                        uvm_pagedequeue(&mut q, pp, owner);
                    }
                    uvm_pagefree(pp, owner);
                }
            }
            curoff += ps;
            continue;
        }

        // pp points to a page in the object that we are working on. if it is
        // !PG_CLEAN,!PG_BUSY and we asked for cleaning (PGO_CLEANIT). we clean it now.
        //
        // let uvm_pager_put attempted a clustered page out. note: locked: page queues.
        let mut ppsp = 0;
        let mut npages;
        let result = loop {
            // ReTry:
            pp.set_bits(PG_BUSY);
            // UVM_PAGE_OWN(pp, "uvn_flush"): not configured.
            hal::pmap::page_protect(&pp.mdpage, PROT_READ);
            npages = FLUSH_PPS as i32;

            let result = uvm_pager_put(
                &handle,
                w,
                pp,
                &mut pps,
                &mut ppsp,
                &mut npages,
                flags | PGO_DOACTCLUST,
                start,
                stop,
                q,
            );

            // if we did an async I/O it is remotely possible for the async i/o to complete
            // and the page "pp" be freed or what not before we get a chance to relock the
            // object. Therefore, we only touch it when it won't be freed, RELEASED took care
            // of the rest.
            q = uvm_lock_pageq();

            // VM_PAGER_AGAIN: given the structure of this pager, this can only happen when we
            // are doing async I/O and can't map the pages into kernel memory (pager_map) due
            // to lack of vm space. if this happens we drop back to sync I/O.
            if result == VM_PAGER_AGAIN {
                kassert!(flags & PGO_SYNCIO == 0);
                flags |= PGO_SYNCIO;
                continue;
            }
            break result;
        };

        // the cleaning operation is now done. finish up. note that on error (!OK, !PEND)
        // uvm_pager_put drops the cluster for us. if success (OK, PEND) then uvm_pager_put
        // returns the cluster to us in ppsp/npages.

        // for pending async i/o if we are not deactivating we can move on to the next page.
        // aiodoned deals with the freeing case for us.
        if result == VM_PAGER_PEND && flags & PGO_DEACTIVATE == 0 {
            curoff += ps;
            continue;
        }

        // need to look at each page of the I/O operation, and do what we gotta do.
        for lcv in 0..npages as usize {
            let ptmp = slot_page(pps[ppsp + lcv], "uvn_flush");
            // verify the page didn't get moved
            if result == VM_PAGER_PEND && !ptmp.owned_by_object(uobj) {
                continue;
            }

            // unbusy the page if I/O is done. note that for pending I/O it is possible that
            // the I/O op finished (in which case the page is no longer busy).
            if result != VM_PAGER_PEND {
                if ptmp.flags() & PG_WANTED != 0 {
                    wakeup(ptmp.chan());
                }

                ptmp.clear_bits(PG_WANTED | PG_BUSY);
                // UVM_PAGE_OWN(ptmp, NULL): not configured.
                ptmp.set_bits(PG_CLEAN | PG_CLEANCHK);
                if flags & PGO_FREE == 0 {
                    let _ = hal::pmap::clear_modify(&ptmp.mdpage);
                }
            }

            // dispose of page
            if flags & PGO_DEACTIVATE != 0 {
                drop(q);
                uvm_pagedeactivate(ptmp, owner);
                q = uvm_lock_pageq();
            } else if flags & PGO_FREE != 0 && result != VM_PAGER_PEND {
                if result != VM_PAGER_OK {
                    if ratecheck(&FLUSH_LASTTIME, (5, 0)) {
                        printf(format_args!(
                            "uvn_flush: obj={:p}, offset={:#x}.  error during pageout.\n",
                            ptr::from_ref(uobj),
                            pp.offset()
                        ));
                        printf(format_args!(
                            "uvn_flush: WARNING: changes to page may be lost!\n"
                        ));
                    }
                    retval = false;
                }
                hal::pmap::page_protect(&ptmp.mdpage, PROT_NONE);
                // dequeue first to prevent lock recursion
                if ptmp.flags() & (PQ_ACTIVE | PQ_INACTIVE) != 0 {
                    uvm_pagedequeue(&mut q, ptmp, owner);
                }
                uvm_pagefree(ptmp, owner);
            }
        } // end of "lcv" for loop
        curoff += ps;
    } // end of "pp" for loop

    // done with pagequeues: unlock
    drop(q);

    // now wait for all I/O if required.
    if need_iosync {
        while uvn.nio() != 0 {
            uvn.or_flags(UVM_VNODE_IOSYNC);
            let _ = w.sleep(uvn.nio_chan(), PVM, "uvn_flush");
        }
        if uvn.flags() & UVM_VNODE_IOSYNCWANTED != 0 {
            wakeup(uvn.flags_chan());
        }
        uvn.clear_flags(UVM_VNODE_IOSYNC | UVM_VNODE_IOSYNCWANTED);
    }

    retval
}

/// uvn_cluster
///
/// we are about to do I/O in an object at offset. this function is called to establish a
/// range of offsets around "offset" in which we can cluster I/O. The object is locked (the
/// caller's witness, `kernel::vm::object::ObjRef::cluster`).
pub fn uvn_cluster(uvn: &Uvn, offset: Voff) -> (Voff, Voff) {
    let loffset = offset;

    kassert!(uvn.obj.write_held());

    if loffset >= uvn.size() {
        kpanic(format_args!("uvn_cluster: offset out of range"));
    }

    // XXX: old pager claims we could use VOP_BMAP to get maxcontig value.
    let end = round_page(uvn.size() as usize) as Voff;
    // past end?
    let hoffset = (loffset + MAXBSIZE as Voff).min(end);
    (loffset, hoffset)
}

/// uvn_put: flush page data to backing store.
///
/// => object locked for writing (`w`), let go around the I/O
/// => prefer map unlocked (not required)
/// => flags: PGO_SYNCIO -- use sync. I/O
/// => note: caller must set PG_CLEAN and pmap_clear_modify (if needed)
/// => XXX: currently we use VOP_READ/VOP_WRITE which are only sync. [thus we never do async
///    i/o! see iodone comment]
pub fn uvn_put(
    uvn: &Uvn,
    w: &mut WriteWitness<'_>,
    pps: &[PageSlot],
    npages: i32,
    flags: i32,
) -> i32 {
    kassert!(uvn.obj.write_held());

    // Unless we're recycling this vnode, grab a reference to it to prevent it from being
    // recycled from under our feet. This also makes sure we can don't panic if we end up in
    // uvn_vnp_uncache() as a result of the I/O operation as that function assumes we hold a
    // reference.
    //
    // If the vnode is in the process of being recycled by someone else, grabbing a reference
    // will fail. In that case the pages will already be written out by whoever is cleaning
    // the vnode, so simply return VM_PAGER_AGAIN such that we skip these pages.
    let dying = uvn.flags() & UVM_VNODE_DYING != 0;
    if !dying && uvn.vnode.vget_nowait().is_err() {
        return VM_PAGER_AGAIN;
    }

    let retval = uvn_io(uvn, w, &pps[..npages as usize], flags, UioRw::Write);

    if !dying {
        uvn.vnode.vrele();
    }

    retval
}

/// uvn_get with PGO_LOCKED: get the pages that are already resident and unbusy; the object
/// is locked in either mode (`w`) and stays locked.
///
/// => NOTE: offset is the offset of pps[0], _NOT_ pps[centeridx]
/// => NOTE: caller must check for released pages!!
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn uvn_get_locked(
    uvn: &Uvn,
    w: &impl Witness,
    offset: Voff,
    pps: &mut [PageSlot],
    npagesp: &mut i32,
    centeridx: i32,
    _advice: i32,
    flags: i32,
) -> i32 {
    let uobj = &uvn.obj;
    let ps = PAGE_SIZE as Voff;
    uobj.check_witness(w.lock_id(), "uvn_get");
    kassert!(flags & PGO_LOCKED != 0);

    // step 1: handled the case where fault data structures are locked.
    //
    // gotpages is the current number of pages we've gotten (which we pass back up to caller
    // via *npagesp.
    let mut gotpages = 0;

    // step 1a: get pages that are already resident. only do this if the data structures are
    // locked (i.e. the first time through).
    let mut done = true; // be optimistic

    kassert!(pps.len() >= *npagesp as usize);
    for (lcv, slot) in pps.iter_mut().enumerate().take(*npagesp as usize) {
        let off = offset + lcv as Voff * ps;
        // do we care about this page? if not, skip it
        if slot.is_dontcare() {
            continue;
        }

        // lookup page; to be useful must get a non-busy page
        let Some(ptmp) = uvm_pagelookup(uobj, off).filter(|p| p.flags() & PG_BUSY == 0) else {
            if lcv as i32 == centeridx {
                // need to do a wait or I/O!
                done = false;
            }
            if flags & PGO_ALLPAGES != 0 {
                done = false;
                break;
            }
            continue;
        };

        // useful page: busy it and plug it in our result array
        *slot = PageSlot::Page(ptmp);
        gotpages += 1;
    }

    // XXX: given the "advice", should we consider async read-ahead? XXX: fault current does
    // deactivate of pages behind us. is this good (other callers might now).
    //
    // XXX: read-ahead currently handled by buffer cache (bread) level. XXX: no async i/o
    // available. XXX: so we don't do anything now.

    // step 1c: now we've either done everything needed or we to unlock and do some waiting
    // or I/O.
    *npagesp = gotpages; // let caller know
    if done { VM_PAGER_OK } else { VM_PAGER_UNLOCK }
}

/// uvn_get without PGO_LOCKED: get pages (synchronously) from backing store. The object is
/// locked for writing (`w`) and comes back unlocked.
///
/// => prefer map unlocked (not required)
/// => flags: PGO_ALLPAGES: get all of the pages
/// => NOTE: offset is the offset of pps[0], _NOT_ pps[centeridx]
/// => NOTE: caller must check for released pages!!
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
pub fn uvn_get_sync(
    uvn: &Uvn,
    w: WriteWitness<'_>,
    offset: Voff,
    pps: &mut [PageSlot],
    npagesp: &mut i32,
    centeridx: i32,
    _access_type: VmProt,
    _advice: i32,
    flags: i32,
) -> i32 {
    let uobj = &uvn.obj;
    let lock = uobj.lock();
    let ps = PAGE_SIZE as Voff;
    uobj.check_witness(w.lock_id(), "uvn_get");
    kassert!(uobj.write_held());
    kassert!(flags & PGO_LOCKED == 0);

    let owner = Owner::Object(uobj);
    let mut w = w;
    // step 2: get non-resident or busy pages. data structures are unlocked.
    //
    // XXX: because we can't do async I/O at this level we get things page at a time
    // (otherwise we'd chunk). the VOP_READ() will do async-read-ahead for us at a lower level.
    kassert!(pps.len() >= *npagesp as usize);
    for (lcv, slot) in pps.iter_mut().enumerate().take(*npagesp as usize) {
        let off = offset + lcv as Voff * ps;

        // skip over pages we've already gotten or don't want
        // skip over pages we don't _have_ to get
        if !slot.is_empty() || (lcv as i32 != centeridx && flags & PGO_ALLPAGES == 0) {
            continue;
        }

        // we have yet to locate the current page (pps[lcv]). we first look for a page that is
        // already at the current offset. if we fine a page, we check to see if it is busy or
        // released. if that is the case, then we sleep on the page until it is no longer busy
        // or released and repeat the lookup. if the page we found is neither busy nor
        // released, then we busy it (so we own it) and plug it into pps[lcv]. this breaks the
        // following while loop and indicates we are ready to move on to the next page in the
        // "lcv" loop above.
        //
        // if we exit the while loop with pps[lcv] still empty, then it means that we allocated
        // a new busy/fake/clean page ptmp in the object and we need to do I/O to fill in the
        // data.
        let ptmp: &'static VmPage = loop {
            // top of "pps" while loop
            // look for a current page
            let Some(ptmp) = uvm_pagelookup(uobj, off) else {
                // nope? allocate one now (if we can)
                let Some(ptmp) = uvm_pagealloc(owner, off, 0) else {
                    // out of RAM?
                    uvm_wait("uvn_getpage");
                    // goto top of pps while loop
                    continue;
                };
                // got new page ready for I/O. break pps while loop. pps[lcv] is still empty.
                break ptmp;
            };

            // page is there, see if we need to wait on it
            if ptmp.flags() & PG_BUSY != 0 {
                uvm_pagewait(ptmp, owner, w, "uvn_get");
                w = lock.write();
                continue; // goto top of pps while loop
            }

            // if we get here then the page has become resident and unbusy between steps 1 and
            // 2. we busy it now (so we own it) and set pps[lcv] (so that we exit the while
            // loop).
            ptmp.set_bits(PG_BUSY);
            // UVM_PAGE_OWN(ptmp, "uvn_get2"): not configured.
            *slot = PageSlot::Page(ptmp);
            break ptmp;
        };

        // if we own the a valid page at the correct offset, pps[lcv] will point to it.
        // nothing more to do except go to the next page.
        if !slot.is_empty() {
            continue; // next lcv
        }

        // we have a "fake/busy/clean" page that we just allocated. do I/O to fill it with
        // valid data.
        let result = uvn_io(
            uvn,
            &mut w,
            &[PageSlot::Page(ptmp)],
            PGO_SYNCIO | PGO_NOWAIT,
            UioRw::Read,
        );

        // I/O done. because we used syncio the result can not be PEND or AGAIN.
        if result != VM_PAGER_OK {
            if ptmp.flags() & PG_WANTED != 0 {
                wakeup(ptmp.chan());
            }

            ptmp.clear_bits(PG_WANTED | PG_BUSY);
            // UVM_PAGE_OWN(ptmp, NULL): not configured.
            uvm_pagefree(ptmp, owner);
            drop(w);
            return result;
        }

        // we got the page! clear the fake flag (indicates valid data now in page) and plug
        // into our result array. note that page is still busy.
        //
        // it is the callers job to:
        // => check if the page is released
        // => unbusy the page
        // => activate the page

        // data is valid ...
        ptmp.clear_bits(PG_FAKE);
        let _ = hal::pmap::clear_modify(&ptmp.mdpage); // ... and clean
        *slot = PageSlot::Page(ptmp);
    }

    drop(w);
    VM_PAGER_OK
}

/// uvn_io: do I/O to a vnode
///
/// => object locked for writing (`w`); let go during the I/O, taken again before returning
/// => prefer map unlocked (not required)
/// => flags: PGO_SYNCIO -- use sync. I/O
/// => XXX: currently we use VOP_READ/VOP_WRITE which are only sync. [thus we never do async
///    i/o! see iodone comment]
fn uvn_io(uvn: &Uvn, w: &mut WriteWitness<'_>, pps: &[PageSlot], flags: i32, rw: UioRw) -> i32 {
    let lkflags = if flags & PGO_NOWAIT != 0 {
        LK_NOWAIT
    } else {
        0
    };
    let npages = pps.len();

    kassert!(uvn.obj.write_held());

    // init values
    let waitok = flags & PGO_SYNCIO != 0;
    let vn = uvn.vnode;
    let file_offset = slot_page(pps[0], "uvn_io").offset();

    // check for sync'ing I/O.
    while uvn.flags() & UVM_VNODE_IOSYNC != 0 {
        if !waitok {
            return VM_PAGER_AGAIN;
        }
        uvn.or_flags(UVM_VNODE_IOSYNCWANTED);
        let _ = w.sleep(uvn.flags_chan(), PVM, "uvn_iosync");
    }

    // check size
    if file_offset >= uvn.size() {
        return VM_PAGER_BAD;
    }

    // first try and map the pages in (without waiting)
    let mapinflags = if rw == UioRw::Read {
        UVMPAGER_MAPIN_READ
    } else {
        UVMPAGER_MAPIN_WRITE
    };

    let kva = uvm_pagermapin(pps, mapinflags);
    if kva.is_none() && !waitok {
        return VM_PAGER_AGAIN;
    }

    // ok, now bump u_nio up. at this point we are done with uvn and can unlock it. if we
    // still don't have a kva, try again (this time with sleep ok).
    uvn.add_nio(1); // we have an I/O in progress!
    let vnlocked = uvn.flags() & UVM_VNODE_VNISLOCKED != 0;
    let uvnsize = uvn.size();
    let result = w.unlocked(|| {
        let mut kva = match kva {
            Some(k) => k,
            None => match uvm_pagermapin(pps, mapinflags | UVMPAGER_MAPIN_WAITOK) {
                Some(k) => k,
                None => kpanic(format_args!("uvn_io: no pager window")),
            },
        };

        // ok, mapped in. our pages are PG_BUSY so they are not going to get touched (so we
        // can look at "offset" without having to lock the object). set up for I/O.
        let mut wanted = npages << PAGE_SHIFT;
        if file_offset + wanted as Voff > uvnsize {
            wanted = (uvnsize - file_offset) as usize; // XXX: needed?
        }

        // This process may already have the NET_LOCK(), if we faulted in copyin() or
        // copyout() in the network stack.
        let netunlocked = net_lock_write_held();
        if netunlocked {
            net_unlock();
        }

        // do the I/O! (XXX: curproc?)
        //
        // This process may already have this vnode locked, if we faulted in copyin() or
        // copyout() on a region backed by this vnode while doing I/O to the vnode. If this is
        // the case, don't panic.. instead, return the error to the user.
        //
        // XXX this is a stopgap to prevent a panic. Ideally, this kind of operation *should*
        // work.
        kernel_lock();
        let mut result = Ok(0);
        if !vnlocked {
            result = vn.lock(LK_EXCLUSIVE | LK_RECURSEFAIL | lkflags).map(|()| 0);
        }
        if result.is_ok() {
            // NOTE: vnode now locked!
            result = if rw == UioRw::Read {
                vn.read(file_offset, &mut kva.as_slice_mut()[..wanted], 0)
            } else {
                let ioflag = if flags & PGO_PDFREECLUST != 0 {
                    IO_NOCACHE
                } else {
                    0
                };
                vn.write(file_offset, &kva.as_slice()[..wanted], ioflag)
            };

            if !vnlocked {
                vn.unlock();
            }
        }
        kernel_unlock();

        if netunlocked {
            net_lock();
        }

        // NOTE: vnode now unlocked (unless vnislocked)
        //
        // result == unix style errno (0 == OK!)
        //
        // zero out rest of buffer (if needed)
        let result = result.and_then(|got| {
            if wanted != 0 && got == 0 {
                Err(Errno::EIO) // XXX: error?
            } else {
                if got < PAGE_SIZE * npages && rw == UioRw::Read {
                    kva.as_slice_mut()[got..].fill(0);
                }
                Ok(())
            }
        });

        // now remove pager mapping
        uvm_pagermapout(kva);
        result
    });

    // now clean up the object (i.e. drop I/O count)
    uvn.add_nio(-1); // I/O DONE!
    if uvn.flags() & UVM_VNODE_IOSYNC != 0 && uvn.nio() == 0 {
        wakeup(uvn.nio_chan());
    }

    match result {
        Ok(()) => VM_PAGER_OK,
        Err(Errno::EBUSY) => {
            kassert!(flags & PGO_NOWAIT != 0);
            VM_PAGER_AGAIN
        }
        Err(_) => {
            if rebooting() {
                kernel_lock();
                while rebooting() {
                    let chan = SYS_INFO.get().map_or(ptr::null(), |s| s.rebooting_chan());
                    let _ = tsleep_nsec(chan, PVM, "uvndead", INFSLP);
                }
                kernel_unlock();
            }
            VM_PAGER_ERROR
        }
    }
}

/// uvm_vnp_uncache: disable "persisting" in a vnode... when last reference is gone we will
/// kill the object (flushing dirty pages back to the vnode if needed).
///
/// => returns TRUE if there was no uvm_object attached or if there was one and we killed it
///    [i.e. if there is no active uvn]
/// => called with the vnode VOP_LOCK'd [we will unlock it for I/O, if needed]
///
/// => XXX: given that we now kill uvn's when a vnode is recycled (without having to hold a
///    reference on the vnode) and given a working uvm_vnp_sync(), how does that effect the
///    need for this function? [XXXCDC: seems like it can die?]
///
/// => XXX: this function should DIE once we merge the VM and buffer cache.
///
/// research shows that this is called in the following places: ext2fs_truncate,
/// ffs_truncate, detrunc[msdosfs]: called when vnode changes sizes; ext2fs_write, WRITE
/// [ufs_readwrite], msdosfs_write: called when we are written to; ex2fs_chmod, ufs_chmod:
/// called if VTEXT vnode and the sticky bit is off; ffs_realloccg: when we can't extend the
/// current block and have to allocate a new one we call this [XXX: why?]; nfsrv_rename,
/// rename_files: called when the target filename is there and we want to remove it;
/// nfsrv_remove, sys_unlink: called on file we are removing; nfsrv_access: if VTEXT and we
/// want WRITE access and we don't uncache then return "text busy"; nfs_open: seems to uncache
/// any file opened with nfs; vn_writechk: if VTEXT vnode and can't uncache return "text
/// busy"; fusefs_open: uncaches any file that is opened; fusefs_write: uncaches on every
/// write.
pub fn uvm_vnp_uncache(vp: &'static dyn VnodeOps) -> bool {
    let Some(uvn) = vp.uvn() else {
        return true;
    };
    let uobj = &uvn.obj;
    let w = uobj.lock().write();
    if uvn.flags() & UVM_VNODE_VALID == 0 || uvn.flags() & UVM_VNODE_BLOCKED != 0 {
        return true;
    }

    // we have a valid, non-blocked uvn. clear persist flag. if uvn is currently active we can
    // return now.
    uvn.clear_flags(UVM_VNODE_CANPERSIST);
    if uobj.refs() != 0 {
        return false;
    }

    // uvn is currently persisting! we have to gain a reference to it so that we can call
    // uvn_detach to kill the uvn.
    vp.vref(); // seems ok, even with VOP_LOCK
    uobj.set_refs(uobj.refs() + 1); // value is now 1
    drop(w);

    // VFSLCKDEBUG: the "vnode not locked" check is not configured.

    // now drop our reference to the vnode. if we have the sole reference to the vnode then
    // this will cause it to die [as we just cleared the persist flag]. we have to unlock the
    // vnode while we are doing this as it may trigger I/O.
    //
    // XXX: it might be possible for uvn to get reclaimed while we are unlocked causing us to
    // return TRUE when we should not. we ignore this as a false-positive return value doesn't
    // hurt us.
    vp.unlock();
    uvn_detach(uvn);
    let _ = vp.lock(LK_EXCLUSIVE | LK_RETRY);

    true
}

/// uvm_vnp_setsize: grow or shrink a vnode uvn
///
/// grow   => just update size value
/// shrink => toss un-needed pages
///
/// => we assume that the caller has a reference of some sort to the vnode in question so that
///    it will not be yanked out from under us.
///
/// called from:
///  => truncate fns (ext2fs_truncate, ffs_truncate, detrunc[msdos], fusefs_setattr)
///  => "write" fns (ext2fs_write, WRITE [ufs/ufs], msdosfs_write, nfs_write fusefs_write)
///  => ffs_balloc [XXX: why? doesn't WRITE handle?]
///  => NFS: nfs_loadattrcache, nfs_getattrcache, nfs_setattr
///  => union fs: union_newsize
pub fn uvm_vnp_setsize(vp: &'static dyn VnodeOps, newsize: Voff) {
    kernel_assert_locked();
    let Some(uvn) = vp.uvn() else {
        return;
    };
    let mut w = uvn.obj.lock().write();
    if uvn.flags() & UVM_VNODE_VALID != 0 {
        // now check if the size has changed: if we shrink we had better toss some pages...
        if uvn.size() > newsize {
            let _ = uvn_flush(uvn, &mut w, newsize, uvn.size(), PGO_FREE);
        }
        uvn.set_size(newsize);
    }
}

/// uvm_vnp_sync: flush all dirty VM pages back to their backing vnodes.
///
/// => called from sys_sync with no VM structures locked
/// => only one process can do a sync at a time (because the uvn structure only has one queue
///    for sync'ing). we ensure this by holding the uvn_sync_lock while the sync is in
///    progress. other processes attempting a sync will sleep on this lock until we are done.
pub fn uvm_vnp_sync(mp: Option<usize>) {
    // step 1: ensure we are only ones using the uvn_sync_q by locking our lock...
    let syncq = UVN_SYNC.write();

    // step 2: build up a simpleq of uvns of interest based on the write list. we gain a
    // reference to uvns of interest.
    for uvn in UVN_WLIST.iter() {
        let vp = uvn.vnode;
        if let Some(mp) = mp
            && vp.mount_id() != mp
        {
            continue;
        }

        // If the vnode is "blocked" it means it must be dying, which in turn means its in the
        // process of being flushed out so we can safely skip it.
        if uvn.flags() & UVM_VNODE_BLOCKED != 0 {
            continue;
        }

        // gain reference. watch out for persisting uvns (need to regain vnode REF).
        if uvn.obj.refs() == 0 {
            vp.vref();
        }
        uvn.obj.refs_inc();

        syncq.insert_head(uvn);
    }

    // step 3: we now have a list of uvn's that may need cleaning.
    while let Some(uvn) = syncq.first() {
        syncq.remove(uvn);
        let mut w = uvn.obj.lock().write();
        // DEBUG: the "dying vnode on sync list" printf.
        let _ = uvn_flush(
            uvn,
            &mut w,
            0,
            0,
            PGO_CLEANIT | PGO_ALLPAGES | PGO_DOACTCLUST,
        );

        // if we have the only reference and we just cleaned the uvn, then we can pull it out
        // of the UVM_VNODE_WRITEABLE state thus allowing us to avoid thinking about flushing
        // it again on later sync ops.
        if uvn.obj.refs() == 1 && uvn.flags() & UVM_VNODE_WRITEABLE != 0 {
            UVN_WLIST.remove(uvn);
            uvn.clear_flags(UVM_VNODE_WRITEABLE);
        }
        drop(w);

        // now drop our reference to the uvn
        uvn_detach(uvn);
    }

    drop(syncq);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the vnode pager over a vnode double whose reads and writes work on an
    // in-memory file (`testutil::MemVnode`): attaching, paging in (with the tail of the last
    // page zeroed), cleaning a dirty page back to the file through `uvm_vnp_sync`, persisting
    // after the last detach, shrinking with `uvm_vnp_setsize`, `uvm_vnp_uncache` and
    // `uvm_vnp_terminate`; the three of LZ's `uvm_vnode.rs` with equal assertions, then the
    // locks `uvn_io` lets go of and an I/O error.

    use std::sync::MutexGuard;
    use std::vec::Vec;

    use super::*;
    use crate::testutil::{self, MemVnode};

    /// Memory, the lock objects, the pager pools, the kernel lock held, a file of `len` bytes
    /// (`i % 251`) and its vnode.
    fn setup(len: usize) -> (MutexGuard<'static, ()>, &'static MemVnode) {
        let g = testutil::pages_install(256 * 4096);
        crate::testutil::init_shared_once();
        let _ = uvn_init();
        kernel_lock();
        let vp = testutil::mem_vnode((0..len).map(|i| (i % 251) as u8).collect());
        (g, vp)
    }

    fn teardown() {
        kernel_unlock();
    }

    /// The vnode object of `obj`.
    fn uvn(obj: &ObjRef) -> &'static Uvn {
        obj.uvn().expect("a vnode object")
    }

    /// The page's bytes, copied out through a view of it (the page is busy).
    fn bytes(pg: &VmPage) -> Vec<u8> {
        let view = super::super::km::mapper().map_direct(pg.phys_addr());
        view.as_slice().to_vec()
    }

    /// Writes `data` at the start of the busy page.
    fn write_bytes(pg: &VmPage, data: &[u8]) {
        let mut view = super::super::km::mapper().map_direct(pg.phys_addr());
        view.as_slice_mut()[..data.len()].copy_from_slice(data);
    }

    /// `pgo_get` of the page at `off` without the fault's locks: busy, resident.
    fn get(obj: &ObjRef, off: Voff) -> &'static VmPage {
        let mut pps = [PageSlot::Empty; 1];
        let mut npages = 1;
        let w = obj.lock().write();
        let r = obj.get_sync(w, off, &mut pps, &mut npages, 0, PROT_READ, 0, 0);
        assert_eq!(r, VM_PAGER_OK);
        pps[0].page().expect("a page")
    }

    #[test]
    fn attach_get_dirty_sync_and_persist() {
        let (_g, vp) = setup(PAGE_SIZE + 100);
        let use0 = vp.usecount();

        let uobj = uvn_attach(vp, PROT_READ | PROT_WRITE).expect("attached");
        let uvn = uvn(&uobj);
        assert!(vp.uvn().is_some_and(|u| ptr::eq(u, uvn)));
        assert_eq!(uobj.refs(), 1);
        assert_eq!(uvn.size(), (PAGE_SIZE + 100) as Voff);
        assert!(uvn.flags() & UVM_VNODE_VALID != 0);
        assert!(uvn.flags() & UVM_VNODE_WRITEABLE != 0);
        assert_eq!(vp.usecount(), use0 + 1);
        // A second attach shares the object.
        let again = uvn_attach(vp, PROT_READ).expect("attached");
        assert!(ObjRef::ptr_eq(&again, &uobj));
        assert_eq!(uobj.refs(), 2);
        again.detach();

        // With the fault's locks nothing is resident yet.
        let mut pps = [PageSlot::Empty; 2];
        let mut npages = 2;
        let w = uobj.lock().write();
        let r = uobj.get(&w, 0, &mut pps, &mut npages, 0, PROT_READ, 0, PGO_LOCKED);
        drop(w);
        assert_eq!((r, npages), (VM_PAGER_UNLOCK, 0));

        // Paging in reads the file; the second page holds 100 bytes and zeroes.
        let pg0 = get(&uobj, 0);
        assert!(pg0.flags() & PG_BUSY != 0 && pg0.flags() & PG_FAKE == 0);
        assert_eq!(bytes(pg0)[..8], [0, 1, 2, 3, 4, 5, 6, 7]);
        let pg1 = get(&uobj, PAGE_SIZE as Voff);
        let b1 = bytes(pg1);
        assert_eq!(b1[99], ((PAGE_SIZE + 99) % 251) as u8);
        assert!(b1[100..].iter().all(|&b| b == 0));
        assert_eq!(uobj.npages(), 2);

        // Dirty the first page and let the sync write it back.
        write_bytes(pg0, b"EMI!");
        pg0.clear_bits(PG_CLEAN | PG_BUSY);
        pg1.clear_bits(PG_BUSY);
        uvm_vnp_sync(None);
        assert_eq!(vp.contents()[..4], *b"EMI!");
        assert_eq!(vp.contents().len(), PAGE_SIZE + 100);
        assert!(pg0.flags() & PG_CLEAN != 0 && pg0.flags() & PG_BUSY == 0);
        assert_eq!(uobj.refs(), 1);

        // The last detach keeps the object (it persists) and drops the vnode reference.
        uobj.detach();
        assert_eq!(uobj.refs(), 0);
        assert!(uvn.flags() & UVM_VNODE_VALID != 0);
        assert_eq!(uobj.npages(), 2);
        assert_eq!(vp.usecount(), use0);

        // Re-attaching finds the cached pages.
        let uobj = uvn_attach(vp, PROT_READ).expect("attached");
        assert_eq!(uobj.refs(), 1);
        assert!(uvm_pagelookup(&*uobj, 0).is_some_and(|p| ptr::eq(p, pg0)));

        // Shrinking tosses the pages from the one holding the new end (the C truncates the
        // start to a page).
        uvm_vnp_setsize(vp, PAGE_SIZE as Voff + 10);
        assert_eq!(uobj.npages(), 1);
        assert!(uvm_pagelookup(&*uobj, PAGE_SIZE as Voff).is_none());
        assert_eq!(uvn.size(), PAGE_SIZE as Voff + 10);

        // Uncaching an active object only stops it persisting.
        assert!(!uvm_vnp_uncache(vp));
        assert!(uvn.flags() & UVM_VNODE_CANPERSIST == 0);
        uobj.detach();
        // Without CANPERSIST the last detach kills the object and frees its pages.
        assert_eq!(uvn.flags(), 0);
        assert_eq!(uobj.npages(), 0);
        assert_eq!(vp.usecount(), use0);
        teardown();
    }

    #[test]
    fn terminate_frees_a_persisting_object() {
        let (_g, vp) = setup(3 * PAGE_SIZE);
        let uobj = uvn_attach(vp, PROT_READ).expect("attached");
        let pg = get(&uobj, PAGE_SIZE as Voff);
        pg.clear_bits(PG_BUSY);
        uobj.detach();
        let uvn = uvn(&uobj);
        assert!(uvn.flags() & UVM_VNODE_VALID != 0);
        assert_eq!(uobj.npages(), 1);

        uvm_vnp_terminate(vp);
        assert_eq!(uvn.flags(), 0);
        assert_eq!(uobj.npages(), 0);
        // A dead object is not synced and cannot be uncached.
        uvm_vnp_sync(None);
        assert!(uvm_vnp_uncache(vp));
        teardown();
    }

    #[test]
    fn a_page_past_the_end_is_bad() {
        let (_g, vp) = setup(100);
        let uobj = uvn_attach(vp, PROT_READ).expect("attached");
        let mut pps = [PageSlot::Empty; 1];
        let mut npages = 1;
        let w = uobj.lock().write();
        let r = uobj.get_sync(
            w,
            PAGE_SIZE as Voff,
            &mut pps,
            &mut npages,
            0,
            PROT_READ,
            0,
            0,
        );
        // uvn_io refuses an offset past the end; uvn_get freed the page and dropped the lock.
        assert_eq!(r, VM_PAGER_BAD);
        assert_eq!(uobj.npages(), 0);
        assert!(!uobj.lock_held());
        let w = uobj.lock().write();
        let (lo, hi) = uobj.cluster(&w, 0);
        drop(w);
        assert_eq!((lo, hi), (0, PAGE_SIZE as Voff));
        uobj.detach();
        teardown();
    }

    #[test]
    fn a_read_lets_go_of_the_object_and_net_locks_and_holds_the_kernel_lock() {
        let (_g, vp) = setup(10);
        let uobj = uvn_attach(vp, PROT_READ).expect("attached");
        testutil::net_lock_set(true);
        let pg = get(&uobj, 0);
        let seen = vp.last_read().expect("a VOP_READ");
        assert!(
            !seen.object_locked,
            "the object's lock is let go for the VOP"
        );
        assert!(!seen.net_locked, "the net lock is let go for the VOP");
        assert!(seen.kernel_locked && seen.vnode_locked);
        assert!(testutil::net_lock_held(), "and taken again");
        assert_eq!(vp.nlocked(), 0, "the vnode is unlocked after");
        let b = bytes(pg);
        assert_eq!(b[..10], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert!(b[10..].iter().all(|&x| x == 0), "the tail is zeroed");
        assert_eq!(uvn(&uobj).nio(), 0);
        testutil::net_lock_set(false);
        let w = uobj.lock().write();
        super::super::page::uvm_page_unbusy(&[Some(pg)], Owner::Object(&*uobj));
        drop(w);
        uobj.detach();
        teardown();
    }

    #[test]
    fn a_read_error_frees_the_page_and_is_a_pager_error() {
        let (_g, vp) = setup(2 * PAGE_SIZE);
        let uobj = uvn_attach(vp, PROT_READ).expect("attached");
        vp.fail_io(true);
        let mut pps = [PageSlot::Empty; 1];
        let mut npages = 1;
        let w = uobj.lock().write();
        let r = uobj.get_sync(w, 0, &mut pps, &mut npages, 0, PROT_READ, 0, 0);
        assert_eq!(r, VM_PAGER_ERROR);
        assert!(pps[0].is_empty());
        assert_eq!(uobj.npages(), 0);
        vp.fail_io(false);
        uobj.detach();
        teardown();
    }

    #[test]
    fn ratecheck_lets_one_message_through_per_interval() {
        testutil::install();
        static LAST: Mutexed<(i64, i64)> = Mutexed::new(IPL_HIGH, (0, 0));
        // (0, 0) passes at once, whatever the clock says.
        assert!(ratecheck(&LAST, (5, 0)));
        // A second check within the interval does not.
        assert!(!ratecheck(&LAST, (3600, 0)));
        assert!(ratecheck(&LAST, (0, 0)));
    }

    #[test]
    fn a_flush_deactivating_a_page_under_read_keeps_the_pagers_view() {
        let (_g, vp) = setup(PAGE_SIZE);
        let uobj = uvn_attach(vp, PROT_READ).expect("attached");
        let u = uvn(&uobj);
        let flushed = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
        let seen = std::sync::Arc::clone(&flushed);
        // Inside VOP_READ (the page busy, held by the pager's view, the object unlocked),
        // another process deactivates the range (madvise(MADV_DONTNEED), the fault's
        // MADV_SEQUENTIAL flush): uvn_flush deactivates the busy page, whose page_protect
        // spares the view (the C deactivates it harmlessly).
        vp.during_read(move || {
            let mut w = u.object().lock().write();
            let _ = uvn_flush(u, &mut w, 0, PAGE_SIZE as Voff, PGO_DEACTIVATE);
            drop(w);
            seen.store(true, core::sync::atomic::Ordering::Relaxed);
        });
        let pg = get(&uobj, 0);
        assert!(flushed.load(core::sync::atomic::Ordering::Relaxed));
        assert!(pg.flags() & PQ_INACTIVE != 0, "deactivated while busy");
        // The read went on through the view.
        assert_eq!(bytes(pg)[..4], [0, 1, 2, 3]);
        let w = uobj.lock().write();
        pg.clear_bits(PG_BUSY);
        drop(w);
        uobj.detach();
        teardown();
    }
}
/* </TESTS> */
