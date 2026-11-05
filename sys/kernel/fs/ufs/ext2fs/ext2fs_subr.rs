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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)ffs_subr.c	8.2 (Berkeley) 9/21/93
 * Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! ext2fs helpers: the directory block of an offset (`ext2fs_bufatoff`, through the extent
//! tree when the inode has one) and the vnode set-up of a new inode (`ext2fs_vinit`).
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_subr.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_subr.rs@3c62ede68802
//!
//! `ext2fs_vinit` joins the module with the vnode operations' tables (N5.6c2); until then
//! `bsd`'s is the one the mounts call.
//!
//! ## Deviations
//! - `ext2fs_bufatoff` returns the buffer and the offset of `*res` in its data, as
//!   `ffs_bufatoff` does (the C's optional `res` is always computed).
//! - When the inode has an extent tree but no extent covers the block, the C goes on to the
//!   normal path without releasing the leaf buffer `ext4_ext_find_extent` may hold; here it
//!   is released first.
//!
//! ## Redesign
//! - LZ's lent buffers (`&'static Buf`) -> the kernel cache's tokens: `ext2fs_bufatoff`
//!   returns the `BufRef` `bread` gave (the kernel's `ffs_bufatoff` shape), and the extent
//!   path's leaf is a token released by its owner.

use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Off};

use super::ext2fs::{blkoff, fsbtodb, lblkno};
use super::ext2fs_dinode::EXT4_EXTENTS;
use super::ext2fs_extents::{Ext4ExtentPath, ext4_ext_find_extent};
use crate::fs::bio::{bread, brelse};
use crate::fs::buf::BufRef;
use crate::fs::ufs::ufs::inode::Inode;
use crate::proc::ProcGlue;

/// `ext2fs_bufatoff`: the buffer with the contents of block `offset` from the beginning of
/// directory `ip`, and the offset in it of the remaining space in the directory (`*res`).
pub fn ext2fs_bufatoff<G: ProcGlue>(ip: &Inode<G>, offset: Off) -> Result<(BufRef, usize), Errno> {
    let vp = ip.itov();
    let fs = ip.i_e2fs();
    let lbn = lblkno(fs, offset);

    if ip.i_e2din().i_e2fs_flags() & EXT4_EXTENTS != 0 {
        let mut path = Ext4ExtentPath::new();
        let ep = ext4_ext_find_extent(fs, ip, lbn, &mut path).and_then(|p| p.ext());
        if let Some(t) = path.ep_bp.take() {
            brelse(t);
        }

        if let Some(ep) = ep {
            let pos = lbn - Daddr::from(ep.e_blk)
                + ((Daddr::from(ep.e_start_hi) << 32) | Daddr::from(ep.e_start_lo));
            let (t, error) = bread(&ip.i_devvp().bufs, fsbtodb(fs, pos), fs.e2fs_bsize.get());
            if let Err(e) = error {
                brelse(t);
                return Err(e);
            }
            return Ok((t, blkoff(fs, offset) as usize));
        }
    }

    // normal:
    let (t, error) = bread(&vp.bufs, lbn, fs.e2fs_bsize.get());
    if let Err(e) = error {
        brelse(t);
        return Err(e);
    }
    Ok((t, blkoff(fs, offset) as usize))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `ext2fs_bufatoff` over the extents image: a file mapped by its extent
    // tree (the disk block of the extent, read from the device) and a directory mapped by its
    // block pointers (read through the directory's vnode), each with the offset in the block.

    use std::assert_eq;

    use hal::sys::mount::MNT_RDONLY;

    use super::super::ext2fs::Ext2Superblock;
    use super::super::testext2fs::newfs::{self, B, ROOTDIR};
    use super::super::testext2fs::{attach, mount, vget};
    use super::*;
    use crate::fs::subr::vput;
    use crate::fs::ufs::ufs::inode::vtoi;

    /// The disk address of file system block `b`.
    fn blkno(fs: &Ext2Superblock, b: usize) -> Daddr {
        fsbtodb(fs, b as Daddr)
    }

    #[test]
    fn bufatoff_reads_through_the_extent_tree_or_the_block_pointers() {
        let (_g, _p) = attach(newfs::image(true));
        let mp = mount(MNT_RDONLY);
        let vp = vget(mp, 14).unwrap();
        let ip = vtoi(vp);
        let fs = ip.i_e2fs();
        let want = newfs::pattern(newfs::EXT_SIZE, 14);
        for (off, blk) in [(0, 27), (B + 3, 28), (2 * B + 5, 30)] {
            let (t, res) = ext2fs_bufatoff(&ip, off as Off).unwrap();
            assert_eq!(t.buf().blkno(), blkno(fs, blk), "offset {off}");
            assert_eq!(res, off % B);
            let lbn = off / B;
            let n = (want.len() - lbn * B).min(B);
            assert_eq!(t.bytes()[..n], want[lbn * B..lbn * B + n]);
            brelse(t);
        }
        drop(ip);
        vput(vp);

        // The root directory has no extent tree: its block pointer, through its vnode.
        let vp = vget(mp, 2).unwrap();
        let ip = vtoi(vp);
        let (t, res) = ext2fs_bufatoff(&ip, 24).unwrap();
        assert_eq!(t.buf().blkno(), blkno(ip.i_e2fs(), ROOTDIR));
        assert_eq!(res, 24);
        // The third entry, "hello.txt" (inode 12), starts there.
        assert_eq!(t.bytes()[24..28], 12u32.to_le_bytes());
        assert_eq!(t.bytes()[32..41], *b"hello.txt");
        brelse(t);
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
