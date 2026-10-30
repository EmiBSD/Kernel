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

/*
 * Copyright (c) 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)vfs_syscalls.c	8.28 (Berkeley) 12/10/94
 */
/* </LICENSES> */

/* <CODE> */
//! Virtual File System System Calls: `bsd`'s `sysent[]` rows of mounting (`mount`,
//! `unmount`, `sync`, `quotactl`, `statfs`, `fstatfs`, `getfsstat`), the current and root
//! directories (`chdir`, `fchdir`, `chroot`, `__getcwd`, `__realpath`, `unveil`), opening
//! (`open`, `openat`, `__pledge_open`, the file handle calls), the name space (`mknod`,
//! `mkfifo`, `link`, `symlink`, `unlink`, `rename`, `mkdir`, `rmdir`), attributes (`access`,
//! `stat`, `pathconf`, `readlink`, `chflags`, `chmod`, `chown`, `utimes`, `truncate`),
//! `lseek`, `fsync`, `getdents`, `umask`, `revoke` and the positioned reads and writes, and
//! the `Native` entries of the VFS calls (`VFS_ENTRIES`).
//!
//! Upstream: sys/kern/vfs_syscalls.c @ 3ce1f3f79392
//!
//! Since N5.4 every call is `kernel::fs::syscalls`'s (and `kernel::fs::getcwd`'s, and
//! `kernel::descrip`'s for the positioned I/O): each row here unpacks its `struct
//! sys_*_args` into the kernel function's typed arguments (a user address as a `UserPtr`)
//! and puts its answer in `retval[0]`. The helpers other `bsd` modules and their tests call
//! keep LZ's names and signatures over the kernel's (`dounmount` and `getvnode` themselves,
//! the `do*at` calls over a pathname's address). No logic of the C's is left here.
//!
//! ## Deviations
//! - The calls' deviations are `kernel::fs::syscalls`'s (its `## Deviations`): `access(2)`'s
//!   placed credentials, `statfs`'s copy of `mnt_stat`, `getvnode`'s returned file,
//!   `unveil(2)`'s `unveil_path`, `option FIFO` not configured (`mkfifo` answers
//!   `EOPNOTSUPP` as the C does without it), `KTRACE` not configured.
//! - `truncate(2)` and `ftruncate(2)` pin the thread for the kernel's calls (the
//!   `RLIMIT_FSIZE` overrun's `psignal`), as `proc_glue`'s rows do for `fork(2)`.

use core::pin::Pin;

use hal::api::VfsEntries;
use hal::syscall::SysResult;
use hal::user::UserPtr;
use kernel::fs::syscalls as fs;

pub use kernel::fs::syscalls::{dounmount, getvnode};

use crate::kern::kern_descrip::native;
use crate::sys::errno::Errno;
use crate::sys::proc::Proc;
use crate::sys::syscall::{
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
use crate::sys::syscallargs::*;
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::{Dev, Mode, Register};

/// The user address an argument holds (`SCARG(uap, x)` as the kernel's typed pointer).
fn uptr<T, U>(a: *const T) -> UserPtr<U> {
    UserPtr::new(a.addr())
}

/// The thread pinned, as the kernel's `truncate` calls take it (`proc_glue`'s `pinned`).
fn pinned(p: &Proc) -> Pin<&'static Proc> {
    hal::pin::pin_static(crate::sys::proc::ProcExt::as_static(p))
}

/// Mount a file system.
pub fn sys_mount(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMountArgs = sysargs(v);
    fs::sys_mount(
        p,
        uptr(uap.r#type.get()),
        uptr(uap.path.get()),
        uap.flags.get(),
        uap.data.get().addr(),
    )
}

/// Unmount a file system.
pub fn sys_unmount(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysUnmountArgs = sysargs(v);
    fs::sys_unmount(p, uptr(uap.path.get()), uap.flags.get())
}

/// Sync each mounted filesystem.
pub fn sys_sync(p: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    fs::sys_sync(p)
}

/// Change filesystem quotas.
pub fn sys_quotactl(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysQuotactlArgs = sysargs(v);
    fs::sys_quotactl(
        p,
        uptr(uap.path.get()),
        uap.cmd.get(),
        uap.uid.get(),
        uap.arg.get().addr(),
    )
}

/// Get filesystem statistics.
pub fn sys_statfs(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysStatfsArgs = sysargs(v);
    fs::sys_statfs(p, uptr(uap.path.get()), uptr(uap.buf.get()))
}

/// Get filesystem statistics.
pub fn sys_fstatfs(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFstatfsArgs = sysargs(v);
    fs::sys_fstatfs(p, uap.fd.get(), uptr(uap.buf.get()))
}

/// Get statistics on all filesystems.
pub fn sys_getfsstat(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetfsstatArgs = sysargs(v);
    let count = fs::sys_getfsstat(p, uptr(uap.buf.get()), uap.bufsize.get(), uap.flags.get())?;
    retval[0] = count as Register;
    Ok(())
}

/// Change current working directory to a given file descriptor.
pub fn sys_fchdir(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFchdirArgs = sysargs(v);
    fs::sys_fchdir(p, uap.fd.get())
}

/// Change current working directory (``.'').
pub fn sys_chdir(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysChdirArgs = sysargs(v);
    fs::sys_chdir(p, uptr(uap.path.get()))
}

/// Change notion of root (``/'') directory.
pub fn sys_chroot(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysChrootArgs = sysargs(v);
    fs::sys_chroot(p, uptr(uap.path.get()))
}

/// `__getcwd(buf, len)`: the pathname of the current directory (the kernel's,
/// `kernel::fs::getcwd`), as the system call table calls it.
#[allow(non_snake_case)] // the C name: sys___getcwd
pub fn sys___getcwd(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetcwdArgs = sysargs(v);
    kernel::fs::getcwd::sys___getcwd(p, uap.buf.get().addr(), uap.len.get())
}

/// `__realpath(pathname, resolved)`: the canonical absolute name of a path.
#[allow(non_snake_case)] // the C name: sys___realpath
pub fn sys___realpath(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRealpathArgs = sysargs(v);
    fs::sys___realpath(p, uptr(uap.pathname.get()), uptr(uap.resolved.get()))
}

/// `unveil(path, permissions)`.
pub fn sys_unveil(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysUnveilArgs = sysargs(v);
    fs::sys_unveil(p, uptr(uap.path.get()), uptr(uap.permissions.get()))
}

/// Check permissions, allocate an open file structure, and call the device open routine if
/// any.
pub fn sys_open(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysOpenArgs = sysargs(v);
    let fd = fs::sys_open(p, uptr(uap.path.get()), uap.flags.get(), uap.mode.get())?;
    retval[0] = fd as Register;
    Ok(())
}

/// `__pledge_open(path, flags, mode)`: libc's opens of its own files.
#[allow(non_snake_case)] // the C name: sys___pledge_open
pub fn sys___pledge_open(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPledgeOpenArgs = sysargs(v);
    let fd = fs::sys___pledge_open(p, uptr(uap.path.get()), uap.flags.get(), uap.mode.get())?;
    retval[0] = fd as Register;
    Ok(())
}

/// `openat(fd, path, flags, mode)`.
pub fn sys_openat(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysOpenatArgs = sysargs(v);
    let fd = fs::sys_openat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.flags.get(),
        uap.mode.get(),
    )?;
    retval[0] = fd as Register;
    Ok(())
}

/// Get file handle system call.
pub fn sys_getfh(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetfhArgs = sysargs(v);
    fs::sys_getfh(p, uptr(uap.fname.get()), uptr(uap.fhp.get()))
}

/// Open a file given a file handle.
pub fn sys_fhopen(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFhopenArgs = sysargs(v);
    let fd = fs::sys_fhopen(p, uptr(uap.fhp.get()), uap.flags.get())?;
    retval[0] = fd as Register;
    Ok(())
}

/// `fhstat(fhp, sb)`.
pub fn sys_fhstat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFhstatArgs = sysargs(v);
    fs::sys_fhstat(p, uptr(uap.fhp.get()), uptr(uap.sb.get()))
}

/// `fhstatfs(fhp, buf)`.
pub fn sys_fhstatfs(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFhstatfsArgs = sysargs(v);
    fs::sys_fhstatfs(p, uptr(uap.fhp.get()), uptr(uap.buf.get()))
}

/// Create a special file.
pub fn sys_mknod(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMknodArgs = sysargs(v);
    fs::sys_mknod(p, uptr(uap.path.get()), uap.mode.get(), uap.dev.get())
}

/// `mknodat(fd, path, mode, dev)`.
pub fn sys_mknodat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMknodatArgs = sysargs(v);
    fs::sys_mknodat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.mode.get(),
        uap.dev.get(),
    )
}

/// `domknodat` over a pathname's address (the callers of LZ's signature).
pub fn domknodat(p: &Proc, fd: i32, path: *const u8, mode: Mode, dev: Dev) -> Result<(), Errno> {
    fs::domknodat(p, fd, uptr(path), mode, dev)
}

/// Create a named pipe.
pub fn sys_mkfifo(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMkfifoArgs = sysargs(v);
    fs::sys_mkfifo(p, uptr(uap.path.get()), uap.mode.get())
}

/// `mkfifoat(fd, path, mode)`.
pub fn sys_mkfifoat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMkfifoatArgs = sysargs(v);
    fs::sys_mkfifoat(p, uap.fd.get(), uptr(uap.path.get()), uap.mode.get())
}

/// Make a hard file link.
pub fn sys_link(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysLinkArgs = sysargs(v);
    fs::sys_link(p, uptr(uap.path.get()), uptr(uap.link.get()))
}

/// `linkat(fd1, path1, fd2, path2, flag)`.
pub fn sys_linkat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysLinkatArgs = sysargs(v);
    fs::sys_linkat(
        p,
        uap.fd1.get(),
        uptr(uap.path1.get()),
        uap.fd2.get(),
        uptr(uap.path2.get()),
        uap.flag.get(),
    )
}

/// `dolinkat` over two pathnames' addresses (the callers of LZ's signature).
pub fn dolinkat(
    p: &Proc,
    fd1: i32,
    path1: *const u8,
    fd2: i32,
    path2: *const u8,
    flag: i32,
) -> Result<(), Errno> {
    fs::dolinkat(p, fd1, uptr(path1), fd2, uptr(path2), flag)
}

/// Make a symbolic link.
pub fn sys_symlink(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSymlinkArgs = sysargs(v);
    fs::sys_symlink(p, uptr(uap.path.get()), uptr(uap.link.get()))
}

/// `symlinkat(path, fd, link)`.
pub fn sys_symlinkat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSymlinkatArgs = sysargs(v);
    fs::sys_symlinkat(p, uptr(uap.path.get()), uap.fd.get(), uptr(uap.link.get()))
}

/// `dosymlinkat` over two pathnames' addresses (the callers of LZ's signature).
pub fn dosymlinkat(p: &Proc, upath_: *const u8, fd: i32, link: *const u8) -> Result<(), Errno> {
    fs::dosymlinkat(p, uptr(upath_), fd, uptr(link))
}

/// Delete a name from the filesystem.
pub fn sys_unlink(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysUnlinkArgs = sysargs(v);
    fs::sys_unlink(p, uptr(uap.path.get()))
}

/// `unlinkat(fd, path, flag)`.
pub fn sys_unlinkat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysUnlinkatArgs = sysargs(v);
    fs::sys_unlinkat(p, uap.fd.get(), uptr(uap.path.get()), uap.flag.get())
}

/// `dounlinkat` over a pathname's address (the callers of LZ's signature).
pub fn dounlinkat(p: &Proc, fd: i32, path: *const u8, flag: i32) -> Result<(), Errno> {
    fs::dounlinkat(p, fd, uptr(path), flag)
}

/// Reposition read/write file offset.
pub fn sys_lseek(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysLseekArgs = sysargs(v);
    let off = fs::sys_lseek(p, uap.fd.get(), uap.offset.get(), uap.whence.get())?;
    retval[0] = off as Register;
    Ok(())
}

/// Check access permissions.
pub fn sys_access(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysAccessArgs = sysargs(v);
    fs::sys_access(p, uptr(uap.path.get()), uap.amode.get())
}

/// `faccessat(fd, path, amode, flag)`.
pub fn sys_faccessat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFaccessatArgs = sysargs(v);
    fs::sys_faccessat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.amode.get(),
        uap.flag.get(),
    )
}

/// Get file status; this version follows links.
pub fn sys_stat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysStatArgs = sysargs(v);
    fs::sys_stat(p, uptr(uap.path.get()), uptr(uap.ub.get()))
}

/// `fstatat(fd, path, buf, flag)`.
pub fn sys_fstatat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFstatatArgs = sysargs(v);
    fs::sys_fstatat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uptr(uap.buf.get()),
        uap.flag.get(),
    )
}

/// Get file status; this version does not follow links.
pub fn sys_lstat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysLstatArgs = sysargs(v);
    fs::sys_lstat(p, uptr(uap.path.get()), uptr(uap.ub.get()))
}

/// Get configurable pathname variables.
pub fn sys_pathconf(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPathconfArgs = sysargs(v);
    retval[0] = fs::sys_pathconf(p, uptr(uap.path.get()), uap.name.get())?;
    Ok(())
}

/// `pathconfat(fd, path, name, flag)`.
pub fn sys_pathconfat(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPathconfatArgs = sysargs(v);
    retval[0] = fs::sys_pathconfat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.name.get(),
        uap.flag.get(),
    )?;
    Ok(())
}

/// Return target name of a symbolic link.
pub fn sys_readlink(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysReadlinkArgs = sysargs(v);
    let n = fs::sys_readlink(
        p,
        uptr(uap.path.get()),
        uap.buf.get().addr(),
        uap.count.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// `readlinkat(fd, path, buf, count)`.
pub fn sys_readlinkat(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysReadlinkatArgs = sysargs(v);
    let n = fs::sys_readlinkat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.buf.get().addr(),
        uap.count.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// `doreadlinkat` over a pathname's and a buffer's addresses, the count in `retval[0]` (the
/// callers of LZ's signature).
pub fn doreadlinkat(
    p: &Proc,
    fd: i32,
    path: *const u8,
    buf: *mut u8,
    count: usize,
    retval: &mut [Register; 2],
) -> Result<(), Errno> {
    retval[0] = fs::doreadlinkat(p, fd, uptr(path), buf.addr(), count)? as Register;
    Ok(())
}

/// Change flags of a file given a path name.
pub fn sys_chflags(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysChflagsArgs = sysargs(v);
    fs::sys_chflags(p, uptr(uap.path.get()), uap.flags.get())
}

/// `chflagsat(fd, path, flags, atflags)`.
pub fn sys_chflagsat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysChflagsatArgs = sysargs(v);
    fs::sys_chflagsat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.flags.get(),
        uap.atflags.get(),
    )
}

/// Change flags of a file given a file descriptor.
pub fn sys_fchflags(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFchflagsArgs = sysargs(v);
    fs::sys_fchflags(p, uap.fd.get(), uap.flags.get())
}

/// Change mode of a file given path name.
pub fn sys_chmod(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysChmodArgs = sysargs(v);
    fs::sys_chmod(p, uptr(uap.path.get()), uap.mode.get())
}

/// `fchmodat(fd, path, mode, flag)`.
pub fn sys_fchmodat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFchmodatArgs = sysargs(v);
    fs::sys_fchmodat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.mode.get(),
        uap.flag.get(),
    )
}

/// Change mode of a file given a file descriptor.
pub fn sys_fchmod(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFchmodArgs = sysargs(v);
    fs::sys_fchmod(p, uap.fd.get(), uap.mode.get())
}

/// Set ownership given a path name.
pub fn sys_chown(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysChownArgs = sysargs(v);
    fs::sys_chown(p, uptr(uap.path.get()), uap.uid.get(), uap.gid.get())
}

/// `fchownat(fd, path, uid, gid, flag)`.
pub fn sys_fchownat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFchownatArgs = sysargs(v);
    fs::sys_fchownat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uap.uid.get(),
        uap.gid.get(),
        uap.flag.get(),
    )
}

/// Set ownership given a path name, without following links.
pub fn sys_lchown(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysLchownArgs = sysargs(v);
    fs::sys_lchown(p, uptr(uap.path.get()), uap.uid.get(), uap.gid.get())
}

/// Set ownership given a file descriptor.
pub fn sys_fchown(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFchownArgs = sysargs(v);
    fs::sys_fchown(p, uap.fd.get(), uap.uid.get(), uap.gid.get())
}

/// Set the access and modification times given a path name.
pub fn sys_utimes(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysUtimesArgs = sysargs(v);
    fs::sys_utimes(p, uptr(uap.path.get()), uptr(uap.tptr.get()))
}

/// `utimensat(fd, path, times, flag)`.
pub fn sys_utimensat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysUtimensatArgs = sysargs(v);
    fs::sys_utimensat(
        p,
        uap.fd.get(),
        uptr(uap.path.get()),
        uptr(uap.times.get()),
        uap.flag.get(),
    )
}

/// Set the access and modification times given a file descriptor.
pub fn sys_futimes(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFutimesArgs = sysargs(v);
    fs::sys_futimes(p, uap.fd.get(), uptr(uap.tptr.get()))
}

/// `futimens(fd, times)`.
pub fn sys_futimens(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFutimensArgs = sysargs(v);
    fs::sys_futimens(p, uap.fd.get(), uptr(uap.times.get()))
}

/// Truncate a file given its path name.
pub fn sys_truncate(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysTruncateArgs = sysargs(v);
    fs::sys_truncate(pinned(p), uptr(uap.path.get()), uap.length.get())
}

/// Truncate a file given a file descriptor.
pub fn sys_ftruncate(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFtruncateArgs = sysargs(v);
    fs::sys_ftruncate(pinned(p), uap.fd.get(), uap.length.get())
}

/// Sync an open file.
pub fn sys_fsync(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysFsyncArgs = sysargs(v);
    fs::sys_fsync(p, uap.fd.get())
}

/// Rename files.
pub fn sys_rename(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRenameArgs = sysargs(v);
    fs::sys_rename(p, uptr(uap.from.get()), uptr(uap.to.get()))
}

/// `renameat(fromfd, from, tofd, to)`.
pub fn sys_renameat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRenameatArgs = sysargs(v);
    fs::sys_renameat(
        p,
        uap.fromfd.get(),
        uptr(uap.from.get()),
        uap.tofd.get(),
        uptr(uap.to.get()),
    )
}

/// `dorenameat` over two pathnames' addresses (the callers of LZ's signature).
pub fn dorenameat(
    p: &Proc,
    fromfd: i32,
    from: *const u8,
    tofd: i32,
    to: *const u8,
) -> Result<(), Errno> {
    fs::dorenameat(p, fromfd, uptr(from), tofd, uptr(to))
}

/// Make a directory file.
pub fn sys_mkdir(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMkdirArgs = sysargs(v);
    fs::sys_mkdir(p, uptr(uap.path.get()), uap.mode.get())
}

/// `mkdirat(fd, path, mode)`.
pub fn sys_mkdirat(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMkdiratArgs = sysargs(v);
    fs::sys_mkdirat(p, uap.fd.get(), uptr(uap.path.get()), uap.mode.get())
}

/// `domkdirat` over a pathname's address (the callers of LZ's signature).
pub fn domkdirat(p: &Proc, fd: i32, path: *const u8, mode: Mode) -> Result<(), Errno> {
    fs::domkdirat(p, fd, uptr(path), mode)
}

/// Remove a directory file.
pub fn sys_rmdir(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRmdirArgs = sysargs(v);
    fs::sys_rmdir(p, uptr(uap.path.get()))
}

/// Read a block of directory entries in a file system independent format.
pub fn sys_getdents(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysGetdentsArgs = sysargs(v);
    let n = fs::sys_getdents(p, uap.fd.get(), uap.buf.get().addr(), uap.buflen.get())?;
    retval[0] = n as Register;
    Ok(())
}

/// Set the mode mask for creation of filesystem nodes.
pub fn sys_umask(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysUmaskArgs = sysargs(v);
    retval[0] = fs::sys_umask(p, uap.newmask.get())? as Register;
    Ok(())
}

/// Void all references to file by ripping underlying filesystem away from vnode.
pub fn sys_revoke(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysRevokeArgs = sysargs(v);
    fs::sys_revoke(p, uptr(uap.path.get()))
}

/// Positional read system call (`sysent[]`'s row: `kernel::descrip::sys_pread`).
pub fn sys_pread(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPreadArgs = sysargs(v);
    let n = kernel::descrip::sys_pread(
        p,
        uap.fd.get(),
        uap.buf.get().addr(),
        uap.nbyte.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Positional scatter read system call (`sysent[]`'s row: `kernel::descrip::sys_preadv`).
pub fn sys_preadv(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPreadvArgs = sysargs(v);
    let n = kernel::descrip::sys_preadv(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Positional write system call (`sysent[]`'s row: `kernel::descrip::sys_pwrite`).
pub fn sys_pwrite(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPwriteArgs = sysargs(v);
    let n = kernel::descrip::sys_pwrite(
        p,
        uap.fd.get(),
        uap.buf.get().addr(),
        uap.nbyte.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Positional gather write system call (`sysent[]`'s row: `kernel::descrip::sys_pwritev`).
pub fn sys_pwritev(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPwritevArgs = sysargs(v);
    let n = kernel::descrip::sys_pwritev(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// `KernelApi::vfs`: the `Native` entries of the VFS system calls, each `mi_syscall` on its
/// own `sysent[]` row (`kern_descrip::native`: the prologue, pledge, the pinned syscalls and
/// the row's kernel lock, as on the `Legacy` path).
pub const VFS_ENTRIES: VfsEntries = VfsEntries {
    open: native_open,
    link: native_link,
    unlink: native_unlink,
    chdir: native_chdir,
    fchdir: native_fchdir,
    mknod: native_mknod,
    chmod: native_chmod,
    chown: native_chown,
    mount: native_mount,
    unmount: native_unmount,
    access: native_access,
    chflags: native_chflags,
    fchflags: native_fchflags,
    sync: native_sync,
    stat: native_stat,
    lstat: native_lstat,
    fstatat: native_fstatat,
    revoke: native_revoke,
    symlink: native_symlink,
    readlink: native_readlink,
    umask: native_umask,
    chroot: native_chroot,
    getfsstat: native_getfsstat,
    statfs: native_statfs,
    fstatfs: native_fstatfs,
    fhstatfs: native_fhstatfs,
    utimes: native_utimes,
    futimes: native_futimes,
    utimensat: native_utimensat,
    futimens: native_futimens,
    fsync: native_fsync,
    getdents: native_getdents,
    chflagsat: native_chflagsat,
    unveil: native_unveil,
    realpath: native_realpath,
    fchown: native_fchown,
    fchmod: native_fchmod,
    pledge_open: native_pledge_open,
    rename: native_rename,
    mkfifo: native_mkfifo,
    mkdir: native_mkdir,
    rmdir: native_rmdir,
    quotactl: native_quotactl,
    getfh: native_getfh,
    lseek: native_lseek,
    truncate: native_truncate,
    ftruncate: native_ftruncate,
    pathconfat: native_pathconfat,
    pathconf: native_pathconf,
    lchown: native_lchown,
    fhopen: native_fhopen,
    fhstat: native_fhstat,
    getcwd: native_getcwd,
    faccessat: native_faccessat,
    fchmodat: native_fchmodat,
    fchownat: native_fchownat,
    linkat: native_linkat,
    mkdirat: native_mkdirat,
    mkfifoat: native_mkfifoat,
    mknodat: native_mknodat,
    openat: native_openat,
    readlinkat: native_readlinkat,
    renameat: native_renameat,
    symlinkat: native_symlinkat,
    unlinkat: native_unlinkat,
};

/// `KernelApi::vfs.open`.
fn native_open(
    nr: Register,
    path: usize,
    flags: i32,
    mode: u32,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::vfs.link`.
fn native_link(nr: Register, path: usize, link: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_link,
        [path as Register, link as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.unlink`.
fn native_unlink(nr: Register, path: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_unlink, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.chdir`.
fn native_chdir(nr: Register, path: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_chdir, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.fchdir`.
fn native_fchdir(nr: Register, fd: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_fchdir, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.mknod`.
fn native_mknod(
    nr: Register,
    path: usize,
    mode: u32,
    dev: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_mknod,
        [path as Register, mode as Register, dev as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.chmod`.
fn native_chmod(nr: Register, path: usize, mode: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_chmod,
        [path as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.chown`.
fn native_chown(
    nr: Register,
    path: usize,
    uid: u32,
    gid: u32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_chown,
        [path as Register, uid as Register, gid as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.mount`.
fn native_mount(
    nr: Register,
    r#type: usize,
    path: usize,
    flags: i32,
    data: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.unmount`.
fn native_unmount(nr: Register, path: usize, flags: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_unmount,
        [path as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.access`.
fn native_access(nr: Register, path: usize, amode: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_access,
        [path as Register, amode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.chflags`.
fn native_chflags(nr: Register, path: usize, flags: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_chflags,
        [path as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fchflags`.
fn native_fchflags(nr: Register, fd: i32, flags: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fchflags,
        [fd as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.sync`.
fn native_sync(nr: Register, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_sync, [0; 6], rval)
}

/// `KernelApi::vfs.stat`.
fn native_stat(nr: Register, path: usize, ub: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_stat,
        [path as Register, ub as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.lstat`.
fn native_lstat(nr: Register, path: usize, ub: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_lstat,
        [path as Register, ub as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fstatat`.
fn native_fstatat(
    nr: Register,
    fd: i32,
    path: usize,
    buf: usize,
    flag: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.revoke`.
fn native_revoke(nr: Register, path: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_revoke, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.symlink`.
fn native_symlink(nr: Register, path: usize, link: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_symlink,
        [path as Register, link as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.readlink`.
fn native_readlink(
    nr: Register,
    path: usize,
    buf: usize,
    count: usize,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::vfs.umask`.
fn native_umask(nr: Register, newmask: u32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_umask, [newmask as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.chroot`.
fn native_chroot(nr: Register, path: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_chroot, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.getfsstat`.
fn native_getfsstat(
    nr: Register,
    buf: usize,
    bufsize: usize,
    flags: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.statfs`.
fn native_statfs(nr: Register, path: usize, buf: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_statfs,
        [path as Register, buf as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fstatfs`.
fn native_fstatfs(nr: Register, fd: i32, buf: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fstatfs,
        [fd as Register, buf as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fhstatfs`.
fn native_fhstatfs(nr: Register, fhp: usize, buf: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fhstatfs,
        [fhp as Register, buf as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.utimes`.
fn native_utimes(nr: Register, path: usize, tptr: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_utimes,
        [path as Register, tptr as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.futimes`.
fn native_futimes(nr: Register, fd: i32, tptr: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_futimes,
        [fd as Register, tptr as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.utimensat`.
fn native_utimensat(
    nr: Register,
    fd: i32,
    path: usize,
    times: usize,
    flag: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.futimens`.
fn native_futimens(nr: Register, fd: i32, times: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_futimens,
        [fd as Register, times as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fsync`.
fn native_fsync(nr: Register, fd: i32, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_fsync, [fd as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.getdents`.
fn native_getdents(
    nr: Register,
    fd: i32,
    buf: usize,
    buflen: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_getdents,
        [fd as Register, buf as Register, buflen as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.chflagsat`.
fn native_chflagsat(
    nr: Register,
    fd: i32,
    path: usize,
    flags: u32,
    atflags: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.unveil`.
fn native_unveil(
    nr: Register,
    path: usize,
    permissions: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_unveil,
        [path as Register, permissions as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.realpath`.
fn native_realpath(
    nr: Register,
    pathname: usize,
    resolved: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS___realpath,
        [pathname as Register, resolved as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fchown`.
fn native_fchown(nr: Register, fd: i32, uid: u32, gid: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fchown,
        [fd as Register, uid as Register, gid as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fchmod`.
fn native_fchmod(nr: Register, fd: i32, mode: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fchmod,
        [fd as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.pledge_open`.
fn native_pledge_open(
    nr: Register,
    path: usize,
    flags: i32,
    mode: u32,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::vfs.rename`.
fn native_rename(nr: Register, from: usize, to: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_rename,
        [from as Register, to as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.mkfifo`.
fn native_mkfifo(nr: Register, path: usize, mode: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_mkfifo,
        [path as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.mkdir`.
fn native_mkdir(nr: Register, path: usize, mode: u32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_mkdir,
        [path as Register, mode as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.rmdir`.
fn native_rmdir(nr: Register, path: usize, rval: &mut [Register; 2]) -> SysResult {
    native(nr, SYS_rmdir, [path as Register, 0, 0, 0, 0, 0], rval)
}

/// `KernelApi::vfs.quotactl`.
fn native_quotactl(
    nr: Register,
    path: usize,
    cmd: i32,
    uid: i32,
    arg: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.getfh`.
fn native_getfh(nr: Register, fname: usize, fhp: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_getfh,
        [fname as Register, fhp as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.lseek`.
fn native_lseek(
    nr: Register,
    fd: i32,
    offset: i64,
    whence: i32,
    rval: &mut [Register; 2],
) -> SysResult {
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

/// `KernelApi::vfs.truncate`.
fn native_truncate(nr: Register, path: usize, length: i64, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_truncate,
        [path as Register, length as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.ftruncate`.
fn native_ftruncate(nr: Register, fd: i32, length: i64, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_ftruncate,
        [fd as Register, length as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.pathconfat`.
fn native_pathconfat(
    nr: Register,
    fd: i32,
    path: usize,
    name: i32,
    flag: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.pathconf`.
fn native_pathconf(nr: Register, path: usize, name: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_pathconf,
        [path as Register, name as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.lchown`.
fn native_lchown(
    nr: Register,
    path: usize,
    uid: u32,
    gid: u32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_lchown,
        [path as Register, uid as Register, gid as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fhopen`.
fn native_fhopen(nr: Register, fhp: usize, flags: i32, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fhopen,
        [fhp as Register, flags as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.fhstat`.
fn native_fhstat(nr: Register, fhp: usize, sb: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS_fhstat,
        [fhp as Register, sb as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.getcwd`.
fn native_getcwd(nr: Register, buf: usize, len: usize, rval: &mut [Register; 2]) -> SysResult {
    native(
        nr,
        SYS___getcwd,
        [buf as Register, len as Register, 0, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.faccessat`.
fn native_faccessat(
    nr: Register,
    fd: i32,
    path: usize,
    amode: i32,
    flag: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.fchmodat`.
fn native_fchmodat(
    nr: Register,
    fd: i32,
    path: usize,
    mode: u32,
    flag: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.fchownat`.
fn native_fchownat(
    nr: Register,
    fd: i32,
    path: usize,
    uid: u32,
    gid: u32,
    flag: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.linkat`.
fn native_linkat(
    nr: Register,
    fd1: i32,
    path1: usize,
    fd2: i32,
    path2: usize,
    flag: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.mkdirat`.
fn native_mkdirat(
    nr: Register,
    fd: i32,
    path: usize,
    mode: u32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_mkdirat,
        [fd as Register, path as Register, mode as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.mkfifoat`.
fn native_mkfifoat(
    nr: Register,
    fd: i32,
    path: usize,
    mode: u32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_mkfifoat,
        [fd as Register, path as Register, mode as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.mknodat`.
fn native_mknodat(
    nr: Register,
    fd: i32,
    path: usize,
    mode: u32,
    dev: i32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.openat`.
fn native_openat(
    nr: Register,
    fd: i32,
    path: usize,
    flags: i32,
    mode: u32,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.readlinkat`.
fn native_readlinkat(
    nr: Register,
    fd: i32,
    path: usize,
    buf: usize,
    count: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.renameat`.
fn native_renameat(
    nr: Register,
    fromfd: i32,
    from: usize,
    tofd: i32,
    to: usize,
    rval: &mut [Register; 2],
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

/// `KernelApi::vfs.symlinkat`.
fn native_symlinkat(
    nr: Register,
    path: usize,
    fd: i32,
    link: usize,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_symlinkat,
        [path as Register, fd as Register, link as Register, 0, 0, 0],
        rval,
    )
}

/// `KernelApi::vfs.unlinkat`.
fn native_unlinkat(
    nr: Register,
    fd: i32,
    path: usize,
    flag: i32,
    rval: &mut [Register; 2],
) -> SysResult {
    native(
        nr,
        SYS_unlinkat,
        [fd as Register, path as Register, flag as Register, 0, 0, 0],
        rval,
    )
}
/* </CODE> */
