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
//! The virtual memory system calls' translations (`kernel::vm`: `break`, `mmap(2)` and its
//! siblings, `kbind(2)`, `pinsyscalls(2)`).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, off_t, ptr, size_t, u_long};
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use sysdefs::proc::syscall::{
    SYS_break, SYS_kbind, SYS_madvise, SYS_mimmutable, SYS_minherit, SYS_mlock, SYS_mlockall,
    SYS_mmap, SYS_mprotect, SYS_mquery, SYS_msync, SYS_munlock, SYS_munlockall, SYS_munmap,
    SYS_pinsyscalls,
};

/// 17 = `break(char *nsize)`.
pub(crate) fn sys_obreak(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::obreak(SYS_break as Register, ptr(args[0]))
}

/// 49 = `mmap(void *addr, size_t len, int prot, int flags, int fd, off_t pos)`: the
/// address in `rval[0]`.
pub(crate) fn sys_mmap(args: &SysArgs, rval: &mut Rval) -> SysResult {
    let r = kernel::api::vm::mmap(
        SYS_mmap as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
        int(args[3]),
        int(args[4]),
        off_t(args[5]),
    );
    match r {
        Ok(addr) => {
            rval[0] = addr.cast_signed();
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// 73 = `munmap(void *addr, size_t len)`.
pub(crate) fn sys_munmap(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::munmap(SYS_munmap as Register, ptr(args[0]), size_t(args[1]))
}

/// 74 = `mprotect(void *addr, size_t len, int prot)`.
pub(crate) fn sys_mprotect(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::mprotect(
        SYS_mprotect as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
    )
}

/// 75 = `madvise(void *addr, size_t len, int behav)`.
pub(crate) fn sys_madvise(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::madvise(
        SYS_madvise as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
    )
}

/// 78 = `mquery(void *addr, size_t len, int prot, int flags, int fd, off_t pos)`: the
/// address in `rval[0]`.
pub(crate) fn sys_mquery(args: &SysArgs, rval: &mut Rval) -> SysResult {
    let r = kernel::api::vm::mquery(
        SYS_mquery as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
        int(args[3]),
        int(args[4]),
        off_t(args[5]),
    );
    match r {
        Ok(addr) => {
            rval[0] = addr.cast_signed();
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// 86 = `kbind(const struct __kbind *param, size_t psize, int64_t proc_cookie)`.
pub(crate) fn sys_kbind(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::kbind(
        SYS_kbind as Register,
        ptr(args[0]),
        size_t(args[1]),
        u_long(args[2]),
    )
}

/// 158 = `pinsyscalls(void *base, size_t len, u_int *pins, int npins)`.
pub(crate) fn sys_pinsyscalls(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::pinsyscalls(
        SYS_pinsyscalls as Register,
        ptr(args[0]),
        size_t(args[1]),
        ptr(args[2]),
        int(args[3]),
    )
}

/// 159 = `mimmutable(void *addr, size_t len)`.
pub(crate) fn sys_mimmutable(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::mimmutable(SYS_mimmutable as Register, ptr(args[0]), size_t(args[1]))
}

/// 203 = `mlock(const void *addr, size_t len)`.
pub(crate) fn sys_mlock(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::mlock(SYS_mlock as Register, ptr(args[0]), size_t(args[1]))
}

/// 204 = `munlock(const void *addr, size_t len)`.
pub(crate) fn sys_munlock(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::munlock(SYS_munlock as Register, ptr(args[0]), size_t(args[1]))
}

/// 250 = `minherit(void *addr, size_t len, int inherit)`.
pub(crate) fn sys_minherit(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::minherit(
        SYS_minherit as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
    )
}

/// 256 = `msync(void *addr, size_t len, int flags)`.
pub(crate) fn sys_msync(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::msync(
        SYS_msync as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
    )
}

/// 271 = `mlockall(int flags)`.
pub(crate) fn sys_mlockall(args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::mlockall(SYS_mlockall as Register, int(args[0]))
}

/// 272 = `munlockall(void)`.
pub(crate) fn sys_munlockall(_args: &SysArgs, _rval: &mut Rval) -> SysResult {
    kernel::api::vm::munlockall(SYS_munlockall as Register)
}
/* </CODE> */
