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
//! The kernel ext2fs's test fixtures (host tests and benches only, `cfg(any(test, feature =
//! "doubles"))`; not an OpenBSD file): [`newfs`], the image builder (a small ext2 file system
//! of one block group, `newfs_ext2fs(8)`'s layout in small: the super-block, the group
//! descriptor, the bitmaps, the inode table, a root directory and three files), lifted from
//! `bsd`'s `ext2fs_vfsops.rs` tests; and the device half, testffs's: the image is the disk of
//! testfs's block device ([`attach`], `testffs::disk`, `testffs::sync`).
//!
//! Until the kernel's `ext2fs_mountfs` exists (N5.6c3), [`superblock`] reads the super-block
//! in the way `ext2fs_mountfs` does (its geometry computed as `e2fs_sbfill` computes it, the
//! descriptors read) and [`ufsmount`] and [`detached`] make the mount's UFS structure and an
//! inode of it.

use std::boxed::Box;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::intr::IPL_NONE;
use hal::pool::{BoxPool, PR_WAITOK};
use hal::sys::types::Daddr;

use super::ext2fs::{
    E2FS_REV0, EXT2_GD_SIZE, EXT2F_INCOMPAT_EXTENTS, EXT2F_ROCOMPAT_LARGE_FILE, Ext2Superblock,
    LOG_MINBSIZE, SBLOCK, SBSIZE, e2fs_cgload, e2fs_sbload, fsbtodb, nindir,
};
use super::ext2fs_dinode::Ext2Inode;
use crate::fs::bio::{bread, brelse};
use crate::fs::testfs::TestGlue;
use crate::fs::ufs::ffs::testffs;
use crate::fs::ufs::ufs::dinode::Ufsino;
use crate::fs::ufs::ufs::inode::{Dinode, Inode, InodeInit};
use crate::fs::ufs::ufs::testufs::TEST_DEV;
use crate::fs::ufs::ufs::ufsmount::{UM_EXT2FS, Ufsmount, UfsmountRef};
use crate::proc::Thread;

/// `newfs`: the test ext2 file system, built in memory, 1 KiB blocks, one block group.
pub mod newfs {
    use core::mem::offset_of;

    use std::vec;
    use std::vec::Vec;

    use hal::user::pod_to_bytes;

    use super::super::ext2fs::{
        E2FS_ISCLEAN, E2FS_MAGIC, E2FS_REV1, EXT2_GD_SIZE, EXT2F_INCOMPAT_EXTENTS,
        EXT2F_INCOMPAT_FTYPE, EXT2F_ROCOMPAT_SPARSE_SUPER, Ext2Gd, Ext2fs,
    };
    use super::super::ext2fs_dinode::{EXT2_FIRSTINO, EXT2_ROOTINO, EXT4_EXTENTS, Ext2fsDinode};
    use super::super::ext2fs_extents::EXT4_EXT_MAGIC;

    /// The block size.
    pub const B: usize = 1024;
    /// Blocks in the file system.
    pub const NBLK: u32 = 64;
    /// Inodes per group (and in all).
    pub const IPG: u32 = 32;
    /// The inode size.
    pub const ISZ: usize = 128;
    /// The group descriptors' block.
    pub const GDT: usize = 2;
    /// The block bitmap's block.
    pub const BBITMAP: usize = 3;
    /// The inode bitmap's block.
    pub const IBITMAP: usize = 4;
    /// The inode table's first block.
    pub const ITABLE: usize = 5;
    /// The root directory's block.
    pub const ROOTDIR: usize = 9;
    /// `hello.txt`'s block.
    pub const HELLO: usize = 10;
    /// `big`: 12 direct blocks at 11..=22, its indirect block at 23 pointing at 24 and 25.
    pub const BIG0: usize = 11;
    /// `big`'s indirect block.
    pub const BIGIND: usize = 23;
    /// `ext`: its leaf at 26, extents {0, 2 blocks at 27} and {2, 1 block at 30}.
    pub const EXTLEAF: usize = 26;
    /// The free blocks: 29 and 31..=63.
    pub const NFREE: u32 = 34;
    /// The free inodes: 15..=32.
    pub const NIFREE: u32 = 18;
    /// `hello.txt`'s contents.
    pub const HELLO_DATA: &[u8] = b"hello, ext2\n";
    /// `big`'s size.
    pub const BIG_SIZE: usize = 14 * B - 100;
    /// `ext`'s size.
    pub const EXT_SIZE: usize = 3 * B - 10;

    /// `n` bytes of a pattern that differs per `seed`.
    pub fn pattern(n: usize, seed: u8) -> Vec<u8> {
        (0..n)
            .map(|i| ((i * 7 + usize::from(seed)) % 251) as u8)
            .collect()
    }

    /// The image being built, 1 KiB blocks.
    pub struct Image(pub Vec<u8>);

    impl Image {
        /// `b` at byte `off` of block `blk`.
        pub fn put(&mut self, blk: usize, off: usize, b: &[u8]) {
            let o = blk * B + off;
            self.0[o..o + b.len()].copy_from_slice(b);
        }

        /// A little-endian `u16` at byte `off` of block `blk`.
        pub fn u16(&mut self, blk: usize, off: usize, v: u16) {
            self.put(blk, off, &v.to_le_bytes());
        }

        /// A little-endian `u32` at byte `off` of block `blk`.
        pub fn u32(&mut self, blk: usize, off: usize, v: u32) {
            self.put(blk, off, &v.to_le_bytes());
        }

        /// Bit `bit` of the map in block `blk` set.
        pub fn setbit(&mut self, blk: usize, bit: usize) {
            self.0[blk * B + bit / 8] |= 1 << (bit % 8);
        }

        /// The block and the byte offset in it of inode `ino` (from 1).
        pub fn ioff(ino: u32) -> (usize, usize) {
            let i = (ino - 1) as usize;
            (ITABLE + i * ISZ / B, (i * ISZ) % B)
        }

        /// Inode `ino`: mode, links, size, block pointers (`blocks`), `DEV_BSIZE` blocks,
        /// flags.
        pub fn inode(
            &mut self,
            ino: u32,
            mode: u16,
            nlink: u16,
            size: u32,
            blocks: &[u32],
            flags: u32,
        ) {
            let (blk, off) = Self::ioff(ino);
            self.u16(blk, off + offset_of!(Ext2fsDinode, e2di_mode), mode);
            self.u16(blk, off + offset_of!(Ext2fsDinode, e2di_uid_low), 1000);
            self.u16(blk, off + offset_of!(Ext2fsDinode, e2di_uid_high), 1);
            self.u16(blk, off + offset_of!(Ext2fsDinode, e2di_gid_low), 20);
            self.u32(blk, off + offset_of!(Ext2fsDinode, e2di_size), size);
            self.u16(blk, off + offset_of!(Ext2fsDinode, e2di_nlink), nlink);
            self.u32(blk, off + offset_of!(Ext2fsDinode, e2di_flags), flags);
            self.u32(blk, off + offset_of!(Ext2fsDinode, e2di_gen), 7 + ino);
            let mut n = 0;
            for (i, &b) in blocks.iter().enumerate() {
                self.u32(blk, off + offset_of!(Ext2fsDinode, e2di_blocks) + 4 * i, b);
                if b != 0 && flags & EXT4_EXTENTS == 0 {
                    n += 2;
                }
            }
            self.u32(blk, off + offset_of!(Ext2fsDinode, e2di_nblock), n);
        }

        /// A directory entry at `off` of block `blk`.
        pub fn dirent(
            &mut self,
            blk: usize,
            off: usize,
            ino: u32,
            reclen: u16,
            name: &[u8],
            t: u8,
        ) {
            self.u32(blk, off, ino);
            self.u16(blk, off + 4, reclen);
            self.put(blk, off + 6, &[name.len() as u8, t]);
            self.put(blk, off + 8, name);
        }

        /// An extent tree node header.
        pub fn eh(&mut self, blk: usize, off: usize, ecount: u16, max: u16, depth: u16) {
            self.u16(blk, off, EXT4_EXT_MAGIC);
            self.u16(blk, off + 2, ecount);
            self.u16(blk, off + 4, max);
            self.u16(blk, off + 6, depth);
        }

        /// An extent: `len` blocks from logical `lblk` at disk block `start`.
        pub fn extent(&mut self, blk: usize, off: usize, lblk: u32, len: u16, start: u32) {
            self.u32(blk, off, lblk);
            self.u16(blk, off + 4, len);
            self.u16(blk, off + 6, 0);
            self.u32(blk, off + 8, start);
        }
    }

    /// The test file system; with `extents`, it has the `EXTENTS` incompatible feature (and
    /// mounts only read-only). Inode 12 is `hello.txt` (one block), 13 `big` (14 blocks,
    /// two behind the single indirect block), 14 `ext` (an index root in the inode, one leaf
    /// with two extents).
    pub fn image(extents: bool) -> Vec<u8> {
        let mut img = Image(vec![0; NBLK as usize * B]);

        // Super block.
        let mut sb = Ext2fs::new();
        sb.e2fs_icount = IPG;
        sb.e2fs_bcount = NBLK;
        sb.e2fs_rbcount = 3;
        sb.e2fs_fbcount = NFREE;
        sb.e2fs_ficount = NIFREE;
        sb.e2fs_first_dblock = 1;
        sb.e2fs_bpg = 8192;
        sb.e2fs_fpg = 8192;
        sb.e2fs_ipg = IPG;
        sb.e2fs_wtime = 1_700_000_000;
        sb.e2fs_max_mnt_count = 20;
        sb.e2fs_magic = E2FS_MAGIC;
        sb.e2fs_state = E2FS_ISCLEAN;
        sb.e2fs_beh = 1;
        sb.e2fs_rev = E2FS_REV1;
        sb.e2fs_first_ino = EXT2_FIRSTINO;
        sb.e2fs_inode_size = ISZ as u16;
        sb.e2fs_features_incompat =
            EXT2F_INCOMPAT_FTYPE | if extents { EXT2F_INCOMPAT_EXTENTS } else { 0 };
        sb.e2fs_features_rocompat = EXT2F_ROCOMPAT_SPARSE_SUPER;
        sb.to_disk(&mut img.0[1024..2048]);

        // Group descriptor.
        let gd = Ext2Gd {
            ext2bgd_b_bitmap: BBITMAP as u32,
            ext2bgd_i_bitmap: IBITMAP as u32,
            ext2bgd_i_tables: ITABLE as u32,
            ext2bgd_nbfree: NFREE as u16,
            ext2bgd_nifree: NIFREE as u16,
            ext2bgd_ndirs: 1,
            ..Ext2Gd::default()
        };
        let mut b = [0u8; EXT2_GD_SIZE];
        let _ = pod_to_bytes(&gd, &mut b);
        img.put(GDT, 0, &b);

        // Block bitmap: bit b is block b + 1; blocks 1..=28 and 30 are used, and the bits past
        // the last block.
        for b in 1..=28 {
            img.setbit(BBITMAP, b - 1);
        }
        img.setbit(BBITMAP, 30 - 1);
        for bit in (NBLK as usize - 1)..8 * B {
            img.setbit(BBITMAP, bit);
        }
        // Inode bitmap: inodes 1..=14, and the bits past the last inode.
        for i in 1..=14 {
            img.setbit(IBITMAP, i - 1);
        }
        for bit in IPG as usize..8 * B {
            img.setbit(IBITMAP, bit);
        }

        // The root directory.
        let mut root = [0u32; 15];
        root[0] = ROOTDIR as u32;
        img.inode(EXT2_ROOTINO, 0o40755, 2, B as u32, &root, 0);
        img.dirent(ROOTDIR, 0, 2, 12, b".", 2);
        img.dirent(ROOTDIR, 12, 2, 12, b"..", 2);
        img.dirent(ROOTDIR, 24, 12, 20, b"hello.txt", 1);
        img.dirent(ROOTDIR, 44, 13, 12, b"big", 1);
        img.dirent(ROOTDIR, 56, 14, (B - 56) as u16, b"ext", 1);

        // hello.txt (inode 12).
        let mut hello = [0u32; 15];
        hello[0] = HELLO as u32;
        img.inode(12, 0o100644, 1, HELLO_DATA.len() as u32, &hello, 0);
        img.put(HELLO, 0, HELLO_DATA);

        // big (inode 13): 14 blocks, two of them behind the indirect block.
        let mut big = [0u32; 15];
        for (i, b) in big.iter_mut().take(12).enumerate() {
            *b = (BIG0 + i) as u32;
        }
        big[12] = BIGIND as u32;
        img.inode(13, 0o100644, 1, BIG_SIZE as u32, &big, 0);
        img.u32(BIGIND, 0, 24);
        img.u32(BIGIND, 4, 25);
        let data = pattern(BIG_SIZE, 13);
        let blocks: Vec<usize> = (BIG0..BIG0 + 12).chain([24, 25]).collect();
        for (i, chunk) in data.chunks(B).enumerate() {
            img.put(blocks[i], 0, chunk);
        }

        // ext (inode 14): an index root in the inode, one leaf with two extents.
        img.inode(14, 0o100644, 1, EXT_SIZE as u32, &[], EXT4_EXTENTS);
        let (iblk, ioff) = Image::ioff(14);
        let broot = ioff + offset_of!(Ext2fsDinode, e2di_blocks);
        img.eh(iblk, broot, 1, 4, 1);
        img.u32(iblk, broot + 12, 0); // ei_blk
        img.u32(iblk, broot + 16, EXTLEAF as u32); // ei_leaf_lo
        img.eh(EXTLEAF, 0, 2, 84, 0);
        img.extent(EXTLEAF, 12, 0, 2, 27);
        img.extent(EXTLEAF, 24, 2, 1, 30);
        let data = pattern(EXT_SIZE, 14);
        for (chunk, blk) in data.chunks(B).zip([27, 28, 30]) {
            img.put(blk, 0, chunk);
        }

        img.0
    }
}

/// The fixture's dinodes (`ext2dinopl`).
static DINO: BoxPool<Ext2Inode> = BoxPool::new();

/// A test thread over fresh VFS tables with page memory and a buffer cache, and `image` as
/// the disk of testfs's block device (testffs's [`testffs::attach`]): nothing mounted yet.
pub fn attach(image: Vec<u8>) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let r = testffs::attach(image);
    let _ = DINO.init(IPL_NONE, PR_WAITOK, "ext2dinopl");
    r
}

/// The in-core super-block of the disk, as `ext2fs_mountfs` reads it and `e2fs_sbfill`
/// computes it: the super block, the geometry, the group descriptors.
pub fn superblock() -> Ext2Superblock {
    let devvp = testffs::devvp();
    let (t, error) = bread(&devvp.bufs, SBLOCK, SBSIZE as i32);
    if let Err(e) = error {
        std::panic!("testext2fs: the super block: {e:?}");
    }
    let fs = Ext2Superblock::new();
    e2fs_sbload(t.bytes(), &fs);
    brelse(t);
    let log_bsize = fs.e2fs_log_bsize();
    fs.e2fs_ncg
        .set((fs.e2fs_bcount() - fs.e2fs_first_dblock()).div_ceil(fs.e2fs_bpg()) as i32);
    fs.e2fs_fsbtodb.set(log_bsize as i32 + 1);
    fs.e2fs_bsize.set(1024 << log_bsize);
    fs.e2fs_bshift.set((LOG_MINBSIZE + log_bsize) as i32);
    fs.e2fs_fsize.set(1024 << fs.e2fs_log_fsize());
    fs.e2fs_qbmask.set(i64::from(fs.e2fs_bsize.get() - 1));
    fs.e2fs_bmask.set(!(fs.e2fs_bsize.get() - 1));
    fs.e2fs_ipb
        .set(fs.e2fs_bsize.get() / fs.dinode_size() as i32);
    fs.e2fs_itpg
        .set((fs.e2fs_ipg() / fs.e2fs_ipb.get() as u32) as i32);
    let bsize = fs.e2fs_bsize.get() as usize;
    let per_block = bsize / EXT2_GD_SIZE;
    let ngdb = (fs.e2fs_ncg.get() as usize).div_ceil(per_block);
    fs.e2fs_ngdb.set(ngdb as i32);
    fs.gd_alloc(ngdb, bsize);
    for i in 0..ngdb {
        let dblk = Daddr::from(i32::from(bsize <= 1024)) + i as Daddr + 1;
        let (t, error) = bread(&devvp.bufs, fsbtodb(&fs, dblk), bsize as i32);
        if let Err(e) = error {
            std::panic!("testext2fs: group descriptors: {e:?}");
        }
        e2fs_cgload(t.bytes(), &fs, i * per_block, bsize);
        brelse(t);
    }
    let large = fs.e2fs_features_rocompat() & EXT2F_ROCOMPAT_LARGE_FILE != 0;
    let maxfilesize = if !large || fs.e2fs_rev() == E2FS_REV0 {
        i64::from(i32::MAX)
    } else {
        let b = i64::from(fs.e2fs_bsize.get());
        let n = b / 4;
        ((12 + n + n * n + n * n * n) * b).min(i64::from(u32::MAX) << 9)
    };
    let ext = fs.e2fs_features_incompat() & EXT2F_INCOMPAT_EXTENTS != 0;
    fs.e2fs_maxfilesize
        .set(if ext { maxfilesize * 4 } else { maxfilesize });
    fs
}

/// The UFS structure of an ext2fs mount of `fs` on testffs's device, as `ext2fs_mountfs`
/// fills it.
pub fn ufsmount(fs: Ext2Superblock) -> UfsmountRef<TestGlue> {
    let ump = Ufsmount::<TestGlue>::alloc();
    ump.um_fstype.set(UM_EXT2FS);
    ump.um_nindir.set(nindir(&fs) as u64);
    ump.um_bptrtodb.set(fs.e2fs_fsbtodb.get() as u64);
    ump.um_seqinc.set(1);
    ump.um_dev.set(TEST_DEV);
    ump.um_devvp.set(Some(testffs::devvp()));
    ump.set_um_e2fs(fs);
    ump
}

/// An inode numbered `ino` of the mount whose UFS structure is `ump`, with no vnode (the
/// allocators' and the extent tree's pure tests): its dinode a fresh, zeroed item of the
/// fixture's pool.
pub fn detached(ump: &UfsmountRef<TestGlue>, ino: Ufsino) -> &'static Inode<TestGlue> {
    let ip: &'static Inode<TestGlue> = Box::leak(Box::new(Inode::new(InodeInit {
        ump: Some(ump.clone()),
        devvp: Some(testffs::devvp()),
        dev: TEST_DEV,
        number: ino,
        fstype: UM_EXT2FS,
        nindir: ump.um_nindir.get(),
        bptrtodb: ump.um_bptrtodb.get(),
        seqinc: 1,
        ..InodeInit::default()
    })));
    let Some(din) = DINO.get(PR_WAITOK, Ext2Inode::new) else {
        std::panic!("testext2fs: no dinode");
    };
    ip.set_din(Dinode::Ext2(din));
    ip
}
/* </CODE> */
