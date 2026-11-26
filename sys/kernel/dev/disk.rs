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
 * Copyright (c) 1995 Jason R. Thorpe.  All rights reserved.
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)ufs_disksubr.c	8.5 (Berkeley) 1/21/94
 */

/*
 * Copyright (c) 1995 Jason R. Thorpe.  All rights reserved.
 * Copyright (c) 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This software was developed by the Computer Systems Engineering group
 * at Lawrence Berkeley Laboratory under DARPA contract BG 91-66 and
 * contributed to Berkeley.
 *
 * All advertising materials mentioning features or use of this software
 * must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Lawrence Berkeley Laboratory.
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
 * from: Header: disk.h,v 1.5 92/11/19 04:33:03 torek Exp  (LBL)
 *
 *	@(#)disk.h	8.1 (Berkeley) 6/2/93
 */

/*-
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * William Jolitz.
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
 *	@(#)swapgeneric.c	5.5 (Berkeley) 5/9/91
 */
/* </LICENSES> */

/* <CODE> */
//! The disk layer: `struct disk` and the list of attached disks (`disk_attach`,
//! `disk_detach`, `disk_openpart`, the statistics of `disk_busy` and
//! `disk_unbusy`), the label code (`dkcksum`, `initdisklabel`, `checkdisklabel`,
//! `readdoslabel` and its GPT, MBR and FAT spoofs, `setdisklabel`), the transfer bounds
//! (`bounds_check_with_label`), `diskerr`, `disk_readlabel`, `disk_map` (the DUID lookup of
//! `mount(2)` and of `opendev(3)`'s `DIOCMAP`), and the root, swap and dump choice (`setroot`,
//! `getdisk`, `parsedisk`, `dk_mountroot`) with `swapgeneric.c`'s `rootdev`, `dumpdev`,
//! `swdevt[]` and `mountroot`.
//!
//! Upstream: sys/kern/subr_disk.c @ 3ce1f3f79392
//! Upstream: sys/sys/disk.h @ 3ce1f3f79392
//! Upstream: sys/conf/swapgeneric.c @ 3ce1f3f79392
//! LZ: sys/kern/subr_disk.rs@e8249449d905
//! LZ: sys/sys/disk.rs@e8249449d905
//! LZ: sys/conf/swapgeneric.rs@e8249449d905
//!
//! The disk drivers still in `bsd` (sd, cd: N7.4b; rd, vnd, softraid: N7.7; wd, fd: N10)
//! reach this module through one shim, `bsd`'s `sys/sys/disk.rs`: a `DiskSlot` in each
//! softc holds the driver's [`DiskRef`].
//!
//! ## Deviations
//! - `gpt_get_hdr` returns the header (zeroed when invalid) and `gpt_get_parts` the
//!   partition array as an `Option<MArray<u8>>` (the C's `mallocarray(M_DEVBUF, M_NOWAIT |
//!   M_ZERO)` `*gp`, `None` on a bad checksum); the array is read entry by entry with
//!   `GptPartition::from_bytes`. The checksums are `crc32` (`libz`).
//! - `checkdisklabel` takes the raw label and the in-core label as two `&mut`: the C's
//!   `lp != dlp` test is always true for its callers (the raw label sits in a sector buffer),
//!   so the copy is unconditional.
//! - `readdoslabel`'s `daddr_t *partoffp` is an `Option<&mut Daddr>`, its `spoofonly` a
//!   `bool`; the temporary label the C mallocs is a local copy.
//! - `bounds_check_with_label` returns `true` where the C returns 0 (go on with the
//!   transfer) and `false` for -1 (the buffer is finished: an error or end of partition).
//! - `disk_readlabel` returns `Err(DiskReadlabelError)`, whose `Display` is the C's message,
//!   where the C fills `errbuf` and returns it.
//! - `duid_format` returns the 16 hex digits by value instead of a static buffer.
//! - `disk_map` returns `true` where the C returns 0 and `false` for -1; `mappath` is a
//!   separate buffer (the C lets `path` and `mappath` be the same array; a Rust caller
//!   copies).
//! - `diskerr` formats its line into a buffer and prints it with one `printf` (or one
//!   `log(pri, ..)` for the C's `log(pri, "")` and `addlog`s): the same bytes reach the
//!   console and the message buffer.
//! - `DEBUG`'s `DPRINTF`s are not configured.
//! - `dk_state`, `dk_blkshift` and `dk_byteshift`, which no driver of the tree reads, are not
//!   members of [`Disk`].
//! - `setroot`'s `RB_ASKNAME` dialogue (the "root device:" and "swap device:" prompts read
//!   with `getsn` under `cnpollc`) is reported and skipped (LZ's gap: its text blames
//!   `getsn`; the pin is frozen). A kernel booted with `-a` goes on with its configured root.
//!   `NFSCLIENT` (feature `nfsclient`): the `nfs_mountroot` branches are ported; `swapdev =
//!   NODEV` (a constant of `conf.c` here) is not assigned, nothing reads it before `swdevt[0]`
//!   is.
//! - `dk_mountroot`: `EXT2FS` (feature `ext2fs`) and `CD9660` (feature `cd9660`) are the file
//!   systems with a mountroot beside FFS, which the kernel always has (LZ's `!FFS` panic arm
//!   has no configuration here).
//! - `parsedisk` returns the device and the `dev_t` as an `Option` pair instead of filling
//!   `*devp`; `getdisk` likewise.
//!
//! ## Redesign
//! - N7.1d: `struct disk` was LZ's `Disk` of `Cell`s embedded in each zeroed softc, with a
//!   `Rwlock` and a `Mutex` whose level `disk_construct` set, its label a raw pointer to a
//!   `malloc`'d block reached through `label()`, `with_label` and an `unsafe fn label_mut`, the
//!   list an intrusive TAILQ of `&'static Disk`s (the C's "XXX: Assumes dk is part of the
//!   device softc"). It is a counted block now, [`DiskRef`] (`hal::mem::malloc::MArc`):
//!   [`Disk::new`] makes it (`disk_construct`, with the name and flags the C sets before the
//!   attach); its lock is a [`RawRwlock`] (`disk_lock`, `disk_lock_nointr` are
//!   [`Disk::lock`], [`Disk::lock_nointr`], whose guard's drop is `disk_unlock`), its
//!   statistics a [`Mutexed`] at `IPL_BIO` (the C's `dk_mtx`, the same one mutex and level in
//!   `disk_busy` and `disk_unbusy`), its other members atomics.
//! - The in-core label is 147 relaxed words in the block (the inode's `DinWords` shape over
//!   `fs::ufs::ufs::inode`'s word helpers): [`Disk::label_store`] writes them,
//!   [`Disk::label`] copies them out, and the I/O path reads only the members it needs
//!   ([`Disk::geom`]: a [`LabelGeom`], at most six words, no lock, as the C's plain loads).
//!   Writers hold `dk_lock` or the kernel lock (the drivers' ioctls and opens), as the C's;
//!   a reader racing a store sees old and new words mixed, as the C's reader races `*lp =
//!   *nlp`. `disk_attach`'s two `malloc`s (the label, the task record) are gone: the block
//!   holds both.
//! - `disklist` is a `PinList` of the blocks under [`DISKS`] (a `Mutexed<()>` at
//!   `IPL_NONE`: the C's kernel lock, still asserted in `disk_attach` and `disk_detach`),
//!   held for one list step at a time, never across the softraid hook, a sleep or `dk_mtx`.
//!   The list's place holds a handle of each attached disk, parked on the disk itself
//!   (`dk_self`, the fuse(4) device's shape) and taken back by `disk_detach`; a walk
//!   ([`disk_next`], `TAILQ_NEXT`) hands out a handle per step, O(1) a step. The label task
//!   holds one until it has run (`Task::set_shared`), so a disk detached meanwhile stays
//!   alive for it, which the C's `device_ref` of the softc did. `dk_device` is a counted
//!   `DevRef`, given back by `disk_detach`.
//! - The label code's `bp` and `strat` pair is a [`LabelBuf`]: the C's `geteblk` buffer as
//!   its token (`fs::buf::BufRef`) and the driver's strategy as a [`Strat`] closure, which
//!   starts the transfer and waits for it (`(*strat)(bp); biowait(bp)`); its drop is the C's
//!   `B_INVAL` and `brelse`. No `&'static Buf` and no `unsafe` view of `b_data`: the bytes are
//!   the token's. `bsd`'s drivers make the closure out of their strategy routine (the shim).
//! - `softraid_disk_attach` (softraid callback, do not use!) is a registered `fn`
//!   ([`softraid_hooks`]), with `sr_map_root` beside it, instead of a `bsd` import: the
//!   kernel names nothing of `bsd`.
//! - The root choice (`setroot`, `dk_mountroot`) is the kernel's with `swapgeneric.c`'s
//!   statics: `rootdev`, `dumpdev`, `swdevt[]` atomics, `mountroot` a [`Mountroot`] (the C's
//!   function pointer compared with `nfs_mountroot`'s address), `bootduid`/`rootduid` one word
//!   each, `rootdv` a counted `DevRef`; the machine's `nam2blk[]`, `swapdev` and `boothowto
//!   |= RB_ASKNAME` come through `ConfigGlue`.
//! - LZ's 22 `unsafe` of `subr_disk.rs` and 7 of `sys/disk.rs` are gone (the module has
//!   none).
//! - The label words are read and written through `fs::ufs::ufs::inode`'s word helpers
//!   (`words_get`, `words_read_bytes`, `words_write_bytes`), the inode's `DinWords` core: a
//!   neutral home for them (`fs::word`) is a later step's, not this one's.

use core::fmt::{self, Write as _};
use core::mem::offset_of;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU64, Ordering};
use hal::sysbytes::DisklabelBytes;
use hal::user::AbiBytes;

use hal::cpu::intr::{IPL_BIO, IPL_NONE};
use hal::ds::pinlist::{PinEntry, PinList};
use hal::mem::malloc::{M_NOWAIT, MArc, MArray};
use hal::mem::pool::AnyArc;
use hal::random::{arc4random_buf, enqueue_randomness};
use hal::sync::mutex::Mutexed;
use hal::sync::once::OnceStatic;
use hal::sync::rwlock::{RW_INTR, RawRwlock, WriteWitness};
use hal::sync::sleep::tsleep_nsec;
use hal::sync::slot::ReadMostly;
use hal::sys::disk::{DKF_CONSTRUCTED, DKF_NOLABELREAD, DKF_OPENED, DM_OPENBLCK, DM_OPENPART};
use hal::sys::disk::{DS_DISKNAMELEN, Diskstats};
use hal::sys::disklabel::{
    DISKLABEL_SIZE, DISKMAGIC, DOS_LABELSECTOR, DOS_MAXEBR, DOSBBSECTOR, DOSMBR_SIGNATURE,
    DOSMBR_SIGNATURE_OFF, DOSPTYP_EFI, DOSPTYP_EFISYS, DOSPTYP_EXTEND, DOSPTYP_EXTENDL,
    DOSPTYP_FAT12, DOSPTYP_FAT16B, DOSPTYP_FAT16L, DOSPTYP_FAT16S, DOSPTYP_FAT32, DOSPTYP_FAT32L,
    DOSPTYP_LINUX, DOSPTYP_NTFS, DOSPTYP_OPENBSD, DOSPTYP_UNUSED, Disklabel, DosPartition, FS_BOOT,
    FS_BSDFFS, FS_EXT2FS, FS_HFS, FS_MSDOS, FS_NTFS, FS_OTHER, FS_UNUSED, GPTMINHDRSIZE,
    GPTMINPARTSIZE, GPTREVISION, GPTSECTOR, GPTSIGNATURE, GptHeader, GptPartition, MAXDISKSIZE,
    MAXPARTITIONS, MAXPARTITIONSUNIT, NDOSPART, NSPARE, Partition, RAW_PART, diskpart, diskunit,
    dl_blkoffset, dl_blktosec, dl_getbend, dl_getbstart, dl_getdsize, dl_getpoffset, dl_getpsize,
    dl_partname2num, dl_partnum2name, dl_sectoblk, dl_setbend, dl_setbstart, dl_setdsize,
    dl_setpoffset, dl_setpsize, makediskdev,
};
use hal::sys::dkio::DIOCGDINFO;
use hal::sys::fcntl::FREAD;
use hal::sys::net_if::IfTimeval;
use hal::sys::reboot::RB_ASKNAME;
use hal::sys::stat::{S_IFBLK, S_IFCHR};
use hal::sys::types::{Daddr, Dev, major, makedev, minor};
use hal::sys::uuid::Uuid;
use libz::crc32;

use crate::assert::kpanic;
use crate::base::log::LOG_PRINTF;
use crate::base::prf::unported;
use crate::base::prf::{Str, printf};
use crate::base::task::{SYSTQ, Task, task_add};
use crate::dev::autoconf::{
    alldevs_next, config_boothowto, config_boothowto_or, config_nam2blk, config_swapdev,
};
use crate::dev::device::{DV_DISK, DevRef, Device};
use crate::fs::bio::{brelse, geteblk};
use crate::fs::buf::{
    B_BUSY, B_DONE, B_ERROR, B_INVAL, B_RAW, B_READ, B_WRITE, Buf, BufRef, MAXPHYS,
};
use crate::fs::subr::{cdevvp, vput};
use crate::fs::ufs::ufs::inode::{words_get, words_read_bytes, words_write_bytes};
use crate::fs::vops::{Cred, vop_close, vop_ioctl, vop_open};
use crate::proc::ProcGlue;
use crate::sys::errno::Errno;
use crate::time::clock::{Timeval, sec_to_nsec};
use crate::time::tc::microuptime;

/// `DUID_SIZE`.
pub const DUID_SIZE: usize = 8;

/// `DEV_BSIZE` (`<sys/param.h>`).
const DEV_BSIZE: usize = 512;
/// `DEV_BSHIFT` (`<sys/param.h>`).
const DEV_BSHIFT: u32 = 9;
/// `NODEV` (`<sys/param.h>`).
const NODEV: Dev = -1;
/// `M_DEVBUF` (`<sys/malloc.h>`): the disk's block and the GPT partition array.
const M_DEVBUF: i32 = 2;

/// The in-core label's words: `sizeof(struct disklabel)` rounded up to 8 bytes.
const LABEL_WORDS: usize = DISKLABEL_SIZE.div_ceil(8);

/// The name the word helpers give the label in their `panic(9)` messages.
const LABEL: &str = "disklabel";

/// Why `disk_readlabel` failed; `Display` writes the C's `errbuf` message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskReadlabelError {
    /// `cannot obtain vnode for 0x%x/0x%x`.
    NoVnode {
        /// The device asked for.
        dev: Dev,
        /// Its raw character partition.
        rawdev: Dev,
    },
    /// `cannot open disk, 0x%x/0x%x, error %d`.
    Open {
        /// The device asked for.
        dev: Dev,
        /// Its raw character partition.
        rawdev: Dev,
        /// The open's error.
        error: Errno,
    },
    /// `cannot read disk label, 0x%x/0x%x, error %d`.
    Ioctl {
        /// The device asked for.
        dev: Dev,
        /// Its raw character partition.
        rawdev: Dev,
        /// The `DIOCGDINFO` error.
        error: Errno,
    },
}

impl fmt::Display for DiskReadlabelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NoVnode { dev, rawdev } => {
                write!(f, "cannot obtain vnode for {dev:#x}/{rawdev:#x}")
            }
            Self::Open { dev, rawdev, error } => write!(
                f,
                "cannot open disk, {dev:#x}/{rawdev:#x}, error {}",
                error as i32
            ),
            Self::Ioctl { dev, rawdev, error } => write!(
                f,
                "cannot read disk label, {dev:#x}/{rawdev:#x}, error {}",
                error as i32
            ),
        }
    }
}

/// The metrics `dk_mtx` guards (`struct disk`'s `dk_busy` .. `dk_time`); some may have no
/// meaning on certain types of disks.
#[derive(Clone, Copy, Default)]
struct DiskStats {
    /// `dk_busy`: busy counter.
    busy: i32,
    /// `dk_rxfer`: total number of read transfers.
    rxfer: u64,
    /// `dk_wxfer`: total number of write transfers.
    wxfer: u64,
    /// `dk_seek`: total independent seek operations.
    seek: u64,
    /// `dk_rbytes`: total bytes read.
    rbytes: u64,
    /// `dk_wbytes`: total bytes written.
    wbytes: u64,
    /// `dk_attachtime`: time disk was attached.
    attachtime: Timeval,
    /// `dk_timestamp`: time of first busy or any unbusy.
    timestamp: Timeval,
    /// `dk_time`: total time spent busy.
    time: Timeval,
}

/// `struct disk`: a disk's name, device, lock, statistics, open masks and in-core label (the
/// module's redesign: a counted block, [`DiskRef`]).
pub struct Disk {
    /// `dk_link`: link in global disklist.
    dk_link: PinEntry<Disk>,
    /// The handle the list's place holds, parked here by `disk_attach` and taken back by
    /// `disk_detach` (`None` while the disk is not listed).
    dk_self: ReadMostly<Option<DiskRef>>,
    /// `dk_lock`: disk lock.
    dk_lock: RawRwlock,
    /// `dk_mtx`: busy/unbusy mtx, and the metrics it guards.
    dk_mtx: Mutexed<DiskStats>,
    /// `dk_name`: disk name, NUL padded.
    dk_name: [u8; DS_DISKNAMELEN],
    /// `dk_device`: disk device structure, set by `disk_attach`, given back by `disk_detach`.
    dk_device: ReadMostly<Option<DevRef>>,
    /// `dk_devno`: disk device number.
    dk_devno: AtomicI32,
    /// `dk_flags`: disk flags (`DKF_*`).
    dk_flags: AtomicI32,
    /// `dk_bopenmask`: block devices open.
    dk_bopenmask: AtomicU64,
    /// `dk_copenmask`: character devices open.
    dk_copenmask: AtomicU64,
    /// `dk_openmask`: composite (bopen|copen).
    dk_openmask: AtomicU64,
    /// `dk_label`: disk label information, as words (the module's redesign).
    dk_label: [AtomicU64; LABEL_WORDS],
    /// `struct disk_attach_task`'s task: the label read `disk_attach` queues on `systq`.
    dk_task: Task,
}

hal::pin_adapter!(
    /// `TAILQ_ENTRY(disk) dk_link`: `disklist`.
    pub DiskLink: Disk, dk_link
);

/// `struct disk *` held: a counted handle to a disk's block of `malloc(M_DEVBUF)`, which the
/// last handle frees.
pub type DiskRef = MArc<Disk>;

/// `disk_lock(dk)` held: its drop is `disk_unlock(dk)`.
pub type DiskLock<'a> = WriteWitness<'a>;

/// The label's geometry the I/O path reads (`d_secsize`, `d_nsectors`, `d_secpercyl` and
/// one partition's `p_offset`, `p_size`, `p_fstype`): [`Disk::geom`], a few relaxed loads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LabelGeom {
    /// `d_secsize`: bytes per sector.
    pub secsize: u32,
    /// `d_nsectors`: data sectors per track.
    pub nsectors: u32,
    /// `d_secpercyl`: data sectors per cylinder.
    pub secpercyl: u32,
    /// `DL_GETPOFFSET(&d_partitions[part])`: the partition's first sector.
    pub poffset: u64,
    /// `DL_GETPSIZE(&d_partitions[part])`: the partition's sectors.
    pub psize: u64,
    /// `d_partitions[part].p_fstype`.
    pub fstype: u8,
}

impl LabelGeom {
    /// The geometry of partition `part` of the label `lp` (the tests' reference for
    /// [`Disk::geom`]).
    #[cfg(test)]
    pub(crate) fn of(lp: &Disklabel, part: u32) -> Self {
        let p = lp
            .d_partitions
            .get(part as usize)
            .copied()
            .unwrap_or_default();
        Self {
            secsize: lp.d_secsize,
            nsectors: lp.d_nsectors,
            secpercyl: lp.d_secpercyl,
            poffset: dl_getpoffset(&p),
            psize: dl_getpsize(&p),
            fstype: p.p_fstype,
        }
    }

    /// `DL_BLKSPERSEC(d)`: `DEV_BSIZE` blocks per sector.
    pub const fn blkspersec(&self) -> u64 {
        self.secsize as u64 / DEV_BSIZE as u64
    }

    /// `DL_SECTOBLK(d, n)`: sectors to `DEV_BSIZE` blocks.
    pub const fn sectoblk(&self, n: u64) -> u64 {
        n * self.blkspersec()
    }

    /// `DL_BLKTOSEC(d, n)`: `DEV_BSIZE` blocks to sectors.
    pub const fn blktosec(&self, n: u64) -> u64 {
        n / self.blkspersec()
    }
}

impl Disk {
    /// `disk_construct` of a disk named `name` (up to `DS_DISKNAMELEN` bytes, the C's
    /// `dk_name`) with the flags `flags` (`dk_flags`) its driver sets before `disk_attach`: a
    /// fresh block, `M_NOWAIT` (attach may not sleep), `panic(9)` when there is none, as the
    /// C's label allocation.
    pub fn new(name: &[u8], flags: i32) -> DiskRef {
        let mut dk_name = [0u8; DS_DISKNAMELEN];
        let n = name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(name.len())
            .min(DS_DISKNAMELEN - 1);
        dk_name[..n].copy_from_slice(&name[..n]);
        let dk = Disk {
            dk_link: PinEntry::new(),
            dk_self: ReadMostly::new(None),
            dk_lock: RawRwlock::new("dklk"),
            dk_mtx: Mutexed::new(IPL_BIO, DiskStats::default()),
            dk_name,
            dk_device: ReadMostly::new(None),
            dk_devno: AtomicI32::new(0),
            dk_flags: AtomicI32::new(flags | DKF_CONSTRUCTED),
            dk_bopenmask: AtomicU64::new(0),
            dk_copenmask: AtomicU64::new(0),
            dk_openmask: AtomicU64::new(0),
            dk_label: [const { AtomicU64::new(0) }; LABEL_WORDS],
            dk_task: Task::unset(),
        };
        match MArc::new(M_DEVBUF, M_NOWAIT, dk) {
            Some(dk) => dk,
            None => kpanic(format_args!(
                "disk_attach: can't allocate storage for disklabel"
            )),
        }
    }

    /// `dk_name` up to its NUL.
    pub fn name(&self) -> &[u8] {
        let n = self
            .dk_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(DS_DISKNAMELEN);
        &self.dk_name[..n]
    }

    /// `dk_devno`: the disk's raw partition's block device (`NODEV` without one).
    pub fn devno(&self) -> Dev {
        self.dk_devno.load(Ordering::Relaxed)
    }

    /// `dk_device`: the disk's device, while it is attached.
    pub fn device(&self) -> Option<DevRef> {
        self.dk_device.read().clone()
    }

    /// `dk->dk_device == dv`.
    pub fn is_device(&self, dv: &Device) -> bool {
        self.dk_device
            .read()
            .as_ref()
            .is_some_and(|d| core::ptr::eq(&**d, dv))
    }

    /// Whether the disk is on `disklist` (between `disk_attach` and `disk_detach`).
    pub fn is_attached(&self) -> bool {
        self.dk_link.is_linked()
    }

    /// `dk_flags`.
    pub fn flags(&self) -> i32 {
        self.dk_flags.load(Ordering::Relaxed)
    }

    /// `dk_flags |= f`.
    pub fn flags_or(&self, f: i32) {
        self.dk_flags.fetch_or(f, Ordering::Relaxed);
    }

    /// `dk_openmask`: the partitions open (block or character).
    pub fn openmask(&self) -> u64 {
        self.dk_openmask.load(Ordering::Relaxed)
    }

    /// `dk_bopenmask`: the partitions open as block devices.
    pub fn bopenmask(&self) -> u64 {
        self.dk_bopenmask.load(Ordering::Relaxed)
    }

    /// `dk_copenmask`: the partitions open as character devices.
    pub fn copenmask(&self) -> u64 {
        self.dk_copenmask.load(Ordering::Relaxed)
    }

    /// `dk_openmask = m`.
    fn set_openmask(&self, m: u64) {
        self.dk_openmask.store(m, Ordering::Relaxed);
    }

    /// `disk_lock(dk)`: takes the disk lock, interruptibly; the guard's drop is
    /// `disk_unlock`.
    pub fn lock(&self) -> Result<DiskLock<'_>, Errno> {
        self.dk_lock.write_flags(RW_INTR)
    }

    /// `disk_lock_nointr(dk)`: takes the disk lock; the guard's drop is `disk_unlock`.
    pub fn lock_nointr(&self) -> DiskLock<'_> {
        self.dk_lock.write()
    }

    /// `*out = *dk->dk_label`: the in-core label, copied out.
    pub fn label(&self, out: &mut Disklabel) {
        words_read_bytes(&self.dk_label, LABEL, 0, out.as_bytes_mut());
    }

    /// `*dk->dk_label = *lp`: the in-core label replaced, by a holder of the kernel lock
    /// (asserted under `diagnostic`; the drivers' opens and ioctls, which take `dk_lock`
    /// too, run under it, as `disk_attach` does).
    pub fn label_store(&self, lp: &Disklabel) {
        words_write_bytes(&self.dk_label, LABEL, 0, lp.as_bytes());
    }

    /// The in-core label's address, for `DIOCGPART`'s `struct partinfo` only (`bsd`'s, N7.3c):
    /// the words are the label's bytes in memory order. A reader through it races the
    /// stores, as the C's reader of `pi.disklab` does.
    pub fn label_ptr(&self) -> *mut Disklabel {
        self.dk_label.as_ptr().cast::<Disklabel>().cast_mut()
    }

    /// `&dk->dk_label->d_partitions[part]`'s address, for `DIOCGPART` (as [`Self::label_ptr`]).
    pub fn partition_ptr(&self, part: u32) -> *mut Partition {
        self.label_ptr()
            .cast::<u8>()
            .wrapping_add(
                offset_of!(Disklabel, d_partitions)
                    + (part as usize).min(MAXPARTITIONSUNIT as usize - 1) * size_of::<Partition>(),
            )
            .cast::<Partition>()
    }

    /// `dk->dk_label->d_uid`.
    pub fn uid(&self) -> [u8; DUID_SIZE] {
        let mut uid = [0u8; DUID_SIZE];
        words_read_bytes(
            &self.dk_label,
            LABEL,
            offset_of!(Disklabel, d_uid),
            &mut uid,
        );
        uid
    }

    /// The geometry of partition `part` of the in-core label (the I/O path's loads: no lock,
    /// at most six words). A partition past the table reads as an empty one.
    pub fn geom(&self, part: u32) -> LabelGeom {
        let w = &self.dk_label;
        let mut g = LabelGeom {
            secsize: words_get(w, LABEL, offset_of!(Disklabel, d_secsize)),
            nsectors: words_get(w, LABEL, offset_of!(Disklabel, d_nsectors)),
            secpercyl: words_get(w, LABEL, offset_of!(Disklabel, d_secpercyl)),
            ..LabelGeom::default()
        };
        if part < MAXPARTITIONSUNIT {
            let p = offset_of!(Disklabel, d_partitions) + part as usize * size_of::<Partition>();
            let size: u32 = words_get(w, LABEL, p + offset_of!(Partition, p_size));
            let off: u32 = words_get(w, LABEL, p + offset_of!(Partition, p_offset));
            let offh: u16 = words_get(w, LABEL, p + offset_of!(Partition, p_offseth));
            let sizeh: u16 = words_get(w, LABEL, p + offset_of!(Partition, p_sizeh));
            g.poffset = (u64::from(offh) << 32) + u64::from(off);
            g.psize = (u64::from(sizeh) << 32) + u64::from(size);
            g.fstype = words_get(w, LABEL, p + offset_of!(Partition, p_fstype));
        }
        g
    }

    /// `d_npartitions` and `d_partitions[part].p_fstype` of the in-core label.
    fn part_exists(&self, part: u32) -> bool {
        let w = &self.dk_label;
        let n: u16 = words_get(w, LABEL, offset_of!(Disklabel, d_npartitions));
        part < u32::from(n)
            && part < MAXPARTITIONSUNIT
            && words_get::<u8>(
                w,
                LABEL,
                offset_of!(Disklabel, d_partitions)
                    + part as usize * size_of::<Partition>()
                    + offset_of!(Partition, p_fstype),
            ) != FS_UNUSED
    }

    /// The handle the list's place holds, cloned (`None` once detached).
    fn listed(&self) -> Option<DiskRef> {
        self.dk_self.read().clone()
    }
}

/// The label code's buffer: the C's `geteblk` buffer (its token) and the driver's strategy
/// routine (the module's redesign); its drop invalidates and releases the buffer
/// (`bp->b_flags |= B_INVAL; brelse(bp)`).
pub struct LabelBuf<'a> {
    /// The buffer, `None` only while the strategy holds it.
    t: Option<BufRef>,
    /// `geteblk`'s size.
    size: usize,
    /// `(*strat)(bp); biowait(bp)`.
    io: Strat<'a>,
}

/// A driver's strategy routine as the label code calls it: the busy buffer handed to the
/// driver (`(*strat)(bp)`) and waited for (`biowait(bp)`), its token given back with the
/// transfer's error.
pub type Strat<'a> = &'a mut dyn FnMut(BufRef) -> (BufRef, Result<(), Errno>);

impl<'a> LabelBuf<'a> {
    /// `bp = geteblk(size); bp->b_dev = dev;` with the driver's strategy `io`.
    pub fn new(dev: Dev, size: usize, io: Strat<'a>) -> Self {
        let t = geteblk(size);
        t.buf().set_dev(dev);
        Self {
            t: Some(t),
            size,
            io,
        }
    }

    /// The label spoofs' read of the block `blkno` (`iso_disklabelspoof`,
    /// `udf_disklabelspoof`): the whole buffer read raw by the driver's strategy (`b_blkno`,
    /// `b_bcount`, the flags, `b_resid = resid` when given), its first `out.len()` bytes
    /// copied out when the transfer succeeded.
    pub fn read_block(
        &mut self,
        blkno: Daddr,
        resid: Option<usize>,
        out: &mut [u8],
    ) -> Result<(), Errno> {
        let b = self.buf();
        b.set_blkno(blkno);
        b.set_bcount(self.size as i64);
        b.clr(B_READ | B_WRITE | B_DONE);
        b.set(B_BUSY | B_READ | B_RAW);
        if let Some(r) = resid {
            b.set_resid(r);
        }
        self.strategy()?;
        let data = self.bytes();
        let n = out.len().min(data.len());
        out[..n].copy_from_slice(&data[..n]);
        Ok(())
    }

    /// The buffer's token.
    fn tok(&self) -> &BufRef {
        match &self.t {
            Some(t) => t,
            None => kpanic(format_args!("disk: label buffer lent")),
        }
    }

    /// The buffer's header.
    pub fn buf(&self) -> &Buf {
        self.tok().buf()
    }

    /// The buffer's `b_bcount` bytes.
    pub fn bytes(&self) -> &[u8] {
        self.tok().bytes()
    }

    /// The buffer's `b_bcount` bytes, writable.
    pub fn data(&mut self) -> &mut [u8] {
        match &mut self.t {
            Some(t) => t.data(),
            None => kpanic(format_args!("disk: label buffer lent")),
        }
    }

    /// `(*strat)(bp); return biowait(bp);`: the transfer the header describes.
    pub fn strategy(&mut self) -> Result<(), Errno> {
        let Some(t) = self.t.take() else {
            kpanic(format_args!("disk: label buffer lent"));
        };
        let (t, error) = (self.io)(t);
        self.t = Some(t);
        error
    }

    /// The first `N` bytes at `off` of the buffer, copied out (zeros past its end).
    fn bytes_at<const N: usize>(&self, off: usize) -> [u8; N] {
        let mut out = [0u8; N];
        if let Some(src) = self.bytes().get(off..) {
            let n = src.len().min(N);
            out[..n].copy_from_slice(&src[..n]);
        }
        out
    }
}

impl Drop for LabelBuf<'_> {
    fn drop(&mut self) {
        if let Some(t) = self.t.take() {
            t.buf().set(B_INVAL);
            brelse(t);
        }
    }
}

/// `struct nam2blk`: a disk driver's name and its block major (the machine's `nam2blk[]`,
/// read through `ConfigGlue::nam2blk`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nam2blk {
    /// The driver's name (`"sd"`, `"wd"`, ...).
    pub name: &'static [u8],
    /// Its block major.
    pub maj: i32,
}

/// The softraid(4) callbacks (`softraid_disk_attach` and `sr_map_root`), registered by
/// `bsd`'s `sr_attach` (the module's redesign).
#[derive(Clone, Copy)]
struct SoftraidHooks {
    /// `softraid_disk_attach(dk, action)`: `1` on attach, `-1` on detach.
    attach: fn(&DiskRef, i32),
    /// `sr_map_root()`: the root disk's DUID mapped to its volume's.
    map_root: fn(),
}

/// The witness of `disklist`, `disk_count` and the disks' list handles (the C's kernel lock,
/// the module's redesign): held for one list step at a time.
static DISKS: Mutexed<()> = Mutexed::new(IPL_NONE, ());

/// `disklist`: a global list of all disks attached to the system. May grow or shrink over
/// time.
static DISKLIST: PinList<DiskLink> = PinList::new();

/// `disk_count`: number of drives in global disklist.
pub static DISK_COUNT: AtomicI32 = AtomicI32::new(0);

/// `disk_change`: set if a disk has been attached/detached since last we looked at this
/// variable. This is reset by `hw_sysctl()`.
pub static DISK_CHANGE: AtomicI32 = AtomicI32::new(0);

/// `softraid_disk_attach` (softraid callback, do not use!) and `sr_map_root`.
static SOFTRAID: OnceStatic<SoftraidHooks> = OnceStatic::new();

/// `softraid_disk_attach != NULL`: set by `sr_attach`, cleared by `sr_detach`.
static SOFTRAID_ON: AtomicBool = AtomicBool::new(false);

/// `softraid_disk_attach = sr_disk_attach` (`sr_attach`): softraid(4)'s callbacks registered
/// (the first registration's are kept: there is one softraid) and the attach hook switched
/// on.
pub fn softraid_hooks(attach: fn(&DiskRef, i32), map_root: fn()) {
    let _ = SOFTRAID.set(SoftraidHooks { attach, map_root });
    SOFTRAID_ON.store(true, Ordering::Relaxed);
}

/// `softraid_disk_attach = NULL` (`sr_detach`).
pub fn softraid_hooks_off() {
    SOFTRAID_ON.store(false, Ordering::Relaxed);
}

/// `softraid_disk_attach`, when it is set.
fn softraid_attach_hook() -> Option<fn(&DiskRef, i32)> {
    if SOFTRAID_ON.load(Ordering::Relaxed) {
        SOFTRAID.get().map(|h| h.attach)
    } else {
        None
    }
}

/// `sr_map_root()`, when softraid(4) registered it (`setroot`'s `NSOFTRAID > 0`).
pub fn sr_map_root() {
    if let Some(h) = SOFTRAID.get() {
        (h.map_root)();
    }
}

/// Compute checksum for disk label.
pub fn dkcksum(lp: &Disklabel) -> u16 {
    lp.cksum_words(usize::from(lp.d_npartitions))
        .fold(0, |sum, w| sum ^ w)
}

/// `initdisklabel`: the minimal requirements for an archetypal disk label: the raw
/// partition covers the disk, the others are empty.
pub fn initdisklabel(lp: &mut Disklabel) -> Result<(), Errno> {
    // minimal requirements for archetypal disk label
    if lp.d_secsize as usize > MAXPHYS {
        return Err(Errno::ERANGE);
    }
    if (lp.d_secsize as usize) < DEV_BSIZE {
        lp.d_secsize = DEV_BSIZE as u32;
    }
    if dl_getdsize(lp) == 0 {
        dl_setdsize(lp, MAXDISKSIZE);
    }
    if lp.d_secpercyl == 0 {
        return Err(Errno::ERANGE);
    }
    lp.d_npartitions = MAXPARTITIONS as u16;
    for p in lp.d_partitions.iter_mut().take(RAW_PART as usize) {
        dl_setpsize(p, 0);
        dl_setpoffset(p, 0);
    }
    let dsize = dl_getdsize(lp);
    let raw = &mut lp.d_partitions[RAW_PART as usize];
    if dl_getpsize(raw) == 0 {
        dl_setpsize(raw, dsize);
    }
    dl_setpoffset(raw, 0);
    dl_setbstart(lp, 0);
    dl_setbend(lp, dsize);
    lp.d_version = 1;
    Ok(())
}

/// Check an incoming block to make sure it is a disklabel, convert it to a newer version
/// if needed, etc etc. `dlp` is the label as read from the disk, `lp` the in-core label,
/// which comes in holding the real disk size and leaves holding the checked label.
pub fn checkdisklabel(
    _dev: Dev,
    dlp: &mut Disklabel,
    lp: &mut Disklabel,
    boundstart: u64,
    boundend: u64,
) -> Result<(), Errno> {
    // These fields may not be 0, no point trying a byteswap
    if dlp.d_secpercyl == 0 || dlp.d_nsectors == 0 || dlp.d_version == 0 {
        return Err(Errno::EINVAL); // invalid label
    } else if dlp.d_secsize == 0 {
        return Err(Errno::ENOSPC); // disk too small
    }

    let error = if dlp.d_magic != DISKMAGIC || dlp.d_magic2 != DISKMAGIC {
        Some(Errno::ENOENT) // no disk label
    } else if usize::from(dlp.d_npartitions) > MAXPARTITIONS {
        Some(Errno::E2BIG) // too many partitions
    } else if dkcksum(dlp) != 0 {
        Some(Errno::EINVAL) // incorrect checksum
    } else {
        None
    };

    if let Some(error) = error {
        // If it is byte-swapped, attempt to convert it
        if dlp.d_magic.swap_bytes() != DISKMAGIC
            || dlp.d_magic2.swap_bytes() != DISKMAGIC
            || usize::from(dlp.d_npartitions.swap_bytes()) > MAXPARTITIONS
        {
            return Err(error);
        }

        // Need a byte-swap aware dkcksum variant inlined, because dkcksum uses a sub-field
        let sum = dlp
            .cksum_words(usize::from(dlp.d_npartitions.swap_bytes()))
            .fold(0u16, |sum, w| sum ^ w);
        if sum != 0 {
            return Err(error);
        }

        swap_label(dlp);
        dlp.d_checksum = 0;
        dlp.d_checksum = dkcksum(dlp);
    }

    // XXX should verify lots of other fields and whine a lot

    // Initial passed in lp contains the real disk size.
    let disksize = dl_getdsize(lp);

    *lp = *dlp;

    dl_setdsize(lp, disksize);
    let raw = &mut lp.d_partitions[RAW_PART as usize];
    dl_setpsize(raw, disksize);
    dl_setpoffset(raw, 0);
    dl_setbstart(lp, boundstart);
    dl_setbend(lp, boundend.min(dl_getdsize(lp)));

    lp.d_checksum = 0;
    lp.d_checksum = dkcksum(lp);
    Ok(())
}

/// `checkdisklabel`'s conversion of a byte-swapped label: every integer member swapped, the
/// strings (`d_typename`, `d_packname`, `d_uid`) left as they are.
fn swap_label(dlp: &mut Disklabel) {
    dlp.d_magic = dlp.d_magic.swap_bytes();
    dlp.d_type = dlp.d_type.swap_bytes();

    // d_typename and d_packname are strings

    dlp.d_secsize = dlp.d_secsize.swap_bytes();
    dlp.d_nsectors = dlp.d_nsectors.swap_bytes();
    dlp.d_ntracks = dlp.d_ntracks.swap_bytes();
    dlp.d_ncylinders = dlp.d_ncylinders.swap_bytes();
    dlp.d_secpercyl = dlp.d_secpercyl.swap_bytes();
    dlp.d_secperunit = dlp.d_secperunit.swap_bytes();

    // d_uid is a string

    dlp.d_acylinders = dlp.d_acylinders.swap_bytes();

    dlp.d_flags = dlp.d_flags.swap_bytes();

    dlp.d_secperunith = dlp.d_secperunith.swap_bytes();
    dlp.d_version = dlp.d_version.swap_bytes();

    for s in dlp.d_spare.iter_mut().take(NSPARE) {
        *s = s.swap_bytes();
    }

    dlp.d_magic2 = dlp.d_magic2.swap_bytes();

    dlp.d_npartitions = dlp.d_npartitions.swap_bytes();

    for pp in dlp.d_partitions.iter_mut().take(MAXPARTITIONS) {
        pp.p_size = pp.p_size.swap_bytes();
        pp.p_offset = pp.p_offset.swap_bytes();
        pp.p_offseth = pp.p_offseth.swap_bytes();
        pp.p_sizeh = pp.p_sizeh.swap_bytes();
        pp.p_cpg = pp.p_cpg.swap_bytes();
    }
}

/// Read a disk sector.
pub fn readdisksector(bp: &mut LabelBuf<'_>, lp: &Disklabel, sector: u64) -> Result<(), Errno> {
    let b = bp.buf();
    b.set_blkno(dl_sectoblk(lp, sector) as Daddr);
    b.set_bcount(i64::from(lp.d_secsize));
    b.set_error(None);
    b.clr(B_READ | B_WRITE | B_DONE | B_ERROR);
    b.set(B_BUSY | B_READ | B_RAW);

    bp.strategy()
}

/// `readdoslabel`: reads the DOS boot block and spoofs a label from its MBR (or GPT, or
/// FAT boot sector), then reads the disklabel where the spoofed label says it is. With
/// `partoffp`, only finds the label's block (for `writedisklabel`) and leaves `lp` alone.
pub fn readdoslabel(
    bp: &mut LabelBuf<'_>,
    lp: &mut Disklabel,
    partoffp: Option<&mut Daddr>,
    spoofonly: bool,
) -> Result<(), Errno> {
    readdisksector(bp, lp, DOSBBSECTOR)?;
    let dosbb: [u8; DEV_BSIZE] = bp.bytes_at(0);

    let mut nlp = *lp;
    nlp.d_partitions = [Partition::new(); _];
    nlp.d_partitions[RAW_PART as usize] = lp.d_partitions[RAW_PART as usize];
    nlp.d_magic = 0;

    let mut partoff: Daddr = 0;
    spoofgpt(bp, &dosbb, &mut nlp, &mut partoff)?;
    if nlp.d_magic != DISKMAGIC {
        spoofmbr(bp, &dosbb, &mut nlp, &mut partoff);
    }
    if nlp.d_magic != DISKMAGIC {
        spooffat(&dosbb, &mut nlp, &mut partoff);
    }
    if nlp.d_magic != DISKMAGIC {
        // readdoslabel: N/A -- label partition @ daddr_t 0 (default)
        partoff = 0;
    }

    if let Some(partoffp) = partoffp {
        // If a non-zero value is returned writedisklabel() exits with EIO. If 0 is
        // returned the label sector is read from disk and lp is copied into it. So leave
        // lp alone!
        if partoff == -1 {
            return Err(Errno::ENXIO);
        }
        *partoffp = partoff;
        return Ok(());
    }

    nlp.d_magic = lp.d_magic;
    *lp = nlp;

    lp.d_checksum = 0;
    lp.d_checksum = dkcksum(lp);

    if spoofonly || partoff == -1 {
        return Ok(());
    }

    partoff += DOS_LABELSECTOR;
    if readdisksector(bp, lp, dl_blktosec(lp, partoff as u64)).is_err() {
        // return bp->b_error: 0 (success, the spoofed label) when the strategy set no error.
        return match bp.buf().error() {
            Some(e) => Err(e),
            None => Ok(()),
        };
    }

    let mut rlp = Disklabel::from_bytes(
        &bp.bytes_at::<DISKLABEL_SIZE>(dl_blkoffset(lp, partoff as u64) as usize),
    );
    let (bstart, bend) = (dl_getbstart(&rlp), dl_getbend(&rlp));
    checkdisklabel(bp.buf().dev(), &mut rlp, lp, bstart, bend)
}

/// Return the index into `dp[]` of the EFI GPT (0xEE) partition, or `None` if no such
/// partition exists.
pub fn gpt_chk_mbr(dp: &[DosPartition; NDOSPART], dsize: u64) -> Option<usize> {
    let mut found = 0;
    let mut efi = 0;
    let mut eficnt = 0;
    for (i, dp2) in dp.iter().enumerate() {
        if dp2.dp_typ == DOSPTYP_UNUSED {
            continue;
        }
        found += 1;
        if dp2.dp_typ != DOSPTYP_EFI {
            continue;
        }
        if u64::from(u32::from_le(dp2.dp_start)) != GPTSECTOR {
            continue;
        }
        let psize = u32::from_le(dp2.dp_size);
        if u64::from(psize) <= dsize.wrapping_sub(GPTSECTOR) || psize == u32::MAX {
            efi = i;
            eficnt += 1;
        }
    }
    if found == 1 && eficnt == 1 {
        Some(efi)
    } else {
        None
    }
}

/// `gpt_get_hdr`: reads the GPT header at `sector` and returns it, or a zeroed header when
/// it is not a valid one (wrong signature, revision, sizes or checksum).
pub fn gpt_get_hdr(bp: &mut LabelBuf<'_>, lp: &Disklabel, sector: u64) -> Result<GptHeader, Errno> {
    readdisksector(bp, lp, sector)?;

    let mut raw: [u8; GPTMINHDRSIZE as usize] = bp.bytes_at(0);
    let ngh = GptHeader::from_bytes(&raw);

    let size = u32::from_le(ngh.gh_size);
    let partsize = u32::from_le(ngh.gh_part_size);
    let lbaend = u64::from_le(ngh.gh_lba_end);
    let lbastart = u64::from_le(ngh.gh_lba_start);

    let csum = ngh.gh_csum;
    raw[GptHeader::CSUM_OFF..GptHeader::CSUM_OFF + 4].fill(0);
    let ncsum = crc32(0, &raw).to_le();

    if u64::from_le(ngh.gh_sig) == GPTSIGNATURE
        && u32::from_le(ngh.gh_rev) == GPTREVISION
        && size == GPTMINHDRSIZE
        && lbastart <= lbaend
        && partsize == GPTMINPARTSIZE
        && lp.d_secsize.is_multiple_of(partsize)
        && csum == ncsum
    {
        Ok(GptHeader {
            gh_csum: ncsum,
            ..ngh
        })
    } else {
        Ok(GptHeader::default())
    }
}

/// `gpt_get_parts`: reads and checks the partition array `gh` points at; `None` (the C's
/// NULL `*gp`) when its checksum does not match.
pub fn gpt_get_parts(
    bp: &mut LabelBuf<'_>,
    lp: &Disklabel,
    gh: &GptHeader,
) -> Result<Option<MArray<u8>>, Errno> {
    let partlba = u64::from_le(gh.gh_part_lba);
    let partnum = u32::from_le(gh.gh_part_num);
    let partsize = u32::from_le(gh.gh_part_size);
    let secsize = u64::from(lp.d_secsize);

    let sectors = (u64::from(partnum) * u64::from(partsize)).div_ceil(secsize);

    // mallocarray(sectors, d_secsize, M_DEVBUF, M_NOWAIT | M_ZERO)
    let bytes = sectors
        .checked_mul(secsize)
        .and_then(|b| usize::try_from(b).ok())
        .ok_or(Errno::ENOMEM)?;
    let Some(mut ngp) = MArray::new(M_DEVBUF, M_NOWAIT, bytes, |_| 0u8) else {
        return Err(Errno::ENOMEM);
    };

    for i in 0..sectors {
        readdisksector(bp, lp, partlba + i)?;
        let off = (i * secsize) as usize;
        let data = bp.bytes();
        let n = data.len().min(secsize as usize);
        ngp[off..off + n].copy_from_slice(&data[..n]);
    }

    let len = (partnum as usize) * (partsize as usize);
    let partcsum = crc32(0, &ngp[..len.min(bytes)]).to_le();
    if partcsum != gh.gh_part_csum {
        // DEBUG: DPRINTF("invalid %s GPT partition array @ %llu\n", ...): not configured.
        return Ok(None);
    }
    Ok(Some(ngp))
}

/// The GPT partition type GUIDs `gpt_get_fstype` knows, in memory order (LE format!), with
/// their file system types.
const KNOWNFS: [([u8; 16], u8); 8] = [
    // GPT_UUID_UNUSED
    ([0; 16], FS_UNUSED),
    // GPT_LEUUID_OPENBSD
    (
        [
            0xa0, 0xc7, 0x4c, 0x82, 0xa8, 0x36, 0xe3, 0x11, 0x89, 0x0a, 0x95, 0x25, 0x19, 0xad,
            0x3f, 0x61,
        ],
        FS_BSDFFS,
    ),
    // GPT_UUID_MICROSOFT_BASIC_DATA
    (
        [
            0xa2, 0xa0, 0xd0, 0xeb, 0xe5, 0xb9, 0x33, 0x44, 0x87, 0xc0, 0x68, 0xb6, 0xb7, 0x26,
            0x99, 0xc7,
        ],
        FS_MSDOS,
    ),
    // GPT_UUID_CHROMEOS_ROOTFS
    (
        [
            0x02, 0xe2, 0xb8, 0x3c, 0x7e, 0x3b, 0xdd, 0x47, 0x8a, 0x3c, 0x7f, 0xf2, 0xa1, 0x3c,
            0xfc, 0xec,
        ],
        FS_EXT2FS,
    ),
    // GPT_UUID_LINUX_FILES
    (
        [
            0xaf, 0x3d, 0xc6, 0x0f, 0x83, 0x84, 0x72, 0x47, 0x8e, 0x79, 0x3d, 0x69, 0xd8, 0x47,
            0x7d, 0xe4,
        ],
        FS_EXT2FS,
    ),
    // GPT_UUID_MAC_OS_X_HFS
    (
        [
            0x00, 0x53, 0x46, 0x48, 0x00, 0x00, 0xaa, 0x11, 0xaa, 0x11, 0x00, 0x30, 0x65, 0x43,
            0xec, 0xac,
        ],
        FS_HFS,
    ),
    // GPT_LEUUID_EFI_SYSTEM
    (
        [
            0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e,
            0xc9, 0x3b,
        ],
        FS_MSDOS,
    ),
    // GPT_UUID_BIOS_BOOT
    (
        [
            0x48, 0x61, 0x68, 0x21, 0x49, 0x64, 0x6f, 0x6e, 0x74, 0x4e, 0x65, 0x65, 0x64, 0x45,
            0x46, 0x49,
        ],
        FS_BOOT,
    ),
];

/// `gpt_get_fstype`: the file system type of a GPT partition type GUID.
pub fn gpt_get_fstype(uuid_part: &Uuid) -> u8 {
    let b = uuid_part.as_bytes();
    KNOWNFS
        .iter()
        .find(|(gptype, _)| *gptype == b)
        .map_or(FS_OTHER, |&(_, fstype)| fstype)
}

/// `spoofgpt`: spoof a label from a GPT: the OpenBSD partition's place (where the real
/// label is) and the other known partitions as `i`, `j`, ...
pub fn spoofgpt(
    bp: &mut LabelBuf<'_>,
    dosbb: &[u8; DEV_BSIZE],
    lp: &mut Disklabel,
    partoffp: &mut Daddr,
) -> Result<(), Errno> {
    let dp = DosPartition::table(dosbb);
    let sig = u16::from_ne_bytes([dosbb[DOSMBR_SIGNATURE_OFF], dosbb[DOSMBR_SIGNATURE_OFF + 1]]);

    if u16::from_le(sig) != DOSMBR_SIGNATURE || gpt_chk_mbr(&dp, dl_getdsize(lp)).is_none() {
        return Ok(());
    }

    let mut gp = None;
    let mut gh = gpt_get_hdr(bp, lp, GPTSECTOR);
    if let Ok(h) = &gh
        && u64::from_le(h.gh_sig) == GPTSIGNATURE
    {
        match gpt_get_parts(bp, lp, h) {
            Ok(parts) => gp = parts,
            Err(e) => gh = Err(e),
        }
    }

    let primary_bad = match &gh {
        Err(_) => true,
        Ok(h) => u64::from_le(h.gh_sig) != GPTSIGNATURE || gp.is_none(),
    };
    if primary_bad {
        gh = gpt_get_hdr(bp, lp, dl_getdsize(lp) - 1);
        if let Ok(h) = &gh
            && u64::from_le(h.gh_sig) == GPTSIGNATURE
        {
            match gpt_get_parts(bp, lp, h) {
                Ok(parts) => gp = parts,
                Err(e) => gh = Err(e),
            }
        }
    }

    let gh = gh?;
    let Some(gp) = gp else {
        return Err(Errno::ENXIO);
    };

    let lbastart = u64::from_le(gh.gh_lba_start);
    let lbaend = u64::from_le(gh.gh_lba_end);
    let partnum = u32::from_le(gh.gh_part_num);

    let mut n = (b'i' - b'a') as usize; // Start spoofing at 'i', a.k.a. 8.

    dl_setbstart(lp, lbastart);
    dl_setbend(lp, lbaend + 1);
    let mut partoff = dl_sectoblk(lp, lbastart) as Daddr;
    let mut obsdfound = false;
    for i in 0..partnum as usize {
        let off = i * GPTMINPARTSIZE as usize;
        let Some(entry) = gp.get(off..off + GPTMINPARTSIZE as usize) else {
            break;
        };
        let mut e = [0u8; GPTMINPARTSIZE as usize];
        e.copy_from_slice(entry);
        let part = GptPartition::from_bytes(&e);

        let fstype = gpt_get_fstype(&part.gp_type);
        if fstype == FS_UNUSED {
            continue;
        }
        if fstype == FS_OTHER {
            // DEBUG: "spoofgpt: Skipping partition %u (unknown filesystem)".
            continue;
        }

        let start = u64::from_le(part.gp_lba_start);
        if start > lbaend || start < lbastart {
            continue;
        }

        let end = u64::from_le(part.gp_lba_end);
        if start > end {
            continue;
        }

        if obsdfound && fstype == FS_BSDFFS {
            continue;
        }

        if fstype == FS_BSDFFS {
            obsdfound = true;
            partoff = dl_sectoblk(lp, start) as Daddr;
            let labelsec = dl_blktosec(lp, (partoff + DOS_LABELSECTOR) as u64);
            if labelsec > end.min(lbaend) {
                partoff = -1;
            }
            dl_setbstart(lp, start);
            dl_setbend(lp, end + 1);
            continue;
        }

        if partoff != -1 {
            let labelsec = dl_blktosec(lp, (partoff + DOS_LABELSECTOR) as u64);
            if labelsec >= start && labelsec <= end {
                partoff = -1;
            }
        }

        if n < MAXPARTITIONS && end <= lbaend {
            let pp = &mut lp.d_partitions[n];
            n += 1;
            pp.p_fstype = fstype;
            dl_setpoffset(pp, start);
            dl_setpsize(pp, end - start + 1);
        }
    }

    lp.d_magic = DISKMAGIC;
    *partoffp = partoff;
    // free(gp, M_DEVBUF, gpbytes): the array is dropped here.

    // DEBUG: the "readdoslabel: GPT -- ..." print is not configured.
    Ok(())
}

/// `mbr_get_fstype`: the file system type of an MBR partition type.
pub fn mbr_get_fstype(dp_typ: u8) -> u8 {
    match dp_typ {
        DOSPTYP_OPENBSD => FS_BSDFFS,
        DOSPTYP_UNUSED => FS_UNUSED,
        DOSPTYP_LINUX => FS_EXT2FS,
        DOSPTYP_NTFS => FS_NTFS,
        DOSPTYP_EFISYS | DOSPTYP_FAT12 | DOSPTYP_FAT16S | DOSPTYP_FAT16B | DOSPTYP_FAT16L
        | DOSPTYP_FAT32 | DOSPTYP_FAT32L => FS_MSDOS,
        // DOSPTYP_EFI, DOSPTYP_EXTEND, DOSPTYP_EXTENDL and the rest
        _ => FS_OTHER,
    }
}

/// `spoofmbr`: spoofs partitions `i`.. from an MBR and its extended boot records, and
/// finds where the OpenBSD label lives (the A6 partition, else free space at sector 1).
pub fn spoofmbr(
    bp: &mut LabelBuf<'_>,
    dosbb: &[u8; DEV_BSIZE],
    lp: &mut Disklabel,
    partoffp: &mut Daddr,
) {
    let sig = u16::from_ne_bytes([dosbb[DOSMBR_SIGNATURE_OFF], dosbb[DOSMBR_SIGNATURE_OFF + 1]]);
    if u16::from_le(sig) != DOSMBR_SIGNATURE {
        return;
    }
    let mut dp = DosPartition::table(dosbb);

    let mut sector: u64 = DOSBBSECTOR;
    let mut obsdfound = false;
    let mut partoff: Daddr = 0;
    let mut parts = 0u32;
    let mut n = usize::from(b'i' - b'a');
    let mut wander = true;
    let mut ebr = 0;
    let mut extoff: u32 = 0;
    while wander && ebr < DOS_MAXEBR {
        ebr += 1;
        wander = false;
        if sector < u64::from(extoff) {
            sector = u64::from(extoff);
        }

        if sector != DOSBBSECTOR {
            if readdisksector(bp, lp, sector).is_err() {
                break;
            }
            let ebrbb: [u8; DEV_BSIZE] = bp.bytes_at(0);
            let sig =
                u16::from_ne_bytes([ebrbb[DOSMBR_SIGNATURE_OFF], ebrbb[DOSMBR_SIGNATURE_OFF + 1]]);
            if u16::from_le(sig) != DOSMBR_SIGNATURE {
                break;
            }
            dp = DosPartition::table(&ebrbb);
        }

        for d in dp {
            if u32::from_le(d.dp_size) == 0 {
                continue;
            }
            if obsdfound && d.dp_typ == DOSPTYP_OPENBSD {
                continue;
            }

            if d.dp_typ != DOSPTYP_OPENBSD {
                if u64::from(u32::from_le(d.dp_start)) > dl_getdsize(lp) {
                    continue;
                }
                if u64::from(u32::from_le(d.dp_size)) > dl_getdsize(lp) {
                    continue;
                }
            }

            let start = sector + u64::from(u32::from_le(d.dp_start));
            let end = start + u64::from(u32::from_le(d.dp_size));

            parts += 1;
            if !obsdfound {
                let labeloff = partoff + DOS_LABELSECTOR;
                if labeloff >= dl_sectoblk(lp, start) as Daddr
                    && labeloff < dl_sectoblk(lp, end) as Daddr
                {
                    partoff = -1;
                }
            }

            match d.dp_typ {
                DOSPTYP_OPENBSD => {
                    obsdfound = true;
                    partoff = dl_sectoblk(lp, start) as Daddr;
                    let labeloff = partoff + DOS_LABELSECTOR;
                    if labeloff >= dl_sectoblk(lp, end) as Daddr {
                        partoff = -1;
                    }
                    dl_setbstart(lp, start);
                    dl_setbend(lp, end);
                    continue;
                }
                DOSPTYP_EFI => continue,
                DOSPTYP_EXTEND | DOSPTYP_EXTENDL => {
                    sector = start + u64::from(extoff);
                    if extoff == 0 {
                        extoff = start as u32;
                        sector = 0;
                    }
                    wander = true;
                    continue;
                }
                _ => {}
            }

            let fstype = mbr_get_fstype(d.dp_typ);
            if n < MAXPARTITIONS {
                let pp = &mut lp.d_partitions[n];
                n += 1;
                pp.p_fstype = fstype;
                if start != 0 {
                    dl_setpoffset(pp, start);
                }
                dl_setpsize(pp, end - start);
            }
        }
    }

    if parts > 0 {
        lp.d_magic = DISKMAGIC;
        *partoffp = partoff;
    }
}

/// `spooffat`: a disk that is one FAT file system gets it as partition `i`, and no label.
pub fn spooffat(dosbb: &[u8; DEV_BSIZE], lp: &mut Disklabel, partoffp: &mut Daddr) {
    let secsize = u16::from_le_bytes([dosbb[11], dosbb[12]]);

    let valid_jmp = (dosbb[0] == 0xeb && dosbb[2] == 0x90) || dosbb[0] == 0xe9;
    let valid_fat = dosbb[16] == 1 || dosbb[16] == 2;
    let valid_sec =
        usize::from(secsize) >= DEV_BSIZE && secsize <= 4096 && secsize.is_multiple_of(512);

    if valid_jmp && valid_sec && valid_fat {
        let i = usize::from(b'i' - b'a');
        lp.d_partitions[i] = lp.d_partitions[RAW_PART as usize];
        lp.d_partitions[i].p_fstype = FS_MSDOS;
        *partoffp = -1;
        lp.d_magic = DISKMAGIC;
    }
}

/// Check new disk label for sensibility before setting it.
pub fn setdisklabel(olp: &mut Disklabel, nlp: &mut Disklabel, openmask: u64) -> Result<(), Errno> {
    // sanity clause
    if nlp.d_secpercyl == 0
        || nlp.d_secsize == 0
        || nlp.d_secsize as usize > MAXPHYS
        || !(nlp.d_secsize as usize).is_multiple_of(DEV_BSIZE)
    {
        return Err(Errno::EINVAL);
    }

    // special case to allow disklabel to be invalidated
    if nlp.d_magic == 0xffff_ffff {
        *olp = *nlp;
        return Ok(());
    }

    if nlp.d_magic != DISKMAGIC || nlp.d_magic2 != DISKMAGIC {
        return Err(Errno::ENOENT);
    } else if usize::from(nlp.d_npartitions) > MAXPARTITIONS {
        return Err(Errno::E2BIG);
    } else if dkcksum(nlp) != 0 {
        return Err(Errno::EINVAL);
    }

    // XXX missing check if other dos partitions will be overwritten

    for i in 0..MAXPARTITIONS {
        let opp = olp.d_partitions[i];
        let npp = &mut nlp.d_partitions[i];
        if openmask & (1u64 << i) != 0
            && (dl_getpoffset(npp) != dl_getpoffset(&opp) || dl_getpsize(npp) < dl_getpsize(&opp))
        {
            return Err(Errno::EBUSY);
        }
        // Copy internally-set partition information if new label doesn't include it. XXX
        if npp.p_fstype == FS_UNUSED && opp.p_fstype != FS_UNUSED {
            npp.p_fragblock = opp.p_fragblock;
            npp.p_cpg = opp.p_cpg;
        }
    }

    // Generate a UID if the disklabel does not already have one.
    if duid_iszero(&nlp.d_uid) {
        loop {
            arc4random_buf(&mut nlp.d_uid);
            if !duid_iszero(&nlp.d_uid) && !duid_taken(&nlp.d_uid) {
                break;
            }
        }
    }

    // Preserve the disk size and RAW_PART values.
    dl_setdsize(nlp, dl_getdsize(olp));
    let dsize = dl_getdsize(nlp);
    let npp = &mut nlp.d_partitions[RAW_PART as usize];
    dl_setpoffset(npp, 0);
    dl_setpsize(npp, dsize);

    nlp.d_checksum = 0;
    nlp.d_checksum = dkcksum(nlp);
    *olp = *nlp;

    DISK_CHANGE.store(1, Ordering::Relaxed);

    Ok(())
}

/// `setdisklabel`'s walk of `disklist`: whether an attached disk's label carries `uid`.
fn duid_taken(uid: &[u8; DUID_SIZE]) -> bool {
    let mut d = disk_next(None);
    while let Some(dk) = d {
        if dk.uid() == *uid {
            return true;
        }
        d = disk_next(Some(&dk));
    }
    false
}

/// Determine the size of the transfer, and make sure it is within the boundaries of the
/// partition. Adjust transfer if needed, and signal errors or early completion: `false`
/// means the buffer is finished (`b_resid` set, and `B_ERROR` on an error). `g` is the
/// geometry of the buffer's partition ([`Disk::geom`] of `DISKPART(bp->b_dev)`).
pub fn bounds_check_with_label(bp: &Buf, g: &LabelGeom) -> bool {
    let blkno = bp.blkno();
    let bcount = bp.bcount();

    let ok = 'check: {
        // Avoid division by zero, negative offsets, and negative sizes.
        if g.secpercyl == 0 || blkno < 0 || bcount < 0 {
            break 'check Err(());
        }

        // Ensure transfer is a whole number of aligned sectors.
        if !(blkno as u64).is_multiple_of(g.blkspersec()) || bcount % i64::from(g.secsize) != 0 {
            break 'check Err(());
        }

        // Ensure transfer starts within partition boundary.
        let partblocks = g.sectoblk(g.psize) as Daddr;
        if blkno > partblocks {
            break 'check Err(());
        }

        // If exactly at end of partition or null transfer, return EOF.
        if blkno == partblocks || bcount == 0 {
            break 'check Ok(false);
        }

        // Truncate request if it extends past the end of the partition.
        let mut sz = bcount >> DEV_BSHIFT;
        if sz > partblocks - blkno {
            sz = partblocks - blkno;
            bp.set_bcount(sz << DEV_BSHIFT);
        }

        Ok(true)
    };

    match ok {
        Ok(true) => return true,
        Ok(false) => {}
        Err(()) => {
            bp.set_error(Some(Errno::EINVAL));
            bp.set(B_ERROR);
        }
    }
    bp.set_resid(usize::try_from(bp.bcount()).unwrap_or(0));
    false
}

/// A line of at most `N` bytes formatted in place (the C's `snprintf` into a stack buffer),
/// truncated past its end.
struct LineBuf<const N: usize> {
    /// The bytes written.
    buf: [u8; N],
    /// How many.
    len: usize,
}

impl<const N: usize> LineBuf<N> {
    /// Empty.
    const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    /// The bytes written.
    fn bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

impl<const N: usize> fmt::Write for LineBuf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let n = s.len().min(N - self.len);
        self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

/// Disk error is the preface to plaintive error messages about failing disk transfers. It
/// prints messages of the form
///
/// `hp0g: hard error reading fsbn 12345 of 12344-12347 (hp0 bn %d cn %d tn %d sn %d)`
///
/// if the offset of the error in the transfer and a disk label are both available.
/// `blkdone` should be -1 if the position of the error is unknown; the disklabel may be
/// absent from drivers that have not been converted to use them (`g` is the geometry of the
/// buffer's partition). The message is printed with printf if `pri` is `LOG_PRINTF`,
/// otherwise it uses log at the specified priority. The message should be completed (with
/// at least a newline) with printf or addlog, respectively. There is no trailing space.
pub fn diskerr(bp: &Buf, dname: &str, what: &str, pri: i32, blkdone: i32, g: Option<&LabelGeom>) {
    let unit = diskunit(bp.dev());
    let part = diskpart(bp.dev());
    let partname = dl_partnum2name(part as usize).map_or('?', char::from);
    let mut line = LineBuf::<256>::new();

    let _ = write!(
        line,
        "{dname}{unit}{partname}: {what} {}ing fsbn ",
        if bp.isset(B_READ) { "read" } else { "writ" }
    );
    let bcount = bp.bcount();
    let mut sn = bp.blkno();
    if bcount <= DEV_BSIZE as i64 {
        let _ = write!(line, "{sn}");
    } else {
        if blkdone >= 0 {
            sn += Daddr::from(blkdone);
            let _ = write!(line, "{sn} of ");
        }
        let _ = write!(
            line,
            "{}-{}",
            bp.blkno(),
            bp.blkno() + (bcount - 1) / DEV_BSIZE as i64
        );
    }
    if let Some(g) = g
        && (blkdone >= 0 || bcount <= i64::from(g.secsize))
    {
        sn += g.sectoblk(g.poffset) as Daddr;
        let cyl = g.sectoblk(u64::from(g.secpercyl)) as Daddr;
        let trk = g.sectoblk(u64::from(g.nsectors)) as Daddr;
        let _ = write!(line, " ({dname}{unit} bn {sn}; cn {}", sn / cyl.max(1));
        sn %= cyl.max(1);
        let _ = write!(line, " tn {} sn {})", sn / trk.max(1), sn % trk.max(1));
    }
    if pri != LOG_PRINTF {
        hal::prf::log(pri, format_args!("{}", Str(line.bytes())));
    } else {
        printf(format_args!("{}", Str(line.bytes())));
    }
}

/// Initialize the disklist. Called by main() before autoconfiguration (the list's head is
/// constant: the counters only).
pub fn disk_init() {
    DISK_COUNT.store(0, Ordering::Relaxed);
    DISK_CHANGE.store(0, Ordering::Relaxed);
}

/// Attach a disk: link it on `disklist`, record its device `dv` and the raw partition's
/// block device, and queue the read of its label on `systq`.
pub fn disk_attach<G: ProcGlue>(dv: Option<&DevRef>, dk: &DiskRef) {
    attach::<G>(dv, dv.map(|d| (d.xname().as_bytes(), d.unit())), dk);
}

/// `disk_attach` of a pseudo-device's disk that has no node in the device tree (`bsd`'s
/// vnd(4), whose `struct device` its `vndattach` makes by hand): no `dk_device`, the block
/// device from the device's name `xname` and `unit`, as the C's `findblkmajor(dv)` and
/// `dv->dv_unit` give it.
pub fn disk_attach_unit<G: ProcGlue>(xname: &[u8], unit: i32, dk: &DiskRef) {
    attach::<G>(None, Some((xname, unit)), dk);
}

/// `disk_attach`'s body: `ident` is the device's name and unit (`dv_xname`, `dv_unit`), when
/// there is a device.
fn attach<G: ProcGlue>(dv: Option<&DevRef>, ident: Option<(&[u8], i32)>, dk: &DiskRef) {
    hal::sync::klock::kernel_assert_locked();

    // Allocate and initialize the disklabel structures: the block holds them, zeroed afresh
    // (the C mallocs an M_ZERO label: a disk attached again, vnd's, starts with no label).
    dk.label_store(&Disklabel::zeroed());

    // Set the attached timestamp.
    dk.dk_mtx.lock().attachtime = microuptime();

    // Link into the disklist.
    drop(dk.dk_self.replace(Some(dk.clone())));
    {
        let mut w = DISKS.lock();
        DISKLIST.insert_tail(&mut w, dk.as_pin());
        DISK_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    DISK_CHANGE.store(1, Ordering::Relaxed);

    // Store device structure and number for later use.
    drop(dk.dk_device.replace(dv.cloned()));
    dk.dk_devno.store(NODEV, Ordering::Relaxed);
    if let Some((xname, unit)) = ident {
        let majdev = findblkmajor_name(xname);
        if majdev >= 0 {
            dk.dk_devno.store(
                makediskdev(majdev as u32, unit as u32, RAW_PART),
                Ordering::Relaxed,
            );
        }

        if dk.devno() != NODEV {
            // The task holds a handle of the disk until it has run (the C's device_ref).
            dk.dk_task
                .set_shared(disk_attach_callback::<G>, MArc::erase(dk.clone()));
            let _ = task_add(&SYSTQ, hal::ds::pin::project(dk.as_pin(), |d| &d.dk_task));
        }
    }

    if let Some(attach) = softraid_attach_hook() {
        attach(dk, 1);
    }
}

/// `disk_attach_callback`: reads the label of a newly attached disk (from `systq`), so
/// that its `d_checksum` feeds the entropy pool and `setroot` can wait for it.
fn disk_attach_callback<G: ProcGlue>(a: &AnyArc) {
    let Some(dk) = a.downcast_ref::<Disk>() else {
        return;
    };

    if dk.flags() & (DKF_OPENED | DKF_NOLABELREAD) == 0 {
        // Read disklabel.
        let mut dl = Disklabel::zeroed();
        if disk_readlabel::<G>(&mut dl, dk.devno()).is_ok() {
            enqueue_randomness(u32::from(dl.d_checksum));
        }
    }

    dk.flags_or(DKF_OPENED);
    // The task's handle given back (the C's free of the task record and device_unref).
    dk.dk_task.clear();
    hal::sync::sleep::wakeup(core::ptr::from_ref(dk));
}

/// Detach a disk: off `disklist`, its device given back.
pub fn disk_detach(dk: &DiskRef) {
    hal::sync::klock::kernel_assert_locked();

    if let Some(attach) = softraid_attach_hook() {
        attach(dk, -1);
    }

    // Free the space used by the disklabel structures: they are the block's, freed with it.

    // Remove from the disklist.
    let parked = {
        let mut w = DISKS.lock();
        if dk.dk_link.is_linked() {
            DISKLIST.remove(&mut w, dk);
        }
        let n = DISK_COUNT.fetch_sub(1, Ordering::Relaxed) - 1;
        if n < 0 {
            kpanic(format_args!("disk_detach: disk_count < 0"));
        }
        dk.dk_self.replace(None)
    };
    DISK_CHANGE.store(1, Ordering::Relaxed);
    drop(dk.dk_device.replace(None));
    drop(parked);
}

/// `disk_openpart`: records an open of a partition, after checking that it exists (unless
/// it is the raw partition).
pub fn disk_openpart(dk: &Disk, part: u32, fmt: i32, haslabel: bool) -> Result<(), Errno> {
    // Unless opening the raw partition, check that the partition exists.
    if part != RAW_PART && (!haslabel || !dk.part_exists(part)) {
        return Err(Errno::ENXIO);
    }

    // Ensure the partition doesn't get changed under our feet.
    match fmt as u32 {
        S_IFCHR => {
            dk.dk_copenmask.fetch_or(1u64 << part, Ordering::Relaxed);
        }
        S_IFBLK => {
            dk.dk_bopenmask.fetch_or(1u64 << part, Ordering::Relaxed);
        }
        _ => {}
    }
    dk.set_openmask(dk.copenmask() | dk.bopenmask());

    Ok(())
}

/// `disk_closepart`: records a close of a partition.
pub fn disk_closepart(dk: &Disk, part: u32, fmt: i32) {
    match fmt as u32 {
        S_IFCHR => {
            dk.dk_copenmask
                .fetch_and(!(1u64 << part), Ordering::Relaxed);
        }
        S_IFBLK => {
            dk.dk_bopenmask
                .fetch_and(!(1u64 << part), Ordering::Relaxed);
        }
        _ => {}
    }
    dk.set_openmask(dk.copenmask() | dk.bopenmask());
}

/// Increment a disk's busy counter. If the counter is going from 0 to 1, set the
/// timestamp.
pub fn disk_busy(dk: &Disk) {
    // XXX We'd like to use something as accurate as microtime(), but that doesn't depend
    // on the system TOD clock.
    let mut s = dk.dk_mtx.lock();
    if s.busy == 0 {
        s.timestamp = microuptime();
    }
    s.busy += 1;
}

/// Decrement a disk's busy counter, increment the byte count, total busy time, and reset
/// the timestamp.
pub fn disk_unbusy(dk: &Disk, bcount: i64, blkno: Daddr, read: bool) {
    let diff_time = {
        let mut s = dk.dk_mtx.lock();

        let busy = s.busy;
        s.busy -= 1;
        if busy == 0 {
            printf(format_args!(
                "disk_unbusy: {}: dk_busy < 0\n",
                Str(dk.name())
            ));
        }

        let dv_time = microuptime();

        let diff_time = dv_time.sub(s.timestamp);
        s.time = s.time.add(diff_time);

        s.timestamp = dv_time;
        if bcount > 0 {
            if read {
                s.rbytes += bcount as u64;
                s.rxfer += 1;
            } else {
                s.wbytes += bcount as u64;
                s.wxfer += 1;
            }
        } else {
            s.seek += 1;
        }
        diff_time
    };

    enqueue_randomness((bcount ^ diff_time.tv_usec ^ (blkno >> 32) ^ (blkno & 0xffff_ffff)) as u32);
}

/// `TAILQ_FIRST(&disklist)` (`after` `None`) or `TAILQ_NEXT(after, dk_link)`: the next
/// attached disk, as a handle; one list step under [`DISKS`]. A disk detached between two
/// steps ends the walk at it, as the C's walks never meet one (the kernel lock).
pub fn disk_next(after: Option<&DiskRef>) -> Option<DiskRef> {
    let w = DISKS.lock();
    let next = match after {
        None => DISKLIST.first(&w),
        Some(dk) if dk.dk_link.is_linked() => DISKLIST.next(&w, dk),
        Some(_) => None,
    };
    next.and_then(Disk::listed)
}

/// `disk_count`.
pub fn disk_count() -> usize {
    DISK_COUNT.load(Ordering::Relaxed).max(0) as usize
}

/// `if (disk_change) { disk_change = 0; ... }`: whether the list changed since the last call.
pub fn disk_change_take() -> bool {
    DISK_CHANGE.swap(0, Ordering::Relaxed) != 0
}

/// `sysctl_diskinit`'s copy of one disk: its name and statistics, under `dk_mtx`.
pub fn disk_stats(dk: &Disk) -> Diskstats {
    let tv = |t: Timeval| IfTimeval {
        tv_sec: t.tv_sec,
        tv_usec: t.tv_usec,
    };
    let mut sdk = Diskstats::default();
    let name = dk.name();
    // strlcpy(sdk->ds_name, dk->dk_name, sizeof(sdk->ds_name))
    let n = name.len().min(DS_DISKNAMELEN - 1);
    sdk.ds_name[..n].copy_from_slice(&name[..n]);
    let s = dk.dk_mtx.lock();
    sdk.ds_busy = s.busy;
    sdk.ds_rxfer = s.rxfer;
    sdk.ds_wxfer = s.wxfer;
    sdk.ds_seek = s.seek;
    sdk.ds_rbytes = s.rbytes;
    sdk.ds_wbytes = s.wbytes;
    sdk.ds_attachtime = tv(s.attachtime);
    sdk.ds_timestamp = tv(s.timestamp);
    sdk.ds_time = tv(s.time);
    sdk
}

/// `findblkmajor`: the block major of a disk device, from its name (`nam2blk[]`), or -1.
pub fn findblkmajor(dv: &Device) -> i32 {
    findblkmajor_name(dv.xname().as_bytes())
}

/// `findblkmajor`'s lookup of the device name `name` (`sd0`, `vnd1`, ...).
fn findblkmajor_name(name: &[u8]) -> i32 {
    let len = name
        .iter()
        .position(u8::is_ascii_digit)
        .unwrap_or(name.len());
    let name = &name[..len];

    config_nam2blk()
        .iter()
        .find(|n| n.name == name)
        .map_or(-1, |n| n.maj)
}

/// `findblkname`: the driver name of a block major.
pub fn findblkname(maj: i32) -> Option<&'static [u8]> {
    config_nam2blk()
        .iter()
        .find(|n| n.maj == maj)
        .map(|n| n.name)
}

/// `blktochr`: the character device of a block device (the machine's `chrtoblktbl[]`,
/// walked through the device switch's `chrtoblk`), `NODEV` for none.
fn blktochr<G: ProcGlue>(dev: Dev) -> Dev {
    let sw = crate::fs::spec::devsw::<G>();
    let blkmaj = major(dev);

    if blkmaj >= sw.nblkdev() {
        return NODEV;
    }
    (0..sw.nchrdev())
        .find(|&c| {
            let b = sw.chrtoblk(makedev(c, minor(dev)));
            b != NODEV && major(b) == blkmaj
        })
        .map_or(NODEV, |c| makedev(c, minor(dev)))
}

/// `disk_readlabel`: reads the label of the disk `dev` is on through its raw character
/// partition (`DIOCGDINFO`).
pub fn disk_readlabel<G: ProcGlue>(dl: &mut Disklabel, dev: Dev) -> Result<(), DiskReadlabelError> {
    let chrdev = blktochr::<G>(dev);
    let rawdev = makediskdev(major(chrdev), diskunit(chrdev), RAW_PART);

    let vn = match cdevvp::<G>(rawdev) {
        Ok(Some(vn)) => vn,
        _ => return Err(DiskReadlabelError::NoVnode { dev, rawdev }),
    };

    G::with_curproc(|p| {
        let Some(p) = p else {
            // Every caller runs in a thread (proc0, a kernel thread or a process).
            vput(vn);
            return Err(DiskReadlabelError::Open {
                dev,
                rawdev,
                error: Errno::ENXIO,
            });
        };
        let result = match vop_open(vn, FREAD, Cred::NOCRED, p) {
            Err(error) => Err(DiskReadlabelError::Open { dev, rawdev, error }),
            Ok(()) => vop_ioctl(vn, DIOCGDINFO, dl.as_bytes_mut(), FREAD, Cred::NOCRED, p)
                .map_err(|error| DiskReadlabelError::Ioctl { dev, rawdev, error }),
        };
        let _ = vop_close(vn, FREAD, Cred::NOCRED, Some(p));
        vput(vn);
        result
    })
}

/// `disk_map`: maps a disklabel UID name to the device of the disk carrying that label.
///
/// `path` (up to its NUL) must have the format `[disklabel uid] . [partition]`, or, with
/// `DM_OPENPART` in `flags`, be the DUID on its own (the raw partition). On success
/// `mappath` holds `/dev/<disk><part>` (`/dev/r<disk><part>` without `DM_OPENBLCK`),
/// NUL-terminated and truncated to the buffer as `snprintf` does, and the result is `true`;
/// `false` is the C's `-1`: not a DUID, no disk or more than one disk with that UID.
pub fn disk_map(path: &[u8], mappath: &mut [u8], flags: i32) -> bool {
    let path = &path[..path.iter().position(|&c| c == 0).unwrap_or(path.len())];

    // Attempt to map a request for a disklabel UID to the correct device. We should be
    // supplied with a disklabel UID which has the following format:
    //
    // [disklabel uid] . [partition]
    //
    // Alternatively, if the DM_OPENPART flag is set the disklabel UID can based passed on
    // its own.

    if path.contains(&b'/') {
        return false;
    }

    // Verify that the device name is properly formed.
    if !((path.len() == 16 && flags & DM_OPENPART != 0) || (path.len() == 18 && path[16] == b'.')) {
        return false;
    }

    // Get partition.
    let (part, partno) = if flags & DM_OPENPART != 0 {
        (dl_partnum2name(RAW_PART as usize), Some(RAW_PART as usize))
    } else {
        (Some(path[17]), dl_partname2num(path[17]))
    };
    let (Some(part), Some(_)) = (part, partno) else {
        return false;
    };

    // Derive label UID.
    let mut uid = [0u8; DUID_SIZE];
    for (i, &c) in path[..2 * DUID_SIZE].iter().enumerate() {
        let nibble = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => return false,
        };
        uid[i / 2] = (uid[i / 2] << 4) | nibble;
    }

    let mut mdk: Option<DiskRef> = None;
    let mut d = disk_next(None);
    while let Some(dk) = d {
        if dk.uid() == uid {
            // Fail if there are duplicate UIDs!
            if mdk.is_some() {
                return false;
            }
            mdk = Some(dk.clone());
        }
        d = disk_next(Some(&dk));
    }

    // mdk->dk_name == NULL: the name is a copy here, empty when the driver gave none.
    let Some(mdk) = mdk.filter(|dk| !dk.name().is_empty()) else {
        return false;
    };

    let raw = if flags & DM_OPENBLCK != 0 { "" } else { "r" };
    let mut line = LineBuf::<128>::new();
    let _ = write!(line, "/dev/{raw}{}{}", Str(mdk.name()), char::from(part));
    // snprintf(mappath, size, ...): truncated, NUL-terminated.
    if let Some(room) = mappath.len().checked_sub(1) {
        let n = line.len.min(room);
        mappath[..n].copy_from_slice(&line.bytes()[..n]);
        mappath[n] = 0;
    }

    true
}

/// `duid_equal`.
pub fn duid_equal(duid1: &[u8; DUID_SIZE], duid2: &[u8; DUID_SIZE]) -> bool {
    duid1 == duid2
}

/// `duid_iszero`.
pub fn duid_iszero(duid: &[u8; DUID_SIZE]) -> bool {
    duid_equal(duid, &[0; DUID_SIZE])
}

/// `duid_format`: the DUID as 16 hex digits.
pub fn duid_format(duid: &[u8; DUID_SIZE]) -> [u8; 2 * DUID_SIZE] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = [0u8; 2 * DUID_SIZE];
    for (i, b) in duid.iter().enumerate() {
        s[2 * i] = HEX[usize::from(b >> 4)];
        s[2 * i + 1] = HEX[usize::from(b & 0xf)];
    }
    s
}

/// `int (*mountroot)(void)`: which routine mounts the root file system (the module's
/// redesign: the C's function pointer, compared with `nfs_mountroot`'s address, is an enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mountroot {
    /// `dk_mountroot`: a disk root.
    Disk,
    /// `nfs_mountroot`: a root on NFS.
    Nfs,
}

/// `rootdev`: the root device, `NODEV` until `setroot` picks one (`swapgeneric.c`'s).
pub static ROOTDEV: AtomicI32 = AtomicI32::new(NODEV);

/// `dumpdev`: the dump device (`swapgeneric.c`'s).
pub static DUMPDEV: AtomicI32 = AtomicI32::new(NODEV);

/// `swdevt[]`: the swap devices, `NODEV`-terminated ("to be filled in" by `setroot`);
/// `swapgeneric.c`'s, with room for the RAMDISK configuration's one swap device.
pub static SWDEVT: [AtomicI32; 2] = [AtomicI32::new(NODEV), AtomicI32::new(NODEV)];

/// `mountroot`: 0 is NULL (the kernel is "generic": `setroot` picks one), then
/// [`Mountroot::Disk`] and [`Mountroot::Nfs`].
static MOUNTROOT: AtomicU8 = AtomicU8::new(0);

/// `bootduid`: DUID of boot disk, its 8 bytes in one word. Written by the machine's boot glue
/// before autoconf.
static BOOTDUID: AtomicU64 = AtomicU64::new(0);

/// `rootduid`: DUID of root disk, its 8 bytes in one word. Written by `setroot` (and
/// softraid's `sr_map_root`, which it calls).
static ROOTDUID: AtomicU64 = AtomicU64::new(0);

/// `rootdv`: the root device, chosen by `setroot`.
static ROOTDV: ReadMostly<Option<DevRef>> = ReadMostly::new(None);

/// `mountroot = m` (the configuration's `swapgeneric.c`/`swapbsd.c`, and `setroot`).
pub fn mountroot_set(m: Option<Mountroot>) {
    let v = match m {
        None => 0,
        Some(Mountroot::Disk) => 1,
        Some(Mountroot::Nfs) => 2,
    };
    MOUNTROOT.store(v, Ordering::Relaxed);
}

/// `mountroot`.
pub fn mountroot_get() -> Option<Mountroot> {
    match MOUNTROOT.load(Ordering::Relaxed) {
        1 => Some(Mountroot::Disk),
        2 => Some(Mountroot::Nfs),
        _ => None,
    }
}

/// `(*mountroot)()`: `None` when no root is configured (the C's NULL `mountroot`).
pub fn mountroot<G: ProcGlue>() -> Option<Result<(), Errno>> {
    match mountroot_get()? {
        Mountroot::Disk => Some(dk_mountroot::<G>()),
        Mountroot::Nfs => Some(nfs_mountroot::<G>()),
    }
}

/// `bootduid`.
pub fn bootduid() -> [u8; DUID_SIZE] {
    BOOTDUID.load(Ordering::Relaxed).to_ne_bytes()
}

/// `memcpy(bootduid, duid, sizeof(bootduid))` (the machine's boot glue).
pub fn set_bootduid(duid: [u8; DUID_SIZE]) {
    BOOTDUID.store(u64::from_ne_bytes(duid), Ordering::Relaxed);
}

/// `rootduid`.
pub fn rootduid() -> [u8; DUID_SIZE] {
    ROOTDUID.load(Ordering::Relaxed).to_ne_bytes()
}

/// `memcpy(rootduid, duid, sizeof(rootduid))` (`setroot`, softraid's `sr_map_root`).
pub fn set_rootduid(duid: [u8; DUID_SIZE]) {
    ROOTDUID.store(u64::from_ne_bytes(duid), Ordering::Relaxed);
}

/// `rootdv`: the root device `setroot` chose.
pub fn rootdv() -> Option<DevRef> {
    ROOTDV.read().clone()
}

/// `nfs_mountroot`: the `mountroot` of a root on NFS, over `swdevt[0]` and the machine's
/// `swapdev`; without `NFSCLIENT` there is none (`setroot` never chooses it).
fn nfs_mountroot<G: ProcGlue>() -> Result<(), Errno> {
    #[cfg(feature = "nfsclient")]
    {
        crate::fs::nfs::nfs_vfsops::nfs_mountroot::<G>(&SWDEVT[0], config_swapdev())
    }
    #[cfg(not(feature = "nfsclient"))]
    Err(Errno::ENODEV)
}

/// `dk_mountroot`: the `mountroot` of a disk root: reads `rootdev`'s label and mounts the
/// root partition with the mountroot of its file system type.
pub fn dk_mountroot<G: ProcGlue>() -> Result<(), Errno> {
    let rootdev = ROOTDEV.load(Ordering::Relaxed);
    let part = diskpart(rootdev) as usize;
    let swapdev = config_swapdev();

    let mut dl = Disklabel::zeroed();
    if let Err(error) = disk_readlabel::<G>(&mut dl, rootdev) {
        kpanic(format_args!("{error}"));
    }

    if dl_getpsize(&dl.d_partitions[part]) == 0 {
        kpanic(format_args!("root filesystem has size 0"));
    }
    let fstype = dl.d_partitions[part].p_fstype;
    #[cfg(feature = "ext2fs")]
    if fstype == hal::sys::disklabel::FS_EXT2FS {
        return crate::fs::ufs::ext2fs::ext2fs_vfsops::ext2fs_mountroot::<G>(rootdev, swapdev);
    }
    #[cfg(feature = "cd9660")]
    if fstype == hal::sys::disklabel::FS_ISO9660 {
        return crate::fs::isofs::cd9660::cd9660_vfsops::cd9660_mountroot::<G>(rootdev, swapdev);
    }
    if fstype != FS_BSDFFS {
        printf(format_args!(
            "filesystem type {fstype} not known.. assuming ffs\n"
        ));
    }
    crate::fs::ufs::ffs::ffs_vfsops::ffs_mountroot::<G>(rootdev, swapdev)
}

/// `getdisk`: `parsedisk`, listing the choices when `str` names no disk.
pub fn getdisk(str: &[u8], defpart: u32) -> Option<(DevRef, Dev)> {
    let found = parsedisk(str, defpart);
    if found.is_none() {
        printf(format_args!("use one of: exit"));
        let mut d = alldevs_next(None);
        while let Some(n) = d {
            if n.class() == DV_DISK {
                printf(format_args!(" {}[a-p]", n.xname()));
            }
            #[cfg(feature = "nfsclient")]
            if n.class() == crate::dev::device::DV_IFNET {
                printf(format_args!(" {}", n.xname()));
            }
            d = alldevs_next(Some(&n));
        }
        printf(format_args!("\n"));
    }
    found
}

/// `parsedisk`: the disk device `str` names (`wd0`, `rd0a`, ...) and its block device for
/// the partition the name ends in (`defpart` without one); with `NFSCLIENT`, a network
/// interface and `NODEV`.
pub fn parsedisk(str: &[u8], defpart: u32) -> Option<(DevRef, Dev)> {
    let mut len = str.len();
    if len == 0 {
        return None;
    }
    let part = match dl_partname2num(str[len - 1]) {
        Some(n) if n < MAXPARTITIONS => {
            len -= 1;
            n as u32
        }
        _ => defpart,
    };

    let mut d = alldevs_next(None);
    while let Some(n) = d {
        if n.class() == DV_DISK && n.xname().as_bytes() == &str[..len] {
            let majdev = findblkmajor(&n);
            if majdev < 0 {
                return None;
            }
            let dev = makediskdev(majdev as u32, n.unit() as u32, part);
            return Some((n, dev));
        }
        #[cfg(feature = "nfsclient")]
        if n.class() == crate::dev::device::DV_IFNET && n.xname().as_bytes() == &str[..len] {
            return Some((n, NODEV));
        }
        d = alldevs_next(Some(&n));
    }
    None
}

/// The first attached disk for which `f` holds (`TAILQ_FOREACH` ... `break`).
fn disk_find(f: impl Fn(&Disk) -> bool) -> Option<DiskRef> {
    let mut d = disk_next(None);
    while let Some(dk) = d {
        if f(&dk) {
            return Some(dk);
        }
        d = disk_next(Some(&dk));
    }
    None
}

/// `setroot`: chooses the root, swap and dump devices (`rootdev`, `swdevt[0]`, `dumpdev`)
/// from the configuration, the boot device and the boot disk's DUID, and sets `mountroot`.
pub fn setroot<G: ProcGlue>(bootdv: Option<DevRef>, part: u32, exitflags: i32) {
    let _ = exitflags; // reboot(exitflags) answers "exit" at the prompt (RB_ASKNAME, below).
    let mut bootdv = bootdv;
    let mut part = part;

    // Ensure that all disk attach callbacks have completed.
    let mut slept = 0;
    loop {
        let pending = disk_find(|dk| dk.devno() != NODEV && dk.flags() & DKF_OPENED == 0);
        let Some(dk) = pending else { break };
        let _ = tsleep_nsec(
            core::ptr::from_ref::<Disk>(&dk),
            0,
            "dkopen",
            sec_to_nsec(1),
        );
        slept += 1;
        if slept >= 5 {
            break;
        }
    }

    if slept == 5 {
        printf(format_args!("disklabels not read:"));
        let mut d = disk_next(None);
        while let Some(dk) = d {
            if dk.devno() != NODEV && dk.flags() & DKF_OPENED == 0 {
                printf(format_args!(" {}", Str(dk.name())));
            }
            d = disk_next(Some(&dk));
        }
        printf(format_args!("\n"));
    }

    let mut bduid = bootduid();
    if duid_iszero(&bduid) {
        // Locate DUID for boot disk since it was not provided.
        let dk = disk_find(|dk| match &bootdv {
            Some(b) => dk.is_device(b),
            None => dk.device().is_none(),
        });
        if let Some(dk) = dk {
            bduid = dk.uid();
            set_bootduid(bduid);
        }
    } else if bootdv.is_none() {
        // Locate boot disk based on the provided DUID.
        if let Some(dv) = disk_find(|dk| dk.uid() == bduid).and_then(|dk| dk.device()) {
            bootdv = Some(dv);
        }
    }
    set_rootduid(bduid);

    // NSOFTRAID > 0
    sr_map_root();
    let rduid = rootduid();

    // If `swap generic' and we couldn't determine boot device, ask the user.
    let mut dk_found: Option<DiskRef> = None;
    let generic = mountroot_get().is_none();
    if generic && bootdv.is_none() {
        config_boothowto_or(RB_ASKNAME);
    }
    if config_boothowto() & RB_ASKNAME != 0 {
        // The "root device" and "swap device" prompts (getsn under cnpollc).
        let _ = unported!("setroot: RB_ASKNAME prompts (getsn)");
        if generic && bootdv.is_none() {
            printf(format_args!("root device: none configured\n"));
            return;
        }
    }
    let rootdev = ROOTDEV.load(Ordering::Relaxed);
    let rootdv: DevRef;
    if mountroot_get() == Some(Mountroot::Nfs) {
        // `mountroot == nfs_mountroot'
        let Some(dv) = bootdv.clone() else { return };
        rootdv = dv;
        ROOTDEV.store(NODEV, Ordering::Relaxed);
        DUMPDEV.store(NODEV, Ordering::Relaxed);
        // swapdev = NODEV: see the module's deviations.
    } else if generic && rootdev == NODEV {
        // `swap generic'
        let Some(mut dv) = bootdv.clone() else { return };

        if dv.class() == DV_DISK && !duid_iszero(&rduid) {
            let Some(dk) = disk_find(|dk| dk.uid() == rduid) else {
                kpanic(format_args!(
                    "root device ({}) not found",
                    Str(&duid_format(&rduid))
                ));
            };
            if let Some(d) = dk.device() {
                dv = d;
            }
            dk_found = Some(dk);
        }
        rootdv = dv;

        let majdev = findblkmajor(&rootdv);
        let nswapdev = if majdev >= 0 {
            // Root and swap are on the disk. Assume swap is on partition b.
            let unit = rootdv.unit() as u32;
            ROOTDEV.store(makediskdev(majdev as u32, unit, part), Ordering::Relaxed);
            makediskdev(majdev as u32, unit, 1)
        } else {
            // Root and swap are on a net.
            NODEV
        };
        DUMPDEV.store(nswapdev, Ordering::Relaxed);
        SWDEVT[0].store(nswapdev, Ordering::Relaxed);
    } else {
        // Completely pre-configured, but we want rootdv ..
        let majdev = major(rootdev) as i32;
        let Some(name) = findblkname(majdev) else {
            return;
        };
        let unit = diskunit(rootdev);
        part = diskpart(rootdev);
        let mut buf = LineBuf::<128>::new();
        let _ = write!(
            buf,
            "{}{}{}",
            Str(name),
            unit,
            char::from(dl_partnum2name(part as usize).unwrap_or(b'?'))
        );
        let Some((dv, _)) = parsedisk(buf.bytes(), 0) else {
            kpanic(format_args!("root device ({}) not found", Str(buf.bytes())));
        };
        rootdv = dv;
    }
    drop(ROOTDV.replace(Some(rootdv.clone())));

    if let Some(bdv) = &bootdv
        && bdv.class() == crate::dev::device::DV_IFNET
        && let Some(ifp) = crate::net::if_::if_unit(bdv.xname().as_bytes())
    {
        let _ = crate::net::if_::if_addgroup(ifp, b"netboot");
        crate::net::if_::if_put(Some(ifp));
    }

    if rootdv.class() == DV_DISK {
        mountroot_set(Some(Mountroot::Disk));
        part = diskpart(ROOTDEV.load(Ordering::Relaxed));
    } else {
        #[cfg(feature = "nfsclient")]
        if rootdv.class() == crate::dev::device::DV_IFNET {
            mountroot_set(Some(Mountroot::Nfs));
            crate::fs::nfs::nfs_boot::set_nfsbootdevname(rootdv.xname().as_bytes());
            return;
        }
        printf(format_args!(
            "can't figure root, hope your kernel is right\n"
        ));
        return;
    }

    let partname = char::from(dl_partnum2name(part as usize).unwrap_or(b'?'));
    printf(format_args!("root on {}{partname}", rootdv.xname()));

    if let Some(dk) = dk_found
        && dk.is_device(&rootdv)
    {
        printf(format_args!(" ({}.{partname})", Str(&duid_format(&rduid))));
    }

    // Make the swap partition on the root drive the primary swap.
    let rootdev = ROOTDEV.load(Ordering::Relaxed);
    let mut temp = NODEV;
    let mut found = false;
    for sw in &SWDEVT {
        let d = sw.load(Ordering::Relaxed);
        if d == NODEV {
            break;
        }
        if major(rootdev) == major(d) && diskunit(rootdev) == diskunit(d) {
            temp = SWDEVT[0].load(Ordering::Relaxed);
            SWDEVT[0].store(d, Ordering::Relaxed);
            sw.store(temp, Ordering::Relaxed);
            found = true;
            break;
        }
    }
    if found {
        // If dumpdev was the same as the old primary swap device, move it to the new
        // primary swap device.
        if temp == DUMPDEV.load(Ordering::Relaxed) {
            DUMPDEV.store(SWDEVT[0].load(Ordering::Relaxed), Ordering::Relaxed);
        }
    }
    let print_dev = |what: &str, d: Dev| {
        let name = findblkname(major(d) as i32).unwrap_or(b"??");
        let p = char::from(dl_partnum2name(diskpart(d) as usize).unwrap_or(b'?'));
        printf(format_args!(" {what} on {}{}{p}", Str(name), diskunit(d)));
    };
    let sw0 = SWDEVT[0].load(Ordering::Relaxed);
    if sw0 != NODEV {
        print_dev("swap", sw0);
    }
    let dumpdev = DUMPDEV.load(Ordering::Relaxed);
    if dumpdev != NODEV {
        print_dev("dump", dumpdev);
    }
    printf(format_args!("\n"));
}

const _: () = {
    assert!(LABEL_WORDS * 8 >= DISKLABEL_SIZE);
    assert!(LABEL_WORDS == 147);
    assert!(core::mem::align_of::<AtomicU64>() >= core::mem::align_of::<Disklabel>());
};

/// Host tests: the list emptied and the counters zeroed (`testing.md`: the disks' blocks
/// are an earlier test's memory, their handles forgotten, never dropped into the next
/// test's), the softraid hooks left as registered (a `OnceStatic`).
#[cfg(any(test, feature = "doubles"))]
pub fn disk_test_reset() {
    let mut w = DISKS.lock();
    while let Some(dk) = DISKLIST.first(&w) {
        core::mem::forget(dk.dk_self.replace(None));
        DISKLIST.remove_first(&mut w);
    }
    drop(w);
    disk_init();
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the disk layer (LZ's moved with it, over a strategy closure on an
    // in-memory disk instead of a `&'static Buf`): label checksums and checks (byte-swapped
    // labels too), `bounds_check_with_label` over a `LabelGeom`, `readdoslabel` (a bare disk
    // with a label at sector 1, an MBR with an OpenBSD partition, a FAT boot sector, a GPT
    // disk with its backup header), `setdisklabel`, the open masks, the DUID helpers and
    // `disk_map`; and the counted block's: the list's order and counters across attach and
    // detach, a handle held across a detach, the statistics of busy/unbusy, the words of the
    // in-core label (`geom`, `label`, `uid`) and `diskerr`'s line.

    use std::sync::{Mutex, MutexGuard};
    use std::vec::Vec;
    use std::{assert, assert_eq, format, vec};

    use super::*;
    use hal::sys::disklabel::{DOSPARTOFF, DOSPTYP_FAT32, FS_SWAP};
    use hal::sysbytes::DisklabelBytes;
    use hal::user::AbiBytes;

    /// The in-memory disk the strategy below reads and writes, in `DEV_BSIZE` sectors.
    static IMG: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    /// The strategy over `IMG`: a synchronous transfer at `b_blkno` (`EIO` past the end).
    fn img_io(mut t: BufRef) -> (BufRef, Result<(), Errno>) {
        let off = t.buf().blkno() as usize * DEV_BSIZE;
        let len = t.buf().bcount() as usize;
        let mut img = IMG.lock().unwrap_or_else(|e| e.into_inner());
        if off + len > img.len() {
            t.buf().set_error(Some(Errno::EIO));
            t.buf().set(B_ERROR);
            return (t, Err(Errno::EIO));
        }
        if t.buf().isset(B_READ) {
            t.data()[..len].copy_from_slice(&img[off..off + len]);
        } else {
            img[off..off + len].copy_from_slice(&t.bytes()[..len]);
        }
        t.buf().set_resid(0);
        (t, Ok(()))
    }

    /// The test lock (a buffer cache, the kernel lock held), an empty disk list and a disk
    /// of `sectors` zero sectors.
    fn setup(sectors: usize) -> MutexGuard<'static, ()> {
        let g = crate::testutil::bufcache_install(1 << 20, 64);
        disk_test_reset();
        *IMG.lock().unwrap_or_else(|e| e.into_inner()) = vec![0u8; sectors * DEV_BSIZE];
        g
    }

    /// Writes `bytes` at byte offset `off` of the disk.
    fn poke(off: usize, bytes: &[u8]) {
        IMG.lock().unwrap_or_else(|e| e.into_inner())[off..off + bytes.len()]
            .copy_from_slice(bytes);
    }

    /// `readdoslabel` over the image, through a fresh label buffer of one sector.
    fn read_dos(
        lp: &mut Disklabel,
        partoffp: Option<&mut Daddr>,
        spoofonly: bool,
    ) -> Result<(), Errno> {
        let mut io = img_io;
        let mut bp = LabelBuf::new(makediskdev(17, 0, RAW_PART), 512, &mut io);
        readdoslabel(&mut bp, lp, partoffp, spoofonly)
    }

    /// A valid label for a disk of `sectors`: `a` is FFS over the whole disk, `b` swap.
    fn label(sectors: u32) -> Disklabel {
        let mut d = Disklabel::zeroed();
        d.d_magic = DISKMAGIC;
        d.d_magic2 = DISKMAGIC;
        d.d_secsize = 512;
        d.d_nsectors = sectors;
        d.d_ntracks = 1;
        d.d_ncylinders = 1;
        d.d_secpercyl = sectors;
        d.d_secperunit = sectors;
        d.d_version = 1;
        d.d_npartitions = 3;
        dl_setpsize(&mut d.d_partitions[0], u64::from(sectors));
        d.d_partitions[0].p_fstype = FS_BSDFFS;
        dl_setpsize(&mut d.d_partitions[1], 8);
        dl_setpoffset(&mut d.d_partitions[1], 8);
        d.d_partitions[1].p_fstype = FS_SWAP;
        dl_setpsize(&mut d.d_partitions[2], u64::from(sectors));
        d.d_checksum = dkcksum(&d);
        d
    }

    /// The label a driver spoofs for a disk of `sectors` before reading (as rd(4) does).
    fn spoofed(sectors: u32) -> Disklabel {
        let mut d = Disklabel::zeroed();
        d.d_secsize = 512;
        d.d_nsectors = sectors;
        d.d_ntracks = 1;
        d.d_ncylinders = 1;
        d.d_secpercyl = sectors;
        dl_setdsize(&mut d, u64::from(sectors));
        d.d_version = 1;
        d
    }

    #[test]
    fn a_label_with_its_checksum_sums_to_zero() {
        let d = label(64);
        assert_eq!(dkcksum(&d), 0);
        let mut bad = d;
        bad.d_ntracks = 2;
        assert!(dkcksum(&bad) != 0);
    }

    #[test]
    fn initdisklabel_makes_the_raw_partition_the_disk() {
        let mut d = spoofed(100);
        d.d_partitions[0].p_size = 5;
        assert_eq!(initdisklabel(&mut d), Ok(()));
        assert_eq!(usize::from(d.d_npartitions), MAXPARTITIONS);
        assert_eq!(dl_getpsize(&d.d_partitions[0]), 0);
        assert_eq!(dl_getpsize(&d.d_partitions[RAW_PART as usize]), 100);
        assert_eq!(dl_getbend(&d), 100);
        d.d_secpercyl = 0;
        assert_eq!(initdisklabel(&mut d), Err(Errno::ERANGE));
    }

    #[test]
    fn checkdisklabel_accepts_native_and_byte_swapped_labels() {
        let mut lp = spoofed(64);
        let mut dlp = label(64);
        assert_eq!(checkdisklabel(0, &mut dlp, &mut lp, 0, 1000), Ok(()));
        assert_eq!(lp.d_partitions[0].p_fstype, FS_BSDFFS);
        assert_eq!(dl_getbend(&lp), 64); // clamped to the disk
        assert_eq!(dkcksum(&lp), 0);

        // The same label written by a machine of the other byte order.
        let native = label(64);
        let mut swapped = native;
        swapped.d_magic = native.d_magic.swap_bytes();
        swapped.d_magic2 = native.d_magic2.swap_bytes();
        swapped.d_secsize = native.d_secsize.swap_bytes();
        swapped.d_nsectors = native.d_nsectors.swap_bytes();
        swapped.d_ntracks = native.d_ntracks.swap_bytes();
        swapped.d_ncylinders = native.d_ncylinders.swap_bytes();
        swapped.d_secpercyl = native.d_secpercyl.swap_bytes();
        swapped.d_secperunit = native.d_secperunit.swap_bytes();
        swapped.d_version = native.d_version.swap_bytes();
        swapped.d_npartitions = native.d_npartitions.swap_bytes();
        for p in swapped.d_partitions.iter_mut().take(3) {
            p.p_size = p.p_size.swap_bytes();
            p.p_offset = p.p_offset.swap_bytes();
        }
        // The checksum over the swapped words, as the other machine computed it.
        swapped.d_checksum = 0;
        let sum = swapped
            .cksum_words(usize::from(swapped.d_npartitions.swap_bytes()))
            .fold(0u16, |s, w| s ^ w);
        swapped.d_checksum = sum;
        let mut lp = spoofed(64);
        assert_eq!(checkdisklabel(0, &mut swapped, &mut lp, 0, 64), Ok(()));
        assert_eq!(lp.d_secsize, 512);
        assert_eq!(dl_getpsize(&lp.d_partitions[1]), 8);
        assert_eq!(dl_getpoffset(&lp.d_partitions[1]), 8);

        let mut zero = Disklabel::zeroed();
        assert_eq!(
            checkdisklabel(0, &mut zero, &mut lp, 0, 64),
            Err(Errno::EINVAL)
        );
        let mut nomagic = spoofed(64);
        assert_eq!(
            checkdisklabel(0, &mut nomagic, &mut lp, 0, 64),
            Err(Errno::ENOENT)
        );
    }

    #[test]
    fn bounds_check_truncates_and_rejects() {
        let lp = label(64);
        let g = LabelGeom::of(&lp, 0);
        let bp = Buf::new();
        bp.set_dev(makediskdev(17, 0, 0));

        bp.set_blkno(60);
        bp.set_bcount(8 * 512);
        assert!(bounds_check_with_label(&bp, &g));
        assert_eq!(bp.bcount(), 4 * 512); // truncated at the end of `a`

        bp.set_blkno(64);
        bp.set_bcount(512);
        assert!(!bounds_check_with_label(&bp, &g)); // EOF
        assert!(!bp.isset(B_ERROR));
        assert_eq!(bp.resid(), 512);

        bp.set_blkno(65);
        assert!(!bounds_check_with_label(&bp, &g));
        assert!(bp.isset(B_ERROR));
        assert_eq!(bp.error(), Some(Errno::EINVAL));

        let bp = Buf::new();
        bp.set_bcount(100); // not a whole sector
        assert!(!bounds_check_with_label(&bp, &g));
        assert!(bp.isset(B_ERROR));

        // No label (the zero geometry): EINVAL, as the C's division guard.
        let bp = Buf::new();
        bp.set_bcount(512);
        assert!(!bounds_check_with_label(&bp, &LabelGeom::default()));
        assert_eq!(bp.error(), Some(Errno::EINVAL));
    }

    #[test]
    fn readdoslabel_reads_the_label_of_a_bare_disk() {
        let _g = setup(64);
        poke(512, &label(64).as_bytes()[..512]);

        let mut lp = spoofed(64);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, false), Ok(()));
        assert_eq!(lp.d_magic, DISKMAGIC);
        assert_eq!(lp.d_partitions[0].p_fstype, FS_BSDFFS);
        assert_eq!(dl_getpsize(&lp.d_partitions[0]), 64);
        assert_eq!(dl_getpsize(&lp.d_partitions[RAW_PART as usize]), 64);
        assert_eq!(dkcksum(&lp), 0);

        // No label at sector 1: the spoofed raw partition only, and the check's error.
        poke(512, &[0u8; 512]);
        let mut lp = spoofed(64);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, false), Err(Errno::EINVAL));
        // spoofonly stops before reading the label.
        let mut lp = spoofed(64);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, true), Ok(()));
        assert_eq!(dl_getpsize(&lp.d_partitions[RAW_PART as usize]), 64);
        assert_eq!(dl_getpsize(&lp.d_partitions[0]), 0);
    }

    /// An MBR at sector 0 with the given `(type, start, size)` entries and its signature.
    fn mbr(entries: &[(u8, u32, u32)]) {
        let mut s = [0u8; 512];
        for (i, &(typ, start, size)) in entries.iter().enumerate() {
            let e = DOSPARTOFF + 16 * i;
            s[e + 4] = typ;
            s[e + 8..e + 12].copy_from_slice(&start.to_le_bytes());
            s[e + 12..e + 16].copy_from_slice(&size.to_le_bytes());
        }
        s[510..512].copy_from_slice(&DOSMBR_SIGNATURE.to_le_bytes());
        poke(0, &s);
    }

    #[test]
    fn an_mbr_puts_the_label_in_the_openbsd_partition() {
        let _g = setup(256);
        mbr(&[(DOSPTYP_FAT32, 1, 31), (DOSPTYP_OPENBSD, 64, 192)]);
        // The label lives at sector 1 of the A6 partition.
        let mut inner = label(192);
        dl_setpoffset(&mut inner.d_partitions[0], 64);
        inner.d_checksum = 0;
        inner.d_checksum = dkcksum(&inner);
        poke(65 * 512, &inner.as_bytes()[..512]);

        let mut partoff: Daddr = -1;
        let mut lp = spoofed(256);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, Some(&mut partoff), true), Ok(()));
        assert_eq!(partoff, 64);

        let mut lp = spoofed(256);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, true), Ok(()));
        // The FAT partition is spoofed as `i`, the OpenBSD one bounds the label.
        let i = usize::from(b'i' - b'a');
        assert_eq!(lp.d_partitions[i].p_fstype, FS_MSDOS);
        assert_eq!(dl_getpoffset(&lp.d_partitions[i]), 1);
        assert_eq!(dl_getpsize(&lp.d_partitions[i]), 31);
        assert_eq!((dl_getbstart(&lp), dl_getbend(&lp)), (64, 256));

        let mut lp = spoofed(256);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, false), Ok(()));
        assert_eq!(dl_getpoffset(&lp.d_partitions[0]), 64);
        assert_eq!(lp.d_partitions[0].p_fstype, FS_BSDFFS);
    }

    #[test]
    fn a_fat_boot_sector_spoofs_partition_i_without_a_label() {
        let mut s = [0u8; DEV_BSIZE];
        s[0] = 0xeb;
        s[2] = 0x90;
        s[11..13].copy_from_slice(&512u16.to_le_bytes());
        s[16] = 2;
        let mut lp = spoofed(64);
        initdisklabel(&mut lp).expect("geometry");
        let mut partoff: Daddr = 0;
        spooffat(&s, &mut lp, &mut partoff);
        assert_eq!(partoff, -1);
        assert_eq!(lp.d_magic, DISKMAGIC);
        let i = usize::from(b'i' - b'a');
        assert_eq!(lp.d_partitions[i].p_fstype, FS_MSDOS);
        assert_eq!(dl_getpsize(&lp.d_partitions[i]), 64);
    }

    #[test]
    fn a_protective_mbr_is_recognised() {
        let mut dp = [DosPartition::default(); NDOSPART];
        dp[0].dp_typ = DOSPTYP_EFI;
        dp[0].dp_start = 1u32.to_le();
        dp[0].dp_size = 1000u32.to_le();
        assert_eq!(gpt_chk_mbr(&dp, 2000), Some(0));
        dp[1].dp_typ = DOSPTYP_OPENBSD;
        dp[1].dp_size = 5u32.to_le();
        assert_eq!(gpt_chk_mbr(&dp, 2000), None); // a hybrid MBR is not protective
    }

    /// GPT test disk geometry: 256 sectors, the entries at LBA 2..34, usable 34..=222.
    const GPT_SECTORS: u64 = 256;
    const GPT_LBA_START: u64 = 34;
    const GPT_LBA_END: u64 = 222;

    /// One 128-byte GPT entry of type `ty` (memory order) over `start..=end`.
    fn gpt_entry(ty: [u8; 16], start: u64, end: u64) -> [u8; 128] {
        let mut e = [0u8; 128];
        e[..16].copy_from_slice(&ty);
        e[16] = 0x42; // a non-zero unique GUID
        e[32..40].copy_from_slice(&start.to_le_bytes());
        e[40..48].copy_from_slice(&end.to_le_bytes());
        e
    }

    /// Writes a GPT header at `lba` whose 128 entries live at `part_lba`, and returns it.
    fn gpt_header(lba: u64, alt: u64, part_lba: u64, parts: &[u8]) -> [u8; 92] {
        let mut h = [0u8; 92];
        h[0..8].copy_from_slice(&GPTSIGNATURE.to_le_bytes());
        h[8..12].copy_from_slice(&GPTREVISION.to_le_bytes());
        h[12..16].copy_from_slice(&GPTMINHDRSIZE.to_le_bytes());
        h[24..32].copy_from_slice(&lba.to_le_bytes());
        h[32..40].copy_from_slice(&alt.to_le_bytes());
        h[40..48].copy_from_slice(&GPT_LBA_START.to_le_bytes());
        h[48..56].copy_from_slice(&GPT_LBA_END.to_le_bytes());
        h[72..80].copy_from_slice(&part_lba.to_le_bytes());
        h[80..84].copy_from_slice(&128u32.to_le_bytes());
        h[84..88].copy_from_slice(&GPTMINPARTSIZE.to_le_bytes());
        h[88..92].copy_from_slice(&crc32(0, parts).to_le_bytes());
        let csum = crc32(0, &h);
        h[16..20].copy_from_slice(&csum.to_le_bytes());
        poke(lba as usize * 512, &h);
        h
    }

    /// A GPT disk: protective MBR, the EFI system partition 34..=49, OpenBSD 64..=222 with a
    /// label at its sector 1, primary header at LBA 1 and backup at the last sector.
    fn gpt_disk() {
        mbr(&[(DOSPTYP_EFI, 1, GPT_SECTORS as u32 - 1)]);
        let mut parts = vec![0u8; 128 * 128];
        parts[..128].copy_from_slice(&gpt_entry(KNOWNFS[6].0, 34, 49));
        parts[128..256].copy_from_slice(&gpt_entry(KNOWNFS[1].0, 64, GPT_LBA_END));
        poke(2 * 512, &parts);
        gpt_header(1, GPT_SECTORS - 1, 2, &parts);
        gpt_header(GPT_SECTORS - 1, 1, 2, &parts);

        let mut inner = label(GPT_SECTORS as u32);
        dl_setpoffset(&mut inner.d_partitions[0], 64);
        dl_setpsize(&mut inner.d_partitions[0], GPT_LBA_END - 64 + 1);
        inner.d_checksum = 0;
        inner.d_checksum = dkcksum(&inner);
        poke(65 * 512, &inner.as_bytes()[..512]);
    }

    #[test]
    fn a_gpt_puts_the_label_in_the_openbsd_partition() {
        let _g = setup(GPT_SECTORS as usize);
        gpt_disk();

        let mut partoff: Daddr = -1;
        let mut lp = spoofed(GPT_SECTORS as u32);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, Some(&mut partoff), true), Ok(()));
        assert_eq!(partoff, 64);

        let mut lp = spoofed(GPT_SECTORS as u32);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, true), Ok(()));
        let i = usize::from(b'i' - b'a');
        assert_eq!(lp.d_partitions[i].p_fstype, FS_MSDOS);
        assert_eq!(dl_getpoffset(&lp.d_partitions[i]), 34);
        assert_eq!(dl_getpsize(&lp.d_partitions[i]), 16);
        assert_eq!((dl_getbstart(&lp), dl_getbend(&lp)), (64, GPT_LBA_END + 1));

        let mut lp = spoofed(GPT_SECTORS as u32);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, false), Ok(()));
        assert_eq!(dl_getpoffset(&lp.d_partitions[0]), 64);
        assert_eq!(lp.d_partitions[0].p_fstype, FS_BSDFFS);
    }

    #[test]
    fn a_bad_primary_gpt_falls_back_to_the_backup_and_two_bad_ones_fail() {
        let _g = setup(GPT_SECTORS as usize);
        gpt_disk();
        poke(512 + 20, &[0xff]); // break the primary header (its checksum no longer matches)

        let mut partoff: Daddr = -1;
        let mut lp = spoofed(GPT_SECTORS as u32);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, Some(&mut partoff), true), Ok(()));
        assert_eq!(partoff, 64);

        // gpt_get_hdr zeroes an invalid header.
        let lp = spoofed(GPT_SECTORS as u32);
        let mut io = img_io;
        let mut bp = LabelBuf::new(0, 512, &mut io);
        assert_eq!(gpt_get_hdr(&mut bp, &lp, 1), Ok(GptHeader::default()));
        drop(bp);

        poke((GPT_SECTORS as usize - 1) * 512 + 20, &[0xff]);
        let mut lp = spoofed(GPT_SECTORS as u32);
        initdisklabel(&mut lp).expect("geometry");
        assert_eq!(read_dos(&mut lp, None, false), Err(Errno::ENXIO));
    }

    #[test]
    fn gpt_types_map_to_file_systems() {
        assert_eq!(gpt_get_fstype(&Uuid::default()), FS_UNUSED);
        assert_eq!(gpt_get_fstype(&Uuid::from_bytes(KNOWNFS[1].0)), FS_BSDFFS);
        assert_eq!(gpt_get_fstype(&Uuid::from_bytes([0x5a; 16])), FS_OTHER);
    }

    #[test]
    fn mbr_types_map_to_file_systems() {
        assert_eq!(mbr_get_fstype(DOSPTYP_OPENBSD), FS_BSDFFS);
        assert_eq!(mbr_get_fstype(DOSPTYP_LINUX), FS_EXT2FS);
        assert_eq!(mbr_get_fstype(DOSPTYP_EFISYS), FS_MSDOS);
        assert_eq!(mbr_get_fstype(DOSPTYP_EXTEND), FS_OTHER);
        assert_eq!(mbr_get_fstype(DOSPTYP_UNUSED), FS_UNUSED);
    }

    #[test]
    fn setdisklabel_checks_and_keeps_the_disk_size() {
        let _g = setup(1);
        crate::testutil::install();
        let mut olp = label(64);
        let mut nlp = label(64);
        dl_setdsize(&mut nlp, 1); // ignored: the disk size is preserved
        nlp.d_checksum = 0;
        nlp.d_checksum = dkcksum(&nlp);
        assert_eq!(setdisklabel(&mut olp, &mut nlp, 0), Ok(()));
        assert_eq!(dl_getdsize(&olp), 64);
        assert!(!duid_iszero(&olp.d_uid)); // a DUID was generated
        assert_eq!(dkcksum(&olp), 0);

        // Shrinking an open partition is refused.
        let mut nlp = label(64);
        dl_setpsize(&mut nlp.d_partitions[0], 10);
        nlp.d_checksum = 0;
        nlp.d_checksum = dkcksum(&nlp);
        assert_eq!(setdisklabel(&mut olp, &mut nlp, 1), Err(Errno::EBUSY));

        let mut bad = label(64);
        bad.d_secsize = 100;
        assert_eq!(setdisklabel(&mut olp, &mut bad, 0), Err(Errno::EINVAL));
    }

    #[test]
    fn open_masks_follow_opens_and_closes() {
        let _g = setup(1);
        let dk = Disk::new(b"sd0", 0);
        dk.label_store(&label(64));
        assert_eq!(disk_openpart(&dk, 0, S_IFBLK as i32, true), Ok(()));
        assert_eq!(disk_openpart(&dk, 2, S_IFCHR as i32, false), Ok(()));
        assert_eq!(
            disk_openpart(&dk, 5, S_IFBLK as i32, true),
            Err(Errno::ENXIO)
        );
        assert_eq!(
            disk_openpart(&dk, 0, S_IFBLK as i32, false),
            Err(Errno::ENXIO)
        );
        assert_eq!(dk.openmask(), 0b101);
        disk_closepart(&dk, 0, S_IFBLK as i32);
        assert_eq!(dk.openmask(), 0b100);
        assert_eq!((dk.bopenmask(), dk.copenmask()), (0, 0b100));
    }

    #[test]
    fn duids_format_as_hex() {
        let duid = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
        assert_eq!(&duid_format(&duid), b"0123456789abcdef");
        assert!(duid_iszero(&[0; DUID_SIZE]));
        assert!(!duid_iszero(&duid));
        assert!(duid_equal(&duid, &duid));
    }

    #[test]
    fn readlabel_errors_print_the_c_messages() {
        let e = DiskReadlabelError::Open {
            dev: 0x1100,
            rawdev: 0x2f02,
            error: Errno::ENXIO,
        };
        assert_eq!(format!("{e}"), "cannot open disk, 0x1100/0x2f02, error 6");
        let e = DiskReadlabelError::NoVnode {
            dev: 0x1100,
            rawdev: 0x2f02,
        };
        assert_eq!(format!("{e}"), "cannot obtain vnode for 0x1100/0x2f02");
    }

    /// A disk named `name` whose label has the DUID `uid`, attached (no device).
    fn listed_disk(name: &[u8], uid: [u8; DUID_SIZE]) -> DiskRef {
        let dk = Disk::new(name, 0);
        disk_attach::<crate::fs::testfs::TestGlue>(None, &dk);
        // After the attach, which zeroes the label (the C's M_ZERO malloc).
        let mut lp = label(64);
        lp.d_uid = uid;
        dk.label_store(&lp);
        dk
    }

    /// `disk_map(path, flags)` as a string, `None` for the C's -1.
    fn map(path: &[u8], flags: i32) -> Option<std::string::String> {
        let mut out = [0xffu8; 90];
        if !disk_map(path, &mut out, flags) {
            return None;
        }
        let n = out.iter().position(|&c| c == 0).unwrap_or(out.len());
        Some(std::string::String::from_utf8_lossy(&out[..n]).into_owned())
    }

    #[test]
    fn disk_map_finds_a_disk_by_its_duid() {
        let _g = setup(1);
        let uid = [0x4a, 0x5b, 0x6c, 0x7d, 0x8e, 0x9f, 0x01, 0x23];
        let dk = listed_disk(b"sd3", uid);

        assert_eq!(
            map(b"4a5b6c7d8e9f0123.a\0junk", DM_OPENBLCK).as_deref(),
            Some("/dev/sd3a")
        );
        assert_eq!(map(b"4a5b6c7d8e9f0123.d", 0).as_deref(), Some("/dev/rsd3d"));
        assert_eq!(
            map(b"4a5b6c7d8e9f0123", DM_OPENPART).as_deref(),
            Some("/dev/rsd3c")
        );
        assert_eq!(
            map(b"4a5b6c7d8e9f0123.e", DM_OPENPART).as_deref(),
            Some("/dev/rsd3c")
        );
        // Truncated to the buffer, as snprintf does.
        let mut small = [0xffu8; 6];
        assert!(disk_map(b"4a5b6c7d8e9f0123.a", &mut small, DM_OPENBLCK));
        assert_eq!(&small, b"/dev/\0");

        // Not a DUID name.
        for bad in [
            &b"/dev/sd3a"[..],
            b"4a5b6c7d8e9f0123",
            b"4a5b6c7d8e9f0123:a",
            b"4a5b6c7d8e9f0123.a/",
            b"4A5B6C7D8E9F0123.a",
            b"4a5b6c7d8e9f0123.?",
            // No such disk.
            b"4a5b6c7d8e9f0124.a",
        ] {
            assert_eq!(
                map(bad, 0),
                None,
                "{}",
                std::string::String::from_utf8_lossy(bad)
            );
        }

        // Fail if there are duplicate UIDs!
        let twin = listed_disk(b"sd4", uid);
        assert_eq!(map(b"4a5b6c7d8e9f0123.a", 0), None);

        disk_detach(&twin);
        disk_detach(&dk);
        assert_eq!(map(b"4a5b6c7d8e9f0123.a", 0), None);
    }

    #[test]
    fn attach_and_detach_keep_the_list_order_and_counters() {
        let _g = setup(1);
        let a = listed_disk(b"sd0", [1; 8]);
        let b = listed_disk(b"sd1", [2; 8]);
        let c = listed_disk(b"cd0", [0; 8]);
        assert_eq!(disk_count(), 3);
        assert!(disk_change_take());
        assert!(!disk_change_take());

        let names = || {
            let mut v = Vec::new();
            let mut d = disk_next(None);
            while let Some(dk) = d {
                v.push(dk.name().to_vec());
                d = disk_next(Some(&dk));
            }
            v
        };
        assert_eq!(
            names(),
            vec![b"sd0".to_vec(), b"sd1".to_vec(), b"cd0".to_vec()]
        );

        disk_detach(&b);
        assert_eq!(disk_count(), 2);
        assert!(disk_change_take());
        assert_eq!(names(), vec![b"sd0".to_vec(), b"cd0".to_vec()]);
        // A walk standing on a detached disk ends there.
        assert!(disk_next(Some(&b)).is_none());
        // The handle held across the detach keeps the block, its name and label.
        assert_eq!(b.name(), b"sd1");
        assert_eq!(b.uid(), [2; 8]);

        disk_detach(&a);
        disk_detach(&c);
        assert_eq!(disk_count(), 0);
        assert!(disk_next(None).is_none());
    }

    #[test]
    fn busy_and_unbusy_account_transfers_bytes_and_seeks() {
        let _g = setup(1);
        crate::testutil::install();
        let dk = Disk::new(b"wd0", 0);
        disk_busy(&dk);
        disk_busy(&dk);
        assert_eq!(disk_stats(&dk).ds_busy, 2);
        disk_unbusy(&dk, 4096, 8, true);
        disk_unbusy(&dk, 1024, 16, false);
        disk_busy(&dk);
        disk_unbusy(&dk, 0, 0, true);
        let s = disk_stats(&dk);
        assert_eq!(s.ds_busy, 0);
        assert_eq!((s.ds_rxfer, s.ds_rbytes), (1, 4096));
        assert_eq!((s.ds_wxfer, s.ds_wbytes), (1, 1024));
        assert_eq!(s.ds_seek, 1);
        assert_eq!(&s.ds_name[..4], b"wd0\0");
        assert_eq!(s.as_bytes().len(), 112);
    }

    #[test]
    fn geom_and_the_label_words_equal_the_stored_label() {
        let _g = setup(1);
        let dk = Disk::new(b"sd0", 0);
        let mut lp = label(64);
        lp.d_npartitions = MAXPARTITIONS as u16;
        lp.d_uid = [9, 8, 7, 6, 5, 4, 3, 2];
        for (i, p) in lp.d_partitions.iter_mut().enumerate().take(MAXPARTITIONS) {
            dl_setpoffset(p, 0x1_0000_0000 * i as u64 + 3 * i as u64);
            dl_setpsize(p, 0x2_0000_0000 + 7 * i as u64);
            p.p_fstype = i as u8;
        }
        lp.d_checksum = 0;
        lp.d_checksum = dkcksum(&lp);
        dk.label_store(&lp);
        for part in 0..MAXPARTITIONS as u32 {
            assert_eq!(dk.geom(part), LabelGeom::of(&lp, part), "partition {part}");
        }
        // Past the table: an empty partition, the label's own geometry.
        assert_eq!(dk.geom(MAXPARTITIONS as u32).psize, 0);
        assert_eq!(dk.geom(MAXPARTITIONS as u32).secsize, 512);
        let mut back = Disklabel::zeroed();
        dk.label(&mut back);
        assert_eq!(back, lp);
        assert_eq!(dk.uid(), lp.d_uid);
        // DIOCGPART's pointers name the words' bytes.
        assert_eq!(
            dk.partition_ptr(1) as usize - dk.label_ptr() as usize,
            148 + 16
        );
    }

    #[test]
    fn diskerr_prints_the_c_line() {
        crate::testutil::install();
        let lp = label(64);
        let bp = Buf::new();
        bp.set_dev(makediskdev(0, 1, 0));
        bp.set_blkno(10);
        bp.set_bcount(512);
        bp.set(B_READ);
        diskerr(
            &bp,
            "wd",
            "hard error",
            LOG_PRINTF,
            0,
            Some(&LabelGeom::of(&lp, 0)),
        );
        assert!(
            crate::testutil::printf_count(
                "wd1a: hard error reading fsbn 10 (wd1 bn 10; cn 0 tn 0 sn 10)"
            ) > 0
        );
    }

    #[test]
    fn a_label_buffer_reads_and_writes_through_its_strategy() {
        let _g = setup(4);
        poke(1024, b"sector two");
        let mut io = img_io;
        let mut bp = LabelBuf::new(0, 512, &mut io);
        let lp = spoofed(4);
        assert_eq!(readdisksector(&mut bp, &lp, 2), Ok(()));
        assert_eq!(&bp.bytes()[..10], b"sector two");
        // Past the disk: the strategy's error.
        assert_eq!(readdisksector(&mut bp, &lp, 9), Err(Errno::EIO));
    }

    #[test]
    fn the_root_statics_round_trip() {
        let _g = setup(1);
        mountroot_set(Some(Mountroot::Nfs));
        assert_eq!(mountroot_get(), Some(Mountroot::Nfs));
        mountroot_set(Some(Mountroot::Disk));
        assert_eq!(mountroot_get(), Some(Mountroot::Disk));
        mountroot_set(None);
        assert_eq!(mountroot_get(), None);
        assert!(mountroot::<crate::fs::testfs::TestGlue>().is_none());
        let d = [1, 2, 3, 4, 5, 6, 7, 8];
        set_bootduid(d);
        assert_eq!(bootduid(), d);
        set_rootduid([0; DUID_SIZE]);
        assert!(duid_iszero(&rootduid()));
        set_bootduid([0; DUID_SIZE]);
    }

    #[test]
    fn parsedisk_refuses_what_names_no_disk() {
        let _g = setup(1);
        assert!(parsedisk(b"", 0).is_none());
        assert!(parsedisk(b"nosuch9a", 0).is_none());
    }

    #[test]
    fn a_disk_attached_again_starts_with_no_label() {
        let _g = setup(1);
        let dk = listed_disk(b"vnd0", [7; 8]);
        assert_eq!(dk.uid(), [7; 8]);
        disk_detach(&dk);
        assert!(!dk.is_attached());
        disk_attach::<crate::fs::testfs::TestGlue>(None, &dk);
        assert!(dk.is_attached());
        assert_eq!(dk.uid(), [0; 8]);
        assert_eq!(dk.geom(0), LabelGeom::default());
        disk_detach(&dk);
    }

    #[test]
    fn unbusy_warns_once_the_count_goes_below_zero() {
        let _g = setup(1);
        crate::testutil::install();
        let dk = Disk::new(b"wd7", 0);
        let before = crate::testutil::printf_count("disk_unbusy: wd7: dk_busy < 0");
        disk_unbusy(&dk, 0, 0, true);
        disk_unbusy(&dk, 0, 0, true);
        // `dk_busy-- == 0`: only the first underflow warns.
        assert_eq!(
            crate::testutil::printf_count("disk_unbusy: wd7: dk_busy < 0") - before,
            1
        );
    }
}
/* </TESTS> */
