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
 * Copyright (c) 2012-2013 Sylvestre Gallon <ccna.syl@gmail.com>
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
//! `fusefs.h`: the FUSE file system's sysctl identifiers, its per-mount structure, the
//! operations a daemon turned out not to implement, and the file system's globals of a glue
//! ([`FuseTables`]: the C's `fusefs_fbuf_pool`, `fuse_d_list`, the three `stat_*` counters and
//! the inode hash, which the header and the `.c` files declare).
//!
//! Upstream: sys/miscfs/fuse/fusefs.h @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fusefs.rs@3c62ede68802
//!
//! The prototypes of the header are functions of the files that define them:
//! `fusefs_file_open`/`fusefs_file_close` (`fuse_file`) and the device helpers
//! `fuse_device_cleanup`, `fuse_device_queue_fbuf`, `fuse_device_set_fmp` (`fuse_device`);
//! `fusefs_vops` is `fuse_vnops`'s [`FuseVnops`], the tables' member `vops`.
//!
//! ## Deviations
//! - `FUSE_DEBUG` is not defined (as in GENERIC), so `DPRINTF` expands to nothing and every
//!   `DPRINTF` of the `.c` files is a comment (LZ's).
//! - `vmstat -m`'s sizes (the C's `sizeof`s pinned by `_Static_assert`s over the reference
//!   headers): the `"fmsg"` pool's item is 304 bytes (the 288-byte `Fusebuf` and the item's
//!   count and pool; C 224, LZ 216: the N3 class of item sizes, U3); a mount's `M_FUSEFS`
//!   block 48 bytes (`MArc`'s 16 beside the 32-byte `FusefsMnt`; C 32, LZ 32), in the 64-byte
//!   bucket where the C's is in the 32; a node's 224 (C 160, LZ 200), the 256-byte bucket as
//!   the C's; the device's 280 in `M_DEVBUF`'s 512-byte bucket (C 128: `fuse_device`'s
//!   deviations). The kinds, counts and places are the C's.
//!
//! ## Redesign
//! - `struct fusefs_mnt` ([`FusefsMnt`]) is a counted block of `malloc(M_FUSEFS)`
//!   ([`FusefsMntRef`], `hal::malloc::MArc`): the mount's typed slot holds one (N5.9f2's
//!   `fusefs_mount` hangs it where the C sets `mnt_data`), every node and the device it
//!   serves hold one, so `VFSTOFUSEFS` is a handle ([`vfstofusefs`]) and no reference
//!   outlives the block; LZ's `&'static` over a raw block and its `unsafe` deref are gone.
//!   The members that change after the mount (`undef_op`, `max_write`, `sess_init`) are
//!   words, written under the kernel lock as the C's.
//! - The C's globals of `fuse_device.c`, `fuse_ihash.c` and `fuse_vfsops.c` are the members of
//!   [`FuseTables`], `VfsTables::fuse` (a static generic over the glue cannot exist): the
//!   device list a `PinList` under the kernel lock (the C's `LIST_FOREACH` with `next(e)`,
//!   O(1) a step), the counters atomics, the hash a `ReadMostly` cell `fuse_ihashinit` fills,
//!   the pool a recorded `TypedPool` (`"fmsg"`), and `fusefs_vops` (`vops`, N5.9f2: one
//!   address per glue for `vops_id`, a byte, tmpfs's `TmpfsTables` shape).
//! - `FUSEFS_NAMES` is not kept: nothing reads it (the `vfs.fuse` names are sysctl(8)'s own
//!   table in userland).

use core::sync::atomic::AtomicI32;

use hal::malloc::MArc;
use hal::pinlist::PinList;
use hal::pool::TypedPool;
use hal::slot::ReadMostly;
use hal::sys::types::Dev;

use crate::assert::kpanic;
use crate::fs::fuse::fuse_device::FuseDLink;
use crate::fs::fuse::fuse_ihash::Fuseihash;
use crate::fs::fuse::fuse_vnops::FuseVnops;
use crate::fs::fuse::fusebuf_h::Fusebuf;
use crate::fs::mount::Mount;
use crate::fs::word::{I32Word, U32Word};
use crate::proc::ProcGlue;

/// `M_FUSEFS` (`<sys/malloc.h>`): FUSE FS mount structures (the type `bsd`'s malloc(9)
/// counts the hash, the nodes, the mounts and the requests' data under).
pub(crate) const M_FUSEFS: i32 = 65;

/// `FUSEFS_OPENDEVS`: # of fuse devices opened.
pub const FUSEFS_OPENDEVS: i32 = 1;
/// `FUSEFS_INFBUFS`: # of in fbufs.
pub const FUSEFS_INFBUFS: i32 = 2;
/// `FUSEFS_WAITFBUFS`: # of fbufs waiting for a response.
pub const FUSEFS_WAITFBUFS: i32 = 3;
/// `FUSEFS_POOL_NBPAGES`: # total fusefs size.
pub const FUSEFS_POOL_NBPAGES: i32 = 4;
/// `FUSEFS_MAXID`: number of valid fusefs ids.
pub const FUSEFS_MAXID: usize = 5;

/// `UNDEF_ACCESS`: the daemon does not implement access.
pub const UNDEF_ACCESS: u32 = 1 << 0;
/// `UNDEF_MKDIR`.
pub const UNDEF_MKDIR: u32 = 1 << 1;
/// `UNDEF_CREATE`.
pub const UNDEF_CREATE: u32 = 1 << 2;
/// `UNDEF_LINK`.
pub const UNDEF_LINK: u32 = 1 << 3;
/// `UNDEF_READLINK`.
pub const UNDEF_READLINK: u32 = 1 << 4;
/// `UNDEF_RMDIR`.
pub const UNDEF_RMDIR: u32 = 1 << 5;
/// `UNDEF_REMOVE`.
pub const UNDEF_REMOVE: u32 = 1 << 6;
/// `UNDEF_SETATTR`.
pub const UNDEF_SETATTR: u32 = 1 << 7;
/// `UNDEF_RENAME`.
pub const UNDEF_RENAME: u32 = 1 << 8;
/// `UNDEF_SYMLINK`.
pub const UNDEF_SYMLINK: u32 = 1 << 9;
/// `UNDEF_MKNOD`.
pub const UNDEF_MKNOD: u32 = 1 << 10;
/// `UNDEF_FLUSH`.
pub const UNDEF_FLUSH: u32 = 1 << 11;
/// `UNDEF_FSYNC`.
pub const UNDEF_FSYNC: u32 = 1 << 12;

/// `struct fusefs_mnt`: a mounted FUSE file system, `malloc(M_FUSEFS)`ed by `fusefs_mount`
/// and freed with its last handle (the module's `## Redesign`).
pub struct FusefsMnt<G: ProcGlue> {
    /// `mp`: the mount.
    pub mp: &'static Mount<G>,
    /// `undef_op`: the `UNDEF_*` operations the daemon answered `ENOSYS` to.
    pub undef_op: U32Word,
    /// `max_read`: the largest read the daemon accepts (the mount's `max_read` option, at
    /// most `FUSEBUFMAXSIZE`).
    pub max_read: i32,
    /// `max_write`: the largest write sent to the daemon (its `FUSE_INIT` reply).
    pub max_write: I32Word,
    /// `sess_init`: 0 before `fusefs_mount` and after the device closed or the unmount,
    /// `PENDING` (2) until the daemon answered `FUSE_INIT`, 1 afterwards.
    pub sess_init: I32Word,
    /// `allow_other`: whether users other than the one who mounted may use the file system.
    pub allow_other: i32,
    /// `dev`: the fuse(4) device (clone) the daemon talks through.
    pub dev: Dev,
}

/// `struct fusefs_mnt *` held: a counted handle to a FUSE mount's block of
/// `malloc(M_FUSEFS)`, which the last handle frees.
pub type FusefsMntRef<G> = MArc<FusefsMnt<G>>;

/// `VFSTOFUSEFS(mp)`: the FUSE mount of a mount, a handle from its typed slot; `panic(9)` for
/// a mount that has none.
pub fn vfstofusefs<G: ProcGlue>(mp: &Mount<G>) -> FusefsMntRef<G> {
    match mp.node_arc::<FusefsMnt<G>>() {
        Some(fmp) => fmp,
        None => kpanic(format_args!("VFSTOFUSEFS: mount {mp:p} has no fusefs_mnt")),
    }
}

/// FUSE's globals of a glue (the module's `## Redesign`), `VfsTables::fuse`.
pub struct FuseTables<G: ProcGlue> {
    /// `fuse_d_list`: the open devices, under the kernel lock.
    pub devices: PinList<FuseDLink<G>>,
    /// `stat_opened_fusedev`: the open devices (`vfs.fuse.fusefs_open_devices`).
    pub stat_opened_fusedev: AtomicI32,
    /// `stat_fbufs_in`: the fusebufs waiting to be read by a daemon
    /// (`vfs.fuse.fusefs_fbufs_in`).
    pub stat_fbufs_in: AtomicI32,
    /// `stat_fbufs_wait`: the fusebufs waiting for a reply (`vfs.fuse.fusefs_fbufs_wait`).
    pub stat_fbufs_wait: AtomicI32,
    /// `fuse_ihashtbl`, `fuse_ihashsz` and `fuse_ihashkey`, once `fuse_ihashinit` made them.
    pub ihash: ReadMostly<Option<Fuseihash<G>>>,
    /// `fusefs_fbuf_pool` (`"fmsg"`), recorded (a device's queue parks a reference on each
    /// request it holds).
    pub fbuf_pool: TypedPool<Fusebuf>,
    /// `fusefs_vops`, the operations `fusefs_vget` gives a vnode (one address per glue).
    pub vops: FuseVnops<G>,
}

impl<G: ProcGlue> FuseTables<G> {
    /// No device, nothing counted, no hash, no pool made yet, the vnode table.
    pub const fn new() -> Self {
        Self {
            devices: PinList::new(),
            stat_opened_fusedev: AtomicI32::new(0),
            stat_fbufs_in: AtomicI32::new(0),
            stat_fbufs_wait: AtomicI32::new(0),
            ihash: ReadMostly::new(None),
            fbuf_pool: TypedPool::new(),
            vops: FuseVnops::new(),
        }
    }
}

impl<G: ProcGlue> Default for FuseTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `fuse_test_reset`: forgets the inode hash of a test thread's tables without freeing it and
/// sets the counters back to 0 (`vfs_test_reset`'s, as `msdosfs_test_reset`).
#[cfg(any(test, feature = "doubles"))]
pub fn fuse_test_reset<G: ProcGlue>() {
    crate::fs::fuse::fuse_ihash::fuse_ihash_test_reset::<G>();
    crate::fs::fuse::fuse_device::fuse_device_test_reset::<G>();
}
/* </CODE> */
