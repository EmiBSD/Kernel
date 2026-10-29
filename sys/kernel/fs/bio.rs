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
 * Copyright (c) 1994 Christopher G. Demetriou
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)vfs_bio.c	8.6 (Berkeley) 1/11/94
 */
/*
 * Copyright (c) 2014 Ted Unangst <tedu@openbsd.org>
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
//! The buffer cache: `bread`/`bwrite` and their friends over the per-vnode buffers, the
//! cleaner thread (`buf_daemon`) that writes delayed buffers out, the 2Q cache of clean
//! buffers (hot, warm and cold queues) that decides which ones to throw away, and the vnode's
//! buffer half (`vfs_subr.c`'s `bgetvp`, `brelvp`, `buf_replacevnode`, `reassignbuf`,
//! `vwaitforio`, `vwakeup`).
//!
//! Upstream: sys/kern/vfs_bio.c @ 3ce1f3f79392
//! Upstream: sys/kern/vfs_subr.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_bio.rs@3c62ede68802
//! LZ: sys/kern/vfs_subr.rs@3c62ede68802
//!
//! Some references:
//! - Bach: The Design of the UNIX Operating System (Prentice Hall, 1986)
//! - Leffler, et al.: The Design and Implementation of the 4.3BSD UNIX Operating System
//!   (Addison Welley, 1989)
//!
//! ## How a file system uses it
//! - `let (t, error) = bread(vb, lblkno, size)`: the buffer's token comes back in both cases,
//!   as the C's `*bpp` does; on error the caller `brelse`s it (or drops it: a token dropped
//!   is `brelse`d).
//! - `getblk(vb, lblkno, size, 0, INFSLP)` for a block it will overwrite; `bwrite`, `bawrite`
//!   or `bdwrite` to write it (they consume the token), `brelse` to give it back unchanged.
//! - The file system's strategy maps `b_lblkno` to `b_blkno` and hands the token to its device
//!   vnode's strategy, whose driver calls `biodone` when the transfer is over.
//!
//! **Locks.** The C's `splbio()` domain is one lock, [`CACHE`] (a `Mutexed` at `IPL_BIO`,
//! which `biodone` from interrupt context takes too): `bufhead`, the 2Q queues, the dirty
//! queue, `buf_valist`, the arena, and every vnode's tree and lists ([`VnodeBufs`], a cell
//! opened by the cache's guard), and each buffer's busy state, `b_pobj` and `b_iodone` (a
//! cell of the same lock, `kernel::fs::buf`). The kernel lock is held by every caller, as in
//! the C. Where the C sleeps at `splbio`
//! (`getblk` on a busy buffer, `biowait`, `buf_get` for buffers, `buf_map` for kva,
//! `vwaitforio`) the cache's lock is left for the sleep and taken again (`msleep`); a sleeping
//! caller holds no counted link to a buffer it does not own. The strategy routines and the
//! completion callbacks run without the cache's lock (the C calls `VOP_STRATEGY` after
//! `splx`).
//!
//! ## Deviations
//! - `bread`, `breadn` and `bread_cluster` return the buffer's token with the result,
//!   `(BufRef, Result<(), Errno>)`, instead of filling `*bpp`: the C always fills it.
//! - `bcstats` is [`BCSTATS`], `struct bcachestats` as atomics with a `snapshot()` for
//!   `vfs.generic.bcachestat` (as `nchstats`); the tunables (`lodirtypages`, `targetpages`,
//!   ...) are atomics, written under the cache's lock.
//! - `getblk` and `buf_get` return `Option` (the C's NULL); `incore` is a `bool`.
//! - The cleaner's `TRACEPOINT`s (`dt(4)`) are not configured, nor is `HIBERNATE`
//!   (`hibernate_suspend_bufcache`); the DDB printer `bcstats_print` waits for the ddb
//!   command loop (`db_command.c`).
//! - `pool_get(PR_WAITOK)` and `uvm_pagealloc_multi(UVM_PLA_WAITOK)` cannot sleep yet:
//!   `buf_get` returns NULL when the pool fails, which `getblk` retries as the C does after a
//!   sleep, and `buf_alloc_pages` panics as the C does when even the waiting allocation fails
//!   (LZ's).
//! - `bwrite` learns the disk queue its write went on from a note it claims for the buffer
//!   before `VOP_STRATEGY` (written by `bufq_queue`, marked done by `bufq_done`: the C's set
//!   and clear of `b_bq`), not by reading `bp->b_bq` after it: the buffer may be another
//!   owner's by then (the C reads it anyway). A `bwrite` beyond the table's 32 notes sleeps
//!   for one.
//! - `curproc->p_ru.ru_inblock++`/`ru_oublock++` go through the vnode's [`BufVnode`]
//!   (`charge_io`), which reaches the running thread.
//!
//! ## Redesign
//! - LZ's `bufhead`, `cleancache`'s queues, `dirtyqueue` and `buf_valist` (`Sync` statics of
//!   `queue_adapter!` lists over `&'static Buf`, each insert and remove an `unsafe` block) and
//!   the scattered `splbio()`/`splx()` pairs -> [`CacheState`] inside one `Mutexed`
//!   ([`CACHE`]): `bufhead` an `OwnedList` that owns every buffer (a `bufpl` item), the queues
//!   `SecondaryList`s of counted members, so a buffer is freed (`buf_put`, `buf_unmap`) only
//!   once no queue, tree, list or token holds it.
//! - The vnode's `[B]` half (`v_bufs_tree`, `v_cleanblkhd`, `v_dirtyblkhd`, `v_numoutput`,
//!   `v_bioflag`, LZ's `bsd` fields) -> [`VnodeBufs`], embedded in `bsd`'s vnode, its tree and
//!   lists a cell of the cache's lock; what the cache asks of the vnode itself (`vhold`,
//!   `vdrop`, `VOP_STRATEGY`, `VOP_BWRITE`, `VOP_BMAP`, its type and device, its mount's
//!   flags and counters, the syncer's worklist) is the bridge [`BufVnode`] (bridges.toml,
//!   paid by N5.2). `bgetvp`, `brelvp`, `buf_replacevnode`, `reassignbuf`, `vwaitforio` and
//!   `vwakeup` move here from `vfs_subr.c`'s port.
//! - `B_BUSY` is the token (`kernel::fs::buf::BufRef`): `getblk`, `bread`, `geteblk` give
//!   it, `brelse`, `bwrite`, `bdwrite`, `bawrite` and `biodone` consume it. A synchronous
//!   I/O's caller keeps a counted link while the driver holds the token (`biowait`'s); the
//!   driver's `biodone` parks the token in the buffer and `biowait` takes it back.
//! - `bread_cluster`'s `xbpp` array (`malloc(M_TEMP)`, `NULL`-terminated, hung from
//!   `b_saveaddr`) is gone: the read-ahead buffers are the busy buffers of the following
//!   blocks whose pages are the first buffer's (`Pages::Master`), which the callback finds in
//!   one walk of the vnode's tree from the first buffer, taking their parked tokens in the
//!   same section of the cache's lock; the set-up keeps the tokens in a stack array (the C's
//!   `xbpp`) while it fills the buffers in one section, then parks them.
//! - `cleanerproc`/`syncerproc` identities are thread words ([`set_syncer`], the cleaner's
//!   own at its start), compared with `curproc`'s.

use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, AtomicUsize, Ordering as Mem};

use hal::intr::IPL_BIO;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::owned::{Member, OwnedList, SecondaryList, SecondaryRef, SecondaryTree};
use hal::pool::{PR_NOWAIT, PR_WAITOK};
use hal::rwlock::Guarded;
use hal::sleep::{INFSLP, PRIBIO, curproc_word, sched_pause, wakeup};
use hal::sys::errno::Errno;
use hal::sys::mount::Bcachestats;
use hal::sys::param::PAGE_SIZE;
use hal::sys::types::{Daddr, Dev};
use hal::vm::param::{atop, round_page};
use hal::vm::uvmexp::UVMEXP;

#[cfg(any(test, feature = "doubles"))]
use super::biomem::BUF_NEEDVA;
use super::biomem::{
    buf_acquire, buf_acquire_nomap, buf_alloc_pages, buf_dealloc_mem, buf_fix_mapping, buf_map,
    buf_mem_init, buf_release,
};
use super::buf::{
    B_AGE, B_ASYNC, B_BC, B_BUSY, B_CACHE, B_CALL, B_COLD, B_DEFERRED, B_DELWRI, B_DONE, B_EINTR,
    B_ERROR, B_INVAL, B_NEEDCOMMIT, B_NOCACHE, B_PHYS, B_RAW, B_READ, B_WANTED, B_WARM,
    B_WRITEINPROG, BCACHE_MIN, BUFPL, Buf, BufRef, ByBufhead, ByFreelist, ByLblkno, ByValist,
    ByVnbufs, Iodone, MAXPHYS, Pages, RESERVE_PAGES, RESERVE_SLOTS,
};
use crate::assert::{kassert, kpanic};
use crate::vm::km::{NO_CONSTRAINT, VM_KERNEL_SPACE_SIZE};
use crate::vm::page::{Owner, uvm_pagecount, uvm_pagelookup, uvm_pagerealloc};
use crate::vm::types::Voff;
use crate::vm::vnode::VType;

/// `NODEV` (`<sys/param.h>`): no device.
pub const NODEV: Dev = -1;

/// `DEV_BSHIFT` (`<sys/param.h>`): log2 of `DEV_BSIZE`.
pub const DEV_BSHIFT: u32 = 9;

/// `VBIOWAIT` (`v_bioflag`): waiting for output to complete.
pub const VBIOWAIT: u32 = 0x0001;
/// `VBIOONSYNCLIST`: vnode is on syncer worklist.
pub const VBIOONSYNCLIST: u32 = 0x0002;
/// `VBIOONFREELIST`: buffer list is on freelist.
pub const VBIOONFREELIST: u32 = 0x0004;
/// `VBIOERROR`: a write error occurred.
pub const VBIOERROR: u32 = 0x0008;

/// `bcstats`: the buffer cache counters (`struct bcachestats`), as atomics.
pub struct Bcstats {
    /// `numbufs`: number of buffers allocated.
    pub numbufs: AtomicI64,
    /// `numbufpages`: number of pages in buffer cache.
    pub numbufpages: AtomicI64,
    /// `numdirtypages`: number of dirty free pages.
    pub numdirtypages: AtomicI64,
    /// `numcleanpages`: number of clean free pages.
    pub numcleanpages: AtomicI64,
    /// `pendingwrites`: number of pending writes.
    pub pendingwrites: AtomicI64,
    /// `pendingreads`: number of pending reads.
    pub pendingreads: AtomicI64,
    /// `numwrites`: total writes started.
    pub numwrites: AtomicI64,
    /// `numreads`: total reads started.
    pub numreads: AtomicI64,
    /// `cachehits`: total reads found in cache.
    pub cachehits: AtomicI64,
    /// `busymapped`: number of busy and mapped buffers.
    pub busymapped: AtomicI64,
    /// `delwribufs`: delayed write buffers.
    pub delwribufs: AtomicI64,
    /// `kvaslots`: kva slots total.
    pub kvaslots: AtomicI64,
    /// `kvaslots_avail`: available kva slots.
    pub kvaslots_avail: AtomicI64,
}

impl Bcstats {
    /// Every counter zero.
    pub const fn new() -> Self {
        Self {
            numbufs: AtomicI64::new(0),
            numbufpages: AtomicI64::new(0),
            numdirtypages: AtomicI64::new(0),
            numcleanpages: AtomicI64::new(0),
            pendingwrites: AtomicI64::new(0),
            pendingreads: AtomicI64::new(0),
            numwrites: AtomicI64::new(0),
            numreads: AtomicI64::new(0),
            cachehits: AtomicI64::new(0),
            busymapped: AtomicI64::new(0),
            delwribufs: AtomicI64::new(0),
            kvaslots: AtomicI64::new(0),
            kvaslots_avail: AtomicI64::new(0),
        }
    }

    /// The counters as the `struct bcachestats` `vfs.generic.bcachestat` copies out.
    pub fn snapshot(&self) -> Bcachestats {
        let r = |c: &AtomicI64| c.load(Mem::Relaxed);
        Bcachestats {
            numbufs: r(&self.numbufs),
            numbufpages: r(&self.numbufpages),
            numdirtypages: r(&self.numdirtypages),
            numcleanpages: r(&self.numcleanpages),
            pendingwrites: r(&self.pendingwrites),
            pendingreads: r(&self.pendingreads),
            numwrites: r(&self.numwrites),
            numreads: r(&self.numreads),
            cachehits: r(&self.cachehits),
            busymapped: r(&self.busymapped),
            delwribufs: r(&self.delwribufs),
            kvaslots: r(&self.kvaslots),
            kvaslots_avail: r(&self.kvaslots_avail),
        }
    }

    /// Every counter back to zero (a host test's fresh cache).
    #[cfg(any(test, feature = "doubles"))]
    fn reset(&self) {
        for c in [
            &self.numbufs,
            &self.numbufpages,
            &self.numdirtypages,
            &self.numcleanpages,
            &self.pendingwrites,
            &self.pendingreads,
            &self.numwrites,
            &self.numreads,
            &self.cachehits,
            &self.busymapped,
            &self.delwribufs,
            &self.kvaslots,
            &self.kvaslots_avail,
        ] {
            c.store(0, Mem::Relaxed);
        }
    }
}

impl Default for Bcstats {
    fn default() -> Self {
        Self::new()
    }
}

/// A vnode's buffers (the `[B]` fields of `struct vnode`): its tree by logical block and its
/// clean and dirty lists, a cell of the cache's lock; the writes in progress and the flags
/// interrupts read. Embedded in `bsd`'s vnode, which lives for ever (vnodes are never freed,
/// only recycled); its owner, the vnode itself, is set once ([`VnodeBufs::set_owner`]).
pub struct VnodeBufs {
    /// The vnode, as the cache asks things of it.
    owner: OnceStatic<&'static dyn BufVnode>,
    /// `v_bufs_tree`, `v_cleanblkhd`, `v_dirtyblkhd`: under the cache's lock.
    st: Guarded<VbState>,
    /// `v_numoutput`: writes in progress (changed under the cache's lock).
    numoutput: AtomicU32,
    /// `v_bioflag`: `VBIO*` flags accessed in interrupts.
    bioflag: AtomicU32,
}

/// The vnode's tree and lists of buffers.
pub(crate) struct VbState {
    /// `v_bufs_tree`: lookup of all bufs, by logical block.
    tree: SecondaryTree<ByLblkno>,
    /// `v_cleanblkhd`: clean blocklist head.
    clean: SecondaryList<ByVnbufs>,
    /// `v_dirtyblkhd`: dirty blocklist head.
    dirty: SecondaryList<ByVnbufs>,
}

impl VbState {
    const fn new() -> Self {
        Self {
            tree: SecondaryTree::new(),
            clean: SecondaryList::new(),
            dirty: SecondaryList::new(),
        }
    }
}

impl VnodeBufs {
    /// A vnode's empty buffer half, as `getnewvnode` makes one.
    pub const fn new() -> Self {
        Self {
            owner: OnceStatic::new(),
            st: Guarded::new_static_mutexed(&CACHE, VbState::new()),
            numoutput: AtomicU32::new(0),
            bioflag: AtomicU32::new(0),
        }
    }

    /// The vnode this half belongs to, set once when the vnode is made (a recycled vnode
    /// keeps it: the same vnode).
    pub fn set_owner(&self, v: &'static dyn BufVnode) {
        let _ = self.owner.set(v);
    }

    /// The vnode; `panic(9)` before [`VnodeBufs::set_owner`].
    pub fn owner(&self) -> &'static dyn BufVnode {
        match self.owner.get() {
            Some(v) => *v,
            None => kpanic(format_args!("VnodeBufs {:p}: no vnode", self)),
        }
    }

    /// `vp->v_numoutput`.
    pub fn numoutput(&self) -> u32 {
        self.numoutput.load(Mem::Relaxed)
    }

    /// `vp->v_numoutput++`: a write started on the vnode outside the cache (a driver's own
    /// buffer sent down a vnode: `softraid`, `vnd`).
    pub fn numoutput_inc(&self) {
        let _g = CACHE.lock();
        self.numoutput.fetch_add(1, Mem::Relaxed);
    }

    /// `vp->v_bioflag`.
    pub fn bioflag(&self) -> u32 {
        self.bioflag.load(Mem::Relaxed)
    }

    /// `SET(vp->v_bioflag, f)`.
    pub fn set_bioflag(&self, f: u32) {
        self.bioflag.fetch_or(f, Mem::Relaxed);
    }

    /// `CLR(vp->v_bioflag, f)`.
    pub fn clr_bioflag(&self, f: u32) {
        self.bioflag.fetch_and(!f, Mem::Relaxed);
    }

    /// `LIST_EMPTY(&vp->v_dirtyblkhd)`.
    pub fn dirty_is_empty(&self) -> bool {
        let g = CACHE.lock();
        self.st.get(&g).dirty.is_empty()
    }

    /// `LIST_EMPTY(&vp->v_cleanblkhd)`.
    pub fn clean_is_empty(&self) -> bool {
        let g = CACHE.lock();
        self.st.get(&g).clean.is_empty()
    }

    /// `RB_EMPTY(&vp->v_bufs_tree)`: the vnode holds no buffer.
    pub fn tree_is_empty(&self) -> bool {
        let g = CACHE.lock();
        self.st.get(&g).tree.is_empty()
    }
}

impl Default for VnodeBufs {
    fn default() -> Self {
        Self::new()
    }
}

/// What the buffer cache asks of a vnode (`bsd`'s `struct vnode` until N5.2): one call each,
/// implemented by `bsd`'s vnode (bridges.toml `BufVnode`, paid by N5.2).
pub trait BufVnode: Sync {
    /// `vhold(vp)`: the vnode gets a buffer reference (off the free list onto the hold list
    /// when it had none and is unused).
    fn vhold(&'static self);
    /// `vdrop(vp)`: the vnode loses a buffer reference.
    fn vdrop(&'static self);
    /// `VOP_STRATEGY(vp, bp)`: starts the I/O of the buffer `bp` (its token passes to the
    /// file system and on to the driver, whose `biodone` gives it back).
    fn strategy(&'static self, bp: BufRef) -> Result<(), Errno>;
    /// `VOP_BWRITE(bp)` of the buffer's vnode (`bawrite`).
    fn bwrite(&'static self, bp: BufRef) -> Result<(), Errno>;
    /// `VOP_BMAP(vp, bn, NULL, &bnp, &runp)`: the device block of `bn` and how many blocks
    /// follow it contiguously.
    fn bmap(&'static self, bn: Daddr) -> Result<(Daddr, i32), Errno>;
    /// `vp->v_type`.
    fn vtype(&self) -> VType;
    /// `vp->v_rdev` (a device vnode's).
    fn rdev(&self) -> Dev;
    /// Whether the mount the vnode's writes are charged to (`v_specmountpoint` of a block
    /// device, else `v_mount`) has `MNT_ASYNC` (`bwrite` makes a synchronous write a delayed
    /// one).
    fn mnt_async(&self) -> bool;
    /// `mp->mnt_stat.f_{sync,async}{reads,writes}++` of that mount, when there is one.
    fn count_io(&self, write: bool, async_: bool);
    /// `curproc->p_ru.ru_inblock++` (`ru_oublock` for `output`), when a thread runs.
    fn charge_io(&self, output: bool);
    /// `reassignbuf`'s `vn_syncer_add_to_worklist(vp, delay)` of a vnode getting its first
    /// dirty buffer: `syncdelay / 2` for a directory, `/ 3` for a mounted block device, else
    /// `syncdelay` (the worklist sets `VBIOONSYNCLIST`).
    fn syncer_add_dirty(&'static self);
    /// `LIST_REMOVE(vp, v_synclist)`: off the syncer's worklist (`VBIOONSYNCLIST` cleared by
    /// the caller).
    fn syncer_remove(&'static self);
    /// The vnode as `bsd` knows it, for `bsd`'s adapter (`bp->b_vp`).
    fn as_any(&'static self) -> &'static dyn core::any::Any;
}

/// The buffer cache's state, all under [`CACHE`] (the C's `splbio`).
pub struct CacheState {
    /// `bufhead`: all allocated buffers, owned here (and the `B_RELEASED` ones waiting on
    /// `buf_valist` for `buf_unmap`).
    pub(super) bufhead: OwnedList<ByBufhead>,
    /// `cleancache.hotqueue`.
    hot: SecondaryList<ByFreelist>,
    /// `cleancache.warmqueue`.
    warm: SecondaryList<ByFreelist>,
    /// `cleancache.coldqueue`.
    cold: SecondaryList<ByFreelist>,
    /// `cleancache.hotbufpages`.
    hotbufpages: i64,
    /// `cleancache.warmbufpages`.
    warmbufpages: i64,
    /// `cleancache.cachepages`.
    cachepages: i64,
    /// `dirtyqueue`: the delayed-write buffers, oldest first.
    dirty: SecondaryList<ByFreelist>,
    /// `buf_valist`: idle buffers keeping kva, least recently used first.
    pub(super) valist: SecondaryList<ByValist>,
    /// The kva arena (`buf_kva_start`, `buf_kva_end`), from `buf_mem_init`.
    pub(super) arena: Option<crate::vm::km::BufArena>,
}

impl CacheState {
    const fn new() -> Self {
        Self {
            bufhead: OwnedList::new(),
            hot: SecondaryList::new(),
            warm: SecondaryList::new(),
            cold: SecondaryList::new(),
            hotbufpages: 0,
            warmbufpages: 0,
            cachepages: 0,
            dirty: SecondaryList::new(),
            valist: SecondaryList::new(),
            arena: None,
        }
    }
}

/// How many `bwrite`s may be between their strategy call and their `bufq_wait` at once; one
/// more sleeps for a free note.
const QUEUE_NOTES: usize = 32;

/// A note's word: the buffer's address with this bit set once its I/O is done (`bufq_done`:
/// the C's `bp->b_bq = NULL`). A `Buf` is aligned to 8, so the bit is never an address's.
const NOTE_DONE: usize = 1;

/// A `bwrite`'s note of the disk queue its buffer went on. The C reads `bp->b_bq` after
/// `VOP_STRATEGY`, when the buffer is the driver's (and an asynchronous write's may be done
/// and reused): here the token went with the buffer, so `bwrite` claims a note for the
/// buffer's address before the strategy call, `bufq_queue` writes the queue into it, and
/// `bufq_done` marks it done, as the C clears `b_bq`.
struct QueueNote {
    /// The buffer's address (with [`NOTE_DONE`]), 0 for a free note.
    word: AtomicUsize,
    /// The queue `bufq_queue` put the buffer on; written and read by the claiming thread only.
    bq: hal::link::Link<super::bufq::Bufq>,
}

impl QueueNote {
    const fn new() -> Self {
        Self {
            word: AtomicUsize::new(0),
            bq: hal::link::Link::new(),
        }
    }
}

/// The cache's lock held: what the C's `splbio()` section is.
pub type Cache = MutexedGuard<'static, CacheState>;

/// The buffer cache (the C's `splbio()` domain), at `IPL_BIO`.
pub static CACHE: Mutexed<CacheState> = Mutexed::new(IPL_BIO, CacheState::new());

/// `bcstats`: counters.
pub static BCSTATS: Bcstats = Bcstats::new();
/// `lodirtypages`: dirty page count low water mark.
pub static LODIRTYPAGES: AtomicI64 = AtomicI64::new(0);
/// `hidirtypages`: dirty page count high water mark.
pub static HIDIRTYPAGES: AtomicI64 = AtomicI64::new(0);
/// `targetpages`: target number of pages for cache size.
pub static TARGETPAGES: AtomicI64 = AtomicI64::new(0);
/// `buflowpages`: smallest size cache allowed.
pub static BUFLOWPAGES: AtomicI64 = AtomicI64::new(0);
/// `bufhighpages`: largest size cache allowed.
pub static BUFHIGHPAGES: AtomicI64 = AtomicI64::new(0);
/// `bufbackpages`: minimum number of pages we shrink when asked to.
pub static BUFBACKPAGES: AtomicI64 = AtomicI64::new(0);
/// `bufkvm`: the kernel virtual space for mapping buffers (0: let `bufinit` decide).
pub static BUFKVM: AtomicUsize = AtomicUsize::new(0);
/// `bufpages`: max number of pages for buffers' data (`param.c`'s `BUFPAGES`, 0: let
/// `bufinit` decide).
pub static BUFPAGES: AtomicI64 = AtomicI64::new(0);
/// `BUFCACHEPERCENT` (`param.c`): the default share of memory for the buffer cache.
pub const BUFCACHEPERCENT_DEFAULT: i32 = 20;
/// `bufcachepercent`: the cache's share of DMA-reachable memory (`param.c`'s
/// `BUFCACHEPERCENT`; 0: `bufinit` takes 10).
pub static BUFCACHEPERCENT: AtomicI32 = AtomicI32::new(BUFCACHEPERCENT_DEFAULT);
/// `nobuffers`: the syncer or the cleaner sleeps for buffers (its address the channel).
pub static NOBUFFERS: AtomicI32 = AtomicI32::new(0);
/// `needbuffer`: threads sleeping for the cache to shrink (its address the channel).
pub static NEEDBUFFER: AtomicI32 = AtomicI32::new(0);
/// `bd_req`: sleep point for cleaner daemon.
pub static BD_REQ: AtomicI32 = AtomicI32::new(0);
/// `cleanerproc`: the cleaner thread's word (0 before it runs).
static CLEANER: AtomicUsize = AtomicUsize::new(0);
/// `syncerproc`: the syncer thread's word (0 before it runs).
static SYNCER: AtomicUsize = AtomicUsize::new(0);
/// The notes of the `bwrite`s in their strategy call (keyed by buffer, so a strategy routine
/// that writes another buffer itself, `vnd`'s, has its own; nothing global to save).
static QUEUE_NOTE: [QueueNote; QUEUE_NOTES] = [const { QueueNote::new() }; QUEUE_NOTES];
/// How many notes are claimed: `bufq_queue` and `bufq_done` look only when some is.
static NOTES_CLAIMED: AtomicUsize = AtomicUsize::new(0);
/// Threads sleeping for a free note (its address the channel).
static NOTES_WANTED: AtomicUsize = AtomicUsize::new(0);

/// `cleanerproc` as a thread word.
pub(crate) fn cleaner_word() -> usize {
    CLEANER.load(Mem::Relaxed)
}

/// `syncerproc` as a thread word.
pub(crate) fn syncer_word() -> usize {
    SYNCER.load(Mem::Relaxed)
}

/// `syncerproc = curproc`: the syncer thread says who it is (`bsd`'s `syncer_thread`).
pub fn set_syncer(word: usize) {
    SYNCER.store(word, Mem::Relaxed);
}

/// `UNCLEAN_PAGES`: the cache's pages that are not clean.
pub fn unclean_pages() -> i64 {
    BCSTATS.numbufpages.load(Mem::Relaxed) - BCSTATS.numcleanpages.load(Mem::Relaxed)
}

/// `BUFPAGES_DEFICIT`: how far the buffer cache is below its low water mark, in pages.
pub fn bufpages_deficit() -> i64 {
    let d = BUFLOWPAGES.load(Mem::Relaxed) - BCSTATS.numbufpages.load(Mem::Relaxed);
    d.max(0)
}

/// `BUFPAGES_INACT`: the cache's clean pages above its low water mark.
pub fn bufpages_inact() -> i64 {
    let i = BCSTATS.numcleanpages.load(Mem::Relaxed) - BUFLOWPAGES.load(Mem::Relaxed);
    i.max(0)
}

/// A counted link to `bp` (a buffer some container of the cache owns), which the caller
/// keeps while it lets the borrow it found `bp` through end.
fn counted(bp: &Buf) -> SecondaryRef<ByBufhead> {
    let mut r = SecondaryRef::new();
    r.set(Some(bp));
    r
}

/// The `bcstats` counter `c` plus `d`.
fn bc_add(c: &AtomicI64, d: i64) {
    c.fetch_add(d, Mem::Relaxed);
}

/// `bp->b_vp`, which a buffer on its way to the disk has.
fn b_vp(bp: &Buf) -> &'static VnodeBufs {
    match bp.vnode_bufs() {
        Some(vb) => vb,
        None => kpanic(format_args!("buf {:p}: no b_vp", bp)),
    }
}

/// `buf_put`: frees a buffer that is on no list (its memory first; the header goes back to
/// `bufpl`, or later, from `buf_unmap`, when `buf_dealloc_mem` keeps it for its kva).
pub(crate) fn buf_put(g: &mut Cache, t: BufRef) {
    let bp = t.buf();
    if bp.has_pages(g) {
        kassert!(bp.bufsize() > 0);
    }
    if bp.isset(B_DELWRI) {
        kpanic(format_args!("buf_put: releasing dirty buffer"));
    }
    if bp.b_freelist_linked() {
        kpanic(format_args!("buf_put: still on the free list"));
    }
    if bp.b_vnbufs_linked() {
        kpanic(format_args!("buf_put: still on the vnode list"));
    }
    bc_add(&BCSTATS.numbufs, -1);

    if let Some(t) = buf_dealloc_mem(g, t) {
        match t.into_counted(g) {
            // pool_put(&bufpool, bp)
            Some(r) => drop(g.bufhead.take(r)),
            None => kpanic(format_args!("buf_put: a driver's buffer")),
        }
    }
}

/// `bufinit`: initializes buffers and hash links for buffers: the cache's size from the
/// DMA-reachable memory (`physmem` pages in all, the machine's), the water marks, `bufpl`,
/// the queues and the kva arena.
pub fn bufinit(physmem: usize) {
    let mut pages = uvm_pagecount(&NO_CONSTRAINT) as u64;
    // take away a guess at how much of this the kernel will consume: the C's
    // `atop(physmem) - atop(uvmexp.free)`, kept as written although both are page counts
    // already (so the guess takes away almost nothing): OpenBSD's cache size is the spec.
    let free = UVMEXP.free.load(Mem::Relaxed).max(0) as usize;
    pages = pages.wrapping_sub((atop(physmem) as u64).wrapping_sub(atop(free) as u64));
    let pages = pages as i64;

    // If MD code doesn't say otherwise, use up to 10% of DMA'able memory for buffers.
    if BUFCACHEPERCENT.load(Mem::Relaxed) == 0 {
        BUFCACHEPERCENT.store(10, Mem::Relaxed);
    }

    // XXX these values and their same use in kern_sysctl need to move into buf.h
    let pct = i64::from(BUFCACHEPERCENT.load(Mem::Relaxed));
    kassert!(pct <= 90);
    kassert!(pct >= 5);
    if BUFPAGES.load(Mem::Relaxed) == 0 {
        BUFPAGES.store(pages * pct / 100, Mem::Relaxed);
    }
    if BUFPAGES.load(Mem::Relaxed) < BCACHE_MIN {
        BUFPAGES.store(BCACHE_MIN, Mem::Relaxed);
    }
    kassert!(BUFPAGES.load(Mem::Relaxed) < pages);

    BUFHIGHPAGES.store(BUFPAGES.load(Mem::Relaxed), Mem::Relaxed);

    // Set the base backoff level for the buffer cache. We will not allow uvm to steal back
    // more than this number of pages.
    let buflowpages = (pages * 5 / 100).max(BCACHE_MIN);
    BUFLOWPAGES.store(buflowpages, Mem::Relaxed);

    // set bufbackpages to 100 pages, or 10 percent of the low water mark if we don't have
    // that many pages.
    BUFBACKPAGES.store((buflowpages * 10 / 100).min(100), Mem::Relaxed);

    // If the MD code does not say otherwise, reserve 10% of kva space for mapping buffers.
    let mut bufkvm = BUFKVM.load(Mem::Relaxed);
    if bufkvm == 0 {
        bufkvm = VM_KERNEL_SPACE_SIZE / 10;
    }

    // Don't use more than twice the amount of bufpages for mappings. It's twice since we map
    // things sparsely.
    let bufpages_bytes = BUFPAGES.load(Mem::Relaxed) as usize * PAGE_SIZE;
    if bufkvm > bufpages_bytes {
        bufkvm = bufpages_bytes;
    }
    // Round bufkvm to MAXPHYS because we allocate chunks of va space in MAXPHYS chunks.
    bufkvm &= !(MAXPHYS - 1);
    BUFKVM.store(bufkvm, Mem::Relaxed);

    // pool_init(&bufpool, ..., "bufpl"): once (a host test process sets it up once).
    if BUFPL.item_size().is_none() {
        let _ = BUFPL.init(IPL_BIO, 0, "bufpl");
    }
    if BUFPL.item_size().is_none() {
        kpanic(format_args!("bufinit: bufpl"));
    }

    let mut g = CACHE.lock();
    bufcache_init(&mut g);

    // hmm - bufkvm is an argument because it's static, while bufpages is global because it
    // can change while running.
    buf_mem_init(&mut g, bufkvm);
    drop(g);

    // Set the dirty page high water mark to be less than the low water mark for pages in the
    // buffer cache. This ensures we can always back off by throwing away clean pages, and give
    // ourselves a chance to write out the dirty pages eventually.
    HIDIRTYPAGES.store((buflowpages / 4) * 3, Mem::Relaxed);
    LODIRTYPAGES.store(buflowpages / 2, Mem::Relaxed);

    // We are allowed to use up to the reserve.
    TARGETPAGES.store(BUFPAGES.load(Mem::Relaxed) - RESERVE_PAGES, Mem::Relaxed);
}

/// `bufadjust(newbufpages)`: changes the cache's size (`kern.bufcachepercent`, the page
/// daemon's back-off).
pub fn bufadjust(newbufpages: i64) {
    let mut g = CACHE.lock();
    bufadjust_locked(&mut g, newbufpages);
}

/// [`bufadjust`] with the cache's lock held.
fn bufadjust_locked(g: &mut Cache, newbufpages: i64) {
    let newbufpages = newbufpages.max(BUFLOWPAGES.load(Mem::Relaxed));
    BUFPAGES.store(newbufpages, Mem::Relaxed);

    // We are allowed to use up to the reserve
    let targetpages = newbufpages - RESERVE_PAGES;
    TARGETPAGES.store(targetpages, Mem::Relaxed);

    // Shrinking the cache happens here only if someone has manually adjusted bufcachepercent
    // - or the pagedaemon has told us to give back memory *now* - so we give it all back.
    let numbufpages = BCSTATS.numbufpages.load(Mem::Relaxed);
    if numbufpages > targetpages {
        let _ = bufcache_recover_pages(g, false, numbufpages - targetpages);
    }
    bufcache_adjust(g);

    // Wake up the cleaner if we have lots of dirty pages, or if we are getting low on buffer
    // cache kva.
    if unclean_pages() >= HIDIRTYPAGES.load(Mem::Relaxed)
        || BCSTATS.kvaslots_avail.load(Mem::Relaxed) <= 2 * RESERVE_SLOTS
    {
        wakeup(&raw const BD_REQ);
    }
}

/// `bufbackoff(range, size)`: backs off `size` buffer cache pages (at least `bufbackpages`);
/// the number of pages freed. Called by the page daemon to consume buffer cache pages rather
/// than scanning.
pub fn bufbackoff(size: i64) -> u64 {
    let mut g = CACHE.lock();
    bufbackoff_locked(&mut g, size)
}

/// [`bufbackoff`] with the cache's lock held (`buf_alloc_pages`').
pub(crate) fn bufbackoff_locked(g: &mut Cache, size: i64) -> u64 {
    // Back off by at least bufbackpages. If the page daemon gave us a larger size, back off by
    // that much.
    let mut pdelta = size.max(BUFBACKPAGES.load(Mem::Relaxed));

    let buflowpages = BUFLOWPAGES.load(Mem::Relaxed);
    let cur = BUFPAGES.load(Mem::Relaxed);
    if cur <= buflowpages {
        return 0;
    }
    if cur - pdelta < buflowpages {
        pdelta = cur - buflowpages;
    }
    bufadjust_locked(g, cur - pdelta);
    (cur - BUFPAGES.load(Mem::Relaxed)) as u64
}

/// What `bio_doread` leaves its caller.
pub(crate) enum Doread {
    /// The buffer holds valid data (or a delayed write): its token.
    Held(BufRef),
    /// The same for a legacy caller ([`Park::Valid`]): the token parked in getblk's section,
    /// a counted link.
    Parked(SecondaryRef<ByBufhead>),
    /// A synchronous read was started: a counted link for `biowait`.
    Started(SecondaryRef<ByBufhead>),
    /// An asynchronous read was started, or the buffer was valid and released.
    Released,
}

/// When `getblk` parks the token it gives, for `bsd`'s legacy callers, who name the buffer by
/// address with its token parked in it: inside getblk's own section of the cache's lock, so
/// the park costs no section of its own.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Park {
    /// Never: the caller takes the token.
    Never,
    /// A cached buffer always (a legacy `getblk`); a new one is parked by the caller.
    Cached,
    /// A cached buffer holding valid data or a delayed write (a legacy read: a buffer to read
    /// into goes to the strategy routine with its token).
    Valid,
}

/// What [`getblk_in`] gives.
enum Got {
    /// The token.
    Token(BufRef),
    /// The token parked in the buffer ([`Park`]), a counted link.
    Parked(SecondaryRef<ByBufhead>),
}

/// `bio_doread`: the buffer of `blkno`, with a read started when it holds no valid data
/// (asynchronous with `async_` = `B_ASYNC`).
pub(crate) fn bio_doread(vb: &'static VnodeBufs, blkno: Daddr, size: i32, async_: i64) -> Doread {
    bio_doread_in(vb, blkno, size, async_, Park::Never)
}

/// [`bio_doread`], a valid cached buffer's token parked for a legacy caller (`park`
/// [`Park::Valid`]: a synchronous read only).
fn bio_doread_in(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
    async_: i64,
    park: Park,
) -> Doread {
    let got = loop {
        if let Some(got) = getblk_in(vb, blkno, size, 0, INFSLP, park) {
            break got;
        }
    };
    let owner = vb.owner();
    let t = match got {
        Got::Token(t) => t,
        Got::Parked(r) => {
            // Valid already (getblk parked it only so): no read, the statistics only.
            owner.count_io(false, async_ != 0);
            return Doread::Parked(r);
        }
    };

    // If buffer does not have valid data, start a read. Note that if buffer is B_INVAL,
    // getblk() won't return it. Therefore, it's valid if its I/O has completed or been
    // delayed.
    let r = if !t.buf().isset(B_DONE | B_DELWRI) {
        t.buf().set(B_READ | async_);
        bc_add(&BCSTATS.pendingreads, 1);
        bc_add(&BCSTATS.numreads, 1);
        let r = if async_ == 0 { t.counted() } else { None };
        let _ = owner.strategy(t);
        // Pay for the read.
        owner.charge_io(false); // XXX
        match r {
            Some(r) => Doread::Started(r),
            None => Doread::Released,
        }
    } else if async_ != 0 {
        brelse(t);
        Doread::Released
    } else {
        Doread::Held(t)
    };

    // Collect statistics on synchronous and asynchronous reads. Reads from block devices are
    // charged to their associated filesystem (if any).
    owner.count_io(false, async_ != 0);
    r
}

/// The result of an I/O whose buffer is done (`biowait`'s end): `EINTR` when it was
/// interrupted (`B_EINTR`, cleared), the buffer's error (`EIO` without one) on `B_ERROR`.
fn io_result(bp: &Buf) -> Result<(), Errno> {
    // check for interruption of I/O (e.g. via NFS), then errors.
    if bp.isset(B_EINTR) {
        bp.clr(B_EINTR);
        return Err(Errno::EINTR);
    }
    if bp.isset(B_ERROR) {
        Err(bp.error().unwrap_or(Errno::EIO))
    } else {
        Ok(())
    }
}

/// `biowait(bp)` for a buffer whose busy token stays where it is (parked in it by the driver's
/// `biodone`, as `bsd`'s callers that name the buffer by address keep it): waits for `B_DONE`
/// and gives the I/O's error value.
pub fn biowait_buf(bp: &Buf) -> Result<(), Errno> {
    kassert!(!bp.isset(B_ASYNC));
    let mut g = CACHE.lock();
    while !bp.isset(B_DONE) {
        let _ = g.sleep_nsec(ptr::from_ref(bp), PRIBIO + 1, "biowait", INFSLP);
    }
    drop(g);
    io_result(bp)
}

/// The buffer of a synchronous `bio_doread` and its read's result (`biowait`).
fn doread_wait(d: Doread) -> (BufRef, Result<(), Errno>) {
    match d {
        Doread::Held(t) => {
            kassert!(t.buf().isset(B_DONE));
            let r = io_result(t.buf());
            (t, r)
        }
        Doread::Started(r) => biowait(r),
        Doread::Parked(r) => {
            let Some(t) = r.get().and_then(Buf::take_parked) else {
                kpanic(format_args!("bread: a parked buffer without its token"));
            };
            let rv = io_result(t.buf());
            (t, rv)
        }
        Doread::Released => kpanic(format_args!("bread: a synchronous read went away")),
    }
}

/// [`doread_wait`] for a legacy caller: the buffer with its token parked (by getblk, or by
/// the driver's `biodone` for `biowait`, where it stays), as a counted link, and the read's
/// result.
fn doread_wait_parked(d: Doread) -> (SecondaryRef<ByBufhead>, Result<(), Errno>) {
    match d {
        Doread::Parked(r) => {
            let Some(bp) = r.get() else {
                kpanic(format_args!("bread: a link to nothing"));
            };
            kassert!(bp.isset(B_DONE));
            let rv = io_result(bp);
            (r, rv)
        }
        Doread::Started(r) => {
            let Some(bp) = r.get() else {
                kpanic(format_args!("bread: a link to nothing"));
            };
            let rv = biowait_buf(bp);
            (r, rv)
        }
        Doread::Held(t) => {
            let Some(r) = t.counted() else {
                kpanic(format_args!("bread: a driver's buffer"));
            };
            Buf::park(t);
            let rv = r.get().map_or(Err(Errno::EIO), io_result);
            (r, rv)
        }
        Doread::Released => kpanic(format_args!("bread: a synchronous read went away")),
    }
}

/// Reads a disk block (Bach p.54): the buffer's token and the read's result.
pub fn bread(vb: &'static VnodeBufs, blkno: Daddr, size: i32) -> (BufRef, Result<(), Errno>) {
    // Get buffer for block.
    let d = bio_doread(vb, blkno, size, 0);

    // Wait for the read to complete, and return result.
    doread_wait(d)
}

/// [`bread`] for `bsd`'s legacy callers: the buffer with its token parked in it, as a counted
/// link (no section of the cache's lock of its own for the park).
pub fn bread_parked(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
) -> (SecondaryRef<ByBufhead>, Result<(), Errno>) {
    doread_wait_parked(bio_doread_in(vb, blkno, size, 0, Park::Valid))
}

/// Read-ahead multiple disk blocks. The first is sync, the rest async. Trivial modification
/// to the breada algorithm presented in Bach (p.55).
pub fn breadn(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
    rablks: &[Daddr],
    rasizes: &[i32],
) -> (BufRef, Result<(), Errno>) {
    let d = bio_doread(vb, blkno, size, 0);

    // For each of the read-ahead blocks, start a read, if necessary.
    for (&rablk, &rasize) in rablks.iter().zip(rasizes) {
        // If it's in the cache, just go on to next one.
        if incore(vb, rablk) {
            continue;
        }

        // Get a buffer for the read-ahead block
        let _ = bio_doread(vb, rablk, rasize, B_ASYNC);
    }

    // Otherwise, we had to start a read for it; wait until it's valid.
    doread_wait(d)
}

/// The most buffers a cluster read gives (`MAXPHYS` of one-page blocks).
const CLUSTER_MAX: usize = MAXPHYS / PAGE_SIZE;

/// Called from interrupt context: the end of a cluster read (`bread_cluster`'s `B_CALL`). The
/// read-ahead buffers are the busy buffers of the blocks after the master's whose pages are
/// its (`Pages::Master`): found in one walk of the vnode's tree under the cache's lock, their
/// parked tokens taken in the same section (the C's `xbpp` array, which `b_saveaddr` carries),
/// in which their pages move to their own objects too (the cache's lock comes before the
/// pages' locks, as `buf_free_pages` takes them).
pub fn bread_cluster_callback(mut t: BufRef) {
    let vb = b_vp(t.buf());
    let mut kids: [Option<BufRef>; CLUSTER_MAX] = [const { None }; CLUSTER_MAX];
    let mut n = 1;
    let mut g = CACHE.lock();
    {
        let mut next = {
            let st = vb.st.get(&g);
            st.tree.next(t.buf()).map(counted)
        };
        while let Some(r) = next {
            let Some(x) = r.get() else { break };
            if !x.is_cluster_of(&g, t.buf()) || n >= CLUSTER_MAX {
                break;
            }
            let Some(xt) = x.take_parked_locked(&mut g) else {
                kpanic(format_args!(
                    "bread_cluster_callback: buffer {n} has no token"
                ));
            };
            next = {
                let st = vb.st.get(&g);
                st.tree.next(xt.buf()).map(counted)
            };
            drop(r);
            kids[n] = Some(xt);
            n += 1;
        }
    }

    if n > 1 {
        let newsize = kids[1].as_ref().map_or(0, |x| x.buf().bufsize());

        // Shrink this buffer's mapping to only cover its part of the total I/O.
        buf_fix_mapping(&mut t, newsize as usize);
        t.buf().set_bcount(newsize);
    }

    // Invalidate read-ahead buffers if read short
    let bp = t.buf();
    if bp.resid() > 0 {
        let mut i = n;
        while i > 1 {
            i -= 1;
            let Some(x) = kids[i].as_ref() else { break };
            let xsize = x.buf().bufsize() as usize;
            if xsize <= bp.resid() {
                bp.set_resid(bp.resid() - xsize);
                x.buf().set(B_INVAL);
            } else if bp.resid() > 0 {
                bp.set_resid(0);
                x.buf().set(B_INVAL);
            } else {
                break;
            }
        }
    }

    // Move the pages from the master buffer's uvm object into the individual buffer's uvm
    // objects.
    for x in kids[1..n].iter().flatten() {
        let x = x.buf();
        if t.buf().isset(B_ERROR) {
            x.set(B_INVAL | B_ERROR);
        }
        let newobj = x.uobj();
        let oldobj = t.buf().uobj();
        newobj.reinit_dummy(1); // uvm_obj_init(newobj, &bufcache_pager, 1)
        for page in 0..atop(x.bufsize() as usize) {
            let off = x.poffs() + (page * PAGE_SIZE) as Voff;
            let Some(pg) = uvm_pagelookup(oldobj, off) else {
                kpanic(format_args!(
                    "bread_cluster_callback: page {off:#x} is gone"
                ));
            };
            kassert!(pg.wire_count() == 1);
            uvm_pagerealloc(pg, Owner::Object(oldobj), Owner::Object(newobj), off);
        }
        // Its pages are its own now: the master's link given back.
        x.set_pages(&mut g, Pages::Own);
    }
    drop(g);
    for xt in kids[1..n].iter_mut().filter_map(Option::take) {
        biodone(xt);
    }

    if t.buf().isset(B_ASYNC) {
        brelse(t);
    } else {
        let chan = ptr::from_ref(t.buf());
        let mut g = CACHE.lock();
        t.buf().clr(B_WANTED);
        Buf::park_locked(&mut g, t);
        drop(g);
        wakeup(chan);
    }
}

/// Read-ahead multiple disk blocks, but make sure only one (big) I/O request is sent to the
/// disk.
///
/// XXX This should probably be dropped and breadn should instead be optimized XXX to do fewer
/// I/O requests.
pub fn bread_cluster(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
) -> (BufRef, Result<(), Errno>) {
    let rbp = bio_doread(vb, blkno, size, 0);
    cluster_readahead(vb, blkno, size, &rbp);
    doread_wait(rbp)
}

/// [`bread_cluster`] for `bsd`'s legacy callers: the first buffer with its token parked in
/// it, as a counted link.
pub fn bread_cluster_parked(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
) -> (SecondaryRef<ByBufhead>, Result<(), Errno>) {
    let rbp = bio_doread_in(vb, blkno, size, 0, Park::Valid);
    cluster_readahead(vb, blkno, size, &rbp);
    doread_wait_parked(rbp)
}

/// `bread_cluster`'s read-ahead of the blocks after `blkno`, when the first one was not in
/// the cache: one buffer with the pages of all of them, read in one request, whose callback
/// hands each block's pages to its own buffer. The read-ahead buffers' tokens are kept in a
/// stack array while they are set up (the C's `xbpp`), then parked in their buffers for the
/// I/O.
fn cluster_readahead(vb: &'static VnodeBufs, blkno: Daddr, size: i32, rbp: &Doread) {
    // If the buffer is in the cache skip any I/O operation (the first buffer's B_CACHE,
    // whether its read was started or not).
    let cached = match rbp {
        Doread::Held(t) => t.buf().isset(B_CACHE),
        Doread::Parked(r) | Doread::Started(r) => r.get().is_some_and(|b| b.isset(B_CACHE)),
        Doread::Released => false,
    };
    if cached {
        return;
    }

    if size as usize != round_page(size as usize) {
        return;
    }

    let owner = vb.owner();
    let Ok((sblkno, maxra)) = owner.bmap(blkno + 1) else {
        return;
    };

    let maxra = maxra + 1;
    if sblkno == -1 || maxra < 2 {
        return;
    }

    let howmany = ((MAXPHYS / size as usize) as i32).min(maxra) as usize;

    // From the last to the first, which allocates big enough size to cover what all the other
    // buffers need.
    let mut xbpp: [Option<BufRef>; CLUSTER_MAX] = [const { None }; CLUSTER_MAX];
    let mut i = howmany;
    while i > 0 {
        i -= 1;
        let sz = if i == 0 { howmany * size as usize } else { 0 };
        match buf_get(Some(vb), blkno + i as Daddr + 1, sz) {
            Some(t) => xbpp[i] = Some(t),
            None => {
                // Undo: the buffers already made are invalidated and released.
                for xt in xbpp[i + 1..howmany].iter_mut().filter_map(Option::take) {
                    xt.buf().set(B_INVAL);
                    brelse(xt);
                }
                return;
            }
        }
    }

    let Some(t) = xbpp[0].take() else {
        return;
    };

    let inc = (size as i64) >> DEV_BSHIFT;
    let bp = t.buf();
    {
        let mut g = CACHE.lock();
        for (i, slot) in xbpp.iter_mut().enumerate().take(howmany).skip(1) {
            let Some(xt) = slot.take() else {
                kpanic(format_args!("bread_cluster: read-ahead buffer {i} is gone"));
            };
            let x = xt.buf();
            bc_add(&BCSTATS.pendingreads, 1);
            bc_add(&BCSTATS.numreads, 1);
            x.set(B_READ | B_ASYNC);
            x.set_blkno(sblkno + i as Daddr * inc);
            x.set_bufsize(i64::from(size));
            x.set_bcount(i64::from(size));
            x.set_data_addr(0);
            x.set_pages(&mut g, Pages::Master(counted(bp)));
            x.set_poffs(bp.poffs() + i as i64 * i64::from(size));
            // Parked in its buffer for the I/O: the callback takes it back.
            Buf::park_locked(&mut g, xt);
        }
        bp.set_iodone_locked(&mut g, Some(Iodone::Kernel(bread_cluster_callback)));
    }

    kassert!(bp.lblkno() == blkno + 1);
    kassert!(bp.vnode_bufs().is_some_and(|v| ptr::eq(v, vb)));

    bp.set_blkno(sblkno);
    bp.set(B_READ | B_ASYNC | B_CALL);

    bc_add(&BCSTATS.pendingreads, 1);
    bc_add(&BCSTATS.numreads, 1);
    let _ = owner.strategy(t);
    owner.charge_io(false);
}

/// Block write (Bach p.56): the buffer is written and released, synchronously unless it is
/// `B_ASYNC`; the write's result.
pub fn bwrite(t: BufRef) -> Result<(), Errno> {
    let bp = t.buf();
    let vb = b_vp(bp);
    let owner = vb.owner();

    // Remember buffer type, to switch on it later. If the write was synchronous, but the file
    // system was mounted with MNT_ASYNC, convert it to a delayed write.
    // XXX note that this relies on delayed tape writes being converted to async, not sync
    // writes (which is safe, but ugly).
    let async_ = bp.isset(B_ASYNC);
    if !async_ && t.foreign_buf().is_some() {
        // A synchronous write waits for the buffer (`biowait`) and then releases it to the
        // cache: neither applies to a buffer a driver embeds, and no caller writes one so.
        kpanic(format_args!(
            "bwrite: a driver's own buffer {:p} written synchronously",
            bp
        ));
    }
    if !async_ && owner.mnt_async() {
        // Don't convert writes from VND on async filesystems that already have delayed writes
        // in the upper layer.
        if !bp.isset(B_NOCACHE) {
            bdwrite(t);
            return Ok(());
        }
    }

    // Collect statistics on synchronous and asynchronous writes. Writes to block devices are
    // charged to their associated filesystem (if any).
    owner.count_io(true, async_);
    bc_add(&BCSTATS.pendingwrites, 1);
    bc_add(&BCSTATS.numwrites, 1);

    let wasdelayed = bp.isset(B_DELWRI);
    bp.clr(B_READ | B_DONE | B_ERROR | B_DELWRI);

    let mut g = CACHE.lock();

    // If not synchronous, pay for the I/O operation and make sure the buf is on the correct
    // vnode queue. We have to do this now, because if we don't, the vnode may not be properly
    // notified that its I/O has completed.
    if wasdelayed {
        reassignbuf(&mut g, bp);
    } else {
        owner.charge_io(true);
    }

    // Initiate disk write. Make sure the appropriate party is charged.
    vb.numoutput.fetch_add(1, Mem::Relaxed);
    bp.set(B_WRITEINPROG);
    drop(g);
    let r = if async_ { None } else { t.counted() };
    let note = note_claim(bp);
    let _ = owner.strategy(t);

    // If the queue is above the high water mark, wait till the number of outstanding write
    // bufs drops below the low water mark.
    if let Some(bq) = note_take(note) {
        super::bufq::bufq_wait(bq);
    }

    let Some(r) = r else {
        return Ok(());
    };

    // If I/O was synchronous, wait for it to complete.
    let (t, rv) = biowait(r);

    // Release the buffer.
    brelse(t);

    rv
}

/// The address a note keys a buffer by.
fn note_key(bp: &Buf) -> usize {
    ptr::from_ref(bp).addr()
}

/// A free note claimed for `bp`, its index; `None` when all are taken.
fn note_try_claim(bp: &Buf) -> Option<usize> {
    let key = note_key(bp);
    for (i, n) in QUEUE_NOTE.iter().enumerate() {
        if n.word
            .compare_exchange(0, key, Mem::SeqCst, Mem::Relaxed)
            .is_ok()
        {
            n.bq.set(None);
            NOTES_CLAIMED.fetch_add(1, Mem::SeqCst);
            return Some(i);
        }
    }
    None
}

/// `bwrite`'s note for `bp`, claimed before its strategy call; sleeps while all are taken
/// (the check repeated after `sleep_setup`, so a note given back meanwhile is not missed).
fn note_claim(bp: &Buf) -> usize {
    loop {
        if let Some(i) = note_try_claim(bp) {
            return i;
        }
        NOTES_WANTED.fetch_add(1, Mem::SeqCst);
        hal::sleep::sleep_setup(&raw const NOTES_WANTED, PRIBIO, "bqnote");
        let got = note_try_claim(bp);
        let _ = hal::sleep::sleep_finish(INFSLP, got.is_none());
        NOTES_WANTED.fetch_sub(1, Mem::SeqCst);
        if let Some(i) = got {
            return i;
        }
    }
}

/// Gives the note `i` back: the queue its buffer went on, `None` when it went on none or its
/// I/O is done already (the C's `bp->b_bq` after the strategy call).
fn note_take(i: usize) -> Option<&'static super::bufq::Bufq> {
    let n = &QUEUE_NOTE[i];
    let bq = n.bq.get();
    n.bq.set(None);
    let word = n.word.swap(0, Mem::SeqCst);
    NOTES_CLAIMED.fetch_sub(1, Mem::SeqCst);
    if NOTES_WANTED.load(Mem::SeqCst) != 0 {
        wakeup(&raw const NOTES_WANTED);
    }
    if word & NOTE_DONE != 0 { None } else { bq }
}

/// `bufq_queue` put `bp` on `bq`: written into the note a `bwrite` claimed for `bp`, if any.
pub(crate) fn note_queued(bp: &Buf, bq: &'static super::bufq::Bufq) {
    if NOTES_CLAIMED.load(Mem::SeqCst) == 0 {
        return;
    }
    let key = note_key(bp);
    if let Some(n) = QUEUE_NOTE.iter().find(|n| n.word.load(Mem::SeqCst) == key) {
        n.bq.set(Some(bq));
    }
}

/// `bufq_done` for `bp`: its note, if a `bwrite` holds one, marked done (the C's `b_bq =
/// NULL`). The mark is a compare-and-swap on the claimed word, so a note given back and
/// claimed again for another buffer meanwhile is left alone.
pub(crate) fn note_done(bp: &Buf) {
    if NOTES_CLAIMED.load(Mem::SeqCst) == 0 {
        return;
    }
    let key = note_key(bp);
    for n in &QUEUE_NOTE {
        if n.word
            .compare_exchange(key, key | NOTE_DONE, Mem::SeqCst, Mem::Relaxed)
            .is_ok()
        {
            return;
        }
    }
}

/// Delayed write.
///
/// The buffer is marked dirty, but is not queued for I/O. This routine should be used when
/// the buffer is expected to be modified again soon, typically a small write that partially
/// fills a buffer.
///
/// NB: magnetic tapes cannot be delayed; they must be written in the order that the writes are
/// requested.
///
/// Described in Leffler, et al. (pp. 208-213).
pub fn bdwrite(t: BufRef) {
    let bp = t.buf();
    // If the block hasn't been seen before:
    //	(1) Mark it as having been seen,
    //	(2) Charge for the write.
    //	(3) Make sure it's on its vnode's correct block list,
    //	(4) If a buffer is rewritten, move it to end of dirty list
    if !bp.isset(B_DELWRI) {
        bp.set(B_DELWRI);
        let mut g = CACHE.lock();
        reassignbuf(&mut g, bp);
        drop(g);
        b_vp(bp).owner().charge_io(true); // XXX
    }

    // The "write" is done, so mark and release the buffer.
    bp.clr(B_NEEDCOMMIT);
    bp.clr(B_NOCACHE); // Must cache delayed writes
    bp.set(B_DONE);
    brelse(t);
}

/// Asynchronous block write; just an asynchronous bwrite().
pub fn bawrite(t: BufRef) {
    t.buf().set(B_ASYNC);
    let owner = b_vp(t.buf()).owner();
    let _ = owner.bwrite(t);
}

/// `buf_dirty(bp)`: marks a busy buffer dirty, on its vnode's dirty list.
pub fn buf_dirty(t: &BufRef) {
    let bp = t.buf();
    let mut g = CACHE.lock();
    if !bp.isset(B_DELWRI) {
        bp.set(B_DELWRI);
        reassignbuf(&mut g, bp);
    }
}

/// `buf_undirty(bp)`: marks a busy buffer clean, on its vnode's clean list.
pub fn buf_undirty(t: &BufRef) {
    let bp = t.buf();
    let mut g = CACHE.lock();
    if bp.isset(B_DELWRI) {
        bp.clr(B_DELWRI);
        reassignbuf(&mut g, bp);
    }
}

/// Releases a buffer on to the free lists (Bach p. 46): its token is consumed.
pub fn brelse(t: BufRef) {
    let mut g = CACHE.lock();
    brelse_locked(&mut g, t);
}

/// [`brelse`] of a buffer whose token is parked in it (`bsd`'s legacy callers, which hold the
/// buffer by address): the token taken back and the buffer released in one section of the
/// cache's lock. `false`, with nothing done, when no token is parked (a driver's own idle
/// buffer, whose token the caller makes).
pub fn brelse_parked(bp: &Buf) -> bool {
    let mut g = CACHE.lock();
    match bp.take_parked_locked(&mut g) {
        Some(t) => {
            brelse_locked(&mut g, t);
            true
        }
        None => false,
    }
}

/// [`brelse`] with the cache's lock held.
fn brelse_locked(g: &mut Cache, t: BufRef) {
    let bp = t.buf();
    if t.is_mapped() {
        kassert!(bp.bufsize() > 0);
    }

    // Determine which queue the buffer should be on, then put it there.

    // If it's not cacheable, or an error, mark it invalid.
    if bp.isset(B_NOCACHE | B_ERROR) {
        bp.set(B_INVAL);
    }
    // If it's a write error, also mark the vnode as damaged.
    if bp.isset(B_ERROR)
        && !bp.isset(B_READ)
        && let Some(vb) = bp.vnode_bufs()
        && vb.owner().vtype() == VType::VREG
    {
        vb.set_bioflag(VBIOERROR);
    }

    let chan = ptr::from_ref(bp);
    if bp.isset(B_INVAL) {
        // If the buffer is invalid, free it now rather than leaving it in a queue and wasting
        // memory.
        if bp.isset(B_DELWRI) {
            bp.clr(B_DELWRI);
        }

        if let Some(vb) = bp.vnode_bufs() {
            vb.st.get_mut(g).tree.remove(bp);
            brelvp(g, bp);
        }
        bp.set_vnode_bufs(None);

        // Wake up any processes waiting for _this_ buffer to become free. They are not allowed
        // to grab it since it will be freed. But the only sleeper is getblk and it will
        // restart the operation after sleep.
        if bp.isset(B_WANTED) {
            bp.clr(B_WANTED);
            wakeup(chan);
        }
        buf_put(g, t);
    } else {
        // It has valid data. Put it on the end of the appropriate queue, so that it'll stick
        // around for as long as possible.
        bufcache_release(g, bp);

        // Unlock the buffer.
        bp.clr(B_AGE | B_ASYNC | B_NOCACHE | B_DEFERRED);
        let wanted = bp.isset(B_WANTED);
        if wanted {
            bp.clr(B_WANTED);
        }
        buf_release(g, t);

        // Wake up any processes waiting for _this_ buffer to become free.
        if wanted {
            wakeup(chan);
        }

        let numbufpages = BCSTATS.numbufpages.load(Mem::Relaxed);
        let targetpages = TARGETPAGES.load(Mem::Relaxed);
        if numbufpages > targetpages {
            let _ = bufcache_recover_pages(g, false, numbufpages - targetpages);
        }
        bufcache_adjust(g);
    }

    // Wake up syncer and cleaner processes waiting for buffers.
    if NOBUFFERS.load(Mem::Relaxed) != 0 {
        NOBUFFERS.store(0, Mem::Relaxed);
        wakeup(&raw const NOBUFFERS);
    }

    // Wake up any processes waiting for any buffer to become free.
    if NEEDBUFFER.load(Mem::Relaxed) != 0
        && BCSTATS.numbufpages.load(Mem::Relaxed) < TARGETPAGES.load(Mem::Relaxed)
        && BCSTATS.kvaslots_avail.load(Mem::Relaxed) > RESERVE_SLOTS
    {
        NEEDBUFFER.store(0, Mem::Relaxed);
        wakeup(&raw const NEEDBUFFER);
    }
}

/// The buffer of `blkno` in `vb`'s tree, valid or not, as a counted link (the C's
/// `RBT_FIND` with a key `struct buf` on the stack).
fn rbt_find(g: &Cache, vb: &'static VnodeBufs, blkno: Daddr) -> Option<SecondaryRef<ByBufhead>> {
    vb.st
        .get(g)
        .tree
        .find_by(|b| blkno.cmp(&b.lblkno()))
        .map(counted)
}

/// Whether the block is in the cache: on its vnode's tree and not `B_INVAL`
/// (`incore_locked`).
fn incore_locked(g: &Cache, vb: &'static VnodeBufs, blkno: Daddr) -> bool {
    vb.st
        .get(g)
        .tree
        .find_by(|b| blkno.cmp(&b.lblkno()))
        .is_some_and(|b| !b.isset(B_INVAL))
}

/// `incore(vp, blkno)`: whether the cache holds a valid buffer of the block.
pub fn incore(vb: &'static VnodeBufs, blkno: Daddr) -> bool {
    let g = CACHE.lock();
    incore_locked(&g, vb, blkno)
}

/// Get a block of requested size that is associated with a given vnode and block offset. If it
/// is found in the block cache, mark it as having been found, make it busy and return it.
/// Otherwise, return an empty block of the correct size. It is up to the caller to ensure that
/// the cached blocks be of the correct size. `None` when the sleep for a busy buffer was cut
/// short (`slpflag`'s `PCATCH`, `slptimeo`).
pub fn getblk(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
    slpflag: i32,
    slptimeo: u64,
) -> Option<BufRef> {
    match getblk_in(vb, blkno, size, slpflag, slptimeo, Park::Never)? {
        Got::Token(t) => Some(t),
        Got::Parked(_) => kpanic(format_args!("getblk: a token parked unasked")),
    }
}

/// [`getblk`] for `bsd`'s legacy callers: the buffer with its token parked in it, as a
/// counted link (a cached one's parked in getblk's own section of the cache's lock).
pub fn getblk_parked(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
    slpflag: i32,
    slptimeo: u64,
) -> Option<SecondaryRef<ByBufhead>> {
    match getblk_in(vb, blkno, size, slpflag, slptimeo, Park::Cached)? {
        Got::Parked(r) => Some(r),
        Got::Token(t) => {
            let Some(r) = t.counted() else {
                kpanic(format_args!("getblk: a driver's buffer"));
            };
            Buf::park(t);
            Some(r)
        }
    }
}

/// [`getblk`]'s body, the token parked as `park` says.
fn getblk_in(
    vb: &'static VnodeBufs,
    blkno: Daddr,
    size: i32,
    slpflag: i32,
    slptimeo: u64,
    park: Park,
) -> Option<Got> {
    // XXX
    // The following is an inlined version of 'incore()', but with the 'invalid' test moved to
    // after the 'busy' test. It's necessary because there are some cases in which the NFS code
    // sets B_INVAL prior to writing data to the server, but in which the buffers actually
    // contain valid data. In this case, we can't allow the system to allocate a new buffer for
    // the block until the write is finished.
    loop {
        // start:
        let mut g = CACHE.lock();
        if let Some(r) = rbt_find(&g, vb, blkno) {
            let Some(bp) = r.get() else {
                kpanic(format_args!("getblk: a link to nothing"));
            };
            if bp.isset(B_BUSY) {
                bp.set(B_WANTED);
                let chan = ptr::from_ref(bp);
                drop(r);
                let error = g.sleep_nsec(chan, slpflag | (PRIBIO + 1), "getblk", slptimeo);
                drop(g);
                if error.is_err() {
                    return None;
                }
                continue;
            }

            if !bp.isset(B_INVAL) {
                bc_add(&BCSTATS.cachehits, 1);
                bp.set(B_CACHE);
                bufcache_take(&mut g, bp);
                let t = buf_acquire(&mut g, r);
                let parks = match park {
                    Park::Never => false,
                    Park::Cached => true,
                    Park::Valid => t.buf().isset(B_DONE | B_DELWRI),
                };
                if parks {
                    let r = counted(t.buf());
                    Buf::park_locked(&mut g, t);
                    return Some(Got::Parked(r));
                }
                return Some(Got::Token(t));
            }
        }
        drop(g);

        if let Some(t) = buf_get(Some(vb), blkno, size as usize) {
            return Some(Got::Token(t));
        }
    }
}

/// Get an empty, disassociated buffer of given size.
pub fn geteblk(size: usize) -> BufRef {
    loop {
        if let Some(t) = buf_get(None, 0, size) {
            return t;
        }
    }
}

/// Allocates a buffer. If `vb` is given, puts it into the buffer cache for that vnode. If size
/// != 0, allocates memory and maps it. If there is already a buffer for the given vnode/blkno,
/// or the cache must shrink first (the caller slept on it), returns `None`.
pub fn buf_get(vb: Option<&'static VnodeBufs>, blkno: Daddr, size: usize) -> Option<BufRef> {
    let poolwait = if size == 0 { PR_NOWAIT } else { PR_WAITOK };

    let mut g = CACHE.lock();
    if size != 0 {
        // Wake up the cleaner if we have lots of dirty pages, or if we are getting low on
        // buffer cache kva.
        if unclean_pages() >= HIDIRTYPAGES.load(Mem::Relaxed)
            || BCSTATS.kvaslots_avail.load(Mem::Relaxed) <= 2 * RESERVE_SLOTS
        {
            wakeup(&raw const BD_REQ);
        }

        let npages = atop(round_page(size)) as i64;

        // if our cache has been previously shrunk, allow it to grow again with use up to
        // bufhighpages (cachepercent)
        if BUFPAGES.load(Mem::Relaxed) < BUFHIGHPAGES.load(Mem::Relaxed) {
            bufadjust_locked(&mut g, BUFHIGHPAGES.load(Mem::Relaxed));
        }

        // If we would go over the page target with our new allocation, free enough buffers
        // first to stay at the target with our new allocation.
        if BCSTATS.numbufpages.load(Mem::Relaxed) + npages > TARGETPAGES.load(Mem::Relaxed) {
            let _ = bufcache_recover_pages(&mut g, false, npages);
            bufcache_adjust(&mut g);
        }

        // If we get here, we tried to free the world down above, and couldn't get down - Wake
        // the cleaner and wait for it to push some buffers out.
        let cur = curproc_word();
        if (BCSTATS.numbufpages.load(Mem::Relaxed) + npages > TARGETPAGES.load(Mem::Relaxed)
            || BCSTATS.kvaslots_avail.load(Mem::Relaxed) <= RESERVE_SLOTS)
            && cur != syncer_word()
            && cur != cleaner_word()
        {
            wakeup(&raw const BD_REQ);
            NEEDBUFFER.fetch_add(1, Mem::Relaxed);
            let _ = g.sleep_nsec(&raw const NEEDBUFFER, PRIBIO, "needbuffer", INFSLP);
            return None;
        }
        if BCSTATS.numbufpages.load(Mem::Relaxed) + npages > BUFPAGES.load(Mem::Relaxed) {
            // cleaner or syncer
            NOBUFFERS.store(1, Mem::Relaxed);
            let _ = g.sleep_nsec(&raw const NOBUFFERS, PRIBIO, "nobuffers", INFSLP);
            return None;
        }
    }
    // pool_get(&bufpool, poolwait) at splbio, as the C calls it: first without sleeping, under
    // the cache's lock; only when the pool must wait is the lock left for the sleep (the C's
    // splbio is, by a sleep) and taken again.
    let b = match BUFPL.get(PR_NOWAIT, Buf::new) {
        Some(b) => b,
        None if poolwait == PR_NOWAIT => return None,
        None => {
            drop(g);
            let b = BUFPL.get(poolwait, Buf::new)?;
            g = CACHE.lock();
            b
        }
    };

    // bp->b_freelist.tqe_next = NOLIST: on no queue.
    b.set_dev(NODEV);
    b.set_bcount(size as i64);

    if let Some(vb) = vb
        && incore_locked(&g, vb, blkno)
    {
        // We insert the buffer into the hash with B_BUSY set while we allocate pages for it.
        // This way any getblk that happens while we allocate pages will wait for this buffer
        // instead of starting its own buf_get. But first, we check if someone beat us to it.
        drop(g);
        drop(b); // pool_put
        return None;
    }

    g.bufhead.push(b);
    let r = match g.bufhead.last() {
        Some(bp) => counted(bp),
        None => kpanic(format_args!("buf_get: bufhead is empty")),
    };
    let mut t = buf_acquire_nomap(&mut g, r);

    if let Some(vb) = vb {
        let bp = t.buf();
        bp.set_blkno(blkno);
        bp.set_lblkno(blkno);
        bgetvp(&mut g, vb, bp);
        if vb.st.get_mut(&mut g).tree.insert(bp).is_some() {
            kpanic(format_args!("buf_get: dup lblk vp {:p} bp {:p}", vb, bp));
        }
    } else {
        // bp->b_vnbufs.le_next = NOLIST: on no vnode list.
        t.buf().set(B_INVAL);
        t.buf().set_vnode_bufs(None);
    }

    bc_add(&BCSTATS.numbufs, 1);

    if size != 0 {
        buf_alloc_pages(&mut g, &t, round_page(size));
        buf_map(&mut g, &mut t);
    }

    t.buf().set(B_BC);
    drop(g);

    Some(t)
}

/// Buffer cleaning daemon (`cleanerproc`'s loop): writes the delayed-write buffers out while
/// the cache holds too many dirty pages or too little kva, sleeping on `bd_req` otherwise.
pub fn buf_daemon() -> ! {
    CLEANER.store(curproc_word(), Mem::Relaxed);
    let mut pushed = 0;
    let mut slept_on_none = true;

    loop {
        {
            let mut g = CACHE.lock();
            if slept_on_none
                || (pushed >= 16
                    && unclean_pages() < HIDIRTYPAGES.load(Mem::Relaxed)
                    && BCSTATS.kvaslots_avail.load(Mem::Relaxed) > 2 * RESERVE_SLOTS)
            {
                pushed = 0;
                // Wake up anyone who was waiting for buffers to be released.
                if NEEDBUFFER.load(Mem::Relaxed) != 0 {
                    NEEDBUFFER.store(0, Mem::Relaxed);
                    wakeup(&raw const NEEDBUFFER);
                }
                let _ = g.sleep_nsec(&raw const BD_REQ, PRIBIO - 7, "cleaner", INFSLP);
            }
        }
        slept_on_none = true;

        loop {
            let mut g = CACHE.lock();
            let Some(r) = g.dirty.first().map(counted) else {
                break;
            };
            // TRACEPOINT(vfs, cleaner, ...): dt(4), not configured.

            if unclean_pages() < LODIRTYPAGES.load(Mem::Relaxed)
                && BCSTATS.kvaslots_avail.load(Mem::Relaxed) > 2 * RESERVE_SLOTS
                && pushed >= 16
            {
                slept_on_none = false;
                break;
            }

            if let Some(bp) = r.get() {
                bufcache_take(&mut g, bp);
            }
            let t = buf_acquire(&mut g, r);
            drop(g);

            if t.buf().isset(B_INVAL) {
                brelse(t);
                continue;
            }
            if !t.buf().isset(B_DELWRI) {
                kpanic(format_args!("Clean buffer on dirty queue"));
            }
            bawrite(t);
            pushed += 1;

            sched_pause();
        }
    }
}

/// Waits for the synchronous I/O of the buffer `r` names (whose token its driver holds) to
/// complete, takes the token back (parked by `biodone`) and gives it with the I/O's error
/// value (`biowait`).
pub fn biowait(r: SecondaryRef<ByBufhead>) -> (BufRef, Result<(), Errno>) {
    let Some(bp) = r.get() else {
        kpanic(format_args!("biowait: no buffer"));
    };
    kassert!(!bp.isset(B_ASYNC));

    let mut g = CACHE.lock();
    while !bp.isset(B_DONE) {
        let _ = g.sleep_nsec(ptr::from_ref(bp), PRIBIO + 1, "biowait", INFSLP);
    }
    let t = bp.take_parked_locked(&mut g);
    drop(g);
    let Some(t) = t else {
        kpanic(format_args!(
            "biowait: buffer {:p} done without its token",
            bp
        ));
    };
    drop(r);
    let rv = io_result(t.buf());
    (t, rv)
}

/// Marks I/O complete on a buffer, consuming its token: a callback requested (`B_CALL`) is
/// called with it; an asynchronous I/O's buffer is released; otherwise the token is parked in
/// the buffer for `biowait` and its waiters woken.
///
/// \[ Leffler, et al., says on p.247: "This routine wakes up the blocked process, frees the
/// buffer for an asynchronous write, or, for a request by the pagedaemon process, invokes a
/// procedure specified in the buffer structure" \]
///
/// In real life, the pagedaemon (or other system processes) wants to do async stuff to, and
/// doesn't want the buffer brelse()'d. (for swap pager, that puts swap buffers on the free
/// lists (!!!), for the vn device, that puts malloc'd buffers on the free lists!)
///
/// Called at `splbio()` (from interrupt context, or by a strategy routine): it takes the cache's
/// lock, which masks the interrupt.
pub fn biodone(t: BufRef) {
    let g = CACHE.lock();
    biodone_locked(t, g);
}

/// [`biodone`] of a buffer whose token is parked in it (a legacy driver's, which holds the
/// buffer by address): the token taken back in biodone's own section of the cache's lock.
/// `false`, with nothing done, when no token is parked.
pub fn biodone_parked(bp: &Buf) -> bool {
    let mut g = CACHE.lock();
    match bp.take_parked_locked(&mut g) {
        Some(t) => {
            biodone_locked(t, g);
            true
        }
        None => false,
    }
}

/// [`biodone`] from its section of the cache's lock, `g`, which it gives up before the
/// callback or the release. (`g` comes after `t`: a panic that unwinds, the host's, drops the
/// parameters in reverse order, so the lock goes before the token's drop takes it again.)
fn biodone_locked(t: BufRef, g: Cache) {
    let chan = ptr::from_ref(t.buf());
    let flags = t.buf().flags();
    if flags & B_DONE != 0 {
        drop(g);
        kpanic(format_args!("biodone already"));
    }
    let wait = flags & (B_CALL | B_ASYNC) == 0;

    {
        let bp = t.buf();
        bp.set(B_DONE); // note that it's done

        if let Some(bq) = bp.bq() {
            super::bufq::bufq_done(bq, bp);
        }

        if !bp.isset(B_READ) {
            bp.clr(B_WRITEINPROG);
            vwakeup_locked(bp.vnode_bufs());
        }
        if BCSTATS.numbufs.load(Mem::Relaxed) != 0 && !bp.isset(B_RAW | B_PHYS) {
            if !bp.isset(B_READ) {
                bc_add(&BCSTATS.pendingwrites, -1);
            } else {
                bc_add(&BCSTATS.pendingreads, -1);
            }
        }
    }
    if wait {
        // or just wakeup the buffer
        t.buf().clr(B_WANTED);
        let mut g = g;
        Buf::park_locked(&mut g, t);
        drop(g);
        wakeup(chan);
        return;
    }
    let iodone = t.buf().iodone(&g);
    drop(g);

    if flags & B_CALL != 0 {
        // if necessary, call out
        t.buf().clr(B_CALL); // but note callout done
        match iodone {
            Some(Iodone::Kernel(f)) => f(t),
            Some(Iodone::Legacy(f)) => {
                let Some(bp) = t.foreign_buf() else {
                    kpanic(format_args!(
                        "biodone: a driver's callback on a cache buffer"
                    ));
                };
                t.release_unlocked();
                f(bp);
            }
            None => kpanic(format_args!("biodone: B_CALL without b_iodone")),
        }
    } else {
        // if async, release it
        brelse(t);
    }
}

// DDB: bcstats_print waits for the ddb command loop (db_command.c).

/// `buf_adjcnt(bp, ncount)`: shortens the valid part of a buffer.
pub fn buf_adjcnt(t: &BufRef, ncount: i64) {
    kassert!(ncount <= t.buf().bufsize());
    t.buf().set_bcount(ncount);
}

// bufcache freelist code below
//
// The code below implements a variant of the 2Q buffer cache algorithm by Johnson and Shasha.
//
// General Outline
// We divide the buffer cache into three working sets: current, previous, and long term. Each
// list is itself LRU and buffers get promoted and moved around between them. A buffer starts
// its life in the current working set. As time passes and newer buffers push it out, it will
// turn into the previous working set and is subject to recycling. But if it's accessed again
// from the previous working set, that's an indication that it's actually in the long term
// working set, so we promote it there. The separation of current and previous working sets
// prevents us from promoting a buffer that's only temporarily hot to the long term cache.
//
// The objective is to provide scan resistance by making the long term working set ineligible
// for immediate recycling, even as the current working set is rapidly turned over.
//
// Implementation
// The code below identifies the current, previous, and long term sets as hotqueue, coldqueue,
// and warmqueue. The hot and warm queues are capped at 1/3 of the total clean pages, after
// which point they start pushing their oldest buffers into coldqueue.
// A buf always starts out with neither WARM or COLD flags set (implying HOT). When released,
// it will be returned to the tail of the hotqueue list. When the hotqueue gets too large, the
// oldest hot buf will be moved to the coldqueue, with the B_COLD flag set. When a cold buf is
// released, we set the B_WARM flag and put it onto the warmqueue. Warm bufs are also directly
// returned to the end of the warmqueue. As with the hotqueue, when the warmqueue grows too
// large, B_WARM bufs are moved onto the coldqueue.
//
// Note that this design does still support large working sets, greater than the cap of
// hotqueue or warmqueue would imply. The coldqueue is still cached and has no maximum length.
// The hot and warm queues form a Y feeding into the coldqueue. Moving bufs between queues is
// constant time, so this design decays to one long warm->cold queue.
//
// In the 2Q paper, hotqueue and coldqueue are A1in and A1out. The warmqueue is Am. We always
// cache pages, as opposed to pointers to pages for A1.
//
// This implementation adds support for multiple 2q caches.
//
// If we have more than one 2q cache, as bufs fall off the cold queue for recycling, bufs that
// have been warm before (which retain the B_WARM flag in addition to B_COLD) can be put into
// the hot queue of a second level 2Q cache. buffers which are only B_COLD are recycled. Bufs
// falling off the last cache's cold queue are always recycled.

/// Which queue of a cache `chillbufs` works on (the C compares the queue's address with the
/// cache's members).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ChillQueue {
    /// `cache->hotqueue`, counted by `hotbufpages`.
    Hot,
    /// `cache->warmqueue`, counted by `warmbufpages`.
    Warm,
}

/// `bufcache_init`: the empty queues (a host test's fresh cache forgets what was there).
fn bufcache_init(g: &mut Cache) {
    kassert!(g.hot.is_empty() && g.warm.is_empty() && g.cold.is_empty() && g.dirty.is_empty());
    g.hotbufpages = 0;
    g.warmbufpages = 0;
    g.cachepages = 0;
}

/// if the buffer caches have shrunk, we may need to rebalance our queues.
fn bufcache_adjust(g: &mut Cache) {
    while chillbufs(g, ChillQueue::Warm) || chillbufs(g, ChillQueue::Hot) {}
}

/// The pages of a buffer's memory.
fn bufpages_of(bp: &Buf) -> i64 {
    atop(bp.bufsize() as usize) as i64
}

/// The next victim of the cache: the first buffer of the cold, else warm, else hot queue,
/// as a counted link (`bufcache_getcleanbuf`'s choice).
fn bufcache_first(g: &Cache) -> Option<SecondaryRef<ByBufhead>> {
    g.cold
        .first()
        .or_else(|| g.warm.first())
        .or_else(|| g.hot.first())
        .map(counted)
}

/// `discard_buffer(bp)`: takes the idle clean buffer `r` names out of the cache and frees it.
fn discard_buffer(g: &mut Cache, r: SecondaryRef<ByBufhead>) {
    let Some(bp) = r.get() else {
        kpanic(format_args!("discard_buffer: no buffer"));
    };
    bufcache_take(g, bp);
    if let Some(vb) = bp.vnode_bufs() {
        vb.st.get_mut(g).tree.remove(bp);
        brelvp(g, bp);
    }
    // The buffer is not busy: its ownership taken for the free, B_BUSY left clear.
    let t = BufRef::acquire(g, r);
    buf_put(g, t);
}

/// `bufcache_recover_pages(discard, howmany)`: frees clean buffers, oldest first, until
/// `howmany` pages came back; the pages freed.
fn bufcache_recover_pages(g: &mut Cache, _discard: bool, howmany: i64) -> i64 {
    let mut recovered = 0;
    while recovered < howmany {
        let Some(r) = bufcache_first(g) else {
            break;
        };
        recovered += r.get().map_or(0, bufpages_of);
        discard_buffer(g, r);
    }
    recovered
}

/// `bufcache_take(bp)`: takes a free buffer off its cache queue (clean or dirty), to be
/// acquired.
pub fn bufcache_take(g: &mut Cache, bp: &Buf) {
    kassert!(bp.isset(B_BC));

    let pages = bufpages_of(bp);

    // TRACEPOINT(vfs, bufcache_take, ...): dt(4), not configured.

    let m = if !bp.isset(B_DELWRI) {
        let m = if bp.isset(B_COLD) {
            g.cold.remove(bp)
        } else if bp.isset(B_WARM) {
            g.warmbufpages -= pages;
            g.warm.remove(bp)
        } else {
            g.hotbufpages -= pages;
            g.hot.remove(bp)
        };
        bc_add(&BCSTATS.numcleanpages, -pages);
        g.cachepages -= pages;
        m
    } else {
        bc_add(&BCSTATS.numdirtypages, -pages);
        bc_add(&BCSTATS.delwribufs, -1);
        g.dirty.remove(bp)
    };
    drop(m);
}

/// move buffers from a hot or warm queue to a cold queue in a cache
fn chillbufs(g: &mut Cache, which: ChillQueue) -> bool {
    // We limit the hot queue to be small, with a max of 4096 pages. We limit the warm queue
    // to half the cache size.
    //
    // We impose a minimum size of 96 to prevent too much "wobbling".
    let (queuepages, limit) = match which {
        ChillQueue::Hot => (g.hotbufpages, (g.cachepages / 20).min(4096)),
        ChillQueue::Warm => (g.warmbufpages, g.cachepages / 2),
    };

    if queuepages > 96 && queuepages > limit {
        let m = match which {
            ChillQueue::Hot => g.hot.pop_head(),
            ChillQueue::Warm => g.warm.pop_head(),
        };
        let Some(m) = m else {
            kpanic(format_args!("inconsistent bufpage counts"));
        };
        let pages = bufpages_of(m.get());
        match which {
            ChillQueue::Hot => g.hotbufpages -= pages,
            ChillQueue::Warm => g.warmbufpages -= pages,
        }
        // we do not clear B_WARM
        m.get().set(B_COLD);
        g.cold.push_tail(m);
        return true;
    }
    false
}

/// `bufcache_release(bp)`: puts a buffer that is being released on the right cache queue:
/// the dirty queue, the warm queue for a buffer that was warm or cold, the hot queue for the
/// rest.
pub fn bufcache_release(g: &mut Cache, bp: &Buf) {
    kassert!(bp.isset(B_BC));
    let pages = bufpages_of(bp);

    // TRACEPOINT(vfs, bufcache_rel, ...): dt(4), not configured.

    let m = Member::counted(bp);
    if !bp.isset(B_DELWRI) {
        let which = if bp.isset(B_WARM | B_COLD) {
            bp.set(B_WARM);
            bp.clr(B_COLD);
            g.warmbufpages += pages;
            g.warm.push_tail(m);
            ChillQueue::Warm
        } else {
            g.hotbufpages += pages;
            g.hot.push_tail(m);
            ChillQueue::Hot
        };
        bc_add(&BCSTATS.numcleanpages, pages);
        g.cachepages += pages;
        let _ = chillbufs(g, which);
    } else {
        bc_add(&BCSTATS.numdirtypages, pages);
        bc_add(&BCSTATS.delwribufs, 1);
        g.dirty.push_tail(m);
    }
}

// HIBERNATE (hibernate_suspend_bufcache, hibernate_resume_bufcache): not configured.

// From vfs_subr.c: the vnode's buffer half.

/// Associates a buffer with a vnode (`bgetvp`): the vnode is held, the buffer goes on its
/// clean list. Called with the cache's lock held.
fn bgetvp(g: &mut Cache, vb: &'static VnodeBufs, bp: &Buf) {
    if bp.vnode_bufs().is_some() {
        kpanic(format_args!("bgetvp: not free"));
    }
    let owner = vb.owner();
    owner.vhold();
    bp.set_vnode_bufs(Some(vb));
    let vt = owner.vtype();
    if vt == VType::VBLK || vt == VType::VCHR {
        bp.set_dev(owner.rdev());
    } else {
        bp.set_dev(NODEV);
    }
    // Insert onto list for new vnode.
    vb.st.get_mut(g).clean.push_head(Member::counted(bp));
}

/// Takes `bp` off its vnode's clean or dirty list, if it is on one (`bufremvn`).
fn bufremvn(g: &mut Cache, vb: &'static VnodeBufs, bp: &Buf) {
    let st = vb.st.get_mut(g);
    if st.clean.contains(bp) {
        drop(st.clean.remove(bp));
    } else if st.dirty.contains(bp) {
        drop(st.dirty.remove(bp));
    }
}

/// Disassociates a buffer from its vnode (`brelvp`): off the vnode's lists (and the vnode
/// off the syncer's worklist when it has no dirty buffer left), the vnode dropped. Called
/// with the cache's lock held.
fn brelvp(g: &mut Cache, bp: &Buf) {
    let Some(vb) = bp.vnode_bufs() else {
        kpanic(format_args!("brelvp: NULL"));
    };
    // Delete from old vnode list, if on one.
    bufremvn(g, vb, bp);
    if vb.bioflag() & VBIOONSYNCLIST != 0 && vb.st.get(g).dirty.is_empty() {
        vb.clr_bioflag(VBIOONSYNCLIST);
        vb.owner().syncer_remove();
    }
    bp.set_vnode_bufs(None);

    vb.owner().vdrop();
}

/// Disassociates the buffer of the token `t` from its vnode (`brelvp`), for `physio` and the
/// legacy callers outside the cache.
pub fn brelvp_buf(t: &BufRef) {
    let mut g = CACHE.lock();
    brelvp(&mut g, t.buf());
}

/// Replaces the vnode associated with the buffer, if any, with `newvb` (`buf_replacevnode`).
/// If an output I/O is pending on the buffer, the old vnode's I/O count is moved. The buffer
/// ends on no vnode list.
pub fn buf_replacevnode(t: &BufRef, newvb: &'static VnodeBufs) {
    let mut g = CACHE.lock();
    let bp = t.buf();
    let oldvb = bp.vnode_bufs();

    if oldvb.is_some() {
        brelvp(&mut g, bp);
    }

    if bp.flags() & (B_READ | B_DONE) == 0 {
        newvb.numoutput.fetch_add(1, Mem::Relaxed); // put it on swapdev
        vwakeup_locked(oldvb);
    }

    bgetvp(&mut g, newvb, bp);
    bufremvn(&mut g, newvb, bp);
}

/// Puts `bp` on its vnode's clean or dirty list as its `B_DELWRI` says, and a vnode getting
/// its first dirty buffer on the syncer's worklist (`reassignbuf`). Called with the cache's
/// lock held.
fn reassignbuf(g: &mut Cache, bp: &Buf) {
    let Some(vb) = bp.vnode_bufs() else {
        kpanic(format_args!("reassignbuf: NULL"));
    };

    // Delete from old vnode list, if on one.
    bufremvn(g, vb, bp);

    // If dirty, put on list of dirty buffers; otherwise insert onto list of clean buffers.
    if !bp.isset(B_DELWRI) {
        if vb.bioflag() & VBIOONSYNCLIST != 0 && vb.st.get(g).dirty.is_empty() {
            vb.clr_bioflag(VBIOONSYNCLIST);
            vb.owner().syncer_remove();
        }
        vb.st.get_mut(g).clean.push_head(Member::counted(bp));
    } else {
        if vb.bioflag() & VBIOONSYNCLIST == 0 {
            vb.owner().syncer_add_dirty();
        }
        vb.st.get_mut(g).dirty.push_head(Member::counted(bp));
    }
}

/// Waits for all outstanding I/Os of the vnode to complete (`vwaitforio`); the sleep's error
/// (`slpflag`'s `PCATCH`, `timeo`).
pub fn vwaitforio(
    vb: &'static VnodeBufs,
    slpflag: i32,
    wmesg: &'static str,
    timeo: u64,
) -> Result<(), Errno> {
    let mut g = CACHE.lock();
    while vb.numoutput.load(Mem::Relaxed) != 0 {
        vb.set_bioflag(VBIOWAIT);
        g.sleep_nsec(
            &raw const vb.numoutput,
            slpflag | (PRIBIO + 1),
            wmesg,
            timeo,
        )?;
    }
    Ok(())
}

/// Updates the outstanding I/O count and wakes a waiter (`vwakeup`). Called with the cache's
/// lock held.
fn vwakeup_locked(vb: Option<&'static VnodeBufs>) {
    if let Some(vb) = vb {
        if vb.numoutput.load(Mem::Relaxed) == 0 {
            kpanic(format_args!("vwakeup: neg numoutput"));
        }
        let n = vb.numoutput.fetch_sub(1, Mem::Relaxed) - 1;
        if vb.bioflag() & VBIOWAIT != 0 && n == 0 {
            vb.clr_bioflag(VBIOWAIT);
            wakeup(&raw const vb.numoutput);
        }
    }
}

/// `vwakeup(vp)`: a write on the vnode is over (a driver's own buffer's: `softraid`).
pub fn vwakeup(vb: Option<&'static VnodeBufs>) {
    let _g = CACHE.lock();
    vwakeup_locked(vb);
}

/// What a scan of a vnode's dirty buffers ([`vnode_dirty_scan`]) does with the buffer it is
/// at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirtyPick {
    /// Go on to the next one.
    Skip,
    /// Take it off its cache queue and make it busy: the scan gives its token (`bremfree`,
    /// `buf_acquire`). Only for a buffer that is not busy.
    Take,
    /// Wait for it to be released (`B_WANTED`, a sleep on the buffer): the scan ends.
    Wait,
}

/// What a dirty scan found.
pub enum DirtyScan {
    /// The token of the buffer the scan took.
    Taken(BufRef),
    /// The scan slept on a busy buffer; the sleep's result (`PCATCH`, the timeout).
    Waited(Result<(), Errno>),
    /// No buffer was taken or waited for.
    Done,
}

/// One pass over `vb`'s dirty list from its head (`LIST_FOREACH(bp, &vp->v_dirtyblkhd,
/// b_vnbufs)` at `splbio`, the C's fsync loops: `spec_fsync`, `ffs_fsync`, `nfs_flush`,
/// `vflushbuf`, `nfssvc_iod`'s write-behind), `pick` deciding at each buffer: the first one
/// taken ends the scan with its token, as the C's loops restart from the head after each
/// write. `pick` runs under the cache's lock: it reads and sets flags, nothing else of the
/// cache. A buffer to wait for is marked `B_WANTED` and slept on with `slpflag | (PRIBIO +
/// 1)`, `wmesg` and `slptimeo`.
pub fn vnode_dirty_scan(
    vb: &'static VnodeBufs,
    wmesg: &'static str,
    slpflag: i32,
    slptimeo: u64,
    mut pick: impl FnMut(&Buf) -> DirtyPick,
) -> DirtyScan {
    let mut g = CACHE.lock();
    let found = {
        let st = vb.st.get(&g);
        let mut found = None;
        for bp in st.dirty.iter() {
            match pick(bp) {
                DirtyPick::Skip => continue,
                p => {
                    found = Some((counted(bp), p));
                    break;
                }
            }
        }
        found
    };
    let Some((r, p)) = found else {
        return DirtyScan::Done;
    };
    let Some(bp) = r.get() else {
        kpanic(format_args!("vnode_dirty_scan: a link to nothing"));
    };
    if p == DirtyPick::Wait {
        bp.set(B_WANTED);
        let chan = ptr::from_ref(bp);
        drop(r);
        return DirtyScan::Waited(g.sleep_nsec(chan, slpflag | (PRIBIO + 1), wmesg, slptimeo));
    }
    if bp.isset(B_BUSY) {
        kpanic(format_args!("vnode_dirty_scan: buffer {:p} taken busy", bp));
    }
    bufcache_take(&mut g, bp);
    DirtyScan::Taken(buf_acquire(&mut g, r))
}

/// `f` on each of `vb`'s dirty buffers, under the cache's lock (`ffs_fsync`'s clearing of
/// `B_SCANNED`, `nfs_clearcommit`'s of `B_NEEDCOMMIT`): flags only.
pub fn vnode_dirty_for_each(vb: &VnodeBufs, mut f: impl FnMut(&Buf)) {
    let g = CACHE.lock();
    for bp in vb.st.get(&g).dirty.iter() {
        f(bp);
    }
}

/// The first buffer of `l` from its head, the metadata (negative logical blocks) skipped when
/// `savemeta` (`vinvalbuf`'s `V_SAVEMETA`).
fn first_kept(l: &SecondaryList<ByVnbufs>, savemeta: bool) -> Option<&Buf> {
    let mut b = l.first();
    if savemeta {
        while let Some(bp) = b
            && bp.lblkno() < 0
        {
            b = l.next(bp);
        }
    }
    b
}

/// `vinvalbuf`'s buffer half: flushes out (`save`: a delayed write is written with
/// `VOP_BWRITE`) and invalidates every buffer of the vnode, the clean list's first, then the
/// dirty list's (the metadata kept for `savemeta`), waiting for a busy one
/// (`slpflag | (PRIBIO + 1)`, "vinvalbuf", `slptimeo`: its error ends the call). The C walks
/// a list from its head and restarts after each write or sleep; here every step restarts
/// from the head (an invalidated buffer has left the list). Every 100 buffers it yields.
/// Panics when, without `savemeta`, a buffer is left (`vinvalbuf: flush failed`).
pub fn vinvalbuf_bufs(
    vb: &'static VnodeBufs,
    save: bool,
    savemeta: bool,
    slpflag: i32,
    slptimeo: u64,
) -> Result<(), Errno> {
    let mut count = 0;
    loop {
        // loop:
        let mut g = CACHE.lock();
        let first = {
            let st = vb.st.get(&g);
            first_kept(&st.clean, savemeta)
                .or_else(|| first_kept(&st.dirty, savemeta))
                .map(counted)
        };
        let Some(r) = first else {
            let st = vb.st.get(&g);
            if !savemeta && (!st.dirty.is_empty() || !st.clean.is_empty()) {
                kpanic(format_args!("vinvalbuf: flush failed, vnode {:p}", vb));
            }
            return Ok(());
        };
        let Some(bp) = r.get() else {
            kpanic(format_args!("vinvalbuf: a link to nothing"));
        };
        if bp.isset(B_BUSY) {
            bp.set(B_WANTED);
            let chan = ptr::from_ref(bp);
            drop(r);
            g.sleep_nsec(chan, slpflag | (PRIBIO + 1), "vinvalbuf", slptimeo)?;
            count = 0;
            continue;
        }
        bufcache_take(&mut g, bp);
        // XXX Since there are no node locks for NFS, I believe there is a slight chance that a
        // delayed write will occur while sleeping just above, so check for it.
        if bp.isset(B_DELWRI) && save {
            let t = buf_acquire(&mut g, r);
            drop(g);
            let _ = vb.owner().bwrite(t);
            count = 0;
            continue;
        }
        let t = buf_acquire_nomap(&mut g, r);
        t.buf().set(B_INVAL);
        brelse_locked(&mut g, t);
        count += 1;
        // XXX Temporary workaround XXX
        //
        // If this is a gigantisch vnode and we are trashing a ton of buffers, drop the lock and
        // yield every so often. The longer term fix is to add a separate list for these invalid
        // buffers so we don't have to do the work to free these here.
        if count > 100 {
            drop(g);
            sched_pause();
            count = 0;
        }
    }
}

/// `vfs_syncwait`'s count of the busy buffers being written (`(b_flags & (B_BUSY | B_INVAL |
/// B_READ)) == B_BUSY` over `bufhead`).
pub fn bufhead_busy() -> i32 {
    let g = CACHE.lock();
    g.bufhead
        .iter()
        .filter(|bp| bp.flags() & (B_BUSY | B_INVAL | B_READ) == B_BUSY)
        .count() as i32
}

/// `vfs_syncwait`'s next delayed-write buffer to push: the head of the dirty queue (the idle
/// `B_DELWRI` buffers), taken off it and made busy; `None` when there is none.
pub fn take_dirty_buf() -> Option<BufRef> {
    let mut g = CACHE.lock();
    let r = g.dirty.first().map(counted)?;
    if let Some(bp) = r.get() {
        bufcache_take(&mut g, bp);
    }
    Some(buf_acquire(&mut g, r))
}

/// The cache forgotten, without freeing anything (a host test's fresh memory makes what the
/// last test left meaningless): `bufhead`, the queues and the arena leaked, the counters and
/// the tunables zeroed, the thread identities cleared (testing.md's resets).
#[cfg(any(test, feature = "doubles"))]
pub fn bio_test_reset() {
    let mut g = CACHE.lock();
    core::mem::forget(core::mem::replace(&mut *g, CacheState::new()));
    drop(g);
    BCSTATS.reset();
    for a in [
        &LODIRTYPAGES,
        &HIDIRTYPAGES,
        &TARGETPAGES,
        &BUFLOWPAGES,
        &BUFHIGHPAGES,
        &BUFBACKPAGES,
        &BUFPAGES,
    ] {
        a.store(0, Mem::Relaxed);
    }
    BUFKVM.store(0, Mem::Relaxed);
    BUFCACHEPERCENT.store(BUFCACHEPERCENT_DEFAULT, Mem::Relaxed);
    NOBUFFERS.store(0, Mem::Relaxed);
    NEEDBUFFER.store(0, Mem::Relaxed);
    BUF_NEEDVA.store(0, Mem::Relaxed);
    CLEANER.store(0, Mem::Relaxed);
    SYNCER.store(0, Mem::Relaxed);
    for n in &QUEUE_NOTE {
        n.word.store(0, Mem::Relaxed);
        n.bq.set(None);
    }
    NOTES_CLAIMED.store(0, Mem::Relaxed);
    NOTES_WANTED.store(0, Mem::Relaxed);
}

const _: () = assert!(DEV_BSHIFT == 9);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the buffer cache over `testutil::BufTestVnode` (LZ's `blkfs`): `getblk`/
    // `brelse`/`incore` bookkeeping, `bread` through the vnode's strategy (hits, misses,
    // errors), delayed and synchronous writes, the 2Q queues, `geteblk`, the token's rules,
    // `bread_cluster`, the vnode's output count, and the kva of `vfs_biomem` (a slot stolen
    // from an idle buffer, a released buffer freed by `buf_unmap`).

    use std::boxed::Box;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use super::*;
    use crate::fs::buf::B_RELEASED;
    use crate::testutil::{BufTestVnode, bufcache_install};

    /// The memory each test loads.
    const BYTES: usize = 32 << 20;
    /// A sector.
    const DEV_BSIZE: usize = 1 << DEV_BSHIFT;
    /// A page's sectors.
    const SECTORS: Daddr = (PAGE_SIZE / DEV_BSIZE) as Daddr;

    /// A fresh cache of 2,048 pages (128 kva slots) and a vnode over a disk of `blocks`
    /// sectors.
    fn setup(blocks: usize) -> (MutexGuard<'static, ()>, &'static BufTestVnode) {
        let g = bufcache_install(BYTES, 2048);
        (g, BufTestVnode::new(blocks))
    }

    fn qlen(which: fn(&CacheState) -> &SecondaryList<ByFreelist>) -> usize {
        let g = CACHE.lock();
        which(&g).iter().count()
    }

    fn on_clean(vb: &'static VnodeBufs, bp: &Buf) -> bool {
        let g = CACHE.lock();
        vb.st.get(&g).clean.contains(bp)
    }

    fn on_dirty(vb: &'static VnodeBufs, bp: &Buf) -> bool {
        let g = CACHE.lock();
        vb.st.get(&g).dirty.contains(bp)
    }

    fn stat(c: &AtomicI64) -> i64 {
        c.load(Mem::Relaxed)
    }

    #[test]
    fn getblk_brelse_and_incore_keep_the_books() {
        let (_g, tv) = setup(256);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;

        assert!(!incore(vb, 5));
        let mut t = getblk(vb, 5, size, 0, INFSLP).expect("a buffer");
        let bp: *const Buf = t.buf();
        assert!(t.buf().isset(B_BUSY) && t.buf().isset(B_BC) && !t.buf().isset(B_CACHE));
        assert_eq!((t.buf().lblkno(), t.buf().blkno()), (5, 5));
        assert!(on_clean(vb, t.buf()));
        assert_eq!(tv.holdcnt.load(Mem::Relaxed), 1);
        assert_eq!(stat(&BCSTATS.numbufs), 1);
        assert_eq!(stat(&BCSTATS.numbufpages), 1);
        assert_eq!(stat(&BCSTATS.busymapped), 1);
        assert!(incore(vb, 5));
        // The mapping is usable.
        t.data().fill(0xa5);
        brelse(t);
        assert_eq!(qlen(|c| &c.hot), 1);
        assert_eq!(stat(&BCSTATS.numcleanpages), 1);
        assert_eq!(stat(&BCSTATS.busymapped), 0);

        // A second getblk finds it in the cache, with its data.
        let t = getblk(vb, 5, size, 0, INFSLP).expect("the buffer");
        assert!(ptr::eq(t.buf(), bp));
        assert!(t.buf().isset(B_CACHE));
        assert_eq!(stat(&BCSTATS.cachehits), 1);
        assert_eq!(qlen(|c| &c.hot), 0);
        assert!(t.bytes().iter().all(|&b| b == 0xa5));
        brelse(t);

        // An invalidated buffer is freed by brelse, and incore no longer finds the block.
        let t = getblk(vb, 5, size, 0, INFSLP).expect("the buffer");
        t.buf().set(B_INVAL);
        brelse(t);
        assert!(!incore(vb, 5));
        assert!(vb.clean_is_empty() && vb.tree_is_empty());
        assert_eq!(tv.holdcnt.load(Mem::Relaxed), 0);
        assert_eq!(stat(&BCSTATS.numbufs), 0);
        assert_eq!(stat(&BCSTATS.numbufpages), 0);
    }

    #[test]
    fn bread_reads_once_and_reports_errors() {
        let (_g, tv) = setup(256);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;

        let (t, error) = bread(vb, 8, size);
        assert_eq!(error, Ok(()));
        assert!(t.buf().isset(B_DONE) && !t.buf().isset(B_CACHE));
        assert_eq!(t.bytes()[0], 8);
        assert_eq!(t.bytes()[DEV_BSIZE], 9);
        assert_eq!(tv.reads.load(Mem::Relaxed), 1);
        assert_eq!(stat(&BCSTATS.numreads), 1);
        assert_eq!(stat(&BCSTATS.pendingreads), 0);
        brelse(t);

        // The second read is a cache hit: no I/O.
        let (t, error) = bread(vb, 8, size);
        assert_eq!(error, Ok(()));
        assert!(t.buf().isset(B_CACHE));
        assert_eq!(tv.reads.load(Mem::Relaxed), 1);
        brelse(t);

        // A failing read returns the buffer and the error; brelse throws it away.
        tv.bad.store(40, Mem::Relaxed);
        let (t, error) = bread(vb, 40, size);
        assert_eq!(error, Err(Errno::EIO));
        brelse(t);
        assert!(!incore(vb, 40));

        // breadn starts the read-ahead too.
        let (t, error) = breadn(vb, 16, size, &[24], &[size]);
        assert_eq!(error, Ok(()));
        assert!(incore(vb, 24));
        assert_eq!(tv.reads.load(Mem::Relaxed), 3);
        brelse(t);
    }

    #[test]
    fn delayed_writes_go_dirty_and_a_bwrite_writes_them() {
        let (_g, tv) = setup(256);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;

        let mut t = getblk(vb, 32, size, 0, INFSLP).expect("a buffer");
        t.data().fill(0x5a);
        let bp: *const Buf = t.buf();
        bdwrite(t);
        let t = getblk(vb, 32, size, 0, INFSLP).expect("the dirty buffer");
        assert!(ptr::eq(t.buf(), bp) && t.buf().isset(B_DELWRI));
        assert!(on_dirty(vb, t.buf()));
        assert!(vb.bioflag() & VBIOONSYNCLIST != 0);
        assert_eq!(tv.synced.load(Mem::Relaxed), 1);
        assert_eq!(tv.writes.load(Mem::Relaxed), 0);
        // On the dirty queue while it was idle (taken off it now, busy).
        assert_eq!(stat(&BCSTATS.delwribufs), 0);
        brelse(t);
        assert_eq!(stat(&BCSTATS.delwribufs), 1);
        assert_eq!(stat(&BCSTATS.numdirtypages), 1);

        // The cleaner's step: the dirty buffer written synchronously.
        let t = getblk(vb, 32, size, 0, INFSLP).expect("the dirty buffer");
        assert_eq!(bwrite(t), Ok(()));
        assert_eq!(tv.writes.load(Mem::Relaxed), 1);
        assert!(
            lock_disk(tv)[32 * DEV_BSIZE..33 * DEV_BSIZE]
                .iter()
                .all(|&b| b == 0x5a)
        );
        assert!(vb.dirty_is_empty());
        assert!(vb.bioflag() & VBIOONSYNCLIST == 0);
        assert_eq!(vb.numoutput(), 0);
        assert_eq!(stat(&BCSTATS.delwribufs), 0);
        assert_eq!(stat(&BCSTATS.pendingwrites), 0);
        let t = getblk(vb, 32, size, 0, INFSLP).expect("the clean buffer");
        assert!(on_clean(vb, t.buf()) && !t.buf().isset(B_DELWRI));
        brelse(t);

        // An asynchronous write is released by biodone.
        let mut t = getblk(vb, 48, size, 0, INFSLP).expect("a buffer");
        t.data().fill(0x77);
        bawrite(t);
        assert_eq!(lock_disk(tv)[48 * DEV_BSIZE], 0x77);
        let t = getblk(vb, 48, size, 0, INFSLP).expect("released");
        assert!(!t.buf().isset(B_ASYNC));
        brelse(t);
    }

    #[test]
    fn a_cached_first_block_whose_read_is_started_gets_no_read_ahead() {
        let (_g, tv) = setup(512);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;
        tv.run.store(3, Mem::Relaxed);
        // Block 0 cached but not valid (released without B_DONE): getblk finds it (B_CACHE)
        // and bio_doread starts its read; the C tests the first buffer's B_CACHE in every
        // case, so no cluster is read.
        let t = getblk(vb, 0, size, 0, INFSLP).expect("a buffer");
        assert!(!t.buf().isset(B_DONE));
        brelse(t);
        assert!(incore(vb, 0));
        let (t, error) = bread_cluster(vb, 0, size);
        assert_eq!(error, Ok(()));
        assert!(t.buf().isset(B_CACHE));
        brelse(t);
        assert_eq!(
            tv.reads.load(Mem::Relaxed),
            1,
            "block 0's own read, no cluster"
        );
        assert!(!incore(vb, 1));
    }

    /// LZ's `delayed_writes_go_dirty_and_vflushbuf_writes_them`, on the kernel's halves:
    /// `vflushbuf`'s loop over `vnode_dirty_scan` (as `bsd`'s `vflushbuf` runs it) and
    /// `vinvalbuf_bufs` with `V_SAVE`.
    #[test]
    fn a_dirty_scan_flushes_delayed_writes_and_vinvalbuf_saves_them() {
        let (_g, tv) = setup(256);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;

        let mut t = getblk(vb, 32, size, 0, INFSLP).expect("a buffer");
        t.data().fill(0x5a);
        let bp: *const Buf = t.buf();
        bdwrite(t);
        {
            let g = CACHE.lock();
            let b = vb.st.get(&g).dirty.first().expect("on the dirty list");
            assert!(ptr::eq(b, bp) && b.isset(B_DELWRI) && !b.isset(B_BUSY));
            assert!(g.dirty.first().is_some_and(|d| ptr::eq(d, bp)));
        }
        assert!(vb.bioflag() & VBIOONSYNCLIST != 0);
        assert_eq!(stat(&BCSTATS.delwribufs), 1);
        assert_eq!(stat(&BCSTATS.numdirtypages), 1);
        assert_eq!(tv.writes.load(Mem::Relaxed), 0);

        // vflushbuf(vp, 1): every dirty buffer taken by the scan and written.
        let pick = |b: &Buf| {
            if b.isset(B_BUSY) {
                return DirtyPick::Skip;
            }
            assert!(b.isset(B_DELWRI), "vflushbuf: not dirty");
            DirtyPick::Take
        };
        let mut taken = 0;
        while let DirtyScan::Taken(t) = vnode_dirty_scan(vb, "vflushbuf", 0, INFSLP, pick) {
            assert!(t.buf().vnode_bufs().is_some_and(|v| ptr::eq(v, vb)));
            bawrite(t);
            taken += 1;
        }
        assert_eq!(taken, 1);
        assert_eq!(vwaitforio(vb, 0, "vflushbuf", INFSLP), Ok(()));
        assert_eq!(tv.writes.load(Mem::Relaxed), 1);
        assert!(
            lock_disk(tv)[32 * DEV_BSIZE..32 * DEV_BSIZE + PAGE_SIZE]
                .iter()
                .all(|&b| b == 0x5a)
        );
        assert!(vb.dirty_is_empty());
        {
            let g = CACHE.lock();
            assert!(vb.st.get(&g).clean.iter().any(|b| ptr::eq(b, bp)));
        }
        assert!(vb.bioflag() & VBIOONSYNCLIST == 0);
        assert_eq!(vb.numoutput(), 0);
        assert_eq!(stat(&BCSTATS.delwribufs), 0);
        assert_eq!(stat(&BCSTATS.pendingwrites), 0);

        // A synchronous bwrite writes and releases.
        let mut t = getblk(vb, 48, size, 0, INFSLP).expect("a buffer");
        t.data().fill(0x77);
        let bp48: *const Buf = t.buf();
        assert_eq!(bwrite(t), Ok(()));
        assert!(idle_on_its_lists(vb, bp48));
        assert_eq!(lock_disk(tv)[48 * DEV_BSIZE], 0x77);

        // vinvalbuf with V_SAVE writes a dirty buffer before throwing it away.
        let mut t = getblk(vb, 64, size, 0, INFSLP).expect("a buffer");
        t.data().fill(0x33);
        bdwrite(t);
        assert_eq!(vinvalbuf_bufs(vb, true, false, 0, INFSLP), Ok(()));
        assert_eq!(lock_disk(tv)[64 * DEV_BSIZE], 0x33);
        assert!(vb.dirty_is_empty() && vb.clean_is_empty());
        assert_eq!(stat(&BCSTATS.numbufs), 0);
    }

    /// Whether the buffer at `bp` is on `vb`'s lists and idle (`!B_BUSY`).
    fn idle_on_its_lists(vb: &'static VnodeBufs, bp: *const Buf) -> bool {
        let g = CACHE.lock();
        let st = vb.st.get(&g);
        st.clean
            .iter()
            .chain(st.dirty.iter())
            .any(|b| ptr::eq(b, bp) && !b.isset(B_BUSY))
    }

    #[test]
    fn a_dirty_scan_skips_takes_and_ends() {
        let (_g, tv) = setup(256);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;
        for blk in [8, 16] {
            let t = getblk(vb, blk, size, 0, INFSLP).expect("a buffer");
            bdwrite(t);
        }
        // Everything skipped: the scan ends with nothing.
        assert!(matches!(
            vnode_dirty_scan(vb, "test", 0, INFSLP, |_| DirtyPick::Skip),
            DirtyScan::Done
        ));
        // The first buffer the pick takes ends the scan, busy and off the cache's queue.
        let scan = vnode_dirty_scan(vb, "test", 0, INFSLP, |b| {
            if b.lblkno() == 16 {
                DirtyPick::Take
            } else {
                DirtyPick::Skip
            }
        });
        let DirtyScan::Taken(t) = scan else {
            panic!("block 16 taken");
        };
        assert_eq!(t.buf().lblkno(), 16);
        assert!(t.buf().isset(B_BUSY) && t.buf().isset(B_DELWRI));
        assert!(!t.buf().b_freelist_linked());
        // A busy buffer is skipped by a pick that skips the busy ones.
        assert!(matches!(
            vnode_dirty_scan(vb, "test", 0, INFSLP, |b| {
                if b.isset(B_BUSY) || b.lblkno() != 16 {
                    DirtyPick::Skip
                } else {
                    DirtyPick::Take
                }
            }),
            DirtyScan::Done
        ));
        brelse(t);
        assert_eq!(stat(&BCSTATS.delwribufs), 2);
    }

    #[test]
    fn vinvalbuf_keeps_the_metadata_for_savemeta_and_waits_for_nothing_idle() {
        let (_g, tv) = setup(256);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;
        // A data block (clean, valid) and a metadata block (negative lblkno, dirty).
        let (t, error) = bread(vb, 8, size);
        assert_eq!(error, Ok(()));
        brelse(t);
        let t = getblk(vb, -12, size, 0, INFSLP).expect("a metadata buffer");
        bdwrite(t);
        // V_SAVEMETA without V_SAVE: the data invalidated, the metadata kept, nothing written.
        assert_eq!(vinvalbuf_bufs(vb, false, true, 0, INFSLP), Ok(()));
        assert!(!incore(vb, 8));
        assert!(vb.clean_is_empty());
        {
            let g = CACHE.lock();
            let st = vb.st.get(&g);
            assert_eq!(st.dirty.iter().map(Buf::lblkno).collect::<Vec<_>>(), [-12]);
        }
        assert_eq!(tv.writes.load(Mem::Relaxed), 0);
        // Without it, the dirty metadata is thrown away too (no V_SAVE: not written).
        assert_eq!(vinvalbuf_bufs(vb, false, false, 0, INFSLP), Ok(()));
        assert!(vb.dirty_is_empty() && vb.clean_is_empty());
        assert_eq!(tv.writes.load(Mem::Relaxed), 0);
        assert_eq!(stat(&BCSTATS.delwribufs), 0);
    }

    /// `bwrite`'s queue notes (the C's `bp->b_bq` read after the strategy call): the queue
    /// noted for the claimed buffer only, cleared by `bufq_done`, a note reused for another
    /// buffer left alone by a late `done`, and the table's bound.
    #[test]
    fn a_bwrite_note_names_its_buffers_queue_until_the_io_is_done() {
        let (_g, _tv) = setup(64);
        let bq: &'static super::super::bufq::Bufq =
            Box::leak(Box::new(super::super::bufq::Bufq::new()));
        let a: &'static Buf = Box::leak(Box::new(Buf::new()));
        let b: &'static Buf = Box::leak(Box::new(Buf::new()));

        // Queued and not done: the writer waits on that queue.
        let n = note_claim(a);
        note_queued(b, bq); // another buffer's queue is not noted
        assert!(QUEUE_NOTE[n].bq.get().is_none());
        note_queued(a, bq);
        assert!(note_take(n).is_some_and(|q| ptr::eq(q, bq)));
        assert_eq!(NOTES_CLAIMED.load(Mem::SeqCst), 0);

        // Done before the strategy call returned: no wait (b_bq is NULL again).
        let n = note_claim(a);
        note_queued(a, bq);
        note_done(a);
        assert!(note_take(n).is_none());

        // Queued on none (a strategy that finished the I/O itself): no wait.
        let n = note_claim(a);
        assert!(note_take(n).is_none());

        // A note given back and claimed for another buffer: a late done of the first leaves
        // it alone.
        let n = note_claim(a);
        assert!(note_take(n).is_none());
        let m = note_claim(b);
        note_queued(b, bq);
        note_done(a);
        assert!(note_take(m).is_some_and(|q| ptr::eq(q, bq)));

        // The table's bound: every note claimed, then none.
        let bufs: Vec<&'static Buf> = (0..QUEUE_NOTES)
            .map(|_| &*Box::leak(Box::new(Buf::new())))
            .collect();
        let held: Vec<usize> = bufs.iter().map(|bp| note_claim(bp)).collect();
        assert!(note_try_claim(a).is_none());
        for i in held {
            assert!(note_take(i).is_none());
        }
        assert!(note_try_claim(a).is_some_and(|i| note_take(i).is_none()));
    }

    /// The legacy boundary's `brelse` and `biodone` take the parked token in the cache's own
    /// section: the buffer released, or done and its token parked again for `biowait`; with
    /// no token parked they do nothing and say so.
    #[test]
    fn brelse_and_biodone_of_a_parked_token() {
        let (_g, tv) = setup(64);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;

        let t = getblk(vb, 8, size, 0, INFSLP).expect("a buffer");
        let r = t.counted().expect("a cache buffer");
        let bp = r.get().expect("the buffer");
        assert!(!brelse_parked(bp), "nothing parked while the token is held");
        Buf::park(t);
        assert!(brelse_parked(bp));
        assert!(!bp.is_held() && !bp.isset(B_BUSY));
        assert!(!brelse_parked(bp), "an idle buffer has no parked token");

        // A synchronous read done by a legacy driver: the token parked again, B_DONE set.
        let t = getblk(vb, 8, size, 0, INFSLP).expect("the buffer again");
        bc_add(&BCSTATS.pendingreads, 1);
        t.buf().set(B_READ);
        t.buf().clr(B_DONE);
        Buf::park(t);
        assert!(biodone_parked(bp));
        assert!(bp.isset(B_DONE) && bp.is_parked());
        assert!(biowait_buf(bp).is_ok());
        assert!(brelse_parked(bp));
        assert!(!biodone_parked(bp), "an idle buffer has no parked token");
        drop(r);
    }

    /// The legacy boundary's reads: the buffer comes back with its token parked (a miss's
    /// left parked by `biodone` for the wait, a hit's parked in getblk's own section), and
    /// the legacy `brelse` takes it back.
    #[test]
    fn a_legacy_read_gives_the_buffer_with_its_token_parked() {
        let (_g, tv) = setup(64);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;

        // A miss: read from the disk, the token parked by biodone and left there.
        let (r, rv) = bread_parked(vb, 2, size);
        assert_eq!(rv, Ok(()));
        let bp = r.get().expect("the buffer");
        assert!(bp.is_parked() && bp.isset(B_DONE | B_BUSY));
        assert_eq!(tv.reads.load(Mem::Relaxed), 1);
        assert!(brelse_parked(bp));
        drop(r);
        // A hit: no read, the token parked in getblk's section.
        let (r, rv) = bread_parked(vb, 2, size);
        assert_eq!(rv, Ok(()));
        let bp = r.get().expect("the buffer");
        assert!(bp.is_parked() && bp.isset(B_CACHE));
        assert_eq!(tv.reads.load(Mem::Relaxed), 1);
        assert!(brelse_parked(bp));
        drop(r);

        // getblk for a legacy caller: a new buffer parked after buf_get, a cached one in
        // getblk's section; neither is read.
        let r = getblk_parked(vb, 3, size, 0, INFSLP).expect("a new buffer");
        let bp = r.get().expect("the buffer");
        assert!(bp.is_parked() && !bp.isset(B_DONE));
        assert!(brelse_parked(bp));
        drop(r);
        let r = getblk_parked(vb, 3, size, 0, INFSLP).expect("the cached buffer");
        let bp = r.get().expect("the buffer");
        assert!(bp.is_parked() && bp.isset(B_CACHE));
        assert!(brelse_parked(bp));
        drop(r);
        assert_eq!(tv.reads.load(Mem::Relaxed), 1);
    }

    fn lock_disk(tv: &BufTestVnode) -> std::sync::MutexGuard<'_, Vec<u8>> {
        tv.disk.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn released_buffers_move_through_the_2q_queues() {
        let (_g, tv) = setup(1024);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;

        // 100 one-page buffers released in order: the hot queue holds at most 96 pages (the
        // minimum before chilling), the oldest four go cold.
        for blk in 0..100 {
            let t = getblk(vb, blk * SECTORS, size, 0, INFSLP).expect("a buffer");
            brelse(t);
        }
        assert_eq!(qlen(|c| &c.hot), 96);
        assert_eq!(qlen(|c| &c.cold), 4);
        {
            let g = CACHE.lock();
            assert_eq!((g.hotbufpages, g.cachepages), (96, 100));
            let cold: Vec<Daddr> = g.cold.iter().map(|b| b.lblkno() / SECTORS).collect();
            assert_eq!(cold, [0, 1, 2, 3]);
            assert!(g.cold.iter().all(|b| b.isset(B_COLD)));
        }

        // A cold buffer used again becomes warm.
        let t = getblk(vb, 2 * SECTORS, size, 0, INFSLP).expect("the buffer");
        assert!(t.buf().isset(B_CACHE));
        let bp: *const Buf = t.buf();
        brelse(t);
        {
            let g = CACHE.lock();
            let w = g.warm.first().expect("warm");
            assert!(ptr::eq(w, bp) && w.isset(B_WARM) && !w.isset(B_COLD));
            assert_eq!(g.warmbufpages, 1);
        }
        assert_eq!(qlen(|c| &c.cold), 3);

        // Recovering pages takes the cold ones first, then warm, then hot.
        {
            let mut g = CACHE.lock();
            assert_eq!(bufcache_recover_pages(&mut g, false, 4), 4);
        }
        assert_eq!(qlen(|c| &c.cold) + qlen(|c| &c.warm), 0);
        assert!(!incore(vb, 0) && !incore(vb, 2 * SECTORS));
        assert!(incore(vb, 4 * SECTORS));
        assert_eq!(stat(&BCSTATS.numbufs), 96);
    }

    #[test]
    fn geteblk_gives_an_anonymous_invalid_buffer() {
        let (_g, _tv) = setup(16);
        let t = geteblk(2 * PAGE_SIZE);
        assert!(t.buf().vnode_bufs().is_none());
        assert!(t.buf().isset(B_INVAL) && t.buf().isset(B_BUSY));
        assert_eq!(t.buf().bufsize(), 2 * PAGE_SIZE as i64);
        assert_eq!(t.buf().dev(), NODEV);
        assert!(t.is_mapped());
        brelse(t);
        assert_eq!(stat(&BCSTATS.numbufs), 0);
    }

    #[test]
    fn a_dropped_token_is_released() {
        let (_g, tv) = setup(64);
        let vb = &tv.bufs;
        let t = getblk(vb, 0, PAGE_SIZE as i32, 0, INFSLP).expect("a buffer");
        drop(t);
        assert_eq!(qlen(|c| &c.hot), 1);
        let t = getblk(vb, 0, PAGE_SIZE as i32, 0, INFSLP).expect("cached");
        assert!(t.buf().isset(B_CACHE));
        brelse(t);
    }

    #[test]
    fn a_drivers_callback_is_refused_on_a_cache_buffer() {
        fn done(_: &'static Buf) {}
        let (_g, tv) = setup(64);
        let vb = &tv.bufs;
        let t = getblk(vb, 0, PAGE_SIZE as i32, 0, INFSLP).expect("a buffer");
        t.buf().set(B_CALL);
        t.buf().set_iodone(Some(Iodone::Legacy(done)));
        let r = catch_unwind(AssertUnwindSafe(|| biodone(t)));
        assert!(
            r.is_err(),
            "a cache buffer's token never goes to a driver's callback"
        );
    }

    #[test]
    fn biodone_twice_panics() {
        let (_g, tv) = setup(64);
        let vb = &tv.bufs;
        let t = getblk(vb, 0, PAGE_SIZE as i32, 0, INFSLP).expect("a buffer");
        t.buf().set(B_DONE);
        let r = catch_unwind(AssertUnwindSafe(|| biodone(t)));
        assert!(r.is_err(), "biodone already");
    }

    #[test]
    fn the_vnodes_output_count_wakes_its_waiter() {
        let (_g, tv) = setup(64);
        let vb = &tv.bufs;
        assert_eq!(vwaitforio(vb, 0, "test", INFSLP), Ok(()));
        vb.numoutput_inc();
        vb.set_bioflag(VBIOWAIT);
        vwakeup(Some(vb));
        assert_eq!(vb.numoutput(), 0);
        assert!(vb.bioflag() & VBIOWAIT == 0);
        let r = catch_unwind(AssertUnwindSafe(|| vwakeup(Some(vb))));
        assert!(r.is_err(), "vwakeup: neg numoutput");
    }

    #[test]
    fn bread_cluster_reads_ahead_into_buffers_of_their_own() {
        let (_g, tv) = setup(512);
        let vb = &tv.bufs;
        let size = PAGE_SIZE as i32;
        // Blocks are pages here: lblkno i is at sector i * SECTORS on the disk.
        tv.run.store(3, Mem::Relaxed);
        let (t, error) = bread_cluster(vb, 0, size);
        assert_eq!(error, Ok(()));
        brelse(t);
        // One read for block 0, one for the cluster of blocks 1..4 (howmany = run + 1).
        assert_eq!(tv.reads.load(Mem::Relaxed), 2);
        for blk in 1..5 {
            assert!(incore(vb, blk), "block {blk} read ahead");
            let t = getblk(vb, blk, size, 0, INFSLP).expect("cached");
            assert!(t.buf().isset(B_CACHE));
            assert_eq!(t.buf().bufsize(), PAGE_SIZE as i64);
            // The identity bmap put the cluster at sector 1, a page (8 sectors) per block:
            // block `blk`'s first byte is sector 1 + 8 * (blk - 1)'s.
            assert_eq!(t.bytes()[0], (1 + 8 * (blk - 1)) as u8);
            assert!(!t.buf().pages_are_masters(&CACHE.lock()));
            brelse(t);
        }
        assert!(!incore(vb, 5));
        assert_eq!(stat(&BCSTATS.pendingreads), 0);
    }

    #[test]
    fn a_slot_is_stolen_from_an_idle_buffer_when_the_arena_is_used_up() {
        let (_g, tv) = setup(4096);
        let vb = &tv.bufs;
        let slots = stat(&BCSTATS.kvaslots) as Daddr;
        // Every slot taken by an idle mapped buffer, then one more buffer.
        for blk in 0..slots {
            let t = getblk(vb, blk * SECTORS, PAGE_SIZE as i32, 0, INFSLP).expect("a buffer");
            brelse(t);
        }
        assert_eq!(stat(&BCSTATS.kvaslots_avail), slots as i64);
        let mut t = getblk(vb, slots * SECTORS, PAGE_SIZE as i32, 0, INFSLP).expect("one more");
        // It took the least recently used buffer's slot, which lost its mapping.
        assert!(t.is_mapped());
        t.data()[0] = 1;
        assert_eq!(stat(&BCSTATS.kvaslots_avail), slots as i64 - 1);
        let g = CACHE.lock();
        let lru = g.bufhead.iter().find(|b| b.lblkno() == 0).expect("block 0");
        assert!(lru.data_addr() == 0 && !lru.idle_mapped(&g));
        assert!(!g.valist.contains(lru));
        drop(g);
        brelse(t);
        assert_eq!(stat(&BCSTATS.kvaslots_avail), slots as i64);
        // Block 0 maps again when it is used: a slot stolen from the next idle buffer.
        let t = getblk(vb, 0, PAGE_SIZE as i32, 0, INFSLP).expect("block 0");
        assert!(t.is_mapped() && t.buf().isset(B_CACHE));
        brelse(t);
    }

    #[test]
    fn a_released_buffer_is_freed_when_its_slot_is_taken() {
        let (_g, tv) = setup(256);
        let vb = &tv.bufs;
        let t = getblk(vb, 0, PAGE_SIZE as i32, 0, INFSLP).expect("a buffer");
        t.buf().set(B_INVAL);
        // brelse of an invalid mapped buffer: the pages go, the kva stays (B_RELEASED, at
        // the head of buf_valist), the header stays on bufhead.
        brelse(t);
        assert_eq!(stat(&BCSTATS.numbufs), 0);
        {
            let g = CACHE.lock();
            let bp = g.valist.first().expect("the released buffer");
            assert!(bp.isset(B_RELEASED) && bp.idle_mapped(&g) && !bp.has_pages(&g));
            assert_eq!(g.bufhead.iter().count(), 1);
        }
        // buf_unmap takes the slot and gives the buffer back to bufpl.
        let slot = {
            let mut g = CACHE.lock();
            super::super::biomem::buf_unmap(&mut g)
        };
        let g = CACHE.lock();
        assert!(g.bufhead.is_empty() && g.valist.is_empty());
        drop(g);
        drop(slot);
    }
}
/* </TESTS> */
