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
//! Sleep and wakeup: `kern/kern_synch.c`, as `bsd`'s adapter of `kernel::sched`.
//!
//! Upstream: sys/kern/kern_synch.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_synch.rs@3c62ede68802
//!
//! The sleep queues, `sleep_setup`, `sleep_finish`, `wakeup_n`, `wakeup_proc`, `unsleep`,
//! `endtsleep` and `tsleep_nsec` are `kernel::sched`'s (N4.5a). This module keeps LZ's
//! signatures for `bsd`'s callers (a wait channel as any pointer, compared, never followed):
//! `tsleep`, `msleep` and `rwsleep` with their interlocks, the `wakeup` family, the reference
//! counts (`refcnt_*`), the condition variables (`cond_init`, `cond_wait`) and
//! `sched_yield(2)`. `sleep_signal_check`, the signal half of a `PCATCH` sleep, is
//! `kernel::sched`'s since N4.5b (over `kernel::sig`).
//!
//! ## Deviations
//! - The sleep functions return `Result<(), Errno>` (`EWOULDBLOCK` on timeout, `EINTR`/
//!   `ERESTART` from a signal) instead of an `int`.
//! - The `cold == 2` ddb stack dump is not here (`cold` is a flag, not a counter).
//! - `safepri` and `cold` are `sys/systm.rs` statics (see there).
//! - The `cold || panicstr` path's release and reacquisition of the kernel lock, written
//!   twice in C, is one helper, `kernel_lock_bounce`.
//!
//! ## Redesign
//! - LZ kept the sleep queues here (`slpque`, `hal::list::List`s of `&'static Proc` under the
//!   legacy `SCHED_LOCK`); they are `kernel::sched`'s `PinList`s under
//!   `hal::thread::SCHED_LOCK`'s hold, and a sleeper is the running thread
//!   (`hal::thread::with_current`), never a reference made from `curproc`'s pointer.
//!   `sleep_queue_init` has nothing left to do (the queues are empty when made).
//! - `wakeup_proc` and `unsleep` take the scheduler's hold their caller has (the C's
//!   `SCHED_ASSERT_LOCKED`), as a parameter.
//! - `cond_signal_handler`, a handler that turns its argument back into a `struct cond`, is
//!   `sys/sys/proc.rs`'s, beside `struct cond` (re-exported here).

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{Ordering, fence};

use hal::thread::SchedHold;

use crate::conf::param::TICK_NSEC;
use crate::kassert;
use crate::kern::kern_lock::{
    __mp_acquire_count, __mp_release_all, _kernel_lock_held, KERNEL_LOCK, mtx_enter, mtx_leave,
};
use crate::kern::kern_rwlock::{rw_assert_anylock, rw_enter, rw_exit, rw_status};
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::{panic, panicstr};
use crate::machine::intr::splx;
use crate::sys::errno::Errno;
use crate::sys::mutex::Mutex;
use crate::sys::param::{PCATCH, PNORELOCK, PRIMASK, PWAIT};
use crate::sys::proc::{Cond, Proc, ProcExt};
use crate::sys::refcnt::Refcnt;
use crate::sys::rwlock::Rwlock;
use crate::sys::systm::{COLD, INFSLP, SAFEPRI, SysArgs};
use crate::sys::types::Register;

pub use crate::sys::proc::cond_signal_handler;
pub use kernel::sched::NOWAKE;

/// The glue every scheduler call here is made for.
type G = BsdProcGlue;

/// `&nowake` as an ident.
pub fn nowake() -> *const c_void {
    ptr::from_ref(&NOWAKE).cast()
}

/// A wait channel as the scheduler compares it: its address.
#[inline]
fn chan<T: ?Sized>(ident: *const T) -> usize {
    ident.cast::<c_void>().addr()
}

/// `tsleep_nsec`: general sleep call. Suspends the current process until a wakeup is
/// performed on the specified identifier. The process will then be made runnable with the
/// specified priority. Sleeps at most `nsecs` (`INFSLP` means no timeout). If `priority`
/// includes the `PCATCH` flag, signals are checked before and after sleeping, else signals
/// are not checked. Returns `Ok` if awakened, `EWOULDBLOCK` if the timeout expires. If
/// `PCATCH` is set and a signal needs to be delivered, `ERESTART` is returned if the current
/// system call should be restarted if possible, and `EINTR` is returned if the system call
/// should be interrupted by the signal.
#[inline]
pub fn tsleep_nsec<T: ?Sized>(
    ident: *const T,
    priority: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    kernel::sched::tsleep_nsec::<G>(chan(ident), priority, wmesg, nsecs)
}

/// The `cold || panicstr` path's step in `tsleep_nsec` and `msleep_nsec`: if this CPU holds
/// the kernel lock, drop every hold and take them back, so another CPU waiting for the lock
/// gets a turn.
pub fn kernel_lock_bounce() {
    if _kernel_lock_held() {
        let hold_count = __mp_release_all(&KERNEL_LOCK);
        __mp_acquire_count(&KERNEL_LOCK, hold_count);
    }
}

/// `tsleep`: `tsleep_nsec` with the timeout in ticks.
pub fn tsleep<T: ?Sized>(
    ident: *const T,
    priority: i32,
    wmesg: &'static str,
    timo: i32,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if timo < 0 {
        panic(format_args!("tsleep: negative timo {timo}"));
    }
    if timo > 0 {
        nsecs = timo as u64 * TICK_NSEC.load(Ordering::Relaxed) as u64;
    }

    tsleep_nsec(ident, priority, wmesg, nsecs)
}

/// `msleep_nsec`: same as `tsleep`, but if we have a mutex provided, then once we've entered
/// the sleep queue we drop the mutex. After sleeping we re-lock.
pub fn msleep_nsec<T: ?Sized>(
    ident: *const T,
    mtx: &Mutex,
    priority: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let ident = ident.cast::<c_void>();
    kassert!(priority & !(PRIMASK | PCATCH | PNORELOCK) == 0);
    kassert!(!ptr::eq(ident, nowake()) || priority & PCATCH != 0 || nsecs != INFSLP);

    if COLD.load(Ordering::Relaxed) || panicstr() {
        // After a panic, or during autoconfiguration, just give interrupts a chance, then just
        // return; don't run any other procs or panic below, in case this is the idle process
        // and already asleep.
        let spl = mtx.mtx_oldipl.get();
        mtx.mtx_oldipl.set(SAFEPRI.load(Ordering::Relaxed));
        mtx_leave(mtx);
        kernel_lock_bounce();
        if priority & PNORELOCK == 0 {
            mtx_enter(mtx);
            mtx.mtx_oldipl.set(spl);
        } else {
            splx(spl);
        }
        return Ok(());
    }

    sleep_setup(ident, priority, wmesg);

    mtx_leave(mtx);
    // signal may stop the process, release mutex before that
    let error = sleep_finish(nsecs, true);

    if priority & PNORELOCK == 0 {
        mtx_enter(mtx);
    }

    error
}

/// `msleep`: `msleep_nsec` with the timeout in ticks.
pub fn msleep<T: ?Sized>(
    ident: *const T,
    mtx: &Mutex,
    priority: i32,
    wmesg: &'static str,
    timo: i32,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if timo < 0 {
        panic(format_args!("msleep: negative timo {timo}"));
    }
    if timo > 0 {
        nsecs = timo as u64 * TICK_NSEC.load(Ordering::Relaxed) as u64;
    }

    msleep_nsec(ident, mtx, priority, wmesg, nsecs)
}

/// `rwsleep_nsec`: same as `tsleep`, but if we have a rwlock provided, then once we've
/// entered the sleep queue we drop the it. After sleeping we re-lock.
pub fn rwsleep_nsec<T: ?Sized>(
    ident: *const T,
    rwl: &Rwlock,
    priority: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let ident = ident.cast::<c_void>();
    kassert!(priority & !(PRIMASK | PCATCH | PNORELOCK) == 0);
    kassert!(!ptr::eq(ident, nowake()) || priority & PCATCH != 0 || nsecs != INFSLP);
    kassert!(!ptr::eq(ident, ptr::from_ref(rwl).cast()));
    rw_assert_anylock(rwl);
    let status = rw_status(rwl);

    sleep_setup(ident, priority, wmesg);

    rw_exit(rwl);
    // signal may stop the process, release rwlock before that
    let error = sleep_finish(nsecs, true);

    if priority & PNORELOCK == 0 {
        let _ = rw_enter(rwl, status);
    }

    error
}

/// `rwsleep`: `rwsleep_nsec` with the timeout in ticks.
pub fn rwsleep<T: ?Sized>(
    ident: *const T,
    rwl: &Rwlock,
    priority: i32,
    wmesg: &'static str,
    timo: i32,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if timo < 0 {
        panic(format_args!("rwsleep: negative timo {timo}"));
    }
    if timo > 0 {
        nsecs = timo as u64 * TICK_NSEC.load(Ordering::Relaxed) as u64;
    }

    rwsleep_nsec(ident, rwl, priority, wmesg, nsecs)
}

/// `sleep_setup`: puts the current thread on the sleep queue of `ident`.
#[inline]
pub fn sleep_setup(ident: *const c_void, prio: i32, wmesg: &'static str) {
    kernel::sched::sleep_setup::<G>(ident.addr(), prio, wmesg);
}

/// `sleep_finish`: sleeps (or not) after `sleep_setup`; returns the sleep's error.
#[inline]
pub fn sleep_finish(nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
    kernel::sched::sleep_finish::<G>(nsecs, do_sleep)
}

/// `wakeup_proc`: if process hasn't been awakened (wchan non-zero), undo the sleep. If proc is
/// stopped, just unsleep so it will remain stopped.
pub fn wakeup_proc(h: &mut SchedHold, p: &Proc) -> bool {
    kernel::sched::wakeup_proc(h, hal::pin::pin_static(p.as_static()))
}

/// `unsleep`: remove a process from its wait queue.
pub fn unsleep(h: &mut SchedHold, p: &Proc) {
    kernel::sched::unsleep(h, p);
}

/// `wakeup_n`: make a number of processes sleeping on the specified identifier runnable.
#[inline]
pub fn wakeup_n<T: ?Sized>(ident: *const T, n: i32) {
    kernel::sched::wakeup_n::<G>(chan(ident), n);
}

/// `wakeup`: make all processes sleeping on the specified identifier runnable.
#[inline]
pub fn wakeup<T: ?Sized>(chan: *const T) {
    wakeup_n(chan, -1);
}

/// `sched_yield(2)`: give the CPU up; a thread of a multi-threaded process drops to the
/// lowest priority among its siblings so that they can make some progress.
pub fn sys_sched_yield(p: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    kernel::sched::sched_yield::<G>(hal::pin::pin_static(p.as_static()));
    Ok(())
}

/// `wakeup_one(c)`: `wakeup_n((c), 1)` (`<sys/systm.h>`).
#[inline]
pub fn wakeup_one<T: ?Sized>(chan: *const T) {
    wakeup_n(chan, 1);
}

// thrsleep_unlock, tslp_init, thrsleep_bucket, thrsleep, sys___thrsleep, tslp_wakeups,
// sys___thrwakeup: the syscalls (M6).

/// `refcnt_init`.
pub fn refcnt_init(r: &Refcnt) {
    refcnt_init_trace(r, 0);
}

/// `refcnt_init_trace`.
pub fn refcnt_init_trace(r: &Refcnt, trace: i32) {
    r.r_traceidx.set(trace);
    r.r_refs.store(1, Ordering::Relaxed);
    // TRACEINDEX(refcnt, ...): dt(4), not configured.
}

/// `refcnt_take`.
pub fn refcnt_take(r: &Refcnt) {
    let refs = r.r_refs.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    kassert!(refs > 1);
}

/// `refcnt_rele`: drops a reference; `true` when it was the last.
pub fn refcnt_rele(r: &Refcnt) -> bool {
    fence(Ordering::Release); // membar_exit_before_atomic()
    let refs = r.r_refs.fetch_sub(1, Ordering::Relaxed).wrapping_sub(1);
    kassert!(refs != u32::MAX);
    if refs == 0 {
        fence(Ordering::Acquire); // membar_enter_after_atomic()
        return true;
    }
    false
}

/// `refcnt_rele_wake`.
pub fn refcnt_rele_wake(r: &Refcnt) {
    if refcnt_rele(r) {
        wakeup_one(ptr::from_ref(r));
    }
}

/// `refcnt_finalize`: drops the caller's reference and sleeps until every other one is gone.
pub fn refcnt_finalize(r: &Refcnt, wmesg: &'static str) {
    fence(Ordering::Release); // membar_exit_before_atomic()
    let mut refs = r.r_refs.fetch_sub(1, Ordering::Relaxed).wrapping_sub(1);
    kassert!(refs != u32::MAX);
    while refs != 0 {
        sleep_setup(ptr::from_ref(r).cast(), PWAIT, wmesg);
        refs = r.r_refs.load(Ordering::Relaxed);
        let _ = sleep_finish(INFSLP, refs != 0);
    }
    // Order subsequent loads and stores after refs == 0 load.
    fence(Ordering::SeqCst); // membar_sync()
}

/// `refcnt_read`.
pub fn refcnt_read(r: &Refcnt) -> u32 {
    r.r_refs.load(Ordering::Relaxed)
}

/// `refcnt_shared(_r)`.
pub fn refcnt_shared(r: &Refcnt) -> bool {
    refcnt_read(r) > 1
}

/// `cond_init`.
pub fn cond_init(c: &Cond) {
    c.c_wait.store(1, Ordering::Relaxed);
}

/// `cond_wait`: sleeps until the cond is signalled.
pub fn cond_wait(c: &Cond, wmesg: &'static str) {
    let mut wait = c.c_wait.load(Ordering::Relaxed);
    while wait != 0 {
        sleep_setup(ptr::from_ref(c).cast(), PWAIT, wmesg);
        wait = c.c_wait.load(Ordering::Relaxed);
        let _ = sleep_finish(INFSLP, wait != 0);
    }
}
/* </CODE> */
