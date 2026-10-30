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
 * Copyright (c) 1989, 1991, 1993
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
 *	@(#)mount.h	8.15 (Berkeley) 7/14/94
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
 *	@(#)ucred.h	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! The part of `<sys/mount.h>` the kernel and `bsd` share: the buffer cache statistics
//! (`struct bcachestats`), copied out whole by `vfs.generic.bcachestat`; and (N5.2) the
//! structures userland sees of a mount (`struct statfs` and its `fsid_t`, `struct fid`,
//! `fhandle_t`, the `vfs.generic.conf` image of `struct vfsconf`), the file system type names
//! (`MOUNT_*`), the mount flags (`MNT_*`), the `vfs_busy` flags (`VB_*`) and the `CTL_VFS`
//! names (`VFS_*`).
//!
//! Upstream: sys/sys/mount.h @ 3ce1f3f79392
//! Upstream: sys/kern/vfs_subr.c @ 3ce1f3f79392
//! Upstream: sys/sys/ucred.h @ 3ce1f3f79392
//! LZ: sys/sys/mount.rs@3c62ede68802
//! LZ: sys/kern/vfs_subr.rs@3c62ede68802
//! LZ: sys/sys/ucred.rs@3c62ede68802
//!
//! ## Redesign
//! - N5.1: `struct bcachestats` is `hal`'s, so that `kernel::fs::bio` (the counters' owner,
//!   `bcstats`) and `bsd` (`vfs.generic.bcachestat`'s copy-out, `vfs_subr.rs`) name one copy;
//!   `bsd`'s `sys/sys/mount.rs` re-exports it. It is `AbiPod` through [`crate::abi_pod!`]
//!   (LZ declared it there, in `vfs_bio.rs`). The rest of the header moves with the VFS
//!   (N5.2, N5.4).
//! - N5.2: `struct statfs`, `fsid_t`, `struct fid`, `fhandle_t`, the `MOUNT_*`, `MNT_*`,
//!   `VB_*` and `VFS_*` constants and `VfsconfAbi` (`vfs.generic.conf`'s copy of `struct
//!   vfsconf`, LZ's private struct in `vfs_subr.rs`) are `hal`'s, so that `kernel::fs::mount`
//!   (the mount's `mnt_stat`) and `bsd` (the file systems, `statfs(2)`, `getfsstat(2)`) name
//!   one copy; `bsd`'s `sys/sys/mount.rs` re-exports them. `struct statfs` and `fhandle_t` are
//!   `AbiPod` through [`crate::abi_pod!`] (LZ's two `unsafe impl AbiPod` go); the layout
//!   asserts came with them. The file systems' `*_args` stay `bsd`'s until N5.4.
//! - N5.4 (D-N5.4-3): the file systems' mount arguments (`struct export_args`, `ufs_args`,
//!   `mfs_args`, `iso_args`, `nfs_args`, `msdosfs_args`, `ntfs_args`, `udf_args`,
//!   `tmpfs_args`, `fusefs_args`, their constants, and `<sys/ucred.h>`'s `struct xucred`, which
//!   `export_args` embeds) are `hal`'s, so that the kernel's `mount(2)` (which copies the
//!   bytes in, `kernel::fs::syscalls`) and the file systems that read them name one copy;
//!   `bsd`'s `sys/sys/mount.rs` and `sys/sys/ucred.rs` re-export them. Each is `AbiPod`
//!   through [`crate::abi_pod!`], the holes the C compiler leaves named (`_pad0`, ...), and
//!   its `from_bytes` is `crate::user::pod_from_bytes` (LZ's three `read_unaligned` go); the
//!   layout asserts came with them. `ntfs_args` and `fusefs_args` lose their `cfg`s here
//!   (`option_ntfs`, `fuse`): a type costs nothing, and `bsd` builds their readers under
//!   them.

/// `struct bcachestats`: buffer cache statistics.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bcachestats {
    /// `numbufs`: number of buffers allocated.
    pub numbufs: i64,
    /// `numbufpages`: number of pages in buffer cache.
    pub numbufpages: i64,
    /// `numdirtypages`: number of dirty free pages.
    pub numdirtypages: i64,
    /// `numcleanpages`: number of clean free pages.
    pub numcleanpages: i64,
    /// `pendingwrites`: number of pending writes.
    pub pendingwrites: i64,
    /// `pendingreads`: number of pending reads.
    pub pendingreads: i64,
    /// `numwrites`: total writes started.
    pub numwrites: i64,
    /// `numreads`: total reads started.
    pub numreads: i64,
    /// `cachehits`: total reads found in cache.
    pub cachehits: i64,
    /// `busymapped`: number of busy and mapped buffers.
    pub busymapped: i64,
    /// `delwribufs`: delayed write buffers.
    pub delwribufs: i64,
    /// `kvaslots`: kva slots total.
    pub kvaslots: i64,
    /// `kvaslots_avail`: available kva slots.
    pub kvaslots_avail: i64,
}

// `#[repr(C)]`, thirteen `i64`s, no padding (`vfs.generic.bcachestat` copies it out).
crate::abi_pod!(Bcachestats {
    numbufs: i64,
    numbufpages: i64,
    numdirtypages: i64,
    numcleanpages: i64,
    pendingwrites: i64,
    pendingreads: i64,
    numwrites: i64,
    numreads: i64,
    cachehits: i64,
    busymapped: i64,
    delwribufs: i64,
    kvaslots: i64,
    kvaslots_avail: i64,
});

/// `fsid_t`: file system id type.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fsid {
    /// `val`.
    pub val: [i32; 2],
}

crate::abi_pod!(Fsid { val: [i32; 2] });

/// `MAXFIDSZ`: the bytes of a file identifier's data.
pub const MAXFIDSZ: usize = 16;

/// `struct fid`: file identifier.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fid {
    /// `fid_len`: length of data in bytes.
    pub fid_len: u16,
    /// `fid_reserved`: force longword alignment.
    pub fid_reserved: u16,
    /// `fid_data`: data (variable length).
    pub fid_data: [u8; MAXFIDSZ],
}

crate::abi_pod!(Fid {
    fid_len: u16,
    fid_reserved: u16,
    fid_data: [u8; MAXFIDSZ],
});

/// `MFSNAMELEN`: length of fs type name, including nul.
pub const MFSNAMELEN: usize = 16;
/// `MNAMELEN`: length of buffer for returned name.
pub const MNAMELEN: usize = 90;

/// `union mount_info`: the per-file-system mount arguments `statfs(2)` returns, as its 160
/// bytes (8-aligned); the file systems read and write their own `*_args` views of it.
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct MountInfo {
    /// `__align`: the union's bytes.
    pub __align: [u8; 160],
}

crate::abi_pod!(MountInfo { __align: [u8; 160] });

/// `struct statfs`: file system statistics. The holes the C compiler leaves are named
/// (`_pad0` after `f_iosize`, `_pad1` before `mount_info`), so the structure is plain data
/// `copyout` may read whole.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Statfs {
    /// `f_flags`: copy of mount flags.
    pub f_flags: u32,
    /// `f_bsize`: file system block size.
    pub f_bsize: u32,
    /// `f_iosize`: optimal transfer block size.
    pub f_iosize: u32,
    /// The hole after `f_iosize`.
    pub _pad0: u32,
    /// `f_blocks`: total data blocks in file system.
    pub f_blocks: u64,
    /// `f_bfree`: free blocks in fs.
    pub f_bfree: u64,
    /// `f_bavail`: free blocks avail to non-superuser.
    pub f_bavail: i64,
    /// `f_files`: total file nodes in file system.
    pub f_files: u64,
    /// `f_ffree`: free file nodes in fs.
    pub f_ffree: u64,
    /// `f_favail`: free file nodes avail to non-root.
    pub f_favail: i64,
    /// `f_syncwrites`: count of sync writes since mount.
    pub f_syncwrites: u64,
    /// `f_syncreads`: count of sync reads since mount.
    pub f_syncreads: u64,
    /// `f_asyncwrites`: count of async writes since mount.
    pub f_asyncwrites: u64,
    /// `f_asyncreads`: count of async reads since mount.
    pub f_asyncreads: u64,
    /// `f_fsid`: file system id.
    pub f_fsid: Fsid,
    /// `f_namemax`: maximum filename length.
    pub f_namemax: u32,
    /// `f_owner`: user that mounted the file system.
    pub f_owner: crate::sys::types::Uid,
    /// `f_ctime`: last mount \[-u\] time.
    pub f_ctime: u64,
    /// `f_fstypename`: fs type name.
    pub f_fstypename: [u8; MFSNAMELEN],
    /// `f_mntonname`: directory on which mounted.
    pub f_mntonname: [u8; MNAMELEN],
    /// `f_mntfromname`: mounted file system.
    pub f_mntfromname: [u8; MNAMELEN],
    /// `f_mntfromspec`: special for mount request.
    pub f_mntfromspec: [u8; MNAMELEN],
    /// The hole before `mount_info`.
    pub _pad1: [u8; 2],
    /// `mount_info`: per-filesystem mount options.
    pub mount_info: MountInfo,
}

impl Statfs {
    /// An all-zero `struct statfs`.
    pub const fn new() -> Self {
        Self {
            f_flags: 0,
            f_bsize: 0,
            f_iosize: 0,
            _pad0: 0,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 0,
            f_ffree: 0,
            f_favail: 0,
            f_syncwrites: 0,
            f_syncreads: 0,
            f_asyncwrites: 0,
            f_asyncreads: 0,
            f_fsid: Fsid { val: [0; 2] },
            f_namemax: 0,
            f_owner: 0,
            f_ctime: 0,
            f_fstypename: [0; MFSNAMELEN],
            f_mntonname: [0; MNAMELEN],
            f_mntfromname: [0; MNAMELEN],
            f_mntfromspec: [0; MNAMELEN],
            _pad1: [0; 2],
            mount_info: MountInfo { __align: [0; 160] },
        }
    }
}

impl Default for Statfs {
    fn default() -> Self {
        Self::new()
    }
}

// `#[repr(C)]` integers and bytes, the two holes named: no implicit padding (the layout
// asserts below pin it).
crate::abi_pod!(Statfs {
    f_flags: u32,
    f_bsize: u32,
    f_iosize: u32,
    _pad0: u32,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: i64,
    f_files: u64,
    f_ffree: u64,
    f_favail: i64,
    f_syncwrites: u64,
    f_syncreads: u64,
    f_asyncwrites: u64,
    f_asyncreads: u64,
    f_fsid: Fsid,
    f_namemax: u32,
    f_owner: crate::sys::types::Uid,
    f_ctime: u64,
    f_fstypename: [u8; MFSNAMELEN],
    f_mntonname: [u8; MNAMELEN],
    f_mntfromname: [u8; MNAMELEN],
    f_mntfromspec: [u8; MNAMELEN],
    _pad1: [u8; 2],
    mount_info: MountInfo,
});

/// `MOUNT_FFS`: UNIX "Fast" Filesystem.
pub const MOUNT_FFS: &[u8] = b"ffs";
/// `MOUNT_UFS`: for compatibility.
pub const MOUNT_UFS: &[u8] = MOUNT_FFS;
/// `MOUNT_NFS`: Network Filesystem.
pub const MOUNT_NFS: &[u8] = b"nfs";
/// `MOUNT_MFS`: Memory Filesystem.
pub const MOUNT_MFS: &[u8] = b"mfs";
/// `MOUNT_MSDOS`: MSDOS Filesystem.
pub const MOUNT_MSDOS: &[u8] = b"msdos";
/// `MOUNT_AFS`: Andrew Filesystem.
pub const MOUNT_AFS: &[u8] = b"afs";
/// `MOUNT_CD9660`: ISO9660 (aka CDROM) Filesystem.
pub const MOUNT_CD9660: &[u8] = b"cd9660";
/// `MOUNT_EXT2FS`: Second Extended Filesystem.
pub const MOUNT_EXT2FS: &[u8] = b"ext2fs";
/// `MOUNT_NCPFS`: NetWare Network File System.
pub const MOUNT_NCPFS: &[u8] = b"ncpfs";
/// `MOUNT_NTFS`: NTFS.
pub const MOUNT_NTFS: &[u8] = b"ntfs";
/// `MOUNT_UDF`: UDF.
pub const MOUNT_UDF: &[u8] = b"udf";
/// `MOUNT_TMPFS`: tmpfs.
pub const MOUNT_TMPFS: &[u8] = b"tmpfs";
/// `MOUNT_FUSEFS`: FUSE.
pub const MOUNT_FUSEFS: &[u8] = b"fuse";

/// `MNT_RDONLY`: read only filesystem.
pub const MNT_RDONLY: i32 = 0x0000_0001;
/// `MNT_SYNCHRONOUS`: file system written synchronously.
pub const MNT_SYNCHRONOUS: i32 = 0x0000_0002;
/// `MNT_NOEXEC`: can't exec from filesystem.
pub const MNT_NOEXEC: i32 = 0x0000_0004;
/// `MNT_NOSUID`: don't honor setuid bits on fs.
pub const MNT_NOSUID: i32 = 0x0000_0008;
/// `MNT_NODEV`: don't interpret special files.
pub const MNT_NODEV: i32 = 0x0000_0010;
/// `MNT_NOPERM`: don't enforce permission checks.
pub const MNT_NOPERM: i32 = 0x0000_0020;
/// `MNT_ASYNC`: file system written asynchronously.
pub const MNT_ASYNC: i32 = 0x0000_0040;
/// `MNT_WXALLOWED`: filesystem allows W|X mappings.
pub const MNT_WXALLOWED: i32 = 0x0000_0800;
/// `MNT_EXRDONLY`: exported read only.
pub const MNT_EXRDONLY: i32 = 0x0000_0080;
/// `MNT_EXPORTED`: file system is exported.
pub const MNT_EXPORTED: i32 = 0x0000_0100;
/// `MNT_DEFEXPORTED`: exported to the world.
pub const MNT_DEFEXPORTED: i32 = 0x0000_0200;
/// `MNT_EXPORTANON`: use anon uid mapping for everyone.
pub const MNT_EXPORTANON: i32 = 0x0000_0400;
/// `MNT_LOCAL`: filesystem is stored locally.
pub const MNT_LOCAL: i32 = 0x0000_1000;
/// `MNT_QUOTA`: quotas are enabled on filesystem.
pub const MNT_QUOTA: i32 = 0x0000_2000;
/// `MNT_ROOTFS`: identifies the root filesystem.
pub const MNT_ROOTFS: i32 = 0x0000_4000;
/// `MNT_NOATIME`: don't update access times on fs.
pub const MNT_NOATIME: i32 = 0x0000_8000;
/// `MNT_VISFLAGMASK`: mask of all the flags a `statfs` shows.
pub const MNT_VISFLAGMASK: i32 = 0x0400_ffff;
/// `MNT_BITS`: the `%b` description of the flags.
pub const MNT_BITS: &[u8] = b"\x10\x01RDONLY\x02SYNCHRONOUS\x03NOEXEC\x04NOSUID\x05NODEV\x06NOPERM\
\x07ASYNC\x08EXRDONLY\x09EXPORTED\x0aDEFEXPORTED\x0bEXPORTANON\
\x0cWXALLOWED\x0dLOCAL\x0eQUOTA\x0fROOTFS\x10NOATIME\x11UPDATE\
\x12DELEXPORT\x13RELOAD\x14FORCE\x15STALLED\x16SWAPPABLE\x19UNMOUNT\
\x1aWANTRDWR\x1bSOFTDEP\x1cDOOMED";
/// `MNT_UPDATE`: not a real mount, just an update.
pub const MNT_UPDATE: i32 = 0x0001_0000;
/// `MNT_DELEXPORT`: delete export host lists.
pub const MNT_DELEXPORT: i32 = 0x0002_0000;
/// `MNT_RELOAD`: reload filesystem data.
pub const MNT_RELOAD: i32 = 0x0004_0000;
/// `MNT_FORCE`: force unmount or readonly change.
pub const MNT_FORCE: i32 = 0x0008_0000;
/// `MNT_STALLED`: filesystem stalled.
pub const MNT_STALLED: i32 = 0x0010_0000;
/// `MNT_SWAPPABLE`: filesystem can be used for swap.
pub const MNT_SWAPPABLE: i32 = 0x0020_0000;
/// `MNT_UNMOUNT`: unmount in progress.
pub const MNT_UNMOUNT: i32 = 0x0100_0000;
/// `MNT_WANTRDWR`: want upgrade to read/write.
pub const MNT_WANTRDWR: i32 = 0x0200_0000;
/// `MNT_SOFTDEP`: soft dependencies being done.
pub const MNT_SOFTDEP: i32 = 0x0400_0000;
/// `MNT_DOOMED`: device behind filesystem is gone.
pub const MNT_DOOMED: i32 = 0x0800_0000;
/// `MNT_OP_FLAGS`: the external operation flags.
pub const MNT_OP_FLAGS: i32 = MNT_UPDATE | MNT_RELOAD | MNT_FORCE | MNT_WANTRDWR;

/// `MNT_WAIT`: synchronously wait for I/O to complete.
pub const MNT_WAIT: i32 = 1;
/// `MNT_NOWAIT`: start all I/O, but do not wait for it.
pub const MNT_NOWAIT: i32 = 2;
/// `MNT_LAZY`: push data not written by filesystem syncer.
pub const MNT_LAZY: i32 = 3;

/// `fhandle_t`: generic file handle.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fhandle {
    /// `fh_fsid`: file system id of mount point.
    pub fh_fsid: Fsid,
    /// `fh_fid`: file system specific id.
    pub fh_fid: Fid,
}

crate::abi_pod!(Fhandle {
    fh_fsid: Fsid,
    fh_fid: Fid,
});

/// `VFS_GENERIC`: generic filesystem information.
pub const VFS_GENERIC: i32 = 0;
/// `VFS_MAXTYPENUM`: int: highest defined filesystem type.
pub const VFS_MAXTYPENUM: i32 = 1;
/// `VFS_CONF`: struct: vfsconf for filesystem given as next argument.
pub const VFS_CONF: i32 = 2;
/// `VFS_BCACHESTAT`: struct: buffer cache statistics.
pub const VFS_BCACHESTAT: i32 = 3;

/// `VB_READ`: `vfs_busy` for reading.
pub const VB_READ: i32 = 0x01;
/// `VB_WRITE`: `vfs_busy` for writing.
pub const VB_WRITE: i32 = 0x02;
/// `VB_NOWAIT`: immediately fail on busy lock.
pub const VB_NOWAIT: i32 = 0x04;
/// `VB_WAIT`: sleep fail on busy lock.
pub const VB_WAIT: i32 = 0x08;
/// `VB_DUPOK`: permit duplicate mount busying.
pub const VB_DUPOK: i32 = 0x10;

/// `struct vfsconf` as `vfs.generic.conf` copies it out: the operations pointer cleared, the
/// hole after `vfc_flags` named.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VfsconfAbi {
    /// `vfc_vfsops`: 0 (a kernel pointer is not copied out).
    pub vfc_vfsops: u64,
    /// `vfc_name`: filesystem type name.
    pub vfc_name: [u8; MFSNAMELEN],
    /// `vfc_typenum`: historic filesystem type number.
    pub vfc_typenum: i32,
    /// `vfc_refcount`: number mounted of this type.
    pub vfc_refcount: u32,
    /// `vfc_flags`: permanent flags.
    pub vfc_flags: i32,
    /// The hole after `vfc_flags`.
    pub _pad: u32,
    /// `vfc_datasize`: size of data args.
    pub vfc_datasize: u64,
}

// `#[repr(C)]` integers and bytes with the one hole named; no padding.
crate::abi_pod!(VfsconfAbi {
    vfc_vfsops: u64,
    vfc_name: [u8; MFSNAMELEN],
    vfc_typenum: i32,
    vfc_refcount: u32,
    vfc_flags: i32,
    _pad: u32,
    vfc_datasize: u64,
});

/// `struct xucred` (`<sys/ucred.h>`): userspace version of the credentials, for use in
/// syscalls arguments: `export_args`'s anonymous mapping (N5.4: here with the mount
/// arguments that embed it; `bsd`'s `sys/sys/ucred.rs` re-exports it). The hole after
/// `cr_ngroups` is named.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Xucred {
    /// `cr_uid`: user id.
    pub cr_uid: crate::sys::types::Uid,
    /// `cr_gid`: group id.
    pub cr_gid: crate::sys::types::Gid,
    /// `cr_ngroups`: number of groups.
    pub cr_ngroups: i16,
    /// The hole after `cr_ngroups`.
    pub _pad0: u16,
    /// `cr_groups`: groups.
    pub cr_groups: [crate::sys::types::Gid; crate::sys::syslimits::NGROUPS_MAX],
}

// `#[repr(C)]` integers with the one hole named: no padding.
crate::abi_pod!(Xucred {
    cr_uid: crate::sys::types::Uid,
    cr_gid: crate::sys::types::Gid,
    cr_ngroups: i16,
    _pad0: u16,
    cr_groups: [crate::sys::types::Gid; crate::sys::syslimits::NGROUPS_MAX],
});

/// `struct export_args`: export arguments for local filesystem mount calls. The two
/// `struct sockaddr *` are user addresses (`usize`), which the kernel copies in when it
/// exports (`NFSSERVER`). The holes after `ex_anon`, `ex_addrlen` and `ex_masklen` are
/// named.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ExportArgs {
    /// `ex_flags`: export related flags.
    pub ex_flags: i32,
    /// `ex_root`: mapping for root uid.
    pub ex_root: crate::sys::types::Uid,
    /// `ex_anon`: mapping for anonymous user.
    pub ex_anon: Xucred,
    /// The hole after `ex_anon`.
    pub _pad0: u32,
    /// `ex_addr`: net address to which exported.
    pub ex_addr: usize,
    /// `ex_addrlen`: and the net address length.
    pub ex_addrlen: i32,
    /// The hole after `ex_addrlen`.
    pub _pad1: u32,
    /// `ex_mask`: mask of valid bits in saddr.
    pub ex_mask: usize,
    /// `ex_masklen`: and the smask length.
    pub ex_masklen: i32,
    /// The hole after `ex_masklen`.
    pub _pad2: u32,
}

crate::abi_pod!(ExportArgs {
    ex_flags: i32,
    ex_root: crate::sys::types::Uid,
    ex_anon: Xucred,
    _pad0: u32,
    ex_addr: usize,
    ex_addrlen: i32,
    _pad1: u32,
    ex_mask: usize,
    ex_masklen: i32,
    _pad2: u32,
});

/// `struct ufs_args`: arguments to mount UFS-based filesystems. `fspec` is the user address
/// of the block special device's name.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UfsArgs {
    /// `fspec`: block special device to mount.
    pub fspec: usize,
    /// `export_info`: network export information.
    pub export_info: ExportArgs,
}

impl UfsArgs {
    /// `sizeof(struct ufs_args)`: the `vfc_datasize` of the UFS file systems.
    pub const SIZE: usize = size_of::<UfsArgs>();

    /// The arguments in the kernel copy `sys_mount` made of them (at least `SIZE` bytes),
    /// `None` when there are none (the C's NULL `data`).
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        crate::user::pod_from_bytes(data)
    }
}

crate::abi_pod!(UfsArgs {
    fspec: usize,
    export_info: ExportArgs,
});

/// `struct mfs_args`: arguments to mount MFS. `fspec` is the user address of the name to
/// export for `statfs`, `base` the user address of the file system in the memory of the
/// process that mounts it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MfsArgs {
    /// `fspec`: name to export for statfs.
    pub fspec: usize,
    /// `export_info`: if exported MFSes are supported.
    pub export_info: ExportArgs,
    /// `base`: base of file system in memory.
    pub base: usize,
    /// `size`: size of file system.
    pub size: u64,
}

impl MfsArgs {
    /// `sizeof(struct mfs_args)`: the `vfc_datasize` of MFS.
    pub const SIZE: usize = size_of::<MfsArgs>();

    /// The arguments in the kernel copy `sys_mount` made of them (at least `SIZE` bytes),
    /// `None` when there are none (the C's NULL `data`).
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        crate::user::pod_from_bytes(data)
    }
}

crate::abi_pod!(MfsArgs {
    fspec: usize,
    export_info: ExportArgs,
    base: usize,
    size: u64,
});

/// The `SIZE` and `from_bytes` of a file system's mount arguments, as [`UfsArgs`] has them:
/// `sys_mount` copies `vfc_datasize` bytes in, and the file system reads its structure out
/// (`pod_from_bytes`: the first `SIZE` bytes, `None` for fewer).
macro_rules! mount_args_from_bytes {
    ($t:ident, $c:literal) => {
        impl $t {
            #[doc = concat!("`sizeof(struct ", $c, ")`: the `vfc_datasize` of its file system.")]
            pub const SIZE: usize = size_of::<$t>();

            /// The arguments in the kernel copy `sys_mount` made of them (at least `SIZE`
            /// bytes), `None` when there are none (the C's NULL `data`).
            pub fn from_bytes(data: &[u8]) -> Option<Self> {
                crate::user::pod_from_bytes(data)
            }
        }
    };
}

/// `struct iso_args`: arguments to mount ISO 9660 filesystems. `fspec` is a user address.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IsoArgs {
    /// `fspec`: block special device to mount.
    pub fspec: usize,
    /// `export_info`: network export info.
    pub export_info: ExportArgs,
    /// `flags`: mounting flags, the `ISOFSMNT_*` below.
    pub flags: i32,
    /// `sess`: start sector of session.
    pub sess: i32,
}

mount_args_from_bytes!(IsoArgs, "iso_args");

crate::abi_pod!(IsoArgs {
    fspec: usize,
    export_info: ExportArgs,
    flags: i32,
    sess: i32,
});

/// `ISOFSMNT_NORRIP`: disable Rock Ridge Ext.
pub const ISOFSMNT_NORRIP: i32 = 0x0000_0001;
/// `ISOFSMNT_GENS`: enable generation numbers.
pub const ISOFSMNT_GENS: i32 = 0x0000_0002;
/// `ISOFSMNT_EXTATT`: enable extended attr.
pub const ISOFSMNT_EXTATT: i32 = 0x0000_0004;
/// `ISOFSMNT_NOJOLIET`: disable Joliet Ext.
pub const ISOFSMNT_NOJOLIET: i32 = 0x0000_0008;
/// `ISOFSMNT_SESS`: use `iso_args.sess`.
pub const ISOFSMNT_SESS: i32 = 0x0000_0010;

/// `NFS_ARGSVERSION`: change when `nfs_args` changes.
pub const NFS_ARGSVERSION: i32 = 4;

/// `struct nfs_args`: arguments to mount NFS. `addr`, `fh` and `hostname` are user addresses
/// (the C's `struct sockaddr *`, `u_char *` and `char *`), which the NFS mount copies in.
/// The holes after `version`, `proto` and `deadthresh` are named.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NfsArgs {
    /// `version`: args structure version number (`NFS_ARGSVERSION`).
    pub version: i32,
    /// The hole after `version`.
    pub _pad0: u32,
    /// `addr`: file server address.
    pub addr: usize,
    /// `addrlen`: length of address.
    pub addrlen: i32,
    /// `sotype`: socket type.
    pub sotype: i32,
    /// `proto`: and protocol.
    pub proto: i32,
    /// The hole after `proto`.
    pub _pad1: u32,
    /// `fh`: file handle to be mounted.
    pub fh: usize,
    /// `fhsize`: size, in bytes, of `fh`.
    pub fhsize: i32,
    /// `flags`: the `NFSMNT_*` below.
    pub flags: i32,
    /// `wsize`: write size in bytes.
    pub wsize: i32,
    /// `rsize`: read size in bytes.
    pub rsize: i32,
    /// `readdirsize`: readdir size in bytes.
    pub readdirsize: i32,
    /// `timeo`: initial timeout in .1 secs.
    pub timeo: i32,
    /// `retrans`: times to retry send.
    pub retrans: i32,
    /// `maxgrouplist`: max. size of group list.
    pub maxgrouplist: i32,
    /// `readahead`: number of blocks to readahead.
    pub readahead: i32,
    /// `leaseterm`: term (sec) of lease.
    pub leaseterm: i32,
    /// `deadthresh`: retrans threshold.
    pub deadthresh: i32,
    /// The hole after `deadthresh`.
    pub _pad2: u32,
    /// `hostname`: server's name.
    pub hostname: usize,
    /// `acregmin`: attr cache file recently modified.
    pub acregmin: i32,
    /// `acregmax`: ac file not recently modified.
    pub acregmax: i32,
    /// `acdirmin`: ac for dir recently modified.
    pub acdirmin: i32,
    /// `acdirmax`: ac for dir not recently modified.
    pub acdirmax: i32,
}

mount_args_from_bytes!(NfsArgs, "nfs_args");

crate::abi_pod!(NfsArgs {
    version: i32,
    _pad0: u32,
    addr: usize,
    addrlen: i32,
    sotype: i32,
    proto: i32,
    _pad1: u32,
    fh: usize,
    fhsize: i32,
    flags: i32,
    wsize: i32,
    rsize: i32,
    readdirsize: i32,
    timeo: i32,
    retrans: i32,
    maxgrouplist: i32,
    readahead: i32,
    leaseterm: i32,
    deadthresh: i32,
    _pad2: u32,
    hostname: usize,
    acregmin: i32,
    acregmax: i32,
    acdirmin: i32,
    acdirmax: i32,
});

/// `NFSMNT_RESVPORT`: always use reserved ports.
pub const NFSMNT_RESVPORT: i32 = 0x0000_0000;
/// `NFSMNT_SOFT`: soft mount (hard is default).
pub const NFSMNT_SOFT: i32 = 0x0000_0001;
/// `NFSMNT_WSIZE`: set write size.
pub const NFSMNT_WSIZE: i32 = 0x0000_0002;
/// `NFSMNT_RSIZE`: set read size.
pub const NFSMNT_RSIZE: i32 = 0x0000_0004;
/// `NFSMNT_TIMEO`: set initial timeout.
pub const NFSMNT_TIMEO: i32 = 0x0000_0008;
/// `NFSMNT_RETRANS`: set number of request retries.
pub const NFSMNT_RETRANS: i32 = 0x0000_0010;
/// `NFSMNT_MAXGRPS`: set maximum grouplist size.
pub const NFSMNT_MAXGRPS: i32 = 0x0000_0020;
/// `NFSMNT_INT`: allow interrupts on hard mount.
pub const NFSMNT_INT: i32 = 0x0000_0040;
/// `NFSMNT_NOCONN`: don't connect the socket.
pub const NFSMNT_NOCONN: i32 = 0x0000_0080;
/// `NFSMNT_NQNFS`: use Nqnfs protocol.
pub const NFSMNT_NQNFS: i32 = 0x0000_0100;
/// `NFSMNT_NFSV3`: use NFS version 3 protocol.
pub const NFSMNT_NFSV3: i32 = 0x0000_0200;
/// `NFSMNT_KERB`: use Kerberos authentication.
pub const NFSMNT_KERB: i32 = 0x0000_0400;
/// `NFSMNT_DUMBTIMR`: don't estimate rtt dynamically.
pub const NFSMNT_DUMBTIMR: i32 = 0x0000_0800;
/// `NFSMNT_LEASETERM`: set lease term (nqnfs).
pub const NFSMNT_LEASETERM: i32 = 0x0000_1000;
/// `NFSMNT_READAHEAD`: set read ahead.
pub const NFSMNT_READAHEAD: i32 = 0x0000_2000;
/// `NFSMNT_DEADTHRESH`: set dead server retry thresh.
pub const NFSMNT_DEADTHRESH: i32 = 0x0000_4000;
/// `NFSMNT_NOAC`: disable attribute cache.
pub const NFSMNT_NOAC: i32 = 0x0000_8000;
/// `NFSMNT_RDIRPLUS`: use Readdirplus for V3.
pub const NFSMNT_RDIRPLUS: i32 = 0x0001_0000;
/// `NFSMNT_READDIRSIZE`: set readdir size.
pub const NFSMNT_READDIRSIZE: i32 = 0x0002_0000;
/// `NFSMNT_ACREGMIN`: `acregmin` field valid.
pub const NFSMNT_ACREGMIN: i32 = 0x0004_0000;
/// `NFSMNT_ACREGMAX`: `acregmax` field valid.
pub const NFSMNT_ACREGMAX: i32 = 0x0008_0000;
/// `NFSMNT_ACDIRMIN`: `acdirmin` field valid.
pub const NFSMNT_ACDIRMIN: i32 = 0x0010_0000;
/// `NFSMNT_ACDIRMAX`: `acdirmax` field valid.
pub const NFSMNT_ACDIRMAX: i32 = 0x0020_0000;

// The bits the kernel sets in `nm_flag` for itself; `mount_nfs(8)` never passes them.

/// `NFSMNT_INTERNAL`: bits set internally.
pub const NFSMNT_INTERNAL: i32 = 0xfffc_0000_u32 as i32;
/// `NFSMNT_HASWRITEVERF`: has write verifier for V3.
pub const NFSMNT_HASWRITEVERF: i32 = 0x0004_0000;
/// `NFSMNT_GOTPATHCONF`: got the V3 pathconf info.
pub const NFSMNT_GOTPATHCONF: i32 = 0x0008_0000;
/// `NFSMNT_GOTFSINFO`: got the V3 fsinfo.
pub const NFSMNT_GOTFSINFO: i32 = 0x0010_0000;
/// `NFSMNT_MNTD`: mnt server for mnt point.
pub const NFSMNT_MNTD: i32 = 0x0020_0000;
/// `NFSMNT_DISMINPROG`: dismount in progress.
pub const NFSMNT_DISMINPROG: i32 = 0x0040_0000;
/// `NFSMNT_DISMNT`: dismounted.
pub const NFSMNT_DISMNT: i32 = 0x0080_0000;
/// `NFSMNT_SNDLOCK`: send socket lock.
pub const NFSMNT_SNDLOCK: i32 = 0x0100_0000;
/// `NFSMNT_WANTSND`: want above.
pub const NFSMNT_WANTSND: i32 = 0x0200_0000;
/// `NFSMNT_RCVLOCK`: rcv socket lock.
pub const NFSMNT_RCVLOCK: i32 = 0x0400_0000;
/// `NFSMNT_WANTRCV`: want above.
pub const NFSMNT_WANTRCV: i32 = 0x0800_0000;
/// `NFSMNT_WAITAUTH`: wait for authentication.
pub const NFSMNT_WAITAUTH: i32 = 0x1000_0000;
/// `NFSMNT_HASAUTH`: has authenticator.
pub const NFSMNT_HASAUTH: i32 = 0x2000_0000;
/// `NFSMNT_WANTAUTH`: wants an authenticator.
pub const NFSMNT_WANTAUTH: i32 = 0x4000_0000;
/// `NFSMNT_AUTHERR`: authentication error.
pub const NFSMNT_AUTHERR: i32 = 0x8000_0000_u32 as i32;

/// `struct msdosfs_args`: arguments to mount MSDOS filesystems. `fspec` is a user address.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MsdosfsArgs {
    /// `fspec`: blocks special holding the fs to mount.
    pub fspec: usize,
    /// `export_info`: network export information.
    pub export_info: ExportArgs,
    /// `uid`: uid that owns msdosfs files.
    pub uid: crate::sys::types::Uid,
    /// `gid`: gid that owns msdosfs files.
    pub gid: crate::sys::types::Gid,
    /// `mask`: mask to be applied for msdosfs perms.
    pub mask: crate::sys::types::Mode,
    /// `flags`: the `MSDOSFSMNT_*` below.
    pub flags: i32,
}

mount_args_from_bytes!(MsdosfsArgs, "msdosfs_args");

crate::abi_pod!(MsdosfsArgs {
    fspec: usize,
    export_info: ExportArgs,
    uid: crate::sys::types::Uid,
    gid: crate::sys::types::Gid,
    mask: crate::sys::types::Mode,
    flags: i32,
});

/// `MSDOSFSMNT_SHORTNAME`: force old DOS short names only.
pub const MSDOSFSMNT_SHORTNAME: i32 = 0x01;
/// `MSDOSFSMNT_LONGNAME`: force Win'95 long names.
pub const MSDOSFSMNT_LONGNAME: i32 = 0x02;
/// `MSDOSFSMNT_NOWIN95`: completely ignore Win95 entries.
pub const MSDOSFSMNT_NOWIN95: i32 = 0x04;

/// `struct ntfs_args`: arguments to mount ntfs filesystems. `fspec` is a user address. The
/// hole after `mode` is named. Defined on every machine (a type costs nothing): `bsd`'s NTFS
/// is built under `option_ntfs`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NtfsArgs {
    /// `fspec`: block special device to mount.
    pub fspec: usize,
    /// `export_info`: network export information.
    pub export_info: ExportArgs,
    /// `uid`: uid that owns ntfs files.
    pub uid: crate::sys::types::Uid,
    /// `gid`: gid that owns ntfs files.
    pub gid: crate::sys::types::Gid,
    /// `mode`: mask to be applied for ntfs perms.
    pub mode: crate::sys::types::Mode,
    /// The hole after `mode`.
    pub _pad0: u32,
    /// `flag`: additional flags, the `NTFS_MFLAG_*` below.
    pub flag: u64,
}

mount_args_from_bytes!(NtfsArgs, "ntfs_args");

crate::abi_pod!(NtfsArgs {
    fspec: usize,
    export_info: ExportArgs,
    uid: crate::sys::types::Uid,
    gid: crate::sys::types::Gid,
    mode: crate::sys::types::Mode,
    _pad0: u32,
    flag: u64,
});

/// `NTFS_MFLAG_CASEINS`: ntfs mount option, case-insensitive lookups.
pub const NTFS_MFLAG_CASEINS: u64 = 0x0000_0001;
/// `NTFS_MFLAG_ALLNAMES`: ntfs mount option, list the DOS names too.
pub const NTFS_MFLAG_ALLNAMES: u64 = 0x0000_0002;

/// `struct udf_args`: arguments to mount UDF file systems. `fspec` is a user address. The
/// hole after `lastblock` is named.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UdfArgs {
    /// `fspec`: block special device to mount.
    pub fspec: usize,
    /// `lastblock`: special device last block.
    pub lastblock: u32,
    /// The hole after `lastblock`.
    pub _pad0: u32,
}

mount_args_from_bytes!(UdfArgs, "udf_args");

crate::abi_pod!(UdfArgs {
    fspec: usize,
    lastblock: u32,
    _pad0: u32,
});

/// `TMPFS_ARGS_VERSION`.
pub const TMPFS_ARGS_VERSION: i32 = 1;

/// `struct tmpfs_args`: arguments to mount tmpfs file systems. The holes after `ta_version`
/// and `ta_root_mode` are named.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TmpfsArgs {
    /// `ta_version`: `TMPFS_ARGS_VERSION`.
    pub ta_version: i32,
    /// The hole after `ta_version`.
    pub _pad0: u32,
    /// `ta_nodes_max`: size counter, the most nodes.
    pub ta_nodes_max: crate::sys::types::Ino,
    /// `ta_size_max`: size counter, the most bytes.
    pub ta_size_max: crate::sys::types::Off,
    /// `ta_root_uid`: root node attribute.
    pub ta_root_uid: crate::sys::types::Uid,
    /// `ta_root_gid`: root node attribute.
    pub ta_root_gid: crate::sys::types::Gid,
    /// `ta_root_mode`: root node attribute.
    pub ta_root_mode: crate::sys::types::Mode,
    /// The hole after `ta_root_mode`.
    pub _pad1: u32,
}

mount_args_from_bytes!(TmpfsArgs, "tmpfs_args");

crate::abi_pod!(TmpfsArgs {
    ta_version: i32,
    _pad0: u32,
    ta_nodes_max: crate::sys::types::Ino,
    ta_size_max: crate::sys::types::Off,
    ta_root_uid: crate::sys::types::Uid,
    ta_root_gid: crate::sys::types::Gid,
    ta_root_mode: crate::sys::types::Mode,
    _pad1: u32,
});

/// `struct fusefs_args`: arguments to mount fusefs filesystems. `name` is a user address.
/// The hole after `allow_other` is named. Defined whatever `bsd`'s `fuse` feature says.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FusefsArgs {
    /// `name`.
    pub name: usize,
    /// `fd`: the daemon's open fuse(4) device.
    pub fd: i32,
    /// `max_read`.
    pub max_read: i32,
    /// `allow_other`: FUSE does not allow the file system to be accessed by other users
    /// unless this option is specified. This is to prevent unintentional denial of service
    /// to other users if the file system is not responding. e.g. user executes df(1) or
    /// cron job that scans mounted file systems.
    pub allow_other: i32,
    /// The hole after `allow_other`.
    pub _pad0: u32,
}

mount_args_from_bytes!(FusefsArgs, "fusefs_args");

crate::abi_pod!(FusefsArgs {
    name: usize,
    fd: i32,
    max_read: i32,
    allow_other: i32,
    _pad0: u32,
});

// The amd64/arm64 layout of the `*_args` (both LP64), as `sys/sys/mount.rs` pinned it.
const _: () = {
    use core::mem::offset_of;
    assert!(size_of::<Xucred>() == 76);
    assert!(offset_of!(Xucred, cr_groups) == 12);
    assert!(size_of::<ExportArgs>() == 120);
    assert!(offset_of!(ExportArgs, ex_addr) == 88);
    assert!(offset_of!(ExportArgs, ex_mask) == 104);
    assert!(UfsArgs::SIZE == 128);
    assert!(MfsArgs::SIZE == 144);
    assert!(offset_of!(MfsArgs, base) == 128);
    assert!(offset_of!(MfsArgs, size) == 136);
    assert!(MfsArgs::SIZE <= size_of::<MountInfo>());
    assert!(IsoArgs::SIZE == 136);
    assert!(NfsArgs::SIZE == 112);
    assert!(offset_of!(NfsArgs, fh) == 32);
    assert!(offset_of!(NfsArgs, hostname) == 88);
    assert!(NfsArgs::SIZE <= size_of::<MountInfo>());
    assert!(MsdosfsArgs::SIZE == 144);
    assert!(NtfsArgs::SIZE == 152);
    assert!(offset_of!(NtfsArgs, uid) == 128);
    assert!(offset_of!(NtfsArgs, flag) == 144);
    assert!(NtfsArgs::SIZE <= size_of::<MountInfo>());
    assert!(UdfArgs::SIZE == 16);
    assert!(TmpfsArgs::SIZE == 40);
    assert!(offset_of!(TmpfsArgs, ta_root_uid) == 24);
    assert!(FusefsArgs::SIZE == 24);
    assert!(offset_of!(FusefsArgs, fd) == 8);
    assert!(offset_of!(FusefsArgs, allow_other) == 16);
};

const _: () = assert!(size_of::<Bcachestats>() == 13 * 8);

// The amd64/arm64 layout of `struct statfs` and its neighbours (both LP64).
const _: () = {
    use core::mem::offset_of;
    assert!(offset_of!(Statfs, f_blocks) == 16);
    assert!(offset_of!(Statfs, f_fsid) == 96);
    assert!(offset_of!(Statfs, f_ctime) == 112);
    assert!(offset_of!(Statfs, f_fstypename) == 120);
    assert!(offset_of!(Statfs, f_mntfromspec) == 316);
    assert!(offset_of!(Statfs, mount_info) == 408);
    assert!(size_of::<Statfs>() == 568);
    assert!(size_of::<Fhandle>() == 28);
    assert!(size_of::<VfsconfAbi>() == 48);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::bytes_of;

    /// The bytes of a `T` whose every byte is its own offset (mod 256), then the `T` read
    /// out of them.
    fn offsets<T: crate::user::AbiPod>(len: usize) -> Option<T> {
        let b: [u8; 256] = core::array::from_fn(|i| i as u8);
        let v: [u8; 512] = core::array::from_fn(|i| b[i % 256]);
        crate::user::pod_from_bytes(&v[..len])
    }

    #[test]
    fn short_arguments_are_none_like_the_cs_null_data() {
        assert!(UfsArgs::from_bytes(&[0; UfsArgs::SIZE - 1]).is_none());
        assert!(MfsArgs::from_bytes(&[]).is_none());
        assert!(NfsArgs::from_bytes(&[0; 111]).is_none());
        assert!(TmpfsArgs::from_bytes(&[0; 39]).is_none());
        assert!(FusefsArgs::from_bytes(&[0; 23]).is_none());
        assert!(UdfArgs::from_bytes(&[0; 16]).is_some());
    }

    #[test]
    fn the_fields_are_read_at_the_cs_offsets() {
        let m: MfsArgs = offsets(MfsArgs::SIZE).unwrap();
        assert_eq!(
            m.base.to_ne_bytes(),
            core::array::from_fn(|i| (128 + i) as u8)
        );
        assert_eq!(
            m.size.to_ne_bytes(),
            core::array::from_fn(|i| (136 + i) as u8)
        );
        assert_eq!(
            m.export_info.ex_addr.to_ne_bytes(),
            core::array::from_fn(|i| (8 + 88 + i) as u8)
        );
        let n: NfsArgs = offsets(NfsArgs::SIZE).unwrap();
        assert_eq!(
            n.hostname.to_ne_bytes(),
            core::array::from_fn(|i| (88 + i) as u8)
        );
        assert_eq!(
            n.acdirmax.to_ne_bytes(),
            core::array::from_fn(|i| (108 + i) as u8)
        );
        let t: TmpfsArgs = offsets(TmpfsArgs::SIZE).unwrap();
        assert_eq!(
            t.ta_root_mode.to_ne_bytes(),
            core::array::from_fn(|i| (32 + i) as u8)
        );
        let x: NtfsArgs = offsets(NtfsArgs::SIZE).unwrap();
        assert_eq!(
            x.flag.to_ne_bytes(),
            core::array::from_fn(|i| (144 + i) as u8)
        );
        assert_eq!(
            x.export_info.ex_anon.cr_groups[0].to_ne_bytes(),
            core::array::from_fn(|i| (8 + 8 + 12 + i) as u8)
        );
    }

    #[test]
    fn a_structure_reads_back_from_its_own_bytes() {
        let mut u = UfsArgs::from_bytes(&[0; UfsArgs::SIZE]).unwrap();
        u.fspec = 0x1234;
        u.export_info.ex_flags = 7;
        u.export_info.ex_anon.cr_ngroups = 3;
        u.export_info.ex_masklen = 16;
        let back = UfsArgs::from_bytes(bytes_of(core::slice::from_ref(&u))).unwrap();
        assert_eq!(back.fspec, 0x1234);
        assert_eq!(back.export_info.ex_flags, 7);
        assert_eq!(back.export_info.ex_anon.cr_ngroups, 3);
        assert_eq!(back.export_info.ex_masklen, 16);
        // A longer copy (the kernel's vfc_datasize bytes) reads its first SIZE.
        let mut long = [0u8; UfsArgs::SIZE + 8];
        long[..UfsArgs::SIZE].copy_from_slice(bytes_of(core::slice::from_ref(&u)));
        assert_eq!(UfsArgs::from_bytes(&long).unwrap().fspec, 0x1234);
    }
}
/* </TESTS> */
