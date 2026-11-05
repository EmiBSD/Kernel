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
 * Copyright (c) 1997 Manuel Bouyer.
 * Copyright (c) 1989, 1991, 1993
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
 *	@(#)ufs_bmap.c	8.6 (Berkeley) 1/21/94
 * Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! ext2fs block mapping: `ext2fs_bmap` converts the logical block number of a file to its
//! physical block number on the disk, through the inode's block pointers and indirect blocks
//! (`ext2fs_bmaparray`) or through its ext4 extent tree (`ext4_bmapext`).
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_bmap.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_bmap.rs@3c62ede68802
//!
//! Indirect blocks are on the vnode for the file. They are given negative logical block
//! numbers. Indirect blocks are addressed by the negative address of the first data block
//! to which they point. Double indirect blocks are addressed by one less than the address of
//! the first indirect block to which they point. Triple indirect blocks are addressed by one
//! less than the address of the first double indirect block to which they point.
//!
//! The vnode operation `ext2fs_bmap` over these two joins the module with the vnode
//! operations' tables (N5.6c2); until then `bsd`'s is the one the mounts call.
//!
//! ## Deviations
//! - `ext4_bmapext` and `ext2fs_bmaparray` take the `struct indir` array as a slice and its
//!   count as `Option<&mut i32>`, as `ufs_bmaparray` does; the block pointers of an indirect
//!   block are read at their index in the buffer's bytes, little-endian (`letoh32`).
//! - `ext4_bmapext` starts from an empty path (the C's is uninitialised) and releases the
//!   leaf buffer `ext4_ext_find_extent` may hold once the extent is read; the C keeps it
//!   busy for ever.
//! - `curproc->p_ru.ru_inblock++` is the vnode's owner's I/O charge (`charge_io`), as the
//!   kernel's `ufs_bmaparray`.
//!
//! ## Redesign
//! - LZ's lent indirect block (`&'static Buf`, its pointers read through an `unsafe`
//!   `BufLegacy::data`) and `VOP_STRATEGY` of a lent buffer -> the kernel's `ufs_bmaparray`
//!   shape: the block's token from `getblk`, `vop_strategy` of the token and `biowait` on
//!   its counted hold, the pointers read in the token's bytes. `VFSTOUFS(mp)` is the inode's
//!   own mount words (`blkptrtodb`, `is_sequential`, `MNINDIR`), borrowed: no count on the
//!   read path.

use core::sync::atomic::Ordering;

use hal::sys::errno::Errno;
use hal::sys::types::Daddr;

use super::ext2fs::fsbtodb;
use super::ext2fs_dinode::{NDADDR, NIADDR};
use super::ext2fs_extents::{Ext4ExtentPath, ext4_ext_find_extent};
use crate::assert::kpanic;
use crate::fs::bio::getblk_wait;
use crate::fs::bio::{BCSTATS, biowait, brelse, incore};
use crate::fs::buf::{B_DELWRI, B_DONE, B_READ, BufRef};
use crate::fs::ufs::ufs::inode::{Indir, le32_at, u32_at, vtoi};
use crate::fs::ufs::ufs::ufs_bmap::getlbns;
use crate::fs::vnode::Vnode;
use crate::fs::vops::{IoBuf, vop_strategy};
use crate::proc::ProcGlue;

/// `MAXBSIZE`: the largest transfer (`<sys/param.h>`).
const MAXBSIZE: i32 = 64 * 1024;

/// `ext4_bmapext`: logical block number of a file -> physical block number on disk within
/// ext4 extents. `*bnp` is -1 when the extent maps the block to disk block 0.
pub fn ext4_bmapext<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    bnp: &mut Daddr,
    _ap: Option<&mut [Indir]>,
    nump: Option<&mut i32>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fs = ip.i_e2fs();

    if let Some(runp) = runp {
        *runp = 0;
    }
    if let Some(nump) = nump {
        *nump = 0;
    }

    let mut path = Ext4ExtentPath::new();
    let ep = ext4_ext_find_extent(fs, &ip, bn, &mut path).and_then(|p| p.ext());
    if let Some(t) = path.ep_bp.take() {
        brelse(t);
    }
    let Some(ep) = ep else {
        return Err(Errno::EIO);
    };

    let pos = bn - Daddr::from(ep.e_blk)
        + ((Daddr::from(ep.e_start_hi) << 32) | Daddr::from(ep.e_start_lo));
    *bnp = fsbtodb(fs, pos);
    if *bnp == 0 {
        *bnp = -1;
    }
    Ok(())
}

/// `ext2fs_bmaparray`: the bmap conversion, and if requested the array of logical blocks
/// which must be traversed to get to a block. Each entry contains the offset into that
/// block that gets you to the next block and the disk address of the block (if it is
/// assigned). `*bnp` is -1 for a block that is not allocated (a hole); `*runp`, if asked
/// for, the number of blocks that follow it contiguously on the disk.
pub fn ext2fs_bmaparray<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mut bn: Daddr,
    bnp: &mut Daddr,
    ap: Option<&mut [Indir]>,
    nump: Option<&mut i32>,
    mut runp: Option<&mut i32>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let d = ip.i_e2din();
    let Some(mp) = vp.mount() else {
        kpanic(format_args!("ext2fs_bmaparray: vnode without a mount"));
    };

    #[cfg(feature = "diagnostic")]
    if ap.is_some() != nump.is_some() {
        kpanic(format_args!("ext2fs_bmaparray: invalid arguments"));
    }

    let mut maxrun = 0;
    if let Some(runp) = runp.as_deref_mut() {
        // XXX
        // If MAXBSIZE is the largest transfer the disks can handle, we probably want maxrun
        // to be 1 block less so that we don't create a block larger than the device can
        // handle.
        *runp = 0;
        maxrun = MAXBSIZE / mp.iosize() as i32 - 1;
    }

    let mut a = [Indir::default(); NIADDR + 1];
    let xap: &mut [Indir] = match ap {
        Some(ap) => ap,
        None => &mut a,
    };
    let mut num_local = 0;
    let nump = match nump {
        Some(n) => n,
        None => &mut num_local,
    };
    getlbns(ip.nindir() as i64, bn, xap, Some(&mut *nump))?;

    let mut num = *nump;
    if num == 0 {
        let blk = |i: Daddr| Daddr::from(u32::from_le(d.i_e2fs_block(i as usize)));
        *bnp = ip.blkptrtodb(blk(bn));
        if *bnp == 0 {
            *bnp = -1;
        } else if let Some(runp) = runp {
            bn += 1;
            while bn < NDADDR as Daddr && *runp < maxrun && ip.is_sequential(blk(bn - 1), blk(bn)) {
                bn += 1;
                *runp += 1;
            }
        }
        return Ok(());
    }

    // Get disk address out of indirect block array
    let mut daddr = u32::from_le(d.i_e2fs_block(NDADDR + xap[0].in_off as usize)) as i32;

    #[cfg(feature = "diagnostic")]
    if num > NIADDR as i32 + 1 || num < 1 {
        crate::prf::printf(format_args!("ext2fs_bmaparray: num={num}\n"));
        kpanic(format_args!("ext2fs_bmaparray: num"));
    }

    let mut bp: Option<BufRef> = None;
    let mut x = 1;
    loop {
        num -= 1;
        if num == 0 {
            break;
        }
        // Exit the loop if there is no disk address assigned yet and the indirect block
        // isn't in the cache, or if we were looking for an indirect block and we've found
        // it.
        let metalbn = xap[x].in_lbn;
        if (daddr == 0 && !incore(&vp.bufs, metalbn)) || metalbn == bn {
            break;
        }
        // If we get here, we've either got the block in the cache or we have a disk address
        // for it, go fetch it.
        if let Some(b) = bp.take() {
            brelse(b);
        }

        xap[x].in_exists = 1;
        let mut t = getblk_wait(&vp.bufs, metalbn, mp.iosize() as i32);
        if t.buf().isset(B_DONE | B_DELWRI) {
            // Already there.
        } else if cfg!(feature = "diagnostic") && daddr == 0 {
            kpanic(format_args!("ext2fs_bmaparry: indirect block not in cache"));
        } else {
            t.buf().set_blkno(ip.blkptrtodb(Daddr::from(daddr)));
            t.buf().set(B_READ);
            let Some(r) = t.counted() else {
                kpanic(format_args!("ext2fs_bmaparray: a driver's buffer"));
            };
            let _ = vop_strategy(vp, IoBuf::Token(t));
            vp.bufs.owner().charge_io(false); // XXX
            BCSTATS.pendingreads.fetch_add(1, Ordering::Relaxed);
            let (back, rv) = biowait(r);
            if let Err(e) = rv {
                brelse(back);
                return Err(e);
            }
            t = back;
        }

        let data = t.bytes();
        let off = xap[x].in_off as usize;
        daddr = le32_at(data, off) as i32;
        if num == 1
            && daddr != 0
            && let Some(runp) = runp.as_deref_mut()
        {
            let mut i = off + 1;
            while (i as u64) < ip.nindir()
                && *runp < maxrun
                && ip.is_sequential(
                    Daddr::from(u32_at(data, i - 1)),
                    Daddr::from(u32_at(data, i)),
                )
            {
                i += 1;
                *runp += 1;
            }
        }
        bp = Some(t);
        x += 1;
    }
    if let Some(b) = bp {
        brelse(b);
    }

    let daddr = ip.blkptrtodb(Daddr::from(daddr));
    *bnp = if daddr == 0 { -1 } else { daddr };
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the block maps over the test image: `ext2fs_bmaparray` through the
    // direct pointers and the single indirect block (the runs of contiguous blocks, the
    // path of `struct indir`s, a hole), and `ext4_bmapext` through a two-level extent tree,
    // and a corrupt leaf refused with `EIO`.

    use std::assert_eq;

    use super::super::testext2fs::newfs::{self, B, BIG0, EXTLEAF};
    use super::super::testext2fs::{attach, mount, vget};
    use super::*;
    use crate::fs::subr::vput;
    use hal::sys::mount::MNT_RDONLY;

    #[test]
    fn the_block_pointers_and_the_indirect_block_map_a_file() {
        let (_g, _p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = vget(mp, 13).unwrap();
        let db = |b: usize| (b * B / 512) as Daddr;
        let mut bn = 0;
        let mut run = 0;
        assert_eq!(
            ext2fs_bmaparray(vp, 0, &mut bn, None, None, Some(&mut run)),
            Ok(())
        );
        // Blocks 11..=22 are contiguous: a run of 11 after the first.
        assert_eq!((bn, run), (db(BIG0), 11));
        assert_eq!(
            ext2fs_bmaparray(vp, 11, &mut bn, None, None, Some(&mut run)),
            Ok(())
        );
        assert_eq!((bn, run), (db(BIG0 + 11), 0));
        // Behind the indirect block: 24, then 25 contiguous.
        let mut ap = [Indir::default(); NIADDR + 1];
        let mut num = 0;
        assert_eq!(
            ext2fs_bmaparray(
                vp,
                12,
                &mut bn,
                Some(&mut ap[..]),
                Some(&mut num),
                Some(&mut run)
            ),
            Ok(())
        );
        assert_eq!((bn, run, num), (db(24), 1, 2));
        assert_eq!((ap[1].in_lbn, ap[1].in_off, ap[1].in_exists), (-12, 0, 1));
        assert_eq!(ext2fs_bmaparray(vp, 13, &mut bn, None, None, None), Ok(()));
        assert_eq!(bn, db(25));
        // Past the file's blocks: a hole, the indirect block's pointer 2 is 0.
        assert_eq!(ext2fs_bmaparray(vp, 14, &mut bn, None, None, None), Ok(()));
        assert_eq!(bn, -1);
        vput(vp);
        // A hole in the direct pointers.
        let vp = vget(mp, 12).unwrap();
        assert_eq!(ext2fs_bmaparray(vp, 3, &mut bn, None, None, None), Ok(()));
        assert_eq!(bn, -1);
        vput(vp);
    }

    #[test]
    fn an_extent_tree_maps_a_file() {
        let (_g, _p) = attach(newfs::image(true));
        let mp = mount(MNT_RDONLY);
        let vp = vget(mp, 14).unwrap();
        let db = |b: Daddr| b * (B / 512) as Daddr;
        let mut bn = 0;
        let mut run = 7;
        for (lbn, want) in [(0, 27), (1, 28), (2, 30), (3, 31)] {
            assert_eq!(
                ext4_bmapext(vp, lbn, &mut bn, None, None, Some(&mut run)),
                Ok(())
            );
            assert_eq!((bn, run), (db(want), 0), "lbn {lbn}");
        }
        vput(vp);
    }

    #[test]
    fn a_corrupt_extent_leaf_is_eio() {
        let mut img = newfs::image(true);
        // 85 entries: one more than a 1 KiB leaf holds after its header.
        img[EXTLEAF * B + 2..EXTLEAF * B + 4].copy_from_slice(&85u16.to_le_bytes());
        let (_g, _p) = attach(img);
        let mp = mount(MNT_RDONLY);
        let mut bn = 0;
        let vp = vget(mp, 14).unwrap();
        assert_eq!(
            ext4_bmapext(vp, 0, &mut bn, None, None, None),
            Err(Errno::EIO)
        );
        vput(vp);
    }
}
/* </TESTS> */
