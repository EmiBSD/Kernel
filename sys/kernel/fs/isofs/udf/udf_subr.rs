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
 * Copyright (c) 2006, Miodrag Vallat
 * Copyright (c) 2006, Pedro Martelletto
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
 * (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
 * STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! UDF helpers: CS0 names to Unicode, the volume name a UDF medium's spoofed disk label
//! takes (`udf_disklabelspoof`'s parse), the logical block size a mount accepts, and the
//! virtual allocation table (VAT) of write-once media.
//!
//! Upstream: sys/isofs/udf/udf_subr.c @ 3ce1f3f79392
//! LZ: sys/isofs/udf/udf_subr.rs@3c62ede68802
//!
//! ## Deviations
//! - `udf_rawnametounicode` returns `Err(EINVAL)` where the C returns -1, and the number of
//!   characters otherwise; it also fails when the name does not fit in `cs0string` or in
//!   `transname` (the C trusts its callers). The bytes of the name are unsigned on both
//!   archs (as the C's `char` is on arm64); on amd64 the C's signed `char` sign-extends a
//!   byte >= 0x80, so such a byte of an 8-bit name came out as `?`.
//! - `udf_disklabelspoof` is split (D-N5.8-2): [`udf_volume_id`] here reads the anchor and
//!   the volume descriptor sequence through a reader and returns the primary volume
//!   descriptor's name; `bsd`'s `kern/subr_disk.rs` keeps the shell that lends the C's
//!   buffer to the driver's strategy and fills the disk label (N5.8u2). A sequence whose end
//!   wraps below its start finds no descriptor (`EINVAL`), where the C reads the descriptor
//!   through an uninitialised pointer.
//! - [`udf_lb_size`] refuses a logical block size outside `DEV_BSIZE..=MAXBSIZE` or not a
//!   power of two with `EINVAL`, which `udf_mountfs` (N5.8u2) answers where the C takes
//!   `ffs(um_bsize) - 1` of any size (a shift by -1 for 0, a division by zero in
//!   `udf_get_spartmap`, the wrong blocks for a size not a power of two: the user's
//!   D-N5.8-U3).
//! - `udf_vat_get`'s `M_UDFMOUNT` block holds a counted handle of the VAT's unode, not a
//!   structure copy: the C's `*ump->um_vat = *up` shares `u_fentry` with the cached unode,
//!   which `udf_reclaim` frees while the copy keeps reading it (LZ copied the file entry into
//!   a second, `M_UDFFENTRY` block, a request the C does not make). The handle keeps the
//!   unode, its file entry and its pool item alive until the unmount (or the mount's error
//!   path) takes the block: `vmstat -m`'s requests are the C's, and when the VAT's vnode is
//!   reclaimed during the mount its unode and file entry are released at the unmount instead.
//!   Its size is the handle's, not `sizeof(struct unode)`.
//! - `udf_vat_get` takes `udf_vget` as an argument until the mount lands (N5.8u2, which
//!   drops it).
//! - `udf_vat_map` answers `EINVAL` for a volume that uses a VAT but has none (the C reads
//!   through `NULL`); `udf_vat_read` releases the buffer only when there is one: a VAT
//!   recorded in its file entry has none (the C would `brelse(NULL)`).
//!
//! ## Redesign
//! - LZ's spoof read `b_data` through two `unsafe` views of the buffer it lent the driver ->
//!   the parse over a reader closure, a 56-byte buffer for the anchor's and each
//!   descriptor's head (docs/IDIOMS.md, the label spoof's row; cd9660's `iso_volume_label`
//!   the precedent).
//! - LZ's VAT copy (a raw `malloc` of a `Unode` written with `ptr::write`, a raw `malloc` of
//!   the file entry copied with `copy_nonoverlapping`, `udf_vat_free`'s raw frees) -> an
//!   `MBox<UnodeRef>` in the mount's `um_vat` slot (the module's deviations); LZ's
//!   `udf_data` slice of the buffer (`unsafe`) -> the `BufRef` token's bytes. 0 `unsafe`
//!   (LZ: 5).

use core::mem::offset_of;

use hal::malloc::{M_WAITOK, MBox};
use hal::sys::endian::{letoh32, letoh64};
use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Ino, Off};

use crate::assert::kpanic;
use crate::fs::isofs::udf::ecma167_udf::{
    AnchorVdp, DescTag, PriVolDesc, TAGID_ANCHOR, TAGID_PRI_VOL,
};
use crate::fs::isofs::udf::udf::{
    M_UDFMOUNT, UDF_MNT_FIND_VAT, UDF_MNT_USES_VAT, Umount, Unicode, VTOU,
};
use crate::fs::isofs::udf::udf_vfsops::udf_checktag;
use crate::fs::isofs::udf::udf_vnops::{udf_readatoffset, udf_transname};
use crate::fs::mount::Mount;
use crate::fs::subr::vput;
use crate::fs::vnode::Vnode;
use crate::proc::ProcGlue;

/// `DEV_BSIZE`.
const DEV_BSIZE: u32 = 512;
/// `MAXBSIZE` (`<sys/param.h>`).
const MAXBSIZE: u32 = 64 * 1024;

/// The block size the spoof reads (`bsize = 2048`, the C's).
const SPOOF_BSIZE: u32 = 2048;
/// `btodb(bsize)`: the disk blocks of one.
const SPOOF_DBLOCKS: Daddr = (SPOOF_BSIZE / DEV_BSIZE) as Daddr;
/// The bytes the parse reads of a block: the tag and the anchor's extents, the tag and the
/// primary volume descriptor up to its `vol_id`.
const SPOOF_HEAD: usize = 56;
/// Where the primary volume descriptor's `vol_id` is.
const PVD_VOL_ID: usize = offset_of!(PriVolDesc, vol_id);

/// A reader of [`udf_volume_id`]'s: `read(blkno, buf)` fills `buf` with the first bytes of
/// the 2,048-byte block at the disk block `blkno` (the spoof's shell reads through the
/// driver's strategy; a test, an image).
pub type UdfLabelRead<'a> = dyn FnMut(Daddr, &mut [u8]) -> Result<(), Errno> + 'a;

/// `udf_vget(mp, ino, &vp)` (`udf_vfsops.c`, N5.8u2): the locked vnode of the file entry
/// `ino` of `mp`.
pub type UdfVget<G> = fn(&'static Mount<G>, Ino) -> Result<&'static Vnode<G>, Errno>;

/// Convert a CS0 dstring to a 16-bit Unicode string. Returns the length of the Unicode
/// string, in unicode characters (not bytes!). Note that the transname destination buffer is
/// expected to be large enough to hold the result, and will not be terminated in any way.
pub fn udf_rawnametounicode(
    len: u32,
    cs0string: &[u8],
    transname: &mut [Unicode],
) -> Result<usize, Errno> {
    let Some(len) = len.checked_sub(1) else {
        return Err(Errno::EINVAL);
    };
    let len = len as usize;
    let Some((&kind, name)) = cs0string.split_first() else {
        return Err(Errno::EINVAL);
    };
    let Some(name) = name.get(..len) else {
        return Err(Errno::EINVAL);
    };

    match kind {
        8 => {
            // bytes string
            let Some(out) = transname.get_mut(..len) else {
                return Err(Errno::EINVAL);
            };
            for (t, &c) in out.iter_mut().zip(name) {
                *t = Unicode::from(c);
            }
            Ok(len)
        }
        16 => {
            // 16 bit unicode string
            if len & 1 != 0 {
                return Err(Errno::EINVAL);
            }
            let n = len >> 1;
            let Some(out) = transname.get_mut(..n) else {
                return Err(Errno::EINVAL);
            };
            for (t, c) in out.iter_mut().zip(name.as_chunks::<2>().0) {
                *t = (Unicode::from(c[0]) << 8) | Unicode::from(c[1]);
            }
            Ok(n)
        }
        _ => Err(Errno::EINVAL),
    }
}

/// Do a lazy probe on the underlying media to check if it's a UDF volume, in which case we
/// fake a disk label for it: `udf_disklabelspoof`'s test of the device. The anchor volume
/// descriptor at sector 256, then the first primary volume descriptor of its main volume
/// descriptor sequence; its `vol_id` translated (`udf_transname`), `None` when that fails
/// (the C leaves `d_typename` as it was). Any read error, as a missing descriptor, is
/// `EINVAL` (the C's).
pub fn udf_volume_id<G: ProcGlue>(read: &mut UdfLabelRead<'_>) -> Result<Option<[u8; 32]>, Errno> {
    let mut b = [0u8; SPOOF_HEAD];

    // Look for an Anchor Volume Descriptor at sector 256.
    if read(256 * SPOOF_DBLOCKS, &mut b).is_err() {
        return Err(Errno::EINVAL);
    }
    let Some(tag) = DescTag::at(&b, 0) else {
        return Err(Errno::EINVAL);
    };
    if udf_checktag(&tag, TAGID_ANCHOR).is_err() {
        return Err(Errno::EINVAL);
    }
    let Some(avdp) = AnchorVdp::at(&b, 0) else {
        return Err(Errno::EINVAL);
    };
    let mvds_start = letoh32(avdp.main_vds_ex.loc);
    let mvds_end =
        mvds_start.wrapping_add(letoh32(avdp.main_vds_ex.len).wrapping_sub(1) / SPOOF_BSIZE);

    // Then try to find a reference to a Primary Volume Descriptor.
    let mut vol_id = None;
    let mut sector = mvds_start;
    while sector < mvds_end {
        // `sector * btodb(bsize)`, the C's `uint32_t` product.
        let blkno = Daddr::from(sector.wrapping_mul(SPOOF_DBLOCKS as u32));
        if read(blkno, &mut b).is_err() {
            return Err(Errno::EINVAL);
        }
        if let Some(tag) = DescTag::at(&b, 0)
            && udf_checktag(&tag, TAGID_PRI_VOL).is_ok()
            && let Some(id) = b.get(PVD_VOL_ID..SPOOF_HEAD)
        {
            let mut v = [0u8; 32];
            v.copy_from_slice(id);
            vol_id = Some(v);
            break;
        }
        sector += 1;
    }

    // If we couldn't find a reference, bail out.
    let Some(vol_id) = vol_id else {
        return Err(Errno::EINVAL);
    };

    // Okay, it's a UDF volume.
    let mut vid = [0u8; 32];
    if udf_transname::<G>(&vol_id, &mut vid, vol_id.len() as i32 - 1, None) != 0 {
        Ok(Some(vid))
    } else {
        Ok(None)
    }
}

/// The logical block size `udf_mountfs` takes from a logical volume descriptor's `lb_size`
/// (`um_bsize`, whose `um_bmask` and `um_bshift` follow): `EINVAL` for a size outside
/// `DEV_BSIZE..=MAXBSIZE` or not a power of two (the module's deviations, D-N5.8-U3).
pub fn udf_lb_size(lb_size: u32) -> Result<i32, Errno> {
    if !(DEV_BSIZE..=MAXBSIZE).contains(&lb_size) || !lb_size.is_power_of_two() {
        return Err(Errno::EINVAL);
    }
    // At most `MAXBSIZE`: an `int`.
    Ok(lb_size as i32)
}

/// Get a vnode for the Virtual Allocation Table (VAT).
pub fn udf_vat_get<G: ProcGlue>(ump: &Umount<G>, lb: u32, vget: UdfVget<G>) -> Result<(), Errno> {
    let ino = lb.wrapping_sub(ump.um_start.get()).wrapping_sub(3);
    let vp = vget(ump.um_mountp, Ino::from(ino))?;

    let up = VTOU(vp);
    // The C's `uint64_t` arithmetic, then a `long`.
    up.set_u_vatlen((letoh64(up.fe.inf_len).wrapping_sub(36) >> 2) as i64);

    // ump->um_vat = malloc(sizeof(struct unode), M_UDFMOUNT, M_WAITOK); *ump->um_vat = *up
    // (the module's deviations: the node itself, counted).
    let Some(vat) = MBox::new(M_UDFMOUNT, M_WAITOK, up) else {
        kpanic(format_args!("udf_vat_get: malloc(M_UDFMOUNT) failed"));
    };
    drop(ump.um_vat.replace(Some(vat)));

    ump.um_flags
        .set((ump.um_flags.get() & !UDF_MNT_FIND_VAT) | UDF_MNT_USES_VAT);

    vput(vp);

    Ok(())
}

/// Look up a sector in the VAT.
pub fn udf_vat_map<G: ProcGlue>(ump: &Umount<G>, sector: &mut u32) -> Result<(), Errno> {
    // If there's no VAT, then it's easy
    if ump.um_flags.get() & UDF_MNT_USES_VAT == 0 {
        *sector = sector.wrapping_add(ump.um_start.get());
        return Ok(());
    }

    // Sanity check the given sector
    let Some(vat) = ump.vat() else {
        return Err(Errno::EINVAL);
    };
    if i64::from(*sector) >= vat.u_vatlen() {
        return Err(Errno::EINVAL);
    }

    udf_vat_read(ump, sector)
}

/// Read from the VAT.
fn udf_vat_read<G: ProcGlue>(ump: &Umount<G>, sector: &mut u32) -> Result<(), Errno> {
    let Some(vat) = ump.vat() else {
        return Err(Errno::EINVAL);
    };
    let mut size = 4;

    // Note that we rely on the buffer cache to keep frequently accessed buffers around to
    // avoid reading them from the disk all the time. (`*sector << 2` is the C's `uint32_t`
    // shift.)
    let data = udf_readatoffset(&vat, &mut size, Off::from(sector.wrapping_shl(2)))?;

    // Make sure we read at least a whole entry
    if size < 4 {
        return Err(Errno::EINVAL);
    }

    // Map the sector
    let Some(&[a, b, c, d]) = data.bytes(&vat, size).first_chunk::<4>() else {
        return Err(Errno::EINVAL);
    };
    *sector = letoh32(u32::from_ne_bytes([a, b, c, d])).wrapping_add(ump.um_start.get());

    // brelse(bp): the token, if any, goes with `data`.
    drop(data);

    Ok(())
}

const _: () = assert!(PVD_VOL_ID + 32 == SPOOF_HEAD);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use hal::pool::PoolArc;

    use super::*;
    use crate::fs::isofs::udf::testudf::mkudf::{B, dstring, file_entry, volume};
    use crate::fs::isofs::udf::testudf::{attach, test_node, test_ump};
    use crate::fs::isofs::udf::udf::VFSTOUDFFS;
    use crate::fs::isofs::udf::udf_vnops::{udf_hashins, udf_hashlookup};
    use crate::fs::subr::vrele;
    use crate::fs::testfs::TestGlue;
    use crate::fs::vnode::LK_EXCLUSIVE;
    use crate::fs::vops::vop_unlock;
    use crate::testutil::malloc_requests;

    #[test]
    fn cs0_names_become_unicode() {
        let mut t = [0u16; 8];
        assert_eq!(udf_rawnametounicode(5, b"\x08abcd", &mut t), Ok(4));
        assert_eq!(&t[..4], &[0x61, 0x62, 0x63, 0x64]);
        assert_eq!(udf_rawnametounicode(2, b"\x08\xe9", &mut t), Ok(1));
        assert_eq!(t[0], 0xe9);
        assert_eq!(udf_rawnametounicode(5, b"\x10\x00A\x26\x3a", &mut t), Ok(2));
        assert_eq!(&t[..2], &[0x41, 0x263a]);
        // Odd 16-bit length, unknown compression id, empty, longer than the buffer, longer
        // than the string.
        assert_eq!(
            udf_rawnametounicode(4, b"\x10abc", &mut t),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            udf_rawnametounicode(2, b"\x07a", &mut t),
            Err(Errno::EINVAL)
        );
        assert_eq!(udf_rawnametounicode(0, b"", &mut t), Err(Errno::EINVAL));
        assert_eq!(
            udf_rawnametounicode(10, b"\x08abcdefghi", &mut t),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            udf_rawnametounicode(9, b"\x08ab", &mut t),
            Err(Errno::EINVAL)
        );
    }

    /// `udf_volume_id` over `img`, the disk blocks it read beside its answer.
    fn volume_id(
        img: &[u8],
        fail_at: Option<Daddr>,
    ) -> (Result<Option<[u8; 32]>, Errno>, Vec<Daddr>) {
        let mut reads = Vec::new();
        let r = udf_volume_id::<TestGlue>(&mut |blkno, buf: &mut [u8]| {
            reads.push(blkno);
            let o = blkno as usize * DEV_BSIZE as usize;
            match img.get(o..o + buf.len()) {
                Some(b) if Some(blkno) != fail_at => {
                    buf.copy_from_slice(b);
                    Ok(())
                }
                _ => Err(Errno::EIO),
            }
        });
        (r, reads)
    }

    #[test]
    fn the_volume_id_is_the_primary_descriptor_s() {
        let (_g, _p) = attach(vec![0; B]);
        // The anchor at 256, the sequence at 32 (16 sectors), the descriptor at 33.
        let img = volume(16 * B as u32, 33, &dstring(b"M10C"));
        let (r, reads) = volume_id(&img, None);
        let vid = r.unwrap().unwrap();
        assert_eq!(&vid[..5], b"M10C\0");
        assert_eq!(reads, [1024, 128, 132]);

        // Not UDF: no anchor.
        let mut bare = img.clone();
        bare[256 * B..257 * B].fill(0);
        assert_eq!(volume_id(&bare, None).0, Err(Errno::EINVAL));
        // The descriptor past the sequence's end ((len - 1) / bsize sectors are read).
        assert_eq!(
            volume_id(&volume(2 * B as u32, 33, &dstring(b"M10C")), None).0,
            Err(Errno::EINVAL)
        );
        // A read error, of the anchor or of the sequence.
        assert_eq!(volume_id(&img, Some(1024)).0, Err(Errno::EINVAL));
        assert_eq!(volume_id(&img, Some(128)).0, Err(Errno::EINVAL));
        // A name the translation refuses: the label is spoofed, d_typename left alone.
        let mut bad = dstring(b"M10C");
        bad[0] = 9;
        assert_eq!(
            volume_id(&volume(16 * B as u32, 33, &bad), None).0,
            Ok(None)
        );
        // A sequence whose end wraps below its start: nothing read past the anchor.
        let mut wrap = img.clone();
        let at = 256 * B + 16;
        wrap[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        wrap[at + 4..at + 8].copy_from_slice(&0xffff_fff0u32.to_le_bytes());
        let (r, reads) = volume_id(&wrap, None);
        assert_eq!((r, reads.len()), (Err(Errno::EINVAL), 1));
    }

    #[test]
    fn the_logical_block_size_is_checked() {
        for (lb, want) in [
            (0, Err(Errno::EINVAL)),
            (256, Err(Errno::EINVAL)),
            (512, Ok(512)),
            (1000, Err(Errno::EINVAL)),
            (2048, Ok(2048)),
            (4096, Ok(4096)),
            (65_536, Ok(65_536)),
            (65_537, Err(Errno::EINVAL)),
            (131_072, Err(Errno::EINVAL)),
            (u32::MAX, Err(Errno::EINVAL)),
        ] {
            assert_eq!(udf_lb_size(lb), want, "lb_size {lb}");
        }
    }

    /// `udf_vget` as `udf_vat_get` calls it: the unodes the test hashed.
    fn hashed_vget(
        mp: &'static Mount<TestGlue>,
        ino: Ino,
    ) -> Result<&'static Vnode<TestGlue>, Errno> {
        udf_hashlookup(&VFSTOUDFFS(mp), ino as u32, LK_EXCLUSIVE)?.ok_or(Errno::ENOENT)
    }

    #[test]
    fn the_vat_maps_sectors_through_its_entries() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|u| {
            u.um_start.set(1000);
            u.um_flags.set(UDF_MNT_FIND_VAT);
        });
        // A VAT of three entries and the 36-byte tail, recorded in its extended file entry.
        let mut vat = Vec::new();
        for e in [7u32, 8, 9] {
            vat.extend(e.to_le_bytes());
        }
        vat.extend([0u8; 36]);
        let fe = file_entry(50, true, 248, 3, 0, 0, vat.len() as u64, &vat);
        let (up, vp) = test_node(&ump, 50, &fe);
        udf_hashins(&up).unwrap();
        vop_unlock(vp).unwrap();
        vrele(vp);

        let (n, _) = malloc_requests(M_UDFMOUNT);
        // The VAT's file entry is at lb - um_start - 3.
        udf_vat_get(&ump, 1053, hashed_vget).unwrap();
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n + 1);
        assert_eq!(ump.um_flags.get(), UDF_MNT_USES_VAT);
        assert_eq!(up.u_vatlen(), 3);
        assert!(ump.vat().is_some_and(|v| PoolArc::ptr_eq(&v, &up)));
        // The vnode went back unlocked (vput).
        assert_eq!(up.u_lock.status(), 0);

        for (s, want) in [
            (0, Ok(1007)),
            (1, Ok(1008)),
            (2, Ok(1009)),
            (3, Err(Errno::EINVAL)),
        ] {
            let mut sector = s;
            assert_eq!(
                udf_vat_map(&ump, &mut sector).map(|()| sector),
                want,
                "sector {s}"
            );
        }

        // No VAT: the partition's start.
        let plain = test_ump(|u| u.um_start.set(20));
        let mut sector = 5;
        assert_eq!(udf_vat_map(&plain, &mut sector), Ok(()));
        assert_eq!(sector, 25);
        // A VAT volume without its VAT node.
        let lost = test_ump(|u| u.um_flags.set(UDF_MNT_USES_VAT));
        let mut sector = 1;
        assert_eq!(udf_vat_map(&lost, &mut sector), Err(Errno::EINVAL));
        // No such file entry: the VAT get fails as udf_vget does.
        let missing = test_ump(|u| u.um_flags.set(UDF_MNT_FIND_VAT));
        assert_eq!(udf_vat_get(&missing, 99, hashed_vget), Err(Errno::ENOENT));
        assert_eq!(missing.um_flags.get(), UDF_MNT_FIND_VAT);

        // The unmount's take ends the VAT's hold on its node.
        drop(ump.um_vat.replace(None));
    }
}
/* </TESTS> */
