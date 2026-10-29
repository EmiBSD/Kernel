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
//! `<sys/mount.h>`: the mounted file system (`struct mount`), the operations a file system
//! type provides (`struct vfsops`, called through the `VFS_*` macros), its configuration
//! entry (`struct vfsconf`), `struct statfs` and the `MNT_*` flags, file handles, the
//! `CTL_VFS` names and the buffer cache statistics.
//!
//! Upstream: sys/sys/mount.h @ 3ce1f3f79392
//!
//! Since N5.2 `struct mount` and `struct vfsconf` are the kernel crate's (`kernel::fs::mount`,
//! never freed: a mount whose last reference goes is recycled, D-N5.2-2) and `struct vfsops`
//! is the kernel's trait `Vfs`; this module is their face for the `bsd` file systems: the
//! aliases [`Mount`] and [`Vfsconf`], `struct vfsops` of `fn` members (which
//! `kern/vfs_vops.rs` implements as the kernel's `Vfs`), the `VFS_*` names over the kernel's
//! and the per-file system `*_args`.
//!
//! ## How a file system plugs in
//! - It fills a `static` [`Vfsops`] and an entry of `vfsconflist[]` (`kern/vfs_init.rs`).
//!   `vfs_root`, `vfs_vget` and `vfs_fhtovp` return the vnode where the C fills `*vpp`;
//!   `vfs_mount` receives the kernel copy of the user's arguments as a byte slice of
//!   `vfc_datasize` bytes, which the file system reads as its own `*_args` structure.
//! - Its per-mount data hangs from `mnt_data` (`*mut c_void`, as in C).
//!
//! ## Deviations
//! - `mnt_stat` is read with the kernel's `Mount::stat` and changed with `Mount::with_stat`.
//! - `struct statfs` names the holes the C compiler leaves (`_pad0` after `f_iosize`, `_pad1`
//!   before `mount_info`), so the structure is plain data that `copyout` may read whole;
//!   `union mount_info` is its 160 bytes, 8-aligned: the per-filesystem views come with
//!   their file systems (`ufs_args` and the `export_args` it embeds came with ffs, `struct
//!   mfs_args` (`MfsArgs`) with MFS, `iso_args`, `msdosfs_args`, `udf_args` and `tmpfs_args`
//!   with M10c, `nfs_args` with NFS (M10e); each `*Args::from_bytes` reads them out of the kernel copy of the mount
//!   arguments). `ntfs_args` (`NtfsArgs`) and the two `NTFS_MFLAG_*` came with NTFS (M10d),
//!   under cfg `option_ntfs` (feature `ntfs`, amd64 only: `sys/build.rs`).
//! - `VFS_*` are functions with the macros' names (`#[allow(non_snake_case)]`).
//! - `struct netcred` and `struct netexport` are always defined, as the C header does, over
//!   the ported radix tree (`net/radix.rs`): their members are `Cell`s, the `ne_rtable_inet`
//!   pointer an `Option<&'static RadixNodeHead>`; a `Netcred` is `malloc`ed with the
//!   address and mask it is keyed by behind it, as in C (`kern/vfs_subr.rs`). `struct
//!   nfs_args` (`NfsArgs`) reads like the other `*Args`; its `addr`, `fh` and `hostname`
//!   are user addresses.
//! - The user-level prototypes (`mount`, `statfs`, ...) are not kernel material; the kernel
//!   ones are their functions in `vfs_subr.rs`, `vfs_init.rs` and `vfs_syscalls.rs`.

use core::cell::Cell;
use core::ptr;

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::net::radix::{RadixNode, RadixNodeHead};
use crate::sys::errno::Errno;
use crate::sys::mbuf::Mbuf;
use crate::sys::namei::Nameidata;
use crate::sys::proc::Proc;
use crate::sys::sysctl::Sysctlfn;
use crate::sys::types::{Gid, Ino, Mode, Off, Uid};
use crate::sys::ucred::{Ucred, UcredCell};
use crate::sys::vnode::Vnode;
/// `struct bcachestats`: buffer cache statistics, `hal`'s (N5.1: one copy for the kernel's
/// cache and `vfs.generic.bcachestat`).
pub use hal::sys::mount::{
    Bcachestats, Fhandle, Fid, Fsid, MAXFIDSZ, MFSNAMELEN, MNAMELEN, MNT_ASYNC, MNT_BITS,
    MNT_DEFEXPORTED, MNT_DELEXPORT, MNT_DOOMED, MNT_EXPORTANON, MNT_EXPORTED, MNT_EXRDONLY,
    MNT_FORCE, MNT_LAZY, MNT_LOCAL, MNT_NOATIME, MNT_NODEV, MNT_NOEXEC, MNT_NOPERM, MNT_NOSUID,
    MNT_NOWAIT, MNT_OP_FLAGS, MNT_QUOTA, MNT_RDONLY, MNT_RELOAD, MNT_ROOTFS, MNT_SOFTDEP,
    MNT_STALLED, MNT_SWAPPABLE, MNT_SYNCHRONOUS, MNT_UNMOUNT, MNT_UPDATE, MNT_VISFLAGMASK,
    MNT_WAIT, MNT_WANTRDWR, MNT_WXALLOWED, MOUNT_AFS, MOUNT_CD9660, MOUNT_EXT2FS, MOUNT_FFS,
    MOUNT_FUSEFS, MOUNT_MFS, MOUNT_MSDOS, MOUNT_NCPFS, MOUNT_NFS, MOUNT_NTFS, MOUNT_TMPFS,
    MOUNT_UDF, MOUNT_UFS, MountInfo, Statfs, VB_DUPOK, VB_NOWAIT, VB_READ, VB_WAIT, VB_WRITE,
    VFS_BCACHESTAT, VFS_CONF, VFS_GENERIC, VFS_MAXTYPENUM, VfsconfAbi,
};
use kernel::fs::vops::Cred;

/// `struct export_args`: export arguments for local filesystem mount calls. The two
/// `struct sockaddr *` are user addresses (`usize`), which the kernel copies in when it
/// exports (`NFSSERVER`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ExportArgs {
    /// `ex_flags`: export related flags.
    pub ex_flags: i32,
    /// `ex_root`: mapping for root uid.
    pub ex_root: Uid,
    /// `ex_anon`: mapping for anonymous user.
    pub ex_anon: crate::sys::ucred::Xucred,
    /// `ex_addr`: net address to which exported.
    pub ex_addr: usize,
    /// `ex_addrlen`: and the net address length.
    pub ex_addrlen: i32,
    /// `ex_mask`: mask of valid bits in saddr.
    pub ex_mask: usize,
    /// `ex_masklen`: and the smask length.
    pub ex_masklen: i32,
}

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
        if data.len() < Self::SIZE {
            return None;
        }
        // SAFETY: `data` holds `SIZE` readable bytes (checked), and the structure is
        // integers, valid for any bit pattern (its padding bytes are padding); the read is
        // unaligned.
        Some(unsafe { ptr::read_unaligned(data.as_ptr().cast::<UfsArgs>()) })
    }
}

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
        if data.len() < Self::SIZE {
            return None;
        }
        // SAFETY: `data` holds `SIZE` readable bytes (checked), and the structure is
        // integers, valid for any bit pattern (its padding bytes are padding); the read is
        // unaligned.
        Some(unsafe { ptr::read_unaligned(data.as_ptr().cast::<MfsArgs>()) })
    }
}

/// The `SIZE` and `from_bytes` of a file system's mount arguments, as [`UfsArgs`] has them:
/// `sys_mount` copies `vfc_datasize` bytes in, and the file system reads its structure out.
macro_rules! mount_args_from_bytes {
    ($t:ident, $c:literal) => {
        impl $t {
            #[doc = concat!("`sizeof(struct ", $c, ")`: the `vfc_datasize` of its file system.")]
            pub const SIZE: usize = size_of::<$t>();

            /// The arguments in the kernel copy `sys_mount` made of them (at least `SIZE`
            /// bytes), `None` when there are none (the C's NULL `data`).
            pub fn from_bytes(data: &[u8]) -> Option<Self> {
                if data.len() < Self::SIZE {
                    return None;
                }
                // SAFETY: `data` holds `SIZE` readable bytes (checked), and the structure is
                // integers, valid for any bit pattern (its padding bytes are padding); the
                // read is unaligned.
                Some(unsafe { ptr::read_unaligned(data.as_ptr().cast::<$t>()) })
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
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NfsArgs {
    /// `version`: args structure version number (`NFS_ARGSVERSION`).
    pub version: i32,
    /// `addr`: file server address.
    pub addr: usize,
    /// `addrlen`: length of address.
    pub addrlen: i32,
    /// `sotype`: socket type.
    pub sotype: i32,
    /// `proto`: and protocol.
    pub proto: i32,
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
    pub uid: Uid,
    /// `gid`: gid that owns msdosfs files.
    pub gid: Gid,
    /// `mask`: mask to be applied for msdosfs perms.
    pub mask: Mode,
    /// `flags`: the `MSDOSFSMNT_*` below.
    pub flags: i32,
}

mount_args_from_bytes!(MsdosfsArgs, "msdosfs_args");

/// `MSDOSFSMNT_SHORTNAME`: force old DOS short names only.
pub const MSDOSFSMNT_SHORTNAME: i32 = 0x01;
/// `MSDOSFSMNT_LONGNAME`: force Win'95 long names.
pub const MSDOSFSMNT_LONGNAME: i32 = 0x02;
/// `MSDOSFSMNT_NOWIN95`: completely ignore Win95 entries.
pub const MSDOSFSMNT_NOWIN95: i32 = 0x04;

/// `struct ntfs_args`: arguments to mount ntfs filesystems. `fspec` is a user address.
#[cfg(option_ntfs)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NtfsArgs {
    /// `fspec`: block special device to mount.
    pub fspec: usize,
    /// `export_info`: network export information.
    pub export_info: ExportArgs,
    /// `uid`: uid that owns ntfs files.
    pub uid: Uid,
    /// `gid`: gid that owns ntfs files.
    pub gid: Gid,
    /// `mode`: mask to be applied for ntfs perms.
    pub mode: Mode,
    /// `flag`: additional flags, the `NTFS_MFLAG_*` below.
    pub flag: u64,
}

#[cfg(option_ntfs)]
mount_args_from_bytes!(NtfsArgs, "ntfs_args");

/// `NTFS_MFLAG_CASEINS`: ntfs mount option, case-insensitive lookups.
#[cfg(option_ntfs)]
pub const NTFS_MFLAG_CASEINS: u64 = 0x0000_0001;
/// `NTFS_MFLAG_ALLNAMES`: ntfs mount option, list the DOS names too.
#[cfg(option_ntfs)]
pub const NTFS_MFLAG_ALLNAMES: u64 = 0x0000_0002;

/// `struct udf_args`: arguments to mount UDF file systems. `fspec` is a user address.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UdfArgs {
    /// `fspec`: block special device to mount.
    pub fspec: usize,
    /// `lastblock`: special device last block.
    pub lastblock: u32,
}

mount_args_from_bytes!(UdfArgs, "udf_args");

/// `TMPFS_ARGS_VERSION`.
pub const TMPFS_ARGS_VERSION: i32 = 1;

/// `struct tmpfs_args`: arguments to mount tmpfs file systems.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TmpfsArgs {
    /// `ta_version`: `TMPFS_ARGS_VERSION`.
    pub ta_version: i32,
    /// `ta_nodes_max`: size counter, the most nodes.
    pub ta_nodes_max: Ino,
    /// `ta_size_max`: size counter, the most bytes.
    pub ta_size_max: Off,
    /// `ta_root_uid`: root node attribute.
    pub ta_root_uid: Uid,
    /// `ta_root_gid`: root node attribute.
    pub ta_root_gid: Gid,
    /// `ta_root_mode`: root node attribute.
    pub ta_root_mode: Mode,
}

mount_args_from_bytes!(TmpfsArgs, "tmpfs_args");

/// `struct fusefs_args`: arguments to mount fusefs filesystems. `name` is a user address.
#[cfg(feature = "fuse")]
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
}

#[cfg(feature = "fuse")]
mount_args_from_bytes!(FusefsArgs, "fusefs_args");

/// `struct mount`: the kernel's.
pub type Mount = kernel::fs::mount::Mount<BsdProcGlue>;

/// `struct vfsconf`: the kernel's (`vfsconflist[]` is `kern/vfs_init.rs`'s).
pub type Vfsconf = kernel::fs::mount::Vfsconf<BsdProcGlue>;

pub use kernel::fs::mount::{
    MountIter, mountlist, mountlist_first, mountlist_insert_head, mountlist_insert_tail,
    mountlist_is_empty, mountlist_next, mountlist_remove,
};

/// `BUFPAGES_DEFICIT`: how far the buffer cache is below its low water mark, in pages.
pub fn bufpages_deficit() -> i64 {
    use crate::sys::buf::{BCSTATS, BUFLOWPAGES};
    let d = BUFLOWPAGES.load(core::sync::atomic::Ordering::Relaxed)
        - BCSTATS
            .numbufpages
            .load(core::sync::atomic::Ordering::Relaxed);
    d.max(0)
}

/// `BUFPAGES_INACT`: the buffer cache's clean pages above its low water mark.
pub fn bufpages_inact() -> i64 {
    use crate::sys::buf::{BCSTATS, BUFLOWPAGES};
    let d = BCSTATS
        .numcleanpages
        .load(core::sync::atomic::Ordering::Relaxed)
        - BUFLOWPAGES.load(core::sync::atomic::Ordering::Relaxed);
    d.max(0)
}

/// The type of `vfs_mount(mp, path, data, ndp, p)`.
pub type VfsMountFn =
    fn(&'static Mount, &[u8], &mut [u8], &mut Nameidata<'_>, &Proc) -> Result<(), Errno>;

/// The type of `vfs_checkexp(mp, nam, extflagsp, credanonp)`.
pub type VfsCheckexpFn =
    fn(&'static Mount, &Mbuf, &mut i32, &mut *const Ucred) -> Result<(), Errno>;

/// The type of `vfs_init(vfsconf)`.
pub type VfsInitFn = fn(&'static Vfsconf) -> Result<(), Errno>;

/// `struct vfsops`: operations supported on mounted file system.
pub struct Vfsops {
    /// `vfs_mount(mp, path, data, ndp, p)`: `data` is the kernel copy of the user's
    /// arguments.
    pub vfs_mount: VfsMountFn,
    /// `vfs_start(mp, flags, p)`.
    pub vfs_start: fn(&'static Mount, i32, &Proc) -> Result<(), Errno>,
    /// `vfs_unmount(mp, mntflags, p)`.
    pub vfs_unmount: fn(&'static Mount, i32, &Proc) -> Result<(), Errno>,
    /// `vfs_root(mp, vpp)`: the root vnode, locked.
    pub vfs_root: fn(&'static Mount) -> Result<&'static Vnode, Errno>,
    /// `vfs_quotactl(mp, cmds, uid, arg, p)`: `arg` is a user address.
    pub vfs_quotactl: fn(&'static Mount, i32, Uid, usize, &Proc) -> Result<(), Errno>,
    /// `vfs_statfs(mp, sbp, p)`.
    pub vfs_statfs: fn(&'static Mount, &mut Statfs, &Proc) -> Result<(), Errno>,
    /// `vfs_sync(mp, waitfor, stall, cred, p)`.
    pub vfs_sync: fn(&'static Mount, i32, i32, *const Ucred, &Proc) -> Result<(), Errno>,
    /// `vfs_vget(mp, ino, vpp)`.
    pub vfs_vget: fn(&'static Mount, Ino) -> Result<&'static Vnode, Errno>,
    /// `vfs_fhtovp(mp, fhp, vpp)`.
    pub vfs_fhtovp: fn(&'static Mount, &Fid) -> Result<&'static Vnode, Errno>,
    /// `vfs_vptofh(vp, fhp)`.
    pub vfs_vptofh: fn(&'static Vnode, &mut Fid) -> Result<(), Errno>,
    /// `vfs_init(vfsconf)`, NULL when the type needs no initialisation.
    pub vfs_init: Option<VfsInitFn>,
    /// `vfs_sysctl(name, namelen, oldp, oldlenp, newp, newlen, p)`, NULL when the type has no
    /// `vfs.<type>` node.
    pub vfs_sysctl: Option<Sysctlfn>,
    /// `vfs_checkexp(mp, nam, extflagsp, credanonp)`.
    pub vfs_checkexp: VfsCheckexpFn,
}

/// `VFS_MOUNT(MP, PATH, DATA, NDP, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_MOUNT(
    mp: &'static Mount,
    path: &[u8],
    data: &mut [u8],
    ndp: &mut Nameidata<'_>,
    p: &Proc,
) -> Result<(), Errno> {
    kernel::fs::vops::VFS_MOUNT(mp, path, data, ndp, p)
}

/// `VFS_START(MP, FLAGS, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_START(mp: &'static Mount, flags: i32, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_START(mp, flags, p)
}

/// `VFS_UNMOUNT(MP, FORCE, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_UNMOUNT(mp: &'static Mount, force: i32, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_UNMOUNT(mp, force, p)
}

/// `VFS_ROOT(MP, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_ROOT(mp: &'static Mount) -> Result<&'static Vnode, Errno> {
    kernel::fs::vops::VFS_ROOT(mp)
}

/// `VFS_QUOTACTL(MP, C, U, A, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_QUOTACTL(mp: &'static Mount, c: i32, u: Uid, a: usize, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_QUOTACTL(mp, c, u, a, p)
}

/// `VFS_STATFS(MP, SBP, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_STATFS(mp: &'static Mount, sbp: &mut Statfs, p: &Proc) -> Result<(), Errno> {
    kernel::fs::vops::VFS_STATFS(mp, sbp, p)
}

/// `VFS_SYNC(MP, W, S, C, P)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_SYNC(
    mp: &'static Mount,
    w: i32,
    s: i32,
    c: *const Ucred,
    p: &Proc,
) -> Result<(), Errno> {
    kernel::fs::vops::VFS_SYNC(mp, w, s, Cred::from_raw(c), p)
}

/// `VFS_VGET(MP, INO, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_VGET(mp: &'static Mount, ino: Ino) -> Result<&'static Vnode, Errno> {
    kernel::fs::vops::VFS_VGET(mp, ino)
}

/// `VFS_FHTOVP(MP, FIDP, VPP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_FHTOVP(mp: &'static Mount, fidp: &Fid) -> Result<&'static Vnode, Errno> {
    kernel::fs::vops::VFS_FHTOVP(mp, fidp)
}

/// `VFS_VPTOFH(VP, FIDP)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_VPTOFH(vp: &'static Vnode, fidp: &mut Fid) -> Result<(), Errno> {
    kernel::fs::vops::VFS_VPTOFH(vp, fidp)
}

/// `VFS_CHECKEXP(MP, NAM, EXFLG, CRED)`.
#[allow(non_snake_case)] // the C macro's name
pub fn VFS_CHECKEXP(
    mp: &'static Mount,
    nam: &Mbuf,
    exflg: &mut i32,
    cred: &mut *const Ucred,
) -> Result<(), Errno> {
    // `vfs_checkexp` is the NFS server's, `bsd`'s: the table as `bsd` knows it.
    match mp.op().as_any().and_then(|a| a.downcast_ref::<Vfsops>()) {
        Some(ops) => (ops.vfs_checkexp)(mp, nam, exflg, cred),
        None => panic(format_args!(
            "VFS_CHECKEXP: mount {:p} of a kernel file system",
            mp
        )),
    }
}

/// `struct netcred`: network address lookup element. The radix tree of a [`Netexport`] holds
/// these `malloc(M_NETADDR)`ed with the address and mask they are keyed by right behind
/// them (`netc_len` is the size of the whole allocation); the first member must stay
/// `netc_rnodes`, as `vfs_export_lookup` turns the leaf `rn_match` finds back into its
/// `Netcred`.
#[repr(C)]
pub struct Netcred {
    /// `netc_rnodes`: the radix tree's leaf and internal node for this entry.
    pub netc_rnodes: [RadixNode; 2],
    /// `netc_exflags`: the `export_args`'s `ex_flags` (`MNT_EXRDONLY`, ...).
    pub netc_exflags: Cell<i32>,
    /// `netc_len`: size of the allocation.
    pub netc_len: Cell<i32>,
    /// `netc_anon`: the credentials anonymous clients map to (the entry's own, filled in
    /// place by `vfs_hang_addrlist`).
    pub netc_anon: UcredCell,
}

// SAFETY: the members are changed under the kernel lock while the file system's export
// list is updated (`vfs_export`), as in C; the kernel serialises them like the rest of the
// mount's state.
unsafe impl Sync for Netcred {}

impl Netcred {
    /// A zeroed entry, as `malloc(M_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            netc_rnodes: [RadixNode::new(), RadixNode::new()],
            netc_exflags: Cell::new(0),
            netc_len: Cell::new(0),
            netc_anon: UcredCell::new(),
        }
    }
}

impl Default for Netcred {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct netexport`: network export information of one file system.
pub struct Netexport {
    /// `ne_defexported`: default export.
    pub ne_defexported: Netcred,
    /// `ne_rtable_inet`: individual exports.
    pub ne_rtable_inet: Cell<Option<&'static RadixNodeHead>>,
}

// SAFETY: as for `Netcred`.
unsafe impl Sync for Netexport {}

impl Netexport {
    /// An empty export list, as `malloc(M_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            ne_defexported: Netcred::new(),
            ne_rtable_inet: Cell::new(None),
        }
    }
}

impl Default for Netexport {
    fn default() -> Self {
        Self::new()
    }
}

// The amd64/arm64 layout of the `*_args` (both LP64; `struct statfs` is hal's).
const _: () = {
    use core::mem::offset_of;
    assert!(size_of::<ExportArgs>() == 120);
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
    assert!(UdfArgs::SIZE == 16);
    assert!(TmpfsArgs::SIZE == 40);
};

#[cfg(option_ntfs)]
const _: () = {
    assert!(NtfsArgs::SIZE == 152);
    assert!(core::mem::offset_of!(NtfsArgs, uid) == 128);
    assert!(core::mem::offset_of!(NtfsArgs, flag) == 144);
    assert!(NtfsArgs::SIZE <= size_of::<MountInfo>());
};

#[cfg(feature = "fuse")]
const _: () = {
    assert!(FusefsArgs::SIZE == 24);
    assert!(core::mem::offset_of!(FusefsArgs, fd) == 8);
    assert!(core::mem::offset_of!(FusefsArgs, allow_other) == 16);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/mount.h");
        for (name, value) in [
            ("MNT_RDONLY", MNT_RDONLY),
            ("MNT_NOPERM", MNT_NOPERM),
            ("MNT_WXALLOWED", MNT_WXALLOWED),
            ("MNT_ROOTFS", MNT_ROOTFS),
            ("MNT_VISFLAGMASK", MNT_VISFLAGMASK),
            ("MNT_UPDATE", MNT_UPDATE),
            ("MNT_STALLED", MNT_STALLED),
            ("MNT_UNMOUNT", MNT_UNMOUNT),
            ("MNT_DOOMED", MNT_DOOMED),
            ("MNT_LAZY", MNT_LAZY),
            ("MFSNAMELEN", MFSNAMELEN as i32),
            ("MNAMELEN", MNAMELEN as i32),
            ("MAXFIDSZ", MAXFIDSZ as i32),
            ("VFS_BCACHESTAT", VFS_BCACHESTAT),
            ("VB_DUPOK", VB_DUPOK),
            ("ISOFSMNT_NORRIP", ISOFSMNT_NORRIP),
            ("ISOFSMNT_GENS", ISOFSMNT_GENS),
            ("ISOFSMNT_EXTATT", ISOFSMNT_EXTATT),
            ("ISOFSMNT_NOJOLIET", ISOFSMNT_NOJOLIET),
            ("ISOFSMNT_SESS", ISOFSMNT_SESS),
            ("MSDOSFSMNT_SHORTNAME", MSDOSFSMNT_SHORTNAME),
            ("MSDOSFSMNT_LONGNAME", MSDOSFSMNT_LONGNAME),
            ("MSDOSFSMNT_NOWIN95", MSDOSFSMNT_NOWIN95),
            ("TMPFS_ARGS_VERSION", TMPFS_ARGS_VERSION),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value)),
                "{name}"
            );
        }
        // The NFSMNT_* bits fill all 32 bits; the C's `int` constants are the unsigned values.
        for (name, value) in [
            ("NFS_ARGSVERSION", NFS_ARGSVERSION),
            ("NFSMNT_RESVPORT", NFSMNT_RESVPORT),
            ("NFSMNT_SOFT", NFSMNT_SOFT),
            ("NFSMNT_WSIZE", NFSMNT_WSIZE),
            ("NFSMNT_RSIZE", NFSMNT_RSIZE),
            ("NFSMNT_TIMEO", NFSMNT_TIMEO),
            ("NFSMNT_RETRANS", NFSMNT_RETRANS),
            ("NFSMNT_MAXGRPS", NFSMNT_MAXGRPS),
            ("NFSMNT_INT", NFSMNT_INT),
            ("NFSMNT_NOCONN", NFSMNT_NOCONN),
            ("NFSMNT_NQNFS", NFSMNT_NQNFS),
            ("NFSMNT_NFSV3", NFSMNT_NFSV3),
            ("NFSMNT_KERB", NFSMNT_KERB),
            ("NFSMNT_DUMBTIMR", NFSMNT_DUMBTIMR),
            ("NFSMNT_LEASETERM", NFSMNT_LEASETERM),
            ("NFSMNT_READAHEAD", NFSMNT_READAHEAD),
            ("NFSMNT_DEADTHRESH", NFSMNT_DEADTHRESH),
            ("NFSMNT_NOAC", NFSMNT_NOAC),
            ("NFSMNT_RDIRPLUS", NFSMNT_RDIRPLUS),
            ("NFSMNT_READDIRSIZE", NFSMNT_READDIRSIZE),
            ("NFSMNT_ACREGMIN", NFSMNT_ACREGMIN),
            ("NFSMNT_ACREGMAX", NFSMNT_ACREGMAX),
            ("NFSMNT_ACDIRMIN", NFSMNT_ACDIRMIN),
            ("NFSMNT_ACDIRMAX", NFSMNT_ACDIRMAX),
            ("NFSMNT_INTERNAL", NFSMNT_INTERNAL),
            ("NFSMNT_HASWRITEVERF", NFSMNT_HASWRITEVERF),
            ("NFSMNT_GOTPATHCONF", NFSMNT_GOTPATHCONF),
            ("NFSMNT_GOTFSINFO", NFSMNT_GOTFSINFO),
            ("NFSMNT_MNTD", NFSMNT_MNTD),
            ("NFSMNT_DISMINPROG", NFSMNT_DISMINPROG),
            ("NFSMNT_DISMNT", NFSMNT_DISMNT),
            ("NFSMNT_SNDLOCK", NFSMNT_SNDLOCK),
            ("NFSMNT_WANTSND", NFSMNT_WANTSND),
            ("NFSMNT_RCVLOCK", NFSMNT_RCVLOCK),
            ("NFSMNT_WANTRCV", NFSMNT_WANTRCV),
            ("NFSMNT_WAITAUTH", NFSMNT_WAITAUTH),
            ("NFSMNT_HASAUTH", NFSMNT_HASAUTH),
            ("NFSMNT_WANTAUTH", NFSMNT_WANTAUTH),
            ("NFSMNT_AUTHERR", NFSMNT_AUTHERR),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value as u32)),
                "{name}"
            );
        }
    }

    #[test]
    fn nfs_args_read_out_of_the_kernel_copy() {
        let mut bytes = [0u8; NfsArgs::SIZE];
        bytes[..4].copy_from_slice(&NFS_ARGSVERSION.to_ne_bytes());
        bytes[44..48].copy_from_slice(&(NFSMNT_NFSV3 | NFSMNT_RSIZE).to_ne_bytes());
        bytes[96..100].copy_from_slice(&7i32.to_ne_bytes());
        let args = NfsArgs::from_bytes(&bytes).expect("arguments");
        assert_eq!(args.version, NFS_ARGSVERSION);
        assert_eq!(args.flags, NFSMNT_NFSV3 | NFSMNT_RSIZE);
        assert_eq!(args.acregmin, 7);
        assert!(NfsArgs::from_bytes(&bytes[..NfsArgs::SIZE - 1]).is_none());
    }
}
/* </TESTS> */
