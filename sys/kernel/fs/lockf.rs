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
 *
 * This code is derived from software contributed to Berkeley by
 * Scooter Morris at Genentech Inc.
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
 *	@(#)ufs_lockf.c	8.3 (Berkeley) 1/6/94
 */

/*
 * Copyright (c) 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Scooter Morris at Genentech Inc.
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
 *	@(#)lockf.h	8.1 (Berkeley) 6/11/93
 */
/* </LICENSES> */

/* <CODE> */
//! Advisory record locking (`fcntl(2)`'s `F_SETLK`/`F_GETLK` and `flock(2)`): the byte-range
//! locks of one file, kept sorted by start in a `struct lockf_state` the file's vnode (or
//! inode) points at, with the blocked requests queued on the lock that blocks them; and
//! `<sys/lockf.h>`, its interface.
//!
//! Upstream: sys/kern/vfs_lockf.c @ 3ce1f3f79392
//! Upstream: sys/sys/lockf.h @ 3ce1f3f79392
//! LZ: sys/kern/vfs_lockf.rs@3c62ede68802
//! LZ: sys/sys/lockf.rs@3c62ede68802
//!
//! ## Deviations
//! - `lf_findoverlap` returns the case and the overlap as a pair instead of filling
//!   `**overlap`; `lf_getblock` returns `Option`, `lf_deadlock` a `bool` (LZ's).
//! - `LOCKF_DEBUG` is not configured: `lf_print`, `lf_printlist` and the traces are compiled
//!   out, as in C.
//! - Inherited from the C: a request asleep in `lf_setlock` keeps the slot it came from (the
//!   C's `ls_owner`, here the `&LockfSlot` borrowed from the vnode or the inode) across the
//!   sleep; a forced unmount that reclaims and frees the inode meanwhile leaves it dangling,
//!   as the C's `ls_owner` does. Here the slot's memory is `bsd`'s inode (its `unsafe`); the
//!   reclaim must purge or own the slot when the inodes move (N5.x).
//!
//! ## Redesign
//! - LZ's `&'static Lockf`/`&'static LockfState` pool items of `Cell`s under `unsafe impl
//!   Sync`, freed by `pool_put` while still referenced, `ls_owner` a raw pointer back to the
//!   slot (29 `unsafe`) -> owned items: a state is the [`PoolBox`] its slot holds
//!   ([`LockfSlot`], the C's `struct lockf_state **` in the vnode or the inode: `v_speclockf`,
//!   `i_lockf`, ...), so `ls_rele`'s `*ls->ls_owner = NULL; pool_put` is the slot emptied;
//!   a lock is owned by the list it is on (`ls_locks` or `ls_pending`, one entry, `OwnedList`)
//!   or by its maker until it is linked, its blocker (`lf_blk`) a counted link and the queue
//!   of the locks it blocks (`lf_blkhd`) a list of counted links, so no lock can be freed
//!   while a waiter names it or it waits.
//! - `lockf_lock` guards everything, as in the C: the slot's state is part 0 of its write
//!   witness, every lock's changing members (type, range, flags, `lf_blk`, `lf_blkhd`) part
//!   1; nothing else opens them. The C's walks of `ls_locks` that change the list keep their
//!   pointers as counted links (an element lent by the list cannot cross the list's change):
//!   O(1) a step, as the C.
//! - The order of two changes inside the one write section, where the C's needs an element
//!   to be linked before another link names it: `lf_setlock`'s case 3 links the new lock
//!   before it takes the overlap's waiters (the C takes them first); after a sleep the lock
//!   leaves its blocker's queue before `ls_pending` (the C the other way round).
//! - `ls_rele` frees the state when the last reference goes at the end of the call that
//!   dropped it (`lf_advlock`, `lf_purgelocks`), where the C frees it at once; nothing reads
//!   the state between the two points, which the lock covers.
//! - The lock owner (`lf_id`, a `caddr_t` compared and never followed) is
//!   [`LockOwner`] (D-N5.3-11).

use core::cmp::Ordering;
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering as Mem};

use hal::owned::{Member, OwnedEntry, OwnedList, SecondaryEntry, SecondaryList, SecondaryRef};
use hal::pool::{BoxPool, PR_RWLOCK, PR_WAITOK, PoolBox};
use hal::rwlock::{Divisible, Guarded, RawRwlock, WitnessPart, WriteWitness};
use hal::sleep::{INFSLP, PCATCH, PLOCK, wakeup_one};
use hal::sys::errno::Errno;
use hal::sys::fcntl::{
    F_FLOCK, F_GETLK, F_INTR, F_POSIX, F_RDLCK, F_SETLK, F_UNLCK, F_WAIT, F_WRLCK, Flock,
};
use hal::sys::types::{Off, Pid, Uid};
use hal::sys::unistd::{SEEK_CUR, SEEK_END, SEEK_SET};

use crate::assert::{kassert, kpanic};
use crate::file::LockOwner;
use crate::proc::{ProcGlue, with_uidinfo};

/// `SELF`: `lf_findoverlap` looks at the locks of the same owner.
const SELF: i32 = 0x1;
/// `OTHERS`: `lf_findoverlap` looks at the locks of other owners.
const OTHERS: i32 = 0x2;

/// `lockf_lock`: serializes access to each instance of `struct lockf` and `struct
/// lockf_state` and each pointer from a vnode to `struct lockf_state`.
static LOCKF_LOCK: RawRwlock = RawRwlock::new("lockflk");

/// The part of `lockf_lock`'s write witness that opens the locks' members.
const LOCK_PART: u8 = 1;

/// `lockf_state_pool` (`"lockfspl"`).
///
/// The two pools need no `*_test_reset` (testing.md), as the process-wide pools of
/// `kernel::proc` (`ucredpl`, `zombiepl`): in the host tests they are the heap doubles
/// (`testutil::memory_is_the_doubles`, which the tests assert), so no item comes from the
/// memory `bsd`'s `setup_real_memory` replaces; and nothing global names an item past its
/// test (a state lives in its vnode's slot, a lock on its state's lists, both the test's).
static LOCKF_STATE_POOL: BoxPool<LockfState> = BoxPool::new();
/// `lockf_pool` (`"lockfpl"`), as [`LOCKF_STATE_POOL`].
static LOCKF_POOL: BoxPool<Lockf> = BoxPool::new();

/// `maxlocksperuid`: we enforce a limit on locks by uid, so that a single user cannot run the
/// kernel out of memory. For now, the limit is pretty coarse. There is no limit on root.
///
/// Splitting a lock will always succeed, regardless of current allocations. If you're
/// slightly above the limit, we still have to permit an allocation so that the unlock can
/// succeed. If the unlocking causes too many splits, however, you're totally cutoff.
/// (`kern.maxlocksperuid`.)
pub static MAXLOCKSPERUID: AtomicI32 = AtomicI32::new(1024);

/// The members of a lock that change while it is linked, under `lockf_lock` (part 1).
struct LockfMut {
    /// `lf_flags`: lock semantics: `F_POSIX`, `F_FLOCK`, `F_WAIT`, `F_INTR`.
    flags: i32,
    /// `lf_type`: lock type: `F_RDLCK`, `F_WRLCK`.
    ty: i16,
    /// `lf_start`: the byte # of the start of the lock.
    start: Off,
    /// `lf_end`: the byte # of the end of the lock (-1=EOF).
    end: Off,
    /// `lf_id`: the id of the resource holding the lock.
    id: LockOwner,
    /// `lf_blk`: the lock that blocks us (on `ls_locks`), counted.
    blk: SecondaryRef<ByEntry>,
    /// `lf_blkhd`: the list of blocked locks (on `ls_pending`), counted.
    blkhd: SecondaryList<ByBlock>,
}

/// `struct lockf`: a kernel structure which contains the information associated with a byte
/// range lock. Locks are sorted by the starting byte of the lock for efficiency.
pub struct Lockf {
    /// `lf_entry`: the link in `ls_locks` or `ls_pending`, whichever owns it.
    entry: OwnedEntry<Lockf>,
    /// `lf_block`: a request waiting for a lock (on its blocker's `lf_blkhd`).
    block: SecondaryEntry<Lockf>,
    /// The changing members.
    m: Guarded<LockfMut>,
    /// `lf_uid`: user ID responsible.
    uid: Uid,
    /// `lf_pid`: POSIX - owner pid (set once, before the lock is linked).
    pid: Pid,
}

hal::owned_adapter!(
    /// `TAILQ_HEAD(locklist, lockf)` through `lf_entry`: `ls_locks` and `ls_pending`.
    pub ByEntry: Lockf, entry, |_, _| Ordering::Equal
);

hal::secondary_adapter!(
    /// `TAILQ_HEAD(locklist, lockf)` through `lf_block`: a lock's `lf_blkhd`.
    pub ByBlock: ByEntry, block, |_, _| Ordering::Equal
);

/// `struct lockf_state`: the locks of one file.
pub struct LockfState {
    /// `ls_locks`: list of active locks.
    locks: OwnedList<ByEntry>,
    /// `ls_pending`: list of pending locks.
    pending: OwnedList<ByEntry>,
    /// `ls_refs`: reference counter.
    refs: i32,
}

impl LockfState {
    /// A zeroed state, as `pool_get(PR_ZERO)` returns it.
    const fn new() -> Self {
        Self {
            locks: OwnedList::new(),
            pending: OwnedList::new(),
            refs: 0,
        }
    }
}

/// `struct lockf_state *`: the slot a vnode (`v_speclockf`) or an inode (`i_lockf`, ...)
/// keeps, which owns the state; under `lockf_lock`.
pub struct LockfSlot(Guarded<Option<PoolBox<LockfState>>>);

impl LockfSlot {
    /// An empty slot (`NULL`).
    pub const fn new() -> Self {
        Self(Guarded::new_static(&LOCKF_LOCK, None))
    }

    /// Whether the slot holds a state (`*state != NULL`), read under `lockf_lock`.
    pub fn is_empty(&self) -> bool {
        let mut w = LOCKF_LOCK.write();
        let [mut p0] = w.split::<1>();
        self.0.get_mut(&mut p0).is_none()
    }
}

impl Default for LockfSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// What a request compares (`lock`'s members read once: they change only through the
/// request's own steps).
#[derive(Clone, Copy, Debug)]
struct Range {
    ty: i16,
    start: Off,
    end: Off,
    id: LockOwner,
    flags: i32,
    pid: Pid,
}

impl Range {
    /// `lock`'s members, through the locks' part of the witness.
    fn of(lock: &Lockf, p1: &WitnessPart<'_>) -> Self {
        let m = lock.m.get(p1);
        Self {
            ty: m.ty,
            start: m.start,
            end: m.end,
            id: m.id,
            flags: m.flags,
            pid: lock.pid,
        }
    }
}

/// `lf_init`: initialise the pools.
pub fn lf_init() {
    // A second call (a test's) keeps the first pools.
    let _ = LOCKF_STATE_POOL.init(hal::intr::IPL_NONE, PR_WAITOK | PR_RWLOCK, "lockfspl");
    let _ = LOCKF_POOL.init(hal::intr::IPL_NONE, PR_WAITOK | PR_RWLOCK, "lockfpl");
}

/// The slot's state, under part 0 of the witness: `panic(9)` when there is none (every
/// caller referenced it first).
fn state<'w>(slot: &'w LockfSlot, p0: &'w mut WitnessPart<'_>) -> &'w mut LockfState {
    match slot.0.get_mut(p0).as_deref_mut() {
        Some(ls) => ls,
        None => kpanic(format_args!("lockf: a referenced state is gone")),
    }
}

/// `ls_rele`'s end: the state freed and the slot emptied (`*ls->ls_owner = NULL`) when the
/// last reference went.
fn ls_settle(slot: &LockfSlot, p0: &mut WitnessPart<'_>) {
    let st = slot.0.get_mut(p0);
    if st.as_ref().is_some_and(|ls| ls.refs <= 0) {
        if let Some(ls) = st.as_ref() {
            kassert!(ls.locks.is_empty());
            kassert!(ls.pending.is_empty());
        }
        drop(st.take());
    }
}

/// A counted link to `e`, so the lock can be named while its list changes.
fn hold(e: &Lockf) -> SecondaryRef<ByEntry> {
    let mut h = SecondaryRef::new();
    h.set(Some(e));
    h
}

/// The element a hold names.
fn held(h: &SecondaryRef<ByEntry>) -> &Lockf {
    match h.get() {
        Some(e) => e,
        None => kpanic(format_args!("lockf: an empty hold")),
    }
}

/// `lf_alloc`: a lock charged to `uid`. 3 options for allowfail: 0 - always allocate. 1 -
/// cutoff at limit. 2 - cutoff at double limit.
fn lf_alloc<G: ProcGlue>(
    uid: Uid,
    allowfail: i32,
    m: LockfMut,
    pid: Pid,
) -> Option<PoolBox<Lockf>> {
    let max = i64::from(MAXLOCKSPERUID.load(Mem::Relaxed));
    let charged = with_uidinfo::<G, _>(uid, |uip| {
        let limit = if allowfail == 1 { max } else { max * 2 };
        if uid != 0 && allowfail != 0 && uip.ui_lockcnt.get() > limit {
            return false;
        }
        uip.ui_lockcnt.set(uip.ui_lockcnt.get() + 1);
        true
    });
    if !charged {
        return None;
    }
    let lock = LOCKF_POOL.get(PR_WAITOK, || Lockf {
        entry: OwnedEntry::new(),
        block: SecondaryEntry::new(),
        m: Guarded::keyed(LOCKF_LOCK.id(), LOCK_PART, m),
        uid,
        pid,
    });
    if lock.is_none() {
        kpanic(format_args!("lf_alloc: lf_init not run"));
    }
    lock
}

/// `lf_free`: uncharges and frees a lock, dropping its reference on the state.
fn lf_free<G: ProcGlue>(ls: &mut LockfState, mut lock: PoolBox<Lockf>) {
    kassert!(lock.m.get_mut_owned().blkhd.is_empty());

    ls.refs -= 1;

    with_uidinfo::<G, _>(lock.uid, |uip| {
        uip.ui_lockcnt.set(uip.ui_lockcnt.get() - 1);
    });
    drop(lock);
}

/// Do an advisory lock operation (`op`: `F_SETLK`, `F_UNLCK`, `F_GETLK`) of the owner `id`
/// on the locks of `slot`, a file of `size` bytes; `flags` are `F_POSIX` or `F_FLOCK`, with
/// `F_WAIT` for a waiting `F_SETLK`. The current thread's credentials and process are the
/// lock's (`curproc`).
pub fn lf_advlock<G: ProcGlue>(
    slot: &LockfSlot,
    size: Off,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    let Some((uid, pid)) =
        G::with_curproc(|p| p.map(|p| (p.ucred().cr_uid.get(), p.process().ps_pid)))
    else {
        kpanic(format_args!("lf_advlock: no curproc"));
    };

    // Convert the flock structure into a start and end.
    let mut start = match i32::from(fl.l_whence) {
        // Caller is responsible for adding any necessary offset when SEEK_CUR is used.
        SEEK_SET | SEEK_CUR => fl.l_start,
        SEEK_END => size.wrapping_add(fl.l_start),
        _ => return Err(Errno::EINVAL),
    };
    if start < 0 {
        return Err(Errno::EINVAL);
    }
    let end = match fl.l_len.cmp(&0) {
        Ordering::Greater => {
            if fl.l_len - 1 > Off::MAX - start {
                return Err(Errno::EOVERFLOW);
            }
            let end = start + (fl.l_len - 1);
            // Avoid ambiguity at the end of the range.
            if end == Off::MAX { -1 } else { end }
        }
        Ordering::Less => {
            if start.wrapping_add(fl.l_len) < 0 {
                return Err(Errno::EINVAL);
            }
            let end = start - 1;
            start += fl.l_len;
            end
        }
        Ordering::Equal => -1,
    };

    let mut w = LOCKF_LOCK.write();
    let lock = {
        let [mut p0, _p1] = w.split::<2>();
        let st = slot.0.get_mut(&mut p0);
        // Avoid the common case of unlocking when inode has no locks.
        if st.is_none() && op != F_SETLK {
            fl.l_type = F_UNLCK;
            return Ok(());
        }
        if st.is_none() {
            let Some(ls) = LOCKF_STATE_POOL.get(PR_WAITOK, LockfState::new) else {
                kpanic(format_args!("lf_advlock: lf_init not run"));
            };
            *st = Some(ls);
        }
        state(slot, &mut p0).refs += 1;

        let allowfail = if op == F_SETLK { 1 } else { 2 };
        let m = LockfMut {
            flags,
            ty: fl.l_type,
            start,
            end,
            id,
            blk: SecondaryRef::new(),
            blkhd: SecondaryList::new(),
        };
        let pid = if flags & F_POSIX != 0 { pid } else { -1 };
        let Some(lock) = lf_alloc::<G>(uid, allowfail, m, pid) else {
            state(slot, &mut p0).refs -= 1;
            ls_settle(slot, &mut p0);
            return Err(Errno::ENOLCK);
        };
        lock
    };

    let error = match op {
        F_SETLK => lf_setlock::<G>(slot, &mut w, lock),
        _ if op == i32::from(F_UNLCK) => {
            let [mut p0, mut p1] = w.split::<2>();
            let ls = state(slot, &mut p0);
            let rq = Range::of(&lock, &p1);
            lf_clearlock::<G>(ls, &rq, &mut p1);
            lf_free::<G>(ls, lock);
            Ok(())
        }
        F_GETLK => {
            let [mut p0, p1] = w.split::<2>();
            let ls = state(slot, &mut p0);
            let rq = Range::of(&lock, &p1);
            lf_getlock(ls, &rq, fl, &p1);
            lf_free::<G>(ls, lock);
            Ok(())
        }
        _ => {
            let [mut p0, _p1] = w.split::<2>();
            lf_free::<G>(state(slot, &mut p0), lock);
            Err(Errno::EINVAL)
        }
    };
    let [mut p0] = w.split::<1>();
    ls_settle(slot, &mut p0);
    error
}

/// The lock a request is: its box until it is linked on `ls_locks`, a counted link after.
enum Request {
    /// Not linked (`needtolink`).
    Unlinked(PoolBox<Lockf>),
    /// Linked on `ls_locks`.
    Linked(SecondaryRef<ByEntry>),
}

impl Request {
    /// The lock.
    fn lock(&self) -> &Lockf {
        match self {
            Request::Unlinked(b) => b,
            Request::Linked(h) => held(h),
        }
    }
}

/// Set a byte-range lock: `lock` is the request, not linked.
fn lf_setlock<G: ProcGlue>(
    slot: &LockfSlot,
    w: &mut WriteWitness<'_>,
    lock: PoolBox<Lockf>,
) -> Result<(), Errno> {
    let mut lock = lock;
    let mut priority = PLOCK;
    if lock.m.get_mut_owned().ty == F_WRLCK {
        priority += 4;
    }
    priority |= PCATCH;

    // Scan lock list for this file looking for locks that would block us.
    loop {
        let waiting = {
            let [mut p0, mut p1] = w.split::<2>();
            let ls = state(slot, &mut p0);
            let rq = Range::of(&lock, &p1);
            let Some(block) = lf_getblock(ls.locks.first(), &rq, &ls.locks, &p1) else {
                break;
            };
            if rq.flags & F_WAIT == 0 {
                lf_free::<G>(ls, lock);
                return Err(Errno::EAGAIN);
            }

            // Lock is blocked, check for deadlock before proceeding. Note: flock style locks
            // cover the whole file, there is no chance for deadlock.
            if rq.flags & F_POSIX != 0 && lf_deadlock(ls, &rq, &p1) {
                lf_free::<G>(ls, lock);
                return Err(Errno::EDEADLK);
            }
            let block = hold(block);

            // For flock type locks, we must first remove any shared locks that we hold before
            // we sleep waiting for an exclusive lock.
            if rq.flags & F_FLOCK != 0 && rq.ty == F_WRLCK {
                let unlck = Range { ty: F_UNLCK, ..rq };
                lf_clearlock::<G>(ls, &unlck, &mut p1);
            }
            // Add our lock to the blocked list and sleep until we're free. Remember who
            // blocked us (for deadlock detection).
            ls.pending.push(lock);
            let Some(me) = ls.pending.last() else {
                kpanic(format_args!("lf_setlock: the request is not pending"));
            };
            me.m.get_mut(&mut p1).blk.set(block.get());
            held(&block)
                .m
                .get_mut(&mut p1)
                .blkhd
                .push_tail(Member::counted(me));
            hold(me)
        };
        let chan = ptr::from_ref(held(&waiting));
        let error = w.sleep_nsec(chan, priority, "lockf", INFSLP);
        {
            let [mut p0, mut p1] = w.split::<2>();
            let me = held(&waiting);
            // Out of its blocker's queue (a waker that took it there cleared `lf_blk`), then
            // off the pending list.
            let blk = core::mem::take(&mut me.m.get_mut(&mut p1).blk);
            if let Some(b) = blk.get() {
                drop(b.m.get_mut(&mut p1).blkhd.remove(me));
            }
            drop(blk);
            let ls = state(slot, &mut p0);
            lock = ls.pending.take(waiting);
            wakeup_one(ptr::from_ref::<LockfState>(ls));
            let flags = lock.m.get_mut_owned().flags;
            if let Err(e) = error {
                lf_free::<G>(ls, lock);
                return Err(e);
            }
            if flags & F_INTR != 0 {
                lf_free::<G>(ls, lock);
                return Err(Errno::EINTR);
            }
        }
    }

    // No blocks!! Add the lock. Note that we will downgrade or upgrade any overlapping
    // locks this process already owns.
    //
    // Skip over locks owned by other processes. Handle any locks that overlap and are owned
    // by ourselves.
    let [mut p0, mut p1] = w.split::<2>();
    let ls = state(slot, &mut p0);
    let mut req = Request::Unlinked(lock);
    let mut block = ls.locks.first().map(hold);
    loop {
        let rq = Range::of(req.lock(), &p1);
        let (ovcase, overlap) = lf_findoverlap(block.as_ref().map(held), &rq, SELF, &ls.locks, &p1);
        let overlap = overlap.map(hold);
        if ovcase != 0 {
            block = overlap
                .as_ref()
                .and_then(|o| ls.locks.next(held(o)))
                .map(hold);
        }
        // Six cases:
        //      0) no overlap
        //      1) overlap == lock
        //      2) overlap contains lock
        //      3) lock contains overlap
        //      4) overlap starts before lock
        //      5) overlap ends after lock
        let o = overlap.as_ref().map(held);
        match (ovcase, o) {
            (0, _) => {
                if let Request::Unlinked(b) = req {
                    match o {
                        // insert before overlap
                        Some(o) => ls.locks.insert_before(o, b),
                        // first or last lock in list
                        None => ls.locks.push(b),
                    }
                }
                break;
            }
            (1, Some(o)) => {
                // If downgrading lock, others may be able to acquire it.
                let oty = o.m.get(&p1).ty;
                if rq.ty == F_RDLCK && oty == F_WRLCK {
                    lf_wakelock(o, 0, &mut p1);
                }
                o.m.get_mut(&mut p1).ty = rq.ty;
                drop(overlap);
                lf_free_request::<G>(ls, req);
                break;
            }
            (2, Some(o)) => {
                let om = Range::of(o, &p1);
                // Check for common starting point and different types.
                if om.ty == rq.ty {
                    drop(overlap);
                    lf_free_request::<G>(ls, req);
                    break;
                }
                if om.start == rq.start {
                    match req {
                        Request::Linked(ref h) => ls.locks.move_before(o, held(h)),
                        Request::Unlinked(b) => ls.locks.insert_before(o, b),
                    }
                    o.m.get_mut(&mut p1).start = rq.end + 1;
                } else {
                    let Request::Unlinked(b) = req else {
                        kpanic(format_args!("lf_setlock: a linked lock split"));
                    };
                    lf_split::<G>(ls, o, Some(b), &rq, &mut p1);
                }
                lf_wakelock(o, 0, &mut p1);
                break;
            }
            (3, Some(o)) => {
                // Add the new lock if necessary (before it takes the overlap's waiters, which
                // name it), and delete the overlap.
                if let Request::Unlinked(b) = req {
                    ls.locks.insert_before(o, b);
                    let Some(me) = ls.locks.prev(o) else {
                        kpanic(format_args!(
                            "lf_setlock: the lock is not before its overlap"
                        ));
                    };
                    req = Request::Linked(hold(me));
                }
                // If downgrading lock, others may be able to acquire it, otherwise take the
                // list.
                let oty = o.m.get(&p1).ty;
                if rq.ty == F_RDLCK && oty == F_WRLCK {
                    lf_wakelock(o, 0, &mut p1);
                } else {
                    let me = req.lock();
                    while let Some(ltmp) = o.m.get_mut(&mut p1).blkhd.pop_head() {
                        ltmp.get().m.get_mut(&mut p1).blk.set(Some(me));
                        me.m.get_mut(&mut p1).blkhd.push_tail(ltmp);
                    }
                }
                let Some(oh) = overlap else {
                    kpanic(format_args!("lf_setlock: an overlap without its hold"));
                };
                let ob = ls.locks.take(oh);
                lf_free::<G>(ls, ob);
            }
            (4, Some(o)) => {
                // Add lock after overlap on the list.
                match req {
                    Request::Linked(ref h) => ls.locks.move_after(o, held(h)),
                    Request::Unlinked(b) => {
                        ls.locks.insert_after(o, b);
                        let Some(me) = ls.locks.next(o) else {
                            kpanic(format_args!(
                                "lf_setlock: the lock is not after its overlap"
                            ));
                        };
                        req = Request::Linked(hold(me));
                    }
                }
                o.m.get_mut(&mut p1).end = rq.start - 1;
                lf_wakelock(o, 0, &mut p1);
            }
            (5, Some(o)) => {
                // Add the new lock before overlap.
                if let Request::Unlinked(b) = req {
                    ls.locks.insert_before(o, b);
                }
                o.m.get_mut(&mut p1).start = rq.end + 1;
                lf_wakelock(o, 0, &mut p1);
                break;
            }
            _ => kpanic(format_args!("lf_setlock: case {ovcase} without an overlap")),
        }
    }
    Ok(())
}

/// `lf_free(lock)` of a request `lf_setlock` merged into a lock it overlaps: its box, or
/// (linked by an earlier case, which the C frees in place) taken off `ls_locks` first.
fn lf_free_request<G: ProcGlue>(ls: &mut LockfState, req: Request) {
    let b = match req {
        Request::Unlinked(b) => b,
        Request::Linked(h) => ls.locks.take(h),
    };
    lf_free::<G>(ls, b);
}

/// Remove a byte-range lock on an inode.
///
/// Generally, find the lock (or an overlap to that lock) and remove it (or shrink it), then
/// wakeup anyone we can.
fn lf_clearlock<G: ProcGlue>(ls: &mut LockfState, lock: &Range, p1: &mut WitnessPart<'_>) {
    let Some(first) = ls.locks.first() else {
        return;
    };
    let mut lf = Some(hold(first));
    loop {
        let (ovcase, overlap) = lf_findoverlap(lf.as_ref().map(held), lock, SELF, &ls.locks, p1);
        let Some(o) = overlap.filter(|_| ovcase != 0) else {
            break;
        };
        let oh = hold(o);
        let o = held(&oh);
        lf_wakelock(o, 0, p1);

        match ovcase {
            1 => {
                // overlap == lock
                drop(lf);
                let ob = ls.locks.take(oh);
                lf_free::<G>(ls, ob);
            }
            2 => {
                // overlap contains lock: split it
                if o.m.get(p1).start == lock.start {
                    o.m.get_mut(p1).start = lock.end + 1;
                    break;
                }
                // The lock is not part of the list: lf_split keeps it out.
                lf_split::<G>(ls, o, None, lock, p1);
            }
            3 => {
                // lock contains overlap
                lf = ls.locks.next(o).map(hold);
                let ob = ls.locks.take(oh);
                lf_free::<G>(ls, ob);
                continue;
            }
            4 => {
                // overlap starts before lock
                o.m.get_mut(p1).end = lock.start - 1;
                lf = ls.locks.next(o).map(hold);
                continue;
            }
            5 => {
                // overlap ends after lock
                o.m.get_mut(p1).start = lock.end + 1;
            }
            _ => kpanic(format_args!("lf_clearlock: case {ovcase}")),
        }
        break;
    }
}

/// Check whether there is a blocking lock, and if so return its process identifier.
fn lf_getlock(ls: &LockfState, lock: &Range, fl: &mut Flock, p1: &WitnessPart<'_>) {
    if let Some(block) = lf_getblock(ls.locks.first(), lock, &ls.locks, p1) {
        let b = Range::of(block, p1);
        fl.l_type = b.ty;
        fl.l_whence = SEEK_SET as i16;
        fl.l_start = b.start;
        if b.end == -1 {
            fl.l_len = 0;
        } else {
            fl.l_len = b.end - b.start + 1;
        }
        fl.l_pid = b.pid;
    } else {
        fl.l_type = F_UNLCK;
    }
}

/// Walk the list of locks for an inode and return the first blocking lock.
fn lf_getblock<'l>(
    mut lf: Option<&'l Lockf>,
    lock: &Range,
    locks: &'l OwnedList<ByEntry>,
    p1: &WitnessPart<'_>,
) -> Option<&'l Lockf> {
    loop {
        let (ovcase, overlap) = lf_findoverlap(lf, lock, OTHERS, locks, p1);
        let overlap = overlap.filter(|_| ovcase != 0)?;
        // We've found an overlap, see if it blocks us
        if lock.ty == F_WRLCK || overlap.m.get(p1).ty == F_WRLCK {
            return Some(overlap);
        }
        // Nope, point to the next one on the list and see if it blocks us
        lf = locks.next(overlap);
    }
}

/// Walk the list of locks for an inode to find an overlapping lock (if any): the case and
/// the lock it stopped at.
///
/// NOTE: this returns only the FIRST overlapping lock. There may be more than one.
fn lf_findoverlap<'l>(
    mut lf: Option<&'l Lockf>,
    lock: &Range,
    type_: i32,
    locks: &'l OwnedList<ByEntry>,
    p1: &WitnessPart<'_>,
) -> (i32, Option<&'l Lockf>) {
    let (start, end) = (lock.start, lock.end);
    while let Some(l) = lf {
        let m = l.m.get(p1);
        if (type_ & SELF != 0 && m.id != lock.id) || (type_ & OTHERS != 0 && m.id == lock.id) {
            lf = locks.next(l);
            continue;
        }
        // OK, check for overlap
        //
        // Six cases:
        //      0) no overlap
        //      1) overlap == lock
        //      2) overlap contains lock
        //      3) lock contains overlap
        //      4) overlap starts before lock
        //      5) overlap ends after lock

        // Case 0
        if (m.end != -1 && start > m.end) || (end != -1 && m.start > end) {
            if type_ & SELF != 0 && end != -1 && m.start > end {
                return (0, Some(l));
            }
            lf = locks.next(l);
            continue;
        }
        // Case 1
        if m.start == start && m.end == end {
            return (1, Some(l));
        }
        // Case 2
        if m.start <= start && (m.end == -1 || (end != -1 && m.end >= end)) {
            return (2, Some(l));
        }
        // Case 3
        if start <= m.start && (end == -1 || (m.end != -1 && end >= m.end)) {
            return (3, Some(l));
        }
        // Case 4
        if m.start < start && (m.end >= start || m.end == -1) {
            return (4, Some(l));
        }
        // Case 5
        if m.start > start && end != -1 && (m.end > end || m.end == -1) {
            return (5, Some(l));
        }
        kpanic(format_args!("lf_findoverlap: default"));
    }
    (0, None)
}

/// Purge all locks associated with the given lock state: the blocked requests interrupted
/// (`EINTR`) and waited for, the rest freed (uncharged from the glue's uids), the slot
/// emptied.
pub fn lf_purgelocks<G: ProcGlue>(slot: &LockfSlot) {
    let mut w = LOCKF_LOCK.write();
    let chan = {
        let [mut p0, mut p1] = w.split::<2>();
        let Some(ls) = slot.0.get_mut(&mut p0).as_deref_mut() else {
            return;
        };
        ls.refs += 1;

        // Interrupt blocked locks and wait for all of them to finish.
        let mut cur = ls.locks.first();
        while let Some(lock) = cur {
            lf_wakelock(lock, F_INTR, &mut p1);
            cur = ls.locks.next(lock);
        }
        ptr::from_ref::<LockfState>(ls)
    };
    loop {
        {
            let [mut p0, _p1] = w.split::<2>();
            if state(slot, &mut p0).pending.is_empty() {
                break;
            }
        }
        let _ = w.sleep_nsec(chan, PLOCK, "lockfp", INFSLP);
    }

    let [mut p0, _p1] = w.split::<2>();
    let ls = state(slot, &mut p0);
    // Any remaining locks cannot block other locks at this point and can safely be
    // removed.
    while let Some(lock) = ls.locks.pop() {
        lf_free::<G>(ls, lock);
    }

    // This is the last expected thread to hold a lock state reference.
    kassert!(ls.refs == 1);
    ls.refs -= 1;
    ls_settle(slot, &mut p0);
}

/// Split a lock and a contained region into two or three locks as necessary: `lock1` keeps
/// what is before `lock2`'s range (`range2`), a new lock what is after it, and `lock2`, when
/// given, is linked between them (`lf_setlock`; `lf_clearlock` gives none, its request
/// staying out of the list as the C's `TAILQ_REMOVE` after the split leaves it).
fn lf_split<G: ProcGlue>(
    ls: &mut LockfState,
    lock1: &Lockf,
    lock2: Option<PoolBox<Lockf>>,
    range2: &Range,
    p1: &mut WitnessPart<'_>,
) {
    let l1 = Range::of(lock1, p1);
    // Check to see if splitting into only two pieces.
    if l1.start == range2.start {
        lock1.m.get_mut(p1).start = range2.end + 1;
        if let Some(b) = lock2 {
            ls.locks.insert_before(lock1, b);
        }
        return;
    }
    if l1.end == range2.end {
        lock1.m.get_mut(p1).end = range2.start - 1;
        if let Some(b) = lock2 {
            ls.locks.insert_after(lock1, b);
        }
        return;
    }
    // Make a new lock consisting of the last part of the encompassing lock
    let m = LockfMut {
        flags: l1.flags,
        ty: l1.ty,
        start: range2.end + 1,
        end: l1.end,
        id: l1.id,
        blk: SecondaryRef::new(),
        blkhd: SecondaryList::new(),
    };
    let Some(splitlock) = lf_alloc::<G>(lock1.uid, 0, m, l1.pid) else {
        kpanic(format_args!("lf_split: a lock that cannot fail failed"));
    };
    ls.refs += 1;
    lock1.m.get_mut(p1).end = range2.start - 1;

    match lock2 {
        Some(b) => {
            ls.locks.insert_after(lock1, b);
            let Some(l2) = ls.locks.next(lock1) else {
                kpanic(format_args!("lf_split: the lock is not after its split"));
            };
            let l2 = hold(l2);
            ls.locks.insert_after(held(&l2), splitlock);
        }
        None => ls.locks.insert_after(lock1, splitlock),
    }
}

/// Wakeup a blocklist: every lock `lock` blocks is taken off its queue, forgets its blocker,
/// gets `flags` and is woken.
fn lf_wakelock(lock: &Lockf, flags: i32, p1: &mut WitnessPart<'_>) {
    while let Some(wakelock) = lock.m.get_mut(p1).blkhd.pop_head() {
        let wm = wakelock.get().m.get_mut(p1);
        wm.blk.set(None);
        wm.flags |= flags;
        wakeup_one(ptr::from_ref::<Lockf>(wakelock.get()));
    }
}

/// Returns `true` if the given lock would cause a deadlock.
fn lf_deadlock(ls: &LockfState, lock: &Range, p1: &WitnessPart<'_>) -> bool {
    let mut lf = ls.locks.first();
    while let Some(block) = lf_getblock(lf, lock, &ls.locks, p1) {
        lf = ls.locks.next(block);
        if block.m.get(p1).flags & F_POSIX == 0 {
            continue;
        }
        for pending in &ls.pending {
            let Some(pblk) = pending.m.get(p1).blk.get() else {
                continue; // lock already unblocked
            };
            if pending.pid == block.pid && pblk.pid == lock.pid {
                return true;
            }
        }
    }
    false
}
/* </CODE> */
/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the advisory record locks (LZ's six, on the kernel's types): the
    // splitting, merging, upgrading and downgrading of one owner's ranges, the conflicts
    // between owners, `F_GETLK`, the range conversion of `lf_advlock`, `lf_purgelocks` and the
    // uid charge; and, on a state built by hand (the sleeps a blocked request makes are not a
    // host test's), deadlock detection and the wakeup of a blocked request.

    use std::vec::Vec;

    use super::*;
    use crate::fs::testfs::{TestGlue, test_thread};
    use crate::proc::Thread;

    type G = TestGlue;

    const A: LockOwner = LockOwner::File(0x1000);
    const B: LockOwner = LockOwner::File(0x2000);

    /// The test's thread as `curproc` (test_thread), the pools made.
    fn setup() -> (std::sync::MutexGuard<'static, ()>, &'static Thread<G>) {
        let (g, p) = test_thread();
        lf_init();
        // The pools are the heap doubles: they need no reset between tests (the pools' note).
        assert_eq!(crate::testutil::memory_is_the_doubles(), Some(true));
        (g, p)
    }

    /// A `struct flock` for `[start, start + len)` from `SEEK_SET`.
    fn fl(type_: i16, start: Off, len: Off) -> Flock {
        Flock {
            l_start: start,
            l_len: len,
            l_pid: 0,
            l_type: type_,
            l_whence: SEEK_SET as i16,
        }
    }

    /// `F_SETLK` of `[start, start + len)` by `id`, without waiting.
    fn setlk(
        slot: &LockfSlot,
        id: LockOwner,
        type_: i16,
        start: Off,
        len: Off,
    ) -> Result<(), Errno> {
        lf_advlock::<G>(slot, 0, id, F_SETLK, &mut fl(type_, start, len), F_POSIX)
    }

    /// `F_UNLCK` of `[start, start + len)` by `id`.
    fn unlk(slot: &LockfSlot, id: LockOwner, start: Off, len: Off) -> Result<(), Errno> {
        lf_advlock::<G>(
            slot,
            0,
            id,
            i32::from(F_UNLCK),
            &mut fl(F_UNLCK, start, len),
            F_POSIX,
        )
    }

    /// The locks of the slot, in list order: `(owner, type, start, end)`.
    fn locks(slot: &LockfSlot) -> Vec<(LockOwner, i16, Off, Off)> {
        let mut w = LOCKF_LOCK.write();
        let [mut p0, p1] = w.split::<2>();
        slot.0
            .get_mut(&mut p0)
            .as_deref()
            .map_or_else(Vec::new, |ls| {
                ls.locks
                    .iter()
                    .map(|l| {
                        let m = l.m.get(&p1);
                        (m.id, m.ty, m.start, m.end)
                    })
                    .collect()
            })
    }

    #[test]
    fn unlocking_the_middle_splits_a_lock_in_three_pieces_minus_one() {
        let (_g, _p) = setup();
        let slot = LockfSlot::new();
        setlk(&slot, A, F_WRLCK, 0, 100).unwrap();
        assert_eq!(locks(&slot), [(A, F_WRLCK, 0, 99)]);

        unlk(&slot, A, 10, 10).unwrap();
        assert_eq!(locks(&slot), [(A, F_WRLCK, 0, 9), (A, F_WRLCK, 20, 99)]);

        // Unlocking the front and the back shrinks the pieces (cases 2 and 4/5).
        unlk(&slot, A, 0, 5).unwrap();
        unlk(&slot, A, 90, 0).unwrap();
        assert_eq!(locks(&slot), [(A, F_WRLCK, 5, 9), (A, F_WRLCK, 20, 89)]);

        // Unlocking everything frees the state and clears the slot.
        unlk(&slot, A, 0, 0).unwrap();
        assert!(slot.is_empty());
    }

    #[test]
    fn a_different_type_in_the_middle_splits_the_owners_lock() {
        let (_g, _p) = setup();
        let slot = LockfSlot::new();
        setlk(&slot, A, F_RDLCK, 0, 100).unwrap();
        setlk(&slot, A, F_WRLCK, 40, 20).unwrap();
        assert_eq!(
            locks(&slot),
            [
                (A, F_RDLCK, 0, 39),
                (A, F_WRLCK, 40, 59),
                (A, F_RDLCK, 60, 99)
            ]
        );

        // The same type inside an existing lock changes nothing (case 2).
        setlk(&slot, A, F_RDLCK, 70, 5).unwrap();
        assert_eq!(locks(&slot).len(), 3);

        // A lock covering all of them replaces each of them (case 3).
        setlk(&slot, A, F_WRLCK, 0, 100).unwrap();
        assert_eq!(locks(&slot), [(A, F_WRLCK, 0, 99)]);

        // Downgrading the exact range keeps one lock (case 1).
        setlk(&slot, A, F_RDLCK, 0, 100).unwrap();
        assert_eq!(locks(&slot), [(A, F_RDLCK, 0, 99)]);

        // A different type at the same start: the new lock goes first (case 2, common
        // start).
        setlk(&slot, A, F_WRLCK, 0, 10).unwrap();
        assert_eq!(locks(&slot), [(A, F_WRLCK, 0, 9), (A, F_RDLCK, 10, 99)]);
        unlk(&slot, A, 0, 0).unwrap();
        assert!(slot.is_empty());
    }

    #[test]
    fn overlapping_ends_trim_the_older_lock() {
        let (_g, _p) = setup();
        let slot = LockfSlot::new();
        setlk(&slot, A, F_RDLCK, 0, 50).unwrap();
        // Starts inside, ends after: the old one keeps its front (case 4).
        setlk(&slot, A, F_WRLCK, 25, 50).unwrap();
        assert_eq!(locks(&slot), [(A, F_RDLCK, 0, 24), (A, F_WRLCK, 25, 74)]);
        // Starts before, ends inside: the old one keeps its back (case 5).
        setlk(&slot, A, F_RDLCK, 70, 30).unwrap();
        setlk(&slot, A, F_WRLCK, 60, 20).unwrap();
        assert_eq!(
            locks(&slot),
            [
                (A, F_RDLCK, 0, 24),
                (A, F_WRLCK, 25, 59),
                (A, F_WRLCK, 60, 79),
                (A, F_RDLCK, 80, 99)
            ]
        );
        unlk(&slot, A, 0, 0).unwrap();
        assert!(slot.is_empty());
    }

    #[test]
    fn other_owners_conflict_on_writes_and_getlk_reports_the_blocker() {
        let (_g, p) = setup();
        let slot = LockfSlot::new();
        setlk(&slot, A, F_RDLCK, 0, 10).unwrap();
        setlk(&slot, A, F_WRLCK, 20, 10).unwrap();

        // Readers share; a writer over a reader or a reader over a writer must wait.
        setlk(&slot, B, F_RDLCK, 5, 10).unwrap();
        assert_eq!(setlk(&slot, B, F_WRLCK, 0, 3), Err(Errno::EAGAIN));
        assert_eq!(setlk(&slot, B, F_RDLCK, 25, 1), Err(Errno::EAGAIN));
        // Next to the locks there is no conflict.
        setlk(&slot, B, F_WRLCK, 30, 0).unwrap();

        // F_GETLK finds the first lock of another owner that blocks the request.
        let mut q = fl(F_WRLCK, 0, 0);
        lf_advlock::<G>(&slot, 0, B, F_GETLK, &mut q, F_POSIX).unwrap();
        assert_eq!((q.l_type, q.l_start, q.l_len), (F_RDLCK, 0, 10));
        assert_eq!(q.l_pid, p.process().ps_pid);
        let mut q = fl(F_RDLCK, 0, 15);
        lf_advlock::<G>(&slot, 0, B, F_GETLK, &mut q, F_POSIX).unwrap();
        assert_eq!(q.l_type, F_UNLCK);

        // Each owner's locks are sorted by start; another owner's do not place a new lock, so
        // they end up after the first owner's.
        assert_eq!(
            locks(&slot),
            [
                (A, F_RDLCK, 0, 9),
                (A, F_WRLCK, 20, 29),
                (B, F_RDLCK, 5, 14),
                (B, F_WRLCK, 30, -1)
            ]
        );
        // The owners differ by kind as by address: a table's lock is another owner than a
        // file's of the same address.
        let t = LockOwner::Table(0x1000);
        assert_eq!(setlk(&slot, t, F_WRLCK, 0, 1), Err(Errno::EAGAIN));
        unlk(&slot, A, 0, 0).unwrap();
        unlk(&slot, B, 0, 0).unwrap();
        assert!(slot.is_empty());
    }

    #[test]
    fn ranges_follow_whence_and_negative_lengths() {
        let (_g, _p) = setup();
        let slot = LockfSlot::new();
        // SEEK_END counts from the size; a negative length reaches backwards.
        let mut f = fl(F_WRLCK, -10, 5);
        f.l_whence = SEEK_END as i16;
        lf_advlock::<G>(&slot, 1000, A, F_SETLK, &mut f, F_POSIX).unwrap();
        lf_advlock::<G>(&slot, 0, A, F_SETLK, &mut fl(F_WRLCK, 100, -10), F_POSIX).unwrap();
        assert_eq!(locks(&slot), [(A, F_WRLCK, 90, 99), (A, F_WRLCK, 990, 994)]);

        assert_eq!(setlk(&slot, A, F_WRLCK, -1, 1), Err(Errno::EINVAL));
        assert_eq!(setlk(&slot, A, F_WRLCK, 5, -10), Err(Errno::EINVAL));
        assert_eq!(
            setlk(&slot, A, F_WRLCK, 10, Off::MAX),
            Err(Errno::EOVERFLOW)
        );
        let mut f = fl(F_WRLCK, 0, 1);
        f.l_whence = 7;
        assert_eq!(
            lf_advlock::<G>(&slot, 0, A, F_SETLK, &mut f, F_POSIX),
            Err(Errno::EINVAL)
        );
        // An unknown operation frees its lock and answers EINVAL.
        assert_eq!(
            lf_advlock::<G>(&slot, 0, A, 99, &mut fl(F_WRLCK, 0, 1), F_POSIX),
            Err(Errno::EINVAL)
        );

        // Unlocking or asking with no state answers without allocating one.
        let empty = LockfSlot::new();
        let mut q = fl(F_WRLCK, 0, 0);
        lf_advlock::<G>(&empty, 0, A, F_GETLK, &mut q, F_POSIX).unwrap();
        assert_eq!(q.l_type, F_UNLCK);
        assert!(empty.is_empty());

        lf_purgelocks::<G>(&slot);
        assert!(slot.is_empty());
    }

    #[test]
    fn locks_are_charged_to_the_owners_uid() {
        let (_g, p) = setup();
        let uid = p.ucred().cr_uid.get();
        let count = || with_uidinfo::<G, _>(uid, |uip| uip.ui_lockcnt.get());
        let before = count();
        let slot = LockfSlot::new();
        setlk(&slot, A, F_WRLCK, 0, 100).unwrap();
        unlk(&slot, A, 10, 10).unwrap(); // a split allocates a second lock
        assert_eq!(count(), before + 2);
        lf_purgelocks::<G>(&slot);
        assert_eq!(count(), before);
    }

    /// A lock of `pid` on `[start, end]` made by hand (what a sleeping request or another
    /// process would hold), its uid charge taken as `lf_alloc` takes it.
    fn handmade(id: LockOwner, ty: i16, start: Off, end: Off, pid: Pid) -> PoolBox<Lockf> {
        let m = LockfMut {
            flags: F_POSIX | F_WAIT,
            ty,
            start,
            end,
            id,
            blk: SecondaryRef::new(),
            blkhd: SecondaryList::new(),
        };
        lf_alloc::<G>(0, 0, m, pid).unwrap()
    }

    #[test]
    fn a_request_that_would_close_a_wait_cycle_is_edeadlk() {
        let (_g, _p) = setup();
        let slot = LockfSlot::new();
        // Process 1 (the test's) holds [0, 9]; process 2 holds [20, 29] and waits for [0, 9]
        // (a pending lock blocked by process 1's).
        setlk(&slot, A, F_WRLCK, 0, 10).unwrap();
        {
            let mut w = LOCKF_LOCK.write();
            let [mut p0, mut p1] = w.split::<2>();
            let ls = state(&slot, &mut p0);
            ls.refs += 2;
            ls.locks.push(handmade(B, F_WRLCK, 20, 29, 2));
            ls.pending.push(handmade(B, F_WRLCK, 0, 9, 2));
            let (Some(held1), Some(waiter)) = (ls.locks.first(), ls.pending.first()) else {
                panic!("the state built by hand");
            };
            waiter.m.get_mut(&mut p1).blk.set(Some(held1));
            held1
                .m
                .get_mut(&mut p1)
                .blkhd
                .push_tail(Member::counted(waiter));
            // The cycle: process 1 asking for process 2's range.
            let rq = Range {
                ty: F_WRLCK,
                start: 20,
                end: 29,
                id: A,
                flags: F_POSIX | F_WAIT,
                pid: 1,
            };
            assert!(lf_deadlock(ls, &rq, &p1));
            // Another range of process 2's, not waited on by process 2: no cycle with a
            // process 3.
            let rq3 = Range { pid: 3, ..rq };
            assert!(!lf_deadlock(ls, &rq3, &p1));
        }
        // Through lf_advlock: F_WAIT, POSIX, blocked by process 2, which waits for us.
        assert_eq!(
            lf_advlock::<G>(
                &slot,
                0,
                A,
                F_SETLK,
                &mut fl(F_WRLCK, 20, 10),
                F_POSIX | F_WAIT
            ),
            Err(Errno::EDEADLK)
        );

        // lf_wakelock frees the waiter from its blocker, F_INTR set (what lf_purgelocks
        // does to every lock before it waits for the pending ones).
        let mut w = LOCKF_LOCK.write();
        let [mut p0, mut p1] = w.split::<2>();
        let ls = state(&slot, &mut p0);
        let (Some(held1), Some(waiter)) = (ls.locks.first(), ls.pending.first()) else {
            panic!("the state built by hand");
        };
        lf_wakelock(held1, F_INTR, &mut p1);
        assert!(held1.m.get(&p1).blkhd.is_empty());
        assert!(!waiter.m.get(&p1).blk.is_some());
        assert!(waiter.m.get(&p1).flags & F_INTR != 0);
        // The waiter leaves as a woken request does, and the rest goes.
        let wb = ls.pending.pop().unwrap();
        lf_free::<G>(ls, wb);
        while let Some(l) = ls.locks.pop() {
            lf_free::<G>(ls, l);
        }
        assert_eq!(ls.refs, 0);
        ls_settle(&slot, &mut p0);
        drop(p1);
        drop(w);
        assert!(slot.is_empty());
    }

    #[test]
    fn a_flock_request_clears_its_own_shared_lock_only_when_it_waits() {
        let (_g, _p) = setup();
        let slot = LockfSlot::new();
        let flk = |id, ty| lf_advlock::<G>(&slot, 0, id, F_SETLK, &mut fl(ty, 0, 0), F_FLOCK);
        flk(A, F_RDLCK).unwrap();
        flk(B, F_RDLCK).unwrap();
        // Without F_WAIT the upgrade fails first: A's shared lock stays (lf_setlock returns
        // EAGAIN before the flock clear).
        assert_eq!(flk(A, F_WRLCK), Err(Errno::EAGAIN));
        assert_eq!(locks(&slot), [(A, F_RDLCK, 0, -1), (B, F_RDLCK, 0, -1)]);
        lf_purgelocks::<G>(&slot);
        assert!(slot.is_empty());
    }
}
/* </TESTS> */
