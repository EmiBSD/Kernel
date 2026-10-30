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
 *  Portions of this code are:
 *
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
 */
/* </LICENSES> */

/* <CODE> */
//! The syncer daemon (`update`): a wheel of `SYNCER_MAXDELAY` worklists of vnodes with dirty
//! buffers, one turned per second, each vnode's `VOP_FSYNC(MNT_LAZY)` run when its slot comes
//! up; and the syncer vnode each writable mount gets, whose lazy `fsync` syncs the mount.
//!
//! Upstream: sys/kern/vfs_sync.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_sync.rs@3c62ede68802
//!
//! ## Deviations
//! - The C's `speedup_syncer` does not exist at this version (nothing to port; LZ's note).
//!
//! ## Redesign
//! - LZ's wheel, a `hashinit` table of `ListHead`s behind an `AtomicPtr` and a mask, reached
//!   through an `unsafe` slot accessor, and `LIST_REMOVE` of a vnode from whichever slot holds
//!   it (vfs_sync.rs:80-100, 140) -> [`Wheel`], a fixed array of `SYNCER_MAXDELAY` lists (the
//!   C's `hashinit(32)` gives 32 heads) inside the subsystem's mutex at `IPL_BIO` (the C's
//!   `splbio`), each vnode recording its slot so its removal names its list.
//! - The syncer's loop leaves the wheel's lock around `vget` and `VOP_FSYNC` (the C holds
//!   `splbio` across `vget(LK_NOWAIT)`; the vnode's operations do not run under a guard
//!   here) and finds the slot's first vnode again after each, as the C's `LIST_FIRST` does.
//! - `syncerproc` is the buffer cache's syncer word (`kernel::fs::bio::set_syncer`, N5.1);
//!   `sync_vops` is [`SyncVops`], a kernel table with a fixed id.

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::list::{List, ListEntry};
use hal::sys::errno::Errno;
use hal::sys::mount::{MNT_ASYNC, MNT_LAZY, VB_NOWAIT, VB_READ};

use super::mount::{Mount, vfs_busy, vfs_unbusy};
use super::subr::{getnewvnode_id, vfs, vget, vput};
use super::vnode::{LK_EXCLUSIVE, LK_NOWAIT, VBIOONSYNCLIST, VNON, VT_VFS, Vnode, VopsId};
use super::vops::{Cred, VFS_SYNC, Vnops, vop_fsync, vop_unlock};
use crate::assert::kpanic;
use crate::proc::{ProcGlue, Thread};

/// `SYNCER_MAXDELAY`: maximum sync delay time.
pub const SYNCER_MAXDELAY: i32 = 32;
/// `SYNCER_DEFAULT`: default sync delay time.
pub const SYNCER_DEFAULT: i32 = 30;
/// `PPAUSE`: the priority of a pause (`<sys/param.h>`).
const PPAUSE: i32 = 40;
/// A second, in nanoseconds.
const SEC: u64 = 1_000_000_000;

/// `syncdelay`: time to delay syncing vnodes.
pub static SYNCDELAY: AtomicI32 = AtomicI32::new(SYNCER_DEFAULT);
/// `syncer_chan`: what the syncer sleeps on between rounds.
pub static SYNCER_CHAN: AtomicI32 = AtomicI32::new(0);

/// `v_synclist`'s adapter: a slot of the wheel.
pub struct BySynclist<G: ProcGlue>(PhantomData<G>);

impl<G: ProcGlue> hal::list::ListAdapter for BySynclist<G> {
    type Elem = Vnode<G>;
    fn entry(e: &Vnode<G>) -> &ListEntry<Vnode<G>> {
        &e.synclist
    }
}

/// The workitem queue (`syncer_workitem_pending`, `syncer_delayno`, `syncer_maxdelay`).
///
/// It is useful to delay writes of file data and filesystem metadata for tens of seconds so
/// that quickly created and deleted files need not waste disk bandwidth being created and
/// removed. To realize this, we append vnodes to a "workitem" queue. When running with a soft
/// updates implementation, most pending metadata dependencies should not wait for more than a
/// few seconds. Thus, mounted block devices are delayed only about half the time that file
/// data is delayed. Similarly, directory updates are more critical, so are only delayed about
/// a third the time that file data is delayed. Thus, there are `SYNCER_MAXDELAY` queues that
/// are processed round-robin at a rate of one each second (driven off the filesystem syncer
/// process). The `syncer_delayno` variable indicates the next queue that is to be processed.
/// Items that need to be processed soon are placed in this queue:
///
/// `syncer_workitem_pending[syncer_delayno]`
///
/// A delay of fifteen seconds is done by placing the request fifteen entries later in the
/// queue:
///
/// `syncer_workitem_pending[(syncer_delayno + 15) & syncer_mask]`
pub struct Wheel<G: ProcGlue> {
    /// `syncer_workitem_pending`.
    slots: [List<BySynclist<G>>; SYNCER_MAXDELAY as usize],
    /// `syncer_delayno`: the next slot to process.
    delayno: i32,
    /// `syncer_maxdelay`: the slots in use (`hashinit`'s size).
    maxdelay: i32,
    /// `vfs_allocate_syncvnode`'s `start`, `incr`, `next`.
    start: i64,
    incr: i64,
    next: i64,
}

impl<G: ProcGlue> Wheel<G> {
    /// An empty wheel.
    pub const fn new() -> Self {
        Self {
            slots: [const { List::new() }; SYNCER_MAXDELAY as usize],
            delayno: 0,
            maxdelay: SYNCER_MAXDELAY,
            start: 0,
            incr: 0,
            next: 0,
        }
    }

    /// `syncer_mask`.
    fn mask(&self) -> usize {
        self.maxdelay as usize - 1
    }
}

impl<G: ProcGlue> Default for Wheel<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `vn_initialize_syncerd()`: the wheel's size (`hashinit(syncer_maxdelay)`: a power of two,
/// 32), the slots empty.
pub fn vn_initialize_syncerd<G: ProcGlue>() {
    let mut w = vfs::<G>().syncer.lock();
    w.maxdelay = SYNCER_MAXDELAY;
}

/// Forgets the wheel of a test thread's tables (`vfs_test_reset`).
#[cfg(any(test, feature = "doubles"))]
pub(super) fn syncer_test_reset<G: ProcGlue>() {
    let mut w = vfs::<G>().syncer.lock();
    for slot in w.slots.iter() {
        while slot.pop_head().is_some() {}
    }
    w.delayno = 0;
    w.start = 0;
    w.incr = 0;
    w.next = 0;
}

/// Add an item to the syncer work queue.
pub fn vn_syncer_add_to_worklist<G: ProcGlue>(vp: &'static Vnode<G>, delay: i32) {
    let mut w = vfs::<G>().syncer.lock();
    add_locked(&mut w, vp, delay);
}

/// [`vn_syncer_add_to_worklist`] under the wheel's lock.
fn add_locked<G: ProcGlue>(w: &mut Wheel<G>, vp: &'static Vnode<G>, delay: i32) {
    let delay = delay.min(w.maxdelay - 2);
    let slot = (w.delayno + delay) as usize & w.mask();

    if vp.bufs.bioflag() & VBIOONSYNCLIST != 0 {
        w.slots[usize::from(vp.syncslot.load(Ordering::Relaxed))].remove(vp);
    }

    vp.bufs.set_bioflag(VBIOONSYNCLIST);
    vp.syncslot.store(slot as u8, Ordering::Relaxed);
    w.slots[slot].insert_head(vp);
}

/// `LIST_REMOVE(vp, v_synclist)` of a vnode marked `VBIOONSYNCLIST` (the buffer cache's
/// `reassignbuf`, which clears the flag).
pub fn syncer_remove<G: ProcGlue>(vp: &'static Vnode<G>) {
    let w = vfs::<G>().syncer.lock();
    w.slots[usize::from(vp.syncslot.load(Ordering::Relaxed))].remove(vp);
}

/// System filesystem synchronizer daemon (the `update` thread's body).
pub fn syncer_thread<G: ProcGlue>() -> ! {
    // `syncerproc`, as the buffer cache knows it (its kva and page reserves).
    super::bio::set_syncer(hal::sleep::curproc_word());
    let t = vfs::<G>();

    loop {
        let start = crate::tc::getnsecuptime();

        // Push files whose dirty time has expired.
        let slot = {
            let mut w = t.syncer.lock();
            let slot = w.delayno as usize;
            w.delayno += 1;
            if w.delayno == w.maxdelay {
                w.delayno = 0;
            }
            slot
        };

        loop {
            let Some(vp) = t.syncer.lock().slots[slot].first() else {
                break;
            };
            if vget(vp, LK_EXCLUSIVE | LK_NOWAIT).is_err() {
                // If we fail to get the lock, we move this vnode one second ahead in time.
                // XXX - no good, but the best we can do.
                vn_syncer_add_to_worklist(vp, 1);
                continue;
            }
            G::with_curproc(|p| {
                if let Some(p) = p {
                    let _ = vop_fsync(vp, Cred::real(p.ucred()), MNT_LAZY, p);
                }
            });
            vput(vp);
            let mut w = t.syncer.lock();
            if w.slots[slot].first().is_some_and(|f| ptr::eq(f, vp)) {
                // Note: disk vps can remain on the worklist too with no dirty blocks, but
                // since sync_fsync() moves it to a different slot we are safe.
                #[cfg(feature = "diagnostic")]
                if vp.bufs.dirty_is_empty() && vp.vtype() != super::vnode::VBLK {
                    drop(w);
                    super::subr::vprint(Some("fsync failed"), vp);
                    if let Some(mp) = vp.mount() {
                        let (name, len) = mp.mntonname();
                        crate::prf::printf(format_args!(
                            "mounted on: {}\n",
                            core::str::from_utf8(&name[..len]).unwrap_or("?")
                        ));
                    }
                    kpanic(format_args!("syncer_thread: fsync failed"));
                }
                // Put us back on the worklist. The worklist routine will remove us from our
                // current position and then add us back in at a later position.
                add_locked(&mut w, vp, SYNCDELAY.load(Ordering::Relaxed));
            }
            drop(w);

            hal::sleep::sched_pause();
        }

        // If it has taken us less than a second to process the current work, then wait.
        // Otherwise start right over again. We can still lose time if any single round takes
        // more than two seconds, but it does not really matter as we are just trying to
        // generally pace the filesystem activity.
        let elapsed = crate::tc::getnsecuptime().saturating_sub(start);
        if elapsed < SEC {
            let _ = hal::sleep::tsleep_nsec(
                ptr::from_ref(&SYNCER_CHAN),
                PPAUSE,
                "syncer",
                SEC - elapsed,
            );
        }
    }
}

/// `sync_vops`: routines to create and manage a filesystem syncer vnode.
pub struct SyncVops<G: ProcGlue>(PhantomData<G>);

impl<G: ProcGlue> SyncVops<G> {
    /// The table.
    pub const TABLE: SyncVops<G> = SyncVops(PhantomData);
}

impl<G: ProcGlue> Vnops<G> for SyncVops<G> {
    fn close(
        &self,
        _vp: &'static Vnode<G>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        cred: Cred<'_>,
        waitfor: i32,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        sync_fsync(vp, cred, waitfor, p)
    }
    fn inactive(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        sync_inactive(vp)
    }
    fn reclaim(&self, _vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn lock(&self, _vp: &'static Vnode<G>, _flags: i32) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn unlock(&self, _vp: &'static Vnode<G>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn islocked(&self, _vp: &'static Vnode<G>) -> i32 {
        0 // nullop
    }
    fn print(&self, _vp: &'static Vnode<G>) -> Result<(), Errno> {
        sync_print()
    }
    fn remove(
        &self,
        _dvp: &'static Vnode<G>,
        _vp: &'static Vnode<G>,
        _cnp: &mut super::vops::Cn<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP) // eopnotsupp
    }
}

/// Create a new filesystem syncer vnode for the specified mount point.
pub fn vfs_allocate_syncvnode<G: ProcGlue>(mp: &'static Mount<G>) -> Result<(), Errno> {
    // Allocate a new vnode
    let vp = match getnewvnode_id(VT_VFS, Some(mp), VopsId::SYNC) {
        Ok(vp) => vp,
        Err(error) => {
            mp.set_syncer(None);
            return Err(error);
        }
    };
    vp.set_writecount(1);
    vp.set_vtype(VNON);
    // Place the vnode onto the syncer worklist. We attempt to scatter them about on the list
    // so that they will go off at evenly distributed times even if all the filesystems are
    // mounted at once.
    {
        let mut w = vfs::<G>().syncer.lock();
        let maxdelay = i64::from(w.maxdelay);
        w.next += w.incr;
        if w.next == 0 || w.next > maxdelay {
            w.start /= 2;
            w.incr /= 2;
            if w.start == 0 {
                w.start = maxdelay / 2;
                w.incr = maxdelay;
            }
            w.next = w.start;
        }
        let next = w.next as i32;
        add_locked(&mut w, vp, next);
    }
    mp.set_syncer(Some(vp));
    Ok(())
}

/// Do a lazy sync of the filesystem.
pub fn sync_fsync<G: ProcGlue>(
    syncvp: &'static Vnode<G>,
    cred: Cred<'_>,
    waitfor: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let Some(mp) = syncvp.mount() else {
        kpanic(format_args!("sync_fsync: syncer vnode without a mount"));
    };

    // We only need to do something if this is a lazy evaluation.
    if waitfor != MNT_LAZY {
        return Ok(());
    }

    // Move ourselves to the back of the sync list.
    vn_syncer_add_to_worklist(syncvp, SYNCDELAY.load(Ordering::Relaxed));

    // Walk the list of vnodes pushing all that are dirty and not already on the sync list.
    if vfs_busy(mp, VB_READ | VB_NOWAIT).is_ok() {
        let asyncflag = mp.flag() & MNT_ASYNC;
        mp.flag_clr(MNT_ASYNC);
        let _ = VFS_SYNC(mp, MNT_LAZY, 0, cred, p);
        if asyncflag != 0 {
            mp.flag_set(MNT_ASYNC);
        }
        vfs_unbusy(mp);
    }

    Ok(())
}

/// The syncer vnode is no longer needed and is being decommissioned.
pub fn sync_inactive<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if vp.usecount() == 0 {
        let _ = vop_unlock(vp);
        return Ok(());
    }

    if let Some(mp) = vp.mount() {
        mp.set_syncer(None);
    }

    syncer_remove(vp);
    vp.bufs.clr_bioflag(VBIOONSYNCLIST);

    vp.set_writecount(0);
    vput(vp);

    Ok(())
}

/// Print out a syncer vnode.
pub fn sync_print() -> Result<(), Errno> {
    if cfg!(any(feature = "debug", feature = "diagnostic")) {
        crate::prf::printf(format_args!("syncer vnode\n"));
    }
    Ok(())
}

// A slot's number fits the vnode's byte.
const _: () = assert!(SYNCER_MAXDELAY as u32 <= u8::MAX as u32 + 1);
/* </CODE> */
