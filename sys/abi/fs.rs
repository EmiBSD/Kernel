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
//! The file system calls' translations (`kernel::fs`: the sixty-four of `vfs_syscalls.c`,
//! `__getcwd(2)` and the NFS server's `nfssvc(2)`).
//!
//! Translation only, as [`crate::sysent`] says: each function reads its system call's
//! arguments through [`crate::scarg`] and makes one kernel call; the table that names it
//! is [`crate::sysent::SYSENT`].

use crate::scarg::{int, off_t, ptr, size_t, u_int};
use hal::syscall::{Register, Rval, SysArgs, SysResult};

use sysdefs::proc::syscall::{
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

/// 5 = `int open(const char *path, int flags, mode_t mode)`.
pub(crate) fn sys_open(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::open(
        SYS_open as Register,
        ptr(args[0]),
        int(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 9 = `int link(const char *path, const char *link)`.
pub(crate) fn sys_link(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::link(SYS_link as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 10 = `int unlink(const char *path)`.
pub(crate) fn sys_unlink(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::unlink(SYS_unlink as Register, ptr(args[0]), rval)
}

/// 12 = `int chdir(const char *path)`.
pub(crate) fn sys_chdir(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::chdir(SYS_chdir as Register, ptr(args[0]), rval)
}

/// 13 = `int fchdir(int fd)`.
pub(crate) fn sys_fchdir(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fchdir(SYS_fchdir as Register, int(args[0]), rval)
}

/// 14 = `int mknod(const char *path, mode_t mode, dev_t dev)`.
pub(crate) fn sys_mknod(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::mknod(
        SYS_mknod as Register,
        ptr(args[0]),
        u_int(args[1]),
        int(args[2]),
        rval,
    )
}

/// 15 = `int chmod(const char *path, mode_t mode)`.
pub(crate) fn sys_chmod(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::chmod(SYS_chmod as Register, ptr(args[0]), u_int(args[1]), rval)
}

/// 16 = `int chown(const char *path, uid_t uid, gid_t gid)`.
pub(crate) fn sys_chown(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::chown(
        SYS_chown as Register,
        ptr(args[0]),
        u_int(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 21 = `int mount(const char *type, const char *path, int flags, void *data)`.
pub(crate) fn sys_mount(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::mount(
        SYS_mount as Register,
        ptr(args[0]),
        ptr(args[1]),
        int(args[2]),
        ptr(args[3]),
        rval,
    )
}

/// 22 = `int unmount(const char *path, int flags)`.
pub(crate) fn sys_unmount(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::unmount(SYS_unmount as Register, ptr(args[0]), int(args[1]), rval)
}

/// 33 = `int access(const char *path, int amode)`.
pub(crate) fn sys_access(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::access(SYS_access as Register, ptr(args[0]), int(args[1]), rval)
}

/// 34 = `int chflags(const char *path, u_int flags)`.
pub(crate) fn sys_chflags(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::chflags(SYS_chflags as Register, ptr(args[0]), u_int(args[1]), rval)
}

/// 35 = `int fchflags(int fd, u_int flags)`.
pub(crate) fn sys_fchflags(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fchflags(SYS_fchflags as Register, int(args[0]), u_int(args[1]), rval)
}

/// 36 = `void sync(void)`.
pub(crate) fn sys_sync(_args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::sync(SYS_sync as Register, rval)
}

/// 38 = `int stat(const char *path, struct stat *ub)`.
pub(crate) fn sys_stat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::stat(SYS_stat as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 40 = `int lstat(const char *path, struct stat *ub)`.
pub(crate) fn sys_lstat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::lstat(SYS_lstat as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 42 = `int fstatat(int fd, const char *path, struct stat *buf, int flag)`.
pub(crate) fn sys_fstatat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fstatat(
        SYS_fstatat as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        int(args[3]),
        rval,
    )
}

/// 56 = `int revoke(const char *path)`.
pub(crate) fn sys_revoke(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::revoke(SYS_revoke as Register, ptr(args[0]), rval)
}

/// 57 = `int symlink(const char *path, const char *link)`.
pub(crate) fn sys_symlink(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::symlink(SYS_symlink as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 58 = `ssize_t readlink(const char *path, char *buf, size_t count)`.
pub(crate) fn sys_readlink(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::readlink(
        SYS_readlink as Register,
        ptr(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        rval,
    )
}

/// 60 = `mode_t umask(mode_t newmask)`.
pub(crate) fn sys_umask(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::umask(SYS_umask as Register, u_int(args[0]), rval)
}

/// 61 = `int chroot(const char *path)`.
pub(crate) fn sys_chroot(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::chroot(SYS_chroot as Register, ptr(args[0]), rval)
}

/// 62 = `int getfsstat(struct statfs *buf, size_t bufsize, int flags)`.
pub(crate) fn sys_getfsstat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::getfsstat(
        SYS_getfsstat as Register,
        ptr(args[0]),
        size_t(args[1]),
        int(args[2]),
        rval,
    )
}

/// 63 = `int statfs(const char *path, struct statfs *buf)`.
pub(crate) fn sys_statfs(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::statfs(SYS_statfs as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 64 = `int fstatfs(int fd, struct statfs *buf)`.
pub(crate) fn sys_fstatfs(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fstatfs(SYS_fstatfs as Register, int(args[0]), ptr(args[1]), rval)
}

/// 65 = `int fhstatfs(const fhandle_t *fhp, struct statfs *buf)`.
pub(crate) fn sys_fhstatfs(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fhstatfs(SYS_fhstatfs as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 76 = `int utimes(const char *path, const struct timeval *tptr)`.
pub(crate) fn sys_utimes(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::utimes(SYS_utimes as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 77 = `int futimes(int fd, const struct timeval *tptr)`.
pub(crate) fn sys_futimes(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::futimes(SYS_futimes as Register, int(args[0]), ptr(args[1]), rval)
}

/// 84 = `int utimensat(int fd, const char *path, const struct timespec *times, int flag)`.
pub(crate) fn sys_utimensat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::utimensat(
        SYS_utimensat as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        int(args[3]),
        rval,
    )
}

/// 85 = `int futimens(int fd, const struct timespec *times)`.
pub(crate) fn sys_futimens(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::futimens(SYS_futimens as Register, int(args[0]), ptr(args[1]), rval)
}

/// 95 = `int fsync(int fd)`.
pub(crate) fn sys_fsync(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fsync(SYS_fsync as Register, int(args[0]), rval)
}

/// 99 = `int getdents(int fd, void *buf, size_t buflen)`.
pub(crate) fn sys_getdents(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::getdents(
        SYS_getdents as Register,
        int(args[0]),
        ptr(args[1]),
        size_t(args[2]),
        rval,
    )
}

/// 107 = `int chflagsat(int fd, const char *path, u_int flags, int atflags)`.
pub(crate) fn sys_chflagsat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::chflagsat(
        SYS_chflagsat as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        int(args[3]),
        rval,
    )
}

/// 114 = `int unveil(const char *path, const char *permissions)`.
pub(crate) fn sys_unveil(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::unveil(SYS_unveil as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 115 = `int __realpath(const char *pathname, char *resolved)`.
pub(crate) fn sys_realpath(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::realpath(SYS___realpath as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 123 = `int fchown(int fd, uid_t uid, gid_t gid)`.
pub(crate) fn sys_fchown(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fchown(
        SYS_fchown as Register,
        int(args[0]),
        u_int(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 124 = `int fchmod(int fd, mode_t mode)`.
pub(crate) fn sys_fchmod(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fchmod(SYS_fchmod as Register, int(args[0]), u_int(args[1]), rval)
}

/// 125 = `int __pledge_open(const char *path, int flags, mode_t mode)`.
pub(crate) fn sys_pledge_open(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::pledge_open(
        SYS___pledge_open as Register,
        ptr(args[0]),
        int(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 128 = `int rename(const char *from, const char *to)`.
pub(crate) fn sys_rename(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::rename(SYS_rename as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 132 = `int mkfifo(const char *path, mode_t mode)`.
pub(crate) fn sys_mkfifo(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::mkfifo(SYS_mkfifo as Register, ptr(args[0]), u_int(args[1]), rval)
}

/// 136 = `int mkdir(const char *path, mode_t mode)`.
pub(crate) fn sys_mkdir(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::mkdir(SYS_mkdir as Register, ptr(args[0]), u_int(args[1]), rval)
}

/// 137 = `int rmdir(const char *path)`.
pub(crate) fn sys_rmdir(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::rmdir(SYS_rmdir as Register, ptr(args[0]), rval)
}

/// 148 = `int quotactl(const char *path, int cmd, int uid, char *arg)`.
pub(crate) fn sys_quotactl(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::quotactl(
        SYS_quotactl as Register,
        ptr(args[0]),
        int(args[1]),
        int(args[2]),
        ptr(args[3]),
        rval,
    )
}

/// 161 = `int getfh(const char *fname, fhandle_t *fhp)`.
pub(crate) fn sys_getfh(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::getfh(SYS_getfh as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 166 = `off_t lseek(int fd, off_t offset, int whence)`.
pub(crate) fn sys_lseek(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::lseek(
        SYS_lseek as Register,
        int(args[0]),
        off_t(args[1]),
        int(args[2]),
        rval,
    )
}

/// 167 = `int truncate(const char *path, off_t length)`.
pub(crate) fn sys_truncate(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::truncate(SYS_truncate as Register, ptr(args[0]), off_t(args[1]), rval)
}

/// 168 = `int ftruncate(int fd, off_t length)`.
pub(crate) fn sys_ftruncate(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::ftruncate(
        SYS_ftruncate as Register,
        int(args[0]),
        off_t(args[1]),
        rval,
    )
}

/// 190 = `long pathconfat(int fd, const char *path, int name, int flag)`.
pub(crate) fn sys_pathconfat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::pathconfat(
        SYS_pathconfat as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        int(args[3]),
        rval,
    )
}

/// 191 = `long pathconf(const char *path, int name)`.
pub(crate) fn sys_pathconf(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::pathconf(SYS_pathconf as Register, ptr(args[0]), int(args[1]), rval)
}

/// 254 = `int lchown(const char *path, uid_t uid, gid_t gid)`.
pub(crate) fn sys_lchown(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::lchown(
        SYS_lchown as Register,
        ptr(args[0]),
        u_int(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 264 = `int fhopen(const fhandle_t *fhp, int flags)`.
pub(crate) fn sys_fhopen(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fhopen(SYS_fhopen as Register, ptr(args[0]), int(args[1]), rval)
}

/// 294 = `int fhstat(const fhandle_t *fhp, struct stat *sb)`.
pub(crate) fn sys_fhstat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fhstat(SYS_fhstat as Register, ptr(args[0]), ptr(args[1]), rval)
}

/// 304 = `int __getcwd(char *buf, size_t len)`.
pub(crate) fn sys_getcwd(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::getcwd(
        SYS___getcwd as Register,
        ptr(args[0]),
        size_t(args[1]),
        rval,
    )
}

/// 313 = `int faccessat(int fd, const char *path, int amode, int flag)`.
pub(crate) fn sys_faccessat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::faccessat(
        SYS_faccessat as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        int(args[3]),
        rval,
    )
}

/// 314 = `int fchmodat(int fd, const char *path, mode_t mode, int flag)`.
pub(crate) fn sys_fchmodat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fchmodat(
        SYS_fchmodat as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        int(args[3]),
        rval,
    )
}

/// 315 = `int fchownat(int fd, const char *path, uid_t uid, gid_t gid, int flag)`.
pub(crate) fn sys_fchownat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::fchownat(
        SYS_fchownat as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        u_int(args[3]),
        int(args[4]),
        rval,
    )
}

/// 317 = `int linkat(int fd1, const char *path1, int fd2, const char *path2, int flag)`.
pub(crate) fn sys_linkat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::linkat(
        SYS_linkat as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        ptr(args[3]),
        int(args[4]),
        rval,
    )
}

/// 318 = `int mkdirat(int fd, const char *path, mode_t mode)`.
pub(crate) fn sys_mkdirat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::mkdirat(
        SYS_mkdirat as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 319 = `int mkfifoat(int fd, const char *path, mode_t mode)`.
pub(crate) fn sys_mkfifoat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::mkfifoat(
        SYS_mkfifoat as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        rval,
    )
}

/// 320 = `int mknodat(int fd, const char *path, mode_t mode, dev_t dev)`.
pub(crate) fn sys_mknodat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::mknodat(
        SYS_mknodat as Register,
        int(args[0]),
        ptr(args[1]),
        u_int(args[2]),
        int(args[3]),
        rval,
    )
}

/// 321 = `int openat(int fd, const char *path, int flags, mode_t mode)`.
pub(crate) fn sys_openat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::openat(
        SYS_openat as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        u_int(args[3]),
        rval,
    )
}

/// 322 = `ssize_t readlinkat(int fd, const char *path, char *buf, size_t count)`.
pub(crate) fn sys_readlinkat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::readlinkat(
        SYS_readlinkat as Register,
        int(args[0]),
        ptr(args[1]),
        ptr(args[2]),
        size_t(args[3]),
        rval,
    )
}

/// 323 = `int renameat(int fromfd, const char *from, int tofd, const char *to)`.
pub(crate) fn sys_renameat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::renameat(
        SYS_renameat as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        ptr(args[3]),
        rval,
    )
}

/// 324 = `int symlinkat(const char *path, int fd, const char *link)`.
pub(crate) fn sys_symlinkat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::symlinkat(
        SYS_symlinkat as Register,
        ptr(args[0]),
        int(args[1]),
        ptr(args[2]),
        rval,
    )
}

/// 325 = `int unlinkat(int fd, const char *path, int flag)`.
pub(crate) fn sys_unlinkat(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::unlinkat(
        SYS_unlinkat as Register,
        int(args[0]),
        ptr(args[1]),
        int(args[2]),
        rval,
    )
}

/// 155 = `int nfssvc(int flag, void *argp)`.
pub(crate) fn sys_nfssvc(args: &SysArgs, rval: &mut Rval) -> SysResult {
    kernel::api::fs::nfssvc(SYS_nfssvc as Register, int(args[0]), ptr(args[1]), rval)
}
/* </CODE> */
