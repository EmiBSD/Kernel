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
//! The process system calls' translations: `kernel::proc` (fork, exit, wait, the limits, the
//! ids and credentials, accounting, the TCB, the thread names, the routing table),
//! `kernel::sched` (futex(2)) and `kernel::sig` (sigreturn(2)).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, ptr, size_t, u_int};
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use sysdefs::proc::syscall::{
    SYS___get_tcb, SYS___set_tcb, SYS___tfork, SYS___threxit, SYS_acct, SYS_exit, SYS_fork,
    SYS_futex, SYS_getegid, SYS_geteuid, SYS_getgid, SYS_getgroups, SYS_getlogin_r, SYS_getpgid,
    SYS_getpgrp, SYS_getpid, SYS_getppid, SYS_getpriority, SYS_getresgid, SYS_getresuid,
    SYS_getrlimit, SYS_getrtable, SYS_getrusage, SYS_getsid, SYS_getthrid, SYS_getthrname,
    SYS_getuid, SYS_issetugid, SYS_setegid, SYS_seteuid, SYS_setgid, SYS_setgroups, SYS_setlogin,
    SYS_setpgid, SYS_setpriority, SYS_setregid, SYS_setresgid, SYS_setresuid, SYS_setreuid,
    SYS_setrlimit, SYS_setrtable, SYS_setsid, SYS_setthrname, SYS_setuid, SYS_sigreturn, SYS_vfork,
    SYS_wait4, SYS_waitid,
};

/// 103 = `sigreturn(struct sigcontext *sigcntxp)`: `EJUSTRETURN` once the context is restored.
pub(crate) fn sys_sigreturn(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::sigreturn(SYS_sigreturn as Register, ptr(args[0]), rval)
}

/// 83 = `futex(uint32_t *f, int op, int val, const struct timespec *timeout, uint32_t *g)`:
/// the number woken in `rval[0]`.
pub(crate) fn sys_futex(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::futex(
        SYS_futex as Register,
        ptr(args[0]),
        int(args[1]),
        int(args[2]),
        ptr(args[3]),
        ptr(args[4]),
        rval,
    )
}

/// `fork(void)`.
pub(crate) fn sys_fork(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::fork(SYS_fork as Register, rval)
}

/// `vfork(void)`.
pub(crate) fn sys_vfork(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::vfork(SYS_vfork as Register, rval)
}

/// `__tfork(const struct __tfork *param, size_t psize)`.
pub(crate) fn sys_tfork(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::tfork(SYS___tfork as Register, ptr(args[0]), size_t(args[1]), rval)
}

/// `exit(int rval)`.
pub(crate) fn sys_exit(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::exit(SYS_exit as Register, int(args[0]), rval)
}

/// `__threxit(pid_t *notdead)`.
pub(crate) fn sys_threxit(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::threxit(SYS___threxit as Register, ptr(args[0]), rval)
}

/// `wait4(pid_t pid, int *status, int options, struct rusage *rusage)`.
pub(crate) fn sys_wait4(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::wait4(
        SYS_wait4 as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        ptr(args[3]),
        rval,
    )
}

/// `waitid(int idtype, id_t id, siginfo_t *info, int options)`.
pub(crate) fn sys_waitid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::waitid(
        SYS_waitid as Register,
        int(args[0]),
        u_int(args[1]),
        ptr(args[2]),
        int(args[3]),
        rval,
    )
}

/// `getrusage(int who, struct rusage *rusage)`.
pub(crate) fn sys_getrusage(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getrusage(SYS_getrusage as Register, int(args[0]), ptr(args[1]), rval)
}

/// `getrlimit(int which, struct rlimit *rlp)`.
pub(crate) fn sys_getrlimit(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getrlimit(SYS_getrlimit as Register, int(args[0]), ptr(args[1]), rval)
}

/// `setrlimit(int which, const struct rlimit *rlp)`.
pub(crate) fn sys_setrlimit(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setrlimit(SYS_setrlimit as Register, int(args[0]), ptr(args[1]), rval)
}

/// `getpriority(int which, id_t who)`.
pub(crate) fn sys_getpriority(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getpriority(
        SYS_getpriority as Register,
        int(args[0]),
        u_int(args[1]),
        rval,
    )
}

/// `setpriority(int which, id_t who, int prio)`.
pub(crate) fn sys_setpriority(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setpriority(
        SYS_setpriority as Register,
        int(args[0]),
        u_int(args[1]),
        int(args[2]),
        rval,
    )
}

/// `acct(const char *path)`.
pub(crate) fn sys_acct(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::acct(SYS_acct as Register, ptr(args[0]), rval)
}

/// `getpid(void)`.
pub(crate) fn sys_getpid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getpid(SYS_getpid as Register, rval)
}

/// `getppid(void)`.
pub(crate) fn sys_getppid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getppid(SYS_getppid as Register, rval)
}

/// `getuid(void)`.
pub(crate) fn sys_getuid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getuid(SYS_getuid as Register, rval)
}

/// `geteuid(void)`.
pub(crate) fn sys_geteuid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::geteuid(SYS_geteuid as Register, rval)
}

/// `getgid(void)`.
pub(crate) fn sys_getgid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getgid(SYS_getgid as Register, rval)
}

/// `getegid(void)`.
pub(crate) fn sys_getegid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getegid(SYS_getegid as Register, rval)
}

/// `getgroups(int gidsetsize, gid_t *gidset)`.
pub(crate) fn sys_getgroups(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getgroups(SYS_getgroups as Register, int(args[0]), ptr(args[1]), rval)
}

/// `setgroups(int gidsetsize, const gid_t *gidset)`.
pub(crate) fn sys_setgroups(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setgroups(SYS_setgroups as Register, int(args[0]), ptr(args[1]), rval)
}

/// `getpgrp(void)`.
pub(crate) fn sys_getpgrp(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getpgrp(SYS_getpgrp as Register, rval)
}

/// `getpgid(pid_t pid)`.
pub(crate) fn sys_getpgid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getpgid(SYS_getpgid as Register, int(args[0]), rval)
}

/// `getsid(pid_t pid)`.
pub(crate) fn sys_getsid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getsid(SYS_getsid as Register, int(args[0]), rval)
}

/// `setpgid(pid_t pid, pid_t pgid)`.
pub(crate) fn sys_setpgid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setpgid(SYS_setpgid as Register, int(args[0]), int(args[1]), rval)
}

/// `setsid(void)`.
pub(crate) fn sys_setsid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setsid(SYS_setsid as Register, rval)
}

/// `setuid(uid_t uid)`.
pub(crate) fn sys_setuid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setuid(SYS_setuid as Register, u_int(args[0]), rval)
}

/// `seteuid(uid_t euid)`.
pub(crate) fn sys_seteuid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::seteuid(SYS_seteuid as Register, u_int(args[0]), rval)
}

/// `setgid(gid_t gid)`.
pub(crate) fn sys_setgid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setgid(SYS_setgid as Register, u_int(args[0]), rval)
}

/// `setegid(gid_t egid)`.
pub(crate) fn sys_setegid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setegid(SYS_setegid as Register, u_int(args[0]), rval)
}

/// `setreuid(uid_t ruid, uid_t euid)`.
pub(crate) fn sys_setreuid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setreuid(
        SYS_setreuid as Register,
        u_int(args[0]),
        u_int(args[1]),
        rval,
    )
}

/// `setregid(gid_t rgid, gid_t egid)`.
pub(crate) fn sys_setregid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setregid(
        SYS_setregid as Register,
        u_int(args[0]),
        u_int(args[1]),
        rval,
    )
}

/// `setresuid(uid_t ruid, uid_t euid, uid_t suid)`.
pub(crate) fn sys_setresuid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setresuid(
        SYS_setresuid as Register,
        u_int(args[0]),
        u_int(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// `setresgid(gid_t rgid, gid_t egid, gid_t sgid)`.
pub(crate) fn sys_setresgid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setresgid(
        SYS_setresgid as Register,
        u_int(args[0]),
        u_int(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// `getresuid(uid_t *ruid, uid_t *euid, uid_t *suid)`.
pub(crate) fn sys_getresuid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getresuid(
        SYS_getresuid as Register,
        ptr(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// `getresgid(gid_t *rgid, gid_t *egid, gid_t *sgid)`.
pub(crate) fn sys_getresgid(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getresgid(
        SYS_getresgid as Register,
        ptr(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// `issetugid(void)`.
pub(crate) fn sys_issetugid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::issetugid(SYS_issetugid as Register, rval)
}

/// `getlogin_r(char *namebuf, size_t namelen)`.
pub(crate) fn sys_getlogin_r(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getlogin_r(
        SYS_getlogin_r as Register,
        ptr(args[0]),
        size_t(args[1]),
        rval,
    )
}

/// `setlogin(const char *namebuf)`.
pub(crate) fn sys_setlogin(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setlogin(SYS_setlogin as Register, ptr(args[0]), rval)
}

/// `__set_tcb(void *tcb)`.
pub(crate) fn sys_set_tcb(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::set_tcb(SYS___set_tcb as Register, ptr(args[0]), rval)
}

/// `__get_tcb(void)`.
pub(crate) fn sys_get_tcb(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::get_tcb(SYS___get_tcb as Register, rval)
}

/// `getthrid(void)`.
pub(crate) fn sys_getthrid(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getthrid(SYS_getthrid as Register, rval)
}

/// `getthrname(pid_t tid, char *name, size_t len)`.
pub(crate) fn sys_getthrname(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getthrname(
        SYS_getthrname as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        rval,
    )
}

/// `setthrname(pid_t tid, const char *name)`.
pub(crate) fn sys_setthrname(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setthrname(SYS_setthrname as Register, int(args[0]), ptr(args[1]), rval)
}

/// `getrtable(void)`.
pub(crate) fn sys_getrtable(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::getrtable(SYS_getrtable as Register, rval)
}

/// `setrtable(int rtableid)`.
pub(crate) fn sys_setrtable(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::proc::setrtable(SYS_setrtable as Register, int(args[0]), rval)
}
/* </CODE> */
