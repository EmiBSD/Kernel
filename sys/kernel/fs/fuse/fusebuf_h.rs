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
 * Copyright (c) 2013 Sylvestre Gallon
 * Copyright (c) 2013 Martin Pieuchot
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
//! `<sys/fusebuf.h>`: the FUSE kernel interface. The `fuse_*` structures are the binary
//! protocol between fuse(4) and the userland file system daemon (libfuse), the Linux FUSE
//! kernel interface 7.19 that OpenBSD somewhat emulates; `struct fusebuf` is the kernel's
//! request and reply buffer.
//!
//! Upstream: sys/sys/fusebuf.h @ 3ce1f3f79392
//! LZ: sys/sys/fusebuf.rs@3c62ede68802
//!
//! An operation is issued by the kernel through fuse(4) when the userland file system needs
//! to execute an action (mkdir(2), link(2), etc). The daemon reads it from `/dev/fuse0` as a
//! `struct fuse_in_header`, the operation's input structure (`op_in_len` bytes) and the data
//! (`fb_len` bytes), and answers with a `struct fuse_out_header` carrying the same ID
//! (`unique`), the operation's output structure (`op_out_len` bytes) and, for the operations
//! that expect it (`op_out_buf`), data.
//!
//! ## Deviations
//! - `FUSE_NAME_OFFSET`, `FUSE_DIRENT_ALIGN` and `FUSE_DIRENT_SIZE` are a constant and
//!   `const fn`s (LZ's).
//! - The `!_KERNEL` half of the header (libfuse's `struct fusebuf`, its `fb_*` macros and
//!   `fb_dat(in)`) is userland's view of the bytes `fuseread` produces and is not ported
//!   (LZ's); the device's host tests check that `fuseread` produces exactly that layout.
//! - The prototypes `fb_setup`, `fb_queue` and `fb_delete` are the functions of
//!   `fs::fuse::fusebuf`.
//!
//! ## Redesign
//! - LZ's `fuse_abi!` wrote an `unsafe impl AbiPod` per protocol structure and
//!   `abi_bytes`/`abi_bytes_mut` made byte slices of them with two `unsafe` blocks: the
//!   structures are `hal::abi_pod!`s now (the tiling checked at compile time; of LZ's 37 size
//!   and offset checks 36 kept beside it, `align_of::<FusebufOp>() == 8` replaced by the
//!   union's size being whole words, and every member the daemon's bytes depend on pinned by
//!   a test), their bytes `hal::user::bytes_of`/`bytes_of_mut`.
//! - `struct fuse_dirent` is its 24-byte head: the flexible `name[]` (LZ's `name: [u8; 0]`,
//!   which an `abi_pod!` tiling cannot see) is no member; the name is the bytes past
//!   [`FUSE_NAME_OFFSET`], as the C reads them. The size is the C's `sizeof` (24) either way.
//!   `FuseDirent::from_bytes`'s `read_unaligned` is `hal::user::pod_from_bytes` of the slice.
//! - `struct fusebuf` ([`Fusebuf`]) was LZ's struct of `Cell`s shared under an `unsafe impl
//!   Sync`, its data a raw `*mut u8` lent by an `unsafe fn fb_dat_slice`. Here it is an item
//!   of the `"fmsg"` pool held by counted handles: the requester's and, while it is queued,
//!   one the device's queue parks on it (`PoolArc::forget`/`adopt`, the recorded pool's
//!   [`PoolHome`]). What `fb_setup` writes before the request is published (the header, the
//!   operation's lengths) is plain; what the two owners change in turn is words (the error,
//!   the data's length) and the `op` union is sixteen relaxed words ([`FusebufOp`], the
//!   inode's word idiom, IDIOMS row 101), read and written as whole members at offset 0; the
//!   data is an `MArray<u8>` of `M_FUSEFS` in a `ReadMostly` cell, moved out by the side that
//!   copies it (no reference over a copy that can sleep). The C's two owners in turn hold the
//!   kernel lock; no lock and no interrupt level is added (D-N5.9-8 and D-f1-2 of N5.9f1).

use core::mem::offset_of;
use core::slice;
use core::sync::atomic::AtomicU64;

use hal::malloc::MArray;
use hal::pinlist::PinEntry;
use hal::pool::PoolHome;
use hal::slot::ReadMostly;
use hal::sys::types::Ino;
use hal::user::{AbiPod, bytes_of, bytes_of_mut};

use crate::fs::ufs::ufs::inode::{words_read_bytes, words_write_bytes};
use crate::fs::word::{I32Word, U64Word};

/// `FUSEBUFMAXSIZE`: maximum size of the read or write buffer sent from the kernel for VFS
/// syscalls: read, readdir, readlink, write.
pub const FUSEBUFMAXSIZE: usize = 4096 * 1024;

/// `FUSE_KERNEL_VERSION`: Linux FUSE kernel interface major version we somewhat emulate.
pub const FUSE_KERNEL_VERSION: u32 = 7;

/// `FUSE_KERNEL_MINOR_VERSION`: Linux FUSE kernel interface minor version we somewhat
/// emulate.
pub const FUSE_KERNEL_MINOR_VERSION: u32 = 19;

/// `FUSE_ROOT_ID`: the root inode is the root of the mounted FUSE file system. Also note
/// that inode 0 can't be used for normal purposes.
pub const FUSE_ROOT_ID: Ino = 1;

/// `FUSE_FATTR_MODE`: flag needed by setattr.
pub const FUSE_FATTR_MODE: u32 = 1 << 0;
/// `FUSE_FATTR_UID`.
pub const FUSE_FATTR_UID: u32 = 1 << 1;
/// `FUSE_FATTR_GID`.
pub const FUSE_FATTR_GID: u32 = 1 << 2;
/// `FUSE_FATTR_SIZE`.
pub const FUSE_FATTR_SIZE: u32 = 1 << 3;
/// `FUSE_FATTR_ATIME`.
pub const FUSE_FATTR_ATIME: u32 = 1 << 4;
/// `FUSE_FATTR_MTIME`.
pub const FUSE_FATTR_MTIME: u32 = 1 << 5;

/// `FUSE_LOOKUP`: fusebuf type.
pub const FUSE_LOOKUP: u32 = 1;
/// `FUSE_GETATTR`.
pub const FUSE_GETATTR: u32 = 3;
/// `FUSE_SETATTR`.
pub const FUSE_SETATTR: u32 = 4;
/// `FUSE_READLINK`.
pub const FUSE_READLINK: u32 = 5;
/// `FUSE_SYMLINK`.
pub const FUSE_SYMLINK: u32 = 6;
/// `FUSE_MKNOD`.
pub const FUSE_MKNOD: u32 = 8;
/// `FUSE_MKDIR`.
pub const FUSE_MKDIR: u32 = 9;
/// `FUSE_UNLINK`.
pub const FUSE_UNLINK: u32 = 10;
/// `FUSE_RMDIR`.
pub const FUSE_RMDIR: u32 = 11;
/// `FUSE_RENAME`.
pub const FUSE_RENAME: u32 = 12;
/// `FUSE_LINK`.
pub const FUSE_LINK: u32 = 13;
/// `FUSE_OPEN`.
pub const FUSE_OPEN: u32 = 14;
/// `FUSE_READ`.
pub const FUSE_READ: u32 = 15;
/// `FUSE_WRITE`.
pub const FUSE_WRITE: u32 = 16;
/// `FUSE_STATFS`.
pub const FUSE_STATFS: u32 = 17;
/// `FUSE_RELEASE`.
pub const FUSE_RELEASE: u32 = 18;
/// `FUSE_FSYNC`.
pub const FUSE_FSYNC: u32 = 20;
/// `FUSE_FLUSH`.
pub const FUSE_FLUSH: u32 = 25;
/// `FUSE_INIT`.
pub const FUSE_INIT: u32 = 26;
/// `FUSE_OPENDIR`.
pub const FUSE_OPENDIR: u32 = 27;
/// `FUSE_READDIR`.
pub const FUSE_READDIR: u32 = 28;
/// `FUSE_RELEASEDIR`.
pub const FUSE_RELEASEDIR: u32 = 29;
/// `FUSE_DESTROY`.
pub const FUSE_DESTROY: u32 = 38;
/// `FUSE_FORGET`.
pub const FUSE_FORGET: u32 = 2;

/// `struct fuse_attr`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseAttr {
    /// `ino`.
    pub ino: u64,
    /// `size`.
    pub size: u64,
    /// `blocks`.
    pub blocks: u64,
    /// `atime`.
    pub atime: u64,
    /// `mtime`.
    pub mtime: u64,
    /// `ctime`.
    pub ctime: u64,
    /// `atimensec`.
    pub atimensec: u32,
    /// `mtimensec`.
    pub mtimensec: u32,
    /// `ctimensec`.
    pub ctimensec: u32,
    /// `mode`.
    pub mode: u32,
    /// `nlink`.
    pub nlink: u32,
    /// `uid`.
    pub uid: u32,
    /// `gid`.
    pub gid: u32,
    /// `rdev`.
    pub rdev: u32,
    /// `blksize`.
    pub blksize: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseAttr {
    ino: u64,
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    atimensec: u32,
    mtimensec: u32,
    ctimensec: u32,
    mode: u32,
    nlink: u32,
    uid: u32,
    gid: u32,
    rdev: u32,
    blksize: u32,
    padding: u32,
});

/// `struct fuse_entry_out`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseEntryOut {
    /// `nodeid`: inode number.
    pub nodeid: u64,
    /// `generation`: not implemented.
    pub generation: u64,
    /// `entry_valid`: not implemented.
    pub entry_valid: u64,
    /// `attr_valid`: not implemented.
    pub attr_valid: u64,
    /// `entry_valid_nsec`: not implemented.
    pub entry_valid_nsec: u32,
    /// `attr_valid_nsec`: not implemented.
    pub attr_valid_nsec: u32,
    /// `attr`.
    pub attr: FuseAttr,
}
hal::abi_pod!(FuseEntryOut {
    nodeid: u64,
    generation: u64,
    entry_valid: u64,
    attr_valid: u64,
    entry_valid_nsec: u32,
    attr_valid_nsec: u32,
    attr: FuseAttr,
});

/// `struct fuse_forget_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseForgetIn {
    /// `nlookup`.
    pub nlookup: u64,
}
hal::abi_pod!(FuseForgetIn { nlookup: u64 });

/// `struct fuse_getattr_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseGetattrIn {
    /// `getattr_flags`: not implemented.
    pub getattr_flags: u32,
    /// `dummy`.
    pub dummy: u32,
    /// `fh`: not implemented.
    pub fh: u64,
}
hal::abi_pod!(FuseGetattrIn {
    getattr_flags: u32,
    dummy: u32,
    fh: u64,
});

/// `struct fuse_attr_out`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseAttrOut {
    /// `attr_valid`: not implemented.
    pub attr_valid: u64,
    /// `attr_valid_nsec`: not implemented.
    pub attr_valid_nsec: u32,
    /// `dummy`.
    pub dummy: u32,
    /// `attr`.
    pub attr: FuseAttr,
}
hal::abi_pod!(FuseAttrOut {
    attr_valid: u64,
    attr_valid_nsec: u32,
    dummy: u32,
    attr: FuseAttr,
});

/// `struct fuse_mknod_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseMknodIn {
    /// `mode`.
    pub mode: u32,
    /// `rdev`.
    pub rdev: u32,
    /// `umask`.
    pub umask: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseMknodIn {
    mode: u32,
    rdev: u32,
    umask: u32,
    padding: u32,
});

/// `struct fuse_mkdir_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseMkdirIn {
    /// `mode`.
    pub mode: u32,
    /// `umask`.
    pub umask: u32,
}
hal::abi_pod!(FuseMkdirIn {
    mode: u32,
    umask: u32
});

/// `struct fuse_rename_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseRenameIn {
    /// `newdir`.
    pub newdir: u64,
}
hal::abi_pod!(FuseRenameIn { newdir: u64 });

/// `struct fuse_link_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseLinkIn {
    /// `oldnodeid`.
    pub oldnodeid: u64,
}
hal::abi_pod!(FuseLinkIn { oldnodeid: u64 });

/// `struct fuse_setattr_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseSetattrIn {
    /// `valid`: the `FUSE_FATTR_*` set.
    pub valid: u32,
    /// `padding`.
    pub padding: u32,
    /// `fh`.
    pub fh: u64,
    /// `size`.
    pub size: u64,
    /// `lock_owner`.
    pub lock_owner: u64,
    /// `atime`.
    pub atime: u64,
    /// `mtime`.
    pub mtime: u64,
    /// `unused2`.
    pub unused2: u64,
    /// `atimensec`.
    pub atimensec: u32,
    /// `mtimensec`.
    pub mtimensec: u32,
    /// `unused3`.
    pub unused3: u32,
    /// `mode`.
    pub mode: u32,
    /// `unused4`.
    pub unused4: u32,
    /// `uid`.
    pub uid: u32,
    /// `gid`.
    pub gid: u32,
    /// `unused5`.
    pub unused5: u32,
}
hal::abi_pod!(FuseSetattrIn {
    valid: u32,
    padding: u32,
    fh: u64,
    size: u64,
    lock_owner: u64,
    atime: u64,
    mtime: u64,
    unused2: u64,
    atimensec: u32,
    mtimensec: u32,
    unused3: u32,
    mode: u32,
    unused4: u32,
    uid: u32,
    gid: u32,
    unused5: u32,
});

/// `struct fuse_open_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseOpenIn {
    /// `flags`.
    pub flags: u32,
    /// `unused`.
    pub unused: u32,
}
hal::abi_pod!(FuseOpenIn {
    flags: u32,
    unused: u32
});

/// `struct fuse_open_out`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseOpenOut {
    /// `fh`.
    pub fh: u64,
    /// `open_flags`.
    pub open_flags: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseOpenOut {
    fh: u64,
    open_flags: u32,
    padding: u32,
});

/// `struct fuse_release_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseReleaseIn {
    /// `fh`.
    pub fh: u64,
    /// `flags`.
    pub flags: u32,
    /// `release_flags`.
    pub release_flags: u32,
    /// `lock_owner`.
    pub lock_owner: u64,
}
hal::abi_pod!(FuseReleaseIn {
    fh: u64,
    flags: u32,
    release_flags: u32,
    lock_owner: u64,
});

/// `struct fuse_flush_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseFlushIn {
    /// `fh`.
    pub fh: u64,
    /// `unused`.
    pub unused: u32,
    /// `padding`.
    pub padding: u32,
    /// `lock_owner`.
    pub lock_owner: u64,
}
hal::abi_pod!(FuseFlushIn {
    fh: u64,
    unused: u32,
    padding: u32,
    lock_owner: u64,
});

/// `struct fuse_read_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseReadIn {
    /// `fh`.
    pub fh: u64,
    /// `offset`.
    pub offset: u64,
    /// `size`.
    pub size: u32,
    /// `read_flags`.
    pub read_flags: u32,
    /// `lock_owner`.
    pub lock_owner: u64,
    /// `flags`.
    pub flags: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseReadIn {
    fh: u64,
    offset: u64,
    size: u32,
    read_flags: u32,
    lock_owner: u64,
    flags: u32,
    padding: u32,
});

/// `struct fuse_write_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseWriteIn {
    /// `fh`.
    pub fh: u64,
    /// `offset`.
    pub offset: u64,
    /// `size`.
    pub size: u32,
    /// `write_flags`.
    pub write_flags: u32,
    /// `lock_owner`.
    pub lock_owner: u64,
    /// `flags`.
    pub flags: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseWriteIn {
    fh: u64,
    offset: u64,
    size: u32,
    write_flags: u32,
    lock_owner: u64,
    flags: u32,
    padding: u32,
});

/// `struct fuse_write_out`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseWriteOut {
    /// `size`.
    pub size: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseWriteOut {
    size: u32,
    padding: u32
});

/// `struct fuse_kstatfs`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseKstatfs {
    /// `blocks`.
    pub blocks: u64,
    /// `bfree`.
    pub bfree: u64,
    /// `bavail`.
    pub bavail: u64,
    /// `files`.
    pub files: u64,
    /// `ffree`.
    pub ffree: u64,
    /// `bsize`.
    pub bsize: u32,
    /// `namelen`.
    pub namelen: u32,
    /// `frsize`.
    pub frsize: u32,
    /// `padding`.
    pub padding: u32,
    /// `spare`.
    pub spare: [u32; 6],
}
hal::abi_pod!(FuseKstatfs {
    blocks: u64,
    bfree: u64,
    bavail: u64,
    files: u64,
    ffree: u64,
    bsize: u32,
    namelen: u32,
    frsize: u32,
    padding: u32,
    spare: [u32; 6],
});

/// `struct fuse_statfs_out`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseStatfsOut {
    /// `st`.
    pub st: FuseKstatfs,
}
hal::abi_pod!(FuseStatfsOut { st: FuseKstatfs });

/// `struct fuse_fsync_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseFsyncIn {
    /// `fh`.
    pub fh: u64,
    /// `fsync_flags` (the C marks it XXX).
    pub fsync_flags: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseFsyncIn {
    fh: u64,
    fsync_flags: u32,
    padding: u32,
});

/// `struct fuse_init_in`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseInitIn {
    /// `major`.
    pub major: u32,
    /// `minor`.
    pub minor: u32,
    /// `max_readahead`.
    pub max_readahead: u32,
    /// `flags`.
    pub flags: u32,
}
hal::abi_pod!(FuseInitIn {
    major: u32,
    minor: u32,
    max_readahead: u32,
    flags: u32,
});

/// `struct fuse_init_out`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseInitOut {
    /// `major`.
    pub major: u32,
    /// `minor`.
    pub minor: u32,
    /// `max_readahead`: not implemented.
    pub max_readahead: u32,
    /// `flags`: not implemented.
    pub flags: u32,
    /// `max_background`: not implemented.
    pub max_background: u16,
    /// `congestion_threshold`: not implemented.
    pub congestion_threshold: u16,
    /// `max_write`.
    pub max_write: u32,
}
hal::abi_pod!(FuseInitOut {
    major: u32,
    minor: u32,
    max_readahead: u32,
    flags: u32,
    max_background: u16,
    congestion_threshold: u16,
    max_write: u32,
});

/// `struct fuse_in_header`: the header of every request the daemon reads.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseInHeader {
    /// `len`: the request's length, header included.
    pub len: u32,
    /// `opcode`: the `FUSE_*` operation.
    pub opcode: u32,
    /// `unique`: the request's ID, which the reply carries back.
    pub unique: u64,
    /// `nodeid`: the inode the operation is about.
    pub nodeid: u64,
    /// `uid`.
    pub uid: u32,
    /// `gid`.
    pub gid: u32,
    /// `pid`: the thread ID of the requester.
    pub pid: u32,
    /// `padding`.
    pub padding: u32,
}
hal::abi_pod!(FuseInHeader {
    len: u32,
    opcode: u32,
    unique: u64,
    nodeid: u64,
    uid: u32,
    gid: u32,
    pid: u32,
    padding: u32,
});

/// `struct fuse_out_header`: the header of every reply the daemon writes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseOutHeader {
    /// `len`: the reply's length, header included.
    pub len: u32,
    /// `error`: the negated errno, 0 for success.
    pub error: i32,
    /// `unique`: the ID of the request answered; 0 for a notification.
    pub unique: u64,
}
hal::abi_pod!(FuseOutHeader {
    len: u32,
    error: i32,
    unique: u64,
});

/// `struct fuse_dirent`'s head: one directory entry of a `FUSE_READDIR` reply; the
/// unterminated name (the C's flexible `name[]`) follows at [`FUSE_NAME_OFFSET`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuseDirent {
    /// `ino`.
    pub ino: u64,
    /// `off`.
    pub off: u64,
    /// `namelen`.
    pub namelen: u32,
    /// `type`.
    pub r#type: u32,
}
hal::abi_pod!(FuseDirent {
    ino: u64,
    off: u64,
    namelen: u32,
    r#type: u32,
});

/// `FUSE_NAME_OFFSET`: `offsetof(struct fuse_dirent, name)`, the head's size.
pub const FUSE_NAME_OFFSET: usize = size_of::<FuseDirent>();

/// `FUSE_DIRENT_ALIGN(x)`: `x` rounded up to a multiple of 8.
pub const fn fuse_dirent_align(x: usize) -> usize {
    (x + size_of::<u64>() - 1) & !(size_of::<u64>() - 1)
}

/// `FUSE_DIRENT_SIZE(d)`: the record length of a FUSE dirent.
pub const fn fuse_dirent_size(d: &FuseDirent) -> usize {
    fuse_dirent_align(FUSE_NAME_OFFSET + d.namelen as usize)
}

/// The size of `struct fusebuf`'s `op` union: its largest member (`struct fuse_entry_out`).
pub const FUSEBUF_OP_SIZE: usize = {
    let mut n = 0;
    let sizes = [
        size_of::<FuseForgetIn>(),
        size_of::<FuseGetattrIn>(),
        size_of::<FuseSetattrIn>(),
        size_of::<FuseMknodIn>(),
        size_of::<FuseMkdirIn>(),
        size_of::<FuseRenameIn>(),
        size_of::<FuseLinkIn>(),
        size_of::<FuseOpenIn>(),
        size_of::<FuseReadIn>(),
        size_of::<FuseWriteIn>(),
        size_of::<FuseReleaseIn>(),
        size_of::<FuseFsyncIn>(),
        size_of::<FuseFlushIn>(),
        size_of::<FuseInitIn>(),
        size_of::<FuseEntryOut>(),
        size_of::<FuseAttrOut>(),
        size_of::<FuseOpenOut>(),
        size_of::<FuseWriteOut>(),
        size_of::<FuseStatfsOut>(),
        size_of::<FuseInitOut>(),
    ];
    let mut i = 0;
    while i < sizes.len() {
        if sizes[i] > n {
            n = sizes[i];
        }
        i += 1;
    }
    n
};

/// `struct fusebuf`'s `op`: the union of the operations' input structures (`in`) and output
/// structures (`out`), which overlap, as the words of the largest member, shared by the
/// request's two owners in turn (each holds the kernel lock) with no lock of its own. A
/// member is read and written whole at offset 0, as the C's `fbuf->op.in.read` and
/// `fbuf->op.out.entry`; the bytes past a member written keep their value, as the C's
/// stores into one member of the union do.
pub struct FusebufOp([AtomicU64; FUSEBUF_OP_SIZE / 8]);

impl FusebufOp {
    /// A zeroed union, as `pool_get(PR_ZERO)` leaves it.
    pub const fn new() -> Self {
        Self([const { AtomicU64::new(0) }; FUSEBUF_OP_SIZE / 8])
    }

    /// Reads the member `T` (`fbuf->op.in.read`, `fbuf->op.out.entry`, ...).
    pub fn get<T: FusebufOpMember>(&self) -> T {
        const { assert!(size_of::<T>() <= FUSEBUF_OP_SIZE) };
        let mut v = T::default();
        self.read_bytes(bytes_of_mut(slice::from_mut(&mut v)));
        v
    }

    /// Writes the member `T`; the bytes past it keep their value.
    pub fn set<T: FusebufOpMember>(&self, v: &T) {
        const { assert!(size_of::<T>() <= FUSEBUF_OP_SIZE) };
        self.write_bytes(bytes_of(slice::from_ref(v)));
    }

    /// The first `out.len()` bytes of the union into `out` (`fuseread`'s
    /// `uiomove(&fbuf->op, op_in_len, uio)`).
    pub fn read_bytes(&self, out: &mut [u8]) {
        words_read_bytes(&self.0, "fusebuf op", 0, out);
    }

    /// The first `src.len()` bytes of the union set to `src` (`fusewrite`'s
    /// `uiomove(&fbuf->op, op_out_len, uio)`).
    pub fn write_bytes(&self, src: &[u8]) {
        words_write_bytes(&self.0, "fusebuf op", 0, src);
    }
}

impl Default for FusebufOp {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct fusebuf` (`_KERNEL`): a request issued by the kernel through fuse(4) and, once the
/// daemon answered it with the same ID (`fb_uuid`), its reply; an item of the `"fmsg"` pool
/// (`fusefs_fbuf_pool`) held by counted handles (the module's `## Redesign`).
///
/// The requesting thread fills it and sleeps on it (`fb_queue`); the device moves it from
/// its input queue to its wait queue when the daemon reads it and off the wait queue when
/// the reply is written, then wakes the thread.
pub struct Fusebuf {
    /// `next` (`fb_next`): the link on a device's input queue (under its `fd_lock`) or wait
    /// queue (under the kernel lock).
    pub fb_link: PinEntry<Fusebuf>,
    /// Where the item is, so a queue's parked reference can be taken back from the `&Fusebuf`
    /// the queue lends.
    home: PoolHome<Fusebuf>,
    /// `hdr`: the request's header, written by `fb_setup` before the request is published.
    pub hdr: FuseInHeader,
    /// `error` (`fb_err`): error returned by daemon.
    pub error: I32Word,
    /// `op_in_len`: size of input.
    pub op_in_len: usize,
    /// `op_out_len`: size of output.
    pub op_out_len: usize,
    /// `op_out_buf`: whether to expect data.
    pub op_out_buf: bool,
    /// `op`: the operation's input and output structures.
    pub op: FusebufOp,
    /// `dat_len` (`fb_len`): the length of the data.
    pub dat_len: U64Word,
    /// `dat` (`fb_dat`): the data, `malloc(M_FUSEFS)`ed; `None` for the C's NULL.
    dat: ReadMostly<Option<MArray<u8>>>,
}

hal::pin_adapter!(
    /// `fb_next`: the link of a fusebuf on a device's queues (`struct fusebuf_head`).
    pub FbLink: Fusebuf, fb_link
);

hal::pool_home!(Fusebuf, home);

impl Fusebuf {
    /// A fusebuf of the header `hdr` and the operation's lengths, no error, `fb_len` bytes
    /// of data in `dat` (`None` when there are none), the union zeroed: `fb_setup`'s value.
    pub fn new(
        hdr: FuseInHeader,
        op_in_len: usize,
        op_out_len: usize,
        op_out_buf: bool,
        dat: Option<MArray<u8>>,
    ) -> Self {
        let len = dat.as_ref().map_or(0, |d| d.len());
        Self {
            fb_link: PinEntry::new(),
            home: PoolHome::new(),
            hdr,
            error: I32Word::new(0),
            op_in_len,
            op_out_len,
            op_out_buf,
            op: FusebufOp::new(),
            dat_len: U64Word::new(len as u64),
            dat: ReadMostly::new(dat),
        }
    }

    /// `fb_type` (`hdr.opcode`).
    pub fn fb_type(&self) -> u32 {
        self.hdr.opcode
    }

    /// `fb_uuid` (`hdr.unique`).
    pub fn fb_uuid(&self) -> u64 {
        self.hdr.unique
    }

    /// `fb_ino` (`hdr.nodeid`).
    pub fn fb_ino(&self) -> u64 {
        self.hdr.nodeid
    }

    /// `fb_tid` (`hdr.pid`).
    pub fn fb_tid(&self) -> u32 {
        self.hdr.pid
    }

    /// `fb_uid` (`hdr.uid`).
    pub fn fb_uid(&self) -> u32 {
        self.hdr.uid
    }

    /// `fb_gid` (`hdr.gid`).
    pub fn fb_gid(&self) -> u32 {
        self.hdr.gid
    }

    /// `fb_err` (`error`).
    pub fn fb_err(&self) -> i32 {
        self.error.get()
    }

    /// `fb_err = e`.
    pub fn set_fb_err(&self, e: i32) {
        self.error.set(e);
    }

    /// `fb_len` (`dat_len`).
    pub fn fb_len(&self) -> u64 {
        self.dat_len.get()
    }

    /// `fb_len = len`.
    pub fn set_fb_len(&self, len: u64) {
        self.dat_len.set(len);
    }

    /// `f` on the data's bytes (empty when there are none): a short read that must not
    /// sleep (the cell's rule).
    pub fn dat_with<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        f(self.dat.read().as_deref().unwrap_or(&[]))
    }

    /// `f` on the data's bytes, writable (empty when there are none): the requester's fill
    /// before `fb_queue`, which must not sleep.
    pub fn dat_update<R>(&self, f: impl FnOnce(&mut [u8]) -> R) -> R {
        self.dat.update(|d| f(d.as_deref_mut().unwrap_or_default()))
    }

    /// The data moved out (`fb_dat = NULL` with the block in the caller's hands): what a
    /// copy that can sleep works on.
    pub fn take_dat(&self) -> Option<MArray<u8>> {
        self.dat.replace(None)
    }

    /// `fb_dat = dat`; the block it held, if any, is freed.
    pub fn set_dat(&self, dat: Option<MArray<u8>>) {
        drop(self.dat.replace(dat));
    }

    /// Reads the `op` union's member `T` (`fbuf->op.out.open`, ...).
    pub fn op_get<T: FusebufOpMember>(&self) -> T {
        self.op.get()
    }

    /// Writes the `op` union's member `T` (`fbuf->op.in.open = ...`).
    pub fn op_set<T: FusebufOpMember>(&self, v: &T) {
        self.op.set(v);
    }
}

/// A member of `struct fusebuf`'s `op` union (`op.in.*` or `op.out.*`).
pub trait FusebufOpMember: AbiPod + Default {}

macro_rules! fusebuf_op_members {
    ($($t:ident),+ $(,)?) => {
        $( impl FusebufOpMember for $t {} )+
    };
}

fusebuf_op_members!(
    FuseForgetIn,
    FuseGetattrIn,
    FuseSetattrIn,
    FuseMknodIn,
    FuseMkdirIn,
    FuseRenameIn,
    FuseLinkIn,
    FuseOpenIn,
    FuseReadIn,
    FuseWriteIn,
    FuseReleaseIn,
    FuseFsyncIn,
    FuseFlushIn,
    FuseInitIn,
    FuseEntryOut,
    FuseAttrOut,
    FuseOpenOut,
    FuseWriteOut,
    FuseStatfsOut,
    FuseInitOut,
);

const _: () = {
    assert!(size_of::<FuseAttr>() == 88);
    assert!(size_of::<FuseEntryOut>() == 128);
    assert!(offset_of!(FuseEntryOut, attr) == 40);
    assert!(size_of::<FuseForgetIn>() == 8);
    assert!(size_of::<FuseGetattrIn>() == 16);
    assert!(size_of::<FuseAttrOut>() == 104);
    assert!(offset_of!(FuseAttrOut, attr) == 16);
    assert!(size_of::<FuseMknodIn>() == 16);
    assert!(size_of::<FuseMkdirIn>() == 8);
    assert!(size_of::<FuseRenameIn>() == 8);
    assert!(size_of::<FuseLinkIn>() == 8);
    assert!(size_of::<FuseSetattrIn>() == 88);
    assert!(offset_of!(FuseSetattrIn, atimensec) == 56);
    assert!(offset_of!(FuseSetattrIn, mode) == 68);
    assert!(offset_of!(FuseSetattrIn, uid) == 76);
    assert!(size_of::<FuseOpenIn>() == 8);
    assert!(size_of::<FuseOpenOut>() == 16);
    assert!(size_of::<FuseReleaseIn>() == 24);
    assert!(size_of::<FuseFlushIn>() == 24);
    assert!(size_of::<FuseReadIn>() == 40);
    assert!(size_of::<FuseWriteIn>() == 40);
    assert!(size_of::<FuseWriteOut>() == 8);
    assert!(size_of::<FuseKstatfs>() == 80);
    assert!(size_of::<FuseStatfsOut>() == 80);
    assert!(size_of::<FuseFsyncIn>() == 16);
    assert!(size_of::<FuseInitIn>() == 16);
    assert!(size_of::<FuseInitOut>() == 24);
    assert!(offset_of!(FuseInitOut, max_write) == 20);
    assert!(size_of::<FuseInHeader>() == 40);
    assert!(offset_of!(FuseInHeader, unique) == 8);
    assert!(offset_of!(FuseInHeader, nodeid) == 16);
    assert!(offset_of!(FuseInHeader, pid) == 32);
    assert!(size_of::<FuseOutHeader>() == 16);
    assert!(size_of::<FuseDirent>() == 24);
    assert!(FUSE_NAME_OFFSET == 24);
    assert!(FUSEBUF_OP_SIZE == 128);
    assert!(FUSEBUF_OP_SIZE.is_multiple_of(8));
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `<sys/fusebuf.h>`: the protocol structures' layout (the bytes libfuse
    // reads and writes: every member's offset), the `op` union's overlapping members, the
    // dirent helpers, and the constants against the C header (`just test-ref`).

    use std::{assert, assert_eq};

    use hal::user::pod_from_bytes;

    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/fusebuf.h");
        for (name, value) in [
            ("FUSEBUFMAXSIZE", FUSEBUFMAXSIZE as i64),
            ("FUSE_KERNEL_VERSION", i64::from(FUSE_KERNEL_VERSION)),
            (
                "FUSE_KERNEL_MINOR_VERSION",
                i64::from(FUSE_KERNEL_MINOR_VERSION),
            ),
            ("FUSE_FATTR_MODE", i64::from(FUSE_FATTR_MODE)),
            ("FUSE_FATTR_UID", i64::from(FUSE_FATTR_UID)),
            ("FUSE_FATTR_GID", i64::from(FUSE_FATTR_GID)),
            ("FUSE_FATTR_SIZE", i64::from(FUSE_FATTR_SIZE)),
            ("FUSE_FATTR_ATIME", i64::from(FUSE_FATTR_ATIME)),
            ("FUSE_FATTR_MTIME", i64::from(FUSE_FATTR_MTIME)),
            ("FUSE_LOOKUP", i64::from(FUSE_LOOKUP)),
            ("FUSE_GETATTR", i64::from(FUSE_GETATTR)),
            ("FUSE_SETATTR", i64::from(FUSE_SETATTR)),
            ("FUSE_READLINK", i64::from(FUSE_READLINK)),
            ("FUSE_SYMLINK", i64::from(FUSE_SYMLINK)),
            ("FUSE_MKNOD", i64::from(FUSE_MKNOD)),
            ("FUSE_MKDIR", i64::from(FUSE_MKDIR)),
            ("FUSE_UNLINK", i64::from(FUSE_UNLINK)),
            ("FUSE_RMDIR", i64::from(FUSE_RMDIR)),
            ("FUSE_RENAME", i64::from(FUSE_RENAME)),
            ("FUSE_LINK", i64::from(FUSE_LINK)),
            ("FUSE_OPEN", i64::from(FUSE_OPEN)),
            ("FUSE_READ", i64::from(FUSE_READ)),
            ("FUSE_WRITE", i64::from(FUSE_WRITE)),
            ("FUSE_STATFS", i64::from(FUSE_STATFS)),
            ("FUSE_RELEASE", i64::from(FUSE_RELEASE)),
            ("FUSE_FSYNC", i64::from(FUSE_FSYNC)),
            ("FUSE_FLUSH", i64::from(FUSE_FLUSH)),
            ("FUSE_INIT", i64::from(FUSE_INIT)),
            ("FUSE_OPENDIR", i64::from(FUSE_OPENDIR)),
            ("FUSE_READDIR", i64::from(FUSE_READDIR)),
            ("FUSE_RELEASEDIR", i64::from(FUSE_RELEASEDIR)),
            ("FUSE_DESTROY", i64::from(FUSE_DESTROY)),
            ("FUSE_FORGET", i64::from(FUSE_FORGET)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
    }

    /// Every member's offset in the bytes the daemon reads and writes (`<sys/fusebuf.h>`'s
    /// LP64 layout, the same on both machines: fixed-width members only), beyond the
    /// compile-time checks: a reordered or resized member fails here by name.
    #[test]
    fn every_wire_member_sits_at_the_c_offset() {
        macro_rules! at {
            ($t:ty, $size:expr, $($f:ident = $off:expr),+ $(,)?) => {
                assert_eq!(size_of::<$t>(), $size, "sizeof {}", stringify!($t));
                $( assert_eq!(offset_of!($t, $f), $off, "{}.{}", stringify!($t), stringify!($f)); )+
            };
        }
        at!(
            FuseAttr,
            88,
            ino = 0,
            size = 8,
            blocks = 16,
            atime = 24,
            mtime = 32,
            ctime = 40,
            atimensec = 48,
            mtimensec = 52,
            ctimensec = 56,
            mode = 60,
            nlink = 64,
            uid = 68,
            gid = 72,
            rdev = 76,
            blksize = 80,
            padding = 84
        );
        at!(
            FuseEntryOut,
            128,
            nodeid = 0,
            generation = 8,
            entry_valid = 16,
            attr_valid = 24,
            entry_valid_nsec = 32,
            attr_valid_nsec = 36,
            attr = 40
        );
        at!(FuseForgetIn, 8, nlookup = 0);
        at!(FuseGetattrIn, 16, getattr_flags = 0, dummy = 4, fh = 8);
        at!(
            FuseAttrOut,
            104,
            attr_valid = 0,
            attr_valid_nsec = 8,
            dummy = 12,
            attr = 16
        );
        at!(FuseMknodIn, 16, mode = 0, rdev = 4, umask = 8, padding = 12);
        at!(FuseMkdirIn, 8, mode = 0, umask = 4);
        at!(FuseRenameIn, 8, newdir = 0);
        at!(FuseLinkIn, 8, oldnodeid = 0);
        at!(
            FuseSetattrIn,
            88,
            valid = 0,
            padding = 4,
            fh = 8,
            size = 16,
            lock_owner = 24,
            atime = 32,
            mtime = 40,
            unused2 = 48,
            atimensec = 56,
            mtimensec = 60,
            unused3 = 64,
            mode = 68,
            unused4 = 72,
            uid = 76,
            gid = 80,
            unused5 = 84
        );
        at!(FuseOpenIn, 8, flags = 0, unused = 4);
        at!(FuseOpenOut, 16, fh = 0, open_flags = 8, padding = 12);
        at!(
            FuseReleaseIn,
            24,
            fh = 0,
            flags = 8,
            release_flags = 12,
            lock_owner = 16
        );
        at!(
            FuseFlushIn,
            24,
            fh = 0,
            unused = 8,
            padding = 12,
            lock_owner = 16
        );
        at!(
            FuseReadIn,
            40,
            fh = 0,
            offset = 8,
            size = 16,
            read_flags = 20,
            lock_owner = 24,
            flags = 32,
            padding = 36
        );
        at!(
            FuseWriteIn,
            40,
            fh = 0,
            offset = 8,
            size = 16,
            write_flags = 20,
            lock_owner = 24,
            flags = 32,
            padding = 36
        );
        at!(FuseWriteOut, 8, size = 0, padding = 4);
        at!(
            FuseKstatfs,
            80,
            blocks = 0,
            bfree = 8,
            bavail = 16,
            files = 24,
            ffree = 32,
            bsize = 40,
            namelen = 44,
            frsize = 48,
            padding = 52,
            spare = 56
        );
        at!(FuseStatfsOut, 80, st = 0);
        at!(FuseFsyncIn, 16, fh = 0, fsync_flags = 8, padding = 12);
        at!(
            FuseInitIn,
            16,
            major = 0,
            minor = 4,
            max_readahead = 8,
            flags = 12
        );
        at!(
            FuseInitOut,
            24,
            major = 0,
            minor = 4,
            max_readahead = 8,
            flags = 12,
            max_background = 16,
            congestion_threshold = 18,
            max_write = 20
        );
        at!(
            FuseInHeader,
            40,
            len = 0,
            opcode = 4,
            unique = 8,
            nodeid = 16,
            uid = 24,
            gid = 28,
            pid = 32,
            padding = 36
        );
        at!(FuseOutHeader, 16, len = 0, error = 4, unique = 8);
        at!(FuseDirent, 24, ino = 0, off = 8, namelen = 16, r#type = 20);
        assert_eq!(FUSE_NAME_OFFSET, 24);
    }

    #[test]
    fn header_bytes_are_the_c_layout() {
        let h = FuseInHeader {
            len: 0x0102_0304,
            opcode: FUSE_LOOKUP,
            unique: 0x1122_3344_5566_7788,
            nodeid: 9,
            uid: 1000,
            gid: 1001,
            pid: 100_005,
            padding: 0,
        };
        let b = bytes_of(slice::from_ref(&h));
        assert_eq!(b.len(), 40);
        assert_eq!(&b[0..4], &0x0102_0304u32.to_ne_bytes());
        assert_eq!(&b[4..8], &FUSE_LOOKUP.to_ne_bytes());
        assert_eq!(&b[8..16], &0x1122_3344_5566_7788u64.to_ne_bytes());
        assert_eq!(&b[16..24], &9u64.to_ne_bytes());
        assert_eq!(&b[24..28], &1000u32.to_ne_bytes());
        assert_eq!(&b[28..32], &1001u32.to_ne_bytes());
        assert_eq!(&b[32..36], &100_005u32.to_ne_bytes());

        let mut o = FuseOutHeader::default();
        bytes_of_mut(slice::from_mut(&mut o))[4..8].copy_from_slice(&(-2i32).to_ne_bytes());
        assert_eq!(o.error, -2);
    }

    #[test]
    fn op_members_overlap_like_the_c_union() {
        let op = FusebufOp::new();
        op.set(&FuseOpenIn {
            flags: 0x0202,
            unused: 0,
        });
        // The input occupies the first bytes of the union; the reply overwrites the same bytes.
        assert_eq!(op.get::<FuseOpenIn>().flags, 0x0202);
        let mut first = [0u8; 4];
        op.read_bytes(&mut first);
        assert_eq!(first, 0x0202u32.to_ne_bytes());
        op.set(&FuseOpenOut {
            fh: 77,
            open_flags: 1,
            padding: 0,
        });
        assert_eq!(op.get::<FuseOpenOut>().fh, 77);
        assert_eq!(op.get::<FuseOpenIn>().flags, 77);

        // Writing a small member leaves the bytes past it alone.
        let op = FusebufOp::new();
        op.write_bytes(&[0; 8]);
        let mut nine = [0u8; 9];
        nine[8] = 0xaa;
        op.write_bytes(&nine);
        op.set(&FuseMkdirIn { mode: 1, umask: 2 });
        op.read_bytes(&mut nine);
        assert_eq!(nine[8], 0xaa);
        assert_eq!(op.get::<FuseMkdirIn>(), FuseMkdirIn { mode: 1, umask: 2 });

        // The whole union: the largest member, all 128 bytes.
        let e = FuseEntryOut {
            nodeid: 9,
            attr: FuseAttr {
                padding: 0xdead_beef,
                ..FuseAttr::default()
            },
            ..FuseEntryOut::default()
        };
        op.set(&e);
        assert_eq!(op.get::<FuseEntryOut>(), e);
    }

    #[test]
    fn dirent_size_rounds_to_eight() {
        assert_eq!(fuse_dirent_align(0), 0);
        assert_eq!(fuse_dirent_align(1), 8);
        assert_eq!(fuse_dirent_align(8), 8);
        assert_eq!(fuse_dirent_align(25), 32);
        let mut d = FuseDirent {
            namelen: 1,
            ..FuseDirent::default()
        };
        assert_eq!(fuse_dirent_size(&d), 32);
        d.namelen = 8;
        assert_eq!(fuse_dirent_size(&d), 32);
        d.namelen = 9;
        assert_eq!(fuse_dirent_size(&d), 40);
    }

    #[test]
    fn a_dirent_head_is_read_from_the_reply_bytes() {
        let mut b = [0u8; 32];
        b[0..8].copy_from_slice(&7u64.to_ne_bytes());
        b[8..16].copy_from_slice(&3u64.to_ne_bytes());
        b[16..20].copy_from_slice(&5u32.to_ne_bytes());
        b[20..24].copy_from_slice(&4u32.to_ne_bytes());
        b[24..29].copy_from_slice(b"hello");
        let d: FuseDirent = pod_from_bytes(&b).unwrap();
        assert_eq!((d.ino, d.off, d.namelen, d.r#type), (7, 3, 5, 4));
        assert_eq!(&b[FUSE_NAME_OFFSET..FUSE_NAME_OFFSET + 5], b"hello");
        assert!(pod_from_bytes::<FuseDirent>(&b[..23]).is_none());
    }
}
/* </TESTS> */
