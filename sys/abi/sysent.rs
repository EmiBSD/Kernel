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
//! `<sys/syscall.h>` (`sysdefs::proc::syscall`, the ABI edge), each `Native` (translated into a
//! `kernel` entry point) or `Legacy` (handed to the LZ path `bsd` registered, through
//! `kernel::api::syscall_legacy`), and the dispatcher the machine's handler
//! ([`crate::arch`], which [`crate::abi_init`] registers in `hal`) runs.
//!
//! N2.4: every entry was `Legacy`, so every system call behaves exactly as LZ's: the number
//! is not even read here but to pick the entry, and an out-of-range number goes to the
//! fallback too, which answers `ENOSYS` as the traps did. A subsystem that moves into
//! `kernel` flips its entries to `Native` and raises [`NATIVE`]: N2.6, pipe(2) and pipe2(2)
//! (`kernel::api::file::pipe2`, `sys/kernel/file/pipe.rs`). N3.5: the virtual memory system's fifteen
//! (`kernel::api::vm`, `sys/kernel/vm/mmap.rs` and `unix.rs`): written in S1, `Native` since
//! the flip (S2). N4.3: the descriptor tables' twenty (`kernel::api::file`,
//! `sys/kernel/file/descrip.rs`: `read(2)`, `write(2)`, `close(2)`, `dup(2)`, `fcntl(2)`,
//! `ioctl(2)`, ...), which write the return registers themselves. N4.4: the event system
//! calls' seven (`kernel::api::event`, `sys/kernel/file/event.rs` and `select.rs`: `kqueue(2)`,
//! `kqueue1(2)`, `kevent(2)`, `select(2)`, `pselect(2)`, `poll(2)`, `ppoll(2)`). N4.7a:
//! `sendsyslog(2)` (`kernel::api::base::sendsyslog`, `sys/kernel/base/log.rs`). N4.6c: futex(2)
//! (`kernel::api::proc::futex`, `sys/kernel/sched/futex.rs`). N4.6a: the process system calls'
//! forty-six (`kernel::api::proc`, `sys/kernel/proc/fork.rs`, `exit.rs`, `resource.rs`, `prot.rs`,
//! `acct.rs`). N4.6b: sysctl(2) (`kernel::api::base::sysctl`, `sys/kernel/base/sysctl.rs`). N4.8: the
//! ten time system calls and reboot(2) (`kernel::api::time` and `kernel::api::base::reboot`, `sys/kernel/time/mod.rs` and
//! `reboot.rs`). N4.8b: sigreturn(2) (`kernel::api::proc::sigreturn`, `sys/kernel/sig/mod.rs` over
//! `hal::cpu::sig`'s frame). N5.4: the VFS system calls' sixty-five (`kernel::api::fs`,
//! `sys/kernel/fs/syscalls.rs` and `getcwd.rs`: the sixty-four of `vfs_syscalls.c`, `open(2)`,
//! `stat(2)`, `mount(2)`, `unveil(2)`, ..., and `__getcwd(2)`), which write the return
//! registers themselves. N6.2b: the socket system calls' nineteen (`kernel::api::net`,
//! `sys/kernel/net/syscalls.rs`: `socket(2)`, `bind(2)`, `accept(2)`, `sendmsg(2)`,
//! `recvmmsg(2)`, ..., `ypconnect(2)`; `setrtable(2)` and `getrtable(2)` are `kernel::api::proc`'s).
//! N6.7b4: the NFS server's `nfssvc(2)` (`kernel::api::fs::nfssvc`, `sys/kernel/fs/nfs/nfssvc.rs`).
//!
//! Translation only (decision 21; `cargo xtask abi-check`, decision 25): no loop, a
//! `match` only on the number's conversion and on `Some`/`None`, one kernel call per
//! function, no state. Every argument is read out of its register as the C's `SCARG` reads
//! `syscalls.master`'s type, through the function of [`crate::scarg`] named after its family
//! (`int`, `u_int`, `ptr`, `size_t`, `off_t`, `u_long`; N5 close, the abi casts).
//!
//! Decision 52 (N7): the `kernel::api` functions named above were the members of
//! `hal::api::KernelApi`, a table the boot glue filled and handed to [`crate::abi_init`];
//! `abi` now depends on `kernel` and calls them directly, and its handler is registered in
//! `hal` with no table beside it.
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
use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult};

use sysdefs::proc::syscall::{
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
use sysdefs::proc::syscall::{
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
use sysdefs::proc::syscall::{
    SYS_accept, SYS_accept4, SYS_bind, SYS_connect, SYS_getpeername, SYS_getsockname,
    SYS_getsockopt, SYS_listen, SYS_recvfrom, SYS_recvmmsg, SYS_recvmsg, SYS_sendmmsg, SYS_sendmsg,
    SYS_sendto, SYS_setsockopt, SYS_shutdown, SYS_socket, SYS_socketpair, SYS_ypconnect,
};
use sysdefs::proc::syscall::{
    SYS_adjfreq, SYS_adjtime, SYS_clock_getres, SYS_clock_gettime, SYS_clock_settime,
    SYS_getitimer, SYS_gettimeofday, SYS_nanosleep, SYS_reboot, SYS_setitimer, SYS_settimeofday,
};

/// A `Native` entry: the translation of one system call into its `kernel` entry point.
pub type NativeFn = fn(&SysArgs, &mut Rval) -> SysResult;

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

/// The dispatcher over a table: a `Native` entry is called, a `Legacy` one or a number with
/// no entry goes to the kernel's `Legacy` path with the number unchanged. The machine's
/// handler ([`crate::arch`]) runs it over [`SYSENT`] with the number and arguments it read
/// from the trap's registers.
pub fn dispatch(table: &[Sysent], nr: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
    match entry(table, nr) {
        Some(Sysent::Native(f)) => f(args, rval),
        Some(Sysent::Legacy) | None => kernel::api::syscall_legacy(nr, args, rval),
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
    reason = "the test rows fold the number and the arguments into one i32"
)]
mod tests {
    use super::*;
    use crate::scarg::u_int;
    use kernel::syscall::{LegacySyscalls, install_legacy_syscalls};

    /// `ENOSYS`, the answer of a number outside the table.
    const ENOSYS: i32 = sysdefs::base::errno::Errno::ENOSYS.as_i32();

    /// The `sysent[]` rows of this test process (the kernel's `LegacySyscalls` bridge, which
    /// `bsd` registers in the image): every call, `Native` through `kernel::api` or `Legacy`,
    /// ends here. They write the number into `rval[0]` and the first argument into
    /// `rval[1]`, and answer `ENOSYS` outside the table, else the number times a million
    /// plus the six arguments as decimal digits (9 for an argument outside 0..=9).
    struct Rows;

    impl LegacySyscalls for Rows {
        fn syscall(&self, code: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
            rval[0] = code;
            rval[1] = args[0];
            if code <= 0 || code as usize >= SYS_MAXSYSCALL {
                return Err(ENOSYS);
            }
            let digits = args.iter().fold(0, |acc, &a| {
                acc * 10 + if (0..=9).contains(&a) { a as i32 } else { 9 }
            });
            Err(code as i32 * 1_000_000 + digits)
        }
    }

    static ROWS: Rows = Rows;

    /// [`dispatch`] over [`SYSENT`], what the machine's handler runs.
    fn syscall(nr: Register, args: &SysArgs, rval: &mut Rval) -> SysResult {
        dispatch(&SYSENT, nr, args, rval)
    }

    /// Registers [`ROWS`] (once per process; a second call changes nothing).
    fn rows() {
        let _ = install_legacy_syscalls(&ROWS);
    }

    /// What [`Rows`] answers for number `nr` given `1, 2, 3, 4, 5, 6` of which the call
    /// passes the first `k`.
    fn answer(nr: i32, k: u32) -> SysResult {
        let digits = 123_456 / 10i32.pow(6 - k) * 10i32.pow(6 - k);
        Err(nr * 1_000_000 + digits)
    }

    /// The process system calls N4.6a made `Native` (`kernel::api::proc`).
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

    /// The time system calls and reboot(2) N4.8 made `Native` (`kernel::api::time`, reboot(2) `kernel::api::base`).
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

    /// The event system calls N4.4 made `Native` (`kernel::api::event`).
    const EVENT_CALLS: [i32; 7] = [
        SYS_select,
        SYS_kevent,
        SYS_ppoll,
        SYS_pselect,
        SYS_poll,
        SYS_kqueue,
        SYS_kqueue1,
    ];

    /// The vm system calls N3.5's flip made `Native` (`kernel::api::vm`).
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

    /// The descriptor system calls N4.3 made `Native` (`kernel::api::file`).
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

    /// The calls that run their row over return registers of their own (`kernel::api`'s
    /// vm functions and pipe(2)): the trap's are left alone.
    fn own_rval(nr: i32) -> bool {
        nr == SYS_pipe || nr == SYS_pipe2 || VM_CALLS.contains(&nr)
    }

    /// The other `Native` calls, one each.
    const OTHER_CALLS: [i32; 7] = [
        SYS_pipe,
        SYS_pipe2,
        SYS_sendsyslog,
        SYS_futex,
        SYS_sysctl,
        SYS_sigreturn,
        SYS_nfssvc,
    ];

    /// Whether `nr` is one of the `Native` calls the lists above name.
    fn listed(nr: i32) -> bool {
        OTHER_CALLS.contains(&nr)
            || VM_CALLS.contains(&nr)
            || FD_CALLS.contains(&nr)
            || EVENT_CALLS.contains(&nr)
            || PROC_CALLS.contains(&nr)
            || TIME_CALLS.contains(&nr)
            || VFS_CALLS.contains(&nr)
            || SOCKET_CALLS.contains(&nr)
    }

    /// A `Native` entry of a test table.
    fn native(args: &SysArgs, rval: &mut Rval) -> SysResult {
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
        let src = include_str!("../lib/sysdefs/proc/syscall.rs");
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

    /// The `Native` entries are exactly the listed calls (190), and every other number goes
    /// to the kernel's `Legacy` path with its number and arguments unchanged.
    #[test]
    fn the_native_entries_are_the_listed_calls_and_the_rest_is_legacy() {
        rows();
        let mut native = 0;
        for nr in 1..SYS_MAXSYSCALL as i32 {
            let is_native = matches!(SYSENT[nr as usize], Sysent::Native(_));
            assert_eq!(is_native, listed(nr), "SYS {nr}");
            if is_native {
                native += 1;
                continue;
            }
            let mut rval = [-7; 2];
            let args: SysArgs = [1, 2, 3, 4, 5, 6];
            assert_eq!(syscall(nr as Register, &args, &mut rval), answer(nr, 6));
            assert_eq!(
                rval,
                [nr as Register, 1],
                "SYS {nr} reached the rows unchanged"
            );
        }
        assert_eq!(native, NATIVE);
    }

    /// Every `Native` translation reaches its `kernel::api` function, which runs its own
    /// number's row with the arguments in `<sys/syscallargs.h>`'s order: the first `k` as
    /// given, zeros after (`bsd`'s tests check `k` against `sysent[]`'s `sy_narg`). The trap's
    /// return registers are the row's, but for the calls that keep their own.
    #[test]
    fn every_native_entry_runs_its_own_row_with_its_arguments_in_order() {
        rows();
        let args: SysArgs = [1, 2, 3, 4, 5, 6];
        for nr in 1..SYS_MAXSYSCALL as i32 {
            if !matches!(SYSENT[nr as usize], Sysent::Native(_)) {
                continue;
            }
            let mut rval = [-7; 2];
            let r = syscall(nr as Register, &args, &mut rval);
            let Some(k) = (0..=6).find(|&k| r == answer(nr, k)) else {
                panic!("SYS {nr}: {r:?} is not its own row with a prefix of the arguments");
            };
            if own_rval(nr) {
                assert_eq!(rval, [-7, -7], "SYS {nr} left the trap's registers alone");
            } else {
                let first = if k == 0 { 0 } else { 1 };
                assert_eq!(
                    rval,
                    [nr as Register, first],
                    "SYS {nr} ran over the trap's registers"
                );
            }
        }
    }

    /// pipe(2) reaches `kernel::api::file::pipe2` with its own number and no flags.
    #[test]
    fn pipe_passes_no_flags() {
        rows();
        let mut rval = [-7; 2];
        let args: SysArgs = [1, 2, 3, 4, 5, 6];
        assert_eq!(
            syscall(SYS_pipe as Register, &args, &mut rval),
            answer(SYS_pipe, 1)
        );
        assert_eq!(
            syscall(SYS_pipe2 as Register, &args, &mut rval),
            answer(SYS_pipe2, 2)
        );
        assert_eq!(rval, [-7, -7]);
    }

    #[test]
    fn a_number_without_an_entry_goes_to_the_fallback() {
        rows();
        for nr in [-1, Register::MIN, SYS_MAXSYSCALL as Register, 100_000] {
            let mut rval = [-7; 2];
            assert_eq!(syscall(nr, &[0; 6], &mut rval), Err(ENOSYS));
            assert_eq!(rval[0], nr);
        }
    }

    #[test]
    fn a_native_entry_reaches_its_function() {
        rows();
        let table = [Sysent::Legacy, Sysent::Native(native), Sysent::Legacy];
        let mut rval = [0; 2];
        assert_eq!(dispatch(&table, 1, &[2, 3, 0, 0, 0, 0], &mut rval), Ok(()));
        assert_eq!(rval, [5, -1]);
        assert_eq!(
            dispatch(&table, 2, &[2, 3, 0, 0, 0, 0], &mut rval),
            Err(2_230_000),
            "a Legacy entry: the rows get the number and the arguments unchanged"
        );
        assert_eq!(rval[0], 2);
        assert_eq!(
            dispatch(&table, 3, &[0; 6], &mut rval),
            Err(3_000_000),
            "past this table: the Legacy path with the number unchanged"
        );
        assert_eq!(rval[0], 3);
    }

    /// The `int` and `u_int` families read the low 32 bits of their register, the first
    /// with its sign, the second unsigned, and the kernel's function receives that value:
    /// close(2)'s `int fd` and umask(2)'s `mode_t`, seen as the row's first argument.
    #[test]
    fn the_arguments_are_read_as_scarg_reads_their_c_type() {
        rows();
        let cases: [(i32, Register, Register); 4] = [
            (SYS_close, 0x1_0000_0003, 3),
            (SYS_close, -1, -1),
            (SYS_umask, 0x1_0000_0005, 5),
            (SYS_umask, -1, 0xffff_ffff),
        ];
        for (nr, reg, want) in cases {
            let mut rval = [-7; 2];
            let _ = syscall(nr as Register, &[reg, 9, 9, 9, 9, 9], &mut rval);
            assert_eq!(rval, [nr as Register, want], "SYS {nr} with {reg:#x}");
        }
        assert_eq!(
            u_int(-1),
            0xffff_ffff,
            "umask(2)'s -1 is the C's (mode_t)-1"
        );
    }
}
/* </TESTS> */
