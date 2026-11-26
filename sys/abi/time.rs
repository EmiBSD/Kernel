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
/* </LICENSES> */

/* <CODE> */
//! The time system calls' translations (`kernel::time`: `clock_gettime(2)`, `nanosleep(2)`,
//! `setitimer(2)`, ...).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, ptr};
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use sysdefs::proc::syscall::{
    SYS_adjfreq, SYS_adjtime, SYS_clock_getres, SYS_clock_gettime, SYS_clock_settime,
    SYS_getitimer, SYS_gettimeofday, SYS_nanosleep, SYS_setitimer, SYS_settimeofday,
};

/// 87 = `clock_gettime(clockid_t clock_id, struct timespec *tp)`.
pub(crate) fn sys_clock_gettime(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::clock_gettime(
        SYS_clock_gettime as Register,
        int(args[0]),
        ptr(args[1]),
        rval,
    )
}

/// 88 = `clock_settime(clockid_t clock_id, const struct timespec *tp)`.
pub(crate) fn sys_clock_settime(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::clock_settime(
        SYS_clock_settime as Register,
        int(args[0]),
        ptr(args[1]),
        rval,
    )
}

/// 89 = `clock_getres(clockid_t clock_id, struct timespec *tp)`.
pub(crate) fn sys_clock_getres(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::clock_getres(
        SYS_clock_getres as Register,
        int(args[0]),
        ptr(args[1]),
        rval,
    )
}

/// 91 = `nanosleep(const struct timespec *rqtp, struct timespec *rmtp)`.
pub(crate) fn sys_nanosleep(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::nanosleep(SYS_nanosleep as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 67 = `gettimeofday(struct timeval *tp, struct timezone *tzp)`.
pub(crate) fn sys_gettimeofday(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::gettimeofday(
        SYS_gettimeofday as Register,
        ptr(args[0]),
        ptr(args[1]),
        rval,
    )
}

/// 68 = `settimeofday(const struct timeval *tv, const struct timezone *tzp)`.
pub(crate) fn sys_settimeofday(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::settimeofday(
        SYS_settimeofday as Register,
        ptr(args[0]),
        ptr(args[1]),
        rval,
    )
}

/// 305 = `adjfreq(const int64_t *freq, int64_t *oldfreq)`.
pub(crate) fn sys_adjfreq(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::adjfreq(SYS_adjfreq as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 140 = `adjtime(const struct timeval *delta, struct timeval *olddelta)`.
pub(crate) fn sys_adjtime(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::adjtime(SYS_adjtime as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 70 = `getitimer(int which, struct itimerval *itv)`.
pub(crate) fn sys_getitimer(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::getitimer(SYS_getitimer as Register, int(args[0]), ptr(args[1]), rval)
}

/// 69 = `setitimer(int which, const struct itimerval *itv, struct itimerval *oitv)`.
pub(crate) fn sys_setitimer(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::time::setitimer(
        SYS_setitimer as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        rval,
    )
}
/* </CODE> */
