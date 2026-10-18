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
//! the flip (S2).
//!
//! Translation only (decision 21; `cargo xtask abi-check`, decision 25): no loop, a
//! `match` only on the number's conversion and on `Some`/`None`, one kernel call per
//! function, no state.

use hal::api::KernelApi;
use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

use hal::sys::syscall::{
    SYS_MAXSYSCALL, SYS_break, SYS_kbind, SYS_madvise, SYS_mimmutable, SYS_minherit, SYS_mlock,
    SYS_mlockall, SYS_mmap, SYS_mprotect, SYS_mquery, SYS_msync, SYS_munlock, SYS_munlockall,
    SYS_munmap, SYS_pinsyscalls, SYS_pipe, SYS_pipe2,
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
pub const NATIVE: u32 = 17;

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

    static API: KernelApi = KernelApi::new(register, legacy, copyin, copyout, pipe2, VM);

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
    fn every_entry_but_pipe_and_vm_is_legacy() {
        for nr in 0..SYS_MAXSYSCALL as Register {
            if nr == SYS_pipe as Register
                || nr == SYS_pipe2 as Register
                || VM_CALLS.iter().any(|&v| v as Register == nr)
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
        assert_eq!(NATIVE, 17);
    }
}
/* </TESTS> */
