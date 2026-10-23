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
 *	@(#)kern_time.c	8.4 (Berkeley) 5/26/95
 */

/*-
 * Copyright (c) 1996 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Gordon W. Ross
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
/* </LICENSES> */

/* <CODE> */
//! The time-of-day clock chip, the clock settings and the rate limiters of `kern_time.c`
//! that no process owns: `settime`, `todr_attach`/`inittodr`/`resettodr` and the periodic
//! `resettodr`, `ratecheck(9)` and `ppsratecheck(9)`, the interval-timer arithmetic
//! (`itimerfix`, `itimerdecr`) and `clock_getres(2)`'s timecounter resolution.
//!
//! Upstream: sys/kern/kern_time.c @ 3ce1f3f79392
//! Upstream: sys/dev/clock_subr.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_time.rs@3c62ede68802
//! LZ: sys/dev/clock_subr.rs@3c62ede68802
//!
//! The ten system calls (`clock_gettime(2)` .. `setitimer(2)`) are here since N4.8's a6,
//! generic over the process glue as the other process system calls, and `bsd`'s `sysent[]`
//! rows (the prologue: pledge, the pinned syscalls, the kernel lock) call them. What they
//! reach of `bsd`'s goes through [`ProcGlue`]: `setitimer` itself (the interval timers,
//! `ps_timer` and `ps_realit_to`, are still `bsd`'s members of `struct process`, with
//! `realitexpire` and `itimer_update` over them), the running CPU's `spc_runtime` and
//! `pledge_adjtime` (N8).
//!
//! ## Deviations
//! - `itimerdecr` returns `bool` (`true` where the C returns 1: the timer is still
//!   running).
//! - `settime` takes `securelevel` from its caller (`kern.securelevel`,
//!   `kernel::sysctl::SECURELEVEL`, which `clock_settime(2)` and `settimeofday(2)` read
//!   right before the call); the C reads the global inside it.
//!
//! ## Redesign
//! - `struct todr_chip_handle` (`todr_quality` and two function pointers taking the handle,
//!   a `todr_setwen` no implementer sets) is the [`Todr`] trait (decision 46, the
//!   N4.8 amendment's B): `quality`, `gettime`, `settime`; `cookie`/`bus_cookie` are the
//!   implementer's own fields. `todr_setwen` is omitted: every implementer at the pin leaves
//!   it `NULL` and `todr_wenable` has no caller (a default no-op method if one ever comes).
//!   The handle LZ kept as an `AtomicPtr` dereferenced in `unsafe` is a
//!   `Mutexed<Option<&'static dyn Todr>>`; `todr_attach` keeps the better quality.
//! - `ratecheck_mtx`/`ppsratecheck_mtx` own nothing in the C; here [`ppsratecheck_locked`]
//!   runs a caller's closure under the mutex, so a counter shared between CPUs (the
//!   network's limiters, LZ's `ppsratecheck_shared` raw pointers) is only reached with it
//!   held, and [`ppsratecheck`] over exclusive references is the same check. Its uptime is
//!   `microuptime`'s and `maxpps == 0` allows nothing, as the C (LZ read `getmicrouptime` and
//!   had no `maxpps == 0` branch, so a rate limit set to 0 let one event a second through).
//! - `resettodr_to`/`resettodr_task` are a kernel `Timeout` and `Task`, pinned statics.
//! - The system calls take their arguments typed (`clockid_t`, `int`, a `UserPtr` per user
//!   pointer, `NULL` the null `UserPtr`) instead of LZ's `SysArgs` record, and `p` as a
//!   `Thread<G>`; LZ's `clock_gettime` helper is [`clock_gettime`] over the glue's
//!   `spc_runtime`. The checks, their order and the errnos are the C's: `suser` first in
//!   `clock_settime(2)`, `settimeofday(2)` and (with a new value) `adjfreq(2)`/`adjtime(2)`,
//!   `pledge_adjtime` before anything in `adjtime(2)`, the copyins before any change,
//!   `settime`'s two securelevel gates, `getitimer(2)`/`setitimer(2)`'s range check before
//!   any copy.

use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use hal::intr::IPL_HIGH;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::pin::pin_static;
use hal::rwlock::RwMode;
use hal::sleep::{PCATCH, PWAIT};
use hal::user::UserPtr;

use crate::clock::{Bintime, STATHZ, Timespec, Timeval};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread, suser, tfind_user};
use crate::resource::{Tusage, tuagg_get_proc, tuagg_get_process};
use crate::sched::{NOWAKE, tsleep_nsec};
use crate::sys::errno::Errno;
use crate::sysctl::SECURELEVEL;
use crate::task::{SYSTQ, Task, task_add, task_del};
use crate::tc::{
    TC_LOCK, getmicrouptime, getnanouptime, microtime, microuptime, nanoruntime, nanotime,
    nanouptime, tc_adjfreq, tc_adjtime, tc_getfrequency, tc_getprecision, tc_setclock,
    tc_setrealtimeclock,
};
use crate::timeout::Timeout;

/// `CLOCK_REALTIME`.
pub const CLOCK_REALTIME: i32 = 0;
/// `CLOCK_PROCESS_CPUTIME_ID`.
pub const CLOCK_PROCESS_CPUTIME_ID: i32 = 2;
/// `CLOCK_MONOTONIC`.
pub const CLOCK_MONOTONIC: i32 = 3;
/// `CLOCK_THREAD_CPUTIME_ID`.
pub const CLOCK_THREAD_CPUTIME_ID: i32 = 4;
/// `CLOCK_UPTIME`.
pub const CLOCK_UPTIME: i32 = 5;
/// `CLOCK_BOOTTIME`.
pub const CLOCK_BOOTTIME: i32 = 6;

/// `ITIMER_REAL`: real time.
pub const ITIMER_REAL: i32 = 0;
/// `ITIMER_VIRTUAL`: virtual time.
pub const ITIMER_VIRTUAL: i32 = 1;
/// `ITIMER_PROF`: profiling.
pub const ITIMER_PROF: i32 = 2;

/// `MAXTSLP`: the longest sleep `nanosleep(2)` asks for at once (`<sys/systm.h>`).
const MAXTSLP: u64 = u64::MAX - 1;

/// `SECDAY`: seconds in a day.
const SECDAY: i64 = 86400;
/// `SECYR`: seconds in a (non-leap) year.
const SECYR: i64 = SECDAY * 365;
/// `OpenBSD` (`<sys/param.h>`): the version, year and month, at the pin.
const OPENBSD: i64 = 202_610;
/// `MINYEAR`: the minimum plausible year (`OpenBSD / 100 - 1`).
const MINYEAR: i64 = OPENBSD / 100 - 1;

/// `ADJFREQ_MAX`: `adjfreq(2)`'s bound, 500 ppm in units of 2^-32 ns per second.
pub const ADJFREQ_MAX: i64 = 500_000_000i64 << 32;
/// `ADJFREQ_MIN`.
pub const ADJFREQ_MIN: i64 = -ADJFREQ_MAX;

/// `RESETTODR_PERIOD`: seconds between the periodic writes of the clock chip.
const RESETTODR_PERIOD: i32 = 1800;

/// `struct itimerspec`: an interval timer as timespecs.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Itimerspec {
    /// `it_interval`: timer interval.
    pub it_interval: Timespec,
    /// `it_value`: timer expiration.
    pub it_value: Timespec,
}

impl Itimerspec {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            it_interval: Timespec::ZERO,
            it_value: Timespec::ZERO,
        }
    }
}

hal::abi_pod!(Itimerspec {
    it_interval: Timespec,
    it_value: Timespec
});

/// `struct itimerval`: an interval timer as timevals (`getitimer(2)`, `setitimer(2)`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Itimerval {
    /// `it_interval`: timer interval.
    pub it_interval: Timeval,
    /// `it_value`: current value.
    pub it_value: Timeval,
}

hal::abi_pod!(Itimerval {
    it_interval: Timeval,
    it_value: Timeval
});

/// `struct timezone` (`gettimeofday(2)`'s second argument, always zero).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timezone {
    /// `tz_minuteswest`: minutes west of Greenwich.
    pub tz_minuteswest: i32,
    /// `tz_dsttime`: type of dst correction.
    pub tz_dsttime: i32,
}

hal::abi_pod!(Timezone {
    tz_minuteswest: i32,
    tz_dsttime: i32
});

/// `struct clockinfo` (`kern.clockrate`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Clockinfo {
    /// `hz`: clock frequency.
    pub hz: i32,
    /// `tick`: micro-seconds per hz tick.
    pub tick: i32,
    /// `stathz`: statistics clock frequency.
    pub stathz: i32,
    /// `profhz`: profiling clock frequency.
    pub profhz: i32,
}

hal::abi_pod!(Clockinfo {
    hz: i32,
    tick: i32,
    stathz: i32,
    profhz: i32
});

/// `ppsratecheck_mtx` held, and the precise uptime read just before taking it: what
/// [`ppsratecheck_locked`] hands its closure, the proof [`ppsratecheck_held`] takes.
pub struct PpsLocked<'a> {
    _guard: MutexedGuard<'a, ()>,
    tv: Timeval,
}

/// `ratecheck_mtx`.
static RATECHECK_MTX: Mutexed<()> = Mutexed::new(IPL_HIGH, ());

/// `ppsratecheck_mtx`: held across [`ppsratecheck_locked`]'s closure.
pub static PPSRATECHECK_MTX: Mutexed<()> = Mutexed::new(IPL_HIGH, ());

/// `todr_handle`: the time-of-day clock chip, `None` until a driver attaches one.
static TODR_HANDLE: Mutexed<Option<&'static dyn Todr>> = Mutexed::new(IPL_HIGH, None);

/// `inittodr_done`: whether `inittodr` ran (so `resettodr` may write the chip).
static INITTODR_DONE: AtomicBool = AtomicBool::new(false);

/// `resettodr_to`.
static RESETTODR_TO: Timeout = Timeout::new(periodic_resettodr, core::ptr::null_mut());
/// `resettodr_task`.
static RESETTODR_TASK: Task = Task::new(perform_resettodr, core::ptr::null_mut());

/// `struct todr_chip_handle`: a time-of-day clock chip a driver attaches.
pub trait Todr: Sync {
    /// `todr_quality`: the better chip wins [`todr_attach`] (efi(4) -1000, plrtc(4) 0).
    fn quality(&self) -> i32;
    /// `todr_gettime(handle, tv)`: the chip's time.
    fn gettime(&self) -> Result<Timeval, Errno>;
    /// `todr_settime(handle, tv)`: sets the chip's time.
    fn settime(&self, tv: &Timeval) -> Result<(), Errno>;
}

/// This function is used by `clock_settime` and `settimeofday`: steps the realtime clock,
/// then writes the chip. `securelevel` is `kern.securelevel`'s value.
pub fn settime(ts: &Timespec, securelevel: i32) -> Result<(), Errno> {
    // Don't allow the time to be set forward so far it will wrap and become negative, thus
    // allowing an attacker to bypass the next check below. The cutoff is 1 year before
    // rollover occurs, so even if the attacker uses adjtime(2) to move the time past the
    // cutoff, it will take a very long time to get to the wrap point.
    //
    // XXX: we check against UINT_MAX until we can figure out how to deal with the hardware
    // RTCs.
    if ts.tv_sec > i64::from(u32::MAX) - 365 * 24 * 60 * 60 {
        printf(format_args!(
            "denied attempt to set clock forward to {}\n",
            ts.tv_sec
        ));
        return Err(Errno::EPERM);
    }
    // If the system is secure, we do not allow the time to be set to an earlier value (it
    // may be slowed using adjtime, but not set back). This feature prevent interlopers from
    // setting arbitrary time stamps on files.
    let now = nanotime();
    if securelevel > 1 && *ts <= now {
        printf(format_args!(
            "denied attempt to set clock back {} seconds\n",
            now.tv_sec - ts.tv_sec
        ));
        return Err(Errno::EPERM);
    }

    tc_setrealtimeclock(ts);
    hal::klock::kernel_lock(); // KERNEL_LOCK()
    resettodr();
    hal::klock::kernel_unlock(); // KERNEL_UNLOCK()

    Ok(())
}

/// `clock_getres(2)` of the timecounter clocks (`CLOCK_REALTIME`, `MONOTONIC`, `BOOTTIME`,
/// `UPTIME`): the precision of the counter the timehands read, in time, under `tc_lock`.
pub fn tc_resolution() -> Timespec {
    let r = TC_LOCK.read();
    let scale = ((1u64 << 63) / tc_getfrequency().max(1)) * 2;
    let bt = Bintime::new(0, tc_getprecision().wrapping_mul(scale));
    drop(r);
    bt.to_timespec()
}

/// `clock_gettime`: the time of `clock_id` as `p` sees it: the timecounter clocks, the CPU
/// time of `p`'s process or of `p` (the running part since the slice began added), or of
/// the thread of `p`'s process a `pthread_getcpuclockid()` clock names (`ESRCH` without
/// it); `EINVAL` for any other clock.
pub fn clock_gettime<G: ProcGlue>(p: &Thread<G>, clock_id: i32) -> Result<Timespec, Errno> {
    match clock_id {
        CLOCK_REALTIME => Ok(nanotime()),
        CLOCK_UPTIME => Ok(nanoruntime()),
        CLOCK_MONOTONIC | CLOCK_BOOTTIME => Ok(nanouptime()),
        CLOCK_PROCESS_CPUTIME_ID => {
            let tu = Tusage::new();
            let tp = nanouptime();
            tuagg_get_process(&tu, p.process());
            Ok(tp.sub(G::spc_runtime()).add(tu.tu_runtime.get()))
        }
        CLOCK_THREAD_CPUTIME_ID => {
            let tu = Tusage::new();
            let tp = nanouptime();
            tuagg_get_proc(&tu, p);
            Ok(tp.sub(G::spc_runtime()).add(tu.tu_runtime.get()))
        }
        // check for clock from pthread_getcpuclockid()
        _ if clock_type(clock_id) == CLOCK_THREAD_CPUTIME_ID => {
            hal::klock::kernel_lock(); // KERNEL_LOCK()
            let error = {
                let r = hal::klock::reader();
                match tfind_user::<G, _>(&r, clock_ptid(clock_id), p.process()) {
                    None => Err(Errno::ESRCH),
                    Some(q) => {
                        let tu = Tusage::new();
                        tuagg_get_proc(&tu, q);
                        Ok(tu.tu_runtime.get())
                    }
                }
            };
            hal::klock::kernel_unlock(); // KERNEL_UNLOCK()
            error
        }
        _ => Err(Errno::EINVAL),
    }
}

/// `__CLOCK_TYPE(c)`: the clock of a `pthread_getcpuclockid()` id.
const fn clock_type(c: i32) -> i32 {
    c & 0xfff
}

/// `__CLOCK_PTID(c)`: the thread of a `pthread_getcpuclockid()` id.
const fn clock_ptid(c: i32) -> i32 {
    (c >> 12) & 0xfffff
}

/// `clock_gettime(2)`: the time of `clock_id` copied out to `tp`.
pub fn sys_clock_gettime<G: ProcGlue>(
    p: &Thread<G>,
    clock_id: i32,
    tp: UserPtr<Timespec>,
) -> Result<(), Errno> {
    let ats = clock_gettime(p, clock_id)?;
    tp.copyout(&ats)
    // KTRACE: not configured.
}

/// `clock_settime(2)`: root only; only `CLOCK_REALTIME` can be set, through `settime` and
/// `kern.securelevel`.
pub fn sys_clock_settime<G: ProcGlue>(
    p: &Thread<G>,
    clock_id: i32,
    tp: UserPtr<Timespec>,
) -> Result<(), Errno> {
    clock_settime_secure(p, clock_id, tp, &SECURELEVEL)
}

/// [`sys_clock_settime`] with the `securelevel` it reads right before `settime`.
fn clock_settime_secure<G: ProcGlue>(
    p: &Thread<G>,
    clock_id: i32,
    tp: UserPtr<Timespec>,
    securelevel: &AtomicI32,
) -> Result<(), Errno> {
    suser(p)?;

    let ats = tp.copyin()?;

    match clock_id {
        CLOCK_REALTIME => {
            if !ats.is_valid() {
                return Err(Errno::EINVAL);
            }
            settime(&ats, securelevel.load(Ordering::Relaxed))
        }
        // Other clocks are read-only
        _ => Err(Errno::EINVAL),
    }
}

/// `clock_getres(2)`: the resolution of `clock_id` copied out to `tp` (when not `NULL`),
/// at least one nanosecond: the timecounter's precision for the timecounter clocks, a
/// statclock tick for the CPU-time clocks.
pub fn sys_clock_getres<G: ProcGlue>(
    p: &Thread<G>,
    clock_id: i32,
    tp: UserPtr<Timespec>,
) -> Result<(), Errno> {
    let stathz = || i64::from(STATHZ.load(Ordering::Relaxed).max(1));
    let mut ts = Timespec::ZERO;

    let error = match clock_id {
        CLOCK_REALTIME | CLOCK_MONOTONIC | CLOCK_BOOTTIME | CLOCK_UPTIME => {
            ts = tc_resolution();
            Ok(())
        }
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {
            ts.tv_nsec = 1_000_000_000 / stathz();
            Ok(())
        }
        // check for clock from pthread_getcpuclockid()
        _ if clock_type(clock_id) == CLOCK_THREAD_CPUTIME_ID => {
            hal::klock::kernel_lock(); // KERNEL_LOCK()
            let found = {
                let r = hal::klock::reader();
                tfind_user::<G, _>(&r, clock_ptid(clock_id), p.process()).is_some()
            };
            let error = if found {
                ts.tv_nsec = 1_000_000_000 / stathz();
                Ok(())
            } else {
                Err(Errno::ESRCH)
            };
            hal::klock::kernel_unlock(); // KERNEL_UNLOCK()
            error
        }
        _ => Err(Errno::EINVAL),
    };

    if error.is_ok() && !tp.is_null() {
        ts.tv_nsec = ts.tv_nsec.max(1);
        tp.copyout(&ts)?;
        // KTRACE: not configured.
    }

    error
}

/// `nanosleep(2)`: sleeps for the time at `rqtp`, or until a signal (`EINTR`); the time
/// left is copied out to `rmtp` when not `NULL`, whatever the sleep's result.
pub fn sys_nanosleep<G: ProcGlue>(
    _p: &Thread<G>,
    rqtp: UserPtr<Timespec>,
    rmtp: UserPtr<Timespec>,
) -> Result<(), Errno> {
    let mut request = rqtp.copyin()?;
    // KTRACE: not configured.

    if request.tv_sec < 0 || !request.is_valid() {
        return Err(Errno::EINVAL);
    }

    let nowake = ptr::from_ref(&NOWAKE).addr();
    let mut error;
    loop {
        let start = getnanouptime();
        let nsecs = request.to_nsec().clamp(1, MAXTSLP);
        error = tsleep_nsec::<G>(nowake, PWAIT | PCATCH, "nanoslp", nsecs);
        let stop = getnanouptime();
        request = request.sub(stop.sub(start));
        if request.tv_sec < 0 {
            request = Timespec::ZERO;
        }
        if error != Err(Errno::EWOULDBLOCK) || !request.is_set() {
            break;
        }
    }

    if error == Err(Errno::ERESTART) {
        error = Err(Errno::EINTR);
    }
    if error == Err(Errno::EWOULDBLOCK) {
        error = Ok(());
    }

    if !rmtp.is_null() {
        if let Err(copyout_error) = rmtp.copyout(&request) {
            error = Err(copyout_error);
        }
        // KTRACE: not configured.
    }

    error
}

/// `gettimeofday(2)`: the time of day to `tp` and a zero time zone to `tzp`, each when not
/// `NULL`.
pub fn sys_gettimeofday<G: ProcGlue>(
    _p: &Thread<G>,
    tp: UserPtr<Timeval>,
    tzp: UserPtr<Timezone>,
) -> Result<(), Errno> {
    if !tp.is_null() {
        tp.copyout(&microtime())?;
        // KTRACE: not configured.
    }
    if !tzp.is_null() {
        tzp.copyout(&Timezone::default())?;
    }
    Ok(())
}

/// `settimeofday(2)`: root only; both arguments are copied in before the clock changes, the
/// time zone only checked for readability; the time through `settime` and
/// `kern.securelevel`.
pub fn sys_settimeofday<G: ProcGlue>(
    p: &Thread<G>,
    tv: UserPtr<Timeval>,
    tzp: UserPtr<Timezone>,
) -> Result<(), Errno> {
    settimeofday_secure(p, tv, tzp, &SECURELEVEL)
}

/// [`sys_settimeofday`] with the `securelevel` it reads right before `settime`.
fn settimeofday_secure<G: ProcGlue>(
    p: &Thread<G>,
    tv: UserPtr<Timeval>,
    tzp: UserPtr<Timezone>,
    securelevel: &AtomicI32,
) -> Result<(), Errno> {
    suser(p)?;
    // Verify all parameters before changing time.
    let atv = if tv.is_null() {
        None
    } else {
        Some(tv.copyin()?)
    };
    if !tzp.is_null() {
        let _atz = tzp.copyin()?;
    }
    if let Some(atv) = atv {
        // KTRACE: not configured.
        if !atv.is_valid() {
            return Err(Errno::EINVAL);
        }
        settime(&atv.to_timespec(), securelevel.load(Ordering::Relaxed))?;
    }

    Ok(())
}

/// `adjfreq(2)`: the timecounter's frequency adjustment to `oldfreq` (when not `NULL`),
/// then, root only, the one at `freq` (when not `NULL`, `EINVAL` outside
/// `ADJFREQ_MIN..=ADJFREQ_MAX`) set, under `tc_lock` (for writing when setting).
pub fn sys_adjfreq<G: ProcGlue>(
    p: &Thread<G>,
    freq: UserPtr<i64>,
    oldfreq: UserPtr<i64>,
) -> Result<(), Errno> {
    let mut f = 0i64;

    if !freq.is_null() {
        suser(p)?;
        f = freq.copyin()?;
        if !(ADJFREQ_MIN..=ADJFREQ_MAX).contains(&f) {
            return Err(Errno::EINVAL);
        }
    }

    let w = TC_LOCK.enter(if freq.is_null() {
        RwMode::Read
    } else {
        RwMode::Write
    });
    let error = 'out: {
        if !oldfreq.is_null() {
            let mut oldf = 0i64;
            tc_adjfreq(&w, Some(&mut oldf), None);
            if let Err(e) = oldfreq.copyout(&oldf) {
                break 'out Err(e);
            }
        }
        if !freq.is_null() {
            tc_adjfreq(&w, None, Some(f));
        }
        Ok(())
    };
    drop(w);
    error
}

/// `adjtime(2)`: the clock's remaining adjustment to `olddelta` (when not `NULL`), then,
/// root only and as pledge allows, the one at `delta` (when not `NULL`) set; `EINVAL` for
/// an invalid `delta` or one whose microseconds overflow.
pub fn sys_adjtime<G: ProcGlue>(
    p: &Thread<G>,
    delta: UserPtr<Timeval>,
    olddelta: UserPtr<Timeval>,
) -> Result<(), Errno> {
    let mut adjustment = 0i64;

    G::pledge_adjtime(p, delta.addr())?;

    if !delta.is_null() {
        suser(p)?;
        let atv = delta.copyin()?;
        // KTRACE: not configured.
        if !atv.is_valid() {
            return Err(Errno::EINVAL);
        }

        if atv.tv_sec > i64::MAX / 1_000_000 {
            return Err(Errno::EINVAL);
        }
        if atv.tv_sec < i64::MIN / 1_000_000 {
            return Err(Errno::EINVAL);
        }
        adjustment = atv.tv_sec * 1_000_000;
        if adjustment > i64::MAX - atv.tv_usec {
            return Err(Errno::EINVAL);
        }
        adjustment += atv.tv_usec;
    }
    let w = (!delta.is_null()).then(|| TC_LOCK.write());

    let error = 'out: {
        if !olddelta.is_null() {
            let mut remaining = 0i64;
            tc_adjtime(w.as_ref(), Some(&mut remaining), None);
            let mut atv = Timeval::new(remaining / 1_000_000, remaining % 1_000_000);
            if atv.tv_usec < 0 {
                atv.tv_usec += 1_000_000;
                atv.tv_sec -= 1;
            }

            if let Err(e) = olddelta.copyout(&atv) {
                break 'out Err(e);
            }
        }

        if !delta.is_null() {
            tc_adjtime(w.as_ref(), None, Some(adjustment));
        }
        Ok(())
    };
    drop(w);
    error
}

/// `getitimer(2)`: the calling process's interval timer `which` (`EINVAL` outside
/// `ITIMER_REAL..=ITIMER_PROF`) copied out to `itv`.
pub fn sys_getitimer<G: ProcGlue>(
    _p: &Thread<G>,
    which: i32,
    itv: UserPtr<Itimerval>,
) -> Result<(), Errno> {
    if !(ITIMER_REAL..=ITIMER_PROF).contains(&which) {
        return Err(Errno::EINVAL);
    }

    let mut aitv = Itimerval::default();

    G::setitimer(which, None, Some(&mut aitv));

    itv.copyout(&aitv)
    // KTRACE: not configured.
}

/// `setitimer(2)`: the calling process's interval timer `which` (`EINVAL` outside
/// `ITIMER_REAL..=ITIMER_PROF`) set to the value at `itv` (when not `NULL`, checked by
/// [`itimerfix`] first), its old value copied out to `oitv` (when not `NULL`); nothing to
/// do with both `NULL`.
pub fn sys_setitimer<G: ProcGlue>(
    _p: &Thread<G>,
    which: i32,
    itv: UserPtr<Itimerval>,
    oitv: UserPtr<Itimerval>,
) -> Result<(), Errno> {
    if !(ITIMER_REAL..=ITIMER_PROF).contains(&which) {
        return Err(Errno::EINVAL);
    }

    let newitv = if itv.is_null() {
        None
    } else {
        let mut aitv = itv.copyin()?;
        // KTRACE: not configured.
        itimerfix(&mut aitv)?;
        Some(aitv)
    };
    if newitv.is_none() && oitv.is_null() {
        return Ok(());
    }

    let mut olditv = Itimerval::default();
    G::setitimer(
        which,
        newitv.as_ref(),
        (!oitv.is_null()).then_some(&mut olditv),
    );

    if !oitv.is_null() {
        return oitv.copyout(&olditv);
        // KTRACE: not configured.
    }

    Ok(())
}

/// Checks a `setitimer(2)` value: `EINVAL` for a negative or invalid time or one past
/// `UINT_MAX` seconds; clears `it_interval` when `it_value` is unset and rounds it up to one
/// tick.
pub fn itimerfix(itv: &mut Itimerval) -> Result<(), Errno> {
    let max = Timeval::new(i64::from(u32::MAX), 0);
    let min_interval = Timeval::new(0, i64::from(crate::timeout::tick()));

    if itv.it_value.tv_sec < 0 || !itv.it_value.is_valid() {
        return Err(Errno::EINVAL);
    }
    if itv.it_value > max {
        return Err(Errno::EINVAL);
    }
    if itv.it_interval.tv_sec < 0 || !itv.it_interval.is_valid() {
        return Err(Errno::EINVAL);
    }
    if itv.it_interval > max {
        return Err(Errno::EINVAL);
    }

    if !itv.it_value.is_set() {
        itv.it_interval.clear();
    }
    if itv.it_interval.is_set() && itv.it_interval < min_interval {
        itv.it_interval = min_interval;
    }

    Ok(())
}

/// Decrements an interval timer by `decrement`: `true` while it has not expired; an expired
/// periodic timer is reloaded, the overrun taken from its next period so it does not drift.
pub fn itimerdecr(itp: &mut Itimerspec, decrement: &Timespec) -> bool {
    itp.it_value = itp.it_value.sub(*decrement);
    if itp.it_value.tv_sec >= 0 && itp.it_value.is_set() {
        return true;
    }
    if !itp.it_interval.is_set() {
        itp.it_value.clear();
        return false;
    }
    while itp.it_value.tv_sec < 0 || !itp.it_value.is_set() {
        itp.it_value = itp.it_value.add(itp.it_interval);
    }
    false
}

/// `ratecheck(9)`: `true` (and `*lasttime` now) at most once per `mininterval`, and the
/// first time.
pub fn ratecheck(lasttime: &mut Timeval, mininterval: &Timeval) -> bool {
    let tv = getmicrouptime();
    let g = RATECHECK_MTX.lock();
    let rv = rate_step(tv, lasttime, mininterval);
    drop(g);
    rv
}

/// The decision of `ratecheck(9)` at the uptime `tv`, `ratecheck_mtx` held.
fn rate_step(tv: Timeval, lasttime: &mut Timeval, mininterval: &Timeval) -> bool {
    let mut rv = false;
    let delta = tv.sub(*lasttime);

    // check for 0,0 is so that the message will be seen at least once, even if interval is
    // huge.
    if delta >= *mininterval || (lasttime.tv_sec == 0 && lasttime.tv_usec == 0) {
        *lasttime = tv;
        rv = true;
    }
    rv
}

/// `ppsratecheck(9)`: at most `maxpps` events per second (`maxpps < 0`: no limit, `0`: none
/// at all), the counter bumped either way.
pub fn ppsratecheck(lasttime: &mut Timeval, curpps: &mut i32, maxpps: i32) -> bool {
    ppsratecheck_locked(|l| ppsratecheck_held(l, lasttime, curpps, maxpps))
}

/// Runs `f` with `ppsratecheck_mtx` held: the way to [`ppsratecheck_held`] for a counter
/// shared between CPUs, which must be reached only with the mutex held. The precise uptime
/// is read first, outside the `IPL_HIGH` mutex, as the C's `microuptime(&tv)` before its
/// `mtx_enter`, and travels with the guard.
pub fn ppsratecheck_locked<R>(f: impl FnOnce(&mut PpsLocked<'_>) -> R) -> R {
    let tv = microuptime();
    let mut l = PpsLocked {
        _guard: PPSRATECHECK_MTX.lock(),
        tv,
    };
    f(&mut l)
}

/// `ppsratecheck(9)`'s body, `ppsratecheck_mtx` held (the [`PpsLocked`] is the proof) and
/// the uptime read before it.
pub fn ppsratecheck_held(
    l: &mut PpsLocked<'_>,
    lasttime: &mut Timeval,
    curpps: &mut i32,
    maxpps: i32,
) -> bool {
    pps_step(l.tv, lasttime, curpps, maxpps)
}

/// The decision of `ppsratecheck(9)` at the uptime `tv` (its callers hold
/// `ppsratecheck_mtx`).
fn pps_step(tv: Timeval, lasttime: &mut Timeval, curpps: &mut i32, maxpps: i32) -> bool {
    let delta = tv.sub(*lasttime);

    // check for 0,0 is so that the message will be seen at least once. if more than one
    // second have passed since the last update of lasttime, reset the counter.
    //
    // we do increment *curpps even in *curpps < maxpps case, as some may try to use *curpps
    // for stat purposes as well.
    let rv = if maxpps == 0 {
        // no event allowed at all: the window and the counter's reset are left alone
        false
    } else if (lasttime.tv_sec == 0 && lasttime.tv_usec == 0) || delta.tv_sec >= 1 {
        *lasttime = tv;
        *curpps = 0;
        true
    } else if maxpps < 0 {
        true
    } else {
        *curpps < maxpps
    };

    // the following should be done on 32-bit arithmetic, since the C code did it so
    // (rate-limit "maxpps" is an int).
    *curpps = curpps.saturating_add(1);

    rv
}

/// The attached chip, if any.
fn todr_handle() -> Option<&'static dyn Todr> {
    *TODR_HANDLE.lock()
}

/// `inittodr`: initialises the time from the time-of-day chip, or from `base` (the file
/// system's time) when the chip is absent or implausible, warning as the C does.
pub fn inittodr(base: i64) {
    let mut base = base;
    let badbase;

    INITTODR_DONE.store(true, Ordering::Relaxed);

    if base < (MINYEAR - 1970) * SECYR {
        printf(format_args!("WARNING: preposterous time in file system\n"));
        // read the system clock anyway
        base = (MINYEAR - 1970) * SECYR;
        badbase = true;
    } else {
        badbase = false;
    }

    let todr = todr_handle();
    let rtctime = match todr.map(|t| t.gettime()) {
        Some(Ok(tv)) if tv.tv_sec >= (MINYEAR - 1970) * SECYR => Some(tv),
        _ => None,
    };

    'bad: {
        let Some(rtctime) = rtctime else {
            // Believe the time in the file system for lack of anything better, resetting
            // the TODR.
            if todr.is_some() && !badbase {
                printf(format_args!("WARNING: bad clock chip time\n"));
            }
            tc_setclock(&Timespec::new(base, 0));
            break 'bad;
        };
        tc_setclock(&rtctime.to_timespec());

        if !badbase {
            // See if we gained/lost two or more days; if so, assume something is amiss.
            let deltat = (rtctime.tv_sec - base).abs();
            if deltat < 2 * SECDAY {
                return; // all is well
            }
            printf(format_args!(
                "WARNING: clock {} {} days\n",
                if rtctime.tv_sec < base {
                    "lost"
                } else {
                    "gained"
                },
                deltat / SECDAY
            ));
        }
    }
    // bad:
    printf(format_args!("WARNING: CHECK AND RESET THE DATE!\n"));
}

/// `resettodr`: writes the current time to the chip, once `inittodr` has read it (never a
/// nonsense value over a reasonable one).
pub fn resettodr() {
    if !INITTODR_DONE.load(Ordering::Relaxed) {
        return;
    }

    let rtctime = microtime();

    if let Some(todr) = todr_handle()
        && todr.settime(&rtctime).is_err()
    {
        printf(format_args!("WARNING: can't update clock chip time\n"));
    }
}

/// `todr_attach`: the best chip a driver offers becomes the time-of-day clock.
pub fn todr_attach(todr: &'static dyn Todr) {
    let mut g = TODR_HANDLE.lock();
    let better = match *g {
        None => true,
        Some(cur) => todr.quality() > cur.quality(),
    };
    if better {
        *g = Some(todr);
    }
}

/// `periodic_resettodr`: the timeout hands the write to `systq`.
pub fn periodic_resettodr(_arg: *mut c_void) {
    let _ = task_add(&SYSTQ, resettodr_task());
}

/// `perform_resettodr`: writes the chip and comes back in half an hour.
pub fn perform_resettodr(_arg: *mut c_void) {
    resettodr();
    let _ = resettodr_to().add_sec(RESETTODR_PERIOD);
}

/// `start_periodic_resettodr`.
pub fn start_periodic_resettodr() {
    let _ = resettodr_to().add_sec(RESETTODR_PERIOD);
}

/// `stop_periodic_resettodr`.
pub fn stop_periodic_resettodr() {
    let _ = RESETTODR_TO.del();
    let _ = task_del(&SYSTQ, &RESETTODR_TASK);
}

/// `resettodr_to`, pinned.
fn resettodr_to() -> Pin<&'static Timeout> {
    pin_static(&RESETTODR_TO)
}

/// `resettodr_task`, pinned.
fn resettodr_task() -> Pin<&'static Task> {
    pin_static(&RESETTODR_TASK)
}

/// Forgets the attached chip and `inittodr`'s run (the host tests, which attach doubles).
#[cfg(test)]
pub(crate) fn todr_test_reset() {
    *TODR_HANDLE.lock() = None;
    INITTODR_DONE.store(false, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::AtomicI64;

    use super::*;
    use crate::testutil;

    /// The chip handle and the clock are global: the todr tests serialise.
    static LOCK: StdMutex<()> = StdMutex::new(());

    struct Chip {
        quality: i32,
        secs: AtomicI64,
        fail: AtomicBool,
    }

    impl Todr for Chip {
        fn quality(&self) -> i32 {
            self.quality
        }
        fn gettime(&self) -> Result<Timeval, Errno> {
            if self.fail.load(Ordering::Relaxed) {
                return Err(Errno::EIO);
            }
            Ok(Timeval::new(self.secs.load(Ordering::Relaxed), 0))
        }
        fn settime(&self, tv: &Timeval) -> Result<(), Errno> {
            self.secs.store(tv.tv_sec, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn the_better_chip_wins_and_inittodr_reads_it() {
        testutil::install();
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _t = crate::tc::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let _w = crate::timeout::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        todr_test_reset();
        static EFI: Chip = Chip {
            quality: -1000,
            secs: AtomicI64::new(1_800_000_000),
            fail: AtomicBool::new(false),
        };
        static RTC: Chip = Chip {
            quality: 0,
            secs: AtomicI64::new(1_800_000_100),
            fail: AtomicBool::new(false),
        };
        todr_attach(&RTC);
        todr_attach(&EFI);
        assert_eq!(
            todr_handle().map(|t| t.quality()),
            Some(0),
            "the lower one is refused"
        );

        // resettodr before inittodr writes nothing.
        resettodr();
        assert_eq!(RTC.secs.load(Ordering::Relaxed), 1_800_000_100);

        // The chip's time is within two days of the file system's: no warning.
        let warned = testutil::printf_count("WARNING: CHECK AND RESET THE DATE!");
        inittodr(1_800_000_000);
        assert!(nanotime().tv_sec >= 1_800_000_100);
        assert_eq!(
            testutil::printf_count("WARNING: CHECK AND RESET THE DATE!"),
            warned
        );

        // A failing chip: the file system's time, a warning.
        RTC.fail.store(true, Ordering::Relaxed);
        inittodr(1_800_100_000);
        assert!(testutil::printf_count("WARNING: bad clock chip time") >= 1);
        assert!(testutil::printf_count("WARNING: CHECK AND RESET THE DATE!") > warned);
        RTC.fail.store(false, Ordering::Relaxed);

        // resettodr after inittodr writes the time of day.
        resettodr();
        assert!(RTC.secs.load(Ordering::Relaxed) >= 1_800_100_000);
        todr_test_reset();
    }

    #[test]
    fn settime_refuses_the_far_future_and_going_back_when_secure() {
        testutil::install();
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _t = crate::tc::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let far = Timespec::new(i64::from(u32::MAX) - 365 * 86400 + 1, 0);
        assert_eq!(settime(&far, 0), Err(Errno::EPERM));
        assert!(testutil::printf_count("denied attempt to set clock forward to") >= 1);
        let back = Timespec::new(nanotime().tv_sec - 10, 0);
        assert_eq!(settime(&back, 2), Err(Errno::EPERM));
        assert!(testutil::printf_count("denied attempt to set clock back") >= 1);
        // Forward by a second at securelevel 2 is allowed.
        let ahead = Timespec::new(nanotime().tv_sec + 1, 0);
        assert_eq!(settime(&ahead, 2), Ok(()));
        assert!(nanotime().tv_sec >= ahead.tv_sec);
    }

    #[test]
    fn itimerfix_checks_and_rounds_as_the_c() {
        testutil::install();
        let mut itv = Itimerval {
            it_interval: Timeval::new(0, 5),
            it_value: Timeval::new(1, 0),
        };
        assert_eq!(itimerfix(&mut itv), Ok(()));
        assert_eq!(
            itv.it_interval,
            Timeval::new(0, 10_000),
            "rounded up to a tick"
        );
        let mut itv = Itimerval {
            it_interval: Timeval::new(3, 0),
            it_value: Timeval::new(0, 0),
        };
        assert_eq!(itimerfix(&mut itv), Ok(()));
        assert!(!itv.it_interval.is_set(), "no value, no interval");
        for bad in [
            Itimerval {
                it_interval: Timeval::new(0, 0),
                it_value: Timeval::new(-1, 0),
            },
            Itimerval {
                it_interval: Timeval::new(0, 0),
                it_value: Timeval::new(0, 1_000_000),
            },
            Itimerval {
                it_interval: Timeval::new(0, 0),
                it_value: Timeval::new(i64::from(u32::MAX) + 1, 0),
            },
            Itimerval {
                it_interval: Timeval::new(0, -1),
                it_value: Timeval::new(1, 0),
            },
        ] {
            let mut b = bad;
            assert_eq!(itimerfix(&mut b), Err(Errno::EINVAL), "{bad:?}");
        }
    }

    #[test]
    fn itimerdecr_reloads_without_drift() {
        let mut it = Itimerspec {
            it_interval: Timespec::new(0, 100),
            it_value: Timespec::new(0, 150),
        };
        assert!(itimerdecr(&mut it, &Timespec::new(0, 100)));
        assert_eq!(it.it_value, Timespec::new(0, 50));
        // Expired by 30: reloaded with the overrun taken off the next period.
        assert!(!itimerdecr(&mut it, &Timespec::new(0, 80)));
        assert_eq!(it.it_value, Timespec::new(0, 70));
        // A one-shot timer stops.
        let mut once = Itimerspec {
            it_interval: Timespec::ZERO,
            it_value: Timespec::new(0, 10),
        };
        assert!(!itimerdecr(&mut once, &Timespec::new(0, 20)));
        assert!(!once.it_value.is_set());
    }

    #[test]
    fn ratecheck_lets_one_event_through_per_interval() {
        let now = Timeval::new(100, 500);
        let hour = Timeval::new(3600, 0);
        // The first time, whatever the interval.
        let mut last = Timeval::new(0, 0);
        assert!(rate_step(now, &mut last, &hour), "the first time");
        assert_eq!(last, now);
        // Within the hour: refused, the last time kept.
        assert!(!rate_step(Timeval::new(200, 0), &mut last, &hour));
        assert_eq!(last, now);
        // An interval gone by (`>=`, as `timercmp`).
        assert!(rate_step(Timeval::new(3700, 500), &mut last, &hour));
        assert_eq!(last, Timeval::new(3700, 500));
        // A zero interval: always.
        assert!(rate_step(last, &mut last.clone(), &Timeval::new(0, 0)));
    }

    #[test]
    fn ratecheck_and_ppsratecheck_take_their_mutexes() {
        testutil::install();
        // The first event goes through whatever the uptime is; the counter is bumped.
        let mut last = Timeval::new(0, 0);
        assert!(
            ratecheck(&mut last, &Timeval::new(3600, 0)),
            "the first time"
        );
        let mut last = Timeval::new(0, 0);
        let mut count = 5;
        assert!(ppsratecheck(&mut last, &mut count, 2));
        assert_eq!(count, 1, "reset on the first event, then bumped");
        let mut shared = (Timeval::new(0, 0), i32::MAX);
        assert!(ppsratecheck_locked(|g| ppsratecheck_held(
            g,
            &mut shared.0,
            &mut shared.1,
            5
        )));
        assert_eq!(shared.1, 1, "reset on the first event, then bumped");
        let mut none = (Timeval::new(0, 0), 0);
        assert!(!ppsratecheck(&mut none.0, &mut none.1, 0), "maxpps 0");
        assert_eq!(none, (Timeval::new(0, 0), 1));
    }

    /// `kern_time.c`'s decision table, one row per branch, at a fixed uptime: (`*lasttime`,
    /// `*curpps`, `maxpps`) -> (the answer, `*lasttime` afterwards, `*curpps` afterwards).
    #[test]
    fn ppsratecheck_follows_the_c_branch_by_branch() {
        let now = Timeval::new(100, 500);
        let recent = Timeval::new(100, 0);
        let old = Timeval::new(99, 400);
        let zero = Timeval::new(0, 0);
        #[rustfmt::skip]
        let rows: [(Timeval, i32, i32, bool, Timeval, i32); 9] = [
            // maxpps == 0: never, even the first time; nothing reset, the counter bumped.
            (zero, 0, 0, false, zero, 1),
            (old, 7, 0, false, old, 8),
            (recent, 3, 0, false, recent, 4),
            // the first event, or a second gone by: allowed, the window restarted.
            (zero, 9, 2, true, now, 1),
            (old, 9, 2, true, now, 1),
            // no limit.
            (recent, 50, -1, true, recent, 51),
            // within the second, under and at the limit.
            (recent, 1, 2, true, recent, 2),
            (recent, 2, 2, false, recent, 3),
            // the counter stops at INT_MAX (the C's wrap-around guard).
            (recent, i32::MAX, 2, false, recent, i32::MAX),
        ];
        for (last0, pps0, maxpps, rv, last1, pps1) in rows {
            let (mut last, mut pps) = (last0, pps0);
            let got = pps_step(now, &mut last, &mut pps, maxpps);
            assert_eq!(
                (got, last, pps),
                (rv, last1, pps1),
                "lasttime {last0:?} curpps {pps0} maxpps {maxpps}"
            );
        }
    }

    // The system calls' checks (N4.8's a6), over a test glue with threads and credentials.

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// The tests' region of the user-copy double.
    const UTIME: usize = testutil::USER_BASE + 0xCC00;

    /// A thread of a new process `pid` whose uids are all `uid`.
    fn thread_as(
        pid: hal::sys::types::Pid,
        uid: hal::sys::types::Uid,
    ) -> (
        crate::proc::ProcessRef<Procs>,
        hal::pool::PoolBox<Thread<Procs>>,
    ) {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            crate::proc::procinit::<Procs>(64, 64);
        }
        let pr = crate::proc::process_alloc::<Procs>(pid, ());
        let mut ids = crate::cred::Ucred::new();
        ids.cr_ruid.set(uid);
        ids.cr_uid.set(uid);
        ids.cr_svuid.set(uid);
        drop(pr.set_ucred(crate::cred::CredRef::new(ids)));
        crate::proc::chgproccnt::<Procs>(uid, 1);
        let t = crate::proc::thread_alloc::<Procs>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        (pr, t)
    }

    /// A user pointer to a `T` this test wrote at `UTIME + off`.
    fn user<T: hal::user::AbiPod>(off: usize, v: &T) -> UserPtr<T> {
        let u = UserPtr::<T>::new(UTIME + off);
        u.copyout(v).unwrap();
        u
    }

    /// kern_time.c: `settimeofday(2)`, `clock_settime(2)`, `adjtime(2)` and `adjfreq(2)` (with
    /// a new value) and `reboot(2)` ask `suser` first: a user gets `EPERM` before any user
    /// memory is read (the addresses are outside the double, `EFAULT` had they been).
    #[test]
    fn the_setters_and_reboot_are_root_s_and_ask_first() {
        let (_pr, t) = thread_as(710, 1000);
        let bad = |off| UTIME + 0x10_0000 + off;
        assert_eq!(
            sys_settimeofday(&t, UserPtr::new(bad(0)), UserPtr::new(bad(64))),
            Err(Errno::EPERM)
        );
        assert_eq!(
            sys_settimeofday(&t, UserPtr::new(0), UserPtr::new(0)),
            Err(Errno::EPERM),
            "even with nothing to set"
        );
        for clock in [CLOCK_REALTIME, CLOCK_MONOTONIC, 99] {
            assert_eq!(
                sys_clock_settime(&t, clock, UserPtr::new(bad(0))),
                Err(Errno::EPERM),
                "clock {clock}"
            );
        }
        assert_eq!(
            sys_adjtime(&t, UserPtr::new(bad(0)), UserPtr::new(0)),
            Err(Errno::EPERM)
        );
        assert_eq!(
            sys_adjfreq(&t, UserPtr::new(bad(0)), UserPtr::new(0)),
            Err(Errno::EPERM)
        );
        assert!(matches!(
            crate::reboot::sys_reboot(&t, 0),
            Err(Errno::EPERM)
        ));
        // Reading the adjustments needs no privilege.
        let old = UserPtr::<i64>::new(UTIME + 0x100);
        assert_eq!(sys_adjfreq(&t, UserPtr::new(0), old), Ok(()));
        let oldd = UserPtr::<Timeval>::new(UTIME + 0x110);
        assert_eq!(sys_adjtime(&t, UserPtr::new(0), oldd), Ok(()));
    }

    /// `clock_settime(2)`: root's copy is read before the clock is checked, and only
    /// `CLOCK_REALTIME` can be set (`EINVAL` for the others, the read-only and the unknown);
    /// an invalid time is `EINVAL` too; a bad address `EFAULT`, before the clock's check.
    #[test]
    fn clock_settime_sets_only_the_realtime_clock() {
        let (_pr, t) = thread_as(711, 0);
        let now = user(0x200, &Timespec::new(5, 0));
        for clock in [CLOCK_MONOTONIC, CLOCK_UPTIME, CLOCK_BOOTTIME, 99, -1] {
            assert_eq!(
                sys_clock_settime(&t, clock, now),
                Err(Errno::EINVAL),
                "clock {clock}"
            );
        }
        assert_eq!(
            sys_clock_settime(&t, CLOCK_MONOTONIC, UserPtr::new(UTIME + 0x10_0000)),
            Err(Errno::EFAULT)
        );
        let invalid = user(0x210, &Timespec::new(5, 1_000_000_000));
        assert_eq!(
            sys_clock_settime(&t, CLOCK_REALTIME, invalid),
            Err(Errno::EINVAL)
        );
    }

    /// `settime`'s gate through the system calls: at securelevel 2 the clock is never set
    /// back (`EPERM`), whichever call asks, and the time zone is only read.
    #[test]
    fn the_setters_refuse_to_set_the_clock_back_when_secure() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _t = crate::tc::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (_pr, t) = thread_as(712, 0);
        let secure = AtomicI32::new(2);
        let before = nanotime();
        let tv = user(0x300, &Timeval::new(0, 0));
        let tz = user(0x310, &Timezone::default());
        assert_eq!(settimeofday_secure(&t, tv, tz, &secure), Err(Errno::EPERM));
        let ts = user(0x320, &Timespec::new(0, 0));
        assert_eq!(
            clock_settime_secure(&t, CLOCK_REALTIME, ts, &secure),
            Err(Errno::EPERM)
        );
        assert!(nanotime() >= before, "the clock did not go back");
        assert!(testutil::printf_count("denied attempt to set clock back") >= 2);
        // A time zone alone sets nothing, whatever the level; an unreadable one is EFAULT.
        assert_eq!(
            settimeofday_secure(&t, UserPtr::new(0), tz, &secure),
            Ok(())
        );
        assert_eq!(
            settimeofday_secure(
                &t,
                UserPtr::new(0),
                UserPtr::new(UTIME + 0x10_0000),
                &secure
            ),
            Err(Errno::EFAULT)
        );
        // An invalid time is EINVAL before the gate.
        let invalid = user(0x330, &Timeval::new(0, 1_000_000));
        assert_eq!(
            settimeofday_secure(&t, invalid, UserPtr::new(0), &secure),
            Err(Errno::EINVAL)
        );
    }

    /// `getitimer(2)` and `setitimer(2)` check `which` before any copy, `setitimer(2)` the
    /// value with `itimerfix` before the timer is reached; `gettimeofday(2)` writes a zero
    /// time zone; `clock_getres(2)` answers at least a nanosecond and `EINVAL` for an
    /// unknown clock; `clock_gettime(2)` too.
    #[test]
    fn the_readers_and_the_itimers_check_as_the_c() {
        let (_pr, t) = thread_as(713, 1000);
        let bad = UserPtr::<Itimerval>::new(UTIME + 0x10_0000);
        for which in [-1, 3, 99] {
            assert_eq!(sys_getitimer(&t, which, bad), Err(Errno::EINVAL));
            assert_eq!(sys_setitimer(&t, which, bad, bad), Err(Errno::EINVAL));
        }
        let invalid = user(
            0x400,
            &Itimerval {
                it_interval: Timeval::new(0, 0),
                it_value: Timeval::new(0, 1_000_000),
            },
        );
        assert_eq!(
            sys_setitimer(&t, ITIMER_REAL, invalid, UserPtr::new(0)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_setitimer(&t, ITIMER_PROF, UserPtr::new(0), UserPtr::new(0)),
            Ok(())
        );
        // The test glue keeps no timers: they read zero.
        let out = UserPtr::<Itimerval>::new(UTIME + 0x420);
        assert_eq!(sys_getitimer(&t, ITIMER_VIRTUAL, out), Ok(()));
        assert_eq!(out.copyin().unwrap(), Itimerval::default());

        let tz = user(
            0x440,
            &Timezone {
                tz_minuteswest: 7,
                tz_dsttime: 1,
            },
        );
        assert_eq!(sys_gettimeofday(&t, UserPtr::new(0), tz), Ok(()));
        assert_eq!(tz.copyin().unwrap(), Timezone::default());

        let res = UserPtr::<Timespec>::new(UTIME + 0x460);
        for clock in [CLOCK_REALTIME, CLOCK_MONOTONIC, CLOCK_PROCESS_CPUTIME_ID] {
            assert_eq!(sys_clock_getres(&t, clock, res), Ok(()), "clock {clock}");
            let r = res.copyin().unwrap();
            assert!(r.tv_sec == 0 && r.tv_nsec >= 1, "clock {clock}: {r:?}");
        }
        assert_eq!(
            sys_clock_getres(&t, CLOCK_REALTIME, UserPtr::new(0)),
            Ok(())
        );
        assert_eq!(sys_clock_getres(&t, 99, res), Err(Errno::EINVAL));
        assert_eq!(sys_clock_gettime(&t, 99, res), Err(Errno::EINVAL));
        // A pthread_getcpuclockid() clock naming no thread of the process.
        let other = CLOCK_THREAD_CPUTIME_ID | (5 << 12);
        assert_eq!(sys_clock_gettime(&t, other, res), Err(Errno::ESRCH));
        assert_eq!(sys_clock_getres(&t, other, res), Err(Errno::ESRCH));
    }

    /// `nanosleep(2)`: a negative or invalid request is `EINVAL` before any sleep.
    #[test]
    fn nanosleep_refuses_a_bad_request() {
        let (_pr, t) = thread_as(714, 1000);
        for bad in [
            Timespec::new(-1, 0),
            Timespec::new(0, -1),
            Timespec::new(0, 1_000_000_000),
        ] {
            let rq = user(0x500, &bad);
            assert_eq!(
                sys_nanosleep(&t, rq, UserPtr::new(0)),
                Err(Errno::EINVAL),
                "{bad:?}"
            );
        }
        assert_eq!(
            sys_nanosleep(&t, UserPtr::new(UTIME + 0x10_0000), UserPtr::new(0)),
            Err(Errno::EFAULT)
        );
    }
}
/* </TESTS> */
