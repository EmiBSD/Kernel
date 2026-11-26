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
//! The socket system calls' translations (`kernel::net`: `socket(2)`, `bind(2)`, `accept(2)`,
//! `sendmsg(2)`, ...).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, ptr, size_t, u_int};
use hal::api::KernelApi;
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use hal::sys::syscall::{
    SYS_accept, SYS_accept4, SYS_bind, SYS_connect, SYS_getpeername, SYS_getsockname,
    SYS_getsockopt, SYS_listen, SYS_recvfrom, SYS_recvmmsg, SYS_recvmsg, SYS_sendmmsg, SYS_sendmsg,
    SYS_sendto, SYS_setsockopt, SYS_shutdown, SYS_socket, SYS_socketpair, SYS_ypconnect,
};

/// 27 = `ssize_t recvmsg(int s, struct msghdr *msg, int flags)`.
pub(crate) fn sys_recvmsg(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.recvmsg)(
        SYS_recvmsg as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        rval,
    )
}

/// 28 = `ssize_t sendmsg(int s, const struct msghdr *msg, int flags)`.
pub(crate) fn sys_sendmsg(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.sendmsg)(
        SYS_sendmsg as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        rval,
    )
}

/// 29 = `ssize_t recvfrom(int s, void *buf, size_t len, int flags, struct sockaddr *from, socklen_t *fromlenaddr)`.
pub(crate) fn sys_recvfrom(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.recvfrom)(
        SYS_recvfrom as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        int(args[3]),
        ptr(args[4]),
        ptr(args[5]),
        rval,
    )
}

/// 30 = `int accept(int s, struct sockaddr *name, socklen_t *anamelen)`.
pub(crate) fn sys_accept(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.accept)(
        SYS_accept as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// 31 = `int getpeername(int fdes, struct sockaddr *asa, socklen_t *alen)`.
pub(crate) fn sys_getpeername(
    api: &'static KernelApi,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    (api.socket.getpeername)(
        SYS_getpeername as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// 32 = `int getsockname(int fdes, struct sockaddr *asa, socklen_t *alen)`.
pub(crate) fn sys_getsockname(
    api: &'static KernelApi,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    (api.socket.getsockname)(
        SYS_getsockname as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// 93 = `int accept4(int s, struct sockaddr *name, socklen_t *anamelen, int flags)`.
pub(crate) fn sys_accept4(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.accept4)(
        SYS_accept4 as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        int(args[3]),
        rval,
    )
}

/// 97 = `int socket(int domain, int type, int protocol)`.
pub(crate) fn sys_socket(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.socket)(
        SYS_socket as Register,
        int(args[0]),
        int(args[1]),
        int(args[2]),
        rval,
    )
}

/// 98 = `int connect(int s, const struct sockaddr *name, socklen_t namelen)`.
pub(crate) fn sys_connect(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.connect)(
        SYS_connect as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 104 = `int bind(int s, const struct sockaddr *name, socklen_t namelen)`.
pub(crate) fn sys_bind(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.bind)(
        SYS_bind as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 105 = `int setsockopt(int s, int level, int name, const void *val, socklen_t valsize)`.
pub(crate) fn sys_setsockopt(
    api: &'static KernelApi,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    (api.socket.setsockopt)(
        SYS_setsockopt as Register,
        int(args[0]),
        int(args[1]),
        int(args[2]),
        ptr(args[3]),
        u_int(args[4]),
        rval,
    )
}

/// 106 = `int listen(int s, int backlog)`.
pub(crate) fn sys_listen(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.listen)(SYS_listen as Register, int(args[0]), int(args[1]), rval)
}

/// 116 = `int recvmmsg(int s, struct mmsghdr *mmsg, unsigned int vlen, int flags, struct timespec *timeout)`.
pub(crate) fn sys_recvmmsg(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.recvmmsg)(
        SYS_recvmmsg as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        int(args[3]),
        ptr(args[4]),
        rval,
    )
}

/// 117 = `int sendmmsg(int s, struct mmsghdr *mmsg, unsigned int vlen, int flags)`.
pub(crate) fn sys_sendmmsg(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.sendmmsg)(
        SYS_sendmmsg as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        int(args[3]),
        rval,
    )
}

/// 118 = `int getsockopt(int s, int level, int name, void *val, socklen_t *avalsize)`.
pub(crate) fn sys_getsockopt(
    api: &'static KernelApi,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    (api.socket.getsockopt)(
        SYS_getsockopt as Register,
        int(args[0]),
        int(args[1]),
        int(args[2]),
        ptr(args[3]),
        ptr(args[4]),
        rval,
    )
}

/// 133 = `ssize_t sendto(int s, const void *buf, size_t len, int flags, const struct sockaddr *to, socklen_t tolen)`.
pub(crate) fn sys_sendto(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.sendto)(
        SYS_sendto as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        int(args[3]),
        ptr(args[4]),
        u_int(args[5]),
        rval,
    )
}

/// 134 = `int shutdown(int s, int how)`.
pub(crate) fn sys_shutdown(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.shutdown)(SYS_shutdown as Register, int(args[0]), int(args[1]), rval)
}

/// 135 = `int socketpair(int domain, int type, int protocol, int *rsv)`.
pub(crate) fn sys_socketpair(
    api: &'static KernelApi,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    (api.socket.socketpair)(
        SYS_socketpair as Register,
        int(args[0]),
        int(args[1]),
        int(args[2]),
        ptr(args[3]),
        rval,
    )
}

/// 150 = `int ypconnect(int type)`.
pub(crate) fn sys_ypconnect(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.socket.ypconnect)(SYS_ypconnect as Register, int(args[0]), rval)
}
/* </CODE> */
