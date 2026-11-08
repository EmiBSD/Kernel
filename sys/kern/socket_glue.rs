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

/*
 * Copyright (c) 1982, 1986, 1989, 1990, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)uipc_syscalls.c	8.4 (Berkeley) 2/21/94
 */

/*
 * Copyright (c) 1982, 1986, 1990, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)sys_socket.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The sockets' glue to what is still `bsd`'s: the call-site adapter of the socket system
//! calls (`kernel::net::syscalls`, `sys/kernel/net/syscalls.rs`) and of the socket file
//! (`kernel::net::file`), with 0 `unsafe`, while `sysent[]`, the system call prologue and the
//! file's operations (`struct fileops`, `FileGlue::Ops`) and object (`FileGlue::Data`) are
//! `bsd`'s.
//!
//! Upstream: sys/kern/uipc_syscalls.c @ 3ce1f3f79392
//! Upstream: sys/kern/sys_socket.c @ 3ce1f3f79392
//! LZ: sys/kern/uipc_syscalls.rs@3c62ede68802
//! LZ: sys/kern/sys_socket.rs@3c62ede68802
//!
//! Every decision the socket system calls make is the kernel crate's; this file holds what
//! `bsd`'s tables need to reach it: the `sysent[]` rows of the nineteen calls (each unpacks
//! its `struct sys_*_args` into the kernel function's typed arguments and puts its answer in
//! `retval[0]`), `setrtable(2)`'s and `getrtable(2)`'s rows over `kernel::prot`, the
//! `Native` entries of `KernelApi::socket` (`SOCKET_ENTRIES`: `mi_syscall` on the same
//! rows, the prologue's pledge and pinned system calls), `socketops` over the kernel's
//! `SocketFileOps` with `fp_install` and `fp_socket` for `bsd`'s callers.
//!
//! ## Deviations
//! - The calls' deviations are `kernel::net::syscalls`'s (its `## Deviations`): `KTRACE`
//!   not configured, an mbuf or an iovec array that cannot be had.
//! - [`fp_socket`] is `fp->f_data` of a socket file as the `&'static Socket` LZ's callers
//!   keep (the file's reference keeps the socket).
//!
//! ## Redesign
//! - A call-site adapter, listed as such (the `pipe_glue` precedent): LZ's
//!   `uipc_syscalls.rs` and `sys_socket.rs` minus every decision, which moved to
//!   `sys/kernel/net/syscalls.rs` (N6.2b) and `sys/kernel/net/file.rs` (N6.2a). The rows keep
//!   LZ's names and signatures, which `cargo xtask gen-syscalls` finds for `init_sysent.rs`;
//!   `fp_socket` and `socketops` keep theirs for `bsd`'s other users (`kern_descrip`,
//!   `kern_sysctl`, nfs). nfs's `sockargs` and `getsock` are the kernel's (`park` makes the
//!   address mbuf the `&'static` nfs keeps).
//! - `BsdIfLegacy`, the interface layer's legs of the socket file (the `IfLegacy` row of
//!   `bridges.toml`) that `sys_socket.rs` held with `ifioctl`'s one `unsafe` call, is the
//!   interface layer's own since N6.2b (`sys/net/if_.rs`, beside `ifioctl` and `net_tq`, the
//!   `BsdPf` precedent of `sys/net/pf.rs`), so this file is `forbid`; it goes with N6.3a.

use hal::api::SocketEntries;
use hal::syscall::SysResult;
use hal::user::UserPtr;
use kernel::file::FileOps;
use kernel::net::file::SocketFileOps;
use kernel::net::syscalls as ks;

use crate::kern::kern_descrip::native;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::file::{DTYPE_SOCKET, File, Fileops};
use crate::sys::proc::Proc;
use crate::sys::socketvar::{Socket, as_static};
use crate::sys::stat::Stat;
use crate::sys::syscall::{
    SYS_accept, SYS_accept4, SYS_bind, SYS_connect, SYS_getpeername, SYS_getsockname,
    SYS_getsockopt, SYS_listen, SYS_recvfrom, SYS_recvmmsg, SYS_recvmsg, SYS_sendmmsg, SYS_sendmsg,
    SYS_sendto, SYS_setsockopt, SYS_shutdown, SYS_socket, SYS_socketpair, SYS_ypconnect,
};
use crate::sys::syscallargs::{
    SysAccept4Args, SysAcceptArgs, SysBindArgs, SysConnectArgs, SysGetpeernameArgs,
    SysGetsocknameArgs, SysGetsockoptArgs, SysListenArgs, SysRecvfromArgs, SysRecvmmsgArgs,
    SysRecvmsgArgs, SysSendmmsgArgs, SysSendmsgArgs, SysSendtoArgs, SysSetrtableArgs,
    SysSetsockoptArgs, SysShutdownArgs, SysSocketArgs, SysSocketpairArgs, SysYpconnectArgs,
};
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::Register;
use crate::sys::uio::Uio;

/// `socket(2)` (`sysent[]`'s row: `kernel::net::syscalls::sys_socket`).
pub fn sys_socket(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSocketArgs = sysargs(v);
    retval[0] =
        ks::sys_socket(p, uap.domain.get(), uap.r#type.get(), uap.protocol.get())? as Register;
    Ok(())
}

/// `bind(2)` (`sysent[]`'s row).
pub fn sys_bind(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysBindArgs = sysargs(v);
    ks::sys_bind(
        p,
        uap.s.get(),
        UserPtr::new(uap.name.get().addr()),
        uap.namelen.get(),
    )
}

/// `listen(2)` (`sysent[]`'s row).
pub fn sys_listen(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysListenArgs = sysargs(v);
    ks::sys_listen(p, uap.s.get(), uap.backlog.get())
}

/// `accept(2)` (`sysent[]`'s row).
pub fn sys_accept(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAcceptArgs = sysargs(v);
    retval[0] = ks::sys_accept(
        p,
        uap.s.get(),
        UserPtr::new(uap.name.get().addr()),
        UserPtr::new(uap.anamelen.get().addr()),
    )? as Register;
    Ok(())
}

/// `accept4(2)` (`sysent[]`'s row).
pub fn sys_accept4(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAccept4Args = sysargs(v);
    retval[0] = ks::sys_accept4(
        p,
        uap.s.get(),
        UserPtr::new(uap.name.get().addr()),
        UserPtr::new(uap.anamelen.get().addr()),
        uap.flags.get(),
    )? as Register;
    Ok(())
}

/// `connect(2)` (`sysent[]`'s row).
pub fn sys_connect(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysConnectArgs = sysargs(v);
    ks::sys_connect(
        p,
        uap.s.get(),
        UserPtr::new(uap.name.get().addr()),
        uap.namelen.get(),
    )
}

/// `socketpair(2)` (`sysent[]`'s row).
pub fn sys_socketpair(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSocketpairArgs = sysargs(v);
    ks::sys_socketpair(
        p,
        uap.domain.get(),
        uap.r#type.get(),
        uap.protocol.get(),
        UserPtr::new(uap.rsv.get().addr()),
    )
}

/// `sendto(2)` (`sysent[]`'s row).
pub fn sys_sendto(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSendtoArgs = sysargs(v);
    retval[0] = ks::sys_sendto(
        p,
        uap.s.get(),
        uap.buf.get().addr(),
        uap.len.get(),
        uap.flags.get(),
        UserPtr::new(uap.to.get().addr()),
        uap.tolen.get(),
    )? as Register;
    Ok(())
}

/// `sendmsg(2)` (`sysent[]`'s row).
pub fn sys_sendmsg(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSendmsgArgs = sysargs(v);
    retval[0] = ks::sys_sendmsg(
        p,
        uap.s.get(),
        UserPtr::new(uap.msg.get().addr()),
        uap.flags.get(),
    )? as Register;
    Ok(())
}

/// `sendmmsg(2)` (`sysent[]`'s row).
pub fn sys_sendmmsg(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSendmmsgArgs = sysargs(v);
    retval[0] = ks::sys_sendmmsg(
        p,
        uap.s.get(),
        UserPtr::new(uap.mmsg.get().addr()),
        uap.vlen.get(),
        uap.flags.get(),
    )? as Register;
    Ok(())
}

/// `recvfrom(2)` (`sysent[]`'s row).
pub fn sys_recvfrom(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRecvfromArgs = sysargs(v);
    retval[0] = ks::sys_recvfrom(
        p,
        uap.s.get(),
        uap.buf.get().addr(),
        uap.len.get(),
        uap.flags.get(),
        UserPtr::new(uap.from.get().addr()),
        UserPtr::new(uap.fromlenaddr.get().addr()),
    )? as Register;
    Ok(())
}

/// `recvmsg(2)` (`sysent[]`'s row).
pub fn sys_recvmsg(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRecvmsgArgs = sysargs(v);
    retval[0] = ks::sys_recvmsg(
        p,
        uap.s.get(),
        UserPtr::new(uap.msg.get().addr()),
        uap.flags.get(),
    )? as Register;
    Ok(())
}

/// `recvmmsg(2)` (`sysent[]`'s row).
pub fn sys_recvmmsg(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRecvmmsgArgs = sysargs(v);
    retval[0] = ks::sys_recvmmsg(
        p,
        uap.s.get(),
        UserPtr::new(uap.mmsg.get().addr()),
        uap.vlen.get(),
        uap.flags.get(),
        UserPtr::new(uap.timeout.get().addr()),
    )? as Register;
    Ok(())
}

/// `shutdown(2)` (`sysent[]`'s row).
pub fn sys_shutdown(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysShutdownArgs = sysargs(v);
    ks::sys_shutdown(p, uap.s.get(), uap.how.get())
}

/// `setsockopt(2)` (`sysent[]`'s row).
pub fn sys_setsockopt(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetsockoptArgs = sysargs(v);
    ks::sys_setsockopt(
        p,
        uap.s.get(),
        uap.level.get(),
        uap.name.get(),
        UserPtr::new(uap.val.get().addr()),
        uap.valsize.get(),
    )
}

/// `getsockopt(2)` (`sysent[]`'s row).
pub fn sys_getsockopt(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetsockoptArgs = sysargs(v);
    ks::sys_getsockopt(
        p,
        uap.s.get(),
        uap.level.get(),
        uap.name.get(),
        UserPtr::new(uap.val.get().addr()),
        UserPtr::new(uap.avalsize.get().addr()),
    )
}

/// `getsockname(2)` (`sysent[]`'s row).
pub fn sys_getsockname(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetsocknameArgs = sysargs(v);
    ks::sys_getsockname(
        p,
        uap.fdes.get(),
        UserPtr::new(uap.asa.get().addr()),
        UserPtr::new(uap.alen.get().addr()),
    )
}

/// `getpeername(2)` (`sysent[]`'s row).
pub fn sys_getpeername(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetpeernameArgs = sysargs(v);
    ks::sys_getpeername(
        p,
        uap.fdes.get(),
        UserPtr::new(uap.asa.get().addr()),
        UserPtr::new(uap.alen.get().addr()),
    )
}

/// `setrtable(2)`: moves the process to routing table `rtableid`; only root may leave a
/// table other than 0 (`sysent[]`'s row: `kernel::prot::sys_setrtable`).
pub fn sys_setrtable(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSetrtableArgs = sysargs(v);
    kernel::prot::sys_setrtable(p, uap.rtableid.get())
}

/// `getrtable(2)`: the process's routing table (`sysent[]`'s row:
/// `kernel::prot::sys_getrtable`).
pub fn sys_getrtable(p: &Proc, _v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    retval[0] = kernel::prot::sys_getrtable(p) as Register;
    Ok(())
}

/// `ypconnect(2)` (`sysent[]`'s row).
pub fn sys_ypconnect(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysYpconnectArgs = sysargs(v);
    retval[0] = ks::sys_ypconnect(p, uap.r#type.get())? as Register;
    Ok(())
}

/// `KernelApi::socket`: the `Native` entries of the socket system calls, each `mi_syscall`
/// on its own `sysent[]` row (`kern_descrip::native`: the prologue, pledge, the pinned
/// syscalls, as on the `Legacy` path; every row is `NOLOCK`).
pub const SOCKET_ENTRIES: SocketEntries = SocketEntries {
    recvmsg: native_recvmsg,
    sendmsg: native_sendmsg,
    recvfrom: native_recvfrom,
    accept: native_accept,
    getpeername: native_getpeername,
    getsockname: native_getsockname,
    accept4: native_accept4,
    socket: native_socket,
    connect: native_connect,
    bind: native_bind,
    setsockopt: native_setsockopt,
    listen: native_listen,
    recvmmsg: native_recvmmsg,
    sendmmsg: native_sendmmsg,
    getsockopt: native_getsockopt,
    sendto: native_sendto,
    shutdown: native_shutdown,
    socketpair: native_socketpair,
    ypconnect: native_ypconnect,
};

/// `KernelApi::socket.recvmsg`.
fn native_recvmsg(
    nr: Register,
    s: i32,
    msg: usize,
    flags: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_recvmsg,
        [s as Register, msg as Register, flags as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::socket.sendmsg`.
fn native_sendmsg(
    nr: Register,
    s: i32,
    msg: usize,
    flags: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_sendmsg,
        [s as Register, msg as Register, flags as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::socket.recvfrom`.
#[allow(
    clippy::too_many_arguments,
    reason = "the entry's shape: the number, six arguments, the return registers"
)]
fn native_recvfrom(
    nr: Register,
    s: i32,
    buf: usize,
    len: usize,
    flags: i32,
    from: usize,
    fromlenaddr: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.accept`.
fn native_accept(
    nr: Register,
    s: i32,
    name: usize,
    anamelen: usize,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::socket.getpeername`.
fn native_getpeername(
    nr: Register,
    fdes: i32,
    asa: usize,
    alen: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getpeername,
        [fdes as Register, asa as Register, alen as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::socket.getsockname`.
fn native_getsockname(
    nr: Register,
    fdes: i32,
    asa: usize,
    alen: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getsockname,
        [fdes as Register, asa as Register, alen as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::socket.accept4`.
fn native_accept4(
    nr: Register,
    s: i32,
    name: usize,
    anamelen: usize,
    flags: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.socket`.
fn native_socket(
    nr: Register,
    domain: i32,
    type_: i32,
    protocol: i32,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::socket.connect`.
fn native_connect(
    nr: Register,
    s: i32,
    name: usize,
    namelen: u32,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::socket.bind`.
fn native_bind(
    nr: Register,
    s: i32,
    name: usize,
    namelen: u32,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::socket.setsockopt`.
fn native_setsockopt(
    nr: Register,
    s: i32,
    level: i32,
    name: i32,
    val: usize,
    valsize: u32,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.listen`.
fn native_listen(nr: Register, s: i32, backlog: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_listen,
        [s as Register, backlog as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::socket.recvmmsg`.
fn native_recvmmsg(
    nr: Register,
    s: i32,
    mmsg: usize,
    vlen: u32,
    flags: i32,
    timeout: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.sendmmsg`.
fn native_sendmmsg(
    nr: Register,
    s: i32,
    mmsg: usize,
    vlen: u32,
    flags: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.getsockopt`.
fn native_getsockopt(
    nr: Register,
    s: i32,
    level: i32,
    name: i32,
    val: usize,
    avalsize: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.sendto`.
#[allow(
    clippy::too_many_arguments,
    reason = "the entry's shape: the number, six arguments, the return registers"
)]
fn native_sendto(
    nr: Register,
    s: i32,
    buf: usize,
    len: usize,
    flags: i32,
    to: usize,
    tolen: u32,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.shutdown`.
fn native_shutdown(nr: Register, s: i32, how: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_shutdown,
        [s as Register, how as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::socket.socketpair`.
fn native_socketpair(
    nr: Register,
    domain: i32,
    type_: i32,
    protocol: i32,
    rsv: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::socket.ypconnect`.
fn native_ypconnect(nr: Register, type_: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_ypconnect, [type_ as Register, 0, 0, 0, 0, 0], rval)
}

/// The kernel's operations of a socket file.
static OPS: SocketFileOps<BsdProcGlue> = SocketFileOps::NEW;

/// `socketops`.
pub static SOCKETOPS: Fileops = Fileops {
    fo_read: soo_read,
    fo_write: soo_write,
    fo_ioctl: soo_ioctl,
    fo_kqfilter: soo_kqfilter,
    fo_stat: soo_stat,
    fo_close: soo_close,
    fo_seek: None,
};

/// `fp->f_type = DTYPE_SOCKET; fp->f_ops = &socketops; fp->f_data = so`: `so`'s reference
/// becomes the file's (its last close is the socket's `soclose`).
pub fn fp_install(fp: &File, so: crate::sys::socketvar::SocketRef) {
    let (ops, data) =
        <crate::kern::kern_descrip::BsdFileGlue as kernel::file::FileGlue>::socket_file(so);
    fp.install(DTYPE_SOCKET, ops, data);
}

/// `(struct socket *)fp->f_data` of a socket file.
pub fn fp_socket(fp: &File) -> &'static Socket {
    match fp.data() {
        Some(crate::kern::kern_descrip::BsdFileData::Socket(so)) if fp.f_type() == DTYPE_SOCKET => {
            as_static(so)
        }
        _ => panic(format_args!("file {:#x}: not a socket", fp.addr())),
    }
}

/// `soo_read(fp, uio, fflags)`.
pub fn soo_read(fp: &File, uio: &mut Uio<'_>, fflags: i32) -> Result<(), Errno> {
    OPS.read(fp, uio, fflags)
}

/// `soo_write(fp, uio, fflags)`.
pub fn soo_write(fp: &File, uio: &mut Uio<'_>, fflags: i32) -> Result<(), Errno> {
    OPS.write(fp, uio, fflags)
}

/// `soo_ioctl(fp, cmd, data, p)`.
pub fn soo_ioctl(fp: &File, cmd: u64, data: &mut [u8], p: &Proc) -> Result<(), Errno> {
    OPS.ioctl(fp, cmd, data, p)
}

/// `soo_kqfilter(fp, kn)`.
pub fn soo_kqfilter(fp: &File, kn: &Knote) -> Result<(), Errno> {
    OPS.kqfilter(fp, kn)
}

/// `soo_stat(fp, ub, p)`.
pub fn soo_stat(fp: &File, ub: &mut Stat, p: &Proc) -> Result<(), Errno> {
    OPS.stat(fp, ub, p)
}

/// `soo_close(fp, p)`.
pub fn soo_close(fp: &File, p: Option<&Proc>) -> Result<(), Errno> {
    OPS.close(fp, p)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    //! The glue's own checks: the `<netinet/in.h>` items the kernel's socket calls keep as
    //! constants until N6.4 are `bsd`'s, `ypconnect(2)`'s low port through the internet
    //! protocols' own option sets the C's flag, and a `Native` entry refuses another call's
    //! number.

    use core::mem::offset_of;

    use super::*;
    use crate::netinet::in_::{
        IP_PORTRANGE, IP_PORTRANGE_LOW, IPPORT_RESERVED, IPPROTO_IP, SockaddrIn,
    };

    #[test]
    fn the_kernels_netinet_constants_are_bsds() {
        assert_eq!(i32::from(ks::IPPORT_RESERVED), IPPORT_RESERVED);
        assert_eq!(ks::IPPROTO_IP, IPPROTO_IP);
        assert_eq!(ks::IP_PORTRANGE, IP_PORTRANGE);
        assert_eq!(ks::IP_PORTRANGE_LOW, IP_PORTRANGE_LOW);
        assert_eq!(ks::SOCKADDR_IN_SIZE, size_of::<SockaddrIn>());
        assert_eq!(ks::SIN_PORT_OFFSET, offset_of!(SockaddrIn, sin_port));
        assert_eq!(ks::SIN_ADDR_OFFSET, offset_of!(SockaddrIn, sin_addr));
        #[cfg(feature = "inet6")]
        {
            use crate::netinet6::in6::SockaddrIn6;
            assert_eq!(ks::SOCKADDR_IN6_SIZE, size_of::<SockaddrIn6>());
            assert_eq!(ks::SIN_PORT_OFFSET, offset_of!(SockaddrIn6, sin6_port));
        }
    }

    /// `ypconnect(2)`'s `sotoinpcb(so)->inp_flags |= INP_LOWPORT` is, in the kernel, the call
    /// `ypconnect_lowport` makes: the protocol's `ctloutput(PRCO_SETOPT, IPPROTO_IP,
    /// IP_PORTRANGE)` with a 4-byte `IP_PORTRANGE_LOW`, under `solock`. On a fresh stream and
    /// datagram socket of `bsd`'s internet protocols (tcp_ctloutput, ip_ctloutput) it leaves
    /// the flags the C's assignment leaves: `INP_LOWPORT` added, `INP_HIGHPORT` clear, nothing
    /// else changed.
    #[test]
    fn ypconnects_low_port_option_leaves_the_cs_inp_flags() {
        use kernel::net::mbuf::{M_WAIT, MT_SOOPTS, m_get};
        use kernel::net::proto::PRCO_SETOPT;

        use crate::netinet::in_pcb::{INP_HIGHPORT, INP_LOWPORT, sotoinpcb};
        use crate::sys::socket::{AF_INET, SOCK_DGRAM, SOCK_STREAM};

        let (_g, _t, _p) = crate::netinet::in_pcb::tests::setup();
        crate::netinet::tcp_subr::tcp_init();
        crate::netinet::udp_usrreq::udp_init();
        for ty in [SOCK_STREAM, SOCK_DGRAM] {
            let so = crate::kern::uipc_socket::socreate(i32::from(AF_INET), ty, 0)
                .expect("an internet socket");
            let inp = sotoinpcb(so).expect("attached");
            let before = inp.inp_flags.get();
            assert_eq!(before & (INP_LOWPORT | INP_HIGHPORT), 0, "type {ty}: fresh");

            let mut m = m_get(M_WAIT, MT_SOOPTS).expect("an mbuf");
            m.m_len().set(4);
            m.data_mut()
                .expect("writable")
                .copy_from_slice(&ks::IP_PORTRANGE_LOW.to_ne_bytes());
            let mut w = kernel::net::socket::solock(so);
            let r = so.proto().ctloutput(
                PRCO_SETOPT,
                so,
                &mut w,
                ks::IPPROTO_IP,
                ks::IP_PORTRANGE,
                Some(&mut m),
            );
            drop(w);
            drop(m);
            assert_eq!(r, Ok(()), "type {ty}");
            assert_eq!(inp.inp_flags.get(), before | INP_LOWPORT, "type {ty}");
            assert!(!inp.has_flags(INP_HIGHPORT), "type {ty}");

            let _ = crate::kern::uipc_socket::soclose(so, 0);
        }
        crate::netinet::in_pcb::tests::teardown();
    }

    #[test]
    fn a_native_entry_refuses_another_calls_number() {
        let mut rval = [0; 2];
        let enosys = Err(Errno::ENOSYS.as_i32());
        assert_eq!(
            (SOCKET_ENTRIES.socket)(SYS_bind as Register, 1, 1, 0, &mut rval),
            enosys
        );
        assert_eq!(
            (SOCKET_ENTRIES.recvfrom)(SYS_recvmsg as Register, 3, 0, 0, 0, 0, 0, &mut rval),
            enosys
        );
        assert_eq!(
            (SOCKET_ENTRIES.ypconnect)(SYS_socket as Register, 1, &mut rval),
            enosys
        );
        assert_eq!(rval, [0, 0]);
    }
}
/* </TESTS> */
