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
//! The kernel ffs's test fixtures (host tests and benches only, `cfg(any(test, feature =
//! "doubles"))`; not an OpenBSD file): [`newfs`], the image builder (`newfs(8)`'s `mkfs.c`
//! in small: the super-block, the cylinder groups, the root directory, files in it), and
//! [`setup`], an image mounted under the kernel's ffs code as far as N5.5b1 has it: the
//! super-block in core and the summaries read from the image, the mount's UFS structure in
//! its typed slot, the inodes `vget` makes with the kernel's [`Ffs`] as their operations,
//! and the device a block device of testfs's switch whose strategy reads and writes the
//! image ([`disk`]). N5.5b2's `ffs_mountfs` and `ffs_vget` replace [`setup`]'s mount and
//! `vget` (their steps are the C's, in the order the C takes them).

use core::cell::{Cell, RefCell};

use std::boxed::Box;
use std::sync::MutexGuard;
use std::vec;
use std::vec::Vec;

use hal::intr::IPL_NONE;
use hal::malloc::MArc;
use hal::pool::{BoxPool, PR_WAITOK, PoolArc, TypedPool};
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, MNT_LOCAL, Statfs};
use hal::sys::types::{Ino, Uid};
use hal::user::{pod_from_bytes, pod_to_bytes};

use super::ffs_subr::{ffs_clrblock, ffs_isblock, ffs_setblock};
use super::ffs_vfsops::Ffs;
use super::fs::*;
use crate::fs::bio::{DEV_BSHIFT, biodone};
use crate::fs::buf::{B_READ, BufRef};
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc, vfs_unbusy};
use crate::fs::subr::{bdevvp, getnewvnode, vflushbuf, vput};
use crate::fs::testfs::{TestGlue, set_bdev_strategy, test_thread_cached};
use crate::fs::ufs::ufs::dinode::*;
use crate::fs::ufs::ufs::dir::{
    DIRBLKSIZ, DT_DIR, DT_REG, Direct, d_namlen, d_reclen, dirsiz, set_d_reclen,
};
use crate::fs::ufs::ufs::inode::{
    Dinode, Inode, InodeInit, Ufs1Din, Ufs2Din, dinode1_at, dinode2_at, set_dinode1_at,
    set_dinode2_at, vtoi,
};
use crate::fs::ufs::ufs::testufs::{TEST_DEV, TEST_UFS_VOPS};
use crate::fs::ufs::ufs::ufs_ihash::{ufs_ihashget, ufs_ihashinit, ufs_ihashins};
use crate::fs::ufs::ufs::ufsmount::{UM_UFS1, UM_UFS2, Ufsmount, vfstoufs};
use crate::fs::vnode::{VREG, VT_UFS, Vnode, iftovt};
use crate::fs::vops::{Cred, Nd, Vfs};
use crate::proc::Thread;
use hal::sys::types::Mode;

/// `DEV_BSIZE`.
const DEV_BSIZE: usize = 1 << DEV_BSHIFT;

/// The `int32_t` at byte `off` of `b`.
fn i32_at(b: &[u8], off: usize) -> i32 {
    i32::from_ne_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// `newfs`: an FFS image built in memory the way `newfs(8)` (`sbin/newfs/mkfs.c`) builds one,
/// for the tests and as a record of the on-disk layout.
pub mod newfs {
    use super::*;

    /// The geometry to build: the format, the image size, the block and fragment sizes, the
    /// fragments and inodes per cylinder group.
    #[derive(Clone, Copy, Debug)]
    pub struct Params {
        /// FFS2 (UFS2 dinodes, super-block at 64 KB) or FFS1 (super-block at 8 KB).
        pub ufs2: bool,
        /// The image's size in bytes (a multiple of `fpg * fsize` keeps every group full).
        pub size: usize,
        /// `fs_bsize`.
        pub bsize: i32,
        /// `fs_fsize`.
        pub fsize: i32,
        /// `fs_fpg`.
        pub fpg: i32,
        /// `fs_ipg` (a multiple of `INOPB`).
        pub ipg: u32,
        /// `fs_contigsumsize`: the longest cluster the summary counts, 0 for none (`newfs
        /// -a`'s `maxcontig`, capped at `FS_MAXCONTIG`).
        pub contigsumsize: i32,
    }

    /// FFS2, 4 MB: 8 KB blocks, 1 KB fragments, two cylinder groups of 512 inodes.
    pub const FFS2_4M: Params = Params {
        ufs2: true,
        size: 4 << 20,
        bsize: 8192,
        fsize: 1024,
        fpg: 2048,
        ipg: 512,
        contigsumsize: 0,
    };

    /// FFS1, 4 MB, as `makefs` builds the install ramdisks: 4 KB blocks, 512-byte fragments,
    /// two cylinder groups of 512 inodes.
    pub const FFS1_4M: Params = Params {
        ufs2: false,
        size: 4 << 20,
        bsize: 4096,
        fsize: 512,
        fpg: 4096,
        ipg: 512,
        contigsumsize: 0,
    };

    /// FFS2, 2 MB in one cylinder group: the allocator tests' image.
    pub const FFS2_1G: Params = Params {
        size: 2 << 20,
        ..FFS2_4M
    };

    /// FFS1, 2 MB in one cylinder group.
    pub const FFS1_1G: Params = Params {
        size: 2 << 20,
        ..FFS1_4M
    };

    /// FFS2, 2 MB in one cylinder group, with a cluster summary of runs up to 4 blocks.
    pub const FFS2_1G_CLUSTERS: Params = Params {
        contigsumsize: 4,
        ..FFS2_1G
    };

    /// The fixed time stamp of everything `newfs` writes.
    pub const TIME: i64 = 1_700_000_000;

    /// An image under construction: the bytes, the super-block and each cylinder group
    /// block, which [`Image::finish`] writes into the bytes (the summaries are in the bytes
    /// all along).
    pub struct Image {
        /// The disk.
        pub disk: Vec<u8>,
        /// The super-block.
        fs: Fs,
        /// Each cylinder group block (`fs_cgsize` bytes).
        cgs: Vec<Vec<u8>>,
        /// The next generation number handed out.
        gen_seed: u32,
    }

    /// A dinode of either format, as [`Image::with_dinode`] hands it.
    pub enum Din {
        /// FFS1.
        Ufs1(Ufs1Dinode),
        /// FFS2.
        Ufs2(Ufs2Dinode),
    }

    /// `ilog2`.
    fn ilog2(v: i32) -> i32 {
        31 - v.leading_zeros() as i32
    }

    /// The in-core copy of `fs` (for the kernel's macros and map functions).
    pub fn incore(fs: &Fs) -> Superblock {
        crate::testutil::install();
        let mut b = vec![0u8; size_of::<Fs>()];
        assert!(pod_to_bytes(fs, &mut b));
        Superblock::new(&b)
    }

    impl Image {
        /// The super-block.
        pub fn fs(&self) -> &Fs {
            &self.fs
        }

        /// The view of cylinder group `cg`'s block.
        fn cg(&mut self, cg: u32) -> CgView<'_> {
            CgView::new(&mut self.cgs[cg as usize])
        }

        /// `fs_cs(fs, cg)` field `field` `+= d`, kept in the image's summary area.
        fn cs_add(&mut self, cg: u32, field: usize, d: i32) {
            let off = self.cs_off(cg) + field * 4;
            let v = i32_at(&self.disk, off) + d;
            self.disk[off..off + 4].copy_from_slice(&v.to_ne_bytes());
        }

        /// The byte offset in the image of `fs_cs(fs, cg)`.
        fn cs_off(&self, cg: u32) -> usize {
            (self.fs.fs_csaddr << self.fs.fs_fsbtodb) as usize * DEV_BSIZE + cg as usize * 16
        }

        /// `mkfs` and `fsinit`: an empty file system with a root directory.
        pub fn new(p: Params) -> Self {
            let mut fs = Fs::zeroed();
            let frag = p.bsize / p.fsize;
            fs.fs_postblformat = FS_DYNAMICPOSTBLFMT;
            fs.fs_avgfilesize = AVFILESIZ;
            fs.fs_avgfpdir = AFPDIR;
            fs.fs_bsize = p.bsize;
            fs.fs_fsize = p.fsize;
            fs.fs_bmask = !(p.bsize - 1);
            fs.fs_fmask = !(p.fsize - 1);
            fs.fs_qbmask = i64::from(!fs.fs_bmask);
            fs.fs_qfmask = i64::from(!fs.fs_fmask);
            fs.fs_bshift = ilog2(p.bsize);
            fs.fs_fshift = ilog2(p.fsize);
            fs.fs_frag = frag;
            fs.fs_fragshift = ilog2(frag);
            fs.fs_fsbtodb = ilog2(p.fsize / DEV_BSIZE as i32);
            fs.fs_size = (p.size / p.fsize as usize) as i64;
            fs.fs_nspf = p.fsize / DEV_BSIZE as i32;
            fs.fs_maxcontig = p.contigsumsize.max(1);
            fs.fs_contigsumsize = p.contigsumsize;
            fs.fs_nrpos = 1;
            fs.fs_cpg = 1;
            fs.fs_inodefmt = FS_44INODEFMT;
            if p.ufs2 {
                fs.fs_sblockloc = i64::from(SBLOCK_UFS2);
                fs.fs_nindir = p.bsize / 8;
                fs.fs_inopb = (p.bsize / 256) as u32;
                fs.fs_maxsymlinklen = MAXSYMLINKLEN_UFS2 as i32;
            } else {
                fs.fs_sblockloc = i64::from(SBLOCK_UFS1);
                fs.fs_nindir = p.bsize / 4;
                fs.fs_inopb = (p.bsize / 128) as u32;
                fs.fs_maxsymlinklen = MAXSYMLINKLEN_UFS1 as i32;
                fs.fs_cgoffset = 0;
                fs.fs_cgmask = -1;
                fs.fs_ffs1_size = fs.fs_size as i32;
                fs.fs_rps = 60;
                fs.fs_interleave = 1;
            }
            let div_ceil = |x: i64, y: i64| (x + y - 1) / y;
            let roundup = |x: i64, y: i64| div_ceil(x, y) * y;
            fs.fs_sblkno = roundup(
                div_ceil(fs.fs_sblockloc + SBLOCKSIZE as i64, i64::from(p.fsize)),
                i64::from(frag),
            ) as i32;
            fs.fs_cblkno = fs.fs_sblkno
                + roundup(div_ceil(SBSIZE as i64, i64::from(p.fsize)), i64::from(frag)) as i32;
            fs.fs_iblkno = fs.fs_cblkno + frag;
            let mut maxfilesize = p.bsize as u64 * NDADDR as u64 - 1;
            let mut sizepb = p.bsize as u64;
            for _ in 0..NIADDR {
                sizepb *= fs.fs_nindir as u64;
                maxfilesize += sizepb;
            }
            fs.fs_maxfilesize = maxfilesize;
            fs.fs_fpg = p.fpg;
            fs.fs_ipg = p.ipg;
            let ncg = (fs.fs_size as usize).div_ceil(p.fpg as usize) as u32;
            fs.fs_ncg = ncg;
            if !p.ufs2 {
                fs.fs_spc = p.fpg * fs.fs_nspf;
                fs.fs_nsect = fs.fs_spc;
                fs.fs_npsect = fs.fs_spc;
                fs.fs_ncyl = ncg as i32;
            }
            fs.fs_cssize = roundup((ncg as usize * 16) as i64, i64::from(p.fsize)) as i32;
            {
                let sb = incore(&fs);
                fs.fs_cgsize = fragroundup(&sb, cgsize(&sb) as i64) as i32;
                fs.fs_dblkno = fs.fs_iblkno + (p.ipg / inopf(&sb)) as i32;
                fs.fs_csaddr = cgstart(&sb, 0) + i64::from(fs.fs_dblkno);
                fs.fs_sbsize =
                    fragroundup(&sb, size_of::<Fs>() as i64).min(SBLOCKSIZE as i64) as i32;
            }
            fs.fs_minfree = MINFREE;
            fs.fs_maxbpg = if p.ufs2 { p.bsize / 8 } else { p.bsize / 4 };
            fs.fs_optim = FS_OPTTIME;
            fs.fs_clean = 1;
            fs.fs_id = [TIME as i32, 0x0eb5_d00d];

            let csfrags = (fs.fs_cssize as usize).div_ceil(p.fsize as usize) as i64;
            let dsize = fs.fs_size
                - i64::from(fs.fs_sblkno)
                - i64::from(ncg) * i64::from(fs.fs_dblkno - fs.fs_sblkno);
            fs.fs_dsize = dsize;
            fs.fs_cstotal.cs_nbfree =
                (dsize >> fs.fs_fragshift) - div_ceil(csfrags, i64::from(frag));
            let fragnum = |x: i64| x & i64::from(frag - 1);
            fs.fs_cstotal.cs_nffree = fragnum(fs.fs_size)
                + if fragnum(csfrags) > 0 {
                    i64::from(frag) - fragnum(csfrags)
                } else {
                    0
                };
            fs.fs_cstotal.cs_nifree = i64::from(ncg * p.ipg - ROOTINO);
            fs.fs_cstotal.cs_ndir = 0;
            fs.fs_dsize = dsize - csfrags;
            fs.fs_time = TIME;
            fs.fs_magic = if p.ufs2 { FS_UFS2_MAGIC } else { FS_UFS1_MAGIC };

            let mut img = Image {
                disk: vec![0u8; p.size],
                fs,
                cgs: Vec::new(),
                gen_seed: 0x1234_5678,
            };
            for cg in 0..ncg {
                img.initcg(cg, p);
            }
            img.fsinit();
            img
        }

        /// `initcg`: initialize a cylinder group.
        fn initcg(&mut self, cg: u32, p: Params) {
            let sb = incore(&self.fs);
            let ufs2 = p.ufs2;
            let frag = self.fs.fs_frag;
            let ipg = self.fs.fs_ipg;
            let cbase = cgbase(&sb, cg);
            let dmax = (cbase + i64::from(self.fs.fs_fpg)).min(self.fs.fs_size);
            let dlower = cgsblock(&sb, cg) - cbase;
            let mut dupper = cgdmin(&sb, cg) - cbase;
            if cg == 0 {
                dupper += (self.fs.fs_cssize as usize).div_ceil(self.fs.fs_fsize as usize) as i64;
            }
            let inopb = self.fs.fs_inopb;
            let cpg = self.fs.fs_cpg;
            let ncg = self.fs.fs_ncg;
            let fpg = self.fs.fs_fpg;
            let start = size_of::<Cg>() as u32;
            let mut c = Cg {
                cg_ffs2_time: TIME,
                cg_magic: CG_MAGIC,
                cg_cgx: cg,
                cg_ffs2_niblk: ipg,
                cg_initediblk: ipg.min(2 * inopb),
                cg_ndblk: (dmax - cbase) as u32,
                ..Cg::default()
            };
            if !ufs2 {
                // Hack to maintain compatibility with old fsck.
                c.cg_ncyl = if cg == ncg - 1 { 0 } else { cpg as i16 };
                c.cg_time = TIME as i32;
                c.cg_ffs2_time = 0;
                c.cg_niblk = c.cg_ffs2_niblk as i16;
                c.cg_ffs2_niblk = 0;
                c.cg_initediblk = 0;
                c.cg_btotoff = start as i32;
                c.cg_boff = c.cg_btotoff + cpg * 4;
                c.cg_iusedoff = (c.cg_boff + cpg * 2) as u32;
            } else {
                c.cg_iusedoff = start;
            }
            c.cg_freeoff = c.cg_iusedoff + (ipg as usize).div_ceil(8) as u32;
            c.cg_nextfreeoff = c.cg_freeoff + (fpg as usize).div_ceil(8) as u32;
            let contig = self.fs.fs_contigsumsize;
            if contig > 0 {
                // mkfs: the cluster summary after the fragment map (int32_t aligned), then
                // the map of free blocks.
                c.cg_clustersumoff = (c.cg_nextfreeoff - 4).next_multiple_of(4);
                c.cg_clusteroff = c.cg_clustersumoff + (contig as u32 + 1) * 4;
                c.cg_nextfreeoff =
                    c.cg_clusteroff + ((fpg / self.fs.fs_frag) as usize).div_ceil(8) as u32;
                c.cg_nclusterblks = c.cg_ndblk / self.fs.fs_frag as u32;
            }
            c.cg_cs.cs_nifree = ipg as i32;
            let mut blk = vec![0u8; self.fs.fs_cgsize as usize];
            assert!(pod_to_bytes(&c, &mut blk));
            self.cgs.push(blk);
            let mut v = self.cg(cg);
            if cg == 0 {
                for i in 0..ROOTINO as usize {
                    setbit(v.cg_inosused(), i);
                    v.add_cg_cs_nifree(-1);
                }
            }
            let blocks_free = |v: &mut CgView<'_>, d: i64| {
                ffs_setblock(&sb, v.cg_blksfree(), d / i64::from(frag));
                if contig > 0 {
                    setbit(v.cg_clustersfree(), (d / i64::from(frag)) as usize);
                }
                v.add_cg_cs_nbfree(1);
                if !ufs2 {
                    v.add_cg_blktot(0, 1);
                    v.add_cg_blks(&sb, 0, 0, 1);
                }
            };
            if cg > 0 {
                // In cg 0, space is reserved for boot and super blocks.
                let mut d = 0;
                while d < dlower {
                    blocks_free(&mut v, d);
                    d += i64::from(frag);
                }
            }
            let r = dupper % i64::from(frag);
            if r != 0 {
                v.add_cg_frsum((i64::from(frag) - r) as usize, 1);
                let end = dupper + i64::from(frag) - r;
                while dupper < end {
                    setbit(v.cg_blksfree(), dupper as usize);
                    v.add_cg_cs_nffree(1);
                    dupper += 1;
                }
            }
            let ndblk = i64::from(v.cg_ndblk());
            let mut d = dupper;
            while d + i64::from(frag) <= ndblk {
                blocks_free(&mut v, d);
                d += i64::from(frag);
            }
            if d < ndblk {
                v.add_cg_frsum((ndblk - d) as usize, 1);
                while d < ndblk {
                    setbit(v.cg_blksfree(), d as usize);
                    v.add_cg_cs_nffree(1);
                    d += 1;
                }
            }
            // *cs = acg.cg_cs
            let cs = [
                v.cg_cs_ndir(),
                v.cg_cs_nbfree(),
                v.cg_cs_nifree(),
                v.cg_cs_nffree(),
            ];
            for (i, x) in cs.iter().enumerate() {
                self.cs_add(cg, i, *x);
            }

            // Generation numbers for the inodes newfs initialises: the first two blocks of
            // each group for FFS2 (the rest lazily, ffs_nodealloccg), all of them for FFS1.
            let n = if ufs2 { ipg.min(2 * inopb) } else { ipg };
            for i in 0..n {
                let ino = cg * ipg + i;
                let g = self.next_gen();
                self.with_dinode(ino, |d| match d {
                    Din::Ufs1(d) => d.di_gen = g,
                    Din::Ufs2(d) => d.di_gen = g as i32,
                });
            }
        }

        /// A fresh nonzero generation number.
        fn next_gen(&mut self) -> u32 {
            self.gen_seed = self
                .gen_seed
                .wrapping_mul(1_103_515_245)
                .wrapping_add(12345)
                | 1;
            self.gen_seed
        }

        /// `f` on the dinode of `ino` in the image.
        pub fn with_dinode(&mut self, ino: Ufsino, f: impl FnOnce(&mut Din)) {
            let sb = incore(&self.fs);
            let blk = fsbtodb(&sb, ino_to_fsba(&sb, ino)) as usize * DEV_BSIZE;
            let idx = ino_to_fsbo(&sb, ino);
            let bsize = self.fs.fs_bsize as usize;
            let b = &mut self.disk[blk..blk + bsize];
            if self.fs.fs_magic == FS_UFS2_MAGIC {
                let mut d = Din::Ufs2(dinode2_at(b, idx));
                f(&mut d);
                if let Din::Ufs2(d) = d {
                    set_dinode2_at(b, idx, &d);
                }
            } else {
                let mut d = Din::Ufs1(dinode1_at(b, idx));
                f(&mut d);
                if let Din::Ufs1(d) = d {
                    set_dinode1_at(b, idx, &d);
                }
            }
        }

        /// `alloc(size, mode)`: a block (or its first fragments) in cylinder group 0.
        fn alloc(&mut self, size: i32, mode: Mode) -> i64 {
            let sb = incore(&self.fs);
            let frag = self.fs.fs_frag;
            let fsize = self.fs.fs_fsize;
            let bsize = self.fs.fs_bsize;
            let ufs2 = self.fs.fs_magic == FS_UFS2_MAGIC;
            let contig = self.fs.fs_contigsumsize;
            let mut v = self.cg(0);
            let ndblk = i64::from(v.cg_ndblk());
            let mut d = 0;
            while d < ndblk {
                if ffs_isblock(&sb, v.cg_blksfree(), d / i64::from(frag)) {
                    break;
                }
                d += i64::from(frag);
            }
            assert!(d < ndblk, "newfs: cg 0 is full");
            ffs_clrblock(&sb, v.cg_blksfree(), d / i64::from(frag));
            if contig > 0 {
                clrbit(v.cg_clustersfree(), (d / i64::from(frag)) as usize);
            }
            v.add_cg_cs_nbfree(-1);
            if mode & IFMT == IFDIR {
                v.add_cg_cs_ndir(1);
            }
            if !ufs2 {
                v.add_cg_blktot(0, -1);
                v.add_cg_blks(&sb, 0, 0, -1);
            }
            let used = (size as usize).div_ceil(fsize as usize) as i32;
            if size != bsize {
                v.add_cg_cs_nffree(frag - used);
                v.add_cg_frsum((frag - used) as usize, 1);
                for i in used..frag {
                    setbit(v.cg_blksfree(), (d + i64::from(i)) as usize);
                }
            }
            self.fs.fs_cstotal.cs_nbfree -= 1;
            self.cs_add(0, 1, -1);
            if mode & IFMT == IFDIR {
                self.fs.fs_cstotal.cs_ndir += 1;
                self.cs_add(0, 0, 1);
            }
            if size != bsize {
                self.fs.fs_cstotal.cs_nffree += i64::from(frag - used);
                self.cs_add(0, 3, frag - used);
            }
            d
        }

        /// `iput`: allocate inode `ino` in the map and write its dinode.
        fn iput(&mut self, ino: Ufsino, f: impl FnOnce(&mut Din)) {
            let mut v = self.cg(0);
            v.add_cg_cs_nifree(-1);
            setbit(v.cg_inosused(), ino as usize);
            self.fs.fs_cstotal.cs_nifree -= 1;
            self.cs_add(0, 2, -1);
            let g = self.next_gen();
            self.with_dinode(ino, |d| {
                match d {
                    Din::Ufs1(d) => d.di_gen = g,
                    Din::Ufs2(d) => d.di_gen = g as i32,
                }
                f(d);
            });
        }

        /// The byte offset in the image of fragment `frag`.
        pub fn frag_off(&self, frag: i64) -> usize {
            (frag << self.fs.fs_fsbtodb) as usize * DEV_BSIZE
        }

        /// `fsinit`: the root directory (`.` and `..` in one `DIRBLKSIZ` block).
        fn fsinit(&mut self) {
            let fsize = self.fs.fs_fsize;
            let mode = IFDIR | 0o755;
            let db0 = self.alloc(fsize, mode);
            let off = self.frag_off(db0);
            let mut dot = Direct::new();
            dot.d_ino = ROOTINO;
            dot.d_type = DT_DIR;
            dot.d_namlen = 1;
            dot.d_name[0] = b'.';
            dot.d_reclen = dirsiz(1) as u16;
            dot.write_to(&mut self.disk, off);
            let mut dotdot = dot;
            dotdot.d_namlen = 2;
            dotdot.d_name[1] = b'.';
            dotdot.d_reclen = (DIRBLKSIZ - dirsiz(1)) as u16;
            dotdot.write_to(&mut self.disk, off + dirsiz(1));
            let blocks = DIRBLKSIZ.next_multiple_of(fsize as usize) / DEV_BSIZE;
            self.iput(ROOTINO, |d| match d {
                Din::Ufs1(d) => {
                    d.di_mode = mode as u16;
                    d.di_nlink = 2;
                    d.di_size = DIRBLKSIZ as u64;
                    d.di_db[0] = db0 as i32;
                    d.di_blocks = blocks as i32;
                    (d.di_atime, d.di_mtime, d.di_ctime) = (TIME as i32, TIME as i32, TIME as i32);
                }
                Din::Ufs2(d) => {
                    d.di_mode = mode as u16;
                    d.di_nlink = 2;
                    d.di_size = DIRBLKSIZ as u64;
                    d.di_db[0] = db0;
                    d.di_blocks = blocks as u64;
                    (d.di_atime, d.di_mtime, d.di_ctime) = (TIME, TIME, TIME);
                }
            });
        }

        /// A regular file `name` holding `data` in the root directory (direct blocks only;
        /// the last block is fragments when it is the file's only partial one): its inode.
        pub fn add_file(&mut self, name: &[u8], data: &[u8]) -> Ufsino {
            let bsize = self.fs.fs_bsize as usize;
            let nblk = data.len().div_ceil(bsize);
            assert!(nblk <= NDADDR, "newfs: add_file is direct blocks only");
            let ipg = self.fs.fs_ipg;
            // The next free inode of cylinder group 0.
            let mut v = self.cg(0);
            let map = v.cg_inosused();
            let Some(ino) = (0..ipg).find(|&i| !isset(map, i as usize)) else {
                std::panic!("newfs: cg 0 has no free inode");
            };
            let mut db = [0i64; NDADDR];
            let mut used = 0i64;
            for (b, chunk) in data.chunks(bsize).enumerate() {
                let size = if chunk.len() == bsize {
                    bsize as i32
                } else {
                    chunk.len().next_multiple_of(self.fs.fs_fsize as usize) as i32
                };
                db[b] = self.alloc(size, IFREG);
                used += i64::from(size);
                let off = self.frag_off(db[b]);
                self.disk[off..off + chunk.len()].copy_from_slice(chunk);
            }
            let mode = IFREG | 0o644;
            let blocks = used / DEV_BSIZE as i64;
            self.iput(ino, |d| match d {
                Din::Ufs1(d) => {
                    d.di_mode = mode as u16;
                    d.di_nlink = 1;
                    d.di_size = data.len() as u64;
                    for (i, b) in db.iter().enumerate() {
                        d.di_db[i] = *b as i32;
                    }
                    d.di_blocks = blocks as i32;
                    (d.di_atime, d.di_mtime, d.di_ctime) = (TIME as i32, TIME as i32, TIME as i32);
                }
                Din::Ufs2(d) => {
                    d.di_mode = mode as u16;
                    d.di_nlink = 1;
                    d.di_size = data.len() as u64;
                    d.di_db = db;
                    d.di_blocks = blocks as u64;
                    (d.di_atime, d.di_mtime, d.di_ctime) = (TIME, TIME, TIME);
                }
            });
            self.dir_add(name, ino, DT_REG);
            ino
        }

        /// Adds an entry to the root directory's block, splitting the last entry's space.
        fn dir_add(&mut self, name: &[u8], ino: Ufsino, dtype: u8) {
            let mut root_db0 = 0;
            self.with_dinode(ROOTINO, |d| {
                root_db0 = match d {
                    Din::Ufs1(d) => i64::from(d.di_db[0]),
                    Din::Ufs2(d) => d.di_db[0],
                }
            });
            let base = self.frag_off(root_db0);
            let blk = &mut self.disk[base..base + DIRBLKSIZ];
            let mut off = 0;
            loop {
                let reclen = usize::from(d_reclen(blk, off));
                if off + reclen >= DIRBLKSIZ {
                    break;
                }
                off += reclen;
            }
            let used = dirsiz(d_namlen(blk, off));
            let reclen = usize::from(d_reclen(blk, off));
            assert!(
                reclen - used >= dirsiz(name.len() as u8),
                "newfs: root directory full"
            );
            set_d_reclen(blk, off, used as u16);
            let mut e = Direct::new();
            e.d_ino = ino;
            e.d_type = dtype;
            e.d_namlen = name.len() as u8;
            e.d_name[..name.len()].copy_from_slice(name);
            e.d_reclen = (reclen - used) as u16;
            e.write_to(blk, off + used);
        }

        /// Writes the super-block (and its copy in each group) and the cylinder groups into
        /// the image, and returns it.
        pub fn finish(mut self) -> Vec<u8> {
            let fs = &mut self.fs;
            if fs.fs_magic == FS_UFS1_MAGIC {
                fs.fs_ffs1_time = fs.fs_time as i32;
                fs.fs_ffs1_dsize = fs.fs_dsize as i32;
                fs.fs_ffs1_csaddr = fs.fs_csaddr as i32;
                fs.fs_ffs1_cstotal = Csum {
                    cs_ndir: fs.fs_cstotal.cs_ndir as i32,
                    cs_nbfree: fs.fs_cstotal.cs_nbfree as i32,
                    cs_nifree: fs.fs_cstotal.cs_nifree as i32,
                    cs_nffree: fs.fs_cstotal.cs_nffree as i32,
                };
            }
            // The cluster summaries from the final maps (mkfs counts them at initcg's end;
            // its later allocations, the root directory's, leave them as they were).
            let contig = self.fs.fs_contigsumsize;
            if contig > 0 {
                for cg in 0..self.fs.fs_ncg {
                    let mut v = self.cg(cg);
                    let n = v.cg_nclusterblks() as usize;
                    let mut run = 0;
                    for i in 0..=n {
                        if i < n && isset(v.cg_clustersfree(), i) {
                            run += 1;
                        } else if run > 0 {
                            v.add_cg_clustersum(run.min(contig as usize), 1);
                            run = 0;
                        }
                    }
                }
            }
            let sb = incore(&self.fs);
            let mut sbb = vec![0u8; SBLOCKSIZE];
            assert!(pod_to_bytes(&self.fs, &mut sbb));
            let loc = self.fs.fs_sblockloc as usize;
            self.disk[loc..loc + SBLOCKSIZE].copy_from_slice(&sbb);
            for cg in 0..self.fs.fs_ncg {
                let sboff = self.frag_off(cgsblock(&sb, cg));
                let cgoff = self.frag_off(cgtod(&sb, cg));
                self.disk[sboff..sboff + SBLOCKSIZE].copy_from_slice(&sbb);
                let blk = &self.cgs[cg as usize];
                self.disk[cgoff..cgoff + blk.len()].copy_from_slice(blk);
            }
            self.disk
        }
    }

    /// The super-block of an image (at the format's offset).
    pub fn super_block(disk: &[u8], ufs2: bool) -> Fs {
        let loc = if ufs2 { SBLOCK_UFS2 } else { SBLOCK_UFS1 } as usize;
        match pod_from_bytes(&disk[loc..]) {
            Some(fs) => fs,
            None => std::panic!("newfs: no super-block at {loc}"),
        }
    }

    /// `fsck`'s counter check: the counters of each cylinder group against its maps, the
    /// summaries `sums(cg)` (`cs_ndir`, `cs_nbfree`, `cs_nifree`, `cs_nffree`) against them,
    /// and their sum against `totals` (the same order). Panics on a mismatch.
    pub fn check_with(disk: &[u8], fs: &Fs, sums: impl Fn(u32) -> [i32; 4], totals: [i64; 4]) {
        let sb = incore(fs);
        let frag = fs.fs_frag;
        let (mut tb, mut tf, mut ti, mut td) = (0i64, 0i64, 0i64, 0i64);
        for cg in 0..fs.fs_ncg {
            let off = fsbtodb(&sb, cgtod(&sb, cg)) as usize * DEV_BSIZE;
            let mut blk = disk[off..off + fs.fs_cgsize as usize].to_vec();
            let mut v = CgView::new(&mut blk);
            assert_eq!(v.cg_magic(), CG_MAGIC, "cg {cg} magic");
            let ndblk = i64::from(v.cg_ndblk());
            let (mut nb, mut nf) = (0, 0);
            let mut d = 0;
            while d < ndblk {
                let free = v.cg_blksfree();
                if ffs_isblock(&sb, free, d / i64::from(frag)) {
                    nb += 1;
                } else {
                    nf += (0..i64::from(frag))
                        .filter(|&i| d + i < ndblk && isset(free, (d + i) as usize))
                        .count() as i32;
                }
                d += i64::from(frag);
            }
            let used = v.cg_inosused();
            let ni = (0..fs.fs_ipg).filter(|&i| !isset(used, i as usize)).count() as i32;
            let nd = v.cg_cs_ndir();
            if fs.fs_contigsumsize > 0 {
                // The cluster summary against the cluster map's runs.
                let contig = fs.fs_contigsumsize as usize;
                let n = v.cg_nclusterblks() as usize;
                let mut want = [0i32; FS_MAXCONTIG + 1];
                let mut run = 0;
                for i in 0..=n {
                    if i < n && isset(v.cg_clustersfree(), i) {
                        run += 1;
                    } else if run > 0 {
                        want[run.min(contig)] += 1;
                        run = 0;
                    }
                }
                for (k, w) in want.iter().enumerate().take(contig + 1).skip(1) {
                    assert_eq!(v.cg_clustersum(k), *w, "cg {cg} clustersum[{k}]");
                }
            }
            assert_eq!(v.cg_cs_nbfree(), nb, "cg {cg} nbfree");
            assert_eq!(v.cg_cs_nffree(), nf, "cg {cg} nffree");
            assert_eq!(v.cg_cs_nifree(), ni, "cg {cg} nifree");
            assert_eq!(sums(cg), [nd, nb, ni, nf], "cg {cg} summary");
            tb += i64::from(nb);
            tf += i64::from(nf);
            ti += i64::from(ni);
            td += i64::from(nd);
        }
        assert_eq!(totals, [td, tb, ti, tf], "totals");
    }

    /// [`check_with`] over the image's own summaries and super-block (a `newfs` image, or
    /// one an unmount wrote back).
    pub fn check(disk: &[u8], ufs2: bool) {
        let fs = super_block(disk, ufs2);
        let cs_off = (fs.fs_csaddr << fs.fs_fsbtodb) as usize * DEV_BSIZE;
        let sums = |cg: u32| {
            core::array::from_fn(|k| {
                let o = cs_off + cg as usize * 16 + k * 4;
                i32_at(disk, o)
            })
        };
        let t = fs.fs_cstotal;
        check_with(
            disk,
            &fs,
            sums,
            [t.cs_ndir, t.cs_nbfree, t.cs_nifree, t.cs_nffree],
        );
    }
}

/// `ffs_ino_pool` of the fixture.
static INODES: TypedPool<Inode<TestGlue>> = TypedPool::new();
/// `ffs_dinode1_pool`.
static DINO1: BoxPool<Ufs1Din> = BoxPool::new();
/// `ffs_dinode2_pool`.
static DINO2: BoxPool<Ufs2Din> = BoxPool::new();

std::thread_local! {
    /// The disk's bytes.
    static DISK: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// The mount's device vnode.
    static DEVVP: Cell<Option<&'static Vnode<TestGlue>>> = const { Cell::new(None) };
}

/// The fixture's `vfsops`: `vget` and `root`, the rest `EOPNOTSUPP` (N5.5b2's are `Ffs`'s).
struct TestFfsVfs;

/// The fixture's `vfsops`.
static TEST_FFS_VFS: TestFfsVfs = TestFfsVfs;

/// The fixture's configuration entry.
static TEST_FFS_CONF: Vfsconf<TestGlue> = Vfsconf::new(&TEST_FFS_VFS, b"testffs", 97, MNT_LOCAL, 0);

/// The disk's bytes as they are now (the cache's delayed writes not included: [`sync`]).
pub fn disk() -> Vec<u8> {
    DISK.with(|d| d.borrow().clone())
}

/// The mount's device vnode.
pub fn devvp() -> &'static Vnode<TestGlue> {
    match DEVVP.with(Cell::get) {
        Some(vp) => vp,
        None => std::panic!("testffs: not mounted"),
    }
}

/// Writes the delayed writes of `vps` (files whose indirect blocks are in their own
/// buffers) and of the device out to the disk.
pub fn sync(vps: &[&'static Vnode<TestGlue>]) {
    for vp in vps {
        vflushbuf(vp, true);
    }
    vflushbuf(devvp(), true);
}

/// A test thread over fresh VFS tables with page memory and a buffer cache, and `image`
/// mounted (not on the mount list): `ffs_mountfs`'s steps as N5.5b1 has them (the
/// super-block of the image's format, its summaries, `fs_contigdirs` of a read-write
/// mount, the mount's UFS structure in its slot). The guard of the kernel's memory, the
/// thread, the mount.
pub fn setup(
    image: Vec<u8>,
) -> (
    MutexGuard<'static, ()>,
    &'static Thread<TestGlue>,
    &'static Mount<TestGlue>,
) {
    let (g, p) = test_thread_cached(1 << 22, 192);
    let _ = INODES.init_recorded(IPL_NONE, PR_WAITOK, "ffsino");
    let _ = DINO1.init(IPL_NONE, PR_WAITOK, "dino1pl");
    let _ = DINO2.init(IPL_NONE, PR_WAITOK, "dino2pl");
    ufs_ihashinit::<TestGlue>(64);

    // The super-block: SBLOCKSEARCH's first two places.
    let (loc, ufs2) = [(SBLOCK_UFS2, true), (SBLOCK_UFS1, false)]
        .into_iter()
        .map(|(l, u)| (l as usize, u))
        .find(|&(l, u)| {
            let m = i32_at(&image, l + core::mem::offset_of!(Fs, fs_magic));
            m == if u { FS_UFS2_MAGIC } else { FS_UFS1_MAGIC }
        })
        .unwrap_or_else(|| std::panic!("testffs: no super-block"));
    let fs = newfs::super_block(&image, ufs2);
    let sbsize = fs.fs_sbsize as usize;
    let sb = Superblock::new(&image[loc..loc + sbsize]);
    let cs = (fs.fs_csaddr << fs.fs_fsbtodb) as usize * DEV_BSIZE;
    sb.write_csum_bytes(0, &image[cs..cs + sb.csum_len()]);
    sb.set_fs_ronly(0);
    sb.contigdirs_alloc();
    DISK.with(|d| *d.borrow_mut() = image);

    let devvp = match bdevvp::<TestGlue>(TEST_DEV) {
        Ok(Some(vp)) => vp,
        Ok(None) => std::panic!("testffs: no device vnode"),
        Err(e) => std::panic!("testffs: no device vnode: {e:?}"),
    };
    set_bdev_strategy(Some(disk_strategy));
    DEVVP.with(|c| c.set(Some(devvp)));
    let mp = vfs_mount_alloc(None, &TEST_FFS_CONF);
    vfs_unbusy(mp);
    mp.with_stat(|st| {
        st.f_iosize = fs.fs_bsize as u32;
        st.f_bsize = fs.fs_fsize as u32;
    });
    let ump = Ufsmount::<TestGlue>::alloc();
    ump.um_mountp.set(Some(mp));
    ump.um_dev.set(TEST_DEV);
    ump.um_devvp.set(Some(devvp));
    ump.um_fstype.set(if ufs2 { UM_UFS2 } else { UM_UFS1 });
    ump.um_nindir.set(fs.fs_nindir as u64);
    ump.um_bptrtodb.set(fs.fs_fsbtodb as u64);
    ump.um_seqinc.set(fs.fs_frag as u64);
    ump.um_maxsymlinklen.set(fs.fs_maxsymlinklen as u32);
    ump.um_savedmaxfilesize.set(fs.fs_maxfilesize);
    ump.set_um_fs(sb);
    mp.set_node(MArc::erase(ump));
    (g, p, mp)
}

/// `ffs_vget`'s steps (N5.5b2 brings the real one): the vnode of inode `ino`, locked and
/// referenced: the inode hash, a new vnode, the inode with the kernel's `Ffs` as its
/// operations, its dinode read from its block, `ufs_ihashins`.
pub fn vget(mp: &'static Mount<TestGlue>, ino: Ufsino) -> Result<&'static Vnode<TestGlue>, Errno> {
    if let Some(vp) = ufs_ihashget::<TestGlue>(TEST_DEV, ino) {
        return Ok(vp);
    }
    let ump = vfstoufs(mp);
    let fs = ump.um_fs();
    let vp = getnewvnode(VT_UFS, Some(mp), &TEST_UFS_VOPS)?;
    let ufs2 = ump.um_fstype.get() == UM_UFS2;
    let Some(ip) = INODES.get_homed(PR_WAITOK, || {
        Inode::new(InodeInit {
            vnode: Some(vp),
            mount: Some(mp),
            ump: Some(ump.clone()),
            devvp: Some(devvp()),
            dev: TEST_DEV,
            number: ino,
            fstype: ump.um_fstype.get(),
            nindir: ump.um_nindir.get(),
            bptrtodb: ump.um_bptrtodb.get(),
            seqinc: ump.um_seqinc.get(),
            vtbl: Some(&Ffs::<TestGlue>::VTBL),
            ..InodeInit::default()
        })
    }) else {
        return Err(Errno::ENOMEM);
    };
    // The dinode, from the image (the device's buffer if the cache holds it).
    let (mut t, r) = crate::fs::bio::bread(
        &devvp().bufs,
        fsbtodb(fs, ino_to_fsba(fs, ino)),
        fs.fs_bsize(),
    );
    r?;
    let idx = ino_to_fsbo(fs, ino);
    let (mode, nlink) = if ufs2 {
        let d = dinode2_at(t.data(), idx);
        let Some(din) = DINO2.get(PR_WAITOK, || Ufs2Din::new(&d)) else {
            return Err(Errno::ENOMEM);
        };
        ip.set_din(Dinode::Ufs2(din));
        (Mode::from(d.di_mode), i32::from(d.di_nlink))
    } else {
        let d = dinode1_at(t.data(), idx);
        let Some(din) = DINO1.get(PR_WAITOK, || Ufs1Din::new(&d)) else {
            return Err(Errno::ENOMEM);
        };
        ip.set_din(Dinode::Ufs1(din));
        (Mode::from(d.di_mode), i32::from(d.di_nlink))
    };
    crate::fs::bio::brelse(t);
    ip.i_effnlink.set(nlink);
    vp.set_node(PoolArc::erase(ip.clone()));
    let _ = ufs_ihashins(ip.as_pin());
    vp.set_vtype(iftovt(mode));
    Ok(vp)
}

impl Vfs<TestGlue> for TestFfsVfs {
    fn mount(
        &self,
        _: &'static Mount<TestGlue>,
        _: &[u8],
        _: &mut [u8],
        _: &mut Nd<'_, TestGlue>,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn start(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn unmount(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn root(&self, mp: &'static Mount<TestGlue>) -> Result<&'static Vnode<TestGlue>, Errno> {
        vget(mp, ROOTINO)
    }
    fn quotactl(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: Uid,
        _: usize,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn statfs(
        &self,
        _: &'static Mount<TestGlue>,
        _: &mut Statfs,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn sync(
        &self,
        _: &'static Mount<TestGlue>,
        _: i32,
        _: i32,
        _: Cred<'_>,
        _: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn vget(
        &self,
        mp: &'static Mount<TestGlue>,
        ino: Ino,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        vget(mp, ino as Ufsino)
    }
    fn fhtovp(
        &self,
        _: &'static Mount<TestGlue>,
        _: &Fid,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn vptofh(&self, _: &'static Vnode<TestGlue>, _: &mut Fid) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
}

/// The disk's `d_strategy`: the buffer's bytes in or out of the image at `b_blkno`, then
/// `biodone`.
fn disk_strategy(mut t: BufRef) {
    {
        let at = t.buf().blkno() as usize * DEV_BSIZE;
        let n = t.buf().bcount() as usize;
        if t.buf().isset(B_READ) {
            let data = t.data();
            DISK.with(|d| data[..n].copy_from_slice(&d.borrow()[at..at + n]));
        } else {
            let b = t.bytes()[..n].to_vec();
            DISK.with(|d| d.borrow_mut()[at..at + n].copy_from_slice(&b));
        }
    }
    biodone(t);
}

/// A new regular file in the root directory's group: an inode of `ffs_inode_alloc` made
/// `IFREG | 0644` with one link (what `ufs_makeinode` does next), its vnode locked and
/// referenced (`vput` it).
pub fn new_file(mp: &'static Mount<TestGlue>, p: &Thread<TestGlue>) -> &'static Vnode<TestGlue> {
    let root = match vget(mp, ROOTINO) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testffs: no root: {e:?}"),
    };
    let rip = vtoi(root);
    let vp = match super::ffs_alloc::ffs_inode_alloc(&rip, IFREG | 0o644, Cred::real(p.ucred())) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testffs: ffs_inode_alloc: {e:?}"),
    };
    drop(rip);
    vput(root);
    let ip = vtoi(vp);
    ip.dip_set_mode(IFREG | 0o644);
    ip.dip_set_nlink(1);
    ip.i_effnlink.set(1);
    vp.set_vtype(VREG);
    vp
}

/// A block of `n` bytes of the pattern `seed` (each byte its index plus the seed).
pub fn pattern(n: usize, seed: u8) -> Box<[u8]> {
    (0..n).map(|i| (i as u8).wrapping_add(seed)).collect()
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::newfs::*;
    use super::*;

    #[test]
    fn newfs_images_are_consistent() {
        for p in [FFS2_4M, FFS1_4M, FFS2_1G, FFS1_1G, FFS2_1G_CLUSTERS] {
            let mut img = Image::new(p);
            img.add_file(b"a", &pattern(3000, 1));
            img.add_file(b"b", &pattern(20000, 2));
            let disk = img.finish();
            check(&disk, p.ufs2);
            let fs = super_block(&disk, p.ufs2);
            std::assert_eq!(
                fs.fs_ncg,
                (p.size / (p.fpg as usize * p.fsize as usize)) as u32
            );
            std::assert_eq!(fs.fs_cstotal.cs_ndir, 1, "the root directory");
        }
    }
}
/* </TESTS> */
