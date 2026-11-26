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
//! since N6.7b4 also the NFS server's `nfssvc(2)`, `sysent[155]`'s row over
//! `kernel::fs::nfs::nfssvc`, `subr_log`'s form for `sendsyslog(2)`. Their `Native` entries
//! (`VFS_ENTRIES`, `native_nfssvc`) moved to `kernel::api::fs` (decision 52, N7).
//!
//! Upstream: sys/kern/vfs_syscalls.c @ 3ce1f3f79392
//!
//! Since N5.4 every call is `kernel::fs::syscalls`'s (and `kernel::fs::getcwd`'s, and
//! `kernel::file::descrip`'s for the positioned I/O): each row here unpacks its `struct
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

use hal::user::UserPtr;
use kernel::fs::syscalls as fs;

pub use kernel::fs::syscalls::{dounmount, getvnode};

use crate::sys::errno::Errno;
use crate::sys::proc::Proc;
use crate::sys::syscallargs::*;
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::{Dev, Mode, Register};

/// The user address an argument holds (`SCARG(uap, x)` as the kernel's typed pointer).
fn uptr<T, U>(a: *const T) -> UserPtr<U> {
    UserPtr::new(a.addr())
}

/// The thread pinned, as the kernel's `truncate` calls take it (`proc_glue`'s `pinned`).
fn pinned(p: &Proc) -> Pin<&'static Proc> {
    hal::ds::pin::pin_static(crate::sys::proc::ProcExt::as_static(p))
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

/// Positional read system call (`sysent[]`'s row: `kernel::file::descrip::sys_pread`).
pub fn sys_pread(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPreadArgs = sysargs(v);
    let n = kernel::file::descrip::sys_pread(
        p,
        uap.fd.get(),
        uap.buf.get().addr(),
        uap.nbyte.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Positional scatter read system call (`sysent[]`'s row: `kernel::file::descrip::sys_preadv`).
pub fn sys_preadv(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPreadvArgs = sysargs(v);
    let n = kernel::file::descrip::sys_preadv(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Positional write system call (`sysent[]`'s row: `kernel::file::descrip::sys_pwrite`).
pub fn sys_pwrite(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPwriteArgs = sysargs(v);
    let n = kernel::file::descrip::sys_pwrite(
        p,
        uap.fd.get(),
        uap.buf.get().addr(),
        uap.nbyte.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// Positional gather write system call (`sysent[]`'s row: `kernel::file::descrip::sys_pwritev`).
pub fn sys_pwritev(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPwritevArgs = sysargs(v);
    let n = kernel::file::descrip::sys_pwritev(
        p,
        uap.fd.get(),
        UserPtr::new(uap.iovp.get().addr()),
        uap.iovcnt.get(),
        uap.offset.get(),
    )?;
    retval[0] = n as Register;
    Ok(())
}

/// `nfssvc(2)` (`sysent[155]`, under the kernel lock: the C's `STD`), the NFS server's pseudo
/// system call for the nfsds: the kernel's (N6.7b4), with `nfs_serv`'s `nfsrv3_procs[]`.
/// Without `option NFSSERVER` the C's `suser` and then `ENOSYS`.
#[cfg(any(feature = "nfsclient", feature = "nfsserver"))]
pub fn sys_nfssvc(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysNfssvcArgs = sysargs(v);
    #[cfg(feature = "nfsserver")]
    {
        kernel::fs::nfs::nfssvc::sys_nfssvc(
            p,
            uap.flag.get(),
            uap.argp.get().addr(),
            &kernel::fs::nfs::nfs_serv::nfsrv3_procs::<crate::kern::proc_glue::BsdProcGlue>(),
        )
    }
    #[cfg(not(feature = "nfsserver"))]
    {
        let _ = uap;
        // Must be super user
        crate::kern::kern_prot::suser(p)?;
        Err(Errno::ENOSYS)
    }
}

/* </CODE> */
