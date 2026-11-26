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
//! The virtual memory system calls (`kernel::vm`: `break`, `mmap(2)` and its siblings,
//! `kbind(2)`, `pinsyscalls(2)`). As before decision 52, the address `mmap(2)` and
//! `mquery(2)` return comes back in the `Ok` (the ABI writes it in `rval[0]`), and the row
//! runs over return registers of its own.
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::{Register, SysResult};

use sysdefs::proc::syscall::{
    SYS_break, SYS_kbind, SYS_madvise, SYS_mimmutable, SYS_minherit, SYS_mlock, SYS_mlockall,
    SYS_mmap, SYS_mprotect, SYS_mquery, SYS_msync, SYS_munlock, SYS_munlockall, SYS_munmap,
    SYS_pinsyscalls,
};

use super::native_rval;

/// `break(2)`, `obreak`.
pub fn obreak(nr: Register, nsize: usize) -> SysResult {
    native_rval(nr, SYS_break, [nsize as Register, 0, 0, 0, 0, 0]).map(|_| ())
}

/// `mmap(2)`.
pub fn mmap(
    nr: Register,
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    pos: i64,
) -> Result<usize, i32> {
    let args = [
        addr as Register,
        len as Register,
        prot as Register,
        flags as Register,
        fd as Register,
        pos as Register,
    ];
    native_rval(nr, SYS_mmap, args).map(|a| a as usize)
}

/// `mquery(2)`.
pub fn mquery(
    nr: Register,
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    pos: i64,
) -> Result<usize, i32> {
    let args = [
        addr as Register,
        len as Register,
        prot as Register,
        flags as Register,
        fd as Register,
        pos as Register,
    ];
    native_rval(nr, SYS_mquery, args).map(|a| a as usize)
}

/// A range system call's `Native` entry: `(addr, len)` on row `want`.
fn native_range(nr: Register, want: i32, addr: usize, len: usize) -> SysResult {
    native_rval(nr, want, [addr as Register, len as Register, 0, 0, 0, 0]).map(|_| ())
}

/// A range system call's `Native` entry with an `int`: `(addr, len, v)` on row `want`.
fn native_range_int(nr: Register, want: i32, addr: usize, len: usize, v: i32) -> SysResult {
    let args = [addr as Register, len as Register, v as Register, 0, 0, 0];
    native_rval(nr, want, args).map(|_| ())
}

/// `munmap(2)`.
pub fn munmap(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_munmap, addr, len)
}

/// `mprotect(2)`.
pub fn mprotect(nr: Register, addr: usize, len: usize, prot: i32) -> SysResult {
    native_range_int(nr, SYS_mprotect, addr, len, prot)
}

/// `madvise(2)`.
pub fn madvise(nr: Register, addr: usize, len: usize, behav: i32) -> SysResult {
    native_range_int(nr, SYS_madvise, addr, len, behav)
}

/// `kbind(2)`.
pub fn kbind(nr: Register, param: usize, psize: usize, cookie: u64) -> SysResult {
    let args = [
        param as Register,
        psize as Register,
        cookie as Register,
        0,
        0,
        0,
    ];
    native_rval(nr, SYS_kbind, args).map(|_| ())
}

/// `pinsyscalls(2)`.
pub fn pinsyscalls(nr: Register, base: usize, len: usize, pins: usize, npins: i32) -> SysResult {
    let args = [
        base as Register,
        len as Register,
        pins as Register,
        npins as Register,
        0,
        0,
    ];
    native_rval(nr, SYS_pinsyscalls, args).map(|_| ())
}

/// `mimmutable(2)`.
pub fn mimmutable(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_mimmutable, addr, len)
}

/// `mlock(2)`.
pub fn mlock(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_mlock, addr, len)
}

/// `munlock(2)`.
pub fn munlock(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_munlock, addr, len)
}

/// `minherit(2)`.
pub fn minherit(nr: Register, addr: usize, len: usize, inherit: i32) -> SysResult {
    native_range_int(nr, SYS_minherit, addr, len, inherit)
}

/// `msync(2)`.
pub fn msync(nr: Register, addr: usize, len: usize, flags: i32) -> SysResult {
    native_range_int(nr, SYS_msync, addr, len, flags)
}

/// `mlockall(2)`.
pub fn mlockall(nr: Register, flags: i32) -> SysResult {
    native_rval(nr, SYS_mlockall, [flags as Register, 0, 0, 0, 0, 0]).map(|_| ())
}

/// `munlockall(2)`.
pub fn munlockall(nr: Register) -> SysResult {
    native_rval(nr, SYS_munlockall, [0; 6]).map(|_| ())
}
/* </CODE> */
