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
//! LZ: sys/sys/mount.rs@3c62ede68802
//! LZ: sys/kern/vfs_subr.rs@3c62ede68802
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
