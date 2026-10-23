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

/*-
 * Copyright (c) 1982, 1986, 1991, 1993
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
 *	@(#)kern_clock.c	8.5 (Berkeley) 1/21/94
 */
/*
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)time.h	8.2 (Berkeley) 7/10/94
 */
/* </LICENSES> */

/* <CODE> */
//! The clock (`kern_clock.c`): `ticks` and `jiffies`, `hardclock`, `initclocks` with the
//! clock periods it computes, `kern.clockrate`, the machine's clock hardware as the kernel
//! reaches it ([`ClockHardware`]), and the kernel's time values (`<sys/time.h>`'s
//! `struct timespec`, `timeval`, `bintime`).
//!
//! Upstream: sys/kern/kern_clock.c @ 3ce1f3f79392
//! Upstream: sys/sys/time.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_clock.rs@3c62ede68802
//! LZ: sys/sys/time.rs@3c62ede68802
//!
//! `ticks` counts the hardclock interrupts since boot, started close to wrapping (fifteen
//! minutes of ticks before `INT_MAX`) so that code comparing tick values by subtraction meets
//! the wrap early. [`hardclock`] increments it on the primary CPU (whose `kernel::clockintr`
//! queue alone binds the hardclock); timeout(9)'s `timeout_adjust_ticks` moves it forward
//! under the wheel's lock; everyone else reads it.
//!
//! The timecounters are `kernel::tc`'s since N4.8a. [`nanouptime`], [`getnanouptime`] and
//! [`nanotime`] here still go through the [`Uptime`] bridge (bridges.toml; paid by N4),
//! which `bsd` registers over `kernel::tc` (through its `kern_tc.rs`); retiring it moves the
//! kernel tests' uptime double, a later slice.
//!
//! ## Redesign
//! - `ticks` moved here from `kern_clock.rs`, which re-exports it: the timeout wheel, now in
//!   this crate, reads and moves it.
//! - N4.8a: `hardclock`, `initclocks`, `sysctl_clockrate`, `tstohz` and the clock statics
//!   (`stathz`, `profhz`, `profprocs`, `jiffies`, the hardclock, statclock and profclock
//!   periods; the round-robin period is `kernel::sched`'s) are here, `bsd`'s `kern_clock.rs`
//!   re-exporting them; `hardclock` takes the typed `hal::clock::ClockFrame` and does what
//!   the C's does (the timehands' tick, `ticks`, `jiffies`, the timeout wheel); `statclock`,
//!   which charges the running thread, stays `bsd`'s until N4.6b. The machine's `cpu_initclocks`,
//!   `cpu_startclock`, `setstatclockrate` and `delay(9)` are the [`ClockHardware`] bridge
//!   (bridges.toml; `bsd`'s machine code until N7), which `initclocks` calls.
//! - `struct timespec` with the `timespecadd`/`timespecsub`/`timespeccmp`/`NSEC_TO_TIMESPEC`
//!   macros of `<sys/time.h>` is [`Timespec`] with methods and `Ord` (seconds, then
//!   nanoseconds: `timespeccmp`'s order); the userland `struct timespec` stays `bsd`'s, at the
//!   ABI edge, and converts field by field.
//! - `nanouptime(9)` is [`nanouptime`], over the [`Uptime`] bridge.
//! - N4.8a (decision 8): the kernel's one `struct timespec`, `struct timeval` and `struct
//!   bintime` are here, `#[repr(C)]` and `AbiPod` (`hal::abi_pod!`), with the field names
//!   and types of `<sys/time.h>` (`time_t` and `long` are `i64` on both machines), so
//!   `bsd`'s `sys/sys/time.rs` names them instead of keeping its own and the system calls
//!   copy them to and from user space as they are. `kernel::resource::Timeval` and
//!   `kernel::pipe::Timespec`, the two copies the earlier slices wrote, are these. The
//!   in-place macros are methods returning the result (`timespecadd(a, b, &c)` is
//!   `c = a.add(b)`), the comparisons the derived `Ord` (seconds, then the fraction:
//!   `timespeccmp`'s and `bintimecmp`'s order), the `BINTIME_TO_*`/`*_TO_BINTIME`,
//!   `TIMECOUNT_TO_BINTIME` and `*_TO_NSEC` macros methods of the types, the C's modular
//!   arithmetic spelled `wrapping_*`.

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};

use hal::bridges::AlreadyRegistered;

use crate::assert::{kassert, kpanic};
use hal::once::OnceStatic;
use hal::sys::errno::Errno;
use hal::sys::kernel::HZ;

/// One second in nanoseconds.
const NSEC_PER_SEC: i64 = 1_000_000_000;

/// `struct timespec`: seconds and nanoseconds (`0..1e9` once normalised), the layout userland
/// shares (`#[repr(C)]`, two `i64`). Ordered as `timespeccmp` orders: seconds, then
/// nanoseconds.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Timespec {
    /// `tv_sec`.
    pub tv_sec: i64,
    /// `tv_nsec`.
    pub tv_nsec: i64,
}

impl Timespec {
    /// Zero (`timespecclear`).
    pub const ZERO: Timespec = Timespec::new(0, 0);

    /// `{ sec, nsec }`.
    #[inline]
    pub const fn new(tv_sec: i64, tv_nsec: i64) -> Self {
        Self { tv_sec, tv_nsec }
    }

    /// `NSEC_TO_TIMESPEC(nsec, &ts)`.
    #[inline]
    pub const fn from_nsec(nsec: u64) -> Self {
        Self {
            tv_sec: (nsec / NSEC_PER_SEC as u64) as i64,
            tv_nsec: (nsec % NSEC_PER_SEC as u64) as i64,
        }
    }

    /// `timespecadd(self, b, &res)`: the sum, the nanoseconds carried.
    #[inline]
    pub const fn add(self, b: Timespec) -> Timespec {
        let mut sec = self.tv_sec.wrapping_add(b.tv_sec);
        let mut nsec = self.tv_nsec + b.tv_nsec;
        if nsec >= NSEC_PER_SEC {
            sec = sec.wrapping_add(1);
            nsec -= NSEC_PER_SEC;
        }
        Timespec::new(sec, nsec)
    }

    /// `timespecsub(self, b, &res)`: the difference, the nanoseconds borrowed.
    #[inline]
    pub const fn sub(self, b: Timespec) -> Timespec {
        let mut sec = self.tv_sec.wrapping_sub(b.tv_sec);
        let mut nsec = self.tv_nsec - b.tv_nsec;
        if nsec < 0 {
            sec = sec.wrapping_sub(1);
            nsec += NSEC_PER_SEC;
        }
        Timespec::new(sec, nsec)
    }

    /// `timespecclear(tsp)`.
    #[inline]
    pub fn clear(&mut self) {
        *self = Timespec::ZERO;
    }

    /// `timespecisset(tsp)`.
    #[inline]
    pub const fn is_set(&self) -> bool {
        self.tv_sec != 0 || self.tv_nsec != 0
    }

    /// `timespecisvalid(tsp)`.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.tv_nsec >= 0 && self.tv_nsec < NSEC_PER_SEC
    }

    /// `TIMESPEC_TO_NSEC(ts)`: saturates at `UINT64_MAX`.
    #[inline]
    pub const fn to_nsec(&self) -> u64 {
        if self.tv_sec as u64 > (u64::MAX - self.tv_nsec as u64) / NSEC_PER_SEC as u64 {
            return u64::MAX;
        }
        (self.tv_sec as u64).wrapping_mul(NSEC_PER_SEC as u64) + self.tv_nsec as u64
    }

    /// `TIMESPEC_TO_BINTIME(ts, bt)`.
    #[inline]
    pub const fn to_bintime(&self) -> Bintime {
        Bintime {
            sec: self.tv_sec,
            // 18446744073 = int(2^64 / 1000000000)
            frac: (self.tv_nsec as u64).wrapping_mul(18_446_744_073),
        }
    }
}

hal::abi_pod!(Timespec {
    tv_sec: i64,
    tv_nsec: i64
});

/// `struct timeval`: seconds and microseconds, the layout userland shares (`#[repr(C)]`,
/// `time_t` and `suseconds_t`, both `i64`). Ordered as `timercmp` orders.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Timeval {
    /// `tv_sec`.
    pub tv_sec: i64,
    /// `tv_usec`.
    pub tv_usec: i64,
}

impl Timeval {
    /// `{ sec, usec }`.
    #[inline]
    pub const fn new(tv_sec: i64, tv_usec: i64) -> Self {
        Self { tv_sec, tv_usec }
    }

    /// `timeradd(self, b, &res)`: the sum, the microseconds carried.
    #[inline]
    pub const fn add(self, b: Timeval) -> Timeval {
        let mut sec = self.tv_sec.wrapping_add(b.tv_sec);
        let mut usec = self.tv_usec + b.tv_usec;
        if usec >= 1_000_000 {
            sec = sec.wrapping_add(1);
            usec -= 1_000_000;
        }
        Timeval::new(sec, usec)
    }

    /// `timersub(self, b, &res)`: the difference, the microseconds borrowed.
    #[inline]
    pub const fn sub(self, b: Timeval) -> Timeval {
        let mut sec = self.tv_sec.wrapping_sub(b.tv_sec);
        let mut usec = self.tv_usec - b.tv_usec;
        if usec < 0 {
            sec = sec.wrapping_sub(1);
            usec += 1_000_000;
        }
        Timeval::new(sec, usec)
    }

    /// `TIMESPEC_TO_TIMEVAL(&tv, &ts)`.
    #[inline]
    pub const fn from_timespec(ts: Timespec) -> Self {
        Self::new(ts.tv_sec, ts.tv_nsec / 1000)
    }

    /// `TIMEVAL_TO_TIMESPEC(&tv, &ts)`.
    #[inline]
    pub const fn to_timespec(&self) -> Timespec {
        Timespec::new(self.tv_sec, self.tv_usec * 1000)
    }

    /// `timerclear(tvp)`.
    #[inline]
    pub fn clear(&mut self) {
        *self = Timeval::new(0, 0);
    }

    /// `timerisset(tvp)`.
    #[inline]
    pub const fn is_set(&self) -> bool {
        self.tv_sec != 0 || self.tv_usec != 0
    }

    /// `timerisvalid(tvp)`.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.tv_usec >= 0 && self.tv_usec < 1_000_000
    }
}

hal::abi_pod!(Timeval {
    tv_sec: i64,
    tv_usec: i64
});

/// `struct bintime`: seconds and fractions of a second in units of 2^-64. Ordered as
/// `bintimecmp` orders.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Bintime {
    /// `sec`.
    pub sec: i64,
    /// `frac`: the fraction of a second, in 1/2^64.
    pub frac: u64,
}

impl Bintime {
    /// Zero.
    pub const ZERO: Bintime = Bintime::new(0, 0);

    /// A bintime from its parts.
    #[inline]
    pub const fn new(sec: i64, frac: u64) -> Self {
        Self { sec, frac }
    }

    /// `bintimeaddfrac(bt, x, &ct)`: `self` plus `x` fractions of a second.
    #[inline]
    pub const fn add_frac(self, x: u64) -> Bintime {
        let (frac, carry) = self.frac.overflowing_add(x);
        Bintime {
            sec: if carry {
                self.sec.wrapping_add(1)
            } else {
                self.sec
            },
            frac,
        }
    }

    /// `bintimeadd(bt, ct, &dt)`.
    #[inline]
    pub const fn add(self, ct: Bintime) -> Bintime {
        let (frac, carry) = self.frac.overflowing_add(ct.frac);
        let sec = self.sec.wrapping_add(ct.sec);
        Bintime {
            sec: if carry { sec.wrapping_add(1) } else { sec },
            frac,
        }
    }

    /// `bintimesub(bt, ct, &dt)`.
    #[inline]
    pub const fn sub(self, ct: Bintime) -> Bintime {
        let (frac, borrow) = self.frac.overflowing_sub(ct.frac);
        let sec = self.sec.wrapping_sub(ct.sec);
        Bintime {
            sec: if borrow { sec.wrapping_sub(1) } else { sec },
            frac,
        }
    }

    /// `TIMECOUNT_TO_BINTIME(count, scale, &bt)`: `count` periods of a counter whose period
    /// is `scale` fractions of a second.
    #[inline]
    pub const fn from_timecount(count: u32, scale: u64) -> Bintime {
        let count = count as u64;
        let hi64 = count.wrapping_mul(scale >> 32);
        let bt = Bintime {
            sec: (hi64 >> 32) as i64,
            frac: hi64 << 32,
        };
        bt.add_frac(count.wrapping_mul(scale & 0xffff_ffff))
    }

    /// `BINTIME_TO_TIMESPEC(bt, ts)`: the fraction rounded down (the date changes at
    /// midnight, not at noon).
    #[inline]
    pub const fn to_timespec(&self) -> Timespec {
        Timespec::new(self.sec, frac_to_nsec(self.frac) as i64)
    }

    /// `BINTIME_TO_TIMEVAL(bt, tv)`.
    #[inline]
    pub const fn to_timeval(&self) -> Timeval {
        Timeval::new(
            self.sec,
            ((1_000_000u64 * ((self.frac >> 32) as u32 as u64)) >> 32) as i64,
        )
    }

    /// `BINTIME_TO_NSEC(bt)`: modulo 2^64, as the C's `uint64_t` arithmetic.
    #[inline]
    pub const fn to_nsec(&self) -> u64 {
        (self.sec as u64)
            .wrapping_mul(NSEC_PER_SEC as u64)
            .wrapping_add(frac_to_nsec(self.frac) as u64)
    }
}

hal::abi_pod!(Bintime {
    sec: i64,
    frac: u64
});

/// `FRAC_TO_NSEC(frac)`: a fraction of a second in nanoseconds, rounded down.
#[inline]
pub const fn frac_to_nsec(frac: u64) -> u32 {
    (((frac >> 32) * 1_000_000_000) >> 32) as u32
}

/// `ticks`: the number of hardclock ticks, started close to wrapping.
pub static TICKS: AtomicI32 = AtomicI32::new(i32::MAX - (15 * 60 * HZ));

/// `jiffies` (volatile): hardclock ticks, not started near a wrap (that triggers a bug in
/// inteldrm).
pub static JIFFIES: AtomicU64 = AtomicU64::new(0);

/// `stathz`: the statistics clock's frequency.
pub static STATHZ: AtomicI32 = AtomicI32::new(0);
/// `profhz`: the profiling clock's frequency.
pub static PROFHZ: AtomicI32 = AtomicI32::new(0);
/// `profprocs`: the number of processes being profiled.
pub static PROFPROCS: AtomicI32 = AtomicI32::new(0);

/// \[I\] `hardclock_period`: hardclock period (ns).
pub static HARDCLOCK_PERIOD: AtomicU64 = AtomicU64::new(0);
/// \[I\] `statclock_avg`: average statclock period (ns).
pub static STATCLOCK_AVG: AtomicU64 = AtomicU64::new(0);
/// \[I\] `statclock_min`: minimum statclock period (ns).
pub static STATCLOCK_MIN: AtomicU64 = AtomicU64::new(0);
/// \[I\] `statclock_mask`: set of allowed offsets.
pub static STATCLOCK_MASK: AtomicU32 = AtomicU32::new(0);
/// \[I\] `statclock_is_randomized`: fixed or pseudorandom period?
pub static STATCLOCK_IS_RANDOMIZED: AtomicBool = AtomicBool::new(false);
/// \[I\] `profclock_period`: the profiling clock's period (ns; `subr_prof.c`'s).
pub static PROFCLOCK_PERIOD: AtomicU64 = AtomicU64::new(0);

/// The [`Uptime`] bridge, registered once by `bsd` (the host tests' doubles).
static UPTIME: OnceStatic<&'static dyn Uptime> = OnceStatic::new();

/// The uptime clock (`nanouptime(9)` over the timecounters of `sys/kern/kern_tc.rs`), until
/// `kern_tc` moves (N4.8): bridges.toml `Uptime`. N4.4: the coarse uptime and the precise
/// time of day too (kqueue's `kqueue_sleep` and `EVFILT_TIMER`'s `NOTE_ABSTIME`).
pub trait Uptime: Sync {
    /// `nanouptime(&ts)`: the time since boot, monotonic.
    fn nanouptime(&self) -> Timespec;
    /// `getnanouptime(&ts)`: the time since boot as of the last tick.
    fn getnanouptime(&self) -> Timespec;
    /// `nanotime(&ts)`: the time of day (`CLOCK_REALTIME`).
    fn nanotime(&self) -> Timespec;
}

/// Registers the [`Uptime`] bridge, once; a second call keeps the first and names it.
pub fn install_uptime(imp: &'static dyn Uptime) -> Result<(), AlreadyRegistered> {
    UPTIME.set(imp).map_err(|_| AlreadyRegistered("Uptime"))
}

/// `ticks`.
pub fn ticks() -> i32 {
    TICKS.load(Ordering::Relaxed)
}

/// `hardclock_period`.
#[inline]
pub fn hardclock_period() -> u64 {
    HARDCLOCK_PERIOD.load(Ordering::Relaxed)
}

/// The registered clock hardware, once.
static CLOCK_HW: OnceStatic<&'static dyn ClockHardware> = OnceStatic::new();

/// The machine's clock hardware: `arm_clock_func` (arm64: `delay`, `initclocks`,
/// `setstatclockrate`, `mpstartclock`, the first driver's registration winning) and amd64's
/// `initclock_func`, `startclock_func`, `delay_func` (replaced at attach) with its
/// `setstatclockrate`. Each machine keeps its own registration policy behind its one
/// implementation; `bsd` registers it at boot (bridges.toml `ClockHardware`, paid by N7 with
/// the clock drivers).
pub trait ClockHardware: Sync {
    /// `cpu_initclocks()`: the clock interrupts' hardware set up; `stathz`, `profhz` and
    /// `statclock_is_randomized` set.
    fn initclocks(&self);
    /// `cpu_startclock()`: this CPU's clock interrupts started.
    fn startclock(&self);
    /// `setstatclockrate(newhz)`.
    fn setstatclockrate(&self, newhz: i32);
    /// `delay(usecs)`: busy-waits.
    fn delay(&self, usecs: u32);
}

/// Registers the machine's clock hardware, once; a second call keeps the first and names it.
pub fn install_clock_hardware(imp: &'static dyn ClockHardware) -> Result<(), AlreadyRegistered> {
    CLOCK_HW
        .set(imp)
        .map_err(|_| AlreadyRegistered("ClockHardware"))
}

/// `cpu_initclocks()`: `panic(9)` before the hardware is registered, as the C's.
pub fn cpu_initclocks() {
    match CLOCK_HW.get() {
        Some(h) => h.initclocks(),
        None => kpanic(format_args!("initclocks function not initialized yet")),
    }
}

/// `cpu_startclock()`: `panic(9)` before the hardware is registered, as the C's.
pub fn cpu_startclock() {
    match CLOCK_HW.get() {
        Some(h) => h.startclock(),
        None => kpanic(format_args!("startclock function not initialized yet")),
    }
}

/// `setstatclockrate(newhz)`: `panic(9)` before the hardware is registered, as the C's.
pub fn setstatclockrate(newhz: i32) {
    match CLOCK_HW.get() {
        Some(h) => h.setstatclockrate(newhz),
        None => kpanic(format_args!(
            "arm_clock_func.setstatclockrate not initialized"
        )),
    }
}

/// `delay(9)`: busy-waits `usecs` microseconds through the hardware, or the calibration-free
/// loop before it is registered (`arm_dflt_delay`: "there is no good way to make this close,
/// but this isn't supposed to be used after the real clock attaches").
pub fn delay(usecs: u32) {
    match CLOCK_HW.get() {
        Some(h) => h.delay(usecs),
        None => {
            for _ in 0..usecs {
                for _ in 0..100 {
                    core::hint::spin_loop();
                }
            }
        }
    }
}

/// `initclocks`: the clock frequencies and periods computed, the timecounters started, and
/// the primary CPU's clock interrupts started.
pub fn initclocks() {
    // Let the machine-specific code do its bit.
    cpu_initclocks();

    let hz = crate::timeout::hz();
    kassert!((1..=1_000_000_000).contains(&hz));
    HARDCLOCK_PERIOD.store(1_000_000_000 / hz.max(1) as u64, Ordering::Relaxed);
    crate::sched::ROUNDROBIN_PERIOD.store(hardclock_period() * 10, Ordering::Relaxed);

    let stathz = STATHZ.load(Ordering::Relaxed);
    kassert!((1..=1_000_000_000).contains(&stathz));

    // Compute the average statclock() period. Then find var, the largest 32-bit power of
    // two such that var <= statclock_avg / 2.
    let statclock_avg = 1_000_000_000 / stathz.max(1) as u64;
    STATCLOCK_AVG.store(statclock_avg, Ordering::Relaxed);
    let half_avg = statclock_avg / 2;
    let mut var: u32 = 1 << 31;
    while u64::from(var) > half_avg {
        var /= 2;
    }

    // Set a lower bound for the range using statclock_avg and var. The mask for that range
    // is just (var - 1).
    STATCLOCK_MIN.store(statclock_avg - u64::from(var / 2), Ordering::Relaxed);
    STATCLOCK_MASK.store(var.wrapping_sub(1), Ordering::Relaxed);

    let profhz = PROFHZ.load(Ordering::Relaxed);
    kassert!((stathz..=1_000_000_000).contains(&profhz));
    kassert!(profhz % stathz.max(1) == 0);
    PROFCLOCK_PERIOD.store(1_000_000_000 / profhz.max(1) as u64, Ordering::Relaxed);

    crate::tc::inittimecounter();

    // Start dispatching clock interrupts on the primary CPU.
    cpu_startclock();
}

/// `hardclock`: the real-time timer, interrupting hz times per second: the timehands wound
/// up every `tc_tick`, `ticks` and `jiffies` counted, the timeout wheel updated.
pub fn hardclock(_frame: &hal::clock::ClockFrame) {
    crate::tc::tc_ticktock();
    TICKS.fetch_add(1, Ordering::Relaxed);
    JIFFIES.fetch_add(1, Ordering::Relaxed);

    // Update the timeout wheel.
    crate::timeout::hardclock_update();
}

/// `nanouptime(9)`: the time since boot (zero before the bridge is registered: no clock runs
/// yet).
pub fn nanouptime() -> Timespec {
    UPTIME.get().map_or(Timespec::ZERO, |u| u.nanouptime())
}

/// `getnanouptime(9)`: the time since boot as of the last tick (zero before the bridge is
/// registered).
pub fn getnanouptime() -> Timespec {
    UPTIME.get().map_or(Timespec::ZERO, |u| u.getnanouptime())
}

/// `nanotime(9)`: the time of day (zero before the bridge is registered).
pub fn nanotime() -> Timespec {
    UPTIME.get().map_or(Timespec::ZERO, |u| u.nanotime())
}

/// `sysctl_clockrate`: `kern.clockrate`, a `struct clockinfo` (tick, hz, profhz, stathz).
pub fn sysctl_clockrate(req: &mut crate::sysctl::SysctlReq<'_>) -> Result<(), Errno> {
    let clkinfo = crate::time::Clockinfo {
        tick: crate::timeout::tick(),
        hz: crate::timeout::hz(),
        profhz: PROFHZ.load(Ordering::Relaxed),
        stathz: STATHZ.load(Ordering::Relaxed),
    };
    let mut bytes = [0u8; size_of::<crate::time::Clockinfo>()];
    let _ = hal::user::pod_to_bytes(&clkinfo, &mut bytes);
    crate::sysctl::rdstruct(req, &bytes)
}

/// `tstohz(ts)`: the ticks of `ts`, rounded up, plus one for the current tick to expire
/// (`tvtohz` of `ts` rounded up to a microsecond); 0 for a time not after zero, `INT_MAX` at
/// most.
pub fn tstohz(ts: &Timespec) -> i32 {
    let mut sec = ts.tv_sec;
    let mut usec = ts.tv_nsec / 1000;
    // Round up.
    if ts.tv_nsec % 1000 != 0 {
        usec += 1;
        if usec >= 1_000_000 {
            usec -= 1_000_000;
            sec += 1;
        }
    }

    // If the number of usecs in the whole seconds part of the time fits in a long, then the
    // total number of usecs will fit in an unsigned long: the total converted to ticks,
    // rounding up and adding 1 to allow for the current tick to expire. Otherwise, if the
    // number of ticks in the whole seconds part fits in a long, the parts converted
    // separately and added. Otherwise the maximum representable value.
    let tick = crate::timeout::tick() as u64;
    let hz = i64::from(crate::timeout::hz());
    let nticks: u64 = if sec < 0 || (sec == 0 && usec <= 0) {
        0
    } else if sec <= i64::MAX / 1_000_000 {
        (sec as u64)
            .wrapping_mul(1_000_000)
            .wrapping_add(usec as u64)
            .div_ceil(tick)
            + 1
    } else if sec <= i64::MAX / hz {
        (sec as u64).wrapping_mul(hz as u64) + (usec as u64).div_ceil(tick) + 1
    } else {
        i64::MAX as u64
    };
    nticks.min(i32::MAX as u64) as i32
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_start_fifteen_minutes_before_the_wrap() {
        // A fresh counter (TICKS itself moves under the timeout tests).
        let fresh = AtomicI32::new(i32::MAX - (15 * 60 * HZ));
        assert_eq!(i32::MAX - fresh.load(Ordering::Relaxed), 90_000);
        assert_eq!(fresh.load(Ordering::Relaxed).wrapping_add(90_001), i32::MIN);
    }

    #[test]
    fn timespec_arithmetic_carries_and_borrows_as_the_macros() {
        let a = Timespec::new(1, 600_000_000);
        let b = Timespec::new(2, 700_000_000);
        assert_eq!(a.add(b), Timespec::new(4, 300_000_000));
        assert_eq!(b.sub(a), Timespec::new(1, 100_000_000));
        assert_eq!(a.sub(b), Timespec::new(-2, 900_000_000));
        assert_eq!(
            Timespec::from_nsec(10_000_000),
            Timespec::new(0, 10_000_000)
        );
        assert_eq!(Timespec::from_nsec(3_000_000_001), Timespec::new(3, 1));
        assert!(a < b && Timespec::new(1, 5) < Timespec::new(1, 6));
        assert!(Timespec::new(2, 0) > Timespec::new(1, 999_999_999));
    }

    struct TestHw {
        started: core::sync::atomic::AtomicU32,
    }

    impl ClockHardware for TestHw {
        fn initclocks(&self) {
            // agtimer_cpu_initclocks: stathz = hz, profhz = 10 hz, randomized.
            STATHZ.store(100, Ordering::Relaxed);
            PROFHZ.store(1000, Ordering::Relaxed);
            STATCLOCK_IS_RANDOMIZED.store(true, Ordering::Relaxed);
        }
        fn startclock(&self) {
            self.started.fetch_add(1, Ordering::Relaxed);
        }
        fn setstatclockrate(&self, _newhz: i32) {}
        fn delay(&self, _usecs: u32) {}
    }

    #[test]
    fn initclocks_computes_the_periods_as_the_c() {
        crate::testutil::install();
        static HW: TestHw = TestHw {
            started: core::sync::atomic::AtomicU32::new(0),
        };
        let _ = install_clock_hardware(&HW);
        let started = HW.started.load(Ordering::Relaxed);
        initclocks();
        // hz 100: a 10 ms hardclock, a 100 ms round robin, stathz 100: statclock_avg 10 ms,
        // var the largest power of two <= 5 ms (2^22 = 4194304 ns), min avg - var/2, mask
        // var - 1; profhz 1000: 1 ms.
        assert_eq!(hardclock_period(), 10_000_000);
        assert_eq!(
            crate::sched::ROUNDROBIN_PERIOD.load(Ordering::Relaxed),
            100_000_000
        );
        assert_eq!(STATCLOCK_AVG.load(Ordering::Relaxed), 10_000_000);
        assert_eq!(
            STATCLOCK_MIN.load(Ordering::Relaxed),
            10_000_000 - 2_097_152
        );
        assert_eq!(STATCLOCK_MASK.load(Ordering::Relaxed), 4_194_303);
        assert_eq!(PROFCLOCK_PERIOD.load(Ordering::Relaxed), 1_000_000);
        assert_eq!(HW.started.load(Ordering::Relaxed), started + 1);
        assert!(install_clock_hardware(&HW).is_err(), "one registration");
    }

    #[test]
    fn tstohz_rounds_up_and_adds_the_current_tick() {
        // hz 100, tick 10 ms: the wheel's (every test's `startup` passes these).
        assert_eq!(tstohz(&Timespec::new(0, 0)), 0);
        assert_eq!(tstohz(&Timespec::new(-1, 0)), 0);
        assert_eq!(tstohz(&Timespec::new(0, 1)), 2, "1 ns: one tick, plus one");
        assert_eq!(tstohz(&Timespec::new(0, 10_000_000)), 2);
        assert_eq!(tstohz(&Timespec::new(0, 10_000_001)), 3);
        assert_eq!(tstohz(&Timespec::new(1, 0)), 101);
        assert_eq!(tstohz(&Timespec::new(i64::MAX, 0)), i32::MAX);
    }

    #[test]
    fn bintime_arithmetic_and_conversions_are_the_macros() {
        // bintimeadd carries the fraction's overflow into the seconds, bintimesub borrows.
        let half = Bintime::new(1, 1 << 63);
        assert_eq!(half.add(half), Bintime::new(3, 0));
        assert_eq!(Bintime::new(3, 0).sub(half), half);
        assert_eq!(Bintime::new(0, u64::MAX).add_frac(1), Bintime::new(1, 0));
        // A half second is 500 ms, 500000 us, rounded down as the C.
        assert_eq!(half.to_timespec(), Timespec::new(1, 500_000_000));
        assert_eq!(half.to_timeval(), Timeval::new(1, 500_000));
        assert_eq!(half.to_nsec(), 1_500_000_000);
        // TIMESPEC_TO_BINTIME then back loses at most a nanosecond (2^64 / 1e9 truncated).
        let ts = Timespec::new(7, 123_456_789);
        let back = ts.to_bintime().to_timespec();
        assert!(back.tv_sec == 7 && (123_456_788..=123_456_789).contains(&back.tv_nsec));
        // TIMECOUNT_TO_BINTIME: 1e6 counts of a 1 MHz counter are one second.
        let scale = ((1u64 << 63) / 1_000_000).wrapping_mul(2);
        let bt = Bintime::from_timecount(1_000_000, scale);
        assert!(bt.to_nsec().abs_diff(1_000_000_000) <= 1, "{bt:?}");
        assert_eq!(frac_to_nsec(1 << 63), 500_000_000);
    }

    #[test]
    fn timespec_and_timeval_predicates_are_the_macros() {
        assert!(!Timespec::ZERO.is_set() && Timespec::new(0, 1).is_set());
        assert!(Timespec::new(0, 999_999_999).is_valid() && !Timespec::new(0, -1).is_valid());
        assert!(!Timespec::new(0, 1_000_000_000).is_valid());
        assert_eq!(Timespec::new(2, 5).to_nsec(), 2_000_000_005);
        assert_eq!(Timespec::new(i64::MAX, 0).to_nsec(), u64::MAX);
        let mut tv = Timeval::new(1, 999_999);
        assert!(tv.is_set() && tv.is_valid());
        assert_eq!(tv.add(Timeval::new(0, 1)), Timeval::new(2, 0));
        assert_eq!(Timeval::new(2, 0).sub(tv), Timeval::new(0, 1));
        assert_eq!(tv.to_timespec(), Timespec::new(1, 999_999_000));
        tv.clear();
        assert!(!tv.is_set());
        assert!(!Timeval::new(0, 1_000_000).is_valid());
        // The layouts userland shares.
        assert_eq!(core::mem::size_of::<Timespec>(), 16);
        assert_eq!(core::mem::size_of::<Timeval>(), 16);
        assert_eq!(core::mem::size_of::<Bintime>(), 16);
    }

    #[test]
    fn bintime_to_nsec_wraps_as_the_c() {
        // `bt->sec * 1000000000ULL + FRAC_TO_NSEC(bt->frac)` in uint64_t.
        let bt = Bintime::new(i64::MAX, u64::MAX);
        let want = (i64::MAX as u64)
            .wrapping_mul(1_000_000_000)
            .wrapping_add(u64::from(frac_to_nsec(u64::MAX)));
        assert_eq!(bt.to_nsec(), want);
        assert_eq!(Bintime::new(2, 1 << 63).to_nsec(), 2_500_000_000);
    }
}
/* </TESTS> */
