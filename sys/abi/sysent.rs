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
//! `ioctl(2)`, ...), which write the return registers themselves. N4.4: the event system
//! calls' seven (`KernelApi::event`, `sys/kernel/event.rs` and `select.rs`: `kqueue(2)`,
//! `kqueue1(2)`, `kevent(2)`, `select(2)`, `pselect(2)`, `poll(2)`, `ppoll(2)`). N4.7a:
//! `sendsyslog(2)` (`KernelApi::sendsyslog`, `sys/kernel/log.rs`). N4.6c: futex(2)
//! (`KernelApi::futex`, `sys/kernel/futex.rs`). N4.6a: the process system calls'
//! forty-six (`KernelApi::proc`, `sys/kernel/fork.rs`, `exit.rs`, `resource.rs`, `prot.rs`,
//! `acct.rs`). N4.6b: sysctl(2) (`KernelApi::sysctl`, `sys/kernel/sysctl.rs`). N4.8: the
//! ten time system calls and reboot(2) (`KernelApi::time`, `sys/kernel/time.rs` and
//! `reboot.rs`). N4.8b: sigreturn(2) (`KernelApi::sigreturn`, `sys/kernel/sig.rs` over
//! `hal::sig`'s frame). N5.4: the VFS system calls' sixty-five (`KernelApi::vfs`,
//! `sys/kernel/fs/syscalls.rs` and `getcwd.rs`: the sixty-four of `vfs_syscalls.c`, `open(2)`,
//! `stat(2)`, `mount(2)`, `unveil(2)`, ..., and `__getcwd(2)`), which write the return
//! registers themselves.
//!
//! Translation only (decision 21; `cargo xtask abi-check`, decision 25): no loop, a
//! `match` only on the number's conversion and on `Some`/`None`, one kernel call per
//! function, no state.

use hal::api::KernelApi;
use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

use hal::sys::syscall::{
    SYS___get_tcb, SYS___set_tcb, SYS___tfork, SYS___threxit, SYS_MAXSYSCALL, SYS_acct, SYS_break,
    SYS_close, SYS_closefrom, SYS_dup, SYS_dup2, SYS_dup3, SYS_exit, SYS_fcntl, SYS_flock,
    SYS_fork, SYS_fpathconf, SYS_fstat, SYS_futex, SYS_getdtablecount, SYS_getegid, SYS_geteuid,
    SYS_getgid, SYS_getgroups, SYS_getlogin_r, SYS_getpgid, SYS_getpgrp, SYS_getpid, SYS_getppid,
    SYS_getpriority, SYS_getresgid, SYS_getresuid, SYS_getrlimit, SYS_getrtable, SYS_getrusage,
    SYS_getsid, SYS_getthrid, SYS_getthrname, SYS_getuid, SYS_ioctl, SYS_issetugid, SYS_kbind,
    SYS_kevent, SYS_kqueue, SYS_kqueue1, SYS_madvise, SYS_mimmutable, SYS_minherit, SYS_mlock,
    SYS_mlockall, SYS_mmap, SYS_mprotect, SYS_mquery, SYS_msync, SYS_munlock, SYS_munlockall,
    SYS_munmap, SYS_pinsyscalls, SYS_pipe, SYS_pipe2, SYS_poll, SYS_ppoll, SYS_pread, SYS_preadv,
    SYS_pselect, SYS_pwrite, SYS_pwritev, SYS_read, SYS_readv, SYS_select, SYS_sendsyslog,
    SYS_setegid, SYS_seteuid, SYS_setgid, SYS_setgroups, SYS_setlogin, SYS_setpgid,
    SYS_setpriority, SYS_setregid, SYS_setresgid, SYS_setresuid, SYS_setreuid, SYS_setrlimit,
    SYS_setrtable, SYS_setsid, SYS_setthrname, SYS_setuid, SYS_sigreturn, SYS_sysctl, SYS_utrace,
    SYS_vfork, SYS_wait4, SYS_waitid, SYS_write, SYS_writev,
};
use hal::sys::syscall::{
    SYS___getcwd, SYS___pledge_open, SYS___realpath, SYS_access, SYS_chdir, SYS_chflags,
    SYS_chflagsat, SYS_chmod, SYS_chown, SYS_chroot, SYS_faccessat, SYS_fchdir, SYS_fchflags,
    SYS_fchmod, SYS_fchmodat, SYS_fchown, SYS_fchownat, SYS_fhopen, SYS_fhstat, SYS_fhstatfs,
    SYS_fstatat, SYS_fstatfs, SYS_fsync, SYS_ftruncate, SYS_futimens, SYS_futimes, SYS_getdents,
    SYS_getfh, SYS_getfsstat, SYS_lchown, SYS_link, SYS_linkat, SYS_lseek, SYS_lstat, SYS_mkdir,
    SYS_mkdirat, SYS_mkfifo, SYS_mkfifoat, SYS_mknod, SYS_mknodat, SYS_mount, SYS_open, SYS_openat,
    SYS_pathconf, SYS_pathconfat, SYS_quotactl, SYS_readlink, SYS_readlinkat, SYS_rename,
    SYS_renameat, SYS_revoke, SYS_rmdir, SYS_stat, SYS_statfs, SYS_symlink, SYS_symlinkat,
    SYS_sync, SYS_truncate, SYS_umask, SYS_unlink, SYS_unlinkat, SYS_unmount, SYS_unveil,
    SYS_utimensat, SYS_utimes,
};
use hal::sys::syscall::{
    SYS_adjfreq, SYS_adjtime, SYS_clock_getres, SYS_clock_gettime, SYS_clock_settime,
    SYS_getitimer, SYS_gettimeofday, SYS_nanosleep, SYS_reboot, SYS_setitimer, SYS_settimeofday,
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
pub const NATIVE: u32 = 170;

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
    table[SYS_select as usize] = Sysent::Native(sys_select);
    table[SYS_kevent as usize] = Sysent::Native(sys_kevent);
    table[SYS_ppoll as usize] = Sysent::Native(sys_ppoll);
    table[SYS_pselect as usize] = Sysent::Native(sys_pselect);
    table[SYS_poll as usize] = Sysent::Native(sys_poll);
    table[SYS_kqueue as usize] = Sysent::Native(sys_kqueue);
    table[SYS_kqueue1 as usize] = Sysent::Native(sys_kqueue1);
    table[SYS_sendsyslog as usize] = Sysent::Native(sys_sendsyslog);
    table[SYS_futex as usize] = Sysent::Native(sys_futex);
    table[SYS_fork as usize] = Sysent::Native(sys_fork);
    table[SYS_vfork as usize] = Sysent::Native(sys_vfork);
    table[SYS___tfork as usize] = Sysent::Native(sys_tfork);
    table[SYS_exit as usize] = Sysent::Native(sys_exit);
    table[SYS___threxit as usize] = Sysent::Native(sys_threxit);
    table[SYS_wait4 as usize] = Sysent::Native(sys_wait4);
    table[SYS_waitid as usize] = Sysent::Native(sys_waitid);
    table[SYS_getrusage as usize] = Sysent::Native(sys_getrusage);
    table[SYS_getrlimit as usize] = Sysent::Native(sys_getrlimit);
    table[SYS_setrlimit as usize] = Sysent::Native(sys_setrlimit);
    table[SYS_getpriority as usize] = Sysent::Native(sys_getpriority);
    table[SYS_setpriority as usize] = Sysent::Native(sys_setpriority);
    table[SYS_acct as usize] = Sysent::Native(sys_acct);
    table[SYS_getpid as usize] = Sysent::Native(sys_getpid);
    table[SYS_getppid as usize] = Sysent::Native(sys_getppid);
    table[SYS_getuid as usize] = Sysent::Native(sys_getuid);
    table[SYS_geteuid as usize] = Sysent::Native(sys_geteuid);
    table[SYS_getgid as usize] = Sysent::Native(sys_getgid);
    table[SYS_getegid as usize] = Sysent::Native(sys_getegid);
    table[SYS_getgroups as usize] = Sysent::Native(sys_getgroups);
    table[SYS_setgroups as usize] = Sysent::Native(sys_setgroups);
    table[SYS_getpgrp as usize] = Sysent::Native(sys_getpgrp);
    table[SYS_getpgid as usize] = Sysent::Native(sys_getpgid);
    table[SYS_getsid as usize] = Sysent::Native(sys_getsid);
    table[SYS_setpgid as usize] = Sysent::Native(sys_setpgid);
    table[SYS_setsid as usize] = Sysent::Native(sys_setsid);
    table[SYS_setuid as usize] = Sysent::Native(sys_setuid);
    table[SYS_seteuid as usize] = Sysent::Native(sys_seteuid);
    table[SYS_setgid as usize] = Sysent::Native(sys_setgid);
    table[SYS_setegid as usize] = Sysent::Native(sys_setegid);
    table[SYS_setreuid as usize] = Sysent::Native(sys_setreuid);
    table[SYS_setregid as usize] = Sysent::Native(sys_setregid);
    table[SYS_setresuid as usize] = Sysent::Native(sys_setresuid);
    table[SYS_setresgid as usize] = Sysent::Native(sys_setresgid);
    table[SYS_getresuid as usize] = Sysent::Native(sys_getresuid);
    table[SYS_getresgid as usize] = Sysent::Native(sys_getresgid);
    table[SYS_issetugid as usize] = Sysent::Native(sys_issetugid);
    table[SYS_getlogin_r as usize] = Sysent::Native(sys_getlogin_r);
    table[SYS_setlogin as usize] = Sysent::Native(sys_setlogin);
    table[SYS___set_tcb as usize] = Sysent::Native(sys_set_tcb);
    table[SYS___get_tcb as usize] = Sysent::Native(sys_get_tcb);
    table[SYS_getthrid as usize] = Sysent::Native(sys_getthrid);
    table[SYS_getthrname as usize] = Sysent::Native(sys_getthrname);
    table[SYS_setthrname as usize] = Sysent::Native(sys_setthrname);
    table[SYS_getrtable as usize] = Sysent::Native(sys_getrtable);
    table[SYS_setrtable as usize] = Sysent::Native(sys_setrtable);
    table[SYS_sysctl as usize] = Sysent::Native(sys_sysctl);
    table[SYS_clock_gettime as usize] = Sysent::Native(sys_clock_gettime);
    table[SYS_clock_settime as usize] = Sysent::Native(sys_clock_settime);
    table[SYS_clock_getres as usize] = Sysent::Native(sys_clock_getres);
    table[SYS_nanosleep as usize] = Sysent::Native(sys_nanosleep);
    table[SYS_gettimeofday as usize] = Sysent::Native(sys_gettimeofday);
    table[SYS_settimeofday as usize] = Sysent::Native(sys_settimeofday);
    table[SYS_adjfreq as usize] = Sysent::Native(sys_adjfreq);
    table[SYS_adjtime as usize] = Sysent::Native(sys_adjtime);
    table[SYS_getitimer as usize] = Sysent::Native(sys_getitimer);
    table[SYS_setitimer as usize] = Sysent::Native(sys_setitimer);
    table[SYS_reboot as usize] = Sysent::Native(sys_reboot);
    table[SYS_sigreturn as usize] = Sysent::Native(sys_sigreturn);
    table[SYS_open as usize] = Sysent::Native(sys_open);
    table[SYS_link as usize] = Sysent::Native(sys_link);
    table[SYS_unlink as usize] = Sysent::Native(sys_unlink);
    table[SYS_chdir as usize] = Sysent::Native(sys_chdir);
    table[SYS_fchdir as usize] = Sysent::Native(sys_fchdir);
    table[SYS_mknod as usize] = Sysent::Native(sys_mknod);
    table[SYS_chmod as usize] = Sysent::Native(sys_chmod);
    table[SYS_chown as usize] = Sysent::Native(sys_chown);
    table[SYS_mount as usize] = Sysent::Native(sys_mount);
    table[SYS_unmount as usize] = Sysent::Native(sys_unmount);
    table[SYS_access as usize] = Sysent::Native(sys_access);
    table[SYS_chflags as usize] = Sysent::Native(sys_chflags);
    table[SYS_fchflags as usize] = Sysent::Native(sys_fchflags);
    table[SYS_sync as usize] = Sysent::Native(sys_sync);
    table[SYS_stat as usize] = Sysent::Native(sys_stat);
    table[SYS_lstat as usize] = Sysent::Native(sys_lstat);
    table[SYS_fstatat as usize] = Sysent::Native(sys_fstatat);
    table[SYS_revoke as usize] = Sysent::Native(sys_revoke);
    table[SYS_symlink as usize] = Sysent::Native(sys_symlink);
    table[SYS_readlink as usize] = Sysent::Native(sys_readlink);
    table[SYS_umask as usize] = Sysent::Native(sys_umask);
    table[SYS_chroot as usize] = Sysent::Native(sys_chroot);
    table[SYS_getfsstat as usize] = Sysent::Native(sys_getfsstat);
    table[SYS_statfs as usize] = Sysent::Native(sys_statfs);
    table[SYS_fstatfs as usize] = Sysent::Native(sys_fstatfs);
    table[SYS_fhstatfs as usize] = Sysent::Native(sys_fhstatfs);
    table[SYS_utimes as usize] = Sysent::Native(sys_utimes);
    table[SYS_futimes as usize] = Sysent::Native(sys_futimes);
    table[SYS_utimensat as usize] = Sysent::Native(sys_utimensat);
    table[SYS_futimens as usize] = Sysent::Native(sys_futimens);
    table[SYS_fsync as usize] = Sysent::Native(sys_fsync);
    table[SYS_getdents as usize] = Sysent::Native(sys_getdents);
    table[SYS_chflagsat as usize] = Sysent::Native(sys_chflagsat);
    table[SYS_unveil as usize] = Sysent::Native(sys_unveil);
    table[SYS___realpath as usize] = Sysent::Native(sys_realpath);
    table[SYS_fchown as usize] = Sysent::Native(sys_fchown);
    table[SYS_fchmod as usize] = Sysent::Native(sys_fchmod);
    table[SYS___pledge_open as usize] = Sysent::Native(sys_pledge_open);
    table[SYS_rename as usize] = Sysent::Native(sys_rename);
    table[SYS_mkfifo as usize] = Sysent::Native(sys_mkfifo);
    table[SYS_mkdir as usize] = Sysent::Native(sys_mkdir);
    table[SYS_rmdir as usize] = Sysent::Native(sys_rmdir);
    table[SYS_quotactl as usize] = Sysent::Native(sys_quotactl);
    table[SYS_getfh as usize] = Sysent::Native(sys_getfh);
    table[SYS_lseek as usize] = Sysent::Native(sys_lseek);
    table[SYS_truncate as usize] = Sysent::Native(sys_truncate);
    table[SYS_ftruncate as usize] = Sysent::Native(sys_ftruncate);
    table[SYS_pathconfat as usize] = Sysent::Native(sys_pathconfat);
    table[SYS_pathconf as usize] = Sysent::Native(sys_pathconf);
    table[SYS_lchown as usize] = Sysent::Native(sys_lchown);
    table[SYS_fhopen as usize] = Sysent::Native(sys_fhopen);
    table[SYS_fhstat as usize] = Sysent::Native(sys_fhstat);
    table[SYS___getcwd as usize] = Sysent::Native(sys_getcwd);
    table[SYS_faccessat as usize] = Sysent::Native(sys_faccessat);
    table[SYS_fchmodat as usize] = Sysent::Native(sys_fchmodat);
    table[SYS_fchownat as usize] = Sysent::Native(sys_fchownat);
    table[SYS_linkat as usize] = Sysent::Native(sys_linkat);
    table[SYS_mkdirat as usize] = Sysent::Native(sys_mkdirat);
    table[SYS_mkfifoat as usize] = Sysent::Native(sys_mkfifoat);
    table[SYS_mknodat as usize] = Sysent::Native(sys_mknodat);
    table[SYS_openat as usize] = Sysent::Native(sys_openat);
    table[SYS_readlinkat as usize] = Sysent::Native(sys_readlinkat);
    table[SYS_renameat as usize] = Sysent::Native(sys_renameat);
    table[SYS_symlinkat as usize] = Sysent::Native(sys_symlinkat);
    table[SYS_unlinkat as usize] = Sysent::Native(sys_unlinkat);
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

/// 71 = `select(int nd, fd_set *in, fd_set *ou, fd_set *ex, struct timeval *tv)`.
pub fn sys_select(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.event.select)(
        SYS_select as Register,
        args[0] as i32,
        [args[1] as usize, args[2] as usize, args[3] as usize],
        args[4] as usize,
        rval,
    )
}

/// 72 = `kevent(int fd, const struct kevent *changelist, int nchanges, struct kevent
/// *eventlist, int nevents, const struct timespec *timeout)`.
pub fn sys_kevent(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.event.kevent)(
        SYS_kevent as Register,
        args[0] as i32,
        (args[1] as usize, args[2] as i32),
        (args[3] as usize, args[4] as i32),
        args[5] as usize,
        rval,
    )
}

/// 109 = `ppoll(struct pollfd *fds, u_int nfds, const struct timespec *ts, const sigset_t
/// *mask)`.
pub fn sys_ppoll(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.event.ppoll)(
        SYS_ppoll as Register,
        args[0] as usize,
        args[1] as u32,
        args[2] as usize,
        args[3] as usize,
        rval,
    )
}

/// 110 = `pselect(int nd, fd_set *in, fd_set *ou, fd_set *ex, const struct timespec *ts,
/// const sigset_t *mask)`.
pub fn sys_pselect(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.event.pselect)(
        SYS_pselect as Register,
        args[0] as i32,
        [args[1] as usize, args[2] as usize, args[3] as usize],
        args[4] as usize,
        args[5] as usize,
        rval,
    )
}

/// 252 = `poll(struct pollfd *fds, u_int nfds, int timeout)`.
pub fn sys_poll(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.event.poll)(
        SYS_poll as Register,
        args[0] as usize,
        args[1] as u32,
        args[2] as i32,
        rval,
    )
}

/// 269 = `kqueue(void)`.
pub fn sys_kqueue(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.event.kqueue)(SYS_kqueue as Register, rval)
}

/// 270 = `kqueue1(int flags)`.
pub fn sys_kqueue1(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.event.kqueue1)(SYS_kqueue1 as Register, args[0] as i32, rval)
}

/// 112 = `sendsyslog(const char *buf, size_t nbyte, int flags)`.
pub fn sys_sendsyslog(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.sendsyslog)(
        SYS_sendsyslog as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
        rval,
    )
}

/// 202 = `sysctl(const int *name, u_int namelen, void *old, size_t *oldlenp, void *new,
/// size_t newlen)`.
pub fn sys_sysctl(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.sysctl)(
        SYS_sysctl as Register,
        args[0] as usize,
        args[1] as u32,
        args[2] as usize,
        args[3] as usize,
        args[4] as usize,
        args[5] as usize,
        rval,
    )
}

/// 103 = `sigreturn(struct sigcontext *sigcntxp)`: `EJUSTRETURN` once the context is restored.
pub fn sys_sigreturn(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.sigreturn)(SYS_sigreturn as Register, args[0] as usize, rval)
}

/// 83 = `futex(uint32_t *f, int op, int val, const struct timespec *timeout, uint32_t *g)`:
/// the number woken in `rval[0]`.
pub fn sys_futex(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.futex)(
        SYS_futex as Register,
        args[0] as usize,
        args[1] as i32,
        args[2] as i32,
        args[3] as usize,
        args[4] as usize,
        rval,
    )
}

/// The entry of number `nr`, if the table has one (a negative number has none).
fn entry(table: &[Sysent], nr: Register) -> Option<&Sysent> {
    match usize::try_from(nr) {
        Ok(i) => table.get(i),
        Err(_) => None,
    }
}
/// `fork(void)`.
fn sys_fork(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.fork)(SYS_fork as Register, rval)
}

/// `vfork(void)`.
fn sys_vfork(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.vfork)(SYS_vfork as Register, rval)
}

/// `__tfork(const struct __tfork *param, size_t psize)`.
fn sys_tfork(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.tfork)(
        SYS___tfork as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// `exit(int rval)`.
fn sys_exit(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.exit)(SYS_exit as Register, args[0] as i32, rval)
}

/// `__threxit(pid_t *notdead)`.
fn sys_threxit(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.threxit)(SYS___threxit as Register, args[0] as usize, rval)
}

/// `wait4(pid_t pid, int *status, int options, struct rusage *rusage)`.
fn sys_wait4(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.wait4)(
        SYS_wait4 as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as usize,
        rval,
    )
}

/// `waitid(int idtype, id_t id, siginfo_t *info, int options)`.
fn sys_waitid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.waitid)(
        SYS_waitid as Register,
        args[0] as i32,
        args[1] as u32,
        args[2] as usize,
        args[3] as i32,
        rval,
    )
}

/// `getrusage(int who, struct rusage *rusage)`.
fn sys_getrusage(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getrusage)(
        SYS_getrusage as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// `getrlimit(int which, struct rlimit *rlp)`.
fn sys_getrlimit(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getrlimit)(
        SYS_getrlimit as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// `setrlimit(int which, const struct rlimit *rlp)`.
fn sys_setrlimit(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setrlimit)(
        SYS_setrlimit as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// `getpriority(int which, id_t who)`.
fn sys_getpriority(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getpriority)(
        SYS_getpriority as Register,
        args[0] as i32,
        args[1] as u32,
        rval,
    )
}

/// `setpriority(int which, id_t who, int prio)`.
fn sys_setpriority(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setpriority)(
        SYS_setpriority as Register,
        args[0] as i32,
        args[1] as u32,
        args[2] as i32,
        rval,
    )
}

/// `acct(const char *path)`.
fn sys_acct(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.acct)(SYS_acct as Register, args[0] as usize, rval)
}

/// `getpid(void)`.
fn sys_getpid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getpid)(SYS_getpid as Register, rval)
}

/// `getppid(void)`.
fn sys_getppid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getppid)(SYS_getppid as Register, rval)
}

/// `getuid(void)`.
fn sys_getuid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getuid)(SYS_getuid as Register, rval)
}

/// `geteuid(void)`.
fn sys_geteuid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.geteuid)(SYS_geteuid as Register, rval)
}

/// `getgid(void)`.
fn sys_getgid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getgid)(SYS_getgid as Register, rval)
}

/// `getegid(void)`.
fn sys_getegid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getegid)(SYS_getegid as Register, rval)
}

/// `getgroups(int gidsetsize, gid_t *gidset)`.
fn sys_getgroups(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getgroups)(
        SYS_getgroups as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// `setgroups(int gidsetsize, const gid_t *gidset)`.
fn sys_setgroups(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setgroups)(
        SYS_setgroups as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// `getpgrp(void)`.
fn sys_getpgrp(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getpgrp)(SYS_getpgrp as Register, rval)
}

/// `getpgid(pid_t pid)`.
fn sys_getpgid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getpgid)(SYS_getpgid as Register, args[0] as i32, rval)
}

/// `getsid(pid_t pid)`.
fn sys_getsid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getsid)(SYS_getsid as Register, args[0] as i32, rval)
}

/// `setpgid(pid_t pid, pid_t pgid)`.
fn sys_setpgid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setpgid)(
        SYS_setpgid as Register,
        args[0] as i32,
        args[1] as i32,
        rval,
    )
}

/// `setsid(void)`.
fn sys_setsid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setsid)(SYS_setsid as Register, rval)
}

/// `setuid(uid_t uid)`.
fn sys_setuid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setuid)(SYS_setuid as Register, args[0] as u32, rval)
}

/// `seteuid(uid_t euid)`.
fn sys_seteuid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.seteuid)(SYS_seteuid as Register, args[0] as u32, rval)
}

/// `setgid(gid_t gid)`.
fn sys_setgid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setgid)(SYS_setgid as Register, args[0] as u32, rval)
}

/// `setegid(gid_t egid)`.
fn sys_setegid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setegid)(SYS_setegid as Register, args[0] as u32, rval)
}

/// `setreuid(uid_t ruid, uid_t euid)`.
fn sys_setreuid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setreuid)(
        SYS_setreuid as Register,
        args[0] as u32,
        args[1] as u32,
        rval,
    )
}

/// `setregid(gid_t rgid, gid_t egid)`.
fn sys_setregid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setregid)(
        SYS_setregid as Register,
        args[0] as u32,
        args[1] as u32,
        rval,
    )
}

/// `setresuid(uid_t ruid, uid_t euid, uid_t suid)`.
fn sys_setresuid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setresuid)(
        SYS_setresuid as Register,
        args[0] as u32,
        args[1] as u32,
        args[2] as u32,
        rval,
    )
}

/// `setresgid(gid_t rgid, gid_t egid, gid_t sgid)`.
fn sys_setresgid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setresgid)(
        SYS_setresgid as Register,
        args[0] as u32,
        args[1] as u32,
        args[2] as u32,
        rval,
    )
}

/// `getresuid(uid_t *ruid, uid_t *euid, uid_t *suid)`.
fn sys_getresuid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getresuid)(
        SYS_getresuid as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// `getresgid(gid_t *rgid, gid_t *egid, gid_t *sgid)`.
fn sys_getresgid(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getresgid)(
        SYS_getresgid as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// `issetugid(void)`.
fn sys_issetugid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.issetugid)(SYS_issetugid as Register, rval)
}

/// `getlogin_r(char *namebuf, size_t namelen)`.
fn sys_getlogin_r(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getlogin_r)(
        SYS_getlogin_r as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// `setlogin(const char *namebuf)`.
fn sys_setlogin(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setlogin)(SYS_setlogin as Register, args[0] as usize, rval)
}

/// `__set_tcb(void *tcb)`.
fn sys_set_tcb(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.set_tcb)(SYS___set_tcb as Register, args[0] as usize, rval)
}

/// `__get_tcb(void)`.
fn sys_get_tcb(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.get_tcb)(SYS___get_tcb as Register, rval)
}

/// `getthrid(void)`.
fn sys_getthrid(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getthrid)(SYS_getthrid as Register, rval)
}

/// `getthrname(pid_t tid, char *name, size_t len)`.
fn sys_getthrname(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getthrname)(
        SYS_getthrname as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// `setthrname(pid_t tid, const char *name)`.
fn sys_setthrname(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setthrname)(
        SYS_setthrname as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// `getrtable(void)`.
fn sys_getrtable(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.getrtable)(SYS_getrtable as Register, rval)
}

/// `setrtable(int rtableid)`.
fn sys_setrtable(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.proc.setrtable)(SYS_setrtable as Register, args[0] as i32, rval)
}

/// 87 = `clock_gettime(clockid_t clock_id, struct timespec *tp)`.
fn sys_clock_gettime(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.clock_gettime)(
        SYS_clock_gettime as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 88 = `clock_settime(clockid_t clock_id, const struct timespec *tp)`.
fn sys_clock_settime(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.clock_settime)(
        SYS_clock_settime as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 89 = `clock_getres(clockid_t clock_id, struct timespec *tp)`.
fn sys_clock_getres(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.clock_getres)(
        SYS_clock_getres as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 91 = `nanosleep(const struct timespec *rqtp, struct timespec *rmtp)`.
fn sys_nanosleep(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.nanosleep)(
        SYS_nanosleep as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 67 = `gettimeofday(struct timeval *tp, struct timezone *tzp)`.
fn sys_gettimeofday(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.gettimeofday)(
        SYS_gettimeofday as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 68 = `settimeofday(const struct timeval *tv, const struct timezone *tzp)`.
fn sys_settimeofday(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.settimeofday)(
        SYS_settimeofday as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 305 = `adjfreq(const int64_t *freq, int64_t *oldfreq)`.
fn sys_adjfreq(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.adjfreq)(
        SYS_adjfreq as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 140 = `adjtime(const struct timeval *delta, struct timeval *olddelta)`.
fn sys_adjtime(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.adjtime)(
        SYS_adjtime as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 70 = `getitimer(int which, struct itimerval *itv)`.
fn sys_getitimer(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.getitimer)(
        SYS_getitimer as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 69 = `setitimer(int which, const struct itimerval *itv, struct itimerval *oitv)`.
fn sys_setitimer(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.setitimer)(
        SYS_setitimer as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// 55 = `reboot(int opt)`.
fn sys_reboot(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.time.reboot)(SYS_reboot as Register, args[0] as i32, rval)
}

/// 5 = `int open(const char *path, int flags, mode_t mode)`.
fn sys_open(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.open)(
        SYS_open as Register,
        args[0] as usize,
        args[1] as i32,
        args[2] as u32,
        rval,
    )
}

/// 9 = `int link(const char *path, const char *link)`.
fn sys_link(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.link)(
        SYS_link as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 10 = `int unlink(const char *path)`.
fn sys_unlink(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.unlink)(SYS_unlink as Register, args[0] as usize, rval)
}

/// 12 = `int chdir(const char *path)`.
fn sys_chdir(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.chdir)(SYS_chdir as Register, args[0] as usize, rval)
}

/// 13 = `int fchdir(int fd)`.
fn sys_fchdir(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fchdir)(SYS_fchdir as Register, args[0] as i32, rval)
}

/// 14 = `int mknod(const char *path, mode_t mode, dev_t dev)`.
fn sys_mknod(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.mknod)(
        SYS_mknod as Register,
        args[0] as usize,
        args[1] as u32,
        args[2] as i32,
        rval,
    )
}

/// 15 = `int chmod(const char *path, mode_t mode)`.
fn sys_chmod(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.chmod)(
        SYS_chmod as Register,
        args[0] as usize,
        args[1] as u32,
        rval,
    )
}

/// 16 = `int chown(const char *path, uid_t uid, gid_t gid)`.
fn sys_chown(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.chown)(
        SYS_chown as Register,
        args[0] as usize,
        args[1] as u32,
        args[2] as u32,
        rval,
    )
}

/// 21 = `int mount(const char *type, const char *path, int flags, void *data)`.
fn sys_mount(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.mount)(
        SYS_mount as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
        args[3] as usize,
        rval,
    )
}

/// 22 = `int unmount(const char *path, int flags)`.
fn sys_unmount(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.unmount)(
        SYS_unmount as Register,
        args[0] as usize,
        args[1] as i32,
        rval,
    )
}

/// 33 = `int access(const char *path, int amode)`.
fn sys_access(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.access)(
        SYS_access as Register,
        args[0] as usize,
        args[1] as i32,
        rval,
    )
}

/// 34 = `int chflags(const char *path, u_int flags)`.
fn sys_chflags(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.chflags)(
        SYS_chflags as Register,
        args[0] as usize,
        args[1] as u32,
        rval,
    )
}

/// 35 = `int fchflags(int fd, u_int flags)`.
fn sys_fchflags(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fchflags)(
        SYS_fchflags as Register,
        args[0] as i32,
        args[1] as u32,
        rval,
    )
}

/// 36 = `void sync(void)`.
fn sys_sync(api: &'static KernelApi, _args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.sync)(SYS_sync as Register, rval)
}

/// 38 = `int stat(const char *path, struct stat *ub)`.
fn sys_stat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.stat)(
        SYS_stat as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 40 = `int lstat(const char *path, struct stat *ub)`.
fn sys_lstat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.lstat)(
        SYS_lstat as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 42 = `int fstatat(int fd, const char *path, struct stat *buf, int flag)`.
fn sys_fstatat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fstatat)(
        SYS_fstatat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        args[3] as i32,
        rval,
    )
}

/// 56 = `int revoke(const char *path)`.
fn sys_revoke(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.revoke)(SYS_revoke as Register, args[0] as usize, rval)
}

/// 57 = `int symlink(const char *path, const char *link)`.
fn sys_symlink(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.symlink)(
        SYS_symlink as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 58 = `ssize_t readlink(const char *path, char *buf, size_t count)`.
fn sys_readlink(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.readlink)(
        SYS_readlink as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// 60 = `mode_t umask(mode_t newmask)`.
fn sys_umask(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.umask)(SYS_umask as Register, args[0] as u32, rval)
}

/// 61 = `int chroot(const char *path)`.
fn sys_chroot(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.chroot)(SYS_chroot as Register, args[0] as usize, rval)
}

/// 62 = `int getfsstat(struct statfs *buf, size_t bufsize, int flags)`.
fn sys_getfsstat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.getfsstat)(
        SYS_getfsstat as Register,
        args[0] as usize,
        args[1] as usize,
        args[2] as i32,
        rval,
    )
}

/// 63 = `int statfs(const char *path, struct statfs *buf)`.
fn sys_statfs(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.statfs)(
        SYS_statfs as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 64 = `int fstatfs(int fd, struct statfs *buf)`.
fn sys_fstatfs(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fstatfs)(
        SYS_fstatfs as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 65 = `int fhstatfs(const fhandle_t *fhp, struct statfs *buf)`.
fn sys_fhstatfs(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fhstatfs)(
        SYS_fhstatfs as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 76 = `int utimes(const char *path, const struct timeval *tptr)`.
fn sys_utimes(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.utimes)(
        SYS_utimes as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 77 = `int futimes(int fd, const struct timeval *tptr)`.
fn sys_futimes(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.futimes)(
        SYS_futimes as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 84 = `int utimensat(int fd, const char *path, const struct timespec *times, int flag)`.
fn sys_utimensat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.utimensat)(
        SYS_utimensat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        args[3] as i32,
        rval,
    )
}

/// 85 = `int futimens(int fd, const struct timespec *times)`.
fn sys_futimens(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.futimens)(
        SYS_futimens as Register,
        args[0] as i32,
        args[1] as usize,
        rval,
    )
}

/// 95 = `int fsync(int fd)`.
fn sys_fsync(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fsync)(SYS_fsync as Register, args[0] as i32, rval)
}

/// 99 = `int getdents(int fd, void *buf, size_t buflen)`.
fn sys_getdents(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.getdents)(
        SYS_getdents as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        rval,
    )
}

/// 107 = `int chflagsat(int fd, const char *path, u_int flags, int atflags)`.
fn sys_chflagsat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.chflagsat)(
        SYS_chflagsat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as u32,
        args[3] as i32,
        rval,
    )
}

/// 114 = `int unveil(const char *path, const char *permissions)`.
fn sys_unveil(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.unveil)(
        SYS_unveil as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 115 = `int __realpath(const char *pathname, char *resolved)`.
fn sys_realpath(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.realpath)(
        SYS___realpath as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 123 = `int fchown(int fd, uid_t uid, gid_t gid)`.
fn sys_fchown(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fchown)(
        SYS_fchown as Register,
        args[0] as i32,
        args[1] as u32,
        args[2] as u32,
        rval,
    )
}

/// 124 = `int fchmod(int fd, mode_t mode)`.
fn sys_fchmod(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fchmod)(SYS_fchmod as Register, args[0] as i32, args[1] as u32, rval)
}

/// 125 = `int __pledge_open(const char *path, int flags, mode_t mode)`.
fn sys_pledge_open(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.pledge_open)(
        SYS___pledge_open as Register,
        args[0] as usize,
        args[1] as i32,
        args[2] as u32,
        rval,
    )
}

/// 128 = `int rename(const char *from, const char *to)`.
fn sys_rename(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.rename)(
        SYS_rename as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 132 = `int mkfifo(const char *path, mode_t mode)`.
fn sys_mkfifo(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.mkfifo)(
        SYS_mkfifo as Register,
        args[0] as usize,
        args[1] as u32,
        rval,
    )
}

/// 136 = `int mkdir(const char *path, mode_t mode)`.
fn sys_mkdir(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.mkdir)(
        SYS_mkdir as Register,
        args[0] as usize,
        args[1] as u32,
        rval,
    )
}

/// 137 = `int rmdir(const char *path)`.
fn sys_rmdir(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.rmdir)(SYS_rmdir as Register, args[0] as usize, rval)
}

/// 148 = `int quotactl(const char *path, int cmd, int uid, char *arg)`.
fn sys_quotactl(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.quotactl)(
        SYS_quotactl as Register,
        args[0] as usize,
        args[1] as i32,
        args[2] as i32,
        args[3] as usize,
        rval,
    )
}

/// 161 = `int getfh(const char *fname, fhandle_t *fhp)`.
fn sys_getfh(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.getfh)(
        SYS_getfh as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 166 = `off_t lseek(int fd, off_t offset, int whence)`.
fn sys_lseek(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.lseek)(
        SYS_lseek as Register,
        args[0] as i32,
        args[1] as i64,
        args[2] as i32,
        rval,
    )
}

/// 167 = `int truncate(const char *path, off_t length)`.
fn sys_truncate(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.truncate)(
        SYS_truncate as Register,
        args[0] as usize,
        args[1] as i64,
        rval,
    )
}

/// 168 = `int ftruncate(int fd, off_t length)`.
fn sys_ftruncate(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.ftruncate)(
        SYS_ftruncate as Register,
        args[0] as i32,
        args[1] as i64,
        rval,
    )
}

/// 190 = `long pathconfat(int fd, const char *path, int name, int flag)`.
fn sys_pathconfat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.pathconfat)(
        SYS_pathconfat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as i32,
        rval,
    )
}

/// 191 = `long pathconf(const char *path, int name)`.
fn sys_pathconf(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.pathconf)(
        SYS_pathconf as Register,
        args[0] as usize,
        args[1] as i32,
        rval,
    )
}

/// 254 = `int lchown(const char *path, uid_t uid, gid_t gid)`.
fn sys_lchown(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.lchown)(
        SYS_lchown as Register,
        args[0] as usize,
        args[1] as u32,
        args[2] as u32,
        rval,
    )
}

/// 264 = `int fhopen(const fhandle_t *fhp, int flags)`.
fn sys_fhopen(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fhopen)(
        SYS_fhopen as Register,
        args[0] as usize,
        args[1] as i32,
        rval,
    )
}

/// 294 = `int fhstat(const fhandle_t *fhp, struct stat *sb)`.
fn sys_fhstat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fhstat)(
        SYS_fhstat as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 304 = `int __getcwd(char *buf, size_t len)`.
fn sys_getcwd(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.getcwd)(
        SYS___getcwd as Register,
        args[0] as usize,
        args[1] as usize,
        rval,
    )
}

/// 313 = `int faccessat(int fd, const char *path, int amode, int flag)`.
fn sys_faccessat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.faccessat)(
        SYS_faccessat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as i32,
        rval,
    )
}

/// 314 = `int fchmodat(int fd, const char *path, mode_t mode, int flag)`.
fn sys_fchmodat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fchmodat)(
        SYS_fchmodat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as u32,
        args[3] as i32,
        rval,
    )
}

/// 315 = `int fchownat(int fd, const char *path, uid_t uid, gid_t gid, int flag)`.
fn sys_fchownat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.fchownat)(
        SYS_fchownat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as u32,
        args[3] as u32,
        args[4] as i32,
        rval,
    )
}

/// 317 = `int linkat(int fd1, const char *path1, int fd2, const char *path2, int flag)`.
fn sys_linkat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.linkat)(
        SYS_linkat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as usize,
        args[4] as i32,
        rval,
    )
}

/// 318 = `int mkdirat(int fd, const char *path, mode_t mode)`.
fn sys_mkdirat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.mkdirat)(
        SYS_mkdirat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as u32,
        rval,
    )
}

/// 319 = `int mkfifoat(int fd, const char *path, mode_t mode)`.
fn sys_mkfifoat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.mkfifoat)(
        SYS_mkfifoat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as u32,
        rval,
    )
}

/// 320 = `int mknodat(int fd, const char *path, mode_t mode, dev_t dev)`.
fn sys_mknodat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.mknodat)(
        SYS_mknodat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as u32,
        args[3] as i32,
        rval,
    )
}

/// 321 = `int openat(int fd, const char *path, int flags, mode_t mode)`.
fn sys_openat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.openat)(
        SYS_openat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as u32,
        rval,
    )
}

/// 322 = `ssize_t readlinkat(int fd, const char *path, char *buf, size_t count)`.
fn sys_readlinkat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.readlinkat)(
        SYS_readlinkat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as usize,
        args[3] as usize,
        rval,
    )
}

/// 323 = `int renameat(int fromfd, const char *from, int tofd, const char *to)`.
fn sys_renameat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.renameat)(
        SYS_renameat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        args[3] as usize,
        rval,
    )
}

/// 324 = `int symlinkat(const char *path, int fd, const char *link)`.
fn sys_symlinkat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.symlinkat)(
        SYS_symlinkat as Register,
        args[0] as usize,
        args[1] as i32,
        args[2] as usize,
        rval,
    )
}

/// 325 = `int unlinkat(int fd, const char *path, int flag)`.
fn sys_unlinkat(api: &'static KernelApi, args: &SysArgs, rval: &mut Rval) -> SysResult {
    (api.vfs.unlinkat)(
        SYS_unlinkat as Register,
        args[0] as i32,
        args[1] as usize,
        args[2] as i32,
        rval,
    )
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

    /// The event entries of the test table: the arguments as digits, in order.
    fn ev_select(nr: Register, nd: i32, s: [usize; 3], tv: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(nd * 10000 + (s[0] * 1000 + s[1] * 100 + s[2] * 10 + tv) as i32)
    }
    fn ev_kevent(
        nr: Register,
        fd: i32,
        c: (usize, i32),
        e: (usize, i32),
        t: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(fd * 100_000
            + c.0 as i32 * 10_000
            + c.1 * 1000
            + e.0 as i32 * 100
            + e.1 * 10
            + t as i32)
    }
    fn ev_ppoll(nr: Register, f: usize, n: u32, t: usize, m: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err((f * 1000 + n as usize * 100 + t * 10 + m) as i32)
    }
    fn ev_pselect(
        nr: Register,
        nd: i32,
        s: [usize; 3],
        t: usize,
        m: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(nd * 100_000 + (s[0] * 10_000 + s[1] * 1000 + s[2] * 100 + t * 10 + m) as i32)
    }
    fn ev_poll(nr: Register, f: usize, n: u32, t: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(f as i32 * 100 + n as i32 * 10 + t)
    }

    fn sendsyslog(nr: Register, b: usize, n: usize, f: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(b as i32 * 100 + n as i32 * 10 + f)
    }

    const EVENT: hal::api::EventEntries = hal::api::EventEntries {
        select: ev_select,
        kevent: ev_kevent,
        ppoll: ev_ppoll,
        pselect: ev_pselect,
        poll: ev_poll,
        kqueue: count,
        kqueue1: fd1,
    };

    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_fork(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_vfork(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_tfork(nr: Register, param: usize, psize: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((param as i64) * 10 + (psize as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_exit(nr: Register, status: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((status as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_threxit(nr: Register, notdead: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((notdead as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_wait4(
        nr: Register,
        pid: i32,
        status: usize,
        options: i32,
        rusage: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1000
            + (status as i64) * 100
            + (options as i64) * 10
            + (rusage as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_waitid(
        nr: Register,
        idtype: i32,
        id: u32,
        info: usize,
        options: i32,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(
            ((idtype as i64) * 1000 + (id as i64) * 100 + (info as i64) * 10 + (options as i64) * 1)
                as i32,
        )
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrusage(nr: Register, who: i32, rusage: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((who as i64) * 10 + (rusage as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrlimit(nr: Register, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (rlp as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setrlimit(nr: Register, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (rlp as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpriority(nr: Register, which: i32, who: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (who as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setpriority(nr: Register, which: i32, who: u32, prio: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 100 + (who as i64) * 10 + (prio as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_acct(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((path as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getppid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getuid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_geteuid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getgid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getegid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getgroups(nr: Register, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gidsetsize as i64) * 10 + (gidset as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setgroups(nr: Register, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gidsetsize as i64) * 10 + (gidset as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpgrp(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpgid(nr: Register, pid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getsid(nr: Register, pid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setpgid(nr: Register, pid: i32, pgid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 10 + (pgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setsid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setuid(nr: Register, uid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((uid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_seteuid(nr: Register, euid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((euid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setgid(nr: Register, gid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setegid(nr: Register, egid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((egid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setreuid(nr: Register, ruid: u32, euid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 10 + (euid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setregid(nr: Register, rgid: u32, egid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 10 + (egid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setresuid(nr: Register, ruid: u32, euid: u32, suid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 100 + (euid as i64) * 10 + (suid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setresgid(nr: Register, rgid: u32, egid: u32, sgid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 100 + (egid as i64) * 10 + (sgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getresuid(
        nr: Register,
        ruid: usize,
        euid: usize,
        suid: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 100 + (euid as i64) * 10 + (suid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getresgid(
        nr: Register,
        rgid: usize,
        egid: usize,
        sgid: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 100 + (egid as i64) * 10 + (sgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_issetugid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getlogin_r(nr: Register, namebuf: usize, namelen: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((namebuf as i64) * 10 + (namelen as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setlogin(nr: Register, namebuf: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((namebuf as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_set_tcb(nr: Register, tcb: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tcb as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_get_tcb(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getthrid(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getthrname(nr: Register, tid: i32, name: usize, len: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tid as i64) * 100 + (name as i64) * 10 + (len as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setthrname(nr: Register, tid: i32, name: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tid as i64) * 10 + (name as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrtable(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setrtable(nr: Register, rtableid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rtableid as i64) * 1) as i32)
    }

    const PROC: hal::api::ProcEntries = hal::api::ProcEntries {
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

    /// The process system calls N4.6a made `Native` (`KernelApi::proc`).
    const PROC_CALLS: [i32; 46] = [
        SYS_fork,
        SYS_vfork,
        SYS___tfork,
        SYS_exit,
        SYS___threxit,
        SYS_wait4,
        SYS_waitid,
        SYS_getrusage,
        SYS_getrlimit,
        SYS_setrlimit,
        SYS_getpriority,
        SYS_setpriority,
        SYS_acct,
        SYS_getpid,
        SYS_getppid,
        SYS_getuid,
        SYS_geteuid,
        SYS_getgid,
        SYS_getegid,
        SYS_getgroups,
        SYS_setgroups,
        SYS_getpgrp,
        SYS_getpgid,
        SYS_getsid,
        SYS_setpgid,
        SYS_setsid,
        SYS_setuid,
        SYS_seteuid,
        SYS_setgid,
        SYS_setegid,
        SYS_setreuid,
        SYS_setregid,
        SYS_setresuid,
        SYS_setresgid,
        SYS_getresuid,
        SYS_getresgid,
        SYS_issetugid,
        SYS_getlogin_r,
        SYS_setlogin,
        SYS___set_tcb,
        SYS___get_tcb,
        SYS_getthrid,
        SYS_getthrname,
        SYS_setthrname,
        SYS_getrtable,
        SYS_setrtable,
    ];

    fn futex(
        nr: Register,
        f: usize,
        op: i32,
        val: i32,
        t: usize,
        g: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err((f * 10_000) as i32 + op * 1000 + val * 100 + (t * 10 + g) as i32)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the entry's shape: the number, six arguments, the return registers"
    )]
    fn sysctl(
        nr: Register,
        name: usize,
        namelen: u32,
        old: usize,
        oldlenp: usize,
        new: usize,
        newlen: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        // The six arguments as the digits of one number, in their order.
        Err((name * 100_000
            + namelen as usize * 10_000
            + old * 1000
            + oldlenp * 100
            + new * 10
            + newlen) as i32)
    }

    fn t_clock(nr: Register, clock: i32, tp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(clock * 10 + tp as i32)
    }
    fn t_pair(nr: Register, a: usize, b: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err((a * 10 + b) as i32)
    }
    fn t_setitimer(
        nr: Register,
        which: i32,
        itv: usize,
        oitv: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(which * 100 + (itv * 10 + oitv) as i32)
    }
    fn t_reboot(nr: Register, opt: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(opt)
    }

    const TIME: hal::api::TimeEntries = hal::api::TimeEntries {
        clock_gettime: t_clock,
        clock_settime: t_clock,
        clock_getres: t_clock,
        nanosleep: t_pair,
        gettimeofday: t_pair,
        settimeofday: t_pair,
        adjfreq: t_pair,
        adjtime: t_pair,
        getitimer: t_clock,
        setitimer: t_setitimer,
        reboot: t_reboot,
    };

    /// The time system calls and reboot(2) N4.8 made `Native` (`KernelApi::time`).
    const TIME_CALLS: [i32; 11] = [
        SYS_clock_gettime,
        SYS_clock_settime,
        SYS_clock_getres,
        SYS_nanosleep,
        SYS_gettimeofday,
        SYS_settimeofday,
        SYS_adjfreq,
        SYS_adjtime,
        SYS_getitimer,
        SYS_setitimer,
        SYS_reboot,
    ];

    fn sigreturn(nr: Register, scp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(scp as i32)
    }

    // The VFS entries of the test table (version 12): the number in rval[0], the
    // arguments as decimal digits in the error.
    fn vfs_digits(a: &[i64]) -> i32 {
        a.iter().fold(0, |d, &v| d * 10 + v as i32)
    }
    fn vfs0(nr: Register, a0: usize, a1: i32, a2: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs1(nr: Register, a0: usize, a1: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn vfs2(nr: Register, a0: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64]))
    }
    fn vfs3(nr: Register, a0: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64]))
    }
    fn vfs4(nr: Register, a0: usize, a1: u32, a2: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs5(nr: Register, a0: usize, a1: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn vfs6(nr: Register, a0: usize, a1: u32, a2: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs7(nr: Register, a0: usize, a1: usize, a2: i32, a3: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs8(nr: Register, a0: usize, a1: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn vfs9(nr: Register, a0: i32, a1: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn vfs10(nr: Register, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[]))
    }
    fn vfs11(nr: Register, a0: i32, a1: usize, a2: usize, a3: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs12(nr: Register, a0: usize, a1: usize, a2: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs13(nr: Register, a0: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64]))
    }
    fn vfs14(nr: Register, a0: usize, a1: usize, a2: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs15(nr: Register, a0: i32, a1: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn vfs16(nr: Register, a0: i32, a1: usize, a2: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs17(nr: Register, a0: i32, a1: usize, a2: u32, a3: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs18(nr: Register, a0: i32, a1: u32, a2: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs19(nr: Register, a0: usize, a1: i32, a2: i32, a3: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs20(nr: Register, a0: i32, a1: i64, a2: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs21(nr: Register, a0: usize, a1: i64, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn vfs22(nr: Register, a0: i32, a1: i64, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn vfs23(nr: Register, a0: i32, a1: usize, a2: i32, a3: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs24(
        nr: Register,
        a0: i32,
        a1: usize,
        a2: u32,
        a3: u32,
        a4: i32,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[
            a0 as i64, a1 as i64, a2 as i64, a3 as i64, a4 as i64,
        ]))
    }
    fn vfs25(
        nr: Register,
        a0: i32,
        a1: usize,
        a2: i32,
        a3: usize,
        a4: i32,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[
            a0 as i64, a1 as i64, a2 as i64, a3 as i64, a4 as i64,
        ]))
    }
    fn vfs26(nr: Register, a0: i32, a1: usize, a2: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs27(nr: Register, a0: i32, a1: usize, a2: i32, a3: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs28(nr: Register, a0: i32, a1: usize, a2: usize, a3: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs29(nr: Register, a0: i32, a1: usize, a2: i32, a3: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn vfs30(nr: Register, a0: usize, a1: i32, a2: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn vfs31(nr: Register, a0: i32, a1: usize, a2: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    const VFS: hal::api::VfsEntries = hal::api::VfsEntries {
        open: vfs0,
        link: vfs1,
        unlink: vfs2,
        chdir: vfs2,
        fchdir: vfs3,
        mknod: vfs4,
        chmod: vfs5,
        chown: vfs6,
        mount: vfs7,
        unmount: vfs8,
        access: vfs8,
        chflags: vfs5,
        fchflags: vfs9,
        sync: vfs10,
        stat: vfs1,
        lstat: vfs1,
        fstatat: vfs11,
        revoke: vfs2,
        symlink: vfs1,
        readlink: vfs12,
        umask: vfs13,
        chroot: vfs2,
        getfsstat: vfs14,
        statfs: vfs1,
        fstatfs: vfs15,
        fhstatfs: vfs1,
        utimes: vfs1,
        futimes: vfs15,
        utimensat: vfs11,
        futimens: vfs15,
        fsync: vfs3,
        getdents: vfs16,
        chflagsat: vfs17,
        unveil: vfs1,
        realpath: vfs1,
        fchown: vfs18,
        fchmod: vfs9,
        pledge_open: vfs0,
        rename: vfs1,
        mkfifo: vfs5,
        mkdir: vfs5,
        rmdir: vfs2,
        quotactl: vfs19,
        getfh: vfs1,
        lseek: vfs20,
        truncate: vfs21,
        ftruncate: vfs22,
        pathconfat: vfs23,
        pathconf: vfs8,
        lchown: vfs6,
        fhopen: vfs8,
        fhstat: vfs1,
        getcwd: vfs1,
        faccessat: vfs23,
        fchmodat: vfs17,
        fchownat: vfs24,
        linkat: vfs25,
        mkdirat: vfs26,
        mkfifoat: vfs26,
        mknodat: vfs17,
        openat: vfs27,
        readlinkat: vfs28,
        renameat: vfs29,
        symlinkat: vfs30,
        unlinkat: vfs31,
    };

    static API: KernelApi = KernelApi::new(
        register, legacy, copyin, copyout, pipe2, VM, FD, EVENT, sendsyslog, futex, PROC, sysctl,
        TIME, sigreturn, VFS,
    );

    /// The event system calls N4.4 made `Native` (`KernelApi::event`).
    const EVENT_CALLS: [i32; 7] = [
        SYS_select,
        SYS_kevent,
        SYS_ppoll,
        SYS_pselect,
        SYS_poll,
        SYS_kqueue,
        SYS_kqueue1,
    ];

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
    fn every_entry_but_pipe_vm_fd_event_sendsyslog_futex_proc_sysctl_time_sigreturn_and_vfs_is_legacy()
     {
        for nr in 0..SYS_MAXSYSCALL as Register {
            if nr == SYS_pipe as Register
                || nr == SYS_pipe2 as Register
                || nr == SYS_sendsyslog as Register
                || nr == SYS_futex as Register
                || nr == SYS_sysctl as Register
                || nr == SYS_sigreturn as Register
                || VM_CALLS.iter().any(|&v| v as Register == nr)
                || FD_CALLS.iter().any(|&v| v as Register == nr)
                || EVENT_CALLS.iter().any(|&v| v as Register == nr)
                || PROC_CALLS.iter().any(|&v| v as Register == nr)
                || TIME_CALLS.iter().any(|&v| v as Register == nr)
                || VFS_CALLS.iter().any(|&v| v as Register == nr)
            {
                continue;
            }
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr, &[0; 6], &mut rval), Err(ENOSYS));
            assert_eq!(rval[0], nr, "number {nr} reached the fallback unchanged");
        }
    }

    /// Each time translation (and reboot(2)'s) calls its own entry with its own number, the
    /// arguments in `<sys/syscallargs.h>`'s order and the trap's return registers, through
    /// the table as well.
    #[test]
    fn the_time_calls_are_native_and_pass_their_number_arguments_and_rval() {
        for nr in TIME_CALLS {
            assert!(
                matches!(SYSENT[nr as usize], Sysent::Native(_)),
                "SYS {nr} is Native"
            );
        }
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let cases: [(NativeFn, i32, i32); 11] = [
            (sys_clock_gettime, SYS_clock_gettime, 12),
            (sys_clock_settime, SYS_clock_settime, 12),
            (sys_clock_getres, SYS_clock_getres, 12),
            (sys_nanosleep, SYS_nanosleep, 12),
            (sys_gettimeofday, SYS_gettimeofday, 12),
            (sys_settimeofday, SYS_settimeofday, 12),
            (sys_adjfreq, SYS_adjfreq, 12),
            (sys_adjtime, SYS_adjtime, 12),
            (sys_getitimer, SYS_getitimer, 12),
            (sys_setitimer, SYS_setitimer, 123),
            (sys_reboot, SYS_reboot, 1),
        ];
        for (f, nr, want) in cases {
            let mut rval = [-7; 2];
            assert_eq!(f(&API, &a, &mut rval), Err(want), "SYS {nr}");
            assert_eq!(rval, [nr as Register, -7], "SYS {nr} wrote rval[0] only");
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr as Register, &a, &mut rval), Err(want));
            assert_eq!(rval[0], nr as Register, "SYS {nr} reached its time entry");
        }
    }

    /// The VFS system calls (N5.4), in `syscalls.master`'s order.
    const VFS_CALLS: [i32; 65] = [
        SYS_open,
        SYS_link,
        SYS_unlink,
        SYS_chdir,
        SYS_fchdir,
        SYS_mknod,
        SYS_chmod,
        SYS_chown,
        SYS_mount,
        SYS_unmount,
        SYS_access,
        SYS_chflags,
        SYS_fchflags,
        SYS_sync,
        SYS_stat,
        SYS_lstat,
        SYS_fstatat,
        SYS_revoke,
        SYS_symlink,
        SYS_readlink,
        SYS_umask,
        SYS_chroot,
        SYS_getfsstat,
        SYS_statfs,
        SYS_fstatfs,
        SYS_fhstatfs,
        SYS_utimes,
        SYS_futimes,
        SYS_utimensat,
        SYS_futimens,
        SYS_fsync,
        SYS_getdents,
        SYS_chflagsat,
        SYS_unveil,
        SYS___realpath,
        SYS_fchown,
        SYS_fchmod,
        SYS___pledge_open,
        SYS_rename,
        SYS_mkfifo,
        SYS_mkdir,
        SYS_rmdir,
        SYS_quotactl,
        SYS_getfh,
        SYS_lseek,
        SYS_truncate,
        SYS_ftruncate,
        SYS_pathconfat,
        SYS_pathconf,
        SYS_lchown,
        SYS_fhopen,
        SYS_fhstat,
        SYS___getcwd,
        SYS_faccessat,
        SYS_fchmodat,
        SYS_fchownat,
        SYS_linkat,
        SYS_mkdirat,
        SYS_mkfifoat,
        SYS_mknodat,
        SYS_openat,
        SYS_readlinkat,
        SYS_renameat,
        SYS_symlinkat,
        SYS_unlinkat,
    ];

    /// Each VFS translation (N5.4) calls its own entry with its own number, the arguments in
    /// `<sys/syscallargs.h>`'s order as the entry types them (a pointer or a `size_t` a
    /// `usize`, an `int` or a `dev_t` an `i32`, a `mode_t`, `uid_t`, `gid_t` or `u_int` a
    /// `u32`, an `off_t` an `i64`) and the trap's return registers, through the table as well.
    #[test]
    fn the_vfs_calls_are_native_and_pass_their_number_arguments_and_rval() {
        for nr in VFS_CALLS {
            assert!(
                matches!(SYSENT[nr as usize], Sysent::Native(_)),
                "SYS {nr} is Native"
            );
        }
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let cases: [(NativeFn, i32, i32); 65] = [
            (sys_open, SYS_open, 123),
            (sys_link, SYS_link, 12),
            (sys_unlink, SYS_unlink, 1),
            (sys_chdir, SYS_chdir, 1),
            (sys_fchdir, SYS_fchdir, 1),
            (sys_mknod, SYS_mknod, 123),
            (sys_chmod, SYS_chmod, 12),
            (sys_chown, SYS_chown, 123),
            (sys_mount, SYS_mount, 1234),
            (sys_unmount, SYS_unmount, 12),
            (sys_access, SYS_access, 12),
            (sys_chflags, SYS_chflags, 12),
            (sys_fchflags, SYS_fchflags, 12),
            (sys_sync, SYS_sync, 0),
            (sys_stat, SYS_stat, 12),
            (sys_lstat, SYS_lstat, 12),
            (sys_fstatat, SYS_fstatat, 1234),
            (sys_revoke, SYS_revoke, 1),
            (sys_symlink, SYS_symlink, 12),
            (sys_readlink, SYS_readlink, 123),
            (sys_umask, SYS_umask, 1),
            (sys_chroot, SYS_chroot, 1),
            (sys_getfsstat, SYS_getfsstat, 123),
            (sys_statfs, SYS_statfs, 12),
            (sys_fstatfs, SYS_fstatfs, 12),
            (sys_fhstatfs, SYS_fhstatfs, 12),
            (sys_utimes, SYS_utimes, 12),
            (sys_futimes, SYS_futimes, 12),
            (sys_utimensat, SYS_utimensat, 1234),
            (sys_futimens, SYS_futimens, 12),
            (sys_fsync, SYS_fsync, 1),
            (sys_getdents, SYS_getdents, 123),
            (sys_chflagsat, SYS_chflagsat, 1234),
            (sys_unveil, SYS_unveil, 12),
            (sys_realpath, SYS___realpath, 12),
            (sys_fchown, SYS_fchown, 123),
            (sys_fchmod, SYS_fchmod, 12),
            (sys_pledge_open, SYS___pledge_open, 123),
            (sys_rename, SYS_rename, 12),
            (sys_mkfifo, SYS_mkfifo, 12),
            (sys_mkdir, SYS_mkdir, 12),
            (sys_rmdir, SYS_rmdir, 1),
            (sys_quotactl, SYS_quotactl, 1234),
            (sys_getfh, SYS_getfh, 12),
            (sys_lseek, SYS_lseek, 123),
            (sys_truncate, SYS_truncate, 12),
            (sys_ftruncate, SYS_ftruncate, 12),
            (sys_pathconfat, SYS_pathconfat, 1234),
            (sys_pathconf, SYS_pathconf, 12),
            (sys_lchown, SYS_lchown, 123),
            (sys_fhopen, SYS_fhopen, 12),
            (sys_fhstat, SYS_fhstat, 12),
            (sys_getcwd, SYS___getcwd, 12),
            (sys_faccessat, SYS_faccessat, 1234),
            (sys_fchmodat, SYS_fchmodat, 1234),
            (sys_fchownat, SYS_fchownat, 12345),
            (sys_linkat, SYS_linkat, 12345),
            (sys_mkdirat, SYS_mkdirat, 123),
            (sys_mkfifoat, SYS_mkfifoat, 123),
            (sys_mknodat, SYS_mknodat, 1234),
            (sys_openat, SYS_openat, 1234),
            (sys_readlinkat, SYS_readlinkat, 1234),
            (sys_renameat, SYS_renameat, 1234),
            (sys_symlinkat, SYS_symlinkat, 123),
            (sys_unlinkat, SYS_unlinkat, 123),
        ];
        for (f, nr, want) in cases {
            let mut rval = [-7; 2];
            assert_eq!(f(&API, &a, &mut rval), Err(want), "SYS {nr}");
            assert_eq!(rval, [nr as Register, -7], "SYS {nr} wrote rval[0] only");
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr as Register, &a, &mut rval), Err(want));
            assert_eq!(rval[0], nr as Register, "SYS {nr} reached its VFS entry");
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
    }

    /// Each event translation calls its own entry with its own number, the arguments in
    /// `<sys/syscallargs.h>`'s order and the trap's return registers.
    #[test]
    fn the_event_calls_are_native_and_pass_their_number_arguments_and_rval() {
        for nr in EVENT_CALLS {
            assert!(
                matches!(SYSENT[nr as usize], Sysent::Native(_)),
                "SYS {nr} is Native"
            );
        }
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let cases: [(NativeFn, i32, i32); 7] = [
            (sys_select, SYS_select, 12345),
            (sys_kevent, SYS_kevent, 123_456),
            (sys_ppoll, SYS_ppoll, 1234),
            (sys_pselect, SYS_pselect, 123_456),
            (sys_poll, SYS_poll, 123),
            (sys_kqueue, SYS_kqueue, 0),
            (sys_kqueue1, SYS_kqueue1, 1),
        ];
        for (f, nr, want) in cases {
            let mut rval = [-7; 2];
            assert_eq!(f(&API, &a, &mut rval), Err(want), "SYS {nr}");
            assert_eq!(rval, [nr as Register, -7], "SYS {nr} wrote rval[0] only");
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr as Register, &a, &mut rval), Err(want));
        }
    }

    #[test]
    fn sendsyslog_is_native_and_reaches_the_kernels_entry() {
        assert!(matches!(
            entry(&SYSENT, SYS_sendsyslog as Register),
            Some(Sysent::Native(_))
        ));
        let a: SysArgs = [1, 2, 3, 0, 0, 0];
        let mut rval = [-7; 2];
        assert_eq!(sys_sendsyslog(&API, &a, &mut rval), Err(123));
        assert_eq!(rval, [SYS_sendsyslog as Register, -7]);
        let mut rval = [-7; 2];
        assert_eq!(
            syscall(&API, SYS_sendsyslog as Register, &a, &mut rval),
            Err(123)
        );
    }

    /// futex(2)'s translation (N4.6c): its number, the five arguments in
    /// `<sys/syscallargs.h>`'s order, the trap's return registers.
    #[test]
    fn futex_is_native_and_passes_its_number_arguments_and_rval() {
        assert!(matches!(SYSENT[SYS_futex as usize], Sysent::Native(_)));
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let mut rval = [-7; 2];
        assert_eq!(sys_futex(&API, &a, &mut rval), Err(12345));
        assert_eq!(rval, [SYS_futex as Register, -7]);
        let mut rval = [-7; 2];
        assert_eq!(
            syscall(&API, SYS_futex as Register, &a, &mut rval),
            Err(12345)
        );
    }

    /// sysctl(2)'s translation (N4.6b): its number, the six arguments in
    /// `<sys/syscallargs.h>`'s order, the trap's return registers.
    #[test]
    fn sysctl_is_native_and_passes_its_number_arguments_and_rval() {
        assert!(matches!(SYSENT[SYS_sysctl as usize], Sysent::Native(_)));
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let mut rval = [-7; 2];
        assert_eq!(sys_sysctl(&API, &a, &mut rval), Err(123_456));
        assert_eq!(rval, [SYS_sysctl as Register, -7]);
        let mut rval = [-7; 2];
        assert_eq!(
            syscall(&API, SYS_sysctl as Register, &a, &mut rval),
            Err(123_456)
        );
    }

    /// sigreturn(2)'s translation (N4.8b): its number and the context's address, the trap's
    /// return registers; the table's count with it.
    #[test]
    fn sigreturn_is_native_and_passes_its_number_and_context() {
        assert!(matches!(SYSENT[SYS_sigreturn as usize], Sysent::Native(_)));
        let a: SysArgs = [4242, 2, 3, 4, 5, 6];
        let mut rval = [-7; 2];
        assert_eq!(sys_sigreturn(&API, &a, &mut rval), Err(4242));
        assert_eq!(rval, [SYS_sigreturn as Register, -7]);
        let mut rval = [-7; 2];
        assert_eq!(
            syscall(&API, SYS_sigreturn as Register, &a, &mut rval),
            Err(4242)
        );
        assert_eq!(
            NATIVE, 170,
            "93 through sysctl(2), 11 more with the time calls, 1 with sigreturn(2), 65 with the \
             VFS calls (N5.4)"
        );
    }

    #[test]
    fn the_proc_translations_pass_their_number_and_arguments_in_order() {
        // Arguments 1, 2, 3, 4 in the registers: each stub answers them as digits.
        let args: SysArgs = [1, 2, 3, 4, 0, 0];
        for nr in PROC_CALLS {
            let mut rval: Rval = [0; 2];
            let r = dispatch(&SYSENT, &API, nr as Register, &args, &mut rval);
            assert_eq!(rval[0], nr as Register, "number {nr}");
            assert!(r.is_err(), "number {nr}");
        }
        let mut rval: Rval = [0; 2];
        assert_eq!(
            dispatch(&SYSENT, &API, SYS_wait4 as Register, &args, &mut rval),
            Err(1234)
        );
        assert_eq!(
            dispatch(&SYSENT, &API, SYS_setpriority as Register, &args, &mut rval),
            Err(123)
        );
        assert_eq!(
            dispatch(&SYSENT, &API, SYS_getpid as Register, &args, &mut rval),
            Err(0)
        );
    }
}
/* </TESTS> */
