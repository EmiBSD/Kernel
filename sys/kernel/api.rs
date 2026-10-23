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
//! The kernel's side of `KernelApi`, the vtable handed to the ABI module when `hal` loads
//! it (decision 22, docs/ZERO_UNSAFE.md 0.1): the module's registration, which the kernel
//! owns, and the table's construction. The type itself is `hal::api`'s, so the module
//! names it without depending on this crate.
//!
//! Who fills it: `bsd` builds the table today with [`table`] (its `main` owns the functions
//! that answer: the LZ system call path, `copyin`/`copyout`, `pipe2` and the virtual memory
//! system's fifteen (`VmEntries`, N3.5), the descriptors' twenty (`FdEntries`, N4.3) and
//! the event system calls' seven (`EventEntries`, N4.4), `sendsyslog(2)` (N4.7a),
//! futex(2) (`FutexEntry`, N4.6c), the process system calls' forty-six (`ProcEntries`,
//! N4.6a), sysctl(2) (`SysctlEntry`, N4.6b) and sigreturn(2) (`SigreturnEntry`, N4.8b),
//! which run the system call prologue still in `bsd` (pledge, the pinned syscalls) before
//! `kernel::pipe`, `kernel::vm`, `kernel::descrip`, `kernel::event`, `kernel::select`,
//! `kernel::log`, `kernel::futex`, the process modules, `kernel::sysctl` and `kernel::sig`
//! (`sys/kern/pipe_glue.rs`, `sys/kern/vm_bsd.rs`, `sys/kern/kern_descrip.rs`,
//! `sys/kern/kern_event.rs`, `sys/kern/sys_generic.rs`, `sys/kern/subr_log.rs`,
//! `sys/kern/sys_futex.rs`, `sys/kern/proc_glue.rs`, `sys/kern/kern_sysctl.rs`,
//! `sys/kern/kern_sig.rs`); an entry moves to a `kernel` function when its subsystem does,
//! and at N9 the kernel builds the table alone.

pub use hal::api::{
    AbiInit, AbiModuleHeader, ApiPrefix, EventEntries, FdEntries, FutexEntry, KERNEL_API_VERSION,
    KernelApi, ProcEntries, SigreturnEntry, SyscallHandler, SysctlEntry, VmEntries,
};
use hal::once::OnceStatic;
pub use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

/// The registered module: the table it was given and its handler.
struct Module {
    api: &'static KernelApi,
    handler: SyscallHandler,
}

/// The module's registration, once `abi_init` made it.
static MODULE: OnceStatic<Module> = OnceStatic::new();

/// The table of this version, with the kernel's registration and the functions the caller
/// owns.
#[allow(
    clippy::too_many_arguments,
    reason = "one argument per member of the table, as KernelApi::new"
)]
pub const fn table(
    syscall_legacy: fn(Register, &SysArgs, &mut Rval) -> SysResult,
    copyin: fn(usize, &mut [u8]) -> Result<(), i32>,
    copyout: fn(&[u8], usize) -> Result<(), i32>,
    pipe2: fn(Register, usize, Register) -> SysResult,
    vm: VmEntries,
    fd: FdEntries,
    event: EventEntries,
    sendsyslog: fn(Register, usize, usize, i32, &mut Rval) -> SysResult,
    futex: FutexEntry,
    proc: ProcEntries,
    sysctl: SysctlEntry,
    sigreturn: SigreturnEntry,
) -> KernelApi {
    KernelApi::new(
        register_syscall_handler,
        syscall_legacy,
        copyin,
        copyout,
        pipe2,
        vm,
        fd,
        event,
        sendsyslog,
        futex,
        proc,
        sysctl,
        sigreturn,
    )
}

/// `KernelApi::register_syscall_handler`: keeps the module's handler with the table it was
/// given and registers in `hal` the trampoline that calls it. Once: a second registration
/// is ignored, so the handler never changes under a running system call.
pub fn register_syscall_handler(
    api: &'static KernelApi,
    handler: SyscallHandler,
    info: HandlerInfo,
) {
    if MODULE.set(Module { api, handler }).is_ok() {
        let _ = hal::syscall::set_handler(module_syscall, info);
    }
}

/// What `hal`'s dispatch point calls once the module registered: the module's handler with
/// its table.
fn module_syscall(code: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
    match MODULE.get() {
        Some(m) => (m.handler)(m.api, code, args, rval),
        None => hal::syscall::legacy(code, args, rval),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn legacy(code: Register, _: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = code;
        Ok(())
    }

    fn copyin(_: usize, _: &mut [u8]) -> Result<(), i32> {
        Ok(())
    }

    fn copyout(_: &[u8], _: usize) -> Result<(), i32> {
        Ok(())
    }

    fn pipe2(nr: Register, _: usize, _: Register) -> SysResult {
        Err(nr as i32)
    }

    fn nr(nr: Register) -> SysResult {
        Err(nr as i32)
    }
    fn nr_int(nr: Register, _: i32) -> SysResult {
        Err(nr as i32)
    }
    fn obreak(nr: Register, _: usize) -> SysResult {
        Err(nr as i32)
    }
    fn range(nr: Register, _: usize, _: usize) -> SysResult {
        Err(nr as i32)
    }
    fn range_int(nr: Register, _: usize, _: usize, _: i32) -> SysResult {
        Err(nr as i32)
    }
    fn mmap(
        nr: Register,
        _: usize,
        _: usize,
        _: i32,
        _: i32,
        _: i32,
        _: i64,
    ) -> Result<usize, i32> {
        Err(nr as i32)
    }
    fn kbind(nr: Register, _: usize, _: usize, _: u64) -> SysResult {
        Err(nr as i32)
    }
    fn pins(nr: Register, _: usize, _: usize, _: usize, _: i32) -> SysResult {
        Err(nr as i32)
    }

    const VM: VmEntries = VmEntries {
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

    fn fd1(nr: Register, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn fd2(nr: Register, _: i32, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn io(nr: Register, _: i32, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn iov(nr: Register, _: i32, _: usize, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn pio(nr: Register, _: i32, _: usize, _: usize, _: i64, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn piov(nr: Register, _: i32, _: usize, _: i32, _: i64, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn count(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn fstat(nr: Register, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn ioctl(nr: Register, _: i32, _: u64, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn fcntl(nr: Register, _: i32, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn dup3(nr: Register, _: i32, _: i32, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn utrace(nr: Register, _: usize, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }

    const FD: FdEntries = FdEntries {
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

    fn select(nr: Register, _: i32, _: [usize; 3], _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn kevent(
        nr: Register,
        _: i32,
        _: (usize, i32),
        _: (usize, i32),
        _: usize,
        _: &mut Rval,
    ) -> SysResult {
        Err(nr as i32)
    }
    fn ppoll(nr: Register, _: usize, _: u32, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn pselect(nr: Register, _: i32, _: [usize; 3], _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn poll(nr: Register, _: usize, _: u32, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }

    const EVENT: EventEntries = EventEntries {
        select,
        kevent,
        ppoll,
        pselect,
        poll,
        kqueue: count,
        kqueue1: fd1,
    };

    fn p_fork(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_vfork(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_tfork(nr: Register, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_exit(nr: Register, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_threxit(nr: Register, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_wait4(nr: Register, _: i32, _: usize, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_waitid(nr: Register, _: i32, _: u32, _: usize, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getrusage(nr: Register, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getrlimit(nr: Register, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setrlimit(nr: Register, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getpriority(nr: Register, _: i32, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setpriority(nr: Register, _: i32, _: u32, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_acct(nr: Register, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getpid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getppid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getuid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_geteuid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getgid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getegid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getgroups(nr: Register, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setgroups(nr: Register, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getpgrp(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getpgid(nr: Register, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getsid(nr: Register, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setpgid(nr: Register, _: i32, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setsid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setuid(nr: Register, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_seteuid(nr: Register, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setgid(nr: Register, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setegid(nr: Register, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setreuid(nr: Register, _: u32, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setregid(nr: Register, _: u32, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setresuid(nr: Register, _: u32, _: u32, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setresgid(nr: Register, _: u32, _: u32, _: u32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getresuid(nr: Register, _: usize, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getresgid(nr: Register, _: usize, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_issetugid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getlogin_r(nr: Register, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setlogin(nr: Register, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_set_tcb(nr: Register, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_get_tcb(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getthrid(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getthrname(nr: Register, _: i32, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setthrname(nr: Register, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_getrtable(nr: Register, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }
    fn p_setrtable(nr: Register, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }

    const PROC: ProcEntries = ProcEntries {
        fork: p_fork,
        vfork: p_vfork,
        tfork: p_tfork,
        exit: p_exit,
        threxit: p_threxit,
        wait4: p_wait4,
        waitid: p_waitid,
        getrusage: p_getrusage,
        getrlimit: p_getrlimit,
        setrlimit: p_setrlimit,
        getpriority: p_getpriority,
        setpriority: p_setpriority,
        acct: p_acct,
        getpid: p_getpid,
        getppid: p_getppid,
        getuid: p_getuid,
        geteuid: p_geteuid,
        getgid: p_getgid,
        getegid: p_getegid,
        getgroups: p_getgroups,
        setgroups: p_setgroups,
        getpgrp: p_getpgrp,
        getpgid: p_getpgid,
        getsid: p_getsid,
        setpgid: p_setpgid,
        setsid: p_setsid,
        setuid: p_setuid,
        seteuid: p_seteuid,
        setgid: p_setgid,
        setegid: p_setegid,
        setreuid: p_setreuid,
        setregid: p_setregid,
        setresuid: p_setresuid,
        setresgid: p_setresgid,
        getresuid: p_getresuid,
        getresgid: p_getresgid,
        issetugid: p_issetugid,
        getlogin_r: p_getlogin_r,
        setlogin: p_setlogin,
        set_tcb: p_set_tcb,
        get_tcb: p_get_tcb,
        getthrid: p_getthrid,
        getthrname: p_getthrname,
        setthrname: p_setthrname,
        getrtable: p_getrtable,
        setrtable: p_setrtable,
    };

    fn sendsyslog(nr: Register, _: usize, _: usize, _: i32, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }

    fn futex(
        nr: Register,
        _: usize,
        _: i32,
        _: i32,
        _: usize,
        _: usize,
        _: &mut Rval,
    ) -> SysResult {
        Err(nr as i32)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the entry's shape: the number, six arguments, the return registers"
    )]
    fn sysctl(
        nr: Register,
        _: usize,
        _: u32,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
        _: &mut Rval,
    ) -> SysResult {
        Err(nr as i32)
    }

    fn sigreturn(nr: Register, _: usize, _: &mut Rval) -> SysResult {
        Err(nr as i32)
    }

    static API: KernelApi = table(
        legacy, copyin, copyout, pipe2, VM, FD, EVENT, sendsyslog, futex, PROC, sysctl, sigreturn,
    );

    #[test]
    fn the_table_is_this_version_with_the_kernels_registration() {
        assert_eq!(API.prefix, ApiPrefix::CURRENT);
        let register: fn(&'static KernelApi, SyscallHandler, HandlerInfo) =
            register_syscall_handler;
        assert!(core::ptr::fn_addr_eq(
            API.register_syscall_handler,
            register
        ));
        let mut rval = [0; 2];
        assert_eq!((API.syscall_legacy)(9, &[0; 6], &mut rval), Ok(()));
        assert_eq!(rval[0], 9);
        assert_eq!((API.pipe2)(263, 0, 0), Err(263));
        assert_eq!((API.vm.mmap)(49, 0, 0, 0, 0, 0, 0), Err(49));
        assert_eq!((API.vm.munlockall)(272), Err(272));
        assert_eq!((API.fd.read)(3, 0, 0, 0, &mut rval), Err(3));
        assert_eq!((API.fd.closefrom)(287, 0, &mut rval), Err(287));
        assert_eq!(
            (API.event.kevent)(72, 0, (0, 0), (0, 0), 0, &mut rval),
            Err(72)
        );
        assert_eq!((API.event.kqueue)(269, &mut rval), Err(269));
        assert_eq!((API.sendsyslog)(112, 0, 0, 0, &mut rval), Err(112));
        assert_eq!((API.futex)(83, 0, 0, 0, 0, 0, &mut rval), Err(83));
        assert_eq!((API.proc.fork)(2, &mut rval), Err(2));
        assert_eq!((API.proc.setrtable)(310, 0, &mut rval), Err(310));
        assert_eq!((API.sysctl)(202, 0, 0, 0, 0, 0, 0, &mut rval), Err(202));
        assert_eq!((API.sigreturn)(103, 0, &mut rval), Err(103));
    }
}
/* </TESTS> */
