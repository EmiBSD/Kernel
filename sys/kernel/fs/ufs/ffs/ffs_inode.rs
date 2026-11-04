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
 *	@(#)ffs_inode.c	8.8 (Berkeley) 10/19/94
 */
/* </LICENSES> */

/* <CODE> */
//! Writing an inode back to its block (`ffs_update`) and truncating a file (`ffs_truncate`,
//! with `ffs_indirtrunc` freeing the blocks under an indirect block).
//!
//! Upstream: sys/ufs/ffs/ffs_inode.c @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_inode.rs@3c62ede68802
//!
//! ## Deviations
//! - The dinode copies between the inode and its block go through `inode.rs`'s
//!   `set_dinode1_at` (FFS1) and `set_dinode2_at` (FFS2); the block pointers in an indirect
//!   block are read and written at their index in the bytes (`BAP`/`BAP_ASSIGN`).
//!   `ffs_indirtrunc`'s copy of the block is an array of `malloc(M_TEMP)`, as in C.
//! - `curproc->p_ru.ru_inblock++` is skipped when no thread runs, as the buffer cache's
//!   own reads do (`VnodeOps::charge_io`).
//! - The quota calls are `quota.rs`'s: `ufs_quota.rs`'s with feature `quota` (`option QUOTA`),
//!   the no-quota answers of `ufs_quota_stub.c` without it.
//!
//! ## Redesign
//! - N5.5b1: LZ's buffers lent by address and their data reached through `unsafe` (four
//!   sites), and `ffs_indirtrunc`'s `malloc`ed copy as a raw pointer cut into a slice (two
//!   `unsafe`) -> the kernel cache's tokens and an owned `MArray<u8>` of `M_TEMP` (freed by
//!   its drop where the C frees it); the read of an indirect block whose disk address is
//!   known is `ufs_bmaparray`'s (`getblk`, `B_READ`, `vop_strategy`, `biowait` on the
//!   buffer's counted link). Over the in-core `Superblock`. `ffs_update` updates the times
//!   with the inode the caller holds (the C's `ufs_itimes(vp)` takes it again through
//!   `VTOI`, a count pair here).

use hal::malloc::{M_WAITOK, MArray};
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Off};

use super::ffs_alloc::ffs_blkfree;
use super::fs::{
    FS_44INODEFMT, FS_UFS1_MAGIC, blkoff, blksize, fsbtodb, ino_to_fsba, ino_to_fsbo, lblkno,
    nindir, numfrags,
};
use crate::assert::kpanic;
use crate::fs::bio::{BCSTATS, bawrite, bdwrite, biowait, bread, brelse, btodb, buf_adjcnt};
use crate::fs::bio::{bwrite, getblk};
use crate::fs::buf::{B_CLRBUF, B_DELWRI, B_DONE, B_INVAL, B_READ, B_SYNC};
use crate::fs::getcwd::M_TEMP;
use crate::fs::subr::vinvalbuf;
use crate::fs::ufs::ufs::dinode::{NDADDR, NIADDR, Ufs1Dinode};
#[cfg(feature = "ffs2")]
use crate::fs::ufs::ufs::inode::set_dinode2_at;
use crate::fs::ufs::ufs::inode::{
    IN_CHANGE, IN_LAZYMOD, IN_MODIFIED, IN_UPDATE, Inode, daddr32_at, doingasync, set_daddr32_at,
    set_dinode1_at,
};
#[cfg(feature = "ffs2")]
use crate::fs::ufs::ufs::inode::{daddr64_at, set_daddr64_at};
use crate::fs::ufs::ufs::quota::{getinoquota, ufs_quota_free_blocks};
use crate::fs::ufs::ufs::ufs_lookup::buf_alloc;
use crate::fs::ufs::ufs::ufs_vnops::itimes;
#[cfg(feature = "ffs2")]
use crate::fs::ufs::ufs::ufsmount::UM_UFS2;
use crate::fs::vnode::{IO_SYNC, V_SAVE, V_SAVEMETA, VDIR, VLNK, VREG};
use crate::fs::vops::{Cred, IoBuf, vop_strategy};
use crate::proc::ProcGlue;
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// `ffs_update` (`iv_update`): write the inode back to its block.
///
/// Update the access, modified, and inode change times as specified by the `IN_ACCESS`,
/// `IN_UPDATE`, and `IN_CHANGE` flags respectively. The `IN_MODIFIED` flag is used to specify
/// that the inode needs to be updated but that the times have already been set. The
/// `IN_LAZYMOD` flag is used to specify that the inode needs to be updated at some point, by
/// reclaim if not in the course of other changes; this is used to defer writes just to
/// update device timestamps. If waitfor is set, then wait for the disk write of the inode
/// to complete.
pub fn ffs_update<G: ProcGlue>(ip: &Inode<G>, waitfor: i32) -> Result<(), Errno> {
    let vp = ip.itov();
    itimes(vp, ip);

    if ip.i_flag.get() & IN_MODIFIED == 0 && waitfor == 0 {
        return Ok(());
    }

    ip.clr_flag(IN_MODIFIED | IN_LAZYMOD);
    let fs = ip.i_fs();

    // Ensure that uid and gid are correct. This is a temporary fix until fsck has been
    // changed to do the update.
    if fs.fs_magic() == FS_UFS1_MAGIC && fs.fs_inodefmt() < FS_44INODEFMT {
        // di_ouid, di_ogid (the di_u halves) from the 32-bit ids.
        let off = core::mem::offset_of!(Ufs1Dinode, di_u);
        let din = ip.ufs1_din();
        din.set::<u16>(off, ip.dip_uid() as u16);
        din.set::<u16>(off + 2, ip.dip_gid() as u16);
    }

    let (mut t, error) = bread(
        &ip.i_devvp().bufs,
        fsbtodb(fs, ino_to_fsba(fs, ip.i_number)),
        fs.fs_bsize(),
    );
    if let Err(e) = error {
        brelse(t);
        return Err(e);
    }

    if ip.i_effnlink.get() != ip.dip_nlink() {
        kpanic(format_args!("ffs_update: bad link cnt"));
    }

    let idx = ino_to_fsbo(fs, ip.i_number);
    #[cfg(feature = "ffs2")]
    if ip.fstype() == UM_UFS2 {
        set_dinode2_at(t.data(), idx, &ip.din2());
    } else {
        set_dinode1_at(t.data(), idx, &ip.din1());
    }
    #[cfg(not(feature = "ffs2"))]
    set_dinode1_at(t.data(), idx, &ip.din1());

    if waitfor != 0 && !doingasync(vp) {
        bwrite(t)
    } else {
        bdwrite(t);
        Ok(())
    }
}

/// `SINGLE`: index of single indirect block.
const SINGLE: usize = 0;
/// `DOUBLE`: index of double indirect block.
const DOUBLE: usize = 1;
/// `TRIPLE`: index of triple indirect block.
const TRIPLE: usize = 2;

/// `ffs_truncate` (`iv_truncate`): truncate the inode `oip` to at most `length` size,
/// freeing the disk blocks.
pub fn ffs_truncate<G: ProcGlue>(
    oip: &Inode<G>,
    length: Off,
    flags: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    if length < 0 {
        return Err(Errno::EINVAL);
    }
    let ovp = oip.itov();

    let t = ovp.vtype();
    if t != VREG && t != VDIR && t != VLNK {
        return Ok(());
    }

    if oip.dip_size() as Off == length {
        return Ok(());
    }

    if t == VLNK && oip.dip_size() < u64::from(oip.ump().um_maxsymlinklen.get()) {
        #[cfg(feature = "diagnostic")]
        if length != 0 {
            kpanic(format_args!("ffs_truncate: partial truncate of symlink"));
        }
        let n = oip.dip_size() as usize;
        oip.with_shortlink(|s| s[..n].fill(0));
        oip.dip_set_size(0);
        oip.set_flag(IN_CHANGE | IN_UPDATE);
        return oip.vtbl().update(oip, 1);
    }

    getinoquota(oip)?;

    let fs = oip.i_fs();
    if length as u64 > fs.fs_maxfilesize() {
        return Err(Errno::EFBIG);
    }

    uvm_vnp_setsize(ovp, length);
    let mut ci = oip.i_ci.get();
    ci.ci_lasta = 0;
    ci.ci_clen = 0;
    ci.ci_cstart = 0;
    ci.ci_lastw = 0;
    oip.i_ci.set(ci);

    let osize = oip.dip_size() as Off;
    // Lengthen the size of the file. We must ensure that the last byte of the file is
    // allocated. Since the smallest value of osize is 0, length will be at least 1.
    if osize < length {
        let mut aflags = B_CLRBUF;
        if flags & IO_SYNC != 0 {
            aflags |= B_SYNC;
        }
        let bp = buf_alloc(oip, length - 1, 1, cred, aflags)?;
        oip.dip_set_size(length as u64);
        uvm_vnp_setsize(ovp, length);
        let _ = uvm_vnp_uncache(ovp);
        if aflags & B_SYNC != 0 {
            let _ = bwrite(bp);
        } else {
            bawrite(bp);
        }
        oip.set_flag(IN_CHANGE | IN_UPDATE);
        return oip.vtbl().update(oip, 1);
    }
    uvm_vnp_setsize(ovp, length);

    // Shorten the size of the file. If the file is not being truncated to a block boundary,
    // the contents of the partial block following the end of the file must be zero'ed in
    // case it ever becomes accessible again because of subsequent file growth. Directories
    // however are not zero'ed as they should grow back initialized to empty.
    let offset = blkoff(fs, length);
    if offset == 0 {
        oip.dip_set_size(length as u64);
    } else {
        let lbn = lblkno(fs, length);
        let mut aflags = B_CLRBUF;
        if flags & IO_SYNC != 0 {
            aflags |= B_SYNC;
        }
        let mut bp = buf_alloc(oip, length - 1, 1, cred, aflags)?;
        oip.dip_set_size(length as u64);
        let size = blksize(fs, oip, lbn) as i64;
        let _ = uvm_vnp_uncache(ovp);
        if t != VDIR {
            match bp.data().get_mut(offset as usize..size as usize) {
                Some(tail) => tail.fill(0),
                None => kpanic(format_args!("ffs_truncate: block of {size} bytes")),
            }
        }
        buf_adjcnt(&bp, size);
        if aflags & B_SYNC != 0 {
            let _ = bwrite(bp);
        } else {
            bawrite(bp);
        }
    }
    // Calculate index into inode's block list of last direct and indirect blocks (if any)
    // which we want to keep. Lastblock is -1 when the file is truncated to 0.
    let lastblock = lblkno(fs, length + Off::from(fs.fs_bsize()) - 1) - 1;
    let mut lastiblock = [0 as Daddr; NIADDR];
    lastiblock[SINGLE] = lastblock - NDADDR as Daddr;
    lastiblock[DOUBLE] = lastiblock[SINGLE] - nindir(fs);
    lastiblock[TRIPLE] = lastiblock[DOUBLE] - nindir(fs) * nindir(fs);
    let nblocks = btodb(i64::from(fs.fs_bsize()));

    // Update file and block pointers on disk before we start freeing blocks. If we crash
    // before free'ing blocks below, the blocks will be returned to the free list. lastiblock
    // values are also normalized to -1 for calls to ffs_indirtrunc below.
    let mut oldblks = [0 as Daddr; NDADDR + NIADDR];
    let mut newblks = [0 as Daddr; NDADDR + NIADDR];
    for level in (SINGLE..=TRIPLE).rev() {
        oldblks[NDADDR + level] = oip.dip_ib(level);
        if lastiblock[level] < 0 {
            oip.dip_set_ib(level, 0);
            lastiblock[level] = -1;
        }
    }

    for (i, old) in oldblks.iter_mut().enumerate().take(NDADDR) {
        *old = oip.dip_db(i);
        if i as Daddr > lastblock {
            oip.dip_set_db(i, 0);
        }
    }

    oip.set_flag(IN_CHANGE | IN_UPDATE);
    // The C keeps this error in allerror, which vinvalbuf's result replaces below.
    let _ = oip.vtbl().update(oip, 1);

    // Having written the new inode to disk, save its new configuration and put back the old
    // block pointers long enough to process them. Note that we save the new block
    // configuration so we can check it when we are done.
    for i in 0..NDADDR {
        newblks[i] = oip.dip_db(i);
        oip.dip_set_db(i, oldblks[i]);
    }

    for i in 0..NIADDR {
        newblks[NDADDR + i] = oip.dip_ib(i);
        oip.dip_set_ib(i, oldblks[NDADDR + i]);
    }

    oip.dip_set_size(osize as u64);
    let vflags = (if length > 0 { V_SAVE } else { 0 }) | V_SAVEMETA;
    let mut allerror = G::with_curproc(|p| vinvalbuf(ovp, vflags, cred, p, 0, INFSLP));

    let mut blocksreleased: i64 = 0;
    'done: {
        // Indirect blocks first.
        let mut indir_lbn = [0 as Daddr; NIADDR];
        indir_lbn[SINGLE] = -(NDADDR as Daddr);
        indir_lbn[DOUBLE] = indir_lbn[SINGLE] - nindir(fs) - 1;
        indir_lbn[TRIPLE] = indir_lbn[DOUBLE] - nindir(fs) * nindir(fs) - 1;
        for level in (SINGLE..=TRIPLE).rev() {
            let bn = oip.dip_ib(level);
            if bn != 0 {
                let mut count = 0;
                if let Err(e) = ffs_indirtrunc(
                    oip,
                    indir_lbn[level],
                    fsbtodb(fs, bn),
                    lastiblock[level],
                    level,
                    &mut count,
                ) {
                    allerror = Err(e);
                }
                blocksreleased += count;
                if lastiblock[level] < 0 {
                    oip.dip_set_ib(level, 0);
                    ffs_blkfree(oip, bn, i64::from(fs.fs_bsize()));
                    blocksreleased += nblocks;
                }
            }
            if lastiblock[level] >= 0 {
                break 'done;
            }
        }

        // All whole direct blocks or frags.
        let mut i = NDADDR as Daddr - 1;
        while i > lastblock {
            let bn = oip.dip_db(i as usize);
            if bn != 0 {
                oip.dip_set_db(i as usize, 0);
                let bsize = blksize(fs, oip, i) as i64;
                ffs_blkfree(oip, bn, bsize);
                blocksreleased += btodb(bsize);
            }
            i -= 1;
        }
        if lastblock < 0 {
            break 'done;
        }

        // Finally, look for a change in size of the last direct block; release any frags.
        let mut bn = oip.dip_db(lastblock as usize);
        if bn != 0 {
            // Calculate amount of space we're giving back as old block size minus new block
            // size.
            let oldspace = blksize(fs, oip, lastblock) as i64;
            oip.dip_set_size(length as u64);
            let newspace = blksize(fs, oip, lastblock) as i64;
            if newspace == 0 {
                kpanic(format_args!("ffs_truncate: newspace"));
            }
            if oldspace - newspace > 0 {
                // Block number of space to be free'd is the old block # plus the number of
                // frags required for the storage we're keeping.
                bn += numfrags(fs, newspace);
                ffs_blkfree(oip, bn, oldspace - newspace);
                blocksreleased += btodb(oldspace - newspace);
            }
        }
    }
    // done:
    #[cfg(feature = "diagnostic")]
    {
        for level in SINGLE..=TRIPLE {
            if newblks[NDADDR + level] != oip.dip_ib(level) {
                kpanic(format_args!("ffs_truncate1"));
            }
        }
        for (i, &blk) in newblks[..NDADDR].iter().enumerate() {
            if blk != oip.dip_db(i) {
                kpanic(format_args!("ffs_truncate2"));
            }
        }
    }
    #[cfg(not(feature = "diagnostic"))]
    let _ = newblks;
    // Put back the real size.
    oip.dip_set_size(length as u64);
    if oip.dip_blocks() >= blocksreleased {
        oip.dip_set_blocks(oip.dip_blocks() - blocksreleased);
    } else {
        // sanity
        oip.dip_set_blocks(0);
    }
    oip.set_flag(IN_CHANGE);
    let _ = ufs_quota_free_blocks(oip, blocksreleased, Cred::NOCRED);
    allerror
}

/// `BAP(ip, i)`: block pointer `i` of an indirect block's bytes.
fn bap<G: ProcGlue>(ip: &Inode<G>, b: &[u8], i: usize) -> Daddr {
    #[cfg(feature = "ffs2")]
    if ip.fstype() == UM_UFS2 {
        return daddr64_at(b, i);
    }
    let _ = ip;
    daddr32_at(b, i)
}

/// `BAP_ASSIGN(ip, i, value)`.
fn bap_assign<G: ProcGlue>(ip: &Inode<G>, b: &mut [u8], i: usize, value: Daddr) {
    #[cfg(feature = "ffs2")]
    if ip.fstype() == UM_UFS2 {
        return set_daddr64_at(b, i, value);
    }
    let _ = ip;
    set_daddr32_at(b, i, value);
}

/// `ffs_indirtrunc`: release blocks associated with the inode `ip` and stored in the
/// indirect block `bn`. Blocks are free'd in LIFO order up to (but not including) `lastbn`.
/// If level is greater than `SINGLE`, the block is an indirect block and recursive calls to
/// indirtrunc must be used to cleanse other indirect blocks. `*countp` is the number of
/// `DEV_BSIZE` blocks released.
///
/// NB: triple indirect blocks are untested.
pub fn ffs_indirtrunc<G: ProcGlue>(
    ip: &Inode<G>,
    lbn: Daddr,
    dbn: Daddr,
    lastbn: Daddr,
    level: usize,
    countp: &mut i64,
) -> Result<(), Errno> {
    let fs = ip.i_fs();
    let bsize = fs.fs_bsize();
    let mut blocksreleased: i64 = 0;
    let mut allerror = Ok(());

    // Calculate index in current block of last block to be kept. -1 indicates the entire
    // block so we need not calculate the index.
    let mut factor: i64 = 1;
    for _ in SINGLE..level {
        factor *= nindir(fs);
    }
    let mut last = lastbn;
    if lastbn > 0 {
        last /= factor;
    }
    let nblocks = btodb(i64::from(bsize));
    // Get buffer of block pointers, zero those entries corresponding to blocks to be
    // free'd, and update on disk copy first. Since double(triple) indirect before
    // single(double) indirect, calls to bmap on these blocks will fail. However, we already
    // have the on disk address, so we have to set the b_blkno field explicitly instead of
    // letting bread do everything for us.
    let vp = ip.itov();
    let mut t = loop {
        if let Some(t) = getblk(&vp.bufs, lbn, bsize, 0, INFSLP) {
            break t;
        }
    };
    if !t.buf().isset(B_DONE | B_DELWRI) {
        vp.bufs.owner().charge_io(false); // pay for read
        BCSTATS
            .pendingreads
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        BCSTATS
            .numreads
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        t.buf().set(B_READ);
        if t.buf().bcount() > t.buf().bufsize() {
            kpanic(format_args!("ffs_indirtrunc: bad buffer size"));
        }
        t.buf().set_blkno(dbn);
        let Some(r) = t.counted() else {
            kpanic(format_args!("ffs_indirtrunc: a driver's buffer"));
        };
        let _ = vop_strategy(vp, IoBuf::Token(t));
        let (back, error) = biowait(r);
        t = back;
        if let Err(e) = error {
            brelse(t);
            *countp = 0;
            return Err(e);
        }
    }

    // The block pointers: the buffer's, or a copy of them once the buffer is written.
    let mut copy: Option<MArray<u8>> = None;
    let mut held = Some(t);
    if lastbn != -1 {
        let Some(mut t) = held.take() else {
            kpanic(format_args!("ffs_indirtrunc: no buffer"));
        };
        let data = t.data();
        let Some(c) = MArray::new(M_TEMP, M_WAITOK, bsize as usize, |i| {
            data.get(i).copied().unwrap_or(0)
        }) else {
            kpanic(format_args!("ffs_indirtrunc: no memory"));
        };
        for i in (last + 1)..nindir(fs) {
            bap_assign(ip, data, i as usize, 0);
        }
        copy = Some(c);

        if doingasync(vp) {
            bawrite(t);
        } else if let Err(e) = bwrite(t) {
            allerror = Err(e);
        }
    }
    let baps: &[u8] = match (&copy, &mut held) {
        (Some(c), _) => c,
        (None, Some(t)) => t.data(),
        (None, None) => kpanic(format_args!("ffs_indirtrunc: no block pointers")),
    };

    // Recursively free totally unused blocks.
    let mut i = nindir(fs) - 1;
    let mut nlbn = lbn + 1 - i * factor;
    while i > last {
        let nb = bap(ip, baps, i as usize);
        if nb != 0 {
            if level > SINGLE {
                let mut blkcount = 0;
                if let Err(e) =
                    ffs_indirtrunc(ip, nlbn, fsbtodb(fs, nb), -1, level - 1, &mut blkcount)
                {
                    allerror = Err(e);
                }
                blocksreleased += blkcount;
            }
            ffs_blkfree(ip, nb, i64::from(bsize));
            blocksreleased += nblocks;
        }
        i -= 1;
        nlbn += factor;
    }

    // Recursively free last partial block.
    if level > SINGLE && lastbn >= 0 {
        let last = lastbn % factor;
        let nb = bap(ip, baps, i as usize);
        if nb != 0 {
            let mut blkcount = 0;
            if let Err(e) =
                ffs_indirtrunc(ip, nlbn, fsbtodb(fs, nb), last, level - 1, &mut blkcount)
            {
                allerror = Err(e);
            }
            blocksreleased += blkcount;
        }
    }
    // free(copy, M_TEMP, fs->fs_bsize): the copy's drop; or the buffer released invalid.
    drop(copy);
    if let Some(t) = held {
        t.buf().set(B_INVAL);
        brelse(t);
    }

    *countp = blocksreleased;
    allerror
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::{assert, assert_eq};

    use super::super::ffs_balloc::ffs_balloc;
    use super::super::fs::{Superblock, fragroundup};
    use super::super::testffs::newfs::{FFS1_1G, FFS2_1G, Image, Params, check_with, super_block};
    use super::super::testffs::{disk, new_file, pattern, setup, sync};
    use super::*;
    use crate::fs::subr::vput;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ufs::dinode::{IFLNK, IFREG};
    use crate::fs::ufs::ufs::inode::{IN_ACCESS, Indir, dinode1_at, dinode2_at, vtoi};
    use crate::fs::ufs::ufs::ufs_bmap::getlbns;
    use crate::fs::vnode::Vnode;

    /// The formats the tests build: FFS2's only with `option FFS2`.
    const FORMATS: &[Params] = if cfg!(feature = "ffs2") {
        &[FFS2_1G, FFS1_1G]
    } else {
        &[FFS1_1G]
    };

    /// `fsck`'s check of the synced image against the in-core summaries and totals.
    fn check_incore(fs: &Superblock, ufs2: bool, vps: &[&'static Vnode<TestGlue>]) {
        sync(vps);
        let d = disk();
        let geom = super_block(&d, ufs2);
        check_with(
            &d,
            &geom,
            |cg| {
                let c = fs.fs_cs(cg);
                [c.cs_ndir(), c.cs_nbfree(), c.cs_nifree(), c.cs_nffree()]
            },
            [
                fs.fs_cstotal_ndir(),
                fs.fs_cstotal_nbfree(),
                fs.fs_cstotal_nifree(),
                fs.fs_cstotal_nffree(),
            ],
        );
    }

    /// A synchronous write of `data` at `off` (`ffs_write`'s steps: the block, the bytes,
    /// the size), every metadata write synchronous.
    fn write(ip: &Inode<TestGlue>, off: Off, data: &[u8], cred: Cred<'_>) {
        let fs = ip.i_fs();
        let on = blkoff(fs, off) as usize;
        let mut t = ffs_balloc(ip, off, data.len() as i32, cred, B_CLRBUF | B_SYNC).unwrap();
        t.data()[on..on + data.len()].copy_from_slice(data);
        let end = (off + data.len() as Off) as u64;
        if end > ip.dip_size() {
            ip.dip_set_size(end);
        }
        bwrite(t).unwrap();
    }

    /// The dinode of `ip` as its block on the disk holds it: (size, mode, nlink, blocks).
    fn on_disk(ip: &Inode<TestGlue>) -> (u64, u16, i16, i64) {
        let fs = ip.i_fs();
        let d = disk();
        let blk = &d[fsbtodb(fs, ino_to_fsba(fs, ip.i_number)) as usize * 512..];
        let i = ino_to_fsbo(fs, ip.i_number);
        if ip.is_ufs2() {
            let x = dinode2_at(blk, i);
            (x.di_size, x.di_mode, x.di_nlink, x.di_blocks as i64)
        } else {
            let x = dinode1_at(blk, i);
            (x.di_size, x.di_mode, x.di_nlink, i64::from(x.di_blocks))
        }
    }

    #[test]
    fn update_writes_the_dinode_into_its_block() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            ip.dip_set_size(1234);
            ip.set_flag(IN_MODIFIED);
            ffs_update(&ip, 1).unwrap();
            assert_eq!(ip.i_flag.get() & (IN_MODIFIED | IN_LAZYMOD), 0);
            assert_eq!(on_disk(&ip), (1234, (IFREG | 0o644) as u16, 1, 0));
            // Nothing to write and no wait: the block is left alone.
            ip.dip_set_size(99);
            ffs_update(&ip, 0).unwrap();
            assert_eq!(on_disk(&ip).0, 1234);
            // An access time to set: written with the next change.
            ip.set_flag(IN_ACCESS);
            ffs_update(&ip, 1).unwrap();
            assert_eq!(on_disk(&ip).0, 99);
            assert_eq!(ip.i_flag.get() & IN_ACCESS, 0);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn truncate_to_zero_frees_the_direct_and_indirect_blocks() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let cred = Cred::real(th.ucred());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            let bsize = fs.fs_bsize() as usize;
            let (nb, nf) = (fs.fs_cstotal_nbfree(), fs.fs_cstotal_nffree());
            write(&ip, 0, &pattern(bsize, 1), cred);
            write(&ip, bsize as Off, &pattern(100, 1), cred);
            write(&ip, (bsize * NDADDR) as Off, &pattern(bsize, 2), cred);
            write(&ip, (bsize * (NDADDR + 1)) as Off, &pattern(10, 3), cred);
            assert!(ip.dip_ib(0) != 0);
            sync(&[vp]);
            ffs_truncate(&ip, 0, IO_SYNC, cred).unwrap();
            assert_eq!(ip.dip_size(), 0);
            assert_eq!(ip.dip_blocks(), 0);
            assert_eq!(ip.dip_ib(0), 0);
            assert!((0..NDADDR).all(|i| ip.dip_db(i) == 0));
            assert_eq!((fs.fs_cstotal_nbfree(), fs.fs_cstotal_nffree()), (nb, nf));
            assert_eq!(on_disk(&ip).0, 0, "the new size written first");
            check_incore(fs, p.ufs2, &[vp]);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn truncate_inside_the_indirect_range_keeps_its_first_pointers() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let cred = Cred::real(th.ucred());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            let bsize = fs.fs_bsize() as usize;
            for lbn in 0..NDADDR + 3 {
                write(&ip, (bsize * lbn) as Off, &pattern(bsize, lbn as u8), cred);
            }
            let blocks = ip.dip_blocks();
            sync(&[vp]);
            // Keep NDADDR + 1 blocks: the indirect block's first pointer stays.
            ffs_truncate(&ip, (bsize * (NDADDR + 1)) as Off, IO_SYNC, cred).unwrap();
            assert_eq!(ip.dip_blocks(), blocks - 2 * btodb(bsize as i64));
            let ib = ip.dip_ib(0);
            assert!(ib != 0);
            sync(&[vp]);
            let d = disk();
            let at = fsbtodb(fs, ib) as usize * 512;
            let ptr = |i: usize| {
                if p.ufs2 {
                    crate::fs::ufs::ufs::inode::daddr64_at(&d[at..], i)
                } else {
                    daddr32_at(&d[at..], i)
                }
            };
            assert!(ptr(0) != 0);
            assert_eq!((ptr(1), ptr(2)), (0, 0), "zeroed on the disk first");
            // The indirect path is the C's: getlbns finds the kept block.
            let mut indirs = [Indir::default(); NIADDR + 2];
            let mut num = 0;
            getlbns(
                ip.nindir() as i64,
                NDADDR as Daddr,
                &mut indirs,
                Some(&mut num),
            )
            .unwrap();
            assert_eq!(indirs[1].in_off, 0);
            check_incore(fs, p.ufs2, &[vp]);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn truncate_inside_a_block_zeroes_its_tail_and_frees_its_fragments() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let cred = Cred::real(th.ucred());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            let (bsize, fsize) = (fs.fs_bsize() as usize, fs.fs_fsize() as usize);
            write(&ip, 0, &pattern(bsize, 7), cred);
            let db0 = ip.dip_db(0);
            let nf = fs.fs_cstotal_nffree();
            sync(&[vp]);
            ffs_truncate(&ip, 100, IO_SYNC, cred).unwrap();
            assert_eq!(ip.dip_size(), 100);
            assert_eq!(ip.dip_db(0), db0);
            // One fragment kept, the rest of the block's free.
            assert_eq!(ip.dip_blocks(), btodb(fsize as i64));
            let frags = (bsize / fsize) as i64;
            assert_eq!(fs.fs_cstotal_nffree(), nf + frags - 1);
            sync(&[vp]);
            let d = disk();
            let at = fsbtodb(fs, db0) as usize * 512;
            assert_eq!(d[at..at + 100], *pattern(100, 7));
            assert!(
                d[at + 100..at + fsize].iter().all(|&b| b == 0),
                "the tail zeroed"
            );
            check_incore(fs, p.ufs2, &[vp]);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn lengthening_allocates_the_last_byte() {
        for &p in FORMATS {
            let (_g, th, mp) = setup(Image::new(p).finish());
            let cred = Cred::real(th.ucred());
            let vp = new_file(mp, th);
            let ip = vtoi(vp);
            let fs = ip.i_fs();
            ffs_truncate(&ip, 5000, IO_SYNC, cred).unwrap();
            assert_eq!(ip.dip_size(), 5000);
            // The block of the last byte, and only its fragments: 5,000 bytes are within
            // FFS2's first block (8 KB) but past FFS1's (4 KB), which stays a hole.
            let lbn = lblkno(fs, 4999);
            assert!(ip.dip_db(lbn as usize) != 0);
            assert_eq!(ip.dip_db(0) == 0, lbn > 0);
            let last = blkoff(fs, 4999) + 1;
            assert_eq!(ip.dip_blocks(), btodb(fragroundup(fs, last)));
            assert_eq!(on_disk(&ip).0, 5000);
            drop(ip);
            vput(vp);
        }
    }

    #[test]
    fn a_short_symlink_is_cleared_in_the_inode() {
        let (_g, th, mp) = setup(Image::new(FORMATS[0]).finish());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        ip.dip_set_mode(IFLNK | 0o755);
        vp.set_vtype(crate::fs::vnode::VLNK);
        ip.with_shortlink(|s| s[..3].copy_from_slice(b"abc"));
        ip.dip_set_size(3);
        ffs_truncate(&ip, 0, 0, Cred::real(th.ucred())).unwrap();
        assert_eq!(ip.dip_size(), 0);
        assert_eq!(ip.with_shortlink(|s| [s[0], s[1], s[2]]), [0, 0, 0]);
        assert_eq!(ip.dip_blocks(), 0);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn truncate_checks_the_length_and_the_type_first() {
        let (_g, th, mp) = setup(Image::new(FORMATS[0]).finish());
        let cred = Cred::real(th.ucred());
        let vp = new_file(mp, th);
        let ip = vtoi(vp);
        assert_eq!(ffs_truncate(&ip, -1, 0, cred), Err(Errno::EINVAL));
        let max = ip.i_fs().fs_maxfilesize();
        assert_eq!(
            ffs_truncate(&ip, (max + 1) as Off, 0, cred),
            Err(Errno::EFBIG)
        );
        // Not a regular file, directory or link: nothing to do.
        vp.set_vtype(crate::fs::vnode::VCHR);
        assert_eq!(ffs_truncate(&ip, 5000, 0, cred), Ok(()));
        assert_eq!(ip.dip_size(), 0);
        vp.set_vtype(crate::fs::vnode::VREG);
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
