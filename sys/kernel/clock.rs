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
/* </LICENSES> */

/* <CODE> */
//! The clock's state the kernel crate owns: `ticks` (`kern_clock.c`), the kernel's time value,
//! and the uptime clock as the kernel reaches it (`nanouptime(9)`).
//!
//! Upstream: sys/kern/kern_clock.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_clock.rs@3c62ede68802
//!
//! `ticks` counts the hardclock interrupts since boot, started close to wrapping (fifteen
//! minutes of ticks before `INT_MAX`) so that code comparing tick values by subtraction meets
//! the wrap early. `hardclock` (`bsd`'s `kern_clock.rs` until N4.8, decision 11) increments it
//! on the primary CPU; timeout(9)'s `timeout_adjust_ticks` moves it forward under the wheel's
//! lock; everyone else reads it.
//!
//! The timecounters stay in `bsd` until N4.8 (`kern_tc.rs`): the kernel reads the uptime
//! through the [`Uptime`] bridge (bridges.toml; paid by N4), registered by `bsd` at boot.
//!
//! ## Redesign
//! - `ticks` moved here from `kern_clock.rs`, which re-exports it (its readers and
//!   `hardclock` unchanged): the timeout wheel, now in this crate, reads and moves it.
//! - `struct timespec` with the `timespecadd`/`timespecsub`/`timespeccmp`/`NSEC_TO_TIMESPEC`
//!   macros of `<sys/time.h>` is [`Timespec`] with methods and `Ord` (seconds, then
//!   nanoseconds: `timespeccmp`'s order); the userland `struct timespec` stays `bsd`'s, at the
//!   ABI edge, and converts field by field.
//! - `nanouptime(9)` is [`nanouptime`], over the [`Uptime`] bridge.

use core::sync::atomic::{AtomicI32, Ordering};

use hal::bridges::AlreadyRegistered;
use hal::once::OnceStatic;
use hal::sys::kernel::HZ;

/// One second in nanoseconds.
const NSEC_PER_SEC: i64 = 1_000_000_000;

/// `struct timespec` as the kernel keeps time: seconds and nanoseconds (`0..1e9` once
/// normalised). Ordered as `timespeccmp` orders: seconds, then nanoseconds.
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
    pub const fn new(tv_sec: i64, tv_nsec: i64) -> Self {
        Self { tv_sec, tv_nsec }
    }

    /// `NSEC_TO_TIMESPEC(nsec, &ts)`.
    pub const fn from_nsec(nsec: u64) -> Self {
        Self {
            tv_sec: (nsec / NSEC_PER_SEC as u64) as i64,
            tv_nsec: (nsec % NSEC_PER_SEC as u64) as i64,
        }
    }

    /// `timespecadd(self, b, &res)`: the sum, the nanoseconds carried.
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
    pub const fn sub(self, b: Timespec) -> Timespec {
        let mut sec = self.tv_sec.wrapping_sub(b.tv_sec);
        let mut nsec = self.tv_nsec - b.tv_nsec;
        if nsec < 0 {
            sec = sec.wrapping_sub(1);
            nsec += NSEC_PER_SEC;
        }
        Timespec::new(sec, nsec)
    }
}

/// `ticks`: the number of hardclock ticks, started close to wrapping.
pub static TICKS: AtomicI32 = AtomicI32::new(i32::MAX - (15 * 60 * HZ));

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
}
/* </TESTS> */
