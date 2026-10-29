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
 *	@(#)buf.h	8.7 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/buf.h>`: the buffer header that describes an I/O operation in the kernel (`struct
//! buf`), its `B_*` flags and limits, and its owner's token: the `B_BUSY` buffer as a value.
//!
//! Upstream: sys/sys/buf.h @ 3ce1f3f79392
//! LZ: sys/sys/buf.rs@3c62ede68802
//!
//! A buffer is a `bufpl` item ([`BUFPL`]) that the buffer cache's `bufhead` owns
//! (`hal::owned::OwnedList`), linked through its own entries on the vnode's tree and lists,
//! the cache's queues, `buf_valist` and a disk's queue; physio's buffer is a `bufpl` item on
//! physio's own list; a driver may embed one in its softc (`softraid`'s `ccb_buf`, `fd`'s
//! format buffer) and hand it down by address. Locks: the cache's (`splbio`, `kernel::fs`'s
//! cache lock) for the links and the 2Q flags; the rest belongs to the buffer's owner, the
//! holder of its [`BufRef`].
//!
//! **The token.** `B_BUSY` is ownership: whoever set it may change the buffer, map and touch
//! its bytes, start its I/O, and must give it back (`brelse`, `bwrite`, `biodone`). Here that
//! owner holds a [`BufRef`]: made once per busy period (the cache's acquire, or
//! [`BufRef::foreign`] for a buffer a driver embeds), consumed by the call that ends it. It
//! holds what keeps the buffer alive (a counted link to a pool item that `bufhead` or physio
//! owns, which they refuse to free while counted; or the `'static` of an embedded one) and,
//! while the buffer is mapped, the view of its bytes (`hal::mapped::MappedBytes`), so
//! [`BufRef::data`] is the buffer's bytes as a slice, with no lock taken. While a driver or a
//! disk queue holds the buffer by address (`bsd`'s `&'static Buf`, the C's `struct buf *`),
//! the token is parked in the buffer ([`Buf::park`]) and taken back by whoever ends that
//! part (`biodone`, `bufq_dequeue`: [`Buf::take_parked`]). An idle mapped buffer keeps its
//! view in the buffer, for the next owner (the C's mapping that survives `buf_release` on
//! `buf_valist`).
//!
//! ## Deviations
//! - `NOLIST`, the sentinel the C writes into `b_vnbufs.le_next` and `b_freelist.tqe_next` of
//!   a buffer on no list, is each entry's own state (`SecondaryEntry::is_linked`).
//! - `union bufq_data` is one entry, `b_bufq`, which both disciplines link through.
//! - `b_error` is an `Option<Errno>` at the API ([`Buf::error`]); 0 is the C's no error.
//! - `b_data`, `b_saveaddr` and `b_proc` are addresses (`usize`): what `bsd`'s drivers and
//!   the C's code compare and print. The kernel reaches a buffer's bytes only through its
//!   token's view; `b_data` records the view's address (exposed, for `bsd`'s legacy
//!   `unsafe fn data`, N7) or physio's window.
//! - `b_iodone` is an [`Iodone`]: a kernel callback given the token, or a `bsd` driver's given
//!   the buffer by address.
//! - `b_pobj` is [`Pages`]: none, the buffer's own `b_uobj`, or, during a cluster read, the
//!   first buffer's, held by a counted link (`bread_cluster`).
//! - `clrbuf` is [`BufRef::clear`]; the prototypes are `kernel::fs`'s functions.
//! - The disk queue (`struct bufq`) and the cache's queues (`struct bufcache`) are
//!   `kernel::fs::bufq`'s and `kernel::fs::bio`'s.
//!
//! ## Redesign
//! - LZ's `Buf`, every member a `Cell` changed through `&'static Buf` (`unsafe impl Sync`, "the
//!   kernel runs one CPU"), `b_data` a raw pointer behind `unsafe fn data()`, the links
//!   `queue_adapter!` entries -> [`Buf`]: the scalars atomics (the C's `volatile long`
//!   `b_flags` and the owner's fields, read and written Relaxed where the C reads them
//!   unlocked), the links `hal::owned` entries ([`ByBufhead`], [`ByLblkno`], [`ByVnbufs`],
//!   [`ByFreelist`], [`ByValist`], [`ByBufq`]), the busy state, the pages' object and the
//!   callback inside the buffer's own leaf lock (`Mutexed` at `IPL_BIO`); `Send` and `Sync`
//!   by construction.
//! - `B_BUSY`'s owner -> [`BufRef`], with the view of the bytes: `data()` is safe, one owner
//!   at a time by construction (the C's rule "touch a buffer only while you have it busy").
//! - `bufpool` (`pool_init(.., "bufpl")`) -> [`BUFPL`], a `hal::pool::BoxPool<Buf>` of the
//!   same name (`vmstat -m`'s `bufpl` row; its item size is this type's, larger than the C's
//!   by the token's slot and the owned links: the pinned test below).

use core::cmp::Ordering;
use core::mem;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicUsize, Ordering as Mem};

use hal::intr::IPL_BIO;
use hal::link::Link;
use hal::mapped::MappedBytes;
use hal::mutex::Mutexed;
use hal::owned::{OwnedEntry, SecondaryEntry, SecondaryRef};
use hal::pool::BoxPool;
use hal::sys::errno::Errno;
use hal::sys::param::PAGE_SIZE;
use hal::sys::types::{Daddr, Dev, Off};

use super::bio::VnodeBufs;
use super::bufq::Bufq;
use crate::assert::kpanic;
use crate::vm::km::BufSlot;
use crate::vm::object::{ObjKind, Object};

/// `MAXPHYS` (`<sys/param.h>`): the largest raw I/O transfer, and a kva slot of the buffer
/// arena.
pub const MAXPHYS: usize = 64 * 1024;

// Buffer queues

/// `BUFQ_NSCAN_N`: the segment the nscan discipline sorts at a time.
pub const BUFQ_NSCAN_N: i32 = 128;
/// `BUFQ_FIFO`.
pub const BUFQ_FIFO: i32 = 0;
/// `BUFQ_NSCAN`.
pub const BUFQ_NSCAN: i32 = 1;
/// `BUFQ_DEFAULT`.
pub const BUFQ_DEFAULT: i32 = BUFQ_NSCAN;
/// `BUFQ_HOWMANY`.
pub const BUFQ_HOWMANY: usize = 2;

// Write limits for bufq - defines high and low water marks for how many kva slots are allowed
// to be consumed to parallelize writes from the buffer cache from any individual bufq.

/// `BUFQ_HI`.
pub const BUFQ_HI: u32 = 128;
/// `BUFQ_LOW`.
pub const BUFQ_LOW: u32 = 64;

// These flags are kept in b_flags.

/// `B_WRITE`: write buffer (pseudo flag).
pub const B_WRITE: i64 = 0x00000000;
/// `B_AGE`: move to age queue when I/O done.
pub const B_AGE: i64 = 0x00000001;
/// `B_NEEDCOMMIT`: needs committing to stable storage.
pub const B_NEEDCOMMIT: i64 = 0x00000002;
/// `B_ASYNC`: start I/O, do not wait.
pub const B_ASYNC: i64 = 0x00000004;
/// `B_BAD`: bad block revectoring in progress.
pub const B_BAD: i64 = 0x00000008;
/// `B_BUSY`: I/O in progress.
pub const B_BUSY: i64 = 0x00000010;
/// `B_CACHE`: bread found us in the cache.
pub const B_CACHE: i64 = 0x00000020;
/// `B_CALL`: call `b_iodone` from biodone.
pub const B_CALL: i64 = 0x00000040;
/// `B_DELWRI`: delay I/O until buffer reused.
pub const B_DELWRI: i64 = 0x00000080;
/// `B_DONE`: I/O completed.
pub const B_DONE: i64 = 0x00000100;
/// `B_EINTR`: I/O was interrupted.
pub const B_EINTR: i64 = 0x00000200;
/// `B_ERROR`: I/O error occurred.
pub const B_ERROR: i64 = 0x00000400;
/// `B_INVAL`: does not contain valid info.
pub const B_INVAL: i64 = 0x00000800;
/// `B_NOCACHE`: do not cache block after use.
pub const B_NOCACHE: i64 = 0x00001000;
/// `B_PHYS`: I/O to user memory.
pub const B_PHYS: i64 = 0x00002000;
/// `B_RAW`: set by physio for raw transfers.
pub const B_RAW: i64 = 0x00004000;
/// `B_READ`: read buffer.
pub const B_READ: i64 = 0x00008000;
/// `B_WANTED`: process wants this buffer.
pub const B_WANTED: i64 = 0x00010000;
/// `B_WRITEINPROG`: write in progress.
pub const B_WRITEINPROG: i64 = 0x00020000;
/// `B_XXX`: debugging flag.
pub const B_XXX: i64 = 0x00040000;
/// `B_DEFERRED`: skipped over for cleaning.
pub const B_DEFERRED: i64 = 0x00080000;
/// `B_SCANNED`: block already pushed during sync.
pub const B_SCANNED: i64 = 0x00100000;
/// `B_PDAEMON`: I/O started by pagedaemon.
pub const B_PDAEMON: i64 = 0x00200000;
/// `B_RELEASED`: free this buffer after its kvm.
pub const B_RELEASED: i64 = 0x00400000;
/// `B_WARM`: buffer is or has been on the warm queue.
pub const B_WARM: i64 = 0x00800000;
/// `B_COLD`: buffer is on the cold queue.
pub const B_COLD: i64 = 0x01000000;
/// `B_BC`: buffer is managed by the cache.
pub const B_BC: i64 = 0x02000000;

/// `B_BITS`: the `%b` description of the flags.
pub const B_BITS: &[u8] = b"\x10\x01AGE\x02NEEDCOMMIT\x03ASYNC\x04BAD\x05BUSY\
\x06CACHE\x07CALL\x08DELWRI\x09DONE\x0aEINTR\x0bERROR\
\x0cINVAL\x0dNOCACHE\x0ePHYS\x0fRAW\x10READ\
\x11WANTED\x12WRITEINPROG\x13XXX(FORMAT)\x14DEFERRED\
\x15SCANNED\x16DAEMON\x17RELEASED\x18WARM\x19COLD\x1aBC";

// Flags to low-level allocation routines.

/// `B_CLRBUF`: request allocated buffer be cleared.
pub const B_CLRBUF: i32 = 0x01;
/// `B_SYNC`: do all allocations synchronously.
pub const B_SYNC: i32 = 0x02;

/// `RESERVE_SLOTS`: kva slots (of size `MAXPHYS`) reserved for syncer and cleaner.
pub const RESERVE_SLOTS: i64 = 4;
/// `RESERVE_PAGES`: buffer cache pages reserved for syncer and cleaner.
pub const RESERVE_PAGES: i64 = RESERVE_SLOTS * (MAXPHYS / PAGE_SIZE) as i64;
/// `BCACHE_MIN`: minimum size of the buffer cache, in pages.
pub const BCACHE_MIN: i64 = RESERVE_PAGES * 2;

/// `struct cluster_info`.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClusterInfo {
    /// `ci_lastr`: last read (read-ahead).
    pub ci_lastr: Daddr,
    /// `ci_lastw`: last write (write cluster).
    pub ci_lastw: Daddr,
    /// `ci_cstart`: start block of cluster.
    pub ci_cstart: Daddr,
    /// `ci_lasta`: last allocation.
    pub ci_lasta: Daddr,
    /// `ci_clen`: length of current cluster.
    pub ci_clen: i32,
    /// `ci_ralen`: read-ahead length.
    pub ci_ralen: i32,
    /// `ci_maxra`: last readahead block.
    pub ci_maxra: Daddr,
}

/// `b_iodone`: what `biodone` calls for a buffer with `B_CALL` (at `splbio`).
#[derive(Clone, Copy)]
pub enum Iodone {
    /// A kernel callback, given the buffer's token (`bread_cluster_callback`).
    Kernel(fn(BufRef)),
    /// A `bsd` driver's callback, given the buffer by address (`softraid`'s ccb completion):
    /// only for a buffer the driver embeds, whose token is a foreign one.
    Legacy(fn(&'static Buf)),
}

/// `b_pobj`: where the buffer's pages are.
pub enum Pages {
    /// `NULL`: the buffer has no pages.
    None,
    /// `&bp->b_uobj`: its own object.
    Own,
    /// The first buffer's `b_uobj` during a cluster read (`bread_cluster`), held by a counted
    /// link: that buffer cannot be freed while this one names its object.
    Master(SecondaryRef<ByBufhead>),
}

/// The kernel virtual space an idle buffer keeps (`b_data` of a buffer that is not busy).
pub enum Kva {
    /// None (`b_data == NULL`).
    None,
    /// The view of its pages, mapped at its slot (on `buf_valist`, for its next owner).
    Mapped(MappedBytes<'static>),
    /// The slot alone: its pages are gone (`B_RELEASED`, `buf_dealloc_mem`), the slot kept
    /// at the head of `buf_valist` until `buf_unmap` takes it and frees the buffer.
    Bare(BufSlot),
}

/// The buffer's busy state: who owns it now.
enum Slot {
    /// Not busy (`B_BUSY` clear), with the kernel virtual space it keeps.
    Idle(Kva),
    /// Busy, its token outside the buffer.
    Busy,
    /// Busy, its token parked here while a driver or a queue holds the buffer by address.
    Parked(BufRef),
}

/// What the buffer's leaf lock guards.
struct Owned {
    slot: Slot,
    pages: Pages,
    iodone: Option<Iodone>,
}

/// `struct buf`: the buffer header describes an I/O operation in the kernel.
///
/// Locks: the links and the 2Q flags are the cache's (`splbio`); `slot`, the pages and the
/// callback are the buffer's leaf lock's (never held across another lock); the other scalars
/// belong to the buffer's owner and are atomics read unlocked where the C reads them so.
pub struct Buf {
    /// `b_list`: all allocated buffers (`bufhead`), or physio's own list.
    b_list: OwnedEntry<Buf>,
    /// `b_rbbufs`: the vnode's tree by logical block.
    b_rbbufs: SecondaryEntry<Buf>,
    /// `b_vnbufs`: the vnode's clean or dirty list.
    b_vnbufs: SecondaryEntry<Buf>,
    /// `b_freelist`: the cache's hot, warm, cold or dirty queue.
    b_freelist: SecondaryEntry<Buf>,
    /// `b_valist`: the LRU of idle mapped buffers whose kva may be reused.
    b_valist: SecondaryEntry<Buf>,
    /// `b_bufq`: the disk queue's discipline (`union bufq_data`).
    b_bufq: SecondaryEntry<Buf>,
    /// `b_flags`: `B_*` flags (the C's `volatile long`).
    b_flags: AtomicI64,
    /// `b_bufsize`: allocated buffer size.
    b_bufsize: AtomicI64,
    /// `b_bcount`: valid bytes in buffer.
    b_bcount: AtomicI64,
    /// `b_resid`: remaining I/O.
    b_resid: AtomicUsize,
    /// `b_error`: errno value, 0 for none.
    b_error: AtomicI32,
    /// `b_dev`: device associated with buffer.
    b_dev: AtomicI32,
    /// `b_lblkno`: logical block number.
    b_lblkno: AtomicI64,
    /// `b_blkno`: underlying physical block number.
    b_blkno: AtomicI64,
    /// `b_dirtyoff`: offset in buffer of dirty region.
    b_dirtyoff: AtomicI32,
    /// `b_dirtyend`: offset of end of dirty region.
    b_dirtyend: AtomicI32,
    /// `b_validoff`: offset in buffer of valid region.
    b_validoff: AtomicI32,
    /// `b_validend`: offset of end of valid region.
    b_validend: AtomicI32,
    /// `b_data`: the address of the buffer's bytes (its view's, or physio's window), 0 for
    /// none.
    b_data: AtomicUsize,
    /// `b_saveaddr`: physio's user address while `B_PHYS`.
    b_saveaddr: AtomicUsize,
    /// `b_proc`: the thread the I/O is for (its address), 0 for the kernel.
    b_proc: AtomicUsize,
    /// `b_vp`: the vnode the buffer belongs to (its buffer half, embedded in the vnode, which
    /// lives for ever).
    b_vp: Link<VnodeBufs>,
    /// `b_bq`: the disk queue the buffer is on (a softc's, which lives for ever).
    b_bq: Link<Bufq>,
    /// `b_uobj`: object containing the pages (the buffer cache's dummy object).
    b_uobj: Object,
    /// `b_poffs`: offset within object.
    b_poffs: AtomicI64,
    /// The busy state, `b_pobj` and `b_iodone`: the buffer's leaf lock.
    owned: Mutexed<Owned>,
}

impl Buf {
    /// A zeroed, idle buffer, as `pool_get(&bufpool, PR_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            b_list: OwnedEntry::new(),
            b_rbbufs: SecondaryEntry::new(),
            b_vnbufs: SecondaryEntry::new(),
            b_freelist: SecondaryEntry::new(),
            b_valist: SecondaryEntry::new(),
            b_bufq: SecondaryEntry::new(),
            b_flags: AtomicI64::new(0),
            b_bufsize: AtomicI64::new(0),
            b_bcount: AtomicI64::new(0),
            b_resid: AtomicUsize::new(0),
            b_error: AtomicI32::new(0),
            b_dev: AtomicI32::new(0),
            b_lblkno: AtomicI64::new(0),
            b_blkno: AtomicI64::new(0),
            b_dirtyoff: AtomicI32::new(0),
            b_dirtyend: AtomicI32::new(0),
            b_validoff: AtomicI32::new(0),
            b_validend: AtomicI32::new(0),
            b_data: AtomicUsize::new(0),
            b_saveaddr: AtomicUsize::new(0),
            b_proc: AtomicUsize::new(0),
            b_vp: Link::new(),
            b_bq: Link::new(),
            b_uobj: Object::dummy(ObjKind::Bufcache, 0),
            b_poffs: AtomicI64::new(0),
            owned: Mutexed::new(
                IPL_BIO,
                Owned {
                    slot: Slot::Idle(Kva::None),
                    pages: Pages::None,
                    iodone: None,
                },
            ),
        }
    }

    /// `bp->b_flags`.
    #[inline]
    pub fn flags(&self) -> i64 {
        self.b_flags.load(Mem::Relaxed)
    }

    /// `bp->b_flags = f`.
    #[inline]
    pub fn set_flags(&self, f: i64) {
        self.b_flags.store(f, Mem::Relaxed);
    }

    /// `ISSET(bp->b_flags, f)`: any of the flags `f`.
    #[inline]
    pub fn isset(&self, f: i64) -> bool {
        self.flags() & f != 0
    }

    /// `SET(bp->b_flags, f)`.
    #[inline]
    pub fn set(&self, f: i64) {
        self.b_flags.fetch_or(f, Mem::Relaxed);
    }

    /// `CLR(bp->b_flags, f)`.
    #[inline]
    pub fn clr(&self, f: i64) {
        self.b_flags.fetch_and(!f, Mem::Relaxed);
    }

    /// `bp->b_bufsize`.
    #[inline]
    pub fn bufsize(&self) -> i64 {
        self.b_bufsize.load(Mem::Relaxed)
    }

    /// `bp->b_bufsize = n`.
    #[inline]
    pub fn set_bufsize(&self, n: i64) {
        self.b_bufsize.store(n, Mem::Relaxed);
    }

    /// `bp->b_bcount`.
    #[inline]
    pub fn bcount(&self) -> i64 {
        self.b_bcount.load(Mem::Relaxed)
    }

    /// `bp->b_bcount = n`.
    #[inline]
    pub fn set_bcount(&self, n: i64) {
        self.b_bcount.store(n, Mem::Relaxed);
    }

    /// `bp->b_resid`.
    #[inline]
    pub fn resid(&self) -> usize {
        self.b_resid.load(Mem::Relaxed)
    }

    /// `bp->b_resid = n`.
    #[inline]
    pub fn set_resid(&self, n: usize) {
        self.b_resid.store(n, Mem::Relaxed);
    }

    /// `bp->b_error`: `None` for 0.
    #[inline]
    pub fn error(&self) -> Option<Errno> {
        Errno::from_raw(self.b_error.load(Mem::Relaxed))
    }

    /// `bp->b_error = e` (`None`: 0).
    #[inline]
    pub fn set_error(&self, e: Option<Errno>) {
        self.b_error.store(e.map_or(0, |e| e as i32), Mem::Relaxed);
    }

    /// `bp->b_dev`.
    #[inline]
    pub fn dev(&self) -> Dev {
        self.b_dev.load(Mem::Relaxed)
    }

    /// `bp->b_dev = dev`.
    #[inline]
    pub fn set_dev(&self, dev: Dev) {
        self.b_dev.store(dev, Mem::Relaxed);
    }

    /// `bp->b_lblkno`.
    #[inline]
    pub fn lblkno(&self) -> Daddr {
        self.b_lblkno.load(Mem::Relaxed)
    }

    /// `bp->b_lblkno = bn`.
    #[inline]
    pub fn set_lblkno(&self, bn: Daddr) {
        self.b_lblkno.store(bn, Mem::Relaxed);
    }

    /// `bp->b_blkno`.
    #[inline]
    pub fn blkno(&self) -> Daddr {
        self.b_blkno.load(Mem::Relaxed)
    }

    /// `bp->b_blkno = bn`.
    #[inline]
    pub fn set_blkno(&self, bn: Daddr) {
        self.b_blkno.store(bn, Mem::Relaxed);
    }

    /// `bp->b_dirtyoff`.
    pub fn dirtyoff(&self) -> i32 {
        self.b_dirtyoff.load(Mem::Relaxed)
    }

    /// `bp->b_dirtyoff = n`.
    pub fn set_dirtyoff(&self, n: i32) {
        self.b_dirtyoff.store(n, Mem::Relaxed);
    }

    /// `bp->b_dirtyend`.
    pub fn dirtyend(&self) -> i32 {
        self.b_dirtyend.load(Mem::Relaxed)
    }

    /// `bp->b_dirtyend = n`.
    pub fn set_dirtyend(&self, n: i32) {
        self.b_dirtyend.store(n, Mem::Relaxed);
    }

    /// `bp->b_validoff`.
    pub fn validoff(&self) -> i32 {
        self.b_validoff.load(Mem::Relaxed)
    }

    /// `bp->b_validoff = n`.
    pub fn set_validoff(&self, n: i32) {
        self.b_validoff.store(n, Mem::Relaxed);
    }

    /// `bp->b_validend`.
    pub fn validend(&self) -> i32 {
        self.b_validend.load(Mem::Relaxed)
    }

    /// `bp->b_validend = n`.
    pub fn set_validend(&self, n: i32) {
        self.b_validend.store(n, Mem::Relaxed);
    }

    /// `bp->b_data` as an address, 0 when the buffer is not mapped.
    #[inline]
    pub fn data_addr(&self) -> usize {
        self.b_data.load(Mem::Relaxed)
    }

    /// `bp->b_data = addr` (physio's window, a driver's own memory: `bsd`'s legacy sites).
    #[inline]
    pub fn set_data_addr(&self, addr: usize) {
        self.b_data.store(addr, Mem::Relaxed);
    }

    /// `bp->b_saveaddr` as an address.
    pub fn saveaddr(&self) -> usize {
        self.b_saveaddr.load(Mem::Relaxed)
    }

    /// `bp->b_saveaddr = addr`.
    pub fn set_saveaddr(&self, addr: usize) {
        self.b_saveaddr.store(addr, Mem::Relaxed);
    }

    /// `bp->b_proc` as the thread's address, 0 for the kernel's I/O.
    pub fn proc_addr(&self) -> usize {
        self.b_proc.load(Mem::Relaxed)
    }

    /// `bp->b_proc = p` (its address).
    pub fn set_proc_addr(&self, p: usize) {
        self.b_proc.store(p, Mem::Relaxed);
    }

    /// `bp->b_poffs`.
    pub fn poffs(&self) -> Off {
        self.b_poffs.load(Mem::Relaxed)
    }

    /// `bp->b_poffs = off`.
    pub fn set_poffs(&self, off: Off) {
        self.b_poffs.store(off, Mem::Relaxed);
    }

    /// `&bp->b_uobj`: the buffer's own object.
    pub fn uobj(&self) -> &Object {
        &self.b_uobj
    }

    /// `bp->b_vp`: the buffer half of the vnode the buffer belongs to, `None` for none.
    #[inline]
    pub fn vnode_bufs(&self) -> Option<&'static VnodeBufs> {
        self.b_vp.get()
    }

    /// `bp->b_vp = vp` (`bgetvp`, `brelvp`).
    pub(crate) fn set_vnode_bufs(&self, vb: Option<&'static VnodeBufs>) {
        self.b_vp.set(vb);
    }

    /// `bp->b_bq`: the disk queue the buffer is on, `None` for none.
    #[inline]
    pub fn bq(&self) -> Option<&'static Bufq> {
        self.b_bq.get()
    }

    /// `bp->b_bq = bq` (`bufq_queue`, `bufq_done`).
    pub(crate) fn set_bq(&self, bq: Option<&'static Bufq>) {
        self.b_bq.set(bq);
    }

    /// Whether the buffer is on one of the cache's queues (`b_freelist.tqe_next != NOLIST`).
    pub fn b_freelist_linked(&self) -> bool {
        self.b_freelist.is_linked()
    }

    /// Whether the buffer is on its vnode's clean or dirty list (`b_vnbufs.le_next != NOLIST`).
    pub fn b_vnbufs_linked(&self) -> bool {
        self.b_vnbufs.is_linked()
    }

    /// Whether the buffer's pages are `master`'s (a read-ahead buffer of the cluster `master`
    /// reads, `bread_cluster`).
    pub(crate) fn is_cluster_of(&self, master: &Buf) -> bool {
        matches!(&self.owned.lock().pages, Pages::Master(r) if r.points_to(master))
    }

    /// `bp->b_pobj != NULL`: whether the buffer has pages.
    pub fn has_pages(&self) -> bool {
        !matches!(self.owned.lock().pages, Pages::None)
    }

    /// `bp->b_pobj = ..`: where the buffer's pages are; the old value is dropped (a master's
    /// link given back).
    pub fn set_pages(&self, p: Pages) {
        let old = mem::replace(&mut self.owned.lock().pages, p);
        drop(old);
    }

    /// Runs `f` on `bp->b_pobj` (`None` when the buffer has no pages) under the buffer's lock:
    /// its own `b_uobj` or, during a cluster read, the first buffer's. `f` may not take this
    /// buffer's lock again.
    pub fn with_pobj<R>(&self, f: impl FnOnce(Option<&Object>) -> R) -> R {
        let g = self.owned.lock();
        match &g.pages {
            Pages::None => f(None),
            Pages::Own => f(Some(&self.b_uobj)),
            Pages::Master(r) => match r.get() {
                Some(m) => f(Some(&m.b_uobj)),
                None => kpanic(format_args!("buf {:p}: a cluster's master is gone", self)),
            },
        }
    }

    /// Whether the pages are a cluster master's (`b_pobj != &bp->b_uobj`).
    pub fn pages_are_masters(&self) -> bool {
        matches!(self.owned.lock().pages, Pages::Master(_))
    }

    /// `bp->b_iodone`.
    pub fn iodone(&self) -> Option<Iodone> {
        self.owned.lock().iodone
    }

    /// `bp->b_iodone = f`.
    pub fn set_iodone(&self, f: Option<Iodone>) {
        self.owned.lock().iodone = f;
    }

    /// Whether the buffer is busy now (its token exists, parked or not): the ownership
    /// `B_BUSY` stands for.
    pub fn is_held(&self) -> bool {
        !matches!(self.owned.lock().slot, Slot::Idle(_))
    }

    /// Whether the buffer is idle with kernel virtual space kept (`b_data != NULL` of an idle
    /// buffer: on `buf_valist`).
    pub fn idle_mapped(&self) -> bool {
        matches!(
            self.owned.lock().slot,
            Slot::Idle(Kva::Mapped(_) | Kva::Bare(_))
        )
    }

    /// Takes away the kernel virtual space an idle buffer keeps (`buf_unmap`: its slot is
    /// stolen; `b_data = NULL`), for the caller to unmap and reuse. Panics on a busy buffer.
    pub(crate) fn take_idle_kva(&self) -> Kva {
        let mut g = self.owned.lock();
        match &mut g.slot {
            Slot::Idle(k) => {
                let k = mem::replace(k, Kva::None);
                self.b_data.store(0, Mem::Relaxed);
                k
            }
            _ => {
                drop(g);
                kpanic(format_args!("buf_unmap: buffer {:p} is busy", self))
            }
        }
    }

    /// Parks the busy buffer's token in the buffer, while a driver or a queue holds the buffer
    /// by address: [`Buf::take_parked`] gives it back. Panics when the buffer is not busy
    /// with its token outside (a token parked twice).
    pub fn park(t: BufRef) {
        let anchor = t.anchor();
        let bp = anchor.buf();
        let mut g = bp.owned.lock();
        if matches!(g.slot, Slot::Busy) {
            g.slot = Slot::Parked(t);
            return;
        }
        drop(g);
        kpanic(format_args!("Buf::park: buffer {:p} is not busy", bp))
    }

    /// The token parked in the buffer ([`Buf::park`]), which the caller now holds; `None`
    /// when none is parked.
    pub fn take_parked(&self) -> Option<BufRef> {
        let mut g = self.owned.lock();
        match mem::replace(&mut g.slot, Slot::Busy) {
            Slot::Parked(t) => Some(t),
            other => {
                g.slot = other;
                None
            }
        }
    }

    /// Whether a token is parked in the buffer.
    pub fn is_parked(&self) -> bool {
        matches!(self.owned.lock().slot, Slot::Parked(_))
    }
}

impl Default for Buf {
    fn default() -> Self {
        Self::new()
    }
}

/// What keeps a token's buffer alive.
enum Hold {
    /// A counted link to a pool item `bufhead` or physio owns (they refuse to free it while
    /// counted); an empty link once the token was consumed.
    Cached(SecondaryRef<ByBufhead>),
    /// A buffer a driver embeds: it lives for as long as the driver says (`'static`).
    Foreign(&'static Buf),
}

impl Hold {
    #[inline]
    fn buf(&self) -> &Buf {
        match self {
            Hold::Cached(r) => match r.get() {
                Some(b) => b,
                None => kpanic(format_args!("BufRef: a consumed token used")),
            },
            Hold::Foreign(b) => b,
        }
    }
}

/// The `B_BUSY` owner's token of a buffer: made once per busy period (the cache's acquire,
/// physio's buffer, [`BufRef::foreign`]) and consumed by the call that ends it (`brelse`,
/// `bwrite`, `biodone`, a queue that parks it). While the buffer is mapped it holds the view
/// of its bytes ([`BufRef::data`]).
pub struct BufRef {
    hold: Hold,
    view: Option<MappedBytes<'static>>,
}

impl BufRef {
    /// The buffer.
    #[inline]
    pub fn buf(&self) -> &Buf {
        self.hold.buf()
    }

    /// A second hold of the same buffer (another count of a pool item's), to reach it while
    /// this token moves (into the buffer's slot, into a queue).
    fn anchor(&self) -> Hold {
        match &self.hold {
            Hold::Cached(r) => {
                let mut a = SecondaryRef::new();
                a.set(r.get());
                Hold::Cached(a)
            }
            Hold::Foreign(b) => Hold::Foreign(b),
        }
    }

    /// A counted link to the buffer, for a caller that keeps it while the token goes
    /// elsewhere (`biowait`'s, a queue's); `None` for a buffer a driver embeds.
    pub fn counted(&self) -> Option<SecondaryRef<ByBufhead>> {
        match self.anchor() {
            Hold::Cached(r) => Some(r),
            Hold::Foreign(_) => None,
        }
    }

    /// The buffer a driver embeds, by address; `None` for a pool item.
    pub fn foreign_buf(&self) -> Option<&'static Buf> {
        match self.hold {
            Hold::Foreign(b) => Some(b),
            Hold::Cached(_) => None,
        }
    }

    /// The token of a buffer a driver embeds (`softraid`'s `ccb_buf`, `fd`'s format buffer,
    /// a test's): marks it busy. Panics when it is busy already (a second owner) or when an
    /// owning container owns it (a pool item is never a driver's).
    pub fn foreign(bp: &'static Buf) -> BufRef {
        if bp.b_list.is_linked() {
            kpanic(format_args!(
                "BufRef::foreign: buffer {:p} is the cache's",
                bp
            ));
        }
        let mut g = bp.owned.lock();
        if !matches!(g.slot, Slot::Idle(Kva::None)) {
            drop(g);
            kpanic(format_args!("BufRef::foreign: buffer {:p} is busy", bp));
        }
        g.slot = Slot::Busy;
        BufRef {
            hold: Hold::Foreign(bp),
            view: None,
        }
    }

    /// `buf_acquire`'s ownership half of the cache's or physio's buffer `r` points at: marks
    /// it busy and takes its idle view, if it kept one. Panics when it is busy already (the
    /// C's `KASSERT(!ISSET(bp->b_flags, B_BUSY))`) or when `r` points at nothing.
    pub fn acquire(r: SecondaryRef<ByBufhead>) -> BufRef {
        let Some(bp) = r.get() else {
            kpanic(format_args!("BufRef::acquire: no buffer"));
        };
        let mut g = bp.owned.lock();
        let view = match mem::replace(&mut g.slot, Slot::Busy) {
            Slot::Idle(Kva::None) => None,
            Slot::Idle(Kva::Mapped(v)) => Some(v),
            other => {
                g.slot = other;
                drop(g);
                kpanic(format_args!(
                    "buf_acquire: buffer {:p} is busy or released",
                    bp
                ))
            }
        };
        drop(g);
        BufRef {
            hold: Hold::Cached(r),
            view,
        }
    }

    /// `buf_release`'s ownership half: the buffer goes idle, keeping the view for its next
    /// owner; the token is consumed. Panics when the buffer's token is not this one's (not
    /// busy, or parked).
    pub fn release(mut self) {
        let view = self.view.take();
        {
            // The token's own hold keeps the buffer until `consumed` (no second count).
            let b = self.hold.buf();
            let mut g = b.owned.lock();
            if !matches!(g.slot, Slot::Busy) {
                drop(g);
                kpanic(format_args!("buf_release: buffer {:p} is not busy", b));
            }
            g.slot = Slot::Idle(view.map_or(Kva::None, Kva::Mapped));
        }
        self.consumed();
    }

    /// `buf_dealloc_mem`'s end of a busy period: the buffer goes idle keeping only the kva
    /// `slot` (its pages are gone: `B_RELEASED`, `b_data` the slot's address); the token is
    /// consumed. Panics on a mapped token (the view must be gone first) or a buffer that is
    /// not busy with it.
    pub(crate) fn release_bare(mut self, slot: BufSlot) {
        if self.view.is_some() {
            kpanic(format_args!(
                "buf_dealloc_mem: buffer {:p} is still mapped",
                self.buf()
            ));
        }
        {
            let b = self.hold.buf();
            let mut g = b.owned.lock();
            if !matches!(g.slot, Slot::Busy) {
                drop(g);
                kpanic(format_args!("buf_dealloc_mem: buffer {:p} is not busy", b));
            }
            b.b_data.store(slot.va().as_usize(), Mem::Relaxed);
            g.slot = Slot::Idle(Kva::Bare(slot));
        }
        self.consumed();
    }

    /// The token taken apart into the counted link it held (`buf_put`'s `bufhead.take`, a
    /// cluster's master link), the buffer marked idle with no view (the caller dropped the
    /// view or kept it); `None` for a foreign token, whose buffer is marked idle too.
    pub fn into_counted(mut self) -> Option<SecondaryRef<ByBufhead>> {
        if self.view.is_some() {
            kpanic(format_args!(
                "BufRef::into_counted: buffer {:p} is still mapped",
                self.buf()
            ));
        }
        {
            let b = self.buf();
            let mut g = b.owned.lock();
            if !matches!(g.slot, Slot::Busy) {
                drop(g);
                kpanic(format_args!(
                    "BufRef::into_counted: buffer {:p} is not busy",
                    b
                ));
            }
            g.slot = Slot::Idle(Kva::None);
        }
        match mem::replace(&mut self.hold, Hold::Cached(SecondaryRef::new())) {
            Hold::Cached(r) => Some(r),
            Hold::Foreign(_) => None,
        }
    }

    /// Whether the token was consumed (taken apart): its drop ends nothing.
    fn is_consumed(&self) -> bool {
        matches!(&self.hold, Hold::Cached(r) if !r.is_some())
    }

    /// Marks the token consumed: its hold an empty link, which ends nothing when dropped.
    fn consumed(&mut self) {
        let _ = mem::replace(&mut self.hold, Hold::Cached(SecondaryRef::new()));
    }

    /// Whether the buffer is mapped (`b_data != NULL` of a busy buffer).
    pub fn is_mapped(&self) -> bool {
        self.view.is_some()
    }

    /// Installs the view `buf_map` made (`bp->b_data = va`). Panics when the buffer is mapped
    /// already.
    pub fn set_view(&mut self, v: MappedBytes<'static>) {
        if self.view.is_some() {
            kpanic(format_args!("buf_map: buffer {:p} is mapped", self.buf()));
        }
        let addr = v.as_slice().as_ptr().expose_provenance();
        self.buf().b_data.store(addr, Mem::Relaxed);
        self.view = Some(v);
    }

    /// Takes the view away (`buf_dealloc_mem`, `buf_unmap` of a busy buffer: `b_data = NULL`),
    /// for the caller to drop (which unmaps it) or keep.
    pub fn take_view(&mut self) -> Option<MappedBytes<'static>> {
        let v = self.view.take();
        if v.is_some() {
            self.buf().b_data.store(0, Mem::Relaxed);
        }
        v
    }

    /// The view itself (`buf_fix_mapping`'s `truncate`).
    pub fn view_mut(&mut self) -> Option<&mut MappedBytes<'static>> {
        self.view.as_mut()
    }

    /// The buffer's bytes, `b_bcount` of them at `b_data` (the C's `bp->b_data`), writable:
    /// the owner's alone. Panics when the buffer is not mapped.
    pub fn data(&mut self) -> &mut [u8] {
        let n = usize::try_from(self.buf().bcount()).unwrap_or(0);
        match &mut self.view {
            Some(v) => {
                let s = v.as_slice_mut();
                let n = n.min(s.len());
                &mut s[..n]
            }
            None => kpanic(format_args!("BufRef::data: the buffer is not mapped")),
        }
    }

    /// The buffer's bytes, read-only.
    pub fn bytes(&self) -> &[u8] {
        let n = usize::try_from(self.buf().bcount()).unwrap_or(0);
        match &self.view {
            Some(v) => {
                let s = v.as_slice();
                &s[..n.min(s.len())]
            }
            None => kpanic(format_args!("BufRef::bytes: the buffer is not mapped")),
        }
    }

    /// `clrbuf(bp)`: zeroes the buffer's data area and its residual count.
    pub fn clear(&mut self) {
        self.data().fill(0);
        self.buf().set_resid(0);
    }
}

impl Drop for BufRef {
    /// A token dropped unconsumed ends its busy period as `brelse` does (a cache buffer's;
    /// the C's rule "every getblk is paired with a brelse", now the type's), or gives a
    /// driver's buffer back idle.
    fn drop(&mut self) {
        if self.is_consumed() {
            return;
        }
        let hold = mem::replace(&mut self.hold, Hold::Cached(SecondaryRef::new()));
        let view = self.view.take();
        let t = BufRef { hold, view };
        if t.foreign_buf().is_some() {
            t.release();
        } else {
            super::bio::brelse(t);
        }
    }
}

/// The address order of two buffers: what the lists' adapters compare (a list does not sort).
fn by_address(a: &Buf, b: &Buf) -> Ordering {
    ptr::from_ref(a).addr().cmp(&ptr::from_ref(b).addr())
}

/// `rb_buf_compare(b1, b2)`: a vnode's buffers ordered by logical block (`RBT_GENERATE(
/// buf_rb_bufs, ..)`).
pub fn rb_buf_compare(b1: &Buf, b2: &Buf) -> Ordering {
    b1.lblkno().cmp(&b2.lblkno())
}

hal::owned_adapter!(
    /// `LIST_ENTRY(buf) b_list`: the owning list (`bufhead`, physio's own).
    pub ByBufhead: Buf, b_list, by_address
);
hal::secondary_adapter!(
    /// `RBT_ENTRY(buf) b_rbbufs`: the vnode's tree by logical block.
    pub ByLblkno: ByBufhead, b_rbbufs, rb_buf_compare
);
hal::secondary_adapter!(
    /// `LIST_ENTRY(buf) b_vnbufs`: the vnode's clean and dirty lists.
    pub ByVnbufs: ByBufhead, b_vnbufs, by_address
);
hal::secondary_adapter!(
    /// `TAILQ_ENTRY(buf) b_freelist`: the cache's hot, warm, cold and dirty queues.
    pub ByFreelist: ByBufhead, b_freelist, by_address
);
hal::secondary_adapter!(
    /// `TAILQ_ENTRY(buf) b_valist`: `buf_valist`.
    pub ByValist: ByBufhead, b_valist, by_address
);
hal::secondary_adapter!(
    /// `union bufq_data b_bufq`: the disk queue's discipline.
    pub ByBufq: ByBufhead, b_bufq, by_address
);

/// `bufpool`: buffer pool for I/O buffers (`vmstat -m`'s `bufpl`), set up by `bufinit`.
pub static BUFPL: BoxPool<Buf> = BoxPool::new();

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Once;

    use hal::owned::OwnedList;
    use hal::pool::{PR_WAITOK, PoolBox};

    use super::*;

    /// `bufpl`, set up once per test process over the doubles.
    fn pool() -> &'static BoxPool<Buf> {
        static MADE: Once = Once::new();
        MADE.call_once(|| {
            crate::testutil::install();
            // bufinit's tests may have made it already.
            let _ = BUFPL.init(IPL_BIO, 0, "bufpl");
        });
        &BUFPL
    }

    fn fresh() -> PoolBox<Buf> {
        pool().get(PR_WAITOK, Buf::new).expect("a buffer")
    }

    /// A counted link to the head of `l`.
    fn head(l: &OwnedList<ByBufhead>) -> SecondaryRef<ByBufhead> {
        let mut r = SecondaryRef::new();
        r.set(l.first());
        r
    }

    #[test]
    fn the_flags_and_scalars_read_back() {
        let bp = Buf::new();
        bp.set(B_BUSY | B_READ);
        assert!(bp.isset(B_READ) && bp.isset(B_BUSY | B_ASYNC) && !bp.isset(B_ASYNC));
        bp.clr(B_BUSY);
        assert_eq!(bp.flags(), B_READ);
        bp.set_error(Some(Errno::EIO));
        assert_eq!(bp.error(), Some(Errno::EIO));
        bp.set_error(None);
        assert_eq!(bp.error(), None);
        bp.set_bcount(4096);
        bp.set_lblkno(-3);
        assert_eq!((bp.bcount(), bp.lblkno()), (4096, -3));
        assert!(!bp.has_pages() && bp.iodone().is_none() && !bp.is_held());
    }

    #[test]
    fn a_token_parks_and_is_taken_back_once() {
        let mut bufhead = OwnedList::<ByBufhead>::new();
        bufhead.push(fresh());
        let t = BufRef::acquire(head(&bufhead));
        let bp = bufhead.first().unwrap();
        assert!(bp.is_held() && !bp.is_parked());
        assert_eq!(bp.b_list.indexed(), 1);
        Buf::park(t);
        assert!(bp.is_parked());
        assert_eq!(
            bp.b_list.indexed(),
            1,
            "the parked token's count, the anchor's gone"
        );
        let t = bp.take_parked().expect("the parked token");
        assert!(bp.take_parked().is_none(), "taken once");
        assert!(bp.is_held() && !bp.is_parked());
        t.release();
        assert!(!bp.is_held());
        assert_eq!(bp.b_list.indexed(), 0);
        assert!(bufhead.pop().is_some());
    }

    #[test]
    #[should_panic(expected = "is busy")]
    fn a_busy_buffer_is_not_acquired_again() {
        let bufhead: &'static mut OwnedList<ByBufhead> = Box::leak(Box::new(OwnedList::new()));
        bufhead.push(fresh());
        let _t = BufRef::acquire(head(bufhead));
        let _u = BufRef::acquire(head(bufhead));
    }

    #[test]
    #[should_panic(expected = "still linked from a secondary tree")]
    fn bufhead_refuses_a_buffer_a_token_holds() {
        let bufhead: &'static mut OwnedList<ByBufhead> = Box::leak(Box::new(OwnedList::new()));
        bufhead.push(fresh());
        let t = BufRef::acquire(head(bufhead));
        // The token stays alive while the list is asked for the buffer.
        Box::leak(Box::new(t));
        let _ = bufhead.pop();
    }

    #[test]
    fn a_consumed_token_gives_the_buffer_back() {
        let mut bufhead = OwnedList::<ByBufhead>::new();
        bufhead.push(fresh());
        let t = BufRef::acquire(head(&bufhead));
        let r = t.into_counted().expect("a pool item's");
        let b = bufhead.take(r);
        assert!(!b.is_held());
    }

    #[test]
    #[should_panic(expected = "still linked from a secondary tree")]
    fn a_master_is_not_freed_while_a_cluster_buffer_names_its_pages() {
        let bufhead: &'static mut OwnedList<ByBufhead> = Box::leak(Box::new(OwnedList::new()));
        bufhead.push(fresh());
        bufhead.push(fresh());
        let bufhead: &'static mut OwnedList<ByBufhead> = bufhead;
        let master = bufhead.first().unwrap();
        let child = bufhead.last().unwrap();
        master.set_pages(Pages::Own);
        child.set_pages(Pages::Master(head(&bufhead)));
        assert!(child.pages_are_masters());
        let held = child.with_pobj(|o| o.is_some_and(|o| ptr::eq(o, master.uobj())));
        assert!(held);
        let _ = bufhead.pop();
    }

    #[test]
    fn a_child_gives_its_master_back_when_its_pages_are_its_own() {
        let mut bufhead = OwnedList::<ByBufhead>::new();
        bufhead.push(fresh());
        bufhead.push(fresh());
        let child = bufhead.last().unwrap();
        child.set_pages(Pages::Master(head(&bufhead)));
        child.set_pages(Pages::Own);
        assert!(child.with_pobj(|o| o.is_some_and(|o| ptr::eq(o, child.uobj()))));
        assert!(bufhead.pop().is_some());
    }

    #[test]
    fn a_foreign_buffer_is_held_once_and_goes_idle() {
        let bp: &'static Buf = Box::leak(Box::new(Buf::new()));
        let t = BufRef::foreign(bp);
        assert!(bp.is_held() && t.counted().is_none());
        assert!(t.foreign_buf().is_some_and(|b| ptr::eq(b, bp)));
        let again = catch_unwind(AssertUnwindSafe(|| drop(BufRef::foreign(bp))));
        assert!(again.is_err(), "a second owner");
        Buf::park(t);
        let t = bp.take_parked().expect("parked");
        assert!(t.into_counted().is_none());
        assert!(!bp.is_held());
    }

    #[test]
    #[should_panic(expected = "is the cache's")]
    fn a_pool_item_is_not_foreign() {
        let bufhead: &'static mut OwnedList<ByBufhead> = Box::leak(Box::new(OwnedList::new()));
        bufhead.push(fresh());
        let bufhead: &'static OwnedList<ByBufhead> = bufhead;
        let _ = BufRef::foreign(bufhead.first().unwrap());
    }

    #[test]
    #[should_panic(expected = "is not mapped")]
    fn an_unmapped_buffer_has_no_bytes() {
        let bp: &'static Buf = Box::leak(Box::new(Buf::new()));
        let mut t = BufRef::foreign(bp);
        let _ = t.data();
    }

    #[test]
    fn the_callback_and_the_pages_follow_the_owner() {
        fn done(_: &'static Buf) {}
        let bp = Buf::new();
        bp.set_iodone(Some(Iodone::Legacy(done)));
        assert!(matches!(bp.iodone(), Some(Iodone::Legacy(_))));
        bp.set_iodone(None);
        assert!(bp.iodone().is_none());
        bp.set_pages(Pages::Own);
        assert!(bp.has_pages() && !bp.pages_are_masters());
    }

    #[test]
    fn bufpl_items_are_the_measured_size() {
        // vmstat -m's `bufpl` item size (D-N5.1-2): LZ's `struct buf` is 288 bytes (the host
        // and both machines, 64-bit alike); the native one carries the owned links (an
        // `OwnedEntry` and five `SecondaryEntry`s: 248 bytes where LZ's six links took about
        // 100) and the buffer's leaf lock with the token's slot (the view of an idle mapped
        // buffer, its bare kva slot, or the parked token), `b_pobj` and `b_iodone`: 504
        // bytes at N5.1's commit 2, 520 with `b_vp` and `b_bq` (a word each).
        assert_eq!(pool().item_size(), Some(size_of::<Buf>()));
        assert_eq!(size_of::<Buf>(), 520);
    }
}
/* </TESTS> */
