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
//! The descriptor system calls' translations (`kernel::file`: pipe(2) and pipe2(2), `read(2)`,
//! `close(2)`, `dup(2)`, `fcntl(2)`, `ioctl(2)`, ...).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, off_t, ptr, size_t, u_long};
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use sysdefs::syscall::{
    SYS_close, SYS_closefrom, SYS_dup, SYS_dup2, SYS_dup3, SYS_fcntl, SYS_flock, SYS_fpathconf,
    SYS_fstat, SYS_getdtablecount, SYS_ioctl, SYS_pipe, SYS_pipe2, SYS_pread, SYS_preadv,
    SYS_pwrite, SYS_pwritev, SYS_read, SYS_readv, SYS_utrace, SYS_write, SYS_writev,
};

/// 101 = `pipe2(int *fdp, int flags)`: the kernel's pipe(2) with the flags.
pub(crate) fn sys_pipe2(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::file::pipe2(SYS_pipe2 as Register, ptr(args[0]), int(args[1]))
}

/// 263 = `pipe(int *fdp)`: the kernel's pipe(2), no flags.
pub(crate) fn sys_pipe(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::file::pipe2(SYS_pipe as Register, ptr(args[0]), 0)
}

/// 3 = `read(int fd, void *buf, size_t nbyte)`.
pub(crate) fn sys_read(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::read(
        SYS_read as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        rval,
    )
}

/// 4 = `write(int fd, const void *buf, size_t nbyte)`.
pub(crate) fn sys_write(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::write(
        SYS_write as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        rval,
    )
}

/// 6 = `close(int fd)`.
pub(crate) fn sys_close(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::close(SYS_close as Register, int(args[0]), rval)
}

/// 18 = `getdtablecount(void)`.
pub(crate) fn sys_getdtablecount(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::getdtablecount(SYS_getdtablecount as Register, rval)
}

/// 41 = `dup(int fd)`.
pub(crate) fn sys_dup(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::dup(SYS_dup as Register, int(args[0]), rval)
}

/// 53 = `fstat(int fd, struct stat *sb)`.
pub(crate) fn sys_fstat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::fstat(SYS_fstat as Register, int(args[0]), ptr(args[1]), rval)
}

/// 54 = `ioctl(int fd, u_long com, void *data)`.
pub(crate) fn sys_ioctl(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::ioctl(
        SYS_ioctl as Register,
        int(args[0]),
        u_long(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// 90 = `dup2(int from, int to)`.
pub(crate) fn sys_dup2(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::dup2(SYS_dup2 as Register, int(args[0]), int(args[1]), rval)
}

/// 92 = `fcntl(int fd, int cmd, void *arg)`.
pub(crate) fn sys_fcntl(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::fcntl(
        SYS_fcntl as Register,
        int(args[0]),
        int(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// 102 = `dup3(int from, int to, int flags)`.
pub(crate) fn sys_dup3(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::dup3(
        SYS_dup3 as Register,
        int(args[0]),
        int(args[1]),
        int(args[2]),
        rval,
    )
}

/// 120 = `readv(int fd, const struct iovec *iovp, int iovcnt)`.
pub(crate) fn sys_readv(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::readv(
        SYS_readv as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        rval,
    )
}

/// 121 = `writev(int fd, const struct iovec *iovp, int iovcnt)`.
pub(crate) fn sys_writev(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::writev(
        SYS_writev as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        rval,
    )
}

/// 131 = `flock(int fd, int how)`.
pub(crate) fn sys_flock(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::flock(SYS_flock as Register, int(args[0]), int(args[1]), rval)
}

/// 169 = `pread(int fd, void *buf, size_t nbyte, off_t offset)`.
pub(crate) fn sys_pread(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::pread(
        SYS_pread as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        off_t(args[3]),
        rval,
    )
}

/// 170 = `pwrite(int fd, const void *buf, size_t nbyte, off_t offset)`.
pub(crate) fn sys_pwrite(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::pwrite(
        SYS_pwrite as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        off_t(args[3]),
        rval,
    )
}

/// 171 = `preadv(int fd, const struct iovec *iovp, int iovcnt, off_t offset)`.
pub(crate) fn sys_preadv(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::preadv(
        SYS_preadv as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        off_t(args[3]),
        rval,
    )
}

/// 172 = `pwritev(int fd, const struct iovec *iovp, int iovcnt, off_t offset)`.
pub(crate) fn sys_pwritev(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::pwritev(
        SYS_pwritev as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        off_t(args[3]),
        rval,
    )
}

/// 192 = `fpathconf(int fd, int name)`.
pub(crate) fn sys_fpathconf(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::fpathconf(SYS_fpathconf as Register, int(args[0]), int(args[1]), rval)
}

/// 209 = `utrace(const char *label, const void *addr, size_t len)`.
pub(crate) fn sys_utrace(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::utrace(
        SYS_utrace as Register,
        ptr(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        rval,
    )
}

/// 287 = `closefrom(int fd)`.
pub(crate) fn sys_closefrom(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::file::closefrom(SYS_closefrom as Register, int(args[0]), rval)
}
/* </CODE> */
