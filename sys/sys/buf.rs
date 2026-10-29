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
//! buf`), its `B_*` flags, the disk queues (`struct bufq`) and the interface of the buffer
//! cache (`vfs_bio.c`, `vfs_biomem.c`, `kern_bufq.c`, `kern_physio.c`), as `bsd` names them.
//!
//! Upstream: sys/sys/buf.h @ 3ce1f3f79392
//!
//! Since N5.1 the header, its owner's token, the cache, the disk queues and physio are the
//! kernel crate's (`kernel::fs::{buf, bio, biomem, bufq, physio}`); this module is their face
//! for the `bsd` modules that still name a busy buffer by address (`&'static Buf`, the C's
//! `struct buf *`): the drivers and the file systems, until N7 and N5.2 give them tokens.
//! The legacy protocol: a busy buffer `bsd` code holds by address has its token
//! (`kernel::fs::buf::BufRef`) parked in it. Every function here that hands a busy buffer out
//! ([`bread`], [`getblk`], [`bufq_dequeue`], a strategy routine's argument, ...) parks the
//! token and lends the buffer ([`lend`]); every one that takes a buffer back ([`brelse`],
//! [`bwrite`], [`biodone`], [`bufq_queue`], ...) takes the token out ([`token_of`]): the
//! parked one, else (a driver's own buffer) a new one. The lent `&'static Buf` is valid until
//! the token is consumed (the C's rule: a `struct buf *` is the `B_BUSY` owner's until
//! `brelse` or `biodone`). The busiest of them keep the token parked through the kernel's own
//! lock sections instead of a take and a park ([`brelse`], [`biodone`], [`bufq_queue`],
//! [`bufq_dequeue`], [`bread`], [`bread_cluster`], [`getblk`]: the kernel's `*_parked`
//! functions). The C's rule is the one `unsafe` this boundary keeps ([`legacy_ref`]),
//! with LZ's `unsafe fn data()` (the drivers' and file systems' view of `b_data`), both paid
//! at N7 (D-N5.1-1, the user's decision of 2026-10-28: option (a)).
//!
//! ## Deviations
//! - `b_error` is `Option<Errno>`: `None` is the C's 0. `b_iodone` is an `Iodone` (a
//!   driver's `fn(&'static Buf)` is `Iodone::Legacy`). `b_flags` is an `i64` (the C's
//!   `volatile long`), the `B_*` flags `i64` constants.
//! - `incore` answers whether the block is cached, not with the buffer (its callers only
//!   test it); `clrbuf` is safe (the token's view clears the bytes).
//! - `bufinit` takes the machine's `physmem` from `sys/systm.rs`.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::owned::{OwnedList, SecondaryRef};
use hal::pool::PR_WAITOK;
use kernel::fs::bio;

use crate::kern::subr_prf::panic;
use crate::machine::cpu::curproc;
use crate::sys::errno::Errno;
use crate::sys::param::DEV_BSIZE;
use crate::sys::proc::Proc;
use crate::sys::systm::PHYSMEM;
use crate::sys::types::{Daddr, Dev, Off};
use crate::sys::uio::Uio;
use crate::sys::vnode::Vnode;

pub use kernel::fs::bio::{
    BCSTATS, BUFHIGHPAGES, BUFKVM, BUFLOWPAGES, Bcstats, bufadjust, unclean_pages,
};
pub use kernel::fs::buf::{
    B_AGE, B_ASYNC, B_BAD, B_BC, B_BITS, B_BUSY, B_CACHE, B_CALL, B_CLRBUF, B_COLD, B_DEFERRED,
    B_DELWRI, B_DONE, B_EINTR, B_ERROR, B_INVAL, B_NEEDCOMMIT, B_NOCACHE, B_PDAEMON, B_PHYS, B_RAW,
    B_READ, B_RELEASED, B_SCANNED, B_SYNC, B_WANTED, B_WARM, B_WRITE, B_WRITEINPROG, B_XXX,
    BCACHE_MIN, BUFPL, BUFQ_DEFAULT, BUFQ_FIFO, BUFQ_HI, BUFQ_HOWMANY, BUFQ_LOW, BUFQ_NSCAN,
    BUFQ_NSCAN_N, Buf, BufRef, ByBufhead, ByFreelist, ClusterInfo, Iodone, RESERVE_PAGES,
    RESERVE_SLOTS,
};
pub use kernel::fs::bufq::{
    Bufq, bufq_destroy, bufq_done, bufq_drain, bufq_init, bufq_peek, bufq_quiesce, bufq_restart,
};
pub use kernel::fs::physio::minphys;

/// The members `bsd`'s drivers and file systems reach as the C's pointers (`b_data`,
/// `b_saveaddr`, `b_proc`), as `bsd`'s vnode (`b_vp`) or as a driver's callback (`b_iodone`),
/// over the kernel's buffer header, whose words they are.
pub trait BufLegacy {
    /// `bp->b_data`: the address of the buffer's bytes, NULL when it is not mapped.
    fn data_ptr(&self) -> *mut u8;
    /// `bp->b_data = p` (a driver's or physio's own memory).
    fn set_data_ptr(&self, p: *mut u8);
    /// `bp->b_saveaddr`.
    fn saveaddr_ptr(&self) -> *mut c_void;
    /// `bp->b_saveaddr = p`.
    fn set_saveaddr_ptr(&self, p: *mut c_void);
    /// `bp->b_proc`: the thread the I/O is for, NULL for the kernel.
    fn proc_ptr(&self) -> *const Proc;
    /// `bp->b_proc = p`.
    fn set_proc_ptr(&self, p: *const Proc);
    /// `bp->b_vp`: the vnode the buffer belongs to.
    fn vp(&self) -> Option<&'static Vnode>;
    /// `bp->b_vp = vp` of a buffer the cache keeps on no vnode list (a driver's own:
    /// `softraid`'s), without a hold.
    fn set_vp(&self, vp: Option<&'static Vnode>);
    /// `bp->b_iodone = f` (a driver's callback, `B_CALL`).
    fn set_iodone_fn(&self, f: Option<fn(&'static Buf)>);
    /// The buffer's data: `b_bcount` bytes at `b_data`.
    ///
    /// # Safety
    ///
    /// The caller owns the buffer (it is `B_BUSY` for the caller, as [`bread`], [`getblk`] and
    /// [`geteblk`] return it, its token parked in it), the buffer is mapped (`b_data` is not
    /// NULL) and no other slice of it is alive.
    #[allow(clippy::mut_from_ref)] // the B_BUSY owner's view, as the C's b_data
    unsafe fn data(&self) -> &mut [u8];
}

impl BufLegacy for Buf {
    fn data_ptr(&self) -> *mut u8 {
        ptr::with_exposed_provenance_mut(self.data_addr())
    }

    fn set_data_ptr(&self, p: *mut u8) {
        self.set_data_addr(p.expose_provenance());
    }

    fn saveaddr_ptr(&self) -> *mut c_void {
        ptr::with_exposed_provenance_mut(self.saveaddr())
    }

    fn set_saveaddr_ptr(&self, p: *mut c_void) {
        self.set_saveaddr(p.expose_provenance());
    }

    fn proc_ptr(&self) -> *const Proc {
        ptr::with_exposed_provenance(self.proc_addr())
    }

    fn set_proc_ptr(&self, p: *const Proc) {
        self.set_proc_addr(p.expose_provenance());
    }

    fn vp(&self) -> Option<&'static Vnode> {
        let vb = self.vnode_bufs()?;
        let any = vb.owner().as_any();
        match any.downcast_ref::<Vnode>() {
            Some(vp) => Some(vp),
            None => panic(format_args!("buf {:p}: b_vp is not a vnode", self)),
        }
    }

    fn set_vp(&self, vp: Option<&'static Vnode>) {
        Buf::set_vp_unheld(self, vp.map(|vp| &vp.v_bufs));
    }

    fn set_iodone_fn(&self, f: Option<fn(&'static Buf)>) {
        self.set_iodone(f.map(Iodone::Legacy));
    }

    unsafe fn data(&self) -> &mut [u8] {
        let len = usize::try_from(self.bcount()).unwrap_or(0);
        // SAFETY: this is the legacy hole N7 closes (D-N5.1-1), not a proof. What holds: for
        // a cache buffer, `b_data` is the address of the view its token holds
        // (`BufRef::set_view` exposed it), `b_bufsize >= b_bcount` bytes the parked token
        // keeps mapped until it is consumed, and the caller, the `B_BUSY` owner by the C's
        // rule, takes no other slice of it meanwhile. What does not: for any other buffer
        // `b_data` is whatever its owner set (`set_data_ptr`: a driver's DMA buffer, or
        // physio's window or bounce), and physio's window is user memory shared with the
        // process by design (the C's semantics), which may write it while a driver holds
        // this `&mut [u8]` (`vnd.rs`'s copy through it, `svnd`): an aliasing Rust cannot
        // rule out here; the C has the same race. Until N7 gives the drivers the token's view
        // (and physio's window no slice at all), the callers keep the C's protocol.
        unsafe { core::slice::from_raw_parts_mut(self.data_ptr(), len) }
    }
}

/// The busy buffer `bp` as the C's `struct buf *`: valid until its token is consumed
/// (`brelse`, `bwrite`, `biodone`), the `B_BUSY` owner's rule `bsd`'s callers keep
/// (D-N5.1-1). The caller holds the token or it is parked in `bp`. Private: [`lend`] (the
/// token in hand, parked next), [`parked_ref`] and [`bufq_dequeue`] (the token parked, a
/// counted link or the queue's in hand) are its only users.
fn legacy_ref(bp: &Buf) -> &'static Buf {
    let p: *const Buf = bp;
    // SAFETY: `bp` is busy and its token is the caller's or parked in it (each caller's own
    // state, above). Either `bp` is a `bufpl` item an owning list keeps (`bufhead`, or
    // physio's or `with_private_buf`'s own) and that token holds a counted link to it: the
    // owner refuses to free a counted element (`hal::owned`'s I4) and a pool item never
    // moves, so the item stays at this address, initialised, while the token lives, parked
    // or held. Or no list owns it, and then it is a buffer a driver embeds, which is
    // `'static` already: only `BufRef::foreign` makes a token of a buffer no list owns and
    // only `Member::of_static` queues one, and both take a `&'static Buf`. The reference is
    // shared, and every member is an atomic or a lock (`Buf` is `Sync`). Its `'static`
    // outlives the token: the bound is the legacy protocol's, the C's rule that a `struct buf
    // *` is used by the `B_BUSY` owner only, until it gives the buffer back (`brelse`,
    // `biodone`), which every caller in `bsd` keeps as LZ's and the C's code did; a use after
    // that is the C's use-after-release, which this boundary does not make impossible
    // (D-N5.1-1, option (a), paid at N7 when the drivers hold tokens).
    unsafe { &*p }
}

/// Lends the buffer of `t` to legacy code: the token is parked in the buffer and the buffer
/// given as `&'static Buf` ([`legacy_ref`]; a driver's own buffer is `'static` already).
pub fn lend(t: BufRef) -> &'static Buf {
    let bp = match t.foreign_buf() {
        Some(bp) => bp,
        None => legacy_ref(t.buf()),
    };
    Buf::park(t);
    bp
}

/// The token of the busy buffer legacy code gives back: the one parked in it, else (a
/// driver's own buffer, idle) a new one, which makes it busy.
pub fn token_of(bp: &'static Buf) -> BufRef {
    match bp.take_parked() {
        Some(t) => t,
        None => BufRef::foreign(bp),
    }
}

/// A buffer of the caller's own for the length of `f` (`softraid`'s `sr_rw`, the C's `struct
/// buf b` on the stack, as physio's): a `bufpl` item on a list of its own (never `bufhead`),
/// lent busy to `f` with its token parked, freed when `f` returns (its token must be parked
/// in it again: `biowait` leaves it so).
pub fn with_private_buf<R>(f: impl FnOnce(&'static Buf) -> R) -> R {
    let mut own: OwnedList<ByBufhead> = OwnedList::new();
    let Some(b) = BUFPL.get(PR_WAITOK, Buf::new) else {
        panic(format_args!("with_private_buf: pool_get"));
    };
    own.push(b);
    let mut r = SecondaryRef::new();
    r.set(own.last());
    // The token first: the cache's guard must be gone before lend parks it (which takes the
    // lock again).
    let t = BufRef::acquire(&mut bio::CACHE.lock(), r);
    let bp = lend(t);
    let rv = f(bp);
    let Some(t) = bp.take_parked() else {
        panic(format_args!(
            "with_private_buf: buffer {:p} not given back",
            bp
        ));
    };
    if t.buf().vnode_bufs().is_some() {
        t.buf().set_vp_unheld(None);
    }
    let Some(r) = t.into_counted(&mut bio::CACHE.lock()) else {
        panic(format_args!("with_private_buf: a driver's buffer"));
    };
    drop(own.take(r));
    rv
}

/// `bufinit()`: the cache over the machine's memory (`physmem`).
pub fn bufinit() {
    bio::bufinit(PHYSMEM.load(Ordering::Relaxed));
}

/// The buffer a counted link names, its token parked in it ([`legacy_ref`]): what the
/// kernel's `*_parked` reads give.
fn parked_ref(r: &SecondaryRef<ByBufhead>) -> &'static Buf {
    match r.get() {
        Some(bp) => legacy_ref(bp),
        None => panic(format_args!("a parked buffer's link names nothing")),
    }
}

/// `bread(vp, blkno, size, &bp)`: reads a disk block; the busy buffer and the read's result
/// (its token parked by the kernel's own section: [`lend`]'s fast path).
pub fn bread(vp: &'static Vnode, blkno: Daddr, size: i32) -> (&'static Buf, Result<(), Errno>) {
    let (r, rv) = bio::bread_parked(&vp.v_bufs, blkno, size);
    (parked_ref(&r), rv)
}

/// `breadn(vp, blkno, size, rablks, rasizes, nrablks, &bp)`: `bread` with read-ahead of the
/// blocks `rablks` (sizes `rasizes`).
pub fn breadn(
    vp: &'static Vnode,
    blkno: Daddr,
    size: i32,
    rablks: &[Daddr],
    rasizes: &[i32],
) -> (&'static Buf, Result<(), Errno>) {
    let (t, rv) = bio::breadn(&vp.v_bufs, blkno, size, rablks, rasizes);
    (lend(t), rv)
}

/// `bread_cluster(vp, blkno, size, &bp)`: `bread` with the following blocks read ahead in
/// one I/O.
pub fn bread_cluster(
    vp: &'static Vnode,
    blkno: Daddr,
    size: i32,
) -> (&'static Buf, Result<(), Errno>) {
    let (r, rv) = bio::bread_cluster_parked(&vp.v_bufs, blkno, size);
    (parked_ref(&r), rv)
}

/// `bwrite(bp)`: writes the busy buffer; a synchronous write waits and releases it.
pub fn bwrite(bp: &'static Buf) -> Result<(), Errno> {
    bio::bwrite(token_of(bp))
}

/// `bdwrite(bp)`: marks the busy buffer a delayed write and releases it.
pub fn bdwrite(bp: &'static Buf) {
    bio::bdwrite(token_of(bp));
}

/// `bawrite(bp)`: starts an asynchronous write of the busy buffer.
pub fn bawrite(bp: &'static Buf) {
    bio::bawrite(token_of(bp));
}

/// `buf_dirty(bp)`: marks the busy buffer dirty (`B_DELWRI`).
pub fn buf_dirty(bp: &'static Buf) {
    let t = token_of(bp);
    bio::buf_dirty(&t);
    Buf::park(t);
}

/// `buf_undirty(bp)`: marks the busy buffer clean.
pub fn buf_undirty(bp: &'static Buf) {
    let t = token_of(bp);
    bio::buf_undirty(&t);
    Buf::park(t);
}

/// `brelse(bp)`: releases the busy buffer to the cache (its parked token taken back in the
/// cache's own section, [`token_of`]'s fast path).
pub fn brelse(bp: &'static Buf) {
    if !bio::brelse_parked(bp) {
        bio::brelse(BufRef::foreign(bp));
    }
}

/// `incore(vp, blkno)`: whether the block's buffer is in the cache and valid.
pub fn incore(vp: &'static Vnode, blkno: Daddr) -> bool {
    bio::incore(&vp.v_bufs, blkno)
}

/// `getblk(vp, blkno, size, slpflag, slptimeo)`: the busy buffer of the block, cached or
/// new; `None` when a sleep for a busy one was interrupted.
pub fn getblk(
    vp: &'static Vnode,
    blkno: Daddr,
    size: i32,
    slpflag: i32,
    slptimeo: u64,
) -> Option<&'static Buf> {
    bio::getblk_parked(&vp.v_bufs, blkno, size, slpflag, slptimeo).map(|r| parked_ref(&r))
}

/// `geteblk(size)`: an empty, disassociated busy buffer.
pub fn geteblk(size: usize) -> &'static Buf {
    lend(bio::geteblk(size))
}

/// `buf_daemon(arg)`: the cleaner kthread's body.
pub fn buf_daemon(_arg: *mut c_void) {
    bio::buf_daemon()
}

/// `biowait(bp)`: waits for the I/O of the busy buffer (its token parked in it by the
/// driver's `biodone`) and gives its error value.
pub fn biowait(bp: &'static Buf) -> Result<(), Errno> {
    bio::biowait_buf(bp)
}

/// `biodone(bp)`: the I/O of the busy buffer is over (a driver's interrupt; its parked token
/// taken back in the cache's own section, [`token_of`]'s fast path).
pub fn biodone(bp: &'static Buf) {
    if !bio::biodone_parked(bp) {
        bio::biodone(BufRef::foreign(bp));
    }
}

/// `buf_adjcnt(bp, ncount)`: shortens the valid part of the busy buffer.
pub fn buf_adjcnt(bp: &'static Buf, ncount: i64) {
    let t = token_of(bp);
    bio::buf_adjcnt(&t, ncount);
    Buf::park(t);
}

/// `clrbuf(bp)`: zeroes the busy buffer's data and its residual count.
pub fn clrbuf(bp: &'static Buf) {
    let mut t = token_of(bp);
    t.clear();
    Buf::park(t);
}

/// `bufq_queue(bq, bp)`: queues the busy buffer on the disk queue (a strategy routine); a
/// cache buffer's parked token stays parked ([`token_of`]'s fast path), a driver's own
/// buffer's token is queued.
pub fn bufq_queue(bq: &'static Bufq, bp: &'static Buf) {
    if !kernel::fs::bufq::bufq_queue_parked(bq, bp) {
        kernel::fs::bufq::bufq_queue(bq, token_of(bp));
    }
}

/// `bufq_dequeue(bq)`: the next buffer of the disk queue, busy, its token parked (left so by
/// the queue: [`lend`]'s fast path).
pub fn bufq_dequeue(bq: &Bufq) -> Option<&'static Buf> {
    kernel::fs::bufq::bufq_dequeue_parked(bq).map(|m| legacy_ref(m.get()))
}

/// `physio(strategy, dev, flags, minphys, uio)`: raw I/O between the device and `curproc`'s
/// buffers, through the driver's strategy routine.
pub fn physio(
    strategy: fn(&'static Buf),
    dev: Dev,
    flags: i64,
    minphys: fn(&Buf),
    uio: &mut Uio<'_>,
) -> Result<(), Errno> {
    if uio.uio_offset % DEV_BSIZE as Off != 0 {
        return Err(Errno::EINVAL);
    }
    let Some(p) = curproc() else {
        panic(format_args!("physio: no curproc"));
    };
    kernel::fs::physio::physio(&mut |t| strategy(lend(t)), dev, flags, minphys, uio, p)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;

    use super::*;

    /// A disk's softc is zeroed by autoconf (`Softc`): an all-zero `Bufq` is the unused
    /// queue `bufq_init` fills in, as the drivers' `Softc` claims say.
    #[test]
    fn a_zeroed_bufq_is_an_unused_queue() {
        // SAFETY: `Bufq` is atomics, a `Mutexed` (an atomic owner word, an integer level, the
        // state of integers and two empty `SecondaryList`s: null links, a zero identity, a
        // false poison flag) and a `hal::list` entry of null links: every member is valid
        // all zero, the claim the test checks by using it.
        let bq: &'static Bufq = Box::leak(Box::new(unsafe { core::mem::zeroed::<Bufq>() }));
        assert!(!bufq_peek(bq));
        assert_eq!(bq.outstanding(), 0);
        bufq_init(bq, BUFQ_FIFO).expect("bufq_init");
        assert_eq!(bq.kind(), BUFQ_FIFO);
        assert!(!bufq_peek(bq));
        bufq_destroy(bq);
    }

    /// A driver's own buffer goes through the legacy protocol: made busy by its first
    /// `token_of`, parked by `lend`, taken back and idle again.
    #[test]
    fn a_drivers_buffer_is_lent_and_taken_back() {
        let bp: &'static Buf = Box::leak(Box::new(Buf::new()));
        assert!(!bp.is_held());
        let t = token_of(bp);
        assert!(bp.is_held() && !bp.is_parked());
        let again = lend(t);
        assert!(core::ptr::eq(again, bp));
        assert!(bp.is_parked());
        let t = token_of(bp);
        assert!(!bp.is_parked());
        t.release_unlocked();
        assert!(!bp.is_held());
    }

    /// `with_private_buf` (softraid's `sr_rw`): a `bufpl` item of its own, lent busy with its
    /// token parked, the token taken and parked again by the caller's I/O (here by hand),
    /// and the item freed when the closure returns.
    #[test]
    fn a_private_buffer_is_lent_parked_and_freed() {
        let (_g, _p) = crate::kern::vfs_subr::tests::setup();
        kernel::fs::bio::bio_test_reset();
        kernel::fs::bufq::bufq_test_reset();
        bufinit();
        let before = BUFPL.nout();
        let rv = with_private_buf(|bp| {
            assert!(bp.is_parked() && bp.is_held() && bp.is_owned());
            assert_eq!(BUFPL.nout(), before + 1);
            let t = token_of(bp);
            assert!(!bp.is_parked() && bp.is_held());
            lend(t);
            7
        });
        assert_eq!(rv, 7);
        assert_eq!(BUFPL.nout(), before);
    }

    /// The legacy pointers are the header's words.
    #[test]
    fn the_legacy_pointers_are_the_words() {
        let bp = Buf::new();
        let mut bytes = [0u8; 8];
        bp.set_data_ptr(bytes.as_mut_ptr());
        assert_eq!(bp.data_ptr(), bytes.as_mut_ptr());
        assert_eq!(bp.data_addr(), bytes.as_ptr() as usize);
        bp.set_saveaddr_ptr(core::ptr::without_provenance_mut(0x1000));
        assert_eq!(bp.saveaddr(), 0x1000);
        assert!(bp.proc_ptr().is_null());
        assert!(bp.vp().is_none());
    }
}
/* </TESTS> */
