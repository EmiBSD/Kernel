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
//! The process system calls (`kernel::proc`: fork, exit, wait, the limits, the ids and
//! credentials, accounting, the TCB, the thread names, the routing table), futex(2)
//! (`kernel::sched::futex`) and sigreturn(2) (`kernel::sig`).
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::{Register, Rval, SysResult};

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

use super::native;

/// `fork(void)`.
pub fn fork(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_fork, [0, 0, 0, 0, 0, 0], rval)
}

/// `vfork(void)`.
pub fn vfork(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_vfork, [0, 0, 0, 0, 0, 0], rval)
}

/// `__tfork(const struct __tfork *param, size_t psize)`.
pub fn tfork(nr: Register, param: usize, psize: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS___tfork,
        [param as Register, psize as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `exit(int rval)`.
pub fn exit(nr: Register, status: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_exit, [status as Register, 0, 0, 0, 0, 0], rval)
}

/// `__threxit(pid_t *notdead)`.
pub fn threxit(nr: Register, notdead: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS___threxit,
        [notdead as Register, 0, 0, 0, 0, 0],
        rval,
    )
}

/// `wait4(pid_t pid, int *status, int options, struct rusage *rusage)`.
pub fn wait4(
    nr: Register,
    pid: i32,
    status: usize,
    options: i32,
    rusage: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_wait4,
        [
            pid as Register,
            status as Register,
            options as Register,
            rusage as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `waitid(int idtype, id_t id, siginfo_t *info, int options)`.
pub fn waitid(
    nr: Register,
    idtype: i32,
    id: u32,
    info: usize,
    options: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_waitid,
        [
            idtype as Register,
            id as Register,
            info as Register,
            options as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `getrusage(int who, struct rusage *rusage)`.
pub fn getrusage(nr: Register, who: i32, rusage: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getrusage,
        [who as Register, rusage as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `getrlimit(int which, struct rlimit *rlp)`.
pub fn getrlimit(nr: Register, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getrlimit,
        [which as Register, rlp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setrlimit(int which, const struct rlimit *rlp)`.
pub fn setrlimit(nr: Register, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setrlimit,
        [which as Register, rlp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `getpriority(int which, id_t who)`.
pub fn getpriority(nr: Register, which: i32, who: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getpriority,
        [which as Register, who as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setpriority(int which, id_t who, int prio)`.
pub fn setpriority(nr: Register, which: i32, who: u32, prio: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setpriority,
        [
            which as Register,
            who as Register,
            prio as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `acct(const char *path)`.
pub fn acct(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS_acct, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `getpid(void)`.
pub fn getpid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getpid, [0, 0, 0, 0, 0, 0], rval)
}

/// `getppid(void)`.
pub fn getppid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getppid, [0, 0, 0, 0, 0, 0], rval)
}

/// `getuid(void)`.
pub fn getuid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getuid, [0, 0, 0, 0, 0, 0], rval)
}

/// `geteuid(void)`.
pub fn geteuid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_geteuid, [0, 0, 0, 0, 0, 0], rval)
}

/// `getgid(void)`.
pub fn getgid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getgid, [0, 0, 0, 0, 0, 0], rval)
}

/// `getegid(void)`.
pub fn getegid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getegid, [0, 0, 0, 0, 0, 0], rval)
}

/// `getgroups(int gidsetsize, gid_t *gidset)`.
pub fn getgroups(nr: Register, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getgroups,
        [gidsetsize as Register, gidset as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setgroups(int gidsetsize, const gid_t *gidset)`.
pub fn setgroups(nr: Register, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setgroups,
        [gidsetsize as Register, gidset as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `getpgrp(void)`.
pub fn getpgrp(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getpgrp, [0, 0, 0, 0, 0, 0], rval)
}

/// `getpgid(pid_t pid)`.
pub fn getpgid(nr: Register, pid: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getpgid, [pid as Register, 0, 0, 0, 0, 0], rval)
}

/// `getsid(pid_t pid)`.
pub fn getsid(nr: Register, pid: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getsid, [pid as Register, 0, 0, 0, 0, 0], rval)
}

/// `setpgid(pid_t pid, pid_t pgid)`.
pub fn setpgid(nr: Register, pid: i32, pgid: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setpgid,
        [pid as Register, pgid as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setsid(void)`.
pub fn setsid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_setsid, [0, 0, 0, 0, 0, 0], rval)
}

/// `setuid(uid_t uid)`.
pub fn setuid(nr: Register, uid: u32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_setuid, [uid as Register, 0, 0, 0, 0, 0], rval)
}

/// `seteuid(uid_t euid)`.
pub fn seteuid(nr: Register, euid: u32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_seteuid, [euid as Register, 0, 0, 0, 0, 0], rval)
}

/// `setgid(gid_t gid)`.
pub fn setgid(nr: Register, gid: u32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_setgid, [gid as Register, 0, 0, 0, 0, 0], rval)
}

/// `setegid(gid_t egid)`.
pub fn setegid(nr: Register, egid: u32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_setegid, [egid as Register, 0, 0, 0, 0, 0], rval)
}

/// `setreuid(uid_t ruid, uid_t euid)`.
pub fn setreuid(nr: Register, ruid: u32, euid: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setreuid,
        [ruid as Register, euid as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setregid(gid_t rgid, gid_t egid)`.
pub fn setregid(nr: Register, rgid: u32, egid: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setregid,
        [rgid as Register, egid as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setresuid(uid_t ruid, uid_t euid, uid_t suid)`.
pub fn setresuid(nr: Register, ruid: u32, euid: u32, suid: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setresuid,
        [
            ruid as Register,
            euid as Register,
            suid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `setresgid(gid_t rgid, gid_t egid, gid_t sgid)`.
pub fn setresgid(nr: Register, rgid: u32, egid: u32, sgid: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setresgid,
        [
            rgid as Register,
            egid as Register,
            sgid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `getresuid(uid_t *ruid, uid_t *euid, uid_t *suid)`.
pub fn getresuid(
    nr: Register,
    ruid: usize,
    euid: usize,
    suid: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_getresuid,
        [
            ruid as Register,
            euid as Register,
            suid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `getresgid(gid_t *rgid, gid_t *egid, gid_t *sgid)`.
pub fn getresgid(
    nr: Register,
    rgid: usize,
    egid: usize,
    sgid: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_getresgid,
        [
            rgid as Register,
            egid as Register,
            sgid as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `issetugid(void)`.
pub fn issetugid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_issetugid, [0, 0, 0, 0, 0, 0], rval)
}

/// `getlogin_r(char *namebuf, size_t namelen)`.
pub fn getlogin_r(nr: Register, namebuf: usize, namelen: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getlogin_r,
        [namebuf as Register, namelen as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `setlogin(const char *namebuf)`.
pub fn setlogin(nr: Register, namebuf: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS_setlogin, [namebuf as Register, 0, 0, 0, 0, 0], rval)
}

/// `__set_tcb(void *tcb)`.
pub fn set_tcb(nr: Register, tcb: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS___set_tcb, [tcb as Register, 0, 0, 0, 0, 0], rval)
}

/// `__get_tcb(void)`.
pub fn get_tcb(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS___get_tcb, [0, 0, 0, 0, 0, 0], rval)
}

/// `getthrid(void)`.
pub fn getthrid(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getthrid, [0, 0, 0, 0, 0, 0], rval)
}

/// `getthrname(pid_t tid, char *name, size_t len)`.
pub fn getthrname(nr: Register, tid: i32, name: usize, len: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getthrname,
        [tid as Register, name as Register, len as Register, 0, 0, 0],
        rval,
    )
}

/// `setthrname(pid_t tid, const char *name)`.
pub fn setthrname(nr: Register, tid: i32, name: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setthrname,
        [tid as Register, name as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `getrtable(void)`.
pub fn getrtable(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_getrtable, [0, 0, 0, 0, 0, 0], rval)
}

/// `setrtable(int rtableid)`.
pub fn setrtable(nr: Register, rtableid: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_setrtable,
        [rtableid as Register, 0, 0, 0, 0, 0],
        rval,
    )
}

/// [`sys_futex`]'s row through the system call prologue.
pub fn futex(
    nr: Register,
    uaddr: usize,
    op: i32,
    val: i32,
    timeout: usize,
    g: usize,
    rval: &mut Rval,
) -> SysResult {
    let args = [
        uaddr as Register,
        op as Register,
        val as Register,
        timeout as Register,
        g as Register,
        0,
    ];
    native(nr, SYS_futex, args, rval)
}

/// `KernelApi::sigreturn` (N4.8b): sigreturn(2) through the system call prologue and its
/// `sysent[]` row ([`sys_sigreturn`], the kernel's on arm64).
pub fn sigreturn(nr: Register, sigcntxp: usize, rval: &mut Rval) -> SysResult {
    let args = [sigcntxp as Register, 0, 0, 0, 0, 0];
    native(nr, SYS_sigreturn, args, rval)
}
/* </CODE> */
