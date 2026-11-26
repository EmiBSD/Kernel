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
//! The event system calls (`kernel::file::event` and `kernel::file::select`: `kqueue(2)`,
//! `kevent(2)`, `select(2)`, `poll(2)` and their siblings).
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::{Register, Rval, SysResult};

use sysdefs::syscall::{
    SYS_kevent, SYS_kqueue, SYS_kqueue1, SYS_poll, SYS_ppoll, SYS_pselect, SYS_select,
};

use super::native;

/// `kevent(2)`.
pub fn kevent(
    nr: Register,
    fd: i32,
    changes: (usize, i32),
    events: (usize, i32),
    timeout: usize,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        fd as Register,
        changes.0 as Register,
        changes.1 as Register,
        events.0 as Register,
        events.1 as Register,
        timeout as Register,
    ];
    native(nr, SYS_kevent, args, rval)
}

/// `kqueue(2)`.
pub fn kqueue(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_kqueue, [0; 6], rval)
}

/// `kqueue1(2)`.
pub fn kqueue1(nr: Register, flags: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_kqueue1, [flags as Register, 0, 0, 0, 0, 0], rval)
}

/// `select(2)`.
pub fn select(nr: Register, nd: i32, sets: [usize; 3], tv: usize, rval: &mut Rval) -> SysResult {
    let args = [
        nd as Register,
        sets[0] as Register,
        sets[1] as Register,
        sets[2] as Register,
        tv as Register,
        0,
    ];
    native(nr, SYS_select, args, rval)
}

/// `pselect(2)`.
pub fn pselect(
    nr: Register,
    nd: i32,
    sets: [usize; 3],
    ts: usize,
    mask: usize,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        nd as Register,
        sets[0] as Register,
        sets[1] as Register,
        sets[2] as Register,
        ts as Register,
        mask as Register,
    ];
    native(nr, SYS_pselect, args, rval)
}

/// `poll(2)`.
pub fn poll(nr: Register, fds: usize, nfds: u32, timeout: i32, rval: &mut Rval) -> SysResult {
    let args = [
        fds as Register,
        nfds as Register,
        timeout as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_poll, args, rval)
}

/// `ppoll(2)`.
pub fn ppoll(
    nr: Register,
    fds: usize,
    nfds: u32,
    ts: usize,
    mask: usize,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        fds as Register,
        nfds as Register,
        ts as Register,
        mask as Register,
        0,
        0,
    ];
    native(nr, SYS_ppoll, args, rval)
}
/* </CODE> */
