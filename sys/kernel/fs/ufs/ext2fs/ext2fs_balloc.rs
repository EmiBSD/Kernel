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
 *	@(#)ffs_balloc.c	8.4 (Berkeley) 9/23/93
 * Modified for ext2fs by Manuel Bouyer.
 */
/* </LICENSES> */

/* <CODE> */
//! ext2fs block allocation for a file: `ext2fs_buf_alloc` defines the structure of file
//! system storage by allocating the physical blocks on a device given the inode and the
//! logical block number in a file, indirect blocks included.
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_balloc.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_balloc.rs@3c62ede68802
//!
//! ## Deviations
//! - The buffer comes back as the `Ok` value (the C's `*bpp`).
//! - The block pointers of an indirect block are read and written at their index in the
//!   buffer's bytes (`bap[i]`), little-endian (`letoh32`/`htole32`).
//! - On failure the C marks the change in `i_e2fs_flags` (the on-disk `e2di_flags`, where
//!   `IN_CHANGE | IN_UPDATE` are `EXT2_UNRM | EXT2_COMPR`); here it goes to `i_flag`, where
//!   every other caller puts it (LZ's deviation, kept).
//!
//! ## Redesign
//! - LZ's lent buffers (`&'static Buf`, their bytes reached through an `unsafe`
//!   `BufLegacy::data` at five sites) -> the busy buffers' tokens of the kernel's cache
//!   (`bread`/`getblk` give a `BufRef`, `bwrite`/`bdwrite`/`brelse` consume it): each
//!   indirect block's pointers are read and written in its token's bytes and the data block
//!   is the result, as the kernel's `ffs_balloc` (N5.5b1). The levels of indirection come from
//!   `getlbns` over the inode's `MNINDIR` (the C's `ufs_getlbns(vp, ..)` reaches the same
//!   inode through `VTOI`, a count pair here).

use hal::sys::errno::Errno;
use hal::sys::types::Daddr;

use super::ext2fs::fsbtodb;
use super::ext2fs_alloc::{ext2fs_alloc, ext2fs_blkfree, ext2fs_blkpref};
use super::ext2fs_dinode::{NDADDR, NIADDR};
use crate::assert::kpanic;
use crate::fs::bio::getblk_wait;
use crate::fs::bio::{bdwrite, bread, brelse, btodb, bwrite};
use crate::fs::buf::{B_CLRBUF, B_INVAL, B_SYNC, BufRef};
use crate::fs::ufs::ufs::inode::{IN_CHANGE, IN_UPDATE, Indir, Inode, le32_at};
use crate::fs::ufs::ufs::ufs_bmap::getlbns;
use crate::fs::vops::Cred;
use crate::proc::ProcGlue;

/// `bwrite` with `B_SYNC` in `flags`, else `bdwrite`: "if required, write synchronously,
/// otherwise use delayed write".
fn write_or_delay(t: BufRef, flags: i32) {
    if flags & B_SYNC != 0 {
        let _ = bwrite(t);
    } else {
        bdwrite(t);
    }
}

/// `bap[i] = htole32(v)`.
fn bap_set(b: &mut [u8], i: usize, v: u32) {
    b[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
}

/// `ext2fs_buf_alloc`: the buffer of logical block `bn` of `ip`, allocating it and the
/// indirect blocks on its path as needed. `flags` holds `B_CLRBUF` (clear a new buffer)
/// and `B_SYNC` (write the indirect blocks synchronously).
pub fn ext2fs_buf_alloc<G: ProcGlue>(
    ip: &Inode<G>,
    bn: u32,
    _size: i32,
    cred: Cred<'_>,
    flags: i32,
) -> Result<BufRef, Errno> {
    let fs = ip.i_e2fs();
    let d = ip.i_e2din();
    let vp = ip.itov();
    let bsize = fs.e2fs_bsize.get();
    let lbn = Daddr::from(bn);

    // The first NDADDR blocks are direct blocks
    if (bn as usize) < NDADDR {
        let nb = u32::from_le(d.i_e2fs_block(bn as usize));
        if nb != 0 {
            let (t, error) = bread(&vp.bufs, lbn, bsize);
            if let Err(e) = error {
                brelse(t);
                return Err(e);
            }
            return Ok(t);
        }

        // allocate a new direct block.
        let mut bap = [0u8; (NDADDR + NIADDR) * 4];
        for (i, v) in d.i_e2fs_blocks().iter().enumerate() {
            bap[4 * i..4 * i + 4].copy_from_slice(&v.to_ne_bytes());
        }
        let pref = ext2fs_blkpref(ip, bn, bn as i32, Some(&bap));
        let newb = ext2fs_alloc(ip, bn, pref as u32, cred)?;
        d.i_e2fs_last_lblk.set(bn);
        d.i_e2fs_last_blk.set(newb);
        d.set_i_e2fs_block(bn as usize, newb.to_le());
        ip.set_flag(IN_CHANGE | IN_UPDATE);
        let mut t = getblk_wait(&vp.bufs, lbn, bsize);
        t.buf().set_blkno(fsbtodb(fs, Daddr::from(newb)));
        if flags & B_CLRBUF != 0 {
            t.clear();
        }
        return Ok(t);
    }
    // Determine the number of levels of indirection.
    let mut indirs = [Indir::default(); NIADDR + 2];
    let mut num = 0;
    getlbns(ip.nindir() as i64, lbn, &mut indirs, Some(&mut num))?;
    #[cfg(feature = "diagnostic")]
    if num < 1 {
        kpanic(format_args!(
            "ext2fs_balloc: ufs_getlbns returned indirect block"
        ));
    }
    let num = num as usize - 1;

    // The blocks allocated so far (allociblk) and the inode's slot of the first indirect
    // block, if it was allocated (allocib).
    let mut allociblk = [0u32; NIADDR + 1];
    let mut nalloc = 0;
    let mut allocib: Option<usize> = None;
    let mut unwindidx: isize = -1;

    let error: Errno = 'fail: {
        // Fetch the first indirect block allocating if necessary.
        let mut nb = u32::from_le(d.i_e2fs_block(NDADDR + indirs[0].in_off as usize));
        if nb == 0 {
            let pref = ext2fs_blkpref(ip, bn, 0, None);
            let newb = ext2fs_alloc(ip, bn, pref as u32, cred)?;
            nb = newb;
            allociblk[nalloc] = nb;
            nalloc += 1;
            d.i_e2fs_last_blk.set(newb);
            let mut t = getblk_wait(&vp.bufs, indirs[1].in_lbn, bsize);
            t.buf().set_blkno(fsbtodb(fs, Daddr::from(newb)));
            t.clear();
            // Write synchronously so that indirect blocks never point at garbage.
            if let Err(e) = bwrite(t) {
                break 'fail e;
            }
            unwindidx = 0;
            let slot = NDADDR + indirs[0].in_off as usize;
            allocib = Some(slot);
            d.set_i_e2fs_block(slot, newb.to_le());
            ip.set_flag(IN_CHANGE | IN_UPDATE);
        }

        // Fetch through the indirect blocks, allocating as necessary.
        let mut i = 1;
        let mut t = loop {
            let (mut t, error) = bread(&vp.bufs, indirs[i].in_lbn, bsize);
            if let Err(e) = error {
                brelse(t);
                break 'fail e;
            }
            nb = le32_at(t.bytes(), indirs[i].in_off as usize);
            if i == num {
                break t;
            }
            i += 1;
            if nb != 0 {
                brelse(t);
                continue;
            }
            let pref = ext2fs_blkpref(ip, bn, 0, None);
            let newb = match ext2fs_alloc(ip, bn, pref as u32, cred) {
                Ok(b) => b,
                Err(e) => {
                    brelse(t);
                    break 'fail e;
                }
            };
            nb = newb;
            allociblk[nalloc] = nb;
            nalloc += 1;
            d.i_e2fs_last_blk.set(newb);
            let mut nbp = getblk_wait(&vp.bufs, indirs[i].in_lbn, bsize);
            nbp.buf().set_blkno(fsbtodb(fs, Daddr::from(nb)));
            nbp.clear();
            // Write synchronously so that indirect blocks never point at garbage.
            if let Err(e) = bwrite(nbp) {
                brelse(t);
                break 'fail e;
            }
            if unwindidx < 0 {
                unwindidx = i as isize - 1;
            }
            bap_set(t.data(), indirs[i - 1].in_off as usize, nb);
            write_or_delay(t, flags);
        };

        // Get the data block, allocating if necessary.
        if nb == 0 {
            let pref = ext2fs_blkpref(ip, bn, indirs[num].in_off, Some(t.bytes()));
            let newb = match ext2fs_alloc(ip, bn, pref as u32, cred) {
                Ok(b) => b,
                Err(e) => {
                    brelse(t);
                    break 'fail e;
                }
            };
            nb = newb;
            // The C also records the data block in allociblk, which nothing reads once the
            // block is in place.
            d.i_e2fs_last_lblk.set(bn);
            d.i_e2fs_last_blk.set(newb);
            bap_set(t.data(), indirs[num].in_off as usize, nb);
            write_or_delay(t, flags);
            let mut nbp = getblk_wait(&vp.bufs, lbn, bsize);
            nbp.buf().set_blkno(fsbtodb(fs, Daddr::from(nb)));
            if flags & B_CLRBUF != 0 {
                nbp.clear();
            }
            return Ok(nbp);
        }
        brelse(t);
        let nbp = if flags & B_CLRBUF != 0 {
            let (nbp, error) = bread(&vp.bufs, lbn, bsize);
            if let Err(e) = error {
                brelse(nbp);
                break 'fail e;
            }
            nbp
        } else {
            let nbp = getblk_wait(&vp.bufs, lbn, bsize);
            nbp.buf().set_blkno(fsbtodb(fs, Daddr::from(nb)));
            nbp
        };

        return Ok(nbp);
    };

    // fail:
    // If we have failed part way through block allocation, we have to deallocate any
    // indirect blocks that we have allocated.
    let mut deallocated: u32 = 0;
    for &blk in &allociblk[..nalloc] {
        ext2fs_blkfree(ip, blk);
        deallocated += bsize as u32;
    }
    if unwindidx >= 0 {
        let unwindidx = unwindidx as usize;
        if unwindidx == 0 {
            if let Some(slot) = allocib {
                d.set_i_e2fs_block(slot, 0);
            }
        } else {
            let (mut t, r) = bread(&vp.bufs, indirs[unwindidx].in_lbn, bsize);
            if let Err(r) = r {
                kpanic(format_args!(
                    "Could not unwind indirect block, error {}",
                    r as i32
                ));
            }
            bap_set(t.data(), indirs[unwindidx].in_off as usize, 0);
            write_or_delay(t, flags);
        }
        for ind in &indirs[unwindidx + 1..=num] {
            let t = getblk_wait(&vp.bufs, ind.in_lbn, bsize);
            t.buf().set(B_INVAL);
            brelse(t);
        }
    }
    if deallocated != 0 {
        let nblock = btodb(i64::from(deallocated)) as u32;
        d.set_i_e2fs_nblock(d.i_e2fs_nblock().wrapping_sub(nblock));
        ip.set_flag(IN_CHANGE | IN_UPDATE);
    }
    Err(error)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `ext2fs_buf_alloc` over the test image: a new file's direct block and
    // its first block behind the single, double and triple indirect blocks (mapped back by
    // `ext2fs_bmaparray`), an existing block read, and the failure unwind.

    use std::{assert, assert_eq};

    use super::super::ext2fs_bmap::ext2fs_bmaparray;
    use super::super::testext2fs::newfs::{self, B, NFREE};
    use super::super::testext2fs::{attach, mount, new_file, vget};
    use super::*;
    use crate::cred::Ucred;
    use crate::fs::subr::vput;
    use crate::fs::ufs::ffs::testffs;
    use crate::fs::ufs::ufs::inode::vtoi;
    use crate::fs::vnode::Vnode;

    /// `ext2fs_bmaparray`'s disk address of logical block `lbn` of `vp`.
    fn bmap(vp: &'static Vnode<crate::fs::testfs::TestGlue>, lbn: Daddr) -> Daddr {
        let mut bn = 0;
        assert_eq!(ext2fs_bmaparray(vp, lbn, &mut bn, None, None, None), Ok(()));
        bn
    }

    #[test]
    fn blocks_are_allocated_directly_and_behind_each_level_of_indirection() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = new_file(mp, p);
        let ip = vtoi(vp);
        let fs = ip.i_e2fs();
        let cred = Cred::real(p.ucred());
        let nindir = (B / 4) as u32;
        for (lbn, metas) in [
            (0u32, 0u32),
            (12, 1),
            (12 + nindir, 2),
            (12 + nindir + nindir * nindir, 3),
        ] {
            let free = fs.e2fs_fbcount();
            let mut t = ext2fs_buf_alloc(&ip, lbn, B as i32, cred, B_CLRBUF | B_SYNC)
                .unwrap_or_else(|e| std::panic!("lbn {lbn}: {e:?}"));
            assert!(
                t.bytes().iter().all(|&b| b == 0),
                "B_CLRBUF: a zeroed block"
            );
            t.data()[..4].copy_from_slice(&lbn.to_le_bytes());
            let blkno = t.buf().blkno();
            assert!(blkno > 0);
            let _ = bwrite(t);
            assert_eq!(fs.e2fs_fbcount(), free - 1 - metas, "lbn {lbn}");
            assert_eq!(bmap(vp, Daddr::from(lbn)), blkno, "lbn {lbn}");
            assert_eq!(
                ip.i_e2din().i_e2fs_last_lblk.get(),
                lbn,
                "the data block's logical number"
            );
            testffs::sync(&[vp]);
            let d = testffs::disk();
            let at = blkno as usize * 512;
            assert_eq!(d[at..at + 4], lbn.to_le_bytes(), "lbn {lbn} on the disk");
        }
        assert_eq!(fs.e2fs_fbcount(), NFREE - 10);
        assert_eq!(ip.i_e2din().i_e2fs_nblock(), 20);
        // The three indirect pointers in the inode.
        for level in 0..NIADDR {
            assert!(
                ip.i_e2din().i_e2fs_block(NDADDR + level) != 0,
                "level {level}"
            );
        }
        // An allocated block is read back, nothing allocated.
        let free = fs.e2fs_fbcount();
        let t = ext2fs_buf_alloc(&ip, 12, B as i32, cred, B_CLRBUF).unwrap();
        assert_eq!(t.bytes()[..4], 12u32.to_le_bytes());
        brelse(t);
        assert_eq!(fs.e2fs_fbcount(), free);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn a_failed_allocation_gives_back_the_indirect_blocks_it_made() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = new_file(mp, p);
        let ip = vtoi(vp);
        let fs = ip.i_e2fs();
        let mut user = Ucred::new();
        user.cr_uid.set(1000);
        // Room for one block for a user: the double indirect block, then nothing.
        fs.set_e2fs_rbcount(fs.e2fs_fbcount() - 1);
        let free = fs.e2fs_fbcount();
        let lbn = 12 + (B / 4) as u32;
        assert_eq!(
            ext2fs_buf_alloc(&ip, lbn, B as i32, Cred::real(&user), B_CLRBUF).map(|_| ()),
            Err(Errno::ENOSPC)
        );
        assert_eq!(fs.e2fs_fbcount(), free, "the indirect block is freed again");
        let d = ip.i_e2din();
        assert_eq!(
            d.i_e2fs_block(NDADDR + 1),
            0,
            "the inode's pointer is cleared"
        );
        assert_eq!(d.i_e2fs_nblock(), 0);
        assert!(ip.i_flag.get() & (IN_CHANGE | IN_UPDATE) != 0);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn an_existing_file_reads_its_blocks() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = vget(mp, 13).unwrap();
        let ip = vtoi(vp);
        let cred = Cred::real(p.ucred());
        let free = ip.i_e2fs().e2fs_fbcount();
        let want = newfs::pattern(newfs::BIG_SIZE, 13);
        for lbn in [0u32, 11, 12, 13] {
            let t = ext2fs_buf_alloc(&ip, lbn, B as i32, cred, B_CLRBUF).unwrap();
            let n = (want.len() - lbn as usize * B).min(B);
            assert_eq!(t.bytes()[..n], want[lbn as usize * B..lbn as usize * B + n]);
            brelse(t);
        }
        assert_eq!(ip.i_e2fs().e2fs_fbcount(), free);
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
