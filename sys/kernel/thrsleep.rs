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
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
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
 *	@(#)kern_synch.c	8.6 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `__thrsleep(2)` and `__thrwakeup(2)`: a thread sleeps on an address of its process (its
//! "ident") until a sibling wakes it, its absolute timeout passes or a signal comes;
//! librthread's `_rthread_dl_lock` is the one compiled caller at the pin, and a program may
//! call the libc stubs itself.
//!
//! Upstream: sys/kern/kern_synch.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_synch.rs@9fd12af2c575
//!
//! A sleeper is a [`Tslpentry`] on its own stack, pinned, linked on one of 64 buckets chosen by
//! the ident ([`thrsleep_bucket`]), or on the shared bucket for ident -1, which any process
//! may wake, and asleep on its own address ("thrsleep"). Each bucket has its mutex at
//! `IPL_MPFLOOR`; `SCHED_LOCK` nests inside it, never the reverse. The order of
//! [`thrsleep`] is the contract with userland's lock: the record is queued before the
//! caller's lock word is freed (a copyout of [`_ATOMIC_LOCK_UNLOCKED`], an `int`), the abort
//! word is read after it, and whether to sleep is decided after `sleep_setup`, so a wakeup
//! that comes in between is never lost. The two calls always succeed: the error is their
//! value (`*retval`), so `ERESTART` is `ECANCELED`, a timeout `EWOULDBLOCK`, and a wakeup that
//! raced a timeout or a signal wins. `__thrwakeup` wakes at most `n` (all for `n <= 0`) of the
//! caller's process's sleepers on the ident, `ESRCH` when none; ident -1 wakes every sleeper
//! of every process and answers 0.
//!
//! The rule the C states for `tslp_p` holds here for the record's link: only a holder of the
//! bucket's mutex takes a record off a bucket, and once it is off its sleeper may return and
//! its frame go; the sleeper reads its link without the mutex.
//!
//! ## Deviations
//! - `KTRACE` is not configured (`ktrabstimespec`).
//! - [`thrsleep`] takes the arguments `sys___thrsleep` copied in and checked (the timeout as
//!   an `Option<Timespec>`) instead of pointing `SCARG(uap, tp)` at the kernel copy, and
//!   returns `Result<(), Errno>`; the system calls still answer with the error as their value
//!   (LZ's).
//! - The two "take the record off unless a waker did" blocks of `thrsleep` (after the sleep,
//!   and `leave:`) are one helper, [`thrsleep_unwait`], and the end after `sleep_finish` is
//!   [`thrsleep_settle`], so the host tests reach the race of a wakeup with a timeout (LZ's).
//!
//! ## Redesign
//! - Into the kernel crate (an `lz-sync` of LZ's port, which `bsd`'s `kern_synch.rs` could
//!   not take: its sleep queues are `kernel::sched`'s since N4.5a). LZ's five `unsafe` (the
//!   buckets' `unsafe impl Sync` over `Cell`-linked `TailqHead`s and a legacy `Mutex`; the
//!   insertion of a stack record into a `'static` list; its removal; the move of a matching
//!   record to the waker's local list; the dereference of the `tslp_p` thread pointer the
//!   waker swapped out) -> 0: the record is pinned on the sleeper's stack
//!   (`core::pin::pin!`) on a `hal::pinlist::PinList` bound to its bucket's
//!   `hal::mutex::Mutexed<()>`, whose guard is the list's witness; the buckets are `const`
//!   statics, `Sync` by their types.
//! - `tslp_p` is no field: "queued" (`tslp_p != NULL`) **is** being linked
//!   ([`Tslpentry::waiting`], the entry's own word, cleared last by the unlink itself), so a
//!   waker never touches a record after letting it go, and a frame dropped while linked stops
//!   the kernel (the entry's drop) instead of leaving a dangling link. `tslp_ps` is the
//!   process's address, compared, never followed.
//! - The waker's `wakeup_proc(tslp_p)` is a wakeup of the sleeper's own channel, the record's
//!   address ([`crate::sched::wakeup_n_locked`]), under the one `SCHED_LOCK` hold the C
//!   takes for the batch, as `kernel::futex` wakes (docs/IDIOMS.md, the stack waiter): no
//!   reference to the sleeping thread is kept in a list that outlives it. The same thread is
//!   woken, at the same point; the difference is inside the kernel only: a sleeper asleep
//!   elsewhere (a page fault in its unlock's `copyout` or its abort word's `copyin32`) is not
//!   cut short, which the C's `wakeup_proc` would do (and the fault would sleep again), and
//!   the wakeup walks the channel's sleep queue (`LOOKUP(ident)`, one of 128) where the C has
//!   the thread (docs/ROADMAP.md, N12).
//! - `tslp_init` has no body: the buckets' lists and mutexes are `const` initialisers, so
//!   nothing runs at boot (`main`'s `unported!("tslp_init")` is gone).
//! - `<machine/spinlock.h>`'s word and values are `hal::spinlock`'s, one definition for both
//!   machines (LZ's `MachineSpinlock` trait is not taken).

use core::pin::{Pin, pin};
use core::ptr;

use hal::intr::IPL_MPFLOOR;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::pinlist::{PinEntry, PinList};
use hal::sleep::{INFSLP, PCATCH, PWAIT};
use hal::spinlock::{_ATOMIC_LOCK_UNLOCKED, AtomicLock};
use hal::sys::errno::Errno;
use hal::thread::sched_lock;
use hal::user::UserPtr;

use crate::clock::Timespec;
use crate::proc::{ProcGlue, Thread};
use crate::sched::{sleep_finish, sleep_setup, wakeup_n_locked};
use crate::time::clock_gettime;

/// `TSLP_BUCKET_BITS`.
const TSLP_BUCKET_BITS: u32 = 6;
/// `TSLP_BUCKET_SIZE`.
const TSLP_BUCKET_SIZE: usize = 1 << TSLP_BUCKET_BITS;
/// `TSLP_BUCKET_MASK`.
const TSLP_BUCKET_MASK: isize = TSLP_BUCKET_SIZE as isize - 1;

/// `MAXTSLP`: the longest sleep that is not `INFSLP`.
const MAXTSLP: u64 = u64::MAX - 1;

/// `struct tslpentry`: a thread sleeping in `__thrsleep(2)`, pinned on its stack while it is
/// on a bucket.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct tslpentry's tslp_* names so the C can be grepped"
)]
struct Tslpentry {
    /// \[b\] `tslp_link`: the link on the bucket's list (or a waker's local list); linked is
    /// the C's `tslp_p != NULL`.
    tslp_link: PinEntry<Tslpentry>,
    /// \[I\] `tslp_ps`: the sleeper's process, an address compared, never followed.
    tslp_ps: usize,
    /// \[I\] `tslp_ident`: the address slept on, as a `long`.
    tslp_ident: isize,
}

impl Tslpentry {
    /// The record of a thread of the process at `ps` sleeping on `ident`, on no list yet.
    fn new(ps: usize, ident: isize) -> Self {
        Self {
            tslp_link: PinEntry::new(),
            tslp_ps: ps,
            tslp_ident: ident,
        }
    }

    /// `entry.tslp_p != NULL`: whether the record is still queued. Read by its sleeper
    /// without the lock, a snapshot: once it reads `false` no waker touches the record again.
    fn waiting(&self) -> bool {
        self.tslp_link.is_linked()
    }

    /// The channel the sleeper sleeps on: the record's own address (`sleep_setup(&entry,
    /// ..)`).
    fn chan(&self) -> usize {
        ptr::from_ref(self).addr()
    }
}

hal::pin_adapter!(
    /// `TAILQ_HEAD(tslpqueue, tslpentry)`: a bucket's sleepers, in arrival order.
    Tslpqueue: Tslpentry, tslp_link
);

/// `struct tslp_bucket`: one chain of `__thrsleep(2)` sleepers, cache-line aligned.
#[repr(align(64))]
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct tslp_bucket's tsb_* names so the C can be grepped"
)]
struct TslpBucket {
    /// \[b\] `tsb_list`.
    tsb_list: PinList<Tslpqueue>,
    /// `tsb_lock`: a mutex at `IPL_MPFLOOR`, the list's witness.
    tsb_lock: Mutexed<()>,
}

impl TslpBucket {
    /// `TAILQ_INIT` and `mtx_init(&tsb_lock, IPL_MPFLOOR)`: an empty bucket.
    const fn new() -> Self {
        Self {
            tsb_list: PinList::new(),
            tsb_lock: Mutexed::new(IPL_MPFLOOR, ()),
        }
    }
}

/// `tsb_shared`: thrsleep queue shared between processes (ident -1).
static TSB_SHARED: TslpBucket = TslpBucket::new();

/// `tsb_buckets[TSLP_BUCKET_SIZE]`.
static TSB_BUCKETS: [TslpBucket; TSLP_BUCKET_SIZE] =
    [const { TslpBucket::new() }; TSLP_BUCKET_SIZE];

/// `thrsleep_unlock`: frees the caller's userland spinlock, if it named one, by copying
/// `_ATOMIC_LOCK_UNLOCKED` out over it (a plain `copyout` of an `int`, as the C's: userland's
/// spinlock needs no atomic store to be released).
fn thrsleep_unlock(atomiclock: UserPtr<AtomicLock>) -> Result<(), Errno> {
    if atomiclock.is_null() {
        return Ok(());
    }

    atomiclock.copyout(&_ATOMIC_LOCK_UNLOCKED)
}

/// `thrsleep_bucket`: the bucket of `ident` (any but 0 and -1). The low three bits of an
/// address say little; the next twelve are folded into six.
fn thrsleep_bucket(ident: isize) -> &'static TslpBucket {
    let mut ident = ident >> 3;
    ident ^= ident >> TSLP_BUCKET_BITS;
    ident &= TSLP_BUCKET_MASK;

    &TSB_BUCKETS[ident as usize]
}

/// Takes `entry` off `tsb` unless a waker already did; whether it was still queued. The C
/// writes this twice in `thrsleep`, after the sleep and on the `leave:` path.
fn thrsleep_unwait(tsb: &TslpBucket, entry: &Tslpentry) -> bool {
    let mut w = tsb.tsb_lock.lock();
    let queued = entry.waiting();
    if queued {
        tsb.tsb_list.remove(&mut w, entry);
    }
    drop(w);

    queued
}

/// The end of `thrsleep` after `sleep_finish`: a sleeper still queued (timed out, signalled,
/// or never asleep) takes itself off and keeps the sleep's error; one a waker already took was
/// woken, which wins over a timeout or a signal that raced it. An interrupted sleep that
/// would restart is `ECANCELED` (the call answers with its error and never restarts).
fn thrsleep_settle(
    tsb: &TslpBucket,
    entry: &Tslpentry,
    error: Result<(), Errno>,
) -> Result<(), Errno> {
    let mut error = error;

    if error.is_err() || entry.waiting() {
        if !thrsleep_unwait(tsb, entry) {
            error = Ok(());
        }

        if error == Err(Errno::ERESTART) {
            error = Err(Errno::ECANCELED);
        }
    }

    error
}

/// `thrsleep`: `__thrsleep(2)` once `sys___thrsleep` has copied in and checked the timeout
/// `tsp` (an absolute time on `clock_id`): sleeps on `ident`, freeing the userland spinlock
/// at `lock` once queued, unless the word at `abortp` is set.
fn thrsleep<G: ProcGlue>(
    p: &Thread<G>,
    ident: isize,
    clock_id: i32,
    tsp: Option<Timespec>,
    lock: UserPtr<AtomicLock>,
    abortp: UserPtr<u32>,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if ident == 0 {
        return Err(Errno::EINVAL);
    }
    if let Some(tsp) = tsp {
        let now = clock_gettime(p, clock_id)?;
        // KTRACE (ktrabstimespec): not configured.

        if tsp <= now {
            // Already passed: still do the unlock.
            thrsleep_unlock(lock)?;
            return Err(Errno::EWOULDBLOCK);
        }

        // MAX(1, MIN(TIMESPEC_TO_NSEC(tsp - now), MAXTSLP))
        nsecs = tsp.sub(now).to_nsec().clamp(1, MAXTSLP);
    }

    let tsb = if ident == -1 {
        &TSB_SHARED
    } else {
        thrsleep_bucket(ident)
    };

    // Interlock with wakeup: queued before the caller's lock is freed.
    let entry: Pin<&mut Tslpentry> = pin!(Tslpentry::new(ptr::from_ref(p.process()).addr(), ident));
    let entry: Pin<&Tslpentry> = entry.into_ref();
    {
        let mut w = tsb.tsb_lock.lock();
        tsb.tsb_list.insert_tail(&mut w, entry);
    }

    // Do not return before the record is off its bucket! (Every path below ends with it off:
    // a waker took it, or `thrsleep_unwait` does.)

    let early = match thrsleep_unlock(lock) {
        Err(error) => Err(error),
        Ok(()) if abortp.is_null() => Ok(()),
        Ok(()) => match abortp.copyin32() {
            Err(error) => Err(error),
            Ok(0) => Ok(()),
            Ok(_) => Err(Errno::EINTR),
        },
    };
    if let Err(error) = early {
        // leave: the error stands even when a waker took the record meanwhile.
        if entry.waiting() {
            thrsleep_unwait(tsb, &entry);
        }
        return Err(error);
    }

    sleep_setup::<G>(entry.chan(), PWAIT | PCATCH, "thrsleep");
    let error = sleep_finish::<G>(nsecs, entry.waiting());
    thrsleep_settle(tsb, &entry, error)
}

/// `__thrsleep(2)`: sleeps on `ident` until `__thrwakeup(2)`, the absolute timeout at `tp`
/// on `clock_id` (none when `tp` is 0), or a signal; frees the userland spinlock at `lock`
/// (if not 0) once queued and gives up with `EINTR` when the word at `abort` (if not 0) is
/// set. The value is the C's `*retval`: 0 or the error's number; the system call itself
/// always succeeds.
#[allow(non_snake_case, reason = "the C's name, sys___thrsleep")]
pub fn sys___thrsleep<G: ProcGlue>(
    p: &Thread<G>,
    ident: isize,
    clock_id: i32,
    tp: usize,
    lock: usize,
    abort: usize,
) -> i32 {
    let mut tsp = None;

    if tp != 0 {
        let ts = match UserPtr::<Timespec>::new(tp).copyin() {
            Ok(ts) => ts,
            Err(error) => return error.as_i32(),
        };
        if !ts.is_valid() {
            return Errno::EINVAL.as_i32();
        }
        tsp = Some(ts);
    }

    match thrsleep(
        p,
        ident,
        clock_id,
        tsp,
        UserPtr::new(lock),
        UserPtr::new(abort),
    ) {
        Ok(()) => 0,
        Err(error) => error.as_i32(),
    }
}

/// `tslp_wakeups`: wakes every sleeper of `tslpq`, a list of the bucket mutex whose guard `w`
/// the caller holds (a waker's local list, or the shared bucket itself). Taking a record off
/// `tslpq` releases it (the C's `tslp_p = NULL`): its sleeper may return from then on, so
/// only its channel, read before, is used after. `SCHED_LOCK` is held for the batch, as the
/// C's, so a returned sleeper cannot sleep again on that channel before its wakeup lands.
fn tslp_wakeups<G: ProcGlue>(w: &mut MutexedGuard<'_, ()>, tslpq: &PinList<Tslpqueue>) {
    let mut h = sched_lock();
    while let Some(chan) = tslpq.first(&*w).map(Tslpentry::chan) {
        tslpq.remove_first(&mut *w);
        // wakeup_proc(p): the one thread asleep on the record's address.
        wakeup_n_locked::<G>(&mut h, chan, 1);
    }
    drop(h);
}

/// `__thrwakeup(2)`: wakes at most `n` (all when `n <= 0`) of the caller's process's threads
/// sleeping on `ident`; ident -1 wakes every sleeper on it, of any process, whatever `n`. The
/// value is the C's `*retval`: 0, `EINVAL` for ident 0, `ESRCH` when none matched; the system
/// call itself always succeeds.
#[allow(non_snake_case, reason = "the C's name, sys___thrwakeup")]
pub fn sys___thrwakeup<G: ProcGlue>(p: &Thread<G>, ident: isize, n: i32) -> i32 {
    if ident == 0 {
        return Errno::EINVAL.as_i32();
    }

    if ident == -1 {
        // Ident -1 is a channel several userland locks may use at once, and nothing here
        // tells their sleepers apart, so the waker wakes them all and lets each recheck.
        let mut w = TSB_SHARED.tsb_lock.lock();
        // Every record leaves the list as it is woken: the C's TAILQ_INIT after the walk.
        tslp_wakeups::<G>(&mut w, &TSB_SHARED.tsb_list);
        drop(w);

        return 0;
    }

    let tsb = thrsleep_bucket(ident);
    let ps = ptr::from_ref(p.process()).addr();
    // `++found == n`: a count of zero or less is never reached, so every match is taken.
    let max = usize::try_from(n)
        .ok()
        .filter(|&m| m > 0)
        .unwrap_or(usize::MAX);

    let mut w = tsb.tsb_lock.lock();
    let wq: PinList<Tslpqueue> = PinList::new();
    let found = tsb.tsb_list.move_where(
        &mut w,
        max,
        |e| e.tslp_ident == ident && e.tslp_ps == ps,
        &wq,
    );
    if found != 0 {
        tslp_wakeups::<G>(&mut w, &wq);
    }
    drop(w);

    if found != 0 { 0 } else { Errno::ESRCH.as_i32() }
}

const _: () = assert!(
    core::mem::align_of::<TslpBucket>() == 64,
    "struct tslp_bucket is __aligned(64)"
);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;
    use std::boxed::Box;
    use std::time::Duration;

    use super::*;
    use crate::proc::{P_INSCHED, Process, SSLEEP};
    use crate::sched::tests::{SchedGlue, T, TABLES, VCPUS, run_as, setup, thread};
    use crate::testutil;
    use crate::time::CLOCK_MONOTONIC;

    type G = SchedGlue;

    /// This module's words of the user-copy double, `0xdc00..0xe000` (futex's tests, below,
    /// hold the same scheduler fixture lock): the caller's lock word, the abort word and a
    /// `struct timespec`.
    const WORDS: usize = testutil::USER_BASE + 0xdc00;
    /// The caller's spinlock.
    const LOCK: usize = WORDS;
    /// The abort word.
    const ABORT: usize = WORDS + 0x10;
    /// The timeout.
    const TS: usize = WORDS + 0x20;
    /// An address outside the user memory.
    const UNMAPPED: usize = 8;

    /// The lock word now.
    fn lock_word() -> i32 {
        UserPtr::<i32>::new(LOCK).copyin().unwrap()
    }

    /// The lock word set to `v` (1 is held, as `_atomic_lock` leaves it).
    fn set_lock(v: i32) {
        UserPtr::<i32>::new(LOCK).copyout(&v).unwrap();
    }

    /// The abort word set to `v`.
    fn set_abort(v: u32) {
        UserPtr::<u32>::new(ABORT).copyout(&v).unwrap();
    }

    /// The timeout `{sec, nsec}` written; its address.
    fn set_ts(sec: i64, nsec: i64) -> usize {
        UserPtr::<Timespec>::new(TS)
            .copyout(&Timespec::new(sec, nsec))
            .unwrap();
        TS
    }

    /// `__thrsleep(ident, clock_id, tp, lock, abort)` by `p`: its value.
    fn thrsleep_call(
        p: &T,
        ident: isize,
        clock_id: i32,
        tp: usize,
        lock: usize,
        abort: usize,
    ) -> i32 {
        sys___thrsleep::<G>(p, ident, clock_id, tp, lock, abort)
    }

    /// `__thrwakeup(ident, n)` by `p`: its value.
    fn thrwakeup_call(p: &T, ident: isize, n: i32) -> i32 {
        sys___thrwakeup::<G>(p, ident, n)
    }

    /// An errno as the calls answer it.
    fn errno(e: Errno) -> i32 {
        e.as_i32()
    }

    /// The bucket `thrsleep` queues a sleeper of `ident` on.
    fn bucket_of(ident: isize) -> &'static TslpBucket {
        if ident == -1 {
            &TSB_SHARED
        } else {
            thrsleep_bucket(ident)
        }
    }

    /// How many records are on `tsb`.
    fn len(tsb: &TslpBucket) -> usize {
        let w = tsb.tsb_lock.lock();
        tsb.tsb_list.iter(&w).count()
    }

    /// The process identity a record of `p` carries.
    fn ps_of(p: &T) -> usize {
        ptr::from_ref(p.process()).addr()
    }

    /// A record made by hand (its thread never asleep), queued on its bucket as `thrsleep`
    /// queues one.
    fn publish(e: Pin<&Tslpentry>) -> Pin<&Tslpentry> {
        let tsb = bucket_of(e.tslp_ident);
        let mut w = tsb.tsb_lock.lock();
        tsb.tsb_list.insert_tail(&mut w, e);
        drop(w);
        e
    }

    /// A thread of a process of its own.
    fn stranger() -> &'static T {
        let other: &'static Process<G> = Box::leak(Box::new(Process::new(3, ())));
        Box::leak(Box::new(Thread::new_static(other, ())))
    }

    /// Whether `t` is asleep in `__thrsleep`: queued on its channel and out of
    /// `sleep_finish`'s window.
    fn asleep_in_thrsleep(t: Pin<&'static T>) -> bool {
        t.stat() == SSLEEP
            && t.p_wmesg.get() == Some("thrsleep")
            && t.p_flag.load(Ordering::Relaxed) & P_INSCHED == 0
    }

    /// Waits until `t` sleeps in `__thrsleep`.
    fn until_asleep(t: Pin<&'static T>) {
        while !asleep_in_thrsleep(t) {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn the_bucket_of_an_ident_is_the_c_s_fold() {
        // ident >>= 3; ident ^= ident >> 6; ident &= 63 (on the signed long).
        for (ident, want) in [
            (8isize, 1usize),
            (0x1f8, 63),
            (0x200, 1),
            (0x1000, 8),
            (-8, 0),
            (-0x1000, 56),
            (isize::MAX, 0),
            (isize::MIN, 0),
        ] {
            assert!(
                ptr::eq(thrsleep_bucket(ident), &TSB_BUCKETS[want]),
                "ident {ident:#x}"
            );
        }
        // Bits above the fifteenth are not hashed: two idents may share a bucket.
        assert!(ptr::eq(
            thrsleep_bucket(0x4000),
            thrsleep_bucket(0x4000 + (1 << 15))
        ));
        assert_eq!(core::mem::align_of::<TslpBucket>(), 64);
    }

    #[test]
    fn ident_zero_is_einval_before_any_unlock() {
        let _g = setup();
        let p = thread(180, 50);
        set_lock(1);
        assert_eq!(
            thrsleep_call(&p, 0, CLOCK_MONOTONIC, 0, LOCK, 0),
            errno(Errno::EINVAL)
        );
        // A timeout already passed does not get it unlocked either.
        let past = set_ts(0, 0);
        assert_eq!(
            thrsleep_call(&p, 0, CLOCK_MONOTONIC, past, LOCK, 0),
            errno(Errno::EINVAL)
        );
        assert_eq!(lock_word(), 1);
        assert_eq!(thrwakeup_call(&p, 0, 1), errno(Errno::EINVAL));
        // The timeout is copied in first: an unreadable one is EFAULT, even for ident 0.
        assert_eq!(
            thrsleep_call(&p, 0, CLOCK_MONOTONIC, UNMAPPED, LOCK, 0),
            errno(Errno::EFAULT)
        );
        assert_eq!(lock_word(), 1);
    }

    #[test]
    fn a_bad_timeout_or_clock_is_einval_and_unlocks_nothing() {
        let _g = setup();
        let p = thread(181, 50);
        set_lock(1);
        for (sec, nsec) in [(0i64, 1_000_000_000i64), (0, -1), (i64::MAX, -1)] {
            let tp = set_ts(sec, nsec);
            assert_eq!(
                thrsleep_call(&p, 0x1000, CLOCK_MONOTONIC, tp, LOCK, 0),
                errno(Errno::EINVAL),
                "{sec}.{nsec}"
            );
        }
        // A clock clock_gettime refuses, with a timeout to read it for.
        let tp = set_ts(i64::MAX, 0);
        assert_eq!(
            thrsleep_call(&p, 0x1000, 99, tp, LOCK, 0),
            errno(Errno::EINVAL)
        );
        assert_eq!(lock_word(), 1);
        assert_eq!(len(thrsleep_bucket(0x1000)), 0);
        // Without a timeout the clock is never read: the call goes on (to the abort word).
        set_abort(1);
        assert_eq!(
            thrsleep_call(&p, 0x1000, 99, 0, LOCK, ABORT),
            errno(Errno::EINTR)
        );
        assert_eq!(lock_word(), 0);
        assert_eq!(len(thrsleep_bucket(0x1000)), 0);
    }

    #[test]
    fn a_timeout_already_passed_unlocks_then_would_block() {
        let _g = setup();
        let p = thread(182, 50);
        // The monotonic clock's epoch, and a second before it (a negative tv_sec is valid).
        for (ident, sec) in [(0x1000isize, 0i64), (-1, -1)] {
            set_lock(1);
            let tp = set_ts(sec, 0);
            assert_eq!(
                thrsleep_call(&p, ident, CLOCK_MONOTONIC, tp, LOCK, 0),
                errno(Errno::EWOULDBLOCK),
                "ident {ident}"
            );
            assert_eq!(lock_word(), 0, "ident {ident}");
            assert_eq!(len(bucket_of(ident)), 0, "ident {ident}");
        }
        // The unlock's fault is the answer.
        let tp = set_ts(0, 0);
        assert_eq!(
            thrsleep_call(&p, 0x1000, CLOCK_MONOTONIC, tp, UNMAPPED, 0),
            errno(Errno::EFAULT)
        );
        // No lock to free.
        assert_eq!(
            thrsleep_call(&p, 0x1000, CLOCK_MONOTONIC, tp, 0, 0),
            errno(Errno::EWOULDBLOCK)
        );
    }

    #[test]
    fn the_abort_word_is_read_after_the_unlock() {
        let _g = setup();
        let p = thread(183, 50);
        set_lock(1);
        set_abort(1);
        assert_eq!(
            thrsleep_call(&p, 0x2000, CLOCK_MONOTONIC, 0, LOCK, ABORT),
            errno(Errno::EINTR)
        );
        assert_eq!(lock_word(), 0);
        assert_eq!(len(thrsleep_bucket(0x2000)), 0);
        assert_eq!(thrwakeup_call(&p, 0x2000, 1), errno(Errno::ESRCH));
        // A misaligned abort word faults, after the lock was freed; the record is gone.
        set_lock(1);
        assert_eq!(
            thrsleep_call(&p, -1, CLOCK_MONOTONIC, 0, LOCK, ABORT + 1),
            errno(Errno::EFAULT)
        );
        assert_eq!(lock_word(), 0);
        assert_eq!(len(&TSB_SHARED), 0);
        // An unmapped lock: its fault, the record gone, the abort word never read (it is set
        // and readable, so a read would answer EINTR).
        set_abort(1);
        assert_eq!(
            thrsleep_call(&p, 0x2000, CLOCK_MONOTONIC, 0, UNMAPPED, ABORT),
            errno(Errno::EFAULT)
        );
        assert_eq!(len(thrsleep_bucket(0x2000)), 0);
    }

    #[test]
    fn a_wakeup_without_sleepers_is_esrch_but_the_broadcast_s_is_zero() {
        let _g = setup();
        let p = thread(184, 50);
        for n in [1, 0, -1] {
            assert_eq!(thrwakeup_call(&p, 0x3000, n), errno(Errno::ESRCH), "n {n}");
        }
        assert_eq!(thrwakeup_call(&p, -1, 1), 0);
        assert_eq!(thrwakeup_call(&p, -1, 0), 0);
    }

    #[test]
    fn a_wakeup_takes_n_of_its_process_s_sleepers_on_the_ident_in_order() {
        let _g = setup();
        let p = thread(185, 50);
        let q = stranger();
        let ident: isize = 0x4000;
        // Same bucket, another ident.
        let alias: isize = ident + (1 << 15);
        assert!(ptr::eq(thrsleep_bucket(ident), thrsleep_bucket(alias)));
        let tsb = thrsleep_bucket(ident);

        let s1 = pin!(Tslpentry::new(ps_of(&p), ident));
        let s1 = publish(s1.into_ref());
        let foreign = pin!(Tslpentry::new(ps_of(q), ident));
        let foreign = publish(foreign.into_ref());
        let s2 = pin!(Tslpentry::new(ps_of(&p), ident));
        let s2 = publish(s2.into_ref());
        let aliased = pin!(Tslpentry::new(ps_of(&p), alias));
        let aliased = publish(aliased.into_ref());
        let s3 = pin!(Tslpentry::new(ps_of(&p), ident));
        let s3 = publish(s3.into_ref());
        assert_eq!(len(tsb), 5);

        // At most two, in queue order.
        assert_eq!(thrwakeup_call(&p, ident, 2), 0);
        assert!(!s1.waiting() && !s2.waiting() && s3.waiting());
        assert_eq!(len(tsb), 3);
        // n == 0 takes every match left.
        assert_eq!(thrwakeup_call(&p, ident, 0), 0);
        assert!(!s3.waiting());
        assert!(foreign.waiting() && aliased.waiting());
        assert_eq!(thrwakeup_call(&p, ident, 1), errno(Errno::ESRCH));

        // n < 0 too.
        let t1 = pin!(Tslpentry::new(ps_of(&p), ident));
        let t1 = publish(t1.into_ref());
        let t2 = pin!(Tslpentry::new(ps_of(&p), ident));
        let t2 = publish(t2.into_ref());
        assert_eq!(thrwakeup_call(&p, ident, -1), 0);
        assert!(!t1.waiting() && !t2.waiting());

        // The other process wakes its own, and the alias its own.
        assert_eq!(thrwakeup_call(q, ident, 5), 0);
        assert!(!foreign.waiting());
        assert_eq!(thrwakeup_call(&p, alias, 1), 0);
        assert!(!aliased.waiting());
        assert_eq!(len(tsb), 0);
    }

    #[test]
    fn the_broadcast_wakes_every_process_whatever_n() {
        let _g = setup();
        let p = thread(186, 50);
        let q = stranger();
        let a = pin!(Tslpentry::new(ps_of(&p), -1));
        let a = publish(a.into_ref());
        let b = pin!(Tslpentry::new(ps_of(q), -1));
        let b = publish(b.into_ref());
        let c = pin!(Tslpentry::new(ps_of(&p), -1));
        let c = publish(c.into_ref());
        assert_eq!(len(&TSB_SHARED), 3);
        assert_eq!(thrwakeup_call(&p, -1, 1), 0);
        assert!(!a.waiting() && !b.waiting() && !c.waiting());
        assert_eq!(len(&TSB_SHARED), 0);
    }

    #[test]
    fn a_wakeup_wins_over_a_racing_timeout_or_signal() {
        let _g = setup();
        let p = thread(187, 50);
        let ident: isize = 0x5000;
        let tsb = thrsleep_bucket(ident);

        // A waker took the record before its sleeper looked: the sleep's error is forgotten.
        for error in [
            Err(Errno::EWOULDBLOCK),
            Err(Errno::EINTR),
            Err(Errno::ERESTART),
            Ok(()),
        ] {
            let e = pin!(Tslpentry::new(ps_of(&p), ident));
            let e = publish(e.into_ref());
            assert_eq!(thrwakeup_call(&p, ident, 1), 0);
            assert_eq!(thrsleep_settle(tsb, &e, error), Ok(()), "{error:?}");
        }

        // Still queued: the sleeper takes itself off and keeps its error, a restart
        // cancelled; a sleep that ended without one (a spurious wakeup) answers 0.
        for (error, want) in [
            (Err(Errno::EWOULDBLOCK), Err(Errno::EWOULDBLOCK)),
            (Err(Errno::EINTR), Err(Errno::EINTR)),
            (Err(Errno::ERESTART), Err(Errno::ECANCELED)),
            (Ok(()), Ok(())),
        ] {
            let e = pin!(Tslpentry::new(ps_of(&p), ident));
            let e = publish(e.into_ref());
            assert_eq!(thrsleep_settle(tsb, &e, error), want, "{error:?}");
            assert!(!e.waiting());
            assert_eq!(len(tsb), 0);
        }
        assert_eq!(thrwakeup_call(&p, ident, 1), errno(Errno::ESRCH));
    }

    #[test]
    fn a_sleeper_is_woken_by_its_sibling_one_at_a_time() {
        let _g = setup();
        let [a, b] = VCPUS;
        let me = thread(188, 50);
        run_as(a, me);
        let ident: isize = 0x6000;
        const ROUNDS: usize = 4;
        let waker = std::thread::spawn(move || {
            let w = thread(189, 50);
            run_as(b, w);
            for _ in 0..ROUNDS {
                until_asleep(me);
                // Queued, then the lock freed, then asleep.
                assert_eq!(lock_word(), 0);
                assert_eq!(thrwakeup_call(&w, ident, 1), 0);
            }
            hal::thread::set_current_static::<T>(None);
        });
        for _ in 0..ROUNDS {
            set_lock(1);
            assert_eq!(thrsleep_call(&me, ident, CLOCK_MONOTONIC, 0, LOCK, 0), 0);
            assert_eq!(me.p_wchan.get(), 0);
        }
        waker.join().unwrap();
        assert_eq!(len(thrsleep_bucket(ident)), 0);
        assert_eq!(thrwakeup_call(&me, ident, 1), errno(Errno::ESRCH));
        hal::thread::set_current_static::<T>(None);
    }

    #[test]
    fn a_timed_sleep_answers_ewouldblock() {
        let _g = setup();
        let [a, b] = VCPUS;
        let me = thread(190, 50);
        crate::sched::sleep_timeout_init(me.get_ref());
        run_as(a, me);
        {
            hal::klock::kernel_lock();
            let mut w = hal::klock::witness();
            TABLES.tidhash(190).insert_head(&mut w, me);
            drop(w);
            hal::klock::kernel_unlock();
        }
        let now = clock_gettime(me.get_ref(), CLOCK_MONOTONIC).unwrap();
        let tp = set_ts(now.tv_sec + 3600, now.tv_nsec);
        set_lock(1);
        let timer = std::thread::spawn(move || {
            let w = thread(191, 50);
            run_as(b, w);
            until_asleep(me);
            // The softclock thread took the timeout off the wheel and runs its handler.
            assert!(hal::pin::project(me, |t| &t.p_sleep_to).del());
            hal::klock::kernel_lock();
            crate::sched::endtsleep::<G>(ptr::without_provenance_mut(190));
            hal::klock::kernel_unlock();
            hal::thread::set_current_static::<T>(None);
        });
        // A timeout is EWOULDBLOCK here (futex(2) answers ETIMEDOUT).
        assert_eq!(
            thrsleep_call(&me, 0x7000, CLOCK_MONOTONIC, tp, LOCK, 0),
            errno(Errno::EWOULDBLOCK)
        );
        timer.join().unwrap();
        assert_eq!(lock_word(), 0);
        // It took itself off its bucket.
        assert_eq!(len(thrsleep_bucket(0x7000)), 0);
        assert_eq!(thrwakeup_call(&me, 0x7000, 1), errno(Errno::ESRCH));
        hal::klock::kernel_lock();
        let mut w = hal::klock::witness();
        TABLES.tidhash(190).remove(&mut w, me.get_ref());
        drop(w);
        hal::klock::kernel_unlock();
        hal::thread::set_current_static::<T>(None);
    }
}
/* </TESTS> */
