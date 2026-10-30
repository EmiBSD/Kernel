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
 * Copyright (c) 1990, 1993
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
 *	@(#)specdev.h	8.3 (Berkeley) 8/10/94
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
 *	@(#)spec_vnops.c	8.8 (Berkeley) 11/21/94
 */
/*
 * Copyright (c) 2006 Pedro Martelletto <pedro@ambientworks.net>
 * Copyright (c) 2006 Thordur Bjornsson <thib@openbsd.org>
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
//! Special files: the information kept about a device vnode (`struct specinfo`,
//! `<sys/specdev.h>`), the hash of device vnodes (`speclisth`), and their operations
//! (`spec_vops`, `spec_vnops.c`), which pass open, close, read, write and ioctl to the
//! device's driver through the device switch and clone `D_CLONE` character devices.
//!
//! Upstream: sys/sys/specdev.h @ 3ce1f3f79392
//! Upstream: sys/kern/spec_vnops.c @ 3ce1f3f79392
//! LZ: sys/sys/specdev.rs@3c62ede68802
//! LZ: sys/kern/spec_vnops.rs@3c62ede68802
//!
//! ## Deviations
//! - Block-device reads and writes go through the buffer cache as in C; the block size
//!   comes from the disk label in the C (`DIOCGPART` through `bdevsw[].d_ioctl`): `bsd` has
//!   `struct partinfo` and `DIOCGPART`, but the ioctl through the device switch with a
//!   kernel buffer for it is N7's (the device switch's move), so it stays a visible gap
//!   (`unported!`) for a configured major and the size is `BLKDEV_IOSIZE`, as when the C's
//!   ioctl fails.
//!
//! ## Redesign
//! - `struct specinfo`, `malloc(M_VNODE)`ed by `checkalias` and freed by `vgonel`, reached
//!   through `v_specinfo` -> [`Specinfo<G>`] owned by its vnode (`v_un`'s `MBox`, freed when
//!   `vgonel` takes it back); its words atomics and links, the advisory locks (`si_lockf`,
//!   `bsd`'s `lockf` until N5.3) the vnode's glue member. `si_hashchain` is the bucket's
//!   number. The clone bitmap a device's aliases share (`si_ci.ci_bitmap`, freed once by the
//!   last alias in the C's protocol) is a counted pool item, "clonemap" (D-N5.2-3, the
//!   user's: `vmstat -m` gains the row, `M_VNODE` loses the 128 bytes), dropped by the last
//!   alias that holds it; its bits are atomics, set and cleared under the kernel lock as the
//!   C's `setbit`/`clrbit`.
//! - `spec_open_clone`'s `struct cloneinfo` in the parent's `v_data` (`M_TEMP`, read back
//!   and freed by `vn_open`) -> the clone recorded in the parent's specinfo
//!   ([`clone_restore`] takes it back): the parent's node word is never overwritten, so
//!   nothing between the two can read a cloneinfo as a node. The allocation that no longer
//!   goes through `malloc(9)` is `spec_open_clone`'s `malloc(sizeof(struct cloneinfo),
//!   M_TEMP, M_WAITOK)` with `vn_open`'s `free(cip, M_TEMP, sizeof(*cip))`: one 16-byte
//!   `M_TEMP` malloc and free per open of a `D_CLONE` device; what replaces it is the
//!   specinfo's `clone` link, one word of the parent's specinfo (`M_VNODE`, allocated once
//!   per device vnode by `checkalias`, not per open). So `vmstat -m`'s `temp` row counts one
//!   request fewer per clone open (`ks_calls`; its in-use and memory figures are the same
//!   after the open, the C's block being freed within it): the user accepted it on
//!   2026-10-29, beside D-N5.2-2's recycled mounts and D-N5.2-3's "clonemap" row; nothing
//!   else userland sees changes.
//! - The device switch (`cdevsw[]`, `bdevsw[]`, `machine::conf` in `bsd`) is the bridge
//!   [`Devsw`] (decision 45, paid by N7's device switch); a buffer passes to the driver's
//!   strategy as it came ([`IoBuf`]).
//! - `speclisth` is a leaf mutex at `IPL_NONE` the C does not have (it covers the chains
//!   with the kernel lock, D-N5.2-13); each walk that sleeps (`checkalias`, `vcount`, the
//!   revoke of an alias) finds under it, leaves it and restarts from the head after the
//!   sleep, as the C's `goto loop`.

use crate::file::LockOwner;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU8, Ordering};

use hal::link::Link;
use hal::list::ListEntry;
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FWRITE, Flock};
use hal::sys::mount::{MNT_NODEV, MNT_WAIT};
use hal::sys::syslimits::{LINK_MAX, MAX_CANON, MAX_INPUT};
use hal::sys::types::{Daddr, Dev, Register, major, makedev, minor};
use hal::sys::unistd::{
    _PC_CHOWN_RESTRICTED, _PC_LINK_MAX, _PC_MAX_CANON, _PC_MAX_INPUT, _PC_TIMESTAMP_RESOLUTION,
    _PC_VDISABLE, _POSIX_VDISABLE,
};

use super::bio::{self, DirtyPick, DirtyScan, NODEV};
use super::buf::{B_BUSY, B_DELWRI, Buf};
use super::default;
use super::mount::Mount;
use super::subr::{cdevvp, vcount, vfinddev, vfs, vfs_mountedon, vinvalbuf, vput, vrele};
use super::vnode::{
    LK_EXCLUSIVE, LK_RETRY, V_SAVE, VBAD, VBLK, VCHR, VCLONE, VCLONED, VDIR, VFIFO, VISTTY, VLNK,
    VNODE_MTX, VNON, VREG, VSOCK, VXLOCK, Vattr, Vnode, stat, vn_lock,
};
use super::vops::{
    Cn, Cred, IoBuf, Vnops, vop_access, vop_getattr, vop_islocked, vop_setattr, vop_unlock,
};
use crate::assert::kpanic;
use crate::event::{__EV_POLL, __EV_SELECT, Knote, seltrue_kqfilter};
use crate::file::FileGlue;
use crate::proc::{ProcGlue, Thread};
use crate::uio::{Uio, UioRw};

/// `CLONE_SHIFT`: we use the upper 16 bits of the minor to record the clone instance. This
/// gives us 8 bits for encoding the real minor number.
pub const CLONE_SHIFT: u32 = 8;
/// `CLONE_MAPSZ`: the bytes of a cloning device's bitmap.
pub const CLONE_MAPSZ: usize = 128;
/// `SPECHSZ`: buckets of the special device hash.
pub const SPECHSZ: usize = 64;

/// `D_DISK` (`<sys/conf.h>`): a disk.
pub const D_DISK: u32 = 1;
/// `D_TTY`: a terminal.
pub const D_TTY: u32 = 2;
/// `D_CLONE`: clone upon open.
pub const D_CLONE: u32 = 0x0001;

/// `BLKDEV_IOSIZE`: I/O size for block devices (`<sys/param.h>`, `PAGE_SIZE`).
const BLKDEV_IOSIZE: i32 = hal::sys::param::PAGE_SIZE as i32;
/// `DEV_BSHIFT`: log2(DEV_BSIZE).
const DEV_BSHIFT: u32 = 9;

/// `btodb(x)`: bytes to device blocks.
const fn btodb(x: i64) -> Daddr {
    x >> DEV_BSHIFT
}

/// The bucket number of no chain.
const NO_CHAIN: u8 = u8::MAX;

/// `SPECHASH(rdev)`: the `speclisth` bucket of a device.
pub const fn spechash(rdev: Dev) -> usize {
    ((rdev >> 5).wrapping_add(rdev) as usize) & (SPECHSZ - 1)
}

/// A cloning device's bitmap (`si_ci.ci_bitmap`): one bit per instance, shared by the
/// device's aliases.
pub type CloneBits = [core::sync::atomic::AtomicU8; CLONE_MAPSZ];
/// A counted handle to a [`CloneBits`] ("clonemap").
pub type CloneMap = PoolArc<CloneBits>;

/// `isclr(map, i)`.
fn isclr(map: &CloneBits, i: usize) -> bool {
    map[i / 8].load(Ordering::Relaxed) & (1 << (i % 8)) == 0
}

/// `setbit(map, i)`.
fn setbit(map: &CloneBits, i: usize) {
    map[i / 8].fetch_or(1 << (i % 8), Ordering::Relaxed);
}

/// `clrbit(map, i)`.
fn clrbit(map: &CloneBits, i: usize) {
    map[i / 8].fetch_and(!(1 << (i % 8)), Ordering::Relaxed);
}

/// `struct specinfo`: the information maintained about special devices. Made by
/// `checkalias`, owned by its vnode, freed by `vgonel`.
pub struct Specinfo<G: ProcGlue> {
    /// `si_hashchain`: the `speclisth` bucket the vnode is on ([`NO_CHAIN`] for none).
    hashchain: AtomicU8,
    /// `si_mountpoint`: the file system mounted from this device.
    mountpoint: Link<Mount<G>>,
    /// `si_rdev`: the device.
    rdev: AtomicI32,
    /// `si_lastr`: last read block, for read-ahead.
    lastr: AtomicI64,
    /// `si_ci.ci_parent`: pointer back to parent device (a clone).
    ci_parent: Link<Vnode<G>>,
    /// `si_ci.ci_bitmap`: bitmap of devices cloned off us (a cloning device's).
    ci_bitmap: Option<CloneMap>,
    /// The clone `spec_open_clone` made for the open in progress (`struct cloneinfo`'s
    /// `ci_vp`), until [`clone_restore`] takes it.
    clone: Link<Vnode<G>>,
}

impl<G: ProcGlue> Specinfo<G> {
    /// `checkalias`'s new specinfo of `rdev`, on the chain `bucket`.
    pub(super) fn new(rdev: Dev, bucket: usize, bitmap: Option<CloneMap>) -> Self {
        Self {
            hashchain: AtomicU8::new(bucket as u8),
            mountpoint: Link::new(),
            rdev: AtomicI32::new(rdev),
            lastr: AtomicI64::new(0),
            ci_parent: Link::new(),
            ci_bitmap: bitmap,
            clone: Link::new(),
        }
    }

    /// `si_rdev`.
    pub fn rdev(&self) -> Dev {
        self.rdev.load(Ordering::Relaxed)
    }

    /// `si_mountpoint`.
    pub fn mountpoint(&self) -> Option<&'static Mount<G>> {
        self.mountpoint.get()
    }

    /// `si_mountpoint = mp`.
    pub fn set_mountpoint(&self, mp: Option<&'static Mount<G>>) {
        self.mountpoint.set(mp);
    }

    /// `si_ci.ci_parent`.
    pub fn parent(&self) -> Option<&'static Vnode<G>> {
        self.ci_parent.get()
    }

    /// The `speclisth` bucket the vnode is on, `None` for none.
    pub(super) fn hashchain(&self) -> Option<usize> {
        match self.hashchain.load(Ordering::Relaxed) {
            NO_CHAIN => None,
            b => Some(usize::from(b)),
        }
    }

    /// The vnode left its chain.
    pub(super) fn clear_hashchain(&self) {
        self.hashchain.store(NO_CHAIN, Ordering::Relaxed);
    }

    /// A clone of the bitmap handle (an alias of a cloning device shares it).
    pub(super) fn bitmap(&self) -> Option<CloneMap> {
        self.ci_bitmap.clone()
    }
}

/// `si_specnext`'s adapter: a `speclisth` chain.
pub struct BySpecnext<G: ProcGlue>(PhantomData<G>);

impl<G: ProcGlue> hal::list::ListAdapter for BySpecnext<G> {
    type Elem = Vnode<G>;
    fn entry(e: &Vnode<G>) -> &ListEntry<Vnode<G>> {
        &e.specnext
    }
}

/// The device switch (`cdevsw[]`, `bdevsw[]`, `nchrdev`, `nblkdev`, `chrtoblk`,
/// `iskmemdev`) as the special files call it: `bsd`'s `machine::conf` tables until the
/// device switch moves (N7.2, decision 45; bridges.toml `Devsw`). A major past a table reads
/// as an empty slot, as `machine::conf` answers.
pub trait Devsw<G: ProcGlue>: Sync {
    /// `nchrdev`.
    fn nchrdev(&self) -> u32;
    /// `nblkdev`.
    fn nblkdev(&self) -> u32;
    /// `cdevsw[maj].d_type`.
    fn cdev_type(&self, maj: u32) -> u32;
    /// `cdevsw[maj].d_flags`.
    fn cdev_flags(&self, maj: u32) -> u32;
    /// `bdevsw[maj].d_type`.
    fn bdev_type(&self, maj: u32) -> u32;
    /// `(*cdevsw[major(dev)].d_open)(dev, oflags, devtype, p)`.
    fn cdev_open(&self, dev: Dev, oflags: i32, devtype: i32, p: &Thread<G>) -> Result<(), Errno>;
    /// `(*cdevsw[major(dev)].d_close)(dev, fflag, devtype, p)`.
    fn cdev_close(
        &self,
        dev: Dev,
        fflag: i32,
        devtype: i32,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno>;
    /// `(*cdevsw[major(dev)].d_read)(dev, uio, ioflag)`.
    fn cdev_read(&self, dev: Dev, uio: &mut Uio<'_, G>, ioflag: i32) -> Result<(), Errno>;
    /// `(*cdevsw[major(dev)].d_write)(dev, uio, ioflag)`.
    fn cdev_write(&self, dev: Dev, uio: &mut Uio<'_, G>, ioflag: i32) -> Result<(), Errno>;
    /// `(*cdevsw[major(dev)].d_ioctl)(dev, cmd, data, fflag, p)`.
    fn cdev_ioctl(
        &self,
        dev: Dev,
        cmd: u64,
        data: &mut [u8],
        fflag: i32,
        p: &Thread<G>,
    ) -> Result<(), Errno>;
    /// `(*cdevsw[major(dev)].d_kqfilter)(dev, kn)`, `None` when the slot is NULL.
    fn cdev_kqfilter(&self, dev: Dev, kn: &Knote<G>) -> Option<Result<(), Errno>>;
    /// `(*bdevsw[major(dev)].d_open)(dev, oflags, devtype, p)`.
    fn bdev_open(&self, dev: Dev, oflags: i32, devtype: i32, p: &Thread<G>) -> Result<(), Errno>;
    /// `(*bdevsw[major(dev)].d_close)(dev, fflag, devtype, p)`.
    fn bdev_close(
        &self,
        dev: Dev,
        fflag: i32,
        devtype: i32,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno>;
    /// `(*bdevsw[major(dev)].d_ioctl)(dev, cmd, data, fflag, p)`.
    fn bdev_ioctl(
        &self,
        dev: Dev,
        cmd: u64,
        data: &mut [u8],
        fflag: i32,
        p: &Thread<G>,
    ) -> Result<(), Errno>;
    /// `(*bdevsw[major(bp->b_dev)].d_strategy)(bp)`.
    fn bdev_strategy(&self, bp: IoBuf);
    /// `chrtoblk(dev)`: the block device of a character device, `NODEV` for none.
    fn chrtoblk(&self, dev: Dev) -> Dev;
    /// `iskmemdev(dev)`: `/dev/mem` or `/dev/kmem`.
    fn iskmemdev(&self, dev: Dev) -> bool;
    /// `iszerodev(dev)`: `/dev/zero` (`mmap(2)`'s private mapping of it).
    fn iszerodev(&self, dev: Dev) -> bool;
}

/// The device switch of `G`'s subsystem; `panic(9)` before it is registered (no device vnode
/// exists then).
pub(super) fn devsw<G: ProcGlue>() -> &'static dyn Devsw<G> {
    match vfs::<G>().devsw.get() {
        Some(d) => *d,
        None => kpanic(format_args!("the device switch is not registered")),
    }
}

/// Registers `G`'s device switch, once (`bsd`'s `hal_bridges_register`); `false` when one was.
pub fn register_devsw<G: ProcGlue>(d: &'static dyn Devsw<G>) -> bool {
    vfs::<G>().devsw.set(d).is_ok()
}

/// `spec_vops`: the operations of a special (device) vnode.
pub struct SpecVops<G: ProcGlue>(PhantomData<G>);

impl<G: ProcGlue> SpecVops<G> {
    /// The table.
    pub const TABLE: SpecVops<G> = SpecVops(PhantomData);
}

impl<G: ProcGlue> Vnops<G> for SpecVops<G> {
    fn lookup(
        &self,
        _dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        _cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_lookup(vpp)
    }
    fn create(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mknod(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn open(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        spec_open(vp, mode, cred, p)
    }
    fn close(
        &self,
        vp: &'static Vnode<G>,
        fflag: i32,
        cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        spec_close(vp, fflag, cred, p)
    }
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        spec_access(vp, mode, cred, p)
    }
    fn getattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        spec_getattr(vp, vap, cred, p)
    }
    fn setattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        spec_setattr(vp, vap, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        spec_read(vp, uio, ioflag)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        spec_write(vp, uio, ioflag)
    }
    fn ioctl(
        &self,
        vp: &'static Vnode<G>,
        command: u64,
        data: &mut [u8],
        fflag: i32,
        _cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        spec_ioctl(vp, command, data, fflag, p)
    }
    fn kqfilter(&self, vp: &'static Vnode<G>, _fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        spec_kqfilter(vp, kn)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        waitfor: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        spec_fsync(vp, waitfor)
    }
    fn remove(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn link(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rename(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
        _: &'static Vnode<G>,
        _: Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn mkdir(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn rmdir(
        &self,
        _: &'static Vnode<G>,
        _: &'static Vnode<G>,
        _: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn symlink(
        &self,
        _: &'static Vnode<G>,
        _: &mut Option<&'static Vnode<G>>,
        _: &mut Cn<'_, G>,
        _: &mut Vattr,
        _: &[u8],
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn readdir(
        &self,
        _: &'static Vnode<G>,
        _: &mut Uio<'_, G>,
        _: Cred<'_>,
        _: &mut i32,
    ) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn readlink(&self, _: &'static Vnode<G>, _: &mut Uio<'_, G>, _: Cred<'_>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn abortop(&self, _: &'static Vnode<G>, _: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_badop()
    }
    fn inactive(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        spec_inactive(vp)
    }
    fn reclaim(&self, _vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn lock(&self, _vp: &'static Vnode<G>, _flags: i32) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn unlock(&self, _vp: &'static Vnode<G>) -> Result<(), Errno> {
        Ok(()) // nullop
    }
    fn islocked(&self, _vp: &'static Vnode<G>) -> i32 {
        0 // nullop
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        default::vop_generic_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, _vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        spec_strategy::<G>(bp)
    }
    fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        spec_print(vp)
    }
    fn pathconf(
        &self,
        _vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        spec_pathconf(name, retval)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        spec_advlock(vp, id, op, fl, flags)
    }
    fn bwrite(&self, bp: IoBuf) -> Result<(), Errno> {
        default::vop_generic_bwrite(bp)
    }
}

/// The securelevel `spec_open` checks (`kern.securelevel`).
fn securelevel() -> i32 {
    crate::sysctl::SECURELEVEL.load(Ordering::Relaxed)
}

/// Open a special file.
pub fn spec_open<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let dev = vp.rdev();
    let maj = major(dev);
    let sw = devsw::<G>();

    // Don't allow open if fs is mounted -nodev.
    if vp.mount().is_some_and(|mp| mp.flag() & MNT_NODEV != 0) {
        return Err(Errno::ENXIO);
    }

    match vp.vtype() {
        VCHR => {
            if maj >= sw.nchrdev() {
                return Err(Errno::ENXIO);
            }
            if !cred.is_fscred() && mode & FWRITE != 0 {
                // When running in very secure mode, do not allow opens for writing of any
                // disk character devices.
                if securelevel() >= 2 && sw.cdev_type(maj) == D_DISK {
                    return Err(Errno::EPERM);
                }
                // When running in secure mode, do not allow opens for writing of /dev/mem,
                // /dev/kmem, or character devices whose corresponding block devices are
                // currently mounted.
                if securelevel() >= 1 {
                    let bdev = sw.chrtoblk(dev);
                    if bdev != NODEV
                        && let Some(bvp) = vfinddev::<G>(bdev, VBLK)
                        && bvp.usecount() > 0
                    {
                        vfs_mountedon(bvp)?;
                    }
                    if sw.iskmemdev(dev) {
                        return Err(Errno::EPERM);
                    }
                }
            }
            if sw.cdev_type(maj) == D_TTY {
                vp.flag_set(VISTTY);
            }
            if sw.cdev_flags(maj) & D_CLONE != 0 {
                return spec_open_clone(vp, mode, p);
            }
            let _ = vop_unlock(vp);
            let error = sw.cdev_open(dev, mode, stat::S_IFCHR as i32, p);
            let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
            error
        }

        VBLK => {
            if maj >= sw.nblkdev() {
                return Err(Errno::ENXIO);
            }
            // When running in very secure mode, do not allow opens for writing of any disk
            // block devices.
            if securelevel() >= 2
                && !cred.is_fscred()
                && mode & FWRITE != 0
                && sw.bdev_type(maj) == D_DISK
            {
                return Err(Errno::EPERM);
            }
            // Do not allow opens of block devices that are currently mounted.
            vfs_mountedon(vp)?;
            sw.bdev_open(dev, mode, stat::S_IFBLK as i32, p)
        }
        VNON | VLNK | VDIR | VREG | VBAD | VFIFO | VSOCK => Ok(()),
    }
}

/// Vnode op for read.
pub fn spec_read<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if uio.uio_rw != UioRw::UIO_READ {
        kpanic(format_args!("spec_read mode"));
    }
    #[cfg(not(feature = "diagnostic"))]
    let _ = UioRw::UIO_READ;
    if uio.uio_resid == 0 {
        return Ok(());
    }

    match vp.vtype() {
        VCHR => {
            let _ = vop_unlock(vp);
            let dev = vp.rdev();
            let error = devsw::<G>().cdev_read(dev, uio, ioflag);
            let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
            error
        }

        VBLK => {
            if uio.uio_offset < 0 {
                return Err(Errno::EINVAL);
            }
            let bsize = spec_bsize(vp);
            let bscale = btodb(i64::from(bsize));
            loop {
                let bn = btodb(uio.uio_offset) & !(bscale - 1);
                let on = (uio.uio_offset % i64::from(bsize)) as usize;
                let mut n = (bsize as usize - on).min(uio.uio_resid);
                let lastr = vp
                    .with_specinfo(|si| si.lastr.load(Ordering::Relaxed))
                    .unwrap_or_else(|| kpanic(format_args!("spec_read: no specinfo")));
                let (mut bp, error) = if lastr + bscale == bn {
                    let nextbn = bn + bscale;
                    bio::breadn(&vp.bufs, bn, bsize, &[nextbn], &[bsize])
                } else {
                    bio::bread(&vp.bufs, bn, bsize)
                };
                let _ = vp.with_specinfo(|si| si.lastr.store(bn, Ordering::Relaxed));
                n = n.min(bsize as usize - bp.buf().resid());
                if let Err(error) = error {
                    bio::brelse(bp);
                    return Err(error);
                }
                let error = <G::FileGlue as FileGlue>::uiomove(&mut bp.data()[on..on + n], uio);
                bio::brelse(bp);
                error?;
                if uio.uio_resid == 0 || n == 0 {
                    return Ok(());
                }
            }
        }

        _ => kpanic(format_args!("spec_read type")),
    }
}

/// `spec_inactive`: nothing to do but unlock.
pub fn spec_inactive<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let _ = vop_unlock(vp);
    Ok(())
}

/// Vnode op for write.
pub fn spec_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if uio.uio_rw != UioRw::UIO_WRITE {
        kpanic(format_args!("spec_write mode"));
    }

    match vp.vtype() {
        VCHR => {
            let _ = vop_unlock(vp);
            let dev = vp.rdev();
            let error = devsw::<G>().cdev_write(dev, uio, ioflag);
            let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
            error
        }

        VBLK => {
            if uio.uio_resid == 0 {
                return Ok(());
            }
            if uio.uio_offset < 0 {
                return Err(Errno::EINVAL);
            }
            let bsize = spec_bsize(vp);
            let bscale = btodb(i64::from(bsize));
            loop {
                let bn = btodb(uio.uio_offset) & !(bscale - 1);
                let on = (uio.uio_offset % i64::from(bsize)) as usize;
                let mut n = (bsize as usize - on).min(uio.uio_resid);
                let (mut bp, error) = bio::bread(&vp.bufs, bn, bsize);
                n = n.min(bsize as usize - bp.buf().resid());
                if let Err(error) = error {
                    bio::brelse(bp);
                    return Err(error);
                }
                let error = <G::FileGlue as FileGlue>::uiomove(&mut bp.data()[on..on + n], uio);
                if n + on == bsize as usize {
                    bio::bawrite(bp);
                } else {
                    bio::bdwrite(bp);
                }
                error?;
                if uio.uio_resid == 0 || n == 0 {
                    return Ok(());
                }
            }
        }

        _ => kpanic(format_args!("spec_write type")),
    }
}

/// The block size of a block device's I/O: `BLKDEV_IOSIZE`, or the FFS block of its
/// partition that `DIOCGPART` reports (see the module's deviations).
fn spec_bsize<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    if major(vp.rdev()) < devsw::<G>().nblkdev() {
        // (*bdevsw[majordev].d_ioctl)(vp->v_rdev, DIOCGPART, &dpart, FREAD, p): N7's ioctl
        // path through the device switch. Without it the C's ioctl failure path:
        // BLKDEV_IOSIZE.
        let _ = crate::prf::unported!("spec_read/spec_write: DIOCGPART (disklabel.h)");
    }
    BLKDEV_IOSIZE
}

/// Device ioctl operation.
pub fn spec_ioctl<G: ProcGlue>(
    vp: &'static Vnode<G>,
    command: u64,
    data: &mut [u8],
    fflag: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let dev = vp.rdev();
    match vp.vtype() {
        VCHR => devsw::<G>().cdev_ioctl(dev, command, data, fflag, p),
        VBLK => devsw::<G>().bdev_ioctl(dev, command, data, fflag, p),
        _ => kpanic(format_args!("spec_ioctl")),
    }
}

/// `spec_kqfilter`: the driver's `d_kqfilter`, or `seltrue_kqfilter` for poll/select.
pub fn spec_kqfilter<G: ProcGlue>(vp: &'static Vnode<G>, kn: &Knote<G>) -> Result<(), Errno> {
    let dev = vp.rdev();
    match vp.vtype() {
        VCHR => match devsw::<G>().cdev_kqfilter(dev, kn) {
            Some(r) => r,
            None => Err(Errno::EOPNOTSUPP),
        },
        _ if kn.has_flags(__EV_POLL | __EV_SELECT) => seltrue_kqfilter(kn),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// Synch buffers associated with a block device.
pub fn spec_fsync<G: ProcGlue>(vp: &'static Vnode<G>, waitfor: i32) -> Result<(), Errno> {
    if vp.vtype() == VCHR {
        return Ok(());
    }
    // Flush all dirty buffers associated with a block device.
    loop {
        // loop: the cache's scan of the dirty list (from its head after each write).
        let pick = |bp: &Buf| {
            if bp.isset(B_BUSY) {
                return DirtyPick::Skip;
            }
            if !bp.isset(B_DELWRI) {
                kpanic(format_args!("spec_fsync: not dirty"));
            }
            DirtyPick::Take
        };
        if let DirtyScan::Taken(t) =
            bio::vnode_dirty_scan(&vp.bufs, "spec_fsync", 0, hal::sleep::INFSLP, pick)
        {
            bio::bawrite(t);
            continue;
        }
        if waitfor == MNT_WAIT {
            let _ = bio::vwaitforio(&vp.bufs, 0, "spec_fsync", hal::sleep::INFSLP);

            #[cfg(feature = "diagnostic")]
            if !vp.bufs.dirty_is_empty() {
                super::subr::vprint(Some("spec_fsync: dirty"), vp);
                continue;
            }
        }
        return Ok(());
    }
}

/// `spec_strategy`: `bdevsw[major(bp->b_dev)].d_strategy(bp)`.
pub fn spec_strategy<G: ProcGlue>(bp: IoBuf) -> Result<(), Errno> {
    devsw::<G>().bdev_strategy(bp);
    Ok(())
}

/// Device close routine.
pub fn spec_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fflag: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let dev = vp.rdev();
    let sw = devsw::<G>();
    let mut clone = false;

    let xlocked = {
        let g = VNODE_MTX.lock();
        vp.vlock.get(&g).lflag & VXLOCK != 0
    };

    match vp.vtype() {
        VCHR => {
            // Hack: a tty device that is a controlling terminal has a reference from the
            // session structure. We cannot easily tell that a character device is a
            // controlling terminal, unless it is the closing process' controlling terminal.
            // In that case, if the reference count is 2 (this last descriptor plus the
            // session), release the reference from the session.
            if vcount(vp) == 2
                && let Some(p) = p
                && <G::FileGlue as FileGlue>::session_ttyvp_take(p, vp)
            {
                let _ = vrele(vp);
            }
            if sw.cdev_flags(major(dev)) & D_CLONE != 0 {
                clone = true;
            } else {
                // If the vnode is locked, then we are in the midst of forcibly closing the
                // device, otherwise we only close on last reference.
                if vcount(vp) > 1 && !xlocked {
                    return Ok(());
                }
            }
        }

        VBLK => {
            // On last close of a block device (that isn't mounted) we must invalidate any in
            // core blocks, so that we can, for instance, change floppy disks. In order to do
            // that, we must lock the vnode. If we are coming from vclean(), the vnode is
            // already locked.
            if !xlocked {
                let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
            }
            let error = vinvalbuf(vp, V_SAVE, cred, p, 0, hal::sleep::INFSLP);
            if !xlocked {
                let _ = vop_unlock(vp);
            }
            error?;
            // We do not want to really close the device if it is still in use unless we are
            // trying to close it forcibly. Since every use (buffer, vnode, swap, cmap) holds a
            // reference to the vnode, and because we mark any other vnodes that alias this
            // device, when the sum of the reference counts on all the aliased vnodes descends
            // to one, we are on last close.
            if vcount(vp) > 1 && !xlocked {
                return Ok(());
            }
        }

        _ => kpanic(format_args!("spec_close: not special")),
    }

    // release lock if held and this isn't coming from vclean()
    let relock = vop_islocked(vp) != 0 && !xlocked;
    if relock {
        let _ = vop_unlock(vp);
    }
    let error = if vp.vtype() == VCHR {
        sw.cdev_close(dev, fflag, stat::S_IFCHR as i32, p)
    } else {
        sw.bdev_close(dev, fflag, stat::S_IFBLK as i32, p)
    };
    if relock {
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    }

    if error.is_ok()
        && clone
        && let Some(pvp) = vp.specparent()
    {
        // get parent device
        let _ = pvp.with_specinfo(|psi| {
            if let Some(map) = &psi.ci_bitmap {
                clrbit(map, (minor(dev) >> CLONE_SHIFT) as usize);
            }
        });
        let _ = vrele(pvp);
    }

    error
}

/// `spec_getattr`: a clone answers with its parent's attributes; other device vnodes have
/// none of their own.
pub fn spec_getattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if vp.flag() & VCLONE == 0 {
        return Err(Errno::EBADF);
    }
    let Some(parent) = vp.specparent() else {
        return Err(Errno::EBADF);
    };

    let _ = vn_lock(parent, LK_EXCLUSIVE | LK_RETRY);
    let error = vop_getattr(parent, vap, cred, p);
    let _ = vop_unlock(parent);

    error
}

/// `spec_setattr`: a clone sets its parent's attributes.
pub fn spec_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if vp.flag() & VCLONE == 0 {
        return Err(Errno::EBADF);
    }
    let Some(parent) = vp.specparent() else {
        return Err(Errno::EBADF);
    };

    let _ = vn_lock(parent, LK_EXCLUSIVE | LK_RETRY);
    let error = vop_setattr(parent, vap, cred, p);
    let _ = vop_unlock(parent);

    error
}

/// `spec_access`: a clone checks its parent's permissions.
pub fn spec_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if vp.flag() & VCLONE == 0 {
        return Err(Errno::EBADF);
    }
    let Some(parent) = vp.specparent() else {
        return Err(Errno::EBADF);
    };

    let _ = vn_lock(parent, LK_EXCLUSIVE | LK_RETRY);
    let error = vop_access(parent, mode, cred, p);
    let _ = vop_unlock(parent);

    error
}

/// Print out the contents of a special device vnode.
pub fn spec_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if cfg!(any(feature = "debug", feature = "diagnostic")) {
        crate::prf::printf(format_args!(
            "tag VT_NON, dev {}, {}\n",
            major(vp.rdev()),
            minor(vp.rdev())
        ));
    }
    Ok(())
}

/// Return POSIX pathconf information applicable to special devices.
#[allow(clippy::match_same_arms, reason = "the C keeps the cases apart")]
pub fn spec_pathconf(name: i32, retval: &mut Register) -> Result<(), Errno> {
    *retval = match name {
        _PC_LINK_MAX => LINK_MAX as Register,
        _PC_MAX_CANON => MAX_CANON as Register,
        _PC_MAX_INPUT => MAX_INPUT as Register,
        _PC_CHOWN_RESTRICTED => 1,
        _PC_VDISABLE => Register::from(_POSIX_VDISABLE),
        _PC_TIMESTAMP_RESOLUTION => 1,
        _ => return Err(Errno::EINVAL),
    };

    Ok(())
}

/// Special device advisory byte-level locks (`lf_advlock(&vp->v_speclockf, 0, ...)`: the
/// device's lock list on the vnode, `Vnode::speclockf`).
pub fn spec_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    if !vp.has_specinfo() {
        kpanic(format_args!("spec_advlock: no specinfo"));
    }
    super::lockf::lf_advlock::<G>(&vp.speclockf, 0, id, op, fl, flags)
}

/// `spec_open_clone(ap)`: opens a new instance of a `D_CLONE` character device: a new vnode
/// whose minor carries the instance number, recorded in the parent's bitmap; the parent is
/// marked `VCLONED` and remembers the clone for [`clone_restore`].
pub fn spec_open_clone<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let rdev = vp.rdev();

    // CLONE_DEBUG: not configured.

    if minor(rdev) >= 1 << CLONE_SHIFT {
        return Err(Errno::ENXIO);
    }

    let Some(map) = vp.with_specinfo(Specinfo::bitmap).flatten() else {
        return Err(Errno::ENXIO);
    };
    let nbits = CLONE_MAPSZ * u8::BITS as usize;
    let mut i = 1;
    while i < nbits {
        if isclr(&map, i) {
            setbit(&map, i);
            break;
        }
        i += 1;
    }

    if i == nbits {
        return Err(Errno::EBUSY); // too many open instances
    }

    let Ok(Some(cvp)) = cdevvp::<G>(makedev(
        major(rdev),
        ((i as u32) << CLONE_SHIFT) | minor(rdev),
    )) else {
        clrbit(&map, i);
        return Err(Errno::ENFILE); // out of vnodes
    };

    let _ = vop_unlock(vp);

    let error = devsw::<G>().cdev_open(cvp.rdev(), mode, stat::S_IFCHR as i32, p);

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);

    if let Err(error) = error {
        vput(cvp);
        clrbit(&map, i);
        return Err(error); // device open failed
    }

    cvp.flag_set(VCLONE);

    let _ = cvp.with_specinfo(|si| si.ci_parent.set(Some(vp)));
    let _ = vp.with_specinfo(|si| si.clone.set(Some(cvp)));
    vp.flag_set(VCLONED);

    Ok(()) // device cloned
}

/// `vn_open`'s restore after `VOP_OPEN` of a `VCLONED` vnode: the parent loses `VCLONED`
/// and the clone it recorded, which is returned (the open goes on with it). `None` for a
/// vnode that was not cloned.
pub fn clone_restore<G: ProcGlue>(vp: &'static Vnode<G>) -> Option<&'static Vnode<G>> {
    if vp.flag() & VCLONED == 0 {
        return None;
    }
    vp.flag_clr(VCLONED);
    let ci_vp = vp.with_specinfo(|si| {
        let c = si.clone.get();
        si.clone.set(None);
        c
    });
    match ci_vp.flatten() {
        Some(c) => Some(c),
        None => kpanic(format_args!(
            "vn_open: VCLONED vnode {vp:p} without a clone"
        )),
    }
}

// SPECHASH's mask needs a power of two.
const _: () = assert!(SPECHSZ.is_power_of_two());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::subr::getnewvnode_id;
    use crate::fs::testfs::{TestGlue, setup_root};
    use crate::fs::vnode::{VT_NON, VopsId};

    #[test]
    fn a_cloning_device_s_aliases_share_its_bitmap() {
        let (_g, _mp) = setup_root();
        let dev = makedev(3, 0);
        let parent = cdevvp::<TestGlue>(dev).unwrap().unwrap();
        let map = parent
            .with_specinfo(Specinfo::bitmap)
            .flatten()
            .expect("a clone bitmap");
        let alias = getnewvnode_id::<TestGlue>(VT_NON, None, VopsId::SPEC).unwrap();
        alias.set_vtype(VCHR);
        assert!(super::super::subr::checkalias(alias, dev, None).is_none());
        let amap = alias
            .with_specinfo(Specinfo::bitmap)
            .flatten()
            .expect("the alias's");
        assert!(PoolArc::ptr_eq(&map, &amap));
        setbit(&map, 5);
        assert!(!isclr(&amap, 5));
        clrbit(&amap, 5);
        assert!(isclr(&map, 5));
        // A clone (an instance in the minor's upper bits) gets none.
        let clone = cdevvp::<TestGlue>(makedev(3, 1 << CLONE_SHIFT))
            .unwrap()
            .unwrap();
        assert!(clone.with_specinfo(Specinfo::bitmap).flatten().is_none());
    }

    #[test]
    fn clone_restore_takes_back_the_recorded_clone_once() {
        let (_g, _mp) = setup_root();
        let parent = cdevvp::<TestGlue>(makedev(3, 0)).unwrap().unwrap();
        assert!(clone_restore(parent).is_none(), "not VCLONED");
        let clone = cdevvp::<TestGlue>(makedev(3, 2 << CLONE_SHIFT))
            .unwrap()
            .unwrap();
        let _ = parent.with_specinfo(|si| si.clone.set(Some(clone)));
        parent.flag_set(VCLONED);
        assert!(clone_restore(parent).is_some_and(|c| core::ptr::eq(c, clone)));
        assert_eq!(parent.flag() & VCLONED, 0);
        assert!(clone_restore(parent).is_none());
    }

    #[test]
    fn spechash_is_the_c_s() {
        assert_eq!(spechash(0), 0);
        assert_eq!(
            spechash(makedev(3, 0)),
            ((makedev(3, 0) >> 5) + makedev(3, 0)) as usize & 63
        );
        let mut r = 0;
        assert_eq!(spec_pathconf(_PC_LINK_MAX, &mut r), Ok(()));
        assert_eq!(r, LINK_MAX as Register);
        assert_eq!(spec_pathconf(-1, &mut r), Err(Errno::EINVAL));
    }
}
/* </TESTS> */
