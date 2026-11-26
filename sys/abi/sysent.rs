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
//! The ABI's system call table: one entry per system call number of
//! `<sys/syscall.h>` (`hal::sys::syscall`, the ABI edge), each `Native` (translated into a
//! `kernel` entry point) or `Legacy` (handed to the LZ path `bsd` registered, through
//! `KernelApi::syscall_legacy`), and the dispatcher `hal` reaches through the handler
//! [`crate::abi_init`] registered.
//!
//! N2.4: every entry was `Legacy`, so every system call behaves exactly as LZ's: the number
//! is not even read here but to pick the entry, and an out-of-range number goes to the
//! fallback too, which answers `ENOSYS` as the traps did. A subsystem that moves into
//! `kernel` flips its entries to `Native` and raises [`NATIVE`]: N2.6, pipe(2) and pipe2(2)
//! (`KernelApi::pipe2`, `sys/kernel/file/pipe.rs`). N3.5: the virtual memory system's fifteen
//! (`KernelApi::vm`, `sys/kernel/vm/mmap.rs` and `unix.rs`): written in S1, `Native` since
//! the flip (S2). N4.3: the descriptor tables' twenty (`KernelApi::fd`,
//! `sys/kernel/file/descrip.rs`: `read(2)`, `write(2)`, `close(2)`, `dup(2)`, `fcntl(2)`,
//! `ioctl(2)`, ...), which write the return registers themselves. N4.4: the event system
//! calls' seven (`KernelApi::event`, `sys/kernel/file/event.rs` and `select.rs`: `kqueue(2)`,
//! `kqueue1(2)`, `kevent(2)`, `select(2)`, `pselect(2)`, `poll(2)`, `ppoll(2)`). N4.7a:
//! `sendsyslog(2)` (`KernelApi::sendsyslog`, `sys/kernel/base/log.rs`). N4.6c: futex(2)
//! (`KernelApi::futex`, `sys/kernel/sched/futex.rs`). N4.6a: the process system calls'
//! forty-six (`KernelApi::proc`, `sys/kernel/proc/fork.rs`, `exit.rs`, `resource.rs`, `prot.rs`,
//! `acct.rs`). N4.6b: sysctl(2) (`KernelApi::sysctl`, `sys/kernel/base/sysctl.rs`). N4.8: the
//! ten time system calls and reboot(2) (`KernelApi::time`, `sys/kernel/time/mod.rs` and
//! `reboot.rs`). N4.8b: sigreturn(2) (`KernelApi::sigreturn`, `sys/kernel/sig/mod.rs` over
//! `hal::cpu::sig`'s frame). N5.4: the VFS system calls' sixty-five (`KernelApi::vfs`,
//! `sys/kernel/fs/syscalls.rs` and `getcwd.rs`: the sixty-four of `vfs_syscalls.c`, `open(2)`,
//! `stat(2)`, `mount(2)`, `unveil(2)`, ..., and `__getcwd(2)`), which write the return
//! registers themselves. N6.2b: the socket system calls' nineteen (`KernelApi::socket`,
//! `sys/kernel/net/syscalls.rs`: `socket(2)`, `bind(2)`, `accept(2)`, `sendmsg(2)`,
//! `recvmmsg(2)`, ..., `ypconnect(2)`; `setrtable(2)` and `getrtable(2)` are `KernelApi::proc`'s).
//! N6.7b4: the NFS server's `nfssvc(2)` (`KernelApi::nfssvc`, `sys/kernel/fs/nfs/nfssvc.rs`).
//!
//! Translation only (decision 21; `cargo xtask abi-check`, decision 25): no loop, a
//! `match` only on the number's conversion and on `Some`/`None`, one kernel call per
//! function, no state. Every argument is read out of its register as the C's `SCARG` reads
//! `syscalls.master`'s type, through the function of [`crate::scarg`] named after its family
//! (`int`, `u_int`, `ptr`, `size_t`, `off_t`, `u_long`; N5 close, the abi casts).
//!
//! N7 (decision 52's slice): the translations left this file for one file per group of the
//! kernel they reach, so each file mirrors the kernel's modules: [`crate::proc`] (the
//! processes, futex(2), sigreturn(2)), [`crate::vm`], [`crate::file`] (pipe(2) and the
//! descriptors), [`crate::event`], [`crate::time`], [`crate::base`] (sysctl(2),
//! sendsyslog(2), reboot(2)), [`crate::fs`] (the VFS and nfssvc(2)) and [`crate::net`] (the
//! sockets). This file keeps the table and the dispatcher.

use crate::base::{sys_reboot, sys_sendsyslog, sys_sysctl};
use crate::event::{
    sys_kevent, sys_kqueue, sys_kqueue1, sys_poll, sys_ppoll, sys_pselect, sys_select,
};
use crate::file::{
    sys_close, sys_closefrom, sys_dup, sys_dup2, sys_dup3, sys_fcntl, sys_flock, sys_fpathconf,
    sys_fstat, sys_getdtablecount, sys_ioctl, sys_pipe, sys_pipe2, sys_pread, sys_preadv,
    sys_pwrite, sys_pwritev, sys_read, sys_readv, sys_utrace, sys_write, sys_writev,
};
use crate::fs::{
    sys_access, sys_chdir, sys_chflags, sys_chflagsat, sys_chmod, sys_chown, sys_chroot,
    sys_faccessat, sys_fchdir, sys_fchflags, sys_fchmod, sys_fchmodat, sys_fchown, sys_fchownat,
    sys_fhopen, sys_fhstat, sys_fhstatfs, sys_fstatat, sys_fstatfs, sys_fsync, sys_ftruncate,
    sys_futimens, sys_futimes, sys_getcwd, sys_getdents, sys_getfh, sys_getfsstat, sys_lchown,
    sys_link, sys_linkat, sys_lseek, sys_lstat, sys_mkdir, sys_mkdirat, sys_mkfifo, sys_mkfifoat,
    sys_mknod, sys_mknodat, sys_mount, sys_nfssvc, sys_open, sys_openat, sys_pathconf,
    sys_pathconfat, sys_pledge_open, sys_quotactl, sys_readlink, sys_readlinkat, sys_realpath,
    sys_rename, sys_renameat, sys_revoke, sys_rmdir, sys_stat, sys_statfs, sys_symlink,
    sys_symlinkat, sys_sync, sys_truncate, sys_umask, sys_unlink, sys_unlinkat, sys_unmount,
    sys_unveil, sys_utimensat, sys_utimes,
};
use crate::net::{
    sys_accept, sys_accept4, sys_bind, sys_connect, sys_getpeername, sys_getsockname,
    sys_getsockopt, sys_listen, sys_recvfrom, sys_recvmmsg, sys_recvmsg, sys_sendmmsg, sys_sendmsg,
    sys_sendto, sys_setsockopt, sys_shutdown, sys_socket, sys_socketpair, sys_ypconnect,
};
use crate::proc::{
    sys_acct, sys_exit, sys_fork, sys_futex, sys_get_tcb, sys_getegid, sys_geteuid, sys_getgid,
    sys_getgroups, sys_getlogin_r, sys_getpgid, sys_getpgrp, sys_getpid, sys_getppid,
    sys_getpriority, sys_getresgid, sys_getresuid, sys_getrlimit, sys_getrtable, sys_getrusage,
    sys_getsid, sys_getthrid, sys_getthrname, sys_getuid, sys_issetugid, sys_set_tcb, sys_setegid,
    sys_seteuid, sys_setgid, sys_setgroups, sys_setlogin, sys_setpgid, sys_setpriority,
    sys_setregid, sys_setresgid, sys_setresuid, sys_setreuid, sys_setrlimit, sys_setrtable,
    sys_setsid, sys_setthrname, sys_setuid, sys_sigreturn, sys_tfork, sys_threxit, sys_vfork,
    sys_wait4, sys_waitid,
};
use crate::time::{
    sys_adjfreq, sys_adjtime, sys_clock_getres, sys_clock_gettime, sys_clock_settime,
    sys_getitimer, sys_gettimeofday, sys_nanosleep, sys_setitimer, sys_settimeofday,
};
use crate::vm::{
    sys_kbind, sys_madvise, sys_mimmutable, sys_minherit, sys_mlock, sys_mlockall, sys_mmap,
    sys_mprotect, sys_mquery, sys_msync, sys_munlock, sys_munlockall, sys_munmap, sys_obreak,
    sys_pinsyscalls,
};
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
    SYS_munmap, SYS_nfssvc, SYS_pinsyscalls, SYS_pipe, SYS_pipe2, SYS_poll, SYS_ppoll, SYS_pread,
    SYS_preadv, SYS_pselect, SYS_pwrite, SYS_pwritev, SYS_read, SYS_readv, SYS_select,
    SYS_sendsyslog, SYS_setegid, SYS_seteuid, SYS_setgid, SYS_setgroups, SYS_setlogin, SYS_setpgid,
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
    SYS_accept, SYS_accept4, SYS_bind, SYS_connect, SYS_getpeername, SYS_getsockname,
    SYS_getsockopt, SYS_listen, SYS_recvfrom, SYS_recvmmsg, SYS_recvmsg, SYS_sendmmsg, SYS_sendmsg,
    SYS_sendto, SYS_setsockopt, SYS_shutdown, SYS_socket, SYS_socketpair, SYS_ypconnect,
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
pub const NATIVE: u32 = 190;

/// What the ABI says about itself when it registers ([`crate::abi_init`]).
#[expect(
    clippy::cast_possible_truncation,
    reason = "SYS_MAXSYSCALL is 331; the compile-time check at the end of the code holds it within u32"
)]
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
    table[SYS_recvmsg as usize] = Sysent::Native(sys_recvmsg);
    table[SYS_sendmsg as usize] = Sysent::Native(sys_sendmsg);
    table[SYS_recvfrom as usize] = Sysent::Native(sys_recvfrom);
    table[SYS_accept as usize] = Sysent::Native(sys_accept);
    table[SYS_getpeername as usize] = Sysent::Native(sys_getpeername);
    table[SYS_getsockname as usize] = Sysent::Native(sys_getsockname);
    table[SYS_accept4 as usize] = Sysent::Native(sys_accept4);
    table[SYS_socket as usize] = Sysent::Native(sys_socket);
    table[SYS_connect as usize] = Sysent::Native(sys_connect);
    table[SYS_bind as usize] = Sysent::Native(sys_bind);
    table[SYS_setsockopt as usize] = Sysent::Native(sys_setsockopt);
    table[SYS_listen as usize] = Sysent::Native(sys_listen);
    table[SYS_recvmmsg as usize] = Sysent::Native(sys_recvmmsg);
    table[SYS_sendmmsg as usize] = Sysent::Native(sys_sendmmsg);
    table[SYS_getsockopt as usize] = Sysent::Native(sys_getsockopt);
    table[SYS_sendto as usize] = Sysent::Native(sys_sendto);
    table[SYS_shutdown as usize] = Sysent::Native(sys_shutdown);
    table[SYS_socketpair as usize] = Sysent::Native(sys_socketpair);
    table[SYS_ypconnect as usize] = Sysent::Native(sys_ypconnect);
    table[SYS_nfssvc as usize] = Sysent::Native(sys_nfssvc);
    table
};

/// The ABI's system call handler (`hal::api::SyscallHandler`).
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

/// The entry of number `nr`, if the table has one (a negative number has none).
fn entry(table: &[Sysent], nr: Register) -> Option<&Sysent> {
    match usize::try_from(nr) {
        Ok(i) => table.get(i),
        Err(_) => None,
    }
}

// `INFO` narrows the table's size to `HandlerInfo::syscalls`'s `u32`: it fits.
const _: () = assert!(SYS_MAXSYSCALL <= u32::MAX as usize);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "the digit stubs fold the number and the arguments into one i32"
)]
mod tests {
    use super::*;
    use crate::scarg::u_int;

    /// `ENOSYS`, the fallback's answer here.
    const ENOSYS: i32 = hal::sys::errno::Errno::ENOSYS.as_i32();

    /// The fallback of the test table: says which number reached it.
    fn legacy(nr: Register, _: &SysArgs, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(ENOSYS) // as the LZ path answers an out-of-range number
    }

    fn register(_: &'static KernelApi, _: hal::api::SyscallHandler, _: HandlerInfo) {}

    /// The kernel's pipe entry of the test table: says what reached it.
    fn pipe2(nr: Register, fdp: usize, flags: i32) -> SysResult {
        Err(nr as i32 * 1000 + fdp as i32 + flags)
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

    fn nfssvc(nr: Register, flag: i32, argp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(flag * 10 + argp as i32)
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

    // The socket entries of the test table (N6.2b): the number in rval[0], the arguments
    // as decimal digits in the error.
    fn so0(nr: Register, a0: i32, a1: usize, a2: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the entry's shape: the number, six arguments, the return registers"
    )]
    fn so1(
        nr: Register,
        a0: i32,
        a1: usize,
        a2: usize,
        a3: i32,
        a4: usize,
        a5: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[
            a0 as i64, a1 as i64, a2 as i64, a3 as i64, a4 as i64, a5 as i64,
        ]))
    }
    fn so2(nr: Register, a0: i32, a1: usize, a2: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn so3(nr: Register, a0: i32, a1: usize, a2: usize, a3: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn so4(nr: Register, a0: i32, a1: i32, a2: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn so5(nr: Register, a0: i32, a1: usize, a2: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64]))
    }
    fn so6(
        nr: Register,
        a0: i32,
        a1: i32,
        a2: i32,
        a3: usize,
        a4: u32,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[
            a0 as i64, a1 as i64, a2 as i64, a3 as i64, a4 as i64,
        ]))
    }
    fn so7(nr: Register, a0: i32, a1: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64]))
    }
    fn so8(
        nr: Register,
        a0: i32,
        a1: usize,
        a2: u32,
        a3: i32,
        a4: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[
            a0 as i64, a1 as i64, a2 as i64, a3 as i64, a4 as i64,
        ]))
    }
    fn so9(nr: Register, a0: i32, a1: usize, a2: u32, a3: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn so10(
        nr: Register,
        a0: i32,
        a1: i32,
        a2: i32,
        a3: usize,
        a4: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[
            a0 as i64, a1 as i64, a2 as i64, a3 as i64, a4 as i64,
        ]))
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the entry's shape: the number, six arguments, the return registers"
    )]
    fn so11(
        nr: Register,
        a0: i32,
        a1: usize,
        a2: usize,
        a3: i32,
        a4: usize,
        a5: u32,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[
            a0 as i64, a1 as i64, a2 as i64, a3 as i64, a4 as i64, a5 as i64,
        ]))
    }
    fn so12(nr: Register, a0: i32, a1: i32, a2: i32, a3: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64, a1 as i64, a2 as i64, a3 as i64]))
    }
    fn so13(nr: Register, a0: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(vfs_digits(&[a0 as i64]))
    }
    const SOCKET: hal::api::SocketEntries = hal::api::SocketEntries {
        recvmsg: so0,
        sendmsg: so0,
        recvfrom: so1,
        accept: so2,
        getpeername: so2,
        getsockname: so2,
        accept4: so3,
        socket: so4,
        connect: so5,
        bind: so5,
        setsockopt: so6,
        listen: so7,
        recvmmsg: so8,
        sendmmsg: so9,
        getsockopt: so10,
        sendto: so11,
        shutdown: so7,
        socketpair: so12,
        ypconnect: so13,
    };
    static API: KernelApi = KernelApi::new(
        register, legacy, pipe2, VM, FD, EVENT, sendsyslog, futex, PROC, sysctl, TIME, sigreturn,
        VFS, SOCKET, nfssvc,
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
    fn every_entry_but_pipe_vm_fd_event_sendsyslog_futex_proc_sysctl_time_sigreturn_vfs_socket_and_nfssvc_is_legacy()
     {
        for nr in 0..SYS_MAXSYSCALL as Register {
            if nr == SYS_pipe as Register
                || nr == SYS_pipe2 as Register
                || nr == SYS_sendsyslog as Register
                || nr == SYS_futex as Register
                || nr == SYS_sysctl as Register
                || nr == SYS_sigreturn as Register
                || nr == SYS_nfssvc as Register
                || VM_CALLS.iter().any(|&v| v as Register == nr)
                || FD_CALLS.iter().any(|&v| v as Register == nr)
                || EVENT_CALLS.iter().any(|&v| v as Register == nr)
                || PROC_CALLS.iter().any(|&v| v as Register == nr)
                || TIME_CALLS.iter().any(|&v| v as Register == nr)
                || VFS_CALLS.iter().any(|&v| v as Register == nr)
                || SOCKET_CALLS.iter().any(|&v| v as Register == nr)
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

    /// The socket system calls (N6.2b), in `syscalls.master`'s order.
    const SOCKET_CALLS: [i32; 19] = [
        SYS_recvmsg,
        SYS_sendmsg,
        SYS_recvfrom,
        SYS_accept,
        SYS_getpeername,
        SYS_getsockname,
        SYS_accept4,
        SYS_socket,
        SYS_connect,
        SYS_bind,
        SYS_setsockopt,
        SYS_listen,
        SYS_recvmmsg,
        SYS_sendmmsg,
        SYS_getsockopt,
        SYS_sendto,
        SYS_shutdown,
        SYS_socketpair,
        SYS_ypconnect,
    ];

    /// Each socket translation (N6.2b) calls its own entry with its own number, the
    /// arguments in `<sys/syscallargs.h>`'s order as the entry types them (a pointer or a
    /// `size_t` a `usize`, an `int` an `i32`, a `socklen_t` or an `unsigned int` a `u32`) and
    /// the trap's return registers, through the table as well.
    #[test]
    fn the_socket_calls_are_native_and_pass_their_number_arguments_and_rval() {
        for nr in SOCKET_CALLS {
            assert!(
                matches!(SYSENT[nr as usize], Sysent::Native(_)),
                "SYS {nr} is Native"
            );
        }
        let a: SysArgs = [1, 2, 3, 4, 5, 6];
        let cases: [(NativeFn, i32, i32); 19] = [
            (sys_recvmsg, SYS_recvmsg, 123),
            (sys_sendmsg, SYS_sendmsg, 123),
            (sys_recvfrom, SYS_recvfrom, 123456),
            (sys_accept, SYS_accept, 123),
            (sys_getpeername, SYS_getpeername, 123),
            (sys_getsockname, SYS_getsockname, 123),
            (sys_accept4, SYS_accept4, 1234),
            (sys_socket, SYS_socket, 123),
            (sys_connect, SYS_connect, 123),
            (sys_bind, SYS_bind, 123),
            (sys_setsockopt, SYS_setsockopt, 12345),
            (sys_listen, SYS_listen, 12),
            (sys_recvmmsg, SYS_recvmmsg, 12345),
            (sys_sendmmsg, SYS_sendmmsg, 1234),
            (sys_getsockopt, SYS_getsockopt, 12345),
            (sys_sendto, SYS_sendto, 123456),
            (sys_shutdown, SYS_shutdown, 12),
            (sys_socketpair, SYS_socketpair, 1234),
            (sys_ypconnect, SYS_ypconnect, 1),
        ];
        for (f, nr, want) in cases {
            let mut rval = [-7; 2];
            assert_eq!(f(&API, &a, &mut rval), Err(want), "SYS {nr}");
            assert_eq!(rval, [nr as Register, -7], "SYS {nr} wrote rval[0] only");
            let mut rval = [-7; 2];
            assert_eq!(syscall(&API, nr as Register, &a, &mut rval), Err(want));
            assert_eq!(rval[0], nr as Register, "SYS {nr} reached its socket entry");
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
        // pipe2's flags are the C's `int`: the register's high word never reaches the kernel.
        assert_eq!(
            syscall(
                &API,
                SYS_pipe2 as Register,
                &[5, 0x1_0000_0003, 9, 9, 9, 9],
                &mut rval
            ),
            Err(101 * 1000 + 5 + 3)
        );
        assert_eq!(
            syscall(&API, SYS_pipe2 as Register, &[5, -1, 9, 9, 9, 9], &mut rval),
            Err(101 * 1000 + 5 - 1)
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

    /// An argument is read as the C's `SCARG` reads its type: close(2)'s `int fd` and
    /// umask(2)'s `mode_t numask` are the register's low 32 bits, whatever its high word
    /// holds, the first with its sign, the second unsigned (umask's stub folds its `u32`
    /// through `i64` into the `i32` it answers, so `0xffff_ffff` reads back as -1).
    ///
    /// The high-word cases test the reading; they cannot tell `int` from `u_int`, nor can
    /// umask's `-1`, which the stub's fold answers as -1 either way. The family is enforced
    /// by the `KernelApi` entry's parameter type (`umask` takes a `u32`, `close` an `i32`: the
    /// other helper does not compile there), and `u_int(-1)` is checked directly below.
    #[test]
    fn the_arguments_are_read_as_scarg_reads_their_c_type() {
        let cases: [(NativeFn, i32, Register, i32); 4] = [
            (sys_close, SYS_close, 0x1_0000_0003, 3),
            (sys_close, SYS_close, -1, -1),
            (sys_umask, SYS_umask, 0x1_0000_0005, 5),
            (sys_umask, SYS_umask, -1, -1),
        ];
        for (f, nr, reg, want) in cases {
            let mut rval = [-7; 2];
            assert_eq!(
                f(&API, &[reg, 9, 9, 9, 9, 9], &mut rval),
                Err(want),
                "SYS {nr} with {reg:#x}"
            );
            assert_eq!(rval[0], nr as Register);
        }
        assert_eq!(
            u_int(-1),
            0xffff_ffff,
            "umask(2)'s -1 is the C's (mode_t)-1"
        );
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

    /// nfssvc(2)'s translation (N6.7b4): its number, the flag as an `int` and the argument's
    /// address, the trap's return registers.
    #[test]
    fn nfssvc_is_native_and_reaches_the_kernels_entry() {
        assert!(matches!(
            entry(&SYSENT, SYS_nfssvc as Register),
            Some(Sysent::Native(_))
        ));
        // The flag is read as the C's `int`: the register's upper half is not the flag's.
        let a: SysArgs = [(1 << 32) | 4, 2, 3, 0, 0, 0];
        let mut rval = [-7; 2];
        assert_eq!(sys_nfssvc(&API, &a, &mut rval), Err(42));
        assert_eq!(rval, [SYS_nfssvc as Register, -7]);
        let mut rval = [-7; 2];
        assert_eq!(
            syscall(&API, SYS_nfssvc as Register, &a, &mut rval),
            Err(42)
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
            NATIVE, 190,
            "93 through sysctl(2), 11 more with the time calls, 1 with sigreturn(2), 65 with the \
             VFS calls (N5.4), 19 with the socket calls (N6.2b), 1 with nfssvc(2) (N6.7b4)"
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
