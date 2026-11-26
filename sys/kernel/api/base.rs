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
//! The kernel's base services' system calls (`kernel::base`: `sysctl(2)`, `sendsyslog(2)`,
//! `reboot(2)`).
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::{Register, Rval, SysResult};

use sysdefs::syscall::{SYS_reboot, SYS_sendsyslog, SYS_sysctl};

use super::native;

/// `KernelApi::sendsyslog`, the module's `Native` sendsyslog(2): the thread on this CPU runs
/// `mi_syscall` on `sysent[112]` (the prologue still `bsd`'s: pledge, the pinned syscalls,
/// `NOLOCK`), whose row is [`sys_sendsyslog`]; another number is `ENOSYS`.
pub fn sendsyslog(
    nr: Register,
    buf: usize,
    nbyte: usize,
    flags: i32,
    rval: &mut Rval,
) -> Result<(), i32> {
    let args = [
        buf as Register,
        nbyte as Register,
        flags as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_sendsyslog, args, rval)
}

/// `sysctl(2)`: its row through the system call prologue (`mi_syscall`: pledge, the pinned
/// syscalls, the kernel-lock rule; the row is `SY_NOLOCK`, as the C's).
#[allow(
    clippy::too_many_arguments,
    reason = "the call's shape: the number, six arguments, the return registers"
)]
pub fn sysctl(
    nr: Register,
    name: usize,
    namelen: u32,
    old: usize,
    oldlenp: usize,
    new: usize,
    newlen: usize,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        name as Register,
        namelen as Register,
        old as Register,
        oldlenp as Register,
        new as Register,
        newlen as Register,
    ];
    native(nr, SYS_sysctl, args, rval)
}

/// [`sys_reboot`]'s row through the system call prologue
/// (`mi_syscall`: pledge, the pinned syscalls, the kernel-lock rule).
pub fn reboot(nr: Register, opt: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_reboot, [opt as Register, 0, 0, 0, 0, 0], rval)
}
/* </CODE> */
