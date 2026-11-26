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
//! The socket system calls (`kernel::net`: `socket(2)`, `bind(2)`, `accept(2)`,
//! `sendmsg(2)`, ...).
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::{Register, Rval, SysResult};

use sysdefs::syscall::{
    SYS_accept, SYS_accept4, SYS_bind, SYS_connect, SYS_getpeername, SYS_getsockname,
    SYS_getsockopt, SYS_listen, SYS_recvfrom, SYS_recvmmsg, SYS_recvmsg, SYS_sendmmsg, SYS_sendmsg,
    SYS_sendto, SYS_setsockopt, SYS_shutdown, SYS_socket, SYS_socketpair, SYS_ypconnect,
};

use super::native;

/// `recvmsg(2)`.
pub fn recvmsg(nr: Register, s: i32, msg: usize, flags: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_recvmsg,
        [s as Register, msg as Register, flags as Register, 0, 0, 0],
        rval,
    )
}

/// `sendmsg(2)`.
pub fn sendmsg(nr: Register, s: i32, msg: usize, flags: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_sendmsg,
        [s as Register, msg as Register, flags as Register, 0, 0, 0],
        rval,
    )
}

/// `recvfrom(2)`.
#[allow(
    clippy::too_many_arguments,
    reason = "the call's shape: the number, six arguments, the return registers"
)]
pub fn recvfrom(
    nr: Register,
    s: i32,
    buf: usize,
    len: usize,
    flags: i32,
    from: usize,
    fromlenaddr: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_recvfrom,
        [
            s as Register,
            buf as Register,
            len as Register,
            flags as Register,
            from as Register,
            fromlenaddr as Register,
        ],
        rval,
    )
}

/// `accept(2)`.
pub fn accept(nr: Register, s: i32, name: usize, anamelen: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_accept,
        [
            s as Register,
            name as Register,
            anamelen as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `getpeername(2)`.
pub fn getpeername(nr: Register, fdes: i32, asa: usize, alen: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getpeername,
        [fdes as Register, asa as Register, alen as Register, 0, 0, 0],
        rval,
    )
}

/// `getsockname(2)`.
pub fn getsockname(nr: Register, fdes: i32, asa: usize, alen: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getsockname,
        [fdes as Register, asa as Register, alen as Register, 0, 0, 0],
        rval,
    )
}

/// `accept4(2)`.
pub fn accept4(
    nr: Register,
    s: i32,
    name: usize,
    anamelen: usize,
    flags: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_accept4,
        [
            s as Register,
            name as Register,
            anamelen as Register,
            flags as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `socket(2)`.
pub fn socket(nr: Register, domain: i32, type_: i32, protocol: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_socket,
        [
            domain as Register,
            type_ as Register,
            protocol as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `connect(2)`.
pub fn connect(nr: Register, s: i32, name: usize, namelen: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_connect,
        [
            s as Register,
            name as Register,
            namelen as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `bind(2)`.
pub fn bind(nr: Register, s: i32, name: usize, namelen: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_bind,
        [
            s as Register,
            name as Register,
            namelen as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `setsockopt(2)`.
pub fn setsockopt(
    nr: Register,
    s: i32,
    level: i32,
    name: i32,
    val: usize,
    valsize: u32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_setsockopt,
        [
            s as Register,
            level as Register,
            name as Register,
            val as Register,
            valsize as Register,
            0,
        ],
        rval,
    )
}

/// `listen(2)`.
pub fn listen(nr: Register, s: i32, backlog: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_listen,
        [s as Register, backlog as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `recvmmsg(2)`.
pub fn recvmmsg(
    nr: Register,
    s: i32,
    mmsg: usize,
    vlen: u32,
    flags: i32,
    timeout: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_recvmmsg,
        [
            s as Register,
            mmsg as Register,
            vlen as Register,
            flags as Register,
            timeout as Register,
            0,
        ],
        rval,
    )
}

/// `sendmmsg(2)`.
pub fn sendmmsg(
    nr: Register,
    s: i32,
    mmsg: usize,
    vlen: u32,
    flags: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_sendmmsg,
        [
            s as Register,
            mmsg as Register,
            vlen as Register,
            flags as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `getsockopt(2)`.
pub fn getsockopt(
    nr: Register,
    s: i32,
    level: i32,
    name: i32,
    val: usize,
    avalsize: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_getsockopt,
        [
            s as Register,
            level as Register,
            name as Register,
            val as Register,
            avalsize as Register,
            0,
        ],
        rval,
    )
}

/// `sendto(2)`.
#[allow(
    clippy::too_many_arguments,
    reason = "the call's shape: the number, six arguments, the return registers"
)]
pub fn sendto(
    nr: Register,
    s: i32,
    buf: usize,
    len: usize,
    flags: i32,
    to: usize,
    tolen: u32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_sendto,
        [
            s as Register,
            buf as Register,
            len as Register,
            flags as Register,
            to as Register,
            tolen as Register,
        ],
        rval,
    )
}

/// `shutdown(2)`.
pub fn shutdown(nr: Register, s: i32, how: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_shutdown,
        [s as Register, how as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `socketpair(2)`.
pub fn socketpair(
    nr: Register,
    domain: i32,
    type_: i32,
    protocol: i32,
    rsv: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_socketpair,
        [
            domain as Register,
            type_ as Register,
            protocol as Register,
            rsv as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `ypconnect(2)`.
pub fn ypconnect(nr: Register, type_: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_ypconnect, [type_ as Register, 0, 0, 0, 0, 0], rval)
}
/* </CODE> */
