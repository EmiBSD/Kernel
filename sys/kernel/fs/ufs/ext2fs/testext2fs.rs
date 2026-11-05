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
//! `bsd`'s `ext2fs_vfsops.rs` tests; and [`attach`], [`mount`], [`mount_root`], the image
//! mounted by the kernel's `ext2fs_mountfs` (since N5.6c3; N5.6c1's mount of its own steps,
//! its `vget`, its vnode tables and its inode table went), its inodes made by `ext2fs_vget`,
//! over testffs's device half: the image is the disk of testfs's block device
//! (`testffs::attach`, `testffs::disk`, `testffs::sync`).

use core::cell::Cell;

use std::boxed::Box;
use std::sync::MutexGuard;

use hal::pool::PR_WAITOK;
use hal::sys::errno::Errno;
use hal::sys::mount::{MNT_LOCAL, MNT_SWAPPABLE, MOUNT_EXT2FS, UfsArgs};
use hal::sys::types::Daddr;
use hal::user::UserPtr;

use super::ext2fs::{Ext2Superblock, SBLOCK, SBSIZE, e2fs_sbload, nindir};
use super::ext2fs_dinode::{EXT2_ROOTINO, Ext2Inode};
use super::ext2fs_vfsops::{
    Ext2fsVfs, e2fs_sbfill, ext2fs_init, ext2fs_mountfs, ext2fs_statfs, ext2fs_vget,
};
use crate::fs::bio::{biodone, bread, brelse};
use crate::fs::buf::{B_ERROR, BufRef};
use crate::fs::init::{rootvnode, set_rootvnode};
use crate::fs::mount::{
    Mount, Vfsconf, mountlist_insert_tail, vfs_mount_alloc, vfs_mount_free, vfs_unbusy,
};
use crate::fs::subr::{vfs, vput, vref, vrele};
use crate::fs::testfs::{TestGlue, set_bdev_strategy};
use crate::fs::ufs::ffs::testffs;
use crate::fs::ufs::ufs::dinode::{IFREG, Ufsino};
use crate::fs::ufs::ufs::inode::{Dinode, Inode, InodeInit, vtoi};
use crate::fs::ufs::ufs::testufs::TEST_DEV;
use crate::fs::ufs::ufs::ufsmount::{UM_EXT2FS, Ufsmount, UfsmountRef};
use crate::fs::vnode::{VREG, Vnode};
use crate::fs::vops::{Cred, vop_unlock};
use crate::proc::{ProcGlue, Thread};

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

    /// What [`Image::new`] makes a file system of: `nblk` blocks and `ipg` inodes of `isz`
    /// bytes in one group, `rbcount` blocks reserved, the free counts and the incompatible
    /// features of the super block.
    pub struct Params {
        /// Blocks in the file system.
        pub nblk: usize,
        /// Inodes per group (and in all).
        pub ipg: u32,
        /// The inode size.
        pub isz: usize,
        /// Blocks reserved for the super-user.
        pub rbcount: u32,
        /// Free blocks.
        pub nfree: u32,
        /// Free inodes.
        pub nifree: u32,
        /// `e2fs_features_incompat`.
        pub incompat: u32,
    }

    impl Image {
        /// A file system of `p` (1 KiB blocks, one group, revision 1, sparse super blocks,
        /// clean, the group descriptor in [`GDT`], the bitmaps in [`BBITMAP`] and [`IBITMAP`],
        /// the inode table from [`ITABLE`]): the super block, the descriptor (one directory)
        /// and the bitmaps' bits past the last block and the last inode; nothing used yet.
        pub fn new(p: &Params) -> Image {
            let mut img = Image(vec![0; p.nblk * B]);

            let mut sb = Ext2fs::new();
            sb.e2fs_icount = p.ipg;
            sb.e2fs_bcount = p.nblk as u32;
            sb.e2fs_rbcount = p.rbcount;
            sb.e2fs_fbcount = p.nfree;
            sb.e2fs_ficount = p.nifree;
            sb.e2fs_first_dblock = 1;
            sb.e2fs_bpg = 8192;
            sb.e2fs_fpg = 8192;
            sb.e2fs_ipg = p.ipg;
            sb.e2fs_wtime = 1_700_000_000;
            sb.e2fs_max_mnt_count = 20;
            sb.e2fs_magic = E2FS_MAGIC;
            sb.e2fs_state = E2FS_ISCLEAN;
            sb.e2fs_beh = 1;
            sb.e2fs_rev = E2FS_REV1;
            sb.e2fs_first_ino = EXT2_FIRSTINO;
            sb.e2fs_inode_size = p.isz as u16;
            sb.e2fs_features_incompat = p.incompat;
            sb.e2fs_features_rocompat = EXT2F_ROCOMPAT_SPARSE_SUPER;
            sb.to_disk(&mut img.0[B..2 * B]);

            let gd = Ext2Gd {
                ext2bgd_b_bitmap: BBITMAP as u32,
                ext2bgd_i_bitmap: IBITMAP as u32,
                ext2bgd_i_tables: ITABLE as u32,
                ext2bgd_nbfree: p.nfree as u16,
                ext2bgd_nifree: p.nifree as u16,
                ext2bgd_ndirs: 1,
                ..Ext2Gd::default()
            };
            let mut b = [0u8; EXT2_GD_SIZE];
            let _ = pod_to_bytes(&gd, &mut b);
            img.put(GDT, 0, &b);

            for bit in (p.nblk - 1)..8 * B {
                img.setbit(BBITMAP, bit);
            }
            for bit in p.ipg as usize..8 * B {
                img.setbit(IBITMAP, bit);
            }
            img
        }

        /// `blocks` marked used in the block bitmap (bit `b - 1` is block `b`).
        pub fn use_blocks(&mut self, blocks: impl IntoIterator<Item = usize>) {
            for b in blocks {
                self.setbit(BBITMAP, b - 1);
            }
        }

        /// `inodes` marked used in the inode bitmap (bit `i - 1` is inode `i`).
        pub fn use_inodes(&mut self, inodes: impl IntoIterator<Item = usize>) {
            for i in inodes {
                self.setbit(IBITMAP, i - 1);
            }
        }
    }

    /// [`mkfs`]'s blocks (one group).
    pub const MKFS_NBLK: usize = 1024;
    /// [`mkfs`]'s inodes (one group).
    pub const MKFS_IPG: usize = 64;

    /// The inode table's size, in blocks, of [`mkfs`]'s file system for inodes of `isz` bytes.
    pub const fn itblocks(isz: usize) -> usize {
        MKFS_IPG * isz / B
    }

    /// A fresh ext2 file system of [`MKFS_NBLK`] blocks and [`MKFS_IPG`] inodes of `isz`
    /// bytes (revision 1, `FILETYPE`, sparse super blocks): the super block, one group
    /// descriptor, the bitmaps, the inode table and an empty root directory owned by root, as
    /// `newfs_ext2fs` leaves it (without lost+found).
    pub fn mkfs(isz: usize) -> Vec<u8> {
        let rootblk = ITABLE + itblocks(isz);
        let reserved = (EXT2_FIRSTINO - 1) as usize; // inodes 1..=10
        let mut img = Image::new(&Params {
            nblk: MKFS_NBLK,
            ipg: MKFS_IPG as u32,
            isz,
            rbcount: 50,
            nfree: (MKFS_NBLK - 1 - rootblk) as u32,
            nifree: (MKFS_IPG - reserved) as u32,
            incompat: EXT2F_INCOMPAT_FTYPE,
        });
        img.use_blocks(1..=rootblk);
        img.use_inodes(1..=reserved);

        // The root directory.
        let at = (EXT2_ROOTINO as usize - 1) * isz;
        img.u16(ITABLE, at + offset_of!(Ext2fsDinode, e2di_mode), 0o40755);
        img.u16(ITABLE, at + offset_of!(Ext2fsDinode, e2di_nlink), 2);
        img.u32(ITABLE, at + offset_of!(Ext2fsDinode, e2di_size), B as u32);
        img.u32(ITABLE, at + offset_of!(Ext2fsDinode, e2di_nblock), 2);
        img.u32(
            ITABLE,
            at + offset_of!(Ext2fsDinode, e2di_blocks),
            rootblk as u32,
        );
        if isz > 128 {
            img.u16(ITABLE, at + offset_of!(Ext2fsDinode, e2di_isize), 32);
        }
        img.dirent(rootblk, 0, EXT2_ROOTINO, 12, b".", 2);
        img.dirent(rootblk, 12, EXT2_ROOTINO, (B - 12) as u16, b"..", 2);
        img.0
    }

    /// The test file system; with `extents`, it has the `EXTENTS` incompatible feature (and
    /// mounts only read-only). Inode 12 is `hello.txt` (one block), 13 `big` (14 blocks,
    /// two behind the single indirect block), 14 `ext` (an index root in the inode, one leaf
    /// with two extents).
    pub fn image(extents: bool) -> Vec<u8> {
        let mut img = Image::new(&Params {
            nblk: NBLK as usize,
            ipg: IPG,
            isz: ISZ,
            rbcount: 3,
            nfree: NFREE,
            nifree: NIFREE,
            incompat: EXT2F_INCOMPAT_FTYPE | if extents { EXT2F_INCOMPAT_EXTENTS } else { 0 },
        });
        // Blocks 1..=28 and 30 are used, inodes 1..=14.
        img.use_blocks((1..=28).chain([30]));
        img.use_inodes(1..=14);

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

/// The user memory of the kernel ext2fs's tests over the system calls, `ext2fs_vfsops`'s and
/// `ext2fs_vnops`'s, which share it (both hold MEMORY: one test at a time; no other test of
/// the binary uses the range): four path slots of 256 bytes from `USER_BASE + 0xe000`
/// ([`up`]), then [`UBUF`] to `USER_BASE + 0xf000`.
pub const UPATH: usize = crate::testutil::USER_BASE + 0xe000;
/// The buffer after the path slots.
pub const UBUF: usize = crate::testutil::USER_BASE + 0xe400;
/// [`UBUF`]'s length.
pub const UBUF_LEN: usize = 0xf000 - 0xe400;

/// The NUL-terminated `s` in user memory, in path slot `i` of [`UPATH`].
pub fn up(i: usize, s: &[u8]) -> UserPtr<u8> {
    if s.len() >= 256 || i >= 4 {
        std::panic!("testext2fs: up: slot {i}, {} bytes", s.len());
    }
    let at = UPATH + i * 256;
    let mut b = s.to_vec();
    b.push(0);
    if let Err(e) = UserPtr::<u8>::new(at).copyout_slice(&b) {
        std::panic!("testext2fs: up: {e:?}");
    }
    UserPtr::new(at)
}

/// The fixture's configuration entry: the kernel's ext2fs as `vfsconflist[]` has it
/// (`"ext2fs"`, type 17, `struct ufs_args`, `MNT_LOCAL | MNT_SWAPPABLE`).
pub static TEST_EXT2_CONF: Vfsconf<TestGlue> = Vfsconf::new(
    &Ext2fsVfs::<TestGlue>::VFS,
    MOUNT_EXT2FS,
    17,
    MNT_LOCAL | MNT_SWAPPABLE,
    UfsArgs::SIZE,
);

std::thread_local! {
    /// The disk block (`b_blkno`, in `DEV_BSIZE` units) whose transfer [`failing_strategy`]
    /// fails with `EIO`, if any.
    static FAIL_BLKNO: Cell<Option<Daddr>> = const { Cell::new(None) };
}

/// A test thread over fresh VFS tables with page memory and a buffer cache, its `uiomove`
/// copying kernel iovecs, `ext2fs_init` run (the pools, the UFS layer), and `image` as the
/// disk of testfs's block device (testffs's [`testffs::attach`]): nothing mounted yet.
pub fn attach(image: std::vec::Vec<u8>) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let r = testffs::attach(image);
    if let Err(e) = ext2fs_init(&TEST_EXT2_CONF) {
        std::panic!("testext2fs: ext2fs_init: {e:?}");
    }
    FAIL_BLKNO.with(|c| c.set(None));
    r
}

/// The disk's strategy with one block that answers `EIO` (`blkno`, or none): testffs's
/// otherwise, installed for this thread's device.
pub fn fail_block(blkno: Option<Daddr>) {
    FAIL_BLKNO.with(|c| c.set(blkno));
    set_bdev_strategy(Some(failing_strategy));
}

/// testffs's `d_strategy`, except for [`FAIL_BLKNO`]'s block: `B_ERROR`, `EIO`, `biodone`.
fn failing_strategy(t: BufRef) {
    if FAIL_BLKNO.with(Cell::get) == Some(t.buf().blkno()) {
        t.buf().set_error(Some(Errno::EIO));
        t.buf().set(B_ERROR);
        biodone(t);
        return;
    }
    testffs::disk_strategy(t);
}

/// The in-core super-block of the disk, as `ext2fs_mountfs` reads it and fills it
/// (`e2fs_sbload`, then the kernel's `e2fs_sbfill`): for the pure tests of the allocators and
/// the extent tree, which take no mount.
pub fn superblock() -> Ext2Superblock {
    let devvp = testffs::devvp();
    let (t, error) = bread(&devvp.bufs, SBLOCK, SBSIZE as i32);
    if let Err(e) = error {
        std::panic!("testext2fs: the super block: {e:?}");
    }
    let fs = Ext2Superblock::new();
    e2fs_sbload(t.bytes(), &fs);
    brelse(t);
    if let Err(e) = e2fs_sbfill(devvp, &fs) {
        std::panic!("testext2fs: e2fs_sbfill: {e:?}");
    }
    fs
}

/// The UFS structure of an ext2fs mount of `fs` on testffs's device, as `ext2fs_mountfs`
/// fills it, hung from no mount (the pure tests').
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
/// glue's `ext2dinopl`.
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
    let Some(din) = vfs::<TestGlue>()
        .ufs
        .ext2fs_dino
        .get(PR_WAITOK, Ext2Inode::new)
    else {
        std::panic!("testext2fs: no dinode");
    };
    ip.set_din(Dinode::Ext2(din));
    ip
}

/// A new mount of the disk with the flags `flags` (`MNT_RDONLY`, `MNT_FORCE`), as
/// `ext2fs_mount` makes one: `ext2fs_mountfs` on the device vnode, the statistics, the mount
/// unbusied; `Err` freed the mount.
pub fn try_mount(p: &Thread<TestGlue>, flags: i32) -> Result<&'static Mount<TestGlue>, Errno> {
    let mp = vfs_mount_alloc(None, &TEST_EXT2_CONF);
    mp.set_flag(mp.flag() | flags);
    mp.with_stat(|sp| sp.f_mntonname[..4].copy_from_slice(b"/ext"));
    let r = ext2fs_mountfs(testffs::devvp(), mp, p);
    if r.is_ok() {
        let mut st = mp.stat();
        let _ = ext2fs_statfs(mp, &mut st, p);
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
        None => std::panic!("testext2fs: no curproc"),
    });
    match r {
        Ok(mp) => mp,
        Err(e) => std::panic!("testext2fs: ext2fs_mountfs: {e:?}"),
    }
}

/// A new mount of the disk at `/`, as `main` mounts the root: [`mount`], the mount list, the
/// root vnode and the thread's current directory (testfs's root, which [`attach`]'s thread
/// started with, released). `testffs::unmount_root` undoes it.
pub fn mount_root(p: &'static Thread<TestGlue>, flags: i32) -> &'static Mount<TestGlue> {
    let mp = match try_mount(p, flags) {
        Ok(mp) => mp,
        Err(e) => std::panic!("testext2fs: ext2fs_mountfs: {e:?}"),
    };
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

/// The inode `ino` of `mp`: `ext2fs_vget`, its vnode locked and referenced (`vput` it).
pub fn vget(mp: &'static Mount<TestGlue>, ino: Ufsino) -> Result<&'static Vnode<TestGlue>, Errno> {
    ext2fs_vget(mp, u64::from(ino))
}

/// The root vnode of `mp`, locked and referenced (`vput` it).
pub fn root(mp: &'static Mount<TestGlue>) -> &'static Vnode<TestGlue> {
    match vget(mp, EXT2_ROOTINO) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testext2fs: no root: {e:?}"),
    }
}

/// A new regular file in the root directory's group: an inode of `ext2fs_inode_alloc` made
/// `IFREG | 0644` with one link (what `ext2fs_makeinode` does next), its vnode locked and
/// referenced (`vput` it).
pub fn new_file(mp: &'static Mount<TestGlue>, p: &Thread<TestGlue>) -> &'static Vnode<TestGlue> {
    let root = root(mp);
    let rip = vtoi(root);
    let vp =
        match super::ext2fs_alloc::ext2fs_inode_alloc(&rip, IFREG | 0o644, Cred::real(p.ucred())) {
            Ok(vp) => vp,
            Err(e) => std::panic!("testext2fs: ext2fs_inode_alloc: {e:?}"),
        };
    drop(rip);
    vput(root);
    let ip = vtoi(vp);
    let d = ip.i_e2din();
    d.set_i_e2fs_mode((IFREG | 0o644) as u16);
    d.set_i_e2fs_nlink(1);
    ip.i_effnlink.set(1);
    vp.set_vtype(VREG);
    vp
}
/* </CODE> */
