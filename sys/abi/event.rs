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
//! The event system calls' translations (`kernel::file::event` and `select`: `kqueue(2)`,
//! `kevent(2)`, `select(2)`, `poll(2)` and their siblings).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, ptr, u_int};
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use sysdefs::proc::syscall::{
    SYS_kevent, SYS_kqueue, SYS_kqueue1, SYS_poll, SYS_ppoll, SYS_pselect, SYS_select,
};

/// 71 = `select(int nd, fd_set *in, fd_set *ou, fd_set *ex, struct timeval *tv)`.
pub(crate) fn sys_select(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::event::select(
        SYS_select as Register,
        int(args[0]),
        [ptr(args[1]), ptr(args[2]), ptr(args[3])],
        ptr(args[4]),
        rval,
    )
}

/// 72 = `kevent(int fd, const struct kevent *changelist, int nchanges, struct kevent
/// *eventlist, int nevents, const struct timespec *timeout)`.
pub(crate) fn sys_kevent(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::event::kevent(
        SYS_kevent as Register,
        int(args[0]),
        (ptr(args[1]), int(args[2])),
        (ptr(args[3]), int(args[4])),
        ptr(args[5]),
        rval,
    )
}

/// 109 = `ppoll(struct pollfd *fds, u_int nfds, const struct timespec *ts, const sigset_t
/// *mask)`.
pub(crate) fn sys_ppoll(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::event::ppoll(
        SYS_ppoll as Register,
        ptr(args[0]),
        u_int(args[1]),
        ptr(args[2]),
        ptr(args[3]),
        rval,
    )
}

/// 110 = `pselect(int nd, fd_set *in, fd_set *ou, fd_set *ex, const struct timespec *ts,
/// const sigset_t *mask)`.
pub(crate) fn sys_pselect(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::event::pselect(
        SYS_pselect as Register,
        int(args[0]),
        [ptr(args[1]), ptr(args[2]), ptr(args[3])],
        ptr(args[4]),
        ptr(args[5]),
        rval,
    )
}

/// 252 = `poll(struct pollfd *fds, u_int nfds, int timeout)`.
pub(crate) fn sys_poll(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::event::poll(
        SYS_poll as Register,
        ptr(args[0]),
        u_int(args[1]),
        int(args[2]),
        rval,
    )
}

/// 269 = `kqueue(void)`.
pub(crate) fn sys_kqueue(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::event::kqueue(SYS_kqueue as Register, rval)
}

/// 270 = `kqueue1(int flags)`.
pub(crate) fn sys_kqueue1(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::event::kqueue1(SYS_kqueue1 as Register, int(args[0]), rval)
}
/* </CODE> */
