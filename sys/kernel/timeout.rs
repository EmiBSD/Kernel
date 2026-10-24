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
 * Copyright (c) 2001 Thomas Nordin <nordin@openbsd.org>
 * Copyright (c) 2000-2001 Artur Grabowski <art@openbsd.org>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES,
 * INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY
 * AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
 * THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL  DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright (c) 2000-2001 Artur Grabowski <art@openbsd.org>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES,
 * INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY
 * AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
 * THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL  DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `timeout(9)`: functions run once, a number of clock ticks or an absolute uptime from now.
//!
//! Upstream: sys/kern/kern_timeout.c @ 3ce1f3f79392
//! Upstream: sys/sys/timeout.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_timeout.rs@3c62ede68802
//! LZ: sys/sys/timeout.rs@3c62ede68802
//!
//! Timeouts are kept in a hierarchical timing wheel: four levels of 256 buckets of tick-based
//! timeouts (`to_time` is the value of `ticks` when one is due) and as many of clock-based
//! ones (`KCLOCK_UPTIME`, an absolute uptime hashed into a bucket). `timeout_add` puts a
//! timeout on the `new` queue; [`hardclock_update`], called by `hardclock` on the primary CPU
//! every tick, moves the buckets that came due onto the `todo` queue and schedules the
//! `softclock` soft interrupt when there is work; [`softclock`] puts every timeout of `new` and
//! `todo` in its bucket, or runs it when it is due, or hands it to a thread when it needs a
//! process context (`TIMEOUT_PROC`): the `softclock` thread, pegged to the primary CPU under
//! the kernel lock, or, with `TIMEOUT_MPSAFE`, `softclockmp`, which drops it. The soft interrupt
//! is established without `IPL_MPSAFE`, so `softclock` runs under the kernel lock too.
//!
//! A timeout runs with the wheel's lock released (`timeout_run`): its function may add or
//! delete itself or others. The context records what it is running, so [`Timeout::barrier`]
//! can wait until a running timeout is done: it queues a timeout of its own at the head of
//! that context's queue and sleeps until it has run.
//!
//! Locks (the C's legend): `[T]` the wheel's lock (`timeout_mutex`, a spin mutex at
//! `IPL_HIGH`; timeouts are added and deleted from interrupt handlers), `[I]` set at startup.
//!
//! ## Deviations
//! - `WITNESS` and `kcov` are not configured: `timeout_sync_*` do nothing and `to_process` is
//!   not kept (LZ recorded it only).
//! - ddb's `show callout` (`db_show_callout` and its printers) is not ported (LZ's gap, "the
//!   real ddb (M7)"), kept visible here.
//! - A timeout dropped while it is pending is deleted first (its `Drop`), where the C, freed
//!   without `timeout_del`, leaves a dangling queue entry; the deletion is counted
//!   (`tos_cancelled`, `tos_deleted`).
//!
//! ## Redesign
//! - The C's circular queues (`struct circq`, the `CIRCQ_*` macros, `timeout_from_circq`'s
//!   cast; LZ: `circq_*` `unsafe fn`s over `Cell<*const Circq>`) are `hal::pinlist::PinRing`s:
//!   a timeout is linked through `Pin<&Timeout>` (a `static`, a pool item, a stack frame:
//!   `core::pin::pin!`), its entry checked unlinked when it is dropped, and `CIRCQ_REMOVE`
//!   names no queue, as the C's. The 2052 heads are a `static` (`pin_static`); the queues are
//!   bound to the wheel's lock, whose identity every timeout carries, so `MOVEBUCKET` and
//!   `CIRCQ_CONCAT` are a splice (addendum C.1). `tos_pending`, counted in the C's macros, is
//!   counted where they ran.
//! - The `[T]` globals (`tostat`, `timeout_kclock`, `tick_ts`, the contexts' `tctx_running`)
//!   are the data of the wheel's `Mutexed`; a context's running timeout is its address as a
//!   word, compared and never followed.
//! - A timeout's `[T]` fields (`to_func`, `to_kclock`, `to_abstime`) are a `hal::Guarded`
//!   cell keyed to that `Mutexed` (`Guarded::new_static_mutexed`, so `TIMEOUT_INITIALIZER` is a
//!   `const fn`); `to_time`, `to_arg` and `to_flags` are atomics written under it (`to_flags` is
//!   read without it by `timeout_pending` and friends, as the C's macros; `to_time` changes in
//!   `timeout_adjust_ticks`'s walk). `timeout_set` writes them under the lock, where the C
//!   writes them bare (its contract: not while pending).
//! - `timeout_set` keeps `TIMEOUT_ONQUEUE` where the C overwrites `to_flags` whole: a set on
//!   a pending timeout, which the C's contract forbids and the C would leave linked but not
//!   `ONQUEUE` (lost to `timeout_del`), stays pending and deletable. `TIMEOUT_TRIGGERED` is
//!   cleared, as the C clears it (the two bits never coexist: a timeout is triggered when it
//!   leaves its queue).
//! - `timeout_barrier`'s `struct cond` is the barrier timeout itself: the context sets its
//!   `TIMEOUT_TRIGGERED` and takes it off its queue under the lock, then its function wakes the
//!   waiter, whose condition is both (`tmobar`, as `cond_wait`): the waiter returns only once
//!   the node is unlinked, at the C's point (`c_wait` cleared by the function) or before it,
//!   never while the node is still on the queue.
//! - `timeout_startup` takes `hz`, `tick` and `tick_nsec` as data (`bsd`'s `conf/param.rs`,
//!   never changed after boot); `softintr_establish` and `kthread_create[_deferred]` go through
//!   `hal::softintr` (`hal`'s own since N4.5c) and `kernel::thread` (`kernel::fork`'s since
//!   N4.6a),
//!   `nanouptime` through
//!   `kernel::clock` (`Uptime`).
//! - `timeout_sysctl` stays at the ABI's side (`sys/sys/timeout.rs`): [`stats`] is the copy of
//!   `tostat` it takes under the lock, in the C's field order.
//! - N4.4: a timeout whose function dereferenced its `void *` (kqueue's `filt_timerexpire`,
//!   the knote) takes a counted, erased argument instead ([`Timeout::set_shared`],
//!   `hal::pool::AnyArc`): the timeout holds the handle until it is set again or cleared
//!   ([`Timeout::clear`]), the wheel clones it under its lock for the call, and the function
//!   gets its object back by naming its type. The C's `fn(void *)` stays for the callers that
//!   pass an address or nothing (`TimeoutFn`, the legacy adapter's).

use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicPtr, AtomicU32, Ordering};

use hal::intr::{IPL_HIGH, IPL_SOFTCLOCK};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::pin::pin_static;
use hal::pinlist::{BoundRing, PinEntry, PinRing};
use hal::pool::AnyArc;
use hal::rwlock::Guarded;
use hal::sleep::{INFSLP, PSWP, PWAIT};
use hal::softintr::SoftHandle;

use crate::assert::{kassert, kpanic};
use crate::clock::{self, TICKS, Timespec};
use crate::thread::{self, Start};

/// `void (*to_func)(void *)`: the function a timeout calls, with its argument (which the
/// kernel never dereferences: it is the caller's).
pub type TimeoutFn = fn(*mut c_void);

/// The function of a timeout set by [`Timeout::set_shared`]: its argument is the counted
/// handle the timeout holds, lent for the call.
pub type SharedFn = fn(&AnyArc);

/// `TIMEOUT_PROC`: needs a process context.
pub const TIMEOUT_PROC: u32 = 0x01;
/// `TIMEOUT_ONQUEUE`: on any timeout queue.
pub const TIMEOUT_ONQUEUE: u32 = 0x02;
/// `TIMEOUT_INITIALIZED`: initialized.
pub const TIMEOUT_INITIALIZED: u32 = 0x04;
/// `TIMEOUT_TRIGGERED`: running or ran.
pub const TIMEOUT_TRIGGERED: u32 = 0x08;
/// `TIMEOUT_MPSAFE`: run without the kernel lock (with `TIMEOUT_PROC` only).
pub const TIMEOUT_MPSAFE: u32 = 0x10;

/// `KCLOCK_NONE`: a tick-based timeout.
pub const KCLOCK_NONE: i32 = -1;
/// `KCLOCK_UPTIME`: the uptime clock; time since boot.
pub const KCLOCK_UPTIME: i32 = 0;
/// `KCLOCK_MAX`.
pub const KCLOCK_MAX: i32 = 1;

/// `WHEELCOUNT`: the levels of each wheel.
const WHEELCOUNT: usize = 4;
/// `WHEELSIZE`: the buckets of a level.
const WHEELSIZE: usize = 256;
/// `WHEELMASK`.
const WHEELMASK: u32 = 255;
/// `WHEELBITS`.
const WHEELBITS: u32 = 8;
/// `BUCKETS`.
const BUCKETS: usize = WHEELCOUNT * WHEELSIZE;

/// `timeout_level_width`: a wheel level's width in seconds (`2 << (level * WHEELBITS)`; the C
/// fills it at startup).
const TIMEOUT_LEVEL_WIDTH: [i64; WHEELCOUNT] = {
    let mut w = [0i64; WHEELCOUNT];
    let mut level = 0;
    while level < WHEELCOUNT {
        w[level] = 2 << (level as u32 * WHEELBITS);
        level += 1;
    }
    w
};

/// `to_func`: the C's `void (*)(void *)` with `to_arg`, or a function of a counted argument.
#[derive(Clone, Copy)]
enum Func {
    /// The C's function, called with `to_arg`.
    Raw(TimeoutFn),
    /// A function of the handle in [`ToState::shared`].
    Shared(SharedFn),
}

/// A timeout's `[T]` fields that the `Guarded` cell keeps.
struct ToState {
    /// `to_func`: the function (`None` for one `timeout_set` never named, the C's `NULL`).
    func: Option<Func>,
    /// `to_kclock`: the kernel clock of `abstime`, or `KCLOCK_NONE`.
    kclock: i32,
    /// `to_abstime`: the absolute time to run at (`KCLOCK_UPTIME`).
    abstime: Timespec,
    /// The argument of a [`Func::Shared`] function, held until the timeout is set again or
    /// cleared.
    shared: Option<AnyArc>,
}

/// What a timeout taken off its queue runs, copied under the wheel's lock.
enum Call {
    /// `to_func(to_arg)`.
    Raw(TimeoutFn, *mut c_void),
    /// A shared function and a clone of its argument.
    Shared(SharedFn, AnyArc),
}

/// `struct timeout`. Not `Unpin`: it is linked into the wheel's queues through `Pin<&Self>`
/// and must stay where it is while it is pending (a `static`, a pool item, `core::pin::pin!`).
pub struct Timeout {
    /// `to_list`: the queue entry (`hal::pinlist`).
    entry: PinEntry<Timeout>,
    /// \[T\] `to_func`, `to_kclock`, `to_abstime`.
    st: Guarded<ToState>,
    /// `to_arg`: the function's argument, written under the wheel's lock.
    arg: AtomicPtr<c_void>,
    /// \[T\] `to_time`: `ticks` on event, written under the wheel's lock.
    time: AtomicI32,
    /// \[T\] `to_flags`: written under the wheel's lock, read without it by the C's macros.
    flags: AtomicU32,
}

hal::pin_adapter!(
    /// The wheel's queues, through `Timeout::entry`.
    ToList: Timeout, entry
);

impl Timeout {
    /// `TIMEOUT_INITIALIZER_FLAGS(func, arg, kclock, flags)`.
    pub const fn new_flags(func: TimeoutFn, arg: *mut c_void, kclock: i32, flags: u32) -> Self {
        Self {
            entry: PinEntry::new(),
            st: Guarded::new_static_mutexed(
                &WHEEL,
                ToState {
                    func: Some(Func::Raw(func)),
                    kclock,
                    abstime: Timespec::ZERO,
                    shared: None,
                },
            ),
            arg: AtomicPtr::new(arg),
            time: AtomicI32::new(0),
            flags: AtomicU32::new(flags | TIMEOUT_INITIALIZED),
        }
    }

    /// `TIMEOUT_INITIALIZER(func, arg)`.
    pub const fn new(func: TimeoutFn, arg: *mut c_void) -> Self {
        Self::new_flags(func, arg, KCLOCK_NONE, 0)
    }

    /// A timeout `timeout_set` has not seen (all zero, a `static struct timeout`): not
    /// initialised, `to_kclock` 0.
    pub const fn unset() -> Self {
        Self {
            entry: PinEntry::new(),
            st: Guarded::new_static_mutexed(
                &WHEEL,
                ToState {
                    func: None,
                    kclock: 0,
                    abstime: Timespec::ZERO,
                    shared: None,
                },
            ),
            arg: AtomicPtr::new(ptr::null_mut()),
            time: AtomicI32::new(0),
            flags: AtomicU32::new(0),
        }
    }

    /// `timeout_set_flags(to, func, arg, kclock, flags)` (`func` `None`: the C's `NULL`, which
    /// `usbd_alloc_xfer` sets). The C writes the fields bare, trusting the caller not to set a
    /// pending timeout; here they are written under the wheel's lock, and the queued state
    /// (`TIMEOUT_ONQUEUE`) survives, so a set on a pending timeout leaves it pending and
    /// deletable: `TIMEOUT_PROC`, `TIMEOUT_MPSAFE`, the clock, the function and the argument
    /// change, and `TIMEOUT_TRIGGERED` is cleared as the C's whole store clears it.
    pub fn set(&self, func: Option<TimeoutFn>, arg: *mut c_void, kclock: i32, flags: u32) {
        let old = self.set_func(func.map(Func::Raw), None, arg, kclock, flags);
        drop(old);
    }

    /// `timeout_set_flags(to, func, arg, kclock, flags)` with a counted argument (the
    /// module's redesign): `func` is called with `arg`, which the timeout holds until it is
    /// set again or [cleared](Timeout::clear). A timeout that holds a handle to the object
    /// that holds it is a cycle its owner breaks with `clear` (after `del_barrier`), as the C's
    /// owner frees the object only once its timeout is gone.
    pub fn set_shared(&self, func: SharedFn, arg: AnyArc, kclock: i32, flags: u32) {
        let old = self.set_func(
            Some(Func::Shared(func)),
            Some(arg),
            ptr::null_mut(),
            kclock,
            flags,
        );
        drop(old);
    }

    /// `timeout_set(to, NULL, NULL)` of a timeout set by [`Timeout::set_shared`]: its
    /// argument given back (dropped after the wheel's lock), its function gone. For its owner
    /// once the timeout is neither pending nor running (`del_barrier`); a cleared timeout that
    /// runs calls nothing.
    pub fn clear(&self) {
        let old = {
            let mut g = WHEEL.lock();
            let st = self.st.get_mut(&mut g);
            st.func = None;
            st.shared.take()
        };
        drop(old);
    }

    /// [`Timeout::set`]'s work under the wheel's lock: the argument it held before comes back,
    /// for its caller to drop after the lock (its last drop may free a value whose own timeout
    /// takes the lock).
    fn set_func(
        &self,
        func: Option<Func>,
        shared: Option<AnyArc>,
        arg: *mut c_void,
        kclock: i32,
        flags: u32,
    ) -> Option<AnyArc> {
        set_checks(kclock, flags);
        let mut g = WHEEL.lock();
        let st = self.st.get_mut(&mut g);
        st.func = func;
        st.kclock = kclock;
        let old = core::mem::replace(&mut st.shared, shared);
        self.arg.store(arg, Ordering::Relaxed);
        let state = self.flags.load(Ordering::Relaxed) & TIMEOUT_ONQUEUE;
        self.flags
            .store(flags | TIMEOUT_INITIALIZED | state, Ordering::Relaxed);
        old
    }

    /// `timeout_set_flags` of a fresh `struct timeout` (all its state the new one's), over a
    /// timeout that may carry a previous user's state: the legacy adapter's node taken back
    /// for a wrapper made again at the same address (`sys/sys/timeout.rs`). A node the
    /// previous wrapper left pending is taken off its queue without counting a deletion (the
    /// C's `timeout_set` touches no counter; `tos_pending` follows, as it counts what is
    /// queued), and its flags, clock, function, argument, `to_time` and `to_abstime` are the
    /// fresh ones: one lock, as `set`.
    pub fn set_fresh(&self, func: Option<TimeoutFn>, arg: *mut c_void, kclock: i32, flags: u32) {
        set_checks(kclock, flags);
        let mut g = WHEEL.lock();
        if self.flags.load(Ordering::Relaxed) & TIMEOUT_ONQUEUE != 0 {
            PinRing::<ToList>::remove(&mut g, self);
            g.stat.tos_pending -= 1;
        }
        let st = self.st.get_mut(&mut g);
        st.func = func.map(Func::Raw);
        st.kclock = kclock;
        st.abstime = Timespec::ZERO;
        let old = st.shared.take();
        self.arg.store(arg, Ordering::Relaxed);
        self.time.store(0, Ordering::Relaxed);
        self.flags
            .store(flags | TIMEOUT_INITIALIZED, Ordering::Relaxed);
        drop(g);
        drop(old);
    }

    /// `timeout_add(to, to_ticks)`: due in `to_ticks` ticks; `true` when it was not pending
    /// (the C's 1). A pending timeout moved earlier is rescheduled now, one moved later where
    /// it is (it is rescheduled when its bucket comes).
    pub fn add(self: Pin<&Self>, to_ticks: i32) -> bool {
        kassert!(self.initialized());
        kassert!(to_ticks >= 0);

        let mut g = WHEEL.lock();
        kassert!(self.st.get(&g).kclock == KCLOCK_NONE);

        // Initialize the time here, it won't change.
        let now = clock::ticks();
        let old_time = self.time.load(Ordering::Relaxed);
        let new_time = to_ticks.wrapping_add(now);
        self.time.store(new_time, Ordering::Relaxed);
        let f = self.flags.load(Ordering::Relaxed) & !TIMEOUT_TRIGGERED;

        let ret = if f & TIMEOUT_ONQUEUE != 0 {
            // Scheduled already: reschedule now only when moved earlier.
            if new_time.wrapping_sub(now) < old_time.wrapping_sub(now) {
                PinRing::<ToList>::remove(&mut g, &self);
                ring(&QUEUES.new).insert_tail(&mut g, self);
            }
            self.flags.store(f, Ordering::Relaxed);
            g.stat.tos_readded += 1;
            false
        } else {
            self.flags.store(f | TIMEOUT_ONQUEUE, Ordering::Relaxed);
            ring(&QUEUES.new).insert_tail(&mut g, self);
            g.stat.tos_pending += 1;
            true
        };
        g.stat.tos_added += 1;
        ret
    }

    /// `timeout_add_ticks`: adds one tick so that at least `to_ticks` have elapsed when the
    /// timeout runs (it runs in the clock interrupt after the one that makes `ticks` reach its
    /// time, see the C's comment).
    fn add_ticks(self: Pin<&Self>, to_ticks: u64) -> bool {
        let to_ticks = if to_ticks >= i32::MAX as u64 {
            i32::MAX
        } else {
            (to_ticks + 1) as i32
        };
        self.add(to_ticks)
    }

    /// `timeout_add_sec(to, secs)`.
    pub fn add_sec(self: Pin<&Self>, secs: i32) -> bool {
        kassert!(secs >= 0);
        // secs is a 31bit int, so this can't overflow 64bits
        let to_ticks = HZ.load(Ordering::Relaxed) as u64 * secs as u64;
        self.add_ticks(to_ticks)
    }

    /// `timeout_add_msec(to, msecs)`: at least `msecs`, rounded up (overflow: as long as
    /// possible).
    pub fn add_msec(self: Pin<&Self>, msecs: u64) -> bool {
        if msecs >= u64::MAX / 1000 {
            return self.add(i32::MAX);
        }
        self.add_usec(msecs * 1000)
    }

    /// `timeout_add_usec(to, usecs)`.
    pub fn add_usec(self: Pin<&Self>, usecs: u64) -> bool {
        let tick = TICK.load(Ordering::Relaxed) as u64;
        if usecs >= u64::MAX - tick {
            return self.add(i32::MAX);
        }
        self.add_ticks(usecs.div_ceil(tick))
    }

    /// `timeout_add_nsec(to, nsecs)`.
    pub fn add_nsec(self: Pin<&Self>, nsecs: u64) -> bool {
        let tick_nsec = TICK_NSEC.load(Ordering::Relaxed) as u64;
        if nsecs >= u64::MAX - tick_nsec {
            return self.add(i32::MAX);
        }
        self.add_ticks(nsecs.div_ceil(tick_nsec))
    }

    /// `timeout_abs_ts(to, abstime)`: due at the uptime `abstime` (`KCLOCK_UPTIME`); `true`
    /// when it was not pending.
    pub fn abs_ts(self: Pin<&Self>, abstime: &Timespec) -> bool {
        let mut g = WHEEL.lock();

        kassert!(self.initialized());
        kassert!(self.st.get(&g).kclock == KCLOCK_UPTIME);

        let st = self.st.get_mut(&mut g);
        let old_abstime = st.abstime;
        st.abstime = *abstime;
        let f = self.flags.load(Ordering::Relaxed) & !TIMEOUT_TRIGGERED;

        let ret = if f & TIMEOUT_ONQUEUE != 0 {
            if *abstime < old_abstime {
                PinRing::<ToList>::remove(&mut g, &self);
                ring(&QUEUES.new).insert_tail(&mut g, self);
            }
            self.flags.store(f, Ordering::Relaxed);
            g.stat.tos_readded += 1;
            false
        } else {
            self.flags.store(f | TIMEOUT_ONQUEUE, Ordering::Relaxed);
            ring(&QUEUES.new).insert_tail(&mut g, self);
            g.stat.tos_pending += 1;
            true
        };
        g.stat.tos_added += 1;
        ret
    }

    /// `timeout_del(to)`: off its queue; `true` when it was pending.
    pub fn del(&self) -> bool {
        let mut g = WHEEL.lock();
        let mut f = self.flags.load(Ordering::Relaxed);
        let ret = if f & TIMEOUT_ONQUEUE != 0 {
            PinRing::<ToList>::remove(&mut g, self);
            g.stat.tos_pending -= 1;
            f &= !TIMEOUT_ONQUEUE;
            g.stat.tos_cancelled += 1;
            true
        } else {
            false
        };
        self.flags.store(f & !TIMEOUT_TRIGGERED, Ordering::Relaxed);
        g.stat.tos_deleted += 1;
        ret
    }

    /// `timeout_del_barrier(to)`: deleted, then [`Timeout::barrier`]: when it returns the
    /// timeout is neither pending nor running.
    pub fn del_barrier(&self) -> bool {
        // timeout_sync_order: WITNESS.
        let removed = self.del();
        self.barrier();
        removed
    }

    /// `timeout_barrier(to)`: when `to` is running in its context, waits until it is done (a
    /// timeout of its own queued at the head of that context's queue, run right after it).
    pub fn barrier(&self) {
        let flags = self.flags.load(Ordering::Relaxed) & (TIMEOUT_PROC | TIMEOUT_MPSAFE);
        // timeout_sync_order: WITNESS.

        let barrier = core::pin::pin!(Timeout::new_flags(
            barrier_signal,
            ptr::null_mut(),
            KCLOCK_NONE,
            flags
        ));
        let barrier = barrier.into_ref();
        // Its argument is its own address, the waiter's channel: compared, never followed.
        barrier.arg.store(
            ptr::from_ref(&*barrier).cast_mut().cast(),
            Ordering::Relaxed,
        );

        let mut g = WHEEL.lock();
        let ctx = if flags & TIMEOUT_PROC != 0 {
            Ctx::proc_of(flags)
        } else {
            Ctx::Si
        };
        if g.running[ctx as usize] != ptr::from_ref(self).addr() {
            return;
        }

        barrier.time.store(clock::ticks(), Ordering::Relaxed);
        barrier.flags.store(
            flags | TIMEOUT_INITIALIZED | TIMEOUT_ONQUEUE,
            Ordering::Relaxed,
        );
        ring(ctx.todo()).insert_head(&mut g, barrier);
        g.stat.tos_pending += 1;
        drop(g);

        // We know the relevant timeout context was running something and now also has the
        // barrier to run, so we just have to wait for it to pick up the barrier now.
        let chan = ptr::from_ref(&*barrier);
        loop {
            hal::sleep::sleep_setup(chan, PWAIT, "tmobar");
            let wait = !barrier.barrier_ran();
            let _ = hal::sleep::sleep_finish(INFSLP, wait);
            if !wait {
                break;
            }
        }
    }

    /// `timeout_pending(to)`: is it scheduled to run? (Read without the lock, as the C's
    /// macro.)
    pub fn pending(&self) -> bool {
        self.flags.load(Ordering::Relaxed) & TIMEOUT_ONQUEUE != 0
    }

    /// `timeout_initialized(to)`.
    pub fn initialized(&self) -> bool {
        self.flags.load(Ordering::Relaxed) & TIMEOUT_INITIALIZED != 0
    }

    /// `timeout_triggered(to)`: is it running, or did it run?
    pub fn triggered(&self) -> bool {
        self.flags.load(Ordering::Relaxed) & TIMEOUT_TRIGGERED != 0
    }

    /// A barrier timeout's condition (`c_wait` cleared): its context took it, so
    /// `TIMEOUT_TRIGGERED` is set, and it is off its queue, its entry released (both under the
    /// wheel's lock, the release last). `TIMEOUT_TRIGGERED` alone is set while the node is still
    /// linked, so a waiter that returned on it could drop the node in that window, and the
    /// drop's `del` would count a spurious `tos_deleted`.
    fn barrier_ran(&self) -> bool {
        self.triggered() && !self.entry.is_linked()
    }

    /// `to->to_time`: the ticks value it is due at (the readers that print or compare it).
    pub fn time(&self) -> i32 {
        self.time.load(Ordering::Relaxed)
    }
}

/// `timeout_set_flags`'s `KASSERT`s: known flags, a known clock, `TIMEOUT_MPSAFE` only with
/// `TIMEOUT_PROC` ("for now, only process context timeouts may be marked MP-safe").
#[inline]
fn set_checks(kclock: i32, flags: u32) {
    kassert!(flags & !(TIMEOUT_PROC | TIMEOUT_MPSAFE) == 0);
    kassert!((KCLOCK_NONE..KCLOCK_MAX).contains(&kclock));
    if flags & TIMEOUT_MPSAFE != 0 {
        kassert!(flags & TIMEOUT_PROC != 0);
    }
}

impl Drop for Timeout {
    /// A pending timeout is deleted before it goes (the module's deviations); then its entry,
    /// unlinked, drops quietly.
    fn drop(&mut self) {
        if self.entry.is_linked() {
            self.del();
        }
    }
}

/// `struct timeoutstat`: statistics and totals, `[T]`.
#[derive(Clone, Copy, Default)]
struct Timeoutstat {
    /// `tos_added`: `timeout_add*(9)` calls.
    tos_added: u64,
    /// `tos_cancelled`: dequeued during `timeout_del*(9)`.
    tos_cancelled: u64,
    /// `tos_deleted`: `timeout_del*(9)` calls.
    tos_deleted: u64,
    /// `tos_late`: run after deadline.
    tos_late: u64,
    /// `tos_pending`: number currently ONQUEUE.
    tos_pending: u64,
    /// `tos_readded`: `timeout_add*(9)` + already ONQUEUE.
    tos_readded: u64,
    /// `tos_rescheduled`: bucketed + already SCHEDULED.
    tos_rescheduled: u64,
    /// `tos_run_softclock`: run from `softclock()`.
    tos_run_softclock: u64,
    /// `tos_run_thread`: run from `softclock_thread()`.
    tos_run_thread: u64,
    /// `tos_scheduled`: bucketed during `softclock()`.
    tos_scheduled: u64,
    /// `tos_softclocks`: `softclock()` calls.
    tos_softclocks: u64,
    /// `tos_thread_wakeups`: wakeups in `softclock_thread()`.
    tos_thread_wakeups: u64,
}

impl Timeoutstat {
    const fn new() -> Self {
        Self {
            tos_added: 0,
            tos_cancelled: 0,
            tos_deleted: 0,
            tos_late: 0,
            tos_pending: 0,
            tos_readded: 0,
            tos_rescheduled: 0,
            tos_run_softclock: 0,
            tos_run_thread: 0,
            tos_scheduled: 0,
            tos_softclocks: 0,
            tos_thread_wakeups: 0,
        }
    }

    /// The twelve counters in the C's `struct timeoutstat` order (`kern.timeout_stats`).
    const fn as_array(&self) -> [u64; 12] {
        [
            self.tos_added,
            self.tos_cancelled,
            self.tos_deleted,
            self.tos_late,
            self.tos_pending,
            self.tos_readded,
            self.tos_rescheduled,
            self.tos_run_softclock,
            self.tos_run_thread,
            self.tos_scheduled,
            self.tos_softclocks,
            self.tos_thread_wakeups,
        ]
    }
}

/// `struct kclock`: a kernel clock's cached scan times, `[T]`.
#[derive(Clone, Copy)]
struct Kclock {
    /// `kc_lastscan`: clock time at last wheel scan.
    lastscan: Timespec,
    /// `kc_late`: late if due prior.
    late: Timespec,
    /// `kc_offset`: offset from primary kclock.
    offset: Timespec,
}

/// A context that runs timeouts (`struct timeout_ctx`): the soft interrupt, or a thread.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ctx {
    /// `timeout_ctx_si`: `softclock`.
    Si = 0,
    /// `timeout_ctx_proc`: the `softclock` thread.
    Proc = 1,
    /// `timeout_ctx_proc_mp`: the `softclockmp` thread.
    ProcMp = 2,
}

impl Ctx {
    /// `tctx_todo`: the context's queue.
    fn todo(self) -> &'static PinRing<ToList> {
        match self {
            Ctx::Si => &QUEUES.todo,
            Ctx::Proc => &QUEUES.proc_q,
            Ctx::ProcMp => &QUEUES.proc_mp,
        }
    }

    /// The thread context of a `TIMEOUT_PROC` timeout with `flags`.
    fn proc_of(flags: u32) -> Ctx {
        if flags & TIMEOUT_MPSAFE != 0 {
            Ctx::ProcMp
        } else {
            Ctx::Proc
        }
    }
}

/// What `timeout_mutex` keeps besides the queues: the C's `[T]` globals.
struct WheelState {
    /// `tctx_running` of each [`Ctx`]: the running timeout's address, 0 for none.
    running: [usize; 3],
    /// `timeout_kclock[KCLOCK_MAX]`.
    kclock: [Kclock; KCLOCK_MAX as usize],
    /// `tostat`.
    stat: Timeoutstat,
    /// \[I\] `tick_ts`: the length of a tick.
    tick_ts: Timespec,
}

impl WheelState {
    const fn new() -> Self {
        Self {
            running: [0; 3],
            kclock: [Kclock {
                lastscan: Timespec::ZERO,
                late: Timespec::ZERO,
                offset: Timespec::ZERO,
            }; KCLOCK_MAX as usize],
            stat: Timeoutstat::new(),
            tick_ts: Timespec::ZERO,
        }
    }
}

/// The wheel's queues, bound to [`WHEEL`]'s lock on first use (`[T]`).
struct Queues {
    /// `timeout_wheel`: tick-based timeouts.
    wheel: [PinRing<ToList>; BUCKETS],
    /// `timeout_wheel_kc`: clock-based timeouts.
    wheel_kc: [PinRing<ToList>; BUCKETS],
    /// `timeout_new`: new, unscheduled timeouts.
    new: PinRing<ToList>,
    /// `timeout_todo`: due or needs rescheduling.
    todo: PinRing<ToList>,
    /// `timeout_proc`: due and needs a process context.
    proc_q: PinRing<ToList>,
    /// `timeout_proc_mp`: process context and no kernel lock.
    proc_mp: PinRing<ToList>,
}

/// Where `softclock` sends a timeout it took off `todo`.
enum Dest {
    /// Onto this queue (a bucket, or a thread's).
    Queue(&'static PinRing<ToList>),
    /// Run now, in `softclock`.
    Run,
}

/// `timeout_mutex` and the `[T]` globals it keeps.
static WHEEL: Mutexed<WheelState> = Mutexed::new(IPL_HIGH, WheelState::new());

/// The queues (`static`: their heads never move, `pin_static`).
static QUEUES: Queues = Queues {
    wheel: [const { PinRing::new() }; BUCKETS],
    wheel_kc: [const { PinRing::new() }; BUCKETS],
    new: PinRing::new(),
    todo: PinRing::new(),
    proc_q: PinRing::new(),
    proc_mp: PinRing::new(),
};

/// \[I\] `softclock_si`: the `softclock()` soft interrupt.
static SOFTCLOCK_SI: OnceStatic<SoftHandle> = OnceStatic::new();

/// \[I\] `hz`, `tick` (microseconds) and `tick_nsec`, as [`startup`] was given them (the C's
/// `param.c` defaults until then).
static HZ: AtomicI32 = AtomicI32::new(100);
/// \[I\] `tick`.
static TICK: AtomicI32 = AtomicI32::new(10_000);
/// \[I\] `tick_nsec`.
static TICK_NSEC: AtomicI32 = AtomicI32::new(10_000_000);

/// `hz`: the clock ticks per second (`kernel::clock::tstohz`).
pub fn hz() -> i32 {
    HZ.load(Ordering::Relaxed)
}

/// `tick_nsec`: the nanoseconds per clock tick (`kernel::tc::tc_setclock`).
pub fn tick_nsec() -> i32 {
    TICK_NSEC.load(Ordering::Relaxed)
}

/// `tick`: the microseconds per clock tick (`kernel::clock::tstohz`).
pub fn tick() -> i32 {
    TICK.load(Ordering::Relaxed)
}

/// A wheel queue, pinned: the queues are a `static`.
#[inline]
fn ring(r: &'static PinRing<ToList>) -> Pin<&'static PinRing<ToList>> {
    pin_static(r)
}

/// `MASKWHEEL(wheel, time)`.
#[inline]
const fn maskwheel(wheel: u32, time: i32) -> usize {
    (((time as u32) >> (wheel * WHEELBITS)) & WHEELMASK) as usize
}

/// `BUCKET(rel, abs)`: the tick wheel's bucket for a timeout `rel` ticks away, due at `abs`.
#[inline]
fn bucket(rel: i32, abs: i32) -> &'static PinRing<ToList> {
    let idx = if rel <= (1 << (2 * WHEELBITS)) {
        if rel <= (1 << WHEELBITS) {
            maskwheel(0, abs)
        } else {
            maskwheel(1, abs) + WHEELSIZE
        }
    } else if rel <= (1 << (3 * WHEELBITS)) {
        maskwheel(2, abs) + 2 * WHEELSIZE
    } else {
        maskwheel(3, abs) + 3 * WHEELSIZE
    };
    &QUEUES.wheel[idx]
}

/// `todo` with the wheel's lock, bound once ([`PinRing::bound`]).
type Todo<'r, 'g> = BoundRing<'r, ToList, MutexedGuard<'g, WheelState>>;

/// `MOVEBUCKET(wheel, time)`: the level's bucket for `time` appended to `todo`.
#[inline]
fn movebucket(todo: &mut Todo<'_, '_>, wheel: u32, time: i32) {
    todo.concat(&QUEUES.wheel[maskwheel(wheel, time) + wheel as usize * WHEELSIZE]);
}

/// `timeout_startup()`: `hz`, `tick` and `tick_nsec` as the machine set them, and `tick_ts`
/// from the last. The queues need no initialisation (`CIRCQ_INIT`: an empty ring's links mean
/// "itself"), nor `timeout_level_width` (a constant).
pub fn startup(hz: i32, tick: i32, tick_nsec: i32) {
    HZ.store(hz, Ordering::Relaxed);
    TICK.store(tick, Ordering::Relaxed);
    TICK_NSEC.store(tick_nsec, Ordering::Relaxed);
    WHEEL.lock().tick_ts = Timespec::from_nsec(tick_nsec as u64);
}

/// `timeout_proc_init()`: the `softclock` soft interrupt established, and the creation of the
/// `softclock` threads deferred until threads can be made. Panics, as the C, when the soft
/// interrupt cannot be had.
pub fn proc_init() {
    let si = match hal::softintr::establish(IPL_SOFTCLOCK, softclock_si, core::ptr::null_mut()) {
        Some(si) => si,
        None => kpanic(format_args!(
            "timeout_proc_init: unable to register softclock interrupt"
        )),
    };
    if SOFTCLOCK_SI.set(si).is_err() {
        kpanic(format_args!("timeout_proc_init: called twice"));
    }

    // WITNESS_INIT: not configured.

    thread::create_deferred(Start::SoftclockCreate);
}

/// `timeout_bucket(to)`: the clock wheel's bucket for a timeout due at `abstime`, past the
/// clock's last scan.
fn timeout_bucket(kc: &Kclock, abstime: &Timespec) -> &'static PinRing<ToList> {
    kassert!(kc.lastscan < *abstime);
    let diff = abstime.sub(kc.lastscan);
    let mut level = 0;
    while level < WHEELCOUNT - 1 {
        if diff.tv_sec < TIMEOUT_LEVEL_WIDTH[level] {
            break;
        }
        level += 1;
    }
    let shifted_abstime = abstime.add(kc.offset);
    &QUEUES.wheel_kc[level * WHEELSIZE + timeout_maskwheel(level as u32, &shifted_abstime) as usize]
}

/// `timeout_maskwheel(level, abstime)`: hashes the absolute time into a bucket on a given
/// level of the wheel. The complete hash is 32 bits: the upper 25 bits are seconds, the lower
/// 7 bits nanoseconds (`tv_nsec` is below one billion, so it is divided to isolate the desired
/// bits, not shifted); the level isolates an 8-bit portion of it.
pub fn timeout_maskwheel(level: u32, abstime: &Timespec) -> u32 {
    let hi = (abstime.tv_sec as u32) << 7;
    let lo = (abstime.tv_nsec / 7_812_500) as u32;

    ((hi | lo) >> (level * WHEELBITS)) & WHEELMASK
}

/// `timeout_hardclock_update()`: called by `hardclock` on the primary CPU at the start of every
/// tick. Moves the tick buckets that came due and the clock buckets that expired since the last
/// scan onto `todo`, and schedules `softclock` when there is work.
pub fn hardclock_update() {
    let mut g = WHEEL.lock();
    // `todo`'s binding to the lock is checked once for every bucket this tick takes.
    let mut todo = ring(&QUEUES.todo).bound(&mut g);

    let t = clock::ticks();
    movebucket(&mut todo, 0, t);
    if maskwheel(0, t) == 0 {
        movebucket(&mut todo, 1, t);
        if maskwheel(1, t) == 0 {
            movebucket(&mut todo, 2, t);
            if maskwheel(2, t) == 0 {
                movebucket(&mut todo, 3, t);
            }
        }
    }

    // Dump the buckets that expired while we were away.
    //
    // If the elapsed time has exceeded a level's limit then we need to dump every bucket in
    // the level. We have necessarily completed a lap of that level, too, so we need to process
    // buckets in the next level.
    //
    // Otherwise we need to compare indices: if the index of the first expired bucket is
    // greater than that of the last then we have completed a lap of the level and need to
    // process buckets in the next level.
    let now = clock::nanouptime();
    let lastscan = todo.witness().kclock[KCLOCK_UPTIME as usize].lastscan;
    let elapsed = now.sub(lastscan);
    for (level, &width) in TIMEOUT_LEVEL_WIDTH.iter().enumerate() {
        let first = timeout_maskwheel(level as u32, &lastscan) as usize;
        let (last, done) = if elapsed.tv_sec >= width {
            (if first == 0 { WHEELSIZE - 1 } else { first - 1 }, false)
        } else {
            let last = timeout_maskwheel(level as u32, &now) as usize;
            (last, first <= last)
        };
        let off = level * WHEELSIZE;
        let mut b = first;
        loop {
            todo.concat(&QUEUES.wheel_kc[off + b]);
            if b == last {
                break;
            }
            b = (b + 1) % WHEELSIZE;
        }
        if done {
            break;
        }
    }

    // Update the cached state for each kclock.
    let w = todo.data_mut();
    let tick_ts = w.tick_ts;
    for kc in w.kclock.iter_mut() {
        kc.lastscan = now.add(kc.offset);
        kc.late = kc.lastscan.sub(tick_ts);
    }

    let need_softclock = !(QUEUES.new.is_empty(todo.witness()) && todo.first().is_none());
    drop(g);

    if need_softclock && let Some(si) = SOFTCLOCK_SI.get() {
        si.schedule();
    }
}

/// `timeout_run(tctx, to)`, the part after the timeout left its queue (`TIMEOUT_ONQUEUE`
/// cleared and `TIMEOUT_TRIGGERED` set while it was taken, its function and argument copied
/// then): recorded as running, its function called with the lock released (it may add or
/// delete itself), the lock taken again, which is returned.
fn run_taken<'w>(
    mut g: MutexedGuard<'w, WheelState>,
    ctx: Ctx,
    call: Option<Call>,
    addr: usize,
) -> MutexedGuard<'w, WheelState> {
    // needsproc: WITNESS only. NKCOV > 0: kcov_process.
    g.running[ctx as usize] = addr;
    drop(g);
    // timeout_sync_enter(needsproc): WITNESS.
    match call {
        Some(Call::Raw(func, arg)) => func(arg),
        // The clone goes before the lock is taken again: its drop may free the value that
        // holds this timeout, whose drop takes the lock.
        Some(Call::Shared(func, arg)) => func(&arg),
        None => {}
    }
    // timeout_sync_leave(needsproc): WITNESS.
    let mut g = WHEEL.lock();
    g.running[ctx as usize] = 0;
    g
}

/// The first timeout of `ctx`'s queue taken off it and run ([`run_taken`]): the guard back,
/// `false` when the queue was empty (a thread's loop).
fn run_first<'w>(
    mut g: MutexedGuard<'w, WheelState>,
    ctx: Ctx,
) -> (MutexedGuard<'w, WheelState>, bool) {
    // The context's queue bound once for the look and the removal.
    let mut q = ring(ctx.todo()).bound(&mut g);
    let Some((call, addr)) = q.first().map(|to| {
        let flags = to.flags.load(Ordering::Relaxed);
        take_for_run(to, flags, to.st.get(q.witness()))
    }) else {
        return (g, false);
    };
    q.remove_first();
    q.data_mut().stat.tos_pending -= 1;
    (run_taken(g, ctx, call, addr), true)
}

/// A timeout about to run, still first on its queue (its `flags` and its state read under the
/// lock by the caller, once): `TIMEOUT_ONQUEUE` cleared, `TIMEOUT_TRIGGERED` set (under the
/// lock, as the C); what to call (its function and argument, a shared argument cloned) and
/// its address.
#[inline]
fn take_for_run(to: &Timeout, flags: u32, st: &ToState) -> (Option<Call>, usize) {
    to.flags.store(
        (flags & !TIMEOUT_ONQUEUE) | TIMEOUT_TRIGGERED,
        Ordering::Relaxed,
    );
    let call = match st.func {
        Some(Func::Raw(func)) => Some(Call::Raw(func, to.arg.load(Ordering::Relaxed))),
        Some(Func::Shared(func)) => st.shared.clone().map(|arg| Call::Shared(func, arg)),
        None => None,
    };
    (call, ptr::from_ref(to).addr())
}

/// What `softclock`'s processing of one timeout adds to `tostat`.
#[derive(Default)]
struct Bumps {
    scheduled: bool,
    rescheduled: bool,
    late: bool,
}

impl Bumps {
    fn apply(&self, s: &mut Timeoutstat) {
        s.tos_scheduled += u64::from(self.scheduled);
        s.tos_rescheduled += u64::from(self.rescheduled);
        s.tos_late += u64::from(self.late);
    }
}

/// `softclock_process_kclock_timeout(to, new)`: where a clock-based timeout due at `abstime`
/// goes, given the clock's cached scan times.
fn process_kclock(kc: &Kclock, abstime: &Timespec, flags: u32, new: bool, b: &mut Bumps) -> Dest {
    if *abstime > kc.lastscan {
        b.scheduled = true;
        b.rescheduled = !new;
        return Dest::Queue(timeout_bucket(kc, abstime));
    }
    b.late = !new && *abstime <= kc.late;
    if flags & TIMEOUT_PROC != 0 {
        return Dest::Queue(Ctx::proc_of(flags).todo());
    }
    Dest::Run
}

/// `softclock_process_tick_timeout(to, new)`: where a tick-based timeout due at `time` goes.
fn process_tick(time: i32, flags: u32, new: bool, b: &mut Bumps) -> Dest {
    let delta = time.wrapping_sub(clock::ticks());

    if delta > 0 {
        b.scheduled = true;
        b.rescheduled = !new;
        return Dest::Queue(bucket(delta, time));
    }
    b.late = !new && delta < 0;
    if flags & TIMEOUT_PROC != 0 {
        return Dest::Queue(Ctx::proc_of(flags).todo());
    }
    Dest::Run
}

/// `softclock(NULL)` as the soft interrupt calls it (its argument unused).
fn softclock_si(_arg: *mut core::ffi::c_void) {
    softclock();
}

/// `softclock()`: the soft interrupt's work, at `IPL_SOFTCLOCK` rather than in
/// [`hardclock_update`], so other interrupts are served promptly through a surge of timeouts.
/// Every timeout of `new` and `todo` goes to its bucket, to a thread, or runs.
pub fn softclock() {
    let mut g = WHEEL.lock();
    let first_new = QUEUES.new.first(&g).map(|to| ptr::from_ref(to).addr());
    ring(&QUEUES.todo).concat(&mut g, &QUEUES.new);
    let mut new = false;
    loop {
        // One look at the first timeout: where it goes, and when it runs here, what to run (the
        // C's `CIRCQ_REMOVE` then `softclock_process_*_timeout` then `timeout_run`); `todo`'s
        // binding checked once for both.
        let mut todo = ring(&QUEUES.todo).bound(&mut g);
        let mut bumps = Bumps::default();
        let Some((dest, run)) = todo.first().map(|to| {
            if Some(ptr::from_ref(to).addr()) == first_new {
                new = true;
            }
            let w = todo.witness();
            let st = to.st.get(w);
            let flags = to.flags.load(Ordering::Relaxed);
            let dest = match st.kclock {
                KCLOCK_NONE => {
                    process_tick(to.time.load(Ordering::Relaxed), flags, new, &mut bumps)
                }
                KCLOCK_UPTIME => process_kclock(
                    &w.kclock[KCLOCK_UPTIME as usize],
                    &st.abstime,
                    flags,
                    new,
                    &mut bumps,
                ),
                other => kpanic(format_args!("softclock: invalid to_clock: {other}")),
            };
            let run = matches!(dest, Dest::Run).then(|| take_for_run(to, flags, st));
            (dest, run)
        }) else {
            break;
        };
        bumps.apply(&mut todo.data_mut().stat);
        match (dest, run) {
            (Dest::Queue(q), _) => {
                todo.move_first(ring(q));
            }
            (Dest::Run, Some((call, addr))) => {
                todo.remove_first();
                g.stat.tos_pending -= 1;
                g = run_taken(g, Ctx::Si, call, addr);
                g.stat.tos_run_softclock += 1;
            }
            (Dest::Run, None) => {}
        }
    }
    g.stat.tos_softclocks += 1;
    let needsproc = !QUEUES.proc_q.is_empty(&g);
    let need_proc_mp = !QUEUES.proc_mp.is_empty(&g);
    drop(g);

    if needsproc {
        hal::sleep::wakeup(ptr::from_ref(Ctx::Proc.todo()));
    }
    if need_proc_mp {
        hal::sleep::wakeup(ptr::from_ref(Ctx::ProcMp.todo()));
    }
}

/// `softclock_create_thread()`: the two threads, once threads can be made (deferred by
/// [`proc_init`]).
pub(crate) fn softclock_create_thread() {
    if thread::create(b"softclock", Start::Softclock).is_err() {
        kpanic(format_args!("fork softclock"));
    }
    if thread::create(b"softclockmp", Start::SoftclockMp).is_err() {
        kpanic(format_args!("kthread_create softclock_thread_mp"));
    }
}

/// `softclock_thread_run(tctx)`: sleeps on the context's queue and runs what `softclock` put
/// there.
fn softclock_thread_run(ctx: Ctx) -> ! {
    let todo = ctx.todo();
    loop {
        // Avoid holding both timeout_mutex and SCHED_LOCK at the same time: the queue is
        // looked at between sleep_setup and sleep_finish, each of which takes and releases
        // the scheduler's lock (the C reads it bare; here the wheel's lock is taken briefly).
        hal::sleep::sleep_setup(ptr::from_ref(todo), PSWP, "tmoslp");
        let empty = todo.is_empty(&WHEEL.lock());
        let _ = hal::sleep::sleep_finish(INFSLP, empty);

        let mut g = WHEEL.lock();
        g.stat.tos_thread_wakeups += 1;
        loop {
            let (ng, ran) = run_first(g, ctx);
            g = ng;
            if !ran {
                break;
            }
            g.stat.tos_run_thread += 1;
        }
        drop(g);
    }
}

/// `softclock_thread()`: runs the `TIMEOUT_PROC` timeouts, under the kernel lock, pegged to
/// the primary CPU ("be conservative for the moment"), at `splsoftclock`.
pub(crate) fn softclock_thread() -> ! {
    hal::klock::kernel_assert_locked();
    thread::peg_to_primary_cpu();
    let _s = hal::intr::splraise(IPL_SOFTCLOCK);
    softclock_thread_run(Ctx::Proc)
}

/// `softclock_thread_mp()`: runs the `TIMEOUT_PROC | TIMEOUT_MPSAFE` timeouts without the
/// kernel lock, on any CPU.
pub(crate) fn softclock_thread_mp() -> ! {
    hal::klock::kernel_assert_locked();
    hal::klock::kernel_unlock();
    softclock_thread_run(Ctx::ProcMp)
}

/// `timeout_adjust_ticks(adj)` (`!SMALL_KERNEL`): `ticks` moved `adj` forward after the clock
/// was stepped (`tc_setclock`); every tick-based timeout goes back through `softclock`, those
/// the step skipped due now. Never backwards.
pub fn adjust_ticks(adj: i32) {
    // adjusting the monotonic clock backwards would be a Bad Thing
    if adj <= 0 {
        return;
    }

    let mut g = WHEEL.lock();
    let now = clock::ticks();
    let new_ticks = now.wrapping_add(adj);
    for b in &QUEUES.wheel {
        while let Some(to) = b.first(&g) {
            // when moving a timeout forward need to reinsert it
            if to.time.load(Ordering::Relaxed).wrapping_sub(now) < adj {
                to.time.store(new_ticks, Ordering::Relaxed);
            }
            b.move_first(&mut g, ring(&QUEUES.todo));
        }
    }
    TICKS.store(new_ticks, Ordering::Relaxed);
}

/// `tostat`, copied under the lock (`timeout_sysctl`'s `memcpy`): the twelve counters of
/// `struct timeoutstat` in its order.
pub fn stats() -> [u64; 12] {
    WHEEL.lock().stat.as_array()
}

/// The barrier timeout's function (`cond_signal_handler`): its argument is its own address,
/// the waiter's channel; the waiter's condition is the barrier's `TIMEOUT_TRIGGERED` and its
/// unlinked entry, both set under the lock when the context took it.
fn barrier_signal(arg: *mut c_void) {
    hal::sleep::wakeup_one(arg.cast_const());
}

// The wheel is cheap to lock but hot: the hot paths' shape (decision 43) keeps one lock, one
// queue operation and the flag words per `timeout_add`/`timeout_del`.
const _: () = {
    assert!(TIMEOUT_LEVEL_WIDTH[0] == 2 && TIMEOUT_LEVEL_WIDTH[3] == 2 << 24);
    assert!(BUCKETS == 1024);
};

/// Test hooks: one hardclock's worth of wheel work (`ticks++`, the update, the soft
/// interrupt's work), as the tests and the host benches drive the clock by hand.
#[cfg(any(test, feature = "doubles"))]
pub fn tick_once() {
    TICKS.fetch_add(1, Ordering::Relaxed);
    hardclock_update();
    softclock();
}

/// Test hook, `timeout_test_reset` (testing.md tier 1): every queued timeout taken off its
/// queue without running, not pending any more (`tos_pending` follows), as LZ's
/// `timeout_startup` re-initialised every queue. The host tests share the wheel: a timeout
/// one left pending would otherwise run in the next one's `tick_once`, with an argument into
/// the earlier test's memory. Every queued timeout is alive (a pinned one is deleted by its
/// `Drop` before it goes, a legacy one is a pool item never given back), so it is touched
/// safely.
#[cfg(any(test, feature = "doubles"))]
pub fn timeout_test_reset() {
    let mut g = WHEEL.lock();
    let queues = QUEUES.wheel.iter().chain(QUEUES.wheel_kc.iter()).chain([
        &QUEUES.new,
        &QUEUES.todo,
        &QUEUES.proc_q,
        &QUEUES.proc_mp,
    ]);
    for q in queues {
        let mut b = ring(q).bound(&mut g);
        while b
            .first()
            .map(|to| {
                to.flags.fetch_and(!TIMEOUT_ONQUEUE, Ordering::Relaxed);
            })
            .is_some()
        {
            b.remove_first();
            let st = b.data_mut();
            st.stat.tos_pending = st.stat.tos_pending.saturating_sub(1);
        }
    }
}

/// Test hook: whether the wheel holds no timeout (every queue empty).
#[cfg(any(test, feature = "doubles"))]
pub fn wheel_is_empty() -> bool {
    let g = WHEEL.lock();
    QUEUES.wheel.iter().all(|q| q.is_empty(&g))
        && QUEUES.wheel_kc.iter().all(|q| q.is_empty(&g))
        && [&QUEUES.new, &QUEUES.todo, &QUEUES.proc_q, &QUEUES.proc_mp]
            .iter()
            .all(|q| q.is_empty(&g))
}

/// Test hook: the `TIMEOUT_PROC | TIMEOUT_MPSAFE` thread started once per process (the tests
/// that need a thread to run a timeout; the non-MP one stays down so `TIMEOUT_PROC` timeouts
/// stay queued for the tests that look at them).
#[cfg(any(test, feature = "doubles"))]
pub fn start_mp_thread_for_tests() {
    static ONCE: OnceStatic<()> = OnceStatic::new();
    if ONCE.set(()).is_ok() && thread::create(b"softclockmp", Start::SoftclockMp).is_err() {
        kpanic(format_args!("kthread_create softclock_thread_mp"));
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // The wheel is driven by hand: `ticks` and `hardclock_update` (`tick_once`), the soft
    // interrupt by calling `softclock`, as LZ's tests did. The wheel is global: the tests
    // serialise on `LOCK` and leave nothing pending.

    use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize};
    use std::sync::{Mutex as StdMutex, MutexGuard};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::testutil;

    /// Serialises the tests that drive the wheel (and `kernel::tc`'s clock step, which moves
    /// it).
    pub(crate) static LOCK: StdMutex<()> = StdMutex::new(());

    static FIRED: AtomicU32 = AtomicU32::new(0);
    static LAST_ARG: AtomicUsize = AtomicUsize::new(0);

    fn fire(arg: *mut c_void) {
        FIRED.fetch_add(1, Ordering::Relaxed);
        LAST_ARG.store(arg.addr(), Ordering::Relaxed);
    }

    fn setup() -> MutexGuard<'static, ()> {
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        testutil::install();
        startup(100, 10_000, 10_000_000);
        FIRED.store(0, Ordering::Relaxed);
        g
    }

    fn pinned(t: &'static Timeout) -> Pin<&'static Timeout> {
        pin_static(t)
    }

    /// `tos_pending`, the fifth counter.
    fn pending_count() -> u64 {
        stats()[4]
    }

    #[test]
    fn a_tick_timeout_fires_after_its_ticks() {
        let _g = setup();
        static TO: Timeout = Timeout::unset();
        let to = pinned(&TO);
        to.set(Some(fire), ptr::without_provenance_mut(7), KCLOCK_NONE, 0);
        assert!(to.initialized());
        assert!(to.add(3));
        assert!(to.pending());
        assert!(!to.add(3), "already scheduled");
        // timeout_add(to, n) fires in the n-th hardclock (timeout_add_ticks adds the extra one).
        for _ in 0..2 {
            tick_once();
            assert_eq!(FIRED.load(Ordering::Relaxed), 0);
        }
        tick_once();
        assert_eq!(FIRED.load(Ordering::Relaxed), 1);
        assert_eq!(LAST_ARG.load(Ordering::Relaxed), 7);
        assert!(to.triggered());
        assert!(!to.pending());
        assert!(!to.del(), "already ran");
        assert!(!to.triggered());
    }

    #[test]
    fn a_set_on_a_pending_timeout_keeps_it_pending_and_deletable() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        let to = pinned(&TO);
        let before = pending_count();
        assert!(to.add(10));
        to.set(
            Some(fire),
            ptr::without_provenance_mut(9),
            KCLOCK_NONE,
            TIMEOUT_PROC,
        );
        assert!(to.pending(), "still on its queue");
        assert!(to.initialized());
        assert_ne!(TO.flags.load(Ordering::Relaxed) & TIMEOUT_PROC, 0);
        assert!(to.del(), "deleted: it was pending");
        assert!(!TO.entry.is_linked());
        assert_eq!(pending_count(), before);
        // Added again (the list's "on a list already" would panic here had the set lost
        // TIMEOUT_ONQUEUE), then left as it was.
        assert!(to.add(10));
        assert!(to.del());
        to.set(Some(fire), ptr::null_mut(), KCLOCK_NONE, 0);
    }

    #[test]
    fn a_re_set_timeout_is_no_longer_triggered() {
        let _g = setup();
        static TO: Timeout = Timeout::unset();
        let to = pinned(&TO);
        to.set(Some(fire), ptr::null_mut(), KCLOCK_NONE, 0);
        assert!(to.add(1));
        tick_once();
        tick_once();
        assert_eq!(FIRED.load(Ordering::Relaxed), 1);
        assert!(to.triggered());
        // The C's timeout_set_flags stores to_flags whole: TIMEOUT_TRIGGERED goes.
        to.set(Some(fire), ptr::null_mut(), KCLOCK_NONE, 0);
        assert!(!to.triggered());
        assert!(to.initialized());
        assert!(!to.pending());
    }

    #[test]
    fn set_fresh_takes_a_pending_timeout_back_without_counting() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        let to = pinned(&TO);
        assert!(to.add(50));
        let before = stats();
        to.set_fresh(Some(fire), ptr::without_provenance_mut(3), KCLOCK_NONE, 0);
        let after = stats();
        assert!(!to.pending());
        assert!(!TO.entry.is_linked());
        assert_eq!(to.time(), 0);
        assert_eq!(after[4], before[4] - 1, "no longer queued");
        for i in [0, 1, 2, 5, 6] {
            assert_eq!(after[i], before[i], "counter {i} untouched");
        }
        // A fresh one again: idle, nothing changes.
        to.set_fresh(Some(fire), ptr::null_mut(), KCLOCK_NONE, 0);
        assert_eq!(stats(), after);
    }

    #[test]
    fn del_cancels_and_a_shorter_readd_moves_earlier() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        let to = pinned(&TO);
        let before = pending_count();
        assert!(to.add(100));
        assert_eq!(pending_count(), before + 1);
        assert!(to.del());
        tick_once();
        assert_eq!(FIRED.load(Ordering::Relaxed), 0);
        assert!(to.add(100));
        tick_once(); // bucketed now
        assert!(!to.add(1), "moved earlier while on the wheel");
        tick_once();
        tick_once();
        assert_eq!(FIRED.load(Ordering::Relaxed), 1);
        assert_eq!(pending_count(), before);
    }

    #[test]
    fn long_timeouts_cascade_through_the_wheel_levels() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        // Past the first level (256 buckets) and into the second.
        let n = 300;
        assert!(pinned(&TO).add(n));
        for _ in 0..n - 1 {
            tick_once();
        }
        assert_eq!(FIRED.load(Ordering::Relaxed), 0);
        tick_once();
        assert_eq!(FIRED.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn add_helpers_round_up() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        let to = pinned(&TO);
        let now = clock::ticks();
        assert!(to.add_sec(2));
        assert_eq!(to.time().wrapping_sub(now), 2 * 100 + 1);
        to.del();
        assert!(to.add_msec(15));
        assert_eq!(
            to.time().wrapping_sub(now),
            2 + 1,
            "15 ms is two 10 ms ticks, plus one"
        );
        to.del();
        assert!(to.add_usec(1));
        assert_eq!(to.time().wrapping_sub(now), 1 + 1);
        to.del();
        assert!(to.add_nsec(u64::MAX));
        assert_eq!(to.time().wrapping_sub(now), i32::MAX);
        to.del();
        assert!(to.add_msec(u64::MAX));
        assert_eq!(to.time().wrapping_sub(now), i32::MAX);
        to.del();
    }

    #[test]
    fn absolute_uptime_timeouts_fire_when_the_clock_passes_them() {
        let _g = setup();
        static TO: Timeout = Timeout::unset();
        let to = pinned(&TO);
        to.set(Some(fire), ptr::null_mut(), KCLOCK_UPTIME, 0);
        // The uptime double advances a microsecond per read: a deadline a few reads ahead is
        // reached after a handful of ticks.
        let deadline = clock::nanouptime().add(Timespec::new(0, 2_000));
        assert!(to.abs_ts(&deadline));
        let mut n = 0;
        while FIRED.load(Ordering::Relaxed) == 0 && n < 100 {
            tick_once();
            n += 1;
        }
        assert_eq!(FIRED.load(Ordering::Relaxed), 1, "fired after {n} ticks");
        assert_eq!(timeout_maskwheel(0, &Timespec::new(1, 0)), 128);
        assert_eq!(timeout_maskwheel(1, &Timespec::new(2, 0)), 1);
    }

    #[test]
    fn adjust_ticks_runs_what_the_step_skipped() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        assert!(pinned(&TO).add(50));
        tick_once(); // bucketed
        adjust_ticks(60);
        softclock();
        assert_eq!(FIRED.load(Ordering::Relaxed), 1);
        adjust_ticks(-5); // never backwards
    }

    #[test]
    fn proc_timeouts_wait_for_the_thread() {
        let _g = setup();
        static TO: Timeout = Timeout::unset();
        let to = pinned(&TO);
        to.set(Some(fire), ptr::null_mut(), KCLOCK_NONE, TIMEOUT_PROC);
        assert!(to.add(1));
        tick_once();
        tick_once();
        assert_eq!(
            FIRED.load(Ordering::Relaxed),
            0,
            "queued for the softclock thread"
        );
        assert!(to.pending());
        assert!(to.del_barrier());
    }

    #[test]
    fn the_statistics_keep_the_c_order() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        let to = pinned(&TO);
        let s0 = stats();
        assert!(to.add(10));
        assert!(!to.add(20));
        assert!(to.del());
        let s1 = stats();
        // added, cancelled, deleted, late, pending, readded.
        assert_eq!(s1[0] - s0[0], 2);
        assert_eq!(s1[1] - s0[1], 1);
        assert_eq!(s1[2] - s0[2], 1);
        assert_eq!(s1[4], s0[4]);
        assert_eq!(s1[5] - s0[5], 1);
        tick_once();
        // softclocks.
        assert_eq!(stats()[10] - s1[10], 1);
    }

    #[test]
    fn a_pinned_stack_timeout_is_deleted_when_dropped() {
        let _g = setup();
        let before = stats();
        {
            let to = core::pin::pin!(Timeout::new(fire, ptr::null_mut()));
            assert!(to.as_ref().add(5));
            assert!(to.pending());
        }
        let after = stats();
        assert_eq!(after[4], before[4], "no longer pending");
        assert_eq!(after[1] - before[1], 1, "cancelled by its drop");
        for _ in 0..10 {
            tick_once();
        }
        assert_eq!(FIRED.load(Ordering::Relaxed), 0);
    }

    static STARTED: AtomicBool = AtomicBool::new(false);
    static RELEASE: AtomicBool = AtomicBool::new(false);

    /// A timeout function that holds its context until the test releases it.
    fn hold(_arg: *mut c_void) {
        STARTED.store(true, Ordering::SeqCst);
        while !RELEASE.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
    }

    #[test]
    fn a_barrier_waits_for_the_running_timeout() {
        let _g = setup();
        static TO: Timeout = Timeout::new(hold, ptr::null_mut());
        STARTED.store(false, Ordering::SeqCst);
        RELEASE.store(false, Ordering::SeqCst);
        assert!(pinned(&TO).add(0));
        let runner = std::thread::spawn(tick_once);
        while !STARTED.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        let deleted = stats()[2];
        let waiter = std::thread::spawn(|| {
            TO.barrier();
            RELEASE.load(Ordering::SeqCst)
        });
        std::thread::sleep(Duration::from_millis(50));
        assert!(!waiter.is_finished(), "the barrier returned while it ran");
        RELEASE.store(true, Ordering::SeqCst);
        assert!(waiter.join().unwrap(), "returned after the release");
        runner.join().unwrap();
        // The waiter returned once its barrier was off the queue: the barrier's drop deleted
        // nothing (`tos_deleted` unchanged).
        assert_eq!(stats()[2], deleted);
        // Not running any more: a barrier returns at once.
        TO.barrier();
    }

    static FIRED_MP: AtomicU32 = AtomicU32::new(0);

    fn fire_mp(_arg: *mut c_void) {
        assert_eq!(
            testutil::kernel_lock_depth(),
            0,
            "run without the kernel lock"
        );
        FIRED_MP.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn an_mpsafe_proc_timeout_runs_in_its_thread() {
        let _g = setup();
        start_mp_thread_for_tests();
        static TO: Timeout = Timeout::unset();
        let to = pinned(&TO);
        to.set(
            Some(fire_mp),
            ptr::null_mut(),
            KCLOCK_NONE,
            TIMEOUT_PROC | TIMEOUT_MPSAFE,
        );
        let before = FIRED_MP.load(Ordering::SeqCst);
        assert!(to.add(1));
        tick_once();
        tick_once();
        let deadline = Instant::now() + Duration::from_secs(10);
        while FIRED_MP.load(Ordering::SeqCst) == before && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(FIRED_MP.load(Ordering::SeqCst), before + 1);
        to.del_barrier();
        assert!(stats()[8] >= 1, "run from the thread");
    }

    /// The object a shared timeout's argument names: its count of calls.
    struct Shared {
        calls: AtomicU32,
        drops: std::sync::Arc<AtomicU32>,
    }

    impl Drop for Shared {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn fire_shared(arg: &AnyArc) {
        if let Some(s) = arg.downcast_ref::<Shared>() {
            s.calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn a_shared_timeout_runs_with_its_counted_argument_until_cleared() {
        let _g = setup();
        static P: hal::pool::TypedPool<Shared> = hal::pool::TypedPool::new();
        static MADE: std::sync::Once = std::sync::Once::new();
        MADE.call_once(|| P.init(0, hal::pool::PR_WAITOK, "sharedto").unwrap());
        let drops = std::sync::Arc::new(AtomicU32::new(0));
        let s = P
            .get_shared(hal::pool::PR_WAITOK, || Shared {
                calls: AtomicU32::new(0),
                drops: std::sync::Arc::clone(&drops),
            })
            .unwrap();
        static TO: Timeout = Timeout::unset();
        let to = pinned(&TO);
        to.set_shared(
            fire_shared,
            hal::pool::PoolArc::erase(s.clone()),
            KCLOCK_NONE,
            0,
        );
        assert_eq!(hal::pool::PoolArc::count(&s), 2, "the timeout holds one");
        assert!(to.add(1));
        for _ in 0..3 {
            tick_once();
        }
        assert_eq!(s.calls.load(Ordering::Relaxed), 1);
        assert_eq!(hal::pool::PoolArc::count(&s), 2, "the call's clone is gone");
        // Set again with the C's function: the held argument goes back.
        to.set(Some(fire), ptr::null_mut(), KCLOCK_NONE, 0);
        assert_eq!(hal::pool::PoolArc::count(&s), 1);
        to.set_shared(
            fire_shared,
            hal::pool::PoolArc::erase(s.clone()),
            KCLOCK_NONE,
            0,
        );
        to.clear();
        assert_eq!(hal::pool::PoolArc::count(&s), 1, "clear gives it back");
        assert!(to.add(1));
        for _ in 0..3 {
            tick_once();
        }
        assert_eq!(
            s.calls.load(Ordering::Relaxed),
            1,
            "a cleared timeout calls nothing"
        );
        drop(s);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }
}
/* </TESTS> */
