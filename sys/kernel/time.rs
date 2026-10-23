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
//! The system calls (`clock_gettime(2)` .. `setitimer(2)`), `setitimer`, `realitexpire` and
//! `itimer_update` stay `bsd`'s `kern_time.rs` over these until the process's interval
//! timers move with `struct process` (N4.6): they read and write `ps_timer`, `ps_realit_to`
//! and the CPU-time aggregates there.
//!
//! ## Deviations
//! - `itimerdecr` returns `bool` (`true` where the C returns 1: the timer is still
//!   running).
//! - `settime` takes `securelevel` from its caller (`bsd`'s `kern.securelevel`, N4.6's core
//!   once it moves); the C reads the global.
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
//!   held, and [`ppsratecheck`] over exclusive references is the same check.
//! - `resettodr_to`/`resettodr_task` are a kernel `Timeout` and `Task`, pinned statics.

use core::ffi::c_void;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};

use hal::intr::IPL_HIGH;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::pin::pin_static;

use crate::clock::{Bintime, Timespec, Timeval};
use crate::prf::printf;
use crate::sys::errno::Errno;
use crate::task::{SYSTQ, Task, task_add, task_del};
use crate::tc::{
    TC_LOCK, getmicrouptime, microtime, nanotime, tc_getfrequency, tc_getprecision, tc_setclock,
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
    let mut rv = false;
    let tv = getmicrouptime();

    let g = RATECHECK_MTX.lock();
    let delta = tv.sub(*lasttime);

    // check for 0,0 is so that the message will be seen at least once, even if interval is
    // huge.
    if delta >= *mininterval || (lasttime.tv_sec == 0 && lasttime.tv_usec == 0) {
        *lasttime = tv;
        rv = true;
    }
    drop(g);

    rv
}

/// `ppsratecheck(9)`: at most `maxpps` events per second (`maxpps < 0`: no limit), the
/// counter bumped either way.
pub fn ppsratecheck(lasttime: &mut Timeval, curpps: &mut i32, maxpps: i32) -> bool {
    ppsratecheck_locked(|g| ppsratecheck_held(g, lasttime, curpps, maxpps))
}

/// Runs `f` with `ppsratecheck_mtx` held: the way to [`ppsratecheck_held`] for a counter
/// shared between CPUs, which must be reached only with the mutex held.
pub fn ppsratecheck_locked<R>(f: impl FnOnce(&mut MutexedGuard<'_, ()>) -> R) -> R {
    let mut g = PPSRATECHECK_MTX.lock();
    f(&mut g)
}

/// `ppsratecheck(9)`'s body, `ppsratecheck_mtx` held (its guard is the proof).
pub fn ppsratecheck_held(
    _g: &mut MutexedGuard<'_, ()>,
    lasttime: &mut Timeval,
    curpps: &mut i32,
    maxpps: i32,
) -> bool {
    let tv = getmicrouptime();
    let delta = tv.sub(*lasttime);

    // check for 0,0 is so that the message will be seen at least once. if more than one
    // second have passed since the last update of lasttime, reset the counter.
    //
    // we do increment *curpps even in *curpps < maxpps case, as some may try to use *curpps
    // for stat purposes as well.
    let rv = if (lasttime.tv_sec == 0 && lasttime.tv_usec == 0) || delta.tv_sec >= 1 {
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
    fn ratecheck_and_ppsratecheck_limit() {
        testutil::install();
        let mut last = Timeval::new(0, 0);
        assert!(
            ratecheck(&mut last, &Timeval::new(3600, 0)),
            "the first time"
        );
        assert!(
            !ratecheck(&mut last, &Timeval::new(3600, 0)),
            "within the hour"
        );
        assert!(ratecheck(&mut last, &Timeval::new(0, 0)));

        let mut last = Timeval::new(0, 0);
        let mut count = 0;
        assert!(ppsratecheck(&mut last, &mut count, 2));
        assert!(ppsratecheck(&mut last, &mut count, 2));
        assert!(
            !ppsratecheck(&mut last, &mut count, 2),
            "the third in a second"
        );
        assert_eq!(count, 3, "counted even when refused");
        assert!(ppsratecheck(&mut last, &mut count, -1), "no limit");
        let mut shared = (Timeval::new(0, 0), i32::MAX);
        assert!(ppsratecheck_locked(|g| ppsratecheck_held(
            g,
            &mut shared.0,
            &mut shared.1,
            5
        )));
        assert_eq!(shared.1, 1, "reset on the first event, then bumped");
    }
}
/* </TESTS> */
