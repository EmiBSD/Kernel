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
//! The kernel udf's test fixtures (host tests and benches only): [`mkudf`], the first part
//! of LZ's in-memory UDF builder (`udf_vnops.rs`'s tests: the tags, the time stamps, the file
//! entries and allocation descriptors, the anchor and the primary volume descriptor), whose
//! whole image moves here with the mount (N5.8u2); and, for the tests that need no mount, a
//! mount structure over testffs's device ([`test_ump`]) and unodes of it with vnodes of a
//! small table ([`TestUdfVops`]: the unode's lock, and `udf_reclaim`'s steps).

use std::sync::MutexGuard;
use std::vec::Vec;

use hal::malloc::{M_WAITOK, MArc, MArray};
use hal::pool::PoolArc;
use hal::sys::errno::Errno;

use crate::fs::isofs::udf::udf::{
    M_UDFFENTRY, M_UDFMOUNT, UDF_HASHTBLSIZE, UdfHash, Udfino, Umount, UmountRef, Unode, UnodeRef,
    VTOU,
};
use crate::fs::isofs::udf::udf_vfsops::udf_init;
use crate::fs::isofs::udf::udf_vnops::udf_hashrem;
use crate::fs::mount::vfs_mount_alloc;
use crate::fs::subr::{getnewvnode, hashinit, siphash_key, vref, vrele};
use crate::fs::testfs::{TEST_CONF, TestGlue};
use crate::fs::ufs::ffs::testffs;
use crate::fs::vnode::{LK_RWFLAGS, VT_UDF, Vnode};
use crate::fs::vops::{Cred, Vnops};
use crate::proc::Thread;

/// The vnode table of the fixture's unodes: the unode's lock, `access` granted, `inactive`
/// the C's unlock, and `udf_reclaim`'s steps (off the hash, the device vnode given back, the node
/// out of the vnode); every other slot the C's NULL.
pub struct TestUdfVops;

impl Vnops<TestGlue> for TestUdfVops {
    fn lock(&self, vp: &'static Vnode<TestGlue>, flags: i32) -> Result<(), Errno> {
        VTOU(vp).u_lock.enter(flags & LK_RWFLAGS)
    }
    fn unlock(&self, vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        VTOU(vp).u_lock.exit();
        Ok(())
    }
    fn islocked(&self, vp: &'static Vnode<TestGlue>) -> i32 {
        VTOU(vp).u_lock.status()
    }
    fn access(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn inactive(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        // udf_inactive: no need to sync anything, so just unlock the vnode and return.
        VTOU(vp).u_lock.exit();
        Ok(())
    }
    fn reclaim(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        let up = VTOU(vp);
        udf_hashrem(&up)?;
        if let Some(devvp) = up.u_devvp.get() {
            vrele(devvp);
            up.u_devvp.set(None);
        }
        drop(vp.take_node());
        Ok(())
    }
}

/// The table, a `static` (one address for the registry, D-b2-10).
pub static TEST_UDF_VOPS: TestUdfVops = TestUdfVops;

/// `image` as the disk of testffs's block device, and udf's pools made: the memory guard
/// and the test thread.
pub fn attach(image: Vec<u8>) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let (g, p) = testffs::attach(image);
    if let Err(e) = udf_init::<TestGlue>() {
        std::panic!("testudf: udf_init: {e:?}");
    }
    (g, p)
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
/// own of [`TEST_UDF_VOPS`] (referenced, unlocked), its device vnode referenced, as
/// `udf_vget` links them; not hashed.
pub fn test_node(
    ump: &UmountRef<TestGlue>,
    ino: Udfino,
    entry: &[u8],
) -> (UnodeRef<TestGlue>, &'static Vnode<TestGlue>) {
    let vp = match getnewvnode(VT_UDF, Some(ump.um_mountp), &TEST_UDF_VOPS) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testudf: getnewvnode: {e:?}"),
    };
    let up = Unode::new(vp, ump.clone(), ino, fentry_copy(entry)).alloc();
    vp.set_node(PoolArc::erase(up.clone()));
    up.u_devvp.set(Some(ump.devvp()));
    vref(ump.devvp());
    (up, vp)
}

/// `mkudf`: the parts of LZ's in-memory UDF builder (`udf_vnops.rs`'s tests) the tests of
/// the descriptors, the label spoof's parse, the node and the VAT need: 2048-byte sectors.
pub mod mkudf {
    use core::mem::offset_of;

    use std::vec;
    use std::vec::Vec;

    use crate::fs::isofs::udf::ecma167_udf::{
        AnchorVdp, ExtfileEntry, FileEntry, PriVolDesc, TAGID_ANCHOR, TAGID_EXTFENTRY,
        TAGID_FENTRY, TAGID_PRI_VOL, UDF_EXTFENTRY_SIZE, UDF_FENTRY_SIZE,
    };

    /// The sector size.
    pub const B: usize = 2048;

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
        let mut img = vec![0u8; 257 * B];
        let mut put = |sector: usize, off: usize, b: &[u8]| {
            let o = sector * B + off;
            img[o..o + b.len()].copy_from_slice(b);
        };
        put(256, 0, &tag_bytes(TAGID_ANCHOR, 256));
        put(
            256,
            offset_of!(AnchorVdp, main_vds_ex),
            &vds_len.to_le_bytes(),
        );
        put(
            256,
            offset_of!(AnchorVdp, main_vds_ex) + 4,
            &32u32.to_le_bytes(),
        );
        put(pvd_at, 0, &tag_bytes(TAGID_PRI_VOL, pvd_at as u32));
        put(pvd_at, offset_of!(PriVolDesc, vol_id), vol_id);
        img
    }

    /// A dstring of 32 bytes: `name` in 8-bit CS0, its length in the last byte.
    pub fn dstring(name: &[u8]) -> [u8; 32] {
        let mut d = [0u8; 32];
        d[0] = 8;
        d[1..=name.len()].copy_from_slice(name);
        d[31] = name.len() as u8 + 1;
        d
    }
}
/* </CODE> */
