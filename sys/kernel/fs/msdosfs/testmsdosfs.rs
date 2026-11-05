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
/* </LICENSES> */

/* <CODE> */
//! The kernel msdosfs's test fixtures (host tests and benches only, `cfg(any(test, feature =
//! "doubles"))`; not an OpenBSD file): [`mkfat`], the FAT image builder (a small
//! `newfs_msdos(8)`: the boot sector, the FAT32 FSInfo block, the FATs, the root directory,
//! files and directories with short and Win95 long names), moved from `bsd`'s
//! `msdosfs_vfsops.rs` tests as it was; and [`attach`], [`try_mount`], [`mount`],
//! [`mount_root`], [`unmount`], [`root`], the image mounted by the kernel's `msdosfs_mountfs`
//! through [`MsdosfsVfs`], over testffs's device half: the image is the disk of testfs's block
//! device (`testffs::attach`, `testffs::disk`, `testffs::set_disk`, `testffs::sync`), and
//! [`full_root`], the FAT16 image of a full root directory the denode cache's, the
//! directories' and the vnode operations' tests of the root's real size use (the user's
//! decision D-N5.7-U4). The tests over the system calls take their user memory from
//! `testfs::up`, `UBUF` (`USER_BASE + 0xe000..0xf000`), holding the kernel memory's guard.

use std::sync::MutexGuard;
use std::vec::Vec;

use hal::sys::errno::Errno;
use hal::sys::mount::{
    MNT_LOCAL, MNT_RDONLY, MNT_SWAPPABLE, MNT_WAIT, MOUNT_MSDOS, MSDOSFSMNT_LONGNAME, MsdosfsArgs,
};

use super::msdosfs_vfsops::{MsdosfsVfs, msdosfs_mountfs};
use super::msdosfsmount::{MsdosfsmountRef, vfstomsdosfs};
use crate::fs::init::{rootvnode, set_rootvnode};
use crate::fs::mount::{
    Mount, Vfsconf, mountlist_insert_tail, vfs_mount_alloc, vfs_mount_free, vfs_unbusy,
};
use crate::fs::subr::{vref, vrele};
use crate::fs::testfs::TestGlue;
use crate::fs::ufs::ffs::testffs;
use crate::fs::vnode::Vnode;
use crate::fs::vops::{Cred, Vfs, vop_unlock};
use crate::proc::{ProcGlue, Thread};

/// `mkfat`: a FAT image built in memory the way `newfs_msdos(8)` and `makefs -t msdos` build
/// one, for the tests and as a record of the on-disk layout.
pub mod mkfat {
    use std::vec;
    use std::vec::Vec;

    /// The geometry to build.
    #[derive(Clone, Copy)]
    pub struct Params {
        /// 12, 16 or 32: the FAT width (FAT32 means no fixed root directory).
        pub fat: u32,
        /// The image's size in bytes.
        pub size: usize,
        /// Bytes per sector.
        pub bps: u16,
        /// Sectors per cluster.
        pub spc: u8,
        /// Root directory entries (0 for FAT32).
        pub rde: u16,
    }

    /// FAT12, 1 MB, 512-byte clusters, 64 root directory entries.
    pub const FAT12_1M: Params = Params {
        fat: 12,
        size: 1 << 20,
        bps: 512,
        spc: 1,
        rde: 64,
    };

    /// FAT16, 4 MB, 512-byte clusters, 512 root directory entries.
    pub const FAT16_4M: Params = Params {
        fat: 16,
        size: 4 << 20,
        bps: 512,
        spc: 1,
        rde: 512,
    };

    /// FAT32, 1 MB, 512-byte clusters (a FAT32 the kernel accepts: the type comes from the
    /// missing root directory, not from the cluster count).
    pub const FAT32_1M: Params = Params {
        fat: 32,
        size: 1 << 20,
        bps: 512,
        spc: 1,
        rde: 0,
    };

    /// Where a directory is: the fixed root directory of FAT12/16, or a cluster chain.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Dir {
        /// The FAT12/16 root directory.
        Root,
        /// The directory whose first cluster is this one.
        Clust(u32),
    }

    /// An image under construction.
    pub struct Image {
        /// The disk.
        pub disk: Vec<u8>,
        /// The geometry.
        pub p: Params,
        /// Reserved sectors.
        pub res: u32,
        /// Sectors per FAT.
        pub spf: u32,
        /// Sectors of the root directory (FAT12/16).
        pub rootsecs: u32,
        /// Data clusters.
        pub nclusters: u32,
        /// The next cluster handed out.
        next: u32,
    }

    /// The Win95 checksum of a DOS name.
    pub fn chksum(name: &[u8; 11]) -> u8 {
        name.iter().fold(0u8, |s, &c| {
            ((s & 1) << 7).wrapping_add(s >> 1).wrapping_add(c)
        })
    }

    /// The Win95 long name entries of `name` for the DOS name `short`, in on-disk order (the
    /// last part first).
    pub fn lfn_entries(name: &[u8], short: &[u8; 11]) -> Vec<[u8; 32]> {
        let ck = chksum(short);
        let n = name.len().div_ceil(13);
        let mut out = Vec::new();
        for i in (1..=n).rev() {
            let mut e = [0u8; 32];
            e[0] = i as u8 | if i == n { 0x40 } else { 0 };
            e[11] = 0x0f;
            e[13] = ck;
            let offs = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];
            for (k, &o) in offs.iter().enumerate() {
                let idx = (i - 1) * 13 + k;
                let c: u16 = match idx.cmp(&name.len()) {
                    core::cmp::Ordering::Less => u16::from(name[idx]),
                    core::cmp::Ordering::Equal => 0,
                    core::cmp::Ordering::Greater => 0xffff,
                };
                e[o..o + 2].copy_from_slice(&c.to_le_bytes());
            }
            out.push(e);
        }
        out
    }

    /// A short directory entry.
    pub fn short_entry(short: &[u8; 11], attr: u8, cluster: u32, size: u32) -> [u8; 32] {
        let mut e = [0u8; 32];
        e[..11].copy_from_slice(short);
        e[11] = attr;
        e[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
        e[24..26].copy_from_slice(&0x21u16.to_le_bytes()); // 1980-01-01
        e[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());
        e[28..32].copy_from_slice(&size.to_le_bytes());
        e
    }

    impl Image {
        /// `newfs_msdos`: an empty file system.
        pub fn new(p: Params) -> Self {
            let bps = u32::from(p.bps);
            let sectors = (p.size / p.bps as usize) as u32;
            let res = if p.fat == 32 { 32 } else { 1 };
            let rootsecs = (u32::from(p.rde) * 32).div_ceil(bps);
            let mut spf = 1;
            let nclusters = loop {
                let data = sectors - res - 2 * spf - rootsecs;
                let ncl = data / u32::from(p.spc);
                let need = ((ncl + 2) * p.fat).div_ceil(8).div_ceil(bps);
                if need <= spf {
                    break ncl;
                }
                spf = need;
            };
            let mut img = Image {
                disk: vec![0u8; p.size],
                p,
                res,
                spf,
                rootsecs,
                nclusters,
                next: 2,
            };

            let d = &mut img.disk;
            d[0..3].copy_from_slice(&[0xeb, 0x3c, 0x90]);
            d[3..11].copy_from_slice(b"EMIBSD  ");
            d[11..13].copy_from_slice(&p.bps.to_le_bytes());
            d[13] = p.spc;
            d[14..16].copy_from_slice(&(res as u16).to_le_bytes());
            d[16] = 2;
            d[17..19].copy_from_slice(&p.rde.to_le_bytes());
            if sectors < 65536 && p.fat != 32 {
                d[19..21].copy_from_slice(&(sectors as u16).to_le_bytes());
            } else {
                d[32..36].copy_from_slice(&sectors.to_le_bytes());
            }
            d[21] = 0xf8;
            if p.fat != 32 {
                d[22..24].copy_from_slice(&(spf as u16).to_le_bytes());
            }
            d[24..26].copy_from_slice(&63u16.to_le_bytes());
            d[26..28].copy_from_slice(&255u16.to_le_bytes());
            if p.fat == 32 {
                d[36..40].copy_from_slice(&spf.to_le_bytes());
                d[44..48].copy_from_slice(&2u32.to_le_bytes()); // root cluster
                d[48..50].copy_from_slice(&1u16.to_le_bytes()); // FSInfo sector
                d[50..52].copy_from_slice(&6u16.to_le_bytes()); // backup boot sector
                d[66] = 0x29;
                d[82..90].copy_from_slice(b"FAT32   ");
                // FSInfo, 1024 bytes from sector 1.
                let fsi = bps as usize;
                d[fsi..fsi + 4].copy_from_slice(b"RRaA");
                d[fsi + 484..fsi + 488].copy_from_slice(b"rrAa");
                d[fsi + 488..fsi + 492].copy_from_slice(&u32::MAX.to_le_bytes());
                d[fsi + 492..fsi + 496].copy_from_slice(&u32::MAX.to_le_bytes());
                d[fsi + 508..fsi + 512].copy_from_slice(&[0, 0, 0x55, 0xaa]);
                d[fsi + 1020..fsi + 1024].copy_from_slice(&[0, 0, 0x55, 0xaa]);
            } else {
                d[38] = 0x29;
                d[54..62].copy_from_slice(if p.fat == 12 {
                    b"FAT12   "
                } else {
                    b"FAT16   "
                });
            }
            d[510] = 0x55;
            d[511] = 0xaa;

            let mask = img.mask();
            img.fat_set(0, 0xffff_fff8 & mask);
            img.fat_set(1, mask);
            if p.fat == 32 {
                let root = img.alloc(1);
                assert_eq!(root, 2);
            }
            img
        }

        /// The FAT entry mask.
        pub fn mask(&self) -> u32 {
            match self.p.fat {
                12 => 0xfff,
                16 => 0xffff,
                _ => 0x0fff_ffff,
            }
        }

        /// Bytes per cluster.
        pub fn bpc(&self) -> usize {
            usize::from(self.p.bps) * usize::from(self.p.spc)
        }

        /// The byte offset of the first data cluster.
        fn data_off(&self) -> usize {
            (self.res + 2 * self.spf + self.rootsecs) as usize * usize::from(self.p.bps)
        }

        /// The byte offset of cluster `cn`.
        pub fn clust_off(&self, cn: u32) -> usize {
            self.data_off() + (cn as usize - 2) * self.bpc()
        }

        /// The byte offset of the FAT12/16 root directory.
        pub fn root_off(&self) -> usize {
            (self.res + 2 * self.spf) as usize * usize::from(self.p.bps)
        }

        /// The entry of cluster `cn` in the first FAT.
        pub fn fat_get(&self, cn: u32) -> u32 {
            let base = self.res as usize * usize::from(self.p.bps);
            match self.p.fat {
                12 => {
                    let o = base + cn as usize * 3 / 2;
                    let v = u16::from_le_bytes([self.disk[o], self.disk[o + 1]]);
                    u32::from(if cn & 1 != 0 { v >> 4 } else { v & 0xfff })
                }
                16 => {
                    let o = base + cn as usize * 2;
                    u32::from(u16::from_le_bytes([self.disk[o], self.disk[o + 1]]))
                }
                _ => {
                    let o = base + cn as usize * 4;
                    u32::from_le_bytes([
                        self.disk[o],
                        self.disk[o + 1],
                        self.disk[o + 2],
                        self.disk[o + 3],
                    ]) & 0x0fff_ffff
                }
            }
        }

        /// Sets the entry of cluster `cn` in both FATs.
        pub fn fat_set(&mut self, cn: u32, v: u32) {
            for f in 0..2 {
                let base = (self.res + f * self.spf) as usize * usize::from(self.p.bps);
                match self.p.fat {
                    12 => {
                        let o = base + cn as usize * 3 / 2;
                        let old = u16::from_le_bytes([self.disk[o], self.disk[o + 1]]);
                        let v = v as u16 & 0xfff;
                        let new = if cn & 1 != 0 {
                            (old & 0x000f) | (v << 4)
                        } else {
                            (old & 0xf000) | v
                        };
                        self.disk[o..o + 2].copy_from_slice(&new.to_le_bytes());
                    }
                    16 => {
                        let o = base + cn as usize * 2;
                        self.disk[o..o + 2].copy_from_slice(&(v as u16).to_le_bytes());
                    }
                    _ => {
                        let o = base + cn as usize * 4;
                        self.disk[o..o + 4].copy_from_slice(&v.to_le_bytes());
                    }
                }
            }
        }

        /// Allocates a chain of `n` clusters, zeroed; its first cluster.
        pub fn alloc(&mut self, n: u32) -> u32 {
            let first = self.next;
            for i in 0..n {
                let cn = first + i;
                let v = if i + 1 == n { self.mask() } else { cn + 1 };
                self.fat_set(cn, v);
                let o = self.clust_off(cn);
                let bpc = self.bpc();
                self.disk[o..o + bpc].fill(0);
            }
            self.next += n;
            first
        }

        /// The byte offsets of the slots of `dir`, in order.
        pub fn slots(&self, dir: Dir) -> Vec<usize> {
            match dir {
                Dir::Root => {
                    let o = self.root_off();
                    (0..usize::from(self.p.rde)).map(|i| o + i * 32).collect()
                }
                Dir::Clust(mut cn) => {
                    let mut v = Vec::new();
                    loop {
                        let o = self.clust_off(cn);
                        v.extend((0..self.bpc() / 32).map(|i| o + i * 32));
                        let next = self.fat_get(cn);
                        if next >= (0x0fff_fff8 & self.mask()) {
                            break v;
                        }
                        cn = next;
                    }
                }
            }
        }

        /// The FAT32 root directory, or the fixed one.
        pub fn root(&self) -> Dir {
            if self.p.fat == 32 {
                Dir::Clust(2)
            } else {
                Dir::Root
            }
        }

        /// Appends raw entries to `dir`, after its last used slot; the byte offset of the
        /// first one in the directory.
        pub fn put(&mut self, dir: Dir, entries: &[[u8; 32]]) -> u32 {
            let slots = self.slots(dir);
            let Some(first) = slots.iter().position(|&o| self.disk[o] == 0) else {
                std::panic!("mkfat: directory full");
            };
            assert!(
                first + entries.len() <= slots.len(),
                "mkfat: directory full"
            );
            for (k, e) in entries.iter().enumerate() {
                let o = slots[first + k];
                self.disk[o..o + 32].copy_from_slice(e);
            }
            (first * 32) as u32
        }

        /// Adds a file: its data in new clusters and its entry (with long name entries when
        /// `long` is given) in `dir`; the directory offset of the short entry.
        pub fn add_file(
            &mut self,
            dir: Dir,
            long: Option<&[u8]>,
            short: &[u8; 11],
            data: &[u8],
        ) -> u32 {
            let n = data.len().div_ceil(self.bpc()) as u32;
            let cn = if n == 0 { 0 } else { self.alloc(n) };
            for (i, chunk) in data.chunks(self.bpc()).enumerate() {
                let o = self.clust_off(cn + i as u32);
                self.disk[o..o + chunk.len()].copy_from_slice(chunk);
            }
            let mut ents = long.map_or(Vec::new(), |l| lfn_entries(l, short));
            ents.push(short_entry(short, 0x20, cn, data.len() as u32));
            self.put(dir, &ents) + 32 * (ents.len() as u32 - 1)
        }

        /// Adds a directory of `nclust` clusters with its "." and ".." entries; its first
        /// cluster.
        pub fn mkdir(&mut self, parent: Dir, short: &[u8; 11], nclust: u32) -> u32 {
            let cn = self.alloc(nclust);
            let pcn = match parent {
                Dir::Root => 0,
                Dir::Clust(c) if self.p.fat == 32 && c == 2 => 0,
                Dir::Clust(c) => c,
            };
            let o = self.clust_off(cn);
            self.disk[o..o + 32].copy_from_slice(&short_entry(b".          ", 0x10, cn, 0));
            self.disk[o + 32..o + 64].copy_from_slice(&short_entry(b"..         ", 0x10, pcn, 0));
            self.put(parent, &[short_entry(short, 0x10, cn, 0)]);
            cn
        }

        /// The finished image.
        pub fn finish(self) -> Vec<u8> {
            self.disk
        }
    }
}

/// The fixture's configuration entry: the kernel's msdosfs as `vfsconflist[]` has it
/// (`"msdos"`, type 4, `struct msdosfs_args`, `MNT_LOCAL | MNT_SWAPPABLE`).
pub static TEST_MSDOS_CONF: Vfsconf<TestGlue> = Vfsconf::new(
    &MsdosfsVfs::<TestGlue>::VFS,
    MOUNT_MSDOS,
    4,
    MNT_LOCAL | MNT_SWAPPABLE,
    MsdosfsArgs::SIZE,
);

/// Zeroed mount arguments (`msdosfs_mountfs` does not read them).
pub fn noargs() -> MsdosfsArgs {
    match MsdosfsArgs::from_bytes(&[0u8; MsdosfsArgs::SIZE]) {
        Some(a) => a,
        None => std::panic!("testmsdosfs: no msdosfs_args"),
    }
}

/// A test thread over fresh VFS tables with page memory and a buffer cache, its `uiomove`
/// copying kernel iovecs, the denode hash made (`vfs_init`: `msdosfs_init`), and `image` as
/// the disk of testfs's block device (testffs's [`testffs::attach`]): nothing mounted yet.
pub fn attach(image: Vec<u8>) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let r = testffs::attach(image);
    if let Err(e) = MsdosfsVfs::<TestGlue>::VFS.init(&TEST_MSDOS_CONF) {
        std::panic!("testmsdosfs: msdosfs_init: {e:?}");
    }
    r
}

/// A new mount of the disk with the flags `flags` (`MNT_RDONLY`, `MNT_SYNCHRONOUS`), as
/// `msdosfs_mount` makes one: `msdosfs_mountfs` on the device vnode, the statistics
/// (`VFS_STATFS`), the mount unbusied, not on the mount list; `Err` freed the mount.
pub fn try_mount(p: &Thread<TestGlue>, flags: i32) -> Result<&'static Mount<TestGlue>, Errno> {
    let mp = vfs_mount_alloc(None, &TEST_MSDOS_CONF);
    mp.set_flag(mp.flag() | flags);
    mp.with_stat(|sp| sp.f_mntonname[..6].copy_from_slice(b"/msdos"));
    let r = msdosfs_mountfs(testffs::devvp(), mp, p, &noargs());
    if r.is_ok() {
        let mut st = mp.stat();
        let _ = MsdosfsVfs::<TestGlue>::VFS.statfs(mp, &mut st, p);
        mp.with_stat(|sp| *sp = st);
    }
    vfs_unbusy(mp);
    match r {
        Ok(()) => Ok(mp),
        Err(e) => {
            vfs_mount_free(mp);
            Err(e)
        }
    }
}

/// [`try_mount`] by the test's thread (the glue's `curproc`); `panic` on an error.
pub fn mount(flags: i32) -> &'static Mount<TestGlue> {
    let r = TestGlue::with_curproc(|p| match p {
        Some(p) => try_mount(p, flags),
        None => std::panic!("testmsdosfs: no curproc"),
    });
    match r {
        Ok(mp) => mp,
        Err(e) => std::panic!("testmsdosfs: msdosfs_mountfs: {e:?}"),
    }
}

/// `VFS_SYNC(MNT_WAIT)` (a read-write mount's, as `dounmount` syncs) and `VFS_UNMOUNT` of a
/// mount of [`try_mount`] whose vnodes are all released (`vflush` without `FORCECLOSE`);
/// `panic` on an error.
pub fn unmount(p: &Thread<TestGlue>, mp: &'static Mount<TestGlue>) {
    let v = &MsdosfsVfs::<TestGlue>::VFS;
    if mp.flag() & MNT_RDONLY == 0
        && let Err(e) = v.sync(mp, MNT_WAIT, 0, Cred::real(p.ucred()), p)
    {
        std::panic!("testmsdosfs: msdosfs_sync: {e:?}");
    }
    if let Err(e) = v.unmount(mp, 0, p) {
        std::panic!("testmsdosfs: msdosfs_unmount: {e:?}");
    }
}

/// A new mount of the disk at `/` as `mount_msdos -l -m 777` makes it (long names, mask
/// 0777, owner and group 0: what `msdosfs_mount` writes after `msdosfs_mountfs`), as `main`
/// mounts the root: [`try_mount`], the mount list, the root vnode and the thread's current
/// directory (testfs's root, which [`attach`]'s thread started with, released).
/// `testffs::unmount_root` undoes it.
pub fn mount_root(p: &'static Thread<TestGlue>, flags: i32) -> &'static Mount<TestGlue> {
    let mp = match try_mount(p, flags) {
        Ok(mp) => mp,
        Err(e) => std::panic!("testmsdosfs: msdosfs_mountfs: {e:?}"),
    };
    let pmp = vfstomsdosfs(mp);
    pmp.pm_mask.set(0o777);
    pmp.pm_flags
        .set(pmp.pm_flags.get() | MSDOSFSMNT_LONGNAME as u32);
    drop(pmp);
    mountlist_insert_tail(mp);
    let vp = root(mp);
    if let Some(old) = rootvnode::<TestGlue>() {
        vrele(old);
    }
    set_rootvnode(Some(vp));
    vref(vp);
    if let Some(old) = p.fd().replace_cdir(Some(vp)) {
        vrele(old);
    }
    let _ = vop_unlock(vp);
    mp
}

/// The root directory's vnode of `mp`, locked and referenced (`vput` it): `VFS_ROOT`.
pub fn root(mp: &'static Mount<TestGlue>) -> &'static Vnode<TestGlue> {
    match MsdosfsVfs::<TestGlue>::VFS.root(mp) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testmsdosfs: no root: {e:?}"),
    }
}

/// The FAT16 file system of the denode cache's, the directories' and the vnode operations'
/// tests of the root's real size (the user's decision D-N5.7-U4), mounted read-write (mask
/// 0777, owner and group 0): sectors of `bps` bytes, clusters of `spc` sectors, over 4,084
/// clusters (FAT16, which the cluster count decides); a root directory of `rootents` entries,
/// every one used (`F0000000.TXT`, `F0000001.TXT`, ..., empty files); and an entry
/// `PHANTOM.TXT` right after the root's end, `pm_rootdirsize` blocks of `DEV_BSIZE` from its
/// start: the first bytes of the first data cluster for the C's geometry (what a scan past
/// the root's real end finds). With 1 KiB sectors and 208 entries the root ends in the
/// middle of a sector, which `mkfat` (`newfs_msdos(8)`) rounds up: the data clusters start
/// half a sector after the phantom for it, at the phantom for the kernel's and the C's
/// `pm_firstcluster`, which counts the root's `DEV_BSIZE` blocks; no test reads a data
/// cluster. The guard of the kernel's memory, the thread and the mount's structure returned.
pub fn full_root(
    bps: u16,
    spc: u8,
    rootents: u16,
) -> (
    MutexGuard<'static, ()>,
    &'static Thread<TestGlue>,
    MsdosfsmountRef<TestGlue>,
) {
    let rootsecs = (usize::from(rootents) * 32).div_ceil(usize::from(bps));
    let sectors = 4200 * usize::from(spc) + 64 + rootsecs;
    let mut img = mkfat::Image::new(mkfat::Params {
        fat: 16,
        size: sectors * usize::from(bps),
        bps,
        spc,
        rde: rootents,
    });
    let ents: Vec<[u8; 32]> = (0..rootents)
        .map(|i| {
            let mut n = *b"F0000000TXT";
            n[1..8].copy_from_slice(std::format!("{i:07}").as_bytes());
            mkfat::short_entry(&n, 0x20, 0, 0)
        })
        .collect();
    img.put(mkfat::Dir::Root, &ents);
    let at = img.root_off() + (usize::from(rootents) * 32).div_ceil(512) * 512;
    let mut disk = img.finish();
    disk[at..at + 32].copy_from_slice(&mkfat::short_entry(b"PHANTOM TXT", 0x20, 0, 0));
    let (g, p) = attach(disk);
    let mp = match try_mount(p, 0) {
        Ok(mp) => mp,
        Err(e) => std::panic!("testmsdosfs: full_root: {e:?}"),
    };
    let pmp = vfstomsdosfs(mp);
    pmp.pm_mask.set(0o777);
    (g, p, pmp)
}
/* </CODE> */
