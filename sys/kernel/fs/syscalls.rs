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
//! Virtual File System System Calls (`vfs_syscalls.c`): mounting (`mount`, `unmount`,
//! `dounmount`, `sync`, `quotactl`, `statfs`, `fstatfs`, `getfsstat`), the current and root
//! directories (`chdir`, `fchdir`, `chroot`, `__realpath`, `unveil`), opening (`open`,
//! `openat`, `__pledge_open`, the file handle calls), the name space (`mknod`, `mkfifo`,
//! `link`, `symlink`, `unlink`, `rename`, `mkdir`, `rmdir`), attributes (`access`, `stat`,
//! `pathconf`, `readlink`, `chflags`, `chmod`, `chown`, `utimes`, `truncate`), `lseek`,
//! `fsync`, `getdents`, `umask` and `revoke`. `__getcwd(2)` is `kernel::fs::getcwd`'s, the
//! positioned reads and writes `kernel::descrip`'s.
//!
//! Upstream: sys/kern/vfs_syscalls.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_syscalls.rs@3c62ede68802
//!
//! Each call is the C's, its arguments typed as `<sys/syscallargs.h>` has them (a user
//! address a [`UserPtr`], an `int` an `i32`, a `mode_t` a [`Mode`]), its `retval` the `Ok` of
//! the `Result` (the descriptor of `open(2)`, the offset of `lseek(2)`, the count of
//! `readlink(2)` and `getdents(2)`). The kernel lock: the rows `syscalls.master` marks
//! `NOLOCK` (`open`, `openat`, `__pledge_open`, `stat`, `lstat`, `fstatat`, `umask`,
//! `__realpath`, `lseek`) run without it and take the C's inner spans themselves
//! (`hal::klock::kernel_lock`/`kernel_unlock`: `doopenat` from before `vn_open` to the last
//! `VOP_UNLOCK`, `dofstatat` around `namei`..`vput`, `__realpath` around `vfs_getcwd_common`
//! and around `namei`..`vrele`); every other row runs under the lock `mi_syscall` takes.
//!
//! What is still `bsd`'s, through the glues (bridges.toml): the unveil table and pledge's
//! checks (`ProcGlue::unveil_add`, `unveil_removevnode`, `unveil_set_done`, `pledge_chown`,
//! `unveil_state`; `FileGlue::pledge_flock`, N8), a vnode file's operations and object
//! (`FileGlue::vnode_file`, N8 with `Data`), the device switch (`spec::Devsw`, N7).
//!
//! ## Deviations
//! - N4.2 (`kernel::cred`): `access(2)`/`faccessat(2)` give the thread a placed copy of its
//!   credentials with the real ids as effective ones (`CredRef::new`) for the check and put
//!   the old ones back after, where the C `crdup`s and edits in place.
//! - `VFS_STATFS(mp, &mp->mnt_stat, p)` works on a copy of `mnt_stat` that is stored back
//!   ([`statfs_mnt`]; `Mount::stat` and `Mount::with_stat`).
//! - [`getvnode`] returns the file instead of filling `*fpp`; [`dorenameat`]'s `error = -1`
//!   ("the same file: nothing to do") is a flag.
//! - `sys_unveil`'s body after `single_thread_set` is `unveil_path`, so that every exit
//!   reaches `single_thread_clear`; the pathname buffer is a `PathBuf` given back on drop.
//! - `option FIFO` is not configured (`miscfs/fifofs` is not ported): `mkfifo` answers
//!   `EOPNOTSUPP` as the C does without it (`tools/xtask/diff-openbsd/expected.toml`,
//!   `syscalls/probes#6`).
//! - `KTRACE` is not configured.
//! - Seven pointers the C dereferences without a test are `Option`s here, and their `None`
//!   answers where the C would fault on NULL: `mount(2)`'s update of a vnode with no
//!   `v_mount` (`EINVAL`), `unmount(2)`'s (`EINVAL`), `quotactl(2)`'s (`ENOENT`),
//!   `statfs(2)`'s (`ENOENT`, vfs_syscalls.c:620), `getfh(2)`'s (a zero `fsid` in the
//!   handle, :1270), `__realpath(2)` with no current directory (`ENOENT`, as `namei` without
//!   a root file system) and `fhstatfs(2)` of a vnode with no mount (`ESTALE`). None is
//!   reachable on a booted system: a looked-up vnode is on a mount and every process has a
//!   current directory. `fstatfs(2)`'s `ENOENT` for a file with no mount is the C's own
//!   check (:647-651), not one of them.
//!
//! ## Redesign
//! - N5.4: LZ's `sys/kern/vfs_syscalls.rs` (`bsd`, each `sys_*(p, v, retval)` over
//!   `SysArgs` and raw `*const u8` paths; 13 `unsafe` at the pin, 3 left after N4.3-N5.3's
//!   adaptations) is this module, generic over the glue, in `kernel` (no `unsafe`, the crate
//!   forbids it). The arguments are typed: paths and
//!   buffers are `UserPtr`s copied through `hal::user` (namei's `NiDirp::User`), `struct
//!   stat` goes out as `Stat::to_bytes` (its padding zeroed), `struct statfs` as
//!   `hal::user::bytes_of` (it is `AbiPod`), `fhandle_t` as `UserPtr<Fhandle>`, the `utimes`
//!   arrays as `UserPtr<[Timeval; 2]>`/`UserPtr<[Timespec; 2]>`. `bsd`'s file of LZ's name
//!   is an adapter of no logic that unpacks the registers into these calls (its `sysent[]`
//!   rows and the 16 modules that import it keep LZ's names).
//! - The three `unsafe` left in `bsd`'s file: `MountArgs` (a `malloc`ed byte buffer read as a
//!   slice) is an `MArray<u8>` of `vfc_datasize` bytes (`malloc(M_TEMP, M_WAITOK)`, zeroed as
//!   the C's `M_ZERO`) filled by `copyin_slice` and freed by its drop where the C `free`s;
//!   `statfs_bytes` is `hal::user::bytes_of`; `__realpath`'s cwd buffer is an `MArray<u8>` of
//!   `MAXPATHLEN * 4` bytes (its zero fill a cost the C's `malloc` lacks, N12's row beside the
//!   `PathBuf` fill). The ten LZ had more (the mount list's and `dounmount`'s queue links,
//!   `queued`'s cast, `NameiBuf::as_mut`, the `iovec_free`s of `preadv`/`pwritev`) went with
//!   the typed lists, `PathBuf` and `kernel::descrip` before this slice.
//! - The vnode a file is open on is installed through `FileGlue::vnode_file` (the
//!   `pipe_file`/`kqueue_file` shape: the operations and the object, `bsd`'s `vnops` and
//!   `BsdFileData::vnode` until `Data` is the kernel's, N8); read back through
//!   `FileGlue::vnode_of`.
//! - The unveil and pledge calls stay `bsd`'s (`kern_unveil.c`, `kern_pledge.c`, N8): four
//!   `ProcGlue` legs called at the C's points (`pledge_chown` in the `chown` family,
//!   `unveil_add` in `unveil(2)`, `unveil_removevnode` in `dounmount_leaf`, `unveil_set_done`
//!   for `unveil(NULL, NULL)`), `ps_uvdone` read through `ProcGlue::unveil_state`.
//! - `truncate(2)`, `ftruncate(2)` and `dotruncate` take the thread pinned: the
//!   `RLIMIT_FSIZE` overrun's `psignal(p, SIGXFSZ)` signals a pinned thread (`kernel::sig`).
//! - `dounmount`'s work list (`SLIST_HEAD(, mount) mplist`) is a `hal::list::List` of the
//!   mounts' `mnt_dounmount` links on the stack (`insert_head`, `first` and `pop_head` O(1),
//!   `iter` O(1) a step: docs/COLLECTIONS.md), drained on every path; the nested mounts are
//!   found walking `mountlist_next` (O(1) a step) against that list, the C's two loops, and
//!   the `MNT_DOOMED` restart from its head is the C's. `sync(2)` and `getfsstat(2)` walk
//!   `mountlist` as `TAILQ_FOREACH(_REVERSE)` (the successor read after the body, O(1) a
//!   step), `dounmount_leaf`'s unveil walk `Mount::vnodes` (`TAILQ_FOREACH_SAFE`, O(1) a
//!   step), `checkdirs` the process list under the kernel lock's reader (the `vrele`s after
//!   it). No element is found again by key anywhere.
//! - The hop: a `Native` entry of the ABI module reaches these functions through `bsd`'s
//!   `kern_descrip::native` (`mi_syscall` on `bsd`'s `sysent[]` row: the prologue, the
//!   pinned syscalls, pledge and the row's `NOLOCK`) and `bsd`'s adapter; the kernel owns the
//!   rows' flags at N9.1 (D-N5.4-1).

use core::mem::{offset_of, size_of};
use core::pin::Pin;
use core::ptr;
use core::slice;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::list::List;
use hal::malloc::{M_WAITOK, MArray};
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{
    AT_EACCESS, AT_FDCWD, AT_REMOVEDIR, AT_SYMLINK_FOLLOW, AT_SYMLINK_NOFOLLOW, F_FLOCK, F_RDLCK,
    F_SETLK, F_WAIT, F_WRLCK, FMASK, FNONBLOCK, FREAD, FWRITE, Flock, O_ACCMODE, O_CLOEXEC,
    O_CLOFORK, O_CREAT, O_DIRECTORY, O_EXLOCK, O_RDONLY, O_RDWR, O_SHLOCK, O_TRUNC, fflags,
};
use hal::sys::limits::INT_MAX;
use hal::sys::mount::{
    Fhandle, Fsid, MFSNAMELEN, MNAMELEN, MNT_ASYNC, MNT_DOOMED, MNT_FORCE, MNT_LAZY, MNT_NOATIME,
    MNT_NODEV, MNT_NOEXEC, MNT_NOPERM, MNT_NOSUID, MNT_NOWAIT, MNT_OP_FLAGS, MNT_RDONLY,
    MNT_RELOAD, MNT_ROOTFS, MNT_SYNCHRONOUS, MNT_UPDATE, MNT_VISFLAGMASK, MNT_WAIT, MNT_WANTRDWR,
    MNT_WXALLOWED, Statfs, VB_DUPOK, VB_NOWAIT, VB_READ, VB_WAIT, VB_WRITE,
};
use hal::sys::pledge::{
    PLEDGE_CHOWN, PLEDGE_CPATH, PLEDGE_DPATH, PLEDGE_FATTR, PLEDGE_RPATH, PLEDGE_TTY,
    PLEDGE_UNVEIL, PLEDGE_WPATH,
};
use hal::sys::signal::SIGXFSZ;
use hal::sys::stat::{
    ACCESSPERMS, ALLPERMS, S_IFBLK, S_IFCHR, S_IFIFO, S_IFMT, S_ISTXT, Stat, UTIME_NOW, UTIME_OMIT,
};
use hal::sys::types::{Dev, Gid, Mode, Off, Register, Uid, major};
use hal::sys::unistd::{R_OK, SEEK_SET, W_OK, X_OK};
use hal::user::{UserPtr, bytes_of};

use super::cache::{cache_purge, cache_purgevfs};
use super::getcwd::{M_TEMP, vfs_getcwd_common};
use super::init::{rootvnode, set_rootvnode, vfs_byname};
use super::lookup::{namei, ndinit, ndinitat};
use super::mount::{
    ByDounmount, Mount, mountlist, mountlist_insert_tail, mountlist_next, mountlist_remove,
    vfs_busy, vfs_mount_alloc, vfs_mount_free, vfs_unbusy,
};
use super::namei::{
    BYPASSUNVEIL, CREATE, DELETE, FOLLOW, LOCKLEAF, LOCKPARENT, LOOKUP, MAXPATHLEN, NOCACHE,
    NOFOLLOW, NiDirp, PathBuf, REALPATH, RENAME, SAVENAME, SAVESTART, STRIPSLASHES, UNVEIL_CREATE,
    UNVEIL_PLEDGEOPEN, UNVEIL_READ, UNVEIL_WRITE, WANTPARENT, pathbuf_get,
};
use super::spec::{D_TTY, devsw};
use super::subr::{vattr_null, vfs_getvfs, vgone, vinvalbuf, vnoperm, vput, vref, vrele};
use super::sync::vfs_allocate_syncvnode;
use super::vnode::{
    GETCWD_CHECK_ACCESS, LK_EXCLUSIVE, LK_RETRY, REVOKEALL, V_SAVE, VA_UTIMES_CHANGE,
    VA_UTIMES_NULL, VALIASED, VBAD, VBLK, VCHR, VDIR, VEXEC, VLNK, VNOVAL, VREAD, VREG, VROOT,
    VSGID, VSOCK, VSUID, VWRITE, Vattr, Vnode, vn_lock,
};
use super::vnops::{vn_open, vn_stat, vn_writechk};
use super::vops::{
    Cred, Nd, VFS_FHTOVP, VFS_MOUNT, VFS_QUOTACTL, VFS_ROOT, VFS_START, VFS_STATFS, VFS_SYNC,
    VFS_UNMOUNT, VFS_VPTOFH, vop_abortop, vop_access, vop_advlock, vop_fsync, vop_getattr,
    vop_link, vop_mkdir, vop_mknod, vop_open, vop_pathconf, vop_readdir, vop_readlink, vop_remove,
    vop_rename, vop_revoke, vop_rmdir, vop_setattr, vop_symlink, vop_unlock,
};
use crate::assert::kpanic;
use crate::clock::{Timespec, Timeval};
use crate::cred::CredRef;
use crate::descrip::{
    UF_EXCLOSE, UF_FORKCLOSE, UF_PLEDGEOPEN, closef, dupfdopen, falloc, fd_getfile, fdinsert,
    fdplock, fdpunlock, fdremove,
};
use crate::file::{DTYPE_VNODE, FIF_HASLOCK, File, FileGlue, FileOps, FileRef, LockOwner, foffset};
use crate::proc::{PS_CHROOT, PS_PLEDGE, ProcGlue, SINGLE_UNWIND, Thread, suser};
use crate::resource::{RLIMIT_FSIZE, lim_cur_proc};
use crate::sig::{psignal, single_thread_clear, single_thread_set};
use crate::uio::{Iovec, Uio, UioRw, UioSeg};
use crate::vm::vnode::{uvm_vnp_sync, uvm_vnp_uncache};

/// The vnode a successful lookup left in a `nameidata` slot.
fn ndvp<G: ProcGlue>(vp: Option<&'static Vnode<G>>) -> &'static Vnode<G> {
    match vp {
        Some(vp) => vp,
        None => kpanic(format_args!("vfs_syscalls: namei returned no vnode")),
    }
}

/// The vnode a `DTYPE_VNODE` file is open on (`(struct vnode *)fp->f_data`): `panic(9)` for
/// another object, which only a file of another type holds.
fn file_vp<G: FileGlue>(fp: &File<G>) -> &'static Vnode<G::Proc> {
    match fp.data().and_then(G::vnode_of) {
        Some(vp) => vp,
        None => kpanic(format_args!(
            "vfs_syscalls: file {:#x} has no vnode",
            fp.addr()
        )),
    }
}

/// `strlen` of a NUL-terminated buffer (its length if it has no NUL).
fn cstrlen(b: &[u8]) -> usize {
    b.iter().position(|&c| c == 0).unwrap_or(b.len())
}

/// The user address `n` bytes after `a`, as the C's pointer arithmetic (a wrap faults at the
/// copy, never here).
fn uadd(a: UserPtr<u8>, n: usize) -> UserPtr<u8> {
    UserPtr::new(a.addr().wrapping_add(n))
}

/// `VFS_STATFS(mp, &mp->mnt_stat, p)`: refreshes the mount's statistics (see the module's
/// deviations).
fn statfs_mnt<G: ProcGlue>(mp: &'static Mount<G>, p: &Thread<G>) -> Result<(), Errno> {
    let mut sp = mp.stat();
    let error = VFS_STATFS(mp, &mut sp, p);
    mp.with_stat(|cur| *cur = sp);
    error
}

/// `sp->f_flags = mp->mnt_flag & MNT_VISFLAGMASK`.
fn statfs_flags<G: ProcGlue>(mp: &'static Mount<G>) {
    mp.with_stat(|sp| sp.f_flags = (mp.flag() & MNT_VISFLAGMASK) as u32);
}

/// `malloc(vfsp->vfc_datasize, M_TEMP, M_WAITOK | M_ZERO)` filled from the user's `data`:
/// the mount arguments' kernel copy, freed by its drop where the C `free`s it.
fn mount_args(data: usize, len: usize) -> Result<MArray<u8>, Errno> {
    let Some(mut args) = MArray::new(M_TEMP, M_WAITOK, len, |_| 0u8) else {
        return Err(Errno::ENOMEM);
    };
    UserPtr::<u8>::new(data).copyin_slice(&mut args[..])?;
    Ok(args)
}

/// Mount a file system.
pub fn sys_mount<G: ProcGlue>(
    p: &Thread<G>,
    ty: UserPtr<u8>,
    path: UserPtr<u8>,
    flags: i32,
    data: usize,
) -> Result<(), Errno> {
    let mut mntflag = 0;

    suser(p)?;

    // Mount points must fit in MNAMELEN, not MAXPATHLEN.
    let mut fspath = [0u8; MNAMELEN];
    path.copyinstr(&mut fspath)?;

    // Get vnode to be covered
    let mut nd = ndinit(LOOKUP, FOLLOW | LOCKLEAF, NiDirp::Sys(&fspath), p);
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let mp: &'static Mount<G>;
    let mut args: MArray<u8>;
    if flags & MNT_UPDATE != 0 {
        if vp.flag() & VROOT == 0 {
            vput(vp);
            return Err(Errno::EINVAL);
        }
        let Some(vmp) = vp.mount() else {
            vput(vp);
            return Err(Errno::EINVAL);
        };
        mp = vmp;
        let vfsp = mp.vfc();

        args = match mount_args(data, vfsp.vfc_datasize) {
            Ok(args) => args,
            Err(e) => {
                vput(vp);
                return Err(e);
            }
        };

        mntflag = mp.flag();
        // We only allow the filesystem to be reloaded if it is currently mounted read-only.
        if flags & MNT_RELOAD != 0 && mp.flag() & MNT_RDONLY == 0 {
            vput(vp);
            return Err(Errno::EOPNOTSUPP); // Needs translation
        }

        if let Err(e) = vfs_busy(mp, VB_READ | VB_NOWAIT) {
            vput(vp);
            return Err(e);
        }
        mp.flag_set(flags & (MNT_RELOAD | MNT_UPDATE));
    } else {
        // Do not allow disabling of permission checks unless exec and access to device files
        // is disabled too.
        if flags & MNT_NOPERM != 0 && flags & (MNT_NODEV | MNT_NOEXEC) != (MNT_NODEV | MNT_NOEXEC) {
            vput(vp);
            return Err(Errno::EPERM);
        }
        if let Err(e) = vinvalbuf(vp, V_SAVE, Cred::real(p.ucred()), Some(p), 0, INFSLP) {
            vput(vp);
            return Err(e);
        }
        if vp.vtype() != VDIR {
            vput(vp);
            return Err(Errno::ENOTDIR);
        }
        let mut fstypename = [0u8; MFSNAMELEN];
        if let Err(e) = ty.copyinstr(&mut fstypename) {
            vput(vp);
            return Err(e);
        }
        let Some(vfsp) = vfs_byname::<G>(&fstypename[..cstrlen(&fstypename)]) else {
            vput(vp);
            return Err(Errno::EOPNOTSUPP);
        };

        args = match mount_args(data, vfsp.vfc_datasize) {
            Ok(args) => args,
            Err(e) => {
                vput(vp);
                return Err(e);
            }
        };

        if vp.mountedhere().is_some() {
            vput(vp);
            return Err(Errno::EBUSY);
        }

        // Allocate and initialize the file system.
        mp = vfs_mount_alloc(Some(vp), vfsp);
        let owner = p.ucred().cr_uid.get();
        mp.with_stat(|sp| sp.f_owner = owner);
    }

    // update:
    let Some(vmount) = vp.mount() else {
        kpanic(format_args!("sys_mount: covered vnode has no mount"));
    };
    // Ensure that the parent mountpoint does not get unmounted.
    if let Err(e) = vfs_busy(vmount, VB_READ | VB_NOWAIT | VB_DUPOK) {
        if mp.flag() & MNT_UPDATE != 0 {
            mp.set_flag(mntflag);
            vfs_unbusy(mp);
        } else {
            vfs_unbusy(mp);
            vfs_mount_free(mp);
        }
        vput(vp);
        return Err(e);
    }

    // Set the mount level flags.
    if flags & MNT_RDONLY != 0 {
        mp.flag_set(MNT_RDONLY);
    } else if mp.flag() & MNT_RDONLY != 0 {
        mp.flag_set(MNT_WANTRDWR);
    }
    let settable = MNT_NOSUID
        | MNT_NOEXEC
        | MNT_WXALLOWED
        | MNT_NODEV
        | MNT_SYNCHRONOUS
        | MNT_ASYNC
        | MNT_NOATIME
        | MNT_NOPERM
        | MNT_FORCE;
    mp.set_flag((mp.flag() & !settable) | (flags & settable));
    // Mount the filesystem.
    let path_len = cstrlen(&fspath);
    let mut error = VFS_MOUNT(mp, &fspath[..path_len], &mut args[..], &mut nd, p);
    if error.is_ok() {
        let now = crate::tc::gettime();
        mp.with_stat(|sp| sp.f_ctime = now as u64);
    }
    if mp.flag() & MNT_UPDATE != 0 {
        vfs_unbusy(vmount);
        vput(vp);
        if mp.flag() & MNT_WANTRDWR != 0 {
            mp.flag_clr(MNT_RDONLY);
        }
        mp.flag_clr(MNT_OP_FLAGS);
        if error.is_err() {
            mp.set_flag(mntflag);
        }

        if mp.flag() & MNT_RDONLY == 0 {
            if mp.syncer().is_none() {
                error = vfs_allocate_syncvnode(mp);
            }
        } else {
            if let Some(syncer) = mp.syncer() {
                vgone(syncer);
            }
            mp.set_syncer(None);
        }

        vfs_unbusy(mp);
        return error;
    }

    mp.flag_clr(MNT_OP_FLAGS);
    vp.set_mountedhere(Some(mp));

    // Put the new filesystem on the mount list after root.
    cache_purge(vp);
    if error.is_ok() {
        mountlist_insert_tail(mp);
        checkdirs(vp);
        vfs_unbusy(vmount);
        let _ = vop_unlock(vp);
        if mp.flag() & MNT_RDONLY == 0 {
            // The C overwrites this error with VFS_START's below.
            let _ = vfs_allocate_syncvnode(mp);
        }
        vfs_unbusy(mp);
        let _ = statfs_mnt(mp, p);
        error = VFS_START(mp, 0, p);
        if error.is_err() {
            vrele(vp);
        }
    } else {
        if let Some(covered) = mp.vnodecovered() {
            covered.set_mountedhere(None);
        }
        vfs_unbusy(mp);
        vfs_mount_free(mp);
        vfs_unbusy(vmount);
        vput(vp);
    }
    error
}

/// Scan all active processes to see if any of them have a current or root directory onto
/// which the new filesystem has just been mounted. If so, replace them with the new mount
/// point, keeping track of how many were replaced. That's the number of references the old
/// vnode had that we've replaced, so finish by vrele()'ing it that many times. This puts off
/// any possible sleeping until we've finished walking the allprocess list.
pub fn checkdirs<G: ProcGlue>(olddp: &'static Vnode<G>) {
    let mut free_count = 0u32;

    if olddp.usecount() == 1 {
        return;
    }
    let Some(newdp) = olddp.mountedhere().and_then(|mp| VFS_ROOT(mp).ok()) else {
        kpanic(format_args!("mount: lost mount"));
    };
    {
        // The walk alone holds the kernel lock's reader: vrele and vput below may sleep, and
        // a sleep with a witness out stops the kernel.
        let r = klock::reader();
        for pr in G::tables().allprocess.iter(&r) {
            // A process has its table from fork1 to its reaping (`fd_opt` for one not yet
            // given its own: nothing to replace).
            let Some(fdp) = pr.fd_opt() else {
                continue;
            };
            if fdp.cdir().is_some_and(|d| ptr::eq(d, olddp)) {
                free_count += 1;
                vref(newdp);
                fdp.set_cdir(Some(newdp));
            }
            if fdp.rdir().is_some_and(|d| ptr::eq(d, olddp)) {
                free_count += 1;
                vref(newdp);
                fdp.set_rdir(Some(newdp));
            }
        }
    }
    if rootvnode::<G>().is_some_and(|r| ptr::eq(r, olddp)) {
        free_count += 1;
        vref(newdp);
        set_rootvnode(Some(newdp));
    }
    while free_count > 0 {
        free_count -= 1;
        vrele(olddp);
    }
    vput(newdp);
}

/// Unmount a file system.
///
/// Note: unmount takes a path to the vnode mounted on as argument, not special file (as
/// before).
pub fn sys_unmount<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>, flags: i32) -> Result<(), Errno> {
    suser(p)?;

    let mut nd = ndinit(LOOKUP, FOLLOW | LOCKLEAF, NiDirp::User(path.addr()), p);
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let Some(mp) = vp.mount() else {
        vput(vp);
        return Err(Errno::EINVAL);
    };

    // Don't allow unmounting the root file system.
    if mp.flag() & MNT_ROOTFS != 0 {
        vput(vp);
        return Err(Errno::EINVAL);
    }

    // Must be the root of the filesystem
    if vp.flag() & VROOT == 0 {
        vput(vp);
        return Err(Errno::EINVAL);
    }
    vput(vp);

    if vfs_busy(mp, VB_WRITE | VB_WAIT).is_err() {
        return Err(Errno::EBUSY);
    }

    dounmount(mp, flags & MNT_FORCE, p)
}

/// Do the actual file system unmount.
pub fn dounmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    // The work queue (`SLIST_HEAD(, mount) mplist`) through `mnt_dounmount`: every mount on it
    // is write-busied by this thread, so no other unmount links it meanwhile; it is drained
    // on every path below.
    let mplist: List<ByDounmount<G>> = List::new();

    mplist.insert_head(mp);

    // Collect nested mount points. This takes advantage of the mount list being ordered -
    // nested mount points come after their parent.
    let error = 'err: {
        let mut cur = mp;
        while let Some(next) = mountlist_next(cur, false) {
            cur = next;
            for nmp in &mplist {
                if !cur
                    .vnodecovered()
                    .and_then(Vnode::mount)
                    .is_some_and(|m| ptr::eq(m, nmp))
                {
                    continue;
                }

                if flags & MNT_FORCE == 0 {
                    break 'err Errno::EBUSY;
                }
                if let Err(error) = vfs_busy(cur, VB_WRITE | VB_WAIT | VB_DUPOK) {
                    if flags & MNT_DOOMED != 0 {
                        // If the mount point was busy due to being unmounted, it has been
                        // removed from the mount list already. Restart the iteration from
                        // the last collected busy entry.
                        if let Some(first) = mplist.first() {
                            cur = first;
                        }
                        break;
                    }
                    break 'err error;
                }
                mplist.insert_head(cur);
                break;
            }
        }

        // Nested mount points cannot appear during this loop as mounting requires a read lock
        // for the parent mount point.
        while let Some(m) = mplist.pop_head() {
            if let Err(error) = dounmount_leaf(m, flags, p) {
                break 'err error;
            }
        }
        return Ok(());
    };

    while let Some(m) = mplist.pop_head() {
        vfs_unbusy(m);
    }
    Err(error)
}

/// `dounmount_leaf(mp, flags, p)`: unmounts one file system with nothing mounted below it.
pub fn dounmount_leaf<G: ProcGlue>(
    mp: &'static Mount<G>,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut hadsyncer = false;

    mp.flag_clr(MNT_ASYNC);
    cache_purgevfs(mp); // remove cache entries for this file sys
    if let Some(syncer) = mp.syncer() {
        hadsyncer = true;
        vgone(syncer);
        mp.set_syncer(None);
    }

    // Before calling file system unmount, make sure all unveils to vnodes in here are
    // dropped.
    for vp in mp.vnodes() {
        G::unveil_removevnode(vp);
    }

    let mut error = if mp.flag() & MNT_RDONLY != 0 {
        Ok(())
    } else {
        VFS_SYNC(mp, MNT_WAIT, 0, Cred::real(p.ucred()), p)
    };
    if error.is_ok() || flags & MNT_FORCE != 0 {
        error = VFS_UNMOUNT(mp, flags, p);
    }

    if error.is_err() && flags & MNT_DOOMED == 0 {
        if mp.flag() & MNT_RDONLY == 0 && hadsyncer {
            let _ = vfs_allocate_syncvnode(mp);
        }
        vfs_unbusy(mp);
        return error;
    }

    mountlist_remove(mp);
    if let Some(coveredvp) = mp.vnodecovered() {
        coveredvp.set_mountedhere(None);
        vrele(coveredvp);
    }

    if !mp.vnodelist_is_empty() {
        kpanic(format_args!("unmount: dangling vnode"));
    }

    vfs_unbusy(mp);
    vfs_mount_free(mp);

    Ok(())
}

/// Sync each mounted filesystem.
pub fn sys_sync<G: ProcGlue>(p: &Thread<G>) -> Result<(), Errno> {
    for mp in mountlist::<G>(true) {
        if vfs_busy(mp, VB_READ | VB_NOWAIT).is_err() {
            continue;
        }
        if mp.flag() & MNT_RDONLY == 0 {
            let asyncflag = mp.flag() & MNT_ASYNC;
            mp.flag_clr(MNT_ASYNC);
            uvm_vnp_sync(Some(mp.addr()));
            let _ = VFS_SYNC(mp, MNT_NOWAIT, 0, Cred::real(p.ucred()), p);
            if asyncflag != 0 {
                mp.flag_set(MNT_ASYNC);
            }
        }
        vfs_unbusy(mp);
    }

    Ok(())
}

/// Change filesystem quotas.
pub fn sys_quotactl<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    cmd: i32,
    uid: i32,
    arg: usize,
) -> Result<(), Errno> {
    let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::User(path.addr()), p);
    nd.ni_unveil = UNVEIL_READ | UNVEIL_WRITE;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let mp = vp.mount();
    vrele(vp);
    let Some(mp) = mp else {
        return Err(Errno::ENOENT);
    };
    VFS_QUOTACTL(mp, cmd, uid as Uid, arg, p)
}

/// `copyout_statfs(sp, uaddr, p)`: copies the statistics out, hiding the filesystem id from
/// non-root (for NFS security).
pub fn copyout_statfs<G: ProcGlue>(
    sp: &Statfs,
    uaddr: UserPtr<u8>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let co_sz1 = offset_of!(Statfs, f_fsid);
    let co_off2 = co_sz1 + size_of::<Fsid>();
    let s = bytes_of(slice::from_ref(sp));

    // Don't let non-root see filesystem id (for NFS security)
    if suser(p).is_err() {
        let fsid = [0u8; size_of::<Fsid>()];

        uaddr.copyout_slice(&s[..co_sz1])?;
        uadd(uaddr, co_sz1).copyout_slice(&fsid)?;
        return uadd(uaddr, co_off2).copyout_slice(&s[co_off2..]);
    }

    uaddr.copyout_slice(s)
}

/// Get filesystem statistics.
pub fn sys_statfs<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    buf: UserPtr<u8>,
) -> Result<(), Errno> {
    let mut nd = ndinit(LOOKUP, FOLLOW | BYPASSUNVEIL, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let mp = vp.mount();
    vrele(vp);
    let Some(mp) = mp else {
        return Err(Errno::ENOENT);
    };
    statfs_mnt(mp, p)?;
    statfs_flags(mp);

    copyout_statfs(&mp.stat(), buf, p)
}

/// Get filesystem statistics.
pub fn sys_fstatfs<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    buf: UserPtr<u8>,
) -> Result<(), Errno> {
    let fp = getvnode(p, fd)?;
    let Some(mp) = file_vp(&fp).mount() else {
        let _ = fp.frele(Some(p));
        return Err(Errno::ENOENT);
    };
    let error = statfs_mnt(mp, p);
    let _ = fp.frele(Some(p));
    error?;
    statfs_flags(mp);

    copyout_statfs(&mp.stat(), buf, p)
}

/// Get statistics on all filesystems: the number of mounts (`*retval`), at most what the
/// buffer holds when one is given.
pub fn sys_getfsstat<G: ProcGlue>(
    p: &Thread<G>,
    buf: UserPtr<u8>,
    bufsize: usize,
    flags: i32,
) -> Result<usize, Errno> {
    let maxcount = bufsize / size_of::<Statfs>();
    let mut sfsp = buf;
    let mut count = 0usize;

    for mp in mountlist::<G>(false) {
        if vfs_busy(mp, VB_READ | VB_NOWAIT).is_err() {
            continue;
        }
        if !sfsp.is_null() && count < maxcount {
            // Refresh stats unless MNT_NOWAIT is specified
            if flags != MNT_NOWAIT
                && flags != MNT_LAZY
                && (flags == MNT_WAIT || flags == 0)
                && statfs_mnt(mp, p).is_err()
            {
                vfs_unbusy(mp);
                continue;
            }

            statfs_flags(mp);
            if let Err(error) = copyout_statfs(&mp.stat(), sfsp, p) {
                vfs_unbusy(mp);
                return Err(error);
            }
            sfsp = uadd(sfsp, size_of::<Statfs>());
        }
        count += 1;
        vfs_unbusy(mp);
    }

    Ok(if !sfsp.is_null() && count > maxcount {
        maxcount
    } else {
        count
    })
}

/// Change current working directory to a given file descriptor.
pub fn sys_fchdir<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
) -> Result<(), Errno> {
    let fdp = p.fd();
    let Some(fp) = fd_getfile(fdp, fd) else {
        return Err(Errno::EBADF);
    };
    if fp.f_type() != DTYPE_VNODE || file_vp(&fp).vtype() != VDIR {
        let _ = fp.frele(Some(p));
        return Err(Errno::ENOTDIR);
    }
    let mut vp = file_vp(&fp);
    vref(vp);
    let _ = fp.frele(Some(p));
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let mut error = vop_access(vp, VEXEC, Cred::real(p.ucred()), p);

    while error.is_ok()
        && let Some(mp) = vp.mountedhere()
    {
        if vfs_busy(mp, VB_READ | VB_WAIT).is_err() {
            continue;
        }
        let root = VFS_ROOT(mp);
        vfs_unbusy(mp);
        match root {
            Err(e) => {
                error = Err(e);
                break;
            }
            Ok(tdp) => {
                vput(vp);
                vp = tdp;
            }
        }
    }
    if let Err(e) = error {
        vput(vp);
        return Err(e);
    }
    let _ = vop_unlock(vp);
    if let Some(old_cdir) = fdp.replace_cdir(Some(vp)) {
        vrele(old_cdir);
    }
    Ok(())
}

/// Change current working directory (``.'').
pub fn sys_chdir<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>) -> Result<(), Errno> {
    let fdp = p.fd();

    let mut nd = ndinit(LOOKUP, FOLLOW | LOCKLEAF, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    change_dir(&mut nd, p)?;
    if let Some(old_cdir) = fdp.replace_cdir(nd.ni_vp) {
        vrele(old_cdir);
    }
    Ok(())
}

/// Change notion of root (``/'') directory.
pub fn sys_chroot<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>) -> Result<(), Errno> {
    let fdp = p.fd();

    suser(p)?;
    let mut nd = ndinit(LOOKUP, FOLLOW | LOCKLEAF, NiDirp::User(path.addr()), p);
    change_dir(&mut nd, p)?;
    let vp = ndvp(nd.ni_vp);
    if let Some(old_rdir) = fdp.rdir() {
        // A chroot() done inside a changed root environment does an automatic chdir to avoid
        // the out-of-tree experience.
        vref(vp);
        fdp.set_rdir(Some(vp));
        let old_cdir = fdp.replace_cdir(Some(vp));
        vrele(old_rdir);
        if let Some(old_cdir) = old_cdir {
            vrele(old_cdir);
        }
    } else {
        fdp.set_rdir(Some(vp));
    }
    p.process().ps_flags.fetch_or(PS_CHROOT, Ordering::SeqCst);
    Ok(())
}

/// Common routine for chroot and chdir.
fn change_dir<G: ProcGlue>(ndp: &mut Nd<'_, G>, p: &Thread<G>) -> Result<(), Errno> {
    namei(ndp)?;
    let vp = ndvp(ndp.ni_vp);
    let error = if vp.vtype() == VDIR {
        vop_access(vp, VEXEC, Cred::real(p.ucred()), p)
    } else {
        Err(Errno::ENOTDIR)
    };
    if error.is_err() {
        vput(vp);
    } else {
        let _ = vop_unlock(vp);
    }
    error
}

/// `__realpath(pathname, resolved)`: the canonical absolute name of a path.
#[allow(non_snake_case)] // the C name: sys___realpath
pub fn sys___realpath<G: ProcGlue>(
    p: &Thread<G>,
    pathname: UserPtr<u8>,
    resolved: UserPtr<u8>,
) -> Result<(), Errno> {
    if pathname.is_null() {
        return Err(Errno::EINVAL);
    }

    let mut path = pathbuf_get::<G>().ok_or(Errno::ENOMEM)?;
    let mut rpbuf: PathBuf = pathbuf_get::<G>().ok_or(Errno::ENOMEM)?;

    let pathlen = pathname.copyinstr(&mut path[..])?;

    if pathlen == 1 {
        // empty string ""
        return Err(Errno::ENOENT);
    }
    if pathlen < 2 {
        return Err(Errno::EINVAL);
    }

    // Get cwd for relative path if needed, prepend to rpbuf
    rpbuf[0] = 0;
    if path[0] != b'/' {
        let cwdlen = MAXPATHLEN * 4; // for vfs_getcwd_common
        let Some(mut cwdbuf) = MArray::new(M_TEMP, M_WAITOK, cwdlen, |_| 0u8) else {
            return Err(Errno::ENOMEM);
        };

        // vfs_getcwd_common fills this in backwards
        let mut bp = cwdlen - 1;
        cwdbuf[bp] = 0;

        klock::kernel_lock();
        let error = match p.fd().cdir() {
            None => Err(Errno::ENOENT), // no root file system yet
            Some(cdir) => vfs_getcwd_common(
                cdir,
                None,
                Some((&mut cwdbuf[..], &mut bp)),
                (cwdlen / 2) as i32,
                GETCWD_CHECK_ACCESS,
                p,
            ),
        };
        klock::kernel_unlock();
        error?;

        // strlcpy(rpbuf, bp, MAXPATHLEN) >= MAXPATHLEN
        let cwd = &cwdbuf[bp..];
        let len = cstrlen(cwd);
        if len >= MAXPATHLEN {
            return Err(Errno::ENAMETOOLONG);
        }
        rpbuf[..len].copy_from_slice(&cwd[..len]);
        rpbuf[len] = 0;
    }

    let mut nd = ndinit(
        LOOKUP,
        FOLLOW | SAVENAME | REALPATH,
        NiDirp::Sys(&path[..]),
        p,
    );

    nd.ni_cnd.cn_rpi = cstrlen(&rpbuf[..]);
    nd.ni_cnd.cn_rpbuf = Some(rpbuf);

    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    klock::kernel_lock();
    if let Err(e) = namei(&mut nd) {
        klock::kernel_unlock();
        return Err(e);
    }

    // release reference from namei
    if let Some(vp) = nd.ni_vp {
        vrele(vp);
    }
    klock::kernel_unlock();

    let rp = nd.ni_cnd.rpbuf().map_or(&[][..], |rp| &rp[..]);
    let error = resolved.copyoutstr(rp).map(|_| ());

    // KTRACE: not configured.
    drop(nd.ni_cnd.take_pnbuf());
    error
}

/// `unveil(path, permissions)`: unveils a path to the process (`kern_unveil.c`'s table,
/// `bsd`'s until N8, through the glue); both NULL forbids any further `unveil`.
pub fn sys_unveil<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    permissions: UserPtr<u8>,
) -> Result<(), Errno> {
    let pr = p.process();

    if path.is_null() && permissions.is_null() {
        G::unveil_set_done(pr);
        return Ok(());
    }

    if G::unveil_state(pr).done {
        return Err(Errno::EPERM);
    }

    let mut perms = [0u8; 5];
    permissions.copyinstr(&mut perms)?;

    // System calls in other threads may sleep between unveil datastructure inspections --
    // this is the simplest way to provide consistency
    let _ = single_thread_set(p, SINGLE_UNWIND);

    let error = unveil_path(p, path, &perms[..cstrlen(&perms)]);

    single_thread_clear(p);
    error
}

/// The body of `sys_unveil` between `single_thread_set` and `single_thread_clear` (the C's
/// jumps to `end` are the returns).
fn unveil_path<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    permissions: &[u8],
) -> Result<(), Errno> {
    let mut pathname = pathbuf_get::<G>().ok_or(Errno::ENOMEM)?;
    let pathlen = path.copyinstr(&mut pathname[..])?;
    let pathname = &pathname[..];

    // KTRACE: not configured.
    if pathlen < 2 {
        return Err(Errno::EINVAL);
    }

    // find root "/" or "//"
    let mut nd = if pathname[..cstrlen(pathname)].iter().all(|&c| c == b'/') {
        // root directory
        ndinit(
            LOOKUP,
            FOLLOW | LOCKLEAF | SAVENAME,
            NiDirp::Sys(pathname),
            p,
        )
    } else {
        ndinit(
            CREATE,
            FOLLOW | LOCKLEAF | LOCKPARENT | SAVENAME,
            NiDirp::Sys(pathname),
            p,
        )
    };

    nd.ni_pledge = PLEDGE_UNVEIL;
    namei(&mut nd)?;

    // XXX Any access to the file or directory will allow us to pledge path it
    let any_access = |vp: &'static Vnode<G>| {
        [VREAD, VWRITE, VEXEC]
            .into_iter()
            .any(|mode| vop_access(vp, mode, Cred::real(p.ucred()), p).is_ok())
    };
    let allow = nd.ni_vp.is_some_and(any_access) || nd.ni_dvp.is_some_and(any_access);

    // release lock from namei, but keep ref
    if let Some(vp) = nd.ni_vp {
        let _ = vop_unlock(vp);
    }
    if let Some(dvp) = nd.ni_dvp
        && !nd.ni_vp.is_some_and(|vp| ptr::eq(vp, dvp))
    {
        let _ = vop_unlock(dvp);
    }

    let error = if allow {
        G::unveil_add(p, &nd, permissions)
    } else {
        Err(Errno::EPERM)
    };

    // release vref from namei, but not vref from unveil_add
    if let Some(vp) = nd.ni_vp {
        vrele(vp);
    }
    if let Some(dvp) = nd.ni_dvp {
        vrele(dvp);
    }

    drop(nd.ni_cnd.take_pnbuf());
    error
}

/// Check permissions, allocate an open file structure, and call the device open routine if
/// any: the new descriptor.
pub fn sys_open<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    path: UserPtr<u8>,
    flags: i32,
    mode: Mode,
) -> Result<i32, Errno> {
    doopenat(p, AT_FDCWD, path, flags, mode, 0)
}

/// Check permissions, allocate an open file structure, and call the device open routine if
/// any: `open(2)` for pledge's `pledge_open`, its descriptor marked `UF_PLEDGEOPEN`.
#[allow(non_snake_case)] // the C name: sys___pledge_open
pub fn sys___pledge_open<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    path: UserPtr<u8>,
    flags: i32,
    mode: Mode,
) -> Result<i32, Errno> {
    let rw = flags & O_ACCMODE;

    // libc only calls with O_RDONLY, O_RDWR, O_CLOEXEC, O_CLOFORK
    if flags & !(O_ACCMODE | O_CLOEXEC | O_CLOFORK) != 0 || !(rw == O_RDONLY || rw == O_RDWR) {
        return Err(Errno::EINVAL);
    }

    doopenat(p, AT_FDCWD, path, flags, mode, UNVEIL_PLEDGEOPEN)
}

/// `openat(fd, path, flags, mode)`: the new descriptor.
pub fn sys_openat<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    path: UserPtr<u8>,
    flags: i32,
    mode: Mode,
) -> Result<i32, Errno> {
    doopenat(p, fd, path, flags, mode, 0)
}

/// A `struct flock` that locks the whole file (`flock(2)` semantics) for `O_EXLOCK` or
/// `O_SHLOCK`, and the `VOP_ADVLOCK` flags to go with it.
fn open_flock(flags: i32) -> (Flock, i32) {
    let lf = Flock {
        l_whence: SEEK_SET as i16,
        l_start: 0,
        l_len: 0,
        l_type: if flags & O_EXLOCK != 0 {
            F_WRLCK
        } else {
            F_RDLCK
        },
        ..Flock::default()
    };
    let mut type_ = F_FLOCK;
    if flags & FNONBLOCK == 0 {
        type_ |= F_WAIT;
    }
    (lf, type_)
}

/// The `UF_*` flags of a new descriptor: close-on-exec and close-on-fork.
fn open_fdflags(flags: i32) -> u8 {
    (if flags & O_CLOEXEC != 0 {
        UF_EXCLOSE
    } else {
        0
    }) | (if flags & O_CLOFORK != 0 {
        UF_FORKCLOSE
    } else {
        0
    })
}

/// `doopenat(p, fd, path, oflags, mode, pledgeopen, retval)`: the common body of `open`,
/// `openat` and `__pledge_open`; the new descriptor.
pub fn doopenat<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    path: UserPtr<u8>,
    oflags: i32,
    mode: Mode,
    pledgeopen: u8,
) -> Result<i32, Errno> {
    let fdp = p.fd();
    let mut ni_pledge = 0;
    let mut ni_unveil = pledgeopen;
    let mut localtrunc = false;

    if oflags & (O_EXLOCK | O_SHLOCK) != 0 {
        G::pledge_flock(p)?;
    }

    let mut fdflags = open_fdflags(oflags);

    let mut g = fdplock(fdp);
    let (fp, indx) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(e) => {
            fdpunlock(g);
            return Err(e);
        }
    };
    fdpunlock(g);

    let mut flags = fflags(oflags);
    if flags & FREAD != 0 {
        ni_pledge |= PLEDGE_RPATH;
        ni_unveil |= UNVEIL_READ;
    }
    if flags & FWRITE != 0 {
        ni_pledge |= PLEDGE_WPATH;
        ni_unveil |= UNVEIL_WRITE;
    }
    if oflags & O_CREAT != 0 {
        ni_pledge |= PLEDGE_CPATH;
        ni_unveil |= UNVEIL_CREATE;
    }

    let mut cmode = ((mode & !fdp.cmask()) & ALLPERMS) & !S_ISTXT;
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0 {
        cmode &= ACCESSPERMS;
    }
    let mut nd = ndinitat(0, 0, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = ni_pledge;
    nd.ni_unveil = ni_unveil;
    p.p_dupfd.set(-1); // XXX check for fdopen
    if flags & O_TRUNC != 0 && flags & (O_EXLOCK | O_SHLOCK) != 0 {
        localtrunc = true;
        flags &= !O_TRUNC; // Must do truncate ourselves
    }
    klock::kernel_lock();
    let (error, g) = 'error: {
        if let Err(error) = vn_open(&mut nd, flags, cmode) {
            let mut g = fdplock(fdp);
            if error == Errno::ENODEV && p.p_dupfd.get() >= 0 {
                // XXX from fdopen
                match dupfdopen(p, &mut g, indx, flags) {
                    Ok(()) => break 'error (Ok(indx), g),
                    Err(e) => {
                        let _empty = fdremove(&mut g, indx);
                        break 'error (Err(e), g);
                    }
                }
            }
            let error = if error == Errno::ERESTART {
                Errno::EINTR
            } else {
                error
            };
            let _empty = fdremove(&mut g, indx);
            break 'error (Err(error), g);
        }
        p.p_dupfd.set(0);
        let vp = ndvp(nd.ni_vp);
        if pledgeopen != 0 && vp.vtype() != VCHR {
            fdflags |= UF_PLEDGEOPEN;
        }
        fp.f_flag.store((flags & FMASK) as u32, Ordering::SeqCst);
        let (ops, data) = G::vnode_file(vp);
        fp.install(DTYPE_VNODE, ops, data);
        if flags & (O_EXLOCK | O_SHLOCK) != 0 {
            let (mut lf, type_) = open_flock(flags);
            let _ = vop_unlock(vp);
            let id = LockOwner::File(fp.addr());
            if let Err(error) = vop_advlock(vp, id, F_SETLK, &mut lf, type_) {
                let mut g = fdplock(fdp);
                // closef will vn_close the file for us.
                let _empty = fdremove(&mut g, indx);
                break 'error (Err(error), g);
            }
            let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
            fp.f_iflags.fetch_or(FIF_HASLOCK, Ordering::SeqCst);
        }
        if localtrunc {
            let error = if fp.flag() & FWRITE == 0 {
                Err(Errno::EACCES)
            } else if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
                Err(Errno::EROFS)
            } else if vp.vtype() == VDIR {
                Err(Errno::EISDIR)
            } else {
                vn_writechk(vp).and_then(|()| {
                    let mut vattr = Vattr::new();
                    vattr_null(&mut vattr);
                    vattr.va_size = 0;
                    vop_setattr(vp, &mut vattr, Cred::real(fp.cred()), p)
                })
            };
            if let Err(error) = error {
                let _ = vop_unlock(vp);
                let mut g = fdplock(fdp);
                // closef will close the file for us.
                let _empty = fdremove(&mut g, indx);
                break 'error (Err(error), g);
            }
        }
        let _ = vop_unlock(vp);
        klock::kernel_unlock();
        let mut g = fdplock(fdp);
        fdinsert(&mut g, indx, fdflags, &fp);
        fdpunlock(g);
        let _ = fp.frele(Some(p));
        return Ok(indx);
    };
    // error:
    klock::kernel_unlock();
    fdpunlock(g);
    let _ = closef(fp, Some(p));
    error
}

/// Get file handle system call.
pub fn sys_getfh<G: ProcGlue>(
    p: &Thread<G>,
    fname: UserPtr<u8>,
    fhp: UserPtr<Fhandle>,
) -> Result<(), Errno> {
    // Must be super user
    suser(p)?;
    let mut nd = ndinit(LOOKUP, FOLLOW | LOCKLEAF, NiDirp::User(fname.addr()), p);
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let mut fh = Fhandle {
        fh_fsid: vp.mount().map_or(Fsid::default(), Mount::fsid),
        ..Fhandle::default()
    };
    let error = VFS_VPTOFH(vp, &mut fh.fh_fid);
    vput(vp);
    error?;
    fhp.copyout(&fh)
}

/// Open a file given a file handle: the new descriptor.
///
/// Check permissions, allocate an open file structure, and call the device open routine if
/// any.
pub fn sys_fhopen<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fhp: UserPtr<Fhandle>,
    flags: i32,
) -> Result<i32, Errno> {
    let fdp = p.fd();
    let cred = Cred::real(p.ucred());

    // Must be super user
    suser(p)?;

    let flags = fflags(flags);
    if flags & (FREAD | FWRITE) == 0 {
        return Err(Errno::EINVAL);
    }
    if flags & O_CREAT != 0 {
        return Err(Errno::EINVAL);
    }

    let fdflags = open_fdflags(flags);

    let mut g = fdplock(fdp);
    let (fp, indx) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(e) => {
            fdpunlock(g);
            return Err(e);
        }
    };
    fdpunlock(g);

    let mut vp: Option<&'static Vnode<P>> = None;
    let error: Errno = 'bad: {
        let fh: Fhandle = match fhp.copyin() {
            Ok(fh) => fh,
            Err(e) => break 'bad e,
        };

        let Some(mp) = vfs_getvfs::<P>(&fh.fh_fsid) else {
            break 'bad Errno::ESTALE;
        };

        let v = match VFS_FHTOVP(mp, &fh.fh_fid) {
            Ok(v) => v,
            Err(e) => break 'bad e, // most likely unnecessary sanity for bad:
        };
        vp = Some(v);

        // Now do an effective vn_open
        if v.vtype() == VSOCK {
            break 'bad Errno::EOPNOTSUPP;
        }
        if flags & O_DIRECTORY != 0 && v.vtype() != VDIR {
            break 'bad Errno::ENOTDIR;
        }
        if flags & FREAD != 0
            && let Err(e) = vop_access(v, VREAD, cred, p)
        {
            break 'bad e;
        }
        if flags & (FWRITE | O_TRUNC) != 0 {
            if v.vtype() == VDIR {
                break 'bad Errno::EISDIR;
            }
            if let Err(e) = vop_access(v, VWRITE, cred, p).and_then(|()| vn_writechk(v)) {
                break 'bad e;
            }
        }
        if flags & O_TRUNC != 0 {
            let mut va = Vattr::new();
            vattr_null(&mut va);
            va.va_size = 0;
            if let Err(e) = vop_setattr(v, &mut va, cred, p) {
                break 'bad e;
            }
        }
        if let Err(e) = vop_open(v, flags, cred, p) {
            break 'bad e;
        }
        if flags & FWRITE != 0 {
            v.set_writecount(v.writecount() + 1);
        }

        // done with modified vn_open, now finish what sys_open does.
        fp.f_flag.store((flags & FMASK) as u32, Ordering::SeqCst);
        let (ops, data) = G::vnode_file(v);
        fp.install(DTYPE_VNODE, ops, data);
        if flags & (O_EXLOCK | O_SHLOCK) != 0 {
            let (mut lf, type_) = open_flock(flags);
            let _ = vop_unlock(v);
            let id = LockOwner::File(fp.addr());
            if let Err(e) = vop_advlock(v, id, F_SETLK, &mut lf, type_) {
                vp = None; // closef will vn_close the file
                break 'bad e;
            }
            let _ = vn_lock(v, LK_EXCLUSIVE | LK_RETRY);
            fp.f_iflags.fetch_or(FIF_HASLOCK, Ordering::SeqCst);
        }
        let _ = vop_unlock(v);
        let mut g = fdplock(fdp);
        fdinsert(&mut g, indx, fdflags, &fp);
        fdpunlock(g);
        let _ = fp.frele(Some(p));
        return Ok(indx);
    };

    // bad:
    let mut g = fdplock(fdp);
    let empty = fdremove(&mut g, indx);
    fdpunlock(g);
    drop(empty);
    let _ = closef(fp, Some(p));
    if let Some(vp) = vp {
        vput(vp);
    }
    Err(error)
}

/// `fhstat(fhp, sb)`: `stat(2)` of a file handle.
pub fn sys_fhstat<G: ProcGlue>(
    p: &Thread<G>,
    fhp: UserPtr<Fhandle>,
    sb: UserPtr<u8>,
) -> Result<(), Errno> {
    let mut st = Stat::default();

    // Must be super user
    suser(p)?;

    let fh: Fhandle = fhp.copyin()?;

    let Some(mp) = vfs_getvfs::<G>(&fh.fh_fsid) else {
        return Err(Errno::ESTALE);
    };
    let vp = VFS_FHTOVP(mp, &fh.fh_fid)?;
    let error = vn_stat(vp, &mut st, p);
    vput(vp);
    error?;
    sb.copyout_slice(&st.to_bytes())
}

/// `fhstatfs(fhp, buf)`: `statfs(2)` of a file handle (the file system id is not hidden: the
/// caller is root).
pub fn sys_fhstatfs<G: ProcGlue>(
    p: &Thread<G>,
    fhp: UserPtr<Fhandle>,
    buf: UserPtr<u8>,
) -> Result<(), Errno> {
    // Must be super user
    suser(p)?;

    let fh: Fhandle = fhp.copyin()?;

    let Some(mp) = vfs_getvfs::<G>(&fh.fh_fsid) else {
        return Err(Errno::ESTALE);
    };
    let vp = VFS_FHTOVP(mp, &fh.fh_fid)?;
    let mp = vp.mount();
    vput(vp);
    let Some(mp) = mp else {
        return Err(Errno::ESTALE);
    };
    statfs_mnt(mp, p)?;
    statfs_flags(mp);
    buf.copyout_slice(bytes_of(slice::from_ref(&mp.stat())))
}

/// Create a special file or named pipe.
pub fn sys_mknod<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    mode: Mode,
    dev: Dev,
) -> Result<(), Errno> {
    domknodat(p, AT_FDCWD, path, mode, dev)
}

/// `mknodat(fd, path, mode, dev)`.
pub fn sys_mknodat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    mode: Mode,
    dev: Dev,
) -> Result<(), Errno> {
    domknodat(p, fd, path, mode, dev)
}

/// `domknodat(p, fd, path, mode, dev)`: the common body of `mknod`, `mknodat` and `mkfifo`.
pub fn domknodat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    mode: Mode,
    dev: Dev,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    if dev == VNOVAL {
        return Err(Errno::EINVAL);
    }
    let mut nd = ndinitat(CREATE, LOCKPARENT, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_DPATH;
    nd.ni_unveil = UNVEIL_CREATE;
    namei(&mut nd)?;
    let vp = nd.ni_vp;
    let dvp = ndvp(nd.ni_dvp);
    let error = 'out: {
        if mode & S_IFMT != S_IFIFO || dev != 0 {
            if !vnoperm(dvp)
                && let Err(e) = suser(p)
            {
                break 'out Err(e);
            }
            if p.fd().rdir().is_some() {
                break 'out Err(Errno::EINVAL);
            }
        }
        if vp.is_some() {
            break 'out Err(Errno::EEXIST);
        }
        vattr_null(&mut vattr);
        vattr.va_mode = (mode & ALLPERMS) & !p.fd().cmask();
        if p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0 {
            vattr.va_mode &= ACCESSPERMS;
        }
        vattr.va_rdev = dev;

        match mode & S_IFMT {
            S_IFMT => {
                // used by badsect to flag bad sectors
                vattr.va_type = VBAD;
                Ok(())
            }
            S_IFCHR => {
                vattr.va_type = VCHR;
                Ok(())
            }
            S_IFBLK => {
                vattr.va_type = VBLK;
                Ok(())
            }
            // option FIFO is not configured (see the module's deviations).
            S_IFIFO => Err(Errno::EOPNOTSUPP),
            _ => Err(Errno::EINVAL),
        }
    };
    match error {
        Ok(()) => {
            let error = vop_mknod(dvp, &mut nd.ni_vp, &mut nd.ni_cnd, &mut vattr);
            vput(dvp);
            error
        }
        Err(e) => {
            let _ = vop_abortop(dvp, &mut nd.ni_cnd);
            if vp.is_some_and(|vp| ptr::eq(vp, dvp)) {
                vrele(dvp);
            } else {
                vput(dvp);
            }
            if let Some(vp) = vp {
                vrele(vp);
            }
            Err(e)
        }
    }
}

/// Create a named pipe.
pub fn sys_mkfifo<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>, mode: Mode) -> Result<(), Errno> {
    domknodat(p, AT_FDCWD, path, (mode & ALLPERMS) | S_IFIFO, 0)
}

/// `mkfifoat(fd, path, mode)`.
pub fn sys_mkfifoat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    mode: Mode,
) -> Result<(), Errno> {
    domknodat(p, fd, path, (mode & ALLPERMS) | S_IFIFO, 0)
}

/// Make a hard file link.
pub fn sys_link<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    link: UserPtr<u8>,
) -> Result<(), Errno> {
    dolinkat(p, AT_FDCWD, path, AT_FDCWD, link, AT_SYMLINK_FOLLOW)
}

/// `linkat(fd1, path1, fd2, path2, flag)`.
pub fn sys_linkat<G: ProcGlue>(
    p: &Thread<G>,
    fd1: i32,
    path1: UserPtr<u8>,
    fd2: i32,
    path2: UserPtr<u8>,
    flag: i32,
) -> Result<(), Errno> {
    dolinkat(p, fd1, path1, fd2, path2, flag)
}

/// `dolinkat(p, fd1, path1, fd2, path2, flag)`.
pub fn dolinkat<G: ProcGlue>(
    p: &Thread<G>,
    fd1: i32,
    path1: UserPtr<u8>,
    fd2: i32,
    path2: UserPtr<u8>,
    flag: i32,
) -> Result<(), Errno> {
    if flag & !AT_SYMLINK_FOLLOW != 0 {
        return Err(Errno::EINVAL);
    }

    let follow = if flag & AT_SYMLINK_FOLLOW != 0 {
        FOLLOW
    } else {
        NOFOLLOW
    };
    let mut nd = ndinitat(LOOKUP, follow, fd1, NiDirp::User(path1.addr()), p);
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);

    let error = 'out: {
        if vp.vtype() == VDIR {
            break 'out Err(Errno::EPERM);
        }

        let mut nd = ndinitat(CREATE, LOCKPARENT, fd2, NiDirp::User(path2.addr()), p);
        nd.ni_pledge = PLEDGE_CPATH;
        nd.ni_unveil = UNVEIL_CREATE;
        if let Err(e) = namei(&mut nd) {
            break 'out Err(e);
        }
        let dvp = ndvp(nd.ni_dvp);
        if let Some(nvp) = nd.ni_vp {
            let _ = vop_abortop(dvp, &mut nd.ni_cnd);
            if ptr::eq(dvp, nvp) {
                vrele(dvp);
            } else {
                vput(dvp);
            }
            vrele(nvp);
            break 'out Err(Errno::EEXIST);
        }

        // No cross-mount links!
        if !opt_mount_eq(dvp, vp) {
            let _ = vop_abortop(dvp, &mut nd.ni_cnd);
            vput(dvp);
            break 'out Err(Errno::EXDEV);
        }

        vop_link(dvp, vp, &mut nd.ni_cnd)
    };
    vrele(vp);
    error
}

/// `a->v_mount == b->v_mount`.
fn opt_mount_eq<G: ProcGlue>(a: &Vnode<G>, b: &Vnode<G>) -> bool {
    match (a.mount(), b.mount()) {
        (Some(x), Some(y)) => ptr::eq(x, y),
        (None, None) => true,
        _ => false,
    }
}

/// Make a symbolic link.
pub fn sys_symlink<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    link: UserPtr<u8>,
) -> Result<(), Errno> {
    dosymlinkat(p, path, AT_FDCWD, link)
}

/// `symlinkat(path, fd, link)`.
pub fn sys_symlinkat<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    fd: i32,
    link: UserPtr<u8>,
) -> Result<(), Errno> {
    dosymlinkat(p, path, fd, link)
}

/// `dosymlinkat(p, upath, fd, link)`: the link `link` (relative to `fd`) to the target
/// `upath`.
pub fn dosymlinkat<G: ProcGlue>(
    p: &Thread<G>,
    upath: UserPtr<u8>,
    fd: i32,
    link: UserPtr<u8>,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    let mut path = pathbuf_get::<G>().ok_or(Errno::ENOMEM)?;
    let len = upath.copyinstr(&mut path[..])?;
    let mut nd = ndinitat(CREATE, LOCKPARENT, fd, NiDirp::User(link.addr()), p);
    nd.ni_pledge = PLEDGE_CPATH;
    nd.ni_unveil = UNVEIL_CREATE;
    namei(&mut nd)?;
    let dvp = ndvp(nd.ni_dvp);
    if let Some(vp) = nd.ni_vp {
        let _ = vop_abortop(dvp, &mut nd.ni_cnd);
        if ptr::eq(dvp, vp) {
            vrele(dvp);
        } else {
            vput(dvp);
        }
        vrele(vp);
        return Err(Errno::EEXIST);
    }
    vattr_null(&mut vattr);
    vattr.va_mode = ACCESSPERMS & !p.fd().cmask();
    vop_symlink(
        dvp,
        &mut nd.ni_vp,
        &mut nd.ni_cnd,
        &mut vattr,
        &path[..len - 1],
    )
}

/// Delete a name from the filesystem.
pub fn sys_unlink<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>) -> Result<(), Errno> {
    dounlinkat(p, AT_FDCWD, path, 0)
}

/// `unlinkat(fd, path, flag)`.
pub fn sys_unlinkat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    flag: i32,
) -> Result<(), Errno> {
    dounlinkat(p, fd, path, flag)
}

/// `dounlinkat(p, fd, path, flag)`: the common body of `unlink`, `unlinkat` and `rmdir`.
pub fn dounlinkat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    flag: i32,
) -> Result<(), Errno> {
    if flag & !AT_REMOVEDIR != 0 {
        return Err(Errno::EINVAL);
    }

    let mut nd = ndinitat(
        DELETE,
        LOCKPARENT | LOCKLEAF,
        fd,
        NiDirp::User(path.addr()),
        p,
    );
    nd.ni_pledge = PLEDGE_CPATH;
    nd.ni_unveil = UNVEIL_CREATE;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let dvp = ndvp(nd.ni_dvp);

    let error = 'out: {
        if flag & AT_REMOVEDIR != 0 {
            if vp.vtype() != VDIR {
                break 'out Err(Errno::ENOTDIR);
            }
            // No rmdir "." please.
            if ptr::eq(dvp, vp) {
                break 'out Err(Errno::EINVAL);
            }
            // A mounted on directory cannot be deleted.
            if vp.mountedhere().is_some() {
                break 'out Err(Errno::EBUSY);
            }
        }

        // The root of a mounted filesystem cannot be deleted.
        if vp.flag() & VROOT != 0 {
            break 'out Err(Errno::EBUSY);
        }
        Ok(())
    };
    match error {
        Ok(()) => {
            if flag & AT_REMOVEDIR != 0 {
                vop_rmdir(dvp, vp, &mut nd.ni_cnd)
            } else {
                let _ = uvm_vnp_uncache(vp);
                vop_remove(dvp, vp, &mut nd.ni_cnd)
            }
        }
        Err(e) => {
            let _ = vop_abortop(dvp, &mut nd.ni_cnd);
            if ptr::eq(dvp, vp) {
                vrele(dvp);
            } else {
                vput(dvp);
            }
            vput(vp);
            Err(e)
        }
    }
}

/// Reposition read/write file offset: the new offset.
pub fn sys_lseek<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    offset: Off,
    whence: i32,
) -> Result<Off, Errno> {
    let Some(fp) = fd_getfile(p.fd(), fd) else {
        return Err(Errno::EBADF);
    };
    // A file without fo_seek answers ESPIPE (FileOps::seek's default).
    let mut offset = offset;
    let error = fp.ops().seek(&fp, &mut offset, whence, p);
    if error.is_ok() {
        fp.stats().f_seek += 1;
    }
    let _ = fp.frele(Some(p));
    error.map(|()| offset)
}

/// Check access permissions.
pub fn sys_access<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>, amode: i32) -> Result<(), Errno> {
    dofaccessat(p, AT_FDCWD, path, amode, 0)
}

/// `faccessat(fd, path, amode, flag)`.
pub fn sys_faccessat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    amode: i32,
    flag: i32,
) -> Result<(), Errno> {
    dofaccessat(p, fd, path, amode, flag)
}

/// `dofaccessat(p, fd, path, amode, flag)`: the common body of `access` and `faccessat`.
pub fn dofaccessat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    amode: i32,
    flag: i32,
) -> Result<(), Errno> {
    let mut vflags = 0;

    if amode & !(R_OK | W_OK | X_OK) != 0 {
        return Err(Errno::EINVAL);
    }
    if flag & !AT_EACCESS != 0 {
        return Err(Errno::EINVAL);
    }

    // The thread's own reference while it borrows the real ids (`oldcred`).
    let mut oldcred = None;
    let old = p.ucred();

    // If access as real ids was requested and they really differ, give the thread new creds
    // with them reset
    if flag & AT_EACCESS == 0
        && (old.cr_uid.get() != old.cr_ruid.get() || old.cr_gid.get() != old.cr_rgid.get())
    {
        let mut cr = old.clone();
        let (ruid, rgid) = (cr.cr_ruid.get(), cr.cr_rgid.get());
        cr.cr_uid.set(ruid);
        cr.cr_gid.set(rgid);
        oldcred = p.set_cred(CredRef::new(cr));
    }

    let mut nd = ndinitat(LOOKUP, FOLLOW | LOCKLEAF, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    if amode & R_OK != 0 {
        vflags |= VREAD;
    }
    if amode & W_OK != 0 {
        vflags |= VWRITE;
        nd.ni_unveil |= UNVEIL_WRITE;
    }
    if amode & X_OK != 0 {
        vflags |= VEXEC;
    }
    let error = namei(&mut nd).and_then(|()| {
        let vp = ndvp(nd.ni_vp);

        // Flags == 0 means only check for existence.
        let mut error = Ok(());
        if amode != 0 {
            error = vop_access(vp, vflags, Cred::real(p.ucred()), p);
            if error.is_ok() && vflags & VWRITE != 0 {
                error = vn_writechk(vp);
            }
        }
        vput(vp);
        error
    });
    // out: the thread's credentials back; the borrowed ones go.
    if let Some(oldcred) = oldcred {
        drop(p.set_cred(oldcred));
    }
    error
}

/// Get file status; this version follows links.
pub fn sys_stat<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    ub: UserPtr<u8>,
) -> Result<(), Errno> {
    dofstatat(p, AT_FDCWD, path, ub, 0)
}

/// `fstatat(fd, path, buf, flag)`.
pub fn sys_fstatat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    buf: UserPtr<u8>,
    flag: i32,
) -> Result<(), Errno> {
    dofstatat(p, fd, path, buf, flag)
}

/// `dofstatat(p, fd, path, buf, flag)`: the common body of `stat`, `lstat` and `fstatat`.
pub fn dofstatat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    buf: UserPtr<u8>,
    flag: i32,
) -> Result<(), Errno> {
    let mut sb = Stat::default();

    if flag & !AT_SYMLINK_NOFOLLOW != 0 {
        return Err(Errno::EINVAL);
    }

    let follow = if flag & AT_SYMLINK_NOFOLLOW != 0 {
        NOFOLLOW
    } else {
        FOLLOW
    };
    let mut nd = ndinitat(LOOKUP, follow | LOCKLEAF, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    klock::kernel_lock();
    if let Err(e) = namei(&mut nd) {
        klock::kernel_unlock();
        return Err(e);
    }
    let vp = ndvp(nd.ni_vp);
    let error = vn_stat(vp, &mut sb, p);
    vput(vp);
    klock::kernel_unlock();
    error?;
    // Don't let non-root see generation numbers (for NFS security)
    if suser(p).is_err() {
        sb.st_gen = 0;
    }
    // KTRACE: not configured.
    buf.copyout_slice(&sb.to_bytes())
}

/// Get file status; this version does not follow links.
pub fn sys_lstat<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    ub: UserPtr<u8>,
) -> Result<(), Errno> {
    dofstatat(p, AT_FDCWD, path, ub, AT_SYMLINK_NOFOLLOW)
}

/// Get configurable pathname variables: the value (`*retval`).
pub fn sys_pathconf<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    name: i32,
) -> Result<Register, Errno> {
    dopathconfat(p, AT_FDCWD, path, name, 0)
}

/// `pathconfat(fd, path, name, flag)`: the value.
pub fn sys_pathconfat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    name: i32,
    flag: i32,
) -> Result<Register, Errno> {
    dopathconfat(p, fd, path, name, flag)
}

/// `dopathconfat(p, fd, path, name, flag, retval)`: the value.
pub fn dopathconfat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    name: i32,
    flag: i32,
) -> Result<Register, Errno> {
    if flag & !AT_SYMLINK_NOFOLLOW != 0 {
        return Err(Errno::EINVAL);
    }

    let follow = if flag & AT_SYMLINK_NOFOLLOW != 0 {
        NOFOLLOW
    } else {
        FOLLOW
    };
    let mut nd = ndinitat(LOOKUP, follow | LOCKLEAF, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let mut retval: Register = 0;
    let error = vop_pathconf(vp, name, &mut retval);
    vput(vp);
    error.map(|()| retval)
}

/// Return target name of a symbolic link: the bytes placed in `buf`.
pub fn sys_readlink<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    buf: usize,
    count: usize,
) -> Result<usize, Errno> {
    doreadlinkat(p, AT_FDCWD, path, buf, count)
}

/// `readlinkat(fd, path, buf, count)`: the bytes placed in `buf`.
pub fn sys_readlinkat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    buf: usize,
    count: usize,
) -> Result<usize, Errno> {
    doreadlinkat(p, fd, path, buf, count)
}

/// A user transfer of `len` bytes at the user address `base` for `p`, from `offset`.
fn user_read_uio<'a, G: ProcGlue>(
    p: &'a Thread<G>,
    aiov: &'a mut [Iovec; 1],
    offset: Off,
) -> Uio<'a, G> {
    let resid = aiov[0].iov_len;
    Uio {
        uio_iov: aiov,
        uio_offset: offset,
        uio_resid: resid,
        uio_segflg: UioSeg::UIO_USERSPACE,
        uio_rw: UioRw::UIO_READ,
        uio_procp: Some(p),
    }
}

/// `doreadlinkat(p, fd, path, buf, count, retval)`: the bytes placed in `buf`.
pub fn doreadlinkat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    buf: usize,
    count: usize,
) -> Result<usize, Errno> {
    let mut nd = ndinitat(
        LOOKUP,
        NOFOLLOW | LOCKLEAF,
        fd,
        NiDirp::User(path.addr()),
        p,
    );
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let error = if vp.vtype() == VLNK {
        let mut aiov = [Iovec {
            iov_base: ptr::without_provenance_mut(buf),
            iov_len: count,
        }];
        let mut auio = user_read_uio(p, &mut aiov, 0);
        let error = vop_readlink(vp, &mut auio, Cred::real(p.ucred()));
        error.map(|()| count - auio.uio_resid)
    } else {
        Err(Errno::EINVAL)
    };
    vput(vp);
    error
}

/// Change flags of a file given a path name.
pub fn sys_chflags<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>, flags: u32) -> Result<(), Errno> {
    dochflagsat(p, AT_FDCWD, path, flags, 0)
}

/// `chflagsat(fd, path, flags, atflags)`.
pub fn sys_chflagsat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    flags: u32,
    atflags: i32,
) -> Result<(), Errno> {
    dochflagsat(p, fd, path, flags, atflags)
}

/// `dochflagsat(p, fd, path, flags, atflags)`.
pub fn dochflagsat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    flags: u32,
    atflags: i32,
) -> Result<(), Errno> {
    if atflags & !AT_SYMLINK_NOFOLLOW != 0 {
        return Err(Errno::EINVAL);
    }

    let follow = if atflags & AT_SYMLINK_NOFOLLOW != 0 {
        NOFOLLOW
    } else {
        FOLLOW
    };
    let mut nd = ndinitat(LOOKUP, follow, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_FATTR | PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_WRITE;
    namei(&mut nd)?;
    dovchflags(p, ndvp(nd.ni_vp), flags)
}

/// Change flags of a file given a file descriptor.
pub fn sys_fchflags<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    flags: u32,
) -> Result<(), Errno> {
    let fp = getvnode(p, fd)?;
    if p.fd().ofileflags(fd) & UF_PLEDGEOPEN != 0 {
        let _ = fp.frele(Some(p));
        return Err(Errno::EPERM);
    }
    let vp = file_vp(&fp);
    vref(vp);
    let _ = fp.frele(Some(p));
    dovchflags(p, vp, flags)
}

/// `dovchflags(p, vp, flags)`: sets the file flags of a referenced, unlocked vnode, and
/// releases it.
pub fn dovchflags<G: ProcGlue>(
    p: &Thread<G>,
    vp: &'static Vnode<G>,
    flags: u32,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = 'out: {
        if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
            break 'out Err(Errno::EROFS);
        } else if flags == VNOVAL as u32 {
            break 'out Err(Errno::EINVAL);
        }
        if suser(p).is_err() {
            if let Err(e) = vop_getattr(vp, &mut vattr, Cred::real(p.ucred()), p) {
                break 'out Err(e);
            }
            if vattr.va_type == VCHR || vattr.va_type == VBLK {
                break 'out Err(Errno::EINVAL);
            }
        }
        vattr_null(&mut vattr);
        vattr.va_flags = u64::from(flags);
        vop_setattr(vp, &mut vattr, Cred::real(p.ucred()), p)
    };
    vput(vp);
    error
}

/// Change mode of a file given path name.
pub fn sys_chmod<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>, mode: Mode) -> Result<(), Errno> {
    dofchmodat(p, AT_FDCWD, path, mode, 0)
}

/// `fchmodat(fd, path, mode, flag)`.
pub fn sys_fchmodat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    mode: Mode,
    flag: i32,
) -> Result<(), Errno> {
    dofchmodat(p, fd, path, mode, flag)
}

/// `dofchmodat(p, fd, path, mode, flag)`.
pub fn dofchmodat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    mut mode: Mode,
    flag: i32,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    if mode & !(S_IFMT | ALLPERMS) != 0 {
        return Err(Errno::EINVAL);
    }
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0 {
        mode &= ACCESSPERMS;
    }
    if flag & !AT_SYMLINK_NOFOLLOW != 0 {
        return Err(Errno::EINVAL);
    }

    let follow = if flag & AT_SYMLINK_NOFOLLOW != 0 {
        NOFOLLOW
    } else {
        FOLLOW
    };
    let mut nd = ndinitat(LOOKUP, follow, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_FATTR | PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_WRITE;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
        Err(Errno::EROFS)
    } else {
        vattr_null(&mut vattr);
        vattr.va_mode = mode & ALLPERMS;
        vop_setattr(vp, &mut vattr, Cred::real(p.ucred()), p)
    };
    vput(vp);
    error
}

/// Change mode of a file given a file descriptor.
pub fn sys_fchmod<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    mode: Mode,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();
    let mut mode = mode;

    if mode & !(S_IFMT | ALLPERMS) != 0 {
        return Err(Errno::EINVAL);
    }
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0 {
        mode &= ACCESSPERMS;
    }

    let fp = getvnode(p, fd)?;
    let vp = file_vp(&fp);
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
        Err(Errno::EROFS)
    } else if p.fd().ofileflags(fd) & UF_PLEDGEOPEN != 0 {
        Err(Errno::EPERM)
    } else {
        vattr_null(&mut vattr);
        vattr.va_mode = mode & ALLPERMS;
        vop_setattr(vp, &mut vattr, Cred::real(p.ucred()), p)
    };
    let _ = vop_unlock(vp);
    let _ = fp.frele(Some(p));
    error
}

/// Set ownership given a path name.
pub fn sys_chown<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    uid: Uid,
    gid: Gid,
) -> Result<(), Errno> {
    dofchownat(p, AT_FDCWD, path, uid, gid, 0)
}

/// `fchownat(fd, path, uid, gid, flag)`.
pub fn sys_fchownat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    uid: Uid,
    gid: Gid,
    flag: i32,
) -> Result<(), Errno> {
    dofchownat(p, fd, path, uid, gid, flag)
}

/// The ownership change on a locked vnode the `chown` family shares (the C writes it out in
/// each): `pledge_chown` first, the set-id bits cleared when the owner or group changes on a
/// file system that checks permissions, then `VOP_SETATTR`.
fn chown_vnode<G: ProcGlue>(
    p: &Thread<G>,
    vp: &'static Vnode<G>,
    uid: Uid,
    gid: Gid,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    G::pledge_chown(p, uid, gid)?;
    let mode = if (uid != VNOVAL as Uid || gid != VNOVAL as Gid) && !vnoperm(vp) {
        vop_getattr(vp, &mut vattr, Cred::real(p.ucred()), p)?;
        let mode = vattr.va_mode & !(VSUID | VSGID);
        if mode == vattr.va_mode {
            VNOVAL as Mode
        } else {
            mode
        }
    } else {
        VNOVAL as Mode
    };
    vattr_null(&mut vattr);
    vattr.va_uid = uid;
    vattr.va_gid = gid;
    vattr.va_mode = mode;
    vop_setattr(vp, &mut vattr, Cred::real(p.ucred()), p)
}

/// `dofchownat(p, fd, path, uid, gid, flag)`.
pub fn dofchownat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    uid: Uid,
    gid: Gid,
    flag: i32,
) -> Result<(), Errno> {
    if flag & !AT_SYMLINK_NOFOLLOW != 0 {
        return Err(Errno::EINVAL);
    }

    let follow = if flag & AT_SYMLINK_NOFOLLOW != 0 {
        NOFOLLOW
    } else {
        FOLLOW
    };
    let mut nd = ndinitat(LOOKUP, follow, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_CHOWN | PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_WRITE;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
        Err(Errno::EROFS)
    } else {
        chown_vnode(p, vp, uid, gid)
    };
    vput(vp);
    error
}

/// Set ownership given a path name, without following links.
pub fn sys_lchown<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    uid: Uid,
    gid: Gid,
) -> Result<(), Errno> {
    let mut nd = ndinit(LOOKUP, NOFOLLOW, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_CHOWN | PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_WRITE;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
        Err(Errno::EROFS)
    } else {
        chown_vnode(p, vp, uid, gid)
    };
    vput(vp);
    error
}

/// Set ownership given a file descriptor.
pub fn sys_fchown<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    uid: Uid,
    gid: Gid,
) -> Result<(), Errno> {
    let fp = getvnode(p, fd)?;
    let vp = file_vp(&fp);
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
        Err(Errno::EROFS)
    } else if p.fd().ofileflags(fd) & UF_PLEDGEOPEN != 0 {
        Err(Errno::EPERM)
    } else {
        chown_vnode(p, vp, uid, gid)
    };
    let _ = vop_unlock(vp);
    let _ = fp.frele(Some(p));
    error
}

/// The `utimes(2)`-style times: the user's two timevals, or `UTIME_NOW` for both.
fn utimes_times(tptr: UserPtr<[Timeval; 2]>) -> Result<[Timespec; 2], Errno> {
    let mut ts = [Timespec::new(0, 0); 2];
    if tptr.is_null() {
        ts[0].tv_nsec = UTIME_NOW;
        ts[1].tv_nsec = UTIME_NOW;
    } else {
        let tv = tptr.copyin()?;
        // KTRACE: not configured.
        if !tv[0].is_valid() || !tv[1].is_valid() {
            return Err(Errno::EINVAL);
        }
        ts[0] = tv[0].to_timespec();
        ts[1] = tv[1].to_timespec();
    }
    Ok(ts)
}

/// The `utimensat(2)`-style times: the user's two timespecs (each possibly `UTIME_NOW` or
/// `UTIME_OMIT`), or `UTIME_NOW` for both.
fn utimens_times(tsp: UserPtr<[Timespec; 2]>) -> Result<[Timespec; 2], Errno> {
    let mut ts = [Timespec::new(0, 0); 2];
    if tsp.is_null() {
        ts[0].tv_nsec = UTIME_NOW;
        ts[1].tv_nsec = UTIME_NOW;
    } else {
        ts = tsp.copyin()?;
        for t in &ts {
            if t.tv_nsec == UTIME_NOW || t.tv_nsec == UTIME_OMIT {
                continue;
            }
            // KTRACE: not configured.
            if !t.is_valid() {
                return Err(Errno::EINVAL);
            }
        }
    }
    Ok(ts)
}

/// Set the access and modification times given a path name.
pub fn sys_utimes<G: ProcGlue>(
    p: &Thread<G>,
    path: UserPtr<u8>,
    tptr: UserPtr<[Timeval; 2]>,
) -> Result<(), Errno> {
    let mut ts = utimes_times(tptr)?;
    doutimensat(p, AT_FDCWD, path, &mut ts, 0)
}

/// `utimensat(fd, path, times, flag)`.
pub fn sys_utimensat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    times: UserPtr<[Timespec; 2]>,
    flag: i32,
) -> Result<(), Errno> {
    let mut ts = utimens_times(times)?;
    doutimensat(p, fd, path, &mut ts, flag)
}

/// `doutimensat(p, fd, path, ts, flag)`.
pub fn doutimensat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    ts: &mut [Timespec; 2],
    flag: i32,
) -> Result<(), Errno> {
    if flag & !AT_SYMLINK_NOFOLLOW != 0 {
        return Err(Errno::EINVAL);
    }

    let follow = if flag & AT_SYMLINK_NOFOLLOW != 0 {
        NOFOLLOW
    } else {
        FOLLOW
    };
    let mut nd = ndinitat(LOOKUP, follow, fd, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_FATTR | PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_WRITE;
    namei(&mut nd)?;
    dovutimens(p, ndvp(nd.ni_vp), ts)
}

/// `dovutimens(p, vp, ts)`: sets the times of a referenced, unlocked vnode, and releases it.
pub fn dovutimens<G: ProcGlue>(
    p: &Thread<G>,
    vp: &'static Vnode<G>,
    ts: &mut [Timespec; 2],
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    // KTRACE: not configured.

    vattr_null(&mut vattr);

    // make sure ctime is updated even if neither mtime nor atime is
    vattr.va_vaflags = VA_UTIMES_CHANGE;

    if ts[0].tv_nsec == UTIME_NOW || ts[1].tv_nsec == UTIME_NOW {
        if ts[0].tv_nsec == UTIME_NOW && ts[1].tv_nsec == UTIME_NOW {
            vattr.va_vaflags |= VA_UTIMES_NULL;
        }

        let now = crate::tc::getnanotime();
        if ts[0].tv_nsec == UTIME_NOW {
            ts[0] = now;
        }
        if ts[1].tv_nsec == UTIME_NOW {
            ts[1] = now;
        }
    }

    if ts[0].tv_nsec != UTIME_OMIT {
        vattr.va_atime = ts[0];
    }
    if ts[1].tv_nsec != UTIME_OMIT {
        vattr.va_mtime = ts[1];
    }

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = if vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0) {
        Err(Errno::EROFS)
    } else {
        vop_setattr(vp, &mut vattr, Cred::real(p.ucred()), p)
    };
    vput(vp);
    error
}

/// Set the access and modification times given a file descriptor.
pub fn sys_futimes<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    tptr: UserPtr<[Timeval; 2]>,
) -> Result<(), Errno> {
    let mut ts = utimes_times(tptr)?;
    dofutimens(p, fd, &mut ts)
}

/// `futimens(fd, times)`.
pub fn sys_futimens<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    times: UserPtr<[Timespec; 2]>,
) -> Result<(), Errno> {
    let mut ts = utimens_times(times)?;
    dofutimens(p, fd, &mut ts)
}

/// `dofutimens(p, fd, ts)`.
pub fn dofutimens<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    ts: &mut [Timespec; 2],
) -> Result<(), Errno> {
    let fp = getvnode(p, fd)?;
    let vp = file_vp(&fp);
    vref(vp);
    let _ = fp.frele(Some(p));

    dovutimens(p, vp, ts)
}

/// Truncate a file given a vnode (locked): `p` is pinned for the `RLIMIT_FSIZE` overrun's
/// `SIGXFSZ`.
pub fn dotruncate<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    vp: &'static Vnode<G>,
    len: Off,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    if len < 0 {
        return Err(Errno::EINVAL);
    }
    if vp.vtype() == VDIR {
        return Err(Errno::EISDIR);
    }
    vn_writechk(vp)?;
    if vp.vtype() == VREG && len as u64 > lim_cur_proc(&p, RLIMIT_FSIZE) {
        vop_getattr(vp, &mut vattr, Cred::real(p.ucred()), &p)?;
        if len as u64 > vattr.va_size {
            // if extending over the limit, send signal and fail
            psignal(p, SIGXFSZ);
            return Err(Errno::EFBIG);
        }
    }
    vattr_null(&mut vattr);
    vattr.va_size = len as u64;
    vop_setattr(vp, &mut vattr, Cred::real(p.ucred()), &p)
}

/// Truncate a file given its path name.
pub fn sys_truncate<G: ProcGlue>(
    p: Pin<&Thread<G>>,
    path: UserPtr<u8>,
    length: Off,
) -> Result<(), Errno> {
    let pr = p.get_ref();
    let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::User(path.addr()), pr);
    nd.ni_pledge = PLEDGE_FATTR | PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_WRITE;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error =
        vop_access(vp, VWRITE, Cred::real(pr.ucred()), pr).and_then(|()| dotruncate(p, vp, length));
    vput(vp);
    error
}

/// Truncate a file given a file descriptor.
pub fn sys_ftruncate<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: Pin<&Thread<P>>,
    fd: i32,
    length: Off,
) -> Result<(), Errno> {
    let pr = p.get_ref();
    let fp = getvnode(pr, fd)?;
    let error = if fp.flag() & FWRITE == 0 {
        Err(Errno::EINVAL)
    } else if pr.fd().ofileflags(fd) & UF_PLEDGEOPEN != 0 {
        Err(Errno::EPERM)
    } else {
        let vp = file_vp(&fp);
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
        let error = dotruncate(p, vp, length);
        let _ = vop_unlock(vp);
        error
    };
    let _ = fp.frele(Some(pr));
    error
}

/// Sync an open file.
pub fn sys_fsync<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
) -> Result<(), Errno> {
    let fp = getvnode(p, fd)?;
    let vp = file_vp(&fp);
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    let error = vop_fsync(vp, Cred::real(fp.cred()), MNT_WAIT, p);

    let _ = vop_unlock(vp);
    let _ = fp.frele(Some(p));
    error
}

/// Rename files. Source and destination must either both be directories, or both not be
/// directories. If target is a directory, it must be empty.
pub fn sys_rename<G: ProcGlue>(
    p: &Thread<G>,
    from: UserPtr<u8>,
    to: UserPtr<u8>,
) -> Result<(), Errno> {
    dorenameat(p, AT_FDCWD, from, AT_FDCWD, to)
}

/// `renameat(fromfd, from, tofd, to)`.
pub fn sys_renameat<G: ProcGlue>(
    p: &Thread<G>,
    fromfd: i32,
    from: UserPtr<u8>,
    tofd: i32,
    to: UserPtr<u8>,
) -> Result<(), Errno> {
    dorenameat(p, fromfd, from, tofd, to)
}

/// `dorenameat(p, fromfd, from, tofd, to)`.
pub fn dorenameat<G: ProcGlue>(
    p: &Thread<G>,
    fromfd: i32,
    from: UserPtr<u8>,
    tofd: i32,
    to: UserPtr<u8>,
) -> Result<(), Errno> {
    let mut fromnd = ndinitat(
        DELETE,
        WANTPARENT | SAVESTART,
        fromfd,
        NiDirp::User(from.addr()),
        p,
    );
    fromnd.ni_pledge = PLEDGE_RPATH | PLEDGE_CPATH;
    fromnd.ni_unveil = UNVEIL_READ | UNVEIL_CREATE;
    namei(&mut fromnd)?;
    let fvp = ndvp(fromnd.ni_vp);
    let fdvp = ndvp(fromnd.ni_dvp);

    let mut flags = LOCKPARENT | LOCKLEAF | NOCACHE | SAVESTART;
    // rename("foo/", "bar/");  is  OK
    if fvp.vtype() == VDIR {
        flags |= STRIPSLASHES;
    }

    let mut tond = ndinitat(RENAME, flags, tofd, NiDirp::User(to.addr()), p);
    tond.ni_pledge = PLEDGE_CPATH;
    tond.ni_unveil = UNVEIL_CREATE;
    let error = 'out1: {
        if let Err(error) = namei(&mut tond) {
            let _ = vop_abortop(fdvp, &mut fromnd.ni_cnd);
            vrele(fdvp);
            vrele(fvp);
            break 'out1 Err(error);
        }
        let tdvp = ndvp(tond.ni_dvp);
        let tvp = tond.ni_vp;
        // The C's `error = -1`: the source is the destination, nothing to do.
        let mut same = false;
        let mut error = 'out: {
            if let Some(tvp) = tvp {
                if fvp.vtype() == VDIR && tvp.vtype() != VDIR {
                    break 'out Err(Errno::ENOTDIR);
                } else if fvp.vtype() != VDIR && tvp.vtype() == VDIR {
                    break 'out Err(Errno::EISDIR);
                }
            }
            let mut error = Ok(());
            if ptr::eq(fvp, tdvp) {
                error = Err(Errno::EINVAL);
            }
            // If source is the same as the destination (that is the same inode number): the
            // C's `error = -1`, which replaces the EINVAL above and makes the call answer 0
            // (`rename(".", "..")` at a root, where `..` is the root itself).
            if tvp.is_some_and(|tvp| ptr::eq(fvp, tvp)) {
                same = true;
                error = Ok(());
            }
            error
        };
        if error.is_ok() && !same {
            if let Some(tvp) = tvp {
                let _ = uvm_vnp_uncache(tvp);
            }
            error = vop_rename(fdvp, fvp, &mut fromnd.ni_cnd, tdvp, tvp, &mut tond.ni_cnd);
        } else {
            let _ = vop_abortop(tdvp, &mut tond.ni_cnd);
            if tvp.is_some_and(|tvp| ptr::eq(tdvp, tvp)) {
                vrele(tdvp);
            } else {
                vput(tdvp);
            }
            if let Some(tvp) = tvp {
                vput(tvp);
            }
            let _ = vop_abortop(fdvp, &mut fromnd.ni_cnd);
            vrele(fdvp);
            vrele(fvp);
        }
        if let Some(sd) = tond.ni_startdir {
            vrele(sd);
        }
        drop(tond.ni_cnd.take_pnbuf());
        error
    };
    // out1:
    if let Some(sd) = fromnd.ni_startdir {
        vrele(sd);
    }
    drop(fromnd.ni_cnd.take_pnbuf());
    error
}

/// Make a directory file.
pub fn sys_mkdir<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>, mode: Mode) -> Result<(), Errno> {
    domkdirat(p, AT_FDCWD, path, mode)
}

/// `mkdirat(fd, path, mode)`.
pub fn sys_mkdirat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    mode: Mode,
) -> Result<(), Errno> {
    domkdirat(p, fd, path, mode)
}

/// `domkdirat(p, fd, path, mode)`.
pub fn domkdirat<G: ProcGlue>(
    p: &Thread<G>,
    fd: i32,
    path: UserPtr<u8>,
    mode: Mode,
) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    let mut nd = ndinitat(
        CREATE,
        LOCKPARENT | STRIPSLASHES,
        fd,
        NiDirp::User(path.addr()),
        p,
    );
    nd.ni_pledge = PLEDGE_CPATH;
    nd.ni_unveil = UNVEIL_CREATE;
    namei(&mut nd)?;
    let dvp = ndvp(nd.ni_dvp);
    if let Some(vp) = nd.ni_vp {
        let _ = vop_abortop(dvp, &mut nd.ni_cnd);
        if ptr::eq(dvp, vp) {
            vrele(dvp);
        } else {
            vput(dvp);
        }
        vrele(vp);
        return Err(Errno::EEXIST);
    }
    vattr_null(&mut vattr);
    vattr.va_type = VDIR;
    vattr.va_mode = (mode & ACCESSPERMS) & !p.fd().cmask();
    let error = vop_mkdir(dvp, &mut nd.ni_vp, &mut nd.ni_cnd, &mut vattr);
    if error.is_ok()
        && let Some(vp) = nd.ni_vp
    {
        vput(vp);
    }
    error
}

/// Remove a directory file.
pub fn sys_rmdir<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>) -> Result<(), Errno> {
    dounlinkat(p, AT_FDCWD, path, AT_REMOVEDIR)
}

/// Read a block of directory entries in a file system independent format: the bytes placed
/// in `buf`.
pub fn sys_getdents<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
    buf: usize,
    buflen: usize,
) -> Result<usize, Errno> {
    let mut eofflag = 0;

    if buflen > INT_MAX as usize {
        return Err(Errno::EINVAL);
    }
    let fp = getvnode(p, fd)?;
    let error = 'bad: {
        if fp.flag() & FREAD == 0 {
            break 'bad Err(Errno::EBADF);
        }
        let vp = file_vp(&fp);
        if vp.vtype() != VDIR {
            break 'bad Err(Errno::EINVAL);
        }

        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);

        if foffset(&fp) < 0 {
            let _ = vop_unlock(vp);
            break 'bad Err(Errno::EINVAL);
        }

        let mut aiov = [Iovec {
            iov_base: ptr::without_provenance_mut(buf),
            iov_len: buflen,
        }];
        let mut auio = user_read_uio(p, &mut aiov, foffset(&fp));
        let error = vop_readdir(vp, &mut auio, Cred::real(fp.cred()), &mut eofflag);
        fp.set_offset(auio.uio_offset);
        let _ = vop_unlock(vp);
        if let Err(e) = error {
            break 'bad Err(e);
        }
        Ok(buflen - auio.uio_resid)
    };
    let _ = fp.frele(Some(p));
    error
}

/// Set the mode mask for creation of filesystem nodes: the old mask.
pub fn sys_umask<G: ProcGlue>(p: &Thread<G>, newmask: Mode) -> Result<Mode, Errno> {
    let fdp = p.fd();

    let g = fdplock(fdp);
    let old = fdp.cmask();
    fdp.set_cmask(newmask & ACCESSPERMS);
    fdpunlock(g);
    Ok(old)
}

/// Void all references to file by ripping underlying filesystem away from vnode.
pub fn sys_revoke<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>) -> Result<(), Errno> {
    let mut vattr = Vattr::new();

    let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::User(path.addr()), p);
    nd.ni_pledge = PLEDGE_RPATH | PLEDGE_TTY;
    nd.ni_unveil = UNVEIL_READ;
    namei(&mut nd)?;
    let vp = ndvp(nd.ni_vp);
    let error = 'out: {
        if vp.vtype() != VCHR
            || major(vp.rdev()) >= devsw::<G>().nchrdev()
            || devsw::<G>().cdev_type(major(vp.rdev())) != D_TTY
        {
            break 'out Err(Errno::ENOTTY);
        }
        if let Err(e) = vop_getattr(vp, &mut vattr, Cred::real(p.ucred()), p) {
            break 'out Err(e);
        }
        if p.ucred().cr_uid.get() != vattr.va_uid
            && let Err(e) = suser(p)
        {
            break 'out Err(e);
        }
        if vp.usecount() > 1 || vp.flag() & VALIASED != 0 {
            let _ = vop_revoke(vp, REVOKEALL);
        }
        Ok(())
    };
    vrele(vp);
    error
}

/// Convert a user file descriptor to a kernel file entry. The file is returned with a
/// reference (`FREF`).
pub fn getvnode<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    p: &Thread<P>,
    fd: i32,
) -> Result<FileRef<G>, Errno> {
    let Some(fp) = fd_getfile(p.fd(), fd) else {
        return Err(Errno::EBADF);
    };

    if fp.f_type() != DTYPE_VNODE {
        let _ = fp.frele(Some(p));
        return Err(Errno::EINVAL);
    }

    if file_vp(&fp).vtype() == VBAD {
        let _ = fp.frele(Some(p));
        return Err(Errno::EBADF);
    }

    Ok(fp)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The system calls over the kernel's test file system (`kernel::fs::testfs`: `/a/b`,
    // `/a/c/d`, `/f`, `/s` a socket, `/dev/console`, `/dev/ttyp0` a tty, `/dev/sd0c` a disk,
    // `/a/abs -> /f`, `/lnk -> a/c`), a thread of a process of its own (root, its current
    // directory the root), each check named at its line of vfs_syscalls.c. The paths and the
    // buffers live in the user memory of the host's double at USER_BASE + 0x1200..0x2000,
    // which no other test uses (the testfs tests hold MEMORY: one at a time).
    use std::vec;
    use std::vec::Vec;

    use hal::sys::fcntl::{O_NONBLOCK, O_WRONLY};
    use hal::sys::mount::MNT_UNMOUNT;
    use hal::sys::signal::sigmask;
    use hal::sys::stat::{S_IFLNK, S_IFREG};
    use hal::sys::unistd::{_PC_LINK_MAX, _PC_PATH_MAX};

    use super::*;
    use crate::cred::Ucred;
    use crate::descrip::sys_close;
    use crate::file::FileOps;
    use crate::fs::testfs::{
        self, A, ABS, B, C, D, DEV, DISK, F, LNK, ROOT, S, TEST_DATASIZE, TTY, TestFiles, TestGlue,
        test_thread, tree_find, usecounts, vnode_in,
    };
    use crate::testutil::{self, TestFileData, USER_BASE};

    type Pg = TestGlue;

    /// The paths' slots: eight of 128 bytes.
    const PATHS: usize = USER_BASE + 0x1200;
    /// The buffers: to USER_BASE + 0x2000.
    const BUF: usize = USER_BASE + 0x1600;
    /// The buffers' length.
    const BUF_LEN: usize = 0x2000 - 0x1600;

    /// The NUL-terminated `s` in user memory, in slot `i`.
    fn up(i: usize, s: &[u8]) -> UserPtr<u8> {
        assert!(s.len() < 128 && i < 8);
        let at = PATHS + i * 128;
        let mut b = s.to_vec();
        b.push(0);
        UserPtr::<u8>::new(at).copyout_slice(&b).unwrap();
        UserPtr::new(at)
    }

    /// `s` as the first path argument.
    fn path(s: &[u8]) -> UserPtr<u8> {
        up(0, s)
    }

    /// `s` as the second path argument.
    fn path2(s: &[u8]) -> UserPtr<u8> {
        up(1, s)
    }

    /// The user buffer at `off`.
    fn ubuf(off: usize) -> UserPtr<u8> {
        assert!(off < BUF_LEN);
        UserPtr::new(BUF + off)
    }

    /// `n` bytes of the user buffer at `off`.
    fn user_bytes(off: usize, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        ubuf(off).copyin_slice(&mut v).unwrap();
        v
    }

    /// The user buffers zeroed.
    fn clear_buf() {
        ubuf(0).copyout_slice(&vec![0u8; BUF_LEN]).unwrap();
    }

    /// The thread's credentials: `uid` (and its group) as every id, no privilege but root's.
    fn as_user(p: &Thread<Pg>, uid: Uid) {
        let mut cr = Ucred::new();
        cr.cr_uid.set(uid);
        cr.cr_ruid.set(uid);
        cr.cr_svuid.set(uid);
        cr.cr_gid.set(uid);
        cr.cr_rgid.set(uid);
        cr.cr_svgid.set(uid);
        drop(p.set_cred(CredRef::new(cr)));
    }

    /// The thread's credentials back to root's.
    fn as_root(p: &Thread<Pg>) {
        as_user(p, 0);
    }

    /// The thread on its process's list of threads, the one thread of it (`single_thread_set`
    /// counts the others out; a process signal finds it there), signals ready: what `fork1`
    /// leaves.
    fn listed(p: &'static Thread<Pg>) -> Pin<&'static Thread<Pg>> {
        let pp = hal::pin::pin_static(p);
        crate::sig::test_sigacts(p.process());
        crate::sig::siginit(p.process());
        {
            let pr = p.process();
            let mut g = pr.ps_mtx.lock();
            pr.ps_threads.insert_tail(&mut g, pp);
        }
        p.process().ps_threadcnt.set(1);
        pp
    }

    /// The thread's process on the list of all processes (`checkdirs` walks it), as `fork1`
    /// leaves it.
    fn on_allprocess(p: &'static Thread<Pg>) {
        let tb = <Pg as ProcGlue>::tables();
        let mut w = klock::witness();
        tb.allprocess.insert_head(&mut w, p.process_ref().as_pin());
    }

    /// The root mount.
    fn rootmp() -> &'static Mount<Pg> {
        rootvnode::<Pg>().and_then(Vnode::mount).unwrap()
    }

    /// The vnode of node `idx` of the root mount, made if it was not (`VFS_VGET`, then the
    /// `vput`: its use count as it was).
    fn node(idx: usize) -> &'static Vnode<Pg> {
        let vp = testfs::vget_node(rootmp(), idx).unwrap();
        vput(vp);
        vp
    }

    /// `open(path, flags)`.
    fn open(p: &Thread<Pg>, s: &[u8], flags: i32) -> Result<i32, Errno> {
        sys_open(p, path(s), flags, 0)
    }

    /// `close(fd)`, which must succeed.
    fn close(p: &Thread<Pg>, fd: i32) {
        sys_close(p, fd).unwrap();
    }

    /// The vnode a descriptor's file is open on.
    fn vp_of(p: &Thread<Pg>, fd: i32) -> &'static Vnode<Pg> {
        let fp = fd_getfile(p.fd(), fd).unwrap();
        let vp = file_vp(&fp);
        let _ = fp.frele(Some(p));
        vp
    }

    /// The `f_flag` and `f_iflags` of a descriptor's file.
    fn flags_of(p: &Thread<Pg>, fd: i32) -> (i32, u32) {
        let fp = fd_getfile(p.fd(), fd).unwrap();
        let f = (fp.flag(), fp.f_iflags.load(Ordering::SeqCst));
        let _ = fp.frele(Some(p));
        f
    }

    /// A file of no vnode (a pipe's type, no seek), for the calls that want a vnode.
    struct OtherOps;

    impl FileOps<TestFiles> for OtherOps {
        fn read(&self, _: &File<TestFiles>, _: &mut Uio<'_, Pg>, _: i32) -> Result<(), Errno> {
            Ok(())
        }
        fn write(&self, _: &File<TestFiles>, _: &mut Uio<'_, Pg>, _: i32) -> Result<(), Errno> {
            Ok(())
        }
        fn ioctl(
            &self,
            _: &File<TestFiles>,
            _: u64,
            _: &mut [u8],
            _: &Thread<Pg>,
        ) -> Result<(), Errno> {
            Err(Errno::ENOTTY)
        }
        fn kqfilter(&self, _: &File<TestFiles>, _: &crate::event::Knote<Pg>) -> Result<(), Errno> {
            Err(Errno::EINVAL)
        }
        fn stat(&self, _: &File<TestFiles>, _: &mut Stat, _: &Thread<Pg>) -> Result<(), Errno> {
            Ok(())
        }
        fn close(&self, _: &File<TestFiles>, _: Option<&Thread<Pg>>) -> Result<(), Errno> {
            Ok(())
        }
    }

    static OTHER_OPS: OtherOps = OtherOps;

    /// A descriptor of a file of type `ty` installed by hand: a pipe-like file with no vnode,
    /// or (`vp`) a vnode file whose flags are `flag` (one `open(2)` cannot make: a directory
    /// open for writing only).
    fn open_by_hand(p: &Thread<Pg>, vp: Option<&'static Vnode<Pg>>, flag: i32) -> i32 {
        let mut g = fdplock(p.fd());
        let (fp, fd) = falloc(p, &mut g).unwrap();
        fp.f_flag.store(flag as u32, Ordering::SeqCst);
        match vp {
            Some(vp) => {
                vref(vp);
                if flag & FWRITE != 0 {
                    vp.set_writecount(vp.writecount() + 1);
                }
                let (ops, data) = <TestFiles as FileGlue>::vnode_file(vp);
                fp.install(DTYPE_VNODE, ops, data);
            }
            None => fp.install(crate::file::DTYPE_PIPE, &OTHER_OPS, TestFileData::default()),
        }
        fdinsert(&mut g, fd, 0, &fp);
        fdpunlock(g);
        let _ = fp.frele(Some(p));
        fd
    }

    /// The `struct stat` copied out at `off`: `st_mode`, `st_ino`, `st_gen`.
    fn stat_at(off: usize) -> (Mode, u64, u32) {
        let b = user_bytes(off, Stat::SIZE);
        let mode = u32::from_ne_bytes(b[0..4].try_into().unwrap());
        let ino = u64::from_ne_bytes(b[8..16].try_into().unwrap());
        let g = offset_of!(Stat, st_gen);
        let gen_ = u32::from_ne_bytes(b[g..g + 4].try_into().unwrap());
        (mode, ino, gen_)
    }

    /// The `struct statfs` copied out at `off`.
    fn statfs_at(off: usize) -> Statfs {
        hal::user::pod_from_bytes(&user_bytes(off, size_of::<Statfs>())).unwrap()
    }

    /// Mounts the test file system on `dir` as `sys_mount` does: the new mount.
    fn mount(p: &Thread<Pg>, dir: &[u8], flags: i32) -> &'static Mount<Pg> {
        let data = ubuf(0x900);
        data.copyout_slice(&[0u8; TEST_DATASIZE]).unwrap();
        sys_mount(p, up(2, b"testfs"), path(dir), flags, data.addr()).unwrap();
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(dir), p);
        namei(&mut nd).unwrap();
        let vp = ndvp(nd.ni_vp);
        let mp = vp.mount().unwrap();
        vrele(vp);
        mp
    }

    // ---- open, openat, __pledge_open (doopenat)

    /// `doopenat` (vfs_syscalls.c:1107-1240): the lowest free descriptor, a vnode file of the
    /// vnode opened (`f_flag` the open's FFLAGS & FMASK, :1181-1184), O_CLOEXEC and O_CLOFORK
    /// as UF_EXCLOSE and UF_FORKCLOSE (:1127-1128), the open's use count given back by the
    /// close; a failed lookup frees the descriptor (:1174).
    #[test]
    fn open_installs_a_vnode_file_at_the_lowest_descriptor() {
        let (_g, p) = test_thread();
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(fd, 0);
        let f = node(F);
        assert!(ptr::eq(vp_of(p, fd), f));
        assert_eq!(flags_of(p, fd).0, FREAD);
        assert_eq!(p.fd().ofileflags(fd), 0);
        let uses = f.usecount();
        close(p, fd);
        assert_eq!(f.usecount(), uses - 1);
        let fd = open(p, b"/f", O_RDWR | O_CLOEXEC | O_CLOFORK | O_NONBLOCK).unwrap();
        assert_eq!(p.fd().ofileflags(fd), UF_EXCLOSE | UF_FORKCLOSE);
        assert_eq!(flags_of(p, fd).0, FREAD | FWRITE | FNONBLOCK);
        close(p, fd);
        assert_eq!(open(p, b"/nope", O_RDONLY), Err(Errno::ENOENT));
        assert_eq!(open(p, b"/s", O_RDONLY), Err(Errno::EOPNOTSUPP));
        // openat relative to a directory's descriptor.
        let dfd = open(p, b"/a", O_RDONLY).unwrap();
        assert_eq!(dfd, 0);
        let fd = sys_openat(p, dfd, path(b"b"), O_RDONLY, 0).unwrap();
        assert_eq!(fd, 1);
        assert!(ptr::eq(vp_of(p, fd), node(B)));
        close(p, fd);
        close(p, dfd);
    }

    /// `doopenat`'s locks (vfs_syscalls.c:1121-1125, :1185-1206): pledge's flock check first,
    /// its refusal returned before a descriptor is taken; then a whole-file lock owned by the
    /// file (FIF_HASLOCK); a lock the open cannot have (O_NONBLOCK) closes the file and frees
    /// the descriptor; the lock goes with the close.
    #[test]
    fn open_locks_take_pledge_flock_first_and_a_failed_lock_closes_the_file() {
        let (_g, p) = test_thread();
        // Pledge refuses flock: only the opens that lock ask it.
        testutil::set_pledge_flock(Some(Errno::EPERM));
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        close(p, fd);
        assert_eq!(open(p, b"/f", O_RDONLY | O_EXLOCK), Err(Errno::EPERM));
        assert_eq!(open(p, b"/f", O_RDONLY | O_SHLOCK), Err(Errno::EPERM));
        testutil::set_pledge_flock(None);
        let fd = open(p, b"/f", O_RDWR | O_EXLOCK).unwrap();
        assert_eq!(fd, 0);
        assert_ne!(flags_of(p, fd).1 & FIF_HASLOCK, 0);
        let f = node(F);
        let uses = f.usecount();
        assert_eq!(
            open(p, b"/f", O_RDONLY | O_SHLOCK | O_NONBLOCK),
            Err(Errno::EAGAIN)
        );
        assert_eq!(f.usecount(), uses);
        let fd2 = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(fd2, 1);
        close(p, fd2);
        close(p, fd);
        let fd = open(p, b"/f", O_RDONLY | O_SHLOCK | O_NONBLOCK).unwrap();
        assert_ne!(flags_of(p, fd).1 & FIF_HASLOCK, 0);
        close(p, fd);
        assert!(!testfs::any_locked());
    }

    /// `__pledge_open` (vfs_syscalls.c:1074-1090): O_RDONLY or O_RDWR with O_CLOEXEC and
    /// O_CLOFORK only (EINVAL); its descriptor is UF_PLEDGEOPEN unless the file is a
    /// character device (:1179-1180).
    #[test]
    fn pledge_open_takes_two_modes_and_marks_all_but_a_character_device() {
        let (_g, p) = test_thread();
        for bad in [
            O_WRONLY,
            O_ACCMODE,
            O_RDONLY | O_CREAT,
            O_RDWR | O_TRUNC,
            O_RDONLY | O_EXLOCK,
            O_RDONLY | O_NONBLOCK,
        ] {
            assert_eq!(
                sys___pledge_open(p, path(b"/f"), bad, 0),
                Err(Errno::EINVAL),
                "{bad:#x}"
            );
        }
        let fd = sys___pledge_open(p, path(b"/f"), O_RDONLY | O_CLOEXEC, 0).unwrap();
        assert_eq!(p.fd().ofileflags(fd), UF_PLEDGEOPEN | UF_EXCLOSE);
        close(p, fd);
        let fd = sys___pledge_open(p, path(b"/dev/ttyp0"), O_RDWR | O_CLOFORK, 0).unwrap();
        assert_eq!(p.fd().ofileflags(fd), UF_FORKCLOSE);
        close(p, fd);
        // open(2) never marks.
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(p.fd().ofileflags(fd) & UF_PLEDGEOPEN, 0);
        close(p, fd);
    }

    /// `doopenat`'s creation mode (vfs_syscalls.c:1151-1153): less the process's cmask, in
    /// ALLPERMS without S_ISTXT, and in ACCESSPERMS when the process is pledged.
    #[test]
    fn open_creates_with_the_mode_less_cmask_and_s_istxt_and_under_pledge_the_set_ids() {
        let (_g, p) = test_thread();
        let _ = sys_umask(p, 0o022);
        let fd = sys_open(p, path(b"/new"), O_RDWR | O_CREAT, 0o7777).unwrap();
        let va = testfs::last_create().unwrap();
        assert_eq!((va.va_type, va.va_mode), (VREG, 0o6755));
        close(p, fd);
        assert!(tree_find(ROOT, b"new").is_some());
        p.process().ps_flags.fetch_or(PS_PLEDGE, Ordering::SeqCst);
        let fd = sys_open(p, path(b"/new2"), O_RDWR | O_CREAT, 0o7777).unwrap();
        assert_eq!(testfs::last_create().unwrap().va_mode, 0o755);
        close(p, fd);
        p.process().ps_flags.fetch_and(!PS_PLEDGE, Ordering::SeqCst);
    }

    /// `doopenat`'s ENODEV with `p_dupfd` set (vfs_syscalls.c:1163-1171, an open of
    /// /dev/fd/N): descriptor N is duplicated into the new one and the open's own file closed;
    /// a dup that fails frees the descriptor; ERESTART becomes EINTR (:1172-1173); ENODEV
    /// without `p_dupfd` stays ENODEV.
    #[test]
    fn open_of_a_dup_device_dups_and_erestart_is_eintr() {
        let (_g, p) = test_thread();
        let fd = open(p, b"/f", O_RDWR).unwrap();
        let tty_before = usecounts()[TTY];
        testfs::set_open_error(Errno::ENODEV, Some(fd));
        let nfd = open(p, b"/dev/ttyp0", O_RDONLY).unwrap();
        assert_eq!(nfd, 1);
        let (a, b) = (
            p.fd().ofile(fd as usize).unwrap(),
            p.fd().ofile(nfd as usize).unwrap(),
        );
        assert!(FileRef::ptr_eq(&a, &b));
        drop((a, b));
        assert_eq!(usecounts()[TTY], tty_before);
        testfs::set_open_error(Errno::ENODEV, Some(7));
        assert_eq!(open(p, b"/dev/ttyp0", O_RDONLY), Err(Errno::EBADF));
        testfs::set_open_error(Errno::ERESTART, None);
        assert_eq!(open(p, b"/f", O_RDONLY), Err(Errno::EINTR));
        testfs::set_open_error(Errno::ENODEV, None);
        assert_eq!(open(p, b"/f", O_RDONLY), Err(Errno::ENODEV));
        // Every failed open freed its descriptor: 2 is next.
        let fd3 = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(fd3, 2);
        for fd in [fd3, nfd, fd] {
            close(p, fd);
        }
    }

    /// `doopenat`'s own truncation under a lock (vfs_syscalls.c:1158-1161, :1207-1226): done
    /// after the lock (`va_size` 0); EACCES without FWRITE; EROFS on a read-only mount (a
    /// device, which vn_open lets through); a failure closes the file and drops its lock.
    #[test]
    fn open_truncates_itself_under_a_lock() {
        let (_g, p) = test_thread();
        let fd = open(p, b"/f", O_RDWR | O_TRUNC | O_EXLOCK).unwrap();
        assert_eq!(testfs::last_setattr().unwrap().va_size, 0);
        close(p, fd);
        let uses = node(F).usecount();
        assert_eq!(
            open(p, b"/f", O_RDONLY | O_TRUNC | O_EXLOCK),
            Err(Errno::EACCES)
        );
        assert!(testfs::last_setattr().is_none());
        assert_eq!(node(F).usecount(), uses);
        rootmp().flag_set(MNT_RDONLY);
        assert_eq!(
            open(p, b"/dev/ttyp0", O_RDWR | O_TRUNC | O_EXLOCK),
            Err(Errno::EROFS)
        );
        rootmp().flag_clr(MNT_RDONLY);
        assert!(!testfs::any_locked());
        // The locks went with the files: an exclusive one is granted.
        let fd = open(p, b"/dev/ttyp0", O_RDWR | O_EXLOCK | O_NONBLOCK).unwrap();
        close(p, fd);
    }

    // ---- chdir, fchdir, chroot

    /// `chroot` (vfs_syscalls.c:797-828): suser first (:807); ENOTDIR and VEXEC through
    /// `change_dir` (:834-851); the root directory set and PS_CHROOT (:825-826); a chroot inside a
    /// chroot changes the current directory too (:813-823), each old one released.
    #[test]
    fn chroot_needs_root_and_chdirs_inside_a_chroot() {
        let (_g, p) = test_thread();
        as_user(p, 1000);
        assert_eq!(sys_chroot(p, path(b"/a")), Err(Errno::EPERM));
        assert!(p.fd().rdir().is_none());
        as_root(p);
        assert_eq!(sys_chroot(p, path(b"/f")), Err(Errno::ENOTDIR));
        testfs::set_deny_access(true);
        assert_eq!(sys_chroot(p, path(b"/a")), Err(Errno::EACCES));
        testfs::set_deny_access(false);
        assert!(p.fd().rdir().is_none());
        assert_eq!(p.process().ps_flags.load(Ordering::SeqCst) & PS_CHROOT, 0);
        let root = rootvnode::<Pg>().unwrap();
        assert_eq!(sys_chroot(p, path(b"/a")), Ok(()));
        let a = node(A);
        assert!(ptr::eq(p.fd().rdir().unwrap(), a));
        assert!(ptr::eq(p.fd().cdir().unwrap(), root));
        assert_ne!(p.process().ps_flags.load(Ordering::SeqCst) & PS_CHROOT, 0);
        let (a_uses, root_uses) = (a.usecount(), root.usecount());
        // Inside the chroot "/c" is /a/c; the current directory follows it.
        assert_eq!(sys_chroot(p, path(b"/c")), Ok(()));
        let c = node(C);
        assert!(ptr::eq(p.fd().rdir().unwrap(), c));
        assert!(ptr::eq(p.fd().cdir().unwrap(), c));
        assert_eq!(a.usecount(), a_uses - 1);
        assert_eq!(root.usecount(), root_uses - 1);
        assert_eq!(c.usecount(), 2);
    }

    /// `chdir` and `fchdir` (vfs_syscalls.c:723-791): ENOTDIR for a file or a descriptor
    /// that is no vnode (:737-740), EBADF for none; `fchdir` crosses to the root of what is
    /// mounted on the directory (:746-755); the old current directory released.
    #[test]
    fn chdir_and_fchdir_set_the_current_directory() {
        let (_g, p) = test_thread();
        let root = rootvnode::<Pg>().unwrap();
        let root_uses = root.usecount();
        assert_eq!(sys_chdir(p, path(b"/a/c")), Ok(()));
        assert!(ptr::eq(p.fd().cdir().unwrap(), node(C)));
        assert_eq!(root.usecount(), root_uses - 1);
        assert_eq!(sys_chdir(p, path(b"/f")), Err(Errno::ENOTDIR));
        testfs::set_deny_access(true);
        assert_eq!(sys_chdir(p, path(b"/a")), Err(Errno::EACCES));
        testfs::set_deny_access(false);
        assert!(ptr::eq(p.fd().cdir().unwrap(), node(C)));
        let dfd = open(p, b"/a", O_RDONLY).unwrap();
        assert_eq!(sys_fchdir(p, dfd), Ok(()));
        assert!(ptr::eq(p.fd().cdir().unwrap(), node(A)));
        let ffd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(sys_fchdir(p, ffd), Err(Errno::ENOTDIR));
        let other = open_by_hand(p, None, FREAD);
        assert_eq!(sys_fchdir(p, other), Err(Errno::ENOTDIR));
        assert_eq!(sys_fchdir(p, 99), Err(Errno::EBADF));
        // A descriptor open on a directory something was mounted on since.
        let cfd = open(p, b"/a/c", O_RDONLY).unwrap();
        let mp2 = testfs::mount_on(node(C));
        assert_eq!(sys_fchdir(p, cfd), Ok(()));
        let cwd = p.fd().cdir().unwrap();
        assert_ne!(cwd.flag() & VROOT, 0);
        assert!(cwd.mount().is_some_and(|m| ptr::eq(m, mp2)));
        for fd in [cfd, other, ffd, dfd] {
            close(p, fd);
        }
    }

    // ---- mount, unmount, sync, quotactl, statfs

    /// `mount` (vfs_syscalls.c:99-304): suser first (:117); MNT_NOPERM without MNT_NODEV and
    /// MNT_NOEXEC is EPERM (:173-178); ENOTDIR (:183-187); an unknown type EOPNOTSUPP
    /// (:193-198); something mounted here EBUSY (:207-211); the arguments copied in
    /// (`vfc_datasize` bytes, :200-205); the mount on the list, the covered directory's
    /// `v_mountedhere`, `f_owner`, the syncer of a read-write mount (:217, :275-299).
    #[test]
    fn mount_checks_the_caller_the_flags_and_the_directory() {
        let (_g, p) = test_thread();
        let data = ubuf(0x900);
        data.copyout_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
            .unwrap();
        let ty = up(2, b"testfs");
        as_user(p, 1000);
        assert_eq!(
            sys_mount(p, ty, path(b"/a"), 0, data.addr()),
            Err(Errno::EPERM)
        );
        as_root(p);
        assert_eq!(
            sys_mount(p, ty, path(b"/a"), MNT_NOPERM | MNT_NODEV, data.addr()),
            Err(Errno::EPERM)
        );
        assert_eq!(
            sys_mount(p, ty, path(b"/f"), 0, data.addr()),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            sys_mount(p, up(3, b"nofs"), path(b"/a"), 0, data.addr()),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sys_mount(p, ty, path(b"/a"), 0, USER_BASE - 0x1000),
            Err(Errno::EFAULT)
        );
        assert!(node(A).mountedhere().is_none());
        assert_eq!(
            sys_mount(
                p,
                ty,
                path(b"/a"),
                MNT_NOPERM | MNT_NODEV | MNT_NOEXEC,
                data.addr()
            ),
            Ok(())
        );
        assert_eq!(testfs::last_mount(), Some((1..=16).collect::<Vec<u8>>()));
        let mp = node(A).mountedhere().unwrap();
        assert!(mp.is_listed());
        assert!(mp.vnodecovered().is_some_and(|v| ptr::eq(v, node(A))));
        assert_eq!(
            mp.flag() & (MNT_NOPERM | MNT_NODEV | MNT_NOEXEC),
            MNT_NOPERM | MNT_NODEV | MNT_NOEXEC
        );
        assert_eq!(mp.flag() & MNT_OP_FLAGS, 0);
        assert!(mp.syncer().is_some());
        assert_eq!(mp.owner(), 0);
        // A second mount on /a stacks on the first one's root (namei crosses to it), whose
        // `v_mountedhere` is empty: EBUSY (:207-211) is the race of two mounts on one
        // directory, which one thread cannot make.
        assert_eq!(sys_mount(p, ty, path(b"/a"), 0, data.addr()), Ok(()));
        let top = vnode_in(mp, ROOT).unwrap().mountedhere().unwrap();
        assert!(
            top.vnodecovered()
                .is_some_and(|v| v.mount().is_some_and(|m| ptr::eq(m, mp)))
        );
    }

    /// `mount`'s updates (vfs_syscalls.c:134-168, :237-245, :253-273): EINVAL on a vnode that is not a
    /// mount's root; MNT_RELOAD of a read-write mount EOPNOTSUPP; the settable flags replaced,
    /// MNT_RDONLY's WANTRDWR, MNT_OP_FLAGS cleared, a failed update restoring the flags;
    /// the syncer gone with read-only and back with read-write.
    #[test]
    fn mount_updates_replace_the_settable_flags_and_a_failure_restores_them() {
        let (_g, p) = test_thread();
        let mp = mount(p, b"/a", MNT_NOSUID);
        let ty = up(2, b"testfs");
        let data = ubuf(0x900).addr();
        // /a/a: the tree's /a in the mount on /a, not its root.
        assert_eq!(
            sys_mount(p, ty, path(b"/a/a"), MNT_UPDATE, data),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_mount(p, ty, path(b"/a"), MNT_UPDATE | MNT_RELOAD, data),
            Err(Errno::EOPNOTSUPP)
        );
        // Read-only: the syncer goes; NOSUID replaced by NOEXEC.
        assert_eq!(
            sys_mount(
                p,
                ty,
                path(b"/a"),
                MNT_UPDATE | MNT_RDONLY | MNT_NOEXEC,
                data
            ),
            Ok(())
        );
        assert_eq!(
            mp.flag() & (MNT_RDONLY | MNT_NOEXEC | MNT_NOSUID),
            MNT_RDONLY | MNT_NOEXEC
        );
        assert_eq!(mp.flag() & MNT_OP_FLAGS, 0);
        assert!(mp.syncer().is_none());
        // A failed update: the flags as they were.
        let before = mp.flag();
        testfs::set_mount_error(Some(Errno::EIO));
        assert_eq!(
            sys_mount(p, ty, path(b"/a"), MNT_UPDATE | MNT_NOATIME, data),
            Err(Errno::EIO)
        );
        assert_eq!(mp.flag(), before);
        testfs::set_mount_error(None);
        // Read-write again (WANTRDWR): RDONLY cleared, the syncer back.
        assert_eq!(sys_mount(p, ty, path(b"/a"), MNT_UPDATE, data), Ok(()));
        assert_eq!(mp.flag() & (MNT_RDONLY | MNT_WANTRDWR | MNT_NOEXEC), 0);
        assert!(mp.syncer().is_some());
        // MNT_RELOAD of a read-only mount is allowed.
        assert_eq!(
            sys_mount(p, ty, path(b"/a"), MNT_UPDATE | MNT_RDONLY, data),
            Ok(())
        );
        assert_eq!(
            sys_mount(
                p,
                ty,
                path(b"/a"),
                MNT_UPDATE | MNT_RELOAD | MNT_RDONLY,
                data
            ),
            Ok(())
        );
    }

    /// A mount that fails (vfs_syscalls.c:293-299): the covered directory's `v_mountedhere`
    /// cleared, the mount freed off the list, the directory released; one that succeeds
    /// moves the current and root directories and the root vnode that were the covered
    /// directory to the new root (`checkdirs`, :316-348), releasing as many references.
    #[test]
    fn a_failed_mount_is_undone_and_a_mount_moves_the_directories_on_it() {
        let (_g, p) = test_thread();
        let a = node(A);
        let a_uses = a.usecount();
        testfs::set_mount_error(Some(Errno::EIO));
        let data = ubuf(0x900).addr();
        assert_eq!(
            sys_mount(p, up(2, b"testfs"), path(b"/a"), 0, data),
            Err(Errno::EIO)
        );
        testfs::set_mount_error(None);
        assert!(a.mountedhere().is_none());
        assert_eq!(a.usecount(), a_uses);
        assert_eq!(crate::fs::mount::mountlist::<Pg>(false).count(), 1);
        // The current directory on /a: it moves to the new mount's root.
        on_allprocess(p);
        assert_eq!(sys_chdir(p, path(b"/a")), Ok(()));
        let a_uses = a.usecount();
        let mp = mount(p, b"/a", 0);
        let newroot = vnode_in(mp, ROOT).unwrap();
        assert!(ptr::eq(p.fd().cdir().unwrap(), newroot));
        // The mount's covered reference stays; the current directory's moved.
        assert_eq!(a.usecount(), a_uses);
        assert_eq!(crate::fs::mount::mountlist::<Pg>(false).count(), 2);
    }

    /// `unmount` (vfs_syscalls.c:357-399): suser first (:368); EINVAL for the root file
    /// system (:381-384) and for a vnode that is not a mount's root (:389-392) (EBUSY when the
    /// mount cannot be busied, :395-396, is another unmount's race: namei waits for it to end
    /// before this call can name the mount); the unmount drops every unveil of its vnodes
    /// (:488-490), syncs a read-write mount (:492-495), unlinks it and releases the covered
    /// directory (:504-508); a mount in use stays (EBUSY, the syncer made again, :497-502).
    #[test]
    fn unmount_checks_the_caller_and_the_mount_and_undoes_the_mount() {
        let (_g, p) = test_thread();
        rootmp().flag_set(MNT_ROOTFS);
        let a = node(A);
        let a_uses = a.usecount();
        let mp = mount(p, b"/a", 0);
        as_user(p, 1000);
        assert_eq!(sys_unmount(p, path(b"/a"), 0), Err(Errno::EPERM));
        as_root(p);
        assert_eq!(sys_unmount(p, path(b"/"), 0), Err(Errno::EINVAL));
        assert_eq!(sys_unmount(p, path(b"/a/a"), 0), Err(Errno::EINVAL));
        // In use: EBUSY, the syncer back.
        let fd = open(p, b"/a/f", O_RDONLY).unwrap();
        assert_eq!(sys_unmount(p, path(b"/a"), 0), Err(Errno::EBUSY));
        assert!(mp.is_listed());
        assert!(mp.syncer().is_some());
        close(p, fd);
        let _ = testutil::file_log();
        let syncs = testfs::SYNCS.load(Ordering::SeqCst);
        assert_eq!(sys_unmount(p, path(b"/a"), 0), Ok(()));
        assert!(
            testutil::file_log()
                .iter()
                .any(|l| l == "unveil_removevnode")
        );
        assert!(testfs::SYNCS.load(Ordering::SeqCst) > syncs);
        assert!(!mp.is_listed());
        assert!(a.mountedhere().is_none());
        assert_eq!(a.usecount(), a_uses);
    }

    /// `dounmount`'s nested mounts (vfs_syscalls.c:405-466): a mount below the one asked for
    /// makes the unmount EBUSY without MNT_FORCE (:424-427) and is unmounted first with it.
    #[test]
    fn unmount_refuses_a_nested_mount_unless_forced() {
        let (_g, p) = test_thread();
        let outer = mount(p, b"/a", 0);
        // The tree's /a inside the outer mount.
        let inner = mount(p, b"/a/a", 0);
        assert!(
            inner
                .vnodecovered()
                .and_then(Vnode::mount)
                .is_some_and(|m| ptr::eq(m, outer))
        );
        assert_eq!(sys_unmount(p, path(b"/a"), 0), Err(Errno::EBUSY));
        assert!(outer.is_listed() && inner.is_listed());
        // Neither stays busy.
        assert_eq!(vfs_busy(outer, VB_WRITE | VB_NOWAIT), Ok(()));
        vfs_unbusy(outer);
        assert_eq!(vfs_busy(inner, VB_WRITE | VB_NOWAIT), Ok(()));
        vfs_unbusy(inner);
        assert_eq!(sys_unmount(p, path(b"/a"), MNT_FORCE), Ok(()));
        assert!(!outer.is_listed() && !inner.is_listed());
        assert!(node(A).mountedhere().is_none());
    }

    /// `dounmount_leaf` of a read-only mount does not sync it (vfs_syscalls.c:492-495);
    /// `sync` (:523-543) syncs each read-write mount with MNT_ASYNC cleared meanwhile and put
    /// back, skips a read-only one and one that cannot be busied.
    #[test]
    fn sync_clears_mnt_async_meanwhile_and_skips_read_only_and_busy_mounts() {
        let (_g, p) = test_thread();
        let mp = rootmp();
        mp.flag_set(MNT_ASYNC);
        let _ = testfs::last_sync_flags();
        assert_eq!(sys_sync(p), Ok(()));
        assert_eq!(testfs::last_sync_flags().unwrap() & MNT_ASYNC, 0);
        assert_ne!(mp.flag() & MNT_ASYNC, 0);
        mp.flag_clr(MNT_ASYNC);
        mp.flag_set(MNT_RDONLY);
        assert_eq!(sys_sync(p), Ok(()));
        assert!(testfs::last_sync_flags().is_none());
        mp.flag_clr(MNT_RDONLY);
        mp.flag_set(MNT_UNMOUNT);
        assert_eq!(sys_sync(p), Ok(()));
        assert!(testfs::last_sync_flags().is_none());
        mp.flag_clr(MNT_UNMOUNT);
        // A read-only mount's unmount does not sync it.
        let inner = mount(p, b"/a", MNT_RDONLY);
        let _ = testfs::last_sync_flags();
        assert_eq!(sys_unmount(p, path(b"/a"), 0), Ok(()));
        assert!(testfs::last_sync_flags().is_none());
        assert!(!inner.is_listed());
    }

    /// `quotactl` (vfs_syscalls.c:549-569): the lookup's error, else the file system's answer.
    #[test]
    fn quotactl_asks_the_file_system_of_the_path() {
        let (_g, p) = test_thread();
        assert_eq!(sys_quotactl(p, path(b"/nope"), 0, 0, 0), Err(Errno::ENOENT));
        assert_eq!(
            sys_quotactl(p, path(b"/f"), 0, 0, 0),
            Err(Errno::EOPNOTSUPP)
        );
    }

    /// `statfs`, `fstatfs` and `copyout_statfs` (vfs_syscalls.c:572-660): the statistics
    /// refreshed, `f_flags` the mount's visible flags, the file system id hidden from
    /// non-root (zeroed in the copy, the rest whole) and given to root.
    #[test]
    fn statfs_refreshes_masks_the_flags_and_hides_the_fsid_from_non_root() {
        let (_g, p) = test_thread();
        let mp = rootmp();
        mp.with_stat(|sp| sp.f_fsid = Fsid { val: [5, 6] });
        mp.flag_set(MNT_NOSUID | MNT_DOOMED);
        let n = testfs::STATFSES.load(Ordering::SeqCst);
        clear_buf();
        assert_eq!(sys_statfs(p, path(b"/a"), ubuf(0)), Ok(()));
        assert!(testfs::STATFSES.load(Ordering::SeqCst) > n);
        let st = statfs_at(0);
        assert_eq!(st.f_blocks, 100);
        assert_eq!(st.f_bavail, 50);
        assert_eq!(st.f_flags as i32, mp.flag() & MNT_VISFLAGMASK);
        assert_eq!(st.f_flags as i32 & MNT_DOOMED, 0);
        assert_ne!(st.f_flags as i32 & MNT_NOSUID, 0);
        assert_eq!(st.f_fsid.val, [5, 6]);
        as_user(p, 1000);
        clear_buf();
        assert_eq!(sys_statfs(p, path(b"/a"), ubuf(0)), Ok(()));
        let st = statfs_at(0);
        assert_eq!(st.f_fsid.val, [0, 0]);
        assert_eq!(st.f_blocks, 100);
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        clear_buf();
        assert_eq!(sys_fstatfs(p, fd, ubuf(0)), Ok(()));
        let st = statfs_at(0);
        assert_eq!((st.f_fsid.val, st.f_bavail), ([0, 0], 50));
        let other = open_by_hand(p, None, FREAD);
        assert_eq!(sys_fstatfs(p, other, ubuf(0)), Err(Errno::EINVAL));
        assert_eq!(sys_fstatfs(p, 99, ubuf(0)), Err(Errno::EBADF));
        as_root(p);
        assert_eq!(sys_fstatfs(p, fd, ubuf(0)), Ok(()));
        assert_eq!(statfs_at(0).f_fsid.val, [5, 6]);
        assert_eq!(sys_statfs(p, path(b"/nope"), ubuf(0)), Err(Errno::ENOENT));
        close(p, other);
        close(p, fd);
        mp.flag_clr(MNT_NOSUID | MNT_DOOMED);
    }

    /// `getfsstat` (vfs_syscalls.c:666-717): without a buffer, the number of mounts; with
    /// one, as many as fit (the count returned at most that); MNT_NOWAIT and MNT_LAZY do not
    /// refresh; the file system ids are root's (`copyout_statfs`, :580-593); a mount that
    /// cannot be busied is neither copied nor counted.
    #[test]
    fn getfsstat_counts_copies_what_fits_and_refreshes_unless_told_not_to() {
        let (_g, p) = test_thread();
        let _mp2 = mount(p, b"/a", 0);
        let sz = size_of::<Statfs>();
        assert_eq!(sys_getfsstat(p, UserPtr::new(0), 0, 0), Ok(2));
        clear_buf();
        assert_eq!(sys_getfsstat(p, ubuf(0), sz, MNT_WAIT), Ok(1));
        assert_eq!(sys_getfsstat(p, ubuf(0), 2 * sz, MNT_WAIT), Ok(2));
        let n = testfs::STATFSES.load(Ordering::SeqCst);
        assert_eq!(sys_getfsstat(p, ubuf(0), 2 * sz, MNT_NOWAIT), Ok(2));
        assert_eq!(sys_getfsstat(p, ubuf(0), 2 * sz, MNT_LAZY), Ok(2));
        assert_eq!(testfs::STATFSES.load(Ordering::SeqCst), n);
        rootmp().with_stat(|sp| sp.f_fsid = Fsid { val: [5, 6] });
        clear_buf();
        assert_eq!(sys_getfsstat(p, ubuf(0), sz, MNT_NOWAIT), Ok(1));
        assert_eq!(statfs_at(0).f_fsid.val, [5, 6]);
        as_user(p, 1000);
        assert_eq!(sys_getfsstat(p, ubuf(0), sz, MNT_NOWAIT), Ok(1));
        assert_eq!(statfs_at(0).f_fsid.val, [0, 0]);
        as_root(p);
        rootmp().flag_set(MNT_UNMOUNT);
        assert_eq!(sys_getfsstat(p, UserPtr::new(0), 0, 0), Ok(1));
        rootmp().flag_clr(MNT_UNMOUNT);
        assert_eq!(
            sys_getfsstat(p, UserPtr::new(USER_BASE - 0x1000), 2 * sz, MNT_NOWAIT),
            Err(Errno::EFAULT)
        );
    }

    // ---- file handles

    /// `getfh`, `fhopen`, `fhstat` and `fhstatfs` (vfs_syscalls.c:1246-1486): suser first
    /// (:1260, :1306, :1435, :1469); the handle names the mount's id and the file; ESTALE for
    /// an unknown file system; `fhopen` refuses O_CREAT and an open for neither reading nor
    /// writing (EINVAL, :1310-1313), a socket (EOPNOTSUPP), a file under O_DIRECTORY (ENOTDIR),
    /// a directory for writing (EISDIR); counts the writer; a failure closes the file and
    /// puts the vnode back.
    #[test]
    fn file_handles_need_root_and_open_what_they_name() {
        let (_g, p) = test_thread();
        rootmp().with_stat(|sp| sp.f_fsid = Fsid { val: [9, 9] });
        let fhp = UserPtr::<Fhandle>::new(BUF);
        as_user(p, 1000);
        assert_eq!(sys_getfh(p, path(b"/f"), fhp), Err(Errno::EPERM));
        assert_eq!(sys_fhopen(p, fhp, O_RDONLY), Err(Errno::EPERM));
        assert_eq!(sys_fhstat(p, fhp, ubuf(0x200)), Err(Errno::EPERM));
        assert_eq!(sys_fhstatfs(p, fhp, ubuf(0x200)), Err(Errno::EPERM));
        as_root(p);
        assert_eq!(sys_getfh(p, path(b"/f"), fhp), Ok(()));
        let fh = fhp.copyin().unwrap();
        assert_eq!(fh.fh_fsid.val, [9, 9]);
        assert_eq!(fh.fh_fid.fid_len, 4);
        clear_buf();
        fhp.copyout(&fh).unwrap();
        assert_eq!(sys_fhstat(p, fhp, ubuf(0x200)), Ok(()));
        assert_eq!(stat_at(0x200).1, 100 + F as u64);
        assert_eq!(sys_fhstatfs(p, fhp, ubuf(0x200)), Ok(()));
        assert_eq!(statfs_at(0x200).f_fsid.val, [9, 9]);
        assert_eq!(sys_fhopen(p, fhp, O_RDONLY | O_CREAT), Err(Errno::EINVAL));
        assert_eq!(sys_fhopen(p, fhp, O_ACCMODE), Err(Errno::EINVAL));
        let f = node(F);
        let (uses, writers) = (f.usecount(), f.writecount());
        let fd = sys_fhopen(p, fhp, O_RDWR | O_CLOEXEC).unwrap();
        assert!(ptr::eq(vp_of(p, fd), f));
        assert_eq!(f.writecount(), writers + 1);
        assert_eq!(p.fd().ofileflags(fd), UF_EXCLOSE);
        close(p, fd);
        assert_eq!((f.usecount(), f.writecount()), (uses, writers));
        assert_eq!(
            sys_fhopen(p, fhp, O_RDONLY | O_DIRECTORY),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(f.usecount(), uses);
        // A directory for writing, a socket.
        assert_eq!(sys_getfh(p, path(b"/a"), fhp), Ok(()));
        assert_eq!(sys_fhopen(p, fhp, O_RDWR), Err(Errno::EISDIR));
        assert_eq!(sys_getfh(p, path(b"/s"), fhp), Ok(()));
        assert_eq!(sys_fhopen(p, fhp, O_RDONLY), Err(Errno::EOPNOTSUPP));
        // An unknown file system.
        let mut stale = fh;
        stale.fh_fsid = Fsid { val: [1, 2] };
        fhp.copyout(&stale).unwrap();
        assert_eq!(sys_fhopen(p, fhp, O_RDONLY), Err(Errno::ESTALE));
        assert_eq!(sys_fhstat(p, fhp, ubuf(0x200)), Err(Errno::ESTALE));
        assert_eq!(sys_fhstatfs(p, fhp, ubuf(0x200)), Err(Errno::ESTALE));
        // Every failure freed its descriptor.
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(fd, 0);
        close(p, fd);
    }

    // ---- the name space

    /// `mknod` (vfs_syscalls.c:1492-1591): EINVAL for a VNOVAL device; suser unless the
    /// file system does not check permissions, and EINVAL under a changed root, both but for
    /// a FIFO of device 0 (:1534-1541); EEXIST; the type from S_IFMT (:1551-1575: S_IFMT is
    /// VBAD, S_IFIFO EOPNOTSUPP without option FIFO, any other EINVAL); the mode in ALLPERMS
    /// less cmask, ACCESSPERMS under pledge (:1546-1548), the device number given.
    #[test]
    fn mknod_checks_privilege_and_type_and_masks_the_mode() {
        let (_g, p) = test_thread();
        let _ = sys_umask(p, 0o022);
        assert_eq!(
            sys_mknod(p, path(b"/n"), S_IFCHR | 0o666, VNOVAL),
            Err(Errno::EINVAL)
        );
        as_user(p, 1000);
        assert_eq!(
            sys_mknod(p, path(b"/n"), S_IFCHR | 0o666, 0x0102),
            Err(Errno::EPERM)
        );
        // A FIFO needs no privilege: refused for want of option FIFO.
        assert_eq!(sys_mkfifo(p, path(b"/n"), 0o666), Err(Errno::EOPNOTSUPP));
        // ... unless the file system does not check permissions (`vnoperm`: never in a
        // mount's root directory).
        rootmp().flag_set(MNT_NOPERM);
        assert_eq!(
            sys_mknod(p, path(b"/n"), S_IFCHR | 0o666, 0x0102),
            Err(Errno::EPERM)
        );
        assert_eq!(sys_mknod(p, path(b"/a/n"), S_IFCHR | 0o666, 0x0102), Ok(()));
        rootmp().flag_clr(MNT_NOPERM);
        as_root(p);
        let va = testfs::last_mknod().unwrap();
        assert_eq!((va.va_type, va.va_mode, va.va_rdev), (VCHR, 0o644, 0x0102));
        assert!(tree_find(A, b"n").is_some());
        assert_eq!(
            sys_mknod(p, path(b"/a/n"), S_IFCHR | 0o666, 0x0102),
            Err(Errno::EEXIST)
        );
        assert_eq!(
            sys_mknodat(p, AT_FDCWD, path(b"/bad"), S_IFMT | 0o7777, 3),
            Ok(())
        );
        let va = testfs::last_mknod().unwrap();
        assert_eq!((va.va_type, va.va_mode), (VBAD, 0o7755));
        assert_eq!(sys_mknod(p, path(b"/blk"), S_IFBLK | 0o600, 4), Ok(()));
        assert_eq!(testfs::last_mknod().unwrap().va_type, VBLK);
        assert_eq!(
            sys_mknod(p, path(b"/reg"), S_IFREG | 0o600, 0),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_mknod(p, path(b"/fifo"), S_IFIFO | 0o600, 1),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            sys_mkfifoat(p, AT_FDCWD, path(b"/fifo"), 0o600),
            Err(Errno::EOPNOTSUPP)
        );
        p.process().ps_flags.fetch_or(PS_PLEDGE, Ordering::SeqCst);
        assert_eq!(sys_mknod(p, path(b"/pl"), S_IFCHR | 0o4777, 5), Ok(()));
        assert_eq!(testfs::last_mknod().unwrap().va_mode, 0o755);
        p.process().ps_flags.fetch_and(!PS_PLEDGE, Ordering::SeqCst);
        // Under a changed root: EINVAL.
        assert_eq!(sys_chroot(p, path(b"/a")), Ok(()));
        assert_eq!(
            sys_mknod(p, path(b"/m"), S_IFCHR | 0o600, 6),
            Err(Errno::EINVAL)
        );
        assert!(!testfs::any_locked());
    }

    /// `link` and `linkat` (vfs_syscalls.c:1625-1703): EINVAL for a flag but
    /// AT_SYMLINK_FOLLOW; EPERM for a directory; EEXIST; EXDEV across mounts; the new name.
    #[test]
    fn link_refuses_directories_existing_names_and_other_mounts() {
        let (_g, p) = test_thread();
        assert_eq!(
            sys_linkat(p, AT_FDCWD, path(b"/f"), AT_FDCWD, path2(b"/g"), 0x8000),
            Err(Errno::EINVAL)
        );
        assert_eq!(sys_link(p, path(b"/a"), path2(b"/g")), Err(Errno::EPERM));
        assert_eq!(sys_link(p, path(b"/f"), path2(b"/a/b")), Err(Errno::EEXIST));
        let _mp = mount(p, b"/a/c", 0);
        assert_eq!(
            sys_link(p, path(b"/f"), path2(b"/a/c/g")),
            Err(Errno::EXDEV)
        );
        let uses = usecounts();
        assert_eq!(sys_link(p, path(b"/f"), path2(b"/g")), Ok(()));
        assert!(tree_find(ROOT, b"g").is_some());
        // linkat without AT_SYMLINK_FOLLOW links the symbolic link itself.
        assert_eq!(
            sys_linkat(p, AT_FDCWD, path(b"/lnk"), AT_FDCWD, path2(b"/l2"), 0),
            Ok(())
        );
        let l2 = tree_find(ROOT, b"l2").unwrap();
        assert!(matches!(
            testfs::tree_node(l2).unwrap().kind,
            testfs::Kind::Lnk(_)
        ));
        assert_eq!(usecounts()[F], uses[F]);
        assert!(!testfs::any_locked());
    }

    /// `symlink` and `symlinkat` (vfs_syscalls.c:1709-1765): EEXIST; the link made with the
    /// target and the mode ACCESSPERMS less cmask; `readlink` reads it back.
    #[test]
    fn symlink_makes_the_link_and_readlink_reads_it() {
        let (_g, p) = test_thread();
        let _ = sys_umask(p, 0o027);
        assert_eq!(sys_symlink(p, path(b"x"), path2(b"/f")), Err(Errno::EEXIST));
        assert_eq!(sys_symlink(p, path(b"a/b"), path2(b"/sl")), Ok(()));
        assert_eq!(testfs::last_create().unwrap().va_mode, 0o750);
        let dfd = open(p, b"/a", O_RDONLY).unwrap();
        assert_eq!(sys_symlinkat(p, path(b"../f"), dfd, path2(b"sl2")), Ok(()));
        assert!(tree_find(A, b"sl2").is_some());
        close(p, dfd);
        clear_buf();
        assert_eq!(sys_readlink(p, path(b"/sl"), BUF, 100), Ok(3));
        assert_eq!(user_bytes(0, 4), b"a/b\0");
        assert!(!testfs::any_locked());
    }

    /// `readlink` and `readlinkat` (vfs_syscalls.c:2114-2173): EINVAL for a file that is not a
    /// link; the bytes moved and their count, at most `count`.
    #[test]
    fn readlink_reads_at_most_count_bytes_of_a_link() {
        let (_g, p) = test_thread();
        assert_eq!(sys_readlink(p, path(b"/f"), BUF, 100), Err(Errno::EINVAL));
        clear_buf();
        assert_eq!(sys_readlink(p, path(b"/lnk"), BUF, 100), Ok(3));
        assert_eq!(user_bytes(0, 3), b"a/c");
        clear_buf();
        assert_eq!(sys_readlink(p, path(b"/a/abs"), BUF, 1), Ok(1));
        assert_eq!(user_bytes(0, 2), b"/\0");
        let dfd = open(p, b"/a", O_RDONLY).unwrap();
        assert_eq!(sys_readlinkat(p, dfd, path(b"abs"), BUF, 100), Ok(2));
        close(p, dfd);
        assert_eq!(
            sys_readlink(p, path(b"/lnk"), USER_BASE - 0x1000, 100),
            Err(Errno::EFAULT)
        );
        assert!(!testfs::any_locked());
    }

    /// `unlink`, `unlinkat` and `rmdir` (vfs_syscalls.c:1771-1854, :3073-3080): EINVAL for a flag but
    /// AT_REMOVEDIR; for rmdir ENOTDIR, EINVAL for ".", EBUSY for a directory mounted on;
    /// EBUSY for a mount's root; the name removed (a directory only empty: ENOTEMPTY from the
    /// file system).
    #[test]
    fn unlink_and_rmdir_refuse_what_the_c_refuses_and_remove_the_name() {
        let (_g, p) = test_thread();
        assert_eq!(
            sys_unlinkat(p, AT_FDCWD, path(b"/f"), 0x1),
            Err(Errno::EINVAL)
        );
        assert_eq!(sys_rmdir(p, path(b"/f")), Err(Errno::ENOTDIR));
        assert_eq!(sys_rmdir(p, path(b"/a/.")), Err(Errno::EINVAL));
        assert_eq!(sys_rmdir(p, path(b"/a")), Err(Errno::ENOTEMPTY));
        let _mp = mount(p, b"/a/c", 0);
        // /a/c names the mount's root: EBUSY for it as the root of a file system.
        assert_eq!(sys_rmdir(p, path(b"/a/c")), Err(Errno::EBUSY));
        assert_eq!(sys_unlink(p, path(b"/a/c")), Err(Errno::EBUSY));
        let uses = usecounts();
        assert_eq!(sys_unlink(p, path(b"/f")), Ok(()));
        assert!(tree_find(ROOT, b"f").is_none());
        assert_eq!(sys_unlink(p, path(b"/f")), Err(Errno::ENOENT));
        assert_eq!(sys_mkdir(p, path(b"/e"), 0o755), Ok(()));
        assert_eq!(sys_unlinkat(p, AT_FDCWD, path(b"/e"), AT_REMOVEDIR), Ok(()));
        assert!(tree_find(ROOT, b"e").is_none());
        assert_eq!(usecounts()[ROOT], uses[ROOT]);
        assert!(!testfs::any_locked());
    }

    /// `rename` and `renameat` (vfs_syscalls.c:2902-3007): ENOTDIR and EISDIR when one is a
    /// directory and the other not; EINVAL when the source is the target's directory; the same
    /// file renamed onto itself does nothing; the name moved (a target replaced); every
    /// reference released whatever the answer.
    #[test]
    fn rename_refuses_mismatched_types_and_moves_the_name() {
        let (_g, p) = test_thread();
        let before = usecounts();
        assert_eq!(
            sys_rename(p, path(b"/a/c"), path2(b"/f")),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            sys_rename(p, path(b"/f"), path2(b"/a/c")),
            Err(Errno::EISDIR)
        );
        assert_eq!(
            sys_rename(p, path(b"/a"), path2(b"/a/x")),
            Err(Errno::EINVAL)
        );
        assert_eq!(sys_rename(p, path(b"/f"), path2(b"/f")), Ok(()));
        assert!(tree_find(ROOT, b"f").is_some());
        assert_eq!(
            sys_rename(p, path(b"/nope"), path2(b"/x")),
            Err(Errno::ENOENT)
        );
        assert_eq!(
            sys_rename(p, path(b"/f"), path2(b"/a/c/d/x")),
            Err(Errno::EACCES)
        );
        assert_eq!(usecounts(), before);
        assert_eq!(sys_rename(p, path(b"/f"), path2(b"/a/g")), Ok(()));
        assert!(tree_find(ROOT, b"f").is_none());
        assert!(tree_find(A, b"g").is_some());
        let dfd = open(p, b"/a", O_RDONLY).unwrap();
        assert_eq!(sys_renameat(p, dfd, path(b"g"), dfd, path2(b"b")), Ok(()));
        assert_eq!(
            tree_find(A, b"b").map(|i| testfs::tree_node(i).unwrap().name),
            Some(b"b".to_vec())
        );
        assert!(tree_find(A, b"g").is_none());
        close(p, dfd);
        // rename("c/", "e/") of a directory strips the slashes.
        assert_eq!(sys_rename(p, path(b"/a/c/"), path2(b"/e/")), Ok(()));
        assert!(tree_find(ROOT, b"e").is_some());
        assert!(!testfs::any_locked());
    }

    /// The C's `error = -1` wins over its EINVAL (vfs_syscalls.c:2971-2978, :3004-3005). At a
    /// root, `..` looked up for the rename's target is the root itself as both the directory
    /// and the file (vfs_lookup.c:555-562), and `.` looked up for DELETE is the directory
    /// (ufs_lookup's): `rename(".", "..")` has fvp == tdvp == tvp and answers 0, renaming
    /// nothing; at the absolute root and at a changed one.
    #[test]
    fn rename_of_dot_onto_dotdot_at_a_root_answers_0() {
        let (_g, p) = test_thread();
        let root = rootvnode::<Pg>().unwrap();
        // namei's half first: `..` at the root, as dorenameat's target looks it up.
        let mut nd = ndinit(
            RENAME,
            LOCKPARENT | LOCKLEAF | NOCACHE | SAVESTART,
            NiDirp::Sys(b".."),
            p,
        );
        namei(&mut nd).unwrap();
        let (dvp, vp) = (ndvp(nd.ni_dvp), ndvp(nd.ni_vp));
        assert!(
            ptr::eq(dvp, root) && ptr::eq(vp, root),
            "`..` at the root is the root"
        );
        let _ = vop_abortop(dvp, &mut nd.ni_cnd);
        vrele(dvp);
        vput(vp);
        if let Some(sd) = nd.ni_startdir {
            vrele(sd);
        }
        drop(nd.ni_cnd.take_pnbuf());

        // The names the tree has, which a rename would move.
        let names = || {
            (
                tree_find(ROOT, b"a"),
                tree_find(ROOT, b"f"),
                tree_find(A, b"c"),
                tree_find(ROOT, b".."),
                tree_find(A, b".."),
            )
        };
        let before = (usecounts(), names());
        assert_eq!(sys_rename(p, path(b"."), path2(b"..")), Ok(()));
        assert_eq!((usecounts(), names()), before);
        assert!(!testfs::any_locked());

        // In a changed root (/a), its `..` is /a itself: the same answer.
        assert_eq!(sys_chroot(p, path(b"/a")), Ok(()));
        assert_eq!(sys_chdir(p, path(b"/")), Ok(()));
        assert!(p.fd().cdir().is_some_and(|c| ptr::eq(c, node(A))));
        assert_eq!(sys_rename(p, path(b"."), path2(b"..")), Ok(()));
        assert!(!testfs::any_locked());
        assert_eq!(names(), before.1);
        // A name and its own parent still differ: EINVAL as before.
        assert_eq!(
            sys_rename(p, path(b"/c"), path2(b"/c/x")),
            Err(Errno::EINVAL)
        );
    }

    /// `mkdir` and `mkdirat` (vfs_syscalls.c:3013-3067): EEXIST; the directory made with the
    /// mode in ACCESSPERMS less cmask, its vnode put back.
    #[test]
    fn mkdir_makes_the_directory_with_the_masked_mode() {
        let (_g, p) = test_thread();
        let _ = sys_umask(p, 0o022);
        assert_eq!(sys_mkdir(p, path(b"/a"), 0o777), Err(Errno::EEXIST));
        assert_eq!(sys_mkdir(p, path(b"/e"), 0o7777), Ok(()));
        let va = testfs::last_create().unwrap();
        assert_eq!((va.va_type, va.va_mode), (VDIR, 0o755));
        let e = tree_find(ROOT, b"e").unwrap();
        assert_eq!(vnode_in(rootmp(), e).unwrap().usecount(), 0);
        let dfd = open(p, b"/e", O_RDONLY).unwrap();
        assert_eq!(sys_mkdirat(p, dfd, path(b"sub"), 0o700), Ok(()));
        assert!(tree_find(e, b"sub").is_some());
        close(p, dfd);
        assert!(!testfs::any_locked());
    }

    // ---- attributes

    /// `access` and `faccessat` (vfs_syscalls.c:1898-1979): EINVAL for a mode or a flag
    /// outside the defined; the real ids used unless AT_EACCESS, the thread's own credentials
    /// back after on every path; amode 0 only checks the name exists; W_OK checks the file
    /// system can be written (`vn_writechk`, EROFS).
    #[test]
    fn access_checks_with_the_real_ids_and_puts_the_credentials_back() {
        let (_g, p) = test_thread();
        assert_eq!(sys_access(p, path(b"/f"), 0x10), Err(Errno::EINVAL));
        assert_eq!(
            sys_faccessat(p, AT_FDCWD, path(b"/f"), R_OK, 0x2),
            Err(Errno::EINVAL)
        );
        // Effective 1000, real 2000.
        let mut cr = Ucred::new();
        cr.cr_uid.set(1000);
        cr.cr_ruid.set(2000);
        cr.cr_gid.set(1000);
        cr.cr_rgid.set(2000);
        drop(p.set_cred(CredRef::new(cr)));
        assert_eq!(sys_access(p, path(b"/f"), R_OK | X_OK), Ok(()));
        assert_eq!(testfs::last_access_uid(), Some(2000));
        assert_eq!(p.ucred().cr_uid.get(), 1000);
        assert_eq!(
            sys_faccessat(p, AT_FDCWD, path(b"/f"), R_OK, AT_EACCESS),
            Ok(())
        );
        assert_eq!(testfs::last_access_uid(), Some(1000));
        // The failures put the credentials back too.
        assert_eq!(sys_access(p, path(b"/nope"), R_OK), Err(Errno::ENOENT));
        assert_eq!(p.ucred().cr_uid.get(), 1000);
        testfs::set_deny_access(true);
        assert_eq!(sys_access(p, path(b"/f"), R_OK), Err(Errno::EACCES));
        assert_eq!(p.ucred().cr_uid.get(), 1000);
        // F_OK: the name exists, no access asked.
        assert_eq!(sys_access(p, path(b"/f"), 0), Ok(()));
        testfs::set_deny_access(false);
        rootmp().flag_set(MNT_RDONLY);
        assert_eq!(sys_access(p, path(b"/f"), W_OK), Err(Errno::EROFS));
        assert_eq!(sys_access(p, path(b"/f"), R_OK), Ok(()));
        rootmp().flag_clr(MNT_RDONLY);
        assert!(!testfs::any_locked());
    }

    /// `stat`, `lstat` and `fstatat` (vfs_syscalls.c:1985-2058): EINVAL for a flag but
    /// AT_SYMLINK_NOFOLLOW; the attributes of the file (a link's own with NOFOLLOW); the
    /// generation number hidden from non-root (:2034-2036).
    #[test]
    fn stat_hides_the_generation_from_non_root_and_lstat_does_not_follow() {
        let (_g, p) = test_thread();
        assert_eq!(
            sys_fstatat(p, AT_FDCWD, path(b"/f"), ubuf(0), 0x1),
            Err(Errno::EINVAL)
        );
        clear_buf();
        assert_eq!(sys_stat(p, path(b"/f"), ubuf(0)), Ok(()));
        let (mode, ino, gen_) = stat_at(0);
        assert_eq!((mode & S_IFMT, ino, gen_), (S_IFREG, 100 + F as u64, 7));
        as_user(p, 1000);
        clear_buf();
        assert_eq!(sys_stat(p, path(b"/f"), ubuf(0)), Ok(()));
        assert_eq!(stat_at(0).2, 0);
        as_root(p);
        assert_eq!(sys_lstat(p, path(b"/lnk"), ubuf(0)), Ok(()));
        assert_eq!(stat_at(0).0 & S_IFMT, S_IFLNK);
        let dfd = open(p, b"/a", O_RDONLY).unwrap();
        assert_eq!(
            sys_fstatat(p, dfd, path(b"abs"), ubuf(0), AT_SYMLINK_NOFOLLOW),
            Ok(())
        );
        assert_eq!(stat_at(0).0 & S_IFMT, S_IFLNK);
        assert_eq!(sys_fstatat(p, dfd, path(b"b"), ubuf(0), 0), Ok(()));
        assert_eq!(stat_at(0).1, 100 + B as u64);
        close(p, dfd);
        assert_eq!(
            sys_stat(p, path(b"/f"), UserPtr::new(USER_BASE - 0x1000)),
            Err(Errno::EFAULT)
        );
        assert!(!testfs::any_locked());
    }

    /// `pathconf` and `pathconfat` (vfs_syscalls.c:2064-2108): EINVAL for a flag but
    /// AT_SYMLINK_NOFOLLOW; the value of the name (the file system's, or the constant ones'
    /// of `VOP_PATHCONF`); EINVAL for a name the file system does not know.
    #[test]
    fn pathconf_answers_the_file_systems_value() {
        let (_g, p) = test_thread();
        assert_eq!(sys_pathconf(p, path(b"/f"), _PC_LINK_MAX), Ok(32767));
        assert_eq!(
            sys_pathconf(p, path(b"/f"), _PC_PATH_MAX),
            Ok(MAXPATHLEN as Register)
        );
        assert_eq!(sys_pathconf(p, path(b"/f"), 9999), Err(Errno::EINVAL));
        assert_eq!(
            sys_pathconfat(p, AT_FDCWD, path(b"/f"), _PC_LINK_MAX, 0x4),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_pathconfat(
                p,
                AT_FDCWD,
                path(b"/lnk"),
                _PC_LINK_MAX,
                AT_SYMLINK_NOFOLLOW
            ),
            Ok(32767)
        );
        assert!(!testfs::any_locked());
    }

    /// `chflags`, `chflagsat`, `fchflags` and `dovchflags` (vfs_syscalls.c:2179-2276): EROFS;
    /// VNOVAL flags EINVAL; non-root may not change a device's flags (EINVAL, :2260-2267);
    /// `fchflags` of an UF_PLEDGEOPEN descriptor EPERM (:2238); the flags given to the file
    /// system; the vnode released on every path.
    #[test]
    fn chflags_refuses_read_only_vnoval_devices_for_users_and_pledged_descriptors() {
        let (_g, p) = test_thread();
        let before = usecounts();
        assert_eq!(
            sys_chflags(p, path(b"/f"), VNOVAL as u32),
            Err(Errno::EINVAL)
        );
        rootmp().flag_set(MNT_RDONLY);
        assert_eq!(sys_chflags(p, path(b"/f"), 1), Err(Errno::EROFS));
        rootmp().flag_clr(MNT_RDONLY);
        as_user(p, 1000);
        assert_eq!(sys_chflags(p, path(b"/dev/ttyp0"), 1), Err(Errno::EINVAL));
        assert_eq!(sys_chflags(p, path(b"/f"), 4), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_flags, 4);
        as_root(p);
        assert_eq!(sys_chflags(p, path(b"/dev/ttyp0"), 2), Ok(()));
        assert_eq!(
            sys_chflagsat(p, AT_FDCWD, path(b"/lnk"), 8, AT_SYMLINK_NOFOLLOW),
            Ok(())
        );
        assert_eq!(
            sys_chflagsat(p, AT_FDCWD, path(b"/f"), 8, 0x8),
            Err(Errno::EINVAL)
        );
        assert_eq!(usecounts()[F], before[F]);
        let fd = sys___pledge_open(p, path(b"/f"), O_RDONLY, 0).unwrap();
        assert_eq!(sys_fchflags(p, fd, 1), Err(Errno::EPERM));
        close(p, fd);
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(sys_fchflags(p, fd, 16), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_flags, 16);
        close(p, fd);
        assert!(!testfs::any_locked());
    }

    /// `chmod`, `fchmodat` and `fchmod` (vfs_syscalls.c:2282-2377): EINVAL for bits outside
    /// S_IFMT|ALLPERMS and for a flag; ACCESSPERMS under pledge (:2316, :2358); EROFS;
    /// `fchmod` of an UF_PLEDGEOPEN descriptor EPERM (:2367); the mode in ALLPERMS given.
    #[test]
    fn chmod_masks_under_pledge_and_refuses_read_only_and_pledged_descriptors() {
        let (_g, p) = test_thread();
        assert_eq!(sys_chmod(p, path(b"/f"), 0o1000000), Err(Errno::EINVAL));
        assert_eq!(
            sys_fchmodat(p, AT_FDCWD, path(b"/f"), 0o644, 0x8),
            Err(Errno::EINVAL)
        );
        assert_eq!(sys_chmod(p, path(b"/f"), S_IFREG | 0o4755), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_mode, 0o4755);
        p.process().ps_flags.fetch_or(PS_PLEDGE, Ordering::SeqCst);
        assert_eq!(sys_chmod(p, path(b"/f"), 0o4755), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_mode, 0o755);
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(sys_fchmod(p, fd, 0o2711), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_mode, 0o711);
        p.process().ps_flags.fetch_and(!PS_PLEDGE, Ordering::SeqCst);
        assert_eq!(sys_fchmod(p, fd, 0o1000000), Err(Errno::EINVAL));
        rootmp().flag_set(MNT_RDONLY);
        assert_eq!(sys_chmod(p, path(b"/f"), 0o644), Err(Errno::EROFS));
        assert_eq!(sys_fchmod(p, fd, 0o644), Err(Errno::EROFS));
        rootmp().flag_clr(MNT_RDONLY);
        close(p, fd);
        let fd = sys___pledge_open(p, path(b"/f"), O_RDONLY, 0).unwrap();
        assert_eq!(sys_fchmod(p, fd, 0o644), Err(Errno::EPERM));
        close(p, fd);
        assert!(!testfs::any_locked());
    }

    /// The `chown` family (vfs_syscalls.c:2383-2556): EINVAL for a flag; EROFS before
    /// anything; `fchown` of an UF_PLEDGEOPEN descriptor EPERM (:2532); pledge's chown check
    /// with the ids asked (:2434, :2485, :2535), its refusal returned; the set-id bits cleared
    /// when an id changes on a file system that checks permissions (:2436-2444), the mode
    /// left otherwise (VNOVAL).
    #[test]
    fn chown_asks_pledge_and_clears_the_set_ids_only_when_an_id_changes() {
        let (_g, p) = test_thread();
        let novl = VNOVAL as Uid;
        assert_eq!(
            sys_fchownat(p, AT_FDCWD, path(b"/f"), 1, 1, 0x8),
            Err(Errno::EINVAL)
        );
        rootmp().flag_set(MNT_RDONLY);
        let _ = testutil::file_log();
        assert_eq!(sys_chown(p, path(b"/f"), 5, 6), Err(Errno::EROFS));
        assert!(
            !testutil::file_log()
                .iter()
                .any(|l| l.starts_with("pledge_chown"))
        );
        rootmp().flag_clr(MNT_RDONLY);
        testfs::set_attr(0o4755, 0);
        assert_eq!(sys_chown(p, path(b"/f"), 5, 6), Ok(()));
        assert!(testutil::file_log().iter().any(|l| l == "pledge_chown 5 6"));
        let va = testfs::last_setattr().unwrap();
        assert_eq!((va.va_uid, va.va_gid, va.va_mode), (5, 6, 0o755));
        testfs::set_attr(0o755, 0);
        assert_eq!(sys_lchown(p, path(b"/lnk"), 5, novl), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_mode, VNOVAL as Mode);
        testfs::set_attr(0o6755, 0);
        assert_eq!(sys_chown(p, path(b"/f"), novl, novl), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_mode, VNOVAL as Mode);
        rootmp().flag_set(MNT_NOPERM);
        assert_eq!(sys_chown(p, path(b"/f"), 7, novl), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_mode, VNOVAL as Mode);
        rootmp().flag_clr(MNT_NOPERM);
        testutil::set_pledge_chown(Some(Errno::EPERM));
        assert_eq!(sys_chown(p, path(b"/f"), 8, 8), Err(Errno::EPERM));
        assert!(testfs::last_setattr().is_none());
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(sys_fchown(p, fd, 8, 8), Err(Errno::EPERM));
        testutil::set_pledge_chown(None);
        assert_eq!(sys_fchown(p, fd, 8, 9), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_gid, 9);
        close(p, fd);
        let fd = sys___pledge_open(p, path(b"/f"), O_RDONLY, 0).unwrap();
        let _ = testutil::file_log();
        assert_eq!(sys_fchown(p, fd, 8, 8), Err(Errno::EPERM));
        assert!(
            !testutil::file_log()
                .iter()
                .any(|l| l.starts_with("pledge_chown"))
        );
        close(p, fd);
        assert!(!testfs::any_locked());
    }

    /// The times (vfs_syscalls.c:2562-2783): a NULL array is "now" for both, with
    /// VA_UTIMES_NULL; an invalid timeval or timespec EINVAL; UTIME_NOW sets one to now
    /// without VA_UTIMES_NULL, UTIME_OMIT leaves one (VNOVAL); VA_UTIMES_CHANGE always; EROFS;
    /// a flag but AT_SYMLINK_NOFOLLOW EINVAL.
    #[test]
    fn utimes_family_now_omit_and_the_flags() {
        let (_g, p) = test_thread();
        let tv = UserPtr::<[Timeval; 2]>::new(BUF);
        let ts = UserPtr::<[Timespec; 2]>::new(BUF);
        assert_eq!(sys_utimes(p, path(b"/f"), UserPtr::new(0)), Ok(()));
        let va = testfs::last_setattr().unwrap();
        assert_eq!(va.va_vaflags, VA_UTIMES_CHANGE | VA_UTIMES_NULL);
        assert_eq!(va.va_atime, va.va_mtime);
        tv.copyout(&[Timeval::new(10, 5), Timeval::new(20, 6)])
            .unwrap();
        assert_eq!(sys_utimes(p, path(b"/f"), tv), Ok(()));
        let va = testfs::last_setattr().unwrap();
        assert_eq!(va.va_vaflags, VA_UTIMES_CHANGE);
        assert_eq!(
            (va.va_atime, va.va_mtime),
            (Timespec::new(10, 5000), Timespec::new(20, 6000))
        );
        tv.copyout(&[Timeval::new(10, 1_000_000), Timeval::new(20, 6)])
            .unwrap();
        assert_eq!(sys_utimes(p, path(b"/f"), tv), Err(Errno::EINVAL));
        ts.copyout(&[Timespec::new(1, UTIME_OMIT), Timespec::new(2, UTIME_NOW)])
            .unwrap();
        assert_eq!(sys_utimensat(p, AT_FDCWD, path(b"/f"), ts, 0), Ok(()));
        let va = testfs::last_setattr().unwrap();
        assert_eq!(va.va_vaflags, VA_UTIMES_CHANGE);
        assert_eq!(va.va_atime.tv_nsec, i64::from(VNOVAL));
        assert_ne!(va.va_mtime.tv_nsec, UTIME_NOW);
        ts.copyout(&[Timespec::new(1, -5), Timespec::new(2, 0)])
            .unwrap();
        assert_eq!(
            sys_utimensat(p, AT_FDCWD, path(b"/f"), ts, 0),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            sys_utimensat(p, AT_FDCWD, path(b"/f"), UserPtr::new(0), 0x1),
            Err(Errno::EINVAL)
        );
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        ts.copyout(&[Timespec::new(3, 4), Timespec::new(5, UTIME_OMIT)])
            .unwrap();
        assert_eq!(sys_futimens(p, fd, ts), Ok(()));
        let va = testfs::last_setattr().unwrap();
        assert_eq!(va.va_atime, Timespec::new(3, 4));
        assert_eq!(va.va_mtime.tv_nsec, i64::from(VNOVAL));
        assert_eq!(sys_futimes(p, fd, UserPtr::new(0)), Ok(()));
        rootmp().flag_set(MNT_RDONLY);
        assert_eq!(sys_futimes(p, fd, UserPtr::new(0)), Err(Errno::EROFS));
        assert_eq!(
            sys_utimes(p, path(b"/f"), UserPtr::new(0)),
            Err(Errno::EROFS)
        );
        rootmp().flag_clr(MNT_RDONLY);
        close(p, fd);
        assert!(!testfs::any_locked());
    }

    /// `truncate`, `ftruncate` and `dotruncate` (vfs_syscalls.c:2789-2871): EINVAL for a
    /// negative length; EISDIR; an extension past RLIMIT_FSIZE is SIGXFSZ and EFBIG, a
    /// shrink past it is not; `ftruncate` of a descriptor not open for writing EINVAL and of
    /// an UF_PLEDGEOPEN one EPERM (:2860).
    #[test]
    fn truncate_checks_the_length_the_limit_and_the_descriptor() {
        let (_g, p) = test_thread();
        let pp = listed(p);
        assert_eq!(sys_truncate(pp, path(b"/f"), -1), Err(Errno::EINVAL));
        assert_eq!(sys_truncate(pp, path(b"/a"), 0), Err(Errno::EISDIR));
        assert_eq!(sys_truncate(pp, path(b"/f"), 100), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_size, 100);
        // A limit of 200: 300 shrinks the 512-byte file and is allowed; 1000 extends it.
        let mut limits = crate::resource::Plimit::new();
        for l in &mut limits.pl_rlimit {
            l.rlim_cur = crate::resource::RLIM_INFINITY;
            l.rlim_max = crate::resource::RLIM_INFINITY;
        }
        limits.pl_rlimit[crate::resource::RLIMIT_NOFILE].rlim_cur = 512;
        limits.pl_rlimit[RLIMIT_FSIZE].rlim_cur = 200;
        drop(
            p.process()
                .set_limit(crate::resource::PlimitRef::new(limits)),
        );
        assert_eq!(sys_truncate(pp, path(b"/f"), 300), Ok(()));
        let pending = |p: &Thread<Pg>| {
            p.process().ps_siglist.load(Ordering::Relaxed) | p.p_siglist.load(Ordering::Relaxed)
        };
        assert_eq!(pending(p) & sigmask(SIGXFSZ), 0);
        assert_eq!(sys_truncate(pp, path(b"/f"), 1000), Err(Errno::EFBIG));
        assert_ne!(pending(p) & sigmask(SIGXFSZ), 0);
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(sys_ftruncate(pp, fd, 0), Err(Errno::EINVAL));
        close(p, fd);
        let fd = open(p, b"/f", O_RDWR).unwrap();
        assert_eq!(sys_ftruncate(pp, fd, 10), Ok(()));
        assert_eq!(testfs::last_setattr().unwrap().va_size, 10);
        assert_eq!(sys_ftruncate(pp, fd, -1), Err(Errno::EINVAL));
        close(p, fd);
        let fd = sys___pledge_open(p, path(b"/f"), O_RDWR, 0).unwrap();
        assert_eq!(sys_ftruncate(pp, fd, 0), Err(Errno::EPERM));
        close(p, fd);
        let other = open_by_hand(p, None, FREAD | FWRITE);
        assert_eq!(sys_ftruncate(pp, other, 0), Err(Errno::EINVAL));
        close(p, other);
        assert!(!testfs::any_locked());
    }

    /// `fsync` (vfs_syscalls.c:2877-2895) and `lseek` (:1860-1892): EINVAL for a descriptor
    /// that is no vnode (`getvnode`), EBADF for none; `lseek` ESPIPE for a file with no seek,
    /// the new offset back and the file's seeks counted.
    #[test]
    fn fsync_and_lseek_of_vnode_and_other_descriptors() {
        let (_g, p) = test_thread();
        let fd = open(p, b"/f", O_RDWR).unwrap();
        assert_eq!(sys_fsync(p, fd), Ok(()));
        let other = open_by_hand(p, None, FREAD);
        assert_eq!(sys_fsync(p, other), Err(Errno::EINVAL));
        assert_eq!(sys_fsync(p, 99), Err(Errno::EBADF));
        assert_eq!(sys_lseek(p, other, 0, SEEK_SET), Err(Errno::ESPIPE));
        assert_eq!(sys_lseek(p, 99, 0, SEEK_SET), Err(Errno::EBADF));
        let seeks = |p: &Thread<Pg>, fd: i32| {
            let fp = fd_getfile(p.fd(), fd).unwrap();
            let n = fp.stats().f_seek;
            let _ = fp.frele(Some(p));
            n
        };
        assert_eq!(sys_lseek(p, fd, 40, SEEK_SET), Ok(40));
        assert_eq!(sys_lseek(p, fd, 2, hal::sys::unistd::SEEK_CUR), Ok(42));
        assert_eq!(seeks(p, fd), 2);
        assert_eq!(sys_lseek(p, fd, -100, SEEK_SET), Err(Errno::EINVAL));
        assert_eq!(seeks(p, fd), 2);
        close(p, other);
        close(p, fd);
        assert!(!testfs::any_locked());
    }

    /// `getdents` (vfs_syscalls.c:3086-3144): EINVAL for a length past INT_MAX, EBADF for a
    /// descriptor not open for reading, EINVAL for a file that is not a directory and for a
    /// negative offset; the file system's records copied out, their bytes counted, the
    /// offset advanced; 0 at the end.
    #[test]
    fn getdents_copies_the_records_and_advances_the_offset() {
        let (_g, p) = test_thread();
        assert_eq!(
            sys_getdents(p, 0, BUF, INT_MAX as usize + 1),
            Err(Errno::EINVAL)
        );
        let w = open_by_hand(p, Some(node(A)), FWRITE);
        assert_eq!(sys_getdents(p, w, BUF, 512), Err(Errno::EBADF));
        close(p, w);
        let ffd = open(p, b"/f", O_RDONLY).unwrap();
        assert_eq!(sys_getdents(p, ffd, BUF, 512), Err(Errno::EINVAL));
        close(p, ffd);
        let fd = open(p, b"/a", O_RDONLY).unwrap();
        let fp = fd_getfile(p.fd(), fd).unwrap();
        fp.set_offset(-1);
        assert_eq!(sys_getdents(p, fd, BUF, 512), Err(Errno::EINVAL));
        fp.set_offset(0);
        clear_buf();
        let n = sys_getdents(p, fd, BUF, 1024).unwrap();
        assert_eq!(fp.offset(), n as Off);
        let b = user_bytes(0, n);
        let mut names = Vec::new();
        let mut at = 0;
        while at < n {
            let d = hal::sys::dirent::Dirent::from_bytes(&b[at..]).unwrap();
            assert_eq!(d.d_reclen % 8, 0);
            assert_eq!(
                usize::from(d.d_reclen),
                hal::sys::dirent::dirent_recsize(d.d_namlen.into())
            );
            let o = hal::sys::dirent::Dirent::NAME_OFFSET;
            names.push(b[at + o..at + o + usize::from(d.d_namlen)].to_vec());
            at += usize::from(d.d_reclen);
        }
        assert_eq!(at, n);
        assert_eq!(names[0], b".");
        assert_eq!(names[1], b"..");
        assert!(names.iter().any(|n| n == b"b"));
        assert!(names.iter().any(|n| n == b"abs"));
        // A short buffer takes the records that fit; then the rest; then nothing.
        fp.set_offset(0);
        let first = sys_getdents(p, fd, BUF, 40).unwrap();
        assert_eq!(first, 32);
        assert_eq!(sys_getdents(p, fd, BUF, 1024), Ok(n - first));
        assert_eq!(sys_getdents(p, fd, BUF, 1024), Ok(0));
        let _ = fp.frele(Some(p));
        close(p, fd);
        assert!(!testfs::any_locked());
    }

    /// `umask` (vfs_syscalls.c:3150-3162): the old mask back, the new one in ACCESSPERMS.
    #[test]
    fn umask_returns_the_old_mask_and_keeps_accessperms() {
        let (_g, p) = test_thread();
        let old = p.fd().cmask();
        assert_eq!(sys_umask(p, 0o7777), Ok(old));
        assert_eq!(p.fd().cmask(), 0o777);
        assert_eq!(sys_umask(p, 0o022), Ok(0o777));
    }

    /// `revoke` (vfs_syscalls.c:3169-3200): ENOTTY for a file that is not a character device
    /// of a tty's major (:3185-3189); the owner or root only (:3192-3194); the revocation only
    /// when the device is open more than once or aliased (:3195-3196).
    #[test]
    fn revoke_is_for_the_owner_of_a_tty_and_revokes_only_shared_ones() {
        let (_g, p) = test_thread();
        assert_eq!(sys_revoke(p, path(b"/f")), Err(Errno::ENOTTY));
        assert_eq!(sys_revoke(p, path(b"/dev/sd0c")), Err(Errno::ENOTTY));
        testfs::set_attr(0o620, 5);
        as_user(p, 1000);
        assert_eq!(sys_revoke(p, path(b"/dev/ttyp0")), Err(Errno::EPERM));
        as_user(p, 5);
        let n = testfs::REVOKES.load(Ordering::SeqCst);
        assert_eq!(sys_revoke(p, path(b"/dev/ttyp0")), Ok(()));
        assert_eq!(testfs::REVOKES.load(Ordering::SeqCst), n);
        let fd = open(p, b"/dev/ttyp0", O_RDONLY).unwrap();
        assert_eq!(sys_revoke(p, path(b"/dev/ttyp0")), Ok(()));
        assert_eq!(testfs::REVOKES.load(Ordering::SeqCst), n + 1);
        as_root(p);
        assert_eq!(sys_revoke(p, path(b"/dev/ttyp0")), Ok(()));
        close(p, fd);
        assert_eq!(usecounts()[TTY], 0);
    }

    // ---- __realpath, unveil, getvnode

    /// `__realpath` (vfs_syscalls.c:854-947): EINVAL for NULL, ENOENT for ""; an absolute
    /// path resolved, a relative one from the current directory (`vfs_getcwd_common`); the
    /// lookup's error.
    #[test]
    fn realpath_resolves_absolute_and_relative_paths() {
        let (_g, p) = test_thread();
        assert_eq!(
            sys___realpath(p, UserPtr::new(0), ubuf(0)),
            Err(Errno::EINVAL)
        );
        assert_eq!(sys___realpath(p, path(b""), ubuf(0)), Err(Errno::ENOENT));
        clear_buf();
        assert_eq!(sys___realpath(p, path(b"/a/./c/"), ubuf(0)), Ok(()));
        assert_eq!(user_bytes(0, 5), b"/a/c\0");
        assert_eq!(sys_chdir(p, path(b"/a")), Ok(()));
        clear_buf();
        assert_eq!(sys___realpath(p, path(b"c/../b"), ubuf(0)), Ok(()));
        assert_eq!(user_bytes(0, 5), b"/a/b\0");
        assert_eq!(
            sys___realpath(p, path(b"/a/nope/x"), ubuf(0)),
            Err(Errno::ENOENT)
        );
        assert!(!testfs::any_locked());
    }

    /// `unveil` (vfs_syscalls.c:950-1050): both NULL ends unveiling (`ps_uvdone`, :963-966),
    /// EPERM after (:968-969); EINVAL for a path shorter than 2 bytes with its NUL; "/" and
    /// "//" looked up (LOOKUP), any other name for creation (CREATE, the parent); EPERM when
    /// neither the file nor its directory gives any access (:1018-1036); the unveil added
    /// with the permissions; every reference released.
    #[test]
    fn unveil_adds_what_the_lookup_found_and_ends_on_null_null() {
        let (_g, p) = test_thread();
        let _ = listed(p);
        let perms = up(2, b"rw");
        let before = usecounts();
        let _ = testutil::file_log();
        assert_eq!(sys_unveil(p, path(b"/a"), perms), Ok(()));
        let log = testutil::file_log();
        assert!(
            log.iter().any(|l| l == "unveil_add vp=true dvp=true rw"),
            "{log:?}"
        );
        assert_eq!(sys_unveil(p, path(b"/"), perms), Ok(()));
        assert!(
            testutil::file_log()
                .iter()
                .any(|l| l == "unveil_add vp=true dvp=false rw")
        );
        assert_eq!(sys_unveil(p, path(b"//"), perms), Ok(()));
        assert!(
            testutil::file_log()
                .iter()
                .any(|l| l == "unveil_add vp=true dvp=false rw")
        );
        assert_eq!(sys_unveil(p, path(b"/a/new"), perms), Ok(()));
        assert!(
            testutil::file_log()
                .iter()
                .any(|l| l == "unveil_add vp=false dvp=true rw")
        );
        assert_eq!(sys_unveil(p, path(b""), perms), Err(Errno::EINVAL));
        testfs::set_deny_access(true);
        assert_eq!(sys_unveil(p, path(b"/a"), perms), Err(Errno::EPERM));
        assert!(
            !testutil::file_log()
                .iter()
                .any(|l| l.starts_with("unveil_add"))
        );
        testfs::set_deny_access(false);
        testutil::set_unveil_add(Some(Errno::E2BIG));
        assert_eq!(sys_unveil(p, path(b"/f"), perms), Err(Errno::E2BIG));
        testutil::set_unveil_add(None);
        assert_eq!(usecounts(), before);
        assert!(!testfs::any_locked());
        // unveil(NULL, NULL): no more.
        assert_eq!(sys_unveil(p, UserPtr::new(0), UserPtr::new(0)), Ok(()));
        assert!(testutil::unveil_done());
        assert_eq!(sys_unveil(p, path(b"/a"), perms), Err(Errno::EPERM));
        assert_eq!(sys_unveil(p, UserPtr::new(0), perms), Err(Errno::EPERM));
    }

    /// `getvnode` (vfs_syscalls.c:3208-3230): EBADF for no descriptor, EINVAL for a file
    /// that is no vnode, EBADF for a vnode of VBAD; the file with a reference otherwise.
    #[test]
    fn getvnode_refuses_other_files_and_bad_vnodes() {
        let (_g, p) = test_thread();
        assert!(matches!(getvnode(p, 99), Err(Errno::EBADF)));
        let other = open_by_hand(p, None, FREAD);
        assert!(matches!(getvnode(p, other), Err(Errno::EINVAL)));
        let fd = open(p, b"/f", O_RDONLY).unwrap();
        let fp = getvnode(p, fd).unwrap();
        assert!(ptr::eq(file_vp(&fp), node(F)));
        let _ = fp.frele(Some(p));
        node(F).set_vtype(VBAD);
        assert!(matches!(getvnode(p, fd), Err(Errno::EBADF)));
        node(F).set_vtype(VREG);
        close(p, fd);
        close(p, other);
    }

    /// The unused names of the tree stay what they were (`/a/c/d`, `/dev`, the console and
    /// `/a/abs`), which the tests above rely on.
    #[test]
    fn the_tree_is_the_test_file_systems() {
        let (_g, _p) = test_thread();
        assert_eq!(tree_find(C, b"d"), Some(D));
        assert_eq!(tree_find(ROOT, b"dev"), Some(DEV));
        assert_eq!(tree_find(DEV, b"ttyp0"), Some(TTY));
        assert_eq!(tree_find(DEV, b"sd0c"), Some(DISK));
        assert_eq!(tree_find(A, b"abs"), Some(ABS));
        assert_eq!(tree_find(ROOT, b"lnk"), Some(LNK));
        assert_eq!(tree_find(ROOT, b"s"), Some(S));
    }
}
/* </TESTS> */
