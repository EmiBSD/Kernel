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
//! pipe(2), pipe2(2) and the descriptor system calls (`kernel::file`: `read(2)`, `close(2)`,
//! `dup(2)`, `fcntl(2)`, `ioctl(2)`, ...), which write the trap's return registers as the
//! C's `retval`.
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::ENOSYS;
use hal::syscall::{Register, Rval, SysResult};

use sysdefs::proc::syscall::{
    SYS_close, SYS_closefrom, SYS_dup, SYS_dup2, SYS_dup3, SYS_fcntl, SYS_flock, SYS_fpathconf,
    SYS_fstat, SYS_getdtablecount, SYS_ioctl, SYS_pipe, SYS_pipe2, SYS_pread, SYS_preadv,
    SYS_pwrite, SYS_pwritev, SYS_read, SYS_readv, SYS_utrace, SYS_write, SYS_writev,
};

use super::{native, syscall_legacy};

/// pipe(2) and pipe2(2): `(nr, fdp, flags)`, the system call's number (`SYS_pipe` or
/// `SYS_pipe2`), the user address of the two descriptors and `pipe2`'s flags (0 for `pipe`).
/// Either row runs through the system call prologue over return registers of its own, as
/// before decision 52; another number is `ENOSYS`.
pub fn pipe2(nr: Register, fdp: usize, flags: i32) -> SysResult {
    if nr != SYS_pipe as Register && nr != SYS_pipe2 as Register {
        return Err(ENOSYS);
    }
    let mut rval: Rval = [0, 0];
    syscall_legacy(
        nr,
        &[fdp as Register, flags as Register, 0, 0, 0, 0],
        &mut rval,
    )
}

/// `read(2)`.
pub fn read(nr: Register, fd: i32, buf: usize, nbyte: usize, rval: &mut Rval) -> SysResult {
    let args = [fd as Register, buf as Register, nbyte as Register, 0, 0, 0];
    native(nr, SYS_read, args, rval)
}

/// `write(2)`.
pub fn write(nr: Register, fd: i32, buf: usize, nbyte: usize, rval: &mut Rval) -> SysResult {
    let args = [fd as Register, buf as Register, nbyte as Register, 0, 0, 0];
    native(nr, SYS_write, args, rval)
}

/// `close(2)`.
pub fn close(nr: Register, fd: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_close, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `getdtablecount(2)`.
pub fn getdtablecount(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getdtablecount, [0; 6], rval)
}

/// `dup(2)`.
pub fn dup(nr: Register, fd: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_dup, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `fstat(2)`.
pub fn fstat(nr: Register, fd: i32, sb: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fstat,
        [fd as Register, sb as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `ioctl(2)`.
pub fn ioctl(nr: Register, fd: i32, com: u64, data: usize, rval: &mut Rval) -> SysResult {
    let args = [fd as Register, com as Register, data as Register, 0, 0, 0];
    native(nr, SYS_ioctl, args, rval)
}

/// `dup2(2)`.
pub fn dup2(nr: Register, from: i32, to: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_dup2,
        [from as Register, to as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fcntl(2)`.
pub fn fcntl(nr: Register, fd: i32, cmd: i32, arg: usize, rval: &mut Rval) -> SysResult {
    let args = [fd as Register, cmd as Register, arg as Register, 0, 0, 0];
    native(nr, SYS_fcntl, args, rval)
}

/// `dup3(2)`.
pub fn dup3(nr: Register, from: i32, to: i32, flags: i32, rval: &mut Rval) -> SysResult {
    let args = [from as Register, to as Register, flags as Register, 0, 0, 0];
    native(nr, SYS_dup3, args, rval)
}

/// `readv(2)`.
pub fn readv(nr: Register, fd: i32, iovp: usize, iovcnt: i32, rval: &mut Rval) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_readv, args, rval)
}

/// `writev(2)`.
pub fn writev(nr: Register, fd: i32, iovp: usize, iovcnt: i32, rval: &mut Rval) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_writev, args, rval)
}

/// `flock(2)`.
pub fn flock(nr: Register, fd: i32, how: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_flock,
        [fd as Register, how as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `pread(2)`.
pub fn pread(
    nr: Register,
    fd: i32,
    buf: usize,
    nbyte: usize,
    offset: i64,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        fd as Register,
        buf as Register,
        nbyte as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_pread, args, rval)
}

/// `pwrite(2)`.
pub fn pwrite(
    nr: Register,
    fd: i32,
    buf: usize,
    nbyte: usize,
    offset: i64,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        fd as Register,
        buf as Register,
        nbyte as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_pwrite, args, rval)
}

/// `preadv(2)`.
pub fn preadv(
    nr: Register,
    fd: i32,
    iovp: usize,
    iovcnt: i32,
    offset: i64,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_preadv, args, rval)
}

/// `pwritev(2)`.
pub fn pwritev(
    nr: Register,
    fd: i32,
    iovp: usize,
    iovcnt: i32,
    offset: i64,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        fd as Register,
        iovp as Register,
        iovcnt as Register,
        offset as Register,
        0,
        0,
    ];
    native(nr, SYS_pwritev, args, rval)
}

/// `fpathconf(2)`.
pub fn fpathconf(nr: Register, fd: i32, name: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fpathconf,
        [fd as Register, name as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `utrace(2)`.
pub fn utrace(nr: Register, label: usize, addr: usize, len: usize, rval: &mut Rval) -> SysResult {
    let args = [
        label as Register,
        addr as Register,
        len as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_utrace, args, rval)
}

/// `closefrom(2)`.
pub fn closefrom(nr: Register, fd: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_closefrom, [fd as Register, 0, 0, 0, 0, 0], rval)
}
/* </CODE> */
