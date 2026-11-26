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
//! The time system calls (`kernel::time`: `clock_gettime(2)`, `nanosleep(2)`,
//! `setitimer(2)`, ...).
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::{Register, Rval, SysResult};

use sysdefs::syscall::{
    SYS_adjfreq, SYS_adjtime, SYS_clock_getres, SYS_clock_gettime, SYS_clock_settime,
    SYS_getitimer, SYS_gettimeofday, SYS_nanosleep, SYS_setitimer, SYS_settimeofday,
};

use super::native;

/// `clock_gettime(2)`.
pub fn clock_gettime(nr: Register, clock_id: i32, tp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_clock_gettime,
        [clock_id as Register, tp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `clock_settime(2)`.
pub fn clock_settime(nr: Register, clock_id: i32, tp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_clock_settime,
        [clock_id as Register, tp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `clock_getres(2)`.
pub fn clock_getres(nr: Register, clock_id: i32, tp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_clock_getres,
        [clock_id as Register, tp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `nanosleep(2)`.
pub fn nanosleep(nr: Register, rqtp: usize, rmtp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_nanosleep,
        [rqtp as Register, rmtp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `gettimeofday(2)`.
pub fn gettimeofday(nr: Register, tp: usize, tzp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_gettimeofday,
        [tp as Register, tzp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `settimeofday(2)`.
pub fn settimeofday(nr: Register, tv: usize, tzp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_settimeofday,
        [tv as Register, tzp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `adjfreq(2)`.
pub fn adjfreq(nr: Register, freq: usize, oldfreq: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_adjfreq,
        [freq as Register, oldfreq as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `adjtime(2)`.
pub fn adjtime(nr: Register, delta: usize, olddelta: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_adjtime,
        [delta as Register, olddelta as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `getitimer(2)`.
pub fn getitimer(nr: Register, which: i32, itv: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getitimer,
        [which as Register, itv as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setitimer(2)`.
pub fn setitimer(nr: Register, which: i32, itv: usize, oitv: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setitimer,
        [
            which as Register,
            itv as Register,
            oitv as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}
/* </CODE> */
