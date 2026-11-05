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
 *	@(#)ffs_inode.c	8.8 (Berkeley) 10/19/94
 * Modified for ext2fs by Manuel Bouyer.
 */

/*
 * Copyright (c) 1982, 1989, 1993
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
 *	@(#)inode.h	8.5 (Berkeley) 7/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! ext2fs inodes: the size of a file (`ext2fs_size`, `ext2fs_setsize`, with the high 32
//! bits of a regular file's size), the last reference to an inode (`ext2fs_inactive`, the
//! vnode operation), writing an inode back to its block (`ext2fs_update`, its times first:
//! `<ufs/ufs/inode.h>`'s `EXT2FS_ITIMES`) and truncating a file (`ext2fs_truncate`, with
//! `ext2fs_indirtrunc` freeing the blocks under an indirect block).
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_inode.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_inode.rs@3c62ede68802
//! Upstream: sys/ufs/ufs/inode.h @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/inode.rs@3c62ede68802
//!
//! ## Deviations
//! - The dinode goes to its block through `e2fs_isave` over the buffer's bytes at the
//!   inode's offset; the block pointers in an indirect block are read and written at their
//!   index in the bytes, little-endian (`letoh32`). `ext2fs_indirtrunc`'s copy of the block
//!   is an array of `malloc(M_TEMP)`, as in C.
//! - `curproc->p_ru.ru_inblock++` is the vnode's owner's I/O charge (`charge_io`), as the
//!   kernel's `ffs_indirtrunc`.
//! - `ext2fs_truncate`'s `allerror` starts as success: the C leaves it unset when
//!   `ext2fs_update` succeeds, and `vinvalbuf`'s result replaces it anyway.
//! - The block counts are kept in `i64` (`blocksreleased`, `count`), the C's `long`.
//! - The `DIAGNOSTIC` check that a file truncated to nothing holds no buffer reads the clean
//!   and the dirty list one cache section each, where the C reads both in one `splbio`.
//!
//! ## Redesign
//! - LZ's lent buffers (`&'static Buf`, their bytes through an `unsafe` `BufLegacy::data`)
//!   and the `malloc`ed copy of an indirect block as a raw slice (two `unsafe`) -> the
//!   kernel cache's tokens and an owned `MArray<u8>` of `M_TEMP`, freed by its drop where
//!   the C frees it, as the kernel's `ffs_indirtrunc` (N5.5b1); the read of an indirect block
//!   whose address is known is `getblk`, `B_READ`, `vop_strategy` of the token and `biowait`
//!   on its counted hold. The dinode is the inode's `Ext2Inode` (`ext2fs_dinode.rs`), its
//!   members read and written in place.
//! - `EXT2FS_ITIMES` (LZ's `InodeExt::ext2fs_itimes` in `bsd`'s `ufs/inode.rs`) is
//!   [`ext2fs_itimes`] here, beside its callers (the kernel's UFS layer calls it under
//!   `option EXT2FS` from N5.6c3, as the C's `#ifdef EXT2FS`).
//! - N5.6c2: `ext2fs_inactive` over `bsd`'s `VopInactiveArgs` -> the kernel's `Vnops<G>`
//!   slot shape (`vp`, `p`), the slot of `ext2fs_vops` and `ext2fs_specvops`
//!   (`ext2fs_vnops.rs`); the C's `ip->i_e2din == NULL` is the inode's empty dinode slot
//!   (`din_is_null`), and the inode's handle is dropped before `vrecycle`, as `ufs_inactive`'s.

use hal::malloc::{M_WAITOK, MArray};
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::mount::MNT_RDONLY;
use hal::sys::types::{Daddr, Mode, Off};

use super::ext2fs::{
    E2FS_REV0, EXT2F_ROCOMPAT_LARGE_FILE, blkoff, fsbtodb, ino_to_fsba, ino_to_fsbo, lblkno, nindir,
};
use super::ext2fs_alloc::{ext2fs_blkfree, ext2fs_inode_free};
use super::ext2fs_balloc::ext2fs_buf_alloc;
use super::ext2fs_dinode::{EXT2_MAXSYMLINKLEN, NDADDR, NIADDR, e2fs_isave};
use crate::assert::kpanic;
use crate::fs::bio::getblk_wait;
use crate::fs::bio::{BCSTATS, bawrite, bdwrite, biowait, bread, brelse, btodb, bwrite};
use crate::fs::buf::{B_CLRBUF, B_DELWRI, B_DONE, B_INVAL, B_READ, B_SYNC};
use crate::fs::getcwd::M_TEMP;
use crate::fs::subr::{vinvalbuf, vrecycle};
use crate::fs::ufs::ufs::dinode::{IFMT, IFREG};
use crate::fs::ufs::ufs::inode::vtoi;
use crate::fs::ufs::ufs::inode::{IN_ACCESS, IN_CHANGE, IN_MODIFIED, IN_UPDATE, Inode, le32_at};
use crate::fs::vnode::{IO_SYNC, V_SAVE, V_SAVEMETA, VDIR, VLNK, VREG, Vnode};
use crate::fs::vops::{Cred, IoBuf, vop_strategy, vop_unlock};
use crate::proc::{ProcGlue, Thread};
use crate::tc::{getnanotime, gettime};
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// `SINGLE`: index of single indirect block.
const SINGLE: usize = 0;
/// `DOUBLE`: index of double indirect block.
const DOUBLE: usize = 1;
/// `TRIPLE`: index of triple indirect block.
const TRIPLE: usize = 2;

/// `EXT2FS_ITIMES(ip)`: updates the access, modification and change times the flags ask
/// for, as `UFS_ITIMES` does for an FFS inode.
pub fn ext2fs_itimes<G: ProcGlue>(ip: &Inode<G>) {
    let flag = ip.i_flag.get();
    if flag & (IN_ACCESS | IN_CHANGE | IN_UPDATE) == 0 {
        return;
    }
    let d = ip.i_e2din();
    ip.i_flag.set(flag | IN_MODIFIED);
    if flag & IN_ACCESS != 0 {
        d.set_i_e2fs_atime(gettime() as u32);
    }
    if flag & IN_UPDATE != 0 {
        d.set_i_e2fs_mtime(gettime() as u32);
    }
    if flag & IN_CHANGE != 0 {
        d.set_i_e2fs_ctime(gettime() as u32);
        ip.i_modrev.set(ip.i_modrev.get().wrapping_add(1));
    }
    ip.i_flag
        .set(ip.i_flag.get() & !(IN_ACCESS | IN_CHANGE | IN_UPDATE));
}

/// `ext2fs_size`: get the size of an inode (the high 32 bits count for a regular file).
pub fn ext2fs_size<G: ProcGlue>(ip: &Inode<G>) -> u64 {
    let d = ip.i_e2din();
    let mut size = u64::from(d.i_e2fs_size());

    if u32::from(d.i_e2fs_mode()) & IFMT == IFREG {
        size |= u64::from(d.i_e2fs_size_hi()) << 32;
    }

    size
}

/// `ext2fs_setsize`: set the size of an inode. `EFBIG` past the file system's maximum (and
/// then the file system is marked as holding large files, as Linux does).
pub fn ext2fs_setsize<G: ProcGlue>(ip: &Inode<G>, size: u64) -> Result<(), Errno> {
    let fs = ip.i_e2fs();
    let d = ip.i_e2din();

    if size <= fs.e2fs_maxfilesize.get() as u64 {
        // If HUGE_FILEs are off, e2fs_maxfilesize will protect us.
        if u32::from(d.i_e2fs_mode()) & IFMT == IFREG || d.i_e2fs_mode() == 0 {
            d.set_i_e2fs_size_hi((size >> 32) as u32);
        }

        d.set_i_e2fs_size(size as u32);
        return Ok(());
    }

    // Linux automagically upgrades to REV1 here! (`<= E2FS_REV0` in C; 0 is the least.)
    if fs.e2fs_rev() == E2FS_REV0 {
        return Err(Errno::EFBIG);
    }

    if fs.e2fs_features_rocompat() & EXT2F_ROCOMPAT_LARGE_FILE == 0 {
        fs.set_e2fs_features_rocompat(fs.e2fs_features_rocompat() | EXT2F_ROCOMPAT_LARGE_FILE);
        fs.e2fs_fmod.set(1);
    }
    Err(Errno::EFBIG)
}

/// `ext2fs_inactive` (`vop_inactive`): last reference to an inode. If necessary, write or
/// delete it.
pub fn ext2fs_inactive<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let mut error = Ok(());

    #[cfg(feature = "diagnostic")]
    crate::fs::ufs::ufs::ufs_inode::pushing_active(vp, "ext2fs_inactive: pushing active");

    // Get rid of inodes related to stale file handles.
    if !(ip.din_is_null() || ip.i_e2din().i_e2fs_mode() == 0 || ip.i_e2din().i_e2fs_dtime() != 0) {
        let d = ip.i_e2din();
        let rdonly = vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0);
        if d.i_e2fs_nlink() == 0 && !rdonly {
            if ext2fs_size(&ip) != 0 {
                error = ext2fs_truncate(&ip, 0, 0, Cred::NOCRED);
            }
            let ts = getnanotime();
            d.set_i_e2fs_dtime(ts.tv_sec as u32);
            ip.set_flag(IN_CHANGE | IN_UPDATE);
            ext2fs_inode_free(&ip, ip.i_number, Mode::from(d.i_e2fs_mode()));
        }
        if ip.i_flag.get() & (IN_ACCESS | IN_CHANGE | IN_MODIFIED | IN_UPDATE) != 0 {
            let _ = ext2fs_update(&ip, 0);
        }
    }
    // out:
    let _ = vop_unlock(vp);
    // If we are done with the inode, reclaim it so that it can be reused immediately.
    if ip.din_is_null() || ip.i_e2din().i_e2fs_dtime() != 0 {
        drop(ip);
        vrecycle(vp, p);
    }
    error
}

/// `ext2fs_update`: update the access, modified, and inode change times as specified by
/// the `IN_ACCESS`, `IN_UPDATE`, and `IN_CHANGE` flags respectively, and write the inode
/// back to its block if it was modified. The `IN_MODIFIED` flag is used to specify that the
/// inode needs to be updated but that the times have already been set. If `waitfor` is
/// set, then wait for the disk write of the inode to complete.
pub fn ext2fs_update<G: ProcGlue>(ip: &Inode<G>, waitfor: i32) -> Result<(), Errno> {
    if ip
        .itov()
        .mount()
        .is_some_and(|mp| mp.flag() & MNT_RDONLY != 0)
    {
        return Ok(());
    }
    ext2fs_itimes(ip);
    if ip.i_flag.get() & IN_MODIFIED == 0 {
        return Ok(());
    }
    ip.clr_flag(IN_MODIFIED);
    let fs = ip.i_e2fs();
    let (mut t, error) = bread(
        &ip.i_devvp().bufs,
        fsbtodb(fs, Daddr::from(ino_to_fsba(fs, ip.i_number))),
        fs.e2fs_bsize.get(),
    );
    if let Err(e) = error {
        brelse(t);
        return Err(e);
    }
    ip.clr_flag(IN_MODIFIED);

    // See note about 16-bit UID/GID limitation in ext2fs_vget(). Now that we are about to
    // write the inode, construct the split UID and GID fields out of the two 32-bit fields
    // we kept in memory.
    let d = ip.i_e2din();
    let uid = d.i_e2fs_uid.get();
    let gid = d.i_e2fs_gid.get();
    d.set_i_e2fs_uid_low(uid as u16);
    d.set_i_e2fs_gid_low(gid as u16);
    d.set_i_e2fs_uid_high((uid >> 16) as u16);
    d.set_i_e2fs_gid_high((gid >> 16) as u16);

    let off = ino_to_fsbo(fs, ip.i_number) as usize * fs.dinode_size();
    let Some(cp) = t.data().get_mut(off..) else {
        kpanic(format_args!(
            "ext2fs_update: inode {} past its block",
            ip.i_number
        ));
    };
    e2fs_isave(fs, d, cp);
    if waitfor != 0 {
        bwrite(t)
    } else {
        bdwrite(t);
        Ok(())
    }
}

/// `ext2fs_truncate`: truncate the inode `oip` to at most `length` size, freeing the disk
/// blocks.
pub fn ext2fs_truncate<G: ProcGlue>(
    oip: &Inode<G>,
    length: Off,
    flags: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let ovp = oip.itov();
    let d = oip.i_e2din();

    if length < 0 {
        return Err(Errno::EINVAL);
    }

    let t = ovp.vtype();
    if t != VREG && t != VDIR && t != VLNK {
        return Ok(());
    }

    if t == VLNK && ext2fs_size(oip) < EXT2_MAXSYMLINKLEN as u64 {
        #[cfg(feature = "diagnostic")]
        if length != 0 {
            kpanic(format_args!("ext2fs_truncate: partial truncate of symlink"));
        }
        let n = ext2fs_size(oip) as usize;
        d.set_e2di_shortlink(0, &[0u8; EXT2_MAXSYMLINKLEN][..n]);
        let _ = ext2fs_setsize(oip, 0);
        oip.set_flag(IN_CHANGE | IN_UPDATE);
        return ext2fs_update(oip, 1);
    }

    if ext2fs_size(oip) == length as u64 {
        oip.set_flag(IN_CHANGE | IN_UPDATE);
        return ext2fs_update(oip, 0);
    }
    let fs = oip.i_e2fs();
    let bsize = fs.e2fs_bsize.get();
    let osize = ext2fs_size(oip) as Off;
    // Lengthen the size of the file. We must ensure that the last byte of the file is
    // allocated. Since the smallest value of osize is 0, length will be at least 1.
    if osize < length {
        let offset = blkoff(fs, length - 1);
        let lbn = lblkno(fs, length - 1);
        let mut aflags = B_CLRBUF;
        if flags & IO_SYNC != 0 {
            aflags |= B_SYNC;
        }
        let t = ext2fs_buf_alloc(oip, lbn as u32, offset as i32 + 1, cred, aflags)?;
        let _ = ext2fs_setsize(oip, length as u64);
        uvm_vnp_setsize(ovp, length);
        let _ = uvm_vnp_uncache(ovp);
        if aflags & B_SYNC != 0 {
            let _ = bwrite(t);
        } else {
            bawrite(t);
        }
        oip.set_flag(IN_CHANGE | IN_UPDATE);
        return ext2fs_update(oip, 1);
    }
    // Shorten the size of the file. If the file is not being truncated to a block
    // boundary, the contents of the partial block following the end of the file must be
    // zero'ed in case it ever become accessible again because of subsequent file growth.
    let offset = blkoff(fs, length);
    if offset == 0 {
        let _ = ext2fs_setsize(oip, length as u64);
    } else {
        let lbn = lblkno(fs, length);
        let mut aflags = B_CLRBUF;
        if flags & IO_SYNC != 0 {
            aflags |= B_SYNC;
        }
        let mut t = ext2fs_buf_alloc(oip, lbn as u32, offset as i32, cred, aflags)?;
        let _ = ext2fs_setsize(oip, length as u64);
        let size = bsize as usize;
        uvm_vnp_setsize(ovp, length);
        let _ = uvm_vnp_uncache(ovp);
        t.data()[offset as usize..size].fill(0);
        t.buf().set_bcount(size as i64);
        if aflags & B_SYNC != 0 {
            let _ = bwrite(t);
        } else {
            bawrite(t);
        }
    }
    // Calculate index into inode's block list of last direct and indirect blocks (if any)
    // which we want to keep. Lastblock is -1 when the file is truncated to 0.
    let (lastblock, mut lastiblock) =
        ext2fs_truncate_lastblocks(lblkno(fs, length + Off::from(bsize) - 1) - 1, nindir(fs));
    let nblocks = btodb(i64::from(bsize));
    // Update file and block pointers on disk before we start freeing blocks. If we crash
    // before free'ing blocks below, the blocks will be returned to the free list.
    // lastiblock values are also normalized to -1 for calls to ext2fs_indirtrunc below.
    let oldblks = d.i_e2fs_blocks();
    for level in (SINGLE..=TRIPLE).rev() {
        if lastiblock[level] < 0 {
            d.set_i_e2fs_block(NDADDR + level, 0);
            lastiblock[level] = -1;
        }
    }
    let mut i = NDADDR as Daddr - 1;
    while i > lastblock {
        d.set_i_e2fs_block(i as usize, 0);
        i -= 1;
    }
    oip.set_flag(IN_CHANGE | IN_UPDATE);
    // The C keeps this error in allerror, which vinvalbuf's result replaces below.
    let _ = ext2fs_update(oip, 1);

    // Having written the new inode to disk, save its new configuration and put back the old
    // block pointers long enough to process them. Note that we save the new block
    // configuration so we can check it when we are done.
    let newblks = d.i_e2fs_blocks();
    d.set_i_e2fs_blocks(&oldblks);
    let _ = ext2fs_setsize(oip, osize as u64);
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
            let bn = u32::from_le(d.i_e2fs_block(NDADDR + level));
            if bn != 0 {
                let mut count = 0;
                if let Err(e) = ext2fs_indirtrunc(
                    oip,
                    indir_lbn[level],
                    fsbtodb(fs, Daddr::from(bn)),
                    lastiblock[level],
                    level,
                    &mut count,
                ) {
                    allerror = Err(e);
                }
                blocksreleased += count;
                if lastiblock[level] < 0 {
                    d.set_i_e2fs_block(NDADDR + level, 0);
                    ext2fs_blkfree(oip, bn);
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
            let bn = u32::from_le(d.i_e2fs_block(i as usize));
            if bn != 0 {
                d.set_i_e2fs_block(i as usize, 0);
                ext2fs_blkfree(oip, bn);
                blocksreleased += btodb(i64::from(bsize));
            }
            i -= 1;
        }
    }
    // done:
    #[cfg(feature = "diagnostic")]
    {
        for level in SINGLE..=TRIPLE {
            if newblks[NDADDR + level] != d.i_e2fs_block(NDADDR + level) {
                kpanic(format_args!("ext2fs_truncate1"));
            }
        }
        for (i, &blk) in newblks[..NDADDR].iter().enumerate() {
            if blk != d.i_e2fs_block(i) {
                kpanic(format_args!("ext2fs_truncate2"));
            }
        }
        if length == 0 && (!ovp.bufs.clean_is_empty() || !ovp.bufs.dirty_is_empty()) {
            kpanic(format_args!("ext2fs_truncate3"));
        }
    }
    #[cfg(not(feature = "diagnostic"))]
    let _ = newblks;
    // Put back the real size.
    let _ = ext2fs_setsize(oip, length as u64);
    d.set_i_e2fs_nblock(ext2fs_truncate_nblock(d.i_e2fs_nblock(), blocksreleased));
    oip.set_flag(IN_CHANGE);
    allerror
}

/// The last direct block to keep (-1 when the file is truncated to 0) and, per level of
/// indirection, the last block to keep below the indirect block (negative: none), from
/// `lastblock = lblkno(fs, length + bsize - 1) - 1` and `NINDIR(fs)`, as `ext2fs_truncate`
/// computes them in `int32_t`.
fn ext2fs_truncate_lastblocks(lastblock: Daddr, nindir: i64) -> (Daddr, [Daddr; NIADDR]) {
    let lastblock = Daddr::from(lastblock as i32);
    let mut lastiblock = [0 as Daddr; NIADDR];
    lastiblock[SINGLE] = Daddr::from((lastblock - NDADDR as Daddr) as i32);
    lastiblock[DOUBLE] = Daddr::from((lastiblock[SINGLE] - nindir) as i32);
    lastiblock[TRIPLE] = Daddr::from((lastiblock[DOUBLE] - nindir * nindir) as i32);
    (lastblock, lastiblock)
}

/// The inode's block count once `blocksreleased` `DEV_BSIZE` blocks are released (none left
/// when more are released than it counts).
fn ext2fs_truncate_nblock(nblock: u32, blocksreleased: i64) -> u32 {
    if blocksreleased >= i64::from(nblock) {
        0
    } else {
        nblock - blocksreleased as u32
    }
}

/// `ext2fs_indirtrunc`: release blocks associated with the inode `ip` and stored in the
/// indirect block `lbn` (at disk block `dbn`). Blocks are free'd in LIFO order up to (but
/// not including) `lastbn`. If level is greater than `SINGLE`, the block is an indirect
/// block and recursive calls to indirtrunc must be used to cleanse other indirect blocks.
/// `*countp` is the number of `DEV_BSIZE` blocks released.
///
/// NB: triple indirect blocks are untested.
fn ext2fs_indirtrunc<G: ProcGlue>(
    ip: &Inode<G>,
    lbn: Daddr,
    dbn: Daddr,
    lastbn: Daddr,
    level: usize,
    countp: &mut i64,
) -> Result<(), Errno> {
    let fs = ip.i_e2fs();
    let bsize = fs.e2fs_bsize.get();
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
    let mut t = getblk_wait(&vp.bufs, lbn, bsize);
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
            kpanic(format_args!("ext2fs_indirtrunc: bad buffer size"));
        }
        t.buf().set_blkno(dbn);
        let Some(r) = t.counted() else {
            kpanic(format_args!("ext2fs_indirtrunc: a driver's buffer"));
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
    if lastbn >= 0 {
        let Some(mut t) = held.take() else {
            kpanic(format_args!("ext2fs_indirtrunc: no buffer"));
        };
        let data = t.data();
        let Some(c) = MArray::new(M_TEMP, M_WAITOK, bsize as usize, |i| {
            data.get(i).copied().unwrap_or(0)
        }) else {
            kpanic(format_args!("ext2fs_indirtrunc: no memory"));
        };
        data[(last + 1) as usize * 4..nindir(fs) as usize * 4].fill(0);
        copy = Some(c);
        if let Err(e) = bwrite(t) {
            allerror = Err(e);
        }
    }
    let baps: &[u8] = match (&copy, &held) {
        (Some(c), _) => c,
        (None, Some(t)) => t.bytes(),
        (None, None) => kpanic(format_args!("ext2fs_indirtrunc: no block pointers")),
    };
    let bap = |i: i64| le32_at(baps, i as usize);

    // Recursively free totally unused blocks.
    let mut i = nindir(fs) - 1;
    let mut nlbn = lbn + 1 - i * factor;
    while i > last {
        let nb = bap(i);
        if nb != 0 {
            if level > SINGLE {
                let mut blkcount = 0;
                if let Err(e) = ext2fs_indirtrunc(
                    ip,
                    nlbn,
                    fsbtodb(fs, Daddr::from(nb)),
                    -1,
                    level - 1,
                    &mut blkcount,
                ) {
                    allerror = Err(e);
                }
                blocksreleased += blkcount;
            }
            ext2fs_blkfree(ip, nb);
            blocksreleased += nblocks;
        }
        i -= 1;
        nlbn += factor;
    }

    // Recursively free last partial block.
    if level > SINGLE && lastbn >= 0 {
        let last = lastbn % factor;
        let nb = bap(i);
        if nb != 0 {
            let mut blkcount = 0;
            if let Err(e) = ext2fs_indirtrunc(
                ip,
                nlbn,
                fsbtodb(fs, Daddr::from(nb)),
                last,
                level - 1,
                &mut blkcount,
            ) {
                allerror = Err(e);
            }
            blocksreleased += blkcount;
        }
    }

    // free(copy, M_TEMP, fs->e2fs_bsize): the copy's drop; or the buffer released invalid.
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
    // Host tests for the size of an inode (the high 32 bits of a regular file's size, the
    // file system's limit), the block arithmetic of `ext2fs_truncate`, and, over the test
    // image, `ext2fs_update`'s bytes and `ext2fs_truncate`/`ext2fs_indirtrunc` through the
    // buffer cache, the bitmaps and counters checked on the disk.

    use std::{assert, assert_eq};

    use super::super::ext2fs::{E2FS_REV1, Ext2Superblock};
    use super::super::testext2fs::newfs::{self, B, BBITMAP, BIGIND, ISZ, NFREE};
    use super::super::testext2fs::{attach, detached, mount, ufsmount, vget};
    use super::*;
    use crate::fs::subr::vput;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs;
    use crate::fs::ufs::ufs::dinode::IFDIR;
    use crate::fs::ufs::ufs::inode::vtoi;

    /// An inode of `mode` with its own dinode, on a file system of revision `rev` whose files
    /// may grow to `maxfilesize`.
    fn inode(mode: u32, rev: u32, maxfilesize: Off) -> &'static Inode<TestGlue> {
        let fs = Ext2Superblock::new();
        fs.set_e2fs_rev(rev);
        fs.e2fs_maxfilesize.set(maxfilesize);
        let ip = detached(&ufsmount(fs), 20);
        ip.i_e2din().set_i_e2fs_mode(mode as u16);
        ip
    }

    #[test]
    fn the_high_size_word_counts_for_regular_files_only() {
        let (_g, _p) = attach(newfs::image(false));
        let ip = inode(IFREG | 0o644, E2FS_REV1, 1 << 40);
        ext2fs_setsize(ip, (5 << 32) | 7).unwrap();
        let d = ip.i_e2din();
        assert_eq!((d.i_e2fs_size(), d.i_e2fs_size_hi()), (7, 5));
        assert_eq!(ext2fs_size(ip), (5 << 32) | 7);

        let dir = inode(IFDIR | 0o755, E2FS_REV1, 1 << 40);
        dir.i_e2din().set_i_e2fs_size_hi(3);
        ext2fs_setsize(dir, 1024).unwrap();
        assert_eq!(
            (ext2fs_size(dir), dir.i_e2din().i_e2fs_size_hi()),
            (1024, 3)
        );
    }

    #[test]
    fn growing_past_the_limit_is_efbig_and_asks_for_large_files() {
        let (_g, _p) = attach(newfs::image(false));
        let ip = inode(IFREG, E2FS_REV0, 1000);
        assert_eq!(ext2fs_setsize(ip, 1001), Err(Errno::EFBIG));
        assert_eq!(ip.i_e2fs().e2fs_features_rocompat(), 0);

        let ip = inode(IFREG, E2FS_REV1, 1000);
        assert_eq!(ext2fs_setsize(ip, 1001), Err(Errno::EFBIG));
        assert_eq!(
            ip.i_e2fs().e2fs_features_rocompat(),
            EXT2F_ROCOMPAT_LARGE_FILE
        );
        assert_eq!(ip.i_e2fs().e2fs_fmod.get(), 1);
        assert_eq!(ext2fs_size(ip), 0);
    }

    #[test]
    fn truncate_keeps_the_blocks_below_the_new_end() {
        // To nothing: no direct block, no block under any indirect one.
        assert_eq!(
            ext2fs_truncate_lastblocks(-1, 256),
            (-1, [-13, -13 - 256, -13 - 256 - 65536])
        );
        // One byte into logical block 13: blocks 0..=13, so two under the single indirect.
        assert_eq!(
            ext2fs_truncate_lastblocks(13, 256),
            (13, [1, 1 - 256, 1 - 256 - 65536])
        );
        // Into the double indirect range.
        let (_, l) = ext2fs_truncate_lastblocks(12 + 256 + 300, 256);
        assert_eq!((l[SINGLE], l[DOUBLE]), (556, 300));

        assert_eq!(ext2fs_truncate_nblock(6, 4), 2);
        assert_eq!(ext2fs_truncate_nblock(6, 6), 0);
        assert_eq!(ext2fs_truncate_nblock(6, 10), 0);
    }

    #[test]
    fn itimes_sets_the_times_the_flags_ask_for() {
        let (_g, _p) = attach(newfs::image(false));
        let ip = inode(IFREG, E2FS_REV1, 1 << 40);
        let d = ip.i_e2din();
        d.set_i_e2fs_atime(1);
        d.set_i_e2fs_mtime(1);
        ip.set_flag(IN_UPDATE | IN_CHANGE);
        let rev = ip.i_modrev.get();
        ext2fs_itimes(ip);
        assert_eq!(ip.i_flag.get(), IN_MODIFIED);
        assert_eq!(d.i_e2fs_atime(), 1, "no IN_ACCESS: the access time stays");
        assert_eq!(d.i_e2fs_mtime(), gettime() as u32);
        assert_eq!(d.i_e2fs_ctime(), gettime() as u32);
        assert_eq!(ip.i_modrev.get(), rev + 1);
        ip.i_flag.set(0);
        ext2fs_itimes(ip);
        assert_eq!(ip.i_flag.get(), 0, "nothing asked: nothing done");
    }

    /// The `u32` at byte `off` of block `blk` of the disk, after the delayed writes.
    fn disk_u32(blk: usize, off: usize) -> u32 {
        testffs::sync(&[]);
        let d = testffs::disk();
        let o = blk * B + off;
        u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
    }

    #[test]
    fn update_writes_the_dinode_and_its_split_ids_into_its_block() {
        let (_g, _p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = vget(mp, 12).unwrap();
        let ip = vtoi(vp);
        let d = ip.i_e2din();
        assert_eq!(
            d.i_e2fs_uid.get(),
            1000 | 1 << 16,
            "vget joins the split uid"
        );
        d.i_e2fs_uid.set(0x0003_0004);
        d.set_i_e2fs_mode(0o100600);
        ip.set_flag(IN_MODIFIED);
        let (blk, off) = newfs::Image::ioff(12);
        let next = testffs::disk()[blk * B + off + ISZ..blk * B + off + 2 * ISZ].to_vec();
        assert_eq!(ext2fs_update(&ip, 1), Ok(()));
        assert_eq!(ip.i_flag.get() & IN_MODIFIED, 0);
        let disk = testffs::disk();
        let at = blk * B + off;
        assert_eq!(disk[at..at + 2], 0o100600u16.to_le_bytes());
        assert_eq!(disk[at + 2..at + 4], 4u16.to_le_bytes(), "e2di_uid_low");
        assert_eq!(
            disk[at + 120..at + 122],
            3u16.to_le_bytes(),
            "e2di_uid_high"
        );
        assert_eq!(
            disk[at + ISZ..at + 2 * ISZ],
            next[..],
            "the next inode is untouched"
        );
        let mut want = [0u8; ISZ];
        e2fs_isave(ip.i_e2fs(), d, &mut want);
        assert_eq!(disk[at..at + ISZ], want);
        drop(ip);
        vput(vp);
    }

    #[test]
    fn a_read_only_mount_writes_no_inode() {
        let (_g, _p) = attach(newfs::image(false));
        let mp = mount(MNT_RDONLY);
        let vp = vget(mp, 12).unwrap();
        let ip = vtoi(vp);
        ip.i_e2din().set_i_e2fs_mode(0o100600);
        ip.set_flag(IN_MODIFIED | IN_CHANGE);
        let before = testffs::disk();
        assert_eq!(ext2fs_update(&ip, 1), Ok(()));
        assert_eq!(ip.i_flag.get() & IN_MODIFIED, IN_MODIFIED, "nothing done");
        assert_eq!(testffs::disk(), before);
        drop(ip);
        vput(vp);
    }

    /// Whether block `b` is used in the disk's bitmap (bit `b - 1`).
    fn used(b: u32) -> bool {
        testffs::sync(&[]);
        let i = (b - 1) as usize;
        testffs::disk()[BBITMAP * B + i / 8] & (1 << (i % 8)) != 0
    }

    #[test]
    fn truncating_into_the_indirect_range_frees_the_blocks_past_the_end() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = vget(mp, 13).unwrap();
        let ip = vtoi(vp);
        let fs = ip.i_e2fs();
        let cred = Cred::real(p.ucred());
        let nblock = ip.i_e2din().i_e2fs_nblock();
        assert!(used(25) && used(24));

        // 12 blocks and a byte: block 13 (disk 25) goes, the indirect block stays with one
        // pointer, the rest of the last block is zeroed.
        let len = (12 * B + 1) as Off;
        assert_eq!(ext2fs_truncate(&ip, len, IO_SYNC, cred), Ok(()));
        assert_eq!(ext2fs_size(&ip), len as u64);
        assert_eq!(fs.e2fs_fbcount(), NFREE + 1);
        assert!(!used(25) && used(24) && used(BIGIND as u32));
        assert_eq!((disk_u32(BIGIND, 0), disk_u32(BIGIND, 4)), (24, 0));
        assert_eq!(ip.i_e2din().i_e2fs_nblock(), nblock - 2);
        let d = testffs::disk();
        assert!(
            d[24 * B + 1..25 * B].iter().all(|&b| b == 0),
            "the tail is zeroed"
        );

        // To nothing: the 12 direct blocks, block 24 and the indirect block go.
        assert_eq!(ext2fs_truncate(&ip, 0, IO_SYNC, cred), Ok(()));
        assert_eq!(ext2fs_size(&ip), 0);
        assert_eq!(fs.e2fs_fbcount(), NFREE + 15);
        assert_eq!(fs.gd_nbfree(0), (NFREE + 15) as u16);
        assert_eq!(ip.i_e2din().i_e2fs_nblock(), 0);
        assert_eq!(ip.i_e2din().i_e2fs_blocks(), [0; NDADDR + NIADDR]);
        for b in 11..=25 {
            assert!(!used(b), "block {b}");
        }
        // The same length again is a change of times only.
        assert_eq!(ext2fs_truncate(&ip, 0, 0, cred), Ok(()));
        assert_eq!(ext2fs_truncate(&ip, -1, 0, cred), Err(Errno::EINVAL));
        drop(ip);
        vput(vp);
    }

    #[test]
    fn lengthening_a_file_allocates_its_last_block() {
        let (_g, p) = attach(newfs::image(false));
        let mp = mount(0);
        let vp = vget(mp, 12).unwrap();
        let ip = vtoi(vp);
        let cred = Cred::real(p.ucred());
        let len = (3 * B + 10) as Off;
        assert_eq!(ext2fs_truncate(&ip, len, IO_SYNC, cred), Ok(()));
        assert_eq!(ext2fs_size(&ip), len as u64);
        let d = ip.i_e2din();
        // Block 3 allocated (blocks 1 and 2 stay holes), 2 sectors more.
        assert_eq!((d.i_e2fs_block(1), d.i_e2fs_block(2)), (0, 0));
        let b3 = u32::from_le(d.i_e2fs_block(3));
        assert!(b3 != 0 && used(b3));
        assert_eq!(ip.i_e2fs().e2fs_fbcount(), NFREE - 1);
        drop(ip);
        vput(vp);
    }
}
/* </TESTS> */
