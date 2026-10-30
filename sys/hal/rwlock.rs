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
//! - N3.4d: the C asks for a lock object in a mode it decides at run time and asks again in
//!   the mode it found (`rw_enter(lock, flt->upper_lock_type)`, `rw_enter(anon->an_lock,
//!   rw_status(anon->an_lock))` in `uvm_fault.c`): [`RwMode`] is that mode as a value
//!   (`RW_READ`/`RW_WRITE`), [`SharedRwlock::enter`] takes the lock in it and returns an
//!   [`RwWitness`], one of the two witnesses as an enum. [`RwWitness::mode`] is the C's
//!   `rw_status(lock) == flt->upper_lock_type` as a fact of the witness rather than a read of
//!   the lock word, [`RwWitness::try_upgrade`] is `rw_enter(lock, RW_UPGRADE | RW_NOSLEEP)`
//!   with the writer's fast path, and its sleeps are the two witnesses'. An `RwWitness` is a
//!   [`Witness`] itself (the sealed trait gains the enum, which holds one of the two), so a
//!   [`Guarded`] cell opens through it in either mode; the cell's mutable access still wants
//!   the write witness itself ([`RwWitness::as_write`]).
//! - N3.5 S0: a lock embedded in the object whose fields it guards (the C's `vm_map.lock`
//!   beside the map's tree and its entries' handles) is a [`RawRwlock`] taken through its own
//!   witnesses ([`RawRwlock::write`], [`RawRwlock::read`], [`RawRwlock::enter`] and the
//!   `_flags`/`try_` forms; [`SharedRwlock`]'s are those of its lock), and the object's cells
//!   are keyed to it at placement ([`Guarded::new_keyed`], [`Guarded::rekey_raw`]). An
//!   `INTRSAFE` map's lock is a mutex: a [`MutexedGuard`] is a witness too
//!   ([`Guarded::new_mutexed`]; a [`LockId`] tags a mutex's so the two kinds never meet).
//!   [`Guarded::get_mut`] takes any [`Exclusive`] witness (a write witness, a mutex guard, a
//!   part). A whole exclusive witness ([`Divisible`]) splits into [`WitnessPart`]s
//!   ([`Divisible::split`]), each opening the cells of its part only ([`Guarded::keyed`] with
//!   a part, `0..WITNESS_PARTS`), so a map's state (part 0) and the handles of an entry
//!   borrowed from it (part 1) open mutably at once: one witness, disjoint cells, the
//!   borrow checker keeping each part's `&mut` unique.
//! - N4.1: cells keyed to a `static` mutex are made in a `const` context
//!   ([`Guarded::new_static_mutexed`], a key holding the mutex's owner word, as
//!   [`Guarded::new_static`] holds a static rwlock): the fields of every timeout under
//!   timeout(9)'s one `timeout_mutex`, initialised by `TIMEOUT_INITIALIZER`.
//! - N4.2: the kernel lock's witnesses, `hal::klock::KernelWitness` (exclusive, divisible)
//!   and `KernelReader` (shared), open cells made by [`Guarded::new_kernel`] (the C's `[K]`
//!   state: the process lists and links); the lock is held by the running CPU while one
//!   lives and the per-CPU count of `hal::klock` keeps the writer and the readers apart on it
//!   (that module's notes).
//! - N5.2: a lock the C enters in one function and leaves in another without a witness in
//!   between (`vfs_busy`/`vfs_unbusy` on `mnt_lock`, a file system's `VOP_LOCK`/`VOP_UNLOCK`
//!   on its node's `rrwlock`) is [`SplitRwlock`] (`rw_enter`/`rw_exit`) or [`RecursiveRwlock`]
//!   (`rrw_enter`/`rrw_exit`/`rrw_status`, `kern_rwlock.c`'s rrwlock: the writer may enter
//!   again, `RW_RECURSEFAIL` makes that `EDEADLK`). Both keep their [`RawRwlock`] private and
//!   hand out no witness, so no [`Guarded`] cell can be keyed to them: what they exclude is the
//!   C's protocol (the file system's node, the mount's unmount), never a Rust reference; a
//!   release the lock does not match is `panic(9)`, as the C's assertions make it.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::assert::{hal_panic, kassert};
use crate::bridges;
use crate::klock::{self, KernelReader, KernelWitness};
use crate::mutex::{Mutexed, MutexedGuard};
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
/// `RW_RECURSEFAIL`: fail on recursion for RRW locks ([`RecursiveRwlock::enter`]).
pub const RW_RECURSEFAIL: i32 = 0x0080;
/// `RW_WRITE_OTHER`: [`Rwlocked::status`] of a lock another process writes.
pub const RW_WRITE_OTHER: i32 = 0x0100;

/// `RW_SPINS`: how many times a writer polls the word before it sleeps, `MULTIPROCESSOR`.
const RW_SPINS: u32 = 1000;

/// `struct rwlock`: the owner word and the two wait counters whose addresses are the wait
/// channels. Inside a [`Rwlocked`] or a [`SharedRwlock`]'s pool item, a `static` of its own
/// ([`SharedRwlock::from_static`]), or embedded in the object whose fields it guards (a map's
/// `vmmaplk`, N3.5), taken there through its own witnesses ([`RawRwlock::write`],
/// [`RawRwlock::read`], [`RawRwlock::enter`]) and keyed to by that object's [`Guarded`] cells
/// ([`Guarded::new_keyed`]).
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

    /// The lock's identity: its address, as a number (two live locks never share one), for
    /// the [`Guarded`] cells keyed to it.
    #[inline]
    pub fn id(&self) -> LockId {
        LockId(ptr::from_ref(self).addr())
    }

    /// `rw_enter_write(9)`: waits (sleeping, uninterruptibly) until the lock is free and takes
    /// it alone; the witness of it.
    pub fn write(&self) -> WriteWitness<'_> {
        let me = self.write_wait();
        WriteWitness::new(self, me)
    }

    /// `rw_enter(9)` with `RW_WRITE | flags` (`RW_NOSLEEP`: `EBUSY` when held; `RW_INTR`: the
    /// sleep's error when a signal ends the wait).
    pub fn write_flags(&self, flags: i32) -> Result<WriteWitness<'_>, Errno> {
        let me = self.enter_write(flags)?;
        Ok(WriteWitness::new(self, me))
    }

    /// `rw_enter(9)` with `RW_WRITE | RW_NOSLEEP`: `None` when the lock is held.
    pub fn try_write(&self) -> Option<WriteWitness<'_>> {
        self.write_flags(RW_NOSLEEP).ok()
    }

    /// `rw_enter_read(9)`: waits (sleeping, uninterruptibly) until no writer holds or waits
    /// for the lock and takes it shared; the witness of it.
    pub fn read(&self) -> ReadWitness<'_> {
        self.read_wait();
        ReadWitness::new(self)
    }

    /// `rw_enter(9)` with `RW_READ | flags`, as [`RawRwlock::write_flags`].
    pub fn read_flags(&self, flags: i32) -> Result<ReadWitness<'_>, Errno> {
        self.enter_read(flags)?;
        Ok(ReadWitness::new(self))
    }

    /// `rw_enter(9)` with `RW_READ | RW_NOSLEEP`: `None` when a writer holds or waits.
    pub fn try_read(&self) -> Option<ReadWitness<'_>> {
        self.read_flags(RW_NOSLEEP).ok()
    }

    /// `rw_enter(lock, mode)`: waits (sleeping, uninterruptibly) for the lock in `mode`; the
    /// witness of it.
    pub fn enter(&self, mode: RwMode) -> RwWitness<'_> {
        match mode {
            RwMode::Read => RwWitness::Read(self.read()),
            RwMode::Write => RwWitness::Write(self.write()),
        }
    }

    /// `rw_enter(lock, mode | flags)` (`RW_NOSLEEP`: `EBUSY` when the lock cannot be had in
    /// `mode` now; `RW_INTR`: the sleep's error when a signal ends the wait).
    pub fn enter_flags(&self, mode: RwMode, flags: i32) -> Result<RwWitness<'_>, Errno> {
        Ok(match mode {
            RwMode::Read => RwWitness::Read(self.read_flags(flags)?),
            RwMode::Write => RwWitness::Write(self.write_flags(flags)?),
        })
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
    fn cas_write(&self, me: usize) -> bool {
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
            self.owner.load(Ordering::Relaxed) == 0 && self.cas_write(me)
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
            || self.cas_write(me),
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

    /// `rw_status(9)`: [`RW_WRITE`] (this process writes), [`RW_WRITE_OTHER`], [`RW_READ`] or
    /// 0 (free). A snapshot, for assertions (`rw_write_held` is `status() == RW_WRITE`).
    pub fn status(&self) -> i32 {
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
        if let Some(b) = bridges::sleep() {
            b.sleep_setup(chan(ident), prio, wmesg);
            exit();
            b.sleep_finish(nsecs, true)
        } else {
            exit();
            core::hint::spin_loop();
            Ok(())
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
        self.raw().write()
    }

    /// `rw_enter(9)` with `RW_WRITE | flags` (`RW_NOSLEEP`: `EBUSY` when held; `RW_INTR`: the
    /// sleep's error when a signal ends the wait).
    pub fn write_flags(&self, flags: i32) -> Result<WriteWitness<'_>, Errno> {
        self.raw().write_flags(flags)
    }

    /// `rw_enter(9)` with `RW_WRITE | RW_NOSLEEP`: `None` when the lock is held.
    pub fn try_write(&self) -> Option<WriteWitness<'_>> {
        self.raw().try_write()
    }

    /// `rw_enter_read(9)`: waits (sleeping, uninterruptibly) until no writer holds or waits
    /// for the lock and takes it shared; the witness of it.
    pub fn read(&self) -> ReadWitness<'_> {
        self.raw().read()
    }

    /// `rw_enter(9)` with `RW_READ | flags`, as [`SharedRwlock::write_flags`].
    pub fn read_flags(&self, flags: i32) -> Result<ReadWitness<'_>, Errno> {
        self.raw().read_flags(flags)
    }

    /// `rw_enter(9)` with `RW_READ | RW_NOSLEEP`: `None` when a writer holds or waits.
    pub fn try_read(&self) -> Option<ReadWitness<'_>> {
        self.raw().try_read()
    }

    /// `rw_enter(lock, mode)`: waits (sleeping, uninterruptibly) for the lock in `mode`, the
    /// mode the caller decided at run time (`flt->upper_lock_type`); the witness of it.
    pub fn enter(&self, mode: RwMode) -> RwWitness<'_> {
        self.raw().enter(mode)
    }

    /// `rw_enter(lock, mode | flags)` (`RW_NOSLEEP`: `EBUSY` when the lock cannot be had in
    /// `mode` now; `RW_INTR`: the sleep's error when a signal ends the wait).
    pub fn enter_flags(&self, mode: RwMode, flags: i32) -> Result<RwWitness<'_>, Errno> {
        self.raw().enter_flags(mode, flags)
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

/// A lock's identity: the address of its word as a number, with [`LOCKID_MUTEX`] set for a
/// [`Mutexed`]'s, compared and never dereferenced. Two live locks never have the same one: a
/// reader-writer lock's is its own address, a mutex's the address of its owner word, and
/// neither kind holds another lock of either kind at its start.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LockId(usize);

impl LockId {
    /// The identity of the mutex whose owner word is `word`.
    pub(crate) fn of_mutex(word: &AtomicUsize) -> LockId {
        LockId(ptr::from_ref(word).addr() | LOCKID_MUTEX)
    }

    /// The identity as a word, never 0 (the pinned lists record it, `pinlist.rs`).
    #[inline]
    pub(crate) fn raw(self) -> usize {
        self.0
    }
}

/// The bit of a [`LockId`] that says a mutex (a lock's word is aligned to 8 bytes, so its
/// address leaves the three low bits free).
const LOCKID_MUTEX: usize = 0b001;
/// Where a [`Guarded`] key keeps its part (the two bits above [`LOCKID_MUTEX`]).
const PART_SHIFT: u32 = 1;
/// The part bits of a [`Guarded`] key.
const PART_BITS: usize = 0b110;
/// How many parts an exclusive witness splits into at most ([`Divisible::split`]), and the
/// parts a [`Guarded`] cell may be made in (`0..WITNESS_PARTS`).
pub const WITNESS_PARTS: usize = 4;

/// One part of an exclusive witness ([`Divisible::split`]): it opens, mutably or not, only
/// the [`Guarded`] cells of its lock that were made in its part ([`Guarded::keyed`]), so the
/// parts of one witness reach disjoint cells at once (a map's state in one part, an entry's
/// handles, which borrow nothing of the state, in another). It borrows the witness it came
/// from mutably, so that witness opens nothing and cannot be released or slept while a part
/// lives; it is not `Clone`, not `Send`, and not divisible again.
#[must_use = "a part opens cells only while it lives"]
pub struct WitnessPart<'w> {
    id: LockId,
    part: u8,
    _borrow: PhantomData<&'w mut ()>,
    _not_send: PhantomData<*const ()>,
}

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

    /// Runs `f` with the lock released, then takes it again for writing before returning
    /// (the C's `rw_exit(lock)` ... `rw_enter(lock, RW_WRITE)` around I/O the holder does
    /// with the lock let go: `uvn_io`). `&mut self`, so no reference from a [`Guarded`] lives
    /// across; what the witness proved must be looked at again afterwards.
    pub fn unlocked<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let raw = self.lock;
        raw.exit_write(self.me);
        let r = f();
        self.me = raw.write_wait();
        r
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
/// the amap's lock": an anon's fields, the amap's tables, the aobj's swap slots), or that is
/// embedded beside it (a map's state and its entries' handles under the map's lock, N3.5): it
/// opens only through a witness of that lock.
///
/// Invariant: `data` is reached only through [`Guarded::get`] and [`Guarded::get_mut`], with
/// a witness that opens the cell (checked at each access: the identity of the witness's lock
/// is the key's, and a [`WitnessPart`]'s part is the cell's), or through `&mut self`. A
/// witness exists only while its lock is held in its mode (the owner word holds its writer,
/// or counts it among the readers; the mutex is this CPU's) and borrows what keeps the lock
/// alive, a part only while the whole exclusive witness it came from is borrowed mutably, and
/// two live locks never share an identity. So every reference alive into the data comes
/// from the witnesses that open it, which the lock keeps apart as `rwlock(9)` and `mutex(9)`
/// keep their holders: readers, or the one exclusive witness that opens the cell (the whole
/// write witness or mutex guard, or, while it is split, the one part of the cell's number);
/// one `&mut T`, or `&T`s, never both. (A key left from a lock that was freed or moved
/// matches only a later lock at the same address, whose witnesses exclude each other just the
/// same.)
pub struct Guarded<T> {
    key: Key,
    data: UnsafeCell<T>,
}

/// The identity a [`Guarded`] is keyed to, and its part.
#[derive(Clone, Copy)]
enum Key {
    /// A lock that lives for ever, named in a `const` context ([`Guarded::new_static`]);
    /// part 0.
    Static(&'static RawRwlock),
    /// A mutex that lives for ever, by its owner word, named in a `const` context
    /// ([`Guarded::new_static_mutexed`]); part 0.
    StaticMutex(&'static AtomicUsize),
    /// A lock's identity, its part in [`PART_BITS`].
    Addr(usize),
}

impl Key {
    /// The key of `id`'s cells of `part`; a part past [`WITNESS_PARTS`] is `panic(9)`.
    fn of(id: LockId, part: u8) -> Key {
        if usize::from(part) >= WITNESS_PARTS {
            hal_panic(format_args!(
                "Guarded: part {part} of at most {WITNESS_PARTS}"
            ));
        }
        Key::Addr(id.0 | (usize::from(part) << PART_SHIFT))
    }

    #[inline]
    fn id(self) -> LockId {
        match self {
            Key::Static(l) => l.id(),
            Key::StaticMutex(w) => LockId::of_mutex(w),
            Key::Addr(a) => LockId(a & !PART_BITS),
        }
    }

    #[inline]
    fn part(self) -> u8 {
        match self {
            Key::Static(_) | Key::StaticMutex(_) => 0,
            // At most `PART_BITS >> PART_SHIFT`, 3: it fits.
            Key::Addr(a) => ((a & PART_BITS) >> PART_SHIFT) as u8,
        }
    }
}

impl<T> Guarded<T> {
    /// `value`, kept under `lock`.
    pub fn new(lock: &SharedRwlock, value: T) -> Self {
        Self::keyed(lock.id(), 0, value)
    }

    /// `value`, kept under `lock`, a lock embedded beside the cell (the map's `vmmaplk`); its
    /// witnesses are `lock`'s own ([`RawRwlock::write`]). Key it after the object is placed
    /// where it stays ([`Guarded::rekey_raw`]): a cell keyed to the address a lock had before
    /// it moved opens with no witness.
    pub fn new_keyed(lock: &RawRwlock, value: T) -> Self {
        Self::keyed(lock.id(), 0, value)
    }

    /// `value`, kept under the mutex `lock` (an `INTRSAFE` map's `vm_map.mtx`); its witness is
    /// the mutex's guard ([`MutexedGuard`]).
    pub fn new_mutexed<U>(lock: &Mutexed<U>, value: T) -> Self {
        Self::keyed(lock.id(), 0, value)
    }

    /// `value`, kept under the lock whose identity is `id`, in `part` (`0..WITNESS_PARTS`; a
    /// cell of part `p` opens with a whole witness or with the part `p` of a split one,
    /// [`Divisible::split`]); a part past them is `panic(9)`.
    pub fn keyed(id: LockId, part: u8, value: T) -> Self {
        Self {
            key: Key::of(id, part),
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

    /// `value`, kept under the mutex that lives for ever at `lock` (a `static` beside it:
    /// timeout(9)'s wheel, whose `timeout_mutex` guards every timeout's `[T]` fields), made in
    /// a `const` context (`TIMEOUT_INITIALIZER`); its witness is the mutex's guard.
    pub const fn new_static_mutexed<U>(lock: &'static Mutexed<U>, value: T) -> Self {
        Self {
            key: Key::StaticMutex(lock.owner_word()),
            data: UnsafeCell::new(value),
        }
    }

    /// `value`, kept under the kernel lock (the C's `[K]` members), made in a `const`
    /// context (a `static` process, a value being built); its witnesses are
    /// `hal::klock::witness` and `hal::klock::reader`.
    pub const fn new_kernel(value: T) -> Self {
        Self {
            key: Key::StaticMutex(&klock::KERNEL_LOCK_ID),
            data: UnsafeCell::new(value),
        }
    }

    /// The value, for the holder of `w`, a witness that opens the cell (of its lock, in
    /// either mode; a part of its part); any other witness is `panic(9)`. The reference
    /// borrows the cell and the witness, so it ends before the witness sleeps, upgrades or
    /// releases the lock.
    #[inline]
    pub fn get<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> &'a T {
        self.check(w);
        // SAFETY: `w` is a witness (the trait is sealed: a `WriteWitness`, a `ReadWitness`, an
        // `RwWitness` holding one of the two, a `MutexedGuard`, a `WitnessPart`, or the kernel
        // lock's `KernelWitness` or `KernelReader`, which `hal::klock` keeps apart per CPU as
        // the lock keeps them apart across CPUs, and whose lock is not released while one is
        // out, its release paths' contract) that opens
        // this cell (checked just above), so its lock is held and every reference alive into
        // the data comes from the witnesses that open it (the type's invariant). A `&mut T`
        // exists only through `get_mut`, which needs the one exclusive witness that opens the
        // cell borrowed mutably: if `w` is that witness, or an `RwWitness` holding it (whose
        // `as_write` needs it mutably), it is borrowed shared here for as long as this
        // reference lives; if `w` is or holds a read witness, no exclusive witness exists; if
        // `w` is a part, the whole witness it came from is borrowed mutably and the other
        // parts open other cells. So no `&mut T` is alive beside this `&T`, and it ends before
        // `w` can be released or slept (both need `w` by value or `&mut`).
        unsafe { &*self.data.get() }
    }

    /// The value, mutably, for the holder of `w`, an exclusive witness that opens the cell
    /// (the write witness or the mutex guard of its lock, or the part of its part); any other
    /// witness is `panic(9)`. The reference borrows the witness mutably, so no other
    /// reference through it exists while it lives.
    #[inline]
    pub fn get_mut<'a, W: Exclusive + ?Sized>(&'a self, w: &'a mut W) -> &'a mut T {
        self.check(w);
        // SAFETY: `w` is an exclusive witness (sealed: a `WriteWitness`, a `MutexedGuard`, a
        // `WitnessPart`, or the kernel lock's `KernelWitness`, alone on its CPU by
        // `hal::klock`'s count) that opens this cell (checked above): the lock is held alone (no
        // read witness and no other writer or guard exists), and of its witnesses only `w`
        // opens the cell now. A whole witness opens nothing while it is split (its parts
        // borrow it mutably), so if `w` is whole no part exists; if `w` is a part, its whole
        // witness is borrowed by the split, and the other parts of that split have other
        // numbers (a part cannot be split again). Every reference into the data comes from a
        // witness that opens it (the type's invariant), so only through `w`, which is borrowed
        // mutably for the life of this reference: no other reference from it (by `get` or
        // `get_mut`) can be alive or made meanwhile. This is the only reference to the data
        // while it lives, and it ends before `w` is released or slept.
        unsafe { &mut *self.data.get() }
    }

    /// Keys the cell to `lock` from now on, in the same part: for a value still its maker's
    /// alone (`&mut self`: no reference into it is alive), as `amap_copy`'s new amap that
    /// takes a lock of its own before anyone sees it.
    pub fn rekey(&mut self, lock: &SharedRwlock) {
        self.rekey_id(lock.id());
    }

    /// Keys the cell to the embedded `lock` from now on, in the same part (a map placed in its
    /// pool item, its lock with it).
    pub fn rekey_raw(&mut self, lock: &RawRwlock) {
        self.rekey_id(lock.id());
    }

    /// Keys the cell to the mutex `lock` from now on, in the same part.
    pub fn rekey_mutexed<U>(&mut self, lock: &Mutexed<U>) {
        self.rekey_id(lock.id());
    }

    /// Keys the cell to the lock whose identity is `id` from now on, in the same part.
    pub fn rekey_id(&mut self, id: LockId) {
        self.key = Key::of(id, self.key.part());
    }

    /// The identity of the cell's lock.
    pub fn key(&self) -> LockId {
        self.key.id()
    }

    /// The cell's part (`0..WITNESS_PARTS`).
    pub fn part(&self) -> u8 {
        self.key.part()
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

    /// `panic(9)` unless `w` opens the cell: a witness of the lock the cell is keyed to, and,
    /// for a part, of the cell's part.
    #[inline]
    fn check<W: Witness + ?Sized>(&self, w: &W) {
        let id = w.lock_id();
        if self.key.id() != id {
            another_lock(id, self.key.id());
        }
        if let Some(p) = w.part()
            && p != self.key.part()
        {
            another_part(p, self.key.part());
        }
    }
}

/// [`Guarded::check`]'s refusal of a witness of another lock (`id`; the cell's is `cell`),
/// out of the accesses' line.
#[cold]
#[inline(never)]
fn another_lock(id: LockId, cell: LockId) -> ! {
    hal_panic(format_args!(
        "Guarded: a witness of another lock ({:#x}, the cell's is {:#x})",
        id.0, cell.0
    ))
}

/// [`Guarded::check`]'s refusal of a part `p` that is not the cell's (`cell`).
#[cold]
#[inline(never)]
fn another_part(p: u8, cell: u8) -> ! {
    hal_panic(format_args!(
        "Guarded: a witness of part {p}, the cell is in part {cell}"
    ))
}

// SAFETY: sharing `&Guarded<T>` lets the readers of the lock on several CPUs read the `T` at
// once (`T: Sync`) and the one writer, on any CPU, write it, moving it out included (`T:
// Send`); the type's invariant keeps the writer and the readers apart, as `Rwlocked`'s.
unsafe impl<T: Send + Sync> Sync for Guarded<T> {}

/// `rwlock_obj_pool`: the lock objects, made by [`init_shared`].
static RWOBJ_POOL: TypedPool<RawRwlock> = TypedPool::new();

/// What keeps [`Witness`] closed.
mod sealed {
    /// Implemented by the witnesses alone.
    pub trait Sealed {}
}

/// A proof of holding a lock: a reader-writer lock in either mode, a [`Mutexed`], or the
/// kernel lock; what [`Guarded::get`] checks. Sealed: [`WriteWitness`], [`ReadWitness`],
/// [`RwWitness`] (which is one of them), a [`MutexedGuard`] (and the scheduler's hold, one),
/// a [`WitnessPart`],
/// [`KernelWitness`] and [`KernelReader`] are the only witnesses, so no other type can claim
/// a lock it does not hold.
pub trait Witness: sealed::Sealed {
    /// The identity of the lock this witness holds.
    fn lock_id(&self) -> LockId;

    /// The part of the lock's cells this witness opens: `None` for all of them (a whole
    /// witness), the part's number for a [`WitnessPart`].
    fn part(&self) -> Option<u8> {
        None
    }
}

/// A witness that holds its lock alone and so opens its cells mutably
/// ([`Guarded::get_mut`]): [`WriteWitness`], a [`MutexedGuard`] (the one holder of a
/// mutex, the same proof as a writer's), [`KernelWitness`], and a [`WitnessPart`] (for its
/// part). Sealed, as [`Witness`].
pub trait Exclusive: Witness {}

/// An exclusive witness of a whole lock, divisible into parts that open disjoint cells at
/// once: [`WriteWitness`], [`MutexedGuard`] and [`KernelWitness`]. Sealed, as [`Witness`].
pub trait Divisible: Exclusive {
    /// The witness divided into `N` parts (`N` at most [`WITNESS_PARTS`]), numbered `0..N`,
    /// each opening the cells of its number ([`Guarded::keyed`]); the witness is borrowed
    /// mutably while they live.
    fn split<const N: usize>(&mut self) -> [WitnessPart<'_>; N] {
        const {
            assert!(
                N <= WITNESS_PARTS,
                "a witness splits into at most WITNESS_PARTS parts"
            );
        }
        let id = self.lock_id();
        core::array::from_fn(|i| WitnessPart {
            id,
            // `i < N <= WITNESS_PARTS`: it fits.
            part: i as u8,
            _borrow: PhantomData,
            _not_send: PhantomData,
        })
    }
}

impl sealed::Sealed for WriteWitness<'_> {}

impl Witness for WriteWitness<'_> {
    #[inline]
    fn lock_id(&self) -> LockId {
        self.lock.id()
    }
}

impl Exclusive for WriteWitness<'_> {}

impl Divisible for WriteWitness<'_> {}

impl<T> sealed::Sealed for MutexedGuard<'_, T> {}

impl<T> Witness for MutexedGuard<'_, T> {
    #[inline]
    fn lock_id(&self) -> LockId {
        self.mutex_id()
    }
}

impl<T> Exclusive for MutexedGuard<'_, T> {}

impl<T> Divisible for MutexedGuard<'_, T> {}

// The scheduler's hold is its mutex's guard (`hal::thread`): the same proof, by delegation.
impl sealed::Sealed for crate::thread::SchedHold {}

impl Witness for crate::thread::SchedHold {
    #[inline]
    fn lock_id(&self) -> LockId {
        self.0.lock_id()
    }
}

impl Exclusive for crate::thread::SchedHold {}

impl Divisible for crate::thread::SchedHold {}

impl sealed::Sealed for WitnessPart<'_> {}

impl Witness for WitnessPart<'_> {
    #[inline]
    fn lock_id(&self) -> LockId {
        self.id
    }

    #[inline]
    fn part(&self) -> Option<u8> {
        Some(self.part)
    }
}

impl Exclusive for WitnessPart<'_> {}

impl sealed::Sealed for ReadWitness<'_> {}

impl Witness for ReadWitness<'_> {
    #[inline]
    fn lock_id(&self) -> LockId {
        self.lock.id()
    }
}

impl sealed::Sealed for KernelWitness {}

impl Witness for KernelWitness {
    #[inline]
    fn lock_id(&self) -> LockId {
        klock::kernel_lock_id()
    }
}

impl Exclusive for KernelWitness {}

impl Divisible for KernelWitness {}

impl sealed::Sealed for KernelReader {}

impl Witness for KernelReader {
    #[inline]
    fn lock_id(&self) -> LockId {
        klock::kernel_lock_id()
    }
}

/// The mode of a lock asked for at run time: `RW_READ` (shared) or `RW_WRITE` (alone), as
/// the C keeps it in an `int` (`flt->upper_lock_type`) and compares it with `rw_status`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RwMode {
    /// `RW_READ`: shared with other readers.
    Read,
    /// `RW_WRITE`: alone.
    Write,
}

/// `RW_OPMASK`: the operation bits of `rw_enter`'s flags.
const RW_OPMASK: i32 = 0x0007;

impl RwMode {
    /// The mode of the C's `RW_READ` or `RW_WRITE` (the operation bits of `flags`; the other
    /// flags ignored); `None` for any other operation (`RW_DOWNGRADE`, `RW_UPGRADE`, none).
    pub fn from_flags(flags: i32) -> Option<Self> {
        match flags & RW_OPMASK {
            RW_READ => Some(RwMode::Read),
            RW_WRITE => Some(RwMode::Write),
            _ => None,
        }
    }

    /// The C's `RW_READ` or `RW_WRITE`, which is also what [`SharedRwlock::status`] answers
    /// for a lock this process holds in the mode.
    pub fn flags(self) -> i32 {
        match self {
            RwMode::Read => RW_READ,
            RwMode::Write => RW_WRITE,
        }
    }
}

/// A witness of a [`SharedRwlock`] held in a mode decided at run time
/// ([`SharedRwlock::enter`]): the write witness or a read witness, whichever the mode asked
/// for. It is that witness: its drop releases the lock in its mode, its sleeps are the
/// witness's, and a [`Guarded`] cell opens through it ([`Witness`]). Not `Send`, as the two.
#[must_use = "dropping the witness at once releases the lock"]
pub enum RwWitness<'a> {
    /// Held shared (`RW_READ`).
    Read(ReadWitness<'a>),
    /// Held alone (`RW_WRITE`).
    Write(WriteWitness<'a>),
}

impl<'a> RwWitness<'a> {
    /// The mode the lock is held in: the C's `rw_status(lock)` of a lock this process holds,
    /// answered by the witness instead of the lock word.
    pub fn mode(&self) -> RwMode {
        match self {
            RwWitness::Read(_) => RwMode::Read,
            RwWitness::Write(_) => RwMode::Write,
        }
    }

    /// `rw_enter(lock, RW_UPGRADE | RW_NOSLEEP)` for a reader, the writer's fast path
    /// otherwise: the write witness at once when the lock is held alone already, or when this
    /// reader is the only one; while other readers hold it, the witness comes back in the
    /// `Err`, still a reader (the C's `EBUSY`).
    pub fn try_upgrade(self) -> Result<WriteWitness<'a>, RwWitness<'a>> {
        match self {
            RwWitness::Write(w) => Ok(w),
            RwWitness::Read(r) => r.try_upgrade().map_err(RwWitness::Read),
        }
    }

    /// The write witness, mutably, when the lock is held alone (for [`Guarded::get_mut`] and
    /// the functions that change what the lock guards); `None` for a reader.
    pub fn as_write(&mut self) -> Option<&mut WriteWitness<'a>> {
        match self {
            RwWitness::Write(w) => Some(w),
            RwWitness::Read(_) => None,
        }
    }

    /// `rwsleep_nsec(ident, lock, prio, wmesg, nsecs)` in the witness's mode: queued on
    /// `ident`, the lock released, asleep, and the lock taken again in the same mode before
    /// the sleep's result is returned ([`WriteWitness::sleep_nsec`],
    /// [`ReadWitness::sleep_nsec`]).
    pub fn sleep_nsec<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        match self {
            RwWitness::Read(r) => r.sleep_nsec(ident, prio, wmesg, nsecs),
            RwWitness::Write(w) => w.sleep_nsec(ident, prio, wmesg, nsecs),
        }
    }

    /// [`RwWitness::sleep_nsec`] with no timeout.
    pub fn sleep<I: ?Sized>(
        &mut self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.sleep_nsec(ident, prio, wmesg, INFSLP)
    }

    /// `rwsleep_nsec` with `PNORELOCK` in the witness's mode: queued on `ident`, the lock
    /// released, asleep; the lock is not taken again and the witness is consumed
    /// ([`WriteWitness::into_sleep_nsec`], [`ReadWitness::into_sleep_nsec`]).
    pub fn into_sleep_nsec<I: ?Sized>(
        self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        match self {
            RwWitness::Read(r) => r.into_sleep_nsec(ident, prio, wmesg, nsecs),
            RwWitness::Write(w) => w.into_sleep_nsec(ident, prio, wmesg, nsecs),
        }
    }

    /// [`RwWitness::into_sleep_nsec`] with no timeout.
    pub fn into_sleep<I: ?Sized>(
        self,
        ident: *const I,
        prio: i32,
        wmesg: &'static str,
    ) -> Result<(), Errno> {
        self.into_sleep_nsec(ident, prio, wmesg, INFSLP)
    }
}

impl<'a> From<WriteWitness<'a>> for RwWitness<'a> {
    fn from(w: WriteWitness<'a>) -> Self {
        RwWitness::Write(w)
    }
}

impl<'a> From<ReadWitness<'a>> for RwWitness<'a> {
    fn from(r: ReadWitness<'a>) -> Self {
        RwWitness::Read(r)
    }
}

impl sealed::Sealed for RwWitness<'_> {}

impl Witness for RwWitness<'_> {
    #[inline]
    fn lock_id(&self) -> LockId {
        match self {
            RwWitness::Read(r) => r.lock_id(),
            RwWitness::Write(w) => w.lock_id(),
        }
    }
}

/// A `struct rwlock` entered and left by separate calls, with no witness in between: the
/// C's `rw_enter(rwl, flags)` in one function and `rw_exit(rwl)` in another (`vfs_busy` and
/// `vfs_unbusy` on a mount's `mnt_lock`). Its [`RawRwlock`] is private and gives no witness,
/// so nothing can key a [`Guarded`] cell to it: it excludes the protocol its users follow,
/// not references. Entering it for writing again from the writer is the C's `panic(9)`
/// ("enter write deadlock"); leaving it when it is not held is `panic(9)` too.
pub struct SplitRwlock {
    lock: RawRwlock,
}

impl SplitRwlock {
    /// `RWLOCK_INITIALIZER(name)`: a free lock whose sleeps say `name`.
    pub const fn new(name: &'static str) -> Self {
        Self {
            lock: RawRwlock::new(name),
        }
    }

    /// `rw_enter(rwl, flags)`: the mode of `flags & RW_OPMASK` ([`RW_WRITE`] or [`RW_READ`]),
    /// with [`RW_NOSLEEP`] (`EBUSY` when it cannot be had now) and [`RW_INTR`] (the sleep's
    /// error when a signal ends the wait). Any other mode is `panic(9)` (the C's
    /// `panic("%s: bad flags")`).
    pub fn enter(&self, flags: i32) -> Result<(), Errno> {
        match flags & RW_OPMASK {
            m if m == RW_WRITE => self.lock.enter_write(flags).map(|_| ()),
            m if m == RW_READ => self.lock.enter_read(flags),
            _ => self
                .lock
                .panic(format_args!("rw_enter: bad flags {flags:#x}")),
        }
    }

    /// `rw_exit(rwl)`: leaves the lock in the mode it is held in (the writer's word, or one
    /// reader of the count); a lock this process does not hold for writing, or that no one
    /// holds, is `panic(9)` (`rw_assert_wrlock`, `rw_assert_rdlock`).
    pub fn exit(&self) {
        if self.lock.owner.load(Ordering::SeqCst) & RWLOCK_WRLOCK != 0 {
            self.lock.exit_write(rw_self());
        } else {
            self.lock.exit_read();
        }
    }

    /// `rw_status(rwl)`: [`RW_WRITE`] (this process writes), [`RW_WRITE_OTHER`], [`RW_READ`]
    /// or 0 (free).
    pub fn status(&self) -> i32 {
        self.lock.status()
    }

    /// Whether this process holds the lock for writing (`rw_write_held`).
    pub fn held_by_self(&self) -> bool {
        self.lock.owner.load(Ordering::SeqCst) == rw_self()
    }

    /// `rwl_name`: the wait message.
    pub fn name(&self) -> &'static str {
        self.lock.name()
    }
}

/// `struct rrwlock`: a [`SplitRwlock`] whose writer may enter it again (`rrw_enter`), the
/// lock a file system's node carries for `VOP_LOCK`/`VOP_UNLOCK`. `wcnt` counts the
/// writer's entries; it is read and written only by the process that holds the lock for
/// writing (the owner word is its own, checked first), so no two writers of it ever race;
/// readers never touch it but for the `rrw_enter` that set it to 1 after a read entry, as the
/// C does (an unused value: a reader's `rrw_exit` never looks at it).
pub struct RecursiveRwlock {
    lock: SplitRwlock,
    wcnt: AtomicU32,
}

impl RecursiveRwlock {
    /// `rrw_init(rrwl, name)`: a free lock.
    pub const fn new(name: &'static str) -> Self {
        Self {
            lock: SplitRwlock::new(name),
            wcnt: AtomicU32::new(0),
        }
    }

    /// `rrw_enter(rrwl, flags)`: the writer that holds the lock enters it once more
    /// (`EDEADLK` instead with [`RW_RECURSEFAIL`]); anyone else enters it as
    /// [`SplitRwlock::enter`] does.
    pub fn enter(&self, flags: i32) -> Result<(), Errno> {
        if self.lock.held_by_self() {
            if flags & RW_RECURSEFAIL != 0 {
                return Err(Errno::EDEADLK);
            }
            self.wcnt.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        self.lock.enter(flags)?;
        self.wcnt.store(1, Ordering::Relaxed);
        Ok(())
    }

    /// `rrw_exit(rrwl)`: the writer leaves one of its entries, the lock with the last; a
    /// reader leaves its read hold.
    pub fn exit(&self) {
        if self.lock.held_by_self() {
            let n = self.wcnt.load(Ordering::Relaxed);
            kassert!(n > 0);
            self.wcnt.store(n.saturating_sub(1), Ordering::Relaxed);
            if n > 1 {
                return;
            }
        }
        self.lock.exit();
    }

    /// `rrw_status(rrwl)`: as [`SplitRwlock::status`].
    pub fn status(&self) -> i32 {
        self.lock.status()
    }

    /// The writer's entries (`rrwl_wcnt`), for assertions.
    pub fn depth(&self) -> u32 {
        if self.lock.held_by_self() {
            self.wcnt.load(Ordering::Relaxed)
        } else {
            0
        }
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

// A lock's word is aligned to 8 bytes, so a `LockId` keeps its kind and a key its part in the
// three low bits.
const _: () = assert!(align_of::<RawRwlock>() >= 8 && align_of::<AtomicUsize>() >= 8);
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
        // Once: a test that lost the race to make the pool (EBUSY while the winner's
        // `OnceStatic::set` was still under way) would find no pool yet.
        static INIT: std::sync::Once = std::sync::Once::new();
        testutil::install();
        INIT.call_once(|| {
            let _ = init_shared();
        });
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

    #[test]
    fn a_write_witness_lets_go_of_the_lock_and_takes_it_again() {
        let a = shared("testul");
        let mut w = a.write();
        let other = std::thread::scope(|s| {
            w.unlocked(|| s.spawn(|| a.try_write().is_some()).join().unwrap())
        });
        assert!(other, "another writer got the lock while it was let go");
        assert_eq!(a.status(), RW_WRITE, "taken again for writing");
        assert!(std::thread::scope(|s| s
            .spawn(|| a.try_write().is_none())
            .join()
            .unwrap()));
        drop(w);
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

    #[test]
    fn a_mode_is_the_c_flag() {
        // (flags, the mode they ask for)
        let table = [
            (RW_READ, Some(RwMode::Read)),
            (RW_WRITE, Some(RwMode::Write)),
            (RW_READ | RW_NOSLEEP, Some(RwMode::Read)),
            (RW_WRITE | RW_INTR, Some(RwMode::Write)),
            (0x0004, None), // RW_DOWNGRADE
            (0x0005, None), // RW_UPGRADE
            (0, None),
        ];
        for (flags, mode) in table {
            assert_eq!(RwMode::from_flags(flags), mode, "flags {flags:#x}");
        }
        assert_eq!(RwMode::Read.flags(), RW_READ);
        assert_eq!(RwMode::Write.flags(), RW_WRITE);
    }

    #[test]
    fn enter_takes_the_lock_in_the_mode_asked() {
        let lock = shared("testlk");
        let r = lock.enter(RwMode::Read);
        assert_eq!(r.mode(), RwMode::Read);
        assert_eq!(lock.status(), RwMode::Read.flags());
        let r2 = lock.enter(RwMode::Read);
        assert_eq!(lock.status(), RW_READ);
        assert!(lock.try_write().is_none());
        drop((r, r2));
        let w = lock.enter(RwMode::Write);
        assert_eq!(w.mode(), RwMode::Write);
        assert_eq!(lock.status(), RwMode::Write.flags());
        drop(w);
        assert_eq!(lock.status(), 0);
    }

    #[test]
    fn enter_flags_answers_ebusy_in_either_mode() {
        let lock = Arc::new(shared("testlk"));
        let w = lock.enter(RwMode::Write);
        let l2 = Arc::clone(&lock);
        let other = thread::spawn(move || {
            (
                l2.enter_flags(RwMode::Read, RW_NOSLEEP).err(),
                l2.enter_flags(RwMode::Write, RW_NOSLEEP).err(),
            )
        });
        assert_eq!(
            other.join().unwrap(),
            (Some(Errno::EBUSY), Some(Errno::EBUSY))
        );
        drop(w);
        let r = lock.enter_flags(RwMode::Read, RW_NOSLEEP).unwrap();
        assert_eq!(r.mode(), RwMode::Read);
        drop(r);
        let w = lock.enter_flags(RwMode::Write, RW_NOSLEEP).unwrap();
        assert_eq!(w.mode(), RwMode::Write);
    }

    #[test]
    fn a_writer_upgrades_at_once_and_a_lone_reader_upgrades() {
        let lock = shared("testlk");
        // The fast path: already the writer.
        let w = lock.enter(RwMode::Write).try_upgrade().ok().unwrap();
        assert_eq!(lock.status(), RW_WRITE);
        drop(w);
        // The only reader.
        let w = lock.enter(RwMode::Read).try_upgrade().ok().unwrap();
        assert_eq!(lock.status(), RW_WRITE);
        drop(w);
        assert_eq!(lock.status(), 0);
    }

    #[test]
    fn a_reader_beside_another_stays_a_reader() {
        let lock = shared("testlk");
        let other = lock.read();
        let r = match lock.enter(RwMode::Read).try_upgrade() {
            Ok(_) => unreachable!("two readers: no upgrade"),
            Err(r) => r,
        };
        assert_eq!(r.mode(), RwMode::Read);
        assert_eq!(lock.status(), RW_READ);
        drop(other);
        drop(r);
        assert_eq!(lock.status(), 0);
    }

    #[test]
    fn a_cell_opens_through_either_arm() {
        let lock = shared("testlk");
        let cell = Guarded::new(&lock, 1u32);
        let r = lock.enter(RwMode::Read);
        assert_eq!(*cell.get(&r), 1);
        let mut r = r;
        assert!(r.as_write().is_none());
        drop(r);
        let mut w = lock.enter(RwMode::Write);
        assert_eq!(*cell.get(&w), 1);
        *cell.get_mut(w.as_write().unwrap()) = 2;
        assert_eq!(*cell.get(&w), 2);
        let w: RwWitness<'_> = w.try_upgrade().ok().unwrap().downgrade().into();
        assert_eq!((w.mode(), *cell.get(&w)), (RwMode::Read, 2));
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_cell_refuses_an_rw_witness_of_another_lock() {
        let mine = shared("testmine");
        let other = shared("testother");
        let cell = Guarded::new(&mine, 0u32);
        let w = other.enter(RwMode::Write);
        let _ = *cell.get(&w);
    }

    #[test]
    fn an_rw_witness_sleeps_in_its_mode() {
        let lock = shared("testlk");
        let x = 0u8;
        for mode in [RwMode::Read, RwMode::Write] {
            let mut w = lock.enter(mode);
            assert_eq!(
                w.sleep_nsec(&raw const x, PLOCK, "testslp", 1_000_000),
                Err(Errno::EWOULDBLOCK)
            );
            // Taken again in the same mode.
            assert_eq!(lock.status(), mode.flags());
            assert_eq!(w.mode(), mode);
            drop(w);
            let w = lock.enter(mode);
            assert_eq!(
                w.into_sleep_nsec(&raw const x, PLOCK, "testslp", 1_000_000),
                Err(Errno::EWOULDBLOCK)
            );
            // Released and not taken again.
            assert_eq!(lock.status(), 0);
        }
    }

    #[test]
    fn an_rw_witness_into_sleep_lets_a_writer_in() {
        let lock = shared("testlk");
        let o = Arc::new(Owner {
            count: Guarded::new(&lock, 0),
            lock,
        });
        let chan_word = Arc::new(0u8);
        let (o2, c2) = (Arc::clone(&o), Arc::clone(&chan_word));
        let sleeper = thread::spawn(move || {
            loop {
                let w = o2.lock.enter(RwMode::Read);
                if *o2.count.get(&w) != 0 {
                    return;
                }
                w.into_sleep(Arc::as_ptr(&c2), PLOCK, "testslp").unwrap();
            }
        });
        thread::sleep(Duration::from_millis(20));
        {
            let mut w = o.lock.enter(RwMode::Write);
            *o.count.get_mut(w.as_write().unwrap()) = 1;
        }
        sleep::wakeup(Arc::as_ptr(&chan_word));
        sleeper.join().unwrap();
    }

    /// A map's shape: its lock embedded beside the state and an entry's handles.
    struct Map {
        lock: RawRwlock,
        state: Guarded<u64>,
        handles: Guarded<u64>,
    }

    impl Map {
        /// Built, moved into place, then keyed (`Vmspace::place`).
        fn placed() -> std::boxed::Box<Map> {
            let mut m = std::boxed::Box::new(Map {
                lock: RawRwlock::new("vmmaplk"),
                state: Guarded::keyed(LockId(0), 0, 0),
                handles: Guarded::keyed(LockId(0), 1, 0),
            });
            let id = m.lock.id();
            m.state.rekey_id(id);
            m.handles.rekey_id(id);
            m
        }
    }

    #[test]
    fn a_raw_lock_gives_its_own_witnesses() {
        testutil::install();
        let l = RawRwlock::new("test");
        let w = l.write();
        assert_eq!(l.status(), RW_WRITE);
        // Another thread finds it held (this one reading it would be "enter read deadlock").
        assert!(thread::scope(|s| s
            .spawn(|| l.try_read().is_none())
            .join()
            .unwrap()));
        drop(w);
        let r1 = l.read();
        let r2 = l.enter(RwMode::Read);
        assert_eq!(r2.mode(), RwMode::Read);
        assert_eq!(l.status(), RW_READ);
        assert!(l.try_write().is_none());
        assert_eq!(l.write_flags(RW_NOSLEEP).err(), Some(Errno::EBUSY));
        drop((r1, r2));
        let w = l.enter_flags(RwMode::Write, 0).unwrap();
        assert_eq!(w.mode(), RwMode::Write);
        drop(w);
        assert_eq!(l.status(), 0);
    }

    #[test]
    fn a_cell_keyed_to_an_embedded_lock_opens_with_its_witnesses() {
        testutil::install();
        let m = Map::placed();
        assert_eq!((m.state.key(), m.state.part()), (m.lock.id(), 0));
        assert_eq!(m.handles.part(), 1);
        let mut w = m.lock.write();
        *m.state.get_mut(&mut w) = 5;
        *m.handles.get_mut(&mut w) = 6;
        drop(w);
        let r = m.lock.read();
        assert_eq!((*m.state.get(&r), *m.handles.get(&r)), (5, 6));
        drop(r);
        let mut rw = m.lock.enter(RwMode::Write);
        *m.state.get_mut(rw.as_write().unwrap()) += 1;
        assert_eq!(*m.state.get(&rw), 6);
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_cell_refuses_a_witness_of_another_raw_lock() {
        testutil::install();
        let m = Map::placed();
        let other = RawRwlock::new("other");
        let w = other.write();
        let _ = m.state.get(&w);
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_cell_keyed_before_its_lock_moved_opens_with_none() {
        testutil::install();
        let m = std::boxed::Box::new(RawRwlock::new("moved"));
        let cell = Guarded::new_keyed(&RawRwlock::new("stack"), 1u8);
        let w = m.write();
        let _ = cell.get(&w);
    }

    #[test]
    fn the_parts_of_a_witness_open_disjoint_cells_at_once() {
        testutil::install();
        let m = Map::placed();
        let mut w = m.lock.write();
        {
            let [mut p0, mut p1] = w.split();
            let state = m.state.get_mut(&mut p0);
            let handles = m.handles.get_mut(&mut p1);
            *state += 10;
            *handles += 20;
            assert_eq!((*state, *handles), (10, 20));
            assert_eq!(*m.state.get(&p0), 10);
        }
        // Whole again: it opens every part.
        assert_eq!((*m.state.get(&w), *m.handles.get(&w)), (10, 20));
    }

    #[test]
    #[should_panic(expected = "a witness of part 1, the cell is in part 0")]
    fn a_part_refuses_a_cell_of_another_part() {
        testutil::install();
        let m = Map::placed();
        let mut w = m.lock.write();
        let [_p0, mut p1] = w.split();
        let _ = m.state.get_mut(&mut p1);
    }

    #[test]
    #[should_panic(expected = "part 4 of at most 4")]
    fn a_part_past_the_parts_is_refused() {
        let l = RawRwlock::new("test");
        let _ = Guarded::keyed(l.id(), 4, 0u8);
    }

    #[test]
    fn a_cell_keyed_to_a_mutex_opens_with_its_guard_and_its_parts() {
        testutil::install();
        let mtx = Mutexed::new(crate::intr::IPL_VM, ());
        let state = Guarded::new_mutexed(&mtx, 1u32);
        let handles = Guarded::keyed(mtx.id(), 1, 2u32);
        let mut g = mtx.lock();
        *state.get_mut(&mut g) += 1;
        assert_eq!(*state.get(&g), 2);
        {
            let [mut p0, mut p1] = g.split();
            let (s, h) = (state.get_mut(&mut p0), handles.get_mut(&mut p1));
            *s += *h;
        }
        assert_eq!(*state.get(&g), 4);
        drop(g);
        let mut c = Guarded::keyed(mtx.id(), 1, 0u8);
        let other = Mutexed::new(crate::intr::IPL_VM, ());
        c.rekey_mutexed(&other);
        assert_eq!((c.key(), c.part()), (other.id(), 1));
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_mutex_cell_refuses_a_guard_of_another_mutex() {
        testutil::install();
        let a = Mutexed::new(crate::intr::IPL_VM, ());
        let b = Mutexed::new(crate::intr::IPL_VM, ());
        let cell = Guarded::new_mutexed(&a, 0u8);
        let g = b.lock();
        let _ = cell.get(&g);
    }

    #[test]
    fn a_mutex_and_an_rwlock_never_share_an_identity() {
        let w = AtomicUsize::new(0);
        let m = LockId::of_mutex(&w);
        assert_ne!(m, LockId(ptr::from_ref(&w).addr()));
        assert_eq!(m.0 & LOCKID_MUTEX, LOCKID_MUTEX);
    }

    #[test]
    fn a_cell_opens_through_a_dyn_witness_of_either_kind() {
        testutil::install();
        let m = Map::placed();
        let mtx = Mutexed::new(crate::intr::IPL_VM, ());
        let intr = Guarded::new_mutexed(&mtx, 7u64);
        {
            let mut w = m.lock.write();
            let any: &mut dyn Exclusive = &mut w;
            *m.state.get_mut(any) = 3;
        }
        let r = m.lock.read();
        let any: &dyn Witness = &r;
        assert_eq!(*m.state.get(any), 3);
        drop(r);
        let mut g = mtx.lock();
        let any: &mut dyn Exclusive = &mut g;
        *intr.get_mut(any) += 1;
        let any: &dyn Witness = &g;
        assert_eq!(*intr.get(any), 8);
    }

    /// A `static` mutex keying cells made in a `const` context (timeout(9)'s wheel).
    static STATIC_MTX: crate::mutex::SharedMutex = Mutexed::new(crate::intr::IPL_NONE, ());

    #[test]
    fn a_const_cell_opens_with_its_static_mutex() {
        testutil::install();
        static CELL: Guarded<u32> = Guarded::new_static_mutexed(&STATIC_MTX, 5);
        let mut g = STATIC_MTX.lock();
        *CELL.get_mut(&mut g) += 1;
        assert_eq!(*CELL.get(&g), 6);
        assert_eq!(CELL.key(), STATIC_MTX.id());
        assert_eq!(CELL.part(), 0);
    }

    #[test]
    #[should_panic(expected = "a witness of another lock")]
    fn a_const_cell_refuses_another_mutex() {
        testutil::install();
        static CELL: Guarded<u32> = Guarded::new_static_mutexed(&STATIC_MTX, 5);
        let other = Mutexed::new(crate::intr::IPL_NONE, ());
        let g = other.lock();
        let _ = CELL.get(&g);
    }

    #[test]
    fn a_split_lock_is_entered_and_left_by_separate_calls() {
        testutil::install();
        let l = SplitRwlock::new("split");
        assert_eq!(l.status(), 0);
        assert_eq!(l.enter(RW_READ), Ok(()));
        assert_eq!(l.enter(RW_READ | RW_NOSLEEP), Ok(()));
        assert_eq!(l.status(), RW_READ);
        assert_eq!(l.enter(RW_WRITE | RW_NOSLEEP), Err(Errno::EBUSY));
        l.exit();
        l.exit();
        assert_eq!(l.status(), 0);
        assert_eq!(l.enter(RW_WRITE), Ok(()));
        assert!(l.held_by_self());
        assert_eq!(l.status(), RW_WRITE);
        l.exit();
        assert_eq!(l.status(), 0);
    }

    #[test]
    #[should_panic(expected = "enter write deadlock")]
    fn a_split_lock_is_not_entered_twice_by_its_writer() {
        testutil::install();
        let l = SplitRwlock::new("split");
        let _ = l.enter(RW_WRITE);
        let _ = l.enter(RW_WRITE);
    }

    #[test]
    #[should_panic(expected = "exit read on unlocked lock")]
    fn a_split_lock_not_held_is_not_left() {
        testutil::install();
        SplitRwlock::new("split").exit();
    }

    #[test]
    fn a_recursive_lock_counts_its_writer_s_entries() {
        testutil::install();
        let l = RecursiveRwlock::new("rrw");
        assert_eq!(l.enter(RW_WRITE), Ok(()));
        assert_eq!(l.depth(), 1);
        assert_eq!(l.enter(RW_WRITE), Ok(()));
        assert_eq!(l.depth(), 2);
        // RW_RECURSEFAIL refuses the third entry and leaves the count alone.
        assert_eq!(l.enter(RW_WRITE | RW_RECURSEFAIL), Err(Errno::EDEADLK));
        assert_eq!(l.depth(), 2);
        l.exit();
        assert_eq!(l.status(), RW_WRITE);
        assert_eq!(l.depth(), 1);
        l.exit();
        assert_eq!(l.status(), 0);
        assert_eq!(l.depth(), 0);
        // A reader is a plain one.
        assert_eq!(l.enter(RW_READ), Ok(()));
        assert_eq!(l.status(), RW_READ);
        l.exit();
        assert_eq!(l.status(), 0);
    }

    #[test]
    fn a_recursive_lock_held_elsewhere_is_busy_for_a_try() {
        testutil::install();
        let l: &'static RecursiveRwlock =
            std::boxed::Box::leak(std::boxed::Box::new(RecursiveRwlock::new("rrw")));
        assert_eq!(l.enter(RW_WRITE), Ok(()));
        thread::scope(|s| {
            s.spawn(|| {
                testutil::install();
                assert_eq!(l.enter(RW_WRITE | RW_NOSLEEP), Err(Errno::EBUSY));
                assert_eq!(l.enter(RW_READ | RW_NOSLEEP), Err(Errno::EBUSY));
                assert_eq!(l.status(), RW_WRITE_OTHER);
                assert_eq!(l.depth(), 0);
            });
        });
        l.exit();
    }
}
/* </TESTS> */
