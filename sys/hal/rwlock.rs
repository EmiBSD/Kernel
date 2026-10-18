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
 * Copyright (c) 2002, 2003 Artur Grabowski <art@openbsd.org>
 * Copyright (c) 2011 Thordur Bjornsson <thib@secnorth.net>
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

/*-
 * Copyright (c) 2008 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Andrew Doran.
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

/*
 * Copyright (c) 2002 Artur Grabowski <art@openbsd.org>
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
//! `Rwlocked<T>`: the sleeping reader-writer lock of `rwlock(9)` that owns its data;
//! `SharedRwlock`: the counted lock object of `rw_obj_alloc` that several objects share, its
//! witnesses, and `Guarded<T>`, a cell of one object under a lock another object may own.
//!
//! Upstream: sys/kern/kern_rwlock.c @ 3ce1f3f79392
//! Upstream: sys/sys/rwlock.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_synch.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_rwlock.rs@3c62ede68802
//! LZ: sys/sys/rwlock.rs@3c62ede68802
//! LZ: sys/kern/kern_synch.rs@3c62ede68802
//!
//! The behaviour of `rw_enter(9)`, the C's word for word where userland or a sleeper could
//! tell: one owner word, `[ writer or reader count | WRLOCK ]`; a writer is the running
//! process (`rw_self`), so the lock belongs to a process, which may sleep holding it. A
//! writer takes a free word; under `MULTIPROCESSOR`, unless it holds the kernel lock, it
//! spins `RW_SPINS` times for a writer to leave; then (unless `RW_NOSLEEP`: `EBUSY`) it
//! counts itself in `rwl_waiters` and sleeps on that counter's address at `PLOCK - 4`,
//! retrying after every wakeup. A reader adds itself to a word with no writer, but only while
//! no writer waits (writers are not starved by a stream of readers), else counts itself in
//! `rwl_readers` and sleeps on that address at `PLOCK`. `RW_INTR` adds `PCATCH`: a signal ends
//! the wait with the sleep's error (`EINTR`, `ERESTART`). The holder that leaves the word
//! free wakes one waiting writer, or else every waiting reader. Not recursive: a writer
//! entering again, or reading
//! what it writes, panics ("enter write deadlock", "enter read deadlock"). A writer may
//! downgrade (waking the readers when no writer waits) and the only reader may try to
//! upgrade (`EBUSY` otherwise). `rwsleep(9)` releases the lock for the sleep and takes it
//! again, in the same mode, before returning, whatever the sleep returned.
//!
//! The sleep queues and `curproc` arrive through the `Sleep` bridge (`sleep.rs`). Before it is
//! registered nothing else runs, so a lock found held then could never be released: it is
//! `panic(9)` naming the lock (the reviewer's finding 7; spinning would hang the boot
//! silently). A guard's sleep then releases the lock, lets the CPU breathe and takes it back.
//!
//! The lock objects (`rw_obj_init`, `rw_obj_alloc`, `rw_obj_hold`, `rw_obj_free`): one pool of
//! counted locks, `"rwobjpl"` at `IPL_MPFLOOR`, `PR_WAITOK`; an allocation is one reference,
//! each hold one more, and the free that drops the last puts the lock back. uvm shares one
//! between an amap, its anons and its copies, and gives one to each object (`uvm_amap.c`,
//! `uvm_anon.c`, `uvm_object.c`).
//!
//! ## Deviations
//! - `WITNESS`, `RWDIAG` and the `dt(4)` tracepoints (`TRACEINDEX`, `rwl_traceidx`) are not
//!   configured, as in LZ.
//! - `rw_enter` with `RW_DOWNGRADE`/`RW_UPGRADE` are [`WriteGuard::downgrade`] and
//!   [`ReadGuard::try_upgrade`] (and the witnesses' of the same names); `PNORELOCK` exists
//!   only as a witness's [`WriteWitness::into_sleep`], which consumes the witness (a guard of
//!   a `Rwlocked` always takes its lock back: it would outlive the lock it proves); the
//!   `rw_assert_*` checks are the guards' and the witnesses' types.
//! - The lock objects have no `ro_magic`: a lock is reached only through its handle
//!   ([`SharedRwlock`]), never cast back from a `struct rwlock *`, so the `KASSERTMSG`s that
//!   check the magic and the count of a pointer have nothing to check. Their pool item holds
//!   the lock, the handle's count and its pool (48 bytes: 8 + 8 + the 32 of `RawRwlock`,
//!   where the C's `struct rwlock_obj` has 40; the user's decision D2 of N3.4, as `pipepl`).
//! - `rwsleep_nsec`'s assertion that a sleep on `nowake` has `PCATCH` or a timeout has nothing
//!   to check here: `hal` offers no `nowake` channel (a guard sleeps on a channel its caller
//!   names, which something wakes).
//! - The structure is not the C's: one `wait` serves writers and readers (counted in their
//!   counter, asleep on its address, retrying after each wakeup), and the tries at the word are
//!   small functions. The C is the specification of the behaviour, not of the shape.
//!
//! ## Redesign
//! - LZ's `Rwlock` is the C's `struct rwlock` with free functions (`rw_enter_write(&l)`, the
//!   protected fields `Cell`s or raw pointers beside it, `rw_assert_wrlock` where the C
//!   asserts). `Rwlocked<T>` holds the data: [`Rwlocked::write`] and [`Rwlocked::read`] return
//!   guards through which alone it is reached (`&mut T` for the one writer, `&T` for the
//!   readers), and the guards' drops are `rw_exit_write`/`rw_exit_read`. Not `Send`: the lock
//!   belongs to the process that took it.
//! - `rwsleep_nsec` is `kern_synch.c`'s (hence the third source; unlike `msleep_nsec` it has
//!   no `cold || panicstr` early return, nor has `RawRwlock::sleep_released`).
//!   `rwsleep_nsec(ident, &l, prio, wmesg, nsecs)` is [`WriteGuard::sleep_nsec`] (and
//!   [`ReadGuard::sleep_nsec`]): `&mut self`, so no reference into the data lives across the
//!   sleep, and the guard is valid again when it returns, `Ok` or `Err` (decision 11's
//!   pipe needs exactly this). [`WriteGuard::unlocked`] runs a closure with the lock released
//!   (a copy that may fault and sleep, while the buffer it copies is owned outside the lock).
//! - A write guard keeps the owner word it stored, so its exit compares with that word and
//!   not with a fresh `rw_self`: the exclusion never depends on the `Sleep` bridge's
//!   `curproc` (a wrong answer costs the deadlock diagnostics only); every word a writer
//!   stores has `WRLOCK` set, so it is never the free 0.
//! - The legacy `Rwlock` (72 files) keeps its API (the timing rule; decision 8 names the
//!   owning type `Rwlocked` beside it, the rename is N9's).
//! - N3.4a: LZ's `rw_obj_alloc_flags(name, flags) -> &'static Rwlock`, `rw_obj_hold(&l)` and
//!   `rw_obj_free(&'static l) -> bool` (the lock found back from its pointer by `offset_of`
//!   and a magic number) are [`SharedRwlock`]: a counted handle, `enum { Static(&'static
//!   RawRwlock), Pooled(PoolArc<RawRwlock>) }`; `SharedRwlock::new(name)` is the
//!   allocation, `Clone` the hold, the drop of the last handle the free; the static arm is a
//!   lock that is never freed (`uvm_aobj.c`'s `bootstrap_kernel_object_lock`), taken by
//!   [`SharedRwlock::from_static`]. The lock owns no data (several objects keep their
//!   fields under it): [`SharedRwlock::write`] and [`SharedRwlock::read`] return a
//!   [`WriteWitness`] or a [`ReadWitness`], guards with no data whose existence is the proof
//!   that the lock is held; their sleeps are `rwsleep_nsec`'s, and
//!   [`WriteWitness::into_sleep`] / [`ReadWitness::into_sleep`] are `PNORELOCK`: the witness
//!   is consumed, so nothing can use it after the lock was released (`uvm_pagewait`).
//! - N3.4a: what the C keeps "locked by the amap's lock" in another object (`an_ref`,
//!   the amap's chunk tables, the aobj's swap slots) sits in a [`Guarded<T>`]: a cell keyed
//!   to one lock's identity that opens only through a witness of that lock, checked at every
//!   access (a witness of another lock is `panic(9)`). [`Witness`] is sealed: only the two
//!   witnesses are witnesses. The C's `KASSERT(anon->an_lock == amap->am_lock)` becomes the
//!   check that makes the access sound.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::assert::{hal_panic, kassert};
use crate::bridges;
use crate::pool::{PR_WAITOK, PoolArc, TypedPool};
use crate::sleep::{self, INFSLP, PCATCH, PLOCK, PRIMASK, chan};
use crate::sys::errno::Errno;

/// `RWLOCK_WRLOCK`: the word holds a writer.
const RWLOCK_WRLOCK: usize = 0x04;
/// `RWLOCK_MASK`: the flag bits below the writer or the reader count.
const RWLOCK_MASK: usize = 0x07;
/// `RWLOCK_READ_INCR`: one reader.
const RWLOCK_READ_INCR: usize = 1 << 3;

/// `RW_INTR`: the wait for the lock ends on a signal, with the sleep's error.
pub const RW_INTR: i32 = 0x0010;
/// `RW_NOSLEEP`: `EBUSY` instead of waiting for the lock.
pub const RW_NOSLEEP: i32 = 0x0040;

/// `RW_WRITE`: [`Rwlocked::status`] of a lock the running process writes.
pub const RW_WRITE: i32 = 0x0001;
/// `RW_READ`: [`Rwlocked::status`] of a lock held by readers.
pub const RW_READ: i32 = 0x0002;
/// `RW_WRITE_OTHER`: [`Rwlocked::status`] of a lock another process writes.
pub const RW_WRITE_OTHER: i32 = 0x0100;

/// `RW_SPINS`: how many times a writer polls the word before it sleeps, `MULTIPROCESSOR`.
const RW_SPINS: u32 = 1000;

/// `struct rwlock`: the owner word and the two wait counters whose addresses are the wait
/// channels. Inside a [`Rwlocked`] or a [`SharedRwlock`]'s pool item; a `static` of its own
/// only for [`SharedRwlock::from_static`], since it is taken and released only through
/// those.
pub struct RawRwlock {
    /// `rwl_owner`.
    owner: AtomicUsize,
    /// `rwl_waiters`: writers waiting.
    waiters: AtomicU32,
    /// `rwl_readers`: readers waiting.
    readers: AtomicU32,
    /// `rwl_name`: the wait message.
    name: &'static str,
}

impl RawRwlock {
    /// `RWLOCK_INITIALIZER(name)`: a free lock whose sleeps say `name`.
    pub const fn new(name: &'static str) -> Self {
        Self {
            owner: AtomicUsize::new(0),
            waiters: AtomicU32::new(0),
            readers: AtomicU32::new(0),
            name,
        }
    }

    /// `rwl_name`: the wait message.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The lock's identity: its address, as a number (two live locks never share one).
    fn id(&self) -> LockId {
        LockId(ptr::from_ref(self).addr())
    }

    /// `rw_cas`: one compare-and-swap of the owner word; the value it found.
    fn cas(&self, expect: usize, new: usize) -> usize {
        match self
            .owner
            .compare_exchange(expect, new, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(found) | Err(found) => found,
        }
    }

    /// `panic(9)` with the C's prefix, `"<name> rwlock <addr>: "`.
    fn panic(&self, what: core::fmt::Arguments<'_>) -> ! {
        hal_panic(format_args!(
            "{} rwlock {:p}: {what}",
            self.name,
            ptr::from_ref(self)
        ))
    }

    /// A writer's one try: the free word becomes `me`.
    fn try_write(&self, me: usize) -> bool {
        self.cas(0, me) == 0
    }

    /// A reader's try (`rw_read_incr`): one more reader on a word with no writer, starting from
    /// the word last seen and following it while other readers move it; `false` as soon as a
    /// writer holds it.
    fn try_read_from(&self, seen: usize) -> bool {
        let mut seen = seen;
        loop {
            if seen & RWLOCK_WRLOCK != 0 {
                return false;
            }
            let found = self.cas(seen, seen + RWLOCK_READ_INCR);
            if found == seen {
                return true;
            }
            seen = found;
        }
    }

    /// Under `MULTIPROCESSOR`, a writer that does not hold the kernel lock polls the word
    /// `RW_SPINS` times before it sleeps, in case the holder is a writer about to leave; one
    /// that holds it gives the CPU up at once, so the CPUs waiting for the kernel lock run.
    fn spin_for_writer(&self, me: usize) -> bool {
        if sleep::kernel_lock_held() {
            return false;
        }
        crate::cpu::spinning(1);
        let won = (0..RW_SPINS).any(|_| {
            core::hint::spin_loop(); // CPU_BUSY_CYCLE()
            self.owner.load(Ordering::Relaxed) == 0 && self.try_write(me)
        });
        crate::cpu::spinning(-1);
        won
    }

    /// The wait shared by writers and readers: counted in `counter` (`rwl_waiters` or
    /// `rwl_readers`), asleep on its address at `prio` while `busy()` says the lock is still
    /// out of reach (checked once queued, so a release in between is not missed), trying
    /// `take()` after each wakeup. Under `RW_INTR` a signal ends the wait with its error.
    /// Before the sleep queues are registered nothing could ever release the lock: that is a
    /// kernel bug, `panic(9)` naming the lock.
    fn wait(
        &self,
        counter: &AtomicU32,
        prio: i32,
        flags: i32,
        busy: impl Fn() -> bool,
        take: impl Fn() -> bool,
    ) -> Result<(), Errno> {
        let Some(b) = bridges::sleep() else {
            self.panic(format_args!("contended before the sleep queues exist"));
        };
        let prio = if flags & RW_INTR != 0 {
            prio | PCATCH
        } else {
            prio
        };
        counter.fetch_add(1, Ordering::SeqCst);
        let outcome = loop {
            b.sleep_setup(chan(ptr::from_ref(counter)), prio, self.name);
            let slept = b.sleep_finish(INFSLP, busy());
            if flags & RW_INTR != 0
                && let Err(e) = slept
            {
                break Err(e);
            }
            if take() {
                break Ok(());
            }
        };
        counter.fetch_sub(1, Ordering::SeqCst);
        outcome
    }

    /// `rw_do_enter_write`: the word stored (this process as a writer).
    fn enter_write(&self, flags: i32) -> Result<usize, Errno> {
        let me = rw_self();
        let seen = self.cas(0, me);
        if seen == 0 {
            return Ok(me);
        }
        if seen == me {
            self.panic(format_args!("enter write deadlock"));
        }
        if self.spin_for_writer(me) {
            return Ok(me);
        }
        if flags & RW_NOSLEEP != 0 {
            return Err(Errno::EBUSY);
        }
        self.wait(
            &self.waiters,
            PLOCK - 4,
            flags,
            || self.owner.load(Ordering::SeqCst) != 0,
            || self.try_write(me),
        )?;
        Ok(me)
    }

    /// `rw_enter_write` without flags, which cannot fail.
    fn write_wait(&self) -> usize {
        match self.enter_write(0) {
            Ok(me) => me,
            Err(e) => self.panic(format_args!("uninterruptible enter failed ({e:?})")),
        }
    }

    /// `rw_do_enter_read`. A reader joins other readers only while no writer waits, so a
    /// stream of readers cannot starve a writer.
    fn enter_read(&self, flags: i32) -> Result<(), Errno> {
        let seen = self.cas(0, RWLOCK_READ_INCR);
        if seen == 0 {
            return Ok(());
        }
        if seen & RWLOCK_WRLOCK != 0 {
            if seen == rw_self() {
                self.panic(format_args!("enter read deadlock"));
            }
        } else if self.waiters.load(Ordering::SeqCst) == 0 && self.try_read_from(seen) {
            return Ok(());
        }
        if flags & RW_NOSLEEP != 0 {
            return Err(Errno::EBUSY);
        }
        self.wait(
            &self.readers,
            PLOCK,
            flags,
            || {
                self.waiters.load(Ordering::SeqCst) > 0
                    || self.owner.load(Ordering::SeqCst) & RWLOCK_WRLOCK != 0
            },
            || self.try_read_from(0),
        )
    }

    /// `rw_enter_read` without flags, which cannot fail.
    fn read_wait(&self) {
        if let Err(e) = self.enter_read(0) {
            self.panic(format_args!("uninterruptible enter failed ({e:?})"));
        }
    }

    /// `rw_exit_read`: one reader fewer; the one that leaves the word free wakes the next.
    fn exit_read(&self) {
        let mut seen = RWLOCK_READ_INCR;
        loop {
            if seen & RWLOCK_WRLOCK != 0 {
                self.panic(format_args!(
                    "exit read on write locked lock (owner {seen:#x})"
                ));
            }
            if seen == 0 {
                self.panic(format_args!("exit read on unlocked lock"));
            }
            let found = self.cas(seen, seen - RWLOCK_READ_INCR);
            if found == seen {
                if seen == RWLOCK_READ_INCR {
                    self.wake_next();
                }
                return;
            }
            seen = found;
        }
    }

    /// `rw_exit_write`, for the writer that stored `me`.
    fn exit_write(&self, me: usize) {
        let found = self.cas(me, 0);
        if found != me {
            self.panic(format_args!(
                "exit write when lock not held (owner {found:#x}, self {me:#x})"
            ));
        }
        self.wake_next();
    }

    /// `rw_exited`: a free word goes to one waiting writer if there is one, else to every
    /// waiting reader.
    fn wake_next(&self) {
        if self.waiters.load(Ordering::SeqCst) > 0 {
            sleep::wakeup_one(&raw const self.waiters);
        } else if self.readers.load(Ordering::SeqCst) > 0 {
            sleep::wakeup(&raw const self.readers);
        }
    }

    /// `rw_downgrade`, for the writer that stored `me`: the word becomes one reader, and the
    /// readers waiting join it unless a writer waits too.
    fn downgrade(&self, me: usize) {
        let found = self.cas(me, RWLOCK_READ_INCR);
        if found != me {
            self.panic(format_args!(
                "downgrade when lock not held (owner {found:#x}, self {me:#x})"
            ));
        }
        if self.waiters.load(Ordering::SeqCst) == 0 && self.readers.load(Ordering::SeqCst) > 0 {
            sleep::wakeup(&raw const self.readers);
        }
    }

    /// `rw_upgrade` with `RW_NOSLEEP`, by a reader: the word stored, or `EBUSY` while other
    /// readers hold it.
    fn upgrade(&self) -> Result<usize, Errno> {
        let me = rw_self();
        let found = self.cas(RWLOCK_READ_INCR, me);
        if found == RWLOCK_READ_INCR {
            return Ok(me);
        }
        if found == 0 {
            self.panic(format_args!("upgrade on unowned lock"));
        }
        if found & RWLOCK_WRLOCK != 0 {
            self.panic(format_args!(
                "upgrade on write locked lock(owner {found:#x}, self {me:#x})"
            ));
        }
        Err(Errno::EBUSY)
    }

    /// `rw_status`.
    fn status(&self) -> i32 {
        let owner = self.owner.load(Ordering::SeqCst);
        match owner {
            0 => 0,
            w if w & RWLOCK_WRLOCK == 0 => RW_READ,
            w if w == rw_self() => RW_WRITE,
            _ => RW_WRITE_OTHER,
        }
    }

    /// The `rwsleep_nsec` checks: the priority bits and a channel that is not the lock.
    fn check_sleep<I: ?Sized>(&self, ident: *const I, prio: i32) {
        kassert!(prio & !(PRIMASK | PCATCH) == 0);
        kassert!(!ptr::eq(chan(ident), chan(ptr::from_ref(self))));
        let _ = (ident, prio);
    }

    /// `rwsleep_nsec` up to the sleep's end, the lock not taken again: queued on `ident`
    /// while the lock is still held (so the wakeup that follows a change under the lock
    /// cannot be missed), released by `exit` before sleeping (so a stop signal taken while
    /// asleep does not hold it), asleep; the sleep's result. With no sleep queues yet there is
    /// no one to wait for: released, the CPU let breathe (the C's `tsleep` while `cold`), `Ok`.
    fn sleep_released<I: ?Sized>(
        &self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
        exit: impl FnOnce(),
    ) -> Result<(), Errno> {
        self.check_sleep(ident, prio);
        match bridges::sleep() {
            Some(b) => {
                b.sleep_setup(chan(ident), prio, wmesg);
                exit();
                b.sleep_finish(nsecs, true)
            }
            None => {
                exit();
                core::hint::spin_loop();
                Ok(())
            }
        }
    }
}

/// A reader-writer lock that owns the `T` it protects (`struct rwlock` with its data).
///
/// Invariant: `data` is reached only through the guards; a [`WriteGuard`] exists only while
/// the owner word holds the writer word it stored (no reader, no other writer), a
/// [`ReadGuard`] only while the word counts it among the readers (no writer). So either one
/// `&mut T` or any number of `&T` is alive, never both.
pub struct Rwlocked<T> {
    lock: RawRwlock,
    data: UnsafeCell<T>,
}

impl<T> Rwlocked<T> {
    /// `RWLOCK_INITIALIZER(name)` around `value`: a free lock whose sleeps say `name`.
    pub const fn new(name: &'static str, value: T) -> Self {
        Self {
            lock: RawRwlock::new(name),
            data: UnsafeCell::new(value),
        }
    }

    /// `rw_enter_write(9)`: waits (sleeping, uninterruptibly) until the lock is free and
    /// takes it alone.
    pub fn write(&self) -> WriteGuard<'_, T> {
        let me = self.lock.write_wait();
        WriteGuard::new(self, me)
    }

    /// `rw_enter(9)` with `RW_WRITE | flags` (`RW_NOSLEEP`, `RW_INTR`): `EBUSY` when the lock
    /// is held and `RW_NOSLEEP` is set; the sleep's error when a signal ends an `RW_INTR`
    /// wait.
    pub fn write_flags(&self, flags: i32) -> Result<WriteGuard<'_, T>, Errno> {
        let me = self.lock.enter_write(flags)?;
        Ok(WriteGuard::new(self, me))
    }

    /// `rw_enter(9)` with `RW_WRITE | RW_NOSLEEP`: `None` when the lock is held.
    pub fn try_write(&self) -> Option<WriteGuard<'_, T>> {
        self.write_flags(RW_NOSLEEP).ok()
    }

    /// `rw_enter_read(9)`: waits (sleeping, uninterruptibly) until no writer holds or waits
    /// for the lock and takes it shared.
    pub fn read(&self) -> ReadGuard<'_, T> {
        self.lock.read_wait();
        ReadGuard::new(self)
    }

    /// `rw_enter(9)` with `RW_READ | flags` (`RW_NOSLEEP`, `RW_INTR`), as
    /// [`Rwlocked::write_flags`].
    pub fn read_flags(&self, flags: i32) -> Result<ReadGuard<'_, T>, Errno> {
        self.lock.enter_read(flags)?;
        Ok(ReadGuard::new(self))
    }

    /// `rw_enter(9)` with `RW_READ | RW_NOSLEEP`: `None` when a writer holds or waits.
    pub fn try_read(&self) -> Option<ReadGuard<'_, T>> {
        self.read_flags(RW_NOSLEEP).ok()
    }

    /// `rw_status(9)`: [`RW_WRITE`] (this process writes), [`RW_WRITE_OTHER`], [`RW_READ`]
    /// or 0 (free). A snapshot, for assertions and diagnostics.
    pub fn status(&self) -> i32 {
        self.lock.status()
    }

    /// `rwl_name`: the wait message.
    pub fn name(&self) -> &'static str {
        self.lock.name
    }

    /// The data, through the exclusive reference that proves no guard is alive.
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// The data, consuming the lock.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

// SAFETY: sharing `&Rwlocked<T>` lets several processes read the `T` at once (`T: Sync`) and
// one of them at a time write it, including moving it out (`T: Send`); the invariant keeps a
// writer and the readers apart.
unsafe impl<T: Send + Sync> Sync for Rwlocked<T> {}

/// The proof of holding a [`Rwlocked`] alone: `&mut T` through `DerefMut`; its drop is
/// `rw_exit_write`. Not `Send`: the lock belongs to the process that took it.
#[must_use = "dropping the guard at once releases the lock"]
pub struct WriteGuard<'a, T> {
    lock: &'a Rwlocked<T>,
    /// The writer word this guard stored (`rw_self` when it entered).
    me: usize,
    _not_send: PhantomData<*const ()>,
}

impl<'a, T> WriteGuard<'a, T> {
    fn new(lock: &'a Rwlocked<T>, me: usize) -> Self {
        Self {
            lock,
            me,
            _not_send: PhantomData,
        }
    }

    /// `rwsleep_nsec(ident, rwl, prio, wmesg, nsecs)` on a write-locked lock: goes on
    /// `ident`'s sleep queue, releases the lock, sleeps, and takes the lock again for writing
    /// (uninterruptibly) before returning the sleep's result: `EWOULDBLOCK` after `nsecs`
    /// ([`INFSLP`]: never), `EINTR`/`ERESTART` on a signal with `PCATCH` in `prio`. `ident`
    /// is a wait channel, never the lock.
    pub fn sleep_nsec<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        let raw = &self.lock.lock;
        let me = self.me;
        let result = raw.sleep_released(ident, prio, wmesg, nsecs, || raw.exit_write(me));
        self.me = raw.write_wait();
        result
    }

    /// `rwsleep_nsec` with no timeout ([`INFSLP`]).
    pub fn sleep<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.sleep_nsec(ident, prio, wmesg, INFSLP)
    }

    /// Runs `f` with the lock released, then takes it again for writing: for work that may
    /// fault or sleep (a copy to or from userland) on data owned outside the lock.
    pub fn unlocked<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let raw = &self.lock.lock;
        raw.exit_write(self.me);
        let r = f();
        self.me = raw.write_wait();
        r
    }

    /// `rw_enter(rwl, RW_DOWNGRADE)`: the writer becomes a reader without letting another
    /// writer in; the readers waiting are woken when no writer waits.
    pub fn downgrade(self) -> ReadGuard<'a, T> {
        let lock = self.lock;
        let me = self.me;
        core::mem::forget(self);
        lock.lock.downgrade(me);
        ReadGuard::new(lock)
    }
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this guard exists, so the owner word holds its writer word: no reader and no
        // other writer, hence no other reference to the data (the type's invariant); the
        // reference borrows the guard, so it ends before the guard releases or sleeps (both
        // take `&mut self` or `self`).
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as for `deref`: the one writer, borrowed mutably, so this is the only
        // reference to the data while it lives.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.exit_write(self.me);
    }
}

/// The proof of holding a [`Rwlocked`] shared: `&T` through `Deref`; its drop is
/// `rw_exit_read`. Not `Send`: the lock belongs to the process that took it.
#[must_use = "dropping the guard at once releases the lock"]
pub struct ReadGuard<'a, T> {
    lock: &'a Rwlocked<T>,
    _not_send: PhantomData<*const ()>,
}

impl<'a, T> ReadGuard<'a, T> {
    fn new(lock: &'a Rwlocked<T>) -> Self {
        Self {
            lock,
            _not_send: PhantomData,
        }
    }

    /// `rwsleep_nsec` on a read-locked lock: as [`WriteGuard::sleep_nsec`], the lock taken
    /// again shared.
    pub fn sleep_nsec<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        let raw = &self.lock.lock;
        let result = raw.sleep_released(ident, prio, wmesg, nsecs, || raw.exit_read());
        raw.read_wait();
        result
    }

    /// `rwsleep_nsec` on a read-locked lock with no timeout.
    pub fn sleep<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.sleep_nsec(ident, prio, wmesg, INFSLP)
    }

    /// `rw_enter(rwl, RW_UPGRADE | RW_NOSLEEP)`: the only reader becomes the writer; while
    /// other readers hold the lock the read guard comes back in the `Err` (the C's `EBUSY`).
    pub fn try_upgrade(self) -> Result<WriteGuard<'a, T>, ReadGuard<'a, T>> {
        match self.lock.lock.upgrade() {
            Ok(me) => {
                let lock = self.lock;
                core::mem::forget(self);
                Ok(WriteGuard::new(lock, me))
            }
            Err(_) => Err(self),
        }
    }
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this guard exists, so the owner word counts it among the readers and no
        // writer holds the lock (the type's invariant): every reference to the data alive is
        // shared; this one borrows the guard and ends before the guard releases or sleeps.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.exit_read();
    }
}

/// A counted lock object (`struct rwlock_obj`, `rw_obj_alloc`): a reader-writer lock that owns
/// no data, shared by the objects that keep fields under it (an amap, its anons and its
/// copies; a uvm object). [`SharedRwlock::write`] and [`SharedRwlock::read`] return a
/// witness, whose existence is the proof of the lock; the fields open through it
/// ([`Guarded`], or atomics written by setters that take it).
///
/// `Clone` is `rw_obj_hold`; the drop of the last handle of a pooled lock is `rw_obj_free`'s
/// `pool_put`. A witness borrows the handle it came from, so the lock outlives every witness.
#[derive(Clone)]
pub struct SharedRwlock(Shared);

/// Where a [`SharedRwlock`]'s lock lives.
#[derive(Clone)]
enum Shared {
    /// A lock that is never freed (`bootstrap_kernel_object_lock`).
    Static(&'static RawRwlock),
    /// An item of `"rwobjpl"`, freed with its last handle.
    Pooled(PoolArc<RawRwlock>),
}

impl SharedRwlock {
    /// `rw_obj_alloc(&lock, name)`: a new lock object, free, counted once. `None` before
    /// [`init_shared`] made the pool or when the pool cannot give an item (with `PR_WAITOK` it
    /// sleeps for one; the C then panics `"rw_obj_alloc: rwlock_obj_pool is empty"`, which its
    /// callers that cannot fail do).
    pub fn new(name: &'static str) -> Option<Self> {
        Self::new_in(&RWOBJ_POOL, name)
    }

    /// [`SharedRwlock::new`] from `pool`.
    fn new_in(pool: &'static TypedPool<RawRwlock>, name: &'static str) -> Option<Self> {
        pool.get_shared(PR_WAITOK, || RawRwlock::new(name))
            .map(|l| Self(Shared::Pooled(l)))
    }

    /// A handle to a lock that lives for ever (the kernel object's, `"kobjlk"`): it counts
    /// nothing and frees nothing.
    pub const fn from_static(lock: &'static RawRwlock) -> Self {
        Self(Shared::Static(lock))
    }

    fn raw(&self) -> &RawRwlock {
        match &self.0 {
            Shared::Static(l) => l,
            Shared::Pooled(l) => l,
        }
    }

    /// `rw_enter_write(9)`: waits (sleeping, uninterruptibly) until the lock is free and takes
    /// it alone; the witness of it.
    pub fn write(&self) -> WriteWitness<'_> {
        let raw = self.raw();
        let me = raw.write_wait();
        WriteWitness::new(raw, me)
    }

    /// `rw_enter(9)` with `RW_WRITE | flags` (`RW_NOSLEEP`: `EBUSY` when held; `RW_INTR`: the
    /// sleep's error when a signal ends the wait).
    pub fn write_flags(&self, flags: i32) -> Result<WriteWitness<'_>, Errno> {
        let raw = self.raw();
        let me = raw.enter_write(flags)?;
        Ok(WriteWitness::new(raw, me))
    }

    /// `rw_enter(9)` with `RW_WRITE | RW_NOSLEEP`: `None` when the lock is held.
    pub fn try_write(&self) -> Option<WriteWitness<'_>> {
        self.write_flags(RW_NOSLEEP).ok()
    }

    /// `rw_enter_read(9)`: waits (sleeping, uninterruptibly) until no writer holds or waits
    /// for the lock and takes it shared; the witness of it.
    pub fn read(&self) -> ReadWitness<'_> {
        let raw = self.raw();
        raw.read_wait();
        ReadWitness::new(raw)
    }

    /// `rw_enter(9)` with `RW_READ | flags`, as [`SharedRwlock::write_flags`].
    pub fn read_flags(&self, flags: i32) -> Result<ReadWitness<'_>, Errno> {
        let raw = self.raw();
        raw.enter_read(flags)?;
        Ok(ReadWitness::new(raw))
    }

    /// `rw_enter(9)` with `RW_READ | RW_NOSLEEP`: `None` when a writer holds or waits.
    pub fn try_read(&self) -> Option<ReadWitness<'_>> {
        self.read_flags(RW_NOSLEEP).ok()
    }

    /// `rw_status(9)`: [`RW_WRITE`] (this process writes), [`RW_WRITE_OTHER`], [`RW_READ`] or
    /// 0 (free). A snapshot, for assertions (`rw_write_held` is `status() == RW_WRITE`).
    pub fn status(&self) -> i32 {
        self.raw().status()
    }

    /// `rwl_name`: the wait message.
    pub fn name(&self) -> &'static str {
        self.raw().name
    }

    /// The lock's identity, for what compares locks (`KASSERT(anon->an_lock ==
    /// amap->am_lock)`).
    pub fn id(&self) -> LockId {
        self.raw().id()
    }

    /// Whether two handles reach the same lock (the C's comparison of `struct rwlock *`).
    pub fn same(a: &Self, b: &Self) -> bool {
        a.id() == b.id()
    }
}

/// A lock's identity: its address as a number, compared and never dereferenced. Two live
/// locks never have the same one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LockId(usize);

/// The proof of holding a [`SharedRwlock`] alone (`rw_enter_write` on a lock object): no
/// data; the [`Guarded`] cells keyed to the lock open mutably through it. Its drop is
/// `rw_exit_write`. Not `Send`: the lock belongs to the process that took it.
#[must_use = "dropping the witness at once releases the lock"]
pub struct WriteWitness<'a> {
    lock: &'a RawRwlock,
    /// The writer word this witness stored (`rw_self` when it entered).
    me: usize,
    _not_send: PhantomData<*const ()>,
}

impl<'a> WriteWitness<'a> {
    fn new(lock: &'a RawRwlock, me: usize) -> Self {
        Self {
            lock,
            me,
            _not_send: PhantomData,
        }
    }

    /// `rwsleep_nsec(ident, lock, prio, wmesg, nsecs)` on the write-locked lock: queued on
    /// `ident`, the lock released, asleep, and the lock taken again for writing
    /// (uninterruptibly) before the sleep's result is returned (`EWOULDBLOCK` after `nsecs`,
    /// [`INFSLP`] never; `EINTR`/`ERESTART` on a signal with `PCATCH`). `&mut self`, so no
    /// reference from a [`Guarded`] lives across the sleep. `ident` is never the lock.
    pub fn sleep_nsec<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        let raw = self.lock;
        let me = self.me;
        let result = raw.sleep_released(ident, prio, wmesg, nsecs, || raw.exit_write(me));
        self.me = raw.write_wait();
        result
    }

    /// [`WriteWitness::sleep_nsec`] with no timeout.
    pub fn sleep<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.sleep_nsec(ident, prio, wmesg, INFSLP)
    }

    /// `rwsleep_nsec(ident, lock, prio | PNORELOCK, wmesg, nsecs)`: queued on `ident`, the lock
    /// released, asleep; the lock is not taken again. The witness is consumed, so nothing it
    /// proved can be used after the release (`uvm_pagewait`: the waiter takes the lock afresh
    /// and looks again).
    pub fn into_sleep_nsec<I: ?Sized>(
        self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        let this = ManuallyDrop::new(self);
        let (raw, me) = (this.lock, this.me);
        raw.sleep_released(ident, prio, wmesg, nsecs, || raw.exit_write(me))
    }

    /// [`WriteWitness::into_sleep_nsec`] with no timeout.
    pub fn into_sleep<I: ?Sized>(
        self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.into_sleep_nsec(ident, prio, wmesg, INFSLP)
    }

    /// `rw_enter(lock, RW_DOWNGRADE)`: the writer becomes a reader without letting another
    /// writer in; the readers waiting are woken when no writer waits.
    pub fn downgrade(self) -> ReadWitness<'a> {
        let this = ManuallyDrop::new(self);
        let (raw, me) = (this.lock, this.me);
        raw.downgrade(me);
        ReadWitness::new(raw)
    }
}

impl Drop for WriteWitness<'_> {
    fn drop(&mut self) {
        self.lock.exit_write(self.me);
    }
}

/// The proof of holding a [`SharedRwlock`] shared (`rw_enter_read` on a lock object): the
/// [`Guarded`] cells keyed to the lock open for reading through it. Its drop is
/// `rw_exit_read`. Not `Send`: the lock belongs to the process that took it.
#[must_use = "dropping the witness at once releases the lock"]
pub struct ReadWitness<'a> {
    lock: &'a RawRwlock,
    _not_send: PhantomData<*const ()>,
}

impl<'a> ReadWitness<'a> {
    fn new(lock: &'a RawRwlock) -> Self {
        Self {
            lock,
            _not_send: PhantomData,
        }
    }

    /// `rwsleep_nsec` on the read-locked lock: as [`WriteWitness::sleep_nsec`], the lock taken
    /// again shared.
    pub fn sleep_nsec<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        let raw = self.lock;
        let result = raw.sleep_released(ident, prio, wmesg, nsecs, || raw.exit_read());
        raw.read_wait();
        result
    }

    /// [`ReadWitness::sleep_nsec`] with no timeout.
    pub fn sleep<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.sleep_nsec(ident, prio, wmesg, INFSLP)
    }

    /// `rwsleep_nsec` with `PNORELOCK` on the read-locked lock: as
    /// [`WriteWitness::into_sleep_nsec`].
    pub fn into_sleep_nsec<I: ?Sized>(
        self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        let this = ManuallyDrop::new(self);
        let raw = this.lock;
        raw.sleep_released(ident, prio, wmesg, nsecs, || raw.exit_read())
    }

    /// [`ReadWitness::into_sleep_nsec`] with no timeout.
    pub fn into_sleep<I: ?Sized>(
        self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.into_sleep_nsec(ident, prio, wmesg, INFSLP)
    }

    /// `rw_enter(lock, RW_UPGRADE | RW_NOSLEEP)`: the only reader becomes the writer; while
    /// other readers hold the lock the witness comes back in the `Err` (the C's `EBUSY`).
    pub fn try_upgrade(self) -> Result<WriteWitness<'a>, ReadWitness<'a>> {
        match self.lock.upgrade() {
            Ok(me) => {
                let this = ManuallyDrop::new(self);
                Ok(WriteWitness::new(this.lock, me))
            }
            Err(_) => Err(self),
        }
    }
}

impl Drop for ReadWitness<'_> {
    fn drop(&mut self) {
        self.lock.exit_read();
    }
}

/// A cell of one object kept under a lock that another object may own (the C's "locked by
/// the amap's lock": an anon's fields, the amap's tables, the aobj's swap slots): it opens
/// only through a witness of that lock.
///
/// Invariant: `data` is reached only through [`Guarded::get`] and [`Guarded::get_mut`], with
/// a witness of the lock whose identity is `key`'s (checked at each access), or through
/// `&mut self`. A witness exists only while its lock is held in its mode (the owner word
/// holds its writer, or counts it among the readers) and borrows a handle that keeps the lock
/// alive, and two live locks never share an identity; so every reference alive into the
/// data comes from witnesses of one lock, and the lock's word keeps them apart as `rwlock(9)`
/// keeps its holders: one `&mut T` from the one writer, or `&T`s, never both. (A key left
/// from a lock that was freed matches only a later lock at the same address, whose witnesses
/// exclude each other just the same.)
pub struct Guarded<T> {
    key: Key,
    data: UnsafeCell<T>,
}

/// The identity a [`Guarded`] is keyed to.
#[derive(Clone, Copy)]
enum Key {
    /// A lock that lives for ever, named in a `const` context ([`Guarded::new_static`]).
    Static(&'static RawRwlock),
    /// A lock's address, as a number.
    Addr(usize),
}

impl Key {
    fn id(self) -> LockId {
        match self {
            Key::Static(l) => l.id(),
            Key::Addr(a) => LockId(a),
        }
    }
}

impl<T> Guarded<T> {
    /// `value`, kept under `lock`.
    pub fn new(lock: &SharedRwlock, value: T) -> Self {
        Self {
            key: Key::Addr(lock.id().0),
            data: UnsafeCell::new(value),
        }
    }

    /// `value`, kept under the lock that lives for ever at `lock` (a `static` beside it, the
    /// kernel object's swap slots under `"kobjlk"`); its witnesses come from
    /// [`SharedRwlock::from_static`] of the same lock.
    pub const fn new_static(lock: &'static RawRwlock, value: T) -> Self {
        Self {
            key: Key::Static(lock),
            data: UnsafeCell::new(value),
        }
    }

    /// The value, for the holder of `w`, a witness of the cell's lock in either mode; a
    /// witness of another lock is `panic(9)`. The reference borrows the cell and the witness,
    /// so it ends before the witness sleeps, upgrades or releases the lock.
    pub fn get<'a, W: Witness>(&'a self, w: &'a W) -> &'a T {
        self.check(w.lock_id());
        // SAFETY: `w` is a witness (the trait is sealed: a `WriteWitness` or a `ReadWitness`)
        // of the lock whose identity is the cell's key (checked just above), so the lock is
        // held and every reference alive into the data comes from witnesses of this lock (the
        // type's invariant). A `&mut T` exists only through `get_mut`, which needs the one
        // write witness borrowed mutably: if `w` is that witness it is borrowed shared here
        // for as long as this reference lives, and if `w` is a read witness no write witness
        // exists. So no `&mut T` is alive beside this `&T`, and it ends before `w` can be
        // released or slept (both need `w` by value or `&mut`).
        unsafe { &*self.data.get() }
    }

    /// The value, mutably, for the holder of `w`, the one write witness of the cell's lock; a
    /// witness of another lock is `panic(9)`. The reference borrows the witness mutably, so
    /// no other reference through it exists while it lives.
    pub fn get_mut<'a>(&'a self, w: &'a mut WriteWitness<'_>) -> &'a mut T {
        self.check(w.lock.id());
        // SAFETY: `w` holds the cell's lock for writing (identity checked above), so no read
        // witness and no other write witness of it exists, and every reference into the data
        // comes from a witness of this lock (the type's invariant): only through `w`, which
        // is borrowed mutably for the life of this reference, so no other reference from it
        // (by `get` or `get_mut`) can be alive or made meanwhile. This is the only reference
        // to the data while it lives, and it ends before `w` is released or slept.
        unsafe { &mut *self.data.get() }
    }

    /// Keys the cell to `lock` from now on: for a value still its maker's alone (`&mut self`:
    /// no reference into it is alive), as `amap_copy`'s new amap that takes a lock of its own
    /// before anyone sees it.
    pub fn rekey(&mut self, lock: &SharedRwlock) {
        self.key = Key::Addr(lock.id().0);
    }

    /// Whether the cell is under `lock` (`KASSERT(anon->an_lock == amap->am_lock)`).
    pub fn is_keyed_to(&self, lock: &SharedRwlock) -> bool {
        self.key.id() == lock.id()
    }

    /// The value, through the exclusive reference to the cell (its maker's, before it is
    /// shared, or its last user's).
    pub fn get_mut_owned(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// The value, consuming the cell.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// `panic(9)` unless `id` is the identity the cell is keyed to.
    fn check(&self, id: LockId) {
        if self.key.id() != id {
            hal_panic(format_args!(
                "Guarded: a witness of another lock ({:#x}, the cell's is {:#x})",
                id.0,
                self.key.id().0
            ));
        }
    }
}

// SAFETY: sharing `&Guarded<T>` lets the readers of the lock on several CPUs read the `T` at
// once (`T: Sync`) and the one writer, on any CPU, write it, moving it out included (`T:
// Send`); the type's invariant keeps the writer and the readers apart, as `Rwlocked`'s.
unsafe impl<T: Send + Sync> Sync for Guarded<T> {}

/// `rwlock_obj_pool`: the lock objects, made by [`init_shared`].
static RWOBJ_POOL: TypedPool<RawRwlock> = TypedPool::new();

/// What keeps [`Witness`] closed.
mod sealed {
    /// Implemented by the two witnesses alone.
    pub trait Sealed {}
}

/// A proof of holding a [`SharedRwlock`], in either mode: what [`Guarded::get`] checks.
/// Sealed: [`WriteWitness`] and [`ReadWitness`] are the only witnesses, so no other type can
/// claim a lock it does not hold.
pub trait Witness: sealed::Sealed {
    /// The identity of the lock this witness holds.
    fn lock_id(&self) -> LockId;
}

impl sealed::Sealed for WriteWitness<'_> {}

impl Witness for WriteWitness<'_> {
    fn lock_id(&self) -> LockId {
        self.lock.id()
    }
}

impl sealed::Sealed for ReadWitness<'_> {}

impl Witness for ReadWitness<'_> {
    fn lock_id(&self) -> LockId {
        self.lock.id()
    }
}

/// `rw_obj_init`: makes the pool of lock objects, `"rwobjpl"` at `IPL_MPFLOOR`, `PR_WAITOK`,
/// once, before the first [`SharedRwlock::new`] (the kernel's `uvm_init`). `ENOMEM` before the
/// pool bridge is registered, `EBUSY` when it was made already.
pub fn init_shared() -> Result<(), Errno> {
    RWOBJ_POOL.init(crate::intr::mpfloor(), PR_WAITOK, "rwobjpl")
}

/// `rw_self`: the running process as a writer word, `WRLOCK` set, so never 0.
fn rw_self() -> usize {
    (sleep::curproc_word() & !RWLOCK_MASK) | RWLOCK_WRLOCK
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::Duration;
    use std::vec::Vec;

    use super::*;
    use crate::testutil;

    #[test]
    fn write_then_read() {
        testutil::install();
        let l = Rwlocked::new("test", 1u32);
        *l.write() += 1;
        assert_eq!(*l.read(), 2);
        assert_eq!(l.status(), 0);
    }

    #[test]
    fn readers_share_and_a_writer_waits_for_them() {
        testutil::install();
        let l = Arc::new(Rwlocked::new("test", 0u32));
        let r1 = l.read();
        let r2 = l.read();
        assert_eq!(l.status(), RW_READ);
        assert!(l.try_write().is_none());
        let l2 = Arc::clone(&l);
        let writer = thread::spawn(move || {
            *l2.write() = 7;
        });
        thread::sleep(Duration::from_millis(20));
        assert_eq!(*r1 + *r2, 0);
        drop(r1);
        drop(r2);
        writer.join().unwrap();
        assert_eq!(*l.read(), 7);
    }

    #[test]
    fn a_waiting_writer_holds_new_readers_back() {
        testutil::install();
        let l = Arc::new(Rwlocked::new("test", 0u32));
        let r = l.read();
        let l2 = Arc::clone(&l);
        let writer = thread::spawn(move || {
            *l2.write() += 1;
        });
        // Wait until the writer counts itself in rwl_waiters.
        while l.lock.waiters.load(Ordering::SeqCst) == 0 {
            thread::yield_now();
        }
        assert!(
            l.try_read().is_none(),
            "a reader must not starve the writer"
        );
        drop(r);
        writer.join().unwrap();
        assert_eq!(*l.read(), 1);
    }

    #[test]
    fn try_write_and_try_read_answer_ebusy() {
        testutil::install();
        let l = Arc::new(Rwlocked::new("test", ()));
        let w = l.write();
        assert_eq!(l.status(), RW_WRITE);
        let l2 = Arc::clone(&l);
        let other = thread::spawn(move || {
            (
                l2.write_flags(RW_NOSLEEP).err(),
                l2.read_flags(RW_NOSLEEP).err(),
                l2.status(),
            )
        });
        assert_eq!(
            other.join().unwrap(),
            (Some(Errno::EBUSY), Some(Errno::EBUSY), RW_WRITE_OTHER)
        );
        drop(w);
    }

    #[test]
    #[should_panic(expected = "enter write deadlock")]
    fn writing_twice_panics() {
        testutil::install();
        let l = Rwlocked::new("test", ());
        let _a = l.write();
        let _b = l.write();
    }

    #[test]
    #[should_panic(expected = "enter read deadlock")]
    fn reading_what_one_writes_panics() {
        testutil::install();
        let l = Rwlocked::new("test", ());
        let _a = l.write();
        let _b = l.read();
    }

    #[test]
    fn writers_exclude_each_other() {
        testutil::install();
        let l = Arc::new(Rwlocked::new("test", 0u64));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let l = Arc::clone(&l);
                thread::spawn(move || {
                    for _ in 0..2000 {
                        let mut g = l.write();
                        let v = *g;
                        *g = v + 1;
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(*l.read(), 8000);
    }

    #[test]
    fn the_guard_sleeps_without_the_lock_and_wakes_holding_it() {
        testutil::install();
        let l = Arc::new(Rwlocked::new("test", false));
        let chan_word = Arc::new(0u8);
        let (l2, c2) = (Arc::clone(&l), Arc::clone(&chan_word));
        let sleeper = thread::spawn(move || {
            let mut g = l2.write();
            while !*g {
                g.sleep(Arc::as_ptr(&c2), PLOCK, "testslp").unwrap();
            }
            // The lock is held again on return.
            assert_eq!(l2.status(), RW_WRITE);
        });
        // The sleeper released the lock: this writer gets in.
        thread::sleep(Duration::from_millis(20));
        *l.write() = true;
        sleep::wakeup(Arc::as_ptr(&chan_word));
        sleeper.join().unwrap();
    }

    #[test]
    fn a_timed_guard_sleep_expires_holding_the_lock() {
        testutil::install();
        let l = Rwlocked::new("test", 0u8);
        let x = 0u8;
        let mut g = l.write();
        assert_eq!(
            g.sleep_nsec(&raw const x, PLOCK, "testslp", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        *g = 1;
        assert_eq!(l.status(), RW_WRITE);
        drop(g);
        assert_eq!(*l.read(), 1);
    }

    #[test]
    fn unlocked_lets_another_writer_in() {
        testutil::install();
        let l = Arc::new(Rwlocked::new("test", 0u32));
        let ran = Arc::new(AtomicBool::new(false));
        let mut g = l.write();
        let (l2, r2) = (Arc::clone(&l), Arc::clone(&ran));
        g.unlocked(|| {
            thread::spawn(move || {
                *l2.write() += 10;
                r2.store(true, Ordering::SeqCst);
            })
            .join()
            .unwrap();
        });
        assert!(ran.load(Ordering::SeqCst));
        *g += 1;
        drop(g);
        assert_eq!(*l.read(), 11);
    }

    #[test]
    fn downgrade_and_upgrade() {
        testutil::install();
        let l = Rwlocked::new("test", 5u32);
        let w = l.write();
        let r = w.downgrade();
        assert_eq!(l.status(), RW_READ);
        let r2 = l.read();
        let r = match r.try_upgrade() {
            Ok(_) => unreachable!("two readers: no upgrade"),
            Err(r) => r,
        };
        drop(r2);
        let mut w = r.try_upgrade().ok().unwrap();
        *w += 1;
        drop(w);
        assert_eq!(*l.read(), 6);
    }

    /// A lock object from `rwobjpl` (made once per test process).
    fn shared(name: &'static str) -> SharedRwlock {
        testutil::install();
        let _ = init_shared();
        SharedRwlock::new(name).unwrap()
    }

    /// The handles' count of a pooled lock.
    fn handles(l: &SharedRwlock) -> usize {
        match &l.0 {
            Shared::Pooled(p) => PoolArc::count(p),
            Shared::Static(_) => 0,
        }
    }

    #[test]
    fn a_lock_object_is_held_by_its_clones_and_freed_by_the_last() {
        testutil::install();
        static P: TypedPool<RawRwlock> = TypedPool::new();
        assert_eq!(P.init(0, PR_WAITOK, "testrwobj"), Ok(()));
        let a = SharedRwlock::new_in(&P, "testlk").unwrap();
        assert_eq!((P.nout(), handles(&a)), (1, 1));
        let b = a.clone(); // rw_obj_hold
        assert!(SharedRwlock::same(&a, &b));
        assert_eq!((P.nout(), handles(&a)), (1, 2));
        drop(a); // rw_obj_free: not the last
        assert_eq!((P.nout(), handles(&b)), (1, 1));
        assert_eq!(b.name(), "testlk");
        drop(b); // the last: pool_put
        assert_eq!(P.nout(), 0);
    }

    #[test]
    fn a_pool_never_made_gives_no_lock_object() {
        static NEVER: TypedPool<RawRwlock> = TypedPool::new();
        assert!(SharedRwlock::new_in(&NEVER, "testlk").is_none());
    }

    #[test]
    fn two_lock_objects_are_two_locks() {
        let a = shared("testa");
        let b = shared("testb");
        assert!(!SharedRwlock::same(&a, &b));
        assert_ne!(a.id(), b.id());
        let _w = a.write();
        assert_eq!(a.status(), RW_WRITE);
        assert_eq!(b.status(), 0);
        assert!(b.try_write().is_some());
    }

    /// An object with a field under a lock it shares, as an amap's tables.
    struct Owner {
        lock: SharedRwlock,
        count: Guarded<u64>,
    }

    #[test]
    fn writers_through_clones_exclude_each_other_on_a_guarded_cell() {
        let lock = shared("testlk");
        let o = Arc::new(Owner {
            count: Guarded::new(&lock, 0),
            lock,
        });
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let o = Arc::clone(&o);
                thread::spawn(move || {
                    // Each thread through its own handle, as an anon through the amap's lock.
                    let lock = o.lock.clone();
                    for _ in 0..2000 {
                        let mut w = lock.write();
                        let n = o.count.get_mut(&mut w);
                        let v = *n;
                        *n = v + 1;
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(*o.count.get(&o.lock.read()), 8000);
        assert_eq!(handles(&o.lock), 1);
    }

    #[test]
    fn readers_share_a_guarded_cell_and_a_writer_waits_for_them() {
        let lock = shared("testlk");
        let o = Arc::new(Owner {
            count: Guarded::new(&lock, 3),
            lock,
        });
        let r1 = o.lock.read();
        let r2 = o.lock.read();
        assert_eq!(*o.count.get(&r1) + *o.count.get(&r2), 6);
        assert_eq!(o.lock.status(), RW_READ);
        assert!(o.lock.try_write().is_none());
        let o2 = Arc::clone(&o);
        let writer = thread::spawn(move || {
            let mut w = o2.lock.write();
            *o2.count.get_mut(&mut w) = 9;
        });
        thread::sleep(Duration::from_millis(20));
        assert_eq!(*o.count.get(&r1), 3);
        drop(r1);
        drop(r2);
        writer.join().unwrap();
        assert_eq!(*o.count.get(&o.lock.read()), 9);
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_cell_refuses_a_write_witness_of_another_lock() {
        let mine = shared("testmine");
        let other = shared("testother");
        let cell = Guarded::new(&mine, 0u32);
        let mut w = other.write();
        *cell.get_mut(&mut w) = 1;
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_cell_refuses_a_read_witness_of_another_lock() {
        let mine = shared("testmine");
        let other = shared("testother");
        let cell = Guarded::new(&mine, 0u32);
        let r = other.read();
        let _ = *cell.get(&r);
    }

    #[test]
    fn a_cell_rekeyed_by_its_maker_opens_with_the_new_lock() {
        let first = shared("testfirst");
        let second = shared("testsecond");
        let mut cell = Guarded::new(&first, 1u32);
        assert!(cell.is_keyed_to(&first));
        *cell.get_mut_owned() += 1;
        cell.rekey(&second);
        assert!(cell.is_keyed_to(&second) && !cell.is_keyed_to(&first));
        assert_eq!(*cell.get(&second.read()), 2);
        assert_eq!(cell.into_inner(), 2);
    }

    #[test]
    fn a_static_lock_keys_a_static_cell() {
        static LOCK: RawRwlock = RawRwlock::new("testkobj");
        static CELL: Guarded<u32> = Guarded::new_static(&LOCK, 5);
        testutil::install();
        let l = SharedRwlock::from_static(&LOCK);
        assert_eq!(handles(&l), 0);
        *CELL.get_mut(&mut l.clone().write()) += 1;
        assert!(CELL.is_keyed_to(&l));
        assert_eq!(*CELL.get(&l.read()), 6);
        assert_eq!(l.name(), "testkobj");
    }

    #[test]
    fn into_sleep_releases_the_lock_and_returns_without_it() {
        let lock = shared("testlk");
        let o = Arc::new(Owner {
            count: Guarded::new(&lock, 0),
            lock,
        });
        let chan_word = Arc::new(0u8);
        let (o2, c2) = (Arc::clone(&o), Arc::clone(&chan_word));
        let sleeper = thread::spawn(move || {
            let mut rounds = 0;
            loop {
                let w = o2.lock.write();
                if *o2.count.get(&w) != 0 {
                    return rounds;
                }
                // PNORELOCK: queued under the lock, released, asleep; not taken again.
                w.into_sleep(Arc::as_ptr(&c2), PLOCK, "testslp").unwrap();
                assert_ne!(o2.lock.status(), RW_WRITE);
                rounds += 1;
            }
        });
        thread::sleep(Duration::from_millis(20));
        {
            // The sleeper released the lock: this writer gets in.
            let mut w = o.lock.write();
            *o.count.get_mut(&mut w) = 1;
        }
        sleep::wakeup(Arc::as_ptr(&chan_word));
        assert!(sleeper.join().unwrap() >= 1);
    }

    #[test]
    fn a_read_witness_sleeps_without_the_lock() {
        let lock = shared("testlk");
        let x = 0u8;
        let mut r = lock.read();
        assert_eq!(
            r.sleep_nsec(&raw const x, PLOCK, "testslp", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        assert_eq!(lock.status(), RW_READ);
        drop(r);
        let r = lock.read();
        assert_eq!(
            r.into_sleep_nsec(&raw const x, PLOCK, "testslp", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        // Both readers gone: the first's witness was dropped, the second's consumed.
        drop(lock.try_write().unwrap());
    }

    #[test]
    fn a_timed_witness_sleep_expires_holding_the_lock() {
        let lock = shared("testlk");
        let x = 0u8;
        let mut w = lock.write();
        assert_eq!(
            w.sleep_nsec(&raw const x, PLOCK, "testslp", 1_000_000),
            Err(Errno::EWOULDBLOCK)
        );
        assert_eq!(lock.status(), RW_WRITE);
        drop(w);
        assert_eq!(lock.status(), 0);
    }

    #[test]
    fn witnesses_downgrade_and_upgrade() {
        let lock = shared("testlk");
        let cell = Guarded::new(&lock, 5u32);
        let w = lock.write();
        let r = w.downgrade();
        assert_eq!(lock.status(), RW_READ);
        let r2 = lock.read();
        let r = match r.try_upgrade() {
            Ok(_) => unreachable!("two readers: no upgrade"),
            Err(r) => r,
        };
        assert_eq!(*cell.get(&r), 5);
        drop(r2);
        let mut w = r.try_upgrade().ok().unwrap();
        *cell.get_mut(&mut w) += 1;
        drop(w);
        assert_eq!(*cell.get(&lock.read()), 6);
    }

    #[test]
    #[should_panic(expected = "enter write deadlock")]
    fn writing_a_lock_object_twice_panics() {
        let lock = shared("testlk");
        let again = lock.clone();
        let _a = lock.write();
        let _b = again.write();
    }
}
/* </TESTS> */
