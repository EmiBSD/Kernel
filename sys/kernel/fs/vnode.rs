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
 *	@(#)vnode.h	8.11 (Berkeley) 11/21/94
 */

/*
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)vfs_vnops.c	8.5 (Berkeley) 12/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/vnode.h>`: the vnode, the focus of all file activity in UNIX. There is a unique
//! vnode allocated for each active file, each current directory, each mounted-on file, text
//! file and the root. Also the vnode types and tags, `struct vattr`, the `IO_*` and `V*`
//! flags; and `vn_lock` (`vfs_vnops.c`), the lock protocol every vnode lock goes through.
//!
//! Upstream: sys/sys/vnode.h @ 3ce1f3f79392
//! Upstream: sys/kern/vfs_vnops.c @ 3ce1f3f79392
//! LZ: sys/sys/vnode.rs@3c62ede68802
//! LZ: sys/kern/vfs_vnops.rs@3c62ede68802
//!
//! A vnode is an item of the pool `"vnodes"` that is never freed: [`super::subr::getnewvnode`]
//! either makes a new one or recycles one from the free lists, so it is handed around as
//! `&'static Vnode<G>`, with the C's `v_usecount` (`vref`/`vrele`/`vput`) saying who uses it.
//! A file system's operations are a table implementing [`Vnops`], found from the vnode by an
//! id ([`Vnode::ops`]).
//!
//! ## Deviations
//! - `v_un` (the union of `v_mountedhere`, `v_socket`, `v_specinfo`, `v_fifoinfo`) is the
//!   enum [`VnodeUn`], with accessors named after the C's shorthand macros; a reader of the
//!   wrong member gets `None`, where the C would reinterpret the pointer. `v_socket` is
//!   `bsd`'s member (the socket is `bsd`'s, [`FileGlue::VnodeG`]); `v_fifoinfo` is set by
//!   nothing (fifofs keeps its node in `v_data`, as LZ's port of it does).
//! - `si_specnext`, the link of the special-device alias chains, is the vnode's own
//!   `specnext` (LZ's deviation, kept): a list's link lives in its element.
//!
//! ## Redesign
//! - LZ's `struct vnode` of `Cell`s under one `unsafe impl Sync` (sys/sys/vnode.rs:226-279) ->
//!   the C's legend made types: the `[K]` words (`v_type`, `v_tag`, `v_flag`, `v_usecount`,
//!   `v_uvcount`, `v_writecount`, `v_id`, `v_op`) are Relaxed atomics behind LZ's accessor
//!   names (the C reads them unlocked and changes them under the kernel lock); `[V]`
//!   (`v_lflag`, `v_lockcount`) is a cell of the one `vnode_mtx` ([`VNODE_MTX`], `IPL_BIO`);
//!   `[B]` `v_holdcnt` a word written under the free lists' lock (the C's `splbio`); the
//!   free, mount and alias links hal's `ListEntry`s of the lists that own them; `v_mount`,
//!   `v_uvm` `Link`s;
//!   `v_un` a [`ReadMostly`] cell holding the device's [`Specinfo`] by value (freed by
//!   `vgonel`, as the C's `free(M_VNODE)`).
//! - `v_op` (a pointer compared with `&dead_vops` and `&spec_vops`) is an id: [`VopsId`], one
//!   byte into the subsystem's registry of tables ([`super::subr::VfsTables`]); `Dead`,
//!   `Spec` and `Sync` are fixed ids 0-2 and [`Vnode::kind`] answers the C's compares; a file
//!   system's table registers by its address on its first `getnewvnode`.
//! - `v_data` is the word [`Vnode::data`] (an `AtomicPtr`: the file system's node, read back
//!   with the file system's own cast, `bsd`'s `data<T>`); the members of `bsd`'s types (the
//!   name cache's heads, `v_socket`, the device's lock list) are the glue's
//!   [`FileGlue::VnodeG`], as `Process<G>` keeps its `bsd` members.
//! - `vn_lock` keeps the C's `Result` and protocol (the `v_lockcount` count under
//!   `vnode_mtx`, the unlock and `ENOENT` when `VXLOCK` appeared, `LK_RETRY`); [`Locked`] is
//!   the guard of a lock the caller leaves in the same function (its drop is `VOP_UNLOCK`).

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicPtr, AtomicU8, AtomicU32, Ordering};

use hal::link::Link;
use hal::list::ListEntry;
use hal::malloc::MBox;
use hal::mutex::Mutexed;
use hal::rwlock::Guarded;
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::types::{Dev, Gid, Mode, Nlink, Uid};

use super::bio::VnodeBufs;
use super::mount::Mount;
use super::spec::Specinfo;
use super::vops::Vnops;
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::event::Klist;
use crate::file::FileGlue;
use crate::proc::ProcGlue;
use crate::vm::vnode::Uvn;

/// `enum vtype`: vnode types. `VNON` means no type.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)] // OpenBSD names, verbatim
pub enum Vtype {
    /// No type.
    VNON,
    /// Regular file.
    VREG,
    /// Directory.
    VDIR,
    /// Block device.
    VBLK,
    /// Character device.
    VCHR,
    /// Symbolic link.
    VLNK,
    /// Socket.
    VSOCK,
    /// Fifo.
    VFIFO,
    /// A vnode that has been cleaned out (`vgone`).
    VBAD,
}

impl Vtype {
    /// The type stored as `n` (its discriminant); a byte no type has is `VBAD`.
    const fn from_u8(n: u8) -> Self {
        match n {
            0 => VNON,
            1 => VREG,
            2 => VDIR,
            3 => VBLK,
            4 => VCHR,
            5 => VLNK,
            6 => VSOCK,
            7 => VFIFO,
            _ => VBAD,
        }
    }
}

pub use Vtype::{VBAD, VBLK, VCHR, VDIR, VFIFO, VLNK, VNON, VREG, VSOCK};

/// `VTYPE_NAMES`.
pub const VTYPE_NAMES: [&str; 9] = [
    "VNON", "VREG", "VDIR", "VBLK", "VCHR", "VLNK", "VSOCK", "VFIFO", "VBAD",
];

/// `enum vtagtype`: vnode tag types. These are for the benefit of external programs only
/// (e.g., pstat) and should NEVER be inspected by the kernel. Note that `v_tag` is actually
/// used to tell MFS from FFS, and EXT2FS from the rest, so don't believe the above comment!
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)] // OpenBSD names, verbatim
pub enum Vtagtype {
    /// No file system.
    VT_NON,
    /// `ufs`.
    VT_UFS,
    /// `nfs`.
    VT_NFS,
    /// `mfs`.
    VT_MFS,
    /// `msdosfs`.
    VT_MSDOSFS,
    /// Unused.
    VT_PORTAL,
    /// Unused.
    VT_PROCFS,
    /// Unused.
    VT_AFS,
    /// `cd9660`.
    VT_ISOFS,
    /// Unused.
    VT_ADOSFS,
    /// `ext2fs`.
    VT_EXT2FS,
    /// The syncer vnode.
    VT_VFS,
    /// `ntfs`.
    VT_NTFS,
    /// `udf`.
    VT_UDF,
    /// `fuse`.
    VT_FUSEFS,
    /// `tmpfs`.
    VT_TMPFS,
}

impl Vtagtype {
    /// The tag stored as `n`; a byte no tag has is `VT_NON`.
    const fn from_u8(n: u8) -> Self {
        match n {
            1 => VT_UFS,
            2 => VT_NFS,
            3 => VT_MFS,
            4 => VT_MSDOSFS,
            5 => VT_PORTAL,
            6 => VT_PROCFS,
            7 => VT_AFS,
            8 => VT_ISOFS,
            9 => VT_ADOSFS,
            10 => VT_EXT2FS,
            11 => VT_VFS,
            12 => VT_NTFS,
            13 => VT_UDF,
            14 => VT_FUSEFS,
            15 => VT_TMPFS,
            _ => VT_NON,
        }
    }
}

pub use Vtagtype::{
    VT_ADOSFS, VT_AFS, VT_EXT2FS, VT_FUSEFS, VT_ISOFS, VT_MFS, VT_MSDOSFS, VT_NFS, VT_NON, VT_NTFS,
    VT_PORTAL, VT_PROCFS, VT_TMPFS, VT_UDF, VT_UFS, VT_VFS,
};

/// `VTAG_NAMES`.
pub const VTAG_NAMES: [&str; 16] = [
    "NON", "UFS", "NFS", "MFS", "MSDOSFS", "unused", "unused", "unused", "ISOFS", "unused",
    "EXT2FS", "VFS", "NTFS", "UDF", "FUSEFS", "TMPFS",
];

/// `VROOT`: root of its file system.
pub const VROOT: u32 = 0x0001;
/// `VTEXT`: vnode is a pure text prototype.
pub const VTEXT: u32 = 0x0002;
/// `VSYSTEM`: vnode being used by kernel.
pub const VSYSTEM: u32 = 0x0004;
/// `VISTTY`: vnode represents a tty.
pub const VISTTY: u32 = 0x0008;
/// `VXLOCK`: vnode is locked to change underlying type.
pub const VXLOCK: u32 = 0x0100;
/// `VXWANT`: process is waiting for vnode.
pub const VXWANT: u32 = 0x0200;
/// `VCLONED`: vnode was cloned.
pub const VCLONED: u32 = 0x0400;
/// `VALIASED`: vnode has an alias.
pub const VALIASED: u32 = 0x0800;
/// `VLARVAL`: vnode data not yet set up by higher level.
pub const VLARVAL: u32 = 0x1000;
/// `VLOCKSWORK`: FS supports locking discipline.
pub const VLOCKSWORK: u32 = 0x4000;
/// `VCLONE`: vnode is a clone.
pub const VCLONE: u32 = 0x8000;

/// `VBIOWAIT`: waiting for output to complete.
pub const VBIOWAIT: u32 = 0x0001;
/// `VBIOONSYNCLIST`: Vnode is on syncer worklist.
pub const VBIOONSYNCLIST: u32 = 0x0002;
/// `VBIOONFREELIST`: Vnode is on a free list.
pub const VBIOONFREELIST: u32 = 0x0004;
/// `VBIOERROR`: A write failed.
pub const VBIOERROR: u32 = 0x0008;

/// `struct vattr`: vnode attributes. A field value of `VNOVAL` represents a field whose value
/// is unavailable (getattr) or which is not to be changed (setattr). For the timespec fields,
/// only the `tv_nsec` member needs to be set to `VNOVAL`: if `tv_nsec != VNOVAL` then both
/// `tv_sec` and `tv_nsec` are valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vattr {
    /// `va_type`: vnode type (for create).
    pub va_type: Vtype,
    /// `va_mode`: files access mode and type.
    pub va_mode: Mode,
    /// `va_nlink`: number of references to file.
    pub va_nlink: Nlink,
    /// `va_uid`: owner user id.
    pub va_uid: Uid,
    /// `va_gid`: owner group id.
    pub va_gid: Gid,
    /// `va_fsid`: file system id (dev for now).
    pub va_fsid: i64,
    /// `va_fileid`: file id.
    pub va_fileid: u64,
    /// `va_size`: file size in bytes.
    pub va_size: u64,
    /// `va_blocksize`: blocksize preferred for i/o.
    pub va_blocksize: i64,
    /// `va_atime`: time of last access.
    pub va_atime: Timespec,
    /// `va_mtime`: time of last modification.
    pub va_mtime: Timespec,
    /// `va_ctime`: time file changed.
    pub va_ctime: Timespec,
    /// `va_gen`: generation number of file.
    pub va_gen: u64,
    /// `va_flags`: flags defined for file.
    pub va_flags: u64,
    /// `va_rdev`: device the special file represents.
    pub va_rdev: Dev,
    /// `va_bytes`: bytes of disk space held by file.
    pub va_bytes: u64,
    /// `va_filerev`: file modification number.
    pub va_filerev: u64,
    /// `va_vaflags`: operations flags, see below.
    pub va_vaflags: u32,
    /// `va_spare`: remain quad aligned.
    pub va_spare: i64,
}

impl Vattr {
    /// An all-zero `struct vattr` (a stack variable before `vattr_null` or a `getattr`).
    pub const fn new() -> Self {
        Self {
            va_type: VNON,
            va_mode: 0,
            va_nlink: 0,
            va_uid: 0,
            va_gid: 0,
            va_fsid: 0,
            va_fileid: 0,
            va_size: 0,
            va_blocksize: 0,
            va_atime: Timespec::new(0, 0),
            va_mtime: Timespec::new(0, 0),
            va_ctime: Timespec::new(0, 0),
            va_gen: 0,
            va_flags: 0,
            va_rdev: 0,
            va_bytes: 0,
            va_filerev: 0,
            va_vaflags: 0,
            va_spare: 0,
        }
    }
}

impl Default for Vattr {
    fn default() -> Self {
        Self::new()
    }
}

/// `VA_UTIMES_NULL`: utimes argument was NULL.
pub const VA_UTIMES_NULL: u32 = 0x01;
/// `VA_EXCLUSIVE`: exclusive create request.
pub const VA_EXCLUSIVE: u32 = 0x02;
/// `VA_UTIMES_CHANGE`: ctime should be updated.
pub const VA_UTIMES_CHANGE: u32 = 0x04;

/// `IO_UNIT`: do I/O as atomic unit.
pub const IO_UNIT: i32 = 0x01;
/// `IO_APPEND`: append write to end.
pub const IO_APPEND: i32 = 0x02;
/// `IO_SYNC`: do I/O synchronously.
pub const IO_SYNC: i32 = 0x04;
/// `IO_NODELOCKED`: underlying node already locked.
pub const IO_NODELOCKED: i32 = 0x08;
/// `IO_NDELAY`: `FNDELAY` flag set in file table.
pub const IO_NDELAY: i32 = 0x10;
/// `IO_NOLIMIT`: don't enforce limits on i/o.
pub const IO_NOLIMIT: i32 = 0x20;
/// `IO_NOCACHE`: don't cache result of this i/o.
pub const IO_NOCACHE: i32 = 0x40;

/// `VSUID`: set user id on execution.
pub const VSUID: Mode = 0o4000;
/// `VSGID`: set group id on execution.
pub const VSGID: Mode = 0o2000;
/// `VSVTX`: save swapped text even after use.
pub const VSVTX: Mode = 0o1000;
/// `VREAD`: read permission.
pub const VREAD: i32 = 0o0400;
/// `VWRITE`: write permission.
pub const VWRITE: i32 = 0o0200;
/// `VEXEC`: execute permission.
pub const VEXEC: i32 = 0o0100;

/// `VNOVAL`: token indicating no attribute value yet assigned. Stored into the unsigned
/// fields as the C's conversion does (all bits set).
pub const VNOVAL: i32 = -1;

/// `SKIPSYSTEM`: vflush: skip vnodes marked `VSYSTEM`.
pub const SKIPSYSTEM: i32 = 0x0001;
/// `FORCECLOSE`: vflush: force file closure.
pub const FORCECLOSE: i32 = 0x0002;
/// `WRITECLOSE`: vflush: only close writeable files.
pub const WRITECLOSE: i32 = 0x0004;
/// `DOCLOSE`: vclean: close active files.
pub const DOCLOSE: i32 = 0x0008;
/// `IGNORECLEAN`: vflush: ignore clean vnodes.
pub const IGNORECLEAN: i32 = 0x0010;
/// `V_SAVE`: vinvalbuf: sync file first.
pub const V_SAVE: i32 = 0x0001;
/// `V_SAVEMETA`: vinvalbuf: leave indirect blocks.
pub const V_SAVEMETA: i32 = 0x0002;

/// `REVOKEALL`: vop_revoke: revoke all aliases.
pub const REVOKEALL: i32 = 0x0001;

/// `GETCWD_CHECK_ACCESS`: `vfs_getcwd_common` checks search (and then read) access on the
/// way up.
pub const GETCWD_CHECK_ACCESS: i32 = 0x0001;

/// The `<sys/stat.h>` file types and permission bits `IFTOVT`, `VTTOIF` and `vaccess` read
/// (the ABI's values).
pub mod stat {
    use hal::sys::types::Mode;
    /// `S_IFMT`: type of file mask.
    pub const S_IFMT: Mode = 0o170000;
    /// `S_IFIFO`: named pipe (fifo).
    pub const S_IFIFO: Mode = 0o010000;
    /// `S_IFCHR`: character special.
    pub const S_IFCHR: Mode = 0o020000;
    /// `S_IFDIR`: directory.
    pub const S_IFDIR: Mode = 0o040000;
    /// `S_IFBLK`: block special.
    pub const S_IFBLK: Mode = 0o060000;
    /// `S_IFREG`: regular.
    pub const S_IFREG: Mode = 0o100000;
    /// `S_IFLNK`: symbolic link.
    pub const S_IFLNK: Mode = 0o120000;
    /// `S_IFSOCK`: socket.
    pub const S_IFSOCK: Mode = 0o140000;
    /// `S_IRUSR`: R for owner.
    pub const S_IRUSR: Mode = 0o000400;
    /// `S_IWUSR`: W for owner.
    pub const S_IWUSR: Mode = 0o000200;
    /// `S_IXUSR`: X for owner.
    pub const S_IXUSR: Mode = 0o000100;
    /// `S_IRGRP`: R for group.
    pub const S_IRGRP: Mode = 0o000040;
    /// `S_IWGRP`: W for group.
    pub const S_IWGRP: Mode = 0o000020;
    /// `S_IXGRP`: X for group.
    pub const S_IXGRP: Mode = 0o000010;
    /// `S_IROTH`: R for other.
    pub const S_IROTH: Mode = 0o000004;
    /// `S_IWOTH`: W for other.
    pub const S_IWOTH: Mode = 0o000002;
    /// `S_IXOTH`: X for other.
    pub const S_IXOTH: Mode = 0o000001;
}

use stat::{S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK};

/// `iftovt_tab[]`: the vnode type of each inode format (`IFTOVT`).
pub const IFTOVT_TAB: [Vtype; 16] = [
    VNON, VFIFO, VCHR, VNON, VDIR, VNON, VBLK, VNON, VREG, VNON, VLNK, VNON, VSOCK, VNON, VNON,
    VBAD,
];

/// `vttoif_tab[]`: the inode format of each vnode type (`VTTOIF`).
pub const VTTOIF_TAB: [Mode; 9] = [
    0, S_IFREG, S_IFDIR, S_IFBLK, S_IFCHR, S_IFLNK, S_IFSOCK, S_IFIFO, S_IFMT,
];

/// `IFTOVT(mode)`: the vnode type of an inode format.
pub fn iftovt(mode: Mode) -> Vtype {
    IFTOVT_TAB[((mode & S_IFMT) >> 12) as usize]
}

/// `VTTOIF(indx)`: the inode format of a vnode type.
pub fn vttoif(indx: Vtype) -> Mode {
    VTTOIF_TAB[indx as usize]
}

/// `MAKEIMODE(indx, mode)`.
pub fn makeimode(indx: Vtype, mode: Mode) -> Mode {
    vttoif(indx) | mode
}

/// `LK_EXCLUSIVE`: exclusive lock (`<sys/lock.h>`).
pub const LK_EXCLUSIVE: i32 = hal::rwlock::RW_WRITE;
/// `LK_SHARED`: shared lock.
pub const LK_SHARED: i32 = hal::rwlock::RW_READ;
/// `LK_TYPE_MASK`: type of lock sought.
pub const LK_TYPE_MASK: i32 = LK_EXCLUSIVE | LK_SHARED;
/// `LK_NOWAIT`: do not sleep to await lock.
pub const LK_NOWAIT: i32 = hal::rwlock::RW_NOSLEEP;
/// `LK_RECURSEFAIL`: fail if recursive exclusive lock.
pub const LK_RECURSEFAIL: i32 = hal::rwlock::RW_RECURSEFAIL;
/// `LK_EXCLOTHER`: exclusive lock held by some other thread.
pub const LK_EXCLOTHER: i32 = hal::rwlock::RW_WRITE_OTHER;
/// `LK_RWFLAGS`: the flags a file system's rrwlock takes.
pub const LK_RWFLAGS: i32 = LK_EXCLUSIVE | LK_SHARED | LK_NOWAIT | LK_RECURSEFAIL | LK_EXCLOTHER;
/// `LK_DRAIN`: wait for all lock activity to end.
pub const LK_DRAIN: i32 = 0x1000;
/// `LK_RETRY`: `vn_lock`: retry until locked.
pub const LK_RETRY: i32 = 0x2000;

/// `PINOD`: the priority of a sleep on a vnode (`<sys/param.h>`).
pub const PINOD: i32 = 8;

/// `vnode_mtx`: the `[V]` members of every vnode (`v_lflag`, `v_lockcount`) and their sleeps,
/// at `IPL_BIO` (vfs_subr.c:122).
pub static VNODE_MTX: Mutexed<()> = Mutexed::new(hal::intr::IPL_BIO, ());

/// The `[V]` members of a vnode, under [`VNODE_MTX`].
#[derive(Clone, Copy, Debug, Default)]
pub struct VLock {
    /// `v_lflag`: lock vnode flags (`VXLOCK`, `VXWANT`).
    pub lflag: u32,
    /// `v_lockcount`: # threads waiting on lock.
    pub lockcount: u32,
}

/// The id of a vnode's operations table (`v_op`): `Dead`, `Spec` and `Sync` are the
/// kernel's own tables, the others index the registry of the file systems' tables
/// ([`super::subr::VfsTables`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VopsId(pub(super) u8);

impl VopsId {
    /// `dead_vops`.
    pub const DEAD: VopsId = VopsId(0);
    /// `spec_vops`.
    pub const SPEC: VopsId = VopsId(1);
    /// `sync_vops`.
    pub const SYNC: VopsId = VopsId(2);
    /// The first id a file system's table gets.
    pub(super) const FIRST_FS: u8 = 3;
    /// The ids there are (the registry's size plus the kernel's three): 32 in a kernel, whose
    /// file systems have at most 25 tables (the C's count).
    #[cfg(not(any(test, feature = "doubles")))]
    pub(super) const MAX: u8 = 32;
    /// The ids there are in the host tests: their one process registers every file system's
    /// tables and the tests' own (the registry is never reset, so a table keeps its id from
    /// test to test), more than a kernel's 29 slots.
    #[cfg(any(test, feature = "doubles"))]
    pub(super) const MAX: u8 = 128;
}

/// What a vnode's operations are, for the C's compares of `v_op` with a table's address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VnodeKind {
    /// `v_op == &dead_vops`: revoked.
    Dead,
    /// `v_op == &spec_vops`: an anonymous device vnode.
    Spec,
    /// `v_op == &sync_vops`: a mount's syncer vnode.
    Sync,
    /// A file system's table.
    Fs(VopsId),
}

/// `v_un`: what a vnode of a given type points at.
pub enum VnodeUn<G: ProcGlue> {
    /// Nothing (the C's NULL in every member).
    None,
    /// `vu_mountedhere`: ptr to mounted vfs (`VDIR`).
    Mountedhere(&'static Mount<G>),
    /// `vu_specinfo`: device (`VCHR`, `VBLK`), owned by the vnode until `vgonel` frees it.
    Specinfo(MBox<Specinfo<G>>),
}

/// `struct vnode`.
///
/// Locks: \[K\] the kernel lock (the words are atomics, read unlocked as the C reads them),
/// \[V\] [`VNODE_MTX`], \[B\] the free lists' lock (`splbio`); the lists' links by the
/// lists' owners.
pub struct Vnode<G: ProcGlue> {
    /// `v_uvm`: uvm data (the vnode pager's object, kept across recycling).
    uvm: Link<Uvn>,
    /// `v_op`: vnode operations vector, as an id.
    ops: AtomicU8,
    /// `v_type`: vnode type.
    vtype: AtomicU8,
    /// `v_tag`: type of underlying data.
    tag: AtomicU8,
    /// `v_flag`: vnode flags.
    flag: AtomicU32,
    /// `v_usecount`: reference count of users.
    usecount: AtomicU32,
    /// `v_uvcount`: unveil references.
    uvcount: AtomicU32,
    /// `v_writecount`: reference count of writers.
    writecount: AtomicU32,
    /// `v_id`: capability identifier.
    id: AtomicU32,
    /// \[V\] `v_lflag`, `v_lockcount`.
    pub(super) vlock: Guarded<VLock>,
    /// \[B\] `v_holdcnt`: buffer references (written under the free lists' lock: a word
    /// of a foreign lock, read unlocked by the diagnostics).
    pub(super) holdcnt: AtomicU32,
    /// `v_mount`: ptr to vfs we are in.
    mount: Link<Mount<G>>,
    /// \[B\] `v_freelist`: vnode freelist.
    pub(super) freelist: ListEntry<Vnode<G>>,
    /// `v_mntvnodes`: vnodes for mount point.
    pub(super) mntvnodes: ListEntry<Vnode<G>>,
    /// \[B\] `v_bioflag`, `v_bufs_tree`, `v_cleanblkhd`, `v_dirtyblkhd`, `v_numoutput`: the
    /// vnode's buffers (the buffer cache's half).
    pub bufs: VnodeBufs,
    /// \[B\] `v_synclist`: vnode with dirty buffers (on the syncer's wheel).
    pub(super) synclist: ListEntry<Vnode<G>>,
    /// The wheel's slot the vnode is on while `VBIOONSYNCLIST` (under the syncer's lock).
    pub(super) syncslot: AtomicU8,
    /// `v_un`.
    un: ReadMostly<VnodeUn<G>>,
    /// `si_specnext`: the special-device alias chain.
    pub(super) specnext: ListEntry<Vnode<G>>,
    /// `v_data`: private data for fs.
    data: AtomicPtr<c_void>,
    /// `v_klist`: identity of poller(s).
    pub klist: Klist<G>,
    /// The members of `bsd`'s types (`v_nc_tree`, `v_cache_dst`, `v_socket`, the device's
    /// `si_lockf`).
    pub g: <G::FileGlue as FileGlue>::VnodeG,
}

impl<G: ProcGlue> Vnode<G> {
    /// A fresh vnode, as `pool_get(&vnode_pool, PR_ZERO)` returns it: its `[V]` cell keyed to
    /// `vnode_mtx`.
    pub(super) fn new() -> Self {
        Self {
            uvm: Link::new(),
            ops: AtomicU8::new(VopsId::DEAD.0),
            vtype: AtomicU8::new(VNON as u8),
            tag: AtomicU8::new(VT_NON as u8),
            flag: AtomicU32::new(0),
            usecount: AtomicU32::new(0),
            uvcount: AtomicU32::new(0),
            writecount: AtomicU32::new(0),
            id: AtomicU32::new(0),
            vlock: Guarded::new_static_mutexed(
                &VNODE_MTX,
                VLock {
                    lflag: 0,
                    lockcount: 0,
                },
            ),
            holdcnt: AtomicU32::new(0),
            mount: Link::new(),
            freelist: ListEntry::new(),
            mntvnodes: ListEntry::new(),
            bufs: VnodeBufs::new(),
            synclist: ListEntry::new(),
            syncslot: AtomicU8::new(0),
            un: ReadMostly::new(VnodeUn::None),
            specnext: ListEntry::new(),
            data: AtomicPtr::new(ptr::null_mut()),
            klist: Klist::new(),
            g: Default::default(),
        }
    }

    /// A vnode outside the vnode pool, for a test double (`bsd`'s file systems' unit tests
    /// build one by hand): deadfs's operations, on no list, no buffers' owner.
    #[cfg(any(test, feature = "doubles"))]
    pub fn new_unpooled() -> Self {
        Self::new()
    }

    /// `vp->v_op`: the vnode's operations table.
    #[inline]
    pub fn ops(&self) -> &'static dyn Vnops<G> {
        super::subr::vops_table::<G>(self.ops_id())
    }

    /// The id of `vp->v_op`.
    #[inline]
    pub fn ops_id(&self) -> VopsId {
        VopsId(self.ops.load(Ordering::Relaxed))
    }

    /// `vp->v_op = ops`, by id.
    pub fn set_ops_id(&self, id: VopsId) {
        self.ops.store(id.0, Ordering::Relaxed);
    }

    /// What the operations are: the C's `v_op == &dead_vops` (and `&spec_vops`,
    /// `&sync_vops`) compares.
    #[inline]
    pub fn kind(&self) -> VnodeKind {
        match self.ops_id() {
            VopsId::DEAD => VnodeKind::Dead,
            VopsId::SPEC => VnodeKind::Spec,
            VopsId::SYNC => VnodeKind::Sync,
            id => VnodeKind::Fs(id),
        }
    }

    /// `vp->v_type`.
    #[inline]
    pub fn vtype(&self) -> Vtype {
        Vtype::from_u8(self.vtype.load(Ordering::Relaxed))
    }

    /// `vp->v_type = t`.
    #[inline]
    pub fn set_vtype(&self, t: Vtype) {
        self.vtype.store(t as u8, Ordering::Relaxed);
    }

    /// `vp->v_tag`.
    #[inline]
    pub fn tag(&self) -> Vtagtype {
        Vtagtype::from_u8(self.tag.load(Ordering::Relaxed))
    }

    /// `vp->v_tag = t`.
    #[inline]
    pub fn set_tag(&self, t: Vtagtype) {
        self.tag.store(t as u8, Ordering::Relaxed);
    }

    /// `vp->v_flag`.
    #[inline]
    pub fn flag(&self) -> u32 {
        self.flag.load(Ordering::Relaxed)
    }

    /// `vp->v_flag = f`.
    #[inline]
    pub fn set_flag(&self, f: u32) {
        self.flag.store(f, Ordering::Relaxed);
    }

    /// `vp->v_flag |= f`.
    #[inline]
    pub fn flag_set(&self, f: u32) {
        self.flag.fetch_or(f, Ordering::Relaxed);
    }

    /// `vp->v_flag &= ~f`.
    #[inline]
    pub fn flag_clr(&self, f: u32) {
        self.flag.fetch_and(!f, Ordering::Relaxed);
    }

    /// `vp->v_usecount`.
    #[inline]
    pub fn usecount(&self) -> u32 {
        self.usecount.load(Ordering::Relaxed)
    }

    /// `vp->v_usecount = n`.
    #[inline]
    pub fn set_usecount(&self, n: u32) {
        self.usecount.store(n, Ordering::Relaxed);
    }

    /// `++vp->v_usecount`: the count after it. A load and a store, as the C's `++`: the
    /// kernel lock serializes the writers of the `[K]` words.
    #[inline]
    pub(super) fn usecount_inc(&self) -> u32 {
        let n = self.usecount.load(Ordering::Relaxed).wrapping_add(1);
        self.usecount.store(n, Ordering::Relaxed);
        n
    }

    /// `--vp->v_usecount`: the count after it (the kernel lock's, as `usecount_inc`).
    #[inline]
    pub(super) fn usecount_dec(&self) -> u32 {
        let n = self.usecount.load(Ordering::Relaxed).wrapping_sub(1);
        self.usecount.store(n, Ordering::Relaxed);
        n
    }

    /// `vp->v_uvcount`.
    #[inline]
    pub fn uvcount(&self) -> u32 {
        self.uvcount.load(Ordering::Relaxed)
    }

    /// `vp->v_uvcount = n`.
    #[inline]
    pub fn set_uvcount(&self, n: u32) {
        self.uvcount.store(n, Ordering::Relaxed);
    }

    /// `vp->v_writecount`.
    #[inline]
    pub fn writecount(&self) -> u32 {
        self.writecount.load(Ordering::Relaxed)
    }

    /// `vp->v_writecount = n`.
    #[inline]
    pub fn set_writecount(&self, n: u32) {
        self.writecount.store(n, Ordering::Relaxed);
    }

    /// `vp->v_id`.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id.load(Ordering::Relaxed)
    }

    /// `vp->v_id = id` (`cache_purge`'s new capability).
    #[inline]
    pub fn set_id(&self, id: u32) {
        self.id.store(id, Ordering::Relaxed);
    }

    /// `vp->v_mount`.
    #[inline]
    pub fn mount(&self) -> Option<&'static Mount<G>> {
        self.mount.get()
    }

    /// `vp->v_mount = mp`, for `insmntque` alone (which moves the vnode between the mounts'
    /// lists with it).
    pub(super) fn set_mount(&self, mp: Option<&'static Mount<G>>) {
        self.mount.set(mp);
    }

    /// `vp->v_uvm`.
    #[inline]
    pub fn uvn(&self) -> Option<&'static Uvn> {
        self.uvm.get()
    }

    /// `vp->v_uvm = uvn`.
    pub fn set_uvn(&self, uvn: &'static Uvn) {
        self.uvm.set(Some(uvn));
    }

    /// `vp->v_data`: the file system's node, as the word it stored.
    #[inline]
    pub fn data(&self) -> *mut c_void {
        self.data.load(Ordering::Relaxed)
    }

    /// `vp->v_data = p`.
    #[inline]
    pub fn set_data(&self, p: *mut c_void) {
        self.data.store(p, Ordering::Relaxed);
    }

    /// `vp->v_mountedhere`.
    pub fn mountedhere(&self) -> Option<&'static Mount<G>> {
        match &*self.un.read() {
            VnodeUn::Mountedhere(mp) => Some(*mp),
            _ => None,
        }
    }

    /// `vp->v_mountedhere = mp` (of a directory, whose `v_un` holds nothing else).
    pub fn set_mountedhere(&self, mp: Option<&'static Mount<G>>) {
        let old = self
            .un
            .replace(mp.map_or(VnodeUn::None, VnodeUn::Mountedhere));
        if matches!(old, VnodeUn::Specinfo(_)) {
            kpanic(format_args!(
                "vnode {self:p}: v_mountedhere over v_specinfo"
            ));
        }
    }

    /// Whether the vnode has a `v_specinfo` (a device vnode `checkalias` gave one).
    pub fn has_specinfo(&self) -> bool {
        matches!(&*self.un.read(), VnodeUn::Specinfo(_))
    }

    /// `f(vp->v_specinfo)`, `None` without one. `f` runs inside the cell's read: it must not
    /// sleep (nor take the cell again for writing).
    pub fn with_specinfo<R>(&self, f: impl FnOnce(&Specinfo<G>) -> R) -> Option<R> {
        match &*self.un.read() {
            VnodeUn::Specinfo(si) => Some(f(si)),
            _ => None,
        }
    }

    /// `vp->v_specinfo = si` (`checkalias`'s new alias): a device vnode with no `v_un` yet.
    pub(super) fn set_specinfo(&self, si: MBox<Specinfo<G>>) {
        let old = self.un.replace(VnodeUn::Specinfo(si));
        if !matches!(old, VnodeUn::None) {
            kpanic(format_args!("vnode {self:p}: checkalias over a v_un"));
        }
    }

    /// `vp->v_un` cleared (`vgonel`, `getnewvnode`), its specinfo given back to be freed.
    pub(super) fn take_un(&self) -> VnodeUn<G> {
        self.un.replace(VnodeUn::None)
    }

    /// `vp->v_rdev` (`v_specinfo->si_rdev`): panics for a vnode without a specinfo.
    pub fn rdev(&self) -> Dev {
        match self.with_specinfo(Specinfo::rdev) {
            Some(d) => d,
            None => kpanic(format_args!("vnode {self:p}: no v_specinfo")),
        }
    }

    /// `vp->v_specmountpoint` (`v_specinfo->si_mountpoint`): `None` without a specinfo.
    pub fn specmountpoint(&self) -> Option<&'static Mount<G>> {
        self.with_specinfo(Specinfo::mountpoint).flatten()
    }

    /// `vp->v_specmountpoint = mp`, when the vnode has a specinfo (a file system's mount and
    /// unmount of its device).
    pub fn set_specmountpoint(&self, mp: Option<&'static Mount<G>>) {
        let _ = self.with_specinfo(|si| si.set_mountpoint(mp));
    }

    /// `vp->v_specparent` (`v_specinfo->si_ci.ci_parent`): a clone's parent device.
    pub fn specparent(&self) -> Option<&'static Vnode<G>> {
        self.with_specinfo(Specinfo::parent).flatten()
    }

    /// The address `kinfo_file`'s `v_un` reports: the mount or the specinfo (the socket is
    /// `bsd`'s member), 0 for none.
    pub fn un_addr(&self) -> usize {
        match &*self.un.read() {
            VnodeUn::None => 0,
            VnodeUn::Mountedhere(mp) => ptr::from_ref(*mp).addr(),
            VnodeUn::Specinfo(si) => ptr::from_ref::<Specinfo<G>>(si).addr(),
        }
    }

    /// `[V]` `vp->v_lflag`, read under `vnode_mtx` (one section).
    pub fn lflag(&self) -> u32 {
        let g = VNODE_MTX.lock();
        self.vlock.get(&g).lflag
    }

    /// `[V]` `vp->v_lockcount`, read under `vnode_mtx`.
    pub fn lockcount(&self) -> u32 {
        let g = VNODE_MTX.lock();
        self.vlock.get(&g).lockcount
    }

    /// `[B]` `vp->v_holdcnt`.
    pub fn holdcnt(&self) -> u32 {
        self.holdcnt.load(Ordering::Relaxed)
    }

    /// The vnode's address as a number (for messages and identity).
    pub fn addr(&self) -> usize {
        ptr::from_ref(self).addr()
    }
}

/// `VN_KNOTE(vp, b)`: posts the vnode event `b` (`NOTE_*`) to the vnode's knotes, the
/// caller holding the kernel lock as the C's `knote_locked`.
#[allow(non_snake_case)] // the C macro's name
pub fn VN_KNOTE<G: ProcGlue>(vp: &Vnode<G>, b: u32) {
    vp.klist.knote_legacy(i64::from(b));
}

/// `vn_lock(vp, flags)`: lock the vnode by the file system's lock (`VOP_LOCK`), counted in
/// `v_lockcount` so `vclean` can wait for the would-be lockers to see `VXLOCK`; a vnode being
/// cleaned (`VXLOCK`) is waited for and then `ENOENT`, and so is one that became so while
/// its lock was taken (unlocked again first); `LK_RETRY` tries again until the lock is had.
pub fn vn_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    loop {
        let error;
        let mut g = VNODE_MTX.lock();
        if vp.vlock.get(&g).lflag & VXLOCK != 0 {
            vp.vlock.get_mut(&mut g).lflag |= VXWANT;
            let _ = g.sleep(ptr::from_ref(vp), PINOD, "vn_lock");
            drop(g);
            error = Errno::ENOENT;
        } else {
            vp.vlock.get_mut(&mut g).lockcount += 1;
            drop(g);

            let lock = vp.ops().lock(vp, flags);

            let mut g = VNODE_MTX.lock();
            let vl = vp.vlock.get_mut(&mut g);
            vl.lockcount -= 1;
            let do_wakeup = vl.lockcount == 0;
            let xlocked = vl.lflag & VXLOCK != 0;
            drop(g);

            match lock {
                Ok(()) => {
                    if !xlocked {
                        return Ok(());
                    }
                    // The vnode was exclusively locked while acquiring the requested lock.
                    // Release it and try again.
                    error = Errno::ENOENT;
                    let _ = vp.ops().unlock(vp);
                }
                Err(e) => error = e,
            }
            if do_wakeup && xlocked {
                hal::sleep::wakeup_one(ptr::from_ref(&vp.vlock));
            }
        }
        if flags & LK_RETRY == 0 {
            return Err(error);
        }
    }
}

/// A vnode locked by [`Locked::lock`] (`vn_lock` that succeeded): its drop is `VOP_UNLOCK`.
#[must_use = "dropping the guard at once unlocks the vnode"]
pub struct Locked<G: ProcGlue> {
    vp: &'static Vnode<G>,
}

impl<G: ProcGlue> Locked<G> {
    /// `vn_lock(vp, flags)`, the guard of the lock when it was had.
    pub fn lock(vp: &'static Vnode<G>, flags: i32) -> Result<Self, Errno> {
        vn_lock(vp, flags)?;
        Ok(Self { vp })
    }

    /// The vnode.
    pub fn vnode(&self) -> &'static Vnode<G> {
        self.vp
    }

    /// `VOP_UNLOCK(vp)`, at the C's own place.
    pub fn unlock(self) {
        drop(self);
    }

    /// The vnode, still locked, for a caller that leaves it locked (the C's protocol hands
    /// the lock on: `vget` with `LK_*`, `VFS_VGET`).
    pub fn forget(self) -> &'static Vnode<G> {
        let vp = self.vp;
        core::mem::forget(self);
        vp
    }
}

impl<G: ProcGlue> Drop for Locked<G> {
    fn drop(&mut self) {
        let _ = self.vp.ops().unlock(self.vp);
    }
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_types_and_tags_go_through_their_bytes() {
        for t in [VNON, VREG, VDIR, VBLK, VCHR, VLNK, VSOCK, VFIFO, VBAD] {
            assert_eq!(Vtype::from_u8(t as u8), t);
        }
        assert_eq!(Vtype::from_u8(200), VBAD);
        for (i, name) in VTAG_NAMES.iter().enumerate() {
            let t = Vtagtype::from_u8(i as u8);
            assert_eq!(t as usize, i, "{name}");
        }
    }

    #[test]
    fn iftovt_and_vttoif_are_the_c_s_tables() {
        assert_eq!(iftovt(stat::S_IFREG | 0o644), VREG);
        assert_eq!(iftovt(stat::S_IFDIR), VDIR);
        assert_eq!(iftovt(stat::S_IFCHR), VCHR);
        assert_eq!(iftovt(stat::S_IFBLK), VBLK);
        assert_eq!(iftovt(stat::S_IFLNK), VLNK);
        assert_eq!(iftovt(stat::S_IFSOCK), VSOCK);
        assert_eq!(iftovt(stat::S_IFIFO), VFIFO);
        assert_eq!(iftovt(stat::S_IFMT), VBAD);
        assert_eq!(iftovt(0), VNON);
        for t in [VREG, VDIR, VBLK, VCHR, VLNK, VSOCK, VFIFO] {
            assert_eq!(iftovt(vttoif(t)), t);
        }
        assert_eq!(makeimode(VREG, 0o755), stat::S_IFREG | 0o755);
    }
}
/* </TESTS> */
