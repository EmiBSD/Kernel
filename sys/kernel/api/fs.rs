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
//! The file system calls (`kernel::fs`: the sixty-four of `vfs_syscalls.c`, `__getcwd(2)`)
//! and the NFS server's `nfssvc(2)` (`kernel::fs::nfs`).
//!
//! One function per system call, `abi`'s door into the kernel (decision 52): each takes the
//! system call's number and its arguments as `<sys/syscallargs.h>` types them, and runs
//! the call's `sysent[]` row, still `bsd`'s, through `mi_syscall` (pledge, the pinned
//! syscalls, the kernel lock; [`super`]'s `LegacySyscalls` bridge), the row calling the
//! kernel's own function; a number that is not the function's own is `ENOSYS`. Moved from
//! `bsd`'s glue, which filled `hal::api::KernelApi` with them until decision 52.

use hal::syscall::{Register, Rval, SysResult};

use sysdefs::syscall::{
    SYS___getcwd, SYS___pledge_open, SYS___realpath, SYS_access, SYS_chdir, SYS_chflags,
    SYS_chflagsat, SYS_chmod, SYS_chown, SYS_chroot, SYS_faccessat, SYS_fchdir, SYS_fchflags,
    SYS_fchmod, SYS_fchmodat, SYS_fchown, SYS_fchownat, SYS_fhopen, SYS_fhstat, SYS_fhstatfs,
    SYS_fstatat, SYS_fstatfs, SYS_fsync, SYS_ftruncate, SYS_futimens, SYS_futimes, SYS_getdents,
    SYS_getfh, SYS_getfsstat, SYS_lchown, SYS_link, SYS_linkat, SYS_lseek, SYS_lstat, SYS_mkdir,
    SYS_mkdirat, SYS_mkfifo, SYS_mkfifoat, SYS_mknod, SYS_mknodat, SYS_mount, SYS_nfssvc, SYS_open,
    SYS_openat, SYS_pathconf, SYS_pathconfat, SYS_quotactl, SYS_readlink, SYS_readlinkat,
    SYS_rename, SYS_renameat, SYS_revoke, SYS_rmdir, SYS_stat, SYS_statfs, SYS_symlink,
    SYS_symlinkat, SYS_sync, SYS_truncate, SYS_umask, SYS_unlink, SYS_unlinkat, SYS_unmount,
    SYS_unveil, SYS_utimensat, SYS_utimes,
};

use super::native;

/// `open(2)`.
pub fn open(nr: Register, path: usize, flags: i32, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_open,
        [
            path as Register,
            flags as Register,
            mode as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `link(2)`.
pub fn link(nr: Register, path: usize, link: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_link,
        [path as Register, link as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `unlink(2)`.
pub fn unlink(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS_unlink, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `chdir(2)`.
pub fn chdir(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS_chdir, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `fchdir(2)`.
pub fn fchdir(nr: Register, fd: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_fchdir, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `mknod(2)`.
pub fn mknod(nr: Register, path: usize, mode: u32, dev: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_mknod,
        [path as Register, mode as Register, dev as Register, 0, 0, 0],
        rval,
    )
}

/// `chmod(2)`.
pub fn chmod(nr: Register, path: usize, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_chmod,
        [path as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `chown(2)`.
pub fn chown(nr: Register, path: usize, uid: u32, gid: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_chown,
        [path as Register, uid as Register, gid as Register, 0, 0, 0],
        rval,
    )
}

/// `mount(2)`.
pub fn mount(
    nr: Register,
    r#type: usize,
    path: usize,
    flags: i32,
    data: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_mount,
        [
            r#type as Register,
            path as Register,
            flags as Register,
            data as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `unmount(2)`.
pub fn unmount(nr: Register, path: usize, flags: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_unmount,
        [path as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `access(2)`.
pub fn access(nr: Register, path: usize, amode: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_access,
        [path as Register, amode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `chflags(2)`.
pub fn chflags(nr: Register, path: usize, flags: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_chflags,
        [path as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fchflags(2)`.
pub fn fchflags(nr: Register, fd: i32, flags: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fchflags,
        [fd as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `sync(2)`.
pub fn sync(nr: Register, rval: &mut Rval) -> SysResult {
    native(nr, SYS_sync, [0; 6], rval)
}

/// `stat(2)`.
pub fn stat(nr: Register, path: usize, ub: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_stat,
        [path as Register, ub as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `lstat(2)`.
pub fn lstat(nr: Register, path: usize, ub: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_lstat,
        [path as Register, ub as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fstatat(2)`.
pub fn fstatat(
    nr: Register,
    fd: i32,
    path: usize,
    buf: usize,
    flag: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_fstatat,
        [
            fd as Register,
            path as Register,
            buf as Register,
            flag as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `revoke(2)`.
pub fn revoke(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS_revoke, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `symlink(2)`.
pub fn symlink(nr: Register, path: usize, link: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_symlink,
        [path as Register, link as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `readlink(2)`.
pub fn readlink(nr: Register, path: usize, buf: usize, count: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_readlink,
        [
            path as Register,
            buf as Register,
            count as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `umask(2)`.
pub fn umask(nr: Register, newmask: u32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_umask, [newmask as Register, 0, 0, 0, 0, 0], rval)
}

/// `chroot(2)`.
pub fn chroot(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS_chroot, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `getfsstat(2)`.
pub fn getfsstat(
    nr: Register,
    buf: usize,
    bufsize: usize,
    flags: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_getfsstat,
        [
            buf as Register,
            bufsize as Register,
            flags as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `statfs(2)`.
pub fn statfs(nr: Register, path: usize, buf: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_statfs,
        [path as Register, buf as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fstatfs(2)`.
pub fn fstatfs(nr: Register, fd: i32, buf: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fstatfs,
        [fd as Register, buf as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fhstatfs(2)`.
pub fn fhstatfs(nr: Register, fhp: usize, buf: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fhstatfs,
        [fhp as Register, buf as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `utimes(2)`.
pub fn utimes(nr: Register, path: usize, tptr: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_utimes,
        [path as Register, tptr as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `futimes(2)`.
pub fn futimes(nr: Register, fd: i32, tptr: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_futimes,
        [fd as Register, tptr as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `utimensat(2)`.
pub fn utimensat(
    nr: Register,
    fd: i32,
    path: usize,
    times: usize,
    flag: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_utimensat,
        [
            fd as Register,
            path as Register,
            times as Register,
            flag as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `futimens(2)`.
pub fn futimens(nr: Register, fd: i32, times: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_futimens,
        [fd as Register, times as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fsync(2)`.
pub fn fsync(nr: Register, fd: i32, rval: &mut Rval) -> SysResult {
    native(nr, SYS_fsync, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `getdents(2)`.
pub fn getdents(nr: Register, fd: i32, buf: usize, buflen: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getdents,
        [fd as Register, buf as Register, buflen as Register, 0, 0, 0],
        rval,
    )
}

/// `chflagsat(2)`.
pub fn chflagsat(
    nr: Register,
    fd: i32,
    path: usize,
    flags: u32,
    atflags: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_chflagsat,
        [
            fd as Register,
            path as Register,
            flags as Register,
            atflags as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `unveil(2)`.
pub fn unveil(nr: Register, path: usize, permissions: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_unveil,
        [path as Register, permissions as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `__realpath(2)`.
pub fn realpath(nr: Register, pathname: usize, resolved: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS___realpath,
        [pathname as Register, resolved as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fchown(2)`.
pub fn fchown(nr: Register, fd: i32, uid: u32, gid: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fchown,
        [fd as Register, uid as Register, gid as Register, 0, 0, 0],
        rval,
    )
}

/// `fchmod(2)`.
pub fn fchmod(nr: Register, fd: i32, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fchmod,
        [fd as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `__pledge_open(2)`.
pub fn pledge_open(nr: Register, path: usize, flags: i32, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS___pledge_open,
        [
            path as Register,
            flags as Register,
            mode as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `rename(2)`.
pub fn rename(nr: Register, from: usize, to: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_rename,
        [from as Register, to as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `mkfifo(2)`.
pub fn mkfifo(nr: Register, path: usize, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_mkfifo,
        [path as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `mkdir(2)`.
pub fn mkdir(nr: Register, path: usize, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_mkdir,
        [path as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `rmdir(2)`.
pub fn rmdir(nr: Register, path: usize, rval: &mut Rval) -> SysResult {
    native(nr, SYS_rmdir, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `quotactl(2)`.
pub fn quotactl(
    nr: Register,
    path: usize,
    cmd: i32,
    uid: i32,
    arg: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_quotactl,
        [
            path as Register,
            cmd as Register,
            uid as Register,
            arg as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `getfh(2)`.
pub fn getfh(nr: Register, fname: usize, fhp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_getfh,
        [fname as Register, fhp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `lseek(2)`.
pub fn lseek(nr: Register, fd: i32, offset: i64, whence: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_lseek,
        [
            fd as Register,
            offset as Register,
            whence as Register,
            0,
            0,
            0,
        ],
        rval,
    )
}

/// `truncate(2)`.
pub fn truncate(nr: Register, path: usize, length: i64, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_truncate,
        [path as Register, length as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `ftruncate(2)`.
pub fn ftruncate(nr: Register, fd: i32, length: i64, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_ftruncate,
        [fd as Register, length as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `pathconfat(2)`.
pub fn pathconfat(
    nr: Register,
    fd: i32,
    path: usize,
    name: i32,
    flag: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_pathconfat,
        [
            fd as Register,
            path as Register,
            name as Register,
            flag as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `pathconf(2)`.
pub fn pathconf(nr: Register, path: usize, name: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_pathconf,
        [path as Register, name as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `lchown(2)`.
pub fn lchown(nr: Register, path: usize, uid: u32, gid: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_lchown,
        [path as Register, uid as Register, gid as Register, 0, 0, 0],
        rval,
    )
}

/// `fhopen(2)`.
pub fn fhopen(nr: Register, fhp: usize, flags: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fhopen,
        [fhp as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `fhstat(2)`.
pub fn fhstat(nr: Register, fhp: usize, sb: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_fhstat,
        [fhp as Register, sb as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `__getcwd(2)`.
pub fn getcwd(nr: Register, buf: usize, len: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS___getcwd,
        [buf as Register, len as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `faccessat(2)`.
pub fn faccessat(
    nr: Register,
    fd: i32,
    path: usize,
    amode: i32,
    flag: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_faccessat,
        [
            fd as Register,
            path as Register,
            amode as Register,
            flag as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `fchmodat(2)`.
pub fn fchmodat(
    nr: Register,
    fd: i32,
    path: usize,
    mode: u32,
    flag: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_fchmodat,
        [
            fd as Register,
            path as Register,
            mode as Register,
            flag as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `fchownat(2)`.
pub fn fchownat(
    nr: Register,
    fd: i32,
    path: usize,
    uid: u32,
    gid: u32,
    flag: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_fchownat,
        [
            fd as Register,
            path as Register,
            uid as Register,
            gid as Register,
            flag as Register,
            0,
        ],
        rval,
    )
}

/// `linkat(2)`.
pub fn linkat(
    nr: Register,
    fd1: i32,
    path1: usize,
    fd2: i32,
    path2: usize,
    flag: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_linkat,
        [
            fd1 as Register,
            path1 as Register,
            fd2 as Register,
            path2 as Register,
            flag as Register,
            0,
        ],
        rval,
    )
}

/// `mkdirat(2)`.
pub fn mkdirat(nr: Register, fd: i32, path: usize, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_mkdirat,
        [fd as Register, path as Register, mode as Register, 0, 0, 0],
        rval,
    )
}

/// `mkfifoat(2)`.
pub fn mkfifoat(nr: Register, fd: i32, path: usize, mode: u32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_mkfifoat,
        [fd as Register, path as Register, mode as Register, 0, 0, 0],
        rval,
    )
}

/// `mknodat(2)`.
pub fn mknodat(
    nr: Register,
    fd: i32,
    path: usize,
    mode: u32,
    dev: i32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_mknodat,
        [
            fd as Register,
            path as Register,
            mode as Register,
            dev as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `openat(2)`.
pub fn openat(
    nr: Register,
    fd: i32,
    path: usize,
    flags: i32,
    mode: u32,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_openat,
        [
            fd as Register,
            path as Register,
            flags as Register,
            mode as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `readlinkat(2)`.
pub fn readlinkat(
    nr: Register,
    fd: i32,
    path: usize,
    buf: usize,
    count: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_readlinkat,
        [
            fd as Register,
            path as Register,
            buf as Register,
            count as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `renameat(2)`.
pub fn renameat(
    nr: Register,
    fromfd: i32,
    from: usize,
    tofd: i32,
    to: usize,
    rval: &mut Rval,
) -> SysResult {
    native(
        nr,
        SYS_renameat,
        [
            fromfd as Register,
            from as Register,
            tofd as Register,
            to as Register,
            0,
            0,
        ],
        rval,
    )
}

/// `symlinkat(2)`.
pub fn symlinkat(nr: Register, path: usize, fd: i32, link: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_symlinkat,
        [path as Register, fd as Register, link as Register, 0, 0, 0],
        rval,
    )
}

/// `unlinkat(2)`.
pub fn unlinkat(nr: Register, fd: i32, path: usize, flag: i32, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_unlinkat,
        [fd as Register, path as Register, flag as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::nfssvc`, the `Native` nfssvc(2): the thread on this CPU runs `mi_syscall` on
/// `sysent[155]` (the prologue still `bsd`'s: pledge, the pinned syscalls, the kernel lock),
/// whose row is [`sys_nfssvc`]; another number is `ENOSYS`.
pub fn nfssvc(nr: Register, flag: i32, argp: usize, rval: &mut Rval) -> SysResult {
    native(
        nr,
        SYS_nfssvc,
        [flag as Register, argp as Register, 0, 0, 0, 0],
        rval,
    )
}
/* </CODE> */
