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
 * Copyright (c) 1988 University of Utah.
 * Copyright (c) 1982, 1990, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
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
 * from: Utah $Hdr: clock.c 1.18 91/01/21$
 *
 *	@(#)clock.c	8.2 (Berkeley) 1/12/94
 */
/* </LICENSES> */

/* <CODE> */
//! Generic routines to convert between a POSIX date (seconds since 1/1/1970) and
//! yr/mo/day/hr/min/sec: `kern/clock_subr.c`, with `struct clock_ymdhms` of `<sys/time.h>`.
//!
//! Upstream: sys/kern/clock_subr.c @ 3ce1f3f79392
//! LZ: sys/kern/clock_subr.rs@e8249449d905
//!
//! The time-of-day chips (`efi_machdep`'s EFI clock, amd64's `mc146818`) keep the date broken
//! down; these two functions turn it into the kernel's seconds and back.
//!
//! ## Deviations
//! - `clock_secs_to_ymdhms` returns the broken-down time instead of filling an out parameter.
//! - The day count and the seconds sum are `i64`, where the C uses `int` for the days (and for
//!   `((days * 24 + hour) * 60 + min)`, which overflows past the year 6000).
//! - A month above 12 in `clock_ymdhms_to_secs` adds no days for the months past December; the
//!   C reads past the end of `month_days[]`.
//! - The C copies `month_days[]` to a local so as to make February 29 days in a leap year;
//!   here the leap day is added to the month's length when it is looked up.
//!
//! ## Redesign
//! - N7.1b: the module moves from `bsd` (`sys/kern/clock_subr.rs`) into the kernel with
//!   `struct clock_ymdhms` ([`ClockYmdhms`]) and `POSIX_BASE_YEAR`, which `bsd`'s
//!   `sys/sys/time.rs` re-exports; the arithmetic is LZ's, already safe. `bsd`'s path
//!   `crate::kern::clock_subr` stays a re-export of this module for the chips' drivers still
//!   in `bsd` (`docs/SCAFFOLDING.md`).

use sysdefs::base::types::Time;

/// `SECDAY`: seconds in a day (`<sys/time.h>`).
const SECDAY: i64 = 86400;

/// `POSIX_BASE_YEAR`: traditional POSIX base year (`<sys/time.h>`).
pub const POSIX_BASE_YEAR: i32 = 1970;

/// `FEBRUARY`.
const FEBRUARY: i32 = 2;

/// `month_days`: days in each month of a non-leap year.
const MONTH_DAYS: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// `struct clock_ymdhms`: "POSIX time" to/from "YY/MM/DD/hh/mm/ss".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClockYmdhms {
    /// `dt_year`.
    pub dt_year: u16,
    /// `dt_mon`.
    pub dt_mon: u8,
    /// `dt_day`.
    pub dt_day: u8,
    /// `dt_wday`: day of week.
    pub dt_wday: u8,
    /// `dt_hour`.
    pub dt_hour: u8,
    /// `dt_min`.
    pub dt_min: u8,
    /// `dt_sec`.
    pub dt_sec: u8,
}

/// `leapyear`: a year divisible by 4, except the centuries not divisible by 400. The cheap
/// test (`year & 3`) comes first, as in the C, so most years take no division.
fn leapyear(year: i32) -> bool {
    year.trailing_zeros() >= 2 && (year % 100 != 0 || year % 400 == 0)
}

/// `days_in_year(a)`.
fn days_in_year(year: i32) -> i32 {
    if leapyear(year) { 366 } else { 365 }
}

/// `days_in_month(a)` on the plain table (`month` is 1-based; 0 outside 1..=12).
fn days_in_month(month: i32) -> i32 {
    usize::try_from(month - 1)
        .ok()
        .and_then(|m| MONTH_DAYS.get(m))
        .copied()
        .unwrap_or(0)
}

/// `clock_ymdhms_to_secs`: the POSIX time (seconds since 1/1/1970) of a broken-down date.
pub fn clock_ymdhms_to_secs(dt: &ClockYmdhms) -> Time {
    let year = i32::from(dt.dt_year);
    let mon = i32::from(dt.dt_mon);

    // The days since the epoch: the whole years, the leap day of this one once February is
    // past, the whole months, then the days of this month.
    let mut days: i64 = (POSIX_BASE_YEAR..year)
        .map(|y| i64::from(days_in_year(y)))
        .sum();
    if leapyear(year) && mon > FEBRUARY {
        days += 1;
    }
    days += (1..mon).map(|m| i64::from(days_in_month(m))).sum::<i64>();
    days += i64::from(dt.dt_day) - 1;

    // Add hours, minutes, seconds.
    ((days * 24 + i64::from(dt.dt_hour)) * 60 + i64::from(dt.dt_min)) * 60 + i64::from(dt.dt_sec)
}

/// `clock_secs_to_ymdhms`: the broken-down date of a POSIX time. The day of the week counts
/// from Sunday = 0.
pub fn clock_secs_to_ymdhms(secs: Time) -> ClockYmdhms {
    let mut days = (secs / SECDAY) as i32;
    let mut rsec = (secs % SECDAY) as i32; // remainder seconds

    // Day of week (Note: 1/1/1970 was a Thursday)
    let dt_wday = ((days + 4) % 7) as u8;

    // Subtract out whole years, counting them in year.
    let mut year = POSIX_BASE_YEAR;
    while days >= days_in_year(year) {
        days -= days_in_year(year);
        year += 1;
    }

    // Subtract out whole months, counting them in mon. February has 29 days in a leap year.
    let leap = leapyear(year);
    let mdays = |m: i32| days_in_month(m) + i32::from(leap && m == FEBRUARY);
    let mut mon = 1;
    while days >= mdays(mon) {
        days -= mdays(mon);
        mon += 1;
    }

    // Hours, minutes, seconds are easy
    let dt_hour = (rsec / 3600) as u8;
    rsec %= 3600;
    let dt_min = (rsec / 60) as u8;
    rsec %= 60;

    ClockYmdhms {
        dt_year: year as u16,
        dt_mon: mon as u8,
        // Days are what is left over (+1) from all that.
        dt_day: (days + 1) as u8,
        dt_wday,
        dt_hour,
        dt_min,
        dt_sec: rsec as u8,
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    const fn ymdhms(year: u16, mon: u8, day: u8, hour: u8, min: u8, sec: u8) -> ClockYmdhms {
        ClockYmdhms {
            dt_year: year,
            dt_mon: mon,
            dt_day: day,
            dt_wday: 0,
            dt_hour: hour,
            dt_min: min,
            dt_sec: sec,
        }
    }

    #[test]
    fn leap_years() {
        assert!(leapyear(2000));
        assert!(leapyear(2024));
        assert!(!leapyear(1900));
        assert!(!leapyear(2100));
        assert!(!leapyear(2026));
        assert_eq!(days_in_year(2000), 366);
        assert_eq!(days_in_year(2001), 365);
    }

    /// (broken-down date, day of the week with Sunday = 0, POSIX time).
    const KNOWN: &[(ClockYmdhms, u8, Time)] = &[
        (ymdhms(1970, 1, 1, 0, 0, 0), 4, 0),
        (ymdhms(1999, 12, 31, 23, 59, 59), 5, 946_684_799),
        (ymdhms(2000, 1, 1, 0, 0, 0), 6, 946_684_800),
        (ymdhms(2000, 2, 29, 0, 0, 0), 2, 951_782_400),
        (ymdhms(2000, 3, 1, 0, 0, 0), 3, 951_868_800),
        (ymdhms(2026, 10, 3, 12, 34, 56), 6, 1_791_030_896),
        (ymdhms(2038, 1, 19, 3, 14, 7), 2, 2_147_483_647),
        (ymdhms(2100, 3, 1, 0, 0, 0), 1, 4_107_542_400),
    ];

    #[test]
    fn ymdhms_to_secs_known_dates() {
        for (dt, _, secs) in KNOWN {
            assert_eq!(clock_ymdhms_to_secs(dt), *secs, "{dt:?}");
        }
    }

    #[test]
    fn secs_to_ymdhms_known_dates() {
        for (dt, wday, secs) in KNOWN {
            let want = ClockYmdhms {
                dt_wday: *wday,
                ..*dt
            };
            assert_eq!(clock_secs_to_ymdhms(*secs), want, "{secs}");
        }
    }

    #[test]
    fn months_outside_the_table_have_no_days() {
        assert_eq!(days_in_month(0), 0);
        assert_eq!(days_in_month(13), 0);
        assert_eq!(days_in_month(-5), 0);
        // A 13th month adds nothing past December (the C reads past `month_days[]`).
        let dec = ymdhms(2001, 12, 1, 0, 0, 0);
        let m13 = ymdhms(2001, 13, 1, 0, 0, 0);
        assert_eq!(
            clock_ymdhms_to_secs(&m13) - clock_ymdhms_to_secs(&dec),
            31 * SECDAY
        );
    }

    #[test]
    fn round_trip_every_day_and_odd_seconds() {
        // Every day from 1970 to 2105, at a time of day that moves with the day.
        for day in 0..49_400i64 {
            let secs = day * SECDAY + (day * 37) % SECDAY;
            let dt = clock_secs_to_ymdhms(secs);
            assert_eq!(clock_ymdhms_to_secs(&dt), secs);
            assert_eq!(i64::from(dt.dt_wday), (day + 4) % 7);
        }
    }
}
/* </TESTS> */
