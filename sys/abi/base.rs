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
//! The translations of the kernel's base services (`kernel::base`: `sysctl(2)`,
//! `sendsyslog(2)`, `reboot(2)`).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, ptr, size_t, u_int};
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use hal::sys::syscall::{SYS_reboot, SYS_sendsyslog, SYS_sysctl};

/// 112 = `sendsyslog(const char *buf, size_t nbyte, int flags)`.
pub(crate) fn sys_sendsyslog(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::base::sendsyslog(
        SYS_sendsyslog as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
        rval,
    )
}

/// 202 = `sysctl(const int *name, u_int namelen, void *old, size_t *oldlenp, void *new,
/// size_t newlen)`.
pub(crate) fn sys_sysctl(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::base::sysctl(
        SYS_sysctl as Register,
        ptr(args[0]),
        u_int(args[1]),
        ptr(args[2]),
        ptr(args[3]),
        ptr(args[4]),
        size_t(args[5]),
        rval,
    )
}

/// 55 = `reboot(int opt)`.
pub(crate) fn sys_reboot(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::base::reboot(SYS_reboot as Register, int(args[0]), rval)
}
/* </CODE> */
