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
//! The module's system call table: one entry per system call number of
//! `<sys/syscall.h>` (`hal::sys::syscall`, the ABI edge), each `Native` (translated into a
//! `kernel` entry point) or `Legacy` (handed to the LZ path `bsd` registered, through
//! `KernelApi::syscall_legacy`), and the dispatcher `hal` reaches through the handler the
//! module registered.
//!
//! N2.4: every entry was `Legacy`, so every system call behaves exactly as LZ's: the number
//! is not even read here but to pick the entry, and an out-of-range number goes to the
//! fallback too, which answers `ENOSYS` as the traps did. A subsystem that moves into
//! `kernel` flips its entries to `Native` and raises [`NATIVE`]: N2.6, pipe(2) and pipe2(2)
//! (`KernelApi::pipe2`, `sys/kernel/pipe.rs`). N3.5: the virtual memory system's fifteen
//! (`KernelApi::vm`, `sys/kernel/vm/mmap.rs` and `unix.rs`): written in S1, `Native` since
//! the flip (S2). N4.3: the descriptor tables' twenty (`KernelApi::fd`,
//! `sys/kernel/descrip.rs`: `read(2)`, `write(2)`, `close(2)`, `dup(2)`, `fcntl(2)`,
//! `ioctl(2)`, ...), which write the return registers themselves.
//!
//! Translation only (decision 21; `cargo xtask abi-check`, decision 25): no loop, a
//! `match` only on the number's conversion and on `Some`/`None`, one kernel call per
//! function, no state.

use hal::api::KernelApi;
use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

use hal::sys::syscall::{
    SYS_MAXSYSCALL, SYS_break, SYS_close, SYS_closefrom, SYS_dup, SYS_dup2, SYS_dup3, SYS_fcntl,
    SYS_flock, SYS_fpathconf, SYS_fstat, SYS_getdtablecount, SYS_ioctl, SYS_kbind, SYS_madvise,
    SYS_mimmutable, SYS_minherit, SYS_mlock, SYS_mlockall, SYS_mmap, SYS_mprotect, SYS_mquery,
    SYS_msync, SYS_munlock, SYS_munlockall, SYS_munmap, SYS_pinsyscalls, SYS_pipe, SYS_pipe2,
    SYS_pread, SYS_preadv, SYS_pwrite, SYS_pwritev, SYS_read, SYS_readv, SYS_utrace, SYS_write,
    SYS_writev,
};

/// A `Native` entry: the translation of one system call into its `kernel` entry point.
pub type NativeFn = fn(&'static KernelApi, &SysArgs, &mut Rval) -> SysResult;

/// One entry of the table.
#[derive(Clone, Copy)]
pub enum Sysent {
    /// Translated here into the kernel crate.
    Native(NativeFn),
    /// Answered by the LZ path `bsd` registered.
    Legacy,
}

/// How many entries of [`SYSENT`] are `Native` (a test counts them).
pub const NATIVE: u32 = 37;

/// What the module says about itself when it registers ([`crate::abi_init`]).
pub const INFO: HandlerInfo = HandlerInfo {
    syscalls: SYS_MAXSYSCALL as u32,
    native: NATIVE,
};

/// `sysent[]`: the entry of every system call number below `SYS_MAXSYSCALL`.
pub static SYSENT: [Sysent; SYS_MAXSYSCALL] = {
    let mut table = [Sysent::Legacy; SYS_MAXSYSCALL];
    table[SYS_pipe2 as usize] = Sysent::Native(sys_pipe2);
    table[SYS_pipe as usize] = Sysent::Native(sys_pipe);
    table[SYS_break as usize] = Sysent::Native(sys_obreak);
    table[SYS_mmap as usize] = Sysent::Native(sys_mmap);
    table[SYS_munmap as usize] = Sysent::Native(sys_munmap);
    table[SYS_mprotect as usize] = Sysent::Native(sys_mprotect);
    table[SYS_madvise as usize] = Sysent::Native(sys_madvise);
    table[SYS_mquery as usize] = Sysent::Native(sys_mquery);
    table[SYS_kbind as usize] = Sysent::Native(sys_kbind);
    table[SYS_pinsyscalls as usize] = Sysent::Native(sys_pinsyscalls);
    table[SYS_mimmutable as usize] = Sysent::Native(sys_mimmutable);
    table[SYS_mlock as usize] = Sysent::Native(sys_mlock);
    table[SYS_munlock as usize] = Sysent::Native(sys_munlock);
    table[SYS_minherit as usize] = Sysent::Native(sys_minherit);
    table[SYS_msync as usize] = Sysent::Native(sys_msync);
    table[SYS_mlockall as usize] = Sysent::Native(sys_mlockall);
    table[SYS_munlockall as usize] = Sysent::Native(sys_munlockall);
    table[SYS_read as usize] = Sysent::Native(sys_read);
    table[SYS_write as usize] = Sysent::Native(sys_write);
    table[SYS_close as usize] = Sysent::Native(sys_close);
    table[SYS_getdtablecount as usize] = Sysent::Native(sys_getdtablecount);
    table[SYS_dup as usize] = Sysent::Native(sys_dup);
    table[SYS_fstat as usize] = Sysent::Native(sys_fstat);
    table[SYS_ioctl as usize] = Sysent::Native(sys_ioctl);
    table[SYS_dup2 as usize] = Sysent::Native(sys_dup2);
    table[SYS_fcntl as usize] = Sysent::Native(sys_fcntl);
    table[SYS_dup3 as usize] = Sysent::Native(sys_dup3);
    table[SYS_readv as usize] = Sysent::Native(sys_readv);
    table[SYS_writev as usize] = Sysent::Native(sys_writev);
    table[SYS_flock as usize] = Sysent::Native(sys_flock);
    table[SYS_pread as usize] = Sysent::Native(sys_pread);
    table[SYS_pwrite as usize] = Sysent::Native(sys_pwrite);
    table[SYS_preadv as usize] = Sysent::Native(sys_preadv);
    table[SYS_pwritev as usize] = Sysent::Native(sys_pwritev);
    table[SYS_fpathconf as usize] = Sysent::Native(sys_fpathconf);
    table[SYS_utrace as usize] = Sysent::Native(sys_utrace);
    table[SYS_closefrom as usize] = Sysent::Native(sys_closefrom);
    table
};

/// The module's system call handler (`hal::api::SyscallHandler`).
pub fn syscall(
    api: &'static KernelApi,
    nr: Register,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    dispatch(&SYSENT, api, nr, args, rval)
}

/// The dispatcher over a table: a `Native` entry is called, a `Legacy` one or a number with
/// no entry goes to the fallback with the number unchanged.
pub fn dispatch(
    table: &[Sysent],
    api: &'static KernelApi,
    nr: Register,
    args: &SysArgs,
    rval: &mut Rval,
) -> SysResult {
    match entry(table, nr) {
        Some(Sysent::Native(f)) => f(api, args, rval),
        Some(Sysent::Legacy) | None => (api.syscall_legacy)(nr, args, rval),
    }
}

/// 101 = `pipe2(int *fdp, int flags)`: the kernel's pipe(2) with the flags.
fn sys_pipe2(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.pipe2)(SYS_pipe2 as Register, args[0] as usize, args[1])
}

/// 263 = `pipe(int *fdp)`: the kernel's pipe(2), no flags.
fn sys_pipe(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.pipe2)(SYS_pipe as Register, args[0] as usize, 0)
}

/// 17 = `break(char *nsize)`.
pub fn sys_obreak(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.obreak)(SYS_break as Register, args[0] as usize)
}

/// 49 = `mmap(void *addr, size_t len, int prot, int flags, int fd, off_t pos)`: the
/// address in `rval[0]`.
pub fn sys_mmap(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    let r = (api.vm.mmap)(
        SYS_mmap as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
        args[3] as i32,
        args[4] as i32,
        args[5] as i64,
    );
    match r {
        Ok(addr) => {
            rval[0] = addr as Register;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// 73 = `munmap(void *addr, size_t len)`.
pub fn sys_munmap(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.munmap)(SYS_munmap as Register, args[0] as usize, args[1] as usize)
}

/// 74 = `mprotect(void *addr, size_t len, int prot)`.
pub fn sys_mprotect(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.mprotect)(
        SYS_mprotect as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
    )
}

/// 75 = `madvise(void *addr, size_t len, int behav)`.
pub fn sys_madvise(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.madvise)(
        SYS_madvise as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
    )
}

/// 78 = `mquery(void *addr, size_t len, int prot, int flags, int fd, off_t pos)`: the
/// address in `rval[0]`.
pub fn sys_mquery(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    let r = (api.vm.mquery)(
        SYS_mquery as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
        args[3] as i32,
        args[4] as i32,
        args[5] as i64,
    );
    match r {
        Ok(addr) => {
            rval[0] = addr as Register;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// 86 = `kbind(const struct __kbind *param, size_t psize, int64_t proc_cookie)`.
pub fn sys_kbind(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.kbind)(
        SYS_kbind as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as u64,
    )
}

/// 158 = `pinsyscalls(void *base, size_t len, u_int *pins, int npins)`.
pub fn sys_pinsyscalls(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.pinsyscalls)(
        SYS_pinsyscalls as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as usize,
        args[3] as i32,
    )
}

/// 159 = `mimmutable(void *addr, size_t len)`.
pub fn sys_mimmutable(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.mimmutable)(
        SYS_mimmutable as Register,
        args[0] as usize,
        args[1] as usize,
    )
}

/// 203 = `mlock(const void *addr, size_t len)`.
pub fn sys_mlock(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.mlock)(SYS_mlock as Register, args[0] as usize, args[1] as usize)
}

/// 204 = `munlock(const void *addr, size_t len)`.
pub fn sys_munlock(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.munlock)(SYS_munlock as Register, args[0] as usize, args[1] as usize)
}

/// 250 = `minherit(void *addr, size_t len, int inherit)`.
pub fn sys_minherit(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.minherit)(
        SYS_minherit as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
    )
}

/// 256 = `msync(void *addr, size_t len, int flags)`.
pub fn sys_msync(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.msync)(
        SYS_msync as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
    )
}

/// 271 = `mlockall(int flags)`.
pub fn sys_mlockall(api: &'static KernelApi, args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.mlockall)(SYS_mlockall as Register, args[0] as i32)
}

/// 272 = `munlockall(void)`.
pub fn sys_munlockall(api: &'static KernelApi, _args: &SysArgs, _rval: &mut Rval) -> SysResult {
    (api.vm.munlockall)(SYS_munlockall as Register)
}

/// 3 = `read(int fd, void *buf, size_t nbyte)`.
pub fn sys_read(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.read)(
        SYS_read as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// 4 = `write(int fd, const void *buf, size_t nbyte)`.
pub fn sys_write(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.write)(
        SYS_write as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// 6 = `close(int fd)`.
pub fn sys_close(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.close)(SYS_close as Register, args[0] as i32, rval)
}

/// 18 = `getdtablecount(void)`.
pub fn sys_getdtablecount(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.getdtablecount)(SYS_getdtablecount as Register, rval)
}

/// 41 = `dup(int fd)`.
pub fn sys_dup(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.dup)(SYS_dup as Register, args[0] as i32, rval)
}

/// 53 = `fstat(int fd, struct stat *sb)`.
pub fn sys_fstat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.fstat)(
        SYS_fstat as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 54 = `ioctl(int fd, u_long com, void *data)`.
pub fn sys_ioctl(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.ioctl)(
        SYS_ioctl as Register,
        args[0] as i32,
        args[1] as u64,
        args[2] as usize,
        rval,
    )
}

/// 90 = `dup2(int from, int to)`.
pub fn sys_dup2(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.dup2)(SYS_dup2 as Register, args[0] as i32, args[1] as i32, rval)
}

/// 92 = `fcntl(int fd, int cmd, void *arg)`.
pub fn sys_fcntl(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.fcntl)(
        SYS_fcntl as Register,
        args[0] as i32,
        args[1] as i32,
        args[2] as usize,
        rval,
    )
}

/// 102 = `dup3(int from, int to, int flags)`.
pub fn sys_dup3(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.dup3)(
        SYS_dup3 as Register,
        args[0] as i32,
        args[1] as i32,
        args[2] as i32,
        rval,
    )
}

/// 120 = `readv(int fd, const struct iovec *iovp, int iovcnt)`.
pub fn sys_readv(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.readv)(
        SYS_readv as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        rval,
    )
}

/// 121 = `writev(int fd, const struct iovec *iovp, int iovcnt)`.
pub fn sys_writev(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.writev)(
        SYS_writev as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        rval,
    )
}

/// 131 = `flock(int fd, int how)`.
pub fn sys_flock(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.flock)(SYS_flock as Register, args[0] as i32, args[1] as i32, rval)
}

/// 169 = `pread(int fd, void *buf, size_t nbyte, off_t offset)`.
pub fn sys_pread(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.pread)(
        SYS_pread as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        args[3] as i64,
        rval,
    )
}

/// 170 = `pwrite(int fd, const void *buf, size_t nbyte, off_t offset)`.
pub fn sys_pwrite(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.pwrite)(
        SYS_pwrite as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        args[3] as i64,
        rval,
    )
}

/// 171 = `preadv(int fd, const struct iovec *iovp, int iovcnt, off_t offset)`.
pub fn sys_preadv(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.preadv)(
        SYS_preadv as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as i64,
        rval,
    )
}

/// 172 = `pwritev(int fd, const struct iovec *iovp, int iovcnt, off_t offset)`.
pub fn sys_pwritev(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.pwritev)(
        SYS_pwritev as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as i64,
        rval,
    )
}

/// 192 = `fpathconf(int fd, int name)`.
pub fn sys_fpathconf(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.fpathconf)(
        SYS_fpathconf as Register,
        args[0] as i32,
        args[1] as i32,
        rval,
    )
}

/// 209 = `utrace(const char *label, const void *addr, size_t len)`.
pub fn sys_utrace(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.utrace)(
        SYS_utrace as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// 287 = `closefrom(int fd)`.
pub fn sys_closefrom(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.fd.closefrom)(SYS_closefrom as Register, args[0] as i32, rval)
}

/// The entry of number `nr`, if the table has one (a negative number has none).
fn entry(table: &[Sysent], nr: Register) -> Option<&Sysent> {
    match usize::try_from(nr) {
        Ok(i) => table.get(i),
        Err(_) => None,
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// `ENOSYS`, the fallback's answer here.
    const ENOSYS: i32 = hal::sys::errno::Errno::ENOSYS.as_i32();

    /// The fallback of the test table: says which number reached it.
    fn legacy(nr: Register, _: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(ENOSYS) // as the LZ path answers an out-of-range number
    }

    fn copyin(_: usize, _: &mut [u8]) -> Result<(), i32> {
        Ok(())
    }

    fn copyout(_: &[u8], _: usize) -> Result<(), i32> {
        Ok(())
    }

    fn register(_: &'static KernelApi, _: hal::api::SyscallHandler, _: HandlerInfo) {}

    /// The kernel's pipe entry of the test table: says what reached it.
    fn pipe2(nr: Register, fdp: usize, flags: Register) -> SysResult {
        Err(nr as i32 * 1000 + fdp as i32 + flags as i32)
    }

    // The vm entries of the test table say what reached them: the number, then each
    // argument at its own decimal place.
    fn nr(nr: Register) -> SysResult {
        Err(nr as i32)
    }
    fn nr_int(nr: Register, v: i32) -> SysResult {
        Err(nr as i32 * 10 + v)
    }
    fn obreak(nr: Register, n: usize) -> SysResult {
        Err(nr as i32 * 10 + n as i32)
    }
    fn range(nr: Register, a: usize, l: usize) -> SysResult {
        Err(nr as i32 * 100 + a as i32 * 10 + l as i32)
    }
    fn range_int(nr: Register, a: usize, l: usize, v: i32) -> SysResult {
        Err(nr as i32 * 1000 + a as i32 * 100 + l as i32 * 10 + v)
    }
    fn mmap(
        nr: Register,
        a: usize,
        l: usize,
        p: i32,
        f: i32,
        fd: i32,
        o: i64,
    ) -> Result<usize, i32> {
        if fd < 0 {
            return Err(-fd);
        }
        Ok(nr as usize * 1_000_000
            + a * 100_000
            + l * 10_000
            + p as usize * 1000
            + f as usize * 100
            + fd as usize * 10
            + o as usize)
    }
    fn kbind(nr: Register, p: usize, s: usize, c: u64) -> SysResult {
        Err(nr as i32 * 1000 + p as i32 * 100 + s as i32 * 10 + c as i32)
    }
    fn pins(nr: Register, b: usize, l: usize, p: usize, n: i32) -> SysResult {
        Err(nr as i32 * 10_000 + b as i32 * 1000 + l as i32 * 100 + p as i32 * 10 + n)
    }

    const VM: hal::api::VmEntries = hal::api::VmEntries {
        obreak,
        mmap,
        munmap: range,
        mprotect: range_int,
        madvise: range_int,
        mquery: mmap,
        kbind,
        pinsyscalls: pins,
        mimmutable: range,
        mlock: range,
        munlock: range,
        minherit: range_int,
        msync: range_int,
        mlockall: nr_int,
        munlockall: nr,
    };

    // The fd entries of the test table: each writes its number in rval[0] and answers
    // with the arguments, each at its own decimal place.
    fn fd1(nr: Register, a: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a)
    }
    fn fd2(nr: Register, a: i32, b: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 10 + b)
    }
    fn io(nr: Register, a: i32, b: usize, c: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 100 + b as i32 * 10 + c as i32)
    }
    fn iov(nr: Register, a: i32, b: usize, c: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 100 + b as i32 * 10 + c)
    }
    fn pio(nr: Register, a: i32, b: usize, c: usize, d: i64, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 1000 + b as i32 * 100 + c as i32 * 10 + d as i32)
    }
    fn piov(nr: Register, a: i32, b: usize, c: i32, d: i64, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 1000 + b as i32 * 100 + c * 10 + d as i32)
    }
    fn count(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    fn fstat(nr: Register, a: i32, b: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 10 + b as i32)
    }
    fn ioctl(nr: Register, a: i32, b: u64, c: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 100 + b as i32 * 10 + c as i32)
    }
    fn fcntl(nr: Register, a: i32, b: i32, c: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 100 + b * 10 + c as i32)
    }
    fn dup3(nr: Register, a: i32, b: i32, c: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(a * 100 + b * 10 + c)
    }
    fn utrace(nr: Register, a: usize, b: usize, c: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err((a * 100 + b * 10 + c) as i32)
    }

    const FD: hal::api::FdEntries = hal::api::FdEntries {
        read: io,
        write: io,
        close: fd1,
        getdtablecount: count,
        dup: fd1,
        fstat,
        ioctl,
        dup2: fd2,
        fcntl,
        dup3,
        readv: iov,
        writev: iov,
        flock: fd2,
        pread: pio,
        pwrite: pio,
        preadv: piov,
        pwritev: piov,
        fpathconf: fd2,
        utrace,
        closefrom: fd1,
    };

    static API: KernelApi = KernelApi::new(register, legacy, copyin, copyout, pipe2, VM, FD);

    /// The vm system calls N3.5's flip made `Native` (`KernelApi::vm`).
    const VM_CALLS: [i32; 15] = [
        SYS_break,
        SYS_mmap,
        SYS_munmap,
        SYS_mprotect,
        SYS_madvise,
        SYS_mquery,
        SYS_kbind,
        SYS_pinsyscalls,
        SYS_mimmutable,
        SYS_mlock,
        SYS_munlock,
        SYS_minherit,
        SYS_msync,
        SYS_mlockall,
        SYS_munlockall,
    ];

    /// The descriptor system calls N4.3 made `Native` (`KernelApi::fd`).
    const FD_CALLS: [i32; 20] = [
        SYS_read,
        SYS_write,
        SYS_close,
        SYS_getdtablecount,
        SYS_dup,
        SYS_fstat,
        SYS_ioctl,
        SYS_dup2,
        SYS_fcntl,
        SYS_dup3,
        SYS_readv,
        SYS_writev,
        SYS_flock,
        SYS_pread,
        SYS_pwrite,
        SYS_preadv,
        SYS_pwritev,
        SYS_fpathconf,
        SYS_utrace,
        SYS_closefrom,
    ];

    /// A `Native` entry of the test table.
    fn native(_: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = args[0] + args[1];
        rval[1] = -1;
        Ok(())
    }

    #[test]
    fn every_number_has_an_entry() {
        assert_eq!(SYSENT.len(), SYS_MAXSYSCALL);
        assert_eq!(INFO.syscalls as usize, SYS_MAXSYSCALL);
    }

    #[test]
    fn native_counts_the_native_entries() {
        let n = SYSENT
            .iter()
            .filter(|e| matches!(e, Sysent::Native(_)))
            .count();
        assert_eq!(n, NATIVE as usize);
        assert_eq!(INFO.native, NATIVE);
    }

    /// Every `SYS_*` number of `<sys/syscall.h>` (the generated file, read as text) is an
    /// entry of the table, and no two names share a number.
    #[test]
    fn the_numbers_are_sys_syscall_h() {
        let src = include_str!("../hal/sys/syscall.rs");
        let mut seen = std::collections::BTreeMap::new();
        for line in src.lines() {
            let Some(rest) = line.strip_prefix("pub const SYS_") else {
                continue;
            };
            let Some((name, value)) = rest.split_once(": i32 = ") else {
                continue;
            };
            let n: usize = value.trim_end_matches(';').parse().unwrap();
            assert!(n < SYSENT.len(), "SYS_{name} = {n} has no entry");
            assert!(n > 0, "SYS_{name} = {n}: the traps refuse 0");
            assert_eq!(seen.insert(n, name), None, "SYS_{name} reuses {n}");
        }
        assert!(seen.len() > 200, "parsed {} names", seen.len());
        assert_eq!(seen.get(&4), Some(&"write"));
        assert_eq!(seen.get(&1), Some(&"exit"));
    }

    #[test]
    fn every_entry_but_pipe_vm_and_fd_is_legacy() {
        for nr in 0..SYS_MAXSYSCALL as Register {
            if nr == SYS_pipe as Register
                || nr == SYS_pipe2 as Register
                || VM_CALLS.iter().any(|&v| v as Register == nr)
                || FD_CALLS.iter().any(|&v| v as Register == nr)
            {
                continue;
            }
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr, &[0; 6], &mut rval), Err(ENOSYS));
            assert_eq!(rval[0], nr, "number {nr} reached the fallback unchanged");
        }
    }

    #[test]
    fn pipe_and_pipe2_reach_the_kernel_with_their_number_and_arguments() {
        let mut rval = [-7; 2];
        // pipe2(fdp = 5, flags = 3): the number, the address, the flags.
        assert_eq!(
            syscall(&API, SYS_pipe2 as Register, &[5, 3, 9, 9, 9, 9], &mut rval),
            Err(101 * 1000 + 5 + 3)
        );
        // pipe(fdp = 5): no flags, whatever the second register holds.
        assert_eq!(
            syscall(&API, SYS_pipe as Register, &[5, 3, 9, 9, 9, 9], &mut rval),
            Err(263 * 1000 + 5)
        );
        assert_eq!(rval, [-7, -7], "the return registers are left to the trap");
    }

    /// Each vm translation calls its own entry with its own number and the arguments in
    /// `<sys/syscallargs.h>`'s order; the address of mmap(2) and mquery(2) lands in
    /// `rval[0]`, an error leaves the return registers alone.
    #[test]
    fn the_vm_translations_pass_their_number_and_arguments_in_order() {
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let mut rval = [-7; 2];
        assert_eq!(sys_mmap(&API, &a, &mut rval), Ok(()));
        assert_eq!(rval, [49_123_456, -7]);
        assert_eq!(sys_mquery(&API, &a, &mut rval), Ok(()));
        assert_eq!(rval, [78_123_456, -7]);
        let mut rval = [-7; 2];
        assert_eq!(sys_mmap(&API, &[1, 2, 3, 4, -9, 6], &mut rval), Err(9));
        assert_eq!(rval, [-7, -7]);
        assert_eq!(sys_obreak(&API, &a, &mut rval), Err(171));
        assert_eq!(sys_munmap(&API, &a, &mut rval), Err(7312));
        assert_eq!(sys_mprotect(&API, &a, &mut rval), Err(74123));
        assert_eq!(sys_madvise(&API, &a, &mut rval), Err(75123));
        assert_eq!(sys_kbind(&API, &a, &mut rval), Err(86123));
        assert_eq!(sys_pinsyscalls(&API, &a, &mut rval), Err(1_581_234));
        assert_eq!(sys_mimmutable(&API, &a, &mut rval), Err(15912));
        assert_eq!(sys_mlock(&API, &a, &mut rval), Err(20312));
        assert_eq!(sys_munlock(&API, &a, &mut rval), Err(20412));
        assert_eq!(sys_minherit(&API, &a, &mut rval), Err(250123));
        assert_eq!(sys_msync(&API, &a, &mut rval), Err(256123));
        assert_eq!(sys_mlockall(&API, &a, &mut rval), Err(2711));
        assert_eq!(sys_munlockall(&API, &a, &mut rval), Err(272));
        assert_eq!(
            rval,
            [-7, -7],
            "no translation but mmap's and mquery's writes rval"
        );
    }

    #[test]
    fn a_number_without_an_entry_goes_to_the_fallback() {
        for nr in [-1, Register::MIN, SYS_MAXSYSCALL as Register, 100_000] {
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr, &[0; 6], &mut rval), Err(ENOSYS));
            assert_eq!(rval[0], nr);
        }
    }

    #[test]
    fn a_native_entry_reaches_its_function() {
        let table = [Sysent::Legacy, Sysent::Native(native), Sysent::Legacy];
        let mut rval = [0; 2];
        assert_eq!(
            dispatch(&table, &API, 1, &[2, 3, 0, 0, 0, 0], &mut rval),
            Ok(())
        );
        assert_eq!(rval, [5, -1]);
        assert_eq!(
            dispatch(&table, &API, 2, &[2, 3, 0, 0, 0, 0], &mut rval),
            Err(ENOSYS)
        );
        assert_eq!(rval[0], 2);
    }

    #[test]
    fn the_vm_calls_are_native_and_reach_their_translation() {
        // N3.5's flip: the fifteen vm system calls go to KernelApi::vm, each to its own
        // entry, never the legacy fallback (which says it was reached in rval[0]).
        for nr in VM_CALLS {
            assert!(
                matches!(SYSENT[nr as usize], Sysent::Native(_)),
                "SYS {nr} is Native"
            );
            let mut rval = [0; 2];
            let r = syscall(&API, nr as Register, &[0; 6], &mut rval);
            let legacy = r == Err(ENOSYS) && rval[0] == nr as Register;
            assert!(!legacy, "SYS {nr} reached the legacy fallback");
        }
    }

    /// Each descriptor translation calls its own entry with its own number, the arguments in
    /// `<sys/syscallargs.h>`'s order and the trap's return registers.
    #[test]
    fn the_fd_translations_pass_their_number_arguments_and_rval() {
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let cases: [(NativeFn, i32, i32); 20] = [
            (sys_read, SYS_read, 123),
            (sys_write, SYS_write, 123),
            (sys_close, SYS_close, 1),
            (sys_getdtablecount, SYS_getdtablecount, 0),
            (sys_dup, SYS_dup, 1),
            (sys_fstat, SYS_fstat, 12),
            (sys_ioctl, SYS_ioctl, 123),
            (sys_dup2, SYS_dup2, 12),
            (sys_fcntl, SYS_fcntl, 123),
            (sys_dup3, SYS_dup3, 123),
            (sys_readv, SYS_readv, 123),
            (sys_writev, SYS_writev, 123),
            (sys_flock, SYS_flock, 12),
            (sys_pread, SYS_pread, 1234),
            (sys_pwrite, SYS_pwrite, 1234),
            (sys_preadv, SYS_preadv, 1234),
            (sys_pwritev, SYS_pwritev, 1234),
            (sys_fpathconf, SYS_fpathconf, 12),
            (sys_utrace, SYS_utrace, 123),
            (sys_closefrom, SYS_closefrom, 1),
        ];
        for (f, nr, want) in cases {
            let mut rval = [-7; 2];
            assert_eq!(f(&API, &a, &mut rval), Err(want), "SYS {nr}");
            assert_eq!(rval, [nr as Register, -7], "SYS {nr} wrote rval[0] only");
        }
    }

    #[test]
    fn the_fd_calls_are_native_and_reach_their_translation() {
        for nr in FD_CALLS {
            assert!(
                matches!(SYSENT[nr as usize], Sysent::Native(_)),
                "SYS {nr} is Native"
            );
            let mut rval = [0; 2];
            // The fallback answers ENOSYS; every fd entry of the test table answers 0 here.
            let r = syscall(&API, nr as Register, &[0; 6], &mut rval);
            assert_eq!(r, Err(0), "SYS {nr} reached its fd entry");
            assert_eq!(rval[0], nr as Register);
        }
        assert_eq!(NATIVE, 37);
    }
}
/* </TESTS> */
