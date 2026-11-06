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
//! The kernel udf's test fixtures (host tests and benches only): [`mkudf`], LZ's in-memory
//! UDF builder (`udf_vnops.rs`'s tests: the tags, the time stamps, the file entries,
//! allocation and file identifier descriptors, the anchor, the volume descriptor sequence
//! and the test volume "M10C") with the variants named tests need (a VAT, sparing tables, a
//! metadata partition); the fixture's configuration entry and the mounts through
//! `udf_mountfs` over testffs's device ([`try_mount`], [`mount_root`]); and, for the tests
//! that need no mount, a mount structure over testffs's device ([`test_ump`]) and unodes of
//! it with vnodes of udf's own table ([`test_node`]).

use std::sync::MutexGuard;
use std::vec::Vec;

use hal::malloc::{M_WAITOK, MArc, MArray};
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::mount::{MNT_LOCAL, MNT_RDONLY, MOUNT_UDF, UdfArgs};

use crate::fs::isofs::udf::udf::{
    M_UDFFENTRY, M_UDFMOUNT, UDF_HASHTBLSIZE, UdfHash, Udfino, Umount, UmountRef, Unode, UnodeRef,
};
use crate::fs::isofs::udf::udf_vfsops::{UdfVfs, udf_init, udf_mountfs, udf_root};
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc};
use crate::fs::subr::{getnewvnode, hashinit, siphash_key, vfs, vref};
use crate::fs::testfs::{TEST_CONF, TestGlue};
use crate::fs::ufs::ffs::testffs;
use crate::fs::vnode::{VT_UDF, Vnode};
use crate::proc::Thread;

/// `image` as the disk of testffs's block device, and udf's pools made: the memory guard
/// and the test thread.
pub fn attach(image: Vec<u8>) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let (g, p) = testffs::attach(image);
    if let Err(e) = udf_init::<TestGlue>() {
        std::panic!("testudf: udf_init: {e:?}");
    }
    (g, p)
}

/// The fixture's configuration entry: the kernel's udf as `vfsconflist[]` has it (`"udf"`,
/// type 13, `struct udf_args`, `MNT_LOCAL`).
pub static TEST_UDF_CONF: Vfsconf<TestGlue> = Vfsconf::new(
    &UdfVfs::<TestGlue>::VFS,
    MOUNT_UDF,
    13,
    MNT_LOCAL,
    UdfArgs::SIZE,
);

/// A new read-only mount of the disk by `udf_mountfs` (the VAT at `lastblock`), as
/// `udf_mount` makes one: the statistics, the mount unbusied, not on the mount list; `Err`
/// freed the mount (`testffs::mount_conf`). The device's reference the mount keeps is
/// testffs's own (where `udf_mount`'s `namei` takes one; [`unmount`] gives one back).
pub fn try_mount(p: &Thread<TestGlue>, lastblock: u32) -> Result<&'static Mount<TestGlue>, Errno> {
    testffs::mount_conf(p, &TEST_UDF_CONF, b"/udf", MNT_RDONLY, |devvp, mp| {
        udf_mountfs(devvp, mp, lastblock, p)
    })
}

/// `testffs::unmount_root` of a [`mount_root`]'s mount, and the device's reference its
/// `udf_unmount` released taken again (the fixture's, which stands for `namei`'s).
pub fn unmount(p: &'static Thread<TestGlue>, mp: &'static Mount<TestGlue>) {
    testffs::unmount_root(p, mp);
    vref(testffs::devvp());
}

/// [`try_mount`] at `/`, made the root as `main` mounts it (`testffs::make_root`;
/// `testffs::unmount_root` undoes it).
pub fn mount_root(p: &'static Thread<TestGlue>, lastblock: u32) -> &'static Mount<TestGlue> {
    let mp = match try_mount(p, lastblock) {
        Ok(mp) => mp,
        Err(e) => std::panic!("testudf: udf_mountfs: {e:?}"),
    };
    testffs::make_root(p, mp);
    mp
}

/// The root directory's vnode of `mp`, locked and referenced (`vput` it): `VFS_ROOT`.
pub fn root(mp: &'static Mount<TestGlue>) -> &'static Vnode<TestGlue> {
    match udf_root(mp) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testudf: no root: {e:?}"),
    }
}

/// A mount structure on a fresh mount of testfs's type and testffs's device, with 2048-byte
/// blocks and its hash, as `udf_mountfs` leaves it before the VAT is read, changed by `f`,
/// published and hung on the mount (`VFSTOUDFFS` finds it).
pub fn test_ump(f: impl FnOnce(&Umount<TestGlue>)) -> UmountRef<TestGlue> {
    let mp = vfs_mount_alloc(None, &TEST_CONF[0]);
    let ump = Umount::new(mp, testffs::devvp());
    ump.um_bsize.set(2048);
    ump.um_bmask.set(2047);
    ump.um_bshift.set(11);
    let Some((tbl, mask)) = hashinit(UDF_HASHTBLSIZE, M_UDFMOUNT, M_WAITOK) else {
        std::panic!("testudf: hashinit");
    };
    ump.set_hashtbl(UdfHash::new(tbl, mask, siphash_key()));
    f(&ump);
    let ump = ump.alloc();
    mp.set_node(MArc::erase(ump.clone()));
    ump
}

/// The file entry copy `udf_vget` makes of `entry` (a block holding a file entry):
/// `malloc(size + UDF_EXTFENTRY_SIZE, M_UDFFENTRY, M_ZERO)` (the zeros written by the
/// array's initialisation) with `size + hdr` bytes copied,
/// `size` being `l_ea + l_ad` and `hdr` the entry's fixed part.
pub fn fentry_copy(entry: &[u8]) -> MArray<u8> {
    let hdr = crate::fs::isofs::udf::udf::FentryHdr::parse(entry);
    let size =
        hal::sys::endian::letoh32(hdr.l_ea) as usize + hal::sys::endian::letoh32(hdr.l_ad) as usize;
    let n = size + crate::fs::isofs::udf::ecma167_udf::UDF_EXTFENTRY_SIZE;
    let Some(mut fe) = MArray::new(M_UDFFENTRY, M_WAITOK, n, |_| 0u8) else {
        std::panic!("testudf: malloc(M_UDFFENTRY)");
    };
    let copy = (size + hdr.data_off()).min(entry.len());
    fe[..copy].copy_from_slice(&entry[..copy]);
    fe
}

/// A unode of `ump` numbered `ino` whose file entry is `entry`'s copy, with a vnode of its
/// own of udf's table (referenced, unlocked), its device vnode referenced, as `udf_vget`
/// links them; not hashed.
pub fn test_node(
    ump: &UmountRef<TestGlue>,
    ino: Udfino,
    entry: &[u8],
) -> (UnodeRef<TestGlue>, &'static Vnode<TestGlue>) {
    let up = Unode::new(ump.clone(), ino, Some(fentry_copy(entry))).alloc();
    let vp = match getnewvnode(VT_UDF, Some(ump.um_mountp), &vfs::<TestGlue>().udf.vops) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testudf: getnewvnode: {e:?}"),
    };
    up.u_vnode.set(Some(vp));
    vp.set_node(PoolArc::erase(up.clone()));
    up.u_devvp.set(Some(ump.devvp()));
    vref(ump.devvp());
    (up, vp)
}

/// `mkudf`: LZ's in-memory UDF builder (`udf_vnops.rs`'s tests) and the variants the tests
/// of the descriptors, the label spoof's parse, the node, the VAT, the mount and the vnode
/// operations need: 2048-byte sectors.
pub mod mkudf {
    use core::mem::offset_of;

    use std::vec;
    use std::vec::Vec;

    use crate::fs::isofs::udf::ecma167_udf::{
        AnchorVdp, ExtfileEntry, FileEntry, FilesetDesc, LogvolDesc, PartDesc, PriVolDesc,
        TAGID_ANCHOR, TAGID_EXTFENTRY, TAGID_FENTRY, TAGID_FID, TAGID_FSD, TAGID_LOGVOL,
        TAGID_PARTITION, TAGID_PRI_VOL, TAGID_SPARING_TABLE, TAGID_TERM, UDF_EXTFENTRY_SIZE,
        UDF_FENTRY_SIZE, UDF_FILE_CHAR_DEL, UDF_FILE_CHAR_DIR, UDF_FILE_CHAR_PAR,
    };

    /// The sector size.
    pub const B: usize = 2048;
    /// The partition's first sector and length.
    pub const PSTART: usize = 260;
    /// The partition's length in sectors.
    pub const PLEN: usize = 20;
    /// The fragmented directory: 60 descriptors of 80 bytes after the parent's 40, over three
    /// extents (2048, 2048 and 744 bytes).
    pub const NSUB: usize = 60;
    /// Its size.
    pub const SUB_SIZE: usize = 40 + NSUB * 80;
    /// The logical volume descriptor's sector.
    pub const LVD: usize = 34;
    /// The test file's contents (`/m10c-udf.txt`, lb 4, data at lb 5).
    pub const TEXT: &[u8; 12] = b"m10c-udf-42\n";

    /// A descriptor tag: `id`, version 2, the checksum, `tag_loc`.
    pub fn tag_bytes(id: u16, loc: u32) -> [u8; 16] {
        let mut t = [0u8; 16];
        t[0..2].copy_from_slice(&id.to_le_bytes());
        t[2..4].copy_from_slice(&2u16.to_le_bytes());
        t[12..16].copy_from_slice(&loc.to_le_bytes());
        let sum = t
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 4)
            .fold(0u8, |s, (_, &b)| s.wrapping_add(b));
        t[4] = sum;
        t
    }

    /// A timestamp: 2026-10-04 12:34:56, time zone `tz` minutes (type 1 when given).
    pub fn timestamp(tz: Option<i16>) -> [u8; 12] {
        let mut t = [0u8; 12];
        let type_tz: u16 = match tz {
            Some(m) => 0x1000 | (m as u16 & 0x0fff),
            None => 0,
        };
        t[0..2].copy_from_slice(&type_tz.to_le_bytes());
        t[2..4].copy_from_slice(&2026u16.to_le_bytes());
        t[4..12].copy_from_slice(&[10, 4, 12, 34, 56, 0, 0, 0]);
        t
    }

    /// A short allocation descriptor.
    pub fn short_ad(len: u32, lb: u32) -> [u8; 8] {
        let mut a = [0u8; 8];
        a[0..4].copy_from_slice(&len.to_le_bytes());
        a[4..8].copy_from_slice(&lb.to_le_bytes());
        a
    }

    /// A long allocation descriptor (partition 0).
    pub fn long_ad(len: u32, lb: u32) -> [u8; 16] {
        let mut a = [0u8; 16];
        a[0..4].copy_from_slice(&len.to_le_bytes());
        a[4..8].copy_from_slice(&lb.to_le_bytes());
        a
    }

    /// A (extended, `efe`) file entry at partition block `lb`, one sector: type, allocation
    /// kind (the low bits of the ICB flags, strategy 4), permissions, owner, size and
    /// allocation descriptors.
    #[allow(
        clippy::too_many_arguments,
        reason = "LZ's builder, one argument per member"
    )]
    pub fn file_entry(
        lb: u32,
        efe: bool,
        file_type: u8,
        alloc: u16,
        perm: u32,
        uid: u32,
        inf_len: u64,
        ads: &[u8],
    ) -> Vec<u8> {
        let mut s = vec![0u8; B];
        let mut put = |off: usize, b: &[u8]| s[off..off + b.len()].copy_from_slice(b);
        put(
            0,
            &tag_bytes(if efe { TAGID_EXTFENTRY } else { TAGID_FENTRY }, lb),
        );
        let icb = offset_of!(FileEntry, icbtag);
        put(icb + 4, &4u16.to_le_bytes()); // strat_type
        put(icb + 8, &1u16.to_le_bytes()); // max_num_entries
        put(icb + 11, &[file_type]);
        put(icb + 18, &alloc.to_le_bytes());
        put(offset_of!(FileEntry, uid), &uid.to_le_bytes());
        put(offset_of!(FileEntry, gid), &20u32.to_le_bytes());
        put(offset_of!(FileEntry, perm), &perm.to_le_bytes());
        put(offset_of!(FileEntry, link_cnt), &1u16.to_le_bytes());
        put(offset_of!(FileEntry, inf_len), &inf_len.to_le_bytes());
        if efe {
            put(offset_of!(ExtfileEntry, logblks_rec), &3u64.to_le_bytes());
            put(offset_of!(ExtfileEntry, atime), &timestamp(None));
            put(offset_of!(ExtfileEntry, mtime), &timestamp(Some(-60)));
            put(
                offset_of!(ExtfileEntry, l_ad),
                &(ads.len() as u32).to_le_bytes(),
            );
            put(UDF_EXTFENTRY_SIZE, ads);
        } else {
            put(offset_of!(FileEntry, logblks_rec), &2u64.to_le_bytes());
            put(offset_of!(FileEntry, atime), &timestamp(None));
            put(offset_of!(FileEntry, mtime), &timestamp(None));
            put(
                offset_of!(FileEntry, l_ad),
                &(ads.len() as u32).to_le_bytes(),
            );
            put(UDF_FENTRY_SIZE, ads);
        }
        s
    }

    /// The anchor at sector 256 naming a main volume descriptor sequence of `vds_len` bytes
    /// at sector 32, and a primary volume descriptor at `pvd_at` named `vol_id` (a dstring),
    /// in an image of 257 sectors.
    pub fn volume(vds_len: u32, pvd_at: usize, vol_id: &[u8; 32]) -> Vec<u8> {
        let mut img = Img(vec![0u8; 257 * B]);
        img.tag(256, TAGID_ANCHOR, 256);
        img.u32(256, offset_of!(AnchorVdp, main_vds_ex), vds_len);
        img.u32(256, offset_of!(AnchorVdp, main_vds_ex) + 4, 32);
        img.tag(pvd_at, TAGID_PRI_VOL, pvd_at as u32);
        img.put(pvd_at, offset_of!(PriVolDesc, vol_id), vol_id);
        img.0
    }

    /// A dstring of 32 bytes: `name` in 8-bit CS0, its length in the last byte.
    pub fn dstring(name: &[u8]) -> [u8; 32] {
        let mut d = [0u8; 32];
        d[0] = 8;
        d[1..=name.len()].copy_from_slice(name);
        d[31] = name.len() as u8 + 1;
        d
    }

    /// An image being built, 2048-byte sectors.
    pub struct Img(pub Vec<u8>);

    impl Img {
        /// `bytes` at byte `off` of `sector`.
        pub fn put(&mut self, sector: usize, off: usize, bytes: &[u8]) {
            let o = sector * B + off;
            self.0[o..o + bytes.len()].copy_from_slice(bytes);
        }

        /// A little-endian `u16` at byte `off` of `sector`.
        pub fn u16(&mut self, sector: usize, off: usize, v: u16) {
            self.put(sector, off, &v.to_le_bytes());
        }

        /// A little-endian `u32` at byte `off` of `sector`.
        pub fn u32(&mut self, sector: usize, off: usize, v: u32) {
            self.put(sector, off, &v.to_le_bytes());
        }

        /// A descriptor tag at the start of `sector`.
        pub fn tag(&mut self, sector: usize, id: u16, loc: u32) {
            self.put(sector, 0, &tag_bytes(id, loc));
        }

        /// A partition block: `bytes` at the start of partition block `lb`.
        pub fn lb(&mut self, lb: usize, bytes: &[u8]) {
            self.put(PSTART + lb, 0, bytes);
        }

        /// The logical volume descriptor's `lb_size` (U3's tests).
        pub fn lb_size(&mut self, v: u32) {
            self.u32(LVD, offset_of!(LogvolDesc, lb_size), v);
        }

        /// The file set descriptor's root ICB length (U3's tests).
        pub fn root_len(&mut self, len: u32) {
            self.u32(PSTART, offset_of!(FilesetDesc, rootdir_icb), len);
        }

        /// A type 2 partition map named `id` (its other members `body`, from byte 36 on)
        /// after the descriptor's maps, counted in `n_pm`.
        pub fn map2(&mut self, id: &[u8; 23], body: &[u8]) {
            let n_pm = offset_of!(LogvolDesc, n_pm);
            let o = LVD * B + n_pm;
            let n = u32::from_le_bytes([self.0[o], self.0[o + 1], self.0[o + 2], self.0[o + 3]]);
            // The maps so far: the type 1 map first, then 64 bytes each.
            let at = LogvolDesc::SIZE + 6 + (n as usize - 1) * 64;
            let mut m = [0u8; 64];
            m[0] = 2;
            m[1] = 64;
            m[5..28].copy_from_slice(id);
            m[36..36 + body.len()].copy_from_slice(body);
            self.put(LVD, at, &m);
            self.u32(LVD, n_pm, n + 1);
        }
    }

    /// A file identifier descriptor: `name` (8-bit CS0, empty for the parent), its file entry
    /// at `lb`, padded to 4 bytes.
    pub fn fid(file_char: u8, name: &[u8], lb: u32) -> Vec<u8> {
        let l_fi = if name.is_empty() { 0 } else { name.len() + 1 };
        let mut f = vec![0u8; (38 + l_fi + 3) & !3];
        f[16..18].copy_from_slice(&1u16.to_le_bytes());
        f[18] = file_char;
        f[19] = l_fi as u8;
        f[20..24].copy_from_slice(&(B as u32).to_le_bytes());
        f[24..28].copy_from_slice(&lb.to_le_bytes());
        if l_fi != 0 {
            f[38] = 8;
            f[39..39 + name.len()].copy_from_slice(name);
        }
        f[..16].copy_from_slice(&tag_bytes(TAGID_FID, 0));
        f
    }

    /// The name of the `i`th entry of the fragmented directory: 41 characters, so that each
    /// descriptor is 80 bytes.
    pub fn sub_name(i: usize) -> Vec<u8> {
        let mut n = std::format!("file-{i:02}-").into_bytes();
        n.resize(41, b'x');
        n
    }

    /// The test volume "M10C" (LZ's):
    /// - `/m10c-udf.txt` ("m10c-udf-42\n", a file entry at lb 4, data at lb 5), mode 0644;
    /// - `/emb.txt` ("hello", recorded in its extended file entry at lb 6);
    /// - `/sub/` (lb 7): 60 files over three extents at lb 8, 10 and 12;
    /// - `/edir/` (lb 13): three files, recorded in its extended file entry;
    /// - a deleted entry `gone`.
    pub fn image() -> Img {
        let mut img = Img(vec![0; (PSTART + PLEN) * B]);

        // Anchor Volume Descriptor Pointer at sector 256: the main sequence at 32, 16 sectors.
        img.tag(256, TAGID_ANCHOR, 256);
        img.u32(256, offset_of!(AnchorVdp, main_vds_ex), 16 * B as u32);
        img.u32(256, offset_of!(AnchorVdp, main_vds_ex) + 4, 32);

        // Primary volume descriptor, its identifier a dstring.
        img.tag(32, TAGID_PRI_VOL, 32);
        img.put(32, offset_of!(PriVolDesc, vol_id), &dstring(b"M10C"));

        // Partition descriptor: partition 0 at PSTART.
        img.tag(33, TAGID_PARTITION, 33);
        img.u16(33, offset_of!(PartDesc, part_num), 0);
        img.u32(33, offset_of!(PartDesc, start_loc), PSTART as u32);
        img.u32(33, offset_of!(PartDesc, part_len), PLEN as u32);

        // Logical volume descriptor: 2048-byte blocks, the FSD at lb 0, one type 1 map.
        img.tag(LVD, TAGID_LOGVOL, LVD as u32);
        img.u32(LVD, offset_of!(LogvolDesc, lb_size), B as u32);
        img.u32(LVD, offset_of!(LogvolDesc, _lvd_use), B as u32);
        img.u32(LVD, offset_of!(LogvolDesc, mt_l), 6);
        img.u32(LVD, offset_of!(LogvolDesc, n_pm), 1);
        img.put(LVD, LogvolDesc::SIZE, &[1, 6, 1, 0, 0, 0]);

        img.tag(35, TAGID_TERM, 35);

        // File set descriptor: the root's file entry at lb 2.
        img.tag(PSTART, TAGID_FSD, 0);
        img.u32(PSTART, offset_of!(FilesetDesc, rootdir_icb), B as u32);
        img.u32(PSTART, offset_of!(FilesetDesc, rootdir_icb) + 4, 2);

        // The root directory, its descriptors at lb 3.
        let mut root = Vec::new();
        root.extend(fid(UDF_FILE_CHAR_DIR | UDF_FILE_CHAR_PAR, b"", 2));
        root.extend(fid(0, b"m10c-udf.txt", 4));
        root.extend(fid(UDF_FILE_CHAR_DEL, b"gone", 4));
        root.extend(fid(UDF_FILE_CHAR_DIR, b"sub", 7));
        root.extend(fid(0, b"emb.txt", 6));
        root.extend(fid(UDF_FILE_CHAR_DIR, b"edir", 13));
        let ad = short_ad(root.len() as u32, 3);
        img.lb(
            2,
            &file_entry(2, false, 4, 0, 0x1ca5, 0, root.len() as u64, &ad),
        );
        img.lb(3, &root);

        // The file and its data.
        img.lb(
            4,
            &file_entry(4, false, 5, 0, 0x1884, 501, 12, &short_ad(12, 5)),
        );
        img.lb(5, TEXT);

        // A file recorded in its extended file entry.
        img.lb(6, &file_entry(6, true, 5, 3, 0x1884, u32::MAX, 5, b"hello"));

        // The fragmented directory.
        let mut sub = Vec::new();
        sub.extend(fid(UDF_FILE_CHAR_DIR | UDF_FILE_CHAR_PAR, b"", 2));
        for i in 0..NSUB {
            sub.extend(fid(0, &sub_name(i), 4));
        }
        let mut ads = Vec::new();
        ads.extend(short_ad(B as u32, 8));
        ads.extend(short_ad(B as u32, 10));
        ads.extend(short_ad((SUB_SIZE - 2 * B) as u32, 12));
        img.lb(
            7,
            &file_entry(7, false, 4, 0, 0x1ca5, 0, SUB_SIZE as u64, &ads),
        );
        img.lb(8, &sub[..B]);
        img.lb(10, &sub[B..2 * B]);
        img.lb(12, &sub[2 * B..]);

        // A directory recorded in its extended file entry.
        let mut edir = Vec::new();
        edir.extend(fid(UDF_FILE_CHAR_DIR | UDF_FILE_CHAR_PAR, b"", 2));
        for name in [b"a1", b"b2", b"c3"] {
            edir.extend(fid(0, name, 4));
        }
        let len = edir.len() as u64;
        img.lb(13, &file_entry(13, true, 4, 3, 0x1ca5, 0, len, &edir));

        img
    }

    /// The VAT's `lastblock` of [`vat_image`]: its file entry at lb 16 (`lb - um_start - 3`).
    pub const VAT_LASTBLOCK: u32 = (16 + PSTART + 3) as u32;

    /// [`image`] on write-once media: a virtual partition map, a VAT of 16 entries recorded
    /// in its extended file entry at lb 16 (`VAT_LASTBLOCK`), every entry the same block but
    /// entry 2, the root's file entry, moved to lb 15 (lb 2 is zero: only the VAT finds it).
    pub fn vat_image() -> Img {
        let mut img = image();
        img.map2(b"*UDF Virtual Partition\0", &[]);
        let root = img.0[(PSTART + 2) * B..(PSTART + 3) * B].to_vec();
        img.lb(15, &root);
        img.lb(2, &[0u8; B]);
        let mut vat = Vec::new();
        for i in 0..16u32 {
            vat.extend((if i == 2 { 15 } else { i }).to_le_bytes());
        }
        vat.extend([0u8; 36]);
        let len = vat.len() as u64;
        img.lb(16, &file_entry(16, true, 248, 3, 0, 0, len, &vat));
        img
    }

    /// A sparing table at `sector` mapping the packets `(org, map)` (sectors), `rt_l` its
    /// entry count.
    pub fn sparing_table(sector: u32, entries: &[(u32, u32)]) -> Vec<u8> {
        let mut t = vec![0u8; 56 + 8 * entries.len()];
        t[..16].copy_from_slice(&tag_bytes(TAGID_SPARING_TABLE, sector));
        t[48..50].copy_from_slice(&(entries.len() as u16).to_le_bytes());
        for (i, &(org, map)) in entries.iter().enumerate() {
            t[56 + 8 * i..60 + 8 * i].copy_from_slice(&org.to_le_bytes());
            t[60 + 8 * i..64 + 8 * i].copy_from_slice(&map.to_le_bytes());
        }
        t
    }

    /// [`image`] with one sparable partition map per table of `tables` (each at sector 50 +
    /// 2 i, packets of `packet_len` bytes: the C's `um_psecs` is `packet_len / um_bsize`).
    pub fn sparing_image(packet_len: u16, tables: &[Vec<u8>]) -> Img {
        let mut img = image();
        for (i, t) in tables.iter().enumerate() {
            let sector = 50 + 2 * i;
            img.put(sector, 0, t);
            // packet_len, n_st, reserved, st_size, st_loc[0], from the map's byte 40.
            let mut body = [0u8; 16];
            body[4..6].copy_from_slice(&packet_len.to_le_bytes());
            body[6] = 1;
            body[8..12].copy_from_slice(&(t.len() as u32).to_le_bytes());
            body[12..16].copy_from_slice(&(sector as u32).to_le_bytes());
            img.map2(b"*UDF Sparable Partition", &body);
        }
        img
    }

    /// The Metadata Partition's first block ([`meta_image`]).
    pub const META_START: u32 = 14;

    /// A volume whose file set lives in a Metadata Partition (UDF 2.50): the Metadata File's
    /// entry at lb 1 (the map's `meta_file_lbn`) names the partition from lb
    /// [`META_START`]; the file set descriptor at its lb 0 names the root at its lb 1 (lb 15,
    /// which the mount reads at `META_START + 1`); the root's descriptors at its lb 2 (lb
    /// 16); `/meta.txt`'s entry at its lb 3 (lb 17, found through the shifted start during
    /// the lookup) and its data at lb 18 (read through the partition's own start).
    pub fn meta_image() -> Img {
        let mut img = image();
        // meta_file_lbn (raw, as the C reads it), from the map's byte 40.
        let mut body = [0u8; 8];
        body[4..8].copy_from_slice(&1u32.to_le_bytes());
        img.map2(b"*UDF Metadata Partition", &body);
        let ms = META_START;
        let la = long_ad(6 * B as u32, ms);
        img.lb(1, &file_entry(1, false, 250, 1, 0, 0, 6 * B as u64, &la));
        let mut fsd = vec![0u8; B];
        fsd[..16].copy_from_slice(&tag_bytes(TAGID_FSD, 0));
        let at = offset_of!(FilesetDesc, rootdir_icb);
        fsd[at..at + 16].copy_from_slice(&long_ad(B as u32, 1));
        img.lb(ms as usize, &fsd);
        let mut root = Vec::new();
        root.extend(fid(UDF_FILE_CHAR_DIR | UDF_FILE_CHAR_PAR, b"", 1));
        root.extend(fid(0, b"meta.txt", 3));
        let ad = short_ad(root.len() as u32, 2);
        let len = root.len() as u64;
        img.lb(
            ms as usize + 1,
            &file_entry(1, false, 4, 0, 0x1ca5, 0, len, &ad),
        );
        img.lb(ms as usize + 2, &root);
        let fe = file_entry(3, false, 5, 0, 0x1884, 501, 12, &short_ad(12, 18));
        img.lb(ms as usize + 3, &fe);
        img.lb(18, TEXT);
        img
    }
}
/* </CODE> */
