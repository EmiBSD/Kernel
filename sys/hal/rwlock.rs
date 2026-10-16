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
/* </LICENSES> */

/* <CODE> */
//! `Rwlocked<T>`: the sleeping reader-writer lock of `rwlock(9)` that owns its data.
//!
//! Upstream: sys/kern/kern_rwlock.c @ 3ce1f3f79392
//! Upstream: sys/sys/rwlock.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_rwlock.rs@3c62ede68802
//! LZ: sys/sys/rwlock.rs@3c62ede68802
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
//! the wait with the sleep's error (`EINTR`, `ERESTART`). The last one out wakes one waiting
//! writer, or else every waiting reader. Not recursive: a writer entering again, or reading
//! what it writes, panics ("enter write deadlock", "enter read deadlock"). A writer may
//! downgrade (waking the readers when no writer waits) and the only reader may try to
//! upgrade (`EBUSY` otherwise). `rwsleep(9)` releases the lock for the sleep and takes it
//! again, in the same mode, before returning, whatever the sleep returned.
//!
//! The sleep queues and `curproc` arrive through the `Sleep` bridge (`sleep.rs`). Before it is
//! registered nothing else runs: a contended lock spins instead of sleeping (which on the one
//! boot CPU never happens but in a bug), and a guard's sleep releases the lock, lets the CPU
//! breathe and takes it back.
//!
//! ## Deviations
//! - `WITNESS`, `RWDIAG` and the `dt(4)` tracepoints (`TRACEINDEX`, `rwl_traceidx`) are not
//!   configured, as in LZ.
//! - `rw_enter` with `RW_DOWNGRADE`/`RW_UPGRADE` are [`WriteGuard::downgrade`] and
//!   [`ReadGuard::try_upgrade`]; `PNORELOCK` does not exist for a guard (the guard would
//!   outlive the lock it proves); the `rw_assert_*` checks are the guards' types.
//!
//! ## Redesign
//! - LZ's `Rwlock` is the C's `struct rwlock` with free functions (`rw_enter_write(&l)`, the
//!   protected fields `Cell`s or raw pointers beside it, `rw_assert_wrlock` where the C
//!   asserts). `Rwlocked<T>` holds the data: [`Rwlocked::write`] and [`Rwlocked::read`] return
//!   guards through which alone it is reached (`&mut T` for the one writer, `&T` for the
//!   readers), and the guards' drops are `rw_exit_write`/`rw_exit_read`. Not `Send`: the lock
//!   belongs to the process that took it.
//! - `rwsleep_nsec(ident, &l, prio, wmesg, nsecs)` is [`WriteGuard::sleep_nsec`] (and
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

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::assert::{hal_panic, kassert};
use crate::bridges;
use crate::errno::Errno;
use crate::sleep::{self, INFSLP, PCATCH, PLOCK, PRIMASK, chan};

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
#[cfg(feature = "multiprocessor")]
const RW_SPINS: u32 = 1000;

/// `struct rwlock`: the owner word and the two wait counters whose addresses are the wait
/// channels.
struct RawRwlock {
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
    const fn new(name: &'static str) -> Self {
        Self {
            owner: AtomicUsize::new(0),
            waiters: AtomicU32::new(0),
            readers: AtomicU32::new(0),
            name,
        }
    }

    /// `rw_cas`: the compare-and-swap of the owner word; the value found.
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

    /// `rw_do_enter_write`: the word stored (this process as a writer).
    fn enter_write(&self, flags: i32) -> Result<usize, Errno> {
        let me = rw_self();
        let owner = self.cas(0, me);
        if owner == 0 {
            // wow, we won. so easy
            return Ok(me);
        }
        if owner == me {
            self.panic(format_args!("enter write deadlock"));
        }

        // If the process holds the kernel lock, give the CPU up as soon as possible so the
        // others waiting for the kernel lock progress: no spinning then.
        #[cfg(feature = "multiprocessor")]
        if !sleep::kernel_lock_held() {
            crate::cpu::spinning(1);
            for _ in 0..RW_SPINS {
                core::hint::spin_loop(); // CPU_BUSY_CYCLE()
                if self.owner.load(Ordering::Relaxed) != 0 {
                    continue;
                }
                if self.cas(0, me) == 0 {
                    crate::cpu::spinning(-1);
                    // ok, we won now.
                    return Ok(me);
                }
            }
            crate::cpu::spinning(-1);
        }

        if flags & RW_NOSLEEP != 0 {
            return Err(Errno::EBUSY);
        }

        let Some(b) = bridges::sleep() else {
            // Nothing else runs before the sleep queues are registered.
            loop {
                core::hint::spin_loop();
                if self.owner.load(Ordering::Relaxed) == 0 && self.cas(0, me) == 0 {
                    return Ok(me);
                }
            }
        };

        let mut prio = PLOCK - 4;
        if flags & RW_INTR != 0 {
            prio |= PCATCH;
        }

        self.waiters.fetch_add(1, Ordering::SeqCst);
        loop {
            b.sleep_setup(chan(&raw const self.waiters), prio, self.name);
            let owner = self.owner.load(Ordering::SeqCst);
            let error = b.sleep_finish(INFSLP, owner != 0);
            if flags & RW_INTR != 0
                && let Err(e) = error
            {
                self.waiters.fetch_sub(1, Ordering::SeqCst);
                return Err(e);
            }
            if self.cas(0, me) == 0 {
                break;
            }
        }
        self.waiters.fetch_sub(1, Ordering::SeqCst);
        Ok(me)
    }

    /// `rw_enter_write` without flags, which cannot fail.
    fn write_wait(&self) -> usize {
        match self.enter_write(0) {
            Ok(me) => me,
            Err(e) => self.panic(format_args!("uninterruptible enter failed ({e:?})")),
        }
    }

    /// `rw_read_incr`: one more reader on a word with no writer, starting from the guess
    /// `owner`; `false` when a writer got in first.
    fn read_incr(&self, owner: usize) -> bool {
        let mut owner = owner;
        loop {
            let found = self.cas(owner, owner + RWLOCK_READ_INCR);
            if found == owner {
                return true;
            }
            owner = found;
            if owner & RWLOCK_WRLOCK != 0 {
                return false;
            }
        }
    }

    /// `rw_do_enter_read`.
    fn enter_read(&self, flags: i32) -> Result<(), Errno> {
        let owner = self.cas(0, RWLOCK_READ_INCR);
        if owner == 0 {
            // ermagerd, we won!
            return Ok(());
        }

        if owner & RWLOCK_WRLOCK != 0 {
            if owner == rw_self() {
                self.panic(format_args!("enter read deadlock"));
            }
        } else if self.waiters.load(Ordering::SeqCst) == 0 && self.read_incr(owner) {
            // nailed it
            return Ok(());
        }

        if flags & RW_NOSLEEP != 0 {
            return Err(Errno::EBUSY);
        }

        let Some(b) = bridges::sleep() else {
            // Nothing else runs before the sleep queues are registered.
            loop {
                core::hint::spin_loop();
                let owner = self.owner.load(Ordering::Relaxed);
                if owner & RWLOCK_WRLOCK == 0 && self.read_incr(owner) {
                    return Ok(());
                }
            }
        };

        let mut prio = PLOCK;
        if flags & RW_INTR != 0 {
            prio |= PCATCH;
        }

        self.readers.fetch_add(1, Ordering::SeqCst);
        loop {
            b.sleep_setup(chan(&raw const self.readers), prio, self.name);
            let busy = self.waiters.load(Ordering::SeqCst) > 0
                || self.owner.load(Ordering::SeqCst) & RWLOCK_WRLOCK != 0;
            let error = b.sleep_finish(INFSLP, busy);
            if flags & RW_INTR != 0
                && let Err(e) = error
            {
                self.readers.fetch_sub(1, Ordering::SeqCst);
                return Err(e);
            }
            if self.read_incr(0) {
                break;
            }
        }
        self.readers.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }

    /// `rw_enter_read` without flags, which cannot fail.
    fn read_wait(&self) {
        if let Err(e) = self.enter_read(0) {
            self.panic(format_args!("uninterruptible enter failed ({e:?})"));
        }
    }

    /// `rw_exit_read` (`rw_do_exit_read` from the guess of one reader).
    fn exit_read(&self) {
        let mut owner = RWLOCK_READ_INCR;
        let left = loop {
            let decr = owner.wrapping_sub(RWLOCK_READ_INCR);
            let found = self.cas(owner, decr);
            if found == owner {
                break decr;
            }
            if found & RWLOCK_WRLOCK != 0 {
                self.panic(format_args!(
                    "exit read on write locked lock (owner {found:#x})"
                ));
            }
            if found == 0 {
                self.panic(format_args!("exit read on unlocked lock"));
            }
            owner = found;
        };
        if left == 0 {
            // last one out
            self.exited();
        }
    }

    /// `rw_exit_write`, for the writer that stored `me`.
    fn exit_write(&self, me: usize) {
        let owner = self.cas(me, 0);
        if owner != me {
            self.panic(format_args!(
                "exit write when lock not held (owner {owner:#x}, self {me:#x})"
            ));
        }
        self.exited();
    }

    /// `rw_exited`: wakes one waiting writer, or else every waiting reader.
    fn exited(&self) {
        if self.waiters.load(Ordering::SeqCst) > 0 {
            sleep::wakeup_one(&raw const self.waiters);
        } else if self.readers.load(Ordering::SeqCst) > 0 {
            sleep::wakeup(&raw const self.readers);
        }
    }

    /// `rw_downgrade`, for the writer that stored `me`.
    fn downgrade(&self, me: usize) {
        let owner = self.cas(me, RWLOCK_READ_INCR);
        if owner != me {
            self.panic(format_args!(
                "downgrade when lock not held (owner {owner:#x}, self {me:#x})"
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
        let owner = self.cas(RWLOCK_READ_INCR, me);
        if owner == RWLOCK_READ_INCR {
            return Ok(me);
        }
        if owner == 0 {
            self.panic(format_args!("upgrade on unowned lock"));
        }
        if owner & RWLOCK_WRLOCK != 0 {
            self.panic(format_args!(
                "upgrade on write locked lock(owner {owner:#x}, self {me:#x})"
            ));
        }
        Err(Errno::EBUSY)
    }

    /// `rw_status`.
    fn status(&self) -> i32 {
        let owner = self.owner.load(Ordering::SeqCst);
        if owner & RWLOCK_WRLOCK != 0 {
            if owner == rw_self() {
                return RW_WRITE;
            }
            return RW_WRITE_OTHER;
        }
        if owner != 0 {
            return RW_READ;
        }
        0
    }

    /// The `rwsleep_nsec` checks: the priority bits and a channel that is not the lock.
    fn check_sleep<I: ?Sized>(&self, ident: *const I, prio: i32) {
        kassert!(prio & !(PRIMASK | PCATCH) == 0);
        kassert!(!ptr::eq(chan(ident), chan(ptr::from_ref(self))));
        let _ = (ident, prio);
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
        raw.check_sleep(ident, prio);
        let result = match bridges::sleep() {
            Some(b) => {
                b.sleep_setup(chan(ident), prio, wmesg);
                raw.exit_write(self.me);
                // signal may stop the process, release rwlock before that
                b.sleep_finish(nsecs, true)
            }
            None => {
                // Nothing else runs before the sleep queues are registered: let the CPU
                // breathe, as the C's cold `tsleep` does.
                raw.exit_write(self.me);
                core::hint::spin_loop();
                Ok(())
            }
        };
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
        raw.check_sleep(ident, prio);
        let result = match bridges::sleep() {
            Some(b) => {
                b.sleep_setup(chan(ident), prio, wmesg);
                raw.exit_read();
                b.sleep_finish(nsecs, true)
            }
            None => {
                raw.exit_read();
                core::hint::spin_loop();
                Ok(())
            }
        };
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
}
/* </TESTS> */
